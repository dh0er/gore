# GORE AngelScript standalone compiler sidecar

This directory contains the hermetic native process boundary and the pinned,
modified UNREANGEL AngelScript core used by the future standalone compiler. The
core is now a Windows-x64/MSVC static library with a real generic compile smoke;
the sidecar's `compile` operation remains deliberately fail-closed until the
G1R profile, multi-module orchestration, preprocessing, and cache emitter are
integrated.

No target links or loads Unreal Engine or a game DLL. Nothing launches the game,
and the sidecar does not yet write compiled output or cache data. CMake performs
no downloads and the source tree contains no generated SDK or game artifacts.

## What the core checkpoint proves

`gore-as-unreangel-core` builds 27 generic translation units from the exact
UNREANGEL revision recorded in `PROVENANCE.toml`. Its compatibility layer is a
narrow replacement for the small Unreal Core surface used by those files.
`gore-as-unreangel-core-smoke` creates an engine, adds a recursive function as
source text, runs the lexer/parser/builder phases, and verifies that the built
function has non-empty bytecode.

The `build_module` adapter is intentionally only a single-module generic smoke
path. It is not parity with `FAngelscriptManager`: the game manager establishes
phase barriers across the complete module graph (parse all modules, generate
types across the graph, then functions/layout and code). A standalone
multi-module graph orchestrator remains required before G1R compilation can be
claimed.

The compatibility layer is also not an Unreal implementation. Its containers
and hashing are sufficient for the generic core checkpoint, settings are safe
defaults rather than profile values, numeric scans use the CRT, and the
UObject-backed `asIScriptObject::GetObjectType()` bridge returns no type. These
are explicit parity boundaries, not claims about G1R cache equivalence.

## Build and test

Use a Visual Studio 2022 x64 developer environment:

```powershell
cmake -S . -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release
ctest --test-dir build -C Release --output-on-failure
```

The targets use the static MSVC runtime and Windows system APIs only.

## Command boundary

```text
gore-as-standalone-compiler --version
gore-as-standalone-compiler --capabilities
gore-as-standalone-compiler compile --request <utf8-json-file>
```

Protocol versions and hard limits live in
`include/gore_as_standalone/protocol.hpp`. Responses are one bounded JSON object
on stdout. Exit status 0 means success, 64 invalid CLI use, 65 invalid request
data, 69 capability unavailable, and 70 an internal software error.

The compile command validates only the safe transport envelope. It does not yet
claim to parse the sealed request schema: every otherwise acceptable request
ends with `GORE_AS_STANDALONE_ENGINE_UNAVAILABLE` and exit status 69. The
capability response exposes the available core independently and leaves full
compilation unavailable. Existing protocol-v1 fields and meanings are unchanged.

## Provenance and extraction boundary

`vendor/unreangel` contains byte-exact files from the pinned UNREANGEL commit and
its root license notice. `SOURCE_INVENTORY.tsv` records every imported source
file and notice with an exact source path and SHA-256; it also names candidate
semantic extractions, reference-only files, dead/foreign call backends, and UE
subtrees that remain excluded. The two embedded xxHash files are not imported;
their future inventory rows identify BSD-2-Clause and require retention of their
in-file notices.

Inventory selectors use repository-relative `/` paths at the recorded revision.
`exact` matches one file and `prefix` a complete subtree. A prefix is only a
future/exclusion boundary: every actual import must first be expanded to exact
file rows with SHA-256 values and reflected in `PROVENANCE.toml`.

Given a checkout of the recorded revision, verify every upstream hash, vendored
byte, and inventory/tree membership mechanically with
`tools/verify-source-inventory.ps1 -UpstreamRoot C:\\path\\to\\UNREANGEL`.
