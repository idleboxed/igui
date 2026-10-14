// SPDX-License-Identifier: BSD-3-Clause
//! Pure menu state. External effects are applied by runtime, never by App.
mod filters;
mod game;
mod navigation;
mod session;
pub mod description;
pub mod platforms;
pub mod search;

use std::collections::{BTreeMap, BTreeSet};
use crate::{locales::Locales, memory::MemoryInfo};
use filters::{alphabetical_key, choices};
use search::Search;
pub use game::{Game, Preview};
pub use imanlib::igui::FilterValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Alphabet,
    Platforms,
    Games,
    Details,
    Header,
    LowerFilters,
    FilterOptions,
    SearchField,
    SearchKeys,
}

impl Focus {
    /// Report whether focus belongs to the search field or its keyboard.
    pub fn is_search(self) -> bool {
        matches!(self, Self::SearchField | Self::SearchKeys)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopTab<'a> {
    Search,
    Favourites,
    Trash,
    Platform(&'a str),
}

const TOP_CONTROLS: [TopTab<'static>; 3] = [TopTab::Search, TopTab::Favourites, TopTab::Trash];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Exit,
    SaveFavourite {
        hash: String,
        enabled: bool,
    },
    SaveTrash {
        hash: String,
        enabled: bool,
    },
    Launch {
        image: String,
        title: String,
        platform: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterField {
    Players,
    Genre,
}

/// Navigation and filtering only: no implicit saves or external side effects.
pub struct App {
    games: Vec<Game>,
    favourites: BTreeSet<String>,
    /// Hash -> position in the persisted newest-first deletion list.
    trash: BTreeMap<String, usize>,
    pending_trash: Option<String>,
    letters: Vec<char>,
    platforms: Vec<String>,
    visible: Vec<usize>,
    selected: Option<usize>,
    focus: Focus,
    input_test_enabled: bool,
    input_test_active: bool,
    letter_cursor: usize,
    platform_cursor: usize,
    letter_filter: Option<char>,
    platform_filter: Option<String>,
    favourites_only: bool,
    trash_only: bool,
    available_bytes: Option<u64>,
    memory: Option<MemoryInfo>,
    locales: Locales,
    description_lines: Vec<String>,
    description_offset: usize,
    player_choices: Vec<FilterValue<u8>>,
    genre_choices: Vec<FilterValue<String>>,
    player_filter: usize,
    genre_filter: usize,
    lower_field: FilterField,
    option_cursor: usize,
    search: Option<Search>,
}

impl App {
    /// Sort a catalogue and initialize menu state without performing I/O.
    pub fn new(mut games: Vec<Game>) -> Self {
        games.sort_by_key(|game_item| (game_item.title.to_uppercase(), game_item.platform.clone(), game_item.image.clone()));
        // Tabs come exclusively from the full catalog, never from a fixed
        // alphabet/platform list or the current filtered subset.
        let mut letters: Vec<_> = games
            .iter()
            .map(Game::initial)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        letters.sort_by_cached_key(|letter| alphabetical_key(&letter.to_string()));
        let platforms = games
            .iter()
            .map(|game_item| game_item.platform.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let visible: Vec<_> = (0..games.len()).collect();
        let selected = (!visible.is_empty()).then_some(0);
        let player_choices = choices(games.iter().map(|game_item| game_item.players));
        let genre_choices = choices(games.iter().map(|game_item| game_item.genre.clone()));
        let mut app = Self {
            games,
            favourites: BTreeSet::new(),
            trash: BTreeMap::new(),
            pending_trash: None,
            letters,
            platforms,
            visible,
            selected,
            focus: Focus::Games,
            input_test_enabled: false,
            input_test_active: false,
            letter_cursor: 0,
            platform_cursor: 0,
            letter_filter: None,
            platform_filter: None,
            favourites_only: false,
            trash_only: false,
            available_bytes: None,
            memory: None,
            locales: Locales::default(),
            description_lines: vec![],
            description_offset: 0,
            player_choices,
            genre_choices,
            player_filter: 0,
            genre_filter: 0,
            lower_field: FilterField::Players,
            option_cursor: 0,
            search: None,
        };
        app.sort_genre_choices();
        app.update_description();
        app
    }

    /// Borrow the active search state, if search is open.
    pub fn search(&self) -> Option<&Search> {
        self.search.as_ref()
    }

    /// Hidden details must not cause metadata preparation or image loading.
    pub fn detail_game(&self) -> Option<&Game> {
        if self.search.is_some() {
            None
        } else {
            self.selected_game()
        }
    }
    /// Replace the displayed free-space sample without querying the filesystem.
    pub fn set_available_bytes(&mut self, bytes: Option<u64>) {
        self.available_bytes = bytes;
    }

    /// Return the latest available-space sample, if known.
    pub fn available_bytes(&self) -> Option<u64> {
        self.available_bytes
    }

    /// Replace the displayed memory sample without reading the system.
    pub fn set_memory(&mut self, memory: Option<MemoryInfo>) {
        self.memory = memory;
    }

    /// Return the latest system memory sample, if known.
    pub fn memory(&self) -> Option<MemoryInfo> {
        self.memory
    }

    /// Select translations and reorder genre choices while keeping selections.
    pub fn set_locales(&mut self, locales: Locales) {
        self.locales = locales;
        self.sort_genre_choices();
    }
    /// Borrow the selected translation dictionary and English fallback.
    pub fn locales(&self) -> &Locales {
        &self.locales
    }
    /// Report whether the header's exit control has focus.
    pub fn exit_focused(&self) -> bool {
        self.focus == Focus::Header
    }
    /// Allow or disable controller-test UI, clearing an active test when disabled.
    pub fn enable_input_test(&mut self, enabled: bool) {
        self.input_test_enabled = enabled;
        if !enabled {
            self.input_test_active = false;
        }
    }
    /// Report whether this session permits the controller-test UI.
    pub fn input_test_enabled(&self) -> bool {
        self.input_test_enabled
    }
    /// Mark the controller test active only if the session enabled it.
    pub fn start_input_test(&mut self) {
        if self.input_test_enabled { self.input_test_active = true; }
    }
    /// Report whether the menu is handing control to the controller test.
    pub fn input_test_active(&self) -> bool {
        self.input_test_active
    }
    /// Clear the active controller-test state.
    pub fn close_input_test(&mut self) {
        self.input_test_active = false;
    }
    /// Translate an English UI key using the selected dictionary.
    pub fn text<'a>(&'a self, english: &'a str) -> &'a str {
        self.locales.text(english)
    }
    /// Borrow the wrapped description for the current detail card.
    pub fn description_lines(&self) -> &[String] {
        &self.description_lines
    }
    /// Return the first visible wrapped description line.
    pub fn description_offset(&self) -> usize {
        self.description_offset
    }
    /// Return the furthest valid description scroll offset.
    pub fn description_max_offset(&self) -> usize {
        self.description_lines
            .len()
            .saturating_sub(description::ROWS)
    }

    /// Return the menu region currently receiving navigation actions.
    pub fn focus(&self) -> Focus {
        self.focus
    }
    /// Borrow alphabet tabs derived from the full catalogue.
    pub fn letters(&self) -> &[char] {
        &self.letters
    }
    /// Borrow platform tabs derived from the full catalogue.
    pub fn platforms(&self) -> &[String] {
        &self.platforms
    }
    /// Return the focused alphabet-tab index.
    pub fn letter_cursor(&self) -> usize {
        self.letter_cursor
    }
    /// Return the focused top-tab index.
    pub fn platform_cursor(&self) -> usize {
        self.platform_cursor
    }
    /// Return the active alphabet filter, or None for all initials.
    pub fn letter_filter(&self) -> Option<char> {
        self.letter_filter
    }
    /// Return the active platform filter, or None for all platforms.
    pub fn platform_filter(&self) -> Option<&str> {
        self.platform_filter.as_deref()
    }
    /// Return the selected index within the filtered game list.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }
    /// Return the number of games matching all active filters.
    pub fn visible_count(&self) -> usize {
        self.visible.len()
    }
    /// Return the number of games in the full catalogue.
    pub fn total_count(&self) -> usize {
        self.games.len()
    }
    /// Borrow a game by filtered-list index, or return None outside the list.
    pub fn visible_game(&self, index: usize) -> Option<&Game> {
        self.visible.get(index).map(|&index_item| &self.games[index_item])
    }
    /// Borrow the selected visible game, if the list has a selection.
    pub fn selected_game(&self) -> Option<&Game> {
        self.selected.and_then(|index_item| self.visible_game(index_item))
    }

    /// Borrow player-count choices derived from the full catalogue.
    pub fn player_choices(&self) -> &[FilterValue<u8>] {
        &self.player_choices
    }
    /// Borrow translated-order genre choices derived from the full catalogue.
    pub fn genre_choices(&self) -> &[FilterValue<String>] {
        &self.genre_choices
    }
    /// Borrow the currently applied player-count filter.
    pub fn player_filter(&self) -> &FilterValue<u8> {
        &self.player_choices[self.player_filter]
    }
    /// Borrow the currently applied genre filter.
    pub fn genre_filter(&self) -> &FilterValue<String> {
        &self.genre_choices[self.genre_filter]
    }
    /// Return the selected lower-filter field.
    pub fn lower_field(&self) -> FilterField {
        self.lower_field
    }
    /// Return the pending choice index in the open lower-filter list.
    pub fn option_cursor(&self) -> usize {
        self.option_cursor
    }
    /// Return how many choices the requested lower-filter field has.
    pub fn option_count(&self, field: FilterField) -> usize {
        match field {
            FilterField::Players => self.player_choices.len(),
            FilterField::Genre => self.genre_choices.len(),
        }
    }
    /// Return the applied choice index for the requested lower-filter field.
    pub fn active_option(&self, field: FilterField) -> usize {
        match field {
            FilterField::Players => self.player_filter,
            FilterField::Genre => self.genre_filter,
        }
    }
    /// Format a valid choice index for the requested field using current translations.
    pub fn option_label(&self, field: FilterField, index: usize) -> String {
        match field {
            FilterField::Players => match &self.player_choices[index] {
                FilterValue::All => self.text("All").into(),
                FilterValue::Unknown => self.text("Unknown").into(),
                FilterValue::Value(value) => value.to_string(),
            },
            FilterField::Genre => match &self.genre_choices[index] {
                FilterValue::All => self.text("All").into(),
                FilterValue::Unknown => self.text("Unknown").into(),
                FilterValue::Value(value) => self.text(value).into(),
            },
        }
    }
    fn update_description(&mut self) {
        self.description_lines = self
            .detail_game()
            .and_then(|game| game.description.as_deref())
            .map_or_else(Vec::new, |text| {
                description::wrap(text, description::COLUMNS)
            });
        self.description_offset = 0;
    }
}
