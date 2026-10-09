
use igui::model::{App, FilterValue, Focus};
use imanlib::igui::GuiState;

#[test]
fn restores_selected_hash_and_all_filters_after_catalogue_reordering() {
    let mut selected = crate::common::game("selected.nes", "Selected", "NES");
    selected.players = Some(2);
    selected.genre = Some("Action".into());
    let hash = selected.hash.clone();
    let mut app = App::new(vec![crate::common::game("different.gen", "Other", "MD"), selected]);
    app.set_favourites([hash.clone()].into());
    app.set_trash([hash.clone()].into());
    let state = GuiState {
        selected_hash: Some(hash.clone()),
        letter: Some('S'),
        platform: Some("NES".into()),
        favourites_only: true,
        trash_only: true,
        players: FilterValue::Value(2),
        genre: FilterValue::Value("Action".into()),
        letter_cursor: 1,
        platform_cursor: 4,
    };

    app.restore_session(&state);

    assert_eq!(app.session_state(), state);
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().hash, hash);
    assert_eq!(app.focus(), Focus::Games);
}

#[test]
fn stale_state_is_clamped_and_unknown_values_are_removed() {
    let mut app = App::new(vec![crate::common::game("one.nes", "One", "NES")]);
    let state = GuiState {
        selected_hash: Some(crate::common::hash("removed")),
        letter: Some('Z'),
        platform: Some("MD".into()),
        players: FilterValue::Value(9),
        genre: FilterValue::Value("Gone".into()),
        letter_cursor: usize::MAX,
        platform_cursor: usize::MAX,
        ..GuiState::default()
    };

    app.restore_session(&state);

    assert_eq!(app.selected_game().unwrap().title, "One");
    assert_eq!(app.letter_filter(), None);
    assert_eq!(app.platform_filter(), None);
    assert_eq!(app.player_filter(), &FilterValue::All);
    assert_eq!(app.genre_filter(), &FilterValue::All);
    assert_eq!(app.letter_cursor(), 0);
    assert_eq!(app.platform_cursor(), 3);
}

#[test]
fn restores_empty_catalogue_without_panic() {
    let mut app = App::new(vec![]);

    app.restore_session(&GuiState::default());

    assert!(app.selected_game().is_none());
    assert_eq!(app.visible_count(), 0);
}
