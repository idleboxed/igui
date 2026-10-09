// SPDX-License-Identifier: BSD-3-Clause
use std::{
    cell::Cell,
    fs,
    time::Duration,
};

use igui::memory::{self, MemoryInfo, MemoryMonitor};
use crate::common::Scratch;

#[test]
fn adapter_exposes_shared_readings_and_maps_read_errors_to_unknown() {
    let dir = Scratch::new();
    let path = dir.root().join("meminfo");

    assert_eq!(memory::read_from(&path), None);
    fs::write(&path, "MemTotal: 256 kB\nMemAvailable: 128 kB\n").unwrap();
    assert_eq!(
        memory::read_from(&path),
        Some(MemoryInfo {
            available_bytes: 128 * 1024,
            total_bytes: 256 * 1024,
        })
    );
    fs::write(&path, "invalid meminfo\n").unwrap();
    assert_eq!(memory::read_from(&path), None);
}

#[test]
fn samples_once_per_second_and_does_not_burst_after_a_stall() {
    let reads = Cell::new(0);
    let mut monitor = MemoryMonitor::default();
    let sample = Some(MemoryInfo {
        available_bytes: 128 * 1_048_576,
        total_bytes: 256 * 1_048_576,
    });

    for (millis, expected_reads) in [
        (0, 1),
        (20, 1),
        (999, 1),
        (1000, 2),
        (1001, 2),
        (10_000, 3),
        (10_000, 3),
        (10_999, 3),
        (11_000, 4),
    ] {
        let changed = monitor.refresh(Duration::from_millis(millis), || {
            reads.set(reads.get() + 1);
            sample
        });
        assert_eq!(reads.get(), expected_reads);
        assert_eq!(changed, millis == 0);
        assert_eq!(monitor.info(), sample);
    }
}

#[test]
fn errors_clear_old_data_and_a_later_success_recovers_without_retaining_stale_values() {
    let mut monitor = MemoryMonitor::default();
    let first = Some(MemoryInfo {
        available_bytes: 128 * 1_048_576,
        total_bytes: 256 * 1_048_576,
    });
    let second = Some(MemoryInfo {
        available_bytes: 0,
        total_bytes: 256 * 1_048_576,
    });

    for (seconds, value, changed) in [
        (0, None, false),
        (1, first, true),
        (2, second, true),
        (3, None, true),
        (4, None, false),
        (5, first, true),
    ] {
        assert_eq!(
            monitor.refresh(Duration::from_secs(seconds), || value),
            changed
        );
        assert_eq!(monitor.info(), value);
    }
}

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_platform_reports_unknown() {
    assert_eq!(memory::read(), None);
}

#[test]
fn raw_memory_change_with_identical_display_does_not_request_redraw() {
    let mut monitor = MemoryMonitor::default();
    for (seconds, bytes, changed) in [
        (0, 104_857_600, true),
        (1, 104_858_624, false),
        (2, 104_960_000, true),
    ] {
        let info = Some(MemoryInfo {
            available_bytes: bytes,
            total_bytes: 250_000_000,
        });
        assert_eq!(
            monitor.refresh(Duration::from_secs(seconds), || info),
            changed
        );
        assert_eq!(monitor.info(), info);
    }
}
