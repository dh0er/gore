# gore-tex — Texture Replacement Design

**Date:** 2026-06-26
**Status:** Approved (brainstorm), pending implementation plan
**Target game:** Gothic 1 Remake (UE 5.4.3 CL169686, Steam appid 1297900, IoStore-packaged)

## Goal

Let gore replace cooked game textures in Gothic 1 Remake. Ship a pure-Rust CLI engine
first (`gore texture …`); a mod-studio GUI tab comes later (separate spec). The engine
extracts a cooked `UTexture2D` from the game's IoStore container, lets the user swap the
image (incl. higher resolution), re-encodes + splices it back, repacks an override
container, and deploys it to the game's `~mods` folder.

## Background / facts (from research, 2026-06-25)

- Textures live in `G1R/Content/Paks/G1R-Windows.ucas` + `.utoc` (IoStore, ~25.5 GB).
  Cooked `UTexture2D` → BC1/BC3/BC5/BC7. Bulk mip data = separate chunks inside the
  `.ucas` (no loose `.ubulk`).
- Container is **UNENCRYPTED + UNSIGNED** (utoc ContainerFlags=0x09 Compressed|Indexed;
  Encrypt/Sign bits clear; EncryptionKeyGuid all-zero). Only barrier = Oodle compression.
- `.usmap` already on disk: `G1R/Binaries/Win64/ue4ss/G1R-5.4.3-168781-272ce2f8.usmap`
  (needed to skip unversioned property blocks when parsing cooked assets).
- `~mods/` and `LogicMods/` folders exist (empty). Proven G1R texture mods on Nexus
  (#133, #320, 4K armor) ship as **Zen triplets** (`.utoc/.ucas/.pak`) in `~mods`.
- The game ships **no loose Oodle DLL** (`oo2core_*` absent — Oodle is static-linked into
  the shipping exe). gore already has `gore-oodle` (vendored ooz Kraken), validated
  byte-identical encode/decode vs real Oodle.
- IoStore/pak handling in the gore repo is **100% greenfield** (zero pak code today). The
  one reusable low-level block is `gore-oodle`.

See memory: `gothic-remake-texture-modding`, `goresave-ooz-codec-validation`,
`gothic-remake-loc-extraction`, `gothic-remake-fmod-audio`.

## Decisions (locked in brainstorm)

1. **Staging:** pure-Rust engine, CLI-first; GUI tab is a later phase / separate spec.
2. **Build strategy:** reuse external Rust expertise, own the novel part.
   - **retoc** (trumank, git dep) handles IoStore container read/write — **forked** so its
     Oodle backend calls `gore-oodle` instead of its built-in `oodle_loader`
     (→ no `oo2core`, no Epic DLL, no runtime download).
   - **gore-oodle** = the only Oodle provider (Kraken de/compress of `.ucas` chunks).
   - **intel_tex_2** (FFI → Intel ISPC) = BCn encode. **texture2ddecoder** (pure-Rust) = BCn
     decode for preview/extract.
   - gore **owns** the cooked-`UTexture2D` mip surgery (the genuinely novel code).
3. **CLI surface:** `list`, `extract`, `replace`, `pack`, `deploy`/`undeploy`.
4. **Replacement envelope:** allow upscale — any dimensions that are multiples of 4 (power-of-two
   when the texture is mipped); **keep the original pixel format** (no BC1→BC7 in v1); always
   **regenerate the full mip pyramid** from the new image.

## Architecture

New crate `gore-tex`. New CLI command group `gore texture <sub>` in `crates/gore`.
GUI later via `gore-ffi` `texture_*` commands.

Dependency graph:
```
gore (CLI)  ──>  gore-tex  ──>  retoc (forked)  ──>  gore-oodle (ooz Kraken)
                     │
                     ├─> intel_tex_2        (BCn encode)
                     └─> texture2ddecoder   (BCn decode, preview)
```

### Modules in `gore-tex`

- **`container`** — retoc-fork glue. List texture chunks from a `.utoc`; unpack one asset to
  loose `.uasset/.uexp/.ubulk`; pack an edited folder → legacy pak → `to-zen` triplet.
- **`texasset`** — *the novel core.* Parse a cooked `UTexture2D`: skip the unversioned property
  block (via `.usmap`) to reach `FTexturePlatformData` → read pixel format, `SizeX/SizeY`, mip
  list, and each mip's `FByteBulkData` header. Splice in new mip data and rewrite all
  bookkeeping: bulk headers (`Flags`, `ElementCount`, `SizeOnDisk`, `OffsetInFile`,
  `CookedIndex`), the export's serial size, and the summary `BulkDataStartOffset`.
  Reference implementation to study: matyalatte/UE4-DDS-Tools.
- **`encode`** — PNG → BCn via intel_tex_2 (BC1/BC3/BC5/BC7) + mip-pyramid generation
  (downsample), honoring the texture's sRGB flag.
- **`decode`** — cooked BCn → RGBA → PNG via texture2ddecoder, for `extract`/preview.
- **`error` / model** — `TexError` (thiserror); a staged-mod manifest type describing
  pending replacements (target asset path → source image, recorded format/dims).

## CLI

```
gore texture list    --container <ucas|auto> [--usmap auto] [--filter <substr>]
        → table of texture asset paths + dims + pixel format

gore texture extract <asset> -o out.png
        → decode top mip → PNG; write sidecar out.png.json (format, dims, mip count) for round-trip

gore texture replace <asset> --image new.png [--mod <dir>]
        → validate (dims mult-of-4, PoT-if-mipped), encode new BCn + regenerate mips,
          unpack original as template, splice, write result into a staged mod dir

gore texture pack    --name MyTex --mod <dir> -o <outdir>
        → build legacy pak from staged edits → retoc to-zen → zzz_MyTex_P.{utoc,ucas,pak}

gore texture deploy  <triplet|name> [--game auto]
        → copy triplet into game ~mods/ + write a deploy record (JSON)

gore texture undeploy <name> [--game auto]
        → delete the files listed in the deploy record (non-destructive — override pak)
```

Container path, `.usmap` path, and game install path are auto-resolved from the install
(reuse the gore-loc path-discovery pattern).

## Data flow (replace one texture)

```
list  →  pick asset path
      →  extract (preview PNG + sidecar)
      →  user edits PNG (may upscale)
      →  replace: unpack template via retoc-fork
                  parse cooked UTexture2D (texasset)
                  encode new BCn + regenerate mips (encode)
                  splice into template (texasset) → write to staged mod dir
      →  pack: staged folder → legacy pak → retoc to-zen → zzz_<name>_P triplet
      →  deploy: copy triplet to ~mods + record
      →  user launches game → confirm visually
```

## Deploy model

Texture mods are **additive override paks** → inherently non-destructive. No game-file is
modified in place, so no `*.gore-bak` backup is required (unlike loc/audio in-place patching).

- `deploy` = copy the triplet into `~mods/` + write a JSON deploy record (mod name → list of
  deployed file paths).
- `undeploy` = delete exactly the files named in the record. Removing the triplet fully reverts
  the game.

Naming: triplet basenames identical, `_P` suffix, high-sorting prefix (`zzz_`) so the override
out-sorts the base container.

## Error handling

- Pre-encode validation: dimensions multiple of 4; power-of-two when the texture is mipped.
  Fail with a clear, actionable message (state the offending dims + the rule).
- Unknown / unsupported pixel format → **hard error, never silent-corrupt** (the texture
  analog of the typed-parser opaque-fallback lesson: do not write bytes we don't fully model).
- `bIsVirtual` (Virtual Texture) detected → reject with "virtual texture unsupported (v1)".
- All writes via atomic staging (temp dir → rename), matching the codebase convention.

## Testing

- **Byte-faithful round-trip** (mirrors loc/fmod codecs): unpack a real cooked texture →
  parse → re-splice unchanged → byte-identical sha256. Commit a real sample asset under
  `work/` (like the decompressed save payloads) as the fixture.
- **Encode**: PNG → BC7 → decode, assert PSNR above a threshold.
- **Container**: retoc-fork unpack → pack → decode, assert equals the original (golden test).
- **E2E (manual, user-run)**: magenta-swatch in-game — the user launches the game and
  confirms the override appears. Cannot run headless.

## Phasing

- **Phase 0 — DE-RISK SPIKE (gates everything).** Hand-produce one magenta-swatch triplet
  (retoc-fork unpack + manual byte poke of one obvious UI texture + `to-zen`) → drop in
  `~mods` → **user launches and confirms the override mounts.** This kills the top risk: does a
  `~mods` Zen container actually override a base-container asset in this game? Build no real
  engine code until this passes.
- **Phase 1 — engine.** `gore-tex`: decode/list/extract first (read path, low risk), then
  encode + splice (replace/pack), then retoc-fork integration. CLI wired in `crates/gore`.
- **Phase 2 — deploy.** `deploy`/`undeploy` + deploy record.
- **Phase 3 — GUI (separate spec).** mod-studio "Textures" tab + `gore-ffi` `texture_*`
  commands; reuse `extract → PNG` as the in-app preview.

## Open risks

1. **`~mods` Zen override honoring a base-container asset** in this game — gated by the Phase 0
   spike. May require SimpleModLoader/BP-loader present and one UE4SS-registered launch; load
   order must out-sort the base.
2. **retoc-fork Oodle swap effort** — adapting retoc's `oodle_loader` call sites to `gore-oodle`'s
   ABI; confirm ooz Kraken output at the default level produces IoStore-valid chunks the game
   decompresses.
3. **usmap-driven property-skip** to locate `FTexturePlatformData` in an unversioned cooked
   asset — UE4-DDS-Tools is the reference; verify struct byte-layouts against CUE4Parse
   `IO/Objects/*.cs` before writing the splicer.
4. **Virtual Textures** scoped out of v1 (packed bordered tiles, no contiguous mip pyramid).

## Out of scope (v1)

- GUI (Phase 3, separate spec).
- Pixel-format changes (BC1→BC7 etc.).
- Virtual Texture replacement.
- Full mod management / stacking / conflict resolution (a future separate app, per the
  mod-studio unified plan).
