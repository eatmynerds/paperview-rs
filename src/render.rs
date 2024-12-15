use crate::{DisplayContext, ImageData, Monitor};
use image::{Rgba, RgbaImage};
use imlib_rs::{imlib_context_push, imlib_context_set_image, imlib_save_image};
use std::ffi::CString;
use x11::xlib::{False, PropModeReplace, XChangeProperty, XInternAtom, _XDisplay, XA_PIXMAP};

fn combine_images_with_blank(
    image_data: Vec<ImageData>,
    canvas_width: u32,
    canvas_height: u32,
) -> RgbaImage {
    let mut canvas = RgbaImage::from_pixel(canvas_width, canvas_height, Rgba([0, 0, 0, 0]));

    for info in image_data {
        let img = image::open(&info.image_path).unwrap().into_rgba8();

        let resized_image = image::imageops::resize(
            &img,
            info.image_size.0 as u32,
            info.image_size.1 as u32,
            image::imageops::FilterType::Nearest,
        );

        image::imageops::overlay(
            &mut canvas,
            &resized_image,
            info.image_position.0 as i64,
            info.image_position.1 as i64,
        );
    }

    canvas
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
    mut display_context: Vec<DisplayContext>,
) {
    let num_elements = display_context.len();

    imlib_context_push(monitor.render_context);

    let mut cycle = 0;

    loop {
        let mut image_data: Vec<ImageData> = vec![];

        for element in 0..num_elements {
            let current_info = &mut display_context[element];
            current_info.current_image = current_info.images[cycle % current_info.images.len()];

            imlib_context_set_image(current_info.current_image);

            let image_path_str = CString::new(format!("temp-bitmap-{}.bmp", element))
                .expect("Failed to convert filename to c-string!");

            imlib_save_image(image_path_str.as_ptr() as *const i8);

            let temp_bitmap =
                std::fs::canonicalize(format!("temp-bitmap-{}.bmp", element)).unwrap();

            let current_image_path = temp_bitmap.to_str().expect("Failed to convert to string!");

            image_data.push(ImageData {
                image_path: current_image_path.to_string(),
                image_size: (current_info.width, current_info.height),
                image_position: (current_info.x, current_info.y),
            });
        }

        let combined_images = combine_images_with_blank(image_data, monitor.width, monitor.height);

        combined_images
            .save_with_format("output.bmp", image::ImageFormat::Bmp)
            .unwrap();

        println!("done combining images!");

        cycle += 1;
    }

    // loop {
    //     let current_index = cycle % num_elements;

    //     println!("{:#?}", image_data);

    //     // run(display, monitor, current_info.clone());
    //     // cycle += 1;

    //     // let timeout = Duration::from_nanos(
    //     //     (MICROSECONDS_PER_SECOND / current_info.fps as u64) * 1_000, // nanoseconds
    //     // );

    //     // std::thread::sleep(timeout);
    // }
}
