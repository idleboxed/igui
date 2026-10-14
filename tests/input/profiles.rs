// SPDX-License-Identifier: BSD-3-Clause
//! Validate application resources separately from the library's sample profiles.
use imanlib::controls;
use std::path::Path;

#[test]
fn shipped_profiles_remain_valid_and_unambiguous() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("controls");

    let profiles = controls::load(&directory, None, None).unwrap();

    assert!(profiles.conflicts.is_empty());
    assert_eq!(profiles.hid.len(), 1);
    assert_eq!(profiles.evdev.len(), 5);
    assert!(profiles.hid[0].matches(3, 0x2563, 0x0555, "ShanWan Twin USB Joystick"));
    for (vendor, product, name) in [
        (0x20bc, 0x5500, "ShanWan Twin USB Joystick"),
        (0x2563, 0x0555, "ShanWan Twin USB Joystick"),
        (0x0810, 0x0001, "HID 0810:0001"),
        (0x045e, 0x028e, "Microsoft X-Box 360 pad"),
        (0x0079, 0x181c, "ShanWan Wireless Gamepad"),
    ] {
        assert_eq!(profiles.evdev.iter().filter(|profile| profile.matches(3, vendor, product, name)).count(), 1);
    }
}
