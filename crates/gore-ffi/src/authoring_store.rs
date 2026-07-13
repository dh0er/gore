//! Bounded JSON bridge for the format-2 immutable working-project store.
//!
//! Head and project documents cross the outer JSON protocol as untouched strings. This preserves
//! duplicate-key rejection in `gore-authoring` and gives the Studio the exact canonical head bytes
//! it must use as both the store CAS token and the filesystem replacement precondition.

use std::collections::BTreeSet;
use std::path::Path;

use gore_authoring::{
    AssetRef, AssetVerification, Diagnostic, OpenedDocumentCheckpoint, ProjectDocument,
    ProjectDocumentError, ProjectJsonError, ProjectRevision3, ProjectRevision3JsonError, ProjectV2,
    ValidationProfile, WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
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
// The nested head/project strings are already canonical JSON. Embedding those exact bytes in the
// outer wire can at most double their quotes/backslashes; a filesystem path can use six-byte JSON
// escapes for each source byte. These route-local raw-decode limits stay below the global 64 MiB
// native transport cap.
const MAX_REVISION3_OPEN_WIRE_BYTES: usize = MAX_FILESYSTEM_PATH_BYTES * 6 + 4 * 1024;
const MAX_REVISION3_HEAD_WIRE_BYTES: usize =
    MAX_FILESYSTEM_PATH_BYTES * 6 + MAX_HEAD_JSON_BYTES * 2 + 4 * 1024;
const MAX_REVISION3_PREPARE_WIRE_BYTES: usize =
    MAX_FILESYSTEM_PATH_BYTES * 6 + MAX_HEAD_JSON_BYTES * 2 + MAX_PROJECT_JSON_BYTES * 2 + 4 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactRevision3WireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenRevision3WirePayload {
    root: String,
    verification: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenRevision3HeadWirePayload {
    head_json: String,
    root: String,
    verification: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareRevision3WirePayload {
    expected_head_json: ExpectedHeadJsonWire,
    project_json: String,
    root: String,
}

#[derive(Debug)]
enum ExpectedHeadJsonWire {
    Absent,
    Present(String),
}

impl<'de> Deserialize<'de> for ExpectedHeadJsonWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<String>::deserialize(deserializer)? {
            Some(head) => Self::Present(head),
            None => Self::Absent,
        })
    }
}

impl ExpectedHeadJsonWire {
    fn into_value(self) -> Value {
        match self {
            Self::Absent => Value::Null,
            Self::Present(head) => Value::String(head),
        }
    }
}

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

pub(super) fn open_document(payload: Value) -> Value {
    command_response(open_document_inner(&payload))
}

pub(super) fn open_revision3_raw(input: &str) -> Value {
    command_response((|| {
        let payload: OpenRevision3WirePayload = parse_exact_revision3_wire(
            input,
            "authoring_store_open_revision3",
            MAX_REVISION3_OPEN_WIRE_BYTES,
        )?;
        open_revision3_inner(&Value::Object(Map::from_iter([
            ("root".to_owned(), Value::String(payload.root)),
            (
                "verification".to_owned(),
                Value::String(payload.verification),
            ),
        ])))
    })())
}

pub(super) fn prepare_checkpoint(payload: Value) -> Value {
    command_response(prepare_checkpoint_inner(&payload))
}

pub(super) fn prepare_document_checkpoint(payload: Value) -> Value {
    command_response(prepare_document_checkpoint_inner(&payload))
}

pub(super) fn prepare_revision3_checkpoint_raw(input: &str) -> Value {
    command_response((|| {
        let payload: PrepareRevision3WirePayload = parse_exact_revision3_wire(
            input,
            "authoring_store_prepare_revision3_checkpoint",
            MAX_REVISION3_PREPARE_WIRE_BYTES,
        )?;
        prepare_revision3_checkpoint_inner(&Value::Object(Map::from_iter([
            (
                "expected_head_json".to_owned(),
                payload.expected_head_json.into_value(),
            ),
            (
                "project_json".to_owned(),
                Value::String(payload.project_json),
            ),
            ("root".to_owned(), Value::String(payload.root)),
        ])))
    })())
}

pub(super) fn open_head_bytes(payload: Value) -> Value {
    command_response(open_head_bytes_inner(&payload))
}

pub(super) fn open_head_bytes_document(payload: Value) -> Value {
    command_response(open_head_bytes_document_inner(&payload))
}

pub(super) fn open_revision3_head_bytes_raw(input: &str) -> Value {
    command_response((|| {
        let payload: OpenRevision3HeadWirePayload = parse_exact_revision3_wire(
            input,
            "authoring_store_open_revision3_head_bytes",
            MAX_REVISION3_HEAD_WIRE_BYTES,
        )?;
        open_revision3_head_bytes_inner(&Value::Object(Map::from_iter([
            ("head_json".to_owned(), Value::String(payload.head_json)),
            ("root".to_owned(), Value::String(payload.root)),
            (
                "verification".to_owned(),
                Value::String(payload.verification),
            ),
        ])))
    })())
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

fn parse_exact_revision3_wire<P: DeserializeOwned>(
    input: &str,
    expected_command: &'static str,
    max_bytes: usize,
) -> Result<P, StoreFailure> {
    if input.len() > max_bytes {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_INPUT_LIMIT",
            format!("revision-3 working-store request exceeds the {max_bytes}-byte limit"),
        ));
    }
    let request: ExactRevision3WireRequest<P> = serde_json::from_str(input).map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_PAYLOAD_INVALID",
            "revision-3 working-store request must be one exact duplicate-free object",
        )
    })?;
    if request.command != expected_command {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_PAYLOAD_INVALID",
            "revision-3 working-store request command does not match its wire schema",
        ));
    }
    Ok(request.payload)
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

fn open_document_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["profile", "root", "verification"])?;
    let store = open_existing_store(required_path(object, "root")?)?;
    let verification = required_verification(object)?;
    let profile = required_profile(object)?;
    let opened = store
        .open_current_document(verification, profile)
        .map_err(map_store_error)?;
    opened_document_response(opened, MAX_AUTHORING_STORE_RESPONSE_BYTES)
}

fn open_revision3_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["root", "verification"])?;
    let store = open_existing_store(required_revision3_root(object)?)?;
    let opened = store
        .open_current_revision3(required_verification(object)?)
        .map_err(map_store_error)?;
    opened_revision3_response(opened, MAX_AUTHORING_STORE_RESPONSE_BYTES)
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

fn prepare_document_checkpoint_inner(payload: &Value) -> Result<Value, StoreFailure> {
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

    // Keep the nested string byte-for-byte intact so the closed document dispatcher can reject
    // duplicate keys before selecting its revision-specific parser.
    let document = ProjectDocument::from_json(project_json).map_err(map_project_document_error)?;
    let prepared = store
        .prepare_document_checkpoint(expected_head.as_ref(), &document, profile)
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

fn prepare_revision3_checkpoint_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["expected_head_json", "project_json", "root"])?;
    let expected_head = required_expected_head(object)?;
    // This dedicated path never creates a store root. A null CAS token means that the fixed head
    // must be absent in an already-existing, explicitly selected store.
    let store = open_existing_store(required_revision3_root(object)?)?;
    let project_json = required_bounded_string(
        object,
        "project_json",
        MAX_PROJECT_JSON_BYTES,
        "canonical schema-revision-3 project JSON",
    )?;

    // Preserve the nested bytes: revision-3 parsing rejects duplicates and every noncanonical
    // spelling before any immutable object can be installed.
    let project = ProjectRevision3::from_json(project_json).map_err(map_revision3_project_error)?;
    let prepared = store
        .prepare_revision3_checkpoint(expected_head.as_ref(), &project)
        .map_err(map_store_error)?;

    // Reopen the exact returned candidate at full verification. This proves that the response is
    // bound to durable immutable objects; it does not publish or replace the fixed head.
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != project {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_INVARIANT",
            "prepared revision-3 checkpoint did not reopen exactly",
        ));
    }
    prepared_revision3_response(&reopened.head, MAX_AUTHORING_STORE_RESPONSE_BYTES)
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

fn open_head_bytes_document_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["head_json", "profile", "root", "verification"])?;
    let store = open_existing_store(required_path(object, "root")?)?;
    let head_json = required_bounded_string(
        object,
        "head_json",
        MAX_HEAD_JSON_BYTES,
        "working-store head JSON",
    )?;
    let opened = store
        .open_head_bytes_document(
            head_json.as_bytes(),
            required_verification(object)?,
            required_profile(object)?,
        )
        .map_err(map_store_error)?;
    opened_document_response(opened, MAX_AUTHORING_STORE_RESPONSE_BYTES)
}

fn open_revision3_head_bytes_inner(payload: &Value) -> Result<Value, StoreFailure> {
    let object = exact_payload(payload, &["head_json", "root", "verification"])?;
    let store = open_existing_store(required_revision3_root(object)?)?;
    let head_json = required_bounded_string(
        object,
        "head_json",
        MAX_HEAD_JSON_BYTES,
        "canonical working-store head JSON",
    )?;
    // The Store sees these exact bytes and therefore retains duplicate-field and canonical-byte
    // rejection instead of accepting a lossy serde_json::Value round trip.
    let opened = store
        .open_revision3_head_bytes(head_json.as_bytes(), required_verification(object)?)
        .map_err(map_store_error)?;
    opened_revision3_response(opened, MAX_AUTHORING_STORE_RESPONSE_BYTES)
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

fn opened_document_response(
    opened: OpenedDocumentCheckpoint,
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

fn opened_revision3_response(
    opened: gore_authoring::OpenedRevision3Checkpoint,
    response_limit: usize,
) -> Result<Value, StoreFailure> {
    let head_json = serde_json::to_string(&opened.head).map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_SERIALIZE",
            "revision-3 working-store head serialization failed",
        )
    })?;
    ensure_head_json(&head_json)?;
    let project_json = opened
        .project
        .to_canonical_json()
        .map_err(map_revision3_response_error)?;
    let response = json!({
        "ok": true,
        "head_json": head_json,
        "project_json": project_json,
    });
    enforce_response_budget(response, response_limit)
}

fn prepared_revision3_response(
    head: &WorkingHead,
    response_limit: usize,
) -> Result<Value, StoreFailure> {
    let head_json = serde_json::to_string(head).map_err(|_| {
        StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_SERIALIZE",
            "revision-3 working-store head serialization failed",
        )
    })?;
    ensure_head_json(&head_json)?;
    enforce_response_budget(json!({"ok": true, "head_json": head_json}), response_limit)
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

fn required_revision3_root(object: &Map<String, Value>) -> Result<&str, StoreFailure> {
    let root = required_path(object, "root")?;
    if root.contains('\0') {
        return Err(StoreFailure::new(
            "AUTHORING_STORE_INPUT_INVALID",
            "'root' must not contain NUL bytes",
        ));
    }
    Ok(root)
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

fn map_project_document_error(error: ProjectDocumentError) -> StoreFailure {
    match error {
        ProjectDocumentError::InputTooLarge { .. } => StoreFailure::new(
            "AUTHORING_STORE_PROJECT_LIMIT",
            format!("authoring project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ),
        error => StoreFailure::new("AUTHORING_STORE_PROJECT_INVALID", error.to_string()),
    }
}

fn map_revision3_project_error(error: ProjectRevision3JsonError) -> StoreFailure {
    match error {
        ProjectRevision3JsonError::InputTooLarge { .. } => StoreFailure::new(
            "AUTHORING_STORE_PROJECT_LIMIT",
            format!("authoring project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ),
        ProjectRevision3JsonError::NonCanonicalJson => {
            StoreFailure::new("AUTHORING_STORE_PROJECT_NONCANONICAL", error.to_string())
        }
        _ => StoreFailure::new("AUTHORING_STORE_PROJECT_INVALID", error.to_string()),
    }
}

fn map_revision3_response_error(error: ProjectRevision3JsonError) -> StoreFailure {
    match error {
        ProjectRevision3JsonError::InputTooLarge { .. } => StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_LIMIT",
            format!("working-store project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ),
        _ => StoreFailure::new(
            "AUTHORING_STORE_RESPONSE_SERIALIZE",
            "revision-3 working-store project serialization failed",
        ),
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

    use gore_authoring::{DiagnosticCode, DiagnosticSeverity, EntityId, Sha256Digest};
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

    fn revision2_project_json() -> String {
        json!({
            "format": 2,
            "schema_revision": 2,
            "project_id": "00000000000000000000000000000002",
            "revision": 0,
            "meta": {"name": "Store document bridge", "version": "1.0.0", "author": "tests"},
            "target": {"executable": {
                "byte_len": 123,
                "sha256": "4343434343434343434343434343434343434343434343434343434343434343"
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        })
        .to_string()
    }

    fn revision3_project_json() -> String {
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "03030303030303030303030303030303",
            "revision": 7,
            "meta": {"name": "Store revision 3 bridge", "version": "1.0.0", "author": "tests"},
            "target": {"executable": {
                "byte_len": 123,
                "sha256": "4545454545454545454545454545454545454545454545454545454545454545"
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap();
        project.to_canonical_json().unwrap()
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
        serde_json::from_str(&call_json(command, payload)).unwrap()
    }

    fn call_json(command: &str, payload: Value) -> String {
        execute_json(&json!({"command": command, "payload": payload}).to_string())
    }

    fn call_raw_json(request: &str) -> Value {
        serde_json::from_str(&execute_json(request)).unwrap()
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

    fn prepare_document(
        temp: &TempDir,
        project_json: String,
        expected_head_json: Value,
        profile: &str,
    ) -> Value {
        call(
            "authoring_store_prepare_document_checkpoint",
            json!({
                "root": temp.path(),
                "expected_head_json": expected_head_json,
                "project_json": project_json,
                "profile": profile,
            }),
        )
    }

    fn prepare_revision3(temp: &TempDir, project_json: String, expected_head_json: Value) -> Value {
        call(
            "authoring_store_prepare_revision3_checkpoint",
            json!({
                "root": temp.path(),
                "expected_head_json": expected_head_json,
                "project_json": project_json,
            }),
        )
    }

    #[test]
    fn document_commands_preserve_revision1_responses_exactly() {
        let temp = TempDir::new().unwrap();
        let legacy_prepared = prepare(&temp, project_json(), Value::Null);
        let document_prepared = prepare_document(&temp, project_json(), Value::Null, "production");
        assert_eq!(document_prepared, legacy_prepared);
        let payload = json!({
            "root": temp.path(),
            "expected_head_json": null,
            "project_json": project_json(),
            "profile": "production",
        });
        assert_eq!(
            call_json(
                "authoring_store_prepare_document_checkpoint",
                payload.clone(),
            ),
            call_json("authoring_store_prepare_checkpoint", payload),
        );

        let head_json = legacy_prepared["head_json"].as_str().unwrap();
        fs::write(temp.path().join("gore-project.json"), head_json).unwrap();
        let payload = json!({
            "root": temp.path(),
            "verification": "full",
            "profile": "production",
        });
        assert_eq!(
            call_json("authoring_store_open_document", payload.clone()),
            call_json("authoring_store_open", payload),
        );

        let payload = json!({
            "root": temp.path(),
            "head_json": head_json,
            "verification": "structural",
            "profile": "experimental",
        });
        assert_eq!(
            call_json("authoring_store_open_head_bytes_document", payload.clone(),),
            call_json("authoring_store_open_head_bytes", payload),
        );
    }

    #[test]
    fn document_commands_round_trip_revision2_but_legacy_commands_stay_closed() {
        let temp = TempDir::new().unwrap();
        let raw_project = revision2_project_json();
        let canonical_project = ProjectDocument::from_json(&raw_project)
            .unwrap()
            .to_canonical_json()
            .unwrap();

        let legacy_rejected = prepare(&temp, raw_project.clone(), Value::Null);
        assert_eq!(
            legacy_rejected["error"]["code"],
            "AUTHORING_STORE_PROJECT_INVALID"
        );

        let prepared = prepare_document(&temp, raw_project, Value::Null, "production");
        assert_eq!(prepared["ok"], true);
        assert_eq!(prepared["blocks_build"], true);
        assert_eq!(prepared["diagnostics"].as_array().unwrap().len(), 1);
        assert_eq!(
            prepared["diagnostics"][0]["code"],
            "REVISION2_COMBINED_VALIDATION_UNAVAILABLE"
        );
        assert_eq!(prepared["diagnostics"][0]["blocks_build"], true);

        let head_json = prepared["head_json"].as_str().unwrap();
        fs::write(temp.path().join("gore-project.json"), head_json).unwrap();

        let legacy_open = call(
            "authoring_store_open",
            json!({
                "root": temp.path(),
                "verification": "full",
                "profile": "production",
            }),
        );
        assert_eq!(legacy_open["error"]["code"], "AUTHORING_STORE_JSON_INVALID");

        for profile in ["production", "experimental"] {
            let opened = call(
                "authoring_store_open_document",
                json!({
                    "root": temp.path(),
                    "verification": "full",
                    "profile": profile,
                }),
            );
            assert_eq!(opened["ok"], true);
            assert_eq!(opened["head_json"], head_json);
            assert_eq!(opened["project_json"], canonical_project);
            assert_eq!(opened["blocks_build"], true);
            assert!(opened["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"]
                    == "REVISION2_COMBINED_VALIDATION_UNAVAILABLE"
                    && diagnostic["blocks_build"] == true));
        }

        let reopened = call(
            "authoring_store_open_head_bytes_document",
            json!({
                "root": temp.path(),
                "head_json": head_json,
                "verification": "full",
                "profile": "experimental",
            }),
        );
        assert_eq!(reopened["ok"], true);
        assert_eq!(reopened["project_json"], canonical_project);
    }

    #[test]
    fn document_prepare_rejects_duplicates_and_unknown_dispatch_markers() {
        for invalid in [
            revision2_project_json().replacen("\"revision\":0", "\"revision\":0,\"revision\":1", 1),
            revision2_project_json().replacen("\"format\":2", "\"format\":3", 1),
        ] {
            let temp = TempDir::new().unwrap();
            let rejected = prepare_document(&temp, invalid, Value::Null, "production");
            assert_eq!(rejected["error"]["code"], "AUTHORING_STORE_PROJECT_INVALID");
            assert!(!temp.path().join("gore-project.json").exists());
        }

        let temp = TempDir::new().unwrap();
        let revision3 =
            prepare_document(&temp, revision3_project_json(), Value::Null, "production");
        assert_eq!(revision3["error"]["code"], "AUTHORING_STORE_INVARIANT");
        assert!(!temp.path().join("gore-project.json").exists());
    }

    #[test]
    fn revision3_prepare_is_deterministic_reopens_and_never_publishes_fixed_head() {
        let temp = TempDir::new().unwrap();
        let canonical_project = revision3_project_json();

        let first = prepare_revision3(&temp, canonical_project.clone(), Value::Null);
        assert_eq!(
            first.as_object().unwrap().keys().collect::<Vec<_>>(),
            vec!["head_json", "ok"]
        );
        assert_eq!(first["ok"], true);
        assert!(!temp.path().join("gore-project.json").exists());
        let second = prepare_revision3(&temp, canonical_project.clone(), Value::Null);
        assert_eq!(second, first);
        assert!(!temp.path().join("gore-project.json").exists());

        let head_json = first["head_json"].as_str().unwrap();
        for verification in ["structural", "full"] {
            let opened = call(
                "authoring_store_open_revision3_head_bytes",
                json!({
                    "root": temp.path(),
                    "head_json": head_json,
                    "verification": verification,
                }),
            );
            assert_eq!(
                opened.as_object().unwrap().keys().collect::<Vec<_>>(),
                vec!["head_json", "ok", "project_json"]
            );
            assert_eq!(opened["ok"], true);
            assert_eq!(opened["head_json"], head_json);
            assert_eq!(opened["project_json"], canonical_project);
        }

        fs::write(temp.path().join("gore-project.json"), head_json).unwrap();
        let fixed_before = fs::read(temp.path().join("gore-project.json")).unwrap();
        let matching = prepare_revision3(&temp, canonical_project.clone(), json!(head_json));
        assert_eq!(matching, first);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_before
        );

        for verification in ["structural", "full"] {
            let opened = call(
                "authoring_store_open_revision3",
                json!({"root": temp.path(), "verification": verification}),
            );
            assert_eq!(opened["ok"], true);
            assert_eq!(opened["head_json"], head_json);
            assert_eq!(opened["project_json"], canonical_project);
        }
    }

    #[test]
    fn revision3_prepare_enforces_absent_and_present_head_cas_exactly() {
        let temp = TempDir::new().unwrap();
        let project = revision3_project_json();
        let prepared = prepare_revision3(&temp, project.clone(), Value::Null);
        let head_json = prepared["head_json"].as_str().unwrap();
        fs::write(temp.path().join("gore-project.json"), head_json).unwrap();
        let fixed_before = fs::read(temp.path().join("gore-project.json")).unwrap();

        let absent_conflict = prepare_revision3(&temp, project.clone(), Value::Null);
        assert_eq!(
            absent_conflict["error"]["code"],
            "AUTHORING_STORE_HEAD_CONFLICT"
        );

        let mut stale: WorkingHead = serde_json::from_str(head_json).unwrap();
        stale.snapshot.sha256 = Sha256Digest::from_bytes([0x11; 32]);
        let stale = serde_json::to_string(&stale).unwrap();
        let present_conflict = prepare_revision3(&temp, project, json!(stale));
        assert_eq!(
            present_conflict["error"]["code"],
            "AUTHORING_STORE_HEAD_CONFLICT"
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_before
        );

        let absent = TempDir::new().unwrap();
        let expected_present =
            prepare_revision3(&absent, revision3_project_json(), json!(head_json));
        assert_eq!(
            expected_present["error"]["code"],
            "AUTHORING_STORE_HEAD_CONFLICT"
        );
        assert!(!absent.path().join("gore-project.json").exists());
    }

    #[test]
    fn revision3_commands_reject_other_revisions_and_noncanonical_nested_bytes() {
        for invalid in [project_json(), revision2_project_json()] {
            let temp = TempDir::new().unwrap();
            let rejected = prepare_revision3(&temp, invalid, Value::Null);
            assert_eq!(rejected["error"]["code"], "AUTHORING_STORE_PROJECT_INVALID");
            assert!(!temp.path().join("gore-project.json").exists());
        }

        let canonical = revision3_project_json();
        let cases = [
            (
                format!(" {canonical}"),
                "AUTHORING_STORE_PROJECT_NONCANONICAL",
            ),
            (
                canonical.replacen("\"revision\":7", "\"revision\":7,\"revision\":8", 1),
                "AUTHORING_STORE_PROJECT_INVALID",
            ),
            (format!("{canonical}\0"), "AUTHORING_STORE_PROJECT_INVALID"),
        ];
        for (invalid, code) in cases {
            let temp = TempDir::new().unwrap();
            let rejected = prepare_revision3(&temp, invalid, Value::Null);
            assert_eq!(rejected["error"]["code"], code);
        }

        let r1 = TempDir::new().unwrap();
        let legacy = prepare(&r1, project_json(), Value::Null);
        fs::write(
            r1.path().join("gore-project.json"),
            legacy["head_json"].as_str().unwrap(),
        )
        .unwrap();
        let rejected = call(
            "authoring_store_open_revision3_head_bytes",
            json!({
                "root": r1.path(),
                "head_json": legacy["head_json"],
                "verification": "full",
            }),
        );
        assert_eq!(rejected["ok"], false);
        let rejected = call(
            "authoring_store_open_revision3",
            json!({"root": r1.path(), "verification": "full"}),
        );
        assert_eq!(rejected["ok"], false);

        let r2 = TempDir::new().unwrap();
        let document = prepare_document(&r2, revision2_project_json(), Value::Null, "experimental");
        fs::write(
            r2.path().join("gore-project.json"),
            document["head_json"].as_str().unwrap(),
        )
        .unwrap();
        let rejected = call(
            "authoring_store_open_revision3_head_bytes",
            json!({
                "root": r2.path(),
                "head_json": document["head_json"],
                "verification": "structural",
            }),
        );
        assert_eq!(rejected["ok"], false);
        let rejected = call(
            "authoring_store_open_revision3",
            json!({"root": r2.path(), "verification": "structural"}),
        );
        assert_eq!(rejected["ok"], false);
    }

    #[test]
    fn revision3_head_bytes_remain_duplicate_safe_and_canonical() {
        let temp = TempDir::new().unwrap();
        let prepared = prepare_revision3(&temp, revision3_project_json(), Value::Null);
        let head = prepared["head_json"].as_str().unwrap();
        let duplicate = head.replacen(
            "\"store_format\":1",
            "\"store_format\":1,\"store_format\":1",
            1,
        );

        for (invalid, expected_code) in [
            (format!(" {head}"), "AUTHORING_STORE_JSON_NONCANONICAL"),
            (duplicate, "AUTHORING_STORE_JSON_INVALID"),
            (
                head.replacen("{", "{\"unknown\":true,", 1),
                "AUTHORING_STORE_JSON_INVALID",
            ),
            (format!("{head}\0"), "AUTHORING_STORE_JSON_INVALID"),
        ] {
            let rejected = call(
                "authoring_store_open_revision3_head_bytes",
                json!({
                    "root": temp.path(),
                    "head_json": invalid,
                    "verification": "full",
                }),
            );
            assert_eq!(rejected["error"]["code"], expected_code);
        }
    }

    #[test]
    fn revision3_payload_paths_verification_and_limits_fail_closed() {
        let temp = TempDir::new().unwrap();
        let canonical = revision3_project_json();
        let oversized_project = "x".repeat(MAX_PROJECT_JSON_BYTES + 1);
        let oversized_head = "x".repeat(MAX_HEAD_JSON_BYTES + 1);
        let unsafe_file = temp.path().join("not-a-directory");
        fs::write(&unsafe_file, b"file").unwrap();
        let missing = temp.path().join("missing").join("store");
        let missing_prepare = temp.path().join("missing-prepare").join("store");

        let cases = [
            call("authoring_store_open_revision3", Value::Null),
            call(
                "authoring_store_open_revision3",
                json!({"root": temp.path(), "verification": "full", "extra": true}),
            ),
            call(
                "authoring_store_open_revision3_head_bytes",
                json!({"root": temp.path(), "head_json": "{}", "verification": "full", "extra": true}),
            ),
            call(
                "authoring_store_prepare_revision3_checkpoint",
                json!({
                    "root": temp.path(),
                    "expected_head_json": null,
                    "project_json": canonical.clone(),
                    "extra": true,
                }),
            ),
            call(
                "authoring_store_open_revision3",
                json!({"root": temp.path(), "verification": "quick"}),
            ),
            call(
                "authoring_store_open_revision3",
                json!({"root": "bad\0root", "verification": "full"}),
            ),
            call(
                "authoring_store_open_revision3",
                json!({"root": "x".repeat(MAX_FILESYSTEM_PATH_BYTES + 1), "verification": "full"}),
            ),
            prepare_revision3(&temp, oversized_project, Value::Null),
            call(
                "authoring_store_open_revision3_head_bytes",
                json!({"root": temp.path(), "head_json": oversized_head, "verification": "full"}),
            ),
            call(
                "authoring_store_open_revision3",
                json!({"root": unsafe_file, "verification": "full"}),
            ),
            call(
                "authoring_store_open_revision3",
                json!({"root": missing, "verification": "full"}),
            ),
            prepare_revision3(&temp, canonical.clone(), json!("{\"store_format\":1}")),
            prepare_revision3(&temp, canonical.clone(), json!(format!(" {}", "{\"store_format\":1,\"snapshot\":{\"byte_len\":1,\"sha256\":\"1111111111111111111111111111111111111111111111111111111111111111\"}}"))),
            prepare_revision3(&temp, canonical.clone(), json!({"store_format": 1})),
            prepare_revision3(&TempDir::new().unwrap(), String::new(), Value::Null),
            prepare_revision3(&TempDir::new().unwrap(), "\0".to_owned(), Value::Null),
            call(
                "authoring_store_prepare_revision3_checkpoint",
                json!({
                    "root": missing_prepare,
                    "expected_head_json": null,
                    "project_json": canonical,
                }),
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
                "AUTHORING_STORE_PAYLOAD_INVALID",
                "AUTHORING_STORE_PAYLOAD_INVALID",
                "AUTHORING_STORE_PAYLOAD_INVALID",
                "AUTHORING_STORE_VERIFICATION_INVALID",
                "AUTHORING_STORE_INPUT_INVALID",
                "AUTHORING_STORE_INPUT_LIMIT",
                "AUTHORING_STORE_INPUT_LIMIT",
                "AUTHORING_STORE_INPUT_LIMIT",
                "AUTHORING_STORE_PATH_UNSAFE",
                "AUTHORING_STORE_ROOT_MISSING",
                "AUTHORING_STORE_HEAD_INVALID",
                "AUTHORING_STORE_HEAD_NONCANONICAL",
                "AUTHORING_STORE_PAYLOAD_INVALID",
                "AUTHORING_STORE_INPUT_LIMIT",
                "AUTHORING_STORE_PROJECT_INVALID",
                "AUTHORING_STORE_ROOT_MISSING",
            ]
        );
        assert!(!missing.exists());
        assert!(!missing_prepare.exists());
    }

    #[test]
    fn revision3_outer_wire_rejects_duplicates_unknowns_and_wrong_types_before_side_effects() {
        const OPEN: &str = "authoring_store_open_revision3";
        const OPEN_HEAD: &str = "authoring_store_open_revision3_head_bytes";
        const PREPARE: &str = "authoring_store_prepare_revision3_checkpoint";

        let temp = TempDir::new().unwrap();
        let root = serde_json::to_string(temp.path().to_str().unwrap()).unwrap();
        let missing = serde_json::to_string(
            temp.path()
                .join("missing")
                .to_str()
                .expect("temporary test path is UTF-8"),
        )
        .unwrap();
        let project = serde_json::to_string(&revision3_project_json()).unwrap();
        let head = serde_json::to_string(
            r#"{"store_format":1,"snapshot":{"byte_len":1,"sha256":"1111111111111111111111111111111111111111111111111111111111111111"}}"#,
        )
        .unwrap();
        let open_payload = format!(r#"{{"root":{root},"verification":"full"}}"#);
        let open_head_payload =
            format!(r#"{{"head_json":{head},"root":{root},"verification":"full"}}"#);
        let prepare_payload =
            format!(r#"{{"expected_head_json":null,"project_json":{project},"root":{root}}}"#);

        let invalid_requests = [
            // Duplicate top-level discriminants and payloads must not be collapsed by the
            // dispatcher's initial Value probe.
            format!(
                r#"{{"command":"{PREPARE}","command":"{PREPARE}","payload":{prepare_payload}}}"#
            ),
            format!(
                r#"{{"command":"{PREPARE}","payload":{prepare_payload},"payload":{prepare_payload}}}"#
            ),
            format!(
                r#"{{"command":"{OPEN_HEAD}","payload":{open_head_payload},"payload":{open_head_payload}}}"#
            ),
            // Every security-sensitive payload field remains duplicate-safe on the raw route.
            format!(
                r#"{{"command":"{OPEN}","payload":{{"root":{missing},"root":{root},"verification":"full"}}}}"#
            ),
            format!(
                r#"{{"command":"{OPEN}","payload":{{"root":{root},"verification":"structural","verification":"full"}}}}"#
            ),
            format!(
                r#"{{"command":"{OPEN_HEAD}","payload":{{"head_json":{head},"head_json":{head},"root":{root},"verification":"full"}}}}"#
            ),
            format!(
                r#"{{"command":"{PREPARE}","payload":{{"expected_head_json":null,"expected_head_json":null,"project_json":{project},"root":{root}}}}}"#
            ),
            format!(
                r#"{{"command":"{PREPARE}","payload":{{"expected_head_json":null,"project_json":{project},"project_json":{project},"root":{root}}}}}"#
            ),
            // Unknown, missing, and wrong-typed fields are rejected by the same closed wire.
            format!(r#"{{"command":"{OPEN}","payload":{open_payload},"unknown":true}}"#),
            format!(
                r#"{{"command":"{OPEN_HEAD}","payload":{{"head_json":{head},"root":{root},"verification":"full","unknown":true}}}}"#
            ),
            format!(r#"{{"command":"{OPEN}","payload":[]}}"#),
            format!(r#"{{"command":"{OPEN}","payload":{{"root":1,"verification":"full"}}}}"#),
            format!(
                r#"{{"command":"{OPEN_HEAD}","payload":{{"head_json":1,"root":{root},"verification":"full"}}}}"#
            ),
            format!(
                r#"{{"command":"{PREPARE}","payload":{{"expected_head_json":false,"project_json":{project},"root":{root}}}}}"#
            ),
            format!(
                r#"{{"command":"{PREPARE}","payload":{{"expected_head_json":null,"project_json":{{}},"root":{root}}}}}"#
            ),
            format!(
                r#"{{"command":"{PREPARE}","payload":{{"expected_head_json":null,"project_json":{project}}}}}"#
            ),
        ];

        for request in invalid_requests {
            let rejected = call_raw_json(&request);
            assert_eq!(
                rejected["error"]["code"], "AUTHORING_STORE_PAYLOAD_INVALID",
                "request unexpectedly crossed the exact raw wire: {request}"
            );
        }

        let wrong_direct_command = open_revision3_raw(&format!(
            r#"{{"command":"{OPEN_HEAD}","payload":{open_payload}}}"#
        ));
        assert_eq!(
            wrong_direct_command["error"]["code"],
            "AUTHORING_STORE_PAYLOAD_INVALID"
        );
        let oversized = open_revision3_raw(&" ".repeat(MAX_REVISION3_OPEN_WIRE_BYTES + 1));
        assert_eq!(oversized["error"]["code"], "AUTHORING_STORE_INPUT_LIMIT");

        assert!(!temp.path().join("gore-project.json").exists());
        assert!(!temp.path().join("entities").exists());
        assert!(!temp.path().join("snapshots").exists());
        assert!(!temp.path().join(".gore").exists());
    }

    #[test]
    fn revision3_response_builders_enforce_their_serialized_budget() {
        let temp = TempDir::new().unwrap();
        let prepared = prepare_revision3(&temp, revision3_project_json(), Value::Null);
        let head_json = prepared["head_json"].as_str().unwrap();
        let store = open_existing_store(temp.path().to_str().unwrap()).unwrap();
        let opened = store
            .open_revision3_head_bytes(head_json.as_bytes(), AssetVerification::Full)
            .unwrap();

        let open_error = opened_revision3_response(opened.clone(), 1).unwrap_err();
        assert_eq!(
            open_error.response()["error"]["code"],
            "AUTHORING_STORE_RESPONSE_LIMIT"
        );
        let prepare_error = prepared_revision3_response(&opened.head, 1).unwrap_err();
        assert_eq!(
            prepare_error.response()["error"]["code"],
            "AUTHORING_STORE_RESPONSE_LIMIT"
        );
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
