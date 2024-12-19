use std::{
    ffi::CString,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use env_logger::Env;
use imlib_rs::{
    ImlibContextPush, ImlibContextSetBlend, ImlibContextSetDither, ImlibContextSetImage,
    ImlibCreateCroppedScaledImage, ImlibFreeImageAndDecache, ImlibImage, ImlibImageGetHeight,
    ImlibImageGetWidth, ImlibLoadImage, ImlibRenderImageOnDrawable,
};
use log::{debug, error, info, warn};
use signal_hook::{consts::signal::*, iterator::Signals};
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
mod tui;
use tui::{
    display::App,
    path::{get_expanded_path, sort_bitmaps},
    screen::get_screens,
};

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(long)]
    tui: bool,
    #[arg(short, long)]
    bg: Vec<String>,
}

// Add logging to the unsafe `run` function
pub unsafe fn run(display: *mut _XDisplay, monitor: Monitor, background_image: ImlibImage) {
    info!("Setting up rendering context for monitor...");
    ImlibContextPush(monitor.render_context);
    ImlibContextSetDither(1);
    ImlibContextSetBlend(1);
    ImlibContextSetImage(background_image);

    let original_width = ImlibImageGetWidth();
    let original_height = ImlibImageGetHeight();
    debug!(
        "Original image dimensions: {}x{}",
        original_width, original_height
    );

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
    info!("Image rendered on drawable");

    set_root_atoms(display, monitor);

    XSetCloseDownMode(display, RetainTemporary);
    XKillClient(display, AllTemporary as u64);
    XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);
    XClearWindow(display, monitor.root);
    XFlush(display);
    XSync(display, False);
    info!("X11 rendering complete");

    ImlibFreeImageAndDecache();
    debug!("Image resources cleaned up");
}

fn setup_signal_handler(running: Arc<AtomicBool>) {
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("Failed to create signal handler");
    info!("Signal handler setup complete");

    thread::spawn(move || {
        for signal in signals.forever() {
            match signal {
                SIGINT | SIGTERM => {
                    info!("Received termination signal: {:?}", signal);
                    running.store(false, Ordering::SeqCst);
                    break;
                }
                _ => warn!("Unhandled signal: {:?}", signal),
            }
        }
    });
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    info!("Program started");

    let args = CliArgs::parse();
    debug!("Parsed CLI arguments: {:?}", args);

    let mut display_contexts: Vec<DisplayContext> = vec![];

    let running = Arc::new(AtomicBool::new(true));
    setup_signal_handler(running.clone());

    if args.tui {
        info!("Running in TUI mode");
        let mut screens = get_screens();

        color_eyre::install().unwrap();
        let terminal = ratatui::init();
        let options: Vec<String> = screens
            .iter()
            .enumerate()
            .map(|(i, screen)| {
                format!(
                    "Screen {} (Dimensions: {}x{}) (Offset: {}, {}) (FPS: {})",
                    i, screen.width, screen.height, screen.x, screen.y, screen.fps
                )
            })
            .collect();

        let app = App::new(options);
        let paths = app.run(terminal).unwrap();
        ratatui::restore();
        debug!("TUI paths selected: {:?}", paths);

        for (monitor, path) in paths {
            screens[monitor].bitmap_dir = path;
        }

        for mut screen in screens {
            let image_dir = get_expanded_path(&screen.bitmap_dir);
            let bmp_files = sort_bitmaps(&image_dir)?;
            debug!("Bitmap files found: {:?}", bmp_files);

            for bmp_file in bmp_files {
                unsafe {
                    if let Some(image_path_str) = bmp_file.to_str() {
                        let image_path_c_str = CString::new(image_path_str).map_err(|_| {
                            error!("Failed to convert path to C string: {}", bmp_file.display());
                            anyhow!("Failed to convert path to C string")
                        })?;
                        let image = ImlibLoadImage(image_path_c_str.as_ptr() as *const i8);
                        screen.images.push(image);
                    } else {
                        error!("Invalid UTF-8 path: {}", bmp_file.display());
                        return Err(anyhow!("Invalid UTF-8 path"));
                    }
                }
            }

            display_contexts.push(screen);
        }
    } else {
        info!("Running in non-TUI mode with backgrounds: {:?}", args.bg);
        for background in args.bg {
            let mut display_context = DisplayContext::from_str(&background).map_err(|e| {
                error!("Failed to parse background '{}': {}", background, e);
                anyhow!("Failed to parse background")
            })?;

            let image_dir = get_expanded_path(&display_context.bitmap_dir);
            let bmp_files = sort_bitmaps(&image_dir)?;
            debug!("Bitmap files found: {:?}", bmp_files);

            for bmp_file in bmp_files {
                unsafe {
                    if let Some(image_path_str) = bmp_file.to_str() {
                        let image_path_c_str = CString::new(image_path_str).map_err(|_| {
                            error!("Failed to convert path to C string: {}", bmp_file.display());
                            anyhow!("Failed to convert path to C string")
                        })?;
                        let image = ImlibLoadImage(image_path_c_str.as_ptr() as *const i8);
                        display_context.images.push(image);
                    } else {
                        error!("Invalid UTF-8 path: {}", bmp_file.display());
                        return Err(anyhow!("Invalid UTF-8 path"));
                    }
                }
            }

            display_contexts.push(display_context);
        }
    }

    unsafe {
        let (display, monitors) = get_monitors();
        info!("Monitors detected: {:?}", monitors);

        info!("Starting render loop...");
        render(
            display,
            monitors[0],
            display_contexts.clone(),
            running.clone(),
        );

        info!("Render loop terminated. Exiting...");
    }

    Ok(())
}
