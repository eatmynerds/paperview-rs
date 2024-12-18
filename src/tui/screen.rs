use std::process::Command;

use regex::Regex;

use crate::DisplayContext;

pub fn get_screens() -> Vec<DisplayContext> {
    let output = Command::new("xrandr")
        .output()
        .expect("Failed to execute xrandr command");

    let xrandr_output = String::from_utf8_lossy(&output.stdout);

    let combined_regex = Regex::new(
    r"(?P<name>\S+)\sconnected(\sprimary)?\s(?P<resolution>(?P<width>\d+)x(?P<height>\d+)\+(?P<x>\d+)\+(?P<y>\d+)).*?\n\s+(?P<current_resolution>(?P<current_width>\d+)x(?P<current_height>\d+))\s+(?P<fps>\d+\.\d+|\d+)\*"
).unwrap();

    let mut monitors: Vec<DisplayContext> = vec![];

    for cap in combined_regex.captures_iter(&xrandr_output) {
        let width: u32 = cap["width"].parse().unwrap();
        let height: u32 = cap["height"].parse().unwrap();
        let x: u32 = cap["x"].parse().unwrap();
        let y: u32 = cap["y"].parse().unwrap();

        let fps: f32 = cap["fps"].parse().unwrap();

        monitors.push(DisplayContext {
            width: width as i32,
            height: height as i32,
            x: x as i32,
            y: y as i32,
            bitmap_dir: String::new(),
            fps,
            images: vec![],
        });
    }

    monitors
}
