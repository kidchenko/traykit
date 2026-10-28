# traykit

Shared tooling for the kidchenko tray apps (Cafe, Cantos, Cota, Luz, Porta,
Sonus, SuaVez), so a fix lands once instead of being copied into seven repos.

Two parts: the `cargo tray` tool and the `traykit` runtime crate. A reusable
release workflow follows; see [Roadmap](#roadmap).

## Install

The repo is private, so cargo fetches it with your git credentials:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git https://github.com/kidchenko/traykit cargo-tray
```

Or from a checkout: `cargo install --path cargo-tray`.

## `cargo tray release`

From an app's root:

```sh
cargo tray release patch --dry-run   # show what would change
cargo tray release patch --push      # bump, test, commit, tag, push
cargo tray release 0.1.0             # an explicit version; push yourself
```

It refuses to run off `main`, with uncommitted changes, behind origin, when
the tag already exists, or with nothing under the changelog's Unreleased
heading (`## [Unreleased]` or `## Unreleased`). Then it:

1. runs `cargo test` (`--skip-tests` to skip),
2. sets `[package].version` in Cargo.toml, leaving dependency versions alone,
3. moves the Unreleased notes under `## [X] - <today>`, and updates the
   compare links if the changelog has them,
4. replaces the old version wherever it stands alone in `packaging/`,
   `README.md` and `*.manifest`: `v0.0.1`, `Setup-0.0.1.exe` and `0.0.1.0`
   change, `127.0.0.1` and `0.0.10` do not,
5. refreshes Cargo.lock, commits `chore: release X` and tags `vX`,
6. with `--push`, pushes the branch and the tag together, which starts the
   app's release workflow.

## `traykit` crate

The runtime modules every app used to copy, named after the app instead of
hard-coding it:

```toml
[dependencies]
traykit = { git = "https://github.com/kidchenko/traykit" }
```

```rust
traykit::init(traykit::App {
    name: "Recorte",
    id: "recorte",
    bundle_id: "com.kidchenko.recorte",
    summary: "Snip a part of the screen, mark it up, copy it",
});
traykit::linfo!("started");
let Some(signal) = traykit::single_instance::acquire() else { return };
```

| module            | what it does                                                  |
|-------------------|---------------------------------------------------------------|
| `dirs`            | `%APPDATA%\<Name>`, `~/Library/Application Support/<Name>`, `~/.config/<id>` |
| `log`             | `<id>.log` there, rotated at 1 MB; `linfo!` and friends; `<ID>_LOG=debug` |
| `config`          | `config.json` there for any serde type: BOM-tolerant load, atomic save |
| `single_instance` | one copy at a time; a second launch summons the first         |
| `autostart`       | Run key, LaunchAgent, or `~/.config/autostart` entry          |
| `theme`           | is the taskbar / menu bar dark, and the accent colour         |
| `time`            | `LocalTime::now()`, for log lines and file names              |
| `util`            | `shell_open`, `attach_console`, and `wide` on Windows         |

To change traykit and an app together, point cargo at the checkout instead
of GitHub with a `.cargo/config.toml` above both (it is not in either repo):

```toml
[patch."https://github.com/kidchenko/traykit"]
traykit = { path = "traykit/traykit" }
```

Take it out again before committing the app: while it is there, cargo writes
the path into the app's Cargo.lock. Push traykit, then `cargo update -p
traykit` in the app to move it to the new commit.

## Roadmap

- `cargo tray bundle`: the .app, .dmg and installer that `build.sh` and
  `build.ps1` build in every app today, with signing in one place.
- `cargo tray new <name>`: a new app with packaging, CI and docs in place.
- `notify` in the crate: the toasts Cota and SuaVez each draw.
- Moving Cafe, Cantos, Cota, Luz, Porta, Sonus and SuaVez onto the crate.
- A reusable release workflow the apps call with their name and bundle id.

The crate and the workflow need this repo public for a public app: its CI
cannot fetch a private dependency or call a private repo's workflow.
