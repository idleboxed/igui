// SPDX-License-Identifier: BSD-3-Clause
//! Application session orchestration, independent of CLI argument parsing.
mod commands;
mod diagnostics;
mod effects;
mod session;
pub mod launch;
pub mod loading;
pub mod startup;

use std::{error::Error, path::PathBuf};

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

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    session::run(config)
}
