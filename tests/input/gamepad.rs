// SPDX-License-Identifier: BSD-3-Clause
#![cfg(target_os = "linux")]
use igui::input::gamepad::Gamepad;
use imanlib::input_metrics::LOGICAL_EDGE_COUNT;
use std::time::Duration;

#[test]
fn automatic_input_without_profiles_never_falls_back_to_unprofiled_joydev() {
    let mut gamepad = Gamepad::with_profiles(None, None, None, vec![], vec![]);
    gamepad.enable_diagnostics();

    for time in [Duration::ZERO, Duration::from_secs(3)] { assert!(!gamepad.poll(time).any()); }

    let diagnostics = gamepad.diagnostics().unwrap();
    assert_eq!(diagnostics.joydev.connections, 0);
    assert_eq!(diagnostics.joydev.errors, 0);
    assert!(!diagnostics.joydev_connected);
    assert_eq!(diagnostics.polls, 2);
    assert_eq!(diagnostics.logical_edges, [0; LOGICAL_EDGE_COUNT]);
}

#[test]
fn explicit_joydev_open_errors_are_counted_without_input_and_scans_are_rate_limited() {
    let directory = crate::common::Scratch::new();
    let mut gamepad = Gamepad::with_profiles(
        Some(directory.root().join("missing-js0")), None, None, vec![], vec![],
    );
    gamepad.enable_diagnostics();

    for (seconds, expected_errors) in [(0, 1), (1, 1), (2, 2), (3, 2)] {
        let buttons = gamepad.poll(Duration::from_secs(seconds));
        let diagnostics = gamepad.diagnostics().unwrap();

        assert!(!buttons.any());
        assert_eq!(diagnostics.joydev.errors, expected_errors);
        assert_eq!(diagnostics.joydev.connections, 0);
        assert!(!diagnostics.joydev_connected);
        assert_eq!(diagnostics.logical_edges, [0; LOGICAL_EDGE_COUNT]);
    }
    assert_eq!(std::fs::read_dir(directory.root()).unwrap().count(), 0);
}
