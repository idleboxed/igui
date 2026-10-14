// SPDX-License-Identifier: BSD-3-Clause
//! Shipped evdev profiles integrated with the menu'events public navigation policy.
#![cfg(target_os = "linux")]
use igui::input::NavigationInput;
use imanlib::{
    evdev::{linux::{Pad, Source}, DeviceInfo, Event},
    gamepad::receiver::Receiver,
    igui::Action::*,
    input::Buttons,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    io,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

fn make_device_info(keyboard_codes: bool) -> DeviceInfo {
    DeviceInfo {
        path: "/synthetic/event0".into(),
        name: "ShanWan Twin USB Joystick".into(),
        axis_codes: vec![0, 1, 2, 5, 16, 17],
        key_codes: if keyboard_codes {
            vec![21, 28, 30, 45, 48, 54, 103, 105, 106, 108]
        } else {
            (288..300).collect()
        },
        abs_info: BTreeMap::from([
            (0, [128, 0, 255, 0, 0, 0]),
            (1, [128, 0, 255, 0, 0, 0]),
            (16, [0, -1, 1, 0, 0, 0]),
            (17, [0, -1, 1, 0, 0, 0]),
        ]),
        identity: BTreeMap::from([
            ("id/bustype".into(), "0003".into()),
            (
                "id/vendor".into(),
                if keyboard_codes { "20bc" } else { "2563" }.into(),
            ),
            (
                "id/product".into(),
                if keyboard_codes { "5500" } else { "0555" }.into(),
            ),
        ]),
    }
}

struct Fixture {
    info: DeviceInfo,
    events: Rc<RefCell<VecDeque<Event>>>,
}

impl Pad for Fixture {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn read(&mut self) -> io::Result<Option<Event>> {
        Ok(self.events.borrow_mut().pop_front())
    }

    fn keys_down(&mut self) -> io::Result<Vec<u16>> {
        Ok(vec![])
    }

    fn axes_state(&mut self) -> io::Result<BTreeMap<u8, i32>> {
        // These scenarios start neutral and contain no SYN_DROPPED packets.
        Ok(self.info.abs_info.iter().map(|(&code, info)| (code, info[0])).collect())
    }
}

impl Source for Fixture {
    fn discover(&mut self) -> io::Result<Vec<PathBuf>> {
        Ok(vec![PathBuf::from(&self.info.path)])
    }

    fn open(&mut self, _: &Path) -> io::Result<Box<dyn Pad>> {
        Ok(Box::new(Self { info: self.info.clone(), events: self.events.clone() }))
    }
}

fn create_fixture(keyboard_codes: bool) -> (Rc<RefCell<VecDeque<Event>>>, Receiver<Fixture>) {
    create_device_fixture(make_device_info(keyboard_codes))
}

fn create_device_fixture(info: DeviceInfo) -> (Rc<RefCell<VecDeque<Event>>>, Receiver<Fixture>) {
    let events = Rc::new(RefCell::new(VecDeque::new()));
    let source = Fixture { info, events: events.clone() };
    (events, Receiver::with_profiles(source, crate::common::evdev_profiles()))
}

// Capabilities and opening values come from stick reports read on 2026-10-14.
// The packets below are synthetic, not a replay or a hardware verification.
fn make_standard_device_info(wireless: bool) -> DeviceInfo {
    let (name, vendor, product, axis_codes, key_codes, abs_info) = if wireless {
        (
            "ShanWan Wireless Gamepad", "0079", "181c",
            vec![0, 1, 2, 5, 9, 10, 16, 17],
            (304..=319).collect(),
            BTreeMap::from([
                (0, [127, 0, 255, 0, 15, 0]),
                (1, [128, 0, 255, 0, 15, 0]),
                (2, [127, 0, 255, 0, 15, 0]),
                (5, [128, 0, 255, 0, 15, 0]),
                (9, [0, 0, 255, 0, 15, 0]),
                (10, [0, 0, 255, 0, 15, 0]),
                (16, [0, -1, 1, 0, 0, 0]),
                (17, [0, -1, 1, 0, 0, 0]),
            ]),
        )
    } else {
        (
            "Microsoft X-Box 360 pad", "045e", "028e",
            vec![0, 1, 2, 3, 4, 5, 16, 17],
            vec![304, 305, 307, 308, 310, 311, 314, 315, 316, 317, 318],
            BTreeMap::from([
                (0, [-1536, -32768, 32767, 16, 128, 0]),
                (1, [1535, -32768, 32767, 16, 128, 0]),
                (2, [0, 0, 255, 0, 0, 0]),
                (3, [0, -32768, 32767, 16, 128, 0]),
                (4, [-1, -32768, 32767, 16, 128, 0]),
                (5, [0, 0, 255, 0, 0, 0]),
                (16, [0, -1, 1, 0, 0, 0]),
                (17, [0, -1, 1, 0, 0, 0]),
            ]),
        )
    };
    DeviceInfo {
        path: "/synthetic/event0".into(),
        name: name.into(),
        axis_codes,
        key_codes,
        abs_info,
        identity: BTreeMap::from([
            ("id/bustype".into(), "0003".into()),
            ("id/vendor".into(), vendor.into()),
            ("id/product".into(), product.into()),
        ]),
    }
}

fn poll(receiver: &mut Receiver<Fixture>) -> Buttons {
    receiver.poll(Duration::from_secs(1))
}

fn arm_receiver(events: &Rc<RefCell<VecDeque<Event>>>, receiver: &mut Receiver<Fixture>, code: u16) {
    assert!(!poll(receiver).any());
    assert!(!poll(receiver).any());
    queue_tap(events, code);
    assert!(!poll(receiver).any());
    assert!(!poll(receiver).any());
    assert!(receiver.diagnostic_identity().is_some());
}

fn make_event(kind: u16, code: u16, value: i32) -> Event {
    Event { kind, code, value }
}

fn queue_packet(events: &Rc<RefCell<VecDeque<Event>>>, event: Event) {
    events.borrow_mut().extend([event, make_event(0, 0, 0)]);
}

fn queue_tap(events: &Rc<RefCell<VecDeque<Event>>>, code: u16) {
    queue_packet(events, make_event(1, code, 1));
    queue_packet(events, make_event(1, code, 0));
}

#[test]
fn evdev_profiles_map_positions_select_and_start() {
    for (keyboard_codes, codes) in [
        (false, [291, 289, 288, 290, 296, 297]),
        (true, [45, 48, 21, 30, 54, 28]),
    ] {
        let (events, mut receiver) = create_fixture(keyboard_codes);
        arm_receiver(&events, &mut receiver, if keyboard_codes { 21 } else { 288 });
        for (code, action) in
            codes
                .into_iter()
                .zip([LowerFilters, Favourite, Filters, Activate, Trash, Start])
        {
            queue_tap(&events, code);
            let mut expected = Buttons::default();
            expected.set(action, true);
            assert_eq!(poll(&mut receiver), expected);
            assert!(!poll(&mut receiver).any());
        }
        for code in [292, 293, 294, 295, 298, 299] {
            queue_tap(&events, code);
        }
        queue_packet(&events, make_event(3, 2, 0));
        queue_packet(&events, make_event(3, 5, 255));
        assert!(!poll(&mut receiver).any());
    }
}

#[test]
fn digital_stick_analog_hat_and_keyboard_dpad_all_navigate() {
    for keyboard_codes in [false, true] {
        let (events, mut receiver) = create_fixture(keyboard_codes);
        arm_receiver(&events, &mut receiver, if keyboard_codes { 21 } else { 288 });
        for (code, value, neutral, action) in [
            (0, 0, 128, Left),
            (0, 255, 128, Right),
            (1, 0, 128, Up),
            (1, 255, 128, Down),
            (16, -1, 0, Left),
            (16, 1, 0, Right),
            (17, -1, 0, Up),
            (17, 1, 0, Down),
        ] {
            queue_packet(&events, make_event(3, code, value));
            assert!(poll(&mut receiver).down(action));
            queue_packet(&events, make_event(3, code, neutral));
            assert!(!poll(&mut receiver).any());
        }
        if keyboard_codes {
            for (code, action) in [(103, Up), (108, Down), (105, Left), (106, Right)] {
                queue_tap(&events, code);
                assert!(poll(&mut receiver).down(action));
            }
        }
    }
}

#[test]
fn recorded_pc_dpad_and_faces_replay_as_two_correct_sequences() {
    let (events, mut receiver) = create_fixture(false);
    arm_receiver(&events, &mut receiver, 288);
    let mut navigation = NavigationInput::default();
    let mut actions = Vec::new();
    for line in include_str!("../_fixtures/shanwan-pc-faces.jsonl").lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let event = make_event(
            row["type"].as_u64().unwrap() as u16,
            row["code"].as_u64().unwrap() as u16,
            row["value"].as_i64().unwrap() as i32,
        );
        events.borrow_mut().push_back(event);
        if event.kind == 0 && event.code == 0 {
            actions.extend(navigation.poll(
                poll(&mut receiver),
                Duration::from_millis(row["elapsed_ms"].as_u64().unwrap()),
            ));
        }
    }
    assert_eq!(
        actions,
        [
            Up, Down, Left, Right, LowerFilters, Filters, Favourite, Activate, Trash,
            // The measured second sequence includes Start confirmations.
            Up, Start, Down, Start, Left, Start, Right, Start, LowerFilters, Start,
            Filters, Start, Favourite, Start, Activate, Start, Trash, Start,
        ]
    );
}

#[test]
fn kernel_repetitions_do_not_repeat_menu_actions() {
    let (events, mut receiver) = create_fixture(true);
    arm_receiver(&events, &mut receiver, 21);
    let mut navigation = NavigationInput::default();

    queue_packet(&events, make_event(1, 48, 1));
    assert_eq!(navigation.poll(poll(&mut receiver), Duration::ZERO), [Favourite]);
    queue_packet(&events, make_event(1, 48, 2));
    assert!(navigation.poll(poll(&mut receiver), Duration::from_secs(1)).is_empty());
}

#[test]
fn xbox_and_wireless_shanwan_profiles_map_measured_face_buttons() {
    for wireless in [false, true] {
        let (events, mut receiver) = create_device_fixture(make_standard_device_info(wireless));
        arm_receiver(&events, &mut receiver, 304);

        for (code, action) in [
            (307, LowerFilters), (305, Favourite), (308, Filters),
            (304, Activate), (314, Trash), (315, Start),
        ] {
            queue_tap(&events, code);
            assert_eq!(poll(&mut receiver), super::held(action));
            assert!(!poll(&mut receiver).any());
        }
        for code in [310, 311, 312, 313, 316, 317, 318] {
            queue_tap(&events, code);
        }
        assert!(!poll(&mut receiver).any());
    }
}

#[test]
fn xbox_and_wireless_shanwan_profiles_navigate_using_their_measured_axis_ranges() {
    for wireless in [false, true] {
        let info = make_standard_device_info(wireless);
        let ranges = info.abs_info.clone();
        let (events, mut receiver) = create_device_fixture(info);
        arm_receiver(&events, &mut receiver, 304);

        for (code, range_position, action) in [(0, 1, Left), (0, 2, Right), (1, 1, Up), (1, 2, Down)] {
            queue_packet(&events, make_event(3, code, ranges[&(code as u8)][range_position]));
            assert_eq!(poll(&mut receiver), super::held(action));
            queue_packet(&events, make_event(3, code, ranges[&(code as u8)][0]));
            assert!(!poll(&mut receiver).any());
        }
        for (code, value, action) in [(16, -1, Left), (16, 1, Right), (17, -1, Up), (17, 1, Down)] {
            queue_packet(&events, make_event(3, code, value));
            // No hat events were measured for 0079:181c: its profile must not claim D-pad support.
            assert_eq!(poll(&mut receiver), if wireless { Buttons::default() } else { super::held(action) });
            queue_packet(&events, make_event(3, code, 0));
            assert!(!poll(&mut receiver).any());
        }
        for code in if wireless { [2, 5, 9, 10] } else { [2, 3, 4, 5] } {
            queue_packet(&events, make_event(3, code, ranges[&(code as u8)][2]));
        }
        assert!(!poll(&mut receiver).any());

        if !wireless {
            for (code, value) in [(0, -2304), (1, 3072), (0, -1536), (1, 1535)] {
                queue_packet(&events, make_event(3, code, value));
            }
            assert!(!poll(&mut receiver).any());
        }
    }
}
