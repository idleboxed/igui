// SPDX-License-Identifier: BSD-3-Clause
use std::{
    io,
    sync::{
        mpsc::{self, Receiver, TryRecvError},
        Arc, Condvar, Mutex,
    },
    thread,
    time::Duration,
};

use crate::model::{App, Game};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Starting,
    Gamepad,
    Preferences,
    Locales,
    Catalogue,
    Library,
    Storage,
    Preview,
    Interface,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Starting => "Starting IGUI",
            Self::Gamepad => "Detecting gamepad",
            Self::Preferences => "Reading preferences",
            Self::Locales => "Loading translations",
            Self::Catalogue => "Reading catalogue",
            Self::Library => "Preparing game list",
            Self::Storage => "Checking free space",
            Self::Preview => "Loading selected image",
            Self::Interface => "Drawing interface",
        }
    }
}

/// Load metadata and build the full catalog off the UI thread. No preview
/// decoding: Game contains only a preview reference, not decoded image pixels.
pub struct CatalogLoader<T = App> {
    result: Receiver<Result<T, String>>,
    cancellation: Cancellation,
    worker: Option<thread::JoinHandle<()>>,
}

/// Cooperative cancellation, including an interruptible diagnostic loading delay.
#[derive(Clone, Default)]
pub struct Cancellation {
    state: Arc<(Mutex<bool>, Condvar)>,
}

impl Cancellation {
    pub fn check(&self) -> Result<(), String> {
        if *self.state.0.lock().expect("catalog cancellation lock poisoned") {
            Err("catalog loading cancelled".into())
        } else {
            Ok(())
        }
    }

    pub fn wait_for(&self, duration: Duration) -> Result<(), String> {
        let cancelled = self.state.0.lock().expect("catalog cancellation lock poisoned");
        let (cancelled, _) = self.state.1
            .wait_timeout_while(cancelled, duration, |cancelled| !*cancelled)
            .expect("catalog cancellation lock poisoned");
        if *cancelled {
            Err("catalog loading cancelled".into())
        } else {
            Ok(())
        }
    }

    fn cancel(&self) {
        *self.state.0.lock().expect("catalog cancellation lock poisoned") = true;
        self.state.1.notify_all();
    }
}

impl CatalogLoader {
    pub fn spawn<F>(source: F) -> io::Result<Self>
    where
        F: FnOnce(&Cancellation) -> Result<Vec<Game>, String> + Send + 'static,
    {
        Self::spawn_value(move |cancellation| source(cancellation).map(App::new))
    }
}

impl<T: Send + 'static> CatalogLoader<T> {
    pub fn spawn_value<F>(source: F) -> io::Result<Self>
    where
        F: FnOnce(&Cancellation) -> Result<T, String> + Send + 'static,
    {
        let (sender, result) = mpsc::channel();
        let cancellation = Cancellation::default();
        let worker_cancellation = cancellation.clone();
        let worker = thread::Builder::new()
            .name("igui-catalog".into())
            .spawn(move || {
                // If the window has closed, discard the result without a panic.
                let _ = sender.send(source(&worker_cancellation));
            })?;
        Ok(Self { result, cancellation, worker: Some(worker) })
    }

    /// Nonblocking; consume the loader after receiving the catalog or an error.
    pub fn poll(&self) -> Result<Option<T>, String> {
        match self.result.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err("catalog loader stopped without returning a result".into())
            }
        }
    }
}

impl<T> CatalogLoader<T> {
    /// Sources must check cancellation between bounded operations. A running
    /// filesystem read cannot be interrupted here; it still finishes normally.
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }
}

impl<T> Drop for CatalogLoader<T> {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("IGUI: catalog worker panicked");
            }
        }
    }
}
