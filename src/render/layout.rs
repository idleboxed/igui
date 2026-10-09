// SPDX-License-Identifier: BSD-3-Clause
//! Shared geometry for full and retained rendering.
use super::{description, App, TopTab, FONT_10X20, FONT_7X13};

pub const WIDTH: u32 = imanlib::surface::WIDTH;
pub const HEIGHT: u32 = imanlib::surface::HEIGHT;
pub const FOOTER_Y: i32 = 684;

/// Shared card geometry: labels, values and scrolling content follow the same rows.
pub mod card {
    use super::{description, FONT_10X20, FONT_7X13};

    pub const LABEL_VALUE_GAP: i32 = 6;
    pub const ROW_GAP: i32 = 15;
    pub const VALUE_OFFSET: i32 = FONT_7X13.character_size.height as i32 + LABEL_VALUE_GAP;
    pub const ROW_STEP: i32 = VALUE_OFFSET + FONT_10X20.character_size.height as i32 + ROW_GAP;
    pub const DESCRIPTION_LINE_HEIGHT: u32 = 14;
    pub const DESCRIPTION_HEIGHT: u32 = description::ROWS as u32 * DESCRIPTION_LINE_HEIGHT;

    // Grow upwards when spacing changes: keep all six description lines inside the panel.
    pub const BOTTOM: i32 = 664;
    pub const HEIGHT: u32 = 4 + 3 * ROW_STEP as u32 + VALUE_OFFSET as u32 + DESCRIPTION_HEIGHT + 3;
    pub const TOP: i32 = BOTTOM - HEIGHT as i32;

    pub const TITLE_LABEL_Y: i32 = TOP + 4;
    pub const TITLE_X: i32 = 676;
    pub const TITLE_Y: i32 = TITLE_LABEL_Y + VALUE_OFFSET;
    pub const TITLE_WIDTH: u32 = 540;
    pub const TITLE_HEIGHT: u32 = FONT_10X20.character_size.height;
    pub const GENRE_LABEL_Y: i32 = TITLE_LABEL_Y + ROW_STEP;
    pub const FACTS_LABEL_Y: i32 = GENRE_LABEL_Y + ROW_STEP;
    pub const DESCRIPTION_LABEL_Y: i32 = FACTS_LABEL_Y + ROW_STEP;
    pub const DESCRIPTION_Y: i32 = DESCRIPTION_LABEL_Y + VALUE_OFFSET;

    pub const GENRE_X: i32 = 676;
    pub const GENRE_REGION_GAP: u32 = 20;
    pub const GENRE_REGION_WIDTH: u32 = (560 - GENRE_REGION_GAP) / 2;
    pub const REGION_X: i32 = GENRE_X + (GENRE_REGION_WIDTH + GENRE_REGION_GAP) as i32;
    pub const GENRE_REGION_COLUMNS: usize =
        (GENRE_REGION_WIDTH / FONT_10X20.character_size.width) as usize;

    pub const PREVIEW_TOP: i32 = 92;
    pub const PREVIEW_BOTTOM: i32 = TOP - 12;
    pub const PREVIEW_IMAGE_Y: i32 = PREVIEW_TOP + 14;
    pub const PREVIEW_IMAGE_HEIGHT: u32 = (PREVIEW_BOTTOM - 8 - PREVIEW_IMAGE_Y) as u32;
    pub const PREVIEW_LABEL_Y: i32 = PREVIEW_IMAGE_Y
        + (PREVIEW_IMAGE_HEIGHT as i32 - FONT_10X20.character_size.height as i32) / 2;
}

pub(super) const STORAGE_COLUMNS: usize = 27;
pub(super) const MEMORY_COLUMNS: usize = 38;
pub(super) const STORAGE_X: i32 = WIDTH as i32 - 24 - STORAGE_COLUMNS as i32 * 7;
/// Fixed telemetry region: changing numbers must not move or truncate the hints.
pub const FOOTER_STATS_X: i32 = STORAGE_X - 24 - MEMORY_COLUMNS as i32 * 7;
pub(super) const LIST_ROWS: usize = 12;
pub(super) const LETTER_ROWS: usize = 20;
pub(super) const TAB_GAP: u32 = 9;
pub(super) const TABS_WIDTH: u32 = 532;

pub(super) fn window_start(cursor: usize, count: usize, capacity: usize) -> usize {
    cursor
        .saturating_sub(capacity / 2)
        .min(count.saturating_sub(capacity))
}

pub(super) fn tab_width(tab: TopTab<'_>) -> u32 {
    match tab {
        TopTab::Platform(_) => 80,
        _ => 40,
    }
}

/// Fit mixed-width tabs around the cursor; shared with the retained snapshot.
pub(super) fn platform_window(app: &App) -> std::ops::Range<usize> {
    let width = |index| tab_width(app.platform_tab(index).expect("tab index"));
    let mut start = app.platform_cursor();
    let mut end = start + 1;
    let mut left = 0;
    while start > 0 && left + TAB_GAP + width(start - 1) <= TABS_WIDTH / 2 {
        start -= 1;
        left += TAB_GAP + width(start);
    }
    let mut used = left + width(app.platform_cursor());
    while end < app.platform_tab_count() && used + TAB_GAP + width(end) <= TABS_WIDTH {
        used += TAB_GAP + width(end);
        end += 1;
    }
    // Near the end, use the remaining space for preceding tabs instead of blanks.
    while start > 0 && used + TAB_GAP + width(start - 1) <= TABS_WIDTH {
        start -= 1;
        used += TAB_GAP + width(start);
    }
    start..end
}
