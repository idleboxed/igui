// SPDX-License-Identifier: BSD-3-Clause
//! Pixel-based, monotonic-time title scrolling; no I/O or frame-rate dependency.
use std::time::Duration;

pub const SPEED_PIXELS_PER_SECOND: u32 = 30;
pub const CYCLE_PAUSE: Duration = Duration::from_secs(1);
pub const GAP_PIXELS: usize = 60;

pub struct Marquee {
    content_width: usize,
    viewport_width: usize,
    elapsed: Duration,
    last_at: Option<Duration>,
    paused: bool,
    offset: usize,
}

impl Marquee {
    /// Create title scrolling state for a content width and clipped viewport width.
    pub fn new(content_width: usize, viewport_width: usize) -> Self {
        assert!(viewport_width > 0);
        Self {
            content_width,
            viewport_width,
            elapsed: Duration::ZERO,
            last_at: None,
            paused: false,
            offset: 0,
        }
    }

    /// Return the title's measured content width.
    pub fn content_width(&self) -> usize {
        self.content_width
    }

    /// Return the current horizontal scrolling offset in pixels.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Pausing/resuming consumes no elapsed interval at the transition, so a
    /// hidden row never catches up in a jump. The first call starts its clock.
    pub fn advance(&mut self, now: Duration, paused: bool) -> bool {
        if let Some(previous) = self.last_at {
            if !paused && !self.paused {
                self.elapsed = self.elapsed.saturating_add(now.saturating_sub(previous));
            }
            self.last_at = Some(previous.max(now));
        } else {
            self.last_at = Some(now);
        }
        self.paused = paused;
        if self.content_width <= self.viewport_width {
            return false;
        }

        let period = self.content_width.saturating_add(GAP_PIXELS) as u128;
        let second = Duration::from_secs(1).as_nanos();
        let speed = u128::from(SPEED_PIXELS_PER_SECOND);
        let cycle = CYCLE_PAUSE.as_nanos() + (period * second).div_ceil(speed);
        let phase = self.elapsed.as_nanos() % cycle;
        let offset = (phase.saturating_sub(CYCLE_PAUSE.as_nanos()) * speed / second) as usize;
        let changed = offset != self.offset;
        self.offset = offset;
        changed
    }
}
