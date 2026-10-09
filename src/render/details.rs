// SPDX-License-Identifier: BSD-3-Clause
use super::*;

pub(super) fn title_width(value: &str) -> usize {
    value.chars().count().saturating_mul(FONT_10X20.character_size.width as usize)
}

// Bound glyph work to the viewport, not the off-screen title length. No full
// title bitmap, extra Surface, abbreviation or preview work is needed per tick.
pub(super) fn draw_card_title<D: DrawTarget<Color = Rgb888>>(
    value: &str,
    content_width: usize,
    offset: usize,
    d: &mut D,
) -> Result<(), D::Error> {
    let bounds = rectangle(card::TITLE_X, card::TITLE_Y, card::TITLE_WIDTH, card::TITLE_HEIGHT);
    fill(d, bounds, INSET)?;
    let advance = FONT_10X20.character_size.width as usize;
    let columns = (card::TITLE_WIDTH as usize).div_ceil(advance) + 1;
    let visible: String = value.chars().skip(offset / advance).take(columns).collect();
    let mut clipped = d.clipped(&bounds);
    text(
        &mut clipped,
        &visible,
        card::TITLE_X - (offset % advance) as i32,
        card::TITLE_Y,
        ACTIVE,
        false,
    )?;
    if content_width > card::TITLE_WIDTH as usize {
        let repeat_x = content_width.saturating_add(crate::render::marquee::GAP_PIXELS).saturating_sub(offset);
        if repeat_x < card::TITLE_WIDTH as usize {
            let visible: String = value.chars().take(columns).collect();
            text(
                &mut clipped,
                &visible,
                card::TITLE_X + repeat_x as i32,
                card::TITLE_Y,
                ACTIVE,
                false,
            )?;
        }
    }
    Ok(())
}

impl Renderer {
    pub(super) fn right<D: DrawTarget<Color = Rgb888>>(
        &mut self,
        app: &App,
        notice: Option<&str>,
        d: &mut D,
    ) -> Result<(), D::Error> {
        fill(d, rectangle(644, 80, 620, 592), PANEL)?;
        if app.search().is_some() {
            // Release the hidden image rather than caching artwork for search results.
            self.prepare_selected(app);
            self.search(app, notice, d)
        } else {
            self.preview(app, d)?;
            self.metadata(app, notice, d)
        }
    }
    pub(super) fn metadata<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        notice: Option<&str>,
        d: &mut D,
    ) -> Result<(), D::Error> {
        fill(d, rectangle(656, card::TOP, 596, card::HEIGHT), INSET)?;
        let Some(game) = app.selected_game() else {
            if let Some(message) = notice {
                draw_notice(d, app, message)?;
            }
            return Ok(());
        };
        text(d, app.text("Title"), 676, card::TITLE_LABEL_Y, MUTED, true)?;
        draw_card_title(&game.title, title_width(&game.title), 0, d)?;
        text(
            d,
            app.text("Genre"),
            card::GENRE_X,
            card::GENRE_LABEL_Y,
            MUTED,
            true,
        )?;
        text(
            d,
            &shorten(
                app.text(game.genre.as_deref().unwrap_or("-")),
                card::GENRE_REGION_COLUMNS,
            ),
            card::GENRE_X,
            card::GENRE_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(
            d,
            app.text("Region"),
            card::REGION_X,
            card::GENRE_LABEL_Y,
            MUTED,
            true,
        )?;
        let region = match game.region.as_deref() {
            None => "?".into(),
            Some([]) => "-".into(),
            Some(regions) => regions
                .iter()
                .map(|region| app.text(region.trim()))
                .collect::<Vec<_>>()
                .join(", "),
        };
        text(
            d,
            &shorten(&region, card::GENRE_REGION_COLUMNS),
            card::REGION_X,
            card::GENRE_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(d, app.text("Year"), 676, card::FACTS_LABEL_Y, MUTED, true)?;
        text(
            d,
            app.text("Players"),
            796,
            card::FACTS_LABEL_Y,
            MUTED,
            true,
        )?;
        text(d, app.text("Image"), 916, card::FACTS_LABEL_Y, MUTED, true)?;
        text(
            d,
            &game.year.map_or_else(|| "-".into(), |v| v.to_string()),
            676,
            card::FACTS_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(
            d,
            &game.players.map_or_else(|| "-".into(), |v| v.to_string()),
            796,
            card::FACTS_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(
            d,
            &shorten(&game.image, 44),
            916,
            card::FACTS_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            true,
        )?;
        text(
            d,
            app.text("Description"),
            676,
            card::DESCRIPTION_LABEL_Y,
            MUTED,
            true,
        )?;
        if let Some(message) = notice {
            // An error replaces the description, including its scrollbar.
            return draw_notice(d, app, message);
        }
        if app.description_lines().is_empty() {
            text(
                d,
                app.text("No description"),
                676,
                card::DESCRIPTION_Y,
                MUTED,
                true,
            )?;
        } else {
            for (row, line) in app
                .description_lines()
                .iter()
                .skip(app.description_offset())
                .take(description::ROWS)
                .enumerate()
            {
                text(
                    d,
                    line,
                    676,
                    card::DESCRIPTION_Y + row as i32 * card::DESCRIPTION_LINE_HEIGHT as i32,
                    TEXT,
                    true,
                )?;
            }
            let max = app.description_max_offset();
            if max > 0 {
                let viewport_height = card::DESCRIPTION_HEIGHT;
                fill(
                    d,
                    rectangle(1236, card::DESCRIPTION_Y, 3, viewport_height),
                    LINE,
                )?;
                let height = (viewport_height as usize * description::ROWS
                    / app.description_lines().len())
                .max(5) as u32;
                let top = (viewport_height - height) as usize * app.description_offset() / max;
                fill(
                    d,
                    rectangle(1236, card::DESCRIPTION_Y + top as i32, 3, height),
                    ACTIVE,
                )?;
            }
        }
        Ok(())
    }
}
