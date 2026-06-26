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

use crate::error::Result;
use crate::texdata::{corrupt, rd_i32, rd_u32, read_fstring, write_fstring_ascii};

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
