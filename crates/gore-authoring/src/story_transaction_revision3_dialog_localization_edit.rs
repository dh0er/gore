//! Atomic, filesystem-free editing of one managed revision-3 localization entry.
//!
//! The transaction replaces the complete locale/text map of one exact authored
//! [`LocalizationEntry`]. It deliberately leaves the stable LocID, display name, provenance,
//! graph, assets, and every other entity untouched. Voice-bearing locales are protected from
//! becoming empty, and a locale whose VoiceSlot already has takes cannot change text. This is
//! authoring metadata only: it grants no dialog-topic, build, runtime, deployment, Store, save,
//! or fixed-head publication authority.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{EntityKind, EntityPayload, OriginRef};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    EntityId, GameGenerationAnchor, LocaleCode, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, WorkingHead, MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1, MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1,
};

pub const MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1: usize =
    MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1;

/// Exact project/head/entity-bound replacement of one complete localization text map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3DialogLocalizationEditRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub localization_id: EntityId,
    pub expected_localization_revision: u64,
    pub expected_loc_id: String,
    pub texts: BTreeMap<LocaleCode, String>,
}

impl Revision3DialogLocalizationEditRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(
        json: &str,
    ) -> Result<Self, Revision3DialogLocalizationEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3DialogLocalizationEditRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3DialogLocalizationEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3DialogLocalizationEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3DialogLocalizationEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3DialogLocalizationEditRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3DialogLocalizationEditRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3DialogLocalizationEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3DialogLocalizationEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogLocalizationEditRequestJsonErrorV1 {
    #[error("revision-3 localization-edit request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 localization-edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 localization-edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 localization-edit request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 localization-edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// A semantic rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3DialogLocalizationEditConflictV1 {
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
    #[error("LocalizationEntry {localization} is missing or has the wrong kind")]
    LocalizationMissingOrWrongKind { localization: EntityId },
    #[error(
        "LocalizationEntry {localization} revision differs: expected {expected}, actual {actual}"
    )]
    LocalizationRevisionConflict {
        localization: EntityId,
        expected: u64,
        actual: u64,
    },
    #[error(
        "LocalizationEntry {localization} identity differs: expected {expected:?}, actual {actual:?}"
    )]
    LocalizationIdentityConflict {
        localization: EntityId,
        expected: String,
        actual: String,
    },
    #[error("LocalizationEntry {localization} is not a newly authored managed entity")]
    LocalizationOriginNotNew { localization: EntityId },
    #[error("localization texts are empty, invalid, or exceed their closed budget")]
    InvalidLocalizationTexts,
    #[error("replacement localization texts are identical to the exact basis")]
    NoChanges,
    #[error("project revision cannot be incremented")]
    ProjectRevisionOverflow,
    #[error("LocalizationEntry {localization} cannot increment its entity revision")]
    LocalizationRevisionOverflow { localization: EntityId },
    #[error("VoiceSlot {slot} on DialogLine {line} requires a non-empty text for locale {locale}")]
    VoiceSlotLocaleRemovedOrBlank {
        line: EntityId,
        slot: EntityId,
        locale: LocaleCode,
    },
    #[error(
        "VoiceSlot {slot} on DialogLine {line} already has takes and protects locale {locale} text"
    )]
    VoiceSlotCandidatesProtectText {
        line: EntityId,
        slot: EntityId,
        locale: LocaleCode,
    },
    #[error("candidate project exceeds the {limit}-byte limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogLocalizationEditRejectionV1 {
    pub conflict: Revision3DialogLocalizationEditConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogLocalizationEditBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogLocalizationEditRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogLocalizationEditTopicAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogLocalizationEditPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogLocalizationEditOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub localization_id: EntityId,
    pub added_locales: BTreeSet<LocaleCode>,
    pub removed_locales: BTreeSet<LocaleCode>,
    pub build_status: Revision3DialogLocalizationEditBuildStatusV1,
    pub runtime_status: Revision3DialogLocalizationEditRuntimeStatusV1,
    pub topic_authority: Revision3DialogLocalizationEditTopicAuthorityV1,
    pub publication_status: Revision3DialogLocalizationEditPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3DialogLocalizationEditEvaluationV1 {
    Applied(Box<Revision3DialogLocalizationEditOutcomeV1>),
    Rejected(Revision3DialogLocalizationEditRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogLocalizationEditErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 localization-edit request: {0}")]
    InvalidRequest(#[source] Revision3DialogLocalizationEditRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 localization-edit candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Replace one exact managed localization's complete text map.
///
/// This function performs no filesystem operation and cannot publish a fixed working head. The
/// returned candidate remains build-blocked, runtime-unqualified, and without dialog-topic
/// authority.
pub fn apply_revision3_dialog_localization_edit_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3DialogLocalizationEditEvaluationV1, Revision3DialogLocalizationEditErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3DialogLocalizationEditErrorV1::InvalidProject)?;
    let request = Revision3DialogLocalizationEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3DialogLocalizationEditErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3DialogLocalizationEditEvaluationV1::Rejected(
                Revision3DialogLocalizationEditRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3DialogLocalizationEditConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3DialogLocalizationEditConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3DialogLocalizationEditConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3DialogLocalizationEditConflictV1::ProjectTargetMismatch);
    }

    let Some(localization_entity) = project.entities.get(&request.localization_id) else {
        reject!(
            Revision3DialogLocalizationEditConflictV1::LocalizationMissingOrWrongKind {
                localization: request.localization_id,
            }
        );
    };
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        reject!(
            Revision3DialogLocalizationEditConflictV1::LocalizationMissingOrWrongKind {
                localization: request.localization_id,
            }
        );
    };
    if localization_entity.revision != request.expected_localization_revision {
        reject!(
            Revision3DialogLocalizationEditConflictV1::LocalizationRevisionConflict {
                localization: request.localization_id,
                expected: request.expected_localization_revision,
                actual: localization_entity.revision,
            }
        );
    }
    if localization.loc_id != request.expected_loc_id {
        reject!(
            Revision3DialogLocalizationEditConflictV1::LocalizationIdentityConflict {
                localization: request.localization_id,
                expected: request.expected_loc_id.clone(),
                actual: localization.loc_id.clone(),
            }
        );
    }
    if !matches!(localization_entity.origin, OriginRef::New { .. }) {
        reject!(
            Revision3DialogLocalizationEditConflictV1::LocalizationOriginNotNew {
                localization: request.localization_id,
            }
        );
    }
    if !valid_replacement_texts(&request.texts) {
        reject!(Revision3DialogLocalizationEditConflictV1::InvalidLocalizationTexts);
    }
    if request.texts == localization.texts {
        reject!(Revision3DialogLocalizationEditConflictV1::NoChanges);
    }

    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3DialogLocalizationEditConflictV1::ProjectRevisionOverflow);
    };
    let Some(next_localization_revision) = localization_entity.revision.checked_add(1) else {
        reject!(
            Revision3DialogLocalizationEditConflictV1::LocalizationRevisionOverflow {
                localization: request.localization_id,
            }
        );
    };

    let previous_texts = localization.texts.clone();
    for (line_id, entity) in &project.entities {
        let EntityPayload::DialogLine(line) = &entity.payload else {
            continue;
        };
        if line.localization.project_id != project.project_id
            || line.localization.expected_kind != EntityKind::LocalizationEntry
            || line.localization.id != request.localization_id
        {
            continue;
        }
        for (locale, slot_reference) in &line.voice_slots {
            let replacement = request.texts.get(locale);
            if replacement.is_none_or(|text| text.trim().is_empty()) {
                reject!(
                    Revision3DialogLocalizationEditConflictV1::VoiceSlotLocaleRemovedOrBlank {
                        line: *line_id,
                        slot: slot_reference.id,
                        locale: locale.clone(),
                    }
                );
            }
            let slot_entity = project
                .entities
                .get(&slot_reference.id)
                .expect("canonical basis closed every DialogLine VoiceSlot reference");
            let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
                unreachable!("canonical basis kind-bound every DialogLine VoiceSlot reference")
            };
            if !slot.candidates.is_empty() && previous_texts.get(locale) != replacement {
                reject!(
                    Revision3DialogLocalizationEditConflictV1::VoiceSlotCandidatesProtectText {
                        line: *line_id,
                        slot: slot_reference.id,
                        locale: locale.clone(),
                    }
                );
            }
        }
    }

    let previous_locales = previous_texts.keys().cloned().collect::<BTreeSet<_>>();
    let replacement_locales = request.texts.keys().cloned().collect::<BTreeSet<_>>();
    let added_locales = replacement_locales
        .difference(&previous_locales)
        .cloned()
        .collect::<BTreeSet<_>>();
    let removed_locales = previous_locales
        .difference(&replacement_locales)
        .cloned()
        .collect::<BTreeSet<_>>();

    project
        .authoring_locales
        .extend(added_locales.iter().cloned());
    let localization_entity = project
        .entities
        .get_mut(&request.localization_id)
        .expect("bound localization remains present");
    let EntityPayload::LocalizationEntry(localization) = &mut localization_entity.payload else {
        unreachable!("bound localization kind remains stable")
    };
    localization.texts = request.texts;
    localization_entity.revision = next_localization_revision;
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3DialogLocalizationEditConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3DialogLocalizationEditConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3DialogLocalizationEditErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3DialogLocalizationEditErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3DialogLocalizationEditEvaluationV1::Applied(
        Box::new(Revision3DialogLocalizationEditOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            localization_id: request.localization_id,
            added_locales,
            removed_locales,
            build_status: Revision3DialogLocalizationEditBuildStatusV1::Blocked,
            runtime_status: Revision3DialogLocalizationEditRuntimeStatusV1::RuntimeUnqualified,
            topic_authority: Revision3DialogLocalizationEditTopicAuthorityV1::NotGranted,
            publication_status: Revision3DialogLocalizationEditPublicationStatusV1::NotSupported,
        }),
    ))
}

fn valid_replacement_texts(texts: &BTreeMap<LocaleCode, String>) -> bool {
    if texts.is_empty() || texts.len() > MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1 {
        return false;
    }
    let mut total = 0usize;
    let mut has_nonblank = false;
    for text in texts.values() {
        total = match total.checked_add(text.len()) {
            Some(total) => total,
            None => return false,
        };
        has_nonblank |= !text.trim().is_empty();
        if text.len() > MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1
            || text.contains('\0')
            || total > MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1
        {
            return false;
        }
    }
    has_nonblank
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
                "revision-3 localization-edit request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
