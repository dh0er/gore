# Cooked DataAsset fixed-leaf workflow

`gore asset` is a deliberately narrow, copy-on-write editor for proven
fixed-width leaves in legacy split Unreal packages. Its four subcommands —
`extract`, `inspect`, `patch-fixed`, and `pack` — replace one fixed-width
value inside a cooked DataAsset with new bytes of the same width. It is not
a generic DataAsset serializer, it cannot make structural edits, and it
never deploys files into the game: every step writes new files outside it.

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

The patch is a strict compare-and-swap: `--expected-hex` must equal the
leaf's complete current on-wire bytes, `--replacement-hex` must be exactly
as wide, and the input pair, USMAP, and sidecars must still match the
extract receipt. `--out` is never overwritten — the output pair, the three
possible sidecar names, and the derived receipt must be absent under an
existing parent, and the output cannot alias an input. The source pair is
never modified; keep the new pair beside its
`<output-stem>.gore-asset-patch.json` receipt, which `pack` requires.

## 5. Re-inspect the output

```powershell
gore asset inspect `
  --uasset 'patched/DA_Example.uasset' `
  --usmap $usmap `
  --json
```

Always re-inspect before a second edit and save a fresh selector. Reusing the
old selector fails on the complete pair seal before mutation.

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

## Limits

A selector addresses exactly one proven fixed-width leaf — a single value
whose byte width never changes. It carries no byte offset: it is sealed to
the exact pair and USMAP it was inspected from and goes stale as soon as
either changes, including after your own patch. Map keys, object and package
references, `FName` values, variable-width values, collection shape changes,
header changes, and unknown wire forms cannot be edited. Inputs are
size-capped (64 MiB `.uasset`, 256 MiB `.uexp`, 512 MiB per complete cooked
package); anything larger or malformed is rejected before output is written.

## Related

- [Textures](textures.md) — the same additive Zen-triplet delivery for
  texture packages.
- [Bundling & deploying](bundles.md) — ship edits as one deployable mod.
- [DataAsset internals](../reference/dataassets-internals.md) — the
  implementation contracts and invariants behind these commands.
