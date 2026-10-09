use igui::locales::{Locale, Locales};

#[test]
fn english_is_builtin_and_missing_translations_fall_back_to_source() {
    let english = Locales::default();
    assert_eq!(english.language(), "en");
    assert_eq!(english.text("Title"), "Title");
    let ru = Locales::new(Locale::parse("ru", br#"{"Title":"Nazvanie"}"#).unwrap());
    assert_eq!(ru.language(), "ru");
    assert_eq!(ru.text("Title"), "Nazvanie");
    assert_eq!(ru.text("No description"), "No description");
}

#[test]
fn invalid_language_codes_and_dictionaries_are_rejected() {
    for code in ["en", "../ru", "r", "russian", "RU", "12"] {
        assert!(Locale::parse(code, b"{}").is_err());
    }
    for bytes in [
        b"{".as_slice(),
        br#"{"Title":1}"#,
        br#"{"Title":""}"#,
        br#"{"Title":"bad\nline"}"#,
    ] {
        assert!(Locale::parse("ru", bytes).is_err());
    }
}

#[test]
fn bundled_russian_translates_labels_genres_and_loading_text() {
    let locales = Locales::new(Locale::parse("ru", include_bytes!("../locales/ru.json")).unwrap());
    assert_eq!(locales.text("Title"), "Название");
    assert_eq!(locales.text("Players"), "Игроки");
    assert_eq!(locales.text("Disk"), "Диск");
    assert_eq!(locales.text("RAM"), "RAM");
    assert_eq!(locales.text("Racing"), "Гонки");
    assert_eq!(locales.text("Reading catalogue"), "Чтение каталога");
    assert_eq!(locales.text("No image"), "Нет изображения");
    assert_eq!(
        igui::runtime::loading::Stage::Preview.label(),
        "Loading selected image"
    );
    assert_eq!(
        locales.text(igui::runtime::loading::Stage::Preview.label()),
        "Загрузка изображения выбранной игры"
    );
    assert_eq!(locales.text("Image"), "Образ"); // The ROM filename is not artwork.
    assert_eq!(locales.text("Search"), "Поиск");
    assert_eq!(locales.text("Space"), "Пробел");
    assert_eq!(locales.text("Move to trash?"), "Переместить в корзину?");
    assert_eq!(locales.text("A: yes | B: no"), "A: да | B: нет");
}
