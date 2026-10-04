//! Manager source recompilation against an authenticated, immutable pristine game cache.
//!
//! This route runs only the product standalone compiler. It never acquires an install-mutation
//! guard, launches the game, emits vanilla sources, or selects another base on the caller's behalf.
//! Complete authored modules are sparse overlays; the existing FullGraph planner and compiler
//! retain all other modules from the base and perform preservation and reference validation.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::cache::remap::{remap_module_to_base_with_options_and_binds, RemapOptions};
use crate::cache::semantic_observer::observe_whole_cache_semantics_v1;
use crate::cache::splice::{extract_modules, SequentialMiniGuard};
use crate::compile::{
    compile_full_graph_standalone_v1, resolved_path_is_within_v1, FullGraphCompileOptsV1,
    FullGraphCompileOutcomeV1, FullGraphPublicationDispositionV1,
    ProjectCompilerClosingAuditDisposition, StandaloneCompilerRunnerV1,
};
use crate::compiler_backend::{CompilerBackendDiagnosticV1, CompilerBackendNameV1};
use crate::compiler_target::{
    repin_compiler_target_parent_chains_v1, CompilerTargetInputPathsV1, CompilerTargetPinHandlesV1,
    ValidatedCompilerTargetInputsV1,
};
use crate::full_graph_plan::{
    module_name_from_relative_path_v1, plan_named_source_overlays_v1, AuthoredSourceV1,
};
use crate::generation_receipt_v2::read_full_graph_compile_output_bytes_v2;
use crate::standalone_package_resolver::{
    resolve_embedded_product_standalone_compiler_package_for_inputs_v1,
    ProductStandaloneCompilerPackageResolutionV1,
};

/// Packaging bounds for one complete atomic mini group. Components may contain many groups.
pub const MAX_MANAGER_REBUILD_GROUP_MODULES_V1: usize = 256;
pub const MAX_MANAGER_REBUILD_GROUP_SOURCE_BYTES_V1: usize = 64 * 1024 * 1024;
pub const MAX_MANAGER_REBUILD_MODULES_V1: usize = MAX_MANAGER_REBUILD_GROUP_MODULES_V1;
pub const MAX_MANAGER_REBUILD_SOURCE_FILE_BYTES_V1: usize = 16 * 1024 * 1024;
pub const MAX_MANAGER_REBUILD_SOURCE_BYTES_V1: usize = MAX_MANAGER_REBUILD_GROUP_SOURCE_BYTES_V1;
const MAX_BASE_CACHE_BYTES: usize = 512 * 1024 * 1024;
const MAX_BINDS_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;

/// Exact authored bytes, including the complete contents of an edited vanilla module.
#[derive(Debug, Clone, Copy)]
pub struct ManagerRebuildSourceV1<'a> {
    pub module_name: &'a str,
    pub relative_path: &'a str,
    pub source: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct ManagerRebuildInputsV1<'a> {
    /// Product-owned executable/DLL location; its parent selects only the fixed `compiler/` root.
    /// Identity authority still comes exclusively from the embedded product catalog.
    pub host_module: &'a Path,
    /// Installation root containing `G1R`, or the `G1R` directory itself.
    pub game_root: &'a Path,
    /// Manager-selected pristine Shipping file, possibly a deployment-owned backup.
    pub pristine_shipping_cache: &'a Path,
    /// Effective selected base. A raw replacement differing from the pristine file is unsupported.
    pub base_cache: &'a [u8],
    /// The caller resolves load order before passing one set of unique module identities.
    pub sources: &'a [ManagerRebuildSourceV1<'a>],
    /// Existing real directory, strictly outside (and not an ancestor of) the installation.
    pub temporary_root: &'a Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerRebuildErrorKindV1 {
    InvalidInput,
    Unsupported,
    Compiler,
    Audit,
    Cleanup,
}

#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {detail}")]
pub struct ManagerRebuildErrorV1 {
    kind: ManagerRebuildErrorKindV1,
    detail: String,
    diagnostics: Vec<CompilerBackendDiagnosticV1>,
    recovery_required: bool,
}

impl ManagerRebuildErrorV1 {
    fn new(kind: ManagerRebuildErrorKindV1, detail: impl ToString) -> Self {
        Self {
            kind,
            detail: detail.to_string(),
            diagnostics: Vec::new(),
            recovery_required: false,
        }
    }

    pub fn kind(&self) -> ManagerRebuildErrorKindV1 {
        self.kind
    }
    pub fn detail(&self) -> &str {
        &self.detail
    }
    pub fn diagnostics(&self) -> &[CompilerBackendDiagnosticV1] {
        &self.diagnostics
    }
    pub fn recovery_required(&self) -> bool {
        self.recovery_required
    }
}

/// Only authored modules, remapped as one multi-module composition unit onto the selected base.
#[derive(Debug)]
pub struct ManagerRebuildOutputV1 {
    pub mini_cache: Vec<u8>,
    pub module_names: Vec<String>,
    pub diagnostics: Vec<CompilerBackendDiagnosticV1>,
}

/// Holds the authenticated EXE/Shipping/Binds handles and directory pins through caller review.
/// Before acquiring installation ownership, call `prepare_for_commit`. Under that ownership,
/// consume this with `prepare_commit_after_audit` and retain the returned EXE/Binds pins until
/// commit finishes. `release_after_audit` releases all target handles for read-only consumers.
pub struct ManagerRebuildResultV1 {
    output: ManagerRebuildOutputV1,
    target: CompilerTargetPinHandlesV1,
    audit: TargetAuditV1,
}

/// Retains only the immutable compiler target executable and Binds file handles. On Windows
/// these handles deny write/delete sharing while allowing Manager to replace Shipping and write
/// its recovery records. Keep this value alive through the entire guarded Manager commit.
#[must_use = "retain these immutable compiler target pins until Manager commit completes"]
pub struct ManagerRebuildCommitPinsV1 {
    _executable: File,
    _binds: File,
}

impl std::fmt::Debug for ManagerRebuildCommitPinsV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagerRebuildCommitPinsV1")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ManagerRebuildResultV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagerRebuildResultV1")
            .field("module_names", &self.output.module_names)
            .field("mini_cache_byte_len", &self.output.mini_cache.len())
            .field("diagnostics", &self.output.diagnostics)
            .finish_non_exhaustive()
    }
}

impl ManagerRebuildResultV1 {
    pub fn output(&self) -> &ManagerRebuildOutputV1 {
        &self.output
    }

    /// Move the mini to Manager's private staging area while retaining the target pins. A final
    /// `release_after_audit` still audits the target and may return an output with empty mini bytes.
    pub fn take_mini_cache(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.output.mini_cache)
    }

    pub fn audit_target(&mut self) -> Result<(), ManagerRebuildErrorV1> {
        self.audit
            .validate_pins(&mut self.target)
            .map_err(|mut error| {
                error.diagnostics = self.output.diagnostics.clone();
                error
            })
    }

    /// Release only directory pins before Manager acquires its installation lock. The immutable
    /// EXE, Shipping and Binds handles remain live. `prepare_commit_after_audit` rechecks every
    /// path and byte under that lock before releasing Shipping for Manager's planned replacement.
    pub fn prepare_for_commit(&mut self) {
        self.target.directory_pins.clear();
    }

    /// Consume this in Manager's precommit callback, after acquiring installation ownership.
    /// This audits all target paths and bytes, releases Shipping and directory pins, and keeps
    /// EXE/Binds pinned in the returned guard until the guarded commit finishes. Directory pins
    /// deny sibling renames on Windows and would block Shipping/recovery/lock publication.
    pub fn prepare_commit_after_audit(
        mut self,
    ) -> Result<ManagerRebuildCommitPinsV1, ManagerRebuildErrorV1> {
        self.audit_target()?;
        let CompilerTargetPinHandlesV1 {
            executable,
            shipping,
            binds,
            directory_pins,
            paths: _,
        } = self.target;
        drop(shipping);
        drop(directory_pins);
        Ok(ManagerRebuildCommitPinsV1 {
            _executable: executable,
            _binds: binds,
        })
    }

    /// Final read-only audit before a caller-held Manager transaction commits. No pins remain
    /// after this returns, so the transaction can rename its own Shipping cache and backups.
    pub fn release_after_audit(mut self) -> Result<ManagerRebuildOutputV1, ManagerRebuildErrorV1> {
        self.audit_target()?;
        Ok(self.output)
    }
}

/// Stable per-module baseline fingerprints, keyed by exact module name, as lowercase SHA-256 hex.
/// Uses the semantic observer's V1 module digest: random cache GUID and raw pointer/engine IDs
/// are normalized; relative source filename, code hash, and serialized module metadata remain
/// observable content. This is an update-warning signal, not a runtime compatibility proof.
pub fn module_fingerprints(
    base_bytes: &[u8],
) -> Result<BTreeMap<String, String>, ManagerRebuildErrorV1> {
    validate_base_size(base_bytes)?;
    let observation = observe_whole_cache_semantics_v1(base_bytes, None).map_err(|error| {
        ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
    })?;
    let mut fingerprints = BTreeMap::new();
    let mut folded = BTreeSet::new();
    for module in observation.module_identities() {
        if module.map_key() != module.name() || !folded.insert(module.name().to_lowercase()) {
            return Err(ManagerRebuildErrorV1::new(
                ManagerRebuildErrorKindV1::InvalidInput,
                "baseline cache contains ambiguous module identities",
            ));
        }
        fingerprints.insert(module.name().to_owned(), hex(module.semantic_sha256()));
    }
    Ok(fingerprints)
}

/// Compile a composed source overlay set using the authenticated product standalone compiler.
/// Native errors and diagnostics remain strict; accepting a vanilla-baseline warning is a policy
/// decision of Manager and does not enable `--force` or bypass compiler compatibility validation.
pub fn rebuild_manager_sources_v1(
    inputs: ManagerRebuildInputsV1<'_>,
) -> Result<ManagerRebuildResultV1, ManagerRebuildErrorV1> {
    validate_base_size(inputs.base_cache)?;
    validate_sources(inputs.sources)?;
    if crate::force::enabled() {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::Unsupported,
            "Manager source recompilation requires strict checks; process-wide --force is enabled",
        ));
    }
    let game = resolve_game_root(inputs.game_root)?;
    validate_temporary_root(&game, inputs.temporary_root)?;
    if !inputs.pristine_shipping_cache.is_absolute() {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::InvalidInput,
            "selected pristine Shipping cache path must be absolute",
        ));
    }
    let executable = game.join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe");
    let binds = game.join("G1R/Script/Binds.Cache");
    let package = match resolve_embedded_product_standalone_compiler_package_for_inputs_v1(
        inputs.host_module,
        CompilerTargetInputPathsV1 {
            executable: &executable,
            shipping_cache: inputs.pristine_shipping_cache,
            binds_cache: &binds,
        },
    ) {
        ProductStandaloneCompilerPackageResolutionV1::Available(package) => package,
        ProductStandaloneCompilerPackageResolutionV1::BundleAbsent => {
            return Err(ManagerRebuildErrorV1::new(
                ManagerRebuildErrorKindV1::Unsupported,
                "this GORE build contains no authenticated product standalone compiler bundle",
            ));
        }
        ProductStandaloneCompilerPackageResolutionV1::Unavailable(reason) => {
            return Err(ManagerRebuildErrorV1::new(
                ManagerRebuildErrorKindV1::Unsupported,
                format!(
                    "product standalone compiler unavailable ({:?}): {}",
                    reason.kind(),
                    reason.detail()
                ),
            ));
        }
    };
    ensure_pristine_base(package.target_inputs().shipping_cache(), inputs.base_cache)?;
    let workspace = RebuildWorkspaceV1::create(inputs.temporary_root)?;
    let attempt = (|| {
        let mut runner = package
            .sidecar_runner(workspace.root.join("sidecar"))
            .map_err(|error| {
                ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Unsupported, error)
            })?;
        // Keep compiler-package authority live for the entire attempt, without transferring the
        // target to a report that would release its pins before Manager receives the result.
        let (_authority, mut target) = package.into_execution_parts();
        let audit = TargetAuditV1::capture(&mut target)?;
        let output = compile_sources_with_runner(
            &game,
            inputs.base_cache,
            target.binds_cache().to_vec(),
            inputs.sources,
            &workspace,
            &mut runner,
            || audit.validate(&mut target),
        )?;
        audit.validate(&mut target).map_err(|mut error| {
            error.diagnostics = output.diagnostics.clone();
            error
        })?;
        Ok(ManagerRebuildResultV1 {
            output,
            target: target.into_pin_handles(),
            audit,
        })
    })();
    finish_cleanup(workspace, attempt)
}

fn ensure_pristine_base(pristine: &[u8], effective: &[u8]) -> Result<(), ManagerRebuildErrorV1> {
    if pristine != effective {
        return Err(ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Unsupported,
            "effective script base differs from the authenticated selected pristine Shipping cache; raw-cache replacement source recompilation is unsupported"));
    }
    Ok(())
}

fn validate_base_size(bytes: &[u8]) -> Result<(), ManagerRebuildErrorV1> {
    if bytes.is_empty() || bytes.len() > MAX_BASE_CACHE_BYTES {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::InvalidInput,
            "base cache is empty or exceeds 512 MiB",
        ));
    }
    Ok(())
}

fn validate_sources(sources: &[ManagerRebuildSourceV1<'_>]) -> Result<(), ManagerRebuildErrorV1> {
    let invalid =
        |detail| ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, detail);
    if sources.is_empty() || sources.len() > MAX_MANAGER_REBUILD_MODULES_V1 {
        return Err(invalid("source overlay count must be between 1 and 256"));
    }
    let mut total = 0usize;
    let mut names = BTreeSet::new();
    for source in sources {
        module_name_from_relative_path_v1(source.relative_path).map_err(|error| {
            ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
        })?;
        if source.module_name.is_empty()
            || source.module_name.len() > 4096
            || source.module_name.chars().any(char::is_control)
        {
            return Err(invalid("invalid declared module name"));
        }
        if !names.insert(source.module_name.to_lowercase()) {
            return Err(invalid("source overlays contain colliding module names"));
        }
        // Historical paths may have been reused by newer vanilla modules. The named planner
        // rebases each identity before enforcing current path uniqueness; FullGraph then checks
        // the resulting layout against every retained base module before invoking the runner.
        if source.source.len() > MAX_MANAGER_REBUILD_SOURCE_FILE_BYTES_V1 {
            return Err(invalid("authored module source exceeds 16 MiB"));
        }
        total = total
            .checked_add(source.source.len())
            .ok_or_else(|| invalid("source length overflow"))?;
        if total > MAX_MANAGER_REBUILD_SOURCE_BYTES_V1 {
            return Err(invalid("authored source aggregate exceeds 64 MiB"));
        }
        if std::str::from_utf8(source.source).is_err() || source.source.contains(&0) {
            return Err(invalid("authored source must be UTF-8 without NUL bytes"));
        }
    }
    Ok(())
}

fn compile_sources_with_runner<A>(
    game: &Path,
    base: &[u8],
    binds: Vec<u8>,
    sources: &[ManagerRebuildSourceV1<'_>],
    workspace: &RebuildWorkspaceV1,
    runner: &mut dyn StandaloneCompilerRunnerV1,
    closing_audit: A,
) -> Result<ManagerRebuildOutputV1, ManagerRebuildErrorV1>
where
    A: FnOnce() -> Result<(), ManagerRebuildErrorV1>,
{
    validate_sources(sources)?;
    let overlays = sources
        .iter()
        .map(|source| AuthoredSourceV1 {
            module_name: source.module_name.to_owned(),
            relative_path: source.relative_path.to_owned(),
            bytes: source.source.to_vec(),
        })
        .collect();
    let plan = plan_named_source_overlays_v1(base, overlays).map_err(|error| {
        ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
    })?;
    let (changes, final_manifest) = plan.into_parts();
    let module_names = changes
        .iter()
        .map(|change| change.module_name.clone())
        .collect::<Vec<_>>();
    let opts = FullGraphCompileOptsV1 {
        game_dir: game.to_path_buf(),
        work_dir: workspace.root.join("work"),
        output_path: workspace.root.join("output/rebuilt.cache"),
        changes,
        final_manifest,
        base_cache: base.to_vec(),
        binds_cache: binds,
    };
    let report = compile_full_graph_standalone_v1(&opts, runner, || {
        closing_audit().map_err(|error| error.to_string())
    });
    let diagnostics = report.backend_diagnostics().to_vec();
    let recovery_required = report.recovery_required();
    let valid_evidence = report.backend_name() == Some(CompilerBackendNameV1::Standalone)
        && report.standalone_attempted()
        && !report.game_attempted()
        && report.runner_invocations() == 1
        && report.fallback_reason().is_none()
        && report.closing_audit_disposition() == ProjectCompilerClosingAuditDisposition::Passed
        && report.publication_disposition() == FullGraphPublicationDispositionV1::Published
        && !recovery_required;
    match report.outcome {
        FullGraphCompileOutcomeV1::Failed(error) => Err(ManagerRebuildErrorV1 {
            kind: ManagerRebuildErrorKindV1::Compiler,
            detail: error.to_string(),
            diagnostics,
            recovery_required,
        }),
        FullGraphCompileOutcomeV1::Compiled(artifact) => {
            let result = (|| {
                if !valid_evidence {
                    return Err(ManagerRebuildErrorV1::new(
                        ManagerRebuildErrorKindV1::Audit,
                        "compiler output lacks strict standalone publication/audit evidence",
                    ));
                }
                let rebuilt =
                    read_full_graph_compile_output_bytes_v2(&artifact).map_err(|error| {
                        ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Compiler, error)
                    })?;
                let names = module_names.iter().map(String::as_str).collect::<Vec<_>>();
                let extracted = extract_modules(&rebuilt, &names).map_err(|error| {
                    ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Compiler, error)
                })?;
                let (mini_cache, _) = remap_module_to_base_with_options_and_binds(
                    &extracted,
                    base,
                    &opts.binds_cache,
                    RemapOptions {
                        allow_new_symbols: true,
                    },
                )
                .map_err(|error| {
                    ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Compiler, error)
                })?;
                // Validate the exact mini as Manager will compose it, including cross-module refs.
                let mut guard = SequentialMiniGuard::new_with_binds(base, &opts.binds_cache)
                    .map_err(|error| {
                        ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Compiler, error)
                    })?;
                guard.compose_upsert(base, &mini_cache).map_err(|error| {
                    ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Compiler, error)
                })?;
                Ok(ManagerRebuildOutputV1 {
                    mini_cache,
                    module_names,
                    diagnostics: diagnostics.clone(),
                })
            })();
            let neutralized = artifact.neutralize();
            drop(artifact);
            if let Err(error) = neutralized {
                return Err(ManagerRebuildErrorV1 {
                    kind: ManagerRebuildErrorKindV1::Cleanup,
                    detail: error,
                    diagnostics,
                    recovery_required: true,
                });
            }
            result.map_err(|mut error| {
                error.diagnostics = diagnostics;
                error
            })
        }
    }
}

#[derive(Debug)]
struct TargetAuditV1 {
    seals: [(u64, [u8; 32]); 3],
}

impl TargetAuditV1 {
    fn capture(
        target: &mut ValidatedCompilerTargetInputsV1,
    ) -> Result<Self, ManagerRebuildErrorV1> {
        target
            .repin_parent_directories_after_install_mutation_v1()
            .map_err(|error| ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Audit, error))?;
        let seals = [
            file_seal(target.executable_handle(), MAX_EXECUTABLE_BYTES)?,
            file_seal(target.shipping_handle(), MAX_BASE_CACHE_BYTES as u64)?,
            file_seal(target.binds_handle(), MAX_BINDS_BYTES)?,
        ];
        if seals[1]
            != (
                target.shipping_cache().len() as u64,
                Sha256::digest(target.shipping_cache()).into(),
            )
            || seals[2]
                != (
                    target.binds_cache().len() as u64,
                    Sha256::digest(target.binds_cache()).into(),
                )
        {
            return Err(ManagerRebuildErrorV1::new(
                ManagerRebuildErrorKindV1::Audit,
                "pinned Shipping or Binds file differs from the authenticated input snapshot",
            ));
        }
        Ok(Self { seals })
    }

    fn validate(
        &self,
        target: &mut ValidatedCompilerTargetInputsV1,
    ) -> Result<(), ManagerRebuildErrorV1> {
        let current = Self::capture(target)?;
        if current.seals != self.seals {
            return Err(ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Audit,
                "pinned game executable, pristine Shipping cache, or Binds changed during source compilation"));
        }
        Ok(())
    }

    fn validate_pins(
        &self,
        pins: &mut CompilerTargetPinHandlesV1,
    ) -> Result<(), ManagerRebuildErrorV1> {
        pins.directory_pins = repin_compiler_target_parent_chains_v1(
            &pins.paths,
            &pins.executable,
            &pins.shipping,
            &pins.binds,
        )
        .map_err(|error| ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Audit, error))?;
        let current = [
            file_seal(&pins.executable, MAX_EXECUTABLE_BYTES)?,
            file_seal(&pins.shipping, MAX_BASE_CACHE_BYTES as u64)?,
            file_seal(&pins.binds, MAX_BINDS_BYTES)?,
        ];
        if current != self.seals {
            return Err(ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Audit,
                "pinned game executable, pristine Shipping cache, or Binds changed before Manager commit"));
        }
        Ok(())
    }
}

fn file_seal(file: &File, max: u64) -> Result<(u64, [u8; 32]), ManagerRebuildErrorV1> {
    let audit_error = |error| ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::Audit, error);
    let expected = file.metadata().map_err(audit_error)?.len();
    if expected == 0 || expected > max {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::Audit,
            "pinned target file is empty or oversized",
        ));
    }
    let mut digest = Sha256::new();
    let mut offset = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        #[cfg(unix)]
        let read = {
            use std::os::unix::fs::FileExt;
            file.read_at(&mut buffer, offset)
        };
        #[cfg(windows)]
        let read = {
            use std::os::windows::fs::FileExt;
            file.seek_read(&mut buffer, offset)
        };
        let read = read.map_err(audit_error)?;
        if read == 0 {
            break;
        }
        offset += read as u64;
        if offset > max || offset > expected {
            return Err(ManagerRebuildErrorV1::new(
                ManagerRebuildErrorKindV1::Audit,
                "pinned target file grew while being audited",
            ));
        }
        digest.update(&buffer[..read]);
    }
    if offset != expected || file.metadata().map_err(audit_error)?.len() != expected {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::Audit,
            "pinned target file length changed while being audited",
        ));
    }
    Ok((offset, digest.finalize().into()))
}

fn resolve_game_root(game: &Path) -> Result<PathBuf, ManagerRebuildErrorV1> {
    if !game.is_absolute()
        || game
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::InvalidInput,
            "game root must be an absolute path without traversal",
        ));
    }
    let resolved = game.canonicalize().map_err(|error| {
        ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
    })?;
    if resolved
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("G1R"))
    {
        Ok(resolved
            .parent()
            .expect("absolute G1R directory has a parent")
            .to_path_buf())
    } else {
        Ok(resolved)
    }
}

fn validate_temporary_root(game: &Path, root: &Path) -> Result<(), ManagerRebuildErrorV1> {
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::InvalidInput,
            "temporary root must be an absolute existing real directory",
        ));
    }
    for ancestor in root.ancestors() {
        let metadata = std::fs::symlink_metadata(ancestor).map_err(|error| {
            ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
        })?;
        #[cfg(windows)]
        let reparse = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let reparse = metadata.file_type().is_symlink();
        if !metadata.is_dir() || reparse {
            return Err(ManagerRebuildErrorV1::new(
                ManagerRebuildErrorKindV1::InvalidInput,
                "temporary root contains a symlink, reparse point, or non-directory",
            ));
        }
    }
    let resolved = root.canonicalize().map_err(|error| {
        ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
    })?;
    if resolved_path_is_within_v1(&resolved, game) || resolved_path_is_within_v1(game, &resolved) {
        return Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::InvalidInput,
            "temporary root overlaps the game installation",
        ));
    }
    Ok(())
}

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

struct RebuildWorkspaceV1 {
    root: PathBuf,
}

impl RebuildWorkspaceV1 {
    fn create(root: &Path) -> Result<Self, ManagerRebuildErrorV1> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                ManagerRebuildErrorV1::new(ManagerRebuildErrorKindV1::InvalidInput, error)
            })?
            .as_nanos();
        for _ in 0..16 {
            let candidate = root.join(format!(
                "gore-manager-rebuild-{}-{timestamp}-{}",
                std::process::id(),
                NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&candidate) {
                Ok(()) => return Ok(Self { root: candidate }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(ManagerRebuildErrorV1::new(
                        ManagerRebuildErrorKindV1::InvalidInput,
                        error,
                    ))
                }
            }
        }
        Err(ManagerRebuildErrorV1::new(
            ManagerRebuildErrorKindV1::InvalidInput,
            "could not allocate an exclusive source-rebuild workspace",
        ))
    }

    fn cleanup(&self) -> Result<(), std::io::Error> {
        match std::fs::remove_dir_all(&self.root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

impl Drop for RebuildWorkspaceV1 {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn finish_cleanup<T>(
    workspace: RebuildWorkspaceV1,
    result: Result<T, ManagerRebuildErrorV1>,
) -> Result<T, ManagerRebuildErrorV1> {
    if let Err(error) = workspace.cleanup() {
        let diagnostics = result
            .as_ref()
            .err()
            .map(|error| error.diagnostics.clone())
            .unwrap_or_default();
        let primary = result
            .as_ref()
            .err()
            .map(|error| format!("{error}; "))
            .unwrap_or_default();
        return Err(ManagerRebuildErrorV1 {
            kind: ManagerRebuildErrorKindV1::Cleanup,
            detail: format!("{primary}source-rebuild workspace cleanup failed: {error}"),
            diagnostics,
            recovery_required: true,
        });
    }
    result
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::header::CACHE_MAGIC;
    use crate::cache::walk_modules::{module_names, module_ranges};
    use crate::compile::{
        build_full_graph_cache_for_test, StandaloneCompilerInputsV1, StandaloneCompilerOutputV1,
        StandaloneFullGraphCompilerInputsV1,
    };
    use crate::compiler_backend::{
        CompilerBackendDiagnosticSeverityV1, CompilerBackendFailureKindV1, CompilerBackendFailureV1,
    };

    fn cache(modules: &[(&str, &str)]) -> Vec<u8> {
        build_full_graph_cache_for_test(CACHE_MAGIC, [0x31; 16], modules).unwrap()
    }

    fn change_empty_module_hash(bytes: &mut [u8], module: &str) {
        let ranges = module_ranges(bytes).unwrap();
        let (_, start, _) = ranges.iter().find(|(name, _, _)| name == module).unwrap();
        let hash_offset = start + (module.len() + 5) * 2 + 5 * 4;
        bytes[hash_offset..hash_offset + 8].copy_from_slice(&42i64.to_le_bytes());
    }

    fn binds() -> Vec<u8> {
        let mut bytes = 1u32.to_le_bytes().to_vec();
        fn string(bytes: &mut Vec<u8>, value: &str) {
            bytes.extend_from_slice(&((value.len() + 1) as u32).to_le_bytes());
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
        }
        string(&mut bytes, "UNativeType");
        string(&mut bytes, "/Script/Test.NativeType");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        string(&mut bytes, "void NativeCall()");
        string(&mut bytes, "NativeCall");
        bytes.extend_from_slice(&[0u8; 32]);
        bytes
    }

    #[test]
    fn fingerprints_ignore_guid_module_order_and_unrelated_module_content() {
        let original = cache(&[("Keep", "Keep.as"), ("Edit", "Edit.as")]);
        let baseline = module_fingerprints(&original).unwrap();
        let mut reordered = cache(&[("Edit", "Edit.as"), ("Keep", "Keep.as")]);
        reordered[..16].fill(0x92);
        assert_eq!(module_fingerprints(&reordered).unwrap(), baseline);
        change_empty_module_hash(&mut reordered, "Edit");
        let current = module_fingerprints(&reordered).unwrap();
        assert_eq!(current["Keep"], baseline["Keep"]);
        assert_ne!(current["Edit"], baseline["Edit"]);
        assert!(baseline.values().all(|digest| digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))));
    }

    #[test]
    fn fingerprints_use_the_exact_existing_semantic_digest_and_reject_unresolved_refs() {
        let bytes = crate::cache::semantic_observer::tests::synthetic_observer_qualification_cache_for_module_v1("CompleteModel");
        let observation = observe_whole_cache_semantics_v1(&bytes, None).unwrap();
        assert_eq!(
            module_fingerprints(&bytes).unwrap()["CompleteModel"],
            hex(observation.module_identities()[0].semantic_sha256())
        );
        let invalid =
            crate::cache::semantic_observer::tests::synthetic_observer_qualification_fixture_v1()
                .unresolved_runtime_reference;
        assert_eq!(
            module_fingerprints(&invalid).unwrap_err().kind(),
            ManagerRebuildErrorKindV1::InvalidInput
        );
        assert!(module_fingerprints(b"malformed").is_err());
    }

    #[test]
    fn raw_replacement_is_explicitly_unsupported_without_substituting_a_base() {
        let pristine = cache(&[("Base", "Base.as")]);
        let mut raw = pristine.clone();
        raw[..16].fill(0x44);
        ensure_pristine_base(&pristine, &pristine).unwrap();
        let error = ensure_pristine_base(&pristine, &raw).unwrap_err();
        assert_eq!(error.kind(), ManagerRebuildErrorKindV1::Unsupported);
        assert!(error.detail().contains("raw-cache replacement"));
    }

    #[test]
    fn source_validation_refuses_unsafe_colliding_and_unbounded_inputs() {
        for path in [
            "../Escape.as",
            "/Escape.as",
            "Folder\\Escape.as",
            "CON.as",
            "X//Y.as",
        ] {
            let source = ManagerRebuildSourceV1 {
                module_name: "Escape",
                relative_path: path,
                source: b"",
            };
            assert!(validate_sources(&[source]).is_err(), "{path}");
        }
        let source = ManagerRebuildSourceV1 {
            module_name: "A",
            relative_path: "A.as",
            source: b"",
        };
        validate_sources(&[source]).unwrap(); // A complete empty source is valid.
        assert!(validate_sources(&[source, source]).is_err());
        assert!(validate_sources(&vec![source; MAX_MANAGER_REBUILD_MODULES_V1 + 1]).is_err());
        assert!(validate_sources(&[ManagerRebuildSourceV1 {
            module_name: "",
            ..source
        }])
        .is_err());
        assert!(validate_sources(&[ManagerRebuildSourceV1 {
            source: b"\xff",
            ..source
        }])
        .is_err());
        assert!(validate_sources(&[ManagerRebuildSourceV1 {
            source: b"void A() {}\0",
            ..source
        }])
        .is_err());
        let excessive = vec![b' '; MAX_MANAGER_REBUILD_SOURCE_FILE_BYTES_V1 + 1];
        assert!(validate_sources(&[ManagerRebuildSourceV1 {
            source: &excessive,
            ..source
        }])
        .is_err());
        let maximal = vec![b' '; MAX_MANAGER_REBUILD_SOURCE_FILE_BYTES_V1];
        let owned_names = (0..5)
            .map(|index| (format!("M{index}"), format!("M{index}.as")))
            .collect::<Vec<_>>();
        let aggregate = owned_names
            .iter()
            .map(|(name, path)| ManagerRebuildSourceV1 {
                module_name: name,
                relative_path: path,
                source: &maximal,
            })
            .collect::<Vec<_>>();
        assert!(validate_sources(&aggregate)
            .unwrap_err()
            .detail()
            .contains("aggregate"));
    }

    struct FakeRunner {
        output_path: PathBuf,
        output: Vec<u8>,
        edit_path: String,
        added_path: String,
        reject: bool,
        calls: u8,
    }

    fn diagnostic(severity: CompilerBackendDiagnosticSeverityV1) -> CompilerBackendDiagnosticV1 {
        CompilerBackendDiagnosticV1::new(
            severity,
            "fixture_diagnostic".into(),
            "fixture compiler message".into(),
            Some("Edit.as".into()),
            Some(2),
            Some(3),
        )
    }

    impl StandaloneCompilerRunnerV1 for FakeRunner {
        fn run_regen(
            &mut self,
            _: StandaloneCompilerInputsV1<'_>,
        ) -> Result<StandaloneCompilerOutputV1, CompilerBackendFailureV1> {
            panic!("Manager must use the FullGraph route");
        }

        fn run_full_graph(
            &mut self,
            inputs: StandaloneFullGraphCompilerInputsV1<'_>,
        ) -> Result<StandaloneCompilerOutputV1, CompilerBackendFailureV1> {
            self.calls += 1;
            assert_eq!(inputs.changes.len(), 2);
            assert_eq!(inputs.final_manifest.len(), 3);
            assert_eq!(
                std::fs::read(inputs.source_tree.join(&self.edit_path)).unwrap(),
                b"void Edited() {}\n"
            );
            assert_eq!(
                std::fs::read(inputs.source_tree.join(&self.added_path)).unwrap(),
                b"void Added() {}\n"
            );
            assert!(!inputs.source_tree.join("Keep.as").exists());
            if self.reject {
                return Err(CompilerBackendFailureV1::with_diagnostics(
                    CompilerBackendFailureKindV1::Rejected,
                    "fixture source rejected",
                    vec![diagnostic(CompilerBackendDiagnosticSeverityV1::Error)],
                ));
            }
            std::fs::write(&self.output_path, &self.output).unwrap();
            let output_path = self.output_path.clone();
            Ok(StandaloneCompilerOutputV1::with_cleanup_and_diagnostics(
                self.output_path.clone(),
                vec![diagnostic(CompilerBackendDiagnosticSeverityV1::Warning)],
                move || std::fs::remove_file(output_path).map_err(|error| error.to_string()),
            ))
        }
    }

    fn fixture_rebuild(
        reject: bool,
        audit_passes: bool,
        ignore_edit: bool,
    ) -> (Result<ManagerRebuildOutputV1, ManagerRebuildErrorV1>, u8) {
        fixture_rebuild_with_paths(
            reject,
            audit_passes,
            ignore_edit,
            "Edit.as",
            "Edit.as",
            "Mods/New.as",
        )
    }

    fn fixture_rebuild_with_paths(
        reject: bool,
        audit_passes: bool,
        ignore_edit: bool,
        current_path: &str,
        original_path: &str,
        added_path: &str,
    ) -> (Result<ManagerRebuildOutputV1, ManagerRebuildErrorV1>, u8) {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let temporary_root = root.path().join("temporary");
        std::fs::create_dir(&game).unwrap();
        std::fs::create_dir(&temporary_root).unwrap();
        std::fs::write(game.join("sentinel"), b"unchanged game installation").unwrap();
        validate_temporary_root(&game, &temporary_root).unwrap();
        let base = cache(&[("Keep", "Keep.as"), ("Edit", current_path)]);
        let mut output = cache(&[
            ("Keep", "Keep.as"),
            ("Edit", current_path),
            ("Mods.New", "Mods/New.as"),
        ]);
        output[..16].fill(0x76); // Compiler generation differs from the target generation.
        if !ignore_edit {
            change_empty_module_hash(&mut output, "Edit");
        }
        let workspace = RebuildWorkspaceV1::create(&temporary_root).unwrap();
        let workspace_path = workspace.root.clone();
        let mut runner = FakeRunner {
            output_path: workspace.root.join("runner.cache"),
            output,
            edit_path: current_path.into(),
            added_path: "Mods/New.as".into(),
            reject,
            calls: 0,
        };
        let result = compile_sources_with_runner(
            &game,
            &base,
            binds(),
            &[
                ManagerRebuildSourceV1 {
                    module_name: "Mods.New",
                    relative_path: added_path,
                    source: b"void Added() {}\n",
                },
                ManagerRebuildSourceV1 {
                    module_name: "Edit",
                    relative_path: original_path,
                    source: b"void Edited() {}\n",
                },
            ],
            &workspace,
            &mut runner,
            || {
                if audit_passes {
                    Ok(())
                } else {
                    Err(ManagerRebuildErrorV1::new(
                        ManagerRebuildErrorKindV1::Audit,
                        "fixture target drift",
                    ))
                }
            },
        );
        let result = finish_cleanup(workspace, result);
        assert!(!workspace_path.exists());
        assert_eq!(std::fs::read_dir(&temporary_root).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&game).unwrap().count(), 1);
        assert_eq!(
            std::fs::read(game.join("sentinel")).unwrap(),
            b"unchanged game installation"
        );
        if let Ok(result) = &result {
            assert_eq!(&result.mini_cache[..16], &base[..16]);
            let mut guard = SequentialMiniGuard::new_with_binds(&base, &binds()).unwrap();
            let composed = guard.compose_upsert(&base, &result.mini_cache).unwrap();
            assert_eq!(
                module_names(&composed).unwrap(),
                ["Keep", "Edit", "Mods.New"]
            );
            let keep_before = extract_modules(&base, &["Keep"]).unwrap();
            let keep_after = extract_modules(&composed, &["Keep"]).unwrap();
            assert_eq!(keep_before, keep_after);
        }
        (result, runner.calls)
    }

    #[test]
    fn rebuild_compiles_sparse_add_edit_together_and_returns_a_pristine_bound_multi_mini() {
        let (result, calls) = fixture_rebuild(false, true, false);
        let result = result.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.module_names, ["Edit", "Mods.New"]);
        assert_eq!(
            module_names(&result.mini_cache).unwrap(),
            ["Edit", "Mods.New"]
        );
        assert_eq!(
            result.diagnostics[0].severity(),
            CompilerBackendDiagnosticSeverityV1::Warning
        );
    }

    #[test]
    fn rebuild_preserves_declared_names_and_uses_current_canonical_filenames() {
        for (current, original, added) in [
            ("Dir/Fixture.as", "Dir/Fixture.as", "Mods/Unrelated.as"),
            ("Edit.AS", "Edit.as", "Mods/New.as"),
            ("Moved/Edited.as", "Edit.as", "Mods/New.as"),
        ] {
            let (result, calls) =
                fixture_rebuild_with_paths(false, true, false, current, original, added);
            let result = result.unwrap();
            assert_eq!(calls, 1);
            assert_eq!(result.module_names, ["Edit", "Mods.New"]);
            let identities =
                crate::compile::base_full_graph_manifest_v1(&result.mini_cache).unwrap();
            assert!(identities
                .iter()
                .any(|entry| entry.module_name == "Edit" && entry.relative_path == current));
            assert!(identities.iter().any(
                |entry| entry.module_name == "Mods.New" && entry.relative_path == "Mods/New.as"
            ));
        }
    }

    #[test]
    fn rebuild_allows_historical_paths_reused_by_new_vanilla_modules() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let temporary = root.path().join("temporary");
        std::fs::create_dir(&game).unwrap();
        std::fs::create_dir(&temporary).unwrap();
        let base = cache(&[
            ("Keep", "Keep.as"),
            ("Edit", "Moved/Edit.as"),
            ("Mods.New", "Shared.as"),
        ]);
        let mut output = base.clone();
        output[..16].fill(0x76);
        change_empty_module_hash(&mut output, "Edit");
        change_empty_module_hash(&mut output, "Mods.New");
        let workspace = RebuildWorkspaceV1::create(&temporary).unwrap();
        let workspace_path = workspace.root.clone();
        let mut runner = FakeRunner {
            output_path: workspace.root.join("runner.cache"),
            output,
            edit_path: "Moved/Edit.as".into(),
            added_path: "Shared.as".into(),
            reject: false,
            calls: 0,
        };
        let result = compile_sources_with_runner(
            &game,
            &base,
            binds(),
            &[
                ManagerRebuildSourceV1 {
                    module_name: "Edit",
                    relative_path: "Shared.as",
                    source: b"void Edited() {}\n",
                },
                ManagerRebuildSourceV1 {
                    module_name: "Mods.New",
                    relative_path: "Shared.as",
                    source: b"void Added() {}\n",
                },
            ],
            &workspace,
            &mut runner,
            || Ok(()),
        );
        let result = finish_cleanup(workspace, result).unwrap();
        assert_eq!(runner.calls, 1);
        assert!(!workspace_path.exists());
        assert_eq!(std::fs::read_dir(&game).unwrap().count(), 0);
        let mut guard = SequentialMiniGuard::new_with_binds(&base, &binds()).unwrap();
        let composed = guard.compose_upsert(&base, &result.mini_cache).unwrap();
        assert_eq!(
            module_names(&composed).unwrap(),
            ["Keep", "Edit", "Mods.New"]
        );
        assert_eq!(
            extract_modules(&base, &["Keep"]).unwrap(),
            extract_modules(&composed, &["Keep"]).unwrap()
        );
    }

    #[test]
    fn rebuild_rejects_colliding_current_paths_before_the_runner() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let temporary = root.path().join("temporary");
        std::fs::create_dir(&game).unwrap();
        std::fs::create_dir(&temporary).unwrap();
        let workspace = RebuildWorkspaceV1::create(&temporary).unwrap();
        let mut runner = FakeRunner {
            output_path: workspace.root.join("runner.cache"),
            output: Vec::new(),
            edit_path: "Mods/New.as".into(),
            added_path: "Mods/New.as".into(),
            reject: false,
            calls: 0,
        };
        let result = compile_sources_with_runner(
            &game,
            &cache(&[("Keep", "Keep.as"), ("Edit", "Mods/New.as")]),
            binds(),
            &[
                ManagerRebuildSourceV1 {
                    module_name: "Edit",
                    relative_path: "Historical/Edit.as",
                    source: b"void Edited() {}\n",
                },
                ManagerRebuildSourceV1 {
                    module_name: "Mods.New",
                    relative_path: "Historical/New.as",
                    source: b"void Added() {}\n",
                },
            ],
            &workspace,
            &mut runner,
            || Ok(()),
        );
        let error = finish_cleanup(workspace, result).unwrap_err();
        assert_eq!(error.kind(), ManagerRebuildErrorKindV1::InvalidInput);
        assert!(error.detail().contains("collid"));
        assert_eq!(runner.calls, 0);
        assert_eq!(std::fs::read_dir(&game).unwrap().count(), 0);
    }

    #[test]
    fn rejected_source_preserves_diagnostics_and_cleans_every_private_artifact() {
        let (result, calls) = fixture_rebuild(true, true, false);
        let error = result.unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(error.kind(), ManagerRebuildErrorKindV1::Compiler);
        assert_eq!(
            error.diagnostics()[0].severity(),
            CompilerBackendDiagnosticSeverityV1::Error
        );
        assert_eq!(error.diagnostics()[0].source_path(), Some("Edit.as"));
        assert!(!error.recovery_required());
    }

    #[test]
    fn closing_audit_refuses_target_drift_and_disposes_successful_compiler_output() {
        let (result, calls) = fixture_rebuild(false, false, false);
        assert_eq!(calls, 1);
        let error = result.unwrap_err();
        assert!(error.detail().contains("fixture target drift"));
        assert_eq!(
            error.diagnostics()[0].severity(),
            CompilerBackendDiagnosticSeverityV1::Warning
        );
    }

    #[test]
    fn backend_ignored_edit_is_refused_without_force() {
        let (result, calls) = fixture_rebuild(false, true, true);
        assert_eq!(calls, 1);
        assert!(result.unwrap_err().detail().contains("byte-identical"));
    }

    #[test]
    fn temporary_roots_cannot_overlap_the_installation() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let nested = game.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        assert!(validate_temporary_root(&game, &nested).is_err());
        assert!(validate_temporary_root(&game, &game).is_err());
        assert!(validate_temporary_root(&game, root.path()).is_err());
        assert_eq!(std::fs::read_dir(&nested).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn temporary_root_symlink_is_refused_before_any_write() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let alias = root.path().join("alias");
        std::fs::create_dir(&game).unwrap();
        std::os::unix::fs::symlink(&game, &alias).unwrap();
        assert!(validate_temporary_root(&game, &alias).is_err());
        assert_eq!(std::fs::read_dir(&game).unwrap().count(), 0);
    }

    #[test]
    fn target_audit_hashes_retained_handles_and_detects_changed_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("target");
        std::fs::write(&path, b"initial").unwrap();
        let file = File::open(&path).unwrap();
        let initial = file_seal(&file, 64).unwrap();
        std::fs::write(&path, b"changed").unwrap();
        assert_ne!(file_seal(&file, 64).unwrap(), initial);
        assert!(file_seal(&file, 3).is_err());
    }

    #[test]
    fn taking_mini_bytes_retains_only_file_pins_and_keeps_diagnostics() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("exe");
        let shipping = root.path().join("shipping");
        let binds = root.path().join("binds");
        for path in [&executable, &shipping, &binds] {
            std::fs::write(path, b"fixture pinned bytes").unwrap();
        }
        let pins = CompilerTargetPinHandlesV1 {
            executable: File::open(&executable).unwrap(),
            shipping: File::open(&shipping).unwrap(),
            binds: File::open(&binds).unwrap(),
            directory_pins: Vec::new(),
            paths: crate::compiler_target::CompilerTargetOwnedPathsV1::for_test(
                executable, shipping, binds,
            ),
        };
        let audit = TargetAuditV1 {
            seals: [
                file_seal(&pins.executable, MAX_EXECUTABLE_BYTES).unwrap(),
                file_seal(&pins.shipping, MAX_BASE_CACHE_BYTES as u64).unwrap(),
                file_seal(&pins.binds, MAX_BINDS_BYTES).unwrap(),
            ],
        };
        let mini = cache(&[("Added", "Added.as")]);
        let mut result = ManagerRebuildResultV1 {
            output: ManagerRebuildOutputV1 {
                mini_cache: mini.clone(),
                module_names: vec!["Added".into()],
                diagnostics: vec![diagnostic(CompilerBackendDiagnosticSeverityV1::Warning)],
            },
            target: pins,
            audit,
        };
        assert_eq!(result.take_mini_cache(), mini);
        assert!(result.take_mini_cache().is_empty());
        assert_eq!(result.output().module_names, ["Added"]);
        assert_eq!(result.output().diagnostics.len(), 1);
        assert_eq!(result.target.shipping.metadata().unwrap().len(), 20);
        #[cfg(windows)]
        {
            let output = result.release_after_audit().unwrap();
            assert!(output.mini_cache.is_empty());
            assert_eq!(output.diagnostics.len(), 1);
        }
    }

    #[cfg(windows)]
    fn commit_fixture(
        root: &Path,
        deny_target_mutation: bool,
    ) -> (ManagerRebuildResultV1, [PathBuf; 3]) {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };

        let paths = [
            root.join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe"),
            root.join("G1R/Script/PrecompiledScript_Shipping.Cache"),
            root.join("G1R/Script/Binds.Cache"),
        ];
        for path in &paths {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"fixture pinned bytes").unwrap();
        }
        let open = |path: &Path| {
            if deny_target_mutation {
                OpenOptions::new()
                    .read(true)
                    .share_mode(FILE_SHARE_READ)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                    .open(path)
                    .unwrap()
            } else {
                // Allow a simulated out-of-band replacement to exercise the under-lock audit.
                File::open(path).unwrap()
            }
        };
        let target = CompilerTargetPinHandlesV1 {
            executable: open(&paths[0]),
            shipping: open(&paths[1]),
            binds: open(&paths[2]),
            directory_pins: Vec::new(),
            paths: crate::compiler_target::CompilerTargetOwnedPathsV1::for_test(
                paths[0].clone(),
                paths[1].clone(),
                paths[2].clone(),
            ),
        };
        let audit = TargetAuditV1 {
            seals: [
                file_seal(&target.executable, MAX_EXECUTABLE_BYTES).unwrap(),
                file_seal(&target.shipping, MAX_BASE_CACHE_BYTES as u64).unwrap(),
                file_seal(&target.binds, MAX_BINDS_BYTES).unwrap(),
            ],
        };
        let mut result = ManagerRebuildResultV1 {
            output: ManagerRebuildOutputV1 {
                mini_cache: Vec::new(),
                module_names: vec!["Edit".into()],
                diagnostics: vec![diagnostic(CompilerBackendDiagnosticSeverityV1::Warning)],
            },
            target,
            audit,
        };
        result.audit_target().unwrap();
        (result, paths)
    }

    #[cfg(windows)]
    #[test]
    fn commit_pins_allow_shipping_and_recovery_renames_but_keep_exe_and_binds_immutable() {
        let root = tempfile::tempdir().unwrap();
        let (mut result, paths) = commit_fixture(root.path(), true);
        let lock_initial = root.path().join("mutation.initializing");
        let lock_live = root.path().join("mutation.lock");
        std::fs::write(&lock_initial, b"fixture owner").unwrap();
        assert!(std::fs::rename(&lock_initial, &lock_live).is_err());

        result.prepare_for_commit();
        std::fs::rename(&lock_initial, &lock_live).unwrap();
        for path in &paths {
            assert!(std::fs::write(path, b"unauthorized update").is_err());
            assert!(std::fs::rename(path, path.with_extension("displaced")).is_err());
        }
        let guard = result.prepare_commit_after_audit().unwrap();

        let shipping_backup = paths[1].with_extension("gore-backup");
        std::fs::rename(&paths[1], &shipping_backup).unwrap();
        std::fs::write(&paths[1], b"manager rebuilt shipping").unwrap();
        let recovery_initial = root.path().join("recovery.initializing");
        let recovery_record = root.path().join("recovery.json");
        std::fs::write(&recovery_initial, b"manager recovery authority").unwrap();
        std::fs::rename(&recovery_initial, &recovery_record).unwrap();
        for path in [&paths[0], &paths[2]] {
            assert!(std::fs::write(path, b"out-of-band updater").is_err());
            assert!(std::fs::rename(path, path.with_extension("displaced")).is_err());
        }
        assert_eq!(
            std::fs::read(&paths[1]).unwrap(),
            b"manager rebuilt shipping"
        );
        std::fs::remove_file(&lock_live).unwrap();

        drop(guard);
        for path in [&paths[0], &paths[2]] {
            std::fs::write(path, b"updater after completed commit").unwrap();
            std::fs::rename(path, path.with_extension("displaced")).unwrap();
        }
    }

    #[cfg(windows)]
    #[test]
    fn commit_pin_handoff_rejects_identical_byte_replacement_of_every_target() {
        for index in 0..3 {
            let root = tempfile::tempdir().unwrap();
            let (mut result, paths) = commit_fixture(root.path(), false);
            result.prepare_for_commit();
            let original = std::fs::read(&paths[index]).unwrap();
            std::fs::rename(&paths[index], paths[index].with_extension("displaced")).unwrap();
            std::fs::write(&paths[index], &original).unwrap();

            let error = result.prepare_commit_after_audit().unwrap_err();
            assert_eq!(error.kind(), ManagerRebuildErrorKindV1::Audit);
            assert!(error.detail().contains("changed"), "{index}: {error}");
            assert_eq!(error.diagnostics().len(), 1);
            assert_eq!(std::fs::read(&paths[index]).unwrap(), original);
            // A rejected handoff must release all handles rather than strand deployment pins.
            std::fs::remove_file(&paths[index]).unwrap();
        }
    }
}
