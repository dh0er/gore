//! Atomic, filesystem-free create/update/remove transactions for one managed revision-3 item
//! patch.
//!
//! The transaction consumes only an exact canonical project and an exact basis head. It returns
//! a reopened candidate but cannot publish a Store head, build a mod, deploy files, or claim
//! runtime support.

use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{Entity, EntityPayload, ItemPatchV1, ItemScalarValueV1, OriginRef};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    ContentSeal, EntityId, GameGenerationAnchor, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, WorkingHead, MAX_REVISION3_ENTITIES,
};

pub const MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1: usize = 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Revision3ItemPatchMutationV1 {
    /// `expected_entity_revision = None` creates; `Some` updates the exact existing patch.
    Upsert {
        entity_id: EntityId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_entity_revision: Option<u64>,
        display_name: String,
        catalog_layer: String,
        vanilla_class: String,
        source_seal: ContentSeal,
        fields: BTreeMap<String, ItemScalarValueV1>,
    },
    Remove {
        entity_id: EntityId,
        expected_entity_revision: u64,
        expected_catalog_layer: String,
        expected_vanilla_class: String,
        expected_source_seal: ContentSeal,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ItemPatchRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub mutation: Revision3ItemPatchMutationV1,
}

impl Revision3ItemPatchRequestV1 {
    pub fn from_json(json: &str) -> Result<Self, Revision3ItemPatchRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3ItemPatchRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3ItemPatchRequestJsonErrorV1::InvalidJson)?;
        let request = serde_json::from_str::<Self>(json)
            .map_err(Revision3ItemPatchRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3ItemPatchRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3ItemPatchRequestJsonErrorV1> {
        let mut writer = BoundedRequestWriter::new(MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1);
        let result = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3ItemPatchRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1,
            });
        }
        result.map_err(Revision3ItemPatchRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3ItemPatchRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3ItemPatchRequestJsonErrorV1 {
    #[error("revision-3 item-patch request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 item-patch request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 item-patch request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 item-patch request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 item-patch request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3ItemPatchConflictV1 {
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
    #[error("item-patch entity id must not be all zeroes")]
    ZeroEntityId,
    #[error("item-patch entity {entity} already exists")]
    EntityAlreadyExists { entity: EntityId },
    #[error("item-patch entity {entity} is missing or has the wrong kind")]
    EntityMissingOrWrongKind { entity: EntityId },
    #[error("item-patch entity {entity} revision differs: expected {expected}, actual {actual}")]
    EntityRevisionConflict {
        entity: EntityId,
        expected: u64,
        actual: u64,
    },
    #[error("item-patch entity {entity} provenance differs from the exact request")]
    ProvenanceConflict { entity: EntityId },
    #[error("another item patch already targets this exact sealed vanilla class")]
    DuplicateVanillaTarget,
    #[error("replacement item patch is identical to the exact basis")]
    NoChanges,
    #[error("project revision cannot be incremented")]
    ProjectRevisionOverflow,
    #[error("item-patch entity {entity} cannot increment its entity revision")]
    EntityRevisionOverflow { entity: EntityId },
    #[error("project cannot contain another entity")]
    EntityCapacityExceeded,
    #[error("candidate item patch is invalid or not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
    #[error("candidate project exceeds the {limit}-byte limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3ItemPatchRejectionV1 {
    pub conflict: Revision3ItemPatchConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3ItemPatchChangeV1 {
    Created,
    Updated,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3ItemPatchBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3ItemPatchRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3ItemPatchPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3ItemPatchOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub entity_id: EntityId,
    pub entity_revision: Option<u64>,
    pub change: Revision3ItemPatchChangeV1,
    pub build_status: Revision3ItemPatchBuildStatusV1,
    pub runtime_status: Revision3ItemPatchRuntimeStatusV1,
    pub publication_status: Revision3ItemPatchPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3ItemPatchEvaluationV1 {
    Applied(Box<Revision3ItemPatchOutcomeV1>),
    Rejected(Revision3ItemPatchRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3ItemPatchErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 item-patch request: {0}")]
    InvalidRequest(#[source] Revision3ItemPatchRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 item-patch candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Create, replace, or remove one exact managed item patch without filesystem authority.
pub fn apply_revision3_item_patch_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3ItemPatchEvaluationV1, Revision3ItemPatchErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3ItemPatchErrorV1::InvalidProject)?;
    let request = Revision3ItemPatchRequestV1::from_json(canonical_request_json)
        .map_err(Revision3ItemPatchErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3ItemPatchEvaluationV1::Rejected(
                Revision3ItemPatchRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3ItemPatchConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(Revision3ItemPatchConflictV1::ProjectIdentityMismatch {
            expected: request.expected_project_id,
            actual: project.project_id,
        });
    }
    if request.expected_revision != project.revision {
        reject!(Revision3ItemPatchConflictV1::ProjectRevisionConflict {
            expected: request.expected_revision,
            actual: project.revision,
        });
    }
    if request.expected_target != project.target {
        reject!(Revision3ItemPatchConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3ItemPatchConflictV1::ProjectRevisionOverflow);
    };

    let (entity_id, entity_revision, change) = match request.mutation {
        Revision3ItemPatchMutationV1::Upsert {
            entity_id,
            expected_entity_revision,
            display_name,
            catalog_layer,
            vanilla_class,
            source_seal,
            fields,
        } => {
            if is_zero_entity_id(entity_id) {
                reject!(Revision3ItemPatchConflictV1::ZeroEntityId);
            }
            let origin = OriginRef::Vanilla {
                generation: project.target.clone(),
                catalog_layer,
                canonical_selector: vanilla_class.clone(),
                source_seal,
            };
            let payload = EntityPayload::ItemPatch(ItemPatchV1 {
                vanilla_class,
                fields,
            });

            match expected_entity_revision {
                None => {
                    if project.entities.contains_key(&entity_id) {
                        reject!(Revision3ItemPatchConflictV1::EntityAlreadyExists {
                            entity: entity_id
                        });
                    }
                    if project.entities.len() >= MAX_REVISION3_ENTITIES {
                        reject!(Revision3ItemPatchConflictV1::EntityCapacityExceeded);
                    }
                    if has_duplicate_target(&project, entity_id, &origin) {
                        reject!(Revision3ItemPatchConflictV1::DuplicateVanillaTarget);
                    }
                    project.entities.insert(
                        entity_id,
                        Entity {
                            id: entity_id,
                            display_name,
                            origin,
                            revision: 0,
                            payload,
                        },
                    );
                    (entity_id, Some(0), Revision3ItemPatchChangeV1::Created)
                }
                Some(expected_entity_revision) => {
                    let Some(existing) = project.entities.get(&entity_id) else {
                        reject!(Revision3ItemPatchConflictV1::EntityMissingOrWrongKind {
                            entity: entity_id,
                        });
                    };
                    if !matches!(existing.payload, EntityPayload::ItemPatch(_)) {
                        reject!(Revision3ItemPatchConflictV1::EntityMissingOrWrongKind {
                            entity: entity_id,
                        });
                    }
                    if existing.revision != expected_entity_revision {
                        reject!(Revision3ItemPatchConflictV1::EntityRevisionConflict {
                            entity: entity_id,
                            expected: expected_entity_revision,
                            actual: existing.revision,
                        });
                    }
                    if existing.origin != origin {
                        reject!(Revision3ItemPatchConflictV1::ProvenanceConflict {
                            entity: entity_id,
                        });
                    }
                    if existing.display_name == display_name && existing.payload == payload {
                        reject!(Revision3ItemPatchConflictV1::NoChanges);
                    }
                    if has_duplicate_target(&project, entity_id, &origin) {
                        reject!(Revision3ItemPatchConflictV1::DuplicateVanillaTarget);
                    }
                    let Some(next_entity_revision) = existing.revision.checked_add(1) else {
                        reject!(Revision3ItemPatchConflictV1::EntityRevisionOverflow {
                            entity: entity_id,
                        });
                    };
                    let existing = project
                        .entities
                        .get_mut(&entity_id)
                        .expect("bound item patch remains present");
                    existing.display_name = display_name;
                    existing.payload = payload;
                    existing.revision = next_entity_revision;
                    (
                        entity_id,
                        Some(next_entity_revision),
                        Revision3ItemPatchChangeV1::Updated,
                    )
                }
            }
        }
        Revision3ItemPatchMutationV1::Remove {
            entity_id,
            expected_entity_revision,
            expected_catalog_layer,
            expected_vanilla_class,
            expected_source_seal,
        } => {
            if is_zero_entity_id(entity_id) {
                reject!(Revision3ItemPatchConflictV1::ZeroEntityId);
            }
            let Some(existing) = project.entities.get(&entity_id) else {
                reject!(Revision3ItemPatchConflictV1::EntityMissingOrWrongKind {
                    entity: entity_id,
                });
            };
            let EntityPayload::ItemPatch(patch) = &existing.payload else {
                reject!(Revision3ItemPatchConflictV1::EntityMissingOrWrongKind {
                    entity: entity_id,
                });
            };
            if existing.revision != expected_entity_revision {
                reject!(Revision3ItemPatchConflictV1::EntityRevisionConflict {
                    entity: entity_id,
                    expected: expected_entity_revision,
                    actual: existing.revision,
                });
            }
            let expected_origin = OriginRef::Vanilla {
                generation: project.target.clone(),
                catalog_layer: expected_catalog_layer,
                canonical_selector: expected_vanilla_class.clone(),
                source_seal: expected_source_seal,
            };
            if existing.origin != expected_origin || patch.vanilla_class != expected_vanilla_class {
                reject!(Revision3ItemPatchConflictV1::ProvenanceConflict { entity: entity_id });
            }
            project.entities.remove(&entity_id);
            (entity_id, None, Revision3ItemPatchChangeV1::Removed)
        }
    };

    project.revision = next_project_revision;
    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3ItemPatchConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => reject!(Revision3ItemPatchConflictV1::CandidateNotPersistable {
            reason: error.to_string(),
        }),
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3ItemPatchErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3ItemPatchErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3ItemPatchEvaluationV1::Applied(Box::new(
        Revision3ItemPatchOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            entity_id,
            entity_revision,
            change,
            build_status: Revision3ItemPatchBuildStatusV1::Blocked,
            runtime_status: Revision3ItemPatchRuntimeStatusV1::RuntimeUnqualified,
            publication_status: Revision3ItemPatchPublicationStatusV1::NotSupported,
        },
    )))
}

fn has_duplicate_target(project: &ProjectRevision3, except: EntityId, origin: &OriginRef) -> bool {
    let OriginRef::Vanilla {
        generation,
        canonical_selector,
        ..
    } = origin
    else {
        return false;
    };
    project.entities.iter().any(|(id, entity)| {
        if *id == except || !matches!(entity.payload, EntityPayload::ItemPatch(_)) {
            return false;
        }
        matches!(
            &entity.origin,
            OriginRef::Vanilla {
                generation: existing_generation,
                canonical_selector: existing_selector,
                ..
            } if existing_generation == generation
                && existing_selector == canonical_selector
        )
    })
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
            bytes: Vec::with_capacity(limit.min(16 * 1024)),
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
            return Err(io::Error::other(
                "revision-3 item-patch request limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
