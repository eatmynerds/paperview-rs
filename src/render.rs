use std::{
    ffi::{c_void, CString},
    ops::DerefMut,
};

use image::ImageBuffer;
use image::{DynamicImage, GenericImage, GenericImageView, Rgba, RgbaImage};
use imlib_rs::{
    imlib_create_image, imlib_image_get_data, imlib_image_get_height, imlib_image_get_width,
    imlib_image_put_back_data, ImlibContextPush, ImlibContextSetImage,
    ImlibCreateCroppedScaledImage, ImlibFreeImageAndDecache, ImlibImage, ImlibLoadImage,
    ImlibSaveImage,
};
use log::info;
use rayon::prelude::*;
use std::sync::Mutex;
use std::time::Duration;
use x11::xlib::{False, PropModeReplace, XChangeProperty, XInternAtom, _XDisplay, XA_PIXMAP};

use crate::{run, DisplayContext, ImageData, Monitor, MICROSECONDS_PER_SECOND};

fn combine_images(
    index: usize,
    image_position: (i32, i32),
    image_size: (i32, i32),
    monitor: &Monitor,
    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    image_data: &[u32],
) {
    let updated_image_data = image_data
        .iter()
        .flat_map(|data| {
            vec![
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
    info!("Creating bitmap output directory");

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

        if frame_combo
            .iter()
            .zip(&max_frames)
            .all(|(frame, &max)| *frame == max)
        {
            break;
        }
    }

    let mut output_frames = vec![];

    for (i, frame_combo) in all_frame_combos.iter().enumerate() {
        let mut canvas = RgbaImage::from_pixel(
            monitor.width as u32,
            monitor.height as u32,
            Rgba([0, 0, 0, 0]),
        );

        for (i, frame) in frame_combo.iter().enumerate().rev() {
            let current_image = display_contexts[i].images[*frame];

            ImlibContextSetImage(current_image);

            let image_height = imlib_image_get_height();
            let image_width = imlib_image_get_width();

            let scaled_image = ImlibCreateCroppedScaledImage(
                0,
                0,
                image_width,
                image_height,
                monitor.width as i32 / frame_combo.len() as i32,
                monitor.height as i32,
            );

            ImlibContextSetImage(scaled_image);

            let updated_image_height = imlib_image_get_height();
            let updated_image_width = imlib_image_get_width();

            let temp_image_data = std::slice::from_raw_parts(
                imlib_image_get_data(),
                (monitor.width / frame_combo.len() as u32 * monitor.height) as usize,
            );
            combine_images(
                i,
                (display_contexts[i].x, display_contexts[i].y),
                (updated_image_width, updated_image_height),
                &monitor,
                &mut canvas,
                temp_image_data,
            );
        }

        println!("Frame {} done!", i);

        canvas
            .save_with_format(
                format!("output-bmps/output-bmp-{}.bmp", i),
                image::ImageFormat::Bmp,
            )
            .unwrap();

        output_frames.push(ImlibLoadImage(
            CString::new(format!("output-bmps/output-bmp-{}.bmp", i))
                .unwrap()
                .as_ptr() as *const i8,
        ));
    }

    output_frames
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
