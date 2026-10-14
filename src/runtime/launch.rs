//! Presentation state only; starting an emulator belongs to the platform adapter.
use std::time::Duration;

use crate::{locales::Locales, model::platforms};

pub const MINIMUM_VISIBLE: Duration = Duration::from_secs(1);

pub struct LaunchDialog {
    title: String,
    platform: String,
    presented_at: Option<Duration>,
}

impl LaunchDialog {
    /// Create a localized launch dialog from the selected game without starting a process.
    pub fn new(title: String, platform: String) -> Self {
        Self {
            title,
            platform,
            presented_at: None,
        }
    }

    /// Return the platform display name retained for this dialog.
    pub fn platform_name(&self) -> &str {
        platforms::full_name(&self.platform)
    }

    /// Format this dialog's launch message.
    pub fn message(&self, locales: &Locales) -> String {
        locales
            .text("Loading {title} ...")
            .replace("{title}", &self.title)
    }

    /// Start the minimum interval after presenting the first dialog frame,
    /// not at the input event or before potentially slow preview decoding.
    pub fn presented(&mut self, now: Duration) {
        self.presented_at.get_or_insert(now);
    }

    /// In the demo this dismisses the dialog. A real adapter must also be ready
    /// to hand off to the emulator; elapsed time alone does not start a game.
    pub fn minimum_elapsed(&self, now: Duration) -> bool {
        self.presented_at
            .is_some_and(|start| now.saturating_sub(start) >= MINIMUM_VISIBLE)
    }
}
