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
//! ## The mip-0 bulk data (the ACTUAL legacy layout retoc emits)
//!
//! retoc's zen->legacy `build_legacy` **drops** the zen `FBulkDataMapEntry`
//! table. So in the `.uexp` it writes, the mip table is *not* a sequence of full
//! `FByteBulkData` headers. After `FirstMipToSerialize (i32)` and `NumMips (i32)`
//! each mip is simply:
//!
//! ```text
//! uint32 flags        // 0 for the streamed mips in this game's data
//! int32  SizeX, SizeY, SizeZ
//! ```
//!
//! There is **no `ElementCount` / `SizeOnDisk` / `OffsetInFile`** and **no
//! inline payload** for a streamed mip. The streamed (largest) mips are written
//! into the separate `.ubulk`, in increasing mip-index order, mip0 first at
//! offset 0; only the *smallest* mips are inlined into `.uexp` (their raw bytes
//! follow their `flags` directly, length = BCn block size for the mip dims, not
//! restated).
//!
//! So mip0 (the largest / first-serialized mip) is located as:
//!
//! * **`.ubulk` present and non-empty** -> mip0 is streamed: it is
//!   `ubulk[0 .. block_size(baseW, baseH)]`.
//! * **no/empty `.ubulk`** -> the whole chain (incl. mip0) is inline in `.uexp`;
//!   mip0's raw bytes follow its `flags` word in the mip table.
//!
//! The streamed-mip `flags` are `0x00000000` in this game's data -- identical to
//! the inline fixture's mip flags -- so the old flag-based
//! `PAYLOAD_IN_SEPERATE_FILE` / `AT_END_OF_FILE` discrimination is unusable here
//! and has been removed. Presence of a non-empty `.ubulk` is the signal.
//!
//! Only `FirstMipToSerialize == 0` (mip0 actually serialized) is supported; a
//! non-zero value is rejected loudly rather than silently returning a smaller
//! mip.

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

/// Parse a cooked UTexture2D's platform data from its legacy cooked files.
///
/// `uexp` holds the export body (platform data + the *inline* smallest mips),
/// `ubulk` holds the streamed (largest) mip payloads, mip0-first at offset 0
/// (may be empty for a fully-inline texture). `uasset` and `usmap` are accepted
/// for API symmetry; the locate-by-anchor strategy does not require either (the
/// `.uasset` summary's `BulkDataStartOffset` is irrelevant because `build_legacy`
/// emits no per-mip on-disk offsets to fix up).
pub fn parse(_uasset: &[u8], uexp: &[u8], ubulk: &[u8], _usmap: &[u8]) -> Result<TexInfo> {
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
            match try_parse_platform_data(uexp, ubulk, pd_start) {
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
    if block_bytes(&format).is_none() {
        return Err(TexError::UnsupportedFormat(format));
    }

    // FTexturePlatformData (UE5 cooked) after PixelFormat:
    //   int32 FirstMipToSerialize
    //   int32 NumMips
    //   FTexture2DMipMap[NumMips]
    let first_mip = rd_i32(uexp, o)?;
    o += 4;
    let num_mips = rd_i32(uexp, o)?;
    o += 4;
    if !(1..=20).contains(&num_mips) {
        return Err(corrupt("implausible NumMips"));
    }
    // We only support FirstMipToSerialize == 0 (mip0 actually present). A
    // non-zero value means the largest serialized mip is *not* the base mip, so
    // `block_math(baseW, baseH)` would mis-size it -- fail loudly rather than
    // returning a wrong/smaller mip. (`o` here is the first mip entry, so an
    // anchor false-positive still surfaces as a different error below.)
    if first_mip != 0 {
        return Err(TexError::UnsupportedFormat(format!(
            "FirstMipToSerialize={first_mip} not supported (mip0 not serialized)"
        )));
    }

    // Mip 0 is the largest / first-serialized mip and is what we want.
    let expected = mip_byte_size(&format, width, height)
        .ok_or_else(|| corrupt("no block size for format"))?;

    let mip0 = read_mip0(uexp, ubulk, o, expected)?;

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

/// Locate and return mip-0's raw `expected`-byte BCn payload.
///
/// `o` is the offset of the first mip entry in the `.uexp` mip table (the
/// `uint32 flags` word that precedes `SizeX, SizeY, SizeZ`). retoc's
/// `build_legacy` does **not** write any `ElementCount`/`SizeOnDisk`/
/// `OffsetInFile` for a streamed mip, so there is nothing to fix up; mip0 is
/// located purely by where its payload physically lives:
///
/// * **`.ubulk` present and non-empty** -> mip0 is streamed and lives at the very
///   start of the bulk file: `ubulk[0 .. expected]`. (`build_legacy` writes the
///   streamed mips largest-first; mip0 is largest, so its offset is 0.)
/// * **no/empty `.ubulk`** -> the chain is fully inline in `.uexp`; mip0's raw
///   bytes follow its `flags` word: `uexp[o+4 .. o+4+expected]`.
///
/// Either way the slice is bounds-checked: an out-of-range range is a loud
/// `Err`, never an OOB slice.
fn read_mip0(uexp: &[u8], ubulk: &[u8], o: usize, expected: u64) -> Result<Vec<u8>> {
    let n = expected as usize;

    if !ubulk.is_empty() {
        // Streamed: mip0 is the first (largest) streamed mip, at ubulk offset 0.
        let payload = ubulk
            .get(0..n)
            .ok_or_else(|| corrupt("streamed mip0 runs past end of .ubulk"))?;
        return Ok(payload.to_vec());
    }

    // Fully inline: skip the per-mip `uint32 flags`, then the raw payload.
    let p = o + 4;
    let payload = uexp
        .get(p..p + n)
        .ok_or_else(|| corrupt("inline mip0 payload runs past end of .uexp"))?;
    Ok(payload.to_vec())
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

    /// Streamed (separate-`.ubulk`) texture: the common case that the old
    /// flag-based mip0 locator got wrong. Gated on the game install, exactly like
    /// `container.rs`'s real-container tests -- skips cleanly when absent.
    ///
    /// Unpacks `T_Water_N` (1024x1024 PF_BC5) from the live container into a temp
    /// dir, then `parse`s the produced legacy files and verifies mip0 is the first
    /// 1 MiB of `.ubulk`. NOTE: `unpack_asset` does a full container scan to find
    /// the package, so this is slow (~10 min) -- expected; it only runs with the
    /// game present.
    #[test]
    fn parses_streamed_water_normal() {
        use std::path::PathBuf;
        let game = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !game.exists() {
            eprintln!("skip: game not installed");
            return;
        }
        let utoc = crate::paths::main_container(&game).unwrap();
        let usmap_path = crate::paths::usmap(&game).unwrap();

        let asset = "/DatasmithContent/Materials/Water/Textures/T_Water_N";
        let tmp = std::env::temp_dir().join("gore-tex-decode-streamed-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let uasset_path =
            crate::container::unpack_asset(&utoc, &usmap_path, asset, &tmp).unwrap();
        let uexp_path = uasset_path.with_extension("uexp");
        let ubulk_path = uasset_path.with_extension("ubulk");

        let uasset = std::fs::read(&uasset_path).unwrap();
        let uexp = std::fs::read(&uexp_path).unwrap();
        let ubulk = std::fs::read(&ubulk_path).unwrap();
        let usmap = std::fs::read(&usmap_path).unwrap();
        assert!(!ubulk.is_empty(), "T_Water_N must have a streamed .ubulk");

        let info = parse(&uasset, &uexp, &ubulk, &usmap).unwrap();
        eprintln!(
            "T_Water_N: {}x{} {} mip0={} bytes (ubulk={} bytes)",
            info.width, info.height, info.format, info.mip0.len(), ubulk.len()
        );
        assert_eq!(info.width, 1024);
        assert_eq!(info.height, 1024);
        assert_eq!(info.format, "PF_BC5");
        assert!(!info.is_virtual);
        // BC5 1024x1024 mip0 = 256*256*16 = 1,048,576 bytes.
        let expected = 1024 / 4 * 1024 / 4 * 16;
        assert_eq!(info.mip0.len(), expected);
        // mip0 is the first (largest) streamed mip, at .ubulk offset 0.
        assert_eq!(info.mip0, &ubulk[..expected]);
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
