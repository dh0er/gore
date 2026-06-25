//! gore-fmod — FMOD Studio sound bank (`.bank`, RIFF `FEV `) decrypt + parse.
//!
//! Foundation for M0 (decrypt spike) and later extract/repack. Pure Rust, no FMOD
//! dependency. Encryption is the classic symmetric FSB5 cipher (bit-reverse + cycling
//! XOR), applied only to the embedded FSB5 sub-blocks; the FEV/RIFF metadata is plaintext.
//!
//! Refs: vgmstream `meta/fsb5.c`, `meta/fsb5_fev.c`, `meta/fsb_encrypted_streamfile.h`.

pub mod vorbis;

// ---------- little/big-endian readers ----------
#[inline]
pub fn u32_le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
#[inline]
pub fn u32_be(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
#[inline]
pub fn u64_le(b: &[u8], o: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(a)
}

// ---------- FSB5 cipher (symmetric, position-indexed) ----------
/// Decrypt an FSB5 sub-block in place. `key` = raw ASCII key bytes (no NUL).
/// `plain[i] = reverse_bits(cipher[i]) ^ key[i % key.len()]`. Index 0 = block start.
pub fn fsb5_decrypt(data: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return; // guard against `% key.len()` divide-by-zero; callers reject empty keys
    }
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = byte.reverse_bits() ^ key[i % key.len()];
    }
}
/// Inverse of [`fsb5_decrypt`]. `cipher[i] = reverse_bits(plain[i] ^ key[i % len])`.
pub fn fsb5_encrypt(data: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return; // guard against `% key.len()` divide-by-zero; callers reject empty keys
    }
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = (*byte ^ key[i % key.len()]).reverse_bits();
    }
}

// ---------- FEV bank wrapper ----------
#[derive(Debug, Clone, Copy)]
pub struct BankEntry {
    pub fsb5_offset: usize,
    pub fsb5_size: usize,
}

/// Walk the RIFF/`FEV ` wrapper, return the (still-encrypted) embedded FSB5 slices.
pub fn parse_bank(b: &[u8]) -> Result<Vec<BankEntry>, String> {
    if b.len() < 0x18 || &b[0x00..0x04] != b"RIFF" || &b[0x08..0x0C] != b"FEV " {
        return Err("not a RIFF/FEV bank".into());
    }
    let version = u32_le(b, 0x14); // bank version (lives in FMT body)

    // top-level RIFF chunk walk from 0x0C: fourcc(4) + u32_le size(4) + body.
    let list_body = {
        let mut off = 0x0C;
        let mut found = None;
        while off + 8 <= b.len() {
            let fourcc = &b[off..off + 4];
            let size = u32_le(b, off + 4) as usize;
            let body = off + 8;
            if fourcc == b"LIST" {
                found = Some(body);
                break;
            }
            off = body + size;
        }
        found.ok_or("no top-level LIST chunk")?
    };

    if &b[list_body..list_body + 4] != b"PROJ" {
        return Err(format!(
            "LIST body not PROJ (got {:02x?})",
            &b[list_body..list_body + 4]
        ));
    }
    if &b[list_body + 4..list_body + 8] != b"BNKI" {
        return Err("PROJ not followed by BNKI (event .fev, not a baked bank?)".into());
    }

    // walk sub-chunks starting just after PROJ; find SNDH (direct or nested in LIST).
    let mut off = list_body + 4;
    let end = b.len();
    let (mut sndh_off, mut sndh_size) = (0usize, 0usize);
    while sndh_off == 0 && off + 8 <= end {
        let ctype = u32_be(b, off);
        let csize = u32_le(b, off + 4) as usize;
        off += 8;
        match ctype {
            0x4C49_5354 => {
                // "LIST" — nested; check for SNDH
                if off + 8 <= end && u32_be(b, off + 4) == 0x534E_4448 {
                    sndh_off = off + 0x0C;
                    sndh_size = u32_le(b, off + 8) as usize;
                }
            }
            0x534E_4448 => {
                // "SNDH" directly
                sndh_off = off;
                sndh_size = csize;
            }
            0xFFFF_FFFF => return Err("malformed chunk".into()),
            _ => {}
        }
        off += csize;
    }
    if sndh_off == 0 {
        return Err("no SNDH chunk (no embedded FSB5)".into());
    }

    let entry_size = if version <= 0x28 { 4 } else { 8 };
    if sndh_size < 4 {
        return Err("SNDH too small".into());
    }
    let banks = (sndh_size - 4) / entry_size; // skip 4-byte chunk-version
    let mut out = Vec::with_capacity(banks);
    for i in 0..banks {
        let base = sndh_off + 4 + entry_size * i;
        let o = u32_le(b, base) as usize;
        let s = if entry_size == 8 {
            u32_le(b, base + 4) as usize
        } else {
            0 // size reconstructed from FSB5 header for old banks
        };
        out.push(BankEntry {
            fsb5_offset: o,
            fsb5_size: s,
        });
    }
    Ok(out)
}

// ---------- FSB5 container ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    None,
    Pcm8,
    Pcm16,
    Pcm24,
    Pcm32,
    PcmFloat,
    GcAdpcm,
    ImaAdpcm,
    Vag,
    HeVag,
    Xma,
    Mpeg,
    Celt,
    At9,
    XWma,
    Vorbis,
    FAdpcm,
    Opus,
    Unknown(u32),
}
impl Codec {
    pub fn from_u32(v: u32) -> Codec {
        use Codec::*;
        match v {
            0 => None,
            1 => Pcm8,
            2 => Pcm16,
            3 => Pcm24,
            4 => Pcm32,
            5 => PcmFloat,
            6 => GcAdpcm,
            7 => ImaAdpcm,
            8 => Vag,
            9 => HeVag,
            10 => Xma,
            11 => Mpeg,
            12 => Celt,
            13 => At9,
            14 => XWma,
            15 => Vorbis,
            16 => FAdpcm,
            17 => Opus,
            other => Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fsb5Sample {
    pub name: String,
    pub data_offset: u64, // relative to data section start
    pub size: u64,        // encoded byte size
    pub freq: u32,
    pub channels: u32,
    pub num_samples: u32, // decoded PCM frames per channel
    pub vorbis_crc32: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Fsb5 {
    pub version: u32,
    pub codec: Codec,
    pub data_section: u64,
    pub samples: Vec<Fsb5Sample>,
}

const FREQ: [u32; 11] = [
    4000, 8000, 11000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

/// Parse a DECRYPTED FSB5 block.
pub fn parse_fsb5(b: &[u8]) -> Result<Fsb5, String> {
    if b.len() < 0x3C || &b[0..4] != b"FSB5" {
        return Err(format!(
            "no FSB5 magic (got {:02x?})",
            &b[0..4.min(b.len())]
        ));
    }
    let version = u32_le(b, 0x04);
    let n = u32_le(b, 0x08) as usize;
    let shdr_size = u32_le(b, 0x0C) as u64;
    let name_tbl_size = u32_le(b, 0x10) as u64;
    let data_size = u32_le(b, 0x14) as u64;
    let codec = Codec::from_u32(u32_le(b, 0x18));
    let base = if version == 0 { 0x40u64 } else { 0x3Cu64 };

    let name_tbl = (base + shdr_size) as usize;
    let data_section = base + shdr_size + name_tbl_size;

    // pass 1: decode base words + chunks
    struct Raw {
        data_off: u64,
        ch: u32,
        freq: u32,
        ns: u32,
        vcrc: Option<u32>,
    }
    let mut raws: Vec<Raw> = Vec::with_capacity(n);
    let mut off = base as usize;
    for _ in 0..n {
        if off + 8 > b.len() {
            return Err("sample header overrun".into());
        }
        let m = u64_le(b, off);
        let has_chunks = m & 1;
        let freq_idx = ((m >> 1) & 0xF) as usize;
        let ch_e = ((m >> 5) & 0x3) as u32;
        let data_off = ((m >> 7) & 0x07FF_FFFF) << 5;
        let ns = ((m >> 34) & 0x3FFF_FFFF) as u32;
        let mut ch = match ch_e {
            0 => 1,
            1 => 2,
            2 => 6,
            3 => 8,
            _ => 0,
        };
        let mut freq = *FREQ.get(freq_idx).unwrap_or(&0);
        let mut vcrc = None;
        let mut hsz = 0x08usize;
        if has_chunks == 1 {
            let mut co = off + 8;
            loop {
                if co + 4 > b.len() {
                    return Err("chunk overrun".into());
                }
                let w = u32_le(b, co);
                let more = w & 1;
                let csz = ((w >> 1) & 0xFF_FFFF) as usize;
                let ctyp = (w >> 25) & 0x7F;
                match ctyp {
                    0x01 => ch = b[co + 4] as u32,
                    0x02 => freq = u32_le(b, co + 4),
                    0x0B => vcrc = Some(u32_le(b, co + 4)),
                    0x0E => ch *= u32_le(b, co + 4),
                    _ => {}
                }
                co += 4 + csz;
                hsz += 4 + csz;
                if more == 0 {
                    break;
                }
            }
        }
        raws.push(Raw {
            data_off,
            ch,
            freq,
            ns,
            vcrc,
        });
        off += hsz;
    }

    // pass 2: sizes + names
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let r = &raws[i];
        let size = if i + 1 == n {
            data_size - r.data_off
        } else {
            raws[i + 1].data_off - r.data_off
        };
        let name = if name_tbl_size > 0 {
            let rel = u32_le(b, name_tbl + 4 * i) as usize;
            read_cstr(&b[name_tbl + rel..])
        } else {
            format!("{:04}", i)
        };
        samples.push(Fsb5Sample {
            name,
            data_offset: r.data_off,
            size,
            freq: r.freq,
            channels: r.ch,
            num_samples: r.ns,
            vorbis_crc32: r.vcrc,
        });
    }

    Ok(Fsb5 {
        version,
        codec,
        data_section,
        samples,
    })
}

fn read_cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

/// The recovered FMOD Studio bank encryption key for Gothic 1 Remake (StudioBankKey).
pub const GOTHIC_STUDIO_KEY: &[u8] = b"NGpxstJ42kfNfz4z3CsS";

/// Decrypt FSB5 sub-bank #0 and return (decrypted block bytes, parsed view).
pub fn decrypt_fsb0(bank: &[u8], key: &[u8]) -> Result<(Vec<u8>, Fsb5), String> {
    let entries = parse_bank(bank)?;
    let e = entries.first().ok_or("bank has no FSB5")?;
    let mut blk = bank
        .get(e.fsb5_offset..e.fsb5_offset + e.fsb5_size)
        .ok_or("FSB5 out of range")?
        .to_vec();
    fsb5_decrypt(&mut blk, key);
    let fsb = parse_fsb5(&blk)?;
    Ok((blk, fsb))
}

/// Decrypt + parse FSB5 sub-bank #0 (the game's single audio FSB5).
pub fn bank_fsb0(bank: &[u8], key: &[u8]) -> Result<Fsb5, String> {
    decrypt_fsb0(bank, key).map(|(_, f)| f)
}

/// Decode one Vorbis sample (by index in FSB5 #0) to a gapless 16-bit PCM WAV.
/// Preferred over [`extract_ogg`] for preview/editing: the rebuilt Ogg's intermediate
/// granule positions are approximate (some players insert silence at page boundaries),
/// whereas decoded PCM is exact and plays cleanly everywhere.
pub fn extract_wav(block: &[u8], fsb: &Fsb5, index: usize) -> Result<Vec<u8>, String> {
    let ogg = extract_ogg(block, fsb, index)?;
    let mut reader = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(ogg))
        .map_err(|e| format!("vorbis open: {e:?}"))?;
    let channels = reader.ident_hdr.audio_channels as u32;
    let rate = reader.ident_hdr.audio_sample_rate;
    let mut pcm: Vec<i16> = Vec::new();
    loop {
        match reader.read_dec_packet_itl() {
            Ok(Some(s)) => pcm.extend_from_slice(&s),
            Ok(None) => break,
            Err(e) => return Err(format!("vorbis decode: {e:?}")),
        }
    }
    Ok(wav_pcm16(rate, channels, &pcm))
}

/// Wrap interleaved 16-bit PCM in a canonical WAV container.
pub fn wav_pcm16(rate: u32, channels: u32, pcm: &[i16]) -> Vec<u8> {
    let data_len = pcm.len() * 2;
    let mut w = Vec::with_capacity(44 + data_len);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&(channels as u16).to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * channels * 2).to_le_bytes());
    w.extend_from_slice(&((channels * 2) as u16).to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &s in pcm {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

/// Extract one Vorbis sample (by index in FSB5 #0) to a playable Ogg Vorbis byte buffer.
pub fn extract_ogg(block: &[u8], fsb: &Fsb5, index: usize) -> Result<Vec<u8>, String> {
    if fsb.codec != Codec::Vorbis {
        return Err(format!("extract_ogg only supports Vorbis (codec {:?})", fsb.codec));
    }
    let s = fsb.samples.get(index).ok_or("sample index out of range")?;
    let crc = s.vorbis_crc32.ok_or("sample has no Vorbis setup CRC32")?;
    let start = (fsb.data_section + s.data_offset) as usize;
    let end = (start + s.size as usize).min(block.len());
    let audio = block.get(start..end).ok_or("sample data out of range")?;
    vorbis::fsb_vorbis_to_ogg(s.channels, s.freq, crc, s.num_samples as u64, audio)
}

/// Read a 16-bit PCM WAV file → (sample_rate, channels, interleaved samples).
pub fn read_wav_pcm16(b: &[u8]) -> Result<(u32, u32, Vec<i16>), String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let (mut fmt, mut data): (Option<(u32, u32, u16)>, Option<&[u8]>) = (None, None);
    let mut off = 12;
    while off + 8 <= b.len() {
        let id = &b[off..off + 4];
        let sz = u32_le(b, off + 4) as usize;
        let body = off + 8;
        let end = (body + sz).min(b.len());
        match id {
            b"fmt " if end - body >= 16 => {
                let format = u16::from_le_bytes([b[body], b[body + 1]]);
                let channels = u16::from_le_bytes([b[body + 2], b[body + 3]]) as u32;
                let rate = u32_le(b, body + 4);
                let bits = u16::from_le_bytes([b[body + 14], b[body + 15]]);
                if format != 1 {
                    return Err(format!("WAV not PCM (format {format})"));
                }
                fmt = Some((rate, channels, bits));
            }
            b"data" => data = Some(&b[body..end]),
            _ => {}
        }
        off = body + sz + (sz & 1); // chunks are word-aligned
    }
    let (rate, channels, bits) = fmt.ok_or("WAV missing fmt chunk")?;
    if channels == 0 {
        return Err("WAV has zero channels".into());
    }
    if bits != 16 {
        return Err(format!("WAV not 16-bit (got {bits}); convert to PCM16 first"));
    }
    let data = data.ok_or("WAV missing data chunk")?;
    let samples = data
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok((rate, channels, samples))
}

/// High-level replace: for each `(existing_sample_name, replacement)`, repoint that sample
/// to a freshly-built PCM16 FSB5 carrying the replacements. Returns the new bank bytes.
pub fn replace_samples(
    bank: &[u8],
    key: &[u8],
    replacements: Vec<(String, Pcm16Sample)>,
) -> Result<Vec<u8>, String> {
    if replacements.is_empty() {
        return Err("no replacements".into());
    }
    let f0 = bank_fsb0(bank, key)?;
    let mut repoints = Vec::with_capacity(replacements.len());
    let mut samples = Vec::with_capacity(replacements.len());
    for (i, (name, samp)) in replacements.into_iter().enumerate() {
        let idx = f0
            .samples
            .iter()
            .position(|x| x.name == name)
            .ok_or_else(|| format!("sample not found in bank: {name}"))?;
        repoints.push((idx, i as u32));
        samples.push(samp);
    }
    let new_fsb5 = build_fsb5_pcm16_multi(&samples)?;
    inject_fsb5(bank, &repoints, &new_fsb5, key)
}

#[inline]
fn i32_le(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
#[inline]
fn put_u32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
#[inline]
fn printable(cc: &[u8]) -> bool {
    cc.iter().all(|&c| c == 0x20 || (0x21..0x7f).contains(&c))
}

// ---------- build a PCM16 FSB5 ----------
/// One PCM16 sample to pack into an FSB5.
pub struct Pcm16Sample {
    pub name: String,
    pub freq: u32,
    pub channels: u32,
    pub pcm: Vec<i16>, // interleaved
}

/// Build an UNENCRYPTED FSB5 v1 holding one PCM16 sample.
pub fn build_fsb5_pcm16(name: &str, freq: u32, channels: u32, pcm: &[i16]) -> Result<Vec<u8>, String> {
    build_fsb5_pcm16_multi(&[Pcm16Sample {
        name: name.to_string(),
        freq,
        channels,
        pcm: pcm.to_vec(),
    }])
}

/// Build an UNENCRYPTED FSB5 v1 holding multiple PCM16 samples (subsound 0..n).
/// `freq` must be an FSB5 frequency-table value; `channels` ∈ {1,2,6,8}.
pub fn build_fsb5_pcm16_multi(samples: &[Pcm16Sample]) -> Result<Vec<u8>, String> {
    if samples.is_empty() {
        return Err("no samples".into());
    }
    // data section: each sample's PCM 32-byte aligned (FSB5 offsets are <<5).
    let mut data = Vec::new();
    let mut offsets = Vec::with_capacity(samples.len());
    for s in samples {
        if s.channels == 0 {
            return Err("channels must not be zero".into());
        }
        if s.pcm.len() % s.channels as usize != 0 {
            return Err("pcm length not a multiple of channels".into());
        }
        while data.len() % 32 != 0 {
            data.push(0);
        }
        offsets.push(data.len() as u64);
        for &x in &s.pcm {
            data.extend_from_slice(&x.to_le_bytes());
        }
    }

    // sample header table: one 64-bit base word per sample, no extra chunks.
    let mut shdr = Vec::with_capacity(samples.len() * 8);
    for (i, s) in samples.iter().enumerate() {
        let freq_idx = FREQ
            .iter()
            .position(|&f| f == s.freq)
            .ok_or("freq not in FSB5 table")? as u64;
        let ch_e: u64 = match s.channels {
            1 => 0,
            2 => 1,
            6 => 2,
            8 => 3,
            _ => return Err("channels must be 1/2/6/8".into()),
        };
        let frames = (s.pcm.len() / s.channels as usize) as u64;
        let mut m: u64 = 0;
        m |= (freq_idx & 0xF) << 1;
        m |= (ch_e & 0x3) << 5;
        m |= ((offsets[i] >> 5) & 0x07FF_FFFF) << 7;
        m |= (frames & 0x3FFF_FFFF) << 34;
        shdr.extend_from_slice(&m.to_le_bytes());
    }

    // name table: [u32 rel-offset]*n then NUL-terminated names; padded to 16.
    let mut name_tbl = Vec::new();
    let hdr_len = 4 * samples.len();
    let mut strings = Vec::new();
    for s in samples {
        let rel = (hdr_len + strings.len()) as u32;
        name_tbl.extend_from_slice(&rel.to_le_bytes());
        strings.extend_from_slice(s.name.as_bytes());
        strings.push(0);
    }
    name_tbl.extend_from_slice(&strings);
    while name_tbl.len() % 16 != 0 {
        name_tbl.push(0);
    }

    let mut out = Vec::with_capacity(0x3C + shdr.len() + name_tbl.len() + data.len());
    out.extend_from_slice(b"FSB5");
    out.extend_from_slice(&1u32.to_le_bytes()); // version
    out.extend_from_slice(&(samples.len() as u32).to_le_bytes()); // num_samples
    out.extend_from_slice(&(shdr.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name_tbl.len() as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes()); // codec = PCM16
    out.extend_from_slice(&0u32.to_le_bytes()); // 0x1C zero
    out.extend_from_slice(&0u32.to_le_bytes()); // 0x20 flags
    out.extend_from_slice(&[0u8; 16]); // 0x24 hash
    out.extend_from_slice(&[0u8; 8]); // 0x34 subhash
    debug_assert_eq!(out.len(), 0x3C);
    out.extend_from_slice(&shdr);
    out.extend_from_slice(&name_tbl);
    out.extend_from_slice(&data);
    Ok(out)
}

// ---------- inject a new FSB5 + repoint one WAV reference ----------
fn find_top_list(b: &[u8]) -> Result<(usize, usize), String> {
    let mut off = 0x0C;
    while off + 8 <= b.len() {
        let cc = &b[off..off + 4];
        let sz = u32_le(b, off + 4) as usize;
        if !printable(cc) {
            break;
        }
        if cc == b"LIST" {
            return Ok((off, sz));
        }
        off += 8 + sz;
    }
    Err("no top-level LIST".into())
}

/// gather WAV nodes (body_off, SoundBankIndex, SubsoundIndex) and the SNDH chunk
/// (size_field_off, body_off, size) within [start,end).
fn gather(
    b: &[u8],
    start: usize,
    end: usize,
    wavs: &mut Vec<(usize, i32, i32)>,
    sndh: &mut Option<(usize, usize, usize)>,
) {
    let mut off = start;
    while off + 8 <= end {
        let cc = &b[off..off + 4];
        let sz = u32_le(b, off + 4) as usize;
        let body = off + 8;
        if !printable(cc) {
            break;
        }
        if cc == b"WAV " && body + 0x1A <= b.len() {
            wavs.push((body, i32_le(b, body + 0x12), i32_le(b, body + 0x16)));
        } else if cc == b"SNDH" {
            *sndh = Some((off + 4, body, sz));
        }
        if cc == b"LIST" {
            let be = (body + sz).min(b.len());
            gather(b, body + 4, be, wavs, sndh);
        }
        off = body + sz;
    }
}

/// collect size-field offsets of all chunks whose body range encloses position `p`.
fn collect_enclosing(b: &[u8], start: usize, end: usize, p: usize, out: &mut Vec<usize>) {
    let mut off = start;
    while off + 8 <= end {
        let cc = &b[off..off + 4];
        let sz = u32_le(b, off + 4) as usize;
        let body = off + 8;
        if !printable(cc) {
            break;
        }
        let be = body + sz;
        if body < p && p <= be {
            out.push(off + 4);
            if cc == b"LIST" {
                collect_enclosing(b, body + 4, be.min(b.len()), p, out);
            }
        }
        off = be;
    }
}

/// emit ONE `SND ` chunk wrapping a single FSB5, 32-aligned in the file.
/// FMOD pairs the i-th SND chunk with SNDH entry i, so each FSB5 needs its own SND chunk.
/// returns (fsb5_absolute_offset_in_out, len).
fn emit_one_snd(out: &mut Vec<u8>, fsb5: &[u8]) -> (u32, u32) {
    out.extend_from_slice(b"SND ");
    let size_pos = out.len();
    out.extend_from_slice(&[0u8; 4]);
    while out.len() % 32 != 0 {
        out.push(0);
    }
    let off = out.len() as u32;
    out.extend_from_slice(fsb5);
    let body_len = (out.len() - (size_pos + 4)) as u32;
    put_u32(out, size_pos, body_len);
    (off, fsb5.len() as u32)
}

/// Increment the SNDH element count by `delta`. The SNDH body begins with an X16-packed
/// `count<<1 | flag`. Assumes the X16 stays in its 2-byte form (true for < ~16k entries).
fn bump_sndh_count(buf: &mut [u8], sndh_body: usize, delta: u32) -> Result<(), String> {
    let low = u16::from_le_bytes([buf[sndh_body], buf[sndh_body + 1]]);
    if low & 0x8000 != 0 {
        return Err("SNDH X16 count in 4-byte form; not supported".into());
    }
    let new = low as u32 + 2 * delta;
    if new >= 0x8000 {
        return Err("SNDH count would overflow 2-byte X16".into());
    }
    buf[sndh_body..sndh_body + 2].copy_from_slice(&(new as u16).to_le_bytes());
    Ok(())
}

/// Inject `new_fsb5_plain` as a 2nd FSB5 sub-bank and repoint the WAV reference of
/// `target_subsound` (in FSB5 #0) to subsound 0 of the new FSB5.
pub fn inject_pcm_sample(
    bank: &[u8],
    target_subsound: usize,
    new_fsb5_plain: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, String> {
    inject_pcm_sample_multi(bank, &[target_subsound], new_fsb5_plain, key)
}

/// Like [`inject_pcm_sample`] but repoints multiple FSB5#0 subsounds at the single new
/// FSB5 (all → subsound 0). Useful when an event randomly picks among variants.
pub fn inject_pcm_sample_multi(
    bank: &[u8],
    targets: &[usize],
    new_fsb5_plain: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, String> {
    let repoints: Vec<(usize, u32)> = targets.iter().map(|&t| (t, 0)).collect();
    inject_fsb5(bank, &repoints, new_fsb5_plain, key)
}

/// Append `new_fsb5_plain` as a 2nd FSB5 sub-bank (SoundBankIndex 1) and repoint, for each
/// `(target_subsound, new_subsound)`, the WAV reference of `target_subsound` in FSB5 #0 to
/// subsound `new_subsound` of the new FSB5. Assumes a bank with exactly one existing FSB5.
pub fn inject_fsb5(
    bank: &[u8],
    repoints: &[(usize, u32)],
    new_fsb5_plain: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, String> {
    let (list_off, list_size) = find_top_list(bank)?;
    let metadata_end = list_off + 8 + list_size;

    let entries = parse_bank(bank)?;
    if entries.len() != 1 {
        return Err(format!("expected exactly 1 FSB5, found {}", entries.len()));
    }
    if entries[0].fsb5_size == 0 {
        return Err("FSB5 size unknown (old bank version)".into());
    }

    if repoints.is_empty() {
        return Err("no repoints".into());
    }
    let mut wavs = Vec::new();
    let mut sndh = None;
    gather(bank, list_off + 8 + 4, metadata_end, &mut wavs, &mut sndh);
    let (_sndh_size_field, sndh_body, sndh_size) = sndh.ok_or("no SNDH")?;
    let mut repoint_pos = Vec::new(); // (wav_body, new_subsound)
    for &(t, new_idx) in repoints {
        // Repoint EVERY waveform node that references the target subsound, not just the first —
        // otherwise other events pointing at the same sample would still play the original.
        let before = repoint_pos.len();
        for (wav_body, sb, ss) in &wavs {
            if *sb == 0 && *ss as usize == t {
                repoint_pos.push((*wav_body, new_idx));
            }
        }
        if repoint_pos.len() == before {
            return Err(format!("no WAV node for (0,{t})"));
        }
    }

    let p = sndh_body + sndh_size; // append point = end of SNDH body

    let mut fields = Vec::new();
    collect_enclosing(bank, 0x0C, bank.len(), p, &mut fields);

    // metadata copy + edits
    let mut meta = bank[0..metadata_end].to_vec();
    for &f in &fields {
        let v = u32_le(&meta, f) + 8;
        put_u32(&mut meta, f, v);
    }
    meta.splice(p..p, [0u8; 8]);
    for &(wav_body, new_idx) in &repoint_pos {
        let wpos = if wav_body < p { wav_body } else { wav_body + 8 };
        put_u32(&mut meta, wpos + 0x12, 1); // SoundBankIndex = 1
        put_u32(&mut meta, wpos + 0x16, new_idx); // SubsoundIndex
    }

    // bump the SNDH element count (X16) so FMOD reads the 2nd entry. (Size-based readers
    // like vgmstream don't need this, but the runtime uses the embedded count.)
    bump_sndh_count(&mut meta, sndh_body, 1)?;

    // assemble: meta + SND(existing FSB5) + SND(new FSB5) — one SND chunk per FSB5,
    // because FMOD pairs SND-chunk index i with SNDH entry i.
    let mut out = meta;
    let ex = bank[entries[0].fsb5_offset..entries[0].fsb5_offset + entries[0].fsb5_size].to_vec();
    let mut enc = new_fsb5_plain.to_vec();
    fsb5_encrypt(&mut enc, key);
    let (off0, sz0) = emit_one_snd(&mut out, &ex);
    let (off1, sz1) = emit_one_snd(&mut out, &enc);

    // write SNDH entries (entry0 unchanged position, entry1 = inserted 8 bytes at p)
    put_u32(&mut out, sndh_body + 4, off0);
    put_u32(&mut out, sndh_body + 8, sz0);
    put_u32(&mut out, p, off1);
    put_u32(&mut out, p + 4, sz1);

    // RIFF size
    let riff = (out.len() - 8) as u32;
    put_u32(&mut out, 0x04, riff);
    Ok(out)
}
