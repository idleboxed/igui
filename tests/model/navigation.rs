
use {
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Effect,
    igui::model::Focus,
    igui::model::Game,
};

#[test]
fn favourite_requests_persistence_without_launching_or_changing_selection() {
    let mut app = App::new(catalog());
    assert_eq!(
        app.dispatch(Favourite),
        Effect::SaveFavourite {
            hash: crate::common::hash("amber-md"),
            enabled: true,
        }
    );
    // A failed or unhandled save must not turn the star on.
    assert!(!app.is_favourite(&crate::common::hash("amber-md")));
    app.set_favourites([crate::common::hash("amber-md")].into());
    assert_eq!(
        app.dispatch(Favourite),
        Effect::SaveFavourite {
            hash: crate::common::hash("amber-md"),
            enabled: false,
        }
    );
    assert_eq!(app.selected_index(), Some(0));
    app.dispatch(Down);
    assert!(!app.is_favourite(&app.selected_game().unwrap().hash));
    app.dispatch(Left);
    assert_eq!(app.dispatch(Favourite), Effect::None);
    app.dispatch(Right);
    app.dispatch(Up);
    app.dispatch(Up);
    assert_eq!(app.dispatch(Favourite), Effect::None);
    assert_eq!(App::new(vec![]).dispatch(Favourite), Effect::None);
}

fn catalog() -> Vec<Game> {
    vec![
        crate::common::game("amber-md", "Amber", "MD"),
        crate::common::game("amber-nes", "Amber", "NES"),
        crate::common::game("blue", "Blue", "MD"),
        crate::common::game("castle", "Castle", "NES"),
    ]
}

#[test]
fn opens_unfiltered_on_first_game() {
    let app = App::new(catalog());
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.visible_count(), 4);
    assert_eq!(app.selected_game().unwrap().image, "amber-md");
    assert_eq!(app.letter_filter(), None);
    assert_eq!(app.platform_filter(), None);
}

#[test]
fn x_confirms_b_favourites_y_opens_filters_and_a_opens_lower_filters() {
    use imanlib::igui::Action;
    for (key, action) in [
        ("a", Activate),
        ("enter", Activate),
        ("b", Favourite),
        ("y", Filters),
        ("x", LowerFilters),
        ("select", Trash),
        ("delete", Trash),
        ("start", Start),
    ] {
        assert_eq!(Action::parse(key), Some(action));
    }
    assert_eq!(Action::parse("q"), None);
}

#[test]
fn start_launches_the_selected_game_but_never_confirms_other_controls() {
    for route in [vec![], vec![Down], vec![Down, Down]] {
        let mut app = App::new(catalog());
        for action in route { app.dispatch(action); }
        let game = app.selected_game().unwrap().clone();

        assert_eq!(app.dispatch(Start), Effect::Launch {
            image: game.image, title: game.title, platform: game.platform,
        });
    }
    for route in [
        vec![Left], vec![Right], vec![Right, Up], vec![Filters],
        vec![LowerFilters], vec![LowerFilters, Activate], vec![Trash],
        vec![Filters, Activate], vec![Filters, Activate, Down],
    ] {
        let mut app = App::new(catalog());
        for action in route { app.dispatch(action); }
        let focus = app.focus();
        let state = app.session_state();
        let confirming = app.trash_confirmation();

        assert_eq!(app.dispatch(Start), Effect::None);
        assert_eq!(app.focus(), focus);
        assert_eq!(app.session_state(), state);
        assert_eq!(app.trash_confirmation(), confirming);
    }
    assert_eq!(App::new(vec![]).dispatch(Start), Effect::None);
}

#[test]
fn y_reaches_top_filters_from_any_row_and_down_returns_to_the_same_game() {
    let mut app = App::new(catalog());
    for action in [Filters, Right, Right, Right, Down, Down, Down] {
        app.dispatch(action);
    }
    assert_eq!(app.selected_index(), Some(2));
    assert_eq!(app.platform_cursor(), 3);
    let image = app.selected_game().unwrap().image.clone();
    assert_eq!(app.dispatch(Filters), Effect::None);
    assert_eq!(app.focus(), Focus::Platforms);
    assert_eq!(app.platform_cursor(), 3);
    assert_eq!(app.platform_filter(), None); // Y does not select the focused MD tab.
    assert_eq!(app.letter_filter(), None);
    assert!(!app.favourites_only());
    assert_eq!(app.visible_count(), 4);
    assert_eq!(app.dispatch(Filters), Effect::None); // No toggle on another press.
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game().unwrap().image, image);
}

#[test]
fn y_also_leaves_letters_details_header_and_empty_results_without_side_effects() {
    for empty in [false, true] {
        for route in [vec![Left], vec![Right], vec![Right, Up]] {
            let mut app = App::new(if empty { vec![] } else { catalog() });
            for action in route {
                app.dispatch(action);
            }
            let selected = app.selected_index();
            assert_eq!(app.dispatch(Filters), Effect::None);
            assert_eq!(app.focus(), Focus::Platforms);
            assert_eq!(app.selected_index(), selected);
            assert_eq!(app.platform_filter(), None);
            assert_eq!(app.locales().language(), "en");
        }
    }
}

#[test]
fn up_from_platform_filter_goes_to_lower_filters_without_changing_game_or_filter() {
    let mut app = App::new(catalog());
    app.dispatch(Up);
    app.dispatch(Right); // skip search
    app.dispatch(Right); // skip favourites
    app.dispatch(Right); // skip trash
    app.dispatch(Activate); // MD
    app.dispatch(Up);
    assert_eq!(app.focus(), Focus::LowerFilters);
    assert_eq!(app.selected_game().unwrap().image, "amber-md");
    assert_eq!(app.platform_filter(), Some("MD"));
    assert_eq!(app.selected_index(), Some(0));
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::Platforms);
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game().unwrap().image, "amber-md");
    let mut empty = App::new(vec![]);
    empty.dispatch(Up);
    empty.dispatch(Up);
    assert_eq!(empty.focus(), Focus::LowerFilters);
    assert_eq!(empty.selected_index(), None);
}

#[test]
fn filter_tabs_are_only_unique_values_from_the_supplied_catalog() {
    let app = App::new(vec![
        crate::common::game("one", "Zebra", "CUSTOM"),
        crate::common::game("two", "zulu", "CUSTOM"),
        crate::common::game("three", "ёжик", "PICO"),
        crate::common::game("four", "7 puzzles", "PICO"),
    ]);
    assert_eq!(app.letters(), &['#', 'Z', 'Ё']);
    assert_eq!(app.platforms(), &["CUSTOM", "PICO"]);

    let different = App::new(vec![crate::common::game("only", "Башня", "NEW")]);
    assert_eq!(different.letters(), &['Б']);
    assert_eq!(different.platforms(), &["NEW"]);

    let empty = App::new(vec![]);
    assert!(empty.letters().is_empty());
    assert!(empty.platforms().is_empty());
}

#[test]
fn boundaries_need_another_press_and_do_not_wrap_games() {
    let mut app = App::new(catalog());
    for _ in 0..3 {
        app.dispatch(Down);
    }
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_index(), Some(3));
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::LowerFilters);
    app.dispatch(Up);
    assert_eq!(app.selected_index(), Some(3));
    for _ in 0..3 {
        app.dispatch(Up);
    }
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_index(), Some(0));
    app.dispatch(Up);
    assert_eq!(app.focus(), Focus::Platforms);
}

#[test]
fn alphabet_wraps_but_moving_focus_does_not_apply_filter() {
    let mut app = App::new(catalog());
    app.dispatch(Left);
    app.dispatch(Up);
    assert_eq!(app.letter_cursor(), 2);
    assert_eq!(app.visible_count(), 4);
    app.dispatch(Down);
    assert_eq!(app.letter_cursor(), 0);
    app.dispatch(Down);
    app.dispatch(Activate);
    assert_eq!(app.letter_filter(), Some('B'));
    assert_eq!(app.selected_game().unwrap().image, "blue");
    app.dispatch(Activate);
    assert_eq!(app.letter_filter(), None);
    assert_eq!(app.visible_count(), 4);
    assert_eq!(app.selected_game().unwrap().image, "blue");
}

#[test]
fn platform_wraps_and_single_selection_replaces_previous() {
    let mut app = App::new(catalog());
    app.dispatch(Up);
    app.dispatch(Left);
    assert_eq!(app.platform_cursor(), 4);
    assert_eq!(app.platform_filter(), None);
    app.dispatch(Activate);
    assert_eq!(app.platform_filter(), Some("NES"));
    app.dispatch(Right);
    assert_eq!(app.platform_cursor(), 0);
    app.dispatch(Right); // skip search
    app.dispatch(Right); // skip favourites
    app.dispatch(Right); // skip trash
    app.dispatch(Activate);
    assert_eq!(app.platform_filter(), Some("MD"));
    assert_eq!(app.visible_count(), 2);
    app.dispatch(Activate);
    assert_eq!(app.platform_filter(), None);
    assert_eq!(app.visible_count(), 4);
}

#[test]
fn filters_intersect_and_tab_sets_remain_stable() {
    let mut app = App::new(catalog());
    let letters = app.letters().to_vec();
    let platforms = app.platforms().to_vec();
    app.dispatch(Up);
    app.dispatch(Right);
    app.dispatch(Right);
    app.dispatch(Right);
    app.dispatch(Right); // skip search
    app.dispatch(Activate); // NES
    app.dispatch(Down);
    app.dispatch(Left);
    app.dispatch(Activate); // A
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().image, "amber-nes");
    app.dispatch(Down);
    app.dispatch(Activate); // B + NES -> empty
    assert_eq!(app.visible_count(), 0);
    assert!(app.selected_game().is_none());
    assert_eq!(app.letters(), letters);
    assert_eq!(app.platforms(), platforms);
    app.dispatch(Right);
    assert_eq!(app.dispatch(Activate), Effect::None);
    app.dispatch(Up);
    assert_eq!(app.focus(), Focus::Platforms);
    app.dispatch(Activate); // remove NES, keep B
    assert_eq!(app.selected_game().unwrap().image, "blue");
}

#[test]
fn selection_is_preserved_by_identity_not_row_number() {
    let mut app = App::new(catalog());
    app.dispatch(Down);
    app.dispatch(Down); // Blue
    app.dispatch(Left);
    app.dispatch(Down);
    app.dispatch(Activate); // B
    assert_eq!(app.selected_game().unwrap().image, "blue");
    app.dispatch(Up);
    app.dispatch(Activate); // A removes Blue -> first result
    assert_eq!(app.selected_game().unwrap().image, "amber-md");
}

#[test]
fn remembers_each_regions_cursor() {
    let mut app = App::new(catalog());
    app.dispatch(Down);
    app.dispatch(Left);
    app.dispatch(Down);
    app.dispatch(Right);
    assert_eq!(app.selected_index(), Some(1));
    app.dispatch(Left);
    assert_eq!(app.letter_cursor(), 1);
    app.dispatch(Right);
    app.dispatch(Up);
    app.dispatch(Up);
    app.dispatch(Right);
    app.dispatch(Down);
    app.dispatch(Up);
    assert_eq!(app.platform_cursor(), 1);
}

#[test]
fn only_game_activation_emits_a_launch_request() {
    let mut app = App::new(catalog());
    assert_eq!(
        app.dispatch(Activate),
        Effect::Launch {
            image: "amber-md".into(),
            title: "Amber".into(),
            platform: "MD".into(),
        }
    );
    app.dispatch(Left);
    assert_eq!(app.dispatch(Activate), Effect::None);
    app.dispatch(Right);
    app.dispatch(Up);
    assert_eq!(app.dispatch(Activate), Effect::None);
}

#[test]
fn empty_catalog_and_one_item_have_defined_navigation() {
    for mut app in [
        App::new(vec![]),
        App::new(vec![crate::common::game("one", "Only", "GB")]),
    ] {
        let count = app.visible_count();
        app.dispatch(Down);
        assert_eq!(app.focus(), Focus::LowerFilters);
        app.dispatch(Filters);
        app.dispatch(Right);
        app.dispatch(Activate);
        app.dispatch(Activate); // turn favourites back off, without changing a platform
        app.dispatch(Down);
        app.dispatch(Left);
        app.dispatch(Up);
        app.dispatch(Down);
        app.dispatch(Activate);
        app.dispatch(Right);
        assert_eq!(app.visible_count(), count);
    }
}

#[test]
fn favourites_are_an_independent_tab_and_intersect_with_platform_and_letter() {
    let mut app = App::new(catalog());
    app.set_favourites(
        [
            crate::common::hash("amber-md"),
            crate::common::hash("amber-nes"),
            crate::common::hash("blue"),
        ]
        .into(),
    );
    app.dispatch(Up);
    assert_eq!(app.platform_cursor(), 0);
    assert_eq!(app.platform_tab_count(), 5);
    app.dispatch(Right); // favourites follows search
    assert_eq!(app.dispatch(Activate), Effect::None);
    assert!(app.favourites_only());
    assert_eq!(app.visible_count(), 3);
    app.dispatch(Right);
    app.dispatch(Right);
    app.dispatch(Activate); // MD + favourites
    assert_eq!(app.visible_count(), 2);
    app.dispatch(Down);
    app.dispatch(Left);
    app.dispatch(Activate); // A + MD + favourites
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().image, "amber-md");
    app.dispatch(Right);
    app.dispatch(Up);
    app.dispatch(Left);
    app.dispatch(Left);
    app.dispatch(Activate); // turn only favourites off, keep A + MD
    assert!(!app.favourites_only());
    assert_eq!(app.platform_filter(), Some("MD"));
    assert_eq!(app.letter_filter(), Some('A'));
    assert_eq!(app.visible_count(), 1);
}

#[test]
fn removing_favourite_refilters_only_after_save_confirmation_and_can_empty_the_list() {
    let mut app = App::new(catalog());
    app.set_favourites([crate::common::hash("amber-md")].into());
    for action in [Up, Right, Activate, Down] {
        app.dispatch(action);
    }
    assert_eq!(app.visible_count(), 1);
    assert!(matches!(
        app.dispatch(Favourite),
        Effect::SaveFavourite { enabled: false, .. }
    ));
    // An unconfirmed/failed write must not remove the row or its star.
    assert_eq!(app.visible_count(), 1);
    app.set_favourites(Default::default());
    assert_eq!(app.visible_count(), 0);
    assert!(app.selected_game().is_none());
    assert!(app.description_lines().is_empty());
    assert_eq!(app.dispatch(Activate), Effect::None);
    for action in [Up, Activate, Down] {
        app.dispatch(action);
    }
    assert_eq!(app.visible_count(), 4);
    assert!(!app.favourites_only());
}

#[test]
fn empty_catalog_still_has_the_favourites_control() {
    let mut app = App::new(vec![]);
    assert_eq!(app.platform_tab_count(), 3);
    for action in [Up, Left, Right, Right, Activate] {
        app.dispatch(action);
    }
    assert_eq!(app.platform_cursor(), 1);
    assert!(app.favourites_only());
    assert_eq!(app.visible_count(), 0);
    app.dispatch(Activate);
    assert!(!app.favourites_only());
}

#[test]
fn digits_and_cyrillic_have_predictable_initials() {
    assert_eq!(crate::common::game("a", "  1942", "MD").initial(), '#');
    assert_eq!(crate::common::game("b", "ёжик", "MD").initial(), 'Ё');
    assert_eq!(crate::common::game("c", "башня", "MD").initial(), 'Б');
    assert_eq!(crate::common::game("d", "", "MD").initial(), '#');
}

#[test]
fn long_input_walk_never_loses_invariants() {
    let mut app = App::new(catalog());
    let mut seed = 7u32;
    for _ in 0..20_000 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        app.dispatch(
            [
                Up,
                Down,
                Left,
                Right,
                Activate,
                Filters,
                LowerFilters,
                Favourite,
                Trash,
            ][seed as usize % 9],
        );
        assert_eq!(app.selected_game().is_none(), app.visible_count() == 0);
        for i in 0..app.visible_count() {
            let game = app.visible_game(i).unwrap();
            assert!(app.letter_filter().is_none_or(|c| game.initial() == c));
            assert!(app.platform_filter().is_none_or(|p| game.platform == p));
        }
    }
}

#[test]
fn header_focuses_only_exit_and_only_x_requests_exit() {
    for empty in [false, true] {
        for language in ["en", "ru"] {
            let mut app = App::new(if empty { vec![] } else { catalog() });
            app.set_locales(crate::common::locales(language));
            let before = app.session_state();
            app.dispatch(Right);
            app.dispatch(Up);
            assert!(app.exit_focused());
            for action in [Left, Right, Favourite, Trash] {
                assert_eq!(app.dispatch(action), Effect::None);
                assert!(app.exit_focused());
            }
            assert_eq!(app.dispatch(Activate), Effect::Exit);
            assert_eq!(app.session_state(), before);
            assert_eq!(app.locales().language(), language);
            for action in [Up, Down] {
                app.dispatch(action);
                assert_eq!(app.focus(), Focus::Details);
                app.dispatch(Up);
                assert!(app.exit_focused());
            }
        }
    }
}
