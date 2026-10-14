// SPDX-License-Identifier: BSD-3-Clause
use imanlib::input::Buttons;
use std::io;

/// Open the fixed-size desktop window for the menu.
pub(super) fn open() -> io::Result<minifb::Window> {
    let mut window = minifb::Window::new(
        "IGUI",
        crate::render::WIDTH as usize,
        crate::render::HEIGHT as usize,
        minifb::WindowOptions {
            resize: false,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..Default::default()
        },
    )
    .map_err(io::Error::other)?;
    window.set_target_fps(30);
    Ok(window)
}

/// Read held desktop keys as logical menu buttons.
pub(super) fn buttons(window: &minifb::Window) -> Buttons {
    use imanlib::igui::Action;
    use minifb::{Key, KeyRepeat};
    let mut buttons = Buttons::default();
    for (key, action) in [
        (Key::Up, Action::Up),
        (Key::Down, Action::Down),
        (Key::Left, Action::Left),
        (Key::Right, Action::Right),
        (Key::X, Action::LowerFilters),
        (Key::B, Action::Favourite),
        (Key::Y, Action::Filters),
        (Key::A, Action::Activate),
        (Key::Delete, Action::Trash),
        (Key::Enter, Action::Activate),
    ] {
        if window.is_key_down(key) || window.is_key_pressed(key, KeyRepeat::No) {
            buttons.set(action, true);
        }
    }
    buttons
}
