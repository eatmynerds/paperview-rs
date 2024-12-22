use std::{
    ffi::CString,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use image::{ImageBuffer, Rgba, RgbaImage};
use imlib_rs::{
    ImlibContextPush, ImlibContextSetImage, ImlibCreateCroppedScaledImage,
    ImlibFreeImageAndDecache, ImlibImage, ImlibImageGetData, ImlibImageGetHeight,
    ImlibImageGetWidth, ImlibLoadImage,
};
use log::{debug, error, info, warn};
use x11::xlib::{
    AllTemporary, Atom, False, Pixmap, PropModeReplace, RetainTemporary, Window, XChangeProperty,
    XClearWindow, XFlush, XFree, XGetWindowProperty, XInternAtom, XKillClient, XSetCloseDownMode,
    XSetWindowBackgroundPixmap, XSync, _XDisplay, XA_PIXMAP,
};

use crate::{run, DisplayContext, Monitor, MICROSECONDS_PER_SECOND};

fn combine_images(
    image_position: (i32, i32),
    image_size: (i32, i32),
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    image_data: &[u32],
) {
    debug!(
        "Combining image at position {:?} with size {:?}",
        image_position, image_size
    );

    let updated_image_data = image_data
        .iter()
        .flat_map(|data| {
            [
                ((data >> 16) & 0xFF) as u8,
                ((data >> 8) & 0xFF) as u8,
                (data & 0xFF) as u8,
                ((data >> 24) & 0xFF) as u8,
            ]
        })
        .collect::<Vec<u8>>();

    let new_layer =
        ImageBuffer::from_raw(image_size.0 as u32, image_size.1 as u32, updated_image_data)
            .expect("Failed to create image buffer for new layer");

    let resized_layer = image::imageops::resize(
        &new_layer,
        image_size.0 as u32,
        image_size.1 as u32,
        image::imageops::FilterType::Nearest,
    );

    image::imageops::overlay(
        canvas,
        &resized_layer,
        image_position.0 as i64,
        image_position.1 as i64,
    );
    debug!("Image successfully combined");
}

unsafe fn composite_images(
    monitor: Monitor,
    display_contexts: Vec<DisplayContext>,
) -> Vec<ImlibImage> {
    info!("Preparing to composite images");

    if std::fs::exists("output-bmps").unwrap_or(false) {
        warn!("Existing 'output-bmps' directory detected, removing...");
        std::fs::remove_dir_all("output-bmps").expect("Failed to remove old output-bmps directory");
    }

    std::fs::create_dir("output-bmps").expect("Failed to create output-bmps directory");
    debug!("Output directory prepared");

    ImlibContextPush(monitor.render_context);

    let max_length = display_contexts
        .iter()
        .map(|context| context.images.len() as f32 / context.fps)
        .max_by(f32::total_cmp)
        .expect("Failed to calculate maximum animation length");

    let output_fps = display_contexts
        .iter()
        .map(|context| context.fps)
        .max_by(f32::total_cmp)
        .expect("Failed to calculate output FPS");

    let output_frames = (max_length * output_fps) as usize;

    info!(
        "Compositing {} frames at {} FPS, maximum animation length: {} seconds",
        output_frames, output_fps, max_length
    );

    let all_frame_combos = (0..output_frames).map(|frame| {
        display_contexts
            .iter()
            .map(|ctx| (frame as f32 * ctx.fps / output_fps) as usize % ctx.images.len())
            .collect::<Vec<_>>()
    });

    let mut output_frames = vec![];

    for (i, frame_combo) in all_frame_combos.enumerate() {
        debug!("Processing frame {}", i);

        let mut canvas = RgbaImage::from_pixel(
            monitor.width as u32,
            monitor.height as u32,
            Rgba([0, 0, 0, 0]),
        );

        for (ctx_index, frame) in frame_combo.iter().enumerate().rev() {
            let current_image = display_contexts[ctx_index].images[*frame];

            ImlibContextSetImage(current_image);

            let image_height = ImlibImageGetHeight();
            let image_width = ImlibImageGetWidth();

            let scaled_image = ImlibCreateCroppedScaledImage(
                0,
                0,
                image_width,
                image_height,
                display_contexts[ctx_index].width as i32,
                display_contexts[ctx_index].height as i32,
            );

            if scaled_image.is_null() {
                error!(
                    "Failed to scale image for frame {} in context {}",
                    i, ctx_index
                );
                continue;
            }

            ImlibContextSetImage(scaled_image);

            let updated_image_height = ImlibImageGetHeight();
            let updated_image_width = ImlibImageGetWidth();

            let temp_image_data = std::slice::from_raw_parts(
                ImlibImageGetData(),
                (display_contexts[ctx_index].width * display_contexts[ctx_index].height) as usize,
            );

            combine_images(
                (display_contexts[ctx_index].x, display_contexts[ctx_index].y),
                (updated_image_width, updated_image_height),
                &mut canvas,
                temp_image_data,
            );
        }

        info!("Frame {} composited successfully", i);

        let output_path = format!("output-bmps/output-bmp-{}.bmp", i);
        canvas
            .save_with_format(&output_path, image::ImageFormat::Bmp)
            .expect("Failed to save BMP frame");

        debug!("Frame {} saved at {}", i, output_path);

        let image_path_c_str = CString::new(output_path).expect("Failed to create CString");

        output_frames.push(ImlibLoadImage(image_path_c_str.as_ptr()));

        ImlibFreeImageAndDecache();
    }

    info!(
        "Compositing complete, {} frames generated",
        output_frames.len()
    );

    output_frames
}

pub unsafe fn clear_root_atoms(display: *mut _XDisplay, monitor: Monitor, pixmap: Pixmap) {
    info!("Clearing root atoms");

    let atom_root = XInternAtom(display, c"_XROOTPMAP_ID".as_ptr() as *const i8, False);

    let atom_eroot = XInternAtom(display, c"ESETROOT_PMAP_ID".as_ptr() as *const i8, False);

    let monitor_pixmap = pixmap as u64;

    XChangeProperty(
        display,
        monitor.root,
        atom_root,
        XA_PIXMAP,
        32,
        PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );

    XChangeProperty(
        display,
        monitor.root,
        atom_eroot,
        XA_PIXMAP,
        32,
        PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );

    XSetCloseDownMode(display, RetainTemporary);

    XKillClient(display, AllTemporary as u64);

    XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);

    XClearWindow(display, monitor.root);
    XFlush(display);
    XSync(display, False);

    info!("Root atoms cleared successfully");
}

pub unsafe fn get_current_pixmap(display: *mut _XDisplay, root: Window) -> Pixmap {
    info!("Getting current pixmap");

    let atom_root = XInternAtom(display, c"_XROOTPMAP_ID".as_ptr() as *const i8, False);

    let mut actual_type: Atom = 0;
    let mut actual_format: i32 = 0;
    let mut nitems: u64 = 0;
    let mut bytes_after: u64 = 0;
    let mut prop: *mut u8 = ptr::null_mut();

    XGetWindowProperty(
        display,
        root,
        atom_root,
        0,
        1,
        False,
        XA_PIXMAP,
        &mut actual_type,
        &mut actual_format,
        &mut nitems,
        &mut bytes_after,
        &mut prop,
    );

    let pixmap_id = *(prop as *const u64);
    XFree(prop as *mut _);
    debug!("Current pixmap ID: {}", pixmap_id);
    pixmap_id as Pixmap
}

pub unsafe fn set_root_atoms(display: *mut _XDisplay, monitor: Monitor) {
    info!("Setting root atoms");

    let atom_root = XInternAtom(display, c"_XROOTPMAP_ID".as_ptr() as *const i8, False);

    let atom_eroot = XInternAtom(display, c"ESETROOT_PMAP_ID".as_ptr() as *const i8, False);

    let monitor_pixmap = monitor.pixmap as u64;

    XChangeProperty(
        display,
        monitor.root,
        atom_root,
        XA_PIXMAP,
        32,
        PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );

    XChangeProperty(
        display,
        monitor.root,
        atom_eroot,
        XA_PIXMAP,
        32,
        PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );

    info!("Root atoms set successfully");
}

pub unsafe fn render(
    display: *mut _XDisplay,
    monitor: Monitor,
    display_contexts: Vec<DisplayContext>,
    running: Arc<AtomicBool>,
) {
    info!("Starting render process");

    let images = composite_images(monitor, display_contexts.clone());

    let num_elements = images.len();
    let mut cycle = 0;
    let old_background = get_current_pixmap(display, monitor.root);

    let max_fps = display_contexts
        .iter()
        .map(|context| context.fps)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(60.0); // Default to 60 FPS if no contexts are available

    let timeout_duration = Duration::from_nanos((MICROSECONDS_PER_SECOND / max_fps as u64) * 1_000);

    loop {
        if !running.load(Ordering::SeqCst) {
            info!("Stopping render loop");
            clear_root_atoms(display, monitor, old_background);
            break;
        }

        let current_index = cycle % num_elements;

        run(display, monitor, images[current_index]);
        cycle += 1;

        std::thread::sleep(timeout_duration);
    }
}
