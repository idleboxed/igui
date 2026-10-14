// SPDX-License-Identifier: BSD-3-Clause

use {
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Effect,
    igui::model::FilterField,
    igui::model::FilterValue,
    igui::model::Focus,
    igui::model::Game,
    igui::model::TopTab,
};
use igui::{runtime::startup::load_session, storage::Preferences};
use serde_json::json;
use std::fs;

fn catalogue() -> Vec<Game> {
    [
        ("a", "Amber", "MD", Some(2), Some("Action")),
        ("b", "Amber NES", "NES", Some(2), Some("Action")),
        ("c", "Blue", "MD", Some(4), Some("Action")),
        ("d", "Cloud", "MD", Some(2), Some("Puzzle")),
        ("e", "Unknown", "MD", None, None),
    ]
    .into_iter()
    .map(|(id, title, platform, players, genre)| {
        let mut game = crate::common::game(id, title, platform);
        game.players = players;
        game.genre = genre.map(str::to_owned);
        game.description = Some(format!("Description of {title}"));
        game
    })
    .collect()
}

fn lower(app: &mut App, field: FilterField, index: usize) {
    app.dispatch(LowerFilters);
    if app.lower_field() != field {
        app.dispatch(Right);
    }
    app.dispatch(Activate);
    while app.option_cursor() != index {
        app.dispatch(Down);
    }
    app.dispatch(Activate);
}

fn titles(app: &App) -> Vec<&str> {
    (0..app.visible_count())
        .map(|index_item| app.visible_game(index_item).unwrap().title.as_str())
        .collect()
}

fn assert_no_filters(app: &App) {
    assert_eq!(app.letter_filter(), None);
    assert_eq!(app.platform_filter(), None);
    assert!(!app.favourites_only());
    assert_eq!(app.player_filter(), &FilterValue::All);
    assert_eq!(app.genre_filter(), &FilterValue::All);
}

#[test]
fn trash_requires_confirmation_then_success_removes_the_row_and_updates_details() {
    let mut app = App::new(catalogue());
    app.dispatch(Down); // Amber NES
    let before = titles(&app)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(app.dispatch(Trash), Effect::None);
    assert!(app.trash_confirmation());
    assert_eq!(titles(&app), before);
    assert!(!app.is_trashed(&crate::common::hash("b")));

    assert_eq!(
        app.dispatch(Activate),
        Effect::SaveTrash {
            hash: crate::common::hash("b"),
            enabled: true,
        }
    );
    assert!(!app.trash_confirmation());
    // An unhandled/failed save leaves the current list, selection and details intact.
    assert_eq!(titles(&app), before);
    assert_eq!(app.selected_game().unwrap().image, "b");
    assert_eq!(app.description_lines(), ["Description of Amber NES"]);
    assert!(!app.is_trashed(&crate::common::hash("b")));
    app.set_trash([crate::common::hash("b")].into());
    assert_eq!(titles(&app), ["Amber", "Blue", "Cloud", "Unknown"]);
    assert_eq!(app.selected_game().unwrap().image, "c");
    assert_eq!(app.description_lines(), ["Description of Blue"]);
    assert_eq!(app.total_count(), 5);
    crate::common::top_tab(&mut app, TopTab::Trash);
    assert_eq!(titles(&app), ["Amber NES"]);
    assert_eq!(app.description_lines(), ["Description of Amber NES"]);
}

fn trash_app(mut games: Vec<Game>, restoring: bool) -> (App, Vec<String>) {
    let trash = if restoring {
        games.iter().map(|game| game.hash.clone()).collect()
    } else {
        Vec::new()
    };
    games.reverse(); // Catalogue input order is independent of persisted deletion order.
    let mut app = App::new(games);
    app.set_trash(trash.clone());
    if restoring {
        crate::common::top_tab(&mut app, TopTab::Trash);
        app.dispatch(Down);
    }
    (app, trash)
}

fn publish_trash_action(app: &mut App, trash: &mut Vec<String>, restoring: bool) {
    let hash = app.selected_game().unwrap().hash.clone();
    let mut effect = app.dispatch(Trash);
    if !restoring {
        assert_eq!(effect, Effect::None);
        assert!(app.trash_confirmation());
        effect = app.dispatch(Activate);
    }
    assert_eq!(effect, Effect::SaveTrash { hash: hash.clone(), enabled: !restoring });
    assert!(!app.trash_confirmation());
    if restoring {
        let position = trash.iter().position(|entry| entry == &hash).unwrap();
        trash.remove(position);
    } else {
        assert!(!trash.contains(&hash));
        trash.insert(0, hash);
    }
    app.set_trash(trash.clone());
}

#[test]
fn trash_and_restore_select_next_visible_row_or_previous_last_or_none() {
    for restoring in [false, true] {
        for (single, index, expected) in [
            (false, 0, Some((0, "b", "Amber NES"))),
            (false, 2, Some((2, "d", "Cloud"))),
            (false, 4, Some((3, "d", "Cloud"))),
            (true, 0, None),
        ] {
            let mut games = catalogue();
            if single {
                games.truncate(1);
            }
            let (mut app, mut trash) = trash_app(games, restoring);
            for _ in 0..index {
                app.dispatch(Down);
            }

            publish_trash_action(&mut app, &mut trash, restoring);

            assert_eq!(app.focus(), Focus::Games);
            assert_eq!(app.trash_only(), restoring);
            if let Some((selected, image, title)) = expected {
                assert_eq!(app.selected_index(), Some(selected));
                assert_eq!(app.selected_game().unwrap().image, image);
                assert_eq!(app.description_lines(), [format!("Description of {title}")]);
            } else {
                assert_eq!(app.selected_index(), None);
                assert_eq!(app.visible_count(), 0);
                assert!(app.selected_game().is_none());
                assert!(app.description_lines().is_empty());
            }
        }
    }
}

#[test]
fn trash_and_restore_choose_the_next_row_within_current_filters() {
    for restoring in [false, true] {
        let (mut app, mut trash) = trash_app(catalogue(), restoring);
        app.set_favourites([crate::common::hash("a"), crate::common::hash("c"), crate::common::hash("d")].into());
        crate::common::top_tab(&mut app, TopTab::Platform("MD"));
        crate::common::top_tab(&mut app, TopTab::Favourites);
        app.dispatch(Down);
        app.dispatch(Down);
        assert_eq!(titles(&app), ["Amber", "Blue", "Cloud"]);
        assert_eq!(app.selected_game().unwrap().image, "c");

        publish_trash_action(&mut app, &mut trash, restoring);

        assert_eq!(titles(&app), ["Amber", "Cloud"]);
        assert_eq!(app.selected_index(), Some(1));
        assert_eq!(app.selected_game().unwrap().image, "d");
        assert_eq!(app.description_lines(), ["Description of Cloud"]);
        assert_eq!(app.platform_filter(), Some("MD"));
        assert!(app.favourites_only());
    }
}

#[test]
fn trash_and_restore_choose_the_next_search_result_without_closing_search() {
    for restoring in [false, true] {
        let mut games = catalogue();
        games.push(crate::common::game("f", "Glow", "MD"));
        let (mut app, mut trash) = trash_app(games, restoring);
        crate::common::top_tab(&mut app, TopTab::Search);
        crate::common::type_search(&mut app, "l");
        crate::common::search_field(&mut app);
        app.dispatch(Down);
        while app.focus() != Focus::Games {
            app.dispatch(Left);
        }
        app.dispatch(Down);
        assert_eq!(titles(&app), ["Blue", "Cloud", "Glow"]);
        assert_eq!(app.selected_game().unwrap().image, "d");

        publish_trash_action(&mut app, &mut trash, restoring);

        assert_eq!(titles(&app), ["Blue", "Glow"]);
        assert_eq!(app.selected_index(), Some(1));
        assert_eq!(app.selected_game().unwrap().image, "f");
        assert_eq!(app.search().unwrap().query(), "L");
        assert!(app.detail_game().is_none());
        assert!(app.description_lines().is_empty());
    }
}

#[test]
fn publishing_trash_preserves_a_selected_game_that_remains_visible() {
    let mut app = App::new(catalogue());
    for _ in 0..3 {
        app.dispatch(Down);
    }
    assert_eq!(app.selected_game().unwrap().image, "d");

    app.set_trash([crate::common::hash("a")].into());

    assert_eq!(app.selected_index(), Some(2));
    assert_eq!(app.selected_game().unwrap().image, "d");
    assert_eq!(app.description_lines(), ["Description of Cloud"]);
}

#[test]
fn trash_follows_deletion_order_and_filters_keep_the_same_relative_order() {
    let mut app = App::new(catalogue());
    app.set_trash([
        crate::common::hash("d"), crate::common::hash("b"), crate::common::hash("e"),
        crate::common::hash("a"), crate::common::hash("c"), crate::common::hash("absent"), crate::common::hash("d"),
    ].into());
    app.set_favourites([crate::common::hash("a"), crate::common::hash("c"), crate::common::hash("d")].into());

    crate::common::top_tab(&mut app, TopTab::Trash);

    assert_eq!(titles(&app), ["Cloud", "Amber NES", "Unknown", "Amber", "Blue"]);
    crate::common::top_tab(&mut app, TopTab::Favourites);
    crate::common::top_tab(&mut app, TopTab::Platform("MD"));
    assert_eq!(titles(&app), ["Cloud", "Amber", "Blue"]);
    lower(&mut app, FilterField::Players, 2);
    assert_eq!(titles(&app), ["Cloud", "Amber"]);
    lower(&mut app, FilterField::Genre, 2);
    assert_eq!(titles(&app), ["Amber"]);
    crate::common::top_tab(&mut app, TopTab::Trash);
    assert_eq!(app.visible_count(), 0);
    app.set_trash(vec![]);
    assert_eq!(titles(&app), ["Amber", "Amber NES", "Blue", "Cloud", "Unknown"]);
}

#[test]
fn restoration_advances_in_newest_first_order_not_title_order() {
    let mut app = App::new(catalogue());
    let mut trash = [
        crate::common::hash("e"), crate::common::hash("d"), crate::common::hash("b"), crate::common::hash("c"), crate::common::hash("a"),
    ].to_vec();
    app.set_trash(trash.clone());
    crate::common::top_tab(&mut app, TopTab::Trash);
    app.dispatch(Down); // Enter games.
    app.dispatch(Down); // Cloud.

    publish_trash_action(&mut app, &mut trash, true);

    assert_eq!(titles(&app), ["Unknown", "Amber NES", "Blue", "Amber"]);
    assert_eq!(app.selected_index(), Some(1));
    assert_eq!(app.selected_game().unwrap().image, "b");
    assert_eq!(app.description_lines(), ["Description of Amber NES"]);
    app.dispatch(Down);
    app.dispatch(Down); // Last row: Amber.
    publish_trash_action(&mut app, &mut trash, true);
    assert_eq!(titles(&app), ["Unknown", "Amber NES", "Blue"]);
    assert_eq!(app.selected_index(), Some(2));
    assert_eq!(app.selected_game().unwrap().image, "c");
    assert_eq!(app.description_lines(), ["Description of Blue"]);
}

#[test]
fn saved_deletion_order_survives_session_reload_without_load_time_writes() {
    let dir = tempfile::tempdir_in("/dev/shm").unwrap();
    fs::create_dir(dir.path().join("games")).unwrap();
    let games: Vec<_> = catalogue().into_iter().rev().map(|game| json!({
        "hash":game.hash, "title":game.title, "platform":game.platform, "image":game.image,
        "players":game.players, "genre":game.genre,
    })).collect();
    fs::write(dir.path().join("games/catalogue.json"), json!({"games":games}).to_string()).unwrap();
    let mut prefs = Preferences::load(dir.path()).unwrap();
    for id in ["c", "a", "b"] {
        prefs.save_trash(&crate::common::hash(id), true).unwrap();
    }
    prefs.save_current_item(&crate::common::hash("a")).unwrap();
    let original = fs::read(dir.path().join("prefs.json")).unwrap();

    let session = load_session(&dir.path().join("games"), dir.path(), |_, _| {}).unwrap();

    assert!(session.app.trash_only());
    assert_eq!(titles(&session.app), ["Amber NES", "Amber", "Blue"]);
    assert_eq!(session.app.selected_game().unwrap().image, "a");
    assert_eq!(session.preferences.trash(), &[crate::common::hash("b"), crate::common::hash("a"), crate::common::hash("c")]);
    assert_eq!(fs::read(dir.path().join("prefs.json")).unwrap(), original);
    let state = session.app.session_state();
    let mut reloaded = load_session(&dir.path().join("games"), dir.path(), |_, _| {}).unwrap();
    reloaded.app.restore_session(&state);
    assert_eq!(titles(&reloaded.app), titles(&session.app));
    assert_eq!(reloaded.app.session_state(), state);
    assert_eq!(fs::read(dir.path().join("prefs.json")).unwrap(), original);
}

#[test]
fn toggling_trash_resets_all_filters_and_filters_then_intersect_within_trash() {
    let mut app = App::new(catalogue());
    let favourites = [crate::common::hash("a"), crate::common::hash("b"), crate::common::hash("e")].into();
    app.set_favourites(favourites);
    app.set_trash(
        [
            crate::common::hash("a"),
            crate::common::hash("b"),
            crate::common::hash("c"),
            crate::common::hash("d"),
        ]
        .into(),
    );
    let letters = app.letters().to_vec();
    let platforms = app.platforms().to_vec();
    let players = app.player_choices().to_vec();
    let genres = app.genre_choices().to_vec();
    for _ in 0..2 {
        crate::common::top_tab(&mut app, TopTab::Favourites); // favourites
        crate::common::top_tab(&mut app, TopTab::Platform("MD")); // MD
        app.dispatch(Down);
        app.dispatch(Left);
        app.dispatch(Activate); // A
        lower(&mut app, FilterField::Players, 2); // exactly 2
        lower(&mut app, FilterField::Genre, 2); // Action
        assert!(app.favourites_only());
        assert_eq!(app.platform_filter(), Some("MD"));
        assert_eq!(app.letter_filter(), Some('A'));
        assert_eq!(app.player_filter(), &FilterValue::Value(2));
        assert_eq!(app.genre_filter(), &FilterValue::Value("Action".into()));
        if app.trash_only() {
            assert_eq!(titles(&app), ["Amber"]);
        } else {
            assert!(titles(&app).is_empty()); // no trashed favourite leaks into the main list
        }
        crate::common::top_tab(&mut app, TopTab::Trash);
        assert_no_filters(&app);
        assert_eq!(app.focus(), Focus::Platforms);
        assert_eq!(app.platform_cursor(), 2);
        assert_eq!(app.letters(), letters);
        assert_eq!(app.platforms(), platforms);
        assert_eq!(app.player_choices(), players);
        assert_eq!(app.genre_choices(), genres);
        if app.trash_only() {
            assert_eq!(titles(&app), ["Amber", "Amber NES", "Blue", "Cloud"]);
        } else {
            assert_eq!(titles(&app), ["Unknown"]);
        }
    }
    assert!(!app.trash_only());
}

#[test]
fn restoring_preserves_favourites_and_identity_after_image_rename() {
    let mut game = crate::common::game("original.bin", "Game", "MD");
    let hash = game.hash.clone();
    game.image = "renamed.bin".into();
    let mut app = App::new(vec![game]);
    app.set_favourites([hash.clone()].into());
    app.set_trash([hash.clone(), crate::common::hash("absent")].into());
    assert_eq!(app.visible_count(), 0);
    assert!(app.selected_game().is_none());
    assert!(app.description_lines().is_empty());
    assert_eq!(app.dispatch(Trash), Effect::None);
    crate::common::top_tab(&mut app, TopTab::Trash);
    app.dispatch(Down);
    assert_eq!(app.selected_game().unwrap().image, "renamed.bin");
    assert_eq!(
        app.dispatch(Trash),
        Effect::SaveTrash {
            hash: hash.clone(),
            enabled: false
        }
    );
    assert_eq!(app.visible_count(), 1); // restoration also waits for persistence
    app.set_trash([crate::common::hash("absent")].into());
    assert_eq!(app.visible_count(), 0);
    assert!(app.selected_game().is_none());
    assert!(app.description_lines().is_empty());
    assert_eq!(app.dispatch(Activate), Effect::None);
    crate::common::top_tab(&mut app, TopTab::Trash);
    assert_eq!(app.visible_count(), 1);
    assert!(app.is_favourite(&hash));
    assert_eq!(app.selected_game().unwrap().image, "renamed.bin");
}

#[test]
fn trash_action_is_ignored_outside_games_and_for_an_empty_list() {
    for route in [
        vec![Left],
        vec![Filters],
        vec![Right],
        vec![Right, Down],
        vec![LowerFilters],
        vec![LowerFilters, Activate],
    ] {
        let mut app = App::new(catalogue());
        for action in route {
            app.dispatch(action);
        }
        let focus = app.focus();
        assert_ne!(focus, Focus::Games);
        assert_eq!(app.dispatch(Trash), Effect::None);
        assert!(!app.trash_confirmation());
        assert_eq!(app.focus(), focus);
        assert_eq!(app.visible_count(), 5);
    }
    let mut empty = App::new(vec![]);
    assert_eq!(empty.dispatch(Trash), Effect::None);
    assert!(!empty.trash_confirmation());
    assert_eq!(empty.platform_tab_count(), 3);
    crate::common::top_tab(&mut empty, TopTab::Trash);
    assert!(empty.trash_only());
    assert_eq!(empty.visible_count(), 0);
    crate::common::top_tab(&mut empty, TopTab::Trash);
    assert!(!empty.trash_only());
}

#[test]
fn trash_can_be_filtered_by_unknown_metadata_without_rebuilding_choices() {
    let mut app = App::new(catalogue());
    app.set_trash([crate::common::hash("a"), crate::common::hash("e")].into());
    crate::common::top_tab(&mut app, TopTab::Trash);
    lower(&mut app, FilterField::Players, 1);
    lower(&mut app, FilterField::Genre, 1);
    assert_eq!(titles(&app), ["Unknown"]);
    assert_eq!(app.player_choices().len(), 4);
    assert_eq!(app.genre_choices().len(), 4);
}

#[test]
fn trash_confirmation_blocks_navigation_and_b_cancels_without_favourite_or_save() {
    let mut app = App::new(catalogue());
    app.dispatch(Down);
    let before = app.session_state();
    let description = app.description_lines().to_vec();

    assert_eq!(app.dispatch(Trash), Effect::None);
    for action in [Up, Down, Left, Right, Filters, LowerFilters, Trash] {
        assert_eq!(app.dispatch(action), Effect::None);
        assert!(app.trash_confirmation());
        assert_eq!(app.session_state(), before);
        assert_eq!(app.focus(), Focus::Games);
        assert_eq!(app.description_lines(), description);
    }
    assert_eq!(app.dispatch(Favourite), Effect::None);

    assert!(!app.trash_confirmation());
    assert_eq!(app.session_state(), before);
    assert!(!app.is_trashed(&crate::common::hash("b")));
    assert!(!app.is_favourite(&crate::common::hash("b")));
    assert!(matches!(app.dispatch(Activate), Effect::Launch { .. }));
}

#[test]
fn cancelling_trash_keeps_search_and_can_be_reopened_for_another_game() {
    let mut app = App::new(catalogue());
    crate::common::top_tab(&mut app, TopTab::Search);
    crate::common::type_search(&mut app, "amber");
    while app.focus() != Focus::Games {
        app.dispatch(Left);
    }
    let search = app.search().unwrap().clone();

    app.dispatch(Trash);
    assert_eq!(app.dispatch(Favourite), Effect::None);

    assert!(!app.trash_confirmation());
    assert_eq!(app.search(), Some(&search));
    assert_eq!(titles(&app), ["Amber", "Amber NES"]);
    app.dispatch(Down);
    app.dispatch(Trash);
    assert_eq!(app.dispatch(Activate), Effect::SaveTrash {
        hash: crate::common::hash("b"),
        enabled: true,
    });
    assert!(!app.trash_confirmation());
    assert_eq!(app.search(), Some(&search));
}
