# Cooked DataAsset fixed-leaf workflow

`gore asset` is a deliberately narrow, copy-on-write editor for proven
fixed-width leaves in legacy split Unreal packages. It is not a generic
DataAsset serializer and it does not deploy files into the game.

The public commands are:

```text
gore asset extract
gore asset inspect
gore asset patch-fixed
gore asset pack
```

## 1. Extract one package from the installed game

Start from an exact cooked package path. The parent of the output directory must
already exist, the output directory itself must not exist, and output inside the
live game tree is refused.

```powershell
$game = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake'
$asset = '/Game/Example/DA_Example'

$extractJson = & gore asset extract `
  --game $game `
  --asset $asset `
  --out 'extracted' `
  --json
$extract = $extractJson | ConvertFrom-Json
```

This creates a legacy `.uasset`/`.uexp` pair plus the exact correlated
`.ubulk`, `.uptnl`, and `.m.ubulk` sidecar set, an exact sealed
`gore-generation.usmap` copy, and
`gore-asset-extract.json`. It does not modify or deploy anything
under the game directory. `--json` prints the same receipt that is stored in the
new output directory.

Use the copied USMAP and keep the receipt beside the extracted pair. This avoids
silently switching to a mapping file from a later hotfix:

```powershell
$uasset = (Get-ChildItem 'extracted' -Filter '*.uasset' -File |
  Select-Object -First 1).FullName
$usmap = (Resolve-Path 'extracted/gore-generation.usmap').Path
$extractReceipt = (Resolve-Path 'extracted/gore-asset-extract.json').Path
```

## 2. Inspect the extracted package pair

The input is a `.uasset`; its sibling `.uexp` and the exact raw `.usmap` used
for this game build are required.

```powershell
$inspectJson = & gore asset inspect `
  --uasset $uasset `
  --usmap $usmap `
  --json
$inspect = $inspectJson | ConvertFrom-Json
$inspect.summary
$inspect.exports | Select-Object index, object_name, class_path, status, error
```

`status=walked` means the export's property stream was decoded under the
explicit `g1r_ue5_4` profile. Unsupported exports remain listed with a typed
error. The top-level status is `walked`, `partial`, or `unsupported` so a
successful report cannot be mistaken for proof that every export was walked.

Each listed leaf contains:

- a readable semantic path;
- an `editable` structural-safety flag;
- a complete, offset-free selector.

Choose a leaf deliberately. This example saves one complete inspect leaf;
`patch-fixed` accepts the leaf wrapper as well as its nested selector. The
explicit UTF-8 encoding avoids Windows PowerShell's legacy UTF-16 redirection.

```powershell
$leaf = $inspect.exports[0].leaves |
  Where-Object { $_.editable -and $_.semantic_path -eq '/Example/Value' } |
  Select-Object -First 1

if ($null -eq $leaf) { throw 'Requested editable leaf was not found' }

$selectorJson = $leaf | ConvertTo-Json -Depth 100
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText(
  (Join-Path $PWD 'selector.json'),
  $selectorJson,
  $utf8NoBom
)
```

### Mod Studio read-only inspection

Mod Studio's **DataAsset Lab** exposes the same bounded inspection boundary for
local snapshots. Select one `.uasset` and the exact `.usmap`; the native command
derives the sibling `.uexp`, reads all three through guarded no-follow handles,
and returns only sealed package facts plus offset-free selectors. The UI reports
`walked`, `partial`, and `unsupported` exports separately, searches the proven
leaf facts, and keeps large result lists lazy.

The DataAsset Lab itself remains deliberately evidence-only. It has no patch,
project-save, pack, deploy, or runtime-qualification control, and native paths
or raw offsets are never returned in its result. Use the receipt-bound CLI
workflow below for the standalone copy-on-write operation. A separate managed
revision-3 project view can import the resulting verified PatchReceipt into its
stage registry. The same view now offers a first typed value-edit workflow for
one Lab result, without weakening the Lab's read-only contract. That workflow
still requires a separately produced, exact ExtractReceipt-v2; it does not
extract a package, infer provenance, or turn successful inspection into general
write authority.

Managed revision-3 projects also provide a search-first **Browse installed
packages** surface. Its package inventory is tied to the exact project head and
installed executable generation. Search filtering retains each candidate's
original sealed ordinal; the inspection request sends that ordinal and the
expected inventory seals, never a caller-selected package path or output path.
Native code rebuilds the complete installed snapshot, resolves the candidate
server-side, selects and guards the installed generation USMAP, converts the
package to bounded in-memory `.uasset`/`.uexp` bytes, and feeds those bytes into
the same fixed-leaf inspector used by the Lab. No extracted files are published.

The report remains read-only evidence, but each `editable=true` leaf can now
open a separate typed value editor. It shows a Before/After preview and can
publish one managed revision-3 fixed-leaf stage. Refreshing the inventory
invalidates an in-flight selection, and stale or `requiresReopen` publication
closes the editor, inspection, and old browser together. The manual `/Game`
field remains syntax-and-copy only and cannot be inspected or edited because it
has no sealed package candidate.

This installed route creates no ExtractReceipt and accepts no caller-supplied
target path, package ID, package bytes, receipt path, output path, or raw offset.
Native code rebuilds the exact package and USMAP inventories, extracts only the
server-selected ordinal, and independently reconstructs the same target. It
requires the complete package pair, every role-bearing sidecar, the exact USMAP
name and bytes, the opened UTOC set, and every generation-relevant chunk winner
to match before applying the offset-free semantic edit in memory. The normal
closed stage verifier and exact-head publication lane are then reused. A final
generation/source recheck wins over parsing or patch diagnostics.

The result is still an offline, build-blocked project stage. It does not pack,
deploy, touch the game installation or a save, or qualify runtime behavior.

## 3. Prepare one raw fixed-width replacement

The selector's `expected_hex` is the complete current on-wire value. Supply it
again to make the command an explicit compare-and-swap operation. The
replacement must contain exactly the same number of bytes.

```powershell
$expected = $leaf.selector.expected_hex
$replacement = '...' # full-width raw little-endian wire bytes
```

`editable=true` proves only that the byte width, role, path, schema, and
container shape can be preserved. It does **not** validate gameplay meaning or
numeric domains. In particular, raw integer ranges and floating-point NaN or
infinity values remain the author's responsibility.

## 4. Patch to a new pair

```powershell
New-Item -ItemType Directory -Force 'patched' | Out-Null

$patchJson = & gore asset patch-fixed `
  --uasset $uasset `
  --usmap $usmap `
  --extract-receipt $extractReceipt `
  --selector 'selector.json' `
  --expected-hex $expected `
  --replacement-hex $replacement `
  --out 'patched/DA_Example.uasset' `
  --json
$patch = $patchJson | ConvertFrom-Json
$patchReceipt = $patch.output.receipt
```

The source pair and sidecars are never modified. The output parent must already
exist; the pair, all three possible output-sidecar names, and the derived receipt
must all be absent; and the output cannot alias either input component. Every
sidecar named by the extract manifest is bounded, hash-verified, and copied
without clobbering. Sidecars are published first; the pair writer then publishes
`.uexp` and `.uasset` last as the visible commit marker. An ordinary failure
cleans verified files created by this invocation. A forced process stop can
leave orphan sidecars or `.uexp`, but not a `.uasset` claiming a
process-incomplete set. This ordering is not an OS-crash or power-loss
durability guarantee.

`patch-fixed` refuses an input pair, USMAP, or optional-sidecar set that does not
match the extract receipt. It writes a `gore.asset.patch-fixed.v2`
`<output-stem>.gore-asset-patch.json` beside the new package. That receipt seals
the extract receipt hash, original generation, complete extracted component and
sidecar set, input pair, patched pair, and exact copied output-sidecar
roles/names/lengths/hashes. Keep the complete directories and receipts together.

The result calls the embedded selector `input_selector` because it is sealed to
the old pair. It is intentionally stale as soon as the output changes.

## 5. Re-inspect the output

```powershell
gore asset inspect `
  --uasset 'patched/DA_Example.uasset' `
  --usmap $usmap `
  --json
```

Always re-inspect before a second edit and save a fresh selector. Reusing the
old selector fails on the complete pair seal before mutation.

### Native revision-3 project staging boundary

The native libraries now provide a project-owned, prepare-only path for this
same proven edit. `verify_fixed_leaf_stage_input` consumes a verified
PatchReceipt-v2 capability, reopens its complete receipt chain, reconstructs
the target package again from the live IoStore in a private game-disjoint
directory, reproduces the semantic selector patch, union-probes the live target
and conversion dependencies, and binds the exact non-empty game executable.
Receipt text alone cannot construct this opaque input.

`WorkingProjectStore::prepare_revision3_dataasset_stage_v1` then imports the
patched `.uasset`/`.uexp`, exact USMAP, optional sidecars, and one canonical
stage manifest into the revision-3 AssetStore/CAS. The manifest media type is
`application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1`. It stores the
project/basis identities, target package, generation facts, offset-free
selector, replacement bytes, component seals, and closed status enums. It does
not persist receipt bytes, local paths, raw offsets, or an authority-bearing
file handle.

Prepare, list, and registry-only remove require one exact published head. They
fully reopen the current project and every unique historical stage basis under
aggregate count/byte/work budgets, and a prepared candidate is fully reopened
before return. Stage preparation fails closed on Store/game aliasing and live
executable or generation drift. No method replaces `gore-project.json`; a
caller must eventually use the managed session's guarded fixed-head publication
lane. Races may leave only verified immutable CAS orphans.

Every staged manifest remains explicitly `blocked`, `runtime_unqualified`,
`not_granted`, and `not_supported`. Closed raw FFI commands now expose exact-head
prepare, list, and registry-only remove. They cap their envelopes before generic
JSON materialization, preflight the PatchReceipt path without following unsafe
file types, validate the complete live generation and manifest closure, reject
all project/manifest numbers outside the signed Studio wire, and never return a
local path, receipt text, raw offset, or authority-bearing handle.

Strict Dart DTOs now mirror the closed prepare/list/remove results without
exposing the PatchReceipt input path, receipt bytes, raw offsets, or a native
publication handle. They validate the real native struct-order manifest seal
even though embedded `serde_json::Value` objects arrive with sorted map keys,
reconstruct the exact nested working-head order, reject non-signed-wire numbers
and expanded status claims, and close candidate/AssetStore bindings.

### Direct typed fixed-leaf staging

`authoring_store_prepare_revision3_dataasset_edit_v1` removes the need to author
an intermediate PatchReceipt independently; it does **not** remove the extraction
proof requirement. Its exact request combines one current managed-project head,
a separately produced ExtractReceipt-v2 path, the confirmed `/Game` target from
that receipt, one offset-free selector returned by `dataasset_fixed_inspect_v1`,
and one strictly typed semantic replacement. Supported values are Bool, bounded
signed and unsigned integers, finite 32-/64-bit floats, linear RGBA, and
four-component vectors. Integers travel as exact decimal strings; native code
performs the authoritative little-endian encoding and requires the replacement
kind and byte width to equal the selector.

Before authoring, the read-only
`authoring_read_dataasset_extract_receipt_v2` command fully verifies the selected
ExtractReceipt and returns only a bounded summary: its exact `/Game` target,
package-component seals, USMAP seal, and component lengths. The Studio requires
those package and schema facts to match the Lab inspection. It also shows the
receipt's target and requires an explicit confirmation because two byte-identical
cooked packages can still represent different in-game targets. The summary
returns no local receipt path, raw offset, selector bytes, or replacement value.

Native code reopens the bounded, no-follow ExtractReceipt chain and exact
pair/USMAP/sidecars, then creates a private game-disjoint PatchReceipt-v2 pair
only for the duration of the call. That private chain passes the unchanged full
PatchReceipt verifier: semantic rewalk, fresh live conversion, executable seal,
generation probes, and Store/source-root disjointness. Temporary artifacts are
deleted before return. The response is the ordinary closed prepare-only R3
stage/candidate/head result; it contains no receipt path, temporary path, raw
offset, or extra build/publication authority. Native code additionally returns
an `intent_binding_sha256` over the confirmed target, canonical offset-free
selector, and exact encoded replacement bytes. The strict Dart wrapper computes
the same domain-separated digest and accepts the candidate only when its target,
stage manifest, and digest preserve that exact user intent.

The Dart semantic model round-trips the canonical selector schema from the
strict inspector DTO instead of maintaining a second selector wire. Its guided
panel searches only `editable=true` leaves, presents typed controls, shows a
friendly Before/After preview, verifies the separately selected ExtractReceipt,
shows and confirms its exact target, and delegates one stage transaction. Wrong
or stale receipts are intentionally not guessed: native verification rejects
any package, schema, generation, target, or intent mismatch. The workflow is
wired through the shared managed session and Home DataAsset surface; successful
publication advances and reloads the exact managed project checkpoint.

`ManagedRevision3AuthoringProjectSession` publishes receipt-import, typed-edit,
and remove candidates through the same serialized full-reopen, crash-repair,
exact fixed-head byte-CAS, and post-publication reopen lane used by other
revision-3 mutations. Listing is an exact-head serialized read. Head drift never
clobbers the winner; malformed or integrity-uncertain results poison the
session, while bounded local-input failures remain retryable after the disk head
is rechecked.

Managed revision-3 Home exposes the exact registry as **Verified DataAsset
edits**. The author can search friendly asset names or `/Game` paths, inspect
bounded verified facts, create the first typed fixed-leaf edit through the
summary/confirmation/preview flow, import a PatchReceipt JSON through the expert
file picker, and remove an entry from the project registry after confirmation.
Initial or failed exact-head loading, checkpoint drift, and `requiresReopen`
lock mutation. Every mutation is bound to the exact project root, project ID,
revision, and head; each success advances and reloads the visible checkpoint. A
registry removal does not modify the source receipt or game installation.

The visible registry manages independently receipt-verified fixed-size edits;
the generic semantic component remains a typed value-editor slice, not a
general DataAsset editor. Neither grants build, pack, deploy, gameplay,
runtime, or future-reinspection authority. The first closed reviewed schema now
covers only the `FeetTextureSize` X/Y field of the exact Human, Scavenger, and
Wolf footstep presets. Broader gameplay schemas, gameplay-qualified units,
multi-edit transactions, undo, build lowering, post-pack verification,
structural edits, and the sealed Unreal handoff remain separate work. The typed
workflow writes only immutable objects and the guarded fixed head inside the
managed project; it never writes the installed game or any save file.

### First reviewed installed schema

The installed-package browser recognizes three exact targets under
`/Game/Blueprints/TrackingSystem/FootstepsPresets`: Human, Scavenger, and Wolf.
Recognition requires one unambiguous editable `vector4_f64x4` leaf on export 0,
class `/Script/G1R.FootstepTag`, with the fully reviewed
`BoneData/BoneFeetData/FeetTextureSize` schema path. Familiar basenames,
near-match classes or paths, duplicate reviewed leaves, non-finite current
components, and unknown targets stay on the generic path.

The guided form exposes only positive finite X and Y values plus 50/100/150/200
percent presets. Z and W are shown as preserved technical components and are
carried forward byte-for-byte. The form labels all values as raw asset units;
their gameplay meaning and runtime effect are not qualified.

`authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1`
receives only the exact head, candidate ordinal, package/source snapshot seals,
and the closed schema/field/X/Y intent. It receives no target path, selector,
USMAP identity, replacement bytes, offset, receipt, output, build, or deployment
authority. Native code independently rebuilds the installed evidence, resolves
the reviewed selector, lowers X/Y while preserving Z/W, then reuses the normal
installed typed-stage executor for a second complete drift-guarded pass. The
response binds the semantic identity and before/after components separately
from the ordinary selector/replacement stage binding and the complete installed
source proof. Dart recomputes the ordinary stage binding from the prior exact
inspection and requested value before the managed session may publish.

The result is still only a managed-project stage. Build is blocked, runtime is
unqualified, native publication is unsupported, and neither the game
installation nor a save file is written.

### Direct installed fixed-leaf staging

`authoring_store_prepare_revision3_installed_dataasset_edit_v1` is the normal
Studio route from the installed-package browser. Its closed request carries the
exact managed head, original candidate ordinal, package-index/source-snapshot
seals, USMAP content/inventory seals, the three package-inspection component
seals, the canonical selector, and the typed replacement. Only the Store and
game roots are paths; no extracted artifact or receipt is created.

The retained installed extraction owns the exact in-memory pair and sidecars
plus path-free evidence for the complete opened UTOC set and every consumed
generation-relevant chunk. A second live conversion must reproduce those UTOC
identities, chunk IDs/types/winners/hashes, package bytes, sidecars, and the
selected USMAP filename and bytes. The hash of the UTOC bytes actually parsed by
the IoStore reader is checked against the held installed inventory so a
transient same-path container swap cannot be relabelled as the retained source.
Preflight and post-operation guards revalidate the installed snapshot and USMAP;
the existing final generation probe then closes the patch-to-stage handoff.

The response is the ordinary `prepared_unpublished` stage candidate plus a
closed path-free installed-source echo and a domain-separated binding over the
ordinal and all caller-visible seals. Dart recomputes that binding and validates
the source echo, target/selector shape, intent binding, candidate, manifest, and
head before the managed session may publish by exact fixed-head CAS. Source or
checkpoint drift fails closed and leaves the game and saves untouched.

## 6. Pack the patched pair without deploying it

Pack the re-inspected pair back into an additive Zen triplet. As with extract,
the output parent must exist, the complete output directory must be absent, and
paths inside the live game tree are refused.

```powershell
$packJson = & gore asset pack `
  --game $game `
  --uasset 'patched/DA_Example.uasset' `
  --patch-receipt $patchReceipt `
  --asset $asset `
  --name 'zzz_MyDataAsset_P' `
  --out 'packed' `
  --json
$pack = $packJson | ConvertFrom-Json
```

The new directory contains `zzz_MyDataAsset_P.utoc`, `.ucas`, `.pak`, and
`gore-asset-pack.json`. Packing strictly reopens the triplet before publication:
it must contain exactly the requested package and cooked path, TOC hashes must
match every output chunk, optional bulk-chunk counts must match the input
sidecars, and the companion V11 pak must be empty with the expected mount point.

Before it creates output staging, `pack` bounded-reads and deserializes the exact
hash/length-bound original `gore.asset.extract.v2` receipt. It validates the
whole envelope and cross-checks its asset, generation anchors, copied USMAP,
input pair, component list, and sidecar set against PatchReceipt v2. It also
requires the patched directory to contain exactly the sealed sidecars, maps
their roles to the original target package's bulk chunk types, and only then
re-probes the currently installed target package. The live target package/bulk
chunk hashes, concrete winning UTOCs, complete participating UTOC set, USMAP,
main UTOC, and global script-store anchors must match. A game hotfix or a new
higher-priority sibling container therefore fails with
`ASSET_GENERATION_MISMATCH`; re-extract and reapply the edit. There is no
cross-generation override.

Both `extract` and `pack` require a clean active-container view before IoStore
parsing. Any `.utoc`, `.ucas`, or `.pak` file below a `Paks` subdirectory such as
`Paks/~mods` is treated as a potentially winning deployed override and fails
with instructions to undeploy/clean the tree. The recursive no-reparse scan is
bounded to 16 levels and 4096 entries. Direct `Paks` files retain their normal
behavior, but extension aliases such as `.UTOC` fail closed; the canonical
lowercase `.utoc`, `.ucas`, and `.pak` spellings are required. The complete
direct-root mount inventory is captured at entry: UTOCs and paks are
content-sealed, while large UCAS files are held by identity. Immediately before
publication the target generation is probed again and then that inventory is
re-captured and compared exactly. Added, removed, replaced, renamed, or changed
direct-root mounts therefore fail the operation.

There is intentionally no deployment step in this workflow. `gore asset pack`
only produces an offline candidate directory; it never copies the triplet into
the game.

## What a selector seals

Format 1 carries no byte offset. Resolution recomputes the path from:

- the exact SHA-256 of both package components;
- the SHA-256 internally captured from the exact raw `.usmap` bytes;
- export index, object name, class path, component, and complete export hash;
- schema property names, declaring classes/modules, fixed-array positions, and
  stable local wire types;
- map-key kind, byte length, and hash instead of an unstable entry index;
- leaf role, fixed wire kind, and complete expected bytes.

Duplicate equal map keys make their entire branch non-editable and ambiguous.
Map keys, object/package references, `FName` values, variable-width values,
collection shape changes, header changes, and unknown wire forms remain
unsupported.

## Installed package browser

The native `gore-tex` installed-package snapshot discovers candidate `/Game`
packages from IoStore Directory Index metadata without parsing Zen package
headers or reading any `ExportBundleData` payload. It scans physical chunk
metadata under hard, caller-tightenable budgets, reproduces first-winner
priority locally, and accepts only canonical package chunk index 0 with a
winner-specific `../../../G1R/Content/*.uasset` path whose derived package ID
matches the chunk ID. Candidates are sorted deterministically.

The path-based boundary is Windows-only. Before Retoc opens the composite
container, it validates the canonical G1R layout, exact project-bound Shipping
executable, bounded direct Paks inventory, mount pairs, unsafe links/reparse
points, case collisions, nested mountables, and ambiguous sibling priority
keys. It retains exact executable, mount-inventory, package-index, and complete
source-snapshot seals and revalidates the live installation before returning.
UTOC and PAK contents are hashed. UCAS files deliberately retain only held
identity, length, and modification evidence; they are not content-hashed. The
composite source-snapshot seal covers the executable, mount inventory, and
canonical candidate index only. Selected-package bytes, sidecars, source
containers/chunk winners, USMAP, and project head are bound separately by the
inspection and installed-edit transaction described above.
Other platforms fail with `PLATFORM_UNSUPPORTED` before inspecting the supplied
tree; this avoids claiming pathname-snapshot guarantees that the current Unix
backend cannot provide.

Missing or noncanonical Directory Index paths, noncanonical ExportBundle chunk
IDs, and package-ID mismatches produce an explicit `partial_index` with reason
counts. The closed revision-3 FFI command binds that one read to the exact
managed project head and target executable. The command is
`authoring_store_read_revision3_dataasset_package_index_v1`; its closed result
states `audit_only`, `metadata_candidates_only`, `not_read`, `not_supported`,
`not_evaluated`, `runtime_unqualified`, and `not_granted`. Its strict
Dart/session/controller lane exposes the snapshot through Mod Studio's
**Browse installed packages** dialog: authors get debounced search, at most 100
lazy rendered matches, complete/partial status, a manual `/Game` fallback, and
advanced seals and counters. Refresh always requests a new exact snapshot. The
manual field validates canonical syntax only; it proves neither that a package
exists nor that it belongs to the returned snapshot, so it stays copy-only.

A listed candidate path remains discovery metadata only. Inspect resolves only
its sealed original ordinal, rebuilds the snapshot, converts the exact package
to bounded memory, selects the installed generation USMAP, and returns the
ordinary fixed-leaf report with closed authority statuses. Only a proven
`editable=true` selector can enter the typed editor. Save triggers another fresh
snapshot/USMAP verification and an independent live conversion; it never trusts
the display path as authority. Success publishes only a build-blocked managed
stage. Package construction, deployment, runtime authority, structural edits,
and writes to the installed game remain unavailable.

## Receipts, source proofs, and limits

Extract, patch, and pack form a mandatory receipt chain. Extract and pack place
their receipt in the newly published directory; patch writes its receipt beside
the new pair. `--json` prints the same JSON.

The extract receipt contains SHA-256 seals for the raw USMAP and its exact copied
artifact, global script
store, participating UTOCs, and every extracted package component, plus the
complete legacy package-pair seal. For each decompressed IoStore chunk actually
used, it records the chunk ID/type, the winning source UTOC, length, full BLAKE3,
and the BLAKE3 bytes stored in that winner's TOC. Conversion reads an immutable
verified snapshot, and all participating winner/metadata UTOC seals are checked
again before publication.

The large main UCAS is a deliberately narrower environment anchor. Its receipt
records held file identity, length, modification timestamp, and a final identity
point-check; it has `sha256: null` and `content_hash_omitted: true`. The main
UCAS is **not** content-hashed, and these checks are not a claim of safety
against adversarial concurrent same-length writes.

The patch-v2 receipt embeds the generation and exact extracted component list,
seals the original extract receipt, copied USMAP, input/output pairs, and records
the complete input/output sidecar role/name/length/hash chain. Pack requires the
original extract receipt to remain present and unchanged, deserializes it rather
than trusting only its file hash, and requires all duplicated provenance facts
to agree. Extract-v2 and patch-v2 are closed schemas: unknown fields are
rejected, every field is typed and validated, and duplicated selector, package,
USMAP, generation, path, component, sidecar, and patch-range facts must agree.
Receipt paths are bound to their canonical artifact directories, so moving only
part of an extract/patch artifact set is refused. The pack receipt seals the input package and sidecars, records the
verified global script/header source chunks and their winner UTOCs,
SHA-256-seals all three output files, and embeds the strict-reopen result
described above.

The CLI rejects data before large allocations or publication at these limits:

- raw/copied USMAP: 128 MiB; selector JSON: 4 MiB; provenance receipt: 8 MiB;
- `.uasset`: 64 MiB; `.uexp`: 256 MiB; pair aggregate: 320 MiB;
- each optional `.ubulk`, `.uptnl`, or `.m.ubulk`: 256 MiB;
- complete cooked package aggregate, including optional sidecars: 512 MiB;
- each verified IoStore chunk: 512 MiB; immutable source-snapshot aggregate:
  1 GiB.

Before `retoc` opens an IoStore, a bounded raw preflight validates the fixed
UTOC header, supported version, every allocation-driving count and checked
table size, compression blocks/methods, signatures, perfect-hash tables,
directory graph, chunk/UCAS ranges, sibling-container count, and aggregate UTOC
metadata. ContainerHeader vector counts and indirect array ranges receive the
same treatment before deserialization. Malformed inputs fail as errors; panic
catching is only a final defense, not the allocation guard.
An uncompressed method-0 block must advertise identical compressed and
uncompressed sizes, and its UCAS bounds use the same byte count that the reader
will consume.
The outer ceilings are 256 sibling containers, 256 MiB per UTOC, 128 MiB of
serialized composite metadata, 128 MiB per ContainerHeader, 500,000 TOC/package
entries, and 1.2 million compression blocks. Lower checked table/range limits
apply inside those envelopes.

The underlying repacker additionally caps an individual legacy component at
1 GiB and its bundle aggregate at 2 GiB; the public `gore asset` workflow's
512 MiB cooked-package cap is stricter.

## No-clobber, concurrency, and power-loss boundary

Extract and pack publish a complete private staging directory with an exclusive
no-replace rename. They never overwrite an existing output directory or deploy
into the game. `patch-fixed` likewise requires both destination component names,
all three possible sidecar names, and its derived receipt name to be absent. It
never modifies its source package and copies only the exact receipt-bound
sidecar set.
Virtual `/Game` paths are limited to 32 ASCII identifier segments; Win32 device
aliases (including case/extension/trailing-dot variants) and traversal are
rejected before staging. Cleanup's fixed depth ceiling remains above the
largest accepted cooked path.

Reads are bounded, sealed where stated above, and reverified with sequential
point checks; they are not a cross-file lock. Source, staging, and output
directories are trusted single-writer boundaries. A hostile process with
concurrent write or rename rights can still race path-based checks. Vanilla
opaque source files do not carry a shared semantic generation ID; the generated
receipt is this workflow's manifest, not a cross-file lock. The active-container
tree is checked at entry and exactly re-inventoried after the final generation
probe. Publication follows that last check, but the small check-to-rename window
still relies on the documented single-writer game tree. Do not run another
writer against these directories during an operation.

Receipts are hash-bound manifests, not digital signatures. They detect ordinary
drift, hotfixes, accidental mixing, and post-receipt file changes; an attacker
who can rewrite every artifact and every receipt remains outside this trust
model. Encrypted or multi-part IoStore layouts are not accepted by this bounded
G1R workflow and fail closed.

Files and directories are flushed around publication, but this is not a formal
end-to-end power-loss guarantee: filesystem, storage-controller, and hardware
durability semantics still apply. Patch sidecars and `.uexp` precede the final
`.uasset` commit marker and can be left orphaned if interrupted, as described
above, but this does not promise survival across sudden power loss. Sidecar and
receipt publication uses same-directory temporary files plus no-replace
promotion. Ownership cleanup is armed immediately after each successful
promotion, before any later fallible sync or verification; ordinary failures
remove invocation-owned outputs and temporary files so the same command can be
retried. Forced process termination and power loss remain outside RAII cleanup.

## Verification coverage

The tracked integration test creates a fictional UE5.4 package and raw USMAP at
runtime, then proves inspect -> selector -> patch -> reopen/reinspect. It also
proves source preservation, exact byte locality, stale-selector rejection,
in-place rejection, receipt/USMAP/pair generation mismatch rejection, all three
sidecar-role copies, missing/mutated/extra-sidecar rejection, chained-receipt
deserialization/cross-checking, active `~mods` and uppercase-extension rejection,
public 64/256/320 MiB limits, hostile path rejection without staging residue,
and existing-output no-clobber behavior. Tiny malicious UTOC/ContainerHeader
tests exercise oversized counts, invalid fixed fields/chunk types, indirect
ranges, duplicate sibling ContainerHeader IDs, and legal zero-length chunks
without panics or large allocation. Additional adversarial cases mutate every
formerly unchecked patch-v2 proof, nested extract source proofs, and unknown
fields; inject post-publication sidecar/receipt failures; add a mount triplet
between the final probe and inventory check; and reject a method-0 block whose
two advertised sizes disagree.

The current real-fixture proof for `DA_WolfFootsteps` changed one finite lane in
the validated `Vector4` leaf at
`/BoneData/struct:BoneFeetData/FeetTextureSize`. The `.uasset` stayed
byte-identical; exactly one `.uexp` byte changed, the result reopened and walked,
and both stale-selector and existing-output attempts failed without clobbering.
