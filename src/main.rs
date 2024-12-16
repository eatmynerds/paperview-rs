use std::{ffi::CString, fs, path::Path, str::FromStr};

use clap::Parser;
use env_logger::Env;
use imlib_rs::{
    ImlibContextPush, ImlibContextSetBlend, ImlibContextSetDither, ImlibContextSetImage,
    ImlibCreateCroppedScaledImage, ImlibFreeImageAndDecache, ImlibImageGetHeight,
    ImlibImageGetWidth, ImlibLoadImage, ImlibRenderImageOnDrawable,
};
use log::info;
use x11::xlib::{
    AllTemporary, False, RetainTemporary, XClearWindow, XFlush, XKillClient, XSetCloseDownMode,
    XSetWindowBackgroundPixmap, XSync, _XDisplay,
};

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
    ImlibContextPush(monitor.render_context);
    ImlibContextSetDither(1);
    ImlibContextSetBlend(1);
    ImlibContextSetImage(background_info.current_image);

    let original_width = ImlibImageGetWidth();
    let original_height = ImlibImageGetHeight();

    let scaled_image = ImlibCreateCroppedScaledImage(
        0,
        0,
        original_width,
        original_height,
        background_info.width as i32,
        background_info.height as i32,
    );

    ImlibContextSetImage(scaled_image);
    ImlibRenderImageOnDrawable(background_info.x, background_info.y);

    set_root_atoms(display, monitor);

    XSetCloseDownMode(display, RetainTemporary);
    XKillClient(display, AllTemporary as u64);
    XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);
    XClearWindow(display, monitor.root);
    XFlush(display);
    XSync(display, False);

    ImlibFreeImageAndDecache();
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = CliArgs::parse();

    let mut display_contexts: Vec<DisplayContext> = vec![];

    for background in args.bg {
        let mut display_context: DisplayContext =
            DisplayContext::from_str(background.as_str()).unwrap();

        let image_dir = Path::new(&display_context.bitmap_dir);

        let images_count = fs::read_dir(image_dir)
            .expect("Failed to open bitmap directory")
            .count();

        for i in 0..images_count {
            let image_path = image_dir.join(format!("{}-{}.bmp", image_dir.display(), i));

            unsafe {
                let image_path_c_str = CString::new(image_path.to_str().unwrap()).unwrap();
                let image = ImlibLoadImage(image_path_c_str.as_ptr() as *const i8);
                display_context.images.push(image);
            }
        }

        display_contexts.push(display_context);
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
