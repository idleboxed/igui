// SPDX-License-Identifier: BSD-3-Clause

use {
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Effect,
    igui::model::FilterValue,
    igui::model::Focus,
    igui::model::Game,
    igui::model::TopTab,
    igui::model::search::KEY_ROWS,
    igui::model::search::MAX_QUERY,
};
use imanlib::igui::GuiState;

fn catalogue() -> Vec<Game> {
    ["ALPHA", "Super Mario 2", "Mario 20", "мир Mario 2", "Zebra"]
        .into_iter().enumerate().map(|(i, title)| {
            let mut game = crate::common::game(&i.to_string(), title, "NES");
            game.description = Some(format!("Description of {title}"));
            game
        }).collect()
}

fn open(app: &mut App) {
    crate::common::top_tab(app, TopTab::Search);
    assert_eq!(app.focus(), Focus::SearchField);
    assert_eq!(app.search().unwrap().query(), "");
}

fn titles(app: &App) -> Vec<&str> {
    (0..app.visible_count()).map(|i| app.visible_game(i).unwrap().title.as_str()).collect()
}

#[test]
fn start_launches_a_search_result_without_changing_a_selection_behaviour() {
    let mut app = App::new(catalogue());
    open(&mut app);
    crate::common::type_search(&mut app, "mario");
    crate::common::search_field(&mut app);
    app.dispatch(Down);
    while app.focus() != Focus::Games { app.dispatch(Left); }
    assert_eq!(app.focus(), Focus::Games);
    let game = app.selected_game().unwrap().clone();

    assert_eq!(app.dispatch(Start), Effect::Launch {
        image: game.image, title: game.title, platform: game.platform,
    });
    assert!(app.search().is_some());
    assert_eq!(app.dispatch(Activate), Effect::None);
    assert!(app.search().is_none());
}

#[test]
fn search_is_before_favourites_and_available_even_without_games() {
    let mut app = App::new(vec![]);
    assert_eq!(app.platform_tab_count(), 3);
    assert_eq!(app.platform_tab(0), Some(TopTab::Search));
    assert_eq!(app.platform_tab(1), Some(TopTab::Favourites));
    assert_eq!(app.platform_tab(2), Some(TopTab::Trash));
    assert_eq!(app.platform_tab(3), None);

    open(&mut app);
    crate::common::type_search(&mut app, "a");

    assert_eq!(app.visible_count(), 0);
    assert!(app.tab_active(TopTab::Search));
    app.dispatch(Filters);
    assert!(app.search().is_none());
    assert_eq!(app.focus(), Focus::Platforms);
}

#[test]
fn filters_immediately_by_case_insensitive_substring_including_spaces_and_digits() {
    let mut app = App::new(catalogue());
    open(&mut app);

    crate::common::type_search(&mut app, "rio 2");

    assert_eq!(app.search().unwrap().query(), "RIO 2");
    assert_eq!(titles(&app), ["Mario 20", "Super Mario 2", "мир Mario 2"]);
    assert!(app.description_lines().is_empty());
    assert!(app.detail_game().is_none());
    crate::common::type_search(&mut app, "0");
    assert_eq!(titles(&app), ["Mario 20"]);
    crate::common::type_search(&mut app, "9");
    assert!(titles(&app).is_empty());
}

#[test]
fn preserves_selected_identity_then_falls_back_and_clear_removes_only_search() {
    let mut app = App::new(catalogue());
    app.dispatch(Down);
    app.dispatch(Down); // Super Mario 2
    let hash = app.selected_game().unwrap().hash.clone();
    open(&mut app);

    crate::common::type_search(&mut app, "mario 2");
    assert_eq!(app.selected_game().unwrap().hash, hash);
    crate::common::type_search(&mut app, "0");
    assert_eq!(titles(&app), ["Mario 20"]);
    crate::common::search_field(&mut app);
    app.dispatch(Favourite);

    assert_eq!(app.search().unwrap().query(), "");
    assert_eq!(app.search().unwrap().cursor(), 0);
    assert_eq!(app.visible_count(), 5);
    assert_eq!(app.selected_game().unwrap().title, "Mario 20");
}

#[test]
fn field_cursor_insertion_backspace_and_boundaries_do_not_trigger_other_actions() {
    let mut app = App::new(catalogue());
    open(&mut app);
    crate::common::type_search(&mut app, "ab");
    crate::common::search_field(&mut app);
    app.dispatch(Left);
    assert_eq!(app.search().unwrap().cursor(), 1);
    crate::common::type_search(&mut app, "x");
    assert_eq!(app.search().unwrap().query(), "AXB");
    crate::common::search_field(&mut app);

    assert_eq!(app.dispatch(LowerFilters), Effect::None);
    assert_eq!(app.search().unwrap().query(), "AB");
    assert_eq!(app.search().unwrap().cursor(), 1);
    app.dispatch(Left);
    app.dispatch(Left);
    app.dispatch(LowerFilters);
    assert_eq!(app.search().unwrap().query(), "AB");
    assert_eq!(app.search().unwrap().cursor(), 0);
    for _ in 0..4 { app.dispatch(Right); }
    assert_eq!(app.search().unwrap().cursor(), 2);
    for action in [Activate, Trash] {
        assert_eq!(app.dispatch(action), Effect::None);
        assert_eq!(app.focus(), Focus::SearchField);
        assert_eq!(app.search().unwrap().query(), "AB");
    }
    app.dispatch(Favourite);
    assert_eq!(app.search().unwrap().query(), "");
    assert_eq!(app.visible_count(), 5);
}

#[test]
fn keyboard_only_a_inserts_and_no_key_requests_persistence() {
    let mut app = App::new(catalogue());
    open(&mut app);
    app.dispatch(Down);
    for action in [Favourite, LowerFilters, Trash] {
        assert_eq!(app.dispatch(action), Effect::None);
        assert_eq!(app.focus(), Focus::SearchKeys);
        assert_eq!(app.search().unwrap().query(), "");
    }
    app.dispatch(Activate);
    assert_eq!(app.search().unwrap().query(), "0");
}

#[test]
fn all_keyboard_characters_are_reachable_and_query_has_a_bounded_length() {
    let mut app = App::new(catalogue());
    open(&mut app);
    let text = KEY_ROWS.join("");
    crate::common::type_search(&mut app, &text);
    assert_eq!(app.search().unwrap().query(), text);
    crate::common::type_search(&mut app, &"Z".repeat(MAX_QUERY));
    assert_eq!(app.search().unwrap().query().len(), MAX_QUERY);
    assert_eq!(app.search().unwrap().cursor(), MAX_QUERY);
    crate::common::search_field(&mut app);
    app.dispatch(LowerFilters);
    assert_eq!(app.search().unwrap().query().len(), MAX_QUERY - 1);
    crate::common::type_search(&mut app, "Y");
    assert!(app.search().unwrap().query().ends_with('Y'));
}

#[test]
fn keyboard_edges_field_and_header_return_to_results_without_losing_search() {
    let mut app = App::new(catalogue());
    open(&mut app);
    app.dispatch(Up);
    assert_eq!(app.focus(), Focus::Header);
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::SearchField);
    app.dispatch(Down);
    for _ in 0..12 { app.dispatch(Right); }
    assert_eq!(app.search().unwrap().key_position(), (0, 9));
    for _ in 0..3 { app.dispatch(Down); }
    assert_eq!(app.search().unwrap().key_position(), (3, 6));
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::SearchField);
    app.dispatch(Down);
    for _ in 0..7 { app.dispatch(Left); }
    assert_eq!(app.focus(), Focus::Games);
    assert!(app.search().is_some());
    app.dispatch(Right);
    assert_eq!(app.focus(), Focus::SearchField);
    assert_eq!(app.search().unwrap().key_position(), (3, 0));
}

#[test]
fn close_preserves_other_filters_and_reopen_always_starts_empty() {
    let mut games = catalogue();
    for game in &mut games {
        game.players = Some(2);
        game.genre = Some("Action".into());
    }
    let mut app = App::new(games);
    let hash = crate::common::hash("1");
    app.set_favourites([hash.clone()].into());
    app.restore_session(&GuiState {
        selected_hash: Some(hash), letter: Some('S'), platform: Some("NES".into()),
        favourites_only: true, players: FilterValue::Value(2),
        genre: FilterValue::Value("Action".into()), ..GuiState::default()
    });
    let before = app.session_state();
    open(&mut app);
    crate::common::type_search(&mut app, "z");
    assert!(titles(&app).is_empty());

    app.dispatch(Filters);

    assert!(app.search().is_none());
    assert_eq!(app.session_state(), before);
    assert_eq!(titles(&app), ["Super Mario 2"]);
    assert!(!app.description_lines().is_empty());
    open(&mut app);
    assert_eq!(app.visible_count(), 1);
}

#[test]
fn same_top_button_closes_search_and_y_closes_from_results() {
    for close_with_y in [false, true] {
        let mut app = App::new(catalogue());
        open(&mut app);
        crate::common::type_search(&mut app, "a");
        // A is the leftmost key in its row.
        app.dispatch(Left);
        assert_eq!(app.focus(), Focus::Games);
        if close_with_y {
            app.dispatch(Filters);
        } else {
            while app.focus() == Focus::Games { app.dispatch(Up); }
            assert_eq!(app.platform_tab(app.platform_cursor()), Some(TopTab::Search));
            app.dispatch(Activate);
        }
        assert!(app.search().is_none());
        assert_eq!(app.visible_count(), 5);
        assert_eq!(app.focus(), Focus::Platforms);
        assert!(app.detail_game().is_some());
    }
}

#[test]
fn trash_switch_clears_search_with_other_filters_without_showing_hidden_details() {
    let mut app = App::new(catalogue());
    app.set_trash([crate::common::hash("4")].into());
    open(&mut app);
    crate::common::type_search(&mut app, "a");
    app.dispatch(Left);
    while app.focus() == Focus::Games { app.dispatch(Up); }
    for action in [Right, Right, Activate] { app.dispatch(action); }

    assert!(app.trash_only());
    assert_eq!(app.search().unwrap().query(), "");
    assert_eq!(titles(&app), ["Zebra"]);
    assert!(app.description_lines().is_empty());
}

#[test]
fn session_does_not_carry_search_and_restore_also_clears_an_existing_search() {
    let mut app = App::new(catalogue());
    open(&mut app);
    crate::common::type_search(&mut app, "mario");
    app.dispatch(Left); // may stay within the keyboard; session data is independent of focus
    let state = app.session_state();
    let json = serde_json::to_value(&state).unwrap();
    assert!(!json.to_string().contains("query"));
    assert!(!json.to_string().contains("search"));

    app.restore_session(&state);

    assert!(app.search().is_none());
    assert_eq!(app.visible_count(), 5);
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game().unwrap().hash, state.selected_hash.unwrap());
    assert!(!app.description_lines().is_empty());
}

#[test]
fn activating_result_closes_search_keeps_identity_and_only_next_a_launches() {
    let mut app = App::new(catalogue());
    app.set_favourites([crate::common::hash("1"), crate::common::hash("4")].into());
    crate::common::top_tab(&mut app, TopTab::Favourites);
    open(&mut app);
    crate::common::type_search(&mut app, "z");
    assert_eq!(titles(&app), ["Zebra"]);
    while app.focus() != Focus::Games {
        app.dispatch(Left);
    }
    let hash = app.selected_game().unwrap().hash.clone();
    assert_eq!(app.selected_index(), Some(0));

    assert_eq!(app.dispatch(Activate), Effect::None);

    assert!(app.search().is_none());
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game().unwrap().hash, hash);
    assert_eq!(app.selected_index(), Some(1));
    assert_eq!(titles(&app), ["Super Mario 2", "Zebra"]);
    assert!(app.favourites_only());
    assert_eq!(app.description_lines(), ["Description of Zebra"]);
    assert!(matches!(app.dispatch(Activate), Effect::Launch { title, .. } if title == "Zebra"));
}

#[test]
fn activating_an_empty_search_result_does_not_close_search() {
    let mut app = App::new(catalogue());
    open(&mut app);
    crate::common::type_search(&mut app, "00");
    app.dispatch(Left);
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game(), None);

    assert_eq!(app.dispatch(Activate), Effect::None);

    assert_eq!(app.search().unwrap().query(), "00");
    assert_eq!(app.focus(), Focus::Games);
}
