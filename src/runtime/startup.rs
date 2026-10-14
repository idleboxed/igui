// SPDX-License-Identifier: BSD-3-Clause
//! Prepare one complete menu session before publishing it to the UI.
use std::path::{Path, PathBuf};
use crate::{locales::Locales, model::App, paths::PREFERENCES_FILE, storage::{self, Preferences}};
use super::loading::{Cancellation, Stage};

pub struct Session {
    pub app: App,
    pub preferences: Preferences,
    pub root: PathBuf,
}

/// Report each actual operation before it starts. Once translations are ready,
/// pass them to the UI without reading preferences/locales twice. No preview I/O.
pub fn load_session(
    root: &Path,
    locales_root: &Path,
    progress: impl FnMut(Stage, Option<&Locales>),
) -> Result<Session, String> {
    load_session_with_cancellation(root, locales_root, &Cancellation::default(), progress)
}

/// Load checked catalogue, preferences and translations while honoring worker cancellation.
pub(super) fn load_session_with_cancellation(
    root: &Path,
    locales_root: &Path,
    cancellation: &Cancellation,
    mut report: impl FnMut(Stage, Option<&Locales>),
) -> Result<Session, String> {
    let mut progress = |stage: Stage, locales: Option<&Locales>| {
        cancellation.check()?;
        report(stage, locales);
        cancellation.check()
    };
    progress(Stage::Preferences, None)?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let prefs_root = root
        .parent()
        .ok_or_else(|| format!("games directory must have a parent for {PREFERENCES_FILE}"))?;
    let preferences = Preferences::load(prefs_root)?;
    progress(Stage::Locales, None)?;
    let locales = storage::load_locales(locales_root, preferences.language());
    progress(Stage::Catalogue, Some(&locales))?;
    let games = storage::load_catalogue(&root)?;
    progress(Stage::Library, None)?;
    let current_in_trash = preferences.current_item().is_some_and(|hash| {
        preferences.trash().iter().any(|entry| entry == hash) && games.iter().any(|game| game.hash == hash)
    });
    let mut app = App::new(games);
    app.set_favourites(preferences.favourites().clone());
    app.set_trash(preferences.trash().to_vec());
    app.set_locales(locales);
    if let Some(hash) = preferences.current_item() {
        app.restore_session(&imanlib::igui::GuiState {
            selected_hash: Some(hash.to_owned()),
            // A game launched from trash is still an existing catalogue item.
            trash_only: current_in_trash,
            ..Default::default()
        });
    }
    progress(Stage::Storage, None)?;
    app.set_available_bytes(storage::available_bytes(&root));
    Ok(Session {
        app,
        preferences,
        root,
    })
}
