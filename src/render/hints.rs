// SPDX-License-Identifier: BSD-3-Clause
//! Footer glyphs: decorate key tokens, not letters in translated action labels.
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};

use super::{text, FONT_7X13};

const CELL_WIDTH: u32 = FONT_7X13.character_size.width;
const ICON_SIZE: u32 = 9;
const ICON_GAP: u32 = 2;

#[derive(Clone, Copy)]
struct Icon {
    rows: [u16; ICON_SIZE as usize],
    color: Rgb888,
}

impl Icon {
    fn for_button(symbol: char) -> Option<Self> {
        let (rows, color) = match symbol {
            'X' => (
                [
                    0x1ff, 0x101, 0x101, 0x101, 0x101, 0x101, 0x101, 0x101, 0x1ff,
                ],
                Rgb888::new(196, 155, 232),
            ),
            'B' => (
                [
                    0x038, 0x044, 0x082, 0x101, 0x101, 0x101, 0x082, 0x044, 0x038,
                ],
                Rgb888::new(240, 120, 120),
            ),
            'Y' => (
                [
                    0x010, 0x028, 0x028, 0x044, 0x044, 0x082, 0x082, 0x101, 0x1ff,
                ],
                Rgb888::new(121, 207, 154),
            ),
            'A' => (
                [
                    0x101, 0x082, 0x044, 0x028, 0x010, 0x028, 0x044, 0x082, 0x101,
                ],
                Rgb888::new(120, 174, 240),
            ),
            _ => return None,
        };
        Some(Self { rows, color })
    }

    fn draw<D: DrawTarget<Color = Rgb888>>(
        self,
        device: &mut D,
        position: Point,
    ) -> Result<(), D::Error> {
        device.draw_iter(self.rows.iter().enumerate().flat_map(|(row, bits)| {
            (0..ICON_SIZE).filter_map(move |col| {
                (bits & (1 << (ICON_SIZE - 1 - col)) != 0).then_some(Pixel(
                    position + Point::new(col as i32, row as i32),
                    self.color,
                ))
            })
        }))
    }
}

#[derive(Clone, Copy)]
struct Glyph {
    symbol: char,
    icon: Option<Icon>,
}

impl Glyph {
    fn width(self) -> u32 {
        CELL_WIDTH + self.icon.map_or(0, |_| ICON_SIZE + 2 * ICON_GAP)
    }

    fn draw<D: DrawTarget<Color = Rgb888>>(
        self,
        device: &mut D,
        position: Point,
        color: Rgb888,
    ) -> Result<(), D::Error> {
        // ISO-8859-5 has no arrows. Keep their existing 7x7 masks in 7x13 cells.
        let arrow = match self.symbol {
            '↑' => Some([8, 28, 42, 8, 8, 8, 8]),
            '↓' => Some([8, 8, 8, 8, 42, 28, 8]),
            '←' => Some([8, 16, 32, 127, 32, 16, 8]),
            '→' => Some([8, 4, 2, 127, 2, 4, 8]),
            _ => None,
        };
        if let Some(rows) = arrow {
            device.draw_iter(rows.iter().enumerate().flat_map(|(row, bits)| {
                (0..7).filter_map(move |col| {
                    (bits & (1 << (6 - col)) != 0)
                        .then_some(Pixel(position + Point::new(col, 3 + row as i32), color))
                })
            }))?;
        } else {
            let mut buffer = [0; 4];
            text(
                device,
                self.symbol.encode_utf8(&mut buffer),
                position.x,
                position.y,
                color,
                true,
            )?;
        }
        if let Some(icon) = self.icon {
            icon.draw(device, position + Point::new((CELL_WIDTH + ICON_GAP) as i32, 2))?;
        }
        Ok(())
    }
}

fn glyphs(value: &str) -> impl Iterator<Item = Glyph> + Clone + '_ {
    value.split_inclusive(" | ").flat_map(|hint| {
        let keys = hint.split_once(' ').map_or("", |(keys, _)| keys);
        hint.char_indices().map(move |(index, symbol)| {
            let end = index + symbol.len_utf8();
            // Only whole slash-separated keys in the first word are buttons.
            let is_key = index < keys.len()
                && (index == 0 || keys.as_bytes()[index - 1] == b'/')
                && (end == keys.len() || keys.as_bytes()[end] == b'/');
            Glyph {
                symbol,
                icon: is_key.then(|| Icon::for_button(symbol)).flatten(),
            }
        })
    })
}

/// Draw the compact icon and text hints for available actions.
pub(super) fn draw<D: DrawTarget<Color = Rgb888>>(
    device: &mut D,
    value: &str,
    mut position: Point,
    max_width: u32,
    color: Rgb888,
) -> Result<(), D::Error> {
    let glyphs = glyphs(value);
    let truncated = glyphs
        .clone()
        .try_fold(max_width, |remaining, glyph| {
            remaining.checked_sub(glyph.width())
        })
        .is_none();
    let dots = if truncated {
        (max_width / CELL_WIDTH).min(3)
    } else {
        0
    };
    let mut remaining = max_width - dots * CELL_WIDTH;
    for glyph in glyphs {
        // The letter and its icon are one unit, including at the ellipsis boundary.
        let Some(rest) = remaining.checked_sub(glyph.width()) else {
            break;
        };
        glyph.draw(device, position, color)?;
        position.x += glyph.width() as i32;
        remaining = rest;
    }
    text(
        device,
        &"..."[..dots as usize],
        position.x,
        position.y,
        color,
        true,
    )
}
