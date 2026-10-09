// SPDX-License-Identifier: BSD-3-Clause
mod cli;

fn main() {
    if let Err(error) = cli::run() {
        eprintln!("IGUI: {error}");
        std::process::exit(1);
    }
}
