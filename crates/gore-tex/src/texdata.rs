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
}
