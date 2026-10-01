# Scripts (AngelScript) — experimental

The game's compiled AngelScript lives in a precompiled cache,
`$GAME\G1R\Script\PrecompiledScript_Shipping.Cache`. `gore as` reads that cache,
turns modules back into readable AngelScript, compiles complete source graphs or
individual modules, and splices edited modules back in.

This is reverse-engineering-stage tooling. It works, and the current
same-module Diego dialog path has been validated in game on BuildID `24878692`,
but treat every step as experimental and keep backups. Compilation, bundle
packaging, installation and observed runtime behavior remain separate claims.

## Reading the cache

```powershell
$CACHE = "$GAME\G1R\Script\PrecompiledScript_Shipping.Cache"

gore as info          "$CACHE"            # module count + TAIL_OFF (the splice insertion point)
gore as decode-header "$CACHE"            # the outer cache header
gore as decompile     "$CACHE" <needle>   # → readable AngelScript
gore as emit-all      "$CACHE" out_as     # every module as recompilable .as
gore as emit          "$CACHE" <needle>   # only modules matching <needle>
gore as disasm        "$CACHE" <needle>   # asBC bytecode listing
gore as static-names  "$CACHE"            # the n"…" FName literal pool
gore as walk          "$CACHE"            # raw type-name string scan (decode aid)
```

Every one of these takes a module cache and proves it first: the `0x9e377abe`
magic at offset 0x10 is checked before anything walks the container, so pointing
one at `Binds.Cache` or another side table names the format mismatch and the
path rather than failing somewhere inside the parse. `walk` is no exception —
its raw string scan starts after the outer header, so it reads caches, not
arbitrary blobs.

`<needle>` is a substring filter on `module.Class::func` and defaults to
everything. `decompile` and `disasm` print at most `--max` functions (default
20); `emit` at most `--max` modules (default 5); `emit-all` has no limit and
mirrors each module's `ScriptRelativeFilename` into the output tree.

`static-names` with no arguments prints the entry count plus the first ten
entries; pass indices to print specific ones. These are the literals that
`__STATIC_NAME(Id)` resolves against.

Decompilation and emit resolve native-call arities and native field types from
the **matching** `Binds.Cache` placed next to the input cache, or from the path
in `GORE_AS_BINDS`. Keep both caches from the same game build. Without that
type evidence GORE does not guess native enum or scalar field types: if such a
store occurs in `__InitDefaults`, it suppresses authored defaults for the whole
module and leaves recompilation to the byte-exact carry fallback.

Emitted classes carry their `default` statements — the class-scope statements
that give an item its name, value and damage, an NPC its config, a camera its
settings. They come out of the compiler-generated `__InitDefaults` method, which
holds every one of them:

```angelscript
class UItMw_1H_Sword_Old_01 : USword1H
{
    default m_Name = "ItMw_1H_Sword_Old_01";
    default m_Value = 10;
    default m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Edge, 10.0f);
    default SetItemType(GameplayTag::Item_Weapon_Sword_OneHand);
}
```

You can edit those statements and splice the module back with `compile-module
--op edit`; the compiler regenerates the class defaults from your source and the
old copies are dropped rather than carried. Every existing default-bearing class
and every existing semantic target must remain represented at least as often as
in the base cache; values and call arguments may change, but a partial overlay is
refused instead of silently losing defaults. Authored-default edits containing
preprocessor directives are also refused: coverage is checked before compiler
preprocessing, so a disabled branch must not count as a surviving default.

If you would rather not author defaults for existing classes, emit the module
with `gore as emit --no-defaults` (or `emit-all --no-defaults`) and edit that:
the module's existing defaults are carried back byte-exact instead of being
regenerated. With `--allow-new-symbols`, that fallback also permits defaults on
appended classes while continuing to carry every existing class initializer and
compiler wrapper byte-exact. It does not permit a mixture in which only some
existing classes author their defaults.

Every module in the shipped game writes its defaults, down to the main map's
worldpoint and item-spawn tables. Recovery stays all-or-nothing per module: if a
class in some future build cannot be recovered in full, the module's header says
so by name and reason and none of its defaults are written. Nothing is lost
either way — a module without authored defaults keeps them byte-exact when it is
recompiled.

How much of the decompiled tree is proven identical to the shipping cache — and which of those
numbers come from the whole corpus rather than a sample — is written down in the repository, in
`crates/gore-as/DECOMPILER_STATUS.md`.

## Engine callbacks in new classes

To override a native Blueprint event in a new script class, use
`UFUNCTION(BlueprintOverride)` and the unsuffixed event name. For example:

```angelscript
class UAIState_MyWalk : UGothicCharacterSimulateableAIState
{
    UFUNCTION(BlueprintOverride)
    void DoTask()
    {
        ::GotoPreferredLocation(this.AI);
    }
}
```

Likewise, a graceful-exit handler is `UFUNCTION(BlueprintOverride)` followed by
`void OnGracefulExitRequested()`. The compiler adds `_Implementation` to the
script method and records its binding to the native event. This rule concerns
overridden native events; ordinary helpers and delegate/timer callbacks do not
become Blueprint overrides just because another object calls them.

The emitter prints bare `UFUNCTION()` and suffixed implementation names for
these methods; it does not reproduce every reflection flag. Editing an existing
shipping declaration restores its original metadata, but a newly authored class
has no shipping metadata to restore. Copying the emitted form into a new class
can therefore compile and install successfully while its callback never runs.
Comparing emitted source before and after installation cannot detect that loss.

This occurred in the NPC activity fixture: both `DoTask` and the graceful-exit
handler were ordinary callable methods, so the new state never reached its
navigation/action body. See the
[failure and correction](../../scripts/fixtures/npc-appearance-routine/RESULTS.md).
Correct native event names and override/event flags are verified in the binary
cache separately from the subsequent user-run gameplay test.

## Recompiling: standalone first, game fallback

The MCP server automatically includes a read-only compiler readiness snapshot
in its initialization instructions. It checks the configured or auto-detected
game before the first tool call and does not launch or modify the game. For a
different game or changed inputs, run Doctor again.

Before exporting a tree or starting script authoring, run `gore doctor`. Its
`standalone_compiler` check must be `ok` for the offline compiler route. Doctor
returns exit code 0 when it produced a report, including reports with problems;
automation must inspect the verdicts in `gore doctor --json`.

An `ok` verdict establishes authenticated compiler and target cache/API
readiness, including `native_api: ready` for the sealed extended native
declarations. A compatible compiler with missing native authority is a
`problem`: sources may compile while selective or Manager composition refuses
them. Doctor and MCP startup check both using the authenticated input bytes.
This does not compile the requested sources or prove that every
reconstructed native call binds. Workspace checks, successful compilation,
bundle inspection and observed gameplay are separate evidence. Unchanged
shipped source can still expose a toolkit/compiler defect after readiness passes.

Doctor reports the running CLI path and the deployment-aware pristine cache
path/hash. Use that original for exports and `--expect-base-sha256`. With a
script mod installed, exporting the live modified cache would include that
mod and disagree with the original that the compiler selects.

An **absent embedded compiler catalog** means the host `gore.exe` was built
without its product catalog. It does not mean the compiler beside it is too
old. Reinstall the complete matching CLI package, or build it using
`python build.py gore-cli dist`. A plain `cargo build -p gore` without a catalog is refused. Only an explicitly
selected debug `development-cli` build may omit it; that binary is labelled
`development-unbundled` and must not be installed as the product CLI. Copy the
contents of the generated package together, including `compiler/`, and restart
MCP after replacing the CLI. Do not downgrade the CLI to bypass this error.

GORE ships its standalone compiler as an internal part of GORE CLI and Mod
Studio. The normal product commands authenticate the sidecar and profile, then
check the installed game's cache format and ordered AngelScript API. Users do
not provide sidecar paths or hashes.

This check is deliberately independent of Steam or GOG packaging. Whole-file
EXE/cache hashes, Steam build numbers and depot metadata document the build used
for qualification, but they are not runtime locks. A differently packed AMD64
EXE remains usable when its Shipping format and complete Binds API match.

The checked-in source package contains the qualified `24539464` and `24878692`
profiles and their evidence, but no compiler executable. Each CLI/Studio build
compiles and tests a fresh sidecar. The product catalog seals its exact release
bytes while a separate semantic ABI links it to the historical qualification
reference. Rebuilding or signing may therefore change the EXE hash without
turning either Steam tuple or whole executable into a runtime gate.

The mixed cached/source compiler rehydrates unchanged modules from the sealed
base instead of recompiling their source. Its current rehydration restores the
compiler metadata authored overlays depend on: automatic-import relationships
are wired after a source module reset when `AutomaticImports` is enabled,
cached const qualification is reconstructed from both object-const and
const-handle flags,
and cached script enums are published in the engine-wide script-type registry.
Cached `__StaticType` globals are also available through engine-wide automatic
imports, reconstructed script-class type IDs retain their `SCRIPT_OBJECT`,
`TEMPLATE` and `APPOBJECT` kind, and cached mixin globals are exposed only while
source binding runs before their original traits are restored. Those are
compiler-resolution fixes; they do not by themselves prove deployment or game
runtime behavior.

`gore as compile` resolves coordinated Add/Edit sources together, so visible
references between changed modules can bind in one compiler run. The native
standalone backend compiles only those sources and reads unchanged dependencies
from the sealed original cache. Choose the input shape explicitly:

### Compile only authored modules

For a mod changing several modules, put **only the new or edited modules** in a
source directory and use `--overlays --backend standalone`. No `emit-all` export
or full-tree baseline emission is needed. Each file is the complete source of
one module, not a fragment to append to its original. Keep its canonical
Script-relative path, for example:

```text
authored/
  MyMod/Provider.as
  Story/G1R/Conversation/Conversation_OC_STT_DIEGO.as
```

An existing path requests Edit; a new canonical module/path requests Add. Every
supplied `.as` file is an explicit change, even if its bytes equal an original
export. Leave unrelated files out. Sparse input accepts **1–256 modules**;
more than 256 supplied `.as` files are rejected during discovery, before
compilation. Original modules absent from `authored/` stay in the base cache;
absence never requests Delete in this mode. Overlay compilation does not
support deleting a module or bypass module-private symbols.

```powershell
# Author complete new/edited modules under authored/ at their Script-relative paths.
New-Item -ItemType Directory -Force .gore-as-work, out | Out-Null
gore as compile authored --overlays --backend standalone `
  -o out/full.Cache --mini out/MyMod.mini.Cache `
  --work-dir .gore-as-work --game "$GAME"
```

Use `--expect-base-sha256 <hash-from-doctor>` to bind the build to the original
cache you authored against. Optionally repeat
`--only-change add:MyMod.Provider:MyMod/Provider.as` and
`--only-change edit:Story.G1R.Conversation.Conversation_OC_STT_DIEGO:Story/G1R/Conversation/Conversation_OC_STT_DIEGO.as`
to require exactly that change set. Append `:SHA256` to each entry to bind its
source bytes, too. `--only-change` is a **scope check**, not a file filter or a
performance switch: it rejects unexpected, missing or differently hashed
changes. `--overlays` is what avoids exporting and comparing the full tree.

Overlay compilation requires explicit `--backend standalone`. `game`,
`standalone-then-game`, and an omitted backend are rejected before source
planning; there is no game fallback for sparse inputs. The installed pristine
cache and matching Binds API still have to pass normal compatibility checks.

Both input modes publish the same selective full cache and optional
multi-module mini-cache. Untouched modules and every pre-existing global-tail
record remain pristine; only records required by new symbols are appended.
The full cache is never installed implicitly. For a bundle, use the `--mini`
output as described in [Multi-module mini-caches](#multi-module-mini-caches).
New modules can depend on each other in an acyclic chain; cycles among new
modules remain unsupported and fail closed.

The install is resolved from `--game`, else the configured game path, else
Steam auto-detect. Source directory, output and existing workspace are required.
The output must be outside the game installation and the workspace; its parent
and the workspace must be disjoint (neither contains the other). Keep source,
work and output in separate sibling directories as above. Outputs are published
without overwriting existing files.

Through MCP, call `gore_as_compile` with `overlays: true` and these paths as
top-level arguments; the tool selects strict standalone itself. For example:

```json
{
  "src": "authored",
  "overlays": true,
  "out": "out/full.Cache",
  "mini": "out/MyMod.mini.Cache",
  "work_dir": ".gore-as-work"
}
```

The mixed `gore_as` equivalent uses `subcommand: "compile"` and an `args` object
containing those values plus `backend: "standalone"`. Prefer the dedicated tool
for ordinary authoring. A fresh workspace and ordinary outputs outside the
installation need no consent; existing generated work trees retain their write
protection. `gore_as_compile_module` remains the dedicated one-module tool.

### Compile a complete source tree

Without `--overlays`, the original complete-tree contract remains unchanged.
Start from a current `emit-all` tree. Planning re-emits the original modules and
compares their exact source bytes: an unchanged file stays Base, a modified file
requests Edit, and a new file requests Add. Missing base files request Delete
and fail closed because safe tail pruning and retained-reference proof are not
available. `--only-change` checks the resulting change set after that scan; it
does not accelerate it. Reusing an export saves the initial export, not the
repeated baseline comparison.

```powershell
# Use the pristine cache path reported by Doctor (possibly the owned .gore-bak).
$BASE = '<pristine cache path from gore doctor>'
gore as emit-all "$BASE" out_as
# …edit modules in out_as…
New-Item -ItemType Directory -Force .gore-as-work, out | Out-Null
gore as compile out_as -o out/full.Cache --work-dir .gore-as-work `
  --backend standalone --game "$GAME"
```

Keep the emitter and compiler versions matched for this route. An emitter
correction can change the spelling of an untouched call, so an old export may
be classified as thousands of edits. Retain your authored files separately;
refresh only exports proved untouched against the same pristine cache.

A timed-out MCP export may still be running. Establish whether its child process
stopped before retrying; do not rename or reuse a destination an active export
is still writing.

Complete trees also support `--backend game` or `--backend standalone-then-game`
(the default when the backend is omitted). The latter tries standalone first and
reports its failure before falling back to the game's embedded compiler. A
game-backed run launches the shipping executable with
**`-as-generate-precompiled-data`** and temporarily stages loose `.as` files under
`<install>\G1R\Script\`. Its raw whole-tree regeneration is intermediate
dependency evidence and is never the published cache; publication still
selectively composes only the authored Add/Edit modules. Through MCP these
game-capable policies require the mixed `gore_as` route and the corresponding
launch and installation-write consent. `standalone` and `game` never fall back
silently.

### Safety rules around compilation

These are enforced, not advisory:

- Strict `standalone` does not launch the game and does not enter a live-install
  mutation window.
- A game-capable policy (`game` or `standalone-then-game`) **fails closed**
  before any staging if the shipping game process is running, if process
  inspection is unavailable, or if a prior compile/recovery artifact exists.
- Compile, deploy, manager apply, and undeploy share the atomic
  `.gore-install-mutation.lock`, so two toolkit processes cannot mutate the same
  installation concurrently.
- The shipping process is re-checked immediately before the first live-content
  or recovery write. That narrows but cannot eliminate a later launch race,
  because the game does not participate in the toolkit lock. **Keep the game
  closed for the whole operation.**
- A confirmed compiler exit restores every touched path before releasing
  ownership. If process exit or exact restoration cannot be proved, recovery
  artifacts and cross-tool ownership are retained and no usable compile result
  is returned.

### Compiling while a script mod is installed

An installed script mod does not have to be undeployed before its next version
compiles. Every compile route validates the standalone compiler target against
the deployment-aware pristine cache: while a deployment owns the script cache
that is the `*.gore-bak` its record authenticates, otherwise the live file (also
after a game update, when the backup is stale and the updated live cache is the
new original). The CLI says so when it compiles against the backup. The pinned
base is checked again after the pin and, for `gore as compile`, at the end of
the run; if it changed in between, the compile fails closed and asks for a
retry rather than an undeploy. The installed version is replaced only by the
next `gore mod deploy` or Manager apply, which rebuild from the same pristine
backup.

This holds for the standalone compiler. The game compiler regenerates into the
live cache and restores it from the pinned target afterwards, so it cannot run
while a script mod is installed: `--backend game` is refused up front, and
`standalone-then-game` runs the standalone compiler only, saying that the game
fallback was skipped. Should the standalone compile fail, its own diagnostics
are what you see, with that note appended.

To pin which original a compile may use, pass `--expect-base <CACHE>` (a file
the selected original must equal byte for byte, for example a frozen copy of
the vanilla cache) or `--expect-base-sha256 <HEX>`. Both refuse the compile
when the selected original differs and print both hashes; neither picks the
base. The deployment-aware selection stays the only source of truth.

Use that same original for creating, checking and staging related workspaces.
Pass Doctor's pristine path through `--cache` where supported; compare every
workspace manifest's `cache_sha256` with Doctor before combining overlays.
An inspection of the live deployed cache describes the installed modded state
and may have a different hash. If a workspace used the wrong base, preserve it,
create a fresh workspace against the original, carry over only intentional
source edits and check again. Do not rewrite manifest hashes, reset the loadout
or select a `.gore-bak` by filename to silence the mismatch. If staging still
requires the live hash while Doctor and the workspace agree on the pristine
hash, report the conflicting paths/hashes as a toolkit inconsistency.

### Compiler diagnostics

Strict `standalone` returns the bundled compiler's native diagnostics with the
source file, line, column, severity, and message. On the normal fresh-workspace,
outside-install route, diagnostics require no game launch and no consent question.
Existing-path and inside-install write protections still apply. The optional runtime diagnostics
hook belongs only to the game backend; strict standalone compilation never loads it.

For a failure in unchanged reconstructed source, keep the exact diagnostic,
CLI version/path, pristine cache hash and authored module paths. For dialog/quest
work, strictly compile the minimal checkout plus scaffolded topic before adding
quest/content helpers. Compare failing lines with the original source; preserve
emitted shipped calls and defaults. After readiness passes, swapping in an older
EXE, rewriting emitter output or guessing default parameters can conceal a
compiler/emitter defect. Keep the failure unresolved until the affected sources
compile successfully. Sparse overlays
have no game fallback; any deliberate complete-tree game compile still needs
game-launch and installation-write authority.

When a game-capable backend runs on Windows, compile automatically attempts an
embedded, temporary x86-64 diagnostics hook. The selected AMD64 executable must
have exactly one raw masked match for both the AngelScript callback and the
manager's structured ClassGenerator-diagnostic boundary; their sparse
`asSMessageInfo` and `FString`/`FDiagnostic` structure fingerprints must both
verify. Hook-captured errors are then printed like a normal compiler:

```
file:line:column: severity: message
```

Candidate signatures are retained as notes. The helper is never installed into
the game. A missing, changed, or ambiguous signature, a structural mismatch, or
a confirmed hook failure falls back to the unchanged generator.
`--no-diagnostics` is a silent explicit opt-out;
`--diagnostics-inject-delay-ms` (default 2000) tunes the loader warm-up wait.

Audit compatibility without launching the game, including custom and non-Steam
executables:

```powershell
gore as diagnostics-check --game "$GAME"
gore as diagnostics-check --exe "D:\Custom\G1R\Binaries\Win64\G1R-Win64-Shipping.exe"
```

The check reports the executable's SHA-256, both raw match counts and RVA sets,
and both structure-verification results. An explicitly trusted helper
override is available through `--diagnostics-hook DLL` or `GORE_AS_DIAGNOSTICS_HOOK`;
the embedded and sibling release helpers are SHA-256 verified. Internal
full-tree release qualification rejects that development override and requires
one of the verified release helpers.

Archived 1.0.0–1.0.5 executables pass the same offline two-boundary signature
and structure audit. Runtime results for each explicitly supported standalone-compiler
target are recorded with that generation's differential qualification evidence. The native
profile loader accepts only the complete `24539464` and `24878692` target tuples; cross-generation
BuildID/depot-manifest hybrids fail closed. This profile admission is separate from product
runtime compatibility, which remains the structural cache/API check described above rather than
a whole-file executable checksum.

## The normal authoring workflow: one module

For a change confined to one module, `compile-module` returns a deployable
mini-cache. Strict standalone reads unchanged dependencies from the original
cache and needs no full source-tree export:

```powershell
gore as compile-module --backend standalone --op add --module MyMod.Dialog `
  --rel-path MyMod/Dialog.as --source Dialog.as --work-dir .gore-as-work `
  --allow-new-symbols -o MyMod.Dialog.mini.Cache --game "$GAME"
```

| Flag | Meaning |
|---|---|
| `--op add\|edit` | `add` for a new module, `edit` for an existing one. |
| `--module <NAME>` | Expected module name. For `add`, the compiler-detected name is reported and used. |
| `--rel-path <PATH>` | Safe path of the authored file relative to the game's `Script\` tree. |
| `--source <FILE>` | The authored `.as` file to overlay. |
| `--work-dir <DIR>` | Existing workspace outside the game installation for intermediate artifacts; only the game backend emits a full source tree. |
| `--allow-new-symbols` | Retain minimal rows for classes/functions/names absent from the pristine cache. |
| `-o, --out <PATH>` | The remapped 1-module mini-cache. |
| `--expect-base <CACHE>` / `--expect-base-sha256 <HEX>` | Refuse to compile unless the selected original is this file's bytes / has this SHA-256. Neither selects the base; both exist on `compile` as well. |

New native references also need verified engine declarations. GORE includes
sealed qualification of the mesh/material APIs used by the
[NPC head probe](../../scripts/fixtures/npc-head/README.md). This is reusable
technical API evidence: it admits exact native declarations, independently of
the chosen head, clothing, NPC or combination. Compilation, selective FullGraph
composition and Manager composition use the same authority. Complete type and
function identities, datatype flags and native property offsets remain exact;
unknown signatures and unqualified template specializations still fail closed.

Snapshot selection starts from the exact supported generation row for the
pristine cache's hash and GUID. A supported target generation reuses an
authenticated snapshot when its row and the snapshot's source row share exact
Binds bytes, native ancestry, class and field profiles. Build 25414091 can
therefore reuse the existing authenticated evidence; a changed cache hash or
GUID does not require duplicate snapshots or qualification of each asset choice.
Unknown generations or changed API/layout evidence still require qualification.
The reused snapshot retains its original source provenance; emitted mini-caches
are bound to the current target GUID, so recompile after a game update.

The original [qualification record](../../scripts/fixtures/npc-head/native-api-qualification.json)
identifies the audited compiler-profile, registration and Binds evidence.
The [poseable API record](../../scripts/fixtures/npc-head/native-poseable-api-qualification.json)
and [material API extension](../../scripts/fixtures/npc-batch-tests/heads/fresh-mid-native-api-qualification.json)
record later additions; the [hotfix record](../../scripts/fixtures/npc-batch-tests/quest/native-api-25168047-qualification.json)
records the 25168047 snapshot's provenance. These establish declaration and
binding evidence. Visual quality, animation, restoration and persistence still
need an observed game test of the authored mod.

### Probe head APIs before assembling the mod

After Doctor passes, compile a minimal probe against its pristine cache before
adding the NPC, dialog and quest content. Put only the required native types,
calls and property accesses in a small source. For a poseable head, this starting
probe at `head-probe/MyMod/HeadApiProbe.as` exercises the type, factory and
pose-copy/reset references; add any other native APIs the intended head path uses:

```angelscript
void GoreHeadApiProbe(AActor Owner, USkeletalMeshComponent Body)
{
    UPoseableMeshComponent Head =
        UPoseableMeshComponent::GetOrCreate(Owner, n"GoreHeadApiProbe");
    Head.CopyPoseFromSkeletalComponent(Body);
    Head.ResetBoneTransformByName(n"head");
}
```

Use the overlay route to exercise selective composition as well as compilation,
with fresh work and output paths outside the installation:

```powershell
New-Item -ItemType Directory -Force .gore-as-head-probe, out | Out-Null
gore as compile head-probe --overlays --backend standalone `
  --expect-base-sha256 <hash-from-doctor> --game "$GAME" `
  --work-dir .gore-as-head-probe -o out/head-probe.Cache `
  --mini out/head-probe.mini.Cache
```

A compiler success followed by a native declaration-membership refusal is a
composition/qualification failure. Preserve the exact target hash/GUID and
missing identity. On a known compatible generation, resolve snapshot selection
or update the matching toolkit package; do not use `--force` to bypass it.
Probe success establishes offline reference admission, not head-swap gameplay.

The high-level `dialog new-topic` scaffold uses the same compiler command in a
more specific shape. A new root or direct sub-topic is appended to the
**existing** shipped conversation module, so it is an edit with intentional new
symbols:

```powershell
gore as compile-module --backend standalone --op edit `
  --module Story.G1R.Conversation.Conversation_OC_STT_DIEGO `
  --rel-path Story/G1R/Conversation/Conversation_OC_STT_DIEGO.as `
  --source work/Conversation_OC_STT_DIEGO.as --work-dir work/.gore-as-work `
  --allow-new-symbols -o work/MyDialogMod.mini.Cache --game "$GAME"
```

`gore dialog stage` prints that command and writes a script-only bundle spec.
The current `dialog new-topic` root manifest contains no `dialog_topics` row and
therefore adds no generated UE4SS component: the same-module root uses the
game's native script discovery, while a direct sub-topic is reached through an
authored `Subdialog` call in a shipped parent. The private conversation base is
why neither shape should be turned into an isolated cross-module `--op add`.
Root scaffolds automatically choose an ordinary rank before the recognized
End/Back row, while sub-topics default to rank 0 and retain equal-rank slot
order. `--priority-rank` is the exact override; `-1` is intentionally forced and
is never selected automatically.

`gore dialog new-conversation` is the corresponding path when an NPC has no
root topic. It requires one exact, already-loaded per-NPC conversation-settings
module, keeps its shipped settings class intact, and appends the private root
and choices under `G1R::Conversation` in that module. Its staged command is
`--op edit --allow-new-symbols`; a missing or ambiguous settings anchor fails
closed instead of producing an unreferenced Add module. Further all-new levels
stay in the same source module, so new-to-new `Subdialog` references do not
depend on another mini-cache. The first choice defaults to rank 2;
`--priority-rank` overrides it exactly.

On BuildID `24878692`, the anchored Guard fixture opened automatically, spoke a
shipped line, rendered and accepted both wholly new choices in sequence, then
returned control. A separate new conversation Add module for the same Guard
compiled, packaged and deployed but was never discovered, which is why it is no
longer a staged product shape. A wholly new three-level tree also ran end to end
when a real `Say` separated consecutive nested menu transitions. Two actionless direct
`Subdialog` transitions soft-locked, so `dialog check` now refuses that narrow
shape. These bundles are script-only and need no UE4SS insertion.

On BuildID `24878692`, strict standalone compilation, mini-cache packaging and
deployment were followed by separate in-game observations of a selectable new
Diego root and direct sub-topic. The same bounded campaign also observed a
persisted inventory effect, explicit knowledge and quest state after save/load,
a new localization/Ogg/`Say` path whose loopback correlated `0.763` with the
source recording, and a manual rebuild of an existing four-child sub-menu. This
qualifies those exact fixtures on that build, not arbitrary game APIs, other
builds, or every possible conversation action. Earlier complete-cache tests
found that a raw FullGraph regeneration produced unusable main-menu input while
a manually selective hybrid booted and loaded a save. `gore as compile` now
publishes only the corresponding selective Add/Edit product. Its current live
fixture booted and loaded gameplay, rendered and selected a new same-module
root, and executed a new cross-module provider call from an edited shipped
automatic topic before returning control. The practical limits are maintained in
[AngelScript dialog authoring](dialog-authoring.md).

`compile-module` is the CLI equivalent of Mod Studio's Compile action, and it
uses the same `standalone-then-game` default. It resolves the embedded,
catalogued package automatically. If fallback launches the game compiler, GORE
restores the game install before returning the mini-cache; the command prints
the standalone failure that caused that fallback. Use `--backend standalone` to
forbid a game launch or `--backend game` to request the embedded game compiler
directly.

For compiler development only, `compile-module` retains a separate complete
override group:

```powershell
gore as compile-module ... --backend standalone `
  --development-standalone-sidecar .\gore-as-standalone-compiler.exe `
  --development-standalone-sidecar-sha256 <64-lowercase-hex> `
  --development-compiler-profile-manifest .\profile.json `
  --development-compiler-profile-root .\profile-root `
  --development-standalone-scratch-root .\scratch
```

All five development values are required together. They are not a product
package and cannot request `--generation-receipt`. The module receipt is a local
V1 build record produced only after the normal package was authenticated for
that run; unlike the full-graph V2 receipt, it does not itself carry the product
catalog identity. Normal users should not set the development overrides.

## Multi-module mini-caches

A mini-cache may carry more than one module. When a mod spans several
modules — a new provider module plus an edited shipped module that calls it —
compile them together and let `gore as compile` publish the mini next to the
complete cache. Put only their complete sources in `authored/` at canonical
Script-relative paths; no full export is needed:

```powershell
gore as compile authored --overlays -o out/full.Cache --mini out/MyMod.mini.Cache `
  --work-dir .gore-as-work --backend standalone --game "$GAME"
```

The mini holds only the authored Add/Edit modules, remapped to the pristine
cache like a `compile-module` output, so references between its own modules
resolve inside the one file. Reference it from a bundle spec with a single
entry: `op` is `edit` when any module edits a shipped one (existing modules are
replaced in place, new ones appended, as one unit), `add` when every module is
new; `module_name` names one of the carried modules. The command prints the
exact entry. `gore mod build`, `deploy` and the Manager compose such a mini as
one unit: in a loadout it is shadowed only as a whole, and a later mod that
re-targets some but not all of its modules is refused rather than partially
overridden. The low-level `gore as splice` appends by default and refuses a
mini that edits a shipped module; `gore as splice --upsert` replaces the
existing modules in place and appends the new ones, like deploy does.

Qualified in game on 2026-09-03: a two-module mini (new provider module plus
an edited Diego conversation whose new root topic takes its caption from the
provider) compiled standalone, deployed, showed the provider's text as a
selectable Diego topic at game start, and ended the conversation cleanly.

## Low-level splicing

For debugging or custom pipelines the individual stages remain available:

```powershell
# existing module — remap refs to the vanilla cache, then replace in place
gore as extract-remap regen.Cache <Module> vanilla.Cache -o mini.Cache
gore as replace       vanilla.Cache mini.Cache <Module>  -o modded.Cache

# new class/function-bearing module — carry only genuinely new symbol rows
gore as extract-remap regen.Cache <Module> vanilla.Cache `
                      --allow-new-symbols -o mini.Cache
gore as splice        vanilla.Cache mini.Cache -o modded.Cache

# pull a dependency-heavy module out with its full tail tables
gore as extract regen.Cache <Module> -o mini.Cache
```

`replace` and `splice` accept only a mini-cache already bound to the exact base
generation by `compile-module` or `extract-remap`. Raw
`-as-generate-precompiled-data` output carries a fresh GUID and is refused; remap
it against the intended pristine base first.

When a remap refuses a module — `unresolved`, or `ambiguous` — the message names the symbol but
not what differs about it. `GORE_AS_REMAP_DIAG=1` prints the regenerated and the base identity
side by side; they differ in exactly one field, and that field is the answer.

When a module's header says its class defaults were not authored,
`GORE_AS_DEFAULTS_DEBUG=1` prints the recovered method and the statements the recovery works on,
per class. `GORE_AS_MAX_DEFAULTS_DWORDS` and `GORE_AS_MAX_DEFAULT_STATEMENTS` lower the recovery
bounds when a fast emit matters more than the two machine-generated map tables.

`--allow-new-symbols` is deliberately opt-in. Existing references are still
mapped back to the vanilla cache; only rows for classes, functions, and names
that do not exist there are retained, with collision checks before deployment.
Mod Studio defaults it **on** for a new module and **off** for an edit; an
existing-module edit can enable it explicitly when it intentionally adds a class
or function.

Portable-identity construction remains bounded. The remapper permits at most
four times the composed input size, clamped to a 512 MiB hard ceiling; the
namespace-tolerant comparison work is separately limited to four times the
materialized identity footprint with the same ceiling. The larger ceiling lets
a legitimate allow-new edit hold both the pristine and regenerated identity
graphs without turning malformed input into unbounded memory or comparison work.

The remapped mini-cache is bound to the exact target cache GUID. Apply checks
that binding again and validates every executable reference and retained symbol
dependency against the effective base-plus-mini tables before it creates a game
backup, deploy record, or mutation lock. A mini built for an older game cache is
therefore refused rather than spliced. After a game update, compile or remap the
module again against the new pristine `PrecompiledScript_Shipping.Cache`; do not
reuse the previous mini-cache or copy its old GUID. Supported equivalent
generations reuse the authenticated native API evidence described above while
still targeting the new cache. A membership refusal on a known compatible
generation needs a toolkit correction, not `--force`, even if a generic error
hint suggests it. If the generation is genuinely unqualified, the error says
whether the global `--force` flag can override that check. A forced result may
be broken; see
[game updates](../reference/game-updates.md#forcing-compile-and-decompile-before-qualification).

These checks depend only on the cache contents, never on where the mod came
from. A GORE bundle, a community download, and a manually prepared package all
follow the same path and receive no origin-based exception.

A whole-cache replacement without additional script patches is validated as a
complete cache and then copied byte-for-byte. Its GUID belongs to that complete
replacement and does not have to match the currently installed cache. If other
script patches are layered on top, the replacement becomes their effective base
and the normal GUID and dependency checks apply before deployment.

Mod Manager plans all enabled script patches together before changing the game,
so internal number collisions between otherwise independent mods do not depend
on load order. Patches for different modules are combined. If several entries
target the same module, the later loadout entry is the displayed and deployed
winner. A complete raw cache is a base rather than a winner over compatible
module patches; those patches are applied on top of it in either order.

Prepared minis and raw extracted minis intentionally encode private
StaticNames operands differently. A prepared mini addresses a private row after
the pristine pool; a raw mini uses its compact local row. The loadout composer
now preserves that distinction, reuses matching names, and fails closed if a
prepared operand has no row. Do not rewrite those numeric operands by hand; the
wire-level contract is in [`gore-as/FORMAT.md`](../../crates/gore-as/FORMAT.md#staticnames-indices-in-raw-and-prepared-minis).

The retired `dialog_topics` registration adapter has one older live observation.
`gore mod build` now refuses that section. On 2026-08-18 the GORE-authored Viper fixture rendered
`[Gore probe] UI fixture`; `UE4SS.log`
recorded `ARMED`, `CHOICE_PASS`, and `RENDER_PASS` with `exact_count=1`. The run
used the PR #91-fixed app-local Core DLL. It was not a genuine third-party
AngelScript mod or a three-way script conflict, and no save was written during
the check. See the [Manager evidence boundary](mod-manager.md#evidence-boundary).

`compile` always leaves its result outside the installation. After a successful
standalone run the installation was never changed; after a successful game
fallback exact restoration has been proven. The live
`PrecompiledScript_Shipping.Cache` therefore remains the pristine cache these
commands use as `vanilla.Cache`.

## Verifying faithfulness

`bytediff` is the semantic byte-faithfulness oracle: it diffs a vanilla cache
against a regen (a re-compilation of decompiled source) per function, after
normalizing away build noise, and classifies each aligned function as
`IDENTICAL`, `BENIGN-DIFF`, or `SEMANTIC-DIFF`.

```powershell
gore as bytediff vanilla.Cache regen.Cache
gore as bytediff vanilla.Cache regen.Cache --module Dialog --verdict semantic
gore as bytediff vanilla.Cache regen.Cache --json scoreboard.json --fail-on-semantic
```

| Flag | Meaning |
|---|---|
| `--module <TEXT>` / `--func <TEXT>` | Substring filters on module or `module.Class::func`. |
| `--verdict identical\|benign\|semantic` | Filter output; repeatable. |
| `--show-benign` | List which normalizers fired for benign diffs. |
| `--context <N>` | Instruction window around each semantic divergence (default 6). |
| `--norm-slots` | Opt-in, fail-closed N2 slot-allocation normalization (default off). |
| `--no-norm-scope` | Disable the N5 `FScopeCycleCounter` profiler-scope strip (on by default). |
| `--no-norm-reguard` | Disable the N6 dominated boolean-cascade re-guard fold (on by default). |
| `--json <PATH>` | Machine-readable scoreboard (per-verdict counts + alignment loss). |
| `--fail-on-semantic` | Exit non-zero on any semantic diff — the CI gate. |

### What the module you are editing carries

Splicing recompiles the **whole** module from the emitted source, not only the function you
changed. So a module holding a function the decompiler does not reproduce exactly hands that
difference to the game as well, in code you never touched.

`emit` and `compile-module` say so before you get that far, using a table measured against the
shipped build:

```
warning: AI.States.FightAI.CombatState.CombatMoves carries 1 function the decompiler does not
reproduce as the same program. Splicing this module recompiles all of it, so those come out
changed as well.
```

The BuildID-24878692 warning table finds no semantic difference among aligned functions in
**7,311 of the 7,317 modules**. The remaining six contain 10 unresolved semantic differences;
they are not established to be harmless source-text variations. The oracle normalises
reference keys, slot numbers and other supported encoding differences before comparing.
Passing that comparison is useful evidence, but does not by itself prove behavior equivalence.

The later BuildID-25168047 hotfix round trip aligns all 164,724 functions with zero semantic
differences. The warning table above is keyed to the earlier BuildID-24878692 cache; its zero
behaviour-risk count refers only to its specific detected risk patterns and does not clear the
remaining differences on that older build.

The table is keyed by the generation the measurement was taken on. Point the tools at a build it
does not cover and no warning appears — that means *not measured*, not *byte-faithful*.

## Shipping a script mod

A compiled mini-cache is folded into a deployable bundle:

```json
{ "scripts": [ { "op": "add", "module_name": "MyModule", "mini_cache": "MyModule.cache" } ] }
```

See [Bundling & deploying](bundles.md). Deploy splices the mini-cache into
`PrecompiledScript_Shipping.Cache` in place, with a `*.gore-bak` backup.

## Related

- [AngelScript dialog authoring](dialog-authoring.md) — the compiled topic
  template, native same-module path, low-level legacy adapter, runtime evidence,
  safe test order, and practical limits.
- [Offline AngelScript default patching](angelscript-defaults.md) —
  `default-sites`, `patch-default`, `tag-map-sites`, `patch-tag-map`: changing
  proven scalar and GameplayTag-map defaults directly in the cache, without
  recompiling.
- [Mod Studio](mod-studio.md) — the no-code NPC and quest workflows built on top
  of this.
