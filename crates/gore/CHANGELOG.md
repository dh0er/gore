# Changelog

All notable changes to gore-cli are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- Add the 2026-08-27/28 game generation (Steam BuildID 24878692) to the
  structural cache/API compatibility registry. Compatible repacks remain
  qualified by format and ordered Binds API rather than a whole-EXE hash.
- Update the native compiler diagnostics hook and capture contracts for the
  new Shipping layout while retaining structured source, line, column,
  severity, and native compiler messages.
- Recreate cached string literals through the registered string factory and
  project the Shipping StaticJIT FINAL/candidate boundary structurally for
  source-only full graphs. Full-tree comparisons for both supported profiles
  now have exact structural equality with zero semantic or alignment
  differences; the parser and bytecode generator remain unchanged.
- Require both supported compiler profiles and a strict full-tree differential
  receipt per profile in internal-input V2 publication packages. The pinned
  signed V1 package remains usable for local builds only and is rejected by
  distribution, installer, tag, and push paths until an explicitly authorized
  final promotion replaces it.

## [0.2.0] - 2026-08-27

- Bundle the qualified standalone AngelScript compiler with gore-cli. Script
  builds and `as compile-module` can now compile without starting the game.
- Compile the complete script project in one standalone run, including added,
  edited, and deleted modules, cross-module references, and emitted sources
  whose global function names collide across modules.
- Stage only authored script changes and reuse already authenticated compiler
  inputs instead of exporting and copying the full 7,000-plus-module base tree
  for every compile. In the one-file comparison used during qualification, the
  standalone path completed faster than the game's embedded compiler.
- Use `standalone-then-game` by default: GORE shows why the standalone attempt
  was rejected before using the game's embedded compiler as a fallback.
  Strict `standalone` and explicit `game` modes remain available.
- Match compatible game installations by Shipping cache format and the
  complete AngelScript Binds API instead of requiring an exact whole-EXE hash,
  so compatible Steam, GOG, and differently packed binaries are supported.
- Add dedicated MCP tools for strict standalone script compilation. They cannot
  select or fall back to the game compiler and therefore never ask for game
  launch or installation-write approval.
- Make `gore doctor` authenticate the bundled compiler and check the installed
  Shipping/Binds inputs through the same compatibility resolver used by real
  standalone compilation.
- Preserve structured standalone diagnostics with source file, line, column,
  severity, code, and message, and keep temporary compiler work isolated and
  cleaned up after success or failure.
- Reject ambiguous global calls and function handles before compilation while
  preserving calls whose overload is proven safe, including qualified calls
  in return expressions and arguments containing generic types.
- Add `gore mod inspect` and its read-only MCP alias for bounded offline
  validation of bundle directories and ZIPs, including declared payloads,
  UE4SS `Scripts/main.lua`, component formats, and deterministic hashes.
- Add `gore voice validate` for read-only Ogg/Vorbis and Opus validation with
  exact duration metadata, and reject invalid end-of-stream timing before a
  voice bundle is built.
- Derive newly added AngelScript module identities from their relative `.as`
  paths, matching both the standalone compiler and the game's source discovery.

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
