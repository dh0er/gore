# Auto-Updater + GitHub Release Workflow — Design

Date: 2026-06-11
Status: Approved

## Goal

goresave updates itself from GitHub Releases. A tag push produces a Velopack
installer release; the installed app silently downloads new versions and
applies them on user-confirmed restart.

## Decisions

- **Repo goes public.** The updater reads release assets anonymously; no token
  ships in the binary.
- **Velopack** is the update/packaging mechanism (Rust crate in
  `goresave_core`, `vpk` CLI in CI). Chosen over WinSparkle (too much
  infrastructure: appcast hosting, Inno Setup, signing keys) and over a custom
  self-swap updater (more edge cases to own).
- **Tag-driven releases** stay: `git tag vX.Y.Z` + push triggers the workflow.
  The workflow fails if the tag version does not match `pubspec.yaml`.
- **Build number removed.** `pubspec.yaml` version becomes plain SemVer
  (`0.1.0`, no `+1`). No store requires a build code, and Velopack ignores
  SemVer build metadata when comparing versions. The About dialog shows the
  version plus the git short SHA instead, injected via
  `--dart-define=GIT_SHA=<sha>` in CI (empty/`dev` for local builds).
- **Update UX:** check on startup in the background, download silently, show a
  discreet banner "Update X ready — restart". Clicking applies the update and
  restarts. No modal dialog, no forced update. Errors (offline, rate limit,
  parse) are logged and never block the app.
- **Portable zip stays** as a secondary artifact without auto-update. Velopack
  detects a non-installed app; the updater then disables itself (no banner).
  There are no existing portable users, so no migration path is needed.

## Architecture

### CI / Packaging (`.github/workflows/release.yml`)

Existing flow (checkout, Rust, Flutter, `python build.py dist`) is extended:

1. Verify tag `vX.Y.Z` matches `pubspec.yaml` version; fail on mismatch.
2. Pass git short SHA into the Flutter build (`--dart-define=GIT_SHA=...`,
   plumbed through `build.py`).
3. `dotnet tool install -g vpk`, then
   `vpk pack --packId Goresave --packVersion X.Y.Z --packDir <Release folder>
   --mainExe goresave.exe`.
4. Attach to the GitHub release: `*-Setup.exe`, `*-full.nupkg`, delta
   packages, `releases.win.json`, and the portable zip from `build.py dist`.

### Update source — no token, no API

Base URL: `https://github.com/dh0er/goresave/releases/latest/download/`

GitHub redirects this stable URL to the newest release's assets.
`releases.win.json` references packages by file name relative to that base, so
Velopack's `HttpSource` works against it directly — no GitHub API calls, no
auth, no rate-limit concerns.

### Rust (`crates/goresave_core`)

- Add the `velopack` crate.
- New FFI export `goresave_velopack_startup()`: runs
  `VelopackApp::build().run()`. Must be called first thing in Dart `main()`
  because Velopack install/update hooks may exit the process.
- Update operations ride the existing `goresave_execute(json) -> json`
  command channel:
  - `update_check` — returns available version or "none" (also "disabled"
    when running un-installed, e.g. portable zip).
  - `update_download` — downloads silently, reports progress/completion.
  - `update_apply_restart` — applies staged update and restarts the app.

### Flutter (`apps/goresave`)

- On startup: `update_check` in the background; if an update exists,
  `update_download` silently; on completion show the banner.
- Banner click calls `update_apply_restart`.
- About dialog: version string becomes `X.Y.Z (shortsha)` from
  `String.fromEnvironment('GIT_SHA')`; build number display removed.

## Error handling

- Any updater failure is non-blocking: log and continue; app works fully
  offline.
- Workflow hard-fails on tag/pubspec version mismatch before building.

## Testing

- Rust unit tests for update command parsing/serialization (network layer kept
  thin; Velopack calls isolated behind the command handlers).
- Manual end-to-end: release v0.1.1, install via Setup.exe, tag v0.1.2, verify
  banner appears and restart applies the new version.
- Portable zip: verify the updater self-disables (no banner, `update_check`
  returns "disabled").

## Out of scope

- Code signing of the installer (unsigned for now; SmartScreen warning
  accepted).
- Update channels (beta/stable) — single stable channel.
- Linux/macOS builds.
