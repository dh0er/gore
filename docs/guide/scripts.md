# Scripts (AngelScript) — experimental

The game's compiled AngelScript lives in a precompiled cache,
`$GAME\G1R\Script\PrecompiledScript_Shipping.Cache`. `gore as` reads that cache,
turns modules back into readable AngelScript, drives the game's own compiler,
and splices edited modules back in.

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

## Recompiling: the game is the compiler

There is no standalone AngelScript compiler for this game. The shipping
executable **is** the compiler: it accepts
**`-as-generate-precompiled-data`**, which makes it read the loose `.as` files
under `<install>\G1R\Script\`, compile them, and overwrite
`PrecompiledScript_Shipping.Cache` in that same folder.

`gore as compile` wraps that flag as an ordinary compiler. Give it a source
tree and optionally an output path; it does the backup, staging into `Script\`,
launching the game, and restoring the install itself.

```powershell
# dump the vanilla modules as an editable tree
gore as emit-all "$GAME\G1R\Script\PrecompiledScript_Shipping.Cache" out_as
# …edit modules in out_as…

# compile to a cache file, leaving the install untouched
gore as compile out_as -o regen.Cache --game "$GAME"

# …or install the fresh cache in place (previous one saved to *.gore-bak)
gore as compile out_as --game "$GAME"

# with no source tree: recompile whatever .as already sit in Script\
gore as compile --game "$GAME"
```

The install is resolved from `--game`, else the configured game path, else
Steam auto-detect. `--no-backup` skips the `*.gore-bak` when installing in
place.

### Safety rules around compilation

These are enforced, not advisory:

- Before any staging, compile **fails closed** if the shipping game process is
  running, if process inspection is unavailable, or if a prior compile/recovery
  artifact exists.
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

On Windows, compile automatically attempts an embedded, temporary x86-64
diagnostics hook. When the selected AMD64 executable has exactly one raw masked
callback match and its sparse `asSMessageInfo` structure fingerprint verifies,
errors are printed like a normal compiler:

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

The check reports the executable's SHA-256, the raw match count, the matched
RVA(s), and callback-structure verification. An explicitly trusted helper
override is available through `--diagnostics-hook DLL` or `GORE_AS_DIAGNOSTICS_HOOK`;
the embedded and sibling release helpers are SHA-256 verified.

The currently embedded helper has passed both a full-tree positive compile and
an intentional compiler-error run on the installed 1.0.3 executable. Archived
1.0.0–1.0.3 executables pass the same offline signature and structure audit;
only installed 1.0.3 has been runtime-injected.

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
| `--work-dir <DIR>` | Persistent compiler workspace (emitted tree + intermediate regen cache). |
| `--allow-new-symbols` | Retain minimal rows for classes/functions/names absent from the pristine cache. |
| `-o, --out <PATH>` | The remapped 1-module mini-cache. |

`compile-module` is the CLI equivalent of Mod Studio's Compile action, and it
restores the game install after the compiler run.

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

The `-o` form of `compile` leaves the install exactly as it was, so the live
`PrecompiledScript_Shipping.Cache` is still the pristine cache these commands
use as `vanilla.Cache`.

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
