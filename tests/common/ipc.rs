// SPDX-License-Identifier: BSD-3-Clause
//! Integration-test launcher: real GUI process, common bus, no iman or display.
use super::{ChildGuard, HostGuard};
use imanlib::{client::Client, igui_iman_stub::Host as ImanStub};
use serde_json::json;
use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub struct Driver {
    command: Command,
    cwd: PathBuf,
    actions: String,
    screenshot: Option<PathBuf>,
}
pub struct Running {
    pub client: Client,
    pub socket: PathBuf,
    child: Option<ChildGuard>,
    host: Option<HostGuard<()>>,
}
impl Drop for Running {
    fn drop(&mut self) {
        drop(self.child.take());
        drop(self.host.take());
    }
}
impl Driver {
    pub fn new(binary: &Path, cwd: &Path) -> Self {
        let mut command = Command::new(binary);
        command.current_dir(cwd);
        Self {
            command,
            cwd: cwd.into(),
            actions: String::new(),
            screenshot: None,
        }
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }
    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.command.env_remove(key);
        self
    }
    pub fn input(&mut self, actions: &str) -> &mut Self {
        self.actions = actions.into();
        self
    }
    pub fn screenshot(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.screenshot = Some(self.cwd.join(path));
        self
    }
    pub fn start(&mut self) -> io::Result<Running> {
        if !self.command.get_args().any(|arg| arg == "--games" || arg == "--managed") {
            self.command.arg("--games").arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/_fixtures/games"));
        }
        let mut host = ImanStub::bind(Path::new("/dev/shm"))?;
        let socket = host.path().to_owned();
        let child = ChildGuard::new(self
            .command
            .args(["--headless", "--ipc-socket"])
            .arg(&socket)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?);
        let stop = Arc::new(AtomicBool::new(false));
        let mut running = Running {
            client: Client::connect(&socket)?,
            socket: socket.clone(),
            child: Some(child),
            host: None,
        };
        host.expect_gui(running.child.as_ref().unwrap().id())?;
        let task_stop = stop.clone();
        running.host = Some(HostGuard::new(stop, thread::spawn(move || {
            while !task_stop.load(Ordering::Relaxed) {
                let errors = host.poll();
                assert!(errors.is_empty(), "{errors:?}");
                thread::sleep(Duration::from_millis(1));
            }
        })));
        Ok(running)
    }
    pub fn output(&mut self) -> io::Result<Output> {
        let mut running = self.start()?;
        running.wait_ready()?;
        let mut closing = false;
        for action in self.actions.split(',').filter(|a| !a.is_empty()) {
            let result = running
                .client
                .value("igui.input", json!({"action":action}))?;
            if result["closing"] == true {
                closing = true;
                break;
            }
        }
        if !closing {
            if let Some(path) = &self.screenshot {
                let reply = running.client.value("igui.screenshot", json!({}))?;
                let mut input = std::fs::File::open(reply["path"].as_str().unwrap())?;
                let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
                io::copy(&mut input, &mut output)?;
            }
            running.client.value("igui.close", json!({}))?;
        }
        running.finish()
    }
}
impl Running {
    pub fn wait_status(&mut self) -> io::Result<serde_json::Value> {
        self.wait_for_status(|_| true)
    }

    pub fn wait_ready(&mut self) -> io::Result<()> {
        self.wait_for_status(|status| status["ready"] == true).map(|_| ())
    }

    fn wait_for_status(
        &mut self,
        matches: impl Fn(&serde_json::Value) -> bool,
    ) -> io::Result<serde_json::Value> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(status) = self.client.value("igui.status", json!({})) {
                if matches(&status) {
                    return Ok(status);
                }
            }
            if self.child.as_mut().unwrap().try_wait()?.is_some() {
                return Err(io::Error::other("GUI exited before returning the expected status"));
            }
            if Instant::now() > deadline {
                return Err(io::Error::other("GUI did not return the expected status"));
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    pub fn finish(mut self) -> io::Result<Output> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.child.as_mut().unwrap().try_wait()?.is_none() {
            if Instant::now() > deadline {
                return Err(io::Error::other("GUI did not close"));
            }
            thread::sleep(Duration::from_millis(2));
        }
        let output = self.child.take().unwrap().wait_with_output()?;
        self.host.take().unwrap().join().unwrap();
        let socket = self.socket.clone();
        drop(self);
        assert!(
            !socket.exists(),
            "host must clean the socket and screenshots"
        );
        Ok(output)
    }
}
