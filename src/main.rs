extern crate imlib_rs;

use clap::Parser;
use core::mem::{align_of, size_of};
use env_logger::Env;
use log::info;
use std::{ffi::CString, fs, path::Path, time::Duration};
use x11::xlib::{Pixmap, Window};

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;
const TARGET_FPS: u64 = 60;

#[derive(Clone, Copy, Debug)]
struct Monitor {
    root: Window,
    pixmap: Pixmap,
    width: usize,
    height: usize,
    render_context: imlib_rs::Imlib_Context,
}

#[derive(Parser, Debug)]
struct CliImagePath {
    #[arg(
        short,
        long,
        help = "Path to the directory containing the bitmap images"
    )]
    path: String,
}

/// Struct for safe casts, from bytemuck
struct Cast<A, B>((A, B));
impl<A, B> Cast<A, B> {
    const ASSERT_ALIGN_GREATER_THAN_EQUAL: () = assert!(align_of::<A>() >= align_of::<B>());
    const ASSERT_SIZE_EQUAL: () = assert!(size_of::<A>() == size_of::<B>());
}

fn safe_ptr_cast<A, B>(a: *mut A) -> *mut B {
    let _ = Cast::<A, B>::ASSERT_SIZE_EQUAL;
    let _ = Cast::<A, B>::ASSERT_ALIGN_GREATER_THAN_EQUAL;

    a.cast()
}

unsafe fn set_root_atoms(display: *mut x11::xlib::_XDisplay, monitor: &Monitor) {
    let atom_root: x11::xlib::Atom = x11::xlib::XInternAtom(
        display,
        CString::new("_XROOTPMAP_ID").unwrap().as_ptr() as *const i8,
        false as i32,
    );

    let atom_eroot: x11::xlib::Atom = x11::xlib::XInternAtom(
        display,
        CString::new("ESETROOT_PMAP_ID").unwrap().as_ptr() as *const i8,
        false as i32,
    );

    let monitor_pixmap = monitor.pixmap as u8;

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_root,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        &monitor_pixmap as *const u8,
        1,
    );

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_eroot,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        &monitor_pixmap as *const u8,
        1,
    );
}

unsafe fn get_monitors() -> (*mut x11::xlib::_XDisplay, Vec<Monitor>) {
    let display = x11::xlib::XOpenDisplay(std::ptr::null());

    let screen_count = x11::xlib::XScreenCount(display);

    info!("Found {} screens", screen_count);

    let mut monitors: Vec<Monitor> = Vec::with_capacity(screen_count as usize);

    for current_screen in 0..=screen_count - 1 {
        info!("Running screen {}", current_screen);

        let width = x11::xlib::XDisplayWidth(display, current_screen);
        let height = x11::xlib::XDisplayHeight(display, current_screen);
        let depth = x11::xlib::XDefaultDepth(display, current_screen);
        let visual = x11::xlib::XDefaultVisual(display, current_screen);
        if visual as usize == 0x8 {
            // TODO: Total insanity because for some reason for my second monitor it just
            // returns 0x8 and segfaults on imlib_context_set_visual
            continue;
        }

        let cm = x11::xlib::XDefaultColormap(display, current_screen);
        info!(
            "Screen {}: width: {}, height: {}, depth: {}",
            current_screen, width, height, depth
        );

        let root = x11::xlib::XRootWindow(display, current_screen);
        let pixmap =
            x11::xlib::XCreatePixmap(display, root, width as u32, height as u32, depth as u32);

        monitors.push(Monitor {
            root,
            pixmap,
            width: width as usize,
            height: height as usize,
            render_context: imlib_rs::imlib_context_new(),
        });

        imlib_rs::imlib_context_push(monitors[current_screen as usize].render_context);
        imlib_rs::imlib_context_set_display(display.cast());
        imlib_rs::imlib_context_set_visual(safe_ptr_cast(visual));
        imlib_rs::imlib_context_set_colormap(cm);
        imlib_rs::imlib_context_set_drawable(pixmap);
        imlib_rs::imlib_context_set_color_range(imlib_rs::imlib_create_color_range());
        imlib_rs::imlib_context_pop();
    }

    info!("Loaded {} screens", screen_count);

    (display, monitors)
}

unsafe fn run(
    display: *mut x11::xlib::_XDisplay,
    monitors: &[Monitor],
    current_image: imlib_rs::Imlib_Image,
) {
    for monitor in monitors {
        imlib_rs::imlib_context_push(monitor.render_context);
        imlib_rs::imlib_context_set_dither(1);
        imlib_rs::imlib_context_set_blend(1);
        imlib_rs::imlib_context_set_image(current_image);

        let original_width = imlib_rs::imlib_image_get_width();
        let original_height = imlib_rs::imlib_image_get_height();

        let scaled_image = imlib_rs::imlib_create_cropped_scaled_image(
            0,
            0,
            original_width,
            original_height,
            monitor.width as i32,
            monitor.height as i32,
        );

        imlib_rs::imlib_context_set_image(scaled_image);
        imlib_rs::imlib_render_image_on_drawable(0, 0);

        set_root_atoms(display, monitor);

        // ----

        x11::xlib::XKillClient(display, x11::xlib::AllTemporary as u64);
        x11::xlib::XSetCloseDownMode(display, x11::xlib::RetainTemporary);
        x11::xlib::XSetWindowBackgroundPixmap(display, monitor.root, monitor.pixmap);
        x11::xlib::XClearWindow(display, monitor.root);
        x11::xlib::XFlush(display);
        x11::xlib::XSync(display, false as i32);

        imlib_rs::imlib_free_image_and_decache();
        // ----
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = CliImagePath::parse();

    info!("Loading images");

    let image_dir = Path::new(&args.path);

    let images_count = fs::read_dir(image_dir)
        .expect("Failed to open bitmap directory")
        .count();

    let mut images: Vec<imlib_rs::Imlib_Image> = Vec::with_capacity(images_count);

    for i in 0..images_count {
        let image_path = image_dir.join(format!("{}-{}.bmp", args.path, i));

        unsafe {
            let image_path_c_str = CString::new(image_path.to_str().unwrap()).unwrap();
            let image = imlib_rs::imlib_load_image(image_path_c_str.as_ptr() as *const i8);
            images.push(image);
        }
    }

    info!("Loading monitors");

    unsafe {
        let (display, monitors) = get_monitors();

        info!("Starting render loop");

        info!("Starting the program...");

        let mut cycle = 0;

        loop {
            cycle += 1;
            let current: imlib_rs::Imlib_Image = images[cycle % images_count];

            /* TODO: Figure out why it does not render with picom
            running while the C version does */
            run(display, &monitors.clone(), current);

            let timeout = Duration::from_nanos(
                (MICROSECONDS_PER_SECOND / TARGET_FPS) * 1_000, // nanoseconds
            );

            std::thread::sleep(timeout);
        }
    }
}
