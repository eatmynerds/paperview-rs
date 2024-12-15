use anyhow::anyhow;
use imlib_rs::Imlib_Image;
use std::str::FromStr;

#[derive(Debug)]
pub struct ImageData {
    pub image_path: String,
    pub image_size: (i32, i32),
    pub image_position: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct DisplayContext {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub image_path: String,
    pub fps: f32,
    pub current_image: Imlib_Image,
    pub images: Vec<Imlib_Image>,
}

impl FromStr for DisplayContext {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.split(":");

        let width: i32 = chars
            .next()
            .ok_or_else(|| anyhow!("Failed to parse height!"))?
            .parse()?;

        let height: i32 = chars
            .next()
            .ok_or_else(|| anyhow!("Failed to parse height!"))?
            .parse()?;

        let x: i32 = chars
            .next()
            .ok_or_else(|| anyhow!("Failed to parse x!"))?
            .parse()?;

        let y: i32 = chars
            .next()
            .ok_or_else(|| anyhow!("Failed to parse y!"))?
            .parse()?;

        let image_path: String = chars
            .next()
            .ok_or_else(|| anyhow!("Failed to parse image path!"))?
            .parse()?;

        let fps: f32 = chars
            .next()
            .ok_or_else(|| anyhow!("Failed to parse frames per second!"))?
            .parse()?;

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
