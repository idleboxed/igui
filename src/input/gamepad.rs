// SPDX-License-Identifier: BSD-3-Clause
//! Application input selection: profiled HID/evdev arbitration or explicit joydev.
use super::joydev::{self, Device};
use imanlib::{
    evdev::linux::ProfileSource,
    gamepad::{automatic::Automatic, profile::Profile, receiver::Receiver},
    hid::{linux::LinuxSource, receiver::Receivers, Profile as HidProfile},
    input::{Buttons, ACTIONS},
    input_metrics::{LogicalEdgeSampler, Sample},
};
use std::{collections::BTreeSet, io, path::{Path, PathBuf}, time::Duration};

pub struct Gamepad {
    automatic: Automatic<ProfileSource, LinuxSource>,
    device: Option<Device>,
    path: Option<PathBuf>,
    overrides: Option<(u8, u8, u8, u8)>,
    select_override: Option<u8>,
    next_scan: Duration,
    reported_errors: BTreeSet<String>,
    diagnostics: Option<Sample>,
    logical_edge_sampler: LogicalEdgeSampler,
}

impl Gamepad {
    pub fn with_profiles(
        path: Option<PathBuf>,
        overrides: Option<(u8, u8, u8, u8)>,
        select_override: Option<u8>,
        hid_profiles: Vec<HidProfile>,
        evdev_profiles: Vec<Profile>,
    ) -> Self {
        Self {
            automatic: Automatic::new(
                Receiver::with_profiles(ProfileSource::new(evdev_profiles.clone()), evdev_profiles),
                (!hid_profiles.is_empty()).then(|| {
                    Receivers::with_profiles(LinuxSource::new(hid_profiles.clone()), hid_profiles)
                }),
            ),
            device: None,
            path,
            overrides,
            select_override,
            next_scan: Duration::ZERO,
            reported_errors: BTreeSet::new(),
            diagnostics: None,
            logical_edge_sampler: LogicalEdgeSampler::default(),
        }
    }

    pub fn enable_diagnostics(&mut self) {
        self.diagnostics = Some(Sample::default());
        self.logical_edge_sampler = LogicalEdgeSampler::default();
        self.automatic.enable_diagnostics();
    }
    /// No I/O: snapshots only the already-owned input backends.
    pub fn diagnostics(&self) -> Option<Sample> {
        let mut sample = self.diagnostics.clone()?;
        sample.logical_edges = self.logical_edge_sampler.snapshot();
        self.automatic.update_diagnostics(&mut sample);
        sample.joydev_connected = self.device.is_some();
        Some(sample)
    }
    pub fn poll(&mut self, now: Duration) -> Buttons {
        let buttons = self.poll_inner(now);
        if let Some(diagnostics) = &mut self.diagnostics {
            diagnostics.polls += 1;
            self.logical_edge_sampler.record(ACTIONS.map(|action| buttons.down(action)));
        }
        buttons
    }
    fn poll_inner(&mut self, now: Duration) -> Buttons {
        // Explicit joydev paths/indices retain their original CLI meaning.
        // Automatic mode has one arbiter, never merged backend states.
        if self.device.is_none()
            && self.path.is_none()
            && self.overrides.is_none()
            && self.select_override.is_none()
        {
            // Unknown pads may enter the diagnostic test, not navigation
            // through a guessed joydev layout. Manual joydev remains explicit.
            return self.automatic.poll(now);
        }
        if self.device.is_none() && now >= self.next_scan {
            self.next_scan = now + Duration::from_secs(2);
            let paths = if let Some(path) = &self.path {
                vec![path.clone()]
            } else {
                let directory = Path::new(imanlib::input::INPUT_DIR);
                match joydev::discover(directory) {
                    Ok(paths) => paths,
                    Err(error) => {
                        self.report_joydev_error(directory, &error);
                        Vec::new()
                    }
                }
            };
            for path in paths {
                match Device::open(&path, self.overrides, self.select_override) {
                    Ok(device) => {
                        if let Some(diagnostics) = &mut self.diagnostics {
                            diagnostics.joydev.connections += 1;
                        }
                        eprintln!(
                            "IGUI gamepad: {} (X={}, B={}, Y={}, A={}, Select={:?})",
                            path.display(),
                            device.state.mapping.x,
                            device.state.mapping.b,
                            device.state.mapping.y,
                            device.state.mapping.a,
                            device.state.mapping.select
                        );
                        self.device = Some(device);
                        self.reported_errors.clear();
                        break;
                    }
                    Err(error) => self.report_joydev_error(&path, &error),
                }
            }
        }
        if let Some(device) = &mut self.device {
            match device.poll(self.diagnostics.as_mut().map(|diagnostics| &mut diagnostics.joydev)) {
                Ok(buttons) => return buttons,
                Err(error) => {
                    if let Some(diagnostics) = &mut self.diagnostics {
                        diagnostics.joydev.disconnects += 1;
                        diagnostics.joydev.errors += 1;
                    }
                    eprintln!("IGUI gamepad disconnected: {error}");
                    self.device = None;
                    self.next_scan = now + Duration::from_secs(2);
                }
            }
        }
        Buttons::default()
    }

    fn report_joydev_error(&mut self, path: &Path, error: &io::Error) {
        if let Some(diagnostics) = &mut self.diagnostics {
            diagnostics.joydev.errors += 1;
        }
        let message = format!("{}: {error}", path.display());
        if self.reported_errors.len() < 32 && self.reported_errors.insert(message.clone()) {
            eprintln!("IGUI gamepad: {message}");
        }
    }
}
