//! Read-only construction and projection of one exact pinned `story_catalog.v1` document.

use std::{io, path::PathBuf};

use gore_story_catalog::{
    build_known_catalog, build_known_catalog_with_shipping_snapshot, CatalogError, ContentSeal,
    GenerationInputLimits, GenerationPaths, StoryCatalogFile, MAX_CATALOG_JSON_BYTES, MAX_NPCS,
    MAX_QUEST_PARENTS,
};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use crate::{err, err_with_details, unsupported_generation_details};

const MAX_SELECTION_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_BUILD_RESPONSE_BYTES: usize = 48 * 1024 * 1024;
const BUILD_REQUEST_BINDING_DOMAIN: &[u8] =
    b"gore-story-catalog.authoring-build-v1.request-binding\0";
const GAME_ROOT_REQUEST_BINDING_DOMAIN: &[u8] =
    b"gore-story-catalog.authoring-build-for-game-root-v1.request-binding\0";

pub(super) fn build_story_catalog_v1(payload: Value) -> Value {
    match build_story_catalog_v1_inner(&payload) {
        Ok(response) => response,
        Err(error) => error,
    }
}

fn build_story_catalog_v1_inner(payload: &Value) -> Result<Value, Value> {
    let object = exact_build_payload(payload)?;
    let executable = bounded_path(object, "executable")?;
    let shipping_cache = bounded_path(object, "shipping_cache")?;
    let binds_cache = bounded_path(object, "binds_cache")?;
    let request_binding_sha256 = build_request_binding(executable, shipping_cache, binds_cache);
    let paths = GenerationPaths {
        executable: PathBuf::from(executable),
        shipping_cache: PathBuf::from(shipping_cache),
        binds_cache: PathBuf::from(binds_cache),
    };
    let catalog =
        build_known_catalog(&paths, GenerationInputLimits::default()).map_err(map_build_error)?;

    serialize_catalog_build(&catalog, request_binding_sha256, || Ok(()))
}

/// Build the pinned catalog from a game root while keeping deployment-record and pristine-backup
/// selection entirely inside native code.
pub(super) fn build_story_catalog_for_game_root_v1(payload: Value) -> Value {
    match build_story_catalog_for_game_root_v1_inner(&payload) {
        Ok(response) => response,
        Err(error) => error,
    }
}

fn build_story_catalog_for_game_root_v1_inner(payload: &Value) -> Result<Value, Value> {
    let object = exact_game_root_payload(payload)?;
    let game_root = bounded_game_root(object)?;
    let request_binding_sha256 = game_root_request_binding(game_root);
    let game_root = PathBuf::from(game_root);
    let g1r = if game_root.file_name().is_some_and(|name| name == "G1R") {
        game_root.clone()
    } else {
        game_root.join("G1R")
    };
    let executable = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let binds_cache = g1r.join("Script").join("Binds.Cache");

    let shipping_snapshot =
        gore_mod::pristine_script_cache(&game_root).map_err(map_pristine_error)?;
    let catalog = build_known_catalog_with_shipping_snapshot(
        &executable,
        &shipping_snapshot,
        &binds_cache,
        GenerationInputLimits::default(),
    )
    .map_err(map_build_error)?;
    drop(shipping_snapshot);

    let expected_shipping = catalog.generation().shipping_cache.clone();
    serialize_catalog_build(&catalog, request_binding_sha256, || {
        reselect_pristine_and_verify(&expected_shipping, || {
            gore_mod::pristine_script_cache(&game_root).map_err(map_pristine_error)
        })
    })
}

fn serialize_catalog_build<F>(
    catalog: &StoryCatalogFile,
    request_binding_sha256: String,
    mut revalidate_pristine: F,
) -> Result<Value, Value>
where
    F: FnMut() -> Result<(), Value>,
{
    // Reopen every live guarded path and reselect the deployment-aware pristine snapshot
    // immediately around both canonical and outer response serialization.
    catalog
        .revalidate_generation_inputs()
        .map_err(map_build_error)?;
    revalidate_pristine()?;
    let catalog_json = catalog.to_canonical_json().map_err(map_build_error)?;
    catalog
        .revalidate_generation_inputs()
        .map_err(map_build_error)?;
    revalidate_pristine()?;
    let catalog_json = String::from_utf8(catalog_json).map_err(|_| {
        err(
            "AUTHORING_STORY_CATALOG_BUILD_FAILED",
            "built story catalog was not UTF-8",
        )
    })?;

    let response = json!({
        "ok": true,
        "request_binding_sha256": request_binding_sha256,
        "catalog_json": catalog_json,
        "generation": catalog.generation(),
        "catalog_seal": catalog.catalog_seal(),
    });
    let encoded = serde_json::to_vec(&response).map_err(|_| {
        err(
            "AUTHORING_STORY_CATALOG_BUILD_FAILED",
            "built story catalog response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_BUILD_RESPONSE_BYTES {
        return Err(err(
            "AUTHORING_STORY_CATALOG_BUILD_RESPONSE_LIMIT",
            "built story catalog response exceeds its bounded transport budget",
        ));
    }
    catalog
        .revalidate_generation_inputs()
        .map_err(map_build_error)?;
    revalidate_pristine()?;
    Ok(response)
}

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

fn exact_build_payload(payload: &Value) -> Result<&Map<String, Value>, Value> {
    let Some(object) = payload.as_object() else {
        return Err(err(
            "AUTHORING_STORY_CATALOG_BUILD_REQUEST_INVALID",
            "payload must be an object containing the three generation paths",
        ));
    };
    if object.len() != 3
        || !object.contains_key("executable")
        || !object.contains_key("shipping_cache")
        || !object.contains_key("binds_cache")
    {
        return Err(err(
            "AUTHORING_STORY_CATALOG_BUILD_REQUEST_INVALID",
            "payload must contain exactly executable, shipping_cache, and binds_cache",
        ));
    }
    Ok(object)
}

fn exact_game_root_payload(payload: &Value) -> Result<&Map<String, Value>, Value> {
    let Some(object) = payload.as_object() else {
        return Err(invalid_game_root_request());
    };
    if object.len() != 1 || !object.contains_key("game_root") {
        return Err(invalid_game_root_request());
    }
    Ok(object)
}

fn bounded_game_root(object: &Map<String, Value>) -> Result<&str, Value> {
    object
        .get("game_root")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_PATH_BYTES && !value.contains('\0'))
        .ok_or_else(invalid_game_root_request)
}

fn invalid_game_root_request() -> Value {
    err(
        "AUTHORING_STORY_CATALOG_GAME_ROOT_REQUEST_INVALID",
        "payload must contain exactly one non-empty bounded game_root path",
    )
}

fn bounded_path<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str, Value> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_PATH_BYTES && !value.contains('\0'))
        .ok_or_else(|| {
            err(
                "AUTHORING_STORY_CATALOG_BUILD_REQUEST_INVALID",
                "each generation path must be a non-empty bounded UTF-8 string without NUL",
            )
        })
}

fn build_request_binding(executable: &str, shipping_cache: &str, binds_cache: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(BUILD_REQUEST_BINDING_DOMAIN);
    for value in [executable, shipping_cache, binds_cache] {
        let bytes = value.as_bytes();
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    hex_digest(hasher.finalize())
}

fn game_root_request_binding(game_root: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(GAME_ROOT_REQUEST_BINDING_DOMAIN);
    let bytes = game_root.as_bytes();
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hex_digest(hasher.finalize())
}

fn verify_shipping_snapshot(bytes: &[u8], expected: &ContentSeal) -> Result<(), Value> {
    let Ok(byte_len) = u64::try_from(bytes.len()) else {
        return Err(story_catalog_input_changed());
    };
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    if byte_len != expected.byte_len || digest.as_slice() != expected.sha256.as_bytes() {
        return Err(story_catalog_input_changed());
    }
    Ok(())
}

fn reselect_pristine_and_verify<F>(expected: &ContentSeal, mut select: F) -> Result<(), Value>
where
    F: FnMut() -> Result<Vec<u8>, Value>,
{
    let current = select()?;
    verify_shipping_snapshot(&current, expected)
}

fn story_catalog_input_changed() -> Value {
    err(
        "AUTHORING_STORY_CATALOG_BUILD_INPUT_CHANGED",
        "a generation input changed while the catalog was being built",
    )
}

fn map_pristine_error(error: gore_mod::ModError) -> Value {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return err(
            "AUTHORING_STORY_CATALOG_BUILD_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before Story authoring",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return err(
            "AUTHORING_STORY_CATALOG_BUILD_LIMIT",
            "the pristine Shipping cache exceeds the supported resource limits",
        );
    }
    if message.contains("not a regular non-link file") {
        return err(
            "AUTHORING_STORY_CATALOG_BUILD_UNSAFE_INPUT",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    err(
        "AUTHORING_STORY_CATALOG_BUILD_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_build_error(error: CatalogError) -> Value {
    let (code, message) = match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => (
            "AUTHORING_STORY_CATALOG_BUILD_LIMIT",
            "a generation input exceeds the supported resource limits",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => (
            "AUTHORING_STORY_CATALOG_BUILD_UNSAFE_INPUT",
            "a generation input is not a safe single-link regular file",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => (
            "AUTHORING_STORY_CATALOG_BUILD_INPUT_CHANGED",
            "a generation input changed while the catalog was being built",
        ),
        // The error arrives by value, so the observed triple and the supported ones are right here.
        // Flattening them into one sentence was the last place these facts existed before the
        // surface, and the surface is exactly where somebody needs them.
        CatalogError::UnsupportedGeneration { supported, actual } => {
            return err_with_details(
                "AUTHORING_STORY_CATALOG_BUILD_UNSUPPORTED_GENERATION",
                "the three inputs do not match any supported game generation",
                unsupported_generation_details(&supported, &actual),
            );
        }
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => (
            "AUTHORING_STORY_CATALOG_BUILD_INPUT_MISSING",
            "a required generation input does not exist",
        ),
        CatalogError::Io { .. } => (
            "AUTHORING_STORY_CATALOG_BUILD_INPUT_IO",
            "a generation input could not be read safely",
        ),
        CatalogError::MissingInputGuard => (
            "AUTHORING_STORY_CATALOG_BUILD_FAILED",
            "built story catalog lost its generation-input guard",
        ),
        _ => (
            "AUTHORING_STORY_CATALOG_BUILD_FAILED",
            "the pinned story catalog could not be built",
        ),
    };
    err(code, message)
}

fn hex_sha256(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(digest: impl IntoIterator<Item = u8>) -> String {
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
    use gore_story_catalog::Sha256Digest;
    use std::collections::BTreeMap;
    use std::fs;

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

    #[test]
    fn build_transport_binds_paths_and_maps_fixture_failures_without_leaking_paths() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("fixture-game.exe");
        let shipping_cache = root.path().join("fixture-shipping.cache");
        let binds_cache = root.path().join("fixture-binds.cache");
        fs::write(&executable, b"fixture executable").unwrap();
        fs::write(&shipping_cache, b"fixture shipping cache").unwrap();
        fs::write(&binds_cache, b"fixture binds cache").unwrap();
        let payload = json!({
            "executable": executable.to_string_lossy(),
            "shipping_cache": shipping_cache.to_string_lossy(),
            "binds_cache": binds_cache.to_string_lossy(),
        });
        let request = serde_json::to_string(&json!({
            "command": "authoring_story_catalog_v1_build",
            "payload": payload,
        }))
        .unwrap();
        let response: Value = serde_json::from_str(&crate::execute_json(&request)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_STORY_CATALOG_BUILD_UNSUPPORTED_GENERATION"
        );
        assert!(!response
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));

        fs::remove_file(&binds_cache).unwrap();
        let missing = build_story_catalog_v1(payload.clone());
        assert_eq!(
            missing["error"]["code"],
            "AUTHORING_STORY_CATALOG_BUILD_INPUT_MISSING"
        );
        assert!(!missing
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));

        fs::write(&binds_cache, b"fixture binds cache").unwrap();
        let binds_alias = root.path().join("fixture-binds-alias.cache");
        fs::hard_link(&binds_cache, &binds_alias).unwrap();
        let unsafe_input = build_story_catalog_v1(payload);
        assert_eq!(
            unsafe_input["error"]["code"],
            "AUTHORING_STORY_CATALOG_BUILD_UNSAFE_INPUT"
        );
    }

    #[test]
    fn build_payload_and_binding_are_exact_and_bounded() {
        let valid = json!({
            "executable": "A/game.exe",
            "shipping_cache": "B/Shipping-G1-Game.cache",
            "binds_cache": "C/Binds.cache",
        });
        let object = exact_build_payload(&valid).unwrap();
        let binding = build_request_binding(
            bounded_path(object, "executable").unwrap(),
            bounded_path(object, "shipping_cache").unwrap(),
            bounded_path(object, "binds_cache").unwrap(),
        );
        assert_eq!(
            binding,
            "86c32f29c17846499a62e6acf9778610fe25b445930519e6e055aa427519cb37"
        );
        assert_ne!(
            binding,
            build_request_binding("A/game.exe", "C/Binds.cache", "B/Shipping-G1-Game.cache")
        );

        for invalid in [
            Value::Null,
            json!({}),
            json!({"executable":"a","shipping_cache":"b","binds_cache":"c","extra":true}),
            json!({"executable":"","shipping_cache":"b","binds_cache":"c"}),
            json!({"executable":"a\u{0}b","shipping_cache":"b","binds_cache":"c"}),
            json!({"executable":"x".repeat(MAX_PATH_BYTES + 1),"shipping_cache":"b","binds_cache":"c"}),
        ] {
            let response = build_story_catalog_v1(invalid);
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_STORY_CATALOG_BUILD_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn game_root_request_is_exact_bound_and_derives_no_client_cache_path() {
        let valid = json!({"game_root": "C:/Games/Gothic"});
        let object = exact_game_root_payload(&valid).unwrap();
        let root = bounded_game_root(object).unwrap();
        assert_eq!(root, "C:/Games/Gothic");
        assert_eq!(
            game_root_request_binding(root),
            "208d76c5754bc4457ea54b30605d1081b21894d3d8ea925c5e925257da370f7b"
        );
        assert_ne!(
            game_root_request_binding(root),
            game_root_request_binding("C:/Games/Other")
        );
        for invalid in [
            Value::Null,
            json!({}),
            json!({"game_root": "",}),
            json!({"game_root": "x\0y"}),
            json!({"game_root": "x", "shipping_cache": "client-choice"}),
            json!({"game_root": "x".repeat(MAX_PATH_BYTES + 1)}),
        ] {
            assert_eq!(
                build_story_catalog_for_game_root_v1(invalid)["error"]["code"],
                "AUTHORING_STORY_CATALOG_GAME_ROOT_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn game_root_transport_uses_native_pristine_selection_and_sanitizes_paths() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("private-game-root");
        let g1r = game.join("G1R");
        fs::create_dir_all(g1r.join("Binaries/Win64")).unwrap();
        fs::create_dir_all(g1r.join("Script")).unwrap();
        fs::write(
            g1r.join("Binaries/Win64/G1R-Win64-Shipping.exe"),
            b"fixture exe",
        )
        .unwrap();
        fs::write(
            g1r.join("Script/PrecompiledScript_Shipping.Cache"),
            b"fixture pristine",
        )
        .unwrap();
        fs::write(g1r.join("Script/Binds.Cache"), b"fixture binds").unwrap();
        let response = build_story_catalog_for_game_root_v1(json!({
            "game_root": game.to_string_lossy(),
        }));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_STORY_CATALOG_BUILD_UNSUPPORTED_GENERATION"
        );
        assert!(!response
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));

        let request = serde_json::to_string(&json!({
            "command": "authoring_story_catalog_v1_build_for_game_root",
            "payload": {"game_root": game.to_string_lossy()},
        }))
        .unwrap();
        let dispatched: Value = serde_json::from_str(&crate::execute_json(&request)).unwrap();
        assert_eq!(dispatched, response);
    }

    #[test]
    fn native_pristine_selection_covers_live_backup_drift_and_recovery() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let script = game.join("G1R/Script");
        fs::create_dir_all(&script).unwrap();
        let live = script.join("PrecompiledScript_Shipping.Cache");
        let backup = PathBuf::from(format!("{}.gore-bak", live.display()));
        let pristine = b"pristine cache";
        let modded = b"deployed cache";
        fs::write(&live, pristine).unwrap();
        assert_eq!(gore_mod::pristine_script_cache(&game).unwrap(), pristine);

        fs::write(&backup, pristine).unwrap();
        fs::write(&live, modded).unwrap();
        // Production deploy records persist canonical paths even when the caller uses an alias.
        let recorded_live = fs::canonicalize(&live).unwrap();
        let recorded_backup = fs::canonicalize(&backup).unwrap();
        let mut record = gore_mod::DeployRecord {
            mod_name: "fixture".to_owned(),
            backups: vec![(
                recorded_live.display().to_string(),
                recorded_backup.display().to_string(),
                true,
            )],
            ..Default::default()
        };
        record.deployed_hashes =
            BTreeMap::from([(recorded_live.display().to_string(), fnv1a64_hex(modded))]);
        record.backup_hashes = BTreeMap::from([(
            recorded_backup.display().to_string(),
            format!("sha256:{}", hex_sha256(pristine)),
        )]);
        let record_path = game.join("gore-mod.deployed.json");
        fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert_eq!(gore_mod::pristine_script_cache(&game).unwrap(), pristine);

        let updated = b"hotfix cache";
        fs::write(&live, updated).unwrap();
        assert_eq!(gore_mod::pristine_script_cache(&game).unwrap(), updated);

        record.phase = gore_mod::DeployPhase::RecoveryRequired;
        fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
        let mapped = map_pristine_error(gore_mod::pristine_script_cache(&game).unwrap_err());
        assert_eq!(
            mapped["error"]["code"],
            "AUTHORING_STORY_CATALOG_BUILD_RECOVERY_REQUIRED"
        );
        assert!(!mapped
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn pristine_reselection_hook_fails_on_a_raced_snapshot() {
        let original = b"original pristine";
        let expected = ContentSeal {
            byte_len: original.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(original).into()),
        };
        reselect_pristine_and_verify(&expected, || Ok(original.to_vec())).unwrap();
        let error =
            reselect_pristine_and_verify(&expected, || Ok(b"raced pristine".to_vec())).unwrap_err();
        assert_eq!(
            error["error"]["code"],
            "AUTHORING_STORY_CATALOG_BUILD_INPUT_CHANGED"
        );
    }

    fn fnv1a64_hex(bytes: &[u8]) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }
}
