use crate::common::ipc as ipc_support;

use imanlib::igui::ScreenshotResult;
use std::{
    fs,
    path::PathBuf,
    process::Command,
};

use crate::common::Scratch;

impl Scratch {
    fn games(&self) -> PathBuf {
        let path = self.root().join("games");
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn driver(&self) -> ipc_support::Driver {
        ipc_support::Driver::new(std::path::Path::new(env!("CARGO_BIN_EXE_igui")), self.root())
    }
    fn installed_driver(&self) -> ipc_support::Driver {
        let binary = self.root().join("igui");
        if !binary.exists() {
            fs::hard_link(env!("CARGO_BIN_EXE_igui"), &binary)
                .or_else(|_| fs::copy(env!("CARGO_BIN_EXE_igui"), &binary).map(|_| ())).unwrap();
        }
        ipc_support::Driver::new(&binary, self.root())
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_igui"));
        command.current_dir(self.root());
        command
    }
}

fn pixels_without_telemetry(path: impl AsRef<std::path::Path>) -> Vec<u8> {
    let mut pixels = crate::common::images::read_screenshot(path.as_ref());
    // System memory and disk space can change independently between screenshots.
    for platform_button in 684..720 {
        pixels[(platform_button * 1280 + igui::render::FOOTER_STATS_X as usize) * 3..(platform_button * 1280 + 1280) * 3]
            .fill(0);
    }
    pixels
}

#[test]
fn explicit_screenshot_works_without_display_and_creates_only_requested_file() {
    let scratch = Scratch::new();
    let output = scratch
        .driver()
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .input("up,right,a,down,left,down,a,right,a")
        .screenshot("result.png")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let png = fs::read(scratch.root().join("result.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 1280);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 720);
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 1);
}

#[test]
fn search_edit_clear_and_close_do_not_write_preferences() {
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        serde_json::json!({"version":3,"games":[
            {"hash":crate::common::hash("a"), "image":"a.bin", "title":"Alpha", "platform":"NES"},
            {"hash":crate::common::hash("b"), "image":"b.bin", "title":"Beta", "platform":"NES"}
        ]}).to_string(),
    ).unwrap();
    let preferences = br#"{"lang":"ru","favourites":[],"trash":[],"custom":true}"#;
    fs::write(scratch.root().join("prefs.json"), preferences).unwrap();
    for (file, events) in [
        ("baseline.png", "y"),
        ("search.png", "y,a,down,down,right,a"), // B, filters immediately
        ("field.png", "y,a,down,down,right,a,up,up"),
        ("closed.png", "y,a,down,down,right,a,up,up,left,right,x,b,y"),
    ] {
        let output = scratch.driver().env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events).screenshot(file).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(fs::read(scratch.root().join("prefs.json")).unwrap(), preferences);
        assert!(!scratch.root().join(".prefs.json.tmp").exists());
    }
    // The filtered selection survives clear/close, so compare the panel and hints,
    // not the selected row or card with the initial Alpha selection.
    let baseline = pixels_without_telemetry(scratch.root().join("baseline.png"));
    let closed = pixels_without_telemetry(scratch.root().join("closed.png"));
    for platform_button in 90..156 {
        assert_eq!(&baseline[(platform_button * 1280 + 82) * 3..(platform_button * 1280 + 620) * 3],
            &closed[(platform_button * 1280 + 82) * 3..(platform_button * 1280 + 620) * 3]);
    }
    assert_eq!(&baseline[684 * 1280 * 3..], &closed[684 * 1280 * 3..]);
}

#[test]
fn selecting_a_search_result_restores_the_same_game_without_opening_launch_modal() {
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        serde_json::json!({"version":3,"games":[
            {"hash":crate::common::hash("a"), "image":"a.bin", "title":"Alpha", "platform":"NES"},
            {"hash":crate::common::hash("b"), "image":"b.bin", "title":"Beta", "platform":"NES"}
        ]}).to_string(),
    ).unwrap();
    for (file, events) in [
        ("selected.png", "down"),
        ("search-selected.png", "y,a,down,down,right,a,left,left,a"),
    ] {
        let output = scratch.driver().env_remove("DISPLAY")
            .args(["--games", "games"]).input(events).screenshot(file).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    assert_eq!(
        pixels_without_telemetry(scratch.root().join("selected.png")),
        pixels_without_telemetry(scratch.root().join("search-selected.png")),
    );
    assert!(!scratch.root().join("prefs.json").exists());
}

#[test]
fn invalid_inputs_and_help_do_not_create_files() {
    let scratch = Scratch::new();
    let unknown = scratch.command().arg("--unknown").output().unwrap();
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown option: --unknown"));

    for args in [
        vec!["--headless"],
        vec!["--ipc-socket"],
        vec!["--frames", "0"],
        vec!["--controls"],
        vec!["--controls", "one", "--controls", "two"],
        vec!["--hid-profile"],
        vec!["--evdev-profile"],
        vec!["--hid-profile", "one", "--hid-profile", "two"],
        vec!["--evdev-profile", "one", "--evdev-profile", "two"],
        vec!["--hid-profile", "one", "--gamepad", "js0"],
        vec!["--evdev-profile", "one", "--input-probe", "--frames", "1"],
        vec![
            "--hid-profile",
            "one",
            "--headless",
            "--ipc-socket",
            "absent",
        ],
        vec!["--input-probe"],
        vec!["--input-probe", "--frames", "18001"],
        vec![
            "--input-probe",
            "--frames",
            "5",
            "--gamepad",
            "/dev/input/js0",
        ],
        vec!["--input-probe", "--frames", "5", "--demo-load-ms", "1"],
        vec!["--input-probe", "--input-probe", "--frames", "5"],
        vec!["--drm"],
        vec!["--drm", "--drm"],
        vec!["--drm", "--games", "games"],
        vec!["--drm", "--games", "games", "--frames", "18001"],
        vec!["--drm", "--games", "games", "--frames", "1", "--managed"],
        vec![
            "--drm",
            "--games",
            "games",
            "--frames",
            "1",
            "--demo-load-ms",
            "1",
        ],
        vec!["--allow-vendor-einval"],
        vec!["--managed", "--managed"],
        vec!["--managed", "--games", "games"],
        vec!["--games"],
        vec!["--gamepad"],
        vec!["--gamepad-select"],
        vec!["--gamepad-select", "8"],
        vec!["--gamepad", "one", "--gamepad-select", "256"],
        vec!["--gamepad", "one", "--gamepad-select", "-1"],
        vec![
            "--gamepad",
            "one",
            "--gamepad-select",
            "8",
            "--gamepad-select",
            "9",
        ],
        vec![
            "--gamepad",
            "one",
            "--gamepad-buttons",
            "2,1,3,0",
            "--gamepad-select",
            "2",
        ],
        vec!["--gamepad-buttons", "2,1,3"],
        vec!["--gamepad", "one", "--gamepad-buttons", "1,1,2"],
        vec!["--gamepad", "one", "--gamepad-buttons", "256,1,2"],
        vec!["--gamepad", "one", "--gamepad-buttons", "1,2"],
        vec!["--gamepad", "one", "--gamepad-buttons", "1,2,1"],
        vec!["--gamepad", "one", "--gamepad-buttons", "1,2,2"],
        vec!["--gamepad", "one", "--gamepad-buttons", "1,2,3,3"],
        vec!["--gamepad", "one", "--gamepad", "two"],
        vec!["--games", "one", "--games", "two"],
        vec!["--demo-load-ms"],
        vec!["--demo-load-ms", "-1"],
        vec!["--demo-load-ms", "60001"],
        vec!["--demo-load-ms", "no"],
        vec!["--demo-load-ms", "1", "--demo-load-ms", "2"],
    ] {
        assert!(!scratch
            .command()
            .args(args)
            .output()
            .unwrap()
            .status
            .success());
    }
    let help = scratch.command().arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert_eq!(text.lines().next(), Some("IGUI"));
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 0);
}

#[test]
fn headless_input_probe_exposes_diagnostics_but_refuses_logical_actions_and_saves_no_preferences() {
    let scratch = Scratch::new();
    let prefs = scratch.root().join("prefs.json");
    fs::write(&prefs, b"preserve exact preferences\n").unwrap();
    let mut running = scratch
        .driver()
        .args(["--input-probe", "--frames", "18000"])
        .start()
        .unwrap();
    running.wait_ready().unwrap();
    let status = running
        .client
        .value("igui.status", serde_json::json!({"extra":true}))
        .unwrap();
    assert_eq!(status["input_probe"], true);
    assert_eq!(status["headless"], true);
    assert_eq!(status["connected_streams"], 0);
    assert_eq!(status["physical_players_verified"], false);
    assert!(running
        .client
        .value("igui.input", serde_json::json!({"action":"b"}))
        .is_err());
    let screenshot: ScreenshotResult = serde_json::from_value(
        running.client.value("igui.screenshot", serde_json::json!({"extra":true})).unwrap(),
    ).unwrap();
    assert_eq!(screenshot.format, "png");
    assert!(screenshot.temporary);
    assert_eq!((screenshot.width, screenshot.height), (1280, 720));
    let path = screenshot.path;
    assert!(path.exists());
    running
        .client
        .value("igui.close", serde_json::json!({"extra":true}))
        .unwrap();
    let output = running.finish().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(log.contains("\"reason\":\"ipc_close\""));
    assert!(log.contains("\"connections\":0"));
    assert!(log.contains("\"format\":\"IGUI-INPUT-2\""));
    assert!(log.contains("\"usb_snapshots\":0"));
    assert!(log.contains("\"operator_confirmed_steps\":[]"));
    assert!(!path.exists());
    assert_eq!(fs::read(prefs).unwrap(), b"preserve exact preferences\n");
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 1);
}

#[test]
fn saved_language_is_loaded_without_rewriting_preferences_and_survives_favourite_save() {
    let scratch = Scratch::new();
    fs::create_dir(scratch.root().join("locales")).unwrap();
    fs::write(
        scratch.root().join("locales/ru.json"),
        include_bytes!("../locales/ru.json"),
    )
    .unwrap();
    fs::write(scratch.games().join("catalogue.json"), br#"{"version":3,"games":[{"platform":"GG","image":"demo.gg","title":"Demo","genre":"Platformer","hash":"64656d6f2e676700000000000000000000000000000000000000000000000000"}]}"#).unwrap();
    let original = br#"{"lang":"ru","favourites":[]}"#;
    fs::write(scratch.root().join("prefs.json"), original).unwrap();
    let output = scratch
        .installed_driver()
        .env_remove("DISPLAY")
        .args(["--games", "games"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read(scratch.root().join("prefs.json")).unwrap(), original);
    for (events, file) in [("b,right,up", "selected.png"), ("right,up", "restored.png")] {
        let output = scratch
            .installed_driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events)
            .screenshot(file)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let prefs: serde_json::Value =
            serde_json::from_slice(&fs::read(scratch.root().join("prefs.json")).unwrap()).unwrap();
        assert_eq!(
            prefs,
            serde_json::json!({"lang":"ru", "favourites":[crate::common::hash("demo.gg")]})
        );
    }
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("selected.png")),
        pixels_without_telemetry(scratch.root().join("restored.png"))
    );
}

#[test]
fn missing_or_bad_language_preferences_and_files_render_builtin_english_without_writes() {
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        br#"{"version":3,"games":[]}"#,
    )
    .unwrap();
    fs::create_dir(scratch.root().join("locales")).unwrap();
    fs::write(
        scratch.root().join("locales/ru.json"),
        include_bytes!("../locales/ru.json"),
    )
    .unwrap();
    fs::write(scratch.root().join("locales/de.json"), b"bad JSON").unwrap();
    let output = scratch
        .installed_driver()
        .env_remove("DISPLAY")
        .args(["--games", "games"])
        .screenshot("english.png")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!scratch.root().join("prefs.json").exists());
    let english = pixels_without_telemetry(scratch.root().join("english.png"));
    for lang in [
        None,
        Some(serde_json::json!("en")),
        Some(serde_json::json!("zz")),
        Some(serde_json::json!("de")),
        Some(serde_json::json!("../ru")),
        Some(serde_json::json!(42)),
        Some(serde_json::json!(null)),
    ] {
        let mut prefs = serde_json::json!({"favourites":[]});
        if let Some(lang) = lang {
            prefs["lang"] = lang;
        }
        let original = serde_json::to_vec(&prefs).unwrap();
        fs::write(scratch.root().join("prefs.json"), &original).unwrap();
        let output = scratch
            .installed_driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .screenshot("fallback.png")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(scratch.root().join("prefs.json")).unwrap(), original);
        assert_eq!(
            pixels_without_telemetry(scratch.root().join("fallback.png")),
            english
        );
        fs::remove_file(scratch.root().join("fallback.png")).unwrap();
    }
    fs::write(
        scratch.root().join("prefs.json"),
        br#"{"favourites":[],"lang":"ru"}"#,
    )
    .unwrap();
    assert!(scratch
        .installed_driver()
        .args(["--games", "games"])
        .screenshot("russian.png")
        .output()
        .unwrap()
        .status
        .success());
    assert_ne!(
        pixels_without_telemetry(scratch.root().join("russian.png")),
        english
    );
}

#[test]
fn b_persists_immediately_and_a_second_run_restores_then_removes_the_star() {
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        br#"{"version":3,"games":[{"platform":"GG","image":"demo.gg","title":"Demo","hash":"64656d6f2e676700000000000000000000000000000000000000000000000000"}]}"#,
    )
    .unwrap();
    for (events, file, expected) in [
        ("b", "on.png", true),
        ("", "loaded.png", true),
        ("b", "off.png", false),
    ] {
        let output = scratch
            .driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events)
            .screenshot(file)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let prefs: serde_json::Value =
            serde_json::from_slice(&fs::read(scratch.root().join("prefs.json")).unwrap()).unwrap();
        assert_eq!(
            prefs["favourites"].as_array().unwrap().len(),
            usize::from(expected)
        );
    }
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("on.png")),
        pixels_without_telemetry(scratch.root().join("loaded.png"))
    );
    assert_ne!(
        pixels_without_telemetry(scratch.root().join("on.png")),
        pixels_without_telemetry(scratch.root().join("off.png"))
    );
    assert!(!scratch.root().join(".prefs.json.tmp").exists());
}

#[test]
fn screenshot_refuses_to_overwrite_existing_data() {
    let scratch = Scratch::new();
    fs::write(scratch.root().join("existing.png"), b"untouched").unwrap();
    let output = scratch.driver().screenshot("existing.png").output();
    assert_eq!(
        output.unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        fs::read(scratch.root().join("existing.png")).unwrap(),
        b"untouched"
    );
}

#[cfg(unix)]
#[test]
fn screenshot_refuses_symlink_destination() {
    let scratch = Scratch::new();
    fs::write(scratch.root().join("original"), b"untouched").unwrap();
    std::os::unix::fs::symlink("original", scratch.root().join("link.png")).unwrap();
    assert!(scratch.driver().screenshot("link.png").output().is_err());
    assert_eq!(fs::read(scratch.root().join("original")).unwrap(), b"untouched");
}

#[test]
fn b_closes_dropdown_without_writing_preferences() {
    let scratch = Scratch::new();
    let catalogue = serde_json::json!({"version":3,"games":[{
        "hash": crate::common::hash("demo"), "image":"demo.bin", "title":"Demo", "platform":"MD", "players":2
    }]});
    fs::write(
        scratch.games().join("catalogue.json"),
        catalogue.to_string(),
    )
    .unwrap();
    let output = scratch
        .driver()
        .env_remove("DISPLAY")
        .args(["--games", "games"])
        .input("x,a,down,b")
        .screenshot("closed.png")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!scratch.root().join("prefs.json").exists());
    assert!(!scratch.root().join("games/prefs.json").exists());
    assert!(!scratch.root().join(".prefs.json.tmp").exists());
}

#[cfg(all(feature = "drm", target_os = "linux"))]
#[test]
fn drm_mode_uses_explicit_device_without_board_or_frame_limit_checks() {
    let scratch = Scratch::new();
    let device = scratch.root().join("card2");
    fs::write(&device, b"regular file, not a real device").unwrap();
    // No --frames: the generic backend must reach device validation, on any CPU.
    let output = scratch
        .command()
        .args([
            "--drm",
            "--games",
            "nonexistent",
            "--drm-device",
        ])
        .arg(&device)
        .arg("--controls")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(igui::paths::CONTROLS_DIR))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("unexpected DRM primary device"), "{error}");
    assert_eq!(
        fs::read(&device).unwrap(),
        b"regular file, not a real device"
    );
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 1);
}

#[test]
fn launch_modal_blocks_following_actions_without_writing_or_changing_selection() {
    let scratch = Scratch::new();
    let catalogue = serde_json::json!({"version":3,"games":[{
        "hash":crate::common::hash("first"), "image":"first.bin", "title":"Aladdin", "platform":"MD"
    }, {
        "hash":crate::common::hash("second"), "image":"second.bin", "title":"Zebra", "platform":"MD"
    }]});
    fs::write(
        scratch.games().join("catalogue.json"),
        catalogue.to_string(),
    )
    .unwrap();
    for (file, events) in [
        ("launch.png", "a"),
        (
            "blocked.png",
            "a,b,delete,select,down,y,x,a,right,up,right,a",
        ),
    ] {
        let output = scratch
            .driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events)
            .screenshot(file)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let first = pixels_without_telemetry(scratch.root().join("launch.png"));
    assert_eq!(
        first,
        pixels_without_telemetry(scratch.root().join("blocked.png"))
    );
    assert_eq!(
        &first[(360 * 1280 + 200) * 3..(360 * 1280 + 200) * 3 + 3],
        &[0xe6, 0xea, 0xe7]
    );
    assert!(!scratch.root().join("prefs.json").exists());
    assert!(!scratch.root().join(".prefs.json.tmp").exists());
}

#[test]
fn launching_with_an_empty_list_never_shows_the_dialog() {
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        br#"{"version":3,"games":[]}"#,
    )
    .unwrap();
    for (file, events) in [("empty.png", ""), ("empty-x.png", "a")] {
        assert!(scratch
            .driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events)
            .screenshot(file)
            .output()
            .unwrap()
            .status
            .success());
    }
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("empty.png")),
        pixels_without_telemetry(scratch.root().join("empty-x.png"))
    );
    assert!(!scratch.root().join("prefs.json").exists());
}

#[test]
fn trash_persists_across_restarts_and_restores_without_deleting_game_files() {
    use serde_json::{json, Value};
    let scratch = Scratch::new();
    let first = crate::common::hash("first");
    let second = crate::common::hash("second");
    let catalogue = json!({"version":3,"games":[
        {"hash":first,"image":"first.bin","title":"Alpha","platform":"MD"},
        {"hash":second,"image":"second.bin","title":"Beta","platform":"NES"}
    ]})
    .to_string();
    fs::write(scratch.games().join("catalogue.json"), &catalogue).unwrap();
    fs::create_dir(scratch.games().join("images")).unwrap();
    fs::write(
        scratch.games().join("images/first.bin"),
        b"ROM fixture, not executed",
    )
    .unwrap();
    fs::write(
        scratch.root().join("prefs.json"),
        json!({"favourites":[first],"other":42}).to_string(),
    )
    .unwrap();
    for (file, events, trashed) in [
        ("before.png", "", false),
        ("confirm.png", "select", false),
        ("blocked.png", "delete,down,up,left,right,y,x,select", false),
        ("cancelled.png", "delete,b", false),
        ("deleted.png", "delete,a", true),
        ("reloaded.png", "", true),
        ("trash.png", "y,right,right,a,down", true),
        ("restored.png", "y,right,right,a,down,select", false),
        ("after.png", "", false),
    ] {
        let original = fs::read(scratch.root().join("prefs.json")).unwrap();
        let output = scratch
            .driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events)
            .screenshot(file)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = fs::read(scratch.root().join("prefs.json")).unwrap();
        let prefs: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(prefs["favourites"], json!([first]));
        assert_eq!(prefs["other"], 42);
        if trashed {
            assert_eq!(prefs["trash"], json!([first]));
        } else {
            assert!(prefs.get("trash").is_none_or(|value_item| *value_item == json!([])));
        }
        if !events.ends_with("delete,a") && !events.ends_with("down,select") {
            assert_eq!(bytes, original); // opening/cancelling the modal also must not save
        }
        let session =
            igui::runtime::startup::load_session(&scratch.games(), &scratch.root().join("no-locales"), |_, _| {})
                .unwrap();
        assert_eq!(session.app.visible_count(), if trashed { 1 } else { 2 });
        assert_eq!(
            session.app.selected_game().unwrap().hash,
            if trashed { &second } else { &first }.as_str()
        );
    }
    assert_ne!(
        pixels_without_telemetry(scratch.root().join("before.png")),
        pixels_without_telemetry(scratch.root().join("confirm.png"))
    );
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("confirm.png")),
        pixels_without_telemetry(scratch.root().join("blocked.png"))
    );
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("before.png")),
        pixels_without_telemetry(scratch.root().join("cancelled.png"))
    );
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("deleted.png")),
        pixels_without_telemetry(scratch.root().join("reloaded.png"))
    );
    assert_eq!(
        pixels_without_telemetry(scratch.root().join("before.png")),
        pixels_without_telemetry(scratch.root().join("after.png"))
    );
    assert_ne!(
        pixels_without_telemetry(scratch.root().join("trash.png")),
        pixels_without_telemetry(scratch.root().join("reloaded.png"))
    );
    assert_eq!(
        fs::read_to_string(scratch.games().join("catalogue.json")).unwrap(),
        catalogue
    );
    assert_eq!(
        fs::read(scratch.games().join("images/first.bin")).unwrap(),
        b"ROM fixture, not executed"
    );
    assert!(!scratch.root().join(".prefs.json.tmp").exists());
}

#[test]
fn failed_trash_write_keeps_the_row_and_displays_a_save_error() {
    use serde_json::json;
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        json!({"version":3,"games":[
            {"hash":crate::common::hash("a"),"image":"a.bin","title":"Alpha","platform":"MD"},
            {"hash":crate::common::hash("b"),"image":"b.bin","title":"Beta","platform":"MD"},
            {"hash":crate::common::hash("c"),"image":"c.bin","title":"Gamma","platform":"MD"}
        ]})
        .to_string(),
    )
    .unwrap();
    fs::write(scratch.root().join(".prefs.json.tmp"), b"other writer").unwrap();
    for (file, events) in [("before.png", "down"), ("failed.png", "down,delete,a")] {
        let output = scratch
            .driver()
            .env_remove("DISPLAY")
            .args(["--games", "games"])
            .input(events)
            .screenshot(file)
            .output()
            .unwrap();
        assert!(output.status.success());
        if events.ends_with("delete,a") {
            assert!(String::from_utf8_lossy(&output.stderr).contains("save not confirmed"));
        }
    }
    let before = pixels_without_telemetry(scratch.root().join("before.png"));
    let failed = pixels_without_telemetry(scratch.root().join("failed.png"));
    for platform_button in 164..588 {
        assert_eq!(
            &before[(platform_button * 1280 + 82) * 3..(platform_button * 1280 + 620) * 3],
            &failed[(platform_button * 1280 + 82) * 3..(platform_button * 1280 + 620) * 3]
        );
    }
    assert_ne!(before, failed); // visible error notice, not a silent failure
    assert!(!scratch.root().join("prefs.json").exists());
    assert_eq!(
        fs::read(scratch.root().join(".prefs.json.tmp")).unwrap(),
        b"other writer"
    );
}

#[test]
fn explicit_exit_stops_later_ipc_actions() {
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        br#"{"version":3,"games":[]}"#,
    )
    .unwrap();
    let output = scratch
        .driver()
        .args(["--games", "games"])
        .input("right,up,a,down,left,b")
        .screenshot("after-exit.png")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!scratch.root().join("after-exit.png").exists());
    assert!(!scratch.root().join("prefs.json").exists());
}

#[test]
fn ipc_screenshot_is_ram_only_replaced_and_removed_on_close() {
    use serde_json::json;
    use std::os::unix::fs::MetadataExt;
    let scratch = Scratch::new();
    let mut gui = scratch.driver().start().unwrap();
    gui.wait_ready().unwrap();

    // A separate connection, not the registered GUI, controls the same session.
    let mut other = imanlib::client::Client::connect(&gui.socket).unwrap();
    let first = other.value("igui.screenshot", json!({})).unwrap();
    let path = PathBuf::from(first["path"].as_str().unwrap());
    assert!(path.starts_with(gui.socket.parent().unwrap()));
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    assert_eq!(
        fs::metadata(path.parent().unwrap()).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(&fs::read(&path).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(first["temporary"], true);
    assert_eq!(first["width"], 1280);
    assert_eq!(first["height"], 720);
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 0);

    assert!(other.value("igui.screenshot", json!({"path":42})).is_err());
    assert!(path.exists()); // invalid requests do not destroy the previous capture
    let next = gui.client.value("igui.screenshot", json!({})).unwrap();
    let next = PathBuf::from(next["path"].as_str().unwrap());
    assert!(!path.exists());
    assert!(next.exists());
    gui.client.value("igui.close", json!({})).unwrap();
    assert!(gui.finish().unwrap().status.success());
    assert!(!next.exists());
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 0);
}

#[test]
fn ipc_close_during_loading_cancels_the_delay_without_writing_preferences() {
    use serde_json::json;
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        br#"{"version":3,"games":[]}"#,
    )
    .unwrap();
    let preferences = br#"{"lang":"en","favourites":[]}"#;
    fs::write(scratch.root().join("prefs.json"), preferences).unwrap();
    let mut gui = scratch
        .driver()
        .args(["--games", "games", "--demo-load-ms", "60000"])
        .start()
        .unwrap();

    assert_eq!(gui.wait_status().unwrap()["ready"], false);
    gui.client.value("igui.close", json!({})).unwrap();
    let output = gui.finish().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fs::read(scratch.root().join("prefs.json")).unwrap(), preferences);
    assert!(!scratch.root().join(".prefs.json.tmp").exists());
}

#[test]
fn ipc_loading_refuses_input_without_queueing_and_reports_invalid_parameters() {
    use serde_json::json;
    let scratch = Scratch::new();
    fs::write(
        scratch.games().join("catalogue.json"),
        br#"{"version":3,"games":[]}"#,
    )
    .unwrap();
    let mut gui = scratch
        .driver()
        .args(["--games", "games", "--demo-load-ms", "800"])
        .start()
        .unwrap();
    assert_eq!(gui.wait_status().unwrap()["ready"], false);
    let error = gui
        .client
        .value("igui.input", json!({"action":"b"}))
        .unwrap_err();
    assert!(error.to_string().contains("not_ready"));
    let splash = gui.client.value("igui.screenshot", json!({})).unwrap();
    assert!(PathBuf::from(splash["path"].as_str().unwrap()).exists());
    gui.wait_ready().unwrap();
    for params in [
        json!({}),
        json!({"action":"bogus"}),
    ] {
        assert!(gui
            .client
            .value("igui.input", params)
            .unwrap_err()
            .to_string()
            .contains("invalid_params"));
    }
    assert_eq!(gui.client.value("igui.input", json!({"action":"up","extra":1})).unwrap()["processed"], true);
    assert_eq!(gui.client.value("igui.status", json!({"extra":1})).unwrap()["ready"], true);
    assert!(!scratch.root().join("prefs.json").exists());
    gui.client.value("igui.close", json!({"extra":true})).unwrap();
    assert!(gui.finish().unwrap().status.success());
}

#[test]
fn ipc_explicit_screenshot_survives_replacement_and_exit() {
    use serde_json::json;
    let scratch = Scratch::new();
    let mut gui = scratch.driver().start().unwrap();
    gui.wait_ready().unwrap();
    let ram = gui.client.value("igui.screenshot", json!({})).unwrap();
    let ram = PathBuf::from(ram["path"].as_str().unwrap());
    let destination = scratch.root().join("chosen name.png");

    let result = gui
        .client
        .value("igui.screenshot", json!({"path":destination,"extra":true}))
        .unwrap();

    assert_eq!(result["path"], destination.to_str().unwrap());
    assert_eq!(result["temporary"], false);
    assert_eq!(result["width"], 1280);
    assert_eq!(result["height"], 720);
    let bytes = fs::read(&destination).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(ram.exists()); // Explicit save does not replace the last temporary capture.
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 1);
    gui.client.value("igui.screenshot", json!({})).unwrap();
    assert!(!ram.exists());
    gui.client.value("igui.close", json!({})).unwrap();
    assert!(gui.finish().unwrap().status.success());
    assert_eq!(fs::read(destination).unwrap(), bytes);
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 1);
}

#[test]
fn ipc_explicit_screenshot_refuses_existing_entries_and_invalid_paths_without_changes() {
    use serde_json::json;
    let scratch = Scratch::new();
    let existing = scratch.root().join("existing.png");
    fs::write(&existing, b"untouched").unwrap();
    let link = scratch.root().join("link.png");
    std::os::unix::fs::symlink(&existing, &link).unwrap();
    let dangling = scratch.root().join("dangling.png");
    std::os::unix::fs::symlink(scratch.root().join("missing.png"), &dangling).unwrap();
    let directory = scratch.root().join("directory");
    fs::create_dir(&directory).unwrap();
    let mut gui = scratch.driver().start().unwrap();
    gui.wait_ready().unwrap();

    for params in [
        json!({"path":existing}),
        json!({"path":link}),
        json!({"path":dangling}),
        json!({"path":directory}),
        json!({"path":scratch.root().join("missing/shot.png")}),
        json!({"path":scratch.root().join("existing.png/shot.png")}),
        json!({"path":"relative.png"}),
        json!({"path":""}),
        json!({"path":"/"}),
        json!({"path":format!("{}/new/", scratch.root().display())}),
        json!({"path":scratch.root().join("../escape.png")}),
        json!({"path":format!("{}/new/.", scratch.root().display())}),
        json!({"path":42}),
    ] {
        assert!(
            gui.client.value("igui.screenshot", params.clone()).is_err(),
            "{params}"
        );
        assert_eq!(fs::read(&existing).unwrap(), b"untouched");
        assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 4);
    }
    assert!(!scratch.root().join("missing.png").exists());
    assert!(link.is_symlink());
    assert!(dangling.is_symlink());
    gui.client.value("igui.close", json!({})).unwrap();
    assert!(gui.finish().unwrap().status.success());
}

#[test]
fn invalid_profiles_are_rejected_before_display_or_ipc_access() {
    let scratch = Scratch::new();
    let path = scratch.root().join("bad.json");
    fs::write(&path, "{}\n").unwrap();
    for option in ["--hid-profile", "--evdev-profile"] {
        let output = scratch
            .command()
            .env_remove("DISPLAY")
            .arg(option)
            .arg(&path)
            .args(["--games", "absent-games", "--ipc-socket", "absent"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("profile"), "{error}");
        assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 1);
    }
}


#[cfg(target_os = "linux")]
#[test]
fn installed_controls_follow_executable_not_cwd_and_are_reloaded_without_rebuild() {
    let scratch = Scratch::new();
    let executable = scratch.root().join("igui");
    if fs::hard_link(env!("CARGO_BIN_EXE_igui"), &executable).is_err() {
        fs::copy(env!("CARGO_BIN_EXE_igui"), &executable).unwrap();
    }
    let controls = scratch.root().join("controls");
    fs::create_dir(&controls).unwrap();
    for name in [crate::common::HIDRAW_FILE, crate::common::EVDEV_20BC_5500_FILE, crate::common::EVDEV_2563_0555_FILE] {
        fs::write(controls.join(name), crate::common::control_bytes(name)).unwrap();
    }
    let working = scratch.root().join("unrelated");
    fs::create_dir(&working).unwrap();
    let run = || Command::new(&executable).current_dir(&working)
        .args(["--games", "absent-games", "--ipc-socket", "absent-socket"]).env_remove("DISPLAY").output().unwrap();

    let output = run();

    // The deliberately missing IPC socket stops execution before display/input access.
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("input profile"));
    let profile = controls.join(crate::common::EVDEV_20BC_5500_FILE);
    fs::write(&profile, b"{}").unwrap();

    let output = run();

    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("input profile") && error.contains(profile.to_str().unwrap()), "{error}");
    assert_eq!(fs::read_dir(&working).unwrap().count(), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn explicit_controls_directory_is_validated_before_ipc_or_display_access() {
    let scratch = Scratch::new();
    let controls = scratch.root().join("custom-controls");
    fs::create_dir(&controls).unwrap();
    for probe in [false, true] {
        let mut command = scratch.command();
        command.arg("--controls").arg(&controls).args(["--games", "absent-games", "--ipc-socket", "absent"]);
        if probe {
            command.args(["--input-probe", "--frames", "1"]);
        }

        let output = command.output().unwrap();

        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("no input profiles") && error.contains(controls.to_str().unwrap()), "{error}");
        assert_eq!(fs::read_dir(&controls).unwrap().count(), 0);
    }
}

#[test]
fn headless_session_needs_no_controller_files() {
    let scratch = Scratch::new();

    let output = scratch.driver().args(["--controls", "absent-controls"]).output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fs::read_dir(scratch.root()).unwrap().count(), 0);
}

#[test]
fn standalone_requires_explicit_games() {
    let scratch = Scratch::new();
    let missing = scratch.command().output().unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("requires --games"));
}

#[cfg(target_os = "linux")]
#[test]
fn installed_locales_follow_executable_not_cwd_or_games() {
    let installed = Scratch::new();
    fs::create_dir(installed.root().join("locales")).unwrap();
    fs::write(installed.root().join("locales/ru.json"), include_bytes!("../locales/ru.json")).unwrap();
    let data = Scratch::new();
    fs::write(data.games().join("catalogue.json"), br#"{"games":[]}"#).unwrap();
    fs::write(data.root().join("prefs.json"), br#"{"lang":"ru","favourites":[]}"#).unwrap();
    let output = installed.installed_driver().args(["--games".into(), data.games().into_os_string()])
        .screenshot("expected.png").output().unwrap();
    assert!(output.status.success());
    let unrelated = Scratch::new();
    fs::create_dir(unrelated.root().join("locales")).unwrap();
    fs::write(unrelated.root().join("locales/ru.json"), b"invalid dictionary from cwd").unwrap();
    let output = ipc_support::Driver::new(&installed.root().join("igui"), unrelated.root())
        .args(["--games".into(), data.games().into_os_string()])
        .screenshot(installed.root().join("actual.png")).output().unwrap();
    assert!(output.status.success());
    assert_eq!(pixels_without_telemetry(installed.root().join("expected.png")),
        pixels_without_telemetry(installed.root().join("actual.png")));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("invalid dictionary"));
}

#[cfg(target_os = "linux")]
#[test]
fn ambiguous_profiles_warn_once_and_continue_startup_in_normal_and_probe_modes() {
    let scratch = Scratch::new();
    let controls = scratch.root().join("controls");
    fs::create_dir(&controls).unwrap();
    for name in ["first.json", "second.json"] {
        fs::write(controls.join(name), crate::common::control_bytes(crate::common::EVDEV_20BC_5500_FILE)).unwrap();
    }
    for probe in [false, true] {
        let mut command = scratch.command();
        command.arg("--controls").arg(&controls)
            .args(["--games", "absent-games", "--ipc-socket", "absent-socket"])
            .env_remove("DISPLAY");
        if probe {
            command.args(["--input-probe", "--frames", "1"]);
        }

        let output = command.output().unwrap();

        // The missing IPC endpoint stops this test before any display or input access.
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert_eq!(error.matches("для 20bc:5500 найдено 2 профиля").count(), 1, "{error}");
        assert!(error.contains("evdev") && error.contains("этот backend отключён"), "{error}");
        assert!(error.contains("No such file or directory"), "startup did not reach IPC: {error}");
        assert!(!error.contains("no input profiles"), "{error}");
        assert_eq!(fs::read_dir(&controls).unwrap().count(), 2);
    }
}
