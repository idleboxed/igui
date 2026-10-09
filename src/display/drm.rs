// SPDX-License-Identifier: BSD-3-Clause
use super::*;
use {
    imanlib::kms::Kms,
    imanlib::kms::LinuxDrm,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

pub struct Signals(Vec<(libc::c_int, libc::sigaction)>);
impl Signals {
    fn install() -> io::Result<Self> {
        STOP.store(false, Ordering::Relaxed);
        let mut guard = Self(Vec::new());
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // SAFETY: sigaction/sigemptyset receive valid initialized structs;
            // handler only stores a lock-free atomic flag, no I/O or cleanup.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                let mut old: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = stop as *const () as usize;
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(signal, &action, &mut old) != 0 {
                    return Err(io::Error::last_os_error());
                }
                guard.0.push((signal, old));
            }
        }
        Ok(guard)
    }
    pub fn stopped(&self) -> bool {
        STOP.load(Ordering::Relaxed)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for (signal, action) in self.0.iter().rev() {
            // SAFETY: restore the dispositions returned by sigaction.
            if unsafe { libc::sigaction(*signal, action, std::ptr::null_mut()) } != 0 {
                eprintln!(
                    "IGUI: cannot restore signal handler: {}",
                    io::Error::last_os_error()
                );
            }
        }
    }
}

pub struct Display {
    pub kms: Kms<LinuxDrm>, // drop KMS before restoring signal dispositions
    pub signals: Signals,
    pub last_frame: Instant,
    pub timings: Option<(Timing, Timing)>,
}
impl Display {
    pub fn open(first: &Surface, options: &super::Config) -> io::Result<Self> {
        let backend = LinuxDrm::open(
            options
                .drm_device
                .as_deref()
                .unwrap_or(std::path::Path::new("/dev/dri/card0")),
            options.drm_connector,
        )?;
        let signals = Signals::install()?;
        let kms = Kms::start(backend, first, options.allow_vendor_einval)?;
        Ok(Self {
            kms,
            signals,
            last_frame: Instant::now(),
            timings: None,
        })
    }
}
