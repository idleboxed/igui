// SPDX-License-Identifier: BSD-3-Clause
//! GUI-side RPC state machine. No launcher dependency or blocking frame-loop waits.
pub(crate) mod commands;
use imanlib::{
    client::Client,
    igui::{Bootstrap, Handoff, InputTestAcknowledgement, LaunchRequest},
    protocol::{Outcome, Request, Response},
};
use serde_json::json;
use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};

pub enum Update {
    Waiting,
    Ready,
    HandedOff,
    Close,
    Failed(String),
}
enum Pending {
    Prepare,
    Handoff,
    InputTest,
}
pub struct Managed {
    client: Client,
    pub bootstrap: Option<Bootstrap>,
    pending: Option<(u64, Pending, Instant)>,
    ready: bool,
    handed_off: bool,
    closing: bool,
}
impl Managed {
    /// Connect to iman and read this managed GUI session's bootstrap settings.
    pub fn connect(path: &Path) -> io::Result<Self> {
        Self::open(path, true)
    }
    /// Register a standalone GUI with its authorized IPC test host.
    pub fn attach(path: &Path) -> io::Result<Self> {
        Self::open(path, false)
    }
    fn open(path: &Path, managed: bool) -> io::Result<Self> {
        let mut client = Client::connect(path)?;
        let bootstrap = if managed {
            Some(serde_json::from_value(
                client.value("igui.bootstrap", json!({}))?,
            )?)
        } else {
            client.value("igui.attach", json!({}))?;
            None
        };
        Ok(Self {
            client,
            bootstrap,
            pending: None,
            ready: false,
            handed_off: false,
            closing: false,
        })
    }
    /// Take the next forwarded GUI-control request, if one has arrived.
    pub fn take_request(&mut self) -> Option<Request> {
        self.client.take_request()
    }
    /// Queue a processed GUI command's actual outcome.
    pub fn reply(&mut self, response: Response) -> io::Result<()> {
        self.client.reply(response)
    }
    /// Request preparation of one selected game when no handoff is already pending.
    pub fn prepare(&mut self, game: LaunchRequest) -> io::Result<()> {
        if self.pending.is_none() && !self.ready && !self.closing && !self.handed_off {
            let id = self
                .client
                .send("igui.prepare_launch", serde_json::to_value(game)?)?;
            self.pending = Some((id, Pending::Prepare, Instant::now()));
        }
        Ok(())
    }
    /// Request game handoff only after iman has confirmed preparation.
    pub fn handoff(&mut self, value: Handoff) -> io::Result<()> {
        if self.ready && self.pending.is_none() && !self.closing && !self.handed_off {
            let id = self
                .client
                .send("igui.handoff", serde_json::to_value(value)?)?;
            self.pending = Some((id, Pending::Handoff, Instant::now()));
        }
        Ok(())
    }
    /// Process IPC outcomes and close events without blocking the frame loop.
    pub fn poll(&mut self) -> io::Result<Update> {
        let mut update = Update::Waiting;
        if let Some(response) = self.client.poll()? {
            let Some((id, kind, _)) = self.pending.take() else {
                return Err(io::Error::other("unexpected GUI IPC response"));
            };
            if id != response.id {
                return Err(io::Error::other("GUI IPC response ID mismatch"));
            }
            match response.outcome {
                Outcome::Done { result } => match kind {
                    Pending::Prepare => {
                        self.ready = true;
                        update = Update::Ready;
                    }
                    Pending::Handoff => {
                        self.handed_off = true;
                        update = Update::HandedOff;
                    }
                    Pending::InputTest => {
                        let acknowledgement: InputTestAcknowledgement = serde_json::from_value(result)
                            .map_err(|_| io::Error::other("Invalid input test handoff acknowledgement"))?;
                        if !acknowledgement.handoff_received || acknowledgement.test_started {
                            return Err(io::Error::other("Invalid input test handoff acknowledgement"));
                        }
                        self.handed_off = true;
                        update = Update::HandedOff;
                    }
                },
                Outcome::Error { error } => {
                    self.ready = false;
                    update = Update::Failed(error.to_string());
                }
                Outcome::Accepted { .. } => {
                    return Err(io::Error::other("unexpected async GUI RPC"))
                }
            }
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|(_, _, pending_started)| pending_started.elapsed() > Duration::from_secs(5))
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "GUI IPC timed out; not retried",
            ));
        }
        if self.client.close_requested() {
            self.closing = true;
            update = Update::Close;
        }
        Ok(update)
    }
    /// Transfers ownership, not a journal. The host must reap this process first.
    pub fn start_input_test(&mut self) -> io::Result<()> {
        if !self.bootstrap.as_ref().is_some_and(|bootstrap| bootstrap.input_test)
            || self.pending.is_some() || self.ready || self.closing || self.handed_off {
            return Err(io::Error::other("Input test handoff is not available"));
        }
        let id = self.client.send("igui.input_test", json!({}))?;
        self.pending = Some((id, Pending::InputTest, Instant::now()));
        Ok(())
    }
    /// Best-effort diagnostics are sent only after rendering/input work has stopped.
    pub fn display_report(&mut self, sample: &imanlib::display_metrics::Sample) -> io::Result<()> {
        if self.bootstrap.as_ref().is_some_and(|bootstrap| bootstrap.diag_display) {
            self.drain_pending()?;
            self.client
                .value("diag_display.submit", serde_json::to_value(sample)?)?;
        }
        Ok(())
    }
    /// Send best-effort input diagnostics after pending GUI operations have settled.
    pub fn input_report(&mut self, sample: &imanlib::input_metrics::Sample) -> io::Result<()> {
        if self.bootstrap.as_ref().is_some_and(|bootstrap| bootstrap.diag_input) {
            self.drain_pending()?;
            self.client
                .value("diag_input.submit", serde_json::to_value(sample)?)?;
        }
        Ok(())
    }
    fn drain_pending(&mut self) -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.pending.is_some() {
            if Instant::now() >= deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "GUI report drain timed out"));
            }
            self.poll()?;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
    /// Called after display cleanup, never within the rendering loop.
    pub fn closed(&mut self) -> io::Result<()> {
        self.drain_pending()?;
        if !self.handed_off {
            self.client.value("igui.closed", json!({}))?;
        }
        Ok(())
    }
}
