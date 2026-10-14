// SPDX-License-Identifier: BSD-3-Clause
//! Software menu rendering with one demand-loaded preview and retained regions.
mod chrome;
mod details;
mod dialogs;
mod hints;
mod list;
mod preview;
mod primitives;
mod retained;
mod search;
pub mod layout;
pub mod marquee;
pub mod notice;
pub mod splash;

pub use dialogs::{draw_input_test_countdown, draw_launch_dialog, draw_trash_dialog};
pub use layout::{FOOTER_STATS_X, FOOTER_Y, HEIGHT, WIDTH};
pub use retained::{PaintUpdate, RetainedFrame, Statistics};
use details::{draw_card_title, title_width};
use dialogs::{trash_dialog_text, DialogLayout};
use layout::{
    card, platform_window, tab_width, window_start, LETTER_ROWS, LIST_ROWS,
    MEMORY_COLUMNS, STORAGE_COLUMNS, TAB_GAP,
};
use notice::draw as draw_notice;
use preview::Picture;
use primitives::{
    bitmap_icon, border, fill, rectangle, shorten, star, text, trash_icon,
    ACTIVE, ACTIVE_BG, BG, ERROR_BG, FILTER_PANEL, INSET, LINE, MUTED, NUMBER_DIM, PANEL, SELECTION, TEXT,
};

use embedded_graphics::{
    mono_font::{
        iso_8859_5::{FONT_10X20, FONT_7X13},
        MonoTextStyle,
    },
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Arc, Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

use crate::{
    model::{description, App, FilterField, Focus, Preview, TopTab},
    runtime::launch::LaunchDialog,
};

type PreviewLoader = Box<dyn Fn(&str) -> Option<Vec<u8>>>;

#[derive(Default)]
pub struct Renderer {
    cached_kind: Option<Preview>,
    picture: Option<Picture>,
    load_preview: Option<PreviewLoader>,
}

impl Renderer {
    /// The adapter supplies bytes on demand. Rendering never opens files itself.
    pub fn with_previews(load: impl Fn(&str) -> Option<Vec<u8>> + 'static) -> Self {
        Self {
            load_preview: Some(Box::new(load)),
            ..Self::default()
        }
    }

    /// Draw a complete menu frame using model state and demand-loaded preview bytes.
    pub fn draw<D: DrawTarget<Color = Rgb888>>(
        &mut self,
        app: &App,
        notice: Option<&str>,
        device: &mut D,
    ) -> Result<(), D::Error> {
        device.clear(BG)?;
        self.header(app, device)?;

        fill(device, rectangle(16, 80, 616, 592), PANEL)?;
        self.alphabet(app, device)?;
        self.platforms(app, device)?;
        self.games(app, device)?;
        self.right(app, notice, device)?;
        self.lower_filters(app, device)?;
        let focused = match app.focus() {
            Focus::Alphabet
            | Focus::Platforms
            | Focus::Games
            | Focus::LowerFilters
            | Focus::FilterOptions => rectangle(16, 80, 616, 592),
            Focus::Details | Focus::SearchField | Focus::SearchKeys => rectangle(644, 80, 620, 592),
            Focus::Header => rectangle(16, 16, 1248, 52),
        };
        border(device, focused, TEXT, 1)?;
        self.footer(app, device)?;
        Ok(())
    }
}
