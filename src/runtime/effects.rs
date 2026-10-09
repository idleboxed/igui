// SPDX-License-Identifier: BSD-3-Clause
use super::startup::Session;
use super::launch::LaunchDialog;
use crate::{model::Effect, storage};
use imanlib::igui::{Action, LaunchRequest};

pub(super) fn apply(
    session: &mut Session,
    action: Action,
    notice: &mut Option<String>,
    launch: &mut Option<LaunchDialog>,
    preferences_writable: bool,
    managed_games: bool,
) -> bool {
    // Launch input is blocked; trash confirmation is handled by the model.
    if launch.is_some() || session.app.input_test_active() {
        return false;
    }
    if !preferences_writable && action == Action::Trash {
        return false;
    }
    let effect = session.app.dispatch(action);
    if !preferences_writable
        && matches!(
            effect,
            Effect::SaveFavourite { .. } | Effect::SaveTrash { .. }
        )
    {
        return false;
    }
    *notice = match effect {
        Effect::None => None,
        Effect::Exit => return true,
        Effect::Launch {
            title, platform, ..
        } => {
            let mut save_notice = None;
            // A standalone dialog does not launch a game. Diagnostic read-only
            // sessions must not write even when a host permits game handoff.
            if managed_games && preferences_writable {
                let hash = &session.app.selected_game().expect("launch requires a game").hash;
                if let Err(error) = session.preferences.save_current_item(hash) {
                    eprintln!("IGUI: {error}");
                    save_notice = Some(session.app.text("Save not confirmed.").into());
                }
            }
            *launch = Some(LaunchDialog::new(title, platform));
            save_notice
        }
        Effect::SaveFavourite { hash, enabled } => {
            match session.preferences.save_favourite(&hash, enabled) {
                Ok(()) => {
                    session
                        .app
                        .set_favourites(session.preferences.favourites().clone());
                    session
                        .app
                        .set_available_bytes(storage::available_bytes(&session.root));
                    None
                }
                Err(error) => {
                    eprintln!("IGUI: {error}");
                    Some(session.app.text("Save not confirmed.").into())
                }
            }
        }
        Effect::SaveTrash { hash, enabled } => {
            match session.preferences.save_trash(&hash, enabled) {
                Ok(()) => {
                    session.app.set_trash(session.preferences.trash().to_vec());
                    session
                        .app
                        .set_available_bytes(storage::available_bytes(&session.root));
                    None
                }
                Err(error) => {
                    eprintln!("IGUI: {error}");
                    Some(session.app.text("Save not confirmed.").into())
                }
            }
        }
    };
    false
}

pub(super) fn launch_request(session: &Session) -> Option<LaunchRequest> {
    session.app.selected_game().map(|game| LaunchRequest {
        hash: game.hash.clone(),
        image: game.image.clone(),
        platform: game.platform.clone(),
    })
}
