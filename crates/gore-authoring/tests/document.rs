use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision2::{
    Entity, EntityPayload, LocalizationEntry, OriginRef, ProjectRevision2, SchemaRevisionV2,
};
use gore_authoring::{
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode,
    ProjectDocument, ProjectDocumentError, ProjectId, ProjectMeta, ProjectV2, Sha256Digest,
    MAX_PROJECT_JSON_BYTES,
};

fn project_id(value: u128) -> ProjectId {
    format!("{value:032x}").parse().unwrap()
}

fn entity_id(value: u128) -> EntityId {
    format!("{value:032x}").parse().unwrap()
}

fn digest(byte: &str) -> Sha256Digest {
    byte.repeat(32).parse().unwrap()
}

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn generation() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: ContentSeal {
            byte_len: 123,
            sha256: digest("ab"),
        },
    }
}

fn revision2_project() -> ProjectRevision2 {
    let id = entity_id(1);
    let entity = Entity {
        id,
        display_name: "Greeting".into(),
        origin: OriginRef::New {
            authored_runtime_id: "dialog:greeting".into(),
        },
        revision: 4,
        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
            loc_id: "dialog_greeting".into(),
            texts: BTreeMap::from([
                (locale("de"), "Hallo".into()),
                (locale("en"), "Hello".into()),
            ]),
        }),
    };
    ProjectRevision2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV2,
        project_id: project_id(2),
        revision: 9,
        meta: ProjectMeta {
            name: "RevisionTwo".into(),
            version: "1.0".into(),
            author: "tester".into(),
        },
        target: generation(),
        authoring_locales: BTreeSet::from([locale("de"), locale("en")]),
        entities: BTreeMap::from([(id, entity)]),
        asset_store: AssetStoreIndex::default(),
    }
}

fn canonical_empty_revision1() -> String {
    format!(
        concat!(
            "{{\"format\":2,\"schema_revision\":1,",
            "\"project_id\":\"00000000000000000000000000000001\",",
            "\"revision\":0,",
            "\"meta\":{{\"name\":\"FrozenRev1\",\"version\":\"\",\"author\":\"\"}},",
            "\"target\":{{\"executable\":{{\"byte_len\":123,\"sha256\":\"{}\"}}}},",
            "\"authoring_locales\":[],\"entities\":{{}},\"asset_store\":{{\"assets\":{{}}}}}}"
        ),
        "ab".repeat(32)
    )
}

#[test]
fn revision1_dispatch_preserves_existing_canonical_bytes_exactly() {
    let raw = canonical_empty_revision1();
    let legacy = ProjectV2::from_json(&raw).unwrap();
    assert_eq!(legacy.to_canonical_json().unwrap(), raw);

    let document = ProjectDocument::from_json(&raw).unwrap();
    assert!(matches!(document, ProjectDocument::Revision1(_)));
    assert_eq!(document.to_canonical_json().unwrap(), raw);
    assert_eq!(serde_json::to_string(&document).unwrap(), raw);
}

#[test]
fn revision2_dispatch_round_trips_one_canonical_spelling() {
    let project = revision2_project();
    let canonical = project.to_canonical_json().unwrap();
    let document = ProjectDocument::from_json(&canonical).unwrap();
    let ProjectDocument::Revision2(reopened) = &document else {
        panic!("revision-2 marker dispatched to the wrong model");
    };

    assert_eq!(reopened, &project);
    assert_eq!(reopened.to_canonical_json().unwrap(), canonical);
    assert_eq!(document.to_canonical_json().unwrap(), canonical);
    assert_eq!(serde_json::to_string(&document).unwrap(), canonical);
}

#[test]
fn dispatcher_rejects_unknown_format_revision_and_payload_fail_closed() {
    let canonical = revision2_project().to_canonical_json().unwrap();

    let wrong_format = canonical.replacen("\"format\":2", "\"format\":3", 1);
    assert!(matches!(
        ProjectDocument::from_json(&wrong_format),
        Err(ProjectDocumentError::UnsupportedFormat { found: 3 })
    ));

    let unknown_revision =
        canonical.replacen("\"schema_revision\":2", "\"schema_revision\":4294967295", 1);
    assert!(matches!(
        ProjectDocument::from_json(&unknown_revision),
        Err(ProjectDocumentError::UnsupportedSchemaRevision { found: u32::MAX })
    ));

    let unknown_kind = canonical.replacen("\"kind\":\"localization_entry\"", "\"kind\":\"npc\"", 1);
    assert!(matches!(
        ProjectDocument::from_json(&unknown_kind),
        Err(ProjectDocumentError::InvalidRevision2(_))
    ));

    let unknown_payload_field = canonical.replacen(
        "\"loc_id\":\"dialog_greeting\"",
        "\"loc_id\":\"dialog_greeting\",\"future_npc_field\":true",
        1,
    );
    assert!(matches!(
        ProjectDocument::from_json(&unknown_payload_field),
        Err(ProjectDocumentError::InvalidRevision2(_))
    ));
}

#[test]
fn raw_probe_rejects_duplicate_dispatch_and_nested_payload_keys() {
    let canonical = revision2_project().to_canonical_json().unwrap();

    let duplicate_dispatch = canonical.replacen("\"format\":2", "\"format\":2,\"format\":2", 1);
    let error = ProjectDocument::from_json(&duplicate_dispatch).unwrap_err();
    assert!(matches!(error, ProjectDocumentError::InvalidProbeJson(_)));
    assert!(error.to_string().contains("duplicate JSON object key"));
    assert!(error.to_string().contains("format"));

    let duplicate_payload = canonical.replacen(
        "\"loc_id\":\"dialog_greeting\"",
        "\"loc_id\":\"dialog_greeting\",\"loc_id\":\"shadowed\"",
        1,
    );
    let error = ProjectDocument::from_json(&duplicate_payload).unwrap_err();
    assert!(matches!(error, ProjectDocumentError::InvalidProbeJson(_)));
    assert!(error.to_string().contains("duplicate JSON object key"));
    assert!(error.to_string().contains("loc_id"));

    assert!(ProjectRevision2::from_json(&duplicate_payload).is_err());
}

#[test]
fn dispatcher_checks_the_same_coarse_bound_before_probing() {
    let raw = canonical_empty_revision1();
    let mut at_limit = raw;
    at_limit.push_str(&" ".repeat(MAX_PROJECT_JSON_BYTES - at_limit.len()));
    assert_eq!(at_limit.len(), MAX_PROJECT_JSON_BYTES);
    assert!(ProjectDocument::from_json(&at_limit).is_ok());

    let mut oversized = at_limit;
    oversized.push(' ');

    assert!(matches!(
        ProjectDocument::from_json(&oversized),
        Err(ProjectDocumentError::InputTooLarge { actual, limit })
            if actual == MAX_PROJECT_JSON_BYTES + 1 && limit == MAX_PROJECT_JSON_BYTES
    ));
}

#[test]
fn raw_probe_fails_closed_at_excessive_json_recursion_depth() {
    let depth = 256;
    let deeply_nested = format!(
        "{{\"format\":2,\"schema_revision\":1,\"deep\":{}null{}}}",
        "[".repeat(depth),
        "]".repeat(depth)
    );

    let error = ProjectDocument::from_json(&deeply_nested).unwrap_err();
    assert!(matches!(error, ProjectDocumentError::InvalidProbeJson(_)));
    assert!(error.to_string().contains("recursion limit exceeded"));
}
