# Cooked DataAsset fixed-leaf workflow

`gore asset` is a deliberately narrow, copy-on-write editor for proven
fixed-width leaves in legacy split Unreal packages. It is not a generic
DataAsset serializer and it does not deploy files into the game.

The public commands are:

```text
gore asset inspect
gore asset patch-fixed
```

## 1. Inspect a package pair

The input is a `.uasset`; its sibling `.uexp` and the exact raw `.usmap` used
for this game build are required.

```powershell
$inspectJson = & gore asset inspect `
  --uasset 'DA_Example.uasset' `
  --usmap 'G1R-current.usmap' `
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

## 2. Prepare one raw fixed-width replacement

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

## 3. Patch to a new pair

```powershell
New-Item -ItemType Directory -Force 'patched' | Out-Null

gore asset patch-fixed `
  --uasset 'DA_Example.uasset' `
  --usmap 'G1R-current.usmap' `
  --selector 'selector.json' `
  --expected-hex $expected `
  --replacement-hex $replacement `
  --out 'patched/DA_Example.uasset' `
  --json
```

The source pair is never modified. The output parent must already exist, both
output names must be absent, and the output cannot alias either input
component. The output is staged and verified, then `.uexp` is published first
and `.uasset` last as the visible commit marker. If the process stops after a
completed payload-publication step but before commit-marker publication, it can
leave an orphan `.uexp`, but not a `.uasset` claiming that process-incomplete
pair. This ordering is not an OS-crash or power-loss durability guarantee.

The result calls the embedded selector `input_selector` because it is sealed to
the old pair. It is intentionally stale as soon as the output changes.

## 4. Re-inspect the output

```powershell
gore asset inspect `
  --uasset 'patched/DA_Example.uasset' `
  --usmap 'G1R-current.usmap' `
  --json
```

Always re-inspect before a second edit and save a fresh selector. Reusing the
old selector fails on the complete pair seal before mutation.

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

## Local concurrency boundary

Reads are bounded and reverified with sequential point checks; they are not a
cross-file lock. Two already-stable opaque vanilla files cannot prove that they
came from one semantic generation without a shared manifest. Source and output
directories are single-writer trusted boundaries: a hostile process with
concurrent write or rename rights can race the final path-based checks. Do not
run another writer against either package directory during a patch.

## Verification coverage

The tracked integration test creates a fictional UE5.4 package and raw USMAP at
runtime, then proves inspect -> selector -> patch -> reopen/reinspect. It also
proves source preservation, exact byte locality, stale-selector rejection,
in-place rejection, and existing-output no-clobber behavior.

The current real-fixture proof for `DA_WolfFootsteps` changed one finite lane in
the validated `Vector4` leaf at
`/BoneData/struct:BoneFeetData/FeetTextureSize`. The `.uasset` stayed
byte-identical; exactly one `.uexp` byte changed, the result reopened and walked,
and both stale-selector and existing-output attempts failed without clobbering.
