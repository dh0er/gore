//! Exact-current, reviewed-only revision-3 DataAsset build transport.
//!
//! The route accepts only Store/project identity, one virtual staged target, one live game root,
//! and one absent output directory. Native authoring code owns the complete Store/live replay,
//! pack, semantic readback, canonical receipt, final source gate, and atomic no-clobber lifecycle.
//! This boundary never accepts raw package/USMAP bytes, selectors, replacements, receipt paths,
//! overwrite flags, deployment, or runtime authority.

use std::path::Path;

use gore_asset::dataasset_workflow::validate_game_asset_path;
use gore_authoring::{
    ProjectRevision3, Revision3DataAssetStagingErrorV1, Revision3ReviewedDataAssetBuildErrorV1,
    Revision3ReviewedDataAssetBuildPublicationV1, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits,
    MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1, MAX_PROJECT_JSON_BYTES,
    REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FILE_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_build_revision3_reviewed_dataasset_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_TARGET_PATH_BYTES: usize = 512;
const MAX_PACK_NAME_BYTES: usize = 96;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_HEAD_JSON_BYTES * 2
    + MAX_PATH_BYTES * 6
    + MAX_TARGET_PATH_BYTES * 2
    + MAX_PACK_NAME_BYTES * 2
    + 4096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildReviewedDataAssetWirePayload {
    current_project_json: String,
    expected_head_json: String,
    game_root: String,
    output: String,
    pack_name: String,
    root: String,
    target_path: String,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalKind {
    Published,
    PublishedWithCleanupWarning,
    PublicationUncertain,
}

pub(super) fn build_revision3_reviewed_dataasset_v1_raw(input: &str) -> Value {
    build_revision3_reviewed_dataasset_v1_inner(input).unwrap_or_else(Failure::response)
}

fn build_revision3_reviewed_dataasset_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: BuildReviewedDataAssetWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let expected_project =
        ProjectRevision3::from_json(&payload.current_project_json).map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID",
                "current_project_json is not an exact canonical closed revision-3 project",
            )
        })?;
    if expected_project.revision > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_RESPONSE_LIMIT",
            "project revision is outside the signed response range",
        ));
    }

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let publication = store
        .build_revision3_reviewed_dataasset_v1(
            &expected_head,
            &expected_project,
            Path::new(&payload.game_root),
            &payload.target_path,
            &payload.pack_name,
            Path::new(&payload.output),
        )
        .map_err(map_build_error)?;

    // The authoring API returns only after a typed terminal publication exists. Everything below
    // is a closed, bounded projection from already validated values; no filesystem or parser work
    // remains that could turn a published build into an error response.
    Ok(publication_response(publication, &payload.output))
}

fn validate_payload(payload: &BuildReviewedDataAssetWirePayload) -> Result<(), Failure> {
    if payload.current_project_json.is_empty()
        || payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID",
            "current_project_json is empty or exceeds its closed project limit",
        ));
    }
    for (value, label) in [
        (&payload.root, "managed Store root"),
        (&payload.game_root, "game installation root"),
        (&payload.output, "DataAsset build output"),
    ] {
        if value.is_empty() || value.len() > MAX_PATH_BYTES || value.contains('\0') {
            return Err(Failure::new(
                "AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_INVALID",
                format!("{label} is empty, unsafe, or exceeds its bounded path limit"),
            ));
        }
    }
    if payload.target_path.len() > MAX_TARGET_PATH_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID",
            "target_path is not a closed extensionless /Game package path",
        ));
    }
    validate_game_asset_path(
        &payload.target_path,
        "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET",
    )
    .map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID",
            "target_path is not a closed extensionless /Game package path",
        )
    })?;
    validate_pack_name(&payload.pack_name)?;
    Ok(())
}

fn validate_pack_name(value: &str) -> Result<(), Failure> {
    if value.is_empty()
        || value.len() > MAX_PACK_NAME_BYTES
        || !value.is_ascii()
        || !matches!(
            value.as_bytes().first(),
            Some(byte) if byte.is_ascii_alphanumeric()
        )
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || windows_reserved_name(value)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_PACK_NAME_INVALID",
            "pack_name must be a non-reserved 1..=96 byte ASCII component",
        ));
    }
    Ok(())
}

fn windows_reserved_name(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_LIMIT",
            "reviewed DataAsset build request exceeds its bounded wire limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    if serde_json::to_string(&request).map_err(|_| invariant())? != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_INVALID",
            "expected_head_json is empty or exceeds its closed head limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_INVALID",
            "expected_head_json is not a valid closed working-store head",
        )
    })?;
    if serde_json::to_string(&head).map_err(|_| invariant())? != input
        || head.snapshot.byte_len == 0
        || head.snapshot.byte_len > WorkingStoreLimits::default().max_snapshot_bytes as u64
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_INVALID",
            "expected_head_json is not the exact supported canonical head",
        ));
    }
    Ok(head)
}

fn publication_response(
    publication: Revision3ReviewedDataAssetBuildPublicationV1,
    output_spelling: &str,
) -> Value {
    let terminal = match &publication {
        Revision3ReviewedDataAssetBuildPublicationV1::Published(_) => TerminalKind::Published,
        Revision3ReviewedDataAssetBuildPublicationV1::PublishedWithCleanupWarning(_) => {
            TerminalKind::PublishedWithCleanupWarning
        }
        Revision3ReviewedDataAssetBuildPublicationV1::PublicationUncertain(_) => {
            TerminalKind::PublicationUncertain
        }
    };
    let published = publication.published();
    let receipt = published.caller_receipt();
    let receipt_seal = published.receipt_seal();
    let files = receipt
        .files()
        .iter()
        .map(|file| {
            json!({
                "relative_name": file.relative_name(),
                "byte_len": file.byte_len(),
                "sha256": file.sha256().to_string(),
            })
        })
        .collect::<Vec<_>>();
    let (outcome, artifact_publication_status, warning) = terminal_metadata(terminal);
    let response = json!({
        "ok": true,
        "outcome": outcome,
        "basis_head_json": serde_json::to_string(receipt.current_head())
            .expect("a verified build receipt always carries a serializable head"),
        "project_id": receipt.project_id().to_string(),
        "project_revision": receipt.project_revision(),
        "target_path": receipt.target_path(),
        "pack_name": receipt.pack_name(),
        // Preserve the exact caller-observed spelling, as the Voice build route does. Native
        // authoring code used its own canonical protected-root authority for every write.
        "output": output_spelling,
        "files": files,
        "receipt": {
            "format": MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1,
            "relative_name": REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FILE_V1,
            "byte_len": receipt_seal.byte_len(),
            "sha256": encode_hex(receipt_seal.sha256()),
        },
        "build_authority": "reviewed_fixed_leaf_single_package_triplet",
        "artifact_publication_status": artifact_publication_status,
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
        "retry_safe": false,
        "warning": warning,
    });
    // Every variable field above is independently bounded and the file set is fixed at three.
    debug_assert!(
        serde_json::to_vec(&response)
            .expect("a JSON Value is always serializable")
            .len()
            <= MAX_RESPONSE_BYTES
    );
    response
}

fn terminal_metadata(terminal: TerminalKind) -> (&'static str, &'static str, Value) {
    match terminal {
        TerminalKind::Published => ("built", "published", Value::Null),
        TerminalKind::PublishedWithCleanupWarning => (
            "built_with_cleanup_warning",
            "published_with_cleanup_warning",
            json!({
                "code": "AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING",
                "message": "the verified build was published, but private staging cleanup was incomplete"
            }),
        ),
        TerminalKind::PublicationUncertain => (
            "publication_uncertain",
            "publication_uncertain",
            json!({
                "code": "AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN",
                "message": "publication may have completed; do not retry automatically"
            }),
        ),
    }
}

fn map_build_error(error: Revision3ReviewedDataAssetBuildErrorV1) -> Failure {
    match error {
        Revision3ReviewedDataAssetBuildErrorV1::Store(error) => map_store_error(error),
        Revision3ReviewedDataAssetBuildErrorV1::StageSource(error) => map_staging_error(error),
        Revision3ReviewedDataAssetBuildErrorV1::Receipt(_) => invariant(),
        Revision3ReviewedDataAssetBuildErrorV1::ExpectedProjectMismatch { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::CurrentSourceChanged => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_CONFLICT",
            "the exact-current reviewed stage changed during the build",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::OutputAlreadyExists => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_EXISTS",
            "the output already exists; reviewed DataAsset builds never overwrite",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::OutputInspection { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_INVALID",
            "the output could not be inspected safely",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::LiveVerification { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_INVALID",
            "the reviewed stage could not be independently replayed from the installed game",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::PackPreparation { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_INVALID",
            "the protected Store/game/output layout or pack request is invalid",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::PackStaging { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_PACK_FAILED",
            "the reviewed triplet could not be packed, reopened, and receipt-bound exactly",
        ),
        Revision3ReviewedDataAssetBuildErrorV1::Publication { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_FAILED",
            "the verified build could not be published without clobbering",
        ),
    }
}

fn map_staging_error(error: Revision3DataAssetStagingErrorV1) -> Failure {
    match error {
        Revision3DataAssetStagingErrorV1::Store(error) => map_store_error(error),
        Revision3DataAssetStagingErrorV1::Conflict(_) => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_MISSING",
            "no exact reviewed DataAsset stage exists for target_path",
        ),
        Revision3DataAssetStagingErrorV1::Reviewed(_) => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_NOT_REVIEWED",
            "the exact staged DataAsset edit is outside the closed reviewed build profile",
        ),
        Revision3DataAssetStagingErrorV1::Manifest(_)
        | Revision3DataAssetStagingErrorV1::VerifiedInput(_)
        | Revision3DataAssetStagingErrorV1::ProjectBinding(_)
        | Revision3DataAssetStagingErrorV1::CandidateReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID",
            "the reviewed DataAsset stage is not exactly bound to the current project",
        ),
        Revision3DataAssetStagingErrorV1::CurrentSourceReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_CONFLICT",
            "the exact-current reviewed stage changed during the build",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) | WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_LIMIT"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_MISSING",
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_ROOT_MISSING"
        }
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DATAASSET_BUILD_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 working-store operation failed")
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_INVALID",
        "request must be exact canonical JSON with only the closed reviewed build fields",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DATAASSET_BUILD_INVARIANT",
        "the native reviewed DataAsset build failed an internal invariant",
    )
}

fn encode_hex(bytes: &[u8]) -> String {
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
    use gore_authoring::{ContentSeal, Sha256Digest, WorkingStoreFormat};
    use serde_json::json;

    use super::*;

    const TARGET: &str = "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps";

    fn head() -> WorkingHead {
        WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([7; 32]),
            },
        }
    }

    fn payload() -> BuildReviewedDataAssetWirePayload {
        BuildReviewedDataAssetWirePayload {
            current_project_json: "{}".to_owned(),
            expected_head_json: serde_json::to_string(&head()).unwrap(),
            game_root: r"C:\Games\GORE".to_owned(),
            output: r"D:\Mods\WolfReview".to_owned(),
            pack_name: "WolfReview".to_owned(),
            root: r"D:\Projects\Wolf\Store".to_owned(),
            target_path: TARGET.to_owned(),
        }
    }

    fn request(payload: BuildReviewedDataAssetWirePayload) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload,
        })
        .unwrap()
    }

    #[test]
    fn closed_wire_accepts_exact_shape_and_refuses_authority_fields() {
        let canonical = request(payload());
        let parsed: BuildReviewedDataAssetWirePayload = parse_exact_wire(&canonical).unwrap();
        assert_eq!(parsed.target_path, TARGET);
        assert_eq!(parsed.pack_name, "WolfReview");

        let mut value: Value = serde_json::from_str(&canonical).unwrap();
        for forbidden in [
            "selector",
            "replacement_hex",
            "uasset_bytes",
            "usmap_path",
            "receipt_path",
            "deploy",
            "runtime",
            "overwrite",
        ] {
            value["payload"][forbidden] = json!("forged-authority");
            assert!(parse_exact_wire::<BuildReviewedDataAssetWirePayload>(
                &serde_json::to_string(&value).unwrap()
            )
            .is_err());
            value["payload"].as_object_mut().unwrap().remove(forbidden);
        }

        assert!(
            parse_exact_wire::<BuildReviewedDataAssetWirePayload>(&format!(" {canonical}"))
                .is_err()
        );
        let duplicate = canonical.replacen("{", &format!("{{\"command\":\"{COMMAND}\","), 1);
        assert!(parse_exact_wire::<BuildReviewedDataAssetWirePayload>(&duplicate).is_err());
    }

    #[test]
    fn payload_bounds_paths_target_and_pack_before_filesystem_work() {
        validate_payload(&payload()).unwrap();

        for invalid in ["", "_leading", "-leading", "bad/name", "CON"] {
            let mut candidate = payload();
            candidate.pack_name = invalid.to_owned();
            assert_eq!(
                validate_payload(&candidate).unwrap_err().code,
                "AUTHORING_REVISION3_DATAASSET_BUILD_PACK_NAME_INVALID"
            );
        }
        let mut too_long_pack = payload();
        too_long_pack.pack_name = "x".repeat(MAX_PACK_NAME_BYTES + 1);
        assert!(validate_payload(&too_long_pack).is_err());
        let mut maximum_pack = payload();
        maximum_pack.pack_name = "x".repeat(MAX_PACK_NAME_BYTES);
        validate_payload(&maximum_pack).unwrap();

        for invalid in ["Game/Wolf", "/Game/Bad.ext", "/Game/Bad\\Name"] {
            let mut candidate = payload();
            candidate.target_path = invalid.to_owned();
            assert_eq!(
                validate_payload(&candidate).unwrap_err().code,
                "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID"
            );
        }
        let mut maximum_target = payload();
        maximum_target.target_path = format!(
            "/Game/{}",
            "x".repeat(MAX_TARGET_PATH_BYTES - "/Game/".len())
        );
        validate_payload(&maximum_target).unwrap();
        let mut too_long_target = maximum_target;
        too_long_target.target_path.push('x');
        assert_eq!(
            validate_payload(&too_long_target).unwrap_err().code,
            "AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID"
        );

        for project_json in [String::new(), "x".repeat(MAX_PROJECT_JSON_BYTES + 1)] {
            let mut candidate = payload();
            candidate.current_project_json = project_json;
            assert_eq!(
                validate_payload(&candidate).unwrap_err().code,
                "AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID"
            );
        }
        for field in 0..3 {
            let mut candidate = payload();
            match field {
                0 => candidate.root.clear(),
                1 => candidate.game_root = "x".repeat(MAX_PATH_BYTES + 1),
                _ => candidate.output.push('\0'),
            }
            assert_eq!(
                validate_payload(&candidate).unwrap_err().code,
                "AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_INVALID"
            );
        }
    }

    #[test]
    fn canonical_head_and_wire_limits_are_enforced_locally() {
        let canonical = serde_json::to_string(&head()).unwrap();
        assert_eq!(parse_canonical_head(&canonical).unwrap(), head());
        assert!(parse_canonical_head("").is_err());
        assert!(parse_canonical_head(&format!(" {canonical}")).is_err());
        assert!(parse_canonical_head(&"x".repeat(MAX_HEAD_JSON_BYTES + 1)).is_err());
        assert_eq!(
            parse_exact_wire::<BuildReviewedDataAssetWirePayload>(&" ".repeat(MAX_WIRE_BYTES + 1))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_LIMIT"
        );
    }

    #[test]
    fn typed_publication_terminals_distinguish_complete_from_uncertain() {
        for (terminal, expected_outcome, expected_status, expected_warning) in [
            (TerminalKind::Published, "built", "published", false),
            (
                TerminalKind::PublishedWithCleanupWarning,
                "built_with_cleanup_warning",
                "published_with_cleanup_warning",
                true,
            ),
            (
                TerminalKind::PublicationUncertain,
                "publication_uncertain",
                "publication_uncertain",
                true,
            ),
        ] {
            let (outcome, status, warning) = terminal_metadata(terminal);
            assert_eq!(outcome, expected_outcome);
            assert_eq!(status, expected_status);
            assert_eq!(warning.is_object(), expected_warning);
        }

        let (_, _, uncertain_warning) = terminal_metadata(TerminalKind::PublicationUncertain);
        assert_eq!(
            uncertain_warning["code"],
            "AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN"
        );
        assert!(uncertain_warning["message"]
            .as_str()
            .unwrap()
            .contains("do not retry automatically"));
    }

    #[test]
    fn build_errors_are_sanitized_into_phase_specific_codes() {
        assert_eq!(
            map_build_error(Revision3ReviewedDataAssetBuildErrorV1::OutputAlreadyExists).code,
            "AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_EXISTS"
        );
        assert_eq!(
            map_build_error(Revision3ReviewedDataAssetBuildErrorV1::CurrentSourceChanged).code,
            "AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_CONFLICT"
        );
    }

    #[test]
    fn public_dispatch_routes_oversize_wire_to_the_command_local_cap() {
        let request = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"padding\":\"{}\"}}}}",
            "x".repeat(MAX_WIRE_BYTES + 1)
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&request)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_LIMIT"
        );
    }
}
