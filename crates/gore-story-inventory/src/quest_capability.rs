//! Closed base-game plus exact-project Quest collision capability.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};

use gore_authoring::{
    collect_project_story_collision_identities, collect_revision3_story_collision_identities,
    ContentSeal as AuthoringContentSeal, EntityId, GameGenerationAnchor, ProjectId,
    ProjectRevision2, ProjectRevision3, ProjectStoryCollisionIdentities,
    QuestCollisionCatalogInput, Revision2QuestGiverInput as QuestGiverInput,
    Revision2QuestParentInput as QuestParentInput, Sha256Digest as AuthoringSha256Digest,
    StoryCollisionCollectionError, MAX_PROJECT_JSON_BYTES,
};
use gore_story_catalog::{CatalogError, StoryCatalogFile, MAX_CATALOG_JSON_BYTES};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

use super::{
    BaseGameCollisionInventory, ContentSeal, MAX_COLLISION_ENTRIES, MAX_COLLISION_ENTRY_BYTES,
    MAX_COLLISION_TOTAL_BYTES, MAX_INVENTORY_JSON_BYTES,
};

/// Honest layer identity: pristine base game plus one exact canonical authoring project only.
pub const BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER: &str =
    "base-game-plus-exact-project.story-collisions.v1";
const COMBINED_SEAL_DOMAIN: &[u8] =
    b"gore-story-inventory.quest-collision-capability.v1.combined-payload\0";
const COMBINED_FORMAT: &str = "quest_collision_capability";
const COMBINED_SCHEMA_REVISION: u32 = 1;
const MAX_COMBINED_MARKER_BYTES: usize = 128;
const MAX_JSON_STRING_TOKEN_BYTES: usize = MAX_COLLISION_ENTRY_BYTES * 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionCoverage {
    BaseGameAndExactProjectOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionRuntimeQualification {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionBuildStatus {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionPublicationStatus {
    NotSupported,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedQuestCollisionCapability {
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    canonical_project: AuthoringContentSeal,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    combined_source_seal: ContentSeal,
    modules: BTreeSet<String>,
    relative_paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
    parents: BTreeMap<String, QuestParentInput>,
    givers: BTreeMap<String, QuestGiverInput>,
}

/// Opaque, content-addressable form of one verified base-game plus exact-project collision set.
///
/// The retained bytes are the exact canonical [`CombinedPayload`] JSON. `artifact_seal` is the
/// ordinary SHA-256 content address used by a future project asset store; `source_seal` is the
/// existing domain-separated semantic seal. Neither seal is runtime, build, deployment, or
/// publication evidence.
pub struct QuestCollisionCapabilityArtifactV1 {
    canonical_json: Vec<u8>,
    artifact_seal: ContentSeal,
    source_seal: ContentSeal,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    canonical_project: AuthoringContentSeal,
}

/// Linear bridge between one freshly source-bound collision capability and its exact structural
/// artifact.
///
/// This capsule is opaque and deliberately does not implement [`Clone`]. It retains the original
/// capability (including its one owned collision-set representation) next to one canonical JSON
/// byte vector bounded by [`MAX_INVENTORY_JSON_BYTES`]. Preparing it never creates a second copy
/// of the module, relative-path, or symbol sets. The borrowed [`Self::artifact`] view is
/// structural evidence only. The capsule can be consumed solely through [`Self::finalize`] into
/// freshly source-bound generation input; that plain result type does not itself grant artifact,
/// build, runtime, or publication authority.
pub struct PreparedQuestCollisionArtifactV1 {
    capability: VerifiedQuestCollisionCapability,
    artifact: QuestCollisionCapabilityArtifactV1,
}

impl fmt::Debug for QuestCollisionCapabilityArtifactV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuestCollisionCapabilityArtifactV1")
            .field("canonical_json_bytes", &self.canonical_json.len())
            .field("artifact_seal", &self.artifact_seal)
            .field("source_seal", &self.source_seal)
            .field(
                "base_inventory_payload_seal",
                &self.base_inventory_payload_seal,
            )
            .field("story_catalog_seal", &self.story_catalog_seal)
            .field("project_id", &self.project_id)
            .field("project_revision", &self.project_revision)
            .field("project_target", &self.project_target)
            .field("canonical_project", &self.canonical_project)
            .finish()
    }
}

impl PartialEq for QuestCollisionCapabilityArtifactV1 {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_json == other.canonical_json
            && self.artifact_seal == other.artifact_seal
            && self.source_seal == other.source_seal
            && self.base_inventory_payload_seal == other.base_inventory_payload_seal
            && self.story_catalog_seal == other.story_catalog_seal
            && self.project_id == other.project_id
            && self.project_revision == other.project_revision
            && self.project_target == other.project_target
            && self.canonical_project == other.canonical_project
    }
}

impl Eq for QuestCollisionCapabilityArtifactV1 {}

impl QuestCollisionCapabilityArtifactV1 {
    pub fn canonical_json(&self) -> &[u8] {
        &self.canonical_json
    }

    pub fn into_canonical_json(self) -> Vec<u8> {
        self.canonical_json
    }

    pub fn artifact_seal(&self) -> &ContentSeal {
        &self.artifact_seal
    }

    pub fn source_seal(&self) -> &ContentSeal {
        &self.source_seal
    }

    pub fn base_inventory_payload_seal(&self) -> &ContentSeal {
        &self.base_inventory_payload_seal
    }

    pub fn story_catalog_seal(&self) -> &ContentSeal {
        &self.story_catalog_seal
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn project_revision(&self) -> u64 {
        self.project_revision
    }

    pub fn project_target(&self) -> &GameGenerationAnchor {
        &self.project_target
    }

    pub fn canonical_project(&self) -> &AuthoringContentSeal {
        &self.canonical_project
    }

    pub const fn catalog_layer(&self) -> &'static str {
        BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
    }

    pub const fn coverage(&self) -> QuestCollisionCoverage {
        QuestCollisionCoverage::BaseGameAndExactProjectOnly
    }

    pub const fn runtime_qualification(&self) -> QuestCollisionRuntimeQualification {
        QuestCollisionRuntimeQualification::RuntimeUnqualified
    }

    pub const fn build_status(&self) -> QuestCollisionBuildStatus {
        QuestCollisionBuildStatus::Blocked
    }

    pub const fn publication_status(&self) -> QuestCollisionPublicationStatus {
        QuestCollisionPublicationStatus::NotSupported
    }
}

impl PreparedQuestCollisionArtifactV1 {
    /// Borrow the exact structural artifact without exposing or duplicating the retained
    /// source-bound capability.
    pub const fn artifact(&self) -> &QuestCollisionCapabilityArtifactV1 {
        &self.artifact
    }

    /// Re-verify the retained capability against its own exact artifact, then consume that
    /// capability against the supplied exact project head.
    ///
    /// Both success and failure consume the capsule. The returned artifact remains structural.
    /// The separately returned collision input was derived from the retained fresh capability,
    /// but its plain public type is not authority evidence and grants no build, runtime, artifact,
    /// or publication claim.
    pub fn finalize(
        self,
        exact_project: &ProjectRevision2,
    ) -> Result<
        (
            QuestCollisionCapabilityArtifactV1,
            QuestCollisionCatalogInput,
        ),
        PreparedQuestCollisionArtifactFinalizeError,
    > {
        let Self {
            capability,
            artifact,
        } = self;
        let capability = capability
            .verify_artifact_exact(&artifact)
            .map_err(PreparedQuestCollisionArtifactFinalizeError::ArtifactOrCapabilityDrift)?;
        let collision_input = capability
            .into_quest_collision_input(exact_project)
            .map_err(PreparedQuestCollisionArtifactFinalizeError::Project)?;
        Ok((artifact, collision_input))
    }
}

impl VerifiedQuestCollisionCapability {
    /// Bind closed base/catalog capabilities to one exact canonical project snapshot.
    ///
    /// The verified base inventory is consumed so its potentially multi-megabyte collision
    /// strings move into this capability instead of being cloned.
    pub fn bind(
        base: BaseGameCollisionInventory,
        catalog: &StoryCatalogFile,
        project: &ProjectRevision2,
    ) -> Result<Self, QuestCollisionCapabilityError> {
        let project_identities = collect_project_story_collision_identities(project)?;
        Self::bind_project_identities(base, catalog, &project.target, project_identities)
    }

    /// Bind closed base/catalog capabilities directly to one exact native revision-3 Quest-free
    /// project snapshot.
    pub fn bind_revision3(
        base: BaseGameCollisionInventory,
        catalog: &StoryCatalogFile,
        project: &ProjectRevision3,
    ) -> Result<Self, QuestCollisionCapabilityError> {
        let project_identities = collect_revision3_story_collision_identities(project)?;
        Self::bind_project_identities(base, catalog, &project.target, project_identities)
    }

    fn bind_project_identities(
        base: BaseGameCollisionInventory,
        catalog: &StoryCatalogFile,
        project_target: &GameGenerationAnchor,
        project_identities: ProjectStoryCollisionIdentities,
    ) -> Result<Self, QuestCollisionCapabilityError> {
        let selections = catalog.authoring_selections()?;
        if base.generation() != catalog.generation()
            || base.story_catalog_seal() != catalog.catalog_seal()
        {
            return Err(QuestCollisionCapabilityError::CatalogBindingMismatch);
        }
        let expected_target = authoring_generation(catalog.generation());
        if project_target != &expected_target {
            return Err(QuestCollisionCapabilityError::TargetMismatch);
        }

        let project_id = project_identities.project_id();
        let project_revision = project_identities.project_revision();
        let project_target = project_identities.target().clone();
        let canonical_project = project_identities.canonical_project().clone();
        let (project_modules, project_relative_paths, project_symbols) =
            project_identities.into_collision_maps();
        let base_inventory_payload_seal = base.payload_seal().clone();
        let story_catalog_seal = base.story_catalog_seal().clone();
        let (base_modules, base_relative_paths, base_symbols) = base.into_collision_domains();
        let mut modules = base_modules.into_iter().collect::<BTreeSet<_>>();
        let mut relative_paths = base_relative_paths.into_iter().collect::<BTreeSet<_>>();
        let mut symbols = base_symbols.into_iter().collect::<BTreeSet<_>>();
        merge_project_domain("module", &mut modules, project_modules)?;
        merge_project_domain("relative path", &mut relative_paths, project_relative_paths)?;
        merge_project_domain("symbol", &mut symbols, project_symbols)?;
        enforce_combined_limits(&modules, &relative_paths, &symbols)?;

        let generation = authoring_generation(&selections.generation);
        let parents = selections
            .quest_parents
            .into_iter()
            .map(|parent| {
                (
                    parent.catalog_id,
                    QuestParentInput {
                        generation: generation.clone(),
                        source_seal: authoring_seal(&parent.quest_class.source_seal),
                        catalog_layer: parent.quest_class.catalog_layer,
                        canonical_selector: parent.quest_class.authoring_selector,
                        runtime_class: parent.quest_class.runtime_class,
                    },
                )
            })
            .collect();
        let givers = selections
            .npcs
            .into_iter()
            .map(|npc| {
                (
                    npc.catalog_id,
                    QuestGiverInput {
                        generation: generation.clone(),
                        source_seal: authoring_seal(&npc.quest_giver.source_seal),
                        catalog_layer: npc.quest_giver.catalog_layer,
                        canonical_selector: npc.quest_giver.authoring_selector,
                        runtime_unique_name: npc.quest_giver.runtime_unique_name,
                    },
                )
            })
            .collect();

        let combined_source_seal = seal_combined_payload(&CombinedPayload {
            format: COMBINED_FORMAT,
            schema_revision: COMBINED_SCHEMA_REVISION,
            coverage: QuestCollisionCoverage::BaseGameAndExactProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &base_inventory_payload_seal,
            story_catalog_seal: &story_catalog_seal,
            project_id,
            project_revision,
            project_target: &project_target,
            canonical_project: &canonical_project,
            modules: &modules,
            relative_paths: &relative_paths,
            symbols: &symbols,
        })?;
        Ok(Self {
            project_id,
            project_revision,
            project_target,
            canonical_project,
            base_inventory_payload_seal,
            story_catalog_seal,
            combined_source_seal,
            modules,
            relative_paths,
            symbols,
            parents,
            givers,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn project_revision(&self) -> u64 {
        self.project_revision
    }

    pub fn project_target(&self) -> &GameGenerationAnchor {
        &self.project_target
    }

    pub fn canonical_project(&self) -> &AuthoringContentSeal {
        &self.canonical_project
    }

    pub fn base_inventory_payload_seal(&self) -> &ContentSeal {
        &self.base_inventory_payload_seal
    }

    pub fn story_catalog_seal(&self) -> &ContentSeal {
        &self.story_catalog_seal
    }

    pub fn combined_source_seal(&self) -> &ContentSeal {
        &self.combined_source_seal
    }

    pub const fn catalog_layer(&self) -> &'static str {
        BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
    }

    pub const fn coverage(&self) -> QuestCollisionCoverage {
        QuestCollisionCoverage::BaseGameAndExactProjectOnly
    }

    pub const fn runtime_qualification(&self) -> QuestCollisionRuntimeQualification {
        QuestCollisionRuntimeQualification::RuntimeUnqualified
    }

    pub const fn build_status(&self) -> QuestCollisionBuildStatus {
        QuestCollisionBuildStatus::Blocked
    }

    pub const fn publication_status(&self) -> QuestCollisionPublicationStatus {
        QuestCollisionPublicationStatus::NotSupported
    }

    pub fn contains_module(&self, value: &str) -> bool {
        self.modules.contains(&value.to_ascii_lowercase())
    }

    pub fn contains_relative_path(&self, value: &str) -> bool {
        self.relative_paths.contains(&value.to_ascii_lowercase())
    }

    pub fn contains_symbol(&self, value: &str) -> bool {
        self.symbols.contains(&value.to_ascii_lowercase())
    }

    pub fn resolve_parent(
        &self,
        catalog_id: &str,
    ) -> Result<QuestParentInput, QuestCollisionCapabilityError> {
        self.parents
            .get(catalog_id)
            .cloned()
            .ok_or_else(|| QuestCollisionCapabilityError::UnknownParent(catalog_id.to_owned()))
    }

    pub fn resolve_giver(
        &self,
        catalog_id: &str,
    ) -> Result<QuestGiverInput, QuestCollisionCapabilityError> {
        self.givers
            .get(catalog_id)
            .cloned()
            .ok_or_else(|| QuestCollisionCapabilityError::UnknownGiver(catalog_id.to_owned()))
    }

    /// Whether one persisted parent identity is an exact member of the freshly loaded Story
    /// catalog used to bind this capability.
    ///
    /// Revision-3 Quest drafts retain the closed parent identity rather than its UI-facing
    /// catalog id. Comparing the complete value keeps that durable representation source-bound
    /// without exposing or cloning the capability's catalog map.
    pub fn authorizes_parent(&self, candidate: &QuestParentInput) -> bool {
        self.parents.values().any(|parent| parent == candidate)
    }

    /// Whether one persisted giver identity is an exact member of the freshly loaded Story
    /// catalog used to bind this capability.
    pub fn authorizes_giver(&self, candidate: &QuestGiverInput) -> bool {
        self.givers.values().any(|giver| giver == candidate)
    }

    /// Consume the verified capability into the existing revision-2 Quest input without cloning
    /// the multi-megabyte collision sets. The exact project is re-collected first so a stale
    /// capability cannot be applied to a different head.
    pub fn into_quest_collision_input(
        self,
        project: &ProjectRevision2,
    ) -> Result<QuestCollisionCatalogInput, QuestCollisionCapabilityError> {
        let current = collect_project_story_collision_identities(project)?;
        self.into_collision_input_for_identities(current)
    }

    /// Consume this capability against the exact native revision-3 Quest-free project that
    /// originally bound it.
    pub fn into_revision3_quest_collision_input(
        self,
        project: &ProjectRevision3,
    ) -> Result<QuestCollisionCatalogInput, QuestCollisionCapabilityError> {
        let current = collect_revision3_story_collision_identities(project)?;
        self.into_collision_input_for_identities(current)
    }

    fn into_collision_input_for_identities(
        self,
        current: ProjectStoryCollisionIdentities,
    ) -> Result<QuestCollisionCatalogInput, QuestCollisionCapabilityError> {
        if current.project_id() != self.project_id
            || current.project_revision() != self.project_revision
            || current.target() != &self.project_target
            || current.canonical_project() != &self.canonical_project
        {
            return Err(QuestCollisionCapabilityError::ProjectDrift);
        }
        Ok(QuestCollisionCatalogInput {
            generation: self.project_target,
            source_seal: authoring_seal(&self.combined_source_seal),
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER.to_owned(),
            modules: self.modules,
            relative_paths: self.relative_paths,
            symbols: self.symbols,
        })
    }

    /// Consume this freshly source-bound capability while borrowing one opaque artifact and
    /// return the still-authoritative capability only when every identity matches exactly.
    ///
    /// Canonical payload bytes are materialized directly from the capability's borrowed sets;
    /// the multi-megabyte collision collections are never cloned. Failure consumes and drops the
    /// capability, while the artifact remains merely structural. In particular, this is not an
    /// artifact-to-capability conversion: callers must independently bind trusted sources first.
    pub fn verify_artifact_exact(
        self,
        artifact: &QuestCollisionCapabilityArtifactV1,
    ) -> Result<Self, QuestCollisionCapabilityArtifactVerificationError> {
        let expected = self.materialize_artifact_identity(&artifact.canonical_json)?;
        if artifact.base_inventory_payload_seal != self.base_inventory_payload_seal {
            return Err(
                QuestCollisionCapabilityArtifactVerificationError::BaseInventoryPayloadSealMismatch,
            );
        }
        if artifact.story_catalog_seal != self.story_catalog_seal {
            return Err(
                QuestCollisionCapabilityArtifactVerificationError::StoryCatalogSealMismatch,
            );
        }
        if artifact.project_id != self.project_id {
            return Err(QuestCollisionCapabilityArtifactVerificationError::ProjectIdMismatch);
        }
        if artifact.project_revision != self.project_revision {
            return Err(QuestCollisionCapabilityArtifactVerificationError::ProjectRevisionMismatch);
        }
        if artifact.project_target != self.project_target {
            return Err(QuestCollisionCapabilityArtifactVerificationError::ProjectTargetMismatch);
        }
        if artifact.canonical_project != self.canonical_project {
            return Err(
                QuestCollisionCapabilityArtifactVerificationError::CanonicalProjectMismatch,
            );
        }
        if expected.source_seal != self.combined_source_seal
            || artifact.source_seal != expected.source_seal
        {
            return Err(
                QuestCollisionCapabilityArtifactVerificationError::SemanticSourceSealMismatch,
            );
        }
        if !expected.canonical_matches {
            return Err(
                QuestCollisionCapabilityArtifactVerificationError::CanonicalIdentityMismatch,
            );
        }
        if artifact.artifact_seal != expected.artifact_seal {
            return Err(QuestCollisionCapabilityArtifactVerificationError::RawArtifactSealMismatch);
        }
        Ok(self)
    }

    /// Materialize the exact legacy artifact while retaining this original source-bound
    /// capability in one linear, non-clone capsule.
    ///
    /// Peak retained data is one collision-set representation plus one canonical byte vector of
    /// at most [`MAX_INVENTORY_JSON_BYTES`]. No collision set and no artifact byte vector is
    /// cloned. [`Self::into_artifact`] remains the artifact-only compatibility API.
    pub fn prepare_artifact(
        self,
    ) -> Result<PreparedQuestCollisionArtifactV1, QuestCollisionCapabilityArtifactError> {
        let artifact = self.materialize_structural_artifact()?;
        Ok(PreparedQuestCollisionArtifactV1 {
            capability: self,
            artifact,
        })
    }

    /// Consume this verified capability into one immutable, content-addressable artifact.
    ///
    /// The large collision strings remain in their single owned collection while the serializer
    /// writes the canonical bytes and are then dropped; they are never cloned into a second
    /// retained collection. The returned value keeps only canonical JSON bytes and bounded
    /// provenance metadata.
    pub fn into_artifact(
        self,
    ) -> Result<QuestCollisionCapabilityArtifactV1, QuestCollisionCapabilityArtifactError> {
        let canonical_json = self.materialize_canonical_payload()?;
        let Self {
            project_id,
            project_revision,
            project_target,
            canonical_project,
            base_inventory_payload_seal,
            story_catalog_seal,
            combined_source_seal,
            modules: _,
            relative_paths: _,
            symbols: _,
            parents: _,
            givers: _,
        } = self;
        let actual_source_seal = seal_combined_payload_bytes(&canonical_json);
        if actual_source_seal != combined_source_seal {
            return Err(QuestCollisionCapabilityArtifactError::Invariant(
                "verified capability semantic seal changed while materializing its artifact"
                    .to_owned(),
            ));
        }
        let artifact_seal = raw_artifact_seal(&canonical_json);
        Ok(QuestCollisionCapabilityArtifactV1 {
            canonical_json,
            artifact_seal,
            source_seal: combined_source_seal,
            base_inventory_payload_seal,
            story_catalog_seal,
            project_id,
            project_revision,
            project_target,
            canonical_project,
        })
    }

    fn materialize_structural_artifact(
        &self,
    ) -> Result<QuestCollisionCapabilityArtifactV1, QuestCollisionCapabilityArtifactError> {
        let canonical_json = self.materialize_canonical_payload()?;
        let actual_source_seal = seal_combined_payload_bytes(&canonical_json);
        if actual_source_seal != self.combined_source_seal {
            return Err(QuestCollisionCapabilityArtifactError::Invariant(
                "verified capability semantic seal changed while materializing its artifact"
                    .to_owned(),
            ));
        }
        let artifact_seal = raw_artifact_seal(&canonical_json);
        Ok(QuestCollisionCapabilityArtifactV1 {
            canonical_json,
            artifact_seal,
            source_seal: self.combined_source_seal.clone(),
            base_inventory_payload_seal: self.base_inventory_payload_seal.clone(),
            story_catalog_seal: self.story_catalog_seal.clone(),
            project_id: self.project_id,
            project_revision: self.project_revision,
            project_target: self.project_target.clone(),
            canonical_project: self.canonical_project.clone(),
        })
    }

    fn materialize_canonical_payload(
        &self,
    ) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactError> {
        canonical_combined_payload(&self.combined_payload())
    }

    fn materialize_artifact_identity(
        &self,
        expected_canonical: &[u8],
    ) -> Result<MaterializedArtifactIdentity, QuestCollisionCapabilityArtifactVerificationError>
    {
        materialize_artifact_identity(&self.combined_payload(), expected_canonical)
    }

    fn combined_payload(&self) -> CombinedPayload<'_> {
        CombinedPayload {
            format: COMBINED_FORMAT,
            schema_revision: COMBINED_SCHEMA_REVISION,
            coverage: QuestCollisionCoverage::BaseGameAndExactProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &self.base_inventory_payload_seal,
            story_catalog_seal: &self.story_catalog_seal,
            project_id: self.project_id,
            project_revision: self.project_revision,
            project_target: &self.project_target,
            canonical_project: &self.canonical_project,
            modules: &self.modules,
            relative_paths: &self.relative_paths,
            symbols: &self.symbols,
        }
    }
}

/// Structurally reopen one untrusted artifact under two independently supplied expected seals.
///
/// This proves bounded canonical structure and exact content identity. It deliberately does not
/// re-extract Shipping/Binds sources or upgrade the artifact's fixed unsupported capability
/// claims; source-backed re-verification belongs to the later resolver boundary.
pub fn reopen_quest_collision_capability_artifact_v1(
    canonical_json: &[u8],
    expected_artifact_seal: &ContentSeal,
    expected_source_seal: &ContentSeal,
) -> Result<QuestCollisionCapabilityArtifactV1, QuestCollisionCapabilityArtifactError> {
    if canonical_json.is_empty() {
        return Err(QuestCollisionCapabilityArtifactError::Invariant(
            "canonical artifact JSON must not be empty".to_owned(),
        ));
    }
    if canonical_json.len() > MAX_INVENTORY_JSON_BYTES {
        return Err(QuestCollisionCapabilityArtifactError::Limit {
            kind: "canonical artifact JSON bytes",
            actual: canonical_json.len(),
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    preflight_json_string_tokens(canonical_json)?;
    validate_expected_seal("raw artifact", expected_artifact_seal, canonical_json.len())?;
    validate_expected_seal(
        "semantic source",
        expected_source_seal,
        canonical_json.len(),
    )?;
    let actual_artifact_seal = raw_artifact_seal(canonical_json);
    if &actual_artifact_seal != expected_artifact_seal {
        return Err(QuestCollisionCapabilityArtifactError::ArtifactSealMismatch);
    }
    let actual_source_seal = seal_combined_payload_bytes(canonical_json);
    if &actual_source_seal != expected_source_seal {
        return Err(QuestCollisionCapabilityArtifactError::SourceSealMismatch);
    }

    let wire: CombinedArtifactWire = serde_json::from_slice(canonical_json)
        .map_err(QuestCollisionCapabilityArtifactError::InvalidJson)?;
    validate_artifact_wire(&wire)?;
    let canonical = canonical_artifact_wire(&wire)?;
    if canonical.as_slice() != canonical_json {
        return Err(QuestCollisionCapabilityArtifactError::NonCanonicalJson);
    }

    Ok(QuestCollisionCapabilityArtifactV1 {
        canonical_json: canonical,
        artifact_seal: actual_artifact_seal,
        source_seal: actual_source_seal,
        base_inventory_payload_seal: wire.base_inventory_payload_seal,
        story_catalog_seal: wire.story_catalog_seal,
        project_id: wire.project_id,
        project_revision: wire.project_revision,
        project_target: wire.project_target,
        canonical_project: wire.canonical_project,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum QuestCollisionCapabilityError {
    #[error("base inventory and trusted Story catalog are not exactly bound")]
    CatalogBindingMismatch,
    #[error("authoring project target does not match the trusted base-game generation")]
    TargetMismatch,
    #[error("verified Quest collision capability no longer matches the exact project head")]
    ProjectDrift,
    #[error("project {kind} identity {value:?} owned by {owner} collides with the base game")]
    BaseProjectCollision {
        kind: &'static str,
        value: String,
        owner: EntityId,
    },
    #[error("combined Quest collision capability exceeds {kind}: {actual} > {max}")]
    Limit {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("unknown trusted Story catalog Quest parent {0:?}")]
    UnknownParent(String),
    #[error("unknown trusted Story catalog Quest giver {0:?}")]
    UnknownGiver(String),
    #[error("could not serialize combined Quest collision provenance: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Project(#[from] StoryCollisionCollectionError),
}

/// Fail-closed mismatch categories for capability-owned artifact verification.
#[derive(Debug, thiserror::Error)]
pub enum QuestCollisionCapabilityArtifactVerificationError {
    #[error("Quest collision artifact raw content seal does not match exact canonical bytes")]
    RawArtifactSealMismatch,
    #[error("Quest collision artifact semantic source seal does not match exact canonical bytes")]
    SemanticSourceSealMismatch,
    #[error("Quest collision artifact base inventory payload seal mismatch")]
    BaseInventoryPayloadSealMismatch,
    #[error("Quest collision artifact Story catalog seal mismatch")]
    StoryCatalogSealMismatch,
    #[error("Quest collision artifact project id mismatch")]
    ProjectIdMismatch,
    #[error("Quest collision artifact project revision mismatch")]
    ProjectRevisionMismatch,
    #[error("Quest collision artifact project target mismatch")]
    ProjectTargetMismatch,
    #[error("Quest collision artifact canonical project seal mismatch")]
    CanonicalProjectMismatch,
    #[error(
        "Quest collision artifact canonical identity differs from the source-bound capability"
    )]
    CanonicalIdentityMismatch,
    #[error("source-bound Quest collision artifact identity exceeds bytes: {actual} > {max}")]
    Limit { actual: usize, max: usize },
    #[error("could not serialize source-bound Quest collision artifact identity: {0}")]
    Serialize(#[source] serde_json::Error),
}

/// Stable failure boundary for consuming a prepared Quest collision capsule.
#[derive(Debug, thiserror::Error)]
pub enum PreparedQuestCollisionArtifactFinalizeError {
    #[error(
        "prepared Quest collision artifact and retained source capability no longer match exactly"
    )]
    ArtifactOrCapabilityDrift(#[source] QuestCollisionCapabilityArtifactVerificationError),
    #[error("prepared Quest collision capability no longer matches the supplied exact project")]
    Project(#[source] QuestCollisionCapabilityError),
}

#[derive(Debug, thiserror::Error)]
pub enum QuestCollisionCapabilityArtifactError {
    #[error("Quest collision capability artifact exceeds {kind}: {actual} > {max}")]
    Limit {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("invalid Quest collision capability artifact JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize canonical Quest collision capability artifact JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("Quest collision capability artifact JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error(
        "Quest collision capability artifact {kind} seal declares {declared} bytes; actual is {actual}"
    )]
    SealLengthMismatch {
        kind: &'static str,
        declared: u64,
        actual: u64,
    },
    #[error("Quest collision capability artifact raw content seal mismatch")]
    ArtifactSealMismatch,
    #[error("Quest collision capability artifact semantic source seal mismatch")]
    SourceSealMismatch,
    #[error("invalid Quest collision capability artifact invariant: {0}")]
    Invariant(String),
}

fn merge_project_domain(
    kind: &'static str,
    combined: &mut BTreeSet<String>,
    project: BTreeMap<String, EntityId>,
) -> Result<(), QuestCollisionCapabilityError> {
    for (value, owner) in project {
        if combined.contains(&value) {
            return Err(QuestCollisionCapabilityError::BaseProjectCollision { kind, value, owner });
        }
        combined.insert(value);
    }
    Ok(())
}

fn enforce_combined_limits(
    modules: &BTreeSet<String>,
    relative_paths: &BTreeSet<String>,
    symbols: &BTreeSet<String>,
) -> Result<(), QuestCollisionCapabilityError> {
    let count = modules
        .len()
        .checked_add(relative_paths.len())
        .and_then(|count| count.checked_add(symbols.len()))
        .unwrap_or(usize::MAX);
    if count > MAX_COLLISION_ENTRIES {
        return Err(QuestCollisionCapabilityError::Limit {
            kind: "entry count",
            actual: count,
            max: MAX_COLLISION_ENTRIES,
        });
    }
    let bytes = modules
        .iter()
        .chain(relative_paths)
        .chain(symbols)
        .try_fold(0usize, |total, value| total.checked_add(value.len()))
        .unwrap_or(usize::MAX);
    if bytes > MAX_COLLISION_TOTAL_BYTES {
        return Err(QuestCollisionCapabilityError::Limit {
            kind: "aggregate entry bytes",
            actual: bytes,
            max: MAX_COLLISION_TOTAL_BYTES,
        });
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CombinedPayload<'a> {
    format: &'static str,
    schema_revision: u32,
    coverage: QuestCollisionCoverage,
    catalog_layer: &'static str,
    runtime_qualification: QuestCollisionRuntimeQualification,
    build_status: QuestCollisionBuildStatus,
    publication_status: QuestCollisionPublicationStatus,
    base_inventory_payload_seal: &'a ContentSeal,
    story_catalog_seal: &'a ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: &'a GameGenerationAnchor,
    canonical_project: &'a AuthoringContentSeal,
    modules: &'a BTreeSet<String>,
    relative_paths: &'a BTreeSet<String>,
    symbols: &'a BTreeSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct BoundedArtifactString<const MAX: usize>(String);

impl<const MAX: usize> Serialize for BoundedArtifactString<MAX> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de, const MAX: usize> Deserialize<'de> for BoundedArtifactString<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoundedArtifactStringVisitor<const MAX: usize>;

        impl<const MAX: usize> de::Visitor<'_> for BoundedArtifactStringVisitor<MAX> {
            type Value = BoundedArtifactString<MAX>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a UTF-8 string of at most {MAX} bytes")
            }

            fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(value)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedArtifactString(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedArtifactString(value))
            }
        }

        deserializer.deserialize_string(BoundedArtifactStringVisitor::<MAX>)
    }
}

#[derive(Debug, Serialize)]
struct CombinedArtifactWire {
    format: BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>,
    schema_revision: u32,
    coverage: BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>,
    catalog_layer: BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>,
    runtime_qualification: BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>,
    build_status: BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>,
    publication_status: BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    canonical_project: AuthoringContentSeal,
    modules: Vec<String>,
    relative_paths: Vec<String>,
    symbols: Vec<String>,
}

impl<'de> Deserialize<'de> for CombinedArtifactWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CombinedArtifactVisitor;

        impl<'de> de::Visitor<'de> for CombinedArtifactVisitor {
            type Value = CombinedArtifactWire;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed bounded Quest collision capability artifact")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut format = None;
                let mut schema_revision = None;
                let mut coverage = None;
                let mut catalog_layer = None;
                let mut runtime_qualification = None;
                let mut build_status = None;
                let mut publication_status = None;
                let mut base_inventory_payload_seal = None;
                let mut story_catalog_seal = None;
                let mut project_id = None;
                let mut project_revision = None;
                let mut project_target = None;
                let mut canonical_project = None;
                let mut modules = None;
                let mut relative_paths = None;
                let mut symbols = None;
                let mut remaining_count = MAX_COLLISION_ENTRIES;
                let mut remaining_bytes = MAX_COLLISION_TOTAL_BYTES;

                while let Some(BoundedArtifactString(field)) =
                    access.next_key::<BoundedArtifactString<MAX_COMBINED_MARKER_BYTES>>()?
                {
                    macro_rules! scalar {
                        ($slot:ident, $name:literal) => {{
                            if $slot.is_some() {
                                return Err(de::Error::duplicate_field($name));
                            }
                            $slot = Some(access.next_value()?);
                        }};
                    }
                    match field.as_str() {
                        "format" => scalar!(format, "format"),
                        "schema_revision" => scalar!(schema_revision, "schema_revision"),
                        "coverage" => scalar!(coverage, "coverage"),
                        "catalog_layer" => scalar!(catalog_layer, "catalog_layer"),
                        "runtime_qualification" => {
                            scalar!(runtime_qualification, "runtime_qualification")
                        }
                        "build_status" => scalar!(build_status, "build_status"),
                        "publication_status" => {
                            scalar!(publication_status, "publication_status")
                        }
                        "base_inventory_payload_seal" => {
                            scalar!(base_inventory_payload_seal, "base_inventory_payload_seal")
                        }
                        "story_catalog_seal" => {
                            scalar!(story_catalog_seal, "story_catalog_seal")
                        }
                        "project_id" => scalar!(project_id, "project_id"),
                        "project_revision" => scalar!(project_revision, "project_revision"),
                        "project_target" => scalar!(project_target, "project_target"),
                        "canonical_project" => scalar!(canonical_project, "canonical_project"),
                        "modules" | "relative_paths" | "symbols" => {
                            let (slot, kind): (&mut Option<Vec<String>>, &'static str) =
                                match field.as_str() {
                                    "modules" => (&mut modules, "module"),
                                    "relative_paths" => (&mut relative_paths, "relative path"),
                                    "symbols" => (&mut symbols, "symbol"),
                                    _ => unreachable!(),
                                };
                            if slot.is_some() {
                                return Err(de::Error::duplicate_field(match kind {
                                    "module" => "modules",
                                    "relative path" => "relative_paths",
                                    _ => "symbols",
                                }));
                            }
                            let parsed = access.next_value_seed(CollisionEntriesSeed {
                                kind,
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= parsed.count;
                            remaining_bytes -= parsed.bytes;
                            *slot = Some(parsed.values);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &field,
                                &[
                                    "format",
                                    "schema_revision",
                                    "coverage",
                                    "catalog_layer",
                                    "runtime_qualification",
                                    "build_status",
                                    "publication_status",
                                    "base_inventory_payload_seal",
                                    "story_catalog_seal",
                                    "project_id",
                                    "project_revision",
                                    "project_target",
                                    "canonical_project",
                                    "modules",
                                    "relative_paths",
                                    "symbols",
                                ],
                            ));
                        }
                    }
                }

                macro_rules! required {
                    ($slot:ident, $name:literal) => {
                        $slot.ok_or_else(|| de::Error::missing_field($name))?
                    };
                }
                Ok(CombinedArtifactWire {
                    format: required!(format, "format"),
                    schema_revision: required!(schema_revision, "schema_revision"),
                    coverage: required!(coverage, "coverage"),
                    catalog_layer: required!(catalog_layer, "catalog_layer"),
                    runtime_qualification: required!(
                        runtime_qualification,
                        "runtime_qualification"
                    ),
                    build_status: required!(build_status, "build_status"),
                    publication_status: required!(publication_status, "publication_status"),
                    base_inventory_payload_seal: required!(
                        base_inventory_payload_seal,
                        "base_inventory_payload_seal"
                    ),
                    story_catalog_seal: required!(story_catalog_seal, "story_catalog_seal"),
                    project_id: required!(project_id, "project_id"),
                    project_revision: required!(project_revision, "project_revision"),
                    project_target: required!(project_target, "project_target"),
                    canonical_project: required!(canonical_project, "canonical_project"),
                    modules: required!(modules, "modules"),
                    relative_paths: required!(relative_paths, "relative_paths"),
                    symbols: required!(symbols, "symbols"),
                })
            }
        }

        deserializer.deserialize_map(CombinedArtifactVisitor)
    }
}

struct CollisionEntriesSeed {
    kind: &'static str,
    remaining_count: usize,
    remaining_bytes: usize,
}

struct ParsedCollisionEntries {
    values: Vec<String>,
    count: usize,
    bytes: usize,
}

struct MaterializedArtifactIdentity {
    artifact_seal: ContentSeal,
    source_seal: ContentSeal,
    canonical_matches: bool,
}

fn materialize_artifact_identity(
    payload: &CombinedPayload<'_>,
    expected_canonical: &[u8],
) -> Result<MaterializedArtifactIdentity, QuestCollisionCapabilityArtifactVerificationError> {
    let mut writer =
        ExactCanonicalIdentityWriter::new(expected_canonical, MAX_INVENTORY_JSON_BYTES);
    let serialized = serde_json::to_writer(&mut writer, payload);
    if let Some(actual) = writer.first_exceeded_size {
        return Err(QuestCollisionCapabilityArtifactVerificationError::Limit {
            actual,
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    serialized.map_err(QuestCollisionCapabilityArtifactVerificationError::Serialize)?;
    let payload_len = writer.bytes_written;
    let canonical_matches = writer.canonical_matches && payload_len == expected_canonical.len();
    let artifact_seal = ContentSeal {
        byte_len: payload_len as u64,
        sha256: super::Sha256Digest::from_bytes(writer.raw_hasher.finalize().into()),
    };

    let mut semantic_hasher = Sha256::new();
    semantic_hasher.update(COMBINED_SEAL_DOMAIN);
    semantic_hasher.update((payload_len as u64).to_be_bytes());
    serde_json::to_writer(HashWriter(&mut semantic_hasher), payload)
        .map_err(QuestCollisionCapabilityArtifactVerificationError::Serialize)?;
    let source_seal = ContentSeal {
        byte_len: payload_len as u64,
        sha256: super::Sha256Digest::from_bytes(semantic_hasher.finalize().into()),
    };
    Ok(MaterializedArtifactIdentity {
        artifact_seal,
        source_seal,
        canonical_matches,
    })
}

struct ExactCanonicalIdentityWriter<'a> {
    expected: &'a [u8],
    bytes_written: usize,
    limit: usize,
    canonical_matches: bool,
    first_exceeded_size: Option<usize>,
    raw_hasher: Sha256,
}

impl<'a> ExactCanonicalIdentityWriter<'a> {
    fn new(expected: &'a [u8], limit: usize) -> Self {
        Self {
            expected,
            bytes_written: 0,
            limit,
            canonical_matches: true,
            first_exceeded_size: None,
            raw_hasher: Sha256::new(),
        }
    }
}

impl Write for ExactCanonicalIdentityWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let start = self.bytes_written;
        let actual = start.saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "source-bound canonical payload byte limit exceeded",
            ));
        }
        if self.expected.get(start..actual) != Some(bytes) {
            self.canonical_matches = false;
        }
        self.raw_hasher.update(bytes);
        self.bytes_written = actual;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'de> de::DeserializeSeed<'de> for CollisionEntriesSeed {
    type Value = ParsedCollisionEntries;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntriesVisitor {
            kind: &'static str,
            remaining_count: usize,
            remaining_bytes: usize,
        }

        impl<'de> de::Visitor<'de> for EntriesVisitor {
            type Value = ParsedCollisionEntries;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "a strict array within {} remaining entries and {} remaining bytes",
                    self.remaining_count, self.remaining_bytes
                )
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                if sequence
                    .size_hint()
                    .is_some_and(|hint| hint > self.remaining_count)
                {
                    return Err(de::Error::custom("collision entry count limit exceeded"));
                }
                let mut values =
                    Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(self.remaining_count));
                let mut bytes = 0usize;
                while let Some(BoundedArtifactString(value)) =
                    sequence.next_element::<BoundedArtifactString<MAX_COLLISION_ENTRY_BYTES>>()?
                {
                    if values.len() == self.remaining_count {
                        return Err(de::Error::custom("collision entry count limit exceeded"));
                    }
                    if value.is_empty()
                        || !value.is_ascii()
                        || value.bytes().any(|byte| byte.is_ascii_uppercase())
                        || value.bytes().any(|byte| byte.is_ascii_control())
                    {
                        return Err(de::Error::custom(format!(
                            "invalid {} collision entry",
                            self.kind
                        )));
                    }
                    if values.last().is_some_and(|previous| previous >= &value) {
                        return Err(de::Error::custom(format!(
                            "{} collision entries are not in strict canonical order",
                            self.kind
                        )));
                    }
                    bytes = bytes
                        .checked_add(value.len())
                        .ok_or_else(|| de::Error::custom("collision byte count overflow"))?;
                    if bytes > self.remaining_bytes {
                        return Err(de::Error::custom(
                            "aggregate collision entry byte limit exceeded",
                        ));
                    }
                    values.push(value);
                }
                Ok(ParsedCollisionEntries {
                    count: values.len(),
                    values,
                    bytes,
                })
            }
        }

        deserializer.deserialize_seq(EntriesVisitor {
            kind: self.kind,
            remaining_count: self.remaining_count,
            remaining_bytes: self.remaining_bytes,
        })
    }
}

fn canonical_combined_payload(
    payload: &CombinedPayload<'_>,
) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactError> {
    let mut writer = BoundedBytesWriter::new(MAX_INVENTORY_JSON_BYTES);
    let result = serde_json::to_writer(&mut writer, payload);
    finish_bounded_artifact_json(writer, result)
}

fn canonical_artifact_wire(
    wire: &CombinedArtifactWire,
) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactError> {
    let mut writer = BoundedBytesWriter::new(MAX_INVENTORY_JSON_BYTES);
    let result = serde_json::to_writer(&mut writer, wire);
    finish_bounded_artifact_json(writer, result)
}

fn finish_bounded_artifact_json(
    writer: BoundedBytesWriter,
    result: Result<(), serde_json::Error>,
) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactError> {
    if let Some(actual) = writer.first_exceeded_size {
        return Err(QuestCollisionCapabilityArtifactError::Limit {
            kind: "canonical artifact JSON bytes",
            actual,
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    result.map_err(QuestCollisionCapabilityArtifactError::Serialize)?;
    Ok(writer.bytes)
}

fn validate_artifact_wire(
    wire: &CombinedArtifactWire,
) -> Result<(), QuestCollisionCapabilityArtifactError> {
    let fixed = [
        (wire.format.0.as_str(), COMBINED_FORMAT, "format"),
        (
            wire.coverage.0.as_str(),
            "base_game_and_exact_project_only",
            "coverage",
        ),
        (
            wire.catalog_layer.0.as_str(),
            BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            "catalog layer",
        ),
        (
            wire.runtime_qualification.0.as_str(),
            "runtime_unqualified",
            "runtime qualification",
        ),
        (wire.build_status.0.as_str(), "blocked", "build status"),
        (
            wire.publication_status.0.as_str(),
            "not_supported",
            "publication status",
        ),
    ];
    for (actual, expected, kind) in fixed {
        if actual != expected {
            return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
                "unsupported {kind} {actual:?}; expected {expected:?}"
            )));
        }
    }
    if wire.schema_revision != COMBINED_SCHEMA_REVISION {
        return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
            "unsupported schema revision {}; expected {COMBINED_SCHEMA_REVISION}",
            wire.schema_revision
        )));
    }
    validate_catalog_seal(
        "base inventory payload",
        &wire.base_inventory_payload_seal,
        MAX_INVENTORY_JSON_BYTES as u64,
    )?;
    validate_catalog_seal(
        "Story catalog",
        &wire.story_catalog_seal,
        MAX_CATALOG_JSON_BYTES as u64,
    )?;
    if wire.project_id.as_bytes().iter().all(|byte| *byte == 0) {
        return Err(QuestCollisionCapabilityArtifactError::Invariant(
            "project id must not be all zeroes".to_owned(),
        ));
    }
    validate_authoring_seal(
        "project target executable",
        &wire.project_target.executable,
        u64::MAX,
    )?;
    validate_authoring_seal(
        "canonical project",
        &wire.canonical_project,
        MAX_PROJECT_JSON_BYTES as u64,
    )?;

    let count = wire
        .modules
        .len()
        .checked_add(wire.relative_paths.len())
        .and_then(|count| count.checked_add(wire.symbols.len()))
        .unwrap_or(usize::MAX);
    if count > MAX_COLLISION_ENTRIES {
        return Err(QuestCollisionCapabilityArtifactError::Limit {
            kind: "collision entry count",
            actual: count,
            max: MAX_COLLISION_ENTRIES,
        });
    }
    let mut bytes = 0usize;
    for (kind, entries) in [
        ("module", &wire.modules),
        ("relative path", &wire.relative_paths),
        ("symbol", &wire.symbols),
    ] {
        if entries.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
                "{kind} collision entries are not in strict canonical order"
            )));
        }
        for entry in entries {
            if entry.is_empty()
                || entry.len() > MAX_COLLISION_ENTRY_BYTES
                || !entry.is_ascii()
                || entry.bytes().any(|byte| byte.is_ascii_uppercase())
                || entry.bytes().any(|byte| byte.is_ascii_control())
            {
                return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
                    "invalid {kind} collision entry"
                )));
            }
            bytes = bytes.saturating_add(entry.len());
            if bytes > MAX_COLLISION_TOTAL_BYTES {
                return Err(QuestCollisionCapabilityArtifactError::Limit {
                    kind: "aggregate collision entry bytes",
                    actual: bytes,
                    max: MAX_COLLISION_TOTAL_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn validate_expected_seal(
    kind: &'static str,
    seal: &ContentSeal,
    actual_len: usize,
) -> Result<(), QuestCollisionCapabilityArtifactError> {
    if seal.byte_len == 0 {
        return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
            "{kind} seal has zero byte length"
        )));
    }
    if seal.byte_len != actual_len as u64 {
        return Err(QuestCollisionCapabilityArtifactError::SealLengthMismatch {
            kind,
            declared: seal.byte_len,
            actual: actual_len as u64,
        });
    }
    Ok(())
}

fn validate_catalog_seal(
    kind: &'static str,
    seal: &ContentSeal,
    max: u64,
) -> Result<(), QuestCollisionCapabilityArtifactError> {
    if seal.byte_len == 0 || seal.byte_len > max {
        return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
            "{kind} seal byte length is outside 1..={max}"
        )));
    }
    Ok(())
}

fn validate_authoring_seal(
    kind: &'static str,
    seal: &AuthoringContentSeal,
    max: u64,
) -> Result<(), QuestCollisionCapabilityArtifactError> {
    if seal.byte_len == 0 || seal.byte_len > max {
        return Err(QuestCollisionCapabilityArtifactError::Invariant(format!(
            "{kind} seal byte length is outside 1..={max}"
        )));
    }
    Ok(())
}

fn raw_artifact_seal(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: super::Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn seal_combined_payload_bytes(bytes: &[u8]) -> ContentSeal {
    let mut hasher = Sha256::new();
    hasher.update(COMBINED_SEAL_DOMAIN);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: super::Sha256Digest::from_bytes(hasher.finalize().into()),
    }
}

fn preflight_json_string_tokens(bytes: &[u8]) -> Result<(), QuestCollisionCapabilityArtifactError> {
    let mut in_string = false;
    let mut escaped = false;
    let mut raw_len = 0usize;

    for &byte in bytes {
        if !in_string {
            if byte == b'"' {
                in_string = true;
                escaped = false;
                raw_len = 0;
            }
            continue;
        }
        if byte == b'"' && !escaped {
            in_string = false;
            continue;
        }
        raw_len = raw_len.saturating_add(1);
        if raw_len > MAX_JSON_STRING_TOKEN_BYTES {
            return Err(QuestCollisionCapabilityArtifactError::Limit {
                kind: "raw JSON string token bytes",
                actual: raw_len,
                max: MAX_JSON_STRING_TOKEN_BYTES,
            });
        }
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        }
    }
    Ok(())
}

struct BoundedBytesWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedBytesWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedBytesWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes.len().saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other("canonical artifact JSON limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn seal_combined_payload(
    payload: &CombinedPayload<'_>,
) -> Result<ContentSeal, QuestCollisionCapabilityError> {
    let mut counter = BoundedCountingWriter::new(MAX_INVENTORY_JSON_BYTES);
    let counted = serde_json::to_writer(&mut counter, &payload);
    if let Some(actual) = counter.first_exceeded_size {
        return Err(QuestCollisionCapabilityError::Limit {
            kind: "combined canonical payload bytes",
            actual,
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    counted.map_err(QuestCollisionCapabilityError::Serialize)?;
    let payload_len = counter.bytes_written;
    let mut hasher = Sha256::new();
    hasher.update(COMBINED_SEAL_DOMAIN);
    hasher.update((payload_len as u64).to_be_bytes());
    serde_json::to_writer(HashWriter(&mut hasher), &payload)
        .map_err(QuestCollisionCapabilityError::Serialize)?;
    Ok(ContentSeal {
        byte_len: payload_len as u64,
        sha256: super::Sha256Digest::from_bytes(hasher.finalize().into()),
    })
}

struct BoundedCountingWriter {
    bytes_written: usize,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedCountingWriter {
    const fn new(limit: usize) -> Self {
        Self {
            bytes_written: 0,
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedCountingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes_written.saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "combined canonical payload byte limit exceeded",
            ));
        }
        self.bytes_written = actual;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct HashWriter<'a>(&'a mut Sha256);

impl Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn authoring_generation(
    generation: &gore_story_catalog::GameGenerationSeal,
) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: authoring_seal(&generation.executable),
    }
}

fn authoring_seal(seal: &ContentSeal) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: seal.byte_len,
        sha256: AuthoringSha256Digest::from_bytes(*seal.sha256.as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_authoring::{AssetStoreIndex, FormatV2, ProjectMeta, SchemaRevisionV2};

    trait AmbiguousIfClone<Marker> {
        fn marker() {}
    }

    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}

    fn test_capability() -> VerifiedQuestCollisionCapability {
        let project_id = ProjectId::from_bytes([3; 16]);
        let project_revision = 9;
        let project_target = GameGenerationAnchor {
            executable: AuthoringContentSeal {
                byte_len: 13,
                sha256: AuthoringSha256Digest::from_bytes([4; 32]),
            },
        };
        let canonical_project = AuthoringContentSeal {
            byte_len: 14,
            sha256: AuthoringSha256Digest::from_bytes([5; 32]),
        };
        let base_inventory_payload_seal = ContentSeal {
            byte_len: 11,
            sha256: crate::Sha256Digest::from_bytes([1; 32]),
        };
        let story_catalog_seal = ContentSeal {
            byte_len: 12,
            sha256: crate::Sha256Digest::from_bytes([2; 32]),
        };
        let modules = BTreeSet::from(["base.module".to_owned(), "project.module".to_owned()]);
        let relative_paths =
            BTreeSet::from(["base/module.as".to_owned(), "project/module.as".to_owned()]);
        let symbols = BTreeSet::from(["ubase".to_owned(), "uproject".to_owned()]);
        let combined_source_seal = seal_combined_payload(&CombinedPayload {
            format: COMBINED_FORMAT,
            schema_revision: COMBINED_SCHEMA_REVISION,
            coverage: QuestCollisionCoverage::BaseGameAndExactProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &base_inventory_payload_seal,
            story_catalog_seal: &story_catalog_seal,
            project_id,
            project_revision,
            project_target: &project_target,
            canonical_project: &canonical_project,
            modules: &modules,
            relative_paths: &relative_paths,
            symbols: &symbols,
        })
        .unwrap();
        VerifiedQuestCollisionCapability {
            project_id,
            project_revision,
            project_target,
            canonical_project,
            base_inventory_payload_seal,
            story_catalog_seal,
            combined_source_seal,
            modules,
            relative_paths,
            symbols,
            parents: BTreeMap::new(),
            givers: BTreeMap::new(),
        }
    }

    fn unrelated_test_project() -> ProjectRevision2 {
        let capability = test_capability();
        ProjectRevision2 {
            format: FormatV2,
            schema_revision: SchemaRevisionV2,
            project_id: capability.project_id,
            revision: capability.project_revision,
            meta: ProjectMeta {
                name: "prepared collision capsule".to_owned(),
                version: "0.1.0".to_owned(),
                author: "test".to_owned(),
            },
            target: capability.project_target,
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn finalized_verification_error(
        prepared: PreparedQuestCollisionArtifactV1,
    ) -> QuestCollisionCapabilityArtifactVerificationError {
        match prepared.finalize(&unrelated_test_project()) {
            Err(PreparedQuestCollisionArtifactFinalizeError::ArtifactOrCapabilityDrift(error)) => {
                error
            }
            Err(other) => panic!("unexpected prepared capsule error: {other}"),
            Ok(_) => panic!("drifted prepared capsule unexpectedly finalized"),
        }
    }

    fn reopen_with_actual_seals(
        bytes: &[u8],
    ) -> Result<QuestCollisionCapabilityArtifactV1, QuestCollisionCapabilityArtifactError> {
        reopen_quest_collision_capability_artifact_v1(
            bytes,
            &raw_artifact_seal(bytes),
            &seal_combined_payload_bytes(bytes),
        )
    }

    #[test]
    fn capability_is_not_clone_and_bind_consumes_the_base_inventory() {
        let _ = <VerifiedQuestCollisionCapability as AmbiguousIfClone<_>>::marker as fn();
        let _ = <QuestCollisionCapabilityArtifactV1 as AmbiguousIfClone<_>>::marker as fn();
        let _ = <PreparedQuestCollisionArtifactV1 as AmbiguousIfClone<_>>::marker as fn();
        let _bind: fn(
            BaseGameCollisionInventory,
            &StoryCatalogFile,
            &ProjectRevision2,
        )
            -> Result<VerifiedQuestCollisionCapability, QuestCollisionCapabilityError> =
            VerifiedQuestCollisionCapability::bind;
        let _verify: fn(
            VerifiedQuestCollisionCapability,
            &QuestCollisionCapabilityArtifactV1,
        ) -> Result<
            VerifiedQuestCollisionCapability,
            QuestCollisionCapabilityArtifactVerificationError,
        > = VerifiedQuestCollisionCapability::verify_artifact_exact;
        let _prepare: fn(
            VerifiedQuestCollisionCapability,
        ) -> Result<
            PreparedQuestCollisionArtifactV1,
            QuestCollisionCapabilityArtifactError,
        > = VerifiedQuestCollisionCapability::prepare_artifact;
        let _borrow: for<'a> fn(
            &'a PreparedQuestCollisionArtifactV1,
        ) -> &'a QuestCollisionCapabilityArtifactV1 = PreparedQuestCollisionArtifactV1::artifact;
        type FinalizeResult = Result<
            (
                QuestCollisionCapabilityArtifactV1,
                QuestCollisionCatalogInput,
            ),
            PreparedQuestCollisionArtifactFinalizeError,
        >;
        let _finalize: fn(PreparedQuestCollisionArtifactV1, &ProjectRevision2) -> FinalizeResult =
            PreparedQuestCollisionArtifactV1::finalize;
    }

    #[test]
    fn preparing_moves_each_collision_set_once_and_retains_one_bounded_byte_vector() {
        let capability = test_capability();
        let module_bytes = capability.modules.first().unwrap().as_ptr();
        let path_bytes = capability.relative_paths.first().unwrap().as_ptr();
        let symbol_bytes = capability.symbols.first().unwrap().as_ptr();

        let prepared = capability.prepare_artifact().unwrap();

        assert_eq!(
            prepared.capability.modules.first().unwrap().as_ptr(),
            module_bytes
        );
        assert_eq!(
            prepared.capability.relative_paths.first().unwrap().as_ptr(),
            path_bytes
        );
        assert_eq!(
            prepared.capability.symbols.first().unwrap().as_ptr(),
            symbol_bytes
        );
        assert!(!prepared.artifact().canonical_json().is_empty());
        assert!(prepared.artifact().canonical_json().len() <= MAX_INVENTORY_JSON_BYTES);
    }

    #[test]
    fn prepared_finalize_rechecks_every_artifact_and_capability_identity_gate() {
        macro_rules! artifact_drift {
            ($mutation:expr, $pattern:pat) => {{
                let mut prepared = test_capability().prepare_artifact().unwrap();
                ($mutation)(&mut prepared.artifact);
                assert!(matches!(finalized_verification_error(prepared), $pattern));
            }};
        }

        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.artifact_seal.sha256 = crate::Sha256Digest::from_bytes([0x81; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::RawArtifactSealMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.source_seal.sha256 = crate::Sha256Digest::from_bytes([0x82; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::SemanticSourceSealMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.base_inventory_payload_seal.sha256 =
                    crate::Sha256Digest::from_bytes([0x83; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::BaseInventoryPayloadSealMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.story_catalog_seal.sha256 = crate::Sha256Digest::from_bytes([0x84; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::StoryCatalogSealMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.project_id = ProjectId::from_bytes([0x85; 16]);
            },
            QuestCollisionCapabilityArtifactVerificationError::ProjectIdMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.project_revision += 1;
            },
            QuestCollisionCapabilityArtifactVerificationError::ProjectRevisionMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.project_target.executable.sha256 =
                    AuthoringSha256Digest::from_bytes([0x86; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::ProjectTargetMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                artifact.canonical_project.sha256 = AuthoringSha256Digest::from_bytes([0x87; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::CanonicalProjectMismatch
        );
        artifact_drift!(
            |artifact: &mut QuestCollisionCapabilityArtifactV1| {
                let index = artifact
                    .canonical_json
                    .iter()
                    .position(|byte| *byte == b'p')
                    .unwrap();
                artifact.canonical_json[index] = b'q';
            },
            QuestCollisionCapabilityArtifactVerificationError::CanonicalIdentityMismatch
        );

        macro_rules! capability_drift {
            ($mutation:expr, $pattern:pat) => {{
                let mut prepared = test_capability().prepare_artifact().unwrap();
                ($mutation)(&mut prepared.capability);
                assert!(matches!(finalized_verification_error(prepared), $pattern));
            }};
        }

        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.base_inventory_payload_seal.sha256 =
                    crate::Sha256Digest::from_bytes([0x91; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::BaseInventoryPayloadSealMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.story_catalog_seal.sha256 = crate::Sha256Digest::from_bytes([0x92; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::StoryCatalogSealMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.project_id = ProjectId::from_bytes([0x93; 16]);
            },
            QuestCollisionCapabilityArtifactVerificationError::ProjectIdMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.project_revision += 1;
            },
            QuestCollisionCapabilityArtifactVerificationError::ProjectRevisionMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.project_target.executable.sha256 =
                    AuthoringSha256Digest::from_bytes([0x94; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::ProjectTargetMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.canonical_project.sha256 = AuthoringSha256Digest::from_bytes([0x95; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::CanonicalProjectMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.modules.insert("unexpected.module".to_owned());
            },
            QuestCollisionCapabilityArtifactVerificationError::SemanticSourceSealMismatch
        );
        capability_drift!(
            |capability: &mut VerifiedQuestCollisionCapability| {
                capability.combined_source_seal.sha256 =
                    crate::Sha256Digest::from_bytes([0x96; 32]);
            },
            QuestCollisionCapabilityArtifactVerificationError::SemanticSourceSealMismatch
        );
    }

    fn canonical_limit_capability() -> VerifiedQuestCollisionCapability {
        let mut capability = test_capability();
        let escaped_suffix = "\\".repeat(MAX_COLLISION_ENTRY_BYTES - 12);
        capability.modules = (0..(MAX_COLLISION_TOTAL_BYTES / MAX_COLLISION_ENTRY_BYTES))
            .map(|index| format!("{index:012}{escaped_suffix}"))
            .collect();
        capability.relative_paths.clear();
        capability.symbols.clear();
        capability
    }

    #[test]
    fn prepared_capsule_inherits_the_legacy_canonical_artifact_limit() {
        let prepared_error = match canonical_limit_capability().prepare_artifact() {
            Err(error) => error,
            Ok(_) => panic!("oversized prepared artifact unexpectedly succeeded"),
        };
        for error in [
            canonical_limit_capability().into_artifact().unwrap_err(),
            prepared_error,
        ] {
            assert!(matches!(
                error,
                QuestCollisionCapabilityArtifactError::Limit {
                    kind: "canonical artifact JSON bytes",
                    max: MAX_INVENTORY_JSON_BYTES,
                    ..
                }
            ));
        }
    }

    #[test]
    fn artifact_is_deterministic_content_addressed_and_reopenable() {
        let expected_source_seal = test_capability().combined_source_seal().clone();
        let first = test_capability().into_artifact().unwrap();
        let second = test_capability().into_artifact().unwrap();

        assert_eq!(first, second);
        assert_eq!(first.source_seal(), &expected_source_seal);
        assert_eq!(
            first.artifact_seal(),
            &raw_artifact_seal(first.canonical_json())
        );
        assert_eq!(first.artifact_seal().byte_len, first.source_seal().byte_len);
        assert_ne!(first.artifact_seal().sha256, first.source_seal().sha256);
        assert_eq!(first.project_id(), ProjectId::from_bytes([3; 16]));
        assert_eq!(first.project_revision(), 9);
        assert_eq!(
            first.coverage(),
            QuestCollisionCoverage::BaseGameAndExactProjectOnly
        );
        assert_eq!(first.build_status(), QuestCollisionBuildStatus::Blocked);
        assert_eq!(
            first.publication_status(),
            QuestCollisionPublicationStatus::NotSupported
        );

        let reopened = reopen_quest_collision_capability_artifact_v1(
            first.canonical_json(),
            first.artifact_seal(),
            first.source_seal(),
        )
        .unwrap();
        assert_eq!(reopened, first);
        assert_eq!(reopened.into_canonical_json(), first.canonical_json());
    }

    #[test]
    fn only_a_fresh_source_bound_capability_can_authorize_an_exact_reopened_artifact() {
        let artifact = test_capability().into_artifact().unwrap();
        let reopened = reopen_quest_collision_capability_artifact_v1(
            artifact.canonical_json(),
            artifact.artifact_seal(),
            artifact.source_seal(),
        )
        .unwrap();

        let authoritative = test_capability().verify_artifact_exact(&reopened).unwrap();
        assert!(authoritative.contains_module("PROJECT.MODULE"));
        assert!(authoritative.contains_relative_path("PROJECT/MODULE.AS"));
        assert!(authoritative.contains_symbol("UPROJECT"));
        assert_eq!(authoritative.combined_source_seal(), reopened.source_seal());
        // Verification borrowed the opaque artifact; it remains structural and reusable only as
        // an identity input to another independently source-bound capability.
        assert_eq!(reopened, artifact);
    }

    #[test]
    fn exact_bridge_distinguishes_raw_semantic_and_canonical_identity_mismatches() {
        let mut wrong_raw = test_capability().into_artifact().unwrap();
        wrong_raw.artifact_seal.sha256 = crate::Sha256Digest::from_bytes([0x81; 32]);
        assert!(matches!(
            test_capability().verify_artifact_exact(&wrong_raw),
            Err(QuestCollisionCapabilityArtifactVerificationError::RawArtifactSealMismatch)
        ));

        let mut wrong_semantic = test_capability().into_artifact().unwrap();
        wrong_semantic.source_seal.sha256 = crate::Sha256Digest::from_bytes([0x82; 32]);
        assert!(matches!(
            test_capability().verify_artifact_exact(&wrong_semantic),
            Err(QuestCollisionCapabilityArtifactVerificationError::SemanticSourceSealMismatch)
        ));

        let mut wrong_canonical = test_capability().into_artifact().unwrap();
        let canonical = String::from_utf8(wrong_canonical.canonical_json).unwrap();
        let mutated = canonical.replacen("project.module", "project.modulf", 1);
        assert_ne!(mutated, canonical);
        wrong_canonical.canonical_json = mutated.into_bytes();
        wrong_canonical.artifact_seal = raw_artifact_seal(&wrong_canonical.canonical_json);
        assert!(matches!(
            test_capability().verify_artifact_exact(&wrong_canonical),
            Err(QuestCollisionCapabilityArtifactVerificationError::CanonicalIdentityMismatch)
        ));

        let mut changed_collision = test_capability().into_artifact().unwrap();
        let canonical = String::from_utf8(changed_collision.canonical_json).unwrap();
        changed_collision.canonical_json = canonical
            .replacen("project.module", "project.modulf", 1)
            .into_bytes();
        changed_collision.artifact_seal = raw_artifact_seal(&changed_collision.canonical_json);
        changed_collision.source_seal =
            seal_combined_payload_bytes(&changed_collision.canonical_json);
        assert!(matches!(
            test_capability().verify_artifact_exact(&changed_collision),
            Err(QuestCollisionCapabilityArtifactVerificationError::SemanticSourceSealMismatch)
        ));
    }

    fn assert_provenance_mismatch(
        mutate: impl FnOnce(&mut QuestCollisionCapabilityArtifactV1),
        matches_expected: impl FnOnce(&QuestCollisionCapabilityArtifactVerificationError) -> bool,
    ) {
        let mut artifact = test_capability().into_artifact().unwrap();
        mutate(&mut artifact);
        let error = test_capability()
            .verify_artifact_exact(&artifact)
            .unwrap_err();
        assert!(matches_expected(&error), "unexpected error: {error}");
    }

    #[test]
    fn exact_bridge_checks_every_retained_bounded_provenance_field() {
        assert_provenance_mismatch(
            |artifact| {
                artifact.base_inventory_payload_seal.sha256 =
                    crate::Sha256Digest::from_bytes([0x91; 32]);
            },
            |error| {
                matches!(error, QuestCollisionCapabilityArtifactVerificationError::BaseInventoryPayloadSealMismatch)
            },
        );
        assert_provenance_mismatch(
            |artifact| {
                artifact.story_catalog_seal.sha256 = crate::Sha256Digest::from_bytes([0x92; 32]);
            },
            |error| {
                matches!(
                    error,
                    QuestCollisionCapabilityArtifactVerificationError::StoryCatalogSealMismatch
                )
            },
        );
        assert_provenance_mismatch(
            |artifact| artifact.project_id = ProjectId::from_bytes([0x93; 16]),
            |error| {
                matches!(
                    error,
                    QuestCollisionCapabilityArtifactVerificationError::ProjectIdMismatch
                )
            },
        );
        assert_provenance_mismatch(
            |artifact| artifact.project_revision += 1,
            |error| {
                matches!(
                    error,
                    QuestCollisionCapabilityArtifactVerificationError::ProjectRevisionMismatch
                )
            },
        );
        assert_provenance_mismatch(
            |artifact| {
                artifact.project_target.executable.sha256 =
                    AuthoringSha256Digest::from_bytes([0x94; 32]);
            },
            |error| {
                matches!(
                    error,
                    QuestCollisionCapabilityArtifactVerificationError::ProjectTargetMismatch
                )
            },
        );
        assert_provenance_mismatch(
            |artifact| {
                artifact.canonical_project.sha256 = AuthoringSha256Digest::from_bytes([0x95; 32]);
            },
            |error| {
                matches!(
                    error,
                    QuestCollisionCapabilityArtifactVerificationError::CanonicalProjectMismatch
                )
            },
        );
    }

    #[test]
    fn reopen_requires_both_external_seals_and_exact_lengths() {
        let artifact = test_capability().into_artifact().unwrap();
        let mut wrong_raw_length = artifact.artifact_seal().clone();
        wrong_raw_length.byte_len += 1;
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v1(
                artifact.canonical_json(),
                &wrong_raw_length,
                artifact.source_seal(),
            ),
            Err(QuestCollisionCapabilityArtifactError::SealLengthMismatch {
                kind: "raw artifact",
                ..
            })
        ));

        let mut wrong_source_length = artifact.source_seal().clone();
        wrong_source_length.byte_len += 1;
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v1(
                artifact.canonical_json(),
                artifact.artifact_seal(),
                &wrong_source_length,
            ),
            Err(QuestCollisionCapabilityArtifactError::SealLengthMismatch {
                kind: "semantic source",
                ..
            })
        ));

        let mut wrong_raw_digest = artifact.artifact_seal().clone();
        wrong_raw_digest.sha256 = crate::Sha256Digest::from_bytes([0x91; 32]);
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v1(
                artifact.canonical_json(),
                &wrong_raw_digest,
                artifact.source_seal(),
            ),
            Err(QuestCollisionCapabilityArtifactError::ArtifactSealMismatch)
        ));

        let mut wrong_source_digest = artifact.source_seal().clone();
        wrong_source_digest.sha256 = crate::Sha256Digest::from_bytes([0x92; 32]);
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v1(
                artifact.canonical_json(),
                artifact.artifact_seal(),
                &wrong_source_digest,
            ),
            Err(QuestCollisionCapabilityArtifactError::SourceSealMismatch)
        ));
    }

    #[test]
    fn reopen_rejects_noncanonical_duplicate_unknown_and_fixed_claim_tampering() {
        let artifact = test_capability().into_artifact().unwrap();
        let canonical = std::str::from_utf8(artifact.canonical_json()).unwrap();

        let whitespace = format!(" {canonical}");
        assert!(matches!(
            reopen_with_actual_seals(whitespace.as_bytes()),
            Err(QuestCollisionCapabilityArtifactError::NonCanonicalJson)
        ));

        let duplicate = canonical.replacen(
            "{\"format\":",
            "{\"format\":\"quest_collision_capability\",\"format\":",
            1,
        );
        assert!(matches!(
            reopen_with_actual_seals(duplicate.as_bytes()),
            Err(QuestCollisionCapabilityArtifactError::InvalidJson(_))
        ));

        let unknown = canonical.replacen("{\"format\":", "{\"unknown\":0,\"format\":", 1);
        assert!(matches!(
            reopen_with_actual_seals(unknown.as_bytes()),
            Err(QuestCollisionCapabilityArtifactError::InvalidJson(_))
        ));

        for (from, to) in [
            (
                "\"format\":\"quest_collision_capability\"",
                "\"format\":\"forged\"",
            ),
            ("\"build_status\":\"blocked\"", "\"build_status\":\"ready\""),
            (
                "\"publication_status\":\"not_supported\"",
                "\"publication_status\":\"supported\"",
            ),
        ] {
            let forged = canonical.replacen(from, to, 1);
            assert!(matches!(
                reopen_with_actual_seals(forged.as_bytes()),
                Err(QuestCollisionCapabilityArtifactError::Invariant(_))
            ));
        }
    }

    #[test]
    fn reopen_rejects_non_strict_and_unsafe_collision_entries() {
        let artifact = test_capability().into_artifact().unwrap();
        let canonical = std::str::from_utf8(artifact.canonical_json()).unwrap();
        for replacement in [
            "\"modules\":[\"project.module\",\"base.module\"]",
            "\"modules\":[\"base.module\",\"base.module\"]",
            "\"modules\":[\"Base.module\",\"project.module\"]",
        ] {
            let tampered = canonical.replacen(
                "\"modules\":[\"base.module\",\"project.module\"]",
                replacement,
                1,
            );
            assert!(matches!(
                reopen_with_actual_seals(tampered.as_bytes()),
                Err(QuestCollisionCapabilityArtifactError::InvalidJson(_))
            ));
        }

        let overlong = "a".repeat(MAX_COLLISION_ENTRY_BYTES + 1);
        let tampered = canonical.replacen("base.module", &overlong, 1);
        assert!(matches!(
            reopen_with_actual_seals(tampered.as_bytes()),
            Err(QuestCollisionCapabilityArtifactError::InvalidJson(_))
        ));
    }

    #[test]
    fn artifact_limits_cover_raw_tokens_shared_counts_and_aggregate_bytes() {
        let mut raw_token = Vec::with_capacity(MAX_JSON_STRING_TOKEN_BYTES + 3);
        raw_token.push(b'"');
        raw_token.extend(std::iter::repeat_n(b'a', MAX_JSON_STRING_TOKEN_BYTES + 1));
        raw_token.push(b'"');
        assert!(matches!(
            preflight_json_string_tokens(&raw_token),
            Err(QuestCollisionCapabilityArtifactError::Limit {
                kind: "raw JSON string token bytes",
                actual,
                max: MAX_JSON_STRING_TOKEN_BYTES,
            }) if actual == MAX_JSON_STRING_TOKEN_BYTES + 1
        ));

        let artifact = test_capability().into_artifact().unwrap();
        let mut wire: CombinedArtifactWire =
            serde_json::from_slice(artifact.canonical_json()).unwrap();
        wire.modules = (0..60_000).map(|index| format!("m{index:06}")).collect();
        wire.symbols = (0..40_001).map(|index| format!("s{index:06}")).collect();
        wire.relative_paths.clear();
        assert!(matches!(
            validate_artifact_wire(&wire),
            Err(QuestCollisionCapabilityArtifactError::Limit {
                kind: "collision entry count",
                actual: 100_001,
                max: MAX_COLLISION_ENTRIES,
            })
        ));
        let over_count_json = canonical_artifact_wire(&wire).unwrap();
        assert!(matches!(
            reopen_with_actual_seals(&over_count_json),
            Err(QuestCollisionCapabilityArtifactError::InvalidJson(_))
        ));

        let chunk = "a".repeat(MAX_COLLISION_ENTRY_BYTES - 8);
        wire.modules.clear();
        wire.symbols.clear();
        wire.relative_paths = (0..=(MAX_COLLISION_TOTAL_BYTES / MAX_COLLISION_ENTRY_BYTES))
            .map(|index| format!("{index:08}{chunk}"))
            .collect();
        assert!(matches!(
            validate_artifact_wire(&wire),
            Err(QuestCollisionCapabilityArtifactError::Limit {
                kind: "aggregate collision entry bytes",
                ..
            })
        ));
        let over_bytes_json = canonical_artifact_wire(&wire).unwrap();
        assert!(matches!(
            reopen_with_actual_seals(&over_bytes_json),
            Err(QuestCollisionCapabilityArtifactError::InvalidJson(_))
        ));
    }

    #[test]
    fn streaming_seal_preserves_the_exact_canonical_payload_semantics() {
        let base_inventory_payload_seal = ContentSeal {
            byte_len: 11,
            sha256: crate::Sha256Digest::from_bytes([1; 32]),
        };
        let story_catalog_seal = ContentSeal {
            byte_len: 12,
            sha256: crate::Sha256Digest::from_bytes([2; 32]),
        };
        let project_id = ProjectId::from_bytes([3; 16]);
        let project_revision = 9;
        let project_target = GameGenerationAnchor {
            executable: AuthoringContentSeal {
                byte_len: 13,
                sha256: AuthoringSha256Digest::from_bytes([4; 32]),
            },
        };
        let canonical_project = AuthoringContentSeal {
            byte_len: 14,
            sha256: AuthoringSha256Digest::from_bytes([5; 32]),
        };
        let modules = BTreeSet::from(["base.module".to_owned(), "project.module".to_owned()]);
        let relative_paths =
            BTreeSet::from(["base/module.as".to_owned(), "project/module.as".to_owned()]);
        let symbols = BTreeSet::from(["ubase".to_owned(), "uproject".to_owned()]);

        let payload = CombinedPayload {
            format: COMBINED_FORMAT,
            schema_revision: COMBINED_SCHEMA_REVISION,
            coverage: QuestCollisionCoverage::BaseGameAndExactProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &base_inventory_payload_seal,
            story_catalog_seal: &story_catalog_seal,
            project_id,
            project_revision,
            project_target: &project_target,
            canonical_project: &canonical_project,
            modules: &modules,
            relative_paths: &relative_paths,
            symbols: &symbols,
        };
        let streamed = seal_combined_payload(&payload).unwrap();
        let reference = serde_json::to_vec(&payload).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(COMBINED_SEAL_DOMAIN);
        hasher.update((reference.len() as u64).to_be_bytes());
        hasher.update(&reference);
        assert_eq!(streamed.byte_len, reference.len() as u64);
        assert_eq!(streamed.byte_len, 950);
        assert_eq!(
            streamed.sha256,
            crate::Sha256Digest::from_bytes(hasher.finalize().into())
        );
        assert_eq!(
            serde_json::to_value(streamed.sha256).unwrap(),
            serde_json::json!("4c6f8868c2dd31ac881d7afae79f544f3d1ccee8495286a76737b7bfc57efa3c")
        );
    }

    fn exact_writer_matches(expected: &[u8], chunks: &[&[u8]]) -> bool {
        let mut writer = ExactCanonicalIdentityWriter::new(expected, MAX_INVENTORY_JSON_BYTES);
        for chunk in chunks {
            writer.write_all(chunk).unwrap();
        }
        writer.canonical_matches && writer.bytes_written == expected.len()
    }

    #[test]
    fn exact_identity_writer_handles_chunk_boundaries_and_length_drift() {
        let expected = b"0123456789abcdef";
        assert!(exact_writer_matches(
            expected,
            &[b"0", b"12345", b"6789", b"abcdef"]
        ));
        assert!(!exact_writer_matches(expected, &[b"0123456789abcdee"]));
        assert!(!exact_writer_matches(expected, &[b"0123456789abcde"]));
        assert!(!exact_writer_matches(
            expected,
            &[b"01234567", b"89abcdef", b"x"]
        ));
        assert!(exact_writer_matches(
            expected,
            &[b"0123456", b"789abcde", b"f"]
        ));
    }

    #[test]
    fn combined_union_is_bounded_by_count_and_aggregate_bytes() {
        let too_many = (0..=MAX_COLLISION_ENTRIES)
            .map(|index| format!("module{index:06}"))
            .collect::<BTreeSet<_>>();
        assert!(matches!(
            enforce_combined_limits(&too_many, &BTreeSet::new(), &BTreeSet::new()),
            Err(QuestCollisionCapabilityError::Limit {
                kind: "entry count",
                actual,
                max: MAX_COLLISION_ENTRIES,
            }) if actual == MAX_COLLISION_ENTRIES + 1
        ));

        let chunk = "a".repeat(crate::MAX_COLLISION_ENTRY_BYTES - 8);
        let too_large = (0..=(MAX_COLLISION_TOTAL_BYTES / crate::MAX_COLLISION_ENTRY_BYTES))
            .map(|index| format!("{index:08}{chunk}"))
            .collect::<BTreeSet<_>>();
        assert!(too_large.len() < MAX_COLLISION_ENTRIES);
        assert!(matches!(
            enforce_combined_limits(&BTreeSet::new(), &BTreeSet::new(), &too_large),
            Err(QuestCollisionCapabilityError::Limit {
                kind: "aggregate entry bytes",
                actual,
                max: MAX_COLLISION_TOTAL_BYTES,
            }) if actual > MAX_COLLISION_TOTAL_BYTES
        ));
    }
}
