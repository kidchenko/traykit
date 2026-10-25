# traykit

Shared tooling for the kidchenko tray apps (Cafe, Cantos, Cota, Luz, Porta,
Sonus, SuaVez), so a fix lands once instead of being copied into seven repos.

Today it is one tool, `cargo tray`. A shared runtime crate and a reusable
release workflow follow; see [Roadmap](#roadmap).

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

## Roadmap

- `cargo tray bundle`: the .app, .dmg and installer that `build.sh` and
  `build.ps1` build in every app today, with signing in one place.
- `cargo tray new <name>`: a new app with packaging, CI and docs in place.
- `traykit` crate: the runtime modules every app copies (`log`,
  `single_instance`, `autostart`, `notify`, `theme`, config load and save).
- A reusable release workflow the apps call with their name and bundle id.

The last two wait for this repo to be public: a public app's CI cannot fetch
a private dependency or call a private repo's workflow.
