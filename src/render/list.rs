// SPDX-License-Identifier: BSD-3-Clause
use super::*;

impl Renderer {
    /// Draw the visible alphabet tabs and their selected and active states.
    pub(super) fn alphabet<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        device: &mut D,
    ) -> Result<(), D::Error> {
        let pane = rectangle(24, 90, 48, 558);
        fill(device, pane, INSET)?;
        if app.focus() == Focus::Alphabet {
            border(device, pane, LINE, 1)?;
        }
        let start = window_start(app.letter_cursor(), app.letters().len(), LETTER_ROWS);
        if start > 0 {
            text(device, "^", 43, 95, MUTED, true)?;
        }
        for (index, &letter) in app
            .letters()
            .iter()
            .enumerate()
            .skip(start)
            .take(LETTER_ROWS)
        {
            let position_y = 114 + (index - start) as i32 * 26;
            let rect = rectangle(29, position_y, 38, 24);
            let active = app.letter_filter() == Some(letter);
            if active {
                fill(device, rect, ACTIVE_BG)?;
            }
            text(
                device,
                &letter.to_string(),
                42,
                position_y + 2,
                if active { ACTIVE } else { MUTED },
                false,
            )?;
            if app.focus() == Focus::Alphabet && index == app.letter_cursor() {
                border(device, rect, TEXT, 2)?;
            }
        }
        if start + LETTER_ROWS < app.letters().len() {
            text(device, "v", 43, 633, MUTED, true)?;
        }
        Ok(())
    }

    /// Draw the visible top tabs for controls and catalogue platforms.
    pub(super) fn platforms<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        device: &mut D,
    ) -> Result<(), D::Error> {
        fill(device, rectangle(82, 90, 538, 66), PANEL)?;
        text(
            device,
            app.text("Search | Favourites | Trash | Platforms"),
            86,
            94,
            MUTED,
            true,
        )?;
        let tabs = platform_window(app);
        // Indicators reflect hidden tabs, not merely the cyclic navigation rule.
        if tabs.start > 0 {
            text(device, "<", 585, 94, ACTIVE, true)?;
        }
        if tabs.end < app.platform_tab_count() {
            text(device, ">", 607, 94, ACTIVE, true)?;
        }
        let mut position_x = 84;
        for index in tabs {
            let tab = app.platform_tab(index).expect("visible tab");
            let width = tab_width(tab);
            let rect = rectangle(position_x, 115, width, 34);
            let active = app.tab_active(tab);
            let color = if active { ACTIVE } else { MUTED };
            let background = match tab {
                _ if active => ACTIVE_BG,
                TopTab::Platform(_) => FILTER_PANEL,
                _ => INSET,
            };
            fill(device, rect, background)?;
            let icon_x = position_x + (width as i32 - 13) / 2;
            match tab {
                TopTab::Platform(platform) => text(
                    device,
                    &shorten(platform, 6),
                    position_x + 11,
                    123,
                    color,
                    false,
                )?,
                TopTab::Search => search::icon(device, icon_x, 125, color)?,
                TopTab::Trash => trash_icon(device, icon_x, 125, color)?,
                TopTab::Favourites => star(device, icon_x, 125, color)?,
            }
            if app.focus() == Focus::Platforms && index == app.platform_cursor() {
                border(device, rect, TEXT, 2)?;
            }
            position_x += (width + TAB_GAP) as i32;
        }
        Ok(())
    }

    /// Draw the filtered game rows and selection marker.
    pub(super) fn games<D: DrawTarget<Color = Rgb888>>(&self, app: &App, device: &mut D) -> Result<(), D::Error> {
        fill(device, rectangle(82, 164, 538, 424), INSET)?;
        if app.visible_count() == 0 {
            text(device, app.text("No matching games"), 114, 354, TEXT, false)?;
            text(
                device,
                app.text("Clear filters or switch list"),
                114,
                387,
                MUTED,
                true,
            )?;
            if app.focus() == Focus::Games {
                border(device, rectangle(82, 164, 538, 424), TEXT, 1)?;
            }
        }
        let start = window_start(
            app.selected_index().unwrap_or(0),
            app.visible_count(),
            LIST_ROWS,
        );
        // Ordinals belong to the filtered list, not to a screenful or ROM ID.
        let digits = app.visible_count().max(1).to_string().len();
        let title_x = 104 + digits as i32 * 7 + 14;
        let title_columns = (516 - title_x) as usize / 10;
        for index in start..(start + LIST_ROWS).min(app.visible_count()) {
            let game = app.visible_game(index).expect("visible index");
            let position_y = 176 + (index - start) as i32 * 34;
            let rect = rectangle(91, position_y, 520, 32);
            let selected = app.selected_index() == Some(index);
            if selected {
                fill(device, rect, SELECTION)?;
                fill(device, rectangle(92, position_y + 5, 3, 24), ACTIVE)?;
            }
            text(
                device,
                &format!("{:0digits$}", index + 1),
                104,
                position_y + 10,
                NUMBER_DIM,
                true,
            )?;
            text(
                device,
                &shorten(&game.title, title_columns),
                title_x,
                position_y + 7,
                if selected { TEXT } else { MUTED },
                false,
            )?;
            text(device, &shorten(&game.platform, 6), 530, position_y + 11, MUTED, true)?;
            if app.is_favourite(&game.hash) {
                star(device, 590, position_y + 10, ACTIVE)?;
            }
            if selected && app.focus() == Focus::Games {
                border(device, rect, TEXT, 2)?;
            }
        }
        let position = app.selected_index().map_or(0, |index_item| index_item + 1);
        let count = shorten(&format!("{position:02} / {:02}", app.visible_count()), 14);
        text(
            device,
            &count,
            612 - count.chars().count() as i32 * 7,
            650,
            MUTED,
            true,
        )?;
        Ok(())
    }

    /// Draw lower filter fields and any open choice list.
    pub(super) fn lower_filters<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        device: &mut D,
    ) -> Result<(), D::Error> {
        for (field, label, position_x, width) in [
            (FilterField::Players, "Players", 84, 174),
            (FilterField::Genre, "Genre", 270, 342),
        ] {
            text(device, app.text(label), position_x + 4, 598, MUTED, true)?;
            let rect = rectangle(position_x, 614, width, 28);
            let active = app.active_option(field) != 0;
            fill(device, rect, if active { ACTIVE_BG } else { INSET })?;
            let selected = app.option_label(field, app.active_option(field));
            text(
                device,
                &shorten(&selected, (width as usize - 40) / 7),
                position_x + 10,
                621,
                if active { ACTIVE } else { MUTED },
                true,
            )?;
            text(device, "^", position_x + width as i32 - 18, 621, MUTED, true)?;
            if matches!(app.focus(), Focus::LowerFilters | Focus::FilterOptions)
                && app.lower_field() == field
            {
                border(device, rect, TEXT, 1)?;
            }
            if app.focus() != Focus::FilterOptions || app.lower_field() != field {
                continue;
            }
            let count = app.option_count(field);
            let capacity = 6;
            let rows = count.min(capacity);
            let start = window_start(app.option_cursor(), count, capacity);
            let height = 24 + rows as u32 * 26;
            let top = 592 - height as i32;
            let popup = rectangle(position_x, top, width, height);
            fill(device, popup, PANEL)?;
            border(device, popup, TEXT, 1)?;
            text(
                device,
                &format!("{} / {count}", app.option_cursor() + 1),
                position_x + 10,
                top + 3,
                MUTED,
                true,
            )?;
            if start > 0 {
                text(device, "^", position_x + width as i32 - 36, top + 3, ACTIVE, true)?;
            }
            if start + rows < count {
                text(device, "v", position_x + width as i32 - 18, top + 3, ACTIVE, true)?;
            }
            for index in start..start + rows {
                let position_y = top + 20 + (index - start) as i32 * 26;
                let rect = rectangle(position_x + 4, position_y, width - 8, 24);
                let active = app.active_option(field) == index;
                fill(device, rect, if active { ACTIVE_BG } else { INSET })?;
                text(
                    device,
                    &shorten(&app.option_label(field, index), (width as usize - 28) / 7),
                    position_x + 12,
                    position_y + 5,
                    if active { ACTIVE } else { TEXT },
                    true,
                )?;
                if app.option_cursor() == index {
                    border(device, rect, TEXT, 1)?;
                }
            }
        }
        Ok(())
    }

}
