# Managed revision-3 Voice authoring

GORE Mod Studio has a bounded managed revision-3 workflow for importing Voice
takes, resolving their exact installed archive targets, and building an
offline sealed replacement bundle. The complete path remains deliberately
narrow: it can replace proven existing archive members, but it does not deploy
the bundle or claim that the result has been heard and qualified in game.

The older compatibility-project Voice editor and deployment engine remain a
separate path. Managed-R3 content is never silently copied into that project or
treated as deployed compatibility state.

## Visible Studio workflow

With a managed revision-3 project open, **Localization & Voice** is a direct
search/list/editor workspace rather than a capability-card landing page. It
keeps project text, language editing, the guided line action, and the bounded
Voice authoring actions in steps 1–5 together. Text/line and take-selection
flows are project-only. Import and installed-target resolution additionally
need a Gothic 1 Remake installation configured in Settings. Bundle construction
remains a separate **Build & Release** action and needs that installation too.

1. **Project texts** searches exact project-owned localization entries and opens
   their complete multilingual text maps inline. Its bounded edit contract is
   described below.
2. The **guided dialog-line V1** flow lets a fresh project create the minimum
   managed line/localization structure needed by the Voice tools. Its narrow
   contract and limits are described below.
3. **Add Voice take** imports one real local Ogg for an existing dialog line and
   locale. The search-first wizard hides technical identities, retains
   alternate takes, supports Draft/Recorded/Reviewed/Approved status, and lets
   only an Approved take become selected.
4. **Manage Voice takes** searches existing dialog lines and lets the author
   select one already retained Approved candidate for an existing locale slot,
   or explicitly clear its current selection. It imports, removes, and changes
   no take or media asset, and it needs no game path.
5. **Resolve Voice target** inspects the exact installed locale archive for one
   existing structurally intact Voice slot. It records zero, one, or multiple
   matching members as unresolved, resolved, or ambiguous. It never chooses an
   ambiguous match implicitly.
6. Under **Build & Release**, **Build Voice bundle** evaluates every current
   Voice slot and either shows all structured blockers without creating output,
   or writes one sealed voice-only bundle into a brand-new folder selected by
   the author. This is an offline build; the dialog has no deployment action.

All authoring actions reload or bind the exact current project checkpoint. A
stale dialog, changed project identity, changed canonical head, or session
requiring reopen fails closed. After a successful authoring publication, Home
refreshes to the new managed project revision and head.

The normal UI never asks for entity IDs, archive paths, member names, hashes,
CAS paths, or bundle internals. A full Voice slot remains eligible for target
resolution even though its candidate-capacity limit correctly prevents adding
another take.

## Project-text editor V1

The managed workspace discovers only intact project-owned
`LocalizationEntry` entities with `origin.type == new`. A friendly opaque list
key keeps entity IDs and LocIDs out of normal presentation. Selecting a row
performs a separate exact-current Store read and returns the complete bounded
text for every locale, not the older 512-byte reuse preview. Shared
`DialogLine` backlinks and speaker labels are shown so the author can see the
scope of a text change before saving.

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

The guided V1 flow can create one new project-owned `LocalizationEntry` and one
new `DialogLine`, with an optional empty unresolved `VoiceSlot` for one locale.
Alternatively, it can bind the new line to one exact existing, currently unused
managed `LocalizationEntry`. Reuse is tied to the exact entity ID, entity
revision, localization identity, project revision, target, and fixed head; the
existing localization is preserved byte-for-byte. This is deliberately a
small prerequisite flow, not a complete dialog or localization editor.

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

## Exact take import and Ogg safety

Take import operates only on an existing exact-project `DialogLine` and its
existing `LocalizationEntry`. It creates a locale `VoiceSlot` when necessary,
or appends a new `VoiceTake` to the exact existing slot. It preserves dialog
text, keeps alternate candidates, and changes the selection only when the new
take is Approved and any replacement was explicitly confirmed.

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

## Existing take selection

Take selection is a project-only transaction over one exact existing
`DialogLine`, locale, and uniquely owned `VoiceSlot`. The friendly dialog shows
candidates in their authored order, distinguishes duplicate display names,
marks the current choice, and disables non-Approved candidates. It never
chooses the first take implicitly. Saving is available only for a real change;
clearing an existing choice is explicit and warns that Voice bundle builds will
remain blocked until another Approved take is selected.

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

## Deliberate remaining boundaries

The managed-R3 workflow still does not provide:

- managed deployment, undeployment, load-order integration, or an isolated
  playable test profile for the sealed bundle;
- audible in-game qualification for the selected line, persistence, save/load,
  or clean runtime removal;
- explicit choice among ambiguous installed archive matches;
- Ogg preview, take removal/unlink, history/undo, recording, trimming,
  normalization, transcoding, loudness comparison, actor notes, or lineage;
- folder/batch import, translation/Voice coverage, CSV/XLIFF, or review queues;
- qualified Opus output; or
- new-member namespace/lookup proof or a sealed generation-bound path for
  adopting vanilla dialog/localization identities;
- topic registration, AngelScript generation, conditions/effects, or a
  playable dialog path for a newly authored managed line; or
- localization delete/clone, line/slot relationship editing, bulk language
  production, history, and provenance/rebase workflows, or a complete
  conversation graph editor.

This closes the fresh-project project-local prerequisite and retains the
managed existing-member target and offline build foundation. It does not
complete the Voice production milestone, vanilla adoption, or any runtime
dialog workflow.
