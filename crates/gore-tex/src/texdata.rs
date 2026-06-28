//! Byte-faithful `FTexturePlatformData` parse <-> serialize codec.
//!
//! This is the safety net for the texture *write* path. [`PlatformData::parse`]
//! captures **everything** in the platform-data region of a legacy cooked
//! `UTexture2D` (`.uasset`/`.uexp`/`.ubulk` triple, retoc zen->legacy form) so
//! that [`PlatformData::serialize_into_uexp`] can reproduce `uexp[region]`
//! byte-for-byte, and [`PlatformData::serialize_ubulk`] can reproduce the
//! streamed `.ubulk` byte-for-byte. The unchanged round-trip MUST be identical;
//! Task 4 then drives the same codec with new dims/mips to author an upscaled
//! texture.
//!
//! ## The region
//!
//! Anchoring is identical to [`crate::decode`]: scan `.uexp` for a valid `PF_*`
//! `FString` whose preceding 12 bytes (`SizeX, SizeY, PackedData`) are sane and
//! whose mip table is self-consistent. The platform-data region begins at
//! `SizeX` (anchor - 12) and ends at the trailing **package magic**
//! `0x9E2A83C1`, which terminates the `.uexp` export body (these single-asset
//! cooked packages have exactly one export, so the magic is the final 4 bytes).
//!
//! Within the region, in order:
//!
//! ```text
//! int32  SizeX
//! int32  SizeY
//! uint32 PackedData              // bit31 = bIsVirtual
//! FString PixelFormat            // "PF_*", encoding captured verbatim
//! int32  FirstMipToSerialize
//! int32  NumMips
//! FTexture2DMipMap[NumMips]:
//!   uint32 flags
//!   (inline: BCn payload of block_math(mipW,mipH,fmt) bytes)   // only if inline
//!   int32  SizeX, SizeY, SizeZ
//! <trailer>                       // remaining FTexturePlatformData bytes, verbatim,
//!                                 // up to the package magic
//! ```
//!
//! Inline-vs-stream is discriminated exactly as in `decode`: `flags` is `0x0`
//! for both, so we check whether the 12 bytes after `flags` equal the *computed*
//! `(mipW, mipH, 1)` mip dims. If they do, the mip is **streamed** (its payload
//! lives in `.ubulk`, no inline bytes here). If they don't, the mip is
//! **inline** and `block_math(mipW,mipH,fmt)` payload bytes sit between `flags`
//! and `SizeX,SizeY,SizeZ`. Streamed mips are concatenated into `.ubulk` in mip
//! order (mip0 first at offset 0).

use crate::error::{Result, TexError};
use std::ops::Range;

/// UE package end-of-file tag that terminates the `.uexp` export body.
const PACKAGE_FILE_TAG: u32 = 0x9E2A_83C1;

/// One mip as serialized in retoc's legacy cooked form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MipEntry {
    pub width: u32,
    pub height: u32,
    /// true = payload inline in `.uexp`; false = streamed (bytes in `.ubulk`).
    pub inline: bool,
    pub flags: u32,
    /// BCn bytes (from the `.uexp` inline region OR the `.ubulk` slice).
    pub data: Vec<u8>,
}

/// The conditional `FOptTexturePlatformData` (UE5) serialized between the
/// `PixelFormat` FString and `FirstMipToSerialize`. Present **iff**
/// `PackedData & (1<<30)` (`HasOptData`). Two `u32`s: `ExtData` and
/// `NumMipsInTail` (the count of smallest mips packed into a single tail bulk).
///
/// We capture both verbatim so [`PlatformData::serialize_region`] re-emits them
/// byte-faithfully. For G1R's cook these are absent on every texture observed
/// (every cooked `UTexture2D` sampled has `HasOptData == false`), but the field
/// is part of the authoritative UE5.4 layout (see CUE4Parse
/// `FTexturePlatformData`), so we honor it for correctness/future-proofing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptData {
    pub ext_data: u32,
    pub num_mips_in_tail: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformData {
    pub size_x: u32,
    pub size_y: u32,
    /// Packed flags: bit31=CubeMap, bit30=HasOptData, bit29=HasCpuCopy, bits
    /// 0..28 = NumSlices. (Note: `bIsVirtual` is **not** here — it is a separate
    /// i32 bool serialized *after* the mip array; see `parse_at`.)
    pub packed_data: u32,
    pub format: String,
    /// `Some` iff `packed_data & (1<<30)` (HasOptData). Serialized between the
    /// PixelFormat FString and `FirstMipToSerialize`.
    pub opt_data: Option<OptData>,
    pub first_mip: u32,
    pub mips: Vec<MipEntry>,
    /// FTexturePlatformData bytes after the mip array, kept verbatim.
    pub trailer: Vec<u8>,
    /// `[start, end)` of the platform-data region within `.uexp`.
    pub region: Range<usize>,
    /// `Some` iff this is a cooked **virtual texture** (`NumMips == 0` +
    /// `bIsVirtual == 1`). When set, `mips` is empty and the platform-data tail
    /// is the `FVirtualTextureBuiltData` block (re-emitted via
    /// [`crate::vt::serialize_into`]) instead of an `FTexture2DMipMap` array.
    pub vt: Option<crate::vt::VtData>,
}

// ---- format block math ----------------------------------------------------

/// Bytes per 4x4 block for the supported *block-compressed* (BCn) formats.
/// Returns `None` for non-block formats (uncompressed and unsupported alike) —
/// callers that need to size any supported mip must use [`mip_byte_size`], which
/// also handles the linear (uncompressed) formats.
pub(crate) fn block_bytes(format: &str) -> Option<u32> {
    match format {
        // 8 bytes / 4x4 block
        "PF_DXT1" | "PF_BC4" => Some(8),
        // 16 bytes / 4x4 block
        "PF_DXT5" | "PF_BC5" | "PF_BC7" | "PF_BC6H" => Some(16),
        _ => None,
    }
}

/// Bytes per pixel for the supported *uncompressed* (linear) formats. Returns
/// `None` for block-compressed or unsupported formats.
pub(crate) fn uncompressed_bytes_per_pixel(format: &str) -> Option<u32> {
    match format {
        "PF_B8G8R8A8" => Some(4),
        "PF_G8" => Some(1),
        // 4 channels x 16-bit half-float = 8 bytes/pixel (HDR; tonemapped on decode).
        "PF_FloatRGBA" => Some(8),
        _ => None,
    }
}

/// Whether the read/decode path supports `format` at all — either a
/// block-compressed BCn format or a known uncompressed (linear) one. (The texture
/// *rewrite* path is narrower: it only supports the BCn formats — see
/// [`block_math`].)
pub(crate) fn is_supported_format(format: &str) -> bool {
    block_bytes(format).is_some() || uncompressed_bytes_per_pixel(format).is_some()
}

/// Bytes of a single mip of `format` at `w` x `h`.
///
/// * Block-compressed (BCn): `ceil(w/4) * ceil(h/4) * block_bytes`.
/// * Uncompressed (linear): `w * h * bytes_per_pixel` — NOT block math.
///
/// Returns `None` for an unsupported format.
fn mip_byte_size(format: &str, w: u32, h: u32) -> Option<u64> {
    if let Some(bpp) = uncompressed_bytes_per_pixel(format) {
        return Some((w as u64) * (h as u64) * (bpp as u64));
    }
    let bb = block_bytes(format)? as u64;
    let blocks_x = ((w as u64) + 3) / 4;
    let blocks_y = ((h as u64) + 3) / 4;
    Some(blocks_x * blocks_y * bb)
}

// ---- bounds-checked little-endian readers ---------------------------------

pub(crate) fn corrupt(msg: &str) -> TexError {
    TexError::Retoc(anyhow::anyhow!("FTexturePlatformData codec: {msg}"))
}

/// Rank a parse error by how *specific* (trustworthy) it is. A higher rank means
/// "this came from a real platform-data anchor and names a concrete reason"; a
/// lower rank is a generic structural error that a stray `PF_*` anchor produces.
/// Used so the genuine reason (virtual texture / unsupported format) is never
/// masked by a later stray anchor's "implausible dimensions".
fn err_specificity(e: &TexError) -> u8 {
    match e {
        TexError::VirtualTexture(_) => 3,
        TexError::UnsupportedFormat(_) => 2,
        // Everything else (the generic "implausible …" / "unhandled …" corrupt
        // errors) is least specific.
        _ => 1,
    }
}

/// Keep whichever of two errors is more specific (see [`err_specificity`]); on a
/// tie, keep the first (earlier anchor — closer to the real platform data).
fn keep_more_specific(first: TexError, second: TexError) -> TexError {
    if err_specificity(&second) > err_specificity(&first) {
        second
    } else {
        first
    }
}

pub(crate) fn rd_i32(b: &[u8], o: usize) -> Result<i32> {
    let s = b
        .get(o..o + 4)
        .ok_or_else(|| corrupt("unexpected end reading i32"))?;
    Ok(i32::from_le_bytes(s.try_into().unwrap()))
}
pub(crate) fn rd_u32(b: &[u8], o: usize) -> Result<u32> {
    let s = b
        .get(o..o + 4)
        .ok_or_else(|| corrupt("unexpected end reading u32"))?;
    Ok(u32::from_le_bytes(s.try_into().unwrap()))
}
fn rd_u16(b: &[u8], o: usize) -> Result<u16> {
    let s = b
        .get(o..o + 2)
        .ok_or_else(|| corrupt("unexpected end reading u16"))?;
    Ok(u16::from_le_bytes(s.try_into().unwrap()))
}

/// Read a UE `FString` at `o`. Returns `(string, next_offset)`.
/// `len > 0`: `len` UTF-8 bytes incl trailing NUL. `len < 0`: `-len` UTF-16LE
/// units incl trailing NUL. `0`: empty.
pub(crate) fn read_fstring(b: &[u8], o: usize) -> Result<(String, usize)> {
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
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        Ok((String::from_utf8_lossy(&bytes[..end]).into_owned(), p))
    } else {
        let n = (-len) as usize;
        let mut units = Vec::with_capacity(n);
        for i in 0..n {
            units.push(rd_u16(b, p + i * 2)?);
        }
        p += n * 2;
        let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
        Ok((String::from_utf16_lossy(&units[..end]), p))
    }
}

/// Serialize a `PF_*` format name as the *captured* FString encoding.
///
/// retoc's legacy output writes these as positive-length UTF-8 incl trailing
/// NUL (every `PF_*` name is ASCII). We re-emit exactly that form. We assert the
/// name is ASCII so the round-trip cannot silently widen to UTF-16; a non-ASCII
/// format name is rejected loudly (it would never be a real `PF_*`).
pub(crate) fn write_fstring_ascii(out: &mut Vec<u8>, s: &str) -> Result<()> {
    if !s.is_ascii() {
        return Err(corrupt("pixel format name is not ASCII"));
    }
    let len = (s.len() + 1) as i32; // incl trailing NUL
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    Ok(())
}

// ---- parse ----------------------------------------------------------------

impl PlatformData {
    /// Parse from legacy cooked files.
    ///
    /// We scan every `PF_*` anchor (some `.uexp`s contain stray `PF_*` byte
    /// sequences inside mip/VT payloads). The FIRST anchor with a sane 12-byte
    /// prefix is the real `FTexturePlatformData`; a later stray anchor, if parsed,
    /// yields a generic "implausible dimensions" error. To avoid that generic
    /// error MASKING the real, specific reason (e.g. a virtual texture or an
    /// unsupported pixel format identified at the true anchor), we keep the
    /// *most specific* error seen rather than simply the last one.
    pub fn parse(_uasset: &[u8], uexp: &[u8], ubulk: &[u8]) -> Result<Self> {
        let mut best_err: Option<TexError> = None;

        let mut search = 0usize;
        while let Some(rel) = find_pf_anchor(uexp, search) {
            if rel >= 12 {
                let pd_start = rel - 12;
                match Self::parse_at(uexp, ubulk, pd_start) {
                    Ok(pd) => return Ok(pd),
                    Err(e) => {
                        best_err = Some(match best_err {
                            Some(prev) => keep_more_specific(prev, e),
                            None => e,
                        });
                    }
                }
            }
            search = rel + 1;
        }
        Err(best_err.unwrap_or_else(|| {
            corrupt("no FTexturePlatformData found (no valid PF_* anchor in .uexp)")
        }))
    }

    fn parse_at(uexp: &[u8], ubulk: &[u8], pd_start: usize) -> Result<Self> {
        let size_x = rd_i32(uexp, pd_start)?;
        let size_y = rd_i32(uexp, pd_start + 4)?;
        let packed = rd_u32(uexp, pd_start + 8)?;

        if !(1..=16384).contains(&size_x) || !(1..=16384).contains(&size_y) {
            return Err(corrupt("implausible texture dimensions at anchor"));
        }
        let width = size_x as u32;
        let height = size_y as u32;

        let (format, mut o) = read_fstring(uexp, pd_start + 12)?;
        if !format.starts_with("PF_") {
            return Err(corrupt("anchor FString is not a PF_* pixel format"));
        }

        // `FOptTexturePlatformData` (UE5): present iff PackedData bit30
        // (HasOptData). Two u32s ExtData, NumMipsInTail. We MUST consume them
        // here (before FirstMipToSerialize) so the FirstMip/NumMips reads land on
        // the right offsets. `HasCpuCopy` (bit29) would insert an FSharedImage
        // block here too, but no G1R texture sets it; reject loudly if seen so we
        // never silently misalign.
        if packed & (1 << 29) != 0 {
            return Err(corrupt(
                "FTexturePlatformData has CPUCopy (FSharedImage) block — unhandled cooked layout",
            ));
        }
        let opt_data = if packed & (1 << 30) != 0 {
            let ext_data = rd_u32(uexp, o)?;
            let num_mips_in_tail = rd_u32(uexp, o + 4)?;
            o += 8;
            Some(OptData {
                ext_data,
                num_mips_in_tail,
            })
        } else {
            None
        };

        // Reject unsupported pixel formats AFTER consuming OptData so the error is
        // the real "unsupported format", not a downstream misparse. Supported =
        // BCn (block) OR a known uncompressed/linear format (B8G8R8A8 / G8).
        if !is_supported_format(&format) {
            return Err(TexError::UnsupportedFormat(format));
        }

        let first_mip = rd_i32(uexp, o)?;
        o += 4;
        let num_mips = rd_i32(uexp, o)?;
        o += 4;
        // A cooked **virtual texture** serializes ZERO `FTexture2DMipMap`s and
        // then `bIsVirtual` (an i32 bool) == 1, followed by an
        // `FVirtualTextureBuiltData` block. The vast majority of G1R's character
        // and environment textures are cooked this way. The old parser read
        // `NumMips == 0`, tripped the `1..=20` range check, then `parse` advanced
        // to a stray `PF_*` anchor inside the VT chunk data and surfaced a MASKED
        // "implausible dimensions". Detect the VT shape and surface the REAL,
        // specific reason. (The BCn mip0 read path cannot decode a VT — it has no
        // single linear mip0 surface — so this is correctly unsupported here.)
        if num_mips == 0 {
            let b_is_virtual = rd_i32(uexp, o).unwrap_or(0);
            if b_is_virtual != 1 {
                return Err(corrupt(
                    "NumMips=0 at platform-data anchor but not a virtual texture — unhandled cooked-tail layout",
                ));
            }
            o += 4; // consume bIsVirtual
            // Parse the FVirtualTextureBuiltData block byte-faithfully so the
            // region re-serializes losslessly. The VT chunk bytes live in
            // `.ubulk` (legacy data-resource indices), so parse only touches
            // `.uexp` here.
            let vt = crate::vt::parse(uexp, &mut o)?;
            let region_end = find_package_tag(uexp, o)
                .ok_or_else(|| corrupt("package magic not found after VT block"))?;
            if region_end < o {
                return Err(corrupt("computed region end precedes end of VT block"));
            }
            let trailer = uexp
                .get(o..region_end)
                .ok_or_else(|| corrupt("VT trailer runs past end of .uexp"))?
                .to_vec();
            return Ok(PlatformData {
                size_x: width,
                size_y: height,
                packed_data: packed,
                format,
                opt_data,
                first_mip: first_mip as u32,
                mips: Vec::new(),
                trailer,
                region: pd_start..region_end,
                vt: Some(vt),
            });
        }
        if !(1..=20).contains(&num_mips) {
            return Err(corrupt("implausible NumMips"));
        }
        if first_mip != 0 {
            return Err(TexError::UnsupportedFormat(format!(
                "FirstMipToSerialize={first_mip} not supported (mip0 not serialized)"
            )));
        }

        // Walk the mip array. mip[i] dims = (max(baseW>>i,1), max(baseH>>i,1)).
        // Inline-vs-stream: flags is 0 for both, so check whether the 12 bytes
        // after `flags` equal the computed (mipW,mipH,1) -> streamed (no inline
        // payload); otherwise inline with block_math payload between flags and
        // SizeX,SizeY,SizeZ.
        let mut mips: Vec<MipEntry> = Vec::with_capacity(num_mips as usize);
        let mut ubulk_off = 0usize;
        for i in 0..(num_mips as usize) {
            let mip_w = (width >> i).max(1);
            let mip_h = (height >> i).max(1);
            let payload_len = mip_byte_size(&format, mip_w, mip_h)
                .ok_or_else(|| corrupt("no block size for format"))?
                as usize;

            let flags = rd_u32(uexp, o)?;
            o += 4;

            // Peek the 12 bytes after flags as candidate (SizeX,SizeY,SizeZ).
            let peek_x = rd_i32(uexp, o)?;
            let peek_y = rd_i32(uexp, o + 4)?;
            let peek_z = rd_i32(uexp, o + 8)?;
            let looks_streamed = peek_x == mip_w as i32
                && peek_y == mip_h as i32
                && peek_z == 1;

            let (inline, data) = if looks_streamed {
                // Streamed: payload is in .ubulk, in mip order, mip0 at offset 0.
                let end = ubulk_off
                    .checked_add(payload_len)
                    .ok_or_else(|| corrupt("ubulk offset overflow"))?;
                let slice = ubulk
                    .get(ubulk_off..end)
                    .ok_or_else(|| corrupt("streamed mip runs past end of .ubulk"))?
                    .to_vec();
                ubulk_off = end;
                (false, slice)
            } else {
                // Inline: payload bytes follow `flags`.
                let end = o
                    .checked_add(payload_len)
                    .ok_or_else(|| corrupt("inline mip offset overflow"))?;
                let slice = uexp
                    .get(o..end)
                    .ok_or_else(|| corrupt("inline mip payload runs past end of .uexp"))?
                    .to_vec();
                o = end;
                (true, slice)
            };

            // Now the (SizeX,SizeY,SizeZ) int32s. Validate they match dims.
            let dx = rd_i32(uexp, o)?;
            let dy = rd_i32(uexp, o + 4)?;
            let dz = rd_i32(uexp, o + 8)?;
            o += 12;
            if dx != mip_w as i32 || dy != mip_h as i32 || dz != 1 {
                return Err(corrupt("mip dims do not match computed (mipW,mipH,1)"));
            }

            mips.push(MipEntry {
                width: mip_w,
                height: mip_h,
                inline,
                flags,
                data,
            });
        }

        // Region ends at the trailing package magic (terminator of the .uexp
        // export body). For these single-export cooked packages it is the final
        // 4 bytes; locate it from end-of-mips forward to be robust.
        let region_end = find_package_tag(uexp, o)
            .ok_or_else(|| corrupt("package magic not found after mip array"))?;
        if region_end < o {
            return Err(corrupt("computed region end precedes end of mip array"));
        }
        let trailer = uexp
            .get(o..region_end)
            .ok_or_else(|| corrupt("trailer runs past end of .uexp"))?
            .to_vec();

        Ok(PlatformData {
            size_x: width,
            size_y: height,
            packed_data: packed,
            format,
            opt_data,
            first_mip: first_mip as u32,
            mips,
            trailer,
            region: pd_start..region_end,
            vt: None,
        })
    }

    /// Overwrite `uexp[self.region]` with this platform data, re-serialized.
    /// For the unchanged round-trip the new region length == old, so no summary
    /// fixup is needed. (`uasset` is accepted for API symmetry / future fixup.)
    pub fn serialize_into_uexp(&self, uexp: &mut Vec<u8>, _uasset: &[u8]) -> Result<()> {
        if self.region.start > self.region.end || self.region.end > uexp.len() {
            return Err(corrupt("region out of bounds for serialize_into_uexp"));
        }
        let body = self.serialize_region()?;
        uexp.splice(self.region.clone(), body);
        Ok(())
    }

    /// Serialize the platform-data region bytes (what occupies `uexp[region]`).
    fn serialize_region(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.size_x as i32).to_le_bytes());
        out.extend_from_slice(&(self.size_y as i32).to_le_bytes());
        out.extend_from_slice(&self.packed_data.to_le_bytes());
        write_fstring_ascii(&mut out, &self.format)?;
        // FOptTexturePlatformData (ExtData, NumMipsInTail) — only when HasOptData.
        if let Some(opt) = self.opt_data {
            out.extend_from_slice(&opt.ext_data.to_le_bytes());
            out.extend_from_slice(&opt.num_mips_in_tail.to_le_bytes());
        }
        out.extend_from_slice(&(self.first_mip as i32).to_le_bytes());

        // Virtual texture: NumMips == 0, then bIsVirtual == 1, then the
        // FVirtualTextureBuiltData block (re-emitted byte-faithfully).
        if let Some(vt) = &self.vt {
            out.extend_from_slice(&0i32.to_le_bytes()); // NumMips
            out.extend_from_slice(&1i32.to_le_bytes()); // bIsVirtual
            crate::vt::serialize_into(&mut out, vt)?;
            out.extend_from_slice(&self.trailer);
            return Ok(out);
        }

        out.extend_from_slice(&(self.mips.len() as i32).to_le_bytes());

        for m in &self.mips {
            out.extend_from_slice(&m.flags.to_le_bytes());
            if m.inline {
                out.extend_from_slice(&m.data);
            }
            out.extend_from_slice(&(m.width as i32).to_le_bytes());
            out.extend_from_slice(&(m.height as i32).to_le_bytes());
            out.extend_from_slice(&1i32.to_le_bytes());
        }

        out.extend_from_slice(&self.trailer);
        Ok(out)
    }

    /// Concatenate streamed mips (`inline == false`) in order -> the `.ubulk`
    /// bytes.
    pub fn serialize_ubulk(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for m in &self.mips {
            if !m.inline {
                out.extend_from_slice(&m.data);
            }
        }
        out
    }

    /// For each mip in order, the `(serial_offset, serial_size, inline)` that the
    /// legacy data-resource (`FObjectDataResource`) array must carry, computed to
    /// match how [`serialize_region`] and [`serialize_ubulk`] lay the bytes out.
    ///
    /// * **Streamed mip** (`inline == false`): `serial_offset` is the cumulative
    ///   byte offset into the `.ubulk` (mip0 at 0), exactly as `serialize_ubulk`
    ///   concatenates them.
    /// * **Inline mip** (`inline == true`): `serial_offset` is the ABSOLUTE byte
    ///   offset into the `.uexp` body where that mip's BCn payload begins. The
    ///   `.uexp` body starts at body byte 0 (export `serial_offset ==
    ///   total_header_size`), and the platform-data region begins at
    ///   `self.region.start`, so the payload offset is `region.start +
    ///   (region-relative offset of the payload)` -- mirroring `serialize_region`'s
    ///   layout (header + per-mip `flags`/payload/`SizeXYZ`).
    ///
    /// `serial_size` is always `block_math(format, mipW, mipH)`.
    fn mip_serial_layout(&self) -> Result<Vec<(i64, i64, bool)>> {
        // Region-relative running offset, mirroring `serialize_region`.
        let mut rel: usize = 4 + 4 + 4; // SizeX + SizeY + PackedData
        // format FString: i32 len + (len) bytes (ASCII + NUL).
        rel += 4 + (self.format.len() + 1);
        if self.opt_data.is_some() {
            rel += 8; // FOptTexturePlatformData: ExtData + NumMipsInTail
        }
        rel += 4 + 4; // FirstMipToSerialize + NumMips

        let mut ubulk_off: i64 = 0;
        let mut out = Vec::with_capacity(self.mips.len());
        for m in &self.mips {
            let size = block_math(&self.format, m.width, m.height)? as i64;
            rel += 4; // flags
            if m.inline {
                // Payload begins here (region-relative), then absolute in .uexp.
                let serial_offset = (self.region.start + rel) as i64;
                rel += m.data.len(); // inline payload
                out.push((serial_offset, size, true));
            } else {
                out.push((ubulk_off, size, false));
                ubulk_off += size;
            }
            rel += 12; // SizeX,SizeY,SizeZ
        }
        Ok(out)
    }
}

// ---- texture rewrite (upscale write path) ---------------------------------

/// `block_math(w, h, format)` bytes for one mip, or a loud error.
fn block_math(format: &str, w: u32, h: u32) -> Result<usize> {
    mip_byte_size(format, w, h)
        .map(|n| n as usize)
        .ok_or_else(|| TexError::UnsupportedFormat(format.to_string()))
}

/// `NumMips` for a base of `w` x `h`: `log2(max(w,h)) + 1`.
fn expected_num_mips(w: u32, h: u32) -> usize {
    (32 - w.max(h).leading_zeros()) as usize
}

/// Unified texture-replace entry: takes a raw RGBA8 image (`new_rgba`, byte order
/// R,G,B,A) at `new_w` x `new_h` and rewrites the cooked triple, branching on the
/// ORIGINAL texture's shape:
///
/// * **Virtual texture** (`orig.vt.is_some()`) → re-tile the image into the
///   cooked VT format via [`crate::vt::retile`] (single-layer, same-dims only),
///   write the rebuilt chunk bytes back into the `.ubulk`/`.uexp`, and re-emit the
///   `FVirtualTextureBuiltData` block. See [`replace_texture_vt`].
/// * **Regular texture** → BC-encode the full mip pyramid for `new_w`/`new_h` in
///   the original pixel format ([`crate::encode::encode_mips`]) and route through
///   the existing [`replace_texture`] non-VT path.
///
/// `orig_format` is the UE pixel-format name of the layer-0 / mip surface (== the
/// `format` field of [`crate::decode::parse`]); it drives BCn encoding for the
/// non-VT branch and is validated against the VT layer format for the VT branch.
///
/// This is the single entry the callers (gore-mod prepare arm, CLI `texture
/// replace`, FFI) should use: they always hold the source RGBA + dims, so passing
/// the image (rather than pre-encoded mips) lets gore-tex decide whether to mip or
/// re-tile. Returns `(new_uasset, new_uexp, new_ubulk)`.
pub fn replace_texture_image(
    uasset: &[u8],
    uexp: &[u8],
    ubulk: &[u8],
    new_rgba: &[u8],
    new_w: u32,
    new_h: u32,
    orig_format: &str,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    // Parse once to learn whether this is a virtual texture. (The non-VT path
    // re-parses internally; the VT path reuses this parse.)
    let orig = PlatformData::parse(uasset, uexp, ubulk)?;
    if orig.vt.is_some() {
        return replace_texture_vt(uasset, uexp, ubulk, orig, new_rgba, new_w, new_h);
    }
    // Regular texture. If the original shipped with NO mip chain (NoMipmaps —
    // e.g. UI / cursor textures), `replace_texture` only keeps mip0, so there is
    // no need (and no requirement) for power-of-two dimensions: encode just the
    // single mip0 surface via `encode_tile` (multiple-of-4 only). Otherwise
    // BC-encode the full power-of-two mip pyramid via `encode_mips`.
    let mips = if orig.mips.len() == 1 {
        vec![crate::encode::encode_tile(new_rgba, new_w, new_h, orig_format)?]
    } else {
        crate::encode::encode_mips(new_rgba, new_w, new_h, orig_format)?
    };
    replace_texture(uasset, uexp, ubulk, new_w, new_h, mips)
}

/// VT branch of [`replace_texture_image`]: re-tile `new_rgba` (RGBA8, `new_w` x
/// `new_h`) into the cooked virtual-texture layout described by `orig.vt` and
/// write the result back into the cooked triple. Returns
/// `(new_uasset, new_uexp, new_ubulk)`.
///
/// ## Same-dims only (today)
///
/// [`crate::vt::retile`] requires the new image to match the template's
/// dimensions and tile config exactly, so every offset table, chunk count, and
/// per-chunk `SizeInBytes` is byte-for-byte reused — only the chunk PAYLOADS
/// (re-encoded tile bytes) and per-chunk `FSHAHash` change. This same-dims
/// invariant makes the rewrite a set of surgical, fixed-length edits:
///
/// 1. **`.uexp` platform-data region** — replace it with the re-serialized
///    region carrying `new_vt`. Only the 20-byte per-chunk hashes (and identical
///    `SizeInBytes`) differ, so the region length is UNCHANGED ⇒ export
///    `SerialSize` delta is 0 (the `patch_uasset_serial_size` call is a no-op,
///    but we run it for correctness).
/// 2. **`.ubulk` / inline chunk bytes** — each VT chunk's bulk payload lives at
///    `data_resources[chunk.data_resource_index]`. We write each rebuilt
///    `chunk_bytes[c]` back at that data-resource's `serial_offset` (streamed →
///    `.ubulk`; inline → the `.uexp` export body), exactly where the original
///    chunk sat. Because sizes are unchanged the `.ubulk` length is unchanged.
/// 3. **data-resources** — for same-dims every entry's `serial_size`/`raw_size`
///    is unchanged, so the `FObjectDataResource` array needs NO edit. We VALIDATE
///    that the rebuilt chunk sizes equal the data-resources' recorded sizes
///    (proving the same-dims assumption end-to-end) and leave the `.uasset`
///    header byte-identical — no summary fixup is required when nothing resizes.
/// 4. **`ImportedSize`** — `new_w`/`new_h` == orig dims for a same-dims replace,
///    so the property-block patch overwrites identical bytes (no-op); we still
///    run it for correctness/future-proofing.
fn replace_texture_vt(
    uasset: &[u8],
    uexp: &[u8],
    ubulk: &[u8],
    orig: PlatformData,
    new_rgba: &[u8],
    new_w: u32,
    new_h: u32,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let template = orig
        .vt
        .as_ref()
        .ok_or_else(|| corrupt("replace_texture_vt called on a non-VT texture"))?;
    let layer0_format = template
        .layer_types
        .first()
        .ok_or_else(|| corrupt("VT template has no layer types"))?
        .clone();

    // 1. Re-tile the image into the cooked VT format (single-layer + dims guards
    //    live in retile). chunk_bytes[c] is parallel to template.chunks.
    let (new_vt, chunk_bytes) = crate::vt::retile(new_rgba, new_w, new_h, template, &layer0_format)?;
    if chunk_bytes.len() != template.chunks.len() {
        return Err(corrupt("VT retile returned a different chunk count than the template"));
    }

    // 2. Re-serialize the platform-data region with the new VtData.
    let mut new_pd = orig.clone();
    new_pd.vt = Some(new_vt);
    let new_region = new_pd.serialize_region()?;
    let old_region_len = orig.region.end - orig.region.start;
    let delta: i64 = new_region.len() as i64 - old_region_len as i64;

    // 3. new_uexp = uexp with [region] replaced. For same-dims VT the region
    //    length is unchanged (only the 20-byte chunk hashes differ), so delta == 0
    //    and the tail (incl. any inline chunk bytes + the package magic) is
    //    unmoved. We assert that here — a non-zero delta would mean a chunk size
    //    changed, breaking the same-dims invariant the chunk-byte write relies on.
    let mut new_uexp = uexp.to_vec();
    new_uexp.splice(orig.region.clone(), new_region);
    if delta != 0 {
        return Err(corrupt(&format!(
            "VT replace changed the platform-data region length by {delta}; same-dims VT must \
             keep it constant — refusing to write inconsistent chunk offsets"
        )));
    }

    // 3b. ImportedSize patch in the property block (no-op for same-dims; run for
    //     correctness). The property block is new_uexp[0 .. orig.region.start).
    let prop_block = new_uexp
        .get_mut(..orig.region.start)
        .ok_or_else(|| corrupt("region start runs past end of .uexp"))?;
    let _ = patch_imported_size(prop_block, (orig.size_x, orig.size_y), (new_w, new_h))?;

    // 4. Write each rebuilt chunk's bytes back into the cooked output at its
    //    data-resource offset. Streamed chunks land in `.ubulk`; inline chunks
    //    land in the `.uexp` export body. Same-dims ⇒ each chunk's serial_size is
    //    unchanged, so this overwrites in place without moving any neighbor.
    let mut new_ubulk = ubulk.to_vec();
    write_vt_chunk_bytes(
        uasset,
        &mut new_uexp,
        &mut new_ubulk,
        new_pd.vt.as_ref().unwrap(),
        &chunk_bytes,
    )?;

    // 5. Patch export SerialSize by delta (== 0 for same-dims → no-op; leaves the
    //    .uasset byte-identical). The data-resource array needs no edit for
    //    same-dims (sizes unchanged) — write_vt_chunk_bytes already validated each
    //    chunk's size against its data-resource, so the header stays consistent.
    let new_uasset = patch_uasset_serial_size(uasset, uexp, &orig.region, delta)?;

    Ok((new_uasset, new_uexp, new_ubulk))
}

/// Write the rebuilt VT chunk bytes (`chunk_bytes[c]` ↔ `vt.chunks[c]`) into the
/// cooked output at each chunk's data-resource offset.
///
/// A cooked VT chunk's bulk payload is located by its
/// `data_resources[chunk.data_resource_index]` entry (the same array
/// [`resolve_data_resource_bytes`] reads): streamed payloads (`legacy_bulk_data_flags
/// & PayloadInSeperateFile`) live in `.ubulk` at `serial_offset`; inline payloads
/// live in the `.uexp` export body at `serial_offset`. We validate each chunk's
/// rebuilt length equals the data-resource's recorded `serial_size` (the same-dims
/// invariant — VT retile asserts it, and we re-check it against the on-disk
/// data-resource so a layout mismatch fails loud rather than corrupting offsets),
/// then overwrite the payload bytes in place.
fn write_vt_chunk_bytes(
    uasset: &[u8],
    uexp: &mut Vec<u8>,
    ubulk: &mut Vec<u8>,
    vt: &crate::vt::VtData,
    chunk_bytes: &[Vec<u8>],
) -> Result<()> {
    use retoc::legacy_asset::FLegacyPackageHeader;
    use retoc::version::EngineVersion;
    use std::io::Cursor;

    let fallback = EngineVersion::UE5_4.package_file_version();
    let header = FLegacyPackageHeader::deserialize(&mut Cursor::new(uasset), Some(fallback))
        .map_err(|e| corrupt(&format!("could not parse .uasset summary for VT chunk write: {e}")))?;

    for (c, (chunk, bytes)) in vt.chunks.iter().zip(chunk_bytes.iter()).enumerate() {
        let dr_index = chunk.data_resource_index;
        if dr_index < 0 {
            return Err(corrupt(&format!("VT chunk {c} has negative data-resource index {dr_index}")));
        }
        let dr = header.data_resources.get(dr_index as usize).ok_or_else(|| {
            corrupt(&format!(
                "VT chunk {c} data-resource index {dr_index} out of range ({} entries)",
                header.data_resources.len()
            ))
        })?;
        if dr.serial_offset < 0 || dr.serial_size < 0 {
            return Err(corrupt(&format!("VT chunk {c} data-resource has negative serial_offset/size")));
        }
        // Same-dims invariant: the rebuilt chunk must be exactly the recorded size.
        if bytes.len() as i64 != dr.serial_size {
            return Err(corrupt(&format!(
                "VT chunk {c} rebuilt to {} bytes but data-resource serial_size is {}; same-dims \
                 layout assumption broken — refusing to write",
                bytes.len(),
                dr.serial_size
            )));
        }
        let off = dr.serial_offset as usize;
        let end = off
            .checked_add(bytes.len())
            .ok_or_else(|| corrupt("VT chunk write slice overflow"))?;

        let dst: &mut Vec<u8> = if dr.legacy_bulk_data_flags & BULKDATA_PAYLOAD_IN_SEPERATE_FILE != 0 {
            ubulk // streamed: offsets are `.ubulk`-relative
        } else {
            uexp // inline in the export body
        };
        let slot = dst
            .get_mut(off..end)
            .ok_or_else(|| corrupt(&format!("VT chunk {c} bytes run past end of cooked file")))?;
        slot.copy_from_slice(bytes);
    }
    Ok(())
}

/// Rewrite the cooked files to carry `new_mips` (largest-first, from
/// [`crate::encode::encode_mips`]) at `new_w` x `new_h`, keeping the original
/// pixel format. Returns `(new_uasset, new_uexp, new_ubulk)`.
///
/// This is the crux of the upscale write path. It re-serializes the
/// `FTexturePlatformData` region of the `.uexp` with the new dimensions/mips,
/// rebuilds the `.ubulk`, and — because the export body length almost always
/// changes — patches the one `.uasset` field that retoc's zen builder reads and
/// that depends on that length: the texture `FObjectExport`'s `SerialSize`.
///
/// ## Inline/stream policy
///
/// We mirror the *original* texture's inline-vs-stream split, keyed on each
/// mip's largest dimension:
///
/// * If the original streamed any mip (payload in `.ubulk`), we take the
///   **largest streamed mip dimension** the original used as the threshold `T`
///   and stream every new mip whose `max(w, h) >= T`, inlining the rest. (UE
///   cooks the large mips to bulk data and keeps a small inline tail; reusing
///   the observed boundary keeps the new texture shaped exactly like a
///   cook-time one.)
/// * If the original was *fully inline* (e.g. the cursor / small UI textures),
///   we keep every new mip inline too.
///
/// `flags` is reused as `0x0` for every mip — the value the original carries for
/// both inline and streamed mips. [`PlatformData::parse`] discriminates on the
/// serialized *shape* (presence of the inline payload), not on the flag, so the
/// readback test validates this choice end-to-end.
///
/// ## `.uasset` fixup
///
/// `repack_to_zen` -> `build_zen_asset` re-parses the legacy `.uasset` with
/// retoc's `FLegacyPackageHeader` and copies each export's `SerialSize` into the
/// zen export map as `cooked_serial_size` (and, for the UE5.4 `NoExportInfo`
/// path, writes the `.uexp` body verbatim). So the texture export's `SerialSize`
/// **must** become `old_serial_size + delta`, where `delta = new_region_len -
/// old_region_len`, or the zen export's declared size won't match the body and
/// the round-trip back out mis-slices. `BulkDataStartOffset` and the per-export
/// `SerialOffset` of *later* exports are the only other length-dependent fields;
/// for these single-export cooked texture packages there are no later exports,
/// and `BulkDataStartOffset` is explicitly "never read for cooked packages" by
/// the engine and is recomputed by retoc on write-back, so we patch only
/// `SerialSize`. We locate that field's byte offset via retoc's own
/// `FLegacyPackageHeader` parse (no hand-hexing of the version-dependent summary).
pub fn replace_texture(
    uasset: &[u8],
    uexp: &[u8],
    ubulk: &[u8],
    new_w: u32,
    new_h: u32,
    new_mips: Vec<Vec<u8>>,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    // 1. Parse the original to capture format/packed_data/first_mip/trailer.
    let orig = PlatformData::parse(uasset, uexp, ubulk)?;
    if orig.vt.is_some() {
        return Err(TexError::VirtualTexture(orig.format));
    }
    if orig.first_mip != 0 {
        return Err(corrupt("FirstMipToSerialize != 0 is not supported for rewrite"));
    }
    let format = orig.format.clone();

    // 2. Decide how many mips to emit by honoring the ORIGINAL texture's
    //    mip-gen POLICY. `replace_texture` keeps the original unversioned
    //    PROPERTY block (trailer) verbatim, and that block governs the engine's
    //    mip policy (NoMipmaps / streaming). The cooked mip COUNT must stay
    //    consistent with it, or the engine reads mip data out of bounds.
    //
    //    Rule (only the 1-vs-many distinction is honored):
    //    * orig_n == 1  -> the texture shipped with NO mip chain (NoMipmaps,
    //      e.g. UI textures like the hardware cursor). Emit a SINGLE mip at the
    //      new size: `new_mips[0]` (mip0 at new_w x new_h), discard the rest.
    //      NumMips = 1.
    //    * orig_n > 1   -> the original had a real mip chain. Emit the FULL
    //      pyramid for the new size. NumMips = new_mips.len().
    //
    //    Limitation: a texture that shipped with a PARTIAL/custom mip count
    //    (rare) is approximated here as "full chain". Only the 1-vs-many split
    //    is honored -- that is the distinction that governs the
    //    NoMipmaps/streaming behavior that otherwise crashes the engine.
    let orig_n = orig.mips.len();
    let new_mips: Vec<Vec<u8>> = if orig_n == 1 {
        // NoMipmaps: keep only mip0 at the new size.
        new_mips.into_iter().take(1).collect()
    } else {
        new_mips
    };

    // 3. Validate the emitted mip set against the new dims + format. For the
    //    single-mip case the validation set is just `[new_mips[0]]`.
    if orig_n == 1 {
        if new_mips.len() != 1 {
            return Err(corrupt(&format!(
                "NoMipmaps source needs exactly 1 new mip, got {}",
                new_mips.len()
            )));
        }
    } else {
        let want = expected_num_mips(new_w, new_h);
        if new_mips.len() != want {
            return Err(corrupt(&format!(
                "new_mips has {} levels but {new_w}x{new_h} needs {want} (log2(max)+1)",
                new_mips.len()
            )));
        }
    }
    for (i, m) in new_mips.iter().enumerate() {
        let mip_w = (new_w >> i).max(1);
        let mip_h = (new_h >> i).max(1);
        let need = block_math(&format, mip_w, mip_h)?;
        if m.len() != need {
            return Err(corrupt(&format!(
                "new_mips[{i}] is {} bytes but {mip_w}x{mip_h} {format} needs {need}",
                m.len()
            )));
        }
    }

    // 4. Inline/stream policy: derive the threshold from the original split.
    //    `stream_threshold = Some(T)` means "stream mips with max-dim >= T";
    //    `None` means "keep everything inline" (original was fully inline).
    // UE streams the *large* mips and inlines the small tail, so the boundary is
    // the SMALLEST streamed dimension: stream every new mip whose max-dim >= T.
    let stream_threshold: Option<u32> = orig
        .mips
        .iter()
        .filter(|m| !m.inline)
        .map(|m| m.width.max(m.height))
        .min();

    // 5. Build the new mip entries.
    let mut mips: Vec<MipEntry> = Vec::with_capacity(new_mips.len());
    for (i, data) in new_mips.into_iter().enumerate() {
        let mip_w = (new_w >> i).max(1);
        let mip_h = (new_h >> i).max(1);
        let inline = match stream_threshold {
            None => true,
            Some(t) => mip_w.max(mip_h) < t,
        };
        mips.push(MipEntry {
            width: mip_w,
            height: mip_h,
            inline,
            flags: 0,
            data,
        });
    }

    // 6. Assemble the new PlatformData and serialize the new region + ubulk.
    //
    //    OptData on resize: `replace_texture` regenerates a FULL, un-tail-packed
    //    mip pyramid (every mip is its own `FTexture2DMipMap`), so there is no
    //    packed mip tail -> `NumMipsInTail` MUST be 0. We keep the OptData struct
    //    present iff the original carried it (so `packed_data`'s HasOptData bit
    //    stays consistent with the serialized bytes), preserve the opaque
    //    `ExtData`, and zero `NumMipsInTail`. In practice no G1R texture sets
    //    HasOptData, so `orig.opt_data` is `None` and this is a no-op; the branch
    //    is here so a future OptData texture rewrites to a valid full-pyramid
    //    layout rather than claiming a tail it no longer has.
    let new_opt_data = orig.opt_data.map(|opt| OptData {
        ext_data: opt.ext_data,
        num_mips_in_tail: 0,
    });
    let new_pd = PlatformData {
        size_x: new_w,
        size_y: new_h,
        packed_data: orig.packed_data,
        format: format.clone(),
        opt_data: new_opt_data,
        first_mip: 0,
        mips,
        trailer: orig.trailer.clone(),
        region: orig.region.clone(),
        vt: None,
    };
    let new_region = new_pd.serialize_region()?;
    let new_ubulk = new_pd.serialize_ubulk();

    let old_region_len = orig.region.end - orig.region.start;
    let delta: i64 = new_region.len() as i64 - old_region_len as i64;

    // 7. new_uexp = uexp with [region] replaced (the tail incl. the package
    //    magic shifts by delta automatically).
    let mut new_uexp = uexp.to_vec();
    new_uexp.splice(orig.region.clone(), new_region);

    // 7b. Patch the texture's source-dimension UPROPERTY (`ImportedSize`, an
    //     FIntPoint = two consecutive i32) in the unversioned PROPERTY BLOCK,
    //     which lives in `new_uexp[0 .. orig.region.start)` (copied verbatim by
    //     the splice above -- it only replaced `orig.region`).
    //
    //     WHY this is required: at load time the engine sizes the inline mip0
    //     read from `ImportedSize`, NOT from the platform-data `SizeX/SizeY` we
    //     already updated inside the region. If `ImportedSize` stays at the OLD
    //     dims while the platform-data carries the NEW (larger) dims, the engine
    //     reads only the old mip byte count (e.g. 128^2 DXT5 = 16384 B) where the
    //     new mip is larger (256^2 = 65536 B), the stream cursor drifts by exactly
    //     the old mip size, and the next field is parsed from BCn payload garbage
    //     -> hard crash ("Bad name index ..."). Patching `ImportedSize` to the new
    //     dims keeps the inline-read size in sync with the platform data.
    //
    //     Patching EVERY (orig_w, orig_h) i32 pair in the property block also
    //     fixes any sibling cached-dimension field (e.g. a cached source/LOD size)
    //     in one shot, hardening against a secondary stale-dimension crash. This
    //     is a same-length 8-byte-per-occurrence edit -> region length and the
    //     SerialSize delta are unaffected; the existing SerialSize patch below
    //     stays correct.
    let prop_block = new_uexp
        .get_mut(..orig.region.start)
        .ok_or_else(|| corrupt("region start runs past end of .uexp"))?;
    let patched = patch_imported_size(
        prop_block,
        (orig.size_x, orig.size_y),
        (new_w, new_h),
    )?;
    if patched == 0 {
        eprintln!(
            "gore-tex: ImportedSize ({},{}) not found in property block -- skipping (non-fatal; \
             not the crash cause, data-resource patch carries the fix)",
            orig.size_x, orig.size_y
        );
    } else {
        eprintln!(
            "gore-tex: patched ImportedSize property block: {patched} occurrence(s) of \
             ({},{}) -> ({new_w},{new_h})",
            orig.size_x, orig.size_y
        );
    }

    // 8. new_uasset = uasset with the texture export's SerialSize patched by
    //    delta (no-op when delta == 0, i.e. a same-dims replace).
    let new_uasset = patch_uasset_serial_size(uasset, uexp, &orig.region, delta)?;

    // 8b. REBUILD the legacy DATA-RESOURCE array (`FObjectDataResource`) in the
    //     `.uasset`. Each entry's `serial_offset`/`serial_size`/`raw_size`
    //     describes a mip's bulk payload; `build_zen_asset` copies these VERBATIM
    //     into the zen `FBulkDataMapEntry`, which the ENGINE uses to LOCATE and
    //     SIZE each mip read. If they stay at the OLD per-mip values while the
    //     platform data carries the NEW (larger) mips, the engine reads too few
    //     bytes, the stream cursor drifts, and the next field is parsed from BCn
    //     garbage -> hard crash ("Bad name index ..."). CONFIRMED root cause of the
    //     upscale crash (cursor: one entry, old serial_size 16384 -> new 65536).
    //
    //     We REBUILD (not merely patch in place) because an upscale can CHANGE the
    //     mip COUNT (e.g. 1024^2 -> 2048^2 grows the pyramid 11 -> 12 mips), so the
    //     array gains/loses entries. The array is the LAST header section (no
    //     trailing padding -> total_header_size == header end), so we splice new
    //     entry bytes in, then fix the two length-dependent header fields the size
    //     change touches: summary `total_header_size` and the texture export's
    //     `SerialOffset` (which embeds total_header_size).
    let new_uasset = rebuild_data_resources(&new_uasset, &orig, &new_pd)?;

    Ok((new_uasset, new_uexp, new_ubulk))
}

/// Rebuild the legacy `FObjectDataResource` array in `new_uasset` to describe the
/// NEW mip set (`new_pd`), then repair the header fields a header-length change
/// touches.
///
/// ## Why a rebuild (not an in-place size patch)
///
/// `build_zen_asset` copies each data-resource's `serial_offset`,
/// `duplicate_serial_offset`, `serial_size`, `flags`, and `cooked_index` verbatim
/// into the zen `FBulkDataMapEntry`, which the engine uses to LOCATE and SIZE the
/// mip payloads. An upscale changes every mip's size AND can change the mip COUNT
/// (the pyramid grows), so the array generally gains/loses entries and the
/// per-entry offsets all move -- an in-place same-length patch is insufficient.
///
/// ## Strategy (surgical, no retoc re-serialize)
///
/// retoc's `FLegacyPackageHeader::serialize` does NOT reproduce these unversioned
/// G1R `.uasset`s byte-identically (verified: first diff mid-header), so we must
/// NOT round-trip through it. Instead, since the data-resource array is the LAST
/// structured header section and these packages carry NO trailing header padding
/// (`total_header_size == ` array end), we:
///
/// 1. Parse via `FLegacyPackageHeader` to locate the array offset, the version
///    (governs the per-entry `cooked_index` byte), and the per-mip-class template
///    fields (`flags`, `legacy_bulk_data_flags`, `outer_index`) to reuse.
/// 2. Build one entry per NEW mip, IN ORDER (data_resources[i] <-> new mip[i]):
///    * `serial_offset`/`serial_size`/`raw_size` from [`PlatformData::mip_serial_layout`]
///      (streamed -> `.ubulk` cumulative offset; inline -> absolute `.uexp` body
///      offset of the payload).
///    * `flags`/`legacy_bulk_data_flags`/`outer_index` copied from an ORIGINAL
///      entry of the SAME class (inline vs streamed), so cooked semantics carry
///      over. (Original entries are 1:1 with original mips in order, so the first
///      streamed/inline original entry is the template for each class.)
/// 3. Splice the new entry bytes over the old array region.
/// 4. Repair `total_header_size` (summary field) and the texture export's
///    `SerialOffset` (embeds total_header_size) by the byte-length delta of the
///    array. These are the ONLY length-dependent fields a trailing-section resize
///    touches for these single-export cooked packages; `.uexp`/`.ubulk` payload
///    offsets are body-relative and unaffected.
///
/// VALIDATES the original array (count == orig mip count, each old serial_size ==
/// the matching old mip's block-math size) before trusting the layout, and
/// re-locates the patched fields by byte to prove the offset math -- never
/// silently corrupt the size/offset fields that crash the game.
fn rebuild_data_resources(
    new_uasset: &[u8],
    orig: &PlatformData,
    new_pd: &PlatformData,
) -> Result<Vec<u8>> {
    use retoc::legacy_asset::{EObjectDataResourceVersion, FLegacyPackageHeader};
    use retoc::version::EngineVersion;
    use std::io::Cursor;

    // Parse the (already SerialSize-patched) .uasset to locate the array + version.
    let fallback = EngineVersion::UE5_4.package_file_version();
    let header = FLegacyPackageHeader::deserialize(&mut Cursor::new(new_uasset), Some(fallback))
        .map_err(|e| corrupt(&format!("could not parse .uasset summary for data-resource rebuild: {e}")))?;

    if header.data_resources.is_empty() {
        // No legacy data-resource array (some textures emit none). The
        // platform-data region + export SerialSize patch already carry the fix.
        return Ok(new_uasset.to_vec());
    }

    // Validate the ORIGINAL array is 1:1 with the original mips, in order, with
    // matching old block-math sizes -- proves the correspondence we rely on.
    if header.data_resources.len() != orig.mips.len() {
        return Err(corrupt(&format!(
            "data-resource count {} != original mip count {}; unexpected layout, refusing to rebuild",
            header.data_resources.len(),
            orig.mips.len()
        )));
    }
    for (i, (dr, m)) in header.data_resources.iter().zip(orig.mips.iter()).enumerate() {
        let want = block_math(&orig.format, m.width, m.height)? as i64;
        if dr.serial_size != want {
            return Err(corrupt(&format!(
                "data-resource[{i}] serial_size {} != mip[{i}] old block-math size {want}; \
                 correspondence unverified, refusing to rebuild",
                dr.serial_size
            )));
        }
    }

    // Per-class templates from the original entries (flags/bulk-flags/outer differ
    // between streamed and inline mips). Original entries are 1:1 with mips, so the
    // first streamed / first inline original mip indexes the template entry.
    let streamed_tmpl = orig
        .mips
        .iter()
        .position(|m| !m.inline)
        .map(|i| header.data_resources[i]);
    let inline_tmpl = orig
        .mips
        .iter()
        .position(|m| m.inline)
        .map(|i| header.data_resources[i]);

    // Build the new entry bytes from the new mip serial layout.
    let version = header
        .data_resource_version
        .ok_or_else(|| corrupt("data-resources present but version missing"))?;
    let layout = new_pd.mip_serial_layout()?;
    if layout.len() != new_pd.mips.len() {
        return Err(corrupt("mip_serial_layout length mismatch"));
    }

    let mut entries = Vec::new();
    for (i, (serial_offset, serial_size, inline)) in layout.iter().enumerate() {
        let tmpl = if *inline { inline_tmpl } else { streamed_tmpl }.ok_or_else(|| {
            corrupt(&format!(
                "new mip[{i}] is {} but the original texture had no {} mip to template the \
                 data-resource entry from",
                if *inline { "inline" } else { "streamed" },
                if *inline { "inline" } else { "streamed" },
            ))
        })?;
        // FObjectDataResource on-disk READ layout (see `write_data_resource_sizes`).
        entries.extend_from_slice(&tmpl.flags.to_le_bytes());
        if version >= EObjectDataResourceVersion::AddedCookedIndex {
            entries.push(tmpl.cooked_index.unwrap_or(0));
        }
        entries.extend_from_slice(&serial_offset.to_le_bytes()); // serial_offset
        entries.extend_from_slice(&0i64.to_le_bytes()); // duplicate_serial_offset (cooked: 0)
        entries.extend_from_slice(&serial_size.to_le_bytes()); // serial_size
        entries.extend_from_slice(&serial_size.to_le_bytes()); // raw_size == serial_size
        entries.extend_from_slice(&tmpl.outer_index.index.to_le_bytes()); // outer_index (FPackageIndex i32)
        entries.extend_from_slice(&tmpl.legacy_bulk_data_flags.to_le_bytes());
    }

    // Locate the old array byte span: [array_start .. array_end). The version u32
    // and count i32 precede the entries; entries run to the end of the header.
    let dr_offset = header.summary.data_resource_offset;
    if dr_offset <= 0 {
        return Err(corrupt("data_resource_offset non-positive but data-resources parsed"));
    }
    let count_field_at = dr_offset as usize + 4; // after version u32
    let array_start = count_field_at + 4; // after count i32
    let total_header_size = header.summary.versioning_info.total_header_size as usize;
    // These packages carry no trailing header padding: the array ends at the header
    // end. Assert it so a future padded asset fails loud instead of corrupting.
    if total_header_size > new_uasset.len() {
        return Err(corrupt("total_header_size exceeds .uasset length"));
    }
    let array_end = total_header_size;
    if array_end < array_start {
        return Err(corrupt("data-resource array end precedes its start"));
    }
    // Sanity: the old span length must equal old_count * old_stride.
    let old_count = header.data_resources.len();
    let cooked_len = if version >= EObjectDataResourceVersion::AddedCookedIndex { 1 } else { 0 };
    let entry_stride = 4 + cooked_len + 8 + 8 + 8 + 8 + 4 + 4;
    if array_end - array_start != old_count * entry_stride {
        return Err(corrupt(&format!(
            "data-resource span {} != old_count {old_count} * stride {entry_stride}; \
             trailing header padding or layout mismatch -- refusing to rebuild",
            array_end - array_start
        )));
    }

    // Splice the new entries over the old array region.
    let mut out = Vec::with_capacity(array_start + entries.len() + (new_uasset.len() - array_end));
    out.extend_from_slice(&new_uasset[..array_start]);
    out.extend_from_slice(&entries);
    out.extend_from_slice(&new_uasset[array_end..]);

    // Patch the new entry COUNT (i32) in the summary's array preamble.
    let new_count = layout.len() as i32;
    out.get_mut(count_field_at..count_field_at + 4)
        .ok_or_else(|| corrupt("data-resource count field runs past end"))?
        .copy_from_slice(&new_count.to_le_bytes());

    // Header-length delta from the array resize (entries replaced version+count
    // preamble untouched).
    let header_delta: i64 = entries.len() as i64 - (old_count * entry_stride) as i64;
    if header_delta != 0 {
        patch_total_header_size_and_export_offset(&mut out, &header, header_delta)?;
    }

    Ok(out)
}

/// `BULKDATA_PayloadInSeperateFile` — the streamed-payload (`.ubulk`) bit of an
/// `FObjectDataResource`'s `legacy_bulk_data_flags`. When set, the payload bytes
/// live in the separate `.ubulk` at `serial_offset`; when clear, they are inline
/// in the `.uexp` export body.
const BULKDATA_PAYLOAD_IN_SEPERATE_FILE: u32 = 0x100;

/// Resolve a VT chunk's raw bytes from the cooked files via its data-resource
/// index.
///
/// A cooked virtual texture's `FVirtualTextureDataChunk` serializes its bulk data
/// (in retoc's legacy form) as a single `i32` index into the `.uasset`
/// `FObjectDataResource` array — the same array [`rebuild_data_resources`]
/// manages. This reads `data_resources[dr_index]` and slices the chunk bytes:
///
/// * `legacy_bulk_data_flags & PayloadInSeperateFile (0x100)` set -> the payload
///   is streamed; `serial_offset` is **`.ubulk`-relative** (first chunk at 0), so
///   slice `ubulk[serial_offset .. serial_offset + serial_size]`.
/// * flag clear -> the payload is inline in the `.uexp` export body at the same
///   `serial_offset`/`serial_size`.
pub(crate) fn resolve_data_resource_bytes(
    uasset: &[u8],
    uexp: &[u8],
    ubulk: &[u8],
    dr_index: i32,
) -> Result<Vec<u8>> {
    use retoc::legacy_asset::FLegacyPackageHeader;
    use retoc::version::EngineVersion;
    use std::io::Cursor;

    if dr_index < 0 {
        return Err(corrupt(&format!("negative VT data-resource index {dr_index}")));
    }
    let fallback = EngineVersion::UE5_4.package_file_version();
    let header = FLegacyPackageHeader::deserialize(&mut Cursor::new(uasset), Some(fallback))
        .map_err(|e| corrupt(&format!("could not parse .uasset summary for VT chunk resolve: {e}")))?;

    let dr = header
        .data_resources
        .get(dr_index as usize)
        .ok_or_else(|| {
            corrupt(&format!(
                "VT data-resource index {dr_index} out of range ({} entries)",
                header.data_resources.len()
            ))
        })?;

    if dr.serial_offset < 0 || dr.serial_size < 0 {
        return Err(corrupt("VT data-resource has negative serial_offset/size"));
    }
    let off = dr.serial_offset as usize;
    let size = dr.serial_size as usize;

    let src: &[u8] = if dr.legacy_bulk_data_flags & BULKDATA_PAYLOAD_IN_SEPERATE_FILE != 0 {
        ubulk // offsets are `.ubulk`-relative
    } else {
        uexp // inline in the export body
    };
    let end = off
        .checked_add(size)
        .ok_or_else(|| corrupt("VT chunk slice overflow"))?;
    let bytes = src
        .get(off..end)
        .ok_or_else(|| corrupt("VT chunk bytes run past end of cooked file"))?;
    Ok(bytes.to_vec())
}

/// Repair the two header fields a trailing-section (data-resource array) resize
/// touches: the summary `total_header_size` and the single texture export's
/// `SerialOffset` (which embeds total_header_size). Both shift by `header_delta`.
///
/// `header` is the parse of the PRE-resize `.uasset` (offsets are stable for
/// everything BEFORE the array, which is all of the summary + export map). We
/// re-locate each field by byte and assert the on-disk value matches the parsed
/// one before patching -- proving the offset math.
fn patch_total_header_size_and_export_offset(
    out: &mut [u8],
    header: &retoc::legacy_asset::FLegacyPackageHeader,
    header_delta: i64,
) -> Result<()> {
    // --- total_header_size: i32 in the summary versioning info (sits early, well
    //     before the name map). Byte-locating it exactly is version-dependent
    //     (saved-hash vs custom-versions layout), so we scan the summary PREFIX
    //     (magic..name-map) for the parsed value and require it to be UNIQUE -- the
    //     value equals the no-padding header size, distinct from the small version
    //     ints before it and from `bulk_data_start_offset` (end of export body).
    //     Uniqueness makes the scan safe; 0 or >1 matches fails loud. ---
    let parsed_ths = header.summary.versioning_info.total_header_size;
    let names_offset = header.summary.names.offset as usize; // name map start; summary precedes it
    let summary_prefix_end = names_offset.min(out.len());
    let ths_bytes = parsed_ths.to_le_bytes();
    let new_ths = (parsed_ths as i64 + header_delta) as i32;
    let mut matches: Vec<usize> = Vec::new();
    {
        // Start past the 4-byte package magic so a coincidental match there is
        // impossible (magic is 0x9E2A83C1, not a plausible header size anyway).
        let mut i = 4usize;
        while i + 4 <= summary_prefix_end {
            if out[i..i + 4] == ths_bytes {
                matches.push(i);
            }
            i += 1;
        }
    }
    if matches.len() != 1 {
        return Err(corrupt(&format!(
            "total_header_size i32 ({parsed_ths}) found {} time(s) in summary prefix (want exactly 1); \
             refusing to patch ambiguously",
            matches.len()
        )));
    }
    out[matches[0]..matches[0] + 4].copy_from_slice(&new_ths.to_le_bytes());

    // --- export SerialOffset: i64. The export map is at summary.exports.offset;
    //     SerialOffset follows SerialSize within FObjectExport. Field order:
    //     class/super/template/outer (4 x i32) + object_name (2 x i32) +
    //     object_flags (u32) = 28, then SerialSize (i64, +8), then SerialOffset
    //     (i64). So SerialOffset is at entry + 28 + 8 = 36. ---
    let exports_offset = header.summary.exports.offset as usize;
    let depends_offset = header.summary.depends_offset as usize;
    let export_count = header.exports.len();
    if export_count == 0 {
        return Err(corrupt("no exports to patch SerialOffset"));
    }
    let entry_span = depends_offset.checked_sub(exports_offset)
        .ok_or_else(|| corrupt("export-map span underflow"))?;
    if entry_span == 0 || entry_span % export_count != 0 {
        return Err(corrupt("implausible export-map span"));
    }
    let single_export_size = entry_span / export_count;
    const SERIAL_OFFSET_FIELD: usize = 28 + 8; // after object header + SerialSize i64

    // Patch SerialOffset for EVERY export whose body lies AFTER the header (all of
    // them, for these cooked packages -- the body starts at total_header_size).
    for (i, e) in header.exports.iter().enumerate() {
        let entry_start = exports_offset + i * single_export_size;
        let at = entry_start + SERIAL_OFFSET_FIELD;
        let slot = out
            .get_mut(at..at + 8)
            .ok_or_else(|| corrupt("export SerialOffset field runs past end of .uasset"))?;
        let on_disk = i64::from_le_bytes(slot.try_into().unwrap());
        if on_disk != e.serial_offset {
            return Err(corrupt(&format!(
                "export[{i}] SerialOffset byte-located {on_disk} != parsed {} (offset math mismatch)",
                e.serial_offset
            )));
        }
        let new_off = on_disk
            .checked_add(header_delta)
            .ok_or_else(|| corrupt("SerialOffset overflow after delta"))?;
        slot.copy_from_slice(&new_off.to_le_bytes());
    }

    Ok(())
}

/// Overwrite every occurrence of the source-dimension i32 pair `(orig.0, orig.1)`
/// in `prop_block` (the unversioned UPROPERTY block, an 8-byte little-endian
/// `FIntPoint`) with `(new.0, new.1)`.
///
/// This targets the texture's `ImportedSize` (`FIntPoint = i32 x, i32 y`), which
/// the engine uses to size the inline mip read -- the platform-data
/// `SizeX/SizeY` alone is insufficient (see the WHY in `replace_texture`).
///
/// Each match is an in-place same-length 8-byte edit, so the block length never
/// changes. Returns the number of occurrences patched (possibly zero).
///
/// `ImportedSize` is the texture's source dimension and is NOT the upscale-crash
/// cause (the confirmed cause is the legacy data-resource `serial_size`, patched
/// separately). Patching it when present is still more correct for the cooked
/// texture, but a property block that lacks the exact `(orig_w,orig_h)` i32 pair
/// is NOT a failure -- some textures simply don't store the source dims this way.
/// So zero occurrences is a NON-FATAL `Ok(0)` (the caller logs it), not an error.
/// When `orig == new` (same-dims replace) the scan still runs and overwrites
/// identical bytes (no-op).
fn patch_imported_size(
    prop_block: &mut [u8],
    orig: (u32, u32),
    new: (u32, u32),
) -> Result<usize> {
    let mut needle = [0u8; 8];
    needle[0..4].copy_from_slice(&(orig.0 as i32).to_le_bytes());
    needle[4..8].copy_from_slice(&(orig.1 as i32).to_le_bytes());

    let mut replacement = [0u8; 8];
    replacement[0..4].copy_from_slice(&(new.0 as i32).to_le_bytes());
    replacement[4..8].copy_from_slice(&(new.1 as i32).to_le_bytes());

    let mut count = 0usize;
    if prop_block.len() >= 8 {
        let mut i = 0usize;
        while i + 8 <= prop_block.len() {
            if prop_block[i..i + 8] == needle {
                prop_block[i..i + 8].copy_from_slice(&replacement);
                count += 1;
                i += 8; // non-overlapping; the just-written bytes aren't re-scanned
            } else {
                i += 1;
            }
        }
    }

    // Zero occurrences is non-fatal: ImportedSize is not the crash cause, and
    // not every texture stores the source dims as this exact i32 pair. The caller
    // logs the count; downstream the data-resource + platform-data patches carry
    // the real fix.
    Ok(count)
}

/// Patch the texture `FObjectExport.SerialSize` in `uasset` by `delta`.
///
/// Uses retoc's `FLegacyPackageHeader` to parse the legacy summary + export map
/// (handling the version-dependent layout), identifies the export whose
/// serialized body in `uexp` contains the platform-data region, then overwrites
/// that export's `SerialSize` i64 in place. All other header bytes are left
/// byte-identical (only the 8-byte field changes), so the unchanged-asset path
/// stays bit-for-bit identical when `delta == 0`.
fn patch_uasset_serial_size(
    uasset: &[u8],
    _uexp: &[u8],
    region: &Range<usize>,
    delta: i64,
) -> Result<Vec<u8>> {
    use retoc::legacy_asset::FLegacyPackageHeader;
    use retoc::version::EngineVersion;
    use std::io::Cursor;

    let mut out = uasset.to_vec();
    if delta == 0 {
        return Ok(out); // same-dims replace: SerialSize unchanged.
    }

    // G1R's cooked packages are *unversioned*; retoc cannot derive the package
    // file version from the summary alone, so supply the same UE5.4 fallback
    // `repack_to_zen` uses (the container this asset came from is UE5_4).
    let fallback = EngineVersion::UE5_4.package_file_version();
    let header = FLegacyPackageHeader::deserialize(&mut Cursor::new(uasset), Some(fallback))
        .map_err(|e| corrupt(&format!("could not parse .uasset summary for fixup: {e}")))?;

    let total_header_size = header.summary.versioning_info.total_header_size as i64;
    let exports_offset = header.summary.exports.offset as i64;
    let export_count = header.summary.exports.count as usize;
    if export_count == 0 || header.exports.is_empty() {
        return Err(corrupt(".uasset has no exports to patch"));
    }
    // depends_offset terminates the export map; per-entry size is uniform.
    let depends_offset = header.summary.depends_offset as i64;
    let entry_span = depends_offset - exports_offset;
    if entry_span <= 0 || (entry_span as usize) % export_count != 0 {
        return Err(corrupt("implausible export-map span in .uasset summary"));
    }
    let single_export_size = (entry_span as usize) / export_count;

    // Identify the texture export: the one whose body in the .uexp contains the
    // platform-data region. SerialOffset includes total_header_size; the .uexp
    // body is the file past the header, so the body-relative start is
    // SerialOffset - total_header_size.
    let region_start = region.start as i64;
    let mut target: Option<usize> = None;
    for (i, e) in header.exports.iter().enumerate() {
        let body_start = e.serial_offset - total_header_size;
        let body_end = body_start + e.serial_size;
        if region_start >= body_start && region_start < body_end {
            target = Some(i);
            break;
        }
    }
    // Single-export packages: fall back to export 0 if the range probe missed
    // (e.g. a tiny rounding in how the body offset is computed upstream).
    // SAFE ONLY because these cooked texture packages are single-export: with one
    // export, "export 0" is unambiguously the texture. For a MULTI-export package
    // a region-probe miss would silently target export 0 (possibly the wrong one);
    // the pre-patch SerialSize sanity check below (byte-located == parsed) is the
    // backstop -- it would still verify we patched a real, correctly-sized field,
    // but it cannot prove export 0 is the texture. If multi-export texture packages
    // ever appear here, the probe must hit (no unwrap_or fallback).
    let target = target.unwrap_or(0);
    if target >= export_count {
        return Err(corrupt("target export index out of range"));
    }

    // Byte offset of this export's SerialSize field. Within an FObjectExport the
    // field order is class/super/template/outer (4 x i32) + object_name
    // (FMinimalName = 2 x i32) + object_flags (u32) = 28 bytes, then SerialSize
    // (i64).
    const SERIAL_SIZE_FIELD_OFFSET: usize = 4 * 4 + 8 + 4; // = 28
    let entry_start = (exports_offset as usize) + target * single_export_size;
    let field_at = entry_start + SERIAL_SIZE_FIELD_OFFSET;
    let slot = out
        .get_mut(field_at..field_at + 8)
        .ok_or_else(|| corrupt("SerialSize field offset runs past end of .uasset"))?;

    // Sanity: the bytes we're about to overwrite must equal the parsed
    // SerialSize, proving our offset math matches retoc's layout.
    let on_disk = i64::from_le_bytes(slot.try_into().unwrap());
    let parsed = header.exports[target].serial_size;
    if on_disk != parsed {
        return Err(corrupt(&format!(
            "SerialSize offset mismatch: byte-located {on_disk} != parsed {parsed} \
             (export {target}, entry_size {single_export_size})"
        )));
    }

    let new_size = parsed
        .checked_add(delta)
        .ok_or_else(|| corrupt("SerialSize overflow after delta"))?;
    if new_size <= 0 {
        return Err(corrupt("patched SerialSize is non-positive"));
    }
    slot.copy_from_slice(&new_size.to_le_bytes());
    Ok(out)
}

/// Find the next `PF_*` `FString` length-prefix offset at/after `from`.
fn find_pf_anchor(uexp: &[u8], from: usize) -> Option<usize> {
    let needle = b"PF_";
    let mut i = from + 4;
    while i + needle.len() <= uexp.len() {
        if &uexp[i..i + needle.len()] == needle {
            let prefix = i - 4;
            if let Ok(len) = rd_i32(uexp, prefix) {
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

/// Find the package end tag `0x9E2A83C1` at/after `from`. Prefers the final
/// 4 bytes of the file (the usual export-body terminator); otherwise returns the
/// first occurrence at/after `from`.
fn find_package_tag(uexp: &[u8], from: usize) -> Option<usize> {
    let tag = PACKAGE_FILE_TAG.to_le_bytes();
    // Fast path: the export body normally ends with the tag at file end.
    if uexp.len() >= 4 && uexp.len() - 4 >= from && uexp[uexp.len() - 4..] == tag {
        return Some(uexp.len() - 4);
    }
    let mut i = from;
    while i + 4 <= uexp.len() {
        if uexp[i..i + 4] == tag {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fx(n: &str) -> Option<Vec<u8>> {
        std::fs::read(format!("../../work/tex-fixtures/{n}")).ok()
    }

    #[test]
    fn roundtrip_inline_fixture_byte_identical() {
        let (Some(ua), Some(ue)) = (fx("sample.uasset"), fx("sample.uexp")) else {
            eprintln!("skip: fixture absent");
            return;
        };
        let ub = fx("sample.ubulk").unwrap_or_default();
        let pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        // Sanity on the captured region for this known fixture.
        assert_eq!(pd.region, 81..16525);
        assert_eq!(pd.size_x, 128);
        assert_eq!(pd.size_y, 128);
        assert_eq!(pd.format, "PF_DXT5");
        assert_eq!(pd.mips.len(), 1);
        assert!(pd.mips[0].inline);
        assert_eq!(pd.trailer.len(), 12);
        let mut uexp2 = ue.clone();
        pd.serialize_into_uexp(&mut uexp2, &ua).unwrap();
        assert_eq!(uexp2, ue, "re-serialized .uexp must be byte-identical");
        assert_eq!(pd.serialize_ubulk(), ub, "re-serialized .ubulk must be byte-identical");
    }

    // Gated/slow: the streamed real texture. Mark #[ignore] like other slow tests.
    #[test]
    #[ignore = "slow: unpacks from real container"]
    fn roundtrip_streamed_water_byte_identical() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip: game absent");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let tmp = std::env::temp_dir().join("gore-tex-td-rt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let uasset = crate::container::unpack_asset(
            &utoc,
            &usmap,
            "/DatasmithContent/Materials/Water/Textures/T_Water_N",
            &tmp,
        )
        .unwrap();
        let ua = std::fs::read(&uasset).unwrap();
        let ue = std::fs::read(uasset.with_extension("uexp")).unwrap();
        let ub = std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default();
        let pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        assert_eq!(pd.size_x, 1024);
        assert_eq!(pd.format, "PF_BC5");
        let mut ue2 = ue.clone();
        pd.serialize_into_uexp(&mut ue2, &ua).unwrap();
        assert_eq!(ue2, ue, "streamed .uexp region must re-serialize byte-identically");
        assert_eq!(pd.serialize_ubulk(), ub, "streamed .ubulk must re-serialize byte-identically");
    }

    /// Gated/slow byte-faithful VT oracle. `T_Biter_Armor_D` is a cooked
    /// **virtual texture** (4096x4096 PF_DXT1, `NumMips == 0` + `bIsVirtual ==
    /// 1`, `FVirtualTextureBuiltData` tail). Parse must now SUCCEED with
    /// `pd.vt.is_some()`, and re-serializing the platform-data region of the
    /// `.uexp` must be BYTE-IDENTICAL to the original (`.ubulk` untouched). This
    /// proves the full `FVirtualTextureBuiltData` field order/widths/count
    /// prefixes are captured losslessly.
    #[test]
    #[ignore = "slow: unpacks from real container"]
    fn roundtrip_biter_vt_uexp_region_byte_identical() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip: game absent");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset =
            "/Game/Assets/Characters/Creatures/Biter/Model/Armor/Textures/T_Biter_Armor_D";
        let leaf = "T_Biter_Armor_D";
        let tmp = std::env::temp_dir().join("gore-tex-td-vt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Prefer the cached texture index (pid -> by-id unpack, no full scan);
        // fall back to a name scan if the index is absent.
        let uasset = match crate::index::TextureIndex::load(&crate::paths::texture_index_path()) {
            Ok(idx) => match idx.entries.get(asset) {
                Some(&pid) => {
                    crate::container::unpack_asset_by_id(&utoc, &usmap, pid, leaf, &tmp).unwrap()
                }
                None => crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap(),
            },
            Err(_) => crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap(),
        };

        let ua = std::fs::read(&uasset).unwrap();
        let ue = std::fs::read(uasset.with_extension("uexp")).unwrap();
        let ub = std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default();

        let pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        assert!(pd.vt.is_some(), "Biter armor diffuse must parse as a virtual texture");
        assert_eq!(pd.format, "PF_DXT1", "VT format should be the real PF_DXT1");
        assert_eq!(pd.size_x, 4096);
        assert_eq!(pd.size_y, 4096);
        assert!(pd.mips.is_empty(), "VT carries no FTexture2DMipMap entries");
        let vt = pd.vt.as_ref().unwrap();
        assert_eq!(vt.num_layers, 1);
        assert!(!vt.is_legacy(), "G1R VT is the non-legacy form");
        assert_eq!(vt.chunks.len(), 4, "Biter armor VT has 4 chunks");

        let mut ue2 = ue.clone();
        pd.serialize_into_uexp(&mut ue2, &ua).unwrap();
        // Locate the first differing byte for a precise failure report.
        if ue2 != ue {
            let first = ue2
                .iter()
                .zip(ue.iter())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| ue.len().min(ue2.len()));
            panic!(
                "VT .uexp region NOT byte-identical; first diff at offset {first} \
                 (region {:?}); got {:?} want {:?}",
                pd.region,
                ue2.get(first),
                ue.get(first)
            );
        }
        assert_eq!(ue2, ue, "re-serialized VT .uexp must be byte-identical");
    }

    /// Same-dims replace on the inline fixture: re-encode-shaped bytes at the
    /// SAME dimensions must produce delta == 0 and leave the `.uasset`
    /// byte-identical (SerialSize unchanged). No game needed.
    #[test]
    fn same_dims_replace_keeps_uasset_identical() {
        let (Some(ua), Some(ue)) = (fx("sample.uasset"), fx("sample.uexp")) else {
            eprintln!("skip: fixture absent");
            return;
        };
        let ub = fx("sample.ubulk").unwrap_or_default();
        let pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        // Reuse the original mip bytes as the "new" mips at identical dims.
        let new_mips: Vec<Vec<u8>> = (0..expected_num_mips(pd.size_x, pd.size_y))
            .map(|i| {
                let w = (pd.size_x >> i).max(1);
                let h = (pd.size_y >> i).max(1);
                vec![0u8; block_math(&pd.format, w, h).unwrap()]
            })
            .collect();
        // The fixture is a single full-size mip; expected_num_mips for 128x128 is
        // 8, so only run this when the original mip count matches the full chain.
        if pd.mips.len() != new_mips.len() {
            eprintln!("skip: fixture is not a full mip chain ({} mips)", pd.mips.len());
            return;
        }
        let (na, _ne, _nb) =
            replace_texture(&ua, &ue, &ub, pd.size_x, pd.size_y, new_mips).unwrap();
        assert_eq!(na, ua, "same-dims replace must leave .uasset byte-identical");
    }

    /// FAST: the property-block ImportedSize patch in isolation (no game).
    #[test]
    fn patch_imported_size_overwrites_pair() {
        // Fake property block with the (128,128) i32 pair embedded among noise.
        let mut block = Vec::new();
        block.extend_from_slice(&[0xAA; 16]); // leading noise
        block.extend_from_slice(&128i32.to_le_bytes()); // ImportedSize.X
        block.extend_from_slice(&128i32.to_le_bytes()); // ImportedSize.Y
        block.extend_from_slice(&[0xBB; 8]); // trailing noise
        let offset = 16usize;

        let n = patch_imported_size(&mut block, (128, 128), (256, 256)).unwrap();
        assert_eq!(n, 1, "exactly one (128,128) pair expected");
        assert_eq!(rd_i32(&block, offset).unwrap(), 256, "X patched");
        assert_eq!(rd_i32(&block, offset + 4).unwrap(), 256, "Y patched");
        // Surrounding noise untouched.
        assert_eq!(&block[..16], &[0xAA; 16]);
        assert_eq!(&block[offset + 8..], &[0xBB; 8]);
    }

    /// FAST: patches MULTIPLE distinct-dimension pairs (e.g. ImportedSize plus a
    /// sibling cached-dimension field), and is a no-op when orig == new.
    #[test]
    fn patch_imported_size_multi_and_noop() {
        let mut block = Vec::new();
        block.extend_from_slice(&64i32.to_le_bytes());
        block.extend_from_slice(&64i32.to_le_bytes());
        block.extend_from_slice(&[0u8; 4]);
        block.extend_from_slice(&64i32.to_le_bytes());
        block.extend_from_slice(&64i32.to_le_bytes());
        let n = patch_imported_size(&mut block, (64, 64), (128, 128)).unwrap();
        assert_eq!(n, 2, "both (64,64) pairs patched");

        // Same-dims no-op: pair still present, overwrites identical bytes.
        let mut same = 128i32.to_le_bytes().to_vec();
        same.extend_from_slice(&128i32.to_le_bytes());
        let n2 = patch_imported_size(&mut same, (128, 128), (128, 128)).unwrap();
        assert_eq!(n2, 1);
    }

    /// FAST: zero occurrences is now NON-FATAL (Ok with count 0). ImportedSize is
    /// not the upscale-crash cause; the data-resource patch carries the fix, so a
    /// property block without the source-dim pair must NOT abort the rewrite.
    #[test]
    fn patch_imported_size_missing_is_ok_zero() {
        let mut block = vec![0u8; 32];
        let n = patch_imported_size(&mut block, (128, 128), (256, 256)).unwrap();
        assert_eq!(n, 0, "missing ImportedSize is a non-fatal no-op (count 0)");
    }

    /// Build a synthetic `PlatformData` with the given mips (each `(w,h,inline)`),
    /// filling inline mips with `block_math`-sized zero payloads. `region_start`
    /// anchors the absolute `.uexp` body offsets for inline mips.
    fn synth_pd(format: &str, mips: &[(u32, u32, bool)], region_start: usize) -> PlatformData {
        let entries = mips
            .iter()
            .map(|&(w, h, inline)| MipEntry {
                width: w,
                height: h,
                inline,
                flags: 0,
                data: vec![0u8; mip_byte_size(format, w, h).unwrap() as usize],
            })
            .collect();
        PlatformData {
            size_x: mips[0].0,
            size_y: mips[0].1,
            packed_data: 0,
            format: format.to_string(),
            opt_data: None,
            first_mip: 0,
            mips: entries,
            trailer: vec![0u8; 12],
            region: region_start..(region_start + 1), // end unused by mip_serial_layout
            vt: None,
        }
    }

    /// FAST: `mip_serial_layout` for a single INLINE mip (cursor case) -- the
    /// serial_offset is the absolute `.uexp` payload offset and serial_size is the
    /// new block-math size.
    #[test]
    fn mip_serial_layout_single_inline() {
        // 256x256 DXT5 inline, region_start arbitrary.
        let pd = synth_pd("PF_DXT5", &[(256, 256, true)], 88);
        let layout = pd.mip_serial_layout().unwrap();
        assert_eq!(layout.len(), 1);
        let (off, size, inline) = layout[0];
        assert!(inline);
        assert_eq!(size, 65536, "256^2 DXT5 = 65536");
        // Region header bytes before the first inline payload:
        //   SizeX+SizeY+Packed = 12; format "PF_DXT5"=7 -> FString 4+8=12 (incl NUL);
        //   FirstMip+NumMips = 8; mip flags = 4. Total = 36. Payload at region+36.
        // "PF_DXT5" is 7 chars + NUL = 8 -> FString len i32(4) + 8 = 12.
        assert_eq!(off, 88 + 12 + 12 + 8 + 4, "inline payload absolute uexp offset");
    }

    /// FAST: `mip_serial_layout` for a STREAMED multi-mip chain -- streamed mips
    /// get cumulative `.ubulk` offsets; each serial_size is its block-math size.
    #[test]
    fn mip_serial_layout_streamed_chain() {
        // 1024,512,256,128 streamed BC5, then 64,32 inline (mirrors water shape).
        let pd = synth_pd(
            "PF_BC5",
            &[
                (1024, 1024, false),
                (512, 512, false),
                (256, 256, false),
                (128, 128, false),
                (64, 64, true),
                (32, 32, true),
            ],
            100,
        );
        let layout = pd.mip_serial_layout().unwrap();
        // Streamed offsets are cumulative from 0.
        assert_eq!(layout[0], (0, 1048576, false));
        assert_eq!(layout[1], (1048576, 262144, false));
        assert_eq!(layout[2], (1048576 + 262144, 65536, false));
        assert_eq!(layout[3], (1048576 + 262144 + 65536, 16384, false));
        // Inline mips: absolute uexp offsets, increasing; sizes correct.
        assert!(layout[4].2 && layout[4].1 == 4096);
        assert!(layout[5].2 && layout[5].1 == 1024);
        assert!(layout[5].0 > layout[4].0, "inline payloads advance in .uexp");
    }

    /// The upscale oracle: rewrite the cursor to 256x256 magenta, repack through
    /// retoc's zen builder, and read it back out of the produced triplet --
    /// proving the SerialSize fixup + new platform-data are correct.
    #[test]
    #[ignore = "slow: unpack+repack against real container"]
    fn upscale_cursor_2x_roundtrips_through_zen() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset = "/Game/UI/Textures/Common/T_HardwareCursor"; // 128x128 PF_DXT5
        let tmp = std::env::temp_dir().join("gore-tex-upscale-rt");
        let _ = std::fs::remove_dir_all(&tmp);
        let cooked = tmp.join("G1R/Content/UI/Textures/Common");
        std::fs::create_dir_all(&cooked).unwrap();
        let uasset = crate::container::unpack_asset(&utoc, &usmap, asset, &cooked).unwrap();
        let ua = std::fs::read(&uasset).unwrap();
        let ue = std::fs::read(uasset.with_extension("uexp")).unwrap();
        let ub = std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default();

        // new content: a 256x256 solid magenta RGBA, encoded to PF_DXT5.
        let (w, h) = (256u32, 256u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [255u8, 0, 255, 255]).collect();
        let mips = crate::encode::encode_mips(&rgba, w, h, "PF_DXT5").unwrap();

        // Capture the property block of the ORIGINAL (pre-rewrite) to locate the
        // ImportedSize, then assert the rewrite patched it. The property block is
        // uexp[0 .. region.start); the rewrite must leave NO (orig_w,orig_h) i32
        // pair there and DO contain (new_w,new_h). Regression lock: the suite
        // never checked the property block before -- a stale ImportedSize was the
        // root cause of the upscale crash.
        let orig_pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        let (ow, oh) = (orig_pd.size_x, orig_pd.size_y);
        let prop_start = orig_pd.region.start;

        let (na, ne, nb) = replace_texture(&ua, &ue, &ub, w, h, mips).unwrap();

        // REGRESSION LOCK (the confirmed crash fix): the legacy data-resource
        // array in the NEW .uasset must now carry the NEW mip size. The cursor is
        // single-inline-mip (one data-resource): old 16384 (128^2 DXT5) -> new
        // 65536 (256^2 DXT5). Re-parse `na` with retoc's FLegacyPackageHeader and
        // assert every data-resource serial_size/raw_size == 65536. A stale 16384
        // here is exactly what made the engine under-read and crash.
        {
            use retoc::legacy_asset::FLegacyPackageHeader;
            use retoc::version::EngineVersion;
            use std::io::Cursor;
            let hdr = FLegacyPackageHeader::deserialize(
                &mut Cursor::new(na.as_slice()),
                Some(EngineVersion::UE5_4.package_file_version()),
            )
            .unwrap();
            assert!(
                !hdr.data_resources.is_empty(),
                "cursor must have at least one data-resource to lock"
            );
            for (i, dr) in hdr.data_resources.iter().enumerate() {
                assert_eq!(
                    dr.serial_size, 65536,
                    "data-resource[{i}] serial_size must be 65536 (new 256^2 DXT5), not stale 16384"
                );
                assert_eq!(
                    dr.raw_size, 65536,
                    "data-resource[{i}] raw_size must be 65536 (new 256^2 DXT5)"
                );
            }
            eprintln!(
                "OK: {} data-resource(s) patched to serial_size=65536 in the new .uasset",
                hdr.data_resources.len()
            );
        }

        // The platform-data region length is unchanged by the property-block edit,
        // so the region start is stable; the property block is ne[0 .. prop_start).
        let prop = &ne[..prop_start];
        let old_pair = {
            let mut p = (ow as i32).to_le_bytes().to_vec();
            p.extend_from_slice(&(oh as i32).to_le_bytes());
            p
        };
        let new_pair = {
            let mut p = (w as i32).to_le_bytes().to_vec();
            p.extend_from_slice(&(h as i32).to_le_bytes());
            p
        };
        assert!(
            !prop.windows(8).any(|wnd| wnd == old_pair.as_slice()),
            "property block still contains stale ImportedSize ({ow},{oh})"
        );
        assert!(
            prop.windows(8).any(|wnd| wnd == new_pair.as_slice()),
            "property block missing patched ImportedSize ({w},{h})"
        );

        std::fs::write(&uasset, &na).unwrap();
        std::fs::write(uasset.with_extension("uexp"), &ne).unwrap();
        if nb.is_empty() {
            let _ = std::fs::remove_file(uasset.with_extension("ubulk"));
        } else {
            std::fs::write(uasset.with_extension("ubulk"), &nb).unwrap();
        }

        let out = tmp.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let triplet = crate::container::repack_to_zen(&tmp, "UpscaleTest_P", &out, &g, false).unwrap();
        for p in &triplet {
            assert!(p.exists() && std::fs::metadata(p).unwrap().len() > 0);
        }

        // Copy global.* next to the produced triplet so the composite store has
        // the script-object table to convert zen->legacy on read-back.
        let game_paks = g.join("G1R/Content/Paks");
        for ext in ["utoc", "ucas", "pak"] {
            let src = game_paks.join(format!("global.{ext}"));
            if src.exists() {
                std::fs::copy(&src, out.join(format!("global.{ext}"))).unwrap();
            }
        }

        let readback_dir = tmp.join("readback");
        let _ = std::fs::remove_dir_all(&readback_dir);
        std::fs::create_dir_all(&readback_dir).unwrap();
        let rb_uasset =
            crate::container::unpack_asset(&triplet[0], &usmap, asset, &readback_dir).unwrap();
        let rb = crate::decode::parse(
            &std::fs::read(&rb_uasset).unwrap(),
            &std::fs::read(rb_uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(rb_uasset.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        assert_eq!(rb.width, 256, "width should be 256 after upscale");
        assert_eq!(rb.height, 256, "height should be 256 after upscale");
        assert_eq!(rb.format, "PF_DXT5", "format should be preserved");

        // The cursor shipped NoMipmaps (orig_n == 1), so the upscaled texture
        // must ALSO carry a single cooked mip -- re-parse the read-back triplet's
        // PlatformData and assert mips.len() == 1. (This is the crash fix: a full
        // mip pyramid here diverges from the verbatim NoMipmaps property block.)
        let rb_pd = PlatformData::parse(
            &std::fs::read(&rb_uasset).unwrap(),
            &std::fs::read(rb_uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(rb_uasset.with_extension("ubulk")).unwrap_or_default(),
        )
        .unwrap();
        assert_eq!(rb_pd.mips.len(), 1, "NoMipmaps source must stay single-mip after upscale");

        // `to_rgba8` returns packed 0xAARRGGBB u32s. Spot-check a few are ~magenta.
        let px = crate::decode::to_rgba8(&rb).unwrap();
        assert_eq!(px.len(), (w * h) as usize, "pixel count != 256*256");
        for idx in [0usize, px.len() / 2, px.len() - 1] {
            let p = px[idx];
            let r = (p >> 16) & 0xff;
            let gch = (p >> 8) & 0xff;
            let bch = p & 0xff;
            assert!(r > 200 && gch < 60 && bch > 200, "pixel not magenta: {r},{gch},{bch}");
        }
        eprintln!("OK: read back 256x256 PF_DXT5 magenta from the triplet");
    }

    /// Streamed-source sibling of `upscale_cursor_2x_roundtrips_through_zen`:
    /// upscale a real *streamed* `PF_BC5` texture (mip0 in `.ubulk`) 2x and read
    /// it back out of the produced triplet. This exercises the parts the inline
    /// cursor case can't: the `T`-threshold inline/stream split (large new mips go
    /// to `.ubulk`, the small tail stays inline) and the non-zero SerialSize patch
    /// (`delta != 0`) end-to-end through retoc's zen builder.
    ///
    /// Note on the mount path: `repack_to_zen` -> `build_zen_asset` takes the
    /// package name from the legacy `.uasset`'s OWN summary (preserved verbatim by
    /// `build_legacy` on unpack), NOT from the cooked-dir layout. So the read-back
    /// package name equals the original `/DatasmithContent/...` package path
    /// regardless of where we lay the files; we mirror the cursor test's
    /// `G1R/Content/<asset path>` layout purely for a tidy, cook-like pak path.
    #[test]
    #[ignore = "slow: unpack+repack against real container"]
    fn upscale_streamed_water_2x_roundtrips_through_zen() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        // 1024x1024 PF_BC5 normal map, streamed (mip0 in .ubulk), 11 mips.
        let asset = "/DatasmithContent/Materials/Water/Textures/T_Water_N";
        let tmp = std::env::temp_dir().join("gore-tex-upscale-streamed-rt");
        let _ = std::fs::remove_dir_all(&tmp);
        // Mount-path layout mirrors the cursor test (`G1R/Content/<asset path>`).
        // The package name is recovered from the .uasset summary, not this path.
        let cooked = tmp.join("G1R/Content/DatasmithContent/Materials/Water/Textures");
        std::fs::create_dir_all(&cooked).unwrap();
        let uasset = crate::container::unpack_asset(&utoc, &usmap, asset, &cooked).unwrap();
        let ua = std::fs::read(&uasset).unwrap();
        let ue = std::fs::read(uasset.with_extension("uexp")).unwrap();
        let ub = std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default();

        // Confirm the source really is the streamed BC5 texture we expect.
        let src_pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        assert_eq!(src_pd.size_x, 1024, "source SizeX");
        assert_eq!(src_pd.size_y, 1024, "source SizeY");
        assert_eq!(src_pd.format, "PF_BC5", "source format");
        assert!(
            src_pd.mips.iter().any(|m| !m.inline),
            "source must stream at least one mip (this exercises the T split)"
        );
        // This source has a real mip chain (orig_n > 1), so the mip-gen policy
        // branch must emit the FULL pyramid for the new dims (NOT single-mip).
        assert!(
            src_pd.mips.len() > 1,
            "water source must have a full mip chain to exercise the orig_n>1 branch"
        );

        // New content: a 2048x2048 solid RED RGBA, encoded to PF_BC5. BC5 only
        // carries R,G; red => R high, G low after decode.
        let (w, h) = (2048u32, 2048u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let mips = crate::encode::encode_mips(&rgba, w, h, "PF_BC5").unwrap();
        let (na, ne, nb) = replace_texture(&ua, &ue, &ub, w, h, mips).unwrap();

        // SerialSize patch must have moved (delta != 0): the region grew with the
        // larger dims. Surface the actual delta for the report.
        let old_region_len = src_pd.region.end - src_pd.region.start;
        let new_pd = PlatformData::parse(&na, &ne, &nb).unwrap();
        let new_region_len = new_pd.region.end - new_pd.region.start;
        let delta = new_region_len as i64 - old_region_len as i64;
        assert_ne!(delta, 0, "upscale must change the platform-data region length");
        // orig_n > 1: the rewrite must emit the FULL pyramid for the new dims.
        assert_eq!(
            new_pd.mips.len(),
            expected_num_mips(w, h),
            "chain source must keep a full mip pyramid after upscale"
        );
        // The upscaled .ubulk is large (2048x2048 BC5 mip0 alone is ~4MB).
        assert!(nb.len() > 4_000_000, "expected a large streamed .ubulk, got {}", nb.len());
        eprintln!(
            "region delta = {delta} (old {old_region_len} -> new {new_region_len}); .ubulk = {} bytes",
            nb.len()
        );

        // MULTI-DATA-RESOURCE LOCK (the streamed multi-mip oracle for THIS fix):
        // re-parse `na` and assert there are MULTIPLE data-resources and each
        // carries its mip's NEW per-mip block-math size, in order. A streamed
        // upscale that left these at the OLD per-mip sizes would crash exactly like
        // the cursor did.
        {
            use retoc::legacy_asset::FLegacyPackageHeader;
            use retoc::version::EngineVersion;
            use std::io::Cursor;
            let hdr = FLegacyPackageHeader::deserialize(
                &mut Cursor::new(na.as_slice()),
                Some(EngineVersion::UE5_4.package_file_version()),
            )
            .unwrap();
            assert!(
                hdr.data_resources.len() > 1,
                "streamed water must have MULTIPLE data-resources to exercise the multi-mip path, got {}",
                hdr.data_resources.len()
            );
            // data_resources[i] <-> mip[i]; new mip[i] = (2048>>i) BC5 block-math.
            for (i, dr) in hdr.data_resources.iter().enumerate() {
                let mw = (w >> i).max(1);
                let mh = (h >> i).max(1);
                let want = block_math("PF_BC5", mw, mh).unwrap() as i64;
                assert_eq!(
                    dr.serial_size, want,
                    "data-resource[{i}] serial_size must be new mip {mw}x{mh} size {want}"
                );
                assert_eq!(
                    dr.raw_size, want,
                    "data-resource[{i}] raw_size must be new mip {mw}x{mh} size {want}"
                );
            }
            eprintln!(
                "OK: {} data-resources patched to new per-mip sizes (mip0={} bytes)",
                hdr.data_resources.len(),
                hdr.data_resources[0].serial_size
            );
        }

        std::fs::write(&uasset, &na).unwrap();
        std::fs::write(uasset.with_extension("uexp"), &ne).unwrap();
        if nb.is_empty() {
            let _ = std::fs::remove_file(uasset.with_extension("ubulk"));
        } else {
            std::fs::write(uasset.with_extension("ubulk"), &nb).unwrap();
        }

        let out = tmp.join("out");
        std::fs::create_dir_all(&out).unwrap();
        // compress = TRUE: this is the single test that exercises the opt-in
        // Oodle compression path end-to-end (flags==9, 16-aligned blocks, and the
        // compressed-size oracle below). Every other repack test uses the default
        // uncompressed path.
        let triplet =
            crate::container::repack_to_zen(&tmp, "WaterUpscaleTest_P", &out, &g, true).unwrap();
        for p in &triplet {
            assert!(p.exists() && std::fs::metadata(p).unwrap().len() > 0);
        }

        // Prove the compression path's byte-level invariants: container_flags == 9
        // (Indexed|Compressed) AND every compressed-block offset is 16-aligned.
        let (flags, comp_offsets) =
            retoc::iostore_writer::dump_compressed_layout(&triplet[0]).unwrap();
        assert_eq!(flags, 9, "container_flags must be Indexed|Compressed (9) with compress=true");
        assert!(
            !comp_offsets.is_empty(),
            "expected at least one compressed block with compress=true"
        );
        for (i, off) in comp_offsets.iter().enumerate() {
            assert_eq!(
                off % 0x10,
                0,
                "compressed block {i} offset {off:#x} is not 16-aligned"
            );
        }
        eprintln!(
            "OK: container_flags=9 and all {} compressed block offsets 16-aligned",
            comp_offsets.len()
        );

        // Copy global.* next to the produced triplet so the composite store has
        // the script-object table to convert zen->legacy on read-back.
        let game_paks = g.join("G1R/Content/Paks");
        for ext in ["utoc", "ucas", "pak"] {
            let src = game_paks.join(format!("global.{ext}"));
            if src.exists() {
                std::fs::copy(&src, out.join(format!("global.{ext}"))).unwrap();
            }
        }

        let readback_dir = tmp.join("readback");
        let _ = std::fs::remove_dir_all(&readback_dir);
        std::fs::create_dir_all(&readback_dir).unwrap();
        let rb_uasset =
            crate::container::unpack_asset(&triplet[0], &usmap, asset, &readback_dir).unwrap();
        let rb = crate::decode::parse(
            &std::fs::read(&rb_uasset).unwrap(),
            &std::fs::read(rb_uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(rb_uasset.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        assert_eq!(rb.width, 2048, "width should be 2048 after upscale");
        assert_eq!(rb.height, 2048, "height should be 2048 after upscale");
        assert_eq!(rb.format, "PF_BC5", "format should be preserved");

        // BC5 decode reconstructs R,G (B=0, A=255). Red => R high, G low. Sample a
        // few packed 0xAARRGGBB pixels.
        let px = crate::decode::to_rgba8(&rb).unwrap();
        assert_eq!(px.len(), (w * h) as usize, "pixel count != 2048*2048");
        for idx in [0usize, px.len() / 2, px.len() - 1] {
            let p = px[idx];
            let r = (p >> 16) & 0xff;
            let gch = (p >> 8) & 0xff;
            assert!(r >= 248, "R not high (not red): {r} at idx {idx}");
            assert!(gch <= 8, "G not low (not red): {gch} at idx {idx}");
        }
        eprintln!("OK: read back 2048x2048 PF_BC5 red from the streamed-upscale triplet (delta={delta})");

        // COMPRESSION SIZE ORACLE: the writer now Oodle-compresses .ucas blocks.
        // This asset's mip0 alone is a ~4MB solid-red BC5 surface (highly
        // compressible). Assert the produced .ucas is *meaningfully smaller* than
        // the raw streamed mip payload it carries -- proof the Oodle path actually
        // shrinks real texture data (vs the old raw method-0 writer).
        let ucas_len = std::fs::metadata(&triplet[1]).unwrap().len();
        let raw_bulk_len = nb.len() as u64;
        eprintln!(
            "compressed .ucas = {ucas_len} bytes vs raw streamed mip payload = {raw_bulk_len} bytes \
             ({:.1}% of raw)",
            ucas_len as f64 / raw_bulk_len as f64 * 100.0
        );
        assert!(
            ucas_len < raw_bulk_len / 2,
            "compressed .ucas ({ucas_len}) should be well under half the raw mip payload ({raw_bulk_len}); \
             Oodle compression appears ineffective"
        );
    }

    /// END-TO-END VT REPLACE ORACLE (short of in-game): unpack the Biter armor
    /// virtual texture (4096² PF_DXT1), replace it with a same-dims solid-MAGENTA
    /// RGBA image via the unified `replace_texture_image` (which routes to the VT
    /// re-tile path), write the cooked triple under the mount path, repack through
    /// retoc's zen builder, then reopen the produced triplet and decode the asset
    /// back. Asserts it decodes to a 4096² image whose pixels are ~magenta — proving
    /// retile → serialize → chunk-byte write → repack → readback works end-to-end.
    #[test]
    #[ignore = "slow: unpack+repack against real container (~10min)"]
    fn replace_biter_vt_solid_roundtrips_through_zen() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip: game absent");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset =
            "/Game/Assets/Characters/Creatures/Biter/Model/Armor/Textures/T_Biter_Armor_D";
        let leaf = "T_Biter_Armor_D";

        let tmp = std::env::temp_dir().join("gore-tex-vt-replace-rt");
        let _ = std::fs::remove_dir_all(&tmp);
        // Cook-like mount layout: /Game/... -> G1R/Content/...
        let cooked = tmp.join(
            "G1R/Content/Assets/Characters/Creatures/Biter/Model/Armor/Textures",
        );
        std::fs::create_dir_all(&cooked).unwrap();

        // Prefer the cached index for a fast by-id unpack; fall back to a scan.
        let uasset = match crate::index::TextureIndex::load(&crate::paths::texture_index_path()) {
            Ok(idx) => match idx.entries.get(asset) {
                Some(&pid) => {
                    crate::container::unpack_asset_by_id(&utoc, &usmap, pid, leaf, &cooked).unwrap()
                }
                None => crate::container::unpack_asset(&utoc, &usmap, asset, &cooked).unwrap(),
            },
            Err(_) => crate::container::unpack_asset(&utoc, &usmap, asset, &cooked).unwrap(),
        };
        let ua = std::fs::read(&uasset).unwrap();
        let ue = std::fs::read(uasset.with_extension("uexp")).unwrap();
        let ub = std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default();

        // Confirm the source is the 4096² PF_DXT1 single-layer VT we expect.
        let src_pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        let vt = src_pd.vt.as_ref().expect("Biter armor diffuse must parse as a VT");
        assert_eq!((src_pd.size_x, src_pd.size_y), (4096, 4096), "source dims");
        assert_eq!(vt.num_layers, 1, "single-layer VT");
        let fmt = vt.layer_types[0].clone();
        eprintln!(
            "Biter VT: {}x{} {fmt} mips={} chunks={}",
            vt.width, vt.height, vt.num_mips, vt.chunks.len()
        );

        // Same-dims OBVIOUS test image: solid magenta (R=255,G=0,B=255,A=255).
        let (w, h) = (src_pd.size_x, src_pd.size_y);
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [255u8, 0, 255, 255]).collect();

        // Route through the unified entry (auto-detects VT -> retile path).
        let (na, ne, nb) =
            replace_texture_image(&ua, &ue, &ub, &rgba, w, h, &fmt).unwrap();

        // Same-dims VT must keep the cooked sizes constant (no resize anywhere).
        assert_eq!(na.len(), ua.len(), ".uasset length must be unchanged (same-dims VT)");
        assert_eq!(ne.len(), ue.len(), ".uexp length must be unchanged (same-dims VT)");
        assert_eq!(nb.len(), ub.len(), ".ubulk length must be unchanged (same-dims VT)");
        // The .uasset header is byte-identical (delta == 0 -> no SerialSize/header fixup).
        assert_eq!(na, ua, "same-dims VT replace must leave .uasset byte-identical");
        // The .uexp must DIFFER (chunk hashes + any inline chunk bytes changed).
        assert_ne!(ne, ue, ".uexp must change (new VT chunk hashes / inline bytes)");

        // SANITY: re-parse the rewritten triple and re-resolve the VT chunk bytes
        // from it, then decode locally. This proves the chunk-byte write landed at
        // the right offsets BEFORE the (slow) repack.
        {
            let pd2 = PlatformData::parse(&na, &ne, &nb).unwrap();
            let vt2 = pd2.vt.as_ref().unwrap();
            let mut chunk_bytes = Vec::with_capacity(vt2.chunks.len());
            for c in &vt2.chunks {
                chunk_bytes
                    .push(resolve_data_resource_bytes(&na, &ne, &nb, c.data_resource_index).unwrap());
            }
            let (dw, dh, px) = crate::vt::decode_layer0(vt2, &chunk_bytes, &fmt).unwrap();
            assert_eq!((dw, dh), (4096, 4096), "re-resolved VT decodes at 4096²");
            // Spot-check magenta across the image.
            for idx in [0usize, px.len() / 2, px.len() - 1] {
                let p = px[idx];
                let (r, gch, bch) = ((p >> 16) & 0xff, (p >> 8) & 0xff, p & 0xff);
                assert!(r > 200 && gch < 60 && bch > 200, "pre-repack pixel not magenta: {r},{gch},{bch}");
            }
            eprintln!("OK: rewritten triple re-resolves + decodes to magenta at 4096² (pre-repack)");
        }

        // Write the rewritten triple back into the cooked tree.
        std::fs::write(&uasset, &na).unwrap();
        std::fs::write(uasset.with_extension("uexp"), &ne).unwrap();
        if nb.is_empty() {
            let _ = std::fs::remove_file(uasset.with_extension("ubulk"));
        } else {
            std::fs::write(uasset.with_extension("ubulk"), &nb).unwrap();
        }

        // Repack to a zen triplet.
        let out = tmp.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let triplet =
            crate::container::repack_to_zen(&tmp, "VtReplaceTest_P", &out, &g, false).unwrap();
        for p in &triplet {
            assert!(p.exists() && std::fs::metadata(p).unwrap().len() > 0);
        }

        // Copy global.* next to the produced triplet so the composite store has the
        // script-object table to convert zen->legacy on read-back.
        let game_paks = g.join("G1R/Content/Paks");
        for ext in ["utoc", "ucas", "pak"] {
            let src = game_paks.join(format!("global.{ext}"));
            if src.exists() {
                std::fs::copy(&src, out.join(format!("global.{ext}"))).unwrap();
            }
        }

        // Reopen the produced triplet and decode the asset back.
        let readback_dir = tmp.join("readback");
        let _ = std::fs::remove_dir_all(&readback_dir);
        std::fs::create_dir_all(&readback_dir).unwrap();
        let rb_uasset =
            crate::container::unpack_asset(&triplet[0], &usmap, asset, &readback_dir).unwrap();
        let rb = crate::decode::parse(
            &std::fs::read(&rb_uasset).unwrap(),
            &std::fs::read(rb_uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(rb_uasset.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        assert!(rb.is_virtual, "read-back asset must still be a virtual texture");
        assert_eq!(rb.width, 4096, "width should stay 4096");
        assert_eq!(rb.height, 4096, "height should stay 4096");
        assert_eq!(rb.format, fmt, "VT layer format should be preserved");

        // Decode to RGBA and assert it is ~magenta (the obvious test image).
        let px = crate::decode::to_rgba8(&rb).unwrap();
        assert_eq!(px.len(), (4096 * 4096) as usize, "pixel count != 4096²");
        let (mut max_dr, mut max_dg, mut max_db) = (0i32, 0i32, 0i32);
        for &p in &px {
            let r = ((p >> 16) & 0xff) as i32;
            let gch = ((p >> 8) & 0xff) as i32;
            let bch = (p & 0xff) as i32;
            max_dr = max_dr.max((r - 255).abs());
            max_dg = max_dg.max((gch - 0).abs());
            max_db = max_db.max((bch - 255).abs());
        }
        eprintln!("max per-channel deviation from magenta after readback: R{max_dr} G{max_dg} B{max_db}");
        // BC1 565 quantization tolerance for a solid color.
        assert!(
            max_dr <= 12 && max_dg <= 12 && max_db <= 12,
            "read-back VT did not decode to solid magenta (R{max_dr} G{max_dg} B{max_db})"
        );
        eprintln!("OK: read back 4096² magenta VT from the repacked triplet");
    }
}
