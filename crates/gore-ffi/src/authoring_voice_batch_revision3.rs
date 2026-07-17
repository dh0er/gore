//! Safe, deterministic revision-3 Voice folder planning and prepare-only import.
//!
//! V1 accepts one direct folder and one locale. Every direct `.ogg` member is
//! part of the all-or-nothing plan. Planning performs no Store/CAS write;
//! preparation repeats and seals all source reads before installing immutable
//! objects, applies one filesystem-free batch transaction, prepares one fully
//! reopenable checkpoint, and deliberately leaves the published head fixed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(test)]
use std::{fs, path::PathBuf};

use gore_authoring::model_revision3::{EntityKind, EntityPayload, VoiceTakeStatus};
use gore_authoring::{
    apply_revision3_voice_take_batch_transaction_v1, preflight_revision3_voice_take_transaction_v1,
    AssetVerification, EntityId, ImportedOgg, LocaleCode, OggCodec, OggImportFailureContext,
    PreparedOggImport, ProjectRevision3, Revision3VoiceTakeBatchEvaluationV1,
    Revision3VoiceTakePreflightEvaluationV1, Revision3VoiceTakeStageRequestV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_BATCH_ITEMS_V1, MAX_REVISION3_VOICE_BATCH_PROJECT_WORK_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::err;
use crate::voice::{
    SecureDirectDirectory, SecureDirectEntry, SecureDirectEntryKind, SecureSourceReadError,
};

pub(super) const PLAN_COMMAND: &str = "authoring_store_plan_revision3_voice_batch_v1";
pub(super) const PREPARE_COMMAND: &str = "authoring_store_prepare_revision3_voice_batch_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_DIRECT_ENTRIES: usize = 4096;
const MAX_DIRECT_NAME_BYTES: usize = 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_AGGREGATE_OGG_BYTES: u64 = 256 * 1024 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2 + MAX_PATH_BYTES * 18 + 256 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanPayload {
    current_project_json: String,
    expected_head_json: String,
    game_root: String,
    locale: String,
    root: String,
    source_folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparePayload {
    current_project_json: String,
    expected_head_json: String,
    game_root: String,
    locale: String,
    root: String,
    source_folder: String,
    expected_source_manifest_sha256: String,
    expected_plan_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct AssetWire {
    sha256: String,
    byte_len: u64,
    logical_name: String,
}

#[derive(Debug, Clone, Serialize)]
struct OggWire {
    codec: &'static str,
    channels: u8,
    sample_rate: u32,
    pages: u32,
    logical_streams: u32,
}

#[derive(Debug, Clone, Serialize)]
struct PlanItemWire {
    source_name: String,
    status: &'static str,
    line_display_name: Option<String>,
    speaker: Option<String>,
    line_id: Option<String>,
    localization_id: Option<String>,
    loc_id: Option<String>,
    slot_id: Option<String>,
    take_id: Option<String>,
    slot_created: Option<bool>,
    voice_request_json: Option<String>,
    asset: Option<AssetWire>,
    ogg: Option<OggWire>,
}

impl PlanItemWire {
    fn bare(source_name: String, status: &'static str) -> Self {
        Self {
            source_name,
            status,
            line_display_name: None,
            speaker: None,
            line_id: None,
            localization_id: None,
            loc_id: None,
            slot_id: None,
            take_id: None,
            slot_created: None,
            voice_request_json: None,
            asset: None,
            ogg: None,
        }
    }
}

struct InternalItem {
    wire: PlanItemWire,
    prepared: Option<PreparedOggImport>,
}

#[derive(Debug, Clone)]
struct TargetMatch {
    line_display_name: String,
    speaker: Option<String>,
    line_id: EntityId,
    localization_id: EntityId,
    loc_id: String,
    existing_slot_id: Option<EntityId>,
}

struct Analysis {
    store: WorkingProjectStore,
    root_guards: DisjointRootGuards,
    basis: gore_authoring::OpenedRevision3Checkpoint,
    basis_head_json: String,
    locale: LocaleCode,
    source: SecureDirectDirectory,
    scan: Vec<SecureDirectEntry>,
    scanned_entry_count: usize,
    ogg_file_count: usize,
    ignored_entry_count: usize,
    items: Vec<InternalItem>,
    source_manifest_sha256: String,
    plan_sha256: String,
    ready_count: usize,
    already_present_count: usize,
    blocked_count: usize,
}

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
}

impl Failure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: truncate_utf8(message.into(), MAX_ERROR_MESSAGE_BYTES),
        }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

pub(super) fn plan_revision3_voice_batch_v1_raw(input: &str) -> Value {
    plan_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn prepare_revision3_voice_batch_v1_raw(input: &str) -> Value {
    prepare_inner(input).unwrap_or_else(Failure::response)
}

fn plan_inner(input: &str) -> Result<Value, Failure> {
    let payload: PlanPayload = parse_exact_wire(input, PLAN_COMMAND)?;
    let analysis = analyze(&payload)?;
    let status = if analysis.blocked_count > 0 {
        "blocked"
    } else if analysis.ready_count > 0 {
        "ready"
    } else {
        "no_changes"
    };
    let response = json!({
        "ok": true,
        "outcome": "planned",
        "basis_head_json": analysis.basis_head_json,
        "project_id": analysis.basis.project.project_id.to_string(),
        "revision": analysis.basis.project.revision,
        "locale": analysis.locale.as_str(),
        "source_manifest_sha256": analysis.source_manifest_sha256,
        "plan_sha256": analysis.plan_sha256,
        "status": status,
        "scanned_entry_count": analysis.scanned_entry_count,
        "ogg_file_count": analysis.ogg_file_count,
        "ready_count": analysis.ready_count,
        "already_present_count": analysis.already_present_count,
        "blocked_count": analysis.blocked_count,
        "ignored_entry_count": analysis.ignored_entry_count,
        "items": analysis.items.into_iter().map(|item| item.wire).collect::<Vec<_>>(),
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "target_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(response)
}

fn prepare_inner(input: &str) -> Result<Value, Failure> {
    prepare_inner_with_source_guard(input, || {})
}

fn prepare_inner_with_source_guard(
    input: &str,
    after_first_analysis: impl FnOnce(),
) -> Result<Value, Failure> {
    let payload: PreparePayload = parse_exact_wire(input, PREPARE_COMMAND)?;
    validate_sha256(&payload.expected_source_manifest_sha256)?;
    validate_sha256(&payload.expected_plan_sha256)?;
    let plan_payload = PlanPayload {
        current_project_json: payload.current_project_json,
        expected_head_json: payload.expected_head_json,
        game_root: payload.game_root,
        locale: payload.locale,
        root: payload.root,
        source_folder: payload.source_folder,
    };
    let mut analysis = analyze(&plan_payload)?;
    if analysis.source_manifest_sha256 != payload.expected_source_manifest_sha256
        || analysis.plan_sha256 != payload.expected_plan_sha256
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_PLAN_CHANGED",
            "the exact source folder or Voice batch plan differs from the reviewed seals",
        ));
    }
    if analysis.blocked_count != 0 || analysis.ready_count == 0 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_NOT_READY",
            "the exact Voice folder plan is blocked or contains no new take",
        ));
    }
    after_first_analysis();

    // Re-read every source accepted by the reviewed plan, including no-op
    // entries. Only exact bytes and metadata may proceed to CAS installation.
    for item in &mut analysis.items {
        let Some(first) = item.prepared.as_ref() else {
            continue;
        };
        let bytes = analysis
            .source
            .read_single_link_member(&item.wire.source_name, max_ogg_bytes()?)
            .map_err(map_source_failure)?;
        let second = analysis
            .store
            .prepare_ogg_bytes_classified(bytes, item.wire.source_name.clone())
            .map_err(map_prepare_ogg_failure)?;
        if !first.has_same_content(&second) {
            return Err(source_changed());
        }
    }
    analysis.source.revalidate().map_err(map_source_failure)?;
    if analysis
        .source
        .scan(MAX_DIRECT_ENTRIES, MAX_DIRECT_NAME_BYTES)
        .map_err(map_source_failure)?
        != analysis.scan
    {
        return Err(source_changed());
    }
    analysis.root_guards.revalidate()?;
    require_current_basis(&analysis.store, &analysis.basis)?;

    let mut request_jsons = Vec::with_capacity(analysis.ready_count);
    let mut receipts = Vec::with_capacity(analysis.ready_count);
    let mut response_sources = Vec::with_capacity(analysis.ready_count);
    analysis.root_guards.revalidate()?;
    for item in &mut analysis.items {
        if item.wire.status != "ready" {
            continue;
        }
        let request_json = item.wire.voice_request_json.clone().ok_or_else(invariant)?;
        let prepared = item.prepared.take().ok_or_else(invariant)?;
        let imported = analysis
            .store
            .install_prepared_ogg(prepared, Some(&analysis.basis.head))
            .map_err(map_store_error)?;
        let preview = imported_wire(&imported);
        if item
            .wire
            .asset
            .as_ref()
            .map(|asset| (&asset.sha256, asset.byte_len))
            != Some((&preview.0.sha256, preview.0.byte_len))
            || item.wire.ogg.as_ref().map(ogg_tuple) != Some(ogg_tuple(&preview.1))
        {
            return Err(invariant());
        }
        response_sources.push((item.wire.source_name.clone(), imported.deduplicated));
        request_jsons.push(request_json);
        receipts.push(imported);
    }
    analysis.root_guards.revalidate()?;

    let outcome = match apply_revision3_voice_take_batch_transaction_v1(
        &analysis.basis.head,
        &plan_payload.current_project_json,
        &request_jsons,
        receipts,
    )
    .map_err(|_| invariant())?
    {
        Revision3VoiceTakeBatchEvaluationV1::Applied(outcome) => *outcome,
        Revision3VoiceTakeBatchEvaluationV1::Rejected(_) => return Err(invariant()),
    };
    require_current_basis(&analysis.store, &analysis.basis)?;
    analysis.root_guards.revalidate()?;
    let prepared_checkpoint = analysis
        .store
        .prepare_revision3_checkpoint(Some(&analysis.basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = analysis
        .store
        .open_revision3_head_bytes(&prepared_checkpoint.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared_checkpoint.head || reopened.project != outcome.project {
        return Err(invariant());
    }
    for item in &outcome.items {
        analysis
            .store
            .verify_asset(&item.imported_ogg.asset, AssetVerification::Full)
            .map_err(map_store_error)?;
    }
    require_current_basis(&analysis.store, &analysis.basis)?;
    analysis.root_guards.revalidate()?;

    let head_json = String::from_utf8(prepared_checkpoint.head_bytes).map_err(|_| invariant())?;
    let response_items = outcome
        .items
        .iter()
        .zip(response_sources)
        .map(|(item, (source_name, asset_deduplicated))| {
            let (asset, ogg) = imported_wire(&item.imported_ogg);
            json!({
                "source_name": source_name,
                "line_id": item.line_id.to_string(),
                "localization_id": item.localization_id.to_string(),
                "slot_id": item.slot_id.to_string(),
                "take_id": item.take_id.to_string(),
                "take_status": "recorded",
                "slot_created": item.slot_created,
                "selected": false,
                "asset": asset,
                "ogg": ogg,
                "asset_deduplicated": asset_deduplicated,
            })
        })
        .collect::<Vec<_>>();
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": analysis.basis_head_json,
        "head_json": head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "locale": analysis.locale.as_str(),
        "source_manifest_sha256": analysis.source_manifest_sha256,
        "plan_sha256": analysis.plan_sha256,
        "imported_count": outcome.items.len(),
        "already_present_count": analysis.already_present_count,
        "items": response_items,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "target_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(response)
}

fn analyze(payload: &PlanPayload) -> Result<Analysis, Failure> {
    validate_common_payload(payload)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let locale: LocaleCode = payload.locale.parse().map_err(|_| invalid_request())?;
    let source = SecureDirectDirectory::open(Path::new(&payload.source_folder))
        .map_err(map_source_failure)?;
    // Preserve the Store-specific missing/unsafe taxonomy before binding the
    // same existing root as a retained no-follow disjointness capability.
    let store =
        WorkingProjectStore::open_existing(Path::new(&payload.root), WorkingStoreLimits::default())
            .map_err(map_store_error)?;
    let root_guards = ensure_roots_pairwise_disjoint(
        Path::new(&payload.root),
        Path::new(&payload.game_root),
        &source,
    )?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    root_guards.revalidate()?;
    if basis.head != expected_head {
        return Err(head_conflict());
    }
    let canonical_project = basis.project.to_canonical_json().map_err(|_| invariant())?;
    if canonical_project != payload.current_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    if basis.project.revision >= i64::MAX as u64 || basis.target_bytes_outside_signed_wire() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_RESPONSE_LIMIT",
            "the exact project contains an integer outside the signed transport range",
        ));
    }

    let scan = source
        .scan(MAX_DIRECT_ENTRIES, MAX_DIRECT_NAME_BYTES)
        .map_err(map_source_failure)?;
    let scanned_entry_count = scan.len();
    let ogg_file_count = scan.iter().filter(|entry| is_ogg_name(&entry.name)).count();
    let ignored_entry_count = scanned_entry_count - ogg_file_count;
    if ogg_file_count > MAX_REVISION3_VOICE_BATCH_ITEMS_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT",
            "the source folder contains more than 256 direct Ogg entries",
        ));
    }
    if payload
        .current_project_json
        .len()
        .checked_mul(ogg_file_count)
        .is_none_or(|work_bytes| work_bytes > MAX_REVISION3_VOICE_BATCH_PROJECT_WORK_BYTES_V1)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_INPUT_LIMIT",
            "project size and selected Ogg count exceed the bounded Voice batch work budget",
        ));
    }

    let mut case_counts = BTreeMap::<String, usize>::new();
    for entry in scan.iter().filter(|entry| is_ogg_name(&entry.name)) {
        *case_counts
            .entry(entry.name.to_ascii_lowercase())
            .or_default() += 1;
    }
    let mut used_ids = basis
        .project
        .entities
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut total_ogg_bytes = 0u64;
    let mut items = Vec::with_capacity(ogg_file_count);
    for entry in scan.iter().filter(|entry| is_ogg_name(&entry.name)) {
        let item = analyze_entry(
            &source,
            &store,
            &basis.head,
            &basis.project,
            &locale,
            entry,
            case_counts[&entry.name.to_ascii_lowercase()] > 1,
            &mut used_ids,
            &mut total_ogg_bytes,
        )?;
        items.push(item);
    }
    if total_ogg_bytes > MAX_AGGREGATE_OGG_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT",
            "the selected Ogg sources exceed the aggregate byte limit",
        ));
    }

    // Prove aggregate graph/capacity closure against the exact untouched basis.
    let ready_requests = items
        .iter()
        .filter(|item| item.wire.status == "ready")
        .map(|item| item.wire.voice_request_json.clone().ok_or_else(invariant))
        .collect::<Result<Vec<_>, _>>()?;
    let ready_receipts = items
        .iter()
        .filter(|item| item.wire.status == "ready")
        .map(|item| {
            item.prepared
                .as_ref()
                .map(PreparedOggImport::preview)
                .ok_or_else(invariant)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !ready_requests.is_empty()
        && !matches!(
            apply_revision3_voice_take_batch_transaction_v1(
                &basis.head,
                &payload.current_project_json,
                &ready_requests,
                ready_receipts,
            )
            .map_err(|_| invariant())?,
            Revision3VoiceTakeBatchEvaluationV1::Applied(_)
        )
    {
        for item in &mut items {
            if item.wire.status == "ready" {
                item.wire.status = "target_blocked";
                item.wire.slot_id = None;
                item.wire.take_id = None;
                item.wire.slot_created = None;
                item.wire.voice_request_json = None;
            }
        }
    }

    source.revalidate().map_err(map_source_failure)?;
    if source
        .scan(MAX_DIRECT_ENTRIES, MAX_DIRECT_NAME_BYTES)
        .map_err(map_source_failure)?
        != scan
    {
        return Err(source_changed());
    }
    require_current_basis(&store, &basis)?;
    root_guards.revalidate()?;
    let basis_head_json = canonical_head_json(&basis.head)?;
    let source_manifest_sha256 = hash_source_manifest(&scan, &items)?;
    let plan_sha256 = hash_plan(
        &basis_head_json,
        locale.as_str(),
        &source_manifest_sha256,
        &items,
    )?;
    let ready_count = items
        .iter()
        .filter(|item| item.wire.status == "ready")
        .count();
    let already_present_count = items
        .iter()
        .filter(|item| item.wire.status == "already_present")
        .count();
    let blocked_count = items.len() - ready_count - already_present_count;
    Ok(Analysis {
        store,
        root_guards,
        basis,
        basis_head_json,
        locale,
        source,
        scan,
        scanned_entry_count,
        ogg_file_count,
        ignored_entry_count,
        items,
        source_manifest_sha256,
        plan_sha256,
        ready_count,
        already_present_count,
        blocked_count,
    })
}

trait OpenedBasisWireGuard {
    fn target_bytes_outside_signed_wire(&self) -> bool;
}

impl OpenedBasisWireGuard for gore_authoring::OpenedRevision3Checkpoint {
    fn target_bytes_outside_signed_wire(&self) -> bool {
        self.project.target.executable.byte_len > i64::MAX as u64
    }
}

#[allow(clippy::too_many_arguments)]
fn analyze_entry(
    source: &SecureDirectDirectory,
    store: &WorkingProjectStore,
    basis_head: &WorkingHead,
    project: &ProjectRevision3,
    locale: &LocaleCode,
    entry: &SecureDirectEntry,
    case_collision: bool,
    used_ids: &mut BTreeSet<EntityId>,
    total_ogg_bytes: &mut u64,
) -> Result<InternalItem, Failure> {
    if case_collision {
        return Ok(InternalItem {
            wire: PlanItemWire::bare(entry.name.clone(), "case_collision"),
            prepared: None,
        });
    }
    let Some(stem) = ogg_stem(&entry.name) else {
        return Ok(InternalItem {
            wire: PlanItemWire::bare(entry.name.clone(), "source_invalid"),
            prepared: None,
        });
    };
    let matches = match_targets(project, stem, locale);
    let target = (matches.len() == 1).then(|| matches[0].clone());
    let mut wire = match matches.as_slice() {
        [] => PlanItemWire::bare(entry.name.clone(), "unmatched"),
        [_] => PlanItemWire::bare(entry.name.clone(), "source_unavailable"),
        _ => PlanItemWire::bare(entry.name.clone(), "ambiguous"),
    };
    if let Some(target) = &target {
        validate_target_presentation(target)?;
        set_target_base(&mut wire, target);
    }
    if entry.kind != SecureDirectEntryKind::File {
        wire.status = "source_unsafe";
        return Ok(InternalItem {
            wire,
            prepared: None,
        });
    }

    let bytes = match source.read_single_link_member(&entry.name, max_ogg_bytes()?) {
        Ok(bytes) => bytes,
        Err(error) => {
            wire.status = source_status(error);
            return Ok(InternalItem {
                wire,
                prepared: None,
            });
        }
    };
    *total_ogg_bytes = total_ogg_bytes
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT",
                "the aggregate Ogg byte count cannot be represented",
            )
        })?;
    if *total_ogg_bytes > MAX_AGGREGATE_OGG_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT",
            "the selected Ogg sources exceed the aggregate byte limit",
        ));
    }
    let prepared = match store.prepare_ogg_bytes_classified(bytes, entry.name.clone()) {
        Ok(prepared) => prepared,
        Err(error) => match error.context() {
            OggImportFailureContext::Store => {
                return Err(map_store_error(error.into_store_error()));
            }
            context => {
                wire.status = import_context_status(context);
                return Ok(InternalItem {
                    wire,
                    prepared: None,
                });
            }
        },
    };
    let preview = prepared.preview();
    let (asset, ogg) = imported_wire(&preview);
    wire.asset = Some(asset);
    wire.ogg = Some(ogg);

    let Some(target) = target else {
        return Ok(InternalItem {
            wire,
            prepared: Some(prepared),
        });
    };
    if let Some((slot_id, take_id)) = existing_take_with_digest(project, &target, locale, &preview)
    {
        wire.status = "already_present";
        wire.slot_id = Some(slot_id.to_string());
        wire.take_id = Some(take_id.to_string());
        wire.slot_created = Some(false);
        return Ok(InternalItem {
            wire,
            prepared: Some(prepared),
        });
    }

    let (slot_id, slot_created) = match target.existing_slot_id {
        Some(slot_id) => (slot_id, false),
        None => (
            derive_unique_entity_id(
                b"gore.revision3.voice-folder.slot.v1",
                basis_head,
                target.line_id,
                locale,
                preview.asset.sha256.as_bytes(),
                used_ids,
            )?,
            true,
        ),
    };
    let take_id = derive_unique_entity_id(
        b"gore.revision3.voice-folder.take.v1",
        basis_head,
        target.line_id,
        locale,
        preview.asset.sha256.as_bytes(),
        used_ids,
    )?;
    let request = Revision3VoiceTakeStageRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: target.line_id,
        slot_id,
        take_id,
        locale: locale.clone(),
        text: None,
        take_display_name: take_display_name(&target.line_display_name, locale),
        logical_name: entry.name.clone(),
        status: VoiceTakeStatus::Recorded,
        select_take: false,
    };
    let request_json = request.to_canonical_json().map_err(|_| invariant())?;
    match preflight_revision3_voice_take_transaction_v1(
        basis_head,
        &project.to_canonical_json().map_err(|_| invariant())?,
        &request_json,
    )
    .map_err(|_| invariant())?
    {
        Revision3VoiceTakePreflightEvaluationV1::Ready => {
            wire.status = "ready";
            wire.slot_id = Some(slot_id.to_string());
            wire.take_id = Some(take_id.to_string());
            wire.slot_created = Some(slot_created);
            wire.voice_request_json = Some(request_json);
        }
        Revision3VoiceTakePreflightEvaluationV1::Rejected(_) => {
            wire.status = "target_blocked";
        }
    }
    Ok(InternalItem {
        wire,
        prepared: Some(prepared),
    })
}

fn match_targets(project: &ProjectRevision3, stem: &str, locale: &LocaleCode) -> Vec<TargetMatch> {
    let localization_ids = project
        .entities
        .iter()
        .filter_map(|(entity_id, entity)| match &entity.payload {
            EntityPayload::LocalizationEntry(localization)
                if localization.loc_id.eq_ignore_ascii_case(stem) =>
            {
                Some((*entity_id, localization.loc_id.clone()))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut matches = project
        .entities
        .iter()
        .filter_map(|(line_id, entity)| {
            let EntityPayload::DialogLine(line) = &entity.payload else {
                return None;
            };
            if line.localization.project_id != project.project_id
                || line.localization.expected_kind != EntityKind::LocalizationEntry
            {
                return None;
            }
            let loc_id = localization_ids.get(&line.localization.id)?;
            Some(TargetMatch {
                line_display_name: entity.display_name.clone(),
                speaker: line.speaker_hint.clone(),
                line_id: *line_id,
                localization_id: line.localization.id,
                loc_id: loc_id.clone(),
                existing_slot_id: line.voice_slots.get(locale).map(|reference| reference.id),
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|target| target.line_id);
    matches
}

fn set_target_base(wire: &mut PlanItemWire, target: &TargetMatch) {
    wire.line_display_name = Some(target.line_display_name.clone());
    wire.speaker = target.speaker.clone();
    wire.line_id = Some(target.line_id.to_string());
    wire.localization_id = Some(target.localization_id.to_string());
    wire.loc_id = Some(target.loc_id.clone());
}

fn validate_target_presentation(target: &TargetMatch) -> Result<(), Failure> {
    fn valid(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 256
            && value.trim() == value
            && !value.chars().any(char::is_control)
    }
    if !valid(&target.line_display_name)
        || target
            .speaker
            .as_deref()
            .is_some_and(|speaker| !valid(speaker))
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_PROJECT_CONFLICT",
            "the matched DialogLine has presentation text outside the bounded Voice UI contract",
        ));
    }
    Ok(())
}

fn existing_take_with_digest(
    project: &ProjectRevision3,
    target: &TargetMatch,
    locale: &LocaleCode,
    imported: &ImportedOgg,
) -> Option<(EntityId, EntityId)> {
    let slot_id = target.existing_slot_id?;
    let EntityPayload::VoiceSlot(slot) = &project.entities.get(&slot_id)?.payload else {
        return None;
    };
    if &slot.locale != locale {
        return None;
    }
    slot.candidates.iter().find_map(|candidate| {
        let entity = project.entities.get(&candidate.id)?;
        let EntityPayload::VoiceTake(take) = &entity.payload else {
            return None;
        };
        (&take.locale == locale && take.asset.sha256 == imported.asset.sha256)
            .then_some((slot_id, candidate.id))
    })
}

fn derive_unique_entity_id(
    domain: &[u8],
    basis_head: &WorkingHead,
    line_id: EntityId,
    locale: &LocaleCode,
    source_digest: &[u8; 32],
    used_ids: &mut BTreeSet<EntityId>,
) -> Result<EntityId, Failure> {
    for probe in 0..=u32::MAX {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update([0]);
        hasher.update(basis_head.snapshot.sha256.as_bytes());
        hasher.update(line_id.as_bytes());
        hasher.update(locale.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(source_digest);
        hasher.update(probe.to_le_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        if bytes == [0; 16] {
            continue;
        }
        let candidate = EntityId::from_bytes(bytes);
        if used_ids.insert(candidate) {
            return Ok(candidate);
        }
    }
    Err(Failure::new(
        "AUTHORING_REVISION3_VOICE_BATCH_STORE_LIMIT",
        "a deterministic free Voice entity identity could not be allocated",
    ))
}

fn take_display_name(line_display_name: &str, locale: &LocaleCode) -> String {
    let mut value = format!("{line_display_name} {} Take", locale.as_str());
    if value.chars().any(char::is_control) {
        value = format!("Voice {} Take", locale.as_str());
    }
    truncate_utf8(
        value,
        gore_authoring::MAX_REVISION3_VOICE_DISPLAY_NAME_BYTES_V1,
    )
}

fn is_ogg_name(name: &str) -> bool {
    name.as_bytes()
        .get(name.len().saturating_sub(4)..)
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case(b".ogg"))
}

fn ogg_stem(name: &str) -> Option<&str> {
    if !name.is_ascii() || !is_ogg_name(name) {
        return None;
    }
    let stem = &name[..name.len() - 4];
    gore_authoring::validate_revision3_voice_loc_id_basename_stem_v1(stem)
        .ok()
        .map(|_| stem)
}

fn imported_wire(imported: &ImportedOgg) -> (AssetWire, OggWire) {
    (
        AssetWire {
            sha256: imported.asset.sha256.to_string(),
            byte_len: imported.asset.byte_len,
            logical_name: imported.asset.logical_name.clone(),
        },
        OggWire {
            codec: match imported.ogg.codec {
                OggCodec::Vorbis => "vorbis",
                OggCodec::Opus => "opus",
            },
            channels: imported.ogg.channels,
            sample_rate: imported.ogg.sample_rate,
            pages: imported.ogg.pages,
            logical_streams: imported.ogg.logical_streams,
        },
    )
}

fn ogg_tuple(ogg: &OggWire) -> (&'static str, u8, u32, u32, u32) {
    (
        ogg.codec,
        ogg.channels,
        ogg.sample_rate,
        ogg.pages,
        ogg.logical_streams,
    )
}

#[derive(Serialize)]
struct SourceManifest<'a> {
    entries: Vec<SourceManifestEntry<'a>>,
}

#[derive(Serialize)]
struct SourceManifestEntry<'a> {
    name: &'a str,
    kind: &'static str,
    sha256: Option<&'a str>,
    byte_len: Option<u64>,
}

fn hash_source_manifest(
    scan: &[SecureDirectEntry],
    items: &[InternalItem],
) -> Result<String, Failure> {
    let assets = items
        .iter()
        .map(|item| (item.wire.source_name.as_str(), item.wire.asset.as_ref()))
        .collect::<BTreeMap<_, _>>();
    let entries = scan
        .iter()
        .map(|entry| {
            let asset = assets.get(entry.name.as_str()).and_then(|asset| *asset);
            SourceManifestEntry {
                name: &entry.name,
                kind: entry_kind_name(entry.kind),
                sha256: asset.map(|asset| asset.sha256.as_str()),
                byte_len: asset.map(|asset| asset.byte_len),
            }
        })
        .collect();
    hash_canonical(&SourceManifest { entries })
}

#[derive(Serialize)]
struct PlanSeal<'a> {
    basis_head_json: &'a str,
    locale: &'a str,
    source_manifest_sha256: &'a str,
    items: Vec<&'a PlanItemWire>,
}

fn hash_plan(
    basis_head_json: &str,
    locale: &str,
    source_manifest_sha256: &str,
    items: &[InternalItem],
) -> Result<String, Failure> {
    hash_canonical(&PlanSeal {
        basis_head_json,
        locale,
        source_manifest_sha256,
        items: items.iter().map(|item| &item.wire).collect(),
    })
}

fn hash_canonical(value: &impl Serialize) -> Result<String, Failure> {
    let bytes = serde_json::to_vec(value).map_err(|_| invariant())?;
    Ok(format_digest(Sha256::digest(bytes).into()))
}

fn format_digest(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn entry_kind_name(kind: SecureDirectEntryKind) -> &'static str {
    match kind {
        SecureDirectEntryKind::File => "file",
        SecureDirectEntryKind::Directory => "directory",
        SecureDirectEntryKind::Symlink => "symlink",
        SecureDirectEntryKind::Other => "other",
    }
}

fn source_status(error: SecureSourceReadError) -> &'static str {
    match error {
        SecureSourceReadError::Unavailable => "source_unavailable",
        SecureSourceReadError::Unsafe => "source_unsafe",
        SecureSourceReadError::Limit => "source_limit",
        SecureSourceReadError::Changed => "source_changed",
    }
}

fn import_context_status(context: OggImportFailureContext) -> &'static str {
    match context {
        OggImportFailureContext::Store => "target_blocked",
        OggImportFailureContext::SourceMissing => "source_missing",
        OggImportFailureContext::SourceUnavailable => "source_unavailable",
        OggImportFailureContext::SourceUnsafe => "source_unsafe",
        OggImportFailureContext::SourceLimit => "source_limit",
        OggImportFailureContext::SourceInvalid => "source_invalid",
        OggImportFailureContext::SourceChanged => "source_changed",
    }
}

fn parse_exact_wire<P>(input: &str, command: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.is_empty() || input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_INPUT_LIMIT",
            "the Voice batch request is empty or exceeds its bounded transport limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != command {
        return Err(invalid_request());
    }
    if serde_json::to_string(&request).map_err(|_| invariant())? != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_common_payload(payload: &PlanPayload) -> Result<(), Failure> {
    for path in [&payload.root, &payload.game_root, &payload.source_folder] {
        if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
            return Err(invalid_request());
        }
    }
    if payload.current_project_json.is_empty()
        || payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES
        || payload.expected_head_json.is_empty()
        || payload.expected_head_json.len() > MAX_HEAD_JSON_BYTES
        || payload.locale.is_empty()
        || payload.locale.len() > 35
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_INPUT_LIMIT",
            "one Voice batch input is empty or exceeds its bounded transport limit",
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), Failure> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    if canonical_head_json(&head)? != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_HEAD_INVALID",
            "expected_head_json is not exact canonical JSON",
        ));
    }
    Ok(head)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    serde_json::to_string(head).map_err(|_| invariant())
}

struct DisjointRootGuards {
    store: SecureDirectDirectory,
    game: SecureDirectDirectory,
}

impl DisjointRootGuards {
    fn revalidate(&self) -> Result<(), Failure> {
        self.store.revalidate().map_err(map_root_binding_failure)?;
        self.game.revalidate().map_err(map_root_binding_failure)
    }
}

fn ensure_roots_pairwise_disjoint(
    store_root: &Path,
    game_root: &Path,
    source_directory: &SecureDirectDirectory,
) -> Result<DisjointRootGuards, Failure> {
    let store_guard = SecureDirectDirectory::open(store_root).map_err(map_root_binding_failure)?;
    let semantic_game_root = gore_mod::semantic_install_root(game_root);
    let game_guard =
        SecureDirectDirectory::open(&semantic_game_root).map_err(map_root_binding_failure)?;
    let store = store_guard
        .canonical_path_bound_to_identity()
        .map_err(map_root_binding_failure)?;
    let game = game_guard
        .canonical_path_bound_to_identity()
        .map_err(map_root_binding_failure)?;
    let source = source_directory
        .canonical_path_bound_to_identity()
        .map_err(map_root_binding_failure)?;
    let retained_overlap = store_guard
        .overlaps_retained_path(&game_guard)
        .map_err(map_root_binding_failure)?
        || store_guard
            .overlaps_retained_path(source_directory)
            .map_err(map_root_binding_failure)?
        || game_guard
            .overlaps_retained_path(source_directory)
            .map_err(map_root_binding_failure)?;
    if retained_overlap
        || paths_overlap(&store, &game)
        || paths_overlap(&store, &source)
        || paths_overlap(&game, &source)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_ROOT_OVERLAP",
            "Store, semantic game installation, and source folder must be pairwise disjoint",
        ));
    }
    Ok(DisjointRootGuards {
        store: store_guard,
        game: game_guard,
    })
}

#[cfg(not(windows))]
fn paths_overlap(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

#[cfg(windows)]
fn paths_overlap(left: &Path, right: &Path) -> bool {
    fn components(path: &Path) -> Vec<String> {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect()
    }
    let left = components(left);
    let right = components(right);
    left.starts_with(&right) || right.starts_with(&left)
}

fn require_current_basis(
    store: &WorkingProjectStore,
    basis: &gore_authoring::OpenedRevision3Checkpoint,
) -> Result<(), Failure> {
    let current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current.head != basis.head || current.project != basis.project {
        return Err(head_conflict());
    }
    Ok(())
}

fn max_ogg_bytes() -> Result<u64, Failure> {
    u64::try_from(WorkingStoreLimits::default().max_ogg_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_STORE_LIMIT",
            "the supported Ogg byte limit cannot be represented",
        )
    })
}

fn map_source_failure(error: SecureSourceReadError) -> Failure {
    let (code, message) = match error {
        SecureSourceReadError::Unavailable => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNAVAILABLE",
            "the source folder or one selected source could not be read",
        ),
        SecureSourceReadError::Unsafe => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNSAFE",
            "the source folder is not one safe no-follow directory capability",
        ),
        SecureSourceReadError::Limit => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT",
            "the source folder exceeds a bounded resource limit",
        ),
        SecureSourceReadError::Changed => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED",
            "the source folder changed while it was read",
        ),
    };
    Failure::new(code, message)
}

fn map_root_binding_failure(error: SecureSourceReadError) -> Failure {
    let (code, message) = match error {
        SecureSourceReadError::Unavailable => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNAVAILABLE",
            "one required root directory could not be safely opened",
        ),
        SecureSourceReadError::Unsafe => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNSAFE",
            "one required root directory is not a safe no-follow capability",
        ),
        SecureSourceReadError::Limit => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT",
            "one required root directory exceeds a bounded resource limit",
        ),
        SecureSourceReadError::Changed => (
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED",
            "one required root directory changed while its identity was bound",
        ),
    };
    Failure::new(code, message)
}

fn map_prepare_ogg_failure(error: gore_authoring::OggImportError) -> Failure {
    if error.context() == OggImportFailureContext::Store {
        map_store_error(error.into_store_error())
    } else {
        source_changed()
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    use WorkingStoreError::*;
    let code = match error {
        HeadConflict { .. } | MissingHead(_) => "AUTHORING_REVISION3_VOICE_BATCH_HEAD_CONFLICT",
        MissingRoot(_) => "AUTHORING_REVISION3_VOICE_BATCH_STORE_MISSING",
        MissingObject(_) => "AUTHORING_REVISION3_VOICE_BATCH_STORE_OBJECT_MISSING",
        UnsafePath { .. } => "AUTHORING_REVISION3_VOICE_BATCH_STORE_UNSAFE",
        LimitExceeded { .. } | InvalidLimits(_) => "AUTHORING_REVISION3_VOICE_BATCH_STORE_LIMIT",
        SealMismatch { .. } => "AUTHORING_REVISION3_VOICE_BATCH_STORE_SEAL_MISMATCH",
        Collision { .. } => "AUTHORING_REVISION3_VOICE_BATCH_STORE_COLLISION",
        InvalidJson { .. } | NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_VOICE_BATCH_STORE_JSON_INVALID"
        }
        Invariant(_) | InvalidOgg(_) | OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_BATCH_STORE_INVARIANT"
        }
        StagingCleanup { .. } | Io(_) => "AUTHORING_REVISION3_VOICE_BATCH_STORE_IO",
    };
    Failure::new(code, "the managed Voice working-store operation failed")
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    if serde_json::to_vec(&response)
        .map_err(|_| invariant())?
        .len()
        > MAX_RESPONSE_BYTES
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BATCH_RESPONSE_LIMIT",
            "the Voice batch response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BATCH_REQUEST_INVALID",
        "request must be exact canonical JSON for one supported Voice batch command",
    )
}

fn source_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED",
        "the source folder or one selected Ogg changed during preparation",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BATCH_HEAD_CONFLICT",
        "the exact published revision-3 project changed during Voice batch work",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BATCH_INVARIANT",
        "the Voice batch operation could not preserve its exact internal contract",
    )
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef,
        SchemaRevisionV3, TypedRef,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        Sha256Digest,
    };
    use tempfile::TempDir;

    use super::*;

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x10; 16])
    }

    fn origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
        }
    }

    fn basis_project() -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision: 7,
            meta: ProjectMeta {
                name: "Voice folder".into(),
                version: "1.0.0".into(),
                author: "tests".into(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 171_698_176,
                    sha256: Sha256Digest::from_bytes([0x21; 32]),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::from([
                (
                    id(2),
                    Entity {
                        id: id(2),
                        display_name: "Asghan text".into(),
                        origin: origin(2),
                        revision: 4,
                        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: "GRD_263_ASGHAN_OPEN_INFO_06_02".into(),
                            texts: BTreeMap::new(),
                        }),
                    },
                ),
                (
                    id(3),
                    Entity {
                        id: id(3),
                        display_name: "Asghan greeting".into(),
                        origin: origin(3),
                        revision: 2,
                        payload: EntityPayload::DialogLine(DialogLine {
                            localization: TypedRef::new(
                                project_id(),
                                id(2),
                                EntityKind::LocalizationEntry,
                            ),
                            speaker_hint: Some("Asghan".into()),
                            voice_slots: BTreeMap::new(),
                        }),
                    },
                ),
            ]),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn two_line_basis_project() -> ProjectRevision3 {
        let mut project = basis_project();
        project.entities.insert(
            id(6),
            Entity {
                id: id(6),
                display_name: "Viper text".into(),
                origin: origin(6),
                revision: 1,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "VLK_574_VIPER_HELLO_01_01".into(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        project.entities.insert(
            id(7),
            Entity {
                id: id(7),
                display_name: "Viper greeting".into(),
                origin: origin(7),
                revision: 1,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(project_id(), id(6), EntityKind::LocalizationEntry),
                    speaker_hint: Some("Viper".into()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        project
    }

    struct Fixture {
        store_root: TempDir,
        game_root: TempDir,
        source_root: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        head_bytes: Vec<u8>,
    }

    impl Fixture {
        fn new(source_name: &str, source_bytes: &[u8]) -> Self {
            Self::with_project_and_sources(basis_project(), &[(source_name, source_bytes)])
        }

        fn with_project_and_sources(project: ProjectRevision3, sources: &[(&str, &[u8])]) -> Self {
            let store_root = TempDir::new().unwrap();
            let game_root = TempDir::new().unwrap();
            let source_root = TempDir::new().unwrap();
            for (source_name, source_bytes) in sources {
                fs::write(source_root.path().join(source_name), source_bytes).unwrap();
            }
            let store =
                WorkingProjectStore::at(store_root.path(), WorkingStoreLimits::default()).unwrap();
            let project_json = project.to_canonical_json().unwrap();
            let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
            fs::write(
                store_root.path().join("gore-project.json"),
                &prepared.head_bytes,
            )
            .unwrap();
            Self {
                store_root,
                game_root,
                source_root,
                project,
                project_json,
                head: prepared.head,
                head_bytes: prepared.head_bytes,
            }
        }

        fn plan_payload(&self) -> PlanPayload {
            PlanPayload {
                current_project_json: self.project_json.clone(),
                expected_head_json: serde_json::to_string(&self.head).unwrap(),
                game_root: self.game_root.path().display().to_string(),
                locale: "de".into(),
                root: self.store_root.path().display().to_string(),
                source_folder: self.source_root.path().display().to_string(),
            }
        }

        fn plan_wire(&self) -> String {
            serde_json::to_string(&ExactWireRequest {
                command: PLAN_COMMAND.to_owned(),
                payload: self.plan_payload(),
            })
            .unwrap()
        }
    }

    fn prepare_wire(fixture: &Fixture, plan_response: &Value) -> String {
        let plan = fixture.plan_payload();
        serde_json::to_string(&ExactWireRequest {
            command: PREPARE_COMMAND.to_owned(),
            payload: PreparePayload {
                current_project_json: plan.current_project_json,
                expected_head_json: plan.expected_head_json,
                game_root: plan.game_root,
                locale: plan.locale,
                root: plan.root,
                source_folder: plan.source_folder,
                expected_source_manifest_sha256: plan_response["source_manifest_sha256"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                expected_plan_sha256: plan_response["plan_sha256"].as_str().unwrap().to_owned(),
            },
        })
        .unwrap()
    }

    fn snapshot_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &path, output);
                } else {
                    output.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut output = BTreeMap::new();
        visit(root, root, &mut output);
        output
    }

    #[test]
    fn valid_folder_plan_is_read_only_deterministic_and_friendly() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        fs::write(fixture.source_root.path().join("notes.txt"), b"ignored").unwrap();
        let before = snapshot_files(fixture.store_root.path());
        let first = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        let second = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        assert_eq!(first, second);
        assert_eq!(first["ok"], true);
        assert_eq!(first["status"], "ready");
        assert_eq!(first["ready_count"], 1);
        assert_eq!(first["ignored_entry_count"], 1);
        assert_eq!(first["items"][0]["line_display_name"], "Asghan greeting");
        assert_eq!(first["items"][0]["speaker"], "Asghan");
        assert_eq!(first["items"][0]["status"], "ready");
        assert_eq!(
            first["items"][0]["asset"]["logical_name"],
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"
        );
        assert_eq!(snapshot_files(fixture.store_root.path()), before);

        let through_dispatch: Value =
            serde_json::from_str(&crate::execute_json(&fixture.plan_wire())).unwrap();
        assert_eq!(through_dispatch, first);
    }

    #[test]
    fn non_ascii_non_ogg_entry_is_ignored_without_panicking() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        fs::write(fixture.source_root.path().join("éabc"), b"ignored").unwrap();
        let response = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        assert_eq!(response["ok"], true);
        assert_eq!(response["status"], "ready");
        assert_eq!(response["scanned_entry_count"], 2);
        assert_eq!(response["ogg_file_count"], 1);
        assert_eq!(response["ignored_entry_count"], 1);
    }

    #[test]
    fn retained_root_identity_rejects_a_source_nested_inside_the_store() {
        let store = TempDir::new().unwrap();
        let game = TempDir::new().unwrap();
        let source_path = store.path().join("voice-source");
        fs::create_dir(&source_path).unwrap();
        let source = SecureDirectDirectory::open(&source_path).unwrap();
        let failure = match ensure_roots_pairwise_disjoint(store.path(), game.path(), &source) {
            Ok(_) => panic!("nested retained directory paths must overlap"),
            Err(failure) => failure,
        };
        assert_eq!(failure.code, "AUTHORING_REVISION3_VOICE_BATCH_ROOT_OVERLAP");
    }

    #[test]
    fn project_and_file_count_work_product_is_rejected_before_item_analysis() {
        let mut project = basis_project();
        let EntityPayload::LocalizationEntry(localization) =
            &mut project.entities.get_mut(&id(2)).unwrap().payload
        else {
            unreachable!()
        };
        localization
            .texts
            .insert("de".parse().unwrap(), "x".repeat(256 * 1024));
        let fixture = Fixture::with_project_and_sources(
            project,
            &[(
                "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
                include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            )],
        );
        for index in 0..255 {
            fs::write(
                fixture
                    .source_root
                    .path()
                    .join(format!("UNMATCHED_{index:03}.ogg")),
                include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            )
            .unwrap();
        }
        assert!(fixture.project_json.len() * 256 > MAX_REVISION3_VOICE_BATCH_PROJECT_WORK_BYTES_V1);
        let before = snapshot_files(fixture.store_root.path());
        let response = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BATCH_INPUT_LIMIT"
        );
        assert_eq!(snapshot_files(fixture.store_root.path()), before);
    }

    #[test]
    fn prepare_builds_one_reopenable_candidate_without_publishing() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        let plan = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        let response = prepare_revision3_voice_batch_v1_raw(&prepare_wire(&fixture, &plan));
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["revision"], 8);
        assert_eq!(response["imported_count"], 1);
        assert_eq!(response["items"][0]["take_status"], "recorded");
        assert_eq!(response["items"][0]["selected"], false);

        assert_eq!(
            fs::read(fixture.store_root.path().join("gore-project.json")).unwrap(),
            fixture.head_bytes
        );
        let store = WorkingProjectStore::open_existing(
            fixture.store_root.path(),
            WorkingStoreLimits::default(),
        )
        .unwrap();
        let candidate = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(candidate.project.revision, 8);
        assert_eq!(
            candidate.project.entities.len(),
            fixture.project.entities.len() + 2
        );
        let current = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(current.head, fixture.head);
        assert_eq!(current.project, fixture.project);
    }

    #[test]
    fn already_present_folder_is_a_read_only_no_change_plan() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        let first_plan = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        let first_prepare =
            prepare_revision3_voice_batch_v1_raw(&prepare_wire(&fixture, &first_plan));
        assert_eq!(first_prepare["ok"], true);

        let candidate_head_json = first_prepare["head_json"].as_str().unwrap();
        fs::write(
            fixture.store_root.path().join("gore-project.json"),
            candidate_head_json.as_bytes(),
        )
        .unwrap();
        let plan_payload = PlanPayload {
            current_project_json: first_prepare["project_json"].as_str().unwrap().to_owned(),
            expected_head_json: candidate_head_json.to_owned(),
            game_root: fixture.game_root.path().display().to_string(),
            locale: "de".into(),
            root: fixture.store_root.path().display().to_string(),
            source_folder: fixture.source_root.path().display().to_string(),
        };
        let plan_wire = serde_json::to_string(&ExactWireRequest {
            command: PLAN_COMMAND.to_owned(),
            payload: plan_payload.clone(),
        })
        .unwrap();
        let before = snapshot_files(fixture.store_root.path());
        let no_change_plan = plan_revision3_voice_batch_v1_raw(&plan_wire);
        assert_eq!(no_change_plan["ok"], true);
        assert_eq!(no_change_plan["status"], "no_changes");
        assert_eq!(no_change_plan["ready_count"], 0);
        assert_eq!(no_change_plan["already_present_count"], 1);
        assert_eq!(no_change_plan["blocked_count"], 0);
        assert_eq!(no_change_plan["items"][0]["status"], "already_present");
        assert_eq!(snapshot_files(fixture.store_root.path()), before);

        let prepare_wire = serde_json::to_string(&ExactWireRequest {
            command: PREPARE_COMMAND.to_owned(),
            payload: PreparePayload {
                current_project_json: plan_payload.current_project_json,
                expected_head_json: plan_payload.expected_head_json,
                game_root: plan_payload.game_root,
                locale: plan_payload.locale,
                root: plan_payload.root,
                source_folder: plan_payload.source_folder,
                expected_source_manifest_sha256: no_change_plan["source_manifest_sha256"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                expected_plan_sha256: no_change_plan["plan_sha256"].as_str().unwrap().to_owned(),
            },
        })
        .unwrap();
        let response = prepare_revision3_voice_batch_v1_raw(&prepare_wire);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BATCH_NOT_READY"
        );
        assert_eq!(snapshot_files(fixture.store_root.path()), before);
    }

    #[test]
    fn two_ready_files_are_one_revision_and_one_complete_candidate() {
        let fixture = Fixture::with_project_and_sources(
            two_line_basis_project(),
            &[
                (
                    "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
                    include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
                ),
                (
                    "VLK_574_VIPER_HELLO_01_01.ogg",
                    include_bytes!("../../gore-vo/testdata/tiny-opus.ogg"),
                ),
            ],
        );
        let plan = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        assert_eq!(plan["status"], "ready");
        assert_eq!(plan["ready_count"], 2);
        assert_eq!(
            plan["items"][0]["source_name"],
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"
        );
        assert_eq!(
            plan["items"][1]["source_name"],
            "VLK_574_VIPER_HELLO_01_01.ogg"
        );
        let response = prepare_revision3_voice_batch_v1_raw(&prepare_wire(&fixture, &plan));
        assert_eq!(response["ok"], true);
        assert_eq!(response["revision"], 8);
        assert_eq!(response["imported_count"], 2);
        assert_eq!(response["items"].as_array().unwrap().len(), 2);

        let candidate =
            ProjectRevision3::from_json(response["project_json"].as_str().unwrap()).unwrap();
        assert_eq!(candidate.revision, 8);
        assert_eq!(candidate.entities.len(), fixture.project.entities.len() + 4);
        assert_eq!(
            fs::read(fixture.store_root.path().join("gore-project.json")).unwrap(),
            fixture.head_bytes
        );
    }

    #[test]
    fn changed_source_invalidates_reviewed_seals_before_any_cas_write() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        let plan = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        fs::write(
            fixture
                .source_root
                .path()
                .join("GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"),
            include_bytes!("../../gore-vo/testdata/tiny-opus.ogg"),
        )
        .unwrap();
        let before = snapshot_files(fixture.store_root.path());
        let response = prepare_revision3_voice_batch_v1_raw(&prepare_wire(&fixture, &plan));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BATCH_PLAN_CHANGED"
        );
        assert_eq!(snapshot_files(fixture.store_root.path()), before);
    }

    #[test]
    fn source_drift_between_complete_reads_is_rejected_before_cas() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        let plan = plan_revision3_voice_batch_v1_raw(&fixture.plan_wire());
        let request = prepare_wire(&fixture, &plan);
        let source = fixture
            .source_root
            .path()
            .join("GRD_263_ASGHAN_OPEN_INFO_06_02.ogg");
        let before = snapshot_files(fixture.store_root.path());
        let failure = prepare_inner_with_source_guard(&request, || {
            fs::write(
                &source,
                include_bytes!("../../gore-vo/testdata/tiny-opus.ogg"),
            )
            .unwrap();
        })
        .unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED"
        );
        assert_eq!(snapshot_files(fixture.store_root.path()), before);
    }

    #[test]
    fn unmatched_or_invalid_selected_ogg_blocks_the_whole_folder() {
        let unmatched = Fixture::new(
            "UNKNOWN_LINE.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        let plan = plan_revision3_voice_batch_v1_raw(&unmatched.plan_wire());
        assert_eq!(plan["status"], "blocked");
        assert_eq!(plan["items"][0]["status"], "unmatched");
        let before = snapshot_files(unmatched.store_root.path());
        let response = prepare_revision3_voice_batch_v1_raw(&prepare_wire(&unmatched, &plan));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BATCH_NOT_READY"
        );
        assert_eq!(snapshot_files(unmatched.store_root.path()), before);

        let invalid = Fixture::new("GRD_263_ASGHAN_OPEN_INFO_06_02.ogg", b"not ogg");
        let invalid_plan = plan_revision3_voice_batch_v1_raw(&invalid.plan_wire());
        assert_eq!(invalid_plan["status"], "blocked");
        assert_eq!(invalid_plan["items"][0]["status"], "source_invalid");
        assert!(invalid_plan["items"][0]["asset"].is_null());
        assert!(invalid_plan["items"][0]["ogg"].is_null());
    }

    #[test]
    fn exact_wire_rejects_unknown_fields_and_noncanonical_order() {
        let fixture = Fixture::new(
            "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        );
        let valid: Value = serde_json::from_str(&fixture.plan_wire()).unwrap();
        let mut payload = valid["payload"].clone();
        payload["publish"] = json!(true);
        let forged = json!({"command": PLAN_COMMAND, "payload": payload}).to_string();
        assert_eq!(
            plan_revision3_voice_batch_v1_raw(&forged)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BATCH_REQUEST_INVALID"
        );

        let noncanonical = format!(
            "{{\"payload\":{},\"command\":\"{PLAN_COMMAND}\"}}",
            valid["payload"]
        );
        assert_eq!(
            plan_revision3_voice_batch_v1_raw(&noncanonical)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BATCH_REQUEST_INVALID"
        );
    }
}
