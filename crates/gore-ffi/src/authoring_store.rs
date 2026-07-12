//! Bounded JSON bridge for the format-2 immutable working-project store.
//!
//! Head and project documents cross the outer JSON protocol as untouched strings. This preserves
//! duplicate-key rejection in `gore-authoring` and gives the Studio the exact canonical head bytes
//! it must use as both the store CAS token and the filesystem replacement precondition.

use std::collections::BTreeSet;
use std::path::Path;

use gore_authoring::{
    AssetRef, AssetVerification, Diagnostic, ProjectJsonError, ProjectV2, ValidationProfile,
    WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES,
};
use serde_json::{json, Map, Value};

use crate::err;

const MAX_FILESYSTEM_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_LOGICAL_NAME_BYTES: usize = 1024;
const MAX_AUTHORING_STORE_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMPORT_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTICS: usize = 262_144;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_RELATED_ENTITIES: usize = 100_000;

#[derive(Debug)]
struct StoreFailure {
    code: &'static str,
    message: String,
}

impl StoreFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: bounded_message(message.into()),
        }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

pub(super) fn open(payload: Value) -> Value {
    command_response(open_inner(&payload))
}

pub(super) fn prepare_checkpoint(payload: Value) -> Value {
    command_response(prepare_checkpoint_inner(&payload))
}

pub(super) fn open_head_bytes(payload: Value) -> Value {
    command_response(open_head_bytes_inner(&payload))
}

pub(super) fn import_ogg(payload: Value) -> Value {
    command_response(import_ogg_inner(&payload))
}

pub(super) fn verify_asset(payload: Value) -> Value {
    command_response(verify_asset_inner(&payload))
}

fn command_response(result: Result<Value, StoreFailure>) -> Value {
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn open_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["profile", "root", "verification"])?;
    let store = open_existing_store(required_path(object, "root")?)?;
    let verification = required_verification(object)?;
    let profile = required_profile(object)?;
    let opened = store
        .open_current(verification, profile)
        .map_err(map_store_error)?;
    opened_response(opened, MAX_AUTHORING_STORE_RESPONSE_BYTES)
}

fn prepare_checkpoint_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(
        payload,
        &["expected_head_json", "profile", "project_json", "root"],
    )?;
    let expected_head = required_expected_head(object)?;
    let store = store_for_expected_head(required_path(object, "root")?, expected_head.as_ref())?;
    let profile = required_profile(object)?;
    let project_json = required_bounded_string(
        object,
        "project_json",
        MAX_PROJECT_JSON_BYTES,
        "authoring project JSON",
    )?;

    // Keep the nested string byte-for-byte intact. A Value round trip here would erase duplicate
    // object keys before ProjectV2's strict deserializer sees them.
    let project = ProjectV2::from_json(project_json).map_err(map_project_error)?;
    let prepared = store
        .prepare_checkpoint(expected_head.as_ref(), &project, profile)
        .map_err(map_store_error)?;
    let head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_INVARIANT",
            "prepared head is not valid UTF-8 JSON",
        )
    })?;
    ensure_head_json(&head_json)?;
    let diagnostics = diagnostics_to_wire(prepared.diagnostics)?;

    let response = json!({
        "ok": true,
        "head_json": head_json,
        "diagnostics": diagnostics,
        "blocks_build": prepared.blocks_build,
    });
    enforce_response_budget(response, MAX_AUTHORING_STORE_RESPONSE_BYTES)
}

fn open_head_bytes_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["head_json", "profile", "root", "verification"])?;
    let store = open_existing_store(required_path(object, "root")?)?;
    let head_json = required_bounded_string(
        object,
        "head_json",
        MAX_HEAD_JSON_BYTES,
        "working-store head JSON",
    )?;
    // The store performs strict canonical parsing and duplicate-field rejection on these exact
    // bytes. Do not decode through serde_json::Value first.
    let opened = store
        .open_head_bytes(
            head_json.as_bytes(),
            required_verification(object)?,
            required_profile(object)?,
        )
        .map_err(map_store_error)?;
    opened_response(opened, MAX_AUTHORING_STORE_RESPONSE_BYTES)
}

fn import_ogg_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(
        payload,
        &["expected_head_json", "logical_name", "root", "source"],
    )?;
    let expected_head = required_expected_head(object)?;
    let store = store_for_expected_head(required_path(object, "root")?, expected_head.as_ref())?;
    let source = required_path(object, "source")?;
    let logical_name = required_bounded_string(
        object,
        "logical_name",
        MAX_LOGICAL_NAME_BYTES,
        "asset logical name",
    )?;
    let imported = store
        .import_ogg(source, logical_name, expected_head.as_ref())
        .map_err(map_store_error)?;
    let response = json!({
        "ok": true,
        "asset": imported.asset,
        "ogg": imported.ogg,
        "deduplicated": imported.deduplicated,
    });
    enforce_response_budget(response, MAX_IMPORT_RESPONSE_BYTES)
}

fn verify_asset_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["asset", "root", "verification"])?;
    let store = open_existing_store(required_path(object, "root")?)?;
    let asset_value = object
        .get("asset")
        .ok_or_else(|| StoreFailure::new("AUTHORING_STORE_ASSET_REQUIRED", "missing 'asset'"))?;
    let asset: AssetRef = serde_json::from_value(asset_value.clone()).map_err(|error| {
        StoreFailure::new(
            "AUTHORING_STORE_ASSET_INVALID",
            format!("invalid asset reference: {error}"),
        )
    })?;
    if asset.logical_name.is_empty() || asset.logical_name.len() > MAX_LOGICAL_NAME_BYTES {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_ASSET_INVALID",
            format!("asset logical name must be 1..={MAX_LOGICAL_NAME_BYTES} bytes"),
        ));
    }
    store
        .verify_asset(&asset, required_verification(object)?)
        .map_err(map_store_error)?;
    Ok(json!({"ok": true}))
}

fn opened_response(
    opened: gore_authoring::OpenedCheckpoint,
    response_limit: usize,
) -> Result<Value, StoreFailure> {
    let head_json = serde_json::to_string(&opened.head).map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_SERIALIZE",
            "working-store head serialization failed",
        )
    })?;
    ensure_head_json(&head_json)?;
    let project_json = opened.project.to_canonical_json().map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_SERIALIZE",
            "working-store project serialization failed",
        )
    })?;
    if project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_LIMIT",
            format!("working-store project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    let diagnostics = diagnostics_to_wire(opened.diagnostics)?;
    let response = json!({
        "ok": true,
        "head_json": head_json,
        "project_json": project_json,
        "diagnostics": diagnostics,
        "blocks_build": opened.blocks_build,
    });
    enforce_response_budget(response, response_limit)
}

fn diagnostics_to_wire(diagnostics: Vec<Diagnostic>) -> Result<Vec<Value>, StoreFailure> {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_LIMIT",
            format!("authoring diagnostic count exceeds the {MAX_DIAGNOSTICS}-item response limit"),
        ));
    }

    let mut wire = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        if diagnostic
            .property_path
            .as_ref()
            .is_some_and(|path| path.len() > MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES)
        {
            return Err(StoreFailure::new(
                "AUTHORING_STORE_RESPONSE_LIMIT",
                format!(
                    "authoring diagnostic property path exceeds the \
                     {MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES}-byte response limit"
                ),
            ));
        }
        if diagnostic.related_entities.len() > MAX_DIAGNOSTIC_RELATED_ENTITIES {
            return Err(StoreFailure::new(
                "AUTHORING_STORE_RESPONSE_LIMIT",
                format!(
                    "authoring diagnostic related-entity count exceeds the \
                     {MAX_DIAGNOSTIC_RELATED_ENTITIES}-item response limit"
                ),
            ));
        }
        let message =
            truncate_utf8_with_suffix(diagnostic.message, MAX_DIAGNOSTIC_MESSAGE_BYTES, "...");
        wire.push(json!({
            "code": diagnostic.code,
            "severity": diagnostic.severity,
            "entity": diagnostic.entity.map(|entity| entity.to_string()),
            "property_path": diagnostic.property_path,
            "message": message,
            "related_entities": diagnostic
                .related_entities
                .into_iter()
                .map(|entity| entity.to_string())
                .collect::<Vec<_>>(),
            "blocks_build": diagnostic.blocks_build,
        }));
    }
    Ok(wire)
}

fn exact_payload<'a>(
    payload: &'a Value,
    expected_fields: &[&str],
) -> Result<&'a Map<String, Value>, StoreFailure> {
    let object = payload.as_object().ok_or_else(|| {
        StoreFailure::new(
            "AUTHORING_STORE_PAYLOAD_INVALID",
            "payload must be an object",
        )
    })?;
    let expected = expected_fields.iter().copied().collect::<BTreeSet<_>>();
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_PAYLOAD_INVALID",
            format!(
                "payload fields must be exactly: {}",
                expected_fields.join(", ")
            ),
        ));
    }
    Ok(object)
}

fn required_path<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str, StoreFailure> {
    required_bounded_string(object, field, MAX_FILESYSTEM_PATH_BYTES, "filesystem path")
}

fn required_bounded_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    max_bytes: usize,
    kind: &str,
) -> Result<&'a str, StoreFailure> {
    let value = object.get(field).and_then(Value::as_str).ok_or_else(|| {
        StoreFailure::new(
            "AUTHORING_STORE_INPUT_INVALID",
            format!("'{field}' must be a {kind} string"),
        )
    })?;
    if value.is_empty() || value.len() > max_bytes {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_INPUT_LIMIT",
            format!("'{field}' must be 1..={max_bytes} UTF-8 bytes"),
        ));
    }
    Ok(value)
}

fn required_profile(object: &Map<String, Value>) -> Result<ValidationProfile, StoreFailure> {
    match object.get("profile").and_then(Value::as_str) {
        Some("production") => Ok(ValidationProfile::Production),
        Some("experimental") => Ok(ValidationProfile::Experimental),
        _ => Err(StoreFailure::new(
            "AUTHORING_STORE_PROFILE_INVALID",
            "'profile' must be 'production' or 'experimental'",
        )),
    }
}

fn required_verification(object: &Map<String, Value>) -> Result<AssetVerification, StoreFailure> {
    match object.get("verification").and_then(Value::as_str) {
        Some("structural") => Ok(AssetVerification::Structural),
        Some("full") => Ok(AssetVerification::Full),
        _ => Err(StoreFailure::new(
            "AUTHORING_STORE_VERIFICATION_INVALID",
            "'verification' must be 'structural' or 'full'",
        )),
    }
}

fn required_expected_head(
    object: &Map<String, Value>,
) -> Result<Option<WorkingHead>, StoreFailure> {
    match object.get("expected_head_json") {
        Some(Value::Null) => Ok(None),
        Some(Value::String(json)) => parse_head_json(json).map(Some),
        _ => Err(StoreFailure::new(
            "AUTHORING_STORE_HEAD_INVALID",
            "'expected_head_json' must be a canonical JSON string or null",
        )),
    }
}

fn parse_head_json(head_json: &str) -> Result<WorkingHead, StoreFailure> {
    ensure_head_json(head_json)?;
    let head: WorkingHead = serde_json::from_str(head_json).map_err(|error| {
        StoreFailure::new(
            "AUTHORING_STORE_HEAD_INVALID",
            format!("invalid working-store head JSON: {error}"),
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_HEAD_INVALID",
            "working-store head serialization failed",
        )
    })?;
    if canonical != head_json {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_HEAD_NONCANONICAL",
            "working-store head JSON is not canonical",
        ));
    }
    if head.snapshot.byte_len == 0
        || head.snapshot.byte_len > WorkingStoreLimits::default().max_snapshot_bytes as u64
    {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_HEAD_INVALID",
            "working-store head snapshot byte_len is outside the supported range",
        ));
    }
    Ok(head)
}

fn ensure_head_json(head_json: &str) -> Result<(), StoreFailure> {
    if head_json.is_empty() || head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_HEAD_LIMIT",
            format!("working-store head JSON must be 1..={MAX_HEAD_JSON_BYTES} UTF-8 bytes"),
        ));
    }
    Ok(())
}

fn ffi_store_limits() -> WorkingStoreLimits {
    // The general store supports larger future/lazy readers, but this ABI eagerly reconstructs
    // and returns one project JSON string. Cap aggregate entity shards at the same 16 MiB project
    // wire contract before the store allocates them; the bounded serializer below still rejects a
    // project whose manifest data pushes its final canonical representation over that contract.
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn create_store(root: &str) -> Result<WorkingProjectStore, StoreFailure> {
    WorkingProjectStore::at(Path::new(root), ffi_store_limits()).map_err(map_store_error)
}

fn open_existing_store(root: &str) -> Result<WorkingProjectStore, StoreFailure> {
    WorkingProjectStore::open_existing(Path::new(root), ffi_store_limits()).map_err(map_store_error)
}

/// A null expected head is the one explicit create-store operation. Once a caller owns a head,
/// even prepare/import must be side-effect-free for a missing or mistyped root.
fn store_for_expected_head(
    root: &str,
    expected_head: Option<&WorkingHead>,
) -> Result<WorkingProjectStore, StoreFailure> {
    if expected_head.is_none() {
        create_store(root)
    } else {
        open_existing_store(root)
    }
}

fn map_project_error(error: ProjectJsonError) -> StoreFailure {
    match error {
        ProjectJsonError::InputTooLarge { .. } => StoreFailure::new(
            "AUTHORING_STORE_PROJECT_LIMIT",
            format!("authoring project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ),
        ProjectJsonError::InvalidJson(_) => {
            StoreFailure::new("AUTHORING_STORE_PROJECT_INVALID", error.to_string())
        }
    }
}

fn map_store_error(error: WorkingStoreError) -> StoreFailure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_STORE_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_STORE_HEAD_MISSING",
        WorkingStoreError::MissingObject(path)
            if path.file_name().and_then(|name| name.to_str()) == Some("gore-project.json") =>
        {
            "AUTHORING_STORE_HEAD_MISSING"
        }
        WorkingStoreError::MissingObject(_) => "AUTHORING_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } => "AUTHORING_STORE_JSON_INVALID",
        WorkingStoreError::NonCanonicalJson { .. } => "AUTHORING_STORE_JSON_NONCANONICAL",
        WorkingStoreError::Invariant(_) => "AUTHORING_STORE_INVARIANT",
        WorkingStoreError::InvalidOgg(_) => "AUTHORING_STORE_OGG_INVALID",
        WorkingStoreError::OggMetadataMismatch { .. } => "AUTHORING_STORE_OGG_METADATA_MISMATCH",
        WorkingStoreError::StagingCleanup { .. } => "AUTHORING_STORE_STAGING_CLEANUP",
        WorkingStoreError::Io(_) => "AUTHORING_STORE_IO",
    };
    StoreFailure::new(code, error.to_string())
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, StoreFailure> {
    match serde_json::to_vec(&response) {
        Ok(encoded) if encoded.len() <= limit => Ok(response),
        Ok(_) => Err(StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_LIMIT",
            format!("working-store response exceeds the {limit}-byte limit"),
        )),
        Err(_) => Err(StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_SERIALIZE",
            "working-store response serialization failed",
        )),
    }
}

fn bounded_message(message: String) -> String {
    truncate_utf8_with_suffix(message, MAX_ERROR_MESSAGE_BYTES, "...")
}

fn truncate_utf8_with_suffix(mut value: String, max_bytes: usize, suffix: &str) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    debug_assert!(suffix.len() <= max_bytes);
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
    use std::fs;

    use gore_authoring::{DiagnosticCode, DiagnosticSeverity, EntityId};
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::execute_json;

    fn diagnostic(message: String, property_path: Option<String>) -> Diagnostic {
        Diagnostic {
            code: DiagnosticCode::AssetMediaTypeMismatch,
            severity: DiagnosticSeverity::Error,
            entity: None,
            property_path,
            message,
            related_entities: Vec::new(),
            blocks_build: true,
        }
    }

    fn project_json() -> String {
        json!({
            "format": 2,
            "schema_revision": 1,
            "project_id": "00000000000000000000000000000001",
            "revision": 0,
            "meta": {"name": "Store bridge", "version": "1.0.0", "author": "tests"},
            "target": {"executable": {
                "byte_len": 123,
                "sha256": "4242424242424242424242424242424242424242424242424242424242424242"
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        })
        .to_string()
    }

    fn vorbis_ogg(sample_rate: u32) -> Vec<u8> {
        let mut data = include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg").to_vec();
        let ident = data
            .windows(7)
            .position(|window| window == b"\x01vorbis")
            .expect("fixture has Vorbis identification");
        data[ident + 12..ident + 16].copy_from_slice(&sample_rate.to_le_bytes());

        let mut offset = 0usize;
        while offset < data.len() {
            let segment_count = usize::from(data[offset + 26]);
            let header_len = 27 + segment_count;
            let body_len = data[offset + 27..offset + header_len]
                .iter()
                .map(|value| usize::from(*value))
                .sum::<usize>();
            let page_len = header_len + body_len;
            data[offset + 22..offset + 26].fill(0);
            let crc = ogg_crc(&data[offset..offset + page_len]);
            data[offset + 22..offset + 26].copy_from_slice(&crc.to_le_bytes());
            offset += page_len;
        }
        data
    }

    fn ogg_crc(bytes: &[u8]) -> u32 {
        let mut crc = 0u32;
        for byte in bytes {
            crc ^= u32::from(*byte) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 != 0 {
                    (crc << 1) ^ 0x04c1_1db7
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    #[test]
    fn long_media_type_diagnostic_is_utf8_truncated_to_dart_wire_limit() {
        let media_type = "\u{97f3}\u{58f0}/".repeat(2_000);
        let message = format!(
            "voice asset has media type {media_type:?}; expected canonical media type audio/ogg"
        );
        let wire = diagnostics_to_wire(vec![diagnostic(
            message,
            Some("payload.data.asset.sha256".to_owned()),
        )])
        .unwrap();
        let message = wire[0]["message"].as_str().unwrap();
        assert!(message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES);
        assert!(message.ends_with("..."));
        assert!(std::str::from_utf8(message.as_bytes()).is_ok());
    }

    #[test]
    fn oversized_diagnostic_paths_and_related_sets_fail_response_closed() {
        let too_many = vec![diagnostic(String::new(), None); MAX_DIAGNOSTICS + 1];
        let count_error = diagnostics_to_wire(too_many).unwrap_err();
        assert_eq!(
            count_error.response()["error"]["code"],
            "AUTHORING_STORE_RESPONSE_LIMIT"
        );

        let path_error = diagnostics_to_wire(vec![diagnostic(
            "failure".to_owned(),
            Some("p".repeat(MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES + 1)),
        )])
        .unwrap_err();
        assert_eq!(
            path_error.response()["error"]["code"],
            "AUTHORING_STORE_RESPONSE_LIMIT"
        );

        let id: EntityId = "00000000000000000000000000000001".parse().unwrap();
        let mut related = diagnostic("failure".to_owned(), None);
        related.related_entities = vec![id; MAX_DIAGNOSTIC_RELATED_ENTITIES + 1];
        let related_error = diagnostics_to_wire(vec![related]).unwrap_err();
        assert_eq!(
            related_error.response()["error"]["code"],
            "AUTHORING_STORE_RESPONSE_LIMIT"
        );
    }

    fn call(command: &str, payload: Value) -> Value {
        serde_json::from_str(&execute_json(
            &json!({"command": command, "payload": payload}).to_string(),
        ))
        .unwrap()
    }

    fn prepare(temp: &TempDir, project_json: String, expected_head_json: Value) -> Value {
        call(
            "authoring_store_prepare_checkpoint",
            json!({
                "root": temp.path(),
                "expected_head_json": expected_head_json,
                "project_json": project_json,
                "profile": "production",
            }),
        )
    }

    #[test]
    fn prepare_returns_exact_head_and_open_round_trips_canonical_project() {
        let temp = TempDir::new().unwrap();
        let prepared = prepare(&temp, project_json(), Value::Null);
        assert_eq!(prepared["ok"], true);
        let head_json = prepared["head_json"].as_str().unwrap();
        assert_eq!(
            serde_json::to_string(&serde_json::from_str::<WorkingHead>(head_json).unwrap())
                .unwrap(),
            head_json
        );

        fs::write(temp.path().join("gore-project.json"), head_json).unwrap();
        let stale_absent = prepare(&temp, project_json(), Value::Null);
        assert_eq!(
            stale_absent["error"]["code"],
            "AUTHORING_STORE_HEAD_CONFLICT"
        );
        let matching_head = prepare(&temp, project_json(), json!(head_json));
        assert_eq!(matching_head["ok"], true);

        let opened = call(
            "authoring_store_open",
            json!({
                "root": temp.path(),
                "verification": "full",
                "profile": "production",
            }),
        );
        assert_eq!(opened["ok"], true);
        assert_eq!(opened["head_json"], prepared["head_json"]);
        assert_eq!(
            opened["project_json"],
            ProjectV2::from_json(&project_json())
                .unwrap()
                .to_canonical_json()
                .unwrap()
        );
        assert_eq!(opened["diagnostics"], json!([]));
        assert_eq!(opened["blocks_build"], false);

        let reopened = call(
            "authoring_store_open_head_bytes",
            json!({
                "root": temp.path(),
                "head_json": head_json,
                "verification": "structural",
                "profile": "experimental",
            }),
        );
        assert_eq!(reopened["ok"], true);
        assert_eq!(reopened["head_json"], head_json);
    }

    #[test]
    fn raw_duplicate_project_keys_and_noncanonical_heads_fail_closed() {
        let temp = TempDir::new().unwrap();
        let duplicate =
            project_json().replacen("\"revision\":0", "\"revision\":0,\"revision\":1", 1);
        let rejected = prepare(&temp, duplicate, Value::Null);
        assert_eq!(rejected["error"]["code"], "AUTHORING_STORE_PROJECT_INVALID");

        let prepared = prepare(&temp, project_json(), Value::Null);
        let noncanonical = format!(" {}", prepared["head_json"].as_str().unwrap());
        let rejected = call(
            "authoring_store_open_head_bytes",
            json!({
                "root": temp.path(),
                "head_json": noncanonical,
                "verification": "full",
                "profile": "production",
            }),
        );
        assert_eq!(
            rejected["error"]["code"],
            "AUTHORING_STORE_JSON_NONCANONICAL"
        );

        let rejected = call(
            "authoring_store_open_head_bytes",
            json!({
                "root": temp.path(),
                "head_json": "{ }",
                "verification": "full",
                "profile": "production",
            }),
        );
        assert_eq!(rejected["error"]["code"], "AUTHORING_STORE_JSON_INVALID");
    }

    #[test]
    fn payloads_profiles_paths_and_expected_heads_are_closed_and_bounded() {
        let temp = TempDir::new().unwrap();
        let missing = call(
            "authoring_store_open",
            json!({"root": temp.path(), "verification": "full", "profile": "production"}),
        );
        assert_eq!(missing["error"]["code"], "AUTHORING_STORE_HEAD_MISSING");

        let cases = [
            call("authoring_store_open", Value::Null),
            call(
                "authoring_store_open",
                json!({"root": temp.path(), "verification": "quick", "profile": "production"}),
            ),
            call(
                "authoring_store_open",
                json!({"root": "x".repeat(MAX_FILESYSTEM_PATH_BYTES + 1), "verification": "full", "profile": "production"}),
            ),
            prepare(&temp, project_json(), json!({"store_format": 1})),
            call(
                "authoring_store_verify_asset",
                json!({"root": temp.path(), "verification": "full", "asset": {}, "extra": true}),
            ),
        ];
        let codes = cases
            .iter()
            .map(|value| value["error"]["code"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            [
                "AUTHORING_STORE_PAYLOAD_INVALID",
                "AUTHORING_STORE_VERIFICATION_INVALID",
                "AUTHORING_STORE_INPUT_LIMIT",
                "AUTHORING_STORE_HEAD_INVALID",
                "AUTHORING_STORE_PAYLOAD_INVALID",
            ]
        );
    }

    #[test]
    fn read_and_existing_head_commands_never_create_a_missing_root() {
        let parent = TempDir::new().unwrap();
        let missing = parent.path().join("missing").join("project");
        let canonical_missing_head = concat!(
            "{\"store_format\":1,\"snapshot\":{\"byte_len\":1,\"sha256\":\"",
            "1111111111111111111111111111111111111111111111111111111111111111",
            "\"}}",
        );

        let opened = call(
            "authoring_store_open",
            json!({
                "root": missing.display().to_string(),
                "verification": "full",
                "profile": "production",
            }),
        );
        assert_eq!(opened["error"]["code"], "AUTHORING_STORE_ROOT_MISSING");
        assert!(!missing.exists());

        let stale = call(
            "authoring_store_prepare_checkpoint",
            json!({
                "root": missing.display().to_string(),
                "expected_head_json": canonical_missing_head,
                "project_json": project_json(),
                "profile": "production",
            }),
        );
        assert_eq!(stale["error"]["code"], "AUTHORING_STORE_ROOT_MISSING");
        assert!(!missing.exists());
    }

    #[test]
    fn asset_commands_return_stable_fail_closed_errors() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("voice.ogg");
        fs::write(&source, vorbis_ogg(48_000)).unwrap();
        let imported = call(
            "authoring_store_import_ogg",
            json!({
                "root": temp.path(),
                "source": source,
                "logical_name": "voice/test.ogg",
                "expected_head_json": null,
            }),
        );
        assert_eq!(imported["ok"], true);
        assert_eq!(imported["ogg"]["codec"], "vorbis");
        assert_eq!(imported["ogg"]["sample_rate"], 48_000);
        assert_eq!(imported["deduplicated"], false);

        let verified = call(
            "authoring_store_verify_asset",
            json!({
                "root": temp.path(),
                "verification": "full",
                "asset": imported["asset"].clone(),
            }),
        );
        assert_eq!(verified, json!({"ok": true}));

        let imported_again = call(
            "authoring_store_import_ogg",
            json!({
                "root": temp.path(),
                "source": source,
                "logical_name": "voice/test.ogg",
                "expected_head_json": null,
            }),
        );
        assert_eq!(imported_again["ok"], true);
        assert_eq!(imported_again["deduplicated"], true);

        let source = temp.path().join("not-ogg.bin");
        fs::write(&source, b"not ogg").unwrap();
        let imported = call(
            "authoring_store_import_ogg",
            json!({
                "root": temp.path(),
                "source": source,
                "logical_name": "voice/test.ogg",
                "expected_head_json": null,
            }),
        );
        assert_eq!(imported["error"]["code"], "AUTHORING_STORE_OGG_INVALID");

        let verified = call(
            "authoring_store_verify_asset",
            json!({
                "root": temp.path(),
                "verification": "full",
                "asset": {
                    "sha256": "1111111111111111111111111111111111111111111111111111111111111111",
                    "byte_len": 1,
                    "logical_name": "missing.ogg"
                },
            }),
        );
        assert_eq!(verified["error"]["code"], "AUTHORING_STORE_OBJECT_MISSING");
    }
}
