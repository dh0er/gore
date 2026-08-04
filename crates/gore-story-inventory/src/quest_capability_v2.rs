//! Closed base-game plus exact-current revision-3 Quest collision capability.
//!
//! Its only project authority input is an opaque [`PreparedRevision3QuestCollisionSourceV2`]
//! produced by the working store from one exact current head. Historical Quest artifacts and
//! their historical basis snapshots are never authority inputs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};

use gore_authoring::{
    ContentSeal as AuthoringContentSeal, EntityId, GameGenerationAnchor,
    PreparedRevision3QuestCollisionInspectionSourceV2, PreparedRevision3QuestCollisionSourceV2,
    ProjectId, QuestCollisionCatalogInput, Revision3NonQuestCollisionBasisV2,
    Revision3PriorQuestEvidenceV2, Revision3QuestGiverInput as QuestGiverInput,
    Revision3QuestParentInput as QuestParentInput, Sha256Digest as AuthoringSha256Digest,
    WorkingHead, MAX_PROJECT_JSON_BYTES, MAX_QUEST_COLLISION_ARTIFACT_BYTES,
    MAX_REVISION3_COLLISION_IDENTITIES_V2, MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2,
    MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2, MAX_REVISION3_PRIOR_QUESTS_V2,
    MAX_REVISION3_SNAPSHOT_BYTES, QUEST_COLLISION_CATALOG_LAYER_V2,
};
use gore_story_catalog::{CatalogError, StoryCatalogFile, MAX_CATALOG_JSON_BYTES};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

use super::{
    BaseGameCollisionInventory, ContentSeal, MAX_COLLISION_ENTRIES, MAX_COLLISION_ENTRY_BYTES,
    MAX_COLLISION_TOTAL_BYTES, MAX_INVENTORY_JSON_BYTES,
};

/// Offline collision inspection does not qualify runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionRuntimeQualification {
    RuntimeUnqualified,
}

/// Collision inspection alone cannot authorize a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionBuildStatus {
    Blocked,
}

/// Publication remains outside this capability boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCollisionPublicationStatus {
    NotSupported,
}

/// Honest layer identity: pristine base game plus one exact current revision-3 project.
pub const BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2: &str =
    QUEST_COLLISION_CATALOG_LAYER_V2;

const COMBINED_FORMAT_V2: &str = "quest_collision_capability";
const COMBINED_SCHEMA_REVISION_V2: u32 = 2;
const COMBINED_SEAL_DOMAIN_V2: &[u8] =
    b"gore-story-inventory.quest-collision-capability.v2.exact-current-revision3-payload\0";
const MAX_COMBINED_MARKER_BYTES_V2: usize = 128;
const MAX_JSON_STRING_TOKEN_BYTES_V2: usize = MAX_COLLISION_ENTRY_BYTES * 6;

const _: () = {
    assert!(MAX_INVENTORY_JSON_BYTES as u64 == MAX_QUEST_COLLISION_ARTIFACT_BYTES);
    assert!(MAX_COLLISION_ENTRIES == MAX_REVISION3_COLLISION_IDENTITIES_V2);
    assert!(MAX_COLLISION_TOTAL_BYTES == MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2);
    assert!(MAX_COLLISION_ENTRY_BYTES == MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2);
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3QuestCollisionCoverageV2 {
    BaseGameAndExactRevision3ProjectOnly,
}

/// Fresh, opaque collision capability bound to one exact current revision-3 head.
///
/// The complete source capsule is retained and consumed with this value. This type has no
/// `Clone`, deserializer, or public constructor. Structural artifacts can only be checked by
/// consuming an independently fresh instance through [`Self::verify_artifact_exact`].
pub struct VerifiedRevision3QuestCollisionCapabilityV2 {
    source: PreparedRevision3QuestCollisionSourceV2,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    combined_source_seal: ContentSeal,
    modules: BTreeSet<String>,
    relative_paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
    parents: BTreeMap<String, QuestParentInput>,
    givers: BTreeMap<String, QuestGiverInput>,
}

/// Linear, inspection-only capability bound to one immutable historical version-2 source.
///
/// Unlike [`VerifiedRevision3QuestCollisionCapabilityV2`], this type cannot resolve catalog
/// selections, create an artifact, or enter any authoring transaction. Its sole consuming
/// operation verifies one already-persisted artifact and the Quest identities that artifact was
/// originally used with, then returns only a plain source-generation input.
///
/// ```compile_fail
/// use gore_authoring::PreparedRevision3QuestCollisionInspectionSourceV2;
/// use gore_story_catalog::StoryCatalogFile;
/// use gore_story_inventory::{
///     BaseGameCollisionInventory, VerifiedRevision3QuestCollisionCapabilityV2,
/// };
/// fn cannot_rebind_historical_source_as_authoring(
///     base: BaseGameCollisionInventory,
///     catalog: &StoryCatalogFile,
///     source: PreparedRevision3QuestCollisionInspectionSourceV2,
/// ) {
///     let _ = VerifiedRevision3QuestCollisionCapabilityV2::bind(base, catalog, source);
/// }
/// ```
///
/// ```compile_fail
/// use gore_story_inventory::VerifiedRevision3QuestCollisionInspectionCapabilityV2;
/// fn cannot_create_artifact(
///     capability: VerifiedRevision3QuestCollisionInspectionCapabilityV2,
/// ) {
///     let _ = capability.prepare_artifact();
/// }
/// ```
pub struct VerifiedRevision3QuestCollisionInspectionCapabilityV2 {
    source: PreparedRevision3QuestCollisionInspectionSourceV2,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    combined_source_seal: ContentSeal,
    modules: BTreeSet<String>,
    relative_paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
    parents: BTreeMap<String, QuestParentInput>,
    givers: BTreeMap<String, QuestGiverInput>,
}

/// Opaque structural form of one exact-current revision-3 collision capability.
///
/// Reopening proves bounded canonical structure plus the supplied raw and semantic seals. It
/// never rehydrates source authority, build readiness, runtime qualification, or publication
/// support.
pub struct QuestCollisionCapabilityArtifactV2 {
    canonical_json: Vec<u8>,
    artifact_seal: ContentSeal,
    source_seal: ContentSeal,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    current_head: WorkingHead,
    current_project: AuthoringContentSeal,
    nonquest_project: AuthoringContentSeal,
    prior_quest_count: u64,
    prior_quest_evidence: AuthoringContentSeal,
}

/// Linear bridge retaining one fresh capability beside its exact structural artifact.
///
/// The capsule deliberately has no `Clone`, serialization, public constructor, or public
/// decomposition API. [`Self::finalize`] consumes both halves once and returns only structural
/// artifact data plus a plain generation input, neither of which is authority evidence.
pub struct PreparedQuestCollisionArtifactV2 {
    capability: VerifiedRevision3QuestCollisionCapabilityV2,
    artifact: QuestCollisionCapabilityArtifactV2,
}

impl fmt::Debug for VerifiedRevision3QuestCollisionCapabilityV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedRevision3QuestCollisionCapabilityV2")
            .field("project_id", &self.project_id())
            .field("project_revision", &self.project_revision())
            .field("project_target", self.project_target())
            .field("current_head", self.current_head())
            .field("current_project", self.current_project())
            .field("nonquest_project", self.nonquest_project())
            .field("prior_quest_count", &self.prior_quest_count())
            .field("prior_quest_evidence", self.prior_quest_evidence())
            .field("module_count", &self.modules.len())
            .field("relative_path_count", &self.relative_paths.len())
            .field("symbol_count", &self.symbols.len())
            .finish()
    }
}

impl fmt::Debug for VerifiedRevision3QuestCollisionInspectionCapabilityV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedRevision3QuestCollisionInspectionCapabilityV2")
            .field("project_id", &self.source.project_id())
            .field("project_revision", &self.source.project_revision())
            .field("historical_head", self.source.historical_head())
            .field("retains_selection_authority", &false)
            .field("can_create_artifact", &false)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for QuestCollisionCapabilityArtifactV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuestCollisionCapabilityArtifactV2")
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
            .field("current_head", &self.current_head)
            .field("current_project", &self.current_project)
            .field("nonquest_project", &self.nonquest_project)
            .field("prior_quest_count", &self.prior_quest_count)
            .field("prior_quest_evidence", &self.prior_quest_evidence)
            .finish()
    }
}

impl fmt::Debug for PreparedQuestCollisionArtifactV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedQuestCollisionArtifactV2")
            .field("artifact", &self.artifact)
            .field("retains_fresh_capability", &true)
            .finish()
    }
}

impl PartialEq for QuestCollisionCapabilityArtifactV2 {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_json == other.canonical_json
            && self.artifact_seal == other.artifact_seal
            && self.source_seal == other.source_seal
            && self.base_inventory_payload_seal == other.base_inventory_payload_seal
            && self.story_catalog_seal == other.story_catalog_seal
            && self.project_id == other.project_id
            && self.project_revision == other.project_revision
            && self.project_target == other.project_target
            && self.current_head == other.current_head
            && self.current_project == other.current_project
            && self.nonquest_project == other.nonquest_project
            && self.prior_quest_count == other.prior_quest_count
            && self.prior_quest_evidence == other.prior_quest_evidence
    }
}

impl Eq for QuestCollisionCapabilityArtifactV2 {}

impl QuestCollisionCapabilityArtifactV2 {
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

    pub fn current_head(&self) -> &WorkingHead {
        &self.current_head
    }

    pub fn current_project(&self) -> &AuthoringContentSeal {
        &self.current_project
    }

    pub fn nonquest_project(&self) -> &AuthoringContentSeal {
        &self.nonquest_project
    }

    pub const fn prior_quest_count(&self) -> u64 {
        self.prior_quest_count
    }

    pub fn prior_quest_evidence(&self) -> &AuthoringContentSeal {
        &self.prior_quest_evidence
    }

    pub const fn catalog_layer(&self) -> &'static str {
        BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2
    }

    pub const fn coverage(&self) -> Revision3QuestCollisionCoverageV2 {
        Revision3QuestCollisionCoverageV2::BaseGameAndExactRevision3ProjectOnly
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

    #[cfg(test)]
    pub(crate) fn test_only_with_noncanonical_identity(&self) -> Self {
        let mut canonical_json = self.canonical_json.clone();
        canonical_json.push(b' ');
        Self {
            canonical_json,
            artifact_seal: self.artifact_seal.clone(),
            source_seal: self.source_seal.clone(),
            base_inventory_payload_seal: self.base_inventory_payload_seal.clone(),
            story_catalog_seal: self.story_catalog_seal.clone(),
            project_id: self.project_id,
            project_revision: self.project_revision,
            project_target: self.project_target.clone(),
            current_head: self.current_head.clone(),
            current_project: self.current_project.clone(),
            nonquest_project: self.nonquest_project.clone(),
            prior_quest_count: self.prior_quest_count,
            prior_quest_evidence: self.prior_quest_evidence.clone(),
        }
    }

    #[cfg(test)]
    pub(crate) fn test_only_with_raw_seal_drift(&self) -> Self {
        let mut artifact_seal = self.artifact_seal.clone();
        artifact_seal.sha256 = super::Sha256Digest::from_bytes([0xfd; 32]);
        Self {
            canonical_json: self.canonical_json.clone(),
            artifact_seal,
            source_seal: self.source_seal.clone(),
            base_inventory_payload_seal: self.base_inventory_payload_seal.clone(),
            story_catalog_seal: self.story_catalog_seal.clone(),
            project_id: self.project_id,
            project_revision: self.project_revision,
            project_target: self.project_target.clone(),
            current_head: self.current_head.clone(),
            current_project: self.current_project.clone(),
            nonquest_project: self.nonquest_project.clone(),
            prior_quest_count: self.prior_quest_count,
            prior_quest_evidence: self.prior_quest_evidence.clone(),
        }
    }

    /// Reconstruct the plain collision input encoded by this structural artifact.
    ///
    /// This is crate-private because reopening an artifact must never be mistaken for restoring
    /// the fresh capability consumed by authoring. Callers still receive no authority token.
    pub(crate) fn structural_collision_input(
        &self,
    ) -> Result<QuestCollisionCatalogInput, QuestCollisionCapabilityArtifactErrorV2> {
        let wire: CombinedArtifactWireV2 = serde_json::from_slice(&self.canonical_json)
            .map_err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson)?;
        validate_artifact_wire_v2(&wire)?;
        let canonical = canonical_artifact_wire_v2(&wire)?;
        if canonical != self.canonical_json {
            return Err(QuestCollisionCapabilityArtifactErrorV2::NonCanonicalJson);
        }
        Ok(QuestCollisionCatalogInput {
            generation: wire.project_target,
            source_seal: authoring_seal(&self.source_seal),
            catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2.to_owned(),
            modules: wire.modules.into_iter().collect(),
            relative_paths: wire.relative_paths.into_iter().collect(),
            symbols: wire.symbols.into_iter().collect(),
        })
    }
}

impl PreparedQuestCollisionArtifactV2 {
    #[cfg(test)]
    pub(crate) fn insert_test_parent_selection(
        &mut self,
        catalog_id: String,
        parent: QuestParentInput,
    ) {
        self.capability.parents.insert(catalog_id, parent);
    }

    /// Borrow the exact structural artifact without exposing or duplicating the capability.
    pub const fn artifact(&self) -> &QuestCollisionCapabilityArtifactV2 {
        &self.artifact
    }

    /// Re-verify and consume this capsule once.
    ///
    /// The returned collision input is a plain generator input and is not an artifact, build,
    /// runtime, or publication capability. The retained current-head source is consumed and
    /// cannot be separated or reused.
    pub fn finalize(
        self,
    ) -> Result<
        (
            QuestCollisionCapabilityArtifactV2,
            QuestCollisionCatalogInput,
        ),
        PreparedQuestCollisionArtifactFinalizeErrorV2,
    > {
        let Self {
            capability,
            artifact,
        } = self;
        let capability = capability
            .verify_artifact_exact(&artifact)
            .map_err(PreparedQuestCollisionArtifactFinalizeErrorV2::ArtifactOrCapabilityDrift)?;
        Ok((artifact, capability.into_quest_collision_input()))
    }

    /// Consume the linear capsule at the crate-internal revision-3 transaction boundary.
    ///
    /// This is deliberately not public: callers cannot split structural data from its fresh
    /// capability or use this handoff as an authority constructor. The transaction must still
    /// bind the untrusted current-project transport before it may mutate a local candidate.
    pub(crate) fn into_transaction_authority(
        self,
    ) -> Result<
        (
            VerifiedRevision3QuestCollisionCapabilityV2,
            QuestCollisionCapabilityArtifactV2,
        ),
        Revision3QuestCollisionCapabilityArtifactVerificationErrorV2,
    > {
        let Self {
            capability,
            artifact,
        } = self;
        let capability = capability.verify_artifact_exact(&artifact)?;
        Ok((capability, artifact))
    }
}

trait Revision3QuestCollisionSourceViewV2 {
    fn project_id(&self) -> ProjectId;
    fn project_revision(&self) -> u64;
    fn target(&self) -> &GameGenerationAnchor;
    fn current_head(&self) -> &WorkingHead;
    fn current_project(&self) -> &AuthoringContentSeal;
    fn nonquest_basis(&self) -> &Revision3NonQuestCollisionBasisV2;
    fn prior_quest_count(&self) -> usize;
    fn prior_quest_count_u64(&self) -> u64;
    fn prior_quest_evidence(&self) -> &AuthoringContentSeal;
    fn prior_quests(&self) -> &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2>;
}

impl Revision3QuestCollisionSourceViewV2 for PreparedRevision3QuestCollisionSourceV2 {
    fn project_id(&self) -> ProjectId {
        self.project_id()
    }

    fn project_revision(&self) -> u64 {
        self.project_revision()
    }

    fn target(&self) -> &GameGenerationAnchor {
        self.target()
    }

    fn current_head(&self) -> &WorkingHead {
        self.current_head()
    }

    fn current_project(&self) -> &AuthoringContentSeal {
        self.current_project()
    }

    fn nonquest_basis(&self) -> &Revision3NonQuestCollisionBasisV2 {
        self.nonquest_basis()
    }

    fn prior_quest_count(&self) -> usize {
        self.prior_quest_count()
    }

    fn prior_quest_count_u64(&self) -> u64 {
        self.prior_quest_count_u64()
    }

    fn prior_quest_evidence(&self) -> &AuthoringContentSeal {
        self.prior_quest_evidence()
    }

    fn prior_quests(&self) -> &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2> {
        self.prior_quests()
    }
}

impl Revision3QuestCollisionSourceViewV2 for PreparedRevision3QuestCollisionInspectionSourceV2 {
    fn project_id(&self) -> ProjectId {
        self.project_id()
    }

    fn project_revision(&self) -> u64 {
        self.project_revision()
    }

    fn target(&self) -> &GameGenerationAnchor {
        self.target()
    }

    fn current_head(&self) -> &WorkingHead {
        self.historical_head()
    }

    fn current_project(&self) -> &AuthoringContentSeal {
        self.historical_project()
    }

    fn nonquest_basis(&self) -> &Revision3NonQuestCollisionBasisV2 {
        self.nonquest_basis()
    }

    fn prior_quest_count(&self) -> usize {
        self.prior_quest_count()
    }

    fn prior_quest_count_u64(&self) -> u64 {
        self.prior_quest_count_u64()
    }

    fn prior_quest_evidence(&self) -> &AuthoringContentSeal {
        self.prior_quest_evidence()
    }

    fn prior_quests(&self) -> &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2> {
        self.prior_quests()
    }
}

struct BoundRevision3QuestCollisionInputsV2 {
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    combined_source_seal: ContentSeal,
    modules: BTreeSet<String>,
    relative_paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
    parents: BTreeMap<String, QuestParentInput>,
    givers: BTreeMap<String, QuestGiverInput>,
}

fn bind_revision3_quest_collision_inputs_v2<S>(
    base: BaseGameCollisionInventory,
    catalog: &StoryCatalogFile,
    source: &S,
) -> Result<BoundRevision3QuestCollisionInputsV2, Revision3QuestCollisionCapabilityErrorV2>
where
    S: Revision3QuestCollisionSourceViewV2,
{
    let selections = catalog.authoring_selections()?;
    if base.generation() != catalog.generation()
        || base.story_catalog_seal() != catalog.catalog_seal()
    {
        return Err(Revision3QuestCollisionCapabilityErrorV2::CatalogBindingMismatch);
    }
    let expected_target = authoring_generation(catalog.generation());
    if source.target() != &expected_target {
        return Err(Revision3QuestCollisionCapabilityErrorV2::TargetMismatch);
    }
    validate_source_bindings(source)?;

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
        .collect::<BTreeMap<_, _>>();
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
        .collect::<BTreeMap<_, _>>();

    for prior in source.prior_quests().values() {
        if !parents.values().any(|parent| parent == prior.parent()) {
            return Err(
                Revision3QuestCollisionCapabilityErrorV2::PriorQuestParentDrift {
                    quest: prior.quest_id(),
                },
            );
        }
        if !givers.values().any(|giver| giver == prior.giver()) {
            return Err(
                Revision3QuestCollisionCapabilityErrorV2::PriorQuestGiverDrift {
                    quest: prior.quest_id(),
                },
            );
        }
    }

    let base_inventory_payload_seal = base.payload_seal().clone();
    let story_catalog_seal = base.story_catalog_seal().clone();
    let (base_modules, base_relative_paths, base_symbols) = base.into_collision_domains();
    let mut budget = CollisionBudgetV2::default();
    let mut modules = base_domain_map("module", base_modules, &mut budget)?;
    let mut relative_paths = base_domain_map("relative path", base_relative_paths, &mut budget)?;
    let mut symbols = base_domain_map("symbol", base_symbols, &mut budget)?;

    let nonquest = source.nonquest_basis().story_identities();
    merge_current_domain(
        "module",
        &mut modules,
        nonquest
            .modules()
            .iter()
            .map(|(value, owner)| (value.as_str(), *owner)),
        &mut budget,
    )?;
    merge_current_domain(
        "relative path",
        &mut relative_paths,
        nonquest
            .relative_paths()
            .iter()
            .map(|(value, owner)| (value.as_str(), *owner)),
        &mut budget,
    )?;
    merge_current_domain(
        "symbol",
        &mut symbols,
        nonquest
            .symbols()
            .iter()
            .map(|(value, owner)| (value.as_str(), *owner)),
        &mut budget,
    )?;
    for prior in source.prior_quests().values() {
        merge_current_domain(
            "module",
            &mut modules,
            std::iter::once((prior.module_namespace(), prior.quest_id())),
            &mut budget,
        )?;
        merge_current_domain(
            "relative path",
            &mut relative_paths,
            std::iter::once((prior.module_relative_path(), prior.quest_id())),
            &mut budget,
        )?;
        merge_current_domain(
            "symbol",
            &mut symbols,
            prior
                .symbols()
                .iter()
                .map(|symbol| (symbol.as_str(), prior.quest_id())),
            &mut budget,
        )?;
    }

    let modules = modules.into_keys().collect::<BTreeSet<_>>();
    let relative_paths = relative_paths.into_keys().collect::<BTreeSet<_>>();
    let symbols = symbols.into_keys().collect::<BTreeSet<_>>();
    debug_assert_eq!(
        budget.count,
        modules.len() + relative_paths.len() + symbols.len()
    );

    let combined_source_seal = seal_combined_payload_v2(&CombinedPayloadV2 {
        format: COMBINED_FORMAT_V2,
        schema_revision: COMBINED_SCHEMA_REVISION_V2,
        coverage: Revision3QuestCollisionCoverageV2::BaseGameAndExactRevision3ProjectOnly,
        catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
        runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
        build_status: QuestCollisionBuildStatus::Blocked,
        publication_status: QuestCollisionPublicationStatus::NotSupported,
        base_inventory_payload_seal: &base_inventory_payload_seal,
        story_catalog_seal: &story_catalog_seal,
        project_id: source.project_id(),
        project_revision: source.project_revision(),
        project_target: source.target(),
        current_head: source.current_head(),
        current_project: source.current_project(),
        nonquest_project: source.nonquest_basis().canonical_project(),
        prior_quest_count: source.prior_quest_count_u64(),
        prior_quest_evidence: source.prior_quest_evidence(),
        modules: &modules,
        relative_paths: &relative_paths,
        symbols: &symbols,
    })?;

    Ok(BoundRevision3QuestCollisionInputsV2 {
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

impl VerifiedRevision3QuestCollisionCapabilityV2 {
    /// Bind fresh base/catalog capabilities to one opaque exact-current revision-3 source.
    pub fn bind(
        base: BaseGameCollisionInventory,
        catalog: &StoryCatalogFile,
        source: PreparedRevision3QuestCollisionSourceV2,
    ) -> Result<Self, Revision3QuestCollisionCapabilityErrorV2> {
        let bound = bind_revision3_quest_collision_inputs_v2(base, catalog, &source)?;

        Ok(Self {
            source,
            base_inventory_payload_seal: bound.base_inventory_payload_seal,
            story_catalog_seal: bound.story_catalog_seal,
            combined_source_seal: bound.combined_source_seal,
            modules: bound.modules,
            relative_paths: bound.relative_paths,
            symbols: bound.symbols,
            parents: bound.parents,
            givers: bound.givers,
        })
    }

    pub fn project_id(&self) -> ProjectId {
        self.source.project_id()
    }

    pub fn project_revision(&self) -> u64 {
        self.source.project_revision()
    }

    pub fn project_target(&self) -> &GameGenerationAnchor {
        self.source.target()
    }

    pub fn current_head(&self) -> &WorkingHead {
        self.source.current_head()
    }

    pub fn current_project(&self) -> &AuthoringContentSeal {
        self.source.current_project()
    }

    pub fn nonquest_project(&self) -> &AuthoringContentSeal {
        self.source.nonquest_basis().canonical_project()
    }

    pub fn prior_quest_count(&self) -> u64 {
        self.source.prior_quest_count_u64()
    }

    pub fn prior_quest_evidence(&self) -> &AuthoringContentSeal {
        self.source.prior_quest_evidence()
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
        BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2
    }

    pub const fn coverage(&self) -> Revision3QuestCollisionCoverageV2 {
        Revision3QuestCollisionCoverageV2::BaseGameAndExactRevision3ProjectOnly
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
        bounded_collision_query_v2(value).is_some_and(|canonical| self.modules.contains(&canonical))
    }

    pub fn contains_relative_path(&self, value: &str) -> bool {
        bounded_collision_query_v2(value)
            .is_some_and(|canonical| self.relative_paths.contains(&canonical))
    }

    pub fn contains_symbol(&self, value: &str) -> bool {
        bounded_collision_query_v2(value).is_some_and(|canonical| self.symbols.contains(&canonical))
    }

    pub fn resolve_parent(
        &self,
        catalog_id: &str,
    ) -> Result<QuestParentInput, Revision3QuestCollisionCapabilityErrorV2> {
        validate_catalog_query_v2("Quest parent", catalog_id)?;
        self.parents.get(catalog_id).cloned().ok_or_else(|| {
            Revision3QuestCollisionCapabilityErrorV2::UnknownParent(catalog_id.to_owned())
        })
    }

    pub fn resolve_giver(
        &self,
        catalog_id: &str,
    ) -> Result<QuestGiverInput, Revision3QuestCollisionCapabilityErrorV2> {
        validate_catalog_query_v2("Quest giver", catalog_id)?;
        self.givers.get(catalog_id).cloned().ok_or_else(|| {
            Revision3QuestCollisionCapabilityErrorV2::UnknownGiver(catalog_id.to_owned())
        })
    }

    pub fn authorizes_parent(&self, candidate: &QuestParentInput) -> bool {
        self.parents.values().any(|parent| parent == candidate)
    }

    pub fn authorizes_giver(&self, candidate: &QuestGiverInput) -> bool {
        self.givers.values().any(|giver| giver == candidate)
    }

    /// Consume into a plain generation input. This drops the opaque source and grants no durable
    /// artifact, build, runtime, or publication authority.
    pub fn into_quest_collision_input(self) -> QuestCollisionCatalogInput {
        let Self {
            source,
            combined_source_seal,
            modules,
            relative_paths,
            symbols,
            base_inventory_payload_seal: _,
            story_catalog_seal: _,
            parents: _,
            givers: _,
        } = self;
        let generation = source.target().clone();
        drop(source);
        QuestCollisionCatalogInput {
            generation,
            source_seal: authoring_seal(&combined_source_seal),
            catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2.to_owned(),
            modules,
            relative_paths,
            symbols,
        }
    }

    /// Consume a fresh capability and return it only when every structural artifact binding and
    /// every canonical collision identity matches exactly.
    pub fn verify_artifact_exact(
        self,
        artifact: &QuestCollisionCapabilityArtifactV2,
    ) -> Result<Self, Revision3QuestCollisionCapabilityArtifactVerificationErrorV2> {
        verify_revision3_quest_collision_artifact_exact_v2(
            self.combined_payload(),
            &self.combined_source_seal,
            artifact,
        )?;
        Ok(self)
    }

    pub fn prepare_artifact(
        self,
    ) -> Result<PreparedQuestCollisionArtifactV2, QuestCollisionCapabilityArtifactErrorV2> {
        let artifact = self.materialize_structural_artifact()?;
        Ok(PreparedQuestCollisionArtifactV2 {
            capability: self,
            artifact,
        })
    }

    pub fn into_artifact(
        self,
    ) -> Result<QuestCollisionCapabilityArtifactV2, QuestCollisionCapabilityArtifactErrorV2> {
        let canonical_json = self.materialize_canonical_payload()?;
        let actual_source_seal = seal_combined_payload_bytes_v2(&canonical_json);
        if actual_source_seal != self.combined_source_seal {
            return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(
                "verified capability semantic seal changed while materializing its artifact"
                    .to_owned(),
            ));
        }
        let artifact_seal = raw_artifact_seal_v2(&canonical_json);
        Ok(self.finish_artifact(canonical_json, artifact_seal, actual_source_seal))
    }

    fn finish_artifact(
        self,
        canonical_json: Vec<u8>,
        artifact_seal: ContentSeal,
        source_seal: ContentSeal,
    ) -> QuestCollisionCapabilityArtifactV2 {
        let Self {
            source,
            base_inventory_payload_seal,
            story_catalog_seal,
            combined_source_seal: _,
            modules: _,
            relative_paths: _,
            symbols: _,
            parents: _,
            givers: _,
        } = self;
        QuestCollisionCapabilityArtifactV2 {
            canonical_json,
            artifact_seal,
            source_seal,
            base_inventory_payload_seal,
            story_catalog_seal,
            project_id: source.project_id(),
            project_revision: source.project_revision(),
            project_target: source.target().clone(),
            current_head: source.current_head().clone(),
            current_project: source.current_project().clone(),
            nonquest_project: source.nonquest_basis().canonical_project().clone(),
            prior_quest_count: source.prior_quest_count_u64(),
            prior_quest_evidence: source.prior_quest_evidence().clone(),
        }
    }

    fn materialize_structural_artifact(
        &self,
    ) -> Result<QuestCollisionCapabilityArtifactV2, QuestCollisionCapabilityArtifactErrorV2> {
        let canonical_json = self.materialize_canonical_payload()?;
        let source_seal = seal_combined_payload_bytes_v2(&canonical_json);
        if source_seal != self.combined_source_seal {
            return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(
                "verified capability semantic seal changed while materializing its artifact"
                    .to_owned(),
            ));
        }
        Ok(QuestCollisionCapabilityArtifactV2 {
            artifact_seal: raw_artifact_seal_v2(&canonical_json),
            canonical_json,
            source_seal,
            base_inventory_payload_seal: self.base_inventory_payload_seal.clone(),
            story_catalog_seal: self.story_catalog_seal.clone(),
            project_id: self.project_id(),
            project_revision: self.project_revision(),
            project_target: self.project_target().clone(),
            current_head: self.current_head().clone(),
            current_project: self.current_project().clone(),
            nonquest_project: self.nonquest_project().clone(),
            prior_quest_count: self.prior_quest_count(),
            prior_quest_evidence: self.prior_quest_evidence().clone(),
        })
    }

    fn materialize_canonical_payload(
        &self,
    ) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactErrorV2> {
        canonical_combined_payload_v2(&self.combined_payload())
    }

    fn combined_payload(&self) -> CombinedPayloadV2<'_> {
        CombinedPayloadV2 {
            format: COMBINED_FORMAT_V2,
            schema_revision: COMBINED_SCHEMA_REVISION_V2,
            coverage: Revision3QuestCollisionCoverageV2::BaseGameAndExactRevision3ProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &self.base_inventory_payload_seal,
            story_catalog_seal: &self.story_catalog_seal,
            project_id: self.project_id(),
            project_revision: self.project_revision(),
            project_target: self.project_target(),
            current_head: self.current_head(),
            current_project: self.current_project(),
            nonquest_project: self.nonquest_project(),
            prior_quest_count: self.prior_quest_count(),
            prior_quest_evidence: self.prior_quest_evidence(),
            modules: &self.modules,
            relative_paths: &self.relative_paths,
            symbols: &self.symbols,
        }
    }
}

impl VerifiedRevision3QuestCollisionInspectionCapabilityV2 {
    /// Bind trusted base/catalog inputs to one store-reconstructed historical source.
    ///
    /// Successful binding still grants no reusable authoring authority. The result has exactly
    /// one public consuming verification operation and cannot resolve catalog entries or create a
    /// replacement artifact.
    pub fn bind(
        base: BaseGameCollisionInventory,
        catalog: &StoryCatalogFile,
        source: PreparedRevision3QuestCollisionInspectionSourceV2,
    ) -> Result<Self, Revision3QuestCollisionCapabilityErrorV2> {
        let bound = bind_revision3_quest_collision_inputs_v2(base, catalog, &source)?;
        Ok(Self {
            source,
            base_inventory_payload_seal: bound.base_inventory_payload_seal,
            story_catalog_seal: bound.story_catalog_seal,
            combined_source_seal: bound.combined_source_seal,
            modules: bound.modules,
            relative_paths: bound.relative_paths,
            symbols: bound.symbols,
            parents: bound.parents,
            givers: bound.givers,
        })
    }

    /// Consume the inspection capability, require exact artifact identity and exact catalog-backed
    /// Quest context, and return only the plain collision input needed to regenerate source.
    pub fn verify_artifact_for_quest(
        self,
        artifact: &QuestCollisionCapabilityArtifactV2,
        parent: &QuestParentInput,
        giver: &QuestGiverInput,
    ) -> Result<QuestCollisionCatalogInput, Revision3QuestCollisionInspectionVerificationErrorV2>
    {
        self.verify_artifact_exact(artifact)?;
        if !self.parents.values().any(|candidate| candidate == parent) {
            return Err(Revision3QuestCollisionInspectionVerificationErrorV2::UnauthorizedParent);
        }
        if !self.givers.values().any(|candidate| candidate == giver) {
            return Err(Revision3QuestCollisionInspectionVerificationErrorV2::UnauthorizedGiver);
        }

        let Self {
            source,
            base_inventory_payload_seal: _,
            story_catalog_seal: _,
            combined_source_seal,
            modules,
            relative_paths,
            symbols,
            parents: _,
            givers: _,
        } = self;
        let generation = source.target().clone();
        drop(source);
        Ok(QuestCollisionCatalogInput {
            generation,
            source_seal: authoring_seal(&combined_source_seal),
            catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2.to_owned(),
            modules,
            relative_paths,
            symbols,
        })
    }

    fn verify_artifact_exact(
        &self,
        artifact: &QuestCollisionCapabilityArtifactV2,
    ) -> Result<(), Revision3QuestCollisionCapabilityArtifactVerificationErrorV2> {
        verify_revision3_quest_collision_artifact_exact_v2(
            self.combined_payload(),
            &self.combined_source_seal,
            artifact,
        )
    }

    fn combined_payload(&self) -> CombinedPayloadV2<'_> {
        CombinedPayloadV2 {
            format: COMBINED_FORMAT_V2,
            schema_revision: COMBINED_SCHEMA_REVISION_V2,
            coverage: Revision3QuestCollisionCoverageV2::BaseGameAndExactRevision3ProjectOnly,
            catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
            runtime_qualification: QuestCollisionRuntimeQualification::RuntimeUnqualified,
            build_status: QuestCollisionBuildStatus::Blocked,
            publication_status: QuestCollisionPublicationStatus::NotSupported,
            base_inventory_payload_seal: &self.base_inventory_payload_seal,
            story_catalog_seal: &self.story_catalog_seal,
            project_id: self.source.project_id(),
            project_revision: self.source.project_revision(),
            project_target: self.source.target(),
            current_head: self.source.historical_head(),
            current_project: self.source.historical_project(),
            nonquest_project: self.source.nonquest_basis().canonical_project(),
            prior_quest_count: self.source.prior_quest_count_u64(),
            prior_quest_evidence: self.source.prior_quest_evidence(),
            modules: &self.modules,
            relative_paths: &self.relative_paths,
            symbols: &self.symbols,
        }
    }
}

/// Structurally reopen untrusted V2 bytes under independently supplied raw and semantic seals.
pub fn reopen_quest_collision_capability_artifact_v2(
    canonical_json: &[u8],
    expected_artifact_seal: &ContentSeal,
    expected_source_seal: &ContentSeal,
) -> Result<QuestCollisionCapabilityArtifactV2, QuestCollisionCapabilityArtifactErrorV2> {
    if canonical_json.is_empty() {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(
            "canonical artifact JSON must not be empty".to_owned(),
        ));
    }
    if canonical_json.len() > MAX_INVENTORY_JSON_BYTES {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
            kind: "canonical artifact JSON bytes",
            actual: canonical_json.len(),
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    preflight_json_string_tokens_v2(canonical_json)?;
    validate_expected_seal_v2("raw artifact", expected_artifact_seal, canonical_json.len())?;
    validate_expected_seal_v2(
        "semantic source",
        expected_source_seal,
        canonical_json.len(),
    )?;
    let artifact_seal = raw_artifact_seal_v2(canonical_json);
    if &artifact_seal != expected_artifact_seal {
        return Err(QuestCollisionCapabilityArtifactErrorV2::ArtifactSealMismatch);
    }
    let source_seal = seal_combined_payload_bytes_v2(canonical_json);
    if &source_seal != expected_source_seal {
        return Err(QuestCollisionCapabilityArtifactErrorV2::SourceSealMismatch);
    }

    let wire: CombinedArtifactWireV2 = serde_json::from_slice(canonical_json)
        .map_err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson)?;
    validate_artifact_wire_v2(&wire)?;
    let canonical = canonical_artifact_wire_v2(&wire)?;
    if canonical.as_slice() != canonical_json {
        return Err(QuestCollisionCapabilityArtifactErrorV2::NonCanonicalJson);
    }

    Ok(QuestCollisionCapabilityArtifactV2 {
        canonical_json: canonical,
        artifact_seal,
        source_seal,
        base_inventory_payload_seal: wire.base_inventory_payload_seal,
        story_catalog_seal: wire.story_catalog_seal,
        project_id: wire.project_id,
        project_revision: wire.project_revision,
        project_target: wire.project_target,
        current_head: wire.current_head,
        current_project: wire.current_project,
        nonquest_project: wire.nonquest_project,
        prior_quest_count: wire.prior_quest_count,
        prior_quest_evidence: wire.prior_quest_evidence,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestCollisionCapabilityErrorV2 {
    #[error("base inventory and trusted Story catalog are not exactly bound")]
    CatalogBindingMismatch,
    #[error("exact current revision-3 project target does not match the trusted generation")]
    TargetMismatch,
    #[error("opaque exact-current source binding drifted: {reason}")]
    SourceBindingDrift { reason: String },
    #[error("prior Quest {quest} parent is absent from the fresh Story catalog")]
    PriorQuestParentDrift { quest: EntityId },
    #[error("prior Quest {quest} giver is absent from the fresh Story catalog")]
    PriorQuestGiverDrift { quest: EntityId },
    #[error("current {kind} identity {value:?} owned by {owner} collides with the base game")]
    BaseCurrentCollision {
        kind: &'static str,
        value: String,
        owner: EntityId,
    },
    #[error("current {kind} identity {value:?} collides between {first_owner} and {second_owner}")]
    CurrentIdentityCollision {
        kind: &'static str,
        value: String,
        first_owner: EntityId,
        second_owner: EntityId,
    },
    #[error("invalid {kind} collision identity {value:?}")]
    InvalidCollisionIdentity { kind: &'static str, value: String },
    #[error("combined revision-3 collision capability exceeds {kind}: {actual} > {max}")]
    Limit {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("unknown trusted Story catalog Quest parent {0:?}")]
    UnknownParent(String),
    #[error("unknown trusted Story catalog Quest giver {0:?}")]
    UnknownGiver(String),
    #[error("{kind} catalog query is invalid or exceeds {max} bytes: {actual} bytes")]
    InvalidCatalogQuery {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("could not serialize combined revision-3 Quest collision provenance: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestCollisionCapabilityArtifactVerificationErrorV2 {
    #[error("revision-3 Quest collision artifact raw content seal mismatch")]
    RawArtifactSealMismatch,
    #[error("revision-3 Quest collision artifact semantic source seal mismatch")]
    SemanticSourceSealMismatch,
    #[error("revision-3 Quest collision artifact base inventory payload seal mismatch")]
    BaseInventoryPayloadSealMismatch,
    #[error("revision-3 Quest collision artifact Story catalog seal mismatch")]
    StoryCatalogSealMismatch,
    #[error("revision-3 Quest collision artifact project id mismatch")]
    ProjectIdMismatch,
    #[error("revision-3 Quest collision artifact project revision mismatch")]
    ProjectRevisionMismatch,
    #[error("revision-3 Quest collision artifact project target mismatch")]
    ProjectTargetMismatch,
    #[error("revision-3 Quest collision artifact current head mismatch")]
    CurrentHeadMismatch,
    #[error("revision-3 Quest collision artifact exact current-project seal mismatch")]
    CurrentProjectMismatch,
    #[error("revision-3 Quest collision artifact non-Quest project seal mismatch")]
    NonQuestProjectMismatch,
    #[error("revision-3 Quest collision artifact prior-Quest count mismatch")]
    PriorQuestCountMismatch,
    #[error("revision-3 Quest collision artifact prior-Quest evidence seal mismatch")]
    PriorQuestEvidenceMismatch,
    #[error("revision-3 Quest collision artifact canonical identity differs from fresh sources")]
    CanonicalIdentityMismatch,
    #[error("source-bound revision-3 artifact identity exceeds bytes: {actual} > {max}")]
    Limit { actual: usize, max: usize },
    #[error("could not serialize source-bound revision-3 artifact identity: {0}")]
    Serialize(#[source] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestCollisionInspectionVerificationErrorV2 {
    #[error(transparent)]
    Artifact(#[from] Revision3QuestCollisionCapabilityArtifactVerificationErrorV2),
    #[error("persisted Quest parent is absent from the freshly bound Story catalog")]
    UnauthorizedParent,
    #[error("persisted Quest giver is absent from the freshly bound Story catalog")]
    UnauthorizedGiver,
}

#[derive(Debug, thiserror::Error)]
pub enum PreparedQuestCollisionArtifactFinalizeErrorV2 {
    #[error("prepared revision-3 artifact and retained fresh capability no longer match exactly")]
    ArtifactOrCapabilityDrift(
        #[source] Revision3QuestCollisionCapabilityArtifactVerificationErrorV2,
    ),
}

#[derive(Debug, thiserror::Error)]
pub enum QuestCollisionCapabilityArtifactErrorV2 {
    #[error("revision-3 Quest collision artifact exceeds {kind}: {actual} > {max}")]
    Limit {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("invalid revision-3 Quest collision artifact JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize canonical revision-3 Quest collision artifact JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Quest collision artifact JSON is not in exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Quest collision artifact {kind} seal declares {declared} bytes; actual is {actual}")]
    SealLengthMismatch {
        kind: &'static str,
        declared: u64,
        actual: u64,
    },
    #[error("revision-3 Quest collision artifact raw content seal mismatch")]
    ArtifactSealMismatch,
    #[error("revision-3 Quest collision artifact semantic source seal mismatch")]
    SourceSealMismatch,
    #[error("invalid revision-3 Quest collision artifact invariant: {0}")]
    Invariant(String),
}

#[derive(Default)]
struct CollisionBudgetV2 {
    count: usize,
    bytes: usize,
}

impl CollisionBudgetV2 {
    fn debit_canonical(
        &mut self,
        kind: &'static str,
        value: &str,
    ) -> Result<(), Revision3QuestCollisionCapabilityErrorV2> {
        validate_collision_identity_v2(kind, value, true)?;
        self.debit_validated(value)
    }

    fn debit_validated(
        &mut self,
        value: &str,
    ) -> Result<(), Revision3QuestCollisionCapabilityErrorV2> {
        let Some(next_count) = self.count.checked_add(1) else {
            return Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
                kind: "collision entry count",
                actual: usize::MAX,
                max: MAX_COLLISION_ENTRIES,
            });
        };
        if next_count > MAX_COLLISION_ENTRIES {
            return Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
                kind: "collision entry count",
                actual: next_count,
                max: MAX_COLLISION_ENTRIES,
            });
        }
        let Some(next_bytes) = self.bytes.checked_add(value.len()) else {
            return Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
                kind: "aggregate collision entry bytes",
                actual: usize::MAX,
                max: MAX_COLLISION_TOTAL_BYTES,
            });
        };
        if next_bytes > MAX_COLLISION_TOTAL_BYTES {
            return Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
                kind: "aggregate collision entry bytes",
                actual: next_bytes,
                max: MAX_COLLISION_TOTAL_BYTES,
            });
        }
        self.count = next_count;
        self.bytes = next_bytes;
        Ok(())
    }
}

fn base_domain_map(
    kind: &'static str,
    values: Vec<String>,
    budget: &mut CollisionBudgetV2,
) -> Result<BTreeMap<String, Option<EntityId>>, Revision3QuestCollisionCapabilityErrorV2> {
    let mut domain = BTreeMap::new();
    for value in values {
        budget.debit_canonical(kind, &value)?;
        if domain.contains_key(&value) {
            return Err(
                Revision3QuestCollisionCapabilityErrorV2::InvalidCollisionIdentity { kind, value },
            );
        }
        domain.insert(value, None);
    }
    Ok(domain)
}

fn merge_current_domain<'a>(
    kind: &'static str,
    domain: &mut BTreeMap<String, Option<EntityId>>,
    entries: impl IntoIterator<Item = (&'a str, EntityId)>,
    budget: &mut CollisionBudgetV2,
) -> Result<(), Revision3QuestCollisionCapabilityErrorV2> {
    for (value, owner) in entries {
        // Generated Story identities retain presentation case. Collision keys are the canonical
        // ASCII-lowercase form used by the base inventory and the durable artifact.
        let mut folded = [0u8; MAX_COLLISION_ENTRY_BYTES];
        let canonical = canonical_collision_key_v2(kind, value, &mut folded)?;
        if let Some(existing) = domain.get(canonical).copied() {
            return match existing {
                Some(first_owner) => Err(
                    Revision3QuestCollisionCapabilityErrorV2::CurrentIdentityCollision {
                        kind,
                        value: canonical.to_owned(),
                        first_owner,
                        second_owner: owner,
                    },
                ),
                None => Err(
                    Revision3QuestCollisionCapabilityErrorV2::BaseCurrentCollision {
                        kind,
                        value: canonical.to_owned(),
                        owner,
                    },
                ),
            };
        }
        budget.debit_canonical(kind, canonical)?;
        domain.insert(canonical.to_owned(), Some(owner));
    }
    Ok(())
}

fn validate_collision_identity_v2(
    kind: &'static str,
    value: &str,
    require_lowercase: bool,
) -> Result<(), Revision3QuestCollisionCapabilityErrorV2> {
    if value.is_empty()
        || value.len() > MAX_COLLISION_ENTRY_BYTES
        || !value.is_ascii()
        || (require_lowercase && value.bytes().any(|byte| byte.is_ascii_uppercase()))
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(
            Revision3QuestCollisionCapabilityErrorV2::InvalidCollisionIdentity {
                kind,
                value: value.to_owned(),
            },
        );
    }
    Ok(())
}

fn canonical_collision_key_v2<'a>(
    kind: &'static str,
    value: &str,
    folded: &'a mut [u8; MAX_COLLISION_ENTRY_BYTES],
) -> Result<&'a str, Revision3QuestCollisionCapabilityErrorV2> {
    validate_collision_identity_v2(kind, value, false)?;
    for (destination, source) in folded.iter_mut().zip(value.bytes()) {
        *destination = source.to_ascii_lowercase();
    }
    Ok(std::str::from_utf8(&folded[..value.len()])
        .expect("validated ASCII collision identity remains UTF-8 after folding"))
}

fn bounded_collision_query_v2(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > MAX_COLLISION_ENTRY_BYTES
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return None;
    }
    Some(value.to_ascii_lowercase())
}

fn validate_catalog_query_v2(
    kind: &'static str,
    value: &str,
) -> Result<(), Revision3QuestCollisionCapabilityErrorV2> {
    if value.is_empty()
        || value.len() > MAX_COLLISION_ENTRY_BYTES
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(
            Revision3QuestCollisionCapabilityErrorV2::InvalidCatalogQuery {
                kind,
                actual: value.len(),
                max: MAX_COLLISION_ENTRY_BYTES,
            },
        );
    }
    Ok(())
}

fn validate_source_bindings<S>(source: &S) -> Result<(), Revision3QuestCollisionCapabilityErrorV2>
where
    S: Revision3QuestCollisionSourceViewV2,
{
    let identities = source.nonquest_basis().story_identities();
    if identities.project_id() != source.project_id()
        || identities.project_revision() != source.project_revision()
        || identities.target() != source.target()
        || identities.canonical_project() != source.nonquest_basis().canonical_project()
    {
        return Err(
            Revision3QuestCollisionCapabilityErrorV2::SourceBindingDrift {
                reason: "native non-Quest identities differ from their retained basis".to_owned(),
            },
        );
    }
    validate_authoring_seal_for_binding(
        "current head snapshot",
        &source.current_head().snapshot,
        MAX_REVISION3_SNAPSHOT_BYTES,
    )?;
    validate_authoring_seal_for_binding(
        "current project",
        source.current_project(),
        MAX_PROJECT_JSON_BYTES as u64,
    )?;
    validate_authoring_seal_for_binding(
        "non-Quest project",
        source.nonquest_basis().canonical_project(),
        MAX_PROJECT_JSON_BYTES as u64,
    )?;
    validate_authoring_seal_for_binding(
        "prior-Quest evidence",
        source.prior_quest_evidence(),
        MAX_PROJECT_JSON_BYTES as u64,
    )?;
    if source.prior_quest_count() > MAX_REVISION3_PRIOR_QUESTS_V2 {
        return Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
            kind: "prior Quest count",
            actual: source.prior_quest_count(),
            max: MAX_REVISION3_PRIOR_QUESTS_V2,
        });
    }
    Ok(())
}

fn validate_authoring_seal_for_binding(
    kind: &'static str,
    seal: &AuthoringContentSeal,
    max: u64,
) -> Result<(), Revision3QuestCollisionCapabilityErrorV2> {
    if seal.byte_len == 0 || seal.byte_len > max {
        return Err(
            Revision3QuestCollisionCapabilityErrorV2::SourceBindingDrift {
                reason: format!("{kind} seal byte length is outside 1..={max}"),
            },
        );
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CombinedPayloadV2<'a> {
    format: &'static str,
    schema_revision: u32,
    coverage: Revision3QuestCollisionCoverageV2,
    catalog_layer: &'static str,
    runtime_qualification: QuestCollisionRuntimeQualification,
    build_status: QuestCollisionBuildStatus,
    publication_status: QuestCollisionPublicationStatus,
    base_inventory_payload_seal: &'a ContentSeal,
    story_catalog_seal: &'a ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: &'a GameGenerationAnchor,
    current_head: &'a WorkingHead,
    current_project: &'a AuthoringContentSeal,
    nonquest_project: &'a AuthoringContentSeal,
    prior_quest_count: u64,
    prior_quest_evidence: &'a AuthoringContentSeal,
    modules: &'a BTreeSet<String>,
    relative_paths: &'a BTreeSet<String>,
    symbols: &'a BTreeSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct BoundedArtifactStringV2<const MAX: usize>(String);

impl<const MAX: usize> Serialize for BoundedArtifactStringV2<MAX> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de, const MAX: usize> Deserialize<'de> for BoundedArtifactStringV2<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorV2<const MAX: usize>;
        impl<const MAX: usize> de::Visitor<'_> for VisitorV2<MAX> {
            type Value = BoundedArtifactStringV2<MAX>;
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
                Ok(BoundedArtifactStringV2(value.to_owned()))
            }
            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedArtifactStringV2(value))
            }
        }
        deserializer.deserialize_string(VisitorV2::<MAX>)
    }
}

#[derive(Debug, Serialize)]
struct CombinedArtifactWireV2 {
    format: BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>,
    schema_revision: u32,
    coverage: BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>,
    catalog_layer: BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>,
    runtime_qualification: BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>,
    build_status: BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>,
    publication_status: BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>,
    base_inventory_payload_seal: ContentSeal,
    story_catalog_seal: ContentSeal,
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    current_head: WorkingHead,
    current_project: AuthoringContentSeal,
    nonquest_project: AuthoringContentSeal,
    prior_quest_count: u64,
    prior_quest_evidence: AuthoringContentSeal,
    modules: Vec<String>,
    relative_paths: Vec<String>,
    symbols: Vec<String>,
}

impl<'de> Deserialize<'de> for CombinedArtifactWireV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArtifactVisitorV2;
        impl<'de> de::Visitor<'de> for ArtifactVisitorV2 {
            type Value = CombinedArtifactWireV2;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed bounded revision-3 Quest collision artifact")
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
                let mut current_head = None;
                let mut current_project = None;
                let mut nonquest_project = None;
                let mut prior_quest_count = None;
                let mut prior_quest_evidence = None;
                let mut modules = None;
                let mut relative_paths = None;
                let mut symbols = None;
                let mut remaining_count = MAX_COLLISION_ENTRIES;
                let mut remaining_bytes = MAX_COLLISION_TOTAL_BYTES;

                while let Some(BoundedArtifactStringV2(field)) =
                    access.next_key::<BoundedArtifactStringV2<MAX_COMBINED_MARKER_BYTES_V2>>()?
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
                        "publication_status" => scalar!(publication_status, "publication_status"),
                        "base_inventory_payload_seal" => {
                            scalar!(base_inventory_payload_seal, "base_inventory_payload_seal")
                        }
                        "story_catalog_seal" => scalar!(story_catalog_seal, "story_catalog_seal"),
                        "project_id" => scalar!(project_id, "project_id"),
                        "project_revision" => scalar!(project_revision, "project_revision"),
                        "project_target" => scalar!(project_target, "project_target"),
                        "current_head" => scalar!(current_head, "current_head"),
                        "current_project" => scalar!(current_project, "current_project"),
                        "nonquest_project" => scalar!(nonquest_project, "nonquest_project"),
                        "prior_quest_count" => scalar!(prior_quest_count, "prior_quest_count"),
                        "prior_quest_evidence" => {
                            scalar!(prior_quest_evidence, "prior_quest_evidence")
                        }
                        "modules" | "relative_paths" | "symbols" => {
                            let (slot, kind): (&mut Option<Vec<String>>, &'static str) =
                                match field.as_str() {
                                    "modules" => (&mut modules, "module"),
                                    "relative_paths" => (&mut relative_paths, "relative path"),
                                    _ => (&mut symbols, "symbol"),
                                };
                            if slot.is_some() {
                                return Err(de::Error::duplicate_field(match kind {
                                    "module" => "modules",
                                    "relative path" => "relative_paths",
                                    _ => "symbols",
                                }));
                            }
                            let parsed = access.next_value_seed(CollisionEntriesSeedV2 {
                                kind,
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= parsed.count;
                            remaining_bytes -= parsed.bytes;
                            *slot = Some(parsed.values);
                        }
                        _ => return Err(de::Error::unknown_field(&field, V2_WIRE_FIELDS)),
                    }
                }

                macro_rules! required {
                    ($slot:ident, $name:literal) => {
                        $slot.ok_or_else(|| de::Error::missing_field($name))?
                    };
                }
                Ok(CombinedArtifactWireV2 {
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
                    current_head: required!(current_head, "current_head"),
                    current_project: required!(current_project, "current_project"),
                    nonquest_project: required!(nonquest_project, "nonquest_project"),
                    prior_quest_count: required!(prior_quest_count, "prior_quest_count"),
                    prior_quest_evidence: required!(prior_quest_evidence, "prior_quest_evidence"),
                    modules: required!(modules, "modules"),
                    relative_paths: required!(relative_paths, "relative_paths"),
                    symbols: required!(symbols, "symbols"),
                })
            }
        }
        deserializer.deserialize_map(ArtifactVisitorV2)
    }
}

const V2_WIRE_FIELDS: &[&str] = &[
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
    "current_head",
    "current_project",
    "nonquest_project",
    "prior_quest_count",
    "prior_quest_evidence",
    "modules",
    "relative_paths",
    "symbols",
];

struct CollisionEntriesSeedV2 {
    kind: &'static str,
    remaining_count: usize,
    remaining_bytes: usize,
}

struct ParsedCollisionEntriesV2 {
    values: Vec<String>,
    count: usize,
    bytes: usize,
}

impl<'de> de::DeserializeSeed<'de> for CollisionEntriesSeedV2 {
    type Value = ParsedCollisionEntriesV2;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntriesVisitorV2 {
            kind: &'static str,
            remaining_count: usize,
            remaining_bytes: usize,
        }
        impl<'de> de::Visitor<'de> for EntriesVisitorV2 {
            type Value = ParsedCollisionEntriesV2;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "a strict collision array within the remaining global budget"
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
                while let Some(BoundedArtifactStringV2(value)) =
                    sequence.next_element::<BoundedArtifactStringV2<MAX_COLLISION_ENTRY_BYTES>>()?
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
                Ok(ParsedCollisionEntriesV2 {
                    count: values.len(),
                    values,
                    bytes,
                })
            }
        }
        deserializer.deserialize_seq(EntriesVisitorV2 {
            kind: self.kind,
            remaining_count: self.remaining_count,
            remaining_bytes: self.remaining_bytes,
        })
    }
}

struct MaterializedArtifactIdentityV2 {
    artifact_seal: ContentSeal,
    source_seal: ContentSeal,
    canonical_matches: bool,
}

fn verify_revision3_quest_collision_artifact_exact_v2(
    payload: CombinedPayloadV2<'_>,
    combined_source_seal: &ContentSeal,
    artifact: &QuestCollisionCapabilityArtifactV2,
) -> Result<(), Revision3QuestCollisionCapabilityArtifactVerificationErrorV2> {
    let expected = materialize_artifact_identity_v2(&payload, &artifact.canonical_json)?;
    if artifact.base_inventory_payload_seal != *payload.base_inventory_payload_seal {
        return Err(Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::BaseInventoryPayloadSealMismatch);
    }
    if artifact.story_catalog_seal != *payload.story_catalog_seal {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::StoryCatalogSealMismatch,
        );
    }
    if artifact.project_id != payload.project_id {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::ProjectIdMismatch,
        );
    }
    if artifact.project_revision != payload.project_revision {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::ProjectRevisionMismatch,
        );
    }
    if artifact.project_target != *payload.project_target {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::ProjectTargetMismatch,
        );
    }
    if artifact.current_head != *payload.current_head {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::CurrentHeadMismatch,
        );
    }
    if artifact.current_project != *payload.current_project {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::CurrentProjectMismatch,
        );
    }
    if artifact.nonquest_project != *payload.nonquest_project {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::NonQuestProjectMismatch,
        );
    }
    if artifact.prior_quest_count != payload.prior_quest_count {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::PriorQuestCountMismatch,
        );
    }
    if artifact.prior_quest_evidence != *payload.prior_quest_evidence {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::PriorQuestEvidenceMismatch,
        );
    }
    if expected.source_seal != *combined_source_seal || artifact.source_seal != expected.source_seal
    {
        return Err(Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::SemanticSourceSealMismatch);
    }
    if !expected.canonical_matches {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::CanonicalIdentityMismatch,
        );
    }
    if artifact.artifact_seal != expected.artifact_seal {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::RawArtifactSealMismatch,
        );
    }
    Ok(())
}

fn materialize_artifact_identity_v2(
    payload: &CombinedPayloadV2<'_>,
    expected_canonical: &[u8],
) -> Result<
    MaterializedArtifactIdentityV2,
    Revision3QuestCollisionCapabilityArtifactVerificationErrorV2,
> {
    let mut writer =
        ExactCanonicalIdentityWriterV2::new(expected_canonical, MAX_INVENTORY_JSON_BYTES);
    let serialized = serde_json::to_writer(&mut writer, payload);
    if let Some(actual) = writer.first_exceeded_size {
        return Err(
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::Limit {
                actual,
                max: MAX_INVENTORY_JSON_BYTES,
            },
        );
    }
    serialized.map_err(Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::Serialize)?;
    let payload_len = writer.bytes_written;
    let artifact_seal = ContentSeal {
        byte_len: payload_len as u64,
        sha256: super::Sha256Digest::from_bytes(writer.raw_hasher.finalize().into()),
    };
    let mut semantic_hasher = Sha256::new();
    semantic_hasher.update(COMBINED_SEAL_DOMAIN_V2);
    semantic_hasher.update((payload_len as u64).to_be_bytes());
    serde_json::to_writer(HashWriterV2(&mut semantic_hasher), payload)
        .map_err(Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::Serialize)?;
    Ok(MaterializedArtifactIdentityV2 {
        artifact_seal,
        source_seal: ContentSeal {
            byte_len: payload_len as u64,
            sha256: super::Sha256Digest::from_bytes(semantic_hasher.finalize().into()),
        },
        canonical_matches: writer.canonical_matches && payload_len == expected_canonical.len(),
    })
}

struct ExactCanonicalIdentityWriterV2<'a> {
    expected: &'a [u8],
    bytes_written: usize,
    limit: usize,
    canonical_matches: bool,
    first_exceeded_size: Option<usize>,
    raw_hasher: Sha256,
}

impl<'a> ExactCanonicalIdentityWriterV2<'a> {
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

impl Write for ExactCanonicalIdentityWriterV2<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let start = self.bytes_written;
        let actual = start.saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "source-bound canonical payload limit exceeded",
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

fn canonical_combined_payload_v2(
    payload: &CombinedPayloadV2<'_>,
) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactErrorV2> {
    let mut writer = BoundedBytesWriterV2::new(MAX_INVENTORY_JSON_BYTES);
    let result = serde_json::to_writer(&mut writer, payload);
    finish_bounded_artifact_json_v2(writer, result)
}

fn canonical_artifact_wire_v2(
    wire: &CombinedArtifactWireV2,
) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactErrorV2> {
    let mut writer = BoundedBytesWriterV2::new(MAX_INVENTORY_JSON_BYTES);
    let result = serde_json::to_writer(&mut writer, wire);
    finish_bounded_artifact_json_v2(writer, result)
}

fn finish_bounded_artifact_json_v2(
    writer: BoundedBytesWriterV2,
    result: Result<(), serde_json::Error>,
) -> Result<Vec<u8>, QuestCollisionCapabilityArtifactErrorV2> {
    if let Some(actual) = writer.first_exceeded_size {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
            kind: "canonical artifact JSON bytes",
            actual,
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    result.map_err(QuestCollisionCapabilityArtifactErrorV2::Serialize)?;
    Ok(writer.bytes)
}

fn validate_artifact_wire_v2(
    wire: &CombinedArtifactWireV2,
) -> Result<(), QuestCollisionCapabilityArtifactErrorV2> {
    let fixed = [
        (wire.format.0.as_str(), COMBINED_FORMAT_V2, "format"),
        (
            wire.coverage.0.as_str(),
            "base_game_and_exact_revision3_project_only",
            "coverage",
        ),
        (
            wire.catalog_layer.0.as_str(),
            BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
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
            return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
                "unsupported {kind} {actual:?}; expected {expected:?}"
            )));
        }
    }
    if wire.schema_revision != COMBINED_SCHEMA_REVISION_V2 {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
            "unsupported schema revision {}; expected {COMBINED_SCHEMA_REVISION_V2}",
            wire.schema_revision
        )));
    }
    validate_catalog_seal_v2(
        "base inventory payload",
        &wire.base_inventory_payload_seal,
        MAX_INVENTORY_JSON_BYTES as u64,
    )?;
    validate_catalog_seal_v2(
        "Story catalog",
        &wire.story_catalog_seal,
        MAX_CATALOG_JSON_BYTES as u64,
    )?;
    if wire.project_id.as_bytes().iter().all(|byte| *byte == 0) {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(
            "project id must not be all zeroes".to_owned(),
        ));
    }
    validate_authoring_seal_v2(
        "project target executable",
        &wire.project_target.executable,
        u64::MAX,
    )?;
    validate_authoring_seal_v2(
        "current head snapshot",
        &wire.current_head.snapshot,
        MAX_REVISION3_SNAPSHOT_BYTES,
    )?;
    validate_authoring_seal_v2(
        "current project",
        &wire.current_project,
        MAX_PROJECT_JSON_BYTES as u64,
    )?;
    validate_authoring_seal_v2(
        "non-Quest project",
        &wire.nonquest_project,
        MAX_PROJECT_JSON_BYTES as u64,
    )?;
    validate_authoring_seal_v2(
        "prior-Quest evidence",
        &wire.prior_quest_evidence,
        MAX_PROJECT_JSON_BYTES as u64,
    )?;
    if wire.prior_quest_count > MAX_REVISION3_PRIOR_QUESTS_V2 as u64 {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
            kind: "prior Quest count",
            actual: usize::try_from(wire.prior_quest_count).unwrap_or(usize::MAX),
            max: MAX_REVISION3_PRIOR_QUESTS_V2,
        });
    }

    let count = wire
        .modules
        .len()
        .checked_add(wire.relative_paths.len())
        .and_then(|count| count.checked_add(wire.symbols.len()))
        .unwrap_or(usize::MAX);
    if count > MAX_COLLISION_ENTRIES {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
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
            return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
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
                return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
                    "invalid {kind} collision entry"
                )));
            }
            let Some(next_bytes) = bytes.checked_add(entry.len()) else {
                return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
                    kind: "aggregate collision entry bytes",
                    actual: usize::MAX,
                    max: MAX_COLLISION_TOTAL_BYTES,
                });
            };
            bytes = next_bytes;
            if bytes > MAX_COLLISION_TOTAL_BYTES {
                return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
                    kind: "aggregate collision entry bytes",
                    actual: bytes,
                    max: MAX_COLLISION_TOTAL_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn validate_expected_seal_v2(
    kind: &'static str,
    seal: &ContentSeal,
    actual_len: usize,
) -> Result<(), QuestCollisionCapabilityArtifactErrorV2> {
    if seal.byte_len == 0 {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
            "{kind} seal has zero byte length"
        )));
    }
    if seal.byte_len != actual_len as u64 {
        return Err(
            QuestCollisionCapabilityArtifactErrorV2::SealLengthMismatch {
                kind,
                declared: seal.byte_len,
                actual: actual_len as u64,
            },
        );
    }
    Ok(())
}

fn validate_catalog_seal_v2(
    kind: &'static str,
    seal: &ContentSeal,
    max: u64,
) -> Result<(), QuestCollisionCapabilityArtifactErrorV2> {
    if seal.byte_len == 0 || seal.byte_len > max {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
            "{kind} seal byte length is outside 1..={max}"
        )));
    }
    Ok(())
}

fn validate_authoring_seal_v2(
    kind: &'static str,
    seal: &AuthoringContentSeal,
    max: u64,
) -> Result<(), QuestCollisionCapabilityArtifactErrorV2> {
    if seal.byte_len == 0 || seal.byte_len > max {
        return Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(format!(
            "{kind} seal byte length is outside 1..={max}"
        )));
    }
    Ok(())
}

fn raw_artifact_seal_v2(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: super::Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn seal_combined_payload_bytes_v2(bytes: &[u8]) -> ContentSeal {
    let mut hasher = Sha256::new();
    hasher.update(COMBINED_SEAL_DOMAIN_V2);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: super::Sha256Digest::from_bytes(hasher.finalize().into()),
    }
}

fn seal_combined_payload_v2(
    payload: &CombinedPayloadV2<'_>,
) -> Result<ContentSeal, Revision3QuestCollisionCapabilityErrorV2> {
    let mut counter = BoundedCountingWriterV2::new(MAX_INVENTORY_JSON_BYTES);
    let counted = serde_json::to_writer(&mut counter, payload);
    if let Some(actual) = counter.first_exceeded_size {
        return Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
            kind: "combined canonical payload bytes",
            actual,
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    counted.map_err(Revision3QuestCollisionCapabilityErrorV2::Serialize)?;
    let payload_len = counter.bytes_written;
    let mut hasher = Sha256::new();
    hasher.update(COMBINED_SEAL_DOMAIN_V2);
    hasher.update((payload_len as u64).to_be_bytes());
    serde_json::to_writer(HashWriterV2(&mut hasher), payload)
        .map_err(Revision3QuestCollisionCapabilityErrorV2::Serialize)?;
    Ok(ContentSeal {
        byte_len: payload_len as u64,
        sha256: super::Sha256Digest::from_bytes(hasher.finalize().into()),
    })
}

fn preflight_json_string_tokens_v2(
    bytes: &[u8],
) -> Result<(), QuestCollisionCapabilityArtifactErrorV2> {
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
        if raw_len > MAX_JSON_STRING_TOKEN_BYTES_V2 {
            return Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
                kind: "raw JSON string token bytes",
                actual: raw_len,
                max: MAX_JSON_STRING_TOKEN_BYTES_V2,
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

struct BoundedBytesWriterV2 {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedBytesWriterV2 {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedBytesWriterV2 {
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

struct BoundedCountingWriterV2 {
    bytes_written: usize,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedCountingWriterV2 {
    const fn new(limit: usize) -> Self {
        Self {
            bytes_written: 0,
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedCountingWriterV2 {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes_written.saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "combined canonical payload limit exceeded",
            ));
        }
        self.bytes_written = actual;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct HashWriterV2<'a>(&'a mut Sha256);

impl Write for HashWriterV2<'_> {
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
    use gore_authoring::{WorkingStoreFormat, MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2};

    trait AmbiguousIfClone<Marker> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}

    fn seal(byte: u8, len: u64) -> AuthoringContentSeal {
        AuthoringContentSeal {
            byte_len: len,
            sha256: AuthoringSha256Digest::from_bytes([byte; 32]),
        }
    }

    fn test_wire() -> CombinedArtifactWireV2 {
        CombinedArtifactWireV2 {
            format: BoundedArtifactStringV2(COMBINED_FORMAT_V2.to_owned()),
            schema_revision: COMBINED_SCHEMA_REVISION_V2,
            coverage: BoundedArtifactStringV2(
                "base_game_and_exact_revision3_project_only".to_owned(),
            ),
            catalog_layer: BoundedArtifactStringV2(
                BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2.to_owned(),
            ),
            runtime_qualification: BoundedArtifactStringV2("runtime_unqualified".to_owned()),
            build_status: BoundedArtifactStringV2("blocked".to_owned()),
            publication_status: BoundedArtifactStringV2("not_supported".to_owned()),
            base_inventory_payload_seal: ContentSeal {
                byte_len: 11,
                sha256: crate::Sha256Digest::from_bytes([1; 32]),
            },
            story_catalog_seal: ContentSeal {
                byte_len: 12,
                sha256: crate::Sha256Digest::from_bytes([2; 32]),
            },
            project_id: ProjectId::from_bytes([3; 16]),
            project_revision: 7,
            project_target: GameGenerationAnchor {
                executable: seal(4, 13),
            },
            current_head: WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: seal(5, 14),
            },
            current_project: seal(6, 15),
            nonquest_project: seal(7, 16),
            prior_quest_count: 2,
            prior_quest_evidence: seal(8, 17),
            modules: vec!["base.module".to_owned(), "project.module".to_owned()],
            relative_paths: vec!["base/module.as".to_owned(), "project/module.as".to_owned()],
            symbols: vec!["ubase".to_owned(), "uproject".to_owned()],
        }
    }

    fn canonical_wire(wire: &CombinedArtifactWireV2) -> Vec<u8> {
        canonical_artifact_wire_v2(wire).unwrap()
    }

    fn reopen_bytes(
        bytes: &[u8],
    ) -> Result<QuestCollisionCapabilityArtifactV2, QuestCollisionCapabilityArtifactErrorV2> {
        let raw = raw_artifact_seal_v2(bytes);
        let semantic = seal_combined_payload_bytes_v2(bytes);
        reopen_quest_collision_capability_artifact_v2(bytes, &raw, &semantic)
    }

    #[test]
    fn structural_v2_roundtrip_is_exact_and_fixed() {
        let bytes = canonical_wire(&test_wire());
        let artifact = reopen_bytes(&bytes).unwrap();
        assert_eq!(artifact.canonical_json(), bytes);
        assert_eq!(artifact.project_revision(), 7);
        assert_eq!(artifact.prior_quest_count(), 2);
        assert_eq!(
            artifact.catalog_layer(),
            BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2
        );
        assert_eq!(artifact.build_status(), QuestCollisionBuildStatus::Blocked);
        assert_eq!(
            artifact.runtime_qualification(),
            QuestCollisionRuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(
            artifact.publication_status(),
            QuestCollisionPublicationStatus::NotSupported
        );
    }

    #[test]
    fn structural_parser_rejects_duplicate_unknown_noncanonical_and_wrong_version() {
        let canonical = canonical_wire(&test_wire());
        let text = String::from_utf8(canonical.clone()).unwrap();
        let duplicate = text.replacen(
            "{\"format\":",
            "{\"format\":\"quest_collision_capability\",\"format\":",
            1,
        );
        assert!(matches!(
            reopen_bytes(duplicate.as_bytes()),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
        ));
        let unknown = text.replacen("{\"format\":", "{\"unknown\":1,\"format\":", 1);
        assert!(matches!(
            reopen_bytes(unknown.as_bytes()),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
        ));
        let noncanonical = format!(" {text}");
        assert!(matches!(
            reopen_bytes(noncanonical.as_bytes()),
            Err(QuestCollisionCapabilityArtifactErrorV2::NonCanonicalJson)
        ));
        let wrong_version = text.replacen("\"schema_revision\":2", "\"schema_revision\":1", 1);
        assert!(matches!(
            reopen_bytes(wrong_version.as_bytes()),
            Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(_))
        ));
    }

    #[test]
    fn structural_parser_rejects_identity_order_case_and_value_limit() {
        let mut unsorted = test_wire();
        unsorted.modules.reverse();
        assert!(matches!(
            reopen_bytes(&canonical_wire(&unsorted)),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
                | Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(_))
        ));
        let mut uppercase = test_wire();
        uppercase.symbols = vec!["Upper".to_owned()];
        assert!(matches!(
            reopen_bytes(&canonical_wire(&uppercase)),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
                | Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(_))
        ));
        let mut oversized = test_wire();
        oversized.symbols = vec!["a".repeat(MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2 + 1)];
        assert!(matches!(
            reopen_bytes(&canonical_wire(&oversized)),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
        ));
    }

    #[test]
    fn structural_parser_checks_seals_before_trusting_json() {
        let bytes = canonical_wire(&test_wire());
        let mut raw = raw_artifact_seal_v2(&bytes);
        raw.sha256 = crate::Sha256Digest::from_bytes([0xaa; 32]);
        let semantic = seal_combined_payload_bytes_v2(&bytes);
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v2(&bytes, &raw, &semantic),
            Err(QuestCollisionCapabilityArtifactErrorV2::ArtifactSealMismatch)
        ));
        let raw = raw_artifact_seal_v2(&bytes);
        let mut semantic = semantic;
        semantic.sha256 = crate::Sha256Digest::from_bytes([0xbb; 32]);
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v2(&bytes, &raw, &semantic),
            Err(QuestCollisionCapabilityArtifactErrorV2::SourceSealMismatch)
        ));
    }

    #[test]
    fn structural_parser_enforces_shared_count_token_seal_and_prior_limits() {
        let mut too_many = test_wire();
        too_many.modules = (0..=MAX_COLLISION_ENTRIES)
            .map(|index| format!("m{index:06}"))
            .collect();
        assert!(matches!(
            reopen_bytes(&canonical_wire(&too_many)),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
                | Err(QuestCollisionCapabilityArtifactErrorV2::Limit { .. })
        ));
        drop(too_many);

        let mut too_many_bytes = test_wire();
        too_many_bytes.modules = (0..=(MAX_COLLISION_TOTAL_BYTES / MAX_COLLISION_ENTRY_BYTES))
            .map(|index| format!("{index:05}{}", "a".repeat(MAX_COLLISION_ENTRY_BYTES - 5)))
            .collect();
        assert!(matches!(
            reopen_bytes(&canonical_wire(&too_many_bytes)),
            Err(QuestCollisionCapabilityArtifactErrorV2::InvalidJson(_))
                | Err(QuestCollisionCapabilityArtifactErrorV2::Limit { .. })
        ));
        drop(too_many_bytes);

        let mut prior_overflow = test_wire();
        prior_overflow.prior_quest_count = MAX_REVISION3_PRIOR_QUESTS_V2 as u64 + 1;
        assert!(matches!(
            reopen_bytes(&canonical_wire(&prior_overflow)),
            Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
                kind: "prior Quest count",
                ..
            })
        ));

        let mut zero_snapshot = test_wire();
        zero_snapshot.current_head.snapshot.byte_len = 0;
        assert!(matches!(
            reopen_bytes(&canonical_wire(&zero_snapshot)),
            Err(QuestCollisionCapabilityArtifactErrorV2::Invariant(_))
        ));

        let long_token = format!(
            "{{\"{}\":0}}",
            "x".repeat(MAX_JSON_STRING_TOKEN_BYTES_V2 + 1)
        );
        assert!(matches!(
            reopen_bytes(long_token.as_bytes()),
            Err(QuestCollisionCapabilityArtifactErrorV2::Limit {
                kind: "raw JSON string token bytes",
                ..
            })
        ));

        let bytes = canonical_wire(&test_wire());
        let mut wrong_length = raw_artifact_seal_v2(&bytes);
        wrong_length.byte_len += 1;
        let semantic = seal_combined_payload_bytes_v2(&bytes);
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v2(&bytes, &wrong_length, &semantic),
            Err(
                QuestCollisionCapabilityArtifactErrorV2::SealLengthMismatch {
                    kind: "raw artifact",
                    ..
                }
            )
        ));
    }

    #[test]
    fn capability_and_prepared_capsule_are_not_cloneable() {
        let _ =
            <VerifiedRevision3QuestCollisionCapabilityV2 as AmbiguousIfClone<_>>::marker as fn();
        let _ = <PreparedQuestCollisionArtifactV2 as AmbiguousIfClone<_>>::marker as fn();
        let _artifact_borrow: for<'a> fn(
            &'a PreparedQuestCollisionArtifactV2,
        ) -> &'a QuestCollisionCapabilityArtifactV2 = PreparedQuestCollisionArtifactV2::artifact;
    }
}
