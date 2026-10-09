// SPDX-License-Identifier: BSD-3-Clause

use igui::render::notice;

#[test]
fn launch_failure_shows_localized_reason_and_complete_bios_path_without_terminal_advice() {
    let message = notice::launch_failed("launch_refused: Required BIOS missing: system/bios/FDS/disksys.rom");

    let english = notice::lines(&crate::common::locales("en"), &message, 76);
    let russian = notice::lines(&crate::common::locales("ru"), &message, 76);

    assert_eq!(english, ["Cannot launch game.", "Required BIOS missing: system/bios/FDS/disksys.rom"]);
    assert_eq!(russian, ["Не удалось запустить игру.", "Не найден обязательный BIOS: system/bios/FDS/disksys.rom"]);
    assert!(!message.contains("terminal"));
    assert!(!message.contains("launch_refused"));
}

#[test]
fn failed_emulator_status_and_advice_are_visible_on_the_device() {
    let message = "Game session failed.\nEmulator status: exit status: 7\nCheck ROM, core and BIOS.";

    let lines = notice::lines(&crate::common::locales("ru"), message, 76);

    assert_eq!(lines, ["Ошибка игрового сеанса.", "Статус эмулятора: exit status: 7", "Проверьте ROM, ядро и BIOS."]);
}

#[test]
fn notice_details_are_wrapped_bounded_and_control_characters_are_not_rendered() {
    let message = notice::launch_failed(&format!("no core for platform FDS\n\u{1b}[0m{}", "x".repeat(5000)));

    let lines = notice::lines(&crate::common::locales("ru"), &message, 76);

    assert!(lines.contains(&"Нет ядра для платформы: FDS".to_string()));
    assert!(lines.iter().all(|line| line.chars().count() <= 76));
    assert!(lines.iter().flat_map(|line| line.chars()).all(|c| !c.is_control()));
    assert!(lines.iter().map(|s| s.chars().count()).sum::<usize>() < 2100);
}

#[test]
fn missing_file_has_generic_localized_advice_for_ipc_and_returned_launch_failures() {
    let raw = "No such file or directory (os error 2)";
    for message in [
        notice::launch_failed(raw),
        notice::launch_failed(&format!("launch_refused: {raw}")),
        format!("Cannot launch game.\n{raw}"),
    ] {
        let english = notice::lines(&crate::common::locales("en"), &message, 76);
        let russian = notice::lines(&crate::common::locales("ru"), &message, 76);

        assert_eq!(english, ["Cannot launch game.", "A required file was not found.",
            "Check the ROM and emulator installation on the SD card."]);
        assert_eq!(russian, ["Не удалось запустить игру.", "Не найден необходимый файл.",
            "Проверьте ROM и установку эмуляторов на SD."]);
        assert!(message.contains(raw)); // Original detail remains available to diagnostics.
        assert!(english.iter().chain(&russian).all(|line| !line.contains("os error")));
    }
}

#[test]
fn generic_missing_file_message_does_not_replace_other_errors_or_contextual_details() {
    for reason in [
        "Permission denied (os error 13)",
        "Not a directory (os error 20)",
        "unsupported manifest architecture",
        "Cannot read BIOS: system/bios/FDS. No such file or directory (os error 2)",
    ] {
        let message = notice::launch_failed(reason);

        let lines = notice::lines(&crate::common::locales("en"), &message, 120);

        assert_eq!(lines, ["Cannot launch game.", reason]);
    }
    let ordinary = "No such file or directory (os error 2)";
    assert_eq!(notice::lines(&crate::common::locales("en"), ordinary, 76), [ordinary]);
}

#[test]
fn missing_file_advice_preserves_rom_path_for_rpc_and_returned_errors() {
    let path = "/mnt/sdcard/games/images/FDS/ab/Игра ' (J) [!].fds";
    let reason = format!("ROM: {path}\nNo such file or directory (os error 2)");
    for message in [notice::launch_failed(&format!("launch_refused: {reason}")),
        format!("Cannot launch game.\n{reason}")]
    {
        let lines = notice::lines(&crate::common::locales("ru"), &message, 76);

        assert_eq!(lines, ["Не удалось запустить игру.", &format!("ROM: {path}"),
            "Не найден необходимый файл.", "Проверьте ROM и установку эмуляторов на SD."]);
    }
}

#[test]
fn failed_conversion_path_status_and_advice_fit_the_notice_area() {
    let path = "/mnt/sdcard/games/images/NES/fe/Almana no Kiseki (Unl) (FDS Conversion) [U].unf";
    let message = format!("Game session failed.\nROM: {path}\nEmulator status: exit status: 1\nCheck ROM, core and BIOS.");

    let lines = notice::lines(&crate::common::locales("ru"), &message, igui::model::description::COLUMNS);

    assert!(lines.len() <= igui::model::description::ROWS);
    assert!(lines.join(" ").contains(&format!("ROM: {path}")));
    assert!(lines.contains(&"Статус эмулятора: exit status: 1".to_string()));
    assert_eq!(lines.last().unwrap(), "Проверьте ROM, ядро и BIOS.");
}

#[test]
fn rom_context_does_not_hide_other_launch_errors() {
    let reason = "ROM: /mnt/sdcard/games/images/NES/ab/Game.nes\nPermission denied (os error 13)";
    let message = notice::launch_failed(reason);

    let lines = notice::lines(&crate::common::locales("en"), &message, 76);

    assert_eq!(lines, ["Cannot launch game.", "ROM: /mnt/sdcard/games/images/NES/ab/Game.nes",
        "Permission denied (os error 13)"]);
}
