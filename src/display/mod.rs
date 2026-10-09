// SPDX-License-Identifier: BSD-3-Clause
//! Presentation/input adapters for the shared GUI event loop.
use {
    imanlib::input::Buttons,
    imanlib::surface::Surface,
};
#[cfg(all(feature = "drm", target_os = "linux"))]
use imanlib::display_metrics::Timing;
use imanlib::display_metrics::{Backend, Sample, Statistics};
use std::io;

pub enum Display {
    Headless,
    #[cfg(feature = "desktop")]
    Desktop(Box<minifb::Window>),
    #[cfg(all(feature = "drm", target_os = "linux"))]
    Drm(Box<drm::Display>),
}

impl Display {
    pub fn open(options: &Config, first: &Surface) -> io::Result<Self> {
        if options.headless {
            return Ok(Self::Headless);
        }
        if options.drm {
            #[cfg(all(feature = "drm", target_os = "linux"))]
            return Ok(Self::Drm(Box::new(drm::Display::open(first, options)?)));
            #[cfg(not(all(feature = "drm", target_os = "linux")))]
            return Err(io::Error::other("DRM support is not enabled in this build"));
        }
        let _ = first;
        #[cfg(feature = "desktop")]
        {
            Ok(Self::Desktop(Box::new(desktop::open()?)))
        }
        #[cfg(not(feature = "desktop"))]
        Err(io::Error::other(
            "desktop support is disabled; explicitly select --drm or --headless",
        ))
    }

    pub fn enable_diagnostics(&mut self) {
        #[cfg(all(feature = "drm", target_os = "linux"))]
        if let Self::Drm(w) = self {
            w.timings = Some((Timing::default(), Timing::default()));
        }
    }

    pub fn diagnostic_report(&self, render: Option<Statistics>, cleanup_ok: bool) -> Sample {
        let (backend, kms, present, idle_check) = match self {
            Self::Headless => (Backend::Headless, None, None, None),
            #[cfg(feature = "desktop")]
            Self::Desktop(_) => (Backend::Desktop, None, None, None),
            #[cfg(all(feature = "drm", target_os = "linux"))]
            Self::Drm(w) => (
                Backend::Drm,
                w.kms.diagnostic_state(),
                w.timings.as_ref().map(|t| t.0.clone()),
                w.timings.as_ref().map(|t| t.1.clone()),
            ),
        };
        Sample {
            backend,
            kms,
            present,
            idle_check,
            render,
            probe: None,
            cleanup_ok: Some(cleanup_ok),
            operation_ok: Some(true),
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_alive() && !self.escape_down()
    }

    pub fn is_alive(&self) -> bool {
        match self {
            Self::Headless => true,
            #[cfg(feature = "desktop")]
            Self::Desktop(w) => w.is_open(),
            #[cfg(all(feature = "drm", target_os = "linux"))]
            Self::Drm(w) => !w.signals.stopped(),
        }
    }

    pub fn escape_down(&self) -> bool {
        #[cfg(feature = "desktop")]
        if let Self::Desktop(window) = self {
            return window.is_key_down(minifb::Key::Escape);
        }
        false
    }

    pub fn is_active(&mut self) -> bool {
        match self {
            Self::Headless => true,
            #[cfg(feature = "desktop")]
            Self::Desktop(w) => w.is_active(),
            #[cfg(all(feature = "drm", target_os = "linux"))]
            Self::Drm(_) => true,
        }
    }

    pub fn buttons(&self) -> Buttons {
        match self {
            Self::Headless => Buttons::default(),
            #[cfg(feature = "desktop")]
            Self::Desktop(w) => {
                desktop::buttons(w)
            }
            #[cfg(all(feature = "drm", target_os = "linux"))]
            Self::Drm(_) => Buttons::default(), // joydev is shared by both adapters
        }
    }

    pub fn present(&mut self, surface: &Surface, changed: bool) -> io::Result<()> {
        let _ = (surface, changed);
        match self {
            Self::Headless => {
                std::thread::sleep(std::time::Duration::from_millis(2));
                Ok(())
            }
            #[cfg(feature = "desktop")]
            Self::Desktop(w) => w
                .update_with_buffer(
                    surface.pixels(),
                    surface.width() as usize,
                    surface.height() as usize,
                )
                .map_err(io::Error::other),
            #[cfg(all(feature = "drm", target_os = "linux"))]
            Self::Drm(w) => {
                // Idle scanout needs no pixel upload/mapping; check ownership.
                let start = w.timings.as_ref().map(|_| std::time::Instant::now());
                let result = if changed {
                    w.kms.present(surface)
                } else {
                    w.kms.check_scanout()
                };
                if let (Some(start), Some((present, idle))) = (start, &mut w.timings) {
                    if changed {
                        present.record(start.elapsed().as_micros());
                    } else {
                        idle.record(start.elapsed().as_micros());
                    }
                }
                result?;
                let elapsed = w.last_frame.elapsed();
                std::thread::sleep(std::time::Duration::from_millis(33).saturating_sub(elapsed));
                w.last_frame = std::time::Instant::now();
                Ok(())
            }
        }
    }

    pub fn finish(&mut self) -> io::Result<()> {
        match self {
            Self::Headless => Ok(()),
            #[cfg(feature = "desktop")]
            Self::Desktop(_) => Ok(()),
            #[cfg(all(feature = "drm", target_os = "linux"))]
            Self::Drm(w) => w.kms.finish(),
        }
    }
}

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(all(feature = "drm", target_os = "linux"))]
mod drm;

#[derive(Default)]
pub struct Config {
    pub headless: bool,
    pub drm: bool,
    pub drm_device: Option<std::path::PathBuf>,
    pub drm_connector: Option<u32>,
    pub allow_vendor_einval: bool,
}
