
use std::{
    fs,
};

use igui::storage::{load_catalogue, load_locales, load_preview, Preferences};
use serde_json::{json, Value};

use crate::common::Scratch;
impl Scratch {
    fn prefs(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root().join("prefs.json")).unwrap()).unwrap()
    }
}

#[test]
fn missing_preferences_are_not_written_until_an_explicit_toggle() {
    let dir = Scratch::new();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert!(prefs.favourites().is_empty());
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 0);
    prefs
        .save_favourite(&crate::common::hash("demo.md"), true)
        .unwrap();
    assert_eq!(dir.prefs(), json!({"favourites":[crate::common::hash("demo.md")]}));
    let mut reloaded = Preferences::load(dir.root()).unwrap();
    assert!(reloaded.favourites().contains(&crate::common::hash("demo.md")));
    reloaded
        .save_favourite(&crate::common::hash("demo.md"), false)
        .unwrap();
    assert_eq!(dir.prefs(), json!({"favourites":[]}));
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 1);
}

#[test]
fn language_preferences_are_read_only_and_survive_explicit_favourite_saves() {
    let dir = Scratch::new();
    assert_eq!(Preferences::load(dir.root()).unwrap().language(), "en");
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 0);
    for language in [
        json!("ru"),
        json!("en"),
        json!("zz"),
        json!("../ru"),
        json!("RU"),
        json!(""),
        json!(null),
        json!(42),
        json!([]),
        json!({}),
    ] {
        let original = json!({"lang":language,"favourites":[]}).to_string();
        fs::write(dir.root().join("prefs.json"), &original).unwrap();

        let mut prefs = Preferences::load(dir.root()).unwrap();
        let expected = language
            .as_str()
            .filter(|code| igui::locales::valid_code(code))
            .unwrap_or("en");
        assert_eq!(prefs.language(), expected);
        assert_eq!(
            fs::read_to_string(dir.root().join("prefs.json")).unwrap(),
            original
        );
        prefs
            .save_favourite(&crate::common::hash("demo.gg"), true)
            .unwrap();

        assert_eq!(
            dir.prefs(),
            json!({"lang":language,"favourites":[crate::common::hash("demo.gg")]})
        );
    }
}

#[test]
fn only_the_requested_locale_is_loaded_without_discovery_or_a_language_limit() {
    let dir = Scratch::new();
    fs::write(dir.root().join("ru.json"), br#"{"Title":"Russian"}"#).unwrap();
    fs::write(dir.root().join("en.json"), br#"{"Title":"Not English"}"#).unwrap();
    for code in [
        "de", "es", "fr", "ja", "uk", "cs", "pl", "nl", "sv", "fi", "pt",
    ] {
        fs::write(dir.root().join(format!("{code}.json")), b"{}").unwrap();
    }
    fs::write(dir.root().join("zz.json"), br#"{"Title":"Selected"}"#).unwrap();
    for (code, title) in [("en", "Title"), ("ru", "Russian"), ("zz", "Selected")] {
        let locales = load_locales(dir.root(), code);
        assert_eq!(locales.language(), code);
        assert_eq!(locales.text("Title"), title);
        assert_eq!(locales.text("No description"), "No description");
    }
}

#[test]
fn unavailable_or_invalid_locales_fall_back_to_source_english() {
    let dir = Scratch::new();
    fs::write(dir.root().join("fr.json"), br#"{"Title":"French"}"#).unwrap();
    for code in ["ru", "", "../fr", "/fr", "FR", "f", "french", "12"] {
        let locales = load_locales(dir.root(), code);
        assert_eq!(locales.language(), "en");
        assert_eq!(locales.text("Title"), "Title");
    }
    for bytes in [
        b"{".as_slice(),
        br#"{"Title":42}"#,
        br#"{"Title":""}"#,
        &[0xff],
        &vec![b' '; 64 * 1024 + 1],
    ] {
        fs::write(dir.root().join("ru.json"), bytes).unwrap();
        assert_eq!(load_locales(dir.root(), "ru").language(), "en");
        assert_eq!(fs::read(dir.root().join("ru.json")).unwrap(), bytes);
    }
    fs::remove_file(dir.root().join("ru.json")).unwrap();
    fs::create_dir(dir.root().join("ru.json")).unwrap();
    assert_eq!(load_locales(dir.root(), "ru").language(), "en");
    assert_eq!(load_locales(&dir.root().join("fr.json"), "ru").language(), "en");
    assert_eq!(load_locales(&dir.root().join("missing"), "ru").language(), "en");
}

#[cfg(unix)]
#[test]
fn symlinked_locale_is_not_loaded() {
    let dir = Scratch::new();
    fs::write(dir.root().join("other.json"), br#"{"Title":"Linked"}"#).unwrap();
    std::os::unix::fs::symlink("other.json", dir.root().join("ru.json")).unwrap();
    assert_eq!(load_locales(dir.root(), "ru").language(), "en");
}

#[test]
fn saves_preserve_other_preferences_and_favourites_absent_from_catalogue() {
    let dir = Scratch::new();
    fs::write(
        dir.root().join("prefs.json"),
        br#"{"favourites":["676f6e652e6e6573000000000000000000000000000000000000000000000000","676f6e652e6e6573000000000000000000000000000000000000000000000000"],"audio":{"volume":42}}"#,
    )
    .unwrap();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    prefs.save_favourite(&crate::common::hash("new.gg"), true).unwrap();
    assert_eq!(
        dir.prefs(),
        json!({"favourites":[crate::common::hash("gone.nes"),crate::common::hash("new.gg")],"audio":{"volume":42}})
    );
}

#[test]
fn invalid_preferences_and_external_changes_are_not_overwritten() {
    let dir = Scratch::new();
    for bytes in [
        b"{".as_slice(),
        br#"{"favourites":[4]}"#,
        br#"{"favourites":["../x"]}"#,
        br#"{}"#,
    ] {
        fs::write(dir.root().join("prefs.json"), bytes).unwrap();
        assert!(Preferences::load(dir.root()).is_err());
        assert_eq!(fs::read(dir.root().join("prefs.json")).unwrap(), bytes);
    }
    fs::write(dir.root().join("prefs.json"), b"{\"favourites\":[]}").unwrap();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    fs::write(dir.root().join("prefs.json"), b"external").unwrap();
    assert!(prefs.save_favourite(&crate::common::hash("a.nes"), true).is_err());
    assert!(prefs.favourites().is_empty());
    assert_eq!(fs::read(dir.root().join("prefs.json")).unwrap(), b"external");
    assert!(!dir.root().join(".prefs.json.tmp").exists());
    assert!(prefs.save_favourite(&crate::common::hash("b.nes"), true).is_err());
}

#[test]
fn refuses_existing_temp_file_and_does_not_remove_it() {
    let dir = Scratch::new();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    fs::write(dir.root().join(".prefs.json.tmp"), b"another writer").unwrap();
    assert!(prefs.save_favourite(&crate::common::hash("a.nes"), true).is_err());
    assert!(prefs.favourites().is_empty());
    assert_eq!(
        fs::read(dir.root().join(".prefs.json.tmp")).unwrap(),
        b"another writer"
    );
    assert!(!dir.root().join("prefs.json").exists());
}

#[test]
fn loading_catalogue_does_not_require_roms_or_thumbs_and_never_scans() {
    let dir = Scratch::new();
    assert!(load_catalogue(dir.root()).is_err());
    fs::write(
        dir.root().join("catalogue.json"),
        br#"{"version":3,"games":[{"platform":"NES","image":"a.nes","title":"A","hash":"612e6e6573000000000000000000000000000000000000000000000000000000"}]}"#,
    )
    .unwrap();
    assert_eq!(load_catalogue(dir.root()).unwrap().len(), 1);
    assert!(load_preview(dir.root(), &crate::common::hash("a.nes")).is_none());
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn refuses_preference_symlinks_and_previews_escaping_thumbs() {
    use std::os::unix::fs::symlink;
    let dir = Scratch::new();
    let outside = Scratch::new();
    fs::write(outside.root().join("prefs.json"), br#"{"favourites":[]}"#).unwrap();
    symlink(outside.root().join("prefs.json"), dir.root().join("prefs.json")).unwrap();
    assert!(Preferences::load(dir.root()).is_err());
    let hash = crate::common::hash("a.nes");
    fs::create_dir(outside.root().join(&hash[..2])).unwrap();
    fs::write(
        outside.root().join(&hash[..2]).join(format!("{hash}.png")),
        b"private",
    )
    .unwrap();
    symlink(outside.root(), dir.root().join("thumbs")).unwrap();
    assert!(load_preview(dir.root(), &crate::common::hash("a.nes")).is_none());
    assert_eq!(
        fs::read(outside.root().join("prefs.json")).unwrap(),
        br#"{"favourites":[]}"#
    );
}

#[test]
fn previews_use_only_hash_filenames_not_rom_names() {
    let dir = Scratch::new();
    let hash = crate::common::hash("image");
    let shard = dir.root().join("thumbs").join(&hash[..2]);
    fs::create_dir_all(&shard).unwrap();
    let path = shard.join(format!("{hash}.png"));
    let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), 1, 1);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[10, 20, 30])
        .unwrap();
    assert!(load_preview(dir.root(), &hash).is_some());
    assert!(load_preview(dir.root(), "image.bin").is_none());
    assert!(load_preview(dir.root(), &crate::common::hash("different")).is_none());
    fs::rename(path, dir.root().join("thumbs/image.bin.png")).unwrap();
    assert!(load_preview(dir.root(), &hash).is_none());
}

#[test]
fn trash_is_optional_on_load_and_explicit_saves_preserve_other_preferences() {
    let dir = Scratch::new();
    let hash = crate::common::hash("game");
    let absent = crate::common::hash("absent");
    let original = json!({"favourites":[hash], "lang":"ru", "audio":{"volume":42}}).to_string();
    fs::write(dir.root().join("prefs.json"), &original).unwrap();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert!(prefs.trash().is_empty());
    prefs.save_trash(&hash, false).unwrap();
    assert_eq!(
        fs::read_to_string(dir.root().join("prefs.json")).unwrap(),
        original
    );
    prefs.save_trash(&absent, true).unwrap();
    prefs.save_trash(&hash, true).unwrap();
    prefs.save_trash(&hash, true).unwrap(); // no duplicates
    assert_eq!(dir.prefs()["trash"], json!([hash, absent]));
    assert_eq!(dir.prefs()["favourites"], json!([hash]));
    assert_eq!(dir.prefs()["audio"], json!({"volume":42}));
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert_eq!(prefs.trash(), &[hash.clone(), absent.clone()]);
    prefs
        .save_favourite(&crate::common::hash("another"), true)
        .unwrap();
    assert_eq!(dir.prefs()["trash"], json!([hash, absent]));
    prefs.save_trash(&hash, false).unwrap();
    assert_eq!(dir.prefs()["trash"], json!([absent]));
    assert!(prefs.favourites().contains(&hash));
    assert_eq!(dir.prefs()["lang"], "ru");
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 1);
}

#[test]
fn first_trash_save_creates_preferences_without_any_other_files() {
    let dir = Scratch::new();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert!(prefs.trash().is_empty());
    assert!(!dir.root().join("prefs.json").exists());
    let hash = crate::common::hash("game");
    prefs.save_trash(&hash, true).unwrap();
    assert_eq!(dir.prefs(), json!({"favourites":[], "trash":[hash]}));
    assert_eq!(Preferences::load(dir.root()).unwrap().trash(), &[hash]);
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 1);
}

#[test]
fn trash_load_keeps_array_order_and_deduplicates_without_rewriting() {
    let dir = Scratch::new();
    let (first_hash, second_hash, third_hash, absent) = (crate::common::hash("a"), crate::common::hash("b"), crate::common::hash("c"), crate::common::hash("absent"));
    let original = json!({"favourites":[], "trash":[second_hash, absent, first_hash, second_hash, third_hash, first_hash]}).to_string();
    fs::write(dir.root().join("prefs.json"), &original).unwrap();

    let mut prefs = Preferences::load(dir.root()).unwrap();

    assert_eq!(prefs.trash(), &[second_hash.clone(), absent.clone(), first_hash.clone(), third_hash.clone()]);
    assert_eq!(fs::read_to_string(dir.root().join("prefs.json")).unwrap(), original);
    fs::write(dir.root().join(".prefs.json.tmp"), b"other writer").unwrap();
    prefs.save_trash(&first_hash, true).unwrap(); // No duplicate and no promotion of an already trashed game.
    prefs.save_trash(&crate::common::hash("missing"), false).unwrap();
    assert_eq!(fs::read_to_string(dir.root().join("prefs.json")).unwrap(), original);
    assert_eq!(fs::read(dir.root().join(".prefs.json.tmp")).unwrap(), b"other writer");
    fs::remove_file(dir.root().join(".prefs.json.tmp")).unwrap();
    prefs.save_trash(&third_hash, false).unwrap();
    assert_eq!(dir.prefs()["trash"], json!([second_hash, absent, first_hash]));
    prefs.save_trash(&third_hash, true).unwrap(); // A restored game deleted again is now the newest.
    assert_eq!(prefs.trash(), &[third_hash.clone(), second_hash.clone(), absent.clone(), first_hash.clone()]);
    assert_eq!(dir.prefs()["trash"], json!([third_hash, second_hash, absent, first_hash]));
    prefs.save_favourite(&first_hash, true).unwrap();
    prefs.save_current_item(&second_hash).unwrap();
    assert_eq!(dir.prefs()["trash"], json!([third_hash, second_hash, absent, first_hash]));
    assert_eq!(Preferences::load(dir.root()).unwrap().trash(), prefs.trash());
}

#[test]
fn failed_deletion_or_restoration_does_not_change_confirmed_trash_order() {
    for enabled in [false, true] {
        let dir = Scratch::new();
        let first_hash = crate::common::hash("a");
        let second_hash = crate::common::hash("b");
        let original = json!({"favourites":[], "trash":[second_hash, first_hash]}).to_string();
        fs::write(dir.root().join("prefs.json"), &original).unwrap();
        let mut prefs = Preferences::load(dir.root()).unwrap();
        fs::write(dir.root().join(".prefs.json.tmp"), b"other writer").unwrap();
        let changed = if enabled { crate::common::hash("new") } else { second_hash.clone() };

        assert!(prefs.save_trash(&changed, enabled).is_err());

        assert_eq!(prefs.trash(), &[second_hash, first_hash]);
        assert_eq!(fs::read_to_string(dir.root().join("prefs.json")).unwrap(), original);
        assert!(prefs.save_trash(&changed, enabled).is_err());
    }
}

#[test]
fn malformed_trash_is_rejected_without_overwriting_preferences() {
    let dir = Scratch::new();
    for trash in [
        json!(null),
        json!({}),
        json!("game"),
        json!([1]),
        json!([null]),
        json!(["game.bin"]),
        json!(["A".repeat(64)]),
        json!(["a".repeat(63)]),
    ] {
        let original = json!({"favourites":[], "trash":trash}).to_string();
        fs::write(dir.root().join("prefs.json"), &original).unwrap();
        assert!(Preferences::load(dir.root()).is_err());
        assert_eq!(
            fs::read_to_string(dir.root().join("prefs.json")).unwrap(),
            original
        );
    }
    fs::remove_file(dir.root().join("prefs.json")).unwrap();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert!(prefs.save_trash("not-a-hash", true).is_err());
    assert!(!dir.root().join("prefs.json").exists());
}

#[test]
fn failed_trash_save_keeps_state_and_locks_all_further_preference_writes() {
    for external_change in [false, true] {
        let dir = Scratch::new();
        let mut prefs = Preferences::load(dir.root()).unwrap();
        let hash = crate::common::hash("game");
        if external_change {
            fs::write(dir.root().join("prefs.json"), b"external").unwrap();
        } else {
            fs::write(dir.root().join(".prefs.json.tmp"), b"other writer").unwrap();
        }
        assert!(prefs.save_trash(&hash, true).is_err());
        assert!(prefs.trash().is_empty());
        assert!(prefs.favourites().is_empty());
        assert!(prefs.save_trash(&hash, true).is_err());
        assert!(prefs.save_favourite(&hash, true).is_err());
        if external_change {
            assert_eq!(fs::read(dir.root().join("prefs.json")).unwrap(), b"external");
            assert!(!dir.root().join(".prefs.json.tmp").exists());
        } else {
            assert!(!dir.root().join("prefs.json").exists());
            assert_eq!(
                fs::read(dir.root().join(".prefs.json.tmp")).unwrap(),
                b"other writer"
            );
        }
    }
}

#[test]
fn preview_loader_reads_only_shards_without_flat_fallback() {
    let dir = Scratch::new();
    let hash = crate::common::hash("sharded");
    let shard = dir.root().join("thumbs").join(&hash[..2]);
    fs::create_dir_all(&shard).unwrap();
    fs::write(shard.join(format!("{hash}.png")), b"sharded").unwrap();
    fs::write(dir.root().join("thumbs").join(format!("{hash}.png")), b"flat").unwrap();
    assert_eq!(load_preview(dir.root(), &hash).unwrap(), b"sharded");
    fs::remove_file(shard.join(format!("{hash}.png"))).unwrap();
    assert!(load_preview(dir.root(), &hash).is_none());
}

#[test]
fn current_item_save_preserves_other_fields_and_does_not_rewrite_the_same_hash() {
    let dir = Scratch::new();
    let hash = crate::common::hash("selected");
    let original = json!({"favourites":[], "trash":[crate::common::hash("missing")], "lang":"ru", "other":42});
    fs::write(dir.root().join("prefs.json"), original.to_string()).unwrap();
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert_eq!(prefs.current_item(), None);

    prefs.save_current_item(&hash).unwrap();

    let mut expected = original;
    expected["current_item"] = json!(hash);
    assert_eq!(dir.prefs(), expected);
    let mut prefs = Preferences::load(dir.root()).unwrap();
    assert_eq!(prefs.current_item(), Some(hash.as_str()));
    // Any actual write would collide with this file, even on a writable FS.
    fs::write(dir.root().join(".prefs.json.tmp"), b"other writer").unwrap();
    prefs.save_current_item(&hash).unwrap();
    assert_eq!(fs::read(dir.root().join(".prefs.json.tmp")).unwrap(), b"other writer");
    fs::remove_file(dir.root().join(".prefs.json.tmp")).unwrap();
    prefs.save_favourite(&hash, true).unwrap();
    prefs.save_trash(&hash, true).unwrap();
    assert_eq!(dir.prefs()["current_item"], hash);
}

#[test]
fn current_item_is_optional_and_malformed_values_do_not_trigger_repairs() {
    let dir = Scratch::new();
    for value in [json!(null), json!(42), json!([]), json!("invalid"), json!("A".repeat(64))] {
        let original = json!({"favourites":[], "current_item":value}).to_string();
        fs::write(dir.root().join("prefs.json"), &original).unwrap();

        let mut prefs = Preferences::load(dir.root()).unwrap();

        assert_eq!(prefs.current_item(), None);
        assert_eq!(fs::read_to_string(dir.root().join("prefs.json")).unwrap(), original);
        assert!(prefs.save_current_item("invalid").is_err());
        assert_eq!(fs::read_to_string(dir.root().join("prefs.json")).unwrap(), original);
    }
}

#[test]
fn failed_current_item_save_keeps_memory_and_locks_further_writes() {
    for external_change in [false, true] {
        let dir = Scratch::new();
        let mut prefs = Preferences::load(dir.root()).unwrap();
        if external_change {
            fs::write(dir.root().join("prefs.json"), b"external").unwrap();
        } else {
            fs::write(dir.root().join(".prefs.json.tmp"), b"other writer").unwrap();
        }
        let hash = crate::common::hash("selected");

        assert!(prefs.save_current_item(&hash).is_err());

        assert_eq!(prefs.current_item(), None);
        assert!(prefs.save_current_item(&hash).is_err());
        assert!(prefs.save_favourite(&hash, true).is_err());
        assert!(prefs.save_trash(&hash, true).is_err());
        if external_change {
            assert_eq!(fs::read(dir.root().join("prefs.json")).unwrap(), b"external");
        } else {
            assert!(!dir.root().join("prefs.json").exists());
            assert_eq!(fs::read(dir.root().join(".prefs.json.tmp")).unwrap(), b"other writer");
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn space_is_read_from_the_requested_filesystem_and_errors_are_unknown() {
    use igui::storage::available_bytes;
    use std::path::Path;
    assert!(available_bytes(Path::new(env!("CARGO_MANIFEST_DIR"))).is_some());
    assert_eq!(available_bytes(Path::new("/not-a-real-igui-path\0")), None);
    assert_eq!(
        available_bytes(Path::new(file!()).join("not-a-directory").as_path()),
        None
    );
}
