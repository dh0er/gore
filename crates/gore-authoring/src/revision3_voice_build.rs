//! Deterministic, filesystem-free lowering plan for managed revision-3 Voice content.
//!
//! This module grants build-only authority for sealed replacements of existing archive members.
//! It neither reads Store blobs nor writes a bundle. Callers must obtain every referenced Ogg
//! through the verified Store boundary and pass the resulting bytes to a hardened bundle builder.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::model_revision3::{
    revision3_voice_target_key_v1, EntityKind, EntityPayload, OggCodec, ProjectRevision3,
    VoiceMemberProof, VoiceOperation, VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use crate::{AssetRef, EntityId, LocaleCode, ProjectId, ProjectMeta};

/// Largest managed Voice build that may be lowered into one bounded plan/report.
pub const MAX_REVISION3_VOICE_BUILD_SLOTS_V1: usize = 1024;
/// Largest exact DialogLine display name copied into one structured build blocker.
pub const MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1: usize = 256;
/// Largest aggregate selected Ogg payload lowered by one managed Voice build.
///
/// This counts every planned slot occurrence, including multiple slots that intentionally reuse
/// the same VoiceTake/asset. It matches the hardened bundle builder's retained-byte ceiling.
pub const MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3VoiceBuildBlockReasonV1 {
    NoVoiceSlots,
    VoiceSlotLimitExceeded,
    UnresolvedTarget,
    AmbiguousTarget,
    UnqualifiedAdd,
    MissingSelectedTake,
    SelectedTakeNotApproved,
    SelectedTakeCodecUnqualified,
    VoicePayloadBudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceBuildBlockerV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_id: Option<EntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_id: Option<EntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<LocaleCode>,
    pub reason: Revision3VoiceBuildBlockReasonV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceBuildBlockedV1 {
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub total_slots: u64,
    pub ready_slots: u64,
    pub blockers: Vec<Revision3VoiceBuildBlockerV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceBuildEditV1 {
    pub slot_id: EntityId,
    pub take_id: EntityId,
    pub locale: LocaleCode,
    pub asset: AssetRef,
    pub target: VoiceTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceBuildPlanV1 {
    pub schema_revision: u32,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub meta: ProjectMeta,
    pub edits: Vec<Revision3VoiceBuildEditV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Revision3VoiceBuildPlanEvaluationV1 {
    Ready {
        plan: Revision3VoiceBuildPlanV1,
    },
    Blocked {
        report: Revision3VoiceBuildBlockedV1,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceBuildPlanErrorV1 {
    #[error("invalid revision-3 project: {0}")]
    InvalidProject(String),
}

/// Plan every authored Voice slot as one exact existing-member replacement.
///
/// A build is all-or-nothing: unresolved/ambiguous targets and slots without one approved selected
/// take are returned as structured blockers instead of being silently omitted. Alternate unselected
/// takes remain authoring history and do not enter the plan.
pub fn plan_revision3_voice_build_v1(
    project: &ProjectRevision3,
) -> Result<Revision3VoiceBuildPlanEvaluationV1, Revision3VoiceBuildPlanErrorV1> {
    project
        .validate_closed_model()
        .map_err(|error| Revision3VoiceBuildPlanErrorV1::InvalidProject(error.to_string()))?;
    if !valid_bundle_mod_name(&project.meta.name) {
        return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(
            "project metadata name is not one safe bundle name".to_owned(),
        ));
    }

    let voice_slot_count = project
        .entities
        .values()
        .filter(|entity| matches!(&entity.payload, EntityPayload::VoiceSlot(_)))
        .count();
    let total_slots = u64::try_from(voice_slot_count).map_err(|_| {
        Revision3VoiceBuildPlanErrorV1::InvalidProject(
            "VoiceSlot count is not representable by the build report".to_owned(),
        )
    })?;
    if voice_slot_count > MAX_REVISION3_VOICE_BUILD_SLOTS_V1 {
        return Ok(Revision3VoiceBuildPlanEvaluationV1::Blocked {
            report: Revision3VoiceBuildBlockedV1 {
                project_id: project.project_id,
                project_revision: project.revision,
                total_slots,
                ready_slots: 0,
                blockers: vec![global_blocker(
                    Revision3VoiceBuildBlockReasonV1::VoiceSlotLimitExceeded,
                )],
            },
        });
    }

    let slot_owners = collect_slot_owner_facts(project)?;

    let mut ready_slots = 0u64;
    let mut blockers = Vec::new();
    let mut edits = Vec::new();
    let mut deployment_targets = BTreeSet::new();
    let mut selected_payload_bytes = 0u64;

    for (slot_id, entity) in &project.entities {
        let EntityPayload::VoiceSlot(slot) = &entity.payload else {
            continue;
        };
        let owner = slot_owners.get(slot_id).ok_or_else(|| {
            Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceSlot {slot_id} has no exact owning DialogLine facts"
            ))
        })?;

        let target = match &slot.target_resolution {
            VoiceTargetResolution::Unresolved => {
                blockers.push(slot_blocker(
                    *slot_id,
                    &slot.locale,
                    owner,
                    Revision3VoiceBuildBlockReasonV1::UnresolvedTarget,
                ));
                None
            }
            VoiceTargetResolution::Ambiguous { .. } => {
                blockers.push(slot_blocker(
                    *slot_id,
                    &slot.locale,
                    owner,
                    Revision3VoiceBuildBlockReasonV1::AmbiguousTarget,
                ));
                None
            }
            VoiceTargetResolution::Resolved { target }
                if target.operation != VoiceOperation::Replace
                    || !matches!(
                        &target.member_proof,
                        VoiceMemberProof::Present { uncompressed_size, .. }
                            if *uncompressed_size > 0
                    ) =>
            {
                blockers.push(slot_blocker(
                    *slot_id,
                    &slot.locale,
                    owner,
                    Revision3VoiceBuildBlockReasonV1::UnqualifiedAdd,
                ));
                None
            }
            VoiceTargetResolution::Resolved { target } => Some(target),
        };

        let selected = match &slot.selected {
            Some(selected) => Some(selected),
            None => {
                blockers.push(slot_blocker(
                    *slot_id,
                    &slot.locale,
                    owner,
                    Revision3VoiceBuildBlockReasonV1::MissingSelectedTake,
                ));
                None
            }
        };

        let (Some(target), Some(selected)) = (target, selected) else {
            continue;
        };
        if selected.project_id != project.project_id
            || selected.expected_kind != EntityKind::VoiceTake
        {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceSlot {slot_id} has an invalid selected-take reference"
            )));
        }
        let Some(selected_entity) = project.entities.get(&selected.id) else {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceSlot {slot_id} selected take {} is missing",
                selected.id
            )));
        };
        let EntityPayload::VoiceTake(take) = &selected_entity.payload else {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceSlot {slot_id} selected entity {} is not a VoiceTake",
                selected.id
            )));
        };
        if take.status != VoiceTakeStatus::Approved {
            blockers.push(slot_blocker(
                *slot_id,
                &slot.locale,
                owner,
                Revision3VoiceBuildBlockReasonV1::SelectedTakeNotApproved,
            ));
            continue;
        }
        if take.ogg.codec != OggCodec::Vorbis {
            blockers.push(slot_blocker(
                *slot_id,
                &slot.locale,
                owner,
                Revision3VoiceBuildBlockReasonV1::SelectedTakeCodecUnqualified,
            ));
            continue;
        }
        if take.locale != slot.locale {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceSlot {slot_id} locale differs from selected take {}",
                selected.id
            )));
        }
        let Some(meta) = project.asset_store.assets.get(&take.asset.sha256) else {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceTake {} asset is absent from the Store index",
                selected.id
            )));
        };
        if meta.byte_len != take.asset.byte_len || meta.media_type != "audio/ogg" {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceTake {} asset metadata is not exact audio/ogg",
                selected.id
            )));
        }
        let deployment_key = revision3_voice_target_key_v1(target);
        if !deployment_targets.insert(deployment_key) {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "VoiceSlot {slot_id} duplicates another deployment target"
            )));
        }
        if !add_selected_payload_occurrence(&mut selected_payload_bytes, take.asset.byte_len) {
            return Ok(Revision3VoiceBuildPlanEvaluationV1::Blocked {
                report: Revision3VoiceBuildBlockedV1 {
                    project_id: project.project_id,
                    project_revision: project.revision,
                    total_slots,
                    ready_slots: 0,
                    blockers: vec![global_blocker(
                        Revision3VoiceBuildBlockReasonV1::VoicePayloadBudgetExceeded,
                    )],
                },
            });
        }

        ready_slots += 1;
        edits.push(Revision3VoiceBuildEditV1 {
            slot_id: *slot_id,
            take_id: selected.id,
            locale: slot.locale.clone(),
            asset: take.asset.clone(),
            target: target.clone(),
        });
    }

    if total_slots == 0 {
        blockers.push(global_blocker(
            Revision3VoiceBuildBlockReasonV1::NoVoiceSlots,
        ));
    }
    if !blockers.is_empty() {
        return Ok(Revision3VoiceBuildPlanEvaluationV1::Blocked {
            report: Revision3VoiceBuildBlockedV1 {
                project_id: project.project_id,
                project_revision: project.revision,
                total_slots,
                ready_slots,
                blockers,
            },
        });
    }

    Ok(Revision3VoiceBuildPlanEvaluationV1::Ready {
        plan: Revision3VoiceBuildPlanV1 {
            schema_revision: 1,
            project_id: project.project_id,
            project_revision: project.revision,
            meta: project.meta.clone(),
            edits,
        },
    })
}

/// Checked occurrence accounting. `false` includes both arithmetic overflow and a finite total
/// above the public retained-payload contract.
fn add_selected_payload_occurrence(total: &mut u64, occurrence_bytes: u64) -> bool {
    let Some(next) = total.checked_add(occurrence_bytes) else {
        return false;
    };
    if next > MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1 {
        return false;
    }
    *total = next;
    true
}

fn valid_bundle_mod_name(value: &str) -> bool {
    !value.contains('/')
        && !value.contains('\\')
        && gore_vo::validate_archive_entry_path(value, &gore_vo::Limits::default()).is_ok()
}

/// Wire-safe presentation label shared with the Mod Studio parser.
///
/// Keep this predicate explicit instead of using language-specific `trim` or Unicode-control
/// helpers: Rust and Dart do not promise the same whitespace tables across runtime versions.
fn valid_voice_build_line_label(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1
        || value.chars().any(voice_build_line_label_control)
    {
        return false;
    }
    let mut characters = value.chars();
    let first = characters.next().expect("non-empty label has a first char");
    let last = characters.next_back().unwrap_or(first);
    !voice_build_line_label_boundary_whitespace(first)
        && !voice_build_line_label_boundary_whitespace(last)
}

fn voice_build_line_label_control(character: char) -> bool {
    matches!(character, '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}')
}

/// Unicode White_Space plus the legacy zero-width no-break-space/BOM recognized by common
/// string trim implementations. The identical code-point set is enumerated in Dart.
fn voice_build_line_label_boundary_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VoiceSlotOwnerFactsV1 {
    line_id: EntityId,
    line_label: String,
    loc_id: String,
}

fn collect_slot_owner_facts(
    project: &ProjectRevision3,
) -> Result<BTreeMap<EntityId, VoiceSlotOwnerFactsV1>, Revision3VoiceBuildPlanErrorV1> {
    let mut owners = BTreeMap::new();
    for (line_id, line_entity) in &project.entities {
        let EntityPayload::DialogLine(line) = &line_entity.payload else {
            continue;
        };
        if line.voice_slots.is_empty() {
            continue;
        }
        if !valid_voice_build_line_label(&line_entity.display_name) {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "DialogLine {line_id} display name is not one canonical build label ({} UTF-8 bytes; maximum {})",
                line_entity.display_name.len(),
                MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1
            )));
        }
        if line.localization.project_id != project.project_id
            || line.localization.expected_kind != EntityKind::LocalizationEntry
        {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "DialogLine {line_id} has no exact LocalizationEntry reference"
            )));
        }
        let localization_entity = project.entities.get(&line.localization.id).ok_or_else(|| {
            Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "DialogLine {line_id} LocalizationEntry is missing"
            ))
        })?;
        let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
            return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                "DialogLine {line_id} localization reference has the wrong entity kind"
            )));
        };

        for (locale, slot_ref) in &line.voice_slots {
            if slot_ref.project_id != project.project_id
                || slot_ref.expected_kind != EntityKind::VoiceSlot
            {
                return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                    "DialogLine {line_id}/{locale} has no exact VoiceSlot reference"
                )));
            }
            let slot_entity = project.entities.get(&slot_ref.id).ok_or_else(|| {
                Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                    "DialogLine {line_id}/{locale} VoiceSlot is missing"
                ))
            })?;
            let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
                return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                    "DialogLine {line_id}/{locale} slot reference has the wrong entity kind"
                )));
            };
            if &slot.locale != locale {
                return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                    "DialogLine {line_id}/{locale} owns a VoiceSlot for {}",
                    slot.locale
                )));
            }
            let facts = VoiceSlotOwnerFactsV1 {
                line_id: *line_id,
                line_label: line_entity.display_name.clone(),
                loc_id: localization.loc_id.clone(),
            };
            if owners.insert(slot_ref.id, facts).is_some() {
                return Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(format!(
                    "VoiceSlot {} has more than one DialogLine owner",
                    slot_ref.id
                )));
            }
        }
    }
    Ok(owners)
}

fn global_blocker(reason: Revision3VoiceBuildBlockReasonV1) -> Revision3VoiceBuildBlockerV1 {
    Revision3VoiceBuildBlockerV1 {
        slot_id: None,
        line_id: None,
        line_label: None,
        loc_id: None,
        locale: None,
        reason,
    }
}

fn slot_blocker(
    slot_id: EntityId,
    locale: &LocaleCode,
    owner: &VoiceSlotOwnerFactsV1,
    reason: Revision3VoiceBuildBlockReasonV1,
) -> Revision3VoiceBuildBlockerV1 {
    Revision3VoiceBuildBlockerV1 {
        slot_id: Some(slot_id),
        line_id: Some(owner.line_id),
        line_label: Some(owner.line_label.clone()),
        loc_id: Some(owner.loc_id.clone()),
        locale: Some(locale.clone()),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        add_selected_payload_occurrence, MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1,
    };

    #[test]
    fn selected_payload_occurrence_accounting_is_checked_before_comparison() {
        let mut boundary = MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1 - 1;
        assert!(add_selected_payload_occurrence(&mut boundary, 1));
        assert_eq!(
            boundary,
            MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1
        );
        assert!(!add_selected_payload_occurrence(&mut boundary, 1));

        let mut overflowing = u64::MAX;
        assert!(!add_selected_payload_occurrence(&mut overflowing, 1));
        assert_eq!(overflowing, u64::MAX);
    }
}
