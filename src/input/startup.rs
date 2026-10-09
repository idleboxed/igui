// SPDX-License-Identifier: BSD-3-Clause
//! One startup invitation only; ordinary catalogue input consumes it permanently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition { None, Cancelled, Start }

#[derive(Default)]
pub struct StartupInputTest {
    ready_ms: Option<u64>,
    countdown_started_ms: Option<u64>,
    consumed: bool,
    countdown: Option<u64>,
}

impl StartupInputTest {
    pub fn update(&mut self, ms: u64, ready: bool, activity: bool, b_pressed: bool) -> Transition {
        if self.consumed || !ready { return Transition::None; }
        let started = *self.ready_ms.get_or_insert(ms);
        // Input at either deadline wins over opening/finishing the invitation.
        if (self.countdown.is_none() && activity) || (self.countdown.is_some() && b_pressed) {
            let was_visible = self.countdown.take().is_some();
            self.consumed = true;
            return if was_visible { Transition::Cancelled } else { Transition::None };
        }
        if ms.saturating_sub(started) >= 10_000 {
            // A delayed first frame must still show a full ten-second invitation.
            let visible_since = *self.countdown_started_ms.get_or_insert(ms);
            let elapsed = ms.saturating_sub(visible_since);
            if elapsed >= 10_000 {
                self.countdown = None;
                self.consumed = true;
                return Transition::Start;
            }
            self.countdown = Some((10_000 - elapsed).div_ceil(1000));
        }
        Transition::None
    }

    pub fn cancel(&mut self) {
        self.countdown = None;
        self.consumed = true;
    }

    pub fn countdown(&self) -> Option<u64> { self.countdown }

    pub fn message(&self) -> Option<String> {
        self.countdown.map(|seconds| format!(
            "Controller test starts in {seconds} ...\nPress B to cancel."
        ))
    }
}
