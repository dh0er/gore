# Rebuilding the five NPC test bundles

These are offline source/build instructions. They do not install a bundle,
prepare saves, launch the game, or establish that a runtime test passed.

`source-manifest.json` pins the complete shared four-module campaign baseline
and both A/B source files for every variant. `base/` contains the tested 0.1.12
A/B plus the existing edited Diego and XardasTower modules. The latter two are
intentional campaign edits, not pristine copies. Each variant replaces A/B only.
The role assembler now reads this tracked baseline; it can be rerun with
`python scripts/fixtures/npc-batch-tests/roles/prepare.py`. Complete assembled
role sources are also tracked, so that regeneration is optional.

## Prerequisites

- A lawful matching game installation, original cache SHA256
  `7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511`,
  and sibling `Binds.Cache` SHA256
  `aa73402c11d4007035a2df32c55e50086a6d9c5b6da8619cdfcb4df53f02cea2`.
- A current GORE CLI with its matching qualified standalone compiler package.
  For this batch use local `work/npc-continuation/runtime/gore-palette-v4.exe`,
  SHA256 `3e16d37f2c04361e6c80764295a648837a91273c6bef792ac14990c9758c9ada`.
  It includes the two narrowly qualified material-parameter methods needed by
  the head palette and the quest availability property, and retains that authenticated
  native authority throughout selective FullGraph composition. The original 0.1.0
  voice/economy builds used `gore-current.exe`,
  SHA256 `9428ceacd052ba3f6ed98162cfd505536e57979e3ffcfe96a555fb3d2bddf005`.
  That executable, its compiler runtime, the full emitted game tree and game
  caches are **not supplied by these fixtures**. Provision the supported runtime
  for the CLI build; a Rust executable alone does not establish that prerequisite.
  Building this product CLI also requires the audited compiler catalog embedded
  through `GORE_STANDALONE_COMPILER_CATALOG_PATH` and
  `GORE_STANDALONE_COMPILER_CATALOG_SHA256`; the local `compiler/catalog.json`
  has SHA256 `de2a2820193eb81f3032e3731d469a02dbd9ea05942f0ef48ebab58966a46e64`.

## Emit, overlay, compile

Use fresh directories outside the game installation. Set `$gore` to the supplied
CLI, `$game` to the game root, and `$original` to the verified original cache
(the campaign used `G1R/Script/PrecompiledScript_Shipping.Cache.gore-bak`).
Keep its exact matching `Binds.Cache` beside it. From the repository root:

```powershell
& $gore as emit-all $original work/npc-rebuild/pristine-tree
```

Keep defaults enabled. Preserve this pristine tree and its file hashes; reuse it
for all five variants only while CLI, original cache and bindings are unchanged.
For each variant copy the complete pristine tree to its own fresh `tree/`, then
overlay the manifest's four `base` rows followed by that variant's two `sources`
rows. `source` paths are relative to this fixture directory; destination paths
are the exact `module_path` values. Check every source SHA256 before copying.

| Manifest variant | Bundle name |
| --- | --- |
| `heads` | `NpcHeadPaletteTest` |
| `voice` | `NpcNaturalVoiceTest` |
| `roles/economy` | `NpcEconomyRolesTest` |
| `roles/field` | `NpcFieldRolesTest` |
| `quest` | `NpcQuestCallbacksTest` |

The final tree must contain exactly 7,319 modules: precisely the four manifest
module paths differ from or are added to pristine; the other 7,315 source files
must remain byte-exact. Freeze its hashes. Create an empty private `compiler/`
and an `artifacts/` directory, then run (substitute the variant directory):

```powershell
& $gore as compile work/npc-rebuild/VARIANT/tree --backend standalone `
  --game $game `
  --expect-base-sha256 7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511 `
  --out work/npc-rebuild/VARIANT/artifacts/test.full.Cache `
  --mini work/npc-rebuild/VARIANT/artifacts/test.mini.Cache `
  --work-dir work/npc-rebuild/VARIANT/compiler `
  --generation-receipt work/npc-rebuild/VARIANT/artifacts/generation-receipt.json
```

Require success, unchanged input hashes, four authored rows in the receipt,
7,319 full-cache modules and four mini-cache modules. The product selective
composition preserves unselected pristine modules; the campaign additionally
compares all 7,315 unselected serialized module spans byte-for-byte. A decompiled
source comparison alone is not that binary preservation proof. Retain build
receipts/readback results separately from the source manifest.

## Recorded batch builds

The original 0.1.0 voice and economy builds used the CLI invocation above. Heads, field roles and quest
used `compile_full_graph_standalone_v1_with_target` through a local Rust caller
linked against the product Release library. That caller supplies the same four
changes from a frozen 7,319-file manifest; it avoids repeating the costly original
source emission/comparison. Native compilation, selective composition, receipt
publication and the closing original-cache audit still run in the product core.
`build-checks.json` records the initial successful routes, hashes and preservation
checks. The 0.1.1 head/voice follow-up also uses the product Core route and is
recorded separately in `daylight-build-checks.json`: noon setup, 14:00/16:00
voice-routine transitions and hidden nonfunctional full-beard controls.
`source-manifest.json` pins the latest source variants.

The quest build exposed a core defect: after adding the first module, the running
cache has a different SHA and no longer authenticates the sealed native API
snapshot. FullGraph now authenticates native authority once against the original
cache and retains it through remapping and the composition guard. Script authority
still comes from the current composed cache. Ordinary callers still require their
own exact base SHA. A two-module regression and wrong-SHA/generation negatives
pass; the cache suite reported 383 passed and 19 ignored.

## Bundle payloads

Put `spec.json` beside `test.mini.Cache`. Minimal script-only shape:

```json
{
  "meta": {"name": "NpcEconomyRolesTest", "version": "0.1.0", "author": ""},
  "scripts": [{
    "op": "edit",
    "module_name": "LevelScripts.XardasTower_AI",
    "mini_cache": "test.mini.Cache"
  }]
}
```

Use the table's name for each variant. This is the existing multi-module mini
pattern: the mini carries all four authored modules; do not replace it with a
full-cache payload. Build it with:

```powershell
& $gore mod build --spec work/npc-rebuild/VARIANT/artifacts/spec.json `
  --out work/npc-rebuild/VARIANT/bundles
```

The campaign packages retain the six localization rows and five voice entries
from local `work/npc-warning-fix/artifacts/spec.json`, plus their five OGG files
from local `work/npc-warning-fix/audio/` (193,605 bytes total):

- `GORE_NPCVOICE_A_48M_01.ogg`
- `GORE_NPCVOICE_A_44M_01.ogg`
- `GORE_NPCVOICE_A_48S_01.ogg`
- `GORE_NPCVOICE_HERO_REPLY_01_padded.ogg`
- `GORE_NPCVOICE_A_END_01.ogg`

Those original campaign recordings and that local spec are **not repository
inputs**. To reproduce the campaign's audio payload exactly, obtain those local
assets and retain their original archive paths under
`german_new/GoreMods/NpcVoiceProof/{GORE_TEST_A,Hero}/`; the padded Hero file maps
to archive name `GORE_NPCVOICE_HERO_REPLY_01.ogg`. Put the files in the build's
`audio/` directory to preserve the spec's `../audio/` source paths. Keep the
intentional missing-audio localization row without adding an invented recording.

A script-only rebuild may omit this legacy audio/localization payload because
all old voice choices are hidden; record that it is **not byte-identical to the
campaign bundles**. New quest/menu text is embedded in source, natural voices
use the installed stock recordings, and head controls use installed stock assets.
Optional displayed NPC-name localization must be recorded as a bundle payload;
it must not rename the A/B/C technical identities. Inspect the resulting bundle
and verify its four script modules and declared payloads before handing it off
for any separately authorized game test.
