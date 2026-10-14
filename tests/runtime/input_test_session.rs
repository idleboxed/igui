// SPDX-License-Identifier: BSD-3-Clause
//! Real headless IGUI and synthetic host, with a manually advanced startup clock.

use imanlib::{igui::{Bootstrap, GuiState}, protocol::{Message, Response}, server::Server};
use serde_json::json;
use std::{fs, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::{Duration, Instant}};

use crate::common::{ipc::ClockedSession, HostGuard};

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
    let pid = std::process::id();
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
    let mut running = ClockedSession::start(igui::runtime::Config {
        managed: true, ipc_socket: Some(socket), ..Default::default()
    }).unwrap();
    let ready = running.wait_status(|status| status["ready"] == true);
    assert_eq!(ready["input_test_countdown"], serde_json::Value::Null);
    running.set_time(9_999);
    assert_eq!(running.client.value("igui.status", json!({})).unwrap()["input_test_countdown"], serde_json::Value::Null);
    assert!(!requested.load(Ordering::Relaxed));

    running.set_time(10_000);
    let countdown = running.wait_status(|status| status["input_test_countdown"] == 10);
    assert_eq!(countdown["input_test_active"], false);

    if cancel_countdown {
        let result = running.client.value("igui.input", json!({"action":"b"})).unwrap();
        assert_eq!(result["processed"], true);
        assert_eq!(result["closing"], false);
        for milliseconds in [20_000, 80_000] {
            running.set_time(milliseconds);
            let status = running.client.value("igui.status", json!({})).unwrap();
            assert_eq!(status["ready"], true);
            assert_eq!(status["input_test_countdown"], serde_json::Value::Null);
            assert_eq!(status["input_test_active"], false);
            assert_eq!(status["launching"], false);
            assert!(!directory.path().join("prefs.json").exists());
        }
        running.client.value("igui.close", json!({})).unwrap();
    } else {
        running.set_time(19_999);
        running.wait_status(|status| status["input_test_countdown"] == 1);
        assert!(!requested.load(Ordering::Relaxed));
        running.set_time(20_000);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !requested.load(Ordering::Relaxed) {
            assert!(Instant::now() < deadline, "host did not receive the input-test handoff");
            thread::yield_now();
        }
    }
    running.finish();
    host.join().unwrap();
    assert_eq!(requested.load(Ordering::Relaxed), !cancel_countdown);
    assert!(!directory.path().join("prefs.json").exists());
    assert!(!directory.path().join("iman-reports").exists());
}
