// SPDX-License-Identifier: BSD-3-Clause
//! Decode screenshots shared by pixel-level integration checks.
use igui::render::{HEIGHT, WIDTH};
use std::{fs, io::Cursor, path::Path};

pub fn read_screenshot(path: &Path) -> Vec<u8> {
    let mut reader = png::Decoder::new(Cursor::new(fs::read(path).unwrap()))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];

    let info = reader.next_frame(&mut pixels).unwrap();

    assert_eq!((info.width, info.height, info.color_type), (WIDTH, HEIGHT, png::ColorType::Rgb));
    pixels.truncate(info.buffer_size());
    pixels
}
