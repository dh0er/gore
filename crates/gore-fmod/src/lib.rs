//! gore-fmod — FMOD Studio sound bank (`.bank`, RIFF `FEV `) decrypt + parse.
//!
//! Foundation for M0 (decrypt spike) and later extract/repack. Pure Rust, no FMOD
//! dependency. Encryption is the classic symmetric FSB5 cipher (bit-reverse + cycling
//! XOR), applied only to the embedded FSB5 sub-blocks; the FEV/RIFF metadata is plaintext.
//!
//! Refs: vgmstream `meta/fsb5.c`, `meta/fsb5_fev.c`, `meta/fsb_encrypted_streamfile.h`.

pub mod vorbis;

/// Synthetic `.bank` fixtures shared by downstream crates' tests.
///
/// Deliberately hidden behind the default-off `test-fixtures` feature: it builds banks, it never
/// reads or writes a game one, and nothing in a shipped binary should be able to reach it. This
/// crate's own `#[cfg(test)]` tests reach it too, so a plain `cargo test` -- which is what CI runs
/// -- still compiles and exercises it.
///
/// It exists because the public builders stop one layer short of a bank. [`build_fsb5_pcm16_multi`]
/// emits the inner FSB5 block, which [`parse_bank`](crate::parse_bank) rejects as "not a RIFF/FEV
/// bank", so a test that wants to exercise a reader end to end has no input at all -- and the real
/// banks are 260 MB of game data that is never vendored into this repository.
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_fixture {
    use super::{build_fsb5_pcm16_multi, fsb5_encrypt, is_pristine_bank, Pcm16Sample};

    /// How SNDH hangs off BNKI. `parse_bank` has an arm for each and they read the size field from
    /// different offsets; all ten shipped banks take the nested one, so a fixture that only emits
    /// `Direct` leaves the arm the game actually uses untested.
    #[derive(Clone, Copy)]
    pub enum Sndh {
        /// A direct sub-chunk of BNKI.
        Direct,
        /// One level down, inside a nested `LIST`, the way the shipped banks are written.
        NestedInList,
    }

    /// A minimal RIFF/`FEV ` bank whose SNDH body is exactly `body`, followed by `trailing_snd` as
    /// the payload of a top-level `SND ` chunk — empty for the sample-free banks, which have no
    /// `SND ` chunk at all. Carries only the wrapper `parse_bank` actually walks — FMT (bank
    /// version at absolute 0x14), the top-level LIST holding PROJ/BNKI, then SNDH — and backpatches
    /// the RIFF size so the whole-file length field is honest. The real game banks are never
    /// vendored here; the chunk layout is the whole of what these tests need.
    pub fn bank_with_sndh(body: &[u8], sndh: Sndh, trailing_snd: &[u8]) -> Vec<u8> {
        let u32b = |v: u32| v.to_le_bytes();
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&u32b(0)); // riff size @0x04 (backpatched)
        b.extend_from_slice(b"FEV ");
        b.extend_from_slice(b"FMT ");
        let fmt_size_pos = b.len(); // 0x10
        b.extend_from_slice(&u32b(0));
        assert_eq!(b.len(), 0x14, "FMT body must land at 0x14");
        b.extend_from_slice(&u32b(0x30)); // version 0x30 (>0x28) → 8-byte SNDH entries
        b.extend_from_slice(&u32b(0)); // filler
        let fmt_size = (b.len() - (fmt_size_pos + 4)) as u32;
        b[fmt_size_pos..fmt_size_pos + 4].copy_from_slice(&u32b(fmt_size));

        b.extend_from_slice(b"LIST");
        let list_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let list_body = b.len();
        b.extend_from_slice(b"PROJ");
        // The sub-chunk walk begins right after PROJ framing chunks as [fourcc][u32 size][body],
        // so BNKI is the first such header and needs a size of its own.
        b.extend_from_slice(b"BNKI");
        b.extend_from_slice(&u32b(0)); // empty BNKI body
        match sndh {
            Sndh::Direct => {
                b.extend_from_slice(b"SNDH");
                b.extend_from_slice(&u32b(body.len() as u32));
                b.extend_from_slice(body);
            }
            Sndh::NestedInList => {
                // [LIST][size][list type][SNDH][size][body]: the walk recognises the nested chunk
                // by the fourcc four bytes into the LIST body and takes the body twelve bytes in.
                b.extend_from_slice(b"LIST");
                b.extend_from_slice(&u32b((0x0C + body.len()) as u32));
                b.extend_from_slice(b"MODS");
                b.extend_from_slice(b"SNDH");
                b.extend_from_slice(&u32b(body.len() as u32));
                b.extend_from_slice(body);
            }
        }
        let list_size = (b.len() - list_body) as u32;
        b[list_size_pos..list_size_pos + 4].copy_from_slice(&u32b(list_size));

        // A bank that carries samples keeps them in a top-level `SND ` chunk behind the LIST, so
        // its LIST is not the last chunk in the file. A sample-free bank ends with the LIST.
        if !trailing_snd.is_empty() {
            b.extend_from_slice(b"SND ");
            b.extend_from_slice(&u32b(trailing_snd.len() as u32));
            b.extend_from_slice(trailing_snd);
        }

        let riff = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&u32b(riff));
        b
    }

    /// The sample-free shape with SNDH directly under BNKI.
    pub fn bank_with_sndh_body(body: &[u8]) -> Vec<u8> {
        bank_with_sndh(body, Sndh::Direct, &[])
    }

    /// The sample-free shape with SNDH nested in a `LIST`, as every shipped bank writes it.
    pub fn bank_with_nested_sndh_body(body: &[u8]) -> Vec<u8> {
        bank_with_sndh(body, Sndh::NestedInList, &[])
    }

    /// A bank that carries no sample data at all, written the way the shipped ones are.
    ///
    /// Six of the ten banks a Gothic 1 Remake install ships are this shape — `Master.bank`,
    /// `Master.strings.bank` and the four ~506-byte placeholders — so any listing of that directory
    /// is mostly these. Downstream tests need one because "describe a bank that has nothing in it"
    /// is a case a pristine PCM16 fixture cannot produce.
    pub fn sample_free_bank() -> Vec<u8> {
        bank_with_nested_sndh_body(&[])
    }

    /// `count` mono PCM16 samples named `{prefix}{index:02}`, one frame each.
    ///
    /// The names are what a `--filter` is tested against and the frame is what makes the bank
    /// small; a listing reads neither the audio nor its length.
    pub fn numbered_pcm16_samples(prefix: &str, count: usize, freq: u32) -> Vec<Pcm16Sample> {
        (0..count)
            .map(|index| Pcm16Sample {
                name: format!("{prefix}{index:02}"),
                freq,
                channels: 1,
                pcm: vec![0i16; 1],
            })
            .collect()
    }

    /// Wrap `samples` in a pristine RIFF/`FEV ` bank encrypted with `key`, the way the game ships
    /// one: [`is_pristine_bank`] accepts it, [`super::bank_fsb0`] lists it, and
    /// [`super::replace_samples`] takes it as a base.
    ///
    /// Only the wrapper both gore-fmod walkers actually read is emitted -- FMT (the bank version
    /// lives at absolute 0x14), the top-level LIST holding PROJ/BNKI, an SNDH entry pointing at the
    /// FSB5, and one WAV node per sample referencing (SoundBankIndex 0, SubsoundIndex i). Every
    /// sub-chunk is framed as `[fourcc][u32 size][body]` starting right after PROJ, so BNKI carries
    /// its own size too. The codec is PCM16, not the shipped banks' Vorbis, which is a feature for
    /// a listing test: `codec` in the output can only be right by reading it.
    pub fn pristine_bank_pcm16(samples: &[Pcm16Sample], key: &[u8]) -> Result<Vec<u8>, String> {
        let mut fsb5 = build_fsb5_pcm16_multi(samples)?;
        fsb5_encrypt(&mut fsb5, key);
        let u32b = |v: u32| v.to_le_bytes();

        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&u32b(0)); // riff size @0x04 (backpatched)
        b.extend_from_slice(b"FEV ");
        b.extend_from_slice(b"FMT ");
        let fmt_size_pos = b.len(); // 0x10
        b.extend_from_slice(&u32b(0));
        debug_assert_eq!(b.len(), 0x14, "FMT body must land at 0x14");
        b.extend_from_slice(&u32b(0x30)); // version 0x30 (>0x28) → 8-byte SNDH entries
        b.extend_from_slice(&u32b(0)); // filler
        let fmt_size = (b.len() - (fmt_size_pos + 4)) as u32;
        b[fmt_size_pos..fmt_size_pos + 4].copy_from_slice(&u32b(fmt_size));

        b.extend_from_slice(b"LIST");
        let list_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let list_body = b.len();
        b.extend_from_slice(b"PROJ");
        b.extend_from_slice(b"BNKI");
        b.extend_from_slice(&u32b(0)); // empty BNKI body

        // SNDH: a 4-byte chunk-version prefix (its low 2 bytes double as the injector's X16 count)
        // plus one 8-byte entry (absolute FSB5 offset, FSB5 size).
        b.extend_from_slice(b"SNDH");
        let sndh_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let sndh_body = b.len();
        b.extend_from_slice(&[2u8, 0, 0, 0]); // X16 count = 1 (1<<1)
        let sndh_entry = b.len();
        b.extend_from_slice(&u32b(0)); // entry.offset (backpatched)
        b.extend_from_slice(&u32b(0)); // entry.size   (backpatched)
        let sndh_size = (b.len() - sndh_body) as u32;
        b[sndh_size_pos..sndh_size_pos + 4].copy_from_slice(&u32b(sndh_size));

        // The waveform table: one WAV node per sample, body ≥ 0x1A, carrying SoundBankIndex
        // (i32 @+0x12) and SubsoundIndex (i32 @+0x16). Slot i reads (0, i) in every shipped bank,
        // and that identity is the reference an injection repoints. A fixture with a single node
        // would leave every sample past the first unreachable by [`super::replace_samples`] and
        // would hide a repoint from [`super::read_bank`], which is exactly the pairing under test.
        for index in 0..samples.len() {
            b.extend_from_slice(b"WAV ");
            let wav_size_pos = b.len();
            b.extend_from_slice(&u32b(0));
            let wav_body = b.len();
            b.extend_from_slice(&[0u8; 0x1A]);
            b[wav_body + 0x12..wav_body + 0x16].copy_from_slice(&u32b(0));
            b[wav_body + 0x16..wav_body + 0x1A].copy_from_slice(&u32b(index as u32));
            let wav_size = (b.len() - wav_body) as u32;
            b[wav_size_pos..wav_size_pos + 4].copy_from_slice(&u32b(wav_size));
        }

        let list_size = (b.len() - list_body) as u32;
        b[list_size_pos..list_size_pos + 4].copy_from_slice(&u32b(list_size));

        // SND chunk carrying the encrypted FSB5, 32-aligned.
        b.extend_from_slice(b"SND ");
        let snd_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let pad = (32 - (b.len() % 32)) % 32;
        b.resize(b.len() + pad, 0);
        let fsb5_abs = b.len() as u32;
        b.extend_from_slice(&fsb5);
        let snd_size = (b.len() - (snd_size_pos + 4)) as u32;
        b[snd_size_pos..snd_size_pos + 4].copy_from_slice(&u32b(snd_size));

        b[sndh_entry..sndh_entry + 4].copy_from_slice(&u32b(fsb5_abs));
        b[sndh_entry + 4..sndh_entry + 8].copy_from_slice(&u32b(fsb5.len() as u32));
        let riff = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&u32b(riff));

        // A fixture that is not pristine would send its reader down a different code path than the
        // one under test, and say nothing about why.
        if !is_pristine_bank(&b) {
            return Err("fixture bank is not pristine".into());
        }
        Ok(b)
    }
}

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
/// Whether `bank` is a usable PRISTINE bank: it parses cleanly AND holds exactly one FSB5 (i.e. it
/// has not been injected). Returns false if parsing fails — a corrupt/truncated bank is NOT treated
/// as pristine, so callers don't drop a good `*.gore-bak` or rebuild from broken bytes.
pub fn is_pristine_bank(bank: &[u8]) -> bool {
    parse_bank(bank).map(|e| e.len() == 1).unwrap_or(false)
}

/// What a bank's wrapper says about its sample data, before a single byte has been decrypted.
///
/// The split exists because "intact, and carries nothing" and "damaged" are different answers that
/// [`parse_bank`] has to collapse into one — it returns sub-banks, and a bank with none leaves its
/// caller nothing to work with either way. A listing of a whole directory does have somewhere to
/// put the distinction: six of the ten shipped banks are sample-free, and describing them as ten
/// failures would be describing the install wrongly.
enum BankShape {
    /// The wrapper points at these FSB5 sub-banks.
    Entries(Vec<BankEntry>),
    /// An intact wrapper with no sample data behind it: a placeholder, or a metadata-only bank.
    SampleFree,
}

pub fn parse_bank(b: &[u8]) -> Result<Vec<BankEntry>, String> {
    match bank_shape(b)? {
        BankShape::Entries(entries) => Ok(entries),
        // Not a new verdict: this is the message the sample-free branch has always returned, kept
        // here so `parse_bank`'s callers see exactly what they saw before the shape was split out.
        BankShape::SampleFree => Err(
            "bank carries no sample data (its SNDH chunk is empty): a placeholder or a \
             metadata-only bank such as Master.bank or *.strings.bank, not a damaged one — the \
             samples are in SFX.bank, Music.bank, VO.bank and CINEMATICS.bank"
                .into(),
        ),
    }
}

fn bank_shape(b: &[u8]) -> Result<BankShape, String> {
    bank_shape_within(b, b.len())
}

/// The shape from the wrapper alone.
///
/// `b` starts at the beginning of the file and must reach at least the end of the top-level LIST
/// chunk; everything walked below lives inside it. `file_len` is the length of the whole file,
/// which is not `b.len()` when only the wrapper was read — the FSB5 payloads sit in the `SND `
/// chunk after that LIST, and for `SFX.bank` they are 260 MB nobody reading a listing needs.
///
/// Callers holding the whole file pass `b.len()` for both, so there is one parser and not two that
/// can drift apart.
fn bank_shape_within(b: &[u8], file_len: usize) -> Result<BankShape, String> {
    debug_assert!(
        file_len >= b.len(),
        "the prefix read cannot be longer than the file it came from"
    );
    if b.len() < 0x18 || &b[0x00..0x04] != b"RIFF" || &b[0x08..0x0C] != b"FEV " {
        return Err("not a RIFF/FEV bank".into());
    }
    let version = u32_le(b, 0x14); // bank version (lives in FMT body)

    // The top-level chunk walk, shared with the one the on-disk route uses. It was written out
    // again here, which cost twice: the bound added to the shared one did not apply, so a damaged
    // 260 MB bank whose chunks all declare zero still stepped through it 8 bytes at a time; and
    // the two copies had to be kept to the same rule by hand. `TopWalk::Any` is exactly what this
    // loop did — follow every declared size, with no guard on the fourcc.
    let list_body = find_top_list_with(b.len(), TopWalk::Any, &mut |off| {
        Ok(b.get(off..off + 8).map(|header| {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(header);
            bytes
        }))
    })
    .map(|(off, _)| off + 8)
    .map_err(|_| "no top-level LIST chunk".to_string())?;

    if list_body + 8 > b.len() {
        return Err("truncated LIST chunk (no room for PROJ/BNKI)".into());
    }
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
                // "LIST" — nested; check for SNDH. This is the arm every shipped bank takes.
                if off + 8 <= end && u32_be(b, off + 4) == 0x534E_4448 {
                    // The size field is the four bytes after that fourcc, so it needs `off + 12`,
                    // not the `off + 8` that proved the fourcc. A file cut short between the two
                    // is damage; reading it anyway indexed past the buffer and panicked.
                    if off + 12 > end {
                        return Err("SNDH chunk header cut short (truncated bank)".into());
                    }
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
    // A zero-length SNDH body is how FMOD writes a bank that holds no sub-banks at all — it is not
    // damage. Six of the ten shipped banks look like this: the four 506-byte placeholders
    // (Music_NotDemo, Music_NyrasPrologue, SFX_NotDemo, SFX_NyrasPrologue) plus Master.bank (mixer
    // and buses only) and Master.strings.bank (string table only). Every chunk header in them is
    // intact down to the trailing PLAT; there is simply no `SND ` chunk and no FSB5. That absent
    // `SND ` is what makes the branch decidable: it sits after the top-level LIST in every bank
    // that has one, so in a bank that has none the LIST body runs exactly to EOF. Comparing only
    // the RIFF length would be suggestive rather than exact — zeroing these four size bytes in
    // place leaves both the file length and the RIFF field honest, and would let a bank still
    // carrying its whole FSB5 payload be told it is intact and empty. Neither subtraction can
    // underflow: `b.len() >= 0x18` from the header check, and the PROJ/BNKI check above proved
    // `list_body + 8 <= b.len()`.
    // The file's own length, not the prefix that was read: both of these compare a declared extent
    // against where the file actually ends. A wrapper-only read would otherwise satisfy them for
    // any bank whose prefix happens to end where the LIST does, and a bank still carrying its
    // whole FSB5 payload would be reported as intact and empty.
    let sample_free = sndh_size == 0
        && file_len - 8 == u32_le(b, 0x04) as usize
        && file_len - list_body == u32_le(b, list_body - 4) as usize;
    if sample_free {
        return Ok(BankShape::SampleFree);
    }
    // 1..=3 bytes cannot hold SNDH's mandatory 4-byte chunk-version prefix, so the body is torn.
    if sndh_size < 4 {
        return Err("SNDH too small (truncated or corrupt bank)".into());
    }
    let banks = (sndh_size - 4) / entry_size; // skip 4-byte chunk-version
    let mut out = Vec::with_capacity(banks);
    for i in 0..banks {
        let base = sndh_off + 4 + entry_size * i;
        // `banks` is derived from the (untrusted) SNDH size; a truncated/corrupt bank could make
        // an entry run past the buffer. Bounds-check before reading so a bad file returns a decode
        // error instead of panicking the CLI/FFI.
        if base + entry_size > b.len() {
            return Err("SNDH entry out of bounds (truncated bank)".into());
        }
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
    Ok(BankShape::Entries(out))
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

/// Nearest FSB5 enum-table slot for a rate carried by an explicit FREQUENCY chunk — a sane
/// fallback for readers that only look at the enum index.
fn nearest_freq_idx(freq: u32) -> usize {
    FREQ.iter()
        .enumerate()
        .min_by_key(|(_, &f)| (f as i64 - freq as i64).abs())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

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
                // The chunk payload must fit in the buffer, and be large enough for the fields we
                // read for this type — otherwise a corrupt/truncated bank would index past the end
                // and panic instead of returning a decode error.
                if co + 4 + csz > b.len() {
                    return Err("FSB5 chunk payload out of bounds".into());
                }
                let need = match ctyp {
                    0x01 => 1,
                    0x02 | 0x0B | 0x0E => 4,
                    _ => 0,
                };
                if csz < need {
                    return Err("FSB5 chunk too small for its type".into());
                }
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
        // Sample sizes are the gap to the next sample's offset (or to the data end for the last).
        // A corrupt/user-modified bank could have non-monotonic offsets or an offset past the data
        // section; validate the range before subtracting so it returns a decode error instead of
        // underflowing (debug panic / release huge size that reads the wrong byte range).
        let end = if i + 1 == n {
            data_size
        } else {
            raws[i + 1].data_off
        };
        if end > data_size || r.data_off > end {
            return Err("FSB5 sample offsets out of range (corrupt bank)".into());
        }
        let size = end - r.data_off;
        // Bounds-check the name-table offset array and the string offset before reading, so a
        // corrupt/truncated bank yields a synthesized name instead of an out-of-range panic.
        let name = if name_tbl_size > 0 && name_tbl + 4 * i + 4 <= b.len() {
            let start = name_tbl + u32_le(b, name_tbl + 4 * i) as usize;
            if start <= b.len() {
                read_cstr(&b[start..])
            } else {
                format!("{i:04}")
            }
        } else {
            format!("{i:04}")
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

/// Decrypt FSB5 sub-bank `index` and return (decrypted block bytes, parsed view).
pub fn decrypt_sub_bank(bank: &[u8], key: &[u8], index: usize) -> Result<(Vec<u8>, Fsb5), String> {
    let entries = parse_bank(bank)?;
    let e = entries
        .get(index)
        .ok_or_else(|| format!("bank has no FSB5 sub-bank {index}"))?;
    let mut blk = bank
        .get(e.fsb5_offset..e.fsb5_offset + e.fsb5_size)
        .ok_or("FSB5 out of range")?
        .to_vec();
    fsb5_decrypt(&mut blk, key);
    let fsb = parse_fsb5(&blk)?;
    Ok((blk, fsb))
}

/// Decrypt FSB5 sub-bank #0 and return (decrypted block bytes, parsed view).
///
/// Sub-bank 0 is the audio the bank shipped with. On a bank that [`replace_samples`] has been
/// through it is therefore NOT what the runtime plays for a replaced waveform — use [`read_bank`]
/// for that.
pub fn decrypt_fsb0(bank: &[u8], key: &[u8]) -> Result<(Vec<u8>, Fsb5), String> {
    decrypt_sub_bank(bank, key, 0)
}

/// Decrypt + parse FSB5 sub-bank #0 (the audio the bank shipped with).
pub fn bank_fsb0(bank: &[u8], key: &[u8]) -> Result<Fsb5, String> {
    decrypt_fsb0(bank, key).map(|(_, f)| f)
}

/// Bytes of an FSB5 block ahead of its per-sample header table, and therefore the most a reader
/// that only wants the block's own counts ever has to look at. [`parse_fsb5`] refuses anything
/// shorter, so both readers agree on what "the header" is.
const FSB5_HEADER_LEN: usize = 0x3C;

/// What one bank holds, read without decrypting its audio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankSummary {
    /// The bank carries sample data.
    Samples {
        /// FSB5 sub-banks in the wrapper. One is a bank as it shipped; more means
        /// [`replace_samples`] has appended to it.
        sub_banks: usize,
        /// Waveforms in sub-bank 0 — the audio the bank shipped with, which is also what
        /// [`read_bank`] lists, so the two never disagree about how many there are.
        sample_count: usize,
        /// The codec of that shipped audio.
        codec: Codec,
    },
    /// An intact bank with nothing in it to list, extract or replace: `Master.bank`,
    /// `Master.strings.bank`, or one of the four ~506-byte placeholders.
    SampleFree,
}

/// Describe a bank — how many samples, in what codec — by decrypting only its FSB5 header.
///
/// This exists because describing a directory must not cost what reading one bank costs.
/// [`read_bank`] and [`bank_fsb0`] decrypt every byte of every sub-bank, which is 247 MB for
/// `SFX.bank`'s FSB5 alone and roughly 520 MB across the ten banks a Gothic 1 Remake install
/// carries — an unreasonable price for a listing whose whole job is to hand back a path.
///
/// Reading a prefix on its own is sound because the cipher is position-indexed:
/// `plain[i] = reverse_bits(cipher[i]) ^ key[i % key.len()]`, with `i` counted from the start of
/// the FSB5 block, so byte `i` decrypts without reference to any byte after it (see
/// [`fsb5_decrypt`]). The three facts a summary needs — the magic, the sample count at 0x08 and
/// the codec at 0x18 — all live in the first [`FSB5_HEADER_LEN`] bytes, ahead of the per-sample
/// header table that makes a full parse proportional to the bank.
///
/// A sample-free bank is [`BankSummary::SampleFree`] rather than an error: it is a file the install
/// really has, and a directory listing that dropped six of ten files while claiming to describe the
/// directory would mislead worse than no listing at all.
pub fn bank_summary(bank: &[u8], key: &[u8]) -> Result<BankSummary, String> {
    summarize(
        bank,
        bank.len(),
        METADATA_MAX_BYTES,
        &mut |offset, len| {
            Ok(offset
                .checked_add(len)
                .and_then(|end| bank.get(offset..end))
                .map(<[u8]>::to_vec))
        },
        key,
    )
}

/// The whole of [`bank_summary`], over bytes it does not have to hold.
///
/// `wrapper` is the file from its start through at least the end of its top-level LIST chunk —
/// every chunk this walks lives in there. `read` fetches the small ranges at the FSB5 offsets,
/// which sit in the `SND ` chunk after that LIST. One body, so the in-memory and the on-disk route
/// cannot answer differently about the same bank.
fn summarize(
    wrapper: &[u8],
    file_len: usize,
    max: usize,
    read: &mut dyn FnMut(usize, usize) -> Result<Option<Vec<u8>>, String>,
    key: &[u8],
) -> Result<BankSummary, String> {
    let bank = wrapper;
    // Asked here so both routes answer the same. A wrapper the on-disk route refuses to read is a
    // wrapper the in-memory route must refuse to walk, or the two would disagree about the same
    // damaged file — and telling them apart by which one was used is the failure this whole
    // shared body exists to prevent.
    if let Ok((off, size)) = find_top_list(bank) {
        let declared = off.saturating_add(8).saturating_add(size).min(file_len);
        if let Some(error) = oversized_metadata(declared, max, "wrapper") {
            return Err(error);
        }
    }
    let entries = match bank_shape_within(bank, file_len)? {
        BankShape::SampleFree => return Ok(BankSummary::SampleFree),
        BankShape::Entries(entries) => entries,
    };
    // Sub-bank 0, for the same reason `read_bank` names it the bank's own audio: an injection
    // appends sub-banks and repoints into them without ever renaming or dropping a waveform, so
    // sub-bank 0 is what "how many samples does this bank have" means in every other subcommand.
    // Every sub-bank, not only the one the counts come from. An injection appends sub-banks and
    // repoints waveforms into them, so a damaged appended block is where the audio the bank
    // currently PLAYS lives — and `read_bank` decrypts all of them and would reject the file that
    // this summary called intact.
    let mut headers = Vec::with_capacity(entries.len());
    for entry in &entries {
        headers.push(sub_bank_header_from(read, file_len, max, entry, key)?);
    }
    // And where the waveforms point. Every block parsing does not mean the bank plays: a slot
    // naming a sub-bank that is not there, or a subsound past the end of the one it names, is
    // rejected by `read_bank`'s bounds checks — after this summary had already called the file
    // intact. Repointing those pairs is the whole of what an injection does, so a bank with a bad
    // one is a replacement that went wrong, which is exactly what somebody runs `banks` to find.
    let counts: Vec<usize> = headers
        .iter()
        .map(|header| u32_le(header, 0x08) as usize)
        .collect();
    // Only when `read_bank` would use them. It takes the table positionally — slot `i` for sample
    // `i` — and only if the two have the same length; any other length and it ignores every slot
    // and maps each sample to itself. Validating them regardless made `banks` call a bank
    // unreadable that `list` opens without touching the table at all, which is a false alarm and
    // the opposite of what checking against the parser is for.
    let slots = waveform_slots(bank)?;
    let positional = counts.first().is_some_and(|count| slots.len() == *count);
    for (slot_index, slot) in slots.iter().enumerate().filter(|_| positional) {
        let Some(count) = counts.get(slot.sub_bank) else {
            return Err(format!(
                "waveform {slot_index} points at sub-bank {} and the bank has {}: the bank is \
                 damaged, or a replacement was written against a different one",
                slot.sub_bank,
                counts.len()
            ));
        };
        if slot.subsound >= *count {
            return Err(format!(
                "waveform {slot_index} points at subsound {} of sub-bank {}, which holds {count}: \
                 the bank is damaged, or a replacement was written against a different one",
                slot.subsound, slot.sub_bank
            ));
        }
    }

    // Taken, not indexed. `entries.first().ok_or(..)` guarded this until the value it produced
    // stopped being used and went with it; `remove(0)` on an empty list panics, which is the one
    // thing a listing command must never do to a file it cannot read.
    let Some(header) = headers.into_iter().next() else {
        return Err("bank has no FSB5".into());
    };
    Ok(BankSummary::Samples {
        sub_banks: entries.len(),
        sample_count: u32_le(&header, 0x08) as usize,
        codec: Codec::from_u32(u32_le(&header, 0x18)),
    })
}

/// How much of a bank is read before its wrapper has been walked.
///
/// Every shipped bank puts the LIST header itself at offset 28, so this always covers finding it;
/// what it does not always cover is the LIST body, and half the shipped banks take the second read
/// below. Measured on the shipped install: `Music_*`/`SFX_*NotDemo` 506 B, `VO` 55 KB,
/// `CINEMATICS` 90 KB, `Master.strings` 142 KB, `Music` 1.0 MB, `Master` 5.1 MB, `SFX` 13.8 MB.
/// Raising this to cover those would read the whole wrapper of every bank whether or not the walk
/// needs it; two reads of exactly what is declared is the cheaper shape.
const WRAPPER_PROBE_BYTES: usize = 64 * 1024;

/// The most metadata a summary will materialize out of one bank before calling it damaged.
///
/// A bank declares two independent extents, and a corrupt size field in either turns a listing
/// back into a full read of a 260 MB file — while diagnosing a damaged bank, which is the moment
/// that cost is least wanted. Measured on the shipped install, the largest of each are the FEV
/// wrapper of `SFX.bank` at 13.8 MB and the FSB5 sample and name tables of the same bank at
/// 619 KB (7218 samples). One number covers both with room to spare rather than two to keep in
/// step.
///
/// Above this, `banks` refuses a file `audio list` may still open. That is deliberate and the
/// message says so: past this size the summary is no longer the cheap answer it exists to be, and
/// pretending otherwise means allocating what the full reader allocates without doing its work.
const METADATA_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Whether a declared extent is past what a summary reads at all. `what` names which of the two.
///
/// `max` is a parameter and not the constant so both routes can be driven past it from a test
/// without a 64 MB fixture — and driven past it TOGETHER, which is what shows they still answer
/// alike about a bank neither of them will read.
fn oversized_metadata(needed: usize, max: usize, what: &str) -> Option<String> {
    (needed > max).then(|| {
        format!(
            "the bank's {what} declares {needed} bytes and a summary reads at most {max}: the \
             size field is corrupt, or this bank is too large to describe without reading it — \
             'audio list' opens it the slow way"
        )
    })
}

/// [`bank_summary`] for a bank on disk, reading its wrapper and the FSB5 headers and nothing else.
///
/// `bank_summary` needs the caller to have the whole file in memory first, which for a directory
/// listing is roughly 520 MB of reads and a 260 MB allocation for `SFX.bank` — the cost the
/// summary exists to avoid, paid before it is called. What it actually reads is the RIFF wrapper
/// and a few dozen bytes at each FSB5 offset: about 20 MB across the ten shipped banks, most of it
/// `SFX.bank`'s own 13.8 MB of sample metadata.
pub fn bank_summary_at(path: &std::path::Path, key: &[u8]) -> Result<BankSummary, String> {
    bank_summary_probing(path, key, WRAPPER_PROBE_BYTES, METADATA_MAX_BYTES)
}

/// [`bank_summary_at`] with the first read's size in the caller's hands, so the grow-once path is
/// reachable from a test without a bank carrying tens of thousands of waveforms.
fn bank_summary_probing(
    path: &std::path::Path,
    key: &[u8],
    probe: usize,
    max: usize,
) -> Result<BankSummary, String> {
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let file_len = file
        .metadata()
        .map_err(|error| format!("{}: {error}", path.display()))?
        .len();
    let file_len = usize::try_from(file_len).map_err(|_| "bank is larger than this address space")?;

    // The wrapper, grown once if this bank's LIST reaches past the probe. Two reads at worst, and
    // in an install that ships these banks, one.
    let mut wrapper = read_at(&mut file, 0, probe.min(file_len))?;
    let needed = wrapper_extent(&wrapper, file_len, &mut |off| match off.checked_add(8) {
        Some(end) if end <= file_len => {
            let mut header = [0u8; 8];
            header.copy_from_slice(&read_at(&mut file, off, 8)?);
            Ok(Some(header))
        }
        _ => Ok(None),
    })?;
    // Before the read, not after: the point of the limit is not to allocate that much.
    if let Some(error) = oversized_metadata(needed, max, "wrapper") {
        return Err(error);
    }
    if needed > wrapper.len() {
        wrapper = read_at(&mut file, 0, needed)?;
    }

    summarize(
        &wrapper,
        file_len,
        max,
        &mut |offset, len| {
            match offset.checked_add(len) {
                Some(end) if end <= file_len => read_at(&mut file, offset, len).map(Some),
                // Past the end of the file is the bank being truncated, which is the caller's
                // sentence to write — not a read error.
                _ => Ok(None),
            }
        },
        key,
    )
}

/// How far into the file the wrapper has to reach, found without reading a chunk body.
///
/// Zero when no walk finds a LIST at all: the prefix already in hand then goes to the parser,
/// which reports the malformed wrapper itself. Reading the rest of the file to look would put the
/// whole cost back — and put it back exactly when `banks` is being run to diagnose a damaged bank,
/// which is the case that reaches here.
fn wrapper_extent(
    probe: &[u8],
    file_len: usize,
    header_at: &mut dyn FnMut(usize) -> Result<Option<[u8; 8]>, String>,
) -> Result<usize, String> {
    // Never more than the file holds. The FSB5 payloads are outside the LIST, which is what keeps
    // this small for a 260 MB bank.
    let extent = |list: Result<(usize, usize), String>| match list {
        Ok((off, size)) => off.saturating_add(8).saturating_add(size).min(file_len),
        Err(_) => 0,
    };
    let mut needed = extent(find_top_list(probe));
    if needed > 0 && needed <= probe.len() {
        return Ok(needed);
    }
    // Either no LIST inside the probe, or one that reaches past it. Both rules are walked because
    // the two readers of this wrapper do not agree on one, and a prefix cut to the shorter answer
    // would hide the LIST from the other.
    for rule in [TopWalk::Printable, TopWalk::Any] {
        needed = needed.max(extent(find_top_list_with(file_len, rule, header_at)));
    }
    Ok(needed)
}

/// Exactly `len` bytes at `offset`, or the reason there are not that many.
fn read_at(file: &mut std::fs::File, offset: usize, len: usize) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek};
    file.seek(std::io::SeekFrom::Start(offset as u64))
        .map_err(|error| format!("seeking to {offset}: {error}"))?;
    let mut buffer = vec![0u8; len];
    file.read_exact(&mut buffer)
        .map_err(|error| format!("reading {len} bytes at {offset}: {error}"))?;
    Ok(buffer)
}

/// One sub-bank's decrypted header, refused unless it is both the right shape and all there.
///
/// The magic proves the key; it does not prove the file. A block cut off after its header still
/// decrypts to a valid-looking one, and reporting a sample count and codec off it described audio
/// that is not in the file — while `audio list`, which parses the whole block, could not open the
/// same bank at all. Two commands disagreeing about one file is the failure, and the summary was
/// the one making a claim it had never checked.
///
/// The extent arithmetic is `parse_fsb5`'s, which is what would have to succeed later: the sample
/// table, the name table and the audio are all declared in the header, so their total can be
/// checked without reading any of them.
/// `read(offset, len)` yields those bytes, `Ok(None)` if they run past the end of the file, and
/// `Err` if the read itself failed — two states a listing must not confuse, since one is a damaged
/// bank and the other is a disk or a permission. A caller holding the whole bank hands over a
/// slice and takes the same path, so nothing about WHAT is checked depends on where the bytes came
/// from.
fn sub_bank_header_from(
    read: &mut dyn FnMut(usize, usize) -> Result<Option<Vec<u8>>, String>,
    file_len: usize,
    max: usize,
    entry: &BankEntry,
    key: &[u8],
) -> Result<Vec<u8>, String> {
    entry
        .fsb5_offset
        .checked_add(FSB5_HEADER_LEN)
        .ok_or("FSB5 offset out of range (corrupt bank)")?;
    let mut header = read(entry.fsb5_offset, FSB5_HEADER_LEN)?
        .ok_or("FSB5 header runs past the end of the file (truncated bank)")?;
    fsb5_decrypt(&mut header, key);
    if &header[0..4] != b"FSB5" {
        // The realistic cause by far, since every other field is read from this same block: a key
        // that is not the one the bank was encrypted with turns the header into noise, and a noisy
        // 0x08 would otherwise be printed as a sample count.
        return Err(format!(
            "decrypting the FSB5 header produced {:02x?}, not the FSB5 magic: the key is not the \
             one this bank was encrypted with, or the bank is damaged",
            &header[0..4]
        ));
    }

    let remaining = (file_len - entry.fsb5_offset) as u64;
    let present = match entry.fsb5_size {
        // Checked, not clamped. `min` quietly replaced a declared extent that runs past the end of
        // the file with the bytes that are there, so a block whose own tables fit inside the
        // remainder summarized fine — and `decrypt_sub_bank` then slices
        // `fsb5_offset..fsb5_offset + fsb5_size` and rejects the same bank as out of range.
        // Clamping is how a check turns into a guess about the value it was given.
        size if size > 0 && size as u64 > remaining => {
            return Err(format!(
                "an FSB5 block is declared {size} bytes long and only {remaining} are left \
                 in the file: the bank is truncated"
            ))
        }
        size if size > 0 => size as u64,
        // Not "the rest of the file". `parse_bank` records 0 for an old wrapper whose entries
        // carry no length — "size reconstructed from FSB5 header for old banks" — and nothing in
        // this toolkit reconstructs it: `decrypt_sub_bank` slices
        // `fsb5_offset..fsb5_offset + fsb5_size` for every sub-bank `read_bank` reads, so a zero
        // there is an empty block and `audio list` cannot open the file at all. Treating the
        // remainder as the block let `banks` summarize, and mark complete, a bank no other command
        // can touch.
        _ => {
            return Err(
                "the bank's wrapper declares no length for an FSB5 block, and this toolkit reads \
                 blocks by the length the wrapper gives: the bank cannot be opened"
                    .to_string(),
            )
        }
    };
    header_fits(&header, present)?;

    // And then the table itself. Sizes fitting is not the same as records parsing: one sample with
    // an 8-byte table and its `has_chunks` bit set satisfies every arithmetic check above and has
    // no room for the chunk header that bit promises, which is where `parse_fsb5` stops with
    // "chunk overrun". Three fixes in a row narrowed this gap by adding one more size condition;
    // the gap closes by walking the records the way the parser does, not by guessing at their
    // shape from their length.
    //
    // Only the metadata is decrypted for it — the sample table and the name table, never the
    // audio, which for `SFX.bank` is 260 MB and has nothing to say about whether the table parses.
    let base = sample_table_base(&header);
    let shdr_size = u32_le(&header, 0x0C) as usize;
    let names_size = u32_le(&header, 0x10) as usize;
    let metadata_end = base
        .saturating_add(shdr_size)
        .saturating_add(names_size)
        .min(present as usize);
    // The block's own extent is no bound at all: a corrupt `shdr_size` reaching the end of a
    // 247 MB block clamps to the block, and the listing reads all of it — the same full read the
    // wrapper limit above already had to close, one field further in. Both go through one limit.
    if let Some(error) = oversized_metadata(metadata_end, max, "sample and name tables") {
        return Err(error);
    }
    let mut metadata = read(entry.fsb5_offset, metadata_end)?
        .ok_or("FSB5 metadata runs past the end of the file (truncated bank)")?;
    fsb5_decrypt(&mut metadata, key);
    walk_sample_headers(
        &metadata,
        u32_le(&header, 0x08) as usize,
        base,
        u32_le(&header, 0x14) as u64,
    )?;

    Ok(header)
}

/// Where the sample header table starts, which `parse_fsb5` derives from the version field.
fn sample_table_base(header: &[u8]) -> usize {
    match u32_le(header, 0x04) {
        0 => 0x40,
        _ => 0x3C,
    }
}

/// Walk `n` sample header records the way [`parse_fsb5`] does, reading nothing but their shape.
///
/// Mirrors that function's pass 1: an 8-byte base word per sample, and when its `has_chunks` bit
/// is set, a chain of `(more, size, type)` words that must each fit. The values are not decoded —
/// this answers only whether the records are there to be decoded, which is exactly what a summary
/// claiming a sample count has to know and could not tell from sizes alone.
///
/// `b` is the decrypted metadata region, so the bound is the sample table and the name table
/// together — deliberately, and not the declared `shdr_size` alone.
///
/// `parse_fsb5` bounds its own walk by the whole block, name table and audio included, and so
/// accepts a chain that runs past the declared table: a bank with `shdr_size = 8`, one sample with
/// `has_chunks` set and a name table after it parses there, reading the chunk header out of the
/// name table's first four bytes. Verified by construction, not by reading. Bounding this walk at
/// `base + shdr_size` would therefore make it refuse banks `audio list` opens without complaint,
/// and this walk exists to predict that command, not to grade the format.
///
/// What remains is the other direction: a chain reaching past the metadata and into the audio is
/// accepted by the parser and refused here, because decrypting 260 MB of `SFX.bank` to follow it
/// would cost more than the answer is worth. Such a bank is corrupt by any reading, so of the two
/// ways to be wrong this is the one that misleads nobody.
fn walk_sample_headers(b: &[u8], n: usize, base: usize, data_size: u64) -> Result<(), String> {
    let mut off = base;
    let mut offsets = Vec::with_capacity(n);
    for _ in 0..n {
        if off + 8 > b.len() {
            return Err("sample header overrun".into());
        }
        let word = u64_le(b, off);
        let has_chunks = word & 1;
        offsets.push(((word >> 7) & 0x07FF_FFFF) << 5);
        let mut co = off + 8;
        if has_chunks == 1 {
            loop {
                if co + 4 > b.len() {
                    return Err("chunk overrun".into());
                }
                let w = u32_le(b, co);
                let more = w & 1;
                let csz = ((w >> 1) & 0xFF_FFFF) as usize;
                let ctyp = (w >> 25) & 0x7F;
                if co + 4 + csz > b.len() {
                    return Err("FSB5 chunk payload out of bounds".into());
                }
                // The parser reads a field out of these three and refuses a payload too small to
                // hold it. Mirroring its bounds and dropping its minima left a `0x02` frequency
                // chunk with no payload passing here and failing there — the same disagreement one
                // condition further in, which is the whole reason this walk exists.
                let need = match ctyp {
                    0x01 => 1,
                    0x02 | 0x0B | 0x0E => 4,
                    _ => 0,
                };
                if csz < need {
                    return Err("FSB5 chunk too small for its type".into());
                }
                co += 4 + csz;
                if more == 0 {
                    break;
                }
            }
        }
        off = co;
    }

    // Where each sample's audio starts, which the records also declare. `parse_fsb5` derives every
    // sample's SIZE from the gap to the next offset, so an offset that moves backwards or past the
    // declared data section makes it stop with "FSB5 sample offsets out of range" rather than
    // subtract its way to a bogus length. Reading the bit and not the field left this walk
    // accepting a table whose records parse and whose audio cannot be located.
    for (index, &start) in offsets.iter().enumerate() {
        let end = match offsets.get(index + 1) {
            Some(&next) => next,
            None => data_size,
        };
        if end > data_size || start > end {
            return Err("FSB5 sample offsets out of range (corrupt bank)".into());
        }
    }
    Ok(())
}

/// Whether a decrypted FSB5 header describes something the bytes present can actually hold.
///
/// Two questions, and the first one alone is not enough. The block's declared total has to fit in
/// what is there — otherwise the file is truncated — and the sample-header table has to be big
/// enough for the number of samples the same header claims. `parse_fsb5` reads a mandatory 8-byte
/// word per sample and gives up with "sample header overrun" when it runs out, so a header
/// declaring a thousand samples in a sixteen-byte table passes any check of the total and fails
/// the parse that would follow. `audio banks` would print that thousand, and mark its totals
/// complete, for a bank `audio list` cannot open.
fn header_fits(header: &[u8], present: u64) -> Result<(), String> {
    let base = sample_table_base(header) as u64;
    let samples = u32_le(header, 0x08) as u64;
    let shdr_size = u32_le(header, 0x0C) as u64;
    // The TABLES, not the audio. `parse_fsb5` reads the sample headers and the name table out of
    // the block, and bounds every sample offset by the declared `data_size` — it never compares
    // that size against the block it was handed. Including it here made `banks` refuse a bank
    // `list` reads and prints: the same disagreement as before with the sign reversed, and a
    // refusal the reader does not back is the worse half of it.
    //
    // Audio declared past the end of the block is caught by `extract_wav`, where those bytes are
    // actually wanted. That sentence used to be false: `extract_wav` clamped the range to the
    // block and handed back a shorter recording, reported as a success. It refuses now, so the
    // division of labour here is real — this checks what it can read, that checks what it plays.
    let declared = base
        .saturating_add(shdr_size)
        .saturating_add(u32_le(header, 0x10) as u64);
    if declared > present {
        return Err(format!(
            "an FSB5 block declares {declared} bytes of tables and only {present} are there: the \
             bank is truncated, so its sample table cannot be read"
        ));
    }

    // `parse_fsb5`'s own minimum: eight bytes of base word per sample, before any chunk.
    let needed = samples.saturating_mul(8);
    if needed > shdr_size {
        return Err(format!(
            "an FSB5 block declares {samples} sample(s) and a {shdr_size}-byte sample header \
             table, which cannot hold them: the bank is damaged, so its samples cannot be read"
        ));
    }
    Ok(())
}

/// Where one waveform's audio is taken from: a sub-bank index and a subsound within it.
///
/// In every shipped bank slot `i` reads `(0, i)` — the identity. Repointing that pair is the whole
/// of what makes a replacement audible, so it is also the whole of what a reader has to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveformSlot {
    pub sub_bank: usize,
    pub subsound: usize,
}

/// The bank's waveform table, in file order: slot `i` is the `i`-th `WAV ` node in the metadata.
///
/// This is the table [`inject_fsb5`] rewrites. Reading it is what separates "the bank still
/// contains the original audio" (always true — sub-bank 0 is never edited) from "the bank still
/// plays the original audio" (only true if nothing repointed the slot).
pub fn waveform_slots(bank: &[u8]) -> Result<Vec<WaveformSlot>, String> {
    let (list_off, list_size) = find_top_list(bank)?;
    let metadata_end = (list_off + 8 + list_size).min(bank.len());
    let mut wavs = Vec::new();
    let mut sndh = None;
    gather(bank, list_off + 8 + 4, metadata_end, &mut wavs, &mut sndh);
    wavs.iter()
        .map(|&(_, sub_bank, subsound)| {
            if sub_bank < 0 || subsound < 0 {
                return Err("negative waveform reference (corrupt bank)".to_string());
            }
            Ok(WaveformSlot {
                sub_bank: sub_bank as usize,
                subsound: subsound as usize,
            })
        })
        .collect()
}

/// One waveform as the runtime addresses it: named by the bank it shipped in, described by the
/// audio it now plays.
#[derive(Debug, Clone)]
pub struct BankSample {
    /// The name from sub-bank 0. A replacement never renames a waveform — the name is the stable
    /// identity a `--map` entry, a patch manifest and a mod bundle all key on.
    pub name: String,
    /// Where the audio comes from now.
    pub slot: WaveformSlot,
    /// Whether [`Self::slot`] has been repointed away from this waveform's own subsound in
    /// sub-bank 0, i.e. whether an injection replaced it.
    pub replaced: bool,
    pub codec: Codec,
    pub freq: u32,
    pub channels: u32,
    /// Decoded PCM frames per channel, of the audio that actually plays.
    pub num_samples: u32,
}

/// A bank read the way the runtime reads it: every sub-bank decrypted once, every waveform resolved
/// to the sub-bank it points at.
pub struct BankView {
    /// `(decrypted block, parsed header)` per FSB5 sub-bank, in SNDH order.
    pub sub_banks: Vec<(Vec<u8>, Fsb5)>,
    /// One entry per waveform in sub-bank 0, in its order.
    pub samples: Vec<BankSample>,
}

impl BankView {
    /// The codec of the audio the bank shipped with. `list` prints this as the bank's codec; a
    /// replaced waveform carries its own in [`BankSample::codec`].
    pub fn codec(&self) -> Codec {
        self.sub_banks
            .first()
            .map(|(_, f)| f.codec)
            .unwrap_or(Codec::None)
    }

    /// Decode waveform `index` to a gapless PCM16 WAV, reading from the sub-bank its slot points at
    /// rather than always from sub-bank 0.
    pub fn extract_wav(&self, index: usize) -> Result<Vec<u8>, String> {
        let sample = self.samples.get(index).ok_or("sample index out of range")?;
        let (block, fsb) = self
            .sub_banks
            .get(sample.slot.sub_bank)
            .ok_or("sub-bank out of range")?;
        crate::extract_wav(block, fsb, sample.slot.subsound)
    }
}

/// Read every sub-bank and resolve every waveform reference.
///
/// The reason this exists rather than [`bank_fsb0`]: [`replace_samples`] does not overwrite audio,
/// it appends a sub-bank and repoints the waveform at it. Sub-bank 0 therefore still holds — byte
/// for byte — the audio that was replaced, and a reader that stops there reports the replacement as
/// having done nothing, on a bank where the runtime plays the new audio.
///
/// The waveform table is matched to sub-bank 0 by position, which is what every shipped bank
/// writes (slot `i` reads `(0, i)`). A bank whose table is a different length is one this mapping
/// cannot speak for, so every waveform there is reported unreplaced rather than guessed at.
pub fn read_bank(bank: &[u8], key: &[u8]) -> Result<BankView, String> {
    let count = parse_bank(bank)?.len();
    let mut sub_banks = Vec::with_capacity(count);
    for index in 0..count {
        sub_banks.push(decrypt_sub_bank(bank, key, index)?);
    }
    let base = &sub_banks.first().ok_or("bank has no FSB5")?.1;
    let slots = waveform_slots(bank)?;
    let positional = slots.len() == base.samples.len();

    let mut samples = Vec::with_capacity(base.samples.len());
    for (index, shipped) in base.samples.iter().enumerate() {
        let identity = WaveformSlot {
            sub_bank: 0,
            subsound: index,
        };
        let slot = if positional { slots[index] } else { identity };
        let (_, fsb) = sub_banks.get(slot.sub_bank).ok_or_else(|| {
            format!(
                "waveform {index} ({}) points at sub-bank {}, which the bank does not carry",
                shipped.name, slot.sub_bank
            )
        })?;
        let source = fsb.samples.get(slot.subsound).ok_or_else(|| {
            format!(
                "waveform {index} ({}) points at subsound {} of sub-bank {}, which holds {}",
                shipped.name,
                slot.subsound,
                slot.sub_bank,
                fsb.samples.len()
            )
        })?;
        samples.push(BankSample {
            name: shipped.name.clone(),
            slot,
            replaced: slot != identity,
            codec: fsb.codec,
            freq: source.freq,
            channels: source.channels,
            num_samples: source.num_samples,
        });
    }
    Ok(BankView {
        sub_banks,
        samples,
    })
}

/// Decode one Vorbis sample (by index in FSB5 #0) to a gapless 16-bit PCM WAV.
/// Preferred over [`extract_ogg`] for preview/editing: the rebuilt Ogg's intermediate
/// granule positions are approximate (some players insert silence at page boundaries),
/// whereas decoded PCM is exact and plays cleanly everywhere.
pub fn extract_wav(block: &[u8], fsb: &Fsb5, index: usize) -> Result<Vec<u8>, String> {
    // A replaced sample is PCM16, because that is what `replace_samples` appends — and the whole
    // point of reading the bank the game plays is being able to read your own replacement back. A
    // Vorbis-only path here made `audio extract` skip exactly the sample the user had just written,
    // and report success having produced no file at all.
    if fsb.codec == Codec::Pcm16 {
        let s = fsb.samples.get(index).ok_or("sample index out of range")?;
        let start = (fsb.data_section + s.data_offset) as usize;
        // `size` spans to the next sample's offset, and FSB5 offsets are 32-byte aligned — so for
        // any sample whose PCM does not land on that boundary it runs past the audio into as much
        // as 30 bytes of padding. Emitting those appends silence the replacement never had, which
        // means `extract` does not reproduce what `replace` wrote. The frame count is what says
        // where the audio actually ends; `size` only says where the next one begins.
        let declared = (s.num_samples as usize)
            .saturating_mul(s.channels as usize)
            .saturating_mul(2);
        let mut len = s.size as usize;
        // Only ever narrows. A bank whose header understates its own frame count would otherwise
        // lose real audio here, and `size` remains the outer bound in either direction.
        if declared > 0 {
            len = len.min(declared);
        }
        // Refused, not clamped. `min(block.len())` turned a block cut short into a shorter WAV
        // that `extract` reported as a success — the recording came out quietly missing its tail,
        // which is worse than an error, because nothing about it looks wrong until somebody plays
        // it. The summary's own comment claimed this arm caught that, and it did the opposite.
        let end = start.checked_add(len).ok_or("sample extent out of range (corrupt bank)")?;
        let audio = block.get(start..end).ok_or_else(|| {
            format!(
                "sample '{}' runs to {end} and the block holds {}: the bank is truncated",
                s.name,
                block.len()
            )
        })?;
        // Truncated rather than refused: an odd length means one dangling byte, and dropping it
        // costs half a sample of the tail where erroring would cost the whole recording.
        let pcm: Vec<i16> = audio
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return Ok(wav_pcm16(s.freq, s.channels, &pcm));
    }
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
        return Err(format!(
            "extract_ogg only supports Vorbis (codec {:?})",
            fsb.codec
        ));
    }
    let s = fsb.samples.get(index).ok_or("sample index out of range")?;
    let crc = s.vorbis_crc32.ok_or("sample has no Vorbis setup CRC32")?;
    let start = (fsb.data_section + s.data_offset) as usize;
    // Refused, not clamped — the same fix the PCM arm just took, and this is the arm that matters
    // for the shipped banks, which are all Vorbis. Clamping handed `extract_packets` a complete
    // packet prefix, which it accepts and remuxes with an EOS page: a playable Ogg, quietly
    // missing its tail, reported as a success.
    let end = start
        .checked_add(s.size as usize)
        .ok_or("sample extent out of range (corrupt bank)")?;
    let audio = block.get(start..end).ok_or_else(|| {
        format!(
            "sample '{}' runs to {end} and the block holds {}: the bank is truncated",
            s.name,
            block.len()
        )
    })?;
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
        return Err(format!(
            "WAV not 16-bit (got {bits}); convert to PCM16 first"
        ));
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
    // The injection appends a 2nd FSB5 and repoints into it, so it requires a pristine
    // single-FSB5 bank. If `bank` is already injected (>1 FSB5) — e.g. a re-deploy that read the
    // live, already-modded bank because its `*.gore-bak` pristine backup is missing — fail with
    // an actionable message instead of the cryptic "expected exactly 1 FSB5" from inject_fsb5.
    let blocks = parse_bank(bank)?;
    if blocks.len() != 1 {
        return Err(format!(
            "bank already contains modded audio ({} FSB5 blocks); restore the original bank \
             (its *.gore-bak backup, or Steam → Verify integrity of game files) before replacing",
            blocks.len()
        ));
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
pub fn build_fsb5_pcm16(
    name: &str,
    freq: u32,
    channels: u32,
    pcm: &[i16],
) -> Result<Vec<u8>, String> {
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

    // sample header table: a 64-bit base word per sample, optionally followed by a FREQUENCY
    // chunk for rates outside FSB5's fixed enum table.
    let mut shdr = Vec::with_capacity(samples.len() * 8);
    for (i, s) in samples.iter().enumerate() {
        if s.freq == 0 {
            return Err("sample rate must not be zero".into());
        }
        let ch_e: u64 = match s.channels {
            1 => 0,
            2 => 1,
            6 => 2,
            8 => 3,
            _ => return Err("channels must be 1/2/6/8".into()),
        };
        let frames = (s.pcm.len() / s.channels as usize) as u64;
        // Use the exact enum slot when the rate is in the table; otherwise emit an explicit
        // FREQUENCY chunk (type 0x02) carrying the real rate — the same mechanism the game's own
        // banks use (see parse_fsb5), so e.g. 88200 Hz is supported instead of rejected. The
        // freq_idx field then holds the nearest table slot as a fallback for enum-only readers.
        let table_idx = FREQ.iter().position(|&f| f == s.freq);
        let use_chunk = table_idx.is_none();
        let freq_idx = table_idx.unwrap_or_else(|| nearest_freq_idx(s.freq)) as u64;
        let mut m: u64 = 0;
        m |= use_chunk as u64; // bit 0: more chunks follow the base word
        m |= (freq_idx & 0xF) << 1;
        m |= (ch_e & 0x3) << 5;
        m |= ((offsets[i] >> 5) & 0x07FF_FFFF) << 7;
        m |= (frames & 0x3FFF_FFFF) << 34;
        shdr.extend_from_slice(&m.to_le_bytes());
        if use_chunk {
            // chunk header: next=0, size=4, type=0x02 (FREQUENCY); then the u32 rate.
            let w: u32 = (4u32 << 1) | (0x02u32 << 25);
            shdr.extend_from_slice(&w.to_le_bytes());
            shdr.extend_from_slice(&s.freq.to_le_bytes());
        }
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
    find_top_list_with(b.len(), TopWalk::Printable, &mut |off| {
        Ok(b.get(off..off + 8).map(|header| {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(header);
            bytes
        }))
    })
}

/// How many top-level chunk headers a walk will look at before giving up.
///
/// Every shipped bank carries its LIST as the second chunk, at offset 28. This is far past that on
/// purpose: it is not a limit on real banks, it is a limit on a damaged one whose chunk sizes send
/// the walk stepping through the whole file.
const MAX_TOP_CHUNK_PROBES: usize = 4096;

/// The two rules by which this file walks the top-level chunks for the LIST.
///
/// They are not the same rule, and that is not an oversight worth quietly unifying: [`bank_shape`]
/// walks every chunk it is given, while [`find_top_list`] stops at the first fourcc that is not
/// printable ASCII rather than follow a size read out of noise. A wrapper read from disk has to
/// cover whatever EITHER of them would reach in memory, or a bank would summarize differently
/// depending on how it was read.
#[derive(Clone, Copy)]
enum TopWalk {
    /// [`find_top_list`]'s: stop at a fourcc that cannot be one.
    Printable,
    /// [`bank_shape`]'s: follow every declared size.
    Any,
}

/// The same walk over chunk headers alone.
///
/// Top-level chunks are 8 bytes of header and a body this skips over, so finding the LIST costs
/// one header read per chunk and never touches a body. `header_at` yields the eight bytes at an
/// offset, or `None` where the file ends before them. Shared with the slice version so a bank read
/// from disk and the same bank in memory cannot disagree about where its metadata is.
fn find_top_list_with(
    end: usize,
    rule: TopWalk,
    header_at: &mut dyn FnMut(usize) -> Result<Option<[u8; 8]>, String>,
) -> Result<(usize, usize), String> {
    let mut off = 0x0C;
    let mut probes = 0usize;
    while off + 8 <= end {
        // Chunks are followed by their declared size, and a declared size of zero advances by the
        // 8-byte header alone. A damaged 260 MB bank whose top-level chunks all declare zero
        // therefore walks it in 8-byte steps — 32 million of them, each a seek and a read on the
        // on-disk route, which is not a bounded read but is very much a hang. The shipped banks
        // put their LIST second, so this bound is three orders of magnitude past any real one, and
        // reaching it means the same thing the walk running out means: there is no wrapper here.
        probes += 1;
        if probes > MAX_TOP_CHUNK_PROBES {
            break;
        }
        let Some(header) = header_at(off)? else { break };
        let sz = u32_le(&header, 4) as usize;
        if matches!(rule, TopWalk::Printable) && !printable(&header[0..4]) {
            break;
        }
        if &header[0..4] == b"LIST" {
            return Ok((off, sz));
        }
        // A corrupt size that overflows the offset ends the walk instead of wrapping it back into
        // the file. The slice version could not overflow: `end` was a buffer it had already read.
        match off.checked_add(8).and_then(|off| off.checked_add(sz)) {
            Some(next) => off = next,
            None => break,
        }
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

#[cfg(test)]
mod truncation_tests {
    use super::{bank_summary, BankSummary, GOTHIC_STUDIO_KEY};
    use crate::test_fixture::{numbered_pcm16_samples, pristine_bank_pcm16};

    /// A synthetic 0x3C header, so the arithmetic can be exercised without building and
    /// re-encrypting a whole bank around it.
    fn header(samples: u32, shdr: u32, names: u32, data: u32) -> Vec<u8> {
        let mut h = vec![0u8; 0x3C];
        h[0x00..0x04].copy_from_slice(b"FSB5");
        h[0x04..0x08].copy_from_slice(&1u32.to_le_bytes());
        h[0x08..0x0C].copy_from_slice(&samples.to_le_bytes());
        h[0x0C..0x10].copy_from_slice(&shdr.to_le_bytes());
        h[0x10..0x14].copy_from_slice(&names.to_le_bytes());
        h[0x14..0x18].copy_from_slice(&data.to_le_bytes());
        h
    }

    #[test]
    fn a_record_promising_chunks_it_has_no_room_for_is_refused() {
        use super::walk_sample_headers;

        // One sample, eight bytes of table, and the base word's `has_chunks` bit set. Every size
        // condition is satisfied — 1 * 8 == 8 — and the chunk header that bit promises is not
        // there. `parse_fsb5` stops at exactly this point with "chunk overrun", so a summary that
        // only measured sizes reported a sample `audio list` cannot read.
        let mut table = vec![0u8; 0x3C];
        table.extend_from_slice(&1u64.to_le_bytes()); // has_chunks = 1, nothing after it
        let error =
            walk_sample_headers(&table, 1, 0x3C, u64::MAX).expect_err("the chunk header is missing");
        assert_eq!(error, "chunk overrun");

        // The same record with room for its chunk walks fine: one 4-byte word, `more` clear.
        let mut ok_table = vec![0u8; 0x3C];
        ok_table.extend_from_slice(&1u64.to_le_bytes());
        ok_table.extend_from_slice(&0u32.to_le_bytes());
        assert!(walk_sample_headers(&ok_table, 1, 0x3C, u64::MAX).is_ok());

        // A chunk whose declared payload runs past the metadata is corruption, not a long chunk.
        let mut overrun = vec![0u8; 0x3C];
        overrun.extend_from_slice(&1u64.to_le_bytes());
        overrun.extend_from_slice(&(0xFFu32 << 1).to_le_bytes()); // 255-byte payload, none present
        assert_eq!(
            walk_sample_headers(&overrun, 1, 0x3C, u64::MAX).expect_err("the payload is not there"),
            "FSB5 chunk payload out of bounds"
        );

        // And a record without chunks needs only its base word.
        let mut plain = vec![0u8; 0x3C];
        plain.extend_from_slice(&0u64.to_le_bytes());
        assert!(walk_sample_headers(&plain, 1, 0x3C, u64::MAX).is_ok());
    }

    #[test]
    fn a_sample_table_too_small_for_its_own_count_is_refused() {
        use super::header_fits;

        // Four samples, thirty-two bytes of table: the mandatory eight bytes each, and it fits.
        let good = header(4, 32, 0, 64);
        assert!(header_fits(&good, 0x3C + 32 + 64).is_ok());

        // The same total, and a table that cannot hold the count the header states. `parse_fsb5`
        // gives up here with "sample header overrun", so `banks` printed a thousand samples, and
        // called its totals complete, for a bank `audio list` cannot open.
        let lying = header(1000, 16, 0, 80);
        let error = header_fits(&lying, 0x3C + 16 + 80).expect_err("1000 samples do not fit in 16 bytes");
        assert!(error.contains("cannot hold them"), "{error}");
        assert!(error.contains("1000"), "{error}");

        // And the truncation check still fires on its own: 0x3C of header plus an 8-byte table is
        // 68 bytes, and 64 are there.
        let truncated = header(1, 8, 0, 4096);
        let error = header_fits(&truncated, 64).expect_err("the table itself is not all there");
        assert!(error.contains("truncated"), "{error}");

        // Audio declared past the end of the block is NOT this check's business: `parse_fsb5`
        // bounds sample offsets by that declared size and never compares it with the block, so
        // refusing here made `banks` reject a bank `list` reads and prints.
        assert!(
            header_fits(&truncated, 68).is_ok(),
            "the tables fit; the audio is `extract_wav`'s to complain about"
        );
    }

    #[test]
    fn a_wrapper_declaring_no_length_is_refused_by_both_readers() {
        // The property rather than the message: whatever `bank_summary` says about a bank,
        // `read_bank` has to be able to say the same. A zero extent is where they had drifted
        // apart — `banks` reconstructed a block from the rest of the file, `list` sliced an empty
        // one and gave up.
        let samples = numbered_pcm16_samples("SFX_UI_Click_", 3, 44_100);
        let mut bank = pristine_bank_pcm16(&samples, GOTHIC_STUDIO_KEY).unwrap();
        assert!(bank_summary(&bank, GOTHIC_STUDIO_KEY).is_ok());
        assert!(super::read_bank(&bank, GOTHIC_STUDIO_KEY).is_ok());

        let sndh = bank
            .windows(4)
            .position(|w| w == b"SNDH")
            .expect("the fixture writes an SNDH chunk");
        let size_field = sndh + 4 + 4 + 4 + 4;
        bank[size_field..size_field + 4].copy_from_slice(&0u32.to_le_bytes());

        assert!(
            bank_summary(&bank, GOTHIC_STUDIO_KEY).is_err(),
            "the summary must refuse what the reader refuses"
        );
        assert!(super::read_bank(&bank, GOTHIC_STUDIO_KEY).is_err());
    }

    #[test]
    fn a_wrapper_extent_past_the_end_of_the_file_is_refused() {
        // The SNDH entry says how long the embedded FSB5 is. Taking the smaller of that and what
        // is left in the file made a block whose own tables fit inside the remainder summarize
        // fine, while `decrypt_sub_bank` slices `fsb5_offset..fsb5_offset + fsb5_size` and rejects
        // the same bank.
        let samples = numbered_pcm16_samples("SFX_UI_Click_", 3, 44_100);
        let mut bank = pristine_bank_pcm16(&samples, GOTHIC_STUDIO_KEY).unwrap();
        assert!(bank_summary(&bank, GOTHIC_STUDIO_KEY).is_ok());

        // The entry is the 8 bytes after SNDH's size field and its 4-byte chunk-version prefix:
        // absolute offset, then size. Only the size is touched, so the block still starts where it
        // did and only its declared length becomes a lie.
        let sndh = bank
            .windows(4)
            .position(|w| w == b"SNDH")
            .expect("the fixture writes an SNDH chunk");
        let size_field = sndh + 4 + 4 + 4 + 4;
        bank[size_field..size_field + 4].copy_from_slice(&u32::MAX.to_le_bytes());

        let error = bank_summary(&bank, GOTHIC_STUDIO_KEY)
            .expect_err("a block longer than the file must not summarize");
        assert!(error.contains("are left in the file"), "{error}");
    }

    #[test]
    fn a_bank_cut_off_after_its_header_is_not_summarized_as_intact() {
        let samples = numbered_pcm16_samples("SFX_UI_Click_", 4, 44_100);
        let bank = pristine_bank_pcm16(&samples, GOTHIC_STUDIO_KEY).unwrap();

        // Whole, it summarizes.
        let whole = bank_summary(&bank, GOTHIC_STUDIO_KEY).expect("an intact bank summarizes");
        assert!(matches!(whole, BankSummary::Samples { sample_count: 4, .. }), "{whole:?}");

        // Cut short. The magic still decrypts — it lives in the first 60 bytes — so the check that
        // used to run passed, and `banks` reported this file's count and codec as fact while
        // `audio list` could not open it at all.
        let cut = &bank[..bank.len() - 64];
        let error = bank_summary(cut, GOTHIC_STUDIO_KEY)
            .expect_err("a truncated bank must not summarize as intact");
        // The wrapper extent is what catches this now: the SNDH entry still declares the block's
        // full length while the file is 64 bytes shorter, which is an earlier and more precise
        // reading of the same damage than the header's own totals. `header_fits` keeps its own
        // coverage in `a_sample_table_too_small_for_its_own_count_is_refused`.
        assert!(
            error.contains("are left in the file"),
            "the wrapper extent is what must have refused this: {error}"
        );
        assert!(error.contains("truncated"), "{error}");
    }
}

#[cfg(test)]
mod tests {
    use super::test_fixture::{
        bank_with_nested_sndh_body, bank_with_sndh, bank_with_sndh_body, Sndh,
    };
    use super::*;

    #[test]
    fn build_fsb5_table_rate_roundtrips() {
        // 44100 is in the enum table → no chunk; parse must read it back exactly.
        let fsb = build_fsb5_pcm16("s", 44100, 1, &[0i16; 64]).unwrap();
        let parsed = parse_fsb5(&fsb).unwrap();
        assert_eq!(parsed.samples[0].freq, 44100);
        assert_eq!(parsed.samples[0].channels, 1);
    }

    #[test]
    fn build_fsb5_arbitrary_rate_roundtrips() {
        // 88200 is NOT in the table → must be carried via a FREQUENCY chunk and read back exactly,
        // and stereo channels must survive too.
        let fsb = build_fsb5_pcm16("hi", 88200, 2, &[0i16; 128]).unwrap();
        let parsed = parse_fsb5(&fsb).unwrap();
        assert_eq!(parsed.samples[0].freq, 88200);
        assert_eq!(parsed.samples[0].channels, 2);
    }

    #[test]
    fn a_pcm16_sample_reads_back_as_the_wav_it_was_built_from() {
        // What `replace_samples` appends is PCM16, so this is the codec of every replacement — and
        // `extract_wav` used to hand the whole job to a Vorbis-only path. `audio extract` therefore
        // skipped precisely the sample the user had just written, and reported success having
        // produced nothing: the one check that tells someone their replacement actually landed.
        let rate = 22050;
        let pcm: Vec<i16> = (0..512).map(|n| (n as i16).wrapping_mul(311)).collect();
        let fsb_bytes = build_fsb5_pcm16("replaced", rate, 1, &pcm).unwrap();
        let fsb = parse_fsb5(&fsb_bytes).unwrap();
        assert_eq!(fsb.codec, Codec::Pcm16, "the fixture has to be the codec under test");

        let wav = extract_wav(&fsb_bytes, &fsb, 0).expect("a PCM16 sample must extract");
        assert_eq!(wav, wav_pcm16(rate, 1, &pcm), "the samples must survive the round trip");

        // The header has to describe the audio, not the defaults: a replacement recorded at another
        // rate would otherwise play back at the wrong speed in whatever the user opens it with.
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), rate);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 1);
    }

    #[test]
    fn a_padded_sample_extracts_its_audio_and_not_the_padding_after_it() {
        // FSB5 offsets are 32-byte aligned, so the builder pads between samples and a sample's
        // `size` — the gap to the next offset — overshoots its audio by up to 30 bytes. Reading to
        // `size` appended that padding as silence, which meant `extract` did not reproduce what
        // `replace` had written. 100 frames is 200 bytes, which is 8 short of the boundary.
        let first: Vec<i16> = (0..100).map(|n| (n as i16).wrapping_mul(37).wrapping_add(1)).collect();
        let second: Vec<i16> = (0..64).map(|n| (n as i16).wrapping_mul(-11)).collect();
        assert_ne!(first.len() * 2 % 32, 0, "the fixture has to be the unaligned case");

        let bytes = build_fsb5_pcm16_multi(&[
            Pcm16Sample { name: "first".into(), freq: 44_100, channels: 1, pcm: first.clone() },
            Pcm16Sample { name: "second".into(), freq: 44_100, channels: 1, pcm: second.clone() },
        ])
        .unwrap();
        let fsb = parse_fsb5(&bytes).unwrap();

        assert_eq!(
            extract_wav(&bytes, &fsb, 0).unwrap(),
            wav_pcm16(44_100, 1, &first),
            "the padding between this sample and the next is not audio"
        );
        // The last sample has nothing after it to pad against, which is why the bug only ever
        // showed on the earlier ones — and why a single-sample fixture would have missed it.
        assert_eq!(extract_wav(&bytes, &fsb, 1).unwrap(), wav_pcm16(44_100, 1, &second));
    }

    #[test]
    fn parse_fsb5_truncated_chunk_errors_not_panics() {
        // A bank whose sample-header FREQUENCY chunk payload is cut off must return a decode error
        // rather than indexing past the buffer and panicking. The 88200 sample has a 4-byte freq
        // chunk at bytes 0x44..0x4C; truncating to 0x4A leaves the chunk header but not its payload.
        let fsb = build_fsb5_pcm16("hi", 88200, 2, &[0i16; 128]).unwrap();
        assert!(fsb.len() > 0x4A);
        assert!(parse_fsb5(&fsb[..0x4A]).is_err());
    }

    /// Offset of the four bytes `parse_bank` reads as the SNDH size, i.e. just past the fourcc.
    fn sndh_size_field(b: &[u8]) -> usize {
        b.windows(4)
            .position(|w| w == b"SNDH")
            .expect("the fixture writes exactly one SNDH fourcc")
            + 4
    }

    #[test]
    fn parse_bank_reports_an_empty_sndh_as_a_sample_free_bank_not_a_broken_one() {
        // Six of the ten shipped banks are shaped like this — the four 506-byte placeholders plus
        // Master.bank (mixer only) and Master.strings.bank (string table only). Calling them
        // "SNDH too small" sent people looking for a corrupt file that was never corrupt.
        let err = parse_bank(&bank_with_sndh_body(&[])).unwrap_err();
        assert!(
            err.contains("no sample data"),
            "an intact bank that simply has no samples must say so, got {err:?}"
        );
        assert!(
            !err.contains("too small"),
            "a bank that is not damaged must not be described as damaged, got {err:?}"
        );
    }

    #[test]
    fn parse_bank_still_reports_damage_when_the_sndh_body_is_a_partial_version_prefix() {
        // 1..=3 bytes cannot hold SNDH's mandatory 4-byte chunk-version prefix, so this file is
        // torn, not empty. Widening the friendly branch to the whole `< 4` guard would swallow it.
        let err = parse_bank(&bank_with_sndh_body(&[0, 0, 0])).unwrap_err();
        assert!(
            err.contains("SNDH too small"),
            "a torn SNDH body must still be reported as damage, got {err:?}"
        );
    }

    #[test]
    fn parse_bank_calls_an_empty_sndh_damaged_when_the_riff_length_disagrees_with_the_file_size() {
        // What the RIFF-length corroboration buys: a file whose byte count disagrees with its own
        // RIFF header must not pass itself off as a placeholder just because SNDH reads zero.
        let mut b = bank_with_sndh_body(&[]);
        b.push(0); // RIFF now claims the file ends one byte earlier than it does
        let err = parse_bank(&b).unwrap_err();
        assert!(
            err.contains("SNDH too small"),
            "a bank whose RIFF length disagrees with its size must be called damaged, got {err:?}"
        );
    }

    #[test]
    fn parse_bank_does_not_call_a_bank_sample_free_when_its_sndh_size_was_zeroed_in_place() {
        // The case the RIFF length alone cannot see, and the reason the LIST must reach EOF too.
        // Zeroing those four bytes in a real `VO.bank` — 652,800 bytes, 598 KB of `SND ` payload
        // it can no longer find — changes neither the file length nor the RIFF field, so a
        // corroboration built only on those two tells a badly damaged bank it is "not a damaged
        // one". Saying nothing is wrong is worse than the "SNDH too small" this branch replaced.
        let mut body = vec![2u8, 0, 0, 0]; // X16 count = 1
        body.extend_from_slice(&[0u8; 8]); // one entry: FSB5 offset, FSB5 size
        let mut b = bank_with_sndh(&body, Sndh::NestedInList, &[0xAB; 64]);
        assert_eq!(
            parse_bank(&b).unwrap().len(),
            1,
            "the fixture must be a bank that really does carry a sample before it is corrupted"
        );
        let size_field = sndh_size_field(&b);
        b[size_field..size_field + 4].copy_from_slice(&0u32.to_le_bytes());
        let err = parse_bank(&b).unwrap_err();
        assert!(
            !err.contains("no sample data"),
            "a bank whose `SND ` payload is still there must not be called sample-free, got {err:?}"
        );
    }

    #[test]
    fn parse_bank_reports_a_bank_truncated_inside_its_nested_sndh_header_instead_of_panicking() {
        // The window this exists for: `Music_NotDemo.bank` cut to 444..=447 bytes. The guard proved
        // room for the fourcc and the body then read four more bytes for the size, so `gore audio
        // list` answered a short file with a Rust backtrace — and the Studio, which catches the
        // unwind, with an opaque transport panic instead of a decode error.
        let whole = bank_with_nested_sndh_body(&[0u8; 12]);
        let size_field = sndh_size_field(&whole);
        for cut in size_field..size_field + 4 {
            let err = parse_bank(&whole[..cut]).unwrap_err();
            assert!(
                err.contains("truncated"),
                "a bank cut off at {cut} inside its SNDH size field must be reported as \
                 truncated, got {err:?}"
            );
        }
    }

    #[test]
    fn parse_bank_finds_the_sndh_chunk_that_a_shipped_bank_nests_inside_a_list() {
        // Every one of the ten real banks reaches SNDH through the nested `LIST` arm, not the
        // direct one the other fixtures emit. Pin that the nested fixture is genuinely walked, so
        // the truncation case above cannot pass by failing some earlier check.
        let mut body = vec![2u8, 0, 0, 0]; // X16 count = 1
        body.extend_from_slice(&[0u8; 8]); // one entry: FSB5 offset, FSB5 size
        let entries = parse_bank(&bank_with_nested_sndh_body(&body)).unwrap();
        assert_eq!(entries.len(), 1, "a nested SNDH with one entry holds one FSB5");
    }

    #[test]
    fn parse_bank_still_returns_one_entry_for_a_bank_that_ships_an_fsb5() {
        // The ordinary bank runs through the same guard, so pin it next to the new branch: a
        // 4-byte version prefix plus one 8-byte entry is still exactly one FSB5.
        let mut body = vec![2u8, 0, 0, 0]; // X16 count = 1
        body.extend_from_slice(&[0u8; 8]); // one entry: FSB5 offset, FSB5 size
        let entries = parse_bank(&bank_with_sndh_body(&body)).unwrap();
        assert_eq!(entries.len(), 1, "a bank with one SNDH entry holds one FSB5");
    }

    /// A ramp, so two samples built with different lengths are also different bytes.
    fn ramp(name: &str, freq: u32, frames: usize) -> Pcm16Sample {
        Pcm16Sample {
            name: name.to_owned(),
            freq,
            channels: 1,
            pcm: (0..frames).map(|i| (i as i16).wrapping_mul(37)).collect(),
        }
    }

    /// Two samples, the second of which every test below replaces.
    fn two_sample_bank() -> Vec<u8> {
        let samples = [ramp("SFX_UI_Click_00", 22_050, 64), ramp("SFX_UI_Click_01", 22_050, 96)];
        test_fixture::pristine_bank_pcm16(&samples, GOTHIC_STUDIO_KEY).unwrap()
    }

    /// Count the top-level `SND ` chunks, walking the RIFF exactly as `find_top_list` does.
    fn top_level_snd_chunks(b: &[u8]) -> usize {
        let mut off = 0x0C;
        let mut count = 0;
        while off + 8 <= b.len() {
            let cc = &b[off..off + 4];
            if !printable(cc) {
                break;
            }
            if cc == b"SND " {
                count += 1;
            }
            off = off + 8 + u32_le(b, off + 4) as usize;
        }
        count
    }

    #[test]
    fn a_replaced_sample_reads_back_as_the_replacement_and_not_as_what_it_replaced() {
        // The defect this exists for. `replace_samples` does not overwrite audio: it appends a
        // second FSB5 and repoints the waveform at it, and the runtime follows that repoint —
        // FMOD's own loader, handed a bank injected by this code, plays the injected audio. But
        // every reader here went to sub-bank 0, which by design still holds the audio that was
        // replaced, byte for byte. So `gore audio list` answered a 0.30 s injection with the
        // original's 32.70 s, `extract` wrote out the original under the replaced sample's name,
        // and there was no way through these tools to tell a landed replacement from a lost one.
        let bank = two_sample_bank();
        let injected = replace_samples(
            &bank,
            GOTHIC_STUDIO_KEY,
            vec![("SFX_UI_Click_01".into(), ramp("whatever_it_is_called", 44_100, 8))],
        )
        .unwrap();

        let view = read_bank(&injected, GOTHIC_STUDIO_KEY).unwrap();
        let replaced = &view.samples[1];
        assert!(replaced.replaced, "the injected waveform must read as replaced");
        assert_eq!(
            (replaced.freq, replaced.num_samples),
            (44_100, 8),
            "the replacement's own rate and length must be what a listing reports, not the \
             22050 Hz / 96 frames it replaced"
        );
        assert_eq!(
            replaced.name, "SFX_UI_Click_01",
            "a replacement never renames the waveform: the name stays the identity a --map keys on"
        );
        assert_eq!(replaced.slot, WaveformSlot { sub_bank: 1, subsound: 0 });

        let untouched = &view.samples[0];
        assert!(!untouched.replaced, "replacing one waveform must not mark the others");
        assert_eq!((untouched.freq, untouched.num_samples), (22_050, 64));

        // Nothing above would fail if the injected audio itself were wrong, so read the PCM back
        // out of the sub-bank the slot names and compare it with what went in.
        let (block, fsb) = &view.sub_banks[replaced.slot.sub_bank];
        let source = &fsb.samples[replaced.slot.subsound];
        let start = (fsb.data_section + source.data_offset) as usize;
        let played: Vec<i16> = block[start..start + 16]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(
            played,
            ramp("x", 44_100, 8).pcm,
            "the bytes read back must be the bytes injected"
        );
    }

    #[test]
    fn an_unreplaced_bank_reads_every_waveform_as_its_own_subsound_in_sub_bank_zero() {
        // The other half of the pair: `replaced` has to be false for a bank nobody has touched, or
        // the flag says nothing. It also pins the identity mapping the resolver assumes — slot i
        // reads (0, i) — which is what makes a repointed slot detectable at all.
        let view = read_bank(&two_sample_bank(), GOTHIC_STUDIO_KEY).unwrap();
        assert_eq!(view.samples.len(), 2);
        for (index, sample) in view.samples.iter().enumerate() {
            assert!(!sample.replaced, "{} must not read as replaced", sample.name);
            assert_eq!(sample.slot, WaveformSlot { sub_bank: 0, subsound: index });
        }
    }

    #[test]
    fn an_injected_bank_carries_one_snd_chunk_for_every_sndh_entry() {
        // Why the `SND ` count is *supposed* to grow, and what must hold while it does. FMOD pairs
        // the i-th `SND ` chunk with SNDH entry i, so a second sub-bank needs a second chunk; two
        // chunks where the original had one is the injection working, not a file being appended to
        // by accident. The failure worth catching is the two counts drifting apart — an SNDH entry
        // with no chunk behind it is a bank the runtime cannot load, and `parse_bank` would still
        // read it happily because it only ever looks at SNDH.
        let bank = two_sample_bank();
        assert_eq!(top_level_snd_chunks(&bank), parse_bank(&bank).unwrap().len());

        let injected = replace_samples(
            &bank,
            GOTHIC_STUDIO_KEY,
            vec![("SFX_UI_Click_00".into(), ramp("tone", 44_100, 8))],
        )
        .unwrap();
        let entries = parse_bank(&injected).unwrap();
        assert_eq!(entries.len(), 2, "an injection adds exactly one sub-bank");
        assert_eq!(
            top_level_snd_chunks(&injected),
            entries.len(),
            "every SNDH entry must have the `SND ` chunk FMOD pairs with it"
        );
        assert_eq!(
            u32_le(&injected, 0x04) as usize,
            injected.len() - 8,
            "the RIFF length must describe the file the injection actually wrote"
        );
    }

    #[test]
    fn a_bank_summary_says_what_a_full_read_would_say_about_the_count_and_the_codec() {
        // `audio banks` prints these two numbers and `audio list` prints them again from a full
        // decrypt of the same file. Two readers of one bank that disagreed would send someone to
        // the wrong `--bank`, so the summary is compared against the reader it is a shortcut for
        // rather than against a literal.
        let bank = two_sample_bank();
        let summary = bank_summary(&bank, GOTHIC_STUDIO_KEY).unwrap();
        let view = read_bank(&bank, GOTHIC_STUDIO_KEY).unwrap();

        assert_eq!(
            summary,
            BankSummary::Samples {
                sub_banks: 1,
                sample_count: view.samples.len(),
                codec: view.codec(),
            }
        );
        assert_eq!(view.codec(), Codec::Pcm16, "the fixture's codec has to be read, not assumed");
    }

    /// A bank whose audio is far larger than the wrapper probe, so that reading the wrapper is
    /// visibly not reading the file. 400 KB of PCM against a wrapper of a few hundred bytes.
    fn big_bank() -> Vec<u8> {
        let samples = [ramp("SFX_Big_00", 44_100, 100_000), ramp("SFX_Big_01", 44_100, 100_000)];
        let bank = test_fixture::pristine_bank_pcm16(&samples, GOTHIC_STUDIO_KEY).unwrap();
        assert!(
            bank.len() > WRAPPER_PROBE_BYTES,
            "the fixture has to be bigger than the probe to say anything: {} bytes",
            bank.len()
        );
        bank
    }

    fn write_bank(dir: &std::path::Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn summarising_a_bank_on_disk_says_exactly_what_summarising_its_bytes_says() {
        // `audio banks` reads from disk and every test above reads from memory. Two routes to one
        // answer is how a listing starts contradicting `audio list` about a file — so the route
        // that ships is compared against the one that is tested, on every shape that reaches it,
        // errors included: a wrong key and a truncated bank have to fail the same way too.
        let dir = tempfile::tempdir().unwrap();
        let injected = replace_samples(
            &two_sample_bank(),
            GOTHIC_STUDIO_KEY,
            vec![("SFX_UI_Click_00".into(), ramp("tone", 44_100, 8))],
        )
        .unwrap();
        let big = big_bank();
        let truncated = big[..big.len() / 2].to_vec();

        let shapes: Vec<(&str, Vec<u8>, &[u8])> = vec![
            ("two-sample", two_sample_bank(), GOTHIC_STUDIO_KEY),
            ("sample-free", test_fixture::sample_free_bank(), GOTHIC_STUDIO_KEY),
            ("injected", injected, GOTHIC_STUDIO_KEY),
            ("bigger-than-the-probe", big, GOTHIC_STUDIO_KEY),
            ("truncated", truncated, GOTHIC_STUDIO_KEY),
            ("wrong-key", two_sample_bank(), b"not-the-studio-key"),
            ("not-a-bank", b"RIFFnope".to_vec(), GOTHIC_STUDIO_KEY),
        ];

        for (label, bytes, key) in shapes {
            let path = write_bank(dir.path(), &format!("{label}.bank"), &bytes);
            assert_eq!(
                bank_summary_at(&path, key),
                bank_summary(&bytes, key),
                "the two routes disagree about the {label} bank"
            );
        }
    }

    #[test]
    fn a_wrapper_that_reaches_past_the_first_read_is_read_again_rather_than_misparsed() {
        // The shipped banks all fit the probe, so the second read is the branch nothing would
        // exercise until an install turned up whose bank has a longer metadata chunk — and the
        // failure would be a wrong answer, not an error, since a wrapper cut short still walks.
        let dir = tempfile::tempdir().unwrap();
        let bank = two_sample_bank();
        let path = write_bank(dir.path(), "grown.bank", &bank);

        let whole = bank_summary(&bank, GOTHIC_STUDIO_KEY);
        assert!(matches!(whole, Ok(BankSummary::Samples { .. })), "{whole:?}");
        for probe in [1, 0x18, 64, 256, bank.len(), bank.len() * 2] {
            assert_eq!(
                bank_summary_probing(&path, GOTHIC_STUDIO_KEY, probe, METADATA_MAX_BYTES),
                whole,
                "a {probe}-byte first read changed the answer"
            );
        }
    }

    #[test]
    fn audio_cut_short_is_refused_rather_than_handed_back_shorter() {
        // `extract_wav` clamped the sample's range to the block, so a truncated bank produced a
        // WAV missing its tail and `gore audio extract` called that a success. Nothing about the
        // file looks wrong until somebody plays it, which is the worst shape a failure can take.
        let bank = two_sample_bank();
        let entry = parse_bank(&bank).unwrap()[0];
        let end = entry.fsb5_offset + entry.fsb5_size;

        // Whole, it reads.
        assert!(read_bank(&bank, GOTHIC_STUDIO_KEY).is_ok());

        // A block cut short by a hundred bytes: the tables still parse, the audio does not fit.
        let mut cut = bank.clone();
        cut.truncate(end - 100);
        let error = match read_bank(&cut, GOTHIC_STUDIO_KEY) {
            Err(error) => error,
            Ok(_) => panic!("a block cut short must not read as a whole bank"),
        };
        assert!(
            error.contains("truncated") || error.contains("out of range"),
            "the error has to name the shape of the damage, got {error:?}"
        );
    }

    #[test]
    fn truncated_vorbis_is_refused_too_and_not_only_the_pcm_arm() {
        // The fixture builder makes PCM16 banks, so the truncation test above cannot reach this
        // arm — and this is the arm that matters for the shipped banks, which are all Vorbis.
        // Clamping there handed the remuxer a complete packet prefix, which it accepts and closes
        // with an EOS page: a playable Ogg quietly missing its tail, reported as a success.
        let fsb = Fsb5 {
            version: 1,
            codec: Codec::Vorbis,
            data_section: 0,
            samples: vec![Fsb5Sample {
                name: "SFX_Truncated".to_string(),
                data_offset: 0,
                size: 4096,
                freq: 44_100,
                channels: 1,
                num_samples: 2048,
                vorbis_crc32: Some(0x1234_5678),
            }],
        };

        // Half the audio the sample declares.
        let short = vec![0u8; 2048];
        let error = extract_ogg(&short, &fsb, 0).unwrap_err();
        assert!(error.contains("truncated"), "{error}");
        assert!(error.contains("SFX_Truncated"), "the sample has to be named: {error}");

        // The control: with the declared bytes present it gets past the bounds check and fails on
        // the payload instead, which is a different sentence about a different thing.
        let whole = vec![0u8; 4096];
        let error = extract_ogg(&whole, &fsb, 0).unwrap_err();
        assert!(!error.contains("truncated"), "{error}");
    }

    #[test]
    fn a_wrapper_declaring_more_than_this_reads_is_refused_by_both_routes_alike() {
        // The other half of the bounded fallback: a LIST whose size field is corrupt still parses,
        // and clamping the extent to the file made the on-disk route materialize the whole bank —
        // hundreds of megabytes to report that a file is damaged. Refused instead, in the shared
        // body, so a caller holding the bytes gets the same sentence as one reading the file.
        let dir = tempfile::tempdir().unwrap();
        let mut bank = big_bank();
        let (list_off, list_size) = find_top_list(&bank).unwrap();
        let honest = list_off + 8 + list_size;

        // Past the end of the file, which is what a corrupt size field looks like.
        bank[list_off + 4..list_off + 8].copy_from_slice(&u32::MAX.to_le_bytes());
        let path = write_bank(dir.path(), "oversized.bank", &bank);

        // A limit below the file rather than a 64 MB fixture, driven through both routes at once.
        let max = honest;
        let from_disk = bank_summary_probing(&path, GOTHIC_STUDIO_KEY, 0x40, max);
        let from_memory = summarize(
            &bank,
            bank.len(),
            max,
            &mut |offset, len| Ok(bank.get(offset..offset + len).map(<[u8]>::to_vec)),
            GOTHIC_STUDIO_KEY,
        );
        assert_eq!(from_disk, from_memory, "one route read what the other refused");
        let error = from_disk.unwrap_err();
        assert!(error.contains("declares"), "{error}");
        assert!(error.contains("corrupt"), "{error}");

        // And the honest wrapper still goes through at the same limit, so the refusal is about the
        // declaration and not about the bank being large.
        let path = write_bank(dir.path(), "honest.bank", &big_bank());
        assert_eq!(
            bank_summary_probing(&path, GOTHIC_STUDIO_KEY, 0x40, max),
            bank_summary(&big_bank(), GOTHIC_STUDIO_KEY)
        );
    }

    #[test]
    fn a_sample_table_declaring_more_than_this_reads_is_refused_by_both_routes_alike() {
        // The second extent a bank declares, and the same failure one field further in: the block
        // is no bound, so a corrupt `shdr_size` reaching the end of a 247 MB block clamps to the
        // block and the listing reads all of it. The bank's wrapper is intact here — only the
        // FSB5 header is not — so nothing above this catches it.
        let dir = tempfile::tempdir().unwrap();
        let mut bank = big_bank();
        let entry = parse_bank(&bank).unwrap()[0];

        // Written through the cipher, since this field lives in the encrypted block.
        let mut header = bank[entry.fsb5_offset..entry.fsb5_offset + FSB5_HEADER_LEN].to_vec();
        fsb5_decrypt(&mut header, GOTHIC_STUDIO_KEY);
        let base = sample_table_base(&header);
        let names_size = u32_le(&header, 0x10) as usize;
        let honest = base + u32_le(&header, 0x0C) as usize + names_size;
        // Filling the block, not overflowing it. `header_fits` already refuses tables declared
        // past the block's own extent, and deliberately does not count the audio against them —
        // so a size field that stops exactly at the end of a 247 MB block passes every check
        // there is and asks the summary to read the whole thing.
        let filling = entry.fsb5_size - base - names_size;
        header[0x0C..0x10].copy_from_slice(&(filling as u32).to_le_bytes());
        fsb5_encrypt(&mut header, GOTHIC_STUDIO_KEY);
        bank[entry.fsb5_offset..entry.fsb5_offset + FSB5_HEADER_LEN].copy_from_slice(&header);

        let path = write_bank(dir.path(), "bad-table.bank", &bank);
        // Above both honest extents, so this bank clears the wrapper limit and only the corrupted
        // table trips it. A limit below the wrapper would have been answered before reaching here,
        // and the test would have passed while proving nothing about the table.
        let (list_off, list_size) = find_top_list(&bank).unwrap();
        let max = honest.max(list_off + 8 + list_size);
        let from_disk = bank_summary_probing(&path, GOTHIC_STUDIO_KEY, 0x40, max);
        let from_memory = summarize(
            &bank,
            bank.len(),
            max,
            &mut |offset, len| Ok(bank.get(offset..offset + len).map(<[u8]>::to_vec)),
            GOTHIC_STUDIO_KEY,
        );
        assert_eq!(from_disk, from_memory, "one route read what the other refused");
        let error = from_disk.unwrap_err();
        assert!(error.contains("sample and name tables"), "{error}");
        assert!(error.contains("audio list"), "the slow route has to be named: {error}");

        // The control: the honest table goes through at the very same limit, so the refusal is
        // about the declaration and not about the bank.
        let path = write_bank(dir.path(), "good-table.bank", &big_bank());
        assert_eq!(
            bank_summary_probing(&path, GOTHIC_STUDIO_KEY, 0x40, max),
            bank_summary(&big_bank(), GOTHIC_STUDIO_KEY)
        );
    }

    #[test]
    fn a_bank_of_zero_sized_chunks_does_not_walk_itself_to_a_standstill() {
        // Bounding the READ was not bounding the work. A chunk that declares zero bytes advances
        // the walk by its 8-byte header alone, so a damaged 260 MB bank whose top-level chunks all
        // declare zero is walked in 8-byte steps — 32 million of them, each a seek and a read on
        // the on-disk route. Nothing allocates and the command never comes back.
        let mut bank = big_bank();
        // A printable fourcc that is not LIST, and a zero size, all the way down: the walk cannot
        // stop on the fourcc and cannot advance on the size.
        for chunk in bank[0x0C..].chunks_mut(8) {
            if chunk.len() == 8 {
                chunk[0..4].copy_from_slice(b"JUNK");
                chunk[4..8].copy_from_slice(&0u32.to_le_bytes());
            }
        }

        let mut probes = 0usize;
        let found = find_top_list_with(bank.len(), TopWalk::Any, &mut |off| {
            probes += 1;
            let mut header = [0u8; 8];
            match bank.get(off..off + 8) {
                Some(bytes) => {
                    header.copy_from_slice(bytes);
                    Ok(Some(header))
                }
                None => Ok(None),
            }
        });

        assert!(found.is_err(), "there is no LIST in this bank");
        assert!(
            probes <= MAX_TOP_CHUNK_PROBES,
            "the walk looked at {probes} headers in a {} byte bank",
            bank.len()
        );
        // And the bound is what stopped it, rather than the file happening to be small.
        assert!(bank.len() / 8 > MAX_TOP_CHUNK_PROBES, "the fixture has to be able to outrun it");
    }

    #[test]
    fn a_damaged_bank_with_no_wrapper_to_find_is_not_read_whole_to_say_so() {
        // The case that reaches the fallback is a bank `banks` is being run to diagnose, and
        // reading the file to report it damaged put back the whole cost the prefix read exists to
        // avoid — the worst moment to allocate 260 MB. The walk needs the 8-byte chunk headers and
        // nothing between them, so what it touches is counted rather than assumed.
        let mut bank = big_bank();
        // A RIFF header, then noise where the chunk table belongs: nothing that walks this finds a
        // LIST, under either rule.
        for byte in bank[0x0C..].iter_mut() {
            *byte = 0xFF;
        }

        let probe = &bank[..0x40];
        let mut read = 0usize;
        let needed = wrapper_extent(probe, bank.len(), &mut |off| {
            read += 8;
            let mut header = [0u8; 8];
            match bank.get(off..off + 8) {
                Some(bytes) => {
                    header.copy_from_slice(bytes);
                    Ok(Some(header))
                }
                None => Ok(None),
            }
        })
        .unwrap();

        assert_eq!(needed, 0, "there is no LIST to find, so nothing more should be read");

        // `bank_shape_within` had its own copy of this walk, so the bound above did not reach the
        // in-memory route at all. It shares the walker now; what a test can pin is that sharing it
        // did not change the verdict.
        let error = bank_summary(&bank, GOTHIC_STUDIO_KEY).unwrap_err();
        assert!(error.contains("no top-level LIST chunk"), "{error}");
        assert!(
            (probe.len() + read) * 20 < bank.len(),
            "locating the wrapper touched {} of {} bytes",
            probe.len() + read,
            bank.len()
        );

        // And the answer is still the one a full read gives.
        let dir = tempfile::tempdir().unwrap();
        let path = write_bank(dir.path(), "damaged.bank", &bank);
        assert_eq!(
            bank_summary_probing(&path, GOTHIC_STUDIO_KEY, 0x40, METADATA_MAX_BYTES),
            bank_summary(&bank, GOTHIC_STUDIO_KEY)
        );
    }

    #[test]
    fn summarising_reads_the_wrapper_and_the_headers_and_not_the_audio() {
        // The claim `audio banks` is built on. Agreement above would still hold if the disk route
        // read all 520 MB, so what it reads is counted here rather than assumed: everything the
        // summary fetches outside the wrapper is FSB5 metadata, and the audio those blocks declare
        // is never touched.
        let bank = big_bank();
        let (list_off, list_size) = find_top_list(&bank).unwrap();
        let wrapper_end = list_off + 8 + list_size;

        let mut fetched = 0usize;
        let summary = summarize(
            &bank[..wrapper_end],
            bank.len(),
            METADATA_MAX_BYTES,
            &mut |offset, len| {
                fetched += len;
                Ok(bank.get(offset..offset + len).map(<[u8]>::to_vec))
            },
            GOTHIC_STUDIO_KEY,
        )
        .unwrap();

        assert_eq!(summary, bank_summary(&bank, GOTHIC_STUDIO_KEY).unwrap());
        let read = wrapper_end + fetched;
        assert!(
            read * 20 < bank.len(),
            "a summary read {read} of {} bytes, which is not a shortcut",
            bank.len()
        );
    }

    #[test]
    fn decrypting_a_prefix_of_a_sub_bank_gives_the_same_bytes_as_decrypting_all_of_it() {
        // The property that lets `bank_summary` read a header on its own, and the whole reason it
        // is cheap: the cipher is position-indexed from the block start, so byte i does not depend
        // on any byte after it. If this ever stopped holding, `bank_summary` would keep returning
        // numbers — wrong ones — on every bank in the install.
        let bank = two_sample_bank();
        let entry = parse_bank(&bank).unwrap()[0];
        let block = &bank[entry.fsb5_offset..entry.fsb5_offset + entry.fsb5_size];

        let mut whole = block.to_vec();
        fsb5_decrypt(&mut whole, GOTHIC_STUDIO_KEY);
        let mut prefix = block[..FSB5_HEADER_LEN].to_vec();
        fsb5_decrypt(&mut prefix, GOTHIC_STUDIO_KEY);

        assert_eq!(prefix, whole[..FSB5_HEADER_LEN]);
        assert_eq!(&prefix[0..4], b"FSB5", "the prefix has to be the real header, not noise");
    }

    #[test]
    fn a_sample_free_bank_summarises_as_sample_free_rather_than_as_a_failure() {
        // Six of the ten shipped banks are this shape. `parse_bank` reports it as an error, which
        // is right for a caller asking for sub-banks and wrong for a listing of a directory: four
        // rows under a heading that claims to describe ten files is a worse answer than none.
        assert_eq!(
            bank_summary(&test_fixture::sample_free_bank(), GOTHIC_STUDIO_KEY).unwrap(),
            BankSummary::SampleFree
        );
    }

    #[test]
    fn a_summary_counts_the_sub_bank_an_injection_appended_and_still_counts_shipped_samples() {
        // What makes `audio banks` able to say a bank has been modded without decrypting it. The
        // sample count must stay the shipped one: an injection repoints waveforms, it never adds
        // or renames one, so a count that grew would contradict `audio list` on the same file.
        let injected = replace_samples(
            &two_sample_bank(),
            GOTHIC_STUDIO_KEY,
            vec![("SFX_UI_Click_00".into(), ramp("tone", 44_100, 8))],
        )
        .unwrap();

        assert_eq!(
            bank_summary(&injected, GOTHIC_STUDIO_KEY).unwrap(),
            BankSummary::Samples { sub_banks: 2, sample_count: 2, codec: Codec::Pcm16 }
        );
    }

    #[test]
    fn a_summary_read_with_the_wrong_key_names_the_key_instead_of_printing_a_number() {
        // Every field a summary reports comes out of the encrypted header, so a wrong `--key` does
        // not fail — it decodes noise. Without the magic check `audio banks` would print an
        // arbitrary sample count and an `Unknown(…)` codec for a bank that is perfectly fine.
        let err = bank_summary(&two_sample_bank(), b"not-the-studio-key").unwrap_err();
        assert!(
            err.contains("FSB5 magic") && err.contains("key"),
            "the message must name what was read and what to suspect, got {err:?}"
        );
    }

    #[test]
    fn replacing_a_second_time_refuses_instead_of_stacking_another_sub_bank() {
        // Rebuilding from an already-injected bank would append a third FSB5 whose sub-bank 0 is
        // itself a modded bank, and each round would carry the last round's audio forward for ever.
        // The refusal is what makes `*.gore-bak` the input, and it has to name that remedy.
        let injected = replace_samples(
            &two_sample_bank(),
            GOTHIC_STUDIO_KEY,
            vec![("SFX_UI_Click_00".into(), ramp("tone", 44_100, 8))],
        )
        .unwrap();
        let err = replace_samples(
            &injected,
            GOTHIC_STUDIO_KEY,
            vec![("SFX_UI_Click_01".into(), ramp("tone", 44_100, 8))],
        )
        .unwrap_err();
        assert!(
            err.contains("already contains modded audio") && err.contains("gore-bak"),
            "the refusal must name the backup that is the real input, got {err:?}"
        );
    }
}
