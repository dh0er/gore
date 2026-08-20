# Changelog

All notable changes to gore-cli are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- `as emit` / `as emit-all` now write class `default` statements, so item, NPC
  and config classes decompile with their values instead of as empty shells.
- A module whose defaults cannot all be recovered says so in its header and
  keeps them byte-exact on recompile.
- A module whose source declares defaults can be edited and spliced back;
  `as emit --no-defaults` still produces the previous shape.
- Scalar member stores into fields declared on a native base are recovered
  instead of dropped.
- Editing a module and splicing it back now keeps its `n"..."` names: they were
  remapped to unrelated entries before, so an item could come back wearing
  another item's model.
- Decompiled source keeps namespaces, `const` methods and parameter defaults, so
  quest, document and conversation modules can be edited and spliced at all.
- Every module in the game writes its class defaults now, down to the main map's
  worldpoint and item-spawn tables.
- A fluent chain keeps its links: a temporary's destructor between two of them
  no longer ends the statement, so AI rule tables decompile as the one call
  chain they were written as.
- A `Cast<>` comes back as a cast instead of the compiler's null-guarded
  if/else, and a bool field written from an int gets the bool form.
- A method's `const` return type is written again, so an edited module keeps
  that part of its identity; locals that receive such a value are declared
  const to match.
- A `default` statement whose call lost an argument is refused instead of
  written, so a module keeps its byte-exact defaults rather than quietly
  changing meaning.
- Decompiled bodies write the range-for the compiler desugared, fold the
  temporaries it invented, drop stores nothing reads, and call `Super::` where
  an override calls the method it overrides.
- `as extract-remap` and `compile-module` resolve a repeated string literal
  instead of refusing the module.
- `GORE_AS_REMAP_DIAG=1` prints the two identities behind an unresolved or
  ambiguous reference.

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
