// SPDX-License-Identifier: BSD-3-Clause
use crate::common::ipc as ipc_support;

use embedded_graphics::{
    mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text},
};
use {
    igui::input::startup::StartupInputTest,
    igui::runtime::launch::LaunchDialog,
    igui::render::marquee::Marquee,
    igui::memory::MemoryInfo,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Preview,
    igui::model::TopTab,
    igui::render::layout::card as card,
    igui::render::PaintUpdate,
    igui::render::Renderer,
    igui::render::RetainedFrame,
    igui::render::HEIGHT,
    igui::render::WIDTH,
    imanlib::surface::Surface,
};
use serde_json::json;
use std::{cell::Cell, fs, path::Path, rc::Rc, thread, time::Duration};

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn fitting_text_never_scrolls_even_after_long_intervals() {
    for width in [0, 70, 540] {
        let mut marquee = Marquee::new(width, 540);

        for (time, paused) in [(0, false), (10_000, false), (50_000, true), (90_000, false)] {
            assert!(!marquee.advance(ms(time), paused));
            assert_eq!(marquee.offset(), 0);
        }
    }
}

#[test]
fn scrolling_starts_after_a_full_pause_and_uses_elapsed_time_not_frame_count() {
    let mut marquee = Marquee::new(600, 540);

    assert!(!marquee.advance(ms(42_000), false));
    assert!(!marquee.advance(ms(43_000), false));
    assert!(!marquee.advance(ms(43_033), false));
    assert!(marquee.advance(ms(43_034), false));
    assert_eq!(marquee.offset(), 1);
    assert!(marquee.advance(ms(44_500), false));
    assert_eq!(marquee.offset(), 45);
    assert!(!marquee.advance(ms(44_501), false));
}

#[test]
fn each_loop_has_a_gap_and_a_new_pause_without_a_backward_sweep() {
    let mut marquee = Marquee::new(600, 540);
    marquee.advance(ms(0), false);

    marquee.advance(ms(22_999), false);
    assert_eq!(marquee.offset(), 659);
    assert!(marquee.advance(ms(23_000), false));
    assert_eq!(marquee.offset(), 0);
    assert!(!marquee.advance(ms(24_000), false));
    assert!(marquee.advance(ms(24_034), false));
    assert_eq!(marquee.offset(), 1);
}

#[test]
fn pausing_freezes_the_position_and_resuming_does_not_catch_up() {
    let mut marquee = Marquee::new(2_000, 540);
    marquee.advance(ms(0), false);
    marquee.advance(ms(2_000), false);
    assert_eq!(marquee.offset(), 30);

    for (time, paused) in [(10_000, true), (100_000, true), (110_000, false)] {
        assert!(!marquee.advance(ms(time), paused));
        assert_eq!(marquee.offset(), 30);
    }
    assert!(marquee.advance(ms(111_000), false));
    assert_eq!(marquee.offset(), 60);
}

#[test]
fn backwards_timestamps_do_not_count_the_same_interval_twice() {
    let mut marquee = Marquee::new(2_000, 540);
    marquee.advance(ms(10_000), false);
    marquee.advance(ms(12_000), false);

    assert!(!marquee.advance(ms(11_000), false));
    assert!(!marquee.advance(ms(12_000), false));
    marquee.advance(ms(13_000), false);
    assert_eq!(marquee.offset(), 60);
}

#[test]
fn large_dimensions_and_clock_values_do_not_overflow() {
    let mut marquee = Marquee::new(usize::MAX, 540);
    marquee.advance(Duration::ZERO, false);

    marquee.advance(Duration::MAX, false);

    assert!(marquee.offset() < usize::MAX);
    assert!(!marquee.advance(Duration::MAX, true));
}

fn long_title(alphabet: &str) -> String {
    alphabet.chars().cycle().take(60).collect()
}

fn title_bounds() -> Rectangle {
    Rectangle::new(Point::new(card::TITLE_X, card::TITLE_Y), Size::new(card::TITLE_WIDTH, card::TITLE_HEIGHT))
}

// Independent reference draws the complete string twice through a viewport.
// Production instead selects at most 55 visible Unicode scalars per copy.
fn reference(app: &App, notice: Option<&str>, offset: usize) -> Surface {
    let mut surface = Surface::new(WIDTH, HEIGHT);
    Renderer::default().draw(app, notice, &mut surface).unwrap();
    if let Some(game) = app.detail_game() {
        let mut target = surface.clipped(&title_bounds());
        target.clear(Rgb888::new(18, 26, 35)).unwrap();
        let width = game.title.chars().count() * 10;
        let mut origins = vec![card::TITLE_X - offset as i32];
        if width > card::TITLE_WIDTH as usize {
            origins.push(card::TITLE_X + (width + 60) as i32 - offset as i32);
        }
        for x in origins {
            Text::with_baseline(
                &game.title,
                Point::new(x, card::TITLE_Y),
                MonoTextStyle::new(&FONT_10X20, Rgb888::new(119, 207, 180)),
                Baseline::Top,
            ).draw(&mut target).unwrap();
        }
    }
    surface
}

fn assert_pixels(frame: &RetainedFrame, expected: &Surface) {
    let first = frame.surface().pixels().iter().zip(expected.pixels()).position(|(a, b)| a != b);
    assert!(first.is_none(), "different pixel: {:?}", first.map(|i| (i % WIDTH as usize, i / WIDTH as usize)));
}

#[test]
fn moving_title_matches_full_text_is_clipped_and_never_reads_the_preview_again() {
    for (language, alphabet) in [("en", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"), ("ru", "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ")] {
        let mut game = crate::common::game("a.rom", &long_title(alphabet), "NES");
        game.preview = Preview::Hash(game.hash.clone());
        let mut app = App::new(vec![game]);
        app.set_locales(crate::common::locales(language));
        let reads = Rc::new(Cell::new(0));
        let counter = reads.clone();
        let renderer = Renderer::with_previews(move |_| { counter.set(counter.get() + 1); None });
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), renderer);
        assert!(frame.update(&app, None, None).unwrap().full);
        let mut before = frame.surface().pixels().to_vec();
        let mut old_offset = 0;

        for (time, offset) in [(0, 0), (1_000, 0), (1_034, 1), (2_000, 30), (5_000, 120),
            (10_000, 270), (20_000, 570), (22_999, 659), (23_000, 0), (24_000, 0), (24_034, 1)] {
            let update = frame.advance_title(ms(time), true).unwrap();

            assert_eq!(update, PaintUpdate { title: offset != old_offset, ..Default::default() });
            assert_pixels(&frame, &reference(&app, None, offset));
            for (i, (&a, &b)) in before.iter().zip(frame.surface().pixels()).enumerate() {
                if a != b {
                    assert!(title_bounds().contains(Point::new((i % WIDTH as usize) as i32, (i / WIDTH as usize) as i32)));
                }
            }
            before.copy_from_slice(frame.surface().pixels());
            old_offset = offset;
        }
        assert_eq!(reads.get(), 1);
        assert_eq!(frame.statistics().full, 1);
        assert_eq!(frame.statistics().metadata, 0);
        assert_eq!(frame.statistics().preview, 0);
    }
}

#[test]
fn empty_and_fitting_cards_do_not_trigger_animation_redraws() {
    for games in [vec![], vec![crate::common::game("short.rom", "Short", "NES")],
        vec![crate::common::game("fits.rom", &"A".repeat(54), "NES")]] {
        let app = App::new(games);
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), Renderer::default());
        frame.update(&app, None, None).unwrap();

        assert!(!frame.advance_title(ms(0), true).unwrap().changed());
        assert!(!frame.advance_title(ms(100_000), true).unwrap().changed());
        assert_pixels(&frame, &reference(&app, None, 0));
        assert_eq!(frame.statistics().requests, 1);
    }
}

#[test]
fn selection_or_title_changes_restart_from_the_beginning_with_a_full_pause() {
    let title = long_title("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    for (image, next_title) in [("b.rom", title.clone()), ("a.rom", long_title("ZYXWVUTSRQPONMLKJIHGFEDCBA"))] {
        let app = App::new(vec![crate::common::game("a.rom", &title, "NES")]);
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), Renderer::default());
        frame.update(&app, None, None).unwrap();
        frame.advance_title(ms(0), true).unwrap();
        frame.advance_title(ms(2_000), true).unwrap();

        let app = App::new(vec![crate::common::game(image, &next_title, "NES")]);
        assert!(!frame.update(&app, None, None).unwrap().full);
        assert_pixels(&frame, &reference(&app, None, 0));
        for time in [2_000, 2_999, 3_000] {
            assert!(!frame.advance_title(ms(time), true).unwrap().changed());
        }
        assert!(frame.advance_title(ms(3_034), true).unwrap().title);
        assert_pixels(&frame, &reference(&app, None, 1));
    }
}

#[test]
fn metadata_memory_and_locale_updates_preserve_the_scroll_position() {
    let mut game = crate::common::game("a.rom", &long_title("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), "NES");
    game.description = Some("Long description. ".repeat(200));
    let mut app = App::new(vec![game]);
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), Renderer::default());
    frame.update(&app, None, None).unwrap();
    frame.advance_title(ms(0), true).unwrap();
    frame.advance_title(ms(2_000), true).unwrap();

    app.dispatch(Right);
    app.dispatch(Down);
    assert!(frame.update(&app, None, None).unwrap().metadata);
    assert_pixels(&frame, &reference(&app, None, 30));
    for notice in [Some("Save not confirmed."), None] {
        assert!(frame.update(&app, notice, None).unwrap().metadata);
        assert_pixels(&frame, &reference(&app, notice, 30));
    }
    app.set_memory(Some(MemoryInfo { available_bytes: 100_000_000, total_bytes: 256_000_000 }));
    assert_eq!(frame.update(&app, None, None).unwrap(), PaintUpdate { telemetry: true, ..Default::default() });
    assert_pixels(&frame, &reference(&app, None, 30));
    app.set_locales(crate::common::locales("ru"));
    assert!(frame.update(&app, None, None).unwrap().full);
    assert_pixels(&frame, &reference(&app, None, 30));
    assert!(!frame.advance_title(ms(10_000), false).unwrap().changed());
    assert!(!frame.advance_title(ms(20_000), true).unwrap().changed());
    frame.advance_title(ms(21_000), true).unwrap();
    assert_pixels(&frame, &reference(&app, None, 60));
}

#[test]
fn all_dialogs_freeze_the_title_and_restore_its_current_pixels_without_a_full_redraw() {
    for kind in ["launch", "trash", "countdown"] {
        let mut app = App::new(vec![crate::common::game("a.rom", &long_title("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), "NES")]);
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), Renderer::default());
        frame.update(&app, None, None).unwrap();
        frame.advance_title(ms(0), true).unwrap();
        frame.advance_title(ms(2_000), true).unwrap();
        let launch = LaunchDialog::new("Game".into(), "NES".into());
        let mut countdown = StartupInputTest::default();
        let dialog = (kind == "launch").then_some(&launch);
        if kind == "trash" { app.dispatch(Trash); }
        if kind == "countdown" {
            countdown.update(0, true, false, false);
            countdown.update(10_000, true, false, false);
        }

        assert_eq!(frame.update_with_countdown(&app, None, dialog, Some(&countdown)).unwrap(),
            PaintUpdate { dialog: true, ..Default::default() });
        assert!(!frame.advance_title(ms(10_000), true).unwrap().changed());
        let mut expected = reference(&app, None, 30);
        if let Some(dialog) = dialog { igui::render::draw_launch_dialog(&app, dialog, &mut expected).unwrap(); }
        if kind == "trash" { igui::render::draw_trash_dialog(&app, &mut expected).unwrap(); }
        igui::render::draw_input_test_countdown(&countdown, &mut expected).unwrap();
        assert_pixels(&frame, &expected);
        if kind == "trash" { app.dispatch(Favourite); }
        countdown.cancel();
        assert_eq!(frame.update_with_countdown(&app, None, None, Some(&countdown)).unwrap(),
            PaintUpdate { dialog: true, ..Default::default() });
        assert_pixels(&frame, &reference(&app, None, 30));
        assert!(!frame.advance_title(ms(20_000), true).unwrap().changed());
        assert!(frame.advance_title(ms(21_000), true).unwrap().title);
        assert_pixels(&frame, &reference(&app, None, 60));
        assert_eq!(frame.statistics().full, 1);
    }
}

#[test]
fn search_has_no_animation_and_reopening_the_card_starts_a_new_cycle() {
    let mut app = App::new(vec![crate::common::game("a.rom", &long_title("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), "NES")]);
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), Renderer::default());
    frame.update(&app, None, None).unwrap();
    frame.advance_title(ms(0), true).unwrap();
    frame.advance_title(ms(2_000), true).unwrap();

    crate::common::top_tab(&mut app, TopTab::Search);
    assert!(frame.update(&app, None, None).unwrap().right);
    assert!(!frame.advance_title(ms(100_000), true).unwrap().changed());
    assert_pixels(&frame, &reference(&app, None, 0));
    app.dispatch(Filters);
    assert!(frame.update(&app, None, None).unwrap().right);
    assert!(!frame.advance_title(ms(110_000), true).unwrap().changed());
    assert!(!frame.advance_title(ms(111_000), true).unwrap().changed());
    assert_pixels(&frame, &reference(&app, None, 0));
    assert!(frame.advance_title(ms(111_034), true).unwrap().title);
    assert_pixels(&frame, &reference(&app, None, 1));
    assert_eq!(frame.statistics().full, 1);
}

#[test]
fn real_headless_loop_animates_without_input_or_preference_writes() {
    let directory = tempfile::tempdir_in("/dev/shm").unwrap();
    let games = directory.path().join("games");
    fs::create_dir(&games).unwrap();
    fs::write(games.join("catalogue.json"), serde_json::to_vec(&json!({"games":[{
        "platform":"NES", "image":"synthetic.nes", "title":long_title("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        "hash":crate::common::hash("synthetic.nes")
    }]})).unwrap()).unwrap();
    let mut driver = ipc_support::Driver::new(Path::new(env!("CARGO_BIN_EXE_igui")), Path::new("/"));
    driver.args(["--games"]).args([games.as_os_str()]);
    let mut running = driver.start().unwrap();
    running.wait_ready().unwrap();
    let reply = running.client.value("igui.screenshot", json!({})).unwrap();
    let before = crate::common::images::read_screenshot(Path::new(reply["path"].as_str().unwrap()));

    thread::sleep(ms(1_300));
    let reply = running.client.value("igui.screenshot", json!({})).unwrap();
    let after = crate::common::images::read_screenshot(Path::new(reply["path"].as_str().unwrap()));

    let mut changed = 0;
    for y in card::TOP as usize..card::BOTTOM as usize {
        for x in 656..1252 {
            let i = (y * WIDTH as usize + x) * 3;
            if before[i..i + 3] != after[i..i + 3] {
                assert!(title_bounds().contains(Point::new(x as i32, y as i32)));
                changed += 1;
            }
        }
    }
    assert!(changed > 0);
    assert!(!directory.path().join("prefs.json").exists());
    assert_eq!(fs::read_dir(&games).unwrap().count(), 1);
    running.client.value("igui.close", json!({})).unwrap();
    assert!(running.finish().unwrap().status.success());
}
