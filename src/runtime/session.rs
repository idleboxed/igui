// SPDX-License-Identifier: BSD-3-Clause
use super::{
    diagnostics::submit_diagnostics,
    effects::{apply, launch_request},
    launch::LaunchDialog,
    startup::{load_session_with_cancellation, Session},
    Config,
};
use crate::{display, memory, paths, render::{Renderer, RetainedFrame, HEIGHT, WIDTH}, storage};
use imanlib::{igui::{Action, Handoff}, surface::Surface};
use std::{error::Error, path::PathBuf};

fn renderer(root: PathBuf) -> Renderer {
    Renderer::with_previews(move |hash| storage::load_preview(&root, hash))
}

pub(super) fn run(options: Config) -> Result<(), Box<dyn Error>> {
    use crate::input::NavigationInput;
    use crate::{
        runtime::loading::{CatalogLoader, Stage},
        locales::Locales,
        render::splash,
    };
    use std::time::{Duration, Instant};
    #[cfg(target_os = "linux")]
    let profiles = crate::input::load_profiles(&options.input, options.display.headless)?;
    #[cfg(target_os = "linux")]
    for conflict in &profiles.conflicts {
        eprintln!("IGUI controls: {conflict}");
    }
    let mut managed = options
        .ipc_socket
        .as_deref()
        .map(|path| {
            if options.managed {
                crate::ipc::Managed::connect(path)
            } else {
                crate::ipc::Managed::attach(path)
            }
        })
        .transpose()?;
    let bootstrap = managed.as_ref().and_then(|m| m.bootstrap.clone());
    let preferences_writable = !options.input.input_probe
        && bootstrap
            .as_ref()
            .map_or(true, |bootstrap| bootstrap.preferences_writable);
    let managed_games = bootstrap.is_some();
    let mut splash_renderer = splash::RetainedSplash::default();
    let mut surface = Surface::new(WIDTH, HEIGHT);
    if !options.input.input_probe {
        splash_renderer.update(Duration::ZERO, Stage::Starting, &Locales::default(), &mut surface)?;
    }
    let mut window = display::Display::open(&options.display, &surface)?;
    if bootstrap.as_ref().is_some_and(|b| b.diag_display) {
        window.enable_diagnostics();
    }
    #[cfg(target_os = "linux")]
    if options.input.input_probe {
        let probe = crate::input::probe::Config {
            headless: options.display.headless,
            frames: options.frames.ok_or("input probe needs a frame limit")?,
            ipc_socket: options.ipc_socket.as_deref(),
        };
        return crate::input::probe::run(&probe, &mut managed, &mut window, &mut surface, profiles.evdev);
    }
    let root = bootstrap
        .as_ref()
        .map(|b| b.root.join(paths::GAMES_DIR))
        .or_else(|| options.games.clone())
        .ok_or("session has no games directory")?;
    let mut frame = RetainedFrame::new(surface, renderer(root.clone()));
    let locales_root = paths::executable_directory()?.join(paths::LOCALES_DIR);
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let mut splash_locales = Locales::default();
    let mut stage = if cfg!(target_os = "linux") { Stage::Gamepad } else { Stage::Starting };
    let started = Instant::now();
    #[cfg(target_os = "linux")]
    let mut gamepad = {
        let changed = splash_renderer.update(
            started.elapsed(),
            stage,
            &splash_locales,
            frame.surface_mut(),
        )?;
        window.present(frame.surface(), changed)?;
        let mut gamepad = crate::input::gamepad::Gamepad::with_profiles(
            options.input.gamepad.clone(),
            options.input.gamepad_buttons,
            options.input.gamepad_select,
            profiles.hid,
            profiles.evdev,
        );
        if bootstrap.as_ref().is_some_and(|b| b.diag_input) {
            gamepad.enable_diagnostics();
        }
        if !options.display.headless {
            gamepad.poll(started.elapsed());
        }
        gamepad
    };
    // Window/input processing is already running before catalog work starts.
    let delay = options.demo_load_ms.unwrap_or(0);
    let mut loader = Some(CatalogLoader::spawn_value(move |cancellation| {
        load_session_with_cancellation(&root, &locales_root, cancellation, |stage, locales| {
            let _ = progress_tx.send((stage, locales.cloned()));
            if stage == Stage::Catalogue && delay != 0 {
                let _ = cancellation.wait_for(Duration::from_millis(delay));
            }
        })
    })?);
    let mut app: Option<Session> = None;
    let mut notice = None;
    let mut launch: Option<LaunchDialog> = None;
    let mut logo_visible = true;
    let mut dirty = true;
    let mut iteration = 0;
    let mut input = NavigationInput::default();
    let mut wait_for_release = false;
    let mut screenshot = None;
    let mut memory_monitor = memory::MemoryMonitor::default();
    let mut escape_held = false;
    let mut startup_test = crate::input::startup::StartupInputTest::default();
    let mut previous_buttons = imanlib::input::Buttons::default();
    let mut previous_countdown = None;
    'session: while window.is_alive() {
        if app.is_none() && window.escape_down() { break; }
        let mut present_changed = false;
        if let Some(connection) = &mut managed {
            match connection.poll()? {
                crate::ipc::Update::HandedOff => {
                    drop(loader);
                    if options.display.drm {
                        eprintln!("IGUI render: {:?}", frame.statistics());
                    }
                    let cleanup = window.finish();
                    submit_diagnostics(
                        &window,
                        &frame,
                        connection,
                        {
                            #[cfg(target_os = "linux")]
                            {
                                gamepad.diagnostics().map(|mut s| {
                                    s.headless = options.display.headless;
                                    s
                                })
                            }
                            #[cfg(not(target_os = "linux"))]
                            {
                                None
                            }
                        },
                        cleanup.is_ok(),
                    );
                    cleanup?;
                    return Ok(());
                }
                crate::ipc::Update::Close => break,
                crate::ipc::Update::Failed(error) => {
                    eprintln!("IGUI launch: {error}");
                    notice = Some(crate::render::notice::launch_failed(&error));
                    launch = None;
                    if let Some(app) = &mut app { app.app.close_input_test(); }
                    dirty = true;
                    wait_for_release = true;
                }
                _ => {}
            }
            if managed_games && launch.is_some() {
                let session: &Session = app.as_ref().ok_or("launch without a session")?;
                let game = launch_request(session).ok_or("launch without a selected game")?;
                connection.prepare(game.clone())?;
                if launch
                    .as_ref()
                    .is_some_and(|d| d.minimum_elapsed(started.elapsed()))
                {
                    connection.handoff(Handoff {
                        game,
                        state: session.app.session_state(),
                    })?;
                }
            }
        }
        if !managed_games
            && launch
                .as_ref()
                .is_some_and(|d| d.minimum_elapsed(started.elapsed()))
        {
            launch = None;
            dirty = true;
            wait_for_release = true;
        }
        let mut stage_changed = false;
        // Display the latest actual operation; do not replay completed stages
        // with artificial pauses just to make their captions readable.
        for (next, locales) in progress_rx.try_iter() {
            stage = next;
            stage_changed = true;
            if let Some(locales) = locales {
                splash_locales = locales;
            }
        }
        if let Some(pending) = &loader {
            if let Some(mut ready) = pending.poll()? {
                if let Some(bootstrap) = &bootstrap {
                    if bootstrap.state != Default::default() {
                        ready.app.restore_session(&bootstrap.state);
                    }
                    ready.app.enable_input_test(bootstrap.input_test);
                    notice = bootstrap.notice.clone();
                }
                if ready.app.selected_game().is_some() {
                    let changed = splash_renderer.update(
                        started.elapsed(),
                        Stage::Preview,
                        ready.app.locales(),
                        frame.surface_mut(),
                    )?;
                    window.present(frame.surface(), changed)?;
                    frame.prepare_selected(&ready.app);
                }
                let changed = splash_renderer.update(
                    started.elapsed(),
                    Stage::Interface,
                    ready.app.locales(),
                    frame.surface_mut(),
                )?;
                window.present(frame.surface(), changed)?;
                app = Some(ready);
                loader = None;
                wait_for_release = true;
                dirty = true;
            } else {
                let elapsed = started.elapsed();
                let visible = splash::logo_visible(elapsed);
                if visible != logo_visible || stage_changed {
                    present_changed = splash_renderer.update(elapsed, stage, &splash_locales, frame.surface_mut())?;
                    logo_visible = visible;
                }
            }
        }
        let mut buttons = window.buttons();
        #[cfg(target_os = "linux")]
        if !options.display.headless {
            buttons = buttons.union(gamepad.poll(started.elapsed()));
        }
        if let Some(app) = &mut app {
            if app.app.input_test_enabled() {
                use crate::input::startup::Transition;
                let b_pressed = buttons.down(Action::Favourite) && !previous_buttons.down(Action::Favourite);
                match startup_test.update(started.elapsed().as_millis() as u64, true, buttons.any(), b_pressed) {
                    Transition::Start => {
                        if let Some(connection) = &mut managed {
                            connection.start_input_test()?;
                            app.app.start_input_test();
                        }
                        wait_for_release = true;
                    }
                    Transition::Cancelled => { dirty = true; wait_for_release = true; }
                    Transition::None => {}
                }
            }
            let escape = window.escape_down();
            if escape && !escape_held && !app.app.input_test_active() && startup_test.countdown().is_none() {
                break;
            }
            escape_held = escape;
            dirty |= memory_monitor.refresh(started.elapsed(), memory::read);
            app.app.set_memory(memory_monitor.info());
            if (dirty || startup_test.countdown() != previous_countdown) && !app.app.input_test_active() {
                present_changed = frame
                    .update_with_countdown(&app.app, notice.as_deref(), launch.as_ref(), Some(&startup_test))?
                    .changed();
            }
            present_changed |= frame.advance_title(
                started.elapsed(), window.is_active() && !app.app.input_test_active(),
            )?.changed();
            previous_countdown = startup_test.countdown();
            dirty = false;
        }
        previous_buttons = buttons;
        // Present the cached frame even when idle: X11 can invalidate it on map
        // or Expose. minifb's update() only polls events and must not be mixed
        // with update_with_buffer(), which already polls input as well.
        window.present(frame.surface(), present_changed)?;
        if let Some(dialog) = &mut launch {
            dialog.presented(started.elapsed());
        }
        if let Some(connection) = &mut managed {
            if let Some(request) = connection.take_request() {
                let (response, closing) = super::commands::handle(request, super::commands::Context {
                    options: &options, app: &mut app, notice: &mut notice, launch: &mut launch,
                    startup_test: &mut startup_test, wait_for_release: &mut wait_for_release,
                    dirty: &mut dirty, screenshot: &mut screenshot, surface: frame.surface(),
                    preferences_writable, managed_games,
                    input_test_available: bootstrap.as_ref().is_some_and(|bootstrap| bootstrap.input_test),
                    now: started.elapsed(),
                });
                connection.reply(response)?;
                if closing {
                    break 'session;
                }
            }
        }
        iteration += 1;
        if options.frames.is_some_and(|max| iteration >= max) {
            break;
        }
        let Some(app) = &mut app else {
            input.suspend(buttons);
            continue;
        };
        if launch.is_some() || app.app.input_test_active() || startup_test.countdown().is_some() {
            input.suspend(buttons);
            continue;
        }
        if !window.is_active() {
            input.suspend(buttons);
            continue;
        }
        // Inputs during the splash must not launch/filter a game or move focus on transition.
        if wait_for_release {
            input.suspend(buttons);
            if buttons.any() || window.escape_down() {
                continue;
            }
            wait_for_release = false;
        }
        for action in input.poll(buttons, started.elapsed()) {
            let before = app.app.focus();
            let confirming = app.app.trash_confirmation();
            let testing = app.app.input_test_active();
            if apply(app, action, &mut notice, &mut launch, preferences_writable, managed_games) {
                break 'session;
            }
            dirty = true;
            if launch.is_some() || confirming != app.app.trash_confirmation()
                || testing != app.app.input_test_active() {
                input.suspend(buttons);
                wait_for_release = true;
                break;
            }
            if app.app.focus() != before {
                input.block_until_release(action);
                break;
            }
        }
    }
    // Complete metadata work before releasing the display or acknowledging close.
    drop(loader);
    if options.display.drm {
        eprintln!("IGUI render: {:?}", frame.statistics());
    }
    let cleanup = window.finish();
    if let Some(connection) = &mut managed {
        #[cfg(target_os = "linux")]
        let input = gamepad.diagnostics().map(|mut s| {
            s.headless = options.display.headless;
            s
        });
        #[cfg(not(target_os = "linux"))]
        let input = None;
        submit_diagnostics(&window, &frame, connection, input, cleanup.is_ok());
    }
    cleanup?;
    if let Some(connection) = &mut managed {
        connection.closed()?;
    }
    drop(screenshot);
    Ok(())
}
