// SPDX-License-Identifier: BSD-3-Clause
//! Retained CPU image; presentation still uploads the complete image atomically.
use super::*;
use {
    crate::locales::Locales,
    crate::render::marquee::Marquee,
    crate::model::Game,
    crate::model::search::Search,
    imanlib::surface::Surface,
};
use imanlib::igui::GuiState;
use std::{convert::Infallible, time::{Duration, Instant}};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintUpdate {
    pub full: bool,
    pub dialog: bool,
    pub header: bool,
    pub left: bool,
    pub right: bool,
    pub preview: bool,
    pub metadata: bool,
    pub title: bool,
    pub borders: bool,
    pub hints: bool,
    pub telemetry: bool,
}
impl PaintUpdate {
    pub fn changed(self) -> bool {
        self != Self::default()
    }
}

pub use imanlib::display_metrics::Statistics;

#[derive(PartialEq, Eq)]
struct VisibleRows {
    start: usize,
    total: usize,
    selected: Option<usize>,
    items: Vec<(String, String, bool)>,
}

// Capture only the visible windows, not a duplicate catalogue or image collection.
#[derive(PartialEq, Eq)]
struct LeftState {
    session: GuiState,
    search: bool,
    focus: Option<Focus>,
    letters: (usize, usize, Vec<char>),
    platforms: (usize, usize, Vec<String>),
    rows: VisibleRows,
    field: FilterField,
    options: (usize, usize, usize, Vec<String>),
    labels: [String; 2],
}
impl LeftState {
    fn capture(app: &App) -> Self {
        let letters = window_start(app.letter_cursor(), app.letters().len(), LETTER_ROWS);
        let platforms = platform_window(app);
        let rows = window_start(
            app.selected_index().unwrap_or(0),
            app.visible_count(),
            LIST_ROWS,
        );
        let count = app.option_count(app.lower_field());
        let options = window_start(app.option_cursor(), count, 6);
        Self {
            session: app.session_state(),
            search: app.search().is_some(),
            focus: match app.focus() {
                Focus::Details | Focus::Header | Focus::SearchField | Focus::SearchKeys => None,
                focus => Some(focus),
            },
            letters: (
                letters,
                app.letters().len(),
                app.letters()
                    .iter()
                    .skip(letters)
                    .take(LETTER_ROWS)
                    .copied()
                    .collect(),
            ),
            platforms: (
                platforms.start,
                app.platform_tab_count(),
                platforms
                    .filter_map(|i| match app.platform_tab(i) {
                        Some(TopTab::Platform(p)) => Some(p.to_owned()),
                        _ => None,
                    })
                    .collect(),
            ),
            rows: VisibleRows {
                start: rows,
                total: app.visible_count(),
                selected: app.selected_index(),
                items: (rows..(rows + LIST_ROWS).min(app.visible_count()))
                    .map(|i| {
                        let game = app.visible_game(i).expect("visible row");
                        (
                            game.title.clone(),
                            game.platform.clone(),
                            app.is_favourite(&game.hash),
                        )
                    })
                    .collect(),
            },
            field: app.lower_field(),
            options: (
                options,
                count,
                app.option_cursor(),
                (options..(options + 6).min(count))
                    .map(|i| app.option_label(app.lower_field(), i))
                    .collect(),
            ),
            labels: [
                app.option_label(
                    FilterField::Players,
                    app.active_option(FilterField::Players),
                ),
                app.option_label(FilterField::Genre, app.active_option(FilterField::Genre)),
            ],
        }
    }
}

struct State {
    locales: Locales,
    left: LeftState,
    selected: Option<Game>,
    search: Option<Search>,
    description_offset: usize,
    notice: Option<String>,
    focus: Focus,
    trash: bool,
    stats: (String, String, i32, i32),
    dialog: Option<DialogLayout>,
}

struct DialogBackground {
    position: Point,
    surface: Surface,
}
impl DialogBackground {
    fn capture(surface: &Surface, dialog: &DialogLayout) -> Self {
        let bounds = dialog.bounds();
        let mut saved = Surface::new(bounds.size.width, bounds.size.height);
        saved.blit(surface, -bounds.top_left);
        Self { position: bounds.top_left, surface: saved }
    }
}

/// Owns the retained Surface so a snapshot cannot accidentally be applied to a
/// different framebuffer. Mutable access explicitly invalidates all regions.
pub struct RetainedFrame {
    surface: Surface,
    renderer: Renderer,
    previous: Option<State>,
    dialog_background: Option<DialogBackground>,
    title_marquee: Marquee,
    statistics: Statistics,
}
impl RetainedFrame {
    pub fn new(surface: Surface, renderer: Renderer) -> Self {
        assert_eq!((surface.width(), surface.height()), (WIDTH, HEIGHT));
        Self {
            surface,
            renderer,
            previous: None,
            dialog_background: None,
            title_marquee: Marquee::new(0, card::TITLE_WIDTH as usize),
            statistics: Statistics::default(),
        }
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn surface_mut(&mut self) -> &mut Surface {
        self.previous = None;
        self.dialog_background = None;
        &mut self.surface
    }
    pub fn prepare_selected(&mut self, app: &App) {
        self.renderer.prepare_selected(app);
    }
    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }
    pub fn prepared_preview_bytes(&self) -> usize {
        self.renderer
            .picture
            .as_ref()
            .and_then(|p| p.surface.as_ref())
            .map_or(0, |surface| surface.pixels().len() * 4)
    }

    /// Animate only the visible title row. Modal/search/inactive frames freeze
    /// the clock; normal updates restore the same position when repainting it.
    pub fn advance_title(&mut self, now: Duration, active: bool) -> Result<PaintUpdate, Infallible> {
        let started = Instant::now();
        let Some(state) = &self.previous else { return Ok(PaintUpdate::default()); };
        let paused = !active || state.dialog.is_some() || state.search.is_some();
        if !self.title_marquee.advance(now, paused) {
            return Ok(PaintUpdate::default());
        }
        let game = state.selected.as_ref().expect("scrolling card");
        draw_card_title(
            &game.title,
            self.title_marquee.content_width(),
            self.title_marquee.offset(),
            &mut self.surface,
        )?;
        let update = PaintUpdate { title: true, ..Default::default() };
        self.record_update(update, started, false);
        Ok(update)
    }

    pub fn update(
        &mut self,
        app: &App,
        notice: Option<&str>,
        dialog: Option<&LaunchDialog>,
    ) -> Result<PaintUpdate, Infallible> {
        self.update_with_countdown(app, notice, dialog, None)
    }

    /// All catalogue overlays share retained state; none invalidates the menu.
    pub fn update_with_countdown(
        &mut self,
        app: &App,
        notice: Option<&str>,
        dialog: Option<&LaunchDialog>,
        countdown: Option<&crate::input::startup::StartupInputTest>,
    ) -> Result<PaintUpdate, Infallible> {
        let started = Instant::now();
        let left = LeftState::capture(app);
        let stats = Renderer::stats_text(app);
        let selected = app.detail_game();
        let search = app.search();
        let old = self.previous.as_ref();
        if old.and_then(|s| s.selected.as_ref()).map(|g| (&g.hash, &g.title))
            != selected.map(|g| (&g.hash, &g.title)) {
            self.title_marquee = Marquee::new(selected.map_or(0, |g| title_width(&g.title)), card::TITLE_WIDTH as usize);
        }
        let dialog_key = countdown
            .and_then(|c| c.message())
            .map(|message| DialogLayout::new("Проверка геймпада", &message))
            .or_else(|| dialog.map(|d| {
                DialogLayout::new(app.text(d.platform_name()), &d.message(app.locales()))
            }))
            .or_else(|| {
                app.trash_confirmation().then(|| {
                    let (heading, message) = trash_dialog_text(app);
                    DialogLayout::new(heading, message)
                })
            });
        let modal_visible = dialog_key.is_some();
        let full = old.is_none_or(|s| s.locales != *app.locales());
        let update = if full {
            PaintUpdate {
                full: true,
                ..Default::default()
            }
        } else {
            let old = old.expect("retained state");
            let mode_changed = old.search.is_some() != search.is_some();
            let mut update = PaintUpdate {
                header: (old.focus == Focus::Header) != app.exit_focused(),
                left: old.left != left,
                right: mode_changed
                    || (search.is_some()
                        && (old.search.as_ref() != search
                            || (old.focus != app.focus()
                                && (old.focus.is_search() || app.focus().is_search()))
                            || old.notice.as_deref() != notice)),
                preview: search.is_none()
                    && !mode_changed
                    && old.selected.as_ref().map(|g| &g.preview) != selected.map(|g| &g.preview),
                metadata: search.is_none()
                    && !mode_changed
                    && (old.selected.as_ref() != selected
                        || old.description_offset != app.description_offset()
                        || old.notice.as_deref() != notice),
                borders: focus_group(old.focus) != focus_group(app.focus()),
                hints: old.focus != app.focus() || old.trash != app.trash_only() || mode_changed,
                telemetry: old.stats != stats,
                ..Default::default()
            };
            // Region redraw can intersect the modal, including restored outer
            // borders. The header/footer themselves do not overlap its pixels.
            update.dialog = old.dialog != dialog_key
                || (modal_visible
                    && (update.left
                    || update.right
                    || update.preview
                    || update.metadata
                    || update.borders
                    || update.header));
            update
        };
        if full {
            self.dialog_background = None;
            self.renderer.draw(app, notice, &mut self.surface)?;
            if let Some(game) = selected.filter(|_| self.title_marquee.offset() != 0) {
                draw_card_title(&game.title, self.title_marquee.content_width(), self.title_marquee.offset(), &mut self.surface)?;
            }
            if let Some(dialog) = &dialog_key {
                self.dialog_background = Some(DialogBackground::capture(&self.surface, dialog));
                dialog.draw(&mut self.surface)?;
            }
        } else {
            let old_dialog = old.and_then(|s| s.dialog.as_ref());
            let background_changed = update.left || update.right || update.preview
                || update.metadata || update.borders || update.header;
            // Restore only the covered rectangle before repainting underlying
            // regions, removing the modal, or replacing it with a different size.
            // Text-only changes keep both the opaque panel and its saved underlay.
            if update.dialog && old_dialog.is_some() && (background_changed
                || old_dialog.map(|d| d.panel) != dialog_key.as_ref().map(|d| d.panel)) {
                let saved = self.dialog_background.take().expect("modal background");
                self.surface.blit(&saved.surface, saved.position);
            }
            if update.header {
                self.renderer.header(app, &mut self.surface)?;
            }
            if update.left {
                fill(&mut self.surface, rectangle(16, 80, 616, 592), PANEL)?;
                self.renderer.alphabet(app, &mut self.surface)?;
                self.renderer.platforms(app, &mut self.surface)?;
                self.renderer.games(app, &mut self.surface)?;
                self.renderer.lower_filters(app, &mut self.surface)?;
            }
            if update.preview {
                self.renderer.prepare_selected(app);
                if let Some(picture) = &self.renderer.picture {
                    fill(
                        &mut self.surface,
                        rectangle(
                            656,
                            card::PREVIEW_TOP,
                            596,
                            (card::PREVIEW_BOTTOM - card::PREVIEW_TOP) as u32,
                        ),
                        INSET,
                    )?;
                    if let Some(surface) = &picture.surface {
                        self.surface.blit(surface, picture.position);
                    }
                } else {
                    self.renderer.preview(app, &mut self.surface)?;
                }
            }
            if update.metadata {
                self.renderer.metadata(app, notice, &mut self.surface)?;
            }
            if update.right {
                self.renderer.right(app, notice, &mut self.surface)?;
            }
            if update.metadata || update.right {
                if let Some(game) = selected.filter(|_| self.title_marquee.offset() != 0) {
                    draw_card_title(&game.title, self.title_marquee.content_width(), self.title_marquee.offset(), &mut self.surface)?;
                }
            }
            if update.borders || update.left || update.right || update.header {
                for (group, rect) in [
                    (0, rectangle(16, 80, 616, 592)),
                    (1, rectangle(644, 80, 620, 592)),
                    (2, rectangle(16, 16, 1248, 52)),
                ] {
                    border(
                        &mut self.surface,
                        rect,
                        if group == focus_group(app.focus()) {
                            TEXT
                        } else {
                            PANEL
                        },
                        1,
                    )?;
                }
            }
            if update.hints {
                self.renderer.footer_hints(app, &mut self.surface)?;
            }
            if update.telemetry {
                self.renderer.footer_stats(app, &mut self.surface)?;
            }
            if update.dialog {
                if let Some(dialog) = &dialog_key {
                    if self.dialog_background.is_none() {
                        self.dialog_background = Some(DialogBackground::capture(&self.surface, dialog));
                        dialog.draw(&mut self.surface)?;
                    } else {
                        dialog.draw_changed(old_dialog.expect("retained modal"), &mut self.surface)?;
                    }
                }
            }
        }
        // Do not clone the selected metadata/dictionary again for a RAM tick.
        let (selected, locales) = match self.previous.take() {
            Some(mut previous) => {
                if previous.selected.as_ref() != selected {
                    previous.selected = selected.cloned();
                }
                if previous.locales != *app.locales() {
                    previous.locales = app.locales().clone();
                }
                (previous.selected, previous.locales)
            }
            None => (selected.cloned(), app.locales().clone()),
        };
        self.previous = Some(State {
            locales,
            selected,
            search: search.cloned(),
            left,
            stats,
            description_offset: app.description_offset(),
            notice: notice.map(str::to_owned),
            focus: app.focus(),
            trash: app.trash_only(),
            dialog: dialog_key,
        });
        self.record_update(update, started, modal_visible);
        Ok(update)
    }

    fn record_update(&mut self, update: PaintUpdate, started: Instant, modal_visible: bool) {
        let elapsed = started.elapsed().as_micros();
        let s = &mut self.statistics;
        s.requests += 1;
        s.unchanged += u64::from(!update.changed());
        s.full += u64::from(update.full);
        s.dialog += u64::from(update.dialog || (update.full && modal_visible));
        s.left += u64::from(update.left);
        s.preview += u64::from(update.preview);
        s.metadata += u64::from(update.metadata);
        s.telemetry += u64::from(update.telemetry);
        s.render_us_sum += elapsed;
        s.render_us_max = s.render_us_max.max(elapsed);
        if update.full {
            s.full_cost.record(elapsed);
        } else if update
            == (PaintUpdate {
                dialog: true,
                ..Default::default()
            })
        {
            s.dialog_cost.record(elapsed);
        } else if update
            == (PaintUpdate {
                telemetry: true,
                ..Default::default()
            })
        {
            s.telemetry_only_cost.record(elapsed);
        } else if update.changed() {
            s.partial_cost.record(elapsed);
        }
    }
}

fn focus_group(focus: Focus) -> u8 {
    match focus {
        Focus::Details | Focus::SearchField | Focus::SearchKeys => 1,
        Focus::Header => 2,
        _ => 0,
    }
}
