// SPDX-License-Identifier: BSD-3-Clause
//! Menu input policy and explicit adapters over imanlib's shared sources.
#[cfg(target_os = "linux")]
pub mod gamepad;
pub mod joydev;
mod navigation;
#[cfg(target_os = "linux")]
pub(crate) mod probe;
pub mod startup;

pub use navigation::NavigationInput;

#[derive(Default)]
pub struct Config {
    pub controls: Option<std::path::PathBuf>,
    pub gamepad: Option<std::path::PathBuf>,
    pub gamepad_buttons: Option<(u8, u8, u8, u8)>,
    pub gamepad_select: Option<u8>,
    pub input_probe: bool,
    pub hid_profile: Option<std::path::PathBuf>,
    pub evdev_profile: Option<std::path::PathBuf>,
}

#[cfg(target_os = "linux")]
/// Load selected controller profiles, rejecting incompatible overrides and unsafe resource files.
pub(crate) fn load_profiles(config: &Config, headless: bool) -> std::io::Result<imanlib::controls::Profiles> {
    if headless || config.gamepad.is_some() {
        return Ok(imanlib::controls::Profiles::default());
    }
    let directory = match &config.controls {
        Some(path) => path.clone(),
        None => crate::paths::controls_directory()?,
    };
    if config.input_probe {
        imanlib::controls::load_probe(&directory)
    } else {
        imanlib::controls::load(&directory, config.hid_profile.as_deref(), config.evdev_profile.as_deref())
    }
}
