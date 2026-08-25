# Scripts (AngelScript) — experimental

The game's compiled AngelScript lives in a precompiled cache,
`$GAME\G1R\Script\PrecompiledScript_Shipping.Cache`. `gore as` reads that cache,
turns modules back into readable AngelScript, compiles complete source graphs or
individual modules, and splices edited modules back in.

This is reverse-engineering-stage tooling. It works, and the complete
new-dialog path has been validated in game, but treat every step as
experimental and keep backups.

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

Decompilation and emit resolve native-call arities from a `Binds.Cache` placed
next to the input cache, or from the path in `GORE_AS_BINDS`.

## Recompiling: standalone first, game fallback

GORE ships its standalone compiler as an internal part of GORE CLI and Mod
Studio. The normal product commands authenticate the sidecar and profile, then
check the installed game's cache format and ordered AngelScript API. Users do
not provide sidecar paths or hashes.

This check is deliberately independent of Steam or GOG packaging. Whole-file
EXE/cache hashes, Steam build numbers and depot metadata document the build used
for qualification, but they are not runtime locks. A differently packed AMD64
EXE remains usable when its Shipping format and complete Binds API match.

The default policy is `standalone-then-game`: GORE tries the qualified
standalone compiler first. If the package is absent, the selected game's cache
format or API is incompatible, or the standalone result is rejected, the reason is retained and
shown before GORE uses the game's embedded compiler as a fallback. That fallback
launches the shipping executable with **`-as-generate-precompiled-data`** and
temporarily stages loose `.as` files under `<install>\G1R\Script\`.

`gore as compile` takes one complete authoritative source tree and always
publishes a separate complete cache. It never installs that output implicitly.

```powershell
# dump the vanilla modules as an editable tree
gore as emit-all "$GAME\G1R\Script\PrecompiledScript_Shipping.Cache" out_as
# …edit modules in out_as…

# compile to a new no-clobber cache file
New-Item -ItemType Directory -Force .gore-as-work | Out-Null
gore as compile out_as -o regen.Cache --work-dir .gore-as-work --game "$GAME"
```

The install is resolved from `--game`, else the configured game path, else
Steam auto-detect. The source tree, output, and workspace are required. The
output must be outside the game installation and outside the workspace; it is
published without overwriting an existing file.

Choose the policy explicitly when needed:

```powershell
# Never launch or modify the game; fail if no compatible standalone package is available.
gore as compile out_as -o regen.Cache --work-dir .gore-as-work `
  --backend standalone --game "$GAME"

# Deliberately use only the game's embedded compiler.
gore as compile out_as -o regen.Cache --work-dir .gore-as-work `
  --backend game --game "$GAME"
```

`--backend standalone-then-game` is the default. `standalone` and `game` never
fall back silently.

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

### Compiler diagnostics

When the game fallback runs on Windows, compile automatically attempts an
embedded, temporary x86-64 diagnostics hook. The selected AMD64 executable must
have exactly one raw masked match for both the AngelScript callback and the
manager's structured ClassGenerator-diagnostic boundary; their sparse
`asSMessageInfo` and `FString`/`FDiagnostic` structure fingerprints must both
verify. Errors are then printed like a normal compiler:

```
file:line:column: error: message
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
the embedded and sibling release helpers are SHA-256 verified.

Archived 1.0.0–1.0.4a executables pass the same offline two-boundary signature
and structure audit. Runtime results for the pinned standalone-compiler target
are recorded with its differential qualification evidence.

## The normal authoring workflow: one module

Do not ship a whole regenerated cache. Compile one authored module and splice
it into the vanilla cache. The high-level command performs the entire
emit → overlay → compile → extract → remap chain and returns a deployable
mini-cache:

```powershell
gore as compile-module --op add --module MyMod.Dialog `
  --rel-path MyMod/Dialog.as --source Dialog.as --work-dir .gore-as-work `
  --allow-new-symbols -o MyMod.Dialog.mini.Cache --game "$GAME"
```

| Flag | Meaning |
|---|---|
| `--op add\|edit` | `add` for a new module, `edit` for an existing one. |
| `--module <NAME>` | Expected module name. For `add`, the compiler-detected name is reported and used. |
| `--rel-path <PATH>` | Safe path of the authored file relative to the game's `Script\` tree. |
| `--source <FILE>` | The authored `.as` file to overlay. |
| `--work-dir <DIR>` | Existing workspace outside the game installation (emitted tree + intermediate cache). |
| `--allow-new-symbols` | Retain minimal rows for classes/functions/names absent from the pristine cache. |
| `-o, --out <PATH>` | The remapped 1-module mini-cache. |

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

`--allow-new-symbols` is deliberately opt-in. Existing references are still
mapped back to the vanilla cache; only rows for classes, functions, and names
that do not exist there are retained, with collision checks before deployment.
Mod Studio defaults it **on** for a new module and **off** for an edit; an
existing-module edit can enable it explicitly when it intentionally adds a class
or function.

The remapped mini-cache is bound to the exact target cache GUID. Apply checks
that binding again and validates every executable reference and retained symbol
dependency against the effective base-plus-mini tables before it creates a game
backup, deploy record, or mutation lock. A mini built for an older game cache is
therefore refused rather than spliced. After a game update, compile or remap the
module again against the new pristine `PrecompiledScript_Shipping.Cache`; do not
reuse the previous mini-cache or copy its old GUID.

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

This composition path has one current live observation. On 2026-08-18 the
GORE-authored Viper fixture rendered `[Gore probe] UI fixture`; `UE4SS.log`
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

## Shipping a script mod

A compiled mini-cache is folded into a deployable bundle:

```json
{ "scripts": [ { "op": "add", "module_name": "MyModule", "mini_cache": "MyModule.cache" } ] }
```

See [Bundling & deploying](bundles.md). Deploy splices the mini-cache into
`PrecompiledScript_Shipping.Cache` in place, with a `*.gore-bak` backup.

## Related

- [AngelScript dialog authoring](dialog-authoring.md) — the compiled topic
  template, runtime evidence, safe test order, and the boundary between a
  renderable new class and automatic topic discovery.
- [Offline AngelScript default patching](angelscript-defaults.md) —
  `default-sites`, `patch-default`, `tag-map-sites`, `patch-tag-map`: changing
  proven scalar and GameplayTag-map defaults directly in the cache, without
  recompiling.
- [Mod Studio](mod-studio.md) — the no-code NPC and quest workflows built on top
  of this.
