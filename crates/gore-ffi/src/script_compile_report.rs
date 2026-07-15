//! Bounded structured AngelScript module compilation for Mod Studio.
//!
//! This additive route keeps the legacy `script_compile` wire intact while exposing the compiler
//! diagnostics/capture disposition and live-install restoration result as data. The game compiler
//! is entered only after a drift-aware pristine base has been resolved. Once entered, gore-as owns
//! the transactional backup/stage/restore window and the response never claims a usable mini-cache
//! unless that window closed with an exact restore.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gore_as::compile::{
    acquire_compile_install_mutation, compile_module_with_diagnostics_report_with_guard,
    probe_install_compile_state, CompileError, CompileModuleReport, CompileModuleReportOutcome,
    CompileOpts, InstallCompileArtifactKind, InstallCompileGameProcessDisposition,
    InstallCompileInspectionIssueKind, InstallCompileStateDisposition, InstallCompileStateProbe,
    InstallMutationGuard, InstallRestoreDisposition,
};
use gore_as::diagnostics::{
    CompilerDiagnostic, DiagnosticSeverity, DiagnosticsCaptureDisposition, DiagnosticsOptions,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::err;

pub(super) const COMMAND: &str = "script_compile_report_v1";
pub(super) const INSTALL_STATE_COMMAND: &str = "script_compile_install_state_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_MODULE_NAME_BYTES: usize = 4 * 1024;
const MAX_REL_PATH_BYTES: usize = 32 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_WIRE_DIAGNOSTICS: usize = 4_096;
const MAX_WIRE_DIAGNOSTIC_TEXT_BYTES: usize = 4 * 1024 * 1024;
// Six is the maximum JSON expansion per input byte (`\u00XX`). Keep the raw envelope far below
// the global 64 MiB transport ceiling before serde allocates the request tree.
const MAX_WIRE_BYTES: usize =
    MAX_PATH_BYTES * 18 + MAX_MODULE_NAME_BYTES * 6 + MAX_REL_PATH_BYTES * 6 + 4 * 1024;
const MAX_INSTALL_STATE_WIRE_BYTES: usize = MAX_PATH_BYTES * 6 + 1024;
const MAX_INSTALL_STATE_ARTIFACTS: usize = 7;
const MAX_INSTALL_STATE_ISSUES: usize = 8;
const MAX_INSTALL_STATE_DISPLAY_PATH_BYTES: usize = 4_096;
const MAX_INSTALL_STATE_MESSAGE_BYTES: usize = 2_048;
const OWNED_COMPILE_PREFIX: &str = "gore-owned-compile-";
const OWNED_COMPILE_MARKER: &str = ".gore-owned-compile-v1";
const OWNED_COMPILE_MARKER_CONTENT: &[u8] = b"gore-owned-compile-staging-v1\n";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest {
    command: String,
    payload: CompileWirePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompileWirePayload {
    allow_new_symbols: bool,
    as_path: String,
    game_dir: String,
    module_name: String,
    op: String,
    rel_path: String,
    work_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactInstallStateWireRequest {
    command: String,
    payload: InstallStateWirePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallStateWirePayload {
    game_dir: String,
}

/// A directory created by this invocation beneath the caller's workspace root. gore-as may reset
/// only this directory's `tree` child; it never receives the caller-controlled workspace itself.
/// The retained ancestor/child handles reject pre-existing links and, on Windows, exclude delete
/// sharing so the path cannot be renamed or replaced until gore-as has finished using it.
struct OwnedCompileStaging {
    path: PathBuf,
    workspace_real: PathBuf,
    _anchors: Vec<std::fs::File>,
}

impl OwnedCompileStaging {
    fn create(workspace: &Path, game_dir: &Path) -> Result<Self, String> {
        let broad_anchors = open_directory_anchor_chain(workspace, false)?;
        let workspace_real = workspace.canonicalize().map_err(|error| {
            format!(
                "resolving the compile workspace {}: {error}",
                workspace.display()
            )
        })?;
        let install_root = gore_mod::semantic_install_root(game_dir);
        let install_real = install_root.canonicalize().map_err(|error| {
            format!(
                "resolving the selected game installation {}: {error}",
                install_root.display()
            )
        })?;
        if workspace_real.starts_with(&install_real) {
            return Err(
                "compile work_dir must be outside the selected game installation".to_owned(),
            );
        }
        // Keep the returned/output path under the caller's validated lexical absolute workspace;
        // Windows canonicalization commonly adds a `\\?\` prefix that strict Dart containment
        // checks intentionally reject. Canonical paths remain identity-only below.
        let (path, child_anchor) = allocate_owned_compile_child(workspace)?;
        let marker = path.join(OWNED_COMPILE_MARKER);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
            .map_err(|error| format!("creating the compile-staging ownership marker: {error}"))?;
        file.write_all(OWNED_COMPILE_MARKER_CONTENT)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                format!("initializing the compile-staging ownership marker: {error}")
            })?;
        drop(file);

        // Reopen and retain the whole lexical chain after the marker has been created. Directory
        // anchors allow child writes (gore-as must create/reset `tree`) but exclude delete sharing;
        // the marker anchor excludes both write and delete sharing, keeping the child non-empty.
        let mut anchors = open_directory_anchor_chain(workspace, true)?;
        anchors.push(child_anchor);
        anchors.push(open_file_anchor(&marker, "compile-staging marker")?);
        drop(broad_anchors);
        let staging = Self {
            path,
            workspace_real,
            _anchors: anchors,
        };
        staging.verify_owned()?;
        Ok(staging)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn verify_owned(&self) -> Result<(), String> {
        let directory = self.path();
        let metadata = std::fs::symlink_metadata(directory)
            .map_err(|error| format!("inspecting the compile staging directory: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("compile staging is no longer a real directory".to_owned());
        }
        let directory_real = directory
            .canonicalize()
            .map_err(|error| format!("resolving the compile staging directory: {error}"))?;
        if directory_real.parent() != Some(self.workspace_real.as_path()) {
            return Err("compile staging escaped its caller workspace".to_owned());
        }
        let marker = directory.join(OWNED_COMPILE_MARKER);
        let marker_metadata = std::fs::symlink_metadata(&marker)
            .map_err(|error| format!("inspecting the compile-staging ownership marker: {error}"))?;
        if marker_metadata.file_type().is_symlink() || !marker_metadata.is_file() {
            return Err("compile-staging ownership marker is not a regular file".to_owned());
        }
        let marker_bytes = std::fs::read(&marker)
            .map_err(|error| format!("reading the compile-staging ownership marker: {error}"))?;
        if marker_bytes != OWNED_COMPILE_MARKER_CONTENT {
            return Err("compile-staging ownership marker content changed".to_owned());
        }
        Ok(())
    }

    fn retain(self) -> PathBuf {
        // This intentionally only releases the held identity anchors. Native code never performs
        // recursive cleanup through a caller-controlled workspace path; Mod Studio owns its outer
        // temporary workspace and validates the marker before removing it.
        self.path.clone()
    }
}

fn allocate_owned_compile_child(workspace: &Path) -> Result<(PathBuf, std::fs::File), String> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    for attempt in 0..128u64 {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut hasher = Sha256::new();
        hasher.update(workspace.as_os_str().to_string_lossy().as_bytes());
        hasher.update(std::process::id().to_le_bytes());
        hasher.update(sequence.to_le_bytes());
        hasher.update(attempt.to_le_bytes());
        hasher.update(stamp.to_le_bytes());
        let digest = hasher.finalize();
        let suffix = digest[..6]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let candidate = workspace.join(format!("{OWNED_COMPILE_PREFIX}{suffix}"));
        match std::fs::create_dir(&candidate) {
            Ok(()) => {
                run_owned_child_create_hook(&candidate);
                // Pin the exact newly created directory before returning it to any later staging
                // step. On Windows the handle excludes DELETE sharing, so it cannot become a
                // junction/replacement while gore-as resolves and resets its `tree` child.
                let anchor = open_directory_anchor(&candidate, true)?;
                return Ok((candidate, anchor));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "allocating a unique compile staging directory: {error}"
                ));
            }
        }
    }
    Err("allocating a unique compile staging directory exhausted its bounded retries".to_owned())
}

#[cfg(not(test))]
fn run_owned_child_create_hook(_path: &Path) {}

#[cfg(test)]
thread_local! {
    static OWNED_CHILD_CREATE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce(&Path)>>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
fn run_owned_child_create_hook(path: &Path) {
    OWNED_CHILD_CREATE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(test)]
fn inject_owned_child_create_hook(hook: impl FnOnce(&Path) + 'static) {
    OWNED_CHILD_CREATE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn open_directory_anchor_chain(path: &Path, strict: bool) -> Result<Vec<std::fs::File>, String> {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err("compile work_dir must be an absolute normalized path".to_owned());
    }
    let mut prefixes = path.ancestors().collect::<Vec<_>>();
    prefixes.reverse();
    let mut anchors = Vec::with_capacity(prefixes.len());
    for prefix in prefixes {
        if !prefix.as_os_str().is_empty() {
            anchors.push(open_directory_anchor(prefix, strict)?);
        }
    }
    Ok(anchors)
}

#[cfg(windows)]
fn open_directory_anchor(path: &Path, _strict: bool) -> Result<std::fs::File, String> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        // Child creation/removal needs WRITE sharing; excluding DELETE sharing pins the directory
        // identity against rename/replacement while its owned compiler tree is in use.
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path).map_err(|error| {
        format!(
            "opening compile-workspace directory without following links {}: {error}",
            path.display()
        )
    })?;
    let metadata = file.metadata().map_err(|error| {
        format!(
            "inspecting opened compile-workspace directory {}: {error}",
            path.display()
        )
    })?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!(
            "compile work_dir contains a link or non-directory ancestor: {}",
            path.display()
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn open_file_anchor(path: &Path, label: &str) -> Result<std::fs::File, String> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path).map_err(|error| {
        format!(
            "opening {label} without following links {}: {error}",
            path.display()
        )
    })?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspecting opened {label}: {error}"))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!("{label} is not a regular non-reparse file"));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_directory_anchor(path: &Path, _strict: bool) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW);
    let file = options.open(path).map_err(|error| {
        format!(
            "opening compile-workspace directory without following links {}: {error}",
            path.display()
        )
    })?;
    if !file
        .metadata()
        .map_err(|error| format!("inspecting opened compile-workspace directory: {error}"))?
        .is_dir()
    {
        return Err("compile work_dir contains a non-directory ancestor".to_owned());
    }
    Ok(file)
}

#[cfg(unix)]
fn open_file_anchor(path: &Path, label: &str) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options.open(path).map_err(|error| {
        format!(
            "opening {label} without following links {}: {error}",
            path.display()
        )
    })?;
    if !file
        .metadata()
        .map_err(|error| format!("inspecting opened {label}: {error}"))?
        .is_file()
    {
        return Err(format!("{label} is not a regular file"));
    }
    Ok(file)
}

fn anchor_owned_compiled_mini(
    staging: &OwnedCompileStaging,
    output: &gore_as::compile::CompileOutput,
) -> Result<std::fs::File, String> {
    let expected = staging.path().join("module.cache");
    if output.mini_path != expected {
        return Err(
            "compiler output path is not the exact owned-staging module.cache child".to_owned(),
        );
    }
    let metadata = std::fs::symlink_metadata(&expected).map_err(|error| {
        format!("inspecting compiled module.cache without following links: {error}")
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("compiled module.cache is not a regular non-link file".to_owned());
    }
    open_file_anchor(&expected, "compiled module.cache")
}

pub(super) fn compile_report_v1_raw(input: &str) -> Value {
    let payload = match parse_request(input) {
        Ok(payload) => payload,
        Err(message) => return err("SCRIPT_COMPILE_REPORT_BAD_REQUEST", message),
    };

    let game_dir = PathBuf::from(&payload.game_dir);
    // This guard remains held across authoritative pristine selection and the complete compiler
    // transaction, so deploy/undeploy cannot change the selected bytes before live compiler use.
    let guard = match acquire_compile_install_mutation(&game_dir) {
        Ok(guard) => guard,
        Err(message) => return install_guard_failure(&game_dir, message),
    };
    // Do not silently fall back to an arbitrary live/backup choice here. The exact base used for
    // remapping must be the same drift-aware pristine base that deployment would later splice.
    let base_override = match gore_mod::pristine_script_cache(&game_dir) {
        Ok(base) => base,
        Err(error) => {
            let message = error.to_string();
            let failure = if message.contains("RECOVERY_REQUIRED") {
                preflight_failure_with_state(
                    "COMPILE_BASE_RECOVERY_REQUIRED",
                    format!("the deployment-aware pristine cache requires recovery: {message}"),
                    true,
                )
            } else {
                preflight_failure(
                    "COMPILE_BASE_UNAVAILABLE",
                    format!("reading the drift-aware pristine script cache: {message}"),
                )
            };
            return release_guard_after_preflight_failure(
                guard,
                failure,
                "compiler base selection failed before launch",
            );
        }
    };
    let staging = match OwnedCompileStaging::create(Path::new(&payload.work_dir), &game_dir) {
        Ok(staging) => staging,
        Err(message) => {
            return release_guard_after_preflight_failure(
                guard,
                preflight_failure("COMPILE_STAGING_UNAVAILABLE", message),
                "compiler staging failed before launch",
            );
        }
    };
    let opts = CompileOpts {
        game_dir,
        op: payload.op,
        module_name: payload.module_name,
        rel_path: payload.rel_path,
        as_path: PathBuf::from(payload.as_path),
        work_dir: staging.path().to_path_buf(),
        allow_new_symbols: payload.allow_new_symbols,
        base_override: Some(base_override),
    };
    if let Err(message) = staging.verify_owned() {
        return release_guard_after_preflight_failure(
            guard,
            preflight_failure("COMPILE_STAGING_CHANGED", message),
            "compiler staging identity changed before launch",
        );
    }
    let report = compile_module_with_diagnostics_report_with_guard(
        &opts,
        &DiagnosticsOptions {
            disabled: false,
            hook_dll: None,
            inject_delay: Duration::from_secs(2),
        },
        guard,
    );
    let diagnostics_rejection = compiled_diagnostics_rejection(report.diagnostics());
    let (mini_anchor, output_rejection) = match &report.outcome {
        CompileModuleReportOutcome::Compiled(output) => {
            match anchor_owned_compiled_mini(&staging, output) {
                Ok(anchor) => (Some(anchor), None),
                Err(message) => (None, Some(message)),
            }
        }
        CompileModuleReportOutcome::Failed(_) => (None, None),
    };
    let retain_staging = matches!(&report.outcome, CompileModuleReportOutcome::Compiled(_))
        && compiled_output_is_usable(
            report.install_restore_disposition(),
            diagnostics_rejection,
            output_rejection.is_none(),
        );
    if retain_staging {
        // The response's mini_path remains usable after this call. Failed/recovery-required
        // attempts never cause native recursive deletion through caller-controlled paths.
        let _retained = staging.retain();
        debug_assert!(mini_anchor
            .as_ref()
            .and_then(|anchor| anchor.metadata().ok())
            .is_some_and(|metadata| metadata.is_file()));
    }
    let response = report_response(report, output_rejection);
    drop(mini_anchor);
    response
}

pub(super) fn install_state_v1_raw(input: &str) -> Value {
    let payload = match parse_install_state_request(input) {
        Ok(payload) => payload,
        Err(message) => return err("SCRIPT_COMPILE_INSTALL_STATE_BAD_REQUEST", message),
    };
    let game_dir = Path::new(&payload.game_dir);
    let probe = probe_install_compile_state(game_dir);
    let deploy_recovery = match gore_mod::deploy_recovery_required(game_dir) {
        Ok(true) => DeployRecoveryProbe::Required,
        Ok(false) => DeployRecoveryProbe::NotRequired,
        Err(error) => DeployRecoveryProbe::InspectionFailed(error.to_string()),
    };
    install_state_response(probe, deploy_recovery)
}

enum DeployRecoveryProbe {
    NotRequired,
    Required,
    InspectionFailed(String),
}

fn install_state_response(
    probe: InstallCompileStateProbe,
    deploy_recovery: DeployRecoveryProbe,
) -> Value {
    let mut artifacts = probe
        .artifacts
        .iter()
        .take(MAX_INSTALL_STATE_ARTIFACTS - 1)
        .map(|artifact| {
            let display_path =
                truncate_utf8(artifact.path.clone(), MAX_INSTALL_STATE_DISPLAY_PATH_BYTES);
            json!({
                "kind": install_artifact_kind_label(artifact.kind),
                "display_path": display_path,
                "path_truncated": artifact.path_truncated
                    || artifact.path.len() > MAX_INSTALL_STATE_DISPLAY_PATH_BYTES,
            })
        })
        .collect::<Vec<_>>();
    let mut issues = probe
        .issues
        .iter()
        .take(MAX_INSTALL_STATE_ISSUES - 1)
        .map(|issue| {
            let display_path = issue
                .path
                .as_ref()
                .map(|path| truncate_utf8(path.clone(), MAX_INSTALL_STATE_DISPLAY_PATH_BYTES));
            json!({
                "kind": install_issue_kind_label(issue.kind),
                "display_path": display_path,
                "message": truncate_utf8(
                    issue.message.clone(),
                    MAX_INSTALL_STATE_MESSAGE_BYTES,
                ),
                "path_truncated": issue.path_truncated || issue.path.as_ref().is_some_and(
                    |path| path.len() > MAX_INSTALL_STATE_DISPLAY_PATH_BYTES
                ),
                "message_truncated": issue.message_truncated
                    || issue.message.len() > MAX_INSTALL_STATE_MESSAGE_BYTES,
            })
        })
        .collect::<Vec<_>>();
    match &deploy_recovery {
        DeployRecoveryProbe::NotRequired => {}
        DeployRecoveryProbe::Required => artifacts.push(json!({
            "kind": "deploy_recovery_record",
            "display_path": "gore-mod.deployed.json",
            "path_truncated": false,
        })),
        DeployRecoveryProbe::InspectionFailed(message) => issues.push(json!({
            "kind": "deploy_recovery_inspection",
            "display_path": Value::Null,
            "message": truncate_utf8(message.clone(), MAX_INSTALL_STATE_MESSAGE_BYTES),
            "path_truncated": false,
            "message_truncated": message.len() > MAX_INSTALL_STATE_MESSAGE_BYTES,
        })),
    }
    let disposition = if matches!(&deploy_recovery, DeployRecoveryProbe::InspectionFailed(_))
        || probe.disposition == InstallCompileStateDisposition::InspectionFailed
    {
        InstallCompileStateDisposition::InspectionFailed
    } else if matches!(&deploy_recovery, DeployRecoveryProbe::Required) {
        InstallCompileStateDisposition::RecoveryArtifactsPresent
    } else {
        probe.disposition
    };
    json!({
        "ok": true,
        "disposition": install_state_disposition_label(disposition),
        "safe_to_compile": disposition == InstallCompileStateDisposition::SafeToCompile,
        "game_process": install_game_process_label(probe.game_process),
        "artifacts": artifacts,
        "issues": issues,
    })
}

fn parse_request(input: &str) -> Result<CompileWirePayload, &'static str> {
    if input.len() > MAX_WIRE_BYTES {
        return Err("compile-report request exceeds its bounded wire limit");
    }
    let request: ExactWireRequest =
        serde_json::from_str(input).map_err(|_| "compile-report request has an invalid schema")?;
    if request.command != COMMAND {
        return Err("compile-report request command does not match this route");
    }
    validate_path(&request.payload.game_dir)?;
    validate_path(&request.payload.as_path)?;
    validate_path(&request.payload.work_dir)?;
    if request.payload.module_name.is_empty()
        || request.payload.module_name.len() > MAX_MODULE_NAME_BYTES
        || request.payload.module_name.contains('\0')
    {
        return Err("module_name is empty or exceeds its bounded length");
    }
    if request.payload.rel_path.is_empty()
        || request.payload.rel_path.len() > MAX_REL_PATH_BYTES
        || request.payload.rel_path.contains('\0')
    {
        return Err("rel_path is empty or exceeds its bounded length");
    }
    if !matches!(request.payload.op.as_str(), "add" | "edit") {
        return Err("op must be exactly 'add' or 'edit'");
    }
    Ok(request.payload)
}

fn parse_install_state_request(input: &str) -> Result<InstallStateWirePayload, &'static str> {
    if input.len() > MAX_INSTALL_STATE_WIRE_BYTES {
        return Err("compile-install-state request exceeds its bounded wire limit");
    }
    let request: ExactInstallStateWireRequest = serde_json::from_str(input)
        .map_err(|_| "compile-install-state request has an invalid schema")?;
    if request.command != INSTALL_STATE_COMMAND {
        return Err("compile-install-state request command does not match this route");
    }
    validate_path(&request.payload.game_dir)?;
    Ok(request.payload)
}

fn validate_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err("a path is empty or exceeds its bounded length");
    }
    Ok(())
}

fn preflight_failure(code: &'static str, message: String) -> Value {
    preflight_failure_with_state(code, message, false)
}

fn preflight_failure_with_state(
    code: &'static str,
    message: String,
    recovery_required: bool,
) -> Value {
    json!({
        "ok": true,
        "outcome": "failed",
        "mini_path": Value::Null,
        "module": Value::Null,
        "compile_error": {
            "code": code,
            "message": truncate_utf8(message, MAX_ERROR_MESSAGE_BYTES),
        },
        "compiler_diagnostics": Value::Null,
        "install_restore": "not_started",
        "recovery_required": recovery_required,
    })
}

fn install_guard_failure(game_dir: &Path, message: String) -> Value {
    let state = probe_install_compile_state(game_dir);
    let (code, recovery_required) = match state.disposition {
        InstallCompileStateDisposition::GameProcessRunning => {
            ("COMPILE_GAME_PROCESS_RUNNING", false)
        }
        InstallCompileStateDisposition::RecoveryArtifactsPresent => {
            ("COMPILE_INSTALL_RECOVERY_REQUIRED", true)
        }
        InstallCompileStateDisposition::InspectionFailed => {
            ("COMPILE_INSTALL_INSPECTION_FAILED", false)
        }
        InstallCompileStateDisposition::SafeToCompile => {
            ("COMPILE_INSTALL_GUARD_UNAVAILABLE", false)
        }
    };
    preflight_failure_with_state(code, message, recovery_required)
}

fn release_guard_after_preflight_failure(
    mut guard: InstallMutationGuard,
    failure: Value,
    context: &'static str,
) -> Value {
    match guard.release() {
        Ok(()) => failure,
        Err(error) => {
            // A release failure is itself persistent recovery state. Do not let Drop retry and
            // possibly erase the only blocker/evidence after the response claims recovery.
            guard.preserve_for_manual_recovery();
            guard_release_failure(context, error)
        }
    }
}

fn guard_release_failure(context: &'static str, error: String) -> Value {
    preflight_failure_with_state(
        "COMPILE_INSTALL_GUARD_RELEASE_FAILED",
        format!("{context}; install guard release failed: {error}"),
        true,
    )
}

fn report_response(report: CompileModuleReport, output_rejection: Option<String>) -> Value {
    let restore = report.install_restore_disposition();
    let diagnostics_rejection = compiled_diagnostics_rejection(report.diagnostics());
    let diagnostics = report
        .diagnostics()
        .map(|report| diagnostics_json(report.disposition(), report.diagnostics()));
    let install_restore = install_restore_label(restore);
    let recovery_required = matches!(
        restore,
        InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
            | InstallRestoreDisposition::RecoveryRequiredRestoreFailed
    );

    match report.outcome {
        CompileModuleReportOutcome::Compiled(output) => compiled_response(
            output,
            restore,
            diagnostics_rejection,
            output_rejection,
            diagnostics,
            install_restore,
            recovery_required,
        ),
        CompileModuleReportOutcome::Failed(error) => {
            let (code, message) = compile_error(error);
            preflight_failure_with_diagnostics(
                code,
                &message,
                diagnostics,
                install_restore,
                recovery_required,
            )
        }
    }
}

fn compiled_response(
    output: gore_as::compile::CompileOutput,
    restore: InstallRestoreDisposition,
    diagnostics_rejection: Option<(&'static str, &'static str)>,
    output_rejection: Option<String>,
    diagnostics: Option<Value>,
    install_restore: &'static str,
    recovery_required: bool,
) -> Value {
    if restore != InstallRestoreDisposition::RestoredExact {
        return preflight_failure_with_diagnostics(
            "COMPILE_RESTORE_INVARIANT",
            "the compiler produced output without proving an exact installation restore",
            diagnostics,
            install_restore,
            recovery_required,
        );
    }
    if let Some((code, message)) = diagnostics_rejection {
        return preflight_failure_with_diagnostics(
            code,
            message,
            diagnostics,
            install_restore,
            false,
        );
    }
    if let Some(message) = output_rejection {
        return preflight_failure_with_diagnostics(
            "COMPILE_OUTPUT_UNSAFE",
            &message,
            diagnostics,
            install_restore,
            false,
        );
    }
    json!({
        "ok": true,
        "outcome": "compiled",
        "mini_path": output.mini_path.display().to_string(),
        "module": output.module_name,
        "compile_error": Value::Null,
        "compiler_diagnostics": diagnostics,
        "install_restore": install_restore,
        "recovery_required": false,
    })
}

fn compiled_output_is_usable(
    restore: InstallRestoreDisposition,
    diagnostics_rejection: Option<(&'static str, &'static str)>,
    output_is_anchored: bool,
) -> bool {
    restore == InstallRestoreDisposition::RestoredExact
        && diagnostics_rejection.is_none()
        && output_is_anchored
}

fn compiled_diagnostics_rejection(
    report: Option<&gore_as::diagnostics::CompilerDiagnosticsReport>,
) -> Option<(&'static str, &'static str)> {
    match report {
        Some(report) => {
            compiled_diagnostics_rejection_parts(Some(report.disposition()), report.diagnostics())
        }
        None => compiled_diagnostics_rejection_parts(None, &[]),
    }
}

fn compiled_diagnostics_rejection_parts(
    disposition: Option<DiagnosticsCaptureDisposition>,
    diagnostics: &[CompilerDiagnostic],
) -> Option<(&'static str, &'static str)> {
    let rejection = match disposition {
        None => Some((
            "COMPILE_DIAGNOSTICS_MISSING",
            "the compiler produced output without a structured diagnostics disposition",
        )),
        Some(DiagnosticsCaptureDisposition::CaptureInvalid) => Some((
            "COMPILE_DIAGNOSTICS_INVALID",
            "the compiler diagnostics capture was invalid; compiled output was discarded",
        )),
        Some(DiagnosticsCaptureDisposition::Disabled) => Some((
            "COMPILE_DIAGNOSTICS_DISABLED",
            "structured diagnostics were unexpectedly disabled; compiled output was discarded",
        )),
        Some(
            DiagnosticsCaptureDisposition::UnavailableWithoutFallback
            | DiagnosticsCaptureDisposition::ProcessExitUnconfirmed,
        ) => Some((
            "COMPILE_DIAGNOSTICS_UNUSABLE",
            "the compiler diagnostics attempt did not reach a usable capture or fallback",
        )),
        Some(
            DiagnosticsCaptureDisposition::Captured
            | DiagnosticsCaptureDisposition::UnavailableFallback,
        ) => None,
    };
    rejection.or_else(|| {
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .then_some((
                "COMPILE_DIAGNOSTICS_REPORTED_ERROR",
                "the compiler reported an error; compiled output was discarded",
            ))
    })
}

fn preflight_failure_with_diagnostics(
    code: &'static str,
    message: &str,
    diagnostics: Option<Value>,
    install_restore: &'static str,
    recovery_required: bool,
) -> Value {
    json!({
        "ok": true,
        "outcome": "failed",
        "mini_path": Value::Null,
        "module": Value::Null,
        "compile_error": {
            "code": code,
            "message": truncate_utf8(message.to_owned(), MAX_ERROR_MESSAGE_BYTES),
        },
        "compiler_diagnostics": diagnostics,
        "install_restore": install_restore,
        "recovery_required": recovery_required,
    })
}

fn compile_error(error: CompileError) -> (&'static str, String) {
    let code = match &error {
        CompileError::Io(_) => "COMPILE_IO",
        CompileError::Regen(_) => "COMPILER_REGEN_FAILED",
        CompileError::NoRegen(_) => "COMPILER_OUTPUT_MISSING",
        CompileError::Other(_) => "COMPILE_FAILED",
    };
    (code, error.to_string())
}

fn install_restore_label(disposition: InstallRestoreDisposition) -> &'static str {
    match disposition {
        InstallRestoreDisposition::NotStarted => "not_started",
        InstallRestoreDisposition::RestoredExact => "restored_exact",
        InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed => {
            "recovery_required_process_exit_unconfirmed"
        }
        InstallRestoreDisposition::RecoveryRequiredRestoreFailed => {
            "recovery_required_restore_failed"
        }
    }
}

fn install_state_disposition_label(disposition: InstallCompileStateDisposition) -> &'static str {
    match disposition {
        InstallCompileStateDisposition::SafeToCompile => "safe_to_compile",
        InstallCompileStateDisposition::GameProcessRunning => "game_process_running",
        InstallCompileStateDisposition::RecoveryArtifactsPresent => "recovery_artifacts_present",
        InstallCompileStateDisposition::InspectionFailed => "inspection_failed",
    }
}

fn install_game_process_label(disposition: InstallCompileGameProcessDisposition) -> &'static str {
    match disposition {
        InstallCompileGameProcessDisposition::NotRunning => "not_running",
        InstallCompileGameProcessDisposition::Running => "running",
        InstallCompileGameProcessDisposition::InspectionFailed => "inspection_failed",
    }
}

fn install_artifact_kind_label(kind: InstallCompileArtifactKind) -> &'static str {
    match kind {
        InstallCompileArtifactKind::InstallMutationLock => "install_mutation_lock",
        InstallCompileArtifactKind::CompileLock => "compile_lock",
        InstallCompileArtifactKind::RecoveryJournal => "recovery_journal",
        InstallCompileArtifactKind::ShippingCacheBackup => "shipping_cache_backup",
        InstallCompileArtifactKind::JittedCodeBackup => "jitted_code_backup",
        InstallCompileArtifactKind::Ue4ssProxyBackup => "ue4ss_proxy_backup",
    }
}

fn install_issue_kind_label(kind: InstallCompileInspectionIssueKind) -> &'static str {
    match kind {
        InstallCompileInspectionIssueKind::GameProcessEnumeration => "game_process_enumeration",
        InstallCompileInspectionIssueKind::ArtifactMetadata => "artifact_metadata",
    }
}

fn diagnostics_json(
    disposition: DiagnosticsCaptureDisposition,
    diagnostics: &[CompilerDiagnostic],
) -> Value {
    let mut text_bytes = 0usize;
    let mut projected = Vec::new();
    for diagnostic in diagnostics.iter().take(MAX_WIRE_DIAGNOSTICS) {
        let Some(next_bytes) = text_bytes
            .checked_add(diagnostic.file.len())
            .and_then(|size| size.checked_add(diagnostic.message.len()))
        else {
            break;
        };
        if next_bytes > MAX_WIRE_DIAGNOSTIC_TEXT_BYTES {
            break;
        }
        text_bytes = next_bytes;
        projected.push(json!({
            "file": diagnostic.file,
            "line": diagnostic.line,
            "column": diagnostic.column,
            "severity": severity_label(diagnostic.severity),
            "message": diagnostic.message,
        }));
    }
    let omitted = diagnostics.len().saturating_sub(projected.len());
    json!({
        "capture": diagnostics_capture_label(disposition),
        "messages": projected,
        "omitted": omitted,
    })
}

fn diagnostics_capture_label(disposition: DiagnosticsCaptureDisposition) -> &'static str {
    match disposition {
        DiagnosticsCaptureDisposition::Captured => "captured",
        DiagnosticsCaptureDisposition::CaptureInvalid => "capture_invalid",
        DiagnosticsCaptureDisposition::UnavailableFallback => "unavailable_fallback",
        DiagnosticsCaptureDisposition::UnavailableWithoutFallback => "unavailable_without_fallback",
        DiagnosticsCaptureDisposition::ProcessExitUnconfirmed => "process_exit_unconfirmed",
        DiagnosticsCaptureDisposition::Disabled => "disabled",
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Note => "note",
    }
}

fn truncate_utf8(mut text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes.saturating_sub(3).min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str("...");
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_as::compile::{InstallCompileArtifact, InstallCompileInspectionIssue};
    use std::fs;

    fn request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_payload(root: &Path) -> Value {
        json!({
            "allow_new_symbols": false,
            "as_path": root.join("Probe.as").display().to_string(),
            "game_dir": root.display().to_string(),
            "module_name": "GoreMods.Probe",
            "op": "add",
            "rel_path": "GoreMods/Probe.as",
            "work_dir": root.join("work").display().to_string(),
        })
    }

    #[test]
    fn missing_pristine_base_returns_a_non_launching_structured_failure() {
        let root = tempfile::tempdir().unwrap();
        let response = compile_report_v1_raw(&request(valid_payload(root.path())));

        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "failed");
        assert_eq!(
            response["compile_error"]["code"],
            "COMPILE_BASE_UNAVAILABLE"
        );
        assert_eq!(response["install_restore"], "not_started");
        assert_eq!(response["recovery_required"], false);
        assert!(response["compiler_diagnostics"].is_null());
        assert!(!root.path().join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn caller_work_tree_is_never_used_or_removed_as_compile_staging() {
        let root = tempfile::tempdir().unwrap();
        let victim = root.path().join("victim");
        let victim_tree = victim.join("tree");
        let game = root.path().join("game");
        fs::create_dir_all(&victim_tree).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::write(victim_tree.join("keep.txt"), b"do not delete").unwrap();

        let staging = OwnedCompileStaging::create(&victim, &game).unwrap();
        assert_eq!(staging.path().parent(), Some(victim.as_path()));
        #[cfg(windows)]
        assert!(!staging.path().display().to_string().starts_with(r"\\?\"));
        assert_eq!(
            staging.path().parent().unwrap().canonicalize().unwrap(),
            victim.canonicalize().unwrap()
        );
        let basename = staging.path().file_name().unwrap().to_str().unwrap();
        assert!(basename.starts_with(OWNED_COMPILE_PREFIX));
        assert_eq!(basename.len(), OWNED_COMPILE_PREFIX.len() + 12);
        assert!(basename[OWNED_COMPILE_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric()));
        assert_eq!(
            fs::read(staging.path().join(OWNED_COMPILE_MARKER)).unwrap(),
            OWNED_COMPILE_MARKER_CONTENT
        );
        fs::create_dir_all(staging.path().join("tree")).unwrap();
        fs::write(staging.path().join("tree/owned.txt"), b"owned").unwrap();
        let staging_path = staging.path().to_path_buf();

        drop(staging);

        assert!(staging_path.exists());
        assert_eq!(
            fs::read(victim_tree.join("keep.txt")).unwrap(),
            b"do not delete"
        );
        fs::remove_dir_all(staging_path).unwrap();
    }

    #[test]
    fn direct_g1r_game_dir_rejects_work_inside_semantic_install_root() {
        let root = tempfile::tempdir().unwrap();
        let install = root.path().join("install");
        let direct_g1r = install.join("g1R");
        let workspace = install.join("compile-work");
        fs::create_dir_all(&direct_g1r).unwrap();
        fs::create_dir_all(&workspace).unwrap();

        let error = OwnedCompileStaging::create(&workspace, &direct_g1r)
            .err()
            .expect("a direct-G1R path must still protect the whole install root");
        assert!(
            error.contains("outside the selected game installation"),
            "got: {error}"
        );
        assert!(
            fs::read_dir(&workspace).unwrap().next().is_none(),
            "rejected staging must not create an owned child"
        );
    }

    #[test]
    fn compiled_mini_must_be_the_exact_owned_regular_file() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let game = root.path().join("game");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&game).unwrap();
        let staging = OwnedCompileStaging::create(&workspace, &game).unwrap();
        let staging_path = staging.path().to_path_buf();
        let exact = staging.path().join("module.cache");
        fs::write(&exact, b"mini").unwrap();
        let output = gore_as::compile::CompileOutput {
            mini_path: exact.clone(),
            module_name: "GoreMods.Probe".to_owned(),
        };
        let anchor = anchor_owned_compiled_mini(&staging, &output).unwrap();
        assert!(anchor.metadata().unwrap().is_file());
        drop(anchor);

        let wrong = gore_as::compile::CompileOutput {
            mini_path: staging.path().join("elsewhere.cache"),
            module_name: "GoreMods.Probe".to_owned(),
        };
        assert!(anchor_owned_compiled_mini(&staging, &wrong).is_err());
        fs::remove_file(&exact).unwrap();
        fs::create_dir(&exact).unwrap();
        assert!(anchor_owned_compiled_mini(&staging, &output).is_err());

        let response = compiled_response(
            output,
            InstallRestoreDisposition::RestoredExact,
            None,
            Some("compiled module.cache failed its owned-file check".to_owned()),
            None,
            "restored_exact",
            false,
        );
        assert_eq!(response["outcome"], "failed");
        assert_eq!(response["compile_error"]["code"], "COMPILE_OUTPUT_UNSAFE");
        assert!(response["mini_path"].is_null());
        assert!(response["module"].is_null());
        assert!(!compiled_output_is_usable(
            InstallRestoreDisposition::RestoredExact,
            None,
            false,
        ));

        drop(staging);
        fs::remove_dir_all(staging_path).unwrap();
    }

    #[test]
    fn child_replacement_before_identity_pin_fails_closed() {
        use std::sync::{Arc, Mutex};

        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let victim = root.path().join("victim");
        let victim_tree = victim.join("tree");
        let game = root.path().join("game");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&victim_tree).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::write(victim_tree.join("keep.txt"), b"do not delete").unwrap();
        let replacement_path = Arc::new(Mutex::new(None::<PathBuf>));
        let hook_path = Arc::clone(&replacement_path);
        let victim_for_hook = victim.clone();
        inject_owned_child_create_hook(move |candidate| {
            if fs::remove_dir(candidate).is_err() {
                return;
            }
            *hook_path.lock().unwrap() = Some(candidate.to_path_buf());
            #[cfg(unix)]
            std::os::unix::fs::symlink(&victim_for_hook, candidate).unwrap();
            #[cfg(windows)]
            {
                let status = std::process::Command::new("cmd")
                    .args(["/c", "mklink", "/J"])
                    .arg(candidate)
                    .arg(&victim_for_hook)
                    .status()
                    .unwrap();
                assert!(status.success());
            }
        });

        let result = OwnedCompileStaging::create(&workspace, &game);
        let replaced = replacement_path.lock().unwrap().clone();
        if let Some(path) = replaced {
            let error = result.err().expect("a replaced child must fail closed");
            assert!(
                error.contains("link") || error.contains("reparse"),
                "{error}"
            );
            #[cfg(unix)]
            fs::remove_file(path).unwrap();
            #[cfg(windows)]
            fs::remove_dir(path).unwrap();
        } else if let Ok(staging) = result {
            // Some platforms may make the retained parent anchor block removal before the child
            // itself is opened. That is an equally safe outcome.
            let path = staging.path().to_path_buf();
            drop(staging);
            fs::remove_dir_all(path).unwrap();
        }
        assert_eq!(
            fs::read(victim_tree.join("keep.txt")).unwrap(),
            b"do not delete"
        );
    }

    #[test]
    fn linked_workspace_parent_cannot_redirect_staging_to_a_victim_tree() {
        let root = tempfile::tempdir().unwrap();
        let victim = root.path().join("victim");
        let workspace = victim.join("work");
        let victim_tree = workspace.join("tree");
        let linked_parent = root.path().join("linked-parent");
        let game = root.path().join("game");
        fs::create_dir_all(&victim_tree).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::write(victim_tree.join("keep.txt"), b"do not delete").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&victim, &linked_parent).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&linked_parent)
                .arg(&victim)
                .status()
                .unwrap();
            if !status.success() {
                return;
            }
        }

        let error = OwnedCompileStaging::create(&linked_parent.join("work"), &game)
            .err()
            .expect("a linked ancestor must fail closed");
        assert!(
            error.contains("link") || error.contains("reparse"),
            "{error}"
        );
        assert_eq!(
            fs::read(victim_tree.join("keep.txt")).unwrap(),
            b"do not delete"
        );

        #[cfg(unix)]
        fs::remove_file(&linked_parent).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&linked_parent).unwrap();
    }

    #[test]
    fn staging_failure_releases_the_install_guard_without_launching() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let script = game.join("G1R/Script");
        fs::create_dir_all(&script).unwrap();
        fs::write(script.join("PrecompiledScript_Shipping.Cache"), b"pristine").unwrap();
        let mut payload = valid_payload(&game);
        payload["work_dir"] = json!(root.path().join("missing-work").display().to_string());

        let response = compile_report_v1_raw(&request(payload));

        assert_eq!(response["outcome"], "failed");
        assert_eq!(
            response["compile_error"]["code"],
            "COMPILE_STAGING_UNAVAILABLE"
        );
        assert_eq!(response["recovery_required"], false);
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn guard_release_failure_dominates_an_earlier_preflight_failure() {
        let response = guard_release_failure(
            "test preflight failed",
            "injected release failure".to_owned(),
        );

        assert_eq!(
            response["compile_error"]["code"],
            "COMPILE_INSTALL_GUARD_RELEASE_FAILED"
        );
        assert_eq!(response["recovery_required"], true);
    }

    #[test]
    fn deploy_recovery_is_a_dominant_structured_preflight_failure() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        fs::create_dir_all(&game).unwrap();
        let record = gore_mod::DeployRecord {
            mod_name: "fixture".to_owned(),
            phase: gore_mod::DeployPhase::RecoveryRequired,
            ..Default::default()
        };
        fs::write(
            game.join("gore-mod.deployed.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        let response = compile_report_v1_raw(&request(valid_payload(&game)));

        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "failed");
        assert_eq!(
            response["compile_error"]["code"],
            "COMPILE_BASE_RECOVERY_REQUIRED"
        );
        assert_eq!(response["install_restore"], "not_started");
        assert_eq!(response["recovery_required"], true);
    }

    #[test]
    fn request_schema_is_closed_and_operation_is_explicit() {
        let root = tempfile::tempdir().unwrap();
        let mut payload = valid_payload(root.path());
        payload["surprise"] = json!(true);
        let unknown = compile_report_v1_raw(&request(payload));
        assert_eq!(
            unknown["error"]["code"],
            "SCRIPT_COMPILE_REPORT_BAD_REQUEST"
        );

        let mut payload = valid_payload(root.path());
        payload["op"] = json!("replace");
        let operation = compile_report_v1_raw(&request(payload));
        assert_eq!(
            operation["error"]["code"],
            "SCRIPT_COMPILE_REPORT_BAD_REQUEST"
        );
    }

    #[test]
    fn install_state_schema_is_closed_and_projection_is_bounded() {
        let root = tempfile::tempdir().unwrap();
        let unknown = install_state_v1_raw(
            &json!({
                "command": INSTALL_STATE_COMMAND,
                "payload": {
                    "game_dir": root.path().display().to_string(),
                    "surprise": true,
                }
            })
            .to_string(),
        );
        assert_eq!(
            unknown["error"]["code"],
            "SCRIPT_COMPILE_INSTALL_STATE_BAD_REQUEST"
        );

        let artifacts = (0..(MAX_INSTALL_STATE_ARTIFACTS + 2))
            .map(|_| InstallCompileArtifact {
                kind: InstallCompileArtifactKind::RecoveryJournal,
                path: "x".repeat(MAX_INSTALL_STATE_DISPLAY_PATH_BYTES + 10),
                path_truncated: false,
            })
            .collect();
        let issues = (0..(MAX_INSTALL_STATE_ISSUES + 2))
            .map(|_| InstallCompileInspectionIssue {
                kind: InstallCompileInspectionIssueKind::ArtifactMetadata,
                path: None,
                path_truncated: false,
                message: "inspection failed".to_owned(),
                message_truncated: false,
            })
            .collect();
        let response = install_state_response(
            InstallCompileStateProbe {
                disposition: InstallCompileStateDisposition::InspectionFailed,
                safe_to_compile: false,
                game_process: InstallCompileGameProcessDisposition::InspectionFailed,
                artifacts,
                issues,
            },
            DeployRecoveryProbe::InspectionFailed("deploy inspection failed".to_owned()),
        );

        assert_eq!(response["ok"], true);
        assert_eq!(response["disposition"], "inspection_failed");
        assert_eq!(response["safe_to_compile"], false);
        assert_eq!(response["game_process"], "inspection_failed");
        assert_eq!(
            response["artifacts"].as_array().unwrap().len(),
            MAX_INSTALL_STATE_ARTIFACTS - 1
        );
        assert_eq!(
            response["issues"].as_array().unwrap().len(),
            MAX_INSTALL_STATE_ISSUES
        );
        assert_eq!(response["artifacts"][0]["kind"], "recovery_journal");
        assert_eq!(response["artifacts"][0]["path_truncated"], true);
        assert!(
            response["artifacts"][0]["display_path"]
                .as_str()
                .unwrap()
                .len()
                <= MAX_INSTALL_STATE_DISPLAY_PATH_BYTES
        );
        assert!(response["issues"][0]["display_path"].is_null());
        assert_eq!(
            response["issues"][MAX_INSTALL_STATE_ISSUES - 1]["kind"],
            "deploy_recovery_inspection"
        );
    }

    #[test]
    fn install_state_projects_persistent_deploy_recovery_after_restart() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        fs::create_dir_all(&game).unwrap();
        let record = gore_mod::DeployRecord {
            mod_name: "fixture".to_owned(),
            phase: gore_mod::DeployPhase::RecoveryRequired,
            ..Default::default()
        };
        fs::write(
            game.join("gore-mod.deployed.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();

        let response = install_state_v1_raw(
            &json!({
                "command": INSTALL_STATE_COMMAND,
                "payload": {"game_dir": game.display().to_string()},
            })
            .to_string(),
        );

        assert_eq!(response["ok"], true);
        assert_eq!(response["disposition"], "recovery_artifacts_present");
        assert_eq!(response["safe_to_compile"], false);
        assert!(response["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                artifact["kind"] == "deploy_recovery_record"
                    && artifact["display_path"] == "gore-mod.deployed.json"
            }));
    }

    #[test]
    fn diagnostics_projection_is_bounded_and_preserves_compiler_coordinates() {
        let diagnostics = (0..(MAX_WIRE_DIAGNOSTICS + 2))
            .map(|index| CompilerDiagnostic {
                file: "GoreMods/Probe.as".into(),
                line: index as u32 + 1,
                column: 7,
                severity: if index == 0 {
                    DiagnosticSeverity::Error
                } else {
                    DiagnosticSeverity::Warning
                },
                message: format!("message {index}"),
            })
            .collect::<Vec<_>>();
        let projected = diagnostics_json(DiagnosticsCaptureDisposition::Captured, &diagnostics);

        assert_eq!(projected["capture"], "captured");
        assert_eq!(
            projected["messages"].as_array().unwrap().len(),
            MAX_WIRE_DIAGNOSTICS
        );
        assert_eq!(projected["omitted"], 2);
        assert_eq!(projected["messages"][0]["line"], 1);
        assert_eq!(projected["messages"][0]["column"], 7);
        assert_eq!(projected["messages"][0]["severity"], "error");
    }

    #[test]
    fn unusable_or_error_diagnostics_discard_compiled_output() {
        let error = CompilerDiagnostic {
            file: "GoreMods/Probe.as".into(),
            line: 4,
            column: 2,
            severity: DiagnosticSeverity::Error,
            message: "syntax error".into(),
        };
        let warning = CompilerDiagnostic {
            severity: DiagnosticSeverity::Warning,
            ..error.clone()
        };
        assert!(compiled_diagnostics_rejection_parts(
            Some(DiagnosticsCaptureDisposition::Captured),
            std::slice::from_ref(&warning),
        )
        .is_none());
        for (disposition, diagnostics, expected_code) in [
            (
                DiagnosticsCaptureDisposition::CaptureInvalid,
                Vec::new(),
                "COMPILE_DIAGNOSTICS_INVALID",
            ),
            (
                DiagnosticsCaptureDisposition::Disabled,
                Vec::new(),
                "COMPILE_DIAGNOSTICS_DISABLED",
            ),
            (
                DiagnosticsCaptureDisposition::Captured,
                vec![error.clone()],
                "COMPILE_DIAGNOSTICS_REPORTED_ERROR",
            ),
        ] {
            let rejection = compiled_diagnostics_rejection_parts(Some(disposition), &diagnostics);
            assert_eq!(rejection.map(|(code, _)| code), Some(expected_code));
            assert!(!compiled_output_is_usable(
                InstallRestoreDisposition::RestoredExact,
                rejection,
                true,
            ));
            let response = compiled_response(
                gore_as::compile::CompileOutput {
                    mini_path: PathBuf::from("must-not-escape.cache"),
                    module_name: "GoreMods.Probe".to_owned(),
                },
                InstallRestoreDisposition::RestoredExact,
                rejection,
                None,
                Some(diagnostics_json(disposition, &diagnostics)),
                "restored_exact",
                false,
            );
            assert_eq!(response["outcome"], "failed");
            assert_eq!(response["compile_error"]["code"], expected_code);
            assert!(response["mini_path"].is_null());
            assert!(response["module"].is_null());
        }
    }

    #[test]
    fn disposition_labels_cover_fallback_and_recovery_states() {
        assert_eq!(
            diagnostics_capture_label(DiagnosticsCaptureDisposition::UnavailableFallback),
            "unavailable_fallback"
        );
        assert_eq!(
            install_restore_label(
                InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
            ),
            "recovery_required_process_exit_unconfirmed"
        );
        assert_eq!(
            install_restore_label(InstallRestoreDisposition::RecoveryRequiredRestoreFailed),
            "recovery_required_restore_failed"
        );
    }
}
