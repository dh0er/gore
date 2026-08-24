//! Productive offline adapter for the sealed 26-case standalone qualification corpus.
//!
//! The request contains only corpus-sealed source, target/profile identities, and an optional
//! invoke declaration. Cache bytes, diagnostics, build flags, and frontend traces are emitted by
//! the exact pinned sidecar process. Callers cannot inject supplemental witness JSON.

use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use super::capture::{reload_unqualified_profile_package_v1, PROFILE_MANIFEST_FILE_V1};
use super::manifest::{CompilerProfileV1, Sha256Digest};
use super::qualification::{
    CompilerProbeCaseV1, CompilerProbeCorpusV1, ProbeModeV1, ProbeOutcomeV1,
    QualifiedSidecarIdentityV1,
};
use super::qualification_runner::CompilerProbeBackendErrorV1;
use super::qualification_suite::{
    full_qualification_corpus_v1, validate_frontend_coverage_witness,
    validate_full_suite_accepted_cache_boundary_v1, OfflineCapturedProbeOutputV1,
    OfflineCapturedSupplementalWitnessV1, OfflineQualificationCaptureBackendV1,
    FULL_QUALIFICATION_SUITE_ID_V1,
};
use crate::compiler_backend::CompilerBackendFailureV1;
use crate::standalone_sidecar::{
    load_qualification_target_inputs_v3, run_qualification_sidecar_v3,
    validate_qualification_sidecar_config_v3, QualificationPhaseV3, QualificationSidecarRunV3,
    StandaloneSidecarConfigV1,
};

/// Exact external authority needed before a profile is qualified and therefore cannot yet
/// authenticate its own sidecar. Product release tooling obtains this identity from the signed
/// sidecar release/catalog authority; the adapter verifies the open executable against it.
#[derive(Debug, Clone)]
pub struct StandaloneQualificationHarnessConfigV1 {
    pub sidecar: StandaloneSidecarConfigV1,
    pub sidecar_authority: QualifiedSidecarIdentityV1,
    pub base_cache_path: PathBuf,
    pub binds_cache_path: PathBuf,
}

/// Offline backend which owns the canonical corpus and accepts no caller-selected cases or
/// witnesses. It never launches the game or mutates an install.
pub struct StandaloneQualificationHarnessV1 {
    config: StandaloneSidecarConfigV1,
    sidecar_authority: QualifiedSidecarIdentityV1,
    profile: CompilerProfileV1,
    corpus: CompilerProbeCorpusV1,
    base_cache: Vec<u8>,
    binds_cache: Vec<u8>,
}

impl StandaloneQualificationHarnessV1 {
    pub fn new(
        config: StandaloneQualificationHarnessConfigV1,
    ) -> Result<Self, CompilerBackendFailureV1> {
        validate_qualification_sidecar_config_v3(&config.sidecar, config.sidecar_authority)?;
        let expected_manifest = config.sidecar.profile_root.join(PROFILE_MANIFEST_FILE_V1);
        if config.sidecar.profile_manifest_path != expected_manifest {
            return Err(CompilerBackendFailureV1::unavailable(
                "qualification requires the typed materializer package manifest at its fixed name",
            ));
        }
        let profile = reload_unqualified_profile_package_v1(&config.sidecar.profile_root).map_err(
            |error| {
                CompilerBackendFailureV1::unavailable(format!(
                    "unqualified compiler-profile package failed typed reload: {error}"
                ))
            },
        )?;
        let corpus = full_qualification_corpus_v1().map_err(|error| {
            CompilerBackendFailureV1::unavailable(format!(
                "canonical qualification corpus is invalid: {error}"
            ))
        })?;
        if profile.qualification.required_probe_suite_version != FULL_QUALIFICATION_SUITE_ID_V1 {
            return Err(CompilerBackendFailureV1::unavailable(
                "unqualified profile does not target the complete qualification suite",
            ));
        }
        bind_profile_corpus(&profile, &corpus)?;
        let (base_cache, binds_cache) = load_qualification_target_inputs_v3(
            &profile,
            &config.base_cache_path,
            &config.binds_cache_path,
        )?;
        Ok(Self {
            config: config.sidecar,
            sidecar_authority: config.sidecar_authority,
            profile,
            corpus,
            base_cache,
            binds_cache,
        })
    }

    pub fn profile(&self) -> &CompilerProfileV1 {
        &self.profile
    }

    pub fn corpus(&self) -> &CompilerProbeCorpusV1 {
        &self.corpus
    }

    pub fn sidecar_authority(&self) -> QualifiedSidecarIdentityV1 {
        self.sidecar_authority
    }

    fn run(
        &self,
        case: &CompilerProbeCaseV1,
        phase: QualificationPhaseV3,
        sections: &[super::qualification::ProbeSourceSectionV1],
        invoke: Option<(&str, &str)>,
    ) -> Result<QualificationSidecarRunV3, CompilerProbeBackendErrorV1> {
        run_qualification_sidecar_v3(
            &self.config,
            &self.profile,
            &self.base_cache,
            &self.binds_cache,
            &self.corpus.suite_id,
            self.corpus.canonical_sha256,
            &case.case_id,
            phase,
            sections,
            invoke,
        )
        .map_err(backend_error)
    }
}

impl OfflineQualificationCaptureBackendV1 for StandaloneQualificationHarnessV1 {
    fn source_profile_sha256(&self) -> Sha256Digest {
        self.profile.profile_sha256
    }

    fn source_target(&self) -> super::manifest::CompilerTargetV1 {
        self.profile.target.clone()
    }

    fn standalone_compiler_identity(&self) -> Option<QualifiedSidecarIdentityV1> {
        Some(self.sidecar_authority)
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
                backend_error_text("caller case is not the sealed harness corpus row")
            })?;
        eprintln!(
            "standalone qualification [{}/{}] {}",
            authoritative.ordinal + 1,
            self.corpus.cases.len(),
            authoritative.case_id
        );
        match &authoritative.mode {
            ProbeModeV1::CompileGraphTransition {
                baseline_sections, ..
            } => {
                let baseline = self.run(
                    authoritative,
                    QualificationPhaseV3::GraphBaseline,
                    baseline_sections,
                    None,
                )?;
                let final_run = self.run(
                    authoritative,
                    QualificationPhaseV3::GraphFinal,
                    &authoritative.sections,
                    None,
                )?;
                match (baseline, final_run) {
                    (
                        QualificationSidecarRunV3::Accepted {
                            cache_bytes: baseline_cache,
                            diagnostics: baseline_diagnostics,
                            ..
                        },
                        QualificationSidecarRunV3::Accepted {
                            cache_bytes: final_cache,
                            diagnostics,
                            ..
                        },
                    ) if baseline_diagnostics.is_empty() => {
                        Ok(OfflineCapturedProbeOutputV1::accepted_graph_transition(
                            diagnostics,
                            baseline_cache,
                            final_cache,
                        ))
                    }
                    (baseline, final_run) => Err(backend_error_text(&format!(
                        "graph qualification did not produce accepted raw baseline/final caches; baseline {}; final {}",
                        graph_run_summary(&baseline),
                        graph_run_summary(&final_run),
                    ))),
                }
            }
            mode => {
                let invoke = match mode {
                    ProbeModeV1::CompileOnly => None,
                    ProbeModeV1::Invoke { declaration } => {
                        let module = authoritative
                            .sections
                            .first()
                            .filter(|_| authoritative.sections.len() == 1)
                            .ok_or_else(|| {
                                backend_error_text(
                                    "safe qualification invoke requires one exact source module",
                                )
                            })?;
                        Some((module.module.as_str(), declaration.as_str()))
                    }
                    ProbeModeV1::CompileGraphTransition { .. } => unreachable!(),
                };
                let run = self.run(
                    authoritative,
                    QualificationPhaseV3::Single,
                    &authoritative.sections,
                    invoke,
                )?;
                match (authoritative.expected_outcome, run) {
                    (
                        ProbeOutcomeV1::Accepted,
                        QualificationSidecarRunV3::Accepted {
                            diagnostics,
                            cache_bytes,
                            build_flags,
                            frontend,
                            invoke_return,
                        },
                    ) => {
                        validate_full_suite_accepted_cache_boundary_v1(authoritative, &cache_bytes)
                            .map_err(|error| {
                                backend_error_text(&format!(
                                    "standalone accepted-cache witness is incomplete: {error}"
                                ))
                            })?;
                        let invoke_return = invoke_return.map(|value| value.into_observer_value());
                        let supplemental = match authoritative.case_id.as_str() {
                            "positive.frontend.hooks-editor-release" => {
                                validate_frontend_coverage_witness(&frontend).map_err(|error| {
                                    backend_error_text(&format!(
                                        "standalone frontend witness is incomplete: {error}"
                                    ))
                                })?;
                                OfflineCapturedSupplementalWitnessV1::Frontend(frontend)
                            }
                            "positive.bytecode.fork-reference-lifecycle"
                            | "positive.bytecode.unresolved-object-property" => {
                                OfflineCapturedSupplementalWitnessV1::CompilerBuildFlags(
                                    build_flags,
                                )
                            }
                            _ => OfflineCapturedSupplementalWitnessV1::None,
                        };
                        Ok(OfflineCapturedProbeOutputV1::accepted_with_supplemental(
                            diagnostics,
                            cache_bytes,
                            invoke_return,
                            supplemental,
                        ))
                    }
                    (
                        ProbeOutcomeV1::Rejected,
                        QualificationSidecarRunV3::Rejected { diagnostics },
                    ) => {
                        if authoritative.case_id == "negative.diagnostics.located-warning-as-error"
                        {
                            validate_warning_as_error_diagnostics(&diagnostics)?;
                        }
                        Ok(OfflineCapturedProbeOutputV1::rejected(diagnostics))
                    }
                    (
                        ProbeOutcomeV1::Accepted,
                        QualificationSidecarRunV3::Rejected { diagnostics },
                    ) => Err(backend_error_text(&format!(
                        "standalone rejected a corpus case sealed as accepted{}",
                        diagnostic_summary(&diagnostics),
                    ))),
                    (ProbeOutcomeV1::Rejected, QualificationSidecarRunV3::Accepted { .. }) => Err(
                        backend_error_text("standalone accepted a corpus case sealed as rejected"),
                    ),
                }
            }
        }
    }
}

fn bind_profile_corpus(
    profile: &CompilerProfileV1,
    corpus: &CompilerProbeCorpusV1,
) -> Result<(), CompilerBackendFailureV1> {
    let json = corpus.to_json().map_err(|error| {
        CompilerBackendFailureV1::unavailable(format!("serializing canonical corpus: {error}"))
    })?;
    let seal = &profile.bytecode.codegen_probe_corpus;
    let digest = Sha256Digest::from_bytes(Sha256::digest(&json).into());
    if seal.byte_len != json.len() as u64 || seal.sha256 != digest {
        return Err(CompilerBackendFailureV1::unavailable(
            "unqualified profile does not seal the executable canonical 26-case corpus",
        ));
    }
    Ok(())
}

fn backend_error(failure: CompilerBackendFailureV1) -> CompilerProbeBackendErrorV1 {
    let prefix = failure.kind().as_str();
    backend_error_text(&format!("{prefix}: {}", failure.detail()))
}

fn backend_error_text(detail: &str) -> CompilerProbeBackendErrorV1 {
    CompilerProbeBackendErrorV1::new(detail).unwrap_or_else(|_| {
        CompilerProbeBackendErrorV1::static_internal(
            "standalone qualification failed with an invalid/oversized detail",
        )
    })
}

fn diagnostic_summary(diagnostics: &[super::qualification::ExpectedDiagnosticV1]) -> String {
    if diagnostics.is_empty() {
        return " without a diagnostic".to_owned();
    }
    let mut summary = format!("; diagnostics ({} total):", diagnostics.len());
    for diagnostic in diagnostics.iter().take(8) {
        let mut message = diagnostic.message.replace(['\r', '\n'], " ");
        message.truncate(message.floor_char_boundary(256));
        summary.push_str(&format!(
            " [{:?} at {:?}:{:?}:{:?}: {}]",
            diagnostic.severity, diagnostic.section, diagnostic.row, diagnostic.column, message,
        ));
    }
    summary
}

fn graph_run_summary(run: &QualificationSidecarRunV3) -> String {
    match run {
        QualificationSidecarRunV3::Accepted { diagnostics, .. } => {
            format!("accepted{}", diagnostic_summary(diagnostics))
        }
        QualificationSidecarRunV3::Rejected { diagnostics } => {
            format!("rejected{}", diagnostic_summary(diagnostics))
        }
    }
}

fn validate_warning_as_error_diagnostics(
    diagnostics: &[super::qualification::ExpectedDiagnosticV1],
) -> Result<(), CompilerProbeBackendErrorV1> {
    let located_warning = diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == super::qualification::ProbeDiagnosticSeverityV1::Warning
            && diagnostic.section.is_some()
            && diagnostic.row.is_some()
            && diagnostic.column.is_some()
    });
    let policy_error = diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == super::qualification::ProbeDiagnosticSeverityV1::Error
    });
    if !located_warning || !policy_error {
        return Err(backend_error_text(
            "warnings-as-errors probe omitted its located warning or policy error",
        ));
    }
    Ok(())
}

/// Fixed manifest path expected by the harness, useful to CLI/tool adapters without granting
/// any path authority.
pub fn unqualified_profile_manifest_path_v1(profile_root: &Path) -> PathBuf {
    profile_root.join(PROFILE_MANIFEST_FILE_V1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_backend::CompilerBackendFailureKindV1;

    #[test]
    fn harness_corpus_is_closed_and_contains_all_product_cases() {
        let corpus = full_qualification_corpus_v1().unwrap();
        assert_eq!(corpus.cases.len(), 26);
        assert_eq!(corpus.suite_id, FULL_QUALIFICATION_SUITE_ID_V1);
    }

    #[test]
    fn backend_errors_are_bounded() {
        let error = backend_error(CompilerBackendFailureV1::new(
            CompilerBackendFailureKindV1::Unavailable,
            "host-object invoke semantics are not sealed",
        ));
        assert!(error.detail().contains("unavailable"));
    }
}
