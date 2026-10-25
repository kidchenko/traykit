//! `cargo tray release`: bump the version everywhere an app keeps it, move
//! the changelog's notes under it, commit, tag and, if asked, push. The tag
//! is what starts the app's release workflow.

use crate::changelog;
use crate::version::{self, Version};
use std::path::Path;
use std::process::Command;

const USAGE: &str = "\
usage: cargo tray release <patch|minor|major|X.Y.Z> [--push] [--dry-run] [--skip-tests]

Run from the app's root. Bumps [package].version, Cargo.lock, CHANGELOG.md
and the version wherever it stands alone in packaging/, README.md and the
Windows manifest; runs the tests; commits \"chore: release X\" and tags vX.

  --push        push the branch and the tag, which starts the release workflow
  --dry-run     show what would change, change nothing
  --skip-tests  do not run cargo test first
";

struct Options {
    how: String,
    push: bool,
    dry_run: bool,
    skip_tests: bool,
}

/// One file's new contents, and what to say about it.
struct Edit {
    path: String,
    before: String,
    after: String,
    what: String,
}

pub fn main(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;

    let manifest = read("Cargo.toml").map_err(|_| {
        "no Cargo.toml here; run cargo tray release from the app's root".to_string()
    })?;
    let name =
        version::package_field(&manifest, "name").ok_or("Cargo.toml has no [package].name")?;
    let current = version::package_field(&manifest, "version")
        .and_then(|v| Version::parse(&v))
        .ok_or("Cargo.toml has no [package].version that reads as X.Y.Z")?;
    let next = current.next(&opts.how)?;
    let (old, new) = (current.to_string(), next.to_string());
    let tag = format!("v{new}");

    let mut problems = preflight(&tag);
    let edits = plan(&manifest, &old, &new, &mut problems)?;
    if !problems.is_empty() && !opts.dry_run {
        return Err(problems.join("\n"));
    }
    let lock = Path::new("Cargo.lock").exists();

    println!("{name} {old} -> {new}\n");
    for e in &edits {
        println!("  {:<52} {}", e.path, e.what);
        for line in changed_lines(&e.before, &e.after) {
            println!("      {line}");
        }
    }
    if lock {
        println!("  {:<52} cargo update --workspace", "Cargo.lock");
    }
    let push = if opts.push {
        ", push and start the release".to_string()
    } else {
        String::new()
    };
    println!("\nthen: commit \"chore: release {new}\", tag {tag}{push}");

    if opts.dry_run {
        for p in &problems {
            println!("\nwould stop: {p}");
        }
        return Ok(());
    }

    if !opts.skip_tests {
        println!("\nrunning the tests");
        run("cargo", &["test", "--quiet"])?;
    }

    for e in &edits {
        std::fs::write(&e.path, &e.after).map_err(|err| format!("{}: {err}", e.path))?;
    }
    if lock && run_quiet("cargo", &["update", "--workspace", "--offline"]).is_err() {
        run_quiet("cargo", &["update", "--workspace"])?;
    }

    let mut paths: Vec<&str> = edits.iter().map(|e| e.path.as_str()).collect();
    if lock {
        paths.push("Cargo.lock");
    }
    let mut add = vec!["add", "--"];
    add.extend(&paths);
    git(&add)?;
    git(&["commit", "--quiet", "-m", &format!("chore: release {new}")])?;
    git(&[
        "tag",
        "-a",
        &tag,
        "-m",
        &format!("{} {new}", display_name(&name)),
    ])?;
    println!("\ncommitted and tagged {tag}");

    if opts.push {
        let branch = git(&["symbolic-ref", "--short", "HEAD"])?;
        git(&["push", "--quiet", "--atomic", "origin", &branch, &tag])?;
        println!("pushed {branch} and {tag}");
        if let Some(repo) = version::package_field(&manifest, "repository") {
            println!("release workflow: {}/actions", repo.trim_end_matches('/'));
        }
    } else {
        println!("push with: git push --atomic origin HEAD {tag}");
    }
    Ok(())
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut how = None;
    let (mut push, mut dry_run, mut skip_tests) = (false, false, false);
    for a in args {
        match a.as_str() {
            "--push" => push = true,
            "--dry-run" | "-n" => dry_run = true,
            "--skip-tests" => skip_tests = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}\n\n{USAGE}"))
            }
            v if how.is_none() => how = Some(v.to_string()),
            extra => return Err(format!("unexpected {extra:?}\n\n{USAGE}")),
        }
    }
    let how = how.ok_or_else(|| USAGE.to_string())?;
    Ok(Options {
        how,
        push,
        dry_run,
        skip_tests,
    })
}

/// Everything that should stop a release, each as one line.
fn preflight(tag: &str) -> Vec<String> {
    let mut problems = Vec::new();
    match git(&["symbolic-ref", "--short", "HEAD"]) {
        Ok(b) if b == "main" || b == "master" => {}
        Ok(b) => problems.push(format!("on branch {b}; releases are cut from main")),
        Err(_) => problems.push("not on a branch (detached HEAD?)".into()),
    }
    match git(&["status", "--porcelain"]) {
        Ok(s) if s.is_empty() => {}
        Ok(_) => problems.push("uncommitted changes; commit or stash them first".into()),
        Err(e) => problems.push(e),
    }
    if git(&["tag", "--list", tag]).is_ok_and(|t| !t.is_empty()) {
        problems.push(format!("tag {tag} already exists"));
    }
    // Behind origin would tag a commit that is not the latest main, and the
    // push would be refused anyway. A failed fetch (offline) is not fatal.
    if git(&["fetch", "--quiet"]).is_ok() {
        if let Ok(n) = git(&["rev-list", "--count", "HEAD..@{upstream}"]) {
            if n != "0" {
                problems.push(format!("{n} commit(s) behind origin; pull first"));
            }
        }
    }
    problems
}

/// The edits for every file that holds the version. A changelog that cannot
/// be released from is a problem, not an error, so a dry run still shows
/// the rest.
fn plan(
    manifest: &str,
    old: &str,
    new: &str,
    problems: &mut Vec<String>,
) -> Result<Vec<Edit>, String> {
    let mut edits = vec![Edit {
        path: "Cargo.toml".into(),
        before: manifest.into(),
        after: version::set_package_version(manifest, new).ok_or("could not set the version")?,
        what: "[package].version".into(),
    }];

    if let Ok(log) = read("CHANGELOG.md") {
        match changelog::release(&log, new, &today()) {
            Ok(after) => edits.push(Edit {
                path: "CHANGELOG.md".into(),
                before: log,
                after,
                what: format!("Unreleased -> [{new}]"),
            }),
            Err(e) => problems.push(e),
        }
    }

    let tracked = git(&["ls-files"])?;
    for path in tracked.lines().filter(|p| holds_version(p)) {
        // Images and other binaries do not read as text; none hold it.
        let Ok(before) = read(path) else { continue };
        let (after, n) = version::replace_standalone(&before, old, new);
        if n > 0 {
            edits.push(Edit {
                path: path.into(),
                before,
                after,
                what: format!("{n} replaced"),
            });
        }
    }
    Ok(edits)
}

/// The files besides Cargo.toml and the changelog where an app writes its
/// own version: installers, store manifests, the README's build notes, the
/// Windows assembly manifest. Never source: a version-like number there is
/// an IP address or a port as often as not.
fn holds_version(path: &str) -> bool {
    path.starts_with("packaging/") || path == "README.md" || path.ends_with(".manifest")
}

/// `NNN | line` for each line that differs, by position. Replacing a
/// version never adds lines, so position is enough; the changelog, which
/// does, is summed up instead.
fn changed_lines(before: &str, after: &str) -> Vec<String> {
    let (b, a): (Vec<_>, Vec<_>) = (before.lines().collect(), after.lines().collect());
    if a.len() != b.len() {
        return vec![format!("+{} lines", a.len() - b.len())];
    }
    a.iter()
        .zip(&b)
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, (x, _))| format!("{:>4} | {}", i + 1, x.trim()))
        .collect()
}

fn display_name(name: &str) -> String {
    let mut c = name.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Today in local time, as the changelog dates its releases.
fn today() -> String {
    if let Ok(d) = run_quiet("date", &["+%F"]) {
        return d;
    }
    crate::date::utc_today()
}

fn read(path: &str) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

fn git(args: &[&str]) -> Result<String, String> {
    run_quiet("git", args)
}

/// Run and capture stdout, trimmed; stderr goes into the error.
fn run_quiet(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "{program} {} failed: {}",
            args.join(" "),
            err.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Run with the terminal attached, for output worth watching.
fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} {} failed; nothing was changed",
            args.join(" ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_release_files_are_searched() {
        for yes in [
            "packaging/inno/cantos.iss",
            "packaging/winget/kidchenko.Cantos.yaml",
            "README.md",
            "assets/app.manifest",
        ] {
            assert!(holds_version(yes), "{yes}");
        }
        for no in [
            "src/ports.rs",
            "docs/index.html",
            "Cargo.toml",
            "CHANGELOG.md",
            "docs/README.md",
        ] {
            assert!(!holds_version(no), "{no}");
        }
    }

    #[test]
    fn shows_changed_lines_by_number() {
        let lines = changed_lines("a\nv1\nb\n", "a\nv2\nb\n");
        assert_eq!(lines, ["   2 | v2"]);
    }

    #[test]
    fn capitalises_the_name_for_the_tag_message() {
        assert_eq!(display_name("cantos"), "Cantos");
        assert_eq!(display_name(""), "");
    }
}
