# GORE AngelScript standalone compiler sidecar

This directory contains the hermetic native process boundary and the pinned,
modified UNREANGEL AngelScript core used by the standalone compiler. The
core is now a Windows-x64/MSVC static library with a source frontend, mixed
cache/source graph compiler, whole-graph cache exporter, and a working
Protocol-v1 compile operation. The operation accepts only a qualified,
digest-bound profile and sealed base/Binds/source inputs, then publishes the
new full cache with create-new atomic rename semantics.

No target links or loads Unreal Engine or a game DLL. Nothing launches the game.
CMake performs no downloads and the source tree contains no generated SDK or
game artifacts. Productive G1R compilation requires a completely qualified,
signed internal profile package; unknown, incomplete, or incompatible profiles
fail closed.

The native manifest loader admits exactly two complete target tuples:
Steam BuildID `24539464` with depot manifest `1585071322101748861`, and Steam
BuildID `24878692` with depot manifest `382135126159906494`; both require AppID
`1297900`, depot `1297901`, Windows/x86-64/Shipping. Fields from the two tuples
cannot be mixed. These are profile-provenance authorities, not whole-file
executable-hash runtime gates: product target compatibility remains the Rust
parent's authenticated cache/API qualification.

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
3. run the optional post-type hook used to classify generated delegate types;
4. generate functions for every successful module;
5. lay out classes across the graph, then calculate deferred template sizes;
6. lay out functions for every successful module;
7. compile code for every source builder and release each builder;
8. validate deferred template instances once for the graph;
9. initialize globals only when the graph has no compile error.

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

The source-only graph remains useful as a focused frontend boundary. The mixed
cache/source bridge described below now owns active/precompiled selection and
reference replacement for edited modules. Automatic dependency closure and
the remaining G1R-only records must still come from decoded cache/profile facts;
the native layer does not infer or manufacture them.

The compatibility layer is also not an Unreal implementation. Its containers
and hashing are sufficient for the generic core checkpoint, settings are safe
defaults rather than profile values, numeric scans use the CRT, and the
UObject-backed `asIScriptObject::GetObjectType()` bridge returns no type. These
are explicit parity boundaries, not claims about G1R cache equivalence.

## Frontend/module-graph checkpoint

`module_preprocessor.hpp` and `src/module_preprocessor.cpp` implement the
bounded source front of the pinned preprocessor. It covers effective
conditionals, exact whitespace-preserving directive/manual-import removal,
explicit-import DFS order and diagnostics, class/struct/enum/namespace chunks,
class/default/specifier metadata, UPROPERTY/UFUNCTION, delegate/event wrappers,
name and format strings, range-for and literal-asset lowering, static classes,
native superclass analysis and generated Actor/Component/Subsystem helpers.
Automatic-import mode preserves the donor's source/input behavior. Deliberate
donor quirks such as `ReadIdentifier`, `KillRawLine`, and its `0..1`
identifier-start digit range are retained rather than normalized.

The decoded cache contributes authoritative base class ancestry. Add overlays
must not collide with base modules; edit overlays must name and replace one.
Native roots are mapped from bound AngelScript names to serialized Unreal class
paths, property offsets, derivation bans and helper categories. Haze versus
non-Haze specifiers and mandatory server-RPC validation are explicit sealed
profile switches rather than build guesses.

`frontend_compile.hpp` materializes successful module descriptors, attaches
shadow/delegate pre-class data, binds dependencies and invokes the graph
barriers atomically. Its smoke compiles a consumer against a provider's
preprocessed USTRUCT and proves a rejected graph leaves no module behind. The
product FullGraph path additionally binds the `ClassAnalyze` behavior first
reversed on BuildID `24539464` and retained only after the BuildID `24878692`
differential qualification, enforces both supported targets' unbound `OnProcessChunks` and
`OnPostProcessCode` delegates, performs editor/release source discovery,
requires canonical UTF-8 source bytes, resolves automatic dependency closure,
and uses captured identities for non-ASCII FName comparison. Processed-code
hashes match the donor's XXH64(seed 0) over UTF-16LE `FString` code units,
including its empty-code sentinel and strict UTF-8-to-UTF-16 conversion
boundary. All of these dimensions are mandatory differential-corpus witnesses,
not capabilities inferred from a successful parse.

The Rust compiler profile now parses and validates all three frontend payloads
as typed, independently digest-bound schemas. The preprocessor payload carries
the final effective flag map, direct settings, Haze/RPC build switches,
blueprint-event argument specializations and the native-superclass projection
needed by source generation. The class-generator payload carries its direct
transient-setting input; compiler options carry the five diagnostic switches
read directly by the modified builder/compiler sources. Bind-only settings
remain represented by their effective ordered registry trace rather than
duplicated as inputs. The native profile loader now projects these payloads
into the preprocessor, compiler settings and registry runtime after verifying
their manifest seals. The class-generator transient switch is parsed and
sealed separately from `ClassAnalyze`. Both supported target profiles capture it as
`false`; implicit struct-property metadata and script-class omission are
explicit differential witnesses so the sidecar cannot silently ignore the
donor's independent `RequiresProperty=false` default or its class-versus-struct
transient rule.

Protocol staging also names every authored add/edit overlay explicitly. The
sealed source tree still contains compatibility decompilations for the game
backend, but a standalone implementation can no longer confuse those lossy
base reconstructions with authoritative new source. Base descriptors and
bytecode come from the decoded sealed cache; only the named overlays are source
frontend inputs.

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
authoritative direct import list/order and all preprocessor-only fields are
supplied by the sealed mixed-graph module descriptor rather than inferred here.

The engine smoke compiles real free functions, a value struct and a reference
class, exports and codec-roundtrips them, rehydrates two modules into a fresh
engine, binds an imported declaration, and executes reference-relocated calls,
registered globals and a rehydrated struct method. It also compiles a consumer
against rehydrated value/reference class metadata, exercises synthetic base and
derived layouts, and proves that an invalid tail mapping rejects atomically.
Reference-class allocation is not executed in this hermetic smoke because the
fork delegates those objects to Unreal allocation/runtime semantics.

`compile_mixed_cache_checkpoint` now substitutes edited source modules directly
into the pristine cache graph and appends source additions under one engine
build session. It creates every final module and type shell before declarations,
publishes cached function declarations before source Stage 2, resolves a single
cross-source/cache layout dependency graph, relocates unchanged bytecode only
after replacement identities exist, compiles source code, validates templates,
and initializes globals atomically. Its smoke deliberately shifts both target
type and function IDs, changes an edited struct's property offset, executes an
unchanged precompiled consumer against the replacements, compiles an added
module against that consumer, and proves a parse failure leaves no module.

The mixed path now validates cached G1R descriptor records, recreates native
shadow layouts only from sealed native-super rows, retains cached statics and
post-init records, and projects source class/property/function metadata plus
delegate/event declarations. String-literal globals are recreated through the
registered string factory rather than treated as ordinary global storage.
For a source-only full graph it also derives the Shipping StaticJIT fixed point
from a disposable rehydration of the sealed base. New and fully analyzed
modules use their fresh Stage-3 candidates; partially analyzed modules retain a
base-FINAL function only when the stable declaration has a constructor,
destructor or generated-function role, or maps uniquely to the preprocessor's
UFUNCTION identity. The complete projection is preflighted before mutation, so
ambiguous descriptors and same-name ordinary overloads fail closed without
leaking FINAL traits. These two boundaries -- string-factory instantiation and
StaticJIT candidate projection -- are the only compiler/export-path changes
required by the 2026-08 generation; the parser, language semantics and bytecode
generator remain unchanged.
`export_mixed_graph_checkpoint` re-exports every final module into one fresh
reference-table namespace, restores descriptor-only metadata for unchanged
modules, projects source metadata, and validates the complete encoded wire
artifact atomically. The mixed smoke then decodes that artifact, loads it into
a second fresh engine, executes edited/cached/added modules and a string
literal, verifies a profiled shadow-property offset, and cleanly destroys both
engines.

The plain complete-cache rehydrator intentionally keeps its narrower
fail-closed contract. The product path uses the mixed graph bridge together
with a sealed captured registry/frontend profile, exact source/FName behavior,
the reversed `ClassAnalyze`/ComposeOnto semantics and the mandatory
differential oracle corpus. Protocol v2 is the production FullGraph contract;
protocol v1 remains a source-level compatibility smoke only. Create-new
publication is covered from request through encoded cache.

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

All nine callbacks now have closed, version-pinned implementations. The five
class-constrained callbacks (`TSubclassOf`, `TObjectPtr`, `TWeakObjectPtr`,
`TSoftObjectPtr`, and `TSoftClassPtr`) use the fork's exact subtype check.
`TArray`, `TMap`, `TSet`, and `TOptional` resolve the sealed primitive and
application-type operation tables plus dynamic script struct/object/enum
formulas, cache engine-lifetime operation records, reject nested containers,
and preserve the fork's exact validation strings. Array `opCmp` discovery and
`isInUse` marking and the exact `uint32 Hash() const` fallback for set/map keys
are reproduced as compile-time side effects.

Script delegate and multicast-delegate traits are sealed separately because
the game distinguishes them through class-generator user-data tags. The source
graph bridge calls `classify_dynamic_script_type` immediately after the type
barrier. The precompiled loader must do the same when it creates one of those
tagged types. An unclassified script value with
non-null user data is rejected rather than guessed. The registry smoke covers
all nine adapters, primitive and application values, script structs/enums,
hash fallback, `opCmp`, cached invalid instances, exact negative diagnostics,
descriptor mismatch and pre-mutation validation.

The registry smoke deliberately uses a deterministic synthetic profile. The
product replay layer consumes the separately captured G1R registry only through
all three sealed documents and refuses to compile unless they parse,
cross-reference, replay and read back exactly. Capture alone does not make a
profile distributable; catalog admission still requires the complete embedded
versus standalone differential promotion gate.

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
gore-as-standalone-compiler qualify --request <utf8-json-file>
```

Protocol versions, the semantic compatibility id, and hard limits live in
`include/gore_as_standalone/protocol.hpp`. `--capabilities` reports that id so a
freshly rebuilt or signed executable can use already qualified profiles without
being byte-identical to the historical reference. Responses are one bounded JSON object
on stdout. Exit status 0 means success, 64 invalid CLI use, 65 invalid request
data, 69 capability unavailable, and 70 an internal software error.

The compile command parses the complete closed request schema, rejects unknown
fields, opens regular inputs without following a final reparse point, verifies
every length/SHA-256 seal, recomputes the domain-separated profile identity,
loads all sixteen profile blobs, replays the registry, preprocesses the ordered
add/edit overlays, compiles the mixed graph, exports and encodes the full cache,
then atomically renames a private sibling temporary file to the requested new
output path. The requested path is never partially written and is never
replaced. Diagnostics preserve source/line/column where the core supplies them.

The capability response reports compile availability together with
`requires_qualified_profile=true`. This means the implementation exists, not
that an uncaptured game build is accepted. Exit 69 denotes an unavailable or
mismatched qualified profile; malformed inputs and source rejection use exit
65, and output/internal failures use exit 70.

`qualify` is an additive promotion-only v3 operation. The normal `compile` command rejects v3.
It preserves FullGraph-v2 target/profile/cache sealing but may load the typed unqualified
materializer state, emits request-digest-bound native hook/build evidence, and permits only
instruction-whitelisted zero-argument invocation. Requests contain no supplemental witness fields.
Primitive cases stay inside the VM. The sealed TArray<int32>, FName, and FString corpus rows may
activate narrowly matched donor-ABI adapters only when captured declarations, layouts, type
operations, call conventions, hidden metadata, string factory, and FName comparison identities all
agree. Every other host call and host-object return stays unavailable instead of executing the
registry's compile-only inert stubs.

## Provenance and extraction boundary

`vendor/unreangel` contains 62 byte-exact files from the pinned UNREANGEL commit,
two explicitly inventoried downstream modifications, and the root license notice.
`SOURCE_INVENTORY.tsv` records every imported source file and notice with an exact
source path and repository-canonical SHA-256; CRLF and LF checkouts are reduced to
LF before hashing, while every other byte remains exact. Modified rows additionally
record the current vendored SHA-256. `PROVENANCE.toml` explains both changes. The inventory also names candidate
semantic extractions, reference-only files, dead/foreign call backends, and UE
subtrees that remain excluded. The two embedded xxHash files are not imported;
their future inventory rows identify BSD-2-Clause and require retention of their
in-file notices.

Inventory selectors use repository-relative `/` paths at the recorded revision.
`exact` matches one file and `prefix` a complete subtree. A prefix is only a
future/exclusion boundary: every actual import must first be expanded to exact
file rows with SHA-256 values and reflected in `PROVENANCE.toml`.

Given a checkout of the recorded revision, verify every upstream hash, exact or
explicitly modified vendored canonical text, and inventory/tree membership mechanically with
`tools/verify-source-inventory.ps1 -UpstreamRoot C:\\path\\to\\UNREANGEL`.

## Internal product packaging

The compiler has a semantic compatibility id independent of the exact executable hash. Qualified
profiles retain the exact historical sidecar identity that produced their differential evidence;
the checked-in source asset contains those profiles but no executable. Every CLI/Studio build
compiles and tests a fresh sidecar from this source tree. A product release signs that copy exactly
once, pins its final length/SHA-256 in the embedded catalog, and excludes it from later directory
signing. No separate compiler release or promotion workflow exists. The exact order, qualification
rules, compatibility selection and game-compiler fallback are documented in
[`docs/standalone-compiler-internal-bundle.md`](../../../../docs/standalone-compiler-internal-bundle.md).
