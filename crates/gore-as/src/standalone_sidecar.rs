//! Process-safe Protocol-v1 adapter for the native standalone compiler sidecar.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::compile::{
    StandaloneCompilerInputsV1, StandaloneCompilerOutputV1, StandaloneCompilerOverlayOperationV1,
    StandaloneCompilerOverlayV1, StandaloneCompilerRunnerV1,
};
use crate::compiler_backend::{CompilerBackendFailureKindV1, CompilerBackendFailureV1};
use crate::compiler_profile::frontend::{
    validate_frontend_profile_payloads, MAX_CLASS_GENERATOR_CONFIG_BYTES_V1,
    MAX_COMPILER_OPTIONS_BYTES_V1, MAX_PREPROCESSOR_CONFIG_BYTES_V1,
};
use crate::compiler_profile::manifest::{
    CompilerProfileV1, FileSealV1, SealedBlobV1, Sha256Digest, MAX_COMPILER_PROFILE_JSON_BYTES,
};
use crate::compiler_profile::qualification::{
    validate_qualification_payloads, MAX_QUALIFICATION_JSON_BYTES_V1,
};
use crate::compiler_profile::registry::validate_engine_profile_payloads;

pub const SIDECAR_REQUEST_VERSION_V1: u32 = 1;
pub const SIDECAR_RESPONSE_VERSION_V1: u32 = 1;
pub const MAX_SIDECAR_REQUEST_BYTES_V1: usize = 1024 * 1024;
pub const MAX_SIDECAR_RESPONSE_BYTES_V1: usize = 64 * 1024;
pub const MAX_SIDECAR_STDERR_BYTES_V1: usize = 64 * 1024;
pub const MAX_SIDECAR_DIAGNOSTICS_V1: usize = 64;
pub const MAX_SIDECAR_DIAGNOSTIC_MESSAGE_BYTES_V1: usize = 2 * 1024;
pub const MAX_SIDECAR_SOURCE_FILES_V1: usize = 4_096;
pub const MAX_SIDECAR_SOURCE_FILE_BYTES_V1: u64 = 16 * 1024 * 1024;
pub const MAX_SIDECAR_SOURCE_BYTES_V1: u64 = 256 * 1024 * 1024;
pub const MAX_SIDECAR_OVERLAY_MODULES_V1: usize = 1_024;
pub const MAX_SIDECAR_MODULE_IDENTITY_BYTES_V1: usize = 4 * 1024;
const MAX_SIDECAR_BASE_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_SIDECAR_BINDS_BYTES_V1: u64 = 128 * 1024 * 1024;
const MAX_SIDECAR_OUTPUT_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_SIDECAR_EXECUTABLE_BYTES_V1: u64 = 256 * 1024 * 1024;
const MAX_PROFILE_BLOB_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_PROFILE_AGGREGATE_BYTES_V1: u64 = 1024 * 1024 * 1024;
const MAX_ENGINE_PROPERTIES_BYTES_V1: u64 = 1024 * 1024;
const MAX_REGISTRATION_TRACE_BYTES_V1: u64 = 256 * 1024 * 1024;
const MAX_POST_BIND_SNAPSHOT_BYTES_V1: u64 = 128 * 1024 * 1024;
const DEFAULT_SIDECAR_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const DEFAULT_TERMINATION_GRACE: Duration = Duration::from_secs(5);
const DEFAULT_SIDECAR_MEMORY_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MIN_SIDECAR_MEMORY_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SIDECAR_MEMORY_LIMIT_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
const ENGINE_UNAVAILABLE_CODE: &str = "GORE_AS_STANDALONE_ENGINE_UNAVAILABLE";
const SCRATCH_PREFIX: &str = "gore-as-sidecar-v1-";
static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Paths and process bounds for one standalone sidecar instance.
#[derive(Debug, Clone)]
pub struct StandaloneSidecarConfigV1 {
    pub sidecar_path: PathBuf,
    pub sidecar_seal: SidecarExecutableSealV1,
    pub profile_manifest_path: PathBuf,
    pub profile_root: PathBuf,
    pub scratch_root: PathBuf,
    pub timeout: Duration,
    pub termination_grace: Duration,
    /// Hard per-process and aggregate process-tree memory ceiling.
    pub memory_limit_bytes: u64,
    fixed_args: Vec<OsString>,
}

/// Package-authored identity of the exact sidecar binary that may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidecarExecutableSealV1 {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

impl StandaloneSidecarConfigV1 {
    pub fn new(
        sidecar_path: PathBuf,
        sidecar_seal: SidecarExecutableSealV1,
        profile_manifest_path: PathBuf,
        profile_root: PathBuf,
        scratch_root: PathBuf,
    ) -> Self {
        Self {
            sidecar_path,
            sidecar_seal,
            profile_manifest_path,
            profile_root,
            scratch_root,
            timeout: DEFAULT_SIDECAR_TIMEOUT,
            termination_grace: DEFAULT_TERMINATION_GRACE,
            memory_limit_bytes: DEFAULT_SIDECAR_MEMORY_LIMIT_BYTES,
            fixed_args: Vec::new(),
        }
    }

    #[cfg(test)]
    fn with_test_fixed_args(mut self, args: impl IntoIterator<Item = OsString>) -> Self {
        self.fixed_args = args.into_iter().collect();
        self
    }
}

/// Opaque proof that a compiler-profile package passed every typed qualification gate.
///
/// Construction reads the exact manifest plus all registry, frontend, and differential-parity
/// payloads from `profile_root`. Product code must pass this handle, rather than a raw manifest,
/// when it creates authority-bearing generation evidence for either backend.
#[derive(Debug)]
pub struct ValidatedCompilerProfilePackageV1 {
    profile: CompilerProfileV1,
}

impl ValidatedCompilerProfilePackageV1 {
    pub fn load(
        profile_manifest_path: &Path,
        profile_root: &Path,
    ) -> Result<Self, CompilerBackendFailureV1> {
        require_absolute(profile_manifest_path, "compiler profile manifest")?;
        require_absolute(profile_root, "compiler profile root")?;
        ensure_real_directory(profile_root, "compiler profile root")?;
        let manifest = read_regular_bounded_no_follow(
            profile_manifest_path,
            MAX_COMPILER_PROFILE_JSON_BYTES as u64,
            "compiler profile manifest",
        )
        .map_err(|error| unavailable(error.to_string()))?;
        let profile = CompilerProfileV1::from_json(&manifest)
            .map_err(|error| unavailable(format!("compiler profile is not qualified: {error}")))?;
        validate_typed_profile_payloads_at_root(&profile, profile_root)?;
        Ok(Self { profile })
    }

    pub fn profile(&self) -> &CompilerProfileV1 {
        &self.profile
    }
}

/// Native sidecar runner with a fully parsed and qualified compiler profile.
#[derive(Debug)]
pub struct StandaloneSidecarRunnerV1 {
    config: StandaloneSidecarConfigV1,
    profile_package: ValidatedCompilerProfilePackageV1,
}

impl StandaloneSidecarRunnerV1 {
    pub fn new(config: StandaloneSidecarConfigV1) -> Result<Self, CompilerBackendFailureV1> {
        require_absolute(&config.sidecar_path, "sidecar executable")?;
        require_absolute(&config.profile_manifest_path, "compiler profile manifest")?;
        require_absolute(&config.profile_root, "compiler profile root")?;
        require_absolute(&config.scratch_root, "sidecar scratch root")?;
        if !(MIN_SIDECAR_MEMORY_LIMIT_BYTES..=MAX_SIDECAR_MEMORY_LIMIT_BYTES)
            .contains(&config.memory_limit_bytes)
        {
            return Err(unavailable(format!(
                "sidecar memory limit must be between {MIN_SIDECAR_MEMORY_LIMIT_BYTES} and \
                 {MAX_SIDECAR_MEMORY_LIMIT_BYTES} bytes"
            )));
        }
        ensure_real_directory(&config.scratch_root, "sidecar scratch root")?;
        let mut sidecar = open_regular_no_follow(&config.sidecar_path, "sidecar executable")
            .map_err(unavailable)?;
        verify_open_sidecar_seal(&mut sidecar, config.sidecar_seal)?;
        let profile_package = ValidatedCompilerProfilePackageV1::load(
            &config.profile_manifest_path,
            &config.profile_root,
        )?;
        Ok(Self {
            config,
            profile_package,
        })
    }

    pub fn profile(&self) -> &CompilerProfileV1 {
        self.profile_package.profile()
    }

    pub fn profile_package(&self) -> &ValidatedCompilerProfilePackageV1 {
        &self.profile_package
    }
}

impl StandaloneCompilerRunnerV1 for StandaloneSidecarRunnerV1 {
    fn run_regen(
        &mut self,
        inputs: StandaloneCompilerInputsV1<'_>,
    ) -> Result<StandaloneCompilerOutputV1, CompilerBackendFailureV1> {
        let base_cache = inputs.base_cache.ok_or_else(|| {
            unavailable("standalone sidecar requires a sealed base-cache snapshot")
        })?;
        let binds_cache = inputs.binds_cache.ok_or_else(|| {
            unavailable("standalone sidecar requires a sealed Binds.Cache snapshot")
        })?;
        verify_memory_seal(
            "base cache",
            base_cache,
            &self.profile().oracle.shipping_cache,
            MAX_SIDECAR_BASE_BYTES_V1,
        )?;
        verify_memory_seal(
            "Binds.Cache",
            binds_cache,
            &self.profile().oracle.binds_cache,
            MAX_SIDECAR_BINDS_BYTES_V1,
        )?;
        self.profile().validate_complete().map_err(|error| {
            unavailable(format!("compiler profile is no longer valid: {error}"))
        })?;

        let mut sidecar_handle =
            open_regular_no_follow(&self.config.sidecar_path, "sidecar executable")
                .map_err(unavailable)?;
        verify_open_sidecar_seal(&mut sidecar_handle, self.config.sidecar_seal)?;
        let mut scratch = ScratchDirectory::create(&self.config.scratch_root)?;
        let staged_profile_root = scratch.path.join("profile");
        let staged_sources = scratch.path.join("sources");
        let staged_inputs = scratch.path.join("inputs");
        let staged_output = scratch.path.join("output");
        for directory in [
            &staged_profile_root,
            &staged_sources,
            &staged_inputs,
            &staged_output,
        ] {
            std::fs::create_dir(directory)
                .map_err(|error| internal(format!("creating {}: {error}", directory.display())))?;
        }

        let staged_manifest = stage_profile(
            self.profile(),
            &self.config.profile_root,
            &staged_profile_root,
        )?;
        let staged_base = staged_inputs.join("PrecompiledScript_Shipping.Cache");
        write_new_readonly(&staged_base, base_cache, "staged base cache")?;
        let staged_binds = staged_inputs.join("Binds.Cache");
        write_new_readonly(&staged_binds, binds_cache, "staged Binds.Cache")?;
        let source_files = stage_source_tree(inputs.source_tree, &staged_sources)?;
        let overlays = stage_overlay_manifest(inputs.overlays, &source_files)?;
        let output_path = staged_output.join("PrecompiledScript.Cache");
        if output_path.exists() {
            return Err(internal(
                "sidecar output path unexpectedly exists before launch",
            ));
        }

        let request = SidecarCompileRequestV1 {
            request_version: SIDECAR_REQUEST_VERSION_V1,
            operation: SidecarOperationV1::Compile,
            profile: SidecarProfileIdentityV1 {
                manifest_path: json_path(&staged_manifest, "staged profile manifest")?,
                profile_root: json_path(&staged_profile_root, "staged profile root")?,
                profile_sha256: self.profile().profile_sha256,
                steam_build_id: self.profile().target.steam_build_id,
                depot_id: self.profile().target.depot_id,
                depot_manifest_gid: self.profile().target.depot_manifest_gid,
                required_probe_suite_version: self
                    .profile()
                    .qualification
                    .required_probe_suite_version
                    .clone(),
            },
            inputs: SidecarInputsV1 {
                base_cache: sealed_path(&staged_base, base_cache, "staged base cache")?,
                binds_cache: sealed_path(&staged_binds, binds_cache, "staged Binds.Cache")?,
                source_tree: SidecarSourceTreeV1 {
                    root: json_path(&staged_sources, "staged source root")?,
                    files: source_files,
                },
                overlays,
            },
            output: SidecarOutputRequestV1 {
                cache_path: json_path(&output_path, "sidecar output")?,
            },
        };
        let request_bytes = serde_json::to_vec(&request)
            .map_err(|error| internal(format!("serializing sidecar request: {error}")))?;
        if request_bytes.len() > MAX_SIDECAR_REQUEST_BYTES_V1 {
            return Err(internal(format!(
                "sidecar request has {} bytes; maximum is {}",
                request_bytes.len(),
                MAX_SIDECAR_REQUEST_BYTES_V1
            )));
        }
        let request_path = scratch.path.join("request-v1.json");
        write_new_readonly(&request_path, &request_bytes, "sidecar request")?;
        validate_regular_no_reparse(&request_path, "sidecar request")?;

        let completed =
            run_sidecar_process(&self.config, &request_path, &scratch.path, sidecar_handle)?;
        let response = parse_sidecar_response(&completed, &output_path, self.profile())?;
        verify_output(&output_path, &response)?;
        set_readonly(&output_path, true).map_err(|error| {
            invalid_output(format!("sealing sidecar output read-only: {error}"))
        })?;
        validate_regular_no_reparse(&output_path, "sidecar output")?;

        Ok(scratch.retain_output(output_path))
    }
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarCompileRequestV1 {
    request_version: u32,
    operation: SidecarOperationV1,
    profile: SidecarProfileIdentityV1,
    inputs: SidecarInputsV1,
    output: SidecarOutputRequestV1,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SidecarOperationV1 {
    Compile,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarProfileIdentityV1 {
    manifest_path: String,
    profile_root: String,
    profile_sha256: Sha256Digest,
    steam_build_id: u64,
    depot_id: u32,
    depot_manifest_gid: u64,
    required_probe_suite_version: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarInputsV1 {
    base_cache: SidecarSealedPathV1,
    binds_cache: SidecarSealedPathV1,
    source_tree: SidecarSourceTreeV1,
    overlays: Vec<SidecarOverlayModuleV1>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarOverlayModuleV1 {
    ordinal: u32,
    operation: SidecarOverlayOperationV1,
    module_name: String,
    relative_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SidecarOverlayOperationV1 {
    Add,
    Edit,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarSealedPathV1 {
    path: String,
    byte_len: u64,
    sha256: Sha256Digest,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarSourceTreeV1 {
    root: String,
    files: Vec<SidecarSourceFileV1>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarSourceFileV1 {
    path: String,
    byte_len: u64,
    sha256: Sha256Digest,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarOutputRequestV1 {
    cache_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarCompileResponseV1 {
    response_version: u32,
    ok: bool,
    #[serde(default)]
    failure_kind: Option<SidecarFailureKindV1>,
    #[serde(default)]
    output: Option<SidecarOutputResponseV1>,
    diagnostics: Vec<SidecarDiagnosticV1>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SidecarFailureKindV1 {
    EngineUnavailable,
    Rejected,
    InvalidOutput,
    Internal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarOutputResponseV1 {
    cache_path: String,
    byte_len: u64,
    sha256: Sha256Digest,
    profile_sha256: Sha256Digest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarDiagnosticV1 {
    severity: SidecarDiagnosticSeverityV1,
    code: String,
    message: String,
    #[serde(default)]
    source_path: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SidecarDiagnosticSeverityV1 {
    Info,
    Warning,
    Error,
}

struct CompletedSidecarProcess {
    status: ExitStatus,
    stdout: CapturedStream,
    stderr: CapturedStream,
}

struct CapturedStream {
    bytes: Vec<u8>,
    exceeded: bool,
    error: Option<String>,
}

fn run_sidecar_process(
    config: &StandaloneSidecarConfigV1,
    request_path: &Path,
    current_dir: &Path,
    _sidecar_handle: std::fs::File,
) -> Result<CompletedSidecarProcess, CompilerBackendFailureV1> {
    let mut command = Command::new(&config.sidecar_path);
    command
        .args(&config.fixed_args)
        .arg("compile")
        .arg("--request")
        .arg(request_path)
        .current_dir(current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command, config.memory_limit_bytes)?;
    let mut child = command
        .spawn()
        .map_err(|error| unavailable(format!("starting standalone sidecar: {error}")))?;
    let mut process_tree = ProcessTreeGuard::attach(&mut child, config.memory_limit_bytes)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| internal("sidecar stdout pipe was not created"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| internal("sidecar stderr pipe was not created"))?;
    let stdout_reader = spawn_bounded_reader(stdout, MAX_SIDECAR_RESPONSE_BYTES_V1);
    let stderr_reader = spawn_bounded_reader(stderr, MAX_SIDECAR_STDERR_BYTES_V1);

    let status = wait_for_sidecar(
        &mut child,
        &mut process_tree,
        config.timeout,
        config.termination_grace,
    )?;
    process_tree.disarm();
    let stdout = stdout_reader
        .join()
        .map_err(|_| internal("sidecar stdout reader panicked"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| internal("sidecar stderr reader panicked"))?;
    Ok(CompletedSidecarProcess {
        status,
        stdout,
        stderr,
    })
}

fn spawn_bounded_reader<R>(mut reader: R, limit: usize) -> std::thread::JoinHandle<CapturedStream>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
        let mut exceeded = false;
        let mut error = None;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let remaining = limit.saturating_sub(bytes.len());
                    let retained = remaining.min(read);
                    bytes.extend_from_slice(&buffer[..retained]);
                    exceeded |= retained != read;
                }
                Err(read_error) => {
                    error = Some(read_error.to_string());
                    break;
                }
            }
        }
        CapturedStream {
            bytes,
            exceeded,
            error,
        }
    })
}

fn wait_for_sidecar(
    child: &mut std::process::Child,
    process_tree: &mut ProcessTreeGuard,
    timeout: Duration,
    termination_grace: Duration,
) -> Result<ExitStatus, CompilerBackendFailureV1> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| internal("sidecar timeout is too large"))?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(
                    PROCESS_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(None) => {
                process_tree.terminate(child)?;
                let termination = match observe_terminated_child(child, termination_grace) {
                    Ok(status) => format!("terminated with {status}"),
                    Err(detail) => detail,
                };
                return Err(internal(format!(
                    "standalone sidecar exceeded {timeout:?}; {termination}"
                )));
            }
            Err(error) => {
                process_tree.terminate(child)?;
                return Err(internal(format!("waiting for standalone sidecar: {error}")));
            }
        }
    }
}

fn observe_terminated_child(
    child: &mut std::process::Child,
    grace: Duration,
) -> Result<ExitStatus, String> {
    let deadline = Instant::now()
        .checked_add(grace)
        .unwrap_or_else(Instant::now);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(PROCESS_POLL_INTERVAL);
            }
            Ok(None) => {
                return Err(format!("termination was not confirmed within {grace:?}"));
            }
            Err(error) => return Err(format!("confirming sidecar termination: {error}")),
        }
    }
}

fn parse_sidecar_response(
    completed: &CompletedSidecarProcess,
    expected_output: &Path,
    profile: &CompilerProfileV1,
) -> Result<SidecarOutputResponseV1, CompilerBackendFailureV1> {
    for (label, capture) in [("stdout", &completed.stdout), ("stderr", &completed.stderr)] {
        if let Some(error) = &capture.error {
            return Err(invalid_output(format!("reading sidecar {label}: {error}")));
        }
        if capture.exceeded {
            return Err(invalid_output(format!(
                "sidecar {label} exceeded its protocol limit"
            )));
        }
    }
    let stderr = std::str::from_utf8(&completed.stderr.bytes)
        .map_err(|error| invalid_output(format!("sidecar stderr is not UTF-8: {error}")))?;
    let response: SidecarCompileResponseV1 = serde_json::from_slice(&completed.stdout.bytes)
        .map_err(|error| {
            invalid_output(format!(
                "invalid sidecar response JSON: {error}{}",
                stderr_suffix(stderr)
            ))
        })?;
    if response.response_version != SIDECAR_RESPONSE_VERSION_V1 {
        return Err(invalid_output(format!(
            "sidecar response version {} is unsupported",
            response.response_version
        )));
    }
    validate_diagnostics(&response.diagnostics)?;
    let exit_code = completed.status.code();
    let engine_unavailable = exit_code == Some(69)
        || response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == ENGINE_UNAVAILABLE_CODE)
        || matches!(
            response.failure_kind,
            Some(SidecarFailureKindV1::EngineUnavailable)
        );
    if engine_unavailable {
        return Err(unavailable(format!(
            "{}{}",
            diagnostics_detail(&response.diagnostics),
            stderr_suffix(stderr)
        )));
    }

    if response.ok {
        if !completed.status.success() {
            return Err(invalid_output(format!(
                "sidecar returned ok=true with exit status {}{}",
                completed.status,
                stderr_suffix(stderr)
            )));
        }
        if response.failure_kind.is_some() {
            return Err(invalid_output(
                "sidecar returned ok=true together with failure_kind",
            ));
        }
        if response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == SidecarDiagnosticSeverityV1::Error)
        {
            return Err(invalid_output(
                "sidecar returned ok=true together with error diagnostics",
            ));
        }
        let output = response.output.ok_or_else(|| {
            invalid_output("sidecar returned ok=true without an output descriptor")
        })?;
        if Path::new(&output.cache_path) != expected_output {
            return Err(invalid_output(format!(
                "sidecar reported unexpected output path {:?}",
                output.cache_path
            )));
        }
        if output.profile_sha256 != profile.profile_sha256 {
            return Err(invalid_output(format!(
                "sidecar output profile identity {} does not match requested {}",
                output.profile_sha256, profile.profile_sha256
            )));
        }
        return Ok(output);
    }

    if completed.status.success() {
        return Err(invalid_output(
            "sidecar returned ok=false with a successful exit status",
        ));
    }
    if response.output.is_some() {
        return Err(invalid_output(
            "sidecar failure response unexpectedly exposed an output descriptor",
        ));
    }
    let detail = format!(
        "{}{}",
        diagnostics_detail(&response.diagnostics),
        stderr_suffix(stderr)
    );
    let kind = match response.failure_kind {
        Some(SidecarFailureKindV1::Rejected) => CompilerBackendFailureKindV1::Rejected,
        Some(SidecarFailureKindV1::InvalidOutput) => CompilerBackendFailureKindV1::InvalidOutput,
        Some(SidecarFailureKindV1::Internal) | None => CompilerBackendFailureKindV1::Internal,
        Some(SidecarFailureKindV1::EngineUnavailable) => CompilerBackendFailureKindV1::Unavailable,
    };
    Err(CompilerBackendFailureV1::new(kind, detail))
}

fn validate_diagnostics(
    diagnostics: &[SidecarDiagnosticV1],
) -> Result<(), CompilerBackendFailureV1> {
    if diagnostics.len() > MAX_SIDECAR_DIAGNOSTICS_V1 {
        return Err(invalid_output(format!(
            "sidecar returned {} diagnostics; maximum is {}",
            diagnostics.len(),
            MAX_SIDECAR_DIAGNOSTICS_V1
        )));
    }
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic.code.is_empty()
            || diagnostic.code.len() > 256
            || diagnostic.message.as_bytes().len() > MAX_SIDECAR_DIAGNOSTIC_MESSAGE_BYTES_V1
        {
            return Err(invalid_output(format!(
                "sidecar diagnostic {index} violates protocol bounds"
            )));
        }
        if diagnostic
            .source_path
            .as_ref()
            .is_some_and(|path| path.len() > 16 * 1024)
        {
            return Err(invalid_output(format!(
                "sidecar diagnostic {index} source path exceeds its bound"
            )));
        }
        if diagnostic.line == Some(0) || diagnostic.column == Some(0) {
            return Err(invalid_output(format!(
                "sidecar diagnostic {index} uses a zero source position"
            )));
        }
    }
    Ok(())
}

fn diagnostics_detail(diagnostics: &[SidecarDiagnosticV1]) -> String {
    if diagnostics.is_empty() {
        return "sidecar failed without diagnostics".to_owned();
    }
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn stderr_suffix(stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        String::new()
    } else {
        format!("; stderr: {stderr}")
    }
}

fn verify_output(
    path: &Path,
    response: &SidecarOutputResponseV1,
) -> Result<(), CompilerBackendFailureV1> {
    validate_regular_no_reparse(path, "sidecar output")?;
    let (byte_len, sha256) =
        hash_regular_file(path, MAX_SIDECAR_OUTPUT_BYTES_V1, "sidecar output")?;
    if byte_len != response.byte_len || sha256 != response.sha256 {
        return Err(invalid_output(format!(
            "sidecar output seal mismatch: response {} bytes/{}, actual {byte_len}/{sha256}",
            response.byte_len, response.sha256
        )));
    }
    Ok(())
}

fn stage_profile(
    profile: &CompilerProfileV1,
    source_root: &Path,
    destination_root: &Path,
) -> Result<PathBuf, CompilerBackendFailureV1> {
    let mut blobs = BTreeMap::<String, SealedBlobV1>::new();
    for blob in profile_blobs(profile) {
        blobs
            .entry(blob.path.clone())
            .or_insert_with(|| blob.clone());
    }
    let aggregate = blobs.values().try_fold(0u64, |total, blob| {
        if blob.byte_len > MAX_PROFILE_BLOB_BYTES_V1 {
            return Err(unavailable(format!(
                "compiler profile blob {:?} exceeds the sidecar bound",
                blob.path
            )));
        }
        total
            .checked_add(blob.byte_len)
            .filter(|sum| *sum <= MAX_PROFILE_AGGREGATE_BYTES_V1)
            .ok_or_else(|| unavailable("compiler profile blobs exceed the aggregate sidecar bound"))
    })?;
    let _ = aggregate;
    for blob in blobs.values() {
        let source = source_root.join(Path::new(&blob.path));
        let destination = destination_root.join(Path::new(&blob.path));
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                internal(format!("creating profile snapshot directory: {error}"))
            })?;
        }
        copy_verified_blob(&source, &destination, blob)?;
    }
    let manifest = serde_json::to_vec(profile)
        .map_err(|error| internal(format!("serializing compiler profile snapshot: {error}")))?;
    let path = destination_root.join("compiler-profile-v1.json");
    write_new_readonly(&path, &manifest, "compiler profile snapshot")?;
    Ok(path)
}

fn validate_typed_profile_payloads_at_root(
    profile: &CompilerProfileV1,
    profile_root: &Path,
) -> Result<(), CompilerBackendFailureV1> {
    let properties = read_sealed_profile_blob(
        profile_root,
        &profile.engine.ordered_engine_properties,
        MAX_ENGINE_PROPERTIES_BYTES_V1,
        "ordered engine properties",
    )?;
    let trace = read_sealed_profile_blob(
        profile_root,
        &profile.engine.registration_trace,
        MAX_REGISTRATION_TRACE_BYTES_V1,
        "registration trace",
    )?;
    let snapshot = read_sealed_profile_blob(
        profile_root,
        &profile.engine.post_bind_snapshot,
        MAX_POST_BIND_SNAPSHOT_BYTES_V1,
        "post-bind snapshot",
    )?;
    validate_engine_profile_payloads(&profile.engine, &properties, &trace, &snapshot)
        .map_err(|error| unavailable(format!("compiler registry profile is invalid: {error}")))?;

    let preprocessor = read_sealed_profile_blob(
        profile_root,
        &profile.frontend.preprocessor_config,
        MAX_PREPROCESSOR_CONFIG_BYTES_V1 as u64,
        "preprocessor config",
    )?;
    let class_generator = read_sealed_profile_blob(
        profile_root,
        &profile.frontend.class_generator_config,
        MAX_CLASS_GENERATOR_CONFIG_BYTES_V1 as u64,
        "class generator config",
    )?;
    let compiler_options = read_sealed_profile_blob(
        profile_root,
        &profile.frontend.compiler_options,
        MAX_COMPILER_OPTIONS_BYTES_V1 as u64,
        "compiler options",
    )?;
    validate_frontend_profile_payloads(
        &profile.frontend,
        &preprocessor,
        &class_generator,
        &compiler_options,
    )
    .map_err(|error| unavailable(format!("compiler frontend profile is invalid: {error}")))?;

    let probe_corpus = read_sealed_profile_blob(
        profile_root,
        &profile.bytecode.codegen_probe_corpus,
        MAX_QUALIFICATION_JSON_BYTES_V1 as u64,
        "compiler probe corpus",
    )?;
    let expected_results = read_sealed_profile_blob(
        profile_root,
        &profile.bytecode.expected_probe_results,
        MAX_QUALIFICATION_JSON_BYTES_V1 as u64,
        "expected compiler probe results",
    )?;
    let diagnostic_parity = read_sealed_profile_blob(
        profile_root,
        &profile.qualification.diagnostic_parity,
        MAX_QUALIFICATION_JSON_BYTES_V1 as u64,
        "compiler diagnostic parity",
    )?;
    let semantic_parity = read_sealed_profile_blob(
        profile_root,
        &profile.qualification.semantic_parity,
        MAX_QUALIFICATION_JSON_BYTES_V1 as u64,
        "compiler semantic parity",
    )?;
    validate_qualification_payloads(
        &profile.bytecode,
        &profile.qualification,
        &probe_corpus,
        &expected_results,
        &diagnostic_parity,
        &semantic_parity,
    )
    .map_err(|error| unavailable(format!("compiler qualification is invalid: {error}")))?;
    Ok(())
}

fn read_sealed_profile_blob(
    root: &Path,
    expected: &SealedBlobV1,
    max_bytes: u64,
    label: &'static str,
) -> Result<Vec<u8>, CompilerBackendFailureV1> {
    if expected.byte_len > max_bytes {
        return Err(unavailable(format!(
            "{label} is {} bytes; maximum accepted size is {max_bytes}",
            expected.byte_len
        )));
    }
    let bytes = read_regular_bounded_no_follow(&root.join(&expected.path), max_bytes, label)
        .map_err(|error| unavailable(error.to_string()))?;
    if bytes.len() as u64 != expected.byte_len || sha256_bytes(&bytes) != expected.sha256 {
        return Err(unavailable(format!(
            "{label} does not match its compiler-profile seal"
        )));
    }
    Ok(bytes)
}

fn profile_blobs(profile: &CompilerProfileV1) -> [&SealedBlobV1; 16] {
    [
        &profile.engine.ordered_engine_properties,
        &profile.engine.registration_trace,
        &profile.engine.post_bind_snapshot,
        &profile.unreal_semantics.reflected_type_graph,
        &profile.frontend.preprocessor_config,
        &profile.frontend.class_generator_config,
        &profile.frontend.compiler_options,
        &profile.bytecode.opcode_table,
        &profile.bytecode.operand_schema,
        &profile.bytecode.codegen_probe_corpus,
        &profile.bytecode.expected_probe_results,
        &profile.cache_writer.serializer_schema,
        &profile.cache_writer.reference_table_order,
        &profile.cache_writer.normalized_oracle_corpus,
        &profile.qualification.diagnostic_parity,
        &profile.qualification.semantic_parity,
    ]
}

fn copy_verified_blob(
    source: &Path,
    destination: &Path,
    expected: &SealedBlobV1,
) -> Result<(), CompilerBackendFailureV1> {
    let mut input = open_regular_no_follow(source, "compiler profile blob").map_err(unavailable)?;
    let metadata = input
        .metadata()
        .map_err(|error| unavailable(format!("inspecting profile blob: {error}")))?;
    if metadata.len() != expected.byte_len {
        return Err(unavailable(format!(
            "compiler profile blob {:?} length mismatch",
            expected.path
        )));
    }
    let mut output = open_create_new_no_follow(destination, "profile snapshot")?;
    let mut hash = Sha256::new();
    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| unavailable(format!("reading profile blob: {error}")))?;
        if read == 0 {
            break;
        }
        copied = copied
            .checked_add(read as u64)
            .ok_or_else(|| unavailable("profile blob length overflowed"))?;
        if copied > expected.byte_len {
            return Err(unavailable(format!(
                "compiler profile blob {:?} grew while being read",
                expected.path
            )));
        }
        hash.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|error| internal(format!("writing profile snapshot: {error}")))?;
    }
    output
        .sync_all()
        .map_err(|error| internal(format!("syncing profile snapshot: {error}")))?;
    drop(output);
    let digest = Sha256Digest::from_bytes(hash.finalize().into());
    if copied != expected.byte_len || digest != expected.sha256 {
        return Err(unavailable(format!(
            "compiler profile blob {:?} SHA-256 mismatch",
            expected.path
        )));
    }
    set_readonly(destination, true)
        .map_err(|error| internal(format!("sealing profile snapshot: {error}")))?;
    Ok(())
}

fn stage_overlay_manifest(
    overlays: &[StandaloneCompilerOverlayV1<'_>],
    source_files: &[SidecarSourceFileV1],
) -> Result<Vec<SidecarOverlayModuleV1>, CompilerBackendFailureV1> {
    if overlays.is_empty() || overlays.len() > MAX_SIDECAR_OVERLAY_MODULES_V1 {
        return Err(preflight(format!(
            "standalone overlay count must be between 1 and {}",
            MAX_SIDECAR_OVERLAY_MODULES_V1
        )));
    }

    let available_paths = source_files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut module_names = BTreeMap::<String, String>::new();
    let mut relative_paths = BTreeMap::<String, String>::new();
    let mut staged = Vec::with_capacity(overlays.len());
    for (ordinal, overlay) in overlays.iter().enumerate() {
        if overlay.module_name.is_empty()
            || overlay.module_name.len() > MAX_SIDECAR_MODULE_IDENTITY_BYTES_V1
            || overlay.module_name.contains('\0')
            || overlay.module_name.chars().any(char::is_control)
        {
            return Err(preflight(
                "standalone overlay has an invalid or oversized module name",
            ));
        }
        let relative_path = relative_json_path(Path::new(overlay.relative_path)).map_err(|_| {
            preflight("standalone overlay has an unsafe or non-Unicode relative path")
        })?;
        if relative_path.len() > MAX_SIDECAR_MODULE_IDENTITY_BYTES_V1 {
            return Err(preflight(
                "standalone overlay relative path exceeds the identity bound",
            ));
        }
        if !available_paths.contains(relative_path.as_str()) {
            return Err(preflight(format!(
                "standalone overlay source {relative_path:?} is absent from the sealed source tree"
            )));
        }

        let folded_name = overlay.module_name.to_lowercase();
        if let Some(previous) = module_names.insert(folded_name, overlay.module_name.to_owned()) {
            return Err(preflight(format!(
                "standalone overlays contain colliding module names {previous:?} and {:?}",
                overlay.module_name
            )));
        }
        let folded_path = relative_path.to_lowercase();
        if let Some(previous) = relative_paths.insert(folded_path, relative_path.clone()) {
            return Err(preflight(format!(
                "standalone overlays contain colliding relative paths {previous:?} and {relative_path:?}"
            )));
        }

        staged.push(SidecarOverlayModuleV1 {
            ordinal: ordinal as u32,
            operation: match overlay.operation {
                StandaloneCompilerOverlayOperationV1::Add => SidecarOverlayOperationV1::Add,
                StandaloneCompilerOverlayOperationV1::Edit => SidecarOverlayOperationV1::Edit,
            },
            module_name: overlay.module_name.to_owned(),
            relative_path,
        });
    }
    Ok(staged)
}

fn stage_source_tree(
    source_root: &Path,
    destination_root: &Path,
) -> Result<Vec<SidecarSourceFileV1>, CompilerBackendFailureV1> {
    ensure_real_directory(source_root, "emitted source tree")?;
    let mut pending = vec![(source_root.to_path_buf(), PathBuf::new())];
    let mut files = Vec::new();
    let mut aggregate = 0u64;
    while let Some((directory, relative_directory)) = pending.pop() {
        ensure_real_directory(&directory, "source-tree directory")?;
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| internal(format!("reading source tree: {error}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| internal(format!("reading source-tree entry: {error}")))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries.into_iter().rev() {
            let name = entry.file_name();
            let relative = relative_directory.join(&name);
            let source = entry.path();
            let metadata = std::fs::symlink_metadata(&source)
                .map_err(|error| internal(format!("inspecting source-tree entry: {error}")))?;
            if is_reparse_or_symlink(&metadata) {
                return Err(internal(format!(
                    "source tree contains a symlink/reparse point: {}",
                    source.display()
                )));
            }
            if metadata.is_dir() {
                let destination = destination_root.join(&relative);
                std::fs::create_dir(&destination).map_err(|error| {
                    internal(format!("creating staged source directory: {error}"))
                })?;
                pending.push((source, relative));
                continue;
            }
            if !metadata.is_file() {
                return Err(internal(format!(
                    "source tree contains a non-regular entry: {}",
                    source.display()
                )));
            }
            if files.len() == MAX_SIDECAR_SOURCE_FILES_V1 {
                return Err(internal("source tree exceeds the sidecar file-count bound"));
            }
            let bytes = read_regular_bounded_no_follow(
                &source,
                MAX_SIDECAR_SOURCE_FILE_BYTES_V1,
                "source file",
            )?;
            aggregate = aggregate
                .checked_add(bytes.len() as u64)
                .filter(|sum| *sum <= MAX_SIDECAR_SOURCE_BYTES_V1)
                .ok_or_else(|| internal("source tree exceeds the aggregate sidecar bound"))?;
            let destination = destination_root.join(&relative);
            write_new_readonly(&destination, &bytes, "staged source file")?;
            files.push(SidecarSourceFileV1 {
                path: relative_json_path(&relative)?,
                byte_len: bytes.len() as u64,
                sha256: sha256_bytes(&bytes),
            });
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn sealed_path(
    path: &Path,
    bytes: &[u8],
    label: &str,
) -> Result<SidecarSealedPathV1, CompilerBackendFailureV1> {
    Ok(SidecarSealedPathV1 {
        path: json_path(path, label)?,
        byte_len: bytes.len() as u64,
        sha256: sha256_bytes(bytes),
    })
}

fn verify_memory_seal(
    label: &str,
    bytes: &[u8],
    expected: &FileSealV1,
    max: u64,
) -> Result<(), CompilerBackendFailureV1> {
    if bytes.len() as u64 > max {
        return Err(unavailable(format!(
            "sealed {label} has {} bytes; maximum is {max}",
            bytes.len()
        )));
    }
    let actual = sha256_bytes(bytes);
    if bytes.len() as u64 != expected.byte_len || actual != expected.sha256 {
        return Err(unavailable(format!(
            "sealed {label} does not match compiler profile identity"
        )));
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn verify_open_sidecar_seal(
    file: &mut std::fs::File,
    expected: SidecarExecutableSealV1,
) -> Result<(), CompilerBackendFailureV1> {
    let metadata = file
        .metadata()
        .map_err(|error| unavailable(format!("inspecting sidecar executable: {error}")))?;
    if expected.byte_len == 0
        || expected.byte_len > MAX_SIDECAR_EXECUTABLE_BYTES_V1
        || metadata.len() != expected.byte_len
    {
        return Err(unavailable(
            "sidecar executable does not match its packaged length seal",
        ));
    }

    file.seek(SeekFrom::Start(0))
        .map_err(|error| unavailable(format!("seeking sidecar executable: {error}")))?;
    let mut hash = Sha256::new();
    let mut read_total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| unavailable(format!("reading sidecar executable: {error}")))?;
        if read == 0 {
            break;
        }
        read_total = read_total
            .checked_add(read as u64)
            .ok_or_else(|| unavailable("sidecar executable length overflowed"))?;
        if read_total > expected.byte_len {
            return Err(unavailable(
                "sidecar executable changed while its identity was checked",
            ));
        }
        hash.update(&buffer[..read]);
    }
    let actual = Sha256Digest::from_bytes(hash.finalize().into());
    if read_total != expected.byte_len || actual != expected.sha256 {
        return Err(unavailable(
            "sidecar executable does not match its packaged SHA-256 seal",
        ));
    }
    Ok(())
}

fn write_new_readonly(
    path: &Path,
    bytes: &[u8],
    label: &str,
) -> Result<(), CompilerBackendFailureV1> {
    let mut file = open_create_new_no_follow(path, label)?;
    file.write_all(bytes)
        .map_err(|error| internal(format!("writing {label}: {error}")))?;
    file.sync_all()
        .map_err(|error| internal(format!("syncing {label}: {error}")))?;
    drop(file);
    set_readonly(path, true)
        .map_err(|error| internal(format!("marking {label} read-only: {error}")))?;
    validate_regular_no_reparse(path, label)
}

fn read_regular_bounded_no_follow(
    path: &Path,
    max: u64,
    label: &str,
) -> Result<Vec<u8>, CompilerBackendFailureV1> {
    let mut file = open_regular_no_follow(path, label).map_err(internal)?;
    let length = file
        .metadata()
        .map_err(|error| internal(format!("inspecting {label}: {error}")))?
        .len();
    if length > max {
        return Err(internal(format!(
            "{label} has {length} bytes; maximum is {max}"
        )));
    }
    let mut bytes = Vec::with_capacity(length as usize);
    Read::by_ref(&mut file)
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| internal(format!("reading {label}: {error}")))?;
    if bytes.len() as u64 > max {
        return Err(internal(format!("{label} grew beyond {max} bytes")));
    }
    Ok(bytes)
}

fn hash_regular_file(
    path: &Path,
    max: u64,
    label: &str,
) -> Result<(u64, Sha256Digest), CompilerBackendFailureV1> {
    let mut file = open_regular_no_follow(path, label).map_err(invalid_output)?;
    let expected_len = file
        .metadata()
        .map_err(|error| invalid_output(format!("inspecting {label}: {error}")))?
        .len();
    if expected_len > max {
        return Err(invalid_output(format!(
            "{label} has {expected_len} bytes; maximum is {max}"
        )));
    }
    let mut hash = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| invalid_output(format!("reading {label}: {error}")))?;
        if read == 0 {
            break;
        }
        length += read as u64;
        if length > max {
            return Err(invalid_output(format!("{label} grew beyond {max} bytes")));
        }
        hash.update(&buffer[..read]);
    }
    if length != expected_len {
        return Err(invalid_output(format!("{label} changed while being read")));
    }
    Ok((length, Sha256Digest::from_bytes(hash.finalize().into())))
}

fn require_absolute(path: &Path, label: &str) -> Result<(), CompilerBackendFailureV1> {
    if !path.is_absolute() {
        return Err(unavailable(format!("{label} must use an absolute path")));
    }
    Ok(())
}

fn json_path(path: &Path, label: &str) -> Result<String, CompilerBackendFailureV1> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| internal(format!("{label} path is not valid Unicode")))
}

fn relative_json_path(path: &Path) -> Result<String, CompilerBackendFailureV1> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(internal("source tree produced an unsafe relative path"));
        };
        let part = part
            .to_str()
            .ok_or_else(|| internal("source path is not valid Unicode"))?;
        if part.is_empty() || part == "." || part == ".." || part.contains(['/', '\\', '\0']) {
            return Err(internal("source tree produced an unsafe path component"));
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return Err(internal("source tree produced an empty file path"));
    }
    Ok(parts.join("/"))
}

fn ensure_real_directory(path: &Path, label: &str) -> Result<(), CompilerBackendFailureV1> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| unavailable(format!("inspecting {label}: {error}")))?;
    if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
        return Err(unavailable(format!(
            "{label} must be a real non-reparse directory"
        )));
    }
    Ok(())
}

fn validate_regular_no_reparse(path: &Path, label: &str) -> Result<(), CompilerBackendFailureV1> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| invalid_output(format!("inspecting {label}: {error}")))?;
    if !metadata.is_file() || is_reparse_or_symlink(&metadata) {
        return Err(invalid_output(format!(
            "{label} must be a regular non-reparse file"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse_or_symlink(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_or_symlink(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path, label: &str) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| format!("opening {label} no-follow: {error}"))?;
    validate_open_file(&file, label)?;
    Ok(file)
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path, label: &str) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| format!("opening {label} no-follow: {error}"))?;
    validate_open_file(&file, label)?;
    Ok(file)
}

#[cfg(windows)]
fn open_create_new_no_follow(
    path: &Path,
    label: &str,
) -> Result<std::fs::File, CompilerBackendFailureV1> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| internal(format!("creating {label} no-follow: {error}")))?;
    validate_open_file(&file, label).map_err(internal)?;
    Ok(file)
}

#[cfg(unix)]
fn open_create_new_no_follow(
    path: &Path,
    label: &str,
) -> Result<std::fs::File, CompilerBackendFailureV1> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| internal(format!("creating {label} no-follow: {error}")))?;
    validate_open_file(&file, label).map_err(internal)?;
    Ok(file)
}

fn validate_open_file(file: &std::fs::File, label: &str) -> Result<(), String> {
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspecting opened {label}: {error}"))?;
    if !metadata.is_file() || is_reparse_or_symlink(&metadata) {
        return Err(format!("opened {label} is not a regular non-reparse file"));
    }
    Ok(())
}

fn set_readonly(path: &Path, readonly: bool) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    let mut permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut mode = permissions.mode();
        if readonly {
            mode &= !0o222;
        } else {
            mode |= 0o200;
        }
        permissions.set_mode(mode);
    }
    #[cfg(windows)]
    permissions.set_readonly(readonly);
    std::fs::set_permissions(path, permissions)
}

struct ScratchDirectory {
    path: PathBuf,
    root: PathBuf,
    armed: bool,
}

impl ScratchDirectory {
    fn create(root: &Path) -> Result<Self, CompilerBackendFailureV1> {
        ensure_real_directory(root, "sidecar scratch root")?;
        for _ in 0..32 {
            let sequence = SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = root.join(format!(
                "{SCRATCH_PREFIX}{}-{nanos}-{sequence}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        root: root.to_path_buf(),
                        armed: true,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(internal(format!(
                        "creating sidecar scratch directory: {error}"
                    )));
                }
            }
        }
        Err(internal(
            "could not allocate a unique sidecar scratch directory",
        ))
    }

    fn retain_output(&mut self, output_path: PathBuf) -> StandaloneCompilerOutputV1 {
        self.armed = false;
        let path = self.path.clone();
        let root = self.root.clone();
        StandaloneCompilerOutputV1::with_cleanup(output_path, move || {
            cleanup_scratch_directory(&root, &path);
        })
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        if self.armed {
            cleanup_scratch_directory(&self.root, &self.path);
        }
    }
}

fn cleanup_scratch_directory(root: &Path, path: &Path) {
    if path.parent() != Some(root)
        || !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(SCRATCH_PREFIX))
    {
        return;
    }
    make_tree_writable(path);
    let _ = std::fs::remove_dir_all(path);
}

fn make_tree_writable(path: &Path) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if is_reparse_or_symlink(&metadata) {
        return;
    }
    if metadata.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                make_tree_writable(&entry.path());
            }
        }
    }
    let _ = set_readonly(path, false);
}

fn unavailable(detail: impl Into<String>) -> CompilerBackendFailureV1 {
    CompilerBackendFailureV1::new(CompilerBackendFailureKindV1::Unavailable, detail)
}

fn preflight(detail: impl Into<String>) -> CompilerBackendFailureV1 {
    CompilerBackendFailureV1::new(CompilerBackendFailureKindV1::Preflight, detail)
}

fn internal(detail: impl Into<String>) -> CompilerBackendFailureV1 {
    CompilerBackendFailureV1::new(CompilerBackendFailureKindV1::Internal, detail)
}

fn invalid_output(detail: impl Into<String>) -> CompilerBackendFailureV1 {
    CompilerBackendFailureV1::new(CompilerBackendFailureKindV1::InvalidOutput, detail)
}

#[cfg(windows)]
fn configure_process_group(
    command: &mut Command,
    _memory_limit_bytes: u64,
) -> Result<(), CompilerBackendFailureV1> {
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
    Ok(())
}

#[cfg(unix)]
fn configure_process_group(
    command: &mut Command,
    memory_limit_bytes: u64,
) -> Result<(), CompilerBackendFailureV1> {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
    // Safety: the closure calls only the async-signal-safe setrlimit syscall and
    // constructs no Rust-owned state in the child between fork and exec.
    unsafe {
        command.pre_exec(move || {
            let limit = libc::rlimit {
                rlim_cur: memory_limit_bytes as libc::rlim_t,
                rlim_max: memory_limit_bytes as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(())
}

#[cfg(windows)]
struct ProcessTreeGuard {
    job: windows_sys::Win32::Foundation::HANDLE,
    armed: bool,
}

#[cfg(windows)]
impl ProcessTreeGuard {
    fn attach(
        child: &mut std::process::Child,
        memory_limit_bytes: u64,
    ) -> Result<Self, CompilerBackendFailureV1> {
        use std::mem::size_of;
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        };
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(internal(format!(
                    "creating sidecar job object failed with {}",
                    GetLastError()
                )));
            }
            let mut information: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
                | JOB_OBJECT_LIMIT_PROCESS_MEMORY
                | JOB_OBJECT_LIMIT_JOB_MEMORY;
            information.BasicLimitInformation.ActiveProcessLimit = 1;
            information.ProcessMemoryLimit = memory_limit_bytes as usize;
            information.JobMemoryLimit = memory_limit_bytes as usize;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &information as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle() as _) == 0
            {
                let error = GetLastError();
                let _ = child.kill();
                let _ = child.wait();
                CloseHandle(job);
                return Err(internal(format!(
                    "assigning sidecar to kill-on-close job failed with {error}"
                )));
            }
            Ok(Self { job, armed: true })
        }
    }

    fn terminate(
        &mut self,
        child: &mut std::process::Child,
    ) -> Result<(), CompilerBackendFailureV1> {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        unsafe {
            if TerminateJobObject(self.job, 70) == 0 {
                let job_error = GetLastError();
                child.kill().map_err(|child_error| {
                    internal(format!(
                        "terminating sidecar job failed with {job_error}; direct kill failed: \
                         {child_error}"
                    ))
                })?;
            }
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(windows)]
impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        unsafe {
            if self.armed {
                let _ = TerminateJobObject(self.job, 70);
            }
            CloseHandle(self.job);
        }
    }
}

#[cfg(unix)]
struct ProcessTreeGuard {
    process_group: i32,
    armed: bool,
}

#[cfg(unix)]
impl ProcessTreeGuard {
    fn attach(
        child: &mut std::process::Child,
        _memory_limit_bytes: u64,
    ) -> Result<Self, CompilerBackendFailureV1> {
        let process_group = i32::try_from(child.id())
            .map_err(|_| internal("sidecar process id cannot be represented"))?;
        Ok(Self {
            process_group,
            armed: true,
        })
    }

    fn terminate(
        &mut self,
        child: &mut std::process::Child,
    ) -> Result<(), CompilerBackendFailureV1> {
        let group_result = unsafe { libc::kill(-self.process_group, libc::SIGKILL) };
        if group_result != 0 {
            child
                .kill()
                .map_err(|error| internal(format!("terminating sidecar process group: {error}")))?;
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(unix)]
impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        if self.armed {
            unsafe {
                let _ = libc::kill(-self.process_group, libc::SIGKILL);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_profile::frontend::{
        ClassGeneratorConfigV1, CompilerOptionsV1, EffectivePreprocessorFlagV1,
        PreprocessorConfigV1, PropertyBlueprintSpecifierV1, PropertyEditSpecifierV1,
        StaticClassModeV1, CLASS_GENERATOR_CONFIG_SCHEMA, COMPILER_OPTIONS_SCHEMA,
        FRONTEND_SCHEMA_VERSION, PREPROCESSOR_CONFIG_SCHEMA,
    };
    use crate::compiler_profile::manifest::{
        BindsProfileV1, BytecodeProfileV1, CacheWriterProfileV1, CompilerArchitectureV1,
        CompilerBuildConfigurationV1, CompilerOracleV1, CompilerPlatformV1, CompilerTargetV1,
        EngineProfileV1, FrontendProfileV1, PeCodeViewV1, QualificationProfileV1, Sha1Digest,
        UnrealSemanticsProfileV1, COMPILER_PROFILE_SCHEMA, COMPILER_PROFILE_SCHEMA_VERSION,
    };
    use crate::compiler_profile::qualification::{
        CompilerProbeCaseV1, CompilerProbeCorpusV1, DiagnosticParityEntryV1,
        DiagnosticParityReportV1, ExpectedProbeResultV1, ExpectedProbeResultsV1, ProbeModeV1,
        ProbeOutcomeV1, ProbeSourceSectionV1, SemanticParityEntryV1, SemanticParityReportV1,
        DIAGNOSTIC_PARITY_SCHEMA, EXPECTED_RESULTS_SCHEMA, PROBE_CORPUS_SCHEMA,
        QUALIFICATION_SCHEMA_VERSION, SEMANTIC_PARITY_SCHEMA,
    };
    use crate::compiler_profile::registry::{
        DynamicScriptTypeOperationsV1, EnginePropertySettingV1, EnginePropertyV1,
        FixedTypeOperationsV1, OrderedEnginePropertiesV1, PostBindEntryV1, PostBindResultV1,
        PostBindSnapshotV1, PrimitiveTypeOperationsV1, PrimitiveTypeV1, RegistrationContextV1,
        RegistrationEntryV1, RegistrationTraceV1, TypeOperationsV1, ENGINE_PROPERTIES_SCHEMA,
        POST_BIND_SNAPSHOT_SCHEMA, REGISTRATION_TRACE_SCHEMA,
    };

    const TEST_OVERLAYS: [StandaloneCompilerOverlayV1<'static>; 1] =
        [StandaloneCompilerOverlayV1 {
            operation: StandaloneCompilerOverlayOperationV1::Add,
            module_name: "Module",
            relative_path: "Module.as",
        }];

    struct TestFixture {
        root: PathBuf,
        profile_root: PathBuf,
        manifest: PathBuf,
        scratch: PathBuf,
        sources: PathBuf,
        base: Vec<u8>,
        binds: Vec<u8>,
    }

    impl TestFixture {
        fn create(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "gore-as-sidecar-test-{label}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = std::fs::remove_dir_all(&root);
            let profile_root = root.join("profile-source");
            let scratch = root.join("scratch");
            let sources = root.join("sources");
            for path in [&profile_root, &scratch, &sources] {
                std::fs::create_dir_all(path).unwrap();
            }
            std::fs::write(sources.join("Module.as"), b"void Test() {}\n").unwrap();
            let base = b"sealed-base-cache".to_vec();
            let binds = b"sealed-binds-cache".to_vec();
            let blob_bytes = b"profile-blob";
            std::fs::write(profile_root.join("blob.bin"), blob_bytes).unwrap();
            let blob = SealedBlobV1 {
                path: "blob.bin".to_owned(),
                byte_len: blob_bytes.len() as u64,
                sha256: sha256_bytes(blob_bytes),
            };
            let mut properties = OrderedEnginePropertiesV1 {
                schema: ENGINE_PROPERTIES_SCHEMA.into(),
                schema_version: 1,
                settings: vec![EnginePropertySettingV1 {
                    ordinal: 0,
                    property: EnginePropertyV1::OptimizeBytecode,
                    value: 1,
                }],
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            properties.seal().unwrap();
            let properties_json = properties.to_json().unwrap();
            let primitive_layouts = [
                (PrimitiveTypeV1::Bool, 1, 1),
                (PrimitiveTypeV1::Int8, 1, 1),
                (PrimitiveTypeV1::Int16, 2, 2),
                (PrimitiveTypeV1::Int32, 4, 4),
                (PrimitiveTypeV1::Int64, 8, 8),
                (PrimitiveTypeV1::Uint8, 1, 1),
                (PrimitiveTypeV1::Uint16, 2, 2),
                (PrimitiveTypeV1::Uint32, 4, 4),
                (PrimitiveTypeV1::Uint64, 8, 8),
                (PrimitiveTypeV1::Float32, 4, 4),
                (PrimitiveTypeV1::Float64, 8, 8),
            ];
            let mut trace = RegistrationTraceV1 {
                schema: REGISTRATION_TRACE_SCHEMA.into(),
                schema_version: 1,
                host_stubs: vec![],
                primitive_operations: primitive_layouts
                    .into_iter()
                    .enumerate()
                    .map(|(ordinal, (primitive, value_size, value_alignment))| {
                        PrimitiveTypeOperationsV1 {
                            ordinal: ordinal as u32,
                            primitive,
                            operations: FixedTypeOperationsV1 {
                                can_be_template_subtype: true,
                                can_construct: true,
                                need_construct: false,
                                can_destruct: true,
                                need_destruct: false,
                                can_copy: true,
                                need_copy: false,
                                can_compare: true,
                                can_hash_value: true,
                                value_size,
                                value_alignment,
                                is_object_pointer: false,
                            },
                        }
                    })
                    .collect(),
                dynamic_script_operations: DynamicScriptTypeOperationsV1 {
                    delegate: FixedTypeOperationsV1 {
                        can_be_template_subtype: true,
                        can_construct: true,
                        need_construct: true,
                        can_destruct: true,
                        need_destruct: true,
                        can_copy: true,
                        need_copy: true,
                        can_compare: true,
                        can_hash_value: false,
                        value_size: 16,
                        value_alignment: 8,
                        is_object_pointer: false,
                    },
                    multicast_delegate: FixedTypeOperationsV1 {
                        can_be_template_subtype: true,
                        can_construct: true,
                        need_construct: true,
                        can_destruct: true,
                        need_destruct: true,
                        can_copy: true,
                        need_copy: true,
                        can_compare: true,
                        can_hash_value: false,
                        value_size: 16,
                        value_alignment: 8,
                        is_object_pointer: false,
                    },
                },
                entries: vec![RegistrationEntryV1::Enum {
                    ordinal: 0,
                    registration_id: 0,
                    context: RegistrationContextV1 {
                        namespace: String::new(),
                        config_group: None,
                        access_mask: u32::MAX,
                    },
                    type_id: 1,
                    declaration: "ETest".into(),
                    type_operations: TypeOperationsV1::Fixed {
                        operations: FixedTypeOperationsV1 {
                            can_be_template_subtype: true,
                            can_construct: true,
                            need_construct: false,
                            can_destruct: true,
                            need_destruct: false,
                            can_copy: true,
                            need_copy: true,
                            can_compare: true,
                            can_hash_value: true,
                            value_size: 1,
                            value_alignment: 1,
                            is_object_pointer: false,
                        },
                    },
                }],
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            trace.seal().unwrap();
            let trace_json = trace.to_json().unwrap();
            let mut snapshot = PostBindSnapshotV1 {
                schema: POST_BIND_SNAPSHOT_SCHEMA.into(),
                schema_version: 1,
                engine_properties_sha256: properties.canonical_sha256,
                registration_trace_sha256: trace.canonical_sha256,
                entries: vec![PostBindEntryV1 {
                    ordinal: 0,
                    trace_registration_id: 0,
                    result: PostBindResultV1::Enum { engine_type_id: 1 },
                }],
                final_states: vec![],
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            snapshot.seal().unwrap();
            let snapshot_json = snapshot.to_json().unwrap();
            let registry_blob = |path: &str, bytes: &[u8]| {
                let destination = profile_root.join(path);
                std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
                std::fs::write(&destination, bytes).unwrap();
                SealedBlobV1 {
                    path: path.into(),
                    byte_len: bytes.len() as u64,
                    sha256: sha256_bytes(bytes),
                }
            };
            let properties_blob = registry_blob("engine/properties.json", &properties_json);
            let trace_blob = registry_blob("engine/registrations.json", &trace_json);
            let snapshot_blob = registry_blob("engine/post-bind.json", &snapshot_json);
            let mut preprocessor = PreprocessorConfigV1 {
                schema: PREPROCESSOR_CONFIG_SCHEMA.into(),
                schema_version: FRONTEND_SCHEMA_VERSION,
                automatic_imports: true,
                warn_on_manual_import_statements: true,
                use_editor_scripts: false,
                effective_flags: [
                    ("COOK_COMMANDLET", false),
                    ("EDITOR", false),
                    ("EDITORONLY_DATA", false),
                    ("RELEASE", true),
                    ("TEST", false),
                    ("WITH_SERVER_CODE", true),
                ]
                .into_iter()
                .enumerate()
                .map(|(ordinal, (name, value))| EffectivePreprocessorFlagV1 {
                    ordinal: ordinal as u32,
                    name: name.into(),
                    value,
                })
                .collect(),
                default_function_blueprint_callable: true,
                default_property_edit_specifier: PropertyEditSpecifierV1::EditAnywhere,
                default_property_edit_specifier_for_structs: PropertyEditSpecifierV1::EditAnywhere,
                default_property_blueprint_specifier:
                    PropertyBlueprintSpecifierV1::BlueprintReadWrite,
                static_class_mode: StaticClassModeV1::Allowed,
                script_float_is_float64: true,
                angelscript_haze: false,
                enforce_server_rpc_validation: false,
                blueprint_event_argument_specializations: vec![
                    "FName".to_owned(),
                    "int32".to_owned(),
                ],
                native_super_types: vec![
                    crate::compiler_profile::frontend::NativeSuperTypeV1 {
                        ordinal: 0,
                        angelscript_type_name: "AActor".to_owned(),
                        unreal_class_path: "/Script/Engine.Actor".to_owned(),
                        property_offset: 0,
                        kind: crate::compiler_profile::frontend::NativeSuperKindV1::Actor,
                        cannot_derive_angelscript: false,
                    },
                    crate::compiler_profile::frontend::NativeSuperTypeV1 {
                        ordinal: 1,
                        angelscript_type_name: "UObject".to_owned(),
                        unreal_class_path: "/Script/CoreUObject.Object".to_owned(),
                        property_offset: 0,
                        kind: crate::compiler_profile::frontend::NativeSuperKindV1::OtherUObject,
                        cannot_derive_angelscript: false,
                    },
                ],
                fname_comparison_keys: Vec::new(),
                external_hooks: crate::compiler_profile::frontend::ExternalFrontendHooksV1::unbound(
                ),
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            preprocessor.seal().unwrap();
            let preprocessor_json = preprocessor.to_json().unwrap();
            let mut class_generator = ClassGeneratorConfigV1 {
                schema: CLASS_GENERATOR_CONFIG_SCHEMA.into(),
                schema_version: FRONTEND_SCHEMA_VERSION,
                mark_non_uproperty_properties_as_transient: false,
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            class_generator.seal().unwrap();
            let class_generator_json = class_generator.to_json().unwrap();
            let mut compiler_options = CompilerOptionsV1 {
                schema: COMPILER_OPTIONS_SCHEMA.into(),
                schema_version: FRONTEND_SCHEMA_VERSION,
                error_on_incorrect_editor_only_code: true,
                warn_on_divergent_comparison_operator_overloads: true,
                warn_on_implicit_signed_unsigned_conversion: true,
                warn_on_increment_decrement_in_complex_expression: true,
                warn_on_unused_return_value_for_const_methods: true,
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            compiler_options.seal().unwrap();
            let compiler_options_json = compiler_options.to_json().unwrap();
            let preprocessor_blob = registry_blob("frontend/preprocessor.json", &preprocessor_json);
            let class_generator_blob =
                registry_blob("frontend/class-generator.json", &class_generator_json);
            let compiler_options_blob =
                registry_blob("frontend/compiler-options.json", &compiler_options_json);
            let source_text = "void Test() {}\n";
            let mut probe_corpus = CompilerProbeCorpusV1 {
                schema: PROBE_CORPUS_SCHEMA.into(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: "sidecar-test-v1".into(),
                cases: vec![CompilerProbeCaseV1 {
                    ordinal: 0,
                    case_id: "positive.compile".into(),
                    category: "smoke".into(),
                    expected_outcome: ProbeOutcomeV1::Accepted,
                    mode: ProbeModeV1::CompileOnly,
                    sections: vec![ProbeSourceSectionV1 {
                        ordinal: 0,
                        module: "Module".into(),
                        relative_path: "Module.as".into(),
                        source_utf8: source_text.into(),
                        source_sha256: sha256_bytes(source_text.as_bytes()),
                    }],
                }],
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            probe_corpus.seal().unwrap();
            let semantic_sha256 = sha256_bytes(b"normalized-smoke-result");
            let mut expected_results = ExpectedProbeResultsV1 {
                schema: EXPECTED_RESULTS_SCHEMA.into(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: probe_corpus.suite_id.clone(),
                corpus_sha256: probe_corpus.canonical_sha256,
                results: vec![ExpectedProbeResultV1 {
                    ordinal: 0,
                    case_id: probe_corpus.cases[0].case_id.clone(),
                    outcome: ProbeOutcomeV1::Accepted,
                    diagnostics: vec![],
                    semantic_sha256: Some(semantic_sha256),
                }],
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            expected_results.seal().unwrap();
            let diagnostics_sha256 = expected_results.results[0].diagnostics_sha256().unwrap();
            let mut diagnostic_parity = DiagnosticParityReportV1 {
                schema: DIAGNOSTIC_PARITY_SCHEMA.into(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: probe_corpus.suite_id.clone(),
                corpus_sha256: probe_corpus.canonical_sha256,
                expected_results_sha256: expected_results.canonical_sha256,
                entries: vec![DiagnosticParityEntryV1 {
                    ordinal: 0,
                    case_id: probe_corpus.cases[0].case_id.clone(),
                    expected_sha256: diagnostics_sha256,
                    embedded_sha256: diagnostics_sha256,
                    standalone_sha256: diagnostics_sha256,
                }],
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            diagnostic_parity.seal().unwrap();
            let mut semantic_parity = SemanticParityReportV1 {
                schema: SEMANTIC_PARITY_SCHEMA.into(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: probe_corpus.suite_id.clone(),
                corpus_sha256: probe_corpus.canonical_sha256,
                expected_results_sha256: expected_results.canonical_sha256,
                entries: vec![SemanticParityEntryV1 {
                    ordinal: 0,
                    case_id: probe_corpus.cases[0].case_id.clone(),
                    expected_sha256: semantic_sha256,
                    embedded_sha256: semantic_sha256,
                    standalone_sha256: semantic_sha256,
                }],
                unexplained_differences: vec![],
                qualified: true,
                canonical_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            semantic_parity.seal().unwrap();
            let probe_corpus_json = probe_corpus.to_json().unwrap();
            let expected_results_json = expected_results.to_json().unwrap();
            let diagnostic_parity_json = diagnostic_parity.to_json().unwrap();
            let semantic_parity_json = semantic_parity.to_json().unwrap();
            let probe_corpus_blob = registry_blob("qualification/corpus.json", &probe_corpus_json);
            let expected_results_blob =
                registry_blob("qualification/expected.json", &expected_results_json);
            let diagnostic_parity_blob =
                registry_blob("qualification/diagnostics.json", &diagnostic_parity_json);
            let semantic_parity_blob =
                registry_blob("qualification/semantics.json", &semantic_parity_json);
            let file = |bytes: &[u8], steam: bool| FileSealV1 {
                byte_len: bytes.len() as u64,
                sha256: sha256_bytes(bytes),
                steam_content_sha1: steam.then(|| Sha1Digest::from_bytes([0x5a; 20])),
            };
            let mut profile = CompilerProfileV1 {
                schema: COMPILER_PROFILE_SCHEMA.to_owned(),
                schema_version: COMPILER_PROFILE_SCHEMA_VERSION,
                target: CompilerTargetV1 {
                    steam_app_id: 1_297_900,
                    steam_build_id: 24_539_464,
                    depot_id: 1_297_901,
                    depot_manifest_gid: 1_585_071_322_101_748_861,
                    platform: CompilerPlatformV1::Windows,
                    architecture: CompilerArchitectureV1::X86_64,
                    build_configuration: CompilerBuildConfigurationV1::Shipping,
                },
                oracle: CompilerOracleV1 {
                    executable: file(b"exe", true),
                    binds_cache: file(&binds, true),
                    shipping_cache: file(&base, true),
                    depot_manifest: file(b"manifest", false),
                    pe_codeview: PeCodeViewV1 {
                        guid: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
                        age: 1,
                    },
                },
                binds: BindsProfileV1 {
                    wire_schema_version: 1,
                    struct_count: 1,
                    class_count: 1,
                    method_count: 1,
                    struct_property_count: 1,
                    class_property_count: 1,
                    canonical_database_sha256: sha256_bytes(b"database"),
                },
                engine: EngineProfileV1 {
                    as_create_version: 23_300,
                    ordered_engine_properties: properties_blob,
                    registration_trace: trace_blob,
                    registration_trace_count: 1,
                    post_bind_snapshot: snapshot_blob,
                },
                unreal_semantics: UnrealSemanticsProfileV1 {
                    reflected_type_graph: blob.clone(),
                    metadata_schema_version: 1,
                },
                frontend: FrontendProfileV1 {
                    preprocessor_config: preprocessor_blob,
                    class_generator_config: class_generator_blob,
                    compiler_options: compiler_options_blob,
                },
                bytecode: BytecodeProfileV1 {
                    opcode_table_version: "g1r-v1".to_owned(),
                    opcode_table: blob.clone(),
                    operand_schema: blob.clone(),
                    codegen_probe_corpus: probe_corpus_blob,
                    expected_probe_results: expected_results_blob,
                },
                cache_writer: CacheWriterProfileV1 {
                    format_version: 1,
                    serializer_schema: blob.clone(),
                    build_identifier: 0x9e37_7abe,
                    reference_table_order: blob.clone(),
                    normalized_oracle_corpus: blob.clone(),
                },
                qualification: QualificationProfileV1 {
                    required_probe_suite_version: "sidecar-test-v1".to_owned(),
                    diagnostic_parity: diagnostic_parity_blob,
                    semantic_parity: semantic_parity_blob,
                    qualified: true,
                },
                profile_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            profile.seal().unwrap();
            let manifest = profile_root.join("profile.json");
            std::fs::write(&manifest, serde_json::to_vec(&profile).unwrap()).unwrap();
            Self {
                root,
                profile_root,
                manifest,
                scratch,
                sources,
                base,
                binds,
            }
        }

        fn runner(
            &self,
            label: &str,
            script: &str,
            timeout: Duration,
        ) -> Option<StandaloneSidecarRunnerV1> {
            let python = find_python()?;
            let script_path = self.root.join(format!("{label}.py"));
            std::fs::write(&script_path, script).unwrap();
            let mut config = StandaloneSidecarConfigV1::new(
                python.clone(),
                executable_seal(&python),
                self.manifest.clone(),
                self.profile_root.clone(),
                self.scratch.clone(),
            )
            .with_test_fixed_args([script_path.into_os_string()]);
            config.timeout = timeout;
            config.termination_grace = Duration::from_secs(3);
            Some(StandaloneSidecarRunnerV1::new(config).unwrap())
        }

        fn inputs(&self) -> StandaloneCompilerInputsV1<'_> {
            StandaloneCompilerInputsV1 {
                source_tree: &self.sources,
                overlays: &TEST_OVERLAYS,
                base_cache: Some(&self.base),
                binds_cache: Some(&self.binds),
            }
        }

        fn assert_scratch_empty(&self) {
            assert_eq!(std::fs::read_dir(&self.scratch).unwrap().count(), 0);
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            make_tree_writable(&self.root);
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn find_python() -> Option<PathBuf> {
        let names: &[&str] = if cfg!(windows) {
            &["python.exe", "python3.exe"]
        } else {
            &["python3", "python"]
        };
        for directory in std::env::split_paths(&std::env::var_os("PATH")?) {
            for name in names {
                let candidate = directory.join(name);
                if !candidate.is_file() {
                    continue;
                }
                let Ok(candidate) = std::fs::canonicalize(candidate) else {
                    continue;
                };
                if Command::new(&candidate)
                    .arg("--version")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_ok_and(|status| status.success())
                {
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn executable_seal(path: &Path) -> SidecarExecutableSealV1 {
        let (byte_len, sha256) =
            hash_regular_file(path, MAX_SIDECAR_EXECUTABLE_BYTES_V1, "test sidecar").unwrap();
        SidecarExecutableSealV1 { byte_len, sha256 }
    }

    #[test]
    fn fake_sidecar_success_receives_sealed_request_and_retains_output_until_drop() {
        let fixture = TestFixture::create("success");
        let Some(mut runner) = fixture.runner(
            "success",
            r#"
import hashlib, json, pathlib, sys
assert sys.argv[1] == "compile" and sys.argv[2] == "--request"
request = json.loads(pathlib.Path(sys.argv[3]).read_text(encoding="utf-8"))
assert request["request_version"] == 1 and request["operation"] == "compile"
assert request["profile"]["profile_sha256"]
assert request["inputs"]["base_cache"]["sha256"]
assert request["inputs"]["binds_cache"]["sha256"]
assert request["inputs"]["source_tree"]["files"][0]["path"] == "Module.as"
assert request["inputs"]["overlays"] == [{"ordinal":0,"operation":"add","module_name":"Module","relative_path":"Module.as"}]
data = b"fake-full-cache"
output = pathlib.Path(request["output"]["cache_path"])
output.write_bytes(data)
print(json.dumps({"response_version":1,"ok":True,"output":{"cache_path":str(output),"byte_len":len(data),"sha256":hashlib.sha256(data).hexdigest(),"profile_sha256":request["profile"]["profile_sha256"]},"diagnostics":[]}))
"#,
            Duration::from_secs(5),
        ) else {
            eprintln!("python unavailable; fake-sidecar process test skipped");
            return;
        };

        let output = runner.run_regen(fixture.inputs()).unwrap();
        assert_eq!(std::fs::read(output.path()).unwrap(), b"fake-full-cache");
        assert_eq!(std::fs::read_dir(&fixture.scratch).unwrap().count(), 1);
        drop(output);
        fixture.assert_scratch_empty();
    }

    #[test]
    fn overlay_manifest_rejects_missing_or_unsealed_sources_before_process_start() {
        let fixture = TestFixture::create("overlay-preflight");
        let Some(mut runner) = fixture.runner(
            "overlay-preflight",
            "raise AssertionError('sidecar must not start after overlay preflight failure')\n",
            Duration::from_secs(5),
        ) else {
            eprintln!("python unavailable; overlay preflight test skipped");
            return;
        };

        let empty = StandaloneCompilerInputsV1 {
            source_tree: &fixture.sources,
            overlays: &[],
            base_cache: Some(&fixture.base),
            binds_cache: Some(&fixture.binds),
        };
        let error = runner.run_regen(empty).unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::Preflight);
        assert!(error.detail().contains("overlay count"), "{error}");
        fixture.assert_scratch_empty();

        const MISSING: [StandaloneCompilerOverlayV1<'static>; 1] = [StandaloneCompilerOverlayV1 {
            operation: StandaloneCompilerOverlayOperationV1::Edit,
            module_name: "Missing",
            relative_path: "Missing.as",
        }];
        let missing = StandaloneCompilerInputsV1 {
            source_tree: &fixture.sources,
            overlays: &MISSING,
            base_cache: Some(&fixture.base),
            binds_cache: Some(&fixture.binds),
        };
        let error = runner.run_regen(missing).unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::Preflight);
        assert!(error.detail().contains("absent"), "{error}");
        fixture.assert_scratch_empty();
    }

    #[test]
    fn fake_sidecar_preserves_unavailable_and_rejected_classes() {
        let fixture = TestFixture::create("failures");
        let Some(mut unavailable_runner) = fixture.runner(
            "unavailable",
            r#"
import json, sys
print(json.dumps({"response_version":1,"ok":False,"diagnostics":[{"severity":"error","code":"GORE_AS_STANDALONE_ENGINE_UNAVAILABLE","message":"engine absent"}]}))
sys.exit(69)
"#,
            Duration::from_secs(5),
        ) else {
            eprintln!("python unavailable; fake-sidecar process test skipped");
            return;
        };
        let unavailable_error = unavailable_runner.run_regen(fixture.inputs()).unwrap_err();
        assert_eq!(
            unavailable_error.kind(),
            CompilerBackendFailureKindV1::Unavailable
        );
        assert!(unavailable_error.detail().contains("engine absent"));
        fixture.assert_scratch_empty();

        let mut rejected_runner = fixture
            .runner(
                "rejected",
                r#"
import json, sys
print(json.dumps({"response_version":1,"ok":False,"failure_kind":"rejected","diagnostics":[{"severity":"error","code":"GORE_AS_COMPILE_REJECTED","message":"bad expression"}]}))
sys.exit(65)
"#,
                Duration::from_secs(5),
            )
            .unwrap();
        let rejected_error = rejected_runner.run_regen(fixture.inputs()).unwrap_err();
        assert_eq!(
            rejected_error.kind(),
            CompilerBackendFailureKindV1::Rejected
        );
        assert!(rejected_error.detail().contains("bad expression"));
        fixture.assert_scratch_empty();
    }

    #[test]
    fn fake_sidecar_unknown_response_field_fails_closed() {
        let fixture = TestFixture::create("unknown-response");
        let Some(mut runner) = fixture.runner(
            "unknown-response",
            r#"
import json, sys
print(json.dumps({"response_version":1,"ok":False,"future_field":True,"diagnostics":[{"severity":"error","code":"GORE_AS_INTERNAL","message":"failed"}]}))
sys.exit(70)
"#,
            Duration::from_secs(5),
        ) else {
            eprintln!("python unavailable; fake-sidecar process test skipped");
            return;
        };
        let error = runner.run_regen(fixture.inputs()).unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::InvalidOutput);
        assert!(error.detail().contains("unknown field"), "{error}");
        fixture.assert_scratch_empty();
    }

    #[test]
    fn fake_sidecar_timeout_terminates_the_process() {
        let fixture = TestFixture::create("timeout");
        let Some(mut runner) = fixture.runner(
            "timeout",
            r#"
import time
time.sleep(30)
"#,
            Duration::from_millis(50),
        ) else {
            eprintln!("python unavailable; fake-sidecar process test skipped");
            return;
        };
        let started = Instant::now();
        let error = runner.run_regen(fixture.inputs()).unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::Internal);
        assert!(error.detail().contains("exceeded"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5));
        fixture.assert_scratch_empty();
    }

    #[test]
    fn sidecar_memory_limit_is_bounded_before_process_start() {
        let fixture = TestFixture::create("memory-limit");
        let Some(python) = find_python() else {
            eprintln!("python unavailable; sidecar configuration test skipped");
            return;
        };
        let mut config = StandaloneSidecarConfigV1::new(
            python.clone(),
            executable_seal(&python),
            fixture.manifest.clone(),
            fixture.profile_root.clone(),
            fixture.scratch.clone(),
        );
        config.memory_limit_bytes = MIN_SIDECAR_MEMORY_LIMIT_BYTES - 1;
        let error = StandaloneSidecarRunnerV1::new(config).unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::Unavailable);
        assert!(error.detail().contains("memory limit"), "{error}");
        fixture.assert_scratch_empty();
    }

    #[test]
    fn sidecar_executable_must_match_the_packaged_seal() {
        let fixture = TestFixture::create("executable-seal");
        let Some(python) = find_python() else {
            eprintln!("python unavailable; sidecar seal test skipped");
            return;
        };
        let actual = executable_seal(&python);
        let config = StandaloneSidecarConfigV1::new(
            python,
            SidecarExecutableSealV1 {
                byte_len: actual.byte_len,
                sha256: Sha256Digest::from_bytes([0; 32]),
            },
            fixture.manifest.clone(),
            fixture.profile_root.clone(),
            fixture.scratch.clone(),
        );
        let error = StandaloneSidecarRunnerV1::new(config).unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::Unavailable);
        assert!(error.detail().contains("SHA-256 seal"), "{error}");
        fixture.assert_scratch_empty();
    }

    #[test]
    fn opaque_profile_package_rejects_self_consistent_but_untyped_qualification() {
        let fixture = TestFixture::create("opaque-profile-package");
        let manifest_bytes = std::fs::read(&fixture.manifest).unwrap();
        let mut profile = CompilerProfileV1::from_json(&manifest_bytes).unwrap();
        let semantic_path = fixture
            .profile_root
            .join(&profile.qualification.semantic_parity.path);
        let forged = b"{}";
        std::fs::write(&semantic_path, forged).unwrap();
        profile.qualification.semantic_parity.byte_len = forged.len() as u64;
        profile.qualification.semantic_parity.sha256 = sha256_bytes(forged);
        profile.seal().unwrap();
        std::fs::write(&fixture.manifest, serde_json::to_vec(&profile).unwrap()).unwrap();

        let error =
            ValidatedCompilerProfilePackageV1::load(&fixture.manifest, &fixture.profile_root)
                .unwrap_err();
        assert_eq!(error.kind(), CompilerBackendFailureKindV1::Unavailable);
        assert!(
            error.detail().contains("qualification is invalid"),
            "{error}"
        );
        fixture.assert_scratch_empty();
    }
}
