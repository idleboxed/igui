
// SPDX-License-Identifier: BSD-3-Clause
use embedded_graphics::{
    mono_font::{iso_8859_5::FONT_7X13, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    text::{Baseline, Text},
};
use {
    igui::locales::Locale,
    igui::locales::Locales,
    crate::common::sample_catalogue,
    imanlib::igui::Action::*,
    igui::model::App,
    igui::model::Focus,
    igui::render::FOOTER_STATS_X,
    igui::render::FOOTER_Y,
    igui::render::HEIGHT,
    igui::render::WIDTH,
    imanlib::surface::Surface,
};

const HINT_X: usize = 24;
const HINT_Y: usize = FOOTER_Y as usize + 12;
const HINT_WIDTH: usize = FOOTER_STATS_X as usize - 48;
const GAMES_HINT: &str =
    "↑/↓ games | ←/→ panels | A/Start launch | B favourite | Select trash | Y/X filters";
const BUTTONS: [(char, u32, [&str; 9]); 4] = [
    (
        'X',
        0xc49be8,
        [
            "#########",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#########",
        ],
    ),
    (
        'B',
        0xf07878,
        [
            "...###...",
            "..#...#..",
            ".#.....#.",
            "#.......#",
            "#.......#",
            "#.......#",
            ".#.....#.",
            "..#...#..",
            "...###...",
        ],
    ),
    (
        'Y',
        0x79cf9a,
        [
            "....#....",
            "...#.#...",
            "...#.#...",
            "..#...#..",
            "..#...#..",
            ".#.....#.",
            ".#.....#.",
            "#.......#",
            "#########",
        ],
    ),
    (
        'A',
        0x78aef0,
        [
            "#.......#",
            ".#.....#.",
            "..#...#..",
            "...#.#...",
            "....#....",
            "...#.#...",
            "..#...#..",
            ".#.....#.",
            "#.......#",
        ],
    ),
];

fn frame(app: &App) -> Vec<u32> {
    let mut surface = Surface::new(WIDTH, HEIGHT);
    crate::common::renderer().draw(app, None, &mut surface).unwrap();
    surface.pixels().to_vec()
}

fn with_hint(value: &str) -> App {
    let mut app = App::new(vec![]);
    let dictionary = serde_json::to_vec(&serde_json::json!({ GAMES_HINT: value })).unwrap();
    app.set_locales(Locales::new(Locale::parse("zz", &dictionary).unwrap()));
    app
}

fn area(pixels: &[u32], position_x: usize, width: usize) -> Vec<u32> {
    pixels
        .chunks(WIDTH as usize)
        .skip(HINT_Y)
        .take(13)
        .flat_map(|row| row[position_x..position_x + width].iter().copied())
        .collect()
}

fn expected_surface(width: usize) -> Surface {
    let mut surface = Surface::new(width as u32, 13);
    surface.clear(Rgb888::new(23, 32, 42)).unwrap();
    surface
}

fn label(surface: &mut Surface, value: &str, position_x: usize) {
    Text::with_baseline(
        value,
        Point::new(position_x as i32, 0),
        MonoTextStyle::new(&FONT_7X13, Rgb888::new(151, 169, 174)),
        Baseline::Top,
    )
    .draw(surface)
    .unwrap();
}

fn pair(surface: &mut Surface, symbol: char, position_x: usize) {
    let (_, color, mask) = BUTTONS.iter().find(|&&(key, _, _)| key == symbol).unwrap();
    label(surface, &symbol.to_string(), position_x);
    let color = Rgb888::new((color >> 16) as u8, (color >> 8) as u8, *color as u8);
    for (row, bits) in mask.iter().enumerate() {
        for (col, bit) in bits.bytes().enumerate() {
            if bit == b'#' {
                Pixel(Point::new((position_x + 9 + col) as i32, (2 + row) as i32), color)
                    .draw(surface)
                    .unwrap();
            }
        }
    }
}

#[test]
fn button_shapes_and_neutral_letters_are_correct_in_every_context_and_language() {
    for empty in [false, true] {
        for language in ["en", "ru"] {
            for (actions, focus, keys) in [
                (vec![], Focus::Games, "ABYX"),
                (vec![Left], Focus::Alphabet, "AYX"),
                (vec![Filters], Focus::Platforms, "AX"),
                (vec![Right], Focus::Details, "YX"),
                (vec![Right, Up], Focus::Header, "AYX"),
                (vec![LowerFilters], Focus::LowerFilters, "AY"),
                (vec![LowerFilters, Activate], Focus::FilterOptions, "ABY"),
                (vec![Filters, Activate], Focus::SearchField, "XBY"),
                (vec![Filters, Activate, Down], Focus::SearchKeys, "AY"),
                (vec![Filters, Activate, Down, Left], Focus::Games, "ABYX"),
                (vec![Filters, Right, Right, Activate, Down], Focus::Games, "ABYX"),
            ] {
                let mut app = App::new(if empty { vec![] } else { sample_catalogue() });
                app.set_locales(crate::common::locales(language));
                for action in actions {
                    app.dispatch(action);
                }
                assert_eq!(app.focus(), focus);

                let pixels = frame(&app);

                for (key, color, _) in BUTTONS {
                    let positions: Vec<_> = pixels
                        .iter()
                        .enumerate()
                        .filter_map(|(index, &pixel)| {
                            (pixel == color).then_some((index % WIDTH as usize, index / WIDTH as usize))
                        })
                        .collect();
                    if !keys.contains(key) {
                        assert!(positions.is_empty(), "unexpected {key} in {focus:?}");
                        continue;
                    }
                    assert!(
                        !positions.is_empty(),
                        "missing {key} in {focus:?}, {language}"
                    );
                    assert!(positions.iter().all(|&(position_x, position_y)| {
                        (HINT_X..HINT_X + HINT_WIDTH).contains(&position_x)
                            && (HINT_Y + 2..HINT_Y + 11).contains(&position_y)
                    }));
                    let position_x = positions.iter().map(|&(position_x, _)| position_x).min().unwrap() - 9;
                    let mut expected = expected_surface(20);
                    pair(&mut expected, key, 0);
                    assert_eq!(area(&pixels, position_x, 20), expected.pixels(), "{key}, {language}");
                }
            }
        }
    }
}

#[test]
fn letters_inside_words_and_action_labels_are_not_decorated() {
    for value in [
        "Xylophone ABY",
        "Menu X/B Y/A",
        "Select X B Y A",
        "AB/XY action",
        "x/b/y/a action",
        "Игра X/B Y/A",
        "Menu title | Xylophone ABY",
    ] {
        let app = with_hint(value);

        let pixels = frame(&app);

        let mut expected = expected_surface(HINT_WIDTH);
        label(&mut expected, value, 0);
        assert_eq!(
            area(&pixels, HINT_X, HINT_WIDTH),
            expected.pixels(),
            "{value}"
        );
    }
}

#[test]
fn slash_combinations_keep_their_separators_and_decorate_only_whole_keys() {
    let pixels = frame(&with_hint("Y/A filters | X/AB choose | B cancel"));

    let mut expected = expected_surface(HINT_WIDTH);
    pair(&mut expected, 'Y', 0);
    label(&mut expected, "/", 20);
    pair(&mut expected, 'A', 27);
    label(&mut expected, " filters | ", 47);
    let position_x = 47 + " filters | ".len() * 7;
    pair(&mut expected, 'X', position_x);
    label(&mut expected, "/AB choose | ", position_x + 20);
    let position_x = position_x + 20 + "/AB choose | ".len() * 7;
    pair(&mut expected, 'B', position_x);
    label(&mut expected, " cancel", position_x + 20);
    assert_eq!(area(&pixels, HINT_X, HINT_WIDTH), expected.pixels());
}

#[test]
fn trash_and_restore_hints_show_only_select_without_its_keyboard_alias() {
    for (language, prefix, actions) in [
        (
            "en",
            "↑/↓ games | ←/→ panels | A/Start launch | B favourite | ",
            ["trash", "restore"],
        ),
        (
            "ru",
            "↑/↓ игры | ←/→ блоки | A/Start запуск | B избранное | ",
            ["корзина", "вернуть"],
        ),
    ] {
        let mut app = App::new(sample_catalogue());
        app.set_locales(crate::common::locales(language));
        for (trash, action) in actions.into_iter().enumerate() {
            if trash == 1 {
                for event in [Filters, Right, Right, Activate, Down] {
                    app.dispatch(event);
                }
            }
            assert_eq!(app.focus(), Focus::Games);
            assert_eq!(app.trash_only(), trash == 1);

            let pixels = frame(&app);

            let value = format!("Select {action} | ");
            let width = value.chars().count() * 7;
            let mut expected = expected_surface(width);
            label(&mut expected, &value, 0);
            // X and B before Select each add a 9px icon and two 2px gaps.
            let position_x = HINT_X + prefix.chars().count() * 7 + 2 * 13;
            assert_eq!(
                area(&pixels, position_x, width),
                expected.pixels(),
                "{language}: {action}"
            );
        }
    }
}

#[test]
fn truncation_keeps_button_pairs_whole_and_reserves_space_for_the_ellipsis() {
    for (count, fits) in [(95, true), (96, false)] {
        let prefix = format!("{} | ", "я".repeat(count));
        let pixels = frame(&with_hint(&format!("{prefix}X {}", "д".repeat(150))));

        let mut expected = expected_surface(HINT_WIDTH);
        label(&mut expected, &prefix, 0);
        let mut position_x = prefix.chars().count() * 7;
        if fits {
            pair(&mut expected, 'X', position_x);
            position_x += 20;
        }
        label(&mut expected, "...", position_x);
        assert_eq!(area(&pixels, HINT_X, HINT_WIDTH), expected.pixels());
        assert!(area(&pixels, HINT_X + HINT_WIDTH, 24)
            .iter()
            .all(|&pixel_item| pixel_item == 0x17202a));
    }
}

#[test]
fn measured_width_includes_icons_and_does_not_shorten_a_fitting_hint() {
    for count in [100, 101] {
        let pixels = frame(&with_hint(&format!("X {}", "a".repeat(count))));

        let mut expected = expected_surface(HINT_WIDTH);
        pair(&mut expected, 'X', 0);
        if count == 100 {
            label(&mut expected, &format!(" {}", "a".repeat(count)), 20);
        } else {
            label(&mut expected, &format!(" {}...", "a".repeat(97)), 20);
        }
        assert_eq!(area(&pixels, HINT_X, HINT_WIDTH), expected.pixels());
    }
}

#[test]
fn long_hints_cannot_paint_in_the_telemetry_region() {
    let mut baseline = with_hint("X launch");
    let mut long = with_hint(&format!("X launch | Y/A {}", "word ".repeat(500)));
    for app in [&mut baseline, &mut long] {
        app.set_available_bytes(Some(u64::MAX));
        app.set_memory(Some(igui::memory::MemoryInfo {
            available_bytes: u64::MAX,
            total_bytes: u64::MAX,
        }));
    }

    let before = frame(&baseline);
    let after = frame(&long);

    let position_x = HINT_X + HINT_WIDTH;
    assert_eq!(
        area(&before, position_x, WIDTH as usize - position_x),
        area(&after, position_x, WIDTH as usize - position_x)
    );
}
