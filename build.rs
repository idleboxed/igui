// SPDX-License-Identifier: BSD-3-Clause
use std::process::Command;

fn main() {
    // Keep Cargo's default package-wide change tracking: source edits refresh the stamp,
    // while reusing an unchanged binary preserves its original build time.
    let output = Command::new("date")
        .args(["-u", "+%Y%m%d%H%M"])
        .output()
        .expect("failed to run date for the IGUI UTC build timestamp");
    assert!(output.status.success(), "date failed: {}", output.status);
    let timestamp = String::from_utf8(output.stdout).expect("date output must be UTF-8");
    let timestamp = timestamp.trim();
    assert!(
        timestamp.len() == 12 && timestamp.bytes().all(|byte| byte.is_ascii_digit()),
        "invalid IGUI UTC build timestamp: {timestamp:?}"
    );
    println!("cargo:rustc-env=IGUI_BUILD_UTC={timestamp}");
}
