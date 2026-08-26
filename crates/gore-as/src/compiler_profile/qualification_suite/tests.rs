use sha2::{Digest as _, Sha256};

use super::*;
use crate::cache::semantic_observer::observe_whole_cache_semantics_v1;
use crate::cache::semantic_observer::tests::{
    synthetic_observer_qualification_cache_for_module_v1,
    synthetic_observer_qualification_fixture_v1,
};
use crate::compiler_profile::manifest::{
    CompilerArchitectureV1, CompilerBuildConfigurationV1, CompilerPlatformV1, CompilerTargetV1,
};
use crate::compiler_profile::qualification::{
    ProbeDiagnosticSeverityV1, QUALIFIED_SIDECAR_REQUEST_VERSION_V2,
    QUALIFIED_SIDECAR_RESPONSE_VERSION_V1,
};

fn source_profile_sha256() -> Sha256Digest {
    Sha256Digest::from_bytes([0x44; 32])
}

fn source_target() -> CompilerTargetV1 {
    CompilerTargetV1 {
        steam_app_id: 1_297_900,
        steam_build_id: 24_539_464,
        depot_id: 1_297_901,
        depot_manifest_gid: 1_585_071_322_101_748_861,
        platform: CompilerPlatformV1::Windows,
        architecture: CompilerArchitectureV1::X86_64,
        build_configuration: CompilerBuildConfigurationV1::Shipping,
    }
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

fn cache_with_modules(build_identifier: i32, modules: &[(&str, i64)]) -> Vec<u8> {
    cache_with_modules_and_static_names(build_identifier, modules, &[])
}

fn cache_with_modules_and_static_names(
    build_identifier: i32,
    modules: &[(&str, i64)],
    static_names: &[&str],
) -> Vec<u8> {
    fn i32v(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn i64v(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn sia(bytes: &mut Vec<u8>, value: &str) {
        i32v(bytes, value.len() as i32);
        if !value.is_empty() {
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
        }
    }
    fn fstring(bytes: &mut Vec<u8>, value: &str) {
        i32v(bytes, value.len() as i32 + 1);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
    }

    let mut bytes = vec![0; 16];
    i32v(&mut bytes, build_identifier);
    i32v(&mut bytes, modules.len() as i32);
    for &(module, code_hash) in modules {
        fstring(&mut bytes, module);
        sia(&mut bytes, module);
        for _ in 0..5 {
            i32v(&mut bytes, 0);
        }
        i64v(&mut bytes, code_hash);
        i32v(&mut bytes, 0); // imported modules
        sia(&mut bytes, &format!("{module}Statics"));
        i32v(&mut bytes, 0); // events
        i32v(&mut bytes, 0); // delegates
        sia(&mut bytes, &format!("Graph/{module}.as"));
        i32v(&mut bytes, 0); // post-init functions
    }
    for _ in 0..5 {
        i32v(&mut bytes, 0);
    }
    i32v(&mut bytes, static_names.len() as i32);
    for name in static_names {
        sia(&mut bytes, name);
    }
    i32v(&mut bytes, 0);
    bytes
}

fn seal_blob(blob_id: &str, bytes: &[u8]) -> OfflineCacheArtifactSealV1 {
    OfflineCacheArtifactSealV1 {
        blob_id: blob_id.into(),
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn rejected_diagnostics(case: &CompilerProbeCaseV1) -> Vec<ExpectedDiagnosticV1> {
    let mut diagnostics = Vec::new();
    if case.case_id == "negative.overloads.ambiguous" {
        diagnostics.push(ExpectedDiagnosticV1 {
            ordinal: 0,
            severity: ProbeDiagnosticSeverityV1::Info,
            section: Some(case.sections[0].relative_path.clone()),
            row: Some(1),
            column: Some(1),
            message: "candidate overload context".into(),
        });
    }
    if case.case_id == "negative.diagnostics.located-warning-as-error" {
        diagnostics.push(ExpectedDiagnosticV1 {
            ordinal: diagnostics.len() as u32,
            severity: ProbeDiagnosticSeverityV1::Warning,
            section: Some(case.sections[0].relative_path.clone()),
            row: Some(1),
            column: Some(1),
            message: "assigning variable Value to itself".into(),
        });
    }
    diagnostics.push(ExpectedDiagnosticV1 {
        ordinal: diagnostics.len() as u32,
        severity: ProbeDiagnosticSeverityV1::Error,
        section: Some(case.sections[0].relative_path.clone()),
        row: Some(1),
        column: Some(1),
        message: format!("captured rejection for {}", case.case_id),
    });
    diagnostics
}

fn accepted_diagnostics(case: &CompilerProbeCaseV1) -> Vec<ExpectedDiagnosticV1> {
    let _ = case;
    vec![]
}

fn package(
    corpus: &CompilerProbeCorpusV1,
    backend: CompilerProbeBackendKindV1,
) -> (
    OfflineCompilerProbeArtifactManifestV1,
    BTreeMap<String, Vec<u8>>,
) {
    let mut blobs = BTreeMap::new();
    let entries = corpus
        .cases
        .iter()
        .map(|case| match case.expected_outcome {
            ProbeOutcomeV1::Accepted => {
                let blob_id = format!("{}.cache", case.case_id);
                let bytes = if case.case_id == "positive.fname.name-none-canonical" {
                    cache_with_modules_and_static_names(
                        1000 + case.ordinal as i32,
                        &[(case.sections[0].module.as_str(), 1)],
                        &["None"],
                    )
                } else if matches!(
                    case.case_id.as_str(),
                    "positive.bytecode.fork-reference-lifecycle"
                        | "positive.bytecode.unresolved-object-property"
                        | "positive.class-generator.editor-flags"
                        | "positive.model.globals-classes-all-tails"
                        | "positive.strings.factory-roundtrip"
                ) {
                    synthetic_observer_qualification_cache_for_module_v1(&case.sections[0].module)
                } else if case.case_id == "positive.module-graph.change-delete" {
                    cache_with_modules(
                        1000 + case.ordinal as i32,
                        &[
                            ("Graph.ChangedModule", 2),
                            ("Graph.AddedModule", 1),
                            ("RetainedBaseModule", 1),
                        ],
                    )
                } else {
                    let modules: Vec<_> = case
                        .sections
                        .iter()
                        .map(|section| (section.module.as_str(), 1))
                        .collect();
                    cache_with_modules(1000 + case.ordinal as i32, &modules)
                };
                let cache = seal_blob(&blob_id, &bytes);
                let invoke_return = match case.case_id.as_str() {
                    "positive.invoke.structured" => Some(OfflineCanonicalInvokeReturnV1 {
                        type_identity: "TArray<int32>".into(),
                        value: OfflineCanonicalInvokeValueV1::Sequence(vec![
                            OfflineCanonicalInvokeValueV1::I64(0),
                            OfflineCanonicalInvokeValueV1::I64(0),
                        ]),
                    }),
                    "positive.fname.non-ascii-equivalence"
                    | "positive.fname.name-none-canonical" => {
                        Some(OfflineCanonicalInvokeReturnV1 {
                            type_identity: "bool".into(),
                            value: OfflineCanonicalInvokeValueV1::Bool(true),
                        })
                    }
                    "positive.strings.factory-roundtrip" => Some(OfflineCanonicalInvokeReturnV1 {
                        type_identity: "FString".into(),
                        value: OfflineCanonicalInvokeValueV1::Utf8("Grüße_日本".into()),
                    }),
                    _ if matches!(case.mode, ProbeModeV1::Invoke { .. }) => {
                        Some(OfflineCanonicalInvokeReturnV1 {
                            type_identity: "qualification::canonical-result::i64".into(),
                            value: OfflineCanonicalInvokeValueV1::I64(42),
                        })
                    }
                    _ => None,
                };
                let observer_invoke = invoke_return
                    .clone()
                    .map(OfflineCanonicalInvokeReturnV1::into_observer_value);
                let semantic_observation =
                    observe_whole_cache_semantics_v1(&bytes, observer_invoke.as_ref()).unwrap();
                let cache_semantics =
                    OfflineCacheSemanticWitnessV1::from_observation(&semantic_observation);
                let graph_transition = if case.case_id == "positive.module-graph.change-delete" {
                    let baseline_bytes = cache_with_modules(
                        1000 + case.ordinal as i32,
                        &[
                            ("Graph.ChangedModule", 1),
                            ("Graph.DeletedModule", 1),
                            ("RetainedBaseModule", 1),
                        ],
                    );
                    let baseline_blob_id = format!("{}.baseline.cache", case.case_id);
                    let baseline_cache = seal_blob(&baseline_blob_id, &baseline_bytes);
                    let baseline_observation =
                        observe_whole_cache_semantics_v1(&baseline_bytes, None).unwrap();
                    blobs.insert(baseline_blob_id, baseline_bytes);
                    graph_transition_witness_for_case(
                        case,
                        baseline_cache,
                        &baseline_observation,
                        &semantic_observation,
                    )
                } else {
                    None
                };
                blobs.insert(blob_id, bytes);
                OfflineCompilerProbeArtifactEntryV1 {
                    ordinal: case.ordinal,
                    case_id: case.case_id.clone(),
                    outcome: ProbeOutcomeV1::Accepted,
                    diagnostics: accepted_diagnostics(case),
                    cache: Some(cache),
                    cache_semantics: Some(cache_semantics),
                    invoke_return,
                    frontend_coverage: (case.case_id == "positive.frontend.hooks-editor-release")
                        .then(frontend_coverage_witness),
                    graph_transition,
                    compiler_build_flags: (matches!(
                        case.case_id.as_str(),
                        "positive.bytecode.fork-reference-lifecycle"
                            | "positive.bytecode.unresolved-object-property"
                    ))
                    .then_some(OfflineCompilerBuildFlagsWitnessV1 {
                        as_reference_debugging: false,
                        resolve_object_ptr_callback_registered: false,
                    }),
                }
            }
            ProbeOutcomeV1::Rejected => OfflineCompilerProbeArtifactEntryV1 {
                ordinal: case.ordinal,
                case_id: case.case_id.clone(),
                outcome: ProbeOutcomeV1::Rejected,
                diagnostics: rejected_diagnostics(case),
                cache: None,
                cache_semantics: None,
                invoke_return: None,
                frontend_coverage: None,
                graph_transition: None,
                compiler_build_flags: None,
            },
        })
        .collect();
    let mut manifest = OfflineCompilerProbeArtifactManifestV1 {
        schema: OFFLINE_PROBE_ARTIFACT_SCHEMA_V1.into(),
        schema_version: OFFLINE_PROBE_ARTIFACT_SCHEMA_VERSION_V1,
        semantic_observer: SEMANTIC_OBSERVER_CONTRACT_V1.into(),
        suite_id: corpus.suite_id.clone(),
        corpus_sha256: corpus.canonical_sha256,
        backend,
        source_profile_sha256: source_profile_sha256(),
        source_target: source_target(),
        standalone_compiler: (backend == CompilerProbeBackendKindV1::Standalone)
            .then_some(sidecar()),
        entries,
        canonical_sha256: zero_digest(),
    };
    manifest.seal().unwrap();
    (manifest, blobs)
}

fn frontend_coverage_witness() -> OfflineFrontendCoverageWitnessV1 {
    let capture = |subject: &str, declaration: &str| OfflineFrontendHookCaptureV1 {
        subject_identity: subject.into(),
        generated_declarations: vec![declaration.into()],
    };
    OfflineFrontendCoverageWitnessV1 {
        class_analyze_bound: true,
        class_analyze_captures: vec![capture(
            "UQualificationEditorHook",
            "class-analyze::UQualificationEditorHook",
        )],
        process_chunks_bound: false,
        process_chunks_captures: Vec::new(),
        post_process_code_bound: false,
        post_process_captures: Vec::new(),
        generated_declarations: vec!["class-analyze::UQualificationEditorHook".into()],
        editor_discovery: vec!["UQualificationEditorHook".into()],
        release_discovery: vec!["QualificationReleaseDiscovery".into()],
    }
}

fn expected_from_package(
    corpus: &CompilerProbeCorpusV1,
    manifest: &OfflineCompilerProbeArtifactManifestV1,
    blobs: &BTreeMap<String, Vec<u8>>,
) -> ExpectedProbeResultsV1 {
    let results = corpus
        .cases
        .iter()
        .zip(&manifest.entries)
        .map(|(case, entry)| {
            let semantic_sha256 = entry.cache.as_ref().map(|seal| {
                let invoke = entry
                    .invoke_return
                    .clone()
                    .map(OfflineCanonicalInvokeReturnV1::into_observer_value);
                Sha256Digest::from_bytes(
                    *observe_whole_cache_semantics_v1(&blobs[&seal.blob_id], invoke.as_ref())
                        .unwrap()
                        .sha256(),
                )
            });
            ExpectedProbeResultV1 {
                ordinal: case.ordinal,
                case_id: case.case_id.clone(),
                outcome: entry.outcome,
                diagnostics: entry.diagnostics.clone(),
                semantic_sha256,
            }
        })
        .collect();
    let mut expected = ExpectedProbeResultsV1 {
        schema: EXPECTED_RESULTS_SCHEMA.into(),
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        suite_id: corpus.suite_id.clone(),
        corpus_sha256: corpus.canonical_sha256,
        results,
        canonical_sha256: zero_digest(),
    };
    expected.seal().unwrap();
    expected
}

fn sidecar() -> QualifiedSidecarIdentityV1 {
    QualifiedSidecarIdentityV1 {
        byte_len: 4096,
        sha256: Sha256Digest::from_bytes([0x5a; 32]),
        request_version: QUALIFIED_SIDECAR_REQUEST_VERSION_V2,
        response_version: QUALIFIED_SIDECAR_RESPONSE_VERSION_V1,
    }
}

pub(crate) fn canonical_full_promotion_fixture_v1(
    source_profile_sha256: Sha256Digest,
    source_target: CompilerTargetV1,
) -> (CompilerProbeCorpusV1, OfflineQualificationPromotionV1) {
    let corpus = full_qualification_corpus_v1().unwrap();
    let mut embedded_package = package(&corpus, CompilerProbeBackendKindV1::EmbeddedGame);
    embedded_package.0.source_profile_sha256 = source_profile_sha256;
    embedded_package.0.source_target = source_target.clone();
    embedded_package.0.seal().unwrap();
    let mut standalone_package = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    standalone_package.0.source_profile_sha256 = source_profile_sha256;
    standalone_package.0.source_target = source_target;
    standalone_package.0.seal().unwrap();
    let mut embedded_backend = SyntheticCaptureBackend::from_package(embedded_package);
    let mut standalone_backend = SyntheticCaptureBackend::from_package(standalone_package);
    let embedded = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        &mut embedded_backend,
    )
    .unwrap();
    let standalone = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut standalone_backend,
    )
    .unwrap();
    let reloaded_embedded = reload_generated_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        embedded.manifest_json(),
        embedded.cache_blobs().clone(),
    )
    .unwrap();
    let reloaded_standalone = reload_generated_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        standalone.manifest_json(),
        standalone.cache_blobs().clone(),
    )
    .unwrap();
    let promotion = promote_generated_offline_qualification_artifacts_v1(
        &corpus,
        &reloaded_embedded,
        &reloaded_standalone,
    )
    .unwrap();
    (corpus, promotion)
}

fn reseal_blob(
    manifest: &mut OfflineCompilerProbeArtifactManifestV1,
    blobs: &mut BTreeMap<String, Vec<u8>>,
    case_id: &str,
    bytes: Vec<u8>,
) {
    let entry = manifest
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == case_id)
        .unwrap();
    let blob_id = entry.cache.as_ref().unwrap().blob_id.clone();
    entry.cache = Some(seal_blob(&blob_id, &bytes));
    let invoke = entry
        .invoke_return
        .clone()
        .map(OfflineCanonicalInvokeReturnV1::into_observer_value);
    if let Ok(observation) = observe_whole_cache_semantics_v1(&bytes, invoke.as_ref()) {
        entry.cache_semantics = Some(OfflineCacheSemanticWitnessV1::from_observation(
            &observation,
        ));
    }
    blobs.insert(blob_id, bytes);
    manifest.seal().unwrap();
}

fn reseal_entry_semantics(
    manifest: &mut OfflineCompilerProbeArtifactManifestV1,
    blobs: &BTreeMap<String, Vec<u8>>,
    case_id: &str,
) {
    let entry = manifest
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == case_id)
        .unwrap();
    let blob_id = &entry.cache.as_ref().unwrap().blob_id;
    let invoke = entry
        .invoke_return
        .clone()
        .map(OfflineCanonicalInvokeReturnV1::into_observer_value);
    entry.cache_semantics = Some(OfflineCacheSemanticWitnessV1::from_observation(
        &observe_whole_cache_semantics_v1(&blobs[blob_id], invoke.as_ref()).unwrap(),
    ));
    manifest.seal().unwrap();
}

fn run(
    corpus: &CompilerProbeCorpusV1,
    expected: &ExpectedProbeResultsV1,
    embedded: &(
        OfflineCompilerProbeArtifactManifestV1,
        BTreeMap<String, Vec<u8>>,
    ),
    standalone: &(
        OfflineCompilerProbeArtifactManifestV1,
        BTreeMap<String, Vec<u8>>,
    ),
) -> Result<DifferentialQualificationRunV1, OfflineQualificationErrorV1> {
    run_offline_artifact_differential_qualification_v1(
        corpus,
        expected,
        &embedded.0.to_json().unwrap(),
        embedded.1.clone(),
        &standalone.0.to_json().unwrap(),
        standalone.1.clone(),
    )
}

struct SyntheticCaptureBackend {
    entries: VecDeque<OfflineCompilerProbeArtifactEntryV1>,
    blobs: BTreeMap<String, Vec<u8>>,
    source_profile_sha256: Sha256Digest,
    source_target: CompilerTargetV1,
    standalone_compiler: Option<QualifiedSidecarIdentityV1>,
    relabel_after_capture: Option<QualifiedSidecarIdentityV1>,
}

impl SyntheticCaptureBackend {
    fn from_package(
        package: (
            OfflineCompilerProbeArtifactManifestV1,
            BTreeMap<String, Vec<u8>>,
        ),
    ) -> Self {
        let manifest = package.0;
        Self {
            entries: manifest.entries.into(),
            blobs: package.1,
            source_profile_sha256: manifest.source_profile_sha256,
            source_target: manifest.source_target,
            standalone_compiler: manifest.standalone_compiler,
            relabel_after_capture: None,
        }
    }
}

impl OfflineQualificationCaptureBackendV1 for SyntheticCaptureBackend {
    fn source_profile_sha256(&self) -> Sha256Digest {
        self.source_profile_sha256
    }

    fn source_target(&self) -> CompilerTargetV1 {
        self.source_target.clone()
    }

    fn standalone_compiler_identity(&self) -> Option<QualifiedSidecarIdentityV1> {
        if self.entries.is_empty() {
            self.relabel_after_capture.or(self.standalone_compiler)
        } else {
            self.standalone_compiler
        }
    }

    fn capture_probe(
        &mut self,
        case: &CompilerProbeCaseV1,
    ) -> Result<OfflineCapturedProbeOutputV1, CompilerProbeBackendErrorV1> {
        let entry = self.entries.pop_front().ok_or_else(|| {
            CompilerProbeBackendErrorV1::static_internal("synthetic capture ended early")
        })?;
        if entry.case_id != case.case_id {
            return Err(CompilerProbeBackendErrorV1::static_internal(
                "synthetic capture order mismatch",
            ));
        }
        match entry.outcome {
            ProbeOutcomeV1::Accepted => {
                let seal = entry.cache.unwrap();
                let bytes = self.blobs.remove(&seal.blob_id).ok_or_else(|| {
                    CompilerProbeBackendErrorV1::static_internal("synthetic cache missing")
                })?;
                if let Some(graph) = entry.graph_transition {
                    if entry.frontend_coverage.is_some()
                        || entry.compiler_build_flags.is_some()
                        || entry.invoke_return.is_some()
                    {
                        return Err(CompilerProbeBackendErrorV1::static_internal(
                            "synthetic graph supplemental shape mismatch",
                        ));
                    }
                    let baseline = self
                        .blobs
                        .remove(&graph.baseline_cache.blob_id)
                        .ok_or_else(|| {
                            CompilerProbeBackendErrorV1::static_internal(
                                "synthetic graph baseline cache missing",
                            )
                        })?;
                    return Ok(OfflineCapturedProbeOutputV1::accepted_graph_transition(
                        entry.diagnostics,
                        baseline,
                        bytes,
                    ));
                }
                let supplemental = match (entry.frontend_coverage, entry.compiler_build_flags) {
                    (Some(value), None) => OfflineCapturedSupplementalWitnessV1::Frontend(value),
                    (None, Some(value)) => {
                        OfflineCapturedSupplementalWitnessV1::CompilerBuildFlags(value)
                    }
                    (None, None) => OfflineCapturedSupplementalWitnessV1::None,
                    _ => {
                        return Err(CompilerProbeBackendErrorV1::static_internal(
                            "synthetic supplemental shape mismatch",
                        ));
                    }
                };
                let invoke_return = entry
                    .invoke_return
                    .map(OfflineCanonicalInvokeReturnV1::into_observer_value);
                if matches!(supplemental, OfflineCapturedSupplementalWitnessV1::None) {
                    Ok(OfflineCapturedProbeOutputV1::accepted(
                        entry.diagnostics,
                        bytes,
                        invoke_return,
                    ))
                } else {
                    Ok(OfflineCapturedProbeOutputV1::accepted_with_supplemental(
                        entry.diagnostics,
                        bytes,
                        invoke_return,
                        supplemental,
                    ))
                }
            }
            ProbeOutcomeV1::Rejected => {
                Ok(OfflineCapturedProbeOutputV1::rejected(entry.diagnostics))
            }
        }
    }
}

#[test]
fn full_corpus_has_stable_seal_ids_sources_and_required_dimensions() {
    let first = full_qualification_corpus_v1().unwrap();
    let second = full_qualification_corpus_v1().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.cases.len(), 27);
    assert_eq!(first.canonical_sha256, second.canonical_sha256);
    assert_eq!(
        first.canonical_sha256,
        Sha256Digest::from_hex("01afb701c5f7ef7959a6d6c9b81f290f2e8d0b33671342ea50d9db87123daa94")
            .unwrap()
    );
    let ids: Vec<_> = first
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec![
            "positive.syntax.control-flow",
            "negative.syntax.missing-type",
            "positive.overloads.defaults",
            "negative.overloads.ambiguous",
            "positive.templates.containers",
            "positive.namespaces.imports",
            "negative.imports.missing-symbol",
            "positive.metadata.defaults",
            "positive.class-generator.editor-flags",
            "negative.class-generator.unsupported-required-property",
            "negative.metadata.invalid-specifier",
            "negative.types.assignment",
            "positive.invoke.scalar",
            "positive.invoke.structured",
            "positive.bytecode.fork-reference-lifecycle",
            "positive.bytecode.unresolved-object-property",
            "positive.model.globals-classes-all-tails",
            "negative.templates.validator",
            "positive.preprocessor.import-closure",
            "positive.game.dialog-diego-authoring",
            "positive.fname.non-ascii-equivalence",
            "positive.fname.name-none-canonical",
            "positive.strings.factory-roundtrip",
            "positive.frontend.hooks-editor-release",
            "positive.module-graph.change-delete",
            "negative.diagnostics.located-warning-as-error",
            "negative.unsupported.try-catch",
        ]
    );
    validate_full_qualification_coverage_v1(&first).unwrap();
    assert_eq!(FULL_QUALIFICATION_COVERAGE_V1.len(), 44);
    for requirement in FULL_QUALIFICATION_COVERAGE_V1 {
        let case = first
            .cases
            .iter()
            .find(|case| case.case_id == requirement.case_id)
            .unwrap();
        assert!(case.category.contains(requirement.requirement_id));
    }
    assert!(first
        .cases
        .iter()
        .any(|case| case.category.starts_with("requires_captured_profile/")));
    assert!(first
        .cases
        .iter()
        .any(|case| matches!(case.mode, ProbeModeV1::Invoke { .. })));
    assert!(first
        .cases
        .iter()
        .any(|case| matches!(case.mode, ProbeModeV1::CompileGraphTransition { .. })));
    assert!(first
        .cases
        .iter()
        .any(|case| case.expected_outcome == ProbeOutcomeV1::Rejected));
    for case in &first.cases {
        for section in &case.sections {
            assert_eq!(
                section.source_sha256,
                Sha256Digest::from_bytes(Sha256::digest(section.source_utf8.as_bytes()).into())
            );
        }
    }

    let mut path_identity_mismatch = first.clone();
    path_identity_mismatch.cases[17].sections[0].module = "ClosureProvider".into();
    assert!(path_identity_mismatch.seal().is_err());

    let mut resealed = first.clone();
    resealed.cases[0].category.push_str(";caller-added-label");
    resealed.seal().unwrap();
    validate_full_qualification_coverage_v1(&resealed).unwrap();
    assert!(matches!(
        validate_canonical_full_qualification_corpus_v1(&resealed),
        Err(OfflineQualificationErrorV1::CorpusMismatch)
    ));
    let resealed_package = package(&resealed, CompilerProbeBackendKindV1::Standalone);
    assert!(matches!(
        OfflineCompilerProbeArtifactBackendV1::load(
            &resealed,
            CompilerProbeBackendKindV1::Standalone,
            &resealed_package.0.to_json().unwrap(),
            resealed_package.1,
        ),
        Err(OfflineQualificationErrorV1::CorpusMismatch)
    ));
}

#[test]
fn qualification_source_sections_are_checkout_line_ending_independent() {
    let lf = section("Fixture", "Fixture.as", "int A;\nint B;\n");
    let crlf = section("Fixture", "Fixture.as", "int A;\r\nint B;\r\n");
    let cr = section("Fixture", "Fixture.as", "int A;\rint B;\r");

    assert_eq!(lf, crlf);
    assert_eq!(lf, cr);
}

#[test]
fn sealed_offline_artifacts_qualify_with_zero_unexplained_differences() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let embedded = package(&corpus, CompilerProbeBackendKindV1::EmbeddedGame);
    let standalone = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    let expected = expected_from_package(&corpus, &embedded.0, &embedded.1);
    let result = run(&corpus, &expected, &embedded, &standalone).unwrap();
    assert!(result.qualified());
    assert!(result.semantic_parity.unexplained_differences.is_empty());
    assert_eq!(
        result.semantic_parity.entries.len(),
        corpus
            .cases
            .iter()
            .filter(|case| case.expected_outcome == ProbeOutcomeV1::Accepted)
            .count()
    );
}

#[test]
fn capture_generator_and_promotion_gate_derive_authority_from_run_outputs() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let mut embedded_backend = SyntheticCaptureBackend::from_package(package(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
    ));
    let mut standalone_backend = SyntheticCaptureBackend::from_package(package(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
    ));
    let embedded = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        &mut embedded_backend,
    )
    .unwrap();
    let standalone = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut standalone_backend,
    )
    .unwrap();
    let promotion =
        promote_generated_offline_qualification_artifacts_v1(&corpus, &embedded, &standalone)
            .unwrap();
    assert!(promotion.qualified());
    assert_eq!(promotion.expected_results.results.len(), corpus.cases.len());
    assert_eq!(promotion.source_profile_sha256(), source_profile_sha256());
    assert_eq!(promotion.source_target(), &source_target());
    assert_eq!(promotion.standalone_compiler(), sidecar());
    let embedded_authority = embedded.authority_summary().unwrap();
    let standalone_authority = standalone.authority_summary().unwrap();
    assert_eq!(embedded_authority.standalone_compiler, None);
    assert_eq!(standalone_authority.standalone_compiler, Some(sidecar()));

    let mut incomplete = embedded.cache_blobs().clone();
    incomplete.pop_first();
    assert!(matches!(
        reload_generated_offline_qualification_artifacts_v1(
            &corpus,
            CompilerProbeBackendKindV1::EmbeddedGame,
            embedded.manifest_json(),
            incomplete,
        ),
        Err(OfflineQualificationErrorV1::MissingBlob(_))
    ));

    let mut caller_defined_suite = corpus.clone();
    caller_defined_suite.suite_id = "caller-defined-full-suite".into();
    caller_defined_suite.seal().unwrap();
    assert!(matches!(
        promote_generated_offline_qualification_artifacts_v1(
            &caller_defined_suite,
            &embedded,
            &standalone,
        ),
        Err(OfflineQualificationErrorV1::CorpusMismatch)
    ));

    let mut drifted_package = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    reseal_blob(
        &mut drifted_package.0,
        &mut drifted_package.1,
        "positive.syntax.control-flow",
        cache_with_modules(0x7777, &[("SyntaxPositive", 1)]),
    );
    let mut drifted_backend = SyntheticCaptureBackend::from_package(drifted_package);
    let drifted = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut drifted_backend,
    )
    .unwrap();
    assert!(matches!(
        promote_generated_offline_qualification_artifacts_v1(&corpus, &embedded, &drifted,),
        Err(OfflineQualificationErrorV1::PromotionRejected(_))
    ));

    let mut mismatched_package = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    mismatched_package.0.source_profile_sha256 = Sha256Digest::from_bytes([0x45; 32]);
    mismatched_package.0.seal().unwrap();
    let mut mismatched_backend = SyntheticCaptureBackend::from_package(mismatched_package);
    let mismatched = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut mismatched_backend,
    )
    .unwrap();
    assert!(matches!(
        promote_generated_offline_qualification_artifacts_v1(&corpus, &embedded, &mismatched,),
        Err(OfflineQualificationErrorV1::CorpusMismatch)
    ));

    let mut relabeling_backend = SyntheticCaptureBackend::from_package(package(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
    ));
    let mut relabelled_sidecar = sidecar();
    relabelled_sidecar.sha256 = Sha256Digest::from_bytes([0x6b; 32]);
    relabeling_backend.relabel_after_capture = Some(relabelled_sidecar);
    assert!(matches!(
        capture_and_seal_offline_qualification_artifacts_v1(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &mut relabeling_backend,
        ),
        Err(OfflineQualificationErrorV1::InvalidManifest(
            "capture backend authority changed during the sealed run"
        ))
    ));
}

#[test]
fn promotion_compares_frontend_exactly_and_graph_supplemental_artifacts_semantically() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let mut embedded_backend = SyntheticCaptureBackend::from_package(package(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
    ));
    let embedded = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        &mut embedded_backend,
    )
    .unwrap();

    let mut frontend_package = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    frontend_package
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.frontend.hooks-editor-release")
        .unwrap()
        .frontend_coverage
        .as_mut()
        .unwrap()
        .class_analyze_captures[0]
        .subject_identity = "UQualificationDifferentHook".into();
    frontend_package.0.seal().unwrap();
    let mut frontend_backend = SyntheticCaptureBackend::from_package(frontend_package);
    let frontend = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut frontend_backend,
    )
    .unwrap();
    assert!(matches!(
        promote_generated_offline_qualification_artifacts_v1(
            &corpus,
            &embedded,
            &frontend,
        ),
        Err(OfflineQualificationErrorV1::PromotionRejected(differences))
            if differences == vec!["positive.frontend.hooks-editor-release:supplemental-artifacts:standalone"]
    ));

    let mut guid_package = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    let guid_entry = guid_package
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.module-graph.change-delete")
        .unwrap();
    let guid_baseline_blob_id = guid_entry
        .graph_transition
        .as_ref()
        .unwrap()
        .baseline_cache
        .blob_id
        .clone();
    let guid_baseline_bytes = guid_package.1.get_mut(&guid_baseline_blob_id).unwrap();
    guid_baseline_bytes[0] ^= 0x5a;
    guid_entry.graph_transition.as_mut().unwrap().baseline_cache =
        seal_blob(&guid_baseline_blob_id, guid_baseline_bytes);
    guid_package.0.seal().unwrap();
    let mut guid_backend = SyntheticCaptureBackend::from_package(guid_package);
    let guid_only = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut guid_backend,
    )
    .unwrap();
    assert!(
        promote_generated_offline_qualification_artifacts_v1(&corpus, &embedded, &guid_only,)
            .is_ok()
    );

    let mut graph_package = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    let graph_entry = graph_package
        .0
        .entries
        .iter()
        .find(|entry| entry.case_id == "positive.module-graph.change-delete")
        .unwrap();
    let graph_blob_id = graph_entry.cache.as_ref().unwrap().blob_id.clone();
    let baseline_blob_id = graph_entry
        .graph_transition
        .as_ref()
        .unwrap()
        .baseline_cache
        .blob_id
        .clone();
    let build_identifier = 1000 + graph_entry.ordinal as i32;
    graph_package.1.insert(
        graph_blob_id,
        cache_with_modules(
            build_identifier,
            &[
                ("Graph.ChangedModule", 2),
                ("Graph.AddedModule", 1),
                ("RetainedBaseModule", 2),
            ],
        ),
    );
    graph_package.1.insert(
        baseline_blob_id,
        cache_with_modules(
            build_identifier,
            &[
                ("Graph.ChangedModule", 1),
                ("Graph.DeletedModule", 1),
                ("RetainedBaseModule", 2),
            ],
        ),
    );
    let mut graph_backend = SyntheticCaptureBackend::from_package(graph_package);
    let graph = capture_and_seal_offline_qualification_artifacts_v1(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &mut graph_backend,
    )
    .unwrap();
    assert!(matches!(
        promote_generated_offline_qualification_artifacts_v1(
            &corpus,
            &embedded,
            &graph,
        ),
        Err(OfflineQualificationErrorV1::PromotionRejected(differences))
            if differences == vec!["positive.module-graph.change-delete:supplemental-artifacts:standalone"]
    ));
}

#[test]
fn cache_diagnostic_and_return_mutations_are_isolated_dimensions() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let embedded = package(&corpus, CompilerProbeBackendKindV1::EmbeddedGame);
    let expected = expected_from_package(&corpus, &embedded.0, &embedded.1);

    let mut cache_drift = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    reseal_blob(
        &mut cache_drift.0,
        &mut cache_drift.1,
        "positive.syntax.control-flow",
        cache_with_modules(9999, &[("SyntaxPositive", 1)]),
    );
    assert_eq!(
        run(&corpus, &expected, &embedded, &cache_drift)
            .unwrap()
            .semantic_parity
            .unexplained_differences,
        vec!["positive.syntax.control-flow:semantics:standalone"]
    );

    let mut diagnostic_drift = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    diagnostic_drift
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "negative.syntax.missing-type")
        .unwrap()
        .diagnostics[0]
        .message
        .push_str(" changed");
    diagnostic_drift.0.seal().unwrap();
    assert_eq!(
        run(&corpus, &expected, &embedded, &diagnostic_drift)
            .unwrap()
            .semantic_parity
            .unexplained_differences,
        vec!["negative.syntax.missing-type:diagnostics:standalone"]
    );

    let mut return_drift = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    return_drift
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.invoke.scalar")
        .unwrap()
        .invoke_return
        .as_mut()
        .unwrap()
        .value = OfflineCanonicalInvokeValueV1::I64(43);
    reseal_entry_semantics(
        &mut return_drift.0,
        &return_drift.1,
        "positive.invoke.scalar",
    );
    assert_eq!(
        run(&corpus, &expected, &embedded, &return_drift)
            .unwrap()
            .semantic_parity
            .unexplained_differences,
        vec!["positive.invoke.scalar:semantics:standalone"]
    );
}

#[test]
fn every_observer_model_group_mutation_reaches_differential_parity() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let embedded = package(&corpus, CompilerProbeBackendKindV1::EmbeddedGame);
    let expected = expected_from_package(&corpus, &embedded.0, &embedded.1);
    let fixture = synthetic_observer_qualification_fixture_v1();

    for (dimension, bytes) in fixture.semantic_mutations {
        let mut standalone = package(&corpus, CompilerProbeBackendKindV1::Standalone);
        reseal_blob(
            &mut standalone.0,
            &mut standalone.1,
            "positive.model.globals-classes-all-tails",
            bytes,
        );
        assert_eq!(
            run(&corpus, &expected, &embedded, &standalone)
                .unwrap_or_else(|error| panic!("{dimension}: {error}"))
                .semantic_parity
                .unexplained_differences,
            vec!["positive.model.globals-classes-all-tails:semantics:standalone"],
            "{dimension}"
        );
    }

    for (dimension, bytes) in [
        (
            "unresolved_runtime_reference",
            fixture.unresolved_runtime_reference,
        ),
        (
            "legacy_bytecode_references",
            fixture.legacy_bytecode_references,
        ),
    ] {
        let mut standalone = package(&corpus, CompilerProbeBackendKindV1::Standalone);
        reseal_blob(
            &mut standalone.0,
            &mut standalone.1,
            "positive.model.globals-classes-all-tails",
            bytes,
        );
        assert!(
            matches!(
                run(&corpus, &expected, &embedded, &standalone),
                Err(OfflineQualificationErrorV1::SemanticObservation { .. })
            ),
            "{dimension}"
        );
    }
}

#[test]
fn coverage_labels_without_actual_witnesses_never_qualify() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let embedded = package(&corpus, CompilerProbeBackendKindV1::EmbeddedGame);
    let expected = expected_from_package(&corpus, &embedded.0, &embedded.1);

    for case_id in [
        "positive.bytecode.fork-reference-lifecycle",
        "positive.bytecode.unresolved-object-property",
        "positive.model.globals-classes-all-tails",
        "positive.strings.factory-roundtrip",
    ] {
        let mut standalone = package(&corpus, CompilerProbeBackendKindV1::Standalone);
        reseal_blob(&mut standalone.0, &mut standalone.1, case_id, cache(7000));
        assert!(matches!(
            run(&corpus, &expected, &embedded, &standalone),
            Err(OfflineQualificationErrorV1::InvalidManifest(_))
        ));
    }

    let mut false_fname = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    false_fname
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.fname.non-ascii-equivalence")
        .unwrap()
        .invoke_return
        .as_mut()
        .unwrap()
        .value = OfflineCanonicalInvokeValueV1::Bool(false);
    reseal_entry_semantics(
        &mut false_fname.0,
        &false_fname.1,
        "positive.fname.non-ascii-equivalence",
    );
    assert!(matches!(
        run(&corpus, &expected, &embedded, &false_fname),
        Err(OfflineQualificationErrorV1::InvalidManifest(_))
    ));

    let mut forged_semantics = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    forged_semantics
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.syntax.control-flow")
        .unwrap()
        .cache_semantics
        .as_mut()
        .unwrap()
        .global_count += 1;
    forged_semantics.0.seal().unwrap();
    assert!(matches!(
        run(&corpus, &expected, &embedded, &forged_semantics),
        Err(OfflineQualificationErrorV1::SemanticWitnessMismatch(_))
    ));

    let mut forged_graph_binding = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    forged_graph_binding
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.frontend.hooks-editor-release")
        .unwrap()
        .frontend_coverage
        .as_mut()
        .unwrap()
        .post_process_code_bound = true;
    assert!(matches!(
        forged_graph_binding.0.seal(),
        Err(OfflineQualificationErrorV1::InvalidManifest(_))
    ));

    let mut missing_graph_add = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    missing_graph_add
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.module-graph.change-delete")
        .unwrap()
        .graph_transition
        .as_mut()
        .unwrap()
        .added_modules
        .clear();
    assert!(matches!(
        missing_graph_add.0.seal(),
        Err(OfflineQualificationErrorV1::InvalidManifest(_))
    ));

    let mut wrong_callback_state = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    wrong_callback_state
        .0
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.bytecode.unresolved-object-property")
        .unwrap()
        .compiler_build_flags
        .as_mut()
        .unwrap()
        .resolve_object_ptr_callback_registered = true;
    wrong_callback_state.0.seal().unwrap();
    assert!(matches!(
        run(&corpus, &expected, &embedded, &wrong_callback_state),
        Err(OfflineQualificationErrorV1::InvalidManifest(_))
    ));

    let mut unlisted_base_mutation = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    let graph_entry = unlisted_base_mutation
        .0
        .entries
        .iter()
        .find(|entry| entry.case_id == "positive.module-graph.change-delete")
        .unwrap();
    let graph_blob_id = graph_entry.cache.as_ref().unwrap().blob_id.clone();
    let build_identifier = 1000 + graph_entry.ordinal as i32;
    unlisted_base_mutation.1.insert(
        graph_blob_id,
        cache_with_modules(
            build_identifier,
            &[
                ("ChangedModule", 2),
                ("AddedModule", 1),
                ("RetainedBaseModule", 99),
            ],
        ),
    );
    let mut backend = SyntheticCaptureBackend::from_package(unlisted_base_mutation);
    assert!(matches!(
        capture_and_seal_offline_qualification_artifacts_v1(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &mut backend,
        ),
        Err(OfflineQualificationErrorV1::InvalidManifest(_))
    ));
}

#[test]
fn alignment_observer_contract_and_blob_set_fail_closed() {
    let corpus = full_qualification_corpus_v1().unwrap();
    let embedded = package(&corpus, CompilerProbeBackendKindV1::EmbeddedGame);
    let expected = expected_from_package(&corpus, &embedded.0, &embedded.1);

    let mut alignment = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    let mut malformed = cache(1000);
    malformed.push(0);
    reseal_blob(
        &mut alignment.0,
        &mut alignment.1,
        "positive.syntax.control-flow",
        malformed,
    );
    assert!(matches!(
        run(&corpus, &expected, &embedded, &alignment),
        Err(OfflineQualificationErrorV1::SemanticObservation { .. })
    ));

    let valid = package(&corpus, CompilerProbeBackendKindV1::Standalone);
    let mut wrong_contract = valid.0.clone();
    wrong_contract.semantic_observer = "caller-selected-digest/v0".into();
    assert!(matches!(
        wrong_contract.seal(),
        Err(OfflineQualificationErrorV1::ObserverContract)
    ));

    let manifest_json = valid.0.to_json().unwrap();
    let mut tampered = valid.1.clone();
    tampered
        .get_mut("positive.syntax.control-flow.cache")
        .unwrap()[16] ^= 1;
    assert!(matches!(
        OfflineCompilerProbeArtifactBackendV1::load(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &manifest_json,
            tampered,
        ),
        Err(OfflineQualificationErrorV1::BlobSealMismatch(_))
    ));
    let graph_baseline_blob_id = valid
        .0
        .entries
        .iter()
        .find(|entry| entry.case_id == "positive.module-graph.change-delete")
        .unwrap()
        .graph_transition
        .as_ref()
        .unwrap()
        .baseline_cache
        .blob_id
        .clone();
    let mut tampered_baseline = valid.1.clone();
    tampered_baseline.get_mut(&graph_baseline_blob_id).unwrap()[16] ^= 1;
    assert!(matches!(
        OfflineCompilerProbeArtifactBackendV1::load(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &manifest_json,
            tampered_baseline,
        ),
        Err(OfflineQualificationErrorV1::BlobSealMismatch(_))
    ));
    let mut missing = valid.1.clone();
    missing.pop_first();
    assert!(matches!(
        OfflineCompilerProbeArtifactBackendV1::load(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &manifest_json,
            missing,
        ),
        Err(OfflineQualificationErrorV1::MissingBlob(_))
    ));
    let valid_blobs = valid.1.clone();
    let mut extra = valid.1;
    extra.insert("undeclared.cache".into(), cache(1));
    assert!(matches!(
        OfflineCompilerProbeArtifactBackendV1::load(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &manifest_json,
            extra,
        ),
        Err(OfflineQualificationErrorV1::UnexpectedBlob(_))
    ));

    let mut missing_return = valid.0;
    missing_return
        .entries
        .iter_mut()
        .find(|entry| entry.case_id == "positive.invoke.scalar")
        .unwrap()
        .invoke_return = None;
    missing_return.seal().unwrap();
    assert!(matches!(
        OfflineCompilerProbeArtifactBackendV1::load(
            &corpus,
            CompilerProbeBackendKindV1::Standalone,
            &missing_return.to_json().unwrap(),
            valid_blobs,
        ),
        Err(OfflineQualificationErrorV1::InvalidManifest(
            "accepted invoke artifact requires a typed return"
        ))
    ));
}
