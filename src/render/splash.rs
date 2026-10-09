// SPDX-License-Identifier: BSD-3-Clause
use std::time::Duration;

use embedded_graphics::{
    mono_font::{iso_8859_5::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text},
};

use {
    crate::runtime::loading::Stage,
    crate::locales::Locales,
    imanlib::surface::Surface,
};

const BACKGROUND: Rgb888 = Rgb888::new(15, 21, 29);

struct State {
    bounds: Rectangle,
    logo: bool,
    label: String,
}

/// Incremental loading screen. Reuse only with the same unmodified target;
/// create a new painter when replacing or overwriting its retained pixels.
#[derive(Default)]
pub struct RetainedSplash {
    previous: Option<State>,
}

impl RetainedSplash {
    /// Returns false when neither the caption nor the blink phase changed.
    /// Presentation still uploads the complete retained image to a hidden buffer.
    pub fn update<D: DrawTarget<Color = Rgb888>>(
        &mut self,
        elapsed: Duration,
        stage: Stage,
        locales: &Locales,
        target: &mut D,
    ) -> Result<bool, D::Error> {
        let next = State {
            bounds: target.bounding_box(),
            logo: logo_visible(elapsed),
            label: caption(stage, locales),
        };
        // A failed partial draw invalidates the retained state: retry with a full frame.
        let previous = self.previous.take();
        let full = previous.as_ref().is_none_or(|old| old.bounds != next.bounds);
        let logo_changed = full || previous.as_ref().is_some_and(|old| old.logo != next.logo);
        let label_changed = full || previous.as_ref().is_some_and(|old| old.label != next.label);
        if full {
            target.clear(BACKGROUND)?;
        }
        if logo_changed {
            if !full {
                target.fill_solid(&logo_bounds(next.bounds), BACKGROUND)?;
            }
            if next.logo {
                draw_logo(next.bounds, target)?;
            }
        }
        if label_changed {
            if !full {
                // Clear the whole text row so shorter/localized captions leave no tail.
                let row = Rectangle::new(
                    Point::new(next.bounds.top_left.x, next.bounds.center().y + 64),
                    Size::new(next.bounds.size.width, 20),
                );
                target.fill_solid(&row, BACKGROUND)?;
            }
            draw_caption(&next.label, next.bounds, target)?;
        }
        self.previous = Some(next);
        Ok(logo_changed || label_changed)
    }
}

/// Prepare the actual first scanout before opening the display, never a black placeholder.
pub fn initial_frame() -> Surface {
    let mut frame = Surface::new(crate::render::WIDTH, crate::render::HEIGHT);
    draw_stage(Duration::ZERO, Stage::Starting, &Locales::default(), &mut frame)
        .unwrap(); // Surface is infallible.
    frame
}

/// One second on, one second off: a full blink takes two seconds (0.5 Hz).
pub fn logo_visible(elapsed: Duration) -> bool {
    elapsed.as_secs() % 2 == 0
}

/// Loading is independent of the game renderer and never decodes screenshots.
pub fn draw<D: DrawTarget<Color = Rgb888>>(
    elapsed: Duration,
    target: &mut D,
) -> Result<(), D::Error> {
    draw_localized(elapsed, &Locales::default(), target)
}

pub fn draw_localized<D: DrawTarget<Color = Rgb888>>(
    elapsed: Duration,
    locales: &Locales,
    target: &mut D,
) -> Result<(), D::Error> {
    draw_stage(elapsed, Stage::Starting, locales, target)
}

pub fn draw_stage<D: DrawTarget<Color = Rgb888>>(
    elapsed: Duration,
    stage: Stage,
    locales: &Locales,
    target: &mut D,
) -> Result<(), D::Error> {
    let bounds = target.bounding_box();
    target.clear(BACKGROUND)?;
    if logo_visible(elapsed) {
        draw_logo(bounds, target)?;
    }
    draw_caption(&caption(stage, locales), bounds, target)
}

fn caption(stage: Stage, locales: &Locales) -> String {
    format!("{} ...", locales.text(stage.label()))
}

fn logo_bounds(bounds: Rectangle) -> Rectangle {
    Rectangle::new(bounds.center() - Point::new(80, 40), Size::new(160, 80))
}

fn draw_logo<D: DrawTarget<Color = Rgb888>>(bounds: Rectangle, target: &mut D) -> Result<(), D::Error> {
    // Enlarge the existing bitmap font, rather than loading another asset.
    let mut logo = Surface::new(40, 20);
    Text::with_baseline(
        "IGUI",
        Point::zero(),
        MonoTextStyle::new(&FONT_10X20, Rgb888::WHITE),
        Baseline::Top,
    )
    .draw(&mut logo)
    .unwrap(); // Surface's error type is Infallible.
    let origin = logo_bounds(bounds).top_left;
    for (i, &pixel) in logo.pixels().iter().enumerate() {
        if pixel != 0 {
            target.fill_solid(
                &Rectangle::new(
                    origin + Point::new((i % 40) as i32 * 4, (i / 40) as i32 * 4),
                    Size::new(4, 4),
                ),
                Rgb888::new(119, 207, 180),
            )?;
        }
    }
    Ok(())
}

fn draw_caption<D: DrawTarget<Color = Rgb888>>(
    label: &str,
    bounds: Rectangle,
    target: &mut D,
) -> Result<(), D::Error> {
    Text::with_baseline(
        label,
        bounds.center() + Point::new(-(label.chars().count() as i32 * 10) / 2, 64),
        MonoTextStyle::new(&FONT_10X20, Rgb888::new(151, 169, 174)),
        Baseline::Top,
    )
    .draw(target)?;
    Ok(())
}
