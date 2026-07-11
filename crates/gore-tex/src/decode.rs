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
    /// Number of VT layers for a virtual texture; `None` for a regular texture.
    /// `Some(1)` is the single-layer VT the replace path supports; `Some(n)` with
    /// `n > 1` is a multi-layer VT [`crate::vt::retile`] rejects. Populated from
    /// `FVirtualTextureBuiltData::NumLayers` (== `vt.layer_types.len()`).
    pub vt_layers: Option<u32>,
    /// True for a *legacy* VT tile layout, which [`crate::vt::retile`] rejects.
    /// `false` for regular textures and modern VTs. Gates Replace alongside
    /// `vt_layers` so a legacy VT isn't reported as replaceable.
    pub vt_legacy: bool,
    /// True when a regular (non-virtual) texture shipped a full mip chain
    /// (`mips.len() > 1`). Such sources go through `encode_mips` on replace, which
    /// requires power-of-two dimensions; a single-mip source uses `encode_tile`
    /// (multiple-of-4 only). The UI uses this to validate a replacement's size up
    /// front. `false` for single-mip and virtual textures.
    pub mipmapped: bool,
    /// Pre-decoded RGBA (`0xAARRGGBB`, `width * height` pixels) for inputs that
    /// can't go through the plain BCn mip0 path — currently virtual textures,
    /// whose layer-0 mip-0 surface is stitched from morton-ordered BCn tiles
    /// (see [`crate::vt::decode_layer0`]). `None` for regular cooked textures;
    /// [`to_rgba8`] returns it directly when present.
    pub(crate) decoded_rgba: Option<Vec<u32>>,
}

// ---- supported formats & block math ---------------------------------------

/// Bytes per 4x4 block for the supported BCn (block-compressed) formats. Returns
/// `None` for uncompressed/linear formats — use [`mip_byte_size`] to size any
/// supported mip (it also covers the linear formats).
fn block_bytes(format: &str) -> Option<u32> {
    match format {
        // 8 bytes / 4x4 block
        "PF_DXT1" | "PF_BC4" => Some(8),
        // 16 bytes / 4x4 block
        "PF_DXT5" | "PF_BC5" | "PF_BC7" | "PF_BC6H" => Some(16),
        _ => None,
    }
}

/// Bytes per pixel for the supported uncompressed (linear) formats. `None` for
/// block-compressed or unsupported formats.
fn uncompressed_bytes_per_pixel(format: &str) -> Option<u32> {
    match format {
        "PF_B8G8R8A8" => Some(4),
        "PF_G8" => Some(1),
        // 4 channels x 16-bit half-float = 8 bytes/pixel (HDR; tonemapped on decode).
        "PF_FloatRGBA" => Some(8),
        _ => None,
    }
}

/// Whether this read/decode path supports `format` (BCn block OR a known
/// uncompressed/linear format).
fn is_supported_format(format: &str) -> bool {
    block_bytes(format).is_some() || uncompressed_bytes_per_pixel(format).is_some()
}

/// Bytes of a single mip of `format` at `w` x `h`.
///
/// * BCn (block): `ceil(w/4) * ceil(h/4) * block_bytes`.
/// * Uncompressed (linear): `w * h * bytes_per_pixel`.
fn mip_byte_size(format: &str, w: u32, h: u32) -> Option<u64> {
    if let Some(bpp) = uncompressed_bytes_per_pixel(format) {
        return Some((w as u64) * (h as u64) * (bpp as u64));
    }
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
        if !is_supported_format(&layer0_format) {
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
            vt_layers: Some(vt.num_layers),
            vt_legacy: vt.is_legacy(),
            mipmapped: false,
            decoded_rgba: Some(rgba),
        });
    }
    if !is_supported_format(&pd.format) {
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
        return Err(corrupt(
            "mip0 length does not match format/dimension block math",
        ));
    }

    Ok(TexInfo {
        width: pd.size_x,
        height: pd.size_y,
        format: pd.format,
        mip0: mip0_entry.data.clone(),
        is_virtual: false,
        vt_layers: None,
        vt_legacy: false,
        mipmapped: pd.mips.len() > 1,
        decoded_rgba: None,
    })
}

/// Whether the texture **rewrite** ("Replace") path can handle `info` — the
/// authoritative capability flag the mod-studio UI gates the Replace button on.
/// It mirrors, WITHOUT a replacement image, exactly the structural constraints
/// [`crate::texdata::replace_texture_image`] enforces at build/cook time:
///
/// * **Regular texture** (`!is_virtual`): the pixel format must be re-encodable,
///   i.e. [`crate::encode::supports_format`] (BCn: `PF_DXT1`/`PF_DXT5`/`PF_BC5`/
///   `PF_BC7`). Linear and `PF_BC4`/`PF_BC6H` textures decode for preview but the
///   encoder cannot produce them, so Replace is unsupported.
/// * **Virtual texture** (`is_virtual`): [`crate::vt::retile`] hard-requires a
///   SINGLE-LAYER VT (`num_layers == 1` — it returns "multi-layer VT replace not
///   supported" otherwise) whose layer-0 tile format is encodable (it sizes the
///   physical tile via `block_bytes`, so non-BCn formats error). Hence we require
///   `vt_layers == Some(1)` AND `encode::supports_format(&format)`.
///
/// `retile` additionally rejects *legacy* VT tile layouts, so a legacy VT is
/// reported as not replaceable too (via [`TexInfo::vt_legacy`]). Legacy isn't
/// observed in G1R's cook, but gating on it keeps the flag honest rather than
/// letting such a texture fail only later at cook.
pub fn replace_supported(info: &TexInfo) -> bool {
    if info.is_virtual {
        info.vt_layers == Some(1) && !info.vt_legacy && crate::encode::supports_format(&info.format)
    } else {
        crate::encode::supports_format(&info.format)
    }
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
/// * `PF_DXT1`     -> BC1   (`decode_bc1`)
/// * `PF_DXT5`     -> BC3   (`decode_bc3`)
/// * `PF_BC5`      -> BC5   (`decode_bc5`)
/// * `PF_BC7`      -> BC7   (`decode_bc7`)
/// * `PF_BC4`      -> BC4   (`decode_bc4`, single-channel -> grayscale, see note)
/// * `PF_BC6H`     -> BC6H HDR (`decode_bc6` unsigned; tonemapped to LDR by the decoder)
/// * `PF_B8G8R8A8` -> linear BGRA8 (bytes ARE pixels; repacked to 0xAARRGGBB)
/// * `PF_G8`       -> linear 8-bit gray (one byte/pixel -> opaque gray)
/// * `PF_FloatRGBA`-> linear 16-bit half-float RGBA (HDR; clamped/tonemapped to 8-bit)
///
/// **BC5 note:** BC5 is a two-channel (RG) format, used in this game for normal
/// maps. `texture2ddecoder` fills R and G from the two compressed channels and
/// leaves **B = 0, A = 255**. That is fine for a flat preview (the texture is a
/// tangent-space normal map, not a color image) but the blue/alpha channels are
/// not meaningful.
///
/// **BC4 note:** BC4 is a single-channel format. `texture2ddecoder::decode_bc4`
/// writes the decompressed value into the **R** channel ONLY (it reuses the BC3
/// alpha-block path with channel index 2 == R in this crate's 0xAARRGGBB layout)
/// and leaves G, B, A untouched (zero from the cleared buffer) — so its raw output
/// is R-only with A = 0 (fully transparent). For a usable grayscale preview we
/// post-process each pixel: splat R into G and B and force A = 255.
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

    // Uncompressed (linear) formats: the mip0 bytes ARE the pixels — no block
    // decode. Validate the length against w*h*bpp, then repack to 0xAARRGGBB.
    match info.format.as_str() {
        "PF_B8G8R8A8" => {
            // Each pixel is 4 bytes in B, G, R, A order. Repack to the pipeline's
            // 0xAARRGGBB u32 (= in-memory B,G,R,A on a little-endian host).
            let need = w * h * 4;
            if info.mip0.len() < need {
                return Err(TexError::DecodeFailed {
                    format: info.format.clone(),
                    reason: format!(
                        "B8G8R8A8 mip0 is {} bytes, need {need} for {w}x{h}",
                        info.mip0.len()
                    ),
                });
            }
            for (i, px) in image.iter_mut().enumerate() {
                let b = info.mip0[i * 4] as u32;
                let g = info.mip0[i * 4 + 1] as u32;
                let r = info.mip0[i * 4 + 2] as u32;
                let a = info.mip0[i * 4 + 3] as u32;
                *px = (a << 24) | (r << 16) | (g << 8) | b;
            }
            return Ok(image);
        }
        "PF_G8" => {
            // One byte per pixel = gray; emit opaque gray (R=G=B=g, A=255).
            let need = w * h;
            if info.mip0.len() < need {
                return Err(TexError::DecodeFailed {
                    format: info.format.clone(),
                    reason: format!(
                        "G8 mip0 is {} bytes, need {need} for {w}x{h}",
                        info.mip0.len()
                    ),
                });
            }
            for (i, px) in image.iter_mut().enumerate() {
                let g = info.mip0[i] as u32;
                *px = (0xFF << 24) | (g << 16) | (g << 8) | g;
            }
            return Ok(image);
        }
        "PF_FloatRGBA" => {
            // 8 bytes/pixel: four 16-bit IEEE half-floats in R,G,B,A order (HDR).
            // Decode each half to f32 and TONEMAP to 8-bit for a usable preview:
            // simple clamp to [0,1] then *255 (no exposure/Reinhard curve — these
            // are preview thumbnails, and clamping ensures values >1.0 don't wrap).
            let need = w * h * 8;
            if info.mip0.len() < need {
                return Err(TexError::DecodeFailed {
                    format: info.format.clone(),
                    reason: format!(
                        "FloatRGBA mip0 is {} bytes, need {need} for {w}x{h}",
                        info.mip0.len()
                    ),
                });
            }
            let to_u8 = |bytes: &[u8]| -> u32 {
                let bits = u16::from_le_bytes([bytes[0], bytes[1]]);
                let v = half::f16::from_bits(bits).to_f32();
                (v.clamp(0.0, 1.0) * 255.0).round() as u32
            };
            for (i, px) in image.iter_mut().enumerate() {
                let base = i * 8;
                let r = to_u8(&info.mip0[base..base + 2]);
                let g = to_u8(&info.mip0[base + 2..base + 4]);
                let b = to_u8(&info.mip0[base + 4..base + 6]);
                let a = to_u8(&info.mip0[base + 6..base + 8]);
                *px = (a << 24) | (r << 16) | (g << 8) | b;
            }
            return Ok(image);
        }
        _ => {}
    }

    let res = match info.format.as_str() {
        "PF_DXT1" => texture2ddecoder::decode_bc1(&info.mip0, w, h, &mut image),
        "PF_DXT5" => texture2ddecoder::decode_bc3(&info.mip0, w, h, &mut image),
        "PF_BC5" => texture2ddecoder::decode_bc5(&info.mip0, w, h, &mut image),
        "PF_BC7" => texture2ddecoder::decode_bc7(&info.mip0, w, h, &mut image),
        "PF_BC4" => texture2ddecoder::decode_bc4(&info.mip0, w, h, &mut image),
        // BC6H is HDR; UE cooks it UNSIGNED for nearly all assets, so decode as
        // unsigned (signed=false). texture2ddecoder tonemaps internally
        // (f16 -> clamp [0,1] * 255) and emits opaque 0xAARRGGBB LDR u32s, so the
        // output is used directly like the other BCn arms — no extra clamp here.
        "PF_BC6H" => texture2ddecoder::decode_bc6(&info.mip0, w, h, &mut image, false),
        _ => return Err(TexError::UnsupportedFormat(info.format.clone())),
    };

    res.map_err(|reason| TexError::DecodeFailed {
        format: info.format.clone(),
        reason: reason.to_string(),
    })?;

    // BC4 is single-channel: decode_bc4 writes only R (channel 2 == R in the
    // 0xAARRGGBB layout) and leaves G=B=A=0. Promote to an opaque grayscale
    // preview: splat R into G and B and force A=255.
    if info.format == "PF_BC4" {
        for px in image.iter_mut() {
            let r = (*px >> 16) & 0xff;
            *px = (0xFF << 24) | (r << 16) | (r << 8) | r;
        }
    }

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

        let uasset_path = crate::container::unpack_asset(&utoc, &usmap_path, asset, &tmp).unwrap();
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
            info.width,
            info.height,
            info.format,
            info.mip0.len(),
            ubulk.len()
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
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
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
        // BC4: 8 bytes / 4x4 block (like BC1).
        assert_eq!(mip_byte_size("PF_BC4", 128, 128), Some(8192));
        assert_eq!(mip_byte_size("PF_BC4", 5, 5), Some(2 * 2 * 8));
        // Uncompressed (linear): w*h*bpp, NOT block math.
        assert_eq!(mip_byte_size("PF_B8G8R8A8", 4, 4), Some(4 * 4 * 4));
        assert_eq!(mip_byte_size("PF_B8G8R8A8", 1, 1), Some(4));
        assert_eq!(mip_byte_size("PF_G8", 4, 4), Some(16));
        assert_eq!(mip_byte_size("PF_G8", 3, 5), Some(15));
        // Supported-format gate.
        assert!(is_supported_format("PF_BC4"));
        assert!(is_supported_format("PF_B8G8R8A8"));
        assert!(is_supported_format("PF_G8"));
        assert!(!is_supported_format("PF_UNKNOWN"));
    }

    /// `replace_supported` mirrors the rewrite path's structural gating:
    /// regular textures need an encodable format; VTs additionally need exactly
    /// one layer.
    #[test]
    fn replace_supported_gates_correctly() {
        let regular = |fmt: &str| TexInfo {
            width: 4,
            height: 4,
            format: fmt.into(),
            mip0: Vec::new(),
            is_virtual: false,
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
            decoded_rgba: None,
        };
        let vt = |fmt: &str, layers: u32, legacy: bool| TexInfo {
            width: 4,
            height: 4,
            format: fmt.into(),
            mip0: Vec::new(),
            is_virtual: true,
            vt_layers: Some(layers),
            vt_legacy: legacy,
            mipmapped: false,
            decoded_rgba: Some(vec![0u32; 16]),
        };

        // Regular: encodable -> true; non-encodable (BC6H, linear) -> false.
        assert!(replace_supported(&regular("PF_DXT5")));
        assert!(replace_supported(&regular("PF_BC7")));
        assert!(!replace_supported(&regular("PF_BC6H")));
        assert!(!replace_supported(&regular("PF_B8G8R8A8")));

        // Virtual: single-layer + encodable -> true; multi-layer -> false even if
        // the tile format is encodable; non-encodable single-layer -> false; a
        // legacy layout -> false even when single-layer + encodable.
        assert!(replace_supported(&vt("PF_DXT1", 1, false)));
        assert!(!replace_supported(&vt("PF_DXT1", 2, false)));
        assert!(!replace_supported(&vt("PF_BC6H", 1, false)));
        assert!(!replace_supported(&vt("PF_DXT1", 1, true)));
    }

    /// Pure unit test: a 2x1 `PF_B8G8R8A8` surface. Input bytes are B,G,R,A per
    /// pixel; the output must be 0xAARRGGBB (so previews are not channel-swapped).
    #[test]
    fn decode_b8g8r8a8_channel_order() {
        // px0 = B=0x10 G=0x20 R=0x30 A=0x40 -> 0x40302010
        // px1 = B=0xAA G=0xBB R=0xCC A=0xFF -> 0xFFCCBBAA
        let mip0 = vec![0x10, 0x20, 0x30, 0x40, 0xAA, 0xBB, 0xCC, 0xFF];
        let info = TexInfo {
            width: 2,
            height: 1,
            format: "PF_B8G8R8A8".into(),
            mip0,
            is_virtual: false,
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
            decoded_rgba: None,
        };
        let px = to_rgba8(&info).unwrap();
        assert_eq!(px, vec![0x4030_2010, 0xFFCC_BBAA]);
    }

    /// Pure unit test: a 2x1 `PF_G8` surface decodes to opaque gray (R=G=B=g, A=255).
    #[test]
    fn decode_g8_is_opaque_gray() {
        let mip0 = vec![0x00, 0x7F];
        let info = TexInfo {
            width: 2,
            height: 1,
            format: "PF_G8".into(),
            mip0,
            is_virtual: false,
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
            decoded_rgba: None,
        };
        let px = to_rgba8(&info).unwrap();
        assert_eq!(px, vec![0xFF00_0000, 0xFF7F_7F7F]);
    }

    /// Pure unit test: a `PF_FloatRGBA` pixel built from half-floats
    /// (1.0, 0.0, 0.0, 1.0) decodes to opaque red `0xFFFF0000`.
    #[test]
    fn decode_floatrgba_opaque_red() {
        let mut mip0 = Vec::new();
        for v in [1.0f32, 0.0, 0.0, 1.0] {
            mip0.extend_from_slice(&half::f16::from_f32(v).to_bits().to_le_bytes());
        }
        let info = TexInfo {
            width: 1,
            height: 1,
            format: "PF_FloatRGBA".into(),
            mip0,
            is_virtual: false,
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
            decoded_rgba: None,
        };
        let px = to_rgba8(&info).unwrap();
        assert_eq!(px, vec![0xFFFF_0000]);
    }

    /// Pure unit test: an HDR `PF_FloatRGBA` value > 1.0 (half 4.0 in R) CLAMPS to
    /// 255 rather than wrapping. Verifies the tonemap clamp.
    #[test]
    fn decode_floatrgba_hdr_clamps_not_wraps() {
        let mut mip0 = Vec::new();
        // R=4.0 (HDR, >1), G=0, B=0, A=1.0
        for v in [4.0f32, 0.0, 0.0, 1.0] {
            mip0.extend_from_slice(&half::f16::from_f32(v).to_bits().to_le_bytes());
        }
        let info = TexInfo {
            width: 1,
            height: 1,
            format: "PF_FloatRGBA".into(),
            mip0,
            is_virtual: false,
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
            decoded_rgba: None,
        };
        let px = to_rgba8(&info).unwrap();
        // R clamps to 255 (not wrapped), opaque red.
        assert_eq!(px, vec![0xFFFF_0000]);
    }

    /// Pure unit test: a solid BC4 block (single channel) decodes to opaque gray.
    /// A BC4 block is the BC3 alpha-block layout: [a0, a1, 6 bytes of 3-bit idx].
    /// a0 = a1 = 0xC0 with all indices 0 selects a0 for every pixel -> gray 0xC0,
    /// promoted to R=G=B=0xC0, A=255.
    #[test]
    fn decode_solid_bc4_block_is_opaque_gray() {
        let mut block = vec![0xC0u8, 0xC0];
        block.extend_from_slice(&[0u8; 6]); // all indices 0
        let info = TexInfo {
            width: 4,
            height: 4,
            format: "PF_BC4".into(),
            mip0: block,
            is_virtual: false,
            vt_layers: None,
            vt_legacy: false,
            mipmapped: false,
            decoded_rgba: None,
        };
        let px = to_rgba8(&info).unwrap();
        assert_eq!(px.len(), 16);
        for (i, &p) in px.iter().enumerate() {
            let a = (p >> 24) & 0xff;
            let r = (p >> 16) & 0xff;
            let g = (p >> 8) & 0xff;
            let b = p & 0xff;
            assert_eq!(a, 255, "pixel {i}: A={a} not opaque ({p:#010x})");
            assert_eq!(r, 0xC0, "pixel {i}: R={r} not 0xC0 ({p:#010x})");
            assert_eq!(g, r, "pixel {i}: G != R ({p:#010x})");
            assert_eq!(b, r, "pixel {i}: B != R ({p:#010x})");
        }
    }

    // ---- gated real-asset decode tests -----------------------------------
    //
    // Each unpacks a real cooked texture (via the cached texture index ->
    // by-id unpack, no full scan) and asserts it decodes to a real image.
    // Skips cleanly when the game or the cached index is absent.

    fn game_dir() -> Option<std::path::PathBuf> {
        let p = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }

    /// Resolve `asset` to its package id via the cached texture index. `None` if
    /// the index is absent or the asset is not in it.
    fn pid_for(asset: &str) -> Option<u64> {
        let idx = crate::index::TextureIndex::load(&crate::paths::texture_index_path()).ok()?;
        idx.entries.get(asset).copied()
    }

    /// Resolve the first index entry whose asset path CONTAINS `needle` to its
    /// `(asset_path, package_id)`. Used for assets we know by leaf/substring rather
    /// than full path. `None` if the index is absent or nothing matches.
    fn pid_containing(needle: &str) -> Option<(String, u64)> {
        let idx = crate::index::TextureIndex::load(&crate::paths::texture_index_path()).ok()?;
        idx.entries
            .iter()
            .find(|(path, _)| path.contains(needle))
            .map(|(path, &pid)| (path.clone(), pid))
    }

    /// True if the image has at least two distinct pixel values (i.e. not a
    /// flat/all-zero/all-identical surface) — a cheap "is this a real image" check.
    fn is_real_image(px: &[u32]) -> bool {
        px.first()
            .map(|&first| px.iter().any(|&p| p != first))
            .unwrap_or(false)
    }

    fn extract(asset: &str, leaf: &str) -> Option<(TexInfo, Vec<u32>)> {
        let g = game_dir()?;
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let pid = pid_for(asset)?;
        Some(crate::index::extract_by_package_id(&utoc, &usmap, pid, leaf).unwrap())
    }

    #[test]
    #[ignore = "slow: unpacks from real container; needs game + cached index"]
    fn decode_real_b8g8r8a8_default_alpha_texture() {
        let Some((info, px)) = extract(
            "/Engine/EditorLandscapeResources/DefaultAlphaTexture",
            "DefaultAlphaTexture",
        ) else {
            eprintln!("skip: game or cached index absent");
            return;
        };
        eprintln!(
            "DefaultAlphaTexture: {}x{} {} px={}",
            info.width,
            info.height,
            info.format,
            px.len()
        );
        assert_eq!(info.format, "PF_B8G8R8A8");
        assert!(info.width >= 1 && info.height >= 1 && info.width <= 16384 && info.height <= 16384);
        assert_eq!(px.len(), (info.width * info.height) as usize);
        assert!(
            is_real_image(&px),
            "B8G8R8A8 decoded to a flat/identical image"
        );
    }

    #[test]
    #[ignore = "slow: unpacks from real container; needs game + cached index"]
    fn decode_real_g8_roboto_distance_field() {
        let Some((info, px)) = extract(
            "/Engine/EngineFonts/RobotoDistanceField",
            "RobotoDistanceField",
        ) else {
            eprintln!("skip: game or cached index absent");
            return;
        };
        eprintln!(
            "RobotoDistanceField: {}x{} {} px={}",
            info.width,
            info.height,
            info.format,
            px.len()
        );
        assert_eq!(info.format, "PF_G8");
        assert!(info.width >= 1 && info.height >= 1 && info.width <= 16384 && info.height <= 16384);
        assert_eq!(px.len(), (info.width * info.height) as usize);
        assert!(is_real_image(&px), "G8 decoded to a flat/identical image");
    }

    #[test]
    #[ignore = "slow: unpacks from real container; needs game + cached index"]
    fn decode_real_bc4_meatbug_eye_mask() {
        let Some((info, px)) = extract(
            "/Game/Assets/Characters/Creatures/Meatbug_Crushed/Model/Textures/Optimized/T_MeatBug_Crushed_EyeMask",
            "T_MeatBug_Crushed_EyeMask",
        ) else {
            eprintln!("skip: game or cached index absent");
            return;
        };
        eprintln!(
            "T_MeatBug_Crushed_EyeMask: {}x{} {} px={}",
            info.width,
            info.height,
            info.format,
            px.len()
        );
        assert_eq!(info.format, "PF_BC4");
        assert!(info.width >= 1 && info.height >= 1 && info.width <= 16384 && info.height <= 16384);
        assert_eq!(px.len(), (info.width * info.height) as usize);
        assert!(is_real_image(&px), "BC4 decoded to a flat/identical image");
    }

    /// PF_FloatRGBA: the engine `Black_1x1_EXR_Texture_VT` asset that previously
    /// failed with "unsupported pixel format: PF_FloatRGBA". It is a 1x1 BLACK
    /// virtual texture, so we assert it DECODES cleanly with the right format and
    /// pixel count — NOT that it is a non-flat image (all-black is the expected,
    /// correct content here).
    #[test]
    #[ignore = "slow: unpacks from real container; needs game + cached index"]
    fn decode_real_floatrgba_black_1x1_exr() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let Some((asset, pid)) = pid_containing("Black_1x1_EXR_Texture_VT") else {
            eprintln!("skip: cached index absent or asset not found");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let leaf = asset.rsplit('/').next().unwrap_or(&asset);
        let (info, px) = crate::index::extract_by_package_id(&utoc, &usmap, pid, leaf).unwrap();
        eprintln!(
            "Black_1x1_EXR_Texture_VT ({asset}): {}x{} {} px={}",
            info.width,
            info.height,
            info.format,
            px.len()
        );
        assert_eq!(info.format, "PF_FloatRGBA");
        assert!(info.width >= 1 && info.height >= 1 && info.width <= 16384 && info.height <= 16384);
        assert_eq!(px.len(), (info.width * info.height) as usize);
        // Black EXR fill texture: every pixel has RGB == 0 (the expected content).
        // Do NOT require non-flat content — a flat black image is correct here.
        // (Alpha is whatever the source EXR carries — observed A=0, i.e. fully
        // transparent black — so we do NOT assert opacity, only that R=G=B=0.)
        for (i, &p) in px.iter().enumerate() {
            let r = (p >> 16) & 0xff;
            let g = (p >> 8) & 0xff;
            let b = p & 0xff;
            assert_eq!((r, g, b), (0, 0, 0), "pixel {i}: RGB not black ({p:#010x})");
        }
    }

    /// PF_BC6H (HDR block). Scans the cached index for a real BC6H asset by
    /// decoding candidates; if none is found quickly it SKIPS with an eprintln
    /// rather than failing (BC6H is rare in this game's cook). When found, asserts
    /// it decodes to a real (non-flat) image.
    #[test]
    #[ignore = "slow: scans real container for a BC6H asset; needs game + cached index"]
    fn decode_real_bc6h_if_present() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let Some(idx) = crate::index::TextureIndex::load(&crate::paths::texture_index_path()).ok()
        else {
            eprintln!("skip: cached index absent");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();

        // Prefer paths that hint at HDR/cubemap content to keep the scan short.
        let mut candidates: Vec<(&String, &u64)> = idx
            .entries
            .iter()
            .filter(|(p, _)| {
                let lp = p.to_ascii_lowercase();
                lp.contains("hdr")
                    || lp.contains("cubemap")
                    || lp.contains("_cube")
                    || lp.contains("skylight")
                    || lp.contains("specular")
            })
            .collect();
        candidates.extend(idx.entries.iter());

        let mut scanned = 0usize;
        for (asset, &pid) in candidates.into_iter().take(400) {
            scanned += 1;
            let leaf = asset.rsplit('/').next().unwrap_or(asset);
            let Ok((info, px)) = crate::index::extract_by_package_id(&utoc, &usmap, pid, leaf)
            else {
                continue;
            };
            if info.format == "PF_BC6H" {
                eprintln!(
                    "BC6H asset found after {scanned} candidates: {asset} {}x{} px={}",
                    info.width,
                    info.height,
                    px.len()
                );
                assert_eq!(px.len(), (info.width * info.height) as usize);
                assert!(is_real_image(&px), "BC6H decoded to a flat/identical image");
                return;
            }
        }
        eprintln!("skip: no PF_BC6H asset located in {scanned} scanned index entries");
    }
}
