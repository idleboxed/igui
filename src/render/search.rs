// SPDX-License-Identifier: BSD-3-Clause
use super::*;
use crate::model::search::{COLUMNS, KEY_ROWS};

pub(super) fn icon<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    x: i32,
    y: i32,
    color: Rgb888,
) -> Result<(), D::Error> {
    const GLASS: [u16; 13] = [
        0x0780, 0x0840, 0x1020, 0x1020, 0x1020, 0x1020, 0x0840,
        0x0780, 0x0060, 0x0030, 0x0018, 0x000c, 0x0006,
    ];
    bitmap_icon(d, &GLASS, x, y, color)
}

impl Renderer {
    pub(super) fn search<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        notice: Option<&str>,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let search = app.search().expect("open search");
        text(d, app.text("Search"), 668, 98, ACTIVE, false)?;
        let field = rectangle(668, 132, 572, 48);
        fill(d, field, INSET)?;
        let field_color = if app.focus() == Focus::SearchField { TEXT } else { LINE };
        border(d, field, field_color, 1)?;
        // The non-blinking caret avoids periodic redraws. Leave a cell for it at EOF.
        const VISIBLE: usize = 54;
        let start = search.cursor().saturating_sub(VISIBLE - 1);
        let end = (start + VISIBLE).min(search.query().len());
        text(d, &search.query()[start..end], 684, 146, TEXT, false)?;
        let cursor_x = 684 + (search.cursor() - start) as i32 * 10;
        fill(d, rectangle(cursor_x, 144, 1, 24), ACTIVE)?;

        for (row, keys) in KEY_ROWS.iter().enumerate() {
            for (column, key) in keys.chars().enumerate() {
                let x = 668 + column as i32 * 58;
                let y = 204 + row as i32 * 62;
                let width = if key == ' ' {
                    (COLUMNS - column) as u32 * 58 - 8
                } else {
                    50
                };
                let rect = rectangle(x, y, width, 50);
                let focused = app.focus() == Focus::SearchKeys && search.key_position() == (row, column);
                fill(d, rect, if focused { SELECTION } else { INSET })?;
                let label = if key == ' ' {
                    app.text("Space").to_owned()
                } else {
                    key.to_string()
                };
                text(
                    d,
                    &label,
                    x + (width as i32 - label.chars().count() as i32 * 10) / 2,
                    y + 15,
                    TEXT,
                    false,
                )?;
                if focused {
                    border(d, rect, TEXT, 2)?;
                }
            }
        }
        if let Some(message) = notice {
            draw_notice(d, app, message)?;
        }
        Ok(())
    }
}
