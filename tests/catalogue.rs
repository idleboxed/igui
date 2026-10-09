
use igui::catalogue::{parse, valid_image_name};
use serde_json::json;

#[test]
fn sample_games_have_distinct_descriptions_with_short_and_scrollable_examples() {
    use igui::model::description;
    use crate::common::sample_catalogue;
    use std::collections::BTreeSet;

    let games = sample_catalogue();
    let descriptions: BTreeSet<_> = games
        .iter()
        .map(|game| {
            let text = game.description.as_deref().expect("demo description");
            assert!(!text.trim().is_empty());
            text
        })
        .collect();
    assert_eq!(descriptions.len(), games.len());
    let lengths: Vec<_> = descriptions
        .iter()
        .map(|text| description::wrap(text, description::COLUMNS).len())
        .collect();
    assert!(lengths.iter().any(|&count| count <= description::ROWS));
    assert!(lengths.iter().any(|&count| count > description::ROWS));
    assert!(descriptions.iter().any(|text| text.contains("\n\n")));
}

#[test]
fn reads_metadata_and_does_not_invent_unknown_values() {
    let data = json!({"version": 3, "games": [{
        "hash": crate::common::hash("aladdin.gg"), "platform": "GG", "image": "aladdin.gg", "title": "Aladdin",
        "year": null, "players": null, "genre": null
    }]});
    let games = parse(data.to_string().as_bytes()).unwrap();
    let game = &games[0];
    assert_eq!(game.image, "aladdin.gg");
    assert_eq!(game.title, "Aladdin");
    assert_eq!(game.platform, "GG");
    assert_eq!(game.year, None);
    assert_eq!(game.players, None);
    assert_eq!(game.genre, None);
    assert_eq!(game.region, None);
}

#[test]
fn optional_region_supports_multiple_markets_without_changing_content_identity() {
    let mut data = json!({"version": 3, "games": [{
        "hash": crate::common::hash("aladdin"), "platform": "MD",
        "image": "Aladdin (USA).bin", "title": "Aladdin"
    }]});
    let original = parse(data.to_string().as_bytes()).unwrap().remove(0);
    // A filename tag is not a trusted source of metadata in the GUI.
    assert_eq!(original.region, None);
    for region in [
        None,
        Some(vec![]),
        Some(vec!["Europe".to_owned()]),
        Some(vec![
            "Japan".to_owned(),
            "USA".to_owned(),
            "Europe".to_owned(),
        ]),
        Some(vec!["Custom market".to_owned()]),
    ] {
        data["games"][0]["region"] = json!(region);
        let game = parse(data.to_string().as_bytes()).unwrap().remove(0);
        assert_eq!(game.region, region);
        assert_eq!(game.hash, original.hash);
        assert_eq!(game.preview, original.preview);
    }
    let mut second = data["games"][0].clone();
    second["image"] = json!("Aladdin (Japan).bin");
    second["region"] = json!(["Japan"]);
    data["games"].as_array_mut().unwrap().push(second);
    // Region is metadata, not an extra component of the uniqueness key.
    assert!(parse(data.to_string().as_bytes()).is_err());
    data["games"][1]["hash"] = json!(crate::common::hash("other ROM bytes"));
    assert_eq!(parse(data.to_string().as_bytes()).unwrap().len(), 2);
}

#[test]
fn region_rejects_wrong_types_empty_components_and_unbounded_text() {
    let mut data = json!({"version": 3, "games": [{
        "hash": crate::common::hash("demo"), "platform": "MD", "image": "demo.bin", "title": "Demo"
    }]});
    for region in [
        json!(42),
        json!({}),
        json!("Europe"),
        json!("Japan, USA"),
        json!([null]),
        json!([42]),
        json!([[]]),
        json!([""]),
        json!(["  "]),
        json!(["USA\n"]),
        json!(["Japan\u{0000}"]),
        json!(["Japan, USA"]),
        json!(["?"]),
        json!(["-"]),
        json!(["World"]),
        json!(["USA", "world"]),
        json!(["x".repeat(129)]),
        json!(["é".repeat(65)]),
        json!(["x".repeat(65), "y".repeat(64)]),
    ] {
        data["games"][0]["region"] = region;
        assert!(parse(data.to_string().as_bytes()).is_err(), "{data}");
    }
    data["games"][0]["region"] = json!(["x".repeat(128)]);
    assert!(parse(data.to_string().as_bytes()).is_ok());
    data["games"][0]["region"] = json!(["x".repeat(64), "y".repeat(64)]);
    assert!(parse(data.to_string().as_bytes()).is_ok());
}

#[test]
fn optional_description_accepts_paragraphs_but_rejects_control_codes() {
    let mut data = json!({"version":3,"games":[{"hash":crate::common::hash("demo.gg"),"platform":"GG","image":"demo.gg","title":"Demo",
        "genre":"Platformer", "description":"First paragraph.\r\n\r\nSecond."}]});
    let games = parse(data.to_string().as_bytes()).unwrap();
    assert_eq!(
        games[0].description.as_deref(),
        Some("First paragraph.\n\nSecond.")
    );
    data["games"][0]["description"] = json!("bad\u{001b}");
    assert!(parse(data.to_string().as_bytes()).is_err());
    data["games"][0]["description"] = json!("x".repeat(16385));
    assert!(parse(data.to_string().as_bytes()).is_err());
}

#[test]
fn rejects_ambiguous_filenames_and_bad_fields() {
    let entry = json!({"hash":crate::common::hash("demo.md"),"platform":"MD", "image":"demo.md", "title":"Demo",
        "year":1991, "players":2, "genre":"Платформер"});
    for (field, bad) in [
        ("image", json!("../escape.md")),
        ("image", json!("/escape.md")),
        ("image", json!("MD\\demo.md")),
        ("title", json!("")),
        ("title", json!("bad\nname")),
        ("platform", json!("sega mega drive")),
        ("players", json!(0)),
        ("players", json!(-1)),
        ("players", json!(1.5)),
        ("year", json!(0)),
        ("genre", json!("")),
    ] {
        let mut wrong = entry.clone();
        wrong[field] = bad;
        assert!(
            parse(json!({"version":3,"games":[wrong]}).to_string().as_bytes()).is_err(),
            "{field}"
        );
    }
    let mut duplicate = entry.clone();
    duplicate["image"] = json!("DEMO.MD");
    duplicate["hash"] = json!(crate::common::hash("other"));
    assert!(parse(
        json!({"version":3,"games":[entry.clone(),duplicate]})
            .to_string()
            .as_bytes()
    )
    .is_err());
    for name in ["", ".", "..", "x.md ", "x.md.", "x\0.md", "x:y.md"] {
        assert!(!valid_image_name(name));
    }
    assert!(valid_image_name("Игра (USA).gg"));
}

#[test]
fn hash_is_required_canonical_unique_and_independent_of_filename() {
    use igui::{catalogue::valid_hash, model::Preview};
    let entry = json!({"hash":crate::common::hash("a"),"platform":"MD","image":"demo.bin","title":"Demo"});
    let mut data = json!({"version":3,"games":[entry.clone()]});
    let first = parse(data.to_string().as_bytes()).unwrap();
    assert_eq!(first[0].hash, crate::common::hash("a"));
    assert_eq!(first[0].preview, Preview::Hash(crate::common::hash("a")));
    data["games"][0]["image"] = json!("renamed.bin");
    assert_eq!(
        parse(data.to_string().as_bytes()).unwrap()[0].hash,
        first[0].hash
    );
    for invalid in [
        "".to_owned(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        "é".repeat(32),
    ] {
        assert!(!valid_hash(&invalid));
        data["games"][0]["hash"] = json!(invalid);
        assert!(parse(data.to_string().as_bytes()).is_err());
    }
    data["games"][0].as_object_mut().unwrap().remove("hash");
    assert!(parse(data.to_string().as_bytes()).is_err());
    let mut duplicate = entry.clone();
    duplicate["image"] = json!("different.bin");
    assert!(parse(
        json!({"version":3,"games":[entry.clone(),duplicate]})
            .to_string()
            .as_bytes()
    )
    .is_err());
}

#[test]
fn accepts_safe_relative_paths() {
    use igui::catalogue::valid_image_path;
    let mut entry = json!({"hash": crate::common::hash("nested"), "platform": "NES",
        "image": "NES/ab/Game.nes", "title": "Game"});
    assert!(parse(
        json!({"version":3,"games":[entry.clone()]})
            .to_string()
            .as_bytes()
    )
    .is_ok());
    for path in [
        "/a.nes",
        "../a.nes",
        "NES/../a.nes",
        "NES//a.nes",
        "NES/./a.nes",
        "NES/a.nes/",
        "C:/a.nes",
        "NES\\a.nes",
        "NES/ab /a.nes",
    ] {
        assert!(!valid_image_path(path), "{path}");
        entry["image"] = json!(path);
        assert!(parse(
            json!({"version":3,"games":[entry.clone()]})
                .to_string()
                .as_bytes()
        )
        .is_err());
    }
    assert!(!valid_image_path(&format!(
        "{}/{}.nes",
        "a".repeat(255),
        "b".repeat(255)
    )));
}

#[test]
fn ignores_unknown_fields_but_requires_games_and_entry_identity() {
    let entry = json!({
        "hash": crate::common::hash("demo"), "platform": "MD", "image": "demo.md", "title": "Demo",
        "future": {"value": 1}
    });
    assert!(parse(
        json!({"version": 999, "future": true, "games": [entry.clone()]})
            .to_string()
            .as_bytes()
    )
    .is_ok());
    assert!(parse(json!({"future": true}).to_string().as_bytes())
        .unwrap_err()
        .contains("missing field `games`"));
    for field in ["hash", "platform", "image", "title"] {
        let mut incomplete = entry.clone();
        incomplete.as_object_mut().unwrap().remove(field);
        assert!(parse(json!({"games": [incomplete]}).to_string().as_bytes()).is_err());
    }
}

#[test]
fn sample_catalogue_is_ordinary_filesystem_data() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/_fixtures/games");

    let bytes = std::fs::read(root.join(igui::paths::CATALOGUE_FILE)).unwrap();
    let games = igui::storage::load_catalogue(&root).unwrap();

    assert_eq!(igui::catalogue::parse(&bytes).unwrap(), games);
    assert!(!games.is_empty());
    assert_eq!(root.file_name().unwrap(), igui::paths::GAMES_DIR);
}
