// SPDX-License-Identifier: BSD-3-Clause
//! Read-only Linux gamepads: declarative HID/evdev profiles, explicit joydev mode.
//! Button numbers are device-specific; automatic X/B/Y/A uses physical positions.
use {
    imanlib::input::Buttons,
    imanlib::igui::Action,
};
#[cfg(target_os = "linux")]
use std::{fs::{self, File, OpenOptions}, io::{self, Read}, os::{fd::AsRawFd, unix::fs::{FileTypeExt, OpenOptionsExt}}, path::{Path, PathBuf}};
#[cfg(target_os = "linux")]
use imanlib::input_metrics::Counters;

/// List joydev names in stable order without opening devices; report discovery errors.
#[cfg(target_os = "linux")]
pub fn discover(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_name().to_str().is_some_and(|name| {
            name.strip_prefix("js").is_some_and(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            })
        }) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

#[derive(Clone, Debug)]
pub struct Mapping {
    /// Joystick button index for X / lower filters; the field name follows the physical label.
    pub x: u8,
    /// Joystick button index for B / favourite; the field name follows the physical label.
    pub b: u8,
    /// Joystick button index for Y / upper filters; the field name follows the physical label.
    pub y: u8,
    /// Joystick button index for A / confirmation; the field name follows the physical label.
    pub a: u8,
    pub select: Option<u8>,
    pub start: Option<u8>,
    pub axis_x: Option<u8>,
    pub axis_y: Option<u8>,
    /// Up, Down, Left, Right digital button indices, if provided by the device.
    pub dpad: [Option<u8>; 4],
}

impl Mapping {
    /// Decode the kernel maps, not vendor button indices or ambiguous BTN_X/Y
    /// aliases. X/square is WEST, B/circle EAST, Y/triangle NORTH, A SOUTH.
    pub fn from_codes(
        axis_map: &[u8],
        button_map: &[u16],
        overrides: Option<(u8, u8, u8, u8)>,
        select_override: Option<u8>,
    ) -> Result<Self, String> {
        if axis_map.len() > 64 || button_map.len() > 256 {
            return Err("invalid joystick map size".into());
        }
        let axis = |code| axis_map.iter().position(|&value| value == code).map(|index| index as u8);
        let button = |code| button_map.iter().position(|&value| value == code).map(|index| index as u8);
        let (button_x, button_b, button_y, button_a) = overrides
            .or_else(|| {
                Some((
                    button(0x134)?,
                    button(0x131)?,
                    button(0x133)?,
                    button(0x130)?,
                ))
            })
            .ok_or(
                "nonstandard X/B/Y/A; set --gamepad-buttons X,B,Y,A using joystick button indices",
            )?;
        if button_x == button_b
            || button_x == button_y
            || button_b == button_y
            || button_a == button_x
            || button_a == button_b
            || button_a == button_y
            || [button_x, button_b, button_y, button_a]
                .iter()
                .any(|&index| usize::from(index) >= button_map.len())
        {
            return Err("invalid X/B/Y/A joystick button indices".into());
        }
        let (axis_x, axis_y) = match (axis(0x10), axis(0x11)) {
            (Some(horizontal_axis), Some(vertical_axis)) => (Some(horizontal_axis), Some(vertical_axis)), // ABS_HAT0X/Y preferred
            _ => (axis(0), axis(1)),                  // ABS_X/Y fallback, normalized by joydev
        };
        let dpad = [button(0x220), button(0x221), button(0x222), button(0x223)];
        let select = select_override.or_else(|| button(0x13a)); // BTN_SELECT, never Start
        let start = button(0x13b); // BTN_START, never a confirmation or exit
        if select.is_some_and(|index| {
            usize::from(index) >= button_map.len()
                || [button_x, button_b, button_y, button_a].contains(&index)
                || dpad.contains(&Some(index))
                || start == Some(index)
        }) {
            return Err("invalid or overlapping Select joystick button index".into());
        }
        if start.is_some_and(|index| [button_x, button_b, button_y, button_a].contains(&index) || dpad.contains(&Some(index))) {
            return Err("overlapping Start joystick button index".into());
        }
        if !(axis_x.is_some() && axis_y.is_some() || dpad.iter().all(Option::is_some)) {
            return Err("joystick has no supported D-pad or X/Y axes".into());
        }
        Ok(Self {
            x: button_x,
            b: button_b,
            y: button_y,
            a: button_a,
            select,
            start,
            axis_x,
            axis_y,
            dpad,
        })
    }
}

pub struct State {
    pub(super) mapping: Mapping,
    buttons: Buttons,
    axes: Buttons,
    pressed: Buttons,
    waiting_for_neutral: bool,
}

impl State {
    /// Create released joystick state for the selected button and axis mapping.
    pub fn new(mapping: Mapping) -> Self {
        Self {
            mapping,
            buttons: Buttons::default(),
            axes: Buttons::default(),
            pressed: Buttons::default(),
            waiting_for_neutral: true,
        }
    }

    /// Native-endian 8-byte Linux js_event; INIT is state, never an activation.
    pub fn event(&mut self, bytes: [u8; 8]) {
        let before = self.buttons.union(self.axes);
        let value = i16::from_ne_bytes([bytes[4], bytes[5]]);
        let kind = bytes[6];
        let number = bytes[7];
        if kind & 0x80 != 0 {
            self.waiting_for_neutral = true;
        }
        match kind & !0x80 {
            1 => {
                if number == self.mapping.x {
                    self.buttons.set(Action::LowerFilters, value != 0);
                }
                if number == self.mapping.b {
                    self.buttons.set(Action::Favourite, value != 0);
                }
                if number == self.mapping.y {
                    self.buttons.set(Action::Filters, value != 0);
                }
                if number == self.mapping.a {
                    self.buttons.set(Action::Activate, value != 0);
                }
                if Some(number) == self.mapping.select {
                    self.buttons.set(Action::Trash, value != 0);
                }
                if Some(number) == self.mapping.start {
                    self.buttons.set(Action::Start, value != 0);
                }
                for (index, action) in [Action::Up, Action::Down, Action::Left, Action::Right]
                    .into_iter()
                    .enumerate()
                {
                    if self.mapping.dpad[index] == Some(number) {
                        self.buttons.set(action, value != 0);
                    }
                }
            }
            2 => {
                if self.mapping.axis_x == Some(number) {
                    self.axes.set(Action::Left, value < -16000);
                    self.axes.set(Action::Right, value > 16000);
                }
                if self.mapping.axis_y == Some(number) {
                    self.axes.set(Action::Up, value < -16000);
                    self.axes.set(Action::Down, value > 16000);
                }
            }
            _ => {}
        }
        let after = self.buttons.union(self.axes);
        for action in imanlib::input::ACTIONS {
            if after.down(action) && !before.down(action) {
                self.pressed.set(action, true);
            }
        }
    }

    /// Return mapped buttons, suppressing input until a newly connected joystick is neutral.
    pub fn sample(&mut self) -> Buttons {
        let buttons = self.buttons.union(self.axes);
        let sample = buttons.union(self.pressed);
        self.pressed = Buttons::default();
        if self.waiting_for_neutral {
            if !buttons.any() {
                self.waiting_for_neutral = false;
            }
            Buttons::default()
        } else {
            sample
        }
    }
}

// _IOR('j', nr, T), Linux asm-generic ABI used by the x86/ARM targets.
#[cfg(target_os = "linux")]
/// Read one correctly sized joystick ioctl result into the supplied buffer.
pub(crate) fn query<T>(file: &File, nr: u8, value: &mut T) -> io::Result<()> {
    let request = 0x80000000u32
        | ((std::mem::size_of::<T>() as u32) << 16)
        | ((b'j' as u32) << 8)
        | nr as u32;
    // SAFETY: request size is exactly T; every caller uses initialized integer arrays.
    // musl declares the request as int, glibc as unsigned long. Preserve the
    // ioctl bit pattern, not a checked numeric conversion of the sign bit.
    if unsafe { libc::ioctl(file.as_raw_fd(), request as _, value as *mut T) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub(super) struct Device {
    file: File,
    pub(super) state: State,
}

#[cfg(target_os = "linux")]
pub(crate) struct RawJoystick {
    pub file: File,
    pub axes: Vec<u8>,
    pub buttons: Vec<u16>,
}

#[cfg(target_os = "linux")]
/// Open a read-only joystick device and read its bounded button and axis maps.
pub(crate) fn open_raw(path: &Path) -> io::Result<RawJoystick> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    if !file.metadata()?.file_type().is_char_device() {
        return Err(io::Error::other("not a joystick character device"));
    }
    let mut axes = 0u8;
    let mut buttons = 0u8;
    let mut axis_map = [0u8; 64];
    let mut button_map = [0u16; 512];
    query(&file, 0x11, &mut axes)?;
    query(&file, 0x12, &mut buttons)?;
    query(&file, 0x32, &mut axis_map)?;
    query(&file, 0x34, &mut button_map)?;
    if usize::from(axes) > axis_map.len() {
        return Err(io::Error::other("invalid joystick axis count"));
    }
    Ok(RawJoystick {
        file,
        axes: axis_map[..usize::from(axes)].to_vec(),
        buttons: button_map[..usize::from(buttons)].to_vec(),
    })
}

#[cfg(target_os = "linux")]
impl Device {
    /// Open a joystick with checked automatic or explicitly supplied control mapping.
    pub(super) fn open(
        path: &Path,
        overrides: Option<(u8, u8, u8, u8)>,
        select_override: Option<u8>,
    ) -> io::Result<Self> {
        let raw = open_raw(path)?;
        let mapping = Mapping::from_codes(&raw.axes, &raw.buttons, overrides, select_override)
            .map_err(io::Error::other)?;
        Ok(Self {
            file: raw.file,
            state: State::new(mapping),
        })
    }

    /// Drain bounded joystick events and return the current logical buttons.
    pub(super) fn poll(&mut self, mut diagnostics: Option<&mut Counters>) -> io::Result<Buttons> {
        // Bound work even with a noisy device; never block the UI thread.
        for _ in 0..1024 {
            let mut event = [0u8; 8];
            match self.file.read(&mut event) {
                Ok(8) => {
                    if let Some(counters) = diagnostics.as_deref_mut() {
                        let kind = match event[6] & !0x80 {
                            1 => 1,
                            2 => 3,
                            _ => 255,
                        };
                        counters.event(
                            kind,
                            u16::from(event[7]),
                            i32::from(i16::from_ne_bytes([event[4], event[5]])),
                        );
                    }
                    self.state.event(event);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(self.state.sample())
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
                _ => {
                    return Err(io::Error::other(
                        "joystick disconnected or incomplete event",
                    ))
                }
            }
        }
        if let Some(counters) = diagnostics {
            counters.poll_limits += 1;
        }
        // Do not publish a partially drained INIT/overflow state.
        self.state.waiting_for_neutral = true;
        Ok(Buttons::default())
    }
}
