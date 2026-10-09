
use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use igui::{
    runtime::loading::{CatalogLoader, Stage},
    runtime::startup::load_session,
};

use crate::common::Scratch;

#[test]
fn session_reports_real_stages_and_language_before_catalogue_without_preview_io() {
    let dir = Scratch::with_games();
    fs::create_dir(dir.root().join("locales")).unwrap();
    fs::write(
        dir.root().join("locales/ru.json"),
        include_bytes!("../../locales/ru.json"),
    )
    .unwrap();
    fs::write(
        dir.root().join("prefs.json"),
        br#"{"lang":"ru","favourites":["6f6e652e6d640000000000000000000000000000000000000000000000000000"]}"#,
    )
    .unwrap();
    fs::write(
        dir.root().join("games/catalogue.json"),
        br#"{"version":3,"games":[{"platform":"MD","image":"one.md","title":"One","hash":"6f6e652e6d640000000000000000000000000000000000000000000000000000"}]}"#,
    )
    .unwrap();
    let mut stages = Vec::new();
    let mut languages = Vec::new();
    let session = load_session(
        &dir.root().join("games"),
        &dir.root().join("locales"),
        |stage, locales| {
            stages.push(stage);
            if let Some(locales) = locales {
                languages.push((stage, locales.language().to_owned()));
            }
        },
    )
    .unwrap();
    assert_eq!(
        stages,
        [
            Stage::Preferences,
            Stage::Locales,
            Stage::Catalogue,
            Stage::Library,
            Stage::Storage
        ]
    );
    assert_eq!(languages, [(Stage::Catalogue, "ru".into())]);
    assert_eq!(session.app.locales().language(), "ru");
    assert!(session.app.is_favourite(&crate::common::hash("one.md")));
    assert_eq!(session.app.letters(), &['O']);
    // Neither images/ nor thumbs/ exists: metadata loading never needs them.
    assert_eq!(fs::read_dir(dir.root()).unwrap().count(), 3);
}

#[test]
fn startup_failure_never_announces_operations_after_the_failed_stage() {
    let dir = Scratch::with_games();
    for (prefs, expected) in [
        (b"broken json".as_slice(), vec![Stage::Preferences]),
        (
            br#"{"favourites":[]}"#.as_slice(),
            vec![Stage::Preferences, Stage::Locales, Stage::Catalogue],
        ),
    ] {
        fs::write(dir.root().join("prefs.json"), prefs).unwrap();
        let mut stages = Vec::new();
        assert!(load_session(
            &dir.root().join("games"),
            &dir.root().join("absent-locales"),
            |stage, _| stages.push(stage)
        )
        .is_err());
        assert_eq!(stages, expected);
        assert_eq!(fs::read(dir.root().join("prefs.json")).unwrap(), prefs);
    }
}

fn finish<T: Send + 'static>(loader: &CatalogLoader<T>) -> Result<T, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(app) = loader.poll()? {
            return Ok(app);
        }
        assert!(Instant::now() < deadline, "catalog worker did not finish");
        thread::yield_now();
    }
}

#[test]
fn cancelling_loading_wakes_the_source_and_reports_cancellation() {
    let (started, started_rx) = mpsc::channel();
    let loader = CatalogLoader::spawn_value(move |cancellation| {
        started.send(()).unwrap();
        cancellation.wait_for(Duration::from_secs(60))
    })
    .unwrap();
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    loader.cancel();
    loader.cancel();

    assert_eq!(finish(&loader).unwrap_err(), "catalog loading cancelled");
}

#[test]
fn dropping_loading_cancels_and_joins_the_worker_before_returning() {
    let (started, started_rx) = mpsc::channel();
    let (finished, finished_rx) = mpsc::channel();
    let loader = CatalogLoader::spawn_value(move |cancellation| {
        started.send(()).unwrap();
        let result = cancellation.wait_for(Duration::from_secs(60));
        finished.send(result).unwrap();
        Ok(())
    })
    .unwrap();
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    drop(loader);

    assert_eq!(finished_rx.try_recv().unwrap().unwrap_err(), "catalog loading cancelled");
}

#[test]
fn metadata_loading_does_not_block_polling_and_publishes_only_the_full_catalog() {
    let (started, started_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let loader = CatalogLoader::spawn(move |_| {
        started.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        Ok(vec![
            crate::common::game("two", "Zulu", "OTHER"),
            crate::common::game("one", "Alpha", "CUSTOM"),
        ])
    })
    .unwrap();
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(loader.poll().unwrap().is_none());
    release.send(()).unwrap();
    let app = finish(&loader).unwrap();
    assert_eq!(app.total_count(), 2);
    assert_eq!(app.selected_game().unwrap().image, "one");
    assert_eq!(app.letters(), &['A', 'Z']);
    assert_eq!(app.platforms(), &["CUSTOM", "OTHER"]);
}

#[test]
fn empty_catalog_finishes_instead_of_leaving_the_splash_running() {
    let loader = CatalogLoader::spawn(|_| Ok(vec![])).unwrap();
    let app = finish(&loader).unwrap();
    assert_eq!(app.total_count(), 0);
    assert!(app.letters().is_empty());
    assert!(app.platforms().is_empty());
}

#[test]
fn loading_errors_are_reported_instead_of_an_endless_splash() {
    let loader = CatalogLoader::spawn(|_| Err("test source unavailable".into())).unwrap();
    assert!(matches!(finish(&loader), Err(message) if message == "test source unavailable"));
}

#[test]
fn preferences_are_saved_beside_games() {
    let dir = Scratch::with_games();
    let games = dir.root().join("games");
    fs::write(games.join("catalogue.json"), br#"{"version":3,"games":[]}"#).unwrap();
    let mut session = load_session(&games, &dir.root().join("absent-locales"), |_, _| {}).unwrap();
    assert!(!dir.root().join("prefs.json").exists());
    session
        .preferences
        .save_favourite(&crate::common::hash("a.nes"), true)
        .unwrap();
    assert!(dir.root().join("prefs.json").exists());
    assert!(!games.join("prefs.json").exists());
}
