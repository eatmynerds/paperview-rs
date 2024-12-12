extern crate imlib_rs;

use clap::Parser;
use core::mem::{align_of, size_of};
use display_info::DisplayInfo;
use env_logger::Env;
use log::{error, info};
use std::{ffi::CString, fs, path::Path, time::Duration};
use x11::xlib::{Pixmap, Window};
use std::str::FromStr;
use image::{Rgba, RgbaImage};

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;

#[derive(Debug)]
struct ImageData {
    image_path: String,
    image_size: (i32, i32),
    image_position: (i32, i32)
}

#[derive(Clone, Copy, Debug)]
struct Monitor {
    root: Window,
    pixmap: Pixmap,
    width: u32,
    height: u32,
    render_context: imlib_rs::Imlib_Context,
}

#[derive(Clone, Debug)]
struct BackgroundInfo {
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    image_path: String,
    fps: f32,
    current_image: imlib_rs::Imlib_Image,
    images: Vec<imlib_rs::Imlib_Image>
}

impl FromStr for BackgroundInfo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.split(":");
        let width: i32 = chars.next().expect("Failed to parse width!").parse().expect("Incorrect input!");
        let height: i32 = chars.next().expect("Failed to parse height!").parse().expect("Incorrect input!");
        let x: i32 = chars.next().expect("Failed to parse x!").parse().expect("Incorrect Input!");
        let y: i32 = chars.next().expect("Failed to parse y!").parse().expect("Incorrect Input!");
        let image_path = chars.next().expect("Failed to parse image path!");
        let fps: f32 = chars.next().expect("Failed to parse frames per second!").parse().expect("Incorrect Input!");

        Ok(Self {
            width,
            height,
            x,
            y,
            fps,
            image_path: image_path.to_string(),
            current_image: std::ptr::null_mut(),
            images: vec![]
        })
    }
}

#[derive(Parser, Debug)]
struct CliImagePath {
    #[arg(short)]
    bg: Vec<String>,
}

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
            info.image_position.1 as i64
        );
    }

    canvas
}


unsafe fn set_root_atoms(display: *mut x11::xlib::_XDisplay, monitor: Monitor) {
    let atom_root = x11::xlib::XInternAtom(
        display,
        c"_XROOTPMAP_ID".as_ptr() as *const i8,
        false as i32,
    );

    let atom_eroot = x11::xlib::XInternAtom(
        display,
        c"ESETROOT_PMAP_ID".as_ptr() as *const i8,
        false as i32,
    );

    let monitor_pixmap = monitor.pixmap as u64;

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_root,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_eroot,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );
}

unsafe fn get_monitors() -> (*mut x11::xlib::_XDisplay, Vec<Monitor>) {
    let display = x11::xlib::XOpenDisplay(std::ptr::null());

    let screen_count = x11::xlib::XScreenCount(display);

    info!("Found {} screens", screen_count);

    let mut monitors: Vec<Monitor> = Vec::with_capacity(screen_count as usize);

    for current_screen  in 0..screen_count {
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
            width: width as u32,
            height: height as u32,
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
    monitor: Monitor,
    mut background_info: BackgroundInfo
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

unsafe fn render(
    display: *mut x11::xlib::_XDisplay,
    monitor: Monitor,
    mut monitor_background_info: Vec<BackgroundInfo>
) {
    let mut cycle = 0;
    let num_elements = monitor_background_info.len();
    
    imlib_rs::imlib_context_push(monitor.render_context);

    loop {
        let current_index = cycle % num_elements;
        let current_info = &mut monitor_background_info[current_index];

        current_info.current_image = current_info.images[cycle % current_info.images.len()];
        imlib_rs::imlib_context_image(current_info.current_image);

        imlib_rs::imlib_save_image(c"temp.bmp".as_ptr() as *const i8);

        // let image_data = vec![ImageData {
        //     image_path: current_info.current_image, 
        //     image_size: (current_info.width, current_info.height),
        //     image_position: (current_info.x, current_info.y)
        // }];

        // println!("{:#?}", image_data);

        // let combined_images  =combine_images_with_blank(image_data, monitor.width, monitor.height);

        // run(display, monitor, current_info.clone());
        // cycle += 1;

        // let timeout = Duration::from_nanos(
        //     (MICROSECONDS_PER_SECOND / current_info.fps as u64) * 1_000, // nanoseconds
        // );

        // std::thread::sleep(timeout);
    }
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
