
use {
    igui::model::description,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Focus,
};

fn app() -> App {
    let mut first = crate::common::game("a.nes", "A", "NES");
    first.description = Some(
        (0..8)
            .map(|i| format!("Description line {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    App::new(vec![first, crate::common::game("b.nes", "B", "NES")])
}

#[test]
fn wrapping_preserves_paragraphs_and_never_splits_utf8_bytes() {
    assert_eq!(
        description::wrap("one two\n\nthree", 7),
        ["one two", "", "three"]
    );
    assert_eq!(
        description::wrap("ёжикёжик next", 3),
        ["ёжи", "кёж", "ик", "nex", "t"]
    );
    assert!(description::wrap(" \n ", 5).is_empty());
    assert!(description::wrap("word", 0).is_empty());
    for line in description::wrap("12 verylongword 34 567 eight", 4) {
        assert!(line.chars().count() <= 4);
    }
}

#[test]
fn description_scrolls_to_both_limits_before_entering_header() {
    let mut app = app();
    app.dispatch(Right);
    assert_eq!(app.focus(), Focus::Details);
    assert_eq!(app.description_offset(), 0);
    for offset in 1..=app.description_max_offset() {
        app.dispatch(Down);
        assert_eq!(app.focus(), Focus::Details);
        assert_eq!(app.description_offset(), offset);
    }
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::Header);
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::Details);
    assert_eq!(app.description_offset(), app.description_max_offset());
    for offset in (0..app.description_max_offset()).rev() {
        app.dispatch(Up);
        assert_eq!(app.focus(), Focus::Details);
        assert_eq!(app.description_offset(), offset);
    }
    app.dispatch(Up);
    assert_eq!(app.focus(), Focus::Header);
}

#[test]
fn short_and_empty_descriptions_go_directly_to_header_and_left_keeps_game() {
    let mut app = app();
    app.dispatch(Down);
    app.dispatch(Right);
    assert!(app.description_lines().is_empty());
    app.dispatch(Down);
    assert_eq!(app.focus(), Focus::Header);
    app.dispatch(Down);
    app.dispatch(Left);
    assert_eq!(app.focus(), Focus::Games);
    assert_eq!(app.selected_game().unwrap().image, "b.nes");
    let mut empty = App::new(vec![]);
    empty.dispatch(Right);
    empty.dispatch(Up);
    assert_eq!(empty.focus(), Focus::Header);
}

#[test]
fn scroll_position_survives_region_changes_but_resets_for_another_game() {
    let mut app = app();
    app.dispatch(Right);
    app.dispatch(Down);
    app.dispatch(Left);
    app.dispatch(Right);
    assert_eq!(app.description_offset(), 1);
    app.dispatch(Left);
    app.dispatch(Down);
    assert_eq!(app.description_offset(), 0);
    app.dispatch(Up);
    assert_eq!(app.description_offset(), 0);
}
