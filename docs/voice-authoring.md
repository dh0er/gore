# Managed revision-3 Voice authoring

GORE Mod Studio now has a bounded managed revision-3 workflow for importing one
real Ogg take at an existing dialog line. It is an offline project-authoring
slice, not a production voice deployment workflow.

The older compatibility-project editor remains a separate proven path for
replacing an exact existing archive member and lowering it to the legacy build
and deployment engine. The managed-R3 workflow documented here does not yet
lower to that path.

## Visible Home workflow

With a managed revision-3 project open and the Gothic 1 Remake installation
configured in Settings, **Add Voice take** is available from Home. The
configured game root is used only as a read-only forbidden-root safety
boundary: the project Store's resolved path must remain outside the
installation. The workflow does not inspect game catalogs or content through
that root; the selected Ogg remains a separate read-only safe-file input. The
wizard:

1. loads the exact-current semantic content index;
2. searches intact existing `DialogLine` entities by speaker, line name, or
   localization identity, with at most 50 visible matches per query;
3. requires an explicit line choice and a canonical locale such as `de` or
   `en-US`;
4. chooses one local `.ogg` file, proposes a take name from its filename, and
   lets the author assign Draft, Recorded, Reviewed, or Approved status;
5. shows whether the line/locale already has a slot, how many candidate takes
   it contains, and whether one is selected;
6. allows only an Approved take to become selected and requires explicit
   confirmation before replacing an existing selection; and
7. reloads the exact content index before publication, then refreshes the
   visible project revision and head after success.

Technical entity IDs, AssetStore hashes, archive paths, build controls, and
runtime controls stay out of the normal wizard. The dialog-line result list is
bounded rather than eagerly rendering a large catalog.

The wizard intentionally does not offer dialog-text editing. It sends no text
change, so the existing `LocalizationEntry` is preserved exactly. The native
transaction supports an explicit optional text request for a future verified
surface, but that is not part of the normal workflow.

## Exact transaction and Ogg safety

The managed service and current-project coordinator bind the operation to the
exact project root, configured safety game root, project ID, project revision,
game-generation target, and canonical working head. A stale wizard or a
project that requires reopen cannot publish. Native code resolves the configured
game root only to prove that the working Store cannot alias it; it does not use
the root as a game-catalog or content input.
The native FFI route prepares and fully reopens an immutable candidate without
replacing the fixed head; the managed session owns the later guarded head-CAS,
repair-journal, and full published-reopen sequence.

Native code first performs every line/locale/slot/take check that does not need
an Ogg receipt. It then reads, bounds, hashes, and parses the source Ogg into an
opaque in-memory preparation without creating Store staging or CAS state. That
preview drives the remaining pure transaction and complete candidate-capacity
evaluation. The same source is prepared a second time, also without Store
mutation. Only an exact byte-and-metadata match allows the first accepted bytes
to be installed in AssetStore CAS.
Missing, unavailable, unsafe, oversized, invalid, or changed source files are
reported as retryable source failures. Basis-head or published-state
uncertainty remains a session-fatal/reopen boundary; a candidate verification
failure before head CAS can remain retryable when the exact basis is still
proven current. Unexpected derived-ID collisions, response-limit violations,
revision exhaustion, and malformed or unknown native responses fail closed and
require reopening the managed project.
An unavailable configured game root or Store/game-root overlap is reported as
an actionable, retryable configuration error before the Store is opened.

After the accepted bytes have entered immutable CAS, a later exact-head race
can leave only a verified unreferenced CAS object. It cannot publish a partial
candidate or replace the fixed head.

The accepted Ogg is stored content-addressed with verified Vorbis or Opus
metadata. The external source path is neither an authored entity identity nor a
deployment target.

## Semantic result

The transaction operates only on an existing exact-project `DialogLine` and
its existing `LocalizationEntry`:

- it creates an unresolved locale `VoiceSlot` when none exists, or appends to
  the exact existing unresolved slot;
- it creates one new `VoiceTake` backed by the verified Ogg AssetStore object;
- it retains multiple take candidates and their closed production status;
- it selects the new take only when its status is Approved; and
- it increments the affected project/entity revisions and requires canonical
  reopen equality.

Every successful result remains explicitly `blocked`, `runtime_unqualified`,
`not_granted`, and native-publication `not_supported`. Durable output stays
inside the managed project store: the content-addressed Ogg and the published
project checkpoint. The workflow does not compile or deploy, write the game
installation, or touch a save file.

## Deliberate remaining boundaries

This first slice does not provide:

- archive-member target resolution or a managed-R3 `BuildSpec.voice` lowerer;
- managed build, deploy, undeploy, or audible-gameplay qualification;
- Ogg preview, take removal/unlink, or history/undo controls;
- recording, trim, normalization, transcoding, loudness/duration comparison,
  notes, actor assignments, or lineage;
- folder/batch import, translation/voice coverage, CSV/XLIFF, or review queues;
- new-member namespace or runtime lookup proof; or
- creation of a new `DialogLine` or `LocalizationEntry`.

Accordingly, this completes only the bounded transaction/import portion of the
Voice milestone. It is a safe managed-project foundation for the later
production workflow, not completion of that workflow.
