// SPDX-License-Identifier: BSD-3-Clause
pub mod images;
mod sample;
pub mod ipc;
mod resources;
pub use sample::{sample_catalogue, renderer};
pub use resources::{ChildGuard, HostGuard, Scratch};

/// Deterministic 64-hex fixture IDs, NOT hashes of real ROMs.
pub fn hash(seed: &str) -> String {
    assert!(seed.len() <= 32);
    let hex: String = seed.bytes().map(|byte| format!("{byte:02x}")).collect();
    format!("{hex:0<64}")
}

pub fn game(image: &str, title: &str, platform: &str) -> igui::model::Game {
    igui::model::Game::new(&hash(image), image, title, platform)
}

pub fn top_tab(app: &mut igui::model::App, tab: igui::model::TopTab<'_>) {
    use {
        imanlib::igui::Action::*,
        igui::model::Focus,
    };
    app.dispatch(Filters);
    assert_eq!(app.focus(), Focus::Platforms);
    for _ in 0..app.platform_tab_count() {
        if app.platform_tab(app.platform_cursor()) == Some(tab) {
            app.dispatch(Activate);
            return;
        }
        app.dispatch(Right);
    }
    panic!("missing tab: {tab:?}");
}

/// Operate the real keyboard through public actions, including shortened rows.
pub fn search_key_actions(app: &igui::model::App, key: char) -> Vec<imanlib::igui::Action> {
    use {
        imanlib::igui::Action::*,
        igui::model::Focus,
        igui::model::search::KEY_ROWS,
    };
    let key = key.to_ascii_uppercase();
    let (row, column) = KEY_ROWS.iter().enumerate()
        .find_map(|(row, keys)| keys.find(key).map(|column| (row, column)))
        .expect("keyboard character");
    let mut actions = Vec::new();
    if app.focus() == Focus::SearchField {
        actions.push(Down);
    } else {
        assert_eq!(app.focus(), Focus::SearchKeys);
    }
    let (mut current_row, mut current_column) = app.search().unwrap().key_position();
    while current_row != row {
        if current_row < row {
            actions.push(Down);
            current_row += 1;
        } else {
            actions.push(Up);
            current_row -= 1;
        }
        current_column = current_column.min(KEY_ROWS[current_row].len() - 1);
    }
    while current_column != column {
        if current_column < column {
            actions.push(Right);
            current_column += 1;
        } else {
            actions.push(Left);
            current_column -= 1;
        }
    }
    actions.push(Activate);
    actions
}

pub fn type_search(app: &mut igui::model::App, text: &str) {
    for key in text.chars() {
        for action in search_key_actions(app, key) {
            assert_eq!(app.dispatch(action), igui::model::Effect::None);
        }
    }
}

pub fn search_field(app: &mut igui::model::App) {
    while app.focus() == igui::model::Focus::SearchKeys {
        app.dispatch(imanlib::igui::Action::Up);
    }
    assert_eq!(app.focus(), igui::model::Focus::SearchField);
}

/// The same dictionary used by the shipped UI; English needs no external file.
pub fn locales(language: &str) -> igui::locales::Locales {
    match language {
        "en" => igui::locales::Locales::default(),
        "ru" => igui::locales::Locales::new(
            igui::locales::Locale::parse("ru", include_bytes!("../../locales/ru.json")).unwrap(),
        ),
        _ => panic!("unsupported fixture language"),
    }
}

/// Shipped JSON resources are read from disk even by controller tests.
pub fn control_bytes(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(igui::paths::CONTROLS_DIR).join(name)).unwrap()
}

pub fn evdev_profiles() -> Vec<imanlib::gamepad::profile::Profile> {
    [EVDEV_20BC_5500_FILE, EVDEV_2563_0555_FILE].into_iter()
        .map(|name| imanlib::gamepad::profile::Profile::parse(&control_bytes(name)).unwrap())
        .collect()
}

pub const HIDRAW_FILE: &str = "2563-0555-hidraw.json";
pub const EVDEV_20BC_5500_FILE: &str = "20bc-5500-evdev.json";
pub const EVDEV_2563_0555_FILE: &str = "2563-0555-evdev.json";
