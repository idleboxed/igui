// SPDX-License-Identifier: BSD-3-Clause
use super::*;

/// Opaque modal above the already drawn menu. No extra framebuffer or I/O.
pub fn draw_launch_dialog<D: DrawTarget<Color = Rgb888>>(
    app: &App,
    dialog: &LaunchDialog,
    d: &mut D,
) -> Result<(), D::Error> {
    draw_dialog(app.text(dialog.platform_name()), &dialog.message(app.locales()), d)
}

pub(super) fn trash_dialog_text(app: &App) -> (&str, &str) {
    (app.text("Move to trash?"), app.text("A: yes | B: no"))
}

pub fn draw_trash_dialog<D: DrawTarget<Color = Rgb888>>(
    app: &App,
    d: &mut D,
) -> Result<(), D::Error> {
    let (heading, message) = trash_dialog_text(app);
    draw_dialog(heading, message, d)
}

pub fn draw_input_test_countdown<D: DrawTarget<Color = Rgb888>>(
    countdown: &crate::input::startup::StartupInputTest,
    d: &mut D,
) -> Result<(), D::Error> {
    if let Some(message) = countdown.message() {
        draw_dialog("Проверка геймпада", &message, d)?;
    }
    Ok(())
}

fn draw_dialog<D: DrawTarget<Color = Rgb888>>(
    heading: &str,
    message: &str,
    d: &mut D,
) -> Result<(), D::Error> {
    DialogLayout::new(heading, message).draw(d)
}

#[derive(PartialEq, Eq)]
pub(super) struct DialogLayout {
    pub(super) panel: Rectangle,
    heading: String,
    lines: Vec<String>,
}

impl DialogLayout {
    pub(super) fn new(heading: &str, message: &str) -> Self {
        let mut lines = description::wrap(message, 76);
        if lines.len() > 7 {
            lines.truncate(7);
            lines[6] = format!("{} ...", lines[6].chars().take(72).collect::<String>());
        }
        let height = 64 + (1 + lines.len() as u32) * 24;
        Self {
            panel: rectangle(
                (WIDTH as i32 - 880) / 2,
                (HEIGHT as i32 - height as i32) / 2,
                880,
                height,
            ),
            heading: shorten(heading, 76),
            lines,
        }
    }

    pub(super) fn bounds(&self) -> Rectangle {
        Rectangle::new(self.panel.top_left, self.panel.size + Size::new(8, 8))
    }

    fn row<D: DrawTarget<Color = Rgb888>>(
        &self,
        value: &str,
        offset: i32,
        color: Rgb888,
        d: &mut D,
    ) -> Result<(), D::Error> {
        text(
            d,
            value,
            (WIDTH as i32 - value.chars().count() as i32 * 10) / 2,
            self.panel.top_left.y + offset,
            color,
            false,
        )
    }

    pub(super) fn draw<D: DrawTarget<Color = Rgb888>>(&self, d: &mut D) -> Result<(), D::Error> {
        fill(d, self.panel.translate(Point::new(8, 8)), BG)?;
        fill(d, self.panel, PANEL)?;
        border(d, self.panel, TEXT, 1)?;
        self.row(&self.heading, 32, TEXT, d)?;
        for (row, line) in self.lines.iter().enumerate() {
            self.row(line, 56 + row as i32 * 24, ACTIVE, d)?;
        }
        Ok(())
    }

    // An opaque panel of the same size needs only changed text rows cleared.
    // In particular, 10 -> 9 must erase the old digit without repainting the menu.
    pub(super) fn draw_changed<D: DrawTarget<Color = Rgb888>>(
        &self,
        previous: &Self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        assert_eq!(self.panel, previous.panel);
        for (changed, value, offset, color) in std::iter::once((
            self.heading != previous.heading, self.heading.as_str(), 32, TEXT,
        )).chain(self.lines.iter().zip(&previous.lines).enumerate().map(|(row, (line, old))| {
            (line != old, line.as_str(), 56 + row as i32 * 24, ACTIVE)
        })) {
            if changed {
                fill(d, rectangle(
                    self.panel.top_left.x + 1,
                    self.panel.top_left.y + offset,
                    self.panel.size.width - 2,
                    FONT_10X20.character_size.height,
                ), PANEL)?;
                self.row(value, offset, color, d)?;
            }
        }
        Ok(())
    }
}
