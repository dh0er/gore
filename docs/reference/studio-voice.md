# Mod Studio voice authoring internals

This page records the implementation contracts, invariants, and native
transaction behavior behind Mod Studio's managed revision-3 Voice authoring.
It is not instructions: each section records the binding contract of a
workflow surface described in [Mod Studio](../guide/mod-studio.md).

GORE Mod Studio has a bounded managed revision-3 workflow for importing Voice
takes, resolving their exact installed archive targets, and building an
offline sealed replacement bundle. The complete path remains deliberately
narrow: it can replace proven existing archive members, but it does not deploy
the bundle or claim that the result has been heard and qualified in game.

The older Voice browser and replacement UI may supply standalone browsing and
preview tools while those jobs are rehosted. It is not another project path:
all authored Voice state belongs to the managed-R3 project.

## Voice production Work list V1

The projection retains at most 500 rows in the normal workspace, prioritizes
actionable work before completed decisions, reports the exact omitted count,
and offers search plus All/Needs action/Languages/Recordings/Decisions complete
filters. If Voice catalog loading is unavailable, known language work remains
visible with an explicit warning that recording work and counts were not
verified. Mismatched catalog project/revision checkpoints fail closed.

The host binds the workspace to the current managed root, project identity,
revision, and canonical-head lifecycle. Queue actions recheck the same catalog
objects, exact line/locale, derived next step, and head token before and after
asynchronous work. Same-revision head replacement, project/root replacement,
late reads, stale dirty text, and `requiresReopen` therefore cannot authorize
an old row. Mutations are globally single-flight and expose a visible disabled
reason while another action is unresolved.

## Project-text editor V1

One save replaces the complete locale/text map for that exact entry. The edit
is bound to the fixed head, project identity/revision/target, entity identity
and revision, and LocID. Only the project revision, localization revision,
localization texts, and newly introduced global authoring locales may change;
removing a text locale does not erase the project's global authoring-locale
history. The native transaction rejects no-ops, non-canonical/duplicate
locales, invalid or over-budget text, origin/identity drift, and every unrelated
candidate delta. The prepare route accepts no game root, publishes nothing,
fully reopens its immutable candidate, and repeats the fixed-head guard. The
serialized managed session alone may publish by guarded CAS and then fully
reopen the published project.

A locale attached to a `VoiceSlot` cannot be removed or blanked. If recorded
candidate takes exist for that slot, its transcript is also locked until the
take relationship can be changed through a separately supported workflow. The
current editor never guesses how to retime or rebind recorded audio. At least
one language remains on every project text. New and previously written text
cannot silently become blank. Stale selection, publication disagreement, or
uncertain session state fails closed and asks for refresh/reopen instead of
applying a guessed edit.

This editor changes only the managed project. It grants no topic, AngelScript,
NPC/speaker binding, build, deployment, runtime, game-installation, or save-game
authority. It is the first direct multilingual editing slice, not yet bulk
translation production, provenance/rebase, vanilla adoption, or a complete
conversation graph editor.

## Fresh-project dialog-line prerequisite V1

Before exact reuse can be saved, Studio reads only that exact current managed
localization through a separate read-only Store route. The route fully reopens
the fixed head before and after the read, verifies the project and entity
identity/revision, and returns at most 1,000 sorted locale previews with a
UTF-8-safe 512-byte bound per preview. The normal dialog shows friendly text
previews rather than entity IDs or localization identities, disambiguates equal
display names without exposing those identities, and enables only locales that
contain non-whitespace text. Publication repeats the exact read so a stale or
newly empty selection fails closed instead of creating a dangling Voice slot.

The optional speaker field is an author-facing label only. It does not resolve,
create, or bind an NPC, topic, runtime speaker, or vanilla identity.

The flow does not adopt records from the global extracted localization catalog.
That catalog is not sealed to the managed project and game generation, and its
speaker grouping is heuristic. It therefore cannot authorize a vanilla
identity, speaker, topic, or runtime relationship. A future vanilla-adoption
workflow needs a new sealed, generation-bound catalog and explicit provenance;
matching text or a familiar localization ID is insufficient.

Native code evaluates the change as one exact transaction, prepares an
immutable candidate in the managed Store, reopens it with full asset
verification, and checks that the fixed head has not changed after preparation
and response construction. The native route never publishes the fixed head.
Only the serialized managed session may publish the candidate through guarded
fixed-head compare-and-swap, repair journaling, and a full published reopen.

Neither create nor exact reuse accepts a game root or reads or writes the game
installation or a save. Its result is explicitly build `blocked`, runtime
`runtime_unqualified`, topic authority `not_granted`, and native publication
`not_supported`. The new `DialogLine` can be selected by subsequent managed
Voice authoring, but it creates no dialog topic, AngelScript, conditions,
effects, game registration, or playable conversation.

## Existing-line recording intent V1

The pure native transaction is bound to the exact head, project identity,
revision and target, line identity/revision, localization identity and LocID,
locale, and a deterministic collision-probed slot identity. It rejects an
existing edge, an occupied or locally referenced proposed ID, missing or blank
text, capacity/revision exhaustion, and every unrelated candidate delta. The
new entity has revision zero, the managed Voice-slot generator origin, matching
locale, unresolved target, no candidates, and no selection. Localization text
and revision, authoring locales, all unrelated entities, assets, game files,
and saves remain byte-exact.

The FFI route fully opens the published basis, independently reconstructs the
permitted delta, prepares and fully reopens an immutable candidate, and repeats
the fixed-head guard without publishing it. The serialized managed session
alone may publish by guarded CAS and fully reopen the result. A stale semantic
conflict is retryable after refresh; malformed evidence or uncertain
publication requires recovery or reopen. The inverse remains the separately
confirmed **Remove empty Voice setup** operation.

This project-only intent is build-blocked and runtime-unqualified. It grants no
audio, installed-target, game, save, deployment, playback, or recording
authority, and it does not claim the dialog is playable.

## Exact take import and Ogg safety

Native code performs semantic and capacity preflight before creating any Ogg
CAS object. It then performs two complete bounded, non-publishing preparations
of the source file. Only identical bytes and metadata allow the first accepted
payload to enter immutable AssetStore CAS. Missing, unsafe, oversized, invalid,
unavailable, or changing source files remain retryable. A later head race can
leave only a verified unreferenced CAS object; it cannot publish a partial
candidate.

Vorbis and Opus metadata can be retained as authored source evidence. The
current sealed bundle lowerer is qualified only for Vorbis, so selecting an
Opus take produces an explicit `selected_take_codec_unqualified` build blocker
instead of guessing compatibility.

## Atomic Voice folder import V1

Native code binds the source folder to retained no-follow directory identities,
reads only direct regular members, detects unsafe aliases and source changes,
and revalidates the complete scan, source bytes, root identities, exact head,
and reviewed source/plan seals before preparing a result. Item count, individual
and aggregate Ogg bytes, directory entries, transport, response, and
project-size-by-item work are bounded. Store, semantic game root, and source
folder must remain pairwise disjoint.

Planning is read-only. Preparation repeats every accepted source read, may add
verified immutable Ogg objects to CAS, applies the complete batch as one pure
transaction, and returns one fully reopened but unpublished checkpoint. The
serialized managed session alone may publish that checkpoint with one guarded
fixed-head compare-and-swap and full published reopen. Source, head, or reviewed
plan drift therefore produces no visible project-graph change and never exposes
a partial batch; a late race may leave only verified unreferenced CAS objects.

The workflow writes no game installation, save, deployment, or build output and
grants no target, build, runtime, or media-quality authority. Normal UI copy
shows friendly line, speaker, locale, and review state only. It does not expose
the selected absolute path or folder name, LocIDs, entity IDs, hashes, native
commands/codes, CAS paths, or raw native failures.

## Existing take review status

The status transaction binds the exact head, project/target, dialog line and
localization identity, locale, uniquely owned slot and unchanged slot revision,
take identity/revision, and expected old status. Only the project revision, the
chosen `VoiceTake` revision, and that take's status may change. Candidate order,
slot selection and revision, Ogg asset, codec facts, target evidence, dialog
text, localization, other takes, and all unrelated entities remain identical.
No-op, stale, selected-take demotion, malformed graph, or publication
disagreement fails closed.

The prepare-only native route performs full Store and asset verification,
reconstructs the permitted delta independently, fully reopens the immutable
candidate, repeats fixed-head race guards, and never publishes the head. Only
the serialized managed session may publish through guarded fixed-head CAS,
repair, and a full published reopen. This operation reads or writes neither a
game installation nor a save, accepts no source path, creates no build output,
and grants no media-quality, build, deployment, or runtime authority.

## Existing take selection

The native transaction binds the exact head, project, target, slot revision,
localization identity, and expected current selection. Selecting requires an
existing exact-project candidate with the same locale and `Approved` status.
Only `VoiceSlot.selected`, the slot revision, and the project revision may
change. Candidate order, every `VoiceTake`, all Ogg assets, target evidence,
other slots, dialog text, and localization remain byte-equivalent. The native
route opens the Store with full asset verification, prepares and fully reopens
an immutable candidate, and checks the fixed head after preparation and again
after constructing its response; it never publishes that head. The managed
session alone performs guarded fixed-head publication, repair, and full
published reopen.

No game root, caller-selected source path or external Ogg input, archive
access, build output, deployment, save file, or runtime authority participates
in selection. Existing managed Store Ogg assets are nevertheless reopened and
fully verified as part of the exact project basis and candidate checks. The
operation is therefore available even when no game installation is configured.
Its direct result remains `blocked`, `runtime_unqualified`, and native-
publication `not_supported`; readiness is always re-derived later by the
sealed build planner from the complete exact-current Voice graph.

## Remove a take from one line/language

One `VoiceTake` may be shared by more than one slot. The transaction therefore
removes only the requested slot edge. It retains a shared take byte-for-byte;
only a take with no remaining permitted local use loses its project entity.
Foreign-project references with the same opaque ID do not grant local
ownership. Kind-mismatched, unresolved, or unexpected same-project backlinks
fail closed instead of being guessed away.

The request binds the exact head, project/revision/target, line,
localization/LocID, locale, uniquely owned slot and revision, take and revision,
and current selection. Only the project revision, slot revision, candidate
list, optional selection, and—when this was the final use—the take entity may
change. Candidate order, all surviving entities, target evidence, and the
complete `AssetStore` remain exact. In particular, removing the final project
reference does **not** delete or promise to reclaim the immutable Ogg CAS blob.

The native route has no source path or game root, prepares and fully reopens an
immutable candidate, repeats fixed-head guards, and never publishes the head.
Only the serialized managed session may publish by guarded CAS and full reopen.
The action changes neither the game installation nor a save and grants no
build, deployment, media-quality, or runtime authority.

## Installed target resolution

Target resolution accepts only an existing safe line/locale/slot identity from
the fresh semantic content index. Native code:

- resolves the configured installation and reads the fixed installed
  executable through a bounded verified-file boundary;
- requires its byte length and SHA-256 to equal the managed project's exact
  game-generation anchor;
- resolves the corresponding locale archive through the deployment engine's
  authenticated pristine-source contract (or the live archive when there is no
  active managed deployment) and matches the localization ID;
- records the archive seal, exact member identity, and existing-member proof
  only for a unique match; and
- re-resolves the archive source and repeats the installed-executable
  generation check before returning so a hotfix, deployment-record change, or
  archive replacement during inspection cannot be accepted silently.

Zero matches remain unresolved. Two or more matches remain ambiguous with no
implicit selection. One match becomes a sealed `Replace` plus `Present`
target. The native route prepares and fully reopens an unpublished checkpoint;
the managed session alone owns guarded fixed-head CAS publication, repair, and
full published reopen. No game or save file is written.

## All-or-nothing sealed build

The build planner examines the exact current Voice graph before granting build
authority. A bundle is blocked when there are no Voice slots or when any slot
has an unresolved or ambiguous target, an additive/unqualified target, no
selected take, a selected take that is not Approved, or an unqualified codec.
Unsafe bundle metadata and bounded-evidence violations also fail closed. One
managed build is bounded to 1,024 Voice slots; a larger project receives one
structured global blocker rather than a partial build. The selected payloads
are additionally bounded to 256 MiB in aggregate (counting a reused take once
per planned replacement); exceeding that budget is another structured global
blocker rather than a later lowering failure. Slot blockers carry the owning
dialog-line label, localization ID, and locale for non-technical UI copy. A
blocked response creates no output directory.

For a ready plan, the native Store reopens the exact head with full asset
verification and reads each selected Ogg by its content-addressed asset
receipt. The lowerer accepts owned bytes rather than caller-controlled source
paths and produces only existing-member replacements. The resulting format-3
Voice manifest contains the exact executable-generation seal, archive/member
observations, and a byte length plus SHA-256 seal for every embedded
replacement payload. Duplicate case-insensitive targets are rejected. The
generic deployment reader keeps the committed format-1 and format-2 semantics
unchanged; only the managed generation-sealed path emits format 3.

The writer requires a real existing parent and a target directory that does
not exist. It writes into one unique owned sibling staging directory using
create-new semantics, verifies the complete canonical tree, manifests, Ogg
payloads, payload seals, and final bundle seal, and then atomically promotes
that same retained tree with no-replace semantics. It never clobbers an
existing target. Store, configured installation, and output roots must remain
disjoint, and recognizable game-layout ancestors are rejected even when the
caller supplies a different configured game root. The verified disk-tree seal
must equal the in-memory bundle seal, and root plus executable-generation
guards are repeated before promotion. The returned receipt states
`deployment_status: not_performed` and names the exact project revision/head
basis of the artifact.

The artifact is deliberately basis-snapshot-bound. Once the immutable Store
snapshot has been fully acquired, a later authoring-head advance does not
rewrite or relabel that artifact as the newest project. The valid receipt and
Studio result keep its original basis visible while the session simultaneously
requires reopen before another managed operation, so the author can rebuild
deliberately instead of receiving a misleading "latest" claim.

Fixed game/archive paths and every staging/output directory are traversed
through no-follow directory anchors. Created `voice`/`payload` children, files,
and failure cleanup stay relative to retained identity-owning handles, so a
concurrent junction or symlink substitution cannot redirect a build or
deployment outside its bound tree. Cleanup removes only the exact objects the
writer created and reports an explicit cleanup failure if absence cannot be
confirmed.

## Line-centered Studio workflow

Before import, the take dialog can open the currently selected regular local
`.ogg` file in the operating-system player. This is only an author preview of
the still-local source path. It neither reads a managed CAS path nor changes
review status, and it never claims codec, loudness, runtime, or in-game
qualification. Native Ogg validation remains mandatory before the project can
change.

Existing managed takes now have a separate **Preview** action in **Manage
takes**. The dialog refreshes the exact current content index and binds the
chosen line, localization, locale slot, take revision, and immutable audio
asset before asking native code to materialize anything. Native code reopens
that complete graph, fully verifies the selected CAS object's byte length and
SHA-256 plus its Ogg metadata, and copies only that object to a fixed
`preview.ogg` leaf in a new native-owned system-temporary capability. Before any
CAS work on Windows, native registration pins the managed Store read-only and
the temporary parent, atomically creates and retains a fresh non-overlapping
preview-root identity, rejects identity drift, and returns an opaque cleanup
token plus the private playback path; materialization and release must present
that token. Unsupported desktop platforms fail closed before creating a
capability. The response never exposes the private CAS path, and Studio never
renders or persists the temporary playback path.

Studio rehashes the materialized file before adopting it and plays supported
Ogg Vorbis or Ogg Opus takes in-app with Play/Pause, Replay, seeking, progress,
and Stop. Replacement, selection changes, refresh, Stop, and dialog close first
unload the native decoder handle and then ask native code to release the exact
token-owned leaf and its empty non-recursive root through the retained
capability. Cleanup never follows a later ambient path replacement. A failed
release retains the opaque token as an explicit retry obligation; the serialized
playback lifecycle must settle it before adopting another preview. Abrupt Studio
termination, or an exceptional registration failure whose exact deletion cannot
be proved, can leave an isolated temporary root behind; no unsafe startup sweep
guesses ownership, although a user or operating-system policy may remove the
orphan later. It cannot change the project, Store, game, save, build,
deployment, or runtime state. A stale graph can be refreshed;
Store/head/receipt uncertainty instead requires a verified project reopen.
Successful desktop playback does not qualify audible in-game behavior.

### Exact managed-take media QA V1

The pathless result reports codec metadata plus an integer duration as
`sample_frames / timebase_hz`. For Vorbis, a complete packet-by-packet PCM decode
establishes the timeline; validated initial PCM origin and final EOS trim yield
the playable duration. The Voice profile accepts mono or stereo and rejects
broader channel layouts before decode. Opus uses the normative 48 kHz clock
with granule origin, pre-skip, and EOS trim, but deliberately reports only
packet-and-timing structural assurance because no Opus PCM decode occurs in
this check. The operation evaluates neither loudness, clipping, performance,
subtitle fit, desktop audibility, build/deployment readiness, nor in-game
behavior. It writes no project, Store, game, save, build, or deployment state.

**Manage Voice takes** exposes this as an explicit per-take **Check media**
action, separate from Preview and never as an automatic scan. Studio presents a
friendly duration and either full-decode or structure/timing assurance without
showing paths, hashes, or technical identities. A result is cached only for the
exact project checkpoint, line, locale, take, and take revision; reload,
mutation, context change, or checkpoint drift discards it. Recoverable drift
offers a catalog reload, while uncertain Store/session authority requires the
managed project to be reopened. The visible result states that it is not an
audio-quality or in-game playback test.

**Manage takes** also closes the empty-slot dead end. Once a line/language
VoiceSlot has no candidate and no selection, a separately confirmed exact-head
transaction can remove that line/locale edge and its uniquely owned generated
VoiceSlot atomically. Resolved or ambiguous installed-target evidence is named
before confirmation because it is removed with the slot; the installed archive
itself is untouched. The LocalizationEntry, every VoiceTake, the complete
AssetStore and physical Ogg CAS, game files, and saves are preserved. Adding a
later take recreates the required slot through the existing bounded import
transaction.

## Publication and failure boundaries

The FFI routes do not replace the fixed project head. They return exact
candidate/build receipts only after reopening and checking their complete
contract. The managed session serializes operations and owns publication by
guarded fixed-head compare-and-swap plus repair journaling.

Retryable configuration, source, archive, and output errors keep the current
managed head usable. Basis-head conflict, Store invariant failure, malformed
native responses, or uncertain fixed-head publication require reopening the
project. Bundle-output publication is separate: if the atomic promotion may
have succeeded but final durability or identity cannot be confirmed, Studio
stops retries, preserves the exact native detail, and requires manual
inspection of that output without claiming that the managed project changed.
Every integer and response crossing the Dart boundary is bounded and checked
against the expected project/head identity and the exact build-ready or blocked
state derived from its caller-bound project graph.
