use std::{ffi::CString, time::Duration};

use image::{ImageBuffer, Rgba, RgbaImage};
use imlib_rs::{
    ImlibContextPush, ImlibContextSetImage, ImlibCreateCroppedScaledImage,
    ImlibFreeImageAndDecache, ImlibImage, ImlibImageGetData, ImlibImageGetHeight,
    ImlibImageGetWidth, ImlibLoadImage,
};
use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use x11::xlib::{
    AnyPropertyType, False, PropModeReplace, True, XChangeProperty, XGetWindowProperty,
    XInternAtom, XKillClient, _XDisplay, XA_PIXMAP,
};

use crate::{run, DisplayContext, Monitor, MICROSECONDS_PER_SECOND};

fn combine_images(
    image_position: (i32, i32),
    image_size: (i32, i32),
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    image_data: &[u32],
) {
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
            .unwrap();

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
}

unsafe fn composite_images(
    monitor: Monitor,
    display_contexts: Vec<DisplayContext>,
) -> Vec<ImlibImage> {
    info!("Creating output bitmap directory");

    if std::fs::exists("output-bmps").expect("Failed to check if output bitmap directory exists!") {
        std::fs::remove_dir_all("output-bmps")
            .expect("Failed to remove old output bitmap directory!");
    }

    std::fs::create_dir("output-bmps").expect("Failed to create output bitmap directory!");

    info!("Compositing bitmap images");
    ImlibContextPush(monitor.render_context);

    let max_length = display_contexts
        .iter()
        .map(|context| context.images.len() as f32 / context.fps)
        .max_by(f32::total_cmp)
        .unwrap();

    let output_fps = display_contexts
        .iter()
        .map(|context| context.fps)
        .max_by(f32::total_cmp)
        .unwrap();

    let output_frames = (max_length * output_fps) as usize;

    let all_frame_combos = (0..output_frames).map(|frame| {
        display_contexts
            .iter()
            .map(|ctx| (frame as f32 * ctx.fps / output_fps) as usize % ctx.images.len())
            .collect::<Vec<_>>()
    });

    let mut output_frames = vec![];

    for (i, frame_combo) in all_frame_combos.enumerate() {
        let mut canvas = RgbaImage::from_pixel(
            monitor.width as u32,
            monitor.height as u32,
            Rgba([0, 0, 0, 0]),
        );

        for (i, frame) in frame_combo.iter().enumerate().rev() {
            let current_image = display_contexts[i].images[*frame];

            ImlibContextSetImage(current_image);

            let image_height = ImlibImageGetHeight();
            let image_width = ImlibImageGetWidth();

            let scaled_image = ImlibCreateCroppedScaledImage(
                0,
                0,
                image_width,
                image_height,
                display_contexts[i].width as i32,
                display_contexts[i].height as i32,
            );

            ImlibContextSetImage(scaled_image);

            let updated_image_height = ImlibImageGetHeight();
            let updated_image_width = ImlibImageGetWidth();

            let temp_image_data = std::slice::from_raw_parts(
                ImlibImageGetData(),
                (display_contexts[i].width * display_contexts[i].height) as usize,
            );

            combine_images(
                (display_contexts[i].x, display_contexts[i].y),
                (updated_image_width, updated_image_height),
                &mut canvas,
                temp_image_data,
            );
        }

        info!("Frame {} done!", i);

        canvas
            .save_with_format(
                format!("output-bmps/output-bmp-{}.bmp", i),
                image::ImageFormat::Bmp,
            )
            .unwrap();

        let image_path_c_str = CString::new(format!("output-bmps/output-bmp-{}.bmp", i)).unwrap();

        output_frames.push(ImlibLoadImage(image_path_c_str.as_ptr() as *const i8));

        ImlibFreeImageAndDecache();
    }

    output_frames
}

unsafe fn clear_root_atoms(display: *mut _XDisplay, monitor: Monitor) {
    let atom_root = XInternAtom(display, c"_XROOTMAP_ID".as_ptr() as *const i8, True);
    let atom_eroot = XInternAtom(display, c"ESETROOT_PMAP_ID".as_ptr() as *const i8, True);
    if atom_root != 0 && atom_eroot != 0 {
        todo!();
    }
}

pub unsafe fn set_root_atoms(display: *mut _XDisplay, monitor: Monitor) {
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
}

pub unsafe fn render(
    display: *mut _XDisplay,
    monitor: Monitor,
    display_contexts: Vec<DisplayContext>,
    running: Arc<AtomicBool>,
) {
    let images = composite_images(monitor, display_contexts);

    let num_elements = images.len();
    let mut cycle = 0;

    loop {
        if !running.load(Ordering::SeqCst) {
            clear_root_atoms(display, monitor);
            break;
        }

        let current_index = cycle % num_elements;

        run(display, monitor, images[current_index]);
        cycle += 1;

        let timeout = Duration::from_nanos(
            (MICROSECONDS_PER_SECOND / 60) * 1_000, // nanoseconds
        );

        std::thread::sleep(timeout);
    }

    info!("Render loop terminated.");
}
