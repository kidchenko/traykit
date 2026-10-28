//! `config.json` in the config folder, for any `serde` type the app defines.
//!
//! The app owns its settings struct (and its own sanitising); this owns
//! where the file lives, the BOM Notepad adds, and writing it safely.

use crate::{lerror, linfo};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;

pub fn path() -> Option<PathBuf> {
    crate::dirs::config_dir().map(|d| d.join("config.json"))
}

/// Last-modified time of the file, or `None` if it is not there. For
/// noticing a hand edit without parsing the file on every tick.
pub fn modified() -> Option<std::time::SystemTime> {
    std::fs::metadata(path()?).ok()?.modified().ok()
}

pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    // serde_json treats a UTF-8 BOM as a syntax error, and anyone who edits
    // the file in Notepad or writes it from PowerShell will have one.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    serde_json::from_str(text)
}

/// Read and parse, without logging or falling back to defaults.
///
/// `None` covers both "not there" and "could not be parsed". A caller
/// reacting to a change on disk wants to leave the running config alone in
/// either case: an editor caught mid-save must not reset anyone's settings.
pub fn reload<T: DeserializeOwned>() -> Option<T> {
    parse(&std::fs::read_to_string(path()?).ok()?).ok()
}

/// Never fails: a missing, unreadable, or corrupt file yields the default.
pub fn load<T: DeserializeOwned + Default>() -> T {
    let Some(path) = path() else {
        return T::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        linfo!("no config at {}; using defaults", path.display());
        return T::default();
    };
    match parse(&text) {
        Ok(c) => {
            linfo!("config loaded from {}", path.display());
            c
        }
        Err(e) => {
            lerror!(
                "config at {} is unreadable ({e}); falling back to defaults",
                path.display()
            );
            T::default()
        }
    }
}

/// Write via a temp file + rename so an interrupted save can never leave a
/// half-written config behind.
pub fn save<T: Serialize>(value: &T) -> std::io::Result<()> {
    let Some(path) = path() else {
        return Err(std::io::Error::other("no config directory in environment"));
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(value)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)
}

/// Open the file in the user's editor, writing the defaults first if there
/// is none yet, so there is something to edit.
pub fn reveal<T: Serialize + DeserializeOwned + Default>() {
    let Some(p) = path() else { return };
    if !p.exists() {
        let _ = save(&load::<T>());
    }
    crate::util::shell_open(&p.to_string_lossy());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct Sample {
        on: bool,
    }

    #[test]
    fn a_bom_is_not_a_syntax_error() {
        let s: Sample = parse("\u{feff}{\"on\": true}").unwrap();
        assert_eq!(s, Sample { on: true });
    }
}
