# Running many mods (`gore mgr`)

`gore mod` deploys one bundle. `gore mgr` owns the multi-mod story: a library,
a load order, conflict analysis, and one composed deployment of the whole
enabled set. It is the CLI behind the
[Mod Manager](../../apps/mod-manager/README.md) app, and the two share the same
library and loadout files.

## Library and loadout

The library is the set of mods you have imported. The loadout is what is
enabled and in which order. Both default to a shared per-user location. Every
library/loadout subcommand accepts `--library <DIR>` and `--loadout <FILE>`;
when overriding them, pass both together so one library is never paired with an
unintended loadout. `reset` and token-bound `recover` work only from ownership
or recovery evidence in the game installation and accept neither option.

```powershell
gore mgr import C:\Downloads\SomeMod.zip   # folder, .zip, or a single game file
gore mgr list                             # library joined to loadout state
gore mgr remove <ID>                      # drop from library and target loadout
```

`remove` does not rewrite an already deployed game installation. If the entry
was active, run `apply` afterwards to deploy the updated target loadout.

`import` accepts built GORE bundles (a folder or zip with a root
`gore-mod.json`), foreign mod zips and folders, loose `_P.pak` files, IoStore
pairs (`.utoc`/`.ucas`) with an optional same-stem `.pak`, UE4SS Lua mod
folders, and raw `.lcache`, `.bank`, or `PrecompiledScript*.Cache` game-file
replacements. Extract `.7z` and `.rar` downloads before importing them.
Partitioned/multipart IoStore members are unsupported; an incomplete pair,
unknown content, unsafe path, or corrupt input is refused without publishing a
partial library entry or changing the loadout. `list` prints the entry ids the
other commands take.

Both GORE-produced foreign inputs and genuine downloads have crossed the real
install boundary. The 2026-08-18 campaign applied Nexus #244 Main Menu Replacer
— Remake, #512 Mainmenu Sleeper Enhanced, #269 Gothic UI Reposition, and Attack
Input V4. That proves those exact packages on one installation, not every
archive shape or a third-party AngelScript mod.

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
paks and IoStore triplets, `gm000` through `gm999` keeps Manager-owned targets
unique and the final numeric suffix gives Unreal patch priorities `1` through
`1000`. A later enabled entry therefore receives a strictly higher patch
priority. At most 1,000 entries may be enabled at once, matching both closed
ranges.

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

Reports conflicts among the **enabled** mods across localization, audio, voice
archives, texture/asset containers, item overrides (CDO), scripts, loose and
packed game-file claims, and raw-file replacements, and which mod the analyzed
loadout evidence marks as the intended winner for each recognized target.

The Manager app qualifies that result per component by heading the affected-
entry list with how far it can be trusted: "the full list", "there may be more",
"hints, not proof", or — when nothing can be listed — "GORE cannot tell what
this changes". The grade is worded as part of that heading rather than as a
standalone badge, because a lone adjective on a row reads as a verdict on the
mod. These grades describe target knowledge only; even a full list does not
prove the game's runtime priority. If any enabled component is not Complete, a zero-result
analysis is therefore shown as "no conflicts found" with an
incomplete-knowledge warning, never as proof that the loadout is conflict-free.
The grades themselves are shown while **Advanced details** is on in Settings;
the incomplete-knowledge warning always appears when it applies.

The CLI names only the three incomplete grades, under their wire names. A
zero-result analysis prints `no recognized conflicts`, and whenever any enabled
footprint is partial, advisory, or opaque it adds
`warning: conflict analysis is incomplete for enabled components (partial=…,
advisory=…, opaque=…)` after that line. `exact` is never printed, and
`gore mgr analyze` has no `--json`.

The same view spells out the intended order — mods further down the list
override the ones above them — while **Advanced details** is on. That order predicts winners only
where the analyzer has the corresponding evidence. Numeric container priority
does not make opaque targets or interactions outside that evidence predictable.

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

A soft localization clash has been watched resolve in game in both order
directions. An earlier #244/#512 main-menu run with equal-priority names let
`gm000` win both directions and exposed the ordering bug. After migration to
numeric priorities, the 2026-08-18 campaign confirmed the corrected behavior:
`#244 -> #512` showed the Sleeper/Gothic-II menu and `#512 -> #244` showed the
red Remake artwork. A GORE-authored AngelScript fixture also composed and
rendered live, but no third-party AngelScript mod or three-way script conflict
has been qualified.

## Apply

```powershell
gore mgr preflight --game "$GAME" # read-only setup and recovery evidence
gore mgr apply  --game "$GAME"    # compose the enabled loadout into one deployment
gore mgr status --game "$GAME"    # is the install in sync with the target loadout?
gore mgr reset  --game "$GAME"    # undeploy everything the manager has active
```

`apply` is **declarative**: it recomputes the full modded state from a pristine
base and deploys the whole enabled set, backups first. It is not an incremental
patch on top of whatever happened to be installed, which is what makes
disabling a mod in the middle of the order safe.

An older Manager deployment that owns containers but lacks the numeric-priority
schema marker is reported as changes pending even when its loadout is unchanged.
The next Apply migrates only its receipt- and hash-owned old names; Reset then
cleans the new names through the same ownership evidence.

`reset` restores the pristine install only when the validated deployment owner
is Manager. It rechecks that ownership in the protected mutation path and
refuses a Studio-owned deployment without changing it.

`preflight` is read-only. It returns seven fixed-order checks for the game root,
install, loadout, deployment, install mutation, UE4SS, and write access. Its
`--json` envelope is:

```text
{ "ok": true, "preflight": { "format": 1, "checks": [<seven check objects>] } }
```

Only a check whose action is `recover_manager_mutation` carries an
`action_token`; that token names one exact abandoned Manager operation and is
not general cleanup authority.

`status --json` emits `{ "ok": true, "status": { ... } }` with the full native
status report. Its optional `manager_owned` groups are the same bounded,
record-derived path evidence described below; absence is not an empty ownership
claim.

### Interrupted Manager changes

An interrupted Apply or Reset can leave an installation lock together with
recovery data. Preflight and the Mod Manager app distinguish three cases:

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

Do not delete installation lock files by hand. The app offers the confirmation
flow directly. The equivalent CLI sequence copies the exact `action_token` from
preflight and supplies it as the expected guard:

```powershell
gore mgr preflight --game "$GAME" --json
gore mgr recover --game "$GAME" --expected-guard-id <TOKEN>
```

`recover` probes before writing and refuses missing, still-active,
compiler-owned, ambiguous, invalid, or changed recovery state. The guard token
is required, is bounded to 512 bytes, and must still match exactly. By default
the CLI shows the planned recovery and requires an exact `y`/`N` answer;
`--yes` is the explicit non-interactive approval. `--json` requires `--yes` and
returns `{ "ok": <BOOL>, "outcome": "..." }` after the authoritative recovery
result. Busy, compiler-recovery, and inspection-failure outcomes set `ok` to
`false` and exit nonzero so automation cannot continue as if recovery ran.
`gore mgr preflight` and `gore doctor` remain strictly read-only.
`gore mgr status` never performs install recovery, but opening its authoritative
Store snapshot may persist the same valid loadout reconciliation as `list` and
`analyze`.

The Mod Manager app's status-details dialog can expand **Files GORE manages**
when the same validated deploy-record snapshot has the exact owner `manager` and
**Advanced details** is on in Settings. It groups the recorded paths as replaced
game files, backups of the originals, added mod files, UE4SS directories, and
repair files. The section is absent for no deployment, Studio ownership, and
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

Two manually observed campaigns have gone past that line on one maintainer's
installation. The first, on 2026-08-07 against Steam BuildID 24539464 with
`gore` built from commit `90940340`, imported, enabled, analyzed, applied,
launched, reordered, re-applied, and reset GORE-produced inputs. The two-mod
localization conflict resolved on screen the way `analyze` named it in both
directions; a foreign `gore asset pack` triplet applied; and Reset left the
installation pristine on disk — `~mods\` empty, no `*.gore-bak`, deploy record,
or mutation lock, and every rewritten file restored to its original byte count.
The detailed counts remain in
[bundles](bundles.md#what-is-proven-and-by-what).

The second campaign, on 2026-08-18, used a packaged Manager built from the PR
#90 merge and four genuine Nexus mods: #244 Main Menu Replacer — Remake, #512
Mainmenu Sleeper Enhanced, #269 Gothic UI Reposition, and Attack Input V4. It
observed the corrected numeric container priorities in both #244/#512 order
directions, loaded a new game and an existing save, and exercised the tested
enable, disable, reorder, Apply, and Reset paths. It also rendered the
GORE-authored Viper choice `[Gore probe] UI fixture`; `UE4SS.log` recorded
`ARMED`, `CHOICE_PASS`, and `RENDER_PASS` with `exact_count=1`. That script
probe used the PR #91-fixed app-local Core DLL, so it qualifies that GORE
fixture and composition path only — not a genuine third-party AngelScript mod
or a three-way script conflict. #269 was disabled for the probe after its own
UE4SS Lua loop had crashed while calling `FindAllOf` off the game thread; no
GORE or AngelScript frame was present, and no save was written during the
probe.

Postflight restored the captured loadout byte-for-byte, removed every temporary
campaign entry and game-tree payload, restored the original signed Core DLL,
and reported the original four-mod deployment in sync. It did not reset that
user baseline to a pristine install. These remain manual observations by one
person on one installation; nothing in the toolkit observes the screen.

The separate clean-Windows portable, installer, recovery, Reset, and uninstall
acceptance pass remains open. The real-install campaign is not evidence for
that packaging boundary.

Neither the offline checks nor these sessions grant any authority to modify a
real installation, launch the game, or read or mutate a save; those steps
require separate qualified safety gates.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--library <DIR>` | all except `reset`, `recover` | Library dir. Default: the shared per-user library. Supply it together with `--loadout`. |
| `--loadout <FILE>` | all except `reset`, `recover` | Loadout file. Default: the shared per-user loadout. Supply it together with `--library`. |
| `--game <PATH>` | `apply`, `status`, `reset`, `preflight`, `recover` | Game root containing `G1R\`. Falls back to the configured path. |
| `--expected-guard-id <TOKEN>` | `recover` | Exact `action_token` from the current abandoned-Manager preflight check; required, max 512 bytes. |
| `--yes` | `recover` | Approve the exact token-bound recovery without the interactive `y`/`N` prompt; required with `--json`. |
| `--json` | `preflight`, `recover`, `status` | Emit one machine-readable result document. |

## Related

- [Bundling & deploying](bundles.md) — producing the bundles this manages.
- [Mod Manager app](../../apps/mod-manager/README.md) — the same operations with
  drag-to-reorder and a conflict view.
