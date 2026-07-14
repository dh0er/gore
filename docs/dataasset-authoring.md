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
workflow below for the currently supported copy-on-write patch operation. A
separate managed revision-3 project view can now import the resulting verified
PatchReceipt into its stage registry; that does not turn the Lab into a semantic
editor or grant build/deploy authority. Any future semantic editor must preserve
these exact selector and provenance gates rather than treating a successfully
opened package as general write support.

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

`ManagedRevision3AuthoringProjectSession` publishes prepare and remove
candidates through the same serialized full-reopen, crash-repair, exact
fixed-head byte-CAS, and post-publication reopen lane used by other revision-3
mutations. Listing is an exact-head serialized read. Head drift never clobbers
the winner; malformed or integrity-uncertain results poison the session, while
bounded local-input/response-limit failures remain retryable after the disk
head is rechecked.

Managed revision-3 Home now exposes that exact registry as **Verified DataAsset
edits**. The author can search friendly asset names or `/Game` paths, inspect
bounded verified facts, import a PatchReceipt JSON through a file picker, and
remove an entry from the project registry after confirmation. Initial or failed
exact-head loading, checkpoint drift, and `requiresReopen` lock mutation. Add and
remove are bound to the exact project root, project ID, revision, and head; each
successful mutation advances and reloads the visible project checkpoint. A
registry removal does not modify the source receipt or game installation.

This visible surface manages independently receipt-verified fixed-size edits;
it is not yet a semantic value editor. It grants no build, pack, deploy,
gameplay, runtime, or future-reinspection authority. Semantic schema/forms,
authoring a value directly in Studio, preview/diff/undo, build lowering,
post-pack verification, structural edits, and the sealed Unreal handoff remain
separate work.

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
