use gore_authoring::{
    Diagnostic, ProjectJsonError, ProjectV2, ValidationProfile, MAX_PROJECT_JSON_BYTES,
};
use serde_json::{json, Value};

use crate::err;

/// The canonical project is returned as a JSON string inside the response JSON, so the wire
/// representation is larger than the project itself. Keep that expansion bounded independently
/// from the authoring crate's raw-project ceiling.
const MAX_AUTHORING_RESPONSE_JSON_BYTES: usize = 64 * 1024 * 1024;
const MAX_AUTHORING_ERROR_MESSAGE_BYTES: usize = 4 * 1024;

pub(crate) fn project_check(payload: Value) -> Value {
    project_check_with_response_limit(payload, MAX_AUTHORING_RESPONSE_JSON_BYTES)
}

fn project_check_with_response_limit(payload: Value, response_limit: usize) -> Value {
    let project_json = match payload.get("project_json") {
        None | Some(Value::Null) => {
            return err(
                "AUTHORING_PROJECT_REQUIRED",
                "missing 'project_json' string",
            );
        }
        Some(Value::String(project_json)) => project_json,
        Some(_) => {
            return err(
                "AUTHORING_PROJECT_INVALID",
                "'project_json' must be a string",
            );
        }
    };

    let profile = match payload.get("profile") {
        None | Some(Value::Null) => {
            return err(
                "AUTHORING_PROFILE_REQUIRED",
                "missing authoring validation profile",
            );
        }
        Some(Value::String(profile)) => match profile.as_str() {
            "production" => ValidationProfile::Production,
            "experimental" => ValidationProfile::Experimental,
            _ => {
                return err(
                    "AUTHORING_PROFILE_INVALID",
                    "authoring validation profile must be 'production' or 'experimental'",
                );
            }
        },
        Some(_) => {
            return err(
                "AUTHORING_PROFILE_INVALID",
                "authoring validation profile must be 'production' or 'experimental'",
            );
        }
    };

    // Deliberately pass the untouched nested string to ProjectV2. Parsing it into Value first
    // would erase duplicate object keys before the authoring model can reject them.
    let project = match ProjectV2::from_json(project_json) {
        Ok(project) => project,
        Err(ProjectJsonError::InputTooLarge { .. }) => {
            return err(
                "AUTHORING_PROJECT_LIMIT",
                format!("authoring project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
            );
        }
        Err(error @ ProjectJsonError::InvalidJson(_)) => {
            return err(
                "AUTHORING_PROJECT_INVALID",
                bounded_error_message(error.to_string()),
            );
        }
    };

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(_) => {
            return err(
                "AUTHORING_PROJECT_SERIALIZE",
                "canonical authoring project serialization failed",
            );
        }
    };
    let diagnostics = project.validate_with_profile(profile);
    let blocks_build = diagnostics.iter().any(|diagnostic| diagnostic.blocks_build);
    let diagnostics = diagnostics
        .into_iter()
        .map(diagnostic_to_wire)
        .collect::<Vec<_>>();
    let response = json!({
        "ok": true,
        "canonical_project_json": canonical_project_json,
        "diagnostics": diagnostics,
        "blocks_build": blocks_build,
    });

    match serde_json::to_vec(&response) {
        Ok(encoded) if encoded.len() <= response_limit => response,
        Ok(_) => err(
            "AUTHORING_RESPONSE_LIMIT",
            format!("authoring response exceeds the {response_limit}-byte limit"),
        ),
        Err(_) => err(
            "AUTHORING_RESPONSE_SERIALIZE",
            "authoring response serialization failed",
        ),
    }
}

fn bounded_error_message(mut message: String) -> String {
    const TRUNCATED_SUFFIX: &str = "...";
    if message.len() <= MAX_AUTHORING_ERROR_MESSAGE_BYTES {
        return message;
    }

    let mut end = MAX_AUTHORING_ERROR_MESSAGE_BYTES - TRUNCATED_SUFFIX.len();
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message.truncate(end);
    message.push_str(TRUNCATED_SUFFIX);
    message
}

fn diagnostic_to_wire(diagnostic: Diagnostic) -> Value {
    json!({
        "code": diagnostic.code,
        "severity": diagnostic.severity,
        "entity": diagnostic.entity.map(|entity| entity.to_string()),
        "property_path": diagnostic.property_path,
        "message": diagnostic.message,
        "related_entities": diagnostic
            .related_entities
            .into_iter()
            .map(|entity| entity.to_string())
            .collect::<Vec<_>>(),
        "blocks_build": diagnostic.blocks_build,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute_json;

    fn project_json(executable_len: u64) -> String {
        json!({
            "format": 2,
            "schema_revision": 1,
            "project_id": "00000000000000000000000000000001",
            "revision": 0,
            "meta": {
                "name": "Bridge fixture",
                "version": "1.0.0",
                "author": "Test",
            },
            "target": {
                "executable": {
                    "byte_len": executable_len,
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                },
            },
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}},
        })
        .to_string()
    }

    fn payload(project_json: String, profile: Value) -> Value {
        json!({"project_json": project_json, "profile": profile})
    }

    fn error_code(response: &Value) -> &str {
        response["error"]["code"].as_str().unwrap()
    }

    #[test]
    fn command_returns_canonical_project_and_deterministic_structured_diagnostics() {
        let request = json!({
            "command": "authoring_project_check",
            "payload": payload(project_json(0), json!("production")),
        })
        .to_string();

        let first = execute_json(&request);
        let second = execute_json(&request);
        assert_eq!(first, second);

        let response: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["blocks_build"], true);
        assert_eq!(
            response["diagnostics"],
            json!([{
                "code": "INVALID_GENERATION_ANCHOR",
                "severity": "error",
                "entity": null,
                "property_path": "target.executable.byte_len",
                "message": "game generation executable seal must have a non-zero byte length",
                "related_entities": [],
                "blocks_build": true,
            }])
        );

        let canonical = response["canonical_project_json"].as_str().unwrap();
        assert_eq!(
            canonical,
            ProjectV2::from_json(&project_json(0))
                .unwrap()
                .to_canonical_json()
                .unwrap()
        );
        ProjectV2::from_json(canonical).unwrap();
    }

    #[test]
    fn both_closed_validation_profiles_are_accepted() {
        for profile in ["production", "experimental"] {
            let response = project_check(payload(project_json(1), json!(profile)));
            assert_eq!(response["ok"], true);
            assert_eq!(response["diagnostics"], json!([]));
            assert_eq!(response["blocks_build"], false);
        }
    }

    #[test]
    fn missing_and_bad_profiles_have_stable_errors() {
        for missing_profile in [
            json!({"project_json": project_json(1)}),
            payload(project_json(1), Value::Null),
        ] {
            let response = project_check(missing_profile);
            assert_eq!(error_code(&response), "AUTHORING_PROFILE_REQUIRED");
        }

        for profile in [json!("preview"), json!(1)] {
            let response = project_check(payload(project_json(1), profile));
            assert_eq!(error_code(&response), "AUTHORING_PROFILE_INVALID");
        }
    }

    #[test]
    fn malformed_wrong_format_and_duplicate_key_projects_remain_invalid() {
        let valid = project_json(1);
        let projects = [
            "{".to_owned(),
            valid.replacen("\"format\":2", "\"format\":1", 1),
            valid.replacen("\"revision\":0", "\"revision\":0,\"revision\":1", 1),
        ];

        for project in projects {
            let response = project_check(payload(project, json!("production")));
            assert_eq!(error_code(&response), "AUTHORING_PROJECT_INVALID");
        }
    }

    #[test]
    fn project_and_response_limits_have_stable_errors() {
        let oversized = " ".repeat(MAX_PROJECT_JSON_BYTES + 1);
        let response = project_check(payload(oversized, json!("production")));
        assert_eq!(error_code(&response), "AUTHORING_PROJECT_LIMIT");

        let response =
            project_check_with_response_limit(payload(project_json(1), json!("production")), 1);
        assert_eq!(error_code(&response), "AUTHORING_RESPONSE_LIMIT");
    }

    #[test]
    fn invalid_project_error_responses_are_bounded() {
        let unknown_field = "x".repeat(MAX_AUTHORING_ERROR_MESSAGE_BYTES * 2);
        let invalid = project_json(1).replacen(
            "\"revision\":0",
            &format!("\"{unknown_field}\":0,\"revision\":0"),
            1,
        );
        let response = project_check(payload(invalid, json!("production")));

        assert_eq!(error_code(&response), "AUTHORING_PROJECT_INVALID");
        assert!(
            response["error"]["message"].as_str().unwrap().len()
                <= MAX_AUTHORING_ERROR_MESSAGE_BYTES
        );
        assert!(
            bounded_error_message("é".repeat(MAX_AUTHORING_ERROR_MESSAGE_BYTES)).len()
                <= MAX_AUTHORING_ERROR_MESSAGE_BYTES
        );
    }

    #[test]
    fn missing_or_non_string_project_json_is_rejected() {
        let missing = project_check(json!({"profile": "production"}));
        assert_eq!(error_code(&missing), "AUTHORING_PROJECT_REQUIRED");

        let invalid = project_check(json!({"project_json": {}, "profile": "production"}));
        assert_eq!(error_code(&invalid), "AUTHORING_PROJECT_INVALID");
    }
}
