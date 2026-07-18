# Managed project snapshot import V2

Status: implementation checkpoint, July 2026. This document defines the exact
authority of the current managed revision-3 snapshot V2 inspection and
destination materialization plus the visible receipt-bound Studio restore and
session-adoption workflow.

## Format boundary

Snapshot V2 is the sole project backup/restore contract:
`gore.managed-project-snapshot.v2`, schema `2`,
`portable_snapshot_restorable_copy`, and `restore_status: supported`. Studio can
export it, inspect it read-only, materialize it on Windows into one absent
managed-project directory, and adopt only a fully opened candidate that exactly
matches the native receipt. Mod Studio has never been released and has no active
legacy projects, so it retains no older snapshot parser, compatibility result,
upgrade, or migration path. Any non-V2 manifest is invalid input.

`restore_status: supported` describes the V2 archive format. It means that a
verified V2 archive carries the exact authenticated material consumed by the
reviewed destination importer; it does not by itself grant session adoption.
The visible workflow separately selects a destination and adopts only after an
exact receipt-bound reopen. Neither layer makes the project build- or
runtime-ready.

See [Managed project snapshot export](managed-project-export.md) for the shared
closure, deterministic ZIP dialect, and export publication lifecycle.

## Exact checkpoint authority

The inspection command accepts only one bounded source spelling. The source
must be an absolute path to a `.goremod` file. A successful response is a sealed
receipt for the archive, manifest, project identity, project revision, exact
head, and closure counts.

Inspection has no authority to:

- accept or choose a destination or managed Store root;
- extract members or create a working directory;
- publish a Store head, mutate a project, or adopt a current project path;
- read or write a game installation or save;
- build, deploy, undeploy, launch, or qualify runtime behavior; or
- edit, replace, delete, or quarantine the source archive.

The success receipt therefore states `inspection_status: verified_exact`, while
`import_status`, project/game/save mutation, build, and deployment are all
`not_performed`; publication is `not_supported` and runtime remains
`runtime_unqualified`. `retry_safe: true` applies only to repeating this
read-only inspection. It grants no import authority and does not make a prior
receipt safe to use after the source may have changed.

The separate destination command accepts exactly:

- the same bounded absolute `.goremod` source spelling;
- one bounded absolute destination spelling whose final directory is absent;
  no `.goreproj` or other destination suffix is required; and
- the exact inspected archive byte length and SHA-256 as a compare-and-swap
  token.

Native code reinspects the source and consumes that same retained handle. Only
after its archive seal equals the supplied token may it create a private sibling
staging directory, stream the sealed Store members, publish the fixed head last,
fully verify the candidate, and atomically rename the whole directory without
clobbering an existing destination. A confirmed terminal returns the same
path-independent head, project, archive, manifest, and closure receipt as the
inspection. `imported_with_cleanup_warning` still has a confirmed receipt.
`publication_uncertain` deliberately has no archive, head, project, manifest,
closure, or adoptable receipt fields.

Destination materialization has no authority to:

- replace, merge into, or clean an existing destination;
- clone or mint a project identity, advance a revision, or change project
  contents;
- adopt, close, replace, or recover a Mod Studio session;
- read or write a game installation or save; or
- build, deploy, undeploy, launch, or qualify runtime behavior.

Every destination terminal is `retry_safe: false`, including confirmed success,
cleanup warning, cancellation/staleness in the Dart coordinator, and publication
uncertainty. A retry requires a new explicit user operation and, after source
drift, a new inspection. Publication uncertainty must go through a future
recovery operation; callers may not infer that the requested path is safe to
open.

The pure Dart inspection and destination coordinators bind the plan to one
dialog-owned owner/generation lifecycle, enforce single flight and post-await
stale/cancel guards, and carry no session-adoption callback or game path. The
visible dialog displays only a bounded source filename, verifies V2 before
enabling a destination, asks for an existing real parent plus one new absent
folder name, and invokes native materialization once per confirmed operation.
Only confirmed success or cleanup-warning returns a receipt to the shell.
Publication uncertainty and every receipt-free terminal remain close-only and
cannot reach a project opener.

The app-wide current-project coordinator then opens that exact destination as a
candidate inside its serialized ownership lane. Normalized destination,
project ID, project revision, canonical head, and non-poisoned reopen state must
all match the receipt before adoption. Opener failure or any mismatch closes
the unadopted candidate exactly once and leaves the existing Legacy or managed
project unchanged. A successfully adopted candidate stays current even if
retiring the prior session reports a cleanup warning; native-import cleanup and
prior-session cleanup are reported independently.

## Platform boundary

Safe V2 inspection and destination materialization are currently implemented on
Windows only.

On Windows, the inspector pins the source parent, opens the final file without
following a reparse point, and retains that exact handle for every read. The
open share mode permits other readers but excludes concurrent write or delete
sharing. The source must be one regular non-reparse file with exactly one hard
link, and the parent/path identities are revalidated around the open.

On Unix, inspection and import fail closed before source I/O or destination
filesystem access with
`AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED`. `O_NOFOLLOW` alone cannot
exclude an already-open writer from swapping same-length contents during
structured reads. Unix support requires a separately designed sealed private
snapshot boundary; hash bracketing alone is not treated as sufficient.

This platform restriction applies to V2 inspection/import, not to the existing
platform-specific safe publication paths of snapshot export.

## Source and destination security model

The V2 source is untrusted. Acceptance requires all of the following through the
same retained Windows file handle:

1. The absolute `.goremod` source resolves through a safe directory chain to one
   pinned, regular, non-reparse, single-link file whose write/delete sharing is
   excluded.
2. A whole-file length and SHA-256 pass is taken before structured reads.
3. The archive matches the one closed deterministic ZIP/ZIP64 dialect: fixed
   member order, stored payloads, fixed metadata, no comments or directory
   entries, no duplicate or case-folding-colliding names, and no unknown
   members or trailing layout variants.
4. `gore-export.json` is canonical closed JSON with the exact V2 authority tuple.
   It seals every other member by relative name, byte length, and SHA-256.
5. Member paths must match their embedded Store identities and content digests.
   Every declared payload is read within its bound and rehashed.
6. The exact fixed head and complete reachable revision-3 Store closure are
   reopened from the archive. Missing, unreachable, extra, non-canonical, or
   identity-inconsistent snapshots, entities, assets, or project material fail
   inspection. Referenced Ogg assets receive their normal metadata validation.
7. A second whole-file length and SHA-256 pass must equal the first pass.
8. Import compares the resulting whole-archive seal with the caller's exact CAS
   before any destination operation. A prior receipt alone is not a
   time-of-check/time-of-use capability.
9. The absolute destination resolves to a pinned safe existing parent and one
   absent nonempty final component. A private sibling staging root and all
   planned child directories/files are created no-clobber relative to retained
   exact handles; write/delete replacement remains excluded throughout staged
   verification.
10. Only authenticated Store snapshots, entities, and assets are streamed. Each
    output is bounded by its seal, hashed while writing, synchronized, rehashed,
    and retained in the exact bounded ownership inventory. The snapshot manifest
    and canonical `project.json` member are not installed into the working Store.
11. The canonical fixed head is written last. The planned tree must contain no
    missing or extra entry, every retained identity/seal is rechecked, the Store
    reopens to the exact inspected current project, and the retained source seal
    is revalidated immediately before publication.
12. The whole staging directory is atomically promoted relative to the pinned
    parent with no replacement authority. Windows requires descendant handles
    to be released immediately before a parent-directory rename, so the
    published directory identity, exact tree, every recorded member identity and
    seal, current Store reopen, and source seal are checked again after
    promotion. A recursive overlapped directory-change watch is armed before
    those final checks. A still-pending watch after the last exact pass is the
    receipt linearization point; any reported change, notification overflow, or
    indeterminate watcher state becomes non-adoptable publication uncertainty
    rather than an ordinary error.

The receipt linearizes the exact managed tree, bytes, and recorded identities
inside the published destination. Both final passes and the full Store reopen
also require every member to have one hard link. Windows can nevertheless add a
new hard-link name outside the watched destination without producing an in-tree
directory notification, so this checkpoint does not claim one atomic global
no-external-alias snapshot. A later Studio session open must independently
revalidate the destination and rejects an alias or any other drift before
adoption. Per-file oplocks or volume-journal accounting would be a separate,
broader alias-linearization contract.

## Closed limits

The public bridge, native inspector, and native importer enforce these hard
ceilings before a receipt can be accepted:

| Resource | Ceiling |
|---|---:|
| Source spelling | 32 KiB UTF-8 |
| Destination spelling | 32 KiB UTF-8 |
| Archive file | 70 GiB |
| Total uncompressed archive bytes | 70 GiB |
| `gore-export.json` | 128 MiB |
| Materialized `project.json` | 16 MiB |
| Snapshot objects | 100,000 |
| Entity objects | 100,000 |
| Asset objects | 100,000 |
| Total Store closure objects | 300,000 |
| Archive entries | 300,003: manifest, project copy, fixed head, and at most 300,000 Store objects |
| Aggregate full-reopen work | 262,144 charged objects and 128 GiB charged bytes across all reachable snapshots |
| Fixed-head JSON / response head JSON | 64 KiB |
| Project revision on the response wire | `0..=2^63-1` |

The normal managed Store ceilings are enforced again during closure reopen.
They include a 16 MiB history-free revision-3 snapshot base plus the fixed 1 MiB
history reserve for a 17 MiB final snapshot ceiling, 1 MiB per entity, 512 MiB
each for aggregate snapshot and referenced entity bytes, 64 GiB aggregate
referenced asset bytes, 64 MiB per referenced Ogg payload, and 1 KiB logical
asset names. Stricter caller-supplied Store limits may lower these ceilings;
they can never raise the
format limits. Any count, sum, offset, allocation, member, or wire-range overflow
fails before success.

The full-reopen work budget is charged per unique reachable snapshot before any
of that snapshot's entity payloads or asset index are reopened. Shared entity or
asset members are deliberately charged again when another snapshot references
them. This mirrors the V2 export preflight and prevents a small deduplicated
archive from amplifying into unbounded repeated parsing, iteration, or transient
project allocation during inspection.

Both V2 writer and reader charge every potential nested Quest-basis lookup at
the fixed 17 MiB revision-3 format ceiling. A producer's or consumer's stricter
local Store limits never lower this format charge, so an archive cannot become
unreadable merely because it was exported under different conforming limits.

## Stable failure semantics

Callers branch on the stable code, not on native error text:

| Code | Meaning |
|---|---|
| `AUTHORING_REVISION3_IMPORT_REQUEST_INVALID` | The strict command/payload shape, field set, or command name is invalid. |
| `AUTHORING_REVISION3_IMPORT_LIMIT` | A closed wire, path, archive, member, object-count, byte-count, or Store safety ceiling was exceeded. |
| `AUTHORING_REVISION3_IMPORT_SOURCE_INVALID` | The source spelling, extension, safe-path/file type, share lock, identity, length, or whole-file stability check failed. |
| `AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED` | This platform cannot yet provide the required immutable inspection boundary. |
| `AUTHORING_REVISION3_IMPORT_ARCHIVE_INVALID` | ZIP structure, exact dialect, member order/metadata, layout, or payload seal validation failed. |
| `AUTHORING_REVISION3_IMPORT_MANIFEST_INVALID` | The canonical manifest, V2 authority tuple, member plan, path, basis, or declared seal is invalid or unsupported. |
| `AUTHORING_REVISION3_IMPORT_CLOSURE_INVALID` | Full Store reopen, reachability, identity, canonical object, project materialization, or referenced-asset validation failed. |
| `AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED` | During destination import, the source no longer verifies as the exact inspected archive CAS. All post-inspection source/archive/manifest/closure drift is collapsed into this code. |
| `AUTHORING_REVISION3_IMPORT_DESTINATION_INVALID` | The destination spelling, parent chain, absent-final-name policy, pinned identity, or no-clobber requirement failed. |
| `AUTHORING_REVISION3_IMPORT_MATERIALIZATION_FAILED` | Exact private staging or sealed streaming failed before final publication. |
| `AUTHORING_REVISION3_IMPORT_VERIFICATION_FAILED` | The staged Store, fixed-head order, exact planned tree, seal, or full candidate reopen failed before final publication. |
| `AUTHORING_REVISION3_IMPORT_PUBLICATION_FAILED` | Atomic destination publication provably did not complete. |
| `AUTHORING_REVISION3_IMPORT_CLEANUP_FAILED` | Import failed before publication and bounded cleanup of importer-owned private staging was incomplete. |
| `AUTHORING_REVISION3_IMPORT_INVARIANT` | A native success receipt violated the closed response contract and was suppressed. |

Native validation details are intentionally collapsed into these categories
before crossing the FFI boundary. Returned messages are bounded to 4 KiB and do
not include the internal native reason or source path. A success response echoes
the caller's exact source spelling only to bind request and receipt; user-facing
surfaces should display the bounded filename label rather than parent paths.

Inspection failures are non-mutating. An import error never authorizes a final
destination; `CLEANUP_FAILED` may leave only the bounded importer-owned private
staging inventory for a future recovery/cleanup surface. An
unsupported-platform result is not archive invalidity, and an exact V2
inspection success is not an import success.

Confirmed publication uses either no warning or
`AUTHORING_REVISION3_IMPORT_CLEANUP_WARNING`. An uncertain publication uses
`AUTHORING_REVISION3_IMPORT_PUBLICATION_UNCERTAIN`, contains no receipt fields,
and must never be retried or adopted automatically. These are successful wire
terminals with `retry_safe: false`, not ordinary error responses.

## Visible workflow and remaining recovery boundary

**Restore project backup** is visible from the Project menu in every project
state and on the empty/Legacy landing surfaces. Dirty managed or Legacy edits
are confirmed before source selection. The whole dialog and candidate-adoption
sequence occupies one global project-action lane, while native materialization
additionally disables dialog close/back and duplicate submission. Read-only
inspection can be cancelled safely; disposal invalidates late results.

Publication uncertainty and importer-owned staging cleanup still need a future
deliberate inspection/recovery surface rather than path guessing. The current
UI truthfully names the attempted destination, opens nothing, and forbids an
automatic retry. It does not claim to clean or classify an uncertain final
path.

The current operation preserves project identity and revision: it is Restore /
Relocate, not Clone or Save As. Clone/fork identity policy remains separate.
The landed workflow grants no Clone/Save As product claim, build, deployment,
save/game mutation, or runtime claim.
