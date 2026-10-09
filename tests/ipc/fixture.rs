// SPDX-License-Identifier: BSD-3-Clause
//! Owned scripted IPC peer, including cancellation before or after accepting a client.
use igui::ipc::Managed;
use imanlib::{
    igui::{Bootstrap, GuiState},
    protocol::{Message, Request, Response, REQUEST_LIMIT},
};
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct Server {
    directory: tempfile::TempDir,
    stopped: Arc<AtomicBool>,
    active_socket: Arc<Mutex<Option<UnixStream>>>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub fn bind(serve: impl FnOnce(UnixStream) + Send + 'static) -> Self {
        Self::bind_with_diagnostics(false, serve)
    }

    pub fn bind_with_diagnostics(enabled: bool, serve: impl FnOnce(UnixStream) + Send + 'static) -> Self {
        Self::bind_with_input_test(enabled, false, serve)
    }

    pub fn bind_with_input_test(
        diagnostics: bool,
        input_test: bool,
        serve: impl FnOnce(UnixStream) + Send + 'static,
    ) -> Self {
        let directory = tempfile::tempdir_in("/dev/shm").unwrap();
        let listener = UnixListener::bind(directory.path().join("control.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let active_socket = Arc::new(Mutex::new(None));
        let worker_stop = stopped.clone();
        let worker_socket = active_socket.clone();

        let worker = thread::spawn(move || {
            let mut socket = loop {
                if worker_stop.load(Ordering::Relaxed) {
                    return;
                }
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::park_timeout(Duration::from_millis(1));
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => panic!("accept scripted IPC client: {error}"),
                }
            };
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            socket.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
            let shutdown_socket = socket.try_clone().unwrap();
            {
                let mut active = worker_socket.lock().unwrap();
                // Serialize publishing the stream with Drop's shutdown, avoiding a missed cancellation.
                if worker_stop.load(Ordering::Relaxed) {
                    return;
                }
                *active = Some(shutdown_socket);
            }
            let request = receive(&mut socket);
            assert_eq!(request.command, "igui.bootstrap");
            send(
                &mut socket,
                Message::Response(Response::done(
                    request.id,
                    serde_json::to_value(Bootstrap {
                        diag_display: diagnostics,
                        diag_input: diagnostics,
                        input_test,
                        preferences_writable: true,
                        root: "/synthetic".into(),
                        state: GuiState::default(),
                        notice: None,
                    }).unwrap(),
                )),
            );
            serve(socket);
        });
        Self { directory, stopped, active_socket, worker: Some(worker) }
    }

    pub fn directory(&self) -> &Path {
        self.directory.path()
    }

    pub fn connect(&self) -> Managed {
        Managed::connect(&self.directory.path().join("control.sock")).unwrap()
    }

    pub fn join(&mut self) {
        let result = self.worker.take().unwrap().join();
        self.active_socket.lock().unwrap().take();
        result.unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        let mut active = self.active_socket.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(socket) = active.take() {
            if let Err(error) = socket.shutdown(Shutdown::Both) {
                if error.kind() != io::ErrorKind::NotConnected {
                    eprintln!("IGUI scripted peer shutdown failed: {error}");
                }
            }
        }
        drop(active);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            if worker.join().is_err() && !thread::panicking() {
                eprintln!("IGUI scripted peer worker failed during cleanup");
            }
        }
    }
}

pub fn receive(socket: &mut UnixStream) -> Request {
    let mut header = [0; 4];
    socket.read_exact(&mut header).unwrap();
    let size = u32::from_be_bytes(header) as usize;
    assert!(size <= REQUEST_LIMIT);
    let mut bytes = vec![0; size];
    socket.read_exact(&mut bytes).unwrap();
    match serde_json::from_slice(&bytes).unwrap() {
        Message::Request(request) => request,
        _ => panic!("not a request"),
    }
}

pub fn send(socket: &mut UnixStream, message: Message) {
    let bytes = serde_json::to_vec(&message).unwrap();
    socket.write_all(&(bytes.len() as u32).to_be_bytes()).unwrap();
    socket.write_all(&bytes).unwrap();
}
