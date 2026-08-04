//! Filesystem-free, aggregate build-readiness planning for one managed revision-3 project.
//!
//! This planner classifies semantic output coverage only. It emits no entity identifiers, game
//! targets, paths, generated source, artifact references, deployment instructions, or runtime
//! claim, and grants no build authority.

use std::collections::{BTreeMap, BTreeSet};

use gore_asset::{ReviewedDataAssetErrorV1, ReviewedDataAssetStageBlockReasonV1};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use crate::model_revision3::{EntityPayload, OriginRef, ProjectRevision3};
use crate::{
    plan_revision3_voice_build_v1, verify_reviewed_fixed_leaf_stage_v1, ContentSeal, ProjectId,
    Revision3DataAssetStageViewV1, Revision3VoiceBuildBlockReasonV1,
    Revision3VoiceBuildPlanEvaluationV1, Sha256Digest,
    DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1, MAX_DATAASSET_FIXED_LEAF_STAGES_V1,
};

pub const REVISION3_PROJECT_BUILD_PLAN_SCHEMA_V1: u32 = 1;
pub const MAX_REVISION3_PROJECT_BUILD_BLOCKER_GROUPS_V1: usize = 64;

const INPUT_SEAL_FORMAT_V1: &str = "gore.authoring.revision3-project-build-input.v1";
const PLAN_SEAL_FORMAT_V1: &str = "gore.authoring.revision3-project-build-plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildOutcomeV1 {
    Empty,
    Blocked,
    CoverageComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildDomainV1 {
    Localization,
    Dialog,
    Voice,
    Npc,
    Quest,
    Scripts,
    Items,
    DataAssets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildDomainStatusV1 {
    NotPresent,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildBlockerCategoryV1 {
    AuthorProject,
    ToolkitSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildBlockReasonV1 {
    LocalizationLoweringUnavailable,
    DialogLoweringUnavailable,
    VoiceProjectNameUnsupported,
    VoiceLineLabelUnsupported,
    VoiceSlotLimitExceeded,
    VoiceTargetUnresolved,
    VoiceTargetAmbiguous,
    VoiceAddUnqualified,
    VoiceSelectedTakeMissing,
    VoiceSelectedTakeNotApproved,
    VoiceSelectedTakeCodecUnqualified,
    VoicePayloadBudgetExceeded,
    NpcLoweringUnavailable,
    QuestLoweringUnavailable,
    ScriptLoweringUnavailable,
    ItemPatchLoweringUnavailable,
    DataAssetTargetUnsupported,
    DataAssetSelectorMismatch,
    DataAssetReplacementMalformed,
    DataAssetReplacementNonFinite,
    DataAssetReplacementNonPositive,
    DataAssetPreservedComponentChanged,
    DataAssetReviewedPreparationFailed,
    DataAssetDerivedReplacementMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ProjectBuildDomainSummaryV1 {
    pub domain: Revision3ProjectBuildDomainV1,
    pub status: Revision3ProjectBuildDomainStatusV1,
    pub content_count: u64,
    pub ready_count: u64,
    pub blocked_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ProjectBuildBlockerV1 {
    pub category: Revision3ProjectBuildBlockerCategoryV1,
    pub domain: Revision3ProjectBuildDomainV1,
    pub reason: Revision3ProjectBuildBlockReasonV1,
    /// Number of content records affected by this aggregate reason. Distinct reasons can overlap.
    pub affected_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildPlanScopeV1 {
    ProjectBuildReadinessOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildArtifactStatusV1 {
    NotCreated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildDeploymentStatusV1 {
    NotPerformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ProjectBuildPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ProjectBuildPlanV1 {
    pub schema_revision: u32,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub outcome: Revision3ProjectBuildOutcomeV1,
    pub production_content_count: u64,
    pub input_seal: ContentSeal,
    pub plan_seal: ContentSeal,
    pub domains: Vec<Revision3ProjectBuildDomainSummaryV1>,
    pub blockers: Vec<Revision3ProjectBuildBlockerV1>,
    pub scope: Revision3ProjectBuildPlanScopeV1,
    pub build_authority: Revision3ProjectBuildAuthorityV1,
    pub artifact_status: Revision3ProjectBuildArtifactStatusV1,
    pub deployment_status: Revision3ProjectBuildDeploymentStatusV1,
    pub runtime_status: Revision3ProjectBuildRuntimeStatusV1,
    pub publication_status: Revision3ProjectBuildPublicationStatusV1,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3ProjectBuildPlanErrorV1 {
    #[error("invalid revision-3 project: {0}")]
    InvalidProject(String),
    #[error("revision-3 project could not be serialized canonically: {0}")]
    ProjectSerialization(String),
    #[error("DataAsset stage count {actual} exceeds the planner limit {limit}")]
    DataAssetStageLimitExceeded { actual: usize, limit: usize },
    #[error("DataAsset stage views are not the exact complete project stage set")]
    DataAssetStageSetMismatch,
    #[error("a DataAsset stage view is not bound to the exact project")]
    InvalidDataAssetStageBinding,
    #[error("the DataAsset stage view set contains a duplicate")]
    DuplicateDataAssetStage,
    #[error("project build count overflow")]
    CountOverflow,
    #[error("project build planner invariant failed: {0}")]
    InvariantViolation(&'static str),
    #[error("project build seal serialization failed: {0}")]
    SealSerialization(#[source] serde_json::Error),
    #[error("project build blocker aggregation exceeds {limit} groups: {actual}")]
    TooManyBlockerGroups { actual: usize, limit: usize },
}

type BlockerKey = (
    Revision3ProjectBuildBlockerCategoryV1,
    Revision3ProjectBuildDomainV1,
    Revision3ProjectBuildBlockReasonV1,
);

#[derive(Default)]
struct EntityDomainCounts {
    localization: u64,
    dialog: u64,
    voice: u64,
    npc: u64,
    quest: u64,
    scripts: u64,
    items: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct InputSealProjectionV1<'a> {
    format: &'static str,
    project: &'a ContentSeal,
    dataasset_stage_manifests: &'a [ContentSeal],
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct PlanSealProjectionV1<'a> {
    format: &'static str,
    schema_revision: u32,
    project_id: ProjectId,
    project_revision: u64,
    outcome: Revision3ProjectBuildOutcomeV1,
    production_content_count: u64,
    input_seal: &'a ContentSeal,
    domains: &'a [Revision3ProjectBuildDomainSummaryV1],
    blockers: &'a [Revision3ProjectBuildBlockerV1],
    scope: Revision3ProjectBuildPlanScopeV1,
    build_authority: Revision3ProjectBuildAuthorityV1,
    artifact_status: Revision3ProjectBuildArtifactStatusV1,
    deployment_status: Revision3ProjectBuildDeploymentStatusV1,
    runtime_status: Revision3ProjectBuildRuntimeStatusV1,
    publication_status: Revision3ProjectBuildPublicationStatusV1,
}

/// Classify the exact complete revision-3 project into bounded aggregate production coverage.
///
/// `dataasset_stages` must be the exact fully verified stage set represented by the project's
/// Store index. The result is deterministic and filesystem-free and grants no build authority.
pub fn plan_revision3_project_build_v1(
    project: &ProjectRevision3,
    dataasset_stages: &[Revision3DataAssetStageViewV1],
) -> Result<Revision3ProjectBuildPlanV1, Revision3ProjectBuildPlanErrorV1> {
    project
        .validate_closed_model()
        .map_err(|error| Revision3ProjectBuildPlanErrorV1::InvalidProject(error.to_string()))?;
    let project_json = project.to_canonical_json().map_err(|error| {
        Revision3ProjectBuildPlanErrorV1::ProjectSerialization(error.to_string())
    })?;
    let project_seal = seal_bytes(project_json.as_bytes());
    let stage_manifest_seals = validate_exact_stage_set(project, dataasset_stages)?;
    let input_seal = seal_serializable(&InputSealProjectionV1 {
        format: INPUT_SEAL_FORMAT_V1,
        project: &project_seal,
        dataasset_stage_manifests: &stage_manifest_seals,
    })?;

    let counts = collect_entity_domain_counts(project)?;
    let dataasset_count = usize_to_u64(dataasset_stages.len())?;
    let mut aggregate = BTreeMap::<BlockerKey, u64>::new();
    let mut domains = Vec::with_capacity(8);

    domains.push(all_blocked_domain(
        Revision3ProjectBuildDomainV1::Localization,
        counts.localization,
        Revision3ProjectBuildBlockReasonV1::LocalizationLoweringUnavailable,
        &mut aggregate,
    )?);
    domains.push(all_blocked_domain(
        Revision3ProjectBuildDomainV1::Dialog,
        counts.dialog,
        Revision3ProjectBuildBlockReasonV1::DialogLoweringUnavailable,
        &mut aggregate,
    )?);
    domains.push(plan_voice_domain(project, counts.voice, &mut aggregate)?);
    domains.push(all_blocked_domain(
        Revision3ProjectBuildDomainV1::Npc,
        counts.npc,
        Revision3ProjectBuildBlockReasonV1::NpcLoweringUnavailable,
        &mut aggregate,
    )?);
    domains.push(all_blocked_domain(
        Revision3ProjectBuildDomainV1::Quest,
        counts.quest,
        Revision3ProjectBuildBlockReasonV1::QuestLoweringUnavailable,
        &mut aggregate,
    )?);
    domains.push(all_blocked_domain(
        Revision3ProjectBuildDomainV1::Scripts,
        counts.scripts,
        Revision3ProjectBuildBlockReasonV1::ScriptLoweringUnavailable,
        &mut aggregate,
    )?);
    domains.push(all_blocked_domain(
        Revision3ProjectBuildDomainV1::Items,
        counts.items,
        Revision3ProjectBuildBlockReasonV1::ItemPatchLoweringUnavailable,
        &mut aggregate,
    )?);
    domains.push(plan_dataasset_domain(
        dataasset_stages,
        dataasset_count,
        &mut aggregate,
    )?);

    let production_content_count = production_content_count(&counts, dataasset_count)?;
    if aggregate.len() > MAX_REVISION3_PROJECT_BUILD_BLOCKER_GROUPS_V1 {
        return Err(Revision3ProjectBuildPlanErrorV1::TooManyBlockerGroups {
            actual: aggregate.len(),
            limit: MAX_REVISION3_PROJECT_BUILD_BLOCKER_GROUPS_V1,
        });
    }
    let blockers = aggregate
        .into_iter()
        .map(
            |((category, domain, reason), affected_count)| Revision3ProjectBuildBlockerV1 {
                category,
                domain,
                reason,
                affected_count,
            },
        )
        .collect::<Vec<_>>();
    let outcome = if production_content_count == 0 {
        Revision3ProjectBuildOutcomeV1::Empty
    } else if blockers.is_empty() {
        Revision3ProjectBuildOutcomeV1::CoverageComplete
    } else {
        Revision3ProjectBuildOutcomeV1::Blocked
    };

    let scope = Revision3ProjectBuildPlanScopeV1::ProjectBuildReadinessOnly;
    let build_authority = Revision3ProjectBuildAuthorityV1::NotGranted;
    let artifact_status = Revision3ProjectBuildArtifactStatusV1::NotCreated;
    let deployment_status = Revision3ProjectBuildDeploymentStatusV1::NotPerformed;
    let runtime_status = Revision3ProjectBuildRuntimeStatusV1::RuntimeUnqualified;
    let publication_status = Revision3ProjectBuildPublicationStatusV1::NotSupported;
    let plan_seal = seal_serializable(&PlanSealProjectionV1 {
        format: PLAN_SEAL_FORMAT_V1,
        schema_revision: REVISION3_PROJECT_BUILD_PLAN_SCHEMA_V1,
        project_id: project.project_id,
        project_revision: project.revision,
        outcome,
        production_content_count,
        input_seal: &input_seal,
        domains: &domains,
        blockers: &blockers,
        scope,
        build_authority,
        artifact_status,
        deployment_status,
        runtime_status,
        publication_status,
    })?;

    Ok(Revision3ProjectBuildPlanV1 {
        schema_revision: REVISION3_PROJECT_BUILD_PLAN_SCHEMA_V1,
        project_id: project.project_id,
        project_revision: project.revision,
        outcome,
        production_content_count,
        input_seal,
        plan_seal,
        domains,
        blockers,
        scope,
        build_authority,
        artifact_status,
        deployment_status,
        runtime_status,
        publication_status,
    })
}

fn collect_entity_domain_counts(
    project: &ProjectRevision3,
) -> Result<EntityDomainCounts, Revision3ProjectBuildPlanErrorV1> {
    let mut counts = EntityDomainCounts::default();
    for entity in project.entities.values() {
        let counter = match &entity.payload {
            EntityPayload::LocalizationEntry(_) if authored_output_origin(&entity.origin) => {
                Some(&mut counts.localization)
            }
            EntityPayload::DialogLine(_) if authored_output_origin(&entity.origin) => {
                Some(&mut counts.dialog)
            }
            EntityPayload::VoiceSlot(_) => Some(&mut counts.voice),
            EntityPayload::NpcDraft(_) => Some(&mut counts.npc),
            EntityPayload::QuestDraft(_) => Some(&mut counts.quest),
            EntityPayload::ScriptModule(_) => Some(&mut counts.scripts),
            EntityPayload::ItemPatch(_) => Some(&mut counts.items),
            EntityPayload::LocalizationEntry(_)
            | EntityPayload::DialogLine(_)
            | EntityPayload::VoiceTake(_) => None,
        };
        if let Some(counter) = counter {
            *counter = counter
                .checked_add(1)
                .ok_or(Revision3ProjectBuildPlanErrorV1::CountOverflow)?;
        }
    }
    Ok(counts)
}

fn authored_output_origin(origin: &OriginRef) -> bool {
    matches!(origin, OriginRef::New { .. } | OriginRef::Generated { .. })
}

fn production_content_count(
    counts: &EntityDomainCounts,
    dataasset_count: u64,
) -> Result<u64, Revision3ProjectBuildPlanErrorV1> {
    // ScriptModule is deterministic backing data owned by an NPC/Quest output root. Its domain
    // remains visible for coverage, but it must not double-count the owning production content.
    [
        counts.localization,
        counts.dialog,
        counts.voice,
        counts.npc,
        counts.quest,
        counts.items,
        dataasset_count,
    ]
    .into_iter()
    .try_fold(0u64, |total, count| {
        total
            .checked_add(count)
            .ok_or(Revision3ProjectBuildPlanErrorV1::CountOverflow)
    })
}

fn all_blocked_domain(
    domain: Revision3ProjectBuildDomainV1,
    count: u64,
    reason: Revision3ProjectBuildBlockReasonV1,
    aggregate: &mut BTreeMap<BlockerKey, u64>,
) -> Result<Revision3ProjectBuildDomainSummaryV1, Revision3ProjectBuildPlanErrorV1> {
    if count > 0 {
        add_blocker(
            aggregate,
            Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport,
            domain,
            reason,
            count,
        )?;
    }
    domain_summary(domain, count, 0, count)
}

fn plan_voice_domain(
    project: &ProjectRevision3,
    count: u64,
    aggregate: &mut BTreeMap<BlockerKey, u64>,
) -> Result<Revision3ProjectBuildDomainSummaryV1, Revision3ProjectBuildPlanErrorV1> {
    if count == 0 {
        return domain_summary(Revision3ProjectBuildDomainV1::Voice, 0, 0, 0);
    }
    match plan_revision3_voice_build_v1(project) {
        Ok(Revision3VoiceBuildPlanEvaluationV1::Ready { plan }) => {
            let ready = usize_to_u64(plan.edits.len())?;
            if ready != count {
                return Err(Revision3ProjectBuildPlanErrorV1::InvariantViolation(
                    "Voice ready-plan count differs from VoiceSlot count",
                ));
            }
            domain_summary(Revision3ProjectBuildDomainV1::Voice, count, ready, 0)
        }
        Ok(Revision3VoiceBuildPlanEvaluationV1::Blocked { report }) => {
            if report.project_id != project.project_id
                || report.project_revision != project.revision
                || report.total_slots != count
                || report.ready_slots > count
            {
                return Err(Revision3ProjectBuildPlanErrorV1::InvariantViolation(
                    "Voice blocked report is not exact-project",
                ));
            }
            for blocker in report.blockers {
                let Some((category, reason)) = map_voice_reason(blocker.reason) else {
                    continue;
                };
                let affected = if blocker.slot_id.is_some() { 1 } else { count };
                add_blocker(
                    aggregate,
                    category,
                    Revision3ProjectBuildDomainV1::Voice,
                    reason,
                    affected,
                )?;
            }
            let blocked = count
                .checked_sub(report.ready_slots)
                .ok_or(Revision3ProjectBuildPlanErrorV1::CountOverflow)?;
            if blocked > 0
                && !aggregate
                    .keys()
                    .any(|(_, domain, _)| *domain == Revision3ProjectBuildDomainV1::Voice)
            {
                return Err(Revision3ProjectBuildPlanErrorV1::InvariantViolation(
                    "Voice blocked report contains no classified blocker",
                ));
            }
            domain_summary(
                Revision3ProjectBuildDomainV1::Voice,
                count,
                report.ready_slots,
                blocked,
            )
        }
        Err(crate::Revision3VoiceBuildPlanErrorV1::ProjectNameUnsupported) => {
            add_blocker(
                aggregate,
                Revision3ProjectBuildBlockerCategoryV1::AuthorProject,
                Revision3ProjectBuildDomainV1::Voice,
                Revision3ProjectBuildBlockReasonV1::VoiceProjectNameUnsupported,
                count,
            )?;
            domain_summary(Revision3ProjectBuildDomainV1::Voice, count, 0, count)
        }
        Err(crate::Revision3VoiceBuildPlanErrorV1::LineLabelUnsupported { .. }) => {
            add_blocker(
                aggregate,
                Revision3ProjectBuildBlockerCategoryV1::AuthorProject,
                Revision3ProjectBuildDomainV1::Voice,
                Revision3ProjectBuildBlockReasonV1::VoiceLineLabelUnsupported,
                count,
            )?;
            domain_summary(Revision3ProjectBuildDomainV1::Voice, count, 0, count)
        }
        Err(crate::Revision3VoiceBuildPlanErrorV1::InvalidProject(_)) => {
            Err(Revision3ProjectBuildPlanErrorV1::InvariantViolation(
                "Voice planner rejected a project after closed-model validation",
            ))
        }
    }
}

fn map_voice_reason(
    reason: Revision3VoiceBuildBlockReasonV1,
) -> Option<(
    Revision3ProjectBuildBlockerCategoryV1,
    Revision3ProjectBuildBlockReasonV1,
)> {
    let category = if reason == Revision3VoiceBuildBlockReasonV1::UnqualifiedAdd {
        Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport
    } else {
        Revision3ProjectBuildBlockerCategoryV1::AuthorProject
    };
    let reason = match reason {
        Revision3VoiceBuildBlockReasonV1::NoVoiceSlots => return None,
        Revision3VoiceBuildBlockReasonV1::VoiceSlotLimitExceeded => {
            Revision3ProjectBuildBlockReasonV1::VoiceSlotLimitExceeded
        }
        Revision3VoiceBuildBlockReasonV1::UnresolvedTarget => {
            Revision3ProjectBuildBlockReasonV1::VoiceTargetUnresolved
        }
        Revision3VoiceBuildBlockReasonV1::AmbiguousTarget => {
            Revision3ProjectBuildBlockReasonV1::VoiceTargetAmbiguous
        }
        Revision3VoiceBuildBlockReasonV1::UnqualifiedAdd => {
            Revision3ProjectBuildBlockReasonV1::VoiceAddUnqualified
        }
        Revision3VoiceBuildBlockReasonV1::MissingSelectedTake => {
            Revision3ProjectBuildBlockReasonV1::VoiceSelectedTakeMissing
        }
        Revision3VoiceBuildBlockReasonV1::SelectedTakeNotApproved => {
            Revision3ProjectBuildBlockReasonV1::VoiceSelectedTakeNotApproved
        }
        Revision3VoiceBuildBlockReasonV1::SelectedTakeCodecUnqualified => {
            Revision3ProjectBuildBlockReasonV1::VoiceSelectedTakeCodecUnqualified
        }
        Revision3VoiceBuildBlockReasonV1::VoicePayloadBudgetExceeded => {
            Revision3ProjectBuildBlockReasonV1::VoicePayloadBudgetExceeded
        }
    };
    Some((category, reason))
}

fn plan_dataasset_domain(
    stages: &[Revision3DataAssetStageViewV1],
    count: u64,
    aggregate: &mut BTreeMap<BlockerKey, u64>,
) -> Result<Revision3ProjectBuildDomainSummaryV1, Revision3ProjectBuildPlanErrorV1> {
    let mut ready = 0u64;
    for stage in stages {
        match verify_reviewed_fixed_leaf_stage_v1(stage.clone()) {
            Ok(_) => {
                ready = ready
                    .checked_add(1)
                    .ok_or(Revision3ProjectBuildPlanErrorV1::CountOverflow)?;
            }
            Err(reason) => {
                let (category, reason) = map_dataasset_reason(reason);
                add_blocker(
                    aggregate,
                    category,
                    Revision3ProjectBuildDomainV1::DataAssets,
                    reason,
                    1,
                )?;
            }
        }
    }
    let blocked = count
        .checked_sub(ready)
        .ok_or(Revision3ProjectBuildPlanErrorV1::CountOverflow)?;
    domain_summary(
        Revision3ProjectBuildDomainV1::DataAssets,
        count,
        ready,
        blocked,
    )
}

fn map_dataasset_reason(
    reason: ReviewedDataAssetStageBlockReasonV1,
) -> (
    Revision3ProjectBuildBlockerCategoryV1,
    Revision3ProjectBuildBlockReasonV1,
) {
    use Revision3ProjectBuildBlockReasonV1 as Output;
    use Revision3ProjectBuildBlockerCategoryV1::{AuthorProject, ToolkitSupport};
    match reason {
        ReviewedDataAssetStageBlockReasonV1::UnsupportedTarget => {
            (ToolkitSupport, Output::DataAssetTargetUnsupported)
        }
        ReviewedDataAssetStageBlockReasonV1::SelectorMismatch { .. } => {
            (AuthorProject, Output::DataAssetSelectorMismatch)
        }
        ReviewedDataAssetStageBlockReasonV1::MalformedReplacement => {
            (AuthorProject, Output::DataAssetReplacementMalformed)
        }
        ReviewedDataAssetStageBlockReasonV1::NonFiniteReplacementComponent { .. } => {
            (AuthorProject, Output::DataAssetReplacementNonFinite)
        }
        ReviewedDataAssetStageBlockReasonV1::NonPositiveReplacementComponent { .. } => {
            (AuthorProject, Output::DataAssetReplacementNonPositive)
        }
        ReviewedDataAssetStageBlockReasonV1::PreservedComponentChanged { .. } => {
            (AuthorProject, Output::DataAssetPreservedComponentChanged)
        }
        ReviewedDataAssetStageBlockReasonV1::ReviewedPreparation(
            ReviewedDataAssetErrorV1::UnknownTargetPath,
        ) => (ToolkitSupport, Output::DataAssetTargetUnsupported),
        ReviewedDataAssetStageBlockReasonV1::ReviewedPreparation(_) => {
            (AuthorProject, Output::DataAssetReviewedPreparationFailed)
        }
        ReviewedDataAssetStageBlockReasonV1::DerivedReplacementMismatch => {
            (AuthorProject, Output::DataAssetDerivedReplacementMismatch)
        }
    }
}

fn validate_exact_stage_set(
    project: &ProjectRevision3,
    stages: &[Revision3DataAssetStageViewV1],
) -> Result<Vec<ContentSeal>, Revision3ProjectBuildPlanErrorV1> {
    let expected = project
        .asset_store
        .assets
        .iter()
        .filter_map(|(digest, meta)| {
            (meta.media_type == DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1)
                .then_some((*digest, meta.byte_len))
        })
        .collect::<BTreeMap<_, _>>();
    if expected.len() > MAX_DATAASSET_FIXED_LEAF_STAGES_V1
        || stages.len() > MAX_DATAASSET_FIXED_LEAF_STAGES_V1
    {
        return Err(
            Revision3ProjectBuildPlanErrorV1::DataAssetStageLimitExceeded {
                actual: expected.len().max(stages.len()),
                limit: MAX_DATAASSET_FIXED_LEAF_STAGES_V1,
            },
        );
    }

    let mut seen_assets = BTreeSet::new();
    let mut seen_targets = BTreeSet::new();
    for stage in stages {
        let seal = stage.manifest_asset();
        if !seen_assets.insert(seal.sha256)
            || !seen_targets.insert(stage.target_path().to_ascii_lowercase())
        {
            return Err(Revision3ProjectBuildPlanErrorV1::DuplicateDataAssetStage);
        }
        let manifest = stage.manifest();
        let manifest_json = manifest
            .to_canonical_json()
            .map_err(|_| Revision3ProjectBuildPlanErrorV1::InvalidDataAssetStageBinding)?;
        if expected.get(&seal.sha256) != Some(&seal.byte_len)
            || seal_bytes(manifest_json.as_bytes()) != *seal
            || manifest.project_id() != project.project_id
            || manifest.project_target() != &project.target
            || manifest.staged_project_revision() > project.revision
        {
            return Err(Revision3ProjectBuildPlanErrorV1::InvalidDataAssetStageBinding);
        }
    }
    if seen_assets.len() != expected.len()
        || expected.keys().any(|digest| !seen_assets.contains(digest))
    {
        return Err(Revision3ProjectBuildPlanErrorV1::DataAssetStageSetMismatch);
    }
    Ok(expected
        .into_iter()
        .map(|(sha256, byte_len)| ContentSeal { byte_len, sha256 })
        .collect())
}

fn domain_summary(
    domain: Revision3ProjectBuildDomainV1,
    content_count: u64,
    ready_count: u64,
    blocked_count: u64,
) -> Result<Revision3ProjectBuildDomainSummaryV1, Revision3ProjectBuildPlanErrorV1> {
    if ready_count
        .checked_add(blocked_count)
        .is_none_or(|total| total != content_count)
    {
        return Err(Revision3ProjectBuildPlanErrorV1::InvariantViolation(
            "domain ready and blocked counts do not partition content",
        ));
    }
    let status = if content_count == 0 {
        Revision3ProjectBuildDomainStatusV1::NotPresent
    } else if blocked_count == 0 {
        Revision3ProjectBuildDomainStatusV1::Ready
    } else {
        Revision3ProjectBuildDomainStatusV1::Blocked
    };
    Ok(Revision3ProjectBuildDomainSummaryV1 {
        domain,
        status,
        content_count,
        ready_count,
        blocked_count,
    })
}

fn add_blocker(
    aggregate: &mut BTreeMap<BlockerKey, u64>,
    category: Revision3ProjectBuildBlockerCategoryV1,
    domain: Revision3ProjectBuildDomainV1,
    reason: Revision3ProjectBuildBlockReasonV1,
    affected: u64,
) -> Result<(), Revision3ProjectBuildPlanErrorV1> {
    if affected == 0 {
        return Err(Revision3ProjectBuildPlanErrorV1::InvariantViolation(
            "zero-sized blocker group",
        ));
    }
    let value = aggregate.entry((category, domain, reason)).or_default();
    *value = value
        .checked_add(affected)
        .ok_or(Revision3ProjectBuildPlanErrorV1::CountOverflow)?;
    Ok(())
}

fn usize_to_u64(value: usize) -> Result<u64, Revision3ProjectBuildPlanErrorV1> {
    u64::try_from(value).map_err(|_| Revision3ProjectBuildPlanErrorV1::CountOverflow)
}

fn seal_serializable<T: Serialize>(
    value: &T,
) -> Result<ContentSeal, Revision3ProjectBuildPlanErrorV1> {
    let bytes =
        serde_json::to_vec(value).map_err(Revision3ProjectBuildPlanErrorV1::SealSerialization)?;
    Ok(seal_bytes(&bytes))
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::dataasset_stage::tests::{publish_reviewed_stage, reviewed_wolf_fixture, TestRoot};
    use crate::model_revision3::{
        DialogLine, Entity, EntityKind, LocalizationEntry, OggCodec, OggMetadata, SchemaRevisionV3,
        ScriptModule, ScriptModuleStatus, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot,
        VoiceTake, VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
    };
    use crate::{
        ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, FormatV2, GameGenerationAnchor,
        LocaleCode, ProjectMeta, WorkingProjectStore, WorkingStoreLimits,
        REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
    };

    fn id(tag: u8) -> crate::EntityId {
        crate::EntityId::from_bytes([tag; 16])
    }

    fn digest(tag: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([tag; 32])
    }

    fn content_seal(tag: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: digest(tag),
        }
    }

    fn base_project() -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x10; 16]),
            revision: 7,
            meta: ProjectMeta {
                name: "ReadinessTest".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: content_seal(0x20, 4096),
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn imported_origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "readiness-tests".to_owned(),
            source_seal: content_seal(tag, 100),
            external_identity: None,
        }
    }

    fn domain(
        plan: &Revision3ProjectBuildPlanV1,
        expected: Revision3ProjectBuildDomainV1,
    ) -> &Revision3ProjectBuildDomainSummaryV1 {
        plan.domains
            .iter()
            .find(|summary| summary.domain == expected)
            .unwrap()
    }

    fn existing_target() -> VoiceTarget {
        VoiceTarget {
            archive: "german_new.zip".to_owned(),
            member: "Npc/Asghan/GRD_263_ASGHAN_OPEN_INFO_06_02.ogg".to_owned(),
            operation: VoiceOperation::Replace,
            archive_seal: ArchiveSeal {
                byte_len: 2048,
                sha256: digest(0x51),
            },
            member_proof: VoiceMemberProof::Present {
                uncompressed_size: 8192,
                crc32: 0x1234,
            },
        }
    }

    fn voice_project(resolution: VoiceTargetResolution, selected: bool) -> ProjectRevision3 {
        let mut project = base_project();
        let locale: LocaleCode = "de".parse().unwrap();
        let localization_id = id(1);
        let line_id = id(2);
        let slot_id = id(3);
        let take_id = id(4);
        let asset = AssetRef {
            sha256: digest(0x41),
            byte_len: 8192,
            logical_name: "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg".to_owned(),
        };
        project.authoring_locales.insert(locale.clone());
        project.asset_store.assets.insert(
            asset.sha256,
            AssetMeta {
                byte_len: asset.byte_len,
                media_type: "audio/ogg".to_owned(),
            },
        );
        project.entities.extend([
            (
                localization_id,
                Entity {
                    id: localization_id,
                    display_name: "sensitive localization label".to_owned(),
                    origin: imported_origin(1),
                    revision: 1,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: "GRD_263_ASGHAN_OPEN_INFO_06_02".to_owned(),
                        texts: BTreeMap::from([(
                            locale.clone(),
                            "sensitive spoken text".to_owned(),
                        )]),
                    }),
                },
            ),
            (
                line_id,
                Entity {
                    id: line_id,
                    display_name: "sensitive dialog label".to_owned(),
                    origin: imported_origin(2),
                    revision: 1,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project.project_id,
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("sensitive speaker".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(project.project_id, slot_id, EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                slot_id,
                Entity {
                    id: slot_id,
                    display_name: "sensitive slot label".to_owned(),
                    origin: imported_origin(3),
                    revision: 1,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale.clone(),
                        target_resolution: resolution,
                        candidates: vec![TypedRef::new(
                            project.project_id,
                            take_id,
                            EntityKind::VoiceTake,
                        )],
                        selected: selected.then(|| {
                            TypedRef::new(project.project_id, take_id, EntityKind::VoiceTake)
                        }),
                    }),
                },
            ),
            (
                take_id,
                Entity {
                    id: take_id,
                    display_name: "sensitive take label".to_owned(),
                    origin: imported_origin(4),
                    revision: 1,
                    payload: EntityPayload::VoiceTake(VoiceTake {
                        locale,
                        asset,
                        ogg: OggMetadata {
                            codec: OggCodec::Vorbis,
                            channels: 1,
                            sample_rate: 48_000,
                            pages: 3,
                            logical_streams: 1,
                        },
                        status: VoiceTakeStatus::Approved,
                    }),
                },
            ),
        ]);
        project
    }

    #[test]
    fn empty_project_has_fixed_optional_domains_and_no_authority() {
        let project = base_project();
        let first = plan_revision3_project_build_v1(&project, &[]).unwrap();
        let second = plan_revision3_project_build_v1(&project, &[]).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.outcome, Revision3ProjectBuildOutcomeV1::Empty);
        assert_eq!(first.production_content_count, 0);
        assert_eq!(first.domains.len(), 8);
        assert!(first.domains.iter().all(|summary| {
            summary.status == Revision3ProjectBuildDomainStatusV1::NotPresent
                && summary.content_count == 0
                && summary.ready_count == 0
                && summary.blocked_count == 0
        }));
        assert!(first.blockers.is_empty());
        assert_eq!(
            first.scope,
            Revision3ProjectBuildPlanScopeV1::ProjectBuildReadinessOnly
        );
        assert_eq!(
            first.build_authority,
            Revision3ProjectBuildAuthorityV1::NotGranted
        );
        assert_eq!(
            first.artifact_status,
            Revision3ProjectBuildArtifactStatusV1::NotCreated
        );
        assert_eq!(
            first.deployment_status,
            Revision3ProjectBuildDeploymentStatusV1::NotPerformed
        );
        assert_eq!(
            first.runtime_status,
            Revision3ProjectBuildRuntimeStatusV1::RuntimeUnqualified
        );
        assert_eq!(
            first.publication_status,
            Revision3ProjectBuildPublicationStatusV1::NotSupported
        );
    }

    #[test]
    fn only_project_owned_localization_and_dialog_are_output_roots() {
        let mut project = base_project();
        let locale: LocaleCode = "de".parse().unwrap();
        let imported_id = id(1);
        let new_id = id(2);
        project.entities.insert(
            imported_id,
            Entity {
                id: imported_id,
                display_name: "context only".to_owned(),
                origin: imported_origin(1),
                revision: 0,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "IMPORTED_CONTEXT".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        project.entities.insert(
            new_id,
            Entity {
                id: new_id,
                display_name: "new localization".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "NEW_LOCALIZATION".to_owned(),
                },
                revision: 0,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "NEW_LOCALIZATION".to_owned(),
                    texts: BTreeMap::from([(locale, "text".to_owned())]),
                }),
            },
        );
        let dialog_id = id(3);
        project.entities.insert(
            dialog_id,
            Entity {
                id: dialog_id,
                display_name: "new dialog".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "NEW_DIALOG".to_owned(),
                },
                revision: 0,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project.project_id,
                        imported_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: None,
                    voice_slots: BTreeMap::new(),
                }),
            },
        );

        let plan = plan_revision3_project_build_v1(&project, &[]).unwrap();
        assert_eq!(plan.outcome, Revision3ProjectBuildOutcomeV1::Blocked);
        assert_eq!(plan.production_content_count, 2);
        assert_eq!(
            domain(&plan, Revision3ProjectBuildDomainV1::Localization),
            &Revision3ProjectBuildDomainSummaryV1 {
                domain: Revision3ProjectBuildDomainV1::Localization,
                status: Revision3ProjectBuildDomainStatusV1::Blocked,
                content_count: 1,
                ready_count: 0,
                blocked_count: 1,
            }
        );
        assert_eq!(plan.blockers.len(), 2);
        assert!(plan.blockers.contains(&Revision3ProjectBuildBlockerV1 {
            category: Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport,
            domain: Revision3ProjectBuildDomainV1::Localization,
            reason: Revision3ProjectBuildBlockReasonV1::LocalizationLoweringUnavailable,
            affected_count: 1,
        }));
        assert!(plan.blockers.contains(&Revision3ProjectBuildBlockerV1 {
            category: Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport,
            domain: Revision3ProjectBuildDomainV1::Dialog,
            reason: Revision3ProjectBuildBlockReasonV1::DialogLoweringUnavailable,
            affected_count: 1,
        }));

        assert!(authored_output_origin(&OriginRef::Generated {
            generator_id: "readiness-tests.generated-localization".to_owned(),
            generator_version: 1,
            owner: TypedRef::new(project.project_id, dialog_id, EntityKind::DialogLine,),
        }));
    }

    #[test]
    fn voice_only_imported_context_can_have_complete_coverage_without_build_authority() {
        let project = voice_project(
            VoiceTargetResolution::Resolved {
                target: existing_target(),
            },
            true,
        );
        let plan = plan_revision3_project_build_v1(&project, &[]).unwrap();

        assert_eq!(
            plan.outcome,
            Revision3ProjectBuildOutcomeV1::CoverageComplete
        );
        assert_eq!(plan.production_content_count, 1);
        assert!(plan.blockers.is_empty());
        assert_eq!(
            domain(&plan, Revision3ProjectBuildDomainV1::Localization).status,
            Revision3ProjectBuildDomainStatusV1::NotPresent
        );
        assert_eq!(
            domain(&plan, Revision3ProjectBuildDomainV1::Dialog).status,
            Revision3ProjectBuildDomainStatusV1::NotPresent
        );
        assert_eq!(
            domain(&plan, Revision3ProjectBuildDomainV1::Voice).status,
            Revision3ProjectBuildDomainStatusV1::Ready
        );
        assert_eq!(
            plan.build_authority,
            Revision3ProjectBuildAuthorityV1::NotGranted
        );

        let wire = serde_json::to_string(&plan).unwrap();
        for sensitive in [
            "sensitive localization label",
            "sensitive spoken text",
            "sensitive dialog label",
            "sensitive speaker",
            "sensitive slot label",
            "sensitive take label",
            "Npc/Asghan",
            "german_new.zip",
        ] {
            assert!(!wire.contains(sensitive));
        }
        assert!(!wire.contains("legacy"));
        assert!(!wire.contains("migration"));
        assert!(!wire.contains("compatibility"));
    }

    #[test]
    fn voice_blockers_are_author_project_aggregates_without_entity_details() {
        let project = voice_project(VoiceTargetResolution::Unresolved, false);
        let plan = plan_revision3_project_build_v1(&project, &[]).unwrap();

        assert_eq!(plan.outcome, Revision3ProjectBuildOutcomeV1::Blocked);
        assert_eq!(
            domain(&plan, Revision3ProjectBuildDomainV1::Voice),
            &Revision3ProjectBuildDomainSummaryV1 {
                domain: Revision3ProjectBuildDomainV1::Voice,
                status: Revision3ProjectBuildDomainStatusV1::Blocked,
                content_count: 1,
                ready_count: 0,
                blocked_count: 1,
            }
        );
        assert_eq!(plan.blockers.len(), 2);
        assert!(plan.blockers.iter().all(|blocker| {
            blocker.category == Revision3ProjectBuildBlockerCategoryV1::AuthorProject
                && blocker.domain == Revision3ProjectBuildDomainV1::Voice
                && blocker.affected_count == 1
        }));
        assert!(plan.blockers.iter().any(|blocker| {
            blocker.reason == Revision3ProjectBuildBlockReasonV1::VoiceTargetUnresolved
        }));
        assert!(plan.blockers.iter().any(|blocker| {
            blocker.reason == Revision3ProjectBuildBlockReasonV1::VoiceSelectedTakeMissing
        }));
    }

    #[test]
    fn unsupported_voice_line_label_is_not_misreported_as_a_project_name_problem() {
        let mut project = voice_project(VoiceTargetResolution::Unresolved, false);
        project.entities.get_mut(&id(2)).unwrap().display_name = String::new();

        let plan = plan_revision3_project_build_v1(&project, &[]).unwrap();

        assert_eq!(plan.outcome, Revision3ProjectBuildOutcomeV1::Blocked);
        assert!(plan.blockers.contains(&Revision3ProjectBuildBlockerV1 {
            category: Revision3ProjectBuildBlockerCategoryV1::AuthorProject,
            domain: Revision3ProjectBuildDomainV1::Voice,
            reason: Revision3ProjectBuildBlockReasonV1::VoiceLineLabelUnsupported,
            affected_count: 1,
        }));
        assert!(!plan.blockers.iter().any(|blocker| {
            blocker.reason == Revision3ProjectBuildBlockReasonV1::VoiceProjectNameUnsupported
        }));
    }

    #[test]
    fn unsupported_voice_project_name_retains_its_distinct_blocker() {
        let mut project = voice_project(VoiceTargetResolution::Unresolved, false);
        project.meta.name = "../unsafe".to_owned();

        let plan = plan_revision3_project_build_v1(&project, &[]).unwrap();

        assert!(plan.blockers.contains(&Revision3ProjectBuildBlockerV1 {
            category: Revision3ProjectBuildBlockerCategoryV1::AuthorProject,
            domain: Revision3ProjectBuildDomainV1::Voice,
            reason: Revision3ProjectBuildBlockReasonV1::VoiceProjectNameUnsupported,
            affected_count: 1,
        }));
        assert!(!plan.blockers.iter().any(|blocker| {
            blocker.reason == Revision3ProjectBuildBlockReasonV1::VoiceLineLabelUnsupported
        }));
    }

    #[test]
    fn unsupported_voice_add_is_a_toolkit_gap_not_an_author_error() {
        assert_eq!(
            map_voice_reason(Revision3VoiceBuildBlockReasonV1::UnqualifiedAdd),
            Some((
                Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport,
                Revision3ProjectBuildBlockReasonV1::VoiceAddUnqualified,
            ))
        );
    }

    #[test]
    fn reviewed_dataasset_stage_can_have_complete_coverage_and_exact_set_is_required() {
        let root = TestRoot::new("project-build-plan");
        let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = reviewed_wolf_fixture();
        let staged = publish_reviewed_stage(&root, &store, &fixture);
        let stage = staged.stage().clone();

        let plan = plan_revision3_project_build_v1(staged.project(), &[stage.clone()]).unwrap();
        assert_eq!(
            plan.outcome,
            Revision3ProjectBuildOutcomeV1::CoverageComplete
        );
        assert_eq!(plan.production_content_count, 1);
        assert_eq!(
            domain(&plan, Revision3ProjectBuildDomainV1::DataAssets),
            &Revision3ProjectBuildDomainSummaryV1 {
                domain: Revision3ProjectBuildDomainV1::DataAssets,
                status: Revision3ProjectBuildDomainStatusV1::Ready,
                content_count: 1,
                ready_count: 1,
                blocked_count: 0,
            }
        );
        assert!(matches!(
            plan_revision3_project_build_v1(staged.project(), &[]),
            Err(Revision3ProjectBuildPlanErrorV1::DataAssetStageSetMismatch)
        ));
        assert!(matches!(
            plan_revision3_project_build_v1(staged.project(), &[stage.clone(), stage]),
            Err(Revision3ProjectBuildPlanErrorV1::DuplicateDataAssetStage)
        ));
    }

    #[test]
    fn stage_binding_and_input_and_plan_seals_are_exact_and_revision_sensitive() {
        let root = TestRoot::new("project-build-seals");
        let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = reviewed_wolf_fixture();
        let staged = publish_reviewed_stage(&root, &store, &fixture);
        let stage = staged.stage().clone();
        let first = plan_revision3_project_build_v1(staged.project(), std::slice::from_ref(&stage))
            .unwrap();

        let mut rebound = staged.project().clone();
        rebound.project_id = ProjectId::from_bytes([0x77; 16]);
        assert!(matches!(
            plan_revision3_project_build_v1(&rebound, std::slice::from_ref(&stage)),
            Err(Revision3ProjectBuildPlanErrorV1::InvalidDataAssetStageBinding)
        ));

        let mut later = staged.project().clone();
        later.revision += 1;
        let later_plan =
            plan_revision3_project_build_v1(&later, std::slice::from_ref(&stage)).unwrap();
        assert_ne!(first.input_seal, later_plan.input_seal);
        assert_ne!(first.plan_seal, later_plan.plan_seal);
    }

    #[test]
    fn dataasset_reason_mapping_keeps_support_gap_distinct_from_author_project() {
        assert_eq!(
            map_dataasset_reason(ReviewedDataAssetStageBlockReasonV1::UnsupportedTarget),
            (
                Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport,
                Revision3ProjectBuildBlockReasonV1::DataAssetTargetUnsupported,
            )
        );
        assert_eq!(
            map_dataasset_reason(ReviewedDataAssetStageBlockReasonV1::MalformedReplacement),
            (
                Revision3ProjectBuildBlockerCategoryV1::AuthorProject,
                Revision3ProjectBuildBlockReasonV1::DataAssetReplacementMalformed,
            )
        );
        assert_eq!(
            map_dataasset_reason(ReviewedDataAssetStageBlockReasonV1::ReviewedPreparation(
                ReviewedDataAssetErrorV1::UnknownTargetPath,
            )),
            (
                Revision3ProjectBuildBlockerCategoryV1::ToolkitSupport,
                Revision3ProjectBuildBlockReasonV1::DataAssetTargetUnsupported,
            )
        );
    }

    #[test]
    fn invalid_project_is_rejected_before_planning() {
        let mut project = base_project();
        project.project_id = ProjectId::from_bytes([0; 16]);
        assert!(matches!(
            plan_revision3_project_build_v1(&project, &[]),
            Err(Revision3ProjectBuildPlanErrorV1::InvalidProject(_))
        ));
    }

    #[test]
    fn orphan_script_module_is_rejected_instead_of_reporting_empty() {
        let mut project = base_project();
        let module_id = id(0x31);
        let missing_owner = TypedRef::new(project.project_id, id(0x32), EntityKind::QuestDraft);
        let source = "// orphan Quest backing must not become standalone production\n".to_owned();
        let module = ScriptModule {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            owner: missing_owner.clone(),
            module_namespace: "Readiness.OrphanQuest".to_owned(),
            module_relative_path: "Readiness/OrphanQuest.as".to_owned(),
            source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
            source,
            input_fingerprint: digest(0x33),
            status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        };
        project.entities.insert(
            module_id,
            Entity {
                id: module_id,
                display_name: "orphan Quest backing".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner: missing_owner,
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(module),
            },
        );

        assert!(matches!(
            plan_revision3_project_build_v1(&project, &[]),
            Err(Revision3ProjectBuildPlanErrorV1::InvalidProject(message))
                if message.contains("not the exact generated backing")
        ));
    }

    #[test]
    fn derived_script_modules_do_not_double_count_production_roots() {
        let counts = EntityDomainCounts {
            npc: 2,
            quest: 3,
            scripts: 5,
            ..EntityDomainCounts::default()
        };
        assert_eq!(production_content_count(&counts, 0).unwrap(), 5);
    }
}
