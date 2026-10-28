//! The runtime pieces every kidchenko tray app shares, so a fix lands once
//! instead of being copied into each app.
//!
//! | module            | responsibility                                        |
//! |-------------------|-------------------------------------------------------|
//! | `dirs`            | the per-user folder the app keeps its files in        |
//! | `log`             | `<id>.log` in that folder, rotated at 1 MB            |
//! | `config`          | `config.json` in that folder, loaded and saved        |
//! | `single_instance` | one copy at a time; relaunching summons the first     |
//! | `autostart`       | run at login                                          |
//! | `theme`           | is the taskbar / menu bar dark, and the accent colour |
//! | `time`            | the local wall-clock time                             |
//! | `util`            | open a path in the shell, borrow the parent console   |
//!
//! Call [`init`] first thing in `main`: every module reads the app's name
//! from it.

use std::sync::OnceLock;

pub mod autostart;
pub mod config;
pub mod dirs;
pub mod log;
pub mod single_instance;
pub mod theme;
pub mod time;
pub mod util;

/// Who the app is. Every name the modules write (the config folder, the log,
/// the Run value, the LaunchAgent, the mutex) is derived from this.
#[derive(Debug, Clone, Copy)]
pub struct App {
    /// Shown to people: `Sonus`. The Windows and macOS folder name.
    pub name: &'static str,
    /// Lowercase, no spaces: `sonus`. The Linux folder, the log file, the
    /// `<ID>_LOG` variable and the `.desktop` file.
    pub id: &'static str,
    /// Reverse DNS: `com.kidchenko.sonus`. The LaunchAgent's label.
    pub bundle_id: &'static str,
    /// One line, for the Linux autostart entry's Comment.
    pub summary: &'static str,
}

static APP: OnceLock<App> = OnceLock::new();

/// Record who the app is and open the log. Call it once, before anything
/// else in this crate; later calls are ignored.
pub fn init(app: App) {
    if APP.set(app).is_ok() {
        log::init();
    }
}

/// The app given to [`init`].
///
/// # Panics
///
/// If [`init`] was never called. That is a bug in the app, found the first
/// time it runs, so failing loudly beats writing files under a wrong name.
pub fn app() -> &'static App {
    APP.get().expect("traykit::init must be called first")
}
