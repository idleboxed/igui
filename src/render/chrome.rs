// SPDX-License-Identifier: BSD-3-Clause
use super::*;

impl Renderer {
    /// Draw the menu header and its current exit focus.
    pub(super) fn header<D: DrawTarget<Color = Rgb888>>(&self, app: &App, device: &mut D) -> Result<(), D::Error> {
        fill(device, rectangle(16, 16, 1248, 52), PANEL)?;
        text(device, "IGUI", 32, 30, ACTIVE, false)?;
        text(
            device,
            concat!("idle gui / ", env!("CARGO_PKG_VERSION"), "-", env!("IGUI_BUILD_UTC")),
            92,
            33,
            MUTED,
            true,
        )?;
        // The power symbol means session exit, never a machine poweroff.
        let exit = rectangle(1216, 30, 32, 24);
        let red = Rgb888::new(240, 80, 80);
        Arc::new(Point::new(1226, 37), 12, 310.0.deg(), 280.0.deg())
            .into_styled(PrimitiveStyle::with_stroke(red, 1))
            .draw(device)?;
        Line::new(Point::new(1232, 35), Point::new(1232, 42))
            .into_styled(PrimitiveStyle::with_stroke(red, 1))
            .draw(device)?;
        if app.exit_focused() {
            border(device, exit, TEXT, 1)?;
        }

        Ok(())
    }

    /// Draw footer hints and system statistics.
    pub(super) fn footer<D: DrawTarget<Color = Rgb888>>(&self, app: &App, device: &mut D) -> Result<(), D::Error> {
        self.footer_hints(app, device)?;
        self.footer_stats(app, device)
    }

    /// Draw action hints for the current menu state.
    pub(super) fn footer_hints<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        device: &mut D,
    ) -> Result<(), D::Error> {
        fill(
            device,
            rectangle(0, FOOTER_Y, FOOTER_STATS_X as u32, HEIGHT - FOOTER_Y as u32),
            PANEL,
        )?;
        let hint = app.text(match app.focus() {
            Focus::SearchField => "←/→ cursor | X erase | B reset | Y close | ↓ keys | ↑ header",
            Focus::SearchKeys => "↑/↓/←/→ keys | A type | Y close | ← edge games",
            focus if app.search().is_some() => match focus {
                Focus::Games if app.trash_only() => "↑/↓ games | ←/→ | A select | Start launch | B favourite | Select restore | Y close | X filters",
                Focus::Games => "↑/↓ games | ←/→ | A select | Start launch | B favourite | Select trash | Y close | X filters",
                Focus::Header => "A exit | ↑/↓ search | Y close search | X filters",
                Focus::Alphabet => "↑/↓ letters | A toggle | → games | Y close search | X filters",
                Focus::Platforms => "←/→ filters | A toggle | ↓ games | Y close search | ↑/X filters",
                Focus::LowerFilters => "←/→ field | A open | ↑ games | ↓ filters | Y close search",
                Focus::FilterOptions => "↑/↓ value | A apply | B cancel | Y close search",
                _ => unreachable!("search replaces details"),
            },
            Focus::LowerFilters => "←/→ field | A open | ↑ games | ↓/Y top filters",
            Focus::FilterOptions => "↑/↓ value | A apply | B cancel | Y top filters",
            Focus::Games if app.trash_only() => "↑/↓ games | ←/→ panels | A/Start launch | B favourite | Select restore | Y/X filters",
            Focus::Games => "↑/↓ games | ←/→ panels | A/Start launch | B favourite | Select trash | Y/X filters",
            Focus::Alphabet => "↑/↓ letters | A toggle filter | → games | Y top filters | X bottom filters",
            Focus::Platforms => "←/→ filters | A toggle | ↑/X bottom filters | ↓ games",
            Focus::Details => "↑/↓ scroll or header | ← games | Y top filters | X bottom filters",
            Focus::Header => "A exit | ↑/↓ details | Y top filters | X bottom filters",
        });
        hints::draw(
            device,
            hint,
            Point::new(24, FOOTER_Y + 12),
            (FOOTER_STATS_X - 48) as u32,
            MUTED,
        )?;
        Ok(())
    }

    /// Format catalogue, storage and memory values for the footer.
    pub(super) fn stats_text(app: &App) -> (String, String, i32, i32) {
        let amount = app.available_bytes().map_or_else(
            || "-".into(),
            |bytes| format!("{:.1}", bytes as f64 / 1_000_000_000.0),
        );
        let free = shorten(
            &format!("{}: {amount} {}", app.text("Disk"), app.text("GB")),
            STORAGE_COLUMNS,
        );
        let free_x = WIDTH as i32 - 24 - free.chars().count() as i32 * 7;
        let memory_amount = crate::memory::display_amount(app.memory());
        let memory = shorten(
            &format!("{}: {memory_amount} {}", app.text("RAM"), app.text("MiB")),
            MEMORY_COLUMNS,
        );
        // Anchor to the actual disk label, not the maximum reserved cell width.
        let memory_x = free_x - 16 - memory.chars().count() as i32 * 7;
        (memory, free, memory_x, free_x)
    }

    /// Draw the current catalogue and system statistics.
    pub(super) fn footer_stats<D: DrawTarget<Color = Rgb888>>(
        &self,
        app: &App,
        device: &mut D,
    ) -> Result<(), D::Error> {
        fill(
            device,
            rectangle(
                FOOTER_STATS_X,
                FOOTER_Y,
                WIDTH - FOOTER_STATS_X as u32,
                HEIGHT - FOOTER_Y as u32,
            ),
            PANEL,
        )?;
        let (memory, free, memory_x, free_x) = Self::stats_text(app);
        text(device, &memory, memory_x, FOOTER_Y + 12, MUTED, true)?;
        text(device, &free, free_x, FOOTER_Y + 12, MUTED, true)
    }

}
