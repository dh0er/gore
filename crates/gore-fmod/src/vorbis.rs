//! Rebuild standard Ogg Vorbis from FMOD's FSB5-Vorbis (raw packets + stripped headers).
//!
//! FMOD stores only the raw Vorbis audio packets (each prefixed by a u16le length) and
//! identifies the setup header (codebooks) by a CRC32. We synthesize the identification and
//! comment headers, supply the matching setup header from a small embedded table, and re-mux
//! everything into Ogg pages. No DSP — a lossless remux to a playable .ogg.
//!
//! Refs: vgmstream `coding/vorbis_custom_utils_fsb.c` (packet framing, blocksizes),
//! `coding/vorbis_custom_utils.c` (ident/comment headers); setup blobs extracted from
//! vgmstream's `vorbis_codebooks_fsb.h` (originally from python-fsb5).

// Embedded Vorbis setup headers (packet type 5) keyed by FMOD setup-id CRC32.
const SETUP_C4C30A29: &[u8] = include_bytes!("vorbis_setup/c4c30a29.bin"); // stereo
const SETUP_355295CA: &[u8] = include_bytes!("vorbis_setup/355295ca.bin"); // mono

/// The setup header (codebooks) for a given FMOD setup-id, if known.
pub fn setup_for_crc(crc: u32) -> Option<&'static [u8]> {
    match crc {
        0xc4c3_0a29 => Some(SETUP_C4C30A29),
        0x3552_95ca => Some(SETUP_355295CA),
        _ => None,
    }
}

/// Rebuild a playable Ogg Vorbis stream for one FSB5-Vorbis sample.
/// `audio` = the sample's raw bytes (the inline u16le-length-prefixed packet stream),
/// `total_samples` = decoded PCM frames (for the final granule / duration).
pub fn fsb_vorbis_to_ogg(
    channels: u32,
    rate: u32,
    setup_crc: u32,
    total_samples: u64,
    audio: &[u8],
) -> Result<Vec<u8>, String> {
    let setup = setup_for_crc(setup_crc)
        .ok_or_else(|| format!("unknown Vorbis setup CRC32 0x{setup_crc:08x} (no codebook)"))?;

    let ident = build_ident(channels, rate);
    let comment = build_comment();
    let packets = extract_packets(audio);
    if packets.is_empty() {
        return Err("no Vorbis audio packets".into());
    }

    let serial = 1u32;
    let mut out = Vec::new();
    // page 0 (BOS): identification packet
    write_page(&mut out, 0x02, 0, serial, 0, &[&ident]);
    // page 1: comment + setup
    write_page(&mut out, 0x00, 0, serial, 1, &[&comment, setup]);

    // audio pages: batch packets so each page has <= 255 lacing segments.
    let mut seq = 2u32;
    let mut batch: Vec<&[u8]> = Vec::new();
    let mut segs = 0usize;
    let mut done = 0usize;
    let total_pkts = packets.len();
    for (i, pkt) in packets.iter().enumerate() {
        let need = pkt.len() / 255 + 1;
        if need > 255 {
            return Err("Vorbis packet too large for one page".into());
        }
        if segs + need > 255 && !batch.is_empty() {
            done += batch.len();
            let granule = total_samples * done as u64 / total_pkts as u64;
            write_page(&mut out, 0x00, granule, serial, seq, &batch);
            seq += 1;
            batch.clear();
            segs = 0;
        }
        batch.push(pkt);
        segs += need;
        let _ = i;
    }
    // final page: EOS, granule = true total
    write_page(&mut out, 0x04, total_samples, serial, seq, &batch);
    Ok(out)
}

/// Vorbis identification header (packet type 1), 30 bytes.
fn build_ident(channels: u32, rate: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(30);
    h.push(0x01);
    h.extend_from_slice(b"vorbis");
    h.extend_from_slice(&0u32.to_le_bytes()); // vorbis_version
    h.push(channels as u8);
    h.extend_from_slice(&rate.to_le_bytes());
    h.extend_from_slice(&0u32.to_le_bytes()); // bitrate_max
    h.extend_from_slice(&0u32.to_le_bytes()); // bitrate_nominal
    h.extend_from_slice(&0u32.to_le_bytes()); // bitrate_min
    h.push(0xB8); // blocksize_0=2048(exp11) high nibble, blocksize_1=256(exp8) low nibble
    h.push(0x01); // framing flag
    h
}

/// Minimal Vorbis comment header (packet type 3).
fn build_comment() -> Vec<u8> {
    let vendor = b"gore-fmod";
    let mut h = Vec::new();
    h.push(0x03);
    h.extend_from_slice(b"vorbis");
    h.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    h.extend_from_slice(vendor);
    h.extend_from_slice(&0u32.to_le_bytes()); // user comment count
    h.push(0x01); // framing flag
    h
}

/// Split the FSB inline packet stream (u16le length prefixes) into raw Vorbis packets.
fn extract_packets(audio: &[u8]) -> Vec<&[u8]> {
    let mut packets = Vec::new();
    let mut off = 0usize;
    while off + 2 <= audio.len() {
        let n = u16::from_le_bytes([audio[off], audio[off + 1]]) as usize;
        off += 2;
        if n == 0 || n == 0xFFFF || off + n > audio.len() {
            break; // end / padding
        }
        packets.push(&audio[off..off + n]);
        off += n;
    }
    packets
}

// ---------- Ogg page muxing ----------
fn write_page(
    out: &mut Vec<u8>,
    header_type: u8,
    granule: u64,
    serial: u32,
    seq: u32,
    packets: &[&[u8]],
) {
    // lacing segment table
    let mut segtab = Vec::new();
    for pkt in packets {
        let mut len = pkt.len();
        loop {
            if len >= 255 {
                segtab.push(255u8);
                len -= 255;
            } else {
                segtab.push(len as u8);
                break;
            }
        }
    }
    debug_assert!(segtab.len() <= 255);

    let start = out.len();
    out.extend_from_slice(b"OggS");
    out.push(0); // version
    out.push(header_type);
    out.extend_from_slice(&granule.to_le_bytes());
    out.extend_from_slice(&serial.to_le_bytes());
    out.extend_from_slice(&seq.to_le_bytes());
    let crc_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes()); // crc placeholder
    out.push(segtab.len() as u8);
    out.extend_from_slice(&segtab);
    for pkt in packets {
        out.extend_from_slice(pkt);
    }
    let crc = ogg_crc(&out[start..]);
    out[crc_pos..crc_pos + 4].copy_from_slice(&crc.to_le_bytes());
}

/// Ogg CRC32: poly 0x04C11DB7, init 0, no input/output reflection, no final XOR.
fn ogg_crc(data: &[u8]) -> u32 {
    let mut crc: u32 = 0;
    for &b in data {
        crc ^= (b as u32) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04C1_1DB7
            } else {
                crc << 1
            };
        }
    }
    crc
}
