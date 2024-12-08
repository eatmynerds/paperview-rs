#![allow(dead_code)]
extern crate imlib_rs;

use clap::Parser;
use core::mem::{align_of, size_of};
use std::{
    ffi::{c_long, c_uchar, c_ulong, CString},
    fs,
    path::Path,
    time::Duration,
};
use x11::xlib::{Pixmap, Window, XA_PIXMAP};

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

unsafe fn get_monitors() -> (*mut x11::xlib::_XDisplay, Vec<Monitor>) {
    let display = x11::xlib::XOpenDisplay(std::ptr::null());

    let screen_count = x11::xlib::XScreenCount(display);

    let mut monitors: Vec<Monitor> = Vec::with_capacity(screen_count as usize);

    for current_screen in 0..=screen_count {
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

    (display, monitors)
}

unsafe fn set_root_atoms(display: *mut x11::xlib::_XDisplay, monitor: Monitor) {
    let mut r#type: x11::xlib::Atom = 0;

    let mut data_root: *mut c_uchar = c_uchar::from(1) as *mut c_uchar;
    let mut data_eroot: *mut c_uchar = std::ptr::null_mut();
    let mut format: i32 = 0;
    let mut length: c_ulong = 0;
    let mut after: c_ulong = 128;

    let mut atom_root: x11::xlib::Atom = x11::xlib::XInternAtom(
        display,
        CString::new("_XROOTMAP_ID").unwrap().as_ptr() as *const i8,
        true as i32,
    );

    let mut atom_eroot: x11::xlib::Atom = x11::xlib::XInternAtom(
        display,
        CString::new("ESETROOT_PMAP_ID").unwrap().as_ptr() as *const i8,
        true as i32,
    );

    if atom_root != 0 && atom_eroot != 0 {
        x11::xlib::XGetWindowProperty(
            display,
            monitor.root,
            atom_root,
            0 as c_long,
            1 as c_long,
            false as i32,
            x11::xlib::AnyPropertyType as u64,
            &mut r#type as *mut x11::xlib::Atom,
            &mut format as *mut i32,
            &mut length as *mut c_ulong,
            &mut after as *mut c_ulong,
            &mut data_root as *mut *mut c_uchar,
        );
    }

    atom_root = x11::xlib::XInternAtom(
        display,
        CString::new("_XROOTMAP_ID").unwrap().as_ptr() as *const i8,
        false as i32,
    );

    atom_eroot = x11::xlib::XInternAtom(
        display,
        CString::new("ESETROOT_PMAP_ID").unwrap().as_ptr() as *const i8,
        false as i32,
    );

    let pixmap_ptr: *const c_uchar = monitor.pixmap as *const c_uchar;

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_root,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        pixmap_ptr,
        1,
    );

    // x11::xlib::XChangeProperty(
    //     display,
    //     monitor.root,
    //     atom_eroot,
    //     x11::xlib::XA_PIXMAP,
    //     32,
    //     x11::xlib::PropModeReplace,
    //     pixmap_ptr,
    //     1,
    // );
}

unsafe fn run(
    display: *mut x11::xlib::_XDisplay,
    monitors: Vec<Monitor>,
    images_count: usize,
    current: imlib_rs::Imlib_Image,
) {
    for (i, _) in monitors.iter().enumerate() {
        let c_monitor: Monitor = monitors[i];

        imlib_rs::imlib_context_push(c_monitor.render_context);
        imlib_rs::imlib_context_set_dither(1);
        imlib_rs::imlib_context_set_blend(1);
        imlib_rs::imlib_context_set_image(current);

        let original_width = imlib_rs::imlib_image_get_width();
        let original_height = imlib_rs::imlib_image_get_height();

        let scaled_image: imlib_rs::Imlib_Image = imlib_rs::imlib_create_cropped_scaled_image(
            0,
            0,
            original_width,
            original_height,
            c_monitor.width as i32,
            c_monitor.height as i32,
        );

        imlib_rs::imlib_context_set_image(scaled_image);
        imlib_rs::imlib_render_image_on_drawable(0, 0);
        set_root_atoms(display, c_monitor);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliImagePath::parse();

    let image_dir = Path::new(&args.path);

    let images_count = fs::read_dir(image_dir)
        .expect("Failed to open bitmap directory")
        .count();

    println!("Found {} images", images_count);

    let mut images: Vec<imlib_rs::Imlib_Image> = Vec::with_capacity(images_count);

    for i in 0..images_count {
        let image_path = image_dir.join(format!("{}-{}.bmp", args.path, i));

        unsafe {
            let image_path_c_str = CString::new(image_path.to_str().unwrap()).unwrap();
            let image = imlib_rs::imlib_load_image(image_path_c_str.as_ptr() as *const i8);
            images.push(image);
        }
    }

    unsafe {
        let (display, monitors) = get_monitors();

        println!("Starting the program...");

        let mut cycle = 0;

        loop {
            cycle += 1;
            let current: imlib_rs::Imlib_Image = images[cycle % images_count];

            run(display, monitors.clone(), images_count, current);

            let timeout = Duration::from_nanos(
                (MICROSECONDS_PER_SECOND / TARGET_FPS) * 1_000, // nanoseconds
            );

            std::thread::sleep(timeout);
        }
    }
}
