// SPDX-License-Identifier: BSD-3-Clause
//! Owned temporary directories and child processes for isolated test scenarios.
use std::{
    io,
    ops::{Deref, DerefMut},
    path::Path,
    process::{Child, Output},
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread::{self, JoinHandle},
};

pub struct Scratch {
    directory: tempfile::TempDir,
}

impl Scratch {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().expect("create isolated fixture directory");
        Self { directory }
    }

    pub fn root(&self) -> &Path {
        self.directory.path()
    }

    pub fn with_games() -> Self {
        let scratch = Self::new();
        std::fs::create_dir(scratch.root().join("games")).expect("create fixture games directory");
        scratch
    }
}

pub struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    pub fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub fn wait_with_output(mut self) -> io::Result<Output> {
        self.child.take().expect("owned child").wait_with_output()
    }
}

impl Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Child {
        self.child.as_ref().expect("owned child")
    }
}

impl DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("owned child")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub struct HostGuard<T> {
    stopped: Arc<AtomicBool>,
    worker: Option<JoinHandle<T>>,
}

impl<T> HostGuard<T> {
    pub fn new(stopped: Arc<AtomicBool>, worker: JoinHandle<T>) -> Self {
        Self { stopped, worker: Some(worker) }
    }

    pub fn join(mut self) -> thread::Result<T> {
        self.stopped.store(true, Ordering::Relaxed);
        self.worker.take().expect("owned host worker").join()
    }
}

impl<T> Drop for HostGuard<T> {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("IGUI fixture host panicked during cleanup");
            }
        }
    }
}
