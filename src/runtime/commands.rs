// SPDX-License-Identifier: BSD-3-Clause
use super::{Config, effects::apply, launch::LaunchDialog, startup::Session};
use crate::{input::startup::StartupInputTest, storage::screenshot::Screenshot};
use imanlib::{igui::{Action, InputRequest}, protocol::{Request, Response}, surface::Surface};
use serde_json::json;
use std::time::Duration;

pub(super) struct Context<'a> {
    pub options: &'a Config,
    pub app: &'a mut Option<Session>,
    pub notice: &'a mut Option<String>,
    pub launch: &'a mut Option<LaunchDialog>,
    pub startup_test: &'a mut StartupInputTest,
    pub wait_for_release: &'a mut bool,
    pub dirty: &'a mut bool,
    pub screenshot: &'a mut Option<Screenshot>,
    pub surface: &'a Surface,
    pub preferences_writable: bool,
    pub managed_games: bool,
    pub input_test_available: bool,
    pub now: Duration,
}

pub(super) fn handle(request: Request, context: Context<'_>) -> (Response, bool) {
    let Context { options, app, notice, launch, startup_test, wait_for_release, dirty,
        screenshot, surface, preferences_writable, managed_games, input_test_available, now } = context;
    let mut closing = false;
    let response = match request.command.as_str() {
        "igui.status" => Response::done(
            request.id,
            json!({
                "ready":app.is_some(), "launching":launch.is_some(),
                "preferences_writable":preferences_writable, "headless":options.display.headless,
                "input_test_available":input_test_available,
                "input_test_active":app.as_ref().is_some_and(|s| s.app.input_test_active()),
                "input_test_countdown":startup_test.countdown(),
            }),
        ),
        "igui.close" => {
            closing = true;
            Response::done(request.id, json!({"closing":true}))
        }
        "igui.input" => {
            match serde_json::from_value::<InputRequest>(request.params.clone()) {
                Err(e) => Response::error(request.id, "invalid_params", e),
                Ok(params) if startup_test.countdown().is_some() => {
                    let cancelled = params.action == "b";
                    if cancelled {
                        startup_test.cancel();
                        *wait_for_release = true;
                        *dirty = true;
                    }
                    Response::done(request.id, json!({"processed":true,"blocked":!cancelled,"closing":false}))
                }
                Ok(_) if app.as_ref().is_some_and(|s| s.app.input_test_active()) => {
                    Response::done(request.id, json!({"processed":true,"blocked":true,"closing":false}))
                }
                Ok(params) => match Action::parse(&params.action) {
                    None => Response::error(
                        request.id,
                        "invalid_params",
                        "Unknown input action",
                    ),
                    Some(action) => {
                        if let Some(app) = app {
                            if !app.app.input_test_active() {
                                startup_test.update(now.as_millis() as u64, true, true, action == Action::Favourite);
                            }
                            let confirming = app.app.trash_confirmation();
                            let testing = app.app.input_test_active();
                            let blocked = launch.is_some()
                                || testing
                                || (confirming
                                    && !matches!(action, Action::Activate | Action::Favourite));
                            closing = apply(
                                app,
                                action,
                                notice,
                                launch,
                                preferences_writable,
                                managed_games,
                            );
                            *wait_for_release |= confirming != app.app.trash_confirmation();
                            *wait_for_release |= testing != app.app.input_test_active();
                            *dirty |= !blocked;
                            Response::done(
                                request.id,
                                json!({"processed":true,"blocked":blocked,"closing":closing,"notice":notice}),
                            )
                        } else {
                            Response::error(
                                request.id,
                                "not_ready",
                                "Catalogue is not ready; input was not queued",
                            )
                        }
                    }
                },
            }
        }
        "igui.screenshot" => crate::ipc::commands::screenshot(
            request, options.ipc_socket.as_deref().expect("attached IPC socket"), surface,
            screenshot, crate::ipc::commands::Mode::Menu,
        ),
        _ => Response::error(request.id, "unknown_command", "Unknown GUI command"),
    };
    (response, closing)
}
