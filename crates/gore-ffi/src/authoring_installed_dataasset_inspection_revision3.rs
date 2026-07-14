//! Exact-current, read-only inspection of one server-selected installed DataAsset candidate.
//!
//! The wire carries only a managed Store root, game root, exact canonical head, two path-free
//! package-snapshot seals, and the original candidate ordinal. Native code fully reopens the
//! revision-3 project, rebuilds and compares the installed package snapshot, captures the
//! installed USMAP through retained guards, converts the server-selected package entirely in
//! memory, and runs the bounded fixed-leaf inspector over every export. No caller-supplied package
//! path, package ID, extraction/output path, USMAP path, raw package bytes, or mutation authority
//! crosses this boundary.

use std::io;
use std::path::Path;

use gore_asset::dataasset_workflow::verify_fixed_leaf_stage_edit_from_installed_snapshot_v1;
use gore_asset::{
    prepare_reviewed_footstep_preset_size_v1, reviewed_footstep_preset_target_from_ids_v1,
    FixedLeafSelector, ReviewedDataAssetErrorV1, ReviewedFootstepPresetReplacementV1,
    ReviewedFootstepPresetSizeV1, ReviewedFootstepPresetTargetV1,
};
use gore_authoring::{
    AssetVerification, OpenedRevision3Checkpoint, ProjectRevision3, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_tex::installed_package_index::{
    inspect_installed_package_index_v1, inspect_installed_usmap_v1, ExpectedInstalledExecutableV1,
    InstalledPackageContentSealV1, InstalledPackageExtractionErrorV1, InstalledPackageIndexErrorV1,
    InstalledUsmapErrorV1, VerifiedInstalledPackageIndexV1, VerifiedInstalledUsmapV1,
};
use gore_tex::package_index::PackageIndexError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_dataasset_revision3::{self, SemanticReplacementWire};
use crate::{dataasset, err};

pub(super) const COMMAND: &str = "authoring_store_inspect_revision3_installed_dataasset_v1";
pub(super) const PREPARE_EDIT_COMMAND: &str =
    "authoring_store_prepare_revision3_installed_dataasset_edit_v1";
pub(super) const PREPARE_REVIEWED_EDIT_COMMAND: &str =
    "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = (MAX_PATH_BYTES * 2 + MAX_HEAD_JSON_BYTES) * 6 + 8 * 1024;
const MAX_PREPARE_EDIT_WIRE_BYTES: usize = MAX_WIRE_BYTES + 8 * 1024 * 1024;
const MAX_PREPARE_REVIEWED_EDIT_WIRE_BYTES: usize = MAX_WIRE_BYTES + 4 * 1024;
const MAX_RESPONSE_BYTES: usize = crate::transport::MAX_TRANSPORT_RESPONSE_BYTES;
const MAX_TARGET_PATH_BYTES: usize = 512;
const INSTALLED_SOURCE_FORMAT: &str = "gore.authoring.revision3-installed-dataasset-source.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectInstalledDataAssetWirePayload {
    candidate_ordinal: u64,
    expected_head_json: String,
    expected_package_index_seal: ExpectedSealWire,
    expected_source_snapshot_seal: ExpectedSealWire,
    game_root: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareInstalledDataAssetEditWirePayload {
    candidate_ordinal: u64,
    expected_head_json: String,
    expected_inspection_binding: InspectionBindingWire,
    expected_package_index_seal: ExpectedSealWire,
    expected_source_snapshot_seal: ExpectedSealWire,
    expected_usmap_content_seal: ExpectedSealWire,
    expected_usmap_inventory_seal: ExpectedSealWire,
    game_root: String,
    replacement: SemanticReplacementWire,
    root: String,
    selector: FixedLeafSelector,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareReviewedInstalledDataAssetEditWirePayload {
    candidate_ordinal: u64,
    expected_head_json: String,
    expected_package_index_seal: ExpectedSealWire,
    expected_source_snapshot_seal: ExpectedSealWire,
    game_root: String,
    reviewed_edit: ReviewedInstalledDataAssetEditWire,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedInstalledDataAssetEditWire {
    field_id: String,
    format: u32,
    schema_id: String,
    schema_revision: u32,
    value: ReviewedFootstepPresetSizeWire,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedFootstepPresetSizeWire {
    x: String,
    y: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSealWire {
    byte_len: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InspectionBindingWire {
    uasset: ExpectedSealWire,
    uexp: ExpectedSealWire,
    usmap: ExpectedSealWire,
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

pub(super) fn inspect_revision3_installed_dataasset_v1_raw(input: &str) -> Value {
    inspect_revision3_installed_dataasset_v1_inner(input, MAX_RESPONSE_BYTES)
        .unwrap_or_else(Failure::response)
}

pub(super) fn prepare_revision3_installed_dataasset_edit_v1_raw(input: &str) -> Value {
    prepare_revision3_installed_dataasset_edit_v1_inner(input, MAX_RESPONSE_BYTES)
        .unwrap_or_else(Failure::response)
}

pub(super) fn prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(input: &str) -> Value {
    prepare_revision3_reviewed_installed_dataasset_edit_v1_inner(input, MAX_RESPONSE_BYTES)
        .unwrap_or_else(Failure::response)
}

fn inspect_revision3_installed_dataasset_v1_inner(
    input: &str,
    response_limit: usize,
) -> Result<Value, Failure> {
    let payload: InspectInstalledDataAssetWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    validate_candidate_ordinal(payload.candidate_ordinal)?;
    validate_expected_seal(&payload.expected_package_index_seal)?;
    validate_expected_seal(&payload.expected_source_snapshot_seal)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(head_conflict());
    }
    let head_json = serde_json::to_string(&before.head).map_err(|_| invariant_failure())?;
    if head_json != payload.expected_head_json {
        return Err(head_invalid());
    }
    validate_project(&before.project)?;

    let executable_anchor = ExpectedInstalledExecutableV1 {
        byte_len: before.project.target.executable.byte_len,
        sha256: *before.project.target.executable.sha256.as_bytes(),
    };
    let package_snapshot = match inspect_installed_package_index_v1(
        Path::new(&payload.game_root),
        executable_anchor,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let failure = map_package_snapshot_error(error);
            close_store_window(&store, &before, &expected_head, &payload.expected_head_json)?;
            return Err(failure);
        }
    };

    if let Err(failure) = validate_package_snapshot(&package_snapshot, &before.project) {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(failure);
    }
    if !wire_seal_matches(
        &payload.expected_package_index_seal,
        package_snapshot.index_seal(),
    ) {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(package_index_mismatch());
    }
    if !wire_seal_matches(
        &payload.expected_source_snapshot_seal,
        package_snapshot.source_snapshot_seal(),
    ) {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(source_snapshot_mismatch());
    }

    let ordinal = match usize::try_from(payload.candidate_ordinal) {
        Ok(ordinal) => ordinal,
        Err(_) => {
            close_package_and_store_window(
                &package_snapshot,
                &store,
                &before,
                &expected_head,
                &payload.expected_head_json,
            )?;
            return Err(candidate_invalid());
        }
    };
    let Some(candidate) = package_snapshot.index().candidates.get(ordinal) else {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(candidate_invalid());
    };
    if candidate.target_path.len() > MAX_TARGET_PATH_BYTES
        || candidate.package_id_hex.len() != 16
        || !is_lower_hex(&candidate.package_id_hex)
    {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(invariant_failure());
    }

    let usmap_snapshot = match inspect_installed_usmap_v1(Path::new(&payload.game_root)) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let failure = map_usmap_error(error);
            close_package_and_store_window(
                &package_snapshot,
                &store,
                &before,
                &expected_head,
                &payload.expected_head_json,
            )?;
            return Err(failure);
        }
    };
    if let Err(failure) = validate_usmap_snapshot(&usmap_snapshot) {
        close_all_windows(
            &package_snapshot,
            &usmap_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(failure);
    }

    let operation_result = (|| {
        let extracted = package_snapshot
            .extract_candidate_to_memory_v1(payload.candidate_ordinal)
            .map_err(map_extraction_error)?;
        if extracted.candidate_ordinal() != payload.candidate_ordinal
            || extracted.target_path() != candidate.target_path
            || extracted.package_id_hex() != candidate.package_id_hex
        {
            return Err(invariant_failure());
        }

        let usmap_bytes = clone_verified_bytes(usmap_snapshot.bytes())?;
        let (uasset, uexp) = extracted.into_core_bytes();
        let inspection =
            dataasset::fixed_inspect_verified_bytes_v1(uasset, uexp, usmap_bytes, None)
                .map_err(map_fixed_inspection_error)?;
        validate_nested_inspection(&inspection)?;

        enforce_response_budget(
            json!({
                "ok": true,
                "outcome": "inspection_only",
                "head_json": head_json,
                "project_id": before.project.project_id.to_string(),
                "project_revision": before.project.revision,
                "candidate_ordinal": payload.candidate_ordinal,
                "target_path": candidate.target_path,
                "package_id_hex": candidate.package_id_hex,
                "package_index_seal": package_snapshot.index_seal(),
                "source_snapshot_seal": package_snapshot.source_snapshot_seal(),
                "usmap_content_seal": usmap_snapshot.content_seal(),
                "usmap_inventory_seal": usmap_snapshot.inventory_seal(),
                "inspection": inspection,
                "scope": "selected_installed_dataasset_fixed_leaf_inspection_only",
                "mutation_status": "not_supported",
                "build_status": "not_evaluated",
                "runtime_status": "runtime_unqualified",
                "publication_status": "not_supported",
                "authority_status": "not_granted",
            }),
            response_limit,
        )
    })();

    // Security drift wins over extraction, parsing, and response-budget errors. These checks run
    // after the complete nested response has already been constructed and measured.
    close_all_windows(
        &package_snapshot,
        &usmap_snapshot,
        &store,
        &before,
        &expected_head,
        &payload.expected_head_json,
    )?;
    operation_result
}

fn prepare_revision3_installed_dataasset_edit_v1_inner(
    input: &str,
    response_limit: usize,
) -> Result<Value, Failure> {
    let payload: PrepareInstalledDataAssetEditWirePayload = parse_exact_edit_wire(input)?;
    prepare_revision3_installed_dataasset_edit_v1_payload(payload, response_limit)
}

fn prepare_revision3_installed_dataasset_edit_v1_payload(
    payload: PrepareInstalledDataAssetEditWirePayload,
    response_limit: usize,
) -> Result<Value, Failure> {
    prepare_revision3_installed_dataasset_edit_v1_payload_with_response(payload, move |response| {
        enforce_edit_response_budget(response, response_limit)
    })
}

fn prepare_revision3_installed_dataasset_edit_v1_payload_with_response<F>(
    payload: PrepareInstalledDataAssetEditWirePayload,
    finish_response: F,
) -> Result<Value, Failure>
where
    F: FnOnce(Value) -> Result<Value, Failure>,
{
    validate_edit_path(&payload.root)?;
    validate_edit_path(&payload.game_root)?;
    validate_edit_candidate_ordinal(payload.candidate_ordinal)?;
    for seal in [
        &payload.expected_package_index_seal,
        &payload.expected_source_snapshot_seal,
        &payload.expected_usmap_content_seal,
        &payload.expected_usmap_inventory_seal,
        &payload.expected_inspection_binding.uasset,
        &payload.expected_inspection_binding.uexp,
        &payload.expected_inspection_binding.usmap,
    ] {
        validate_edit_expected_seal(seal)?;
    }
    if payload.expected_inspection_binding.usmap != payload.expected_usmap_content_seal {
        return Err(edit_invalid_request());
    }
    let expected_head = parse_canonical_edit_head(&payload.expected_head_json)?;

    // The Store CAS basis is authoritative and is checked before touching the game install.
    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(edit_head_conflict());
    }
    let head_json = serde_json::to_string(&before.head).map_err(|_| edit_invariant_failure())?;
    if head_json != payload.expected_head_json {
        return Err(edit_head_invalid());
    }
    if validate_project(&before.project).is_err() {
        close_store_window(&store, &before, &expected_head, &payload.expected_head_json)?;
        return Err(edit_invariant_failure());
    }
    if before.project.revision >= i64::MAX as u64 {
        close_store_window(&store, &before, &expected_head, &payload.expected_head_json)?;
        return Err(Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REVISION_LIMIT",
            "the published project revision cannot be incremented safely",
        ));
    }

    let executable_anchor = ExpectedInstalledExecutableV1 {
        byte_len: before.project.target.executable.byte_len,
        sha256: *before.project.target.executable.sha256.as_bytes(),
    };
    let package_snapshot = match inspect_installed_package_index_v1(
        Path::new(&payload.game_root),
        executable_anchor,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let failure = map_package_snapshot_error(error);
            close_store_window(&store, &before, &expected_head, &payload.expected_head_json)?;
            return Err(failure);
        }
    };
    if let Err(failure) = validate_package_snapshot(&package_snapshot, &before.project) {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(failure);
    }
    if !wire_seal_matches(
        &payload.expected_package_index_seal,
        package_snapshot.index_seal(),
    ) {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(edit_package_index_mismatch());
    }
    if !wire_seal_matches(
        &payload.expected_source_snapshot_seal,
        package_snapshot.source_snapshot_seal(),
    ) {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(edit_source_snapshot_mismatch());
    }

    let ordinal = match usize::try_from(payload.candidate_ordinal) {
        Ok(ordinal) => ordinal,
        Err(_) => {
            close_package_and_store_window(
                &package_snapshot,
                &store,
                &before,
                &expected_head,
                &payload.expected_head_json,
            )?;
            return Err(edit_candidate_invalid());
        }
    };
    let Some(candidate) = package_snapshot.index().candidates.get(ordinal) else {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(edit_candidate_invalid());
    };
    if candidate.target_path.len() > MAX_TARGET_PATH_BYTES
        || candidate.package_id_hex.len() != 16
        || !is_lower_hex(&candidate.package_id_hex)
    {
        close_package_and_store_window(
            &package_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(edit_invariant_failure());
    }
    let selected_target_path = candidate.target_path.clone();

    let usmap_snapshot = match inspect_installed_usmap_v1(Path::new(&payload.game_root)) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let failure = map_usmap_error(error);
            close_package_and_store_window(
                &package_snapshot,
                &store,
                &before,
                &expected_head,
                &payload.expected_head_json,
            )?;
            return Err(failure);
        }
    };
    if let Err(failure) = validate_usmap_snapshot(&usmap_snapshot) {
        close_all_windows(
            &package_snapshot,
            &usmap_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(failure);
    }
    if !wire_seal_matches(
        &payload.expected_usmap_content_seal,
        usmap_snapshot.content_seal(),
    ) {
        close_all_windows(
            &package_snapshot,
            &usmap_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(edit_usmap_content_mismatch());
    }
    if !wire_seal_matches(
        &payload.expected_usmap_inventory_seal,
        usmap_snapshot.inventory_seal(),
    ) {
        close_all_windows(
            &package_snapshot,
            &usmap_snapshot,
            &store,
            &before,
            &expected_head,
            &payload.expected_head_json,
        )?;
        return Err(edit_usmap_inventory_mismatch());
    }

    let operation_result = (|| {
        let extracted = package_snapshot
            .extract_candidate_to_memory_v1(payload.candidate_ordinal)
            .map_err(map_extraction_error)?;
        if extracted.candidate_ordinal() != payload.candidate_ordinal
            || extracted.target_path() != selected_target_path
        {
            return Err(edit_invariant_failure());
        }

        // Re-run the same bounded whole-package inspector used to mint the prior proof. The
        // caller cannot smuggle raw bytes or a target; only a selector that is still reported as
        // editable for this exact installed byte triple may advance into staging.
        let inspection = dataasset::fixed_inspect_verified_bytes_v1(
            clone_verified_bytes(extracted.uasset_bytes()).map_err(|_| edit_input_limit())?,
            clone_verified_bytes(extracted.uexp_bytes()).map_err(|_| edit_input_limit())?,
            clone_verified_bytes(usmap_snapshot.bytes()).map_err(|_| edit_input_limit())?,
            None,
        )
        .map_err(map_fixed_edit_inspection_error)?;
        validate_nested_inspection(&inspection).map_err(|_| edit_invariant_failure())?;
        let actual_inspection_binding = inspection_binding(&inspection)?;
        if actual_inspection_binding != payload.expected_inspection_binding {
            return Err(edit_inspection_binding_mismatch());
        }
        require_exact_editable_selector(&inspection, &payload.selector)?;

        let replacement_bytes = payload
            .replacement
            .encode_bytes_for(payload.selector.kind)
            .map_err(map_shared_dataasset_failure)?;
        let selector_json =
            serde_json::to_vec(&payload.selector).map_err(|_| edit_invariant_failure())?;
        let request_intent_binding = authoring_dataasset_revision3::intent_binding_sha256(
            &selected_target_path,
            &selector_json,
            &replacement_bytes,
        );
        let replacement_hex = authoring_dataasset_revision3::encode_wire_bytes(&replacement_bytes);
        let verified = verify_fixed_leaf_stage_edit_from_installed_snapshot_v1(
            Path::new(&payload.game_root),
            extracted,
            &usmap_snapshot,
            payload.selector,
            &replacement_hex,
        )
        .map_err(|_| edit_semantic_invalid())?;
        let prepared = store
            .prepare_revision3_dataasset_stage_v1(&expected_head, verified)
            .map_err(authoring_dataasset_revision3::map_staging_error)
            .map_err(map_shared_dataasset_failure)?;
        let prepared_intent_binding =
            authoring_dataasset_revision3::stage_intent_binding_sha256(prepared.stage())
                .map_err(map_shared_dataasset_failure)?;
        if prepared_intent_binding != request_intent_binding {
            return Err(edit_invariant_failure());
        }

        let proof_binding = installed_proof_binding_sha256(
            payload.candidate_ordinal,
            package_snapshot.index_seal(),
            package_snapshot.source_snapshot_seal(),
            usmap_snapshot.content_seal(),
            usmap_snapshot.inventory_seal(),
            &actual_inspection_binding,
        )?;
        let installed_source = json!({
            "format": INSTALLED_SOURCE_FORMAT,
            "candidate_ordinal": payload.candidate_ordinal,
            "package_index_seal": package_snapshot.index_seal(),
            "source_snapshot_seal": package_snapshot.source_snapshot_seal(),
            "usmap_content_seal": usmap_snapshot.content_seal(),
            "usmap_inventory_seal": usmap_snapshot.inventory_seal(),
            "inspection_binding": actual_inspection_binding,
        });
        let mut response = authoring_dataasset_revision3::prepared_response(
            prepared,
            Some(prepared_intent_binding),
        )
        .map_err(map_shared_dataasset_failure)?;
        let object = response
            .as_object_mut()
            .ok_or_else(edit_invariant_failure)?;
        object.insert(
            "installed_proof_binding_sha256".to_owned(),
            Value::String(proof_binding),
        );
        object.insert("installed_source".to_owned(), installed_source);
        finish_response(response)
    })();

    // Package, USMAP, and published Store drift always outrank parsing, patching, staging, and
    // response errors. Immutable CAS objects may have been installed, but the fixed head is never
    // published by this prepare-only route.
    close_all_windows(
        &package_snapshot,
        &usmap_snapshot,
        &store,
        &before,
        &expected_head,
        &payload.expected_head_json,
    )?;
    operation_result
}

fn prepare_revision3_reviewed_installed_dataasset_edit_v1_inner(
    input: &str,
    response_limit: usize,
) -> Result<Value, Failure> {
    let payload: PrepareReviewedInstalledDataAssetEditWirePayload =
        parse_exact_reviewed_edit_wire(input)?;
    // The target is deliberately server-selected from the exact installed candidate. A fixed
    // registry target is used here only to validate the target-independent schema identity before
    // any filesystem access; the candidate's actual target is still matched natively below.
    reviewed_footstep_preset_target_from_ids_v1(
        payload.reviewed_edit.format,
        &payload.reviewed_edit.schema_id,
        payload.reviewed_edit.schema_revision,
        &payload.reviewed_edit.field_id,
        ReviewedFootstepPresetTargetV1::Wolf.id(),
    )
    .map_err(|_| reviewed_edit_invalid_request())?;
    let requested = ReviewedFootstepPresetSizeV1::try_new(
        parse_reviewed_positive_canonical_decimal(&payload.reviewed_edit.value.x)?,
        parse_reviewed_positive_canonical_decimal(&payload.reviewed_edit.value.y)?,
    )
    .map_err(|_| reviewed_edit_invalid_request())?;

    validate_reviewed_edit_path(&payload.root)?;
    validate_reviewed_edit_path(&payload.game_root)?;
    validate_reviewed_edit_candidate_ordinal(payload.candidate_ordinal)?;
    validate_reviewed_edit_expected_seal(&payload.expected_package_index_seal)?;
    validate_reviewed_edit_expected_seal(&payload.expected_source_snapshot_seal)?;
    parse_canonical_reviewed_edit_head(&payload.expected_head_json)?;

    // Mint a fresh native inspection from only the client's exact-current head/package/source
    // basis. This first pass owns and closes its full Store/package/USMAP drift window.
    let inspection_request = json!({
        "command": COMMAND,
        "payload": {
            "candidate_ordinal": payload.candidate_ordinal,
            "expected_head_json": payload.expected_head_json,
            "expected_package_index_seal": payload.expected_package_index_seal,
            "expected_source_snapshot_seal": payload.expected_source_snapshot_seal,
            "game_root": payload.game_root,
            "root": payload.root,
        },
    })
    .to_string();
    let inspected =
        inspect_revision3_installed_dataasset_v1_inner(&inspection_request, MAX_RESPONSE_BYTES)?;
    let target_path = inspected
        .get("target_path")
        .and_then(Value::as_str)
        .ok_or_else(reviewed_edit_invariant_failure)?;
    let nested_inspection = inspected
        .get("inspection")
        .ok_or_else(reviewed_edit_invariant_failure)?;
    let reviewed = select_exact_reviewed_edit(target_path, nested_inspection, requested)?;

    let actual_inspection_binding =
        inspection_binding(nested_inspection).map_err(|_| reviewed_edit_invariant_failure())?;
    let usmap_content = trusted_response_seal(&inspected, "usmap_content_seal")?;
    let usmap_inventory = trusted_response_seal(&inspected, "usmap_inventory_seal")?;
    if actual_inspection_binding.usmap != usmap_content {
        return Err(reviewed_edit_invariant_failure());
    }

    let current = reviewed.current_components();
    let replacement = reviewed.replacement_components();
    let reviewed_binding = reviewed.binding_sha256();
    let reviewed_format = reviewed.format();
    let reviewed_schema_id = reviewed.schema_id();
    let reviewed_schema_revision = reviewed.schema_revision();
    let reviewed_field_id = reviewed.field_id();
    let reviewed_target_id = reviewed.target().id();
    let selector = reviewed.selector().clone();
    let replacement_wire = SemanticReplacementWire::Vector4F64x4 {
        x: reviewed_decimal_string(replacement[0])?,
        y: reviewed_decimal_string(replacement[1])?,
        z: reviewed_decimal_string(replacement[2])?,
        w: reviewed_decimal_string(replacement[3])?,
    };

    // Lower to the existing typed installed edit executor. It rebuilds every proof again, checks
    // the first pass's USMAP/inspection seals, runs the shared semantic verifier/stager, and closes
    // the complete second drift window. Thus any change between the two native passes also wins.
    let typed_payload = PrepareInstalledDataAssetEditWirePayload {
        candidate_ordinal: payload.candidate_ordinal,
        expected_head_json: payload.expected_head_json,
        expected_inspection_binding: actual_inspection_binding,
        expected_package_index_seal: payload.expected_package_index_seal,
        expected_source_snapshot_seal: payload.expected_source_snapshot_seal,
        expected_usmap_content_seal: usmap_content,
        expected_usmap_inventory_seal: usmap_inventory,
        game_root: payload.game_root,
        replacement: replacement_wire,
        root: payload.root,
        selector,
    };
    prepare_revision3_installed_dataasset_edit_v1_payload_with_response(
        typed_payload,
        move |mut response| {
            let object = response
                .as_object_mut()
                .ok_or_else(reviewed_edit_invariant_failure)?;
            object.insert(
                "reviewed_edit".to_owned(),
                json!({
                    "format": reviewed_format,
                    "schema_id": reviewed_schema_id,
                    "schema_revision": reviewed_schema_revision,
                    "field_id": reviewed_field_id,
                    "target_id": reviewed_target_id,
                }),
            );
            object.insert(
                "reviewed_before".to_owned(),
                reviewed_components_value(current)?,
            );
            object.insert(
                "reviewed_after".to_owned(),
                reviewed_components_value(replacement)?,
            );
            object.insert(
                "reviewed_intent_binding_sha256".to_owned(),
                Value::String(hex_digest(reviewed_binding)),
            );
            enforce_reviewed_edit_response_budget(response, response_limit)
        },
    )
}

fn select_exact_reviewed_edit(
    target_path: &str,
    inspection: &Value,
    requested: ReviewedFootstepPresetSizeV1,
) -> Result<ReviewedFootstepPresetReplacementV1, Failure> {
    let exports = inspection
        .get("exports")
        .and_then(Value::as_array)
        .ok_or_else(reviewed_edit_invariant_failure)?;
    let mut match_count = 0_u64;
    let mut selected = None;
    let mut no_change = false;
    for leaf in exports
        .iter()
        .filter_map(|export| export.get("leaves").and_then(Value::as_array))
        .flatten()
        .filter(|leaf| leaf.get("editable") == Some(&Value::Bool(true)))
    {
        let selector: FixedLeafSelector = serde_json::from_value(
            leaf.get("selector")
                .cloned()
                .ok_or_else(reviewed_edit_invariant_failure)?,
        )
        .map_err(|_| reviewed_edit_invariant_failure())?;
        match prepare_reviewed_footstep_preset_size_v1(target_path, &selector, requested) {
            Ok(candidate) => {
                match_count = match_count
                    .checked_add(1)
                    .ok_or_else(reviewed_edit_invariant_failure)?;
                if selected.replace(candidate).is_some() {
                    return Err(reviewed_edit_match_invalid());
                }
            }
            Err(ReviewedDataAssetErrorV1::NoChange) => {
                match_count = match_count
                    .checked_add(1)
                    .ok_or_else(reviewed_edit_invariant_failure)?;
                no_change = true;
            }
            Err(
                ReviewedDataAssetErrorV1::InvalidExpectedBytes
                | ReviewedDataAssetErrorV1::NonFiniteCurrentComponent { .. }
                | ReviewedDataAssetErrorV1::BindingSerialization,
            ) => return Err(reviewed_edit_semantic_invalid()),
            Err(_) => {}
        }
    }
    if match_count != 1 {
        return Err(reviewed_edit_match_invalid());
    }
    if no_change {
        return Err(reviewed_edit_semantic_invalid());
    }
    selected.ok_or_else(reviewed_edit_invariant_failure)
}

fn trusted_response_seal(response: &Value, field: &str) -> Result<ExpectedSealWire, Failure> {
    let seal: ExpectedSealWire = serde_json::from_value(
        response
            .get(field)
            .cloned()
            .ok_or_else(reviewed_edit_invariant_failure)?,
    )
    .map_err(|_| reviewed_edit_invariant_failure())?;
    validate_edit_expected_seal(&seal).map_err(|_| reviewed_edit_invariant_failure())?;
    Ok(seal)
}

fn reviewed_components_value(components: [f64; 4]) -> Result<Value, Failure> {
    Ok(json!({
        "x": reviewed_decimal_string(components[0])?,
        "y": reviewed_decimal_string(components[1])?,
        "z": reviewed_decimal_string(components[2])?,
        "w": reviewed_decimal_string(components[3])?,
    }))
}

fn reviewed_decimal_string(value: f64) -> Result<String, Failure> {
    if !value.is_finite() {
        return Err(reviewed_edit_invariant_failure());
    }
    let mut encoded = value.to_string();
    if encoded.len() > 64 {
        encoded = format!("{value:e}");
    }
    let round_trip = encoded
        .parse::<f64>()
        .map_err(|_| reviewed_edit_invariant_failure())?;
    if encoded.len() > 64
        || !is_canonical_reviewed_response_decimal(&encoded)
        || round_trip.to_bits() != value.to_bits()
    {
        return Err(reviewed_edit_invariant_failure());
    }
    Ok(encoded)
}

fn is_canonical_reviewed_response_decimal(value: &str) -> bool {
    if value.is_empty() || !value.is_ascii() || value.contains(['+', 'E']) {
        return false;
    }
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let (mantissa, exponent) = match unsigned.split_once('e') {
        Some((mantissa, exponent)) if !exponent.contains('e') => (mantissa, Some(exponent)),
        Some(_) => return false,
        None => (unsigned, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) if !fraction.contains('.') => (whole, Some(fraction)),
        Some(_) => return false,
        None => (mantissa, None),
    };
    let whole_is_canonical = whole == "0"
        || (whole
            .as_bytes()
            .first()
            .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
            && whole.bytes().all(|byte| byte.is_ascii_digit()));
    let fraction_is_canonical = fraction.is_none_or(|fraction| {
        !fraction.is_empty()
            && fraction.bytes().all(|byte| byte.is_ascii_digit())
            && !fraction.ends_with('0')
    });
    let exponent_is_canonical = exponent.is_none_or(|exponent| {
        let magnitude = exponent.strip_prefix('-').unwrap_or(exponent);
        magnitude
            .as_bytes()
            .first()
            .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
            && magnitude.bytes().all(|byte| byte.is_ascii_digit())
    });
    whole_is_canonical && fraction_is_canonical && exponent_is_canonical
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_LIMIT",
            format!(
                "installed DataAsset inspection request exceeds the {MAX_WIRE_BYTES}-byte limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn parse_exact_edit_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_PREPARE_EDIT_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT",
            format!(
                "installed DataAsset edit request exceeds the {MAX_PREPARE_EDIT_WIRE_BYTES}-byte limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| edit_invalid_request())?;
    if request.command != PREPARE_EDIT_COMMAND {
        return Err(edit_invalid_request());
    }
    Ok(request.payload)
}

fn parse_exact_reviewed_edit_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_PREPARE_REVIEWED_EDIT_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT",
            format!(
                "reviewed installed DataAsset edit request exceeds the {MAX_PREPARE_REVIEWED_EDIT_WIRE_BYTES}-byte limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| reviewed_edit_invalid_request())?;
    if request.command != PREPARE_REVIEWED_EDIT_COMMAND {
        return Err(reviewed_edit_invalid_request());
    }
    Ok(request.payload)
}

fn validate_reviewed_edit_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(reviewed_edit_invalid_request());
    }
    Ok(())
}

fn validate_reviewed_edit_candidate_ordinal(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(reviewed_edit_candidate_invalid());
    }
    Ok(())
}

fn validate_reviewed_edit_expected_seal(seal: &ExpectedSealWire) -> Result<(), Failure> {
    if seal.byte_len == 0
        || seal.byte_len > i64::MAX as u64
        || seal.sha256.len() != 64
        || !is_lower_hex(&seal.sha256)
    {
        return Err(reviewed_edit_invalid_request());
    }
    Ok(())
}

fn parse_canonical_reviewed_edit_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(reviewed_edit_head_invalid());
    }
    let head: WorkingHead =
        serde_json::from_str(input).map_err(|_| reviewed_edit_head_invalid())?;
    if serde_json::to_string(&head).map_err(|_| reviewed_edit_invariant_failure())? != input {
        return Err(reviewed_edit_head_invalid());
    }
    Ok(head)
}

fn parse_reviewed_positive_canonical_decimal(value: &str) -> Result<f64, Failure> {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return Err(reviewed_edit_invalid_request());
    }
    let (whole, fraction) = match value.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (value, None),
    };
    let whole_is_canonical = whole == "0"
        || (whole
            .as_bytes()
            .first()
            .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
            && whole.bytes().all(|byte| byte.is_ascii_digit()));
    let fraction_is_canonical = fraction.is_none_or(|fraction| {
        !fraction.is_empty()
            && fraction.bytes().all(|byte| byte.is_ascii_digit())
            && !fraction.ends_with('0')
    });
    if !whole_is_canonical || !fraction_is_canonical {
        return Err(reviewed_edit_invalid_request());
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|_| reviewed_edit_invalid_request())?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(reviewed_edit_invalid_request());
    }
    Ok(parsed)
}

fn validate_edit_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(edit_invalid_request());
    }
    Ok(())
}

fn parse_canonical_edit_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(edit_head_invalid());
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| edit_head_invalid())?;
    if serde_json::to_string(&head).map_err(|_| edit_invariant_failure())? != input {
        return Err(edit_head_invalid());
    }
    Ok(head)
}

fn validate_edit_expected_seal(seal: &ExpectedSealWire) -> Result<(), Failure> {
    if seal.byte_len == 0
        || seal.byte_len > i64::MAX as u64
        || seal.sha256.len() != 64
        || !is_lower_hex(&seal.sha256)
    {
        return Err(edit_invalid_request());
    }
    Ok(())
}

fn validate_edit_candidate_ordinal(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(edit_candidate_invalid());
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(head_invalid());
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| head_invalid())?;
    if serde_json::to_string(&head).map_err(|_| invariant_failure())? != input {
        return Err(head_invalid());
    }
    Ok(head)
}

fn validate_expected_seal(seal: &ExpectedSealWire) -> Result<(), Failure> {
    if seal.byte_len == 0
        || seal.byte_len > i64::MAX as u64
        || seal.sha256.len() != 64
        || !is_lower_hex(&seal.sha256)
    {
        return Err(invalid_request());
    }
    Ok(())
}

fn validate_project(project: &ProjectRevision3) -> Result<(), Failure> {
    signed_wire_u64(project.revision)?;
    signed_wire_u64(project.target.executable.byte_len)?;
    if project.target.executable.byte_len == 0 {
        return Err(invariant_failure());
    }
    Ok(())
}

fn validate_package_snapshot(
    snapshot: &VerifiedInstalledPackageIndexV1,
    project: &ProjectRevision3,
) -> Result<(), Failure> {
    let executable = snapshot.target_executable();
    if executable.byte_len != project.target.executable.byte_len
        || executable.sha256 != project.target.executable.sha256.to_string()
    {
        return Err(invariant_failure());
    }
    for seal in [
        executable,
        snapshot.mount_inventory_seal(),
        snapshot.index_seal(),
        snapshot.source_snapshot_seal(),
    ] {
        validate_native_seal(seal)?;
    }
    signed_wire_u64(snapshot.mount_inventory_entry_count())?;
    let index = snapshot.index();
    for value in [
        index.physical_chunk_count,
        index.winning_export_bundle_count,
        index.directory_indexed_export_bundle_count,
        index.out_of_scope_export_bundle_count,
        u64::try_from(index.candidates.len()).map_err(|_| invariant_failure())?,
        u64::try_from(index.partial_reasons.len()).map_err(|_| invariant_failure())?,
    ] {
        signed_wire_u64(value)?;
    }
    for reason in &index.partial_reasons {
        signed_wire_u64(reason.count)?;
    }

    let canonical_index = serde_json::to_string(index).map_err(|_| invariant_failure())?;
    let canonical_length = u64::try_from(canonical_index.len()).map_err(|_| invariant_failure())?;
    if canonical_index != snapshot.index_json()
        || snapshot.index_seal().byte_len != canonical_length
        || snapshot.index_seal().sha256 != hex_digest(&Sha256::digest(canonical_index.as_bytes()))
    {
        return Err(invariant_failure());
    }
    Ok(())
}

fn validate_usmap_snapshot(snapshot: &VerifiedInstalledUsmapV1) -> Result<(), Failure> {
    validate_native_seal(snapshot.content_seal())?;
    validate_native_seal(snapshot.inventory_seal())?;
    signed_wire_u64(snapshot.inventory_entry_count())?;
    let length = u64::try_from(snapshot.bytes().len()).map_err(|_| invariant_failure())?;
    if length != snapshot.content_seal().byte_len
        || snapshot.content_seal().sha256 != hex_digest(&Sha256::digest(snapshot.bytes()))
    {
        return Err(invariant_failure());
    }
    Ok(())
}

fn validate_native_seal(seal: &InstalledPackageContentSealV1) -> Result<(), Failure> {
    signed_wire_u64(seal.byte_len)?;
    if seal.sha256.len() != 64 || !is_lower_hex(&seal.sha256) {
        return Err(invariant_failure());
    }
    Ok(())
}

fn wire_seal_matches(wire: &ExpectedSealWire, native: &InstalledPackageContentSealV1) -> bool {
    wire.byte_len == native.byte_len && wire.sha256 == native.sha256
}

fn validate_nested_inspection(inspection: &Value) -> Result<(), Failure> {
    let object = inspection.as_object().ok_or_else(invariant_failure)?;
    if object.get("ok") != Some(&Value::Bool(true))
        || object.get("format").and_then(Value::as_str) != Some("gore.dataasset.fixed-inspect.v1")
        || object
            .get("selection")
            .and_then(Value::as_object)
            .and_then(|selection| selection.get("export_index"))
            != Some(&Value::Null)
    {
        return Err(invariant_failure());
    }
    Ok(())
}

fn inspection_binding(inspection: &Value) -> Result<InspectionBindingWire, Failure> {
    let input = inspection
        .get("input")
        .and_then(Value::as_object)
        .ok_or_else(edit_invariant_failure)?;
    let binding = inspection
        .get("binding")
        .and_then(Value::as_object)
        .ok_or_else(edit_invariant_failure)?;
    let package = binding
        .get("package_seal")
        .and_then(Value::as_object)
        .ok_or_else(edit_invariant_failure)?;
    let seal = |length_key: &str, digest_key: &str| -> Result<ExpectedSealWire, Failure> {
        let value = ExpectedSealWire {
            byte_len: input
                .get(length_key)
                .and_then(Value::as_u64)
                .ok_or_else(edit_invariant_failure)?,
            sha256: package
                .get(digest_key)
                .and_then(Value::as_str)
                .ok_or_else(edit_invariant_failure)?
                .to_owned(),
        };
        validate_edit_expected_seal(&value).map_err(|_| edit_invariant_failure())?;
        Ok(value)
    };
    let usmap = ExpectedSealWire {
        byte_len: input
            .get("usmap_length")
            .and_then(Value::as_u64)
            .ok_or_else(edit_invariant_failure)?,
        sha256: binding
            .get("usmap_sha256")
            .and_then(Value::as_str)
            .ok_or_else(edit_invariant_failure)?
            .to_owned(),
    };
    validate_edit_expected_seal(&usmap).map_err(|_| edit_invariant_failure())?;
    Ok(InspectionBindingWire {
        uasset: seal("uasset_length", "uasset_sha256")?,
        uexp: seal("uexp_length", "uexp_sha256")?,
        usmap,
    })
}

fn require_exact_editable_selector(
    inspection: &Value,
    selector: &FixedLeafSelector,
) -> Result<(), Failure> {
    let selector = serde_json::to_value(selector).map_err(|_| edit_invariant_failure())?;
    let exports = inspection
        .get("exports")
        .and_then(Value::as_array)
        .ok_or_else(edit_invariant_failure)?;
    let mut matches = 0_u64;
    for leaf in exports
        .iter()
        .filter_map(|export| export.get("leaves").and_then(Value::as_array))
        .flatten()
    {
        if leaf.get("editable") == Some(&Value::Bool(true))
            && leaf.get("selector") == Some(&selector)
        {
            matches = matches.checked_add(1).ok_or_else(edit_invariant_failure)?;
        }
    }
    if matches != 1 {
        return Err(edit_selector_mismatch());
    }
    Ok(())
}

fn installed_proof_binding_sha256(
    candidate_ordinal: u64,
    package_index: &InstalledPackageContentSealV1,
    source_snapshot: &InstalledPackageContentSealV1,
    usmap_content: &InstalledPackageContentSealV1,
    usmap_inventory: &InstalledPackageContentSealV1,
    inspection: &InspectionBindingWire,
) -> Result<String, Failure> {
    let mut hasher = Sha256::new();
    hasher.update(b"gore.authoring.r3-installed-dataasset-proof-binding.v1\0");
    hasher.update(candidate_ordinal.to_le_bytes());
    for (byte_len, sha256) in [
        (package_index.byte_len, package_index.sha256.as_str()),
        (source_snapshot.byte_len, source_snapshot.sha256.as_str()),
        (usmap_content.byte_len, usmap_content.sha256.as_str()),
        (usmap_inventory.byte_len, usmap_inventory.sha256.as_str()),
        (
            inspection.uasset.byte_len,
            inspection.uasset.sha256.as_str(),
        ),
        (inspection.uexp.byte_len, inspection.uexp.sha256.as_str()),
        (inspection.usmap.byte_len, inspection.usmap.sha256.as_str()),
    ] {
        hasher.update(byte_len.to_le_bytes());
        hasher.update(decode_sha256(sha256)?);
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn decode_sha256(value: &str) -> Result<[u8; 32], Failure> {
    if value.len() != 64 || !is_lower_hex(value) {
        return Err(edit_invariant_failure());
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] =
            (decode_lower_hex_nibble(pair[0])? << 4) | decode_lower_hex_nibble(pair[1])?;
    }
    Ok(decoded)
}

fn decode_lower_hex_nibble(value: u8) -> Result<u8, Failure> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(edit_invariant_failure()),
    }
}

fn clone_verified_bytes(bytes: &[u8]) -> Result<Vec<u8>, Failure> {
    let mut cloned = Vec::new();
    cloned.try_reserve_exact(bytes.len()).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_LIMIT",
            "installed DataAsset inspection could not reserve its bounded verified input",
        )
    })?;
    cloned.extend_from_slice(bytes);
    Ok(cloned)
}

fn close_package_and_store_window(
    package: &VerifiedInstalledPackageIndexV1,
    store: &WorkingProjectStore,
    before: &OpenedRevision3Checkpoint,
    expected_head: &WorkingHead,
    expected_head_json: &str,
) -> Result<(), Failure> {
    let package_after = package.revalidate().map_err(map_package_snapshot_error);
    let store_after = close_store_window(store, before, expected_head, expected_head_json);
    package_after?;
    store_after
}

fn close_all_windows(
    package: &VerifiedInstalledPackageIndexV1,
    usmap: &VerifiedInstalledUsmapV1,
    store: &WorkingProjectStore,
    before: &OpenedRevision3Checkpoint,
    expected_head: &WorkingHead,
    expected_head_json: &str,
) -> Result<(), Failure> {
    let package_after = package.revalidate().map_err(map_package_snapshot_error);
    let usmap_after = usmap.revalidate().map_err(map_usmap_error);
    let store_after = close_store_window(store, before, expected_head, expected_head_json);
    package_after?;
    usmap_after?;
    store_after
}

fn close_store_window(
    store: &WorkingProjectStore,
    before: &OpenedRevision3Checkpoint,
    expected_head: &WorkingHead,
    expected_head_json: &str,
) -> Result<(), Failure> {
    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != *expected_head || after.project != before.project {
        return Err(head_conflict());
    }
    let canonical = serde_json::to_string(&after.head).map_err(|_| invariant_failure())?;
    if canonical != expected_head_json {
        return Err(head_conflict());
    }
    Ok(())
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT",
            "installed DataAsset inspection contains an integer outside the signed wire range",
        ));
    }
    Ok(())
}

fn validate_candidate_ordinal(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(candidate_invalid());
    }
    Ok(())
}

struct BoundedResponseCounter {
    bytes: usize,
    limit: usize,
    exceeded: bool,
}

impl io::Write for BoundedResponseCounter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let Some(next) = self.bytes.checked_add(buffer.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("response counter overflow"));
        };
        if next > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("response budget exceeded"));
        }
        self.bytes = next;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let mut counter = BoundedResponseCounter {
        bytes: 0,
        limit,
        exceeded: false,
    };
    if serde_json::to_writer(&mut counter, &response).is_err() {
        return if counter.exceeded {
            Err(Failure::new(
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT",
                "installed DataAsset inspection response exceeds its bounded transport budget",
            ))
        } else {
            Err(invariant_failure())
        };
    }
    Ok(response)
}

fn enforce_edit_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let mut counter = BoundedResponseCounter {
        bytes: 0,
        limit,
        exceeded: false,
    };
    if serde_json::to_writer(&mut counter, &response).is_err() {
        return if counter.exceeded {
            Err(Failure::new(
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT",
                "installed DataAsset edit response exceeds its bounded transport budget",
            ))
        } else {
            Err(edit_invariant_failure())
        };
    }
    Ok(response)
}

fn enforce_reviewed_edit_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let mut counter = BoundedResponseCounter {
        bytes: 0,
        limit,
        exceeded: false,
    };
    if serde_json::to_writer(&mut counter, &response).is_err() {
        return if counter.exceeded {
            Err(Failure::new(
                "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT",
                "reviewed installed DataAsset edit response exceeds its bounded transport budget",
            ))
        } else {
            Err(reviewed_edit_invariant_failure())
        };
    }
    Ok(response)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID",
        "request must contain exactly candidate_ordinal, expected_head_json, expected_package_index_seal, expected_source_snapshot_seal, game_root, and root",
    )
}

fn edit_invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
        "request must contain exactly the installed snapshot seals, prior inspection binding, candidate ordinal, exact head, roots, selector, and typed replacement",
    )
}

fn reviewed_edit_invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
        "request must contain only exact-current installed snapshot evidence and one closed reviewed footstep-preset size intent",
    )
}

fn reviewed_edit_head_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_HEAD_INVALID",
        "expected_head_json is not one exact duplicate-free canonical revision-3 head",
    )
}

fn reviewed_edit_candidate_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID",
        "candidate_ordinal is outside the supported exact installed snapshot range",
    )
}

fn reviewed_edit_match_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_MATCH_INVALID",
        "the exact installed candidate does not contain exactly one matching reviewed editable leaf",
    )
}

fn reviewed_edit_semantic_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INVALID",
        "the reviewed footstep-preset edit cannot be lowered from this exact installed value",
    )
}

fn reviewed_edit_invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INVARIANT",
        "the native reviewed installed DataAsset edit failed an internal invariant",
    )
}

fn edit_head_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_HEAD_INVALID",
        "expected_head_json is not one exact duplicate-free canonical revision-3 head",
    )
}

fn edit_head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_HEAD_CONFLICT",
        "the revision-3 Store head changed or differs from the caller's exact head",
    )
}

fn edit_candidate_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID",
        "candidate_ordinal is outside the exact rebuilt installed package snapshot",
    )
}

fn edit_package_index_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_PACKAGE_INDEX_MISMATCH",
        "the exact installed package index no longer matches the prior inspection proof",
    )
}

fn edit_source_snapshot_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SOURCE_SNAPSHOT_MISMATCH",
        "the exact installed source snapshot no longer matches the prior inspection proof",
    )
}

fn edit_usmap_content_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_CONTENT_MISMATCH",
        "the exact installed USMAP content no longer matches the prior inspection proof",
    )
}

fn edit_usmap_inventory_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_INVENTORY_MISMATCH",
        "the installed USMAP inventory no longer matches the prior inspection proof",
    )
}

fn edit_inspection_binding_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_BINDING_MISMATCH",
        "the re-inspected installed package bytes no longer match the prior inspection proof",
    )
}

fn edit_selector_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SELECTOR_MISMATCH",
        "the selector is not one exact editable leaf in the re-inspected installed package",
    )
}

fn edit_semantic_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INVALID",
        "the exact installed snapshot cannot authorize this typed fixed-leaf edit",
    )
}

fn edit_input_limit() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT",
        "the verified installed DataAsset exceeds the bounded edit input limit",
    )
}

fn edit_invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INVARIANT",
        "the native installed DataAsset edit failed an internal invariant",
    )
}

fn map_shared_dataasset_failure(error: authoring_dataasset_revision3::Failure) -> Failure {
    Failure::new(error.code(), error.message())
}

fn head_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_INVALID",
        "expected_head_json is not one exact duplicate-free canonical revision-3 head",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_CONFLICT",
        "the revision-3 Store head changed or differs from the caller's exact head",
    )
}

fn candidate_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_CANDIDATE_INVALID",
        "candidate_ordinal is outside the exact rebuilt installed package snapshot",
    )
}

fn package_index_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PACKAGE_INDEX_MISMATCH",
        "the exact installed package index no longer matches the caller's path-free seal",
    )
}

fn source_snapshot_mismatch() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_SOURCE_SNAPSHOT_MISMATCH",
        "the exact installed source snapshot no longer matches the caller's path-free seal",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INVARIANT",
        "the native installed DataAsset inspection failed an internal invariant",
    )
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_HEAD_MISSING"
        }
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 working Store inspection failed")
}

fn map_package_snapshot_error(error: InstalledPackageIndexErrorV1) -> Failure {
    use InstalledPackageIndexErrorV1 as E;
    let code = match error {
        E::InvalidExpectedExecutable => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_GENERATION_MISMATCH"
        }
        E::ParentTraversal
        | E::PathContainsNul
        | E::UnsafePath { .. }
        | E::NonUtf8TreeEntry
        | E::UnsafeTreeEntry => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_PATH_UNSAFE"
        }
        E::NestedMountable
        | E::NoncanonicalMountName { .. }
        | E::MountNameCollision
        | E::MainContainerMissing
        | E::MountCompanionMissing { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_LAYOUT_INVALID"
        }
        E::TreeEntryLimit { .. }
        | E::TreeDepthLimit { .. }
        | E::TreePathLimit { .. }
        | E::AggregateTreePathLimit { .. }
        | E::DirectMountLimit { .. }
        | E::FileLengthLimit { .. }
        | E::AggregateHashedMountLimit { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_LIMIT"
        }
        E::ExecutableMismatch => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_GENERATION_MISMATCH"
        }
        E::SourceChanged { .. } | E::TreeChanged | E::OpenedContainerSetChanged => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_CHANGED"
        }
        E::ContainerPriority(error) | E::PackageIndex(error) => {
            return map_package_index_error(error);
        }
        E::IoStoreOpen => "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_IOSTORE_OPEN_FAILED",
        E::IndexJsonLimit { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT"
        }
        E::IndexSerialization | E::CounterOverflow => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INVARIANT"
        }
        E::Filesystem { .. } => "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_IO",
        E::UnsupportedPlatform => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PLATFORM_UNSUPPORTED"
        }
    };
    Failure::new(
        code,
        "the installed package snapshot could not be verified exactly",
    )
}

fn map_package_index_error(error: PackageIndexError) -> Failure {
    use PackageIndexError as E;
    let code = match error {
        E::InvalidLimits(_) | E::InvalidLimit { .. } | E::CounterOverflow => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INVARIANT"
        }
        E::AmbiguousContainerPriority { .. } | E::ContainerPriorityVersionOverflow { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_LAYOUT_INVALID"
        }
        E::ChildContainerLimit { .. }
        | E::ContainerPriorityNameLimit { .. }
        | E::AggregateContainerPriorityNameLimit { .. }
        | E::ChunkScanLimit { .. }
        | E::WinningExportBundleLimit { .. }
        | E::DirectoryPathLimit { .. }
        | E::AggregateDirectoryPathLimit { .. }
        | E::CandidateLimit { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_LIMIT"
        }
        E::ContainerVersionUnavailable => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PACKAGE_INDEX_FAILED"
        }
    };
    Failure::new(
        code,
        "the installed package index could not be rebuilt exactly",
    )
}

fn map_extraction_error(error: InstalledPackageExtractionErrorV1) -> Failure {
    match error {
        InstalledPackageExtractionErrorV1::Snapshot(error) => map_package_snapshot_error(error),
        InstalledPackageExtractionErrorV1::CandidateOrdinalOutOfRange { .. } => candidate_invalid(),
        InstalledPackageExtractionErrorV1::IoStoreOpen => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_IOSTORE_OPEN_FAILED",
            "the exact installed IoStore could not be reopened for inspection",
        ),
        InstalledPackageExtractionErrorV1::OpenedContainerSetChanged => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_CHANGED",
            "the installed container set changed during DataAsset inspection",
        ),
        InstalledPackageExtractionErrorV1::Conversion => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_EXTRACTION_FAILED",
            "the server-selected installed package could not be converted exactly",
        ),
        InstalledPackageExtractionErrorV1::SourceEvidence => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_EXTRACTION_FAILED",
            "the server-selected installed package source evidence was incomplete",
        ),
        InstalledPackageExtractionErrorV1::CounterOverflow => invariant_failure(),
    }
}

fn map_usmap_error(error: InstalledUsmapErrorV1) -> Failure {
    use InstalledUsmapErrorV1 as E;
    let code = match error {
        E::UnsupportedPlatform => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PLATFORM_UNSUPPORTED"
        }
        E::Source(error) => return map_usmap_source_error(error),
        E::EntryLimit { .. }
        | E::EntryNameLimit { .. }
        | E::AggregateNameLimit { .. }
        | E::CounterOverflow => "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_LIMIT",
        E::NonUtf8EntryName | E::EntryNameCollision | E::NoncanonicalUsmapName { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_UNSAFE"
        }
        E::MissingUsmap => "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_MISSING",
        E::InventoryChanged => "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_CHANGED",
    };
    Failure::new(code, "the installed USMAP could not be verified exactly")
}

fn map_usmap_source_error(error: InstalledPackageIndexErrorV1) -> Failure {
    use InstalledPackageIndexErrorV1 as E;
    let code = match error {
        E::ParentTraversal
        | E::PathContainsNul
        | E::UnsafePath { .. }
        | E::NonUtf8TreeEntry
        | E::UnsafeTreeEntry
        | E::NestedMountable
        | E::NoncanonicalMountName { .. }
        | E::MountNameCollision
        | E::MainContainerMissing
        | E::MountCompanionMissing { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_UNSAFE"
        }
        E::TreeEntryLimit { .. }
        | E::TreeDepthLimit { .. }
        | E::TreePathLimit { .. }
        | E::AggregateTreePathLimit { .. }
        | E::DirectMountLimit { .. }
        | E::FileLengthLimit { .. }
        | E::AggregateHashedMountLimit { .. }
        | E::IndexJsonLimit { .. } => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_LIMIT"
        }
        E::SourceChanged { .. } | E::TreeChanged | E::OpenedContainerSetChanged => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_CHANGED"
        }
        E::Filesystem { .. } | E::IoStoreOpen => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_IO"
        }
        E::UnsupportedPlatform => {
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PLATFORM_UNSUPPORTED"
        }
        E::InvalidExpectedExecutable
        | E::ExecutableMismatch
        | E::ContainerPriority(_)
        | E::PackageIndex(_)
        | E::IndexSerialization
        | E::CounterOverflow => "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INVARIANT",
    };
    Failure::new(code, "the installed USMAP source boundary failed")
}

fn map_fixed_inspection_error(error: dataasset::Failure) -> Failure {
    match error.code() {
        "DATAASSET_INPUT_LIMIT" => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_LIMIT",
            "the verified installed DataAsset exceeds the bounded inspection input limit",
        ),
        "DATAASSET_RESPONSE_LIMIT" => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT",
            "the nested fixed-leaf inspection exceeds its bounded response limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INSPECTION_FAILED",
            "the verified installed package and USMAP could not be inspected exactly",
        ),
    }
}

fn map_fixed_edit_inspection_error(error: dataasset::Failure) -> Failure {
    match error.code() {
        "DATAASSET_INPUT_LIMIT" => edit_input_limit(),
        "DATAASSET_RESPONSE_LIMIT" => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT",
            "the re-inspected fixed-leaf report exceeds its bounded response limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_FAILED",
            "the exact installed package and USMAP could not be re-inspected safely",
        ),
    }
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let suffix = "...";
    let mut end = max_bytes - suffix.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(suffix);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::Path;
    #[cfg(windows)]
    use std::path::PathBuf;

    use gore_asset::{
        FixedLeafRole, FixedLeafSelectorStep, FixedLeafWireType, PackageComponent, PackagePairSeal,
        FIXED_LEAF_SELECTOR_FORMAT, FIXED_LEAF_SELECTOR_PROFILE,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        SchemaRevisionV3, Sha256Digest,
    };
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    const EXE_BYTES: &[u8] = b"gore-ffi installed DataAsset inspection executable fixture v1";
    const TARGET: &str = "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps";

    fn project() -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x68; 16]),
            revision: 9,
            meta: ProjectMeta {
                name: "PRIVATE INSTALLED DATAASSET PROJECT".to_owned(),
                version: "0.1.0-private".to_owned(),
                author: "private-author".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: EXE_BYTES.len() as u64,
                    sha256: Sha256Digest::from_bytes(Sha256::digest(EXE_BYTES).into()),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn publish_store(root: &Path) -> String {
        let project = project();
        let store = WorkingProjectStore::at(root, ffi_store_limits()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        String::from_utf8(prepared.head_bytes).unwrap()
    }

    fn seal_json(byte_len: u64, byte: char) -> Value {
        json!({"byte_len": byte_len, "sha256": byte.to_string().repeat(64)})
    }

    fn valid_shape() -> Value {
        json!({
            "candidate_ordinal": 0,
            "expected_head_json": "{}",
            "expected_package_index_seal": seal_json(1, 'a'),
            "expected_source_snapshot_seal": seal_json(2, 'b'),
            "game_root": "C:/missing-game",
            "root": "C:/missing-store",
        })
    }

    fn bool_selector() -> FixedLeafSelector {
        FixedLeafSelector {
            format: FIXED_LEAF_SELECTOR_FORMAT,
            profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
            package_seal: PackagePairSeal {
                uasset_sha256: [0x11; 32],
                uexp_sha256: [0x22; 32],
            },
            usmap_sha256: "33".repeat(32),
            export_index: 0,
            object_name: "Fixture".to_owned(),
            class_path: "/Script/Test.Fixture".to_owned(),
            component: PackageComponent::Uexp,
            export_sha256: "44".repeat(32),
            role: FixedLeafRole::PropertyValue,
            kind: gore_asset::FixedWireKind::Bool,
            path: vec![FixedLeafSelectorStep::Property {
                schema_index: 0,
                property_name: "Enabled".to_owned(),
                array_index: 0,
                array_dimension: 1,
                declaring_schema_name: "Fixture".to_owned(),
                declaring_module_path: Some("/Script/Test".to_owned()),
                property_type: FixedLeafWireType::Bool {},
            }],
            expected_hex: "01".to_owned(),
        }
    }

    fn valid_edit_shape() -> Value {
        json!({
            "candidate_ordinal": 0,
            "expected_head_json": "{}",
            "expected_inspection_binding": {
                "uasset": seal_json(10, '1'),
                "uexp": seal_json(20, '2'),
                "usmap": seal_json(30, '3'),
            },
            "expected_package_index_seal": seal_json(40, '4'),
            "expected_source_snapshot_seal": seal_json(50, '5'),
            "expected_usmap_content_seal": seal_json(30, '3'),
            "expected_usmap_inventory_seal": seal_json(60, '6'),
            "game_root": "C:/missing-game",
            "replacement": {"kind": "bool", "value": false},
            "root": "C:/missing-store",
            "selector": bool_selector(),
        })
    }

    fn valid_reviewed_edit_shape() -> Value {
        json!({
            "candidate_ordinal": 0,
            "expected_head_json": "{}",
            "expected_package_index_seal": seal_json(40, '4'),
            "expected_source_snapshot_seal": seal_json(50, '5'),
            "game_root": "C:/missing-game",
            "reviewed_edit": {
                "format": gore_asset::REVIEWED_DATAASSET_FORMAT_V1,
                "schema_id": gore_asset::REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID,
                "schema_revision": gore_asset::REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                "field_id": gore_asset::REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
                "value": {"x": "125.5", "y": "225"},
            },
            "root": "C:/missing-store",
        })
    }

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn raw_edit_request(payload: Value) -> String {
        json!({"command": PREPARE_EDIT_COMMAND, "payload": payload}).to_string()
    }

    fn raw_reviewed_edit_request(payload: Value) -> String {
        json!({"command": PREPARE_REVIEWED_EDIT_COMMAND, "payload": payload}).to_string()
    }

    fn tree_bytes(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        let mut output = BTreeMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.is_dir() {
                    output.insert(relative, None);
                    pending.push(path);
                } else {
                    output.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        output
    }

    #[test]
    fn exact_wire_accepts_only_the_six_authorized_payload_fields() {
        let parsed: InspectInstalledDataAssetWirePayload =
            parse_exact_wire(&raw_request(valid_shape())).unwrap();
        assert_eq!(parsed.candidate_ordinal, 0);

        for forbidden in [
            "target_path",
            "package_id_hex",
            "output_path",
            "usmap_path",
            "uasset_path",
            "project_json",
            "export_index",
        ] {
            let mut payload = valid_shape();
            payload[forbidden] = json!("forged-authority");
            assert_eq!(
                inspect_revision3_installed_dataasset_v1_raw(&raw_request(payload))["error"]
                    ["code"],
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID"
            );
        }

        let valid = valid_shape();
        let head = serde_json::to_string(&valid["expected_head_json"]).unwrap();
        let package = serde_json::to_string(&valid["expected_package_index_seal"]).unwrap();
        let source = serde_json::to_string(&valid["expected_source_snapshot_seal"]).unwrap();
        let game = serde_json::to_string(&valid["game_root"]).unwrap();
        let root = serde_json::to_string(&valid["root"]).unwrap();
        for duplicate in [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"candidate_ordinal\":0,\"expected_head_json\":{head},\"expected_package_index_seal\":{package},\"expected_source_snapshot_seal\":{source},\"game_root\":{game},\"root\":{root}}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"candidate_ordinal\":0,\"candidate_ordinal\":1,\"expected_head_json\":{head},\"expected_package_index_seal\":{package},\"expected_source_snapshot_seal\":{source},\"game_root\":{game},\"root\":{root}}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"candidate_ordinal\":0,\"expected_head_json\":{head},\"expected_package_index_seal\":{{\"byte_len\":1,\"byte_len\":2,\"sha256\":\"{}\"}},\"expected_source_snapshot_seal\":{source},\"game_root\":{game},\"root\":{root}}}}}",
                "a".repeat(64)
            ),
        ] {
            assert_eq!(
                inspect_revision3_installed_dataasset_v1_raw(&duplicate)["error"]["code"],
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID"
            );
        }

        for malformed in [
            json!({"command": "wrong", "payload": valid_shape()}),
            json!({"command": COMMAND, "payload": {}}),
            json!({"command": COMMAND, "payload": {
                "candidate_ordinal": "0",
                "expected_head_json": "{}",
                "expected_package_index_seal": seal_json(1, 'a'),
                "expected_source_snapshot_seal": seal_json(2, 'b'),
                "game_root": "g",
                "root": "r",
            }}),
        ] {
            assert_eq!(
                inspect_revision3_installed_dataasset_v1_raw(&malformed.to_string())["error"]
                    ["code"],
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn installed_edit_wire_is_closed_path_minimal_and_seal_bound() {
        let parsed: PrepareInstalledDataAssetEditWirePayload =
            parse_exact_edit_wire(&raw_edit_request(valid_edit_shape())).unwrap();
        assert_eq!(parsed.candidate_ordinal, 0);
        assert_eq!(parsed.expected_inspection_binding.uasset.byte_len, 10);

        for forbidden in [
            "target_path",
            "package_id_hex",
            "output_path",
            "patch_receipt_path",
            "extract_receipt_path",
            "usmap_path",
            "uasset_path",
            "uexp_path",
            "raw_bytes",
            "project_json",
            "export_index",
        ] {
            let mut payload = valid_edit_shape();
            payload[forbidden] = json!("forged-authority");
            assert_eq!(
                prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(payload))
                    ["error"]["code"],
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
                "accepted forbidden field {forbidden}"
            );
        }

        for required in [
            "candidate_ordinal",
            "expected_head_json",
            "expected_inspection_binding",
            "expected_package_index_seal",
            "expected_source_snapshot_seal",
            "expected_usmap_content_seal",
            "expected_usmap_inventory_seal",
            "game_root",
            "replacement",
            "root",
            "selector",
        ] {
            let mut payload = valid_edit_shape();
            payload.as_object_mut().unwrap().remove(required);
            assert_eq!(
                prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(payload))
                    ["error"]["code"],
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
                "accepted missing field {required}"
            );
        }

        let mut invalid = valid_edit_shape();
        invalid["expected_usmap_inventory_seal"]["sha256"] = json!("A".repeat(64));
        assert_eq!(
            prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(invalid))["error"]
                ["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID"
        );
        let mut invalid = valid_edit_shape();
        invalid["expected_inspection_binding"]["uexp"]["byte_len"] = json!(0);
        assert_eq!(
            prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(invalid))["error"]
                ["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID"
        );
        let mut invalid = valid_edit_shape();
        invalid["expected_inspection_binding"]["usmap"]["sha256"] = json!("7".repeat(64));
        assert_eq!(
            prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(invalid))["error"]
                ["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID"
        );
        let mut invalid = valid_edit_shape();
        invalid["candidate_ordinal"] = json!(i64::MAX as u64 + 1);
        assert_eq!(
            prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(invalid))["error"]
                ["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID"
        );
        assert_eq!(
            prepare_revision3_installed_dataasset_edit_v1_raw(
                &" ".repeat(MAX_PREPARE_EDIT_WIRE_BYTES + 1)
            )["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT"
        );
    }

    #[test]
    fn reviewed_installed_edit_wire_is_closed_and_uses_strict_canonical_decimals() {
        let parsed: PrepareReviewedInstalledDataAssetEditWirePayload =
            parse_exact_reviewed_edit_wire(&raw_reviewed_edit_request(valid_reviewed_edit_shape()))
                .unwrap();
        assert_eq!(parsed.candidate_ordinal, 0);
        assert_eq!(parsed.reviewed_edit.value.x, "125.5");

        for forbidden in [
            "target_path",
            "target_id",
            "package_id_hex",
            "selector",
            "replacement",
            "expected_inspection_binding",
            "expected_usmap_content_seal",
            "expected_usmap_inventory_seal",
            "uasset_path",
            "uexp_path",
            "usmap_path",
            "raw_bytes",
            "project_json",
        ] {
            let mut payload = valid_reviewed_edit_shape();
            payload[forbidden] = json!("forged-authority");
            assert_eq!(
                prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
                    &raw_reviewed_edit_request(payload),
                )["error"]["code"],
                "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
                "accepted forbidden field {forbidden}"
            );
        }
        for forbidden in ["target_path", "target_id", "selector", "replacement_bytes"] {
            let mut payload = valid_reviewed_edit_shape();
            payload["reviewed_edit"][forbidden] = json!("forged-authority");
            assert_eq!(
                prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
                    &raw_reviewed_edit_request(payload),
                )["error"]["code"],
                "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
                "accepted forbidden reviewed field {forbidden}"
            );
        }
        for required in [
            "candidate_ordinal",
            "expected_head_json",
            "expected_package_index_seal",
            "expected_source_snapshot_seal",
            "game_root",
            "reviewed_edit",
            "root",
        ] {
            let mut payload = valid_reviewed_edit_shape();
            payload.as_object_mut().unwrap().remove(required);
            assert_eq!(
                prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
                    &raw_reviewed_edit_request(payload),
                )["error"]["code"],
                "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID",
                "accepted missing field {required}"
            );
        }

        for (field, invalid) in [
            ("format", json!(2)),
            ("schema_id", json!("g1r.tracking.near-match")),
            ("schema_revision", json!(2)),
            ("field_id", json!("feet_texture_scale")),
        ] {
            let mut payload = valid_reviewed_edit_shape();
            payload["reviewed_edit"][field] = invalid;
            assert_eq!(
                prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
                    &raw_reviewed_edit_request(payload),
                )["error"]["code"],
                "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID"
            );
        }
        for invalid in [
            json!(""),
            json!("0"),
            json!("01"),
            json!("1.0"),
            json!("1."),
            json!("+1"),
            json!("-1"),
            json!("1e2"),
            json!(" 1"),
            json!(1),
        ] {
            let mut payload = valid_reviewed_edit_shape();
            payload["reviewed_edit"]["value"]["x"] = invalid;
            assert_eq!(
                prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
                    &raw_reviewed_edit_request(payload),
                )["error"]["code"],
                "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID"
            );
        }
        for valid in ["1", "0.5", "10000000000000000000", "1.0000000000000001"] {
            assert!(
                parse_reviewed_positive_canonical_decimal(valid).is_ok(),
                "{valid}"
            );
        }

        let payload = valid_reviewed_edit_shape();
        let payload_json = serde_json::to_string(&payload).unwrap();
        let duplicate_command = format!(
            "{{\"command\":\"{PREPARE_REVIEWED_EDIT_COMMAND}\",\"command\":\"{PREPARE_REVIEWED_EDIT_COMMAND}\",\"payload\":{payload_json}}}"
        );
        assert_eq!(
            prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(&duplicate_command)["error"]
                ["code"],
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID"
        );
        assert_eq!(
            prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
                &" ".repeat(MAX_PREPARE_REVIEWED_EDIT_WIRE_BYTES + 1),
            )["error"]["code"],
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT"
        );
    }

    #[test]
    fn reviewed_response_decimals_are_bounded_canonical_and_bit_exact() {
        let values = [
            0.0,
            -0.0,
            f64::MAX,
            -f64::MAX,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::from_bits(1),
            -f64::from_bits(1),
            1.234_567_890_123_456_7,
        ];
        for value in values {
            let encoded = reviewed_decimal_string(value).unwrap();
            assert!(encoded.len() <= 64, "{encoded}");
            assert!(
                is_canonical_reviewed_response_decimal(&encoded),
                "{encoded}"
            );
            assert!(!encoded.contains(['+', 'E']), "{encoded}");
            assert_eq!(encoded.parse::<f64>().unwrap().to_bits(), value.to_bits());
        }
        assert_eq!(
            reviewed_decimal_string(f64::MAX).unwrap(),
            "1.7976931348623157e308"
        );
        assert_eq!(
            reviewed_decimal_string(f64::from_bits(1)).unwrap(),
            "5e-324"
        );
        assert!(reviewed_decimal_string(f64::NAN).is_err());
        assert!(reviewed_decimal_string(f64::INFINITY).is_err());
    }

    #[test]
    fn installed_proof_digest_contract_is_domain_separated_and_stable() {
        let native = |byte_len: u64, byte: char| InstalledPackageContentSealV1 {
            byte_len,
            sha256: byte.to_string().repeat(64),
        };
        let inspection = InspectionBindingWire {
            uasset: ExpectedSealWire {
                byte_len: 5,
                sha256: "5".repeat(64),
            },
            uexp: ExpectedSealWire {
                byte_len: 6,
                sha256: "6".repeat(64),
            },
            usmap: ExpectedSealWire {
                byte_len: 7,
                sha256: "7".repeat(64),
            },
        };
        let digest = installed_proof_binding_sha256(
            7,
            &native(1, '1'),
            &native(2, '2'),
            &native(3, '3'),
            &native(4, '4'),
            &inspection,
        )
        .unwrap();
        assert_eq!(
            digest,
            "827161c17b537a2b63095c51ff204cb398d653d3144bc012d276b4957cea5aed"
        );

        let changed = installed_proof_binding_sha256(
            8,
            &native(1, '1'),
            &native(2, '2'),
            &native(3, '3'),
            &native(4, '4'),
            &inspection,
        )
        .unwrap();
        assert_ne!(changed, digest);
    }

    #[test]
    fn canonical_head_seals_paths_and_wire_budgets_are_strict() {
        let temp = TempDir::new().unwrap();
        let head_json = publish_store(temp.path());
        assert!(parse_canonical_head(&head_json).is_ok());
        assert_eq!(
            parse_canonical_head(&format!(" {head_json}"))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_INVALID"
        );
        let duplicate = head_json.replacen('{', "{\"store_format\":1,", 1);
        assert_eq!(
            parse_canonical_head(&duplicate).unwrap_err().code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_INVALID"
        );

        assert!(validate_expected_seal(&ExpectedSealWire {
            byte_len: i64::MAX as u64,
            sha256: "a".repeat(64),
        })
        .is_ok());
        for invalid in [
            ExpectedSealWire {
                byte_len: 0,
                sha256: "a".repeat(64),
            },
            ExpectedSealWire {
                byte_len: i64::MAX as u64 + 1,
                sha256: "a".repeat(64),
            },
            ExpectedSealWire {
                byte_len: 1,
                sha256: "A".repeat(64),
            },
            ExpectedSealWire {
                byte_len: 1,
                sha256: "a".repeat(63),
            },
        ] {
            assert_eq!(
                validate_expected_seal(&invalid).unwrap_err().code,
                "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID"
            );
        }
        let mut zero_seal_payload = valid_shape();
        zero_seal_payload["expected_package_index_seal"]["byte_len"] = json!(0);
        assert_eq!(
            inspect_revision3_installed_dataasset_v1_raw(&raw_request(zero_seal_payload))["error"]
                ["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID"
        );
        let mut oversized_ordinal_payload = valid_shape();
        oversized_ordinal_payload["candidate_ordinal"] = json!(i64::MAX as u64 + 1);
        assert_eq!(
            inspect_revision3_installed_dataasset_v1_raw(&raw_request(oversized_ordinal_payload))
                ["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_CANDIDATE_INVALID"
        );
        assert!(validate_path("C:/bounded").is_ok());
        assert!(validate_path("").is_err());
        assert!(validate_path(&"x".repeat(MAX_PATH_BYTES + 1)).is_err());
        assert_eq!(
            inspect_revision3_installed_dataasset_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))["error"]
                ["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_LIMIT"
        );
    }

    #[test]
    fn public_dispatch_routes_the_raw_command_and_sanitizes_missing_game_paths() {
        let temp = TempDir::new().unwrap();
        let store_root = temp.path().join("store");
        let head_json = publish_store(&store_root);
        let missing_game = temp.path().join("PRIVATE-MISSING-GAME-MUST-NOT-ESCAPE");
        let before = tree_bytes(&store_root);
        let request = raw_request(json!({
            "candidate_ordinal": 0,
            "expected_head_json": head_json,
            "expected_package_index_seal": seal_json(1, 'a'),
            "expected_source_snapshot_seal": seal_json(2, 'b'),
            "game_root": missing_game,
            "root": store_root,
        }));
        let response: Value = serde_json::from_str(&crate::execute_json(&request)).unwrap();
        #[cfg(windows)]
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_IO"
        );
        #[cfg(not(windows))]
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PLATFORM_UNSUPPORTED"
        );
        let encoded = response.to_string();
        assert!(!encoded.contains("PRIVATE-MISSING-GAME-MUST-NOT-ESCAPE"));
        assert!(!encoded.contains("PRIVATE INSTALLED DATAASSET PROJECT"));
        assert!(!missing_game.exists());
        assert_eq!(tree_bytes(&store_root), before);

        let info: Value = serde_json::from_str(&crate::execute_json(
            r#"{"command":"core_info","payload":{}}"#,
        ))
        .unwrap();
        let commands = info["commands"].as_array().unwrap();
        assert!(commands.iter().any(|command| command == COMMAND));
        assert!(commands
            .iter()
            .any(|command| command == PREPARE_EDIT_COMMAND));
        assert!(commands
            .iter()
            .any(|command| command == PREPARE_REVIEWED_EDIT_COMMAND));
        assert!(commands
            .windows(2)
            .all(|pair| { pair[0].as_str().unwrap() < pair[1].as_str().unwrap() }));
    }

    #[test]
    fn head_conflict_wins_before_the_game_root_is_accessed() {
        let temp = TempDir::new().unwrap();
        let store_root = temp.path().join("store");
        let head_json = publish_store(&store_root);
        let mut stale: WorkingHead = serde_json::from_str(&head_json).unwrap();
        stale.snapshot.sha256 = Sha256Digest::from_bytes([0xee; 32]);
        let missing_game = temp.path().join("game-must-remain-missing");
        let response = inspect_revision3_installed_dataasset_v1_raw(&raw_request(json!({
            "candidate_ordinal": 0,
            "expected_head_json": serde_json::to_string(&stale).unwrap(),
            "expected_package_index_seal": seal_json(1, 'a'),
            "expected_source_snapshot_seal": seal_json(2, 'b'),
            "game_root": missing_game,
            "root": store_root,
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_CONFLICT"
        );
        assert!(!missing_game.exists());

        let mut edit = valid_edit_shape();
        edit["expected_head_json"] = json!(serde_json::to_string(&stale).unwrap());
        edit["game_root"] = json!(missing_game);
        edit["root"] = json!(store_root);
        let response = prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(edit));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_HEAD_CONFLICT"
        );
        assert!(!missing_game.exists());
    }

    #[test]
    fn response_budget_and_closed_error_mappings_are_stable() {
        let response = json!({"ok": true, "inspection": {"value": "abc"}});
        let exact = serde_json::to_vec(&response).unwrap().len();
        assert_eq!(
            enforce_response_budget(response.clone(), exact).unwrap(),
            response
        );
        assert_eq!(
            enforce_response_budget(response, exact - 1)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT"
        );
        assert_eq!(
            signed_wire_u64(i64::MAX as u64 + 1).unwrap_err().code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT"
        );
        assert_eq!(
            map_package_snapshot_error(InstalledPackageIndexErrorV1::ExecutableMismatch).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_GENERATION_MISMATCH"
        );
        assert_eq!(
            map_package_snapshot_error(InstalledPackageIndexErrorV1::TreeChanged).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_CHANGED"
        );
        assert_eq!(
            map_extraction_error(InstalledPackageExtractionErrorV1::Conversion).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_EXTRACTION_FAILED"
        );
        assert_eq!(
            map_extraction_error(InstalledPackageExtractionErrorV1::SourceEvidence).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_EXTRACTION_FAILED"
        );
        assert_eq!(
            map_usmap_error(InstalledUsmapErrorV1::MissingUsmap).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_MISSING"
        );
        assert_eq!(
            map_usmap_error(InstalledUsmapErrorV1::InventoryChanged).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_CHANGED"
        );
        let fixed = dataasset::fixed_inspect_verified_bytes_v1(Vec::new(), vec![0], vec![0], None)
            .unwrap_err();
        assert_eq!(
            map_fixed_inspection_error(fixed).code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_LIMIT"
        );
        let truncated = Failure::new("TEST", "é".repeat(MAX_ERROR_MESSAGE_BYTES));
        assert!(truncated.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(truncated.message.ends_with("..."));
    }

    #[cfg(windows)]
    struct WindowsFixture {
        _temp: TempDir,
        store_root: PathBuf,
        game_root: PathBuf,
        paks: PathBuf,
        head_json: String,
        package_index_seal: InstalledPackageContentSealV1,
        source_snapshot_seal: InstalledPackageContentSealV1,
    }

    #[cfg(windows)]
    fn write_valid_zen_fixture(utoc: &Path, components: [f64; 4]) {
        use std::collections::HashMap;
        use std::io::Cursor;
        use std::sync::Arc;

        use retoc::iostore_writer::IoStoreWriter;
        use retoc::legacy_asset::{
            EPackageFlags, FLegacyPackageFileSummary, FLegacyPackageHeader, FObjectExport,
            FObjectImport, FSerializedAssetBundle,
        };
        use retoc::logging::Log;
        use retoc::name_map::{EMappedNameType, FNameMap};
        use retoc::script_objects::{FPackageObjectIndex, FScriptObjectEntry, ZenScriptObjects};
        use retoc::version::EngineVersion;
        use retoc::zen::FPackageIndex;
        use retoc::zen_asset_conversion::build_zen_asset;
        use retoc::{build_verse_cell_store, EIoChunkType, FIoChunkId, UEPath, UEPathBuf};

        let version = EngineVersion::UE5_4;
        let mut package = FLegacyPackageHeader::default();
        package.summary.versioning_info.package_file_version = version.package_file_version();
        package.summary.versioning_info.is_unversioned = true;
        package.summary.package_name = TARGET.to_owned();
        package.summary.package_flags = EPackageFlags::Cooked as u32
            | EPackageFlags::FilterEditorOnly as u32
            | EPackageFlags::UsesUnversionedProperties as u32;
        let core_uobject = package.name_map.store("/Script/CoreUObject");
        let package_class = package.name_map.store("Package");
        let class_class = package.name_map.store("Class");
        let module_name = package.name_map.store("/Script/G1R");
        let class_name = package.name_map.store("FootstepTag");
        let module_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: package_class,
            object_name: module_name,
            ..FObjectImport::default()
        });
        let class_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: class_class,
            outer_index: FPackageIndex::create_import(module_index as u32),
            object_name: class_name,
            ..FObjectImport::default()
        });
        let object_name = package.name_map.store("DA_WolfFootsteps");
        let mut exports = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
        exports.extend_from_slice(&[0x00, 0x09]);
        for value in components {
            exports.extend_from_slice(&value.to_le_bytes());
        }
        exports.extend_from_slice(&1_i32.to_le_bytes());
        exports.extend_from_slice(&2_i32.to_le_bytes());
        exports.extend_from_slice(&3_i32.to_le_bytes());
        exports.extend_from_slice(&0_i32.to_le_bytes());
        exports.extend_from_slice(&2_i32.to_le_bytes());
        exports.extend_from_slice(&11_i32.to_le_bytes());
        exports.extend_from_slice(&0_i32.to_le_bytes());
        exports.extend_from_slice(&[0x80, 0x03, 0x01]);
        exports.extend_from_slice(&22_i32.to_le_bytes());
        exports.extend_from_slice(&1_i32.to_le_bytes());
        exports.extend_from_slice(&[0x00, 0x03, 0x01]);
        assert_eq!(exports.len(), 82);
        exports.extend_from_slice(&[0_u8; 4]);
        package.exports.push(FObjectExport {
            class_index: FPackageIndex::create_import(class_index as u32),
            object_name,
            serial_offset: 0,
            serial_size: exports.len() as i64,
            ..FObjectExport::default()
        });
        let mut header = Cursor::new(Vec::new());
        package
            .serialize(&mut header, None, &Log::no_log())
            .unwrap();
        exports.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());
        let bundle = FSerializedAssetBundle {
            asset_file_buffer: header.into_inner(),
            exports_file_buffer: exports,
            ..FSerializedAssetBundle::default()
        };
        let mut global_name_map = FNameMap::create(EMappedNameType::Global);
        let package_name = global_name_map.store("/Script/G1R");
        let imported_class_name = global_name_map.store("FootstepTag");
        let default_object_name = global_name_map.store("Default__FootstepTag");
        let package_index = FPackageObjectIndex::create_script_import("/Script/G1R");
        let imported_class_index =
            FPackageObjectIndex::create_script_import("/Script/G1R.FootstepTag");
        let default_object_index =
            FPackageObjectIndex::create_script_import("/Script/G1R.Default__FootstepTag");
        let script_entries = vec![
            FScriptObjectEntry {
                object_name: package_name,
                global_index: package_index,
                outer_index: FPackageObjectIndex::create_null(),
                cdo_class_index: FPackageObjectIndex::create_null(),
            },
            FScriptObjectEntry {
                object_name: imported_class_name,
                global_index: imported_class_index,
                outer_index: package_index,
                cdo_class_index: FPackageObjectIndex::create_null(),
            },
            FScriptObjectEntry {
                object_name: default_object_name,
                global_index: default_object_index,
                outer_index: package_index,
                cdo_class_index: imported_class_index,
            },
        ];
        let script_objects_table = ZenScriptObjects {
            global_name_map,
            script_object_lookup: script_entries
                .iter()
                .map(|entry| (entry.global_index, *entry))
                .collect(),
            script_objects: script_entries,
        };
        let mut converted = build_zen_asset(
            bundle,
            &HashMap::new(),
            UEPath::new(
                "../../../G1R/Content/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps.uasset",
            ),
            Some(version.package_file_version()),
            version.container_header_version(),
            false,
            Some(Arc::new(script_objects_table.clone())),
            Some(build_verse_cell_store(&Vec::new())),
            &Log::no_log(),
        )
        .unwrap();
        let mut writer = IoStoreWriter::new(
            utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        converted.write(&mut writer).unwrap();
        let mut script_objects = Vec::new();
        script_objects_table
            .serialize_new(&mut script_objects)
            .unwrap();
        writer
            .write_chunk(
                FIoChunkId::create(0, 0, EIoChunkType::ScriptObjects),
                Some(UEPath::new("../../../G1R/Content/ScriptObjects.bin")),
                &script_objects,
            )
            .unwrap();
        writer.finalize().unwrap();

        // The managed-stage generation proof independently follows the game's real split layout:
        // package data in G1R-Windows plus the global script-object table in global.*.
        let mut global_writer = IoStoreWriter::new(
            utoc.with_file_name("global.utoc"),
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        global_writer
            .write_chunk(
                FIoChunkId::create(0, 0, EIoChunkType::ScriptObjects),
                Some(UEPath::new("../../../G1R/Content/ScriptObjects.bin")),
                &script_objects,
            )
            .unwrap();
        global_writer.finalize().unwrap();
    }

    #[cfg(windows)]
    fn write_valid_usmap(path: &Path) {
        let mapping = usmap::Usmap {
            enums: Vec::new(),
            structs: vec![
                usmap::Struct {
                    name: "FootstepTag".to_owned(),
                    super_struct: None,
                    properties: vec![
                        usmap::Property {
                            name: "BoneData".to_owned(),
                            array_dim: 1,
                            index: 0,
                            inner: usmap::PropertyInner::Struct {
                                name: "BoneFeetData".to_owned(),
                            },
                        },
                        usmap::Property {
                            name: "BonesToTrack".to_owned(),
                            array_dim: 1,
                            index: 1,
                            inner: usmap::PropertyInner::Map {
                                key: Box::new(usmap::PropertyInner::Name),
                                value: Box::new(usmap::PropertyInner::Struct {
                                    name: "BoneTrackedData".to_owned(),
                                }),
                            },
                        },
                    ],
                },
                usmap::Struct {
                    name: "BoneFeetData".to_owned(),
                    super_struct: None,
                    properties: vec![
                        usmap::Property {
                            name: "FeetTextureSize".to_owned(),
                            array_dim: 1,
                            index: 0,
                            inner: usmap::PropertyInner::Struct {
                                name: "Vector4".to_owned(),
                            },
                        },
                        usmap::Property {
                            name: "Diffuse".to_owned(),
                            array_dim: 1,
                            index: 1,
                            inner: usmap::PropertyInner::Object,
                        },
                        usmap::Property {
                            name: "Normal".to_owned(),
                            array_dim: 1,
                            index: 2,
                            inner: usmap::PropertyInner::Object,
                        },
                        usmap::Property {
                            name: "AO".to_owned(),
                            array_dim: 1,
                            index: 3,
                            inner: usmap::PropertyInner::Object,
                        },
                    ],
                },
                usmap::Struct {
                    name: "BoneTrackedData".to_owned(),
                    super_struct: None,
                    properties: vec![usmap::Property {
                        name: "InvertX".to_owned(),
                        array_dim: 1,
                        index: 0,
                        inner: usmap::PropertyInner::Bool,
                    }],
                },
                usmap::Struct {
                    name: "Vector4".to_owned(),
                    super_struct: None,
                    properties: ["X", "Y", "Z", "W"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, name)| usmap::Property {
                            name: name.to_owned(),
                            array_dim: 1,
                            index: index as u16,
                            inner: usmap::PropertyInner::Double,
                        })
                        .collect(),
                },
            ],
            cext: None,
            ppth: Some(usmap::ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: vec![
                    "/Script/G1R".to_owned(),
                    "/Script/G1R".to_owned(),
                    "/Script/G1R".to_owned(),
                    "/Script/CoreUObject".to_owned(),
                ],
            }),
            eatr: Some(usmap::ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags: vec![
                    usmap::StructFlags {
                        type_: usmap::FlagsType::Class,
                        value: 0,
                        prop_flags: Vec::new(),
                    },
                    usmap::StructFlags {
                        type_: usmap::FlagsType::Struct,
                        value: 0,
                        prop_flags: Vec::new(),
                    },
                    usmap::StructFlags {
                        type_: usmap::FlagsType::Struct,
                        value: 0,
                        prop_flags: Vec::new(),
                    },
                    usmap::StructFlags {
                        type_: usmap::FlagsType::Struct,
                        value: 0,
                        prop_flags: Vec::new(),
                    },
                ],
            }),
            envp: None,
        };
        let mut bytes = Vec::new();
        mapping.write(&mut bytes).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[cfg(windows)]
    impl WindowsFixture {
        fn new() -> Self {
            Self::with_valid_payload(false)
        }

        fn valid() -> Self {
            Self::with_valid_payload(true)
        }

        fn with_valid_payload(valid: bool) -> Self {
            Self::with_valid_payload_and_components(valid, [100.0, 200.0, 300.0, 400.0])
        }

        fn valid_with_components(components: [f64; 4]) -> Self {
            Self::with_valid_payload_and_components(true, components)
        }

        fn with_valid_payload_and_components(valid: bool, components: [f64; 4]) -> Self {
            use retoc::iostore_writer::IoStoreWriter;
            use retoc::version::EngineVersion;
            use retoc::{EIoChunkType, FIoChunkId, FIoContainerId, FPackageId, UEPath, UEPathBuf};

            // Keep the synthetic game outside the OS temporary directory because the production
            // converter deliberately rejects a temp parent that contains the live game tree.
            let temp = tempfile::Builder::new()
                .prefix(".gore-ffi-installed-")
                .tempdir_in(std::env::current_dir().unwrap())
                .unwrap();
            let store_root = temp.path().join("store");
            let head_json = publish_store(&store_root);
            let game_root = temp.path().join("game");
            let g1r = game_root.join("G1R");
            let paks = g1r.join("Content/Paks");
            let executable = g1r.join("Binaries/Win64/G1R-Win64-Shipping.exe");
            fs::create_dir_all(&paks).unwrap();
            fs::create_dir_all(executable.parent().unwrap()).unwrap();
            fs::write(&executable, EXE_BYTES).unwrap();

            if valid {
                write_valid_zen_fixture(&paks.join("G1R-Windows.utoc"), components);
            } else {
                let version = EngineVersion::UE5_4;
                let mut writer = IoStoreWriter::new(
                    paks.join("G1R-Windows.utoc"),
                    version.toc_version(),
                    None,
                    UEPathBuf::from("../../../"),
                )
                .unwrap();
                let package_id = FPackageId(FIoContainerId::from_name(TARGET).0);
                writer
                    .write_chunk(
                        FIoChunkId::from_package_id(package_id, 0, EIoChunkType::ExportBundleData),
                        Some(UEPath::new(
                            "../../../G1R/Content/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps.uasset",
                        )),
                        b"invalid payload is indexed but must fail closed during extraction",
                    )
                    .unwrap();
                writer.finalize().unwrap();
            }
            let usmap = g1r.join("Binaries/Win64/ue4ss");
            fs::create_dir_all(&usmap).unwrap();
            if valid {
                write_valid_usmap(&usmap.join("Mappings.usmap"));
            } else {
                fs::write(usmap.join("Mappings.usmap"), b"bounded invalid mapping").unwrap();
            }

            let snapshot = inspect_installed_package_index_v1(
                &game_root,
                ExpectedInstalledExecutableV1 {
                    byte_len: EXE_BYTES.len() as u64,
                    sha256: Sha256::digest(EXE_BYTES).into(),
                },
            )
            .unwrap();
            let package_index_seal = snapshot.index_seal().clone();
            let source_snapshot_seal = snapshot.source_snapshot_seal().clone();
            drop(snapshot);
            Self {
                _temp: temp,
                store_root,
                game_root,
                paks,
                head_json,
                package_index_seal,
                source_snapshot_seal,
            }
        }

        fn request(
            &self,
            ordinal: u64,
            package: &ExpectedSealWire,
            source: &ExpectedSealWire,
        ) -> String {
            raw_request(json!({
                "candidate_ordinal": ordinal,
                "expected_head_json": self.head_json,
                "expected_package_index_seal": package,
                "expected_source_snapshot_seal": source,
                "game_root": self.game_root,
                "root": self.store_root,
            }))
        }

        fn package_wire(&self) -> ExpectedSealWire {
            ExpectedSealWire {
                byte_len: self.package_index_seal.byte_len,
                sha256: self.package_index_seal.sha256.clone(),
            }
        }

        fn source_wire(&self) -> ExpectedSealWire {
            ExpectedSealWire {
                byte_len: self.source_snapshot_seal.byte_len,
                sha256: self.source_snapshot_seal.sha256.clone(),
            }
        }

        fn edit_payload(&self, inspection: &Value) -> Value {
            let selector = inspection["inspection"]["exports"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|export| export["leaves"].as_array().unwrap())
                .find_map(|leaf| {
                    (leaf["selector"]["kind"] == "vector4_f64x4").then(|| leaf["selector"].clone())
                })
                .unwrap();
            json!({
                "candidate_ordinal": inspection["candidate_ordinal"],
                "expected_head_json": self.head_json,
                "expected_inspection_binding": {
                    "uasset": {
                        "byte_len": inspection["inspection"]["input"]["uasset_length"],
                        "sha256": inspection["inspection"]["binding"]["package_seal"]["uasset_sha256"],
                    },
                    "uexp": {
                        "byte_len": inspection["inspection"]["input"]["uexp_length"],
                        "sha256": inspection["inspection"]["binding"]["package_seal"]["uexp_sha256"],
                    },
                    "usmap": {
                        "byte_len": inspection["inspection"]["input"]["usmap_length"],
                        "sha256": inspection["inspection"]["binding"]["usmap_sha256"],
                    },
                },
                "expected_package_index_seal": inspection["package_index_seal"],
                "expected_source_snapshot_seal": inspection["source_snapshot_seal"],
                "expected_usmap_content_seal": inspection["usmap_content_seal"],
                "expected_usmap_inventory_seal": inspection["usmap_inventory_seal"],
                "game_root": self.game_root,
                "replacement": {
                    "kind": "vector4_f64x4",
                    "x": "125",
                    "y": "225",
                    "z": "300",
                    "w": "400",
                },
                "root": self.store_root,
                "selector": selector,
            })
        }

        fn reviewed_payload(&self, x: &str, y: &str) -> Value {
            json!({
                "candidate_ordinal": 0,
                "expected_head_json": self.head_json,
                "expected_package_index_seal": self.package_wire(),
                "expected_source_snapshot_seal": self.source_wire(),
                "game_root": self.game_root,
                "reviewed_edit": {
                    "format": gore_asset::REVIEWED_DATAASSET_FORMAT_V1,
                    "schema_id": gore_asset::REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID,
                    "schema_revision": gore_asset::REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                    "field_id": gore_asset::REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
                    "value": {"x": x, "y": y},
                },
                "root": self.store_root,
            })
        }
    }

    #[cfg(windows)]
    #[test]
    fn valid_installed_package_returns_the_closed_whole_package_inspection_response() {
        let fixture = WindowsFixture::valid();
        let package = fixture.package_wire();
        let source = fixture.source_wire();
        let store_before = tree_bytes(&fixture.store_root);
        let game_before = tree_bytes(&fixture.game_root);

        let response =
            inspect_revision3_installed_dataasset_v1_raw(&fixture.request(0, &package, &source));

        assert_eq!(response["ok"], true, "{response}");
        let response_keys = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            response_keys,
            BTreeSet::from([
                "authority_status",
                "build_status",
                "candidate_ordinal",
                "head_json",
                "inspection",
                "mutation_status",
                "ok",
                "outcome",
                "package_id_hex",
                "package_index_seal",
                "project_id",
                "project_revision",
                "publication_status",
                "runtime_status",
                "scope",
                "source_snapshot_seal",
                "target_path",
                "usmap_content_seal",
                "usmap_inventory_seal",
            ])
        );
        assert_eq!(response["outcome"], "inspection_only");
        assert_eq!(response["head_json"], fixture.head_json);
        assert_eq!(response["project_id"], "68".repeat(16));
        assert_eq!(response["project_revision"], 9);
        assert_eq!(response["candidate_ordinal"], 0);
        assert_eq!(response["target_path"], TARGET);
        assert_eq!(response["package_id_hex"].as_str().unwrap().len(), 16);
        assert_eq!(
            response["package_index_seal"],
            serde_json::to_value(&fixture.package_index_seal).unwrap()
        );
        assert_eq!(
            response["source_snapshot_seal"],
            serde_json::to_value(&fixture.source_snapshot_seal).unwrap()
        );
        assert_eq!(
            response["usmap_content_seal"]["sha256"],
            response["inspection"]["binding"]["usmap_sha256"]
        );
        assert!(response["usmap_inventory_seal"]["byte_len"]
            .as_u64()
            .is_some_and(|length| length > 0));
        assert_eq!(
            response["scope"],
            "selected_installed_dataasset_fixed_leaf_inspection_only"
        );
        assert_eq!(response["mutation_status"], "not_supported");
        assert_eq!(response["build_status"], "not_evaluated");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(response["authority_status"], "not_granted");
        assert_eq!(
            response["inspection"]["format"],
            "gore.dataasset.fixed-inspect.v1"
        );
        assert!(response["inspection"]["selection"]["export_index"].is_null());
        assert_eq!(response["inspection"]["status"], "walked");
        assert_eq!(response["inspection"]["summary"]["walked_exports"], 1);
        assert_eq!(response["inspection"]["summary"]["editable_leaves"], 2);
        assert_eq!(
            response["inspection"]["exports"][0]["class_path"],
            "/Script/G1R.FootstepTag"
        );
        assert_eq!(
            response["inspection"]["exports"][0]["leaves"][0]["selector"]["expected_hex"],
            "000000000000594000000000000069400000000000c072400000000000007940"
        );
        assert_eq!(tree_bytes(&fixture.store_root), store_before);
        assert_eq!(tree_bytes(&fixture.game_root), game_before);

        let encoded = response.to_string();
        assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
        assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
        assert!(!encoded.contains("Mappings.usmap"));
        assert!(!encoded.contains("PRIVATE INSTALLED DATAASSET PROJECT"));
        for forbidden in [
            "uasset_path",
            "uexp_path",
            "usmap_path",
            "output_path",
            "metadata_utocs",
            "source_utoc",
            "raw_bytes",
        ] {
            assert!(!encoded.contains(forbidden), "leaked {forbidden}");
        }
        assert_eq!(
            inspect_revision3_installed_dataasset_v1_inner(
                &fixture.request(0, &package, &source),
                128,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT"
        );
        assert_eq!(tree_bytes(&fixture.store_root), store_before);
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[cfg(windows)]
    #[test]
    fn exact_installed_inspection_promotes_to_one_unpublished_typed_stage() {
        let fixture = WindowsFixture::valid();
        let package = fixture.package_wire();
        let source = fixture.source_wire();
        let inspection =
            inspect_revision3_installed_dataasset_v1_raw(&fixture.request(0, &package, &source));
        assert_eq!(inspection["ok"], true, "{inspection}");
        let fixed_head_before = fs::read(fixture.store_root.join("gore-project.json")).unwrap();
        let game_before = tree_bytes(&fixture.game_root);

        let edit_payload = fixture.edit_payload(&inspection);
        let expected_inspection_binding = edit_payload["expected_inspection_binding"].clone();
        let response =
            prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(edit_payload));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["revision"], 10);
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["artifact_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            response["installed_source"]["format"],
            INSTALLED_SOURCE_FORMAT
        );
        assert_eq!(response["installed_source"]["candidate_ordinal"], 0);
        assert_eq!(
            response["installed_source"]["package_index_seal"],
            inspection["package_index_seal"]
        );
        assert_eq!(
            response["installed_source"]["source_snapshot_seal"],
            inspection["source_snapshot_seal"]
        );
        assert_eq!(
            response["installed_source"]["usmap_content_seal"],
            inspection["usmap_content_seal"]
        );
        assert_eq!(
            response["installed_source"]["usmap_inventory_seal"],
            inspection["usmap_inventory_seal"]
        );
        assert_eq!(
            response["installed_source"]["inspection_binding"],
            expected_inspection_binding
        );
        for digest_field in ["intent_binding_sha256", "installed_proof_binding_sha256"] {
            let digest = response[digest_field].as_str().unwrap();
            assert_eq!(digest.len(), 64);
            assert!(is_lower_hex(digest));
        }
        assert_eq!(response["stage"]["manifest"]["target_path"], TARGET);
        assert_eq!(
            response["stage"]["manifest"]["replacement_hex"],
            "0000000000405f400000000000206c400000000000c072400000000000007940"
        );
        assert_eq!(
            fs::read(fixture.store_root.join("gore-project.json")).unwrap(),
            fixed_head_before
        );
        assert_eq!(tree_bytes(&fixture.game_root), game_before);

        let encoded = response.to_string();
        assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
        assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
        for forbidden in [
            "patch_receipt_path",
            "extract_receipt_path",
            "uasset_path",
            "uexp_path",
            "usmap_path",
            "output_path",
            "raw_bytes",
        ] {
            assert!(!encoded.contains(forbidden), "leaked {forbidden}");
        }

        assert_eq!(
            prepare_revision3_installed_dataasset_edit_v1_inner(
                &raw_edit_request(fixture.edit_payload(&inspection)),
                128,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT"
        );
        assert_eq!(
            fs::read(fixture.store_root.join("gore-project.json")).unwrap(),
            fixed_head_before
        );
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[cfg(windows)]
    #[test]
    fn reviewed_footstep_intent_server_selects_and_prepares_one_unpublished_stage() {
        let fixture = WindowsFixture::valid();
        let fixed_head_before = fs::read(fixture.store_root.join("gore-project.json")).unwrap();
        let game_before = tree_bytes(&fixture.game_root);

        let response = prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
            &raw_reviewed_edit_request(fixture.reviewed_payload("125", "225")),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["revision"], 10);
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["artifact_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            response["reviewed_edit"],
            json!({
                "format": 1,
                "schema_id": "g1r.tracking.footstep-preset",
                "schema_revision": 1,
                "field_id": "feet_texture_size",
                "target_id": "g1r:dataasset:footstep-preset:wolf",
            })
        );
        assert_eq!(
            response["reviewed_before"],
            json!({"x": "100", "y": "200", "z": "300", "w": "400"})
        );
        assert_eq!(
            response["reviewed_after"],
            json!({"x": "125", "y": "225", "z": "300", "w": "400"})
        );
        assert_eq!(response["stage"]["manifest"]["target_path"], TARGET);
        assert_eq!(
            response["stage"]["manifest"]["replacement_hex"],
            "0000000000405f400000000000206c400000000000c072400000000000007940"
        );
        assert_eq!(response["installed_source"]["candidate_ordinal"], 0);
        for digest_field in [
            "intent_binding_sha256",
            "installed_proof_binding_sha256",
            "reviewed_intent_binding_sha256",
        ] {
            let digest = response[digest_field].as_str().unwrap();
            assert_eq!(digest.len(), 64);
            assert!(is_lower_hex(digest));
        }
        assert_eq!(
            fs::read(fixture.store_root.join("gore-project.json")).unwrap(),
            fixed_head_before
        );
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
        let encoded = response.to_string();
        assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
        assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
        for forbidden in [
            "patch_receipt_path",
            "extract_receipt_path",
            "uasset_path",
            "uexp_path",
            "usmap_path",
            "output_path",
            "raw_bytes",
        ] {
            assert!(!encoded.contains(forbidden), "leaked {forbidden}");
        }

        let inspection = inspect_revision3_installed_dataasset_v1_raw(&fixture.request(
            0,
            &fixture.package_wire(),
            &fixture.source_wire(),
        ));
        assert_eq!(inspection["ok"], true, "{inspection}");
        let requested = ReviewedFootstepPresetSizeV1::try_new(125.0, 225.0).unwrap();
        assert!(select_exact_reviewed_edit(TARGET, &inspection["inspection"], requested,).is_ok());
        assert_eq!(
            select_exact_reviewed_edit(
                "/Game/Characters/DA_Asghan",
                &inspection["inspection"],
                requested,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_MATCH_INVALID"
        );

        let mut duplicate = inspection["inspection"].clone();
        let matching_leaf = duplicate["exports"][0]["leaves"][0].clone();
        duplicate["exports"][0]["leaves"]
            .as_array_mut()
            .unwrap()
            .push(matching_leaf);
        assert_eq!(
            select_exact_reviewed_edit(TARGET, &duplicate, requested)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_MATCH_INVALID"
        );

        let mut near_match = inspection["inspection"].clone();
        let mut near_leaf = near_match["exports"][0]["leaves"][0].clone();
        near_leaf["selector"]["path"][2]["property_name"] = json!("FeetTextureSizeNearMatch");
        near_match["exports"][0]["leaves"]
            .as_array_mut()
            .unwrap()
            .push(near_leaf);
        assert!(select_exact_reviewed_edit(TARGET, &near_match, requested).is_ok());

        let no_change = prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
            &raw_reviewed_edit_request(fixture.reviewed_payload("100", "200")),
        );
        assert_eq!(
            no_change["error"]["code"],
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INVALID"
        );
        assert!(!no_change.to_string().contains("project_json"));
        assert_eq!(
            prepare_revision3_reviewed_installed_dataasset_edit_v1_inner(
                &raw_reviewed_edit_request(fixture.reviewed_payload("125", "225")),
                128,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT"
        );
        assert_eq!(
            fs::read(fixture.store_root.join("gore-project.json")).unwrap(),
            fixed_head_before
        );
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[cfg(windows)]
    #[test]
    fn reviewed_extreme_requested_and_preserved_components_round_trip_exactly() {
        let current_z = -f64::MAX;
        let current_w = f64::from_bits(1);
        let fixture = WindowsFixture::valid_with_components([100.0, 200.0, current_z, current_w]);
        let requested_x = "9".repeat(64);
        let requested_y = format!("0.{}1", "0".repeat(61));
        assert_eq!(requested_x.len(), 64);
        assert_eq!(requested_y.len(), 64);

        let response = prepare_revision3_reviewed_installed_dataasset_edit_v1_raw(
            &raw_reviewed_edit_request(fixture.reviewed_payload(&requested_x, &requested_y)),
        );
        assert_eq!(response["ok"], true, "{response}");

        let component_bits = |container: &str, component: &str| {
            response[container][component]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap()
                .to_bits()
        };
        assert_eq!(
            component_bits("reviewed_after", "x"),
            requested_x.parse::<f64>().unwrap().to_bits()
        );
        assert_eq!(
            component_bits("reviewed_after", "y"),
            requested_y.parse::<f64>().unwrap().to_bits()
        );
        assert_eq!(component_bits("reviewed_before", "z"), current_z.to_bits());
        assert_eq!(component_bits("reviewed_before", "w"), current_w.to_bits());
        assert_eq!(component_bits("reviewed_after", "z"), current_z.to_bits());
        assert_eq!(component_bits("reviewed_after", "w"), current_w.to_bits());
        assert_eq!(response["reviewed_after"]["x"], "1e64");
        assert_eq!(response["reviewed_before"]["z"], "-1.7976931348623157e308");
        assert_eq!(response["reviewed_before"]["w"], "5e-324");

        for container in ["reviewed_before", "reviewed_after"] {
            for component in ["x", "y", "z", "w"] {
                let encoded = response[container][component].as_str().unwrap();
                assert!(encoded.len() <= 64, "{container}.{component}: {encoded}");
                assert!(
                    is_canonical_reviewed_response_decimal(encoded),
                    "{container}.{component}: {encoded}"
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn final_drift_check_outranks_decorator_and_response_budget_failure() {
        let fixture = WindowsFixture::valid();
        let inspection = inspect_revision3_installed_dataasset_v1_raw(&fixture.request(
            0,
            &fixture.package_wire(),
            &fixture.source_wire(),
        ));
        assert_eq!(inspection["ok"], true, "{inspection}");
        let payload: PrepareInstalledDataAssetEditWirePayload =
            serde_json::from_value(fixture.edit_payload(&inspection)).unwrap();
        let late_source = fixture.paks.join("drift-during-response-decoration.txt");
        let decorated = std::cell::Cell::new(false);

        let failure = prepare_revision3_installed_dataasset_edit_v1_payload_with_response(
            payload,
            |mut response| {
                response
                    .as_object_mut()
                    .unwrap()
                    .insert("reviewed_decoration_probe".to_owned(), json!(true));
                decorated.set(true);
                fs::write(&late_source, b"drift inside bounded response decorator").unwrap();
                enforce_reviewed_edit_response_budget(response, 1)
            },
        )
        .unwrap_err();
        assert!(decorated.get());
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_CHANGED"
        );
    }

    #[cfg(windows)]
    #[test]
    fn installed_edit_rebuilds_every_prior_proof_before_staging() {
        let fixture = WindowsFixture::valid();
        let inspection = inspect_revision3_installed_dataasset_v1_raw(&fixture.request(
            0,
            &fixture.package_wire(),
            &fixture.source_wire(),
        ));
        assert_eq!(inspection["ok"], true, "{inspection}");
        let fixed_head_before = fs::read(fixture.store_root.join("gore-project.json")).unwrap();
        let game_before = tree_bytes(&fixture.game_root);
        let changed_digest = |value: &mut Value| {
            let mut digest = value.as_str().unwrap().to_owned();
            let replacement = if digest.starts_with("ff") { "00" } else { "ff" };
            digest.replace_range(..2, replacement);
            *value = Value::String(digest);
        };

        let mut cases = Vec::new();
        let mut payload = fixture.edit_payload(&inspection);
        payload["candidate_ordinal"] = json!(1);
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID",
        ));
        let mut payload = fixture.edit_payload(&inspection);
        changed_digest(&mut payload["expected_package_index_seal"]["sha256"]);
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_PACKAGE_INDEX_MISMATCH",
        ));
        let mut payload = fixture.edit_payload(&inspection);
        changed_digest(&mut payload["expected_source_snapshot_seal"]["sha256"]);
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SOURCE_SNAPSHOT_MISMATCH",
        ));
        let mut payload = fixture.edit_payload(&inspection);
        changed_digest(&mut payload["expected_usmap_content_seal"]["sha256"]);
        payload["expected_inspection_binding"]["usmap"] =
            payload["expected_usmap_content_seal"].clone();
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_CONTENT_MISMATCH",
        ));
        let mut payload = fixture.edit_payload(&inspection);
        changed_digest(&mut payload["expected_usmap_inventory_seal"]["sha256"]);
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_INVENTORY_MISMATCH",
        ));
        let mut payload = fixture.edit_payload(&inspection);
        changed_digest(&mut payload["expected_inspection_binding"]["uexp"]["sha256"]);
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_BINDING_MISMATCH",
        ));
        let mut payload = fixture.edit_payload(&inspection);
        payload["selector"]["expected_hex"] = json!("00");
        cases.push((
            payload,
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SELECTOR_MISMATCH",
        ));

        for (payload, expected_code) in cases {
            let response =
                prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(payload));
            assert_eq!(response["error"]["code"], expected_code, "{response}");
            let encoded = response.to_string();
            assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
            assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
        }
        assert_eq!(
            fs::read(fixture.store_root.join("gore-project.json")).unwrap(),
            fixed_head_before
        );
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[cfg(windows)]
    #[test]
    fn installed_edit_rejects_package_and_usmap_drift_since_inspection() {
        let package_fixture = WindowsFixture::valid();
        let inspection = inspect_revision3_installed_dataasset_v1_raw(&package_fixture.request(
            0,
            &package_fixture.package_wire(),
            &package_fixture.source_wire(),
        ));
        assert_eq!(inspection["ok"], true, "{inspection}");
        fs::write(
            package_fixture.paks.join("late-source.txt"),
            b"source drift after inspection",
        )
        .unwrap();
        let response = prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(
            package_fixture.edit_payload(&inspection),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SOURCE_SNAPSHOT_MISMATCH",
            "{response}"
        );

        let usmap_fixture = WindowsFixture::valid();
        let inspection = inspect_revision3_installed_dataasset_v1_raw(&usmap_fixture.request(
            0,
            &usmap_fixture.package_wire(),
            &usmap_fixture.source_wire(),
        ));
        assert_eq!(inspection["ok"], true, "{inspection}");
        fs::write(
            usmap_fixture
                .game_root
                .join("G1R/Binaries/Win64/ue4ss/Mappings.usmap"),
            b"changed after exact installed inspection",
        )
        .unwrap();
        let response = prepare_revision3_installed_dataasset_edit_v1_raw(&raw_edit_request(
            usmap_fixture.edit_payload(&inspection),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_CONTENT_MISMATCH",
            "{response}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn exact_seals_and_ordinal_are_rebuilt_before_server_side_extraction() {
        let fixture = WindowsFixture::new();
        let store_before = tree_bytes(&fixture.store_root);
        let game_before = tree_bytes(&fixture.game_root);
        let package = fixture.package_wire();
        let source = fixture.source_wire();

        let out_of_range =
            inspect_revision3_installed_dataasset_v1_raw(&fixture.request(1, &package, &source));
        assert_eq!(
            out_of_range["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_CANDIDATE_INVALID"
        );

        let mut wrong_package = package.clone();
        let replacement = if wrong_package.sha256.starts_with("ff") {
            "00"
        } else {
            "ff"
        };
        wrong_package.sha256.replace_range(..2, replacement);
        let package_mismatch = inspect_revision3_installed_dataasset_v1_raw(&fixture.request(
            0,
            &wrong_package,
            &source,
        ));
        assert_eq!(
            package_mismatch["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PACKAGE_INDEX_MISMATCH"
        );

        let mut wrong_source = source.clone();
        let replacement = if wrong_source.sha256.starts_with("ff") {
            "00"
        } else {
            "ff"
        };
        wrong_source.sha256.replace_range(..2, replacement);
        let source_mismatch = inspect_revision3_installed_dataasset_v1_raw(&fixture.request(
            0,
            &package,
            &wrong_source,
        ));
        assert_eq!(
            source_mismatch["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_SOURCE_SNAPSHOT_MISMATCH"
        );

        let extraction =
            inspect_revision3_installed_dataasset_v1_raw(&fixture.request(0, &package, &source));
        assert_eq!(
            extraction["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_EXTRACTION_FAILED"
        );
        for response in [out_of_range, package_mismatch, source_mismatch, extraction] {
            let encoded = response.to_string();
            assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
            assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
            assert!(!encoded.contains("Mappings.usmap"));
            assert!(!encoded.contains("invalid payload"));
        }
        assert_eq!(tree_bytes(&fixture.store_root), store_before);
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[cfg(windows)]
    #[test]
    fn missing_usmap_and_source_snapshot_drift_fail_with_closed_retry_codes() {
        let fixture = WindowsFixture::new();
        let package = fixture.package_wire();
        let source = fixture.source_wire();
        fs::remove_dir_all(fixture.game_root.join("G1R/Binaries/Win64/ue4ss")).unwrap();
        let response =
            inspect_revision3_installed_dataasset_v1_raw(&fixture.request(0, &package, &source));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_IO"
        );

        fs::write(fixture.paks.join("late.txt"), b"source snapshot drift").unwrap();
        let response =
            inspect_revision3_installed_dataasset_v1_raw(&fixture.request(0, &package, &source));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_SOURCE_SNAPSHOT_MISMATCH"
        );
    }
}
