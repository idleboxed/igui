
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use {
    igui::locales::Locale,
    igui::locales::Locales,
    crate::common::sample_catalogue,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Focus,
    igui::render::layout::card as card,
    igui::render::Renderer,
    igui::render::HEIGHT,
    igui::render::WIDTH,
    imanlib::surface::Surface,
};

fn picture(app: &App, notice: Option<&str>) -> Vec<u32> {
    let mut surface = Surface::new(WIDTH, HEIGHT);
    crate::common::renderer().draw(app, notice, &mut surface).unwrap();
    surface.pixels().to_vec()
}

#[test]
fn region_focus_moves_the_light_border_between_columns_and_header() {
    let mut app = App::new(sample_catalogue());
    let border = 0xe6eae7;
    let pixel = |pixels: &[u32], x, y| pixels[y * WIDTH as usize + x];
    let left = picture(&app, None);
    assert_eq!(pixel(&left, 16, 200), border);
    assert_ne!(pixel(&left, 15, 200), border);
    assert_ne!(pixel(&left, 17, 200), border);
    assert_ne!(pixel(&left, 644, 200), border);
    app.dispatch(Right);
    let right = picture(&app, None);
    assert_ne!(pixel(&right, 16, 200), border);
    assert_eq!(pixel(&right, 644, 200), border);
    assert_ne!(pixel(&right, 643, 200), border);
    assert_ne!(pixel(&right, 645, 200), border);
    app.dispatch(Up);
    let header = picture(&app, None);
    assert_ne!(pixel(&header, 644, 200), border);
    assert_eq!(pixel(&header, 16, 30), border);
    assert_ne!(pixel(&header, 15, 30), border);
    assert_ne!(pixel(&header, 17, 30), border);
    assert_eq!(pixel(&header, 500, 16), border);
    assert_ne!(pixel(&header, 500, 15), border);
    assert_ne!(pixel(&header, 500, 17), border);
}

#[test]
fn header_shows_package_version_and_build_timestamp_in_both_languages() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    let timestamp = env!("IGUI_BUILD_UTC");
    assert_eq!(timestamp.len(), 12);
    assert!(timestamp.bytes().all(|byte| byte.is_ascii_digit()));
    for (part, minimum, maximum) in [
        (&timestamp[0..4], 1970, 9999),
        (&timestamp[4..6], 1, 12),
        (&timestamp[6..8], 1, 31),
        (&timestamp[8..10], 0, 23),
        (&timestamp[10..12], 0, 59),
    ] {
        assert!((minimum..=maximum).contains(&part.parse::<u32>().unwrap()));
    }

    let label = format!("idle gui / {}-{timestamp}", env!("CARGO_PKG_VERSION"));
    let width = label.len() * 7;
    assert!(92 + width < 1216);
    let mut expected = Surface::new(width as u32, 13);
    expected.clear(Rgb888::new(23, 32, 42)).unwrap();
    Text::with_baseline(
        &label,
        Point::zero(),
        MonoTextStyle::new(&FONT_7X13, Rgb888::new(151, 169, 174)),
        Baseline::Top,
    )
    .draw(&mut expected)
    .unwrap();

    for language in ["en", "ru"] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        for action in [None, Some(Right), Some(Up)] {
            if let Some(action) = action {
                app.dispatch(action);
            }
            let pixels = picture(&app, None);
            let actual: Vec<_> = pixels
                .chunks(WIDTH as usize)
                .skip(33)
                .take(13)
                .flat_map(|row| row[92..92 + width].iter().copied())
                .collect();
            assert_eq!(actual, expected.pixels());
        }
    }
}

#[test]
fn header_has_no_language_controls_in_either_language() {
    for language in ["en", "ru"] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        for action in [None, Some(Right), Some(Up), Some(Left), Some(Right)] {
            if let Some(action) = action {
                app.dispatch(action);
            }
            let pixels = picture(&app, None);
            for row in pixels.chunks(WIDTH as usize).skip(20).take(44) {
                assert!(row[810..1216].iter().all(|&pixel| pixel == 0x17202a));
            }
        }
    }
}

#[test]
fn only_platform_buttons_have_a_lighter_fill_and_controls_are_half_width() {
    for language in ["en", "ru"] {
        for games in [vec![], sample_catalogue()] {
            let mut app = App::new(games);
            app.set_locales(crate::common::locales(language));
            for action in [None, Some(Filters), Some(Activate)] {
                if let Some(action) = action {
                    app.dispatch(action);
                }
                let pixels = picture(&app, None);
                let pixel = |x, y| pixels[y * WIDTH as usize + x];

                for y in [90, 91, 153, 154, 155] {
                    for x in 82..620 {
                        assert_eq!(pixel(x, y), 0x17202a);
                    }
                }
                // 40px controls and 9px gaps; only system buttons remain 80px wide.
                for (index, x) in [84, 133, 182].into_iter().enumerate() {
                    let background = if index == 0 && app.search().is_some() {
                        0x224841
                    } else {
                        0x121a23
                    };
                    for col in x + 3..x + 37 {
                        assert_eq!(pixel(col, 119), background);
                    }
                    assert_eq!(pixel(x + 43, 119), 0x17202a);
                }
                if app.platforms().is_empty() {
                    assert_eq!(pixel(235, 119), 0x17202a);
                } else {
                    for col in 231..311 {
                        assert_eq!(pixel(col, 119), 0x1f2b37);
                    }
                    assert_eq!(pixel(315, 119), 0x17202a);
                }
            }
        }
    }
}

#[test]
fn platform_overflow_indicators_follow_hidden_tabs_in_both_directions() {
    let games = (0..8)
        .map(|i| crate::common::game(&format!("{i}.rom"), &format!("Game {i}"), &format!("P{i}")))
        .collect();
    let mut app = App::new(games);
    app.dispatch(Up);
    let indicators = |app: &App| {
        let pixels = picture(app, None);
        let lit = |x| {
            (94..107).any(|y| {
                pixels[y * WIDTH as usize + x..y * WIDTH as usize + x + 7].contains(&0x77cfb4)
            })
        };
        (lit(585), lit(607))
    };
    assert_eq!(indicators(&app), (false, true));
    for _ in 0..5 {
        app.dispatch(Right);
    }
    assert_eq!(indicators(&app), (true, true));
    for _ in 0..4 {
        app.dispatch(Right);
    }
    assert_eq!(indicators(&app), (true, false));
    app.dispatch(Right); // Last platform.
    app.dispatch(Right); // Wrap to the search tab.
    assert_eq!(indicators(&app), (false, true));
    for count in [0, 1, 4] {
        let games = (0..count)
            .map(|i| crate::common::game(&i.to_string(), "Game", &format!("P{i}")))
            .collect();
        assert_eq!(indicators(&App::new(games)), (false, false));
    }
}

#[test]
fn full_width_footer_has_localized_context_hints_even_without_a_game() {
    for language in ["en", "ru"] {
        let mut hints = Vec::new();
        for actions in [
            vec![],
            vec![Left],
            vec![Up],
            vec![Right],
            vec![Right, Up],
            vec![LowerFilters],
            vec![LowerFilters, Activate],
        ] {
            let mut app = App::new(sample_catalogue());
            app.set_locales(crate::common::locales(language));
            for action in actions {
                app.dispatch(action);
            }
            let pixels = picture(&app, None);
            assert_eq!(pixels[684 * WIDTH as usize], 0x17202a);
            assert_eq!(*pixels.last().unwrap(), 0x17202a);
            let hint: Vec<_> = pixels
                .chunks(WIDTH as usize)
                .skip(696)
                .take(13)
                .flat_map(|row| row[24..1043].iter().copied())
                .collect();
            assert!(hint.contains(&0x97a9ae));
            assert!(!hints.contains(&hint));
            hints.push(hint);
        }
    }
    let pixels = picture(&App::new(vec![]), None);
    assert!(pixels
        .chunks(WIDTH as usize)
        .skip(696)
        .take(13)
        .any(|row| row[24..1043].contains(&0x97a9ae)));
}

#[test]
fn footer_draws_four_arrow_glyphs_with_fixed_width_in_both_languages() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    for (language, prefix) in [
        ("en", "↑/↓ games | ←/→ panels"),
        ("ru", "↑/↓ игры | ←/→ блоки"),
    ] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        let pixels = picture(&app, None);
        let width = prefix.chars().count() * 7;
        let mut expected = Surface::new(width as u32, 13);
        expected.clear(Rgb888::new(23, 32, 42)).unwrap();
        let color = Rgb888::new(151, 169, 174);
        for (cell, symbol) in prefix.chars().enumerate() {
            let rows = match symbol {
                '↑' => [
                    "...#...", "..###..", ".#.#.#.", "...#...", "...#...", "...#...", "...#...",
                ],
                '↓' => [
                    "...#...", "...#...", "...#...", "...#...", ".#.#.#.", "..###..", "...#...",
                ],
                '←' => [
                    "...#...", "..#....", ".#.....", "#######", ".#.....", "..#....", "...#...",
                ],
                '→' => [
                    "...#...", "....#..", ".....#.", "#######", ".....#.", "....#..", "...#...",
                ],
                _ => {
                    Text::with_baseline(
                        &symbol.to_string(),
                        Point::new(cell as i32 * 7, 0),
                        MonoTextStyle::new(&FONT_7X13, color),
                        Baseline::Top,
                    )
                    .draw(&mut expected)
                    .unwrap();
                    continue;
                }
            };
            for (row, bits) in rows.iter().enumerate() {
                for (col, bit) in bits.bytes().enumerate() {
                    if bit == b'#' {
                        Pixel(Point::new((cell * 7 + col) as i32, 3 + row as i32), color)
                            .draw(&mut expected)
                            .unwrap();
                    }
                }
            }
        }
        let actual: Vec<_> = pixels
            .chunks(WIDTH as usize)
            .skip(696)
            .take(13)
            .flat_map(|row| row[24..24 + width].iter().copied())
            .collect();
        assert_eq!(actual, expected.pixels(), "{language}");
    }
}

#[test]
fn game_card_omits_platform_without_changing_filters_or_catalogue() {
    let mut expected = None;
    for alias in ["NES", "MD", "SNES", "CUSTOM"] {
        let app = App::new(vec![crate::common::game("game.rom", "Game", alias)]);
        assert_eq!(app.platforms(), &[alias]);
        assert_eq!(app.selected_game().unwrap().platform, alias);
        let pixels = picture(&app, None);
        let actual: Vec<_> = pixels
            .chunks(WIDTH as usize)
            .skip(card::TOP as usize)
            .take(card::HEIGHT as usize)
            .flat_map(|row| row[656..1252].iter().copied())
            .collect();
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
    }
}

#[test]
fn game_card_places_localized_genre_and_region_on_the_same_row() {
    use embedded_graphics::{
        mono_font::{
            iso_8859_5::{FONT_10X20, FONT_7X13},
            MonoTextStyle,
        },
        text::{Baseline, Text},
    };
    for (language, label) in [("en", "Region"), ("ru", "Регион")] {
        let mut unknown = None;
        for (region, en, ru) in [
            (None, "?", "?"),
            (Some(vec![]), "-", "-"),
            (Some(vec!["Europe"]), "Europe", "Европа"),
            (
                Some(vec!["Japan", "USA", "Europe"]),
                "Japan, USA, Europe",
                "Япония, США, Европа",
            ),
            (Some(vec![" Japan ", " USA "]), "Japan, USA", "Япония, США"),
            (
                Some(vec!["Custom market"]),
                "Custom market",
                "Custom market",
            ),
            (
                Some(vec!["A very long region name"]),
                "A very long region name",
                "A very long region name",
            ),
        ] {
            let mut game = crate::common::game("game.bin", "Game", "SNES");
            game.genre = Some("Racing".into());
            game.region = region
                .as_ref()
                .map(|regions| regions.iter().map(|region| (*region).to_owned()).collect());
            let mut app = App::new(vec![game]);
            app.set_locales(crate::common::locales(language));
            let pixels = picture(&app, None);
            if let Some(baseline) = &unknown {
                for (index, (before, after)) in pixels.iter().zip(baseline).enumerate() {
                    if before != after {
                        let region_x = card::REGION_X as usize;
                        assert!((region_x..region_x + card::GENRE_REGION_WIDTH as usize)
                            .contains(&(index % WIDTH as usize)));
                        let value_y = (card::GENRE_LABEL_Y + card::VALUE_OFFSET) as usize;
                        assert!((value_y..value_y + 20).contains(&(index / WIDTH as usize)));
                    }
                }
            } else {
                unknown = Some(pixels.clone());
            }
            let height = card::VALUE_OFFSET as u32 + 20;
            let width = 2 * card::GENRE_REGION_WIDTH + card::GENRE_REGION_GAP;
            let region_offset = card::REGION_X - card::GENRE_X;
            let mut expected = Surface::new(width, height);
            expected.clear(Rgb888::new(18, 26, 35)).unwrap();
            for (value, x, y, font, color) in [
                (
                    app.text("Genre"),
                    0,
                    0,
                    &FONT_7X13,
                    Rgb888::new(151, 169, 174),
                ),
                (
                    app.text("Racing"),
                    0,
                    card::VALUE_OFFSET,
                    &FONT_10X20,
                    Rgb888::new(230, 234, 231),
                ),
                (
                    label,
                    region_offset,
                    0,
                    &FONT_7X13,
                    Rgb888::new(151, 169, 174),
                ),
                (
                    if language == "ru" { ru } else { en },
                    region_offset,
                    card::VALUE_OFFSET,
                    &FONT_10X20,
                    Rgb888::new(230, 234, 231),
                ),
            ] {
                Text::with_baseline(
                    value,
                    Point::new(x, y),
                    MonoTextStyle::new(font, color),
                    Baseline::Top,
                )
                .draw(&mut expected)
                .unwrap();
            }
            let actual: Vec<_> = pixels
                .chunks(WIDTH as usize)
                .skip(card::GENRE_LABEL_Y as usize)
                .take(height as usize)
                .flat_map(|row| {
                    row[card::GENRE_X as usize..card::GENRE_X as usize + width as usize]
                        .iter()
                        .copied()
                })
                .collect();
            assert_eq!(actual, expected.pixels(), "{language}: {region:?}");
        }
    }
}

#[test]
fn game_card_genre_and_region_have_equal_width_and_clip_long_values() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        text::{Baseline, Text},
    };
    let mut game = crate::common::game("game.bin", "Game", "MD");
    let long_value = "1234567890".repeat(6);
    game.genre = Some(long_value.clone());
    game.region = Some(vec![long_value]);
    let app = App::new(vec![game]);

    let pixels = picture(&app, None);

    let mut expected = Surface::new(card::GENRE_REGION_WIDTH, 20);
    expected.clear(Rgb888::new(18, 26, 35)).unwrap();
    Text::with_baseline(
        "123456789012345678901234...",
        Point::zero(),
        MonoTextStyle::new(&FONT_10X20, Rgb888::new(230, 234, 231)),
        Baseline::Top,
    )
    .draw(&mut expected)
    .unwrap();
    let value_y = (card::GENRE_LABEL_Y + card::VALUE_OFFSET) as usize;
    for x in [card::GENRE_X as usize, card::REGION_X as usize] {
        let actual: Vec<_> = pixels
            .chunks(WIDTH as usize)
            .skip(value_y)
            .take(20)
            .flat_map(|row| {
                row[x..x + card::GENRE_REGION_WIDTH as usize]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(actual, expected.pixels());
    }
    for row in pixels.chunks(WIDTH as usize).skip(value_y).take(20) {
        let gap_start = card::GENRE_X as usize + card::GENRE_REGION_WIDTH as usize;
        assert!(row[gap_start..card::REGION_X as usize]
            .iter()
            .all(|&pixel| pixel == 0x121a23));
    }
}

#[test]
fn missing_image_label_is_localized_and_centered() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        text::{Baseline, Text},
    };
    let mut app = App::new(vec![crate::common::game("game.rom", "Game", "MD")]);
    for (language, label) in [("en", "No image"), ("ru", "Нет изображения")] {
        app.set_locales(crate::common::locales(language));
        let pixels = picture(&app, None);
        let mut expected = Surface::new(596, 20);
        expected.clear(Rgb888::new(18, 26, 35)).unwrap();
        Text::with_baseline(
            label,
            Point::new((596 - label.chars().count() as i32 * 10) / 2, 0),
            MonoTextStyle::new(&FONT_10X20, Rgb888::new(151, 169, 174)),
            Baseline::Top,
        )
        .draw(&mut expected)
        .unwrap();
        let actual: Vec<_> = pixels
            .chunks(WIDTH as usize)
            .skip(card::PREVIEW_LABEL_Y as usize)
            .take(20)
            .flat_map(|row| row[656..1252].iter().copied())
            .collect();
        assert_eq!(actual, expected.pixels(), "{language}");
    }
}

#[test]
fn screenshot_has_no_demo_caption_below_it() {
    let pixels = picture(&App::new(sample_catalogue()), None);
    for row in pixels
        .chunks(WIDTH as usize)
        .skip(card::PREVIEW_BOTTOM as usize)
        .take((card::TOP - card::PREVIEW_BOTTOM) as usize)
    {
        assert!(row[672..1236].iter().all(|&pixel| pixel == 0x17202a));
    }
}

#[test]
fn description_scrolling_changes_only_its_viewport_and_scrollbar() {
    let mut app = App::new(sample_catalogue());
    app.dispatch(Right);
    assert!(app.description_max_offset() > 0);
    let first = picture(&app, None);
    app.dispatch(Down);
    let second = picture(&app, None);
    let changed: Vec<_> = first
        .iter()
        .zip(&second)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert!(!changed.is_empty());
    for i in changed {
        assert!((676..1240).contains(&(i % WIDTH as usize)));
        let top = card::DESCRIPTION_Y as usize;
        assert!((top..top + card::DESCRIPTION_HEIGHT as usize).contains(&(i / WIDTH as usize)));
    }
}

#[test]
fn image_filename_is_on_the_players_row() {
    let first = picture(
        &App::new(vec![crate::common::game("first.gg", "Game", "GG")]),
        None,
    );
    let second = picture(
        &App::new(vec![crate::common::game("second.gg", "Game", "GG")]),
        None,
    );
    let changed: Vec<_> = first
        .iter()
        .zip(&second)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert!(!changed.is_empty());
    for i in changed {
        assert!((916..1224).contains(&(i % WIDTH as usize)));
        let value_y = (card::FACTS_LABEL_Y + card::VALUE_OFFSET) as usize;
        assert!((value_y..value_y + 13).contains(&(i / WIDTH as usize)));
    }
}

#[test]
fn image_field_uses_small_font_is_localized_clipped_and_absent_below_description() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    for language in ["en", "ru"] {
        let filename = format!("{}.gg", "1234567890".repeat(6));
        let mut game = crate::common::game("game.gg", "Game", "GG");
        game.image = filename;
        let mut app = App::new(vec![game]);
        app.set_locales(crate::common::locales(language));
        let pixels = picture(&app, None);
        let height = card::VALUE_OFFSET as u32 + 13;
        let mut expected = Surface::new(324, height);
        expected.clear(Rgb888::new(18, 26, 35)).unwrap();
        for (value, y, font, color) in [
            (app.text("Image"), 0, &FONT_7X13, Rgb888::new(151, 169, 174)),
            (
                "12345678901234567890123456789012345678901...",
                card::VALUE_OFFSET,
                &FONT_7X13,
                Rgb888::new(230, 234, 231),
            ),
        ] {
            Text::with_baseline(
                value,
                Point::new(0, y),
                MonoTextStyle::new(font, color),
                Baseline::Top,
            )
            .draw(&mut expected)
            .unwrap();
        }
        let actual: Vec<_> = pixels
            .chunks(WIDTH as usize)
            .skip(card::FACTS_LABEL_Y as usize)
            .take(height as usize)
            .flat_map(|row| row[916..1240].iter().copied())
            .collect();
        assert_eq!(actual, expected.pixels(), "{language}");
        let empty_y = card::DESCRIPTION_Y as usize + 13;
        for row in pixels
            .chunks(WIDTH as usize)
            .skip(empty_y)
            .take(card::BOTTOM as usize - empty_y)
        {
            assert!(row[676..1240].iter().all(|&pixel| pixel == 0x121a23));
        }
    }
}

#[test]
fn game_card_keeps_labels_near_values_and_separates_rows() {
    let mut game = crate::common::game("game.bin", "Game", "MD");
    game.genre = Some("Racing".into());
    game.region = Some(vec!["Europe".into()]);
    game.year = Some(1994);
    game.players = Some(2);
    let mut app = App::new(vec![game]);

    for language in ["en", "ru"] {
        app.set_locales(crate::common::locales(language));
        let pixels = picture(&app, None);

        for label_y in [
            card::TITLE_LABEL_Y,
            card::GENRE_LABEL_Y,
            card::FACTS_LABEL_Y,
        ] {
            let gap_start = (label_y + card::VALUE_OFFSET + 20) as usize;
            for row in pixels
                .chunks(WIDTH as usize)
                .skip(gap_start)
                .take(card::ROW_GAP as usize)
            {
                assert!(
                    row[676..1230].iter().all(|&pixel| pixel == 0x121a23),
                    "{language}: gap starting at {gap_start}"
                );
            }
        }
        for label_y in [
            card::TITLE_LABEL_Y,
            card::GENRE_LABEL_Y,
            card::FACTS_LABEL_Y,
            card::DESCRIPTION_LABEL_Y,
        ] {
            for row in pixels
                .chunks(WIDTH as usize)
                .skip(label_y as usize + 13)
                .take(card::LABEL_VALUE_GAP as usize)
            {
                assert!(row[676..1230].iter().all(|&pixel| pixel == 0x121a23));
            }
        }
    }
}

#[test]
fn game_title_uses_the_accent_color() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        text::{Baseline, Text},
    };
    let app = App::new(vec![crate::common::game("game.bin", "Aladdin", "MD")]);
    let pixels = picture(&app, None);
    let mut expected = Surface::new(540, 20);
    expected.clear(Rgb888::new(18, 26, 35)).unwrap();
    Text::with_baseline(
        "Aladdin",
        Point::zero(),
        MonoTextStyle::new(&FONT_10X20, Rgb888::new(119, 207, 180)),
        Baseline::Top,
    )
    .draw(&mut expected)
    .unwrap();
    let actual: Vec<_> = pixels
        .chunks(WIDTH as usize)
        .skip((card::TITLE_LABEL_Y + card::VALUE_OFFSET) as usize)
        .take(20)
        .flat_map(|row| row[676..1216].iter().copied())
        .collect();
    assert_eq!(actual, expected.pixels());
}

#[test]
fn long_game_titles_stay_in_one_card_row_without_abbreviation() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        text::{Baseline, Text},
    };
    let baseline = picture(&App::new(vec![crate::common::game("game.bin", "Short", "MD")]), None);
    let value_y = (card::TITLE_LABEL_Y + card::VALUE_OFFSET) as usize;
    for (title, displayed) in [
        ("A".repeat(54), "A".repeat(54)),
        ("A".repeat(55), "A".repeat(54)),
        ("Д".repeat(160), "Д".repeat(54)),
    ] {
        let app = App::new(vec![crate::common::game("game.bin", &title, "MD")]);

        let pixels = picture(&app, None);
        let mut expected = Surface::new(540, 20);
        expected.clear(Rgb888::new(18, 26, 35)).unwrap();
        Text::with_baseline(
            &displayed,
            Point::zero(),
            MonoTextStyle::new(&FONT_10X20, Rgb888::new(119, 207, 180)),
            Baseline::Top,
        ).draw(&mut expected).unwrap();

        let actual: Vec<_> = pixels.chunks(WIDTH as usize)
            .skip(value_y).take(20)
            .flat_map(|row| row[676..1216].iter().copied()).collect();
        assert_eq!(actual, expected.pixels());
        for y in card::TOP as usize..card::BOTTOM as usize {
            for x in 656..1252 {
                if !(value_y..value_y + 20).contains(&y) || !(676..1216).contains(&x) {
                    let i = y * WIDTH as usize + x;
                    assert_eq!(pixels[i], baseline[i]);
                }
            }
        }
    }
}

#[test]
fn free_space_changes_only_the_right_side_of_the_footer() {
    let mut app = App::new(sample_catalogue());
    app.set_memory(Some(igui::memory::MemoryInfo {
        available_bytes: 109_496 * 1024,
        total_bytes: 248_576 * 1024,
    }));
    app.set_locales(Locales::new(Locale::parse("ru", b"{}").unwrap()));
    let unknown = picture(&app, None);
    app.set_available_bytes(Some(12_300_000_000));
    let known = picture(&app, None);
    let changed: Vec<_> = unknown
        .iter()
        .zip(&known)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert!(!changed.is_empty());
    for i in changed {
        assert!((igui::render::FOOTER_STATS_X as usize..1256).contains(&(i % WIDTH as usize)));
        assert!((696..709).contains(&(i / WIDTH as usize)));
    }
    app.set_available_bytes(Some(0));
    assert_ne!(unknown, picture(&app, None));
}

#[test]
fn favourites_follow_search_and_have_an_independent_active_fill() {
    let mut app = App::new(sample_catalogue());
    app.dispatch(Up);
    app.dispatch(Right);
    let before = picture(&app, None);
    assert_eq!(before[120 * WIDTH as usize + 139], 0x121a23);
    assert_eq!(before[125 * WIDTH as usize + 152], 0x97a9ae); // star tip
    app.dispatch(Activate);
    let after = picture(&app, None);
    assert_eq!(after[120 * WIDTH as usize + 139], 0x224841);
    assert_eq!(after[125 * WIDTH as usize + 152], 0x77cfb4);
    assert!(app.favourites_only());
}

#[test]
fn favourite_star_is_confined_to_the_right_edge_and_clears_when_removed() {
    let mut app = App::new(sample_catalogue());
    let first = picture(&app, None);
    app.set_favourites([app.selected_game().unwrap().hash.clone()].into());
    let starred = picture(&app, None);
    let changed: Vec<_> = first
        .iter()
        .zip(&starred)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert!(changed.len() > 30);
    for i in changed {
        assert!((590..603).contains(&(i % WIDTH as usize)));
        assert!((186..199).contains(&(i / WIDTH as usize)));
    }
    app.set_favourites(Default::default());
    assert_eq!(first, picture(&app, None));
}

#[test]
fn external_previews_are_requested_only_for_selected_games_and_cached() {
    use igui::model::Preview;
    use std::{cell::RefCell, rc::Rc};
    let mut games = vec![
        crate::common::game("a.nes", "Alpha", "NES"),
        crate::common::game("b.nes", "Beta", "NES"),
    ];
    for game in &mut games {
        game.preview = Preview::Hash(game.hash.clone());
    }
    let mut app = App::new(games);
    let seen = Rc::new(RefCell::new(vec![]));
    let output = seen.clone();
    let mut renderer = Renderer::with_previews(move |name| {
        output.borrow_mut().push(name.to_owned());
        None
    });
    assert!(seen.borrow().is_empty());
    renderer.prepare_selected(&app);
    assert_eq!(*seen.borrow(), [crate::common::hash("a.nes")]);
    let mut surface = Surface::new(WIDTH, HEIGHT);
    renderer.draw(&app, None, &mut surface).unwrap();
    renderer.draw(&app, None, &mut surface).unwrap();
    assert_eq!(*seen.borrow(), [crate::common::hash("a.nes")]);
    app.dispatch(Down);
    renderer.draw(&app, None, &mut surface).unwrap();
    assert_eq!(
        *seen.borrow(),
        [crate::common::hash("a.nes"), crate::common::hash("b.nes")]
    );
}

#[test]
fn renderer_is_deterministic_and_selection_updates_right_column() {
    let mut app = App::new(sample_catalogue());
    let first = picture(&app, None);
    assert_eq!(first, picture(&app, None));
    app.dispatch(Down);
    let second = picture(&app, None);
    let right = |pixels: &[u32]| {
        pixels
            .chunks(WIDTH as usize)
            .skip(90)
            .take(610)
            .flat_map(|row| row[650..1250].iter().copied())
            .collect::<Vec<_>>()
    };
    assert_ne!(right(&first), right(&second));
}

#[test]
fn empty_result_clears_previous_details_even_with_cached_image() {
    let mut app = App::new(sample_catalogue());
    let mut renderer = crate::common::renderer();
    let mut surface = Surface::new(WIDTH, HEIGHT);
    renderer.draw(&app, None, &mut surface).unwrap();
    for action in [Up, Right, Activate, Down, Left, Down, Activate, Right] {
        app.dispatch(action);
    }
    assert_eq!(app.visible_count(), 0); // B + GB
    assert_eq!(app.focus(), Focus::Games);
    renderer.draw(&app, None, &mut surface).unwrap();
    assert_eq!(surface.pixels(), picture(&app, None));
    let row = &surface.pixels()[500 * WIDTH as usize..501 * WIDTH as usize];
    assert!(row[670..1230].windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn all_sample_games_and_focus_regions_render() {
    let mut app = App::new(sample_catalogue());
    let mut renderer = crate::common::renderer();
    let mut surface = Surface::new(WIDTH, HEIGHT);
    for _ in 0..app.total_count() {
        renderer.draw(&app, None, &mut surface).unwrap();
        app.dispatch(Down);
    }
    renderer.draw(&app, None, &mut surface).unwrap();
    app.dispatch(Up);
    app.dispatch(Left);
    for _ in 0..app.letters().len() {
        renderer.draw(&app, None, &mut surface).unwrap();
        app.dispatch(Down);
    }
    renderer
        .draw(&app, Some("Тестовая заглушка"), &mut surface)
        .unwrap();
}

#[test]
fn dropup_is_clipped_to_left_column_and_scrolls_to_all_genres() {
    let games = (0..12)
        .map(|i| {
            let mut game = crate::common::game(&format!("{i}.rom"), &format!("Game {i:02}"), "MD");
            game.genre = Some(format!("Genre {i:02}"));
            game
        })
        .collect();
    let mut app = App::new(games);
    app.dispatch(LowerFilters);
    app.dispatch(Right);
    let closed = picture(&app, None);
    app.dispatch(Activate);
    let open = picture(&app, None);
    assert_ne!(open, closed);
    for (i, (a, b)) in open.iter().zip(&closed).enumerate() {
        if a != b {
            let (x, y) = (i % WIDTH as usize, i / WIDTH as usize);
            assert!(
                ((270..612).contains(&x) && (412..592).contains(&y)) || (696..709).contains(&y)
            );
        }
    }
    app.dispatch(Up); // Last genre, wrapping from All.
    assert_eq!(app.option_cursor(), 12);
    let last = picture(&app, None);
    assert_ne!(open, last);
    app.dispatch(Favourite);
    assert_eq!(closed, picture(&app, None));
}

#[test]
fn hash_is_not_displayed_anywhere_in_game_card() {
    let game = crate::common::game("same.bin", "Same", "MD");
    let first = picture(&App::new(vec![game.clone()]), None);
    let mut changed = game;
    changed.hash = crate::common::hash("new-content");
    assert_eq!(first, picture(&App::new(vec![changed]), None));
}

#[test]
fn launch_dialog_places_full_system_name_above_the_localized_loading_message() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        text::{Baseline, Text},
    };
    use igui::{runtime::launch::LaunchDialog, render::draw_launch_dialog};
    // The dialog owns the launch target; it must not read another selected game.
    let dialog = LaunchDialog::new("Aladdin".into(), "MD".into());
    let mut app = App::new(vec![crate::common::game("other.nes", "Other game", "NES")]);
    for (language, message) in [
        ("en", "Loading Aladdin ..."),
        ("ru", "Загружаю Aladdin ..."),
    ] {
        app.set_locales(crate::common::locales(language));
        let mut surface = Surface::new(WIDTH, HEIGHT);
        crate::common::renderer().draw(&app, None, &mut surface).unwrap();
        draw_launch_dialog(&app, &dialog, &mut surface).unwrap();
        for (value, y, color) in [
            ("Sega Mega Drive / Genesis", 336, Rgb888::new(230, 234, 231)),
            (message, 360, Rgb888::new(119, 207, 180)),
        ] {
            let mut expected = Surface::new(878, 20);
            expected.clear(Rgb888::new(23, 32, 42)).unwrap();
            Text::with_baseline(
                value,
                Point::new((878 - value.chars().count() as i32 * 10) / 2, 0),
                MonoTextStyle::new(&FONT_10X20, color),
                Baseline::Top,
            )
            .draw(&mut expected)
            .unwrap();
            let actual: Vec<_> = surface
                .pixels()
                .chunks(WIDTH as usize)
                .skip(y)
                .take(20)
                .flat_map(|row| row[201..1079].iter().copied())
                .collect();
            assert_eq!(actual, expected.pixels(), "{language}, row at {y}");
        }
    }
}

#[test]
fn trash_dialog_shows_localized_question_and_a_b_answers() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
        text::{Baseline, Text},
    };
    let mut app = App::new(sample_catalogue());
    for (language, question, answers) in [
        ("en", "Move to trash?", "A: yes | B: no"),
        ("ru", "Переместить в корзину?", "A: да | B: нет"),
    ] {
        app.set_locales(crate::common::locales(language));
        let mut surface = Surface::new(WIDTH, HEIGHT);
        crate::common::renderer().draw(&app, None, &mut surface).unwrap();
        igui::render::draw_trash_dialog(&app, &mut surface).unwrap();
        for (value, y, color) in [
            (question, 336, Rgb888::new(230, 234, 231)),
            (answers, 360, Rgb888::new(119, 207, 180)),
        ] {
            let mut expected = Surface::new(878, 20);
            expected.clear(Rgb888::new(23, 32, 42)).unwrap();
            Text::with_baseline(
                value,
                Point::new((878 - value.chars().count() as i32 * 10) / 2, 0),
                MonoTextStyle::new(&FONT_10X20, color),
                Baseline::Top,
            )
            .draw(&mut expected)
            .unwrap();
            let actual: Vec<_> = surface.pixels().chunks(WIDTH as usize)
                .skip(y).take(20).flat_map(|row| row[201..1079].iter().copied()).collect();
            assert_eq!(actual, expected.pixels(), "{language}, row at {y}");
        }
    }
}

#[test]
fn launch_dialog_is_an_opaque_centered_panel_above_the_existing_menu() {
    use igui::{runtime::launch::LaunchDialog, render::draw_launch_dialog};
    let mut app = App::new(sample_catalogue());
    for language in ["en", "ru"] {
        app.set_locales(crate::common::locales(language));
        for title in ["Aladdin".to_owned(), "Длинное название игры ".repeat(100)]
        {
            let mut renderer = crate::common::renderer();
            let mut surface = Surface::new(WIDTH, HEIGHT);
            renderer.draw(&app, None, &mut surface).unwrap();
            let before = surface.pixels().to_vec();
            draw_launch_dialog(&app, &LaunchDialog::new(title, "MD".into()), &mut surface).unwrap();
            assert_ne!(before, surface.pixels());
            assert_eq!(surface.pixels()[360 * WIDTH as usize + 200], 0xe6eae7);
            assert_ne!(surface.pixels()[360 * WIDTH as usize + 201], 0xe6eae7);
            for (i, (a, b)) in before.iter().zip(surface.pixels()).enumerate() {
                if a != b {
                    assert!((200..1088).contains(&(i % WIDTH as usize)));
                    assert!((232..496).contains(&(i / WIDTH as usize)));
                }
            }
            renderer.draw(&app, None, &mut surface).unwrap();
            assert_eq!(before, surface.pixels()); // No remnants after dismissing.
        }
    }
}

fn numbered_catalogue(count: usize) -> App {
    App::new(
        (1..=count)
            .map(|i| crate::common::game(&format!("fixture-{i}"), &format!("Game {i:04}"), "MD"))
            .collect(),
    )
}

fn assert_row_number(pixels: &[u32], row: usize, number: usize, digits: usize, selected: bool) {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    let width = digits * 7;
    let mut expected = Surface::new(width as u32, 13);
    expected
        .clear(if selected {
            Rgb888::new(43, 61, 75)
        } else {
            Rgb888::new(18, 26, 35)
        })
        .unwrap();
    let significant = number.to_string();
    let padding = digits - significant.len();
    for (value, x, color) in [
        ("0".repeat(padding), 0, Rgb888::new(89, 108, 117)),
        (significant, padding as i32 * 7, Rgb888::new(89, 108, 117)),
    ] {
        Text::with_baseline(
            &value,
            Point::new(x, 0),
            MonoTextStyle::new(&FONT_7X13, color),
            Baseline::Top,
        )
        .draw(&mut expected)
        .unwrap();
    }
    let actual: Vec<_> = pixels
        .chunks(WIDTH as usize)
        .skip(186 + row * 34)
        .take(13)
        .flat_map(|line| line[104..104 + width].iter().copied())
        .collect();
    assert_eq!(
        actual,
        expected.pixels(),
        "row={row} number={number} digits={digits}"
    );
}

#[test]
fn row_numbers_use_fixed_width_and_dim_all_digits_even_when_selected() {
    for count in [1, 9, 10, 99, 100, 2000] {
        let app = numbered_catalogue(count);
        let pixels = picture(&app, None);
        let digits = count.to_string().len();
        for number in 1..=count.min(12) {
            assert_row_number(&pixels, number - 1, number, digits, number == 1);
        }
    }
}

#[test]
fn row_ordinals_continue_after_scrolling_and_restart_after_filtering() {
    let mut app = numbered_catalogue(2000);
    for _ in 0..100 {
        app.dispatch(Down);
    }
    let pixels = picture(&app, None);
    // Selected 101 is in the middle of the twelve visible rows: 95..106.
    for row in 0..12 {
        assert_row_number(&pixels, row, 95 + row, 4, row == 6);
    }
    let hash = app.selected_game().unwrap().hash.clone();
    app.set_favourites([hash].into());
    for action in [Filters, Right, Activate, Down] {
        app.dispatch(action);
    }
    assert_eq!(app.visible_count(), 1);
    assert_eq!(app.selected_game().unwrap().title, "Game 0101");
    assert_row_number(&picture(&app, None), 0, 1, 1, true);
}

#[test]
fn list_counter_is_below_lower_filters_not_in_platform_header() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    for count in [0, 1, 2000] {
        let mut app = numbered_catalogue(count);
        let label = format!("{:02} / {count:02}", usize::from(count != 0));
        let width = label.len() * 7;
        let mut expected = Surface::new(width as u32, 13);
        expected.clear(Rgb888::new(23, 32, 42)).unwrap();
        Text::with_baseline(
            &label,
            Point::zero(),
            MonoTextStyle::new(&FONT_7X13, Rgb888::new(151, 169, 174)),
            Baseline::Top,
        )
        .draw(&mut expected)
        .unwrap();
        for action in [LowerFilters, Activate] {
            app.dispatch(action);
            let pixels = picture(&app, None);
            let actual: Vec<_> = pixels
                .chunks(WIDTH as usize)
                .skip(650)
                .take(13)
                .flat_map(|line| line[612 - width..612].iter().copied())
                .collect();
            assert_eq!(actual, expected.pixels());
            for line in pixels.chunks(WIDTH as usize).skip(94).take(13) {
                assert!(line[450..566].iter().all(|&pixel| pixel == 0x17202a));
            }
        }
    }
}

#[test]
fn platform_header_uses_a_vertical_separator_in_both_languages() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    let mut app = numbered_catalogue(1);
    for (language, label) in [
        ("en", "Search | Favourites | Trash | Platforms"),
        ("ru", "Поиск | Избранное | Корзина | Платформы"),
    ] {
        app.set_locales(crate::common::locales(language));
        let width = label.chars().count() * 7;
        let mut expected = Surface::new(width as u32, 13);
        expected.clear(Rgb888::new(23, 32, 42)).unwrap();
        Text::with_baseline(
            label,
            Point::zero(),
            MonoTextStyle::new(&FONT_7X13, Rgb888::new(151, 169, 174)),
            Baseline::Top,
        )
        .draw(&mut expected)
        .unwrap();
        let pixels = picture(&app, None);
        let actual: Vec<_> = pixels
            .chunks(WIDTH as usize)
            .skip(94)
            .take(13)
            .flat_map(|line| line[86..86 + width].iter().copied())
            .collect();
        assert_eq!(actual, expected.pixels());
    }
}

#[test]
fn trash_button_is_next_to_the_star_and_stays_highlighted_without_focus() {
    let pixel = |pixels: &[u32], x: usize, y: usize| pixels[y * WIDTH as usize + x];
    let mut app = App::new(vec![crate::common::game("a", "Amber", "MD")]);
    app.set_trash([crate::common::hash("a")].into());
    for action in [Filters, Right, Right] {
        app.dispatch(action);
    }
    let before = picture(&app, None);
    assert_eq!(pixel(&before, 189, 120), 0x121a23);
    assert_eq!(pixel(&before, 199, 125), 0x97a9ae); // handle
    assert_eq!(pixel(&before, 195, 127), 0x97a9ae); // lid
    assert_eq!(pixel(&before, 152, 125), 0x97a9ae); // adjacent star
    app.dispatch(Activate);
    app.dispatch(Down);
    let active = picture(&app, None);
    assert_eq!(app.focus(), Focus::Games);
    assert!(app.trash_only());
    assert_eq!(pixel(&active, 189, 120), 0x224841);
    assert_eq!(pixel(&active, 199, 125), 0x77cfb4);
    assert_eq!(pixel(&active, 195, 127), 0x77cfb4);
    app.dispatch(Filters);
    app.dispatch(Activate);
    let inactive = picture(&app, None);
    assert_eq!(pixel(&inactive, 189, 120), 0x121a23);
    assert_eq!(pixel(&inactive, 199, 125), 0x97a9ae);
    assert_eq!(app.visible_count(), 0);
}

#[test]
fn memory_footer_is_localized_and_left_of_disk_including_empty_lists_and_read_errors() {
    use embedded_graphics::{
        mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
        text::{Baseline, Text},
    };
    use igui::memory::MemoryInfo;
    for (language, prefix, unit) in [("en", "RAM", "MiB"), ("ru", "RAM", "МиБ")] {
        let mut app = App::new(vec![]);
        app.set_locales(crate::common::locales(language));
        for (disk, disk_amount) in [
            (Some(12_300_000_000), "12.3"),
            (Some(1_234_500_000_000), "1234.5"),
            (None, "-"),
        ] {
            app.set_available_bytes(disk);
            let disk_label = format!("{}: {disk_amount} {}", app.text("Disk"), app.text("GB"));
            let disk_left = 1256 - disk_label.chars().count() * 7;
            for (memory, amount) in [
                (
                    Some(MemoryInfo {
                        available_bytes: 109_496 * 1024,
                        total_bytes: 248_576 * 1024,
                    }),
                    "106.9 / 242.8",
                ),
                (
                    Some(MemoryInfo {
                        available_bytes: 0,
                        total_bytes: 248_576 * 1024,
                    }),
                    "0.0 / 242.8",
                ),
                (None, "-"),
            ] {
                app.set_memory(memory);
                let label = format!("{prefix}: {amount} {unit}");
                let width = label.chars().count() * 7;
                let left = disk_left - 16 - width;
                let mut expected = Surface::new(width as u32, 13);
                expected.clear(Rgb888::new(23, 32, 42)).unwrap();
                Text::with_baseline(
                    &label,
                    Point::zero(),
                    MonoTextStyle::new(&FONT_7X13, Rgb888::new(151, 169, 174)),
                    Baseline::Top,
                )
                .draw(&mut expected)
                .unwrap();
                let pixels = picture(&app, None);
                let actual: Vec<_> = pixels
                    .chunks(WIDTH as usize)
                    .skip(696)
                    .take(13)
                    .flat_map(|line| line[left..left + width].iter().copied())
                    .collect();
                assert_eq!(actual, expected.pixels(), "{language}: {label}");
                for row in pixels.chunks(WIDTH as usize).skip(696).take(13) {
                    assert!(row[left + width..disk_left]
                        .iter()
                        .all(|&color| color == 0x17202a));
                }
                assert!(pixels
                    .chunks(WIDTH as usize)
                    .skip(696)
                    .take(13)
                    .any(|row| row[disk_left..1256].contains(&0x97a9ae)));
            }
        }
    }
}

#[test]
fn memory_updates_only_its_reserved_cell_without_reflowing_hints_or_reading_previews_again() {
    use igui::{
        memory::MemoryInfo,
        render::{FOOTER_STATS_X, FOOTER_Y},
    };
    use std::{cell::Cell, rc::Rc};
    let reads = Rc::new(Cell::new(0));
    let counter = reads.clone();
    let mut renderer = Renderer::with_previews(move |_| {
        counter.set(counter.get() + 1);
        None
    });
    let mut game = crate::common::game("game", "Game", "MD");
    game.preview = igui::model::Preview::Hash(game.hash.clone());
    let mut app = App::new(vec![game]);
    let mut surface = Surface::new(WIDTH, HEIGHT);
    renderer.draw(&app, None, &mut surface).unwrap();
    let original = surface.pixels().to_vec();
    for memory in [
        Some(MemoryInfo {
            available_bytes: 109_496 * 1024,
            total_bytes: 248_576 * 1024,
        }),
        Some(MemoryInfo {
            available_bytes: u64::MAX,
            total_bytes: u64::MAX,
        }),
        None,
    ] {
        app.set_memory(memory);
        renderer.draw(&app, None, &mut surface).unwrap();
        for (i, (&before, &after)) in original.iter().zip(surface.pixels()).enumerate() {
            if before != after {
                // Default disk label is "Disk: - GB" (10 cells), followed by 16 px gap.
                assert!((FOOTER_STATS_X as usize..1170).contains(&(i % WIDTH as usize)));
                assert!((FOOTER_Y as usize + 12..FOOTER_Y as usize + 25)
                    .contains(&(i / WIDTH as usize)));
            }
        }
    }
    assert_eq!(reads.get(), 1);
}

#[test]
fn power_icon_stays_compact_with_a_red_circle_and_one_pixel_focus_frame() {
    let mut app = App::new(sample_catalogue());
    app.set_locales(Locales::new(Locale::parse("ru", b"{}").unwrap()));
    let normal = picture(&app, None);
    let pixel = |pixels: &[u32], x, y| pixels[y * WIDTH as usize + x];
    for x in 1186..1216 {
        assert_eq!(pixel(&normal, x, 42), 0x17202a);
    }
    assert_eq!(pixel(&normal, 1232, 35), 0xf05050);
    assert_eq!(pixel(&normal, 1232, 48), 0xf05050);
    for y in 30..54 {
        for x in 1216..1248 {
            if pixel(&normal, x, y) == 0xf05050 {
                assert!((1226..1238).contains(&x) && (35..49).contains(&y));
            }
        }
    }
    for action in [Right, Up] {
        app.dispatch(action);
    }
    let focused = picture(&app, None);
    assert_eq!(pixel(&focused, 1216, 40), 0xe6eae7);
    assert_eq!(pixel(&focused, 1247, 53), 0xe6eae7);
    assert_ne!(pixel(&focused, 1217, 40), 0xe6eae7);
}

#[test]
fn multiline_notice_stays_inside_its_panel_and_displays_the_last_line() {
    let app = App::new(sample_catalogue());
    let empty = picture(&app, Some(""));
    let filled = picture(&app, Some("First\nSecond\nThird\nFourth\nFifth\nLast"));
    let first_y = card::DESCRIPTION_Y as usize;
    let bottom = first_y + card::DESCRIPTION_HEIGHT as usize;
    let last_y = bottom - card::DESCRIPTION_LINE_HEIGHT as usize;
    let mut last_line_visible = false;

    for (index, (before, after)) in empty.iter().zip(&filled).enumerate() {
        if before != after {
            let x = index % WIDTH as usize;
            let y = index / WIDTH as usize;
            assert!((684..1236).contains(&x) && (first_y..bottom).contains(&y));
            last_line_visible |= y >= last_y;
        }
    }
    assert!(last_line_visible);
}

#[test]
fn error_notice_uses_a_muted_red_panel_in_card_search_and_empty_catalogue() {
    let notice = igui::render::notice::launch_failed("No such file or directory (os error 2)");
    let error_color = 0x4a3033;
    for language in ["en", "ru"] {
        for games in [sample_catalogue(), vec![]] {
            let mut app = App::new(games);
            app.set_locales(crate::common::locales(language));
            for search in [false, true] {
                if search {
                    crate::common::top_tab(&mut app, igui::model::TopTab::Search);
                }
                let plain = picture(&app, None);
                let pixels = picture(&app, Some(&notice));
                let panel_top = (card::DESCRIPTION_Y - 1) as usize;
                let panel_bottom = (card::DESCRIPTION_Y + card::DESCRIPTION_HEIGHT as i32) as usize;
                for y in panel_top..panel_bottom {
                    assert_eq!(pixels[y * WIDTH as usize + 674], error_color);
                }
                assert!(pixels.chunks(WIDTH as usize)
                    .skip(card::DESCRIPTION_Y as usize).take(card::DESCRIPTION_HEIGHT as usize)
                    .all(|row| !row[1236..1239].contains(&0x77cfb4)));
                // The third line (the useful advice) must not disappear in Search.
                let advice_y = card::DESCRIPTION_Y as usize + 12 + 2 * card::DESCRIPTION_LINE_HEIGHT as usize;
                assert!(pixels.chunks(WIDTH as usize).skip(advice_y).take(13)
                    .any(|row| row[684..1236].contains(&0xe6eae7)));
                for (index, (before, after)) in plain.iter().zip(&pixels).enumerate() {
                    if before != after {
                        let x = index % WIDTH as usize;
                        let y = index / WIDTH as usize;
                        // The description scrollbar at x=1236..1239 is cleared as well.
                        assert!((672..1239).contains(&x) && (panel_top..panel_bottom).contains(&y));
                    }
                }
            }
        }
    }
}
