use crate::{CoreError, kraken};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

#[derive(Debug, Clone, Copy, Default)]
pub struct PureRustKrakenBackend;

impl CodecBackend for PureRustKrakenBackend {
    fn probe(&self) -> Result<CodecBackendProbe, CoreError> {
        let details = kraken::codec_status();
        Ok(CodecBackendProbe {
            backend: details
                .get("adapter")
                .and_then(Value::as_str)
                .unwrap_or("pure_rust_kraken")
                .to_string(),
            available: details
                .get("available")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            can_decompress: details
                .get("canDecompress")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            can_compress: details
                .get("canCompress")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            status: details
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            profile: None,
            resolution_mode: None,
            details,
        })
    }

    fn decompress(&self, _input: &[u8], _expected_size: usize) -> Result<Vec<u8>, CoreError> {
        Err(CoreError::Codec(
            "pure Rust Kraken decoder is not implemented yet".to_string(),
        ))
    }

    fn compress(&self, _input: &[u8], _level: u8) -> Result<Vec<u8>, CoreError> {
        Err(CoreError::Codec(
            "pure Rust Kraken encoder is not implemented yet".to_string(),
        ))
    }
}

pub struct G1rBinaryHostBackend {
    exe_path: PathBuf,
    derived_profile_cache_path: Option<PathBuf>,
    invoker: Box<dyn CodecHostInvoker>,
}

impl G1rBinaryHostBackend {
    pub fn new(helper_path: impl Into<PathBuf>, exe_path: impl Into<PathBuf>) -> Self {
        Self {
            exe_path: exe_path.into(),
            derived_profile_cache_path: None,
            invoker: Box::new(ProcessCodecHostInvoker {
                helper_path: helper_path.into(),
            }),
        }
    }

    pub fn with_derived_profile_cache_path(mut self, cache_path: impl Into<PathBuf>) -> Self {
        self.derived_profile_cache_path = Some(cache_path.into());
        self
    }

    #[cfg(test)]
    fn with_invoker(exe_path: PathBuf, invoker: Box<dyn CodecHostInvoker>) -> Self {
        Self {
            exe_path,
            derived_profile_cache_path: None,
            invoker,
        }
    }

    #[cfg(test)]
    fn with_invoker_and_cache(
        exe_path: PathBuf,
        derived_profile_cache_path: PathBuf,
        invoker: Box<dyn CodecHostInvoker>,
    ) -> Self {
        Self {
            exe_path,
            derived_profile_cache_path: Some(derived_profile_cache_path),
            invoker,
        }
    }

    #[cfg(test)]
    fn recorded_requests_for_tests(&self) -> Vec<Value> {
        self.invoker.recorded_requests()
    }

    /// Build a backend whose host responses are produced by a closure that
    /// dispatches on the request's `command` field. Test-only seam used by the
    /// core crate's tests to exercise `probe`/`calibrate` flows without a real
    /// helper process.
    #[cfg(test)]
    pub(crate) fn with_command_dispatch_for_tests<F>(
        exe_path: impl Into<PathBuf>,
        dispatch: F,
    ) -> Self
    where
        F: Fn(&str) -> Result<Value, CoreError> + Send + Sync + 'static,
    {
        Self {
            exe_path: exe_path.into(),
            derived_profile_cache_path: None,
            invoker: Box::new(DispatchingCodecHostInvoker {
                dispatch: Box::new(dispatch),
            }),
        }
    }
}

#[cfg(test)]
struct DispatchingCodecHostInvoker {
    dispatch: Box<dyn Fn(&str) -> Result<Value, CoreError> + Send + Sync>,
}

#[cfg(test)]
impl CodecHostInvoker for DispatchingCodecHostInvoker {
    fn invoke(&self, request: Value) -> Result<Value, CoreError> {
        let command = request
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();
        (self.dispatch)(command)
    }
}

impl CodecBackend for G1rBinaryHostBackend {
    fn probe(&self) -> Result<CodecBackendProbe, CoreError> {
        let data = self.invoker.invoke(self.request(json!({
            "command": "probe",
        })))?;
        Ok(Self::probe_from_response(data))
    }

    fn decompress(&self, input: &[u8], expected_size: usize) -> Result<Vec<u8>, CoreError> {
        let data = self.invoker.invoke(self.request(json!({
            "command": "decompress",
            "inputBase64": BASE64_STANDARD.encode(input),
            "expectedSize": expected_size
        })))?;
        let output = decode_output_base64("decompress", &data)?;
        if output.len() != expected_size {
            return Err(CoreError::Codec(format!(
                "codec host decompress returned {} bytes, expected {expected_size}",
                output.len()
            )));
        }
        Ok(output)
    }

    fn decompress_many(&self, chunks: &[CodecDecodeChunk<'_>]) -> Result<Vec<Vec<u8>>, CoreError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }
        let request_chunks = chunks
            .iter()
            .map(|chunk| {
                json!({
                    "inputBase64": BASE64_STANDARD.encode(chunk.input),
                    "expectedSize": chunk.expected_size
                })
            })
            .collect::<Vec<_>>();
        let data = self.invoker.invoke(self.request(json!({
            "command": "decompress_many",
            "chunks": request_chunks
        })))?;
        let outputs = decode_outputs_base64("decompress_many", &data)?;
        if outputs.len() != chunks.len() {
            return Err(CoreError::Codec(format!(
                "codec host decompress_many returned {} chunks, expected {}",
                outputs.len(),
                chunks.len()
            )));
        }
        for (index, (output, chunk)) in outputs.iter().zip(chunks).enumerate() {
            if output.len() != chunk.expected_size {
                return Err(CoreError::Codec(format!(
                    "codec host decompress_many chunk {index} returned {} bytes, expected {}",
                    output.len(),
                    chunk.expected_size
                )));
            }
        }
        Ok(outputs)
    }

    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, CoreError> {
        let data = self.invoker.invoke(self.request(json!({
            "command": "compress",
            "inputBase64": BASE64_STANDARD.encode(input),
            "level": level
        })))?;
        decode_output_base64("compress", &data)
    }

    fn compress_many(&self, chunks: &[CodecEncodeChunk<'_>]) -> Result<Vec<Vec<u8>>, CoreError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }
        let request_chunks = chunks
            .iter()
            .map(|chunk| {
                json!({
                    "inputBase64": BASE64_STANDARD.encode(chunk.input),
                    "level": chunk.level
                })
            })
            .collect::<Vec<_>>();
        let data = self.invoker.invoke(self.request(json!({
            "command": "compress_many",
            "chunks": request_chunks
        })))?;
        let outputs = decode_outputs_base64("compress_many", &data)?;
        if outputs.len() != chunks.len() {
            return Err(CoreError::Codec(format!(
                "codec host compress_many returned {} chunks, expected {}",
                outputs.len(),
                chunks.len()
            )));
        }
        Ok(outputs)
    }
}

impl G1rBinaryHostBackend {
    pub fn calibrate(&self) -> Result<CodecBackendProbe, CoreError> {
        let data = self.invoker.invoke(self.request(json!({
            "command": "calibrate",
        })))?;
        Ok(Self::probe_from_response(data))
    }

    /// Map a `probe`/`calibrate` host response into a `CodecBackendProbe`.
    /// Shared so the two commands can never drift in how they interpret the
    /// response fields.
    fn probe_from_response(data: Value) -> CodecBackendProbe {
        let supported = data
            .get("supported")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: supported,
            can_decompress: data
                .get("canDecompress")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            can_compress: data
                .get("canCompress")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            status: if supported {
                "supported".to_string()
            } else {
                "unsupported".to_string()
            },
            profile: data
                .get("profile")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            resolution_mode: data
                .get("resolutionMode")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            details: data,
        }
    }

    fn request(&self, mut value: Value) -> Value {
        value["exePath"] = json!(self.exe_path.display().to_string());
        if let Some(cache_path) = &self.derived_profile_cache_path {
            value["derivedProfileCachePath"] = json!(cache_path.display().to_string());
        }
        value
    }
}

trait CodecHostInvoker: Send + Sync {
    fn invoke(&self, request: Value) -> Result<Value, CoreError>;

    #[cfg(test)]
    fn recorded_requests(&self) -> Vec<Value> {
        Vec::new()
    }
}

struct ProcessCodecHostInvoker {
    helper_path: PathBuf,
}

impl CodecHostInvoker for ProcessCodecHostInvoker {
    fn invoke(&self, request: Value) -> Result<Value, CoreError> {
        invoke_codec_host_stdio(&self.helper_path, request)
    }
}

/// Suppress the console window Windows would otherwise flash for each codec
/// host invocation (e.g. on every save inspect). No-op on other platforms.
#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}

fn invoke_codec_host_stdio(helper_path: &Path, request: Value) -> Result<Value, CoreError> {
    let request_line =
        serde_json::to_vec(&request).map_err(|err| CoreError::Codec(err.to_string()))?;
    let mut command = Command::new(helper_path);
    command
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console_window(&mut command);
    let mut child = command.spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CoreError::Codec("codec host stdin was not available".to_string()))?;
    stdin.write_all(&request_line)?;
    stdin.write_all(b"\n")?;
    drop(stdin);

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(CoreError::Codec(format!(
            "codec host exited with status {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| CoreError::Codec(format!("codec host stdout was not valid UTF-8: {err}")))?;
    let response_line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| CoreError::Codec("codec host returned no response".to_string()))?;
    parse_codec_host_response(response_line)
}

fn parse_codec_host_response(line: &str) -> Result<Value, CoreError> {
    let envelope: Value = serde_json::from_str(line)
        .map_err(|err| CoreError::Codec(format!("codec host response JSON is invalid: {err}")))?;
    let ok = envelope
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| CoreError::Codec("codec host response missing ok flag".to_string()))?;
    if ok {
        return envelope
            .get("data")
            .cloned()
            .ok_or_else(|| CoreError::Codec("codec host response missing data".to_string()));
    }

    let error = envelope.get("error").unwrap_or(&Value::Null);
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("codec host request failed");
    Err(CoreError::Codec(format!("codec host {code}: {message}")))
}

fn decode_output_base64(command: &str, data: &Value) -> Result<Vec<u8>, CoreError> {
    let output_base64 = data
        .get("outputBase64")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CoreError::Codec(format!(
                "codec host {command} response missing outputBase64"
            ))
        })?;
    BASE64_STANDARD.decode(output_base64).map_err(|err| {
        CoreError::Codec(format!(
            "codec host {command} outputBase64 is invalid: {err}"
        ))
    })
}

fn decode_outputs_base64(command: &str, data: &Value) -> Result<Vec<Vec<u8>>, CoreError> {
    let outputs_base64 = data
        .get("outputsBase64")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoreError::Codec(format!(
                "codec host {command} response missing outputsBase64"
            ))
        })?;
    outputs_base64
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let output_base64 = value.as_str().ok_or_else(|| {
                CoreError::Codec(format!(
                    "codec host {command} outputsBase64[{index}] is not a string"
                ))
            })?;
            BASE64_STANDARD.decode(output_base64).map_err(|err| {
                CoreError::Codec(format!(
                    "codec host {command} outputsBase64[{index}] is invalid: {err}"
                ))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoreError;
    use serde_json::{Value, json};
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct RecordingInvoker {
        requests: Mutex<Vec<Value>>,
        responses: Mutex<VecDeque<Result<Value, CoreError>>>,
    }

    impl RecordingInvoker {
        fn with_responses(responses: Vec<Result<Value, CoreError>>) -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                responses: Mutex::new(responses.into()),
            }
        }

        fn requests(&self) -> Vec<Value> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl CodecHostInvoker for RecordingInvoker {
        fn invoke(&self, request: Value) -> Result<Value, CoreError> {
            self.requests.lock().unwrap().push(request);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("test response queued")
        }

        fn recorded_requests(&self) -> Vec<Value> {
            self.requests()
        }
    }

    #[test]
    fn pure_rust_backend_reports_current_status() {
        let backend = PureRustKrakenBackend;

        let probe = backend.probe().unwrap();

        assert_eq!(probe.backend, "pure_rust_kraken");
        assert!(!probe.available);
        assert!(!probe.can_decompress);
        assert!(!probe.can_compress);
        assert_eq!(probe.status, "native_encoder_in_progress");
    }

    #[test]
    fn binary_host_probe_maps_supported_response_to_backend_probe() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "supported": true,
            "profile": "g1r-23A85CE7",
            "resolutionMode": "known_profile",
            "canCompress": true,
            "canDecompress": true
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker(
            PathBuf::from("G1R-Win64-Shipping.exe"),
            Box::new(invoker),
        );

        let probe = backend.probe().unwrap();

        assert_eq!(probe.backend, "g1r_binary_host");
        assert!(probe.available);
        assert!(probe.can_decompress);
        assert!(probe.can_compress);
        assert_eq!(probe.profile.as_deref(), Some("g1r-23A85CE7"));
        assert_eq!(probe.resolution_mode.as_deref(), Some("known_profile"));
    }

    #[test]
    fn binary_host_calibrate_maps_promoted_response() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "supported": true,
            "profile": "g1r-derived-77f3d48c",
            "resolutionMode": "derived_profile_cache",
            "canCompress": true,
            "canDecompress": true,
            "calibrationRan": true
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker(
            PathBuf::from("G1R-Win64-Shipping.exe"),
            Box::new(invoker),
        );

        let probe = backend.calibrate().unwrap();

        assert!(probe.available);
        assert!(probe.can_compress);
        assert_eq!(
            probe.resolution_mode.as_deref(),
            Some("derived_profile_cache")
        );
    }

    #[test]
    fn binary_host_probe_forwards_configured_derived_profile_cache_path() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "supported": true,
            "profile": "g1r-23A85CE7",
            "resolutionMode": "derived_profile_cache",
            "canCompress": true,
            "canDecompress": true
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker_and_cache(
            PathBuf::from("D:\\G1R-Win64-Shipping.exe"),
            PathBuf::from("C:\\Users\\Daniel\\AppData\\Local\\goresave\\profiles.json"),
            Box::new(invoker),
        );

        let probe = backend.probe().unwrap();

        assert_eq!(
            probe.resolution_mode.as_deref(),
            Some("derived_profile_cache")
        );
        assert_eq!(
            backend.recorded_requests_for_tests(),
            vec![json!({
                "command": "probe",
                "exePath": "D:\\G1R-Win64-Shipping.exe",
                "derivedProfileCachePath": "C:\\Users\\Daniel\\AppData\\Local\\goresave\\profiles.json"
            })]
        );
    }

    #[test]
    fn binary_host_decompress_sends_expected_json_and_decodes_output_base64() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "outputBase64": "AQIDBA==",
            "profile": "g1r-23A85CE7",
            "resolutionMode": "known_profile"
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker(
            PathBuf::from("D:\\G1R-Win64-Shipping.exe"),
            Box::new(invoker),
        );

        let output = backend.decompress(&[0xAA, 0xBB], 4).unwrap();

        assert_eq!(output, vec![1, 2, 3, 4]);
        assert_eq!(
            backend.recorded_requests_for_tests(),
            vec![json!({
                "command": "decompress",
                "exePath": "D:\\G1R-Win64-Shipping.exe",
                "inputBase64": "qrs=",
                "expectedSize": 4
            })]
        );
    }

    #[test]
    fn binary_host_decompress_many_sends_one_batch_request_and_decodes_outputs() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "outputsBase64": ["AQID", "BAUG"],
            "profile": "g1r-23A85CE7",
            "resolutionMode": "known_profile"
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker(
            PathBuf::from("D:\\G1R-Win64-Shipping.exe"),
            Box::new(invoker),
        );

        let output = backend
            .decompress_many(&[
                CodecDecodeChunk {
                    input: &[0xAA, 0xBB],
                    expected_size: 3,
                },
                CodecDecodeChunk {
                    input: &[0xCC, 0xDD],
                    expected_size: 3,
                },
            ])
            .unwrap();

        assert_eq!(output, vec![vec![1, 2, 3], vec![4, 5, 6]]);
        assert_eq!(
            backend.recorded_requests_for_tests(),
            vec![json!({
                "command": "decompress_many",
                "exePath": "D:\\G1R-Win64-Shipping.exe",
                "chunks": [
                    {
                        "inputBase64": "qrs=",
                        "expectedSize": 3
                    },
                    {
                        "inputBase64": "zN0=",
                        "expectedSize": 3
                    }
                ]
            })]
        );
    }

    #[test]
    fn binary_host_compress_sends_level_and_decodes_output_base64() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "outputBase64": "3q2+7w==",
            "profile": "g1r-23A85CE7",
            "resolutionMode": "known_profile"
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker(
            PathBuf::from("D:\\G1R-Win64-Shipping.exe"),
            Box::new(invoker),
        );

        let output = backend.compress(&[1, 2, 3], 6).unwrap();

        assert_eq!(output, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(
            backend.recorded_requests_for_tests(),
            vec![json!({
                "command": "compress",
                "exePath": "D:\\G1R-Win64-Shipping.exe",
                "inputBase64": "AQID",
                "level": 6
            })]
        );
    }

    #[test]
    fn binary_host_compress_many_sends_one_batch_request_and_decodes_outputs() {
        let invoker = RecordingInvoker::with_responses(vec![Ok(json!({
            "outputsBase64": ["AQID", "BAUG"],
            "profile": "g1r-23A85CE7",
            "resolutionMode": "known_profile"
        }))]);
        let backend = G1rBinaryHostBackend::with_invoker(
            PathBuf::from("D:\\G1R-Win64-Shipping.exe"),
            Box::new(invoker),
        );

        let output = backend
            .compress_many(&[
                CodecEncodeChunk {
                    input: &[1, 2, 3],
                    level: 6,
                },
                CodecEncodeChunk {
                    input: &[4, 5, 6],
                    level: 6,
                },
            ])
            .unwrap();

        assert_eq!(output, vec![vec![1, 2, 3], vec![4, 5, 6]]);
        assert_eq!(
            backend.recorded_requests_for_tests(),
            vec![json!({
                "command": "compress_many",
                "exePath": "D:\\G1R-Win64-Shipping.exe",
                "chunks": [
                    {
                        "inputBase64": "AQID",
                        "level": 6
                    },
                    {
                        "inputBase64": "BAUG",
                        "level": 6
                    }
                ]
            })]
        );
    }

    #[test]
    fn codec_host_error_response_becomes_core_codec_error() {
        let err = parse_codec_host_response(
            r#"{
            "ok": false,
            "error": {
                "code": "unsupported_exe",
                "message": "no verified codec functions"
            }
        }"#,
        )
        .unwrap_err();

        assert!(matches!(err, CoreError::Codec(_)));
        assert!(
            err.to_string()
                .contains("codec host unsupported_exe: no verified codec functions")
        );
    }
}
