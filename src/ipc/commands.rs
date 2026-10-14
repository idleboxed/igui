// SPDX-License-Identifier: BSD-3-Clause
//! Shared PNG command handling; each mode retains its existing error contract.
use crate::storage::screenshot::{self, Screenshot};
use imanlib::{
    igui::{ScreenshotRequest, ScreenshotResult},
    protocol::{Request, Response},
    surface::Surface,
};
use serde_json::json;
use std::{io, path::Path};

pub(crate) enum Mode {
    Menu,
    Probe,
}

/// Capture the supplied surface in a checked destination and return its IPC outcome.
pub(crate) fn screenshot(
    request: Request,
    socket: &Path,
    surface: &Surface,
    retained: &mut Option<Screenshot>,
    mode: Mode,
) -> Response {
    let params = match serde_json::from_value::<ScreenshotRequest>(request.params) {
        Ok(params) => params,
        Err(error) => return Response::error(request.id, "invalid_params", error),
    };
    let capture = if let Some(path) = params.path {
        screenshot::save(&path, surface).map(|()| (path, false))
    } else {
        Screenshot::capture(socket, surface).map(|capture| {
            let path = capture.path.clone();
            *retained = Some(capture);
            (path, true)
        })
    };
    match capture {
        Ok((path, temporary)) => Response::done(
            request.id,
            json!(ScreenshotResult {
                path,
                temporary,
                width: surface.width(),
                height: surface.height(),
                format: "png".into(),
            }),
        ),
        Err(error) => {
            let code = if matches!(mode, Mode::Menu) && error.kind() == io::ErrorKind::InvalidInput {
                "invalid_params"
            } else {
                "screenshot_failed"
            };
            Response::error(request.id, code, error)
        }
    }
}
