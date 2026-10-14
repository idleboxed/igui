// SPDX-License-Identifier: BSD-3-Clause
use super::*;

/// Measure the title's pixel width with the card font.
pub(super) fn title_width(value: &str) -> usize {
    value.chars().count().saturating_mul(FONT_10X20.character_size.width as usize)
}

// Bound glyph work to the viewport, not the off-screen title length. No full
// title bitmap, extra Surface, abbreviation or preview work is needed per tick.
/// Draw a clipped detail-card title at the supplied scrolling offset.
pub(super) fn draw_card_title<D: DrawTarget<Color = Rgb888>>(
    value: &str,
    content_width: usize,
    offset: usize,
    device: &mut D,
) -> Result<(), D::Error> {
    let bounds = rectangle(card::TITLE_X, card::TITLE_Y, card::TITLE_WIDTH, card::TITLE_HEIGHT);
    fill(device, bounds, INSET)?;
    let advance = FONT_10X20.character_size.width as usize;
    let columns = (card::TITLE_WIDTH as usize).div_ceil(advance) + 1;
    let visible: String = value.chars().skip(offset / advance).take(columns).collect();
    let mut clipped = device.clipped(&bounds);
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
    /// Draw the right-hand detail or search panel and any notice.
    pub(super) fn right<D: DrawTarget<Color = Rgb888>>(
        &mut self,
        app: &App,
        notice: Option<&str>,
        device: &mut D,
    ) -> Result<(), D::Error> {
        fill(device, rectangle(644, 80, 620, 592), PANEL)?;
        if app.search().is_some() {
            // Release the hidden image rather than caching artwork for search results.
            self.prepare_selected(app);
            self.search(app, notice, device)
        } else {
            self.preview(app, device)?;
            self.metadata(app, notice, device)
        }
    }
    /// Draw the selected game's metadata and wrapped description.
    pub(super) fn metadata<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        notice: Option<&str>,
        device: &mut D,
    ) -> Result<(), D::Error> {
        fill(device, rectangle(656, card::TOP, 596, card::HEIGHT), INSET)?;
        let Some(game) = app.selected_game() else {
            if let Some(message) = notice {
                draw_notice(device, app, message)?;
            }
            return Ok(());
        };
        text(device, app.text("Title"), 676, card::TITLE_LABEL_Y, MUTED, true)?;
        draw_card_title(&game.title, title_width(&game.title), 0, device)?;
        text(
            device,
            app.text("Genre"),
            card::GENRE_X,
            card::GENRE_LABEL_Y,
            MUTED,
            true,
        )?;
        text(
            device,
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
            device,
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
            device,
            &shorten(&region, card::GENRE_REGION_COLUMNS),
            card::REGION_X,
            card::GENRE_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(device, app.text("Year"), 676, card::FACTS_LABEL_Y, MUTED, true)?;
        text(
            device,
            app.text("Players"),
            796,
            card::FACTS_LABEL_Y,
            MUTED,
            true,
        )?;
        text(device, app.text("Image"), 916, card::FACTS_LABEL_Y, MUTED, true)?;
        text(
            device,
            &game.year.map_or_else(|| "-".into(), |value_item| value_item.to_string()),
            676,
            card::FACTS_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(
            device,
            &game.players.map_or_else(|| "-".into(), |value_item| value_item.to_string()),
            796,
            card::FACTS_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            false,
        )?;
        text(
            device,
            &shorten(&game.image, 44),
            916,
            card::FACTS_LABEL_Y + card::VALUE_OFFSET,
            TEXT,
            true,
        )?;
        text(
            device,
            app.text("Description"),
            676,
            card::DESCRIPTION_LABEL_Y,
            MUTED,
            true,
        )?;
        if let Some(message) = notice {
            // An error replaces the description, including its scrollbar.
            return draw_notice(device, app, message);
        }
        if app.description_lines().is_empty() {
            text(
                device,
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
                    device,
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
                    device,
                    rectangle(1236, card::DESCRIPTION_Y, 3, viewport_height),
                    LINE,
                )?;
                let height = (viewport_height as usize * description::ROWS
                    / app.description_lines().len())
                .max(5) as u32;
                let top = (viewport_height - height) as usize * app.description_offset() / max;
                fill(
                    device,
                    rectangle(1236, card::DESCRIPTION_Y + top as i32, 3, height),
                    ACTIVE,
                )?;
            }
        }
        Ok(())
    }
}
