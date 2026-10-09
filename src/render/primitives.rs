// SPDX-License-Identifier: BSD-3-Clause
use super::{Baseline, DrawTarget, MonoTextStyle, Pixel, Point, PrimitiveStyle, Rectangle, Rgb888, Size, Text, FONT_10X20, FONT_7X13};
use embedded_graphics::prelude::*;

pub(super) const BG: Rgb888 = Rgb888::new(15, 21, 29);
pub(super) const PANEL: Rgb888 = Rgb888::new(23, 32, 42);
pub(super) const FILTER_PANEL: Rgb888 = Rgb888::new(31, 43, 55);
pub(super) const INSET: Rgb888 = Rgb888::new(18, 26, 35);
pub(super) const LINE: Rgb888 = Rgb888::new(45, 60, 72);
pub(super) const TEXT: Rgb888 = Rgb888::new(230, 234, 231);
pub(super) const MUTED: Rgb888 = Rgb888::new(151, 169, 174);
pub(super) const NUMBER_DIM: Rgb888 = Rgb888::new(89, 108, 117);
pub(super) const ACTIVE: Rgb888 = Rgb888::new(119, 207, 180);
pub(super) const ACTIVE_BG: Rgb888 = Rgb888::new(34, 72, 65);
pub(super) const ERROR_BG: Rgb888 = Rgb888::new(74, 48, 51);
pub(super) const SELECTION: Rgb888 = Rgb888::new(43, 61, 75);

pub(super) fn rectangle(x: i32, y: i32, w: u32, h: u32) -> Rectangle {
    Rectangle::new(Point::new(x, y), Size::new(w, h))
}

pub(super) fn fill<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    rect: Rectangle,
    color: Rgb888,
) -> Result<(), D::Error> {
    d.fill_solid(&rect, color)
}

pub(super) fn border<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    rect: Rectangle,
    color: Rgb888,
    width: u32,
) -> Result<(), D::Error> {
    rect.into_styled(PrimitiveStyle::with_stroke(color, width))
        .draw(d)
}

pub(super) fn text<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    value: &str,
    x: i32,
    y: i32,
    color: Rgb888,
    small: bool,
) -> Result<(), D::Error> {
    let font = if small { &FONT_7X13 } else { &FONT_10X20 };
    Text::with_baseline(
        value,
        Point::new(x, y),
        MonoTextStyle::new(font, color),
        Baseline::Top,
    )
    .draw(d)?;
    Ok(())
}

pub(super) fn star<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    x: i32,
    y: i32,
    color: Rgb888,
) -> Result<(), D::Error> {
    // A 13x13 bit mask: no font glyph, texture or heap allocation.
    const STAR: [u16; 13] = [
        0x0040, 0x0040, 0x00e0, 0x00e0, 0x1fff, 0x0ffe, 0x07fc, 0x03f8, 0x03f8, 0x077c, 0x063c,
        0x0c06, 0x0802,
    ];
    bitmap_icon(d, &STAR, x, y, color)
}

pub(super) fn bitmap_icon<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    mask: &[u16; 13],
    x: i32,
    y: i32,
    color: Rgb888,
) -> Result<(), D::Error> {
    d.draw_iter(mask.iter().enumerate().flat_map(|(row, bits)| {
        (0..13).filter_map(move |col| {
            (bits & (1 << (12 - col)) != 0)
                .then_some(Pixel(Point::new(x + col, y + row as i32), color))
        })
    }))
}

pub(super) fn trash_icon<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    x: i32,
    y: i32,
    color: Rgb888,
) -> Result<(), D::Error> {
    const TRASH: [u16; 13] = [
        0x01f0, 0x0110, 0x1fff, 0x0000, 0x0802, 0x0912, 0x0912, 0x0912, 0x0912, 0x0912, 0x0802,
        0x0802, 0x07fc,
    ];
    bitmap_icon(d, &TRASH, x, y, color)
}

pub(super) fn shorten(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.into()
    } else {
        value
            .chars()
            .take(max.saturating_sub(3))
            .chain("...".chars())
            .collect()
    }
}
