// SPDX-License-Identifier: BSD-3-Clause
//! Filesystem adapters. Persistent writes require explicit caller actions.
mod preferences;
mod resources;
pub mod screenshot;

pub use preferences::Preferences;
pub use resources::{available_bytes, load_catalogue, load_locales, load_preview};
