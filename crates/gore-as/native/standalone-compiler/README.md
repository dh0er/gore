# GORE AngelScript standalone compiler sidecar

This directory contains the hermetic native process boundary and the pinned,
modified UNREANGEL AngelScript core used by the future standalone compiler. The
core is now a Windows-x64/MSVC static library with a real generic compile smoke;
the sidecar's `compile` operation remains deliberately fail-closed until the
G1R profile, preprocessing, and cache emitter are integrated.

No target links or loads Unreal Engine or a game DLL. Nothing launches the game,
and the sidecar does not yet publish compiled output. CMake performs no downloads
and the source tree contains no generated SDK or game artifacts.

## What the core checkpoint proves

`gore-as-unreangel-core` builds 27 generic translation units from the exact
UNREANGEL revision recorded in `PROVENANCE.toml`. Its compatibility layer is a
narrow replacement for the small Unreal Core surface used by those files.
`gore-as-unreangel-core-smoke` creates an engine, adds a recursive function as
source text, runs the lexer/parser/builder phases, and verifies that the built
function has non-empty bytecode.

`build_module_graph` now runs one engine build session with the phase barriers
used by the pinned `FAngelscriptManager` initial-build path:

1. parse every module;
2. generate types for every successful module;
3. generate functions for every successful module;
4. lay out classes across the graph, then calculate deferred template sizes;
5. lay out functions for every successful module;
6. compile code for every source builder and release each builder;
7. validate deferred template instances once for the graph;
8. initialize globals only when the graph has no compile error.

`BuildCompleted` is paired exactly once with a successful `RequestBuild`.
Failures retain the first phase/module result, release every builder, reset the
partial graph in reverse input order, and leave the engine reusable.
`build_module` remains as a one-element compatibility wrapper over this path.
The current implementation executes modules serially within each phase; it
preserves the manager's barriers but does not claim its parallel parse
throughput.

`gore-as-unreangel-graph-smoke` places a consumer before its imported provider.
The consumer uses a provider-owned enum in its function declaration, so it can
only compile after the graph-wide type barrier. The same test injects a parse
error and then successfully rebuilds that module, covering builder lifetime and
engine build-lock release.

This is phase-orchestration parity, not yet a complete G1R module graph. The
standalone caller still needs authoritative inputs for module discovery,
declared/automatic import edges, dependency ordering, code-class and delegate
pre-class metadata, active/precompiled-module selection, and hot-reload
reference-replacement policy. Those values must come from preprocessing and the
sealed G1R profile; the native layer does not infer or manufacture them.

The compatibility layer is also not an Unreal implementation. Its containers
and hashing are sufficient for the generic core checkpoint, settings are safe
defaults rather than profile values, numeric scans use the CRT, and the
UObject-backed `asIScriptObject::GetObjectType()` bridge returns no type. These
are explicit parity boundaries, not claims about G1R cache equivalence.

## PrecompiledData codec checkpoint

`gore_as_standalone/precompiled_data.hpp` and `src/precompiled_data.cpp`
implement the complete bounded wire schema used by the pinned fork's
`FAngelscriptPrecompiledData`. This includes every nested function, class,
property, enum, global and import record plus all seven reference/name tail
tables. TMap order and raw `FStringInArchive` bytes are retained, UE `FString`
module keys support validated ANSI and UTF-16, archive bools must be canonical,
counts/strings/total bytes are bounded, and decode requires exact EOF.

The codec covers two easy-to-miss fork details explicitly: `ConfigName` is a
variable-width class field after seven serialized bools, and the global
`InitFunc` record is archived even when `bHasInitFunction` is false. The broad
codec smoke covers every conditional branch and failure leaves the caller's
output unchanged. An optional path argument makes the same executable perform a
byte-exact decode/encode comparison against an external sealed cache; the
qualified BuildID 24539464 Shipping cache (124,354,799 bytes) passes.

The engine bridge now implements the generic fork-side portion of all three
`PrecompiledData` apply stages. It restores module imports and imported-function
signatures, enums, script class/struct inheritance and layouts, methods,
constructors, destructors, behaviour tables, non-primitive globals and global
initializers. It also recreates data types and relocates the fork's exact
type/function/function-id/global/property reference-bearing bytecodes. Saved
IDs are resolved by semantic identity rather than copied, so the target smoke
deliberately skews a registered type ID before applying the cache.

The matching exporter derives functions, object locals, classes, globals,
initializers and all required reference-tail rows from compiled engine objects.
Output and reference tables are staged and become visible only together on
success. A generic engine exposes only its flattened imported-module view; the
authoritative direct import list/order and all preprocessor-only fields must be
supplied by the future module descriptor rather than inferred here.

The engine smoke compiles real free functions, a value struct and a reference
class, exports and codec-roundtrips them, rehydrates two modules into a fresh
engine, binds an imported declaration, and executes reference-relocated calls,
registered globals and a rehydrated struct method. It also compiles a consumer
against rehydrated value/reference class metadata, exercises synthetic base and
derived layouts, and proves that an invalid tail mapping rejects atomically.
Reference-class allocation is not executed in this hermetic smoke because the
fork delegates those objects to Unreal allocation/runtime semantics.

This remains an engine-apply checkpoint, not full G1R cache parity. Unreal
shadow layouts and class/property/function metadata, delegate/event tags,
statics and post-init hooks, string-literal globals, an actual captured G1R
registry, authoritative preprocessing/module discovery and safe sidecar output
publication remain mandatory and fail closed.

## Registry replay checkpoint

`gore_as_standalone/registry_profile.hpp` and `src/registry_profile.cpp`
provide the native projection of GORE's sealed registry contract. Replay
requires a fresh engine, preflights the complete profile before its first
mutation, applies the ordered engine properties and effective registration
contexts, then reproduces every public registration class. Expected engine
type/function IDs and owner/member indices are checked 1:1 before the captured
post-bind type/property/function/global state is applied and read back.

Runtime function and object addresses are never accepted from a profile.
Ordinary bindings receive call-convention-compatible inert compile-only stubs,
global storage is bounded and aligned, and the string factory owns byte-exact
string constants for the lifetime of the engine. Different typedef names may
correctly share one primitive engine type ID, matching `RegisterTypedef` in the
pinned fork. Owner kinds, all stub references, result identities and complete
final-state coverage fail closed; the smoke also proves these rejections occur
before engine mutation.

The five class-constrained callbacks (`TSubclassOf`, `TObjectPtr`,
`TWeakObjectPtr`, `TSoftObjectPtr`, and `TSoftClassPtr`) use the fork's exact
subtype check. `TArray`, `TMap`, `TSet`, and `TOptional` deliberately return
`asNOT_SUPPORTED` during preflight. Their acceptance algorithms depend on the
game's `FAngelscriptTypeUsage` operation registry, including construction,
copy, destruction, comparison, hashing, size/alignment and nested-container
rules. That semantic registry must be captured and ported before these four
adapters can be enabled; accepting them generically would change the language.

The registry smoke uses a deterministic synthetic profile. No G1R registry
payload has been captured or qualified yet, and this replay layer is not yet
wired to the JSON sidecar request.

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
