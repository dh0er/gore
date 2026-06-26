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
use crate::texdata::{block_bytes, corrupt, rd_i32, rd_u32, read_fstring, write_fstring_ascii};

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
        Some(base + (address - self.addresses[block_index]))
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

/// Decode a physical `phys`x`phys` BCn tile (`bytes`) to RGBA (`0xAARRGGBB`).
fn decode_bcn_tile(bytes: &[u8], phys: usize, format: &str) -> Result<Vec<u32>> {
    let mut out = vec![0u32; phys * phys];
    let res = match format {
        "PF_DXT1" => texture2ddecoder::decode_bc1(bytes, phys, phys, &mut out),
        "PF_DXT5" => texture2ddecoder::decode_bc3(bytes, phys, phys, &mut out),
        "PF_BC5" => texture2ddecoder::decode_bc5(bytes, phys, phys, &mut out),
        "PF_BC7" => texture2ddecoder::decode_bc7(bytes, phys, phys, &mut out),
        _ => return Err(TexError::UnsupportedFormat(format.to_string())),
    };
    res.map_err(|reason| TexError::DecodeFailed {
        format: format.to_string(),
        reason: reason.to_string(),
    })?;
    Ok(out)
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
    if vt.num_mips == 0 || vt.tile_offset_data.is_empty() {
        return Err(corrupt("VT has no mips to decode"));
    }
    // Mip 0: the largest level (first TileOffsetData entry).
    const LEVEL: usize = 0;
    let mip = &vt.tile_offset_data[LEVEL];

    let tile_size = vt.tile_size;
    let border = vt.tile_border_size;
    let phys = (tile_size + 2 * border) as usize;
    if phys == 0 {
        return Err(corrupt("VT physical tile size is zero"));
    }

    // Stitched bitmap: grid (in tiles) * tile_size.
    let grid_w = mip.width;
    let grid_h = mip.height;
    let bmp_w = grid_w * tile_size;
    let bmp_h = grid_h * tile_size;
    let mut bitmap = vec![0u32; (bmp_w as usize) * (bmp_h as usize)];

    // Per-tile packed BCn size for the PHYSICAL (bordered) tile.
    let bb = block_bytes(layer0_format)
        .ok_or_else(|| TexError::UnsupportedFormat(layer0_format.to_string()))? as usize;
    let blocks = (phys + 3) / 4;
    let packed_size = blocks * blocks * bb;

    // Per-tile stride across all layers (== TileDataOffsetPerLayer[NumLayers]);
    // layer 0's intra-tile offset is TileDataOffsetPerLayer[0] == 0.
    let per_tile_stride = *vt
        .tile_data_offset_per_layer
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
            let tile_index = base_tile + tile_off;
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
            let off = in_chunk as u64
                + mip.get_offset(addr).map(|_| 0u64).unwrap_or(0)
                + layer0_offset;
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
            let off = base + tile_off * per_tile_stride + layer0_offset;
            (chunk, off)
        };

        let data = chunk_bytes
            .get(chunk_index)
            .ok_or_else(|| corrupt("VT tile references a chunk with no resolved bytes"))?;
        let start = offset as usize;
        let end = start
            .checked_add(packed_size)
            .ok_or_else(|| corrupt("VT tile slice overflow"))?;
        let tile = data
            .get(start..end)
            .ok_or_else(|| corrupt("VT tile bytes run past end of chunk"))?;

        // Decode the physical (bordered) tile, then copy the inner tile_size area.
        let decoded = decode_bcn_tile(tile, phys, layer0_format)?;
        let tile_x = (reverse_morton2(addr) * tile_size) as usize;
        let tile_y = (reverse_morton2(addr >> 1) * tile_size) as usize;
        let b = border as usize;
        let ts = tile_size as usize;
        for y in 0..ts {
            let src_row = (y + b) * phys + b;
            let dst_row = (tile_y + y) * (bmp_w as usize) + tile_x;
            bitmap[dst_row..dst_row + ts].copy_from_slice(&decoded[src_row..src_row + ts]);
        }
    }

    Ok((bmp_w, bmp_h, bitmap))
}

/// Read a `TArray<u32>` at `*pos`: `i32 count` then `count` u32 elements.
fn read_u32_array(b: &[u8], pos: &mut usize) -> Result<Vec<u32>> {
    let count = rd_i32(b, *pos)?;
    *pos += 4;
    if count < 0 {
        return Err(corrupt("negative TArray<u32> count in VT block"));
    }
    let n = count as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(rd_u32(b, *pos)?);
        *pos += 4;
    }
    Ok(out)
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
    if num_layers > 64 {
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

    let tile_data_offset_per_layer = read_u32_array(b, pos)?;

    let num_mips = rd_u32(b, *pos)?;
    *pos += 4;
    if num_mips > 32 {
        return Err(corrupt("implausible VT NumMips"));
    }
    let width = rd_u32(b, *pos)?;
    *pos += 4;
    let height = rd_u32(b, *pos)?;
    *pos += 4;

    let chunk_index_per_mip = read_u32_array(b, pos)?;
    let base_offset_per_mip = read_u32_array(b, pos)?;

    // TileOffsetData is a `TArray<FVirtualTextureTileOffsetData>`: an i32 count
    // (== NumMips for non-legacy) then the structs. The count prefix is REAL on
    // disk — omitting it desyncs the whole tail.
    let tile_offset_count = rd_i32(b, *pos)?;
    *pos += 4;
    if tile_offset_count < 0 {
        return Err(corrupt("negative VT TileOffsetData count"));
    }
    let mut tile_offset_data = Vec::with_capacity(tile_offset_count as usize);
    for _ in 0..tile_offset_count {
        let w = rd_u32(b, *pos)?;
        *pos += 4;
        let h = rd_u32(b, *pos)?;
        *pos += 4;
        let max_address = rd_u32(b, *pos)?;
        *pos += 4;
        let addresses = read_u32_array(b, pos)?;
        let offsets = read_u32_array(b, pos)?;
        tile_offset_data.push(VtTileOffset {
            width: w,
            height: h,
            max_address,
            addresses,
            offsets,
        });
    }

    // Legacy-only arrays (empty in the non-legacy form).
    let tile_index_per_chunk = read_u32_array(b, pos)?;
    let tile_index_per_mip = read_u32_array(b, pos)?;
    let tile_offset_in_chunk = read_u32_array(b, pos)?;

    let mut layer_types = Vec::with_capacity(num_layers as usize);
    for _ in 0..num_layers {
        let (s, next) = read_fstring(b, *pos)?;
        *pos = next;
        layer_types.push(s);
    }

    let mut layer_fallback_colors = Vec::with_capacity(num_layers as usize);
    for _ in 0..num_layers {
        let s = b
            .get(*pos..*pos + 16)
            .ok_or_else(|| corrupt("VT LayerFallbackColor runs past end"))?;
        let mut c = [0u8; 16];
        c.copy_from_slice(s);
        layer_fallback_colors.push(c);
        *pos += 16;
    }

    // Chunks: i32 count then the chunk structs.
    let chunk_count = rd_i32(b, *pos)?;
    *pos += 4;
    if chunk_count < 0 {
        return Err(corrupt("negative VT chunk count"));
    }
    let mut chunks = Vec::with_capacity(chunk_count as usize);
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

        let mut codec = Vec::with_capacity(num_layers as usize);
        for _ in 0..num_layers {
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

        let asset =
            "/Game/Assets/Characters/Creatures/Biter/Model/Armor/Textures/T_Biter_Armor_D";
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
            info.width, info.height, info.format, info.is_virtual, rgba.len()
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
        let png = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../work/vt_t2_decode.png");
        write_png(&png, info.width, info.height, &rgba);
        eprintln!("wrote debug PNG: {}", png.display());
    }
}
