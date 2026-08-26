//! Productive embedded-game adapter for the sealed 27-case qualification corpus.
//!
//! Every cache and diagnostic originates in the pinned game process. Invoke return values are
//! observed by the pinned native sidecar only after the game has independently produced its raw
//! cache; promotion still compares the complete game-cache semantics against the standalone run.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest as _, Sha256};

use super::capture::{
    decode_capture_v1, reload_unqualified_profile_package_v1, PROFILE_MANIFEST_FILE_V1,
};
use super::frontend::{ExternalFrontendHooksV1, PreprocessorConfigV1};
use super::manifest::{CompilerProfileV1, Sha256Digest};
use super::qualification::{
    CompilerProbeCaseV1, CompilerProbeCorpusV1, ExpectedDiagnosticV1, ProbeDiagnosticSeverityV1,
    ProbeModeV1, ProbeOutcomeV1, QualifiedSidecarIdentityV1,
};
use super::qualification_runner::CompilerProbeBackendErrorV1;
use super::qualification_suite::{
    full_qualification_corpus_v1, validate_frontend_coverage_witness,
    validate_full_suite_accepted_cache_boundary_v1, OfflineCapturedProbeOutputV1,
    OfflineCapturedSupplementalWitnessV1, OfflineCompilerBuildFlagsWitnessV1,
    OfflineFrontendCoverageWitnessV1, OfflineFrontendHookCaptureV1,
    OfflineQualificationCaptureBackendV1, FULL_QUALIFICATION_SUITE_ID_V1,
};
use crate::compile::{
    acquire_compile_install_mutation, run_embedded_frontend_qualification_compile_v1,
    run_embedded_qualification_compile_v1, EmbeddedQualificationCompileReportV1,
    InstallRestoreDisposition, ProjectCompilerClosingAuditDisposition,
    ProjectCompilerOutputDisposition,
};
use crate::compiler_backend::CompilerBackendFailureV1;
use crate::compiler_target::{CompilerTargetInputPathsV1, ValidatedCompilerTargetInputsV1};
use crate::diagnostics::{DiagnosticSeverity, DiagnosticsCaptureDisposition, DiagnosticsOptions};
use crate::standalone_sidecar::{
    load_qualification_target_inputs_v3, run_qualification_sidecar_v3,
    validate_qualification_sidecar_config_v3, QualificationPhaseV3, QualificationSidecarRunV3,
    StandaloneSidecarConfigV1,
};

static CASE_SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct EmbeddedQualificationHarnessConfigV1 {
    pub profile_root: PathBuf,
    pub game_dir: PathBuf,
    pub executable_path: PathBuf,
    pub shipping_cache_path: PathBuf,
    pub binds_cache_path: PathBuf,
    pub authority_capture_path: PathBuf,
    pub capture_controller_path: PathBuf,
    pub capture_bridge_path: PathBuf,
    pub diagnostics: DiagnosticsOptions,
    pub scratch_root: PathBuf,
    pub invoke_observer: StandaloneSidecarConfigV1,
    pub invoke_observer_authority: QualifiedSidecarIdentityV1,
}

pub struct EmbeddedQualificationHarnessV1 {
    config: EmbeddedQualificationHarnessConfigV1,
    profile: CompilerProfileV1,
    corpus: CompilerProbeCorpusV1,
    base_cache: Vec<u8>,
    binds_cache: Vec<u8>,
    authority_preprocessor: PreprocessorConfigV1,
    authority_build_jit: super::capture::BuildJitCaptureV1,
}

struct CaseScratch {
    root: PathBuf,
}

impl Drop for CaseScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct EmbeddedRunOutput {
    result: Result<Vec<u8>, String>,
    diagnostics: Vec<ExpectedDiagnosticV1>,
    frontend: Option<OfflineFrontendCoverageWitnessV1>,
}

impl EmbeddedQualificationHarnessV1 {
    pub fn new(
        config: EmbeddedQualificationHarnessConfigV1,
    ) -> Result<Self, CompilerBackendFailureV1> {
        validate_qualification_sidecar_config_v3(
            &config.invoke_observer,
            config.invoke_observer_authority,
        )?;
        if config.invoke_observer.profile_root != config.profile_root
            || config.invoke_observer.profile_manifest_path
                != config.profile_root.join(PROFILE_MANIFEST_FILE_V1)
        {
            return Err(unavailable(
                "embedded invoke observer does not use the exact unqualified profile package",
            ));
        }
        for (label, path) in [
            ("profile root", &config.profile_root),
            ("game directory", &config.game_dir),
            ("target executable", &config.executable_path),
            ("Shipping cache", &config.shipping_cache_path),
            ("Binds cache", &config.binds_cache_path),
            ("authority capture", &config.authority_capture_path),
            ("capture controller", &config.capture_controller_path),
            ("capture bridge", &config.capture_bridge_path),
            ("qualification scratch", &config.scratch_root),
        ] {
            require_absolute_normalized(path, label)?;
        }
        if !config.scratch_root.is_dir() {
            return Err(unavailable(
                "embedded qualification scratch root is not an existing directory",
            ));
        }
        let profile =
            reload_unqualified_profile_package_v1(&config.profile_root).map_err(|error| {
                unavailable(format!(
                    "embedded qualification profile failed typed reload: {error}"
                ))
            })?;
        let corpus = full_qualification_corpus_v1().map_err(|error| {
            unavailable(format!(
                "canonical qualification corpus is invalid: {error}"
            ))
        })?;
        if profile.qualification.required_probe_suite_version != FULL_QUALIFICATION_SUITE_ID_V1 {
            return Err(unavailable(
                "embedded qualification profile does not select the complete suite",
            ));
        }
        bind_profile_corpus(&profile, &corpus)?;
        let (base_cache, binds_cache) = load_qualification_target_inputs_v3(
            &profile,
            &config.shipping_cache_path,
            &config.binds_cache_path,
        )?;
        let authority_capture = fs::read(&config.authority_capture_path)
            .map_err(|error| unavailable(format!("reading authority capture: {error}")))?;
        let decoded = decode_capture_v1(&authority_capture)
            .map_err(|error| unavailable(format!("decoding authority capture: {error}")))?;
        verify_frontend_profile_projection(&profile, &config.profile_root, &decoded)?;
        if decoded.build_jit.as_reference_debugging
            || decoded.build_jit.resolve_object_ptr_callback_registered
        {
            return Err(unavailable(
                "authority capture enables a qualification-forbidden compiler build flag",
            ));
        }
        Ok(Self {
            config,
            profile,
            corpus,
            base_cache,
            binds_cache,
            authority_preprocessor: decoded.frontend_configs.preprocessor,
            authority_build_jit: decoded.build_jit,
        })
    }

    pub fn profile(&self) -> &CompilerProfileV1 {
        &self.profile
    }

    fn execute_sections(
        &self,
        case: &CompilerProbeCaseV1,
        phase: QualificationPhaseV3,
        sections: &[super::qualification::ProbeSourceSectionV1],
        capture_frontend: bool,
    ) -> Result<EmbeddedRunOutput, CompilerProbeBackendErrorV1> {
        let scratch = self.stage_case(case, phase, sections)?;
        let capture_path = scratch.root.join("same-run-frontend.capture");
        let guard = acquire_compile_install_mutation(&self.config.game_dir)
            .map_err(|error| backend_error(format!("embedded qualification preflight: {error}")))?;
        let target = self
            .load_target()
            .map_err(|error| backend_error(format!("pinning embedded target: {error}")))?;
        let closing_profile = self.profile.clone();
        let executable = self.config.executable_path.clone();
        let shipping = self.config.shipping_cache_path.clone();
        let binds = self.config.binds_cache_path.clone();
        let closing_audit = move || {
            ValidatedCompilerTargetInputsV1::load_unqualified_profile_for_qualification(
                &closing_profile,
                CompilerTargetInputPathsV1 {
                    executable: &executable,
                    shipping_cache: &shipping,
                    binds_cache: &binds,
                },
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
        };
        let report = if capture_frontend {
            run_embedded_frontend_qualification_compile_v1(
                &self.config.game_dir,
                &scratch.root,
                &self.config.diagnostics,
                &self.config.capture_controller_path,
                &self.config.capture_bridge_path,
                &capture_path,
                guard,
                target,
                closing_audit,
            )
        } else {
            run_embedded_qualification_compile_v1(
                &self.config.game_dir,
                &scratch.root,
                &self.config.diagnostics,
                guard,
                target,
                closing_audit,
            )
        };
        let diagnostics = validate_and_convert_report(&report, sections, &self.config.game_dir)?;
        let frontend = if capture_frontend {
            Some(self.decode_frontend_witness(case, &capture_path)?)
        } else {
            None
        };
        Ok(EmbeddedRunOutput {
            result: report.result,
            diagnostics,
            frontend,
        })
    }

    fn load_target(
        &self,
    ) -> Result<ValidatedCompilerTargetInputsV1, crate::compiler_target::CompilerTargetInputError>
    {
        ValidatedCompilerTargetInputsV1::load_unqualified_profile_for_qualification(
            &self.profile,
            CompilerTargetInputPathsV1 {
                executable: &self.config.executable_path,
                shipping_cache: &self.config.shipping_cache_path,
                binds_cache: &self.config.binds_cache_path,
            },
        )
    }

    fn stage_case(
        &self,
        case: &CompilerProbeCaseV1,
        phase: QualificationPhaseV3,
        sections: &[super::qualification::ProbeSourceSectionV1],
    ) -> Result<CaseScratch, CompilerProbeBackendErrorV1> {
        let phase = match phase {
            QualificationPhaseV3::Single => "single",
            QualificationPhaseV3::GraphBaseline => "baseline",
            QualificationPhaseV3::GraphFinal => "final",
        };
        let sequence = CASE_SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = self.config.scratch_root.join(format!(
            "embedded-case-{:02}-{phase}-{}-{sequence}",
            case.ordinal,
            std::process::id()
        ));
        fs::create_dir(&root)
            .map_err(|error| backend_error(format!("creating embedded case scratch: {error}")))?;
        let scratch = CaseScratch { root };
        for section in sections {
            let relative = safe_relative_source_path(&section.relative_path)?;
            let destination = scratch.root.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    backend_error(format!("creating embedded source directory: {error}"))
                })?;
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .map_err(|error| {
                    backend_error(format!("creating embedded source file: {error}"))
                })?;
            output
                .write_all(section.source_utf8.as_bytes())
                .and_then(|_| output.sync_all())
                .map_err(|error| backend_error(format!("writing embedded source: {error}")))?;
        }
        Ok(scratch)
    }

    fn observe_invoke(
        &self,
        case: &CompilerProbeCaseV1,
        module: &str,
        declaration: &str,
    ) -> Result<crate::cache::semantic_observer::CanonicalInvokeReturnV1, CompilerProbeBackendErrorV1>
    {
        let run = run_qualification_sidecar_v3(
            &self.config.invoke_observer,
            &self.profile,
            &self.base_cache,
            &self.binds_cache,
            &self.corpus.suite_id,
            self.corpus.canonical_sha256,
            &case.case_id,
            QualificationPhaseV3::Single,
            &case.sections,
            Some((module, declaration)),
        )
        .map_err(|error| backend_error(format!("embedded invoke observer: {error}")))?;
        match run {
            QualificationSidecarRunV3::Accepted {
                invoke_return: Some(value),
                ..
            } => Ok(value.into_observer_value()),
            _ => Err(backend_error(
                "embedded invoke observer did not return the sealed invoke value".to_owned(),
            )),
        }
    }

    fn decode_frontend_witness(
        &self,
        case: &CompilerProbeCaseV1,
        capture_path: &Path,
    ) -> Result<OfflineFrontendCoverageWitnessV1, CompilerProbeBackendErrorV1> {
        let bytes = fs::read(capture_path).map_err(|error| {
            backend_error(format!("reading same-run frontend capture: {error}"))
        })?;
        let decoded = decode_capture_v1(&bytes).map_err(|error| {
            backend_error(format!("decoding same-run frontend capture: {error}"))
        })?;
        if decoded.build_jit != self.authority_build_jit
            || decoded.frontend_configs.class_generator != self.authority_class_generator()?
            || decoded.frontend_configs.compiler_options != self.authority_compiler_options()?
        {
            return Err(backend_error(
                "same-run frontend capture drifted from the authority build/config".to_owned(),
            ));
        }
        let mut normalized = decoded.frontend_configs.preprocessor.clone();
        let hooks = normalized.external_hooks.clone();
        if !fname_comparison_semantics_match(
            &self.authority_preprocessor.fname_comparison_keys,
            &normalized.fname_comparison_keys,
        ) {
            return Err(backend_error(
                "same-run frontend capture changed FName spellings or comparison equivalence"
                    .to_owned(),
            ));
        }
        // FName comparison indices are opaque identities allocated by the current process. Their
        // numeric values move when unrelated source names are interned before these literals; only
        // the spelling set and pairwise equality partition are target semantics. The checks above
        // prove both before replacing the run-local tokens with the authority tokens so the rest of
        // the preprocessor configuration can remain an exact typed comparison.
        normalized.fname_comparison_keys =
            self.authority_preprocessor.fname_comparison_keys.clone();
        normalized.external_hooks = self.authority_preprocessor.external_hooks.clone();
        normalized
            .seal()
            .map_err(|error| backend_error(format!("normalizing frontend capture: {error}")))?;
        if normalized != self.authority_preprocessor {
            return Err(backend_error(format!(
                "same-run frontend capture changed non-hook preprocessor authority at {}",
                first_preprocessor_difference(&self.authority_preprocessor, &normalized)
            )));
        }
        frontend_witness(case, &hooks)
    }

    fn authority_class_generator(
        &self,
    ) -> Result<super::frontend::ClassGeneratorConfigV1, CompilerProbeBackendErrorV1> {
        let path = self
            .config
            .profile_root
            .join(&self.profile.frontend.class_generator_config.path);
        super::frontend::ClassGeneratorConfigV1::from_json(
            &fs::read(path).map_err(|error| backend_error(error.to_string()))?,
        )
        .map_err(|error| backend_error(error.to_string()))
    }

    fn authority_compiler_options(
        &self,
    ) -> Result<super::frontend::CompilerOptionsV1, CompilerProbeBackendErrorV1> {
        let path = self
            .config
            .profile_root
            .join(&self.profile.frontend.compiler_options.path);
        super::frontend::CompilerOptionsV1::from_json(
            &fs::read(path).map_err(|error| backend_error(error.to_string()))?,
        )
        .map_err(|error| backend_error(error.to_string()))
    }
}

fn fname_comparison_semantics_match(
    expected: &[super::frontend::FNameComparisonKeyV1],
    actual: &[super::frontend::FNameComparisonKeyV1],
) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(left, right)| left.ordinal == right.ordinal && left.spelling == right.spelling)
        && expected.iter().enumerate().all(|(left_index, left)| {
            expected
                .iter()
                .zip(actual)
                .all(|(right_expected, right_actual)| {
                    (left.comparison_key == right_expected.comparison_key)
                        == (actual[left_index].comparison_key == right_actual.comparison_key)
                })
        })
}

impl OfflineQualificationCaptureBackendV1 for EmbeddedQualificationHarnessV1 {
    fn source_profile_sha256(&self) -> Sha256Digest {
        self.profile.profile_sha256
    }

    fn source_target(&self) -> super::manifest::CompilerTargetV1 {
        self.profile.target.clone()
    }

    fn standalone_compiler_identity(&self) -> Option<QualifiedSidecarIdentityV1> {
        None
    }

    fn capture_probe(
        &mut self,
        case: &CompilerProbeCaseV1,
    ) -> Result<OfflineCapturedProbeOutputV1, CompilerProbeBackendErrorV1> {
        let authoritative = self
            .corpus
            .cases
            .get(case.ordinal as usize)
            .filter(|expected| *expected == case)
            .ok_or_else(|| {
                backend_error("caller case is not the sealed embedded corpus row".into())
            })?;
        eprintln!(
            "embedded qualification [{}/{}] {}",
            authoritative.ordinal + 1,
            self.corpus.cases.len(),
            authoritative.case_id
        );
        match &authoritative.mode {
            ProbeModeV1::CompileGraphTransition {
                baseline_sections, ..
            } => {
                let baseline = self.execute_sections(
                    authoritative,
                    QualificationPhaseV3::GraphBaseline,
                    baseline_sections,
                    false,
                )?;
                let final_run = self.execute_sections(
                    authoritative,
                    QualificationPhaseV3::GraphFinal,
                    &authoritative.sections,
                    false,
                )?;
                match (baseline.result, final_run.result) {
                    (Ok(baseline_cache), Ok(final_cache))
                        if baseline.diagnostics.is_empty() =>
                    {
                        Ok(OfflineCapturedProbeOutputV1::accepted_graph_transition(
                            final_run.diagnostics,
                            baseline_cache,
                            final_cache,
                        ))
                    }
                    _ => Err(backend_error(
                        "embedded graph qualification did not produce accepted baseline/final caches"
                            .into(),
                    )),
                }
            }
            mode => {
                let capture_frontend =
                    authoritative.case_id == "positive.frontend.hooks-editor-release";
                let run = self.execute_sections(
                    authoritative,
                    QualificationPhaseV3::Single,
                    &authoritative.sections,
                    capture_frontend,
                )?;
                match (authoritative.expected_outcome, run.result) {
                    (ProbeOutcomeV1::Accepted, Ok(cache_bytes)) => {
                        validate_full_suite_accepted_cache_boundary_v1(authoritative, &cache_bytes)
                            .map_err(|error| {
                                backend_error(format!(
                                    "embedded accepted-cache witness is incomplete: {error}"
                                ))
                            })?;
                        let invoke_return = match mode {
                            ProbeModeV1::CompileOnly => None,
                            ProbeModeV1::Invoke { declaration } => {
                                let module = authoritative
                                    .sections
                                    .first()
                                    .filter(|_| authoritative.sections.len() == 1)
                                    .ok_or_else(|| {
                                        backend_error(
                                            "embedded invoke requires one sealed source module"
                                                .into(),
                                        )
                                    })?;
                                Some(self.observe_invoke(
                                    authoritative,
                                    &module.module,
                                    declaration,
                                )?)
                            }
                            ProbeModeV1::CompileGraphTransition { .. } => unreachable!(),
                        };
                        let supplemental = match authoritative.case_id.as_str() {
                            "positive.frontend.hooks-editor-release" => {
                                OfflineCapturedSupplementalWitnessV1::Frontend(
                                    run.frontend.ok_or_else(|| {
                                        backend_error(
                                            "embedded frontend run omitted its same-run witness"
                                                .into(),
                                        )
                                    })?,
                                )
                            }
                            "positive.bytecode.fork-reference-lifecycle"
                            | "positive.bytecode.unresolved-object-property" => {
                                OfflineCapturedSupplementalWitnessV1::CompilerBuildFlags(
                                    OfflineCompilerBuildFlagsWitnessV1 {
                                        as_reference_debugging: self
                                            .authority_build_jit
                                            .as_reference_debugging,
                                        resolve_object_ptr_callback_registered: self
                                            .authority_build_jit
                                            .resolve_object_ptr_callback_registered,
                                    },
                                )
                            }
                            _ => OfflineCapturedSupplementalWitnessV1::None,
                        };
                        Ok(OfflineCapturedProbeOutputV1::accepted_with_supplemental(
                            run.diagnostics,
                            cache_bytes,
                            invoke_return,
                            supplemental,
                        ))
                    }
                    (ProbeOutcomeV1::Rejected, Err(_))
                        if run.diagnostics.iter().any(|diagnostic| {
                            diagnostic.severity == ProbeDiagnosticSeverityV1::Error
                        }) =>
                    {
                        Ok(OfflineCapturedProbeOutputV1::rejected(run.diagnostics))
                    }
                    (ProbeOutcomeV1::Accepted, Err(error)) => Err(backend_error(format!(
                        "embedded game rejected an accepted corpus case: {error}"
                    ))),
                    (ProbeOutcomeV1::Rejected, Ok(_)) => Err(backend_error(
                        "embedded game accepted a rejected corpus case".into(),
                    )),
                    (ProbeOutcomeV1::Rejected, Err(error)) => Err(backend_error(format!(
                        "embedded rejection lacked a captured error diagnostic: {error}"
                    ))),
                }
            }
        }
    }
}

fn validate_and_convert_report(
    report: &EmbeddedQualificationCompileReportV1,
    sections: &[super::qualification::ProbeSourceSectionV1],
    game_dir: &Path,
) -> Result<Vec<ExpectedDiagnosticV1>, CompilerProbeBackendErrorV1> {
    let output_closed = embedded_output_is_closed(&report.result, report.output_disposition);
    if report.install_restore != InstallRestoreDisposition::RestoredExact
        || !output_closed
        || report.closing_audit != ProjectCompilerClosingAuditDisposition::Passed
    {
        let primary = report
            .result
            .as_ref()
            .err()
            .map(String::as_str)
            .unwrap_or("no primary compiler error");
        return Err(backend_error(format!(
            "embedded compile did not close exactly: restore={:?}, output={:?}, audit={:?}; {primary}",
            report.install_restore, report.output_disposition, report.closing_audit,
        )));
    }
    let diagnostics = report.diagnostics.as_ref().ok_or_else(|| {
        backend_error("embedded compile produced no diagnostics disposition".into())
    })?;
    if diagnostics.disposition() != DiagnosticsCaptureDisposition::Captured {
        return Err(backend_error(format!(
            "embedded qualification requires captured diagnostics, got {:?}",
            diagnostics.disposition()
        )));
    }
    Ok(diagnostics
        .diagnostics()
        .iter()
        .enumerate()
        .map(|(ordinal, diagnostic)| ExpectedDiagnosticV1 {
            ordinal: ordinal as u32,
            severity: match diagnostic.severity {
                DiagnosticSeverity::Error => ProbeDiagnosticSeverityV1::Error,
                DiagnosticSeverity::Warning => ProbeDiagnosticSeverityV1::Warning,
                DiagnosticSeverity::Note => ProbeDiagnosticSeverityV1::Info,
            },
            section: canonical_diagnostic_section(&diagnostic.file, sections, game_dir),
            row: (diagnostic.line != 0 && diagnostic.column != 0).then_some(diagnostic.line),
            column: (diagnostic.line != 0 && diagnostic.column != 0).then_some(diagnostic.column),
            message: diagnostic.message.clone(),
        })
        .collect())
}

fn embedded_output_is_closed(
    result: &Result<Vec<u8>, String>,
    output: ProjectCompilerOutputDisposition,
) -> bool {
    match result {
        Ok(_) => output == ProjectCompilerOutputDisposition::Discarded,
        Err(_) => matches!(
            output,
            ProjectCompilerOutputDisposition::NotCreated
                | ProjectCompilerOutputDisposition::Discarded
        ),
    }
}

fn canonical_diagnostic_section(
    value: &str,
    sections: &[super::qualification::ProbeSourceSectionV1],
    game_dir: &Path,
) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let normalized = value.replace('\\', "/");
    let matches = sections
        .iter()
        .filter(|section| {
            normalized.eq_ignore_ascii_case(&section.relative_path)
                || normalized
                    .to_ascii_lowercase()
                    .ends_with(&format!("/{}", section.relative_path.to_ascii_lowercase()))
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Some(matches[0].relative_path.clone())
    } else if normalized.to_ascii_lowercase().contains(
        &game_dir
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase(),
    ) {
        Some("<private embedded compiler path>".to_owned())
    } else {
        Some(normalized)
    }
}

fn frontend_witness(
    case: &CompilerProbeCaseV1,
    hooks: &ExternalFrontendHooksV1,
) -> Result<OfflineFrontendCoverageWitnessV1, CompilerProbeBackendErrorV1> {
    let case_modules: BTreeSet<_> = case
        .sections
        .iter()
        .map(|section| section.module.as_str())
        .collect();
    let class_analyze_captures = hooks
        .class_analyze
        .captures
        .iter()
        .filter(|capture| case_modules.contains(capture.module_name.as_str()))
        .filter(|capture| !capture.generated_statics.is_empty())
        .map(|capture| {
            let subject_identity = if capture.namespace.is_empty() {
                format!("{}::{}", capture.module_name, capture.class_name)
            } else {
                format!(
                    "{}::{}::{}",
                    capture.module_name, capture.namespace, capture.class_name
                )
            };
            OfflineFrontendHookCaptureV1 {
                subject_identity,
                generated_declarations: vec![capture.generated_statics.clone()],
            }
        })
        .collect::<Vec<_>>();
    let graph_captures = |profile: &super::frontend::GraphHookProfileV1| {
        profile
            .captures
            .iter()
            .flat_map(|capture| &capture.modules)
            .filter(|module| {
                case_modules.contains(module.module_name.as_str())
                    && !module.generated_declarations.is_empty()
            })
            .map(|module| OfflineFrontendHookCaptureV1 {
                subject_identity: module.module_name.clone(),
                generated_declarations: vec![module.generated_declarations.clone()],
            })
            .collect::<Vec<_>>()
    };
    let process_chunks_captures = graph_captures(&hooks.process_chunks);
    let post_process_captures = graph_captures(&hooks.post_process_code);
    let mut generated_declarations = class_analyze_captures
        .iter()
        .chain(&process_chunks_captures)
        .chain(&post_process_captures)
        .flat_map(|capture| capture.generated_declarations.iter().cloned())
        .collect::<Vec<_>>();
    generated_declarations.sort();
    generated_declarations.dedup();
    let mut editor_discovery = case
        .sections
        .iter()
        .filter(|section| section.source_utf8.contains("#if EDITOR"))
        .map(|section| section.module.clone())
        .collect::<Vec<_>>();
    let mut release_discovery = case
        .sections
        .iter()
        .filter(|section| {
            section.source_utf8.contains("#if RELEASE")
                && section
                    .source_utf8
                    .contains("QualificationReleaseDiscovery")
        })
        .map(|section| section.module.clone())
        .collect::<Vec<_>>();
    editor_discovery.sort();
    editor_discovery.dedup();
    release_discovery.sort();
    release_discovery.dedup();
    let witness = OfflineFrontendCoverageWitnessV1 {
        class_analyze_bound: hooks.class_analyze.bound,
        class_analyze_captures,
        process_chunks_bound: hooks.process_chunks.bound,
        process_chunks_captures,
        post_process_code_bound: hooks.post_process_code.bound,
        post_process_captures,
        generated_declarations,
        editor_discovery,
        release_discovery,
    };
    validate_frontend_coverage_witness(&witness)
        .map_err(|error| backend_error(format!("embedded frontend witness: {error}")))?;
    Ok(witness)
}

fn verify_frontend_profile_projection(
    profile: &CompilerProfileV1,
    profile_root: &Path,
    decoded: &super::capture::DecodedCaptureV1,
) -> Result<(), CompilerBackendFailureV1> {
    for (label, path, bytes) in [
        (
            "preprocessor",
            &profile.frontend.preprocessor_config.path,
            decoded
                .frontend_configs
                .preprocessor
                .to_json()
                .map_err(|error| {
                    unavailable(format!(
                        "serializing captured preprocessor profile: {error}"
                    ))
                })?,
        ),
        (
            "class generator",
            &profile.frontend.class_generator_config.path,
            decoded
                .frontend_configs
                .class_generator
                .to_json()
                .map_err(|error| {
                    unavailable(format!(
                        "serializing captured class-generator profile: {error}"
                    ))
                })?,
        ),
        (
            "compiler options",
            &profile.frontend.compiler_options.path,
            decoded
                .frontend_configs
                .compiler_options
                .to_json()
                .map_err(|error| {
                    unavailable(format!(
                        "serializing captured compiler-options profile: {error}"
                    ))
                })?,
        ),
    ] {
        let packaged = fs::read(profile_root.join(path))
            .map_err(|error| unavailable(format!("reading packaged {label}: {error}")))?;
        if packaged != bytes {
            return Err(unavailable(format!(
                "authority capture does not exactly project to packaged {label}"
            )));
        }
    }
    Ok(())
}

fn bind_profile_corpus(
    profile: &CompilerProfileV1,
    corpus: &CompilerProbeCorpusV1,
) -> Result<(), CompilerBackendFailureV1> {
    let json = corpus
        .to_json()
        .map_err(|error| unavailable(format!("serializing canonical corpus: {error}")))?;
    let seal = &profile.bytecode.codegen_probe_corpus;
    let digest = Sha256Digest::from_bytes(Sha256::digest(&json).into());
    if seal.byte_len != json.len() as u64 || seal.sha256 != digest {
        return Err(unavailable(
            "unqualified profile does not seal the executable canonical corpus",
        ));
    }
    Ok(())
}

fn safe_relative_source_path(value: &str) -> Result<PathBuf, CompilerProbeBackendErrorV1> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !value.to_ascii_lowercase().ends_with(".as")
    {
        return Err(backend_error(
            "sealed embedded source path is unsafe".to_owned(),
        ));
    }
    Ok(path.to_path_buf())
}

fn require_absolute_normalized(
    path: &Path,
    label: &'static str,
) -> Result<(), CompilerBackendFailureV1> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(unavailable(format!(
            "{label} path must be absolute and normalized"
        )));
    }
    Ok(())
}

fn unavailable(detail: impl Into<String>) -> CompilerBackendFailureV1 {
    CompilerBackendFailureV1::unavailable(detail)
}

fn backend_error(detail: String) -> CompilerProbeBackendErrorV1 {
    CompilerProbeBackendErrorV1::new(detail).unwrap_or_else(|_| {
        CompilerProbeBackendErrorV1::static_internal("embedded qualification failure was invalid")
    })
}

fn first_preprocessor_difference(
    expected: &super::frontend::PreprocessorConfigV1,
    actual: &super::frontend::PreprocessorConfigV1,
) -> &'static str {
    macro_rules! check {
        ($field:ident) => {
            if expected.$field != actual.$field {
                return stringify!($field);
            }
        };
    }
    check!(schema);
    check!(schema_version);
    check!(automatic_imports);
    check!(warn_on_manual_import_statements);
    check!(use_editor_scripts);
    check!(effective_flags);
    check!(default_function_blueprint_callable);
    check!(default_property_edit_specifier);
    check!(default_property_edit_specifier_for_structs);
    check!(default_property_blueprint_specifier);
    check!(static_class_mode);
    check!(script_float_is_float64);
    check!(angelscript_haze);
    check!(enforce_server_rpc_validation);
    check!(blueprint_event_argument_specializations);
    check!(native_super_types);
    check!(fname_comparison_keys);
    check!(external_hooks);
    check!(canonical_sha256);
    "unknown field"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_profile::frontend::{
        ClassAnalyzeCaptureV1, ClassAnalyzeHookProfileV1, FNameComparisonKeyV1, GraphHookProfileV1,
    };

    fn fname_key(ordinal: u32, spelling: &str, comparison_key: &str) -> FNameComparisonKeyV1 {
        FNameComparisonKeyV1 {
            ordinal,
            spelling: spelling.to_owned(),
            comparison_key: comparison_key.to_owned(),
        }
    }

    #[test]
    fn fname_capture_parity_compares_spellings_and_equivalence_not_run_local_tokens() {
        let authority = vec![
            fname_key(0, "Äquivalent", "authority-a"),
            fname_key(1, "ÄQUIVALENT", "authority-a"),
            fname_key(2, "äQUIVALENT", "authority-b"),
        ];
        let relocated = vec![
            fname_key(0, "Äquivalent", "run-17"),
            fname_key(1, "ÄQUIVALENT", "run-17"),
            fname_key(2, "äQUIVALENT", "run-42"),
        ];
        assert!(fname_comparison_semantics_match(&authority, &relocated));

        let mut merged = relocated.clone();
        merged[2].comparison_key = "run-17".to_owned();
        assert!(!fname_comparison_semantics_match(&authority, &merged));

        let mut changed_spelling = relocated;
        changed_spelling[1].spelling = "Other".to_owned();
        assert!(!fname_comparison_semantics_match(
            &authority,
            &changed_spelling
        ));
    }

    #[test]
    fn canonical_frontend_case_projects_exact_same_run_witness_shape() {
        let case = full_qualification_corpus_v1()
            .unwrap()
            .cases
            .into_iter()
            .find(|case| case.case_id == "positive.frontend.hooks-editor-release")
            .unwrap();
        let generated = "namespace UQualificationFrontendHook { int32 Marker(); }";
        let zero = Sha256Digest::from_bytes([0; 32]);
        let hooks = ExternalFrontendHooksV1 {
            class_analyze: ClassAnalyzeHookProfileV1 {
                bound: true,
                captures: vec![ClassAnalyzeCaptureV1 {
                    ordinal: 0,
                    module_name: "FrontendHooks".into(),
                    namespace: String::new(),
                    class_name: "UQualificationFrontendHook".into(),
                    source_sha256: zero,
                    input_generated_statics_sha256: zero,
                    generated_statics: generated.into(),
                    output_generated_statics_sha256: zero,
                    has_statics: false,
                    compose_onto_class: String::new(),
                }],
            },
            process_chunks: GraphHookProfileV1 {
                bound: false,
                captures: Vec::new(),
            },
            post_process_code: GraphHookProfileV1 {
                bound: false,
                captures: Vec::new(),
            },
        };
        let witness = frontend_witness(&case, &hooks).unwrap();
        assert_eq!(witness.class_analyze_captures.len(), 1);
        assert_eq!(
            witness.class_analyze_captures[0].subject_identity,
            "FrontendHooks::UQualificationFrontendHook"
        );
        assert_eq!(witness.generated_declarations, [generated]);
        assert_eq!(witness.editor_discovery, ["FrontendHooks"]);
        assert_eq!(witness.release_discovery, ["FrontendHooks"]);
        assert!(!witness.process_chunks_bound);
        assert!(!witness.post_process_code_bound);
    }

    #[test]
    fn embedded_source_paths_are_closed_relative_as_paths() {
        assert_eq!(
            safe_relative_source_path("Graph/Provider.as").unwrap(),
            PathBuf::from("Graph/Provider.as")
        );
        for invalid in [
            "",
            "../escape.as",
            "/absolute.as",
            "Graph\\Bad.as",
            "file.txt",
        ] {
            assert!(safe_relative_source_path(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn rejected_embedded_compile_closes_without_inventing_an_output() {
        let rejected = Err("compiler rejected source".to_owned());
        assert!(embedded_output_is_closed(
            &rejected,
            ProjectCompilerOutputDisposition::NotCreated
        ));
        assert!(embedded_output_is_closed(
            &rejected,
            ProjectCompilerOutputDisposition::Discarded
        ));
        assert!(!embedded_output_is_closed(
            &rejected,
            ProjectCompilerOutputDisposition::RecoveryRetained
        ));
        assert!(!embedded_output_is_closed(
            &Ok(vec![1]),
            ProjectCompilerOutputDisposition::NotCreated
        ));
    }
}
