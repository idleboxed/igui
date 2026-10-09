// SPDX-License-Identifier: BSD-3-Clause
//! Synthetic evdev streams through public navigation, model and metric APIs.
//! Only model effects are observed; no process launch or preference I/O.
#![cfg(target_os = "linux")]

use {
    imanlib::evdev::linux::Pad,
    imanlib::evdev::linux::Source,
    imanlib::gamepad::profile::Profile,
    imanlib::gamepad::receiver::Receiver,
    imanlib::input::Buttons,
    igui::input::NavigationInput,
    imanlib::input::ACTIONS,
    imanlib::igui::Action,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Effect,
};
use imanlib::{evdev::{DeviceInfo, Event}, input_metrics::LogicalEdgeSampler};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    io,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

#[derive(Default)]
struct Stream {
    events: VecDeque<Event>,
    held: BTreeSet<u16>,
}

struct SyntheticPad {
    info: DeviceInfo,
    stream: Rc<RefCell<Stream>>,
}
impl Pad for SyntheticPad {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn read(&mut self) -> io::Result<Option<Event>> {
        let mut stream = self.stream.borrow_mut();
        let event = stream.events.pop_front();
        if let Some(event) = event {
            if event.kind == 1 {
                if event.value == 0 {
                    stream.held.remove(&event.code);
                } else {
                    stream.held.insert(event.code);
                }
            }
        }
        Ok(event)
    }

    fn keys_down(&mut self) -> io::Result<Vec<u16>> {
        Ok(self.stream.borrow().held.iter().copied().collect())
    }

    fn axes_state(&mut self) -> io::Result<BTreeMap<u8, i32>> {
        Err(io::Error::other("digital-only synthetic pad has no axes"))
    }
}

struct SyntheticSource {
    streams: [Rc<RefCell<Stream>>; 2],
}
impl Source for SyntheticSource {
    fn discover(&mut self) -> io::Result<Vec<PathBuf>> {
        Ok(vec!["/synthetic/event0".into(), "/synthetic/event1".into()])
    }

    fn open(&mut self, path: &Path) -> io::Result<Box<dyn Pad>> {
        let index = match path.to_str() {
            Some("/synthetic/event0") => 0,
            Some("/synthetic/event1") => 1,
            _ => return Err(io::Error::other("unknown synthetic input path")),
        };
        Ok(Box::new(SyntheticPad {
            info: DeviceInfo {
                path: path.to_string_lossy().into_owned(),
                name: "Handoff integration pad".into(),
                key_codes: vec![21, 28, 30, 45, 48, 54, 103, 105, 106, 108],
                axis_codes: vec![],
                abs_info: BTreeMap::new(),
                identity: BTreeMap::from([
                    ("id/bustype".into(), "0003".into()),
                    ("id/vendor".into(), "1234".into()),
                    ("id/product".into(), "5678".into()),
                ]),
            },
            stream: self.streams[index].clone(),
        }))
    }
}

struct Tick {
    buttons: Buttons,
    actions: Vec<Action>,
    effects: Vec<Effect>,
}

struct Menu {
    streams: [Rc<RefCell<Stream>>; 2],
    receiver: Receiver<SyntheticSource>,
    navigation: NavigationInput,
    edges: LogicalEdgeSampler,
    app: App,
}
impl Menu {
    fn new() -> Self {
        let streams = std::array::from_fn(|_| Rc::new(RefCell::new(Stream::default())));
        let profile = Profile::parse(br#"{
            "name":"Handoff integration pad", "backend":"evdev", "access":"read-only",
            "match":{"bus":"usb","vendor":"1234","product":"5678"},
            "buttons":{"west":45,"east":48,"north":21,"south":30,"back":54,"start":28,
                "up":103,"down":108,"left":105,"right":106}, "axes":{}
        }"#).unwrap();
        Self {
            receiver: Receiver::with_profiles(SyntheticSource { streams: streams.clone() }, vec![profile]),
            streams,
            navigation: NavigationInput::default(),
            edges: LogicalEdgeSampler::default(),
            app: App::new((0..6).map(|index| {
                crate::common::game(&format!("game{index}"), &format!("Game {index}"), "NES")
            }).collect()),
        }
    }

    fn key(&mut self, pad: usize, code: u16, value: i32) {
        self.streams[pad].borrow_mut().events.extend([
            Event { kind: 1, code, value },
            Event { kind: 0, code: 0, value: 0 },
        ]);
    }

    fn tick(&mut self, ms: u64) -> Tick {
        let now = Duration::from_millis(ms);
        let buttons = self.receiver.poll(now);
        self.edges.record(ACTIONS.map(|action| buttons.down(action)));
        let actions = self.navigation.poll(buttons, now);
        let effects = actions.iter().map(|&action| self.app.dispatch(action)).collect();
        Tick { buttons, actions, effects }
    }

    fn acquire_first(&mut self) {
        assert!(self.tick(0).actions.is_empty());
        assert!(self.tick(1).actions.is_empty());
        self.key(0, 21, 1);
        self.key(0, 21, 0);
        assert!(self.tick(2).actions.is_empty());
        assert!(self.tick(3).actions.is_empty());
        assert_eq!(self.receiver.diagnostic_identity().unwrap().path, "/synthetic/event0");
    }
}

#[test]
fn handoff_stops_the_old_repeat_and_new_direction_gets_a_full_repeat_delay() {
    let mut menu = Menu::new();
    menu.acquire_first();
    menu.key(0, 108, 1);
    assert_eq!(menu.tick(100).actions, [Down]);
    assert_eq!(menu.tick(450).actions, [Down]);
    assert_eq!(menu.app.selected_index(), Some(2));

    menu.key(1, 21, 1);
    assert!(menu.tick(460).actions.is_empty());
    assert!(menu.tick(470).actions.is_empty());
    menu.key(1, 21, 0);
    assert!(menu.tick(480).actions.is_empty());
    menu.key(1, 108, 1);

    assert_eq!(menu.tick(490).actions, [Down]);
    assert!(menu.tick(600).actions.is_empty());
    assert!(menu.tick(839).actions.is_empty());
    assert_eq!(menu.tick(840).actions, [Down]);
    assert_eq!(menu.app.selected_index(), Some(4));
    assert_eq!(menu.receiver.diagnostic_identity().unwrap().path, "/synthetic/event1");
    assert_eq!(menu.edges.snapshot()[1], 2);
}

#[test]
fn same_start_or_a_on_a_new_pad_is_consumed_before_one_fresh_launch_and_metric_edge() {
    for (code, action, metric) in [(28, Start, 9), (30, Activate, 6)] {
        let mut menu = Menu::new();
        menu.acquire_first();
        menu.key(0, code, 1);
        let original = menu.tick(100);
        assert_eq!(original.actions, [action]);
        assert!(matches!(original.effects.as_slice(), [Effect::Launch { .. }]));

        menu.key(1, code, 1);
        for time in [110, 120] {
            let selecting = menu.tick(time);
            assert!(!selecting.buttons.any());
            assert!(selecting.actions.is_empty() && selecting.effects.is_empty());
            assert_eq!(menu.edges.snapshot()[metric], 1);
        }
        menu.key(1, code, 0);
        assert!(menu.tick(130).actions.is_empty());
        menu.key(1, code, 1);
        let fresh = menu.tick(140);

        assert_eq!(fresh.actions, [action]);
        assert!(matches!(fresh.effects.as_slice(), [Effect::Launch { .. }]));
        assert!(menu.tick(150).actions.is_empty());
        assert_eq!(menu.receiver.diagnostic_identity().unwrap().path, "/synthetic/event1");
        let edges = menu.edges.snapshot();
        for (index, &count) in edges.iter().enumerate() {
            assert_eq!(count, if index == metric { 2 } else { 0 });
        }
    }
}

#[test]
fn handoff_a_cannot_confirm_the_trash_modal_until_release_and_a_new_press() {
    let mut menu = Menu::new();
    menu.acquire_first();
    let hash = menu.app.selected_game().unwrap().hash.clone();
    menu.key(0, 54, 1);
    let opening = menu.tick(100);
    assert_eq!(opening.actions, [Trash]);
    assert_eq!(opening.effects, [Effect::None]);
    assert!(menu.app.trash_confirmation());
    // main.rs suspends navigation when a modal opens, until sampled release.
    menu.navigation.suspend(opening.buttons);

    menu.key(1, 30, 1);
    for time in [110, 120] {
        let selecting = menu.tick(time);
        assert!(selecting.actions.is_empty() && selecting.effects.is_empty());
        assert!(menu.app.trash_confirmation());
    }
    menu.key(1, 30, 0);
    assert!(menu.tick(130).actions.is_empty());
    menu.key(1, 30, 1);
    let confirming = menu.tick(140);

    assert_eq!(confirming.actions, [Activate]);
    assert_eq!(confirming.effects, [Effect::SaveTrash { hash, enabled: true }]);
    assert!(!menu.app.trash_confirmation());
    assert!(menu.tick(150).actions.is_empty());
    let edges = menu.edges.snapshot();
    assert_eq!(edges[6], 1);
    assert_eq!(edges[8], 1);
    assert_eq!(edges[9], 0);
}
