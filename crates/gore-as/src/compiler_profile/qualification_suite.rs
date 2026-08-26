//! Versioned full qualification corpus and sealed offline-artifact adapter.
//!
//! Nothing in this module starts a process. Game and standalone results must already exist as
//! exact cache bytes plus structured diagnostics/return values. The loader authenticates those
//! bytes against a deterministic manifest and exposes them through the same closed backend API as
//! the differential runner; accepted semantics are still recomputed by the whole-cache observer.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::manifest::{CompilerTargetV1, Sha256Digest};
use super::qualification::{
    CompilerProbeCaseV1, CompilerProbeCorpusV1, ExpectedDiagnosticV1, ExpectedProbeResultV1,
    ExpectedProbeResultsV1, ProbeModeV1, ProbeOutcomeV1, ProbeSourceSectionV1, QualificationError,
    QualifiedSidecarIdentityV1, EXPECTED_RESULTS_SCHEMA, PROBE_CORPUS_SCHEMA,
    QUALIFICATION_SCHEMA_VERSION,
};
use super::qualification_runner::{
    run_differential_qualification_v1, CompilerProbeBackendErrorV1, CompilerProbeBackendKindV1,
    CompilerProbeBackendV1, CompilerProbeObservationV1, DifferentialQualificationRunV1,
    QualificationRunnerErrorV1, SEMANTIC_OBSERVER_CONTRACT_V1,
};
use crate::cache::semantic_observer::{
    observe_whole_cache_semantics_v1, CanonicalInvokeReturnV1, CanonicalInvokeValueV1,
    WholeCacheSemanticObservationV1,
};

pub const FULL_QUALIFICATION_SUITE_ID_V1: &str = "gore.as.full-differential-qualification/v2";
pub const OFFLINE_PROBE_ARTIFACT_SCHEMA_V1: &str = "gore.as.offline-probe-artifacts";
pub const OFFLINE_PROBE_ARTIFACT_SCHEMA_VERSION_V1: u32 = 1;

const OFFLINE_ARTIFACT_HASH_DOMAIN_V1: &[u8] = b"gore-as-offline-probe-artifacts-v1\0";
const OFFLINE_CACHE_SEAL_AUTHORITY_DOMAIN_V1: &[u8] = b"gore-as-offline-cache-seal-authority-v1\0";
const OFFLINE_SUPPLEMENTAL_AUTHORITY_DOMAIN_V1: &[u8] =
    b"gore-as-offline-supplemental-authority-v1\0";
const MAX_OFFLINE_ARTIFACT_JSON_BYTES_V1: usize = 32 * 1024 * 1024;
const MAX_ARTIFACT_BLOB_ID_BYTES_V1: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationCoverageDispositionV1 {
    ConcreteSource,
    RequiresCapturedProfile,
    ObserverFailClosedMutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualificationCoverageRequirementV1 {
    pub requirement_id: &'static str,
    pub case_id: &'static str,
    pub disposition: QualificationCoverageDispositionV1,
}

/// Closed coverage ledger. Requirement ids are corpus-sealed documentation and routing metadata;
/// they never grant qualification. Artifact acceptance is separately gated by recomputed semantic
/// witnesses and the strict case-specific witness checks below.
pub const FULL_QUALIFICATION_COVERAGE_V1: &[QualificationCoverageRequirementV1] = &[
    requirement(
        "compiler-emitted-fork-opcodes",
        "positive.bytecode.fork-reference-lifecycle",
        true,
    ),
    requirement(
        "reference-debug-opcodes-disabled",
        "positive.bytecode.fork-reference-lifecycle",
        true,
    ),
    requirement(
        "noncompiler-fork-opcodes-absent",
        "positive.bytecode.fork-reference-lifecycle",
        true,
    ),
    requirement(
        "profiled-unresolved-object-resolve-policy",
        "positive.bytecode.unresolved-object-property",
        true,
    ),
    requirement(
        "handles-refs-out-inout",
        "positive.bytecode.fork-reference-lifecycle",
        true,
    ),
    requirement(
        "locals-stack-refs",
        "positive.bytecode.fork-reference-lifecycle",
        true,
    ),
    requirement(
        "globals-initfunc",
        "positive.model.globals-classes-all-tails",
        true,
    ),
    requirement(
        "classes-inheritance-interfaces-behaviours-properties-accessors",
        "positive.model.globals-classes-all-tails",
        true,
    ),
    requirement("tail-t1", "positive.model.globals-classes-all-tails", true),
    requirement("tail-t2", "positive.model.globals-classes-all-tails", true),
    requirement("tail-t3", "positive.model.globals-classes-all-tails", true),
    requirement("tail-t4", "positive.model.globals-classes-all-tails", true),
    requirement("tail-t5", "positive.model.globals-classes-all-tails", true),
    requirement("tail-t6", "positive.model.globals-classes-all-tails", true),
    requirement("tail-t7", "positive.model.globals-classes-all-tails", true),
    requirement(
        "primitive-object-default-arguments",
        "positive.model.globals-classes-all-tails",
        true,
    ),
    requirement(
        "primitive-default-arguments",
        "positive.overloads.defaults",
        false,
    ),
    requirement(
        "template-validator-positive",
        "positive.templates.containers",
        true,
    ),
    requirement(
        "template-validator-negative",
        "negative.templates.validator",
        true,
    ),
    requirement(
        "preprocessor-flags-automatic-import-closure",
        "positive.preprocessor.import-closure",
        true,
    ),
    requirement(
        "game-dialog-diego-authoring",
        "positive.game.dialog-diego-authoring",
        true,
    ),
    requirement(
        "non-ascii-fname-equivalence",
        "positive.fname.non-ascii-equivalence",
        true,
    ),
    requirement(
        "fname-name-none-canonicalization",
        "positive.fname.name-none-canonical",
        true,
    ),
    requirement(
        "utf8-string-factory-global-roundtrip",
        "positive.strings.factory-roundtrip",
        true,
    ),
    requirement(
        "class-generator-editor-flags",
        "positive.class-generator.editor-flags",
        true,
    ),
    requirement(
        "class-generator-implicit-struct-nontransient-policy",
        "positive.class-generator.editor-flags",
        true,
    ),
    requirement(
        "class-generator-struct-non-never-gc-property",
        "positive.class-generator.editor-flags",
        true,
    ),
    requirement(
        "class-generator-class-nonrequired-property-omission",
        "positive.class-generator.editor-flags",
        true,
    ),
    requirement(
        "class-generator-selected-property-can-create-rejection",
        "negative.class-generator.unsupported-required-property",
        true,
    ),
    requirement(
        "class-analyze-target-state",
        "positive.frontend.hooks-editor-release",
        true,
    ),
    requirement(
        "process-chunks-target-unbound",
        "positive.frontend.hooks-editor-release",
        true,
    ),
    requirement(
        "post-process-target-unbound",
        "positive.frontend.hooks-editor-release",
        true,
    ),
    requirement(
        "editor-release-discovery",
        "positive.frontend.hooks-editor-release",
        true,
    ),
    requirement(
        "fname-comparison-capture-replay",
        "positive.frontend.hooks-editor-release",
        true,
    ),
    requirement(
        "changed-module-graph",
        "positive.module-graph.change-delete",
        false,
    ),
    requirement(
        "deleted-module-graph",
        "positive.module-graph.change-delete",
        false,
    ),
    requirement(
        "added-module-graph",
        "positive.module-graph.change-delete",
        false,
    ),
    requirement(
        "located-warning",
        "negative.diagnostics.located-warning-as-error",
        true,
    ),
    requirement(
        "warnings-as-errors",
        "negative.diagnostics.located-warning-as-error",
        true,
    ),
    requirement("located-info", "negative.overloads.ambiguous", false),
    requirement("located-error", "negative.unsupported.try-catch", false),
    requirement(
        "unsupported-try-catch",
        "negative.unsupported.try-catch",
        false,
    ),
    QualificationCoverageRequirementV1 {
        requirement_id: "unresolved-runtime-id-fail-closed",
        case_id: "negative.unsupported.try-catch",
        disposition: QualificationCoverageDispositionV1::ObserverFailClosedMutation,
    },
    QualificationCoverageRequirementV1 {
        requirement_id: "legacy-bytecode-references-fail-closed",
        case_id: "negative.unsupported.try-catch",
        disposition: QualificationCoverageDispositionV1::ObserverFailClosedMutation,
    },
];

const fn requirement(
    requirement_id: &'static str,
    case_id: &'static str,
    captured: bool,
) -> QualificationCoverageRequirementV1 {
    QualificationCoverageRequirementV1 {
        requirement_id,
        case_id,
        disposition: if captured {
            QualificationCoverageDispositionV1::RequiresCapturedProfile
        } else {
            QualificationCoverageDispositionV1::ConcreteSource
        },
    }
}

pub fn validate_full_qualification_coverage_v1(
    corpus: &CompilerProbeCorpusV1,
) -> Result<(), QualificationError> {
    for requirement in FULL_QUALIFICATION_COVERAGE_V1 {
        let Some(case) = corpus
            .cases
            .iter()
            .find(|case| case.case_id == requirement.case_id)
        else {
            return Err(QualificationError::InvalidField {
                field: "full qualification coverage",
                reason: "required case is absent",
            });
        };
        if !case.category.contains(requirement.requirement_id) {
            return Err(QualificationError::InvalidField {
                field: "full qualification coverage",
                reason: "requirement id is not bound into the case category",
            });
        }
    }
    Ok(())
}

/// Build the immutable ordered qualification corpus.
///
/// Categories prefixed with `requires_captured_profile/` are mandatory coverage whose concrete
/// registrations or frontend metadata come from the captured product profile. They are never
/// silently omitted by hosts that do not yet have that profile.
pub fn full_qualification_corpus_v1() -> Result<CompilerProbeCorpusV1, QualificationError> {
    let cases = vec![
        case(
            "positive.syntax.control-flow",
            "syntax",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "SyntaxPositive",
                "SyntaxPositive.as",
                "int Clamp(int v, int lo, int hi) { if (v < lo) return lo; if (v > hi) return hi; return v; }",
            )],
        ),
        case(
            "negative.syntax.missing-type",
            "diagnostics/syntax",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "SyntaxNegative",
                "SyntaxNegative.as",
                "void Broken( {",
            )],
        ),
        case(
            "positive.overloads.defaults",
            "overloads-defaults;coverage=primitive-default-arguments",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "Overloads",
                "Overloads.as",
                "int Pick(int value, int bias = 2) { return value + bias; }\nfloat Pick(float value, float bias = 0.5f) { return value + bias; }\nint UsePick() { return Pick(40); }",
            )],
        ),
        case(
            "negative.overloads.ambiguous",
            "diagnostics/overloads;coverage=located-info",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "Ambiguous",
                "Ambiguous.as",
                "int Choose(int a, float b) { return 1; }\nint Choose(float a, int b) { return 2; }\nint Broken() { return Choose(1, 1); }",
            )],
        ),
        case(
            "positive.templates.containers",
            "requires_captured_profile/templates-containers;coverage=template-validator-positive",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "Containers",
                "Containers.as",
                "int SumValues() { TArray<int32> Values; Values.SetNum(2); return 42; }",
            )],
        ),
        case(
            "positive.namespaces.imports",
            "namespaces-imports;coverage=namespaces-function-imports",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[
                section(
                    "Provider",
                    "Provider.as",
                    "namespace Qual { int Value() { return 42; } }",
                ),
                section(
                    "Consumer",
                    "Consumer.as",
                    "namespace Qual { import int Value() from \"Provider\"; }\nint ReadImported() { return Qual::Value(); }",
                ),
            ],
        ),
        case(
            "negative.imports.missing-symbol",
            "diagnostics/imports",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "MissingImport",
                "MissingImport.as",
                "import int DoesNotExist() from \"MissingProvider\";\nint ReadMissing() { return DoesNotExist(); }",
            )],
        ),
        case(
            "positive.metadata.defaults",
            "requires_captured_profile/metadata-defaults",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "MetadataDefaults",
                "MetadataDefaults.as",
                "UFUNCTION(BlueprintCallable)\nint32 MetadataDefault(int32 value = 42) { return value; }",
            )],
        ),
        case(
            "positive.class-generator.editor-flags",
            "requires_captured_profile/class-generator-editor-flags;coverage=class-generator-editor-flags,class-generator-implicit-struct-nontransient-policy,class-generator-struct-non-never-gc-property,class-generator-class-nonrequired-property-omission",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "GeneratedClass",
                "GeneratedClass.as",
                "UCLASS()\nclass UQualificationReferenceTarget : UObject {}\nUCLASS()\nclass UQualificationObject : UObject {\n  UPROPERTY(EditAnywhere, BlueprintReadWrite)\n  int32 Value = 42;\n  UQualificationReferenceTarget ImplicitObject;\n  int32 ImplicitScalar = 7;\n  UFUNCTION(BlueprintCallable)\n  int32 Read() const { return Value; }\n}\nUSTRUCT()\nstruct FQualificationStruct {\n  UObject ImplicitObject;\n  int32 ImplicitScalar = 9;\n}",
            )],
        ),
        case(
            "negative.class-generator.unsupported-required-property",
            "requires_captured_profile/class-generator-selected-property-rejection;coverage=class-generator-selected-property-can-create-rejection",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "UnsupportedGeneratedProperty",
                "UnsupportedGeneratedProperty.as",
                "USTRUCT()\nstruct FQualificationUnsupportedProperty {\n  FNumberFormattingOptions Value;\n}",
            )],
        ),
        case(
            "negative.metadata.invalid-specifier",
            "requires_captured_profile/diagnostics-metadata",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "MetadataInvalid",
                "MetadataInvalid.as",
                "UFUNCTION(NotARealSpecifier)\nvoid InvalidMetadata() {}",
            )],
        ),
        case(
            "negative.types.assignment",
            "diagnostics/types",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "TypeMismatch",
                "TypeMismatch.as",
                "int BrokenAssignment() { string value = 42; return value; }",
            )],
        ),
        case(
            "positive.invoke.scalar",
            "invoke",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::Invoke {
                declaration: "int QualificationInvoke()".into(),
            },
            &[section(
                "InvokeScalar",
                "InvokeScalar.as",
                "int QualificationInvoke() { return 42; }",
            )],
        ),
        case(
            "positive.invoke.structured",
            "requires_captured_profile/invoke-container",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::Invoke {
                declaration: "TArray<int32> QualificationInvokeArray()".into(),
            },
            &[section(
                "InvokeStructured",
                "InvokeStructured.as",
                "TArray<int32> QualificationInvokeArray() { TArray<int32> Values; Values.SetNum(2); return Values; }",
            )],
        ),
        case(
            "positive.bytecode.fork-reference-lifecycle",
            "requires_captured_profile/bytecode-reference-lifecycle;coverage=compiler-emitted-fork-opcodes,reference-debug-opcodes-disabled,noncompiler-fork-opcodes-absent,handles-refs-out-inout,locals-stack-refs",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "ForkBytecode",
                "ForkBytecode.as",
                "enum EQualificationBranch { First, Second }\nclass FTrackedValue { int32 Value; FTrackedValue() { Value = 1; } FTrackedValue(const FTrackedValue &in Other) { Value = Other.Value; } }\nstruct FTrackedPayload { TArray<int32> Values; }\nclass FTrackedOwner { FTrackedPayload Stored; FTrackedOwner() { FTrackedPayload Local; Stored = Local; } }\nvoid Mutate(FTrackedValue &inout Handle, int32 &out Result) { FTrackedValue Local = FTrackedValue(); FTrackedValue Copy = Local; if (Handle != nullptr) Copy.Value += Handle.Value; Handle = Copy; Result = Copy.Value; }\nUObject ChooseTrackedObject(bool ReturnNull, UObject Value) { bool SelectedNull = ReturnNull; return SelectedNull ? nullptr : Value; }\nint32 ExhaustiveBranch(EQualificationBranch Value) { switch (Value) { case EQualificationBranch::First: return 1; case EQualificationBranch::Second: return 2; } }",
            )],
        ),
        case(
            "positive.bytecode.unresolved-object-property",
            "requires_captured_profile/unresolved-object-property;coverage=profiled-unresolved-object-resolve-policy",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "UnresolvedObjectProperty",
                "UnresolvedObjectProperty.as",
                "USceneComponent ResolveProfiledUnresolvedProperty(AActor Actor) { return Actor.RootComponent; }",
            )],
        ),
        case(
            "positive.model.globals-classes-all-tails",
            "requires_captured_profile/complete-cache-model;coverage=globals-initfunc,classes-inheritance-interfaces-behaviours-properties-accessors,tail-t1,tail-t2,tail-t3,tail-t4,tail-t5,tail-t6,tail-t7,primitive-object-default-arguments",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "CompleteModel",
                "CompleteModel.as",
                "int32 MakeGlobal() { return 42; }\nconst int32 QualificationGlobal = MakeGlobal();\nFName QualificationModelStaticName() { return n\"CompleteModelName\"; }\nUCLASS()\nclass UQualificationBase : UObject {\n  UPROPERTY(EditAnywhere, BlueprintReadWrite)\n  int32 Value = 7;\n  UQualificationBase() { Value = 8; }\n  ~UQualificationBase() { Value = 0; }\n  int32 GetAccessor() const property { return Value; }\n  void SetAccessor(int32 InValue) property { Value = InValue; }\n}\nclass UQualificationDerived : UQualificationBase {\n  UQualificationDerived() { Value = 42; }\n  int32 Read() const { return Accessor; }\n}\nint32 ReadObject(UQualificationBase Object = nullptr) { return Object == nullptr ? QualificationGlobal : Object.Value; }",
            )],
        ),
        case(
            "negative.templates.validator",
            "requires_captured_profile/template-validator;coverage=template-validator-negative",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "TemplateInvalid",
                "TemplateInvalid.as",
                "TSubclassOf<int32> InvalidTemplateType;",
            )],
        ),
        case(
            "positive.preprocessor.import-closure",
            "requires_captured_profile/preprocessor-automatic-import-closure;coverage=preprocessor-flags-automatic-import-closure",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[
                section(
                    "Graph.ClosureProvider",
                    "Graph/ClosureProvider.as",
                    "int32 ClosureValue() { return 42; }",
                ),
                section(
                    "Graph.ClosureMiddle",
                    "Graph/ClosureMiddle.as",
                    "int32 MiddleValue() { return ClosureValue(); }",
                ),
                section(
                    "Graph.ClosureConsumer",
                    "Graph/ClosureConsumer.as",
                    "#if RELEASE\nint32 ConsumerValue() { return MiddleValue(); }\n#endif",
                ),
            ],
        ),
        case(
            "positive.game.dialog-diego-authoring",
            "requires_captured_profile/game-dialog-diego-authoring;coverage=game-dialog-diego-authoring",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "Gore.Examples.DiegoDialogSmoke",
                "Gore/Examples/DiegoDialogSmoke.as",
                include_str!("../../tests/fixtures/diego_dialog_smoke.as"),
            )],
        ),
        case(
            "positive.fname.non-ascii-equivalence",
            "requires_captured_profile/fname-unicode;coverage=non-ascii-fname-equivalence",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::Invoke {
                declaration: "bool QualificationFNameEquivalent()".into(),
            },
            &[section(
                "FNameUnicode",
                "FNameUnicode.as",
                "bool QualificationFNameEquivalent() { return n\"Äquivalent\" == n\"ÄQUIVALENT\"; }",
            )],
        ),
        case(
            "positive.fname.name-none-canonical",
            "requires_captured_profile/fname-name-none;coverage=fname-name-none-canonicalization",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::Invoke {
                declaration: "bool QualificationFNameNoneCanonical()".into(),
            },
            &[section(
                "FNameNone",
                "FNameNone.as",
                "bool QualificationFNameNoneCanonical() { return n\"\" == n\"None\" && n\"nOnE\" == n\"None\"; }",
            )],
        ),
        case(
            "positive.strings.factory-roundtrip",
            "requires_captured_profile/string-factory;coverage=utf8-string-factory-global-roundtrip",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::Invoke {
                declaration: "FString QualificationStringRoundtrip()".into(),
            },
            &[section(
                "StringFactoryUtf8",
                "StringFactoryUtf8.as",
                "FString QualificationStringRoundtrip() { FString Local = \"Grüße_日本\"; return Local; }",
            )],
        ),
        case(
            "positive.frontend.hooks-editor-release",
            "requires_captured_profile/frontend-hooks-editor-release;coverage=class-analyze-target-state,process-chunks-target-unbound,post-process-target-unbound,editor-release-discovery,fname-comparison-capture-replay",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileOnly,
            &[section(
                "FrontendHooks",
                "FrontendHooks.as",
                "#if EDITOR\nint32 QualificationEditorDiscovery() { return 1; }\n#endif\nFName QualificationFrontendFNameUpper() { return n\"Äquivalent\"; }\nFName QualificationFrontendFNameLower() { return n\"äQUIVALENT\"; }\nUCLASS()\nclass UQualificationFrontendHook : UObject { UFUNCTION(BlueprintCallable) void RunFrontendHook() {} }\n#if RELEASE\nint32 QualificationReleaseDiscovery() { return 42; }\n#endif",
            )],
        ),
        case(
            "positive.module-graph.change-delete",
            "module-graph-transition;coverage=changed-module-graph,deleted-module-graph,added-module-graph",
            ProbeOutcomeV1::Accepted,
            ProbeModeV1::CompileGraphTransition {
                baseline_sections: vec![
                    section(
                        "Graph.ChangedModule",
                        "Graph/ChangedModule.as",
                        "int32 ChangedValue() { return 1; }",
                    ),
                    {
                        let mut value = section(
                            "Graph.DeletedModule",
                            "Graph/DeletedModule.as",
                            "int32 DeletedValue() { return 2; }",
                        );
                        value.ordinal = 1;
                        value
                    },
                ],
                changed_modules: vec!["Graph.ChangedModule".into()],
                deleted_modules: vec!["Graph.DeletedModule".into()],
            },
            &[
                section(
                    "Graph.ChangedModule",
                    "Graph/ChangedModule.as",
                    "int32 ChangedValue() { return 42; }",
                ),
                section(
                    "Graph.AddedModule",
                    "Graph/AddedModule.as",
                    "int32 AddedValue() { return ChangedValue(); }",
                ),
            ],
        ),
        case(
            "negative.diagnostics.located-warning-as-error",
            "requires_captured_profile/compiler-diagnostics;coverage=located-warning,warnings-as-errors",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "LocatedDiagnostics",
                "LocatedDiagnostics.as",
                "int32 LocatedDiagnostics() { int32 Value = 1; Value = Value; return Value; }",
            )],
        ),
        case(
            "negative.unsupported.try-catch",
            "diagnostics/unsupported;coverage=unsupported-try-catch,located-error,unresolved-runtime-id-fail-closed,legacy-bytecode-references-fail-closed",
            ProbeOutcomeV1::Rejected,
            ProbeModeV1::CompileOnly,
            &[section(
                "UnsupportedTryCatch",
                "UnsupportedTryCatch.as",
                "void Unsupported() { try { throw(\"no\"); } catch { } }",
            )],
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(ordinal, mut value)| {
        value.ordinal = ordinal as u32;
        value
    })
    .collect();

    let mut corpus = CompilerProbeCorpusV1 {
        schema: PROBE_CORPUS_SCHEMA.into(),
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        suite_id: FULL_QUALIFICATION_SUITE_ID_V1.into(),
        cases,
        canonical_sha256: zero_digest(),
    };
    corpus.seal()?;
    validate_full_qualification_coverage_v1(&corpus)?;
    Ok(corpus)
}

pub(crate) fn validate_canonical_full_qualification_corpus_v1(
    corpus: &CompilerProbeCorpusV1,
) -> Result<(), OfflineQualificationErrorV1> {
    corpus.validate()?;
    let canonical = full_qualification_corpus_v1()?;
    if corpus != &canonical {
        return Err(OfflineQualificationErrorV1::CorpusMismatch);
    }
    Ok(())
}

fn section(module: &str, relative_path: &str, source_utf8: &str) -> ProbeSourceSectionV1 {
    ProbeSourceSectionV1 {
        ordinal: 0,
        module: module.into(),
        relative_path: relative_path.into(),
        source_utf8: source_utf8.into(),
        source_sha256: Sha256Digest::from_bytes(Sha256::digest(source_utf8.as_bytes()).into()),
    }
}

fn case(
    case_id: &str,
    category: &str,
    expected_outcome: ProbeOutcomeV1,
    mode: ProbeModeV1,
    sections: &[ProbeSourceSectionV1],
) -> CompilerProbeCaseV1 {
    CompilerProbeCaseV1 {
        ordinal: 0,
        case_id: case_id.into(),
        category: category.into(),
        expected_outcome,
        mode,
        sections: sections
            .iter()
            .cloned()
            .enumerate()
            .map(|(ordinal, mut value)| {
                value.ordinal = ordinal as u32;
                value
            })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCacheArtifactSealV1 {
    pub blob_id: String,
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

/// Recomputed from the exact accepted cache bytes. The semantic digest binds all normalized
/// identities; the explicit counters make claimed opcode/model/tail coverage machine-checkable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCacheSemanticWitnessV1 {
    pub semantic_sha256: Sha256Digest,
    pub observed_opcodes: Vec<u8>,
    pub tail_table_counts: [u32; 7],
    pub class_count: u64,
    pub behaviour_function_count: u64,
    pub property_count: u64,
    pub global_count: u64,
    pub initializer_function_count: u64,
    pub string_global_reference_count: u32,
}

impl OfflineCacheSemanticWitnessV1 {
    pub fn from_observation(observation: &WholeCacheSemanticObservationV1) -> Self {
        Self {
            semantic_sha256: Sha256Digest::from_bytes(*observation.sha256()),
            observed_opcodes: observation
                .opcode_counts()
                .iter()
                .enumerate()
                .filter_map(|(opcode, &count)| (count != 0).then_some(opcode as u8))
                .collect(),
            tail_table_counts: *observation.tail_table_counts(),
            class_count: observation.class_count(),
            behaviour_function_count: observation.behaviour_function_count(),
            property_count: observation.property_count(),
            global_count: observation.global_count(),
            initializer_function_count: observation.initializer_function_count(),
            string_global_reference_count: observation.string_global_reference_count(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineFrontendHookCaptureV1 {
    pub subject_identity: String,
    pub generated_declarations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCompilerBuildFlagsWitnessV1 {
    pub as_reference_debugging: bool,
    pub resolve_object_ptr_callback_registered: bool,
}

/// Captured frontend artifacts that cannot be inferred from final cache bytes. The complete
/// structure is part of the artifact-manifest seal and is mandatory for the frontend probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineFrontendCoverageWitnessV1 {
    pub class_analyze_bound: bool,
    pub class_analyze_captures: Vec<OfflineFrontendHookCaptureV1>,
    pub process_chunks_bound: bool,
    pub process_chunks_captures: Vec<OfflineFrontendHookCaptureV1>,
    pub post_process_code_bound: bool,
    pub post_process_captures: Vec<OfflineFrontendHookCaptureV1>,
    pub generated_declarations: Vec<String>,
    pub editor_discovery: Vec<String>,
    pub release_discovery: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineGraphSourceWitnessV1 {
    pub module: String,
    pub relative_path: String,
    pub source_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineObservedModuleIdentityV1 {
    pub map_key: String,
    pub name: String,
    pub semantic_sha256: Sha256Digest,
}

/// Exact captured input manifests for both graph states and their derived operation sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineGraphTransitionWitnessV1 {
    pub baseline_cache: OfflineCacheArtifactSealV1,
    pub baseline_cache_semantics: OfflineCacheSemanticWitnessV1,
    pub baseline_sources: Vec<OfflineGraphSourceWitnessV1>,
    pub final_sources: Vec<OfflineGraphSourceWitnessV1>,
    pub changed_modules: Vec<String>,
    pub deleted_modules: Vec<String>,
    pub added_modules: Vec<String>,
    pub baseline_cache_modules: Vec<OfflineObservedModuleIdentityV1>,
    pub final_cache_modules: Vec<OfflineObservedModuleIdentityV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OfflineCanonicalInvokeValueV1 {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F32Bits(u32),
    F64Bits(u64),
    Utf8(String),
    Bytes(Vec<u8>),
    Sequence(Vec<OfflineCanonicalInvokeValueV1>),
    Record(Vec<OfflineCanonicalInvokeFieldV1>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCanonicalInvokeFieldV1 {
    pub name: String,
    pub value: OfflineCanonicalInvokeValueV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCanonicalInvokeReturnV1 {
    pub type_identity: String,
    pub value: OfflineCanonicalInvokeValueV1,
}

impl OfflineCanonicalInvokeReturnV1 {
    pub(crate) fn into_observer_value(self) -> CanonicalInvokeReturnV1 {
        CanonicalInvokeReturnV1::new(self.type_identity, self.value.into_observer_value())
    }
}

impl OfflineCanonicalInvokeValueV1 {
    fn into_observer_value(self) -> CanonicalInvokeValueV1 {
        match self {
            Self::Null => CanonicalInvokeValueV1::Null,
            Self::Bool(value) => CanonicalInvokeValueV1::Bool(value),
            Self::I64(value) => CanonicalInvokeValueV1::I64(value),
            Self::U64(value) => CanonicalInvokeValueV1::U64(value),
            Self::F32Bits(value) => CanonicalInvokeValueV1::F32Bits(value),
            Self::F64Bits(value) => CanonicalInvokeValueV1::F64Bits(value),
            Self::Utf8(value) => CanonicalInvokeValueV1::Utf8(value),
            Self::Bytes(value) => CanonicalInvokeValueV1::Bytes(value),
            Self::Sequence(values) => CanonicalInvokeValueV1::Sequence(
                values.into_iter().map(Self::into_observer_value).collect(),
            ),
            Self::Record(fields) => CanonicalInvokeValueV1::Record(
                fields
                    .into_iter()
                    .map(|field| (field.name, field.value.into_observer_value()))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCompilerProbeArtifactEntryV1 {
    pub ordinal: u32,
    pub case_id: String,
    pub outcome: ProbeOutcomeV1,
    pub diagnostics: Vec<ExpectedDiagnosticV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<OfflineCacheArtifactSealV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_semantics: Option<OfflineCacheSemanticWitnessV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke_return: Option<OfflineCanonicalInvokeReturnV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_coverage: Option<OfflineFrontendCoverageWitnessV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_transition: Option<OfflineGraphTransitionWitnessV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_build_flags: Option<OfflineCompilerBuildFlagsWitnessV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCompilerProbeArtifactManifestV1 {
    pub schema: String,
    pub schema_version: u32,
    pub semantic_observer: String,
    pub suite_id: String,
    pub corpus_sha256: Sha256Digest,
    pub backend: CompilerProbeBackendKindV1,
    pub source_profile_sha256: Sha256Digest,
    pub source_target: CompilerTargetV1,
    pub standalone_compiler: Option<QualifiedSidecarIdentityV1>,
    pub entries: Vec<OfflineCompilerProbeArtifactEntryV1>,
    pub canonical_sha256: Sha256Digest,
}

/// Supplemental artifacts returned by the same capture transaction as a probe observation.
/// They are not inferred from case labels by the generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflineCapturedSupplementalWitnessV1 {
    None,
    Frontend(OfflineFrontendCoverageWitnessV1),
    CompilerBuildFlags(OfflineCompilerBuildFlagsWitnessV1),
}

/// Raw output of one already-authorized backend run. It deliberately carries cache bytes, not a
/// caller-selected semantic digest or cache witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineCapturedProbeOutputV1 {
    observation: CompilerProbeObservationV1,
    supplemental: OfflineCapturedSupplementalWitnessV1,
    graph_baseline_cache_bytes: Option<Vec<u8>>,
}

impl OfflineCapturedProbeOutputV1 {
    /// Read-only raw observation for the capture harness's explicit debug-case mode. Exposing the
    /// already-constructed value does not let callers attach or forge supplemental evidence.
    pub fn observation(&self) -> &CompilerProbeObservationV1 {
        &self.observation
    }

    /// Read-only compiler flags captured by the same transaction, when this probe owns them.
    pub fn compiler_build_flags(&self) -> Option<&OfflineCompilerBuildFlagsWitnessV1> {
        match &self.supplemental {
            OfflineCapturedSupplementalWitnessV1::CompilerBuildFlags(value) => Some(value),
            _ => None,
        }
    }

    pub fn accepted(
        diagnostics: Vec<ExpectedDiagnosticV1>,
        cache_bytes: Vec<u8>,
        invoke_return: Option<CanonicalInvokeReturnV1>,
    ) -> Self {
        Self {
            observation: CompilerProbeObservationV1::accepted(
                diagnostics,
                cache_bytes,
                invoke_return,
            ),
            supplemental: OfflineCapturedSupplementalWitnessV1::None,
            graph_baseline_cache_bytes: None,
        }
    }

    // Kept crate-private so only a reviewed native capture adapter can attach hook/build traces;
    // no external caller can promote hand-authored supplemental JSON through the generator.
    #[allow(dead_code)]
    pub(crate) fn accepted_with_supplemental(
        diagnostics: Vec<ExpectedDiagnosticV1>,
        cache_bytes: Vec<u8>,
        invoke_return: Option<CanonicalInvokeReturnV1>,
        supplemental: OfflineCapturedSupplementalWitnessV1,
    ) -> Self {
        Self {
            observation: CompilerProbeObservationV1::accepted(
                diagnostics,
                cache_bytes,
                invoke_return,
            ),
            supplemental,
            graph_baseline_cache_bytes: None,
        }
    }

    pub fn accepted_graph_transition(
        diagnostics: Vec<ExpectedDiagnosticV1>,
        baseline_cache_bytes: Vec<u8>,
        final_cache_bytes: Vec<u8>,
    ) -> Self {
        Self {
            observation: CompilerProbeObservationV1::accepted(diagnostics, final_cache_bytes, None),
            supplemental: OfflineCapturedSupplementalWitnessV1::None,
            graph_baseline_cache_bytes: Some(baseline_cache_bytes),
        }
    }

    pub fn rejected(diagnostics: Vec<ExpectedDiagnosticV1>) -> Self {
        Self {
            observation: CompilerProbeObservationV1::rejected(diagnostics),
            supplemental: OfflineCapturedSupplementalWitnessV1::None,
            graph_baseline_cache_bytes: None,
        }
    }
}

/// Adapter implemented by a product-specific capture reader. This crate never starts either
/// backend; it only consumes one complete ordered run and seals what was actually returned.
pub trait OfflineQualificationCaptureBackendV1 {
    /// Exact unqualified profile whose typed registry/frontend authorities drive this run.
    fn source_profile_sha256(&self) -> Sha256Digest;

    /// Target tuple sealed by the same source profile. This is duplicated in the run manifest so
    /// profile identity and target cannot be relabelled after capture.
    fn source_target(&self) -> CompilerTargetV1;

    /// Exact executable identity for standalone runs; embedded-game runs must return `None`.
    fn standalone_compiler_identity(&self) -> Option<QualifiedSidecarIdentityV1>;

    fn capture_probe(
        &mut self,
        case: &CompilerProbeCaseV1,
    ) -> Result<OfflineCapturedProbeOutputV1, CompilerProbeBackendErrorV1>;
}

/// In-memory authority token produced only after a complete capture run has been observed,
/// witnessed, manifest-sealed, and reloaded through the strict offline adapter.
#[derive(Debug, Clone)]
pub struct GeneratedOfflineCompilerProbeArtifactsV1 {
    manifest: OfflineCompilerProbeArtifactManifestV1,
    manifest_json: Vec<u8>,
    cache_blobs: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineCaseCacheSealAuthorityV1 {
    pub case_id: String,
    pub artifact_role: OfflineCacheArtifactRoleV1,
    pub cache: OfflineCacheArtifactSealV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineCacheArtifactRoleV1 {
    AcceptedFinal,
    GraphBaseline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineArtifactAuthoritySummaryV1 {
    pub backend: CompilerProbeBackendKindV1,
    pub suite_id: String,
    pub corpus_sha256: Sha256Digest,
    pub source_profile_sha256: Sha256Digest,
    pub source_target: CompilerTargetV1,
    pub standalone_compiler: Option<QualifiedSidecarIdentityV1>,
    pub manifest_canonical_sha256: Sha256Digest,
    pub manifest_json_sha256: Sha256Digest,
    pub cache_seals: Vec<OfflineCaseCacheSealAuthorityV1>,
    pub cache_seals_sha256: Sha256Digest,
    pub supplemental_witnesses_sha256: Sha256Digest,
}

#[derive(Serialize)]
struct SupplementalAuthorityRowV1<'a> {
    case_id: &'a str,
    frontend_coverage: &'a Option<OfflineFrontendCoverageWitnessV1>,
    graph_transition: &'a Option<OfflineGraphTransitionWitnessV1>,
    compiler_build_flags: &'a Option<OfflineCompilerBuildFlagsWitnessV1>,
}

impl GeneratedOfflineCompilerProbeArtifactsV1 {
    pub fn backend(&self) -> CompilerProbeBackendKindV1 {
        self.manifest.backend
    }

    pub fn manifest_json(&self) -> &[u8] {
        &self.manifest_json
    }

    pub fn cache_blobs(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.cache_blobs
    }

    pub fn authority_summary(
        &self,
    ) -> Result<OfflineArtifactAuthoritySummaryV1, OfflineQualificationErrorV1> {
        artifact_authority_summary(&self.manifest, &self.manifest_json)
    }
}

/// Reload a generated artifact authority from its exact manifest bytes and complete cache-blob
/// set. This is the disk-to-memory boundary used by the promotion tool: the normal offline
/// backend validates the corpus, manifest seal, supplemental witnesses, cache seals, whole-cache
/// semantics, and the absence of missing or extra blobs before an authority token is returned.
pub fn reload_generated_offline_qualification_artifacts_v1(
    corpus: &CompilerProbeCorpusV1,
    expected_backend: CompilerProbeBackendKindV1,
    manifest_json: &[u8],
    cache_blobs: BTreeMap<String, Vec<u8>>,
) -> Result<GeneratedOfflineCompilerProbeArtifactsV1, OfflineQualificationErrorV1> {
    OfflineCompilerProbeArtifactBackendV1::load(
        corpus,
        expected_backend,
        manifest_json,
        cache_blobs.clone(),
    )?;
    let manifest = OfflineCompilerProbeArtifactManifestV1::from_json(manifest_json)?;
    Ok(GeneratedOfflineCompilerProbeArtifactsV1 {
        manifest,
        manifest_json: manifest_json.to_vec(),
        cache_blobs,
    })
}

/// Promotion output ready to be sealed into a compiler-profile package. `qualified()` is true
/// only after both generated artifact authorities pass the normal differential runner.
#[derive(Debug, Clone)]
pub struct OfflineQualificationPromotionV1 {
    source_profile_sha256: Sha256Digest,
    source_target: CompilerTargetV1,
    standalone_compiler: QualifiedSidecarIdentityV1,
    expected_results: ExpectedProbeResultsV1,
    differential: DifferentialQualificationRunV1,
    embedded_artifacts: GeneratedOfflineCompilerProbeArtifactsV1,
    standalone_artifacts: GeneratedOfflineCompilerProbeArtifactsV1,
}

impl OfflineQualificationPromotionV1 {
    pub fn qualified(&self) -> bool {
        self.differential.qualified()
    }

    pub fn expected_results(&self) -> &ExpectedProbeResultsV1 {
        &self.expected_results
    }

    pub fn source_profile_sha256(&self) -> Sha256Digest {
        self.source_profile_sha256
    }

    pub fn source_target(&self) -> &CompilerTargetV1 {
        &self.source_target
    }

    pub fn standalone_compiler(&self) -> QualifiedSidecarIdentityV1 {
        self.standalone_compiler
    }

    pub fn differential(&self) -> &DifferentialQualificationRunV1 {
        &self.differential
    }

    pub fn embedded_artifacts(&self) -> &GeneratedOfflineCompilerProbeArtifactsV1 {
        &self.embedded_artifacts
    }

    pub fn standalone_artifacts(&self) -> &GeneratedOfflineCompilerProbeArtifactsV1 {
        &self.standalone_artifacts
    }
}

impl OfflineCompilerProbeArtifactManifestV1 {
    pub fn seal(&mut self) -> Result<(), OfflineQualificationErrorV1> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), OfflineQualificationErrorV1> {
        self.validate_structure()?;
        let actual = self.computed_digest()?;
        if self.canonical_sha256 != actual {
            return Err(OfflineQualificationErrorV1::ManifestSealMismatch);
        }
        Ok(())
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, OfflineQualificationErrorV1> {
        if bytes.len() > MAX_OFFLINE_ARTIFACT_JSON_BYTES_V1 {
            return Err(OfflineQualificationErrorV1::ManifestTooLarge {
                actual: bytes.len(),
                max: MAX_OFFLINE_ARTIFACT_JSON_BYTES_V1,
            });
        }
        let manifest: Self = serde_json::from_slice(bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, OfflineQualificationErrorV1> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), OfflineQualificationErrorV1> {
        if self.schema != OFFLINE_PROBE_ARTIFACT_SCHEMA_V1
            || self.schema_version != OFFLINE_PROBE_ARTIFACT_SCHEMA_VERSION_V1
        {
            return Err(OfflineQualificationErrorV1::ManifestSchema);
        }
        if self.semantic_observer != SEMANTIC_OBSERVER_CONTRACT_V1 {
            return Err(OfflineQualificationErrorV1::ObserverContract);
        }
        if self.source_profile_sha256 == zero_digest()
            || self.source_target.steam_app_id == 0
            || self.source_target.steam_build_id == 0
            || self.source_target.depot_id == 0
            || self.source_target.depot_manifest_gid == 0
        {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "source profile and target authority must be nonzero",
            ));
        }
        match (self.backend, self.standalone_compiler) {
            (CompilerProbeBackendKindV1::EmbeddedGame, None) => {}
            (CompilerProbeBackendKindV1::Standalone, Some(identity)) => identity.validate()?,
            _ => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "standalone compiler identity must appear exactly on standalone artifacts",
                ));
            }
        }
        if self.suite_id.is_empty() || self.entries.is_empty() {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "suite and entries must be nonempty",
            ));
        }
        let mut case_ids = BTreeSet::new();
        let mut blob_ids = BTreeSet::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.ordinal as usize != index || !case_ids.insert(entry.case_id.as_str()) {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "entry ordinals and case ids must be exact and unique",
                ));
            }
            match (
                entry.outcome,
                &entry.cache,
                &entry.cache_semantics,
                &entry.invoke_return,
                &entry.frontend_coverage,
                &entry.graph_transition,
                &entry.compiler_build_flags,
            ) {
                (ProbeOutcomeV1::Accepted, Some(cache), Some(semantics), _, _, _, _) => {
                    validate_blob_id(&cache.blob_id)?;
                    if cache.byte_len == 0 || !blob_ids.insert(cache.blob_id.as_str()) {
                        return Err(OfflineQualificationErrorV1::InvalidManifest(
                            "accepted cache seals must be nonempty and use unique blob ids",
                        ));
                    }
                    validate_cache_semantic_witness(semantics)?;
                }
                (ProbeOutcomeV1::Rejected, None, None, None, None, None, None) => {}
                _ => {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "accepted entries require cache and semantic witness; rejected entries permit diagnostics only",
                    ));
                }
            }
            if let Some(value) = &entry.invoke_return {
                if value.type_identity.is_empty()
                    || value.type_identity.contains('\0')
                    || value.type_identity.chars().any(char::is_control)
                {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "invoke return type identity is empty or contains controls",
                    ));
                }
            }
            if let Some(witness) = &entry.frontend_coverage {
                validate_frontend_coverage_witness(witness)?;
            }
            if let Some(witness) = &entry.graph_transition {
                validate_blob_id(&witness.baseline_cache.blob_id)?;
                if witness.baseline_cache.byte_len == 0
                    || !blob_ids.insert(witness.baseline_cache.blob_id.as_str())
                {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "graph baseline cache seals must be nonempty and use unique blob ids",
                    ));
                }
                validate_cache_semantic_witness(&witness.baseline_cache_semantics)?;
                validate_graph_transition_witness(witness)?;
            }
            validate_diagnostics(entry)?;
        }
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, OfflineQualificationErrorV1> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        let bytes = serde_json::to_vec(&canonical)?;
        let mut digest = Sha256::new();
        digest.update(OFFLINE_ARTIFACT_HASH_DOMAIN_V1);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
        Ok(Sha256Digest::from_bytes(digest.finalize().into()))
    }
}

pub fn offline_artifact_authority_summary_from_manifest_json_v1(
    manifest_json: &[u8],
) -> Result<OfflineArtifactAuthoritySummaryV1, OfflineQualificationErrorV1> {
    let manifest = OfflineCompilerProbeArtifactManifestV1::from_json(manifest_json)?;
    artifact_authority_summary(&manifest, manifest_json)
}

fn artifact_authority_summary(
    manifest: &OfflineCompilerProbeArtifactManifestV1,
    manifest_json: &[u8],
) -> Result<OfflineArtifactAuthoritySummaryV1, OfflineQualificationErrorV1> {
    manifest.validate()?;
    let mut cache_seals = Vec::new();
    for entry in &manifest.entries {
        if let Some(cache) = entry.cache.clone() {
            cache_seals.push(OfflineCaseCacheSealAuthorityV1 {
                case_id: entry.case_id.clone(),
                artifact_role: OfflineCacheArtifactRoleV1::AcceptedFinal,
                cache,
            });
        }
        if let Some(graph) = &entry.graph_transition {
            cache_seals.push(OfflineCaseCacheSealAuthorityV1 {
                case_id: entry.case_id.clone(),
                artifact_role: OfflineCacheArtifactRoleV1::GraphBaseline,
                cache: graph.baseline_cache.clone(),
            });
        }
    }
    let cache_seals_sha256 =
        domain_separated_json_sha256(OFFLINE_CACHE_SEAL_AUTHORITY_DOMAIN_V1, &cache_seals)?;
    let supplemental: Vec<_> = manifest
        .entries
        .iter()
        .map(|entry| SupplementalAuthorityRowV1 {
            case_id: &entry.case_id,
            frontend_coverage: &entry.frontend_coverage,
            graph_transition: &entry.graph_transition,
            compiler_build_flags: &entry.compiler_build_flags,
        })
        .collect();
    let supplemental_witnesses_sha256 =
        domain_separated_json_sha256(OFFLINE_SUPPLEMENTAL_AUTHORITY_DOMAIN_V1, &supplemental)?;
    Ok(OfflineArtifactAuthoritySummaryV1 {
        backend: manifest.backend,
        suite_id: manifest.suite_id.clone(),
        corpus_sha256: manifest.corpus_sha256,
        source_profile_sha256: manifest.source_profile_sha256,
        source_target: manifest.source_target.clone(),
        standalone_compiler: manifest.standalone_compiler,
        manifest_canonical_sha256: manifest.canonical_sha256,
        manifest_json_sha256: Sha256Digest::from_bytes(Sha256::digest(manifest_json).into()),
        cache_seals,
        cache_seals_sha256,
        supplemental_witnesses_sha256,
    })
}

fn domain_separated_json_sha256(
    domain: &[u8],
    value: &impl Serialize,
) -> Result<Sha256Digest, OfflineQualificationErrorV1> {
    let bytes = serde_json::to_vec(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    Ok(Sha256Digest::from_bytes(digest.finalize().into()))
}

fn validate_cache_semantic_witness(
    witness: &OfflineCacheSemanticWitnessV1,
) -> Result<(), OfflineQualificationErrorV1> {
    if witness
        .observed_opcodes
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "observed opcodes must be sorted and unique",
        ));
    }
    Ok(())
}

fn validate_witness_text(value: &str) -> Result<(), OfflineQualificationErrorV1> {
    if value.is_empty()
        || value.len() > MAX_ARTIFACT_BLOB_ID_BYTES_V1
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "coverage witness identity is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_frontend_coverage_witness(
    witness: &OfflineFrontendCoverageWitnessV1,
) -> Result<(), OfflineQualificationErrorV1> {
    for (bound, captures) in [
        (witness.class_analyze_bound, &witness.class_analyze_captures),
        (
            witness.process_chunks_bound,
            &witness.process_chunks_captures,
        ),
        (
            witness.post_process_code_bound,
            &witness.post_process_captures,
        ),
    ] {
        if bound != !captures.is_empty() {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "frontend hook captures must be nonempty exactly when the target hook is bound",
            ));
        }
        for capture in captures {
            validate_witness_text(&capture.subject_identity)?;
            if capture.generated_declarations.is_empty() {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "frontend hook captures require generated declarations",
                ));
            }
            for value in &capture.generated_declarations {
                validate_witness_text(value)?;
            }
        }
    }
    for values in [&witness.editor_discovery, &witness.release_discovery] {
        if values.is_empty() {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "frontend editor/release witness sets must be nonempty",
            ));
        }
        for value in values {
            validate_witness_text(value)?;
        }
    }
    let mut expected_generated = BTreeSet::new();
    for capture in witness
        .class_analyze_captures
        .iter()
        .chain(&witness.process_chunks_captures)
        .chain(&witness.post_process_captures)
    {
        expected_generated.extend(capture.generated_declarations.iter().cloned());
    }
    let actual_generated: BTreeSet<_> = witness.generated_declarations.iter().cloned().collect();
    if actual_generated.len() != witness.generated_declarations.len()
        || actual_generated != expected_generated
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "frontend generated declarations must exactly equal the same-run hook capture set",
        ));
    }
    for value in &witness.generated_declarations {
        validate_witness_text(value)?;
    }
    Ok(())
}

fn validate_graph_transition_witness(
    witness: &OfflineGraphTransitionWitnessV1,
) -> Result<(), OfflineQualificationErrorV1> {
    if witness.baseline_sources.is_empty()
        || witness.final_sources.is_empty()
        || witness.changed_modules.is_empty()
        || witness.deleted_modules.is_empty()
        || witness.added_modules.is_empty()
        || witness.baseline_cache_modules.is_empty()
        || witness.final_cache_modules.is_empty()
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "graph witness must prove nonempty baseline/final/change/delete/add sets",
        ));
    }
    for source in witness
        .baseline_sources
        .iter()
        .chain(&witness.final_sources)
    {
        validate_witness_text(&source.module)?;
        validate_witness_text(&source.relative_path)?;
    }
    for value in witness
        .changed_modules
        .iter()
        .chain(&witness.deleted_modules)
        .chain(&witness.added_modules)
    {
        validate_witness_text(value)?;
    }
    for (label, modules) in [
        ("baseline", &witness.baseline_cache_modules),
        ("final", &witness.final_cache_modules),
    ] {
        let mut map_keys = BTreeSet::new();
        let mut names = BTreeSet::new();
        for module in modules {
            validate_witness_text(&module.map_key)?;
            validate_witness_text(&module.name)?;
            if !map_keys.insert(module.map_key.as_str()) || !names.insert(module.name.as_str()) {
                return Err(OfflineQualificationErrorV1::InvalidManifest(match label {
                    "baseline" => {
                        "graph baseline-cache module map keys and names must each be unique"
                    }
                    _ => "graph final-cache module map keys and names must each be unique",
                }));
            }
        }
    }
    Ok(())
}

fn validate_blob_id(value: &str) -> Result<(), OfflineQualificationErrorV1> {
    if value.is_empty()
        || value.len() > MAX_ARTIFACT_BLOB_ID_BYTES_V1
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "cache blob id is invalid",
        ));
    }
    Ok(())
}

fn offline_module_identities(
    observation: &WholeCacheSemanticObservationV1,
) -> Vec<OfflineObservedModuleIdentityV1> {
    observation
        .module_identities()
        .iter()
        .map(|module| OfflineObservedModuleIdentityV1 {
            map_key: module.map_key().to_owned(),
            name: module.name().to_owned(),
            semantic_sha256: Sha256Digest::from_bytes(*module.semantic_sha256()),
        })
        .collect()
}

fn validate_diagnostics(
    entry: &OfflineCompilerProbeArtifactEntryV1,
) -> Result<(), OfflineQualificationErrorV1> {
    let mut expected = ExpectedProbeResultsV1 {
        schema: EXPECTED_RESULTS_SCHEMA.into(),
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        suite_id: "offline-artifact-validation".into(),
        corpus_sha256: zero_digest(),
        results: vec![ExpectedProbeResultV1 {
            ordinal: 0,
            case_id: entry.case_id.clone(),
            outcome: entry.outcome,
            diagnostics: entry.diagnostics.clone(),
            semantic_sha256: (entry.outcome == ProbeOutcomeV1::Accepted)
                .then(|| Sha256Digest::from_bytes([1; 32])),
        }],
        canonical_sha256: zero_digest(),
    };
    expected.seal()?;
    Ok(())
}

/// Strict in-memory adapter for already captured cache artifacts.
///
/// Blob ids are logical manifest keys, not filesystem paths. The caller must provide exactly the
/// sealed blob set—missing, extra, wrong-length, or wrong-digest bytes are rejected before a
/// backend can execute a probe.
pub struct OfflineCompilerProbeArtifactBackendV1 {
    observations: VecDeque<(String, CompilerProbeObservationV1)>,
}

impl OfflineCompilerProbeArtifactBackendV1 {
    pub fn load(
        corpus: &CompilerProbeCorpusV1,
        expected_backend: CompilerProbeBackendKindV1,
        manifest_json: &[u8],
        mut cache_blobs: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, OfflineQualificationErrorV1> {
        validate_canonical_full_qualification_corpus_v1(corpus)?;
        let manifest = OfflineCompilerProbeArtifactManifestV1::from_json(manifest_json)?;
        if manifest.backend != expected_backend
            || manifest.suite_id != corpus.suite_id
            || manifest.corpus_sha256 != corpus.canonical_sha256
            || manifest.entries.len() != corpus.cases.len()
        {
            return Err(OfflineQualificationErrorV1::CorpusMismatch);
        }
        if corpus.suite_id == FULL_QUALIFICATION_SUITE_ID_V1 {
            validate_full_suite_diagnostic_coverage(&manifest.entries)?;
        }
        let mut observations = VecDeque::new();
        observations
            .try_reserve(manifest.entries.len())
            .map_err(|_| OfflineQualificationErrorV1::AllocationFailed)?;
        for (case, entry) in corpus.cases.iter().zip(manifest.entries) {
            if entry.ordinal != case.ordinal || entry.case_id != case.case_id {
                return Err(OfflineQualificationErrorV1::CorpusMismatch);
            }
            let expects_graph_witness = matches!(
                (&case.mode, entry.outcome),
                (
                    ProbeModeV1::CompileGraphTransition { .. },
                    ProbeOutcomeV1::Accepted
                )
            );
            if entry.graph_transition.is_some() != expects_graph_witness {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "graph baseline/final witness must appear exactly on an accepted graph-transition case",
                ));
            }
            match (&case.mode, entry.outcome, entry.invoke_return.as_ref()) {
                (ProbeModeV1::CompileOnly, ProbeOutcomeV1::Accepted, None)
                | (ProbeModeV1::CompileGraphTransition { .. }, ProbeOutcomeV1::Accepted, None)
                | (ProbeModeV1::Invoke { .. }, ProbeOutcomeV1::Accepted, Some(_))
                | (_, ProbeOutcomeV1::Rejected, None) => {}
                (
                    ProbeModeV1::CompileOnly | ProbeModeV1::CompileGraphTransition { .. },
                    ProbeOutcomeV1::Accepted,
                    Some(_),
                ) => {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "compile-only artifact cannot carry an invoke return",
                    ));
                }
                (ProbeModeV1::Invoke { .. }, ProbeOutcomeV1::Accepted, None) => {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "accepted invoke artifact requires a typed return",
                    ));
                }
                (_, ProbeOutcomeV1::Rejected, Some(_)) => {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "rejected artifact cannot carry an invoke return",
                    ));
                }
            }
            let observation = match entry.outcome {
                ProbeOutcomeV1::Accepted => {
                    let seal = entry.cache.as_ref().ok_or(
                        OfflineQualificationErrorV1::InvalidManifest(
                            "accepted artifact omitted its validated cache seal",
                        ),
                    )?;
                    let bytes = cache_blobs.remove(&seal.blob_id).ok_or_else(|| {
                        OfflineQualificationErrorV1::MissingBlob(seal.blob_id.clone())
                    })?;
                    if seal.byte_len != bytes.len() as u64
                        || seal.sha256 != Sha256Digest::from_bytes(Sha256::digest(&bytes).into())
                    {
                        return Err(OfflineQualificationErrorV1::BlobSealMismatch(
                            seal.blob_id.clone(),
                        ));
                    }
                    let invoke_return = entry
                        .invoke_return
                        .clone()
                        .map(OfflineCanonicalInvokeReturnV1::into_observer_value);
                    let semantic_observation =
                        observe_whole_cache_semantics_v1(&bytes, invoke_return.as_ref()).map_err(
                            |error| OfflineQualificationErrorV1::SemanticObservation {
                                case_id: case.case_id.clone(),
                                detail: error.to_string(),
                            },
                        )?;
                    let actual_witness =
                        OfflineCacheSemanticWitnessV1::from_observation(&semantic_observation);
                    if entry.cache_semantics.as_ref() != Some(&actual_witness) {
                        return Err(OfflineQualificationErrorV1::SemanticWitnessMismatch(
                            case.case_id.clone(),
                        ));
                    }
                    let baseline_semantic_observation = if let Some(graph) = &entry.graph_transition
                    {
                        let baseline_bytes = cache_blobs
                            .remove(&graph.baseline_cache.blob_id)
                            .ok_or_else(|| {
                                OfflineQualificationErrorV1::MissingBlob(
                                    graph.baseline_cache.blob_id.clone(),
                                )
                            })?;
                        if graph.baseline_cache.byte_len != baseline_bytes.len() as u64
                            || graph.baseline_cache.sha256
                                != Sha256Digest::from_bytes(Sha256::digest(&baseline_bytes).into())
                        {
                            return Err(OfflineQualificationErrorV1::BlobSealMismatch(
                                graph.baseline_cache.blob_id.clone(),
                            ));
                        }
                        let baseline = observe_whole_cache_semantics_v1(&baseline_bytes, None)
                            .map_err(|error| OfflineQualificationErrorV1::SemanticObservation {
                                case_id: case.case_id.clone(),
                                detail: format!("graph baseline: {error}"),
                            })?;
                        if graph.baseline_cache_semantics
                            != OfflineCacheSemanticWitnessV1::from_observation(&baseline)
                            || graph.baseline_cache_modules != offline_module_identities(&baseline)
                        {
                            return Err(OfflineQualificationErrorV1::SemanticWitnessMismatch(
                                format!("{}:graph-baseline", case.case_id),
                            ));
                        }
                        Some(baseline)
                    } else {
                        None
                    };
                    if corpus.suite_id == FULL_QUALIFICATION_SUITE_ID_V1 {
                        validate_full_suite_artifact_witness(
                            case,
                            &entry,
                            &semantic_observation,
                            baseline_semantic_observation.as_ref(),
                        )?;
                    }
                    CompilerProbeObservationV1::accepted(entry.diagnostics, bytes, invoke_return)
                }
                ProbeOutcomeV1::Rejected => {
                    if corpus.suite_id == FULL_QUALIFICATION_SUITE_ID_V1 {
                        validate_full_suite_rejected_witness(case, &entry)?;
                    }
                    CompilerProbeObservationV1::rejected(entry.diagnostics)
                }
            };
            observations.push_back((case.case_id.clone(), observation));
        }
        if let Some(extra) = cache_blobs.into_keys().next() {
            return Err(OfflineQualificationErrorV1::UnexpectedBlob(extra));
        }
        Ok(Self { observations })
    }
}

fn validate_full_suite_artifact_witness(
    case: &CompilerProbeCaseV1,
    entry: &OfflineCompilerProbeArtifactEntryV1,
    observation: &WholeCacheSemanticObservationV1,
    baseline_observation: Option<&WholeCacheSemanticObservationV1>,
) -> Result<(), OfflineQualificationErrorV1> {
    let expects_frontend = case.case_id == "positive.frontend.hooks-editor-release";
    let expects_graph = case.case_id == "positive.module-graph.change-delete";
    let expects_build_flags = matches!(
        case.case_id.as_str(),
        "positive.bytecode.fork-reference-lifecycle"
            | "positive.bytecode.unresolved-object-property"
    );
    if entry.frontend_coverage.is_some() != expects_frontend
        || entry.graph_transition.is_some() != expects_graph
        || entry.compiler_build_flags.is_some() != expects_build_flags
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "full-suite frontend/graph/build witnesses must appear on their exact probe only",
        ));
    }
    let observed_module_names: BTreeSet<_> = observation
        .module_identities()
        .iter()
        .map(|module| module.name())
        .collect();
    if case
        .sections
        .iter()
        .any(|section| !observed_module_names.contains(section.module.as_str()))
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "accepted full-suite cache omitted a corpus source module",
        ));
    }
    for opcode in [
        "DestructScript",
        "TrackRef",
        "UntrackRef",
        "ValidateRef",
        "SaveReturnValue",
        "ResolveObjectPtr",
    ] {
        if observation.opcode_count_named(opcode) != Some(0) {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "target-disabled or noncompiler fork opcode appeared in a real qualification artifact",
            ));
        }
    }
    match case.case_id.as_str() {
        "positive.invoke.structured" => match entry.invoke_return.as_ref() {
            Some(OfflineCanonicalInvokeReturnV1 {
                type_identity,
                value: OfflineCanonicalInvokeValueV1::Sequence(values),
            }) if type_identity == "TArray<int32>"
                && values.as_slice()
                    == [
                        OfflineCanonicalInvokeValueV1::I64(0),
                        OfflineCanonicalInvokeValueV1::I64(0),
                    ] => {}
            _ => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "structured invoke artifact must return the canonical two-element zeroed TArray<int32>",
                ));
            }
        },
        "positive.bytecode.fork-reference-lifecycle" => {
            if ![
                "FinConstruct",
                "CopyScript",
                "FreeNullV8",
                "CpyVtoR1",
                "CmpPtrNull",
                "ThrowException",
            ]
            .into_iter()
            .all(|opcode| {
                observation
                    .opcode_count_named(opcode)
                    .is_some_and(|count| count > 0)
            }) || entry.compiler_build_flags.as_ref()
                != Some(&OfflineCompilerBuildFlagsWitnessV1 {
                    as_reference_debugging: false,
                    resolve_object_ptr_callback_registered: false,
                })
            {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "fork lifecycle artifact lacks a reachable opcode or the disabled reference-debug build flag",
                ));
            }
        }
        "positive.bytecode.unresolved-object-property" => {
            let flags = entry.compiler_build_flags.as_ref().ok_or(
                OfflineQualificationErrorV1::InvalidManifest(
                    "unresolved-object property case omitted captured compiler build flags",
                ),
            )?;
            let count = observation.opcode_count_named("ResolveObjectPtr").ok_or(
                OfflineQualificationErrorV1::InvalidManifest(
                    "canonical opcode table omitted ResolveObjectPtr",
                ),
            )?;
            if flags.as_reference_debugging
                || flags.resolve_object_ptr_callback_registered
                || count != 0
            {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "Shipping target must prove a disabled ResolveObjectPtr callback and exactly zero emitted ResolveObjectPtr opcodes",
                ));
            }
        }
        "positive.class-generator.editor-flags" => {
            validate_class_generator_property_witness(observation)
                .map_err(OfflineQualificationErrorV1::InvalidManifest)?;
        }
        "positive.model.globals-classes-all-tails" => {
            if observation
                .tail_table_counts()
                .iter()
                .any(|&count| count == 0)
                || observation.class_count() == 0
                || observation.behaviour_function_count() == 0
                || observation.property_count() == 0
                || observation.global_count() == 0
                || observation.initializer_function_count() == 0
            {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "complete-model artifact lacks an observed tail/model identity group",
                ));
            }
        }
        "positive.fname.non-ascii-equivalence" => match entry.invoke_return.as_ref() {
            Some(OfflineCanonicalInvokeReturnV1 {
                type_identity,
                value: OfflineCanonicalInvokeValueV1::Bool(true),
            }) if type_identity == "bool" => {}
            _ => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "FName equivalence artifact must return canonical bool true",
                ));
            }
        },
        "positive.fname.name-none-canonical" => match entry.invoke_return.as_ref() {
            Some(OfflineCanonicalInvokeReturnV1 {
                type_identity,
                value: OfflineCanonicalInvokeValueV1::Bool(true),
            }) if type_identity == "bool" && observation.static_names() == ["None".to_owned()] => {}
            _ => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "NAME_None artifact must return true and contain exactly the canonical static name None",
                ));
            }
        },
        "positive.strings.factory-roundtrip" => match entry.invoke_return.as_ref() {
            Some(OfflineCanonicalInvokeReturnV1 {
                type_identity,
                value: OfflineCanonicalInvokeValueV1::Utf8(value),
            }) if type_identity == "FString"
                && value == "Grüße_日本"
                && observation.opcode_count_named("STR") == Some(0)
                && observation.string_global_reference_count() > 0 => {}
            _ => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "string-factory artifact must use a string global, emit no asBC_STR, and roundtrip canonical UTF-8",
                ));
            }
        },
        "positive.frontend.hooks-editor-release" => {
            let frontend = entry.frontend_coverage.as_ref().ok_or(
                OfflineQualificationErrorV1::InvalidManifest(
                    "frontend probe omitted its same-run target witness",
                ),
            )?;
            // BuildID 24539464 contains no binding path for either mutable graph delegate.
            // Requiring synthetic hook hits would make the standalone artifact diverge from the
            // embedded compiler instead of proving parity with it.
            if frontend.process_chunks_bound
                || frontend.post_process_code_bound
                || !frontend.process_chunks_captures.is_empty()
                || !frontend.post_process_captures.is_empty()
            {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "Shipping target graph delegates must be observed unbound with no captures",
                ));
            }
        }
        "positive.module-graph.change-delete" => {
            let graph = entry.graph_transition.as_ref().ok_or(
                OfflineQualificationErrorV1::InvalidManifest(
                    "graph transition artifact omitted its final graph witness",
                ),
            )?;
            let baseline_observation =
                baseline_observation.ok_or(OfflineQualificationErrorV1::InvalidManifest(
                    "graph transition artifact omitted its observed baseline cache",
                ))?;
            let expected = graph_transition_witness_for_case(
                case,
                graph.baseline_cache.clone(),
                baseline_observation,
                observation,
            )
            .ok_or(OfflineQualificationErrorV1::InvalidManifest(
                "graph transition case omitted graph-transition mode",
            ))?;
            if graph != &expected {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "graph artifact manifest does not exactly prove baseline/final/change/delete/add",
                ));
            }
            validate_observed_graph_transition(graph)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_class_generator_property_witness(
    observation: &WholeCacheSemanticObservationV1,
) -> Result<(), &'static str> {
    let property = |class_name: &str, property_name: &str| {
        let mut matches = observation.property_identities().iter().filter(|property| {
            property.module() == "GeneratedClass"
                && property.class_name() == class_name
                && property.property_name() == property_name
        });
        let first = matches.next();
        (matches.next().is_none()).then_some(first).flatten()
    };
    let class_property = property("UQualificationObject", "ImplicitObject");
    let struct_property = property("FQualificationStruct", "ImplicitObject");
    let class_scalar = property("UQualificationObject", "ImplicitScalar");
    let struct_scalar = property("FQualificationStruct", "ImplicitScalar");
    if class_property
        .is_some_and(|property| !property.unreal_property() && property.transient().is_none())
        && struct_property.is_some_and(|property| {
            property.unreal_property() && property.transient() == Some(false)
        })
        && class_scalar
            .is_some_and(|property| !property.unreal_property() && property.transient().is_none())
        && struct_scalar.is_some_and(|property| {
            property.unreal_property() && property.transient() == Some(false)
        })
    {
        Ok(())
    } else {
        Err(
            "class-generator artifact must apply target CanCreateProperty/NeverRequiresGC/RequiresProperty=false and class/struct transient policy",
        )
    }
}

fn class_generator_property_witness_detail(
    observation: &WholeCacheSemanticObservationV1,
) -> String {
    let rows = observation
        .property_identities()
        .iter()
        .filter(|property| {
            property.module() == "GeneratedClass"
                || property.class_name().contains("Qualification")
                || property.property_name().starts_with("Implicit")
        })
        .take(16)
        .map(|property| {
            format!(
                "{}/{}/{}:unreal={},transient={:?}",
                property.module(),
                property.class_name(),
                property.property_name(),
                property.unreal_property(),
                property.transient()
            )
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        "no GeneratedClass/Qualification/Implicit property identities observed".to_owned()
    } else {
        rows.join(", ")
    }
}

pub(crate) fn validate_full_suite_accepted_cache_boundary_v1(
    case: &CompilerProbeCaseV1,
    cache_bytes: &[u8],
) -> Result<(), String> {
    if !matches!(
        case.case_id.as_str(),
        "positive.class-generator.editor-flags" | "positive.fname.name-none-canonical"
    ) {
        return Ok(());
    }
    let observation = observe_whole_cache_semantics_v1(cache_bytes, None)
        .map_err(|error| format!("observing full-suite accepted cache: {error}"))?;
    match case.case_id.as_str() {
        "positive.class-generator.editor-flags" => {
            validate_class_generator_property_witness(&observation).map_err(|error| {
                format!(
                    "{error}; observed {}",
                    class_generator_property_witness_detail(&observation)
                )
            })
        }
        "positive.fname.name-none-canonical"
            if observation.static_names() == ["None".to_owned()] =>
        {
            Ok(())
        }
        "positive.fname.name-none-canonical" => Err(format!(
            "NAME_None cache must contain exactly [None], observed {:?}",
            observation.static_names()
        )),
        _ => unreachable!(),
    }
}

fn validate_full_suite_rejected_witness(
    _case: &CompilerProbeCaseV1,
    entry: &OfflineCompilerProbeArtifactEntryV1,
) -> Result<(), OfflineQualificationErrorV1> {
    if entry.frontend_coverage.is_some()
        || entry.graph_transition.is_some()
        || entry.compiler_build_flags.is_some()
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "rejected full-suite artifact cannot claim accepted-only witnesses",
        ));
    }
    Ok(())
}

fn graph_transition_witness_for_case(
    case: &CompilerProbeCaseV1,
    baseline_cache: OfflineCacheArtifactSealV1,
    baseline_observation: &WholeCacheSemanticObservationV1,
    final_observation: &WholeCacheSemanticObservationV1,
) -> Option<OfflineGraphTransitionWitnessV1> {
    let ProbeModeV1::CompileGraphTransition {
        baseline_sections,
        changed_modules,
        deleted_modules,
    } = &case.mode
    else {
        return None;
    };
    let baseline_modules: BTreeSet<_> = baseline_sections
        .iter()
        .map(|section| section.module.clone())
        .collect();
    let final_modules: BTreeSet<_> = case
        .sections
        .iter()
        .map(|section| section.module.clone())
        .collect();
    Some(OfflineGraphTransitionWitnessV1 {
        baseline_cache,
        baseline_cache_semantics: OfflineCacheSemanticWitnessV1::from_observation(
            baseline_observation,
        ),
        baseline_sources: baseline_sections.iter().map(graph_source_witness).collect(),
        final_sources: case.sections.iter().map(graph_source_witness).collect(),
        changed_modules: changed_modules.clone(),
        deleted_modules: deleted_modules.clone(),
        added_modules: final_modules
            .difference(&baseline_modules)
            .cloned()
            .collect(),
        baseline_cache_modules: offline_module_identities(baseline_observation),
        final_cache_modules: offline_module_identities(final_observation),
    })
}

fn validate_observed_graph_transition(
    witness: &OfflineGraphTransitionWitnessV1,
) -> Result<(), OfflineQualificationErrorV1> {
    let baseline: BTreeMap<_, _> = witness
        .baseline_cache_modules
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();
    let final_modules: BTreeMap<_, _> = witness
        .final_cache_modules
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();
    let changed: BTreeSet<_> = witness.changed_modules.iter().map(String::as_str).collect();
    let deleted: BTreeSet<_> = witness.deleted_modules.iter().map(String::as_str).collect();
    let added: BTreeSet<_> = witness.added_modules.iter().map(String::as_str).collect();
    if changed
        .iter()
        .any(|name| deleted.contains(name) || added.contains(name))
        || deleted.iter().any(|name| added.contains(name))
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "graph changed/deleted/added module sets must be disjoint",
        ));
    }
    for (&name, baseline_module) in &baseline {
        match final_modules.get(name) {
            None if deleted.contains(name) => {}
            Some(final_module) if changed.contains(name) => {
                if baseline_module.map_key != final_module.map_key
                    || baseline_module.semantic_sha256 == final_module.semantic_sha256
                {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "changed graph module must preserve its map identity and change its complete semantic digest",
                    ));
                }
            }
            Some(final_module) if *baseline_module == *final_module => {}
            _ => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "graph transition changed or removed an unlisted baseline module",
                ));
            }
        }
    }
    for (&name, final_module) in &final_modules {
        match baseline.get(name) {
            None if added.contains(name) => {}
            Some(baseline_module) if changed.contains(name) => {
                debug_assert_eq!(baseline_module.map_key, final_module.map_key);
            }
            Some(_) => {}
            None => {
                return Err(OfflineQualificationErrorV1::InvalidManifest(
                    "graph transition added an unlisted module",
                ));
            }
        }
    }
    for name in &changed {
        if !baseline.contains_key(name) || !final_modules.contains_key(name) {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "listed changed graph module is not present in both observed caches",
            ));
        }
    }
    for name in &deleted {
        if !baseline.contains_key(name) || final_modules.contains_key(name) {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "listed deleted graph module is not baseline-only",
            ));
        }
    }
    for name in &added {
        if baseline.contains_key(name) || !final_modules.contains_key(name) {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "listed added graph module is not final-only",
            ));
        }
    }
    Ok(())
}

fn graph_source_witness(section: &ProbeSourceSectionV1) -> OfflineGraphSourceWitnessV1 {
    OfflineGraphSourceWitnessV1 {
        module: section.module.clone(),
        relative_path: section.relative_path.clone(),
        source_sha256: section.source_sha256,
    }
}

fn validate_full_suite_diagnostic_coverage(
    entries: &[OfflineCompilerProbeArtifactEntryV1],
) -> Result<(), OfflineQualificationErrorV1> {
    let warning = entries
        .iter()
        .find(|entry| entry.case_id == "negative.diagnostics.located-warning-as-error")
        .ok_or(OfflineQualificationErrorV1::CorpusMismatch)?;
    if !warning.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == super::qualification::ProbeDiagnosticSeverityV1::Warning
            && diagnostic.section.is_some()
            && diagnostic.row.is_some()
            && diagnostic.column.is_some()
    }) {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "full suite requires a located warning diagnostic",
        ));
    }
    if !warning.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == super::qualification::ProbeDiagnosticSeverityV1::Error
    }) {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "full suite requires the captured warnings-as-errors policy to reject its warning probe",
        ));
    }
    let info = entries
        .iter()
        .find(|entry| entry.case_id == "negative.overloads.ambiguous")
        .ok_or(OfflineQualificationErrorV1::CorpusMismatch)?;
    if !info.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == super::qualification::ProbeDiagnosticSeverityV1::Info
            && diagnostic.section.is_some()
            && diagnostic.row.is_some()
            && diagnostic.column.is_some()
    }) {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "full suite requires located overload-context info diagnostics",
        ));
    }
    let error = entries
        .iter()
        .find(|entry| entry.case_id == "negative.unsupported.try-catch")
        .ok_or(OfflineQualificationErrorV1::CorpusMismatch)?;
    if !error.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == super::qualification::ProbeDiagnosticSeverityV1::Error
            && diagnostic.section.is_some()
            && diagnostic.row.is_some()
            && diagnostic.column.is_some()
    }) {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "full suite requires a located error diagnostic",
        ));
    }
    Ok(())
}

impl CompilerProbeBackendV1 for OfflineCompilerProbeArtifactBackendV1 {
    fn execute_probe(
        &mut self,
        case: &CompilerProbeCaseV1,
    ) -> Result<CompilerProbeObservationV1, CompilerProbeBackendErrorV1> {
        match self.observations.pop_front() {
            Some((case_id, observation)) if case_id == case.case_id => Ok(observation),
            _ => Err(CompilerProbeBackendErrorV1::static_internal(
                "offline artifact order or coverage mismatch",
            )),
        }
    }
}

/// Execute one complete already-authorized capture adapter, derive every cache witness locally,
/// seal the manifest, and immediately prove that the result survives the strict offline loader.
pub fn capture_and_seal_offline_qualification_artifacts_v1(
    corpus: &CompilerProbeCorpusV1,
    backend_kind: CompilerProbeBackendKindV1,
    backend: &mut dyn OfflineQualificationCaptureBackendV1,
) -> Result<GeneratedOfflineCompilerProbeArtifactsV1, OfflineQualificationErrorV1> {
    validate_canonical_full_qualification_corpus_v1(corpus)?;
    let source_profile_sha256 = backend.source_profile_sha256();
    let source_target = backend.source_target();
    let standalone_compiler = backend.standalone_compiler_identity();
    validate_capture_authority(
        backend_kind,
        source_profile_sha256,
        &source_target,
        standalone_compiler,
    )?;
    let mut cache_blobs = BTreeMap::new();
    let mut entries = Vec::new();
    entries
        .try_reserve(corpus.cases.len())
        .map_err(|_| OfflineQualificationErrorV1::AllocationFailed)?;
    for case in &corpus.cases {
        if !capture_authority_matches(
            backend,
            source_profile_sha256,
            &source_target,
            standalone_compiler,
        ) {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "capture backend authority changed during the sealed run",
            ));
        }
        let captured = backend.capture_probe(case).map_err(|error| {
            OfflineQualificationErrorV1::CaptureBackend {
                case_id: case.case_id.clone(),
                detail: error.detail().to_owned(),
            }
        })?;
        if !capture_authority_matches(
            backend,
            source_profile_sha256,
            &source_target,
            standalone_compiler,
        ) {
            return Err(OfflineQualificationErrorV1::InvalidManifest(
                "capture backend authority changed during the sealed run",
            ));
        }
        let (frontend_coverage, compiler_build_flags) = match captured.supplemental {
            OfflineCapturedSupplementalWitnessV1::None => (None, None),
            OfflineCapturedSupplementalWitnessV1::Frontend(value) => (Some(value), None),
            OfflineCapturedSupplementalWitnessV1::CompilerBuildFlags(value) => (None, Some(value)),
        };
        let graph_baseline_cache_bytes = captured.graph_baseline_cache_bytes;
        let observation = captured.observation;
        let entry = match observation.outcome() {
            ProbeOutcomeV1::Accepted => {
                let artifact = observation.accepted_artifact().ok_or(
                    OfflineQualificationErrorV1::InvalidManifest(
                        "accepted capture omitted cache bytes",
                    ),
                )?;
                let invoke_return = artifact
                    .invoke_return()
                    .map(offline_invoke_return_from_observer);
                let semantic = observe_whole_cache_semantics_v1(
                    artifact.cache_bytes(),
                    artifact.invoke_return(),
                )
                .map_err(|error| {
                    OfflineQualificationErrorV1::SemanticObservation {
                        case_id: case.case_id.clone(),
                        detail: error.to_string(),
                    }
                })?;
                let blob_id = format!(
                    "{}.{:04}.cache",
                    match backend_kind {
                        CompilerProbeBackendKindV1::EmbeddedGame => "embedded",
                        CompilerProbeBackendKindV1::Standalone => "standalone",
                    },
                    case.ordinal
                );
                let bytes = artifact.cache_bytes().to_vec();
                let cache = seal_cache_artifact(&blob_id, &bytes);
                if cache_blobs.insert(blob_id, bytes).is_some() {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "capture generator produced a duplicate blob id",
                    ));
                }
                let graph_transition = match (&case.mode, graph_baseline_cache_bytes) {
                    (ProbeModeV1::CompileGraphTransition { .. }, Some(baseline_cache_bytes)) => {
                        let baseline_blob_id = format!("{:04}.graph-baseline.cache", case.ordinal);
                        let baseline_cache =
                            seal_cache_artifact(&baseline_blob_id, &baseline_cache_bytes);
                        let baseline_semantics =
                            observe_whole_cache_semantics_v1(&baseline_cache_bytes, None).map_err(
                                |error| OfflineQualificationErrorV1::SemanticObservation {
                                    case_id: case.case_id.clone(),
                                    detail: format!("graph baseline: {error}"),
                                },
                            )?;
                        if cache_blobs
                            .insert(baseline_blob_id, baseline_cache_bytes)
                            .is_some()
                        {
                            return Err(OfflineQualificationErrorV1::InvalidManifest(
                                "capture generator produced a duplicate graph baseline blob id",
                            ));
                        }
                        Some(
                            graph_transition_witness_for_case(
                                case,
                                baseline_cache,
                                &baseline_semantics,
                                &semantic,
                            )
                            .ok_or(
                                OfflineQualificationErrorV1::InvalidManifest(
                                    "graph capture did not match a graph-transition corpus case",
                                ),
                            )?,
                        )
                    }
                    (ProbeModeV1::CompileGraphTransition { .. }, None) => {
                        return Err(OfflineQualificationErrorV1::InvalidManifest(
                            "accepted graph transition capture omitted raw baseline cache bytes",
                        ));
                    }
                    (_, Some(_)) => {
                        return Err(OfflineQualificationErrorV1::InvalidManifest(
                            "non-graph capture supplied graph baseline cache bytes",
                        ));
                    }
                    (_, None) => None,
                };
                OfflineCompilerProbeArtifactEntryV1 {
                    ordinal: case.ordinal,
                    case_id: case.case_id.clone(),
                    outcome: ProbeOutcomeV1::Accepted,
                    diagnostics: observation.diagnostics().to_vec(),
                    cache: Some(cache),
                    cache_semantics: Some(OfflineCacheSemanticWitnessV1::from_observation(
                        &semantic,
                    )),
                    invoke_return,
                    frontend_coverage,
                    graph_transition,
                    compiler_build_flags,
                }
            }
            ProbeOutcomeV1::Rejected => {
                if graph_baseline_cache_bytes.is_some() {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "rejected capture supplied graph baseline cache bytes",
                    ));
                }
                if !matches!(
                    (frontend_coverage.as_ref(), compiler_build_flags.as_ref()),
                    (None, None)
                ) {
                    return Err(OfflineQualificationErrorV1::InvalidManifest(
                        "rejected capture supplied accepted-only supplemental evidence",
                    ));
                }
                OfflineCompilerProbeArtifactEntryV1 {
                    ordinal: case.ordinal,
                    case_id: case.case_id.clone(),
                    outcome: ProbeOutcomeV1::Rejected,
                    diagnostics: observation.diagnostics().to_vec(),
                    cache: None,
                    cache_semantics: None,
                    invoke_return: None,
                    frontend_coverage: None,
                    graph_transition: None,
                    compiler_build_flags: None,
                }
            }
        };
        entries.push(entry);
    }
    if !capture_authority_matches(
        backend,
        source_profile_sha256,
        &source_target,
        standalone_compiler,
    ) {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "capture backend authority changed during the sealed run",
        ));
    }
    let mut manifest = OfflineCompilerProbeArtifactManifestV1 {
        schema: OFFLINE_PROBE_ARTIFACT_SCHEMA_V1.into(),
        schema_version: OFFLINE_PROBE_ARTIFACT_SCHEMA_VERSION_V1,
        semantic_observer: SEMANTIC_OBSERVER_CONTRACT_V1.into(),
        suite_id: corpus.suite_id.clone(),
        corpus_sha256: corpus.canonical_sha256,
        backend: backend_kind,
        source_profile_sha256,
        source_target,
        standalone_compiler,
        entries,
        canonical_sha256: zero_digest(),
    };
    manifest.seal()?;
    let manifest_json = manifest.to_json()?;
    OfflineCompilerProbeArtifactBackendV1::load(
        corpus,
        backend_kind,
        &manifest_json,
        cache_blobs.clone(),
    )?;
    Ok(GeneratedOfflineCompilerProbeArtifactsV1 {
        manifest,
        manifest_json,
        cache_blobs,
    })
}

/// Promotion gate: derive the golden expected results from the generated embedded capture, run
/// both generated authorities, and refuse to return a promotable payload on any difference.
pub fn promote_generated_offline_qualification_artifacts_v1(
    corpus: &CompilerProbeCorpusV1,
    embedded: &GeneratedOfflineCompilerProbeArtifactsV1,
    standalone: &GeneratedOfflineCompilerProbeArtifactsV1,
) -> Result<OfflineQualificationPromotionV1, OfflineQualificationErrorV1> {
    validate_canonical_full_qualification_corpus_v1(corpus)?;
    if embedded.backend() != CompilerProbeBackendKindV1::EmbeddedGame
        || standalone.backend() != CompilerProbeBackendKindV1::Standalone
    {
        return Err(OfflineQualificationErrorV1::CorpusMismatch);
    }
    if embedded.manifest.source_profile_sha256 != standalone.manifest.source_profile_sha256
        || embedded.manifest.source_target != standalone.manifest.source_target
        || embedded.manifest.standalone_compiler.is_some()
    {
        return Err(OfflineQualificationErrorV1::CorpusMismatch);
    }
    let standalone_compiler = standalone
        .manifest
        .standalone_compiler
        .ok_or(OfflineQualificationErrorV1::CorpusMismatch)?;
    standalone_compiler.validate()?;
    let supplemental_differences: Vec<_> = corpus
        .cases
        .iter()
        .zip(&embedded.manifest.entries)
        .zip(&standalone.manifest.entries)
        .filter_map(|((case, embedded), standalone)| {
            (embedded.frontend_coverage != standalone.frontend_coverage
                || !graph_transition_semantics_match(
                    embedded.graph_transition.as_ref(),
                    standalone.graph_transition.as_ref(),
                )
                || embedded.compiler_build_flags != standalone.compiler_build_flags)
                .then(|| format!("{}:supplemental-artifacts:standalone", case.case_id))
        })
        .collect();
    if !supplemental_differences.is_empty() {
        return Err(OfflineQualificationErrorV1::PromotionRejected(
            supplemental_differences,
        ));
    }
    let expected_results = expected_results_from_generated_embedded(corpus, embedded)?;
    let differential = run_offline_artifact_differential_qualification_v1(
        corpus,
        &expected_results,
        embedded.manifest_json(),
        embedded.cache_blobs().clone(),
        standalone.manifest_json(),
        standalone.cache_blobs().clone(),
    )?;
    if !differential.qualified() {
        return Err(OfflineQualificationErrorV1::PromotionRejected(
            differential.semantic_parity.unexplained_differences.clone(),
        ));
    }
    Ok(OfflineQualificationPromotionV1 {
        source_profile_sha256: embedded.manifest.source_profile_sha256,
        source_target: embedded.manifest.source_target.clone(),
        standalone_compiler,
        expected_results,
        differential,
        embedded_artifacts: embedded.clone(),
        standalone_artifacts: standalone.clone(),
    })
}

fn graph_transition_semantics_match(
    embedded: Option<&OfflineGraphTransitionWitnessV1>,
    standalone: Option<&OfflineGraphTransitionWitnessV1>,
) -> bool {
    match (embedded, standalone) {
        (None, None) => true,
        (Some(embedded), Some(standalone)) => {
            // FAngelscriptPrecompiledData constructs DataGuid with FGuid::NewGuid(), so two
            // correct compiler runs cannot have the same raw baseline-cache seal. Both artifact
            // sets have already been reloaded through the strict blob-seal and whole-cache
            // observer boundary above. Compare the complete normalized graph authority here,
            // while deliberately excluding only the opaque raw cache seal (DataGuid and runtime
            // reference IDs).
            embedded.baseline_cache_semantics == standalone.baseline_cache_semantics
                && embedded.baseline_sources == standalone.baseline_sources
                && embedded.final_sources == standalone.final_sources
                && embedded.changed_modules == standalone.changed_modules
                && embedded.deleted_modules == standalone.deleted_modules
                && embedded.added_modules == standalone.added_modules
                && embedded.baseline_cache_modules == standalone.baseline_cache_modules
                && embedded.final_cache_modules == standalone.final_cache_modules
        }
        _ => false,
    }
}

fn validate_capture_authority(
    backend: CompilerProbeBackendKindV1,
    source_profile_sha256: Sha256Digest,
    source_target: &CompilerTargetV1,
    standalone_compiler: Option<QualifiedSidecarIdentityV1>,
) -> Result<(), OfflineQualificationErrorV1> {
    if source_profile_sha256 == zero_digest()
        || source_target.steam_app_id == 0
        || source_target.steam_build_id == 0
        || source_target.depot_id == 0
        || source_target.depot_manifest_gid == 0
    {
        return Err(OfflineQualificationErrorV1::InvalidManifest(
            "capture source profile and target authority must be nonzero",
        ));
    }
    match (backend, standalone_compiler) {
        (CompilerProbeBackendKindV1::EmbeddedGame, None) => Ok(()),
        (CompilerProbeBackendKindV1::Standalone, Some(identity)) => {
            identity.validate()?;
            Ok(())
        }
        _ => Err(OfflineQualificationErrorV1::InvalidManifest(
            "capture runner identity does not match the selected backend",
        )),
    }
}

fn capture_authority_matches(
    backend: &dyn OfflineQualificationCaptureBackendV1,
    source_profile_sha256: Sha256Digest,
    source_target: &CompilerTargetV1,
    standalone_compiler: Option<QualifiedSidecarIdentityV1>,
) -> bool {
    backend.source_profile_sha256() == source_profile_sha256
        && backend.source_target() == *source_target
        && backend.standalone_compiler_identity() == standalone_compiler
}

fn expected_results_from_generated_embedded(
    corpus: &CompilerProbeCorpusV1,
    embedded: &GeneratedOfflineCompilerProbeArtifactsV1,
) -> Result<ExpectedProbeResultsV1, OfflineQualificationErrorV1> {
    if embedded.manifest.backend != CompilerProbeBackendKindV1::EmbeddedGame
        || embedded.manifest.suite_id != corpus.suite_id
        || embedded.manifest.corpus_sha256 != corpus.canonical_sha256
        || embedded.manifest.entries.len() != corpus.cases.len()
    {
        return Err(OfflineQualificationErrorV1::CorpusMismatch);
    }
    let results = corpus
        .cases
        .iter()
        .zip(&embedded.manifest.entries)
        .map(|(case, entry)| ExpectedProbeResultV1 {
            ordinal: case.ordinal,
            case_id: case.case_id.clone(),
            outcome: entry.outcome,
            diagnostics: entry.diagnostics.clone(),
            semantic_sha256: entry
                .cache_semantics
                .as_ref()
                .map(|witness| witness.semantic_sha256),
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
    expected.seal()?;
    Ok(expected)
}

fn seal_cache_artifact(blob_id: &str, bytes: &[u8]) -> OfflineCacheArtifactSealV1 {
    OfflineCacheArtifactSealV1 {
        blob_id: blob_id.to_owned(),
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn offline_invoke_return_from_observer(
    value: &CanonicalInvokeReturnV1,
) -> OfflineCanonicalInvokeReturnV1 {
    OfflineCanonicalInvokeReturnV1 {
        type_identity: value.type_identity().to_owned(),
        value: offline_invoke_value_from_observer(value.value()),
    }
}

fn offline_invoke_value_from_observer(
    value: &CanonicalInvokeValueV1,
) -> OfflineCanonicalInvokeValueV1 {
    match value {
        CanonicalInvokeValueV1::Null => OfflineCanonicalInvokeValueV1::Null,
        CanonicalInvokeValueV1::Bool(value) => OfflineCanonicalInvokeValueV1::Bool(*value),
        CanonicalInvokeValueV1::I64(value) => OfflineCanonicalInvokeValueV1::I64(*value),
        CanonicalInvokeValueV1::U64(value) => OfflineCanonicalInvokeValueV1::U64(*value),
        CanonicalInvokeValueV1::F32Bits(value) => OfflineCanonicalInvokeValueV1::F32Bits(*value),
        CanonicalInvokeValueV1::F64Bits(value) => OfflineCanonicalInvokeValueV1::F64Bits(*value),
        CanonicalInvokeValueV1::Utf8(value) => OfflineCanonicalInvokeValueV1::Utf8(value.clone()),
        CanonicalInvokeValueV1::Bytes(value) => OfflineCanonicalInvokeValueV1::Bytes(value.clone()),
        CanonicalInvokeValueV1::Sequence(values) => OfflineCanonicalInvokeValueV1::Sequence(
            values
                .iter()
                .map(offline_invoke_value_from_observer)
                .collect(),
        ),
        CanonicalInvokeValueV1::Record(fields) => OfflineCanonicalInvokeValueV1::Record(
            fields
                .iter()
                .map(|(name, value)| OfflineCanonicalInvokeFieldV1 {
                    name: name.clone(),
                    value: offline_invoke_value_from_observer(value),
                })
                .collect(),
        ),
    }
}

/// Load two sealed offline artifact sets and emit the normal differential parity reports.
#[allow(clippy::too_many_arguments)]
pub fn run_offline_artifact_differential_qualification_v1(
    corpus: &CompilerProbeCorpusV1,
    expected: &ExpectedProbeResultsV1,
    embedded_manifest_json: &[u8],
    embedded_cache_blobs: BTreeMap<String, Vec<u8>>,
    standalone_manifest_json: &[u8],
    standalone_cache_blobs: BTreeMap<String, Vec<u8>>,
) -> Result<DifferentialQualificationRunV1, OfflineQualificationErrorV1> {
    let embedded_manifest =
        OfflineCompilerProbeArtifactManifestV1::from_json(embedded_manifest_json)?;
    let standalone_manifest =
        OfflineCompilerProbeArtifactManifestV1::from_json(standalone_manifest_json)?;
    if embedded_manifest.backend != CompilerProbeBackendKindV1::EmbeddedGame
        || standalone_manifest.backend != CompilerProbeBackendKindV1::Standalone
        || embedded_manifest.source_profile_sha256 != standalone_manifest.source_profile_sha256
        || embedded_manifest.source_target != standalone_manifest.source_target
        || embedded_manifest.standalone_compiler.is_some()
    {
        return Err(OfflineQualificationErrorV1::CorpusMismatch);
    }
    let standalone_compiler = standalone_manifest
        .standalone_compiler
        .ok_or(OfflineQualificationErrorV1::CorpusMismatch)?;
    standalone_compiler.validate()?;
    let mut embedded = OfflineCompilerProbeArtifactBackendV1::load(
        corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        embedded_manifest_json,
        embedded_cache_blobs,
    )?;
    let mut standalone = OfflineCompilerProbeArtifactBackendV1::load(
        corpus,
        CompilerProbeBackendKindV1::Standalone,
        standalone_manifest_json,
        standalone_cache_blobs,
    )?;
    Ok(run_differential_qualification_v1(
        corpus,
        expected,
        standalone_compiler,
        &mut embedded,
        &mut standalone,
    )?)
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

#[derive(Debug, thiserror::Error)]
pub enum OfflineQualificationErrorV1 {
    #[error("qualification input is invalid: {0}")]
    Qualification(#[from] QualificationError),
    #[error("offline qualification runner failed: {0}")]
    Runner(#[from] QualificationRunnerErrorV1),
    #[error("offline artifact manifest JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("offline artifact manifest is {actual} bytes; maximum is {max}")]
    ManifestTooLarge { actual: usize, max: usize },
    #[error("offline artifact manifest schema is unsupported")]
    ManifestSchema,
    #[error("offline artifact manifest does not bind the V1 whole-cache observer")]
    ObserverContract,
    #[error("offline artifact manifest seal mismatch")]
    ManifestSealMismatch,
    #[error("offline artifact manifest is invalid: {0}")]
    InvalidManifest(&'static str),
    #[error("offline artifact manifest does not exactly cover the sealed corpus/backend")]
    CorpusMismatch,
    #[error("offline artifact cache blob {0:?} is missing")]
    MissingBlob(String),
    #[error("offline artifact cache blob {0:?} has a mismatched seal")]
    BlobSealMismatch(String),
    #[error("offline artifact cache blob {0:?} is undeclared")]
    UnexpectedBlob(String),
    #[error(
        "offline artifact cache for probe {case_id:?} cannot be semantically observed: {detail}"
    )]
    SemanticObservation { case_id: String, detail: String },
    #[error("offline artifact cache for probe {0:?} does not match its semantic witness")]
    SemanticWitnessMismatch(String),
    #[error("offline capture backend failed for probe {case_id:?}: {detail}")]
    CaptureBackend { case_id: String, detail: String },
    #[error("offline qualification promotion refused differences: {0:?}")]
    PromotionRejected(Vec<String>),
    #[error("allocation failed while loading offline artifacts")]
    AllocationFailed,
}

#[cfg(test)]
pub(crate) mod tests;
