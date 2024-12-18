use std::process::Command;

use regex::Regex;

use crate::DisplayContext;

pub fn get_screens() -> Vec<DisplayContext> {
    let output = Command::new("xrandr")
        .output()
        .expect("Failed to execute xrandr command");

    let xrandr_output = String::from_utf8_lossy(&output.stdout);

    let connected_regex =
        Regex::new(r"(?P<name>\S+)\sconnected\s(?P<resolution>\d+x\d+\+\d+\+\d+)").unwrap();
    let resolution_regex =
        Regex::new(r"(?P<width>\d+)x(?P<height>\d+)\+(?P<x>\d+)\+(?P<y>\d+)").unwrap();
    let fps_regex = Regex::new(r"(?P<fps>\d+\.\d+)\*").unwrap();

    let mut monitors: Vec<DisplayContext> = vec![];

    for cap in connected_regex.captures_iter(&xrandr_output) {
        let resolution = &cap["resolution"];

        if let Some(res_cap) = resolution_regex.captures(resolution) {
            let width: u32 = res_cap["width"].parse().unwrap();
            let height: u32 = res_cap["height"].parse().unwrap();
            let x: u32 = res_cap["x"].parse().unwrap();
            let y: u32 = res_cap["y"].parse().unwrap();

            if let Some(fps_cap) = fps_regex.captures(&xrandr_output) {
                let fps: f32 = fps_cap["fps"].parse().unwrap();

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
        }
    }

    monitors
}
