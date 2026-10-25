//! Versions, and finding and replacing them in an app's files.

use std::fmt;

/// `MAJOR.MINOR.PATCH`, with an optional `-pre` suffix kept as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let (core, pre) = match s.split_once('-') {
            Some((c, p)) if !p.is_empty() => (c, Some(p.to_string())),
            Some(_) => return None,
            None => (s, None),
        };
        let mut parts = core.split('.').map(|p| {
            // "01" is not a version number anyone means.
            if p.is_empty() || (p.len() > 1 && p.starts_with('0')) {
                None
            } else {
                p.parse::<u64>().ok()
            }
        });
        let v = Version {
            major: parts.next()??,
            minor: parts.next()??,
            patch: parts.next()??,
            pre,
        };
        parts.next().is_none().then_some(v)
    }

    /// What `patch`, `minor`, `major` or an explicit version makes of `self`.
    pub fn next(&self, how: &str) -> Result<Version, String> {
        let next = match how {
            "patch" => Version {
                patch: self.patch + 1,
                pre: None,
                ..self.clone()
            },
            "minor" => Version {
                minor: self.minor + 1,
                patch: 0,
                pre: None,
                ..self.clone()
            },
            "major" => Version {
                major: self.major + 1,
                minor: 0,
                patch: 0,
                pre: None,
            },
            explicit => Version::parse(explicit).ok_or_else(|| {
                format!("{explicit:?} is not patch, minor, major or a version like 1.2.3")
            })?,
        };
        if !next.is_after(self) {
            return Err(format!("{next} is not after the current version, {self}"));
        }
        Ok(next)
    }

    /// Precedence as semver has it, simplified: a pre-release sorts before
    /// its release, and two pre-releases compare as text.
    fn is_after(&self, other: &Version) -> bool {
        let a = (self.major, self.minor, self.patch);
        let b = (other.major, other.minor, other.patch);
        if a != b {
            return a > b;
        }
        match (&self.pre, &other.pre) {
            (None, Some(_)) => true,
            (Some(x), Some(y)) => x > y,
            _ => false,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

/// A `key = "value"` from the `[package]` table of a Cargo.toml.
pub fn package_field(cargo_toml: &str, key: &str) -> Option<String> {
    package_lines(cargo_toml)
        .find_map(|line| quoted_value(line, key))
        .map(|(_, value)| value.to_string())
}

/// The Cargo.toml with `[package].version` set to `version`, every other
/// byte as it was. A dependency's `version = ...` is never touched.
pub fn set_package_version(cargo_toml: &str, version: &str) -> Option<String> {
    let line = package_lines(cargo_toml).find(|l| quoted_value(l, "version").is_some())?;
    let (range, _) = quoted_value(line, "version")?;
    // `line` borrows from `cargo_toml`, so its offset is where it sits.
    let start = line.as_ptr() as usize - cargo_toml.as_ptr() as usize;
    let mut out = String::with_capacity(cargo_toml.len());
    out.push_str(&cargo_toml[..start + range.start]);
    out.push_str(version);
    out.push_str(&cargo_toml[start + range.end..]);
    Some(out)
}

fn package_lines(cargo_toml: &str) -> impl Iterator<Item = &str> {
    let mut in_package = false;
    cargo_toml.lines().filter(move |line| {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            return false;
        }
        in_package
    })
}

/// For `key = "value"`, the byte range of `value` within `line`, and value.
fn quoted_value<'a>(line: &'a str, key: &str) -> Option<(std::ops::Range<usize>, &'a str)> {
    let rest = line.trim_start().strip_prefix(key)?;
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let inner = rest.strip_prefix('"')?;
    let len = inner.find('"')?;
    let start = line.len() - inner.len();
    Some((start..start + len, &inner[..len]))
}

/// `text` with every standalone `old` replaced by `new`, and how many.
///
/// Standalone means not part of a longer number: `0.0.1` is replaced in
/// `v0.0.1`, `Setup-0.0.1.exe` and `0.0.1.0`, but not in `127.0.0.1` or
/// `0.0.10`.
pub fn replace_standalone(text: &str, old: &str, new: &str) -> (String, usize) {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut last = 0;
    for (at, _) in text.match_indices(old) {
        if at < last {
            continue;
        }
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + old.len()).copied();
        let joined_before = before.is_some_and(|b| b.is_ascii_digit() || b == b'.');
        let joined_after = after.is_some_and(|b| b.is_ascii_digit());
        if joined_before || joined_after {
            continue;
        }
        out.push_str(&text[last..at]);
        out.push_str(new);
        last = at + old.len();
        count += 1;
    }
    out.push_str(&text[last..]);
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn parses_plain_prefixed_and_pre_release_versions() {
        assert_eq!(v("1.2.3").to_string(), "1.2.3");
        assert_eq!(v("v0.0.4").to_string(), "0.0.4");
        assert_eq!(v("1.0.0-rc.1").to_string(), "1.0.0-rc.1");
        for bad in ["1.2", "1.2.3.4", "1.02.3", "a.b.c", "1.2.3-", ""] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn bumps_each_part_and_resets_the_ones_below() {
        assert_eq!(v("0.0.4").next("patch").unwrap(), v("0.0.5"));
        assert_eq!(v("0.3.4").next("minor").unwrap(), v("0.4.0"));
        assert_eq!(v("0.3.4").next("major").unwrap(), v("1.0.0"));
        assert_eq!(v("1.0.0-rc.1").next("patch").unwrap(), v("1.0.1"));
        assert_eq!(v("0.0.4").next("0.1.0").unwrap(), v("0.1.0"));
    }

    #[test]
    fn refuses_a_version_that_is_not_newer() {
        assert!(v("0.0.4").next("0.0.4").is_err());
        assert!(v("0.0.4").next("0.0.3").is_err());
        assert!(v("1.0.0").next("1.0.0-rc.1").is_err());
        assert!(v("1.0.0-rc.1").next("1.0.0").is_ok());
        assert!(v("0.0.4").next("soon").is_err());
    }

    const MANIFEST: &str = r#"[package]
name         = "cantos"
version      = "0.0.4"
edition      = "2021"

[dependencies]
serde = { version = "1" }

[dependencies.windows]
version  = "0.0.4"
"#;

    #[test]
    fn reads_the_package_table_only() {
        assert_eq!(package_field(MANIFEST, "name").as_deref(), Some("cantos"));
        assert_eq!(package_field(MANIFEST, "version").as_deref(), Some("0.0.4"));
        assert_eq!(
            package_field("[dependencies]\nversion = \"1\"\n", "version"),
            None
        );
    }

    #[test]
    fn sets_the_package_version_and_nothing_else() {
        let out = set_package_version(MANIFEST, "0.0.5").unwrap();
        assert_eq!(
            out,
            MANIFEST.replacen("version      = \"0.0.4\"", "version      = \"0.0.5\"", 1)
        );
        assert!(out.contains("version  = \"0.0.4\""));
    }

    #[test]
    fn replaces_only_standalone_versions() {
        let text = "v0.0.1 Setup-0.0.1.exe 0.0.1.0 127.0.0.1 0.0.10 10.0.0.1 '0.0.1'";
        let (out, n) = replace_standalone(text, "0.0.1", "0.0.2");
        assert_eq!(
            out,
            "v0.0.2 Setup-0.0.2.exe 0.0.2.0 127.0.0.1 0.0.10 10.0.0.1 '0.0.2'"
        );
        assert_eq!(n, 4);
    }
}
