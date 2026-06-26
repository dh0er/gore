# Virtual Texture Preview + Replace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make `gore-tex` decode VT textures to a flat preview image (any VT, layer 0) and re-tile a same-dims single-layer user image back into the VT cooked format (replace) — flowing through the existing preview/replace/deploy pipeline with no FFI/Flutter changes.

**Architecture:** New `crates/gore-tex/src/vt.rs` (VT parse/serialize/decode/re-tile). `texdata.rs` captures `VtData` in `PlatformData` (byte-faithful) and routes VT replace to `vt::retile`. `decode.rs` routes VT decode to `vt::decode_layer0`. Validated by byte-faithful round-trips + gated real-game decode/readback + in-game self-launch.

**Tech Stack:** Rust (`gore-tex`, vendored `retoc`, `texture2ddecoder` decode, `intel_tex_2` encode), real game at `D:\SteamLibrary\steamapps\common\Gothic 1 Remake`, texture index at `%LOCALAPPDATA%\gore-tools\texture_index.json`.

**Spec:** `docs/superpowers/specs/2026-06-26-virtual-texture-preview-replace-design.md` (full RE'd `FVirtualTextureBuiltData` layout + decode algorithm). Spike notes: `work/vt_spike_notes.md` (gitignored — read it; has exact field order, the GetTileData/morton algorithm, and the gotchas).

**Critical gotchas (from the spike — missing any wastes hours):** `bCooked` is a 4-byte UE bool; `bIsVirtual` is a separate i32 AFTER the empty mip array (not PackedData bit31); platform-data anchor SizeX = `pf_pos − 16`; each chunk's `FByteBulkData` in retoc legacy form is a single i32 index into the `.uasset` `FObjectDataResource` array; chunk bytes are raw BCn (CodecType=RawGPU, no decompression); `texture2ddecoder` output is 0xAARRGGBB. G1R VTs are non-legacy (`TileOffsetInChunk` empty) but parse/keep the legacy arrays verbatim.

---

## File Structure
- `crates/gore-tex/src/vt.rs` — NEW: `VtData`/`VtTileOffset`/`VtChunk` types, `parse`/`serialize_into`, `decode_layer0`, `retile`, morton helpers.
- `crates/gore-tex/src/lib.rs` — MODIFY: `pub mod vt;`.
- `crates/gore-tex/src/texdata.rs` — MODIFY: `PlatformData.vt: Option<vt::VtData>` captured+serialized byte-faithfully; `replace_texture` VT routing; expose chunk-byte resolution (reuse data-resources).
- `crates/gore-tex/src/decode.rs` — MODIFY: VT path in `parse`/`to_rgba8` (return the decoded layer-0 RGBA).

---

## Phase 1 — VT preview

### Task 1: VtData types + byte-faithful parse/serialize

**Files:** Create `crates/gore-tex/src/vt.rs`; modify `lib.rs`, `texdata.rs`.

- [ ] **Step 1: Read the spike notes.** Read `work/vt_spike_notes.md` for the EXACT field order + the chunk/legacy details. The deleted spike example `crates/gore-tex/examples/vt_spike.rs` had the working parse — reconstruct from the notes (they document every field).

- [ ] **Step 2: Define the types in `vt.rs`.**
```rust
//! UE5 cooked Virtual Texture (FVirtualTextureBuiltData) parse/serialize/decode/re-tile.
use crate::error::{Result, TexError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtTileOffset { pub width: u32, pub height: u32, pub max_address: u32,
    pub addresses: Vec<u32>, pub offsets: Vec<u32> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtChunk { pub sha: [u8;20], pub size_in_bytes: u32, pub codec_payload_size: u32,
    /// per layer: (codec_type, codec_payload_offset)
    pub per_layer: Vec<(u8,u32)>,
    /// retoc legacy form: index into the .uasset FObjectDataResource array
    pub data_resource_index: i32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtData {
    pub b_cooked: u32, pub num_layers: u32, pub width_in_blocks: u32, pub height_in_blocks: u32,
    pub tile_size: u32, pub tile_border: u32, pub tile_data_offset_per_layer: Vec<u32>,
    pub num_mips: u32, pub width: u32, pub height: u32,
    pub chunk_index_per_mip: Vec<u32>, pub base_offset_per_mip: Vec<u32>,
    pub tile_offset_data: Vec<VtTileOffset>,
    pub tile_index_per_chunk: Vec<u32>, pub tile_index_per_mip: Vec<u32>, pub tile_offset_in_chunk: Vec<u32>,
    pub layer_types: Vec<String>, pub layer_fallback_colors: Vec<[u8;16]>,
    pub chunks: Vec<VtChunk>,
}
impl VtData { pub fn is_legacy(&self) -> bool { !self.tile_offset_in_chunk.is_empty() } }
```

- [ ] **Step 3: Implement `parse` + `serialize_into`.** `pub fn parse(b: &[u8], pos: &mut usize, num_layers_hint: ...) -> Result<VtData>` reading exactly the spec layout (u32 LE fields, FString for layer_types per the existing `read_fstring`, 20-byte SHA, etc.). `pub fn serialize_into(out: &mut Vec<u8>, vt: &VtData)` re-emits byte-identically. Reuse `texdata.rs`'s existing little-endian readers / FString helpers (make them `pub(crate)` if needed). The contract: `serialize_into(parse(x)) == x` for the VT region bytes.

- [ ] **Step 4: Wire into `texdata::PlatformData`.** Add `pub vt: Option<vt::VtData>`. In `parse_at` (the VT branch currently returning `TexError::VirtualTexture`): instead, when `bIsVirtual==1`, call `vt::parse` and store it; set dims/format from the platform-data header; mark the struct as VT (mips empty). In `serialize_region`, when `vt.is_some()`, re-emit the platform-data header + `vt::serialize_into`. Keep non-VT untouched.

- [ ] **Step 5: Byte-faithful round-trip test (the oracle).** Gated `#[ignore]` in `texdata.rs` tests (mirror `roundtrip_streamed_water_byte_identical`): unpack a VT (`/Game/Assets/Characters/Creatures/Biter/...T_Biter_Armor_D` — find its exact path via the index by filtering for "Biter"/"_Armor_D", or any asset that parses as VT) → `PlatformData::parse` → assert `pd.vt.is_some()` → re-serialize the `.uexp` region → assert byte-identical to the original `.uexp` region. (`.ubulk` is unchanged by parse, so only the `.uexp` region matters here.)

- [ ] **Step 6: Run + commit.** `cargo test -p gore-tex` (fast green) + the gated round-trip once (`--ignored`). Then:
```bash
git add crates/gore-tex/src/vt.rs crates/gore-tex/src/lib.rs crates/gore-tex/src/texdata.rs
git commit -m "feat(gore-tex): parse+serialize VT FVirtualTextureBuiltData (byte-faithful)"
```
Escalation: if the VT region isn't byte-identical, STOP and report the first differing offset — the parse missed a field.

### Task 2: VT decode (layer 0 → flat RGBA)

**Files:** Modify `crates/gore-tex/src/vt.rs`, `crates/gore-tex/src/texdata.rs`, `crates/gore-tex/src/decode.rs`.

- [ ] **Step 1: Chunk-byte resolution.** VT chunk bytes are streamed in `.ubulk` (or inline in `.uexp`) via the data-resource the chunk's `data_resource_index` points at. Add a helper (in `texdata.rs`, reusing the `FObjectDataResource` parse already used by `rebuild_data_resources`): given `uasset`, `uexp`, `ubulk`, and a `data_resource_index`, return that chunk's bytes (slice `.ubulk[serial_offset..+serial_size]`, or `.uexp` if the flags say inline). Expose to `vt`.

- [ ] **Step 2: `decode_layer0`.** In `vt.rs` port the spike's algorithm:
```rust
/// Decode mip 0, layer 0, to a flat RGBA image (one u32 per pixel, 0xAARRGGBB).
pub fn decode_layer0(vt: &VtData, chunk_bytes: &[Vec<u8>], layer0_format: &str)
    -> Result<(u32 /*w*/, u32 /*h*/, Vec<u32>)>;
```
- mip 0 grid = `tile_offset_data[0].{width,height}`; bitmap = grid·tile_size.
- phys = tile_size + 2·tile_border; per-tile packed size = block-math(phys,phys,layer0_format).
- for addr in 0..max_address: `IsValidAddress` (offsets[addr] != INVALID per CUE4Parse — read the spike note for the exact sentinel); tileX/tileY via `reverse_morton2(addr)`/`reverse_morton2(addr>>1)` · tile_size.
- non-legacy GetTileData: chunk = `chunk_index_per_mip[0]`; offset = `base_offset_per_mip[0] + tile_offset_data[0].get_offset(addr)·tile_data_offset_per_layer.last() + layer0_offset(=0)`. legacy branch: per the spike note (G1R won't hit it but implement for other assets).
- decode phys×phys BCn (texture2ddecoder, by `layer0_format`), copy inner tile_size×tile_size (strip border) into the bitmap at (tileX,tileY).
Add `reverse_morton2` + `IsValidAddress` helpers per the spike note.

- [ ] **Step 3: Route `decode::parse`/`to_rgba8` for VT.** `TexInfo` for a VT should carry the decoded RGBA. Simplest: add an internal VT path — `decode::parse` detects `pd.vt.is_some()`, resolves the chunk bytes (Step 1 helper), calls `vt::decode_layer0`, and returns a `TexInfo` whose `width/height/format` reflect the VT + carrying the decoded RGBA (e.g. add a `pub(crate) decoded_rgba: Option<Vec<u32>>` to `TexInfo`, or have `to_rgba8` re-call decode for VT). `to_rgba8(info)` returns the VT RGBA directly when present; non-VT path (BCn→decode) unchanged. Keep the public `TexInfo`/`to_rgba8`/`parse` signatures stable (the FFI/CLI call them).

- [ ] **Step 4: Gated decode test.** `#[ignore]` test: extract a VT (`extract_by_package_id` for the Biter VT) → assert width==4096, height==4096, the RGBA is not all-zero and not all-identical (a real image). Optionally write the PNG to temp + eyeball, or compare a downscaled hash to the spike's `work/vt_spike_preview.png`.

- [ ] **Step 5: Run + commit.** Fast suite green + the gated decode test once.
```bash
git add crates/gore-tex/src/vt.rs crates/gore-tex/src/texdata.rs crates/gore-tex/src/decode.rs
git commit -m "feat(gore-tex): decode VT layer 0 to a flat preview image"
```
After this, the mod-studio Textures tab previews VTs with no app change (verify manually later via the CLI: `gore texture extract <vt asset> -o out.png` should now succeed for a VT).

### Task 3: CLI/preview smoke + multi-layer handling

**Files:** Modify `crates/gore-tex/src/decode.rs` / `vt.rs` as needed.

- [ ] **Step 1: Multi-layer preview = layer 0.** Confirm `decode_layer0` always uses layer 0 regardless of `num_layers` (multi-layer VTs preview their base layer). Add a gated test on a multi-layer VT if one can be found (filter index for normal/MRAO packed textures); else note none found in a quick scan.
- [ ] **Step 2: CLI smoke.** `cargo run -q -p gore -- texture extract --game "D:\...\Gothic 1 Remake" "/Game/Assets/Characters/Creatures/Biter/.../T_Biter_Armor_D" -o work/vt_cli.png` (PowerShell) — confirm it writes a valid 4096² PNG (was previously a VirtualTexture error). (Use the real asset path from the index.)
- [ ] **Step 3: Commit** (if any code changed): `git commit -am "feat(gore-tex): VT preview always decodes layer 0"`. Else skip.

---

## Phase 2 — VT re-tile replace (single-layer, same-dims)

### Task 4: `vt::retile`

**Files:** Modify `crates/gore-tex/src/vt.rs`.

- [ ] **Step 1: Signature + guards.**
```rust
/// Re-tile a same-dims single-layer image into the VT format using `template` as the
/// structural blueprint (same tile grid/addresses/mips). Returns the new VtData + per-chunk bytes.
pub fn retile(new_rgba: &[u8], w: u32, h: u32, template: &VtData, layer0_format: &str)
    -> Result<(VtData, Vec<Vec<u8>>)>;
```
Guards: `template.num_layers == 1` else `TexError` "multi-layer VT replace not supported"; `w == template.width && h == template.height` else `TexError` "VT replace requires same dimensions (template WxH vs new WxH)".

- [ ] **Step 2: Implement re-tile.**
- Generate the mip pyramid for `new_rgba` (box-downsample, `template.num_mips` levels; mip dims = max(W>>i,1)).
- For each mip i and each valid address in `template.tile_offset_data[i]`: compute the tile's pixel rect (tileX/Y via morton · tile_size at that mip's grid); extract `tile_size×tile_size`; build the `phys×phys` bordered tile by **replicating the edge pixels** (clamp) into the border (match the cooker's border rule — the seam-risk; the spike notes describe the inner-copy direction, do the inverse for borders); BC-encode (intel_tex_2, `layer0_format`) → the raw packed tile bytes.
- Lay tiles per chunk in the SAME order/structure as the template (same addresses, same `chunk_index_per_mip`, same `tile_offset_data` *addresses*; offsets recomputed from the new packed-tile sizes — which equal the template's since dims+format+tile config are identical, so offsets should match the template too). Concatenate into chunk byte buffers.
- Recompute each `VtChunk.size_in_bytes` (= its chunk byte length), `base_offset_per_mip`, `tile_offset_data[*].offsets` (should equal template's for same-dims — assert they do, as a correctness check), and the per-chunk `FSHAHash` (SHA-1 over the chunk bytes — confirm the hash algorithm from the spike note / CUE4Parse `FSHAHash`).
- Keep `tile_data_offset_per_layer`, `num_mips`, grid, addresses identical to the template.
- Return `(new_vt, chunk_bytes)`.

- [ ] **Step 3: Unit-ish test (no game).** Construct a tiny synthetic single-layer `VtData` template (1 mip, 1 tile, small) + a solid-color image; `retile`; assert the returned VtData has the same structure (addresses/grid) and that decoding the produced chunk bytes via `decode_layer0` yields ~the solid color (PSNR). This validates the tile encode+border+stitch round-trips without the real game.

- [ ] **Step 4: Run + commit.**
```bash
git add crates/gore-tex/src/vt.rs
git commit -m "feat(gore-tex): re-tile a same-dims single-layer image into VT format"
```
Escalation: if `tile_offset_data.offsets`/`base_offset_per_mip` you recompute differ from the template's for the SAME dims, STOP — the layout assumption is wrong; report the diff.

### Task 5: `replace_texture` VT routing + data-resource write

**Files:** Modify `crates/gore-tex/src/texdata.rs`.

- [ ] **Step 1: Route VT in `replace_texture`.** When `orig.vt.is_some()`: load the new image (caller passes rgba+dims as today), call `vt::retile(new_rgba, w, h, orig.vt, layer0_format)`, get `(new_vt, chunk_bytes)`. Set `pd.vt = Some(new_vt)`; the new `.ubulk` = the concatenated chunk bytes (in data-resource order); update the `FObjectDataResource` sizes (reuse `rebuild_data_resources` — the chunk data_resource_index entries get the new chunk sizes/offsets); recompute the export SerialSize + ImportedSize (= new W/H, unchanged for same-dims so likely a no-op) as today. For non-VT, the existing path. Multi-layer/dims-mismatch errors bubble up from `retile`.
- [ ] **Step 2: Gated readback test** (real game): unpack a VT (`T_Biter_Armor_D`) → `replace_texture` with a same-dims (4096²) solid/obvious test image → write cooked files under the mount path → `repack_to_zen` → reopen the triplet → `extract_by_package_id`/decode → assert it decodes back to ~the test image (PSNR; same dims). This proves the full VT replace chain end-to-end short of in-game.
- [ ] **Step 3: Run + commit.**
```bash
git add crates/gore-tex/src/texdata.rs
git commit -m "feat(gore-tex): route VT replace through re-tile + data-resource rebuild"
```

---

## Phase 3 — In-game validation

### Task 6: Self-launch a re-tiled VT

**Files:** none committed (scratch under `work/`); record in `docs/superpowers/notes/2026-06-26-vt-e2e.md`.

- [ ] **Step 1: Pick a VISIBLE VT.** A VT texture visible early (e.g. a main-menu background, or a world/prop texture in the starting area, or the player/an early NPC). Use the index + `gore texture extract` to confirm it's a VT and identify a recognizable one. (If hard to find a guaranteed-visible VT, use a creature/armor VT and load a save where it's on screen, or accept the boot-no-crash signal.)
- [ ] **Step 2: Build a re-tiled mod via the CLI.** `gore texture replace --game ... <vt asset> --image <obvious 4K test png>` → `gore texture pack ...` → triplet (PowerShell). Confirm it produces a triplet (no error).
- [ ] **Step 3: Deploy + self-launch.** Drop the triplet in `~mods` (or `gore texture deploy`). Note the newest crash dir. `Start-Process` the shipping exe `-windowed -ResX=640 -ResY=360`, wait ~100s, check `ue4ss/UE4SS.log` (booted, no crash) + no new crash dir with the asset/`Bad name index`. For the VISIBLE check, computer-use screenshot the area where the VT shows (or report boot-clean if not visible). `Stop-Process`.
- [ ] **Step 4: Assess.** PASS = renders the test pattern with NO seams + no crash. SEAMS = border replication rule wrong (iterate Task 4 Step 2's border code). CRASH = read the dump (likely a size/offset/SHA mismatch in the re-tiled VT) → fix the offending table. Iterate until clean.
- [ ] **Step 5: Undeploy + record.** Remove the triplet, confirm `~mods` clean + base untouched. Write `docs/superpowers/notes/2026-06-26-vt-e2e.md` (commands, result: clean/seams/crash, what was iterated). Commit the note:
```bash
git add docs/superpowers/notes/2026-06-26-vt-e2e.md
git commit -m "docs(gore-tex): VT replace in-game validation result"
```

---

## Self-Review

- **Spec coverage:** VtData byte-faithful parse/serialize ✔ (T1); chunk-byte resolution + decode_layer0 (any VT, layer 0) ✔ (T2); multi-layer preview=layer0 ✔ (T3); retile single-layer same-dims with guards ✔ (T4); replace_texture VT routing + data-resource write ✔ (T5); in-game validation ✔ (T6). No FFI/Flutter changes (preview/replace flow through existing extract/replace/deploy) — stated. Error handling: multi-layer replace reject (T4), dims-mismatch reject (T4), seams→in-game (T6), non-RawGPU→UnsupportedFormat (decode). Same-dims only (T4 guard). Out-of-scope (multi-layer replace, upscale, convert-to-regular) per spec.
- **Placeholder scan:** the parse/serialize/decode/retile bodies reference the spec's exact layout + the spike's algorithm (in `work/vt_spike_notes.md`) rather than inlining 200 lines of byte-offset code — the byte-faithful round-trip (T1 S5) + decode (T2 S4) + readback (T5 S2) tests are the concrete oracles that pin correctness; this matches how the existing texdata parser was built+verified. The `IsValidAddress` sentinel + SHA algorithm + legacy GetTileData branch are explicitly "per the spike note" (a real, readable source in `work/`), not vague TODOs.
- **Type consistency:** `VtData`/`VtTileOffset`/`VtChunk` (T1) consumed by `decode_layer0` (T2), `retile` (T4), `replace_texture` (T5), `serialize_into` (T1). `PlatformData.vt: Option<VtData>` (T1) read by decode (T2) + replace (T5). `decode_layer0(vt, chunk_bytes, layer0_format) -> (w,h,rgba)` and `retile(new_rgba,w,h,template,layer0_format) -> (VtData, Vec<Vec<u8>>)` consistent across tasks.
