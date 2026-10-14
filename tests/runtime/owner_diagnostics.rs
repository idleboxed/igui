// SPDX-License-Identifier: BSD-3-Clause
//! Real headless GUI plus synthetic IPC host; no device access or game execution.

use imanlib::{
    igui::{Bootstrap, GuiState},
    protocol::{Message, Response},
    server::Server,
};
use serde_json::json;
use std::{
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::common::ChildGuard;

#[test]
fn real_gui_reports_only_when_requested_and_rejected_diagnostics_do_not_prevent_close() {
    for (requested, refuse) in [(false, false), (true, false), (true, true)] {
        let dir = tempfile::tempdir_in("/dev/shm").unwrap();
        fs::create_dir(dir.path().join("games")).unwrap();
        fs::write(
            dir.path().join("games/catalogue.json"),
            br#"{"version":3,"games":[]}"#,
        )
        .unwrap();
        let mut server = Server::bind(dir.path()).unwrap();
        let mut child = ChildGuard::new(
            Command::new(env!("CARGO_BIN_EXE_igui"))
                .args(["--managed", "--headless", "--ipc-socket"])
                .arg(server.path())
                .current_dir("/")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let mut peer = None;
        let mut reports = Vec::new();
        let mut closed = false;
        let mut close_sent = false;
        let started = Instant::now();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let polled = server.poll(peer, Some(child.id()));
            assert!(polled.errors.is_empty(), "{:?}", polled.errors);
            for incoming in polled.incoming {
                assert_eq!(incoming.pid, child.id());
                let report = incoming.request;
                let response = match report.command.as_str() {
                    "igui.bootstrap" => {
                        assert!(peer.is_none());
                        peer = Some(incoming.peer);
                        Response::done(
                            report.id,
                            serde_json::to_value(Bootstrap {
                                diag_display: requested,
                                diag_input: requested,
                                input_test: false,
                                preferences_writable: false,
                                root: dir.path().to_owned(),
                                state: GuiState::default(),
                                notice: None,
                            })
                            .unwrap(),
                        )
                    }
                    "diag_display.submit" | "diag_input.submit" => {
                        assert!(requested);
                        if report.command == "diag_display.submit" {
                            let sample: imanlib::display_metrics::Sample =
                                serde_json::from_value(report.params).unwrap();
                            assert!(sample.validate());
                            assert_eq!(sample.backend, imanlib::display_metrics::Backend::Headless);
                            assert!(sample.kms.is_none());
                            assert_eq!(sample.cleanup_ok, Some(true));
                            assert!(sample.render.is_some());
                        } else {
                            let sample: imanlib::input_metrics::Sample =
                                serde_json::from_value(report.params).unwrap();
                            assert!(sample.validate() && sample.headless);
                            assert_eq!(
                                sample.evdev.connections
                                    + sample.hid.connections
                                    + sample.joydev.connections,
                                0
                            );
                        }
                        reports.push(report.command);
                        if refuse {
                            Response::error(report.id, "invalid_state", "synthetic stopped collector")
                        } else {
                            Response::done(report.id, json!({"received":true}))
                        }
                    }
                    "igui.closed" => {
                        closed = true;
                        Response::done(report.id, json!({}))
                    }
                    command => panic!("unexpected command {command}"),
                };
                server
                    .send(incoming.peer, &Message::Response(response))
                    .unwrap();
            }
            if !close_sent && started.elapsed() >= Duration::from_millis(200) {
                if let Some(peer) = peer {
                    server
                        .send(
                            peer,
                            &Message::Event {
                                event: "igui.close".into(),
                                params: json!({}),
                            },
                        )
                        .unwrap();
                    close_sent = true;
                }
            }
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "headless GUI timed out");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(closed);
        assert_eq!(reports.len(), if requested { 2 } else { 0 });
        assert!(!dir.path().join("prefs.json").exists());
    }
}
