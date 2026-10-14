// SPDX-License-Identifier: BSD-3-Clause
//! Application session orchestration, independent of CLI argument parsing.
mod commands;
mod diagnostics;
mod effects;
mod session;
pub mod launch;
pub mod loading;
pub mod startup;

use std::{error::Error, path::PathBuf, time::{Duration, Instant}};

#[derive(Default)]
pub struct Config {
    pub managed: bool,
    pub ipc_socket: Option<PathBuf>,
    pub frames: Option<usize>,
    pub demo_load_ms: Option<u64>,
    pub games: Option<PathBuf>,
    pub display: crate::display::Config,
    pub input: crate::input::Config,
}

/// Run the configured menu session with the system's monotonic clock.
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    run_with_clock(config, || started.elapsed())
}

/// Run the same session with a caller-owned monotonic clock for UI timers.
/// Clock values must not decrease; I/O waits remain independently bounded.
pub fn run_with_clock(config: Config, clock: impl Fn() -> Duration) -> Result<(), Box<dyn Error>> {
    session::run(config, clock)
}
