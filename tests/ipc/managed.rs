// SPDX-License-Identifier: BSD-3-Clause
//! Scripted manager responses over real local sockets; no display or hardware.
use super::fixture::{receive, send, Server};
use igui::ipc::{Managed, Update};
use imanlib::{igui::{GuiState, Handoff, LaunchRequest}, protocol::{Message, Response}};
use serde_json::json;
use std::{
    io::Read,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

fn wait(managed: &mut Managed) -> Update {
    let deadline = Instant::now() + Duration::from_secs(3);

    loop {
        match managed.poll().unwrap() {
            Update::Waiting => {}
            update => return update,
        }
        assert!(Instant::now() < deadline, "manager response timed out");
        thread::sleep(Duration::from_millis(2));
    }
}

fn make_launch_request() -> LaunchRequest {
    LaunchRequest {
        hash: "ab".repeat(32),
        platform: "NES".into(),
        image: "NES/game.nes".into(),
    }
}

#[test]
fn abandoned_manager_fixture_releases_its_listener_without_a_client() {
    let server = Server::bind(|_| panic!("abandoned fixture must not accept a client"));
    let directory = server.directory().to_owned();

    drop(server);

    assert!(!directory.exists());
}

#[test]
fn assertion_failure_releases_the_active_peer_and_joins_its_worker() {
    let finished = Arc::new(AtomicBool::new(false));
    let worker_finished = finished.clone();
    let (entered, waiting) = mpsc::channel();
    let server = Server::bind(move |mut socket| {
        entered.send(()).unwrap();
        let mut byte = [0];
        assert!(matches!(socket.read(&mut byte), Ok(0) | Err(_)));
        worker_finished.store(true, Ordering::Relaxed);
    });
    let directory = server.directory().to_owned();
    let managed = server.connect();
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();

    let result = catch_unwind(AssertUnwindSafe(move || {
        let _server = server;
        panic!("synthetic assertion failure");
    }));

    assert!(result.is_err());
    assert!(finished.load(Ordering::Relaxed));
    assert!(!directory.exists());
    drop(managed);
}

#[test]
fn input_test_handoff_is_async_and_does_not_send_a_gui_journal_or_close() {
    let mut server = Server::bind_with_input_test(false, true, |mut socket| {
        let request = receive(&mut socket);
        assert_eq!(request.command, "igui.input_test");
        assert_eq!(request.params, json!({}));
        send(&mut socket, Message::Response(Response::done(request.id,
            json!({"handoff_received":true,"test_started":false,"extra":true}))));
    });
    let mut managed = server.connect();
    managed.start_input_test().unwrap();
    assert!(managed.start_input_test().is_err());
    assert!(matches!(wait(&mut managed), Update::HandedOff));
    managed.closed().unwrap();
    server.join();
}

#[test]
fn rejected_input_test_handoff_stays_in_gui_and_closes_explicitly() {
    let mut server = Server::bind_with_input_test(false, true, |mut socket| {
        let request = receive(&mut socket);
        assert_eq!(request.command, "igui.input_test");
        send(&mut socket, Message::Response(Response::error(request.id, "invalid_state", "synthetic refusal")));
        let request = receive(&mut socket);
        assert_eq!(request.command, "igui.closed");
        send(&mut socket, Message::Response(Response::done(request.id, json!({}))));
    });
    let mut managed = server.connect();
    managed.start_input_test().unwrap();
    assert!(matches!(wait(&mut managed), Update::Failed(message) if message.contains("synthetic refusal")));
    managed.closed().unwrap();
    server.join();
}

#[test]
fn invalid_input_test_acknowledgements_never_complete_the_handoff() {
    for result in [
        json!({}),
        json!({"handoff_received":true}),
        json!({"handoff_received":"true","test_started":false}),
        json!({"handoff_received":true,"test_started":0}),
        json!({"handoff_received":false,"test_started":false}),
        json!({"handoff_received":true,"test_started":true}),
    ] {
        let mut server = Server::bind_with_input_test(false, true, move |mut socket| {
            let request = receive(&mut socket);
            assert_eq!(request.command, "igui.input_test");
            send(&mut socket, Message::Response(Response::done(request.id, result)));
        });
        let mut managed = server.connect();
        managed.start_input_test().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);

        let error = loop {
            match managed.poll() {
                Ok(Update::Waiting) => {}
                Err(error) => break error,
                Ok(_) => panic!("Invalid acknowledgement completed the handoff"),
            }
            assert!(Instant::now() < deadline);
            thread::yield_now();
        };

        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(error.to_string(), "Invalid input test handoff acknowledgement");
        server.join();
    }
}

#[test]
fn socket_bootstrap_prepare_handoff_do_not_use_stdio_or_a_launcher() {
    let mut server = Server::bind(|mut socket| {
        for command in ["igui.prepare_launch", "igui.handoff"] {
            let request = receive(&mut socket);
            assert_eq!(request.command, command);
            send(&mut socket, Message::Response(Response::done(request.id, json!({}))));
        }
    });
    let mut managed = server.connect();
    managed.prepare(make_launch_request()).unwrap();
    assert!(matches!(wait(&mut managed), Update::Ready));
    managed
        .handoff(Handoff {
            game: make_launch_request(),
            state: GuiState::default(),
        })
        .unwrap();
    assert!(matches!(wait(&mut managed), Update::HandedOff));
    managed.closed().unwrap();
    server.join();
}
#[test]
fn launch_error_remains_in_gui_and_normal_close_is_explicit() {
    let mut server = Server::bind(|mut socket| {
        let request = receive(&mut socket);
        assert_eq!(request.command, "igui.prepare_launch");
        send(
            &mut socket,
            Message::Response(Response::error(request.id, "launch_refused", "synthetic failure")),
        );
        let request = receive(&mut socket);
        assert_eq!(request.command, "igui.closed");
        send(&mut socket, Message::Response(Response::done(request.id, json!({}))));
    });
    let mut managed = server.connect();
    managed.prepare(make_launch_request()).unwrap();
    assert!(
        matches!(wait(&mut managed),Update::Failed(message) if message.contains("launch_refused"))
    );
    managed.closed().unwrap();
    server.join();
}
#[test]
fn manager_close_event_requests_cleanup_and_client_sends_exit_intent() {
    let mut server = Server::bind(|mut socket| {
        send(
            &mut socket,
            Message::Event {
                event: "igui.close".into(),
                params: json!({"extra":true}),
            },
        );
        let request = receive(&mut socket);
        assert_eq!(request.command, "igui.closed");
        send(&mut socket, Message::Response(Response::done(request.id, json!({}))));
    });
    let mut managed = server.connect();
    assert!(matches!(wait(&mut managed), Update::Close));
    managed.closed().unwrap();
    server.join();
}
#[test]
fn manager_disconnect_is_an_error_not_a_successful_handoff() {
    let mut server = Server::bind(drop);
    let mut managed = server.connect();
    server.join();
    assert!(managed.poll().is_err());
}

#[test]
fn owner_reports_follow_handoff_ack_without_a_closed_request() {
    let mut server = Server::bind_with_diagnostics(true, |mut socket| {
        for command in [
            "igui.prepare_launch",
            "igui.handoff",
            "diag_display.submit",
            "diag_input.submit",
        ] {
            let request = receive(&mut socket);
            assert_eq!(request.command, command);
            send(
                &mut socket,
                Message::Response(Response::done(request.id, json!({}))),
            );
        }
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    let mut managed = server.connect();
    managed.prepare(make_launch_request()).unwrap();
    assert!(matches!(wait(&mut managed), Update::Ready));
    managed
        .handoff(Handoff {
            game: make_launch_request(),
            state: GuiState::default(),
        })
        .unwrap();
    assert!(matches!(wait(&mut managed), Update::HandedOff));

    managed
        .display_report(&imanlib::display_metrics::Sample {
            backend: imanlib::display_metrics::Backend::Headless,
            render: None,
            kms: None,
            present: None,
            idle_check: None,
            probe: None,
            cleanup_ok: Some(true),
            operation_ok: Some(true),
        })
        .unwrap();
    managed
        .input_report(&imanlib::input_metrics::Sample::default())
        .unwrap();
    managed.closed().unwrap();
    drop(managed);

    server.join();
}
