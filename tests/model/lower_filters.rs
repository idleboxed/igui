
use {
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Effect,
    igui::model::FilterField,
    igui::model::FilterValue,
    igui::model::Focus,
    igui::model::Game,
};
use FilterValue::{All, Unknown, Value};

fn catalog() -> Vec<Game> {
    [
        ("a", "Amber", "MD", Some(2), Some("Action")),
        ("b", "Amber NES", "NES", Some(2), Some("Action")),
        ("c", "Blue", "MD", Some(4), Some("Action")),
        ("d", "Cloud", "MD", Some(2), Some("Puzzle")),
        ("e", "Dawn", "NES", Some(1), Some("Puzzle")),
        ("f", "Unknown", "MD", None, None),
    ]
    .into_iter()
    .map(|(id, title, platform, players, genre)| {
        let mut game = crate::common::game(id, title, platform);
        game.players = players;
        game.genre = genre.map(str::to_owned);
        game
    })
    .collect()
}

fn genre_catalog() -> Vec<Game> {
    let mut games: Vec<_> = ["Racing", "Puzzle", "Platformer", "Arcade", "Adventure"]
        .into_iter()
        .map(|genre| {
            let mut game = crate::common::game(genre, genre, "MD");
            game.genre = Some(genre.into());
            game
        })
        .collect();
    games.push(crate::common::game("unknown", "Unknown", "NES"));
    games
}

fn select(app: &mut App, field: FilterField, index: usize) {
    app.dispatch(LowerFilters);
    if app.lower_field() != field {
        app.dispatch(Right);
    }
    app.dispatch(Activate);
    while app.option_cursor() != index {
        app.dispatch(Down);
    }
    assert_eq!(app.dispatch(Activate), Effect::None);
    assert_eq!(app.focus(), Focus::LowerFilters);
}

#[test]
fn choices_come_from_full_catalogue_and_players_is_exact_not_a_minimum() {
    let mut app = App::new(catalog());
    assert_eq!(
        app.player_choices(),
        &[All, Unknown, Value(1), Value(2), Value(4)]
    );
    assert_eq!(
        app.genre_choices(),
        &[All, Unknown, Value("Action".into()), Value("Puzzle".into())]
    );
    let players = app.player_choices().to_vec();
    let genres = app.genre_choices().to_vec();
    select(&mut app, FilterField::Players, 3);
    assert_eq!(app.player_filter(), &Value(2));
    assert_eq!(app.visible_count(), 3);
    for i in 0..app.visible_count() {
        assert_eq!(app.visible_game(i).unwrap().players, Some(2));
    }
    select(&mut app, FilterField::Genre, 3);
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().image, "d");
    assert_eq!(app.player_choices(), players);
    assert_eq!(app.genre_choices(), genres);
    select(&mut app, FilterField::Players, 0);
    assert_eq!(app.visible_count(), 2);
    select(&mut app, FilterField::Genre, 0);
    assert_eq!(app.visible_count(), 6);
}

#[test]
fn all_five_filters_intersect_without_resetting_each_other() {
    let mut app = App::new(catalog());
    app.set_favourites([crate::common::hash("a"), crate::common::hash("b"), crate::common::hash("c")].into());
    select(&mut app, FilterField::Players, 3); // exactly 2
    select(&mut app, FilterField::Genre, 2); // Action
    for action in [
        Filters, Right, Activate, Right, Right, Activate, Down, Left, Activate,
    ] {
        assert_eq!(app.dispatch(action), Effect::None);
    }
    assert!(app.favourites_only());
    assert_eq!(app.platform_filter(), Some("MD"));
    assert_eq!(app.letter_filter(), Some('A'));
    assert_eq!(app.player_filter(), &Value(2));
    assert_eq!(app.genre_filter(), &Value("Action".into()));
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().image, "a");
}

#[test]
fn b_cancels_pending_value_without_applying_or_saving_and_reopen_restores_active_value() {
    let mut app = App::new(catalog());
    select(&mut app, FilterField::Players, 3);
    let selected = app.selected_game().unwrap().hash.clone();
    for action in [Activate, Down, LowerFilters] {
        assert_eq!(app.dispatch(action), Effect::None);
    }
    assert_eq!(app.focus(), Focus::FilterOptions); // A is not the cancel button.
    assert_eq!(app.option_cursor(), 4);
    assert_eq!(app.visible_count(), 3); // Cursor alone has no effect.
    assert_eq!(app.dispatch(Favourite), Effect::None); // B never saves here.
    assert_eq!(app.focus(), Focus::LowerFilters);
    assert_eq!(app.player_filter(), &Value(2));
    assert_eq!(app.selected_game().unwrap().hash, selected);
    app.dispatch(Activate);
    assert_eq!(app.option_cursor(), 3);
    app.dispatch(Down);
    app.dispatch(Filters); // Y also leaves the popup without applying.
    assert_eq!(app.focus(), Focus::Platforms);
    assert_eq!(app.player_filter(), &Value(2));
}

#[test]
fn lower_shortcut_preserves_game_and_bottom_boundary_requires_an_extra_press() {
    for route in [
        vec![],
        vec![Left],
        vec![Right],
        vec![Right, Up],
        vec![Filters],
    ] {
        let mut app = App::new(catalog());
        app.dispatch(Down);
        let selected = app.selected_game().unwrap().hash.clone();
        for action in route {
            app.dispatch(action);
        }
        app.dispatch(LowerFilters);
        assert_eq!(app.focus(), Focus::LowerFilters);
        app.dispatch(Up);
        assert_eq!(app.focus(), Focus::Games);
        assert_eq!(app.selected_game().unwrap().hash, selected);
    }
    let mut app = App::new(catalog());
    for _ in 1..app.visible_count() {
        app.dispatch(Down);
    }
    assert_eq!(app.focus(), Focus::Games);
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::LowerFilters);
    app.dispatch(Up);
    assert_eq!(app.selected_index(), Some(5));
}

#[test]
fn unknown_empty_and_wrapped_options_have_defined_navigation() {
    let mut app = App::new(catalog());
    select(&mut app, FilterField::Players, 1);
    assert_eq!(app.selected_game().unwrap().image, "f");
    select(&mut app, FilterField::Genre, 2);
    assert_eq!(app.visible_count(), 0);
    assert!(app.description_lines().is_empty());
    select(&mut app, FilterField::Genre, 1);
    assert_eq!(app.visible_count(), 1);
    app.dispatch(Activate);
    app.dispatch(Up); // All
    app.dispatch(Up); // wrap to Puzzle
    assert_eq!(app.option_cursor(), 3);
    app.dispatch(Down);
    assert_eq!(app.option_cursor(), 0);
    let mut empty = App::new(vec![]);
    assert_eq!(empty.player_choices(), &[All]);
    assert_eq!(empty.genre_choices(), &[All]);
    for action in [Down, Activate, Up, Down, Activate, Up] {
        assert_eq!(empty.dispatch(action), Effect::None);
    }
    assert_eq!(empty.focus(), Focus::Games);
    assert!(empty.selected_game().is_none());
}

#[test]
fn genre_values_stay_english_while_labels_are_localized() {
    let mut app = App::new(catalog());
    app.set_locales(crate::common::locales("ru"));
    assert_eq!(app.option_label(FilterField::Players, 0), "Все");
    assert_eq!(app.option_label(FilterField::Players, 1), "Неизвестно");
    assert_eq!(app.option_label(FilterField::Genre, 3), "Головоломка");
    select(&mut app, FilterField::Genre, 3);
    assert_eq!(app.genre_filter(), &Value("Puzzle".into()));
}

#[test]
fn genres_are_alphabetical_by_displayed_label_in_english_and_russian() {
    for (language, expected) in [
        ("en", ["All", "Unknown", "Adventure", "Arcade", "Platformer", "Puzzle", "Racing"]),
        ("ru", ["Все", "Неизвестно", "Аркада", "Головоломка", "Гонки", "Платформер", "Приключение"]),
    ] {
        let mut app = App::new(genre_catalog());
        app.set_locales(crate::common::locales(language));
        let labels: Vec<_> = (0..app.option_count(FilterField::Genre))
            .map(|index| app.option_label(FilterField::Genre, index))
            .collect();

        assert_eq!(labels, expected);
        let puzzle = app.genre_choices().iter().position(|value| value == &Value("Puzzle".into())).unwrap();
        select(&mut app, FilterField::Genre, puzzle);
        assert_eq!(app.genre_filter(), &Value("Puzzle".into()));
        assert_eq!(app.visible_count(), 1);
        assert_eq!(app.selected_game().unwrap().image, "Puzzle");
    }
}

#[test]
fn sorting_ignores_case_and_player_counts_remain_numeric() {
    let mut games: Vec<_> = [("Zulu", 12), ("alpha", 2), ("Beta", 1), ("Action", 2)]
        .into_iter()
        .map(|(genre, players)| {
            let mut game = crate::common::game(genre, genre, "MD");
            game.genre = Some(genre.into());
            game.players = Some(players);
            game
        })
        .collect();
    games.push(crate::common::game("unknown", "Unknown", "MD"));

    let app = App::new(games);

    assert_eq!(app.player_choices(), &[All, Unknown, Value(1), Value(2), Value(12)]);
    assert_eq!(app.genre_choices(), &[
        All, Unknown, Value("Action".into()), Value("alpha".into()), Value("Beta".into()), Value("Zulu".into()),
    ]);
}

#[test]
fn russian_genre_sorting_places_yo_after_ye_and_before_zhe() {
    let translations = serde_json::json!({
        "First":"Ёж", "Second":"Енот", "Third":"ель", "Fourth":"Жанр", "Fifth":"Аркада"
    });
    let games = ["First", "Second", "Third", "Fourth", "Fifth"].into_iter().map(|genre| {
        let mut game = crate::common::game(genre, genre, "MD");
        game.genre = Some(genre.into());
        game
    }).collect();
    let locales = igui::locales::Locales::new(
        igui::locales::Locale::parse("ru", translations.to_string().as_bytes()).unwrap(),
    );
    let mut app = App::new(games);

    app.set_locales(locales);

    let labels: Vec<_> = (0..app.option_count(FilterField::Genre))
        .map(|index| app.option_label(FilterField::Genre, index))
        .collect();
    assert_eq!(labels, ["All", "Аркада", "ель", "Енот", "Ёж", "Жанр"]);
}

#[test]
fn alphabet_and_platform_tabs_are_sorted_and_deduplicated() {
    let games = [
        ("Жук", "SNES"), ("ёлка", "NES"), ("Яма", "GBA"),
        ("ель", "MD"), ("Арка", "MD"), ("ёж", "NES"), ("2 игры", "GBA"),
    ].into_iter().enumerate().map(|(index, (title, platform))| {
        crate::common::game(&index.to_string(), title, platform)
    }).collect();

    let app = App::new(games);

    assert_eq!(app.letters(), &['#', 'А', 'Е', 'Ё', 'Ж', 'Я']);
    assert_eq!(app.platforms(), &["GBA", "MD", "NES", "SNES"]);
}

#[test]
fn resorting_genres_preserves_active_filter_pending_value_and_selected_game() {
    let mut app = App::new(genre_catalog());
    select(&mut app, FilterField::Genre, 2); // Adventure
    let selected = app.selected_game().unwrap().hash.clone();
    app.dispatch(Activate);
    while app.option_cursor() != 6 {
        app.dispatch(Down);
    }

    app.set_locales(crate::common::locales("ru"));

    assert_eq!(app.focus(), Focus::FilterOptions);
    assert_eq!(app.genre_filter(), &Value("Adventure".into()));
    assert_eq!(app.active_option(FilterField::Genre), 6);
    assert_eq!(app.option_cursor(), 4); // Racing / Гонки
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().hash, selected);
    app.set_locales(crate::common::locales("en"));
    assert_eq!(app.active_option(FilterField::Genre), 2);
    assert_eq!(app.option_cursor(), 6);
    app.dispatch(Activate);
    assert_eq!(app.genre_filter(), &Value("Racing".into()));
    assert_eq!(app.selected_game().unwrap().image, "Racing");
}

#[test]
fn image_rename_keeps_favourites_because_identity_is_the_hash() {
    let mut game = crate::common::game("old.bin", "Game", "MD");
    let hash = game.hash.clone();
    game.image = "new.bin".into();
    let mut app = App::new(vec![game]);
    app.set_favourites([hash.clone()].into());
    assert_eq!(
        app.dispatch(Favourite),
        Effect::SaveFavourite {
            hash,
            enabled: false
        }
    );
    for action in [Filters, Right, Activate, Down] {
        app.dispatch(action);
    }
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().image, "new.bin");
}

#[test]
fn panel_wrap_preserves_cursors_selection_and_all_filters() {
    let mut app = App::new(catalog());
    app.set_favourites([crate::common::hash("a"), crate::common::hash("b")].into());
    select(&mut app, FilterField::Players, 3); // 2 players
    select(&mut app, FilterField::Genre, 2); // Action
    for action in [
        Filters, Right, Activate, Right, Right, Activate, Down, Left, Activate, Right,
    ] {
        app.dispatch(action);
    }
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().image, "a");
    app.dispatch(LowerFilters);
    for _ in 0..3 {
        assert_eq!(app.dispatch(Down), Effect::None);
        assert_eq!(app.focus(), Focus::Platforms);
        assert_eq!(app.platform_cursor(), 3);
        assert_eq!(app.dispatch(Up), Effect::None);
        assert_eq!(app.focus(), Focus::LowerFilters);
        assert_eq!(app.lower_field(), FilterField::Genre);
        assert_eq!(app.player_filter(), &Value(2));
        assert_eq!(app.genre_filter(), &Value("Action".into()));
        assert_eq!(app.letter_filter(), Some('A'));
        assert_eq!(app.platform_filter(), Some("MD"));
        assert!(app.favourites_only());
        assert_eq!(app.selected_game().unwrap().image, "a");
    }
    app.dispatch(Up);
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game().unwrap().image, "a");
}

#[test]
fn panel_wrap_also_works_with_empty_results_but_not_inside_a_dropdown() {
    let mut empty = App::new(vec![]);
    for (action, expected) in [
        (Down, Focus::LowerFilters),
        (Down, Focus::Platforms),
        (Up, Focus::LowerFilters),
        (Up, Focus::Games),
    ] {
        assert_eq!(empty.dispatch(action), Effect::None);
        assert_eq!(empty.focus(), expected);
        assert!(empty.selected_game().is_none());
    }
    let mut app = App::new(catalog());
    app.dispatch(LowerFilters);
    app.dispatch(Activate);
    app.dispatch(Up); // Wrap within the dropdown, not to the top panel.
    assert_eq!(app.focus(), Focus::FilterOptions);
    assert_eq!(app.option_cursor(), 4);
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::FilterOptions);
    assert_eq!(app.option_cursor(), 0);
}
