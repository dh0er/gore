//! Strict raw FFI adapter for Mod Manager V1's read-only first-run evidence.

use std::path::Path;

use gore_mod::mgr::preflight::ManagerPreflightV1;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "mgr_preflight_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
// Three path strings may each expand to six JSON bytes per input byte (`\u00XX`). This route sees
// the untouched wire and rejects it before serde allocates a generic request tree.
const MAX_WIRE_BYTES: usize = MAX_PATH_BYTES * 18 + 4 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactRequest {
    command: String,
    payload: Payload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    game_root: String,
    #[serde(default)]
    library_dir: Option<String>,
    #[serde(default)]
    loadout_path: Option<String>,
}

pub(super) fn mgr_preflight_v1_raw(input: &str) -> Value {
    let payload = match parse_request(input) {
        Ok(payload) => payload,
        Err(message) => return err("MGR_PREFLIGHT_BAD_REQUEST", message),
    };
    let (library_dir, loadout_path) = match crate::mgr_store_paths_from_options(
        payload.library_dir.as_deref(),
        payload.loadout_path.as_deref(),
    ) {
        Ok(paths) => paths,
        Err(message) => return err("MGR_PREFLIGHT_BAD_REQUEST", message),
    };
    let preflight = run_preflight(Path::new(&payload.game_root), &library_dir, &loadout_path);
    bounded_response(preflight)
}

#[cfg(not(test))]
fn run_preflight(game_root: &Path, library: &Path, loadout: &Path) -> ManagerPreflightV1 {
    gore_mod::mgr::preflight::preflight_v1(game_root, library, loadout)
}

#[cfg(test)]
fn run_preflight(game_root: &Path, library: &Path, loadout: &Path) -> ManagerPreflightV1 {
    gore_mod::mgr::preflight::preflight_v1_with_stated_game_process(
        game_root,
        library,
        loadout,
        || Ok(false),
    )
}

fn parse_request(input: &str) -> Result<Payload, &'static str> {
    if input.len() > MAX_WIRE_BYTES {
        return Err("manager preflight request exceeds its bounded wire limit");
    }
    let request: ExactRequest = serde_json::from_str(input)
        .map_err(|_| "manager preflight request has an invalid schema")?;
    if request.command != COMMAND {
        return Err("manager preflight request command does not match this route");
    }
    validate_path(&request.payload.game_root, "game_root")?;
    if let Some(path) = request.payload.library_dir.as_deref() {
        validate_path(path, "library_dir")?;
    }
    if let Some(path) = request.payload.loadout_path.as_deref() {
        validate_path(path, "loadout_path")?;
    }
    Ok(request.payload)
}

fn validate_path(path: &str, _field: &'static str) -> Result<(), &'static str> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err("a path is empty, contains NUL, or exceeds its bounded length");
    }
    Ok(())
}

fn bounded_response(preflight: ManagerPreflightV1) -> Value {
    let response = json!({"ok": true, "preflight": preflight});
    if serde_json::to_vec(&response).is_ok_and(|wire| wire.len() <= MAX_RESPONSE_BYTES) {
        response
    } else {
        err(
            "MGR_PREFLIGHT_OUTPUT_LIMIT",
            "manager preflight response exceeds its bounded output limit",
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use gore_mod::mgr::preflight::{PreflightCheckIdV1, PreflightCheckV1, PreflightStateV1};

    use super::*;

    fn execute(input: Value) -> Value {
        mgr_preflight_v1_raw(&input.to_string())
    }

    fn install_fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for directory in [
            "G1R/Binaries/Win64",
            "G1R/Content/Paks",
            "G1R/Story/Cache",
            "G1R/Script",
        ] {
            fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        fs::write(
            root.path()
                .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe"),
            b"exe",
        )
        .unwrap();
        root
    }

    #[test]
    fn exact_request_returns_environmental_findings_as_ok() {
        let install = install_fixture();
        let response = execute(json!({
            "command": COMMAND,
            "payload": {
                "game_root": install.path(),
                "library_dir": install.path().join("library"),
                "loadout_path": install.path().join("loadout.json"),
            }
        }));
        assert_eq!(response["ok"], true);
        assert_eq!(response["preflight"]["format"], 1);
        assert_eq!(response["preflight"]["checks"].as_array().unwrap().len(), 7);
        assert_eq!(response["preflight"]["checks"][6]["state"], "unverified");
        assert_eq!(
            response["preflight"]["checks"][6]["code"],
            "unverified_read_only"
        );
        assert!(response["preflight"].get("ready").is_none());
        assert!(response["preflight"].get("can_apply").is_none());
        assert!(response.get("ready").is_none());
        assert!(response.get("can_apply").is_none());

        let missing_root = install.path().join("not-installed");
        let response = execute(json!({
            "command": COMMAND,
            "payload": {
                "game_root": missing_root,
                "library_dir": install.path().join("library"),
                "loadout_path": install.path().join("loadout.json"),
            }
        }));
        assert_eq!(response["ok"], true);
        assert_eq!(response["preflight"]["checks"][0]["state"], "problem");
        assert_eq!(
            response["preflight"]["checks"][0]["code"],
            "game_root_not_found"
        );
    }

    #[test]
    fn omitted_manager_paths_are_accepted_but_game_root_is_required() {
        let payload = parse_request(&format!(
            r#"{{"command":"{COMMAND}","payload":{{"game_root":"C:/explicit-game"}}}}"#
        ))
        .unwrap();
        assert_eq!(payload.game_root, "C:/explicit-game");
        assert!(payload.library_dir.is_none());
        assert!(payload.loadout_path.is_none());

        let missing = mgr_preflight_v1_raw(&format!(r#"{{"command":"{COMMAND}","payload":{{}}}}"#));
        assert_eq!(missing["ok"], false);
        assert_eq!(missing["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST");
    }

    #[test]
    fn manager_store_path_overrides_must_be_paired() {
        for payload in [
            json!({"game_root":"C:/game", "library_dir":"C:/library"}),
            json!({"game_root":"C:/game", "loadout_path":"C:/loadout.json"}),
        ] {
            let parsed =
                parse_request(&json!({"command":COMMAND, "payload":payload}).to_string()).unwrap();
            let error = crate::mgr_store_paths_from_options(
                parsed.library_dir.as_deref(),
                parsed.loadout_path.as_deref(),
            )
            .unwrap_err();
            assert!(error.contains("must be supplied together"), "{error}");
        }

        let parsed = parse_request(
            &json!({
                "command":COMMAND,
                "payload":{
                    "game_root":"C:/game",
                    "library_dir":"C:/library",
                    "loadout_path":"C:/loadout.json"
                }
            })
            .to_string(),
        )
        .unwrap();
        assert!(crate::mgr_store_paths_from_options(
            parsed.library_dir.as_deref(),
            parsed.loadout_path.as_deref(),
        )
        .is_ok());
    }

    #[test]
    fn unknown_duplicate_and_missing_outer_fields_are_rejected() {
        for input in [
            format!(r#"{{"command":"{COMMAND}","payload":{{"game_root":"x"}},"extra":true}}"#),
            format!(
                r#"{{"command":"{COMMAND}","command":"{COMMAND}","payload":{{"game_root":"x"}}}}"#
            ),
            format!(r#"{{"command":"{COMMAND}"}}"#),
            r#"{"payload":{"game_root":"x"}}"#.to_owned(),
            format!(
                r#"{{"command":"{COMMAND}","payload":{{"game_root":"x"}},"payload":{{"game_root":"y"}}}}"#
            ),
        ] {
            let response = mgr_preflight_v1_raw(&input);
            assert_eq!(response["ok"], false, "input: {input}");
            assert_eq!(
                response["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST",
                "input: {input}"
            );
        }
    }

    #[test]
    fn public_dispatch_keeps_duplicate_commands_inside_the_strict_raw_parser() {
        for input in [
            format!(
                r#"{{"command":"{COMMAND}","payload":{{"game_root":"x"}},"command":"core_info"}}"#
            ),
            format!(
                r#"{{"command":"core_info","command":"{COMMAND}","payload":{{"game_root":"x"}}}}"#
            ),
        ] {
            let response = crate::dispatch(&input);
            assert_eq!(response["ok"], false, "input: {input}");
            assert_eq!(response["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST");
        }
    }

    #[test]
    fn unknown_duplicate_and_wrongly_typed_payload_fields_are_rejected() {
        for input in [
            format!(r#"{{"command":"{COMMAND}","payload":{{"game_root":"x","extra":true}}}}"#),
            format!(r#"{{"command":"{COMMAND}","payload":{{"game_root":"x","game_root":"y"}}}}"#),
            format!(r#"{{"command":"{COMMAND}","payload":{{"game_root":42}}}}"#),
        ] {
            let response = mgr_preflight_v1_raw(&input);
            assert_eq!(response["ok"], false, "input: {input}");
            assert_eq!(response["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST");
        }
    }

    #[test]
    fn wrong_command_empty_nul_and_oversized_paths_are_rejected() {
        for input in [
            json!({"command":"mgr_status", "payload":{"game_root":"x"}}).to_string(),
            json!({"command":COMMAND, "payload":{"game_root":""}}).to_string(),
            json!({"command":COMMAND, "payload":{"game_root":"x\u{0}"}}).to_string(),
            json!({
                "command":COMMAND,
                "payload":{"game_root":"x", "library_dir":"x".repeat(MAX_PATH_BYTES + 1)}
            })
            .to_string(),
        ] {
            let response = mgr_preflight_v1_raw(&input);
            assert_eq!(response["ok"], false, "input length: {}", input.len());
            assert_eq!(response["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST");
        }
    }

    #[test]
    fn oversized_raw_wire_is_rejected_before_decode() {
        let response = mgr_preflight_v1_raw(&"x".repeat(MAX_WIRE_BYTES + 1));
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST");
    }

    #[test]
    fn output_cap_fails_closed_without_partial_snapshot() {
        let oversized = PreflightCheckV1 {
            id: PreflightCheckIdV1::GameRoot,
            state: PreflightStateV1::Unknown,
            code: "injected_test",
            action: "none",
            action_token: None,
            detail: "x".repeat(MAX_RESPONSE_BYTES),
            items: Vec::new(),
        };
        let response = bounded_response(ManagerPreflightV1 {
            format: 1,
            checks: std::array::from_fn(|_| oversized.clone()),
        });
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "MGR_PREFLIGHT_OUTPUT_LIMIT");
        assert!(response.get("preflight").is_none());
    }
}
