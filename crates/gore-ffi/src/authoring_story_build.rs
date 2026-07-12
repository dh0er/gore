//! Bounded read-only bridge for deterministic, permanently blocked Story source plans.

use gore_authoring::ValidationProfile;
use gore_story_build::{
    plan_story_build, StoryBuildError, StoryBuildPlan, StoryBuildPublicationStatus,
    MAX_STORY_BUILD_PROJECT_JSON_BYTES,
};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

const MAX_RESPONSE_BYTES: usize = 60 * 1024 * 1024;
const REQUEST_BINDING_DOMAIN: &[u8] = b"gore-story-build.authoring-plan-v1.request-binding\0";

pub(super) fn generate_story_build_plan_v1(payload: Value) -> Value {
    generate_story_build_plan_v1_with_limit(&payload, MAX_RESPONSE_BYTES)
        .unwrap_or_else(|error| error)
}

fn generate_story_build_plan_v1_with_limit(
    payload: &Value,
    response_limit: usize,
) -> Result<Value, Value> {
    let object = exact_payload(payload)?;
    let project_json = bounded_project_json(object)?;
    let (profile, profile_wire) = closed_profile(object)?;
    let request_binding_sha256 = request_binding(project_json, profile_wire);

    // The planner is the sole project trust transition. It rejects duplicate/noncanonical bytes,
    // requires schema revision 2, regenerates sources, and always returns a blocked inspection
    // plan. No caller-supplied qualification or catalog capability exists in this request.
    let plan = plan_story_build(project_json, profile).map_err(map_plan_error)?;
    if plan.publication_status != StoryBuildPublicationStatus::NotSupported || !plan.blocks_build {
        return Err(build_failed());
    }
    let plan_json = plan.to_canonical_json().map_err(map_plan_error)?;
    let reopened = StoryBuildPlan::from_json(&plan_json).map_err(map_plan_error)?;
    if reopened != plan {
        return Err(build_failed());
    }
    plan.verify_against_project_json(project_json)
        .map_err(map_plan_error)?;
    let plan_seal = plan.content_seal().map_err(map_plan_error)?;
    let blocking_diagnostic_indexes = plan
        .diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| diagnostic.blocks_build.then_some(index))
        .collect::<Vec<_>>();
    if blocking_diagnostic_indexes.is_empty() {
        return Err(build_failed());
    }

    let response = json!({
        "ok": true,
        "request_binding_sha256": request_binding_sha256,
        "plan_json": plan_json,
        "plan_seal": plan_seal,
        "validation_profile": plan.validation_profile,
        "project": plan.project,
        "runtime_qualification": "runtime_unqualified",
        "publication_status": plan.publication_status,
        "module_count": plan.modules.len(),
        "diagnostic_count": plan.diagnostics.len(),
        "blocking_diagnostic_indexes": blocking_diagnostic_indexes,
        "blocks_build": plan.blocks_build,
    });
    let encoded = serde_json::to_vec(&response).map_err(|_| build_failed())?;
    if encoded.len() > response_limit {
        return Err(err(
            "AUTHORING_STORY_BUILD_PLAN_RESPONSE_LIMIT",
            "Story build-plan response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn exact_payload(payload: &Value) -> Result<&Map<String, Value>, Value> {
    let Some(object) = payload.as_object() else {
        return Err(invalid_request());
    };
    if object.len() != 2 || !object.contains_key("project_json") || !object.contains_key("profile")
    {
        return Err(invalid_request());
    }
    Ok(object)
}

fn bounded_project_json(object: &Map<String, Value>) -> Result<&str, Value> {
    let project_json = object
        .get("project_json")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid_request)?;
    if project_json.len() > MAX_STORY_BUILD_PROJECT_JSON_BYTES {
        return Err(err(
            "AUTHORING_STORY_BUILD_PLAN_PROJECT_LIMIT",
            "Story build-plan project exceeds its bounded input budget",
        ));
    }
    Ok(project_json)
}

fn closed_profile(object: &Map<String, Value>) -> Result<(ValidationProfile, &str), Value> {
    match object.get("profile").and_then(Value::as_str) {
        Some("production") => Ok((ValidationProfile::Production, "production")),
        Some("experimental") => Ok((ValidationProfile::Experimental, "experimental")),
        _ => Err(invalid_request()),
    }
}

fn invalid_request() -> Value {
    err(
        "AUTHORING_STORY_BUILD_PLAN_REQUEST_INVALID",
        "payload must contain exactly canonical project_json and a closed validation profile",
    )
}

fn request_binding(project_json: &str, profile: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_BINDING_DOMAIN);
    for value in [project_json.as_bytes(), profile.as_bytes()] {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value);
    }
    let digest: [u8; 32] = hasher.finalize().into();
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn map_plan_error(error: StoryBuildError) -> Value {
    match error {
        StoryBuildError::ProjectJsonTooLarge { .. }
        | StoryBuildError::PlanJsonTooLarge { .. }
        | StoryBuildError::TooManyModules { .. }
        | StoryBuildError::SourceBytesOverflow
        | StoryBuildError::SourceBytesTooLarge { .. }
        | StoryBuildError::TooManyDiagnostics { .. } => err(
            "AUTHORING_STORY_BUILD_PLAN_LIMIT",
            "Story build planning exceeded a supported resource limit",
        ),
        StoryBuildError::InvalidProjectDocument(_)
        | StoryBuildError::NonCanonicalProjectJson
        | StoryBuildError::Revision2Required => err(
            "AUTHORING_STORY_BUILD_PLAN_PROJECT_INVALID",
            "project_json is not one exact canonical schema-revision-2 authoring project",
        ),
        _ => build_failed(),
    }
}

fn build_failed() -> Value {
    err(
        "AUTHORING_STORY_BUILD_PLAN_FAILED",
        "the deterministic blocked Story build plan could not be produced",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_authoring::ProjectDocument;

    fn project_json(schema_revision: u32) -> String {
        let raw = json!({
            "format": 2,
            "schema_revision": schema_revision,
            "project_id": "01010101010101010101010101010101",
            "revision": 7,
            "meta": {"name": "Story plan", "version": "0.1", "author": "tests"},
            "target": {"executable": {
                "byte_len": 1_000_000,
                "sha256": "0101010101010101010101010101010101010101010101010101010101010101"
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        })
        .to_string();
        ProjectDocument::from_json(&raw)
            .unwrap()
            .to_canonical_json()
            .unwrap()
    }

    fn payload(project_json: &str, profile: &str) -> Value {
        json!({"project_json": project_json, "profile": profile})
    }

    #[test]
    fn canonical_request_returns_deterministic_reopenable_blocked_plan() {
        let project_json = project_json(2);
        let payload = payload(&project_json, "production");
        let first = generate_story_build_plan_v1_with_limit(&payload, MAX_RESPONSE_BYTES).unwrap();
        let second = generate_story_build_plan_v1_with_limit(&payload, MAX_RESPONSE_BYTES).unwrap();
        assert_eq!(first, second);
        assert_eq!(first["ok"], true);
        assert_eq!(first["runtime_qualification"], "runtime_unqualified");
        assert_eq!(first["publication_status"], "not_supported");
        assert_eq!(first["blocks_build"], true);
        assert_eq!(first["module_count"], 0);
        assert_eq!(first["diagnostic_count"], 1);
        assert_eq!(first["blocking_diagnostic_indexes"], json!([0]));

        let raw_plan = first["plan_json"].as_str().unwrap();
        let plan = StoryBuildPlan::from_json(raw_plan).unwrap();
        plan.verify_against_project_json(&project_json).unwrap();
        assert_eq!(
            plan.publication_status,
            StoryBuildPublicationStatus::NotSupported
        );
        assert!(plan.blocks_build);
        assert_eq!(
            first["plan_seal"]["sha256"],
            format!("{:x}", Sha256::digest(raw_plan.as_bytes()))
        );

        let request = serde_json::to_string(&json!({
            "command": "authoring_story_build_plan_v1_generate",
            "payload": payload,
        }))
        .unwrap();
        let dispatched: Value = serde_json::from_str(&crate::execute_json(&request)).unwrap();
        assert_eq!(dispatched, first);
    }

    #[test]
    fn profiles_change_binding_but_never_unblock_or_publish() {
        let project_json = project_json(2);
        let production = generate_story_build_plan_v1_with_limit(
            &payload(&project_json, "production"),
            MAX_RESPONSE_BYTES,
        )
        .unwrap();
        let experimental = generate_story_build_plan_v1_with_limit(
            &payload(&project_json, "experimental"),
            MAX_RESPONSE_BYTES,
        )
        .unwrap();
        assert_ne!(
            production["request_binding_sha256"],
            experimental["request_binding_sha256"]
        );
        for response in [production, experimental] {
            assert_eq!(response["blocks_build"], true);
            assert_eq!(response["publication_status"], "not_supported");
            assert_eq!(response["runtime_qualification"], "runtime_unqualified");
        }
    }

    #[test]
    fn request_shape_limits_and_caller_qualification_fail_closed() {
        let project_json = project_json(2);
        for invalid in [
            Value::Null,
            json!({}),
            json!({"project_json": project_json, "profile": "production", "qualified": true}),
            json!({"project_json": 1, "profile": "production"}),
            json!({"project_json": "", "profile": "production"}),
            json!({"project_json": project_json, "profile": "preview"}),
            json!({"project_json": project_json, "profile": 1}),
        ] {
            assert_eq!(
                generate_story_build_plan_v1(invalid)["error"]["code"],
                "AUTHORING_STORY_BUILD_PLAN_REQUEST_INVALID"
            );
        }
        let oversized = " ".repeat(MAX_STORY_BUILD_PROJECT_JSON_BYTES + 1);
        assert_eq!(
            generate_story_build_plan_v1(payload(&oversized, "production"))["error"]["code"],
            "AUTHORING_STORY_BUILD_PLAN_PROJECT_LIMIT"
        );
    }

    #[test]
    fn duplicate_noncanonical_revision1_and_secret_errors_are_sanitized() {
        let canonical = project_json(2);
        let duplicate = canonical.replacen("\"revision\":7", "\"revision\":7,\"revision\":7", 1);
        for invalid in [format!(" {canonical}"), duplicate, project_json(1)] {
            let response = generate_story_build_plan_v1(payload(&invalid, "production"));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_STORY_BUILD_PLAN_PROJECT_INVALID"
            );
            assert!(!response.to_string().contains("0101010101010101"));
        }
    }

    #[test]
    fn response_limit_is_checked_after_canonical_plan_construction() {
        let project_json = project_json(2);
        let response =
            generate_story_build_plan_v1_with_limit(&payload(&project_json, "production"), 1)
                .unwrap_err();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_STORY_BUILD_PLAN_RESPONSE_LIMIT"
        );
    }
}
