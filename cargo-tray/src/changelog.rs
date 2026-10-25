//! CHANGELOG.md in the Keep a Changelog layout the apps use: what is under
//! `## [Unreleased]` becomes the new version's section.

/// Both spellings the apps use.
const UNRELEASED: [&str; 2] = ["## [Unreleased]", "## Unreleased"];

/// The changelog with `version` released on `date` (`YYYY-MM-DD`).
///
/// An empty Unreleased section is an error: a release with nothing in it is
/// almost always a release cut before its notes were written.
pub fn release(text: &str, version: &str, date: &str) -> Result<String, String> {
    let heading = UNRELEASED
        .iter()
        .flat_map(|h| text.match_indices(h))
        .filter(|(at, h)| {
            let starts_line = *at == 0 || text.as_bytes()[at - 1] == b'\n';
            let rest = &text[at + h.len()..];
            starts_line && (rest.is_empty() || rest.starts_with(['\n', '\r']))
        })
        .map(|(at, _)| at)
        .min()
        .ok_or("CHANGELOG.md has no \"## [Unreleased]\" heading to release from")?;
    let body_start = text[heading..]
        .find('\n')
        .map_or(text.len(), |n| heading + n + 1);
    let body_end = text[body_start..]
        .match_indices("\n## ")
        .next()
        .map_or(text.len(), |(n, _)| body_start + n + 1);
    let body = &text[body_start..body_end];
    let notes = body
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("### ") && !l.starts_with('['))
        .count();
    if notes == 0 {
        return Err(
            "nothing under \"## [Unreleased]\" in CHANGELOG.md; write the notes first".into(),
        );
    }

    let mut out = String::with_capacity(text.len() + 160);
    out.push_str(&text[..body_start]);
    out.push_str(&format!("\n## [{version}] - {date}\n"));
    out.push_str(&text[body_start..]);
    Ok(links(&out, version))
}

/// Point `[Unreleased]:` at the new tag, and add the new version's own
/// compare link under it. A changelog without links is left as it is.
fn links(text: &str, version: &str) -> String {
    let mut out = Vec::new();
    for line in text.split_inclusive('\n') {
        let parsed = line
            .strip_prefix("[Unreleased]: ")
            .and_then(|url| url.trim_end().strip_suffix("...HEAD"))
            .and_then(|url| url.rsplit_once("compare/"));
        match parsed {
            Some((base, from)) => {
                let eol = if line.ends_with('\n') { "\n" } else { "" };
                out.push(format!("[Unreleased]: {base}compare/v{version}...HEAD\n"));
                out.push(format!(
                    "[{version}]: {base}compare/{from}...v{version}{eol}"
                ));
            }
            None => out.push(line.to_string()),
        }
    }
    out.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "# Changelog\n\n## [Unreleased]\n\n### Fixed\n\n- A thing.\n\n## [0.0.3] - 2026-10-23\n\n### Added\n\n- Things.\n\n[Unreleased]: https://github.com/kidchenko/cantos/compare/v0.0.3...HEAD\n[0.0.3]: https://github.com/kidchenko/cantos/compare/v0.0.2...v0.0.3\n";

    #[test]
    fn moves_the_notes_under_the_new_version_and_links_it() {
        let out = release(LOG, "0.0.4", "2026-10-25").unwrap();
        assert_eq!(
            out,
            "# Changelog\n\n## [Unreleased]\n\n## [0.0.4] - 2026-10-25\n\n### Fixed\n\n- A thing.\n\n## [0.0.3] - 2026-10-23\n\n### Added\n\n- Things.\n\n[Unreleased]: https://github.com/kidchenko/cantos/compare/v0.0.4...HEAD\n[0.0.4]: https://github.com/kidchenko/cantos/compare/v0.0.3...v0.0.4\n[0.0.3]: https://github.com/kidchenko/cantos/compare/v0.0.2...v0.0.3\n"
        );
    }

    #[test]
    fn refuses_an_empty_unreleased_section() {
        let empty =
            "# Changelog\n\n## [Unreleased]\n\n### Added\n\n## [0.0.3] - 2026-10-23\n\n- Things.\n";
        assert!(release(empty, "0.0.4", "2026-10-25").is_err());
        assert!(release("# Changelog\n", "0.0.4", "2026-10-25").is_err());
    }

    #[test]
    fn takes_the_heading_without_brackets_too() {
        let plain = "# Changelog\n\n## Unreleased\n\n- A thing.\n\n## [0.0.1] - 2026-10-23\n";
        assert_eq!(
            release(plain, "0.0.2", "2026-10-25").unwrap(),
            "# Changelog\n\n## Unreleased\n\n## [0.0.2] - 2026-10-25\n\n- A thing.\n\n## [0.0.1] - 2026-10-23\n"
        );
    }

    #[test]
    fn works_without_links_or_older_releases() {
        let first = "# Changelog\n\n## [Unreleased]\n\n- First.\n";
        assert_eq!(
            release(first, "0.0.1", "2026-10-25").unwrap(),
            "# Changelog\n\n## [Unreleased]\n\n## [0.0.1] - 2026-10-25\n\n- First.\n"
        );
    }
}
