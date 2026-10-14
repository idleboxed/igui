// SPDX-License-Identifier: BSD-3-Clause
//! Synthetic catalogue and IPC host; real headless IGUI, no devices or emulator.

use igui::runtime::startup::load_session;
use imanlib::{
    client::Client,
    igui::{Bootstrap, GuiState, Handoff, LaunchRequest},
    protocol::{Message, Response},
    server::Server,
};
use crate::common::ChildGuard;
use serde_json::json;
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread,
    time::{Duration, Instant},
};

fn catalogue(root: &Path) {
    fs::create_dir(root.join("games")).unwrap();
    fs::write(root.join("games/catalogue.json"), json!({"games":[
        {"hash":crate::common::hash("z"), "title":"Zulu", "platform":"NES", "image":"z.nes"},
        {"hash":crate::common::hash("a"), "title":"Alpha", "platform":"NES", "image":"a.nes"},
        {"hash":crate::common::hash("b"), "title":"Beta", "platform":"NES", "image":"b.nes"}
    ]}).to_string()).unwrap();
}

#[test]
fn startup_restores_hash_after_sorting_without_writes_and_missing_hash_falls_back() {
    let dir = tempfile::tempdir_in("/dev/shm").unwrap();
    catalogue(dir.path());
    for (hash, trash, expected) in [
        (crate::common::hash("z"), false, "Zulu"),
        (crate::common::hash("b"), false, "Beta"),
        (crate::common::hash("b"), true, "Beta"),
        (crate::common::hash("gone"), false, "Alpha"),
        (crate::common::hash("gone"), true, "Alpha"),
    ] {
        let original = json!({"favourites":[], "current_item":hash,
            "trash":if trash { vec![hash.clone()] } else { vec![] }}).to_string();
        fs::write(dir.path().join("prefs.json"), &original).unwrap();

        let session = load_session(&dir.path().join("games"), dir.path(), |_, _| {}).unwrap();

        assert_eq!(session.app.selected_game().unwrap().title, expected);
        assert_eq!(session.app.trash_only(), trash && expected == "Beta");
        assert_eq!(fs::read_to_string(dir.path().join("prefs.json")).unwrap(), original);
    }
    fs::write(dir.path().join("games/catalogue.json"), br#"{"games":[]}"#).unwrap();
    let session = load_session(&dir.path().join("games"), dir.path(), |_, _| {}).unwrap();
    assert!(session.app.selected_game().is_none());
}


fn launch(
    root: &Path,
    actions: &[&str],
    state: GuiState,
    writable: bool,
) -> (LaunchRequest, Option<Vec<u8>>, Output) {
    let mut server = Server::bind(root).unwrap();
    let socket = server.path().to_owned();
    let mut child = ChildGuard::new(Command::new(env!("CARGO_BIN_EXE_igui"))
        .args(["--managed", "--headless", "--ipc-socket"]).arg(&socket)
        .current_dir("/").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped())
        .spawn().unwrap());
    let pid = child.id();
    let root = root.to_owned();
    let stopped = Arc::new(AtomicBool::new(false));
    let task_stop = stopped.clone();
    let host = crate::common::HostGuard::new(stopped, thread::spawn(move || {
        let mut peer = None;
        let mut prepared = None;
        let mut handed_off = false;
        let deadline = Instant::now() + Duration::from_secs(15);
        while !task_stop.load(Ordering::Relaxed) && Instant::now() < deadline {
            let polled = server.poll(peer, Some(pid));
            assert!(polled.errors.is_empty(), "{:?}", polled.errors);
            for incoming in polled.incoming {
                let response_item = &incoming.request;
                let response = match response_item.command.as_str() {
                    "igui.bootstrap" => {
                        peer = Some(incoming.peer);
                        Response::done(response_item.id, serde_json::to_value(Bootstrap {
                            root: root.clone(), state: state.clone(), notice: None,
                            preferences_writable: writable, diag_display: false,
                            diag_input: false, input_test: false,
                        }).unwrap())
                    }
                    "igui.prepare_launch" => {
                        assert!(prepared.is_none());
                        let game: LaunchRequest = serde_json::from_value(response_item.params.clone()).unwrap();
                        let prefs = match fs::read(root.join("prefs.json")) {
                            Ok(bytes) => Some(bytes),
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                            Err(error) => panic!("{error}"),
                        };
                        prepared = Some((game, prefs));
                        Response::done(response_item.id, json!({}))
                    }
                    "igui.handoff" => {
                        let handoff: Handoff = serde_json::from_value(response_item.params.clone()).unwrap();
                        assert_eq!(handoff.game, prepared.as_ref().unwrap().0);
                        handed_off = true;
                        Response::done(response_item.id, json!({}))
                    }
                    _ => {
                        if let Some(peer) = peer {
                            server.forward(peer, &incoming).unwrap();
                        } else {
                            server.send(incoming.peer, &Message::Response(Response::error(
                                response_item.id, "not_ready", "bootstrap pending",
                            ))).unwrap();
                        }
                        continue;
                    }
                };
                server.send(incoming.peer, &Message::Response(response)).unwrap();
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(handed_off);
        prepared.unwrap()
    }));
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut client = Client::connect(&socket).unwrap();
    loop {
        if let Ok(status) = client.value("igui.status", json!({})) {
            if status["ready"] == true { break; }
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    for action in actions {
        let reply = client.value("igui.input", json!({"action":action})).unwrap();
        assert_eq!(reply["blocked"], false);
    }
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let mut stderr = Vec::new();
    std::io::Read::read_to_end(&mut child.stderr.take().unwrap(), &mut stderr).unwrap();
    let output = Output { status: child.wait().unwrap(), stdout: vec![], stderr };
    let (game, prefs) = host.join().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    (game, prefs, output)
}

#[test]
fn a_and_start_save_before_prepare_and_restart_restores_the_same_game() {
    let dir = tempfile::tempdir_in("/dev/shm").unwrap();
    catalogue(dir.path());
    let (game, prefs, _) = launch(dir.path(), &["down", "a"], GuiState::default(), true);
    assert_eq!(game.hash, crate::common::hash("b"));
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&prefs.unwrap()).unwrap(),
        json!({"favourites":[], "current_item":crate::common::hash("b")}));

    let original = fs::read(dir.path().join("prefs.json")).unwrap();
    fs::write(dir.path().join(".prefs.json.tmp"), b"do not rewrite identical hash").unwrap();
    let (game, prefs, output) = launch(dir.path(), &["start"], GuiState::default(), true);
    assert_eq!(game.hash, crate::common::hash("b"));
    assert_eq!(prefs.unwrap(), original);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("save not confirmed"));
}

#[test]
fn start_saves_new_selection_and_session_state_takes_priority_over_prefs() {
    let dir = tempfile::tempdir_in("/dev/shm").unwrap();
    catalogue(dir.path());
    fs::write(dir.path().join("prefs.json"), json!({"favourites":[],
        "current_item":crate::common::hash("a"), "lang":"en"}).to_string()).unwrap();
    let state = GuiState { selected_hash: Some(crate::common::hash("z")), ..Default::default() };

    let (game, prefs, _) = launch(dir.path(), &["start"], state, true);

    assert_eq!(game.hash, crate::common::hash("z"));
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&prefs.unwrap()).unwrap(),
        json!({"favourites":[], "current_item":crate::common::hash("z"), "lang":"en"}));
}

#[test]
fn preference_failure_does_not_prevent_launch_and_read_only_never_writes() {
    for writable in [true, false] {
        let dir = tempfile::tempdir_in("/dev/shm").unwrap();
        catalogue(dir.path());
        fs::write(dir.path().join(".prefs.json.tmp"), b"other writer").unwrap();

        let (game, prefs, output) = launch(dir.path(), &["start"], GuiState::default(), writable);

        assert_eq!(game.hash, crate::common::hash("a"));
        assert!(prefs.is_none());
        assert!(!dir.path().join("prefs.json").exists());
        assert_eq!(fs::read(dir.path().join(".prefs.json.tmp")).unwrap(), b"other writer");
        assert_eq!(String::from_utf8_lossy(&output.stderr).contains("save not confirmed"), writable);
    }
}
