//! Deterministic differential-qualification orchestration.
//!
//! This module does not launch the game or trust paths. It executes an already sealed corpus
//! through two caller-provided backends, validates every structured observation, and emits the
//! exact diagnostic/semantic parity payloads consumed by [`super::qualification`]. Product code
//! must still supply an authorised embedded-game adapter and an exact, sealed standalone binary.

use std::collections::BTreeSet;

use super::manifest::Sha256Digest;
use super::qualification::{
    CompilerProbeCaseV1, CompilerProbeCorpusV1, DiagnosticParityEntryV1, DiagnosticParityReportV1,
    ExpectedDiagnosticV1, ExpectedProbeResultV1, ExpectedProbeResultsV1, ProbeModeV1,
    ProbeOutcomeV1, QualificationError, QualifiedSidecarIdentityV1, SemanticParityEntryV1,
    SemanticParityReportV1, DIAGNOSTIC_PARITY_SCHEMA, QUALIFICATION_SCHEMA_VERSION,
    SEMANTIC_PARITY_SCHEMA,
};
use crate::cache::semantic_observer::{
    observe_whole_cache_semantics_v1, CanonicalInvokeReturnV1, SemanticObserverError,
};

const MAX_BACKEND_FAILURE_BYTES_V1: usize = 16 * 1024;

/// Evidence contract used for every accepted-case semantic digest.
///
/// The observer itself hashes a fixed V1 domain before any cache field, so the digest carried by
/// the existing parity payloads is cryptographically version-bound even though their wire schema
/// predates this runner.
pub const SEMANTIC_OBSERVER_CONTRACT_V1: &str = "gore.as.whole-cache-semantic-observer/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerProbeBackendKindV1 {
    EmbeddedGame,
    Standalone,
}

impl CompilerProbeBackendKindV1 {
    fn label(self) -> &'static str {
        match self {
            Self::EmbeddedGame => "embedded_game",
            Self::Standalone => "standalone",
        }
    }
}

/// Raw accepted artifact supplied by a backend. It contains no caller-selected semantic digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedCompilerProbeArtifactV1 {
    cache_bytes: Vec<u8>,
    invoke_return: Option<CanonicalInvokeReturnV1>,
}

impl AcceptedCompilerProbeArtifactV1 {
    pub fn cache_bytes(&self) -> &[u8] {
        &self.cache_bytes
    }

    pub fn invoke_return(&self) -> Option<&CanonicalInvokeReturnV1> {
        self.invoke_return.as_ref()
    }
}

/// One closed backend observation.
///
/// Accepted observations always contain exact cache bytes. Rejected observations can contain
/// diagnostics only. The fields and constructors intentionally make it impossible for a backend
/// to inject a precomputed semantic digest or attach accepted artifacts to a rejected result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerProbeObservationV1 {
    outcome: ProbeOutcomeV1,
    diagnostics: Vec<ExpectedDiagnosticV1>,
    accepted_artifact: Option<AcceptedCompilerProbeArtifactV1>,
}

impl CompilerProbeObservationV1 {
    pub fn accepted(
        diagnostics: Vec<ExpectedDiagnosticV1>,
        cache_bytes: Vec<u8>,
        invoke_return: Option<CanonicalInvokeReturnV1>,
    ) -> Self {
        Self {
            outcome: ProbeOutcomeV1::Accepted,
            diagnostics,
            accepted_artifact: Some(AcceptedCompilerProbeArtifactV1 {
                cache_bytes,
                invoke_return,
            }),
        }
    }

    pub fn rejected(diagnostics: Vec<ExpectedDiagnosticV1>) -> Self {
        Self {
            outcome: ProbeOutcomeV1::Rejected,
            diagnostics,
            accepted_artifact: None,
        }
    }

    pub fn outcome(&self) -> ProbeOutcomeV1 {
        self.outcome
    }

    pub fn diagnostics(&self) -> &[ExpectedDiagnosticV1] {
        &self.diagnostics
    }

    pub fn accepted_artifact(&self) -> Option<&AcceptedCompilerProbeArtifactV1> {
        self.accepted_artifact.as_ref()
    }
}

/// Narrow adapter boundary used by the offline runner and the later authorised game oracle.
pub trait CompilerProbeBackendV1 {
    fn execute_probe(
        &mut self,
        case: &CompilerProbeCaseV1,
    ) -> Result<CompilerProbeObservationV1, CompilerProbeBackendErrorV1>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{detail}")]
pub struct CompilerProbeBackendErrorV1 {
    detail: String,
}

impl CompilerProbeBackendErrorV1 {
    pub fn new(detail: impl Into<String>) -> Result<Self, QualificationRunnerErrorV1> {
        let detail = detail.into();
        if detail.is_empty() || detail.len() > MAX_BACKEND_FAILURE_BYTES_V1 || detail.contains('\0')
        {
            return Err(QualificationRunnerErrorV1::InvalidBackendFailure);
        }
        Ok(Self { detail })
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub(crate) fn static_internal(detail: &'static str) -> Self {
        Self {
            detail: detail.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferentialQualificationRunV1 {
    pub diagnostic_parity: DiagnosticParityReportV1,
    pub semantic_parity: SemanticParityReportV1,
}

impl DifferentialQualificationRunV1 {
    pub fn qualified(&self) -> bool {
        self.semantic_parity.qualified
    }
}

/// Execute the complete ordered corpus exactly once per backend and seal both parity reports.
///
/// A backend process/transport failure is terminal and produces no partial report. A valid but
/// different compiler observation produces a sealed, explicitly unqualified report with stable
/// difference identifiers, which is useful for diagnosis but cannot pass the profile loader.
pub fn run_differential_qualification_v1(
    corpus: &CompilerProbeCorpusV1,
    expected: &ExpectedProbeResultsV1,
    standalone_compiler: QualifiedSidecarIdentityV1,
    embedded: &mut dyn CompilerProbeBackendV1,
    standalone: &mut dyn CompilerProbeBackendV1,
) -> Result<DifferentialQualificationRunV1, QualificationRunnerErrorV1> {
    corpus.validate()?;
    expected.validate()?;
    standalone_compiler.validate()?;
    validate_expected_against_corpus(corpus, expected)?;

    let mut diagnostic_entries = Vec::with_capacity(corpus.cases.len());
    let mut semantic_entries = Vec::new();
    let mut differences = BTreeSet::new();

    for (case, expected_result) in corpus.cases.iter().zip(&expected.results) {
        let embedded_observation =
            execute_checked(CompilerProbeBackendKindV1::EmbeddedGame, case, embedded)?;
        let standalone_observation =
            execute_checked(CompilerProbeBackendKindV1::Standalone, case, standalone)?;

        let expected_diagnostics = expected_result.diagnostics_sha256()?;
        let embedded_diagnostics = observation_diagnostics_sha256(&embedded_observation)?;
        let standalone_diagnostics = observation_diagnostics_sha256(&standalone_observation)?;
        diagnostic_entries.push(DiagnosticParityEntryV1 {
            ordinal: case.ordinal,
            case_id: case.case_id.clone(),
            expected_sha256: expected_diagnostics,
            embedded_sha256: embedded_diagnostics,
            standalone_sha256: standalone_diagnostics,
        });

        record_outcome_difference(
            &mut differences,
            case,
            expected_result.outcome,
            embedded_observation.outcome,
            CompilerProbeBackendKindV1::EmbeddedGame,
        );
        record_outcome_difference(
            &mut differences,
            case,
            expected_result.outcome,
            standalone_observation.outcome,
            CompilerProbeBackendKindV1::Standalone,
        );
        record_digest_difference(
            &mut differences,
            case,
            "diagnostics",
            expected_diagnostics,
            embedded_diagnostics,
            CompilerProbeBackendKindV1::EmbeddedGame,
        );
        record_digest_difference(
            &mut differences,
            case,
            "diagnostics",
            expected_diagnostics,
            standalone_diagnostics,
            CompilerProbeBackendKindV1::Standalone,
        );

        if let Some(expected_semantic) = expected_result.semantic_sha256 {
            let embedded_semantic = embedded_observation
                .semantic_sha256
                .unwrap_or_else(zero_digest);
            let standalone_semantic = standalone_observation
                .semantic_sha256
                .unwrap_or_else(zero_digest);
            semantic_entries.push(SemanticParityEntryV1 {
                ordinal: semantic_entries.len() as u32,
                case_id: case.case_id.clone(),
                expected_sha256: expected_semantic,
                embedded_sha256: embedded_semantic,
                standalone_sha256: standalone_semantic,
            });
            record_digest_difference(
                &mut differences,
                case,
                "semantics",
                expected_semantic,
                embedded_semantic,
                CompilerProbeBackendKindV1::EmbeddedGame,
            );
            record_digest_difference(
                &mut differences,
                case,
                "semantics",
                expected_semantic,
                standalone_semantic,
                CompilerProbeBackendKindV1::Standalone,
            );
        }
    }

    let mut diagnostic_parity = DiagnosticParityReportV1 {
        schema: DIAGNOSTIC_PARITY_SCHEMA.into(),
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        suite_id: corpus.suite_id.clone(),
        corpus_sha256: corpus.canonical_sha256,
        expected_results_sha256: expected.canonical_sha256,
        standalone_compiler,
        entries: diagnostic_entries,
        canonical_sha256: zero_digest(),
    };
    diagnostic_parity.seal()?;

    let unexplained_differences: Vec<_> = differences.into_iter().collect();
    let mut semantic_parity = SemanticParityReportV1 {
        schema: SEMANTIC_PARITY_SCHEMA.into(),
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        suite_id: corpus.suite_id.clone(),
        corpus_sha256: corpus.canonical_sha256,
        expected_results_sha256: expected.canonical_sha256,
        standalone_compiler,
        entries: semantic_entries,
        qualified: unexplained_differences.is_empty(),
        unexplained_differences,
        canonical_sha256: zero_digest(),
    };
    semantic_parity.seal()?;

    Ok(DifferentialQualificationRunV1 {
        diagnostic_parity,
        semantic_parity,
    })
}

fn validate_expected_against_corpus(
    corpus: &CompilerProbeCorpusV1,
    expected: &ExpectedProbeResultsV1,
) -> Result<(), QualificationRunnerErrorV1> {
    if expected.suite_id != corpus.suite_id
        || expected.corpus_sha256 != corpus.canonical_sha256
        || expected.results.len() != corpus.cases.len()
    {
        return Err(QualificationRunnerErrorV1::ExpectedCorpusMismatch);
    }
    for (case, result) in corpus.cases.iter().zip(&expected.results) {
        if case.ordinal != result.ordinal
            || case.case_id != result.case_id
            || case.expected_outcome != result.outcome
        {
            return Err(QualificationRunnerErrorV1::ExpectedCorpusMismatch);
        }
    }
    Ok(())
}

fn execute_checked(
    kind: CompilerProbeBackendKindV1,
    case: &CompilerProbeCaseV1,
    backend: &mut dyn CompilerProbeBackendV1,
) -> Result<CheckedCompilerProbeObservationV1, QualificationRunnerErrorV1> {
    let observation =
        backend
            .execute_probe(case)
            .map_err(|source| QualificationRunnerErrorV1::Backend {
                backend: kind.label(),
                case_id: case.case_id.clone(),
                source,
            })?;

    let semantic_sha256 = match (&observation.outcome, &observation.accepted_artifact) {
        (ProbeOutcomeV1::Accepted, Some(artifact)) => {
            match (&case.mode, artifact.invoke_return.as_ref()) {
                (ProbeModeV1::CompileOnly, None)
                | (ProbeModeV1::CompileGraphTransition { .. }, None)
                | (ProbeModeV1::Invoke { .. }, Some(_)) => {}
                (
                    ProbeModeV1::CompileOnly | ProbeModeV1::CompileGraphTransition { .. },
                    Some(_),
                ) => {
                    return Err(QualificationRunnerErrorV1::ObservationShape {
                        backend: kind.label(),
                        case_id: case.case_id.clone(),
                        detail: "compile-only probe supplied an invocation return",
                    });
                }
                (ProbeModeV1::Invoke { .. }, None) => {
                    return Err(QualificationRunnerErrorV1::ObservationShape {
                        backend: kind.label(),
                        case_id: case.case_id.clone(),
                        detail: "invoke probe omitted its typed invocation return",
                    });
                }
            }
            if let Some(value) = artifact.invoke_return.as_ref() {
                if value.type_identity().is_empty()
                    || value.type_identity().contains('\0')
                    || value.type_identity().chars().any(char::is_control)
                {
                    return Err(QualificationRunnerErrorV1::ObservationShape {
                        backend: kind.label(),
                        case_id: case.case_id.clone(),
                        detail: "invoke return type identity is empty or contains controls",
                    });
                }
            }
            let semantic = observe_whole_cache_semantics_v1(
                &artifact.cache_bytes,
                artifact.invoke_return.as_ref(),
            )
            .map_err(|source| QualificationRunnerErrorV1::SemanticObservation {
                backend: kind.label(),
                case_id: case.case_id.clone(),
                source,
            })?;
            Some(Sha256Digest::from_bytes(*semantic.sha256()))
        }
        (ProbeOutcomeV1::Rejected, None) => None,
        (ProbeOutcomeV1::Accepted, None) => {
            return Err(QualificationRunnerErrorV1::ObservationShape {
                backend: kind.label(),
                case_id: case.case_id.clone(),
                detail: "accepted probe omitted its cache artifact",
            });
        }
        (ProbeOutcomeV1::Rejected, Some(_)) => {
            return Err(QualificationRunnerErrorV1::ObservationShape {
                backend: kind.label(),
                case_id: case.case_id.clone(),
                detail: "rejected probe supplied an accepted cache artifact",
            });
        }
    };

    // Reuse the canonical qualification validator instead of maintaining a second set of
    // diagnostic/outcome constraints. The semantic digest written here was just produced by the
    // V1 whole-cache observer; it never came from the backend. Validate a one-row envelope so work
    // remains linear even for the maximum corpus size.
    let mut validation = ExpectedProbeResultsV1 {
        schema: super::qualification::EXPECTED_RESULTS_SCHEMA.into(),
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        suite_id: "backend-observation-validation".into(),
        corpus_sha256: zero_digest(),
        results: vec![ExpectedProbeResultV1 {
            ordinal: 0,
            case_id: case.case_id.clone(),
            outcome: observation.outcome,
            diagnostics: observation.diagnostics.clone(),
            semantic_sha256,
        }],
        canonical_sha256: zero_digest(),
    };
    validation
        .seal()
        .map_err(|source| QualificationRunnerErrorV1::InvalidObservation {
            backend: kind.label(),
            case_id: case.case_id.clone(),
            source,
        })?;
    Ok(CheckedCompilerProbeObservationV1 {
        outcome: observation.outcome,
        diagnostics: observation.diagnostics,
        semantic_sha256,
    })
}

fn observation_diagnostics_sha256(
    observation: &CheckedCompilerProbeObservationV1,
) -> Result<Sha256Digest, QualificationError> {
    ExpectedProbeResultV1 {
        ordinal: 0,
        case_id: "observation".into(),
        outcome: observation.outcome,
        diagnostics: observation.diagnostics.clone(),
        semantic_sha256: observation.semantic_sha256,
    }
    .diagnostics_sha256()
}

struct CheckedCompilerProbeObservationV1 {
    outcome: ProbeOutcomeV1,
    diagnostics: Vec<ExpectedDiagnosticV1>,
    semantic_sha256: Option<Sha256Digest>,
}

fn record_outcome_difference(
    differences: &mut BTreeSet<String>,
    case: &CompilerProbeCaseV1,
    expected: ProbeOutcomeV1,
    actual: ProbeOutcomeV1,
    backend: CompilerProbeBackendKindV1,
) {
    if expected != actual {
        differences.insert(format!("{}:outcome:{}", case.case_id, backend.label()));
    }
}

fn record_digest_difference(
    differences: &mut BTreeSet<String>,
    case: &CompilerProbeCaseV1,
    dimension: &'static str,
    expected: Sha256Digest,
    actual: Sha256Digest,
    backend: CompilerProbeBackendKindV1,
) {
    if expected != actual {
        differences.insert(format!(
            "{}:{}:{}",
            case.case_id,
            dimension,
            backend.label()
        ));
    }
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

#[derive(Debug, thiserror::Error)]
pub enum QualificationRunnerErrorV1 {
    #[error("qualification input is invalid: {0}")]
    Qualification(#[from] QualificationError),
    #[error("expected results do not exactly cover the ordered probe corpus")]
    ExpectedCorpusMismatch,
    #[error("backend failure detail is empty, oversized, or contains NUL")]
    InvalidBackendFailure,
    #[error("{backend} backend failed for probe {case_id:?}: {source}")]
    Backend {
        backend: &'static str,
        case_id: String,
        source: CompilerProbeBackendErrorV1,
    },
    #[error("{backend} returned an invalid observation for probe {case_id:?}: {source}")]
    InvalidObservation {
        backend: &'static str,
        case_id: String,
        source: QualificationError,
    },
    #[error("{backend} returned an invalid artifact shape for probe {case_id:?}: {detail}")]
    ObservationShape {
        backend: &'static str,
        case_id: String,
        detail: &'static str,
    },
    #[error("{backend} cache observation failed for probe {case_id:?}: {source}")]
    SemanticObservation {
        backend: &'static str,
        case_id: String,
        source: SemanticObserverError,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use sha2::{Digest as _, Sha256};

    use super::*;
    use crate::compiler_profile::qualification::{
        CompilerProbeCaseV1, ExpectedProbeResultsV1, ProbeDiagnosticSeverityV1, ProbeModeV1,
        ProbeSourceSectionV1, EXPECTED_RESULTS_SCHEMA, PROBE_CORPUS_SCHEMA,
        QUALIFIED_SIDECAR_REQUEST_VERSION_V1, QUALIFIED_SIDECAR_RESPONSE_VERSION_V1,
    };

    struct QueueBackend(VecDeque<Result<CompilerProbeObservationV1, CompilerProbeBackendErrorV1>>);

    impl CompilerProbeBackendV1 for QueueBackend {
        fn execute_probe(
            &mut self,
            _case: &CompilerProbeCaseV1,
        ) -> Result<CompilerProbeObservationV1, CompilerProbeBackendErrorV1> {
            self.0.pop_front().expect("one observation per case")
        }
    }

    fn digest(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }

    fn cache(build_identifier: i32) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        bytes.extend_from_slice(&build_identifier.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes()); // Modules
        for _ in 0..7 {
            bytes.extend_from_slice(&0i32.to_le_bytes());
        }
        bytes
    }

    fn semantic(cache: &[u8]) -> Sha256Digest {
        Sha256Digest::from_bytes(
            *observe_whole_cache_semantics_v1(cache, None)
                .unwrap()
                .sha256(),
        )
    }

    fn fixture() -> (CompilerProbeCorpusV1, ExpectedProbeResultsV1) {
        let source = "int Answer() { return 42; }";
        let mut corpus = CompilerProbeCorpusV1 {
            schema: PROBE_CORPUS_SCHEMA.into(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: "runner-fixture-v1".into(),
            cases: vec![
                CompilerProbeCaseV1 {
                    ordinal: 0,
                    case_id: "positive".into(),
                    category: "codegen".into(),
                    expected_outcome: ProbeOutcomeV1::Accepted,
                    mode: ProbeModeV1::CompileOnly,
                    sections: vec![ProbeSourceSectionV1 {
                        ordinal: 0,
                        module: "Positive".into(),
                        relative_path: "Positive.as".into(),
                        source_utf8: source.into(),
                        source_sha256: Sha256Digest::from_bytes(Sha256::digest(source).into()),
                    }],
                },
                CompilerProbeCaseV1 {
                    ordinal: 1,
                    case_id: "negative".into(),
                    category: "parser".into(),
                    expected_outcome: ProbeOutcomeV1::Rejected,
                    mode: ProbeModeV1::CompileOnly,
                    sections: vec![ProbeSourceSectionV1 {
                        ordinal: 0,
                        module: "Negative".into(),
                        relative_path: "Negative.as".into(),
                        source_utf8: "void Broken( {".into(),
                        source_sha256: Sha256Digest::from_bytes(
                            Sha256::digest(b"void Broken( {").into(),
                        ),
                    }],
                },
            ],
            canonical_sha256: zero_digest(),
        };
        corpus.seal().unwrap();

        let mut expected = ExpectedProbeResultsV1 {
            schema: EXPECTED_RESULTS_SCHEMA.into(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            results: vec![
                ExpectedProbeResultV1 {
                    ordinal: 0,
                    case_id: "positive".into(),
                    outcome: ProbeOutcomeV1::Accepted,
                    diagnostics: vec![],
                    semantic_sha256: Some(semantic(&cache(42))),
                },
                ExpectedProbeResultV1 {
                    ordinal: 1,
                    case_id: "negative".into(),
                    outcome: ProbeOutcomeV1::Rejected,
                    diagnostics: vec![ExpectedDiagnosticV1 {
                        ordinal: 0,
                        severity: ProbeDiagnosticSeverityV1::Error,
                        section: Some("Negative.as".into()),
                        row: Some(1),
                        column: Some(14),
                        message: "Expected data type".into(),
                    }],
                    semantic_sha256: None,
                },
            ],
            canonical_sha256: zero_digest(),
        };
        expected.seal().unwrap();
        (corpus, expected)
    }

    fn observations(
        expected: &ExpectedProbeResultsV1,
        accepted_cache: Vec<u8>,
    ) -> VecDeque<Result<CompilerProbeObservationV1, CompilerProbeBackendErrorV1>> {
        expected
            .results
            .iter()
            .map(|result| {
                Ok(match result.outcome {
                    ProbeOutcomeV1::Accepted => CompilerProbeObservationV1::accepted(
                        result.diagnostics.clone(),
                        accepted_cache.clone(),
                        None,
                    ),
                    ProbeOutcomeV1::Rejected => {
                        CompilerProbeObservationV1::rejected(result.diagnostics.clone())
                    }
                })
            })
            .collect()
    }

    fn sidecar() -> QualifiedSidecarIdentityV1 {
        QualifiedSidecarIdentityV1 {
            byte_len: 1234,
            sha256: digest(7),
            request_version: QUALIFIED_SIDECAR_REQUEST_VERSION_V1,
            response_version: QUALIFIED_SIDECAR_RESPONSE_VERSION_V1,
        }
    }

    #[test]
    fn complete_run_qualifies_and_drift_is_sealed_but_not_qualified() {
        let (corpus, expected) = fixture();
        let mut embedded = QueueBackend(observations(&expected, cache(42)));
        let mut standalone = QueueBackend(observations(&expected, cache(42)));
        let run = run_differential_qualification_v1(
            &corpus,
            &expected,
            sidecar(),
            &mut embedded,
            &mut standalone,
        )
        .unwrap();
        assert!(run.qualified());
        assert_eq!(run.diagnostic_parity.entries.len(), 2);
        assert_eq!(run.semantic_parity.entries.len(), 1);

        let drifted = observations(&expected, cache(99));
        let mut embedded = QueueBackend(observations(&expected, cache(42)));
        let mut standalone = QueueBackend(drifted);
        let run = run_differential_qualification_v1(
            &corpus,
            &expected,
            sidecar(),
            &mut embedded,
            &mut standalone,
        )
        .unwrap();
        assert!(!run.qualified());
        assert_eq!(
            run.semantic_parity.unexplained_differences,
            vec!["positive:semantics:standalone"]
        );

        let raw_cache_sha256 = Sha256Digest::from_bytes(Sha256::digest(cache(42)).into());
        assert_ne!(raw_cache_sha256, semantic(&cache(42)));
        let mut raw_expected = expected.clone();
        raw_expected.results[0].semantic_sha256 = Some(raw_cache_sha256);
        raw_expected.seal().unwrap();
        let mut embedded = QueueBackend(observations(&raw_expected, cache(42)));
        let mut standalone = QueueBackend(observations(&raw_expected, cache(42)));
        let run = run_differential_qualification_v1(
            &corpus,
            &raw_expected,
            sidecar(),
            &mut embedded,
            &mut standalone,
        )
        .unwrap();
        assert_eq!(
            run.semantic_parity.unexplained_differences,
            vec![
                "positive:semantics:embedded_game",
                "positive:semantics:standalone"
            ]
        );
    }

    #[test]
    fn malformed_observation_or_backend_failure_never_emits_partial_evidence() {
        let (corpus, expected) = fixture();
        let mut invalid = observations(&expected, cache(42));
        invalid[1] = Ok(CompilerProbeObservationV1::rejected(vec![]));
        let mut embedded = QueueBackend(invalid);
        let mut standalone = QueueBackend(observations(&expected, cache(42)));
        assert!(matches!(
            run_differential_qualification_v1(
                &corpus,
                &expected,
                sidecar(),
                &mut embedded,
                &mut standalone,
            ),
            Err(QualificationRunnerErrorV1::InvalidObservation { .. })
        ));

        let mut failed = observations(&expected, cache(42));
        failed[0] = Err(CompilerProbeBackendErrorV1::new("process failed").unwrap());
        let mut embedded = QueueBackend(failed);
        let mut standalone = QueueBackend(observations(&expected, cache(42)));
        assert!(matches!(
            run_differential_qualification_v1(
                &corpus,
                &expected,
                sidecar(),
                &mut embedded,
                &mut standalone,
            ),
            Err(QualificationRunnerErrorV1::Backend { .. })
        ));

        let mut malformed_cache = cache(42);
        malformed_cache.push(0);
        let mut embedded = QueueBackend(observations(&expected, malformed_cache));
        let mut standalone = QueueBackend(observations(&expected, cache(42)));
        assert!(matches!(
            run_differential_qualification_v1(
                &corpus,
                &expected,
                sidecar(),
                &mut embedded,
                &mut standalone,
            ),
            Err(QualificationRunnerErrorV1::SemanticObservation { .. })
        ));

        let mut invoke_corpus = corpus.clone();
        invoke_corpus.cases[0].mode = ProbeModeV1::Invoke {
            declaration: "int Answer()".into(),
        };
        invoke_corpus.seal().unwrap();
        let mut invoke_expected = expected.clone();
        invoke_expected.corpus_sha256 = invoke_corpus.canonical_sha256;
        invoke_expected.seal().unwrap();
        let mut embedded = QueueBackend(observations(&invoke_expected, cache(42)));
        let mut standalone = QueueBackend(observations(&invoke_expected, cache(42)));
        assert!(matches!(
            run_differential_qualification_v1(
                &invoke_corpus,
                &invoke_expected,
                sidecar(),
                &mut embedded,
                &mut standalone,
            ),
            Err(QualificationRunnerErrorV1::ObservationShape { .. })
        ));
    }
}
