// SPDX-License-Identifier: BSD-3-Clause
use super::*;

impl Renderer {
    pub(super) fn alphabet<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let pane = rectangle(24, 90, 48, 558);
        fill(d, pane, INSET)?;
        if app.focus() == Focus::Alphabet {
            border(d, pane, LINE, 1)?;
        }
        let start = window_start(app.letter_cursor(), app.letters().len(), LETTER_ROWS);
        if start > 0 {
            text(d, "^", 43, 95, MUTED, true)?;
        }
        for (index, &letter) in app
            .letters()
            .iter()
            .enumerate()
            .skip(start)
            .take(LETTER_ROWS)
        {
            let y = 114 + (index - start) as i32 * 26;
            let rect = rectangle(29, y, 38, 24);
            let active = app.letter_filter() == Some(letter);
            if active {
                fill(d, rect, ACTIVE_BG)?;
            }
            text(
                d,
                &letter.to_string(),
                42,
                y + 2,
                if active { ACTIVE } else { MUTED },
                false,
            )?;
            if app.focus() == Focus::Alphabet && index == app.letter_cursor() {
                border(d, rect, TEXT, 2)?;
            }
        }
        if start + LETTER_ROWS < app.letters().len() {
            text(d, "v", 43, 633, MUTED, true)?;
        }
        Ok(())
    }

    pub(super) fn platforms<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        d: &mut D,
    ) -> Result<(), D::Error> {
        fill(d, rectangle(82, 90, 538, 66), PANEL)?;
        text(
            d,
            app.text("Search | Favourites | Trash | Platforms"),
            86,
            94,
            MUTED,
            true,
        )?;
        let tabs = platform_window(app);
        // Indicators reflect hidden tabs, not merely the cyclic navigation rule.
        if tabs.start > 0 {
            text(d, "<", 585, 94, ACTIVE, true)?;
        }
        if tabs.end < app.platform_tab_count() {
            text(d, ">", 607, 94, ACTIVE, true)?;
        }
        let mut x = 84;
        for index in tabs {
            let tab = app.platform_tab(index).expect("visible tab");
            let width = tab_width(tab);
            let rect = rectangle(x, 115, width, 34);
            let active = app.tab_active(tab);
            let color = if active { ACTIVE } else { MUTED };
            let background = match tab {
                _ if active => ACTIVE_BG,
                TopTab::Platform(_) => FILTER_PANEL,
                _ => INSET,
            };
            fill(d, rect, background)?;
            let icon_x = x + (width as i32 - 13) / 2;
            match tab {
                TopTab::Platform(platform) => text(
                    d,
                    &shorten(platform, 6),
                    x + 11,
                    123,
                    color,
                    false,
                )?,
                TopTab::Search => search::icon(d, icon_x, 125, color)?,
                TopTab::Trash => trash_icon(d, icon_x, 125, color)?,
                TopTab::Favourites => star(d, icon_x, 125, color)?,
            }
            if app.focus() == Focus::Platforms && index == app.platform_cursor() {
                border(d, rect, TEXT, 2)?;
            }
            x += (width + TAB_GAP) as i32;
        }
        Ok(())
    }

    pub(super) fn games<D: DrawTarget<Color = Rgb888>>(&self, app: &App, d: &mut D) -> Result<(), D::Error> {
        fill(d, rectangle(82, 164, 538, 424), INSET)?;
        if app.visible_count() == 0 {
            text(d, app.text("No matching games"), 114, 354, TEXT, false)?;
            text(
                d,
                app.text("Clear filters or switch list"),
                114,
                387,
                MUTED,
                true,
            )?;
            if app.focus() == Focus::Games {
                border(d, rectangle(82, 164, 538, 424), TEXT, 1)?;
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
            let y = 176 + (index - start) as i32 * 34;
            let rect = rectangle(91, y, 520, 32);
            let selected = app.selected_index() == Some(index);
            if selected {
                fill(d, rect, SELECTION)?;
                fill(d, rectangle(92, y + 5, 3, 24), ACTIVE)?;
            }
            text(
                d,
                &format!("{:0digits$}", index + 1),
                104,
                y + 10,
                NUMBER_DIM,
                true,
            )?;
            text(
                d,
                &shorten(&game.title, title_columns),
                title_x,
                y + 7,
                if selected { TEXT } else { MUTED },
                false,
            )?;
            text(d, &shorten(&game.platform, 6), 530, y + 11, MUTED, true)?;
            if app.is_favourite(&game.hash) {
                star(d, 590, y + 10, ACTIVE)?;
            }
            if selected && app.focus() == Focus::Games {
                border(d, rect, TEXT, 2)?;
            }
        }
        let position = app.selected_index().map_or(0, |i| i + 1);
        let count = shorten(&format!("{position:02} / {:02}", app.visible_count()), 14);
        text(
            d,
            &count,
            612 - count.chars().count() as i32 * 7,
            650,
            MUTED,
            true,
        )?;
        Ok(())
    }

    pub(super) fn lower_filters<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        d: &mut D,
    ) -> Result<(), D::Error> {
        for (field, label, x, width) in [
            (FilterField::Players, "Players", 84, 174),
            (FilterField::Genre, "Genre", 270, 342),
        ] {
            text(d, app.text(label), x + 4, 598, MUTED, true)?;
            let rect = rectangle(x, 614, width, 28);
            let active = app.active_option(field) != 0;
            fill(d, rect, if active { ACTIVE_BG } else { INSET })?;
            let selected = app.option_label(field, app.active_option(field));
            text(
                d,
                &shorten(&selected, (width as usize - 40) / 7),
                x + 10,
                621,
                if active { ACTIVE } else { MUTED },
                true,
            )?;
            text(d, "^", x + width as i32 - 18, 621, MUTED, true)?;
            if matches!(app.focus(), Focus::LowerFilters | Focus::FilterOptions)
                && app.lower_field() == field
            {
                border(d, rect, TEXT, 1)?;
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
            let popup = rectangle(x, top, width, height);
            fill(d, popup, PANEL)?;
            border(d, popup, TEXT, 1)?;
            text(
                d,
                &format!("{} / {count}", app.option_cursor() + 1),
                x + 10,
                top + 3,
                MUTED,
                true,
            )?;
            if start > 0 {
                text(d, "^", x + width as i32 - 36, top + 3, ACTIVE, true)?;
            }
            if start + rows < count {
                text(d, "v", x + width as i32 - 18, top + 3, ACTIVE, true)?;
            }
            for index in start..start + rows {
                let y = top + 20 + (index - start) as i32 * 26;
                let rect = rectangle(x + 4, y, width - 8, 24);
                let active = app.active_option(field) == index;
                fill(d, rect, if active { ACTIVE_BG } else { INSET })?;
                text(
                    d,
                    &shorten(&app.option_label(field, index), (width as usize - 28) / 7),
                    x + 12,
                    y + 5,
                    if active { ACTIVE } else { TEXT },
                    true,
                )?;
                if app.option_cursor() == index {
                    border(d, rect, TEXT, 1)?;
                }
            }
        }
        Ok(())
    }

}
