//! Read-only projection of one exact pinned `story_catalog.v1` document.

use gore_story_catalog::{StoryCatalogFile, MAX_CATALOG_JSON_BYTES, MAX_NPCS, MAX_QUEST_PARENTS};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

const MAX_SELECTION_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn read_story_catalog_v1(payload: Value) -> Value {
    match read_story_catalog_v1_inner(&payload) {
        Ok(response) => response,
        Err(error) => error,
    }
}

fn read_story_catalog_v1_inner(payload: &Value) -> Result<Value, Value> {
    let object = exact_payload(payload)?;
    let catalog_json = object
        .get("catalog_json")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            err(
                "AUTHORING_STORY_CATALOG_REQUEST_INVALID",
                "catalog_json must be one non-empty raw JSON string",
            )
        })?;
    if catalog_json.len() > MAX_CATALOG_JSON_BYTES {
        return Err(err(
            "AUTHORING_STORY_CATALOG_LIMIT",
            format!("story catalog JSON exceeds the {MAX_CATALOG_JSON_BYTES}-byte limit"),
        ));
    }

    // This is the only trust transition. Keeping the nested JSON as a raw string preserves
    // duplicate-key and non-canonical-byte evidence for the pinned reader.
    let catalog = StoryCatalogFile::from_json(catalog_json.as_bytes()).map_err(|_| {
        err(
            "AUTHORING_STORY_CATALOG_UNTRUSTED",
            "catalog_json is not the pinned canonical story_catalog.v1 document",
        )
    })?;
    let selections = catalog.authoring_selections().map_err(|_| {
        err(
            "AUTHORING_STORY_CATALOG_PROJECTION_FAILED",
            "trusted story catalog could not be projected for authoring",
        )
    })?;
    if selections.npcs.len() > MAX_NPCS || selections.quest_parents.len() > MAX_QUEST_PARENTS {
        return Err(err(
            "AUTHORING_STORY_CATALOG_PROJECTION_FAILED",
            "trusted story catalog projection exceeds its record limits",
        ));
    }

    let request_catalog_sha256 = hex_sha256(catalog_json.as_bytes());
    let response = json!({
        "ok": true,
        "request_catalog_sha256": request_catalog_sha256,
        "selections": selections,
    });
    let encoded = serde_json::to_vec(&response).map_err(|_| {
        err(
            "AUTHORING_STORY_CATALOG_PROJECTION_FAILED",
            "story catalog projection could not be serialized",
        )
    })?;
    if encoded.len() > MAX_SELECTION_RESPONSE_BYTES {
        return Err(err(
            "AUTHORING_STORY_CATALOG_RESPONSE_LIMIT",
            "story catalog authoring projection exceeds its response limit",
        ));
    }
    Ok(response)
}

fn exact_payload(payload: &Value) -> Result<&Map<String, Value>, Value> {
    let Some(object) = payload.as_object() else {
        return Err(err(
            "AUTHORING_STORY_CATALOG_REQUEST_INVALID",
            "payload must be an object containing only catalog_json",
        ));
    };
    if object.len() != 1 || !object.contains_key("catalog_json") {
        return Err(err(
            "AUTHORING_STORY_CATALOG_REQUEST_INVALID",
            "payload must contain exactly catalog_json",
        ));
    }
    Ok(object)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trusted() -> &'static str {
        include_str!("../tests/fixtures/story_catalog_v1.json").trim_end_matches(['\r', '\n'])
    }

    #[test]
    fn trusted_catalog_projects_deterministically_without_raw_paths_or_inventory() {
        let trusted = trusted();
        let payload = json!({"catalog_json": trusted});
        let first = read_story_catalog_v1_inner(&payload).unwrap();
        let second = read_story_catalog_v1_inner(&payload).unwrap();
        assert_eq!(first, second);
        let request = serde_json::to_string(&json!({
            "command": "authoring_story_catalog_v1_read",
            "payload": payload,
        }))
        .unwrap();
        let through_transport: Value =
            serde_json::from_str(&crate::execute_json(&request)).unwrap();
        assert_eq!(through_transport, first);
        assert_eq!(first["ok"], true);
        assert_eq!(
            first["request_catalog_sha256"],
            hex_sha256(trusted.as_bytes())
        );
        assert_eq!(first["selections"]["schema_revision"], 1);
        assert_eq!(first["selections"]["npcs"].as_array().unwrap().len(), 2);
        assert_eq!(
            first["selections"]["quest_parents"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            first["selections"]["quest_collision_catalog"]["status"],
            "inventory_unavailable"
        );
        assert_eq!(
            first["selections"]["quest_collision_catalog"]["blocks_draft_creation"],
            true
        );
        let encoded = serde_json::to_string(&first).unwrap();
        assert!(!encoded.contains("relative_path"));
        assert!(!encoded.contains("\"module\""));
        assert!(!encoded.contains("catalog_json"));
    }

    #[test]
    fn nested_duplicate_noncanonical_and_forged_catalogs_fail_closed() {
        let trusted = trusted();
        let duplicate = trusted.replacen(
            "\"format\":\"story_catalog\"",
            "\"format\":\"story_catalog\",\"format\":\"story_catalog\"",
            1,
        );
        let noncanonical = format!("{trusted}\n");
        let forged = trusted.replacen(
            "\"display_name\":\"Asghan\"",
            "\"display_name\":\"Other\"",
            1,
        );
        for raw in [duplicate, noncanonical, forged] {
            let response = read_story_catalog_v1(json!({"catalog_json": raw}));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_STORY_CATALOG_UNTRUSTED"
            );
        }
    }

    #[test]
    fn request_schema_and_raw_size_are_bounded_before_trust_parsing() {
        for payload in [
            Value::Null,
            json!({}),
            json!({"catalog_json": 1}),
            json!({"catalog_json": ""}),
            json!({"catalog_json": trusted(), "extra": true}),
        ] {
            let response = read_story_catalog_v1(payload);
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_STORY_CATALOG_REQUEST_INVALID"
            );
        }

        let oversized = "x".repeat(MAX_CATALOG_JSON_BYTES + 1);
        let response = read_story_catalog_v1(json!({"catalog_json": oversized}));
        assert_eq!(response["error"]["code"], "AUTHORING_STORY_CATALOG_LIMIT");
    }
}
