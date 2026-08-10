use crate::CoreError;
use crate::codec_calibration;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub trait CodecBackend {
    fn probe(&self) -> Result<CodecBackendProbe, CoreError>;
    fn decompress(&self, input: &[u8], expected_size: usize) -> Result<Vec<u8>, CoreError>;
    fn decompress_many(&self, chunks: &[CodecDecodeChunk<'_>]) -> Result<Vec<Vec<u8>>, CoreError> {
        chunks
            .iter()
            .map(|chunk| self.decompress(chunk.input, chunk.expected_size))
            .collect()
    }
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, CoreError>;
    fn compress_many(&self, chunks: &[CodecEncodeChunk<'_>]) -> Result<Vec<Vec<u8>>, CoreError> {
        chunks
            .iter()
            .map(|chunk| self.compress(chunk.input, chunk.level))
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CodecDecodeChunk<'a> {
    pub input: &'a [u8],
    pub expected_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CodecEncodeChunk<'a> {
    pub input: &'a [u8],
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecBackendProbe {
    pub backend: String,
    pub available: bool,
    pub can_decompress: bool,
    pub can_compress: bool,
    pub status: String,
    pub profile: Option<String>,
    pub resolution_mode: Option<String>,
    pub details: Value,
}

/// In-process Oodle Kraken codec, pure Rust. Always available; needs no game
/// executable and no C/C++ toolchain.
#[derive(Debug, Clone, Copy, Default)]
pub struct KrakenBackend;

/// Map the backend's numeric level to the pure-Rust codec's effort `Level`. The v1
/// Kraken encoder's levels are close in ratio; the old 0..=5 "safe" range maps onto
/// the effort enum, with anything higher treated as `Max`.
fn level_to_oodle(level: u8) -> gore_oodle::Level {
    match level {
        0..=1 => gore_oodle::Level::Fastest,
        2..=3 => gore_oodle::Level::Fast,
        4..=6 => gore_oodle::Level::Default,
        _ => gore_oodle::Level::Max,
    }
}

impl KrakenBackend {
    fn self_test(&self) -> (bool, bool, String) {
        let can_decompress = codec_calibration::decode_self_test();
        let can_compress = can_decompress && codec_calibration::compress_roundtrip_self_test();
        let status = match (can_decompress, can_compress) {
            (true, true) => "ready",
            (true, false) => "decode_only",
            _ => "unavailable",
        };
        (can_decompress, can_compress, status.to_string())
    }
}

impl CodecBackend for KrakenBackend {
    fn probe(&self) -> Result<CodecBackendProbe, CoreError> {
        let (can_decompress, can_compress, status) = self.self_test();
        Ok(CodecBackendProbe {
            backend: "kraken".to_string(),
            available: can_decompress,
            can_decompress,
            can_compress,
            status,
            profile: None,
            resolution_mode: None,
            details: json!({ "adapter": "kraken" }),
        })
    }

    fn decompress(&self, input: &[u8], expected_size: usize) -> Result<Vec<u8>, CoreError> {
        gore_oodle::decompress(input, expected_size).map_err(|e| CoreError::Codec(e.to_string()))
    }

    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, CoreError> {
        gore_oodle::compress(input, level_to_oodle(level))
            .map_err(|e| CoreError::Codec(e.to_string()))
    }

    // A save's private stream is hundreds of independent chunks (916 x 128 KiB on a
    // real 2.5 MB save), and the trait's default batch methods run them one at a
    // time. Encoding is the write path's dominant cost, so fan the chunks across
    // cores instead.
    fn decompress_many(&self, chunks: &[CodecDecodeChunk<'_>]) -> Result<Vec<Vec<u8>>, CoreError> {
        map_chunks_in_parallel(chunks, |chunk| {
            self.decompress(chunk.input, chunk.expected_size)
        })
    }

    fn compress_many(&self, chunks: &[CodecEncodeChunk<'_>]) -> Result<Vec<Vec<u8>>, CoreError> {
        map_chunks_in_parallel(chunks, |chunk| self.compress(chunk.input, chunk.level))
    }
}

/// Run one per-chunk codec job across worker threads.
///
/// Safe because `gore_oodle::compress`/`decompress` are pure functions of their
/// arguments: the crate is `no_std`, holds no global or thread-local state, and
/// allocates every scratch buffer per call — so concurrent calls cannot observe each
/// other and each chunk's output is identical to what the serial path produced.
///
/// Results keep their input order (the chunk table and the payload are both emitted
/// by index), and a failure surfaces as the LOWEST-index failing chunk, matching the
/// serial `.collect()` this replaces.
fn map_chunks_in_parallel<T, F>(items: &[T], job: F) -> Result<Vec<Vec<u8>>, CoreError>
where
    T: Sync,
    F: Fn(&T) -> Result<Vec<u8>, CoreError> + Sync,
{
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(items.len());
    // Also covers the empty stream (`minimal_gsav` declares zero chunks), where
    // `items.chunks(0)` below would panic.
    if workers <= 1 {
        return items.iter().map(job).collect();
    }
    let stride = items.len().div_ceil(workers);
    let mut results: Vec<Option<Result<Vec<u8>, CoreError>>> =
        items.iter().map(|_| None).collect();
    std::thread::scope(|scope| {
        let job = &job;
        for (inputs, slots) in items.chunks(stride).zip(results.chunks_mut(stride)) {
            scope.spawn(move || {
                for (item, slot) in inputs.iter().zip(slots.iter_mut()) {
                    *slot = Some(job(item));
                }
            });
        }
    });
    results
        .into_iter()
        .map(|slot| slot.expect("every slot is filled by the worker that owns it"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_backend_roundtrips_and_reports_available() {
        let backend = KrakenBackend::default();

        let input: Vec<u8> = (0..4096u32).map(|i| (i * 5) as u8).collect();
        let comp = backend.compress(&input, 6).unwrap(); // level mapped to effort enum
        let back = backend.decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);

        let probe = backend.probe().unwrap();
        assert_eq!(probe.backend, "kraken");
        assert!(probe.available);
        assert!(probe.can_decompress);
        assert!(probe.can_compress);
    }

    /// The batch methods fan chunks across threads. Their output must stay
    /// byte-identical to the serial path and keep its order, because the chunk table
    /// and the payload are written by index.
    #[test]
    fn batch_codec_matches_the_serial_path_chunk_for_chunk() {
        let backend = KrakenBackend::default();
        // Enough chunks that the fan-out actually splits, with distinct contents so a
        // reordering bug cannot pass.
        let inputs: Vec<Vec<u8>> = (0..64u32)
            .map(|seed| (0..8192u32).map(|i| (i * 31 + seed * 7) as u8).collect())
            .collect();

        let encode: Vec<CodecEncodeChunk<'_>> = inputs
            .iter()
            .map(|input| CodecEncodeChunk { input, level: 6 })
            .collect();
        let batched = backend.compress_many(&encode).unwrap();
        let serial: Vec<Vec<u8>> = inputs
            .iter()
            .map(|input| backend.compress(input, 6).unwrap())
            .collect();
        assert_eq!(batched, serial);

        let decode: Vec<CodecDecodeChunk<'_>> = batched
            .iter()
            .zip(inputs.iter())
            .map(|(compressed, original)| CodecDecodeChunk {
                input: compressed,
                expected_size: original.len(),
            })
            .collect();
        assert_eq!(backend.decompress_many(&decode).unwrap(), inputs);
    }

    /// A save whose private stream declares no chunks reaches the batch methods with
    /// an empty slice; the fan-out must not divide by zero or panic there.
    #[test]
    fn batch_codec_accepts_an_empty_chunk_list() {
        let backend = KrakenBackend::default();
        assert!(backend.compress_many(&[]).unwrap().is_empty());
        assert!(backend.decompress_many(&[]).unwrap().is_empty());
    }

    /// A corrupt chunk must surface the same error the serial path surfaced: the one
    /// belonging to the lowest failing index, not whichever worker finished first.
    #[test]
    fn batch_codec_reports_the_lowest_failing_chunk() {
        let backend = KrakenBackend::default();
        let good = backend.compress(&[7u8; 4096], 6).unwrap();
        let chunks = vec![
            CodecDecodeChunk { input: &good, expected_size: 4096 },
            // Two different-length garbage chunks: whichever error wins is
            // identifiable, so a race would show up as a flaky expected_size.
            CodecDecodeChunk { input: b"not-a-kraken-stream", expected_size: 4096 },
            CodecDecodeChunk { input: b"also-not-a-stream", expected_size: 8192 },
        ];
        let error = backend.decompress_many(&chunks).unwrap_err().to_string();
        let serial = backend
            .decompress(chunks[1].input, chunks[1].expected_size)
            .unwrap_err()
            .to_string();
        assert_eq!(error, serial);
    }
}
