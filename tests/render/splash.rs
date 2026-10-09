
use std::time::Duration;

use embedded_graphics::{pixelcolor::Rgb888, prelude::*, primitives::Rectangle};

use {
    igui::runtime::loading::Stage,
    igui::render::HEIGHT,
    igui::render::WIDTH,
    igui::render::splash,
    imanlib::surface::Surface,
};

#[test]
fn every_startup_stage_has_a_distinct_english_and_russian_caption() {
    let stages = [
        Stage::Starting,
        Stage::Gamepad,
        Stage::Preferences,
        Stage::Locales,
        Stage::Catalogue,
        Stage::Library,
        Stage::Storage,
        Stage::Preview,
        Stage::Interface,
    ];
    for language in ["en", "ru"] {
        let locales = crate::common::locales(language);
        let mut captions = Vec::new();
        for stage in stages {
            if language == "ru" {
                assert_ne!(locales.text(stage.label()), stage.label());
            }
            let mut surface = Surface::new(WIDTH, HEIGHT);
            splash::draw_stage(Duration::ZERO, stage, &locales, &mut surface).unwrap();
            let caption = surface.pixels()[423 * WIDTH as usize..445 * WIDTH as usize].to_vec();
            assert!(!captions.contains(&caption));
            captions.push(caption);
        }
    }
}

fn frame(ms: u64) -> Vec<u32> {
    let mut surface = Surface::new(WIDTH, HEIGHT);
    splash::draw(Duration::from_millis(ms), &mut surface).unwrap();
    surface.pixels().to_vec()
}

#[test]
fn saved_language_translates_loading_label_without_changing_logo() {
    let english = frame(0);
    let locales = crate::common::locales("ru");
    let mut surface = Surface::new(WIDTH, HEIGHT);
    splash::draw_localized(Duration::ZERO, &locales, &mut surface).unwrap();
    let changed: Vec<_> = english
        .iter()
        .zip(surface.pixels())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert!(!changed.is_empty());
    for i in changed {
        assert!((423..445).contains(&(i / WIDTH as usize)));
    }
}

#[test]
fn logo_blinks_at_half_a_hertz_starting_visible() {
    for (ms, visible) in [
        (0, true),
        (999, true),
        (1000, false),
        (1999, false),
        (2000, true),
        (2999, true),
        (3000, false),
    ] {
        assert_eq!(splash::logo_visible(Duration::from_millis(ms)), visible);
    }
}

#[test]
fn only_the_centered_logo_blinks_not_the_background_or_loading_label() {
    let on = frame(0);
    let off = frame(1000);
    assert_eq!(on, frame(999));
    assert_eq!(on, frame(2000));
    assert_eq!(off, frame(1999));
    let mut changed = 0;
    for (index, (&a, &b)) in on.iter().zip(&off).enumerate() {
        if a != b {
            let (x, y) = (index % WIDTH as usize, index / WIDTH as usize);
            assert!((559..720).contains(&x));
            assert!((319..400).contains(&y));
            changed += 1;
        }
    }
    assert!(changed > 0);
    // The stable loading label remains visible even during the off phase.
    assert!(off.iter().any(|&pixel| pixel != off[0]));
}

#[test]
fn stage_captions_end_with_a_space_and_three_dots_in_both_languages() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        pixelcolor::Rgb888,
        prelude::*,
        text::{Baseline, Text},
    };
    for language in ["en", "ru"] {
        let locales = crate::common::locales(language);
        for stage in [
            Stage::Starting,
            Stage::Gamepad,
            Stage::Preferences,
            Stage::Locales,
            Stage::Catalogue,
            Stage::Library,
            Stage::Storage,
            Stage::Preview,
            Stage::Interface,
        ] {
            let mut actual = Surface::new(WIDTH, HEIGHT);
            splash::draw_stage(Duration::from_secs(1), stage, &locales, &mut actual).unwrap();
            let mut expected = Surface::new(WIDTH, HEIGHT);
            expected.clear(Rgb888::new(15, 21, 29)).unwrap();
            let label = format!("{} ...", locales.text(stage.label()));
            Text::with_baseline(
                &label,
                expected.bounding_box().center()
                    + Point::new(-(label.chars().count() as i32 * 10) / 2, 64),
                MonoTextStyle::new(&FONT_10X20, Rgb888::new(151, 169, 174)),
                Baseline::Top,
            )
            .draw(&mut expected)
            .unwrap();
            assert_eq!(actual.pixels(), expected.pixels());
        }
    }
}

#[test]
fn initial_scanout_is_the_ready_starting_splash_not_a_black_placeholder() {
    let first = splash::initial_frame();

    assert_eq!((first.width(), first.height()), (WIDTH, HEIGHT));
    assert_eq!(first.pixels(), frame(0));
    assert!(first.pixels().iter().any(|&pixel| pixel != first.pixels()[0]));
    assert_ne!(first.pixels()[0], 0);
}

#[test]
fn retained_splash_matches_full_frames_through_stages_languages_and_blinks() {
    let stages = [Stage::Starting, Stage::Gamepad, Stage::Preferences, Stage::Locales,
        Stage::Catalogue, Stage::Library, Stage::Storage, Stage::Preview, Stage::Interface];
    for (width, height) in [(WIDTH, HEIGHT), (321, 241)] {
        let mut retained = splash::RetainedSplash::default();
        let mut actual = Surface::new(width, height);
        let mut expected = Surface::new(width, height);
        for language in ["en", "ru", "en"] {
            let locales = crate::common::locales(language);
            for stage in stages {
                for ms in [0, 999, 1000, 1999, 2000] {
                    let elapsed = Duration::from_millis(ms);
                    retained.update(elapsed, stage, &locales, &mut actual).unwrap();
                    splash::draw_stage(elapsed, stage, &locales, &mut expected).unwrap();

                    assert_eq!(actual.pixels(), expected.pixels(), "{language} {stage:?} {ms}");
                }
            }
        }
    }
}

#[test]
fn changed_dictionary_and_shorter_caption_remove_all_old_text() {
    use igui::locales::{Locale, Locales};
    let mut retained = splash::RetainedSplash::default();
    let mut actual = Surface::new(WIDTH, HEIGHT);
    let mut expected = Surface::new(WIDTH, HEIGHT);
    for text in ["Длинная надпись ".repeat(20), "Я".into(), "Обновлённый перевод".into()] {
        let dictionary = serde_json::json!({"Starting IGUI": text});
        let locales = Locales::new(Locale::parse("ru", &serde_json::to_vec(&dictionary).unwrap()).unwrap());

        assert!(retained.update(Duration::ZERO, Stage::Starting, &locales, &mut actual).unwrap());
        splash::draw_stage(Duration::ZERO, Stage::Starting, &locales, &mut expected).unwrap();

        assert_eq!(actual.pixels(), expected.pixels());
    }
}

struct RecordedTarget {
    surface: Surface,
    full_clears: usize,
    writes: usize,
    allowed: Option<Rectangle>,
    outside: bool,
    fail_after: Option<usize>,
}

impl RecordedTarget {
    fn new() -> Self {
        Self { surface: Surface::new(WIDTH, HEIGHT), full_clears: 0, writes: 0,
            allowed: None, outside: false, fail_after: None }
    }

    fn observe(&mut self, allowed: Rectangle) {
        self.full_clears = 0;
        self.writes = 0;
        self.allowed = Some(allowed);
        self.outside = false;
    }
}

impl OriginDimensions for RecordedTarget {
    fn size(&self) -> Size {
        self.surface.size()
    }
}

impl DrawTarget for RecordedTarget {
    type Color = Rgb888;
    type Error = &'static str;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb888>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for pixel in pixels {
            if self.fail_after == Some(0) {
                self.fail_after = None;
                return Err("synthetic draw failure");
            }
            if let Some(remaining) = &mut self.fail_after {
                *remaining -= 1;
            }
            if self.bounding_box().contains(pixel.0) {
                self.writes += 1;
                self.outside |= self.allowed.is_some_and(|area| !area.contains(pixel.0));
            }
            self.surface.draw_iter([pixel]).unwrap();
        }
        Ok(())
    }

    fn clear(&mut self, color: Rgb888) -> Result<(), Self::Error> {
        self.full_clears += 1;
        self.writes += (self.size().width * self.size().height) as usize;
        self.outside |= self.allowed.is_some_and(|area| area != self.bounding_box());
        self.surface.clear(color).unwrap();
        Ok(())
    }
}

#[test]
fn caption_change_writes_only_its_row_and_unchanged_frame_writes_nothing() {
    let mut target = RecordedTarget::new();
    let mut retained = splash::RetainedSplash::default();
    let locales = crate::common::locales("en");
    assert!(retained.update(Duration::ZERO, Stage::Starting, &locales, &mut target).unwrap());
    assert_eq!(target.full_clears, 1);
    assert_eq!(target.surface.pixels(), splash::initial_frame().pixels());
    let row = Rectangle::new(Point::new(0, 423), Size::new(WIDTH, 20));
    target.observe(row);

    assert!(retained.update(Duration::ZERO, Stage::Gamepad, &locales, &mut target).unwrap());

    assert_eq!(target.full_clears, 0);
    assert!(!target.outside);
    assert!(target.writes > 0 && target.writes < (WIDTH * HEIGHT / 20) as usize);
    target.observe(row);

    assert!(!retained.update(Duration::from_millis(999), Stage::Gamepad, &locales, &mut target).unwrap());

    assert_eq!(target.writes, 0);
    assert_eq!(target.full_clears, 0);
}

#[test]
fn blink_writes_only_logo_rectangle_without_touching_caption() {
    let mut target = RecordedTarget::new();
    let mut retained = splash::RetainedSplash::default();
    let locales = crate::common::locales("ru");
    retained.update(Duration::ZERO, Stage::Starting, &locales, &mut target).unwrap();
    for seconds in [1, 2] {
        target.observe(Rectangle::new(Point::new(559, 319), Size::new(160, 80)));

        assert!(retained.update(Duration::from_secs(seconds), Stage::Starting, &locales, &mut target).unwrap());

        assert!(!target.outside);
        assert_eq!(target.full_clears, 0);
        assert!(target.writes >= 160 * 80 && target.writes < 2 * 160 * 80);
    }
}

#[test]
fn failed_partial_draw_is_reported_and_next_update_rebuilds_complete_frame() {
    let mut target = RecordedTarget::new();
    let mut retained = splash::RetainedSplash::default();
    let locales = crate::common::locales("en");
    retained.update(Duration::ZERO, Stage::Starting, &locales, &mut target).unwrap();
    target.full_clears = 0;
    target.fail_after = Some(50);

    assert_eq!(retained.update(Duration::ZERO, Stage::Gamepad, &locales, &mut target), Err("synthetic draw failure"));
    assert!(retained.update(Duration::ZERO, Stage::Gamepad, &locales, &mut target).unwrap());

    let mut expected = Surface::new(WIDTH, HEIGHT);
    splash::draw_stage(Duration::ZERO, Stage::Gamepad, &locales, &mut expected).unwrap();
    assert_eq!(target.full_clears, 1);
    assert_eq!(target.surface.pixels(), expected.pixels());
}

#[test]
fn changed_target_dimensions_require_a_full_frame() {
    let mut target = RecordedTarget::new();
    let mut retained = splash::RetainedSplash::default();
    let locales = crate::common::locales("en");
    retained.update(Duration::ZERO, Stage::Starting, &locales, &mut target).unwrap();
    target.surface = Surface::new(321, 241);
    target.full_clears = 0;

    assert!(retained.update(Duration::ZERO, Stage::Starting, &locales, &mut target).unwrap());

    let mut expected = Surface::new(321, 241);
    splash::draw_stage(Duration::ZERO, Stage::Starting, &locales, &mut expected).unwrap();
    assert_eq!(target.full_clears, 1);
    assert_eq!(target.surface.pixels(), expected.pixels());
}
