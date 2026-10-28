//! Minimal file logging to `<id>.log` in the config folder.
//!
//! Under `windows_subsystem = "windows"` there is no console, so `eprintln!`
//! goes nowhere. Without a log, "it does nothing" is unanswerable.
//!
//! Hand-rolled rather than pulling in `log` + a backend: this needs about
//! sixty lines and the binary size is a stated constraint.
//!
//! Set `<ID>_LOG=debug` (`SONUS_LOG=debug`) for the [`ldebug!`] lines.
//!
//! ```ignore
//! use traykit::{linfo, lwarn};
//! linfo!("started, pid {}", std::process::id());
//! ```

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static SINK: Mutex<Option<File>> = Mutex::new(None);
static DEBUG: AtomicBool = AtomicBool::new(false);

/// Rotate at 1 MB. A tray app can run for months, and an unbounded log on
/// someone else's disk is not our call to make.
const MAX_BYTES: u64 = 1_000_000;

pub fn path() -> Option<PathBuf> {
    crate::dirs::config_dir().map(|d| d.join(format!("{}.log", crate::app().id)))
}

pub fn debug_enabled() -> bool {
    DEBUG.load(Ordering::Relaxed)
}

/// Called by [`crate::init`].
pub(crate) fn init() {
    let var = format!("{}_LOG", crate::app().id.to_uppercase());
    DEBUG.store(
        std::env::var(var)
            .map(|v| v.eq_ignore_ascii_case("debug"))
            .unwrap_or(false),
        Ordering::Relaxed,
    );

    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Keep one previous generation, so a crash loop cannot erase the run that
    // actually explains it.
    if std::fs::metadata(&p)
        .map(|m| m.len() > MAX_BYTES)
        .unwrap_or(false)
    {
        let _ = std::fs::rename(&p, p.with_extension("log.1"));
    }
    if let Ok(f) = OpenOptions::new().create(true).append(true).open(&p) {
        if let Ok(mut sink) = SINK.lock() {
            *sink = Some(f);
        }
    }
}

fn stamp() -> String {
    let t = crate::time::LocalTime::now();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.year, t.month, t.day, t.hour, t.minute, t.second, t.millis
    )
}

pub fn write(level: &str, msg: &str) {
    // Formatted up front so the line reaches the file in a single write;
    // several threads log, and interleaved fragments are worst exactly when
    // the log matters.
    let line = format!("{} {:<5} {}\n", stamp(), level, msg);
    let Ok(mut sink) = SINK.lock() else { return };
    if let Some(f) = sink.as_mut() {
        // A failed write is ignored on purpose: logging must never be the
        // reason the app misbehaves.
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    }
}

#[macro_export]
macro_rules! linfo  { ($($a:tt)*) => { $crate::log::write("INFO",  &format!($($a)*)) } }
#[macro_export]
macro_rules! lwarn  { ($($a:tt)*) => { $crate::log::write("WARN",  &format!($($a)*)) } }
#[macro_export]
macro_rules! lerror { ($($a:tt)*) => { $crate::log::write("ERROR", &format!($($a)*)) } }
/// Only emitted when `<ID>_LOG=debug`; the arguments are not even evaluated
/// otherwise.
#[macro_export]
macro_rules! ldebug {
    ($($a:tt)*) => {
        if $crate::log::debug_enabled() {
            $crate::log::write("DEBUG", &format!($($a)*))
        }
    };
}

/// Open the log in whatever handles .log (Notepad by default). For the tray
/// menu, the only surface these apps have.
pub fn reveal() {
    if let Some(p) = path() {
        crate::util::shell_open(&p.to_string_lossy());
    }
}
