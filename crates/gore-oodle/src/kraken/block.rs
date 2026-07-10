//! Kraken quantum framing: parse/emit stream + quantum headers and classify each
//! quantum as compressed, stored (verbatim), or constant-fill (memset).
//!
//! # Stream layout
//!
//! A Kraken stream is a sequence of **quanta**, each producing up to
//! [`QUANTUM_LEN`] = `0x40000` (256 KiB) decompressed bytes. Every quantum is laid
//! out as:
//!
//! ```text
//! [ 2-byte stream header ] [ quantum header ] [ compressed payload ]
//! ```
//!
//! The 2-byte stream header is repeated in front of **every** quantum, not just the
//! first one (it is re-parsed at every `offset % 0x40000 == 0` boundary, and since a
//! quantum is exactly `0x40000` bytes that fires once per quantum). The first quantum's
//! header has the `keyframe`/`restart_decoder` bit set; later ones usually do not.
//!
//! This module owns the parsing/emitting of both the stream header and the quantum
//! header, plus classification of the quantum body. The top-level decode/encode loop
//! (driving `offset` across quanta) lives in the decode/encode agents — they call
//! [`parse_stream_header`] at each quantum boundary, then [`parse_quantum_header`] +
//! [`classify`].
//!
//! # Byte layout of a quantum header
//!
//! All multi-byte fields here are **big-endian** (the spine `ByteReader`/`ByteWriter`
//! expose little-endian helpers, so this module assembles big-endian bytes by hand).
//!
//! The first 3 bytes form a 24-bit big-endian value `v = (p[0] << 16) | (p[1] << 8) | p[2]`:
//!
//! * `size = v & 0x3FFFF` (low 18 bits).
//!   * If `size != 0x3FFFF` this is a **normal** quantum:
//!     * `compressed_size = size + 1` (1..=0x40000 payload bytes follow)
//!     * `flag1 = (v >> 18) & 1`, `flag2 = (v >> 19) & 1` (currently unused by Kraken;
//!       preserved for round-tripping)
//!     * if the stream's `use_checksum` flag is set, 3 more big-endian bytes follow:
//!       `checksum = (p[3] << 16) | (p[4] << 8) | p[5]` (24-bit). Header is 6 bytes.
//!     * otherwise the header is 3 bytes.
//!   * If `size == 0x3FFFF` this is a **special** quantum. Shift `v >>= 18` and look at
//!     the remaining 6 bits:
//!     * `v == 1` → **memset**: the whole quantum is a constant fill. The fill byte is
//!       `p[3]` (stored in `checksum`). `compressed_size = 0`. Header is 4 bytes
//!       (`07 FF FF <value>`, since `0x07FFFF` has the low 18 bits all set and bit 18
//!       set).
//!     * `v == 0` → **whole-match** (a back-reference covering the entire quantum). The
//!       Kraken quantum parser does **not** emit this (it only appears in the LZNA
//!       parser), so we reject it here as corrupt.
//!     * any other value → corrupt.
//!
//! The special "stored / uncompressed quantum" case is **not** a distinct header
//! encoding: a quantum is stored when its `compressed_size == out_len` (the whole
//! payload is copied verbatim). [`classify`] detects that.
//!
//! Note: there is also a **stream-level** uncompressed mode (the `uncompressed`
//! bit in the stream header), in which case there is *no* quantum header at all and the
//! raw `out_len` bytes follow the 2-byte stream header directly. That bit is surfaced as
//! [`StreamHeader::uncompressed`]; the decode loop must honor it *before* calling
//! [`parse_quantum_header`].

use crate::bytes::{ByteReader, ByteWriter};
use crate::{Error, Result};

/// Decompressed bytes produced by one full quantum (256 KiB).
pub(crate) const QUANTUM_LEN: usize = 0x40000;

/// Sentinel in the low 18 bits of the quantum-header word marking a "special" quantum.
const SIZE_SENTINEL: u32 = 0x3FFFF;

/// Largest compressed/stored payload a quantum header can encode, in bytes.
///
/// The size field is `compressed_size - 1` in 18 bits, and the all-ones value
/// (`0x3FFFF`) is reserved as the special-quantum sentinel, so the maximum encodable
/// `compressed_size` is `0x3FFFF` (one less than [`QUANTUM_LEN`]). A genuinely
/// incompressible *full* quantum is therefore never emitted with a quantum header at
/// all — the encoder falls back to the stream-level `uncompressed` bit (see
/// [`StreamHeader::uncompressed`]).
pub(crate) const MAX_QUANTUM_PAYLOAD: usize = 0x3FFFF;
/// `v >> 18` selector for a memset (constant-fill) special quantum.
const SPECIAL_MEMSET: u32 = 1;
/// `v >> 18` selector for a whole-match special quantum (LZNA only; rejected for Kraken).
const SPECIAL_WHOLE_MATCH: u32 = 0;

/// The 2-byte header that precedes every quantum.
///
/// Byte 0 low nibble must be `0xC`; byte 0 bits 4-5 must be zero. Byte 1 low 7 bits
/// select the decoder; bit 7 enables per-quantum checksums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StreamHeader {
    /// Decoder family id from byte 1 bits 0-6. Kraken proper is `6`; the related
    /// Mermaid/Selkie/Leviathan/LZNA/Bitknit ids (10/5/11/12) share this header.
    pub codec_id: u8,
    /// Byte 1 bit 7: when set, each quantum header carries a 24-bit CRC.
    pub use_checksum: bool,
    /// Byte 0 bit 6: when set, the entire block is stored verbatim and there is **no**
    /// quantum header — `out_len` raw bytes follow this header directly.
    pub uncompressed: bool,
    /// Byte 0 bit 7: "keyframe" / decoder-restart flag (set on the first quantum,
    /// resets stateful decoders such as LZNA/Bitknit).
    pub keyframe: bool,
}

/// One of the valid Kraken-family decoder ids carried in the stream header.
///
/// Kept narrow on purpose: the framing layer only needs to recognize that the id is a
/// known Kraken-family decoder so it can refuse garbage stream headers early. The decode
/// agent decides what to actually do per id.
const fn is_known_codec_id(id: u8) -> bool {
    // 6 = Kraken, 10 = Mermaid, 5 = LZNA, 11 = Bitknit, 12 = Leviathan.
    matches!(id, 5 | 6 | 10 | 11 | 12)
}

/// A parsed quantum header (the per-quantum framing word, before the payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuantumHeader {
    /// Size in bytes of the compressed payload that immediately follows this header.
    /// `0` for a memset (constant-fill) quantum, which has no payload.
    pub compressed_size: u32,
    /// The 24-bit CRC when the stream uses checksums, otherwise `0`. For a memset
    /// quantum this field instead carries the 8-bit fill value (the `checksum` field is
    /// overloaded the same way).
    pub checksum: u32,
    /// Quantum header bit 18 (`flag1`). Unused by Kraken; preserved verbatim so
    /// a parse → emit round-trip is byte-identical.
    pub flag1: bool,
    /// Quantum header bit 19 (`flag2`). Unused by Kraken; preserved verbatim.
    pub flag2: bool,
}

/// A classified quantum: how to materialize `out_len` decompressed bytes from the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quantum<'a> {
    /// A normally compressed quantum. `payload` is the `compressed_size` bytes the
    /// decoder must decompress to exactly `out_len` bytes.
    Compressed { payload: &'a [u8], out_len: usize },
    /// A verbatim quantum: `payload` is exactly `out_len` bytes, copied as-is. Emitted
    /// when compression would not shrink the quantum (`compressed_size == out_len`).
    Stored(&'a [u8]),
    /// A constant-fill quantum: write `value` `out_len` times.
    Memset { value: u8, out_len: usize },
}

/// Parse the 2-byte stream header at the cursor.
///
/// Called once per quantum (the header repeats in front of
/// every quantum). On success the reader is advanced 2 bytes.
///
/// Returns [`Error::Truncated`] if fewer than 2 bytes remain, or
/// [`Error::Corrupt`] for a malformed header (wrong magic nibble, reserved bits set, or
/// an unknown decoder id).
pub(crate) fn parse_stream_header(r: &mut ByteReader) -> Result<StreamHeader> {
    let b0 = r.u8()?;
    let b1 = r.u8()?;

    if (b0 & 0x0F) != 0x0C {
        return Err(Error::Corrupt("kraken stream header: bad magic nibble"));
    }
    if ((b0 >> 4) & 0x3) != 0 {
        return Err(Error::Corrupt("kraken stream header: reserved bits set"));
    }

    let codec_id = b1 & 0x7F;
    if !is_known_codec_id(codec_id) {
        return Err(Error::Corrupt("kraken stream header: unknown decoder id"));
    }

    Ok(StreamHeader {
        codec_id,
        use_checksum: (b1 >> 7) != 0,
        uncompressed: (b0 >> 6) & 1 != 0,
        keyframe: (b0 >> 7) & 1 != 0,
    })
}

/// Parse a quantum header at the cursor.
///
/// `out_len` is the decompressed length this quantum must produce — it equals
/// `min(remaining_output, QUANTUM_LEN)` and is needed by [`classify`] to disambiguate
/// the stored case (it is not consulted here, but is part of the contract: the same
/// `out_len` must be passed to [`classify`]). `use_checksum` comes from the stream
/// header.
///
/// On success the reader is advanced past the
/// header (3 or 6 bytes for a normal quantum, 4 bytes for a memset quantum).
///
/// Returns [`Error::Truncated`] on a short read, or [`Error::Corrupt`] for an
/// unrecognized special encoding (whole-match, which Kraken never emits, or a reserved
/// selector).
pub(crate) fn parse_quantum_header(
    r: &mut ByteReader,
    out_len: usize,
    use_checksum: bool,
) -> Result<QuantumHeader> {
    let _ = out_len; // contract documentation only; classification needs it, parsing does not.

    // First 3 bytes: a 24-bit big-endian word.
    let hi = r.take(3)?;
    let v = (u32::from(hi[0]) << 16) | (u32::from(hi[1]) << 8) | u32::from(hi[2]);
    let size = v & SIZE_SENTINEL;

    if size != SIZE_SENTINEL {
        // Normal quantum.
        let compressed_size = size + 1;
        let flag1 = (v >> 18) & 1 != 0;
        let flag2 = (v >> 19) & 1 != 0;
        let checksum = if use_checksum {
            let c = r.take(3)?;
            (u32::from(c[0]) << 16) | (u32::from(c[1]) << 8) | u32::from(c[2])
        } else {
            0
        };
        return Ok(QuantumHeader {
            compressed_size,
            checksum,
            flag1,
            flag2,
        });
    }

    // Special quantum: the selector lives in bits 18.. of the word.
    match v >> 18 {
        SPECIAL_MEMSET => {
            // memset: one trailing byte is the fill value (stored in `checksum`).
            let value = r.u8()?;
            Ok(QuantumHeader {
                compressed_size: 0,
                checksum: u32::from(value),
                flag1: false,
                flag2: false,
            })
        }
        SPECIAL_WHOLE_MATCH => {
            // Whole-match quanta are an LZNA-only encoding; the Kraken parser rejects
            // them for this selector.
            Err(Error::Corrupt(
                "kraken quantum header: whole-match not valid for Kraken",
            ))
        }
        _ => Err(Error::Corrupt("kraken quantum header: reserved selector")),
    }
}

/// Classify a parsed quantum, borrowing exactly the bytes the decoder needs.
///
/// `body` is the slice starting immediately after the quantum header; it must contain at
/// least `compressed_size` bytes for a normal/stored quantum (memset needs none).
/// `out_len` is the decompressed length this quantum produces (the same value passed to
/// [`parse_quantum_header`]).
///
/// Disambiguation:
/// * `compressed_size == 0` → [`Quantum::Memset`] (fill value = `hdr.checksum`).
/// * `compressed_size == out_len` → [`Quantum::Stored`] (verbatim copy).
/// * otherwise → [`Quantum::Compressed`].
///
/// Returns [`Error::Truncated`] if `body` is shorter than `compressed_size`, or
/// [`Error::Corrupt`] if `compressed_size > out_len` (a compressed/stored quantum can
/// never claim to expand to more than its output budget).
pub(crate) fn classify<'a>(
    hdr: &QuantumHeader,
    body: &'a [u8],
    out_len: usize,
) -> Result<Quantum<'a>> {
    let compressed_size = hdr.compressed_size as usize;

    if compressed_size == 0 {
        // Constant fill: no payload bytes are consumed.
        return Ok(Quantum::Memset {
            value: hdr.checksum as u8,
            out_len,
        });
    }

    if compressed_size > out_len {
        return Err(Error::Corrupt(
            "kraken quantum: compressed size exceeds output length",
        ));
    }

    if body.len() < compressed_size {
        return Err(Error::Truncated);
    }
    let payload = &body[..compressed_size];

    if compressed_size == out_len {
        // Verbatim: payload is the literal output.
        Ok(Quantum::Stored(payload))
    } else {
        Ok(Quantum::Compressed { payload, out_len })
    }
}

/// Write a 24-bit big-endian value.
fn push_be24(dst: &mut ByteWriter, v: u32) {
    dst.push_u8((v >> 16) as u8);
    dst.push_u8((v >> 8) as u8);
    dst.push_u8(v as u8);
}

/// Write the 2-byte stream header.
///
/// `keyframe`/`uncompressed` correspond to byte-0 bits 7/6. The decode loop emits this in
/// front of every quantum; the encode agent decides the `keyframe`/`uncompressed` flags.
pub(crate) fn write_stream_header(
    dst: &mut ByteWriter,
    codec_id: u8,
    use_checksum: bool,
    keyframe: bool,
    uncompressed: bool,
) {
    let b0 = 0x0C | (u8::from(uncompressed) << 6) | (u8::from(keyframe) << 7);
    let b1 = (codec_id & 0x7F) | (u8::from(use_checksum) << 7);
    dst.push_u8(b0);
    dst.push_u8(b1);
}

/// Write a compressed quantum header for a payload of `payload_len` bytes.
///
/// Inverse of the normal-quantum branch of [`parse_quantum_header`]. `payload_len` must
/// be in `1..=MAX_QUANTUM_PAYLOAD` (the on-wire field is `payload_len - 1` in 18 bits,
/// and the all-ones value is the special-quantum sentinel — so a full `0x40000` payload
/// is **not** representable here; that case uses the stream-level `uncompressed` bit
/// instead). The `flag1`/`flag2` quantum bits are written as zero (Kraken never sets
/// them). This writer emits the **non-checksum** 3-byte form; callers using checksums
/// append the 24-bit CRC themselves (kept out of here so this helper stays free of CRC
/// policy).
///
/// # Panics
/// Panics if `payload_len` is `0` or greater than [`MAX_QUANTUM_PAYLOAD`] — both
/// indicate an encoder bug, not malformed input.
pub(crate) fn write_compressed_header(dst: &mut ByteWriter, payload_len: usize) {
    assert!(
        payload_len >= 1 && payload_len <= MAX_QUANTUM_PAYLOAD,
        "compressed quantum payload_len out of range: {payload_len}",
    );
    // size field = payload_len - 1, low 18 bits; flag1/flag2 = 0.
    let v = (payload_len as u32) - 1;
    push_be24(dst, v);
}

/// Write a stored (verbatim) quantum header for an `out_len`-byte quantum.
///
/// A stored quantum is just a normal quantum whose `compressed_size == out_len`, so this
/// is identical to [`write_compressed_header`] with `payload_len = out_len`. The
/// `out_len` bytes of literal payload are written by the caller after this header. This
/// only applies to a *partial* final quantum (`out_len < QUANTUM_LEN`); a full
/// incompressible quantum is stored via the stream-level `uncompressed` bit, not here.
///
/// # Panics
/// Panics if `out_len` is `0` or greater than [`MAX_QUANTUM_PAYLOAD`].
pub(crate) fn write_stored_header(dst: &mut ByteWriter, out_len: usize) {
    write_compressed_header(dst, out_len);
}

/// Write a memset (constant-fill) quantum header.
///
/// Emits the 4-byte form `07 FF FF <value>`: the low 18 bits are the sentinel and bit 18
/// selects memset. No payload follows.
pub(crate) fn write_memset_header(dst: &mut ByteWriter, value: u8) {
    dst.push_u8(0x07);
    dst.push_u8(0xFF);
    dst.push_u8(0xFF);
    dst.push_u8(value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn read_vector(name: &str) -> Vec<u8> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/vectors/");
        std::fs::read(std::format!("{path}{name}")).expect("read test vector")
    }

    // --- Stream header ---------------------------------------------------------------

    #[test]
    fn parses_kraken_stream_header_no_checksum() {
        // 0x8c 0x06: nibble C, keyframe, not uncompressed, dtype 6, no crc.
        let mut r = ByteReader::new(&[0x8c, 0x06]);
        let h = parse_stream_header(&mut r).unwrap();
        assert_eq!(
            h,
            StreamHeader {
                codec_id: 6,
                use_checksum: false,
                uncompressed: false,
                keyframe: true,
            }
        );
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn parses_uncompressed_stream_header() {
        // 0xcc 0x06: nibble C, keyframe + uncompressed, dtype 6 (the `random.krk` case).
        let mut r = ByteReader::new(&[0xcc, 0x06]);
        let h = parse_stream_header(&mut r).unwrap();
        assert!(h.uncompressed);
        assert_eq!(h.codec_id, 6);
        assert!(!h.use_checksum);
    }

    #[test]
    fn parses_checksum_flag_in_stream_header() {
        // dtype 6 with bit7 of byte1 set -> use_checksum.
        let mut r = ByteReader::new(&[0x0c, 0x86]);
        let h = parse_stream_header(&mut r).unwrap();
        assert!(h.use_checksum);
        assert!(!h.keyframe);
    }

    #[test]
    fn rejects_bad_magic_nibble() {
        let mut r = ByteReader::new(&[0x8d, 0x06]); // low nibble D, not C
        assert_eq!(
            parse_stream_header(&mut r),
            Err(Error::Corrupt("kraken stream header: bad magic nibble"))
        );
    }

    #[test]
    fn rejects_reserved_bits_in_stream_header() {
        let mut r = ByteReader::new(&[0x1c, 0x06]); // bit 4 set
        assert_eq!(
            parse_stream_header(&mut r),
            Err(Error::Corrupt("kraken stream header: reserved bits set"))
        );
    }

    #[test]
    fn rejects_unknown_decoder_id() {
        let mut r = ByteReader::new(&[0x0c, 0x07]); // dtype 7 is not a known codec
        assert_eq!(
            parse_stream_header(&mut r),
            Err(Error::Corrupt("kraken stream header: unknown decoder id"))
        );
    }

    #[test]
    fn stream_header_truncated() {
        let mut r = ByteReader::new(&[0x8c]);
        assert_eq!(parse_stream_header(&mut r), Err(Error::Truncated));
    }

    // --- Quantum header: memset ------------------------------------------------------

    #[test]
    fn parses_memset_quantum_header() {
        // 07 ff ff <value>
        let mut r = ByteReader::new(&[0x07, 0xff, 0xff, 0x5a]);
        let h = parse_quantum_header(&mut r, 1, false).unwrap();
        assert_eq!(h.compressed_size, 0);
        assert_eq!(h.checksum, 0x5a);
        assert_eq!(r.remaining(), 0);

        let q = classify(&h, &[], 1).unwrap();
        assert_eq!(q, Quantum::Memset { value: 0x5a, out_len: 1 });
    }

    #[test]
    fn rejects_whole_match_quantum_for_kraken() {
        // size sentinel with selector 0 (v>>18 == 0): 00 ff ff ... but low 18 bits must
        // be all-ones while bit18 == 0. That is v == 0x03FFFF -> bytes 03 ff ff.
        let mut r = ByteReader::new(&[0x03, 0xff, 0xff, 0x00, 0x00]);
        assert_eq!(
            parse_quantum_header(&mut r, QUANTUM_LEN, false),
            Err(Error::Corrupt(
                "kraken quantum header: whole-match not valid for Kraken"
            ))
        );
    }

    #[test]
    fn rejects_reserved_special_selector() {
        // selector 2 (v>>18 == 2): v = 0x0BFFFF -> bytes 0b ff ff.
        let mut r = ByteReader::new(&[0x0b, 0xff, 0xff]);
        assert_eq!(
            parse_quantum_header(&mut r, QUANTUM_LEN, false),
            Err(Error::Corrupt("kraken quantum header: reserved selector"))
        );
    }

    // --- Quantum header: normal / round-trip -----------------------------------------

    #[test]
    fn parses_normal_quantum_header_no_checksum() {
        // 00 00 54 -> size 0x54 -> compressed_size 0x55 = 85.
        let mut r = ByteReader::new(&[0x00, 0x00, 0x54]);
        let h = parse_quantum_header(&mut r, QUANTUM_LEN, false).unwrap();
        assert_eq!(h.compressed_size, 85);
        assert_eq!(h.checksum, 0);
        assert!(!h.flag1 && !h.flag2);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn parses_normal_quantum_header_with_checksum() {
        // 00 00 54 then 24-bit checksum AA BB CC.
        let mut r = ByteReader::new(&[0x00, 0x00, 0x54, 0xAA, 0xBB, 0xCC]);
        let h = parse_quantum_header(&mut r, QUANTUM_LEN, true).unwrap();
        assert_eq!(h.compressed_size, 85);
        assert_eq!(h.checksum, 0x00AA_BBCC);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn preserves_quantum_flags() {
        // Set bit18 (flag1) and bit19 (flag2) on a size-1 payload: v = (3<<18)|0 = 0x0C0000.
        // bytes: 0c 00 00.
        let mut r = ByteReader::new(&[0x0c, 0x00, 0x00]);
        let h = parse_quantum_header(&mut r, QUANTUM_LEN, false).unwrap();
        assert_eq!(h.compressed_size, 1);
        assert!(h.flag1 && h.flag2);
    }

    #[test]
    fn write_then_parse_compressed_recovers_size() {
        // The largest encodable payload is MAX_QUANTUM_PAYLOAD (= QUANTUM_LEN - 1);
        // QUANTUM_LEN itself is not representable (its size field hits the sentinel).
        for &len in &[1usize, 2, 85, 159, 0x1234, MAX_QUANTUM_PAYLOAD - 1, MAX_QUANTUM_PAYLOAD]
        {
            let mut w = ByteWriter::new();
            write_compressed_header(&mut w, len);
            assert_eq!(w.len(), 3, "non-checksum compressed header is 3 bytes");
            let bytes = w.into_vec();
            let mut r = ByteReader::new(&bytes);
            let h = parse_quantum_header(&mut r, QUANTUM_LEN, false).unwrap();
            assert_eq!(h.compressed_size as usize, len);
            assert!(!h.flag1 && !h.flag2);
            assert_eq!(r.remaining(), 0);
        }
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn write_compressed_header_rejects_full_quantum() {
        // A full 0x40000 payload would need size field 0x3FFFF, which is the sentinel.
        let mut w = ByteWriter::new();
        write_compressed_header(&mut w, QUANTUM_LEN);
    }

    #[test]
    fn max_payload_size_field_is_one_below_sentinel() {
        // Sanity on the boundary: MAX_QUANTUM_PAYLOAD - 1 is the size field value, and it
        // is exactly SIZE_SENTINEL - 1.
        let mut w = ByteWriter::new();
        write_compressed_header(&mut w, MAX_QUANTUM_PAYLOAD);
        let bytes = w.into_vec();
        let v = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
        assert_eq!(v & SIZE_SENTINEL, SIZE_SENTINEL - 1);
    }

    #[test]
    fn write_then_parse_stored_recovers_size_and_classifies() {
        let out_len = 1234usize;
        let mut w = ByteWriter::new();
        write_stored_header(&mut w, out_len);
        let bytes = w.into_vec();
        let mut r = ByteReader::new(&bytes);
        let h = parse_quantum_header(&mut r, out_len, false).unwrap();
        assert_eq!(h.compressed_size as usize, out_len);

        // A stored quantum's body is exactly out_len verbatim bytes.
        let body: Vec<u8> = (0..out_len).map(|i| i as u8).collect();
        let q = classify(&h, &body, out_len).unwrap();
        match q {
            Quantum::Stored(p) => assert_eq!(p, &body[..]),
            other => panic!("expected Stored, got {other:?}"),
        }
    }

    #[test]
    fn write_then_parse_memset_roundtrips() {
        let mut w = ByteWriter::new();
        write_memset_header(&mut w, 0xAB);
        assert_eq!(w.len(), 4);
        let bytes = w.into_vec();
        assert_eq!(bytes, vec![0x07, 0xFF, 0xFF, 0xAB]);
        let mut r = ByteReader::new(&bytes);
        let h = parse_quantum_header(&mut r, QUANTUM_LEN, false).unwrap();
        assert_eq!(h.compressed_size, 0);
        let q = classify(&h, &[], QUANTUM_LEN).unwrap();
        assert_eq!(
            q,
            Quantum::Memset {
                value: 0xAB,
                out_len: QUANTUM_LEN
            }
        );
    }

    #[test]
    fn write_stream_header_roundtrips() {
        let mut w = ByteWriter::new();
        write_stream_header(&mut w, 6, false, true, false);
        assert_eq!(w.into_vec(), vec![0x8c, 0x06]);

        let mut w = ByteWriter::new();
        write_stream_header(&mut w, 6, true, false, true);
        let bytes = w.into_vec();
        let mut r = ByteReader::new(&bytes);
        let h = parse_stream_header(&mut r).unwrap();
        assert_eq!(
            h,
            StreamHeader {
                codec_id: 6,
                use_checksum: true,
                uncompressed: true,
                keyframe: false,
            }
        );
    }

    // --- classify edge cases ---------------------------------------------------------

    #[test]
    fn classify_compressed_borrows_only_payload() {
        let hdr = QuantumHeader {
            compressed_size: 4,
            checksum: 0,
            flag1: false,
            flag2: false,
        };
        let body = [1u8, 2, 3, 4, 5, 6]; // extra trailing bytes belong to the next quantum
        let q = classify(&hdr, &body, 100).unwrap();
        assert_eq!(
            q,
            Quantum::Compressed {
                payload: &[1, 2, 3, 4],
                out_len: 100
            }
        );
    }

    #[test]
    fn classify_rejects_compressed_size_over_out_len() {
        let hdr = QuantumHeader {
            compressed_size: 10,
            checksum: 0,
            flag1: false,
            flag2: false,
        };
        assert_eq!(
            classify(&hdr, &[0u8; 16], 8),
            Err(Error::Corrupt(
                "kraken quantum: compressed size exceeds output length"
            ))
        );
    }

    #[test]
    fn classify_truncated_body() {
        let hdr = QuantumHeader {
            compressed_size: 8,
            checksum: 0,
            flag1: false,
            flag2: false,
        };
        assert_eq!(classify(&hdr, &[0u8; 3], 100), Err(Error::Truncated));
    }

    // --- Real-vector framing tests ---------------------------------------------------

    #[test]
    fn zeros_64k_vector_is_memset_zero() {
        let data = read_vector("zeros_64k.krk");
        assert_eq!(data.len(), 6);
        let mut r = ByteReader::new(&data);
        let sh = parse_stream_header(&mut r).unwrap();
        assert_eq!(sh.codec_id, 6);
        assert!(!sh.uncompressed);
        let out_len = 65536usize.min(QUANTUM_LEN);
        let h = parse_quantum_header(&mut r, out_len, sh.use_checksum).unwrap();
        let body = &data[r.pos()..];
        let q = classify(&h, body, out_len).unwrap();
        assert_eq!(q, Quantum::Memset { value: 0, out_len });
    }

    #[test]
    fn one_byte_vector_is_memset_value() {
        let data = read_vector("one_byte.krk");
        assert_eq!(data.len(), 6);
        let mut r = ByteReader::new(&data);
        let sh = parse_stream_header(&mut r).unwrap();
        let out_len = 1usize;
        let h = parse_quantum_header(&mut r, out_len, sh.use_checksum).unwrap();
        let q = classify(&h, &data[r.pos()..], out_len).unwrap();
        assert_eq!(q, Quantum::Memset { value: 0x5a, out_len });
    }

    #[test]
    fn counter_vector_first_quantum_is_compressed() {
        let data = read_vector("counter.krk");
        let raw_len = 200_000usize;
        let mut r = ByteReader::new(&data);
        let sh = parse_stream_header(&mut r).unwrap();
        assert!(!sh.uncompressed);
        let out_len = raw_len.min(QUANTUM_LEN); // first quantum is a full 0x40000
        let h = parse_quantum_header(&mut r, out_len, sh.use_checksum).unwrap();
        assert_eq!(h.compressed_size, 85);
        // compressed_size must fit within the file after the header.
        let body = &data[r.pos()..];
        assert!(body.len() >= h.compressed_size as usize);
        let q = classify(&h, body, out_len).unwrap();
        match q {
            Quantum::Compressed { payload, out_len: ol } => {
                assert_eq!(payload.len(), 85);
                assert_eq!(ol, out_len);
            }
            other => panic!("expected Compressed, got {other:?}"),
        }
    }

    #[test]
    fn random_vector_is_stream_level_uncompressed() {
        // random.krk uses the stream-level `uncompressed` bit: no quantum header at all.
        let data = read_vector("random.krk");
        let raw_len = 50_000usize;
        let mut r = ByteReader::new(&data);
        let sh = parse_stream_header(&mut r).unwrap();
        assert!(sh.uncompressed);
        // The remaining bytes are exactly the raw payload (one quantum's worth).
        let out_len = raw_len.min(QUANTUM_LEN);
        assert_eq!(r.remaining(), out_len);
    }

    #[test]
    fn multiblock_vector_walks_every_quantum() {
        // Confirms each quantum carries its own 2-byte stream header and the framing
        // consumes the file exactly.
        let data = read_vector("multiblock.krk");
        let raw_len = 600_000usize;
        let mut r = ByteReader::new(&data);
        let mut produced = 0usize;
        let mut quanta = 0usize;
        while produced < raw_len {
            let sh = parse_stream_header(&mut r).unwrap();
            assert_eq!(sh.codec_id, 6);
            // keyframe only on the very first quantum.
            assert_eq!(sh.keyframe, quanta == 0);
            let out_len = (raw_len - produced).min(QUANTUM_LEN);
            if sh.uncompressed {
                let _ = r.take(out_len).unwrap();
            } else {
                let h = parse_quantum_header(&mut r, out_len, sh.use_checksum).unwrap();
                let body = &data[r.pos()..];
                match classify(&h, body, out_len).unwrap() {
                    Quantum::Memset { .. } => {}
                    Quantum::Stored(p) | Quantum::Compressed { payload: p, .. } => {
                        let _ = r.take(p.len()).unwrap();
                    }
                }
            }
            produced += out_len;
            quanta += 1;
        }
        assert_eq!(quanta, 3);
        assert_eq!(produced, raw_len);
        assert_eq!(r.remaining(), 0, "framing must consume the file exactly");
    }

    #[test]
    fn calibration_block_first_quantum_header_is_plausible() {
        // tests/calibration_block.b64 is a real-Oodle 4096-byte block, base64-encoded.
        // Decode only as far as the first quantum HEADER (not the payload).
        let b64 = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/calibration_block.b64"
        ))
        .unwrap();
        let data = decode_base64(b64.trim().as_bytes());
        let mut r = ByteReader::new(&data);
        let sh = parse_stream_header(&mut r).unwrap();
        assert_eq!(sh.codec_id, 6);
        assert!(!sh.uncompressed);
        let out_len = 4096usize;
        let h = parse_quantum_header(&mut r, out_len, sh.use_checksum).unwrap();
        // The compressed payload must be plausible: present and < 4096.
        assert!(h.compressed_size > 0);
        assert!(
            (h.compressed_size as usize) < 4096,
            "compressed_size {} should be < 4096",
            h.compressed_size
        );
        // And the claimed payload must actually fit in the file.
        assert!(data.len() - r.pos() >= h.compressed_size as usize);
    }

    /// Minimal standard-alphabet base64 decoder (test-only; avoids a dev-dependency).
    fn decode_base64(input: &[u8]) -> Vec<u8> {
        fn val(c: u8) -> Option<u8> {
            match c {
                b'A'..=b'Z' => Some(c - b'A'),
                b'a'..=b'z' => Some(c - b'a' + 26),
                b'0'..=b'9' => Some(c - b'0' + 52),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            }
        }
        let mut out = Vec::new();
        let mut acc = 0u32;
        let mut bits = 0u32;
        for &c in input {
            if c == b'=' {
                break;
            }
            let Some(v) = val(c) else { continue };
            acc = (acc << 6) | u32::from(v);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        out
    }
}
