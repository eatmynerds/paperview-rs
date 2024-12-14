use clap::Parser;
use core::mem::{align_of, size_of};
use std::str::FromStr;
use x11::xlib::{Pixmap, Window};

#[derive(Debug)]
pub struct ImageData {
    pub image_path: String,
    pub image_size: (i32, i32),
    pub image_position: (i32, i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    pub root: Window,
    pub pixmap: Pixmap,
    pub width: u32,
    pub height: u32,
    pub render_context: imlib_rs::Imlib_Context,
}

#[derive(Clone, Debug)]
pub struct BackgroundInfo {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub image_path: String,
    pub fps: f32,
    pub current_image: imlib_rs::Imlib_Image,
    pub images: Vec<imlib_rs::Imlib_Image>,
}

impl FromStr for BackgroundInfo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.split(":");
        let width: i32 = chars
            .next()
            .expect("Failed to parse width!")
            .parse()
            .expect("Incorrect input!");
        let height: i32 = chars
            .next()
            .expect("Failed to parse height!")
            .parse()
            .expect("Incorrect input!");
        let x: i32 = chars
            .next()
            .expect("Failed to parse x!")
            .parse()
            .expect("Incorrect Input!");
        let y: i32 = chars
            .next()
            .expect("Failed to parse y!")
            .parse()
            .expect("Incorrect Input!");
        let image_path = chars.next().expect("Failed to parse image path!");
        let fps: f32 = chars
            .next()
            .expect("Failed to parse frames per second!")
            .parse()
            .expect("Incorrect Input!");

        Ok(Self {
            width,
            height,
            x,
            y,
            fps,
            image_path: image_path.to_string(),
            current_image: std::ptr::null_mut(),
            images: vec![],
        })
    }
}

#[derive(Parser, Debug)]
pub struct CliImagePath {
    #[arg(short)]
    pub bg: Vec<String>,
}

pub struct Cast<A, B>((A, B));
impl<A, B> Cast<A, B> {
    pub const ASSERT_ALIGN_GREATER_THAN_EQUAL: () = assert!(align_of::<A>() >= align_of::<B>());
    pub const ASSERT_SIZE_EQUAL: () = assert!(size_of::<A>() == size_of::<B>());

    pub fn safe_ptr_cast(a: *mut A) -> *mut B {
        let _ = Self::ASSERT_SIZE_EQUAL;
        let _ = Self::ASSERT_ALIGN_GREATER_THAN_EQUAL;

        a.cast()
    }
}
