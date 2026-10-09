// SPDX-License-Identifier: BSD-3-Clause
//! IGUI filesystem layout. Installed resources follow the executable.
//! SD names are a shared format with icat/iman, not a dependency on their layout modules.

pub const GAMES_DIR: &str = "games";
pub const LOCALES_DIR: &str = "locales";
pub const CONTROLS_DIR: &str = "controls";
pub const THUMBS_DIR: &str = "thumbs";
pub const CATALOGUE_FILE: &str = "catalogue.json";
pub const PREFERENCES_FILE: &str = "prefs.json";
pub const MEMINFO_FILE: &str = "/proc/meminfo";

/// Resolve the installed resource base without embedding a build-machine path.
pub fn executable_directory() -> std::io::Result<std::path::PathBuf> {
    let executable = std::env::current_exe()?;
    executable.parent().map(std::path::Path::to_path_buf)
        .ok_or_else(|| std::io::Error::other("IGUI executable has no parent directory"))
}

/// Application resource layout stays outside the shared profile loader.
pub fn controls_directory() -> std::io::Result<std::path::PathBuf> {
    Ok(executable_directory()?.join(CONTROLS_DIR))
}
