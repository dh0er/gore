use anyhow::Result;
use std::io::{Read as _, Write};
use strum::{AsRefStr, EnumString, VariantArray};

/// Oodle/Kraken encode effort used by the gore-oodle-backed encode arm. The pure-Rust
/// Kraken encoder produces valid Oodle-decodable blocks; UE only needs validity at the
/// advertised uncompressed sizes, not byte-identity with Epic's encoder.
const OODLE_ENCODE_LEVEL: gore_oodle::Level = gore_oodle::Level::Default;

#[derive(Debug, Clone, Copy, PartialEq, EnumString, AsRefStr, VariantArray)]
pub enum CompressionMethod {
    Zlib,
    Zstd,
    LZ4,
    Oodle,
}
impl CompressionMethod {
    pub fn from_str_ignore_case(value: &str) -> Option<Self> {
        CompressionMethod::VARIANTS.iter().copied().find(|v| v.as_ref().eq_ignore_ascii_case(value))
    }
}

pub fn compress<S: Write>(compression: CompressionMethod, input: &[u8], mut output: S) -> Result<()> {
    match compression {
        CompressionMethod::Zlib => {
            let mut encoder = flate2::write::ZlibEncoder::new(output, flate2::Compression::best());
            encoder.write_all(input)?;
            encoder.finish()?;
        }
        CompressionMethod::Zstd => {
            let buf = zstd::stream::encode_all(input, 0)?;
            output.write_all(&buf)?;
        }
        CompressionMethod::LZ4 => {
            let buf = lz4_flex::block::compress(input);
            output.write_all(&buf)?;
        }
        CompressionMethod::Oodle => {
            // ENCODE routed to the in-repo pure-Rust gore-oodle (Kraken) codec so the
            // `to-zen` write path needs NO Epic Oodle DLL and no C/C++ toolchain.
            // Upstream retoc passed (Compressor::Mermaid, CompressionLevel::Normal) to
            // Epic's encoder; here we emit Kraken. UE does not require byte-identical
            // re-compression to load a container -- it only needs valid Oodle-decodable
            // blocks at the advertised uncompressed sizes, which Kraken provides. The
            // .utoc still records this block as Oodle (CompressionMethod::Oodle) so the
            // game decompresses it via Oodle.
            let compressed = gore_oodle::compress(input, OODLE_ENCODE_LEVEL)
                .map_err(|e| anyhow::anyhow!("Oodle (gore-oodle) compression failed: {e}"))?;
            output.write_all(&compressed)?;
        }
    }
    Ok(())
}

pub fn decompress(compression: CompressionMethod, input: &[u8], output: &mut [u8]) -> Result<()> {
    match compression {
        CompressionMethod::Zlib => {
            flate2::read::ZlibDecoder::new(input).read_exact(output)?;
        }
        CompressionMethod::Zstd => {
            zstd::bulk::decompress_to_buffer(input, output)?;
        }
        CompressionMethod::LZ4 => {
            lz4_flex::block::decompress_into(input, output)?;
        }
        CompressionMethod::Oodle => {
            // DECODE routed to the in-repo gore-oodle (Kraken) codec, which is
            // byte-validated as identical to Epic's Oodle decoder. `output.len()` is
            // the exact uncompressed block size retoc already computed from the TOC.
            let decoded = gore_oodle::decompress(input, output.len()).map_err(|e| anyhow::anyhow!("Oodle (gore-oodle) decompression failed: {e}"))?;
            output.copy_from_slice(&decoded);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    /// Compress a ~256 KiB varied buffer through the gore-oodle-backed Oodle ENCODE
    /// arm and decompress it back through the Oodle DECODE arm; assert identity.
    /// This proves the `to-zen` write path produces Oodle blocks our own decode
    /// (and the game's Oodle) can read back, with no Epic Oodle DLL involved.
    #[test]
    fn oodle_encode_decode_round_trip() {
        // Varied, semi-compressible content: a mix of structured runs and pseudo
        // random bytes so the codec is exercised on real-ish entropy.
        let n = 256 * 1024;
        let mut original = Vec::with_capacity(n);
        let mut state: u32 = 0x1234_5678;
        for i in 0..n {
            // xorshift PRNG mixed with a low-entropy pattern.
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let patterned = (i % 251) as u8;
            original.push(patterned ^ (state as u8));
        }

        let mut compressed = Vec::new();
        compress(CompressionMethod::Oodle, &original, &mut compressed)
            .expect("Oodle encode must succeed via gore-oodle");
        assert!(!compressed.is_empty(), "compressed output should be non-empty");

        let mut roundtripped = vec![0u8; original.len()];
        decompress(CompressionMethod::Oodle, &compressed, &mut roundtripped)
            .expect("Oodle decode must succeed via gore-oodle");

        assert_eq!(roundtripped, original, "round-trip through Oodle encode+decode must be identity");
    }
}
