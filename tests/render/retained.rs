// SPDX-License-Identifier: BSD-3-Clause
use {
    crate::common::sample_catalogue,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::render::Renderer,
    igui::render::HEIGHT,
    igui::render::WIDTH,
    imanlib::surface::Surface,
};
use sha2::{Digest, Sha256};
use embedded_graphics::{pixelcolor::Rgb888, prelude::*, primitives::Rectangle};
use igui::{
    runtime::launch::LaunchDialog,
    memory::MemoryInfo,
    model::{Effect, Preview},
    render::{layout::card as card_layout, PaintUpdate, RetainedFrame, FOOTER_STATS_X, FOOTER_Y},
};
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

#[test]
fn full_render_reference_pixels() {
    let mut hashes = Vec::new();
    let version_end = 92 + ("idle gui / ".len() + env!("CARGO_PKG_VERSION").len()) as i32 * 7;
    let build_stamp = Rectangle::new(Point::new(version_end, 33), Size::new(13 * 7, 13));
    for language in ["en", "ru"] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        let mut renderer = crate::common::renderer();
        let mut surface = Surface::new(WIDTH, HEIGHT);
        for action in [
            None,
            Some(Down),
            Some(Right),
            Some(Down),
            Some(Up),
            Some(LowerFilters),
            Some(Activate),
            Some(Down),
            Some(Favourite),
            Some(Filters),
            Some(Right),
            Some(Activate),
        ] {
            if let Some(action) = action {
                app.dispatch(action);
            }
            renderer
                .draw(&app, Some("Test notice"), &mut surface)
                .unwrap();
            // Only the new build suffix varies; its text is checked in the panel tests.
            // The reference includes the search tab; all other pixels remain checked.
            surface
                .fill_solid(&build_stamp, Rgb888::new(23, 32, 42))
                .unwrap();
            hashes.push(format!("{:x}", Sha256::digest(surface.rgb_bytes())));
        }
    }
    let expected = [
        "23a369fd854b18ba401e3610fc9cde0f201796b0babb06592f777f6e269d6bab",
        "5d02681c1577a6034fa42746f53f37240459e51a0335153260ded8097d5c0277",
        "cc235fe5908390ee5ff83b58da928bc383b191d710c13e607943a5306589bb62",
        "cc235fe5908390ee5ff83b58da928bc383b191d710c13e607943a5306589bb62",
        "cc235fe5908390ee5ff83b58da928bc383b191d710c13e607943a5306589bb62",
        "1299ecc33fc2092bfa13744fda6f6aeb1acfd2ddf00c48fa7f34caf702052d89",
        "2907f4d4f09f5b1438db9e8e42a0533e36c92af7996d8ca359d1c653d96b343b",
        "da150f5da2b24b7667280ce9cf53218248e372a1b60cfb54af5bb1501c01cead",
        "1299ecc33fc2092bfa13744fda6f6aeb1acfd2ddf00c48fa7f34caf702052d89",
        "ad1e2c641c4b01c982b9ca999062d45f4517f5b211ccdb666d25d44cf187a560",
        "3f22ba7d5b4d57fc3ae8640609c6d7bd4c987172509250f6a7b7468a1e0d6fa7",
        "9f47221d15f3b6fdd6b181b3be4a9e57e8130fe6341a14ca5757d37c46f24d1d",
        "be1c32b8aa328feeab1a4654eab9ff65078b87bef9669f55c009595c31a68574",
        "253d593d7d38686b9e01c433b70c249d9f420ee5cab0012ca2506383ec6c4e2a",
        "f9b1bbecc4734ad5576d2537c420341cee1be5b148dad7d799eeba0e5b2d5504",
        "f9b1bbecc4734ad5576d2537c420341cee1be5b148dad7d799eeba0e5b2d5504",
        "f9b1bbecc4734ad5576d2537c420341cee1be5b148dad7d799eeba0e5b2d5504",
        "c6746a6ff08ce215b4c01dafa7592ad98e27eb321a0fe6bf8a327e00c4cb493f",
        "d5329ee5c7d4d048af090a50d3902daa98ba4d59e337e1805902dc91f51e721d",
        "e39bc6aad9175414612951ca5ebccdcce5f63da5720b6578379f2d1b79dd95d6",
        "c6746a6ff08ce215b4c01dafa7592ad98e27eb321a0fe6bf8a327e00c4cb493f",
        "6c6773c0e142d1138506f14abb0b1e7f7bc6e877a66d179c82f375c0eb169a68",
        "926628bc93147aaf230b05a3b360e84a2a9db6fddacbd4d457d9e06c34cbb17a",
        "1c779cb06221e6f8faa1bd27ef5ada94e943b0a21e1c9e2ca89effa5069ad8a1",
    ];
    assert_eq!(hashes, expected);
}

fn assert_same(actual: &Surface, expected: &Surface) {
    let first = actual
        .pixels()
        .iter()
        .zip(expected.pixels())
        .position(|(actual, expected)| actual != expected);
    assert!(
        first.is_none(),
        "different pixel at {:?}",
        first.map(|index| (index % WIDTH as usize, index / WIDTH as usize))
    );
}

fn compare(
    frame: &mut RetainedFrame,
    reference: &mut Renderer,
    app: &App,
    notice: Option<&str>,
    dialog: Option<&LaunchDialog>,
) -> PaintUpdate {
    let update = frame.update(app, notice, dialog).unwrap();
    let mut full = Surface::new(WIDTH, HEIGHT);
    reference.draw(app, notice, &mut full).unwrap();
    if let Some(dialog) = dialog {
        igui::render::draw_launch_dialog(app, dialog, &mut full).unwrap();
    } else if app.trash_confirmation() {
        igui::render::draw_trash_dialog(app, &mut full).unwrap();
    }
    assert_same(frame.surface(), &full);
    update
}

#[test]
fn search_matches_full_render_without_hidden_preview_work_or_idle_redraw() {
    use igui::model::TopTab;
    for language in ["en", "ru"] {
        let mut games = vec![crate::common::game("a", "Alpha", "MD"), crate::common::game("b", "Beta", "MD")];
        for game in &mut games {
            game.preview = Preview::Hash(game.hash.clone());
            game.description = Some("Long description. ".repeat(500));
        }
        let mut app = App::new(games);
        app.set_locales(crate::common::locales(language));
        let reads = Rc::new(Cell::new(0));
        let counter = reads.clone();
        let picture = || include_bytes!("../_fixtures/previews/hills.png").to_vec();
        let renderer = Renderer::with_previews(move |_| {
            counter.set(counter.get() + 1);
            Some(picture())
        });
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), renderer);
        let mut reference = Renderer::with_previews(move |_| Some(picture()));
        compare(&mut frame, &mut reference, &app, None, None);
        assert_eq!(reads.get(), 1);
        assert!(frame.prepared_preview_bytes() > 0);
        crate::common::top_tab(&mut app, TopTab::Search);
        let update = compare(&mut frame, &mut reference, &app, None, None);
        assert!(update.right);
        assert!(!update.full);
        assert_eq!(frame.prepared_preview_bytes(), 0);

        for key in "BETA 20".chars() {
            for action in crate::common::search_key_actions(&app, key) {
                app.dispatch(action);
                let update = compare(&mut frame, &mut reference, &app, None, None);
                assert!(!update.full && !update.preview && !update.metadata);
                assert_eq!(reads.get(), 1);
                assert!(app.description_lines().is_empty());
            }
        }
        crate::common::search_field(&mut app);
        compare(&mut frame, &mut reference, &app, None, None);
        for action in [Left, Right, LowerFilters, Favourite, Left, Right, Activate] {
            app.dispatch(action);
            let update = compare(&mut frame, &mut reference, &app, None, None);
            assert!(!update.preview && !update.metadata && !update.full);
            if matches!(action, Left | Right | Activate) {
                assert!(!update.left);
            }
        }
        assert_eq!(reads.get(), 1);
        assert_eq!(compare(&mut frame, &mut reference, &app, None, None), PaintUpdate::default());
        app.set_memory(Some(MemoryInfo { available_bytes: 100, total_bytes: 200 }));
        assert_eq!(compare(&mut frame, &mut reference, &app, None, None), PaintUpdate {
            telemetry: true, ..Default::default()
        });
        // Error notices remain visible even though the card is hidden.
        assert!(compare(&mut frame, &mut reference, &app, Some("Test notice"), None).right);
        crate::common::type_search(&mut app, "B");
        compare(&mut frame, &mut reference, &app, None, None);
        // The selected result changes while the keyboard remains visible.
        assert_eq!(app.selected_game().unwrap().title, "Beta");
        assert_eq!(reads.get(), 1);
        for action in [Left, Left, Up, Up, Down, Down] {
            let previous = app.focus();
            app.dispatch(action);
            let update = compare(&mut frame, &mut reference, &app, None, None);
            if !previous.is_search() && !app.focus().is_search() {
                assert!(!update.right);
            }
        }
        assert_eq!(app.dispatch(Activate), Effect::None);
        let update = compare(&mut frame, &mut reference, &app, None, None);
        assert!(update.right && !update.full);
        assert_eq!(reads.get(), 2);
        assert_eq!(app.focus(), igui::model::Focus::Games);
        assert_eq!(app.selected_game().unwrap().title, "Beta");
        assert!(!app.description_lines().is_empty());
    }
}

#[test]
fn mixed_width_tab_scrolling_matches_full_frame_in_both_directions() {
    for count in [0, 4, 12] {
        let games = (0..count).map(|index_item| {
            crate::common::game(&index_item.to_string(), "Game", &format!("P{index_item:02}"))
        }).collect();
        let mut app = App::new(games);
        app.dispatch(Filters);
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
        let mut reference = crate::common::renderer();
        for direction in [Right, Left] {
            for _ in 0..app.platform_tab_count() + 1 {
                compare(&mut frame, &mut reference, &app, None, None);
                app.dispatch(direction);
            }
        }
    }
}

#[test]
fn long_search_and_modal_transitions_match_full_render() {
    use igui::model::TopTab;
    let mut app = App::new(sample_catalogue());
    crate::common::top_tab(&mut app, TopTab::Search);
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
    let mut reference = crate::common::renderer();
    compare(&mut frame, &mut reference, &app, None, None);
    crate::common::type_search(&mut app, &"A".repeat(128));
    compare(&mut frame, &mut reference, &app, None, None);
    crate::common::search_field(&mut app);
    for _ in 0..130 {
        app.dispatch(Left);
        compare(&mut frame, &mut reference, &app, None, None);
    }
    app.dispatch(Favourite);
    compare(&mut frame, &mut reference, &app, None, None);
    let dialog = LaunchDialog::new("Game".into(), "NES".into());
    compare(&mut frame, &mut reference, &app, None, Some(&dialog));
    compare(&mut frame, &mut reference, &app, None, None);
    app.dispatch(Filters);
    compare(&mut frame, &mut reference, &app, None, None);
}

#[test]
fn incremental_pixels_match_full_for_navigation_filters_prefs_notice_and_modal() {
    for language in ["en", "ru"] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
        let mut reference = crate::common::renderer();
        let mut favourites = BTreeSet::new();
        let mut trash = Vec::new();
        compare(&mut frame, &mut reference, &app, None, None);
        // Exercise scrolling, all regions, dropdown overlays, empty filters and persistence publication.
        let actions = [
            Down,
            Down,
            Favourite,
            Right,
            Down,
            Down,
            Up,
            Up,
            Left,
            LowerFilters,
            Activate,
            Down,
            Down,
            Favourite,
            Right,
            Activate,
            Down,
            Activate,
            Filters,
            Right,
            Right,
            Activate,
            Down,
            Left,
            Down,
            Activate,
            Right,
            Trash,
            Activate,
            Filters,
            Left,
            Activate,
            Down,
        ];
        for action in std::iter::repeat_n(Down, 34).chain(actions.into_iter().cycle().take(70)) {
            match app.dispatch(action) {
                Effect::SaveFavourite { hash, enabled } => {
                    if enabled {
                        favourites.insert(hash);
                    } else {
                        favourites.remove(&hash);
                    }
                    app.set_favourites(favourites.clone());
                }
                Effect::SaveTrash { hash, enabled } => {
                    if enabled {
                        trash.insert(0, hash);
                    } else {
                        trash.retain(|entry| entry != &hash);
                    }
                    app.set_trash(trash.clone());
                }
                _ => {}
            }
            compare(&mut frame, &mut reference, &app, None, None);
        }
        for notice in [
            Some("Cannot launch game.\nRequired BIOS missing: system/bios/FDS/disksys.rom"),
            Some("Short"),
            None,
        ] {
            compare(&mut frame, &mut reference, &app, notice, None);
        }
        let dialog =
            LaunchDialog::new("A long launch name with several words".into(), "NES".into());
        compare(&mut frame, &mut reference, &app, None, Some(&dialog));
        app.set_memory(Some(MemoryInfo {
            available_bytes: 100_000_000,
            total_bytes: 256_000_000,
        }));
        compare(&mut frame, &mut reference, &app, None, Some(&dialog));
        assert_eq!(compare(&mut frame, &mut reference, &app, None, None), PaintUpdate {
            dialog: true, ..Default::default()
        });
        assert!(!compare(&mut frame, &mut reference, &app, None, None).changed());
    }
}

#[test]
fn telemetry_only_changes_reserved_cells_and_rounded_equal_values_do_not_paint() {
    let mut app = App::new(sample_catalogue());
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
    let mut reference = crate::common::renderer();
    compare(&mut frame, &mut reference, &app, None, None);
    let old = frame.surface().pixels().to_vec();
    app.set_memory(Some(MemoryInfo {
        available_bytes: 100 * 1_048_576,
        total_bytes: 242 * 1_048_576,
    }));
    let update = compare(&mut frame, &mut reference, &app, None, None);
    assert_eq!(
        update,
        PaintUpdate {
            telemetry: true,
            ..Default::default()
        }
    );
    for (index_item, (&first_item, &second_item)) in old.iter().zip(frame.surface().pixels()).enumerate() {
        if first_item != second_item {
            assert!(
                index_item % WIDTH as usize >= FOOTER_STATS_X as usize
                    && index_item / WIDTH as usize >= FOOTER_Y as usize
            );
        }
    }
    app.set_memory(Some(MemoryInfo {
        available_bytes: 100 * 1_048_576 + 1024,
        total_bytes: 242 * 1_048_576,
    }));
    assert!(!compare(&mut frame, &mut reference, &app, None, None).changed());
    for available in [Some(9_999_999_999), None, Some(0)] {
        app.set_available_bytes(available);
        assert_eq!(
            compare(&mut frame, &mut reference, &app, None, None),
            PaintUpdate {
                telemetry: true,
                ..Default::default()
            }
        );
    }
    assert_eq!(frame.statistics().full, 1);
}

#[test]
fn replacing_app_locale_or_surface_cannot_reuse_stale_pixels() {
    let mut app = App::new(sample_catalogue());
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
    let mut reference = crate::common::renderer();
    compare(&mut frame, &mut reference, &app, None, None);
    app.set_locales(crate::common::locales("ru"));
    assert!(compare(&mut frame, &mut reference, &app, None, None).full);
    app.set_locales(igui::locales::Locales::new(
        igui::locales::Locale::parse("ru", br#"{"RAM":"Memory","Title":"Name"}"#).unwrap(),
    ));
    assert!(compare(&mut frame, &mut reference, &app, None, None).full);
    frame.surface_mut().clear(Rgb888::RED).unwrap();
    assert!(compare(&mut frame, &mut reference, &app, None, None).full);
    let mut game = crate::common::game("a.nes", "New title", "NES");
    game.description = Some("Many lines of description. ".repeat(150));
    let mut app = App::new(vec![game]);
    compare(&mut frame, &mut reference, &app, None, None);
    for action in std::iter::once(Right)
        .chain(std::iter::repeat_n(Down, 12))
        .chain(std::iter::repeat_n(Up, 15))
    {
        app.dispatch(action);
        compare(&mut frame, &mut reference, &app, None, None);
    }
    compare(&mut frame, &mut reference, &App::new(vec![]), None, None);
}

fn png_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, 3, 2);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[
                255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 0, 255, 255, 255, 0, 255,
            ])
            .unwrap();
    }
    data
}

#[test]
fn prepared_preview_is_bounded_reused_for_ram_and_copied_for_new_selection() {
    let mut games = vec![
        crate::common::game("a.nes", "A", "NES"),
        crate::common::game("b.nes", "B", "NES"),
    ];
    for game in &mut games {
        game.preview = Preview::Hash(game.hash.clone());
    }
    let mut app = App::new(games);
    let reads = Rc::new(Cell::new(0));
    let seen = reads.clone();
    let mut frame = RetainedFrame::new(
        Surface::new(WIDTH, HEIGHT),
        Renderer::with_previews(move |_| {
            seen.set(seen.get() + 1);
            Some(png_fixture())
        }),
    );
    let mut reference = Renderer::with_previews(|_| Some(png_fixture()));
    frame.prepare_selected(&app);
    assert_eq!(reads.get(), 1);
    compare(&mut frame, &mut reference, &app, None, None);
    let bytes = frame.prepared_preview_bytes();
    assert!(bytes > 0 && bytes <= 564 * card_layout::PREVIEW_IMAGE_HEIGHT as usize * 4);
    app.set_memory(Some(MemoryInfo {
        available_bytes: 1,
        total_bytes: 100,
    }));
    assert!(compare(&mut frame, &mut reference, &app, None, None).telemetry);
    assert_eq!(reads.get(), 1);
    app.dispatch(Down);
    assert!(compare(&mut frame, &mut reference, &app, None, None).preview);
    assert_eq!(reads.get(), 2);
    compare(&mut frame, &mut reference, &App::new(vec![]), None, None);
    assert_eq!(frame.prepared_preview_bytes(), 0);
}

#[test]
fn valid_thin_preview_retains_empty_area_without_failure_label() {
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, 640, 1);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&vec![255; 640 * 3])
            .unwrap();
    }
    let mut game = crate::common::game("a.nes", "A", "NES");
    game.preview = Preview::Hash(game.hash.clone());
    let app = App::new(vec![game]);
    let mut frame = RetainedFrame::new(
        Surface::new(WIDTH, HEIGHT),
        Renderer::with_previews(move |_| Some(data.clone())),
    );
    frame.update(&app, None, None).unwrap();
    assert_eq!(frame.prepared_preview_bytes(), 0);
    for position_y in card_layout::PREVIEW_TOP..card_layout::PREVIEW_BOTTOM {
        for position_x in 656..1252 {
            assert_eq!(
                frame.surface().pixels()[position_y as usize * WIDTH as usize + position_x],
                0x121a23
            );
        }
    }
}

#[test]
fn launch_overlay_only_paints_once_and_ram_tick_does_not_repaint_background() {
    let mut app = App::new(sample_catalogue());
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
    let mut reference = crate::common::renderer();
    compare(&mut frame, &mut reference, &app, None, None);
    let mut dialog = LaunchDialog::new("Battle City".into(), "NES".into());
    let before = frame.surface().pixels().to_vec();

    assert_eq!(
        compare(&mut frame, &mut reference, &app, None, Some(&dialog)),
        PaintUpdate {
            dialog: true,
            ..Default::default()
        }
    );
    // This one-line panel plus shadow fits in x=200..1088,y=304..424.
    for (index_item, (&first_item, &second_item)) in before.iter().zip(frame.surface().pixels()).enumerate() {
        if first_item != second_item {
            let position_x = index_item % WIDTH as usize;
            let position_y = index_item / WIDTH as usize;
            assert!((200..1088).contains(&position_x) && (304..424).contains(&position_y));
        }
    }
    dialog.presented(std::time::Duration::from_secs(10));
    assert!(!compare(&mut frame, &mut reference, &app, None, Some(&dialog)).changed());
    app.set_memory(Some(MemoryInfo {
        available_bytes: 100_000_000,
        total_bytes: 256_000_000,
    }));
    assert_eq!(
        compare(&mut frame, &mut reference, &app, None, Some(&dialog)),
        PaintUpdate {
            telemetry: true,
            ..Default::default()
        }
    );
    assert_eq!(frame.statistics().full, 1);
    assert_eq!(frame.statistics().dialog_cost.count, 1);
    assert_eq!(frame.statistics().telemetry_only_cost.count, 1);
    let update = compare(&mut frame, &mut reference, &app, Some("Launch failed"), None);
    assert!(update.dialog && update.metadata && !update.full);
}

#[test]
fn modal_replacement_and_intersecting_background_changes_restore_all_pixels() {
    let mut app = App::new(sample_catalogue());
    let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
    let mut reference = crate::common::renderer();
    let long = LaunchDialog::new("A very long game title ".repeat(30), "MD".into());
    let short = LaunchDialog::new("Short".into(), "NES".into());
    compare(&mut frame, &mut reference, &app, None, Some(&long));
    assert_eq!(compare(&mut frame, &mut reference, &app, None, Some(&short)), PaintUpdate {
        dialog: true, ..Default::default()
    });
    let replacement = LaunchDialog::new("Another short title".into(), "MD".into());
    for dialog in [&replacement, &short] {
        assert_eq!(compare(&mut frame, &mut reference, &app, None, Some(dialog)), PaintUpdate {
            dialog: true, ..Default::default()
        });
    }
    for action in [Down, Right, Down, Up, Left, LowerFilters, Activate, Down] {
        app.dispatch(action);
        let update = compare(&mut frame, &mut reference, &app, None, Some(&short));
        assert!(!update.full);
    }
    app.set_locales(crate::common::locales("ru"));
    assert!(compare(&mut frame, &mut reference, &app, None, Some(&short)).full);
    frame.surface_mut().clear(Rgb888::RED).unwrap();
    assert!(compare(&mut frame, &mut reference, &app, None, Some(&short)).full);
    compare(&mut frame, &mut reference, &app, Some("Failure"), None);
}

#[test]
fn error_panel_appears_changes_and_disappears_without_stale_pixels_in_both_tabs() {
    let missing = igui::render::notice::launch_failed("No such file or directory (os error 2)");
    for language in ["en", "ru"] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
        let mut reference = crate::common::renderer();
        compare(&mut frame, &mut reference, &app, None, None);
        for search in [false, true] {
            if search {
                crate::common::top_tab(&mut app, igui::model::TopTab::Search);
                compare(&mut frame, &mut reference, &app, None, None);
            }
            for notice in [Some(missing.as_str()), Some("Save not confirmed."), None] {
                let update = compare(&mut frame, &mut reference, &app, notice, None);
                assert!(!update.full);
                assert_eq!(update.right, search);
                assert_eq!(update.metadata, !search);
                assert!(!compare(&mut frame, &mut reference, &app, notice, None).changed());
            }
        }
    }
}

#[test]
fn trash_modal_opens_once_survives_ram_ticks_and_closes_cleanly_on_a_or_b() {
    for language in ["en", "ru"] {
        for search in [false, true] {
            for answer in [Activate, Favourite] {
                let mut app = App::new(sample_catalogue());
                app.set_locales(crate::common::locales(language));
                if search {
                    crate::common::top_tab(&mut app, igui::model::TopTab::Search);
                    app.dispatch(Down);
                    app.dispatch(Left);
                }
                let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
                let mut reference = crate::common::renderer();
                compare(&mut frame, &mut reference, &app, None, None);

                assert_eq!(app.dispatch(Trash), Effect::None);
                assert!(app.trash_confirmation());
                assert_eq!(compare(&mut frame, &mut reference, &app, None, None), PaintUpdate {
                    dialog: true,
                    ..Default::default()
                });
                assert!(!compare(&mut frame, &mut reference, &app, None, None).changed());
                app.set_memory(Some(MemoryInfo {
                    available_bytes: 100_000_000,
                    total_bytes: 256_000_000,
                }));
                assert_eq!(compare(&mut frame, &mut reference, &app, None, None), PaintUpdate {
                    telemetry: true,
                    ..Default::default()
                });

                let before = frame.surface().pixels().to_vec();
                if let Effect::SaveTrash { hash, enabled } = app.dispatch(answer) {
                    assert!(enabled);
                    app.set_trash([hash].into());
                }

                assert!(!app.trash_confirmation());
                let update = compare(&mut frame, &mut reference, &app, None, None);
                assert!(update.dialog && !update.full);
                if answer == Favourite {
                    assert_eq!(update, PaintUpdate { dialog: true, ..Default::default() });
                    for (index_item, (&first_item, &second_item)) in before.iter().zip(frame.surface().pixels()).enumerate() {
                        if first_item != second_item {
                            assert!((200..1088).contains(&(index_item % WIDTH as usize))
                                && (304..424).contains(&(index_item / WIDTH as usize)));
                        }
                    }
                }
                assert!(!compare(&mut frame, &mut reference, &app, None, None).changed());
                assert_eq!(app.search().is_some(), search);
                assert_eq!(frame.statistics().full, 1);
            }
        }
    }
}

fn compare_countdown(
    frame: &mut RetainedFrame,
    reference: &mut Renderer,
    app: &App,
    countdown: &igui::input::startup::StartupInputTest,
) -> PaintUpdate {
    let update = frame.update_with_countdown(app, None, None, Some(countdown)).unwrap();
    let mut full = Surface::new(WIDTH, HEIGHT);
    reference.draw(app, None, &mut full).unwrap();
    igui::render::draw_input_test_countdown(countdown, &mut full).unwrap();
    assert_same(frame.surface(), &full);
    update
}

#[test]
fn startup_countdown_only_changes_its_text_row_and_keeps_menu_retained() {
    use igui::input::startup::{StartupInputTest, Transition};
    for language in ["en", "ru"] {
        for search in [false, true] {
            let mut app = App::new(sample_catalogue());
            app.set_locales(crate::common::locales(language));
            if search {
                crate::common::top_tab(&mut app, igui::model::TopTab::Search);
            }
            let mut frame = RetainedFrame::new(Surface::new(WIDTH, HEIGHT), crate::common::renderer());
            let mut reference = crate::common::renderer();
            let mut countdown = StartupInputTest::default();
            countdown.update(0, true, false, false);
            assert!(compare_countdown(&mut frame, &mut reference, &app, &countdown).full);
            countdown.update(10_000, true, false, false);
            assert_eq!(compare_countdown(&mut frame, &mut reference, &app, &countdown), PaintUpdate {
                dialog: true, ..Default::default()
            });

            for ms in (11_000..20_000).step_by(1_000) {
                let before = frame.surface().pixels().to_vec();
                countdown.update(ms, true, false, false);

                assert_eq!(compare_countdown(&mut frame, &mut reference, &app, &countdown), PaintUpdate {
                    dialog: true, ..Default::default()
                });
                // Two-line modal starts at y=292; only its first message row changes.
                for (index_item, (&first_item, &second_item)) in before.iter().zip(frame.surface().pixels()).enumerate() {
                    if first_item != second_item {
                        assert!((201..1079).contains(&(index_item % WIDTH as usize))
                            && (348..368).contains(&(index_item / WIDTH as usize)));
                    }
                }
                assert!(!compare_countdown(&mut frame, &mut reference, &app, &countdown).changed());
                app.set_memory(Some(MemoryInfo {
                    available_bytes: ms * 10_000,
                    total_bytes: 256_000_000,
                }));
                assert_eq!(compare_countdown(&mut frame, &mut reference, &app, &countdown), PaintUpdate {
                    telemetry: true, ..Default::default()
                });
            }

            let before = frame.surface().pixels().to_vec();
            assert_eq!(countdown.update(20_000, true, true, true), Transition::Cancelled);
            assert_eq!(compare_countdown(&mut frame, &mut reference, &app, &countdown), PaintUpdate {
                dialog: true, ..Default::default()
            });
            for (index_item, (&first_item, &second_item)) in before.iter().zip(frame.surface().pixels()).enumerate() {
                if first_item != second_item {
                    assert!((200..1088).contains(&(index_item % WIDTH as usize))
                        && (292..436).contains(&(index_item / WIDTH as usize)));
                }
            }
            assert!(!compare_countdown(&mut frame, &mut reference, &app, &countdown).changed());
            assert_eq!(frame.statistics().full, 1);
        }
    }
}
