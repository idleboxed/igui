// SPDX-License-Identifier: BSD-3-Clause
//! Diagnostic UI lifecycle: no catalogue, preferences, ROM launch or device writes.
use crate::{display::Display, ipc::{Managed, Update}};
use {
    imanlib::input_probe::self,
    imanlib::input_probe::linux::usb_inventory,
    imanlib::input_probe::linux::ProfileSource,
    imanlib::input_probe::linux::Reader,
    imanlib::input_probe::LogBudget,
    imanlib::input_probe::Probe,
    imanlib::input_probe::END_MS,
    imanlib::surface::Surface,
};
use imanlib::protocol::Response;
use serde_json::json;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    time::Instant,
};

pub(crate) struct Config<'a> {
    pub headless: bool,
    pub frames: usize,
    pub ipc_socket: Option<&'a Path>,
}

fn flush_records(probe: &mut Probe, budget: &mut LogBudget) -> io::Result<()> {
    let mut output = io::stderr().lock();
    for record in probe.take_records() {
        if let Some(line) = budget.encode(&record) {
            output.write_all(line.as_bytes())?;
        }
    }
    Ok(())
}

/// Run the bounded read-only input probe, handling IPC without writing menu preferences.
pub(crate) fn run(
    options: &Config<'_>,
    managed: &mut Option<Managed>,
    window: &mut Display,
    surface: &mut Surface,
    profiles: Vec<imanlib::gamepad::profile::Profile>,
) -> Result<(), Box<dyn Error>> {
    let mut probe = Probe::default();
    let mut reader = (!options.headless).then(|| Reader::new(ProfileSource::new(profiles)));
    let mut budget = LogBudget::default();
    let mut screenshot = None;
    let mut iteration = 0;
    let started = Instant::now();
    let mut reason = "display_closed_or_signal";
    let mut next_usb = 0;
    let mut last_usb = None;
    let mut usb_snapshots = 0;
    let result = (|| -> Result<(), Box<dyn Error>> {
        flush_records(&mut probe, &mut budget)?;
        while window.is_open() {
            let ms = started.elapsed().as_millis() as u64;
            if !options.headless && ms >= next_usb && usb_snapshots < 8 {
                next_usb = ms.saturating_add(2000);
                let snapshot = usb_inventory(Path::new("/sys/bus/usb/devices"))
                    .unwrap_or_else(|error_item| json!({"error":error_item.to_string()}));
                if last_usb.as_ref() != Some(&snapshot) {
                    let record = json!({"event":"usb_topology","ms":ms,"snapshot":snapshot});
                    if let Some(line) = budget.encode(&record) {
                        io::stderr().write_all(line.as_bytes())?;
                    }
                    last_usb = Some(snapshot);
                    usb_snapshots += 1;
                }
            }
            if let Some(reader) = &mut reader {
                reader.poll(&mut probe, ms);
            }
            flush_records(&mut probe, &mut budget)?;
            if probe.done(ms) {
                reason = "operator_completed";
                break;
            }
            if ms >= END_MS {
                reason = "diagnostic_time_limit";
                break;
            }
            if iteration >= options.frames {
                reason = "frame_limit";
                break;
            }
            input_probe::draw(&probe, ms, surface)?;
            window.present(surface, true)?;
            iteration += 1;
            if let Some(connection) = managed {
                match connection.poll()? {
                    Update::Close => {
                        reason = "host_close";
                        break;
                    }
                    Update::Failed(error) => return Err(error.into()),
                    Update::HandedOff | Update::Ready => {
                        return Err("unexpected game handoff during input probe".into())
                    }
                    Update::Waiting => {}
                }
                if let Some(request) = connection.take_request() {
                    let mut closing = false;
                    let response = match request.command.as_str() {
                        "igui.status" => Response::done(
                            request.id,
                            json!({
                                "ready":true,"launching":false,"preferences_writable":false,"headless":options.headless,
                                "input_probe":true,"stage":probe.stage,"connected_streams":probe.streams.iter().flatten().count(),
                                "physical_players_verified":false,
                            }),
                        ),
                        "igui.close" => {
                            closing = true;
                            Response::done(request.id, json!({"closing":true}))
                        }
                        "igui.input" => Response::error(
                            request.id,
                            "diagnostic_mode",
                            "Logical input disabled during raw probe",
                        ),
                        "igui.screenshot" => crate::ipc::commands::screenshot(
                            request, options.ipc_socket.expect("attached IPC socket"),
                            surface, &mut screenshot, crate::ipc::commands::Mode::Probe,
                        ),
                        _ => Response::error(request.id, "unknown_command", "Unknown GUI command"),
                    };
                    connection.reply(response)?;
                    if closing {
                        reason = "ipc_close";
                        break;
                    }
                }
            }
        }
        Ok(())
    })();
    // Always release input FDs before final display cleanup / host acknowledgement.
    drop(reader);
    let mut summary = probe.finish(
        started.elapsed().as_millis() as u64,
        if result.is_ok() { reason } else { "error" },
    );
    flush_records(&mut probe, &mut budget)?;
    summary["log_records_omitted"] = json!(budget.omitted);
    summary["headless"] = json!(options.headless);
    summary["usb_snapshots"] = json!(usb_snapshots);
    summary["usb_snapshot_limit_reached"] = json!(usb_snapshots == 8);
    writeln!(io::stderr().lock(), "IGUI input: {summary}")?;
    let cleanup = window.finish();
    if let Some(connection) = managed {
        let mut display = window.diagnostic_report(None, cleanup.is_ok());
        display.operation_ok = Some(result.is_ok());
        if let Err(error) = connection.display_report(&display) {
            eprintln!("IGUI diag_display submission failed: {error}");
        }
        if let Err(error) = connection.input_report(&probe.diagnostic_report(options.headless)) {
            eprintln!("IGUI diag_input submission failed: {error}");
        }
    }
    cleanup?;
    result?;
    if let Some(connection) = managed {
        connection.closed()?;
    }
    Ok(())
}
