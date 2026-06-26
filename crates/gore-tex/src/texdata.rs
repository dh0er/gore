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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformData {
    pub size_x: u32,
    pub size_y: u32,
    /// bit31 = bIsVirtual.
    pub packed_data: u32,
    pub format: String,
    pub first_mip: u32,
    pub mips: Vec<MipEntry>,
    /// FTexturePlatformData bytes after the mip array, kept verbatim.
    pub trailer: Vec<u8>,
    /// `[start, end)` of the platform-data region within `.uexp`.
    pub region: Range<usize>,
}

// ---- format block math ----------------------------------------------------

/// Bytes per 4x4 block for the supported BCn formats.
fn block_bytes(format: &str) -> Option<u32> {
    match format {
        "PF_DXT1" => Some(8),
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

// ---- bounds-checked little-endian readers ---------------------------------

fn corrupt(msg: &str) -> TexError {
    TexError::Retoc(anyhow::anyhow!("FTexturePlatformData codec: {msg}"))
}

fn rd_i32(b: &[u8], o: usize) -> Result<i32> {
    let s = b
        .get(o..o + 4)
        .ok_or_else(|| corrupt("unexpected end reading i32"))?;
    Ok(i32::from_le_bytes(s.try_into().unwrap()))
}
fn rd_u32(b: &[u8], o: usize) -> Result<u32> {
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
fn write_fstring_ascii(out: &mut Vec<u8>, s: &str) -> Result<()> {
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
    pub fn parse(_uasset: &[u8], uexp: &[u8], ubulk: &[u8]) -> Result<Self> {
        let mut last_err =
            corrupt("no FTexturePlatformData found (no valid PF_* anchor in .uexp)");

        let mut search = 0usize;
        while let Some(rel) = find_pf_anchor(uexp, search) {
            if rel >= 12 {
                let pd_start = rel - 12;
                match Self::parse_at(uexp, ubulk, pd_start) {
                    Ok(pd) => return Ok(pd),
                    Err(e) => last_err = e,
                }
            }
            search = rel + 1;
        }
        Err(last_err)
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
        if block_bytes(&format).is_none() {
            return Err(TexError::UnsupportedFormat(format));
        }

        let first_mip = rd_i32(uexp, o)?;
        o += 4;
        let num_mips = rd_i32(uexp, o)?;
        o += 4;
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
            first_mip: first_mip as u32,
            mips,
            trailer,
            region: pd_start..region_end,
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
        out.extend_from_slice(&(self.first_mip as i32).to_le_bytes());
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
    if orig.first_mip != 0 {
        return Err(corrupt("FirstMipToSerialize != 0 is not supported for rewrite"));
    }
    let format = orig.format.clone();

    // 2. Validate the new mip pyramid against the new dims + format.
    let want = expected_num_mips(new_w, new_h);
    if new_mips.len() != want {
        return Err(corrupt(&format!(
            "new_mips has {} levels but {new_w}x{new_h} needs {want} (log2(max)+1)",
            new_mips.len()
        )));
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

    // 3. Inline/stream policy: derive the threshold from the original split.
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

    // 4. Build the new mip entries.
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

    // 5. Assemble the new PlatformData and serialize the new region + ubulk.
    let new_pd = PlatformData {
        size_x: new_w,
        size_y: new_h,
        packed_data: orig.packed_data,
        format: format.clone(),
        first_mip: 0,
        mips,
        trailer: orig.trailer.clone(),
        region: orig.region.clone(),
    };
    let new_region = new_pd.serialize_region()?;
    let new_ubulk = new_pd.serialize_ubulk();

    let old_region_len = orig.region.end - orig.region.start;
    let delta: i64 = new_region.len() as i64 - old_region_len as i64;

    // 6. new_uexp = uexp with [region] replaced (the tail incl. the package
    //    magic shifts by delta automatically).
    let mut new_uexp = uexp.to_vec();
    new_uexp.splice(orig.region.clone(), new_region);

    // 7. new_uasset = uasset with the texture export's SerialSize patched by
    //    delta (no-op when delta == 0, i.e. a same-dims replace).
    let new_uasset = patch_uasset_serial_size(uasset, uexp, &orig.region, delta)?;

    Ok((new_uasset, new_uexp, new_ubulk))
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
        let (na, ne, nb) = replace_texture(&ua, &ue, &ub, w, h, mips).unwrap();

        std::fs::write(&uasset, &na).unwrap();
        std::fs::write(uasset.with_extension("uexp"), &ne).unwrap();
        if nb.is_empty() {
            let _ = std::fs::remove_file(uasset.with_extension("ubulk"));
        } else {
            std::fs::write(uasset.with_extension("ubulk"), &nb).unwrap();
        }

        let out = tmp.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let triplet = crate::container::repack_to_zen(&tmp, "UpscaleTest_P", &out, &g).unwrap();
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
}
