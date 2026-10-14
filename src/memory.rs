// SPDX-License-Identifier: BSD-3-Clause
//! Read-only Linux memory telemetry, independent of the windowing adapter.
use std::{path::Path, time::Duration};

pub use imanlib::metrics::MemoryInfo;

/// Exactly the numeric text used by the footer, also the invalidation key.
pub fn display_amount(info: Option<MemoryInfo>) -> String {
    info.map_or_else(
        || "-".into(),
        |info| {
            format!(
                "{:.1} / {:.1}",
                info.available_bytes as f64 / 1_048_576.0,
                info.total_bytes as f64 / 1_048_576.0
            )
        },
    )
}

/// No fallback to MemFree: it measures a different quantity.
/// Procfs files report zero metadata length, so read their contents directly.
pub fn read_from(path: &Path) -> Option<MemoryInfo> {
    imanlib::metrics::read_memory(path).ok()
}

/// Read bounded system memory information, returning None if unavailable or malformed.
pub fn read() -> Option<MemoryInfo> {
    #[cfg(target_os = "linux")]
    return read_from(Path::new(crate::paths::MEMINFO_FILE));
    #[cfg(not(target_os = "linux"))]
    None
}

#[derive(Default)]
pub struct MemoryMonitor {
    next_read: Duration,
    info: Option<MemoryInfo>,
}

impl MemoryMonitor {
    /// Sample immediately, then at most once per second; never catch up in a burst.
    /// Return whether the displayed state needs refreshing, including read errors.
    pub fn refresh(&mut self, now: Duration, read: impl FnOnce() -> Option<MemoryInfo>) -> bool {
        if now < self.next_read {
            return false;
        }
        self.next_read = now.saturating_add(Duration::from_secs(1));
        let next = read();
        let changed = display_amount(self.info) != display_amount(next);
        self.info = next;
        changed
    }

    /// Return the latest memory sample retained by this monitor.
    pub fn info(&self) -> Option<MemoryInfo> {
        self.info
    }
}
