use std::{ffi::CString, str::FromStr};

use anyhow::{anyhow, Result};
use clap::Parser;
use env_logger::Env;
use imlib_rs::{
    ImlibContextPush, ImlibContextSetBlend, ImlibContextSetDither, ImlibContextSetImage,
    ImlibCreateCroppedScaledImage, ImlibFreeImageAndDecache, ImlibImage, ImlibImageGetHeight,
    ImlibImageGetWidth, ImlibLoadImage, ImlibRenderImageOnDrawable,
};
use log::info;
use x11::xlib::{
    AllTemporary, False, RetainTemporary, XClearWindow, XFlush, XKillClient, XSetCloseDownMode,
    XSetWindowBackgroundPixmap, XSync, _XDisplay,
};

mod models;
use models::DisplayContext;
mod render;
use render::{render, set_root_atoms};
mod monitor;
use monitor::{get_monitors, Monitor};
mod bitmap;
use bitmap::{get_expanded_path, sort_bitmaps};

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short, long)]
    bg: Vec<String>,
}

pub unsafe fn run(display: *mut _XDisplay, monitor: Monitor, background_image: ImlibImage) {
    ImlibContextPush(monitor.render_context);
    ImlibContextSetDither(1);
    ImlibContextSetBlend(1);
    ImlibContextSetImage(background_image);

    let original_width = ImlibImageGetWidth();
    let original_height = ImlibImageGetHeight();

    let scaled_image = ImlibCreateCroppedScaledImage(
        0,
        0,
        original_width,
        original_height,
        monitor.width as i32,
        monitor.height as i32,
    );

    ImlibContextSetImage(scaled_image);
    ImlibRenderImageOnDrawable(0, 0);

    set_root_atoms(display, monitor);

    XSetCloseDownMode(display, RetainTemporary);
    XKillClient(display, AllTemporary as u64);
    XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);
    XClearWindow(display, monitor.root);
    XFlush(display);
    XSync(display, False);

    ImlibFreeImageAndDecache();
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = CliArgs::parse();

    let mut display_contexts: Vec<DisplayContext> = vec![];

    for background in args.bg {
        let mut display_context = DisplayContext::from_str(&background)
            .map_err(|e| anyhow!("Failed to parse background '{}': {}", background, e))?;

        let image_dir = get_expanded_path(&display_context.bitmap_dir);
        let bmp_files = sort_bitmaps(&image_dir)?;

        for bmp_file in bmp_files {
            unsafe {
                if let Some(image_path_str) = bmp_file.to_str() {
                    let image_path_c_str = CString::new(image_path_str).map_err(|_| {
                        anyhow!("Failed to convert path to C string: {}", bmp_file.display())
                    })?;
                    let image = ImlibLoadImage(image_path_c_str.as_ptr() as *const i8);
                    display_context.images.push(image);
                } else {
                    return Err(anyhow!("Invalid UTF-8 path: {}", bmp_file.display()).into());
                }
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

    Ok(())
}
