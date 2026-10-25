//! `cargo tray`: the chores every kidchenko tray app shares, in one place
//! instead of copied into each app.
//!
//! Installed as `cargo-tray`, cargo runs it for `cargo tray ...` and passes
//! `tray` as the first argument, which is dropped here.

mod changelog;
mod date;
mod release;
mod version;

use std::process::ExitCode;

const USAGE: &str = "\
cargo tray: the chores every tray app shares

usage: cargo tray <command>

commands:
  release <patch|minor|major|X.Y.Z>   bump, commit and tag a release

cargo tray <command> --help for more.
";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("tray") {
        args.remove(0);
    }
    let result = match args.first().map(String::as_str) {
        Some("release") => release::main(&args[1..]),
        Some("-V" | "--version") => {
            println!("cargo-tray {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None | Some("-h" | "--help" | "help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command {other:?}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
