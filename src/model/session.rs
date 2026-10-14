// SPDX-License-Identifier: BSD-3-Clause
use super::{App, Focus};
use imanlib::igui::GuiState;

impl App {
    /// Only small navigation state crosses process boundaries, never catalogue/art.
    pub fn session_state(&self) -> GuiState {
        GuiState {
            selected_hash: self.selected_game().map(|game| game.hash.clone()),
            letter: self.letter_filter,
            platform: self.platform_filter.clone(),
            favourites_only: self.favourites_only,
            trash_only: self.trash_only,
            players: self.player_filter().clone(),
            genre: self.genre_filter().clone(),
            letter_cursor: self.letter_cursor,
            platform_cursor: self.platform_cursor,
        }
    }

    /// Re-resolve filters against a freshly loaded catalogue; never trust old indices.
    pub fn restore_session(&mut self, state: &GuiState) {
        self.pending_trash = None;
        self.search = None;
        self.letter_filter = state.letter.filter(|value| self.letters.contains(value));
        self.platform_filter = state
            .platform
            .clone()
            .filter(|value| self.platforms.contains(value));
        self.favourites_only = state.favourites_only;
        self.trash_only = state.trash_only;
        self.player_filter = self
            .player_choices
            .iter()
            .position(|value| value == &state.players)
            .unwrap_or(0);
        self.genre_filter = self
            .genre_choices
            .iter()
            .position(|value| value == &state.genre)
            .unwrap_or(0);
        self.letter_cursor = state
            .letter_cursor
            .min(self.letters.len().saturating_sub(1));
        self.platform_cursor = state
            .platform_cursor
            .min(self.platform_tab_count().saturating_sub(1));
        self.refilter();
        if let Some(index) = self
            .visible
            .iter()
            .position(|&index_item| Some(&self.games[index_item].hash) == state.selected_hash.as_ref())
        {
            self.selected = Some(index);
        }
        self.focus = Focus::Games;
        self.update_description();
    }
}
