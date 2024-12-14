extern crate imlib_rs;

use clap::Parser;
use env_logger::Env;
use log::info;
use std::str::FromStr;
use std::{ffi::CString, fs, path::Path};

mod models;
use models::{BackgroundInfo, Cast, CliImagePath, ImageData, Monitor};

mod monitor;
mod render;
use monitor::{get_monitors, set_root_atoms};
use render::render;

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

unsafe fn run(
    display: *mut x11::xlib::_XDisplay,
    monitor: Monitor,
    background_info: BackgroundInfo,
) {
    imlib_rs::imlib_context_push(monitor.render_context);
    imlib_rs::imlib_context_set_dither(1);
    imlib_rs::imlib_context_set_blend(1);
    imlib_rs::imlib_context_set_image(background_info.current_image);

    let original_width = imlib_rs::imlib_image_get_width();
    let original_height = imlib_rs::imlib_image_get_height();

    let scaled_image = imlib_rs::imlib_create_cropped_scaled_image(
        0,
        0,
        original_width,
        original_height,
        background_info.width as i32,
        background_info.height as i32,
    );

    imlib_rs::imlib_context_set_image(scaled_image);
    imlib_rs::imlib_render_image_on_drawable(background_info.x, background_info.y);

    set_root_atoms(display, monitor);

    x11::xlib::XSetCloseDownMode(display, x11::xlib::RetainTemporary);
    x11::xlib::XKillClient(display, x11::xlib::AllTemporary as u64);
    x11::xlib::XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);
    x11::xlib::XClearWindow(display, monitor.root);
    x11::xlib::XFlush(display);
    x11::xlib::XSync(display, false as i32);

    imlib_rs::imlib_free_image_and_decache();
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = CliImagePath::parse();

    let mut monitor_background_info: Vec<BackgroundInfo> = vec![];

    for background in args.bg {
        let mut bg: BackgroundInfo = BackgroundInfo::from_str(background.as_str()).unwrap();

        let image_dir = Path::new(&bg.image_path);

        let images_count = fs::read_dir(image_dir)
            .expect("Failed to open bitmap directory")
            .count();

        for i in 0..images_count {
            let image_path = image_dir.join(format!("{}-{}.bmp", image_dir.display(), i));

            unsafe {
                let image_path_c_str = CString::new(image_path.to_str().unwrap()).unwrap();
                let image = imlib_rs::imlib_load_image(image_path_c_str.as_ptr() as *const i8);
                bg.images.push(image);
            }
        }

        monitor_background_info.push(bg);
    }

    unsafe {
        let (display, monitors) = get_monitors();

        for monitor in monitors {
            info!("Starting render loop");

            info!("Starting the program...");

            render(display, monitor, monitor_background_info.clone());
        }
    }
}
