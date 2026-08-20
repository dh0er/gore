//! Compile a staged `.as` into a 1-module mini-cache by driving the game's precompiled-data
//! generation, then extracting (add) / extract-remapping (edit) the target module.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::cache::{emit_all, model, refs::RefResolver, remap, splice};

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("io: {0}")]
    Io(String),
    #[error("regen: {0}")]
    Regen(String),
    #[error("the game did not produce a usable regen cache at {0}")]
    NoRegen(String),
    #[error("{0}")]
    Other(String),
    #[error("io: {message}")]
    ArtifactIo {
        message: String,
        artifact: Option<FailedCompiledArtifact>,
    },
}

pub struct CompileOpts {
    pub game_dir: PathBuf,
    pub op: String, // "add" | "edit"
    pub module_name: String,
    pub rel_path: String,
    pub as_path: PathBuf,
    /// Exact authored source bytes. When `Some`, compilation consumes only this buffer and never
    /// opens `as_path`; managed callers use it to keep their sealed source snapshot independent
    /// from caller-controlled workspace path races. Standalone callers keep `None`.
    pub source_override: Option<Vec<u8>>,
    pub work_dir: PathBuf,
    /// Explicitly allow the edited/generated module to introduce symbols absent from the base.
    /// Default callers must pass `false`; the strict historical remap remains the safe default.
    pub allow_new_symbols: bool,
    /// Pristine base cache to emit/remap against. When `Some`, these bytes are the base (skip the
    /// disk read) — the FFI passes gore-mod's drift-aware `pristine_script_cache` so the compile
    /// base matches the bytes deploy will splice against. When `None`, fall back to the on-disk
    /// `*.gore-bak`-or-live read (standalone/CLI/offline). NOTE: `game_run_regen` still uses the
    /// LIVE cache for its own backup/restore — only this emit/remap base is overridden.
    pub base_override: Option<Vec<u8>>,
    /// Exact `Binds.Cache` bytes used to prepare native method/function arities. When `Some`,
    /// parsing uses only this buffer and ignores both the live install and `GORE_AS_BINDS`.
    /// Managed callers pass their already sealed/revalidated game-input snapshot; standalone
    /// callers keep `None` for the historical environment-or-neighbor lookup.
    pub binds_override: Option<Vec<u8>>,
}

/// Maximum number of sealed add-only modules accepted by one project compiler check.
pub const MAX_PROJECT_COMPILER_CHECK_MODULES: usize = 1_024;
/// Maximum aggregate authored source bytes accepted by one project compiler check.
pub const MAX_PROJECT_COMPILER_CHECK_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_PROJECT_COMPILER_CHECK_IDENTITY_BYTES: usize = 4 * 1024;
const MAX_PROJECT_COMPILER_CHECK_BASE_BYTES: usize = 512 * 1024 * 1024;
const MAX_PROJECT_COMPILER_CHECK_BINDS_BYTES: usize = 128 * 1024 * 1024;
const MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES: u64 = 512 * 1024 * 1024;

/// One source module sealed by the managed project/store layer.
///
/// Project checks are intentionally add-only. Edit/remap semantics belong to build planning, not
/// to evidence that all project-owned sources are accepted together by the game compiler.
#[derive(Debug, Clone)]
pub struct ProjectCompileOverlay {
    pub module_name: String,
    pub rel_path: String,
    pub source: Vec<u8>,
}

/// Inputs for one evidence-only, project-wide compiler check.
///
/// Shipping and Binds bytes are mandatory sealed snapshots. This managed API has no environment,
/// backup-file, or live-install fallback for either authoritative input.
#[derive(Debug)]
pub struct ProjectCompilerCheckOpts {
    pub game_dir: PathBuf,
    pub work_dir: PathBuf,
    pub overlays: Vec<ProjectCompileOverlay>,
    pub base_cache: Vec<u8>,
    pub binds_cache: Vec<u8>,
}

#[derive(Debug)]
pub struct CompiledArtifact {
    path: PathBuf,
    file: std::fs::File,
}

#[derive(Debug)]
pub struct FailedCompiledArtifact {
    artifact: CompiledArtifact,
}

impl Drop for FailedCompiledArtifact {
    fn drop(&mut self) {
        // Every caller gets a final best-effort retry, including legacy CLI/report paths that do
        // not explicitly consume the managed cleanup capability.
        let _ = self.artifact.neutralize();
    }
}

impl CompiledArtifact {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Durably destroy the bytes of the exact output handle retained from creation. This stays
    /// identity-safe even if an untrusted observer renamed the pathname after compilation.
    pub fn neutralize(&self) -> Result<(), String> {
        self.file
            .set_len(0)
            .map_err(|error| format!("truncating retained compiler artifact: {error}"))?;
        self.file
            .sync_all()
            .map_err(|error| format!("syncing retained compiler artifact: {error}"))?;
        if self
            .file
            .metadata()
            .map_err(|error| format!("rechecking retained compiler artifact: {error}"))?
            .len()
            != 0
        {
            return Err("retained compiler artifact still contains usable bytes".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CompileOutput {
    pub mini_path: PathBuf,
    pub module_name: String,
    artifact: Option<CompiledArtifact>,
}

impl CompileOutput {
    fn retained(mini_path: PathBuf, module_name: String, artifact: CompiledArtifact) -> Self {
        Self {
            mini_path,
            module_name,
            artifact: Some(artifact),
        }
    }

    /// Bind an existing regular file for tests/adapters that need an identity-retaining output.
    /// Production compilation retains its original create-new handle instead.
    pub fn bind_existing(mini_path: PathBuf, module_name: String) -> Result<Self, String> {
        let file = open_compiled_artifact_existing(&mini_path)?;
        Ok(Self::retained(
            mini_path.clone(),
            module_name,
            CompiledArtifact {
                path: mini_path,
                file,
            },
        ))
    }

    /// Metadata-only value for response-policy tests. It cannot satisfy evidence-only disposal.
    pub fn detached(mini_path: PathBuf, module_name: String) -> Self {
        Self {
            mini_path,
            module_name,
            artifact: None,
        }
    }

    pub fn neutralize_retained_artifact(&mut self) -> Result<(), String> {
        let artifact = self
            .artifact
            .take()
            .ok_or_else(|| "compiler output did not retain its creation handle".to_owned())?;
        if artifact.path() != self.mini_path {
            self.artifact = Some(artifact);
            return Err("retained compiler output identity does not match mini_path".to_owned());
        }
        match artifact.neutralize() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.artifact = Some(artifact);
                Err(error)
            }
        }
    }

    pub fn validate_retained_artifact(&self) -> Result<(), String> {
        let artifact = self
            .artifact
            .as_ref()
            .ok_or_else(|| "compiler output did not retain its creation handle".to_owned())?;
        if artifact.path() != self.mini_path {
            return Err("retained compiler output identity does not match mini_path".to_owned());
        }
        let metadata = artifact
            .file
            .metadata()
            .map_err(|error| format!("inspecting retained compiler output: {error}"))?;
        if !metadata.is_file() {
            return Err("retained compiler output is not a regular file".to_owned());
        }
        Ok(())
    }
}

impl CompileError {
    /// Neutralize a partial final mini-cache write through the exact creation handle, when the
    /// final writer was the source of this failure. Other failures never created that artifact.
    pub fn neutralize_failed_artifact(&mut self) -> Option<Result<(), String>> {
        match self {
            Self::ArtifactIo { artifact, .. } => {
                let retained = match artifact.take() {
                    Some(retained) => retained,
                    None => {
                        return Some(Err(
                            "failed compiler artifact handle was already consumed".to_owned()
                        ))
                    }
                };
                match retained.artifact.neutralize() {
                    Ok(()) => Some(Ok(())),
                    Err(error) => {
                        *artifact = Some(retained);
                        Some(Err(error))
                    }
                }
            }
            _ => None,
        }
    }
}

/// What happened to the live game installation around a compiler attempt.
///
/// This is deliberately separate from compiler success: a syntax error after the generator exited
/// can still have restored every live path exactly, while an otherwise useful compiler report may
/// require manual recovery when process termination or cleanup could not be proven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallRestoreDisposition {
    /// The transactional generator runner was never entered (for example, module preflight failed).
    NotStarted,
    /// The generator is confirmed absent and every live path was restored to its pre-call state.
    RestoredExact,
    /// Generator exit could not be confirmed, so isolation and disk recovery artifacts were kept.
    RecoveryRequiredProcessExitUnconfirmed,
    /// Restore, finalization, or shared-ownership release failed while a persistent recovery
    /// blocker was still retained.
    RecoveryRequiredRestoreFailed,
}

/// Result of one compile-module attempt. A failure remains an ordinary [`CompileError`]; the
/// surrounding report separately preserves whether enhanced diagnostics were captured, fell back,
/// or were unavailable after the original process had already completed.
#[derive(Debug)]
pub enum CompileModuleReportOutcome {
    Compiled(CompileOutput),
    Failed(CompileError),
}

/// Structured companion to [`compile_module`].
///
/// `diagnostics` is `None` only when the operation failed before a game compiler process produced
/// a report (for example during source/base preflight or install transaction setup). Once the
/// compiler path starts, success and failure both retain a bounded report without parsing the
/// human-readable [`CompileError`] string.
#[derive(Debug)]
pub struct CompileModuleReport {
    pub outcome: CompileModuleReportOutcome,
    diagnostics: Option<crate::diagnostics::CompilerDiagnosticsReport>,
    install_restore: InstallRestoreDisposition,
}

impl CompileModuleReport {
    pub fn diagnostics(&self) -> Option<&crate::diagnostics::CompilerDiagnosticsReport> {
        self.diagnostics.as_ref()
    }

    pub fn install_restore_disposition(&self) -> InstallRestoreDisposition {
        self.install_restore
    }

    pub fn into_parts(
        self,
    ) -> (
        CompileModuleReportOutcome,
        Option<crate::diagnostics::CompilerDiagnosticsReport>,
    ) {
        (self.outcome, self.diagnostics)
    }
}

/// Result of a project-wide compiler check. Success is evidence only: it never contains a cache,
/// mini, build plan, deployment capability, or filesystem path.
#[derive(Debug)]
pub enum ProjectCompilerCheckOutcome {
    Checked,
    Failed(CompileError),
}

/// What happened to the private regenerated-cache copy used for manifest verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCompilerOutputDisposition {
    /// Preflight failed before the generator returned any private cache copy.
    NotCreated,
    /// Any private cache copy was neutralized and removed before the report was returned.
    Discarded,
    /// Disposal could not be proven. No path/capability is exposed, and callers must fail closed.
    RecoveryRetained,
}

/// Whether the caller's exact-current closing audit ran while the same install guard was held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCompilerClosingAuditDisposition {
    NotRun,
    Passed,
    Failed,
}

/// Evidence-only report for a shared project compiler attempt.
#[derive(Debug)]
pub struct ProjectCompilerCheckReport {
    pub outcome: ProjectCompilerCheckOutcome,
    diagnostics: Option<crate::diagnostics::CompilerDiagnosticsReport>,
    install_restore: InstallRestoreDisposition,
    output_disposition: ProjectCompilerOutputDisposition,
    closing_audit: ProjectCompilerClosingAuditDisposition,
    runner_invocations: u8,
}

impl ProjectCompilerCheckReport {
    pub fn diagnostics(&self) -> Option<&crate::diagnostics::CompilerDiagnosticsReport> {
        self.diagnostics.as_ref()
    }

    pub fn install_restore_disposition(&self) -> InstallRestoreDisposition {
        self.install_restore
    }

    pub fn output_disposition(&self) -> ProjectCompilerOutputDisposition {
        self.output_disposition
    }

    pub fn closing_audit_disposition(&self) -> ProjectCompilerClosingAuditDisposition {
        self.closing_audit
    }

    /// Whether the live install or the private output could not be proven clean.
    pub fn recovery_required(&self) -> bool {
        matches!(
            self.install_restore,
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
                | InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        ) || self.output_disposition == ProjectCompilerOutputDisposition::RecoveryRetained
    }

    /// Number of calls into the injected/production regen runner. This is zero for all source,
    /// cache, layout, and tree preflight failures and exactly one after the shared tree is ready.
    pub fn runner_invocations(&self) -> u8 {
        self.runner_invocations
    }

    pub fn into_parts(
        self,
    ) -> (
        ProjectCompilerCheckOutcome,
        Option<crate::diagnostics::CompilerDiagnosticsReport>,
    ) {
        (self.outcome, self.diagnostics)
    }
}

/// Compile one module through the transactional game compiler while retaining bounded structured
/// diagnostics and the exact capture/fallback disposition.
///
/// The lower-level [`compile_module`] API remains the injectable single-module primitive. This
/// higher-level production entry point uses the same default diagnostics options and never derives
/// structured messages by reparsing an error string.
pub fn compile_module_with_diagnostics_report(
    opts: &CompileOpts,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> CompileModuleReport {
    compile_module_report_with(opts, |game_dir, source_tree| {
        game_run_regen_with_extended_diagnostics_report(game_dir, source_tree, diagnostics)
    })
}

/// Compile with a caller-held guard acquired by [`acquire_compile_install_mutation`].
///
/// This is the authoritative-read variant for FFI/orchestration callers: resolve the pristine
/// cache while holding `guard`, put those exact bytes in `opts.base_override`, then transfer the
/// same guard here. The runner consumes it without reacquiring, so deploy/undeploy cannot interleave
/// between pristine selection and live compiler use.
pub fn compile_module_with_diagnostics_report_with_guard(
    opts: &CompileOpts,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
    guard: InstallMutationGuard,
) -> CompileModuleReport {
    let guard = std::cell::RefCell::new(Some(guard));
    let generated = std::cell::RefCell::new(None);
    let mut result = compile_module(opts, |game_dir, source_tree| {
        let guard = guard
            .borrow_mut()
            .take()
            .ok_or_else(|| "pre-held compiler guard was consumed more than once".to_owned())?;
        let report = game_run_regen_with_extended_diagnostics_report_with_guard(
            game_dir,
            source_tree,
            diagnostics,
            guard,
        )?;
        *generated.borrow_mut() = Some((report.diagnostics, report.install_restore));
        report.result
    });
    let generated = generated.into_inner();
    let mut install_restore = generated
        .as_ref()
        .map(|(_, install_restore)| *install_restore)
        .unwrap_or(InstallRestoreDisposition::NotStarted);

    // Source/base/overlay preflight can fail before the runner consumes the guard. Release it
    // explicitly so Drop cannot hide a coordination failure behind an ordinary NotStarted report.
    if let Some(mut unused_guard) = guard.into_inner() {
        if let Err(release) = unused_guard.release() {
            unused_guard.preserve_for_manual_recovery();
            let primary = result
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "compiler runner was not entered".to_owned());
            result = Err(CompileError::Other(format!(
                "{primary}; additionally failed to release the pre-held install-mutation guard: \
                 {release}"
            )));
            // The release operation itself failed; a second pathname inspection cannot safely
            // downgrade that uncertainty (NotFound/permission/race are not proof of cleanup).
            install_restore = InstallRestoreDisposition::RecoveryRequiredRestoreFailed;
        }
    }

    CompileModuleReport {
        outcome: match result {
            Ok(output) => CompileModuleReportOutcome::Compiled(output),
            Err(error) => CompileModuleReportOutcome::Failed(error),
        },
        diagnostics: generated.and_then(|(diagnostics, _)| diagnostics),
        install_restore,
    }
}

fn compile_module_report_with<R>(opts: &CompileOpts, run_regen: R) -> CompileModuleReport
where
    R: FnOnce(&Path, &Path) -> Result<GameRunRegenExtendedReport, String>,
{
    let report = std::cell::RefCell::new(None);
    let result = compile_module(opts, |game_dir, source_tree| {
        let generated = run_regen(game_dir, source_tree)?;
        let mut slot = report.borrow_mut();
        if slot.is_some() {
            return Err("compile-module diagnostics runner was invoked more than once".to_owned());
        }
        let result = generated.result;
        *slot = Some((generated.diagnostics, generated.install_restore));
        result
    });
    let generated = report.into_inner();
    let install_restore = generated
        .as_ref()
        .map(|(_, install_restore)| *install_restore)
        .unwrap_or(InstallRestoreDisposition::NotStarted);
    CompileModuleReport {
        outcome: match result {
            Ok(output) => CompileModuleReportOutcome::Compiled(output),
            Err(error) => CompileModuleReportOutcome::Failed(error),
        },
        diagnostics: generated.and_then(|(diagnostics, _)| diagnostics),
        install_restore,
    }
}

#[derive(Debug)]
struct ProjectCompilerRunnerReport {
    result: Result<Vec<u8>, String>,
    diagnostics: Option<crate::diagnostics::CompilerDiagnosticsReport>,
    install_restore: InstallRestoreDisposition,
    output_disposition: ProjectCompilerOutputDisposition,
    closing_audit: ProjectCompilerClosingAuditDisposition,
}

/// Compile every sealed add-only project module in one shared Shipping source tree and return only
/// bounded evidence. The passed install guard covers the complete batch and is consumed exactly
/// once by the transactional game runner.
pub fn check_project_modules_with_diagnostics_report_with_guard<A>(
    opts: &ProjectCompilerCheckOpts,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
    guard: InstallMutationGuard,
    closing_audit: A,
) -> ProjectCompilerCheckReport
where
    A: FnOnce() -> Result<(), String>,
{
    check_project_modules_with_guard_and_runner(
        opts,
        guard,
        closing_audit,
        |game_dir, source_tree, guard, closing_audit| {
            run_project_compiler_with_guard(
                game_dir,
                source_tree,
                diagnostics,
                guard,
                closing_audit,
            )
        },
    )
}

fn check_project_modules_with_guard_and_runner<R, A>(
    opts: &ProjectCompilerCheckOpts,
    guard: InstallMutationGuard,
    closing_audit: A,
    run_regen: R,
) -> ProjectCompilerCheckReport
where
    R: FnOnce(&Path, &Path, InstallMutationGuard, A) -> ProjectCompilerRunnerReport,
    A: FnOnce() -> Result<(), String>,
{
    let guard = std::cell::RefCell::new(Some(guard));
    let closing_audit = std::cell::RefCell::new(Some(closing_audit));
    let generated = std::cell::RefCell::new(None);
    let runner_invocations = std::cell::Cell::new(0u8);
    let mut result = check_project_modules(opts, |game_dir, source_tree| {
        if runner_invocations.replace(1) != 0 {
            return Err("project compiler runner was invoked more than once".to_owned());
        }
        let guard = guard.borrow_mut().take().ok_or_else(|| {
            "pre-held project compiler guard was consumed more than once".to_owned()
        })?;
        let audit = closing_audit.borrow_mut().take().ok_or_else(|| {
            "project compiler closing audit capability was consumed more than once".to_owned()
        })?;
        let report = run_regen(game_dir, source_tree, guard, audit);
        let unsafe_recovery = matches!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
                | InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        ) || report.output_disposition
            == ProjectCompilerOutputDisposition::RecoveryRetained;
        let result = if unsafe_recovery {
            Err(report.result.err().unwrap_or_else(|| {
                "project compiler reported success while recovery remains required".to_owned()
            }))
        } else if report.result.is_ok()
            && report.closing_audit != ProjectCompilerClosingAuditDisposition::Passed
        {
            Err(format!(
                "project compiler reported success without a passed same-guard closing audit ({:?})",
                report.closing_audit
            ))
        } else if report.result.is_ok() {
            match project_diagnostics_acceptance_error(report.diagnostics.as_ref()) {
                Some(error) => Err(error),
                None => report.result,
            }
        } else {
            report.result
        };
        *generated.borrow_mut() = Some((
            report.diagnostics,
            report.install_restore,
            report.output_disposition,
            report.closing_audit,
        ));
        result
    });

    let mut generated = generated.into_inner();
    let mut install_restore = generated
        .as_ref()
        .map(|(_, install_restore, _, _)| *install_restore)
        .unwrap_or(InstallRestoreDisposition::NotStarted);
    let mut output_disposition = generated
        .as_ref()
        .map(|(_, _, output, _)| *output)
        .unwrap_or(ProjectCompilerOutputDisposition::NotCreated);
    let mut closing_audit_disposition = generated
        .as_ref()
        .map(|(_, _, _, closing_audit)| *closing_audit)
        .unwrap_or(ProjectCompilerClosingAuditDisposition::NotRun);

    // Every cache/source/layout/tree failure occurs before the runner consumes the guard. Release
    // it explicitly so an ownership-release failure dominates the ordinary preflight error and is
    // never hidden by Drop.
    if let Some(mut unused_guard) = guard.into_inner() {
        if let Some(audit) = closing_audit.into_inner() {
            match audit() {
                Ok(()) => {
                    closing_audit_disposition = ProjectCompilerClosingAuditDisposition::Passed;
                }
                Err(audit_error) => {
                    let primary = result
                        .err()
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| "project compiler runner was not entered".to_owned());
                    result = Err(CompileError::Other(format!(
                        "{primary}; closing project audit failed while the install guard was held: \
                         {audit_error}"
                    )));
                    closing_audit_disposition = ProjectCompilerClosingAuditDisposition::Failed;
                }
            }
        }
        if let Err(release) = unused_guard.release() {
            unused_guard.preserve_for_manual_recovery();
            let primary = result
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "project compiler runner was not entered".to_owned());
            result = Err(CompileError::Other(format!(
                "{primary}; additionally failed to release the pre-held project compiler guard: \
                 {release}"
            )));
            install_restore = InstallRestoreDisposition::RecoveryRequiredRestoreFailed;
            output_disposition = ProjectCompilerOutputDisposition::RecoveryRetained;
        }
    }

    if matches!(
        install_restore,
        InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
            | InstallRestoreDisposition::RecoveryRequiredRestoreFailed
    ) {
        output_disposition = ProjectCompilerOutputDisposition::RecoveryRetained;
    }

    ProjectCompilerCheckReport {
        outcome: match result {
            Ok(()) => ProjectCompilerCheckOutcome::Checked,
            Err(error) => ProjectCompilerCheckOutcome::Failed(error),
        },
        diagnostics: generated
            .as_mut()
            .and_then(|(diagnostics, _, _, _)| diagnostics.take()),
        install_restore,
        output_disposition,
        closing_audit: closing_audit_disposition,
        runner_invocations: runner_invocations.get(),
    }
}

fn project_diagnostics_acceptance_error(
    diagnostics: Option<&crate::diagnostics::CompilerDiagnosticsReport>,
) -> Option<String> {
    let Some(report) = diagnostics else {
        return Some("game compiler produced a cache without a diagnostics disposition".to_owned());
    };
    if !matches!(
        report.disposition(),
        crate::diagnostics::DiagnosticsCaptureDisposition::Captured
            | crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback
    ) {
        return Some(format!(
            "game compiler diagnostics disposition {:?} cannot authorize project evidence",
            report.disposition()
        ));
    }
    if report
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnostics::DiagnosticSeverity::Error)
    {
        return Some(
            "game compiler reported at least one error diagnostic; refusing generated cache"
                .to_owned(),
        );
    }
    None
}

fn check_project_modules<R>(
    opts: &ProjectCompilerCheckOpts,
    run_regen: R,
) -> Result<(), CompileError>
where
    R: FnOnce(&Path, &Path) -> Result<Vec<u8>, String>,
{
    validate_project_compiler_check_bounds(opts)?;
    validate_generated_cache(&opts.base_cache)
        .map_err(|error| CompileError::Other(format!("invalid sealed Shipping cache: {error}")))?;

    let mut refs = RefResolver::build(&opts.base_cache)
        .map_err(|error| CompileError::Other(format!("resolver: {error}")))?;
    let base_modules = model::parse_modules(&opts.base_cache)
        .map_err(|error| CompileError::Other(format!("parse: {error}")))?;
    let native_api = crate::cache::binds::NativeApi::from_bytes(&opts.binds_cache)
        .ok_or_else(|| CompileError::Other("sealed Binds.Cache is invalid".to_owned()))?;
    // The compile baseline stays defaults-free (the emit default), exactly the tree this path
    // was qualified against. Its job is to satisfy the compiler's dependency graph, not to be
    // read, and the extracted module's own `__InitDefaults` is carried from the base cache
    // byte-for-byte — regenerating 30k of them here would change the regen cache for no gain.
    let prepared = emit_all::PreparedEmit::new(&base_modules, &mut refs, Some(native_api))
        .map_err(|error| CompileError::Other(format!("preparing base modules: {error}")))?;

    let mut sources = Vec::with_capacity(opts.overlays.len());
    for overlay in &opts.overlays {
        sources.push(std::str::from_utf8(&overlay.source).map_err(|error| {
            CompileError::Other(format!(
                "project module {:?} source is not valid UTF-8: {error}",
                overlay.module_name
            ))
        })?);
    }
    let requested = opts
        .overlays
        .iter()
        .zip(&sources)
        .map(|(overlay, source)| emit_all::CompileAddOverlay {
            module_name: &overlay.module_name,
            relative_path: &overlay.rel_path,
            source,
        })
        .collect::<Vec<_>>();
    let overlays = prepared
        .prepare_compile_add_overlays(&requested)
        .map_err(|error| CompileError::Other(format!("preparing project overlays: {error}")))?;

    let base_manifest = emit_all::validated_module_identities(&base_modules).map_err(|error| {
        CompileError::Other(format!("validating base module manifest: {error}"))
    })?;

    // Only after the complete batch has passed every memory-only preflight may the private tree be
    // reset and populated.
    let tree = reset_compile_tree(&opts.work_dir).map_err(CompileError::Other)?;
    prepared
        .emit_tree(&tree)
        .map_err(|error| CompileError::Other(format!("emit tree: {error}")))?;
    for overlay in &overlays {
        let destination = tree.join(&overlay.relative_path);
        if let Some(parent) = destination.parent() {
            ensure_real_directory(parent).map_err(io("mkdir project overlay"))?;
        }
        let mut destination_file = open_compiled_artifact_create_new(&destination)
            .map_err(|error| CompileError::Io(format!("creating project overlay: {error}")))?;
        destination_file
            .write_all(overlay.source.as_bytes())
            .and_then(|_| destination_file.sync_all())
            .map_err(io("writing project overlay"))?;
    }

    let regen = run_regen(&opts.game_dir, &tree).map_err(CompileError::Regen)?;
    validate_generated_cache(&regen).map_err(CompileError::Other)?;
    validate_project_regen_manifest(&base_manifest, &overlays, &regen)
        .map_err(CompileError::Other)?;
    Ok(())
}

fn validate_project_compiler_check_bounds(
    opts: &ProjectCompilerCheckOpts,
) -> Result<(), CompileError> {
    if opts.overlays.is_empty() {
        return Err(CompileError::Other(
            "project compiler check requires at least one sealed module".to_owned(),
        ));
    }
    if opts.overlays.len() > MAX_PROJECT_COMPILER_CHECK_MODULES {
        return Err(CompileError::Other(format!(
            "project compiler module count {} exceeds {}",
            opts.overlays.len(),
            MAX_PROJECT_COMPILER_CHECK_MODULES
        )));
    }
    if opts.base_cache.len() > MAX_PROJECT_COMPILER_CHECK_BASE_BYTES {
        return Err(CompileError::Other(format!(
            "sealed Shipping cache has {} bytes; maximum is {}",
            opts.base_cache.len(),
            MAX_PROJECT_COMPILER_CHECK_BASE_BYTES
        )));
    }
    if opts.binds_cache.is_empty()
        || opts.binds_cache.len() > MAX_PROJECT_COMPILER_CHECK_BINDS_BYTES
    {
        return Err(CompileError::Other(format!(
            "sealed Binds.Cache has {} bytes; expected 1..={}",
            opts.binds_cache.len(),
            MAX_PROJECT_COMPILER_CHECK_BINDS_BYTES
        )));
    }

    let mut source_bytes = 0usize;
    for overlay in &opts.overlays {
        if overlay.module_name.len() > MAX_PROJECT_COMPILER_CHECK_IDENTITY_BYTES
            || overlay.rel_path.len() > MAX_PROJECT_COMPILER_CHECK_IDENTITY_BYTES
        {
            return Err(CompileError::Other(format!(
                "project module identity exceeds {} bytes",
                MAX_PROJECT_COMPILER_CHECK_IDENTITY_BYTES
            )));
        }
        source_bytes = source_bytes
            .checked_add(overlay.source.len())
            .ok_or_else(|| {
                CompileError::Other("project source byte count overflowed".to_owned())
            })?;
        if source_bytes > MAX_PROJECT_COMPILER_CHECK_SOURCE_BYTES {
            return Err(CompileError::Other(format!(
                "project source bytes exceed {}",
                MAX_PROJECT_COMPILER_CHECK_SOURCE_BYTES
            )));
        }
    }
    Ok(())
}

fn validate_project_regen_manifest(
    base: &[emit_all::ValidatedModuleIdentity],
    overlays: &[emit_all::PreparedCompileAddOverlay],
    regen: &[u8],
) -> Result<(), String> {
    use std::collections::BTreeMap;

    let regen_modules = model::parse_modules(regen)
        .map_err(|error| format!("parsing regenerated module manifest: {error}"))?;
    let actual = emit_all::validated_module_identities(&regen_modules)
        .map_err(|error| format!("invalid regenerated module manifest: {error}"))?;

    let fold = |value: &str| {
        value
            .chars()
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let mut expected = BTreeMap::<String, (&str, &str)>::new();
    for module in base {
        expected.insert(
            fold(&module.module_name),
            (&module.module_name, &module.relative_path),
        );
    }
    for module in overlays {
        expected.insert(
            fold(&module.module_name),
            (&module.module_name, &module.relative_path),
        );
    }
    let actual = actual
        .iter()
        .map(|module| {
            (
                fold(&module.module_name),
                (module.module_name.as_str(), module.relative_path.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for (key, (expected_name, expected_path)) in &expected {
        let Some((actual_name, actual_path)) = actual.get(key) else {
            return Err(format!(
                "regenerated cache is missing expected module {expected_name:?} at {expected_path:?}"
            ));
        };
        if actual_name != expected_name {
            return Err(format!(
                "regenerated module name {actual_name:?} does not exactly match expected {expected_name:?}"
            ));
        }
        if actual_path != expected_path {
            return Err(format!(
                "regenerated module {expected_name:?} path {actual_path:?} does not match expected {expected_path:?}"
            ));
        }
    }
    if let Some((_, (extra_name, extra_path))) =
        actual.iter().find(|(key, _)| !expected.contains_key(*key))
    {
        return Err(format!(
            "regenerated cache contains unexpected module {extra_name:?} at {extra_path:?}"
        ));
    }
    if actual.len() != expected.len() {
        return Err(format!(
            "regenerated module count {} does not match expected {}",
            actual.len(),
            expected.len()
        ));
    }
    Ok(())
}

/// Return the compiler-generated class methods that the source emitter deliberately omits.
/// Replacing an existing module without carrying these records forward would silently erase CDO
/// defaults (NPC/quest/dialog configuration among them), so `edit` must fail closed until the
/// records can be preserved byte-for-byte. `PreparedEmit::prepare_compile_overlay` has already
/// proved that `module_name` identifies exactly one base module before this helper is called.
fn omitted_generated_methods(
    mods: &[model::Module],
    module_name: &str,
) -> Result<Vec<String>, CompileError> {
    let matches = mods
        .iter()
        .filter(|module| module.name == module_name)
        .collect::<Vec<_>>();
    let [module] = matches.as_slice() else {
        return Err(CompileError::Other(format!(
            "cannot inventory compiler-generated methods for edit module {module_name:?}: \
             expected exactly one base module, found {}",
            matches.len()
        )));
    };

    // A generated method is identified by class + method name. Refuse malformed/ambiguous class
    // identities even though PreparedEmit normally rejects the surrounding edit first; this
    // helper must never turn ambiguity into an empty inventory if its call order changes later.
    let mut class_names = std::collections::HashSet::new();
    let mut omitted = Vec::new();
    for class in &module.classes {
        if !class_names.insert(class.name.as_str()) {
            return Err(CompileError::Other(format!(
                "cannot inventory compiler-generated methods for edit module {module_name:?}: \
                 duplicate class identity {:?}",
                class.name
            )));
        }
        let mut generated_names = std::collections::HashSet::new();
        for method in &class.methods {
            if !method.name.starts_with("__") {
                continue;
            }
            if !generated_names.insert(method.name.as_str()) {
                return Err(CompileError::Other(format!(
                    "cannot inventory compiler-generated methods for edit module \
                     {module_name:?}: duplicate generated method identity {}::{}",
                    class.name, method.name
                )));
            }
            omitted.push(format!("{}::{}", class.name, method.name));
        }
    }
    Ok(omitted)
}

fn prepare_generated_defaults_edit(
    op: &str,
    mods: &[model::Module],
    module_name: &str,
    base: &[u8],
    overlay: &str,
    allow_new_symbols: bool,
) -> Result<Option<crate::cache::generated_defaults::GeneratedDefaultsPlan>, CompileError> {
    if op != "edit" {
        return Ok(None);
    }
    let omitted = omitted_generated_methods(mods, module_name)?;
    if omitted.is_empty() {
        return Ok(None);
    }
    let preview = omitted
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let more = omitted.len().saturating_sub(4);
    let suffix = if more == 0 {
        String::new()
    } else {
        format!(", and {more} more")
    };
    let refusal = |reason: &str| {
        CompileError::Other(format!(
            "refusing to edit module {module_name:?}: it contains {} compiler-generated `__*` \
             method(s) omitted by source emission ({preview}{suffix}); {reason}",
            omitted.len()
        ))
    };
    if allow_new_symbols {
        return Err(refusal(
            "generated-default carry requires strict base-keyspace remap; disable \
             --allow-new-symbols or use `add` for a new module",
        ));
    }
    if source_contains_default_token(overlay).map_err(|reason| refusal(&reason))? {
        // The overlay authors class defaults itself, so the compiler REGENERATES
        // `__InitDefaults` from that source and the carried copy is superseded — carrying it
        // would reinstate the old values over the edited ones. Skip the carry, but only once
        // every record it would have carried is proven superseded, class by class: a partially
        // authored overlay would silently lose the classes it left out.
        let authored = crate::cache::default_source::classes_with_default_statements(overlay)
            .map_err(|reason| refusal(&reason))?;
        let mut unsuperseded = Vec::new();
        for entry in &omitted {
            match entry.strip_suffix("::__InitDefaults") {
                // Only a class-defaults initializer is regenerated from `default` statements;
                // any other generated method would still need its byte-exact carry.
                None => unsuperseded.push(entry.clone()),
                Some(class) if !authored.contains(class) => unsuperseded.push(entry.clone()),
                Some(_) => {}
            }
        }
        if !unsuperseded.is_empty() {
            return Err(refusal(&format!(
                "the authored overlay declares `default` statements, which makes the compiler \
                 regenerate the class defaults and the carried copies stale, but it does not \
                 supersede {} of them ({}); author defaults for every class, or emit the module \
                 with `--no-defaults` and edit that",
                unsuperseded.len(),
                unsuperseded.join(", ")
            )));
        }
        return Ok(None);
    }
    let plan =
        crate::cache::generated_defaults::GeneratedDefaultsPlan::prepare(base, mods, module_name)
            .map_err(|reason| {
                refusal(&format!(
                    "exact generated-default carry is unproven: {reason}"
                ))
            })?
            .ok_or_else(|| {
                refusal(
                    "the raw base module did not contain the generated methods found by the model",
                )
            })?;
    if plan.generated_count() != omitted.len() {
        return Err(refusal(&format!(
            "raw/model generated-method inventory mismatch ({}/{})",
            plan.generated_count(),
            omitted.len()
        )));
    }
    Ok(Some(plan))
}

/// Find a real `default` token while ignoring comments and quoted literals. A malformed lexical
/// construct is an error, not an excuse to launch the compiler without proving the overlay safe.
fn source_contains_default_token(source: &str) -> Result<bool, String> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        closed = true;
                        break;
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return Err(
                        "authored overlay has an unterminated block comment before generated-default preflight"
                            .into(),
                    );
                }
            }
            quote @ (b'\'' | b'"') => {
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        closed = true;
                        break;
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return Err(
                        "authored overlay has an unterminated quoted literal before generated-default preflight"
                            .into(),
                    );
                }
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                if &bytes[start..index] == b"default"
                    && !default_token_is_switch_label(bytes, index)?
                {
                    return Ok(true);
                }
            }
            _ => index += 1,
        }
    }
    Ok(false)
}

/// `default:` is a normal switch label and does not author a CDO default. Skip trivia after the
/// token so `default /* comment */ :` is classified correctly; malformed comments fail closed.
fn default_token_is_switch_label(bytes: &[u8], mut index: usize) -> Result<bool, String> {
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index..index + 2) == Some(b"//") {
            index += 2;
            while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                index += 1;
            }
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"/*") {
            index += 2;
            let mut closed = false;
            while index < bytes.len() {
                if bytes.get(index..index + 2) == Some(b"*/") {
                    index += 2;
                    closed = true;
                    break;
                } else {
                    index += 1;
                }
            }
            if !closed {
                return Err(
                    "authored overlay has an unterminated block comment after a `default` token"
                        .into(),
                );
            }
            continue;
        }
        return Ok(bytes.get(index) == Some(&b':'));
    }
}

fn io(ctx: &str) -> impl FnOnce(std::io::Error) -> CompileError {
    let ctx = ctx.to_string();
    move |e| CompileError::Io(format!("{ctx}: {e}"))
}

/// The `G1R` game directory: `game_dir` itself if it already ends in `G1R`, else `game_dir/G1R`.
fn is_direct_g1r_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|name| name.eq_ignore_ascii_case("G1R"))
}

fn g1r_dir(game_dir: &Path) -> PathBuf {
    if is_direct_g1r_dir(game_dir) {
        game_dir.to_path_buf()
    } else {
        game_dir.join("G1R")
    }
}

/// The install root containing `G1R/`. AngelScript writes `AS_JITTED_CODE` beside `G1R`, not
/// inside it, even when the process working directory is `G1R`.
fn game_root_dir(game_dir: &Path) -> PathBuf {
    if is_direct_g1r_dir(game_dir) {
        game_dir.parent().unwrap_or(game_dir).to_path_buf()
    } else {
        game_dir.to_path_buf()
    }
}

fn vanilla_cache(game_dir: &Path) -> PathBuf {
    g1r_dir(game_dir)
        .join("Script")
        .join("PrecompiledScript_Shipping.Cache")
}

/// The deploy backup path for a live cache: the live path with `.gore-bak` APPENDED to the full
/// filename (so `…Shipping.Cache` -> `…Shipping.Cache.gore-bak`). Mirrors gore-mod's `bak_path`;
/// built via `OsString::push` (NOT `with_extension`, which would clobber the `.Cache` extension).
fn deploy_bak_path(live: &Path) -> PathBuf {
    let mut s = live.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn compile_bak_path(live: &Path) -> PathBuf {
    append_suffix(live, ".gore-compile-bak")
}

fn compile_lock_path(game_dir: &Path) -> PathBuf {
    game_root_dir(game_dir).join(".gore-as-compile.lock")
}

fn install_mutation_lock_path(game_dir: &Path) -> PathBuf {
    game_root_dir(game_dir).join(".gore-install-mutation.lock")
}

const INSTALL_MUTATION_INIT_PREFIX: &str = ".gore-install-mutation.init-";

fn install_mutation_init_path(game_dir: &Path, owner: &str, guard_id: &str) -> PathBuf {
    let owner_tag = match owner {
        "gore-mod:manager-apply" => "manager-apply",
        "gore-mod:manager-undeploy" => "manager-undeploy",
        "gore-mod:deploy" => "legacy-deploy",
        "gore-mod:undeploy" => "legacy-undeploy",
        owner if owner.starts_with("gore-as:") => "compile",
        _ => "other",
    };
    game_root_dir(game_dir).join(format!(
        "{INSTALL_MUTATION_INIT_PREFIX}{owner_tag}--{guard_id}"
    ))
}

const G1R_SHIPPING_EXE_NAME: &str = "G1R-Win64-Shipping.exe";
const INSTALL_COMPILE_PROBE_PATH_LIMIT: usize = 4096;
const INSTALL_COMPILE_PROBE_MESSAGE_LIMIT: usize = 2048;
const INSTALL_MUTATION_OWNER_LIMIT: usize = 128;
const INSTALL_MUTATION_RECORD_LIMIT: usize = 512;
const INSTALL_MUTATION_GUARD_ID_LIMIT: usize = 160;

/// Cross-tool exclusive ownership of a live game installation mutation.
///
/// A complete, synced ownership payload is published atomically at
/// `<game-root>/.gore-install-mutation.lock`; the retained OS handle/lock, not its display PID,
/// proves liveness. Dropping the guard removes the lock. A caller that deliberately retains the
/// guard (for example because a child process may still be writing the installation) retains the
/// on-disk lock as well.
#[derive(Debug)]
pub struct InstallMutationGuard {
    path: PathBuf,
    owner: String,
    pid: u32,
    guard_id: String,
    payload: String,
    file: Option<std::fs::File>,
    directory_lock: Option<std::fs::File>,
    active: bool,
}

/// An abandoned manager mutation bound through the exact retained operating-system lock handle.
/// Dropping this value preserves the on-disk record; only an explicit successful [`Self::release`]
/// removes it.
#[derive(Debug)]
pub struct AbandonedInstallMutation {
    guard: Option<InstallMutationGuard>,
}

impl AbandonedInstallMutation {
    pub fn owner(&self) -> &str {
        self.guard
            .as_ref()
            .expect("abandoned mutation guard is retained until drop")
            .owner()
    }

    pub fn guard_id(&self) -> &str {
        self.guard
            .as_ref()
            .expect("abandoned mutation guard is retained until drop")
            .guard_id()
    }

    pub fn path(&self) -> &Path {
        self.guard
            .as_ref()
            .expect("abandoned mutation guard is retained until drop")
            .path()
    }

    pub fn release(&mut self) -> Result<(), String> {
        self.guard
            .as_mut()
            .expect("abandoned mutation guard is retained until drop")
            .release()
    }

    pub fn preserve_for_manual_recovery(mut self) {
        if let Some(guard) = self.guard.take() {
            guard.preserve_for_manual_recovery();
        }
    }
}

impl Drop for AbandonedInstallMutation {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            guard.preserve_for_manual_recovery();
        }
    }
}

/// Result of binding an existing lock for Manager recovery. Liveness is decided solely by the
/// operating-system handle/lock; the bounded PID field remains display-only.
#[derive(Debug)]
pub enum InstallMutationTakeover {
    Missing,
    Busy,
    Owned(AbandonedInstallMutation),
    LegacyAmbiguous(AbandonedInstallMutation),
    CompileOwner { owner: String, guard_id: String },
    Invalid { message: String },
}

fn install_mutation_open_options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };

        // Retain DELETE authority on this exact handle while denying write/delete sharing. That
        // makes the ownership record immutable for the guard lifetime and lets release delete the
        // same file object by handle instead of checking one pathname object and unlinking another.
        options
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
}

fn install_mutation_open_existing_options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };
        options
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
}

#[cfg(unix)]
fn acquire_install_mutation_directory_lock(
    game_dir: &Path,
) -> std::io::Result<Option<std::fs::File>> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let root = std::fs::canonicalize(game_root_dir(game_dir))?;
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW);
    let directory = options.open(root)?;
    lock_install_mutation_handle(&directory)?;
    Ok(Some(directory))
}

#[cfg(not(unix))]
fn acquire_install_mutation_directory_lock(
    _game_dir: &Path,
) -> std::io::Result<Option<std::fs::File>> {
    Ok(None)
}

#[cfg(windows)]
fn lock_install_mutation_handle(_file: &std::fs::File) -> std::io::Result<()> {
    // The Windows open requests read/write/delete access while denying write/delete sharing.
    // A successful open is therefore the held-handle exclusivity proof.
    Ok(())
}

#[cfg(unix)]
fn lock_install_mutation_handle(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd as _;

    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, unix)))]
fn lock_install_mutation_handle(_file: &std::fs::File) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "install-mutation OS locking is unsupported on this platform",
    ))
}

#[cfg(windows)]
fn install_mutation_lock_busy(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32) | Some(33))
}

#[cfg(unix)]
fn install_mutation_lock_busy(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::EWOULDBLOCK) || error.raw_os_error() == Some(libc::EAGAIN)
}

#[cfg(not(any(windows, unix)))]
fn install_mutation_lock_busy(_error: &std::io::Error) -> bool {
    false
}

#[cfg(test)]
thread_local! {
    static INSTALL_MUTATION_TAKEOVER_AFTER_LOCK: std::cell::RefCell<
        Option<Box<dyn FnOnce(&Path)>>,
    > = std::cell::RefCell::new(None);
}

#[cfg(test)]
fn run_install_mutation_takeover_after_lock(path: &Path) {
    INSTALL_MUTATION_TAKEOVER_AFTER_LOCK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(unix)]
fn install_mutation_handle_still_names_path(
    file: &std::fs::File,
    path: &Path,
) -> Result<bool, String> {
    use std::os::unix::fs::MetadataExt as _;

    let held = file.metadata().map_err(|error| {
        format!(
            "reading held install-mutation lock identity {}: {error}",
            path.display()
        )
    })?;
    let current = match std::fs::symlink_metadata(path) {
        Ok(current) => current,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "reading current install-mutation lock identity {}: {error}",
                path.display()
            ));
        }
    };
    Ok(!current.file_type().is_symlink()
        && held.nlink() == 1
        && current.nlink() == 1
        && held.dev() == current.dev()
        && held.ino() == current.ino())
}

#[cfg(windows)]
fn install_mutation_handle_still_names_path(
    file: &std::fs::File,
    path: &Path,
) -> Result<bool, String> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT,
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut information) } == 0 {
        return Err(format!(
            "reading held install-mutation lock identity {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(
        information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT == 0
            && information.nNumberOfLinks == 1,
    )
}

#[cfg(not(any(unix, windows)))]
fn install_mutation_handle_still_names_path(
    _file: &std::fs::File,
    _path: &Path,
) -> Result<bool, String> {
    Ok(false)
}

fn valid_install_mutation_token(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-:".contains(&byte))
}

struct InstallMutationRecord {
    owner: String,
    pid: u32,
    guard_id: String,
}

fn parse_install_mutation_record(bytes: &[u8]) -> Result<InstallMutationRecord, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "install-mutation ownership record is not valid UTF-8".to_owned())?;
    let mut lines = text.lines();
    if lines.next() != Some("version=1") {
        return Err("install-mutation ownership record has an unsupported version".to_owned());
    }
    let owner = lines
        .next()
        .and_then(|line| line.strip_prefix("owner="))
        .filter(|owner| valid_install_mutation_token(owner, INSTALL_MUTATION_OWNER_LIMIT))
        .ok_or_else(|| "install-mutation ownership record has an invalid owner".to_owned())?;
    let pid = lines
        .next()
        .and_then(|line| line.strip_prefix("pid="))
        .and_then(|pid| pid.parse::<u32>().ok())
        .ok_or_else(|| "install-mutation ownership record has an invalid display PID".to_owned())?;
    let guard_id = lines
        .next()
        .and_then(|line| line.strip_prefix("guard_id="))
        .filter(|guard_id| valid_install_mutation_token(guard_id, INSTALL_MUTATION_GUARD_ID_LIMIT))
        .ok_or_else(|| "install-mutation ownership record has an invalid guard id".to_owned())?;
    if lines.next().is_some() {
        return Err("install-mutation ownership record contains unexpected fields".to_owned());
    }
    Ok(InstallMutationRecord {
        owner: owner.to_owned(),
        pid,
        guard_id: guard_id.to_owned(),
    })
}

fn read_install_mutation_payload(file: &mut std::fs::File, path: &Path) -> Result<Vec<u8>, String> {
    let metadata = file.metadata().map_err(|error| {
        format!(
            "reading install-mutation lock metadata {}: {error}",
            path.display()
        )
    })?;
    if metadata.len() > INSTALL_MUTATION_RECORD_LIMIT as u64 {
        return Err(format!(
            "refusing oversized install-mutation ownership record at {} ({} bytes; limit {})",
            path.display(),
            metadata.len(),
            INSTALL_MUTATION_RECORD_LIMIT
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        format!(
            "seeking install-mutation ownership record {}: {error}",
            path.display()
        )
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(INSTALL_MUTATION_RECORD_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!(
                "reading install-mutation ownership record {}: {error}",
                path.display()
            )
        })?;
    if bytes.len() > INSTALL_MUTATION_RECORD_LIMIT {
        return Err(format!(
            "refusing oversized install-mutation ownership record at {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn install_mutation_init_owner_and_guard_id(path: &Path) -> Option<(String, String)> {
    let name = path.file_name()?.to_str()?;
    let encoded = name.strip_prefix(INSTALL_MUTATION_INIT_PREFIX)?;
    let (owner_tag, guard_id) = encoded.split_once("--")?;
    if !valid_install_mutation_token(guard_id, INSTALL_MUTATION_GUARD_ID_LIMIT) {
        return None;
    }
    let owner = match owner_tag {
        "manager-apply" => "gore-mod:manager-apply",
        "manager-undeploy" => "gore-mod:manager-undeploy",
        "legacy-deploy" => "gore-mod:deploy",
        "legacy-undeploy" => "gore-mod:undeploy",
        "compile" => "gore-as:initialization",
        "other" => "other:initialization",
        _ => return None,
    };
    Some((owner.to_owned(), guard_id.to_owned()))
}

fn install_mutation_initialization_candidates(game_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let root = game_root_dir(game_dir);
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "enumerating install-mutation initialization records in {}: {error}",
                root.display()
            ));
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "enumerating install-mutation initialization records in {}: {error}",
                root.display()
            )
        })?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(INSTALL_MUTATION_INIT_PREFIX))
        {
            paths.push(entry.path());
            if paths.len() > 16 {
                return Err(format!(
                    "refusing more than 16 install-mutation initialization records in {}",
                    root.display()
                ));
            }
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(windows)]
fn publish_install_mutation_initialization(init: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let init: Vec<u16> = init
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    if unsafe { MoveFileExW(init.as_ptr(), path.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn publish_install_mutation_initialization(init: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    let init = std::ffi::CString::new(init.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "install-mutation initialization path contains NUL",
        )
    })?;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "install-mutation lock path contains NUL",
        )
    })?;
    if unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            init.as_ptr(),
            libc::AT_FDCWD,
            path.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_vendor = "apple")]
fn publish_install_mutation_initialization(init: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    let init = std::ffi::CString::new(init.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "install-mutation initialization path contains NUL",
        )
    })?;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "install-mutation lock path contains NUL",
        )
    })?;
    if unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            init.as_ptr(),
            libc::AT_FDCWD,
            path.as_ptr(),
            libc::RENAME_EXCL,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, target_os = "linux", target_vendor = "apple")))]
fn publish_install_mutation_initialization(_init: &Path, _path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic no-clobber install-mutation publication is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn sync_install_mutation_directory(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_install_mutation_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn remove_install_mutation_file_by_handle(file: &std::fs::File, path: &Path) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };

    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    let removed = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as _,
            FileDispositionInfo,
            std::ptr::from_ref(&disposition).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if removed == 0 {
        return Err(format!(
            "deleting owned install-mutation lock by handle {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn remove_install_mutation_file_by_handle(file: &std::fs::File, path: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt as _;

    let owned = file.metadata().map_err(|error| {
        format!(
            "reading owned install-mutation lock identity {}: {error}",
            path.display()
        )
    })?;
    let current = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "reading current install-mutation lock identity {}: {error}",
            path.display()
        )
    })?;
    if current.file_type().is_symlink()
        || owned.nlink() != 1
        || current.nlink() != 1
        || owned.dev() != current.dev()
        || owned.ino() != current.ino()
    {
        return Err(
            "refusing to remove install-mutation lock because its filesystem identity changed"
                .to_owned(),
        );
    }
    // Unix has no portable unlink-by-handle primitive. The inode check above is the strongest
    // portable guard; the supported live game platform uses the handle-bound Windows path.
    std::fs::remove_file(path).map_err(|error| {
        format!(
            "removing owned install-mutation lock {}: {error}",
            path.display()
        )
    })
}

#[cfg(not(any(windows, unix)))]
fn remove_install_mutation_file_by_handle(
    _file: &std::fs::File,
    _path: &Path,
) -> Result<(), String> {
    Err("identity-bound install-mutation lock release is unsupported on this platform".to_owned())
}

fn remove_install_mutation_initialization_if_owned(
    init_path: &Path,
    expected_payload: &[u8],
) -> Result<(), String> {
    let mut file = install_mutation_open_existing_options()
        .open(init_path)
        .map_err(|error| {
            format!(
                "opening failed install-mutation initialization {}: {error}",
                init_path.display()
            )
        })?;
    lock_install_mutation_handle(&file).map_err(|error| {
        format!(
            "locking failed install-mutation initialization {}: {error}",
            init_path.display()
        )
    })?;
    if !install_mutation_handle_still_names_path(&file, init_path)? {
        return Err(format!(
            "refusing changed install-mutation initialization {}",
            init_path.display()
        ));
    }
    let actual = read_install_mutation_payload(&mut file, init_path)?;
    if actual != expected_payload {
        return Err(format!(
            "refusing changed install-mutation initialization payload {}",
            init_path.display()
        ));
    }
    remove_install_mutation_file_by_handle(&file, init_path)
}

fn take_over_abandoned_initialization(
    game_dir: &Path,
    directory_lock: Option<std::fs::File>,
) -> InstallMutationTakeover {
    let candidates = match install_mutation_initialization_candidates(game_dir) {
        Ok(candidates) => candidates,
        Err(message) => return InstallMutationTakeover::Invalid { message },
    };
    let [path] = candidates.as_slice() else {
        return if candidates.is_empty() {
            InstallMutationTakeover::Missing
        } else {
            InstallMutationTakeover::Invalid {
                message: "multiple interrupted install-mutation initializations require manual inspection"
                    .to_owned(),
            }
        };
    };
    let Some((owner, guard_id)) = install_mutation_init_owner_and_guard_id(path) else {
        return InstallMutationTakeover::Invalid {
            message: format!(
                "invalid install-mutation initialization name: {}",
                path.display()
            ),
        };
    };
    let mut file = match install_mutation_open_existing_options().open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return InstallMutationTakeover::Missing;
        }
        Err(error) if install_mutation_lock_busy(&error) => {
            return InstallMutationTakeover::Busy;
        }
        Err(error) => {
            return InstallMutationTakeover::Invalid {
                message: format!(
                    "opening install-mutation initialization {}: {error}",
                    path.display()
                ),
            };
        }
    };
    if let Err(error) = lock_install_mutation_handle(&file) {
        return if install_mutation_lock_busy(&error) {
            InstallMutationTakeover::Busy
        } else {
            InstallMutationTakeover::Invalid {
                message: format!(
                    "locking install-mutation initialization {}: {error}",
                    path.display()
                ),
            }
        };
    }
    #[cfg(test)]
    run_install_mutation_takeover_after_lock(path);
    match install_mutation_handle_still_names_path(&file, path) {
        Ok(true) => {}
        Ok(false) => {
            return InstallMutationTakeover::Invalid {
                message: bounded_probe_text(
                    &format!(
                        "install-mutation initialization changed filesystem identity after it was \
                         locked: {}",
                        path.display()
                    ),
                    INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                )
                .0,
            };
        }
        Err(message) => return InstallMutationTakeover::Invalid { message },
    }
    let payload = match read_install_mutation_payload(&mut file, path) {
        Ok(payload) => payload,
        Err(message) => return InstallMutationTakeover::Invalid { message },
    };
    if let Ok(parsed) = parse_install_mutation_record(&payload) {
        let owner_matches_name = if owner == "gore-as:initialization" {
            parsed.owner.starts_with("gore-as:")
        } else {
            parsed.owner == owner
        };
        if !owner_matches_name || parsed.guard_id != guard_id {
            return InstallMutationTakeover::Invalid {
                message: format!(
                    "install-mutation initialization name and payload disagree: {}",
                    path.display()
                ),
            };
        }
        if owner == "gore-as:initialization" {
            return InstallMutationTakeover::CompileOwner {
                owner: parsed.owner,
                guard_id: parsed.guard_id,
            };
        }
    }
    if owner.starts_with("gore-as:") {
        return InstallMutationTakeover::CompileOwner { owner, guard_id };
    }
    let legacy_ambiguous = matches!(owner.as_str(), "gore-mod:deploy" | "gore-mod:undeploy");
    if !matches!(
        owner.as_str(),
        "gore-mod:manager-apply"
            | "gore-mod:manager-undeploy"
            | "gore-mod:deploy"
            | "gore-mod:undeploy"
    ) {
        return InstallMutationTakeover::Invalid {
            message: format!(
                "install-mutation initialization owner '{owner}' is not recoverable by Manager"
            ),
        };
    }
    let payload = match String::from_utf8(payload) {
        Ok(payload) => payload,
        Err(_) => {
            return InstallMutationTakeover::Invalid {
                message: "install-mutation initialization is not valid UTF-8".to_owned(),
            };
        }
    };
    let abandoned = AbandonedInstallMutation {
        guard: Some(InstallMutationGuard {
            path: path.clone(),
            owner,
            pid: 0,
            guard_id,
            payload,
            file: Some(file),
            directory_lock,
            active: true,
        }),
    };
    if legacy_ambiguous {
        InstallMutationTakeover::LegacyAmbiguous(abandoned)
    } else {
        InstallMutationTakeover::Owned(abandoned)
    }
}

impl InstallMutationGuard {
    pub fn acquire(game_dir: &Path, owner: &str) -> Result<Self, String> {
        if owner.is_empty()
            || owner.len() > INSTALL_MUTATION_OWNER_LIMIT
            || !owner
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-:".contains(&byte))
        {
            return Err(format!(
                "install-mutation owner must be 1..={INSTALL_MUTATION_OWNER_LIMIT} ASCII bytes \
                 using only letters, digits, '.', '_', '-', or ':'"
            ));
        }
        let directory_lock =
            acquire_install_mutation_directory_lock(game_dir).map_err(|error| {
                let message = if install_mutation_lock_busy(&error) {
                    "another install mutation is active (install-root ownership is held)".to_owned()
                } else {
                    format!("locking the canonical install root: {error}")
                };
                bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT).0
            })?;
        let path = install_mutation_lock_path(game_dir);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                return Err(format!(
                    "another install mutation is active (lock exists: {})",
                    path.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "inspecting install-mutation lock {}: {error}",
                    path.display()
                ));
            }
        }
        let initializations = install_mutation_initialization_candidates(game_dir)?;
        if let Some(initialization) = initializations.first() {
            return Err(format!(
                "an interrupted install-mutation initialization requires recovery: {}",
                initialization.display()
            ));
        }
        let pid = std::process::id();
        static GUARD_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let sequence = GUARD_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let created_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let guard_id = format!("{pid}-{created_nanos}-{sequence}");
        let payload = format!("version=1\nowner={owner}\npid={pid}\nguard_id={guard_id}\n");
        let init_path = install_mutation_init_path(game_dir, owner, &guard_id);
        let mut init_file = install_mutation_open_options()
            .open(&init_path)
            .map_err(|error| {
                bounded_probe_text(
                    &format!(
                        "creating install-mutation initialization {}: {error}",
                        init_path.display()
                    ),
                    INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                )
                .0
            })?;
        if let Err(error) = lock_install_mutation_handle(&init_file) {
            let cleanup = remove_install_mutation_file_by_handle(&init_file, &init_path).err();
            drop(init_file);
            let message = match cleanup {
                Some(cleanup) => format!(
                    "locking new install-mutation initialization {}: {error}; additionally failed \
                     to remove it: {cleanup}",
                    init_path.display()
                ),
                None => format!(
                    "locking new install-mutation initialization {}: {error}",
                    init_path.display()
                ),
            };
            return Err(bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT).0);
        }
        if let Err(error) = init_file
            .write_all(payload.as_bytes())
            .and_then(|_| init_file.sync_all())
        {
            let cleanup = remove_install_mutation_file_by_handle(&init_file, &init_path).err();
            drop(init_file);
            let message = match cleanup {
                Some(cleanup) => format!(
                    "initializing install-mutation lock {}: {error}; additionally failed to \
                     remove it: {cleanup}",
                    init_path.display()
                ),
                None => format!(
                    "initializing install-mutation lock {}: {error}",
                    init_path.display()
                ),
            };
            return Err(bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT).0);
        }
        drop(init_file);
        if let Err(error) = publish_install_mutation_initialization(&init_path, &path) {
            let cleanup =
                remove_install_mutation_initialization_if_owned(&init_path, payload.as_bytes())
                    .err();
            let message = match cleanup {
                Some(cleanup) => format!(
                    "publishing install-mutation lock {}: {error}; additionally failed to remove \
                     initialization {}: {cleanup}",
                    path.display(),
                    init_path.display()
                ),
                None => format!(
                    "publishing install-mutation lock {}: {error}",
                    path.display()
                ),
            };
            return Err(bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT).0);
        }
        if let Some(parent) = path.parent() {
            sync_install_mutation_directory(parent).map_err(|error| {
                format!(
                    "syncing published install-mutation lock directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        let mut file = install_mutation_open_existing_options()
            .open(&path)
            .map_err(|error| {
                bounded_probe_text(
                    &format!(
                        "opening published install-mutation lock {}: {error}",
                        path.display()
                    ),
                    INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                )
                .0
            })?;
        lock_install_mutation_handle(&file).map_err(|error| {
            bounded_probe_text(
                &format!(
                    "locking published install-mutation record {}: {error}",
                    path.display()
                ),
                INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
            )
            .0
        })?;
        if !install_mutation_handle_still_names_path(&file, &path)? {
            return Err(format!(
                "published install-mutation record changed filesystem identity: {}",
                path.display()
            ));
        }
        if read_install_mutation_payload(&mut file, &path)? != payload.as_bytes() {
            return Err(format!(
                "published install-mutation ownership record changed before it was acquired: {}",
                path.display()
            ));
        }
        Ok(Self {
            path,
            owner: owner.to_owned(),
            pid,
            guard_id,
            payload,
            file: Some(file),
            directory_lock,
            active: true,
        })
    }

    /// Bind an existing Manager lock only when the operating system proves its original handle is
    /// no longer held. A stale compile owner is never returned as recoverable Manager ownership.
    pub fn take_over_abandoned_manager(game_dir: &Path) -> InstallMutationTakeover {
        let directory_lock = match acquire_install_mutation_directory_lock(game_dir) {
            Ok(directory_lock) => directory_lock,
            Err(error) if install_mutation_lock_busy(&error) => {
                return InstallMutationTakeover::Busy;
            }
            Err(error) => {
                return InstallMutationTakeover::Invalid {
                    message: bounded_probe_text(
                        &format!("locking the canonical install root for recovery: {error}"),
                        INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                    )
                    .0,
                };
            }
        };
        let path = install_mutation_lock_path(game_dir);
        let mut file = match install_mutation_open_existing_options().open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return take_over_abandoned_initialization(game_dir, directory_lock);
            }
            Err(error) if install_mutation_lock_busy(&error) => {
                return InstallMutationTakeover::Busy;
            }
            Err(error) => {
                return InstallMutationTakeover::Invalid {
                    message: bounded_probe_text(
                        &format!(
                            "opening existing install-mutation lock {}: {error}",
                            path.display()
                        ),
                        INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                    )
                    .0,
                };
            }
        };
        let metadata = match file.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => {
                return InstallMutationTakeover::Invalid {
                    message: format!(
                        "existing install-mutation lock is not a regular file: {}",
                        path.display()
                    ),
                };
            }
            Err(error) => {
                return InstallMutationTakeover::Invalid {
                    message: format!(
                        "reading existing install-mutation lock metadata {}: {error}",
                        path.display()
                    ),
                };
            }
        };
        if metadata.len() > INSTALL_MUTATION_RECORD_LIMIT as u64 {
            return InstallMutationTakeover::Invalid {
                message: format!(
                    "refusing oversized install-mutation ownership record at {} ({} bytes; limit {})",
                    path.display(),
                    metadata.len(),
                    INSTALL_MUTATION_RECORD_LIMIT
                ),
            };
        }
        if let Err(error) = lock_install_mutation_handle(&file) {
            if install_mutation_lock_busy(&error) {
                return InstallMutationTakeover::Busy;
            }
            return InstallMutationTakeover::Invalid {
                message: bounded_probe_text(
                    &format!(
                        "locking existing install-mutation record {}: {error}",
                        path.display()
                    ),
                    INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                )
                .0,
            };
        }
        #[cfg(test)]
        run_install_mutation_takeover_after_lock(&path);
        match install_mutation_handle_still_names_path(&file, &path) {
            Ok(true) => {}
            Ok(false) => {
                return InstallMutationTakeover::Invalid {
                    message: bounded_probe_text(
                        &format!(
                            "install-mutation lock changed filesystem identity after it was \
                             locked: {}",
                            path.display()
                        ),
                        INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                    )
                    .0,
                };
            }
            Err(message) => return InstallMutationTakeover::Invalid { message },
        }
        let payload = match read_install_mutation_payload(&mut file, &path) {
            Ok(payload) => payload,
            Err(message) => return InstallMutationTakeover::Invalid { message },
        };
        let parsed = match parse_install_mutation_record(&payload) {
            Ok(parsed) => parsed,
            Err(message) => {
                return InstallMutationTakeover::Invalid {
                    message: bounded_probe_text(
                        &format!(
                            "invalid install-mutation lock {}: {message}",
                            path.display()
                        ),
                        INSTALL_COMPILE_PROBE_MESSAGE_LIMIT,
                    )
                    .0,
                };
            }
        };
        if parsed.owner.starts_with("gore-as:") {
            return InstallMutationTakeover::CompileOwner {
                owner: parsed.owner,
                guard_id: parsed.guard_id,
            };
        }
        let legacy_ambiguous = matches!(
            parsed.owner.as_str(),
            "gore-mod:deploy" | "gore-mod:undeploy"
        );
        if !matches!(
            parsed.owner.as_str(),
            "gore-mod:manager-apply"
                | "gore-mod:manager-undeploy"
                | "gore-mod:deploy"
                | "gore-mod:undeploy"
        ) {
            return InstallMutationTakeover::Invalid {
                message: format!(
                    "install-mutation owner '{}' is not a recoverable Manager operation",
                    parsed.owner
                ),
            };
        }
        let payload = match String::from_utf8(payload) {
            Ok(payload) => payload,
            Err(_) => {
                return InstallMutationTakeover::Invalid {
                    message: "install-mutation ownership record is not valid UTF-8".to_owned(),
                };
            }
        };
        let abandoned = AbandonedInstallMutation {
            guard: Some(InstallMutationGuard {
                path,
                owner: parsed.owner,
                pid: parsed.pid,
                guard_id: parsed.guard_id,
                payload,
                file: Some(file),
                directory_lock,
                active: true,
            }),
        };
        if legacy_ambiguous {
            InstallMutationTakeover::LegacyAmbiguous(abandoned)
        } else {
            InstallMutationTakeover::Owned(abandoned)
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn guard_id(&self) -> &str {
        &self.guard_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Close this process's handle without deleting or retrying deletion of the on-disk blocker.
    /// Call only after an explicit release failure that must remain stable for manual recovery.
    pub fn preserve_for_manual_recovery(mut self) {
        self.close_handle_preserving_record();
    }

    fn close_handle_preserving_record(&mut self) {
        self.active = false;
        drop(self.file.take());
        drop(self.directory_lock.take());
    }

    pub fn release(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        let file = self.file.as_mut().ok_or_else(|| {
            "install-mutation guard lost its owned file handle before release".to_owned()
        })?;
        let current = read_install_mutation_payload(file, &self.path).map_err(|message| {
            bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT).0
        })?;
        if current != self.payload.as_bytes() {
            return Err(
                "refusing to remove install-mutation lock because its bounded ownership record \
                 changed while the guard was active"
                    .to_owned(),
            );
        }
        remove_install_mutation_file_by_handle(file, &self.path).map_err(|message| {
            bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT).0
        })?;
        drop(self.file.take());
        drop(self.directory_lock.take());
        self.active = false;
        Ok(())
    }
}

impl Drop for InstallMutationGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = self.release();
        }
    }
}

/// Why the live install may or may not be entered by an AngelScript compiler transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCompileStateDisposition {
    SafeToCompile,
    GameProcessRunning,
    RecoveryArtifactsPresent,
    InspectionFailed,
}

/// Result of the native shipping-process inspection used by the install preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCompileGameProcessDisposition {
    NotRunning,
    Running,
    InspectionFailed,
}

/// A known disk artifact owned by an interrupted or currently active compile transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCompileArtifactKind {
    InstallMutationLock,
    CompileLock,
    RecoveryJournal,
    ShippingCacheBackup,
    JittedCodeBackup,
    Ue4ssProxyBackup,
}

/// One present install-compile artifact. `path` is a bounded lossy display value; callers must not
/// use it as filesystem authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCompileArtifact {
    pub kind: InstallCompileArtifactKind,
    pub path: String,
    pub path_truncated: bool,
}

/// Which read-only portion of the install-state inspection could not be completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCompileInspectionIssueKind {
    GameProcessEnumeration,
    ArtifactMetadata,
}

/// A bounded, display-only inspection failure. Any issue makes the probe fail closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCompileInspectionIssue {
    pub kind: InstallCompileInspectionIssueKind,
    pub path: Option<String>,
    pub path_truncated: bool,
    pub message: String,
    pub message_truncated: bool,
}

/// Read-only snapshot of the native game process and every known gore-as recovery artifact.
///
/// The probe never creates, removes, renames, or writes a path. Its returned path/message strings
/// are bounded display data, not paths that recovery code should execute blindly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCompileStateProbe {
    pub disposition: InstallCompileStateDisposition,
    pub safe_to_compile: bool,
    pub game_process: InstallCompileGameProcessDisposition,
    pub artifacts: Vec<InstallCompileArtifact>,
    pub issues: Vec<InstallCompileInspectionIssue>,
}

fn bounded_probe_text(value: &str, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value.to_owned(), false);
    }
    let mut end = limit;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_owned(), true)
}

fn bounded_probe_path(path: &Path) -> (String, bool) {
    bounded_probe_text(
        &path.as_os_str().to_string_lossy(),
        INSTALL_COMPILE_PROBE_PATH_LIMIT,
    )
}

fn install_compile_artifact_paths(game_dir: &Path) -> Vec<(InstallCompileArtifactKind, PathBuf)> {
    let root = game_root_dir(game_dir);
    let g1r = g1r_dir(game_dir);
    let shipping = vanilla_cache(game_dir);
    let jitted = root.join("AS_JITTED_CODE");
    let proxy = g1r.join("Binaries").join("Win64").join("dwmapi.dll");
    vec![
        (
            InstallCompileArtifactKind::InstallMutationLock,
            install_mutation_lock_path(game_dir),
        ),
        (
            InstallCompileArtifactKind::CompileLock,
            compile_lock_path(game_dir),
        ),
        (
            InstallCompileArtifactKind::RecoveryJournal,
            recovery_journal_path(game_dir),
        ),
        (
            InstallCompileArtifactKind::ShippingCacheBackup,
            compile_bak_path(&shipping),
        ),
        (
            InstallCompileArtifactKind::JittedCodeBackup,
            append_suffix(&jitted, ".gore-compile-bak"),
        ),
        (
            InstallCompileArtifactKind::Ue4ssProxyBackup,
            append_suffix(&proxy, ".gore-compile-bak"),
        ),
    ]
}

#[cfg(windows)]
fn native_shipping_game_process_running() -> Result<bool, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    struct ProcessSnapshot(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for ProcessSnapshot {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(format!(
            "CreateToolhelp32Snapshot(processes) failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let snapshot = ProcessSnapshot(snapshot);
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    if unsafe { Process32FirstW(snapshot.0, &mut entry) } == 0 {
        return Err(format!(
            "Process32FirstW failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    loop {
        let len = entry
            .szExeFile
            .iter()
            .position(|ch| *ch == 0)
            .unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..len])
            .eq_ignore_ascii_case(G1R_SHIPPING_EXE_NAME)
        {
            return Ok(true);
        }
        if unsafe { Process32NextW(snapshot.0, &mut entry) } == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                return Ok(false);
            }
            return Err(format!("Process32NextW failed: {error}"));
        }
    }
}

#[cfg(not(windows))]
fn native_shipping_game_process_running() -> Result<bool, String> {
    // The supported game executable is Windows-only. Keeping the non-Windows implementation
    // read-only and deterministic preserves offline cache tooling/tests on other hosts.
    Ok(false)
}

/// The single place from which install-mutating code asks whether the game is open.
///
/// Why this seam exists: a compiler-transaction test is about the transaction — which locks it
/// takes, which journal it leaves behind, which bytes it puts back — and a machine-global process
/// check makes it about the machine instead, so the same code passes or fails depending on whether
/// the developer happens to have Gothic running while the suite runs. Test builds therefore read
/// the answer the fixture states with [`StatedGameProcess`] and never enumerate anything. The real
/// inspection is the only implementation compiled into a shipped binary: there is no environment
/// variable and no runtime switch that can answer for it or turn the guard off.
#[cfg(not(test))]
fn shipping_game_process_running() -> Result<bool, String> {
    native_shipping_game_process_running()
}

#[cfg(test)]
fn shipping_game_process_running() -> Result<bool, String> {
    Ok(STATED_GAME_PROCESS.with(|stated| stated.get()).expect(
        "this test reached the install-mutation game-process guard without stating whether the \
         game is running; hold a StatedGameProcess for the duration of the fixture",
    ))
}

#[cfg(test)]
thread_local! {
    /// The running test's stated answer; `None` means the test never said, which is a test defect
    /// rather than a licence to consult the developer's desktop.
    static STATED_GAME_PROCESS: std::cell::Cell<Option<bool>> = std::cell::Cell::new(None);
}

/// A test's stated answer to "is the game running?", in force until the guard is dropped.
#[cfg(test)]
struct StatedGameProcess {
    previous: Option<bool>,
}

#[cfg(test)]
impl StatedGameProcess {
    fn not_running() -> Self {
        Self::state(false)
    }

    fn running() -> Self {
        Self::state(true)
    }

    fn state(running: bool) -> Self {
        let previous = STATED_GAME_PROCESS.with(|stated| stated.replace(Some(running)));
        Self { previous }
    }
}

#[cfg(test)]
impl Drop for StatedGameProcess {
    fn drop(&mut self) {
        let previous = self.previous;
        STATED_GAME_PROCESS.with(|stated| stated.set(previous));
    }
}

fn require_shipping_game_process_closed_with<C>(check_game_process: C) -> Result<(), String>
where
    C: FnOnce() -> Result<bool, String>,
{
    match check_game_process() {
        Ok(false) => Ok(()),
        Ok(true) => Err(format!(
            "refusing install mutation while {G1R_SHIPPING_EXE_NAME} is running; close the game \
             and retry"
        )),
        Err(error) => Err(format!(
            "refusing install mutation because native game-process inspection failed: {error}"
        )),
    }
}

/// Recheck the native shipping process immediately before the caller's first live-game mutation.
///
/// This deliberately does not claim an OS launch barrier: the game does not participate in the
/// gore install lock, so a process can still start after enumeration. Callers keep this check as
/// close as possible to the first write and must not present it as proof that a later launch is
/// impossible.
pub fn require_shipping_game_process_closed() -> Result<(), String> {
    require_shipping_game_process_closed_with(shipping_game_process_running)
}

fn probe_install_compile_state_with<C>(
    game_dir: &Path,
    check_game_process: C,
) -> InstallCompileStateProbe
where
    C: FnOnce() -> Result<bool, String>,
{
    let mut issues = Vec::new();
    let game_process = match check_game_process() {
        Ok(true) => InstallCompileGameProcessDisposition::Running,
        Ok(false) => InstallCompileGameProcessDisposition::NotRunning,
        Err(error) => {
            let (message, message_truncated) =
                bounded_probe_text(&error, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);
            issues.push(InstallCompileInspectionIssue {
                kind: InstallCompileInspectionIssueKind::GameProcessEnumeration,
                path: None,
                path_truncated: false,
                message,
                message_truncated,
            });
            InstallCompileGameProcessDisposition::InspectionFailed
        }
    };

    let mut artifacts = Vec::new();
    for (kind, path) in install_compile_artifact_paths(game_dir) {
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                let (path, path_truncated) = bounded_probe_path(&path);
                artifacts.push(InstallCompileArtifact {
                    kind,
                    path,
                    path_truncated,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                let (display, path_truncated) = bounded_probe_path(&path);
                let (message, message_truncated) =
                    bounded_probe_text(&error.to_string(), INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);
                issues.push(InstallCompileInspectionIssue {
                    kind: InstallCompileInspectionIssueKind::ArtifactMetadata,
                    path: Some(display),
                    path_truncated,
                    message,
                    message_truncated,
                });
            }
        }
    }
    match install_mutation_initialization_candidates(game_dir) {
        Ok(paths) => {
            for path in paths {
                let (path, path_truncated) = bounded_probe_path(&path);
                artifacts.push(InstallCompileArtifact {
                    kind: InstallCompileArtifactKind::InstallMutationLock,
                    path,
                    path_truncated,
                });
            }
        }
        Err(error) => {
            let root = game_root_dir(game_dir);
            let (path, path_truncated) = bounded_probe_path(&root);
            let (message, message_truncated) =
                bounded_probe_text(&error, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);
            issues.push(InstallCompileInspectionIssue {
                kind: InstallCompileInspectionIssueKind::ArtifactMetadata,
                path: Some(path),
                path_truncated,
                message,
                message_truncated,
            });
        }
    }

    let disposition = if !issues.is_empty() {
        InstallCompileStateDisposition::InspectionFailed
    } else if game_process == InstallCompileGameProcessDisposition::Running {
        InstallCompileStateDisposition::GameProcessRunning
    } else if !artifacts.is_empty() {
        InstallCompileStateDisposition::RecoveryArtifactsPresent
    } else {
        InstallCompileStateDisposition::SafeToCompile
    };
    InstallCompileStateProbe {
        safe_to_compile: disposition == InstallCompileStateDisposition::SafeToCompile,
        disposition,
        game_process,
        artifacts,
        issues,
    }
}

/// Inspect whether gore-as may safely begin a live-install compiler transaction.
///
/// On Windows this fail-closed probe enumerates processes with Toolhelp and compares executable
/// names case-insensitively against `G1R-Win64-Shipping.exe`. It also reports the cross-tool
/// install-mutation lock, gore-as compile lock, recovery journal, Shipping-cache backup, JIT
/// quarantine backup, and UE4SS-proxy backup. Any enumeration/metadata error, running game
/// process, or present artifact sets `safe_to_compile` to false. The function is strictly
/// read-only.
pub fn probe_install_compile_state(game_dir: &Path) -> InstallCompileStateProbe {
    probe_install_compile_state_with(game_dir, shipping_game_process_running)
}

/// [`probe_install_compile_state`] with the game-process answer supplied by the caller.
///
/// For crates that layer their own install mutation on this probe and test it against temporary
/// fixture trees. They link this crate compiled *without* `cfg(test)`, so the in-crate seam our own
/// fixtures use cannot reach them, and the real process list would answer instead — making their
/// deploy tests depend on whether a developer has the game open rather than on the transaction
/// under test. Production has no way to reach this: every shipped path goes through the wrapper
/// above.
pub fn probe_install_compile_state_with_stated_game_process<C>(
    game_dir: &Path,
    check_game_process: C,
) -> InstallCompileStateProbe
where
    C: FnOnce() -> Result<bool, String>,
{
    probe_install_compile_state_with(game_dir, check_game_process)
}

/// Acquire the shared live-install guard before resolving any pristine compiler input.
///
/// The returned guard must be passed to
/// [`compile_module_with_diagnostics_report_with_guard`]. Holding it across pristine resolution and
/// compiler use closes cross-tool deploy/undeploy races without recursively reacquiring the lock.
pub fn acquire_compile_install_mutation(game_dir: &Path) -> Result<InstallMutationGuard, String> {
    acquire_compile_install_mutation_with_stated_game_process(
        game_dir,
        shipping_game_process_running,
    )
}

/// TEST-ONLY entry point: acquire the same guard with the one machine-global question answered by
/// the caller instead of by the machine the suite happens to run on.
///
/// Why a sibling rather than a parameter on [`acquire_compile_install_mutation`]: that function is
/// what every production call site uses, and its shape — a game directory in, a guard out, nobody
/// answering the process question for it — is a property a reader can check by looking at the call.
/// Widening it would let any of those call sites answer for the machine without saying so. Keeping
/// it parameterless means a caller that answers has to spell this name to do it, so the shipped
/// paths are the ones that cannot, and this name appears in test code only.
///
/// This exists because `gore-as`'s own fixtures reach the seam through `#[cfg(test)]`, and a crate
/// that links `gore-as` compiled without `cfg(test)` — `gore-ffi` — cannot. Its transaction tests
/// would otherwise inherit the developer's desktop: the same code passing or failing on whether
/// Gothic happens to be open, which is not what those tests are named for.
///
/// `check_game_process` replaces exactly one question and nothing else. Recovery artifacts, the
/// cross-tool install-mutation lock, the order they are checked in and every refusal message are
/// the same code path the default entry point runs.
#[doc(hidden)]
pub fn acquire_compile_install_mutation_with_stated_game_process<C>(
    game_dir: &Path,
    check_game_process: C,
) -> Result<InstallMutationGuard, String>
where
    C: FnOnce() -> Result<bool, String>,
{
    install_compile_preflight_with(game_dir, check_game_process)?;
    InstallMutationGuard::acquire(game_dir, "gore-as:compile")
}

fn install_compile_preflight_with<C>(game_dir: &Path, check_game_process: C) -> Result<(), String>
where
    C: FnOnce() -> Result<bool, String>,
{
    let probe = probe_install_compile_state_with(game_dir, check_game_process);
    match probe.disposition {
        InstallCompileStateDisposition::SafeToCompile => Ok(()),
        InstallCompileStateDisposition::GameProcessRunning => Err(format!(
            "refusing AngelScript compile while {G1R_SHIPPING_EXE_NAME} is running; close the game \
             and retry"
        )),
        InstallCompileStateDisposition::RecoveryArtifactsPresent => {
            let artifact = probe
                .artifacts
                .first()
                .expect("recovery-artifact disposition requires an artifact");
            let message = match artifact.kind {
                InstallCompileArtifactKind::InstallMutationLock => format!(
                    "another install mutation is active (lock already exists: {}); inspect its \
                     bounded owner/pid record before retrying",
                    artifact.path
                ),
                InstallCompileArtifactKind::CompileLock => format!(
                    "another AngelScript compile is active (compile lock already exists: {}); if \
                     no compile is running, inspect the stale lock and recovery state manually",
                    artifact.path
                ),
                InstallCompileArtifactKind::RecoveryJournal => format!(
                    "compile recovery journal already exists: {} (recover the previous compile \
                     before retrying)",
                    artifact.path
                ),
                InstallCompileArtifactKind::ShippingCacheBackup
                | InstallCompileArtifactKind::JittedCodeBackup
                | InstallCompileArtifactKind::Ue4ssProxyBackup => format!(
                    "compile backup already exists: {} (recover or remove it manually before \
                     retrying)",
                    artifact.path
                ),
            };
            let (message, _) = bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);
            Err(message)
        }
        InstallCompileStateDisposition::InspectionFailed => {
            let issue = probe
                .issues
                .first()
                .expect("inspection-failed disposition requires an issue");
            let message = match &issue.path {
                Some(path) => format!(
                    "refusing AngelScript compile because install-state inspection failed for \
                     {path}: {}",
                    issue.message
                ),
                None => format!(
                    "refusing AngelScript compile because native game-process inspection failed: \
                     {}",
                    issue.message
                ),
            };
            let (message, _) = bounded_probe_text(&message, INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);
            Err(message)
        }
    }
}

#[derive(Debug)]
struct CompileLock {
    path: PathBuf,
    active: bool,
}

impl CompileLock {
    fn acquire(game_dir: &Path) -> Result<Self, String> {
        let path = compile_lock_path(game_dir);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "another AngelScript compile is active (lock exists: {}); if no compile is \
                         running, inspect and remove the stale lock manually",
                        path.display()
                    )
                } else {
                    format!("creating compile lock {}: {e}", path.display())
                }
            })?;
        let payload = format!("pid={}\n", std::process::id());
        if let Err(e) = file
            .write_all(payload.as_bytes())
            .and_then(|_| file.sync_all())
        {
            drop(file);
            let cleanup = std::fs::remove_file(&path).err();
            return Err(match cleanup {
                Some(cleanup) => format!(
                    "initializing compile lock {}: {e}; additionally failed to remove it: \
                     {cleanup}",
                    path.display()
                ),
                None => format!("initializing compile lock {}: {e}", path.display()),
            });
        }
        Ok(Self { path, active: true })
    }

    fn release(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        std::fs::remove_file(&self.path)
            .map_err(|e| format!("removing compile lock {}: {e}", self.path.display()))?;
        self.active = false;
        Ok(())
    }

    fn preserve_for_manual_recovery(&mut self) {
        self.active = false;
    }
}

impl Drop for CompileLock {
    fn drop(&mut self) {
        if self.active && std::fs::remove_file(&self.path).is_ok() {
            self.active = false;
        }
    }
}

#[derive(Debug)]
struct ShippingRecovery {
    path: PathBuf,
    active: bool,
}

impl ShippingRecovery {
    fn create(live: &Path, bytes: &[u8]) -> Result<Self, String> {
        let path = compile_bak_path(live);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "compile backup already exists: {} (recover or remove it manually)",
                        path.display()
                    )
                } else {
                    format!("creating compile backup {}: {e}", path.display())
                }
            })?;
        if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let cleanup = std::fs::remove_file(&path).err();
            return Err(match cleanup {
                Some(cleanup) => format!(
                    "initializing compile backup {}: {e}; additionally failed to remove the \
                     incomplete backup: {cleanup}",
                    path.display()
                ),
                None => format!("initializing compile backup {}: {e}", path.display()),
            });
        }
        drop(file);
        Ok(Self { path, active: true })
    }

    fn retire(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        std::fs::remove_file(&self.path)
            .map_err(|e| format!("removing compile backup {}: {e}", self.path.display()))?;
        self.active = false;
        Ok(())
    }

    fn preserve_for_manual_recovery(&mut self) {
        self.active = false;
    }
}

/// Create the user-requested persistent `.gore-bak` immediately before an in-place install.
/// Returns true only when this call created it, so a later failed install can remove the artifact.
fn validate_existing_deploy_backup(path: &Path, meta: &std::fs::Metadata) -> Result<(), String> {
    if !meta.is_file() || metadata_is_link_or_reparse(meta) {
        return Err(format!(
            "refusing existing deploy backup {} because it is not a regular non-reparse file",
            path.display()
        ));
    }
    Ok(())
}

fn create_deploy_backup_if_absent(live: &Path, bytes: &[u8]) -> Result<bool, String> {
    let path = deploy_bak_path(live);
    match std::fs::symlink_metadata(&path) {
        Ok(meta) => {
            validate_existing_deploy_backup(&path, &meta)?;
            return Ok(false);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("inspecting deploy backup {}: {e}", path.display())),
    }
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another actor created the reserved path after the first inspection. Never accept its
            // path type implicitly: re-inspect without following links before treating it as the
            // persistent recovery copy.
            let meta = std::fs::symlink_metadata(&path).map_err(|inspect| {
                format!(
                    "inspecting raced deploy backup {}: {inspect}",
                    path.display()
                )
            })?;
            validate_existing_deploy_backup(&path, &meta)?;
            return Ok(false);
        }
        Err(e) => return Err(format!("creating deploy backup {}: {e}", path.display())),
    };
    if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let cleanup = std::fs::remove_file(&path).err();
        return Err(match cleanup {
            Some(cleanup) => format!(
                "initializing deploy backup {}: {e}; additionally failed to remove the incomplete \
                 backup: {cleanup}",
                path.display()
            ),
            None => format!("initializing deploy backup {}: {e}", path.display()),
        });
    }
    Ok(true)
}

/// Validate the whole generated container before any caller accepts or installs its bytes. A
/// header-only or module-only prefix is not sufficient: all declared modules and all seven global
/// tail tables must parse, and the final table must end exactly at EOF.
fn validate_generated_cache(bytes: &[u8]) -> Result<(), String> {
    let header = crate::cache::header::CacheHeader::parse(bytes)
        .map_err(|e| format!("invalid generated cache header: {e}"))?;
    if header.type_count == 0 {
        return Err("invalid generated cache: it declares zero modules".into());
    }
    let tail = crate::cache::walk_modules::module_region_end(bytes)
        .map_err(|e| format!("invalid generated cache modules: {e}"))?;
    let tables = crate::cache::tables::parse_tail_tables(bytes, tail)
        .map_err(|e| format!("invalid generated cache tail tables: {e}"))?;
    if tables.end != bytes.len() {
        return Err(format!(
            "invalid generated cache: tail tables end at {:#x}, but file length is {:#x}",
            tables.end,
            bytes.len()
        ));
    }
    Ok(())
}

/// Minis are intermediate module containers; every add/replace splice path publishes the base
/// cache's outer header, never the mini's. Normalize the per-regeneration FGuid anyway so identical
/// source/base inputs produce byte-identical mini artifacts across compiler runs.
fn canonicalize_mini_guid(mini: &mut [u8], base: &[u8]) -> Result<(), String> {
    const GUID_BYTES: usize = 16;
    if mini.len() < GUID_BYTES || base.len() < GUID_BYTES {
        return Err(format!(
            "cannot canonicalize mini FGuid: mini/base shorter than {GUID_BYTES} bytes ({}/{})",
            mini.len(),
            base.len()
        ));
    }
    mini[..GUID_BYTES].copy_from_slice(&base[..GUID_BYTES]);
    Ok(())
}

/// Recreate the fixed `work_dir/tree` child from scratch. Refuse links and containment surprises
/// before recursive deletion, so a hostile/stale tree cannot redirect cleanup outside work_dir.
fn reset_compile_tree(work_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(work_dir)
        .map_err(|e| format!("creating compile work dir {}: {e}", work_dir.display()))?;
    let work_real = work_dir
        .canonicalize()
        .map_err(|e| format!("resolving compile work dir {}: {e}", work_dir.display()))?;
    let tree = work_dir.join("tree");

    match std::fs::symlink_metadata(&tree) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err(format!(
                    "refusing to clear compile tree {} because it is not a real directory",
                    tree.display()
                ));
            }
            let tree_real = tree
                .canonicalize()
                .map_err(|e| format!("resolving compile tree {}: {e}", tree.display()))?;
            if tree_real == work_real || !tree_real.starts_with(&work_real) {
                return Err(format!(
                    "refusing to clear compile tree {} outside work dir {}",
                    tree_real.display(),
                    work_real.display()
                ));
            }
            std::fs::remove_dir_all(&tree_real)
                .map_err(|e| format!("clearing compile tree {}: {e}", tree_real.display()))?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("inspecting compile tree {}: {e}", tree.display())),
    }

    std::fs::create_dir(&tree)
        .map_err(|e| format!("creating clean compile tree {}: {e}", tree.display()))?;
    let tree_real = tree
        .canonicalize()
        .map_err(|e| format!("resolving clean compile tree {}: {e}", tree.display()))?;
    if tree_real.parent() != Some(work_real.as_path()) {
        return Err(format!(
            "clean compile tree {} is not a direct child of work dir {}",
            tree_real.display(),
            work_real.display()
        ));
    }
    Ok(tree)
}

/// Snapshot a file that may legitimately be absent. Generation writes
/// `PrecompiledScript.Cache`, and a developer may already have one there; callers must put that
/// exact prior state back instead of leaving the newly generated development cache installed.
fn snapshot_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("reading {}: {e}", path.display())),
    }
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("removing stale {}: {e}", path.display())),
    }
}

/// Restore an optional file snapshot exactly: rewrite the old bytes when it existed, otherwise
/// remove whatever generation created.
fn restore_optional(path: &Path, saved: &Option<Vec<u8>>) -> Result<(), String> {
    match saved {
        Some(bytes) => {
            std::fs::write(path, bytes).map_err(|e| format!("restoring {}: {e}", path.display()))
        }
        None => remove_if_exists(path).map_err(|e| format!("restoring absent file: {e}")),
    }
}

fn remove_stale_compiled_artifact(path: &Path) -> Result<(), CompileError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CompileError::Io(format!(
            "inspecting stale mini-cache output: {error}"
        ))),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Err(
            CompileError::Io("stale mini-cache output is a directory".to_owned()),
        ),
        Ok(_) => std::fs::remove_file(path).map_err(|error| {
            CompileError::Io(format!("removing stale mini-cache output: {error}"))
        }),
    }
}

fn write_compiled_artifact(path: PathBuf, bytes: &[u8]) -> Result<CompiledArtifact, CompileError> {
    remove_stale_compiled_artifact(&path)?;
    let file = open_compiled_artifact_create_new(&path).map_err(|message| {
        CompileError::Io(format!("creating retained mini-cache output: {message}"))
    })?;
    let mut artifact = CompiledArtifact { path, file };
    if let Err(error) = artifact
        .file
        .write_all(bytes)
        .and_then(|_| artifact.file.sync_all())
    {
        let cleanup = artifact
            .neutralize()
            .err()
            .map(|failure| format!("; exact-handle cleanup also failed: {failure}"))
            .unwrap_or_default();
        return Err(CompileError::ArtifactIo {
            message: format!("writing retained mini-cache output: {error}{cleanup}"),
            artifact: Some(FailedCompiledArtifact { artifact }),
        });
    }
    Ok(artifact)
}

#[cfg(windows)]
fn open_compiled_artifact_create_new(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|error| format!("opening create-new/no-follow output: {error}"))?;
    validate_opened_compiled_artifact(file)
}

#[cfg(windows)]
fn open_compiled_artifact_existing(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|error| format!("opening existing no-follow output: {error}"))?;
    validate_opened_compiled_artifact(file)
}

#[cfg(windows)]
fn open_regular_file_no_follow_read(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        // Deny write/delete sharing while the bounded snapshot is read. The generator process has
        // already exited, so a conflicting handle is uncertainty rather than a usable result.
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|error| format!("opening bounded no-follow input: {error}"))?;
    validate_opened_compiled_artifact(file)
}

#[cfg(windows)]
fn validate_opened_compiled_artifact(file: std::fs::File) -> Result<std::fs::File, String> {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    let metadata = file
        .metadata()
        .map_err(|error| format!("inspecting retained output handle: {error}"))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err("retained output is not a regular non-reparse file".to_owned());
    }
    Ok(file)
}

#[cfg(unix)]
fn open_compiled_artifact_create_new(path: &Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|error| format!("opening create-new/no-follow output: {error}"))?;
    validate_opened_compiled_artifact(file)
}

#[cfg(unix)]
fn open_compiled_artifact_existing(path: &Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|error| format!("opening existing no-follow output: {error}"))?;
    validate_opened_compiled_artifact(file)
}

#[cfg(unix)]
fn open_regular_file_no_follow_read(path: &Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|error| format!("opening bounded no-follow input: {error}"))?;
    validate_opened_compiled_artifact(file)
}

#[cfg(unix)]
fn validate_opened_compiled_artifact(file: std::fs::File) -> Result<std::fs::File, String> {
    if !file
        .metadata()
        .map_err(|error| format!("inspecting retained output handle: {error}"))?
        .is_file()
    {
        return Err("retained output is not a regular file".to_owned());
    }
    Ok(file)
}

/// `run_regen(game_dir, src_dir) -> regen cache path`. Injected so the orchestration is testable
/// offline; the FFI passes [`game_run_regen`].
pub fn compile_module<R>(opts: &CompileOpts, run_regen: R) -> Result<CompileOutput, CompileError>
where
    R: FnOnce(&Path, &Path) -> Result<PathBuf, String>,
{
    if opts.op != "add" && opts.op != "edit" {
        return Err(CompileError::Other(format!(
            "invalid script op {:?} for module {:?} (want \"add\" or \"edit\")",
            opts.op, opts.module_name
        )));
    }
    // Read the overlay before clearing work_dir/tree. This also makes an input that intentionally
    // lives below that old tree safe: its bytes survive the clean rebuild, never its stale siblings.
    // Managed callers instead supply an already sealed byte snapshot and never reopen a
    // caller-controlled source pathname.
    let overlay = match &opts.source_override {
        Some(bytes) => bytes.clone(),
        None => {
            if !opts.as_path.exists() {
                return Err(CompileError::Io(format!(
                    "source .as not found: {}",
                    opts.as_path.display()
                )));
            }
            std::fs::read(&opts.as_path).map_err(io("reading source .as"))?
        }
    };
    // The PRISTINE base cache to emit/remap against. Prefer the caller-supplied `base_override`
    // (the FFI passes gore-mod's drift-aware `pristine_script_cache`, so the base matches exactly
    // what deploy will splice against, even after a game update made the `*.gore-bak` stale).
    // Without an override, fall back to the on-disk read: if a mod is already deployed, the live
    // cache is the spliced (modded) one and gore-mod's deploy backup `…Cache.gore-bak` holds the
    // true pristine bytes, so prefer the backup when present. `base_path` is the on-disk cache
    // location used only to locate `Binds.Cache` next to it — independent of which bytes `base` holds.
    let live_cache = vanilla_cache(&opts.game_dir);
    let bak = deploy_bak_path(&live_cache);
    let base_path = if bak.exists() { bak } else { live_cache };
    let base = match &opts.base_override {
        Some(bytes) => bytes.clone(),
        None => std::fs::read(&base_path).map_err(io("reading vanilla cache"))?,
    };

    let mut refs =
        RefResolver::build(&base).map_err(|e| CompileError::Other(format!("resolver: {e}")))?;
    let mods =
        model::parse_modules(&base).map_err(|e| CompileError::Other(format!("parse: {e}")))?;
    // Use the exact same resolver preparation as `as emit-all`. Class fields, method-shadow names,
    // and id-based free-function collision renames are all compile-significant; the old partial
    // setup produced 287 divergent vanilla files on the 1.0.3 cache before the authored overlay
    // was even considered.
    let native_api = match &opts.binds_override {
        Some(bytes) => Some(
            crate::cache::binds::NativeApi::from_bytes(bytes).ok_or_else(|| {
                CompileError::Other("sealed Binds.Cache override is invalid".to_owned())
            })?,
        ),
        None => native_api(&base_path),
    };
    let overlay = std::str::from_utf8(&overlay)
        .map_err(|error| CompileError::Other(format!("source .as is not valid UTF-8: {error}")))?;
    // The baseline tree has to speak the same dialect as the overlay.
    //
    // A defaults-free overlay keeps the historical, qualified baseline: the tree exists to
    // satisfy the compiler's dependency graph, not to be read, and regenerating 30k
    // `__InitDefaults` for nothing would only churn the regen cache.
    //
    // An overlay that AUTHORS defaults needs the opposite. Class defaults are where most string
    // literals and `FName`s in the game live, so a defaults-free baseline declares a materially
    // different symbol landscape than vanilla: a literal vanilla spells out in two modules
    // appears once, and the ref remap then reports it as ambiguous or unresolvable through no
    // fault of the edit. Emitting the baseline with defaults restores the correspondence.
    let baseline_defaults = source_contains_default_token(overlay)
        .map_err(|reason| CompileError::Other(format!("preparing authored overlay: {reason}")))?;
    let prepared = emit_all::PreparedEmit::new(&mods, &mut refs, native_api)
        .map_err(|error| CompileError::Other(format!("preparing base modules: {error}")))?
        .with_class_defaults(baseline_defaults);
    let (overlay, overlay_rel_path) = prepared
        .prepare_compile_overlay(&opts.op, &opts.module_name, &opts.rel_path, overlay)
        .map_err(|error| CompileError::Other(format!("preparing authored overlay: {error}")))?;

    let generated_defaults = prepare_generated_defaults_edit(
        &opts.op,
        &mods,
        &opts.module_name,
        &base,
        &overlay,
        opts.allow_new_symbols,
    )?;

    // 1. Only after all base and authored target checks succeed, clear and rebuild the tree.
    let tree = reset_compile_tree(&opts.work_dir).map_err(CompileError::Other)?;
    prepared
        .emit_tree(&tree)
        .map_err(|e| CompileError::Other(format!("emit tree: {e}")))?;

    // 2. Overlay the user's .as at its rel path.
    let dst = tree.join(&overlay_rel_path);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(io("mkdir overlay"))?;
    }
    std::fs::write(&dst, overlay.as_bytes()).map_err(io("overlay .as"))?;

    // 3. Drive the game to regenerate the precompiled cache from `tree`.
    let regen_path = run_regen(&opts.game_dir, &tree).map_err(CompileError::Regen)?;
    if !regen_path.exists() {
        return Err(CompileError::NoRegen(regen_path.display().to_string()));
    }
    let regen = std::fs::read(&regen_path).map_err(io("reading regen cache"))?;
    validate_generated_cache(&regen).map_err(CompileError::Other)?;

    // For "add", the game names the new module from its ScriptRelativeFilename, which may differ
    // from `opts.module_name`. Resolve the real name = the single module present in the regen but
    // not in the base; fall back to `opts.module_name` if that diff isn't exactly one (the existing
    // extract call then surfaces any mismatch naturally). "edit" keeps `opts.module_name`.
    let target = if opts.op == "add" {
        match (
            crate::cache::walk_modules::module_names(&base),
            crate::cache::walk_modules::module_names(&regen),
        ) {
            (Ok(base_names), Ok(regen_names)) => {
                use std::collections::HashSet;
                let base_set: HashSet<&str> = base_names.iter().map(String::as_str).collect();
                let mut added = regen_names
                    .iter()
                    .filter(|n| !base_set.contains(n.as_str()));
                match (added.next(), added.next()) {
                    (Some(only), None) => only.clone(),
                    _ => opts.module_name.clone(),
                }
            }
            _ => opts.module_name.clone(),
        }
    } else {
        opts.module_name.clone()
    };

    // 4. Extract + remap the target module against the vanilla base, for BOTH ops. Strict mode
    //    emits the historical empty-tail mini. Explicit new-symbol mode instead carries only the
    //    new rows that cannot resolve in vanilla; it never copies the regen's full global tables.
    //    Deploy still differs by op — gore-mod uses `splice_auto` for add and `replace_module` for
    //    edit — while both accept either minimal shape.
    let mut mini = {
        let out = splice::extract_module(&regen, &target)
            .map_err(|e| CompileError::Other(format!("extract: {e}")))?;
        remap::remap_module_to_base_with_options(
            &out,
            &base,
            remap::RemapOptions {
                allow_new_symbols: opts.allow_new_symbols,
            },
        )
        .map_err(|e| CompileError::Other(format!("remap: {e}")))?
        .0
    };
    if let Some(plan) = generated_defaults {
        mini = plan.apply(&mini).map_err(|reason| {
            CompileError::Other(format!(
                "refusing generated-default carry for edit module {:?}: {reason}",
                opts.module_name
            ))
        })?;
    }
    canonicalize_mini_guid(&mut mini, &base).map_err(CompileError::Other)?;

    let mini_path = opts.work_dir.join("module.cache");
    let artifact = write_compiled_artifact(mini_path.clone(), &mini)?;
    Ok(CompileOutput::retained(mini_path, target, artifact))
}

/// Load native arities from the `GORE_AS_BINDS` env path if set, else a `Binds.Cache` sitting next
/// to `cache_file`, if present. Mirrors `as_cache.rs::load_native_api` / gore-ffi's `as_native_api`
/// so a dev who sets `GORE_AS_BINDS` for the CLI gets the same arities here (no emit/recompile
/// divergence). Quiet by design (library helper — no logging). Absent/unparsable => None.
fn native_api(cache_file: &Path) -> Option<crate::cache::binds::NativeApi> {
    let path = match std::env::var_os("GORE_AS_BINDS") {
        Some(p) => std::path::PathBuf::from(p),
        None => cache_file.parent()?.join("Binds.Cache"),
    };
    if !path.exists() {
        return None;
    }
    crate::cache::binds::NativeApi::load(&path)
}

#[derive(Clone, Copy, Debug)]
enum QuarantineKind {
    File,
    Directory,
}

/// One path whose exact pre-launch presence is restored after generation. Existing content is
/// moved with a same-volume rename; absent content is kept absent by removing anything the game
/// creates at that exact path.
#[derive(Debug)]
struct QuarantinedPath {
    original: PathBuf,
    backup: PathBuf,
    kind: QuarantineKind,
    existed: bool,
    active: bool,
}

impl QuarantinedPath {
    /// Preflight only. In particular, check the reserved backup before ANY path is moved so an
    /// interrupted earlier compile is never overwritten or mistaken for disposable output.
    fn plan(original: PathBuf, backup: PathBuf, kind: QuarantineKind) -> Result<Self, String> {
        match std::fs::symlink_metadata(&backup) {
            Ok(_) => {
                return Err(format!(
                    "compile quarantine backup already exists: {} (recover or remove it manually)",
                    backup.display()
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("inspecting backup {}: {e}", backup.display())),
        }
        let existed = match std::fs::symlink_metadata(&original) {
            Ok(meta) => {
                let expected = match kind {
                    QuarantineKind::File => meta.is_file() && !meta.file_type().is_symlink(),
                    QuarantineKind::Directory => meta.is_dir() && !meta.file_type().is_symlink(),
                };
                if !expected {
                    return Err(format!(
                        "refusing to quarantine {} because it is not a real {}",
                        original.display(),
                        match kind {
                            QuarantineKind::File => "file",
                            QuarantineKind::Directory => "directory",
                        }
                    ));
                }
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("inspecting {}: {e}", original.display())),
        };
        Ok(Self {
            original,
            backup,
            kind,
            existed,
            active: false,
        })
    }

    fn activate(&mut self) -> Result<(), String> {
        if self.existed {
            std::fs::rename(&self.original, &self.backup).map_err(|e| {
                format!(
                    "quarantining {} as {}: {e}",
                    self.original.display(),
                    self.backup.display()
                )
            })?;
        }
        self.active = true;
        Ok(())
    }

    fn remove_generated_original(&self) -> Result<(), String> {
        let meta = match std::fs::symlink_metadata(&self.original) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("inspecting {}: {e}", self.original.display())),
        };
        if meta.file_type().is_symlink() {
            return Err(format!(
                "refusing to remove unexpected symlink at generated path {}",
                self.original.display()
            ));
        }
        match self.kind {
            QuarantineKind::File if meta.is_file() => std::fs::remove_file(&self.original),
            QuarantineKind::Directory if meta.is_dir() => std::fs::remove_dir_all(&self.original),
            QuarantineKind::File => {
                return Err(format!(
                    "expected generated file at {}, found another path type",
                    self.original.display()
                ));
            }
            QuarantineKind::Directory => {
                return Err(format!(
                    "expected generated directory at {}, found another path type",
                    self.original.display()
                ));
            }
        }
        .map_err(|e| format!("removing generated {}: {e}", self.original.display()))
    }

    fn restore(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        // If removal fails, do not touch the backup: it remains the recoverable pre-call state.
        self.remove_generated_original()?;
        if self.existed {
            std::fs::rename(&self.backup, &self.original).map_err(|e| {
                format!(
                    "restoring {} from {}: {e}",
                    self.original.display(),
                    self.backup.display()
                )
            })?;
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for QuarantinedPath {
    fn drop(&mut self) {
        // Explicit restoration remains the normal/reporting path. This is only the unwind safety
        // net after the owning transaction has already had a chance to retain recovery artifacts.
        let _ = self.restore();
    }
}

/// Generation isolation for the two known non-cache side effects:
/// - `<install>/AS_JITTED_CODE`, written by AngelScript;
/// - the UE4SS `dwmapi.dll` loader proxy, temporarily moved so generation runs without hooks.
struct GenerationIsolation {
    jitted: QuarantinedPath,
    proxy: Option<QuarantinedPath>,
}

impl GenerationIsolation {
    /// Plan both quarantines without mutating either path. The transaction stores this owner before
    /// activation, so even a partial activation remains visible to its reporting restore path.
    fn plan(game_dir: &Path, g1r: &Path) -> Result<Self, String> {
        let jitted = game_root_dir(game_dir).join("AS_JITTED_CODE");
        let jitted = QuarantinedPath::plan(
            jitted.clone(),
            append_suffix(&jitted, ".gore-compile-bak"),
            QuarantineKind::Directory,
        )?;

        let win64 = g1r.join("Binaries").join("Win64");
        let proxy_path = win64.join("dwmapi.dll");
        let proxy_exists = match std::fs::symlink_metadata(&proxy_path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("inspecting {}: {e}", proxy_path.display())),
        };
        let ue4ss_payload = win64.join("ue4ss").join("UE4SS.dll");
        let clearly_ue4ss = ue4ss_payload.is_file();
        // A present dwmapi.dll without the UE4SS payload is not clearly the local proxy; leave it
        // entirely alone. An absent path is still tracked so it remains absent after generation.
        let proxy = if !proxy_exists || clearly_ue4ss {
            Some(QuarantinedPath::plan(
                proxy_path.clone(),
                append_suffix(&proxy_path, ".gore-compile-bak"),
                QuarantineKind::File,
            )?)
        } else {
            None
        };

        Ok(Self { jitted, proxy })
    }

    fn activate(&mut self) -> Result<(), String> {
        self.activate_after_jitted(|| {})
    }

    fn activate_after_jitted<F>(&mut self, after_jitted: F) -> Result<(), String>
    where
        F: FnOnce(),
    {
        // Both plans (including collision checks) completed before this first rename. Do not
        // locally roll back a partial activation: the transaction owns `self` and must report any
        // failed restoration before deciding whether its journal/backups can be retired.
        self.jitted.activate()?;
        after_jitted();
        if let Some(proxy) = &mut self.proxy {
            proxy.activate()?;
        }
        Ok(())
    }

    fn restore(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();
        if let Some(proxy) = &mut self.proxy {
            if let Err(e) = proxy.restore() {
                errors.push(e);
            }
        }
        if let Err(e) = self.jitted.restore() {
            errors.push(e);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

impl Drop for GenerationIsolation {
    fn drop(&mut self) {
        // Normal paths call restore explicitly so errors are reported. This fallback covers a
        // panic/unwind and deliberately leaves any un-restorable backups in place.
        let _ = self.restore();
    }
}

struct RestoreReport {
    errors: Vec<String>,
    shipping_restored: bool,
}

impl RestoreReport {
    fn clean(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug)]
struct CompileTransactionBeginFailure {
    message: String,
    recovery_required: bool,
}

/// Finish a failure that happened after shared ownership was transferred but before a complete
/// `CompileTransaction` existed. Release is explicit so a guard Drop can never hide its failure.
fn finalize_compile_transaction_begin_failure(
    game_dir: &Path,
    mut mutation_guard: InstallMutationGuard,
    mut errors: Vec<String>,
) -> CompileTransactionBeginFailure {
    let mut recovery_required = false;
    if let Err(error) = mutation_guard.release() {
        errors.push(format!(
            "failed to release the pre-transaction install-mutation guard: {error}"
        ));
        // The failed release itself is uncertain and therefore recovery-dominant. Do not retry in
        // Drop: keeping the ownership record is the durable blocker when the path still exists.
        mutation_guard.preserve_for_manual_recovery();
        recovery_required = true;
    }
    for (_, path) in install_compile_artifact_paths(game_dir) {
        match std::fs::symlink_metadata(&path) {
            Ok(_) => recovery_required = true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                errors.push(format!(
                    "failed to inspect recovery artifact {} after setup failure: {error}",
                    path.display()
                ));
                recovery_required = true;
            }
        }
    }
    CompileTransactionBeginFailure {
        message: errors.join("; additionally "),
        recovery_required,
    }
}

fn recovery_journal_path(game_dir: &Path) -> PathBuf {
    game_root_dir(game_dir).join(".gore-as-compile-recovery")
}

/// Disk-backed copies of every in-memory snapshot needed to recover an intentionally-preserved
/// transaction after a generator process could not be confirmed dead. The mirrored layout is
/// deliberately human-readable: `overwritten/` files copy back into `G1R/Script/`, paths mirrored
/// under `created/` must be deleted, and `development-cache/` records the pre-call dev-cache state.
struct RecoveryJournal {
    root: PathBuf,
    active: bool,
}

impl RecoveryJournal {
    fn create(game_dir: &Path, saved_dev: &Option<Vec<u8>>) -> Result<Self, String> {
        let root = recovery_journal_path(game_dir);
        match std::fs::create_dir(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(format!(
                    "compile recovery journal already exists: {} (recover the previous compile \
                     before retrying)",
                    root.display()
                ));
            }
            Err(e) => return Err(format!("creating recovery journal {}: {e}", root.display())),
        }

        let initialize = (|| -> Result<(), String> {
            let instructions = b"GORE AngelScript compile recovery\n\
Kill the reported generator process tree before restoring anything.\n\
Copy files from overwritten/ over the same relative paths under G1R/Script/.\n\
Delete the same relative paths listed as zero-byte files under created/.\n\
development-cache/PrecompiledScript.Cache is the pre-call dev cache;\n\
development-cache.absent means that cache did not exist.\n\
Restore *.gore-compile-bak paths beside their originals.\n\
Only after the process is dead and every path is restored, remove .gore-as-compile.lock and\n\
.gore-install-mutation.lock from the install root, then remove this\n\
.gore-as-compile-recovery directory. Never remove a lock owned by a live task.\n";
            std::fs::write(root.join("README.txt"), instructions)
                .map_err(|e| format!("writing recovery instructions: {e}"))?;
            match saved_dev {
                Some(bytes) => {
                    let dev_dir = root.join("development-cache");
                    std::fs::create_dir(&dev_dir)
                        .map_err(|e| format!("creating dev-cache recovery directory: {e}"))?;
                    std::fs::write(dev_dir.join("PrecompiledScript.Cache"), bytes)
                        .map_err(|e| format!("writing dev-cache recovery snapshot: {e}"))?;
                }
                None => {
                    std::fs::write(root.join("development-cache.absent"), b"")
                        .map_err(|e| format!("writing dev-cache absence marker: {e}"))?;
                }
            }
            Ok(())
        })();

        if let Err(error) = initialize {
            let cleanup = std::fs::remove_dir_all(&root).err();
            return Err(match cleanup {
                Some(cleanup) => format!(
                    "{error}; additionally failed to remove partial recovery journal {}: \
                     {cleanup}",
                    root.display()
                ),
                None => error,
            });
        }
        Ok(Self { root, active: true })
    }

    fn record_staged(
        &self,
        staged: &[(PathBuf, Option<Vec<u8>>)],
        script_dir: &Path,
    ) -> Result<(), String> {
        for (path, prior) in staged {
            let rel = path.strip_prefix(script_dir).map_err(|e| {
                format!(
                    "staged recovery path {} escaped Script/ {}: {e}",
                    path.display(),
                    script_dir.display()
                )
            })?;
            let bucket = if prior.is_some() {
                "overwritten"
            } else {
                "created"
            };
            let recovery = self.root.join(bucket).join(rel);
            if let Some(parent) = recovery.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("creating recovery directory {}: {e}", parent.display())
                })?;
            }
            std::fs::write(&recovery, prior.as_deref().unwrap_or_default()).map_err(|e| {
                format!(
                    "writing staged recovery snapshot {}: {e}",
                    recovery.display()
                )
            })?;
        }
        Ok(())
    }

    fn retire(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        std::fs::remove_dir_all(&self.root)
            .map_err(|e| format!("removing recovery journal {}: {e}", self.root.display()))?;
        self.active = false;
        Ok(())
    }
}

/// Owns every live-install mutation made by either compile entry point. Its Drop implementation is
/// the panic/unwind safety net; normal paths call the same restore methods so cleanup errors remain
/// visible to the caller.
struct CompileTransaction {
    game_dir: PathBuf,
    g1r: PathBuf,
    script_dir: PathBuf,
    shipping_cache: PathBuf,
    dev_cache: PathBuf,
    saved_shipping: Vec<u8>,
    saved_dev: Option<Vec<u8>>,
    staged: Vec<(PathBuf, Option<Vec<u8>>)>,
    isolation: Option<GenerationIsolation>,
    recovery: ShippingRecovery,
    journal: RecoveryJournal,
    lock: CompileLock,
    mutation_guard: InstallMutationGuard,
    rollback_needed: bool,
    /// A user-facing `.gore-bak` created immediately before install, removed unless install commits.
    ephemeral_deploy_backup: Option<PathBuf>,
}

impl CompileTransaction {
    /// Complete the read-only, fail-closed preflight before creating either lock. The shared
    /// install-mutation guard is acquired before the gore-as-specific lock; the recovery backup is
    /// the next and only mutation before the fully-owned transaction exists.
    fn begin_with_process_checker<C>(
        game_dir: &Path,
        g1r: &Path,
        script_dir: &Path,
        check_game_process: C,
    ) -> Result<Self, String>
    where
        C: Fn() -> Result<bool, String>,
    {
        Self::begin_with_process_checker_report(game_dir, g1r, script_dir, check_game_process)
            .map_err(|failure| failure.message)
    }

    fn begin_with_process_checker_report<C>(
        game_dir: &Path,
        g1r: &Path,
        script_dir: &Path,
        check_game_process: C,
    ) -> Result<Self, CompileTransactionBeginFailure>
    where
        C: Fn() -> Result<bool, String>,
    {
        install_compile_preflight_with(game_dir, &check_game_process).map_err(|message| {
            CompileTransactionBeginFailure {
                message,
                recovery_required: false,
            }
        })?;
        let mutation_guard =
            InstallMutationGuard::acquire(game_dir, "gore-as:compile").map_err(|message| {
                CompileTransactionBeginFailure {
                    message,
                    recovery_required: false,
                }
            })?;
        Self::begin_with_mutation_guard_and_process_checker(
            game_dir,
            g1r,
            script_dir,
            mutation_guard,
            check_game_process,
        )
    }

    fn begin_with_mutation_guard_and_process_checker<C>(
        game_dir: &Path,
        g1r: &Path,
        script_dir: &Path,
        mutation_guard: InstallMutationGuard,
        check_game_process: C,
    ) -> Result<Self, CompileTransactionBeginFailure>
    where
        C: FnOnce() -> Result<bool, String>,
    {
        let expected_lock = install_mutation_lock_path(game_dir);
        if !mutation_guard.active
            || mutation_guard.owner != "gore-as:compile"
            || mutation_guard.path != expected_lock
        {
            return Err(finalize_compile_transaction_begin_failure(
                game_dir,
                mutation_guard,
                vec![
                    "pre-held install-mutation guard is not the active gore-as compiler guard for \
                     this game installation"
                        .to_owned(),
                ],
            ));
        }
        // This second process enumeration happens after ownership is held and immediately before
        // the compile lock/recovery files become the first live-install mutations. It narrows the
        // launch race but is not an OS barrier; see `require_shipping_game_process_closed`.
        if let Err(error) = require_shipping_game_process_closed_with(check_game_process) {
            return Err(finalize_compile_transaction_begin_failure(
                game_dir,
                mutation_guard,
                vec![error],
            ));
        }
        let mut lock = match CompileLock::acquire(game_dir) {
            Ok(lock) => lock,
            Err(error) => {
                return Err(finalize_compile_transaction_begin_failure(
                    game_dir,
                    mutation_guard,
                    vec![error],
                ));
            }
        };
        let shipping_cache = script_dir.join("PrecompiledScript_Shipping.Cache");
        let dev_cache = script_dir.join("PrecompiledScript.Cache");
        let saved_shipping = match std::fs::read(&shipping_cache) {
            Ok(bytes) => bytes,
            Err(error) => {
                let mut errors = vec![format!(
                    "reading live shipping cache {}: {e}",
                    shipping_cache.display(),
                    e = error
                )];
                if let Err(error) = lock.release() {
                    errors.push(error);
                    lock.preserve_for_manual_recovery();
                }
                return Err(finalize_compile_transaction_begin_failure(
                    game_dir,
                    mutation_guard,
                    errors,
                ));
            }
        };
        let saved_dev = match snapshot_optional(&dev_cache) {
            Ok(saved) => saved,
            Err(error) => {
                let mut errors = vec![error];
                if let Err(error) = lock.release() {
                    errors.push(error);
                    lock.preserve_for_manual_recovery();
                }
                return Err(finalize_compile_transaction_begin_failure(
                    game_dir,
                    mutation_guard,
                    errors,
                ));
            }
        };
        let mut recovery = match ShippingRecovery::create(&shipping_cache, &saved_shipping) {
            Ok(recovery) => recovery,
            Err(error) => {
                let mut errors = vec![error];
                if let Err(error) = lock.release() {
                    errors.push(error);
                    lock.preserve_for_manual_recovery();
                }
                return Err(finalize_compile_transaction_begin_failure(
                    game_dir,
                    mutation_guard,
                    errors,
                ));
            }
        };
        let journal = match RecoveryJournal::create(game_dir, &saved_dev) {
            Ok(journal) => journal,
            Err(error) => {
                let mut errors = vec![error];
                if let Err(error) = recovery.retire() {
                    errors.push(error);
                    recovery.preserve_for_manual_recovery();
                }
                if let Err(error) = lock.release() {
                    errors.push(error);
                    lock.preserve_for_manual_recovery();
                }
                return Err(finalize_compile_transaction_begin_failure(
                    game_dir,
                    mutation_guard,
                    errors,
                ));
            }
        };
        Ok(Self {
            game_dir: game_dir.to_path_buf(),
            g1r: g1r.to_path_buf(),
            script_dir: script_dir.to_path_buf(),
            shipping_cache,
            dev_cache,
            saved_shipping,
            saved_dev,
            staged: Vec::new(),
            isolation: None,
            recovery,
            journal,
            lock,
            mutation_guard,
            rollback_needed: true,
            ephemeral_deploy_backup: None,
        })
    }

    fn begin_isolation(&mut self) -> Result<(), String> {
        if self.isolation.is_none() {
            self.isolation = Some(GenerationIsolation::plan(&self.game_dir, &self.g1r)?);
        }
        self.isolation
            .as_mut()
            .expect("generation isolation was planned above")
            .activate()
    }

    #[cfg(test)]
    fn begin_isolation_after_jitted<F>(&mut self, after_jitted: F) -> Result<(), String>
    where
        F: FnOnce(),
    {
        if self.isolation.is_none() {
            self.isolation = Some(GenerationIsolation::plan(&self.game_dir, &self.g1r)?);
        }
        self.isolation
            .as_mut()
            .expect("generation isolation was planned above")
            .activate_after_jitted(after_jitted)
    }

    fn stage(&mut self, src: &Path) -> Result<(), String> {
        copy_tree(src, &self.script_dir, &mut self.staged)
            .map_err(|e| format!("staging source tree: {e}"))?;
        self.journal.record_staged(&self.staged, &self.script_dir)
    }

    /// Restore all live paths. A completely clean restore disarms the rollback portion of Drop,
    /// while recovery-backup retirement and lock release remain explicit finalization steps.
    fn restore_install(&mut self) -> RestoreReport {
        if !self.rollback_needed {
            return RestoreReport {
                errors: Vec::new(),
                shipping_restored: true,
            };
        }
        let mut errors = Vec::new();
        if let Some(isolation) = &mut self.isolation {
            if let Err(e) = isolation.restore() {
                errors.push(format!("failed to restore generation isolation: {e}"));
            }
        }
        if !self.staged.is_empty() {
            match restore_or_remove(&self.staged, &self.script_dir) {
                Ok(()) => self.staged.clear(),
                Err(e) => errors.push(format!("failed to clean staged sources: {e}")),
            }
        }
        let shipping_restored = match std::fs::write(&self.shipping_cache, &self.saved_shipping) {
            Ok(()) => true,
            Err(e) => {
                errors.push(format!(
                    "FAILED to restore the live shipping cache ({e}); restore it from {}",
                    self.recovery.path.display()
                ));
                false
            }
        };
        if let Err(e) = restore_optional(&self.dev_cache, &self.saved_dev) {
            errors.push(format!("failed to restore development cache: {e}"));
        }
        if errors.is_empty() {
            self.rollback_needed = false;
        }
        RestoreReport {
            errors,
            shipping_restored,
        }
    }

    /// Call immediately before an intentional in-place Shipping write, so a panic or partial write
    /// re-arms Drop's rollback behavior.
    fn arm_install_rollback(&mut self) {
        self.rollback_needed = true;
    }

    fn prepare_deploy_backup(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && create_deploy_backup_if_absent(&self.shipping_cache, &self.saved_shipping)? {
            self.ephemeral_deploy_backup = Some(deploy_bak_path(&self.shipping_cache));
        }
        Ok(())
    }

    fn remove_ephemeral_deploy_backup(&mut self) -> Result<(), String> {
        let Some(path) = self.ephemeral_deploy_backup.take() else {
            return Ok(());
        };
        if let Err(e) = std::fs::remove_file(&path) {
            self.ephemeral_deploy_backup = Some(path.clone());
            return Err(format!(
                "removing deploy backup created by failed compile {}: {e}",
                path.display()
            ));
        }
        Ok(())
    }

    fn mark_install_committed(&mut self) {
        self.rollback_needed = false;
        // The requested deploy backup is now persistent rather than transactional.
        self.ephemeral_deploy_backup = None;
    }

    /// Retire the Shipping backup after exact restore, but retain the recovery journal as a durable
    /// retry blocker until both locks have been released. A shared-lock release failure therefore
    /// remains observable after restart instead of existing only in this guard's in-memory state.
    fn finish(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.rollback_needed {
            errors.push("internal error: compile transaction finalized before restore".into());
            return errors;
        }
        if let Err(e) = self.recovery.retire() {
            errors.push(e);
            return errors; // keep journal + both locks until Drop/recovery
        }
        if let Err(e) = self.lock.release() {
            errors.push(e);
            return errors;
        }
        if let Err(e) = self.mutation_guard.release() {
            errors.push(e);
            return errors; // keep the journal as the persistent retry blocker
        }
        if let Err(e) = self.journal.retire() {
            errors.push(e);
        }
        errors
    }

    fn recovery_retained(&self) -> bool {
        self.journal.active
            || self.recovery.active
            || self.lock.active
            || self.mutation_guard.active
    }

    /// A confirmed-dead generator may release the compile lock after restore/finalization failed,
    /// but the recovery artifacts must remain exactly as reported to the caller. Do not let Drop
    /// turn a structured `RecoveryRequiredRestoreFailed` into a transient, unobservable retry.
    fn preserve_for_restore_failure(mut self) {
        let _ = self.remove_ephemeral_deploy_backup();
        let _ = self.lock.release();
        // Unit tests emulate retained recovery in-process and must be able to remove their unique
        // fixture afterward. Production deliberately keeps this exact handle open as extra
        // protection for the persistent shared lock until process exit/manual recovery.
        #[cfg(test)]
        self.mutation_guard.close_handle_preserving_record();
        std::mem::forget(self);
    }

    /// A generator process that might still be alive must retain exclusive ownership of every
    /// path it can touch. Deliberately leak the transaction guards so Drop cannot race that process
    /// by restoring Script/JIT/proxy state or releasing either lock. The disk recovery backup and
    /// quarantine paths make the pre-call state recoverable after the process is killed.
    fn preserve_for_unconfirmed_generator(self, cause: String) -> String {
        #[cfg(test)]
        let mut transaction = self;
        #[cfg(not(test))]
        let transaction = self;
        let recovery = transaction.recovery.path.display().to_string();
        let journal = transaction.journal.root.display().to_string();
        let lock = transaction.lock.path.display().to_string();
        let mutation_lock = transaction.mutation_guard.path.display().to_string();
        let game_root = game_root_dir(&transaction.game_dir).display().to_string();
        #[cfg(test)]
        transaction.mutation_guard.close_handle_preserving_record();
        std::mem::forget(transaction);
        format!(
            "{cause}; cleanup was intentionally NOT run because the generator's exit could not be \
             confirmed. Kill the reported process tree before recovery; the Shipping recovery \
             cache is {recovery}, the source/dev recovery journal is {journal}, quarantined side \
             effects are beside their originals under {game_root}, and the compile lock remains \
             at {lock}; the cross-tool install-mutation lock remains at {mutation_lock}"
        )
    }
}

impl Drop for CompileTransaction {
    fn drop(&mut self) {
        let mut restore_failed = false;
        if self.rollback_needed {
            let report = self.restore_install();
            // If Shipping itself could not be restored, preserve the recovery backup. Other
            // cleanup failures also keep it as the conservative crash-recovery artifact.
            if !report.clean() || !report.shipping_restored {
                restore_failed = true;
            }
        }
        let _ = self.remove_ephemeral_deploy_backup();
        if restore_failed {
            let _ = self.lock.release();
            return;
        }
        if self.recovery.retire().is_ok()
            && self.lock.release().is_ok()
            && self.mutation_guard.release().is_ok()
        {
            let _ = self.journal.retire();
        }
    }
}

/// The real game launch. Places the loose `.as` tree where the game reads it, launches the
/// shipping exe in AngelScript development/generation mode, waits for the generated development
/// cache (`PrecompiledScript.Cache`), and returns a workspace copy of that cache.
///
/// Compiling normally leaves the install unchanged: on every confirmed-process exit path this restores both
/// `PrecompiledScript_Shipping.Cache` and the optional pre-existing `PrecompiledScript.Cache` to
/// their exact pre-call states, then undoes every staged source file. If generator termination
/// cannot be confirmed, isolation and the lock intentionally remain in place for manual recovery.
pub fn game_run_regen(game_dir: &Path, src_dir: &Path) -> Result<PathBuf, String> {
    game_run_regen_with_diagnostics(game_dir, src_dir, &Default::default())
}

/// Transactional generator result paired with the bounded diagnostics report produced after the
/// compiler process started. Setup failures that occur before a process/report exists remain the
/// outer `Err` of [`game_run_regen_with_diagnostics_report`].
#[derive(Debug)]
pub struct GameRunRegenReport {
    result: Result<PathBuf, String>,
    diagnostics: crate::diagnostics::CompilerDiagnosticsReport,
    install_restore: InstallRestoreDisposition,
}

impl GameRunRegenReport {
    pub fn result(&self) -> Result<&Path, &str> {
        self.result.as_deref().map_err(String::as_str)
    }

    pub fn diagnostics(&self) -> &crate::diagnostics::CompilerDiagnosticsReport {
        &self.diagnostics
    }

    pub fn install_restore_disposition(&self) -> InstallRestoreDisposition {
        self.install_restore
    }

    pub fn into_parts(
        self,
    ) -> (
        Result<PathBuf, String>,
        crate::diagnostics::CompilerDiagnosticsReport,
    ) {
        (self.result, self.diagnostics)
    }
}

/// Internal superset used by module compilation. Unlike the public report, this can represent a
/// transactional setup/restore failure before the diagnostics runner was reached.
#[derive(Debug)]
struct GameRunRegenExtendedReport {
    result: Result<PathBuf, String>,
    diagnostics: Option<crate::diagnostics::CompilerDiagnosticsReport>,
    install_restore: InstallRestoreDisposition,
}

/// Same transactional compiler path as [`game_run_regen`], with explicit diagnostics discovery /
/// opt-out settings. The helper is temporary and optional; generator availability never depends on
/// it.
pub fn game_run_regen_with_diagnostics(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<PathBuf, String> {
    match game_run_regen_with_diagnostics_report(game_dir, src_dir, diagnostics) {
        Ok(report) => report.result,
        Err(error) => Err(error),
    }
}

/// Same transactional install-restoring compiler path as [`game_run_regen_with_diagnostics`], but
/// preserve the structured capture disposition and messages without deriving them from stderr or
/// a formatted error string.
pub fn game_run_regen_with_diagnostics_report(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<GameRunRegenReport, String> {
    let extended = game_run_regen_with_extended_diagnostics_report(game_dir, src_dir, diagnostics)?;
    let GameRunRegenExtendedReport {
        result,
        diagnostics,
        install_restore,
    } = extended;
    let Some(diagnostics) = diagnostics else {
        return Err(result.err().unwrap_or_else(|| {
            "game compiler completed without producing its diagnostics disposition".to_owned()
        }));
    };
    Ok(GameRunRegenReport {
        result,
        diagnostics,
        install_restore,
    })
}

fn game_run_regen_with_extended_diagnostics_report(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<GameRunRegenExtendedReport, String> {
    let diagnostic_report = std::cell::RefCell::new(None);
    let generated = game_run_regen_with_install_report(game_dir, src_dir, |exe, g1r, cache| {
        let generated = real_generate_with_timeout_and_diagnostics_report(
            exe,
            g1r,
            cache,
            Duration::from_secs(30 * 60),
            diagnostics,
        );
        *diagnostic_report.borrow_mut() = Some(generated.diagnostics);
        GeneratorRunResult {
            result: generated.result,
            process_exit: generated.process_exit,
        }
    })?;
    Ok(GameRunRegenExtendedReport {
        result: generated.result,
        diagnostics: diagnostic_report.into_inner(),
        install_restore: generated.install_restore,
    })
}

fn game_run_regen_with_extended_diagnostics_report_with_guard(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
    mutation_guard: InstallMutationGuard,
) -> Result<GameRunRegenExtendedReport, String> {
    let diagnostic_report = std::cell::RefCell::new(None);
    let generated = game_run_regen_with_install_report_with_guard(
        game_dir,
        src_dir,
        mutation_guard,
        |exe, g1r, cache| {
            let generated = real_generate_with_timeout_and_diagnostics_report(
                exe,
                g1r,
                cache,
                Duration::from_secs(30 * 60),
                diagnostics,
            );
            *diagnostic_report.borrow_mut() = Some(generated.diagnostics);
            GeneratorRunResult {
                result: generated.result,
                process_exit: generated.process_exit,
            }
        },
    )?;
    Ok(GameRunRegenExtendedReport {
        result: generated.result,
        diagnostics: diagnostic_report.into_inner(),
        install_restore: generated.install_restore,
    })
}

fn run_project_compiler_with_guard<A>(
    game_dir: &Path,
    source_tree: &Path,
    diagnostics_options: &crate::diagnostics::DiagnosticsOptions,
    mutation_guard: InstallMutationGuard,
    closing_audit: A,
) -> ProjectCompilerRunnerReport
where
    A: FnOnce() -> Result<(), String>,
{
    let expected_copy = source_tree.join("regen.cache");
    let diagnostic_report = std::cell::RefCell::new(None);
    let diagnostic_private_paths = std::cell::RefCell::new(Vec::new());
    let consumed_result = std::cell::RefCell::new(None);
    let output_disposition = std::cell::Cell::new(ProjectCompilerOutputDisposition::NotCreated);
    let closing_audit_disposition =
        std::cell::Cell::new(ProjectCompilerClosingAuditDisposition::NotRun);
    let closing_audit = std::cell::RefCell::new(Some(closing_audit));
    let callback_failure = std::cell::RefCell::new(None);

    let generated = game_run_regen_with_install_report_with_guard_and_after_restore(
        game_dir,
        source_tree,
        mutation_guard,
        |exe, g1r, cache| {
            let generated = real_generate_with_timeout_and_diagnostics_report(
                exe,
                g1r,
                cache,
                Duration::from_secs(30 * 60),
                diagnostics_options,
            );
            *diagnostic_private_paths.borrow_mut() = generated.private_paths;
            *diagnostic_report.borrow_mut() = Some(generated.diagnostics);
            GeneratorRunResult {
                result: generated.result,
                process_exit: generated.process_exit,
            }
        },
        |path_result, retained_copy| {
            let (converted, disposition) = match path_result {
                Ok(path) => match retained_copy {
                    Some(artifact) => consume_project_regen_artifact(source_tree, path, artifact),
                    None => (
                        Err("project compiler lost the retained private output handle".to_owned()),
                        ProjectCompilerOutputDisposition::RecoveryRetained,
                    ),
                },
                Err(error) => {
                    let (cleanup, disposition) = match retained_copy {
                        Some(artifact) => discard_project_regen_artifact(&expected_copy, artifact),
                        None => discard_project_regen_copy(&expected_copy),
                    };
                    (
                        Err(match cleanup {
                            Ok(_) => error.clone(),
                            Err(cleanup) => format!(
                                "{error}; additionally failed to dispose the private regenerated cache: {cleanup}"
                            ),
                        }),
                        disposition,
                    )
                }
            };
            output_disposition.set(disposition);
            let conversion_failure = converted.as_ref().err().cloned();
            *consumed_result.borrow_mut() = Some(converted);

            let mut failures = Vec::new();
            if path_result.is_ok()
                || disposition == ProjectCompilerOutputDisposition::RecoveryRetained
            {
                if let Some(failure) = conversion_failure {
                    failures.push(failure);
                }
            }
            let audit = closing_audit
                .borrow_mut()
                .take()
                .ok_or_else(|| "closing project audit was consumed more than once".to_owned())?;
            match audit() {
                Ok(()) => {
                    closing_audit_disposition.set(ProjectCompilerClosingAuditDisposition::Passed)
                }
                Err(error) => {
                    closing_audit_disposition.set(ProjectCompilerClosingAuditDisposition::Failed);
                    failures.push(format!(
                        "closing project audit failed while the install guard was held: {error}"
                    ));
                }
            }
            if failures.is_empty() {
                Ok(())
            } else {
                let failure = failures.join("; ");
                *callback_failure.borrow_mut() = Some(failure.clone());
                Err(failure)
            }
        },
    );

    let (common_result, install_restore) = match generated {
        Ok(report) => (report.result, report.install_restore),
        Err(error) => (Err(error), InstallRestoreDisposition::NotStarted),
    };
    let diagnostics = diagnostic_report.into_inner();
    let mut output_disposition = output_disposition.get();
    let closing_audit = closing_audit_disposition.get();
    let mut result = if let Some(failure) = callback_failure.into_inner() {
        Err(failure)
    } else if let Some(consumed) = consumed_result.into_inner() {
        consumed
    } else {
        match common_result {
            Ok(path) => {
                let (consumed, disposition) = consume_project_regen_copy(source_tree, &path);
                output_disposition = disposition;
                consumed
            }
            Err(error) => {
                let (cleanup, disposition) = discard_project_regen_copy(&expected_copy);
                output_disposition = disposition;
                Err(match cleanup {
                    Ok(_) => redact_private_regen_path(&error, &expected_copy),
                    Err(cleanup) => format!(
                        "{}; additionally failed to dispose the private regenerated cache: {cleanup}",
                        redact_private_regen_path(&error, &expected_copy)
                    ),
                })
            }
        }
    };

    if result.is_ok() {
        if let Some(error) = project_diagnostics_acceptance_error(diagnostics.as_ref()) {
            result = Err(error);
        }
    }
    if let Err(error) = &mut result {
        *error =
            redact_private_compiler_recovery_paths(error, &diagnostic_private_paths.into_inner());
    }

    if matches!(
        install_restore,
        InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
            | InstallRestoreDisposition::RecoveryRequiredRestoreFailed
    ) {
        output_disposition = ProjectCompilerOutputDisposition::RecoveryRetained;
    }

    ProjectCompilerRunnerReport {
        result,
        diagnostics,
        install_restore,
        output_disposition,
        closing_audit,
    }
}

fn consume_project_regen_copy(
    source_tree: &Path,
    returned_path: &Path,
) -> (Result<Vec<u8>, String>, ProjectCompilerOutputDisposition) {
    let expected = source_tree.join("regen.cache");
    if returned_path != expected {
        let cleanup = discard_project_regen_copy(&expected).0.err();
        let mut error =
            "project compiler returned a cache outside its private output slot".to_owned();
        if let Some(cleanup) = cleanup {
            error.push_str(&format!(
                "; additionally failed to dispose the expected private cache: {cleanup}"
            ));
        }
        return (
            Err(error),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        );
    }

    let file = match open_compiled_artifact_existing(&expected) {
        Ok(file) => file,
        Err(error) => {
            return (
                Err(format!("opening private regenerated cache: {error}")),
                ProjectCompilerOutputDisposition::RecoveryRetained,
            );
        }
    };
    consume_project_regen_artifact(
        source_tree,
        returned_path,
        CompiledArtifact {
            path: expected,
            file,
        },
    )
}

fn consume_project_regen_artifact(
    source_tree: &Path,
    returned_path: &Path,
    mut artifact: CompiledArtifact,
) -> (Result<Vec<u8>, String>, ProjectCompilerOutputDisposition) {
    let expected = source_tree.join("regen.cache");
    if returned_path != expected || artifact.path != expected {
        let artifact_path_matches = artifact.path == expected;
        let cleanup = if artifact_path_matches {
            discard_project_regen_artifact(&expected, artifact).0.err()
        } else {
            artifact.neutralize().err()
        };
        let mut error =
            "project compiler returned a cache that did not match its retained private output handle"
                .to_owned();
        if let Some(cleanup) = cleanup {
            error.push_str(&format!(
                "; additionally failed to neutralize the retained private cache: {cleanup}"
            ));
        }
        return (
            Err(error),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        );
    }

    let mut bytes = Vec::new();
    let read = (|| -> Result<(), String> {
        let length = artifact
            .file
            .metadata()
            .map_err(|error| format!("inspecting private regenerated cache: {error}"))?
            .len();
        if length > MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES {
            return Err(format!(
                "private regenerated cache has {length} bytes; maximum is {MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES}"
            ));
        }
        artifact
            .file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("seeking private regenerated cache: {error}"))?;
        Read::by_ref(&mut artifact.file)
            .take(MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("reading private regenerated cache: {error}"))?;
        if bytes.len() as u64 > MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES {
            return Err(format!(
                "private regenerated cache exceeded {MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES} bytes while reading"
            ));
        }
        Ok(())
    })();
    let cleanup = neutralize_and_remove_project_regen_artifact(&expected, artifact);
    match (read, cleanup) {
        (Ok(()), Ok(())) => (Ok(bytes), ProjectCompilerOutputDisposition::Discarded),
        (Err(error), Ok(())) => (Err(error), ProjectCompilerOutputDisposition::Discarded),
        (Ok(()), Err(cleanup)) => (
            Err(format!(
                "compiled, but private cache disposal failed: {cleanup}"
            )),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        ),
        (Err(error), Err(cleanup)) => (
            Err(format!("{error}; additionally disposal failed: {cleanup}")),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        ),
    }
}

fn neutralize_and_remove_project_regen_artifact(
    path: &Path,
    artifact: CompiledArtifact,
) -> Result<(), String> {
    let neutralize = artifact.neutralize();
    drop(artifact);
    let remove = match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("removing neutralized private cache: {error}")),
    };
    match (neutralize, remove) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(neutralize), Ok(())) => Err(neutralize),
        (Ok(()), Err(remove)) => Err(remove),
        (Err(neutralize), Err(remove)) => Err(format!("{neutralize}; additionally {remove}")),
    }
}

fn create_project_regen_artifact(path: &Path) -> Result<CompiledArtifact, String> {
    Ok(CompiledArtifact {
        path: path.to_path_buf(),
        file: open_compiled_artifact_create_new(path)
            .map_err(|error| format!("creating retained regen copy: {error}"))?,
    })
}

fn redact_private_regen_path(error: &str, private_path: &Path) -> String {
    error.replace(
        &private_path.display().to_string(),
        "<private compiler output>",
    )
}

fn redact_private_compiler_recovery_paths(error: &str, private_paths: &[PathBuf]) -> String {
    private_paths
        .iter()
        .fold(error.to_owned(), |message, path| {
            message.replace(
                &path.display().to_string(),
                "<private compiler recovery path>",
            )
        })
}

fn discard_project_regen_copy(
    path: &Path,
) -> (Result<bool, String>, ProjectCompilerOutputDisposition) {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (Ok(false), ProjectCompilerOutputDisposition::NotCreated)
        }
        Err(error) => (
            Err(format!("inspecting private regenerated cache: {error}")),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        ),
        Ok(_) => match std::fs::remove_file(path) {
            Ok(()) => (
                Err(
                    "removed an unexpected private-cache pathname, but no retained creation handle was available to prove its bytes neutralized"
                        .to_owned(),
                ),
                ProjectCompilerOutputDisposition::RecoveryRetained,
            ),
            Err(error) => (
                Err(format!(
                    "removing unexpected private-cache pathname without following it: {error}"
                )),
                ProjectCompilerOutputDisposition::RecoveryRetained,
            ),
        },
    }
}

fn discard_project_regen_artifact(
    path: &Path,
    artifact: CompiledArtifact,
) -> (Result<bool, String>, ProjectCompilerOutputDisposition) {
    if artifact.path != path {
        let cleanup = artifact.neutralize().err();
        let mut error =
            "retained private cache handle does not match the expected output path".to_owned();
        if let Some(cleanup) = cleanup {
            error.push_str(&format!(
                "; additionally failed to neutralize the retained private cache: {cleanup}"
            ));
        }
        return (
            Err(error),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        );
    }
    match neutralize_and_remove_project_regen_artifact(path, artifact) {
        Ok(()) => (Ok(true), ProjectCompilerOutputDisposition::Discarded),
        Err(error) => (
            Err(error),
            ProjectCompilerOutputDisposition::RecoveryRetained,
        ),
    }
}

/// Testable core of [`game_run_regen`]. `generate` receives the executable, G1R directory, and
/// the *development* cache path. It must return the bytes generated there.
#[cfg(test)]
fn game_run_regen_with<G>(game_dir: &Path, src_dir: &Path, generate: G) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> Result<Vec<u8>, String>,
{
    game_run_regen_with_install_report_and(
        game_dir,
        src_dir,
        || Ok(false),
        CompileTransaction::begin_isolation,
        |exe, g1r, cache| {
            let result = generate(exe, g1r, cache);
            let process_exit = if result
                .as_ref()
                .err()
                .is_some_and(|error| generator_exit_unconfirmed(error))
            {
                GeneratorProcessExitDisposition::Unconfirmed
            } else {
                GeneratorProcessExitDisposition::Confirmed
            };
            GeneratorRunResult {
                result,
                process_exit,
            }
        },
    )
    .and_then(|report| report.result)
}

#[derive(Debug)]
struct GameRunInstallReport {
    result: Result<PathBuf, String>,
    install_restore: InstallRestoreDisposition,
}

fn game_run_regen_with_install_report<G>(
    game_dir: &Path,
    src_dir: &Path,
    generate: G,
) -> Result<GameRunInstallReport, String>
where
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    game_run_regen_with_install_report_and(
        game_dir,
        src_dir,
        shipping_game_process_running,
        CompileTransaction::begin_isolation,
        generate,
    )
}

fn game_run_regen_with_install_report_and<C, I, G>(
    game_dir: &Path,
    src_dir: &Path,
    check_game_process: C,
    begin_isolation: I,
    generate: G,
) -> Result<GameRunInstallReport, String>
where
    C: Fn() -> Result<bool, String>,
    I: FnOnce(&mut CompileTransaction) -> Result<(), String>,
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    let begin_recovery_required = std::cell::Cell::new(false);
    let run = game_run_regen_with_install_report_using_transaction(
        game_dir,
        src_dir,
        |game_dir, g1r, script_dir| match CompileTransaction::begin_with_process_checker_report(
            game_dir,
            g1r,
            script_dir,
            check_game_process,
        ) {
            Ok(transaction) => Ok(transaction),
            Err(failure) => {
                begin_recovery_required.set(failure.recovery_required);
                Err(failure.message)
            }
        },
        begin_isolation,
        generate,
        |_, _| Ok(()),
    );
    match run {
        Err(message) if begin_recovery_required.get() => Ok(GameRunInstallReport {
            result: Err(message),
            install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
        }),
        other => other,
    }
}

fn game_run_regen_with_install_report_with_guard<G>(
    game_dir: &Path,
    src_dir: &Path,
    mutation_guard: InstallMutationGuard,
    generate: G,
) -> Result<GameRunInstallReport, String>
where
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    game_run_regen_with_install_report_with_guard_and_after_restore(
        game_dir,
        src_dir,
        mutation_guard,
        generate,
        |_, _| Ok(()),
    )
}

fn game_run_regen_with_install_report_with_guard_and_after_restore<G, A>(
    game_dir: &Path,
    src_dir: &Path,
    mutation_guard: InstallMutationGuard,
    generate: G,
    after_restore: A,
) -> Result<GameRunInstallReport, String>
where
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
    A: FnOnce(&Result<PathBuf, String>, Option<CompiledArtifact>) -> Result<(), String>,
{
    let mutation_guard = std::cell::RefCell::new(Some(mutation_guard));
    let after_restore = std::cell::RefCell::new(Some(after_restore));
    let begin_recovery_required = std::cell::Cell::new(false);
    let run =
        game_run_regen_with_install_report_using_transaction(
            game_dir,
            src_dir,
            |game_dir, g1r, script_dir| {
                let guard = mutation_guard.borrow_mut().take().ok_or_else(|| {
                    "pre-held install-mutation guard was consumed more than once".to_owned()
                })?;
                match CompileTransaction::begin_with_mutation_guard_and_process_checker(
                    game_dir,
                    g1r,
                    script_dir,
                    guard,
                    shipping_game_process_running,
                ) {
                    Ok(transaction) => Ok(transaction),
                    Err(failure) => {
                        begin_recovery_required.set(failure.recovery_required);
                        Err(failure.message)
                    }
                }
            },
            CompileTransaction::begin_isolation,
            generate,
            |result, artifact| {
                after_restore.borrow_mut().take().ok_or_else(|| {
                    "after-restore callback was consumed more than once".to_owned()
                })?(result, artifact)
            },
        );

    // Exe/source-tree checks in the common runner happen before its begin closure. If one fails,
    // ownership is still here and must be explicitly finalized rather than silently dropped.
    if let Some(unused_guard) = mutation_guard.into_inner() {
        let primary = match &run {
            Err(error) => error,
            Ok(_) => "compiler transaction returned without consuming its pre-held guard",
        };
        let mut errors = vec![primary.to_owned()];
        if let Some(callback) = after_restore.into_inner() {
            if let Err(error) = callback(&Err(primary.to_owned()), None) {
                errors.push(format!(
                    "after-restore callback failed while the pre-held install guard was active: \
                     {error}"
                ));
            }
        }
        let failure = finalize_compile_transaction_begin_failure(game_dir, unused_guard, errors);
        if failure.recovery_required {
            return Ok(GameRunInstallReport {
                result: Err(failure.message),
                install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
            });
        }
        return Err(failure.message);
    }

    match run {
        Err(message) if begin_recovery_required.get() => Ok(GameRunInstallReport {
            result: Err(message),
            install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
        }),
        other => other,
    }
}

fn game_run_regen_with_install_report_using_transaction<B, I, G, A>(
    game_dir: &Path,
    src_dir: &Path,
    begin_transaction: B,
    begin_isolation: I,
    generate: G,
    after_restore: A,
) -> Result<GameRunInstallReport, String>
where
    B: FnOnce(&Path, &Path, &Path) -> Result<CompileTransaction, String>,
    I: FnOnce(&mut CompileTransaction) -> Result<(), String>,
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
    A: FnOnce(&Result<PathBuf, String>, Option<CompiledArtifact>) -> Result<(), String>,
{
    let g1r = g1r_dir(game_dir);
    let exe = g1r
        .join("Binaries")
        .join("Win64")
        .join(G1R_SHIPPING_EXE_NAME);
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let dev_cache = script_dir.join("PrecompiledScript.Cache");

    // Existing loose scripts whose relative path is absent from our complete staged tree would be
    // compiled too but never appear in `written`, silently contaminating the regen. Matching paths
    // are safe: copy_tree snapshots and restores their exact bytes.
    if let Some(stray) = first_uncovered_loose_script(&script_dir, src_dir)
        .map_err(|e| format!("inspecting the game's Script/ tree: {e}"))?
    {
        return Err(format!(
            "the game's Script/ directory contains a loose script not present in the staged tree \
             ({}); refusing a contaminated compile",
            stray.display()
        ));
    }

    let mut txn = begin_transaction(game_dir, &g1r, &script_dir)?;
    let regen_out = src_dir.join("regen.cache");
    let _ = std::fs::remove_file(&regen_out);
    let mut retained_regen_copy = None;
    let mut process_exit = GeneratorProcessExitDisposition::NotStarted;
    let result = (|| -> Result<PathBuf, String> {
        // Quarantine process-wide side effects before staging or deleting either cache. From this
        // point onward CompileTransaction::drop can roll back an unwind at any instruction.
        begin_isolation(&mut txn)?;
        txn.stage(src_dir)?;
        // A source tree may accidentally contain this filename, so remove the stale/staged dev
        // cache immediately before launch. The saved pre-call state is restored below.
        remove_if_exists(&txn.dev_cache)?;
        let generated = generate(&exe, &g1r, &dev_cache);
        process_exit = generated.process_exit;
        let regen = generated.result?;
        if regen.is_empty() {
            return Err("the game produced an empty PrecompiledScript.Cache".into());
        }
        validate_generated_cache(&regen)?;
        let mut artifact = create_project_regen_artifact(&regen_out)?;
        let write = artifact
            .file
            .write_all(&regen)
            .and_then(|_| artifact.file.sync_all())
            .map_err(|error| format!("writing retained regen copy: {error}"));
        retained_regen_copy = Some(artifact);
        write?;
        Ok(regen_out.clone())
    })();

    if process_exit == GeneratorProcessExitDisposition::Unconfirmed {
        let error = result
            .err()
            .unwrap_or_else(|| "generator exit was unconfirmed after reporting success".to_owned());
        return Ok(GameRunInstallReport {
            result: Err(txn.preserve_for_unconfirmed_generator(error)),
            install_restore: InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed,
        });
    }

    // Undo staged files first: they may include either cache filename. Explicit cache restoration
    // then wins and guarantees the exact snapshots even if the generator unexpectedly touched the
    // shipping cache too.
    let report = txn.restore_install();
    let mut cleanup_errors = report.errors;
    if cleanup_errors.is_empty() {
        if let Err(error) = after_restore(&result, retained_regen_copy.take()) {
            cleanup_errors.push(error);
        }
        cleanup_errors.extend(txn.finish());
    }

    let result = match result {
        Ok(p) if cleanup_errors.is_empty() => Ok(p),
        Ok(p) => Err(format!(
            "compiled to {}, but {}",
            p.display(),
            cleanup_errors.join("; ")
        )),
        Err(e) if cleanup_errors.is_empty() => Err(e),
        Err(e) => Err(format!("{e}; additionally {}", cleanup_errors.join("; "))),
    };
    if txn.recovery_retained() {
        txn.preserve_for_restore_failure();
        return Ok(GameRunInstallReport {
            result,
            install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
        });
    }
    Ok(GameRunInstallReport {
        result,
        install_restore: InstallRestoreDisposition::RestoredExact,
    })
}

/// Options for [`precompile`] — driving the game's own `-as-generate-precompiled-data` step as a
/// standalone compiler that handles all the file juggling (backup, staging, output, restore).
pub struct PrecompileOpts {
    /// Game install root (the folder containing `G1R/`, or the `G1R` dir itself).
    pub game_dir: PathBuf,
    /// Source `.as` tree to stage under `Script/` before compiling. `None` recompiles whatever
    /// `.as` are already installed there.
    pub src: Option<PathBuf>,
    /// Where to write the compiled cache. `Some` writes it there and RESTORES the install to its
    /// pre-call state (the live cache and any staged sources are put back → install untouched).
    /// `None` installs the fresh cache in place under `Script/`.
    pub out: Option<PathBuf>,
    /// When installing in place (`out` is `None`), back up the previous cache to `<cache>.gore-bak`
    /// first — unless one already exists, so the earliest (pristine) backup is preserved.
    pub backup: bool,
}

/// Compile `.as` into a precompiled script cache by driving the game, handling backup, staging,
/// output placement and restore internally. Returns the path of the resulting cache (`out` if set,
/// else the in-place `Script/PrecompiledScript_Shipping.Cache`).
pub fn precompile(opts: &PrecompileOpts) -> Result<PathBuf, String> {
    precompile_with_generator_report(opts, |exe, g1r, cache| {
        real_generate_report(exe, g1r, cache, &Default::default())
    })
}

/// [`precompile`] with explicit optional compiler-diagnostic capture settings.
pub fn precompile_with_diagnostics(
    opts: &PrecompileOpts,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<PathBuf, String> {
    precompile_with_generator_report(opts, |exe, g1r, cache| {
        real_generate_report(exe, g1r, cache, diagnostics)
    })
}

/// The first loose `.as` file found anywhere under `dir` (recursively), or `None`. Used to reject a
/// dirty Script/ before staging a SRC tree, so the game never compiles leftover scripts alongside it.
#[cfg(test)]
fn first_loose_script(dir: &Path) -> std::io::Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if metadata_is_link_or_reparse(&meta) {
            return Err(std::io::Error::other(format!(
                "refusing linked/reparse path while scanning Script/: {}",
                path.display()
            )));
        }
        if meta.is_dir() {
            if let Some(found) = first_loose_script(&path)? {
                return Ok(Some(found));
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("as"))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// First loose `.as` under `script_dir` whose relative path is not a real file in `src_dir`.
/// Used by the full-tree compile backend: colliding files are safe because staging records/restores
/// them, while an uncovered extra file would be compiled silently alongside the requested tree.
fn first_uncovered_loose_script(
    script_dir: &Path,
    src_dir: &Path,
) -> std::io::Result<Option<PathBuf>> {
    fn walk(dir: &Path, root: &Path, src: &Path) -> std::io::Result<Option<PathBuf>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if metadata_is_link_or_reparse(&meta) {
                return Err(std::io::Error::other(format!(
                    "refusing linked/reparse path while scanning Script/: {}",
                    path.display()
                )));
            }
            if meta.is_dir() {
                if let Some(found) = walk(&path, root, src)? {
                    return Ok(Some(found));
                }
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("as"))
            {
                let rel = path.strip_prefix(root).map_err(|e| {
                    std::io::Error::other(format!(
                        "walked path {} escaped root {}: {e}",
                        path.display(),
                        root.display()
                    ))
                })?;
                let covered = src.join(rel);
                match std::fs::symlink_metadata(&covered) {
                    Ok(meta) if metadata_is_link_or_reparse(&meta) => {
                        return Err(std::io::Error::other(format!(
                            "refusing linked/reparse source coverage path: {}",
                            covered.display()
                        )));
                    }
                    Ok(meta) if meta.is_file() => {}
                    Ok(_) => return Ok(Some(path)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok(Some(path));
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(None)
    }
    walk(script_dir, script_dir, src_dir)
}

/// Resolve `out` to an absolute path for the "output must be outside Script/" containment check:
/// relative paths are taken relative to `cwd` (so a relative `-o` can't slip past the check), then
/// canonicalized as far as the path exists — the file itself, else its existing parent joined with
/// the filename, else the lexical absolute path. Extracted so the guard is testable without mutating
/// the process cwd.
fn resolve_out_real(out: &Path, cwd: &Path) -> PathBuf {
    let abs = if out.is_absolute() {
        out.to_path_buf()
    } else {
        cwd.join(out)
    };
    abs.canonicalize()
        .unwrap_or_else(|_| match (abs.parent(), abs.file_name()) {
            (Some(parent), Some(name)) => parent
                .canonicalize()
                .map(|p| p.join(name))
                .unwrap_or_else(|_| abs.clone()),
            _ => abs.clone(),
        })
}

/// Testable core of [`precompile`]. `generate(exe, g1r, dev_cache)` must make the game write
/// `PrecompiledScript.Cache` and return its bytes; the real public paths use
/// [`real_generate_report`], while tests inject a stub so orchestration stays offline.
#[cfg(test)]
fn precompile_with<G>(opts: &PrecompileOpts, generate: G) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> Result<Vec<u8>, String>,
{
    precompile_with_generator_report_and_process_checker(
        opts,
        || Ok(false),
        |exe, g1r, cache| GeneratorRunResult::confirmed(generate(exe, g1r, cache)),
    )
}

fn precompile_with_generator_report<G>(
    opts: &PrecompileOpts,
    generate: G,
) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    precompile_with_generator_report_and_process_checker(
        opts,
        shipping_game_process_running,
        generate,
    )
}

fn precompile_with_generator_report_and_process_checker<C, G>(
    opts: &PrecompileOpts,
    check_game_process: C,
    generate: G,
) -> Result<PathBuf, String>
where
    C: Fn() -> Result<bool, String>,
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    let g1r = g1r_dir(&opts.game_dir);
    let exe = g1r
        .join("Binaries")
        .join("Win64")
        .join(G1R_SHIPPING_EXE_NAME);
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let shipping_cache = script_dir.join("PrecompiledScript_Shipping.Cache");
    let dev_cache = script_dir.join("PrecompiledScript.Cache");

    // Reject a source tree that contains (or IS) the Script destination: `copy_tree` would copy the
    // install into its own subtree, recursing `Script/…/Script` until the path or disk blows up while
    // polluting the live install. (Mirrors deploy_shared's self-copy guard.)
    if let Some(src) = &opts.src {
        let src_real = src.canonicalize().unwrap_or_else(|_| src.clone());
        let dst_real = script_dir
            .canonicalize()
            .unwrap_or_else(|_| script_dir.clone());
        if dst_real == src_real || dst_real.starts_with(&src_real) {
            return Err(format!(
                "source {} contains the game's Script/ directory ({}); point the source at your \
                 emitted .as tree, not the game root",
                src.display(),
                script_dir.display()
            ));
        }
    }

    // The output must live OUTSIDE the game's Script/ directory. Writing it inside would pollute the
    // install (breaking out-mode's pristine-install contract); worse, if it lands on the live cache
    // or a file staged from SRC, the later restore/cleanup would overwrite or delete the artifact we
    // just wrote while still returning Ok. Reject any output under Script/ — to update the live
    // cache, omit `-o` (in-place mode).
    if let Some(out) = &opts.out {
        let script_real = script_dir
            .canonicalize()
            .unwrap_or_else(|_| script_dir.clone());
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if resolve_out_real(out, &cwd).starts_with(&script_real) {
            return Err(format!(
                "output {} is inside the game's Script/ directory ({}); write the compiled cache \
                 elsewhere, or omit -o to install in place",
                out.display(),
                script_dir.display()
            ));
        }
    }

    // When compiling a specific SRC tree, the game must see ONLY paths covered by that tree. The
    // transaction snapshots and restores existing files at matching relative paths, so a normal
    // installed source tree is safe to overlay. Refuse only an uncovered loose script: otherwise it
    // would silently participate in the generated cache.
    if let Some(src) = &opts.src {
        if let Some(stray) = first_uncovered_loose_script(&script_dir, src)
            .map_err(|e| format!("inspecting the game's Script/ tree: {e}"))?
        {
            return Err(format!(
                "the game's Script/ directory ({}) contains a loose script not present in the \
                 staged source tree ({}); refusing a contaminated compile",
                script_dir.display(),
                stray.display()
            ));
        }
    }

    // Both compile entry points share the cross-tool mutation guard, the gore-as compile lock, and
    // the disk-backed Shipping recovery guard. Generation isolation begins before staging or
    // deleting the development cache.
    let mut txn = CompileTransaction::begin_with_process_checker(
        &opts.game_dir,
        &g1r,
        &script_dir,
        check_game_process,
    )?;
    let mut process_exit = GeneratorProcessExitDisposition::NotStarted;
    let result = (|| -> Result<Vec<u8>, String> {
        txn.begin_isolation()?;
        if let Some(src) = &opts.src {
            txn.stage(src)?;
        }
        // Delete the old (or accidentally staged) development cache immediately before launch so
        // existence/size can only describe this run. The optional original is restored below.
        remove_if_exists(&txn.dev_cache)?;
        let generated = generate(&exe, &g1r, &dev_cache);
        process_exit = generated.process_exit;
        let regen = generated.result?;
        if regen.is_empty() {
            return Err("the game produced an empty PrecompiledScript.Cache".into());
        }
        validate_generated_cache(&regen)?;
        Ok(regen)
    })();

    if process_exit == GeneratorProcessExitDisposition::Unconfirmed {
        let error = result
            .err()
            .unwrap_or_else(|| "generator exit was unconfirmed after reporting success".to_owned());
        return Err(txn.preserve_for_unconfirmed_generator(error));
    }

    // Always restore the complete install before either publishing an output artifact or starting
    // the explicit in-place install phase.
    let restore = txn.restore_install();
    if !restore.clean() {
        let primary = result.err().unwrap_or_else(|| {
            "compiled, but refusing output/install because cleanup failed".into()
        });
        return Err(format!(
            "{primary}; additionally {}",
            restore.errors.join("; ")
        ));
    }

    let regen = match result {
        Ok(regen) => regen,
        Err(e) => {
            let finish = txn.finish();
            return if finish.is_empty() {
                Err(e)
            } else {
                Err(format!("{e}; additionally {}", finish.join("; ")))
            };
        }
    };

    if let Some(out) = &opts.out {
        let finish = txn.finish();
        if !finish.is_empty() {
            return Err(format!(
                "compiled, but refusing output because transaction cleanup failed: {}",
                finish.join("; ")
            ));
        }
        std::fs::write(out, &regen)
            .map_err(|e| format!("writing output {}: {e}", out.display()))?;
        return Ok(out.clone());
    }

    // A persistent deploy `.gore-bak` is created only now: generation and all restoration already
    // succeeded. A failed install removes a backup created by this call; a pre-existing one is
    // never overwritten or removed.
    if let Err(e) = txn.prepare_deploy_backup(opts.backup) {
        let finish = txn.finish();
        return if finish.is_empty() {
            Err(e)
        } else {
            Err(format!("{e}; additionally {}", finish.join("; ")))
        };
    }

    txn.arm_install_rollback();
    match std::fs::write(&shipping_cache, &regen) {
        Ok(()) => {
            txn.mark_install_committed();
            let finish = txn.finish();
            if finish.is_empty() {
                Ok(shipping_cache)
            } else {
                Err(format!(
                    "installed generated cache, but transaction cleanup failed: {}",
                    finish.join("; ")
                ))
            }
        }
        Err(install_error) => {
            let restore = txn.restore_install();
            let restore_clean = restore.clean();
            let mut errors = restore.errors;
            if let Err(e) = txn.remove_ephemeral_deploy_backup() {
                errors.push(e);
            }
            if restore_clean && errors.is_empty() {
                errors.extend(txn.finish());
            }
            let primary = format!("installing generated cache in place: {install_error}");
            if errors.is_empty() {
                Err(primary)
            } else {
                Err(format!("{primary}; additionally {}", errors.join("; ")))
            }
        }
    }
}

/// Launch the game with the proven AngelScript generation flags, then read the newly-created
/// `PrecompiledScript.Cache`. The caller removes that file before launch, so mere existence is a
/// fresh-run signal; the shipping cache is never the generator output.
fn real_generate_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> GeneratorRunResult<Vec<u8>> {
    let generated = real_generate_with_timeout_and_diagnostics_report(
        exe,
        g1r,
        cache,
        Duration::from_secs(30 * 60),
        diagnostics,
    );
    GeneratorRunResult {
        result: generated.result,
        process_exit: generated.process_exit,
    }
}

const GENERATOR_EXIT_UNCONFIRMED: &str = "[gore:generator-exit-unconfirmed]";

#[cfg(test)]
fn generator_exit_unconfirmed(error: &str) -> bool {
    error.contains(GENERATOR_EXIT_UNCONFIRMED)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratorProcessExitDisposition {
    NotStarted,
    Confirmed,
    Unconfirmed,
}

#[derive(Debug)]
struct GeneratorRunResult<T> {
    result: Result<T, String>,
    process_exit: GeneratorProcessExitDisposition,
}

impl<T> GeneratorRunResult<T> {
    fn not_started(result: Result<T, String>) -> Self {
        Self {
            result,
            process_exit: GeneratorProcessExitDisposition::NotStarted,
        }
    }

    fn confirmed(result: Result<T, String>) -> Self {
        Self {
            result,
            process_exit: GeneratorProcessExitDisposition::Confirmed,
        }
    }

    fn unconfirmed(error: String) -> Self {
        Self {
            result: Err(error),
            process_exit: GeneratorProcessExitDisposition::Unconfirmed,
        }
    }
}

/// Spawn/try_wait implementation with a real wall-clock deadline. Keeping the timeout injectable
/// makes the termination path testable without weakening the production 30-minute maximum.
const GENERATOR_ARGS: &[&str] = &[
    "-as-development-mode",
    "-as-generate-precompiled-data",
    "-as-skip-threaded-initialize",
    "-as-exit-on-error",
];

fn run_normal_generator_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    timeout: Duration,
) -> GeneratorRunResult<Vec<u8>> {
    let mut child = match std::process::Command::new(exe)
        .args(GENERATOR_ARGS)
        .current_dir(g1r)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return GeneratorRunResult::not_started(Err(format!("launching game: {error}")));
        }
    };

    finish_generator_child_report(&mut child, cache, timeout)
}

/// A failed hook attempt may have started the generator and written a partial development cache
/// before its process was confirmed gone. Remove that first-attempt artifact before the fallback
/// launch so the normal result cannot accidentally accept stale bytes.
fn run_clean_fallback_generator_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    timeout: Duration,
) -> GeneratorRunResult<Vec<u8>> {
    if let Err(error) = clear_partial_cache_before_fallback(cache) {
        return GeneratorRunResult::not_started(Err(error));
    }
    run_normal_generator_report(exe, g1r, cache, timeout)
}

fn clear_partial_cache_before_fallback(cache: &Path) -> Result<(), String> {
    remove_if_exists(cache)
}

fn finish_generator_child_report(
    child: &mut std::process::Child,
    cache: &Path,
    timeout: Duration,
) -> GeneratorRunResult<Vec<u8>> {
    let status = match wait_for_child_with_timeout_report(
        child,
        timeout,
        Duration::from_millis(250),
        Duration::from_secs(2),
        "AngelScript generation",
    ) {
        Ok(status) => status,
        Err(error) => {
            return match error.process_exit {
                GeneratorProcessExitDisposition::Unconfirmed => {
                    GeneratorRunResult::unconfirmed(error.message)
                }
                GeneratorProcessExitDisposition::Confirmed => {
                    GeneratorRunResult::confirmed(Err(error.message))
                }
                GeneratorProcessExitDisposition::NotStarted => {
                    GeneratorRunResult::not_started(Err(error.message))
                }
            };
        }
    };
    GeneratorRunResult::confirmed(read_completed_generated_cache(
        cache,
        status.success(),
        &status.to_string(),
    ))
}

#[derive(Debug)]
enum DiagnosticAttempt<T> {
    Completed(GeneratorDiagnosticsResult<T>),
    Disabled,
    Unavailable(String),
    Fatal(GeneratorDiagnosticsResult<T>),
}

#[derive(Debug)]
struct GeneratorDiagnosticsResult<T> {
    result: Result<T, String>,
    diagnostics: crate::diagnostics::CompilerDiagnosticsReport,
    process_exit: GeneratorProcessExitDisposition,
    /// Native-private recovery directories whose exact paths may be present in `result`.
    /// Evidence-only callers consume this list to redact those paths before crossing a wire.
    private_paths: Vec<PathBuf>,
}

/// Infrastructure failure is deliberately not a compiler failure: once the first process is
/// confirmed absent, execute the unchanged normal generator and return its result byte-for-byte.
fn resolve_diagnostic_attempt_report<T, N>(
    attempt: DiagnosticAttempt<T>,
    normal: N,
) -> GeneratorDiagnosticsResult<T>
where
    N: FnOnce() -> GeneratorRunResult<T>,
{
    match attempt {
        DiagnosticAttempt::Completed(report) | DiagnosticAttempt::Fatal(report) => report,
        DiagnosticAttempt::Disabled => {
            let normal = normal();
            GeneratorDiagnosticsResult {
                result: normal.result,
                diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::Disabled,
                ),
                process_exit: normal.process_exit,
                private_paths: Vec::new(),
            }
        }
        DiagnosticAttempt::Unavailable(reason) => {
            eprintln!(
                "gore: AngelScript diagnostics unavailable ({reason}); falling back to the normal generator"
            );
            let normal = normal();
            GeneratorDiagnosticsResult {
                result: normal.result,
                diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback,
                ),
                process_exit: normal.process_exit,
                private_paths: Vec::new(),
            }
        }
    }
}

#[cfg(test)]
fn resolve_diagnostic_attempt<T, N>(attempt: DiagnosticAttempt<T>, normal: N) -> Result<T, String>
where
    N: FnOnce() -> Result<T, String>,
{
    resolve_diagnostic_attempt_report(attempt, || {
        let result = normal();
        if result
            .as_ref()
            .err()
            .is_some_and(|error| generator_exit_unconfirmed(error))
        {
            GeneratorRunResult {
                result,
                process_exit: GeneratorProcessExitDisposition::Unconfirmed,
            }
        } else {
            GeneratorRunResult::confirmed(result)
        }
    })
    .result
}

struct DiagnosticArtifacts {
    dir: PathBuf,
    capture: PathBuf,
    status: PathBuf,
    cleanup: bool,
}

impl DiagnosticArtifacts {
    fn create() -> Result<Self, String> {
        let temp = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for suffix in 0..32u32 {
            let dir = temp.join(format!(
                "gore-as-diagnostics-{}-{stamp}-{suffix}",
                std::process::id()
            ));
            match std::fs::create_dir(&dir) {
                Ok(()) => {
                    return Ok(Self {
                        capture: dir.join("capture.txt"),
                        status: dir.join("status.txt"),
                        dir,
                        cleanup: true,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(format!(
                        "creating diagnostics temp directory {}: {e}",
                        dir.display()
                    ));
                }
            }
        }
        Err("could not reserve a unique diagnostics temp directory".into())
    }

    fn preserve(mut self) -> PathBuf {
        self.cleanup = false;
        self.dir.clone()
    }
}

impl Drop for DiagnosticArtifacts {
    fn drop(&mut self) {
        if !self.cleanup {
            return;
        }
        let _ = std::fs::remove_file(&self.capture);
        let _ = std::fs::remove_file(&self.status);
        let _ = std::fs::remove_dir(&self.dir);
    }
}

fn append_captured_diagnostics(
    generated: GeneratorRunResult<Vec<u8>>,
    artifacts: &DiagnosticArtifacts,
    disposition: crate::diagnostics::DiagnosticsCaptureDisposition,
) -> GeneratorDiagnosticsResult<Vec<u8>> {
    let GeneratorRunResult {
        result,
        process_exit,
    } = generated;
    let (capture, diagnostics, capture_failure) = match crate::diagnostics::read_bounded(
        &artifacts.capture,
        crate::diagnostics::MAX_CAPTURE_BYTES,
    ) {
        Ok((capture, truncated)) => {
            let protocol_truncated = capture.lines().any(|line| {
                line.trim_end_matches('\r') == crate::diagnostics::CAPTURE_TRUNCATED_TOKEN
            });
            let mut capture_failure =
                (truncated || protocol_truncated).then(|| "was truncated".to_owned());
            let report_disposition = if capture_failure.is_some() {
                crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
            } else {
                disposition
            };
            let diagnostics =
                match crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                    report_disposition,
                    &capture,
                ) {
                    Ok(report) => report,
                    Err(error) => {
                        capture_failure = Some(format!(
                            "could not be represented as bounded structured diagnostics ({error})"
                        ));
                        crate::diagnostics::CompilerDiagnosticsReport::empty(
                            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid,
                        )
                    }
                };
            let mut formatted = crate::diagnostics::format_capture(&capture);
            const CAPTURE_TRUNCATED: &str = "<diagnostics truncated after 8 MiB>\n";
            if truncated
                && !protocol_truncated
                && formatted.len().saturating_add(CAPTURE_TRUNCATED.len())
                    <= crate::diagnostics::MAX_FORMATTED_BYTES
            {
                formatted.push_str(CAPTURE_TRUNCATED);
            }
            (formatted, diagnostics, capture_failure)
        }
        Err(_error) if !artifacts.capture.exists() => (
            String::new(),
            crate::diagnostics::CompilerDiagnosticsReport::empty(disposition),
            None,
        ),
        Err(error) => (
            format!("<diagnostics capture unreadable: {error}>\n"),
            crate::diagnostics::CompilerDiagnosticsReport::empty(
                crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid,
            ),
            Some("could not be read".to_owned()),
        ),
    };
    let has_compiler_error = diagnostics
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnostics::DiagnosticSeverity::Error);
    let result = if capture.trim().is_empty() {
        match result {
            Ok(_) if capture_failure.is_some() => Err(format!(
                "AngelScript diagnostics capture {}; refusing to accept an unverified cache",
                capture_failure.as_deref().unwrap_or("was invalid")
            )),
            result => result,
        }
    } else {
        match result {
            Ok(_) if has_compiler_error => Err(format!(
                "AngelScript compiler reported an error despite producing a structurally complete cache\n--- AngelScript compiler diagnostics ---\n{}",
                capture.trim_end()
            )),
            Ok(_) if capture_failure.is_some() => Err(format!(
                "AngelScript diagnostics capture {}; refusing to accept an unverified cache\n--- AngelScript compiler diagnostics ---\n{}",
                capture_failure.as_deref().unwrap_or("was invalid"),
                capture.trim_end()
            )),
            Ok(bytes) => {
                eprint!("{capture}");
                Ok(bytes)
            }
            Err(error) => Err(format!(
                "{error}\n--- AngelScript compiler diagnostics ---\n{}",
                capture.trim_end()
            )),
        }
    };
    GeneratorDiagnosticsResult {
        result,
        diagnostics,
        process_exit,
        private_paths: Vec::new(),
    }
}

fn preserve_unconfirmed_diagnostic_attempt(
    error: String,
    artifacts: DiagnosticArtifacts,
    prep: crate::diagnostics::HookPreparation,
) -> DiagnosticAttempt<Vec<u8>> {
    let diagnostics_dir = artifacts.preserve();
    let helper_dir = prep.preserve_owned();
    let helper_note = helper_dir
        .as_deref()
        .map(|path| format!(", embedded helper directory {}", path.display()))
        .unwrap_or_default();
    DiagnosticAttempt::Fatal(GeneratorDiagnosticsResult {
        result: Err(format!(
            "{error}; process exit is unconfirmed, so diagnostics files were intentionally preserved at {}{}",
            diagnostics_dir.display(),
            helper_note
        )),
        diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
            crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed,
        ),
        process_exit: GeneratorProcessExitDisposition::Unconfirmed,
        private_paths: std::iter::once(diagnostics_dir)
            .chain(helper_dir)
            .collect(),
    })
}

fn classify_hooked_result(
    generated: GeneratorRunResult<Vec<u8>>,
    artifacts: DiagnosticArtifacts,
    prep: crate::diagnostics::HookPreparation,
) -> DiagnosticAttempt<Vec<u8>> {
    let generated = match generated.process_exit {
        GeneratorProcessExitDisposition::Unconfirmed => {
            // The child may still own and append to the capture. Preserve the whole directory for
            // recovery, but never read or expose a snapshot as if it were a completed report.
            return preserve_unconfirmed_diagnostic_attempt(
                generated.result.unwrap_err(),
                artifacts,
                prep,
            );
        }
        _ => generated,
    };
    let report = append_captured_diagnostics(
        generated,
        &artifacts,
        crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
    );
    DiagnosticAttempt::Completed(report)
}

fn classify_started_hook_termination(
    termination: ChildWaitFailure,
    artifacts: DiagnosticArtifacts,
    prep: crate::diagnostics::HookPreparation,
) -> DiagnosticAttempt<Vec<u8>> {
    if termination.process_exit == GeneratorProcessExitDisposition::Unconfirmed {
        preserve_unconfirmed_diagnostic_attempt(termination.message, artifacts, prep)
    } else {
        DiagnosticAttempt::Unavailable(termination.message)
    }
}

fn real_generate_with_timeout_and_diagnostics_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    timeout: Duration,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> GeneratorDiagnosticsResult<Vec<u8>> {
    if diagnostics.disabled {
        return resolve_diagnostic_attempt_report(DiagnosticAttempt::Disabled, || {
            run_clean_fallback_generator_report(exe, g1r, cache, timeout)
        });
    }
    let prep = match crate::diagnostics::prepare_hook(exe, diagnostics) {
        Ok(prep) => prep,
        Err(reason) => {
            return resolve_diagnostic_attempt_report(
                DiagnosticAttempt::Unavailable(reason),
                || run_clean_fallback_generator_report(exe, g1r, cache, timeout),
            );
        }
    };
    let artifacts = match DiagnosticArtifacts::create() {
        Ok(artifacts) => artifacts,
        Err(reason) => {
            return resolve_diagnostic_attempt_report(
                DiagnosticAttempt::Unavailable(reason),
                || run_clean_fallback_generator_report(exe, g1r, cache, timeout),
            );
        }
    };
    let attempt = match crate::diagnostics::spawn_hooked(
        exe,
        g1r,
        GENERATOR_ARGS,
        &prep,
        &artifacts.capture,
        &artifacts.status,
        diagnostics.inject_delay,
    ) {
        Ok(crate::diagnostics::HookSpawnOutcome::Hooked(mut child)) => {
            let result = finish_generator_child_report(&mut child, cache, timeout);
            classify_hooked_result(result, artifacts, prep)
        }
        Ok(crate::diagnostics::HookSpawnOutcome::ExitedBeforeInjection(mut child)) => {
            let generated = finish_generator_child_report(&mut child, cache, timeout);
            if generated.process_exit == GeneratorProcessExitDisposition::Unconfirmed {
                preserve_unconfirmed_diagnostic_attempt(
                    generated.result.unwrap_err(),
                    artifacts,
                    prep,
                )
            } else {
                DiagnosticAttempt::Completed(append_captured_diagnostics(
                    generated,
                    &artifacts,
                    crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableWithoutFallback,
                ))
            }
        }
        Ok(crate::diagnostics::HookSpawnOutcome::ExitedAfterInjectionBeforeReady {
            child,
            status,
        }) => {
            // `try_wait` already confirmed direct-process exit. Keep the child handle alive until
            // this point, then discard the injected attempt and let run_clean_fallback_generator
            // remove any partial cache before relaunching without diagnostics.
            drop(child);
            DiagnosticAttempt::Unavailable(format!(
                "generator exited after diagnostics injection but before helper readiness ({status})"
            ))
        }
        Err(crate::diagnostics::HookSpawnError::SafeFallback(reason)) => {
            DiagnosticAttempt::Unavailable(reason)
        }
        Err(crate::diagnostics::HookSpawnError::Started { mut child, reason }) => {
            match child.try_wait() {
                Ok(Some(status)) => DiagnosticAttempt::Unavailable(format!(
                    "{reason}; first generator already exited ({status})"
                )),
                _ => {
                    let termination = terminate_child_bounded_report(
                        &mut child,
                        &reason,
                        Duration::from_millis(20),
                        Duration::from_secs(5),
                    );
                    classify_started_hook_termination(termination, artifacts, prep)
                }
            }
        }
    };
    resolve_diagnostic_attempt_report(attempt, || {
        run_clean_fallback_generator_report(exe, g1r, cache, timeout)
    })
}

/// Read and structurally validate the generator output. The shipping game build used by G1R exits
/// with status 1 after a successful `-as-generate-precompiled-data` run, so process status alone is
/// not an acceptance signal. Conversely, a merely present/non-empty file is unsafe after an error:
/// accept it only when every module and all seven tail tables parse exactly to EOF.
fn read_completed_generated_cache(
    cache: &Path,
    status_success: bool,
    status_label: &str,
) -> Result<Vec<u8>, String> {
    let bytes = read_regular_file_bounded_no_follow(
        cache,
        MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES,
        "generated PrecompiledScript.Cache",
    )
    .map_err(|error| {
        format!(
            "AngelScript generation exited with {status_label} but produced no usable {}: {error}",
            cache.display()
        )
    })?;
    validate_generated_cache(&bytes).map_err(|e| {
        if status_success {
            e
        } else {
            format!(
                "AngelScript generation exited unsuccessfully ({status_label}) and its output was \
                 incomplete: {e}"
            )
        }
    })?;
    Ok(bytes)
}

fn read_regular_file_bounded_no_follow(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    let mut file = open_regular_file_no_follow_read(path)?;
    let length = file
        .metadata()
        .map_err(|error| format!("inspecting {label}: {error}"))?
        .len();
    if length > max_bytes {
        return Err(format!(
            "{label} has {length} bytes; maximum is {max_bytes}"
        ));
    }
    let capacity = usize::try_from(length)
        .map_err(|_| format!("{label} length cannot be represented on this platform"))?;
    let read_limit = max_bytes
        .checked_add(1)
        .ok_or_else(|| format!("{label} read limit overflowed"))?;
    let mut bytes = Vec::with_capacity(capacity);
    Read::by_ref(&mut file)
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("reading {label}: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("{label} exceeded {max_bytes} bytes while reading"));
    }
    Ok(bytes)
}

/// Wait for a direct child up to a hard execution deadline. On timeout or polling failure, request
/// termination and observe it only for the separately bounded `termination_grace`; this function
/// never calls blocking `Child::wait` after a failed kill (or at all).
#[cfg(test)]
fn wait_for_child_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
    poll_interval: Duration,
    termination_grace: Duration,
    context: &str,
) -> Result<std::process::ExitStatus, String> {
    wait_for_child_with_timeout_report(child, timeout, poll_interval, termination_grace, context)
        .map_err(|failure| failure.message)
}

#[derive(Debug)]
struct ChildWaitFailure {
    message: String,
    process_exit: GeneratorProcessExitDisposition,
}

fn wait_for_child_with_timeout_report(
    child: &mut std::process::Child,
    timeout: Duration,
    poll_interval: Duration,
    termination_grace: Duration,
    context: &str,
) -> Result<std::process::ExitStatus, ChildWaitFailure> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| ChildWaitFailure {
            message: format!("{context} timeout is too large"),
            process_exit: GeneratorProcessExitDisposition::Unconfirmed,
        })?;
    let poll_interval = poll_interval.max(Duration::from_millis(1));
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    let cause = format!("{context} exceeded the {timeout:?} timeout");
                    return Err(terminate_child_bounded_report(
                        child,
                        &cause,
                        poll_interval,
                        termination_grace,
                    ));
                }
                std::thread::sleep(poll_interval.min(deadline.saturating_duration_since(now)));
            }
            Err(e) => {
                let cause = format!("waiting for {context}: {e}");
                return Err(terminate_child_bounded_report(
                    child,
                    &cause,
                    poll_interval,
                    termination_grace,
                ));
            }
        }
    }
}

/// Process-tree termination with a bounded observation window. On Windows, `taskkill /T /F` first
/// handles descendants; `Child::kill` remains the direct-child fallback on every platform. An
/// unconfirmed exit is marked so transaction owners preserve isolation instead of racing cleanup.
fn terminate_child_bounded_report(
    child: &mut std::process::Child,
    cause: &str,
    poll_interval: Duration,
    termination_grace: Duration,
) -> ChildWaitFailure {
    let pid = child.id();
    let deadline = Instant::now()
        .checked_add(termination_grace)
        .unwrap_or_else(Instant::now);
    let tree = request_process_tree_termination(pid, deadline);
    let kill_error = child.kill().err();
    let poll_interval = poll_interval.max(Duration::from_millis(1));
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !tree.confirmed {
                    return ChildWaitFailure {
                        message: format!(
                            "{GENERATOR_EXIT_UNCONFIRMED} {cause}; direct child {pid} exited during \
                         termination ({status}), but descendant termination was not confirmed \
                         ({}). Isolation must remain in place",
                            tree.note
                        ),
                        process_exit: GeneratorProcessExitDisposition::Unconfirmed,
                    };
                }
                let message = match kill_error {
                    Some(kill_error) => format!(
                        "{cause}; child {pid} exited during termination ({status}; direct kill \
                         reported: {kill_error}; {})",
                        tree.note
                    ),
                    None => format!(
                        "{cause}; process tree rooted at child {pid} was terminated ({status}; \
                         {})",
                        tree.note
                    ),
                };
                return ChildWaitFailure {
                    message,
                    process_exit: GeneratorProcessExitDisposition::Confirmed,
                };
            }
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    let kill_note = kill_error
                        .as_ref()
                        .map(|e| format!("direct kill reported: {e}"))
                        .unwrap_or_else(|| "direct kill was requested".into());
                    return ChildWaitFailure {
                        message: format!(
                            "{GENERATOR_EXIT_UNCONFIRMED} {cause}; termination was requested for \
                         process tree {pid}, but exit was not observed within \
                         {termination_grace:?} ({kill_note}; {})",
                            tree.note
                        ),
                        process_exit: GeneratorProcessExitDisposition::Unconfirmed,
                    };
                }
                std::thread::sleep(poll_interval.min(deadline.saturating_duration_since(now)));
            }
            Err(e) => {
                return ChildWaitFailure {
                    message: format!(
                        "{GENERATOR_EXIT_UNCONFIRMED} {cause}; termination was requested for process \
                     tree {pid}, but querying its exit failed: {e} ({})",
                        tree.note
                    ),
                    process_exit: GeneratorProcessExitDisposition::Unconfirmed,
                };
            }
        }
    }
}

struct TreeTermination {
    confirmed: bool,
    note: String,
}

#[cfg(windows)]
fn request_process_tree_termination(pid: u32, deadline: Instant) -> TreeTermination {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut killer = match std::process::Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(killer) => killer,
        Err(e) => {
            return TreeTermination {
                confirmed: false,
                note: format!("process-tree terminator could not start: {e}"),
            };
        }
    };
    loop {
        match killer.try_wait() {
            Ok(Some(status)) if status.success() => {
                return TreeTermination {
                    confirmed: true,
                    note: "taskkill confirmed process-tree termination".into(),
                };
            }
            Ok(Some(status)) => {
                return TreeTermination {
                    confirmed: false,
                    note: format!("taskkill exited unsuccessfully: {status}"),
                };
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = killer.kill();
                return TreeTermination {
                    confirmed: false,
                    note: "taskkill did not finish within the termination grace".into(),
                };
            }
            Err(e) => {
                return TreeTermination {
                    confirmed: false,
                    note: format!("querying taskkill failed: {e}"),
                };
            }
        }
    }
}

#[cfg(not(windows))]
fn request_process_tree_termination(_pid: u32, _deadline: Instant) -> TreeTermination {
    TreeTermination {
        confirmed: true,
        note: "platform uses the direct child as the generation process tree".into(),
    }
}

/// Recursively copy `src` into `dst`, recording every destination FILE path written into `out`
/// together with its PRIOR bytes (`None` if it didn't exist, `Some(bytes)` if the copy overwrote a
/// pre-existing file) — so the caller can delete what it created and RESTORE what it overwrote.
/// Directories created are not recorded individually — empty ones are pruned bottom-up by
/// [`restore_or_remove`].
fn copy_tree(
    src: &Path,
    dst: &Path,
    out: &mut Vec<(PathBuf, Option<Vec<u8>>)>,
) -> std::io::Result<()> {
    copy_tree_with(src, dst, out, &mut |from, to| {
        std::fs::copy(from, to).map(|_| ())
    })
}

fn copy_tree_with<C>(
    src: &Path,
    dst: &Path,
    out: &mut Vec<(PathBuf, Option<Vec<u8>>)>,
    copy_file: &mut C,
) -> std::io::Result<()>
where
    C: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    ensure_real_directory(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let source_meta = std::fs::symlink_metadata(entry.path())?;
        if metadata_is_link_or_reparse(&source_meta) {
            return Err(std::io::Error::other(format!(
                "refusing linked/reparse source path {}",
                entry.path().display()
            )));
        }
        let to = dst.join(entry.file_name());
        if source_meta.is_dir() {
            copy_tree_with(&entry.path(), &to, out, copy_file)?;
        } else if source_meta.is_file() {
            // Capture the pre-existing bytes (if any) BEFORE overwriting, so cleanup can restore a
            // user's own loose script that happens to share this path with the emitted tree.
            let prior = match std::fs::symlink_metadata(&to) {
                Ok(meta) => {
                    if metadata_is_link_or_reparse(&meta) {
                        return Err(std::io::Error::other(format!(
                            "refusing linked/reparse destination path {}",
                            to.display()
                        )));
                    }
                    if !meta.is_file() {
                        return Err(std::io::Error::other(format!(
                            "destination path is not a regular file: {}",
                            to.display()
                        )));
                    }
                    Some(std::fs::read(&to)?)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e),
            };
            // Register rollback BEFORE copy: std::fs::copy may truncate/create the destination and
            // then fail, so recording only after success would leak a partial file.
            out.push((to.clone(), prior));
            copy_file(&entry.path(), &to)?;
        } else {
            return Err(std::io::Error::other(format!(
                "source path is not a regular file or directory: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

/// Rust's ordinary metadata follows links. Compile staging is a privileged write into the live
/// game tree, so reject symlinks and (on Windows) every reparse point, including junctions.
fn metadata_is_link_or_reparse(meta: &std::fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Create a directory one component at a time while proving every existing component is a real
/// directory. `create_dir_all` would otherwise traverse a pre-existing symlink/junction before the
/// caller gets a chance to inspect it.
fn ensure_real_directory(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if metadata_is_link_or_reparse(&meta) {
                return Err(std::io::Error::other(format!(
                    "refusing linked/reparse destination directory {}",
                    path.display()
                )));
            }
            if !meta.is_dir() {
                return Err(std::io::Error::other(format!(
                    "destination path is not a directory: {}",
                    path.display()
                )));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(|| {
                std::io::Error::other(format!(
                    "cannot create destination directory without a parent: {}",
                    path.display()
                ))
            })?;
            if parent.as_os_str().is_empty() {
                ensure_real_directory(Path::new("."))?;
            } else if parent != path {
                ensure_real_directory(parent)?;
            }
            match std::fs::create_dir(path) {
                Ok(()) => Ok(()),
                // Another actor may have created it between inspection and creation. Re-inspect
                // rather than accepting a newly-planted reparse point.
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    ensure_real_directory(path)
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

/// Undo a [`copy_tree`]: for each recorded destination, RESTORE its original bytes if it pre-existed
/// (`Some`) or DELETE it if this copy created it (`None`); then remove any directories that became
/// empty as a result, walking UP toward (but never past) `root`. `root` itself is left in place.
/// A dir still holding a restored pre-existing (or other) file stays non-empty and survives, so
/// pre-existing content is never lost.
///
/// Attempts ALL files (and dirs) even if some fail — cleanup must be maximal — but AGGREGATES any
/// file restore/delete failures into the returned `Err` so a caller can report a polluted install.
/// Directory-prune failures are NOT errors: a dir staying non-empty (e.g. it holds a restored file)
/// is the expected, correct outcome, so empty-dir removal stays best-effort.
fn restore_or_remove(written: &[(PathBuf, Option<Vec<u8>>)], root: &Path) -> Result<(), String> {
    use std::collections::BTreeSet;
    // Restore-or-remove the files first, collecting (not short-circuiting on) failures so every
    // file is attempted before we report.
    let mut errs: Vec<String> = Vec::new();
    for (f, prior) in written {
        match prior {
            Some(bytes) => {
                if let Err(e) = std::fs::write(f, bytes) {
                    errs.push(format!("restore {}: {e}", f.display()));
                }
            }
            None => {
                if let Err(e) = std::fs::remove_file(f) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        errs.push(format!("delete {}: {e}", f.display()));
                    }
                }
            }
        }
    }
    // Collect candidate parent dirs (deepest first via reverse-sorted full paths), bounded to
    // strict descendants of `root`, then try to remove each empty one bottom-up. Removing a child
    // can empty its parent, so seed parents transitively up to `root`.
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for (f, _) in written {
        let mut p = f.parent();
        while let Some(dir) = p {
            if dir == root || !dir.starts_with(root) {
                break;
            }
            dirs.insert(dir.to_path_buf());
            p = dir.parent();
        }
    }
    // Deepest paths sort last; remove in reverse so children go before parents.
    for dir in dirs.iter().rev() {
        // `remove_dir` only succeeds on an empty dir — a restored pre-existing file keeps it alive,
        // so a failure here is expected and NOT aggregated.
        let _ = std::fs::remove_dir(dir);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::model::{Class, Func, Module};
    use crate::cache::types::DataType;

    static PROCESS_TIMEOUT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_function(name: &str) -> Func {
        Func {
            name: name.into(),
            namespace: String::new(),
            param_defaults: Vec::new(),
            ret: DataType::default(),
            params: Vec::new(),
            bytecode: Vec::new(),
            obj_locals: Vec::new(),
            is_ufunction: false,
            traits: 0,
        }
    }

    #[test]
    fn edit_preflight_identifies_only_omitted_generated_class_methods() {
        let modules = vec![Module {
            name: "QuestModule".into(),
            file: "QuestModule.as".into(),
            functions: Vec::new(),
            classes: vec![Class {
                name: "UQuestFixture".into(),
                namespace: String::new(),
                super_class: None,
                fields: Vec::new(),
                methods: vec![test_function("Tick"), test_function("__InitDefaults")],
                ctors: Vec::new(),
                flags: 0,
            }],
            enums: Vec::new(),
            globals: Vec::new(),
        }];

        assert_eq!(
            omitted_generated_methods(&modules, "QuestModule").unwrap(),
            ["UQuestFixture::__InitDefaults"]
        );
        assert!(omitted_generated_methods(&modules, "Missing")
            .unwrap_err()
            .to_string()
            .contains("expected exactly one base module, found 0"));
        assert!(prepare_generated_defaults_edit(
            "add",
            &modules,
            "QuestModule",
            &[],
            "default Foo = 1;",
            true,
        )
        .unwrap()
        .is_none());
        let error = prepare_generated_defaults_edit(
            "edit",
            &modules,
            "QuestModule",
            &[],
            "class UQuestFixture {}",
            true,
        )
        .expect_err("edit must not mix carried defaults with new-symbol remap")
        .to_string();
        assert!(error.contains("UQuestFixture::__InitDefaults"), "{error}");
        assert!(error.contains("strict base-keyspace remap"), "{error}");
    }

    #[test]
    fn edit_preflight_never_treats_ambiguous_identities_as_an_empty_inventory() {
        let module = |classes| Module {
            name: "QuestModule".into(),
            file: "QuestModule.as".into(),
            functions: Vec::new(),
            classes,
            enums: Vec::new(),
            globals: Vec::new(),
        };
        let class = |name: &str, methods| Class {
            name: name.into(),
            namespace: String::new(),
            super_class: None,
            fields: Vec::new(),
            methods,
            ctors: Vec::new(),
            flags: 0,
        };

        let duplicate_modules = vec![module(Vec::new()), module(Vec::new())];
        let error = prepare_generated_defaults_edit(
            "edit",
            &duplicate_modules,
            "QuestModule",
            &[],
            "",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("expected exactly one base module, found 2"),
            "{error}"
        );

        let duplicate_classes = vec![module(vec![
            class("UQuestFixture", Vec::new()),
            class("UQuestFixture", vec![test_function("__InitDefaults")]),
        ])];
        let error = prepare_generated_defaults_edit(
            "edit",
            &duplicate_classes,
            "QuestModule",
            &[],
            "",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("duplicate class identity"), "{error}");

        let duplicate_methods = vec![module(vec![class(
            "UQuestFixture",
            vec![
                test_function("__InitDefaults"),
                test_function("__InitDefaults"),
            ],
        )])];
        let error = prepare_generated_defaults_edit(
            "edit",
            &duplicate_methods,
            "QuestModule",
            &[],
            "",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("duplicate generated method identity"),
            "{error}"
        );

        // `add` never consults the base-module inventory, so new-module authoring remains normal.
        assert!(prepare_generated_defaults_edit(
            "add",
            &duplicate_modules,
            "QuestModule",
            &[],
            "default Foo = 1;",
            true,
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn generated_default_source_gate_ignores_literals_and_comments_but_not_code() {
        assert!(!source_contains_default_token(
            r#"// default Foo = 1;
               string A = "default";
               FName B = n"default";
               /* ordinary default comment */
               void NodefaultValue() {}"#,
        )
        .unwrap());
        assert!(source_contains_default_token("/* /* */ default Health = 100; // */").unwrap());
        assert!(source_contains_default_token("default Health = 100;").unwrap());
        assert!(!source_contains_default_token("switch (X) { default: break; }").unwrap());
        assert!(
            !source_contains_default_token("switch (X) { default /* label */ : break; }").unwrap()
        );
        assert!(source_contains_default_token("default /* CDO */ Health = 100;").unwrap());
        assert!(source_contains_default_token("/* unterminated")
            .unwrap_err()
            .contains("unterminated block comment"));
        assert!(source_contains_default_token("string X = \"unterminated")
            .unwrap_err()
            .contains("unterminated quoted literal"));
    }

    #[test]
    fn g1r_dir_appends_or_keeps() {
        assert_eq!(
            g1r_dir(Path::new("games/Gothic")),
            PathBuf::from("games/Gothic/G1R")
        );
        for spelling in ["G1R", "g1r", "G1r", "g1R"] {
            let direct = PathBuf::from("games/Gothic").join(spelling);
            assert_eq!(g1r_dir(&direct), direct);
            assert_eq!(game_root_dir(&direct), PathBuf::from("games/Gothic"));
        }
    }

    #[test]
    fn mixed_case_direct_g1r_and_install_root_share_one_mutation_lock() {
        let root = unique_test_root("mixed-case-g1r-lock");
        std::fs::create_dir_all(&root).unwrap();
        let direct = root.join("g1R");
        assert_eq!(
            install_mutation_lock_path(&direct),
            install_mutation_lock_path(&root)
        );

        let mut guard = InstallMutationGuard::acquire(&direct, "gore-as:compile").unwrap();
        let blocked = InstallMutationGuard::acquire(&root, "gore-mod:deploy")
            .expect_err("the semantic install root must contend on the direct-G1R lock");
        assert!(
            blocked.contains("install mutation is active"),
            "got: {blocked}"
        );
        assert_eq!(guard.path(), root.join(".gore-install-mutation.lock"));
        guard.release().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn precompile_errors_when_exe_missing() {
        // No shipping exe: the guard fires and the generator is NEVER invoked.
        let dir = std::env::temp_dir().join("gore-as-no-exe-xyz");
        let opts = PrecompileOpts {
            game_dir: dir,
            src: None,
            out: None,
            backup: true,
        };
        let err = precompile_with(&opts, |_, _, _| panic!("must not launch")).unwrap_err();
        assert!(err.contains("game exe not found"), "got: {err}");
    }

    /// A fake install under `base`: a stub shipping exe (so the exists()-guard passes) and a live
    /// cache holding `OLD`. Returns (game_dir, cache_path).
    fn fake_install(base: &Path) -> (PathBuf, PathBuf) {
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        std::fs::create_dir_all(&win64).unwrap();
        std::fs::write(win64.join("G1R-Win64-Shipping.exe"), b"stub").unwrap();
        let script = base.join("G1R").join("Script");
        std::fs::create_dir_all(&script).unwrap();
        let cache = script.join("PrecompiledScript_Shipping.Cache");
        std::fs::write(&cache, b"OLD").unwrap();
        (base.to_path_buf(), cache)
    }

    fn unique_test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "gore-as-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn install_compile_probe_injected_process_states_are_fail_closed_and_bounded() {
        let root = unique_test_root("install-state-process-probe");

        let closed = probe_install_compile_state_with(&root, || Ok(false));
        assert_eq!(
            closed.disposition,
            InstallCompileStateDisposition::SafeToCompile
        );
        assert!(closed.safe_to_compile);
        assert_eq!(
            closed.game_process,
            InstallCompileGameProcessDisposition::NotRunning
        );
        assert!(closed.artifacts.is_empty());
        assert!(closed.issues.is_empty());

        let running = probe_install_compile_state_with(&root, || Ok(true));
        assert_eq!(
            running.disposition,
            InstallCompileStateDisposition::GameProcessRunning
        );
        assert!(!running.safe_to_compile);
        assert_eq!(
            running.game_process,
            InstallCompileGameProcessDisposition::Running
        );

        let oversized = "enumeration failed ".repeat(INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);
        let failed = probe_install_compile_state_with(&root, || Err(oversized));
        assert_eq!(
            failed.disposition,
            InstallCompileStateDisposition::InspectionFailed
        );
        assert!(!failed.safe_to_compile);
        assert_eq!(
            failed.game_process,
            InstallCompileGameProcessDisposition::InspectionFailed
        );
        assert_eq!(failed.issues.len(), 1);
        assert_eq!(
            failed.issues[0].kind,
            InstallCompileInspectionIssueKind::GameProcessEnumeration
        );
        assert!(failed.issues[0].message_truncated);
        assert!(failed.issues[0].message.len() <= INSTALL_COMPILE_PROBE_MESSAGE_LIMIT);

        let oversized_path = PathBuf::from("x".repeat(INSTALL_COMPILE_PROBE_PATH_LIMIT + 128));
        let (display, truncated) = bounded_probe_path(&oversized_path);
        assert!(truncated);
        assert!(display.len() <= INSTALL_COMPILE_PROBE_PATH_LIMIT);
    }

    #[test]
    fn install_compile_probe_reports_every_known_artifact_without_mutation() {
        let root = unique_test_root("install-state-artifacts");
        let (game, shipping) = fake_install(&root);
        let mutation_lock = install_mutation_lock_path(&game);
        let compile_lock = compile_lock_path(&game);
        let journal = recovery_journal_path(&game);
        let shipping_backup = compile_bak_path(&shipping);
        let jitted_backup = append_suffix(&root.join("AS_JITTED_CODE"), ".gore-compile-bak");
        let proxy = root
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("dwmapi.dll");
        let proxy_backup = append_suffix(&proxy, ".gore-compile-bak");

        std::fs::write(&mutation_lock, b"mutation-lock").unwrap();
        std::fs::write(&compile_lock, b"compile-lock").unwrap();
        std::fs::create_dir(&journal).unwrap();
        std::fs::write(journal.join("marker"), b"journal").unwrap();
        std::fs::write(&shipping_backup, b"shipping").unwrap();
        std::fs::create_dir(&jitted_backup).unwrap();
        std::fs::write(jitted_backup.join("marker"), b"jitted").unwrap();
        std::fs::write(&proxy_backup, b"proxy").unwrap();

        let probe = probe_install_compile_state_with(&game, || Ok(false));
        assert_eq!(
            probe.disposition,
            InstallCompileStateDisposition::RecoveryArtifactsPresent
        );
        assert!(!probe.safe_to_compile);
        assert_eq!(probe.artifacts.len(), 6);
        assert_eq!(
            probe
                .artifacts
                .iter()
                .map(|artifact| artifact.kind)
                .collect::<Vec<_>>(),
            [
                InstallCompileArtifactKind::InstallMutationLock,
                InstallCompileArtifactKind::CompileLock,
                InstallCompileArtifactKind::RecoveryJournal,
                InstallCompileArtifactKind::ShippingCacheBackup,
                InstallCompileArtifactKind::JittedCodeBackup,
                InstallCompileArtifactKind::Ue4ssProxyBackup,
            ]
        );
        assert!(probe
            .artifacts
            .iter()
            .all(|artifact| artifact.path.len() <= INSTALL_COMPILE_PROBE_PATH_LIMIT));
        assert_eq!(std::fs::read(&mutation_lock).unwrap(), b"mutation-lock");
        assert_eq!(std::fs::read(&compile_lock).unwrap(), b"compile-lock");
        assert_eq!(std::fs::read(journal.join("marker")).unwrap(), b"journal");
        assert_eq!(std::fs::read(&shipping_backup).unwrap(), b"shipping");
        assert_eq!(
            std::fs::read(jitted_backup.join("marker")).unwrap(),
            b"jitted"
        );
        assert_eq!(std::fs::read(&proxy_backup).unwrap(), b"proxy");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_guard_binds_release_to_the_owned_record() {
        let root = unique_test_root("install-mutation-replaced-owner");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);
        let mut guard = InstallMutationGuard::acquire(&game, "gore-as:test").unwrap();
        #[cfg(windows)]
        {
            assert!(
                std::fs::remove_file(&path).is_err(),
                "the retained Windows handle must deny disappearance of the owned record"
            );
            assert!(
                std::fs::write(&path, b"version=1\nowner=other\npid=1\n").is_err(),
                "the retained Windows handle must deny ownership-record replacement"
            );
            guard.release().unwrap();
            assert!(!path.exists());
        }
        #[cfg(not(windows))]
        {
            std::fs::remove_file(&path).unwrap();
            let disappeared = guard.release().unwrap_err();
            assert!(
                disappeared.contains("current install-mutation lock identity"),
                "got: {disappeared}"
            );
            std::fs::write(&path, b"version=1\nowner=other\npid=1\n").unwrap();
            let error = guard.release().unwrap_err();
            assert!(
                error.contains("filesystem identity changed"),
                "got: {error}"
            );
            guard.preserve_for_manual_recovery();
            assert_eq!(
                std::fs::read(&path).unwrap(),
                b"version=1\nowner=other\npid=1\n"
            );
            std::fs::remove_file(&path).unwrap();
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_payload_reader_rejects_oversized_content_without_unbounded_read() {
        let root = unique_test_root("install-mutation-bounded-read");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("oversized.lock");
        std::fs::write(&path, vec![b'x'; INSTALL_MUTATION_RECORD_LIMIT + 1]).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let error = read_install_mutation_payload(&mut file, &path).unwrap_err();
        assert!(error.contains("oversized"), "got: {error}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_guard_is_bounded_exclusive_and_raii() {
        let root = unique_test_root("install-mutation-guard");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);

        for invalid in [
            "",
            "contains newline\n",
            &"x".repeat(INSTALL_MUTATION_OWNER_LIMIT + 1),
        ] {
            let error = InstallMutationGuard::acquire(&game, invalid)
                .expect_err("invalid owner must be rejected");
            assert!(error.contains("owner must be"), "got: {error}");
            assert!(!path.exists());
        }

        {
            let guard = InstallMutationGuard::acquire(&game, "gore-as:test").unwrap();
            assert_eq!(guard.owner(), "gore-as:test");
            assert_eq!(guard.pid(), std::process::id());
            assert_eq!(guard.path(), path);
            let payload = std::fs::read_to_string(&path).unwrap();
            assert!(payload.starts_with(&format!(
                "version=1\nowner=gore-as:test\npid={}\nguard_id={}-",
                std::process::id(),
                std::process::id()
            )));
            assert!(payload.ends_with('\n'));
            assert!(payload.len() < INSTALL_MUTATION_OWNER_LIMIT + 128);

            let error = InstallMutationGuard::acquire(&game, "gore-mod:deploy")
                .expect_err("a second owner must be rejected");
            assert!(error.contains("install mutation is active"), "got: {error}");
            assert!(path.exists());
        }
        assert!(!path.exists(), "dropping the guard must release the lock");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_takeover_distinguishes_active_abandoned_and_explicit_release() {
        let root = unique_test_root("install-mutation-takeover-manager");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);
        let guard = InstallMutationGuard::acquire(&game, "gore-mod:manager-apply").unwrap();
        let guard_id = guard.guard_id().to_owned();
        assert!(matches!(
            InstallMutationGuard::take_over_abandoned_manager(&game),
            InstallMutationTakeover::Busy
        ));

        guard.preserve_for_manual_recovery();
        let abandoned = match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Owned(abandoned) => abandoned,
            other => panic!("expected abandoned Manager lock, got {other:?}"),
        };
        assert_eq!(abandoned.guard_id(), guard_id);
        assert!(matches!(
            InstallMutationGuard::take_over_abandoned_manager(&game),
            InstallMutationTakeover::Busy
        ));
        drop(abandoned);
        assert!(
            path.exists(),
            "default abandoned drop must preserve evidence"
        );

        let mut abandoned = match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Owned(abandoned) => abandoned,
            other => panic!("expected repeat abandoned takeover, got {other:?}"),
        };
        abandoned.release().unwrap();
        assert!(
            !path.exists(),
            "explicit release must remove the exact lock"
        );
        assert!(matches!(
            InstallMutationGuard::take_over_abandoned_manager(&game),
            InstallMutationTakeover::Missing
        ));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_takeover_never_adopts_compile_legacy_or_unknown_owners() {
        let root = unique_test_root("install-mutation-takeover-owner-classes");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);

        let guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let compile_id = guard.guard_id().to_owned();
        guard.preserve_for_manual_recovery();
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::CompileOwner { owner, guard_id } => {
                assert_eq!(owner, "gore-as:compile");
                assert_eq!(guard_id, compile_id);
            }
            other => panic!("expected compile classification, got {other:?}"),
        }
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();

        let guard = InstallMutationGuard::acquire(&game, "gore-mod:deploy").unwrap();
        guard.preserve_for_manual_recovery();
        let legacy = match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::LegacyAmbiguous(abandoned) => abandoned,
            other => panic!("expected legacy ambiguity, got {other:?}"),
        };
        drop(legacy);
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();

        let guard = InstallMutationGuard::acquire(&game, "other:writer").unwrap();
        guard.preserve_for_manual_recovery();
        assert!(matches!(
            InstallMutationGuard::take_over_abandoned_manager(&game),
            InstallMutationTakeover::Invalid { .. }
        ));
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();

        std::fs::write(&path, b"not a valid ownership record").unwrap();
        assert!(matches!(
            InstallMutationGuard::take_over_abandoned_manager(&game),
            InstallMutationTakeover::Invalid { .. }
        ));
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_takeover_rejects_hardlinked_lock_authority() {
        let root = unique_test_root("install-mutation-takeover-hardlink");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);
        let linked = root.join("linked-lock");
        let guard = InstallMutationGuard::acquire(&game, "gore-mod:manager-apply").unwrap();
        guard.preserve_for_manual_recovery();
        std::fs::hard_link(&path, &linked).unwrap();
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Invalid { message } => {
                assert!(message.contains("filesystem identity"), "{message}");
            }
            other => panic!("expected invalid hardlinked lock authority, got {other:?}"),
        }
        assert!(path.exists());
        assert!(linked.exists());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&linked).unwrap();

        let init = install_mutation_init_path(&game, "gore-mod:manager-apply", "123-456-10");
        let linked_init = root.join("linked-initialization");
        std::fs::write(&init, b"").unwrap();
        std::fs::hard_link(&init, &linked_init).unwrap();
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Invalid { message } => {
                assert!(message.contains("filesystem identity"), "{message}");
            }
            other => panic!("expected invalid hardlinked initialization, got {other:?}"),
        }
        assert!(init.exists());
        assert!(linked_init.exists());
        std::fs::remove_file(&init).unwrap();
        std::fs::remove_file(&linked_init).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn install_mutation_takeover_rejects_a_reparse_lock_authority() {
        let root = unique_test_root("install-mutation-takeover-reparse");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);
        let target = root.join("foreign-lock-target");
        std::fs::write(
            &target,
            b"version=1\nowner=gore-mod:manager-apply\npid=1\nguard_id=reparse-test\n",
        )
        .unwrap();
        match std::os::windows::fs::symlink_file(&target, &path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                std::fs::remove_dir_all(root).unwrap();
                return;
            }
            Err(error) => panic!("creating lock reparse-point fixture failed: {error}"),
        }

        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Invalid { .. } => {}
            other => panic!("expected invalid reparse lock authority, got {other:?}"),
        }
        assert!(path.exists());
        assert!(target.exists());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&target).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn install_mutation_takeover_reports_replaced_identities_as_invalid() {
        let root = unique_test_root("install-mutation-takeover-replaced-identity");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);
        let displaced = root.join("displaced-lock-for-takeover");
        let guard = InstallMutationGuard::acquire(&game, "gore-mod:manager-apply").unwrap();
        guard.preserve_for_manual_recovery();

        let displaced_for_hook = displaced.clone();
        INSTALL_MUTATION_TAKEOVER_AFTER_LOCK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |opened_path| {
                std::fs::rename(opened_path, &displaced_for_hook).unwrap();
                std::fs::write(opened_path, b"replacement lock").unwrap();
            }));
        });
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Invalid { message } => {
                assert!(message.contains("filesystem identity"), "{message}");
            }
            other => panic!("expected invalid replaced lock identity, got {other:?}"),
        }
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&displaced).unwrap();

        let init = install_mutation_init_path(&game, "gore-mod:manager-apply", "123-456-11");
        let displaced_init = root.join("displaced-initialization-for-takeover");
        std::fs::write(&init, b"").unwrap();
        let displaced_init_for_hook = displaced_init.clone();
        INSTALL_MUTATION_TAKEOVER_AFTER_LOCK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |opened_path| {
                std::fs::rename(opened_path, &displaced_init_for_hook).unwrap();
                std::fs::write(opened_path, b"replacement initialization").unwrap();
            }));
        });
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::Invalid { message } => {
                assert!(message.contains("filesystem identity"), "{message}");
            }
            other => panic!("expected invalid replaced initialization, got {other:?}"),
        }
        std::fs::remove_file(&init).unwrap();
        std::fs::remove_file(&displaced_init).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn install_mutation_directory_lock_blocks_replacement_race_and_release_preserves_replacement() {
        let root = unique_test_root("install-mutation-replaced-path");
        let (game, _) = fake_install(&root);
        let path = install_mutation_lock_path(&game);
        let displaced = root.join("displaced-lock");
        let mut guard = InstallMutationGuard::acquire(&game, "gore-mod:manager-apply").unwrap();

        std::fs::rename(&path, &displaced).unwrap();
        std::fs::write(&path, b"replacement owned by another actor").unwrap();
        let contender = InstallMutationGuard::acquire(&game, "gore-as:compile")
            .expect_err("the stable install-root lock must still serialize a replaced lock name");
        assert!(
            contender.contains("install-root ownership is held"),
            "{contender}"
        );

        let release = guard
            .release()
            .expect_err("release must not unlink a replacement at the canonical name");
        assert!(release.contains("filesystem identity changed"), "{release}");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"replacement owned by another actor"
        );
        guard.preserve_for_manual_recovery();

        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&displaced).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_mutation_takeover_clears_a_prepublication_manager_initialization_only_explicitly() {
        let root = unique_test_root("install-mutation-takeover-initialization");
        let (game, _) = fake_install(&root);
        for (guard_id, payload) in [
            ("123-456-7", b"".as_slice()),
            ("123-456-8", b"version=1\nowner=".as_slice()),
        ] {
            let init = install_mutation_init_path(&game, "gore-mod:manager-apply", guard_id);
            std::fs::write(&init, payload).unwrap();
            let mut abandoned = match InstallMutationGuard::take_over_abandoned_manager(&game) {
                InstallMutationTakeover::Owned(abandoned) => abandoned,
                other => panic!("expected abandoned initialization, got {other:?}"),
            };
            assert_eq!(abandoned.guard_id(), guard_id);
            assert!(init.exists());
            abandoned.release().unwrap();
            assert!(!init.exists());
        }

        let compile_init = install_mutation_init_path(&game, "gore-as:compile", "123-456-9");
        std::fs::write(&compile_init, b"").unwrap();
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::CompileOwner { owner, guard_id } => {
                assert_eq!(owner, "gore-as:initialization");
                assert_eq!(guard_id, "123-456-9");
            }
            other => panic!("expected empty compile initialization classification, got {other:?}"),
        }
        assert!(
            compile_init.exists(),
            "Manager recovery must never clear compiler initialization evidence"
        );
        std::fs::remove_file(&compile_init).unwrap();

        let partial_compile_id = "123-456-9-partial";
        let partial_compile_init =
            install_mutation_init_path(&game, "gore-as:compile", partial_compile_id);
        std::fs::write(&partial_compile_init, b"version=1\nowner=").unwrap();
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::CompileOwner { owner, guard_id } => {
                assert_eq!(owner, "gore-as:initialization");
                assert_eq!(guard_id, partial_compile_id);
            }
            other => {
                panic!("expected partial compile initialization classification, got {other:?}")
            }
        }
        assert!(
            partial_compile_init.exists(),
            "Manager recovery must preserve partial compiler initialization evidence"
        );
        std::fs::remove_file(&partial_compile_init).unwrap();

        let full_compile_id = "123-456-10";
        let full_compile_init =
            install_mutation_init_path(&game, "gore-as:compile", full_compile_id);
        std::fs::write(
            &full_compile_init,
            format!("version=1\nowner=gore-as:compile\npid=123\nguard_id={full_compile_id}\n"),
        )
        .unwrap();
        match InstallMutationGuard::take_over_abandoned_manager(&game) {
            InstallMutationTakeover::CompileOwner { owner, guard_id } => {
                assert_eq!(owner, "gore-as:compile");
                assert_eq!(guard_id, full_compile_id);
            }
            other => panic!("expected full compile initialization classification, got {other:?}"),
        }
        assert!(
            full_compile_init.exists(),
            "Manager recovery must preserve full compiler initialization evidence"
        );
        std::fs::remove_file(&full_compile_init).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn one_preheld_guard_spans_authoritative_read_and_compiler_transaction() {
        let root = unique_test_root("preheld-guard-read-through-transaction");
        let (game, shipping) = fake_install(&root);
        let _game_process = StatedGameProcess::not_running();
        let src = root.join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"// staged\n").unwrap();

        let guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let authoritative = std::fs::read(&shipping).unwrap();
        let report =
            game_run_regen_with_install_report_with_guard(&game, &src, guard, |_, _, _| {
                let contender = InstallMutationGuard::acquire(&game, "gore-mod:deploy")
                    .expect_err("deploy must remain blocked after the authoritative read");
                assert!(
                    contender.contains("install mutation is active"),
                    "got: {contender}"
                );
                GeneratorRunResult::confirmed(Ok(cache_with_empty_modules(&[(
                    "Generated",
                    "Generated.as",
                )])))
            })
            .unwrap();

        assert!(report.result.is_ok(), "got: {:?}", report.result);
        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RestoredExact
        );
        assert_eq!(authoritative, b"OLD");
        assert_eq!(std::fs::read(&shipping).unwrap(), authoritative);
        assert!(!install_mutation_lock_path(&game).exists());
        assert!(!recovery_journal_path(&game).exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn after_restore_hook_discards_output_and_audits_before_same_guard_release() {
        let root = unique_test_root("after-restore-same-guard-audit");
        let (game, shipping) = fake_install(&root);
        let _game_process = StatedGameProcess::not_running();
        let src = root.join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"// staged\n").unwrap();
        let guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let audit_called = std::cell::Cell::new(false);

        let report = game_run_regen_with_install_report_with_guard_and_after_restore(
            &game,
            &src,
            guard,
            |_, _, _| {
                GeneratorRunResult::confirmed(Ok(cache_with_empty_modules(&[(
                    "Generated",
                    "Generated.as",
                )])))
            },
            |result, retained_copy| {
                let path = result.as_ref().map_err(|error| error.clone())?;
                let artifact = retained_copy.ok_or_else(|| {
                    "after-restore hook lost the retained output handle".to_owned()
                })?;
                let (consumed, disposition) = consume_project_regen_artifact(&src, path, artifact);
                assert!(!consumed?.is_empty());
                assert_eq!(disposition, ProjectCompilerOutputDisposition::Discarded);
                assert!(!path.exists(), "private output must be gone before audit");
                assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
                let contender = InstallMutationGuard::acquire(&game, "gore-mod:deploy")
                    .expect_err("closing audit must still own the install guard");
                assert!(contender.contains("install mutation is active"));
                audit_called.set(true);
                Ok(())
            },
        )
        .unwrap();

        assert!(report.result.is_ok(), "got: {:?}", report.result);
        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RestoredExact
        );
        assert!(audit_called.get());
        assert!(!install_mutation_lock_path(&game).exists());
        assert!(!src.join("regen.cache").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_running_game_refuses_the_install_mutation_and_leaves_every_byte_where_it_was() {
        // Every other transaction test states a closed game, so this is the case that keeps the
        // refusal honest: state the opposite and the whole sentence a user sees has to come back,
        // with the generator never launched and not one file moved. Without it, softening the
        // guard would break nothing in this suite.
        let root = unique_test_root("stated-running-game-refusal");
        let (game, shipping) = fake_install(&root);
        let _game_process = StatedGameProcess::running();
        let src = root.join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"// staged\n").unwrap();
        let script = g1r_dir(&game).join("Script");

        let guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let error = game_run_regen_with_install_report_with_guard(&game, &src, guard, |_, _, _| {
            panic!("the generator must not launch while the game is running")
        })
        .expect_err("a running game must refuse the transaction outright");

        assert_eq!(
            error,
            "refusing install mutation while G1R-Win64-Shipping.exe is running; close the game \
             and retry",
            "the refusal must name the executable and the remedy the user has to apply"
        );
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(
            !script.join("Mod.as").exists(),
            "a refused mutation must stage nothing into the install"
        );
        assert!(!src.join("regen.cache").exists());
        assert!(
            install_compile_artifact_paths(&game)
                .into_iter()
                .all(|(_, path)| !path.exists()),
            "a refused mutation must leave no lock, journal or backup behind"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_shipped_process_inspection_still_answers_for_this_machine_without_touching_the_install()
    {
        // The seam means no transaction test enumerates processes any more, so this is the only
        // case left that runs the implementation a shipped binary actually uses. If it stopped
        // compiling or began failing on a supported host, every real compile would be refused and
        // nothing else here would notice.
        let root = unique_test_root("native-process-inspection");
        let (game, shipping) = fake_install(&root);

        let answer = native_shipping_game_process_running();
        assert!(
            answer.is_ok(),
            "the shipped inspection must answer on a supported host rather than fail: {answer:?}"
        );
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(
            install_compile_artifact_paths(&game)
                .into_iter()
                .all(|(_, path)| !path.exists()),
            "the inspection is strictly read-only"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_stated_running_game_refuses_the_acquisition_in_the_words_the_shipped_path_uses() {
        // The stated-answer acquisition is the only one another crate's tests can reach, so it is
        // the one that could quietly drift into a softer guard than the shipped path. Ask both the
        // same question and compare the whole sentence rather than a substring: a reworded or
        // downgraded refusal in either is then a failure here instead of a surprise in production.
        let root = unique_test_root("stated-process-entry-point-refusal");
        let (game, shipping) = fake_install(&root);

        let stated = acquire_compile_install_mutation_with_stated_game_process(&game, || Ok(true))
            .expect_err("a stated running game must refuse the acquisition");
        let shipped = {
            let _game_process = StatedGameProcess::running();
            acquire_compile_install_mutation(&game)
                .expect_err("a running game must refuse the acquisition")
        };
        assert_eq!(
            stated, shipped,
            "the stated-answer entry point must refuse in the same words as the shipped one"
        );
        assert_eq!(
            stated,
            "refusing AngelScript compile while G1R-Win64-Shipping.exe is running; close the game \
             and retry"
        );

        let failed = acquire_compile_install_mutation_with_stated_game_process(&game, || {
            Err("injected Toolhelp failure".to_owned())
        })
        .expect_err("a stated inspection failure must fail closed");
        assert_eq!(
            failed,
            "refusing AngelScript compile because native game-process inspection failed: injected \
             Toolhelp failure"
        );

        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(
            install_compile_artifact_paths(&game)
                .into_iter()
                .all(|(_, path)| !path.exists()),
            "a refused acquisition must leave no lock, journal or backup behind"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stating_the_game_closed_does_not_excuse_the_rest_of_the_install_preflight() {
        // Stating an answer states one fact about the machine, not that the install is safe to
        // touch. This is what would notice if the stated-answer path ever shortcut the artifact
        // scan or handed back something weaker than the cross-tool lock: a leftover recovery
        // journal must still refuse, and the guard a foreign test holds must still block a deploy.
        let root = unique_test_root("stated-closed-game-still-preflights");
        let (game, _shipping) = fake_install(&root);
        let journal = recovery_journal_path(&game);

        std::fs::write(&journal, b"{}").unwrap();
        let refused =
            acquire_compile_install_mutation_with_stated_game_process(&game, || Ok(false))
                .expect_err("a leftover recovery journal must refuse the acquisition");
        assert!(
            refused.contains("compile recovery journal already exists"),
            "got: {refused}"
        );
        std::fs::remove_file(&journal).unwrap();

        let lock = install_mutation_lock_path(&game);
        {
            let guard =
                acquire_compile_install_mutation_with_stated_game_process(&game, || Ok(false))
                    .expect("a stated closed game and a clean install must acquire the guard");
            assert_eq!(guard.path(), lock);
            let contender = InstallMutationGuard::acquire(&game, "gore-mod:deploy")
                .expect_err("a stated-answer guard must still block every other install mutation");
            assert!(
                contender.contains("install mutation is active"),
                "got: {contender}"
            );
        }
        assert!(!lock.exists(), "dropping the guard must release the lock");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compile_transaction_process_preflight_mutates_nothing_for_running_or_error() {
        for (label, checker, expected) in [
            (
                "running",
                Ok(true),
                format!("{G1R_SHIPPING_EXE_NAME} is running"),
            ),
            (
                "inspection-error",
                Err("injected Toolhelp failure".to_owned()),
                "native game-process inspection failed".to_owned(),
            ),
        ] {
            let root = unique_test_root(&format!("process-preflight-{label}"));
            let (game, shipping) = fake_install(&root);
            let g1r = g1r_dir(&game);
            let script = g1r.join("Script");
            let error =
                CompileTransaction::begin_with_process_checker(&game, &g1r, &script, || {
                    checker.clone()
                })
                .err()
                .expect("unsafe process state must be rejected");
            assert!(error.contains(&expected), "got: {error}");
            assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
            assert!(install_compile_artifact_paths(&game)
                .into_iter()
                .all(|(_, path)| !path.exists()));
            std::fs::remove_dir_all(root).unwrap();
        }

        let root = unique_test_root("process-preflight-closed");
        let (game, shipping) = fake_install(&root);
        let g1r = g1r_dir(&game);
        let script = g1r.join("Script");
        let txn =
            CompileTransaction::begin_with_process_checker(&game, &g1r, &script, || Ok(false))
                .expect("closed-game preflight should permit transaction ownership");
        assert!(install_mutation_lock_path(&game).exists());
        assert!(compile_lock_path(&game).exists());
        drop(txn);
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(install_compile_artifact_paths(&game)
            .into_iter()
            .all(|(_, path)| !path.exists()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compile_transaction_rechecks_process_after_guard_before_first_live_mutation() {
        let root = unique_test_root("process-started-after-guard");
        let (game, shipping) = fake_install(&root);
        let g1r = g1r_dir(&game);
        let script = g1r.join("Script");
        let calls = std::cell::Cell::new(0usize);
        let error = CompileTransaction::begin_with_process_checker(&game, &g1r, &script, || {
            let call = calls.get();
            calls.set(call + 1);
            Ok(call != 0)
        })
        .err()
        .expect("the post-guard process check must reject a newly running game");

        assert_eq!(calls.get(), 2, "preflight and post-guard checks must run");
        assert!(error.contains(G1R_SHIPPING_EXE_NAME), "got: {error}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(
            install_compile_artifact_paths(&game)
                .into_iter()
                .all(|(_, path)| !path.exists()),
            "the rejected post-guard check must release ownership without creating recovery state"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_guard_process_recheck_release_failure_is_recovery_dominant() {
        let root = unique_test_root("process-recheck-release-failure");
        let (game, shipping) = fake_install(&root);
        let g1r = g1r_dir(&game);
        let script = g1r.join("Script");
        let mut guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        guard.payload.push_str("mismatched-in-memory-owner");

        let failure = CompileTransaction::begin_with_mutation_guard_and_process_checker(
            &game,
            &g1r,
            &script,
            guard,
            || Ok(true),
        )
        .err()
        .expect("the final process check must reject the transaction");
        assert!(failure.recovery_required);
        assert!(failure.message.contains(G1R_SHIPPING_EXE_NAME));
        assert!(
            failure
                .message
                .contains("failed to release the pre-transaction"),
            "got: {}",
            failure.message
        );
        assert!(
            lock.exists(),
            "release failure must retain a durable blocker"
        );
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!compile_lock_path(&game).exists());
        assert!(!recovery_journal_path(&game).exists());

        std::fs::remove_file(lock).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn common_runner_preflight_release_failure_reaches_restore_disposition() {
        let root = unique_test_root("runner-preflight-release-failure");
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("src");
        std::fs::create_dir(&src).unwrap();
        let mut guard = InstallMutationGuard::acquire(&root, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        guard.payload.push_str("mismatched-in-memory-owner");

        let report =
            game_run_regen_with_install_report_with_guard(&root, &src, guard, |_, _, _| {
                panic!("missing executable must not generate")
            })
            .expect("release uncertainty is returned as a structured install report");
        assert!(report.result.unwrap_err().contains("game exe not found"));
        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        assert!(lock.exists());

        std::fs::remove_file(lock).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn both_compile_entry_points_reject_existing_recovery_before_mutation() {
        for entry_point in ["module-generator", "standalone-precompile"] {
            let root = unique_test_root(&format!("existing-recovery-{entry_point}"));
            let (game, shipping) = fake_install(&root);
            let journal = recovery_journal_path(&game);
            std::fs::create_dir(&journal).unwrap();
            std::fs::write(journal.join("KEEP"), b"RECOVERY").unwrap();
            let src = root.join("src");
            std::fs::create_dir(&src).unwrap();
            std::fs::write(src.join("Mod.as"), b"script").unwrap();

            let error = if entry_point == "module-generator" {
                game_run_regen_with(&game, &src, |_, _, _| panic!("must not generate")).unwrap_err()
            } else {
                let opts = PrecompileOpts {
                    game_dir: game.clone(),
                    src: Some(src),
                    out: Some(root.join("out.Cache")),
                    backup: false,
                };
                precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err()
            };

            assert!(
                error.contains("recovery journal already exists"),
                "entry={entry_point}: {error}"
            );
            assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
            assert_eq!(std::fs::read(journal.join("KEEP")).unwrap(), b"RECOVERY");
            assert!(!install_mutation_lock_path(&game).exists());
            assert!(!compile_lock_path(&game).exists());
            assert!(!compile_bak_path(&shipping).exists());

            std::fs::remove_dir_all(root).unwrap();
        }
    }

    fn dev_cache(shipping_cache: &Path) -> PathBuf {
        shipping_cache
            .parent()
            .unwrap()
            .join("PrecompiledScript.Cache")
    }

    fn sia(s: &str) -> Vec<u8> {
        if s.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut out = (s.len() as i32).to_le_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out
    }

    fn fstring(s: &str) -> Vec<u8> {
        let mut out = ((s.len() + 1) as i32).to_le_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out
    }

    fn cache_with_empty_modules(modules: &[(&str, &str)]) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out.extend_from_slice(&crate::cache::header::CACHE_MAGIC.to_le_bytes());
        out.extend_from_slice(&(modules.len() as u32).to_le_bytes());
        for (module, file) in modules {
            out.extend_from_slice(&fstring(module));
            out.extend_from_slice(&sia(module));
            out.extend_from_slice(&0i32.to_le_bytes()); // functions
            out.extend_from_slice(&0i32.to_le_bytes()); // classes
            out.extend_from_slice(&0i32.to_le_bytes()); // enums
            out.extend_from_slice(&0i32.to_le_bytes()); // globals
            out.extend_from_slice(&0i32.to_le_bytes()); // function imports
            out.extend_from_slice(&0i64.to_le_bytes()); // code hash
            out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
            out.extend_from_slice(&sia("")); // statics class
            out.extend_from_slice(&0i32.to_le_bytes()); // events
            out.extend_from_slice(&0i32.to_le_bytes()); // delegates
            out.extend_from_slice(&sia(file));
            out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
        }
        for _ in 0..crate::cache::tables::N_TABLES {
            out.extend_from_slice(&0i32.to_le_bytes());
        }
        out
    }

    fn push_binds_cstr(output: &mut Vec<u8>, value: &str) {
        output.extend_from_slice(&((value.len() + 1) as u32).to_le_bytes());
        output.extend_from_slice(value.as_bytes());
        output.push(0);
    }

    fn minimal_binds_cache() -> Vec<u8> {
        let mut output = 1u32.to_le_bytes().to_vec();
        push_binds_cstr(&mut output, "UNativeType");
        push_binds_cstr(&mut output, "/Script/Test.NativeType");
        output.extend_from_slice(&1u32.to_le_bytes());
        push_binds_cstr(&mut output, "void NativeCall()");
        push_binds_cstr(&mut output, "NativeCall");
        output.extend_from_slice(&[0u8; 32]);
        output
    }

    fn project_check_opts(
        root: &Path,
        overlays: &[(&str, &str, &str)],
    ) -> ProjectCompilerCheckOpts {
        ProjectCompilerCheckOpts {
            game_dir: root.join("game"),
            work_dir: root.join("work"),
            overlays: overlays
                .iter()
                .map(|(module_name, rel_path, source)| ProjectCompileOverlay {
                    module_name: (*module_name).to_owned(),
                    rel_path: (*rel_path).to_owned(),
                    source: source.as_bytes().to_vec(),
                })
                .collect(),
            base_cache: cache_with_empty_modules(&[("Base", "Base.as")]),
            binds_cache: minimal_binds_cache(),
        }
    }

    fn accepted_project_diagnostics() -> crate::diagnostics::CompilerDiagnosticsReport {
        crate::diagnostics::CompilerDiagnosticsReport::empty(
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback,
        )
    }

    fn checked_project_runner_report(regen: Vec<u8>) -> ProjectCompilerRunnerReport {
        ProjectCompilerRunnerReport {
            result: Ok(regen),
            diagnostics: Some(accepted_project_diagnostics()),
            install_restore: InstallRestoreDisposition::RestoredExact,
            output_disposition: ProjectCompilerOutputDisposition::Discarded,
            closing_audit: ProjectCompilerClosingAuditDisposition::Passed,
        }
    }

    #[test]
    fn project_check_stages_mutually_referencing_modules_together_and_runs_once() {
        let root = unique_test_root("project-check-shared-tree");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(
            &root,
            &[
                ("Project.A", "Project/A.as", "void FromA() { FromB(); }\n"),
                ("Project.B", "Project/B.as", "void FromB() { FromA(); }\n"),
            ],
        );
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let calls = std::cell::Cell::new(0usize);
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, tree, mut guard, audit| {
                calls.set(calls.get() + 1);
                assert_eq!(
                    std::fs::read_to_string(tree.join("Project/A.as")).unwrap(),
                    "void FromA() { FromB(); }\n"
                );
                assert_eq!(
                    std::fs::read_to_string(tree.join("Project/B.as")).unwrap(),
                    "void FromB() { FromA(); }\n"
                );
                audit().unwrap();
                guard.release().unwrap();
                checked_project_runner_report(cache_with_empty_modules(&[
                    ("Base", "Base.as"),
                    ("Project.A", "Project/A.as"),
                    ("Project.B", "Project/B.as"),
                ]))
            },
        );

        assert!(matches!(
            report.outcome,
            ProjectCompilerCheckOutcome::Checked
        ));
        assert_eq!(calls.get(), 1);
        assert_eq!(report.runner_invocations(), 1);
        assert_eq!(
            report.output_disposition(),
            ProjectCompilerOutputDisposition::Discarded
        );
        assert_eq!(
            report.closing_audit_disposition(),
            ProjectCompilerClosingAuditDisposition::Passed
        );
        assert!(!report.recovery_required());
        assert!(
            !format!("{report:?}").contains(&root.display().to_string()),
            "evidence report must not expose a work/output path"
        );
        assert!(!install_mutation_lock_path(&opts.game_dir).exists());
        assert!(!opts.work_dir.join("tree/regen.cache").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_rejects_all_identity_and_layout_collisions_before_runner() {
        type OverlayFixture<'a> = (&'a str, &'a str, &'a str);
        type CollisionCase<'a> = (&'a str, &'a [OverlayFixture<'a>], &'a str);
        let cases: &[CollisionCase<'_>] = &[
            (
                "namespace-casefold",
                &[
                    ("Project.Alpha", "Project/Alpha.as", "void A() {}"),
                    ("project.alpha", "project/alpha.as", "void B() {}"),
                ],
                "collide under Windows case folding",
            ),
            (
                "namespace-path-mismatch",
                &[("Project.Alpha", "Project/Elsewhere.as", "void A() {}")],
                "requires relative path",
            ),
            (
                "file-directory-ancestor",
                &[
                    ("Project.Foo", "Project/Foo.as", "void A() {}"),
                    (
                        "Project.Foo.as.Child",
                        "Project/Foo.as/Child.as",
                        "void B() {}",
                    ),
                ],
                "collide under Windows case folding as the same path or a file/directory ancestor",
            ),
        ];

        for (label, overlays, expected) in cases {
            let root = unique_test_root(&format!("project-check-collision-{label}"));
            std::fs::create_dir_all(root.join("game")).unwrap();
            let opts = project_check_opts(&root, overlays);
            let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
            let report = check_project_modules_with_guard_and_runner(
                &opts,
                guard,
                || Ok(()),
                |_, _, _, _| panic!("collision must fail before runner"),
            );
            let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
                panic!("case {label} unexpectedly passed")
            };
            assert!(
                error.to_string().contains(expected),
                "case {label}: {error}"
            );
            assert_eq!(report.runner_invocations(), 0);
            assert_eq!(
                report.install_restore_disposition(),
                InstallRestoreDisposition::NotStarted
            );
            assert!(!install_mutation_lock_path(&opts.game_dir).exists());
            std::fs::remove_dir_all(root).unwrap();
        }

        let root = unique_test_root("project-check-base-collision");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let mut opts =
            project_check_opts(&root, &[("Project.Base", "Project/Base.as", "void A() {}")]);
        opts.base_cache = cache_with_empty_modules(&[("PROJECT.BASE", "Project/Base.as")]);
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, _, _| panic!("base collision must fail before runner"),
        );
        let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
            panic!("base collision unexpectedly passed")
        };
        assert!(error.to_string().contains("collides with base module"));
        assert_eq!(report.runner_invocations(), 0);
        assert!(!install_mutation_lock_path(&opts.game_dir).exists());
        std::fs::remove_dir_all(root).unwrap();

        let root = unique_test_root("project-check-base-path-ancestor");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let mut opts = project_check_opts(
            &root,
            &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
        );
        opts.base_cache = cache_with_empty_modules(&[("BaseDirectoryFile", "Project")]);
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, _, _| panic!("base path ancestor must fail before runner"),
        );
        let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
            panic!("base path ancestor unexpectedly passed")
        };
        assert!(error.to_string().contains("file/directory ancestor"));
        assert_eq!(report.runner_invocations(), 0);
        assert!(!install_mutation_lock_path(&opts.game_dir).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_rejects_missing_extra_and_wrong_regen_modules() {
        let cases = [
            (
                "missing",
                cache_with_empty_modules(&[("Base", "Base.as")]),
                "missing expected module",
            ),
            (
                "extra",
                cache_with_empty_modules(&[
                    ("Base", "Base.as"),
                    ("Project.Alpha", "Project/Alpha.as"),
                    ("Unexpected", "Unexpected.as"),
                ]),
                "unexpected module",
            ),
            (
                "wrong-name",
                cache_with_empty_modules(&[
                    ("Base", "Base.as"),
                    ("project.alpha", "Project/Alpha.as"),
                ]),
                "does not exactly match expected",
            ),
            (
                "wrong-path",
                cache_with_empty_modules(&[
                    ("Base", "Base.as"),
                    ("Project.Alpha", "Wrong/Alpha.as"),
                ]),
                "does not match expected",
            ),
        ];

        for (label, regen, expected) in cases {
            let root = unique_test_root(&format!("project-check-manifest-{label}"));
            std::fs::create_dir_all(root.join("game")).unwrap();
            let opts = project_check_opts(
                &root,
                &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
            );
            let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
            let report = check_project_modules_with_guard_and_runner(
                &opts,
                guard,
                || Ok(()),
                |_, _, mut guard, audit| {
                    audit().unwrap();
                    guard.release().unwrap();
                    checked_project_runner_report(regen)
                },
            );
            let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
                panic!("case {label} unexpectedly passed")
            };
            assert!(
                error.to_string().contains(expected),
                "case {label}: {error}"
            );
            assert_eq!(report.runner_invocations(), 1);
            assert!(!report.recovery_required());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn project_check_rejects_error_diagnostics_even_with_complete_cache() {
        let root = unique_test_root("project-check-error-diagnostic");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(
            &root,
            &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
        );
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, mut guard, audit| {
                audit().unwrap();
                guard.release().unwrap();
                ProjectCompilerRunnerReport {
                    result: Ok(cache_with_empty_modules(&[
                        ("Base", "Base.as"),
                        ("Project.Alpha", "Project/Alpha.as"),
                    ])),
                    diagnostics: Some(
                        crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
                            "=== Project/Alpha.as ===\n(1:1) [E] Broken call\n",
                        )
                        .unwrap(),
                    ),
                    install_restore: InstallRestoreDisposition::RestoredExact,
                    output_disposition: ProjectCompilerOutputDisposition::Discarded,
                    closing_audit: ProjectCompilerClosingAuditDisposition::Passed,
                }
            },
        );
        assert!(matches!(
            report.outcome,
            ProjectCompilerCheckOutcome::Failed(_)
        ));
        assert_eq!(report.diagnostics().unwrap().diagnostics().len(), 1);
        assert_eq!(report.runner_invocations(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_public_report_rejects_every_non_authorizing_diagnostics_disposition() {
        use crate::diagnostics::DiagnosticsCaptureDisposition as D;

        for disposition in [
            D::Disabled,
            D::UnavailableWithoutFallback,
            D::CaptureInvalid,
            D::ProcessExitUnconfirmed,
        ] {
            let root = unique_test_root(&format!(
                "project-check-diagnostics-disposition-{disposition:?}"
            ));
            std::fs::create_dir_all(root.join("game")).unwrap();
            let opts = project_check_opts(
                &root,
                &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
            );
            let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
            let report = check_project_modules_with_guard_and_runner(
                &opts,
                guard,
                || Ok(()),
                |_, _, mut guard, audit| {
                    audit().unwrap();
                    guard.release().unwrap();
                    ProjectCompilerRunnerReport {
                        result: Ok(cache_with_empty_modules(&[
                            ("Base", "Base.as"),
                            ("Project.Alpha", "Project/Alpha.as"),
                        ])),
                        diagnostics: Some(crate::diagnostics::CompilerDiagnosticsReport::empty(
                            disposition,
                        )),
                        install_restore: InstallRestoreDisposition::RestoredExact,
                        output_disposition: ProjectCompilerOutputDisposition::Discarded,
                        closing_audit: ProjectCompilerClosingAuditDisposition::Passed,
                    }
                },
            );
            let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
                panic!("{disposition:?} unexpectedly authorized project evidence")
            };
            assert!(
                error
                    .to_string()
                    .contains("cannot authorize project evidence"),
                "{disposition:?}: {error}"
            );
            assert_eq!(report.runner_invocations(), 1);
            std::fs::remove_dir_all(root).unwrap();
        }

        let root = unique_test_root("project-check-missing-diagnostics");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(
            &root,
            &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
        );
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, mut guard, audit| {
                audit().unwrap();
                guard.release().unwrap();
                ProjectCompilerRunnerReport {
                    result: Ok(cache_with_empty_modules(&[
                        ("Base", "Base.as"),
                        ("Project.Alpha", "Project/Alpha.as"),
                    ])),
                    diagnostics: None,
                    install_restore: InstallRestoreDisposition::RestoredExact,
                    output_disposition: ProjectCompilerOutputDisposition::Discarded,
                    closing_audit: ProjectCompilerClosingAuditDisposition::Passed,
                }
            },
        );
        let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
            panic!("missing diagnostics unexpectedly authorized project evidence")
        };
        assert!(error
            .to_string()
            .contains("without a diagnostics disposition"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_public_report_accepts_captured_and_fallback_diagnostics() {
        use crate::diagnostics::DiagnosticsCaptureDisposition as D;

        for disposition in [D::Captured, D::UnavailableFallback] {
            let root = unique_test_root(&format!(
                "project-check-authorizing-diagnostics-{disposition:?}"
            ));
            std::fs::create_dir_all(root.join("game")).unwrap();
            let opts = project_check_opts(
                &root,
                &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
            );
            let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
            let report = check_project_modules_with_guard_and_runner(
                &opts,
                guard,
                || Ok(()),
                |_, _, mut guard, audit| {
                    audit().unwrap();
                    guard.release().unwrap();
                    ProjectCompilerRunnerReport {
                        result: Ok(cache_with_empty_modules(&[
                            ("Base", "Base.as"),
                            ("Project.Alpha", "Project/Alpha.as"),
                        ])),
                        diagnostics: Some(crate::diagnostics::CompilerDiagnosticsReport::empty(
                            disposition,
                        )),
                        install_restore: InstallRestoreDisposition::RestoredExact,
                        output_disposition: ProjectCompilerOutputDisposition::Discarded,
                        closing_audit: ProjectCompilerClosingAuditDisposition::Passed,
                    }
                },
            );
            assert!(
                matches!(report.outcome, ProjectCompilerCheckOutcome::Checked),
                "{disposition:?} should authorize an error-free project check"
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn project_check_preflight_explicitly_releases_unused_guard() {
        let root = unique_test_root("project-check-preflight-guard-release");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(&root, &[]);
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, _, _| panic!("empty preflight must not run"),
        );
        assert!(matches!(
            report.outcome,
            ProjectCompilerCheckOutcome::Failed(_)
        ));
        assert_eq!(report.runner_invocations(), 0);
        assert_eq!(
            report.output_disposition(),
            ProjectCompilerOutputDisposition::NotCreated
        );
        assert_eq!(
            report.closing_audit_disposition(),
            ProjectCompilerClosingAuditDisposition::Passed
        );
        assert!(!lock.exists(), "preflight must explicitly release guard");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_preflight_guard_release_failure_is_recovery_dominant() {
        let root = unique_test_root("project-check-preflight-guard-release-failure");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(&root, &[]);
        let mut guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        guard.payload.push_str("mismatched-in-memory-owner");

        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, _, _| panic!("empty preflight must not run"),
        );
        let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
            panic!("release failure unexpectedly passed")
        };
        assert!(error
            .to_string()
            .contains("failed to release the pre-held project compiler guard"));
        assert_eq!(
            report.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        assert_eq!(
            report.output_disposition(),
            ProjectCompilerOutputDisposition::RecoveryRetained
        );
        assert!(report.recovery_required());
        assert!(
            lock.exists(),
            "failed ownership release must remain observable"
        );
        std::fs::remove_file(lock).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_closing_audit_failure_is_reported_before_guard_release() {
        let root = unique_test_root("project-check-closing-audit-failure");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(&root, &[]);
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        let audit_saw_lock = std::cell::Cell::new(false);
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || {
                audit_saw_lock.set(lock.exists());
                Err("sealed game input changed".to_owned())
            },
            |_, _, _, _| panic!("empty preflight must not run"),
        );
        let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
            panic!("failed closing audit unexpectedly passed")
        };
        assert!(audit_saw_lock.get(), "audit must run before guard release");
        assert!(error.to_string().contains("sealed game input changed"));
        assert_eq!(
            report.closing_audit_disposition(),
            ProjectCompilerClosingAuditDisposition::Failed
        );
        assert!(
            !lock.exists(),
            "ordinary audit failure must still release guard"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_preserves_runner_recovery_disposition_and_rejects_success() {
        let root = unique_test_root("project-check-runner-recovery");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let opts = project_check_opts(
            &root,
            &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
        );
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, mut guard, _audit| {
                guard.release().unwrap();
                ProjectCompilerRunnerReport {
                    result: Ok(cache_with_empty_modules(&[
                        ("Base", "Base.as"),
                        ("Project.Alpha", "Project/Alpha.as"),
                    ])),
                    diagnostics: Some(accepted_project_diagnostics()),
                    install_restore:
                        InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed,
                    output_disposition: ProjectCompilerOutputDisposition::Discarded,
                    closing_audit: ProjectCompilerClosingAuditDisposition::NotRun,
                }
            },
        );
        assert!(matches!(
            report.outcome,
            ProjectCompilerCheckOutcome::Failed(_)
        ));
        assert_eq!(
            report.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
        );
        assert_eq!(
            report.output_disposition(),
            ProjectCompilerOutputDisposition::RecoveryRetained
        );
        assert!(report.recovery_required());
        assert_eq!(report.runner_invocations(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_bounds_aggregate_sources_before_runner() {
        let root = unique_test_root("project-check-source-bound");
        std::fs::create_dir_all(root.join("game")).unwrap();
        let mut opts = project_check_opts(
            &root,
            &[("Project.Alpha", "Project/Alpha.as", "void A() {}")],
        );
        opts.overlays[0].source = vec![b'x'; MAX_PROJECT_COMPILER_CHECK_SOURCE_BYTES + 1];
        let guard = InstallMutationGuard::acquire(&opts.game_dir, "gore-as:compile").unwrap();
        let report = check_project_modules_with_guard_and_runner(
            &opts,
            guard,
            || Ok(()),
            |_, _, _, _| panic!("oversized source must fail before runner"),
        );
        let ProjectCompilerCheckOutcome::Failed(ref error) = report.outcome else {
            panic!("oversized source unexpectedly passed")
        };
        assert!(error.to_string().contains("project source bytes exceed"));
        assert_eq!(report.runner_invocations(), 0);
        assert!(!install_mutation_lock_path(&opts.game_dir).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_check_consumes_and_neutralizes_private_regen_copy() {
        let root = unique_test_root("project-check-output-disposal");
        let tree = root.join("tree");
        std::fs::create_dir_all(&tree).unwrap();
        let path = tree.join("regen.cache");
        let expected = cache_with_empty_modules(&[("Base", "Base.as")]);
        std::fs::write(&path, &expected).unwrap();

        let (result, disposition) = consume_project_regen_copy(&tree, &path);
        assert_eq!(result.unwrap(), expected);
        assert_eq!(disposition, ProjectCompilerOutputDisposition::Discarded);
        assert!(!path.exists(), "no usable compiler output may survive");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_regen_create_new_refuses_raced_hardlink_without_clobbering_target() {
        let root = unique_test_root("project-check-raced-hardlink");
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("unrelated.cache");
        let output = root.join("regen.cache");
        let original = b"unrelated cache bytes";
        std::fs::write(&target, original).unwrap();
        std::fs::hard_link(&target, &output).unwrap();

        let error = create_project_regen_artifact(&output)
            .expect_err("create-new output must reject a raced hardlink");
        assert!(error.contains("create-new"), "got: {error}");
        assert_eq!(
            std::fs::read(&target).unwrap(),
            original,
            "refusing the raced output must not truncate or overwrite its target"
        );
        let (cleanup, disposition) = discard_project_regen_copy(&output);
        assert!(cleanup.is_err(), "missing exact handle must fail closed");
        assert_eq!(
            disposition,
            ProjectCompilerOutputDisposition::RecoveryRetained
        );
        assert!(
            !output.exists(),
            "the unexpected link name should be removed"
        );
        assert_eq!(
            std::fs::read(&target).unwrap(),
            original,
            "path-only cleanup must never truncate a hardlink target"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    /// Small but structurally complete cache: one empty module followed by all seven empty tails.
    fn valid_cache() -> Vec<u8> {
        cache_with_empty_modules(&[("TestModule", "TestModule.as")])
    }

    /// Stub generator: emulate the game creating a complete development cache and return it.
    fn gen_new(_exe: &Path, _g1r: &Path, cache: &Path) -> Result<Vec<u8>, String> {
        assert_eq!(cache.file_name().unwrap(), "PrecompiledScript.Cache");
        assert!(
            !cache.exists(),
            "caller must remove a stale development cache first"
        );
        let bytes = valid_cache();
        std::fs::write(cache, &bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    #[test]
    fn generated_cache_validation_requires_header_modules_all_tails_and_eof() {
        let good = valid_cache();
        validate_generated_cache(&good).unwrap();

        let mut bad_magic = good.clone();
        bad_magic[0x10..0x14].copy_from_slice(&0u32.to_le_bytes());
        assert!(validate_generated_cache(&bad_magic)
            .unwrap_err()
            .contains("header"));

        let mut no_modules = vec![0u8; 16];
        no_modules.extend_from_slice(&crate::cache::header::CACHE_MAGIC.to_le_bytes());
        no_modules.extend_from_slice(&0u32.to_le_bytes());
        for _ in 0..crate::cache::tables::N_TABLES {
            no_modules.extend_from_slice(&0u32.to_le_bytes());
        }
        assert!(validate_generated_cache(&no_modules)
            .unwrap_err()
            .contains("zero modules"));

        assert!(validate_generated_cache(&good[..good.len() - 1])
            .unwrap_err()
            .contains("tail tables"));

        let mut trailing = good;
        trailing.push(0);
        assert!(validate_generated_cache(&trailing)
            .unwrap_err()
            .contains("file length"));
    }

    #[test]
    fn compile_target_and_layout_preflight_never_reset_or_run_regen_on_rejection() {
        let duplicate_paths =
            cache_with_empty_modules(&[("Alpha", "Dir/Foo.as"), ("Beta", "dir\\foo.AS")]);
        let prefix_paths =
            cache_with_empty_modules(&[("PrefixFile", "Foo.as"), ("PrefixChild", "foo.AS/Bar.as")]);
        let existing_directory = cache_with_empty_modules(&[("Nested", "Existing/Child.as")]);
        let cases = [
            (duplicate_paths, "add", "New", "New.as", "module layout"),
            (prefix_paths, "add", "New", "New.as", "module layout"),
            (
                valid_cache(),
                "edit",
                "TestModule",
                "Wrong.as",
                "does not match",
            ),
            (valid_cache(), "add", "testmodule", "New.as", "module name"),
            (
                valid_cache(),
                "add",
                "NewModule",
                "testmodule.AS",
                "add path",
            ),
            (
                valid_cache(),
                "add",
                "NewModule",
                "New?.as",
                "unsafe Windows output path",
            ),
            (
                valid_cache(),
                "add",
                "NewModule",
                "testmodule.AS/Child.as",
                "file/directory ancestor",
            ),
            (
                existing_directory,
                "add",
                "NewModule",
                "EXISTING",
                "file/directory ancestor",
            ),
        ];

        for (case, (base_cache, op, module_name, rel_path, expected)) in
            cases.into_iter().enumerate()
        {
            let root = std::env::temp_dir().join(format!(
                "gore-as-compile-preflight-{}-{case}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            let tree = root.join("tree");
            std::fs::create_dir_all(&tree).unwrap();
            let sentinel = tree.join("sentinel.as");
            std::fs::write(&sentinel, b"keep").unwrap();
            let source = root.join("overlay.as");
            std::fs::write(&source, b"// overlay").unwrap();
            let opts = CompileOpts {
                game_dir: root.join("game"),
                op: op.into(),
                module_name: module_name.into(),
                rel_path: rel_path.into(),
                as_path: source,
                source_override: None,
                work_dir: root.clone(),
                allow_new_symbols: false,
                base_override: Some(base_cache),
                binds_override: None,
            };
            let called = std::cell::Cell::new(false);
            let error = compile_module(&opts, |_, _| {
                called.set(true);
                Err("regen must not run".into())
            })
            .unwrap_err();

            assert!(error.to_string().contains(expected), "{error}");
            assert!(!called.get(), "regen callback ran for case {case}");
            assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
            assert!(!root.join("module.cache").exists());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn mini_guid_is_canonicalized_to_base_without_changing_payload() {
        let mut mini = valid_cache();
        let mut base = mini.clone();
        mini[..16].fill(0x11);
        base[..16].fill(0xa5);
        let remainder = mini[16..].to_vec();
        let published_before =
            crate::cache::splice::replace_module(&base, &mini, "TestModule").unwrap();

        canonicalize_mini_guid(&mut mini, &base).unwrap();

        assert_eq!(&mini[..16], &base[..16]);
        assert_eq!(&mini[16..], remainder);
        let published_after =
            crate::cache::splice::replace_module(&base, &mini, "TestModule").unwrap();
        assert_eq!(published_after, published_before);
        assert_eq!(&published_after[..16], &base[..16]);
        assert!(canonicalize_mini_guid(&mut mini[..8], &base).is_err());
    }

    #[test]
    fn nonzero_generator_status_accepts_only_a_complete_cache() {
        let base = std::env::temp_dir().join("gore-as-nonzero-complete-cache");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let cache = base.join("PrecompiledScript.Cache");
        let good = valid_cache();
        std::fs::write(&cache, &good).unwrap();

        assert_eq!(
            read_completed_generated_cache(&cache, false, "exit code: 1").unwrap(),
            good,
            "G1R's post-generation exit code 1 is acceptable only with a fully valid cache"
        );

        std::fs::write(&cache, b"partial").unwrap();
        let err = read_completed_generated_cache(&cache, false, "exit code: 1").unwrap_err();
        assert!(err.contains("exited unsuccessfully"), "got: {err}");
        assert!(err.contains("incomplete"), "got: {err}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn generated_cache_reader_rejects_oversized_sparse_file_before_allocation() {
        let root = unique_test_root("generated-cache-size-bound");
        std::fs::create_dir_all(&root).unwrap();
        let cache = root.join("PrecompiledScript.Cache");
        std::fs::File::create(&cache)
            .unwrap()
            .set_len(MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES + 1)
            .unwrap();

        let error = read_completed_generated_cache(&cache, true, "exit code: 0")
            .expect_err("oversized live generator output must fail before reading");
        assert!(
            error.contains(&format!(
                "maximum is {MAX_PROJECT_COMPILER_CHECK_REGEN_BYTES}"
            )),
            "got: {error}"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reset_compile_tree_removes_stale_scripts_and_rebuilds_empty_directory() {
        let base = std::env::temp_dir().join("gore-as-reset-tree");
        let _ = std::fs::remove_dir_all(&base);
        let tree = base.join("tree");
        std::fs::create_dir_all(tree.join("Old")).unwrap();
        std::fs::write(tree.join("Old").join("Stale.as"), b"stale").unwrap();

        let rebuilt = reset_compile_tree(&base).unwrap();
        assert_eq!(rebuilt, tree);
        assert!(rebuilt.is_dir());
        assert_eq!(std::fs::read_dir(&rebuilt).unwrap().count(), 0);

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Invoked only as a subprocess by `child_wait_timeout_is_hard_and_bounded`. Keeping it ignored
    /// prevents an ordinary test run from sleeping; the private environment flag prevents even an
    /// explicit `--ignored` run from doing so accidentally.
    #[test]
    #[ignore = "subprocess helper for the timeout test"]
    fn timeout_helper_process() {
        if std::env::var_os("GORE_AS_TIMEOUT_HELPER_PROCESS").is_some() {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn child_wait_timeout_is_hard_and_bounded() {
        let _serial = PROCESS_TIMEOUT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let test_exe = std::env::current_exe().unwrap();
        let mut child = std::process::Command::new(test_exe)
            .args([
                "--ignored",
                "--exact",
                "compile::tests::timeout_helper_process",
                "--test-threads=1",
            ])
            .env("GORE_AS_TIMEOUT_HELPER_PROCESS", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();

        let timeout = Duration::from_millis(150);
        let termination_grace = Duration::from_millis(750);
        let started = Instant::now();
        let err = wait_for_child_with_timeout(
            &mut child,
            timeout,
            Duration::from_millis(10),
            termination_grace,
            "timeout-test helper",
        )
        .unwrap_err();
        let elapsed = started.elapsed();

        assert!(err.contains("exceeded"), "got: {err}");
        assert!(
            err.contains("terminated") || err.contains(GENERATOR_EXIT_UNCONFIRMED),
            "timeout must report either confirmed termination or the fail-closed unconfirmed-exit marker; got: {err}"
        );
        assert!(
            elapsed <= timeout + termination_grace + Duration::from_secs(1),
            "timeout path exceeded its hard bounds: elapsed={elapsed:?}, error={err}"
        );
        // A heavily loaded Windows host can consume the deliberately short production-observation
        // window inside `taskkill` itself. The production path correctly reports that as
        // unconfirmed and preserves isolation. Give the test helper a separate best-effort cleanup
        // window so a valid fail-closed result neither leaks the sleeper nor poisons the serial
        // mutex for the next test.
        let cleanup_deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() && Instant::now() < cleanup_deadline {
            let _ = child.kill();
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            child.try_wait().unwrap().is_some(),
            "direct helper child must exit during the test-only cleanup window"
        );
    }

    #[test]
    fn game_run_regen_quarantines_jitted_code_and_clear_ue4ss_proxy_then_restores() {
        let base = std::env::temp_dir().join("gore-as-game-isolation");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir_all(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"OLD-JIT").unwrap();
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        let payload = win64.join("ue4ss").join("UE4SS.dll");
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, b"UE4SS").unwrap();
        let proxy = win64.join("dwmapi.dll");
        std::fs::write(&proxy, b"OLD-PROXY").unwrap();

        game_run_regen_with(&game, &src, |_, _, dev| {
            assert!(!jitted.exists(), "old JIT dir must be quarantined");
            assert!(!proxy.exists(), "UE4SS proxy must be disabled");
            std::fs::create_dir_all(&jitted).unwrap();
            std::fs::write(jitted.join("new.bin"), b"NEW-JIT").unwrap();
            std::fs::write(&proxy, b"UNEXPECTED-NEW-PROXY").unwrap();
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap();

        assert_eq!(std::fs::read(jitted.join("old.bin")).unwrap(), b"OLD-JIT");
        assert!(!jitted.join("new.bin").exists());
        assert_eq!(std::fs::read(&proxy).unwrap(), b"OLD-PROXY");
        assert!(!append_suffix(&jitted, ".gore-compile-bak").exists());
        assert!(!append_suffix(&proxy, ".gore-compile-bak").exists());
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_restores_absent_side_effect_paths_on_failure() {
        let base = std::env::temp_dir().join("gore-as-game-isolation-absent");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let jitted = base.join("AS_JITTED_CODE");
        let proxy = base
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("dwmapi.dll");

        let err = game_run_regen_with(&game, &src, |_, _, dev| {
            std::fs::create_dir_all(&jitted).unwrap();
            std::fs::write(jitted.join("new.bin"), b"NEW-JIT").unwrap();
            std::fs::write(&proxy, b"NEW-PROXY").unwrap();
            std::fs::write(dev, b"partial").unwrap();
            Err("generation failed".into())
        })
        .unwrap_err();

        assert!(err.contains("generation failed"), "got: {err}");
        assert!(!jitted.exists());
        assert!(!proxy.exists());
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn regen_diagnostics_report_rejects_existing_recovery_before_runner_invocation() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-before-runner-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let _game_process = StatedGameProcess::not_running();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        // The reserved path is an existing recovery artifact. Both report surfaces must now reject
        // it during the read-only preflight, before transaction ownership or a diagnostics runner.
        let jitted = base.join("AS_JITTED_CODE");
        let collision = append_suffix(&jitted, ".gore-compile-bak");
        std::fs::create_dir(&collision).unwrap();

        let error = game_run_regen_with_diagnostics_report(&game, &src, &Default::default())
            .expect_err("the public report requires a diagnostics-runner disposition");
        assert!(
            error.contains("compile backup already exists"),
            "got: {error}"
        );

        let extended_error =
            game_run_regen_with_extended_diagnostics_report(&game, &src, &Default::default())
                .expect_err("preflight rejection must remain outside the runner report");
        assert!(
            extended_error.contains("compile backup already exists"),
            "got: {extended_error}"
        );
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!compile_bak_path(&shipping).exists());
        assert!(!recovery_journal_path(&game).exists());
        assert!(!compile_lock_path(&game).exists());
        assert!(!install_mutation_lock_path(&game).exists());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn partial_isolation_begin_failure_retains_recovery_and_never_reports_exact_restore() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-partial-isolation-failure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"OLD-JIT").unwrap();
        let jitted_backup = append_suffix(&jitted, ".gore-compile-bak");
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        let payload = win64.join("ue4ss").join("UE4SS.dll");
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, b"UE4SS").unwrap();
        let proxy = win64.join("dwmapi.dll");
        std::fs::write(&proxy, b"OLD-PROXY").unwrap();

        let generator_calls = std::cell::Cell::new(0);
        let report = game_run_regen_with_install_report_and(
            &game,
            &src,
            || Ok(false),
            |txn| {
                txn.begin_isolation_after_jitted(|| {
                    // JIT has been moved. Make proxy activation fail, then block JIT restoration
                    // with the wrong path type so the partial begin cannot clean itself up.
                    std::fs::remove_file(&proxy).unwrap();
                    std::fs::write(&jitted, b"RESTORE-BLOCKER").unwrap();
                })
            },
            |_, _, _| {
                generator_calls.set(generator_calls.get() + 1);
                GeneratorRunResult::confirmed(Ok(valid_cache()))
            },
        )
        .unwrap();

        assert_eq!(generator_calls.get(), 0, "runner must not be invoked");
        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        let error = report.result.unwrap_err();
        assert!(error.contains("quarantining"), "got: {error}");
        assert!(
            error.contains("failed to restore generation isolation"),
            "got: {error}"
        );
        assert!(jitted_backup.exists(), "JIT recovery must remain");
        assert_eq!(
            std::fs::read(&jitted_backup.join("old.bin")).unwrap(),
            b"OLD-JIT"
        );
        assert_eq!(std::fs::read(compile_bak_path(&shipping)).unwrap(), b"OLD");
        assert!(
            recovery_journal_path(&game).exists(),
            "journal must not be retired after a failed isolation restore"
        );
        assert!(
            !compile_lock_path(&game).exists(),
            "no process started, so manual recovery must not retain the compile lock"
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn regen_report_marks_confirmed_syntax_failure_as_restored_exact() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-syntax-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let _game_process = StatedGameProcess::not_running();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Broken.as"), b"void Broken( {").unwrap();

        let report = game_run_regen_with_install_report(&game, &src, |_, _, _| {
            GeneratorRunResult::confirmed(Err("AngelScript syntax/regen failure".to_owned()))
        })
        .unwrap();

        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RestoredExact
        );
        assert!(report.result.unwrap_err().contains("syntax/regen failure"));
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!compile_bak_path(&shipping).exists());
        assert!(!recovery_journal_path(&game).exists());
        assert!(!compile_lock_path(&game).exists());
        assert!(!shipping.parent().unwrap().join("Broken.as").exists());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn regen_report_marks_structured_unconfirmed_exit_as_recovery_required() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-unconfirmed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let _game_process = StatedGameProcess::not_running();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let report = game_run_regen_with_install_report(&game, &src, |_, _, _| {
            // Deliberately omit the legacy text marker: the disposition must drive recovery.
            GeneratorRunResult::unconfirmed("simulated generator still alive".to_owned())
        })
        .unwrap();

        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
        );
        assert!(report.result.unwrap_err().contains("intentionally NOT run"));
        assert!(compile_bak_path(&shipping).exists());
        assert!(recovery_journal_path(&game).exists());
        assert!(compile_lock_path(&game).exists());
        assert!(shipping.parent().unwrap().join("Mod.as").exists());

        // The fake runner has no real process. Removing the isolated fixture is its test-only
        // equivalent of following the retained recovery instructions.
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn regen_report_marks_failed_restore_with_retained_backup_as_recovery_required() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-restore-failure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let _game_process = StatedGameProcess::not_running();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Broken.as"), b"void Broken( {").unwrap();

        let report = game_run_regen_with_install_report(&game, &src, |_, _, _| {
            std::fs::remove_file(&shipping).unwrap();
            std::fs::create_dir(&shipping).unwrap();
            GeneratorRunResult::confirmed(Err("AngelScript syntax/regen failure".to_owned()))
        })
        .unwrap();

        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        let error = report.result.unwrap_err();
        assert!(error.contains("syntax/regen failure"), "got: {error}");
        assert!(error.contains("FAILED to restore"), "got: {error}");
        let recovery = compile_bak_path(&shipping);
        assert_eq!(std::fs::read(&recovery).unwrap(), b"OLD");
        assert!(recovery_journal_path(&game).exists());
        assert!(
            !compile_lock_path(&game).exists(),
            "a confirmed-dead generator must release the lock for manual recovery"
        );

        std::fs::remove_dir(&shipping).unwrap();
        std::fs::rename(&recovery, &shipping).unwrap();
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn game_run_regen_leaves_unidentified_dwmapi_untouched() {
        let base = std::env::temp_dir().join("gore-as-game-non-ue4ss-proxy");
        let _ = std::fs::remove_dir_all(&base);
        let (game, _) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let proxy = base
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("dwmapi.dll");
        std::fs::write(&proxy, b"NOT-KNOWN-TO-BE-UE4SS").unwrap();

        game_run_regen_with(&game, &src, |_, _, dev| {
            assert_eq!(std::fs::read(&proxy).unwrap(), b"NOT-KNOWN-TO-BE-UE4SS");
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap();
        assert_eq!(std::fs::read(&proxy).unwrap(), b"NOT-KNOWN-TO-BE-UE4SS");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_refuses_existing_compile_and_quarantine_backups() {
        for which in ["cache", "jitted"] {
            let base = std::env::temp_dir().join(format!("gore-as-backup-collision-{which}"));
            let _ = std::fs::remove_dir_all(&base);
            let (game, shipping) = fake_install(&base);
            let src = base.join("src");
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(src.join("Mod.as"), b"script").unwrap();
            let collision = if which == "cache" {
                compile_bak_path(&shipping)
            } else {
                append_suffix(&base.join("AS_JITTED_CODE"), ".gore-compile-bak")
            };
            if which == "jitted" {
                std::fs::create_dir_all(base.join("AS_JITTED_CODE")).unwrap();
            }
            std::fs::write(&collision, b"KEEP-ME").unwrap();

            let err = game_run_regen_with(&game, &src, |_, _, _| panic!("must not generate"))
                .unwrap_err();
            assert!(
                err.contains("backup already exists"),
                "which={which}: {err}"
            );
            assert_eq!(std::fs::read(&collision).unwrap(), b"KEEP-ME");
            assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[test]
    fn game_run_regen_keeps_compile_backup_if_shipping_restore_fails() {
        let base = std::env::temp_dir().join("gore-as-backup-restore-failure");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let err = game_run_regen_with(&game, &src, |_, _, dev| {
            std::fs::remove_file(&shipping).unwrap();
            std::fs::create_dir(&shipping).unwrap();
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap_err();

        let backup = compile_bak_path(&shipping);
        assert!(err.contains("FAILED to restore"), "got: {err}");
        assert_eq!(std::fs::read(&backup).unwrap(), b"OLD");
        assert!(shipping.is_dir());

        // Manual recovery for the fake install, mirroring the error's instruction.
        std::fs::remove_dir(&shipping).unwrap();
        std::fs::rename(&backup, &shipping).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_refuses_existing_shipping_recovery_backup_without_mutation() {
        let base = std::env::temp_dir().join("gore-as-precompile-recovery-collision");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let backup = compile_bak_path(&shipping);
        std::fs::write(&backup, b"KEEP-RECOVERY").unwrap();
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: Some(base.join("out.Cache")),
            backup: false,
        };

        let err = precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(err.contains("compile backup already exists"), "got: {err}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert_eq!(std::fs::read(&backup).unwrap(), b"KEEP-RECOVERY");
        assert!(!compile_lock_path(&game).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_preserves_recovery_backup_when_shipping_restore_fails() {
        let base = std::env::temp_dir().join("gore-as-precompile-restore-failure");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: Some(base.join("out.Cache")),
            backup: false,
        };

        let err = precompile_with(&opts, |_, _, dev| {
            std::fs::remove_file(&shipping).unwrap();
            std::fs::create_dir(&shipping).unwrap();
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap_err();

        let recovery = compile_bak_path(&shipping);
        assert!(err.contains("FAILED to restore"), "got: {err}");
        assert_eq!(std::fs::read(&recovery).unwrap(), b"OLD");
        assert!(shipping.is_dir());
        assert!(!compile_lock_path(&game).exists());

        std::fs::remove_dir(&shipping).unwrap();
        std::fs::rename(&recovery, &shipping).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn common_compile_lock_rejects_parallel_cross_entry_point_compile() {
        let _serial = PROCESS_TIMEOUT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let base = std::env::temp_dir().join("gore-as-parallel-compile-lock");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let game_for_thread = game.clone();
        let src_for_thread = src.clone();

        let first = std::thread::spawn(move || {
            game_run_regen_with(&game_for_thread, &src_for_thread, |_, _, dev| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                let bytes = valid_cache();
                std::fs::write(dev, &bytes).unwrap();
                Ok(bytes)
            })
        });
        entered_rx.recv().unwrap();
        assert!(compile_lock_path(&game).exists());
        assert!(install_mutation_lock_path(&game).exists());

        let second_opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: Some(base.join("parallel-out.Cache")),
            backup: false,
        };
        let second =
            precompile_with(&second_opts, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(
            second.contains("install mutation is active"),
            "got: {second}"
        );

        release_tx.send(()).unwrap();
        first.join().unwrap().unwrap();
        assert!(!compile_lock_path(&game).exists());
        assert!(!install_mutation_lock_path(&game).exists());
        assert!(!compile_bak_path(&shipping).exists());
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn panic_unwind_restores_entire_compile_transaction() {
        let base = std::env::temp_dir().join("gore-as-compile-panic-rollback");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let dev = dev_cache(&shipping);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        let live_mod = shipping.parent().unwrap().join("Mod.as");
        std::fs::write(&live_mod, b"LIVE-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"STAGED").unwrap();

        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir_all(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"JIT-OLD").unwrap();
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        let payload = win64.join("ue4ss").join("UE4SS.dll");
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, b"UE4SS").unwrap();
        let proxy = win64.join("dwmapi.dll");
        std::fs::write(&proxy, b"PROXY-OLD").unwrap();

        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = game_run_regen_with(&game, &src, |_, _, dev_path| {
                assert!(!jitted.exists());
                assert!(!proxy.exists());
                std::fs::write(&shipping, b"SHIPPING-PARTIAL").unwrap();
                std::fs::write(dev_path, b"DEV-PARTIAL").unwrap();
                std::fs::create_dir_all(&jitted).unwrap();
                std::fs::write(jitted.join("new.bin"), b"JIT-NEW").unwrap();
                std::fs::write(&proxy, b"PROXY-NEW").unwrap();
                panic!("injected generator panic");
                #[allow(unreachable_code)]
                Ok(Vec::new())
            });
        }));
        assert!(unwind.is_err());

        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert_eq!(std::fs::read(&dev).unwrap(), b"DEV-OLD");
        assert_eq!(std::fs::read(&live_mod).unwrap(), b"LIVE-OLD");
        assert_eq!(std::fs::read(jitted.join("old.bin")).unwrap(), b"JIT-OLD");
        assert!(!jitted.join("new.bin").exists());
        assert_eq!(std::fs::read(&proxy).unwrap(), b"PROXY-OLD");
        assert!(!compile_bak_path(&shipping).exists());
        assert!(!compile_lock_path(&game).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_structurally_incomplete_cache_and_rolls_back() {
        let base = std::env::temp_dir().join("gore-as-invalid-generated-cache");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };

        let err = precompile_with(&opts, |_, _, dev| {
            std::fs::write(dev, b"not-a-cache").unwrap();
            Ok(b"not-a-cache".to_vec())
        })
        .unwrap_err();
        assert!(err.contains("invalid generated cache"), "got: {err}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!dev_cache(&shipping).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_uses_dev_cache_and_restores_both_caches_and_colliding_source() {
        let base = std::env::temp_dir().join("gore-as-game-regen-dev");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let dev = dev_cache(&shipping);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        // A matching loose path is safe: staging overwrites it, then cleanup must restore it.
        let live_mod = shipping.parent().unwrap().join("Mod.as");
        std::fs::write(&live_mod, b"LIVE-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"STAGED-NEW").unwrap();

        let regen = game_run_regen_with(&game, &src, |_, _, dev_path| {
            assert_eq!(dev_path, dev);
            assert!(
                !dev_path.exists(),
                "stale dev cache must be removed before generation"
            );
            // Even an unexpected Shipping write by the game must be undone.
            std::fs::write(
                dev_path
                    .parent()
                    .unwrap()
                    .join("PrecompiledScript_Shipping.Cache"),
                b"TOUCHED",
            )
            .unwrap();
            let bytes = valid_cache();
            std::fs::write(dev_path, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap();

        assert_eq!(std::fs::read(regen).unwrap(), valid_cache());
        assert_eq!(
            std::fs::read(&shipping).unwrap(),
            b"OLD",
            "Shipping restored exactly"
        );
        assert_eq!(
            std::fs::read(&dev).unwrap(),
            b"DEV-OLD",
            "old dev cache restored"
        );
        assert_eq!(
            std::fs::read(&live_mod).unwrap(),
            b"LIVE-OLD",
            "colliding source restored"
        );
        assert!(!shipping.with_extension("Cache.gore-compile-bak").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_failure_removes_new_dev_cache_and_restores_shipping() {
        let base = std::env::temp_dir().join("gore-as-game-regen-fail");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let dev = dev_cache(&shipping);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let err = game_run_regen_with(&game, &src, |_, _, dev_path| {
            std::fs::write(dev_path, b"PARTIAL-DEV").unwrap();
            std::fs::write(
                dev_path
                    .parent()
                    .unwrap()
                    .join("PrecompiledScript_Shipping.Cache"),
                b"PARTIAL-SHIPPING",
            )
            .unwrap();
            Err("compile failed".into())
        })
        .unwrap_err();

        assert!(err.contains("compile failed"), "got: {err}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(
            !dev.exists(),
            "new dev cache removed when none existed before"
        );
        assert!(!shipping.parent().unwrap().join("Mod.as").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_rejects_uncovered_loose_script_before_staging() {
        let base = std::env::temp_dir().join("gore-as-game-regen-stray");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let stray = shipping.parent().unwrap().join("OnlyLive.as");
        std::fs::write(&stray, b"do not compile me").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Wanted.as"), b"wanted").unwrap();

        let err =
            game_run_regen_with(&game, &src, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(err.contains("not present in the staged tree"), "got: {err}");
        assert_eq!(std::fs::read(&stray).unwrap(), b"do not compile me");
        assert!(!shipping.with_extension("Cache.gore-compile-bak").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_out_mode_writes_artifact_and_restores_install() {
        let base = std::env::temp_dir().join("gore-as-compile-out");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let dev = dev_cache(&cache);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::write(src.join("AI").join("Mod.as"), b"script").unwrap();
        let live_src = cache.parent().unwrap().join("AI").join("Mod.as");
        std::fs::create_dir_all(live_src.parent().unwrap()).unwrap();
        std::fs::write(&live_src, b"live-script").unwrap();
        let out = base.join("out.Cache");

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: Some(out.clone()),
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, out);
        assert_eq!(
            std::fs::read(&out).unwrap(),
            valid_cache(),
            "artifact holds the compiled bytes"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache restored (install pristine)"
        );
        assert_eq!(
            std::fs::read(&dev).unwrap(),
            b"DEV-OLD",
            "old dev cache restored exactly"
        );
        assert_eq!(
            std::fs::read(&live_src).unwrap(),
            b"live-script",
            "covered pre-existing source is restored exactly"
        );
        assert!(
            !deploy_bak_path(&cache).exists(),
            "out-mode leaves no .gore-bak"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_installs_new_cache_and_backs_up() {
        let base = std::env::temp_dir().join("gore-as-compile-inplace");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, cache);
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            valid_cache(),
            "new cache installed in place"
        );
        assert!(
            !dev_cache(&cache).exists(),
            "new dev cache removed after in-place install"
        );
        assert_eq!(
            std::fs::read(deploy_bak_path(&cache)).unwrap(),
            b"OLD",
            "previous cache backed up to .gore-bak"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as cleaned"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_rejects_directory_as_existing_deploy_backup() {
        let base = std::env::temp_dir().join("gore-as-compile-invalid-deploy-backup");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let backup = deploy_bak_path(&cache);
        std::fs::create_dir(&backup).unwrap();
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: None,
            backup: true,
        };

        let err = precompile_with(&opts, gen_new).unwrap_err();

        assert!(
            err.contains("not a regular non-reparse file"),
            "unexpected error: {err}"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "invalid backup must fail before installing generated bytes"
        );
        assert!(backup.is_dir(), "the rejected path must remain untouched");
        assert!(!dev_cache(&cache).exists());
        assert!(!compile_bak_path(&cache).exists());
        assert!(!compile_lock_path(&game).exists());
        assert!(!recovery_journal_path(&game).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_preserves_valid_existing_deploy_backup() {
        let base = std::env::temp_dir().join("gore-as-compile-existing-deploy-backup");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let backup = deploy_bak_path(&cache);
        std::fs::write(&backup, b"EARLIEST").unwrap();
        let opts = PrecompileOpts {
            game_dir: game,
            src: None,
            out: None,
            backup: true,
        };

        precompile_with(&opts, gen_new).unwrap();

        assert_eq!(std::fs::read(&cache).unwrap(), valid_cache());
        assert_eq!(
            std::fs::read(&backup).unwrap(),
            b"EARLIEST",
            "a valid existing backup must never be overwritten"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_rejects_link_as_existing_deploy_backup() {
        let base = std::env::temp_dir().join("gore-as-compile-linked-deploy-backup");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let backup = deploy_bak_path(&cache);
        let link_target = base.join("not-the-backup.Cache");
        std::fs::write(&link_target, b"DO-NOT-USE").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&link_target, &backup).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&link_target, &backup).is_err() {
            // Windows without Developer Mode/elevation cannot create this fixture. Reparse paths
            // still hit the same production predicate exercised by the staging-link test.
            let _ = std::fs::remove_dir_all(&base);
            return;
        }

        let opts = PrecompileOpts {
            game_dir: game,
            src: None,
            out: None,
            backup: true,
        };
        let err = precompile_with(&opts, gen_new).unwrap_err();

        assert!(
            err.contains("not a regular non-reparse file"),
            "unexpected error: {err}"
        );
        assert_eq!(std::fs::read(&cache).unwrap(), b"OLD");
        assert_eq!(std::fs::read(&link_target).unwrap(), b"DO-NOT-USE");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_keeps_new_cache_even_if_src_carries_a_cache_file() {
        // Regression: a staged src tree that happens to include a file at the cache path must NOT
        // cause cleanup to restore the old cache over the freshly compiled one.
        let base = std::env::temp_dir().join("gore-as-compile-srccache");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("PrecompiledScript_Shipping.Cache"), b"SRCCACHE").unwrap();
        std::fs::write(src.join("PrecompiledScript.Cache"), b"STALE-DEV").unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, cache);
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            valid_cache(),
            "freshly compiled cache kept, not clobbered by the staged src cache file"
        );
        assert!(
            !dev_cache(&cache).exists(),
            "staged/generated development cache cleaned"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as cleaned"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_src_that_contains_the_script_dir() {
        // SRC = the game root (which contains G1R/Script): staging would copy the install into its
        // own subtree. Must be rejected up front, before any staging or generation.
        let base = std::env::temp_dir().join("gore-as-compile-overlap");
        let _ = std::fs::remove_dir_all(&base);
        let (game, _cache) = fake_install(&base);
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: Some(game), // the game root contains G1R/Script
            out: None,
            backup: true,
        };
        let err =
            precompile_with(&opts, |_, _, _| panic!("must not stage or generate")).unwrap_err();
        assert!(err.contains("contains the game's Script"), "got: {err}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rolls_back_on_generation_failure() {
        // A generation error rolls the install back: the live cache is restored, staged .as removed,
        // and the original error is surfaced.
        let base = std::env::temp_dir().join("gore-as-compile-genfail");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let dev = dev_cache(&cache);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };
        // Stub emulates a partial development-cache write plus an unexpected Shipping write.
        let err = precompile_with(&opts, |_, _, dev_cache| {
            std::fs::write(dev_cache, b"PARTIAL-DEV").unwrap();
            std::fs::write(
                dev_cache
                    .parent()
                    .unwrap()
                    .join("PrecompiledScript_Shipping.Cache"),
                b"PARTIAL-SHIPPING",
            )
            .unwrap();
            Err("boom".to_string())
        })
        .unwrap_err();

        assert!(
            err.contains("boom"),
            "surfaces the original error; got: {err}"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache rolled back"
        );
        assert_eq!(
            std::fs::read(&dev).unwrap(),
            b"DEV-OLD",
            "development cache rolled back"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as removed on rollback"
        );
        assert!(
            !deploy_bak_path(&cache).exists(),
            "failed generation must not create a persistent deploy backup"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_out_inside_script_dir() {
        // `-o` under Script/ (the live cache, or any path there) is rejected: it would pollute the
        // install and could collide with a staged file / the restore.
        let base = std::env::temp_dir().join("gore-as-compile-outinside");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        for out in [cache.clone(), cache.parent().unwrap().join("MyMod.Cache")] {
            let opts = PrecompileOpts {
                game_dir: game.clone(),
                src: None,
                out: Some(out.clone()),
                backup: false,
            };
            let err = precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err();
            assert!(
                err.contains("Script/ directory"),
                "out={:?} got: {err}",
                out
            );
        }
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache left untouched"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_out_mode_write_failure_still_restores_install() {
        // If writing the output fails, the install must STILL be rolled back (cache restored, staged
        // removed), and the write error surfaced.
        let base = std::env::temp_dir().join("gore-as-compile-outfail");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        // Output under a non-existent directory → std::fs::write fails.
        let out = base.join("nope-dir").join("out.Cache");

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: Some(out),
            backup: false,
        };
        let err = precompile_with(&opts, gen_new).unwrap_err();

        assert!(
            err.contains("writing output"),
            "surfaces the write error; got: {err}"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache restored despite write failure"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as removed despite write failure"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn resolve_out_real_makes_relative_paths_absolute_under_cwd() {
        let base = std::env::temp_dir().join("gore-as-resolve-out");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let base_real = base.canonicalize().unwrap();

        // A relative out resolves under cwd, so a Script/-relative path is caught by the guard.
        let rel = resolve_out_real(Path::new("MyMod.Cache"), &base);
        assert!(
            rel.starts_with(&base_real),
            "relative out resolved under cwd: {rel:?}"
        );

        // An absolute out elsewhere stays where it is (not under cwd).
        let other = std::env::temp_dir()
            .join("gore-as-resolve-other")
            .join("x.Cache");
        let abs = resolve_out_real(&other, &base);
        assert!(
            !abs.starts_with(&base_real),
            "absolute out stays put: {abs:?}"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_report_keeps_legacy_diagnostics_and_into_parts_signatures() {
        let report = GameRunRegenReport {
            result: Err("simulated compiler rejection".to_owned()),
            diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                crate::diagnostics::DiagnosticsCaptureDisposition::Disabled,
            ),
            install_restore: InstallRestoreDisposition::RestoredExact,
        };

        let diagnostics: &crate::diagnostics::CompilerDiagnosticsReport = report.diagnostics();
        assert_eq!(
            diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Disabled
        );
        assert_eq!(
            report.install_restore_disposition(),
            InstallRestoreDisposition::RestoredExact
        );
        let (_result, diagnostics): (
            Result<PathBuf, String>,
            crate::diagnostics::CompilerDiagnosticsReport,
        ) = report.into_parts();
        assert_eq!(
            diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Disabled
        );
    }

    #[test]
    fn compile_module_report_retains_structured_diagnostics_on_success_and_failure() {
        let root = std::env::temp_dir().join(format!(
            "gore-as-compile-report-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("must-not-be-opened.as");
        let opts = CompileOpts {
            game_dir: root.join("game"),
            op: "add".to_owned(),
            module_name: "NewModule".to_owned(),
            rel_path: "NewModule.as".to_owned(),
            as_path: source,
            source_override: Some(b"// native-sealed generated module\n".to_vec()),
            work_dir: root.join("work"),
            allow_new_symbols: true,
            base_override: Some(cache_with_empty_modules(&[("Base", "Base.as")])),
            binds_override: None,
        };
        let generated =
            cache_with_empty_modules(&[("Base", "Base.as"), ("NewModule", "NewModule.as")]);
        assert!(!opts.as_path.exists());
        let success = compile_module_report_with(&opts, |_, _| {
            let path = root.join("generated.cache");
            std::fs::write(&path, &generated).unwrap();
            let report = GameRunRegenExtendedReport {
                result: Ok(path),
                diagnostics: Some(
                    crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                        crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
                        "=== NewModule.as ===\n(1:1) [W] retained warning\n",
                    )
                    .unwrap(),
                ),
                install_restore: InstallRestoreDisposition::RestoredExact,
            };
            assert_eq!(
                report.install_restore,
                InstallRestoreDisposition::RestoredExact
            );
            Ok(report)
        });
        assert!(matches!(
            &success.outcome,
            CompileModuleReportOutcome::Compiled(_)
        ));
        let diagnostics = success.diagnostics().unwrap();
        assert_eq!(
            diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured
        );
        assert_eq!(diagnostics.diagnostics().len(), 1);
        assert_eq!(diagnostics.diagnostics()[0].message, "retained warning");
        assert_eq!(
            success.install_restore_disposition(),
            InstallRestoreDisposition::RestoredExact
        );
        // A successful output intentionally pins its exact artifact until the caller consumes it.
        drop(success);

        let failed = compile_module_report_with(&opts, |_, _| {
            Ok(GameRunRegenExtendedReport {
                result: Err("compiler rejected the source".to_owned()),
                diagnostics: Some(
                    crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                        crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
                        "=== NewModule.as ===\n(3:4) [E] broken expression\n",
                    )
                    .unwrap(),
                ),
                install_restore: InstallRestoreDisposition::RestoredExact,
            })
        });
        assert!(matches!(
            &failed.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Regen(_))
        ));
        assert_eq!(
            failed.diagnostics().unwrap().diagnostics()[0].message,
            "broken expression"
        );
        assert_eq!(
            failed.install_restore_disposition(),
            InstallRestoreDisposition::RestoredExact,
            "an ordinary compiler rejection still restores the install exactly"
        );

        let recovery_required = compile_module_report_with(&opts, |_, _| {
            Ok(GameRunRegenExtendedReport {
                result: Err("generator exit could not be confirmed".to_owned()),
                diagnostics: Some(crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed,
                )),
                install_restore: InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed,
            })
        });
        assert!(matches!(
            &recovery_required.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Regen(_))
        ));
        assert_eq!(
            recovery_required.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
        );

        let recovery_before_runner = compile_module_report_with(&opts, |_, _| {
            Ok(GameRunRegenExtendedReport {
                result: Err("isolation setup failed and its rollback also failed".to_owned()),
                diagnostics: None,
                install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
            })
        });
        assert!(matches!(
            &recovery_before_runner.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Regen(_))
        ));
        assert!(recovery_before_runner.diagnostics().is_none());
        assert_eq!(
            recovery_before_runner.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
            "the report must be stored before its inner compiler error is returned"
        );

        let mut invalid_opts = opts;
        invalid_opts.op = "invalid".to_owned();
        let not_run = compile_module_report_with(&invalid_opts, |_, _| {
            panic!("invalid preflight must not launch the compiler")
        });
        assert!(matches!(
            &not_run.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Other(_))
        ));
        assert!(not_run.diagnostics().is_none());
        assert_eq!(
            not_run.install_restore_disposition(),
            InstallRestoreDisposition::NotStarted
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sealed_binds_override_fails_closed_before_regen() {
        let root = unique_test_root("invalid-sealed-binds");
        std::fs::create_dir_all(&root).unwrap();
        let opts = CompileOpts {
            game_dir: root.join("game"),
            op: "add".to_owned(),
            module_name: "Managed".to_owned(),
            rel_path: "Managed.as".to_owned(),
            as_path: root.join("must-not-be-opened.as"),
            source_override: Some(b"// sealed source\n".to_vec()),
            work_dir: root.join("work"),
            allow_new_symbols: true,
            base_override: Some(cache_with_empty_modules(&[("Base", "Base.as")])),
            binds_override: Some(Vec::new()),
        };
        let called = std::cell::Cell::new(false);
        let error = compile_module(&opts, |_, _| {
            called.set(true);
            Err("regen must not run".to_owned())
        })
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("sealed Binds.Cache override is invalid"));
        assert!(!called.get());
        assert!(!opts.as_path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compiled_artifact_neutralizes_the_creation_identity() {
        let root = unique_test_root("retained-compiled-artifact");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("module.cache");
        let escaped = root.join("escaped.cache");
        let artifact = write_compiled_artifact(path.clone(), b"compiler bytes").unwrap();

        #[cfg(windows)]
        {
            assert!(
                std::fs::rename(&path, &escaped).is_err(),
                "the retained creation handle must deny Windows rename/replacement"
            );
            artifact.neutralize().unwrap();
            assert!(std::fs::read(&path).unwrap().is_empty());
        }

        #[cfg(unix)]
        {
            std::fs::rename(&path, &escaped).unwrap();
            std::fs::write(&path, b"replacement").unwrap();
            artifact.neutralize().unwrap();
            assert!(std::fs::read(&escaped).unwrap().is_empty());
            assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        }

        drop(artifact);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compiled_artifact_never_follows_a_precreated_link() {
        let root = unique_test_root("compiled-artifact-link");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("module.cache");
        let victim = root.join("victim.cache");
        std::fs::write(&victim, b"victim").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&victim, &path).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&victim, &path).is_err() {
            std::fs::remove_dir_all(root).unwrap();
            return;
        }

        let artifact = write_compiled_artifact(path.clone(), b"safe output").unwrap();
        assert_eq!(std::fs::read(&victim).unwrap(), b"victim");
        assert_eq!(std::fs::read(&path).unwrap(), b"safe output");
        artifact.neutralize().unwrap();
        drop(artifact);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_artifact_error_keeps_exact_cleanup_capability() {
        let root = unique_test_root("failed-artifact-handle");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("module.cache");
        let artifact = write_compiled_artifact(path.clone(), b"partial bytes").unwrap();
        let mut error = CompileError::ArtifactIo {
            message: "injected final-write failure".to_owned(),
            artifact: Some(FailedCompiledArtifact { artifact }),
        };
        error
            .neutralize_failed_artifact()
            .expect("artifact error exposes cleanup")
            .unwrap();
        assert!(std::fs::read(&path).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preheld_compile_guard_is_explicitly_released_when_module_preflight_fails() {
        let root = unique_test_root("preheld-guard-preflight-release");
        let (game, _) = fake_install(&root);
        let source = root.join("overlay.as");
        std::fs::write(&source, b"// never staged\n").unwrap();
        let guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        let opts = CompileOpts {
            game_dir: game.clone(),
            op: "invalid".to_owned(),
            module_name: "NeverRuns".to_owned(),
            rel_path: "NeverRuns.as".to_owned(),
            as_path: source,
            source_override: None,
            work_dir: root.join("work"),
            allow_new_symbols: false,
            base_override: Some(Vec::new()),
            binds_override: None,
        };

        let report =
            compile_module_with_diagnostics_report_with_guard(&opts, &Default::default(), guard);
        assert!(matches!(
            &report.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Other(_))
        ));
        assert_eq!(
            report.install_restore_disposition(),
            InstallRestoreDisposition::NotStarted
        );
        assert!(
            !lock.exists(),
            "ordinary preflight failure must release the guard"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preheld_compile_guard_release_failure_is_dominant_and_persistent() {
        let root = unique_test_root("preheld-guard-release-failure");
        let (game, _) = fake_install(&root);
        let source = root.join("overlay.as");
        std::fs::write(&source, b"// never staged\n").unwrap();
        let mut guard = InstallMutationGuard::acquire(&game, "gore-as:compile").unwrap();
        let lock = guard.path().to_path_buf();
        // Simulate an ownership mismatch without mutating the live record. The production path
        // must surface this release failure instead of allowing Drop to hide it.
        guard.payload.push_str("mismatched-in-memory-owner");
        let opts = CompileOpts {
            game_dir: game.clone(),
            op: "invalid".to_owned(),
            module_name: "NeverRuns".to_owned(),
            rel_path: "NeverRuns.as".to_owned(),
            as_path: source,
            source_override: None,
            work_dir: root.join("work"),
            allow_new_symbols: false,
            base_override: Some(Vec::new()),
            binds_override: None,
        };

        let report =
            compile_module_with_diagnostics_report_with_guard(&opts, &Default::default(), guard);
        match &report.outcome {
            CompileModuleReportOutcome::Failed(error) => assert!(
                error.to_string().contains("failed to release the pre-held"),
                "got: {error}"
            ),
            CompileModuleReportOutcome::Compiled(_) => panic!("invalid preflight compiled"),
        }
        assert_eq!(
            report.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        assert!(
            lock.exists(),
            "the unreleased lock is the persistent blocker"
        );

        // The test intentionally injected a private in-memory mismatch; no live transaction ran,
        // so remove that exact test-owned record after the API has closed its handle.
        std::fs::remove_file(lock).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shared_guard_release_failure_keeps_the_recovery_journal_until_retry_succeeds() {
        let root = unique_test_root("guard-release-keeps-journal");
        let (game, shipping) = fake_install(&root);
        let g1r = g1r_dir(&game);
        let script = g1r.join("Script");
        let mut txn =
            CompileTransaction::begin_with_process_checker(&game, &g1r, &script, || Ok(false))
                .unwrap();
        let readme = std::fs::read_to_string(txn.journal.root.join("README.txt")).unwrap();
        assert!(readme.contains(".gore-as-compile-recovery"));
        assert!(readme.contains(".gore-as-compile.lock"));
        assert!(readme.contains(".gore-install-mutation.lock"));

        assert!(txn.restore_install().clean());
        let expected_payload = txn.mutation_guard.payload.clone();
        txn.mutation_guard
            .payload
            .push_str("mismatched-in-memory-owner");
        let errors = txn.finish();
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(errors[0].contains("ownership record changed"));
        assert!(
            txn.journal.root.exists(),
            "journal must outlive lock-release failure"
        );
        assert!(txn.mutation_guard.path.exists());
        assert!(
            !txn.lock.path.exists(),
            "the narrower compile lock was released"
        );
        assert!(
            !txn.recovery.path.exists(),
            "the exact Shipping backup was retired"
        );
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        // Exercise a successful explicit retry, after restoring the injected expected identity.
        txn.mutation_guard.payload = expected_payload;
        txn.mutation_guard.release().unwrap();
        txn.journal.retire().unwrap();
        assert!(!install_mutation_lock_path(&game).exists());
        assert!(!recovery_journal_path(&game).exists());
        drop(txn);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_match_fallback_returns_the_normal_generator_result() {
        let called = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(
            DiagnosticAttempt::Unavailable("signature matched 0 times".into()),
            || {
                called.set(called.get() + 1);
                GeneratorRunResult::confirmed(Ok::<_, String>(b"real-cache".to_vec()))
            },
        );
        assert_eq!(report.result.unwrap(), b"real-cache");
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback
        );
        assert!(report.diagnostics.diagnostics().is_empty());
        assert_eq!(called.get(), 1);
    }

    #[test]
    fn captured_compiler_error_rejects_a_structurally_complete_cache() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-captured-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"),
            status: base.join("status.txt"),
            dir: base.clone(),
            cleanup: true,
        };
        std::fs::write(
            &artifacts.capture,
            "=== Test.as ===\n(4:2) [E] No matching signatures to 'Broken()'\n",
        )
        .unwrap();
        let complete = valid_cache();
        validate_generated_cache(&complete).expect("fixture must be structurally complete");
        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(complete)),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured
        );
        assert_eq!(captured.diagnostics.diagnostics().len(), 1);
        let error = captured.result.unwrap_err();
        assert!(error.contains("compiler reported an error"), "got: {error}");
        assert!(error.contains("Test.as:4:2: error"), "got: {error}");
    }

    #[test]
    fn truncated_capture_rejects_a_cache_even_without_a_visible_error() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-truncated-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"),
            status: base.join("status.txt"),
            dir: base.clone(),
            cleanup: true,
        };
        std::fs::write(
            &artifacts.capture,
            format!(
                "=== Test.as ===\n[W] warnings filled the capture\n{}\n",
                crate::diagnostics::CAPTURE_TRUNCATED_TOKEN
            ),
        )
        .unwrap();
        let complete = valid_cache();
        validate_generated_cache(&complete).expect("fixture must be structurally complete");
        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(complete)),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
        );
        assert_eq!(captured.diagnostics.diagnostics().len(), 1);
        assert_eq!(
            captured.diagnostics.diagnostics()[0].message,
            "warnings filled the capture"
        );
        let error = captured.result.unwrap_err();
        assert!(error.contains("capture was truncated"), "got: {error}");
        assert!(error.contains("refusing to accept"), "got: {error}");
    }

    #[test]
    fn unreadable_existing_capture_rejects_an_unverified_cache() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-unreadable-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(base.join("capture.txt")).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"), // a directory: exists, but cannot be read as bytes
            status: base.join("status.txt"),
            dir: base.clone(),
            cleanup: true,
        };
        let complete = valid_cache();
        validate_generated_cache(&complete).expect("fixture must be structurally complete");
        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(complete)),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
        );
        let error = captured.result.unwrap_err();
        assert!(error.contains("could not be read"), "got: {error}");
        assert!(error.contains("refusing to accept"), "got: {error}");

        drop(artifacts);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn structured_capture_limit_rejects_an_unverified_cache() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-structured-capture-limit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"),
            status: base.join("status.txt"),
            dir: base,
            cleanup: true,
        };
        let oversized_file =
            "x".repeat(crate::diagnostics::MAX_STRUCTURED_DIAGNOSTIC_FILE_BYTES + 1);
        std::fs::write(
            &artifacts.capture,
            format!("=== {oversized_file} ===\n[E] failure\n"),
        )
        .unwrap();

        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(valid_cache())),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
        );
        assert!(captured.diagnostics.diagnostics().is_empty());
        let error = captured.result.unwrap_err();
        assert!(
            error.contains("bounded structured diagnostics"),
            "got: {error}"
        );
        assert!(error.contains("refusing to accept"), "got: {error}");
    }

    #[test]
    fn unconfirmed_hooked_timeout_preserves_recovery_without_exposing_live_capture() {
        let root = std::env::temp_dir().join(format!(
            "gore-as-unconfirmed-hooked-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let diagnostics_dir = root.join("diagnostics");
        let helper_dir = root.join("helper");
        std::fs::create_dir_all(&diagnostics_dir).unwrap();
        std::fs::create_dir_all(&helper_dir).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: diagnostics_dir.join("capture.txt"),
            status: diagnostics_dir.join("status.txt"),
            dir: diagnostics_dir.clone(),
            cleanup: true,
        };
        std::fs::write(&artifacts.capture, "[W] retained warning\n").unwrap();
        let helper = helper_dir.join("gore-as-diagnostics-hook.dll");
        std::fs::write(&helper, b"test helper").unwrap();
        let prep =
            crate::diagnostics::HookPreparation::owned_for_test(helper.clone(), helper_dir.clone());

        let attempt = classify_hooked_result(
            GeneratorRunResult::unconfirmed(format!(
                "{GENERATOR_EXIT_UNCONFIRMED} simulated live generator"
            )),
            artifacts,
            prep,
        );
        let fallback_calls = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(attempt, || {
            fallback_calls.set(fallback_calls.get() + 1);
            GeneratorRunResult::confirmed(Ok::<_, String>(b"unsafe fallback".to_vec()))
        });
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed
        );
        assert!(report.diagnostics.diagnostics().is_empty());
        assert_eq!(fallback_calls.get(), 0);
        assert_eq!(
            report.private_paths,
            vec![diagnostics_dir.clone(), helper_dir.clone()]
        );
        let error = report.result.unwrap_err();
        assert!(
            error.contains(&diagnostics_dir.display().to_string()),
            "got: {error}"
        );
        assert!(
            error.contains(&helper_dir.display().to_string()),
            "got: {error}"
        );
        let redacted = redact_private_compiler_recovery_paths(&error, &report.private_paths);
        assert!(!redacted.contains(&diagnostics_dir.display().to_string()));
        assert!(!redacted.contains(&helper_dir.display().to_string()));
        assert_eq!(
            redacted.matches("<private compiler recovery path>").count(),
            2
        );
        assert!(
            diagnostics_dir.is_dir(),
            "diagnostics directory was dropped"
        );
        assert!(helper.is_file(), "mapped helper was dropped");
        assert_eq!(
            std::fs::read_to_string(diagnostics_dir.join("capture.txt")).unwrap(),
            "[W] retained warning\n"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn started_hook_unconfirmed_preserves_recovery_without_exposing_live_capture() {
        let root = std::env::temp_dir().join(format!(
            "gore-as-unconfirmed-started-hook-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let diagnostics_dir = root.join("diagnostics");
        let helper_dir = root.join("helper");
        std::fs::create_dir_all(&diagnostics_dir).unwrap();
        std::fs::create_dir_all(&helper_dir).unwrap();
        let capture = diagnostics_dir.join("capture.txt");
        std::fs::write(&capture, "[E] possibly partial live message\n").unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: capture.clone(),
            status: diagnostics_dir.join("status.txt"),
            dir: diagnostics_dir.clone(),
            cleanup: true,
        };
        let helper = helper_dir.join("gore-as-diagnostics-hook.dll");
        std::fs::write(&helper, b"test helper").unwrap();
        let prep =
            crate::diagnostics::HookPreparation::owned_for_test(helper.clone(), helper_dir.clone());

        let attempt = classify_started_hook_termination(
            ChildWaitFailure {
                message: format!("{GENERATOR_EXIT_UNCONFIRMED} simulated failed termination"),
                process_exit: GeneratorProcessExitDisposition::Unconfirmed,
            },
            artifacts,
            prep,
        );
        let fallback_calls = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(attempt, || {
            fallback_calls.set(fallback_calls.get() + 1);
            GeneratorRunResult::confirmed(Ok::<_, String>(b"unsafe fallback".to_vec()))
        });
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed
        );
        assert!(report.diagnostics.diagnostics().is_empty());
        assert_eq!(fallback_calls.get(), 0);
        let error = report.result.unwrap_err();
        assert!(
            error.contains(&diagnostics_dir.display().to_string()),
            "got: {error}"
        );
        assert!(
            error.contains(&helper_dir.display().to_string()),
            "got: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(&capture).unwrap(),
            "[E] possibly partial live message\n"
        );
        assert!(helper.is_file(), "mapped helper was dropped");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_diagnostics_opt_out_runs_normal_path_without_unavailable_state() {
        let called = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(DiagnosticAttempt::Disabled, || {
            called.set(called.get() + 1);
            GeneratorRunResult::confirmed(Ok::<_, String>("normal"))
        });
        assert_eq!(report.result.unwrap(), "normal");
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Disabled
        );
        assert_eq!(called.get(), 1);
    }

    #[test]
    fn injection_failure_fallback_preserves_the_normal_error() {
        let report = resolve_diagnostic_attempt_report::<Vec<u8>, _>(
            DiagnosticAttempt::Unavailable("CreateRemoteThread failed".into()),
            || {
                GeneratorRunResult::confirmed(
                    Err("normal generator failed exactly this way".into()),
                )
            },
        );
        assert_eq!(
            report.result.unwrap_err(),
            "normal generator failed exactly this way"
        );
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback
        );
    }

    #[test]
    fn pre_injection_exit_keeps_the_first_generator_result() {
        let fallback_calls = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(
            DiagnosticAttempt::Completed(GeneratorDiagnosticsResult {
                result: Ok::<_, String>(b"first-normal-result".to_vec()),
                diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableWithoutFallback,
                ),
                process_exit: GeneratorProcessExitDisposition::Confirmed,
                private_paths: Vec::new(),
            }),
            || {
                fallback_calls.set(fallback_calls.get() + 1);
                GeneratorRunResult::confirmed(Ok(b"unexpected-relaunch".to_vec()))
            },
        );
        assert_eq!(report.result.unwrap(), b"first-normal-result");
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableWithoutFallback
        );
        assert_eq!(fallback_calls.get(), 0);
    }

    #[test]
    fn post_injection_pre_ready_exit_retries_after_partial_cache_cleanup() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-post-injection-fallback-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&base);
        std::fs::write(&base, b"partial-from-injected-attempt").unwrap();
        let result = resolve_diagnostic_attempt(
            DiagnosticAttempt::Unavailable(
                "generator exited after injection before helper readiness".into(),
            ),
            || {
                clear_partial_cache_before_fallback(&base)?;
                Ok::<_, String>(b"clean-normal-result".to_vec())
            },
        )
        .unwrap();
        assert_eq!(result, b"clean-normal-result");
        assert!(!base.exists());
    }

    #[test]
    fn fallback_deletes_partial_first_attempt_cache() {
        let base =
            std::env::temp_dir().join(format!("gore-as-partial-fallback-{}", std::process::id()));
        let _ = std::fs::remove_file(&base);
        std::fs::write(&base, b"partial-from-hook-attempt").unwrap();
        clear_partial_cache_before_fallback(&base).unwrap();
        assert!(!base.exists());
    }

    #[test]
    fn precompile_rejects_src_when_script_dir_has_loose_scripts() {
        // A pre-existing loose .as in Script/ would be compiled alongside SRC — refuse.
        let base = std::env::temp_dir().join("gore-as-compile-dirty");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        std::fs::write(cache.parent().unwrap().join("Stale.as"), b"stale").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: false,
        };
        let err = precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(err.contains("loose script"), "got: {err}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn loose_script_walkers_fail_closed_on_io_errors_and_non_file_coverage() {
        let base = std::env::temp_dir().join("gore-as-loose-walker-errors");
        let _ = std::fs::remove_dir_all(&base);
        let missing = base.join("missing");
        assert!(first_loose_script(&missing).is_err());
        assert!(first_uncovered_loose_script(&missing, &base).is_err());

        let live = base.join("live");
        let src = base.join("src");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::create_dir_all(src.join("Same.as")).unwrap(); // directory is not valid coverage
        let live_script = live.join("Same.as");
        std::fs::write(&live_script, b"live").unwrap();
        assert_eq!(
            first_uncovered_loose_script(&live, &src).unwrap(),
            Some(live_script)
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_tree_records_rollback_before_injected_partial_copy_failure() {
        let base = std::env::temp_dir().join("gore-as-copy-partial-failure");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("Mod.as"), b"NEW-COMPLETE").unwrap();
        let target = dst.join("Mod.as");
        std::fs::write(&target, b"OLD").unwrap();
        let mut written = Vec::new();

        let err = copy_tree_with(&src, &dst, &mut written, &mut |_, to| {
            std::fs::write(to, b"PARTIAL")?;
            Err(std::io::Error::other("injected copy failure"))
        })
        .unwrap_err();
        assert!(err.to_string().contains("injected copy failure"));
        assert_eq!(written.len(), 1, "rollback entry registered before copy");
        assert_eq!(written[0].1.as_deref(), Some(b"OLD".as_slice()));
        assert_eq!(std::fs::read(&target).unwrap(), b"PARTIAL");

        restore_or_remove(&written, &dst).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"OLD");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_tree_propagates_destination_snapshot_errors_before_copy() {
        let base = std::env::temp_dir().join("gore-as-copy-snapshot-error");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(dst.join("Mod.as")).unwrap();
        std::fs::write(src.join("Mod.as"), b"NEW").unwrap();
        let mut written = Vec::new();
        let copied = std::cell::Cell::new(false);

        assert!(copy_tree_with(&src, &dst, &mut written, &mut |_, _| {
            copied.set(true);
            Ok(())
        })
        .is_err());
        assert!(
            !copied.get(),
            "copy must not run without a reliable snapshot"
        );
        assert!(written.is_empty());
        assert!(dst.join("Mod.as").is_dir());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_tree_rejects_linked_destination_directory() {
        let base = std::env::temp_dir().join("gore-as-copy-linked-destination");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        let outside = base.join("outside");
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(src.join("AI").join("Mod.as"), b"NEW").unwrap();
        let linked = dst.join("AI");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &linked).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_dir(&outside, &linked).is_err() {
            // Windows without Developer Mode/elevation cannot create the fixture. Production
            // junctions still carry FILE_ATTRIBUTE_REPARSE_POINT and hit the same guard.
            let _ = std::fs::remove_dir_all(&base);
            return;
        }

        let err = copy_tree(&src, &dst, &mut Vec::new()).unwrap_err();
        assert!(err.to_string().contains("linked/reparse"), "got: {err}");
        assert!(
            !outside.join("Mod.as").exists(),
            "staging must not write through the destination link"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn unconfirmed_generator_preserves_isolation_recovery_and_lock() {
        let base = std::env::temp_dir().join("gore-as-unconfirmed-generator");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        std::fs::write(dev_cache(&shipping), b"DEV-OLD").unwrap();
        let live_mod = shipping.parent().unwrap().join("Mod.as");
        std::fs::write(&live_mod, b"LIVE-OLD").unwrap();
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        std::fs::create_dir_all(win64.join("ue4ss")).unwrap();
        std::fs::write(win64.join("ue4ss").join("UE4SS.dll"), b"ue4ss").unwrap();
        std::fs::write(win64.join("dwmapi.dll"), b"proxy").unwrap();
        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir_all(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"jit").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let err = game_run_regen_with(&game, &src, |_, _, _| {
            Err(format!(
                "{GENERATOR_EXIT_UNCONFIRMED} simulated live generator 123"
            ))
        })
        .unwrap_err();

        assert!(err.contains("intentionally NOT run"), "got: {err}");
        assert!(
            compile_lock_path(&game).exists(),
            "compile lock must remain"
        );
        assert!(
            install_mutation_lock_path(&game).exists(),
            "cross-tool install-mutation lock must remain"
        );
        assert!(
            compile_bak_path(&shipping).exists(),
            "Shipping recovery backup must remain"
        );
        assert_eq!(
            std::fs::read(&live_mod).unwrap(),
            b"script",
            "staged source must remain isolated until the child is killed"
        );
        let journal = recovery_journal_path(&game);
        assert_eq!(
            std::fs::read(journal.join("overwritten").join("Mod.as")).unwrap(),
            b"LIVE-OLD",
            "overwritten loose-script bytes must be recoverable from disk"
        );
        assert_eq!(
            std::fs::read(
                journal
                    .join("development-cache")
                    .join("PrecompiledScript.Cache")
            )
            .unwrap(),
            b"DEV-OLD",
            "the pre-call development cache must be recoverable from disk"
        );
        assert!(!jitted.exists(), "original JIT path must stay quarantined");
        assert!(append_suffix(&jitted, ".gore-compile-bak").exists());
        let proxy = win64.join("dwmapi.dll");
        assert!(!proxy.exists(), "UE4SS proxy must stay quarantined");
        assert!(append_suffix(&proxy, ".gore-compile-bak").exists());

        // The transaction is intentionally leaked; remove the isolated temp fixture as a whole.
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_uses_structured_unconfirmed_exit_without_string_marker() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-precompile-structured-unconfirmed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: Some(src),
            out: Some(base.join("compiled.cache")),
            backup: true,
        };
        let message = "simulated process tree still alive without a legacy marker";
        assert!(!message.contains(GENERATOR_EXIT_UNCONFIRMED));

        let error = precompile_with_generator_report_and_process_checker(
            &opts,
            || Ok(false),
            |_, _, _| GeneratorRunResult::unconfirmed(message.to_owned()),
        )
        .unwrap_err();

        assert!(error.contains(message), "got: {error}");
        assert!(error.contains("intentionally NOT run"), "got: {error}");
        assert!(compile_lock_path(&game).exists());
        assert!(install_mutation_lock_path(&game).exists());
        assert_eq!(std::fs::read(compile_bak_path(&shipping)).unwrap(), b"OLD");
        assert!(recovery_journal_path(&game).exists());
        assert!(shipping.parent().unwrap().join("Mod.as").exists());
        assert!(!opts.out.as_ref().unwrap().exists());

        // No real process exists in this injected test; removing the isolated fixture is the
        // test-only equivalent of completing the documented recovery sequence.
        std::fs::remove_dir_all(base).unwrap();
    }

    /// `copy_tree` records every file it writes (with its prior bytes); `restore_or_remove` then
    /// deletes the ones it created and RESTORES the ones it overwrote, plus prunes the now-empty
    /// dirs it created, while leaving the dst root AND non-colliding pre-existing files untouched.
    /// This is the offline guard for the CRITICAL "don't pollute / don't destroy the install"
    /// cleanup invariant.
    #[test]
    fn copy_tree_then_remove_written_leaves_install_clean() {
        let base = std::env::temp_dir().join("gore-as-cleanup-test");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        // Source tree: a top-level file, a nested subdir file, AND a file that COLLIDES with a
        // pre-existing dst file (its original content must be restored on cleanup).
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::write(src.join("Top.as"), b"top").unwrap();
        std::fs::write(src.join("AI").join("Nested.as"), b"nested").unwrap();
        std::fs::write(src.join("Over.as"), b"new").unwrap();
        // Destination pre-exists and already holds a non-colliding file that must SURVIVE cleanup,
        // plus a colliding file that will be overwritten then RESTORED.
        std::fs::create_dir_all(&dst).unwrap();
        let pre = dst.join("Pre.as");
        std::fs::write(&pre, b"preexisting").unwrap();
        let over = dst.join("Over.as");
        std::fs::write(&over, b"old").unwrap();

        let mut written = Vec::new();
        copy_tree(&src, &dst, &mut written).unwrap();

        // Recorded exactly the three copied files, and they landed on disk.
        assert_eq!(written.len(), 3);
        let top = dst.join("Top.as");
        let nested = dst.join("AI").join("Nested.as");
        assert!(top.exists());
        assert!(nested.exists());
        assert!(written.iter().any(|(p, _)| p == &top));
        assert!(written.iter().any(|(p, _)| p == &nested));
        // The collision was overwritten with the new bytes, and its prior bytes were captured.
        assert_eq!(std::fs::read(&over).unwrap(), b"new");
        assert!(written
            .iter()
            .any(|(p, prior)| p == &over && prior.as_deref() == Some(b"old")));

        // Cleanup succeeds: the colliding file restores and the copied-only files delete cleanly.
        restore_or_remove(&written, &dst).expect("cleanup should succeed in a writable tmp tree");

        // Copied-only files + the dir the copy created are gone.
        assert!(!top.exists(), "copied top-level file should be removed");
        assert!(!nested.exists(), "copied nested file should be removed");
        assert!(
            !dst.join("AI").exists(),
            "now-empty created dir should be pruned"
        );
        // Non-colliding pre-existing file and the dst root itself survive.
        assert!(pre.exists(), "pre-existing file must be left untouched");
        assert!(dst.exists(), "dst root must not be removed");
        // The overwritten pre-existing file is RESTORED to its original bytes (not deleted).
        assert!(
            over.exists(),
            "overwritten pre-existing file must be restored, not deleted"
        );
        assert_eq!(
            std::fs::read(&over).unwrap(),
            b"old",
            "restored bytes must be the original"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
