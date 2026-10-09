
use std::time::Duration;

use igui::runtime::launch::LaunchDialog;

#[test]
fn minimum_interval_starts_at_first_presented_frame_not_creation() {
    let mut dialog = LaunchDialog::new("Game".into(), "MD".into());
    assert!(!dialog.minimum_elapsed(Duration::from_secs(30)));
    dialog.presented(Duration::from_secs(30));
    assert!(!dialog.minimum_elapsed(Duration::from_millis(30_999)));
    assert!(dialog.minimum_elapsed(Duration::from_secs(31)));
    assert!(dialog.minimum_elapsed(Duration::from_secs(100)));
}

#[test]
fn repeated_presentation_does_not_restart_the_timer() {
    let mut dialog = LaunchDialog::new("Game".into(), "MD".into());
    dialog.presented(Duration::from_secs(3));
    dialog.presented(Duration::from_millis(3_900));
    assert!(dialog.minimum_elapsed(Duration::from_secs(4)));
    assert!(!dialog.minimum_elapsed(Duration::from_secs(2)));
}

#[test]
fn message_localizes_template_but_preserves_the_game_title() {
    let dialog = LaunchDialog::new("Ёжик / The Game".into(), "MD".into());
    assert_eq!(
        dialog.message(&crate::common::locales("en")),
        "Loading Ёжик / The Game ..."
    );
    let locales = crate::common::locales("ru");
    assert_eq!(dialog.message(&locales), "Загружаю Ёжик / The Game ...");
}

#[test]
fn dialog_expands_its_platform_alias_and_preserves_unknown_aliases() {
    for (alias, name) in [
        ("NES", "Nintendo Entertainment System"),
        ("MD", "Sega Mega Drive / Genesis"),
        ("SNES", "Super Nintendo Entertainment System"),
        ("CUSTOM", "CUSTOM"),
    ] {
        let dialog = LaunchDialog::new("Game".into(), alias.into());
        assert_eq!(dialog.platform_name(), name);
    }
}
