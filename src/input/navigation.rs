// SPDX-License-Identifier: BSD-3-Clause
//! Menu navigation and repeat policy over shared logical button states.
use std::time::Duration;
use imanlib::{igui::Action, input::{Buttons, ACTIONS}};

#[derive(Default)]
pub struct NavigationInput {
    previous: Buttons,
    blocked: Buttons,
    repeat: Option<(Action, Duration)>,
}

impl NavigationInput {
    /// Discard loading/background input; held buttons need a fresh release.
    pub fn suspend(&mut self, buttons: Buttons) {
        self.previous = buttons;
        self.blocked = buttons;
        self.repeat = None;
    }

    pub fn block_until_release(&mut self, action: Action) {
        self.blocked.set(action, true);
        self.repeat = None;
    }

    pub fn poll(&mut self, buttons: Buttons, now: Duration) -> Vec<Action> {
        self.blocked = self.blocked.intersect(buttons);
        let mut actions = Vec::new();
        let direction = ACTIONS[..4]
            .iter()
            .copied()
            .find(|&action| buttons.down(action) && !self.blocked.down(action));
        if let Some(action) = direction {
            if !self.previous.down(action) {
                actions.push(action);
                self.repeat = Some((action, now + Duration::from_millis(350)));
            } else if self
                .repeat
                .is_some_and(|(repeated, due)| repeated == action && now >= due)
            {
                actions.push(action);
                self.repeat = Some((action, now + Duration::from_millis(100)));
            }
        } else {
            self.repeat = None;
        }
        for &action in &ACTIONS[4..] {
            if buttons.down(action) && !self.previous.down(action) && !self.blocked.down(action) {
                actions.push(action);
            }
        }
        self.previous = buttons;
        actions
    }
}
