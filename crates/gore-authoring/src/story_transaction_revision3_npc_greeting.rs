//! Exact-basis, project-only NPC greeting authoring for schema revision 3.
//!
//! Greetings are ordered authoring metadata on one [`NpcDraft`]. They bind existing or newly
//! created project-local [`DialogLine`] entities without changing NPC generation input, source,
//! ScriptModule, assets, or any game/save state. Build, native/store publication, topic creation,
//! speaker authority, and runtime behavior are deliberately outside this pure transaction.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    EntityKind, EntityPayload, NpcGreetingBindingV1, ProjectRevision3, ProjectRevision3JsonError,
    TypedRef, MAX_REVISION3_NPC_GREETING_BINDINGS_V1,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    apply_revision3_dialog_line_insert_transaction_v1, EntityId, GameGenerationAnchor, ProjectId,
    Revision3DialogLineInsertConflictV1, Revision3DialogLineInsertErrorV1,
    Revision3DialogLineInsertEvaluationV1, Revision3DialogLineInsertRequestJsonErrorV1,
    Revision3DialogLineInsertRequestV1, Revision3DialogLocalizationActionV1, WorkingHead,
};

pub const MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Revision3NpcGreetingIntentV1 {
    Replace {
        bindings: Vec<NpcGreetingBindingV1>,
    },
    CreateAndInsert {
        index: u16,
        line: Revision3DialogLineInsertRequestV1,
    },
}

impl Revision3NpcGreetingIntentV1 {
    pub const fn mode(&self) -> Revision3NpcGreetingModeV1 {
        match self {
            Self::Replace { .. } => Revision3NpcGreetingModeV1::Replace,
            Self::CreateAndInsert { .. } => Revision3NpcGreetingModeV1::CreateAndInsert,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcGreetingEditRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub npc_id: EntityId,
    pub expected_npc_revision: u64,
    pub intent: Revision3NpcGreetingIntentV1,
}

impl Revision3NpcGreetingEditRequestV1 {
    pub fn from_json(json: &str) -> Result<Self, Revision3NpcGreetingEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3NpcGreetingEditRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3NpcGreetingEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3NpcGreetingEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3NpcGreetingEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3NpcGreetingEditRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3NpcGreetingEditRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3NpcGreetingEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3NpcGreetingEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcGreetingEditRequestJsonErrorV1 {
    #[error("revision-3 NPC greeting request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 NPC greeting request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 NPC greeting request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 NPC greeting request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 NPC greeting request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3NpcGreetingEditConflictV1 {
    #[error("request basis head does not match the exact supplied head")]
    CurrentHeadMismatch,
    #[error("expected project {expected}, but exact basis is {actual}")]
    ProjectIdentityMismatch {
        expected: ProjectId,
        actual: ProjectId,
    },
    #[error("expected project revision {expected}, but exact basis is {actual}")]
    ProjectRevisionConflict { expected: u64, actual: u64 },
    #[error("request target does not match the exact project target")]
    ProjectTargetMismatch,
    #[error("project revision cannot be incremented")]
    ProjectRevisionOverflow,
    #[error("NPC entity ID must not be zero")]
    ZeroNpcId,
    #[error("NPC {npc} is missing or has the wrong kind")]
    InvalidNpcEntity { npc: EntityId },
    #[error("expected NPC revision {expected}, but exact entity is {actual}")]
    NpcRevisionConflict { expected: u64, actual: u64 },
    #[error("NPC {npc} revision cannot be incremented")]
    NpcRevisionOverflow { npc: EntityId },
    #[error("NPC {npc} has an invalid source/module closure: {reason}")]
    InvalidNpcClosure { npc: EntityId, reason: String },
    #[error("NPC {npc} owned ScriptModule {module} differs from exact regeneration")]
    OwnedModuleDrift { npc: EntityId, module: EntityId },
    #[error("NPC greeting list has {actual} bindings; maximum is {max}")]
    TooManyBindings { actual: usize, max: usize },
    #[error("NPC greeting binding {index} has an invalid line reference: {reason}")]
    InvalidLineReference { index: usize, reason: String },
    #[error("NPC greeting binding {index} duplicates DialogLine {line}")]
    DuplicateLine { index: usize, line: EntityId },
    #[error("create-and-insert index {index} exceeds greeting-list length {len}")]
    InsertIndexOutOfBounds { index: usize, len: usize },
    #[error("embedded dialog-line request is not bound to the exact NPC greeting basis")]
    DialogRequestBasisMismatch,
    #[error("embedded dialog-line request was rejected: {conflict}")]
    DialogLineRejected {
        conflict: Revision3DialogLineInsertConflictV1,
    },
    #[error("NPC greeting replacement makes no change")]
    NoChanges,
    #[error("candidate project exceeds the {limit}-byte limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcGreetingEditRejectionV1 {
    pub conflict: Revision3NpcGreetingEditConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3NpcGreetingModeV1 {
    Replace,
    CreateAndInsert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcGreetingCreatedLineV1 {
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub voice_slot_id: Option<EntityId>,
    pub localization_action: Revision3DialogLocalizationActionV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcGreetingBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcGreetingRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcGreetingTopicAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcGreetingPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcGreetingEditOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub npc_id: EntityId,
    pub script_module_id: EntityId,
    pub npc_revision: u64,
    pub script_module_revision: u64,
    pub mode: Revision3NpcGreetingModeV1,
    pub greeting_count: u64,
    pub created: Option<Revision3NpcGreetingCreatedLineV1>,
    pub build_status: Revision3NpcGreetingBuildStatusV1,
    pub runtime_status: Revision3NpcGreetingRuntimeStatusV1,
    pub topic_authority: Revision3NpcGreetingTopicAuthorityV1,
    pub publication_status: Revision3NpcGreetingPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3NpcGreetingEditEvaluationV1 {
    Applied(Box<Revision3NpcGreetingEditOutcomeV1>),
    Rejected(Revision3NpcGreetingEditRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcGreetingEditErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 NPC greeting request: {0}")]
    InvalidRequest(#[source] Revision3NpcGreetingEditRequestJsonErrorV1),
    #[error("embedded dialog-line request could not be serialized: {0}")]
    InvalidEmbeddedDialogRequest(#[source] Revision3DialogLineInsertRequestJsonErrorV1),
    #[error("embedded dialog-line transaction failed: {0}")]
    DialogLineTransaction(#[source] Revision3DialogLineInsertErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 NPC greeting candidate reopen changed the project")]
    CanonicalReopenMismatch,
    #[error("NPC greeting candidate changed state outside its exact transaction boundary")]
    CandidatePreservationMismatch,
}

pub fn apply_revision3_npc_greeting_edit_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3NpcGreetingEditEvaluationV1, Revision3NpcGreetingEditErrorV1> {
    let project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3NpcGreetingEditErrorV1::InvalidProject)?;
    let request = Revision3NpcGreetingEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3NpcGreetingEditErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3NpcGreetingEditEvaluationV1::Rejected(
                Revision3NpcGreetingEditRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3NpcGreetingEditConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3NpcGreetingEditConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3NpcGreetingEditConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3NpcGreetingEditConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3NpcGreetingEditConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.npc_id) {
        reject!(Revision3NpcGreetingEditConflictV1::ZeroNpcId);
    }

    let Some(npc_entity) = project.entities.get(&request.npc_id) else {
        reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcEntity {
            npc: request.npc_id,
        });
    };
    let EntityPayload::NpcDraft(npc) = &npc_entity.payload else {
        reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcEntity {
            npc: request.npc_id,
        });
    };
    if request.expected_npc_revision != npc_entity.revision {
        reject!(Revision3NpcGreetingEditConflictV1::NpcRevisionConflict {
            expected: request.expected_npc_revision,
            actual: npc_entity.revision,
        });
    }
    let Some(next_npc_revision) = npc_entity.revision.checked_add(1) else {
        reject!(Revision3NpcGreetingEditConflictV1::NpcRevisionOverflow {
            npc: request.npc_id,
        });
    };
    let script_module_id = npc.script_module.id;
    if npc.script_module.project_id != project.project_id
        || npc.script_module.expected_kind != EntityKind::ScriptModule
        || is_zero_entity_id(script_module_id)
    {
        reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcClosure {
            npc: request.npc_id,
            reason: "ScriptModule reference is not exact-project, non-zero, and kind-bound"
                .to_owned(),
        });
    }
    let Some(module_entity) = project.entities.get(&script_module_id) else {
        reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcClosure {
            npc: request.npc_id,
            reason: "owned ScriptModule is missing".to_owned(),
        });
    };
    let EntityPayload::ScriptModule(existing_module) = &module_entity.payload else {
        reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcClosure {
            npc: request.npc_id,
            reason: "owned entity is not a ScriptModule".to_owned(),
        });
    };
    if existing_module.owner.project_id != project.project_id
        || existing_module.owner.id != request.npc_id
        || existing_module.owner.expected_kind != EntityKind::NpcDraft
    {
        reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcClosure {
            npc: request.npc_id,
            reason: "ScriptModule owner is not the exact NPC".to_owned(),
        });
    }
    let regenerated = match npc.regenerate_script_module(TypedRef::new(
        project.project_id,
        request.npc_id,
        EntityKind::NpcDraft,
    )) {
        Ok(module) => module,
        Err(error) => {
            reject!(Revision3NpcGreetingEditConflictV1::InvalidNpcClosure {
                npc: request.npc_id,
                reason: error.to_string(),
            });
        }
    };
    if &regenerated != existing_module {
        reject!(Revision3NpcGreetingEditConflictV1::OwnedModuleDrift {
            npc: request.npc_id,
            module: script_module_id,
        });
    }
    let script_module_revision = module_entity.revision;
    let basis_greetings = npc.greetings.clone();
    let mode = request.intent.mode();

    let (mut candidate, greetings, created) = match &request.intent {
        Revision3NpcGreetingIntentV1::Replace { bindings } => {
            if let Err(conflict) = validate_bindings(&project, bindings) {
                reject!(conflict);
            }
            if bindings == &basis_greetings {
                reject!(Revision3NpcGreetingEditConflictV1::NoChanges);
            }
            (project.clone(), bindings.clone(), None)
        }
        Revision3NpcGreetingIntentV1::CreateAndInsert { index, line } => {
            let index = usize::from(*index);
            if index > basis_greetings.len() {
                reject!(Revision3NpcGreetingEditConflictV1::InsertIndexOutOfBounds {
                    index,
                    len: basis_greetings.len(),
                });
            }
            if basis_greetings.len() >= MAX_REVISION3_NPC_GREETING_BINDINGS_V1 {
                reject!(Revision3NpcGreetingEditConflictV1::TooManyBindings {
                    actual: basis_greetings.len().saturating_add(1),
                    max: MAX_REVISION3_NPC_GREETING_BINDINGS_V1,
                });
            }
            if line.expected_head != *exact_basis_head
                || line.expected_project_id != project.project_id
                || line.expected_revision != project.revision
                || line.expected_target != project.target
            {
                reject!(Revision3NpcGreetingEditConflictV1::DialogRequestBasisMismatch);
            }
            let line_json = line
                .to_canonical_json()
                .map_err(Revision3NpcGreetingEditErrorV1::InvalidEmbeddedDialogRequest)?;
            let dialog = match apply_revision3_dialog_line_insert_transaction_v1(
                exact_basis_head,
                canonical_project_json,
                &line_json,
            )
            .map_err(Revision3NpcGreetingEditErrorV1::DialogLineTransaction)?
            {
                Revision3DialogLineInsertEvaluationV1::Applied(outcome) => *outcome,
                Revision3DialogLineInsertEvaluationV1::Rejected(rejection) => {
                    reject!(Revision3NpcGreetingEditConflictV1::DialogLineRejected {
                        conflict: rejection.conflict,
                    });
                }
            };
            if dialog.project.revision != next_project_revision
                || dialog.basis_head != *exact_basis_head
                || dialog.line_id != line.line_id
                || dialog.project.entities.get(&request.npc_id)
                    != project.entities.get(&request.npc_id)
                || dialog.project.entities.get(&script_module_id)
                    != project.entities.get(&script_module_id)
            {
                return Err(Revision3NpcGreetingEditErrorV1::CanonicalReopenMismatch);
            }
            let mut greetings = basis_greetings.clone();
            greetings.insert(
                index,
                NpcGreetingBindingV1 {
                    line: TypedRef::new(project.project_id, line.line_id, EntityKind::DialogLine),
                },
            );
            if let Err(conflict) = validate_bindings(&dialog.project, &greetings) {
                reject!(conflict);
            }
            let created = Revision3NpcGreetingCreatedLineV1 {
                line_id: dialog.line_id,
                localization_id: dialog.localization_id,
                voice_slot_id: dialog.voice_slot_id,
                localization_action: dialog.localization_action,
            };
            (dialog.project, greetings, Some(created))
        }
    };

    let greeting_edit_basis = candidate.clone();
    let Some(candidate_npc_entity) = candidate.entities.get_mut(&request.npc_id) else {
        return Err(Revision3NpcGreetingEditErrorV1::CanonicalReopenMismatch);
    };
    let EntityPayload::NpcDraft(candidate_npc) = &mut candidate_npc_entity.payload else {
        return Err(Revision3NpcGreetingEditErrorV1::CanonicalReopenMismatch);
    };
    candidate_npc.greetings = greetings;
    candidate_npc_entity.revision = next_npc_revision;
    candidate.revision = next_project_revision;

    let canonical_project_json = match candidate.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3NpcGreetingEditConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3NpcGreetingEditConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3NpcGreetingEditErrorV1::ReopenCandidate)?;
    if reopened != candidate {
        return Err(Revision3NpcGreetingEditErrorV1::CanonicalReopenMismatch);
    }
    if !preserves_exact_greeting_edit(
        &greeting_edit_basis,
        &reopened,
        request.npc_id,
        next_project_revision,
        next_npc_revision,
    ) {
        return Err(Revision3NpcGreetingEditErrorV1::CandidatePreservationMismatch);
    }
    let greeting_count = reopened
        .entities
        .get(&request.npc_id)
        .and_then(|entity| match &entity.payload {
            EntityPayload::NpcDraft(npc) => Some(npc.greetings.len() as u64),
            _ => None,
        })
        .ok_or(Revision3NpcGreetingEditErrorV1::CanonicalReopenMismatch)?;

    Ok(Revision3NpcGreetingEditEvaluationV1::Applied(Box::new(
        Revision3NpcGreetingEditOutcomeV1 {
            project: reopened,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            npc_id: request.npc_id,
            script_module_id,
            npc_revision: next_npc_revision,
            script_module_revision,
            mode,
            greeting_count,
            created,
            build_status: Revision3NpcGreetingBuildStatusV1::Blocked,
            runtime_status: Revision3NpcGreetingRuntimeStatusV1::RuntimeUnqualified,
            topic_authority: Revision3NpcGreetingTopicAuthorityV1::NotGranted,
            publication_status: Revision3NpcGreetingPublicationStatusV1::NotSupported,
        },
    )))
}

fn validate_bindings(
    project: &ProjectRevision3,
    bindings: &[NpcGreetingBindingV1],
) -> Result<(), Revision3NpcGreetingEditConflictV1> {
    if bindings.len() > MAX_REVISION3_NPC_GREETING_BINDINGS_V1 {
        return Err(Revision3NpcGreetingEditConflictV1::TooManyBindings {
            actual: bindings.len(),
            max: MAX_REVISION3_NPC_GREETING_BINDINGS_V1,
        });
    }
    let mut line_ids = BTreeSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.line.project_id != project.project_id {
            return Err(Revision3NpcGreetingEditConflictV1::InvalidLineReference {
                index,
                reason: "line belongs to another project".to_owned(),
            });
        }
        if binding.line.expected_kind != EntityKind::DialogLine {
            return Err(Revision3NpcGreetingEditConflictV1::InvalidLineReference {
                index,
                reason: "typed reference does not expect DialogLine".to_owned(),
            });
        }
        if !line_ids.insert(binding.line.id) {
            return Err(Revision3NpcGreetingEditConflictV1::DuplicateLine {
                index,
                line: binding.line.id,
            });
        }
        let Some(line) = project.entities.get(&binding.line.id) else {
            return Err(Revision3NpcGreetingEditConflictV1::InvalidLineReference {
                index,
                reason: "DialogLine is missing".to_owned(),
            });
        };
        if !matches!(line.payload, EntityPayload::DialogLine(_)) {
            return Err(Revision3NpcGreetingEditConflictV1::InvalidLineReference {
                index,
                reason: "target entity has the wrong kind".to_owned(),
            });
        }
    }
    Ok(())
}

fn preserves_exact_greeting_edit(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    npc_id: EntityId,
    expected_project_revision: u64,
    expected_npc_revision: u64,
) -> bool {
    if candidate.project_id != basis.project_id
        || candidate.revision != expected_project_revision
        || candidate.meta != basis.meta
        || candidate.target != basis.target
        || candidate.authoring_locales != basis.authoring_locales
        || candidate.asset_store != basis.asset_store
        || candidate.entities.len() != basis.entities.len()
        || candidate.entities.keys().ne(basis.entities.keys())
    {
        return false;
    }

    for (id, entity) in &basis.entities {
        let Some(candidate_entity) = candidate.entities.get(id) else {
            return false;
        };
        if *id != npc_id {
            if candidate_entity != entity {
                return false;
            }
            continue;
        }
        if candidate_entity.id != entity.id
            || candidate_entity.display_name != entity.display_name
            || candidate_entity.origin != entity.origin
            || candidate_entity.revision != expected_npc_revision
        {
            return false;
        }
        let (EntityPayload::NpcDraft(basis_npc), EntityPayload::NpcDraft(candidate_npc)) =
            (&entity.payload, &candidate_entity.payload)
        else {
            return false;
        };
        let mut expected_npc = basis_npc.clone();
        expected_npc.greetings = candidate_npc.greetings.clone();
        if candidate_npc != &expected_npc {
            return false;
        }
    }
    true
}

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

struct BoundedRequestWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedRequestWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedRequestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes.len().saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other("NPC greeting request limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
