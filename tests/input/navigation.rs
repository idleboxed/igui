// SPDX-License-Identifier: BSD-3-Clause
use super::held;
use igui::input::NavigationInput;
use imanlib::{igui::Action::*, input::Buttons};
use std::time::Duration;

#[test]
fn start_is_edge_triggered_and_held_start_is_discarded_after_suspension() {
    let mut input = NavigationInput::default();
    assert_eq!(input.poll(held(Start), Duration::ZERO), [Start]);
    assert!(input.poll(held(Start), Duration::from_secs(10)).is_empty());
    input.suspend(held(Start));
    assert!(input.poll(held(Start), Duration::from_secs(20)).is_empty());
    input.poll(Buttons::default(), Duration::from_secs(21));
    assert_eq!(input.poll(held(Start), Duration::from_secs(22)), [Start]);
}

#[test]
fn dpad_repeats_but_needs_release_after_changing_region() {
    let mut input = NavigationInput::default();
    assert_eq!(input.poll(held(Down), Duration::ZERO), [Down]);
    assert!(input
        .poll(held(Down), Duration::from_millis(349))
        .is_empty());
    assert_eq!(input.poll(held(Down), Duration::from_millis(350)), [Down]);
    assert_eq!(input.poll(held(Down), Duration::from_millis(450)), [Down]);
    input.block_until_release(Down);
    assert!(input.poll(held(Down), Duration::from_secs(2)).is_empty());
    input.poll(Buttons::default(), Duration::from_secs(3));
    assert_eq!(input.poll(held(Down), Duration::from_secs(4)), [Down]);
}

#[test]
fn modal_transition_discards_held_buttons_until_a_fresh_press() {
    for answer in [Activate, Favourite] {
        let mut input = NavigationInput::default();
        assert_eq!(input.poll(held(Trash), Duration::ZERO), [Trash]);
        input.suspend(held(Trash));
        assert!(input.poll(held(Trash), Duration::from_secs(1)).is_empty());
        input.poll(Buttons::default(), Duration::from_secs(2));

        assert_eq!(input.poll(held(answer), Duration::from_secs(3)), [answer]);
        let buttons = held(answer).union(held(Trash)).union(held(Down));
        input.suspend(buttons);

        assert!(input.poll(buttons, Duration::from_secs(4)).is_empty());
        assert!(input.poll(buttons, Duration::from_secs(6)).is_empty());
        input.poll(Buttons::default(), Duration::from_secs(7));
        assert_eq!(input.poll(held(answer), Duration::from_secs(8)), [answer]);
    }
}

#[test]
fn x_b_y_a_select_are_edges_and_keyboard_gamepad_union_does_not_duplicate_actions() {
    let mut input = NavigationInput::default();
    let both = held(Activate)
        .union(held(Activate))
        .union(held(Favourite))
        .union(held(Filters))
        .union(held(LowerFilters))
        .union(held(Trash));
    assert_eq!(
        input.poll(both, Duration::ZERO),
        [Filters, LowerFilters, Activate, Favourite, Trash]
    );
    assert!(input.poll(both, Duration::from_secs(10)).is_empty());
    input.poll(Buttons::default(), Duration::from_secs(11));
    assert_eq!(
        input.poll(held(Favourite), Duration::from_secs(12)),
        [Favourite]
    );
}

#[test]
fn loading_and_unfocused_inputs_cannot_be_replayed_when_ready() {
    let mut input = NavigationInput::default();
    let buttons = held(Activate)
        .union(held(Up))
        .union(held(Favourite))
        .union(held(Filters))
        .union(held(LowerFilters))
        .union(held(Trash));
    input.suspend(buttons);
    assert!(input.poll(buttons, Duration::from_secs(1)).is_empty());
    input.poll(Buttons::default(), Duration::from_secs(2));
    assert_eq!(
        input.poll(held(Activate), Duration::from_secs(3)),
        [Activate]
    );
}
