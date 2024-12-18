use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};

pub fn get_expanded_path(input_path: &str) -> PathBuf {
    if input_path.starts_with("~") {
        let home_dir = dirs::home_dir().expect("Could not determine home directory");
        home_dir.join(&input_path[2..])
    } else {
        Path::new(input_path).to_path_buf()
    }
}

pub fn sort_bitmaps(image_dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut bmp_files: Vec<PathBuf> = fs::read_dir(&image_dir)
        .map_err(|e| anyhow!("Failed to read directory '{}': {}", image_dir.display(), e))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("bmp"))
        })
        .collect();

    bmp_files.sort_by(|a, b| {
        let file1 = parse_filename(a);
        let file2 = parse_filename(b);

        file1.cmp(&file2)
    });

    Ok(bmp_files)
}

fn parse_filename(path: &PathBuf) -> usize {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem_str| {
            let digits: String = stem_str
                .chars()
                .filter_map(|c| c.to_digit(10))
                .map(|d| char::from_digit(d, 10).unwrap())
                .collect();

            if digits.is_empty() {
                None
            } else {
                Some(digits)
            }
        })
        .and_then(|num_str| num_str.parse::<usize>().ok())
        .unwrap_or(usize::MAX)
}
