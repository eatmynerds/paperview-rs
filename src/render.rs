use std::{
    ffi::{c_void, CString},
    ops::DerefMut,
};

use image::{Rgba, RgbaImage};
use imlib_rs::{
    ImlibContextPush, ImlibContextSetImage, ImlibFreeImageAndDecache, ImlibImage, ImlibLoadImage,
    ImlibSaveImage,
};
use log::info;
use rayon::prelude::*;
use std::sync::Mutex;
use std::time::Duration;
use x11::xlib::{False, PropModeReplace, XChangeProperty, XInternAtom, _XDisplay, XA_PIXMAP};

use crate::{run, DisplayContext, ImageData, Monitor, MICROSECONDS_PER_SECOND};

fn combine_images(image_data: Vec<ImageData>, canvas_width: u32, canvas_height: u32) -> RgbaImage {
    let canvas = Mutex::new(RgbaImage::from_pixel(
        canvas_width,
        canvas_height,
        Rgba([0, 0, 0, 0]),
    ));

    image_data.into_par_iter().for_each(|info| {
        let img = image::open(&info.image_path).unwrap().into_rgba8();

        let resized_image = image::imageops::resize(
            &img,
            info.image_size.0 as u32,
            info.image_size.1 as u32,
            image::imageops::FilterType::Nearest,
        );

        let mut canvas = canvas.lock().unwrap();

        image::imageops::overlay(
            canvas.deref_mut(),
            &resized_image,
            info.image_position.0 as i64,
            info.image_position.1 as i64,
        );
    });

    canvas.into_inner().unwrap()
}

unsafe fn composite_images(
    monitor: Monitor,
    display_contexts: Vec<DisplayContext>,
) -> Vec<ImlibImage> {
    info!("Creating bitmap output directory");
    std::fs::create_dir("temp-bmps").expect("Failed to create temporary bitmap directory!");

    if !std::fs::read_dir("output-bmps").is_ok() {
        std::fs::create_dir("output-bmps").expect("Failed to remove output bitmap directory!");
    } else {
        std::fs::remove_dir_all("output-bmps").expect("Failed to create output bitmap directory!");
    }

    info!("Compositing bitmap images");
    ImlibContextPush(monitor.render_context);

    let num_elements = display_contexts.len();

    let mut current_info = display_contexts
        .iter()
        .map(|context| (0..context.images.len()).cycle())
        .collect::<Vec<_>>();

    let max_frames: Vec<_> = display_contexts
        .iter()
        .map(|context| context.images.len() - 1)
        .collect();

    // A loop that will iterate through all the possible frame combinations
    // wizardable-bmp: [0.bmp 1.bmp]
    // cyberpunk-bmp: [0.bmp 1.bmp 2.bmp, 3.bmp]
    //
    // 0.bmp + 0.bmp -> output.bmp
    // 1.bmp + 1.bmp -> output.bmp
    // 2.bmp + 0.bmp -> output.bmp
    // 3.bmp + 1.bmp -> output.bmp
    // ....
    let mut all_frame_combos: Vec<Vec<usize>> = vec![];

    loop {
        let frame_combo = current_info
            .iter_mut()
            .map(|context| context.next().unwrap())
            .collect::<Vec<_>>();

        all_frame_combos.push(frame_combo.clone());

        // Check if the current combo matches the maximum frame numbers
        if frame_combo
            .iter()
            .zip(&max_frames)
            .all(|(frame, &max)| *frame == max)
        {
            break;
        }
    }

    let mut combined_images_loaded = vec![];

    for (i, frame_combo) in all_frame_combos.iter().enumerate() {
        let mut combined_frame_paths = vec![];

        for (i, frame) in frame_combo.iter().enumerate() {
            let current_image = display_contexts[i].images[*frame];

            ImlibContextSetImage(current_image);

            let image_path_str = CString::new(format!("temp-bmps/temp-bitmap-{}-{}.bmp", i, frame))
                .expect("Failed to convert filename to c-string!");

            ImlibSaveImage(image_path_str.as_ptr() as *const i8);

            let temp_bitmap =
                std::fs::canonicalize(format!("temp-bmps/temp-bitmap-{}-{}.bmp", i, frame))
                    .unwrap();

            let current_frame_path = temp_bitmap
                .to_str()
                .expect("Failed to convert to string!")
                .to_string();

            combined_frame_paths.push(ImageData {
                image_path: current_frame_path,
                image_size: (display_contexts[i].width, display_contexts[i].height),
                image_position: (display_contexts[i].x, display_contexts[i].y),
            });
        }

        let combined_frames = combine_images(
            combined_frame_paths,
            monitor.width as u32,
            monitor.height as u32,
        );

        combined_frames
            .save_with_format(
                format!("output-bmps/output-bmp-{}.bmp", i),
                image::ImageFormat::Bmp,
            )
            .unwrap();

        let frame_path = format!("output-bmps/output-bmp-{}.bmp", i);

        let frame_path = CString::new(frame_path).unwrap();

        let combined_image = ImlibLoadImage(frame_path.as_ptr() as *const i8);

        combined_images_loaded.push(combined_image);

        info!("Frame {} done!", i);
    }

    std::fs::remove_dir_all("temp-bmps").expect("Failed to remove temporary bitmap directory!");

    combined_images_loaded
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
) {
    let images = composite_images(monitor, display_contexts);

    let num_elements = images.len();
    let mut cycle = 0;

    loop {
        let current_index = cycle % num_elements;

        run(display, monitor, images[current_index]);
        cycle += 1;

        let timeout = Duration::from_nanos(
            (MICROSECONDS_PER_SECOND / 60) * 1_000, // nanoseconds
        );

        std::thread::sleep(timeout);
    }
}
