//! Internal full-tree differential verifier for compiler-generation qualification.
//!
//! This source-only helper is intentionally not wired into the public `gore` CLI, FFI, MCP, or
//! signing paths. It emits the per-profile publishing evidence consumed by the offline internal
//! package recorder for an already-qualified final sidecar. It produces the embedded-game
//! reference itself through the guarded production path, then runs exactly one strict standalone
//! FullGraph compile over the same in-memory source graph. No caller-supplied cache can stand in
//! for the embedded compiler authority.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use gore_as::cache::bytediff::{self, Filters, NormOpts, Report, Verdict};
use gore_as::cache::semantic_observer::{
    observe_whole_cache_semantics_v1, WholeCacheSemanticObservationV1,
};
use gore_as::compile::{
    acquire_compile_install_mutation, compile_full_graph_standalone_v1_with_target,
    compile_full_graph_with_backend_v1_with_guard_and_target, CompilerBackendModeV1,
    CompilerBackendNameV1, FullGraphCompileArtifactV1, FullGraphCompileOperationV1,
    FullGraphCompileOptsV1, FullGraphCompileOutcomeV1, FullGraphPublicationDispositionV1,
    InstallMutationGuard, InstallRestoreDisposition, ProjectCompilerClosingAuditDisposition,
};
use gore_as::compiler_profile::capture::PROFILE_MANIFEST_FILE_V1;
use gore_as::compiler_profile::manifest::Sha256Digest;
use gore_as::compiler_profile::qualification::QualifiedSidecarIdentityV1;
use gore_as::compiler_target::{CompilerTargetInputPathsV1, ValidatedCompilerTargetInputsV1};
use gore_as::diagnostics::{DiagnosticSeverity, DiagnosticsCaptureDisposition, DiagnosticsOptions};
use gore_as::full_graph_plan::plan_complete_source_tree_v1;
use gore_as::standalone_sidecar::{
    SidecarExecutableSealV1, StandaloneSidecarConfigV1, StandaloneSidecarRunnerV1,
};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

const MAX_SIDECAR_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_BINDS_BYTES: u64 = 128 * 1024 * 1024;
const BYTEDIFF_CONTEXT: usize = 6;
const DIAGNOSTICS_HOOK_ENV: &str = "GORE_AS_DIAGNOSTICS_HOOK";
const USAGE: &str = "usage: gore-as-full-tree-verifier \
<sidecar.exe> <qualified-profile-root> <game-root> \
<G1R-Win64-Shipping.exe> <PrecompiledScript_Shipping.Cache> <Binds.Cache> \
<frozen-source-root> <existing-game-work-root> <existing-standalone-work-root> \
<new-embedded-output.Cache> <new-standalone-output.Cache> \
<new-verification-receipt.json>";

fn main() {
    if let Err(error) = run() {
        eprintln!("full-tree differential verification failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    #[cfg(not(windows))]
    bail!("the authoritative G1R standalone verifier requires Windows Job Object isolation");

    require_release_diagnostics_hook_authority(std::env::var_os(DIAGNOSTICS_HOOK_ENV))?;
    let invocation = Invocation::parse_from(std::env::args_os().skip(1))?;
    invocation.preflight()?;

    // Keep the executable identity pinned across both compiler attempts.
    let sidecar = PinnedBytes::open(&invocation.sidecar, MAX_SIDECAR_BYTES, "standalone sidecar")?;
    let sidecar_seal = SidecarExecutableSealV1 {
        byte_len: sidecar.byte_len,
        sha256: sidecar.sha256,
    };

    let config = StandaloneSidecarConfigV1::new(
        invocation.sidecar.clone(),
        sidecar_seal,
        invocation.profile_root.join(PROFILE_MANIFEST_FILE_V1),
        invocation.profile_root.clone(),
        invocation.standalone_work_root.clone(),
    );
    let mut runner = StandaloneSidecarRunnerV1::new(config)
        .map_err(anyhow::Error::msg)
        .context("initializing the qualified local standalone runner")?;
    let profile_sha256 = runner.profile().profile_sha256;
    let qualified_sidecar = runner.profile_package().standalone_compiler_identity();
    if qualified_sidecar.byte_len != sidecar_seal.byte_len
        || qualified_sidecar.sha256 != sidecar_seal.sha256
    {
        bail!("qualified profile and pinned final sidecar identity disagree");
    }

    let mut target = ValidatedCompilerTargetInputsV1::load(
        runner.profile_package(),
        CompilerTargetInputPathsV1 {
            executable: &invocation.executable,
            shipping_cache: &invocation.shipping_cache,
            binds_cache: &invocation.binds_cache,
        },
    )
    .context("structurally qualifying the explicit executable/Shipping/Binds target")?;
    let target_profile_sha256 = target.profile_sha256();
    if target_profile_sha256 != profile_sha256 {
        bail!("qualified target and standalone runner disagree on compiler profile identity");
    }

    let shipping_seal = MemorySeal::from_bytes(target.shipping_cache());
    let binds_seal = MemorySeal::from_bytes(target.binds_cache());

    // Publish the cross-tool lock only after all static inputs and the final sidecar/profile have
    // passed preflight. The target temporarily releases only the directory pins which would block
    // the guarded compiler's own exact restore, then immediately re-pins under that lock.
    let guard = acquire_game_guard(&invocation.game_root, &mut target)?;
    let planned = (|| -> Result<_> {
        let plan = plan_complete_source_tree_v1(target.shipping_cache(), &invocation.source_root)
            .context("planning the frozen complete source tree")?;
        require_edit_only_full_tree(&plan)?;
        let source_module_count = plan.final_manifest().len();
        let source_seal = seal_frozen_source_tree(&plan)?;
        let (changes, final_manifest) = plan.into_parts();
        Ok((source_module_count, source_seal, changes, final_manifest))
    })();
    let (source_module_count, source_seal, changes, final_manifest) = match planned {
        Ok(planned) => planned,
        Err(error) => return Err(release_guard_after_error(guard, error)),
    };

    let game_opts = FullGraphCompileOptsV1 {
        game_dir: invocation.game_root.clone(),
        work_dir: invocation.game_work_root.clone(),
        output_path: invocation.embedded_output.clone(),
        changes: changes.clone(),
        final_manifest: final_manifest.clone(),
        base_cache: target.shipping_cache().to_vec(),
        binds_cache: target.binds_cache().to_vec(),
    };
    let audit_shipping = invocation.shipping_cache.clone();
    let audit_binds = invocation.binds_cache.clone();
    let closing_audit = move || {
        audit_exact_input(
            &audit_shipping,
            MAX_CACHE_BYTES,
            "Shipping cache",
            shipping_seal,
        )?;
        audit_exact_input(&audit_binds, MAX_BINDS_BYTES, "Binds.Cache", binds_seal)
    };

    let game_report = compile_full_graph_with_backend_v1_with_guard_and_target(
        &game_opts,
        &DiagnosticsOptions::default(),
        CompilerBackendModeV1::Game,
        None,
        guard,
        closing_audit,
        target,
    );
    let (embedded, target) = game_report.finish_while_target_pinned(|report| {
        let embedded = finish_embedded_reference(report, source_module_count)?;
        let next_target = match load_target(&runner, &invocation) {
            Ok(target) => target,
            Err(error) => {
                return Err(neutralize_after_error(
                    error,
                    [("embedded", &embedded.artifact)],
                ))
            }
        };
        if next_target.profile_sha256() != profile_sha256
            || MemorySeal::from_bytes(next_target.shipping_cache()) != shipping_seal
            || MemorySeal::from_bytes(next_target.binds_cache()) != binds_seal
        {
            return Err(neutralize_after_error(
                anyhow::anyhow!(
                    "qualified target identity changed between embedded and standalone attempts"
                ),
                [("embedded", &embedded.artifact)],
            ));
        }
        Ok((embedded, next_target))
    })?;
    drop(game_opts);

    let standalone_opts = FullGraphCompileOptsV1 {
        game_dir: invocation.game_root.clone(),
        work_dir: invocation.standalone_work_root.clone(),
        output_path: invocation.standalone_output.clone(),
        changes,
        final_manifest,
        base_cache: target.shipping_cache().to_vec(),
        binds_cache: target.binds_cache().to_vec(),
    };
    let audit_shipping = invocation.shipping_cache.clone();
    let audit_binds = invocation.binds_cache.clone();
    let closing_audit = move || {
        audit_exact_input(
            &audit_shipping,
            MAX_CACHE_BYTES,
            "Shipping cache",
            shipping_seal,
        )?;
        audit_exact_input(&audit_binds, MAX_BINDS_BYTES, "Binds.Cache", binds_seal)
    };
    let standalone_report = compile_full_graph_standalone_v1_with_target(
        &standalone_opts,
        &mut runner,
        closing_audit,
        target,
    );
    let prior_artifact_failure_disposition =
        prior_artifact_failure_disposition(standalone_report.recovery_required());
    let verified = match standalone_report.finish_while_target_pinned(|report| {
        finish_verification(
            report,
            &embedded,
            source_module_count,
            source_seal,
            profile_sha256,
            qualified_sidecar,
            shipping_seal,
            binds_seal,
        )
    }) {
        Ok(verified) => verified,
        Err(error)
            if prior_artifact_failure_disposition == PriorArtifactFailureDisposition::Preserve =>
        {
            return Err(error)
        }
        Err(error) => {
            return Err(neutralize_after_error(
                error,
                [("embedded", &embedded.artifact)],
            ))
        }
    };
    if let Err(error) = sidecar
        .revalidate()
        .and_then(|_| {
            embedded
                .artifact
                .validate_retained_artifact()
                .map_err(anyhow::Error::msg)
        })
        .and_then(|_| {
            verified
                .artifact
                .validate_retained_artifact()
                .map_err(anyhow::Error::msg)
        })
        .and_then(|_| write_receipt_no_clobber(&invocation.receipt_output, &verified.receipt))
    {
        return Err(neutralize_after_error(
            error,
            [
                ("embedded", &embedded.artifact),
                ("standalone", &verified.artifact),
            ],
        ));
    }
    println!(
        "full-tree verification receipt: {}",
        invocation.receipt_output.display()
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Invocation {
    sidecar: PathBuf,
    profile_root: PathBuf,
    game_root: PathBuf,
    executable: PathBuf,
    shipping_cache: PathBuf,
    binds_cache: PathBuf,
    source_root: PathBuf,
    game_work_root: PathBuf,
    standalone_work_root: PathBuf,
    embedded_output: PathBuf,
    standalone_output: PathBuf,
    receipt_output: PathBuf,
}

impl Invocation {
    fn parse_from(args: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let args = args.into_iter().map(PathBuf::from).collect::<Vec<_>>();
        if args.len() != 12 {
            bail!(USAGE);
        }
        let invocation = Self {
            sidecar: args[0].clone(),
            profile_root: args[1].clone(),
            game_root: args[2].clone(),
            executable: args[3].clone(),
            shipping_cache: args[4].clone(),
            binds_cache: args[5].clone(),
            source_root: args[6].clone(),
            game_work_root: args[7].clone(),
            standalone_work_root: args[8].clone(),
            embedded_output: args[9].clone(),
            standalone_output: args[10].clone(),
            receipt_output: args[11].clone(),
        };
        for (label, path) in invocation.labelled_paths() {
            require_absolute_normalized(path, label)?;
        }
        invocation.validate_target_binding()?;
        Ok(invocation)
    }

    fn labelled_paths(&self) -> [(&'static str, &Path); 12] {
        [
            ("sidecar", &self.sidecar),
            ("qualified profile root", &self.profile_root),
            ("game root", &self.game_root),
            ("game executable", &self.executable),
            ("Shipping cache", &self.shipping_cache),
            ("Binds cache", &self.binds_cache),
            ("frozen source root", &self.source_root),
            ("embedded-game work root", &self.game_work_root),
            ("standalone work root", &self.standalone_work_root),
            ("embedded output", &self.embedded_output),
            ("standalone output", &self.standalone_output),
            ("verification receipt output", &self.receipt_output),
        ]
    }

    fn validate_target_binding(&self) -> Result<()> {
        let g1r = if self
            .game_root
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("G1R"))
        {
            self.game_root.clone()
        } else {
            self.game_root.join("G1R")
        };
        let expected = [
            (
                "game executable",
                &self.executable,
                g1r.join("Binaries")
                    .join("Win64")
                    .join("G1R-Win64-Shipping.exe"),
            ),
            (
                "Shipping cache",
                &self.shipping_cache,
                g1r.join("Script").join("PrecompiledScript_Shipping.Cache"),
            ),
            (
                "Binds cache",
                &self.binds_cache,
                g1r.join("Script").join("Binds.Cache"),
            ),
        ];
        for (label, supplied, expected) in expected {
            if !platform_path_eq(supplied, &expected) {
                bail!(
                    "{label} is not bound to the selected game root: expected {}, got {}",
                    expected.display(),
                    supplied.display()
                );
            }
        }
        Ok(())
    }

    fn preflight(&self) -> Result<()> {
        for (label, path) in [
            ("qualified profile root", self.profile_root.as_path()),
            ("game root", self.game_root.as_path()),
            ("frozen source root", self.source_root.as_path()),
            ("embedded-game work root", self.game_work_root.as_path()),
            ("standalone work root", self.standalone_work_root.as_path()),
        ] {
            ensure_real_directory(path, label)?;
        }
        ensure_new_output_path(&self.embedded_output, "embedded output")?;
        ensure_new_output_path(&self.standalone_output, "standalone output")?;
        ensure_new_output_path(&self.receipt_output, "verification receipt output")?;
        if platform_path_eq(&self.embedded_output, &self.standalone_output)
            || platform_path_eq(&self.embedded_output, &self.receipt_output)
            || platform_path_eq(&self.standalone_output, &self.receipt_output)
        {
            bail!("embedded, standalone, and verification receipt outputs must be distinct");
        }
        let embedded_output_parent = self
            .embedded_output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .context("embedded output has no parent directory")?;
        ensure_real_directory(embedded_output_parent, "embedded output parent")?;
        let standalone_output_parent = self
            .standalone_output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .context("standalone output has no parent directory")?;
        ensure_real_directory(standalone_output_parent, "standalone output parent")?;
        let receipt_parent = self
            .receipt_output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .context("verification receipt output has no parent directory")?;
        ensure_real_directory(receipt_parent, "verification receipt output parent")?;

        let game = self
            .game_root
            .canonicalize()
            .context("resolving game root")?;
        let source = self
            .source_root
            .canonicalize()
            .context("resolving frozen source root")?;
        let profile = self
            .profile_root
            .canonicalize()
            .context("resolving qualified profile root")?;
        let game_work = self
            .game_work_root
            .canonicalize()
            .context("resolving embedded-game work root")?;
        let standalone_work = self
            .standalone_work_root
            .canonicalize()
            .context("resolving standalone work root")?;
        let embedded_output_parent = embedded_output_parent
            .canonicalize()
            .context("resolving embedded output parent")?;
        let standalone_output_parent = standalone_output_parent
            .canonicalize()
            .context("resolving standalone output parent")?;
        let receipt_parent = receipt_parent
            .canonicalize()
            .context("resolving verification receipt output parent")?;
        let sidecar = self
            .sidecar
            .canonicalize()
            .context("resolving standalone sidecar")?;
        for (label, input) in [
            ("frozen source root", &source),
            ("qualified profile root", &profile),
            ("embedded-game work root", &game_work),
            ("standalone work root", &standalone_work),
            ("embedded output parent", &embedded_output_parent),
            ("standalone output parent", &standalone_output_parent),
            ("verification receipt output parent", &receipt_parent),
            ("standalone sidecar", &sidecar),
        ] {
            if path_is_within(input, &game) {
                bail!("{label} must be outside the game installation");
            }
        }
        if paths_overlap(&game_work, &standalone_work) {
            bail!("embedded-game and standalone work roots must be disjoint");
        }
        for (label, work) in [
            ("embedded-game work root", &game_work),
            ("standalone work root", &standalone_work),
        ] {
            if paths_overlap(work, &source)
                || paths_overlap(work, &profile)
                || paths_overlap(work, &embedded_output_parent)
                || paths_overlap(work, &standalone_output_parent)
                || paths_overlap(work, &receipt_parent)
            {
                bail!("{label} must be disjoint from source, profile, and output roots");
            }
        }
        for (label, output_parent) in [
            ("embedded output parent", &embedded_output_parent),
            ("standalone output parent", &standalone_output_parent),
        ] {
            if paths_overlap(output_parent, &source) || paths_overlap(output_parent, &profile) {
                bail!("{label} must be disjoint from source and profile roots");
            }
        }
        if paths_overlap(&receipt_parent, &source) || paths_overlap(&receipt_parent, &profile) {
            bail!(
                "verification receipt output parent must be disjoint from source and profile roots"
            );
        }
        if paths_overlap(&embedded_output_parent, &game_work)
            || paths_overlap(&embedded_output_parent, &standalone_work)
            || paths_overlap(&standalone_output_parent, &game_work)
            || paths_overlap(&standalone_output_parent, &standalone_work)
            || paths_overlap(&receipt_parent, &game_work)
            || paths_overlap(&receipt_parent, &standalone_work)
        {
            bail!("work and output roots must be disjoint");
        }
        Ok(())
    }
}

fn load_target(
    runner: &StandaloneSidecarRunnerV1,
    invocation: &Invocation,
) -> Result<ValidatedCompilerTargetInputsV1> {
    ValidatedCompilerTargetInputsV1::load(
        runner.profile_package(),
        CompilerTargetInputPathsV1 {
            executable: &invocation.executable,
            shipping_cache: &invocation.shipping_cache,
            binds_cache: &invocation.binds_cache,
        },
    )
    .context("re-qualifying the restored executable/Shipping/Binds target")
}

fn acquire_game_guard(
    game_root: &Path,
    target: &mut ValidatedCompilerTargetInputsV1,
) -> Result<InstallMutationGuard> {
    target.release_parent_directory_pins_for_install_mutation_v1();
    let mut guard = acquire_compile_install_mutation(game_root)
        .map_err(anyhow::Error::msg)
        .context("acquiring the full-tree embedded-game install-mutation guard")?;
    if let Err(repin) = target.repin_parent_directories_after_install_mutation_v1() {
        let primary =
            format!("re-pinning compiler target directories after lock publication: {repin}");
        return match guard.release() {
            Ok(()) => Err(anyhow::Error::msg(primary)),
            Err(release) => {
                guard.preserve_for_manual_recovery();
                bail!(
                    "COMPILE_RECOVERY_REQUIRED: {primary}; additionally failed to release the \
                     install-mutation guard: {release}"
                )
            }
        };
    }
    Ok(guard)
}

fn release_guard_after_error(
    mut guard: InstallMutationGuard,
    error: anyhow::Error,
) -> anyhow::Error {
    match guard.release() {
        Ok(()) => error,
        Err(release) => {
            guard.preserve_for_manual_recovery();
            anyhow::anyhow!(
                "COMPILE_RECOVERY_REQUIRED: {error:#}; additionally failed to release the \
                 install-mutation guard: {release}"
            )
        }
    }
}

fn neutralize_after_error<const N: usize>(
    error: anyhow::Error,
    artifacts: [(&str, &FullGraphCompileArtifactV1); N],
) -> anyhow::Error {
    let mut neutralized = Vec::with_capacity(N);
    let mut cleanup_failures = Vec::new();
    for (label, artifact) in artifacts {
        match artifact.neutralize() {
            Ok(()) => neutralized.push(label),
            Err(cleanup) => cleanup_failures.push(format!(
                "{label} output {}: {cleanup}",
                artifact.path().display()
            )),
        }
    }
    if cleanup_failures.is_empty() {
        anyhow::anyhow!(
            "{error:#}; retained {} output(s) were neutralized through their exact handles",
            neutralized.join(" and ")
        )
    } else {
        anyhow::anyhow!(
            "OUTPUT_RECOVERY_REQUIRED: {error:#}; exact-handle neutralization failed for {}",
            cleanup_failures.join("; ")
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PriorArtifactFailureDisposition {
    Preserve,
    Neutralize,
}

fn prior_artifact_failure_disposition(recovery_required: bool) -> PriorArtifactFailureDisposition {
    if recovery_required {
        PriorArtifactFailureDisposition::Preserve
    } else {
        PriorArtifactFailureDisposition::Neutralize
    }
}

fn require_edit_only_full_tree(
    plan: &gore_as::full_graph_plan::PlannedFullGraphSourceTreeV1,
) -> Result<()> {
    if plan.changes().len() != plan.final_manifest().len()
        || plan
            .changes()
            .iter()
            .any(|change| change.operation != FullGraphCompileOperationV1::Edit)
    {
        let adds = plan
            .changes()
            .iter()
            .filter(|change| change.operation == FullGraphCompileOperationV1::Add)
            .count();
        let edits = plan
            .changes()
            .iter()
            .filter(|change| change.operation == FullGraphCompileOperationV1::Edit)
            .count();
        let deletes = plan
            .changes()
            .iter()
            .filter(|change| change.operation == FullGraphCompileOperationV1::Delete)
            .count();
        bail!(
            "frozen full-tree input must cover the exact base module universe as edits only \
             (adds={adds}, edits={edits}, deletes={deletes}, final={})",
            plan.final_manifest().len()
        );
    }
    Ok(())
}

/// Domain-separated aggregate identity of the exact source bytes already pinned into the plan.
/// The planner's canonical order, module/path identities, and length-delimited bytes all
/// participate, so the emitted evidence can be matched to the embedded run's frozen-input record.
fn seal_frozen_source_tree(
    plan: &gore_as::full_graph_plan::PlannedFullGraphSourceTreeV1,
) -> Result<MemorySeal> {
    let mut total = 0u64;
    let mut hash = Sha256::new();
    hash.update(b"gore.as.internal-full-tree-source.v1\0");
    hash.update((plan.changes().len() as u64).to_le_bytes());
    for change in plan.changes() {
        let source = change
            .source
            .as_deref()
            .context("edit-only full-tree plan unexpectedly lacks source bytes")?;
        for identity in [&change.module_name, &change.relative_path] {
            hash.update((identity.len() as u64).to_le_bytes());
            hash.update(identity.as_bytes());
        }
        hash.update((source.len() as u64).to_le_bytes());
        hash.update(source);
        total = total
            .checked_add(source.len() as u64)
            .context("frozen source byte count overflow")?;
    }
    Ok(MemorySeal {
        byte_len: total,
        sha256: Sha256Digest::from_bytes(hash.finalize().into()),
    })
}

struct VerifiedEmbeddedReference {
    artifact: FullGraphCompileArtifactV1,
    bytes: Vec<u8>,
    semantics: WholeCacheSemanticObservationV1,
    backend_diagnostic_count: usize,
    diagnostic_count: usize,
}

struct VerifiedFullTree {
    receipt: Value,
    artifact: FullGraphCompileArtifactV1,
}

fn finish_embedded_reference(
    report: gore_as::compile::FullGraphCompileReportV1,
    source_module_count: usize,
) -> Result<VerifiedEmbeddedReference> {
    let backend = report.backend_name();
    let install_restore = report.install_restore_disposition();
    let closing_audit = report.closing_audit_disposition();
    let publication = report.publication_disposition();
    let runner_invocations = report.runner_invocations();
    let standalone_attempted = report.standalone_attempted();
    let game_attempted = report.game_attempted();
    let recovery_required = report.recovery_required();
    let fallback_present = report.fallback_reason().is_some();
    let backend_diagnostic_count = report.backend_diagnostics().len();
    let diagnostics = report.diagnostics();
    let diagnostics_captured = diagnostics.is_some_and(|diagnostics| {
        diagnostics.disposition() == DiagnosticsCaptureDisposition::Captured
    });
    let diagnostic_count = diagnostics
        .map(|diagnostics| diagnostics.diagnostics().len())
        .unwrap_or_default();
    let diagnostic_error_count = diagnostics
        .map(|diagnostics| {
            diagnostics
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
                .count()
        })
        .unwrap_or_default();
    let exact_game_contract = backend == Some(CompilerBackendNameV1::Game)
        && install_restore == InstallRestoreDisposition::RestoredExact
        && closing_audit == ProjectCompilerClosingAuditDisposition::Passed
        && publication == FullGraphPublicationDispositionV1::Published
        && runner_invocations == 1
        && !standalone_attempted
        && game_attempted
        && !recovery_required
        && !fallback_present
        && backend_diagnostic_count == 0
        && diagnostics_captured
        && diagnostic_error_count == 0;

    let artifact = match report.outcome {
        FullGraphCompileOutcomeV1::Compiled(artifact) => artifact,
        FullGraphCompileOutcomeV1::Failed(error) => {
            if recovery_required {
                bail!("COMPILE_RECOVERY_REQUIRED: {error}");
            }
            return Err(anyhow::Error::new(error))
                .context("compiling the frozen full source graph with the embedded game compiler");
        }
    };
    if recovery_required {
        bail!(
            "COMPILE_RECOVERY_REQUIRED: embedded compiler returned an artifact while recovery \
             remains required; output was retained"
        );
    }
    if !exact_game_contract {
        return Err(neutralize_after_error(
            anyhow::anyhow!(
                "embedded reference lacks the exact game-only/diagnostics/restore/audit/publication contract"
            ),
            [("embedded", &artifact)],
        ));
    }

    let verified = (|| -> Result<(Vec<u8>, WholeCacheSemanticObservationV1)> {
        artifact
            .validate_retained_artifact()
            .map_err(anyhow::Error::msg)
            .context("revalidating the retained embedded-game artifact")?;
        let bytes =
            gore_as::generation_receipt_v2::read_full_graph_compile_output_bytes_v2(&artifact)
                .context("reading the embedded-game cache through its retained handle")?;
        let seal = MemorySeal::from_bytes(&bytes);
        if seal.byte_len != artifact.byte_len() || seal.sha256 != artifact.sha256() {
            bail!("embedded-game bytes do not match the retained artifact seal");
        }
        if artifact.module_count() as usize != source_module_count {
            bail!("embedded-game module count does not match the frozen full-tree plan");
        }
        let semantics = observe_whole_cache_semantics_v1(&bytes, None)
            .context("observing the complete embedded-game reference cache")?;
        artifact
            .validate_retained_artifact()
            .map_err(anyhow::Error::msg)
            .context("revalidating the retained embedded-game artifact after its exact read")?;
        Ok((bytes, semantics))
    })();
    match verified {
        Ok((bytes, semantics)) => Ok(VerifiedEmbeddedReference {
            artifact,
            bytes,
            semantics,
            backend_diagnostic_count,
            diagnostic_count,
        }),
        Err(error) => Err(neutralize_after_error(error, [("embedded", &artifact)])),
    }
}

#[allow(clippy::too_many_arguments)]
fn finish_verification(
    report: gore_as::compile::FullGraphCompileReportV1,
    embedded: &VerifiedEmbeddedReference,
    source_module_count: usize,
    source_seal: MemorySeal,
    profile_sha256: Sha256Digest,
    sidecar: QualifiedSidecarIdentityV1,
    shipping_seal: MemorySeal,
    binds_seal: MemorySeal,
) -> Result<VerifiedFullTree> {
    let backend = report.backend_name();
    let install_restore = report.install_restore_disposition();
    let closing_audit = report.closing_audit_disposition();
    let publication = report.publication_disposition();
    let runner_invocations = report.runner_invocations();
    let standalone_attempted = report.standalone_attempted();
    let game_attempted = report.game_attempted();
    let recovery_required = report.recovery_required();
    let fallback_present = report.fallback_reason().is_some();
    let backend_diagnostic_count = report.backend_diagnostics().len();
    let exact_standalone_contract = backend == Some(CompilerBackendNameV1::Standalone)
        && install_restore == InstallRestoreDisposition::NotStarted
        && closing_audit == ProjectCompilerClosingAuditDisposition::Passed
        && publication == FullGraphPublicationDispositionV1::Published
        && runner_invocations == 1
        && standalone_attempted
        && !game_attempted
        && !recovery_required
        && !fallback_present;

    let artifact = match report.outcome {
        FullGraphCompileOutcomeV1::Compiled(artifact) if exact_standalone_contract => artifact,
        FullGraphCompileOutcomeV1::Compiled(artifact) => {
            if recovery_required {
                bail!(
                    "COMPILE_RECOVERY_REQUIRED: standalone compiler returned an artifact while \
                     recovery remains required; output was retained"
                );
            }
            return Err(neutralize_after_error(
                anyhow::anyhow!(
                    "standalone runner returned a cache without the exact strict-standalone/audit/publication contract"
                ),
                [("standalone", &artifact)],
            ));
        }
        FullGraphCompileOutcomeV1::Failed(error) => {
            if recovery_required {
                bail!("COMPILE_RECOVERY_REQUIRED: {error}");
            }
            return Err(anyhow::Error::new(error))
                .context("compiling the frozen full source graph");
        }
    };
    let verified = (|| -> Result<Value> {
        artifact
            .validate_retained_artifact()
            .map_err(anyhow::Error::msg)
            .context("revalidating the retained standalone artifact")?;
        embedded
            .artifact
            .validate_retained_artifact()
            .map_err(anyhow::Error::msg)
            .context("revalidating the retained embedded-game artifact before comparison")?;
        if embedded.artifact.base_cache_sha256() != artifact.base_cache_sha256()
            || embedded.artifact.module_count() != artifact.module_count()
            || embedded.artifact.final_manifest() != artifact.final_manifest()
            || embedded.artifact.changes() != artifact.changes()
            || embedded.artifact.deleted_modules() != artifact.deleted_modules()
        {
            bail!("embedded-game and standalone artifacts were not built from one exact graph");
        }
        let standalone =
            gore_as::generation_receipt_v2::read_full_graph_compile_output_bytes_v2(&artifact)
                .context("reading the published standalone cache through its retained handle")?;
        let standalone_seal = MemorySeal::from_bytes(&standalone);
        if standalone_seal.byte_len != artifact.byte_len()
            || standalone_seal.sha256 != artifact.sha256()
        {
            bail!("published standalone bytes do not match the retained artifact seal");
        }
        if artifact.module_count() as usize != source_module_count {
            bail!("published standalone module count does not match the frozen full-tree plan");
        }
        artifact
            .validate_retained_artifact()
            .map_err(anyhow::Error::msg)
            .context("revalidating the retained standalone artifact after its exact read")?;

        let norm = NormOpts::default();
        let byte_report = bytediff::run(
            &embedded.bytes,
            &standalone,
            &norm,
            &Filters::default(),
            BYTEDIFF_CONTEXT,
        )
        .context("running fail-on-semantic bytediff-equivalent comparison")?;
        let standalone_semantics = observe_whole_cache_semantics_v1(&standalone, None)
            .context("observing the complete standalone cache semantics")?;
        let bytediff_passed = !byte_report.any_semantic();
        let whole_semantics_equal = embedded.semantics == standalone_semantics;
        if let Err(gate) = require_differential_acceptance(bytediff_passed, whole_semantics_equal) {
            bail!(
                "differential verification rejected the standalone cache at {} (published \
                 comparison outputs will be neutralized): {gate}; {}; {}",
                artifact.path().display(),
                bytediff_failure_summary(&byte_report),
                semantic_failure_summary(&embedded.semantics, &standalone_semantics)
            );
        }

        Ok(json!({
        "schema": "gore.as.internal-full-tree-verification",
        "version": 2,
        "passed": true,
        "execution": {
            "embedded_game": {
                "backend": "game",
                "runner_invocations": 1,
                "standalone_attempted": false,
                "game_attempted": true,
                "install_restore": "restored_exact",
                "closing_audit": "passed",
                "publication": "published",
                "recovery_required": false,
                "fallback_present": false,
                "backend_diagnostic_count": embedded.backend_diagnostic_count,
                "diagnostics_disposition": "captured",
                "diagnostic_count": embedded.diagnostic_count,
            },
            "standalone": {
                "backend": "standalone",
                "runner_invocations": runner_invocations,
                "standalone_attempted": standalone_attempted,
                "game_attempted": game_attempted,
                "install_restore": "not_started",
                "closing_audit": "passed",
                "publication": "published",
                "recovery_required": recovery_required,
                "fallback_present": fallback_present,
                "backend_diagnostic_count": backend_diagnostic_count,
            },
        },
        "authority": {
            "qualified_profile_sha256": profile_sha256,
            "sidecar": {
                "byte_len": sidecar.byte_len,
                "sha256": sidecar.sha256,
                "request_version": sidecar.request_version,
                "response_version": sidecar.response_version,
            },
            "shipping": {
                "byte_len": shipping_seal.byte_len,
                "sha256": shipping_seal.sha256,
            },
            "binds": {
                "byte_len": binds_seal.byte_len,
                "sha256": binds_seal.sha256,
            },
        },
        "frozen_source": {
            "module_count": source_module_count,
            "byte_len": source_seal.byte_len,
            "aggregate_sha256": source_seal.sha256,
            "operations": { "add": 0, "edit": source_module_count, "delete": 0 },
        },
        "embedded_reference": {
            "byte_len": embedded.artifact.byte_len(),
            "sha256": embedded.artifact.sha256(),
            "module_count": embedded.artifact.module_count(),
        },
        "standalone_candidate": {
            "byte_len": artifact.byte_len(),
            "sha256": artifact.sha256(),
            "module_count": artifact.module_count(),
        },
        "bytediff": {
            "equivalent_to_fail_on_semantic": true,
            "context": BYTEDIFF_CONTEXT,
            "normalization": {
                "n1_refs": norm.n1_refs,
                "n2_slots": norm.n2_slots,
                "n3_jumps": norm.n3_jumps,
                "n4_consts": norm.n4_consts,
                "n5_scope": norm.n5_scope,
                "n6_reguard": norm.n6_reguard,
            },
            "aligned_functions": byte_report.diffs.len(),
            "identical": byte_report.count(Verdict::Identical),
            "benign": byte_report.count(Verdict::Benign),
            "semantic": byte_report.count(Verdict::Semantic),
            "alignment_loss": byte_report.alignment_loss_count(),
        },
        "whole_cache_semantics_v1": {
            "exact_struct_equality": true,
            "semantic_sha256": standalone_semantics.sha256_hex(),
            "module_count": standalone_semantics.module_count(),
            "function_count": standalone_semantics.function_count(),
            "opcode_counts": &standalone_semantics.opcode_counts()[..],
            "class_count": standalone_semantics.class_count(),
            "behaviour_function_count": standalone_semantics.behaviour_function_count(),
            "property_count": standalone_semantics.property_count(),
            "global_count": standalone_semantics.global_count(),
            "initializer_function_count": standalone_semantics.initializer_function_count(),
            "string_global_reference_count": standalone_semantics.string_global_reference_count(),
            "tail_table_counts": standalone_semantics.tail_table_counts(),
            "invoke_return_included": standalone_semantics.invoke_return_included(),
        },
        }))
    })();
    match verified {
        Ok(receipt) => Ok(VerifiedFullTree { receipt, artifact }),
        Err(error) => Err(neutralize_after_error(error, [("standalone", &artifact)])),
    }
}

fn require_differential_acceptance(
    bytediff_passed: bool,
    whole_semantics_equal: bool,
) -> Result<()> {
    match (bytediff_passed, whole_semantics_equal) {
        (true, true) => Ok(()),
        (false, true) => bail!("Bytediff semantic gate failed"),
        (true, false) => bail!("WholeCache semantic gate failed"),
        (false, false) => bail!("Bytediff and WholeCache semantic gates failed"),
    }
}

fn require_release_diagnostics_hook_authority(override_path: Option<OsString>) -> Result<()> {
    if override_path.is_some() {
        bail!(
            "{DIAGNOSTICS_HOOK_ENV} must be unset for authoritative full-tree verification; \
             release evidence requires the SHA-256-verified sibling or embedded diagnostics hook"
        );
    }
    Ok(())
}

fn bytediff_failure_summary(report: &Report) -> String {
    let semantic_names = report
        .diffs
        .iter()
        .filter(|diff| diff.verdict == Verdict::Semantic)
        .take(3)
        .map(|diff| diff.name.as_str())
        .collect::<Vec<_>>();
    format!(
        "bytediff semantic={} alignment_loss={} first_semantic={semantic_names:?}",
        report.count(Verdict::Semantic),
        report.alignment_loss_count()
    )
}

fn semantic_failure_summary(
    embedded: &WholeCacheSemanticObservationV1,
    standalone: &WholeCacheSemanticObservationV1,
) -> String {
    if embedded == standalone {
        return "WholeCacheSemanticObservationV1 exact equality passed".to_owned();
    }
    let mut fields = Vec::new();
    if embedded.sha256() != standalone.sha256() {
        fields.push("semantic_sha256");
    }
    if embedded.module_count() != standalone.module_count() {
        fields.push("module_count");
    }
    if embedded.function_count() != standalone.function_count() {
        fields.push("function_count");
    }
    if embedded.opcode_counts() != standalone.opcode_counts() {
        fields.push("opcode_counts");
    }
    if embedded.class_count() != standalone.class_count() {
        fields.push("class_count");
    }
    if embedded.behaviour_function_count() != standalone.behaviour_function_count() {
        fields.push("behaviour_function_count");
    }
    if embedded.property_count() != standalone.property_count() {
        fields.push("property_count");
    }
    if embedded.global_count() != standalone.global_count() {
        fields.push("global_count");
    }
    if embedded.initializer_function_count() != standalone.initializer_function_count() {
        fields.push("initializer_function_count");
    }
    if embedded.string_global_reference_count() != standalone.string_global_reference_count() {
        fields.push("string_global_reference_count");
    }
    if embedded.static_names() != standalone.static_names() {
        fields.push("static_names");
    }
    if embedded.module_identities() != standalone.module_identities() {
        fields.push("module_identities");
    }
    if embedded.property_identities() != standalone.property_identities() {
        fields.push("property_identities");
    }
    if embedded.tail_table_counts() != standalone.tail_table_counts() {
        fields.push("tail_table_counts");
    }
    if embedded.invoke_return_included() != standalone.invoke_return_included() {
        fields.push("invoke_return_included");
    }
    format!(
        "WholeCacheSemanticObservationV1 mismatch in [{}] (embedded={}, standalone={})",
        fields.join(","),
        embedded.sha256_hex(),
        standalone.sha256_hex()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemorySeal {
    byte_len: u64,
    sha256: Sha256Digest,
}

impl MemorySeal {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
        }
    }
}

struct PinnedBytes {
    _file: File,
    byte_len: u64,
    sha256: Sha256Digest,
    path: PathBuf,
    maximum: u64,
    label: &'static str,
}

impl PinnedBytes {
    fn open(path: &Path, maximum: u64, label: &'static str) -> Result<Self> {
        Self::from_file(open_regular_no_follow(path, label)?, path, maximum, label)
    }

    fn from_file(mut file: File, path: &Path, maximum: u64, label: &'static str) -> Result<Self> {
        let bytes = read_exact_bounded_handle(&mut file, maximum, path, label)?;
        let seal = MemorySeal::from_bytes(&bytes);
        Ok(Self {
            _file: file,
            byte_len: seal.byte_len,
            sha256: seal.sha256,
            path: path.to_path_buf(),
            maximum,
            label,
        })
    }

    fn revalidate(&self) -> Result<()> {
        let mut file = self
            ._file
            .try_clone()
            .with_context(|| format!("cloning pinned {} handle", self.label))?;
        let bytes = read_exact_bounded_handle(&mut file, self.maximum, &self.path, self.label)?;
        if bytes.len() as u64 != self.byte_len
            || Sha256Digest::from_bytes(Sha256::digest(&bytes).into()) != self.sha256
        {
            bail!("{} changed while verification was running", self.label);
        }
        Ok(())
    }
}

fn audit_exact_input(
    path: &Path,
    maximum: u64,
    label: &'static str,
    expected: MemorySeal,
) -> std::result::Result<(), String> {
    let current = PinnedBytes::open(path, maximum, label).map_err(|error| error.to_string())?;
    if current.byte_len != expected.byte_len || current.sha256 != expected.sha256 {
        return Err(format!("{label} changed during standalone compilation"));
    }
    Ok(())
}

fn read_exact_bounded_handle(
    file: &mut File,
    maximum: u64,
    path: &Path,
    label: &str,
) -> Result<Vec<u8>> {
    let metadata = file
        .metadata()
        .with_context(|| format!("inspecting {label} {}", path.display()))?;
    if !metadata.is_file() || metadata.len() > maximum {
        bail!(
            "{label} is not a bounded regular file (length {}, maximum {maximum}): {}",
            metadata.len(),
            path.display()
        );
    }
    let capacity = usize::try_from(metadata.len()).context("file is too large to address")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .with_context(|| format!("allocating {label} buffer"))?;
    file.seek(SeekFrom::Start(0))?;
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() || file.metadata()?.len() != metadata.len() {
        bail!("{label} changed while reading: {}", path.display());
    }
    Ok(bytes)
}

fn ensure_new_output_path(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => bail!("{label} already exists (no-clobber): {}", path.display()),
        Err(error) => Err(error).with_context(|| format!("inspecting {label} {}", path.display())),
    }
}

fn write_receipt_no_clobber(path: &Path, receipt: &Value) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(receipt)
        .context("serializing canonical full-tree verification receipt")?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "creating full-tree verification receipt without replacement {}",
                path.display()
            )
        })?;
    let publication = (|| -> Result<()> {
        file.write_all(&bytes)
            .context("writing full-tree verification receipt")?;
        file.sync_all()
            .context("syncing full-tree verification receipt")?;
        let metadata = file
            .metadata()
            .context("inspecting full-tree verification receipt")?;
        if !metadata.is_file() || metadata.len() != bytes.len() as u64 {
            bail!("full-tree verification receipt was not written as one complete regular file");
        }
        Ok(())
    })();
    if let Err(error) = publication {
        if let Err(cleanup) = file.set_len(0).and_then(|_| file.sync_all()) {
            bail!(
                "RECEIPT_RECOVERY_REQUIRED: {error:#}; additionally failed to neutralize the \
                 partial receipt through its exact handle: {cleanup}"
            );
        }
        bail!(
            "{error:#}; the partial verification receipt was neutralized through its exact handle"
        );
    }
    Ok(())
}

fn ensure_real_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting {label} {}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) {
        bail!("{label} must be an existing real, non-reparse directory");
    }
    Ok(())
}

fn require_absolute_normalized(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        bail!(
            "{label} path must be absolute and normalized: {}",
            path.display()
        );
    }
    Ok(())
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path, label: &str) -> Result<File> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    };

    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .with_context(|| format!("opening {label} {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        bail!(
            "{label} is not a non-reparse regular file: {}",
            path.display()
        );
    }
    Ok(file)
}

#[cfg(not(windows))]
fn open_regular_no_follow(path: &Path, label: &str) -> Result<File> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!("{label} is not a regular file: {}", path.display());
    }
    OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("opening {label} {}", path.display()))
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn platform_path_eq(left: &Path, right: &Path) -> bool {
    path_key(left) == path_key(right)
}

#[cfg(not(windows))]
fn platform_path_eq(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(windows)]
fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = path_key(path);
    let root = path_key(root).trim_end_matches('\\').to_owned();
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|tail| tail.starts_with('\\'))
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    path_is_within(left, right) || path_is_within(right, left)
}

#[cfg(not(windows))]
fn path_is_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_args(root: &Path) -> Vec<OsString> {
        let game = root.join("game");
        vec![
            root.join("sidecar.exe").into_os_string(),
            root.join("profile").into_os_string(),
            game.clone().into_os_string(),
            game.join("G1R")
                .join("Binaries")
                .join("Win64")
                .join("G1R-Win64-Shipping.exe")
                .into_os_string(),
            game.join("G1R")
                .join("Script")
                .join("PrecompiledScript_Shipping.Cache")
                .into_os_string(),
            game.join("G1R")
                .join("Script")
                .join("Binds.Cache")
                .into_os_string(),
            root.join("source").into_os_string(),
            root.join("game-work").into_os_string(),
            root.join("standalone-work").into_os_string(),
            root.join("out").join("embedded.Cache").into_os_string(),
            root.join("out").join("standalone.Cache").into_os_string(),
            root.join("out").join("verification.json").into_os_string(),
        ]
    }

    fn create_preflight_directories(args: &[OsString]) {
        for index in [1usize, 2, 6, 7, 8] {
            fs::create_dir_all(PathBuf::from(&args[index])).unwrap();
        }
        for index in [9usize, 10, 11] {
            fs::create_dir_all(PathBuf::from(&args[index]).parent().unwrap()).unwrap();
        }
        fs::write(PathBuf::from(&args[0]), b"sidecar").unwrap();
    }

    #[test]
    fn arguments_are_exact_and_positional() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let args = valid_args(&root);
        let parsed = Invocation::parse_from(args.clone()).unwrap();
        assert_eq!(parsed.embedded_output, PathBuf::from(&args[9]));
        assert_eq!(parsed.standalone_output, PathBuf::from(&args[10]));
        assert_eq!(parsed.receipt_output, PathBuf::from(&args[11]));

        assert!(Invocation::parse_from(args[..11].iter().cloned())
            .unwrap_err()
            .to_string()
            .contains(USAGE));
        let mut extra = args;
        extra.push(root.join("extra").into_os_string());
        assert!(Invocation::parse_from(extra)
            .unwrap_err()
            .to_string()
            .contains(USAGE));
    }

    #[test]
    fn target_paths_are_bound_to_the_selected_game_layout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut args = valid_args(&root);
        args[5] = root.join("other-Binds.Cache").into_os_string();
        let error = Invocation::parse_from(args).unwrap_err().to_string();
        assert!(error.contains("Binds cache is not bound to the selected game root"));
    }

    #[test]
    fn game_and_standalone_work_roots_must_be_disjoint() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let mut args = valid_args(&root);
        args[8] = args[7].clone();
        create_preflight_directories(&args);
        let error = Invocation::parse_from(args)
            .unwrap()
            .preflight()
            .unwrap_err()
            .to_string();
        assert!(error.contains("work roots must be disjoint"), "{error}");
    }

    #[test]
    fn all_three_outputs_must_be_distinct() {
        for pair in [(9usize, 10usize), (9, 11), (10, 11)] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let mut args = valid_args(&root);
            args[pair.1] = args[pair.0].clone();
            create_preflight_directories(&args);
            let error = Invocation::parse_from(args)
                .unwrap()
                .preflight()
                .unwrap_err()
                .to_string();
            assert!(error.contains("outputs must be distinct"), "{error}");
        }
    }

    #[test]
    fn existing_output_is_refused_without_clobbering() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("already.Cache");
        fs::write(&output, b"keep-me").unwrap();
        let error = ensure_new_output_path(&output, "standalone output")
            .unwrap_err()
            .to_string();
        assert!(error.contains("already exists (no-clobber)"));
        assert_eq!(fs::read(&output).unwrap(), b"keep-me");
    }

    #[test]
    fn receipt_is_canonical_utf8_and_never_replaced() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("verification.json");
        let receipt = json!({"passed": true, "count": 2});
        write_receipt_no_clobber(&output, &receipt).unwrap();
        let mut expected = serde_json::to_vec_pretty(&receipt).unwrap();
        expected.push(b'\n');
        assert_eq!(fs::read(&output).unwrap(), expected);
        assert!(write_receipt_no_clobber(&output, &receipt).is_err());
        assert_eq!(fs::read(&output).unwrap(), expected);
    }

    #[test]
    fn work_and_output_roots_are_disjoint_from_frozen_inputs() {
        for (target, input, expected) in [
            (7usize, 6usize, "work root must be disjoint"),
            (7, 1, "work root must be disjoint"),
            (8, 6, "work root must be disjoint"),
            (8, 1, "work root must be disjoint"),
            (9, 6, "output parent must be disjoint"),
            (9, 1, "output parent must be disjoint"),
            (10, 6, "output parent must be disjoint"),
            (10, 1, "output parent must be disjoint"),
            (11, 6, "receipt output parent must be disjoint"),
            (11, 1, "receipt output parent must be disjoint"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let mut args = valid_args(&root);
            let input_root = PathBuf::from(&args[input]);
            args[target] = if matches!(target, 7 | 8) {
                input_root.join("work").into_os_string()
            } else {
                input_root
                    .join("output")
                    .join("standalone.Cache")
                    .into_os_string()
            };
            create_preflight_directories(&args);
            let error = Invocation::parse_from(args)
                .unwrap()
                .preflight()
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn differential_acceptance_requires_both_independent_gates() {
        for (bytediff, whole_cache, rejection) in [
            (true, true, None),
            (false, true, Some("Bytediff semantic gate failed")),
            (true, false, Some("WholeCache semantic gate failed")),
            (
                false,
                false,
                Some("Bytediff and WholeCache semantic gates failed"),
            ),
        ] {
            let result = require_differential_acceptance(bytediff, whole_cache);
            match rejection {
                None => assert!(result.is_ok()),
                Some(expected) => {
                    let error = result.unwrap_err().to_string();
                    assert!(error.contains(expected), "{error}");
                }
            }
        }
    }

    #[test]
    fn ambient_diagnostics_hook_override_is_refused_for_release_evidence() {
        require_release_diagnostics_hook_authority(None).unwrap();
        let error = require_release_diagnostics_hook_authority(Some(
            r"C:\dev\gore-as-diagnostics-hook.dll".into(),
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains(DIAGNOSTICS_HOOK_ENV), "{error}");
        assert!(error.contains("SHA-256-verified"), "{error}");
    }

    #[test]
    fn standalone_recovery_preserves_the_prior_embedded_reference() {
        assert_eq!(
            prior_artifact_failure_disposition(true),
            PriorArtifactFailureDisposition::Preserve
        );
        assert_eq!(
            prior_artifact_failure_disposition(false),
            PriorArtifactFailureDisposition::Neutralize
        );
    }
}
