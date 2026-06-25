use anyhow::{Result, bail};
use std::io::{Read as _, Write};
use strum::{AsRefStr, EnumString, VariantArray};

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
            // ENCODE is intentionally unsupported in this vendored, gore-oodle-backed
            // fork. The gore-tex read path (list / unpack) never compresses, and
            // gore-oodle's ooz Kraken encoder is byte-identical to Epic's Oodle only
            // at the default level -- it cannot reproduce Epic's higher-compression
            // output, so a future `to-zen` repack needs a deliberate backend choice
            // (see docs/superpowers/notes/2026-06-26-retoc-oodle-swap.md). Silently
            // emitting non-matching blocks could produce containers the game rejects,
            // so we fail loudly instead.
            let _ = (input, &mut output);
            bail!("Oodle compression is not supported by the gore-oodle-backed retoc fork (decode only)");
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
            // DECODE routed to the in-repo gore-oodle (ooz Kraken) codec, which is
            // byte-validated as identical to Epic's Oodle decoder. `output.len()` is
            // the exact uncompressed block size retoc already computed from the TOC.
            let decoded = gore_oodle::kraken_decompress(input, output.len()).map_err(|e| anyhow::anyhow!("Oodle (gore-oodle) decompression failed: {e}"))?;
            output.copy_from_slice(&decoded);
        }
    }
    Ok(())
}
