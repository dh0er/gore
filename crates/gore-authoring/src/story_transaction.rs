use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model_revision2::{
    Entity, EntityKind, EntityPayload, NpcDraft, NpcDraftInput, NpcParentClassInput, OriginRef,
    ProjectRevision2, QuestCollisionCatalogInput, QuestDraft, QuestDraftInput, QuestGiverInput,
    QuestParentInput, StoryRegenerationError, TypedRef,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::working_store::validate_revision2_persistability;
use crate::{
    Diagnostic, DiagnosticCode, DraftQuestCollisionKind, DraftQuestField, DraftQuestSkeletonError,
    EntityId, LogicalNpcCloneDraftError, LogicalNpcCloneField, ProjectDocument, ProjectId,
    ProjectJsonError, Sha256Digest, ValidationProfile, WorkingStoreLimits,
    DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_PROJECT_JSON_BYTES,
};

/// The quest collision inventory alone may occupy 16 MiB. Keep bounded request parsing large
/// enough for that closed input while still rejecting before a recursive duplicate-key walk.
pub const MAX_STORY_DRAFT_INSERT_JSON_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_STORY_DRAFT_DISPLAY_NAME_BYTES: usize = 256;
const STORY_DRAFT_INSERT_REQUEST_BINDING_DOMAIN: &[u8] =
    b"gore-authoring.story-draft-insert-v1.request-binding\0";

/// Bind an FFI result to the three exact raw inputs accepted by Story Draft insertion.
///
/// Each component is prefixed by its unsigned 64-bit little-endian byte length. Callers must pass
/// the profile's canonical wire spelling so alternate JSON spellings cannot share a binding.
pub fn story_draft_insert_request_binding_sha256(
    project_json: &str,
    mutation_json: &str,
    profile: ValidationProfile,
) -> Sha256Digest {
    let profile = match profile {
        ValidationProfile::Production => b"production".as_slice(),
        ValidationProfile::Experimental => b"experimental".as_slice(),
    };
    let mut hasher = Sha256::new();
    hasher.update(STORY_DRAFT_INSERT_REQUEST_BINDING_DOMAIN);
    for bytes in [project_json.as_bytes(), mutation_json.as_bytes(), profile] {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Sha256Digest::from_bytes(hasher.finalize().into())
}

/// One exact project-CAS-bound request to add a Draft and its generated owned module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryDraftInsertRequest {
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub draft_id: EntityId,
    pub script_module_id: EntityId,
    pub display_name: String,
    pub draft: StoryDraftCreate,
}

impl StoryDraftInsertRequest {
    /// Parse the untouched request bytes through a recursive duplicate-key preflight and then the
    /// closed typed request. No `serde_json::Value` normalization is permitted on this path.
    pub fn from_json(json: &str) -> Result<Self, StoryDraftInsertJsonError> {
        if json.len() > MAX_STORY_DRAFT_INSERT_JSON_BYTES {
            return Err(StoryDraftInsertJsonError::InputTooLarge {
                actual: json.len(),
                limit: MAX_STORY_DRAFT_INSERT_JSON_BYTES,
            });
        }
        reject_duplicate_object_keys(json).map_err(StoryDraftInsertJsonError::InvalidJson)?;
        serde_json::from_str(json).map_err(StoryDraftInsertJsonError::InvalidJson)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "input",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum StoryDraftCreate {
    Npc(NpcDraftCreateInput),
    Quest(QuestDraftCreateInput),
}

/// Persistable NPC intent without the redundant target, which is always inherited from the
/// exact project snapshot named by the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDraftCreateInput {
    pub module_namespace: String,
    pub unique_name: String,
    pub parent_character_definition: NpcParentClassInput,
    pub parent_ai_agent_config: NpcParentClassInput,
    pub parent_spawn_definition: NpcParentClassInput,
}

/// Persistable quest intent without redundant target or quest ID. The project supplies the
/// target and the transaction's stable Draft ID becomes the generator's exact quest ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDraftCreateInput {
    pub module_namespace: String,
    pub technical_id: String,
    pub text_helper: String,
    pub parent_quest: QuestParentInput,
    pub giver: QuestGiverInput,
    pub title: String,
    pub description: String,
    pub objective_title: String,
    pub collision_catalog: QuestCollisionCatalogInput,
}

#[derive(Debug, thiserror::Error)]
pub enum StoryDraftInsertJsonError {
    #[error("story draft insert JSON exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid story draft insert JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
}

/// A semantic rejection never carries a candidate project. Callers may safely keep their exact
/// base bytes and show these stable diagnostics without guessing whether either entity landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryDraftInsertRejection {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryDraftInsertOutcome {
    pub project: ProjectRevision2,
    pub canonical_project_json: String,
    pub draft_id: EntityId,
    pub draft_kind: EntityKind,
    pub script_module_id: EntityId,
    pub diagnostics: Vec<Diagnostic>,
    pub blocks_build: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryDraftInsertEvaluation {
    Applied(Box<StoryDraftInsertOutcome>),
    Rejected(StoryDraftInsertRejection),
}

#[derive(Debug, thiserror::Error)]
pub enum StoryDraftInsertError {
    #[error("could not serialize the story transaction candidate: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("could not reopen the canonical story transaction candidate: {0}")]
    Reopen(#[source] ProjectJsonError),
    #[error("canonical story transaction reopen changed the candidate")]
    CanonicalReopenMismatch,
}

impl ProjectRevision2 {
    /// Atomically add one Draft and its exact generated module to this in-memory document.
    ///
    /// `self` is consumed. Every rejection returns diagnostics only, so no partially changed
    /// candidate can escape. Filesystem publication remains the managed session's separate CAS.
    pub fn insert_story_draft(
        mut self,
        request: StoryDraftInsertRequest,
        profile: ValidationProfile,
    ) -> Result<StoryDraftInsertEvaluation, StoryDraftInsertError> {
        let mut diagnostics = Vec::new();
        let store_limits = WorkingStoreLimits::default();
        let base_project_json = self
            .to_canonical_json()
            .map_err(StoryDraftInsertError::Serialize)?;
        if base_project_json.len() > MAX_PROJECT_JSON_BYTES {
            diagnostics.push(project_json_limit_diagnostic(base_project_json.len()));
        }

        if request.expected_project_id != self.project_id {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::ProjectIdentityMismatch,
                "expected_project_id",
                format!(
                    "story transaction expected project {}, but candidate is {}",
                    request.expected_project_id, self.project_id
                ),
            ));
        }
        if request.expected_revision != self.revision {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::ProjectRevisionConflict,
                "expected_revision",
                format!(
                    "story transaction expected project revision {}, but candidate is {}",
                    request.expected_revision, self.revision
                ),
            ));
        }
        if is_zero_entity_id(request.draft_id) {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::InvalidStoryMutation,
                "draft_id",
                "story Draft ID must not be all zeroes",
            ));
        }
        if is_zero_entity_id(request.script_module_id) {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::InvalidStoryMutation,
                "script_module_id",
                "generated ScriptModule ID must not be all zeroes",
            ));
        }
        if request.draft_id == request.script_module_id {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::DuplicateEntityId,
                "script_module_id",
                "Draft and generated ScriptModule IDs must be different",
            ));
        }
        for (field, id) in [
            ("draft_id", request.draft_id),
            ("script_module_id", request.script_module_id),
        ] {
            if self.entities.contains_key(&id) {
                diagnostics.push(
                    Diagnostic::project_error(
                        DiagnosticCode::DuplicateEntityId,
                        field,
                        format!("entity ID {id} already exists in the project"),
                    )
                    .related(id),
                );
            }
        }
        if request.display_name.trim().is_empty()
            || request.display_name.len() > MAX_STORY_DRAFT_DISPLAY_NAME_BYTES
            || request.display_name.chars().any(char::is_control)
        {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::InvalidStoryMutation,
                "display_name",
                format!(
                    "display_name must contain visible text, no control characters, and at most {MAX_STORY_DRAFT_DISPLAY_NAME_BYTES} UTF-8 bytes"
                ),
            ));
        }
        if let Err(error) = validate_revision2_persistability(&self, &store_limits) {
            diagnostics.push(persistability_diagnostic(error));
        }

        // An insert cannot safely reinterpret or repair an already malformed story ownership
        // graph. Runtime qualification warnings are expected Draft state and remain insertable.
        diagnostics.extend(structural_story_diagnostics(&self));
        if !diagnostics.is_empty() {
            return Ok(rejected(diagnostics));
        }

        let Some(next_revision) = self.revision.checked_add(1) else {
            return Ok(rejected(vec![Diagnostic::project_error(
                DiagnosticCode::ProjectRevisionOverflow,
                "revision",
                "project revision cannot be incremented",
            )]));
        };

        let owner_kind = match &request.draft {
            StoryDraftCreate::Npc(_) => EntityKind::NpcDraft,
            StoryDraftCreate::Quest(_) => EntityKind::QuestDraft,
        };
        let owner = TypedRef::new(self.project_id, request.draft_id, owner_kind);
        let module_ref = TypedRef::new(
            self.project_id,
            request.script_module_id,
            EntityKind::ScriptModule,
        );

        let (payload, runtime_id, module) = match request.draft {
            StoryDraftCreate::Npc(input) => {
                let draft = NpcDraft {
                    generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    input: NpcDraftInput {
                        target: self.target.clone(),
                        module_namespace: input.module_namespace,
                        unique_name: input.unique_name,
                        parent_character_definition: input.parent_character_definition,
                        parent_ai_agent_config: input.parent_ai_agent_config,
                        parent_spawn_definition: input.parent_spawn_definition,
                    },
                    script_module: module_ref.clone(),
                };
                let runtime_id = draft.input.unique_name.clone();
                let module = match draft.regenerate_script_module(owner.clone()) {
                    Ok(module) => module,
                    Err(error) => {
                        return Ok(rejected(vec![regeneration_diagnostic(
                            request.draft_id,
                            &error,
                        )]));
                    }
                };
                (EntityPayload::NpcDraft(draft), runtime_id, module)
            }
            StoryDraftCreate::Quest(input) => {
                let draft = QuestDraft {
                    generator_id: DRAFT_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: DRAFT_QUEST_GENERATOR_VERSION,
                    input: QuestDraftInput {
                        target: self.target.clone(),
                        quest_id: request.draft_id,
                        module_namespace: input.module_namespace,
                        technical_id: input.technical_id,
                        text_helper: input.text_helper,
                        parent_quest: input.parent_quest,
                        giver: input.giver,
                        title: input.title,
                        description: input.description,
                        objective_title: input.objective_title,
                        collision_catalog: input.collision_catalog,
                    },
                    script_module: module_ref.clone(),
                };
                let runtime_id = draft.input.technical_id.clone();
                let module = match draft.regenerate_script_module(owner.clone()) {
                    Ok(module) => module,
                    Err(error) => {
                        return Ok(rejected(vec![regeneration_diagnostic(
                            request.draft_id,
                            &error,
                        )]));
                    }
                };
                (EntityPayload::QuestDraft(draft), runtime_id, module)
            }
        };

        let draft_entity = Entity {
            id: request.draft_id,
            display_name: request.display_name,
            origin: OriginRef::New {
                authored_runtime_id: runtime_id,
            },
            revision: 0,
            payload,
        };
        let module_entity = Entity {
            id: request.script_module_id,
            display_name: module.module_namespace.clone(),
            origin: OriginRef::Generated {
                generator_id: module.generator_id.clone(),
                generator_version: module.generator_version,
                owner,
            },
            revision: 0,
            payload: EntityPayload::ScriptModule(module),
        };

        let replaced_draft = self.entities.insert(request.draft_id, draft_entity);
        let replaced_module = self
            .entities
            .insert(request.script_module_id, module_entity);
        debug_assert!(replaced_draft.is_none());
        debug_assert!(replaced_module.is_none());
        self.revision = next_revision;

        if let Err(error) = validate_revision2_persistability(&self, &store_limits) {
            return Ok(rejected(vec![persistability_diagnostic(error)]));
        }
        let candidate_structural = structural_story_diagnostics(&self);
        if !candidate_structural.is_empty() {
            return Ok(rejected(candidate_structural));
        }

        let canonical_project_json = self
            .to_canonical_json()
            .map_err(StoryDraftInsertError::Serialize)?;
        if canonical_project_json.len() > MAX_PROJECT_JSON_BYTES {
            return Ok(rejected(vec![project_json_limit_diagnostic(
                canonical_project_json.len(),
            )]));
        }
        let reopened = ProjectRevision2::from_json(&canonical_project_json)
            .map_err(StoryDraftInsertError::Reopen)?;
        if reopened != self {
            return Err(StoryDraftInsertError::CanonicalReopenMismatch);
        }

        let mut diagnostics = self.validate_story_entities_with_profile(profile);
        // Schema revision 2 deliberately remains non-buildable even under Experimental policy.
        // The working store reports the same blocker after checkpoint preparation/reopen.
        diagnostics.push(Diagnostic::project_error(
            DiagnosticCode::Revision2CombinedValidationUnavailable,
            "schema_revision",
            "schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented",
        ));
        sort_diagnostics(&mut diagnostics);
        let blocks_build = diagnostics.iter().any(|diagnostic| diagnostic.blocks_build);
        debug_assert!(blocks_build);

        Ok(StoryDraftInsertEvaluation::Applied(Box::new(
            StoryDraftInsertOutcome {
                project: self,
                canonical_project_json,
                draft_id: request.draft_id,
                draft_kind: owner_kind,
                script_module_id: request.script_module_id,
                diagnostics,
                blocks_build,
            },
        )))
    }
}

impl ProjectDocument {
    /// Dispatch a story insert only to schema revision 2. Revision 1 remains byte-for-byte frozen
    /// and cannot be implicitly migrated by a mutation command.
    pub fn insert_story_draft(
        self,
        request: StoryDraftInsertRequest,
        profile: ValidationProfile,
    ) -> Result<StoryDraftInsertEvaluation, StoryDraftInsertError> {
        match self {
            Self::Revision2(project) => project.insert_story_draft(request, profile),
            Self::Revision1(_) => Ok(rejected(vec![Diagnostic::project_error(
                DiagnosticCode::InvalidStoryMutation,
                "schema_revision",
                "story Draft insertion requires schema revision 2; revision 1 is never migrated implicitly",
            )])),
            Self::Revision3(_) => Ok(rejected(vec![Diagnostic::project_error(
                DiagnosticCode::InvalidStoryMutation,
                "schema_revision",
                "story Draft insertion requires schema revision 2; revision 3 is never mutated implicitly",
            )])),
        }
    }
}

fn rejected(mut diagnostics: Vec<Diagnostic>) -> StoryDraftInsertEvaluation {
    sort_diagnostics(&mut diagnostics);
    StoryDraftInsertEvaluation::Rejected(StoryDraftInsertRejection { diagnostics })
}

fn structural_story_diagnostics(project: &ProjectRevision2) -> Vec<Diagnostic> {
    project
        .validate_story_entities_with_profile(ValidationProfile::Experimental)
        .into_iter()
        .filter(|diagnostic| diagnostic.code != DiagnosticCode::RuntimeUnqualified)
        .collect()
}

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

fn persistability_diagnostic(error: crate::WorkingStoreError) -> Diagnostic {
    Diagnostic::project_error(
        DiagnosticCode::InvalidStoryMutation,
        "project",
        format!("story project is not persistable in the managed store: {error}"),
    )
}

fn project_json_limit_diagnostic(actual: usize) -> Diagnostic {
    Diagnostic::project_error(
        DiagnosticCode::InvalidStoryMutation,
        "project_json",
        format!(
            "story project JSON is {actual} bytes; this transaction supports at most {MAX_PROJECT_JSON_BYTES} bytes"
        ),
    )
}

fn regeneration_diagnostic(owner: EntityId, error: &StoryRegenerationError) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidGeneratorInput,
        owner,
        regeneration_error_path(error),
        error.to_string(),
    )
}

fn regeneration_error_path(error: &StoryRegenerationError) -> String {
    match error {
        StoryRegenerationError::GeneratorContract { .. } => "draft.generator_id".to_owned(),
        StoryRegenerationError::OwnerKind { .. } => "script_module_id".to_owned(),
        StoryRegenerationError::InvalidNpcProvenance(_) => "draft.input".to_owned(),
        StoryRegenerationError::InvalidNpcIntent(error) => {
            format!("draft.input.{}", npc_error_field(error))
        }
        StoryRegenerationError::NpcFingerprint(_) => "draft.input".to_owned(),
        StoryRegenerationError::InvalidQuestIntent(error) => {
            format!("draft.input.{}", quest_error_field(error))
        }
    }
}

fn npc_error_field(error: &LogicalNpcCloneDraftError) -> String {
    let field = match error {
        LogicalNpcCloneDraftError::EmptyValue { field }
        | LogicalNpcCloneDraftError::ValueTooLong { field, .. }
        | LogicalNpcCloneDraftError::InvalidIdentifierStart { field, .. }
        | LogicalNpcCloneDraftError::InvalidIdentifierCharacter { field, .. }
        | LogicalNpcCloneDraftError::ReservedIdentifier { field }
        | LogicalNpcCloneDraftError::UnexpectedParentClassPrefix { field, .. }
        | LogicalNpcCloneDraftError::ClassNameCollision { field, .. } => Some(*field),
        LogicalNpcCloneDraftError::TooManyModuleSegments { .. } => {
            return "module_namespace".to_owned();
        }
        LogicalNpcCloneDraftError::ReservedModuleSegment { index } => {
            return format!("module_namespace.segments[{index}]");
        }
    };
    match field.expect("matched NPC error field") {
        LogicalNpcCloneField::ModuleNamespace => "module_namespace".to_owned(),
        LogicalNpcCloneField::ModuleSegment { index } => {
            format!("module_namespace.segments[{index}]")
        }
        LogicalNpcCloneField::UniqueName => "unique_name".to_owned(),
        LogicalNpcCloneField::ParentCharacterDefinition => {
            "parent_character_definition.runtime_class".to_owned()
        }
        LogicalNpcCloneField::ParentAiAgentConfig => {
            "parent_ai_agent_config.runtime_class".to_owned()
        }
        LogicalNpcCloneField::ParentSpawnDefinition => {
            "parent_spawn_definition.runtime_class".to_owned()
        }
    }
}

fn quest_error_field(error: &DraftQuestSkeletonError) -> String {
    match error {
        DraftQuestSkeletonError::InvalidSeal { field }
        | DraftQuestSkeletonError::GenerationMismatch { field }
        | DraftQuestSkeletonError::EmptyValue { field }
        | DraftQuestSkeletonError::ValueTooLong { field, .. }
        | DraftQuestSkeletonError::InvalidCharacter { field, .. }
        | DraftQuestSkeletonError::ReservedIdentifier { field }
        | DraftQuestSkeletonError::NonCanonicalIdentifier { field, .. }
        | DraftQuestSkeletonError::NonCanonicalText { field } => quest_field(*field),
        DraftQuestSkeletonError::ZeroEntityId => "quest_id".to_owned(),
        DraftQuestSkeletonError::TooManyModuleSegments { .. } => "module_namespace".to_owned(),
        DraftQuestSkeletonError::ReservedModuleSegment { index } => {
            format!("module_namespace.segments[{index}]")
        }
        DraftQuestSkeletonError::InvalidParentQuestClass
        | DraftQuestSkeletonError::ParentClassCollision { .. } => {
            "parent_quest.runtime_class".to_owned()
        }
        DraftQuestSkeletonError::TooManyCollisionEntries { .. }
        | DraftQuestSkeletonError::CollisionCatalogTooLarge { .. } => {
            "collision_catalog".to_owned()
        }
        DraftQuestSkeletonError::UnsafeCollisionEntry { kind, .. }
        | DraftQuestSkeletonError::DuplicateCollisionEntry { kind, .. }
        | DraftQuestSkeletonError::GeneratedNameCollision { kind, .. } => collision_field(*kind),
        DraftQuestSkeletonError::GeneratedSymbolCollision { .. } => "generated_symbols".to_owned(),
        DraftQuestSkeletonError::TooManyObjectives { .. }
        | DraftQuestSkeletonError::ObjectiveTitlesTooLarge { .. }
        | DraftQuestSkeletonError::DuplicateObjectiveTitle { .. } => "objective_titles".to_owned(),
        DraftQuestSkeletonError::InvalidTransitionPlan { .. } => "transition_plan".to_owned(),
    }
}

fn quest_field(field: DraftQuestField) -> String {
    match field {
        DraftQuestField::GameGeneration => "target".to_owned(),
        DraftQuestField::GiverGeneration => "giver.generation".to_owned(),
        DraftQuestField::GiverSourceSeal => "giver.source_seal".to_owned(),
        DraftQuestField::GiverCatalogLayer => "giver.catalog_layer".to_owned(),
        DraftQuestField::GiverSelector => "giver.canonical_selector".to_owned(),
        DraftQuestField::GiverRuntimeUniqueName => "giver.runtime_unique_name".to_owned(),
        DraftQuestField::ParentGeneration => "parent_quest.generation".to_owned(),
        DraftQuestField::ParentSourceSeal => "parent_quest.source_seal".to_owned(),
        DraftQuestField::ParentCatalogLayer => "parent_quest.catalog_layer".to_owned(),
        DraftQuestField::ParentSelector => "parent_quest.canonical_selector".to_owned(),
        DraftQuestField::ParentRuntimeClass => "parent_quest.runtime_class".to_owned(),
        DraftQuestField::CollisionGeneration => "collision_catalog.generation".to_owned(),
        DraftQuestField::CollisionSourceSeal => "collision_catalog.source_seal".to_owned(),
        DraftQuestField::CollisionCatalogLayer => "collision_catalog.catalog_layer".to_owned(),
        DraftQuestField::ModuleNamespace => "module_namespace".to_owned(),
        DraftQuestField::ModuleSegment { index } => {
            format!("module_namespace.segments[{index}]")
        }
        DraftQuestField::TechnicalId => "technical_id".to_owned(),
        DraftQuestField::TextHelper => "text_helper".to_owned(),
        DraftQuestField::Title => "title".to_owned(),
        DraftQuestField::Description => "description".to_owned(),
        DraftQuestField::ObjectiveTitle => "objective_title".to_owned(),
        DraftQuestField::AdditionalObjectiveTitle { index } => {
            // `index` is zero-based across the complete objective list, whose first entry is
            // exposed separately as `objective_title`.
            format!("additional_objective_titles[{}]", index.saturating_sub(1))
        }
    }
}

fn collision_field(kind: DraftQuestCollisionKind) -> String {
    match kind {
        DraftQuestCollisionKind::Module => "collision_catalog.modules".to_owned(),
        DraftQuestCollisionKind::RelativePath => "collision_catalog.relative_paths".to_owned(),
        DraftQuestCollisionKind::Symbol => "collision_catalog.symbols".to_owned(),
    }
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    for diagnostic in diagnostics.iter_mut() {
        diagnostic.related_entities.sort_unstable();
        diagnostic.related_entities.dedup();
    }
    diagnostics.sort_by(|left, right| {
        (
            left.severity,
            left.entity,
            left.property_path.as_deref(),
            left.code,
            left.message.as_str(),
            left.related_entities.as_slice(),
        )
            .cmp(&(
                right.severity,
                right.entity,
                right.property_path.as_deref(),
                right.code,
                right.message.as_str(),
                right.related_entities.as_slice(),
            ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additional_objective_diagnostics_use_list_local_indices() {
        assert_eq!(
            quest_field(DraftQuestField::AdditionalObjectiveTitle { index: 1 }),
            "additional_objective_titles[0]"
        );
        assert_eq!(
            quest_field(DraftQuestField::AdditionalObjectiveTitle {
                index: crate::MAX_DRAFT_QUEST_OBJECTIVES - 1,
            }),
            "additional_objective_titles[6]"
        );
    }
}
