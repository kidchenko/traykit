//! The per-user folder an app keeps its config, log and socket in.

use std::path::PathBuf;

/// `%APPDATA%\<Name>` on Windows, `~/Library/Application Support/<Name>` on
/// macOS, `$XDG_CONFIG_HOME/<id>` (`~/.config/<id>`) on Linux: the per-user,
/// per-app spot each platform expects.
pub fn config_dir() -> Option<PathBuf> {
    imp::config_dir()
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(crate::app().name))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library/Application Support")
                .join(crate::app().name)
        })
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::path::PathBuf;

    /// `$XDG_CONFIG_HOME` only when absolute, as the spec says; otherwise
    /// `~/.config`.
    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|d| d.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|d| d.join(crate::app().id))
    }
}
