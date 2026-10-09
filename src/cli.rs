// SPDX-License-Identifier: BSD-3-Clause
use igui::{paths, runtime::Config};
use std::error::Error;

fn usage() {
    println!(
        "IGUI\n\
        Gamepad: D-pad / A (confirm) / Start (launch) / B (favourite) / Y (top filters) / X (bottom filters) / Select (trash). Keyboard: arrows / Enter or A / B / Y / X / Delete. Esc closes the PC window, or cancels an active input test.\n\
        --headless              Run without display/input devices; requires --ipc-socket\n\
        --games DIRECTORY       Required standalone: read {catalogue} from DIRECTORY; {prefs} from its parent\n\
        --controls DIRECTORY    Read controller JSON (default: controls/ beside the executable)\n\
        --gamepad {input_dir}/jsN  Select a Linux joystick (default: auto-detect)\n\
        --gamepad-buttons X,B,Y,A Override logical X/B/Y/A with joystick button indices\n\
        --gamepad-select N      Override Select with a joystick button index\n\
        --hid-profile PATH     Override the default HID JSON file\n\
        --evdev-profile PATH   Override default evdev JSON files with one profile\n\
        --input-probe           Read-only guided four-pad diagnostic (up to 8 streams); requires --frames\n\
        --frames N              Close the display after N iterations (optional time limit)\n\
        --demo-load-ms N        Simulate slow metadata loading (0..60000 ms)\n\
        --drm                   Linux DRM/KMS 1280x720\n\
        --drm-device PATH       Primary node (default /dev/dri/card0); no auto-probing\n\
        --drm-connector ID      Select one connector when multiple outputs are connected\n\
        --allow-vendor-einval    Only with --drm: try modeset after the observed SET_MASTER EINVAL\n\
        --ipc-socket PATH       Connect to a session host (iman or another launcher)\n\
        --managed               Obtain session configuration and game handoff from host\n\
        --help                  Show this help\n\
        Standalone: launch dialog only. Under iman: save current_item when writable, hand off the selected ROM and exit; a save error does not prevent launch. No mount changes or exit-time writes. B saves favourites; Select/Delete asks to move to trash (A yes, B no), or restores immediately. Explicit saves write {prefs}; diagnostic policy is supplied internally by iman.",
        catalogue = paths::CATALOGUE_FILE,
        prefs = paths::PREFERENCES_FILE,
        input_dir = imanlib::input::INPUT_DIR,
    );
}

fn parse_options() -> Result<Option<Config>, Box<dyn Error>> {
    let mut options = Config::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--managed" if !options.managed => options.managed = true,
            "--ipc-socket" if options.ipc_socket.is_none() => {
                options.ipc_socket = Some(args.next().ok_or("--ipc-socket needs a path")?.into());
            }
            "--drm" if !options.display.drm => options.display.drm = true,
            "--drm-device" if options.display.drm_device.is_none() => {
                options.display.drm_device = Some(args.next().ok_or("--drm-device needs a path")?.into());
            }
            "--drm-connector" if options.display.drm_connector.is_none() => {
                let id = args.next().ok_or("--drm-connector needs an ID")?.parse()?;
                if id == 0 {
                    return Err("DRM connector ID must be positive".into());
                }
                options.display.drm_connector = Some(id);
            }
            "--allow-vendor-einval" if !options.display.allow_vendor_einval => {
                options.display.allow_vendor_einval = true
            }
            "--help" | "-h" => {
                usage();
                return Ok(None);
            }
            "--headless" if !options.display.headless => options.display.headless = true,
            "--input-probe" if !options.input.input_probe => options.input.input_probe = true,
            "--games" | "--controls" | "--gamepad" | "--hid-profile" | "--evdev-profile" => {
                let field = if arg == "--games" {
                    &mut options.games
                } else if arg == "--controls" {
                    &mut options.input.controls
                } else if arg == "--hid-profile" {
                    &mut options.input.hid_profile
                } else if arg == "--evdev-profile" {
                    &mut options.input.evdev_profile
                } else {
                    &mut options.input.gamepad
                };
                if field.is_some() {
                    return Err(format!("duplicate {arg}").into());
                }
                *field = Some(
                    args.next()
                        .ok_or_else(|| format!("{arg} needs a directory"))?
                        .into(),
                );
            }
            "--gamepad-buttons" => {
                if options.input.gamepad_buttons.is_some() {
                    return Err("duplicate --gamepad-buttons".into());
                }
                let value = args
                    .next()
                    .ok_or("--gamepad-buttons needs X,B,Y,A indices")?;
                let values = value
                    .split(',')
                    .map(str::parse::<u8>)
                    .collect::<Result<Vec<_>, _>>()?;
                let [x, b, y, a] = values.as_slice() else {
                    return Err("--gamepad-buttons needs exactly four indices: X,B,Y,A".into());
                };
                if x == b || x == y || b == y || a == x || a == b || a == y {
                    return Err("X, B, Y and A must be different buttons".into());
                }
                options.input.gamepad_buttons = Some((*x, *b, *y, *a));
            }
            "--gamepad-select" => {
                if options.input.gamepad_select.is_some() {
                    return Err("duplicate --gamepad-select".into());
                }
                options.input.gamepad_select = Some(
                    args.next()
                        .ok_or("--gamepad-select needs a button index")?
                        .parse()?,
                );
            }
            "--frames" if options.frames.is_none() => {
                let frames: usize = args
                    .next()
                    .ok_or("--frames needs a positive integer")?
                    .parse()?;
                if frames == 0 {
                    return Err("--frames must be positive".into());
                }
                options.frames = Some(frames);
            }
            "--demo-load-ms" => {
                if options.demo_load_ms.is_some() {
                    return Err("duplicate --demo-load-ms".into());
                }
                let delay = args
                    .next()
                    .ok_or("--demo-load-ms needs milliseconds")?
                    .parse()?;
                if delay > 60_000 {
                    return Err("--demo-load-ms must be in 0..60000".into());
                }
                options.demo_load_ms = Some(delay);
            }
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    if !options.display.drm
        && (options.display.allow_vendor_einval
            || options.display.drm_device.is_some()
            || options.display.drm_connector.is_some())
    {
        return Err("DRM options require --drm".into());
    }
    if options.display.drm {
        if !cfg!(all(feature = "drm", target_os = "linux")) {
            return Err("DRM support is not enabled in this build".into());
        }
        if options.display.headless || options.demo_load_ms.is_some() {
            return Err("--drm forbids --headless and --demo-load-ms".into());
        }
        if !options.managed && options.games.is_none() {
            return Err("standalone --drm requires --games".into());
        }
    }
    if (options.input.hid_profile.is_some() || options.input.evdev_profile.is_some())
        && (!cfg!(target_os = "linux")
            || options.display.headless
            || options.input.input_probe
            || options.input.gamepad.is_some()
            || options.input.gamepad_buttons.is_some()
            || options.input.gamepad_select.is_some())
    {
        return Err("JSON input profiles require normal Linux input; no headless, probe or joydev overrides".into());
    }
    if options.input.input_probe {
        if !cfg!(target_os = "linux")
            || options.frames.is_none_or(|n| n > 18_000)
            || options.input.gamepad.is_some()
            || options.input.gamepad_buttons.is_some()
            || options.input.gamepad_select.is_some()
            || options.demo_load_ms.is_some()
        {
            return Err("--input-probe requires Linux and --frames 1..18000; no gamepad overrides or demo delay".into());
        }
    }
    if options.display.headless && options.frames.is_some() && !options.input.input_probe {
        return Err("--frames is only available in window mode".into());
    }
    if options.managed && options.ipc_socket.is_none() {
        return Err("--managed requires --ipc-socket; pipes are unsupported".into());
    }
    if options.managed && options.games.is_some() {
        return Err("--managed forbids --games override".into());
    }
    if options.display.headless && options.ipc_socket.is_none() {
        return Err("--headless requires --ipc-socket".into());
    }
    if options.input.gamepad_buttons.is_some() && options.input.gamepad.is_none() {
        return Err("--gamepad-buttons requires --gamepad".into());
    }
    if options.input.gamepad_select.is_some() && options.input.gamepad.is_none() {
        return Err("--gamepad-select requires --gamepad".into());
    }
    if let (Some(select), Some((x, b, y, a))) = (options.input.gamepad_select, options.input.gamepad_buttons) {
        if [x, b, y, a].contains(&select) {
            return Err("Select must differ from X, B, Y and A".into());
        }
    }
    if options.display.headless && options.input.gamepad.is_some() {
        return Err("--gamepad is only available in window mode".into());
    }
    #[cfg(not(target_os = "linux"))]
    if options.input.gamepad.is_some() {
        return Err("gamepad adapter currently requires Linux".into());
    }
    if !options.managed && !options.input.input_probe && options.games.is_none() {
        return Err("standalone IGUI requires --games".into());
    }
    Ok(Some(options))
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let Some(options) = parse_options()? else {
        return Ok(());
    };
    igui::runtime::run(options)
}
