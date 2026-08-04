//! Byte-faithful `FVirtualTextureBuiltData` parse <-> serialize.
//!
//! A cooked UE5 **virtual texture** serializes ZERO `FTexture2DMipMap`s in the
//! `FTexturePlatformData` (so `NumMips == 0`), then an `i32 bIsVirtual == 1`,
//! then an `FVirtualTextureBuiltData` block. [`texdata`](crate::texdata) parses
//! the platform-data header up through `bIsVirtual`; this module owns everything
//! after it.
//!
//! [`parse`] captures **every** field so [`serialize_into`] reproduces the block
//! byte-for-byte. Field order/widths verified against G1R (UE5.4) — see
//! `work/vt_spike_notes.md`. Layout (all little-endian):
//!
//! ```text
//! u32 bCooked            // 4-byte UE FArchive bool (NOT 1 byte)
//! u32 NumLayers
//! u32 WidthInBlocks, HeightInBlocks
//! u32 TileSize
//! u32 TileBorderSize
//! u32[] TileDataOffsetPerLayer       // UE5.0+
//! u32 NumMips
//! u32 Width, Height
//! u32[] ChunkIndexPerMip             // UE5.0+
//! u32[] BaseOffsetPerMip             // UE5.0+
//! FVirtualTextureTileOffsetData[NumMips]:
//!   u32 Width, Height, MaxAddress
//!   u32[] Addresses
//!   u32[] Offsets
//! u32[] TileIndexPerChunk            // legacy-only (empty in non-legacy)
//! u32[] TileIndexPerMip             // legacy-only
//! u32[] TileOffsetInChunk           // legacy-only -> is_legacy() == !empty
//! FString[NumLayers] LayerTypes
//! FLinearColor[NumLayers] LayerFallbackColors    // 16 bytes each (UE5.0+)
//! FVirtualTextureDataChunk[] Chunks:
//!   [20] FSHAHash bulkDataHash       // skipped/captured verbatim (UE5.0+)
//!   u32 SizeInBytes
//!   u32 CodecPayloadSize
//!   per layer: u8 CodecType ; u32 CodecPayloadOffset
//!   FByteBulkData -> single i32 data-resource index (retoc legacy form)
//! ```
//!
//! All `TArray<u32>` fields serialize as `i32 count` then `count` u32 elements
//! (UE `FArray` length is an i32). The `Chunks` array is likewise `i32 count`
//! then the chunk structs. This is verified byte-faithful by the gated VT
//! round-trip test in [`crate::texdata`].

use crate::error::{Result, TexError};
use crate::texdata::{
    block_bytes, corrupt, rd_i32, rd_u32, read_fstring, uncompressed_bytes_per_pixel,
    write_fstring_ascii,
};

/// One `FVirtualTextureTileOffsetData` (per-mip tile address/offset tables).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtTileOffset {
    pub width: u32,
    pub height: u32,
    pub max_address: u32,
    pub addresses: Vec<u32>,
    pub offsets: Vec<u32>,
}

/// One `FVirtualTextureDataChunk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtChunk {
    /// 20-byte `FSHAHash bulkDataHash`, captured verbatim.
    pub bulk_data_hash: [u8; 20],
    pub size_in_bytes: u32,
    pub codec_payload_size: u32,
    /// Per layer: `(CodecType u8, CodecPayloadOffset u32)`. Length == `NumLayers`.
    pub codec: Vec<(u8, u32)>,
    /// `FByteBulkData` in retoc legacy form: a single i32 data-resource index.
    pub data_resource_index: i32,
}

/// A fully-captured `FVirtualTextureBuiltData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtData {
    pub b_cooked: u32,
    pub num_layers: u32,
    pub width_in_blocks: u32,
    pub height_in_blocks: u32,
    pub tile_size: u32,
    pub tile_border_size: u32,
    pub tile_data_offset_per_layer: Vec<u32>,
    pub num_mips: u32,
    pub width: u32,
    pub height: u32,
    pub chunk_index_per_mip: Vec<u32>,
    pub base_offset_per_mip: Vec<u32>,
    pub tile_offset_data: Vec<VtTileOffset>,
    /// Legacy-only arrays (empty in the non-legacy G1R form).
    pub tile_index_per_chunk: Vec<u32>,
    pub tile_index_per_mip: Vec<u32>,
    pub tile_offset_in_chunk: Vec<u32>,
    /// `FString[NumLayers]`, captured verbatim (re-emitted as ASCII FStrings).
    pub layer_types: Vec<String>,
    /// `FLinearColor[NumLayers]`, 16 bytes each, captured verbatim.
    pub layer_fallback_colors: Vec<[u8; 16]>,
    pub chunks: Vec<VtChunk>,
}

impl VtData {
    /// A VT is "legacy" iff it carries the legacy `TileOffsetInChunk` array.
    pub fn is_legacy(&self) -> bool {
        !self.tile_offset_in_chunk.is_empty()
    }
}

/// Sentinel used by UE's `FVirtualTextureTileOffsetData` to mark an address run
/// that carries no tile (a gap). A tile whose offset resolves to this value is
/// "not a valid address" and is skipped during decode.
const VT_INVALID_OFFSET: u32 = u32::MAX;

/// Maximum number of Morton addresses a single decode/re-tile operation may
/// inspect across all requested mips. A normal VT mip uses
/// `round_up_pow2(max(width, height))^2` addresses; two million therefore still
/// permits a 1024x1024 top mip and its complete mip pyramid, while bounding the
/// CPU work of extremely skewed or hostile tables.
const MAX_VT_ADDRESS_WORK: u64 = 2 * 1024 * 1024;

/// Fixed structural ceilings for counts serialized by UE arrays. These are
/// deliberately independent of the input buffer length: a large but otherwise
/// well-formed attacker-controlled buffer must not make the parser reserve an
/// unbounded amount of memory.
const MAX_VT_LAYERS: usize = 64;
const MAX_VT_MIPS: usize = 32;
const MAX_VT_CHUNKS: usize = 4 * 1024;
const MAX_VT_LAYER_TYPE_CODE_UNITS: usize = 256;

/// Sparse VT tables normally contain very few alternating present/gap runs.
/// Keep a separate ceiling so a hostile table cannot turn every address lookup
/// into a binary search over an implausibly large run vector.
const MAX_VT_OFFSET_RUNS: usize = 64 * 1024;

/// Legacy tile tables can contain one entry per address inspected by preview or
/// re-tile. Keep their serialized count within that same CPU-work budget.
const MAX_VT_LEGACY_TILE_ENTRIES: usize = MAX_VT_ADDRESS_WORK as usize;

/// Re-tile rebuilds every chunk in memory. Match the decoded-preview allocation
/// ceiling so replacement cannot allocate a larger attacker-selected aggregate
/// chunk payload than preview itself permits for RGBA output.
const MAX_VT_RETILE_CHUNK_BYTES: usize = crate::decode::MAX_DECODED_RGBA_BYTES;

impl VtTileOffset {
    /// UE `FVirtualTextureTileOffsetData::GetTileOffset`: locate the run that
    /// `address` falls in (the last `Addresses` entry `<= address`) and add the
    /// in-run delta to its base `Offsets` value. Returns `None` when the run's
    /// base is the [`VT_INVALID_OFFSET`] sentinel (an address gap).
    ///
    /// `Addresses` is sorted ascending; `Offsets[i]` is the run base for the run
    /// that starts at `Addresses[i]`.
    fn get_offset(&self, address: u32) -> Option<u32> {
        // UpperBound(Addresses, address) - 1: index of the last entry <= address.
        let block_index = match self.addresses.partition_point(|&a| a <= address) {
            0 => return None, // address precedes the first run start
            n => n - 1,
        };
        let base = *self.offsets.get(block_index)?;
        if base == VT_INVALID_OFFSET {
            return None;
        }
        base.checked_add(address - self.addresses[block_index])
    }

    /// True iff `address` maps to a real (non-gap) tile.
    fn is_valid_address(&self, address: u32) -> bool {
        self.get_offset(address).is_some()
    }
}

/// UE `FMath::ReverseMortonCode2`: gather every other bit of `x` (the bits at
/// even positions) into a packed value — the inverse of 2D Morton interleaving.
/// `ReverseMortonCode2(addr)` yields the tile's X, `(addr >> 1)` its Y.
fn reverse_morton2(mut x: u32) -> u32 {
    x &= 0x5555_5555;
    x = (x ^ (x >> 1)) & 0x3333_3333;
    x = (x ^ (x >> 2)) & 0x0f0f_0f0f;
    x = (x ^ (x >> 4)) & 0x00ff_00ff;
    x = (x ^ (x >> 8)) & 0x0000_ffff;
    x
}

/// Validate the UE `FVirtualTextureTileOffsetData` invariants that make an
/// address walk bounded and meaningful. UE initializes `MaxAddress` to the
/// square of the next power of two covering the tile grid; the padding outside
/// the real rectangle must be represented by gap runs.
fn validate_mip_address_space(mip: &VtTileOffset, context: &str, prior_work: u64) -> Result<u64> {
    if mip.width == 0 || mip.height == 0 {
        return Err(corrupt(&format!("{context}: tile geometry is zero")));
    }

    let padded_side = mip
        .width
        .max(mip.height)
        .checked_next_power_of_two()
        .ok_or_else(|| corrupt(&format!("{context}: padded tile geometry overflows")))?;
    let expected_max = (padded_side as u64)
        .checked_mul(padded_side as u64)
        .ok_or_else(|| corrupt(&format!("{context}: Morton address space overflows")))?;
    if expected_max > u32::MAX as u64 {
        return Err(corrupt(&format!(
            "{context}: Morton address space {expected_max} does not fit u32"
        )));
    }
    if mip.max_address as u64 != expected_max {
        return Err(corrupt(&format!(
            "{context}: MaxAddress {} does not match {}x{} tile geometry (expected {expected_max})",
            mip.max_address, mip.width, mip.height
        )));
    }
    // Check the aggregate bound before any loop over this mip's declared
    // address space. This matters when re-tiling walks a complete mip pyramid.
    let total_work = prior_work
        .checked_add(expected_max)
        .ok_or_else(|| corrupt(&format!("{context}: Morton address work overflows")))?;
    enforce_address_work_budget(total_work, context)?;

    if mip.addresses.len() != mip.offsets.len() {
        return Err(corrupt(&format!(
            "{context}: address/offset run counts differ ({} != {})",
            mip.addresses.len(),
            mip.offsets.len()
        )));
    }
    if mip.addresses.is_empty() {
        return Err(corrupt(&format!("{context}: offset run table is empty")));
    }
    if mip.addresses.len() > MAX_VT_OFFSET_RUNS {
        return Err(corrupt(&format!(
            "{context}: offset run count {} exceeds work budget {MAX_VT_OFFSET_RUNS}",
            mip.addresses.len()
        )));
    }
    if mip.addresses[0] != 0 {
        return Err(corrupt(&format!(
            "{context}: first offset run must start at address 0"
        )));
    }

    for (index, (&start, &base)) in mip.addresses.iter().zip(&mip.offsets).enumerate() {
        let end = mip
            .addresses
            .get(index + 1)
            .copied()
            .unwrap_or(mip.max_address);
        if start >= mip.max_address {
            return Err(corrupt(&format!(
                "{context}: offset run {index} starts outside MaxAddress"
            )));
        }
        if end <= start {
            return Err(corrupt(&format!(
                "{context}: offset run addresses are not strictly increasing"
            )));
        }
        if base != VT_INVALID_OFFSET {
            let last_delta = end - start - 1;
            base.checked_add(last_delta)
                .ok_or_else(|| corrupt(&format!("{context}: offset run {index} overflows u32")))?;
        }
    }

    // `MaxAddress` describes a padded square. Any present tile in that padding
    // is corrupt: decoding it wastes work and re-tiling would duplicate an edge
    // pixel into a tile that is not part of the declared grid.
    let mut present_tiles = 0u64;
    for address in 0..mip.max_address {
        if mip.get_offset(address).is_none() {
            continue;
        }
        let x = reverse_morton2(address);
        let y = reverse_morton2(address >> 1);
        if x >= mip.width || y >= mip.height {
            return Err(corrupt(&format!(
                "{context}: present Morton address {address} resolves outside {}x{} tile geometry",
                mip.width, mip.height
            )));
        }
        present_tiles += 1;
    }
    let geometry_tiles = (mip.width as u64) * (mip.height as u64);
    if present_tiles > geometry_tiles {
        return Err(corrupt(&format!(
            "{context}: {present_tiles} present tiles exceed {geometry_tiles} geometry tiles"
        )));
    }

    Ok(total_work)
}

fn enforce_address_work_budget(work: u64, context: &str) -> Result<()> {
    if work > MAX_VT_ADDRESS_WORK {
        return Err(corrupt(&format!(
            "{context}: Morton address work {work} exceeds budget {MAX_VT_ADDRESS_WORK}"
        )));
    }
    Ok(())
}

/// Decode a physical `phys`x`phys` tile (`bytes`) to RGBA (`0xAARRGGBB`).
///
/// Handles both block-compressed (BCn) and uncompressed (linear) layer formats:
/// the tile is sized by [`crate::decode::to_rgba8`]'s format rules
/// (`block_bytes` for BCn, `uncompressed_bytes_per_pixel` for linear), and the
/// per-pixel decode mirrors the regular-texture path (BC6H/FloatRGBA are HDR and
/// tonemapped to LDR for the preview).
fn decode_bcn_tile(bytes: &[u8], phys: usize, format: &str) -> Result<Vec<u32>> {
    if block_bytes(format).is_none() && uncompressed_bytes_per_pixel(format).is_none() {
        return Err(TexError::UnsupportedFormat(format.to_string()));
    }
    let pixels = crate::decode::bounded_preview_pixel_count(
        phys,
        phys,
        format,
        "decoded virtual-texture RGBA tile",
    )?;
    let mut out = vec![0u32; pixels];

    // Uncompressed (linear) layer formats: the tile bytes ARE the pixels.
    if let Some(bpp) = uncompressed_bytes_per_pixel(format) {
        let need = pixels
            .checked_mul(bpp as usize)
            .ok_or_else(|| corrupt("VT tile byte length overflow"))?;
        if bytes.len() < need {
            return Err(TexError::DecodeFailed {
                format: format.to_string(),
                reason: format!("{format} VT tile is {} bytes, need {need}", bytes.len()),
            });
        }
        match format {
            "PF_FloatRGBA" => {
                // 8 bytes/pixel: four 16-bit half-floats R,G,B,A. Tonemap (clamp
                // [0,1]*255) like crate::decode::to_rgba8.
                let to_u8 = |b: &[u8]| -> u32 {
                    let bits = u16::from_le_bytes([b[0], b[1]]);
                    let v = half::f16::from_bits(bits).to_f32();
                    (v.clamp(0.0, 1.0) * 255.0).round() as u32
                };
                for (i, px) in out.iter_mut().enumerate() {
                    let base = i * 8;
                    let r = to_u8(&bytes[base..base + 2]);
                    let g = to_u8(&bytes[base + 2..base + 4]);
                    let b = to_u8(&bytes[base + 4..base + 6]);
                    let a = to_u8(&bytes[base + 6..base + 8]);
                    *px = (a << 24) | (r << 16) | (g << 8) | b;
                }
            }
            "PF_B8G8R8A8" => {
                for (i, px) in out.iter_mut().enumerate() {
                    let b = bytes[i * 4] as u32;
                    let g = bytes[i * 4 + 1] as u32;
                    let r = bytes[i * 4 + 2] as u32;
                    let a = bytes[i * 4 + 3] as u32;
                    *px = (a << 24) | (r << 16) | (g << 8) | b;
                }
            }
            "PF_G8" => {
                for (i, px) in out.iter_mut().enumerate() {
                    let g = bytes[i] as u32;
                    *px = (0xFF << 24) | (g << 16) | (g << 8) | g;
                }
            }
            _ => return Err(TexError::UnsupportedFormat(format.to_string())),
        }
        return Ok(out);
    }

    let res = match format {
        "PF_DXT1" => texture2ddecoder::decode_bc1(bytes, phys, phys, &mut out),
        "PF_DXT5" => texture2ddecoder::decode_bc3(bytes, phys, phys, &mut out),
        "PF_BC5" => texture2ddecoder::decode_bc5(bytes, phys, phys, &mut out),
        "PF_BC7" => texture2ddecoder::decode_bc7(bytes, phys, phys, &mut out),
        "PF_BC4" => texture2ddecoder::decode_bc4(bytes, phys, phys, &mut out),
        "PF_BC6H" => texture2ddecoder::decode_bc6(bytes, phys, phys, &mut out, false),
        _ => return Err(TexError::UnsupportedFormat(format.to_string())),
    };
    res.map_err(|reason| TexError::DecodeFailed {
        format: format.to_string(),
        reason: reason.to_string(),
    })?;
    // BC4 is single-channel: decode_bc4 writes only R (channel 2 == R in the
    // 0xAARRGGBB layout) and leaves G=B=A=0. Promote to an opaque grayscale
    // preview — splat R into G and B and force A=255 — exactly like
    // crate::decode::to_rgba8.
    if format == "PF_BC4" {
        for px in out.iter_mut() {
            let r = (*px >> 16) & 0xff;
            *px = (0xFF << 24) | (r << 16) | (r << 8) | r;
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy)]
struct Layer0PreviewLayout {
    out_w: u32,
    out_h: u32,
    out_pixels: usize,
    tile_size: usize,
    border: usize,
    phys: usize,
    packed_size: usize,
}

/// Validate every dimension used by the layer-0 preview before resolving VT
/// chunks or allocating a decoded bitmap. `decode_layer0` repeats this at its
/// public boundary and consumes the returned checked layout.
pub(crate) fn validate_layer0_preview(vt: &VtData, layer0_format: &str) -> Result<()> {
    layer0_preview_layout(vt, layer0_format).map(|_| ())
}

fn layer0_preview_layout(vt: &VtData, layer0_format: &str) -> Result<Layer0PreviewLayout> {
    if block_bytes(layer0_format).is_none() && uncompressed_bytes_per_pixel(layer0_format).is_none()
    {
        return Err(TexError::UnsupportedFormat(layer0_format.to_string()));
    }
    if vt.num_mips == 0 || vt.tile_offset_data.is_empty() {
        return Err(corrupt("VT has no mips to decode"));
    }
    let mip = &vt.tile_offset_data[0];
    if vt.tile_size == 0 || mip.width == 0 || mip.height == 0 {
        return Err(corrupt("VT mip-0 tile geometry is zero"));
    }

    let physical_tile_size = vt
        .tile_border_size
        .checked_mul(2)
        .and_then(|border| vt.tile_size.checked_add(border))
        .ok_or_else(|| corrupt("VT physical tile size overflow"))?;
    let phys = usize::try_from(physical_tile_size)
        .map_err(|_| corrupt("VT physical tile size does not fit this platform"))?;
    let tile_size = usize::try_from(vt.tile_size)
        .map_err(|_| corrupt("VT tile size does not fit this platform"))?;
    let border = usize::try_from(vt.tile_border_size)
        .map_err(|_| corrupt("VT tile border does not fit this platform"))?;

    // A corrupt VT can declare a tiny logical image with an enormous border.
    // Bound its temporary physical tile independently of the final bitmap.
    crate::decode::bounded_preview_pixel_count(
        phys,
        phys,
        layer0_format,
        "decoded virtual-texture RGBA tile",
    )?;

    let grid_px_w = mip
        .width
        .checked_mul(vt.tile_size)
        .ok_or_else(|| corrupt("VT mip-0 padded pixel width overflow"))?;
    let grid_px_h = mip
        .height
        .checked_mul(vt.tile_size)
        .ok_or_else(|| corrupt("VT mip-0 padded pixel height overflow"))?;

    // The padded grid is decoded work too. Do not let corrupt logical dimensions
    // hide an enormous tile grid behind right/bottom clipping.
    crate::decode::bounded_preview_pixel_count(
        grid_px_w as usize,
        grid_px_h as usize,
        layer0_format,
        "decoded virtual-texture padded RGBA grid",
    )?;

    let out_w = if vt.width == 0 {
        grid_px_w
    } else {
        vt.width.min(grid_px_w)
    };
    let out_h = if vt.height == 0 {
        grid_px_h
    } else {
        vt.height.min(grid_px_h)
    };
    let out_pixels = crate::decode::bounded_preview_pixel_count(
        out_w as usize,
        out_h as usize,
        layer0_format,
        "decoded virtual-texture RGBA preview",
    )?;

    // Per-tile size for the physical (bordered) tile. Keep every operation
    // checked even though the RGBA tile cap above already constrains `phys`.
    let packed_size = if let Some(bb) = block_bytes(layer0_format) {
        let blocks = phys
            .checked_add(3)
            .ok_or_else(|| corrupt("VT block-grid rounding overflow"))?
            / 4;
        blocks
            .checked_mul(blocks)
            .and_then(|value| value.checked_mul(bb as usize))
            .ok_or_else(|| corrupt("VT packed tile byte length overflow"))?
    } else if let Some(bpp) = uncompressed_bytes_per_pixel(layer0_format) {
        phys.checked_mul(phys)
            .and_then(|value| value.checked_mul(bpp as usize))
            .ok_or_else(|| corrupt("VT packed tile byte length overflow"))?
    } else {
        return Err(TexError::UnsupportedFormat(layer0_format.to_string()));
    };

    validate_mip_address_space(mip, "VT mip 0", 0)?;

    Ok(Layer0PreviewLayout {
        out_w,
        out_h,
        out_pixels,
        tile_size,
        border,
        phys,
        packed_size,
    })
}

/// Decode mip 0, layer 0, to a flat RGBA image (one `u32` per pixel,
/// `0xAARRGGBB`, the channel order [`crate::decode::to_rgba8`] documents).
///
/// `chunk_bytes[i]` is the resolved raw bytes of `vt.chunks[i]` (see
/// [`crate::texdata::resolve_data_resource_bytes`]). `layer0_format` is the UE
/// pixel-format name of layer 0 (== `vt.layer_types[0]`).
///
/// Ports the CUE4Parse `DecodeVT` mip-0/layer-0 path: walk the morton-ordered
/// tile grid of mip 0, locate each tile's raw BCn bytes via `GetTileData`, decode
/// the physical (bordered) tile, strip the `tile_border` on every side, and copy
/// the inner `tile_size`x`tile_size` block into the stitched bitmap.
///
/// Returns `(width, height, pixels)` where `width == height == grid * tile_size`.
pub fn decode_layer0(
    vt: &VtData,
    chunk_bytes: &[Vec<u8>],
    layer0_format: &str,
) -> Result<(u32, u32, Vec<u32>)> {
    let layout = layer0_preview_layout(vt, layer0_format)?;
    // Mip 0: the largest level (first TileOffsetData entry).
    const LEVEL: usize = 0;
    let mip = &vt.tile_offset_data[LEVEL];

    let Layer0PreviewLayout {
        out_w,
        out_h,
        out_pixels,
        tile_size,
        border,
        phys,
        packed_size,
    } = layout;

    // The tile grid is padded up to whole tiles; the texture's LOGICAL size
    // (vt.width/height) can be smaller (edge-padded VTs). Allocate and report the
    // logical size and clip tile copies at the right/bottom edges — otherwise the
    // preview carries padding pixels and its dimensions won't equal what `retile`
    // requires for a replacement (template.width/height), breaking the
    // export-then-replace round-trip.
    let mut bitmap = vec![0u32; out_pixels];

    // Per-tile stride across all layers (== TileDataOffsetPerLayer[NumLayers]);
    // layer 0's intra-tile offset is TileDataOffsetPerLayer[0] == 0.
    let per_tile_stride =
        *vt.tile_data_offset_per_layer
            .last()
            .ok_or_else(|| corrupt("VT TileDataOffsetPerLayer is empty"))? as u64;
    let layer0_offset: u64 = 0;

    let is_legacy = vt.is_legacy();

    for addr in 0..mip.max_address {
        if !mip.is_valid_address(addr) {
            continue;
        }
        // GetTileData: (chunk index, byte offset of this tile's layer-0 data).
        let (chunk_index, offset) = if is_legacy {
            // Legacy branch (G1R does not hit this; included for completeness).
            // tileIndex = TileIndexPerMip[level] + GetTileOffset(addr); the chunk
            // is the last TileIndexPerChunk entry <= tileIndex, and the in-chunk
            // byte offset is TileOffsetInChunk[tileIndex] + the layer offset.
            let tile_off = mip
                .get_offset(addr)
                .ok_or_else(|| corrupt("legacy VT: invalid tile after validity check"))?;
            let base_tile = *vt
                .tile_index_per_mip
                .get(LEVEL)
                .ok_or_else(|| corrupt("legacy VT: TileIndexPerMip missing level 0"))?;
            let tile_index = base_tile
                .checked_add(tile_off)
                .ok_or_else(|| corrupt("legacy VT tile index overflow"))?;
            // chunk = UpperBound(TileIndexPerChunk, tile_index) - 1.
            let chunk = match vt
                .tile_index_per_chunk
                .partition_point(|&t| t <= tile_index)
            {
                0 => return Err(corrupt("legacy VT: tile precedes first chunk")),
                n => n - 1,
            };
            let in_chunk = *vt
                .tile_offset_in_chunk
                .get(tile_index as usize)
                .ok_or_else(|| corrupt("legacy VT: TileOffsetInChunk index out of range"))?;
            let off = (in_chunk as u64)
                .checked_add(layer0_offset)
                .ok_or_else(|| corrupt("legacy VT tile byte offset overflow"))?;
            (chunk, off)
        } else {
            // Non-legacy (UE5.0+): ChunkIndexPerMip + BaseOffsetPerMip +
            // GetTileOffset(addr) * per_tile_stride + layer0 intra-tile offset.
            let chunk = *vt
                .chunk_index_per_mip
                .get(LEVEL)
                .ok_or_else(|| corrupt("VT: ChunkIndexPerMip missing level 0"))?
                as usize;
            let base = *vt
                .base_offset_per_mip
                .get(LEVEL)
                .ok_or_else(|| corrupt("VT: BaseOffsetPerMip missing level 0"))?
                as u64;
            let tile_off = mip
                .get_offset(addr)
                .ok_or_else(|| corrupt("VT: invalid tile after validity check"))?
                as u64;
            let off = tile_off
                .checked_mul(per_tile_stride)
                .and_then(|value| base.checked_add(value))
                .and_then(|value| value.checked_add(layer0_offset))
                .ok_or_else(|| corrupt("VT tile byte offset overflow"))?;
            (chunk, off)
        };

        let data = chunk_bytes
            .get(chunk_index)
            .ok_or_else(|| corrupt("VT tile references a chunk with no resolved bytes"))?;
        let start = usize::try_from(offset)
            .map_err(|_| corrupt("VT tile byte offset does not fit this platform"))?;
        let end = start
            .checked_add(packed_size)
            .ok_or_else(|| corrupt("VT tile slice overflow"))?;
        let tile = data
            .get(start..end)
            .ok_or_else(|| corrupt("VT tile bytes run past end of chunk"))?;

        let tile_x = usize::try_from(reverse_morton2(addr))
            .ok()
            .and_then(|value| value.checked_mul(tile_size))
            .ok_or_else(|| corrupt("VT tile X position overflow"))?;
        let tile_y = usize::try_from(reverse_morton2(addr >> 1))
            .ok()
            .and_then(|value| value.checked_mul(tile_size))
            .ok_or_else(|| corrupt("VT tile Y position overflow"))?;
        let ow = out_w as usize;
        let oh = out_h as usize;
        // Skip tiles wholly past the logical right/bottom edge before allocating
        // their temporary decoded buffer; clip partially visible tiles below.
        if tile_x >= ow || tile_y >= oh {
            continue;
        }
        let decoded = decode_bcn_tile(tile, phys, layer0_format)?;
        let copy_w = tile_size.min(ow - tile_x);
        for y in 0..tile_size {
            let dy = tile_y
                .checked_add(y)
                .ok_or_else(|| corrupt("VT destination row overflow"))?;
            if dy >= oh {
                break; // past the logical bottom edge
            }
            let src_row = y
                .checked_add(border)
                .and_then(|value| value.checked_mul(phys))
                .and_then(|value| value.checked_add(border))
                .ok_or_else(|| corrupt("VT source row overflow"))?;
            let dst_row = dy
                .checked_mul(ow)
                .and_then(|value| value.checked_add(tile_x))
                .ok_or_else(|| corrupt("VT destination row overflow"))?;
            let src_end = src_row
                .checked_add(copy_w)
                .ok_or_else(|| corrupt("VT source slice overflow"))?;
            let dst_end = dst_row
                .checked_add(copy_w)
                .ok_or_else(|| corrupt("VT destination slice overflow"))?;
            let source = decoded
                .get(src_row..src_end)
                .ok_or_else(|| corrupt("VT source slice exceeds decoded tile"))?;
            let destination = bitmap
                .get_mut(dst_row..dst_end)
                .ok_or_else(|| corrupt("VT destination slice exceeds preview"))?;
            destination.copy_from_slice(source);
        }
    }

    Ok((out_w, out_h, bitmap))
}

/// Box-downsample a `w`x`h` RGBA8 image to `(w/2)`x`(h/2)` by averaging each 2x2
/// texel quad per channel (round-to-nearest). Clamps the right/bottom source
/// sample for odd dims (so a 1-wide/high row degenerates to a copy). Mirrors
/// [`crate::encode`]'s `downsample_2x2` so VT mips match the regular-texture path.
fn downsample_2x2(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let w = w as usize;
    let h = h as usize;
    let nw = (w / 2).max(1);
    let nh = (h / 2).max(1);
    let mut dst = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        for x in 0..nw {
            let (x0, y0) = (x * 2, y * 2);
            let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
            let idx = |px: usize, py: usize| (py * w + px) * 4;
            let (a, b, c, d) = (idx(x0, y0), idx(x1, y0), idx(x0, y1), idx(x1, y1));
            let dpx = (y * nw + x) * 4;
            for ch in 0..4 {
                let sum = src[a + ch] as u32
                    + src[b + ch] as u32
                    + src[c + ch] as u32
                    + src[d + ch] as u32;
                dst[dpx + ch] = ((sum + 2) / 4) as u8;
            }
        }
    }
    dst
}

/// Build the `template.num_mips` mip levels from a full-res RGBA8 image.
/// Mip `i` dims = `max(w>>i,1) x max(h>>i,1)`; returned largest-first, each as a
/// `(width, height, rgba)` triple. Mip 0 is an owned copy of the input.
fn build_mip_pyramid(
    rgba: &[u8],
    w: u32,
    h: u32,
    num_mips: u32,
) -> Result<Vec<(u32, u32, Vec<u8>)>> {
    let mip_count = usize::try_from(num_mips)
        .map_err(|_| corrupt("VT re-tile: NumMips does not fit this platform"))?;
    if mip_count == 0 || mip_count > MAX_VT_MIPS {
        return Err(corrupt(&format!(
            "VT re-tile: NumMips {mip_count} exceeds limit {MAX_VT_MIPS}"
        )));
    }
    let mut out = Vec::with_capacity(mip_count);
    let mut cur = rgba.to_vec();
    let mut cw = w;
    let mut ch = h;
    for level in 0..mip_count {
        out.push((cw, ch, cur.clone()));
        if level + 1 == mip_count {
            break;
        }
        let (nw, nh) = ((cw >> 1).max(1), (ch >> 1).max(1));
        if cw == 1 && ch == 1 {
            // No further halving possible; subsequent levels (if any) repeat 1x1.
            cur = vec![cur[0], cur[1], cur[2], cur[3]];
        } else {
            cur = downsample_2x2(&cur, cw, ch);
        }
        cw = nw;
        ch = nh;
    }
    Ok(out)
}

/// Validate every output-chunk allocation and return platform-sized lengths.
/// The returned small metadata Vec is itself allocated only after the fixed
/// chunk-count ceiling has been checked.
fn checked_retile_chunk_lengths(chunks: &[VtChunk]) -> Result<Vec<usize>> {
    if chunks.len() > MAX_VT_CHUNKS {
        return Err(corrupt(&format!(
            "VT re-tile: chunk count {} exceeds limit {MAX_VT_CHUNKS}",
            chunks.len()
        )));
    }
    let mut lengths = Vec::with_capacity(chunks.len());
    let mut total_chunk_bytes = 0usize;
    for (index, chunk) in chunks.iter().enumerate() {
        let chunk_len = usize::try_from(chunk.size_in_bytes).map_err(|_| {
            corrupt(&format!(
                "VT re-tile: chunk {index} size does not fit this platform"
            ))
        })?;
        total_chunk_bytes = total_chunk_bytes
            .checked_add(chunk_len)
            .ok_or_else(|| corrupt("VT re-tile: aggregate chunk byte length overflows"))?;
        if total_chunk_bytes > MAX_VT_RETILE_CHUNK_BYTES {
            return Err(corrupt(&format!(
                "VT re-tile: aggregate chunk bytes {total_chunk_bytes} exceed budget {MAX_VT_RETILE_CHUNK_BYTES}"
            )));
        }
        lengths.push(chunk_len);
    }
    Ok(lengths)
}

/// Pure-Rust SHA-1 (FIPS 180-4). VT chunk `FSHAHash bulkDataHash` is the 20-byte
/// SHA-1 of the chunk's raw bytes (matches CUE4Parse `FSHAHash`/UE `FSHA1`).
/// Self-contained to avoid a new crate dependency for one hash.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let ml = (data.len() as u64).wrapping_mul(8);

    // Pad: 0x80, then zeros, then 64-bit big-endian bit length, to a 64-byte mult.
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Re-tile a same-dims single-layer RGBA8 image into the VT cooked format, using
/// `template` as the structural blueprint (identical tile grid/addresses/mips).
///
/// `new_rgba` is row-major RGBA8 (byte order R,G,B,A — the natural image order,
/// NOT the `0xAARRGGBB` u32s [`decode_layer0`] returns), `w`x`h` pixels. Because
/// the dimensions, format and tile config are identical to `template`, every
/// offset table (`BaseOffsetPerMip`, `TileOffsetData`, `ChunkIndexPerMip`,
/// `TileDataOffsetPerLayer`, per-chunk `SizeInBytes`) is byte-for-byte reused;
/// only the chunk *payloads* (re-encoded tile bytes) and per-chunk `FSHAHash`
/// change.
///
/// Returns `(new_vt, chunk_bytes)` where `chunk_bytes[c]` is the rebuilt raw
/// byte buffer for `new_vt.chunks[c]` (parallel to `template.chunks`, chunk
/// order). `chunk_bytes[c].len() == template.chunks[c].size_in_bytes` (asserted —
/// a mismatch means the same-dims layout assumption is wrong; it is an error).
///
/// # Border rule
/// The `tile_border`-wide margin around each tile is filled by **clamp-to-edge**
/// (replicating the nearest in-tile pixel) — the standard VT cooker rule. If the
/// cooker used a different rule, in-game seams would reveal it (Task 6).
///
/// # Errors
/// * Multi-layer template (`num_layers != 1`) — not supported.
/// * Dimension mismatch (`w != template.width || h != template.height`).
/// * Bad buffer length (`new_rgba.len() != w*h*4`).
/// * Encode failure, or a rebuilt chunk length != the template's chunk size.
pub fn retile(
    new_rgba: &[u8],
    w: u32,
    h: u32,
    template: &VtData,
    layer0_format: &str,
) -> Result<(VtData, Vec<Vec<u8>>)> {
    // --- guards -----------------------------------------------------------
    if template.num_layers != 1 {
        return Err(TexError::VirtualTexture(
            "multi-layer VT replace not supported".to_string(),
        ));
    }
    if w != template.width || h != template.height {
        return Err(TexError::VirtualTexture(format!(
            "VT re-tile requires same dimensions: new image is {w}x{h} but template is {}x{}",
            template.width, template.height
        )));
    }
    let input_pixels = crate::decode::bounded_preview_pixel_count(
        w as usize,
        h as usize,
        layer0_format,
        "VT re-tile RGBA input",
    )?;
    let expected_len = input_pixels
        .checked_mul(4)
        .ok_or_else(|| corrupt("VT re-tile: input byte length overflows"))?;
    if new_rgba.len() != expected_len {
        return Err(TexError::VirtualTexture(format!(
            "VT re-tile: rgba length {} != w*h*4 ({expected_len}) for {w}x{h}",
            new_rgba.len()
        )));
    }
    let mip_count = usize::try_from(template.num_mips)
        .map_err(|_| corrupt("VT re-tile: NumMips does not fit this platform"))?;
    if mip_count == 0 || template.tile_offset_data.is_empty() {
        return Err(corrupt("VT re-tile: template has no mips"));
    }
    if mip_count > MAX_VT_MIPS {
        return Err(corrupt(&format!(
            "VT re-tile: NumMips {mip_count} exceeds limit {MAX_VT_MIPS}"
        )));
    }
    if template.is_legacy() {
        return Err(TexError::VirtualTexture(
            "VT re-tile: legacy tile layout not supported".to_string(),
        ));
    }
    if template.tile_offset_data.len() != mip_count {
        return Err(corrupt(&format!(
            "VT re-tile: TileOffsetData count {} does not match NumMips {}",
            template.tile_offset_data.len(),
            template.num_mips
        )));
    }
    if template.chunk_index_per_mip.len() != mip_count
        || template.base_offset_per_mip.len() != mip_count
    {
        return Err(corrupt(&format!(
            "VT re-tile: per-mip index counts ({}, {}) do not match NumMips {mip_count}",
            template.chunk_index_per_mip.len(),
            template.base_offset_per_mip.len()
        )));
    }
    let mut address_work = 0u64;
    for (level, mip) in template.tile_offset_data.iter().enumerate() {
        let context = format!("VT re-tile mip {level}");
        address_work = validate_mip_address_space(mip, &context, address_work)?;
    }

    if template.tile_size == 0 {
        return Err(corrupt("VT re-tile: tile size is zero"));
    }
    let physical_tile_size = template
        .tile_border_size
        .checked_mul(2)
        .and_then(|border| template.tile_size.checked_add(border))
        .ok_or_else(|| corrupt("VT re-tile: physical tile size overflow"))?;
    let tile_size = usize::try_from(template.tile_size)
        .map_err(|_| corrupt("VT re-tile: tile size does not fit this platform"))?;
    let border = usize::try_from(template.tile_border_size)
        .map_err(|_| corrupt("VT re-tile: tile border does not fit this platform"))?;
    let phys = usize::try_from(physical_tile_size)
        .map_err(|_| corrupt("VT re-tile: physical tile size does not fit this platform"))?;

    // Per-tile packed BCn size for the PHYSICAL (bordered) tile — must equal the
    // template's per-tile stride (TileDataOffsetPerLayer.last(), single layer).
    let bb = block_bytes(layer0_format)
        .ok_or_else(|| TexError::UnsupportedFormat(layer0_format.to_string()))?
        as usize;
    let blocks = phys
        .checked_add(3)
        .ok_or_else(|| corrupt("VT re-tile: block-grid rounding overflow"))?
        / 4;
    let packed_size = blocks
        .checked_mul(blocks)
        .and_then(|value| value.checked_mul(bb))
        .ok_or_else(|| corrupt("VT re-tile: packed tile byte length overflow"))?;
    let physical_pixels = crate::decode::bounded_preview_pixel_count(
        phys,
        phys,
        layer0_format,
        "VT re-tile physical RGBA tile",
    )?;
    let bordered_byte_len = physical_pixels
        .checked_mul(4)
        .ok_or_else(|| corrupt("VT re-tile: bordered tile byte length overflow"))?;
    let per_tile_stride = usize::try_from(
        *template
            .tile_data_offset_per_layer
            .last()
            .ok_or_else(|| corrupt("VT re-tile: TileDataOffsetPerLayer is empty"))?,
    )
    .map_err(|_| corrupt("VT re-tile: per-tile stride does not fit this platform"))?;
    if per_tile_stride != packed_size {
        return Err(TexError::VirtualTexture(format!(
            "VT re-tile: per-tile stride {per_tile_stride} != computed packed tile size \
             {packed_size} for {layer0_format} phys {phys}x{phys} — layout assumption wrong"
        )));
    }

    // Bound both the number of allocations and their aggregate payload before
    // reserving the outer Vec or any individual chunk buffer.
    let chunk_lengths = checked_retile_chunk_lengths(&template.chunks)?;

    // Allocate one byte buffer per chunk, sized to the template's chunk size; we
    // place every tile at exactly the offset the template's tables imply.
    let mut chunk_bytes = Vec::with_capacity(chunk_lengths.len());
    for chunk_len in chunk_lengths {
        chunk_bytes.push(vec![0u8; chunk_len]);
    }

    // Build the mip pyramid once, then tile each mip per the template's grid.
    let pyramid = build_mip_pyramid(new_rgba, w, h, template.num_mips)?;
    let ts = tile_size;

    for (level, mip) in template.tile_offset_data.iter().enumerate() {
        let (mw, mh, ref img) = pyramid[level];
        let mw = usize::try_from(mw)
            .map_err(|_| corrupt("VT re-tile: mip width does not fit this platform"))?;
        let mh = usize::try_from(mh)
            .map_err(|_| corrupt("VT re-tile: mip height does not fit this platform"))?;

        let chunk_index = *template
            .chunk_index_per_mip
            .get(level)
            .ok_or_else(|| corrupt("VT re-tile: ChunkIndexPerMip missing a mip"))?
            as usize;
        let base = *template
            .base_offset_per_mip
            .get(level)
            .ok_or_else(|| corrupt("VT re-tile: BaseOffsetPerMip missing a mip"))?
            as usize;

        for addr in 0..mip.max_address {
            let tile_off = match mip.get_offset(addr) {
                Some(o) => o as usize,
                None => continue, // gap address — no tile
            };

            let tile_x = usize::try_from(reverse_morton2(addr))
                .ok()
                .and_then(|value| value.checked_mul(ts))
                .ok_or_else(|| corrupt("VT re-tile: tile X position overflow"))?;
            let tile_y = usize::try_from(reverse_morton2(addr >> 1))
                .ok()
                .and_then(|value| value.checked_mul(ts))
                .ok_or_else(|| corrupt("VT re-tile: tile Y position overflow"))?;

            // Build the phys x phys bordered RGBA tile. For each physical-tile
            // pixel, sample the mip image at the in-tile position, clamped to the
            // tile's TileSize extent AND to the mip image bounds (clamp-to-edge).
            let mut bordered = vec![0u8; bordered_byte_len];
            for py in 0..phys {
                // in-tile y in [0, tile_size): subtract the border, clamp to tile.
                let in_ty = (py as isize - border as isize).clamp(0, ts as isize - 1) as usize;
                let src_y = tile_y
                    .checked_add(in_ty)
                    .ok_or_else(|| corrupt("VT re-tile: source Y position overflow"))?
                    .min(mh.saturating_sub(1));
                for px in 0..phys {
                    let in_tx = (px as isize - border as isize).clamp(0, ts as isize - 1) as usize;
                    let src_x = tile_x
                        .checked_add(in_tx)
                        .ok_or_else(|| corrupt("VT re-tile: source X position overflow"))?
                        .min(mw.saturating_sub(1));
                    let s = src_y
                        .checked_mul(mw)
                        .and_then(|value| value.checked_add(src_x))
                        .and_then(|value| value.checked_mul(4))
                        .ok_or_else(|| corrupt("VT re-tile: source byte offset overflow"))?;
                    let d = py
                        .checked_mul(phys)
                        .and_then(|value| value.checked_add(px))
                        .and_then(|value| value.checked_mul(4))
                        .ok_or_else(|| corrupt("VT re-tile: destination byte offset overflow"))?;
                    let source_end = s
                        .checked_add(4)
                        .ok_or_else(|| corrupt("VT re-tile: source pixel end overflows"))?;
                    let destination_end = d
                        .checked_add(4)
                        .ok_or_else(|| corrupt("VT re-tile: destination pixel end overflows"))?;
                    let source = img
                        .get(s..source_end)
                        .ok_or_else(|| corrupt("VT re-tile: source pixel runs past mip"))?;
                    let destination = bordered
                        .get_mut(d..destination_end)
                        .ok_or_else(|| corrupt("VT re-tile: destination pixel runs past tile"))?;
                    destination.copy_from_slice(source);
                }
            }

            // BC-encode the bordered tile; result must be exactly packed_size.
            let packed =
                crate::encode::encode_tile(&bordered, phys as u32, phys as u32, layer0_format)?;
            if packed.len() != packed_size {
                return Err(TexError::VirtualTexture(format!(
                    "VT re-tile: encoded tile is {} bytes, expected {packed_size}",
                    packed.len()
                )));
            }

            // Place at base + tile_off*stride + layer0 (0) in this mip's chunk.
            let dst = chunk_bytes
                .get_mut(chunk_index)
                .ok_or_else(|| corrupt("VT re-tile: mip references a missing chunk"))?;
            let off = tile_off
                .checked_mul(per_tile_stride)
                .and_then(|value| base.checked_add(value))
                .ok_or_else(|| corrupt("VT re-tile: tile offset overflow"))?;
            let end = off
                .checked_add(packed_size)
                .ok_or_else(|| corrupt("VT re-tile: tile offset overflow"))?;
            if end > dst.len() {
                return Err(TexError::VirtualTexture(format!(
                    "VT re-tile: tile at chunk {chunk_index} offset {off}..{end} exceeds chunk \
                     size {} — layout assumption wrong",
                    dst.len()
                )));
            }
            dst[off..end].copy_from_slice(&packed);
        }
    }

    // Same-dims invariant: each rebuilt chunk must match the template's size.
    for (c, (buf, ch)) in chunk_bytes.iter().zip(template.chunks.iter()).enumerate() {
        if buf.len() != ch.size_in_bytes as usize {
            return Err(TexError::VirtualTexture(format!(
                "VT re-tile: rebuilt chunk {c} is {} bytes but template chunk is {} — STOP, the \
                 layout assumption is wrong",
                buf.len(),
                ch.size_in_bytes
            )));
        }
    }

    // Clone the template structure; only chunk payloads + per-chunk hash change.
    let mut new_vt = template.clone();
    for (ch, buf) in new_vt.chunks.iter_mut().zip(chunk_bytes.iter()) {
        ch.size_in_bytes = buf.len() as u32; // == template's (asserted above)
        ch.bulk_data_hash = sha1(buf); // SHA-1 over the chunk's raw bytes
                                       // codec_payload_size, codec (per-layer entries), data_resource_index all
                                       // kept from the template (identical dims/format/tile config).
    }

    Ok((new_vt, chunk_bytes))
}

/// Validate an attacker-controlled element count before it reaches
/// `Vec::with_capacity`. Both checks matter: remaining bytes prevent an
/// immediate oversized reservation from a truncated input, while the semantic
/// ceiling also bounds a genuinely large input buffer.
fn checked_count_for_remaining(
    b: &[u8],
    pos: usize,
    count: usize,
    minimum_element_bytes: usize,
    semantic_limit: usize,
    context: &str,
) -> Result<usize> {
    if count > semantic_limit {
        return Err(corrupt(&format!(
            "{context} count {count} exceeds limit {semantic_limit}"
        )));
    }
    let required = count
        .checked_mul(minimum_element_bytes)
        .ok_or_else(|| corrupt(&format!("{context} byte count overflows")))?;
    let remaining = b
        .len()
        .checked_sub(pos)
        .ok_or_else(|| corrupt(&format!("{context} starts past end")))?;
    if required > remaining {
        return Err(corrupt(&format!(
            "{context} needs at least {required} bytes for {count} elements, only {remaining} remain"
        )));
    }
    Ok(count)
}

/// Read a `TArray<u32>` at `*pos`: `i32 count` then `count` u32 elements.
fn read_u32_array(
    b: &[u8],
    pos: &mut usize,
    context: &str,
    semantic_limit: usize,
) -> Result<Vec<u32>> {
    let count = rd_i32(b, *pos)?;
    *pos = pos
        .checked_add(4)
        .ok_or_else(|| corrupt(&format!("{context} count position overflows")))?;
    if count < 0 {
        return Err(corrupt(&format!("negative {context} count in VT block")));
    }
    let n = checked_count_for_remaining(
        b,
        *pos,
        count as usize,
        std::mem::size_of::<u32>(),
        semantic_limit,
        context,
    )?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(rd_u32(b, *pos)?);
        *pos = pos
            .checked_add(4)
            .ok_or_else(|| corrupt(&format!("{context} element position overflows")))?;
    }
    Ok(out)
}

/// `read_fstring` supports unbounded UTF-16 lengths internally. VT layer names
/// are pixel-format identifiers, so preflight their serialized length here
/// before calling it and before its UTF-16 `Vec::with_capacity` path.
fn read_vt_layer_type(b: &[u8], pos: &mut usize) -> Result<String> {
    let encoded_count = rd_i32(b, *pos)?;
    let payload_pos = pos
        .checked_add(4)
        .ok_or_else(|| corrupt("VT LayerType count position overflows"))?;
    let (code_units, unit_bytes) = if encoded_count >= 0 {
        (encoded_count as usize, 1usize)
    } else {
        let positive = encoded_count
            .checked_neg()
            .ok_or_else(|| corrupt("VT LayerType code-unit count overflows"))?;
        (positive as usize, 2usize)
    };
    checked_count_for_remaining(
        b,
        payload_pos,
        code_units,
        unit_bytes,
        MAX_VT_LAYER_TYPE_CODE_UNITS,
        "VT LayerType",
    )?;
    let (value, next) = read_fstring(b, *pos)?;
    *pos = next;
    Ok(value)
}

/// Write a `TArray<u32>`: `i32 count` then `count` u32 elements.
fn write_u32_array(out: &mut Vec<u8>, v: &[u32]) {
    out.extend_from_slice(&(v.len() as i32).to_le_bytes());
    for &x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
}

/// Parse an `FVirtualTextureBuiltData` starting at `*pos` (the byte *after* the
/// platform-data `bIsVirtual` i32). Advances `*pos` past the whole block.
pub fn parse(b: &[u8], pos: &mut usize) -> Result<VtData> {
    let b_cooked = rd_u32(b, *pos)?; // 4-byte UE bool (NOT 1 byte)
    *pos += 4;
    let num_layers = rd_u32(b, *pos)?;
    *pos += 4;
    if num_layers as usize > MAX_VT_LAYERS {
        return Err(corrupt("implausible VT NumLayers"));
    }
    let width_in_blocks = rd_u32(b, *pos)?;
    *pos += 4;
    let height_in_blocks = rd_u32(b, *pos)?;
    *pos += 4;
    let tile_size = rd_u32(b, *pos)?;
    *pos += 4;
    let tile_border_size = rd_u32(b, *pos)?;
    *pos += 4;

    let tile_data_offset_per_layer =
        read_u32_array(b, pos, "VT TileDataOffsetPerLayer", MAX_VT_LAYERS)?;

    let num_mips = rd_u32(b, *pos)?;
    *pos += 4;
    if num_mips as usize > MAX_VT_MIPS {
        return Err(corrupt("implausible VT NumMips"));
    }
    let width = rd_u32(b, *pos)?;
    *pos += 4;
    let height = rd_u32(b, *pos)?;
    *pos += 4;

    let chunk_index_per_mip = read_u32_array(b, pos, "VT ChunkIndexPerMip", MAX_VT_MIPS)?;
    let base_offset_per_mip = read_u32_array(b, pos, "VT BaseOffsetPerMip", MAX_VT_MIPS)?;

    // TileOffsetData is a `TArray<FVirtualTextureTileOffsetData>`: an i32 count
    // (== NumMips for non-legacy) then the structs. The count prefix is REAL on
    // disk — omitting it desyncs the whole tail.
    let tile_offset_count = rd_i32(b, *pos)?;
    *pos += 4;
    if tile_offset_count < 0 {
        return Err(corrupt("negative VT TileOffsetData count"));
    }
    // Each entry has three u32 fields plus two empty-array count prefixes at
    // minimum (20 bytes). Validate that lower bound before reserving structs.
    let tile_offset_count = checked_count_for_remaining(
        b,
        *pos,
        tile_offset_count as usize,
        5 * std::mem::size_of::<u32>(),
        MAX_VT_MIPS,
        "VT TileOffsetData",
    )?;
    let mut tile_offset_data = Vec::with_capacity(tile_offset_count);
    for _ in 0..tile_offset_count {
        let w = rd_u32(b, *pos)?;
        *pos += 4;
        let h = rd_u32(b, *pos)?;
        *pos += 4;
        let max_address = rd_u32(b, *pos)?;
        *pos += 4;
        let addresses = read_u32_array(b, pos, "VT Addresses", MAX_VT_OFFSET_RUNS)?;
        let offsets = read_u32_array(b, pos, "VT Offsets", MAX_VT_OFFSET_RUNS)?;
        tile_offset_data.push(VtTileOffset {
            width: w,
            height: h,
            max_address,
            addresses,
            offsets,
        });
    }

    // Legacy-only arrays (empty in the non-legacy form).
    let tile_index_per_chunk = read_u32_array(b, pos, "VT TileIndexPerChunk", MAX_VT_CHUNKS)?;
    let tile_index_per_mip = read_u32_array(b, pos, "VT TileIndexPerMip", MAX_VT_MIPS)?;
    let tile_offset_in_chunk =
        read_u32_array(b, pos, "VT TileOffsetInChunk", MAX_VT_LEGACY_TILE_ENTRIES)?;

    let layer_count = checked_count_for_remaining(
        b,
        *pos,
        num_layers as usize,
        std::mem::size_of::<i32>(),
        MAX_VT_LAYERS,
        "VT LayerTypes",
    )?;
    let mut layer_types = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        layer_types.push(read_vt_layer_type(b, pos)?);
    }

    checked_count_for_remaining(
        b,
        *pos,
        layer_count,
        16,
        MAX_VT_LAYERS,
        "VT LayerFallbackColors",
    )?;
    let mut layer_fallback_colors = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let end = pos
            .checked_add(16)
            .ok_or_else(|| corrupt("VT LayerFallbackColor position overflows"))?;
        let s = b
            .get(*pos..end)
            .ok_or_else(|| corrupt("VT LayerFallbackColor runs past end"))?;
        let mut c = [0u8; 16];
        c.copy_from_slice(s);
        layer_fallback_colors.push(c);
        *pos = end;
    }

    // Chunks: i32 count then the chunk structs.
    let chunk_count = rd_i32(b, *pos)?;
    *pos += 4;
    if chunk_count < 0 {
        return Err(corrupt("negative VT chunk count"));
    }
    let minimum_chunk_bytes = 32usize
        .checked_add(
            layer_count
                .checked_mul(5)
                .ok_or_else(|| corrupt("VT chunk codec byte count overflows"))?,
        )
        .ok_or_else(|| corrupt("VT minimum chunk byte count overflows"))?;
    let chunk_count = checked_count_for_remaining(
        b,
        *pos,
        chunk_count as usize,
        minimum_chunk_bytes,
        MAX_VT_CHUNKS,
        "VT Chunks",
    )?;
    let mut chunks = Vec::with_capacity(chunk_count);
    for _ in 0..chunk_count {
        let hash_slice = b
            .get(*pos..*pos + 20)
            .ok_or_else(|| corrupt("VT chunk FSHAHash runs past end"))?;
        let mut bulk_data_hash = [0u8; 20];
        bulk_data_hash.copy_from_slice(hash_slice);
        *pos += 20;

        let size_in_bytes = rd_u32(b, *pos)?;
        *pos += 4;
        let codec_payload_size = rd_u32(b, *pos)?;
        *pos += 4;

        checked_count_for_remaining(b, *pos, layer_count, 5, MAX_VT_LAYERS, "VT chunk codecs")?;
        let mut codec = Vec::with_capacity(layer_count);
        for _ in 0..layer_count {
            let codec_type = *b
                .get(*pos)
                .ok_or_else(|| corrupt("VT chunk CodecType runs past end"))?;
            *pos += 1;
            let codec_payload_offset = rd_u32(b, *pos)?;
            *pos += 4;
            codec.push((codec_type, codec_payload_offset));
        }

        let data_resource_index = rd_i32(b, *pos)?;
        *pos += 4;

        chunks.push(VtChunk {
            bulk_data_hash,
            size_in_bytes,
            codec_payload_size,
            codec,
            data_resource_index,
        });
    }

    Ok(VtData {
        b_cooked,
        num_layers,
        width_in_blocks,
        height_in_blocks,
        tile_size,
        tile_border_size,
        tile_data_offset_per_layer,
        num_mips,
        width,
        height,
        chunk_index_per_mip,
        base_offset_per_mip,
        tile_offset_data,
        tile_index_per_chunk,
        tile_index_per_mip,
        tile_offset_in_chunk,
        layer_types,
        layer_fallback_colors,
        chunks,
    })
}

/// Re-emit `vt` byte-identically into `out` (the bytes that follow the
/// platform-data `bIsVirtual` i32).
pub fn serialize_into(out: &mut Vec<u8>, vt: &VtData) -> Result<()> {
    out.extend_from_slice(&vt.b_cooked.to_le_bytes());
    out.extend_from_slice(&vt.num_layers.to_le_bytes());
    out.extend_from_slice(&vt.width_in_blocks.to_le_bytes());
    out.extend_from_slice(&vt.height_in_blocks.to_le_bytes());
    out.extend_from_slice(&vt.tile_size.to_le_bytes());
    out.extend_from_slice(&vt.tile_border_size.to_le_bytes());
    write_u32_array(out, &vt.tile_data_offset_per_layer);
    out.extend_from_slice(&vt.num_mips.to_le_bytes());
    out.extend_from_slice(&vt.width.to_le_bytes());
    out.extend_from_slice(&vt.height.to_le_bytes());
    write_u32_array(out, &vt.chunk_index_per_mip);
    write_u32_array(out, &vt.base_offset_per_mip);
    // TileOffsetData TArray: i32 count then the structs.
    out.extend_from_slice(&(vt.tile_offset_data.len() as i32).to_le_bytes());
    for t in &vt.tile_offset_data {
        out.extend_from_slice(&t.width.to_le_bytes());
        out.extend_from_slice(&t.height.to_le_bytes());
        out.extend_from_slice(&t.max_address.to_le_bytes());
        write_u32_array(out, &t.addresses);
        write_u32_array(out, &t.offsets);
    }
    write_u32_array(out, &vt.tile_index_per_chunk);
    write_u32_array(out, &vt.tile_index_per_mip);
    write_u32_array(out, &vt.tile_offset_in_chunk);
    for s in &vt.layer_types {
        write_fstring_ascii(out, s)?;
    }
    for c in &vt.layer_fallback_colors {
        out.extend_from_slice(c);
    }
    out.extend_from_slice(&(vt.chunks.len() as i32).to_le_bytes());
    for ch in &vt.chunks {
        out.extend_from_slice(&ch.bulk_data_hash);
        out.extend_from_slice(&ch.size_in_bytes.to_le_bytes());
        out.extend_from_slice(&ch.codec_payload_size.to_le_bytes());
        for &(ct, off) in &ch.codec {
            out.push(ct);
            out.extend_from_slice(&off.to_le_bytes());
        }
        out.extend_from_slice(&ch.data_resource_index.to_le_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_preview_vt(
        tile_size: u32,
        border: u32,
        grid_width: u32,
        grid_height: u32,
        logical_width: u32,
        logical_height: u32,
    ) -> VtData {
        VtData {
            b_cooked: 1,
            num_layers: 1,
            width_in_blocks: grid_width,
            height_in_blocks: grid_height,
            tile_size,
            tile_border_size: border,
            tile_data_offset_per_layer: vec![8],
            num_mips: 1,
            width: logical_width,
            height: logical_height,
            chunk_index_per_mip: vec![0],
            base_offset_per_mip: vec![0],
            tile_offset_data: vec![VtTileOffset {
                width: grid_width,
                height: grid_height,
                max_address: 1,
                addresses: vec![0],
                offsets: vec![0],
            }],
            tile_index_per_chunk: vec![],
            tile_index_per_mip: vec![],
            tile_offset_in_chunk: vec![],
            layer_types: vec!["PF_DXT1".to_string()],
            layer_fallback_colors: vec![[0; 16]],
            chunks: vec![],
        }
    }

    fn synthetic_retile_vt() -> VtData {
        let mut vt = synthetic_preview_vt(4, 0, 1, 1, 4, 4);
        vt.chunks.push(VtChunk {
            bulk_data_hash: [0; 20],
            size_in_bytes: 8,
            codec_payload_size: 0,
            codec: vec![(4, 0)],
            data_resource_index: 0,
        });
        vt
    }

    fn synthetic_chunk(size_in_bytes: u32) -> VtChunk {
        VtChunk {
            bulk_data_hash: [0; 20],
            size_in_bytes,
            codec_payload_size: 0,
            codec: vec![(4, 0)],
            data_resource_index: 0,
        }
    }

    #[test]
    fn serialized_count_preflight_enforces_remaining_limit_and_overflow() {
        let bytes = [0u8; 16];
        assert_eq!(
            checked_count_for_remaining(&bytes, 0, 4, 4, 4, "test").unwrap(),
            4
        );

        let error = checked_count_for_remaining(&bytes[..15], 0, 4, 4, 4, "test")
            .unwrap_err()
            .to_string();
        assert!(error.contains("only 15 remain"), "{error}");

        let error = checked_count_for_remaining(&bytes, 0, 5, 1, 4, "test")
            .unwrap_err()
            .to_string();
        assert!(error.contains("count 5 exceeds limit 4"), "{error}");

        let error = checked_count_for_remaining(&bytes, 0, 2, usize::MAX, usize::MAX, "test")
            .unwrap_err()
            .to_string();
        assert!(error.contains("byte count overflows"), "{error}");
    }

    #[test]
    fn u32_array_rejects_huge_and_maximum_truncated_counts_before_reserve() {
        let mut huge = i32::MAX.to_le_bytes().to_vec();
        let mut pos = 0;
        let error = read_u32_array(&huge, &mut pos, "test array", MAX_VT_OFFSET_RUNS)
            .unwrap_err()
            .to_string();
        assert!(error.contains("exceeds limit"), "{error}");

        huge[..4].copy_from_slice(&(MAX_VT_OFFSET_RUNS as i32).to_le_bytes());
        pos = 0;
        let error = read_u32_array(&huge, &mut pos, "test array", MAX_VT_OFFSET_RUNS)
            .unwrap_err()
            .to_string();
        assert!(error.contains("only 0 remain"), "{error}");
    }

    #[test]
    fn parser_rejects_huge_struct_and_chunk_counts_before_reserve() {
        let vt = synthetic_retile_vt();
        let mut bytes = Vec::new();
        serialize_into(&mut bytes, &vt).unwrap();

        // Header (24), TileDataOffsetPerLayer (8), mip header (12), and the two
        // one-element per-mip arrays (8 each) place TileOffsetData count at 60.
        let mut huge_tile_offsets = bytes.clone();
        huge_tile_offsets[60..64].copy_from_slice(&i32::MAX.to_le_bytes());
        let error = parse(&huge_tile_offsets, &mut 0).unwrap_err().to_string();
        assert!(error.contains("VT TileOffsetData count"), "{error}");
        assert!(error.contains("exceeds limit"), "{error}");

        let chunk_count_offset = bytes.len() - 4 - (20 + 4 + 4 + 5 + 4);
        let mut huge_chunks = bytes;
        huge_chunks[chunk_count_offset..chunk_count_offset + 4]
            .copy_from_slice(&i32::MAX.to_le_bytes());
        let error = parse(&huge_chunks, &mut 0).unwrap_err().to_string();
        assert!(error.contains("VT Chunks count"), "{error}");
        assert!(error.contains("exceeds limit"), "{error}");
    }

    #[test]
    fn layer_type_rejects_minimum_signed_count_without_negation_overflow() {
        let bytes = i32::MIN.to_le_bytes();
        let error = read_vt_layer_type(&bytes, &mut 0).unwrap_err().to_string();
        assert!(error.contains("code-unit count overflows"), "{error}");
    }

    #[test]
    fn retile_chunk_budget_accepts_exact_limit_and_rejects_excess() {
        let exact =
            checked_retile_chunk_lengths(&[synthetic_chunk(MAX_VT_RETILE_CHUNK_BYTES as u32)])
                .unwrap();
        assert_eq!(exact, vec![MAX_VT_RETILE_CHUNK_BYTES]);

        let over = [
            synthetic_chunk(MAX_VT_RETILE_CHUNK_BYTES as u32),
            synthetic_chunk(1),
        ];
        let error = checked_retile_chunk_lengths(&over).unwrap_err().to_string();
        assert!(error.contains("aggregate chunk bytes"), "{error}");
        assert!(error.contains("exceed budget"), "{error}");
    }

    #[test]
    fn retile_chunk_count_is_bounded_before_payload_allocation() {
        let chunks = vec![synthetic_chunk(0); MAX_VT_CHUNKS + 1];
        let error = checked_retile_chunk_lengths(&chunks)
            .unwrap_err()
            .to_string();
        assert!(error.contains("chunk count"), "{error}");
        assert!(error.contains("exceeds limit"), "{error}");
    }

    #[test]
    fn retile_rejects_physical_and_packed_tile_overflow() {
        let mut physical = synthetic_retile_vt();
        physical.width = 1;
        physical.height = 1;
        physical.tile_size = u32::MAX;
        physical.tile_border_size = 1;
        let error = retile(&[0; 4], 1, 1, &physical, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("physical tile size overflow"), "{error}");

        let mut packed = physical;
        packed.tile_border_size = 0;
        let error = retile(&[0; 4], 1, 1, &packed, "PF_DXT5")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("packed tile byte length overflow")
                || error.contains("block-grid rounding overflow")
                || error.contains("physical tile size does not fit"),
            "{error}"
        );
    }

    #[test]
    fn retile_enforces_preview_memory_budget_before_tile_allocation() {
        let mut vt = synthetic_retile_vt();
        vt.width = 1;
        vt.height = 1;
        vt.tile_size = 1;
        vt.tile_border_size = 4096;
        let error = retile(&[0; 4], 1, 1, &vt, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("physical RGBA tile exceeds 134217728 bytes"),
            "{error}"
        );
    }

    #[test]
    fn retile_rejects_excess_mip_count_before_pyramid_reserve() {
        let mut vt = synthetic_retile_vt();
        vt.num_mips = (MAX_VT_MIPS + 1) as u32;
        vt.tile_offset_data = vec![vt.tile_offset_data[0].clone(); MAX_VT_MIPS + 1];
        vt.chunk_index_per_mip = vec![0; MAX_VT_MIPS + 1];
        vt.base_offset_per_mip = vec![0; MAX_VT_MIPS + 1];
        let error = retile(&[0; 4 * 4 * 4], 4, 4, &vt, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("NumMips 33 exceeds limit 32"), "{error}");
    }

    #[test]
    fn preview_rejects_oversized_padded_grid_before_allocation() {
        // Logical clipping to 1x1 must not hide a 32768x32768 padded grid.
        let vt = synthetic_preview_vt(128, 0, 256, 256, 1, 1);
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("padded RGBA grid exceeds 134217728 bytes"),
            "{error}"
        );
    }

    #[test]
    fn preview_rejects_oversized_physical_tile_before_allocation() {
        // A hostile border can make one temporary tile enormous even when both
        // the logical output and padded grid are a single pixel.
        let vt = synthetic_preview_vt(1, 4096, 1, 1, 1, 1);
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("RGBA tile exceeds 134217728 bytes"),
            "{error}"
        );
    }

    #[test]
    fn preview_rejects_tile_and_grid_arithmetic_overflow() {
        let physical_overflow = synthetic_preview_vt(u32::MAX, 1, 1, 1, 1, 1);
        let error = validate_layer0_preview(&physical_overflow, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("physical tile size overflow"), "{error}");

        let mut grid_overflow = synthetic_preview_vt(2, 0, 1, 1, 1, 1);
        grid_overflow.tile_offset_data[0].width = u32::MAX;
        let error = validate_layer0_preview(&grid_overflow, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("padded pixel width overflow"), "{error}");
    }

    #[test]
    fn physical_tile_decoder_enforces_preview_cap_at_its_boundary() {
        let error = decode_bcn_tile(&[], 8192, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("RGBA tile exceeds 134217728 bytes"),
            "{error}"
        );
    }

    #[test]
    fn preview_rejects_max_address_unrelated_to_geometry_before_walk() {
        let mut vt = synthetic_preview_vt(1, 0, 1, 1, 1, 1);
        vt.tile_offset_data[0].max_address = u32::MAX;
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("MaxAddress 4294967295 does not match 1x1 tile geometry (expected 1)"),
            "{error}"
        );
    }

    #[test]
    fn preview_rejects_morton_address_space_over_work_budget() {
        // UE's padded Morton space for a 1x2048 grid is 2048^2 addresses even
        // though the real geometry contains only 2048 tiles.
        let mut vt = synthetic_preview_vt(1, 0, 1, 2048, 1, 2048);
        vt.tile_offset_data[0].max_address = 2048 * 2048;
        vt.tile_offset_data[0].offsets[0] = VT_INVALID_OFFSET;
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("Morton address work 4194304 exceeds budget 2097152"),
            "{error}"
        );
    }

    #[test]
    fn preview_rejects_present_tiles_in_morton_padding() {
        // A 1x2 grid has a padded 2x2 Morton address space. Address 1 resolves
        // to (1, 0), outside the declared one-tile-wide geometry.
        let mut vt = synthetic_preview_vt(1, 0, 1, 2, 1, 2);
        vt.tile_offset_data[0].max_address = 4;
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("present Morton address 1 resolves outside 1x2 tile geometry"),
            "{error}"
        );
    }

    #[test]
    fn preview_rejects_malformed_offset_runs() {
        let mut vt = synthetic_preview_vt(1, 0, 1, 1, 1, 1);
        vt.tile_offset_data[0].addresses = vec![0, 0];
        vt.tile_offset_data[0].offsets = vec![0, 0];
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("offset run addresses are not strictly increasing"),
            "{error}"
        );

        vt.tile_offset_data[0].addresses = vec![0];
        vt.tile_offset_data[0].offsets.clear();
        let error = decode_layer0(&vt, &[], "PF_DXT1").unwrap_err().to_string();
        assert!(
            error.contains("address/offset run counts differ"),
            "{error}"
        );
    }

    #[test]
    fn address_validation_preserves_sparse_rectangular_padding() {
        // Morton addresses 0 and 2 are the real tiles (0,0) and (0,1); 1 and 3
        // are padding for the power-of-two 2x2 address square.
        let mip = VtTileOffset {
            width: 1,
            height: 2,
            max_address: 4,
            addresses: vec![0, 1, 2, 3],
            offsets: vec![0, VT_INVALID_OFFSET, 1, VT_INVALID_OFFSET],
        };
        assert_eq!(validate_mip_address_space(&mip, "test mip", 0).unwrap(), 4);
    }

    #[test]
    fn retile_rejects_morton_address_space_over_work_budget() {
        let mut vt = synthetic_preview_vt(1, 0, 1, 2048, 1, 2048);
        vt.tile_offset_data[0].max_address = 2048 * 2048;
        vt.tile_offset_data[0].offsets[0] = VT_INVALID_OFFSET;
        let rgba = vec![0u8; 2048 * 4];
        let error = retile(&rgba, 1, 2048, &vt, "PF_DXT1")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("Morton address work 4194304 exceeds budget 2097152"),
            "{error}"
        );
    }

    #[test]
    fn reverse_morton2_unpacks_even_bits() {
        // Interleave x,y into a morton code, then verify the reverse extracts them.
        fn morton(x: u32, y: u32) -> u32 {
            let mut c = 0u32;
            for i in 0..16 {
                c |= ((x >> i) & 1) << (2 * i);
                c |= ((y >> i) & 1) << (2 * i + 1);
            }
            c
        }
        for (x, y) in [(0, 0), (1, 0), (0, 1), (3, 5), (31, 17), (255, 128)] {
            let c = morton(x, y);
            assert_eq!(reverse_morton2(c), x, "X for ({x},{y})");
            assert_eq!(reverse_morton2(c >> 1), y, "Y for ({x},{y})");
        }
    }

    #[test]
    fn sha1_matches_known_vectors() {
        // FIPS 180-4 / RFC 3174 reference vectors.
        let hex = |d: &[u8]| {
            super::sha1(d)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        assert_eq!(hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex(b"The quick brown fox jumps over the lazy dog"),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
        // A 64-byte input exercises the multi-block padding boundary.
        assert_eq!(hex(&[b'a'; 64]), "0098ba824b5c16427bd7a1122a5a442a25ec644d");
    }

    /// Build a tiny self-consistent single-layer VT template (1 mip, 1 tile,
    /// PF_DXT1), `retile` a solid-color image into it, and assert the structure
    /// is preserved and `decode_layer0` of the result recovers the color. Proves
    /// the border-build + BC-encode + chunk-layout round-trips via our own decode.
    #[test]
    fn retile_synthetic_solid_roundtrips() {
        const TILE: u32 = 8;
        const BORDER: u32 = 4;
        let phys = (TILE + 2 * BORDER) as usize; // 16
        let blocks = (phys + 3) / 4; // 4
        let packed = blocks * blocks * 8; // DXT1: 4*4*8 = 128 bytes

        let template = VtData {
            b_cooked: 1,
            num_layers: 1,
            width_in_blocks: TILE / 4,
            height_in_blocks: TILE / 4,
            tile_size: TILE,
            tile_border_size: BORDER,
            tile_data_offset_per_layer: vec![packed as u32],
            num_mips: 1,
            width: TILE,
            height: TILE,
            chunk_index_per_mip: vec![0],
            base_offset_per_mip: vec![0],
            tile_offset_data: vec![VtTileOffset {
                width: 1,
                height: 1,
                max_address: 1,
                addresses: vec![0],
                offsets: vec![0],
            }],
            tile_index_per_chunk: vec![],
            tile_index_per_mip: vec![],
            tile_offset_in_chunk: vec![],
            layer_types: vec!["PF_DXT1".to_string()],
            layer_fallback_colors: vec![[0u8; 16]],
            chunks: vec![VtChunk {
                bulk_data_hash: [0u8; 20],
                size_in_bytes: packed as u32,
                codec_payload_size: 0,
                codec: vec![(4, 0)], // RawGPU
                data_resource_index: 0,
            }],
        };

        // Solid color (orange-ish; avoid pure red so we exercise all channels).
        let color = [200u8, 120, 40, 255];
        let mut rgba = Vec::with_capacity((TILE * TILE * 4) as usize);
        for _ in 0..(TILE * TILE) {
            rgba.extend_from_slice(&color);
        }

        let (new_vt, chunk_bytes) = retile(&rgba, TILE, TILE, &template, "PF_DXT1").unwrap();

        // Structure preserved (grid/addresses/offsets/sizes identical).
        assert_eq!(new_vt.tile_offset_data, template.tile_offset_data);
        assert_eq!(new_vt.chunk_index_per_mip, template.chunk_index_per_mip);
        assert_eq!(new_vt.base_offset_per_mip, template.base_offset_per_mip);
        assert_eq!(
            new_vt.tile_data_offset_per_layer,
            template.tile_data_offset_per_layer
        );
        assert_eq!(new_vt.num_mips, template.num_mips);
        assert_eq!(new_vt.width, template.width);
        assert_eq!(new_vt.height, template.height);
        assert_eq!(chunk_bytes.len(), 1);
        assert_eq!(chunk_bytes[0].len(), packed, "chunk size == template size");
        assert_eq!(new_vt.chunks[0].size_in_bytes, packed as u32);
        // Hash recomputed (no longer the all-zero placeholder).
        assert_ne!(new_vt.chunks[0].bulk_data_hash, [0u8; 20]);
        assert_eq!(
            new_vt.chunks[0].bulk_data_hash,
            super::sha1(&chunk_bytes[0])
        );

        // Decode the re-tiled VT back and check the center pixel ~ the color.
        let (dw, dh, px) = decode_layer0(&new_vt, &chunk_bytes, "PF_DXT1").unwrap();
        assert_eq!((dw, dh), (TILE, TILE));
        let center = px[(dh as usize / 2) * dw as usize + (dw as usize / 2)];
        let r = ((center >> 16) & 0xff) as i32;
        let g = ((center >> 8) & 0xff) as i32;
        let b = (center & 0xff) as i32;
        // BC1 565 quantization tolerance.
        assert!((r - color[0] as i32).abs() <= 12, "R {r} vs {}", color[0]);
        assert!((g - color[1] as i32).abs() <= 12, "G {g} vs {}", color[1]);
        assert!((b - color[2] as i32).abs() <= 12, "B {b} vs {}", color[2]);
    }

    #[test]
    fn retile_rejects_bad_inputs() {
        let mut t = VtData {
            b_cooked: 1,
            num_layers: 1,
            width_in_blocks: 1,
            height_in_blocks: 1,
            tile_size: 4,
            tile_border_size: 0,
            tile_data_offset_per_layer: vec![8],
            num_mips: 1,
            width: 4,
            height: 4,
            chunk_index_per_mip: vec![0],
            base_offset_per_mip: vec![0],
            tile_offset_data: vec![VtTileOffset {
                width: 1,
                height: 1,
                max_address: 1,
                addresses: vec![0],
                offsets: vec![0],
            }],
            tile_index_per_chunk: vec![],
            tile_index_per_mip: vec![],
            tile_offset_in_chunk: vec![],
            layer_types: vec!["PF_DXT1".to_string()],
            layer_fallback_colors: vec![[0u8; 16]],
            chunks: vec![VtChunk {
                bulk_data_hash: [0u8; 20],
                size_in_bytes: 8,
                codec_payload_size: 0,
                codec: vec![(4, 0)],
                data_resource_index: 0,
            }],
        };
        let good = vec![0u8; 4 * 4 * 4];

        // Wrong dims.
        assert!(retile(&good, 8, 4, &t, "PF_DXT1").is_err());
        // Wrong buffer length.
        assert!(retile(&vec![0u8; 10], 4, 4, &t, "PF_DXT1").is_err());
        // Multi-layer.
        t.num_layers = 2;
        assert!(retile(&good, 4, 4, &t, "PF_DXT1").is_err());
        t.num_layers = 1;
        // OK baseline still works.
        assert!(retile(&good, 4, 4, &t, "PF_DXT1").is_ok());
    }

    #[test]
    fn tile_offset_get_offset_and_validity() {
        // Two runs: [0,4) at base 100, a gap [4,8) (sentinel), [8,..) at base 200.
        let t = VtTileOffset {
            width: 4,
            height: 4,
            max_address: 16,
            addresses: vec![0, 4, 8],
            offsets: vec![100, VT_INVALID_OFFSET, 200],
        };
        assert_eq!(t.get_offset(0), Some(100));
        assert_eq!(t.get_offset(3), Some(103));
        assert!(!t.is_valid_address(4)); // in the sentinel run
        assert!(!t.is_valid_address(6));
        assert_eq!(t.get_offset(8), Some(200));
        assert_eq!(t.get_offset(10), Some(202));
    }

    // ---- minimal RGBA PNG writer (no deps) for the debug artifact -----------

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &b in bytes {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }

    fn adler32(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &x in bytes {
            a = (a + x as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    /// Write `rgba` (`0xAARRGGBB`) as an uncompressed (zlib stored-blocks) PNG.
    fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u32]) {
        // Raw scanlines: each row prefixed with filter byte 0, RGBA samples.
        let mut raw = Vec::with_capacity((w as usize * 4 + 1) * h as usize);
        for y in 0..h as usize {
            raw.push(0); // filter: none
            for x in 0..w as usize {
                let p = rgba[y * w as usize + x];
                raw.push(((p >> 16) & 0xff) as u8); // R
                raw.push(((p >> 8) & 0xff) as u8); // G
                raw.push((p & 0xff) as u8); // B
                raw.push(((p >> 24) & 0xff) as u8); // A
            }
        }
        // zlib stream of stored (uncompressed) DEFLATE blocks.
        let mut zlib = vec![0x78u8, 0x01]; // CMF/FLG
        let mut i = 0;
        while i < raw.len() {
            let chunk = (raw.len() - i).min(0xffff);
            let last = if i + chunk >= raw.len() { 1u8 } else { 0 };
            zlib.push(last); // BFINAL + BTYPE=00
            zlib.extend_from_slice(&(chunk as u16).to_le_bytes());
            zlib.extend_from_slice(&(!(chunk as u16)).to_le_bytes());
            zlib.extend_from_slice(&raw[i..i + chunk]);
            i += chunk;
        }
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let chunk = |ty: &[u8; 4], data: &[u8], out: &mut Vec<u8>| {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let mut crc_in = Vec::with_capacity(4 + data.len());
            crc_in.extend_from_slice(ty);
            crc_in.extend_from_slice(data);
            out.extend_from_slice(ty);
            out.extend_from_slice(data);
            out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit, RGBA, deflate, no filter, no interlace
        chunk(b"IHDR", &ihdr, &mut png);
        chunk(b"IDAT", &zlib, &mut png);
        chunk(b"IEND", &[], &mut png);
        std::fs::write(path, &png).unwrap();
    }

    /// Decode the Biter virtual texture end-to-end and assert it is a real 4096²
    /// image. Gated on the game install (like the container tests) — slow (full
    /// container scan to unpack), so `#[ignore]`.
    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn decodes_biter_vt_to_real_image() {
        use std::path::PathBuf;
        let game = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !game.exists() {
            eprintln!("skip: game not installed");
            return;
        }
        let utoc = crate::paths::main_container(&game).unwrap();
        let usmap = crate::paths::usmap(&game).unwrap();

        let asset = "/Game/Assets/Characters/Creatures/Biter/Model/Armor/Textures/T_Biter_Armor_D";
        let tmp = std::env::temp_dir().join("gore-tex-vt-decode-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let uasset_path = crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap();
        let uexp = std::fs::read(uasset_path.with_extension("uexp")).unwrap();
        let ubulk = std::fs::read(uasset_path.with_extension("ubulk")).unwrap_or_default();
        let uasset = std::fs::read(&uasset_path).unwrap();
        let usmap_bytes = std::fs::read(&usmap).unwrap();

        let info = crate::decode::parse(&uasset, &uexp, &ubulk, &usmap_bytes).unwrap();
        let rgba = crate::decode::to_rgba8(&info).unwrap();
        eprintln!(
            "Biter VT: {}x{} {} is_virtual={} pixels={}",
            info.width,
            info.height,
            info.format,
            info.is_virtual,
            rgba.len()
        );

        assert!(info.is_virtual);
        assert_eq!(info.width, 4096);
        assert_eq!(info.height, 4096);
        assert_eq!(rgba.len(), 4096 * 4096);

        // Real image: not all-zero, not a single solid value, with broad variety.
        let mut distinct = std::collections::HashSet::new();
        for &p in &rgba {
            if distinct.len() < 5000 {
                distinct.insert(p);
            }
        }
        eprintln!("distinct pixel values (capped at 5000): {}", distinct.len());
        assert!(rgba.iter().any(|&p| p != 0), "image is all-zero");
        assert!(
            distinct.len() >= 1000,
            "image too uniform ({} distinct values) — likely garbage/blank",
            distinct.len()
        );

        // Debug artifact for visual inspection vs work/vt_spike_preview.png.
        let png = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/vt_t2_decode.png");
        write_png(&png, info.width, info.height, &rgba);
        eprintln!("wrote debug PNG: {}", png.display());
    }

    /// STRONG ORACLE: parse the real Biter VT, `retile` it with a same-dims solid
    /// image, and assert (a) every rebuilt chunk length == the original chunk's
    /// SizeInBytes (the same-dims layout invariant), and (b) `decode_layer0` of
    /// the re-tiled result recovers the solid color across the whole image. This
    /// validates border-build + BC-encode + the full multi-mip chunk layout
    /// against a genuine cooked VT. Gated on the game install; slow.
    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn retile_biter_vt_solid_roundtrips() {
        use std::path::PathBuf;
        let game = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !game.exists() {
            eprintln!("skip: game not installed");
            return;
        }
        let utoc = crate::paths::main_container(&game).unwrap();
        let usmap = crate::paths::usmap(&game).unwrap();

        let asset = "/Game/Assets/Characters/Creatures/Biter/Model/Armor/Textures/T_Biter_Armor_D";
        let tmp = std::env::temp_dir().join("gore-tex-vt-retile-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let uasset_path = crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap();
        let uexp = std::fs::read(uasset_path.with_extension("uexp")).unwrap();
        let ubulk = std::fs::read(uasset_path.with_extension("ubulk")).unwrap_or_default();
        let uasset = std::fs::read(&uasset_path).unwrap();

        let pd = crate::texdata::PlatformData::parse(&uasset, &uexp, &ubulk).unwrap();
        let vt = pd.vt.as_ref().expect("Biter asset is a VT");
        let fmt = vt.layer_types[0].clone();
        eprintln!(
            "Biter VT template: {}x{} {fmt} mips={} chunks={}",
            vt.width,
            vt.height,
            vt.num_mips,
            vt.chunks.len()
        );

        // Same-dims solid-color RGBA8 (R,G,B,A byte order — encode input order).
        let color = [80u8, 160, 220, 255];
        let mut rgba = Vec::with_capacity((vt.width * vt.height * 4) as usize);
        for _ in 0..(vt.width * vt.height) {
            rgba.extend_from_slice(&color);
        }

        let (new_vt, chunk_bytes) = retile(&rgba, vt.width, vt.height, vt, &fmt).unwrap();

        // (a) Same-dims invariant: every rebuilt chunk matches the original size.
        for (c, (buf, ch)) in chunk_bytes.iter().zip(vt.chunks.iter()).enumerate() {
            assert_eq!(
                buf.len() as u32,
                ch.size_in_bytes,
                "chunk {c} rebuilt len {} != template SizeInBytes {}",
                buf.len(),
                ch.size_in_bytes
            );
        }
        eprintln!(
            "chunk sizes OK: {:?}",
            new_vt
                .chunks
                .iter()
                .map(|c| c.size_in_bytes)
                .collect::<Vec<_>>()
        );

        // (b) Decode the re-tiled VT back; the whole image should be ~the color.
        let (dw, dh, px) = decode_layer0(&new_vt, &chunk_bytes, &fmt).unwrap();
        assert_eq!((dw, dh), (vt.width, vt.height));
        let (mut max_dr, mut max_dg, mut max_db) = (0i32, 0i32, 0i32);
        for &p in &px {
            let r = ((p >> 16) & 0xff) as i32;
            let g = ((p >> 8) & 0xff) as i32;
            let b = (p & 0xff) as i32;
            max_dr = max_dr.max((r - color[0] as i32).abs());
            max_dg = max_dg.max((g - color[1] as i32).abs());
            max_db = max_db.max((b - color[2] as i32).abs());
        }
        eprintln!("max per-channel deviation from solid: R{max_dr} G{max_dg} B{max_db}");
        // BC1 565 quantization; a solid color stays well within a small band.
        assert!(
            max_dr <= 12 && max_dg <= 12 && max_db <= 12,
            "re-tiled solid drifted"
        );
    }
}
