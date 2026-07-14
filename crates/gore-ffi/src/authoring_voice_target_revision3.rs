//! Prepare-only resolution of one revision-3 VoiceSlot against an installed voice archive.
//!
//! Client input carries only exact project/head/line/slot/locale intent. Native code derives every
//! archive match from one bounded no-follow `gore-vo` snapshot, applies the pure authoring
//! transaction, verifies the archive seal again, and fully reopens an unpublished Store candidate.
//! It never accepts target evidence from clients, creates an Add target, writes the installation,
//! deploys, or publishes the fixed project head.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use gore_asset::dataasset_workflow::{read_verified_file_bounded, MAX_GAME_EXECUTABLE_BYTES};
use gore_authoring::model_revision3::{
    EntityKind, EntityPayload, VoiceMemberProof, VoiceOperation, VoiceTarget,
};
use gore_authoring::{
    apply_revision3_voice_target_resolution_transaction_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, ArchiveSeal, AssetVerification, EntityId,
    GameGenerationAnchor, LocaleCode, ProjectId, Revision3VoiceTargetResolutionConflictV1,
    Revision3VoiceTargetResolutionErrorV1, Revision3VoiceTargetResolutionEvaluationV1,
    Revision3VoiceTargetResolutionRequestV1, Revision3VoiceTargetResolutionStateV1, Sha256Digest,
    WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_VOICE_TARGET_ARCHIVE_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_MATCHES_V1, MAX_REVISION3_VOICE_TARGET_MEMBER_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1,
};
use gore_mod::{bind_voice_over_root, semantic_install_root, VoiceOverPathGuard};
use gore_vo::{
    validate_archive_entry_path, ArchiveEntry, ArchiveIndex, ArchiveSeal as VoArchiveSeal,
    Error as VoiceArchiveError, Limits as VoiceArchiveLimits,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_voice_target_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_TARGET_INTENT_JSON_BYTES: usize = 64 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_ARCHIVE_ENTRIES: usize = 50_000;
const MAX_CENTRAL_DIRECTORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRY_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_WIRE_BYTES: usize =
    MAX_PROJECT_JSON_BYTES * 6 + MAX_TARGET_INTENT_JSON_BYTES * 6 + MAX_PATH_BYTES * 12 + 8 * 1024;

const ENGLISH_ARCHIVES: &[&str] = &["english_newer", "english_new", "english"];
const GERMAN_ARCHIVES: &[&str] = &["german_new", "german"];
const FRENCH_ARCHIVES: &[&str] = &["french"];
const ITALIAN_ARCHIVES: &[&str] = &["italian"];
const SPANISH_ARCHIVES: &[&str] = &["spanish"];
const POLISH_ARCHIVES: &[&str] = &["polish"];
const RUSSIAN_ARCHIVES: &[&str] = &["russian"];
const JAPANESE_ARCHIVES: &[&str] = &["japanese"];
const SIMPLIFIED_CHINESE_ARCHIVES: &[&str] = &["schinese"];
const BRAZILIAN_PORTUGUESE_ARCHIVES: &[&str] = &["brazilian"];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareVoiceTargetWirePayload {
    current_project_json: String,
    game_root: String,
    root: String,
    voice_target_request_json: String,
}

/// Exact external intent. Native archive matches are deliberately absent from this wire type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoiceTargetIntentV1 {
    expected_head: WorkingHead,
    expected_project_id: ProjectId,
    expected_revision: u64,
    expected_target: GameGenerationAnchor,
    line_id: EntityId,
    slot_id: EntityId,
    locale: LocaleCode,
    expected_loc_id: String,
}

impl VoiceTargetIntentV1 {
    fn from_canonical_json(value: &str) -> Result<Self, Failure> {
        if value.is_empty() || value.len() > MAX_TARGET_INTENT_JSON_BYTES {
            return Err(invalid_request());
        }
        let request: Self = serde_json::from_str(value).map_err(|_| invalid_request())?;
        let canonical = serde_json::to_string(&request).map_err(|_| invalid_request())?;
        if canonical.as_bytes() != value.as_bytes() {
            return Err(invalid_request());
        }
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetRootGuard {
    store: PathBuf,
    install: PathBuf,
}

#[derive(Debug, Clone)]
struct SelectedArchive {
    path: PathBuf,
    canonical_path: PathBuf,
    name: String,
    drifted: bool,
    voice_root: VoiceOverPathGuard,
}

#[derive(Debug)]
struct NativeArchiveEvidence {
    selected: SelectedArchive,
    seal: VoArchiveSeal,
    matches: Vec<VoiceTarget>,
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

pub(super) fn prepare_revision3_voice_target_v1_raw(input: &str) -> Value {
    prepare_revision3_voice_target_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_voice_target_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_voice_target_v1_inner_with_archive_guard(input, || {})
}

fn prepare_revision3_voice_target_v1_inner_with_archive_guard<F>(
    input: &str,
    after_first_archive_open: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    let payload: PrepareVoiceTargetWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    let root_guard = ensure_store_and_install_are_disjoint(
        Path::new(&payload.root),
        Path::new(&payload.game_root),
    )?;
    let store = WorkingProjectStore::open_existing(&root_guard.store, ffi_store_limits())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revision(basis.project.revision)?;
    require_signed_serializable(&basis.project)?;
    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let intent = VoiceTargetIntentV1::from_canonical_json(&payload.voice_target_request_json)?;
    require_signed_serializable(&intent)?;
    validate_intent_basis(&intent, &basis.head, &basis.project)?;
    let actual_loc_id = exact_slot_loc_id(&basis.project, &intent)?;
    validate_voice_basename_stem(&actual_loc_id)?;

    require_installed_executable_generation(&root_guard.install, &basis.project.target)?;

    let native_evidence =
        inspect_installed_archive(&root_guard.install, &intent.locale, &actual_loc_id)?;
    let matches = native_evidence
        .as_ref()
        .map(|evidence| evidence.matches.clone())
        .unwrap_or_default();
    let core_request = Revision3VoiceTargetResolutionRequestV1 {
        expected_head: intent.expected_head.clone(),
        expected_project_id: intent.expected_project_id,
        expected_revision: intent.expected_revision,
        expected_target: intent.expected_target.clone(),
        line_id: intent.line_id,
        slot_id: intent.slot_id,
        locale: intent.locale.clone(),
        expected_loc_id: actual_loc_id.clone(),
        matches,
    };
    let core_request_json = core_request
        .to_canonical_json()
        .map_err(map_core_request_error)?;
    if core_request_json.len() > MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_LIMIT",
            "native Voice target evidence exceeds the bounded core request limit",
        ));
    }

    let outcome = match apply_revision3_voice_target_resolution_transaction_v1(
        &basis.head,
        &canonical_basis,
        &core_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3VoiceTargetResolutionEvaluationV1::Applied(outcome) => *outcome,
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };

    after_first_archive_open();
    revalidate_archive_snapshot(
        &root_guard.install,
        &intent.locale,
        native_evidence.as_ref(),
    )?;
    require_installed_executable_generation(&root_guard.install, &basis.project.target)?;
    revalidate_target_root_guard(
        &root_guard,
        Path::new(&payload.root),
        Path::new(&payload.game_root),
    )?;

    let current_before_prepare = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current_before_prepare.head != basis.head || current_before_prepare.project != basis.project
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT",
            "the published revision-3 project changed during Voice target preparation",
        ));
    }

    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_INVARIANT",
            "the prepared revision-3 Voice target checkpoint did not reopen exactly",
        ));
    }
    revalidate_archive_snapshot(
        &root_guard.install,
        &intent.locale,
        native_evidence.as_ref(),
    )?;
    require_installed_executable_generation(&root_guard.install, &basis.project.target)?;
    revalidate_target_root_guard(
        &root_guard,
        Path::new(&payload.root),
        Path::new(&payload.game_root),
    )?;

    let current_after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current_after.head != basis.head || current_after.project != basis.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT",
            "the published revision-3 project changed before Voice target preparation completed",
        ));
    }

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_INVARIANT",
            "the prepared revision-3 Voice target head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_RESPONSE_LIMIT",
            "the prepared revision-3 Voice target head exceeds its transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;

    let resolution = match outcome.resolution_state {
        Revision3VoiceTargetResolutionStateV1::Unresolved => "unresolved",
        Revision3VoiceTargetResolutionStateV1::Ambiguous => "ambiguous",
        Revision3VoiceTargetResolutionStateV1::Resolved => "resolved",
    };
    let target_resolution = serde_json::to_value(&outcome.resolution).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
            "the Voice target resolution could not be serialized",
        )
    })?;
    let archive_observation = native_evidence.as_ref().map(|evidence| {
        json!({
            "archive": evidence.selected.name,
            "archive_seal": {
                "byte_len": evidence.seal.size,
                "sha256": format_sha256(evidence.seal.sha256),
            },
        })
    });
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "revision": outcome.project.revision,
        "line_id": outcome.line_id.to_string(),
        "localization_id": outcome.localization_id.to_string(),
        "slot_id": outcome.slot_id.to_string(),
        "locale": outcome.locale.to_string(),
        "loc_id": outcome.loc_id,
        "resolution": resolution,
        "match_count": outcome.match_count,
        "target_resolution": target_resolution,
        "archive_observation": archive_observation,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    let final_current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if final_current.head != basis.head || final_current.project != basis.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT",
            "the published revision-3 project changed before the Voice target response was returned",
        ));
    }
    // The response certifies this exact native archive snapshot, not merely whichever path was
    // selected earlier. Re-run deployment's pristine-source selection and the generation/root
    // guards immediately before returning the unpublished candidate.
    revalidate_archive_snapshot(
        &root_guard.install,
        &intent.locale,
        native_evidence.as_ref(),
    )?;
    require_installed_executable_generation(&root_guard.install, &basis.project.target)?;
    revalidate_target_root_guard(
        &root_guard,
        Path::new(&payload.root),
        Path::new(&payload.game_root),
    )?;
    Ok(response)
}

fn require_installed_executable_generation(
    install_root: &Path,
    expected: &GameGenerationAnchor,
) -> Result<(), Failure> {
    let executable = install_root
        .join("G1R")
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let verified = read_verified_file_bounded(
        &executable,
        MAX_GAME_EXECUTABLE_BYTES,
        "AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE",
    )
    .map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_UNAVAILABLE",
            "the installed game executable could not be read and sealed safely",
        )
    })?;
    if verified.length() != expected.executable.byte_len
        || verified.sha256() != expected.executable.sha256.as_bytes()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH",
            "the installed game executable does not match the project's exact generation",
        ));
    }
    Ok(())
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_INPUT_LIMIT",
            format!("revision-3 Voice target request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invalid_request())?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareVoiceTargetWirePayload) -> Result<(), Failure> {
    for path in [&payload.game_root, &payload.root] {
        if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
            return Err(invalid_request());
        }
    }
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.voice_target_request_json.is_empty()
        || payload.voice_target_request_json.len() > MAX_TARGET_INTENT_JSON_BYTES
    {
        return Err(invalid_request());
    }
    Ok(())
}

fn ensure_store_and_install_are_disjoint(
    store_root: &Path,
    game_root: &Path,
) -> Result<TargetRootGuard, Failure> {
    let store_root = canonical_existing_directory_no_reparse(
        store_root,
        "AUTHORING_REVISION3_VOICE_TARGET_STORE_PATH_UNSAFE",
        "revision-3 working-store root",
    )?;
    let install_root = canonical_existing_directory_no_reparse(
        &semantic_install_root(game_root),
        "AUTHORING_REVISION3_VOICE_TARGET_GAME_ROOT_UNAVAILABLE",
        "configured game installation root",
    )?;
    if store_root.starts_with(&install_root) || install_root.starts_with(&store_root) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_GAME_ALIAS",
            "the working-store root and configured game installation must be disjoint",
        ));
    }
    Ok(TargetRootGuard {
        store: store_root,
        install: install_root,
    })
}

fn revalidate_target_root_guard(
    expected: &TargetRootGuard,
    store_root: &Path,
    game_root: &Path,
) -> Result<(), Failure> {
    let actual = ensure_store_and_install_are_disjoint(store_root, game_root)?;
    if &actual != expected {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ROOT_CHANGED",
            "the working-store or configured game installation identity changed during Voice target preparation",
        ));
    }
    Ok(())
}

fn canonical_existing_directory_no_reparse(
    path: &Path,
    code: &'static str,
    label: &str,
) -> Result<PathBuf, Failure> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(Failure::new(
            code,
            format!("{label} must not contain '..' traversal"),
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| Failure::new(code, format!("{label} could not be resolved")))?
            .join(path)
    };
    for ancestor in absolute.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| {
            Failure::new(code, format!("{label} has an unavailable path component"))
        })?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            return Err(Failure::new(
                code,
                format!("{label} crosses a symbolic link, reparse point, or non-directory"),
            ));
        }
    }
    fs::canonicalize(&absolute)
        .map_err(|_| Failure::new(code, format!("{label} could not be canonicalized")))
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn validate_intent_basis(
    intent: &VoiceTargetIntentV1,
    basis_head: &WorkingHead,
    project: &gore_authoring::ProjectRevision3,
) -> Result<(), Failure> {
    if intent.expected_head != *basis_head {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT",
            "the Voice target request head differs from the exact published revision-3 head",
        ));
    }
    if intent.expected_project_id != project.project_id
        || intent.expected_revision != project.revision
        || intent.expected_target != project.target
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT",
            "the Voice target request identity, revision, or target differs from the exact published revision-3 project",
        ));
    }
    Ok(())
}

fn exact_slot_loc_id(
    project: &gore_authoring::ProjectRevision3,
    intent: &VoiceTargetIntentV1,
) -> Result<String, Failure> {
    let Some(line_entity) = project.entities.get(&intent.line_id) else {
        return Err(project_invalid("the requested DialogLine is missing"));
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        return Err(project_invalid("the requested entity is not a DialogLine"));
    };
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
    {
        return Err(project_invalid(
            "the requested DialogLine has an invalid LocalizationEntry reference",
        ));
    }
    let Some(localization_entity) = project.entities.get(&line.localization.id) else {
        return Err(project_invalid(
            "the requested DialogLine LocalizationEntry is missing",
        ));
    };
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        return Err(project_invalid(
            "the requested DialogLine localization reference has the wrong kind",
        ));
    };
    if localization.loc_id != intent.expected_loc_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT",
            "expected_loc_id differs from the exact DialogLine LocalizationEntry",
        ));
    }
    let Some(slot_ref) = line.voice_slots.get(&intent.locale) else {
        return Err(project_invalid(
            "the requested DialogLine locale has no VoiceSlot",
        ));
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != intent.slot_id
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT",
            "the requested line/locale is linked to a different VoiceSlot",
        ));
    }
    let Some(slot_entity) = project.entities.get(&intent.slot_id) else {
        return Err(project_invalid("the requested VoiceSlot is missing"));
    };
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        return Err(project_invalid("the requested entity is not a VoiceSlot"));
    };
    if slot.locale != intent.locale {
        return Err(project_invalid(
            "the requested VoiceSlot locale differs from the line locale",
        ));
    }
    Ok(localization.loc_id.clone())
}

fn validate_voice_basename_stem(loc_id: &str) -> Result<(), Failure> {
    validate_revision3_voice_loc_id_basename_stem_v1(loc_id).map_err(|error| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_LOC_ID_INVALID",
            format!(
                "the exact LocalizationEntry ID is not one bounded portable archive basename stem: {error}"
            ),
        )
    })
}

fn inspect_installed_archive(
    install_root: &Path,
    locale: &LocaleCode,
    loc_id: &str,
) -> Result<Option<NativeArchiveEvidence>, Failure> {
    let Some(selected) = first_existing_locale_archive(install_root, locale)? else {
        return Ok(None);
    };
    let limits = voice_archive_limits();
    let archive = ArchiveIndex::open(&selected.path, limits).map_err(map_archive_error)?;
    let seal = archive.seal();
    if seal.size == 0 || seal.sha256.iter().all(|byte| *byte == 0) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_INVALID",
            "the installed Voice archive has an invalid zero seal",
        ));
    }
    let expected_basename = format!("{loc_id}.ogg");
    let archive_seal = ArchiveSeal {
        byte_len: seal.size,
        sha256: Sha256Digest::from_bytes(seal.sha256),
    };
    let mut matches = Vec::new();
    for entry in archive.list() {
        if !ascii_case_equal(&entry.basename, &expected_basename) {
            continue;
        }
        validate_matching_entry(entry, &limits)?;
        if matches.len() == MAX_REVISION3_VOICE_TARGET_MATCHES_V1 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_LIMIT",
                format!(
                    "installed Voice archive has more than {} exact line matches",
                    MAX_REVISION3_VOICE_TARGET_MATCHES_V1
                ),
            ));
        }
        matches.push(VoiceTarget {
            archive: selected.name.clone(),
            member: entry.path.clone(),
            operation: VoiceOperation::Replace,
            archive_seal: archive_seal.clone(),
            member_proof: VoiceMemberProof::Present {
                uncompressed_size: entry.uncompressed_size,
                crc32: entry.crc32,
            },
        });
    }
    let evidence = NativeArchiveEvidence {
        selected,
        seal,
        matches,
    };
    // Bind the first archive parse to the source deployment would currently choose, rather than
    // trusting that its active record and priority selection stayed fixed across the open.
    revalidate_archive_snapshot(install_root, locale, Some(&evidence))?;
    Ok(Some(evidence))
}

fn first_existing_locale_archive(
    install_root: &Path,
    locale: &LocaleCode,
) -> Result<Option<SelectedArchive>, Failure> {
    let stems = archive_stems(locale).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_LOCALE_UNSUPPORTED",
            "Voice target resolution supports only the 10 canonical game locales",
        )
    })?;
    let Some(voice_root_guard) = bind_voice_over_root(install_root).map_err(|error| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE",
            format!("the installed VoiceOver directory could not be bound safely: {error}"),
        )
    })?
    else {
        return Ok(None);
    };
    let voice_root = voice_root_guard.path();
    for stem in stems {
        let name = format!("{stem}.zip");
        if name.len() > MAX_REVISION3_VOICE_TARGET_ARCHIVE_BYTES_V1 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
                "the native locale archive name exceeds the model limit",
            ));
        }
        let live_path = voice_root.join(&name);
        match fs::symlink_metadata(&live_path) {
            Ok(_) => {
                let source = voice_root_guard
                    .resolve_pristine_archive(&name)
                    .map_err(|error| {
                        Failure::new(
                            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE",
                            format!(
                                "the pristine installed Voice archive could not be selected safely: {error}"
                            ),
                        )
                    })?;
                let drifted = source.drifted;
                let path = source.path;
                let canonical_path = fs::canonicalize(&path).map_err(|_| {
                    Failure::new(
                        "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE",
                        "the selected installed Voice archive could not be resolved",
                    )
                })?;
                if canonical_path.parent() != Some(voice_root) {
                    return Err(Failure::new(
                        "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE",
                        "the selected installed Voice archive escapes G1R/Story/VoiceOver",
                    ));
                }
                return Ok(Some(SelectedArchive {
                    path,
                    canonical_path,
                    name,
                    drifted,
                    voice_root: voice_root_guard.clone(),
                }));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE",
                    "the installed Voice archive could not be inspected",
                ));
            }
        }
    }
    Ok(None)
}

fn archive_stems(locale: &LocaleCode) -> Option<&'static [&'static str]> {
    match locale.to_string().as_str() {
        "en" => Some(ENGLISH_ARCHIVES),
        "de" => Some(GERMAN_ARCHIVES),
        "fr" => Some(FRENCH_ARCHIVES),
        "it" => Some(ITALIAN_ARCHIVES),
        "es" => Some(SPANISH_ARCHIVES),
        "pl" => Some(POLISH_ARCHIVES),
        "ru" => Some(RUSSIAN_ARCHIVES),
        "ja" => Some(JAPANESE_ARCHIVES),
        "zh-Hans" => Some(SIMPLIFIED_CHINESE_ARCHIVES),
        "pt-BR" => Some(BRAZILIAN_PORTUGUESE_ARCHIVES),
        _ => None,
    }
}

fn validate_matching_entry(
    entry: &ArchiveEntry,
    limits: &VoiceArchiveLimits,
) -> Result<(), Failure> {
    validate_archive_entry_path(&entry.path, limits).map_err(|_| ineligible_entry())?;
    if entry.path.len() > MAX_REVISION3_VOICE_TARGET_MEMBER_BYTES_V1
        || entry.is_directory
        || entry.is_symlink
        || entry.encrypted
        || entry.uncompressed_size == 0
    {
        return Err(ineligible_entry());
    }
    if let Some(mode) = entry.unix_mode {
        let file_type = mode & 0o170000;
        if file_type != 0 && file_type != 0o100000 {
            return Err(ineligible_entry());
        }
    }
    if !entry.basename.is_ascii()
        || !entry.basename.to_ascii_lowercase().ends_with(".ogg")
        || entry.path.contains('\\')
    {
        return Err(ineligible_entry());
    }
    #[allow(deprecated)]
    let compression = entry.compression.to_u16();
    if !matches!(compression, 0 | 8) {
        return Err(ineligible_entry());
    }
    Ok(())
}

fn ascii_case_equal(left: &str, right: &str) -> bool {
    left.is_ascii() && right.is_ascii() && left.eq_ignore_ascii_case(right)
}

fn revalidate_archive_snapshot(
    install_root: &Path,
    locale: &LocaleCode,
    expected: Option<&NativeArchiveEvidence>,
) -> Result<(), Failure> {
    let current = first_existing_locale_archive(install_root, locale)
        .map_err(map_archive_snapshot_recheck_error)?;
    match (expected, current) {
        (None, None) => Ok(()),
        (None, Some(_)) | (Some(_), None) => Err(archive_snapshot_changed()),
        (Some(expected), Some(current)) => {
            require_same_archive_selection(&expected.selected, &current)?;
            ArchiveIndex::open_with_expected_seal(
                &current.path,
                voice_archive_limits(),
                expected.seal,
            )
            .map_err(map_archive_error)?;
            let after = first_existing_locale_archive(install_root, locale)
                .map_err(map_archive_snapshot_recheck_error)?
                .ok_or_else(archive_snapshot_changed)?;
            require_same_archive_selection(&expected.selected, &after)
        }
    }
}

fn require_same_archive_selection(
    expected: &SelectedArchive,
    actual: &SelectedArchive,
) -> Result<(), Failure> {
    if expected.name != actual.name
        || expected.canonical_path != actual.canonical_path
        || expected.drifted != actual.drifted
        || !expected.voice_root.same_identity(&actual.voice_root)
    {
        return Err(archive_snapshot_changed());
    }
    Ok(())
}

fn map_archive_snapshot_recheck_error(error: Failure) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED",
        format!(
            "the authenticated pristine Voice archive selection changed during preparation: {}",
            error.message
        ),
    )
}

fn archive_snapshot_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED",
        "the authenticated pristine Voice archive selection changed during preparation",
    )
}

fn voice_archive_limits() -> VoiceArchiveLimits {
    VoiceArchiveLimits {
        max_central_directory_bytes: MAX_CENTRAL_DIRECTORY_BYTES,
        max_entries: MAX_ARCHIVE_ENTRIES,
        max_path_bytes: MAX_REVISION3_VOICE_TARGET_MEMBER_BYTES_V1,
        max_entry_uncompressed_bytes: MAX_ENTRY_UNCOMPRESSED_BYTES,
        max_total_uncompressed_bytes: MAX_TOTAL_UNCOMPRESSED_BYTES,
        ..VoiceArchiveLimits::default()
    }
}

fn map_archive_error(error: VoiceArchiveError) -> Failure {
    match error {
        VoiceArchiveError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_LIMIT",
            "the installed Voice archive exceeds its bounded inspection limits",
        ),
        VoiceArchiveError::UnsafeSource { .. } => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE",
            "the installed Voice archive is not one safe regular non-link file",
        ),
        VoiceArchiveError::ArchiveChanged => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED",
            "the installed Voice archive changed during preparation",
        ),
        VoiceArchiveError::SourceIo { .. } | VoiceArchiveError::Io(_) => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE",
            "the installed Voice archive could not be opened or read",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_INVALID",
            "the installed Voice archive is not a supported bounded ZIP",
        ),
    }
}

fn ineligible_entry() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TARGET_MEMBER_INELIGIBLE",
        "an exact Voice member match is unsafe, encrypted, linked, empty, non-regular, or uses unsupported compression",
    )
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
            "a revision-3 Voice target wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_TARGET_SIGNED_WIRE_LIMIT",
                "a revision-3 Voice target wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
            "the revision-3 Voice target basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_RESPONSE_LIMIT",
            "the revision-3 Voice target basis head exceeds its transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
            "the revision-3 Voice target response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_RESPONSE_LIMIT",
            "the revision-3 Voice target response exceeds its bounded transport budget",
        ));
    }
    Ok(())
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID",
        "request must be exact canonical JSON containing command and exactly current_project_json, game_root, root, and voice_target_request_json; target matches are native-only",
    )
}

fn project_invalid(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_TARGET_PROJECT_INVALID", message)
}

fn map_core_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
        format!("native Voice target evidence could not form a canonical core request: {error}"),
    )
}

fn map_transaction_error(error: Revision3VoiceTargetResolutionErrorV1) -> Failure {
    match error {
        Revision3VoiceTargetResolutionErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_INVALID",
            "the exact current revision-3 project is invalid",
        ),
        Revision3VoiceTargetResolutionErrorV1::InvalidRequest(error) => {
            map_core_request_error(error)
        }
        Revision3VoiceTargetResolutionErrorV1::ReopenCandidate(_)
        | Revision3VoiceTargetResolutionErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_VOICE_TARGET_INVARIANT",
            "the revision-3 Voice target candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3VoiceTargetResolutionConflictV1) -> Failure {
    let code = match &error {
        Revision3VoiceTargetResolutionConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT"
        }
        Revision3VoiceTargetResolutionConflictV1::ProjectIdentityMismatch { .. }
        | Revision3VoiceTargetResolutionConflictV1::ProjectRevisionConflict { .. }
        | Revision3VoiceTargetResolutionConflictV1::ProjectTargetMismatch
        | Revision3VoiceTargetResolutionConflictV1::LocalizationIdentityMismatch { .. }
        | Revision3VoiceTargetResolutionConflictV1::VoiceSlotIdentityMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT"
        }
        Revision3VoiceTargetResolutionConflictV1::ProjectRevisionOverflow
        | Revision3VoiceTargetResolutionConflictV1::VoiceSlotRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_REVISION_LIMIT"
        }
        Revision3VoiceTargetResolutionConflictV1::InvalidEntityIdentity
        | Revision3VoiceTargetResolutionConflictV1::InvalidExpectedLocId => {
            "AUTHORING_REVISION3_VOICE_TARGET_INTENT_INVALID"
        }
        Revision3VoiceTargetResolutionConflictV1::InvalidNativeEvidence { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_EVIDENCE_INVALID"
        }
        Revision3VoiceTargetResolutionConflictV1::DuplicateResolvedTarget { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_COLLISION"
        }
        Revision3VoiceTargetResolutionConflictV1::InvalidDialogLine { .. }
        | Revision3VoiceTargetResolutionConflictV1::InvalidLocalizationReference { .. }
        | Revision3VoiceTargetResolutionConflictV1::InvalidVoiceSlot { .. }
        | Revision3VoiceTargetResolutionConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_VOICE_TARGET_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_VOICE_TARGET_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_VOICE_TARGET_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_VOICE_TARGET_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Voice target working-store operation failed",
    )
}

fn format_sha256(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    const SUFFIX: &str = "...";
    let mut end = max_bytes - SUFFIX.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(SUFFIX);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::File;
    use std::io::Write;

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityPayload, LocalizationEntry, OriginRef, SchemaRevisionV3,
        TypedRef, VoiceSlot, VoiceTargetResolution,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, ProjectMeta, ProjectRevision3, WorkingStoreFormat,
    };
    use gore_mod::{BuildSpec, ModMeta, VoiceArchiveEdit, VoicePatchOp};
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    const LOC_ID: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";
    const EXECUTABLE_BYTES: &[u8] = b"fixture Gothic shipping executable";

    #[cfg(unix)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test directory symlink failed: {error}"),
        }
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x10; 16])
    }

    fn digest(tag: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([tag; 32])
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: EXECUTABLE_BYTES.len() as u64,
                sha256: Sha256Digest::from_bytes(Sha256::digest(EXECUTABLE_BYTES).into()),
            },
        }
    }

    #[test]
    fn native_loc_id_validation_uses_the_shared_portable_basename_contract() {
        assert!(validate_voice_basename_stem(LOC_ID).is_ok());
        for invalid in ["CON", "LÍNE", "LINE."] {
            let failure = validate_voice_basename_stem(invalid).unwrap_err();
            assert_eq!(
                failure.code, "AUTHORING_REVISION3_VOICE_TARGET_LOC_ID_INVALID",
                "unexpected result for {invalid:?}"
            );
        }
    }

    fn imported_origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: digest(tag),
            },
            external_identity: None,
        }
    }

    fn basis_project(revision: u64) -> ProjectRevision3 {
        let localization_id = id(2);
        let line_id = id(3);
        let slot_id = id(4);
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision,
            meta: ProjectMeta {
                name: "Voice target FFI".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale()]),
            entities: BTreeMap::from([
                (
                    localization_id,
                    Entity {
                        id: localization_id,
                        display_name: "Asghan line text".to_owned(),
                        origin: imported_origin(2),
                        revision: 4,
                        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: LOC_ID.to_owned(),
                            texts: BTreeMap::new(),
                        }),
                    },
                ),
                (
                    line_id,
                    Entity {
                        id: line_id,
                        display_name: "Asghan greeting".to_owned(),
                        origin: imported_origin(3),
                        revision: 2,
                        payload: EntityPayload::DialogLine(DialogLine {
                            localization: TypedRef::new(
                                project_id(),
                                localization_id,
                                EntityKind::LocalizationEntry,
                            ),
                            speaker_hint: Some("Asghan".to_owned()),
                            voice_slots: BTreeMap::from([(
                                locale(),
                                TypedRef::new(project_id(), slot_id, EntityKind::VoiceSlot),
                            )]),
                        }),
                    },
                ),
                (
                    slot_id,
                    Entity {
                        id: slot_id,
                        display_name: "Asghan German voice".to_owned(),
                        origin: imported_origin(4),
                        revision: 1,
                        payload: EntityPayload::VoiceSlot(VoiceSlot {
                            locale: locale(),
                            target_resolution: VoiceTargetResolution::Unresolved,
                            candidates: Vec::new(),
                            selected: None,
                        }),
                    },
                ),
            ]),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn publish_store_at(
        root: &Path,
        revision: u64,
    ) -> (ProjectRevision3, String, WorkingHead, Vec<u8>) {
        fs::create_dir_all(root).unwrap();
        let store = WorkingProjectStore::at(root, ffi_store_limits()).unwrap();
        let project = basis_project(revision);
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        (project, project_json, prepared.head, prepared.head_bytes)
    }

    fn published_store(revision: u64) -> (TempDir, ProjectRevision3, String, WorkingHead, Vec<u8>) {
        let temp = TempDir::new().unwrap();
        let (project, project_json, head, head_bytes) = publish_store_at(temp.path(), revision);
        (temp, project, project_json, head, head_bytes)
    }

    fn game_install() -> TempDir {
        let game = TempDir::new().unwrap();
        fs::create_dir_all(voice_root(game.path())).unwrap();
        let executable = executable_path(game.path());
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(executable, EXECUTABLE_BYTES).unwrap();
        game
    }

    fn executable_path(game_root: &Path) -> PathBuf {
        game_root
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe")
    }

    fn voice_root(game_root: &Path) -> PathBuf {
        game_root.join("G1R").join("Story").join("VoiceOver")
    }

    fn target_intent(project: &ProjectRevision3, head: &WorkingHead) -> VoiceTargetIntentV1 {
        VoiceTargetIntentV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            line_id: id(3),
            slot_id: id(4),
            locale: locale(),
            expected_loc_id: LOC_ID.to_owned(),
        }
    }

    fn intent_json(intent: &VoiceTargetIntentV1) -> String {
        serde_json::to_string(intent).unwrap()
    }

    fn raw_request(
        store_root: &Path,
        game_root: &Path,
        project_json: &str,
        target_request_json: String,
    ) -> String {
        json!({
            "command": COMMAND,
            "payload": {
                "current_project_json": project_json,
                "game_root": game_root.display().to_string(),
                "root": store_root.display().to_string(),
                "voice_target_request_json": target_request_json,
            },
        })
        .to_string()
    }

    fn call(
        store_root: &Path,
        game_root: &Path,
        project_json: &str,
        intent: &VoiceTargetIntentV1,
    ) -> Value {
        prepare_revision3_voice_target_v1_raw(&raw_request(
            store_root,
            game_root,
            project_json,
            intent_json(intent),
        ))
    }

    fn write_archive(path: &Path, entries: &[&str]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        for (index, entry) in entries.iter().enumerate() {
            writer
                .start_file(
                    *entry,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            writer
                .write_all(format!("voice-member-{index}").as_bytes())
                .unwrap();
        }
        writer.finish().unwrap();
    }

    fn deploy_voice_replacement(artifacts: &Path, game_root: &Path, archive: &str, member: &str) {
        let replacement = artifacts.join("replacement.ogg");
        fs::write(
            &replacement,
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        )
        .unwrap();
        let bundle_dir = artifacts.join("active-voice-bundle");
        let bundle = gore_mod::build_bundle(&BuildSpec {
            meta: ModMeta {
                name: "ActiveVoiceFixture".to_owned(),
                version: "1".to_owned(),
                author: "gore-ffi tests".to_owned(),
            },
            delay_ms: 0,
            overrides: Vec::new(),
            loc_edits: BTreeMap::new(),
            audio: Vec::new(),
            texture: Vec::new(),
            scripts: Vec::new(),
            dialog_topics: Vec::new(),
            voice: vec![VoiceArchiveEdit {
                archive: archive.to_owned(),
                op: VoicePatchOp::Replace,
                archive_path: member.to_owned(),
                ogg_path: replacement.display().to_string(),
                observation: None,
            }],
        })
        .unwrap();
        gore_mod::write_bundle(&bundle_dir, &bundle).unwrap();
        gore_mod::deploy(&bundle_dir, game_root).unwrap();
    }

    fn write_empty_archive_member(path: &Path, entry: &str) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(
                entry,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.finish().unwrap();
    }

    fn write_symlink_archive(path: &Path, entry: &str) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .add_symlink(entry, "target.ogg", SimpleFileOptions::default())
            .unwrap();
        writer.finish().unwrap();
    }

    fn find_eocd(bytes: &[u8]) -> usize {
        bytes
            .windows(4)
            .rposition(|window| window == 0x0605_4b50u32.to_le_bytes())
            .expect("fixture EOCD")
    }

    fn first_entry_layout(bytes: &[u8]) -> (usize, usize) {
        let eocd = find_eocd(bytes);
        let directory_size =
            u32::from_le_bytes(bytes[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
        let central = eocd - directory_size;
        assert_eq!(bytes[central..central + 4], 0x0201_4b50u32.to_le_bytes());
        let directory_relative =
            u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        let archive_offset = central - directory_relative;
        let local_relative =
            u32::from_le_bytes(bytes[central + 42..central + 46].try_into().unwrap()) as usize;
        let local = archive_offset + local_relative;
        assert_eq!(bytes[local..local + 4], 0x0403_4b50u32.to_le_bytes());
        (central, local)
    }

    fn mark_first_entry_encrypted(path: &Path) {
        let mut bytes = fs::read(path).unwrap();
        let (central, local) = first_entry_layout(&bytes);
        let central_flags =
            u16::from_le_bytes(bytes[central + 8..central + 10].try_into().unwrap()) | 1;
        let local_flags = u16::from_le_bytes(bytes[local + 6..local + 8].try_into().unwrap()) | 1;
        bytes[central + 8..central + 10].copy_from_slice(&central_flags.to_le_bytes());
        bytes[local + 6..local + 8].copy_from_slice(&local_flags.to_le_bytes());
        fs::write(path, bytes).unwrap();
    }

    fn mark_first_entry_unsupported(path: &Path) {
        let mut bytes = fs::read(path).unwrap();
        let (central, local) = first_entry_layout(&bytes);
        bytes[central + 10..central + 12].copy_from_slice(&12u16.to_le_bytes());
        bytes[local + 8..local + 10].copy_from_slice(&12u16.to_le_bytes());
        fs::write(path, bytes).unwrap();
    }

    fn assert_fixed_head(root: &Path, expected: &[u8]) {
        assert_eq!(fs::read(root.join("gore-project.json")).unwrap(), expected);
    }

    #[test]
    fn native_archive_priorities_mirror_all_ten_canonical_locales() {
        for (code, expected) in [
            ("en", ENGLISH_ARCHIVES),
            ("de", GERMAN_ARCHIVES),
            ("fr", FRENCH_ARCHIVES),
            ("it", ITALIAN_ARCHIVES),
            ("es", SPANISH_ARCHIVES),
            ("pl", POLISH_ARCHIVES),
            ("ru", RUSSIAN_ARCHIVES),
            ("ja", JAPANESE_ARCHIVES),
            ("zh-Hans", SIMPLIFIED_CHINESE_ARCHIVES),
            ("pt-BR", BRAZILIAN_PORTUGUESE_ARCHIVES),
        ] {
            let locale: LocaleCode = code.parse().unwrap();
            assert_eq!(archive_stems(&locale), Some(expected), "{code}");
        }
        let install = Path::new("C:/Games/Gothic Remake");
        assert_eq!(semantic_install_root(&install.join("G1R")), install);
        assert_eq!(semantic_install_root(install), install);
    }

    #[test]
    fn exact_wire_rejects_external_matches_and_noncanonical_or_duplicate_fields() {
        let (store, project, project_json, head, _) = published_store(7);
        let game = game_install();
        let intent = target_intent(&project, &head);
        let mut external = serde_json::to_value(&intent).unwrap();
        external
            .as_object_mut()
            .unwrap()
            .insert("matches".to_owned(), json!([]));
        let response = prepare_revision3_voice_target_v1_raw(&raw_request(
            store.path(),
            game.path(),
            &project_json,
            external.to_string(),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID"
        );

        let canonical = raw_request(
            store.path(),
            game.path(),
            &project_json,
            intent_json(&intent),
        );
        let noncanonical = format!(" {canonical}");
        assert_eq!(
            prepare_revision3_voice_target_v1_raw(&noncanonical)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID"
        );
        let duplicate = canonical.replacen(
            "\"command\":",
            "\"command\":\"authoring_store_prepare_revision3_voice_target_v1\",\"command\":",
            1,
        );
        assert_eq!(
            prepare_revision3_voice_target_v1_raw(&duplicate)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID"
        );
    }

    #[test]
    fn missing_archive_and_missing_member_prepare_unresolved_evidence() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let intent = target_intent(&project, &head);

        let missing_archive = call(store.path(), game.path(), &project_json, &intent);
        assert_eq!(missing_archive["ok"], true, "{missing_archive}");
        assert_eq!(missing_archive["resolution"], "unresolved");
        assert_eq!(missing_archive["match_count"], 0);
        assert_eq!(missing_archive["archive_observation"], Value::Null);
        assert_eq!(
            missing_archive["target_resolution"],
            json!({"state": "unresolved"})
        );
        assert_fixed_head(store.path(), &fixed_head);

        let archive = voice_root(game.path()).join("german_new.zip");
        write_archive(&archive, &["Voices/ANOTHER_LINE.ogg"]);
        let before = fs::read(&archive).unwrap();
        let missing_member = call(store.path(), game.path(), &project_json, &intent);
        assert_eq!(missing_member["ok"], true, "{missing_member}");
        assert_eq!(missing_member["resolution"], "unresolved");
        assert_eq!(missing_member["match_count"], 0);
        assert_eq!(
            missing_member["archive_observation"]["archive"],
            "german_new.zip"
        );
        assert_eq!(fs::read(&archive).unwrap(), before);
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn unique_member_prepares_resolved_replace_present_and_fully_reopens() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let archive = voice_root(game.path()).join("german_new.zip");
        write_archive(
            &archive,
            &[&format!("Voices/Hero/{LOC_ID}.ogg"), "notes.txt"],
        );
        let before = fs::read(&archive).unwrap();
        let response = call(
            store.path(),
            game.path(),
            &project_json,
            &target_intent(&project, &head),
        );

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["resolution"], "resolved");
        assert_eq!(response["match_count"], 1);
        assert_eq!(
            response["target_resolution"]["state"],
            Value::String("resolved".to_owned())
        );
        let target = &response["target_resolution"]["target"];
        assert_eq!(target["archive"], "german_new.zip");
        assert_eq!(target["member"], format!("Voices/Hero/{LOC_ID}.ogg"));
        assert_eq!(target["operation"], "replace");
        assert_eq!(target["member_proof"]["state"], "present");
        assert!(target["member_proof"]["uncompressed_size"]
            .as_u64()
            .is_some_and(|value| value > 0));
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");

        let store_api =
            WorkingProjectStore::open_existing(store.path(), ffi_store_limits()).unwrap();
        let candidate_head = response["head_json"].as_str().unwrap();
        let reopened = store_api
            .open_revision3_head_bytes(candidate_head.as_bytes(), AssetVerification::Full)
            .unwrap();
        assert_eq!(
            reopened.project.to_canonical_json().unwrap(),
            response["project_json"].as_str().unwrap()
        );
        assert_eq!(reopened.project.revision, 8);
        assert_eq!(fs::read(&archive).unwrap(), before);
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn active_voice_deployment_resolves_against_authenticated_pristine_backup() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let artifacts = TempDir::new().unwrap();
        let archive_name = "german_new.zip";
        let member = format!("Voices/Hero/{LOC_ID}.ogg");
        let archive = voice_root(game.path()).join(archive_name);
        write_archive(&archive, &[&member]);
        let pristine = fs::read(&archive).unwrap();
        let pristine_sha256 = format!("{:x}", Sha256::digest(&pristine));

        deploy_voice_replacement(artifacts.path(), game.path(), archive_name, &member);
        let deployed = fs::read(&archive).unwrap();
        assert_ne!(deployed, pristine);
        assert_ne!(format!("{:x}", Sha256::digest(&deployed)), pristine_sha256);

        let response = call(
            store.path(),
            game.path(),
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["resolution"], "resolved");
        assert_eq!(response["archive_observation"]["archive"], archive_name);
        assert_eq!(
            response["archive_observation"]["archive_seal"]["byte_len"],
            pristine.len() as u64
        );
        assert_eq!(
            response["archive_observation"]["archive_seal"]["sha256"],
            pristine_sha256
        );
        assert_eq!(
            response["target_resolution"]["target"]["archive_seal"]["sha256"],
            pristine_sha256
        );
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn installed_executable_must_exist_match_and_stay_stable() {
        let (store, project, project_json, head, fixed_head) = published_store(7);

        let missing = game_install();
        fs::remove_file(executable_path(missing.path())).unwrap();
        let response = call(
            store.path(),
            missing.path(),
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_UNAVAILABLE"
        );
        assert_fixed_head(store.path(), &fixed_head);

        let mismatched = game_install();
        fs::write(executable_path(mismatched.path()), b"wrong generation").unwrap();
        let response = call(
            store.path(),
            mismatched.path(),
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH"
        );
        assert_fixed_head(store.path(), &fixed_head);

        let drifting = game_install();
        let raw = raw_request(
            store.path(),
            drifting.path(),
            &project_json,
            intent_json(&target_intent(&project, &head)),
        );
        let response = prepare_revision3_voice_target_v1_inner_with_archive_guard(&raw, || {
            fs::write(
                executable_path(drifting.path()),
                b"changed during inspection",
            )
            .unwrap();
        })
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH"
        );
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn first_existing_locale_archive_wins_and_multiple_exact_paths_are_ambiguous() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let fallback = voice_root(game.path()).join("german.zip");
        write_archive(&fallback, &[&format!("Fallback/{LOC_ID}.ogg")]);
        let preferred = voice_root(game.path()).join("german_new.zip");
        write_archive(
            &preferred,
            &[
                &format!("Voices/A/{LOC_ID}.ogg"),
                &format!("Voices/B/{}.OGG", LOC_ID.to_ascii_lowercase()),
            ],
        );

        let response = call(
            store.path(),
            game.path(),
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["resolution"], "ambiguous");
        assert_eq!(response["match_count"], 2);
        assert_eq!(response["archive_observation"]["archive"], "german_new.zip");
        let candidates = response["target_resolution"]["candidates"]
            .as_array()
            .unwrap();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|target| {
            target["archive"] == "german_new.zip"
                && target["operation"] == "replace"
                && target["member_proof"]["state"] == "present"
        }));
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn in_install_voice_over_link_or_reparse_is_rejected() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let linked_voice_root = voice_root(game.path());
        let redirected_voice_root = game.path().join("G1R/Story/VoiceOver-real");
        fs::rename(&linked_voice_root, &redirected_voice_root).unwrap();
        if !make_test_dir_link(&redirected_voice_root, &linked_voice_root) {
            fs::rename(&redirected_voice_root, &linked_voice_root).unwrap();
            return;
        }
        write_archive(
            &redirected_voice_root.join("german_new.zip"),
            &[&format!("Voices/{LOC_ID}.ogg")],
        );

        let response = call(
            store.path(),
            game.path(),
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(
            response["error"]["code"], "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE",
            "{response}"
        );
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn exact_ineligible_matches_fail_closed_without_publishing() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let intent = target_intent(&project, &head);
        let cases = ["unsafe", "symlink", "encrypted", "unsupported", "empty"];
        for case in cases {
            let game = game_install();
            let archive = voice_root(game.path()).join("german_new.zip");
            let member = format!("Voices/{LOC_ID}.ogg");
            match case {
                "unsafe" => write_archive(&archive, &[&format!("../{LOC_ID}.ogg")]),
                "symlink" => write_symlink_archive(&archive, &member),
                "encrypted" => {
                    write_archive(&archive, &[&member]);
                    mark_first_entry_encrypted(&archive);
                }
                "unsupported" => {
                    write_archive(&archive, &[&member]);
                    mark_first_entry_unsupported(&archive);
                }
                "empty" => write_empty_archive_member(&archive, &member),
                _ => unreachable!(),
            }
            let before = fs::read(&archive).unwrap();
            let response = call(store.path(), game.path(), &project_json, &intent);
            assert_eq!(
                response["error"]["code"], "AUTHORING_REVISION3_VOICE_TARGET_MEMBER_INELIGIBLE",
                "case {case}: {response}"
            );
            assert_eq!(fs::read(&archive).unwrap(), before);
            assert_fixed_head(store.path(), &fixed_head);
        }
    }

    #[test]
    fn archive_drift_is_rejected_by_expected_seal_before_response() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let archive = voice_root(game.path()).join("german_new.zip");
        write_archive(&archive, &[&format!("Voices/{LOC_ID}.ogg")]);
        let request = raw_request(
            store.path(),
            game.path(),
            &project_json,
            intent_json(&target_intent(&project, &head)),
        );
        let response = prepare_revision3_voice_target_v1_inner_with_archive_guard(&request, || {
            write_archive(&archive, &["Voices/CHANGED.ogg"]);
        })
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"], "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED",
            "{response}"
        );
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn locale_archive_priority_drift_is_rejected_before_response() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let fallback = voice_root(game.path()).join("german.zip");
        write_archive(&fallback, &[&format!("Voices/{LOC_ID}.ogg")]);
        let preferred = voice_root(game.path()).join("german_new.zip");
        let request = raw_request(
            store.path(),
            game.path(),
            &project_json,
            intent_json(&target_intent(&project, &head)),
        );

        let response = prepare_revision3_voice_target_v1_inner_with_archive_guard(&request, || {
            write_archive(&preferred, &[&format!("Preferred/{LOC_ID}.ogg")]);
        })
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"], "AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED",
            "{response}"
        );
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn wrong_head_project_and_current_project_are_stable_conflicts() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();

        let mut wrong_head = target_intent(&project, &head);
        wrong_head.expected_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: digest(0xee),
            },
        };
        let response = call(store.path(), game.path(), &project_json, &wrong_head);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT"
        );

        let mut wrong_project = target_intent(&project, &head);
        wrong_project.expected_project_id = ProjectId::from_bytes([0xee; 16]);
        let response = call(store.path(), game.path(), &project_json, &wrong_project);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT"
        );

        let mut forged_project = project.clone();
        forged_project.meta.name = "forged current project".to_owned();
        let response = call(
            store.path(),
            game.path(),
            &forged_project.to_canonical_json().unwrap(),
            &target_intent(&project, &head),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_PROJECT_CONFLICT"
        );
        assert_fixed_head(store.path(), &fixed_head);
    }

    #[test]
    fn working_store_and_install_roots_must_be_bidirectionally_disjoint() {
        let outer_game = TempDir::new().unwrap();
        let nested_store = outer_game.path().join("nested-store");
        let (project, project_json, head, fixed_head) = publish_store_at(&nested_store, 7);
        fs::create_dir_all(voice_root(outer_game.path())).unwrap();
        let response = call(
            &nested_store,
            outer_game.path(),
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_GAME_ALIAS"
        );
        assert_fixed_head(&nested_store, &fixed_head);

        let outer_store = TempDir::new().unwrap();
        let (project, project_json, head, fixed_head) = publish_store_at(outer_store.path(), 7);
        let nested_game = outer_store.path().join("nested-game");
        fs::create_dir_all(voice_root(&nested_game)).unwrap();
        let response = call(
            outer_store.path(),
            &nested_game,
            &project_json,
            &target_intent(&project, &head),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_GAME_ALIAS"
        );
        assert_fixed_head(outer_store.path(), &fixed_head);
    }

    #[test]
    fn direct_g1r_mixed_case_and_root_failures_use_the_shared_guard() {
        let (store, project, project_json, head, fixed_head) = published_store(7);
        let game = game_install();
        let intent = target_intent(&project, &head);

        let direct_g1r = game.path().join("g1R");
        let direct = call(store.path(), &direct_g1r, &project_json, &intent);
        assert_eq!(direct["ok"], true, "{direct}");
        assert_eq!(direct["resolution"], "unresolved");
        assert_fixed_head(store.path(), &fixed_head);

        let missing_game = game.path().join("missing-parent").join("game");
        let unavailable = call(store.path(), &missing_game, &project_json, &intent);
        assert_eq!(
            unavailable["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_GAME_ROOT_UNAVAILABLE"
        );
        assert_fixed_head(store.path(), &fixed_head);

        let traversal_store = store.path().join("..").join(
            store
                .path()
                .file_name()
                .expect("temporary Store has one final component"),
        );
        let unsafe_store = call(&traversal_store, game.path(), &project_json, &intent);
        assert_eq!(
            unsafe_store["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TARGET_STORE_PATH_UNSAFE"
        );
        assert_fixed_head(store.path(), &fixed_head);
    }
}
