use crate::DisplayContext;
use regex::Regex;
use std::process::Command;

pub fn get_screens() -> Vec<DisplayContext> {
    let output = Command::new("xrandr")
        .output()
        .expect("failed to execute xrandr command");

    let xrandr_output = String::from_utf8_lossy(&output.stdout);

    let mut monitors: Vec<DisplayContext> = vec![];

    let screen_re = Regex::new(r"(?<width>\d+)x(?<height>\d+)\+(?<x>\d+)\+(?<y>\d+)").unwrap();
    let fps_re = Regex::new(r"(?<fps>\d+.\d+)\*").unwrap();

    let caps = screen_re
        .captures_iter(&xrandr_output)
        .map(|c| c.extract())
        .zip(fps_re.captures_iter(&xrandr_output).map(|c| c.extract()));

    for ((_, [width, height, x, y]), (_, [fps])) in caps {
        monitors.push(DisplayContext {
            width: width.parse().unwrap(),
            height: height.parse().unwrap(),
            x: x.parse().unwrap(),
            y: y.parse().unwrap(),
            bitmap_dir: String::new(),
            fps: fps.parse().unwrap(),
            images: vec![],
        });
    }

    monitors
}
