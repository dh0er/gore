# Managed project snapshot import V2 foundation

Status: implementation checkpoint, July 2026. This document defines the exact
authority of the current managed revision-3 snapshot V2 foundation. It is not a
claim that Mod Studio can import or restore a project.

## Version boundary

The two snapshot versions are separate, closed contracts:

| Version | Manifest authority | Current meaning |
|---|---|---|
| V1 | `gore.managed-project-snapshot.v1`, schema `1`, `portable_snapshot_review_copy`, `restore_status: not_supported` | Frozen review-only project copy. It remains useful for inspection by a person, but is not and will not become an importable artifact. A recognized canonical V1 manifest receives a dedicated unsupported-review-copy result instead of being accepted as V2; this does not validate the V1 closure as restorable. |
| V2 | `gore.managed-project-snapshot.v2`, schema `2`, `portable_snapshot_restorable_copy`, `restore_status: supported` | Exact restorable-copy format. The current checkpoint can export it through the backend contract and inspect it read-only; it cannot restore it into a destination. |

V1 must not be relabelled, edited, or upgraded in place. Its original managed
project directory remains authoritative. V2 uses the same deterministic member
layout and exact reachable Store closure as V1, but its closed manifest tuple
explicitly declares a future restore contract.

`restore_status: supported` describes the V2 archive format, not a completed
product operation. It means that a verified V2 archive carries the exact
authenticated material required by a separately reviewed future importer. It
does not mean that a destination has been selected, materialized, published, or
adopted.

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

The Dart planning coordinator is likewise inspect-only and is not connected to
a Studio screen, menu, file picker, session transition, or publication callback.
The current **Export project copy** UI still emits V1. The V2 exporter and
inspector are backend/bridge foundations only.

## Platform boundary

Safe V2 inspection is currently implemented on Windows only.

On Windows, the inspector pins the source parent, opens the final file without
following a reparse point, and retains that exact handle for every read. The
open share mode permits other readers but excludes concurrent write or delete
sharing. The source must be one regular non-reparse file with exactly one hard
link, and the parent/path identities are revalidated around the open.

On Unix, inspection fails closed before source I/O with
`AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED`. `O_NOFOLLOW` alone cannot
exclude an already-open writer from swapping same-length contents during
structured reads. Unix support requires a separately designed sealed private
snapshot boundary; hash bracketing alone is not treated as sufficient.

This platform restriction applies to V2 inspection, not to the existing
platform-specific safe publication paths of snapshot export.

## Read-only security model

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
   identity-inconsistent snapshots, entities, assets, or review material fail
   inspection. Referenced Ogg assets receive their normal metadata validation.
7. A second whole-file length and SHA-256 pass must equal the first pass.

Nothing is extracted during these checks. A future destination importer must
either repeat inspection immediately before materialization or retain and
consume the same authenticated open source. The current receipt alone is not a
time-of-check/time-of-use capability.

## Closed limits

The public bridge and native inspector enforce these hard ceilings before a
receipt can be accepted:

| Resource | Ceiling |
|---|---:|
| Source spelling | 32 KiB UTF-8 |
| Archive file | 70 GiB |
| Total uncompressed archive bytes | 70 GiB |
| `gore-export.json` | 128 MiB |
| Materialized review `project.json` | 16 MiB |
| Snapshot objects | 100,000 |
| Entity objects | 100,000 |
| Asset objects | 100,000 |
| Total Store closure objects | 300,000 |
| Archive entries | 300,003: manifest, review project, fixed head, and at most 300,000 Store objects |
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
| `AUTHORING_REVISION3_IMPORT_UNSUPPORTED_REVIEW_COPY` | The canonical manifest identifies V1 review-only authority. It is deliberately not accepted as V2, and its closure is not validated as restorable. |
| `AUTHORING_REVISION3_IMPORT_ARCHIVE_INVALID` | ZIP structure, exact dialect, member order/metadata, layout, or payload seal validation failed. |
| `AUTHORING_REVISION3_IMPORT_MANIFEST_INVALID` | The canonical manifest, V2 authority tuple, member plan, path, basis, or declared seal is invalid or unsupported. |
| `AUTHORING_REVISION3_IMPORT_CLOSURE_INVALID` | Full Store reopen, reachability, identity, canonical object, project materialization, or referenced-asset validation failed. |
| `AUTHORING_REVISION3_IMPORT_INVARIANT` | A native success receipt violated the closed response contract and was suppressed. |

Native validation details are intentionally collapsed into these categories
before crossing the FFI boundary. Returned messages are bounded to 4 KiB and do
not include the internal native reason or source path. A success response echoes
the caller's exact source spelling only to bind request and receipt; user-facing
surfaces should display the bounded filename label rather than parent paths.

Every failure is non-mutating. A recognized V1 result is not corruption, an
unsupported-platform result is not archive invalidity, and an exact V2
inspection success is not an import success.

## Explicitly missing next checkpoint

A real import still needs a separately reviewed contract for destination
selection and collision policy, safe materialization into an absent or proven
empty managed directory, crash-safe publication, fresh source authentication,
post-materialization full reopen, and deliberate session adoption. Recovery and
cleanup terminals, user confirmation, UI copy, and cancellation behavior also
remain undefined.

Until those pieces exist, there is no V2 Destination Import, Restore, Adopt,
Import/Clone/Save As UI, build, deployment, save/game mutation, or runtime claim.
