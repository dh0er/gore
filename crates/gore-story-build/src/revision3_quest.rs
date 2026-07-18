//! Source-bound inspection planning for one schema-revision-3 Quest.
//!
//! This module is deliberately separate from the revision-2 `StoryBuildPlan`. Both revision-3 plan
//! variants retain and validate native revision-3 basis projects: Plan V2 reads the version-1
//! artifact, while Plan V3 opens the version-2 artifact and reconstructs its exact immutable
//! historical head through a distinct inspection-only source and capability. Both require fresh
//! source binding before source is regenerated. Neither result grants compilation, build,
//! deployment, runtime, fixed-head, or publication authority.

use std::fmt;

use gore_authoring::{
    regenerate_revision3_quest_module_v2, revision3_quest_input_fingerprint_v2,
    validate_revision3_quest_free_basis, AssetVerification, ContentSeal, DraftQuestSkeletonError,
    EntityId, PreparedRevision3QuestCollisionInspectionSourceV2, ProjectDocument,
    ProjectDocumentError, ProjectId, ProjectRevision3, Revision3Entity, Revision3EntityKind,
    Revision3EntityPayload, Revision3OriginRef, Revision3QuestCollisionSourceErrorV2,
    Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestFreeBasisError,
    Revision3QuestGenerationError, Revision3ScriptModule, Revision3TypedRef, ScriptModuleStatus,
    Sha256Digest, WorkingHead, WorkingProjectStore, WorkingStoreError,
    MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES, MAX_PROJECT_JSON_BYTES,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ENTITY_JSON_BYTES,
    MAX_REVISION3_PRIOR_QUESTS_V2, MAX_REVISION3_SNAPSHOT_BYTES,
    REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION, REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
};
use gore_story_inventory::{
    reopen_quest_collision_capability_artifact_v1, reopen_quest_collision_capability_artifact_v2,
    QuestCollisionCapabilityArtifactError, QuestCollisionCapabilityArtifactErrorV2,
    QuestCollisionCapabilityArtifactV1, QuestCollisionCapabilityArtifactV2,
    QuestCollisionCapabilityArtifactVerificationError, QuestCollisionCapabilityError,
    Revision3QuestCollisionInspectionVerificationErrorV2, VerifiedQuestCollisionCapability,
    VerifiedRevision3QuestCollisionInspectionCapabilityV2,
};
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

const PLAN_FORMAT: &str = "revision3_quest_source_inspection_plan";
const PLAN_SCHEMA_REVISION: u32 = 2;
const PLAN_SCHEMA_REVISION_V3: u32 = 3;
/// The revision-3 project parser and the resulting plan share the frozen project envelope.
pub const MAX_REVISION3_QUEST_PROJECT_JSON_BYTES: usize = MAX_PROJECT_JSON_BYTES;
/// A one-Quest plan repeats at most one bounded entity's source and provenance.
pub const MAX_REVISION3_QUEST_PLAN_JSON_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlanFormat;

impl Serialize for PlanFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(PLAN_FORMAT)
    }
}

impl<'de> Deserialize<'de> for PlanFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == PLAN_FORMAT {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported revision-3 Quest inspection format {value:?}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlanSchemaRevision;

impl Serialize for PlanSchemaRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(PLAN_SCHEMA_REVISION)
    }
}

impl<'de> Deserialize<'de> for PlanSchemaRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == PLAN_SCHEMA_REVISION {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported revision-3 Quest inspection schema revision {value}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlanSchemaRevisionV3;

impl Serialize for PlanSchemaRevisionV3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(PLAN_SCHEMA_REVISION_V3)
    }
}

impl<'de> Deserialize<'de> for PlanSchemaRevisionV3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == PLAN_SCHEMA_REVISION_V3 {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported revision-3 Quest inspection schema revision {value}"
            )))
        }
    }
}

/// Closed scope marker. There is intentionally no compile/build/deploy variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestInspectionScope {
    SourceInspectionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestInspectionBuildStatus {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestInspectionRuntimeQualification {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestInspectionPublicationStatus {
    NotSupported,
}

/// Exact source identities retained by the inspection plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestInspectionProvenance {
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub target_executable: ContentSeal,
    pub canonical_project: ContentSeal,
    pub basis_snapshot: ContentSeal,
    pub canonical_collision_source_project: ContentSeal,
    pub collision_artifact: ContentSeal,
    pub collision_source: ContentSeal,
}

/// Exact regenerated source and the two typed references that own it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestInspectionModule {
    pub quest: Revision3TypedRef,
    pub script_module: Revision3TypedRef,
    pub draft_input: ContentSeal,
    pub persisted_source: ContentSeal,
    pub generated: Revision3ScriptModule,
}

/// Deterministic, sealed source inspection for exactly one revision-3 Quest.
///
/// Every capability marker is permanently fail-closed. This type cannot express compilation,
/// deployment, runtime qualification, or publication readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestSourceInspectionPlanV2 {
    #[serde(rename = "format")]
    pub(crate) format_marker: PlanFormat,
    pub(crate) schema_revision: PlanSchemaRevision,
    pub scope: QuestInspectionScope,
    pub build_status: QuestInspectionBuildStatus,
    pub runtime_qualification: QuestInspectionRuntimeQualification,
    pub publication_status: QuestInspectionPublicationStatus,
    pub provenance: Revision3QuestInspectionProvenance,
    pub module: Revision3QuestInspectionModule,
}

/// Exact production-V2 collision provenance retained by a schema-3 inspection plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestInspectionProvenanceV3 {
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub target_executable: ContentSeal,
    pub canonical_project: ContentSeal,
    pub collision_basis_head: WorkingHead,
    pub collision_basis_project: ContentSeal,
    pub collision_nonquest_project: ContentSeal,
    pub collision_prior_quest_count: u64,
    pub collision_prior_quest_evidence: ContentSeal,
    pub collision_artifact: ContentSeal,
    pub collision_source: ContentSeal,
}

/// Deterministic, inspection-only source plan for current production V2 collision artifacts.
///
/// This is deliberately additive beside [`Revision3QuestSourceInspectionPlanV2`]. It cannot
/// express compilation, deployment, runtime qualification, publication, or fixed-head authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestSourceInspectionPlanV3 {
    #[serde(rename = "format")]
    pub(crate) format_marker: PlanFormat,
    pub(crate) schema_revision: PlanSchemaRevisionV3,
    pub scope: QuestInspectionScope,
    pub build_status: QuestInspectionBuildStatus,
    pub runtime_qualification: QuestInspectionRuntimeQualification,
    pub publication_status: QuestInspectionPublicationStatus,
    pub provenance: Revision3QuestInspectionProvenanceV3,
    pub module: Revision3QuestInspectionModule,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanWire {
    #[serde(rename = "format")]
    format_marker: PlanFormat,
    schema_revision: PlanSchemaRevision,
    scope: QuestInspectionScope,
    build_status: QuestInspectionBuildStatus,
    runtime_qualification: QuestInspectionRuntimeQualification,
    publication_status: QuestInspectionPublicationStatus,
    provenance: Revision3QuestInspectionProvenance,
    module: Revision3QuestInspectionModule,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanWireV3 {
    #[serde(rename = "format")]
    format_marker: PlanFormat,
    schema_revision: PlanSchemaRevisionV3,
    scope: QuestInspectionScope,
    build_status: QuestInspectionBuildStatus,
    runtime_qualification: QuestInspectionRuntimeQualification,
    publication_status: QuestInspectionPublicationStatus,
    provenance: Revision3QuestInspectionProvenanceV3,
    module: Revision3QuestInspectionModule,
}

impl From<PlanWire> for Revision3QuestSourceInspectionPlanV2 {
    fn from(wire: PlanWire) -> Self {
        Self {
            format_marker: wire.format_marker,
            schema_revision: wire.schema_revision,
            scope: wire.scope,
            build_status: wire.build_status,
            runtime_qualification: wire.runtime_qualification,
            publication_status: wire.publication_status,
            provenance: wire.provenance,
            module: wire.module,
        }
    }
}

impl From<PlanWireV3> for Revision3QuestSourceInspectionPlanV3 {
    fn from(wire: PlanWireV3) -> Self {
        Self {
            format_marker: wire.format_marker,
            schema_revision: wire.schema_revision,
            scope: wire.scope,
            build_status: wire.build_status,
            runtime_qualification: wire.runtime_qualification,
            publication_status: wire.publication_status,
            provenance: wire.provenance,
            module: wire.module,
        }
    }
}

impl Revision3QuestSourceInspectionPlanV2 {
    pub const fn format(&self) -> &'static str {
        PLAN_FORMAT
    }

    pub const fn schema_revision(&self) -> u32 {
        PLAN_SCHEMA_REVISION
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestInspectionError> {
        self.validate_closed_invariants()?;
        let json =
            serde_json::to_string(self).map_err(Revision3QuestInspectionError::SerializePlan)?;
        if json.len() > MAX_REVISION3_QUEST_PLAN_JSON_BYTES {
            return Err(Revision3QuestInspectionError::PlanJsonTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_PLAN_JSON_BYTES,
            });
        }
        Ok(json)
    }

    pub fn from_json(json: &str) -> Result<Self, Revision3QuestInspectionError> {
        if json.len() > MAX_REVISION3_QUEST_PLAN_JSON_BYTES {
            return Err(Revision3QuestInspectionError::PlanJsonTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_PLAN_JSON_BYTES,
            });
        }
        let plan = serde_json::from_str::<PlanWire>(json)
            .map(Self::from)
            .map_err(Revision3QuestInspectionError::InvalidPlanJson)?;
        plan.validate_closed_invariants()?;
        if plan.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestInspectionError::NonCanonicalPlanJson);
        }
        Ok(plan)
    }

    pub fn content_seal(&self) -> Result<ContentSeal, Revision3QuestInspectionError> {
        self.to_canonical_json()
            .map(|json| seal_authoring_bytes(json.as_bytes()))
    }

    /// Re-run the complete store/source verification and require exact plan equivalence.
    ///
    /// Reopening plan JSON proves only its closed shape. This method is the authoritative check
    /// when the current project, immutable store, and freshly bound sources are available.
    pub fn verify_against_sources(
        &self,
        store: &WorkingProjectStore,
        canonical_project_json: &str,
        capability: VerifiedQuestCollisionCapability,
    ) -> Result<(), Revision3QuestInspectionError> {
        let expected = prepare_revision3_quest_source_inspection(
            store,
            canonical_project_json,
            self.module.quest.id,
        )?
        .lower(capability)?;
        if &expected != self {
            return Err(Revision3QuestInspectionError::PlanSourceBindingMismatch);
        }
        Ok(())
    }

    fn validate_closed_invariants(&self) -> Result<(), Revision3QuestInspectionError> {
        if self.scope != QuestInspectionScope::SourceInspectionOnly
            || self.build_status != QuestInspectionBuildStatus::Blocked
            || self.runtime_qualification != QuestInspectionRuntimeQualification::RuntimeUnqualified
            || self.publication_status != QuestInspectionPublicationStatus::NotSupported
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "inspection plan contains an authority claim".to_owned(),
            ));
        }
        for (field, seal) in [
            ("target executable", &self.provenance.target_executable),
            ("canonical project", &self.provenance.canonical_project),
            ("basis snapshot", &self.provenance.basis_snapshot),
            (
                "canonical collision source project",
                &self.provenance.canonical_collision_source_project,
            ),
            ("collision artifact", &self.provenance.collision_artifact),
            ("collision source", &self.provenance.collision_source),
            ("draft input", &self.module.draft_input),
            ("persisted source", &self.module.persisted_source),
        ] {
            if seal.byte_len == 0 {
                return Err(Revision3QuestInspectionError::PlanInvariant(format!(
                    "{field} seal is empty"
                )));
            }
        }
        if self.module.draft_input.byte_len
            > u64::try_from(MAX_REVISION3_ENTITY_JSON_BYTES).unwrap_or(u64::MAX)
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "draft input seal exceeds the revision-3 entity envelope".to_owned(),
            ));
        }
        if self.provenance.canonical_project.byte_len
            > u64::try_from(MAX_REVISION3_QUEST_PROJECT_JSON_BYTES).unwrap_or(u64::MAX)
            || self.provenance.canonical_collision_source_project.byte_len
                > u64::try_from(MAX_REVISION3_QUEST_PROJECT_JSON_BYTES).unwrap_or(u64::MAX)
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "canonical project seal exceeds the project envelope".to_owned(),
            ));
        }
        if self.provenance.basis_snapshot.byte_len > MAX_REVISION3_SNAPSHOT_BYTES
            || self.provenance.collision_artifact.byte_len > MAX_QUEST_COLLISION_ARTIFACT_BYTES
            || self.provenance.collision_source.byte_len
                != self.provenance.collision_artifact.byte_len
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "basis or collision artifact seal exceeds its closed envelope".to_owned(),
            ));
        }
        if self.module.quest.project_id != self.provenance.project_id
            || self.module.quest.expected_kind != Revision3EntityKind::QuestDraft
            || self.module.script_module.project_id != self.provenance.project_id
            || self.module.script_module.expected_kind != Revision3EntityKind::ScriptModule
            || self.module.script_module.id == self.module.quest.id
            || self.module.generated.owner != self.module.quest
            || self.module.generated.generator_id != REVISION3_QUEST_GENERATOR_ID
            || !matches!(
                self.module.generated.generator_version,
                REVISION3_QUEST_GENERATOR_VERSION
                    | REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
                    | REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
            )
            || self.module.generated.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "typed owner or generator contract drifted".to_owned(),
            ));
        }
        if self.module.generated.source.len() > MAX_REVISION3_ENTITY_JSON_BYTES {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated source exceeds the revision-3 entity envelope".to_owned(),
            ));
        }
        if self.module.generated.module_namespace.len() > MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES
            || self.module.generated.module_relative_path.len()
                > MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES + 3
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated module identity exceeds the authoring envelope".to_owned(),
            ));
        }
        let expected_relative_path = format!(
            "{}.as",
            self.module.generated.module_namespace.replace('.', "/")
        );
        if self.module.generated.module_relative_path != expected_relative_path {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated module namespace/path identity drifted".to_owned(),
            ));
        }
        let actual_source = seal_authoring_bytes(self.module.generated.source.as_bytes());
        if actual_source != self.module.persisted_source
            || actual_source.sha256 != self.module.generated.source_sha256
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated source seal drifted".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Revision3QuestSourceInspectionPlanV3 {
    pub const fn format(&self) -> &'static str {
        PLAN_FORMAT
    }

    pub const fn schema_revision(&self) -> u32 {
        PLAN_SCHEMA_REVISION_V3
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestInspectionError> {
        self.validate_closed_invariants()?;
        let json =
            serde_json::to_string(self).map_err(Revision3QuestInspectionError::SerializePlan)?;
        if json.len() > MAX_REVISION3_QUEST_PLAN_JSON_BYTES {
            return Err(Revision3QuestInspectionError::PlanJsonTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_PLAN_JSON_BYTES,
            });
        }
        Ok(json)
    }

    pub fn from_json(json: &str) -> Result<Self, Revision3QuestInspectionError> {
        if json.len() > MAX_REVISION3_QUEST_PLAN_JSON_BYTES {
            return Err(Revision3QuestInspectionError::PlanJsonTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_PLAN_JSON_BYTES,
            });
        }
        let plan = serde_json::from_str::<PlanWireV3>(json)
            .map(Self::from)
            .map_err(Revision3QuestInspectionError::InvalidPlanJson)?;
        plan.validate_closed_invariants()?;
        if plan.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestInspectionError::NonCanonicalPlanJson);
        }
        Ok(plan)
    }

    pub fn content_seal(&self) -> Result<ContentSeal, Revision3QuestInspectionError> {
        self.to_canonical_json()
            .map(|json| seal_authoring_bytes(json.as_bytes()))
    }

    /// Re-run exact project/artifact/capability verification and require byte-equivalent output.
    pub fn verify_against_sources(
        &self,
        store: &WorkingProjectStore,
        canonical_project_json: &str,
        capability: VerifiedRevision3QuestCollisionInspectionCapabilityV2,
    ) -> Result<(), Revision3QuestInspectionError> {
        let expected = prepare_revision3_quest_source_inspection_v3(
            store,
            canonical_project_json,
            self.module.quest.id,
        )?
        .lower(capability)?;
        if &expected != self {
            return Err(Revision3QuestInspectionError::PlanSourceBindingMismatch);
        }
        Ok(())
    }

    fn validate_closed_invariants(&self) -> Result<(), Revision3QuestInspectionError> {
        if self.scope != QuestInspectionScope::SourceInspectionOnly
            || self.build_status != QuestInspectionBuildStatus::Blocked
            || self.runtime_qualification != QuestInspectionRuntimeQualification::RuntimeUnqualified
            || self.publication_status != QuestInspectionPublicationStatus::NotSupported
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "inspection plan contains an authority claim".to_owned(),
            ));
        }
        for (field, seal) in [
            ("target executable", &self.provenance.target_executable),
            ("canonical project", &self.provenance.canonical_project),
            (
                "collision basis snapshot",
                &self.provenance.collision_basis_head.snapshot,
            ),
            (
                "collision basis project",
                &self.provenance.collision_basis_project,
            ),
            (
                "collision non-Quest project",
                &self.provenance.collision_nonquest_project,
            ),
            (
                "collision prior-Quest evidence",
                &self.provenance.collision_prior_quest_evidence,
            ),
            ("collision artifact", &self.provenance.collision_artifact),
            ("collision source", &self.provenance.collision_source),
            ("draft input", &self.module.draft_input),
            ("persisted source", &self.module.persisted_source),
        ] {
            if seal.byte_len == 0 {
                return Err(Revision3QuestInspectionError::PlanInvariant(format!(
                    "{field} seal is empty"
                )));
            }
        }
        if self.module.draft_input.byte_len
            > u64::try_from(MAX_REVISION3_ENTITY_JSON_BYTES).unwrap_or(u64::MAX)
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "draft input seal exceeds the revision-3 entity envelope".to_owned(),
            ));
        }
        let project_limit =
            u64::try_from(MAX_REVISION3_QUEST_PROJECT_JSON_BYTES).unwrap_or(u64::MAX);
        if self.provenance.canonical_project.byte_len > project_limit
            || self.provenance.collision_basis_project.byte_len > project_limit
            || self.provenance.collision_nonquest_project.byte_len > project_limit
            || self.provenance.collision_prior_quest_evidence.byte_len > project_limit
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "canonical project seal exceeds the project envelope".to_owned(),
            ));
        }
        if self.provenance.collision_basis_head.snapshot.byte_len > MAX_REVISION3_SNAPSHOT_BYTES
            || self.provenance.collision_artifact.byte_len > MAX_QUEST_COLLISION_ARTIFACT_BYTES
            || self.provenance.collision_source.byte_len
                != self.provenance.collision_artifact.byte_len
            || self.provenance.collision_prior_quest_count
                > u64::try_from(MAX_REVISION3_PRIOR_QUESTS_V2).unwrap_or(u64::MAX)
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "collision basis or artifact exceeds its closed envelope".to_owned(),
            ));
        }
        if self.module.quest.project_id != self.provenance.project_id
            || self.module.quest.expected_kind != Revision3EntityKind::QuestDraft
            || self.module.script_module.project_id != self.provenance.project_id
            || self.module.script_module.expected_kind != Revision3EntityKind::ScriptModule
            || self.module.script_module.id == self.module.quest.id
            || self.module.generated.owner != self.module.quest
            || self.module.generated.generator_id != REVISION3_QUEST_GENERATOR_ID
            || !matches!(
                self.module.generated.generator_version,
                REVISION3_QUEST_GENERATOR_VERSION
                    | REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
                    | REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
            )
            || self.module.generated.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "typed owner or generator contract drifted".to_owned(),
            ));
        }
        if self.module.generated.source.len() > MAX_REVISION3_ENTITY_JSON_BYTES {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated source exceeds the revision-3 entity envelope".to_owned(),
            ));
        }
        if self.module.generated.module_namespace.len() > MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES
            || self.module.generated.module_relative_path.len()
                > MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES + 3
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated module identity exceeds the authoring envelope".to_owned(),
            ));
        }
        let expected_relative_path = format!(
            "{}.as",
            self.module.generated.module_namespace.replace('.', "/")
        );
        if self.module.generated.module_relative_path != expected_relative_path {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated module namespace/path identity drifted".to_owned(),
            ));
        }
        let actual_source = seal_authoring_bytes(self.module.generated.source.as_bytes());
        if actual_source != self.module.persisted_source
            || actual_source.sha256 != self.module.generated.source_sha256
        {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "generated source seal drifted".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Opaque store-verified state. It exposes only the exact native revision-3 collision source
/// project so callers can freshly bind trusted base-game and Story-catalog sources before
/// lowering.
pub struct PreparedRevision3QuestSourceInspection {
    canonical_project_json: String,
    project: ProjectRevision3,
    quest_id: EntityId,
    basis_snapshot: ContentSeal,
    collision_source_project: ProjectRevision3,
    collision_source_project_seal: ContentSeal,
    artifact: QuestCollisionCapabilityArtifactV1,
}

impl fmt::Debug for PreparedRevision3QuestSourceInspection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedRevision3QuestSourceInspection")
            .field("project_id", &self.project.project_id)
            .field("project_revision", &self.project.revision)
            .field("quest_id", &self.quest_id)
            .field("basis_snapshot", &self.basis_snapshot)
            .field(
                "collision_source_project_seal",
                &self.collision_source_project_seal,
            )
            .finish_non_exhaustive()
    }
}

impl PreparedRevision3QuestSourceInspection {
    pub const fn quest_id(&self) -> EntityId {
        self.quest_id
    }

    /// Exact native Quest-free revision-3 basis against which the caller must bind fresh sources.
    pub fn collision_source_project(&self) -> &ProjectRevision3 {
        &self.collision_source_project
    }

    /// Consume the prepared store evidence and one freshly source-bound capability.
    ///
    /// The opaque artifact is never accepted as collision authority. It is used only as the
    /// borrowed exact identity passed to `verify_artifact_exact`; failure consumes both prepared
    /// state and capability and yields no plan.
    pub fn lower(
        self,
        capability: VerifiedQuestCollisionCapability,
    ) -> Result<Revision3QuestSourceInspectionPlanV2, Revision3QuestInspectionError> {
        let capability = capability
            .verify_artifact_exact(&self.artifact)
            .map_err(Revision3QuestInspectionError::ArtifactVerification)?;

        let quest_entity = self
            .project
            .entities
            .get(&self.quest_id)
            .ok_or(Revision3QuestInspectionError::MissingQuest(self.quest_id))?;
        let Revision3EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
            return Err(Revision3QuestInspectionError::NotAQuest(self.quest_id));
        };
        if !capability.authorizes_parent(&quest.input.parent_quest) {
            return Err(Revision3QuestInspectionError::UnauthorizedParent(
                self.quest_id,
            ));
        }
        if !capability.authorizes_giver(&quest.input.giver) {
            return Err(Revision3QuestInspectionError::UnauthorizedGiver(
                self.quest_id,
            ));
        }

        let collision_input = capability
            .into_revision3_quest_collision_input(&self.collision_source_project)
            .map_err(Revision3QuestInspectionError::CollisionAuthority)?;
        let reference = &quest.input.collision_catalog;
        if collision_input.generation != reference.generation
            || collision_input.source_seal != reference.source_seal
            || collision_input.catalog_layer != reference.catalog_layer
        {
            return Err(Revision3QuestInspectionError::ArtifactReferenceDrift {
                quest: self.quest_id,
                field: ArtifactReferenceField::Capability,
            });
        }

        let expected = regenerate_revision3_quest_module(quest, collision_input)?;
        let module_entity = self.project.entities.get(&quest.script_module.id).ok_or(
            Revision3QuestInspectionError::MissingScriptModule {
                quest: self.quest_id,
                module: quest.script_module.id,
            },
        )?;
        let Revision3EntityPayload::ScriptModule(persisted) = &module_entity.payload else {
            return Err(Revision3QuestInspectionError::MissingScriptModule {
                quest: self.quest_id,
                module: quest.script_module.id,
            });
        };
        validate_module_owner(
            self.project.project_id,
            self.quest_id,
            quest,
            module_entity,
            persisted,
        )?;
        if persisted.source_sha256
            != Sha256Digest::from_bytes(Sha256::digest(persisted.source.as_bytes()).into())
        {
            return Err(Revision3QuestInspectionError::PersistedSourceSealMismatch {
                quest: self.quest_id,
                module: quest.script_module.id,
            });
        }
        if persisted != &expected {
            return Err(Revision3QuestInspectionError::PersistedModuleDrift {
                quest: self.quest_id,
                module: quest.script_module.id,
            });
        }

        let input_bytes = serde_json::to_vec(&quest.input)
            .map_err(Revision3QuestInspectionError::SerializeQuestInput)?;
        let artifact_seal = inventory_seal_to_authoring(self.artifact.artifact_seal());
        let artifact_source = inventory_seal_to_authoring(self.artifact.source_seal());
        let plan = Revision3QuestSourceInspectionPlanV2 {
            format_marker: PlanFormat,
            schema_revision: PlanSchemaRevision,
            scope: QuestInspectionScope::SourceInspectionOnly,
            build_status: QuestInspectionBuildStatus::Blocked,
            runtime_qualification: QuestInspectionRuntimeQualification::RuntimeUnqualified,
            publication_status: QuestInspectionPublicationStatus::NotSupported,
            provenance: Revision3QuestInspectionProvenance {
                project_id: self.project.project_id,
                project_revision: self.project.revision,
                target_executable: self.project.target.executable.clone(),
                canonical_project: seal_authoring_bytes(self.canonical_project_json.as_bytes()),
                basis_snapshot: self.basis_snapshot,
                canonical_collision_source_project: self.collision_source_project_seal,
                collision_artifact: artifact_seal,
                collision_source: artifact_source,
            },
            module: Revision3QuestInspectionModule {
                quest: Revision3TypedRef::new(
                    self.project.project_id,
                    self.quest_id,
                    Revision3EntityKind::QuestDraft,
                ),
                script_module: quest.script_module.clone(),
                draft_input: seal_authoring_bytes(&input_bytes),
                persisted_source: seal_authoring_bytes(expected.source.as_bytes()),
                generated: expected,
            },
        };
        plan.validate_closed_invariants()?;
        let canonical = plan.to_canonical_json()?;
        if Revision3QuestSourceInspectionPlanV2::from_json(&canonical)? != plan {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "canonical plan did not reopen exactly".to_owned(),
            ));
        }
        Ok(plan)
    }
}

/// Opaque structural state for one current-production V2-artifact source inspection.
///
/// The historical source is intentionally not retained here. Callers must reconstruct a fresh,
/// inspection-only source through [`Self::prepare_collision_inspection_source`] and consume the
/// resulting inventory capability when lowering.
pub struct PreparedRevision3QuestSourceInspectionV3 {
    canonical_project_json: String,
    project: ProjectRevision3,
    quest_id: EntityId,
    artifact: QuestCollisionCapabilityArtifactV2,
}

impl fmt::Debug for PreparedRevision3QuestSourceInspectionV3 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedRevision3QuestSourceInspectionV3")
            .field("project_id", &self.project.project_id)
            .field("project_revision", &self.project.revision)
            .field("quest_id", &self.quest_id)
            .field("collision_basis_head", self.artifact.current_head())
            .finish_non_exhaustive()
    }
}

impl PreparedRevision3QuestSourceInspectionV3 {
    pub const fn quest_id(&self) -> EntityId {
        self.quest_id
    }

    pub fn collision_basis_head(&self) -> &WorkingHead {
        self.artifact.current_head()
    }

    /// Reconstruct a non-authoritative source capsule for exactly the artifact's immutable basis.
    pub fn prepare_collision_inspection_source(
        &self,
        store: &WorkingProjectStore,
    ) -> Result<PreparedRevision3QuestCollisionInspectionSourceV2, Revision3QuestInspectionError>
    {
        let source = store
            .prepare_revision3_quest_collision_inspection_source_v2(self.artifact.current_head())
            .map_err(Revision3QuestInspectionError::HistoricalInspectionSource)?;
        if source.historical_head() != self.artifact.current_head() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::CurrentHead,
            });
        }
        if source.project_id() != self.artifact.project_id() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::ProjectId,
            });
        }
        if source.project_revision() != self.artifact.project_revision() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::ProjectRevision,
            });
        }
        if source.target() != self.artifact.project_target() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::ProjectTarget,
            });
        }
        if source.historical_project() != self.artifact.current_project() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::CanonicalProject,
            });
        }
        if source.nonquest_basis().canonical_project() != self.artifact.nonquest_project() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::NonQuestProject,
            });
        }
        if source.prior_quest_count_u64() != self.artifact.prior_quest_count() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::PriorQuestCount,
            });
        }
        if source.prior_quest_evidence() != self.artifact.prior_quest_evidence() {
            return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
                field: ArtifactBasisField::PriorQuestEvidence,
            });
        }
        Ok(source)
    }

    /// Consume exact structural state plus a freshly bound historical inspection capability.
    pub fn lower(
        self,
        capability: VerifiedRevision3QuestCollisionInspectionCapabilityV2,
    ) -> Result<Revision3QuestSourceInspectionPlanV3, Revision3QuestInspectionError> {
        let quest_entity = self
            .project
            .entities
            .get(&self.quest_id)
            .ok_or(Revision3QuestInspectionError::MissingQuest(self.quest_id))?;
        let Revision3EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
            return Err(Revision3QuestInspectionError::NotAQuest(self.quest_id));
        };
        let collision_input = capability
            .verify_artifact_for_quest(
                &self.artifact,
                &quest.input.parent_quest,
                &quest.input.giver,
            )
            .map_err(Revision3QuestInspectionError::ArtifactVerificationV2)?;
        let reference = &quest.input.collision_catalog;
        if collision_input.generation != reference.generation
            || collision_input.source_seal != reference.source_seal
            || collision_input.catalog_layer != reference.catalog_layer
        {
            return Err(Revision3QuestInspectionError::ArtifactReferenceDrift {
                quest: self.quest_id,
                field: ArtifactReferenceField::Capability,
            });
        }

        let expected = regenerate_revision3_quest_module(quest, collision_input)?;
        let module_entity = self.project.entities.get(&quest.script_module.id).ok_or(
            Revision3QuestInspectionError::MissingScriptModule {
                quest: self.quest_id,
                module: quest.script_module.id,
            },
        )?;
        let Revision3EntityPayload::ScriptModule(persisted) = &module_entity.payload else {
            return Err(Revision3QuestInspectionError::MissingScriptModule {
                quest: self.quest_id,
                module: quest.script_module.id,
            });
        };
        validate_module_owner(
            self.project.project_id,
            self.quest_id,
            quest,
            module_entity,
            persisted,
        )?;
        if persisted.source_sha256
            != Sha256Digest::from_bytes(Sha256::digest(persisted.source.as_bytes()).into())
        {
            return Err(Revision3QuestInspectionError::PersistedSourceSealMismatch {
                quest: self.quest_id,
                module: quest.script_module.id,
            });
        }
        if persisted != &expected {
            return Err(Revision3QuestInspectionError::PersistedModuleDrift {
                quest: self.quest_id,
                module: quest.script_module.id,
            });
        }

        let input_bytes = serde_json::to_vec(&quest.input)
            .map_err(Revision3QuestInspectionError::SerializeQuestInput)?;
        let plan = Revision3QuestSourceInspectionPlanV3 {
            format_marker: PlanFormat,
            schema_revision: PlanSchemaRevisionV3,
            scope: QuestInspectionScope::SourceInspectionOnly,
            build_status: QuestInspectionBuildStatus::Blocked,
            runtime_qualification: QuestInspectionRuntimeQualification::RuntimeUnqualified,
            publication_status: QuestInspectionPublicationStatus::NotSupported,
            provenance: Revision3QuestInspectionProvenanceV3 {
                project_id: self.project.project_id,
                project_revision: self.project.revision,
                target_executable: self.project.target.executable.clone(),
                canonical_project: seal_authoring_bytes(self.canonical_project_json.as_bytes()),
                collision_basis_head: self.artifact.current_head().clone(),
                collision_basis_project: self.artifact.current_project().clone(),
                collision_nonquest_project: self.artifact.nonquest_project().clone(),
                collision_prior_quest_count: self.artifact.prior_quest_count(),
                collision_prior_quest_evidence: self.artifact.prior_quest_evidence().clone(),
                collision_artifact: inventory_seal_to_authoring(self.artifact.artifact_seal()),
                collision_source: inventory_seal_to_authoring(self.artifact.source_seal()),
            },
            module: Revision3QuestInspectionModule {
                quest: Revision3TypedRef::new(
                    self.project.project_id,
                    self.quest_id,
                    Revision3EntityKind::QuestDraft,
                ),
                script_module: quest.script_module.clone(),
                draft_input: seal_authoring_bytes(&input_bytes),
                persisted_source: seal_authoring_bytes(expected.source.as_bytes()),
                generated: expected,
            },
        };
        plan.validate_closed_invariants()?;
        let canonical = plan.to_canonical_json()?;
        if Revision3QuestSourceInspectionPlanV3::from_json(&canonical)? != plan {
            return Err(Revision3QuestInspectionError::PlanInvariant(
                "canonical plan did not reopen exactly".to_owned(),
            ));
        }
        Ok(plan)
    }
}

/// Open and verify all current-production V2 artifact structure needed for one Quest inspection.
pub fn prepare_revision3_quest_source_inspection_v3(
    store: &WorkingProjectStore,
    canonical_project_json: &str,
    quest_id: EntityId,
) -> Result<PreparedRevision3QuestSourceInspectionV3, Revision3QuestInspectionError> {
    if canonical_project_json.len() > MAX_REVISION3_QUEST_PROJECT_JSON_BYTES {
        return Err(Revision3QuestInspectionError::ProjectJsonTooLarge {
            actual: canonical_project_json.len(),
            limit: MAX_REVISION3_QUEST_PROJECT_JSON_BYTES,
        });
    }
    let document = ProjectDocument::from_json(canonical_project_json)
        .map_err(Revision3QuestInspectionError::InvalidProjectDocument)?;
    let reopened = document
        .to_canonical_json()
        .map_err(Revision3QuestInspectionError::SerializeProject)?;
    if reopened.as_bytes() != canonical_project_json.as_bytes() {
        return Err(Revision3QuestInspectionError::NonCanonicalProjectJson);
    }
    let ProjectDocument::Revision3(project) = document else {
        return Err(Revision3QuestInspectionError::Revision3Required);
    };
    let quest_entity = project
        .entities
        .get(&quest_id)
        .ok_or(Revision3QuestInspectionError::MissingQuest(quest_id))?;
    let Revision3EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        return Err(Revision3QuestInspectionError::NotAQuest(quest_id));
    };
    validate_current_quest(project.project_id, quest_id, quest_entity, quest, &project)?;
    let reference = &quest.input.collision_catalog;
    let artifact_bytes = store
        .read_indexed_quest_collision_artifact_v2(&project.asset_store, &reference.artifact)
        .map_err(
            |source| Revision3QuestInspectionError::ArtifactUnavailable {
                quest: quest_id,
                source,
            },
        )?;
    let artifact = reopen_quest_collision_capability_artifact_v2(
        &artifact_bytes,
        &authoring_seal_to_inventory(&reference.artifact),
        &authoring_seal_to_inventory(&reference.source_seal),
    )
    .map_err(Revision3QuestInspectionError::InvalidArtifactV2)?;

    if artifact.current_head().snapshot != reference.basis_snapshot {
        return Err(Revision3QuestInspectionError::ArtifactReferenceDrift {
            quest: quest_id,
            field: ArtifactReferenceField::BasisSnapshot,
        });
    }
    if artifact.project_id() != project.project_id {
        return Err(Revision3QuestInspectionError::ForeignProject {
            quest: quest_id,
            expected: project.project_id,
            actual: artifact.project_id(),
        });
    }
    if artifact.project_target() != &project.target {
        return Err(Revision3QuestInspectionError::ForeignGeneration {
            quest: quest_id,
            field: "collision artifact basis",
        });
    }
    if artifact.project_revision() >= project.revision {
        return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
            field: ArtifactBasisField::ProjectRevision,
        });
    }
    if artifact.catalog_layer() != reference.catalog_layer {
        return Err(Revision3QuestInspectionError::ArtifactReferenceDrift {
            quest: quest_id,
            field: ArtifactReferenceField::Capability,
        });
    }

    Ok(PreparedRevision3QuestSourceInspectionV3 {
        canonical_project_json: canonical_project_json.to_owned(),
        project,
        quest_id,
        artifact,
    })
}

/// Open and verify all store-backed structural evidence needed for one revision-3 Quest.
///
/// No capability or raw artifact bytes are accepted from the caller. The returned state is still
/// non-authoritative until [`PreparedRevision3QuestSourceInspection::lower`] consumes a freshly
/// source-bound capability and performs exact artifact verification.
pub fn prepare_revision3_quest_source_inspection(
    store: &WorkingProjectStore,
    canonical_project_json: &str,
    quest_id: EntityId,
) -> Result<PreparedRevision3QuestSourceInspection, Revision3QuestInspectionError> {
    if canonical_project_json.len() > MAX_REVISION3_QUEST_PROJECT_JSON_BYTES {
        return Err(Revision3QuestInspectionError::ProjectJsonTooLarge {
            actual: canonical_project_json.len(),
            limit: MAX_REVISION3_QUEST_PROJECT_JSON_BYTES,
        });
    }
    let document = ProjectDocument::from_json(canonical_project_json)
        .map_err(Revision3QuestInspectionError::InvalidProjectDocument)?;
    let reopened = document
        .to_canonical_json()
        .map_err(Revision3QuestInspectionError::SerializeProject)?;
    if reopened.as_bytes() != canonical_project_json.as_bytes() {
        return Err(Revision3QuestInspectionError::NonCanonicalProjectJson);
    }
    let ProjectDocument::Revision3(project) = document else {
        return Err(Revision3QuestInspectionError::Revision3Required);
    };
    let quest_entity = project
        .entities
        .get(&quest_id)
        .ok_or(Revision3QuestInspectionError::MissingQuest(quest_id))?;
    let Revision3EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        return Err(Revision3QuestInspectionError::NotAQuest(quest_id));
    };
    validate_current_quest(project.project_id, quest_id, quest_entity, quest, &project)?;
    let reference = quest.input.collision_catalog.clone();

    let artifact_bytes = store
        .read_indexed_quest_collision_artifact_v1(&project.asset_store, &reference.artifact)
        .map_err(
            |source| Revision3QuestInspectionError::ArtifactUnavailable {
                quest: quest_id,
                source,
            },
        )?;
    let expected_raw = authoring_seal_to_inventory(&reference.artifact);
    let expected_source = authoring_seal_to_inventory(&reference.source_seal);
    let artifact = reopen_quest_collision_capability_artifact_v1(
        &artifact_bytes,
        &expected_raw,
        &expected_source,
    )
    .map_err(Revision3QuestInspectionError::InvalidArtifact)?;

    let basis = store
        .open_revision3_snapshot(&reference.basis_snapshot, AssetVerification::Full)
        .map_err(|source| Revision3QuestInspectionError::BasisUnavailable {
            quest: quest_id,
            source,
        })?;
    if basis.head.snapshot != reference.basis_snapshot {
        return Err(Revision3QuestInspectionError::ArtifactReferenceDrift {
            quest: quest_id,
            field: ArtifactReferenceField::BasisSnapshot,
        });
    }
    if basis.project.project_id != project.project_id {
        return Err(Revision3QuestInspectionError::ForeignProject {
            quest: quest_id,
            expected: project.project_id,
            actual: basis.project.project_id,
        });
    }
    if basis.project.target != project.target {
        return Err(Revision3QuestInspectionError::ForeignGeneration {
            quest: quest_id,
            field: "basis project target",
        });
    }

    validate_revision3_basis(&basis.project)?;
    let collision_source_project = basis.project;
    let collision_source_json = collision_source_project
        .to_canonical_json()
        .map_err(Revision3QuestInspectionError::SerializeCollisionSourceProject)?;
    let collision_source_project_seal = seal_authoring_bytes(collision_source_json.as_bytes());

    if artifact.project_id() != collision_source_project.project_id {
        return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
            field: ArtifactBasisField::ProjectId,
        });
    }
    if artifact.project_revision() != collision_source_project.revision {
        return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
            field: ArtifactBasisField::ProjectRevision,
        });
    }
    if artifact.project_target() != &collision_source_project.target {
        return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
            field: ArtifactBasisField::ProjectTarget,
        });
    }
    if artifact.canonical_project() != &collision_source_project_seal {
        return Err(Revision3QuestInspectionError::ArtifactBasisDrift {
            field: ArtifactBasisField::CanonicalProject,
        });
    }

    Ok(PreparedRevision3QuestSourceInspection {
        canonical_project_json: canonical_project_json.to_owned(),
        project,
        quest_id,
        basis_snapshot: basis.head.snapshot,
        collision_source_project,
        collision_source_project_seal,
        artifact,
    })
}

pub(crate) fn validate_revision3_basis(
    project: &ProjectRevision3,
) -> Result<(), Revision3QuestInspectionError> {
    validate_revision3_quest_free_basis(project).map_err(|error| match error {
        Revision3QuestFreeBasisError::InvalidProject { reason } => {
            Revision3QuestInspectionError::InvalidBasisProject(reason)
        }
        Revision3QuestFreeBasisError::RecursiveQuest { entity } => {
            Revision3QuestInspectionError::RecursiveQuestBasis { entity }
        }
        Revision3QuestFreeBasisError::ResidualQuestState { entity } => {
            Revision3QuestInspectionError::ResidualQuestBasis { entity }
        }
    })
}

fn validate_current_quest(
    project_id: ProjectId,
    quest_id: EntityId,
    quest_entity: &Revision3Entity,
    quest: &Revision3QuestDraft,
    project: &ProjectRevision3,
) -> Result<(), Revision3QuestInspectionError> {
    if quest.generator_id != REVISION3_QUEST_GENERATOR_ID
        || !matches!(
            quest.generator_version,
            REVISION3_QUEST_GENERATOR_VERSION
                | REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
                | REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        )
    {
        return Err(Revision3QuestInspectionError::ForeignGenerator { entity: quest_id });
    }
    if quest.input.target != project.target
        || quest.input.parent_quest.generation != project.target
        || quest.input.giver.generation != project.target
        || quest.input.collision_catalog.generation != project.target
    {
        return Err(Revision3QuestInspectionError::ForeignGeneration {
            quest: quest_id,
            field: "Quest input",
        });
    }
    let Some(module_entity) = project.entities.get(&quest.script_module.id) else {
        return Err(Revision3QuestInspectionError::MissingScriptModule {
            quest: quest_id,
            module: quest.script_module.id,
        });
    };
    let Revision3EntityPayload::ScriptModule(module) = &module_entity.payload else {
        return Err(Revision3QuestInspectionError::MissingScriptModule {
            quest: quest_id,
            module: quest.script_module.id,
        });
    };
    validate_module_owner(project_id, quest_id, quest, module_entity, module)?;
    if !matches!(
        &quest_entity.origin,
        Revision3OriginRef::New { authored_runtime_id }
            if authored_runtime_id == &quest.input.technical_id
    ) {
        return Err(Revision3QuestInspectionError::OwnerMismatch {
            quest: quest_id,
            module: quest.script_module.id,
        });
    }
    Ok(())
}

fn validate_module_owner(
    project_id: ProjectId,
    quest_id: EntityId,
    quest: &Revision3QuestDraft,
    module_entity: &Revision3Entity,
    module: &Revision3ScriptModule,
) -> Result<(), Revision3QuestInspectionError> {
    let owner = Revision3TypedRef::new(project_id, quest_id, Revision3EntityKind::QuestDraft);
    if quest.script_module.project_id != project_id
        || quest.script_module.expected_kind != Revision3EntityKind::ScriptModule
        || module.owner != owner
        || module.generator_id != REVISION3_QUEST_GENERATOR_ID
        || module.generator_version != quest.generator_version
        || module.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        || !matches!(
            &module_entity.origin,
            Revision3OriginRef::Generated {
                generator_id,
                generator_version,
                owner: origin_owner,
            } if generator_id == REVISION3_QUEST_GENERATOR_ID
                && *generator_version == quest.generator_version
                && origin_owner == &owner
        )
    {
        return Err(Revision3QuestInspectionError::OwnerMismatch {
            quest: quest_id,
            module: quest.script_module.id,
        });
    }
    Ok(())
}

pub(crate) fn regenerate_revision3_quest_module(
    quest: &Revision3QuestDraft,
    collision: gore_authoring::QuestCollisionCatalogInput,
) -> Result<Revision3ScriptModule, Revision3QuestInspectionError> {
    regenerate_revision3_quest_module_v2(quest, collision).map_err(map_quest_generation_error)
}

/// Stable v2 fingerprint of the bounded revision-3 Quest intent, including raw, semantic, and
/// basis seals. This is source-generation metadata only and grants no collision authority.
pub fn revision3_quest_input_fingerprint(
    input: &Revision3QuestDraftInput,
) -> Result<Sha256Digest, Revision3QuestInspectionError> {
    revision3_quest_input_fingerprint_v2(input).map_err(map_quest_generation_error)
}

fn map_quest_generation_error(
    error: Revision3QuestGenerationError,
) -> Revision3QuestInspectionError {
    match error {
        Revision3QuestGenerationError::InvalidQuestIntent(error) => {
            Revision3QuestInspectionError::InvalidQuestIntent(error)
        }
        Revision3QuestGenerationError::SerializeQuestInput(error) => {
            Revision3QuestInspectionError::SerializeQuestInput(error)
        }
        error => Revision3QuestInspectionError::SharedQuestGeneration(error),
    }
}

fn seal_authoring_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn authoring_seal_to_inventory(seal: &ContentSeal) -> gore_story_inventory::ContentSeal {
    gore_story_inventory::ContentSeal {
        byte_len: seal.byte_len,
        sha256: gore_story_inventory::Sha256Digest::from_bytes(*seal.sha256.as_bytes()),
    }
}

fn inventory_seal_to_authoring(seal: &gore_story_inventory::ContentSeal) -> ContentSeal {
    ContentSeal {
        byte_len: seal.byte_len,
        sha256: Sha256Digest::from_bytes(*seal.sha256.as_bytes()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactReferenceField {
    BasisSnapshot,
    Capability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactBasisField {
    ProjectId,
    ProjectRevision,
    ProjectTarget,
    CanonicalProject,
    CurrentHead,
    NonQuestProject,
    PriorQuestCount,
    PriorQuestEvidence,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestInspectionError {
    #[error("revision-3 Quest project JSON exceeds the {limit}-byte limit: {actual} bytes")]
    ProjectJsonTooLarge { actual: usize, limit: usize },
    #[error("invalid authoring project document: {0}")]
    InvalidProjectDocument(#[source] ProjectDocumentError),
    #[error("could not canonicalize authoring project: {0}")]
    SerializeProject(#[source] serde_json::Error),
    #[error("revision-3 Quest inspection requires exact canonical project JSON")]
    NonCanonicalProjectJson,
    #[error("revision-3 Quest inspection requires authoring schema revision 3")]
    Revision3Required,
    #[error("revision-3 Quest {0} is absent")]
    MissingQuest(EntityId),
    #[error("revision-3 entity {0} is not a Quest Draft")]
    NotAQuest(EntityId),
    #[error("revision-3 Quest {quest} ScriptModule {module} is missing or mistyped")]
    MissingScriptModule { quest: EntityId, module: EntityId },
    #[error("revision-3 Quest {quest} references foreign project {actual}; expected {expected}")]
    ForeignProject {
        quest: EntityId,
        expected: ProjectId,
        actual: ProjectId,
    },
    #[error("revision-3 Quest {quest} has foreign generation at {field}")]
    ForeignGeneration {
        quest: EntityId,
        field: &'static str,
    },
    #[error("revision-3 entity {entity} has a foreign generator contract")]
    ForeignGenerator { entity: EntityId },
    #[error("revision-3 Quest {quest} / ScriptModule {module} owner or origin mismatch")]
    OwnerMismatch { quest: EntityId, module: EntityId },
    #[error("revision-3 Quest {quest} collision artifact is unavailable: {source}")]
    ArtifactUnavailable {
        quest: EntityId,
        #[source]
        source: WorkingStoreError,
    },
    #[error("invalid revision-3 Quest collision artifact: {0}")]
    InvalidArtifact(#[source] QuestCollisionCapabilityArtifactError),
    #[error("invalid revision-3 Quest collision artifact V2: {0}")]
    InvalidArtifactV2(#[source] QuestCollisionCapabilityArtifactErrorV2),
    #[error("historical revision-3 Quest inspection source is unavailable: {0}")]
    HistoricalInspectionSource(#[source] Revision3QuestCollisionSourceErrorV2),
    #[error("revision-3 Quest {quest} basis snapshot is unavailable: {source}")]
    BasisUnavailable {
        quest: EntityId,
        #[source]
        source: WorkingStoreError,
    },
    #[error("revision-3 Quest {quest} artifact reference drift at {field:?}")]
    ArtifactReferenceDrift {
        quest: EntityId,
        field: ArtifactReferenceField,
    },
    #[error("revision-3 basis recursively contains Quest {entity}")]
    RecursiveQuestBasis { entity: EntityId },
    #[error("revision-3 basis contains residual Quest generator state at {entity}")]
    ResidualQuestBasis { entity: EntityId },
    #[error("revision-3 Quest-free basis is not a closed valid project: {0}")]
    InvalidBasisProject(String),
    #[error("could not serialize the native Quest-free revision-3 collision source project: {0}")]
    SerializeCollisionSourceProject(#[source] gore_authoring::ProjectRevision3JsonError),
    #[error("collision artifact basis drift at {field:?}")]
    ArtifactBasisDrift { field: ArtifactBasisField },
    #[error("fresh source-bound capability does not exactly verify the stored artifact: {0}")]
    ArtifactVerification(#[source] QuestCollisionCapabilityArtifactVerificationError),
    #[error(
        "fresh inspection-only capability does not exactly verify the stored artifact V2: {0}"
    )]
    ArtifactVerificationV2(#[source] Revision3QuestCollisionInspectionVerificationErrorV2),
    #[error("fresh source-bound capability does not authorize Quest {0} parent identity")]
    UnauthorizedParent(EntityId),
    #[error("fresh source-bound capability does not authorize Quest {0} giver identity")]
    UnauthorizedGiver(EntityId),
    #[error("fresh source-bound collision capability does not match the exact basis: {0}")]
    CollisionAuthority(#[source] QuestCollisionCapabilityError),
    #[error("invalid revision-3 Quest generator intent: {0}")]
    InvalidQuestIntent(#[source] DraftQuestSkeletonError),
    #[error("could not serialize revision-3 Quest input: {0}")]
    SerializeQuestInput(#[source] serde_json::Error),
    #[error("shared revision-3 Quest generator rejected a validated persisted draft: {0}")]
    SharedQuestGeneration(#[source] Revision3QuestGenerationError),
    #[error("revision-3 Quest {quest} ScriptModule {module} source seal mismatch")]
    PersistedSourceSealMismatch { quest: EntityId, module: EntityId },
    #[error(
        "revision-3 Quest {quest} persisted ScriptModule {module} differs from exact lowering"
    )]
    PersistedModuleDrift { quest: EntityId, module: EntityId },
    #[error("could not serialize revision-3 Quest inspection plan: {0}")]
    SerializePlan(#[source] serde_json::Error),
    #[error("invalid revision-3 Quest inspection plan JSON: {0}")]
    InvalidPlanJson(#[source] serde_json::Error),
    #[error("revision-3 Quest inspection plan JSON is not canonical")]
    NonCanonicalPlanJson,
    #[error("revision-3 Quest inspection plan exceeds {limit} bytes: {actual}")]
    PlanJsonTooLarge { actual: usize, limit: usize },
    #[error("revision-3 Quest inspection invariant failed: {0}")]
    PlanInvariant(String),
    #[error("revision-3 Quest inspection plan does not match freshly verified sources")]
    PlanSourceBindingMismatch,
}
