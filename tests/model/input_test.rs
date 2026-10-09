// SPDX-License-Identifier: BSD-3-Clause
//! Startup invitation and modal handoff only; the test itself belongs to iman.
use {
    imanlib::input::Buttons,
    igui::input::NavigationInput,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Effect,
    igui::render::Renderer,
    igui::render::RetainedFrame,
    igui::render::HEIGHT,
    igui::render::WIDTH,
    imanlib::surface::Surface,
};
use std::time::Duration;

#[test]
fn diagnostics_is_opt_in_and_blocks_every_catalogue_action_without_changing_header() {
    let mut app = App::new(vec![]);
    app.dispatch(Right); app.dispatch(Up); app.dispatch(Left);
    assert_eq!(app.dispatch(Activate), Effect::Exit);
    app.start_input_test();
    assert!(!app.input_test_active());

    app.enable_input_test(true);
    app.start_input_test();
    assert!(app.input_test_active());
    let state = app.session_state();
    for action in [Activate, Favourite, Trash, Filters, LowerFilters, Up, Down, Left, Right] {
        assert_eq!(app.dispatch(action), Effect::None);
        assert!(app.input_test_active());
        assert_eq!(app.session_state(), state);
    }
    app.close_input_test();
    app.dispatch(Right);
    assert!(app.exit_focused());
    assert_eq!(app.dispatch(Activate), Effect::Exit);
}

#[test]
fn held_activation_does_not_reopen_after_navigation_is_suspended() {
    let mut input = NavigationInput::default();
    let mut buttons = Buttons::default();
    buttons.set(Activate, true);
    input.suspend(buttons);
    assert!(input.poll(buttons, Duration::from_secs(8)).is_empty());
    assert!(input.poll(Buttons::default(), Duration::from_secs(9)).is_empty());
    assert_eq!(input.poll(buttons, Duration::from_secs(10)), vec![Activate]);
}

#[test]
fn diagnostic_capability_does_not_change_header_rendering() {
    let mut app = App::new(vec![]);
    let mut retained = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), Renderer::default());
    retained.update(&app, None, None).unwrap();
    app.enable_input_test(true);
    for action in [Right, Up, Left, Right] {
        app.dispatch(action);
        retained.update(&app, None, None).unwrap();
        let mut full = Surface::new(WIDTH, HEIGHT);
        Renderer::default().draw(&app, None, &mut full).unwrap();
        assert_eq!(full.pixels(), retained.surface().pixels());
    }
}
