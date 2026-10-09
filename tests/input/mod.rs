// SPDX-License-Identifier: BSD-3-Clause
use imanlib::{igui::Action, input::Buttons};

mod gamepad;
mod handoff;
mod joydev;
mod navigation;
mod profiles;
mod receiver;
mod startup;

fn held(action: Action) -> Buttons {
    let mut buttons = Buttons::default();
    buttons.set(action, true);
    buttons
}
