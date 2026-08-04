//! Atomic, filesystem-free binding of native voice-archive match evidence to one revision-3 slot.
//!
//! The caller supplies only the exact matches produced by a native archive inspection. Zero,
//! one, or multiple matches deterministically become an unresolved, resolved, or ambiguous
//! [`VoiceTargetResolution`]. This module never opens an archive, reads the game installation,
//! lowers a build, or grants deployment authority.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    revision3_voice_target_key_v1, DialogLine, EntityKind, EntityPayload, LocalizationEntry,
    VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTarget, VoiceTargetResolution,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    EntityId, GameGenerationAnchor, LocaleCode, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, WorkingHead,
};

pub const MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1: usize = 1024 * 1024;
pub const MAX_REVISION3_VOICE_TARGET_MATCHES_V1: usize = 512;
pub const MAX_REVISION3_VOICE_TARGET_ARCHIVE_BYTES_V1: usize = 255;
pub const MAX_REVISION3_VOICE_TARGET_MEMBER_BYTES_V1: usize = 1024;
/// Largest LocID stem whose `.ogg` member basename remains within the 1024-byte member limit.
pub const MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1: usize = 1020;
/// Largest installed archive identity that may be persisted as revision-3 Voice evidence.
///
/// This is the native inspection/deployment ceiling, not merely a transport bound. Keeping it in
/// the authoring contract prevents a canonical project from claiming evidence that no supported
/// archive reader can ever reopen.
pub const MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1: u64 = 16 * 1024 * 1024 * 1024;
/// Largest existing member identity that may be persisted as revision-3 Voice evidence.
pub const MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1: u64 = 256 * 1024 * 1024;

/// Why a LocalizationEntry ID cannot be used as one portable Voice member basename stem.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceLocIdBasenameStemErrorV1 {
    #[error("Voice LocID basename stem is empty")]
    Empty,
    #[error("Voice LocID basename stem has non-canonical surrounding whitespace")]
    NonCanonicalWhitespace,
    #[error("Voice LocID basename stem is not ASCII")]
    NonAscii,
    #[error("Voice LocID basename stem is {actual} bytes; maximum is {max}")]
    TooLong { actual: usize, max: usize },
    #[error("Voice LocID plus .ogg is not one portable archive-member basename")]
    UnsafeArchiveMember,
}

/// Validate one canonical Voice LocID as the stem of an exact `<LocID>.ogg` archive basename.
///
/// The closed model, target transaction, native inspector, and bundle layer must agree on this
/// portable identity. Validation is deliberately performed on the complete derived member name,
/// so separators, Windows device names, alternate data streams, and other unsafe spellings cannot
/// survive merely because the raw LocID itself is not a filesystem path yet.
pub fn validate_revision3_voice_loc_id_basename_stem_v1(
    value: &str,
) -> Result<(), Revision3VoiceLocIdBasenameStemErrorV1> {
    if value.is_empty() {
        return Err(Revision3VoiceLocIdBasenameStemErrorV1::Empty);
    }
    if value.trim() != value {
        return Err(Revision3VoiceLocIdBasenameStemErrorV1::NonCanonicalWhitespace);
    }
    if !value.is_ascii() {
        return Err(Revision3VoiceLocIdBasenameStemErrorV1::NonAscii);
    }
    if value.len() > MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1 {
        return Err(Revision3VoiceLocIdBasenameStemErrorV1::TooLong {
            actual: value.len(),
            max: MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1,
        });
    }
    // Validate the raw stem as well as the derived `<stem>.ogg` member. A trailing dot would be
    // hidden by the appended suffix but is still a non-portable Windows basename identity.
    if value.contains(['/', '\\']) || value.ends_with('.') {
        return Err(Revision3VoiceLocIdBasenameStemErrorV1::UnsafeArchiveMember);
    }
    let member = format!("{value}.ogg");
    gore_vo::validate_archive_entry_path(&member, &gore_vo::Limits::default())
        .map_err(|_| Revision3VoiceLocIdBasenameStemErrorV1::UnsafeArchiveMember)
}

/// Exact head/project/line/locale-bound intent carrying native archive match evidence.
///
/// `matches` is deliberately not a caller-selected resolution enum. Its cardinality is the only
/// authority for the persisted resolution state: 0 = unresolved, 1 = resolved, 2+ = ambiguous.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceTargetResolutionRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub line_id: EntityId,
    pub slot_id: EntityId,
    pub locale: LocaleCode,
    pub expected_loc_id: String,
    #[serde(default)]
    pub matches: Vec<VoiceTarget>,
}

impl Revision3VoiceTargetResolutionRequestV1 {
    pub fn from_json(json: &str) -> Result<Self, Revision3VoiceTargetResolutionRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3VoiceTargetResolutionRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3VoiceTargetResolutionRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3VoiceTargetResolutionRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3VoiceTargetResolutionRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3VoiceTargetResolutionRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3VoiceTargetResolutionRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3VoiceTargetResolutionRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3VoiceTargetResolutionRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTargetResolutionRequestJsonErrorV1 {
    #[error("revision-3 Voice target request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Voice target request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Voice target request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Voice target request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Voice target request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTargetResolutionConflictV1 {
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
    #[error("DialogLine and VoiceSlot IDs must be non-zero and different")]
    InvalidEntityIdentity,
    #[error("expected LocID is empty, non-canonical, or exceeds its byte limit")]
    InvalidExpectedLocId,
    #[error("DialogLine {line} is missing or has the wrong entity kind")]
    InvalidDialogLine { line: EntityId },
    #[error("DialogLine {line} has an invalid LocalizationEntry reference")]
    InvalidLocalizationReference { line: EntityId },
    #[error("expected LocID {expected:?}, but the exact line resolves to {actual:?}")]
    LocalizationIdentityMismatch { expected: String, actual: String },
    #[error("line/locale is not linked to requested VoiceSlot {slot}")]
    VoiceSlotIdentityMismatch { slot: EntityId },
    #[error("VoiceSlot {slot} is missing, has the wrong kind, locale, or unique owner")]
    InvalidVoiceSlot { slot: EntityId },
    #[error("VoiceSlot {slot} cannot increment its entity revision")]
    VoiceSlotRevisionOverflow { slot: EntityId },
    #[error("native Voice target evidence is not closed and bounded: {reason}")]
    InvalidNativeEvidence { reason: String },
    #[error("Voice target duplicates resolved slot {existing_slot}")]
    DuplicateResolvedTarget { existing_slot: EntityId },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTargetResolutionRejectionV1 {
    pub conflict: Revision3VoiceTargetResolutionConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTargetResolutionStateV1 {
    Unresolved,
    Ambiguous,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTargetResolutionOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub slot_id: EntityId,
    pub locale: LocaleCode,
    pub loc_id: String,
    pub resolution_state: Revision3VoiceTargetResolutionStateV1,
    pub match_count: u32,
    pub resolved_target: Option<VoiceTarget>,
    pub resolution: VoiceTargetResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTargetResolutionEvaluationV1 {
    Applied(Box<Revision3VoiceTargetResolutionOutcomeV1>),
    Rejected(Revision3VoiceTargetResolutionRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTargetResolutionErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Voice target request: {0}")]
    InvalidRequest(#[source] Revision3VoiceTargetResolutionRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Voice target candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Persist one native Voice target match result against an exact revision-3 slot.
///
/// This is a pure semantic transaction. It performs no filesystem access and does not claim that
/// a previously inspected archive is still installed or grant build/deployment authority.
pub fn apply_revision3_voice_target_resolution_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3VoiceTargetResolutionEvaluationV1, Revision3VoiceTargetResolutionErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3VoiceTargetResolutionErrorV1::InvalidProject)?;
    let request = Revision3VoiceTargetResolutionRequestV1::from_json(canonical_request_json)
        .map_err(Revision3VoiceTargetResolutionErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTargetResolutionEvaluationV1::Rejected(
                Revision3VoiceTargetResolutionRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3VoiceTargetResolutionConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3VoiceTargetResolutionConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3VoiceTargetResolutionConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.line_id)
        || is_zero_entity_id(request.slot_id)
        || request.line_id == request.slot_id
    {
        reject!(Revision3VoiceTargetResolutionConflictV1::InvalidEntityIdentity);
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        reject!(Revision3VoiceTargetResolutionConflictV1::InvalidExpectedLocId);
    }
    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::InvalidDialogLine {
                line: request.line_id,
            }
        );
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::InvalidDialogLine {
                line: request.line_id,
            }
        );
    };
    let Some((localization_id, loc_id)) = exact_localization(&project, line) else {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::InvalidLocalizationReference {
                line: request.line_id,
            }
        );
    };
    if loc_id != request.expected_loc_id {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::LocalizationIdentityMismatch {
                expected: request.expected_loc_id,
                actual: loc_id,
            }
        );
    }
    let Some(slot_ref) = line.voice_slots.get(&request.locale) else {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != request.slot_id
    {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    }
    if !has_unique_slot_owner(&project, request.line_id, &request.locale, request.slot_id) {
        reject!(Revision3VoiceTargetResolutionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }
    let Some(slot_entity) = project.entities.get(&request.slot_id) else {
        reject!(Revision3VoiceTargetResolutionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    let EntityPayload::VoiceSlot(VoiceSlot { locale, .. }) = &slot_entity.payload else {
        reject!(Revision3VoiceTargetResolutionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    if locale != &request.locale {
        reject!(Revision3VoiceTargetResolutionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }
    let Some(next_slot_revision) = slot_entity.revision.checked_add(1) else {
        reject!(
            Revision3VoiceTargetResolutionConflictV1::VoiceSlotRevisionOverflow {
                slot: request.slot_id,
            }
        );
    };

    let resolution = match resolution_from_native_matches(&request.matches) {
        Ok(value) => value,
        Err(reason) => {
            reject!(Revision3VoiceTargetResolutionConflictV1::InvalidNativeEvidence { reason })
        }
    };
    if let VoiceTargetResolution::Resolved { target } = &resolution {
        if let Some(existing_slot) = find_resolved_target_owner(&project, request.slot_id, target) {
            reject!(
                Revision3VoiceTargetResolutionConflictV1::DuplicateResolvedTarget { existing_slot }
            );
        }
    }

    let state = resolution_state(&resolution);
    let resolved_target = match &resolution {
        VoiceTargetResolution::Resolved { target } => Some(target.clone()),
        _ => None,
    };
    let match_count = request.matches.len() as u32;
    let Some(slot_entity) = project.entities.get_mut(&request.slot_id) else {
        unreachable!("slot was resolved above")
    };
    let EntityPayload::VoiceSlot(slot) = &mut slot_entity.payload else {
        unreachable!("slot kind was resolved above")
    };
    slot.target_resolution = resolution.clone();
    slot_entity.revision = next_slot_revision;
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(error) => {
            reject!(
                Revision3VoiceTargetResolutionConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3VoiceTargetResolutionErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3VoiceTargetResolutionErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3VoiceTargetResolutionEvaluationV1::Applied(
        Box::new(Revision3VoiceTargetResolutionOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            localization_id,
            slot_id: request.slot_id,
            locale: request.locale,
            loc_id: request.expected_loc_id,
            resolution_state: state,
            match_count,
            resolved_target,
            resolution,
        }),
    ))
}

pub(crate) fn validate_revision3_voice_target_v1(target: &VoiceTarget) -> Result<(), String> {
    if target.operation != VoiceOperation::Replace {
        return Err("only an existing-member Replace is supported".to_owned());
    }
    let VoiceMemberProof::Present {
        uncompressed_size, ..
    } = target.member_proof
    else {
        return Err("Replace requires native Present member proof".to_owned());
    };
    if uncompressed_size == 0
        || uncompressed_size > MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1
    {
        return Err(format!(
            "Present member proof uncompressed size is outside 1..={MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1}"
        ));
    }
    if target.archive_seal.byte_len == 0
        || target.archive_seal.byte_len > MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1
    {
        return Err(format!(
            "archive seal byte length is outside 1..={MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1}"
        ));
    }
    if is_zero_digest(target.archive_seal.sha256.as_bytes()) {
        return Err("archive seal is zero".to_owned());
    }
    if !valid_archive_name(&target.archive) {
        return Err("archive is not one bounded safe .zip filename".to_owned());
    }
    if !valid_member_path(&target.member) {
        return Err("member is not one bounded safe relative .ogg path".to_owned());
    }
    Ok(())
}

pub(crate) fn validate_revision3_voice_target_resolution_v1(
    resolution: &VoiceTargetResolution,
) -> Result<(), String> {
    match resolution {
        VoiceTargetResolution::Unresolved => Ok(()),
        VoiceTargetResolution::Resolved { target } => validate_revision3_voice_target_v1(target),
        VoiceTargetResolution::Ambiguous { candidates } => {
            if candidates.len() < 2 || candidates.len() > MAX_REVISION3_VOICE_TARGET_MATCHES_V1 {
                return Err(format!(
                    "ambiguous resolution has {} candidates; expected 2..={}",
                    candidates.len(),
                    MAX_REVISION3_VOICE_TARGET_MATCHES_V1
                ));
            }
            validate_unique_targets(candidates)
        }
    }
}

fn resolution_from_native_matches(
    matches: &[VoiceTarget],
) -> Result<VoiceTargetResolution, String> {
    if matches.len() > MAX_REVISION3_VOICE_TARGET_MATCHES_V1 {
        return Err(format!(
            "native match count {} exceeds {}",
            matches.len(),
            MAX_REVISION3_VOICE_TARGET_MATCHES_V1
        ));
    }
    validate_unique_targets(matches)?;
    Ok(match matches {
        [] => VoiceTargetResolution::Unresolved,
        [target] => VoiceTargetResolution::Resolved {
            target: target.clone(),
        },
        candidates => VoiceTargetResolution::Ambiguous {
            candidates: candidates.to_vec(),
        },
    })
}

fn validate_unique_targets(targets: &[VoiceTarget]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for target in targets {
        validate_revision3_voice_target_v1(target)?;
        if !seen.insert(revision3_voice_target_key_v1(target)) {
            return Err("native matches contain a duplicate archive/member target".to_owned());
        }
    }
    Ok(())
}

fn exact_localization(project: &ProjectRevision3, line: &DialogLine) -> Option<(EntityId, String)> {
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
    {
        return None;
    }
    let entity = project.entities.get(&line.localization.id)?;
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, .. }) = &entity.payload else {
        return None;
    };
    validate_revision3_voice_loc_id_basename_stem_v1(loc_id)
        .is_ok()
        .then(|| (line.localization.id, loc_id.clone()))
}

fn has_unique_slot_owner(
    project: &ProjectRevision3,
    expected_line: EntityId,
    expected_locale: &LocaleCode,
    slot_id: EntityId,
) -> bool {
    let mut owners = project.entities.iter().filter_map(|(line_id, entity)| {
        let EntityPayload::DialogLine(line) = &entity.payload else {
            return None;
        };
        line.voice_slots.iter().find_map(|(locale, reference)| {
            (reference.project_id == project.project_id
                && reference.expected_kind == EntityKind::VoiceSlot
                && reference.id == slot_id)
                .then_some((*line_id, locale))
        })
    });
    matches!(
        (owners.next(), owners.next()),
        (Some((line, locale)), None) if line == expected_line && locale == expected_locale
    )
}

fn find_resolved_target_owner(
    project: &ProjectRevision3,
    requested_slot: EntityId,
    requested_target: &VoiceTarget,
) -> Option<EntityId> {
    let requested_key = revision3_voice_target_key_v1(requested_target);
    project.entities.iter().find_map(|(id, entity)| {
        if *id == requested_slot {
            return None;
        }
        let EntityPayload::VoiceSlot(slot) = &entity.payload else {
            return None;
        };
        let VoiceTargetResolution::Resolved { target } = &slot.target_resolution else {
            return None;
        };
        (revision3_voice_target_key_v1(target) == requested_key).then_some(*id)
    })
}

fn resolution_state(resolution: &VoiceTargetResolution) -> Revision3VoiceTargetResolutionStateV1 {
    match resolution {
        VoiceTargetResolution::Unresolved => Revision3VoiceTargetResolutionStateV1::Unresolved,
        VoiceTargetResolution::Ambiguous { .. } => Revision3VoiceTargetResolutionStateV1::Ambiguous,
        VoiceTargetResolution::Resolved { .. } => Revision3VoiceTargetResolutionStateV1::Resolved,
    }
}

fn valid_archive_name(value: &str) -> bool {
    value.trim() == value
        && value.len() > 4
        && value.len() <= MAX_REVISION3_VOICE_TARGET_ARCHIVE_BYTES_V1
        && !value.contains(['/', '\\'])
        && !value.chars().any(char::is_control)
        && value.to_ascii_lowercase().ends_with(".zip")
        && gore_vo::validate_archive_entry_path(value, &gore_vo::Limits::default()).is_ok()
}

fn valid_member_path(value: &str) -> bool {
    value.trim() == value
        && value.len() > 4
        && value.len() <= MAX_REVISION3_VOICE_TARGET_MEMBER_BYTES_V1
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value.to_ascii_lowercase().ends_with(".ogg")
        && gore_vo::validate_archive_entry_path(value, &gore_vo::Limits::default()).is_ok()
}

fn is_zero_entity_id(value: EntityId) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
}

fn is_zero_digest(value: &[u8; 32]) -> bool {
    value.iter().all(|byte| *byte == 0)
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
            return Err(io::Error::other(
                "revision-3 Voice target request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
