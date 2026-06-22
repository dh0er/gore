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

/// In-process Oodle Kraken codec backed by the vendored ooz sources. Always
/// available; needs no game executable.
#[derive(Debug, Clone, Copy, Default)]
pub struct OozKrakenBackend;

impl OozKrakenBackend {
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

impl CodecBackend for OozKrakenBackend {
    fn probe(&self) -> Result<CodecBackendProbe, CoreError> {
        let (can_decompress, can_compress, status) = self.self_test();
        Ok(CodecBackendProbe {
            backend: "ooz_kraken".to_string(),
            available: can_decompress,
            can_decompress,
            can_compress,
            status,
            profile: None,
            resolution_mode: None,
            details: json!({ "adapter": "ooz_kraken" }),
        })
    }

    fn decompress(&self, input: &[u8], expected_size: usize) -> Result<Vec<u8>, CoreError> {
        gore_oodle::kraken_decompress(input, expected_size)
            .map_err(|e| CoreError::Codec(e.to_string()))
    }

    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, CoreError> {
        gore_oodle::kraken_compress(input, level).map_err(|e| CoreError::Codec(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ooz_backend_roundtrips_and_reports_available() {
        let backend = OozKrakenBackend::default();

        let input: Vec<u8> = (0..4096u32).map(|i| (i * 5) as u8).collect();
        let comp = backend.compress(&input, 6).unwrap(); // level clamped internally
        let back = backend.decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);

        let probe = backend.probe().unwrap();
        assert_eq!(probe.backend, "ooz_kraken");
        assert!(probe.available);
        assert!(probe.can_decompress);
        assert!(probe.can_compress);
    }
}
