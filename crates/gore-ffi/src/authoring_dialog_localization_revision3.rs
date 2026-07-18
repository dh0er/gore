//! Exact-current, read-only preview of one managed revision-3 LocalizationEntry.
//!
//! This route is deliberately separate from the closed content-index V1 wire. It returns only
//! bounded per-locale previews from one exact entity and grants no mutation, publication, build,
//! deployment, game, save, topic, or runtime authority.

use std::path::Path;

use gore_authoring::{
    AssetVerification, EntityId, Revision3EntityPayload, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1, MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_read_revision3_dialog_localization_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_LOC_ID_BYTES: usize = MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1;
const ENTITY_ID_BYTES: usize = 32;
const MAX_LOCALES: usize = MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1;
const MAX_PREVIEW_BYTES: usize = 512;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize =
    (MAX_PATH_BYTES + MAX_HEAD_JSON_BYTES + MAX_LOC_ID_BYTES + ENTITY_ID_BYTES) * 6 + 4 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadLocalizationPayload {
    root: String,
    expected_head_json: String,
    localization_id: String,
    expected_localization_revision: u64,
    expected_loc_id: String,
}

#[derive(Debug, Serialize)]
struct LocalePreview {
    locale: String,
    preview: String,
    truncated: bool,
    has_nonempty_text: bool,
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

pub(super) fn read_revision3_dialog_localization_v1_raw(input: &str) -> Value {
    read_inner(input).unwrap_or_else(Failure::response)
}

fn read_inner(input: &str) -> Result<Value, Failure> {
    read_inner_with_seam_and_limit(input, || {}, MAX_RESPONSE_BYTES)
}

fn read_inner_with_seam_and_limit<F>(
    input: &str,
    between_full_opens: F,
    response_limit: usize,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    let payload = parse_wire(input)?;
    validate_path(&payload.root)?;
    validate_loc_id(&payload.expected_loc_id)?;
    signed_wire(
        payload.expected_localization_revision,
        "expected localization revision",
    )?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    signed_wire(expected_head.snapshot.byte_len, "head snapshot byte length")?;
    let localization_id = parse_entity_id(&payload.localization_id)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(head_conflict());
    }
    signed_wire(before.project.revision, "project revision")?;

    let entity = before
        .project
        .entities
        .get(&localization_id)
        .ok_or_else(localization_not_found)?;
    let Revision3EntityPayload::LocalizationEntry(localization) = &entity.payload else {
        return Err(localization_not_found());
    };
    if entity.revision != payload.expected_localization_revision {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_REVISION_CONFLICT",
            "the exact managed LocalizationEntry revision changed",
        ));
    }
    if localization.loc_id != payload.expected_loc_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_IDENTITY_CONFLICT",
            "the exact managed LocalizationEntry identity changed",
        ));
    }
    if localization.texts.len() > MAX_LOCALES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_LOCALE_LIMIT",
            format!("the exact managed LocalizationEntry has more than {MAX_LOCALES} locales"),
        ));
    }

    let locales = localization
        .texts
        .iter()
        .map(|(locale, text)| {
            let (preview, truncated) = preview_prefix(text);
            LocalePreview {
                locale: locale.to_string(),
                preview,
                truncated,
                has_nonempty_text: !text.trim().is_empty(),
            }
        })
        .collect::<Vec<_>>();
    let loc_id = localization.loc_id.clone();
    let localization_revision = entity.revision;

    between_full_opens();

    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != expected_head || after.head != before.head || after.project != before.project {
        return Err(head_conflict());
    }

    enforce_response_budget(
        json!({
            "ok": true,
            "outcome": "read_only",
            "head_json": payload.expected_head_json,
            "project_id": after.project.project_id.to_string(),
            "project_revision": after.project.revision,
            "localization_id": localization_id.to_string(),
            "localization_revision": localization_revision,
            "loc_id": loc_id,
            "locales": locales,
            "content_authority": "read_only_exact_current_localization",
            "build_status": "not_evaluated",
            "runtime_status": "runtime_unqualified",
            "publication_status": "not_applicable",
        }),
        response_limit,
    )
}

fn parse_wire(input: &str) -> Result<ReadLocalizationPayload, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_INPUT_LIMIT",
            format!("localization preview request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<ReadLocalizationPayload> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn validate_loc_id(loc_id: &str) -> Result<(), Failure> {
    if loc_id.is_empty() || loc_id.len() > MAX_LOC_ID_BYTES || loc_id.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_entity_id(input: &str) -> Result<EntityId, Failure> {
    if input.len() != ENTITY_ID_BYTES {
        return Err(invalid_request());
    }
    let id = input.parse::<EntityId>().map_err(|_| invalid_request())?;
    if id.to_string() != input {
        return Err(invalid_request());
    }
    Ok(id)
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_INVALID",
            "expected_head_json is not one closed revision-3 working head",
        )
    })?;
    if serde_json::to_string(&head).ok().as_deref() != Some(input) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn preview_prefix(text: &str) -> (String, bool) {
    if text.len() <= MAX_PREVIEW_BYTES {
        return (text.to_owned(), false);
    }
    let mut end = MAX_PREVIEW_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}

fn signed_wire(value: u64, field: &'static str) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_SIGNED_WIRE_LIMIT",
            format!("{field} exceeds the signed 64-bit transport range"),
        ));
    }
    Ok(())
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let encoded = serde_json::to_vec(&response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_INVARIANT",
            "the localization preview response could not be serialized",
        )
    })?;
    if encoded.len() > limit {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_RESPONSE_LIMIT",
            "the localization preview response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly root, expected_head_json, localization_id, expected_localization_revision, and expected_loc_id",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_CONFLICT",
        "the published revision-3 head or exact project changed during localization preview",
    )
}

fn localization_not_found() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_NOT_FOUND",
        "the exact managed LocalizationEntry is missing or has the wrong entity kind",
    )
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_STORE_IO"
        }
    };
    Failure::new(code, "the exact revision-3 localization Store read failed")
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
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::LocalizationEntry as Revision3LocalizationEntry;
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId,
        ProjectMeta, ProjectRevision3, Revision3DialogLine, Revision3Entity,
        Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef, Revision3TypedRef,
        SchemaRevisionV3, Sha256Digest,
    };
    use serde_json::{json, Map};
    use tempfile::TempDir;

    use super::*;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
        localization_id: EntityId,
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 170_000_000,
                sha256: Sha256Digest::from_bytes([0x10; 32]),
            },
        }
    }

    fn project_with_localization(
        texts: BTreeMap<LocaleCode, String>,
        localization_revision: u64,
    ) -> (ProjectRevision3, EntityId) {
        let localization_id = entity_id(0x41);
        let mut project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x20; 16]),
            revision: 7,
            meta: ProjectMeta {
                name: "Localization preview fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from(["de".parse().unwrap()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        };
        project.entities.insert(
            localization_id,
            Revision3Entity {
                id: localization_id,
                display_name: String::new(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "info_gore_viper_greeting_01".to_owned(),
                },
                revision: localization_revision,
                payload: Revision3EntityPayload::LocalizationEntry(Revision3LocalizationEntry {
                    loc_id: "info_gore_viper_greeting_01".to_owned(),
                    texts,
                }),
            },
        );
        (project, localization_id)
    }

    fn published_store_for(project: ProjectRevision3, localization_id: EntityId) -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            head: prepared.head,
            fixed_head_bytes: prepared.head_bytes,
            localization_id,
        }
    }

    fn published_store(texts: BTreeMap<LocaleCode, String>) -> PublishedStore {
        let (project, localization_id) = project_with_localization(texts, 3);
        published_store_for(project, localization_id)
    }

    fn request_with(
        store: &PublishedStore,
        expected_head: &WorkingHead,
        localization_id: EntityId,
        localization_revision: u64,
        loc_id: &str,
    ) -> String {
        serde_json::to_string(&json!({
            "command": COMMAND,
            "payload": {
                "root": store.temp.path(),
                "expected_head_json": serde_json::to_string(expected_head).unwrap(),
                "localization_id": localization_id.to_string(),
                "expected_localization_revision": localization_revision,
                "expected_loc_id": loc_id,
            }
        }))
        .unwrap()
    }

    fn request(store: &PublishedStore) -> String {
        request_with(
            store,
            &store.head,
            store.localization_id,
            3,
            "info_gore_viper_greeting_01",
        )
    }

    fn error_code(value: &Value) -> &str {
        value["error"]["code"].as_str().unwrap()
    }

    fn file_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, directory: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let mut entries = fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    visit(root, &path, output);
                } else if path.is_file() {
                    output.insert(
                        path.strip_prefix(root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }

        let mut output = BTreeMap::new();
        visit(root, root, &mut output);
        output
    }

    fn keys(object: &Map<String, Value>) -> BTreeSet<&str> {
        object.keys().map(String::as_str).collect()
    }

    #[test]
    fn exact_full_reopen_returns_sorted_closed_read_only_response_without_writes() {
        let store = published_store(BTreeMap::from([
            ("en".parse().unwrap(), "Hello.".to_owned()),
            ("de".parse().unwrap(), "Willkommen.".to_owned()),
        ]));
        let before = file_tree(store.temp.path());

        // Exercise the public dispatcher so registration and raw closed-wire routing are covered
        // by the same exact-current, read-only success proof.
        let response = crate::dispatch(&request(&store));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "read_only");
        assert_eq!(
            response["head_json"],
            serde_json::to_string(&store.head).unwrap()
        );
        assert_eq!(response["project_id"], store.project.project_id.to_string());
        assert_eq!(response["project_revision"], 7);
        assert_eq!(
            response["localization_id"],
            store.localization_id.to_string()
        );
        assert_eq!(response["localization_revision"], 3);
        assert_eq!(response["loc_id"], "info_gore_viper_greeting_01");
        assert!(response.get("display_name").is_none());
        assert_eq!(
            response["content_authority"],
            "read_only_exact_current_localization"
        );
        assert_eq!(response["build_status"], "not_evaluated");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_applicable");
        assert_eq!(
            keys(response.as_object().unwrap()),
            BTreeSet::from([
                "build_status",
                "content_authority",
                "head_json",
                "locales",
                "localization_id",
                "localization_revision",
                "loc_id",
                "ok",
                "outcome",
                "project_id",
                "project_revision",
                "publication_status",
                "runtime_status",
            ])
        );
        let locales = response["locales"].as_array().unwrap();
        assert_eq!(locales.len(), 2);
        assert_eq!(locales[0]["locale"], "de");
        assert_eq!(locales[1]["locale"], "en");
        assert_eq!(locales[0]["preview"], "Willkommen.");
        assert_eq!(locales[0]["truncated"], false);
        assert_eq!(locales[0]["has_nonempty_text"], true);
        for locale in locales {
            assert_eq!(
                keys(locale.as_object().unwrap()),
                BTreeSet::from(["has_nonempty_text", "locale", "preview", "truncated"])
            );
        }
        assert_eq!(file_tree(store.temp.path()), before);
        assert_eq!(
            fs::read(store.temp.path().join("gore-project.json")).unwrap(),
            store.fixed_head_bytes
        );
    }

    #[test]
    fn stale_head_entity_revision_and_loc_id_all_fail_closed() {
        let store = published_store(BTreeMap::from([(
            "de".parse().unwrap(),
            "Willkommen.".to_owned(),
        )]));
        let before = file_tree(store.temp.path());
        let mut stale_head = store.head.clone();
        stale_head.snapshot.sha256 = Sha256Digest::from_bytes([0x77; 32]);

        let cases = [
            (
                request_with(
                    &store,
                    &stale_head,
                    store.localization_id,
                    3,
                    "info_gore_viper_greeting_01",
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_CONFLICT",
            ),
            (
                request_with(
                    &store,
                    &store.head,
                    entity_id(0x55),
                    3,
                    "info_gore_viper_greeting_01",
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_NOT_FOUND",
            ),
            (
                request_with(
                    &store,
                    &store.head,
                    store.localization_id,
                    4,
                    "info_gore_viper_greeting_01",
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_REVISION_CONFLICT",
            ),
            (
                request_with(
                    &store,
                    &store.head,
                    store.localization_id,
                    3,
                    "info_gore_viper_changed",
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_IDENTITY_CONFLICT",
            ),
        ];

        for (wire, expected) in cases {
            let response = read_revision3_dialog_localization_v1_raw(&wire);
            assert_eq!(error_code(&response), expected, "{response}");
        }
        assert_eq!(file_tree(store.temp.path()), before);
    }

    #[test]
    fn exact_entity_with_wrong_kind_fails_as_localization_not_found() {
        let (mut project, localization_id) = project_with_localization(
            BTreeMap::from([("de".parse().unwrap(), "Willkommen.".to_owned())]),
            3,
        );
        let wrong_kind_id = entity_id(0x42);
        project.entities.insert(
            wrong_kind_id,
            Revision3Entity {
                id: wrong_kind_id,
                display_name: "Not a localization".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_DIALOG_WRONG_KIND".to_owned(),
                },
                revision: 3,
                payload: Revision3EntityPayload::DialogLine(Revision3DialogLine {
                    localization: Revision3TypedRef::new(
                        project.project_id,
                        localization_id,
                        Revision3EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: None,
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        let store = published_store_for(project, wrong_kind_id);
        let before = file_tree(store.temp.path());

        let response = read_revision3_dialog_localization_v1_raw(&request(&store));

        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_NOT_FOUND",
            "{response}"
        );
        assert_eq!(file_tree(store.temp.path()), before);
    }

    #[test]
    fn duplicate_envelope_and_payload_fields_fail_as_invalid_request() {
        let store = published_store(BTreeMap::from([(
            "de".parse().unwrap(),
            "Willkommen.".to_owned(),
        )]));
        let before = file_tree(store.temp.path());
        let valid = request(&store);
        let command_field = format!(r#""command":"{COMMAND}""#);
        let duplicate_command = valid.replacen(
            &command_field,
            &format!(r#"{command_field},{command_field}"#),
            1,
        );
        let revision_field = r#""expected_localization_revision":3"#;
        let duplicate_payload_field = valid.replacen(
            revision_field,
            &format!(r#"{revision_field},{revision_field}"#),
            1,
        );
        assert_ne!(duplicate_command, valid);
        assert_ne!(duplicate_payload_field, valid);

        for wire in [duplicate_command, duplicate_payload_field] {
            let response = read_revision3_dialog_localization_v1_raw(&wire);
            assert_eq!(
                error_code(&response),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_REQUEST_INVALID",
                "{response}"
            );
        }
        assert_eq!(file_tree(store.temp.path()), before);
    }

    #[test]
    fn closed_schema_rejects_game_save_and_project_authority_fields() {
        let store = published_store(BTreeMap::from([(
            "de".parse().unwrap(),
            "Willkommen.".to_owned(),
        )]));
        let before = file_tree(store.temp.path());

        for (field, value) in [
            ("game_root", json!("C:/Games/Gothic")),
            ("save_path", json!("C:/Saves/slot.sav")),
            ("current_project_json", json!("{}")),
        ] {
            let mut payload = json!({
                "root": store.temp.path(),
                "expected_head_json": serde_json::to_string(&store.head).unwrap(),
                "localization_id": store.localization_id.to_string(),
                "expected_localization_revision": 3,
                "expected_loc_id": "info_gore_viper_greeting_01",
            });
            payload
                .as_object_mut()
                .unwrap()
                .insert(field.to_owned(), value);
            let wire =
                serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap();
            let response = read_revision3_dialog_localization_v1_raw(&wire);
            assert_eq!(
                error_code(&response),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_REQUEST_INVALID",
                "{response}"
            );
        }
        assert_eq!(file_tree(store.temp.path()), before);
    }

    #[test]
    fn unicode_preview_uses_utf8_boundary_and_whitespace_is_not_content() {
        let unicode = format!("{}😀tail", "a".repeat(510));
        let whitespace = " \t\n\u{2003}".to_owned();
        let bom = "\u{feff}".to_owned();
        let store = published_store(BTreeMap::from([
            ("de".parse().unwrap(), unicode),
            ("en".parse().unwrap(), whitespace.clone()),
            ("fr".parse().unwrap(), "x".repeat(MAX_PREVIEW_BYTES)),
            ("it".parse().unwrap(), bom.clone()),
        ]));

        let response = read_revision3_dialog_localization_v1_raw(&request(&store));

        assert_eq!(response["ok"], true, "{response}");
        let locales = response["locales"].as_array().unwrap();
        assert_eq!(locales[0]["locale"], "de");
        assert_eq!(locales[0]["preview"].as_str().unwrap().len(), 510);
        assert!(locales[0]["preview"]
            .as_str()
            .unwrap()
            .is_char_boundary(510));
        assert_eq!(locales[0]["truncated"], true);
        assert_eq!(locales[0]["has_nonempty_text"], true);
        assert_eq!(locales[1]["locale"], "en");
        assert_eq!(locales[1]["preview"], whitespace);
        assert_eq!(locales[1]["truncated"], false);
        assert_eq!(locales[1]["has_nonempty_text"], false);
        assert_eq!(
            locales[2]["preview"].as_str().unwrap().len(),
            MAX_PREVIEW_BYTES
        );
        assert_eq!(locales[2]["truncated"], false);
        assert_eq!(locales[3]["locale"], "it");
        assert_eq!(locales[3]["preview"], bom);
        assert_eq!(locales[3]["truncated"], false);
        assert_eq!(locales[3]["has_nonempty_text"], true);
    }

    #[test]
    fn locale_count_response_and_error_messages_are_bounded() {
        let texts = |count: usize| {
            (0..count)
                .map(|index| {
                    (
                        format!("aa-{index}").parse::<LocaleCode>().unwrap(),
                        "x".to_owned(),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        };

        let at_limit = published_store(texts(MAX_LOCALES));
        let response = read_revision3_dialog_localization_v1_raw(&request(&at_limit));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["locales"].as_array().unwrap().len(), MAX_LOCALES);

        let too_many = published_store(texts(MAX_LOCALES + 1));
        let response = read_revision3_dialog_localization_v1_raw(&request(&too_many));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_LOCALE_LIMIT"
        );

        let normal = published_store(BTreeMap::from([(
            "de".parse().unwrap(),
            "Willkommen.".to_owned(),
        )]));
        let response = read_inner_with_seam_and_limit(&request(&normal), || {}, 64)
            .unwrap_err()
            .response();
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_RESPONSE_LIMIT"
        );

        let failure = Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_INVARIANT",
            "😀".repeat(MAX_ERROR_MESSAGE_BYTES),
        );
        assert!(failure.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(std::str::from_utf8(failure.message.as_bytes()).is_ok());
        assert!(failure.message.ends_with("..."));
    }

    #[test]
    fn portable_loc_id_at_voice_target_boundary_is_accepted() {
        let boundary_loc_id = "L".repeat(MAX_LOC_ID_BYTES);
        let (mut project, localization_id) = project_with_localization(
            BTreeMap::from([("de".parse().unwrap(), "Willkommen.".to_owned())]),
            3,
        );
        let Revision3EntityPayload::LocalizationEntry(localization) =
            &mut project.entities.get_mut(&localization_id).unwrap().payload
        else {
            unreachable!("fixture entity is a LocalizationEntry")
        };
        localization.loc_id = boundary_loc_id.clone();
        let store = published_store_for(project, localization_id);

        let response = read_revision3_dialog_localization_v1_raw(&request_with(
            &store,
            &store.head,
            store.localization_id,
            3,
            &boundary_loc_id,
        ));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["loc_id"], boundary_loc_id);
        assert!(response.get("display_name").is_none());
    }

    #[test]
    fn second_full_open_rejects_a_concurrent_published_project_change() {
        let store = published_store(BTreeMap::from([(
            "de".parse().unwrap(),
            "Willkommen.".to_owned(),
        )]));
        let wire = request(&store);
        let root = store.temp.path().to_owned();
        let basis = store.head.clone();
        let mut changed = store.project.clone();
        changed.revision += 1;
        changed.meta.name = "Concurrent project".to_owned();

        let failure = read_inner_with_seam_and_limit(
            &wire,
            || {
                let writer = WorkingProjectStore::open_existing(&root, ffi_store_limits()).unwrap();
                let prepared = writer
                    .prepare_revision3_checkpoint(Some(&basis), &changed)
                    .unwrap();
                fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
            },
            MAX_RESPONSE_BYTES,
        )
        .unwrap_err();

        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_CONFLICT"
        );
    }
}
