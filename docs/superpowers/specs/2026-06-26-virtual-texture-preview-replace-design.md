# Virtual Texture Preview + Replace — Design

**Date:** 2026-06-26
**Status:** Approved (brainstorm), pending implementation plan
**Builds on:** the `gore-tex` texture engine + the mod-studio Textures tab (both shipped). A feasibility spike (2026-06-26) proved VT *preview* end-to-end (decoded a real 4096² G1R VT to a clean flat PNG) and RE'd the exact UE5.4 `FVirtualTextureBuiltData` layout.

## Goal

Make `gore-tex` handle UE5 **Virtual Textures** (VT) — ~77% of G1R textures, currently rejected with `TexError::VirtualTexture`. **Preview** any VT (decode layer 0 → flat image, so the Textures tab shows everything). **Replace** a single-layer VT with a same-dimensions user image (re-tile it back into the VT cooked format). Both flow through the existing preview/replace/bundle pipeline with no FFI/Flutter changes.

## Background — the VT cooked format (RE'd, verified vs G1R UE5.4)

Cooked `FTexturePlatformData` for a VT: `i32 SizeX, i32 SizeY, u32 PackedData, FString PixelFormat, i32 FirstMipToSerialize, i32 NumMips(==0 for VT), i32 bIsVirtual(==1)`, then `FVirtualTextureBuiltData`:
```
u32  bCooked            // 4-byte UE FArchive bool (NOT 1 byte) — key gotcha
u32  NumLayers
u32  WidthInBlocks, HeightInBlocks
u32  TileSize           // e.g. 128
u32  TileBorderSize     // e.g. 4  => physical tile = TileSize + 2*Border (136)
u32[] TileDataOffsetPerLayer        // per-layer packed-tile byte size
u32  NumMips
u32  Width, Height
u32[] ChunkIndexPerMip
u32[] BaseOffsetPerMip              // per-mip byte base within its chunk
FVirtualTextureTileOffsetData[NumMips] TileOffsetData  // { u32 Width,Height,MaxAddress; u32[] Addresses; u32[] Offsets }
u32[] TileIndexPerChunk             // legacy-only; EMPTY in G1R
u32[] TileIndexPerMip               // legacy-only; EMPTY in G1R
u32[] TileOffsetInChunk             // legacy-only; EMPTY in G1R -> is_legacy=false
FString[NumLayers] LayerTypes       // e.g. ["PF_DXT1"]
FLinearColor[NumLayers] LayerFallbackColors   // 16 bytes each
FVirtualTextureDataChunk[] Chunks:
   skip 20 bytes (FSHAHash)
   u32 SizeInBytes
   u32 CodecPayloadSize
   per layer: u8 CodecType ; u32 CodecPayloadOffset
   FByteBulkData  ->  in retoc legacy form: a single i32 index into the .uasset FObjectDataResource array
```
**Key facts:** CodecType = **RawGPU (4)** on all G1R chunks → tiles are raw BCn, **no extra decompression**. Chunk bytes live streamed in `.ubulk` (data-resource flags `0x10501` = PayloadAtEndOfFile|PayloadInSeperateFile|NoOffsetFixUp), resolved via the same `FObjectDataResource` array `texdata.rs::rebuild_data_resources` already manages (chunk's i32 index → `data_resources[i]` → `.ubulk[offset..offset+size]`). A fully-inline (small) VT puts bytes in `.uexp` instead — handle both via the `PayloadInSeperateFile` flag.

**Decode (per CUE4Parse `DecodeVT`, non-legacy branch):** physical tile = TileSize+2·Border; per-tile packed size = `(phys/blockX)*(phys/blockY)*blockBytes`. mip 0 grid = `TileOffsetData[0].{Width,Height}`; bitmap = grid·TileSize. For `addr` in `0..MaxAddress`: skip if `!IsValidAddress(addr)`; `tileX = ReverseMortonCode2(addr)·TileSize`, `tileY = ReverseMortonCode2(addr>>1)·TileSize`; `GetTileData` (non-legacy) = `chunk = ChunkIndexPerMip[mip]`, `offset = BaseOffsetPerMip[mip] + TileOffsetData[mip].GetTileOffset(addr)·TileDataOffsetPerLayer.last() + layerOffset` → raw BCn slice → decode `phys×phys` → copy the inner `TileSize×TileSize` (strip the border) into the bitmap.

**Gotchas:** `bCooked`=4 bytes; `bIsVirtual` is a separate i32 after the (empty) mip array (NOT PackedData bit31); platform-data anchor SizeX = `pf_pos − 16`; chunk bulk-data in legacy form = a single i32 data-resource index; texture2ddecoder output is 0xAARRGGBB.

Spike notes (full): `work/vt_spike_notes.md` (gitignored). Reference decoder: CUE4Parse `FVirtualTextureBuiltData.cs` + `CUE4Parse-Conversion` VT decode.

## Decisions (locked in brainstorm)

1. **Full VT: preview + replace, staged** — preview first (de-risked), then re-tile replace.
2. **Replace = same-dimensions only** — re-tile the new image at the VT's exact dims + tile/mip config; same tile addresses/structure, only tile bytes + sizes/offsets/SHA change. Upscale = future.
3. **Preview = any VT (layer 0); replace = single-layer only** — multi-layer VT replace rejected with a clear message (preview still works).
4. **No FFI/Flutter changes** — preview/replace already flow through `texture_extract`/`replace_texture`/`mod_deploy`; they work once gore-tex handles VT.

## Architecture

New module `crates/gore-tex/src/vt.rs` owns VT parse / decode / re-tile. `texdata.rs` captures VT data in `PlatformData` and routes VT replace to `vt`. `decode.rs` routes VT decode to `vt`.

```
PlatformData.vt: Option<VtData>   (captured by texdata parse, byte-faithful)
   preview:  decode::to_rgba8 / extract  -> vt::decode_layer0(VtData, chunk_bytes) -> flat RGBA -> PNG
   replace:  replace_texture(orig=VT, single-layer, same dims)
                 -> vt::retile(new_rgba, VtData template) -> new VtData + chunk bytes + data-resources
                 -> serialize cooked VT -> repack_to_zen -> ~mods
```

## Components

### `vt.rs` — types
- `VtData` — every `FVirtualTextureBuiltData` field above (so it re-serializes byte-faithfully): `num_layers, width_in_blocks, height_in_blocks, tile_size, tile_border, tile_data_offset_per_layer, num_mips, width, height, chunk_index_per_mip, base_offset_per_mip, tile_offset_data: Vec<VtTileOffset>, layer_types: Vec<String>, layer_fallback_colors, chunks: Vec<VtChunk>, is_legacy + the legacy arrays (kept verbatim even if empty)`. `VtTileOffset { width, height, max_address, addresses: Vec<u32>, offsets: Vec<u32> }`. `VtChunk { sha: [u8;20], size_in_bytes, codec_payload_size, per_layer (codec_type, codec_payload_offset), data_resource_index: i32 }`.

### `vt::parse` / `vt::serialize`
- `parse(cursor, num_layers...) -> VtData` reads the layout above (handle the legacy-array emptiness → `is_legacy`). `serialize_into(buf, &VtData)` re-emits it byte-identically. Captured + re-emitted by `texdata.rs` so the byte-faithful round-trip holds.

### `vt::decode_layer0`
- `decode_layer0(vt: &VtData, chunk_bytes: &[Vec<u8>], format: &str) -> Result<(u32 w, u32 h, Vec<u32> rgba)>` — port the spike's mip-0 stitch (morton, GetTileData non-legacy + legacy branches, border strip, texture2ddecoder per the layer-0 format). `chunk_bytes[i]` resolved by `texdata` from the data-resources/`.ubulk`. Returns the flat layer-0 image.

### `vt::retile` (replace)
- `retile(new_rgba: &[u8], w: u32, h: u32, template: &VtData, format: &str) -> Result<(VtData, Vec<Vec<u8>> chunk_bytes)>` — single-layer, same-dims:
  - Require `template.num_layers == 1`, `w == template.width && h == template.height` (else error).
  - Generate the mip pyramid (down to 1×1, same `num_mips`).
  - For each mip, for each tile address present in `template.tile_offset_data[mip]`: extract the `TileSize×TileSize` region, add the `tile_border` border by **replicating/clamping edge pixels** (must match the cooker — the seam-risk), BC-encode the `phys×phys` bordered tile (intel_tex_2, layer format), place in morton order.
  - Rebuild chunks (raw BCn, CodecType=RawGPU), `SizeInBytes`, `BaseOffsetPerMip`, `TileOffsetData` offsets, `ChunkIndexPerMip`, `TileDataOffsetPerLayer`, and the per-chunk `FSHAHash` (compute over the chunk bytes). Same-dims ⇒ tile *addresses* and grid are identical to the template; only the byte payloads + sizes/offsets/SHA change. Emit the new chunk bytes for the data-resource rebuild.
  - Return the new `VtData` + chunk bytes; `texdata` writes them to `.ubulk` and updates the `FObjectDataResource` sizes via the existing `rebuild_data_resources`.

### `texdata.rs` integration
- `PlatformData.vt: Option<VtData>` captured in `parse` when `bIsVirtual`. `serialize` re-emits it. `replace_texture`: when `orig.vt.is_some()` → single-layer + same-dims check → `vt::retile` → write new chunk bytes + rebuild data-resources + recompute SerialSize/ImportedSize as today; multi-layer or dims-mismatch → clear `TexError`. Non-VT path unchanged.
- `decode::parse` builds the `TexInfo` for a VT by calling `vt::decode_layer0` (resolving chunk bytes from the data-resources) → `mip0` = the flat layer-0 RGBA's BCn? No — `to_rgba8` for VT returns the already-decoded RGBA directly (VT has no single BCn mip0). Cleanest: `decode::parse` returns a `TexInfo` flagged as VT carrying the decoded RGBA, and `to_rgba8` returns it as-is. (Keep the non-VT path returning BCn+decode as today.)

## Data flow
- **Preview:** `texture_extract` → `decode::parse` (VT) → `vt::decode_layer0` → flat RGBA → PNG. Tab unchanged.
- **Replace:** tab stages `{VT asset → PNG}` → `mod deploy` → `replace_texture` (VT branch) → `vt::retile` → cooked VT asset → `repack_to_zen` → `~mods`.

## Error handling
- Multi-layer VT **replace** → `TexError` "multi-layer VT replace not supported" (preview works, layer 0).
- New image dims ≠ VT `Width/Height` → reject (same-dims required); state both dims.
- Border replication wrong → seams in-game (the main correctness risk) → validated by the in-game self-launch.
- Legacy-data VT (non-empty legacy arrays) → preview supported (legacy branch); replace may reject if the legacy path complicates re-tile (flag if encountered).
- Non-RawGPU codec (ZippedGPU/Crunch) → `UnsupportedFormat` (absent in G1R).

## Testing
- **Byte-faithful VT round-trip** (real VT, gated `#[ignore]`): unpack a VT → `PlatformData::parse` (captures `VtData`) → `serialize` → assert `.uexp` region + `.ubulk` byte-identical to originals. The safety net before re-tiling.
- **VT decode** (gated): decode a known VT (e.g. `T_Biter_Armor_D`) → flat RGBA; assert dims (4096²) + that it's not all-zero/garbage; optionally compare to the spike's `work/vt_spike_preview.png`.
- **VT re-tile readback** (gated): `retile` a same-dims test image into a VT → `repack_to_zen` → reopen → `decode_layer0` → assert it decodes back to ~the test image (PSNR threshold; BC re-encode is lossy).
- **In-game (self-launch):** deploy a re-tiled VT with an obvious pattern (e.g. a recolored character/world texture visible early), confirm it renders without seams or crash. The ultimate oracle — iterate on borders/tables here.
- Fast suite green; existing non-VT round-trips unregressed.

## Phasing
1. **VT preview** — `vt::{VtData, parse, serialize, decode_layer0}` + `texdata` capture + `decode::parse` routing. Byte-faithful round-trip + decode tests. Tab shows all VTs. (Medium.)
2. **VT re-tile replace** — `vt::retile` (single-layer, same-dims) + `replace_texture` routing + data-resource/chunk write. Readback test. (Hard.)
3. **In-game validation** — self-launch a re-tiled VT; iterate borders/offset tables until clean.

## Out of scope (v1)
- Multi-layer VT replace; VT upscale (different dims); non-RawGPU codecs; per-material VT-sampler patching (the convert-VT-to-regular shortcut — won't work, the sampler type is baked at cook time, re-tile is the correct path).
