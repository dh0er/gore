# Managed revision-3 Voice authoring

GORE Mod Studio has a bounded managed revision-3 workflow for importing Voice
takes, resolving their exact installed archive targets, and building an
offline sealed replacement bundle. The complete path remains deliberately
narrow: it can replace proven existing archive members, but it does not deploy
the bundle or claim that the result has been heard and qualified in game.

The older compatibility-project Voice editor and deployment engine remain a
separate path. Managed-R3 content is never silently copied into that project or
treated as deployed compatibility state.

## Visible Home workflow

With a managed revision-3 project open and a Gothic 1 Remake installation
configured in Settings, Home exposes three separate actions:

1. **Add Voice take** imports one real local Ogg for an existing dialog line and
   locale. The search-first wizard hides technical identities, retains
   alternate takes, supports Draft/Recorded/Reviewed/Approved status, and lets
   only an Approved take become selected.
2. **Resolve Voice target** inspects the exact installed locale archive for one
   existing structurally intact Voice slot. It records zero, one, or multiple
   matching members as unresolved, resolved, or ambiguous. It never chooses an
   ambiguous match implicitly.
3. **Build Voice bundle** evaluates every current Voice slot and either shows
   all structured blockers without creating output, or writes one sealed
   voice-only bundle into a brand-new folder selected by the author. This is an
   offline build; the dialog has no deployment action.

All three actions reload or bind the exact current project checkpoint. A stale
dialog, changed project identity, changed canonical head, or session requiring
reopen fails closed. After a successful authoring publication, Home refreshes
to the new managed project revision and head.

The normal UI never asks for entity IDs, archive paths, member names, hashes,
CAS paths, or bundle internals. A full Voice slot remains eligible for target
resolution even though its candidate-capacity limit correctly prevents adding
another take.

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
- new-member namespace/lookup proof, new dialog-line creation, or new
  localization-entry creation.

This completes the managed existing-member target and offline build foundation,
not the Voice production milestone or runtime workflow.
