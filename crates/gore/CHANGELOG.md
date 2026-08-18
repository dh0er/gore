# Changelog

All notable changes to gore-cli are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-08-18

First release. Command-line toolkit for modding Gothic 1 Remake.

- `mod` — build and transactionally deploy one bundle containing item
  overrides, localization, audio, voice, textures/assets, loose or packed
  files, AngelScript, and dialog topics.
- `mgr` — keep a verified library and ordered loadout; import supported GORE and
  foreign mods; enable, disable, reorder, remove, analyze, Apply, inspect status,
  recover an abandoned Manager operation, and Reset through the shared Manager
  engine. Import accepts zip/folder packages, loose `_P.pak`, IoStore pairs with
  an optional same-stem `.pak`, UE4SS Lua folders, and known raw game files;
  unsupported or malformed inputs fail without partial publication.
- `mgr preflight` exposes the bounded read-only setup/recovery report;
  `mgr recover` requires the exact current abandoned-operation token and either
  interactive confirmation or `--yes`; `status --json` returns the full native
  report. Reset is Manager-only and refuses a Studio deployment, Remove points
  to the required next Apply, and zero-row Analyze output keeps coverage gaps
  visible as "no recognized conflicts" rather than claiming conflict freedom.
- `loc`, `audio`, `voice`, `texture`, `asset`, `as` — edit localized text, FMOD
  banks, voice-over archives, IoStore textures, cooked DataAssets and the
  AngelScript cache.
- `location-catalog` and `location` — build the named-location catalog, and look
  a waypoint or spot name up in it offline.
- `mcp serve` — all 87 commands over the Model Context Protocol, with protected
  installation, Manager-library, deletion, and in-place-rewrite operations
  confirmed first.
- `guide` — the manual, built into the binary and rendered to one HTML file.
- Commands whose evidence is audited against a specific game build say which
  build you have and which ones were audited, instead of quietly doing less.
- `as qualify` derives everything a new game build needs to be audited, and
  refuses rather than guessing when the evidence does not hold together.
- Prepared AngelScript mini-caches now resolve their absolute private
  StaticNames operands during loadout composition while raw minis retain their
  local-index convention; missing prepared rows fail closed.
- A 2026-08-18 real-install Manager campaign ran Nexus mods #244, #512, #269,
  and Attack Input V4, verified both numeric #244/#512 order directions, loaded
  a new game and an existing save, exercised enable/disable/reorder/Reset, and
  restored the captured loadout byte-for-byte. Its live Viper AngelScript proof
  was a GORE-authored fixture using the PR #91 Core DLL, not a third-party or
  three-way script qualification. Clean-Windows package acceptance remains open.
