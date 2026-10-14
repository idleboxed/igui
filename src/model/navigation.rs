// SPDX-License-Identifier: BSD-3-Clause
use super::{App, Effect, FilterField, Focus, Search, TopTab};
use imanlib::igui::Action;

impl App {
    fn right_focus(&self) -> Focus {
        if self.search.is_some() {
            Focus::SearchField
        } else {
            Focus::Details
        }
    }

    fn toggle_search(&mut self) {
        if self.search.is_some() {
            self.close_search(Focus::Platforms);
        } else {
            self.search = Some(Search::default());
            self.focus = Focus::SearchField;
            self.refilter();
        }
    }

    fn close_search(&mut self, focus: Focus) {
        self.search = None;
        self.focus = focus;
        if focus == Focus::Platforms {
            self.platform_cursor = 0;
        }
        self.refilter();
    }
    /// Apply one logical action to menu state and return any explicit I/O effect for the runtime.
    pub fn dispatch(&mut self, action: Action) -> Effect {
        use Action::*;
        if self.input_test_active {
            return Effect::None;
        }
        if self.trash_confirmation() {
            return match action {
                Activate => Effect::SaveTrash {
                    hash: self.pending_trash.take().expect("pending trash confirmation"),
                    enabled: true,
                },
                Favourite => {
                    self.pending_trash = None;
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        let previous_game = self.selected.and_then(|index_item| self.visible.get(index_item).copied());
        let was_search = self.search.is_some();
        match (self.focus, action) {
            (_, Filters) if was_search => self.toggle_search(),
            (focus, _) if focus.is_search() => self.dispatch_search(action),
            (_, Filters) => self.focus = Focus::Platforms,
            (Focus::FilterOptions, Favourite) => self.focus = Focus::LowerFilters,
            (Focus::FilterOptions, LowerFilters) => {}
            (_, LowerFilters) => self.focus = Focus::LowerFilters,
            (Focus::LowerFilters, Up) => self.focus = Focus::Games,
            (Focus::LowerFilters, Down) => self.focus = Focus::Platforms,
            (Focus::LowerFilters, Left | Right) => {
                self.lower_field = match self.lower_field {
                    FilterField::Players => FilterField::Genre,
                    FilterField::Genre => FilterField::Players,
                }
            }
            (Focus::LowerFilters, Activate) => {
                self.option_cursor = self.active_option(self.lower_field);
                self.focus = Focus::FilterOptions;
            }
            (Focus::FilterOptions, Up) => {
                self.option_cursor =
                    previous(self.option_cursor, self.option_count(self.lower_field))
            }
            (Focus::FilterOptions, Down) => {
                self.option_cursor = next(self.option_cursor, self.option_count(self.lower_field))
            }
            (Focus::FilterOptions, Activate) => {
                match self.lower_field {
                    FilterField::Players => self.player_filter = self.option_cursor,
                    FilterField::Genre => self.genre_filter = self.option_cursor,
                }
                self.refilter();
                self.focus = Focus::LowerFilters;
            }
            (Focus::Alphabet, Up) => {
                self.letter_cursor = previous(self.letter_cursor, self.letters.len())
            }
            (Focus::Alphabet, Down) => {
                self.letter_cursor = next(self.letter_cursor, self.letters.len())
            }
            (Focus::Alphabet, Right) | (Focus::Platforms, Down) => self.focus = Focus::Games,
            (Focus::Platforms, Up) => self.focus = Focus::LowerFilters,
            (Focus::Platforms, Left) => {
                self.platform_cursor = previous(self.platform_cursor, self.platform_tab_count())
            }
            (Focus::Platforms, Right) => {
                self.platform_cursor = next(self.platform_cursor, self.platform_tab_count())
            }
            (Focus::Alphabet, Activate) => {
                if let Some(&letter) = self.letters.get(self.letter_cursor) {
                    self.letter_filter = (self.letter_filter != Some(letter)).then_some(letter);
                    self.refilter();
                }
            }
            (Focus::Platforms, Activate) => {
                match self.platform_tab(self.platform_cursor) {
                    Some(TopTab::Search) => self.toggle_search(),
                    Some(TopTab::Favourites) => {
                        self.favourites_only = !self.favourites_only;
                        self.refilter();
                    }
                    Some(TopTab::Trash) => {
                        self.trash_only = !self.trash_only;
                        self.letter_filter = None;
                        self.platform_filter = None;
                        self.favourites_only = false;
                        self.player_filter = 0;
                        self.genre_filter = 0;
                        self.option_cursor = 0;
                        if let Some(search) = &mut self.search {
                            search.clear();
                        }
                        self.refilter();
                    }
                    Some(TopTab::Platform(platform)) => {
                        self.platform_filter =
                            (self.platform_filter() != Some(platform)).then(|| platform.to_owned());
                        self.refilter();
                    }
                    None => {}
                }
            }
            (Focus::Games, Left) => self.focus = Focus::Alphabet,
            (Focus::Games, Right) => self.focus = self.right_focus(),
            (Focus::Details, Left) => self.focus = Focus::Games,
            (Focus::Details, Up) if self.description_offset > 0 => {
                self.description_offset -= 1;
            }
            (Focus::Details, Down) if self.description_offset < self.description_max_offset() => {
                self.description_offset += 1;
            }
            (Focus::Details, Up | Down) => self.focus = Focus::Header,
            (Focus::Header, Up | Down) => self.focus = self.right_focus(),
            (Focus::Header, Activate) => return Effect::Exit,
            (Focus::Games, Up) => match self.selected {
                Some(index_item) if index_item > 0 => self.selected = Some(index_item - 1),
                _ => self.focus = Focus::Platforms,
            },
            (Focus::Games, Down) => match self.selected {
                Some(index_item) if index_item + 1 < self.visible.len() => self.selected = Some(index_item + 1),
                _ => self.focus = Focus::LowerFilters,
            },
            (Focus::Games, Favourite) => {
                if let Some(game) = self.selected_game() {
                    return Effect::SaveFavourite {
                        hash: game.hash.clone(),
                        enabled: !self.is_favourite(&game.hash),
                    };
                }
            }
            (Focus::Games, Trash) => {
                if let Some(game) = self.selected_game() {
                    if self.is_trashed(&game.hash) {
                        return Effect::SaveTrash {
                            hash: game.hash.clone(),
                            enabled: false,
                        };
                    }
                    self.pending_trash = Some(game.hash.clone());
                }
            }
            (Focus::Games, Activate) if was_search && self.selected.is_some() => {
                self.close_search(Focus::Games);
            }
            (Focus::Games, Activate | Start) => {
                if let Some(game) = self.selected_game() {
                    return Effect::Launch {
                        image: game.image.clone(),
                        title: game.title.clone(),
                        platform: game.platform.clone(),
                    };
                }
            }
            _ => {}
        }
        if was_search != self.search.is_some()
            || previous_game != self.selected.and_then(|index_item| self.visible.get(index_item).copied())
        {
            self.update_description();
        }
        Effect::None
    }

    fn dispatch_search(&mut self, action: Action) {
        use Action::*;
        let search = self.search.as_mut().expect("search focus requires an open search");
        let changed = match (self.focus, action) {
            (Focus::SearchField, Favourite) => search.clear(),
            (Focus::SearchField, LowerFilters) => search.backspace(),
            (Focus::SearchKeys, Activate) => search.insert_key(),
            (Focus::SearchField, Left | Right) => {
                search.move_cursor(action == Right);
                false
            }
            (Focus::SearchField, Down) => {
                self.focus = Focus::SearchKeys;
                false
            }
            (Focus::SearchField, Up) => {
                self.focus = Focus::Header;
                false
            }
            (Focus::SearchKeys, Up | Down) => {
                if !search.move_key(if action == Up { -1 } else { 1 }, 0) {
                    self.focus = Focus::SearchField;
                }
                false
            }
            (Focus::SearchKeys, Left | Right) => {
                if !search.move_key(0, if action == Left { -1 } else { 1 }) && action == Left {
                    self.focus = Focus::Games;
                }
                false
            }
            _ => false,
        };
        if changed {
            self.refilter();
        }
    }
}

fn previous(index: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else if index == 0 {
        count - 1
    } else {
        index - 1
    }
}

fn next(index: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        (index + 1) % count
    }
}
