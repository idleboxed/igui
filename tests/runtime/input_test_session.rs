// SPDX-License-Identifier: BSD-3-Clause
//! Real headless IGUI with a synthetic host: the startup timer uses real monotonic time.

use imanlib::{client::Client, igui::{Bootstrap, GuiState}, protocol::{Message, Response}, server::Server};
use serde_json::json;
use std::{fs, process::{Command, Stdio}, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::{Duration, Instant}};

use crate::common::{ChildGuard, HostGuard};

#[test]
fn automatic_input_test_hands_off_and_exits_without_running_in_gui() {
    managed_startup(false);
}

#[test]
fn b_cancels_countdown_without_favourite_write_or_starting_input_test() {
    managed_startup(true);
}

fn managed_startup(cancel_countdown: bool) {
    let directory = tempfile::tempdir_in("/dev/shm").unwrap();
    fs::create_dir(directory.path().join("games")).unwrap();
    fs::write(directory.path().join("games/catalogue.json"), serde_json::to_vec(&json!({
        "version":3,"games":[{"platform":"NES","image":"synthetic.nes",
            "title":"Synthetic test game","hash":"ab".repeat(32)}]
    })).unwrap()).unwrap();
    let mut server = Server::bind(directory.path()).unwrap();
    let socket = server.path().to_owned();
    let mut child = ChildGuard::new(Command::new(env!("CARGO_BIN_EXE_igui"))
        .args(["--managed", "--headless", "--ipc-socket"]).arg(&socket)
        .current_dir("/").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap());
    let pid = child.id();
    let root = directory.path().to_owned();
    let stopped = Arc::new(AtomicBool::new(false));
    let requested = Arc::new(AtomicBool::new(false));
    let task_stop = stopped.clone(); let task_requested = requested.clone();
    let host = HostGuard::new(stopped, thread::spawn(move || {
        let mut peer = None;
        while !task_stop.load(Ordering::Relaxed) {
            let polled = server.poll(peer, Some(pid));
            assert!(polled.errors.is_empty(), "{:?}", polled.errors);
            for incoming in polled.incoming {
                let request = &incoming.request;
                let response = match request.command.as_str() {
                    "igui.bootstrap" => {
                        peer = Some(incoming.peer);
                        Response::done(request.id, serde_json::to_value(Bootstrap {
                            root: root.clone(), state: GuiState::default(), notice: None,
                            preferences_writable: true, diag_display: false, diag_input: false, input_test: true,
                        }).unwrap())
                    }
                    "igui.input_test" => {
                        assert_eq!(request.params, json!({}));
                        task_requested.store(true, Ordering::Relaxed);
                        Response::done(request.id, json!({"handoff_received":true,"test_started":false}))
                    }
                    "igui.closed" => Response::done(request.id, json!({})),
                    "igui.prepare_launch" | "igui.handoff" => panic!("Test must not launch a game"),
                    _ => {
                        if let Some(peer) = peer {
                            server.forward(peer, &incoming).unwrap();
                        } else {
                            server.send(incoming.peer, &Message::Response(Response::error(request.id, "not_ready", "bootstrap pending"))).unwrap();
                        }
                        continue;
                    }
                };
                server.send(incoming.peer, &Message::Response(response)).unwrap();
            }
            thread::sleep(Duration::from_millis(1));
        }
    }));
    let mut client = Client::connect(&socket).unwrap();
    let deadline = Instant::now() + Duration::from_secs(28);
    let mut saw_countdown = false;
    loop {
        if requested.load(Ordering::Relaxed) { break; }
        if let Ok(status) = client.value("igui.status", json!({})) {
            saw_countdown |= status["input_test_countdown"].is_u64();
            if cancel_countdown && saw_countdown { break; }
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    assert!(saw_countdown);
    if cancel_countdown {
        let result = client.value("igui.input", json!({"action":"b"})).unwrap();
        assert_eq!(result["processed"], true);
        assert_eq!(result["closing"], false);
        // The model covers distant deadlines; these frames exercise modal cleanup.
        for _ in 0..8 {
            let status = client.value("igui.status", json!({})).unwrap();
            assert_eq!(status["ready"], true);
            assert_eq!(status["input_test_countdown"], serde_json::Value::Null);
            assert_eq!(status["input_test_active"], false);
            assert_eq!(status["launching"], false);
            assert!(!directory.path().join("prefs.json").exists());
            thread::sleep(Duration::from_millis(25));
        }
        client.value("igui.close", json!({})).unwrap();
    }
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    host.join().unwrap();
    assert_eq!(requested.load(Ordering::Relaxed), !cancel_countdown);
    assert!(!directory.path().join("prefs.json").exists());
    assert!(!directory.path().join("iman-reports").exists());
}
