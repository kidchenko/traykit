//! Small helpers the apps and the other modules share.
//!
//! The platform-specific ones are in `imp` and re-exported, so callers say
//! `util::shell_open` regardless of which one they get.

/// Full path to the running executable, quoted for use in a command line.
#[cfg(windows)]
pub fn exe_path_quoted() -> Option<String> {
    let p = std::env::current_exe().ok()?;
    Some(format!("\"{}\"", p.display()))
}

/// Borrow the console of the shell that started us, so a CLI subcommand
/// prints somewhere. A `windows_subsystem = "windows"` binary has none of
/// its own.
#[cfg(windows)]
pub fn attach_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
pub fn attach_console() {}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use windows::core::PCWSTR;

    /// Null-terminated UTF-16, for the `W` half of the Win32 API. Keep the
    /// buffer alive for as long as the `PCWSTR` made from it is in use.
    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn pcwstr(buf: &[u16]) -> PCWSTR {
        PCWSTR(buf.as_ptr())
    }

    /// Hand a path (or URL) to the shell.
    pub fn shell_open(target: &str) {
        let verb = wide("open");
        let file = wide(target);
        unsafe {
            let _ = windows::Win32::UI::Shell::ShellExecuteW(
                None,
                pcwstr(&verb),
                pcwstr(&file),
                PCWSTR::null(),
                None,
                windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            );
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    /// Hand a path (or URL) to `open(1)`, which routes it to whatever the
    /// user has associated with it.
    pub fn shell_open(target: &str) {
        let _ = std::process::Command::new("/usr/bin/open")
            .arg(target)
            .spawn();
    }
}

#[cfg(target_os = "linux")]
mod imp {
    /// Hand a path (or URL) to `xdg-open`.
    pub fn shell_open(target: &str) {
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}
