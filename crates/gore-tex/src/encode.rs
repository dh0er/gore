//! RGBA8 -> BCn encode + mip-pyramid generation (the write path's inverse of
//! [`crate::decode`]).
//!
//! [`encode_mips`] takes a single full-resolution RGBA8 image, builds the full
//! mip chain down to 1x1 by 2x2 box-averaging, and BCn-encodes every level via
//! Intel's ISPC texture compressor (`intel_tex_2`). The result is one
//! `Vec<u8>` per mip, largest first, each length-checked against the format's
//! BCn block math so the caller can splice them straight back into a cooked
//! `FTexturePlatformData`.
//!
//! ## Input channel order
//!
//! `rgba` is row-major, **byte order R, G, B, A** (i.e. `rgba[4*i+0]` is red).
//! This is the natural PNG/image byte order, *not* the `0xAARRGGBB`-packed
//! `u32`s that [`crate::decode::to_rgba8`] returns. A caller decoding a `u32`
//! image first must unpack to RGBA bytes (`r=(px>>16)&0xff`, `g=(px>>8)&0xff`,
//! `b=px&0xff`, `a=(px>>24)&0xff`) before handing it here.
//!
//! ## Dimension rules
//!
//! BCn works on 4x4 blocks and a clean mip chain to 1x1 requires the base be a
//! power of two, so [`encode_mips`] requires both dims be multiples of 4 *and*
//! powers of two. Anything else is a loud error rather than a silently padded
//! or truncated texture.
//!
//! ## BC5 channel handling
//!
//! BC5 is a two-channel (R,G) format used in this game for tangent-space normal
//! maps. The encoder is fed only the R and G channels (tightly packed,
//! `stride = width*2`); the input's **B and A bytes are dropped**. That matches
//! how the cooked game data stores normal maps and how [`crate::decode`] reads
//! them back (B=0, A=255). Do not route a color image through `PF_BC5`.

use crate::error::{Result, TexError};
use intel_tex_2::{bc1, bc3, bc5, bc7, RgSurface, RgbaSurface};

/// BCn block-byte size for a 4x4 block of `format`. Mirrors
/// [`crate::decode`]'s table: BC1 = 8 bytes/block, BC3/BC5/BC7 = 16.
fn block_bytes(format: &str) -> Option<u32> {
    match format {
        "PF_DXT1" => Some(8),
        "PF_DXT5" | "PF_BC5" | "PF_BC7" => Some(16),
        _ => None,
    }
}

/// Encoded byte size of a single `w` x `h` mip in `format` (BCn, 4x4 blocks,
/// dims rounded up to whole blocks). `None` for an unknown format.
fn mip_byte_size(format: &str, w: u32, h: u32) -> Option<usize> {
    let bb = block_bytes(format)? as usize;
    let blocks_x = ((w as usize) + 3) / 4;
    let blocks_y = ((h as usize) + 3) / 4;
    Some(blocks_x * blocks_y * bb)
}

/// Generate the full mip pyramid for `rgba` (row-major RGBA8, `width*height`
/// pixels), BCn-encoded to `format`. Returns one `Vec<u8>` per mip, largest
/// first (mip0 = full res) down to 1x1.
///
/// `NumMips = log2(max(width, height)) + 1`.
///
/// # Errors
///
/// * [`TexError::EncodeFailed`] if `width`/`height` are not both multiples of 4
///   and powers of two, or if `rgba.len() != width*height*4`.
/// * [`TexError::UnsupportedFormat`] if `format` is not one of `PF_DXT1`,
///   `PF_DXT5`, `PF_BC5`, `PF_BC7`.
pub fn encode_mips(rgba: &[u8], width: u32, height: u32, format: &str) -> Result<Vec<Vec<u8>>> {
    // --- validate format first (cheap, gives the clearest error) -----------
    if block_bytes(format).is_none() {
        return Err(TexError::UnsupportedFormat(format.to_string()));
    }

    // --- validate dimensions ----------------------------------------------
    if width == 0 || height == 0 {
        return Err(TexError::EncodeFailed(format!(
            "zero dimension not allowed: {width}x{height}"
        )));
    }
    if width % 4 != 0 || height % 4 != 0 {
        return Err(TexError::EncodeFailed(format!(
            "dimensions {width}x{height} must both be multiples of 4 for BCn block encoding"
        )));
    }
    if !width.is_power_of_two() || !height.is_power_of_two() {
        return Err(TexError::EncodeFailed(format!(
            "dimensions {width}x{height} must both be powers of two for a clean 1x1 mip chain"
        )));
    }

    // --- validate buffer length -------------------------------------------
    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|wh| wh.checked_mul(4))
        .ok_or_else(|| TexError::EncodeFailed(format!("dimensions {width}x{height} overflow")))?;
    if rgba.len() != expected_len {
        return Err(TexError::EncodeFailed(format!(
            "rgba length {} != width*height*4 ({}) for {width}x{height}",
            rgba.len(),
            expected_len
        )));
    }

    let num_mips = (32 - width.max(height).leading_zeros()) as usize; // log2(max)+1
    let mut out = Vec::with_capacity(num_mips);

    // `cur` holds the current mip level's RGBA8 (mip0 = the input, owned copy).
    let mut cur: Vec<u8> = rgba.to_vec();
    let mut w = width;
    let mut h = height;

    loop {
        out.push(encode_level(&cur, w, h, format)?);
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = (w.max(2) / 2, h.max(2) / 2);
        cur = downsample_2x2(&cur, w, h);
        w = nw;
        h = nh;
    }

    debug_assert_eq!(out.len(), num_mips, "mip count mismatch");
    Ok(out)
}

/// BCn-encode a single arbitrary-size RGBA8 surface (`w`x`h`, both multiples of
/// 4) to `format`, using the *same* `intel_tex_2` mapping [`encode_mips`] uses.
///
/// Unlike [`encode_mips`] this does **not** require power-of-two dims or build a
/// mip chain — it's the per-tile encoder the VT re-tiler ([`crate::vt::retile`])
/// needs for physical (bordered) tiles such as 136x136. The output length is
/// verified against the format's BCn block math (`block_math(w,h,format)`).
///
/// # Errors
/// * [`TexError::UnsupportedFormat`] if `format` is not a supported BCn format.
/// * [`TexError::EncodeFailed`] if `w`/`h` are not multiples of 4, the buffer
///   length is wrong, or the encoder output length doesn't match block math.
pub(crate) fn encode_tile(rgba: &[u8], w: u32, h: u32, format: &str) -> Result<Vec<u8>> {
    if block_bytes(format).is_none() {
        return Err(TexError::UnsupportedFormat(format.to_string()));
    }
    if w == 0 || h == 0 {
        return Err(TexError::EncodeFailed(format!(
            "zero tile dimension not allowed: {w}x{h}"
        )));
    }
    if w % 4 != 0 || h % 4 != 0 {
        return Err(TexError::EncodeFailed(format!(
            "tile dimensions {w}x{h} must both be multiples of 4 for BCn block encoding"
        )));
    }
    let expected_len = (w as usize)
        .checked_mul(h as usize)
        .and_then(|wh| wh.checked_mul(4))
        .ok_or_else(|| TexError::EncodeFailed(format!("tile dimensions {w}x{h} overflow")))?;
    if rgba.len() != expected_len {
        return Err(TexError::EncodeFailed(format!(
            "tile rgba length {} != w*h*4 ({}) for {w}x{h}",
            rgba.len(),
            expected_len
        )));
    }
    encode_level(rgba, w, h, format)
}

/// BCn-encode one mip level's RGBA8 (`w`x`h`) to `format`. Verifies the encoder
/// output length matches the format's block math.
fn encode_level(rgba: &[u8], w: u32, h: u32, format: &str) -> Result<Vec<u8>> {
    let encoded = match format {
        "PF_DXT1" => bc1::compress_blocks(&RgbaSurface {
            data: rgba,
            width: w,
            height: h,
            stride: w * 4,
        }),
        "PF_DXT5" => bc3::compress_blocks(&RgbaSurface {
            data: rgba,
            width: w,
            height: h,
            stride: w * 4,
        }),
        "PF_BC5" => {
            // BC5 wants a tightly-packed 2-channel (R,G) surface. Drop B and A.
            let rg = pack_rg(rgba);
            bc5::compress_blocks(&RgSurface {
                data: &rg,
                width: w,
                height: h,
                stride: w * 2,
            })
        }
        "PF_BC7" => {
            // `alpha_basic_settings` (channels=4) preserves the alpha channel;
            // safe for both opaque and translucent source images.
            let settings = bc7::alpha_basic_settings();
            bc7::compress_blocks(
                &settings,
                &RgbaSurface {
                    data: rgba,
                    width: w,
                    height: h,
                    stride: w * 4,
                },
            )
        }
        // Unreachable: format was validated by `encode_mips` before any level.
        _ => return Err(TexError::UnsupportedFormat(format.to_string())),
    };

    let expected = mip_byte_size(format, w, h)
        .ok_or_else(|| TexError::UnsupportedFormat(format.to_string()))?;
    if encoded.len() != expected {
        return Err(TexError::EncodeFailed(format!(
            "{format} encoder produced {} bytes for {w}x{h}, expected {expected}",
            encoded.len()
        )));
    }
    Ok(encoded)
}

/// Tightly pack the R and G bytes out of an RGBA8 buffer (B, A discarded).
/// Output length is `rgba.len() / 2`.
fn pack_rg(rgba: &[u8]) -> Vec<u8> {
    let mut rg = Vec::with_capacity(rgba.len() / 2);
    for px in rgba.chunks_exact(4) {
        rg.push(px[0]);
        rg.push(px[1]);
    }
    rg
}

/// Box-downsample a `w`x`h` RGBA8 image to `(w/2)`x`(h/2)` by averaging each
/// 2x2 texel quad per channel (rounded). Dimensions are powers of two >= 2 by
/// the time this is called, so the halving is exact.
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
                // +2 for round-to-nearest of the /4 average.
                dst[dpx + ch] = ((sum + 2) / 4) as u8;
            }
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `w`x`h` solid-color RGBA8 buffer.
    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            v.extend_from_slice(&rgba);
        }
        v
    }

    /// 8x8 solid-red BC1: 4 mips (8,4,2,1), correct block-math lengths, and
    /// mip0 decodes back to ~red (BC1 565 quantization tolerated).
    #[test]
    fn encode_solid_red_bc1_pyramid() {
        let rgba = solid(8, 8, [255, 0, 0, 255]);
        let mips = encode_mips(&rgba, 8, 8, "PF_DXT1").unwrap();

        // 8 -> 4 -> 2 -> 1 == 4 mips.
        assert_eq!(mips.len(), 4, "expected 4 mips for 8x8");
        // mip0: block_math(8,8,BC1) = 2*2*8 = 32.
        assert_eq!(mips[0].len(), 32, "mip0 BC1 8x8 should be 32 bytes");
        // last mip 1x1: block_math(1,1,BC1) = 1*1*8 = 8.
        assert_eq!(mips[3].len(), 8, "mip3 BC1 1x1 should be 8 bytes");
        // sanity: every level matches its own block math.
        let dims = [(8, 8), (4, 4), (2, 2), (1, 1)];
        for (i, (w, h)) in dims.iter().enumerate() {
            assert_eq!(
                mips[i].len(),
                mip_byte_size("PF_DXT1", *w, *h).unwrap(),
                "mip{i} length mismatch"
            );
        }

        // Decode mip0 back via texture2ddecoder (0xAARRGGBB order) and check red.
        let mut px = [0u32; 64]; // 8x8
        texture2ddecoder::decode_bc1(&mips[0], 8, 8, &mut px).unwrap();
        for (i, &p) in px.iter().enumerate() {
            let r = (p >> 16) & 0xff;
            let g = (p >> 8) & 0xff;
            let b = p & 0xff;
            assert!(r >= 248, "pixel {i}: R={r} not ~255 ({p:#010x})");
            assert!(g <= 8, "pixel {i}: G={g} not ~0 ({p:#010x})");
            assert!(b <= 8, "pixel {i}: B={b} not ~0 ({p:#010x})");
        }
    }

    /// Non-power-of-two / non-multiple-of-4 dims and an unknown format both
    /// error (the right error each).
    #[test]
    fn encode_rejects_non_pot() {
        // 6x6: multiple-of-2 but neither multiple-of-4 nor power-of-two -> Err.
        let err = encode_mips(&vec![0u8; 6 * 6 * 4], 6, 6, "PF_DXT1").unwrap_err();
        assert!(
            matches!(err, TexError::EncodeFailed(_)),
            "6x6 should be EncodeFailed, got {err:?}"
        );

        // Unknown format -> UnsupportedFormat (checked before dims).
        let err = encode_mips(&vec![0u8; 4 * 4 * 4], 4, 4, "PF_BOGUS").unwrap_err();
        assert!(
            matches!(err, TexError::UnsupportedFormat(f) if f == "PF_BOGUS"),
            "PF_BOGUS should be UnsupportedFormat"
        );

        // Wrong buffer length -> EncodeFailed.
        let err = encode_mips(&vec![0u8; 10], 4, 4, "PF_DXT1").unwrap_err();
        assert!(matches!(err, TexError::EncodeFailed(_)));
    }

    /// BC3 (DXT5) solid-color round-trip: encode a solid green RGBA, decode mip0
    /// back, assert green survives (incl. opaque alpha).
    #[test]
    fn encode_bc3_green_roundtrip() {
        let rgba = solid(4, 4, [0, 255, 0, 255]);
        let mips = encode_mips(&rgba, 4, 4, "PF_DXT5").unwrap();
        assert_eq!(mips.len(), 3, "4x4 -> 4,2,1 == 3 mips");
        assert_eq!(mips[0].len(), 16, "BC3 4x4 = 1 block = 16 bytes");

        let mut px = [0u32; 16];
        texture2ddecoder::decode_bc3(&mips[0], 4, 4, &mut px).unwrap();
        for (i, &p) in px.iter().enumerate() {
            let a = (p >> 24) & 0xff;
            let r = (p >> 16) & 0xff;
            let g = (p >> 8) & 0xff;
            let b = p & 0xff;
            assert!(r <= 8, "pixel {i}: R={r} not ~0");
            assert!(g >= 248, "pixel {i}: G={g} not ~255");
            assert!(b <= 8, "pixel {i}: B={b} not ~0");
            assert!(a >= 248, "pixel {i}: A={a} not ~255");
        }
    }

    /// BC7 solid-color round-trip: BC7 is near-lossless, so a solid blue should
    /// decode back essentially exact.
    #[test]
    fn encode_bc7_blue_roundtrip() {
        let rgba = solid(4, 4, [0, 0, 255, 255]);
        let mips = encode_mips(&rgba, 4, 4, "PF_BC7").unwrap();
        assert_eq!(mips[0].len(), 16, "BC7 4x4 = 1 block = 16 bytes");

        let mut px = [0u32; 16];
        texture2ddecoder::decode_bc7(&mips[0], 4, 4, &mut px).unwrap();
        for (i, &p) in px.iter().enumerate() {
            let a = (p >> 24) & 0xff;
            let r = (p >> 16) & 0xff;
            let g = (p >> 8) & 0xff;
            let b = p & 0xff;
            assert!(r <= 4, "pixel {i}: R={r} not ~0");
            assert!(g <= 4, "pixel {i}: G={g} not ~0");
            assert!(b >= 251, "pixel {i}: B={b} not ~255");
            assert!(a >= 251, "pixel {i}: A={a} not ~255");
        }
    }

    /// 2x2 box downsample averages each channel (round-to-nearest).
    #[test]
    fn downsample_averages() {
        // 2x2: pixels (0,0,0,0),(100,100,100,100),(200,..),(255,..) -> avg ~139.
        let src = [
            0, 0, 0, 0, // (0,0)
            100, 100, 100, 100, // (1,0)
            200, 200, 200, 200, // (0,1)
            255, 255, 255, 255, // (1,1)
        ];
        let down = downsample_2x2(&src, 2, 2);
        assert_eq!(down.len(), 4);
        // (0+100+200+255+2)/4 = 557/4 = 139.
        assert_eq!(down, vec![139, 139, 139, 139]);
    }
}
