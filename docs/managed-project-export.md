# Managed project snapshot export

A managed revision-3 export is a portable copy of one exact published project
snapshot. It is project management, not a mod build, deployment, Save As,
working-directory move, runtime qualification, or save-game operation.

The current Studio **Create project backup** workflow emits V2: an exact,
restorable managed-project backup. V1 is an unreleased superseded review-copy
experiment, not a compatibility contract. It remains temporarily recognizable
only so the current parser can reject it explicitly; no product workflow emits,
imports, upgrades, or promises to preserve it.

V2 exports the same exact closure with a closed restorable-copy manifest. The
visible Windows workflow can inspect it, materialize it into one absent managed
destination, fully open the receipt-bound candidate, and adopt it only after an
exact identity/head comparison. See
[Managed project snapshot import V2](managed-project-import.md).

## Authority boundary

The export command accepts only:

- the managed Store root;
- one exact canonical expected head; and
- one absent absolute `.goremod` output path outside the Store root.

It accepts no game root, save path, World input, project JSON, overwrite flag,
build profile, deploy target, or runtime claim. The managed session calls it
inside the serialized exact-basis lane. Export neither publishes a new project
head nor changes the current project path.

## Versioned archive format

Every V1 archive has the closed marker `gore.managed-project-snapshot.v1` and
schema `1` in `gore-export.json`. The marker binds the exact project identity,
revision and head, declares `portable_snapshot_review_copy` authority and
`restore_status: not_supported`, and seals every other member.

V2 retains the same deterministic layout and member closure but uses
`gore.managed-project-snapshot.v2`, schema `2`,
`portable_snapshot_restorable_copy`, and `restore_status: supported`.
`supported` is the format property consumed by the separately reviewed Windows
destination materializer. It does not by itself grant session adoption, build,
deployment, game, save, or runtime authority; the visible restore workflow adds
the separately reviewed receipt-bound session transition.

The fixed layout is:

```text
gore-export.json
project.json
store/gore-project.json
store/snapshots/sha256/<2>/<62>.json
store/entities/<id2>/<id30>/<sha256>.json
store/assets/sha256/<2>/<62>
```

`project.json` is the canonical materialized current project for review. The
`store/` members preserve the exact immutable Store layout consumed by the
separately reviewed importer. Absolute paths, directory entries, lock files,
publication-repair journals, staging files, caches and unreachable immutable
orphans are excluded.

## Reachable closure

Collection starts at the expected current head and recursively walks exact
schema-revision-3 snapshots. For every snapshot both versions include and fully
verify:

1. the canonical snapshot manifest;
2. every entity shard named by that manifest;
3. every asset named by that manifest's asset index; and
4. every historical basis snapshot retained by a Quest Draft, recursively.

The current snapshot may additionally authenticate a bounded, newest-first
history vector of at most 255 prior checkpoints. Export includes each checkpoint
named directly by that one current vector, together with its entity, asset, and
Quest-basis closure. It deliberately does **not** follow a history vector found
inside a retained checkpoint or a Quest-basis snapshot. This keeps the exported
history closure bounded by the current authority instead of reviving older
checkpoints that the current snapshot has already truncated. Legacy/root
snapshots omit the history field and therefore add no historical checkpoints.

The history-free revision-3 Store manifest remains capped at 16 MiB. A separate
1 MiB history-envelope reserve raises only the final revision-3 snapshot ceiling
to 17 MiB; legacy revision-1/revision-2 snapshots retain their 16 MiB ceiling.
A stricter custom Store base limit also lowers the effective revision-3 total
limit to that base plus the fixed reserve, so deliberately constrained stores
may reject otherwise format-valid projects.

Existing history-free revision-3 snapshots keep their exact canonical bytes and
remain readable by current Studio versions. Once a history-aware writer creates
a snapshot containing the optional history field, older Studio binaries whose
closed revision-3 parser rejects unknown fields cannot reopen that newer head;
opening it requires a current reader. This is a reader-version compatibility
boundary, not evidence of project corruption.

Objects are deduplicated by their derived Store path and content seal. The same
digest with conflicting lengths, a path collision, cycle/resource overflow,
missing object, non-canonical manifest/entity, embedded-identity mismatch,
asset hash mismatch, or failed full project reopen rejects the export before
publication.

## Determinism and verification

Archive members have a fixed order and path encoding, stored compression, fixed
timestamps and permissions, no comments, and explicit large-file support. The
writer streams bounded Store objects instead of buffering the complete archive.

Before publication, native code strictly reopens the staged ZIP and checks the
closed marker, exact member set/order/metadata, declared lengths and every
payload seal. Two exports of the same head and closure to different absent
paths must be byte-identical.

## Publication lifecycle

The output parent must already exist and have a safe link-free identity. Native
code pins that directory and the Store root by stable handle identity, proves
that the destination is outside the Store, and repeats the ambient-chain and
pinned-ancestry checks at the final pre-publication gate. Moving the pinned
directory, replacing its old name with a link, or moving it underneath the
Store therefore cannot redirect a successful export.

The archive is written, flushed and strictly reopened through a stage owned by
its open handle rather than by a later path lookup. Linux uses an anonymous
`O_TMPFILE` and publishes it with no-replace `linkat(AT_EMPTY_PATH)` through a
separately pinned, syncable parent handle. Windows uses an exclusive
share-locked hidden stage and an atomic, parent-handle-relative no-replace
rename. Pre-publication cleanup acts only on that exact open handle; it never
deletes a path that another process could have replaced. Existing or racing
outputs are never read as authority, deleted, or overwritten.

After the publication boundary there are three sealed response terminals:

- **published**: the new archive was published and fully verified;
- **published with cleanup warning**: the verified archive is complete, but
  internal stage-handle cleanup or cleanup accounting was incomplete; and
- **publication uncertain**: publication may have completed, but final
  durability or verification could not be proved.

Publication uncertainty is terminal. Studio tells the author to inspect the
chosen destination and never retries, replaces, or deletes the same output
automatically. Errors that are proven to occur before the publication boundary
leave the output absent.

## Current V2 Studio workflow

Healthy managed projects expose **Create project backup** from the Project
menu. The dialog emits only V2, asks for one new `.goremod` filename and an
existing destination folder, and explains that the result is a restorable Mod
Studio project backup but not a playable mod, build, deployment, or runtime
qualification. Game and save files are untouched. V1 remains parser/test
coverage and is not produced by this visible action. Because Mod Studio has not
been released and has no active legacy projects, this temporary V1 surface is
eligible for removal rather than migration.

**Restore project backup** is available from the Project menu in every project
state and directly on the empty/Legacy landing surfaces. It verifies the V2
archive first, asks for an existing parent plus a new absent project-folder
name, materializes exactly once, and opens only a confirmed receipt. A canonical
V1 archive receives a dedicated review-copy-not-restorable explanation rather
than being cross-wired into V2.

Export is unavailable while the managed session requires recovery or while a
visible project-text draft has not been saved or discarded. It does not require
a configured game installation and it remains independent of build/readiness
blockers. Legacy project export and Save As behavior remain unchanged.

An initial or racing destination collision is a proven pre-publication failure:
Studio keeps the dialog open and asks for a new filename or folder. Head drift,
Store-integrity failure, or another proven pre-publication stop reports that no
output was created. Only an unclassified post-call/malformed result uses the
terminal "output may exist" warning.

## Required proof

The V1 gate requires native, FFI, session, coordinator and widget tests for:

- byte-identical repeated exports;
- recursive Quest-basis closure, bounded direct-history closure, truncation, and
  orphan exclusion;
- byte-exact ZIP64 local headers, central directory, footer and strict reopen;
- stale-head, corrupted-object and resource-bound failures before publication;
- bounded recursive fan-out and repeated full-verification work;
- initial and racing no-clobber collisions, parent moves, link aliases and
  moves underneath the Store;
- cleanup-warning and publication-uncertain terminals;
- exact DTO/head/project/output binding and serialized session behavior;
- managed-only, game-independent UI with dirty/recovery/busy gates; and
- proof that Store bytes, fixed head, current project path, game and saves are
  unchanged.

The V2 gate additionally proves the distinct closed V2 authority tuple,
byte-identical repeated exports, unchanged Store closure, strict V1 rejection by
the V2 parser, exact reopen by the read-only inspector, and the same format-hard
full-reopen work budget on producers and consumers even when their local Store
limits differ. The companion import gate separately proves same-handle archive
CAS, sealed streaming into retained no-clobber staging handles, fixed-head-last
candidate verification, atomic absent-directory publication, and receipt-free
uncertainty. The visible gate additionally proves V2-only UI wiring, sanitized
inspection and terminal copy, dirty/busy lifecycle gates, cleanup-warning
retention, and exact receipt-bound candidate adoption without displacing the
current session on failure.
