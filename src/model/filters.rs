// SPDX-License-Identifier: BSD-3-Clause
use super::{App, FilterField, FilterValue, TopTab, TOP_CONTROLS};
use std::collections::BTreeSet;

/// Build all, unknown and distinct known choices from catalogue values.
pub(super) fn choices<T: Ord>(values: impl Iterator<Item = Option<T>>) -> Vec<FilterValue<T>> {
    let values: BTreeSet<_> = values.collect();
    std::iter::once(FilterValue::All)
        .chain(values.into_iter().map(|value_item| match value_item {
            None => FilterValue::Unknown,
            Some(value_item) => FilterValue::Value(value_item),
        }))
        .collect()
}

/// Build a case-insensitive sort key with Ё in its natural Russian alphabet position.
pub(super) fn alphabetical_key(value: &str) -> Vec<u32> {
    value
        .to_lowercase()
        .chars()
        // Unicode places Ё outside the Russian alphabet; put it between Е and Ж.
        .map(|character| if character == 'ё' { u32::from('е') * 2 + 1 } else { u32::from(character) * 2 })
        .collect()
}

impl App {
    /// Publish loaded or successfully persisted preferences; no filesystem I/O.
    pub fn set_favourites(&mut self, favourites: BTreeSet<String>) {
        self.favourites = favourites;
        if self.favourites_only {
            let previous = self.selected.and_then(|index_item| self.visible.get(index_item).copied());
            self.refilter();
            if previous != self.selected.and_then(|index_item| self.visible.get(index_item).copied()) {
                self.update_description();
            }
        }
    }

    /// Report whether the supplied ROM hash is marked as a favourite.
    pub fn is_favourite(&self, hash: &str) -> bool {
        self.favourites.contains(hash)
    }

    /// Report whether the favourites filter is enabled.
    pub fn favourites_only(&self) -> bool {
        self.favourites_only
    }

    /// Publish loaded or successfully persisted trash, newest first; no filesystem I/O.
    pub fn set_trash(&mut self, trash: Vec<String>) {
        let previous_index = self.selected;
        let previous = self.selected.and_then(|index_item| self.visible.get(index_item).copied());
        self.trash.clear();
        for (position, hash) in trash.into_iter().enumerate() {
            self.trash.entry(hash).or_insert(position);
        }
        self.refilter();
        if previous != self.selected.and_then(|index_item| self.visible.get(index_item).copied()) {
            // A removed row leaves its successor in the same slot; the last row
            // falls back to its predecessor without changing ordinary filters.
            if let Some(index) = previous_index {
                self.selected = self.visible.len().checked_sub(1).map(|last| index.min(last));
            }
            self.update_description();
        }
    }

    /// Report whether the supplied ROM hash is in the trash.
    pub fn is_trashed(&self, hash: &str) -> bool {
        self.trash.contains_key(hash)
    }

    /// Report whether the menu currently shows only trashed games.
    pub fn trash_only(&self) -> bool {
        self.trash_only
    }

    /// Report whether a soft-deletion confirmation is pending.
    pub fn trash_confirmation(&self) -> bool {
        self.pending_trash.is_some()
    }

    /// One tab definition serves navigation, full rendering and retained state.
    pub fn platform_tab_count(&self) -> usize {
        TOP_CONTROLS.len() + self.platforms.len()
    }

    /// Return the control or platform tab at an index, or None outside the tab list.
    pub fn platform_tab(&self, index: usize) -> Option<TopTab<'_>> {
        TOP_CONTROLS.get(index).copied().or_else(|| {
            index
                .checked_sub(TOP_CONTROLS.len())
                .and_then(|index_item| self.platforms.get(index_item))
                .map(|platform_item| TopTab::Platform(platform_item))
        })
    }

    /// Report whether a top tab corresponds to an active filter or search.
    pub fn tab_active(&self, tab: TopTab<'_>) -> bool {
        match tab {
            TopTab::Search => self.search.is_some(),
            TopTab::Favourites => self.favourites_only,
            TopTab::Trash => self.trash_only,
            TopTab::Platform(platform_item) => self.platform_filter() == Some(platform_item),
        }
    }
    /// Sort translated genre choices while preserving active and pending selections.
    pub(super) fn sort_genre_choices(&mut self) {
        let active = self.genre_filter().clone();
        let pending = if self.lower_field == FilterField::Genre {
            self.genre_choices.get(self.option_cursor).cloned()
        } else {
            None
        };
        let locales = &self.locales;
        self.genre_choices.sort_by_cached_key(|value| match value {
            FilterValue::All => (0, Vec::new()),
            FilterValue::Unknown => (1, Vec::new()),
            FilterValue::Value(value) => (2, alphabetical_key(locales.text(value))),
        });
        self.genre_filter = self.genre_choices.iter().position(|value_item| value_item == &active).unwrap();
        if let Some(pending) = pending {
            self.option_cursor = self.genre_choices.iter().position(|value_item| value_item == &pending).unwrap();
        }
    }
    /// Rebuild visible game indices while preserving the selected game when possible.
    pub(super) fn refilter(&mut self) {
        let previous = self.selected.and_then(|index_item| self.visible.get(index_item).copied());
        self.visible = self
            .games
            .iter()
            .enumerate()
            .filter_map(|(index_item, game)| {
                let letter_matches = self
                    .letter_filter
                    .is_none_or(|letter| game.initial() == letter);
                let platform_matches = self
                    .platform_filter
                    .as_ref()
                    .is_none_or(|platform_item| &game.platform == platform_item);
                let favourite_matches = !self.favourites_only || self.is_favourite(&game.hash);
                (letter_matches
                    && platform_matches
                    && favourite_matches
                    && self.search.as_ref().is_none_or(|search| search.matches(&game.title))
                    && self.is_trashed(&game.hash) == self.trash_only
                    && self.player_filter().matches(&game.players)
                    && self.genre_filter().matches(&game.genre))
                .then_some(index_item)
            })
            .collect();
        if self.trash_only {
            self.visible.sort_by_key(|&index_item| self.trash[&self.games[index_item].hash]);
        }
        self.selected = previous
            .and_then(|old| self.visible.iter().position(|&index_item| index_item == old))
            .or_else(|| (!self.visible.is_empty()).then_some(0));
    }
}
