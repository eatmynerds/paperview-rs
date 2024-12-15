use imlib_rs::{
    imlib_context_push, imlib_context_set_blend, imlib_context_set_dither, imlib_context_set_image,
    imlib_create_cropped_scaled_image, imlib_free_image_and_decache, imlib_image_get_height,
    imlib_image_get_width, imlib_load_image, imlib_render_image_on_drawable,
};
use x11::xlib::{
    AllTemporary, False, RetainTemporary, XClearWindow, XFlush, XKillClient, XSetCloseDownMode,
    XSetWindowBackgroundPixmap, XSync, _XDisplay,
};

use clap::Parser;
use env_logger::Env;
use log::info;
use std::str::FromStr;
use std::{ffi::CString, fs, path::Path};

mod models;
use models::{DisplayContext, ImageData};
mod render;
use render::{render, set_root_atoms};
mod monitor;
use monitor::{get_monitors, Monitor};

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short, long)]
    bg: Vec<String>,
}

unsafe fn run(display: *mut _XDisplay, monitor: Monitor, background_info: DisplayContext) {
    imlib_context_push(monitor.render_context);
    imlib_context_set_dither(1);
    imlib_context_set_blend(1);
    imlib_context_set_image(background_info.current_image);

    let original_width = imlib_image_get_width();
    let original_height = imlib_image_get_height();

    let scaled_image = imlib_create_cropped_scaled_image(
        0,
        0,
        original_width,
        original_height,
        background_info.width as i32,
        background_info.height as i32,
    );

    imlib_context_set_image(scaled_image);
    imlib_render_image_on_drawable(background_info.x, background_info.y);

    set_root_atoms(display, monitor);

    XSetCloseDownMode(display, RetainTemporary);
    XKillClient(display, AllTemporary as u64);
    XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);
    XClearWindow(display, monitor.root);
    XFlush(display);
    XSync(display, False);

    imlib_free_image_and_decache();
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = CliArgs::parse();

    let mut display_contexts: Vec<DisplayContext> = vec![];

    for background in args.bg {
        let mut bg: DisplayContext = DisplayContext::from_str(background.as_str()).unwrap();

        let image_dir = Path::new(&bg.image_path);

        let images_count = fs::read_dir(image_dir)
            .expect("Failed to open bitmap directory")
            .count();

        for i in 0..images_count {
            let image_path = image_dir.join(format!("{}-{}.bmp", image_dir.display(), i));

            unsafe {
                let image_path_c_str = CString::new(image_path.to_str().unwrap()).unwrap();
                let image = imlib_load_image(image_path_c_str.as_ptr() as *const i8);
                bg.images.push(image);
            }
        }

        display_contexts.push(bg);
    }

    unsafe {
        let (display, monitors) = get_monitors();

        for monitor in monitors {
            info!("Starting render loop");

            info!("Starting the program...");

            render(display, monitor, display_contexts.clone());
        }
    }
}
