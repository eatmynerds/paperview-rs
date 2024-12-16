use std::str::FromStr;

use anyhow::anyhow;
use imlib_rs::ImlibImage;

#[derive(Clone, Debug)]
pub struct DisplayContext {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub bitmap_dir: String,
    pub fps: f32,
    pub images: Vec<ImlibImage>,
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

        let bitmap_dir: String = chars
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
            bitmap_dir,
            images: vec![],
        })
    }
}
