# Managed project snapshot export

The managed revision-3 export is a portable copy of one exact published project
snapshot. It is project management, not a mod build, deployment, Save As,
working-directory move, runtime qualification, or save-game operation.

V1 deliberately has no importer. Mod Studio must therefore call the result a
**project copy** or **portable snapshot/review copy**, never a restorable backup.
The original managed project directory remains authoritative and must be kept.

## Authority boundary

The export command accepts only:

- the managed Store root;
- one exact canonical expected head; and
- one absent absolute `.goremod` output path outside the Store root.

It accepts no game root, save path, World input, project JSON, overwrite flag,
build profile, deploy target, or runtime claim. The managed session calls it
inside the serialized exact-basis lane. Export neither publishes a new project
head nor changes the current project path.

## Archive format

Every archive has the closed marker `gore.managed-project-snapshot.v1` in
`gore-export.json`. The marker binds the exact project identity, revision and
head, declares `portable_snapshot_review_copy` authority and
`restore_status: not_supported`, and seals every other member.

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
`store/` members preserve the exact immutable Store layout required by a future
separately reviewed importer. Absolute paths, directory entries, lock files,
publication-repair journals, staging files, caches and unreachable immutable
orphans are excluded.

## Reachable closure

Collection starts at the expected current head and recursively walks exact
schema-revision-3 snapshots. For every snapshot V1 includes and fully verifies:

1. the canonical snapshot manifest;
2. every entity shard named by that manifest;
3. every asset named by that manifest's asset index; and
4. every historical basis snapshot retained by a Quest Draft, recursively.

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

## Studio workflow

Healthy managed projects expose one shared **Export project copy** dialog from
the Project menu and Home's Project tools. The dialog asks only for a new
portable filename and destination folder, explains that the result is not a
playable mod, and states that game and save files are untouched.

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
- recursive historical closure and orphan exclusion;
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
