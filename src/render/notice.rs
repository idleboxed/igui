// SPDX-License-Identifier: BSD-3-Clause
//! On-device notices: localized headings and bounded, readable technical details.
use crate::{model::description, locales::Locales};
use super::{card, fill, rectangle, text, App, DrawTarget, Rgb888, ERROR_BG, TEXT};

pub fn launch_failed(reason: &str) -> String {
    let reason = reason.strip_prefix("launch_refused: ").unwrap_or(reason);
    format!("Cannot launch game.\n{reason}")
}

pub fn lines(locales: &Locales, message: &str, columns: usize) -> Vec<String> {
    // A bare ENOENT carries no component identity. Improve the visible wording,
    // without guessing which file is missing or changing the original diagnostic.
    let rewritten = message.strip_prefix("Cannot launch game.\n").and_then(|details| {
        let (rom, reason) = if details.starts_with("ROM: ") {
            details.split_once('\n')?
        } else {
            ("", details)
        };
        (reason.trim() == "No such file or directory (os error 2)").then(|| {
            let separator = if rom.is_empty() { "" } else { "\n" };
            format!("Cannot launch game.\n{rom}{separator}A required file was not found.\n\
                Check the ROM and emulator installation on the SD card.")
        })
    });
    let message = rewritten.as_deref().unwrap_or(message);
    let clean: String = message.chars().take(2048)
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c }).collect();
    let translated = clean.lines().map(|line| {
        for (prefix, label) in [
            ("Required BIOS missing: ", "Required BIOS missing:"),
            ("Invalid BIOS: ", "Invalid BIOS:"),
            ("Cannot read BIOS: ", "Cannot read BIOS:"),
            ("no core for platform ", "No core for platform:"),
            ("Emulator status: ", "Emulator status:"),
        ] {
            if let Some(detail) = line.strip_prefix(prefix) {
                return format!("{} {detail}", locales.text(label));
            }
        }
        locales.text(line).to_owned()
    }).collect::<Vec<_>>().join("\n");
    description::wrap(&translated, columns)
}

// Shared placement gives notices the same six-line area in the card and search.
pub(super) fn draw<D: DrawTarget<Color = Rgb888>>(
    d: &mut D,
    app: &App,
    message: &str,
) -> Result<(), D::Error> {
    fill(
        d,
        rectangle(672, card::DESCRIPTION_Y - 1, 564, card::DESCRIPTION_HEIGHT + 1),
        ERROR_BG,
    )?;
    let lines = crate::render::notice::lines(app.locales(), message, description::COLUMNS);
    let rows = lines.len().min(description::ROWS) as u32;
    let padding = (card::DESCRIPTION_HEIGHT - rows * card::DESCRIPTION_LINE_HEIGHT).min(12);
    for (row, line) in lines.iter().take(description::ROWS).enumerate() {
        text(
            d,
            line,
            684,
            card::DESCRIPTION_Y + padding as i32
                + row as i32 * card::DESCRIPTION_LINE_HEIGHT as i32,
            TEXT,
            true,
        )?;
    }
    Ok(())
}
