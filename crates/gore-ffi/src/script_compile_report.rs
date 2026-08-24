//! Bounded structured AngelScript module compilation for Mod Studio.
//!
//! Compiler diagnostics/capture disposition and live-install restoration are exposed as data. The
//! game compiler is entered only after a drift-aware pristine base has been resolved. Once entered,
//! gore-as owns the transactional backup/stage/restore window and the response never claims a
//! usable mini-cache unless that window closed with an exact restore.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[cfg(not(test))]
use gore_as::compile::acquire_compile_install_mutation;
use gore_as::compile::{
    compile_module_with_backend_v1, compile_module_with_backend_v1_with_guard_and_target,
    compile_module_with_diagnostics_report_with_guard, probe_install_compile_state, CompileError,
    CompileModuleReport, CompileModuleReportOutcome, CompileOpts, CompilerBackendModeV1,
    CompilerBackendNameV1, InstallCompileArtifactKind, InstallCompileGameProcessDisposition,
    InstallCompileInspectionIssueKind, InstallCompileStateDisposition, InstallCompileStateProbe,
    InstallMutationGuard, InstallRestoreDisposition,
};
use gore_as::compiler_backend::CompilerBackendDiagnosticSeverityV1;
use gore_as::diagnostics::{
    CompilerDiagnostic, DiagnosticSeverity, DiagnosticsCaptureDisposition, DiagnosticsOptions,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::err;
use crate::standalone_compiler_package::{
    backend_evidence, backend_evidence_with_package, bundle_absent_fallback_reason,
    package_unavailable_fallback_reason, resolve_product_standalone_compiler_for_game_v1,
    CompilerBackendWireV2, ResolvedProductStandaloneCompilerV1, BUNDLE_ABSENT_DETAIL,
};

pub(super) const COMMAND: &str = "script_compile_report_v1";
pub(super) const COMMAND_V2: &str = "script_compile_report_v2";
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
struct ExactWireRequestV2 {
    command: String,
    payload: CompileWirePayloadV2,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompileWirePayloadV2 {
    allow_new_symbols: bool,
    as_path: String,
    compiler_backend: CompilerBackendWireV2,
    game_dir: String,
    module_name: String,
    op: String,
    rel_path: String,
    work_dir: String,
}

impl CompileWirePayloadV2 {
    fn into_v1(self) -> CompileWirePayload {
        CompileWirePayload {
            allow_new_symbols: self.allow_new_symbols,
            as_path: self.as_path,
            game_dir: self.game_dir,
            module_name: self.module_name,
            op: self.op,
            rel_path: self.rel_path,
            work_dir: self.work_dir,
        }
    }
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
pub(super) struct OwnedCompileStaging {
    path: PathBuf,
    workspace_real: PathBuf,
    _anchors: Vec<std::fs::File>,
}

impl OwnedCompileStaging {
    pub(super) fn create(workspace: &Path, game_dir: &Path) -> Result<Self, String> {
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

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn verify_owned(&self) -> Result<(), String> {
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
type OwnedChildCreateHook = Box<dyn FnOnce(&Path)>;

#[cfg(test)]
thread_local! {
    static OWNED_CHILD_CREATE_HOOK: std::cell::RefCell<Option<OwnedChildCreateHook>> =
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
) -> Result<(), String> {
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
    // Reuse the exact create-new handle retained by gore-as. Reopening with a second handle is
    // both weaker against replacement and a Windows sharing violation because the creation handle
    // intentionally denies WRITE/DELETE sharing.
    output.validate_retained_artifact()
}

/// Remove the exact compiler artifact while the owned staging directory remains identity-pinned.
/// Managed compiler checks return evidence only, never a reusable mini-cache capability.
pub(super) fn discard_owned_compiled_mini(
    staging: &OwnedCompileStaging,
    output: &mut gore_as::compile::CompileOutput,
) -> Result<(), String> {
    let expected = staging.path().join("module.cache");
    if output.mini_path != expected {
        return Err(
            "compiler output path is not the exact owned-staging module.cache child".to_owned(),
        );
    }
    // The exact retained output handle is the last reliable capability once staging verification
    // fails. Always destroy its bytes before consulting the marker, and still run both operations
    // so a caller does not lose either failure when disposal and ownership verification fail.
    let neutralization_error = output.neutralize_retained_artifact().err();
    let ownership_error = staging.verify_owned().err();
    match (neutralization_error, ownership_error) {
        (Some(neutralization), Some(ownership)) => {
            return Err(format!(
                "neutralizing the exact retained compiler output failed: {neutralization}; \
                 compile-staging ownership verification also failed: {ownership}"
            ));
        }
        (Some(error), None) | (None, Some(error)) => return Err(error),
        (None, None) => {}
    }

    // The exact original file was neutralized before its retained handle was released. The
    // identity-pinned staging path is now verified, so pathname cleanup is safe to attempt.
    let _ = std::fs::remove_file(&expected);
    confirm_compiled_output_absent_or_empty(&expected)
}

/// Destroy a partial `module.cache` that may have been left behind by a failed compiler write.
/// Absence is success; any present bytes must be neutralized through the exact opened handle.
pub(super) fn discard_owned_compiled_mini_if_present(
    staging: &OwnedCompileStaging,
) -> Result<(), String> {
    discard_owned_compiled_mini_path(staging, true)
}

/// Destroy a partial final write through gore-as's retained creation handle when available. Other
/// failure modes never created a genuine mini-cache; a same-name residue is still removed closed.
pub(super) fn discard_owned_failed_compiled_mini(
    staging: &OwnedCompileStaging,
    error: &mut CompileError,
) -> Result<(), String> {
    if let Some(neutralized) = error.neutralize_failed_artifact() {
        staging.verify_owned()?;
        neutralized?;
        let expected = staging.path().join("module.cache");
        let _ = std::fs::remove_file(&expected);
        return confirm_compiled_output_absent_or_empty(&expected);
    }
    discard_owned_compiled_mini_if_present(staging)
}

fn discard_owned_compiled_mini_path(
    staging: &OwnedCompileStaging,
    allow_missing: bool,
) -> Result<(), String> {
    staging.verify_owned()?;
    let path = staging.path().join("module.cache");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(())
        }
        Err(error) => {
            return Err(format!(
                "inspecting evidence-only compiler output without following links: {error}"
            ))
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("evidence-only compiler output is not a regular non-link file".to_owned());
    }

    // Neutralize the exact opened file before releasing the identity-protecting handle. In
    // particular, do not drop an anchor and then truncate by pathname: that creates a Windows
    // rename/replacement gap in which the real mini-cache can escape.
    let file = open_compiled_mini_discard_anchor(&path)?;
    file.set_len(0)
        .map_err(|error| format!("truncating the exact compiler output handle: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("syncing the exact truncated compiler output: {error}"))?;
    if file
        .metadata()
        .map_err(|error| format!("rechecking the exact compiler output handle: {error}"))?
        .len()
        != 0
    {
        return Err("the exact compiler output handle still contains usable bytes".to_owned());
    }
    drop(file);

    // Unlink is best-effort after the only compiler-produced bytes are already empty. A retained
    // empty file is safe and lets the app-owned outer-workspace cleanup report its own warning.
    let _ = std::fs::remove_file(&path);
    confirm_compiled_output_absent_or_empty(&path)
}

fn confirm_compiled_output_absent_or_empty(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(metadata)
            if metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() == 0 =>
        {
            Ok(())
        }
        Ok(_) => Err("evidence-only compiler output still contains usable bytes".to_owned()),
        Err(error) => Err(format!(
            "confirming evidence-only compiler output discard: {error}"
        )),
    }
}

#[cfg(windows)]
fn open_compiled_mini_discard_anchor(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|error| format!("opening compiler output without following links: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspecting opened compiler output: {error}"))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err("compiler output is not a regular non-reparse file".to_owned());
    }
    Ok(file)
}

#[cfg(unix)]
fn open_compiled_mini_discard_anchor(path: &Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|error| format!("opening compiler output without following links: {error}"))?;
    if !file
        .metadata()
        .map_err(|error| format!("inspecting opened compiler output: {error}"))?
        .is_file()
    {
        return Err("compiler output is not a regular file".to_owned());
    }
    Ok(file)
}

pub(super) fn compile_report_v1_raw(input: &str) -> Value {
    let payload = match parse_request(input) {
        Ok(payload) => payload,
        Err(message) => return err("SCRIPT_COMPILE_REPORT_BAD_REQUEST", message),
    };
    compile_report_v1_payload(payload)
}

pub(super) fn compile_report_v2_raw(input: &str) -> Value {
    let payload = match parse_request_v2(input) {
        Ok(payload) => payload,
        Err(message) => return err("SCRIPT_COMPILE_REPORT_V2_BAD_REQUEST", message),
    };
    let requested = payload.compiler_backend;
    match requested {
        CompilerBackendWireV2::Game => {
            let (response, game_attempted) =
                compile_report_v1_payload_with_attempt(payload.into_v1());
            attach_backend_evidence(
                response,
                backend_evidence(
                    requested,
                    game_attempted.then_some(CompilerBackendNameV1::Game),
                    false,
                    game_attempted,
                    None,
                ),
            )
        }
        CompilerBackendWireV2::Standalone | CompilerBackendWireV2::StandaloneThenGame => {
            compile_report_v2_product_payload(payload.into_v1(), requested)
        }
    }
}

fn compile_report_v2_product_payload(
    payload: CompileWirePayload,
    requested: CompilerBackendWireV2,
) -> Value {
    let game_dir = PathBuf::from(&payload.game_dir);
    let resolution = match resolve_product_standalone_compiler_for_game_v1(&game_dir) {
        Ok(resolution) => resolution,
        Err(message) => {
            if requested == CompilerBackendWireV2::Standalone {
                return attach_backend_evidence(
                    preflight_failure("COMPILE_STANDALONE_PACKAGE_LOCATION", message),
                    backend_evidence(requested, None, false, false, None),
                );
            }
            let (response, game_attempted) = compile_report_v1_payload_with_attempt(payload);
            return attach_backend_evidence(
                response,
                backend_evidence(
                    requested,
                    game_attempted.then_some(CompilerBackendNameV1::Game),
                    false,
                    game_attempted,
                    Some(json!({
                        "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
                        "failure_kind": "unavailable",
                        "detail": message,
                    })),
                ),
            );
        }
    };
    match resolution {
        ResolvedProductStandaloneCompilerV1::BundleAbsent => {
            if requested == CompilerBackendWireV2::Standalone {
                attach_backend_evidence(
                    preflight_failure(
                        "COMPILE_STANDALONE_BUNDLE_ABSENT",
                        BUNDLE_ABSENT_DETAIL.to_owned(),
                    ),
                    backend_evidence(requested, None, false, false, None),
                )
            } else {
                let (response, game_attempted) = compile_report_v1_payload_with_attempt(payload);
                attach_backend_evidence(
                    response,
                    backend_evidence(
                        requested,
                        game_attempted.then_some(CompilerBackendNameV1::Game),
                        false,
                        game_attempted,
                        Some(bundle_absent_fallback_reason()),
                    ),
                )
            }
        }
        ResolvedProductStandaloneCompilerV1::Unavailable(reason) => {
            if requested == CompilerBackendWireV2::Standalone {
                attach_backend_evidence(
                    preflight_failure(
                        "COMPILE_STANDALONE_PACKAGE_UNAVAILABLE",
                        format!("{:?}: {}", reason.kind(), reason.detail()),
                    ),
                    backend_evidence(requested, None, false, false, None),
                )
            } else {
                let fallback = package_unavailable_fallback_reason(&reason);
                let (response, game_attempted) = compile_report_v1_payload_with_attempt(payload);
                attach_backend_evidence(
                    response,
                    backend_evidence(
                        requested,
                        game_attempted.then_some(CompilerBackendNameV1::Game),
                        false,
                        game_attempted,
                        Some(fallback),
                    ),
                )
            }
        }
        ResolvedProductStandaloneCompilerV1::Available(package) => {
            compile_report_with_available_product_package(payload, requested, package)
        }
    }
}

fn compile_report_with_available_product_package(
    payload: CompileWirePayload,
    requested: CompilerBackendWireV2,
    package: gore_as::standalone_package_resolver::AvailableProductStandaloneCompilerPackageV1,
) -> Value {
    debug_assert!(requested != CompilerBackendWireV2::Game);
    let game_dir = PathBuf::from(&payload.game_dir);
    let staging = match OwnedCompileStaging::create(Path::new(&payload.work_dir), &game_dir) {
        Ok(staging) => staging,
        Err(message) => {
            return attach_backend_evidence(
                preflight_failure("COMPILE_STAGING_UNAVAILABLE", message),
                backend_evidence_with_package(
                    requested,
                    None,
                    false,
                    false,
                    Some(package.identity()),
                    None,
                ),
            );
        }
    };
    let config = gore_as::standalone_sidecar::StandaloneSidecarConfigV1::new(
        package.sidecar_path().to_path_buf(),
        package.sidecar_seal(),
        package.profile_manifest_path().to_path_buf(),
        package.profile_root().to_path_buf(),
        staging.path().to_path_buf(),
    );
    let runner = gore_as::standalone_sidecar::StandaloneSidecarRunnerV1::new(config);
    let (authority, target) = package.into_execution_parts();
    let mut runner_unavailable = None;
    let mut runner = match runner {
        Ok(runner) => Some(runner),
        Err(failure) if requested == CompilerBackendWireV2::Standalone => {
            let mut failure_detail = Value::String(failure.to_string());
            crate::authoring_story_compiler_revision3::redact_private_paths(
                &mut failure_detail,
                &[
                    staging.path(),
                    authority.sidecar_path(),
                    authority.profile_manifest_path(),
                    authority.profile_root(),
                ],
            );
            let failure_detail = failure_detail
                .as_str()
                .unwrap_or("standalone runner initialization failed")
                .to_owned();
            return attach_backend_evidence(
                preflight_failure("COMPILE_STANDALONE_RUNNER_UNAVAILABLE", failure_detail),
                backend_evidence_with_package(
                    requested,
                    None,
                    false,
                    false,
                    Some(authority.identity()),
                    None,
                ),
            );
        }
        Err(failure) => {
            let mut failure_detail = Value::String(failure.to_string());
            crate::authoring_story_compiler_revision3::redact_private_paths(
                &mut failure_detail,
                &[
                    staging.path(),
                    authority.sidecar_path(),
                    authority.profile_manifest_path(),
                    authority.profile_root(),
                ],
            );
            runner_unavailable = Some(json!({
                "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
                "failure_kind": failure.kind().as_str(),
                "detail": failure_detail
                    .as_str()
                    .unwrap_or("standalone runner initialization failed"),
            }));
            None
        }
    };
    let opts = CompileOpts {
        game_dir: game_dir.clone(),
        op: payload.op,
        module_name: payload.module_name,
        rel_path: payload.rel_path,
        as_path: PathBuf::from(payload.as_path),
        source_override: None,
        work_dir: staging.path().to_path_buf(),
        allow_new_symbols: payload.allow_new_symbols,
        base_override: Some(target.shipping_cache().to_vec()),
        binds_override: Some(target.binds_cache().to_vec()),
    };
    if let Err(message) = staging.verify_owned() {
        return attach_backend_evidence(
            preflight_failure("COMPILE_STAGING_CHANGED", message),
            backend_evidence_with_package(
                requested,
                None,
                false,
                false,
                Some(authority.identity()),
                runner_unavailable.clone(),
            ),
        );
    }
    let mut strict_target = Some(target);
    let report = if requested == CompilerBackendWireV2::Standalone {
        // `target` remains alive for the whole attempt and pins EXE/Shipping/Binds without ever
        // acquiring the install-mutation guard or touching the selected installation.
        let report = compile_module_with_backend_v1(
            &opts,
            &DiagnosticsOptions {
                disabled: false,
                hook_dll: None,
                inject_delay: Duration::from_secs(2),
            },
            CompilerBackendModeV1::Standalone,
            Some(
                runner
                    .as_mut()
                    .expect("strict standalone returned after runner initialization failure"),
            ),
        );
        report
    } else {
        let guard = match acquire_guard(&game_dir) {
            Ok(guard) => guard,
            Err(message) => {
                return attach_backend_evidence(
                    install_guard_failure(&game_dir, message),
                    backend_evidence_with_package(
                        requested,
                        None,
                        false,
                        false,
                        Some(authority.identity()),
                        runner_unavailable.clone(),
                    ),
                );
            }
        };
        compile_module_with_backend_v1_with_guard_and_target(
            &opts,
            &DiagnosticsOptions {
                disabled: false,
                hook_dll: None,
                inject_delay: Duration::from_secs(2),
            },
            CompilerBackendModeV1::StandaloneThenGame,
            runner.as_mut().map(|runner| runner as _),
            guard,
            strict_target
                .take()
                .expect("the authenticated target is transferred once"),
        )
    };
    let result_backend = report.backend_name();
    let standalone_attempted = report.standalone_attempted();
    let game_attempted = report.game_attempted();
    let fallback = runner_unavailable.or_else(|| {
        report.fallback_reason().map(|reason| {
            json!({
                "failed_backend": reason.failed_backend().as_str(),
                "failure_kind": reason.failure_kind().as_str(),
                "detail": reason.detail(),
            })
        })
    });
    let evidence = backend_evidence_with_package(
        requested,
        result_backend,
        standalone_attempted,
        game_attempted,
        Some(authority.identity()),
        fallback,
    );
    report.finish_while_target_pinned(|report| {
        finish_while_target_pinned(strict_target, || {
            attach_backend_evidence(
                finish_compile_report_in_staging(report, staging, true),
                evidence,
            )
        })
    })
}

fn finish_while_target_pinned<T, R>(target: Option<T>, finish: impl FnOnce() -> R) -> R {
    let response = finish();
    drop(target);
    response
}

fn attach_backend_evidence(mut response: Value, evidence: Value) -> Value {
    if let Some(fields) = response.as_object_mut() {
        fields.insert("compiler_backend".to_owned(), evidence);
    }
    response
}

fn compile_report_v1_payload(payload: CompileWirePayload) -> Value {
    compile_report_v1_payload_with_attempt(payload).0
}

fn compile_report_v1_payload_with_attempt(payload: CompileWirePayload) -> (Value, bool) {
    let mut game_attempted = false;
    let response = compile_report_v1_payload_recording_attempt(payload, &mut game_attempted);
    (response, game_attempted)
}

fn compile_report_v1_payload_recording_attempt(
    payload: CompileWirePayload,
    game_attempted: &mut bool,
) -> Value {
    let game_dir = PathBuf::from(&payload.game_dir);
    // This guard remains held across authoritative pristine selection and the complete compiler
    // transaction, so deploy/undeploy cannot change the selected bytes before live compiler use.
    let guard = match acquire_guard(&game_dir) {
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
        source_override: None,
        work_dir: staging.path().to_path_buf(),
        allow_new_symbols: payload.allow_new_symbols,
        base_override: Some(base_override),
        binds_override: None,
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
    *game_attempted = report.game_attempted();
    finish_compile_report_in_staging(report, staging, false)
}

fn finish_compile_report_in_staging(
    report: CompileModuleReport,
    staging: OwnedCompileStaging,
    allow_standalone_install_untouched: bool,
) -> Value {
    let standalone_selected = allow_standalone_install_untouched
        && report.backend_name() == Some(CompilerBackendNameV1::Standalone);
    let diagnostics_rejection = if standalone_selected {
        backend_diagnostics_rejection(report.backend_diagnostics())
    } else {
        diagnostics_rejection(report.diagnostics())
    };
    let output_rejection = match &report.outcome {
        CompileModuleReportOutcome::Compiled(output) => {
            anchor_owned_compiled_mini(&staging, output).err()
        }
        CompileModuleReportOutcome::Failed(_) => None,
    };
    let retain_staging = matches!(&report.outcome, CompileModuleReportOutcome::Compiled(_))
        && compiled_output_is_usable(
            report.install_restore_disposition(),
            diagnostics_rejection,
            output_rejection.is_none(),
            standalone_selected,
        );
    if retain_staging {
        // The response's mini_path remains usable after this call. Failed/recovery-required
        // attempts never cause native recursive deletion through caller-controlled paths.
        let _retained = staging.retain();
        debug_assert!(output_rejection.is_none());
    }
    report_response_with_policy(report, output_rejection, standalone_selected)
}

pub(super) fn install_state_v1_raw(input: &str) -> Value {
    let payload = match parse_install_state_request(input) {
        Ok(payload) => payload,
        Err(message) => return err("SCRIPT_COMPILE_INSTALL_STATE_BAD_REQUEST", message),
    };
    let game_dir = Path::new(&payload.game_dir);
    let probe = probe_install_state(game_dir);
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
    validate_compile_payload_fields(
        &request.payload.game_dir,
        &request.payload.as_path,
        &request.payload.work_dir,
        &request.payload.module_name,
        &request.payload.rel_path,
        &request.payload.op,
    )?;
    Ok(request.payload)
}

fn parse_request_v2(input: &str) -> Result<CompileWirePayloadV2, &'static str> {
    if input.len() > MAX_WIRE_BYTES {
        return Err("compile-report V2 request exceeds its bounded wire limit");
    }
    let request: ExactWireRequestV2 = serde_json::from_str(input)
        .map_err(|_| "compile-report V2 request has an invalid schema")?;
    if request.command != COMMAND_V2 {
        return Err("compile-report V2 request command does not match this route");
    }
    validate_compile_payload_fields(
        &request.payload.game_dir,
        &request.payload.as_path,
        &request.payload.work_dir,
        &request.payload.module_name,
        &request.payload.rel_path,
        &request.payload.op,
    )?;
    Ok(request.payload)
}

fn validate_compile_payload_fields(
    game_dir: &str,
    as_path: &str,
    work_dir: &str,
    module_name: &str,
    rel_path: &str,
    op: &str,
) -> Result<(), &'static str> {
    validate_path(game_dir)?;
    validate_path(as_path)?;
    validate_path(work_dir)?;
    if module_name.is_empty()
        || module_name.len() > MAX_MODULE_NAME_BYTES
        || module_name.contains('\0')
    {
        return Err("module_name is empty or exceeds its bounded length");
    }
    if rel_path.is_empty() || rel_path.len() > MAX_REL_PATH_BYTES || rel_path.contains('\0') {
        return Err("rel_path is empty or exceeds its bounded length");
    }
    if !matches!(op, "add" | "edit") {
        return Err("op must be exactly 'add' or 'edit'");
    }
    Ok(())
}

/// The compile guard, and the "is the game running?" probe behind it.
///
/// Under `cfg(test)` both answer from a stated value rather than from the real process list. These
/// tests build a throwaway install in a temp directory and assert on structured failure codes —
/// `COMPILE_BASE_UNAVAILABLE`, `COMPILE_BASE_RECOVERY_REQUIRED` — that they can only reach if the
/// guard lets them past. The production probe made every one of them return
/// `COMPILE_GAME_PROCESS_RUNNING` instead whenever Gothic happened to be open on the developer's
/// machine, which is exactly when someone is working on modding tools. Production is untouched: the
/// real probe is what ships.
#[cfg(not(test))]
fn acquire_guard(game_dir: &Path) -> Result<gore_as::compile::InstallMutationGuard, String> {
    acquire_compile_install_mutation(game_dir)
}

#[cfg(test)]
fn acquire_guard(game_dir: &Path) -> Result<gore_as::compile::InstallMutationGuard, String> {
    gore_as::compile::acquire_compile_install_mutation_with_stated_game_process(game_dir, || {
        Ok(false)
    })
}

#[cfg(not(test))]
fn probe_install_state(game_dir: &Path) -> gore_as::compile::InstallCompileStateProbe {
    probe_install_compile_state(game_dir)
}

#[cfg(test)]
fn probe_install_state(game_dir: &Path) -> gore_as::compile::InstallCompileStateProbe {
    gore_as::compile::probe_install_compile_state_with_stated_game_process(game_dir, || Ok(false))
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

pub(super) fn install_guard_failure(game_dir: &Path, message: String) -> Value {
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

pub(super) fn report_response(
    report: CompileModuleReport,
    output_rejection: Option<String>,
) -> Value {
    report_response_with_policy(report, output_rejection, false)
}

pub(super) fn report_response_with_policy(
    report: CompileModuleReport,
    output_rejection: Option<String>,
    standalone_install_untouched: bool,
) -> Value {
    let restore = report.install_restore_disposition();
    let recovery_required = report.recovery_required();
    let diagnostics_rejection = if standalone_install_untouched {
        backend_diagnostics_rejection(report.backend_diagnostics())
    } else {
        diagnostics_rejection(report.diagnostics())
    };
    let diagnostics = if standalone_install_untouched {
        Some(backend_diagnostics_json(report.backend_diagnostics()))
    } else {
        report
            .diagnostics()
            .map(|report| diagnostics_json(report.disposition(), report.diagnostics()))
    };
    let install_restore = install_restore_label(restore);
    match report.outcome {
        CompileModuleReportOutcome::Compiled(output) => compiled_response(
            output,
            restore,
            diagnostics_rejection,
            output_rejection,
            diagnostics,
            install_restore,
            recovery_required,
            standalone_install_untouched,
        ),
        CompileModuleReportOutcome::Failed(error) => {
            let (code, message) = compile_error_parts(error);
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
    standalone_install_untouched: bool,
) -> Value {
    let restore_is_valid = restore == InstallRestoreDisposition::RestoredExact
        || (standalone_install_untouched && restore == InstallRestoreDisposition::NotStarted);
    if !restore_is_valid {
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
    standalone_install_untouched: bool,
) -> bool {
    (restore == InstallRestoreDisposition::RestoredExact
        || (standalone_install_untouched && restore == InstallRestoreDisposition::NotStarted))
        && diagnostics_rejection.is_none()
        && output_is_anchored
}

fn backend_diagnostics_rejection(
    diagnostics: &[gore_as::compiler_backend::CompilerBackendDiagnosticV1],
) -> Option<(&'static str, &'static str)> {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == CompilerBackendDiagnosticSeverityV1::Error)
        .then_some((
            "COMPILE_DIAGNOSTICS_REPORTED_ERROR",
            "the standalone compiler reported an error; compiled output was discarded",
        ))
}

fn backend_diagnostics_json(
    diagnostics: &[gore_as::compiler_backend::CompilerBackendDiagnosticV1],
) -> Value {
    let messages = diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity() {
                CompilerBackendDiagnosticSeverityV1::Info => "note",
                CompilerBackendDiagnosticSeverityV1::Warning => "warning",
                CompilerBackendDiagnosticSeverityV1::Error => "error",
            };
            json!({
                "file": diagnostic.source_path().unwrap_or(""),
                "line": diagnostic.line().unwrap_or(0),
                "column": diagnostic.column().unwrap_or(0),
                "severity": severity,
                "message": format!("{}: {}", diagnostic.code(), diagnostic.message()),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "capture": "captured",
        "messages": messages,
        "omitted": 0,
    })
}

pub(super) fn diagnostics_rejection(
    report: Option<&gore_as::diagnostics::CompilerDiagnosticsReport>,
) -> Option<(&'static str, &'static str)> {
    match report {
        Some(report) => {
            compiled_diagnostics_rejection_parts(Some(report.disposition()), report.diagnostics())
        }
        None => compiled_diagnostics_rejection_parts(None, &[]),
    }
}

pub(super) fn diagnostics_report_json(
    report: &gore_as::diagnostics::CompilerDiagnosticsReport,
) -> Value {
    diagnostics_json(report.disposition(), report.diagnostics())
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

pub(super) fn compile_error_parts(error: CompileError) -> (&'static str, String) {
    let code = match &error {
        CompileError::Io(_) | CompileError::ArtifactIo { .. } => "COMPILE_IO",
        CompileError::Regen(_) => "COMPILER_REGEN_FAILED",
        CompileError::NoRegen(_) => "COMPILER_OUTPUT_MISSING",
        CompileError::Other(_) => "COMPILE_FAILED",
    };
    (code, error.to_string())
}

pub(super) fn install_restore_label(disposition: InstallRestoreDisposition) -> &'static str {
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

    #[test]
    fn response_is_finished_before_the_qualified_target_pin_is_dropped() {
        struct DropWitness(std::sync::Arc<std::sync::atomic::AtomicBool>);

        impl Drop for DropWitness {
            fn drop(&mut self) {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }

        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let response = finish_while_target_pinned(Some(DropWitness(dropped.clone())), || {
            assert!(!dropped.load(std::sync::atomic::Ordering::SeqCst));
            json!({"qualified_evidence": true})
        });

        assert_eq!(response["qualified_evidence"], true);
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    }

    fn request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn request_v2(payload: Value) -> String {
        json!({"command": COMMAND_V2, "payload": payload}).to_string()
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
    fn v2_requires_explicit_game_and_v1_wire_stays_unchanged() {
        let root = tempfile::tempdir().unwrap();
        let v1 = compile_report_v1_raw(&request(valid_payload(root.path())));
        assert!(v1.get("compiler_backend").is_none());

        let missing = compile_report_v2_raw(&request_v2(valid_payload(root.path())));
        assert_eq!(
            missing["error"]["code"],
            "SCRIPT_COMPILE_REPORT_V2_BAD_REQUEST"
        );
        let mut payload = valid_payload(root.path());
        payload["compiler_backend"] = json!("game");
        let v2 = compile_report_v2_raw(&request_v2(payload));
        assert_eq!(v2["compiler_backend"]["requested_mode"], "game");
        assert!(v2["compiler_backend"]["result_backend"].is_null());
        assert_eq!(v2["compiler_backend"]["standalone_attempted"], false);
        assert_eq!(v2["compiler_backend"]["game_attempted"], false);
        assert!(v2["compiler_backend"]["qualified_package"].is_null());
        assert!(v2["compiler_backend"]["fallback_reason"].is_null());
    }

    #[test]
    fn v2_strict_standalone_fails_before_install_ownership_when_bundle_is_absent() {
        let root = tempfile::tempdir().unwrap();
        let mut payload = valid_payload(root.path());
        payload["compiler_backend"] = json!("standalone");

        let response = crate::dispatch(&request_v2(payload));

        assert_eq!(response["outcome"], "failed");
        assert_eq!(
            response["compile_error"]["code"],
            "COMPILE_STANDALONE_BUNDLE_ABSENT"
        );
        assert_eq!(response["compiler_backend"]["requested_mode"], "standalone");
        assert!(response["compiler_backend"]["result_backend"].is_null());
        assert_eq!(response["compiler_backend"]["standalone_attempted"], false);
        assert_eq!(response["compiler_backend"]["game_attempted"], false);
        assert!(response["compiler_backend"]["qualified_package"].is_null());
        assert!(response["compiler_backend"]["fallback_reason"].is_null());
        assert!(!root.path().join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn v2_explicit_fallback_reports_bundle_absence_and_rejects_wire_paths() {
        let root = tempfile::tempdir().unwrap();
        let mut payload = valid_payload(root.path());
        payload["compiler_backend"] = json!("standalone_then_game");
        let response = compile_report_v2_raw(&request_v2(payload));
        assert!(response["compiler_backend"]["result_backend"].is_null());
        assert_eq!(response["compiler_backend"]["standalone_attempted"], false);
        assert_eq!(response["compiler_backend"]["game_attempted"], false);
        assert_eq!(
            response["compiler_backend"]["fallback_reason"]["failure_kind"],
            "unavailable"
        );
        assert_eq!(
            response["compiler_backend"]["fallback_reason"]["detail"],
            BUNDLE_ABSENT_DETAIL
        );
        assert!(!root.path().join(".gore-install-mutation.lock").exists());

        let mut untrusted = valid_payload(root.path());
        untrusted["compiler_backend"] = json!("standalone");
        untrusted["standalone_sidecar"] = json!("C:/caller/sidecar.exe");
        let rejected = compile_report_v2_raw(&request_v2(untrusted));
        assert_eq!(
            rejected["error"]["code"],
            "SCRIPT_COMPILE_REPORT_V2_BAD_REQUEST"
        );

        let mut wrong_enum = valid_payload(root.path());
        wrong_enum["compiler_backend"] = json!("standalone-then-game");
        assert_eq!(
            compile_report_v2_raw(&request_v2(wrong_enum))["error"]["code"],
            "SCRIPT_COMPILE_REPORT_V2_BAD_REQUEST"
        );
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
    fn evidence_only_discard_removes_the_exact_owned_mini() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let game = root.path().join("game");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&game).unwrap();
        let staging = OwnedCompileStaging::create(&workspace, &game).unwrap();
        let mini_path = staging.path().join("module.cache");
        fs::write(&mini_path, b"not-a-reusable-managed-artifact").unwrap();
        let mut output = gore_as::compile::CompileOutput::bind_existing(
            mini_path.clone(),
            "GoreMods.ManagedCheck".to_owned(),
        )
        .unwrap();

        discard_owned_compiled_mini(&staging, &mut output).unwrap();
        assert!(!mini_path.exists());
        assert!(staging.path().join(OWNED_COMPILE_MARKER).is_file());
    }

    #[test]
    fn evidence_only_discard_neutralizes_before_rejecting_a_corrupt_marker() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let game = root.path().join("game");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&game).unwrap();
        let mut staging = OwnedCompileStaging::create(&workspace, &game).unwrap();
        let mini_path = staging.path().join("module.cache");
        fs::write(&mini_path, b"must-be-destroyed-before-marker-rejection").unwrap();
        let mut output = gore_as::compile::CompileOutput::bind_existing(
            mini_path.clone(),
            "GoreMods.ManagedCheck".to_owned(),
        )
        .unwrap();
        let exact_output_probe = fs::File::open(&mini_path).unwrap();

        // Production retains a read-only marker anchor. Release only that last test-visible
        // anchor so this regression can model a marker damaged by an external actor.
        let marker_anchor = staging
            ._anchors
            .pop()
            .expect("owned staging must retain its marker as the final anchor");
        assert!(marker_anchor.metadata().unwrap().is_file());
        drop(marker_anchor);
        fs::write(staging.path().join(OWNED_COMPILE_MARKER), b"corrupt-marker").unwrap();

        let error = discard_owned_compiled_mini(&staging, &mut output)
            .expect_err("a corrupt ownership marker must reject pathname cleanup");
        assert!(
            error.contains("compile-staging ownership marker content changed"),
            "got: {error}"
        );
        assert!(mini_path.is_file());
        assert_eq!(exact_output_probe.metadata().unwrap().len(), 0);
        assert!(fs::read(&mini_path).unwrap().is_empty());

        let mut detached = gore_as::compile::CompileOutput::detached(
            mini_path,
            "GoreMods.ManagedCheck".to_owned(),
        );
        let combined = discard_owned_compiled_mini(&staging, &mut detached)
            .expect_err("handle and marker failures must both be reported");
        assert!(
            combined.contains("compiler output did not retain its creation handle"),
            "got: {combined}"
        );
        assert!(
            combined.contains("compile-staging ownership verification also failed")
                && combined.contains("compile-staging ownership marker content changed"),
            "got: {combined}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn evidence_only_discard_securely_empties_a_delete_blocked_mini() {
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let game = root.path().join("game");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&game).unwrap();
        let staging = OwnedCompileStaging::create(&workspace, &game).unwrap();
        let mini_path = staging.path().join("module.cache");
        fs::write(&mini_path, b"must-be-destroyed").unwrap();
        let blocker = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(&mini_path)
            .unwrap();
        let mut output = gore_as::compile::CompileOutput::bind_existing(
            mini_path.clone(),
            "GoreMods.ManagedCheck".to_owned(),
        )
        .unwrap();

        discard_owned_compiled_mini(&staging, &mut output).unwrap();
        assert_eq!(fs::metadata(&mini_path).unwrap().len(), 0);
        assert!(fs::read(&mini_path).unwrap().is_empty());
        drop(blocker);
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
        let output = gore_as::compile::CompileOutput::bind_existing(
            exact.clone(),
            "GoreMods.Probe".to_owned(),
        )
        .unwrap();
        anchor_owned_compiled_mini(&staging, &output).unwrap();

        let wrong = gore_as::compile::CompileOutput::detached(
            staging.path().join("elsewhere.cache"),
            "GoreMods.Probe".to_owned(),
        );
        assert!(anchor_owned_compiled_mini(&staging, &wrong).is_err());
        drop(output);
        fs::remove_file(&exact).unwrap();
        fs::create_dir(&exact).unwrap();
        let directory_output =
            gore_as::compile::CompileOutput::detached(exact.clone(), "GoreMods.Probe".to_owned());
        assert!(anchor_owned_compiled_mini(&staging, &directory_output).is_err());

        let response = compiled_response(
            directory_output,
            InstallRestoreDisposition::RestoredExact,
            None,
            Some("compiled module.cache failed its owned-file check".to_owned()),
            None,
            "restored_exact",
            false,
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
                false,
            ));
            let response = compiled_response(
                gore_as::compile::CompileOutput::detached(
                    PathBuf::from("must-not-escape.cache"),
                    "GoreMods.Probe".to_owned(),
                ),
                InstallRestoreDisposition::RestoredExact,
                rejection,
                None,
                Some(diagnostics_json(disposition, &diagnostics)),
                "restored_exact",
                false,
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

    #[test]
    fn qualified_standalone_output_requires_install_untouched_policy() {
        assert!(compiled_output_is_usable(
            InstallRestoreDisposition::NotStarted,
            None,
            true,
            true,
        ));
        assert!(!compiled_output_is_usable(
            InstallRestoreDisposition::NotStarted,
            None,
            true,
            false,
        ));
        let response = compiled_response(
            gore_as::compile::CompileOutput::detached(
                PathBuf::from("owned-standalone.cache"),
                "GoreMods.Probe".to_owned(),
            ),
            InstallRestoreDisposition::NotStarted,
            None,
            None,
            Some(json!({"capture":"captured","messages":[],"omitted":0})),
            "not_started",
            false,
            true,
        );
        assert_eq!(response["outcome"], "compiled");
        assert_eq!(response["install_restore"], "not_started");
    }
}
