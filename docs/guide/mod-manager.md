# Running many mods (`gore mgr`)

`gore mod` deploys one bundle. `gore mgr` owns the multi-mod story: a library,
a load order, conflict analysis, and one composed deployment of the whole
enabled set. It is the CLI behind the
[Mod Manager](../../apps/mod-manager/README.md) app, and the two share the same
library and loadout files.

## Library and loadout

The library is the set of mods you have imported. The loadout is what is
enabled and in which order. Both default to a shared per-user location; every
subcommand accepts `--library <DIR>` and `--loadout <FILE>` to work on a
different set.

```powershell
gore mgr import C:\Downloads\SomeMod.zip   # folder, .zip, or a single game file
gore mgr list                             # library joined to loadout state
gore mgr remove <ID>                      # drop from library and loadout
```

`import` accepts built GORE bundles (a folder or zip with a root
`gore-mod.json`), foreign mod zips and folders, loose `_P.pak` files, IoStore
triplets (`.utoc`/`.ucas`/`.pak`), UE4SS Lua mod folders, and raw game-file
replacements. `list` prints the entry ids the other commands take.

The foreign path has been walked once end to end against a real install: on
BuildID 24539464 a triplet produced by `gore asset pack` — not a GORE bundle,
with no `gore-mod.json` anywhere in it — was imported, classified as a foreign
triplet, and applied with a load-order filename prefix.

### GORE bundle format gate

Recognizing a root `gore-mod.json` commits import to the closed
[bundle-format contract](bundles.md#bundle-format-and-reader-contract): format
1 must not contain `pak_file_patch`, while format 2 must contain at least one.
The manager rejects either mismatch and every unknown format before interpreting
component payloads or publishing the library entry. It does not migrate the
manifest, drop a component, or retry the rejected GORE bundle as a foreign mod.

## Order and enablement

```powershell
gore mgr enable  <ID>
gore mgr disable <ID>
gore mgr order   <ID> <POS>    # 0 is composed first
```

Position `0` is composed first; later entries are selected or reported as the
intended conflict winners. `<POS>` is clamped to the last slot. For additive
paks, this ordering controls the filenames the manager writes; it is not by
itself proof of Unreal's runtime mount priority. At most 1,000 entries may be
enabled at once, matching the closed `gm000` through `gm999` filename range.

The direction reads backwards the first time, so it is worth saying twice, the
way the command's own help says it: `0 = mounts first, loses conflicts`. Going
first is going early, and whatever comes later overwrites you. That has been
watched happen once, on BuildID 24539464. With two bundles editing the same
localization id, moving the winner to position `0` flipped `analyze`'s
prediction, and after a re-apply the game showed the other mod's text — same two
mods, same id, nothing rebuilt.

## Conflicts

```powershell
gore mgr analyze
```

Reports conflicts among the **enabled** mods across localization, audio,
texture/asset, item overrides (CDO), scripts, and raw-file replacements, and
which mod the analyzed loadout evidence marks as the intended winner for each
recognized target.

The Manager app qualifies that result per component with derived footprint
coverage. **Exact** means the component metadata gives conflict analysis a
complete target list. **Partial** means the listed targets are known but the
component can affect more. **Advisory** targets are useful hints rather than an
exhaustive inventory, and **Opaque** means the targets are unknown. These grades
describe target knowledge only; even Exact does not prove the game's runtime
priority. If any enabled component is not Exact, a zero-result analysis is
therefore shown as "no recognized conflicts" with an incomplete-knowledge
warning, never as proof that the loadout is conflict-free.

The same view spells out the intended order: low priority is listed first and
later mods have higher intended priority. That order predicts winners only
where the analyzer has the corresponding evidence; it does not turn container
filename order into a runtime guarantee.

Localization is reported **per language**: the target is `<id>|<language>`, so
two mods editing one id collide once for every language key they both write, and
each line names its own winner. That naming has been checked against the running
game once. On BuildID 24539464 two bundles editing the same id were imported,
enabled and applied; `analyze` reported the clash per language and named a
winner; the game showed that mod's text. Reordering flipped both the prediction
and the screen, as [above](#order-and-enablement).

Voice collisions on `(archive, archive_path)` are case-insensitive, soft, and
order-dependent: the later mod wins while retaining the winning spelling and
operation.

Two pak components claiming the same game path are also a soft, ordered
conflict: the manager reports the later claimant as the intended winner while
retaining both paks. A pak claim against an in-place `files` claim is only
advisory because their runtime precedence is not established by conflict
analysis.

Script mods that do not declare their CDO targets are treated as opaque — the
manager cannot prove what they touch, so it cannot rule out a conflict with
them.

Everything else this section describes is still the manager's stated intent
rather than an observed outcome. Exactly one conflict kind has been watched
resolve in game: a soft localization clash between two bundles, in both order
directions. Texture-versus-texture container precedence, script splices, and
three-way conflicts have never been checked against a running game at all.

## Apply

```powershell
gore mgr apply  --game "$GAME"    # compose the enabled loadout into one deployment
gore mgr status --game "$GAME"    # is the install in sync with the target loadout?
gore mgr reset  --game "$GAME"    # undeploy everything the manager has active
```

`apply` is **declarative**: it recomputes the full modded state from a pristine
base and deploys the whole enabled set, backups first. It is not an incremental
patch on top of whatever happened to be installed, which is what makes
disabling a mod in the middle of the order safe.

`reset` restores the pristine install.

### Evidence boundary

Applying, reordering, and resetting against an offline synthetic game root can
prove deterministic files, owned cleanup, and receipt state. Those checks do
not prove that Unreal mounts one pak ahead of another, that the game reads the
selected bytes, or that any runtime behavior changed.

One session has gone past that line. On 2026-08-07, against Gothic 1 Remake at
Steam BuildID 24539464 with `gore` built from commit `90940340`, a real install
was imported into, enabled, analyzed, applied, launched, reordered, re-applied
and reset by hand. The two-mod localization conflict resolved on screen the way
`analyze` had named it, in both order directions; a foreign `gore asset pack`
triplet imported and applied; and `reset` left the install pristine, verified on
disk afterwards — `~mods\` empty, not one `*.gore-bak` anywhere, no deploy
record, no mutation lock, and every rewritten file back at its original byte
count, the numbers being the ones in
[bundles](bundles.md#what-is-proven-and-by-what).

That is one person, one install, one build, one sitting. It moves import, apply,
reorder, reset and the localization conflict from "deterministic offline" to
"seen working once on the real game", and it leaves every other conflict kind
where it was. Nothing re-runs it, and nothing in the toolkit observes the
screen.

Neither the offline checks nor that session grants any authority to modify a
real installation, launch the game, or read or mutate a save; those steps
require separate qualified safety gates.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--library <DIR>` | all except `reset` | Library dir. Default: the shared per-user library. |
| `--loadout <FILE>` | all except `reset` | Loadout file. Default: the shared per-user loadout. |
| `--game <PATH>` | `apply`, `status`, `reset` | Game root containing `G1R\`. Falls back to the configured path. |

## Related

- [Bundling & deploying](bundles.md) — producing the bundles this manages.
- [Mod Manager app](../../apps/mod-manager/README.md) — the same operations with
  drag-to-reorder and a conflict view.
