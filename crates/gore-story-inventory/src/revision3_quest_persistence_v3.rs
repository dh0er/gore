//! Fail-closed persistence preparation for one revision-3 Quest transaction outcome.
//!
//! This boundary consumes C1's linear result, reopens every structural transport against the
//! exact published basis, installs only the immutable V2 artifact, and prepares (but never
//! publishes) the candidate checkpoint. Structural reopening grants no source, build, runtime,
//! artifact, publication, or head-CAS authority.

use gore_authoring::{
    regenerate_revision3_quest_module_v2, AssetMeta, AssetVerification,
    ContentSeal as AuthoringContentSeal, EntityId, ImportedQuestCollisionArtifactV2,
    ProjectRevision3, ProjectRevision3JsonError, Revision3CheckpointPreparation,
    Revision3EntityKind as EntityKind, Revision3EntityPayload as EntityPayload,
    Revision3OriginRef as OriginRef, Revision3QuestCollisionSourceErrorV2,
    Revision3QuestGenerationError, Revision3TypedRef as TypedRef,
    Sha256Digest as AuthoringSha256Digest, WorkingHead, WorkingProjectStore, WorkingStoreError,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
};
use sha2::{Digest as _, Sha256};

use super::revision3_quest_transaction_v3::is_valid_revision3_quest_draft_display_name_v3;
use super::{
    reopen_quest_collision_capability_artifact_v2, QuestCollisionCapabilityArtifactErrorV2,
    QuestCollisionCapabilityArtifactV2, Revision3QuestArtifactAuthorityV3,
    Revision3QuestDraftBuildStatusV3, Revision3QuestDraftInsertOutcomeV3,
    Revision3QuestDraftPublicationStatusV3, Revision3QuestDraftRuntimeStatusV3,
    Revision3QuestSourceInspectionStatusV3,
    BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
};

/// Structurally verified immutable-object/checkpoint preparation. This value is not authority and
/// intentionally has no `Clone` implementation.
#[derive(Debug, PartialEq, Eq)]
pub struct Revision3QuestDraftPersistencePreparationV3 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub collision_artifact: QuestCollisionCapabilityArtifactV2,
    pub basis_head: WorkingHead,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub imported_artifact: ImportedQuestCollisionArtifactV2,
    pub checkpoint: Revision3CheckpointPreparation,
    pub build_status: Revision3QuestDraftBuildStatusV3,
    pub runtime_status: Revision3QuestDraftRuntimeStatusV3,
    pub artifact_authority: Revision3QuestArtifactAuthorityV3,
    pub source_inspection: Revision3QuestSourceInspectionStatusV3,
    pub publication_status: Revision3QuestDraftPublicationStatusV3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestDraftPersistenceValidationErrorV3 {
    #[error("transaction outcome contains a non-structural readiness or authority claim")]
    StatusDrift,
    #[error("canonical candidate transport does not reopen to the supplied candidate project")]
    CandidateTransportMismatch,
    #[error("structural collision artifact does not reopen exactly")]
    ArtifactReopenMismatch,
    #[error("published store head differs from the transaction basis head")]
    BasisHeadMismatch,
    #[error("collision artifact differs from the exact published basis source")]
    ArtifactBasisMismatch,
    #[error("candidate project revision is not exactly basis revision plus one")]
    CandidateRevisionMismatch,
    #[error("candidate project changed basis-level project metadata")]
    CandidateProjectMetadataMismatch,
    #[error("candidate project entity delta is not exactly one Quest and its generated module")]
    CandidateEntityDeltaMismatch,
    #[error("candidate project asset delta is not exactly the structural V2 artifact")]
    CandidateAssetDeltaMismatch,
    #[error("candidate Quest entity or ArtifactRef is structurally inconsistent")]
    QuestEntityMismatch,
    #[error("candidate generated ScriptModule entity differs from exact regeneration")]
    ScriptModuleMismatch,
    #[error("candidate authored runtime identity collides with the exact basis project")]
    RuntimeIdentityCollision,
    #[error("store artifact receipt differs from the exact validated artifact/basis")]
    ArtifactReceiptMismatch,
    #[error("prepared checkpoint does not fully reopen to the exact candidate")]
    CheckpointReopenMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftPersistenceErrorV3 {
    #[error("invalid exact canonical candidate project: {0}")]
    CandidateProject(#[source] ProjectRevision3JsonError),
    #[error("invalid structural V2 collision artifact: {0}")]
    Artifact(#[source] QuestCollisionCapabilityArtifactErrorV2),
    #[error("invalid exact published Quest collision basis: {0}")]
    BasisSource(#[source] Revision3QuestCollisionSourceErrorV2),
    #[error("candidate Quest regeneration failed: {0}")]
    Generation(#[source] Revision3QuestGenerationError),
    #[error("Quest persistence preparation failed in the working store: {0}")]
    Store(#[source] WorkingStoreError),
    #[error(transparent)]
    Validation(#[from] Revision3QuestDraftPersistenceValidationErrorV3),
}

/// Consume, fully revalidate, and durably prepare one C1 Quest transaction result.
///
/// The current fixed head must equal `outcome.basis_head` throughout. Successful return means the
/// artifact and candidate immutable objects exist and fully verify, but `gore-project.json` is
/// unchanged. Publishing remains a separate external CAS operation.
pub fn prepare_revision3_quest_draft_persistence_v3(
    store: &WorkingProjectStore,
    outcome: Revision3QuestDraftInsertOutcomeV3,
) -> Result<Revision3QuestDraftPersistencePreparationV3, Revision3QuestDraftPersistenceErrorV3> {
    prepare_revision3_quest_draft_persistence_v3_with_after_import_hook(store, outcome, || Ok(()))
}

#[doc(hidden)]
pub(crate) fn prepare_revision3_quest_draft_persistence_v3_with_after_import_hook<F>(
    store: &WorkingProjectStore,
    outcome: Revision3QuestDraftInsertOutcomeV3,
    after_import: F,
) -> Result<Revision3QuestDraftPersistencePreparationV3, Revision3QuestDraftPersistenceErrorV3>
where
    F: FnOnce() -> Result<(), WorkingStoreError>,
{
    let Revision3QuestDraftInsertOutcomeV3 {
        project,
        canonical_project_json,
        collision_artifact,
        basis_head,
        quest_id,
        script_module_id,
        build_status,
        runtime_status,
        artifact_authority,
        source_inspection,
        publication_status,
    } = outcome;

    if build_status != Revision3QuestDraftBuildStatusV3::Blocked
        || runtime_status != Revision3QuestDraftRuntimeStatusV3::RuntimeUnqualified
        || artifact_authority != Revision3QuestArtifactAuthorityV3::NotGranted
        || source_inspection != Revision3QuestSourceInspectionStatusV3::FreshCapabilityRequired
        || publication_status != Revision3QuestDraftPublicationStatusV3::NotSupported
    {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::StatusDrift.into());
    }

    let reopened_project = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestDraftPersistenceErrorV3::CandidateProject)?;
    if reopened_project != project {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CandidateTransportMismatch.into(),
        );
    }
    let reopened_artifact = reopen_quest_collision_capability_artifact_v2(
        collision_artifact.canonical_json(),
        collision_artifact.artifact_seal(),
        collision_artifact.source_seal(),
    )
    .map_err(Revision3QuestDraftPersistenceErrorV3::Artifact)?;
    if reopened_artifact != collision_artifact {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::ArtifactReopenMismatch.into());
    }
    if raw_authoring_seal(collision_artifact.canonical_json())
        != authoring_seal(collision_artifact.artifact_seal())
    {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::ArtifactReopenMismatch.into());
    }

    let opened_basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(Revision3QuestDraftPersistenceErrorV3::Store)?;
    if opened_basis.head != basis_head {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::BasisHeadMismatch.into());
    }
    let basis_source = store
        .prepare_current_revision3_quest_collision_source_v2(&basis_head)
        .map_err(Revision3QuestDraftPersistenceErrorV3::BasisSource)?;
    validate_artifact_basis(&collision_artifact, &basis_head, &basis_source)?;
    validate_candidate_delta(
        &opened_basis.project,
        &project,
        &collision_artifact,
        &basis_head,
        quest_id,
        script_module_id,
    )?;

    let imported_artifact = store
        .import_quest_collision_artifact_v2(collision_artifact.canonical_json(), &basis_head)
        .map_err(Revision3QuestDraftPersistenceErrorV3::Store)?;
    let raw_artifact = authoring_seal(collision_artifact.artifact_seal());
    let expected_meta = AssetMeta {
        byte_len: raw_artifact.byte_len,
        media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
    };
    if imported_artifact.artifact != raw_artifact
        || imported_artifact.asset_meta != expected_meta
        || imported_artifact.basis_head != basis_head
    {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::ArtifactReceiptMismatch.into(),
        );
    }
    after_import().map_err(Revision3QuestDraftPersistenceErrorV3::Store)?;

    let checkpoint = store
        .prepare_revision3_checkpoint(Some(&basis_head), &project)
        .map_err(Revision3QuestDraftPersistenceErrorV3::Store)?;
    let reopened_checkpoint = store
        .open_revision3_head_bytes(&checkpoint.head_bytes, AssetVerification::Full)
        .map_err(Revision3QuestDraftPersistenceErrorV3::Store)?;
    if reopened_checkpoint.head != checkpoint.head || reopened_checkpoint.project != project {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CheckpointReopenMismatch.into(),
        );
    }

    Ok(Revision3QuestDraftPersistencePreparationV3 {
        project,
        canonical_project_json,
        collision_artifact,
        basis_head,
        quest_id,
        script_module_id,
        imported_artifact,
        checkpoint,
        build_status,
        runtime_status,
        artifact_authority,
        source_inspection,
        publication_status,
    })
}

fn validate_artifact_basis(
    artifact: &QuestCollisionCapabilityArtifactV2,
    basis_head: &WorkingHead,
    source: &gore_authoring::PreparedRevision3QuestCollisionSourceV2,
) -> Result<(), Revision3QuestDraftPersistenceValidationErrorV3> {
    if artifact.current_head() != basis_head
        || artifact.project_id() != source.project_id()
        || artifact.project_revision() != source.project_revision()
        || artifact.project_target() != source.target()
        || artifact.current_project() != source.current_project()
        || artifact.nonquest_project() != source.nonquest_basis().canonical_project()
        || artifact.prior_quest_count() != source.prior_quest_count_u64()
        || artifact.prior_quest_evidence() != source.prior_quest_evidence()
    {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::ArtifactBasisMismatch);
    }
    Ok(())
}

fn validate_candidate_delta(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    artifact: &QuestCollisionCapabilityArtifactV2,
    basis_head: &WorkingHead,
    quest_id: EntityId,
    script_module_id: EntityId,
) -> Result<(), Revision3QuestDraftPersistenceErrorV3> {
    if basis
        .revision
        .checked_add(1)
        .is_none_or(|expected| candidate.revision != expected)
    {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CandidateRevisionMismatch.into(),
        );
    }
    if candidate.format != basis.format
        || candidate.schema_revision != basis.schema_revision
        || candidate.project_id != basis.project_id
        || candidate.meta != basis.meta
        || candidate.target != basis.target
        || candidate.authoring_locales != basis.authoring_locales
    {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CandidateProjectMetadataMismatch
                .into(),
        );
    }
    if quest_id == script_module_id
        || basis.entities.contains_key(&quest_id)
        || basis.entities.contains_key(&script_module_id)
        || candidate.entities.len() != basis.entities.len().saturating_add(2)
        || basis
            .entities
            .iter()
            .any(|(id, entity)| candidate.entities.get(id) != Some(entity))
    {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CandidateEntityDeltaMismatch.into(),
        );
    }

    let raw_artifact = authoring_seal(artifact.artifact_seal());
    let semantic_artifact = authoring_seal(artifact.source_seal());
    let expected_meta = AssetMeta {
        byte_len: raw_artifact.byte_len,
        media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
    };
    if basis
        .asset_store
        .assets
        .get(&raw_artifact.sha256)
        .is_some_and(|existing| existing != &expected_meta)
    {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CandidateAssetDeltaMismatch.into(),
        );
    }
    let mut expected_assets = basis.asset_store.clone();
    expected_assets
        .assets
        .insert(raw_artifact.sha256, expected_meta);
    if candidate.asset_store != expected_assets {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::CandidateAssetDeltaMismatch.into(),
        );
    }

    let quest_entity = candidate
        .entities
        .get(&quest_id)
        .ok_or(Revision3QuestDraftPersistenceValidationErrorV3::CandidateEntityDeltaMismatch)?;
    let quest = match &quest_entity.payload {
        EntityPayload::QuestDraft(quest) => quest,
        _ => {
            return Err(Revision3QuestDraftPersistenceValidationErrorV3::QuestEntityMismatch.into())
        }
    };
    let expected_module_ref = TypedRef::new(
        candidate.project_id,
        script_module_id,
        EntityKind::ScriptModule,
    );
    let expected_artifact_ref = gore_authoring::QuestCollisionArtifactRef {
        generation: candidate.target.clone(),
        catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2.to_owned(),
        artifact: raw_artifact,
        source_seal: semantic_artifact,
        basis_snapshot: basis_head.snapshot.clone(),
    };
    if quest_entity.id != quest_id
        || quest_entity.revision != 0
        || !is_valid_revision3_quest_draft_display_name_v3(&quest_entity.display_name)
        || quest.generator_id != REVISION3_QUEST_GENERATOR_ID
        || quest.generator_version != REVISION3_QUEST_GENERATOR_VERSION
        || quest.input.target != candidate.target
        || quest.input.quest_id != quest_id
        || quest.script_module != expected_module_ref
        || quest.input.collision_catalog != expected_artifact_ref
        || !matches!(
            &quest_entity.origin,
            OriginRef::New { authored_runtime_id }
                if authored_runtime_id == &quest.input.technical_id
        )
    {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::QuestEntityMismatch.into());
    }
    if basis.entities.values().any(|entity| {
        matches!(
            (&entity.payload, &entity.origin),
            (
                EntityPayload::NpcDraft(_) | EntityPayload::QuestDraft(_),
                OriginRef::New { authored_runtime_id }
            ) if authored_runtime_id.eq_ignore_ascii_case(&quest.input.technical_id)
        )
    }) {
        return Err(
            Revision3QuestDraftPersistenceValidationErrorV3::RuntimeIdentityCollision.into(),
        );
    }

    let module_entity = candidate
        .entities
        .get(&script_module_id)
        .ok_or(Revision3QuestDraftPersistenceValidationErrorV3::CandidateEntityDeltaMismatch)?;
    let persisted_module = match &module_entity.payload {
        EntityPayload::ScriptModule(module) => module,
        _ => {
            return Err(
                Revision3QuestDraftPersistenceValidationErrorV3::ScriptModuleMismatch.into(),
            )
        }
    };
    let owner = TypedRef::new(candidate.project_id, quest_id, EntityKind::QuestDraft);
    if module_entity.id != script_module_id
        || module_entity.revision != 0
        || module_entity.display_name != persisted_module.module_namespace
        || !matches!(
            &module_entity.origin,
            OriginRef::Generated {
                generator_id,
                generator_version,
                owner: actual_owner,
            } if generator_id == REVISION3_QUEST_GENERATOR_ID
                && *generator_version == REVISION3_QUEST_GENERATOR_VERSION
                && actual_owner == &owner
        )
    {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::ScriptModuleMismatch.into());
    }
    let collision_input = artifact
        .structural_collision_input()
        .map_err(Revision3QuestDraftPersistenceErrorV3::Artifact)?;
    let regenerated = regenerate_revision3_quest_module_v2(quest, collision_input)
        .map_err(Revision3QuestDraftPersistenceErrorV3::Generation)?;
    if &regenerated != persisted_module {
        return Err(Revision3QuestDraftPersistenceValidationErrorV3::ScriptModuleMismatch.into());
    }
    Ok(())
}

fn authoring_seal(seal: &super::ContentSeal) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: seal.byte_len,
        sha256: AuthoringSha256Digest::from_bytes(*seal.sha256.as_bytes()),
    }
}

fn raw_authoring_seal(bytes: &[u8]) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: bytes.len() as u64,
        sha256: AuthoringSha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}
