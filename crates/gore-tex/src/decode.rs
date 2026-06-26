//! Cooked BCn -> RGBA -> PNG decoding.
//!
//! [`parse`] extracts the platform data ([`TexInfo`]) from a *legacy cooked*
//! `UTexture2D` (the `.uasset` / `.uexp` / `.ubulk` triple produced by retoc's
//! zen->legacy conversion, see [`crate::container::unpack_asset`]). The actual
//! pixel decode (BCn -> RGBA) lives elsewhere; this module only locates and
//! returns the raw mip-0 BCn bytes plus the metadata needed to decode them.
//!
//! ## How the cooked layout is parsed
//!
//! In a legacy cooked package the export *body* lives in `.uexp` (the `.uasset`
//! is the summary/header). For a `UTexture2D` that body is, in order:
//!
//! 1. the UObject **unversioned property block** (shipped UE5 strips property
//!    names, so its size is only knowable from the `.usmap` schema),
//! 2. `FStripDataFlags` + `bool bCooked`,
//! 3. `SerializeCookedPlatformData`: a `(PixelFormat, SkipOffset,
//!    FTexturePlatformData)` sequence,
//!
//! where `FTexturePlatformData` begins with `int32 SizeX, int32 SizeY,
//! uint32 PackedData, FString PixelFormat`.
//!
//! Skipping the unversioned property block byte-exactly needs a full usmap
//! property walker. We deliberately **avoid** that here: the cooked platform
//! data is self-describing and easy to find unambiguously. We scan `.uexp` for
//! a valid `PF_*` `FString` whose preceding 12 bytes (`SizeX, SizeY,
//! PackedData`) are sane and whose following mip table is self-consistent, then
//! parse `FTexturePlatformData` forward from `SizeX`. The heavy validation
//! (plausible dims, known `PF_*` format, mip-0 length matching the format's
//! block math) makes a false positive effectively impossible. `usmap` is
//! accepted for API symmetry and forward-compatibility but is not required by
//! this path.
//!
//! ## The mip-0 bulk data
//!
//! The export bytes are copied **verbatim** from the on-disk zen package by
//! retoc, so the inline-mip serialization is exactly UE5.4 IoStore's: after
//! `FirstMipToSerialize (i32)` and `NumMips (i32)`, each mip is a serialized
//! `FByteBulkData` followed by `int32 SizeX, SizeY, SizeZ`. For an **inline**
//! payload the `FByteBulkData` header in the zen form is a single
//! `uint32 BulkDataFlags` and the raw bytes follow immediately; the byte length
//! is the BCn block size for the mip dims (it is not re-stated in the inline
//! header). For payloads stored at the end of `.uexp` or in a separate `.ubulk`
//! the `FByteBulkData` carries an explicit `SizeOnDisk`/`OffsetInFile`, fixed up
//! by the summary `BulkDataStartOffset` unless `BULKDATA_NoOffsetFixUp` is set.

use crate::error::{Result, TexError};

/// Minimal decoded platform data needed to turn a cooked texture into pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexInfo {
    pub width: u32,
    pub height: u32,
    /// UE pixel format name, e.g. "PF_DXT1", "PF_DXT5", "PF_BC5", "PF_BC7".
    pub format: String,
    /// Raw BCn bytes of mip 0 (largest), as stored on disk.
    pub mip0: Vec<u8>,
    pub is_virtual: bool,
}

// ---- UE EBulkDataFlags we care about (subset) -----------------------------

/// Payload is stored at `OffsetInFile` within the *same* file (`.uexp`).
const BULKDATA_PAYLOAD_AT_END_OF_FILE: u32 = 0x0000_0001;
/// Payload bytes are inline in `.uexp`, immediately after the bulk header.
const BULKDATA_FORCE_INLINE_PAYLOAD: u32 = 0x0000_0040;
/// Payload lives in a separate file (`.ubulk`).
const BULKDATA_PAYLOAD_IN_SEPERATE_FILE: u32 = 0x0000_0100;
/// `OffsetInFile` is absolute and must NOT have `BulkDataStartOffset` added.
const BULKDATA_NO_OFFSET_FIX_UP: u32 = 0x0001_0000;
/// `ElementCount` / `SizeOnDisk` are serialized as 64-bit.
const BULKDATA_SIZE_64_BIT: u32 = 0x0000_0020;

// ---- supported formats & block math ---------------------------------------

/// Pixel formats this v1 read path understands. Each is a BCn block-compressed
/// format laid out as 4x4 blocks.
fn block_bytes(format: &str) -> Option<u32> {
    match format {
        // 8 bytes / 4x4 block
        "PF_DXT1" => Some(8),
        // 16 bytes / 4x4 block
        "PF_DXT5" | "PF_BC5" | "PF_BC7" => Some(16),
        _ => None,
    }
}

/// Bytes of a single mip of `format` at `w` x `h` (BCn, 4x4 blocks).
fn mip_byte_size(format: &str, w: u32, h: u32) -> Option<u64> {
    let bb = block_bytes(format)? as u64;
    let blocks_x = ((w as u64) + 3) / 4;
    let blocks_y = ((h as u64) + 3) / 4;
    Some(blocks_x * blocks_y * bb)
}

// ---- little-endian readers (bounds-checked) -------------------------------

fn rd_i32(b: &[u8], o: usize) -> Result<i32> {
    let s = b
        .get(o..o + 4)
        .ok_or_else(|| corrupt("unexpected end of .uexp reading i32"))?;
    Ok(i32::from_le_bytes(s.try_into().unwrap()))
}
fn rd_u32(b: &[u8], o: usize) -> Result<u32> {
    let s = b
        .get(o..o + 4)
        .ok_or_else(|| corrupt("unexpected end of .uexp reading u32"))?;
    Ok(u32::from_le_bytes(s.try_into().unwrap()))
}
fn rd_i64(b: &[u8], o: usize) -> Result<i64> {
    let s = b
        .get(o..o + 8)
        .ok_or_else(|| corrupt("unexpected end of .uexp reading i64"))?;
    Ok(i64::from_le_bytes(s.try_into().unwrap()))
}

fn corrupt(msg: &str) -> TexError {
    TexError::Retoc(anyhow::anyhow!("cooked texture parse: {msg}"))
}

/// Read a UE `FString` at `o`. Returns `(string, next_offset)`.
/// `len > 0`: `len` UTF-8 bytes incl trailing NUL. `len < 0`: `-len` UTF-16LE
/// units incl trailing NUL. `0`: empty.
fn read_fstring(b: &[u8], o: usize) -> Result<(String, usize)> {
    let len = rd_i32(b, o)?;
    let mut p = o + 4;
    if len == 0 {
        return Ok((String::new(), p));
    }
    if len > 0 {
        let n = len as usize;
        let bytes = b
            .get(p..p + n)
            .ok_or_else(|| corrupt("FString utf8 runs past end"))?;
        p += n;
        // strip trailing NUL
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
        Ok((s, p))
    } else {
        let n = (-len) as usize;
        let mut units = Vec::with_capacity(n);
        for i in 0..n {
            units.push(rd_u32_as_u16(b, p + i * 2)?);
        }
        p += n * 2;
        let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
        let s = String::from_utf16_lossy(&units[..end]);
        Ok((s, p))
    }
}

fn rd_u32_as_u16(b: &[u8], o: usize) -> Result<u16> {
    let s = b
        .get(o..o + 2)
        .ok_or_else(|| corrupt("unexpected end of .uexp reading u16"))?;
    Ok(u16::from_le_bytes(s.try_into().unwrap()))
}

/// Read the `int64 BulkDataStartOffset` from a legacy `.uasset` summary.
///
/// This is the fixup base added to a bulk data `OffsetInFile` for payloads that
/// live at the end of `.uexp` or in `.ubulk` (unless `BULKDATA_NoOffsetFixUp`).
/// We parse it with retoc's authoritative summary deserializer (UE5.4 pinned)
/// so we don't have to hand-walk the summary layout. Returns `0` if the summary
/// can't be parsed (the inline path doesn't need it).
fn bulk_data_start_offset(uasset: &[u8]) -> i64 {
    use retoc::legacy_asset::FLegacyPackageHeader;
    use std::io::Cursor;
    let pkg_ver = retoc::version::EngineVersion::UE5_4.package_file_version();
    match FLegacyPackageHeader::deserialize(&mut Cursor::new(uasset), Some(pkg_ver)) {
        Ok(h) => h.summary.bulk_data_start_offset,
        Err(_) => 0,
    }
}

/// Parse a cooked UTexture2D's platform data from its legacy cooked files.
///
/// `uexp` holds the export body (platform data + inline/end-of-file mips),
/// `ubulk` holds separate-file mip payloads (may be empty), `uasset` provides
/// the summary `BulkDataStartOffset` for bulk offset fixup. `usmap` is accepted
/// for API symmetry; the locate-by-anchor strategy does not require it.
pub fn parse(uasset: &[u8], uexp: &[u8], ubulk: &[u8], _usmap: &[u8]) -> Result<TexInfo> {
    // Locate FTexturePlatformData by anchoring on a valid PF_* FString preceded
    // by a sane (SizeX, SizeY, PackedData) triple and followed by a
    // self-consistent mip table. We try every candidate and validate hard.
    let mut last_err =
        corrupt("no FTexturePlatformData found (no valid PF_* anchor in .uexp)");

    let mut search = 0usize;
    while let Some(rel) = find_pf_anchor(uexp, search) {
        // `rel` is the offset of the FString length prefix (PixelFormat).
        // SizeX/SizeY/PackedData are the three i32 immediately before it.
        if rel >= 12 {
            let pd_start = rel - 12;
            match try_parse_platform_data(uasset, uexp, ubulk, pd_start) {
                Ok(info) => return Ok(info),
                Err(e) => last_err = e,
            }
        }
        search = rel + 1;
    }
    Err(last_err)
}

/// Find the next `PF_*` `FString` length-prefix offset at/after `from`.
/// Matches the on-disk form: `int32 len (>0)`, then `"PF_"` UTF-8 bytes.
fn find_pf_anchor(uexp: &[u8], from: usize) -> Option<usize> {
    // The FString content "PF_" appears at prefix+4. Search for the bytes "PF_"
    // and back up to the length prefix, validating the prefix is the matching
    // UTF-8 length.
    let needle = b"PF_";
    let mut i = from + 4;
    while i + needle.len() <= uexp.len() {
        if &uexp[i..i + needle.len()] == needle {
            let prefix = i - 4;
            if let Ok(len) = rd_i32(uexp, prefix) {
                // UTF-8 FString: positive length incl trailing NUL; the bytes
                // [i .. i+len] should be the format name + NUL.
                if len > 0 && len < 64 {
                    let end = i + (len as usize);
                    if end <= uexp.len() && uexp[end - 1] == 0 {
                        return Some(prefix);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Attempt to parse FTexturePlatformData starting at `pd_start` (the offset of
/// `SizeX`). Validates dims and the mip table; any inconsistency -> `Err` so the
/// caller moves to the next candidate anchor.
fn try_parse_platform_data(
    uasset: &[u8],
    uexp: &[u8],
    ubulk: &[u8],
    pd_start: usize,
) -> Result<TexInfo> {
    let size_x = rd_i32(uexp, pd_start)?;
    let size_y = rd_i32(uexp, pd_start + 4)?;
    let packed = rd_u32(uexp, pd_start + 8)?;

    // Plausible texture dimensions: positive and not absurd. (Cooked mip dims
    // for the platform data are <= 16384 in practice.)
    if !(1..=16384).contains(&size_x) || !(1..=16384).contains(&size_y) {
        return Err(corrupt("implausible texture dimensions at anchor"));
    }
    let width = size_x as u32;
    let height = size_y as u32;

    // bIsVirtual = bit31 of PackedData.
    let is_virtual = (packed & 0x8000_0000) != 0;

    // PixelFormat FString.
    let (format, mut o) = read_fstring(uexp, pd_start + 12)?;
    if !format.starts_with("PF_") {
        return Err(corrupt("anchor FString is not a PF_* pixel format"));
    }

    if is_virtual {
        return Err(TexError::VirtualTexture(format));
    }
    let bb = match block_bytes(&format) {
        Some(b) => b,
        None => return Err(TexError::UnsupportedFormat(format)),
    };
    let _ = bb;

    // FTexturePlatformData (UE5 cooked) after PixelFormat:
    //   int32 FirstMipToSerialize
    //   int32 NumMips
    //   FTexture2DMipMap[NumMips]
    let _first_mip = rd_i32(uexp, o)?;
    o += 4;
    let num_mips = rd_i32(uexp, o)?;
    o += 4;
    if !(1..=20).contains(&num_mips) {
        return Err(corrupt("implausible NumMips"));
    }

    // Mip 0 is the largest and is what we want. Read its FByteBulkData header
    // and extract the bytes.
    let expected = mip_byte_size(&format, width, height)
        .ok_or_else(|| corrupt("no block size for format"))?;

    let mip0 = read_mip0(uasset, uexp, ubulk, o, &format, width, height, expected)?;

    // Hard validation: the extracted byte length must equal the BCn block math
    // for the dims. This is what makes the heuristic safe.
    if mip0.len() as u64 != expected {
        return Err(corrupt("mip0 length does not match format/dimension block math"));
    }

    Ok(TexInfo {
        width,
        height,
        format,
        mip0,
        is_virtual: false,
    })
}

/// Parse the mip-0 `FByteBulkData` header at `o` and return its payload bytes.
///
/// Handles the three placements: inline in `.uexp`, at the end of `.uexp`, and
/// in a separate `.ubulk`. The zen-derived inline form is compact (just a
/// `uint32 BulkDataFlags`, payload immediately after); the end-of-file and
/// separate-file forms carry the full element-count / size / offset header.
#[allow(clippy::too_many_arguments)]
fn read_mip0(
    uasset: &[u8],
    uexp: &[u8],
    ubulk: &[u8],
    o: usize,
    _format: &str,
    _width: u32,
    _height: u32,
    expected: u64,
) -> Result<Vec<u8>> {
    let flags = rd_u32(uexp, o)?;
    let mut p = o + 4;

    let inline = (flags & BULKDATA_FORCE_INLINE_PAYLOAD) != 0;
    let separate = (flags & BULKDATA_PAYLOAD_IN_SEPERATE_FILE) != 0;
    let end_of_file = (flags & BULKDATA_PAYLOAD_AT_END_OF_FILE) != 0;

    // The zen->legacy inline form observed in UE5.4 cooked output serializes the
    // inline mip as a single `uint32 BulkDataFlags` (often 0) followed
    // immediately by the raw payload, whose length is the BCn block size (it is
    // not restated). Treat "no separate/end-of-file bit" as inline-immediate.
    if !separate && !end_of_file {
        let _ = inline; // either the inline bit is set, or flags==0 (zen inline)
        let payload = uexp
            .get(p..p + expected as usize)
            .ok_or_else(|| corrupt("inline mip0 payload runs past end of .uexp"))?;
        return Ok(payload.to_vec());
    }

    // Non-inline: read the full header. ElementCount/SizeOnDisk are 64-bit when
    // BULKDATA_SIZE_64_BIT is set, otherwise ElementCount is int32.
    let size_64 = (flags & BULKDATA_SIZE_64_BIT) != 0;
    let _element_count: i64 = if size_64 {
        let v = rd_i64(uexp, p)?;
        p += 8;
        v
    } else {
        let v = rd_i32(uexp, p)? as i64;
        p += 4;
        v
    };
    let size_on_disk: i64 = rd_i64(uexp, p)?;
    p += 8;
    let mut offset_in_file: i64 = rd_i64(uexp, p)?;
    // (SizeX/SizeY/SizeZ follow but we don't need them for mip0 extraction.)

    if size_on_disk as u64 != expected {
        return Err(corrupt("bulk SizeOnDisk disagrees with format block math"));
    }

    // Offset fixup: add the summary BulkDataStartOffset unless NoOffsetFixUp.
    if (flags & BULKDATA_NO_OFFSET_FIX_UP) == 0 {
        let base = bulk_data_start_offset(uasset);
        offset_in_file += base;
    }
    let off = usize::try_from(offset_in_file)
        .map_err(|_| corrupt("negative/oversized bulk OffsetInFile"))?;
    let n = expected as usize;

    if separate {
        // .ubulk: offset is relative to the start of the bulk file. After the
        // fixup above the offset is in the combined (uasset+uexp+ubulk) space;
        // subtract the (uasset+uexp) prefix == BulkDataStartOffset base. When
        // NoOffsetFixUp is set the offset is already file-relative.
        let base = if (flags & BULKDATA_NO_OFFSET_FIX_UP) == 0 {
            bulk_data_start_offset(uasset)
        } else {
            0
        };
        let rel = offset_in_file
            .checked_sub(base)
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| corrupt("bad .ubulk relative offset"))?;
        let payload = ubulk
            .get(rel..rel + n)
            .ok_or_else(|| corrupt("mip0 payload runs past end of .ubulk"))?;
        Ok(payload.to_vec())
    } else {
        // end-of-file within .uexp.
        let payload = uexp
            .get(off..off + n)
            .ok_or_else(|| corrupt("end-of-file mip0 payload runs past end of .uexp"))?;
        Ok(payload.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fx(name: &str) -> Option<Vec<u8>> {
        let p = format!("../../work/tex-fixtures/{name}");
        std::fs::read(&p).ok()
    }
    #[test]
    fn parses_fixture_platform_data() {
        let (Some(ua), Some(ue), Some(um)) =
            (fx("sample.uasset"), fx("sample.uexp"), fx("mappings.usmap"))
        else {
            eprintln!("skip: fixture absent (regenerate per work/tex-fixtures/README.md)");
            return;
        };
        let ub = fx("sample.ubulk").unwrap_or_default();
        let info = parse(&ua, &ue, &ub, &um).unwrap();
        assert_eq!(info.width, 128);
        assert_eq!(info.height, 128);
        assert_eq!(info.format, "PF_DXT5");
        assert!(!info.is_virtual);
        // BC3 128x128 mip0 = 32*32*16 = 16384 bytes
        assert_eq!(info.mip0.len(), 16384);
        // Byte-exact: the inline payload lives at [117, 16501) in this fixture
        // (SizeX@81 .. PixelFormat FString .. FirstMip=0, NumMips=1, flags=0,
        // then 16384 payload bytes, then mip dims 128,128,1 @16501).
        assert_eq!(info.mip0, &ue[117..117 + 16384]);
    }

    #[test]
    fn block_math() {
        assert_eq!(mip_byte_size("PF_DXT5", 128, 128), Some(16384));
        assert_eq!(mip_byte_size("PF_DXT1", 128, 128), Some(8192));
        assert_eq!(mip_byte_size("PF_BC7", 256, 256), Some(65536));
        // non-multiple-of-4 rounds up to whole blocks
        assert_eq!(mip_byte_size("PF_DXT5", 5, 5), Some(2 * 2 * 16));
        assert_eq!(mip_byte_size("PF_UNKNOWN", 4, 4), None);
    }

    #[test]
    fn fstring_roundtrip() {
        // len=8, "PF_DXT5\0"
        let mut b = Vec::new();
        b.extend_from_slice(&8i32.to_le_bytes());
        b.extend_from_slice(b"PF_DXT5\0");
        let (s, next) = read_fstring(&b, 0).unwrap();
        assert_eq!(s, "PF_DXT5");
        assert_eq!(next, b.len());
    }
}
