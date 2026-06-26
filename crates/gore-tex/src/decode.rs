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
    /// Raw BCn bytes of mip 0 (largest), as stored on disk. Empty for a virtual
    /// texture (which has no single linear surface — its pixels come pre-decoded
    /// in [`Self::decoded_rgba`]).
    pub mip0: Vec<u8>,
    pub is_virtual: bool,
    /// Pre-decoded RGBA (`0xAARRGGBB`, `width * height` pixels) for inputs that
    /// can't go through the plain BCn mip0 path — currently virtual textures,
    /// whose layer-0 mip-0 surface is stitched from morton-ordered BCn tiles
    /// (see [`crate::vt::decode_layer0`]). `None` for regular cooked textures;
    /// [`to_rgba8`] returns it directly when present.
    pub(crate) decoded_rgba: Option<Vec<u32>>,
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

fn corrupt(msg: &str) -> TexError {
    TexError::Retoc(anyhow::anyhow!("cooked texture parse: {msg}"))
}

/// Parse a cooked UTexture2D's platform data from its legacy cooked files.
///
/// `uexp` holds the export body (platform data + the *inline* smallest mips),
/// `ubulk` holds the streamed (largest) mip payloads, mip0-first at offset 0
/// (may be empty for a fully-inline texture). `uasset` and `usmap` are accepted
/// for API symmetry; the locate-by-anchor strategy does not require either (the
/// `.uasset` summary's `BulkDataStartOffset` is irrelevant because `build_legacy`
/// emits no per-mip on-disk offsets to fix up).
pub fn parse(uasset: &[u8], uexp: &[u8], ubulk: &[u8], _usmap: &[u8]) -> Result<TexInfo> {
    // Delegate the byte-faithful FTexturePlatformData parse to the codec, then
    // project the fields this read path needs into `TexInfo`. The codec captures
    // the full region (all mips, trailer, FString encoding); we keep only mip0.
    let pd = crate::texdata::PlatformData::parse(uasset, uexp, ubulk)?;

    // Virtual textures have no single linear mip0 surface: their layer-0 mip-0
    // image is stitched from morton-ordered BCn tiles. Resolve every chunk's raw
    // bytes (single-layer VTs reference all chunks from layer 0) and decode the
    // preview here, carrying the result as pre-decoded RGBA.
    if let Some(vt) = pd.vt.as_ref() {
        let layer0_format = vt
            .layer_types
            .first()
            .cloned()
            .ok_or_else(|| corrupt("virtual texture has no layer types"))?;
        if block_bytes(&layer0_format).is_none() {
            return Err(TexError::UnsupportedFormat(layer0_format));
        }
        let mut chunk_bytes = Vec::with_capacity(vt.chunks.len());
        for ch in &vt.chunks {
            chunk_bytes.push(crate::texdata::resolve_data_resource_bytes(
                uasset,
                uexp,
                ubulk,
                ch.data_resource_index,
            )?);
        }
        let (w, h, rgba) = crate::vt::decode_layer0(vt, &chunk_bytes, &layer0_format)?;
        return Ok(TexInfo {
            width: w,
            height: h,
            format: layer0_format,
            mip0: Vec::new(),
            is_virtual: true,
            decoded_rgba: Some(rgba),
        });
    }
    if block_bytes(&pd.format).is_none() {
        return Err(TexError::UnsupportedFormat(pd.format));
    }

    // mip0 = the first (largest / first-serialized) mip. The codec already
    // rejects FirstMipToSerialize != 0, so mips[0] is the base mip.
    let mip0_entry = pd
        .mips
        .first()
        .ok_or_else(|| corrupt("platform data has no mips"))?;

    // Hard validation: the captured byte length must equal the BCn block math
    // for the base dims. This is what makes the heuristic safe.
    let expected = mip_byte_size(&pd.format, pd.size_x, pd.size_y)
        .ok_or_else(|| corrupt("no block size for format"))?;
    if mip0_entry.data.len() as u64 != expected {
        return Err(corrupt("mip0 length does not match format/dimension block math"));
    }

    Ok(TexInfo {
        width: pd.size_x,
        height: pd.size_y,
        format: pd.format,
        mip0: mip0_entry.data.clone(),
        is_virtual: false,
        decoded_rgba: None,
    })
}

// ---- BCn -> RGBA decode ---------------------------------------------------

/// Decode [`TexInfo::mip0`] (block-compressed BCn bytes) to a row-major RGBA8
/// image, one `u32` per pixel. Returns `width * height` pixels.
///
/// ## Channel order of the returned `u32`s
///
/// Each pixel is packed **`0xAARRGGBB`** (ARGB in a native `u32`), which is the
/// in-memory byte order **B, G, R, A** (BGRA) on a little-endian host. This is
/// what `texture2ddecoder` emits: its internal `color(r, g, b, a)` constructs
/// the pixel as `u32::from_le_bytes([b, g, r, a])`. Task 9 (PNG packing) must
/// extract bytes as `r = (px >> 16) & 0xff`, `g = (px >> 8) & 0xff`,
/// `b = px & 0xff`, `a = (px >> 24) & 0xff`.
///
/// ## Format dispatch
///
/// * `PF_DXT1` -> BC1   (`decode_bc1`)
/// * `PF_DXT5` -> BC3   (`decode_bc3`)
/// * `PF_BC5`  -> BC5   (`decode_bc5`)
/// * `PF_BC7`  -> BC7   (`decode_bc7`)
///
/// **BC5 note:** BC5 is a two-channel (RG) format, used in this game for normal
/// maps. `texture2ddecoder` fills R and G from the two compressed channels and
/// leaves **B = 0, A = 255**. That is fine for a flat preview (the texture is a
/// tangent-space normal map, not a color image) but the blue/alpha channels are
/// not meaningful.
///
/// Any unrecognized `format` returns [`TexError::UnsupportedFormat`]; a decoder
/// failure (e.g. truncated block data) returns [`TexError::DecodeFailed`].
pub fn to_rgba8(info: &TexInfo) -> Result<Vec<u32>> {
    // Pre-decoded inputs (virtual textures) carry their stitched RGBA directly.
    if let Some(rgba) = &info.decoded_rgba {
        return Ok(rgba.clone());
    }

    let w = info.width as usize;
    let h = info.height as usize;
    let mut image = vec![0u32; w * h];

    let res = match info.format.as_str() {
        "PF_DXT1" => texture2ddecoder::decode_bc1(&info.mip0, w, h, &mut image),
        "PF_DXT5" => texture2ddecoder::decode_bc3(&info.mip0, w, h, &mut image),
        "PF_BC5" => texture2ddecoder::decode_bc5(&info.mip0, w, h, &mut image),
        "PF_BC7" => texture2ddecoder::decode_bc7(&info.mip0, w, h, &mut image),
        _ => return Err(TexError::UnsupportedFormat(info.format.clone())),
    };

    res.map_err(|reason| TexError::DecodeFailed {
        format: info.format.clone(),
        reason: reason.to_string(),
    })?;

    Ok(image)
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
    #[ignore = "slow: full container scan; run with --ignored"]
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

    /// Pure unit test (no game, no fixture): hand-build a solid-red BC1 (DXT1)
    /// block and assert every decoded pixel is red. Verifies both the format
    /// dispatch and the documented `0xAARRGGBB` channel order.
    #[test]
    fn decode_solid_bc1_block_is_red() {
        // BC1 (DXT1) 8-byte block: [c0:u16le, c1:u16le, 4 bytes of 2-bit idx].
        // c0 = c1 = 0xF800 = RGB565 (R=31, G=0, B=0) => pure red. Indices all 0
        // => every one of the 16 pixels selects color0.
        let mut block = Vec::with_capacity(8);
        block.extend_from_slice(&0xF800u16.to_le_bytes()); // c0
        block.extend_from_slice(&0xF800u16.to_le_bytes()); // c1
        block.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // indices

        let info = TexInfo {
            width: 4,
            height: 4,
            format: "PF_DXT1".into(),
            mip0: block,
            is_virtual: false,
            decoded_rgba: None,
        };
        let px = to_rgba8(&info).unwrap();
        assert_eq!(px.len(), 16);

        for (i, &p) in px.iter().enumerate() {
            // Documented order: 0xAARRGGBB. Mask channels rather than hardcoding
            // a single u32 so the assertion is robust regardless of host endian.
            let a = (p >> 24) & 0xff;
            let r = (p >> 16) & 0xff;
            let g = (p >> 8) & 0xff;
            let b = p & 0xff;
            assert!(r >= 248, "pixel {i}: R={r} not ~255 ({p:#010x})");
            assert_eq!(g, 0, "pixel {i}: G={g} not 0 ({p:#010x})");
            assert_eq!(b, 0, "pixel {i}: B={b} not 0 ({p:#010x})");
            assert_eq!(a, 255, "pixel {i}: A={a} not 255 ({p:#010x})");
        }
    }

    /// Decode the local fixture's mip0 to RGBA. Gated: skips if the fixture is
    /// absent so CI without the fixture stays green.
    #[test]
    fn decode_fixture_to_rgba() {
        let (Some(ua), Some(ue), Some(um)) =
            (fx("sample.uasset"), fx("sample.uexp"), fx("mappings.usmap"))
        else {
            eprintln!("skip: fixture absent");
            return;
        };
        let ub = fx("sample.ubulk").unwrap_or_default();
        let info = parse(&ua, &ue, &ub, &um).unwrap();
        let px = to_rgba8(&info).unwrap();
        assert_eq!(px.len(), (info.width * info.height) as usize);
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
}
