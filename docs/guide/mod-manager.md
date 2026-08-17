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

### Stable import identity and native result

Re-importing the same bound source resolves to its existing library entry:
changed bytes update it, while an unchanged tree is a no-op. Moving an
unchanged source — including moving between a folder and an equivalent ZIP —
rebinds that entry instead of creating a duplicate. The manager verifies a
candidate's current library tree before using its private identity hints; it
does not expose those hints as public entry metadata or use them as a deployment
fingerprint. For a source whose root is itself a UE4SS mod (`Scripts/main.lua`),
the content identity normalizes the source-name-derived storage wrapper while
the publication seal still binds its exact physical name. Ambiguous or
conflicting verified matches are refused before the
loadout is changed. One kernel lock serializes recovery, identity decisions,
publication, listing, and removal across cooperating processes. Windows owns a
byte range in the persistent `.gore-manager-library.lock` file; Unix instead
locks the retained canonical library-directory inode and creates no lock file.
Lock ownership, never the existence of a file, is authoritative, so a crash
releases it without leaving a stale blocker. Unix recovery and namespace
mutation stay relative to that retained directory descriptor even if the
configured pathname is renamed. Windows revalidates the root FileId before
path-based work and the publication journal revalidates payload seals. These
mechanisms coordinate GORE processes; neither platform claims an access-control
boundary against a same-user process deliberately bypassing the manager lock
(including Windows POSIX-style replacement operations).

Before its first publication rename, the manager durably journals the expected
staged seal and, for an update, the expected previous seal. Recovery first
re-seals the live object. An exact staged seal reconstructs promotion even when
cleanup already removed part of the previous tree. A remaining backup is sealed
only when needed to decide restore or quarantine. Each seal binds the
directory's filesystem identity, normalized payload-tree hash, and raw sidecar
hash. A mismatch remains in a dot-prefixed quarantine transaction instead of
being exposed as a live entry or guessed through during recovery.

Identity inspection is deliberately bounded. It validates every inspected
public sidecar and re-hashes candidates selected by an entry id, source hint, or
content hint. It does not globally re-hash readable hintless legacy entries or
entries whose valid content hint is a negative match.

The native `mgr_import` success object keeps the existing `entry` and adds two
top-level string fields:

```json
{
  "ok": true,
  "entry": { "id": "..." },
  "disposition": "created",
  "matched_by": "none"
}
```

`disposition` is exactly `created`, `updated`, or `unchanged`. `matched_by` is
exactly `none`, `source`, `content`, or `entry_id`. Consumers that only read the
existing `entry` object can ignore both additive fields.

Two identity refusals have dedicated native codes. Duplicate verified content
returns `IMPORT_DUPLICATE_AMBIGUOUS` with at most two ids in
`error.details.candidate_ids`; a split source/entry-id match and content match
returns `IMPORT_IDENTITY_CONFLICT`. For example, its exact detail shape is
`{"candidates":[{"id":"alpha","matched_by":["entry_id","source"]},{"id":"beta","matched_by":["content"]}]}`:
at most two deterministic witnesses sorted by id, with one or more roles in
`entry_id`, `source`, `content` order. The bounded witnesses are diagnostic, not
an exhaustive candidate list. Other import parsing, archive, hashing, safety,
and resource-limit failures retain the existing `IMPORT_FAILED` code.

Library publication and loadout registration remain two filesystem steps. A
successful library publication can therefore be followed by a loadout I/O
error; callers must refresh the authoritative library/loadout state. The
library lock does not claim a joint transaction or cross-process atomicity for
the loadout. Identity refusals happen before publication and loadout activation.

All authoritative Manager reads and loadout edits use one native Store
snapshot. It locks the loadout store before the library, validates every
non-dot library entry, and refuses uncertainty instead of silently presenting a
partial library. A valid loadout is reconciled to that snapshot: the first
known occurrence keeps its order and enabled state, duplicates and stale ids
are removed, and newly published ids are appended disabled in stable id order.
Corrupt, oversized, symlinked, or future-format loadouts are refused and left
untouched. Reconciliation writes only when those canonical bytes actually
change.

`gore doctor` remains advisory and read-only. Its deployment check takes only
the existing Library coordination lane, reads the loadout through the same
bounded no-follow stability checks, and applies the same strict reconciliation
in memory. It never creates the Store lock, repairs the loadout, or recovers a
library transaction. On Windows it joins an existing persistent Library lock
file but does not create one; missing coordination or recovery evidence is
reported without changing it. Cooperative Store writers include that same
Library root in their canonical physical lock set before they read or save, so
the Library guard keeps Doctor's projection stable while status consumes it.

Manager root locks coordinate GORE processes and are released by the kernel
after a crash. Store opens both the canonical loadout parent and Library root,
rejects one directory serving both roles, sorts their physical identities, and
locks in that global order. Unix locks each retained directory inode and
performs load/save relative to the Store handle; Windows uses the persistent
`.gore-manager-library.lock` direct child for every Manager root. Both platforms
revalidate retained and named root identities after acquisition. This is
cooperative serialization on a local filesystem, not an access-control boundary
against unrelated processes. Import and remove first
finish their separately locked library publication, release that lock, and then
take the Store snapshot for reconciliation; they intentionally remain two
explicit commits.

The app consumes this native snapshot directly and does not write it back while
refreshing. An explicit full-loadout replacement is serialized, but concurrent
full replacements of existing slots remain last-writer-wins. Reconciliation
still preserves every id currently present in the library; this compatibility
API does not claim CAS semantics or zero lost UI intent.

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

### Interrupted Manager changes

An interrupted Apply or Reset can leave an installation lock together with
recovery data. The Mod Manager app distinguishes three cases:

- If the Manager operation is still active, wait for it to finish and check
  again.
- If the native check can identify a clearly abandoned Manager operation, the
  app offers **Recover** after confirmation. The action is bound to that exact
  recorded operation and rechecks it before writing. Depending on where the old
  operation stopped, recovery can clear a pre-change lock, restore the recorded
  baseline, preserve an Apply that had already completed, or confirm a Reset
  that had already completed. Check the status again afterwards; use Reset only
  after recovery if you want to restore the pristine installation.
- If the lock belongs to script-build recovery or GORE cannot safely tell which
  operation created it, Manager does not change the installation and shows the
  recovery guidance instead.

Do not delete installation lock files by hand. This confirmation flow is a Mod
Manager app action; `gore mgr status` and `gore doctor` only report the next
step and do not perform this recovery.

The Mod Manager app's deployment-details dialog can expand **Recorded ownership
evidence** when the same validated deploy-record snapshot has the exact owner
`manager`. It groups the recorded paths as replaced live files, pristine
backups, additive pak/container files, UE4SS directories, and recovery
files/holders. The section is absent for no deployment, Studio ownership, and
unknown/future status. A Manager recovery record still shows the recovery group,
including the validated deploy-record path, even when it names no other path.

This is a bounded display projection, not another ownership or cleanup engine.
Each group is stable and platform-path-key deduplicated, contains at most 128
whole paths, admits at most 64 KiB of source-path UTF-8 bytes in total, and
omits any individual path over 4096 bytes rather than shortening it. `total`
counts unique validated candidates before those display caps, and the UI says
when fewer paths are shown. The projection contains no content hashes, private
identity values, existence checks, shared install-mutation lock, or new action.
It reports what the validated record says; it does not prove that any named
file or directory currently exists.

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
