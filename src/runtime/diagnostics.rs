// SPDX-License-Identifier: BSD-3-Clause
use crate::{display::Display, ipc::Managed, render::RetainedFrame};

/// Submit available display and input reports after GUI work has stopped.
pub(super) fn submit_diagnostics(
    window: &Display,
    frame: &RetainedFrame,
    connection: &mut Managed,
    input: Option<imanlib::input_metrics::Sample>,
    cleanup_ok: bool,
) {
    if connection
        .bootstrap
        .as_ref()
        .is_some_and(|bootstrap_item| bootstrap_item.diag_display)
    {
        if let Err(error) = connection
            .display_report(&window.diagnostic_report(Some(frame.statistics().clone()), cleanup_ok))
        {
            eprintln!("IGUI diag_display submission failed: {error}");
        }
    }
    if let Some(input) = input {
        if let Err(error) = connection.input_report(&input) {
            eprintln!("IGUI diag_input submission failed: {error}");
        }
    }
}
