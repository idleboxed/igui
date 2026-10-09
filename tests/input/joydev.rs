// SPDX-License-Identifier: BSD-3-Clause
use super::held;
use igui::input::{joydev::{Mapping, State}, NavigationInput};
use imanlib::{igui::Action::*, input::ACTIONS, input_metrics::{LogicalEdgeSampler, LOGICAL_EDGE_COUNT}};
use std::time::Duration;

#[cfg(target_os = "linux")]
#[test]
fn discovery_is_sorted_read_only_and_selects_only_numbered_joydev_names() {
    let directory = crate::common::Scratch::new();
    let names = ["js2", "js10", "event0", "js", "js-1", "js2x"];
    for name in names {
        std::fs::write(directory.root().join(name), b"synthetic").unwrap();
    }

    let paths = igui::input::joydev::discover(directory.root()).unwrap();

    assert_eq!(paths, [directory.root().join("js10"), directory.root().join("js2")]);
    assert_eq!(std::fs::read_dir(directory.root()).unwrap().count(), names.len());
    for name in names {
        assert_eq!(std::fs::read(directory.root().join(name)).unwrap(), b"synthetic");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn discovery_reports_missing_and_non_directory_sources_instead_of_an_empty_list() {
    let directory = crate::common::Scratch::new();
    let file = directory.root().join("not-a-directory");
    std::fs::write(&file, b"synthetic").unwrap();

    for (path, kind) in [
        (directory.root().join("missing"), std::io::ErrorKind::NotFound),
        (file, std::io::ErrorKind::NotADirectory),
    ] {
        let error = igui::input::joydev::discover(&path).unwrap_err();

        assert_eq!(error.kind(), kind);
    }
}

fn event(kind: u8, number: u8, value: i16) -> [u8; 8] {
    let mut event = [0; 8];
    event[4..6].copy_from_slice(&value.to_ne_bytes());
    event[6] = kind;
    event[7] = number;
    event
}

fn pad() -> State {
    State::new(Mapping {
        x: 2,
        b: 1,
        y: 3,
        a: 0,
        select: Some(8),
        start: Some(9),
        axis_x: Some(4),
        axis_y: Some(5),
        dpad: [Some(10), Some(11), Some(12), Some(13)],
    })
}

#[test]
fn joystick_init_and_overflow_do_not_activate_held_buttons() {
    let mut pad = pad();
    pad.event(event(0x81, 1, 1)); // B held at open
    assert!(!pad.sample().any());
    pad.event(event(1, 1, 0));
    assert!(!pad.sample().any());
    pad.event(event(1, 1, 1));
    assert!(pad.sample().down(Favourite));
    pad.event(event(0x81, 1, 1)); // synthetic resync after overflow
    assert!(!pad.sample().any());
    pad.event(event(1, 1, 0));
    pad.sample();
    pad.event(event(1, 2, 1));
    assert!(pad.sample().down(LowerFilters));
}

#[test]
fn joystick_hat_deadzone_buttons_and_ignored_controls_are_mapped() {
    let mut pad = pad();
    pad.sample();
    pad.event(event(2, 4, -32767));
    assert!(pad.sample().down(Left));
    pad.event(event(2, 4, -15000));
    assert!(!pad.sample().any());
    pad.event(event(2, 5, 32767));
    assert!(pad.sample().down(Down));
    pad.event(event(2, 5, 0));
    pad.event(event(1, 10, 1));
    assert!(pad.sample().down(Up));
    pad.event(event(1, 10, 0));
    pad.event(event(1, 9, 1));
    assert_eq!(pad.sample(), held(Start));
    pad.event(event(1, 9, 0));
    assert!(!pad.sample().any());
    pad.event(event(2, 0, 32767)); // other axis ignored when hat selected
    assert!(!pad.sample().any());
}

#[test]
fn short_gamepad_tap_between_frames_is_not_lost_or_repeated() {
    let mut pad = pad();
    pad.sample();
    for (button, action) in [
        (2, LowerFilters),
        (1, Favourite),
        (3, Filters),
        (0, Activate),
        (8, Trash),
        (9, Start),
    ] {
        pad.event(event(1, button, 1));
        pad.event(event(1, button, 0));
        assert!(pad.sample().down(action));
        assert!(!pad.sample().any());
    }
}

#[test]
fn sampled_gamepad_diagnostics_keep_all_action_positions_including_start() {
    let mut pad = pad();
    pad.sample();
    let mut sampler = LogicalEdgeSampler::default();
    let mapped = [
        (10, Up), (11, Down), (12, Left), (13, Right),
        (3, Filters), (2, LowerFilters), (0, Activate), (1, Favourite),
        (8, Trash), (9, Start),
    ];
    assert_eq!(mapped.map(|(_, action)| action), ACTIONS);

    for (index, (button, action)) in mapped.into_iter().enumerate() {
        pad.event(event(1, button, 1));
        for _ in 0..3 {
            let buttons = pad.sample();
            assert_eq!(buttons, held(action));
            sampler.record(ACTIONS.map(|action| buttons.down(action)));
        }
        pad.event(event(1, button, 0));
        let buttons = pad.sample();
        sampler.record(ACTIONS.map(|action| buttons.down(action)));

        let mut expected = [0; LOGICAL_EDGE_COUNT];
        expected[..=index].fill(1);
        assert_eq!(sampler.snapshot(), expected);
    }
}

#[test]
fn sampled_start_diagnostics_and_navigation_both_require_release_for_another_edge() {
    let mut pad = pad();
    pad.sample();
    let mut sampler = LogicalEdgeSampler::default();
    let mut navigation = NavigationInput::default();

    for (step, (down, expected_count, expected_action)) in [
        (true, 1, vec![Start]),
        (true, 1, vec![]),
        (false, 1, vec![]),
        (true, 2, vec![Start]),
    ].into_iter().enumerate() {
        pad.event(event(1, 9, i16::from(down)));
        let buttons = pad.sample();
        sampler.record(ACTIONS.map(|action| buttons.down(action)));
        let actions = navigation.poll(buttons, Duration::from_secs(step as u64));

        assert_eq!(actions, expected_action);
        let counts = sampler.snapshot();
        assert_eq!(&counts[..9], [0; 9]);
        assert_eq!(counts[9], expected_count);
    }
}

#[test]
fn square_circle_triangle_use_physical_positions_not_linux_letter_aliases() {
    let mapping = Mapping::from_codes(
        &[0, 1, 0x10, 0x11],
        &[0x133, 0x130, 0x134, 0x131],
        None,
        None,
    )
    .unwrap();
    assert_eq!((mapping.x, mapping.b, mapping.y, mapping.a), (2, 3, 0, 1));
    assert_eq!((mapping.axis_x, mapping.axis_y), (Some(2), Some(3)));
    let mut pad = State::new(mapping);
    pad.sample();
    for (button, action) in [
        (2, LowerFilters),
        (3, Favourite),
        (0, Filters),
        (1, Activate),
    ] {
        pad.event(event(1, button, 1));
        assert_eq!(pad.sample(), held(action));
        pad.event(event(1, button, 0));
        assert!(!pad.sample().any());
    }
}

#[test]
fn manual_four_button_mapping_validates_indices_and_preserves_direction_fallback() {
    let codes = [0x120, 0x121, 0x122, 0x123];
    assert!(Mapping::from_codes(&[0, 1], &codes, None, None).is_err());
    let mapping = Mapping::from_codes(&[0, 1], &codes, Some((2, 1, 3, 0)), None).unwrap();
    assert_eq!((mapping.x, mapping.b, mapping.y, mapping.a), (2, 1, 3, 0));
    assert_eq!((mapping.axis_x, mapping.axis_y), (Some(0), Some(1)));
    for indices in [
        (1, 1, 2, 0),
        (1, 2, 1, 0),
        (1, 2, 2, 0),
        (4, 1, 2, 0),
        (1, 4, 2, 0),
        (1, 2, 4, 0),
        (1, 2, 3, 4),
        (1, 2, 3, 1),
        (1, 2, 3, 2),
        (1, 2, 3, 3),
    ] {
        assert!(Mapping::from_codes(&[0, 1], &codes, Some(indices), None).is_err());
    }
    assert!(Mapping::from_codes(&[], &codes, Some((2, 1, 3, 0)), None).is_err());
    let mapping = Mapping::from_codes(
        &[],
        &[0x134, 0x131, 0x133, 0x130, 0x220, 0x221, 0x222, 0x223],
        None,
        None,
    )
    .unwrap();
    assert_eq!(mapping.dpad, [Some(4), Some(5), Some(6), Some(7)]);
}

#[test]
fn y_held_at_connect_is_discarded_until_release() {
    let mut pad = pad();
    pad.event(event(0x81, 3, 1));
    assert!(!pad.sample().any());
    pad.event(event(1, 3, 0));
    assert!(!pad.sample().any());
    pad.event(event(1, 3, 1));
    assert!(pad.sample().down(Filters));
}

#[test]
fn a_held_at_connect_is_discarded_until_release() {
    let mut pad = pad();
    pad.event(event(0x81, 0, 1));
    assert!(!pad.sample().any());
    pad.event(event(1, 0, 0));
    assert!(!pad.sample().any());
    pad.event(event(1, 0, 1));
    assert_eq!(pad.sample(), held(Activate));
}

#[test]
fn select_uses_kernel_code_not_start_and_can_be_overridden_for_nonstandard_pads() {
    let codes = [
        0x134, 0x131, 0x133, 0x130, 0x13b, 0x13a, 0x120, 0x220, 0x221, 0x222, 0x223,
    ];
    let mapping = Mapping::from_codes(&[0, 1], &codes, None, None).unwrap();
    assert_eq!(mapping.select, Some(5));
    assert_eq!(mapping.start, Some(4));
    let mut pad = State::new(mapping);
    pad.sample();
    pad.event(event(1, 4, 1));
    assert_eq!(pad.sample(), held(Start));
    pad.event(event(1, 4, 0));
    assert!(!pad.sample().any());
    pad.event(event(1, 5, 1));
    assert_eq!(pad.sample(), held(Trash));
    pad.event(event(1, 5, 0));
    assert!(!pad.sample().any());
    let mapping = Mapping::from_codes(&[0, 1], &codes, None, Some(6)).unwrap();
    assert_eq!(mapping.select, Some(6));
    let mut pad = State::new(mapping);
    pad.sample();
    pad.event(event(1, 5, 1));
    assert!(!pad.sample().any());
    pad.event(event(1, 6, 1));
    assert_eq!(pad.sample(), held(Trash));
    for index in [0, 1, 2, 3, 4, 7, 8, 9, 10, 11, 255] {
        assert!(Mapping::from_codes(&[0, 1], &codes, None, Some(index)).is_err());
    }
    assert!(Mapping::from_codes(&[0, 1], &codes, Some((5, 1, 2, 3)), None).is_err());
    assert!(Mapping::from_codes(&[0, 1], &codes, Some((4, 1, 2, 3)), None).is_err());
    let mapping = Mapping::from_codes(&[0, 1], &codes[..4], None, None).unwrap();
    assert_eq!(mapping.select, None); // keyboard Delete remains available
    assert_eq!(mapping.start, None);
}

#[test]
fn select_held_at_connect_or_resync_is_discarded_until_release() {
    let mut pad = pad();
    for _ in 0..2 {
        pad.event(event(0x81, 8, 1));
        assert!(!pad.sample().any());
        pad.event(event(1, 8, 0));
        assert!(!pad.sample().any());
        pad.event(event(1, 8, 1));
        assert_eq!(pad.sample(), held(Trash));
    }
}
