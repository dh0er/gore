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

## What is proven, and by what

Two different kinds of evidence stand behind this workflow, and they are worth
keeping apart.

The first is offline and runs on every test pass. Inspect resolves a selector,
`patch-fixed` changes exactly the addressed bytes and nothing else, the patched
pair reopens and walks, a stale selector and an existing output are both
refused, and the extract -> patch -> pack receipt chain fails closed once the
installed generation has moved. The real-fixture case changes one lane of one
`Vector4` leaf in `DA_WolfFootsteps`: the `.uasset` stays byte-identical and
exactly one `.uexp` byte differs. `pack` then reopens the triplet it just wrote
and requires the contained package, the TOC chunk hashes, and the mount point to
agree. None of that involves running the game.

The second is one sighting, and it is new. Before 2026-08-07 no edit made by
these commands had ever been observed to take effect in the game — the workflow
was trusted because the bytes and the receipts said so, and for no other reason.
On that date, on Gothic 1 Remake Steam BuildID 24539464 with `gore` built from
commit 90940340, one edit was watched. Note one wrinkle in its provenance: the
triplet had been extracted and packed two days earlier, against the containers
of the preceding build 24340829, and it still applied after the game updated.
That is worth knowing and is not the same as having been produced against the
build it ran on:

- package `/Game/UI/CoreMenus/Settings/W_SettingsRow`, extracted to a sealed
  legacy package;
- export index 44, object `SizeBox_SettingsEntry`, class `/Script/UMG.SizeBox`,
  leaf `/MinDesiredHeight`;
- one `float32`, little-endian: `00002042` (40.0f) replaced by `0000c843`
  (400.0f);
- packed as the additive Zen triplet `zzz_GoreSettingsRowTall_P`.

Because `pack` does not deploy, the triplet went into the game through
[`gore mgr`](mod-manager.md), which classified it as a foreign IoStore triplet
rather than a GORE bundle, imported it, and applied it under the load-order name
`zzz_gm000_GoreSettingsRowTall_P.*`. In game, under Main Menu ->
Einstellungen -> Anzeige, the settings rows were dramatically taller, the
dropdown controls rendered as tall rectangles, and the page layout was visibly
broken. One person looked at the screen, once. There is no screenshot, and
nothing in the test suite checks any of it. Afterwards the mod was undeployed
and `~mods\` was confirmed empty again.

Read that sighting for exactly what it is. It establishes that the engine
honours an additive container for a cooked UMG widget asset on this build, and
that one fixed-width `float32` leaf patched this way reaches the renderer. It
does not establish anything about other leaf kinds, other asset classes, or
other builds: one asset, one leaf, one float, one build, one look. Note also
that the target was a cooked UMG widget package rather than a DataAsset proper —
that is what these commands accepted in this one case, not a general statement
about which classes they cover.

Nothing in this toolkit ever observes the screen. A clean `pack` means the
container is well formed and holds the bytes you asked for; whether the game
draws anything different is a question only a launch answers.

## Related

- [Textures](textures.md) — the same additive Zen-triplet delivery for
  texture packages.
- [Bundling & deploying](bundles.md) — ship edits as one deployable mod.
- [DataAsset internals](../reference/dataassets-internals.md) — the
  implementation contracts and invariants behind these commands.
