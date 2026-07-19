//! Exact-current, read-only whole-project build planning for managed revision-3 projects.
//!
//! The route accepts only one caller-observed Store checkpoint, fully reopens the Store and its
//! managed DataAsset-stage registry before and after native planning, and retains the real Store
//! directory identity across the complete operation. It creates no artifact and grants no build,
//! deployment, runtime, publication, game-write, save-write, or fixed-head authority.

use std::path::Path;

use gore_authoring::{
    plan_revision3_project_build_v1, AssetVerification, DataAssetStageConflictV1,
    DataAssetStageManifestErrorV1, ProjectRevision3, Revision3DataAssetStageViewV1,
    Revision3DataAssetStagingErrorV1, Revision3ProjectBuildPlanErrorV1, WorkingHead,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::authoring_store_root_guard::{RetainedStoreRoot, RetainedStoreRootError};
use crate::err;

pub(super) const COMMAND: &str = "authoring_store_plan_revision3_project_build_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize =
    MAX_PROJECT_JSON_BYTES * 2 + MAX_HEAD_JSON_BYTES * 2 + MAX_PATH_BYTES * 2 + 4096;
const STORE_ROOT_CHANGED_CODE: &str = "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanProjectBuildWirePayload {
    current_project_json: String,
    expected_head_json: String,
    root: String,
}

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

    fn is_store_root_changed(&self) -> bool {
        self.code == STORE_ROOT_CHANGED_CODE
    }
}

enum InitialPlanning {
    Planned {
        stages: Vec<Revision3DataAssetStageViewV1>,
        response: Result<Value, Failure>,
    },
    StageFailure(Failure),
}

fn finalize_after_store_audits(
    initial: InitialPlanning,
    closing_store: Result<(), Failure>,
    after_stages: Result<Vec<Revision3DataAssetStageViewV1>, Failure>,
) -> Result<Value, Failure> {
    // A foreign mount may be observed by any one of the three independently reopened Store
    // surfaces. Preserve that evidence across all remaining audits before applying the ordinary
    // closing-HEAD and stage-registry precedence below.
    let root_changed = matches!(
        &initial,
        InitialPlanning::StageFailure(error) if error.is_store_root_changed()
    ) || matches!(
        &closing_store,
        Err(error) if error.is_store_root_changed()
    ) || matches!(
        &after_stages,
        Err(error) if error.is_store_root_changed()
    );
    if root_changed {
        return Err(store_root_changed());
    }

    closing_store?;
    match (initial, after_stages) {
        (InitialPlanning::Planned { stages, response }, Ok(after_stages)) => {
            if after_stages != stages {
                return Err(stage_conflict());
            }
            response
        }
        (InitialPlanning::Planned { .. }, Err(error)) => Err(error),
        (InitialPlanning::StageFailure(error), Ok(_)) => Err(error),
        (InitialPlanning::StageFailure(_), Err(error)) => Err(error),
    }
}

pub(super) fn plan_revision3_project_build_v1_raw(input: &str) -> Value {
    plan_revision3_project_build_v1_inner(input).unwrap_or_else(Failure::response)
}

fn plan_revision3_project_build_v1_inner(input: &str) -> Result<Value, Failure> {
    plan_revision3_project_build_v1_inner_with_guard(input, |_| {})
}

fn plan_revision3_project_build_v1_inner_with_guard<F>(
    input: &str,
    after_plan_guard: F,
) -> Result<Value, Failure>
where
    F: FnMut(&Path),
{
    plan_revision3_project_build_v1_inner_with_guard_and_planner(
        input,
        after_plan_guard,
        plan_response,
    )
}

fn plan_revision3_project_build_v1_inner_with_guard_and_planner<F, P>(
    input: &str,
    after_plan_guard: F,
    planner: P,
) -> Result<Value, Failure>
where
    F: FnMut(&Path),
    P: FnMut(
        &ProjectRevision3,
        &[Revision3DataAssetStageViewV1],
        &WorkingHead,
    ) -> Result<Value, Failure>,
{
    plan_revision3_project_build_v1_inner_with_guards_and_planner(
        input,
        |_| {},
        after_plan_guard,
        planner,
    )
}

fn plan_revision3_project_build_v1_inner_with_guards_and_planner<C, F, P>(
    input: &str,
    mut after_capture_guard: C,
    mut after_plan_guard: F,
    mut planner: P,
) -> Result<Value, Failure>
where
    C: FnMut(&Path),
    F: FnMut(&Path),
    P: FnMut(
        &ProjectRevision3,
        &[Revision3DataAssetStageViewV1],
        &WorkingHead,
    ) -> Result<Value, Failure>,
{
    let payload: PlanProjectBuildWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty()
        || payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_LIMIT",
            "current_project_json is empty or exceeds its bounded transport limit",
        ));
    }
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let retained_root =
        RetainedStoreRoot::capture(Path::new(&payload.root)).map_err(map_initial_root_error)?;
    after_capture_guard(retained_root.canonical());

    // Once capture succeeds, no later diagnostic may bypass the closing identity audit. Keeping
    // the complete path-based Store operation inside this deferred result makes root replacement
    // dominate initial-open, exact-head, stage, planner, closing-open, and response failures.
    let operation = (|| {
        let canonical_root = retained_root.canonical();
        let store = retained_root
            .open_existing_store(WorkingStoreLimits::default())
            .map_err(map_store_error)?;

        let basis = store
            .open_current_revision3(AssetVerification::Full)
            .map_err(map_store_error)?;
        if basis.head != expected_head {
            return Err(head_conflict());
        }
        let canonical_project = basis.project.to_canonical_json().map_err(|_| invariant())?;
        if canonical_project != payload.current_project_json {
            return Err(Failure::new(
                "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_PROJECT_CONFLICT",
                "current_project_json differs from the exact published revision-3 project",
            ));
        }

        // Stage/planner/serialization failures are evidence about this exact basis. Retain them
        // until the complete Store and stage-registry windows have been closed so mutable drift
        // wins over a stale diagnostic.
        let initial = match store.list_revision3_dataasset_stages_v1(&expected_head) {
            Ok(stages) => InitialPlanning::Planned {
                response: planner(&basis.project, &stages, &basis.head),
                stages,
            },
            Err(error) => InitialPlanning::StageFailure(map_staging_error(error)),
        };
        after_plan_guard(canonical_root);

        let closing_store = store
            .open_current_revision3(AssetVerification::Full)
            .map_err(map_store_error)
            .and_then(|after| {
                let after_project_json =
                    after.project.to_canonical_json().map_err(|_| invariant())?;
                if after.head != expected_head
                    || after.project != basis.project
                    || after_project_json != payload.current_project_json
                {
                    return Err(head_conflict());
                }
                Ok(())
            });
        let after_stages = store
            .list_revision3_dataasset_stages_v1(&expected_head)
            .map_err(map_staging_error);
        finalize_after_store_audits(initial, closing_store, after_stages)
    })();

    // SecureDirectDirectory retains the accepted path chain. Windows handles prevent rename;
    // Linux additionally latches exact retained-chain changes through nonblocking inotify, even
    // when a transient swap is restored before this closing audit. Other Unix targets fail root
    // capture closed until an equivalent retained-handle change primitive exists.
    retained_root.revalidate().map_err(map_closing_root_error)?;
    operation
}

fn plan_response(
    project: &ProjectRevision3,
    stages: &[Revision3DataAssetStageViewV1],
    head: &WorkingHead,
) -> Result<Value, Failure> {
    let plan = plan_revision3_project_build_v1(project, stages).map_err(map_planner_error)?;
    let plan = serde_json::to_value(plan).map_err(|_| invariant())?;
    let response = json!({
        "ok": true,
        "basis_head_json": canonical_head_json(head)?,
        "plan": plan,
    });
    validate_signed_wire_numbers(&response)?;
    enforce_response_budget(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_LIMIT",
            "revision-3 project build-plan request exceeds its bounded wire limit",
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

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    if canonical_head_json(&head)? != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    serde_json::to_string(head).map_err(|_| invariant())
}

fn validate_signed_wire_numbers(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Array(values) => {
            for value in values {
                validate_signed_wire_numbers(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                validate_signed_wire_numbers(value)?;
            }
        }
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            return Err(Failure::new(
                "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_RESPONSE_LIMIT",
                "revision-3 project build plan contains an integer outside the signed wire range",
            ));
        }
        _ => {}
    }
    Ok(())
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    if serde_json::to_vec(&response)
        .map_err(|_| invariant())?
        .len()
        > MAX_RESPONSE_BYTES
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_RESPONSE_LIMIT",
            "revision-3 project build plan response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn map_initial_root_error(_error: RetainedStoreRootError) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_UNAVAILABLE",
        "managed Store root identity could not be captured safely",
    )
}

fn map_closing_root_error(_error: RetainedStoreRootError) -> Failure {
    store_root_changed()
}

fn store_root_changed() -> Failure {
    Failure::new(
        STORE_ROOT_CHANGED_CODE,
        "the managed Store root changed identity during project build planning",
    )
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    if error.is_read_mount_changed() {
        return map_closing_root_error(RetainedStoreRootError::Changed);
    }
    use WorkingStoreError::*;
    let (code, message) = match error {
        HeadConflict { .. } => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_CONFLICT",
            "the published revision-3 project changed during project build planning",
        ),
        MissingHead(_) | MissingRoot(_) | MissingObject(_) => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_MISSING",
            "the managed Store is missing required exact-current content",
        ),
        UnsafePath { .. } => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_UNSAFE",
            "the managed Store contains an unsafe filesystem path",
        ),
        LimitExceeded { .. } | InvalidLimits(_) => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_LIMIT",
            "the managed Store exceeds a bounded resource limit",
        ),
        SealMismatch { .. } | Collision { .. } => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_INTEGRITY",
            "the managed Store failed exact content-integrity verification",
        ),
        InvalidJson { .. }
        | NonCanonicalJson { .. }
        | Invariant(_)
        | InvalidOgg(_)
        | OggMetadataMismatch { .. } => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_INVALID",
            "the managed Store does not contain one valid closed revision-3 project",
        ),
        StagingCleanup { .. } | Io(_) => (
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_IO",
            "the managed Store could not be read completely",
        ),
    };
    Failure::new(code, message)
}

fn map_staging_error(error: Revision3DataAssetStagingErrorV1) -> Failure {
    match error {
        Revision3DataAssetStagingErrorV1::Store(error) => map_store_error(error),
        Revision3DataAssetStagingErrorV1::Manifest(
            DataAssetStageManifestErrorV1::InputTooLarge { .. },
        )
        | Revision3DataAssetStagingErrorV1::Conflict(
            DataAssetStageConflictV1::StageBatchBudgetExceeded { .. },
        ) => Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STAGE_LIMIT",
            "the managed DataAsset-stage registry exceeds a bounded resource limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STAGE_INVALID",
            "the managed DataAsset-stage registry is not one exact closed project binding",
        ),
    }
}

fn map_planner_error(_error: Revision3ProjectBuildPlanErrorV1) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_PROJECT_INVALID",
        "the exact-current revision-3 project cannot produce a closed read-only build plan",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_CONFLICT",
        "the published revision-3 project changed during project build planning",
    )
}

fn stage_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STAGE_CONFLICT",
        "the managed DataAsset-stage registry changed during project build planning",
    )
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID",
        "request must be exact canonical JSON containing command and exactly current_project_json, expected_head_json, and root",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INVARIANT",
        "revision-3 project build planning could not preserve its exact internal contract",
    )
}

fn truncate_utf8(mut value: String, max: usize) -> String {
    if value.len() <= max {
        return value;
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use gore_authoring::{
        AssetMeta, AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId,
        ProjectMeta, ProjectRevision3, SchemaRevisionV3, Sha256Digest, WorkingHead,
        WorkingProjectStore, WorkingStoreLimits, DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1,
    };
    use serde_json::{json, Value};
    use sha2::{Digest as _, Sha256};

    use super::{
        plan_response, plan_revision3_project_build_v1_inner_with_guard,
        plan_revision3_project_build_v1_inner_with_guard_and_planner,
        plan_revision3_project_build_v1_inner_with_guards_and_planner,
        plan_revision3_project_build_v1_raw, ExactWireRequest, Failure,
        PlanProjectBuildWirePayload, COMMAND, MAX_PROJECT_JSON_BYTES,
    };

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x61; 16]),
            revision,
            meta: ProjectMeta {
                name: "ProjectBuildPlan".into(),
                version: "1.0.0".into(),
                author: "gore-ffi tests".into(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 4096,
                    sha256: digest(0x71),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex {
                assets: BTreeMap::new(),
            },
        }
    }

    fn publish_project(parent: &Path, project: &ProjectRevision3) -> (PathBuf, WorkingHead) {
        let store_root = parent.join("store");
        let store = WorkingProjectStore::at(&store_root, WorkingStoreLimits::default()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, project).unwrap();
        std::fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        (store_root, prepared.head)
    }

    fn request(store_root: &Path, project: &ProjectRevision3, head: &WorkingHead) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PlanProjectBuildWirePayload {
                current_project_json: project.to_canonical_json().unwrap(),
                expected_head_json: serde_json::to_string(head).unwrap(),
                root: store_root.to_str().unwrap().to_owned(),
            },
        })
        .unwrap()
    }

    fn execute(input: &str) -> Value {
        serde_json::from_str(&crate::execute_json(input)).unwrap()
    }

    fn read_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &path, files);
                } else {
                    let relative = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/");
                    files.insert(relative, std::fs::read(path).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    fn copy_tree(source: &Path, destination: &Path) {
        std::fs::create_dir(destination).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&source_path, &destination_path);
            } else {
                std::fs::copy(source_path, destination_path).unwrap();
            }
        }
    }

    fn install_raw_asset(store_root: &Path, bytes: &[u8]) -> Sha256Digest {
        let digest = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
        let hex = digest.to_string();
        let path = store_root
            .join("assets")
            .join("sha256")
            .join(&hex[..2])
            .join(&hex[2..]);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
        digest
    }

    fn assert_no_authority_bearing_response_fields(value: &Value) {
        match value {
            Value::Array(values) => {
                for value in values {
                    assert_no_authority_bearing_response_fields(value);
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    assert!(
                        !matches!(
                            key.as_str(),
                            "game"
                                | "game_root"
                                | "output"
                                | "source"
                                | "path"
                                | "target"
                                | "artifact"
                        ),
                        "authority-bearing response field {key:?} was exposed"
                    );
                    assert_no_authority_bearing_response_fields(value);
                }
            }
            _ => {}
        }
    }

    #[cfg(unix)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn whole_project_plan_is_exact_bounded_read_only_evidence_without_authority() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let before = read_tree(temp.path());

        let response = execute(&request(&store_root, &project, &head));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&head).unwrap()
        );
        let keys: BTreeSet<_> = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, BTreeSet::from(["basis_head_json", "ok", "plan"]));
        let plan = response["plan"].as_object().unwrap();
        let plan_keys: BTreeSet<_> = plan.keys().map(String::as_str).collect();
        assert_eq!(
            plan_keys,
            BTreeSet::from([
                "artifact_status",
                "blockers",
                "build_authority",
                "deployment_status",
                "domains",
                "input_seal",
                "outcome",
                "plan_seal",
                "production_content_count",
                "project_id",
                "project_revision",
                "publication_status",
                "runtime_status",
                "schema_revision",
                "scope",
            ])
        );
        assert_eq!(plan["project_id"], project.project_id.to_string());
        assert_eq!(plan["project_revision"], 7);
        assert_eq!(plan["build_authority"], "not_granted");
        assert_eq!(plan["artifact_status"], "not_created");
        assert_eq!(plan["deployment_status"], "not_performed");
        assert_eq!(plan["runtime_status"], "runtime_unqualified");
        assert_eq!(plan["publication_status"], "not_supported");
        assert_no_authority_bearing_response_fields(&response);
        assert!(!response
            .to_string()
            .contains(&store_root.display().to_string()));
        assert_eq!(read_tree(temp.path()), before);
    }

    #[test]
    fn project_head_and_request_schema_are_exact_and_authority_closed() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let canonical = request(&store_root, &project, &head);

        let mut foreign_project = project.clone();
        foreign_project.revision += 1;
        assert_eq!(
            execute(&request(&store_root, &foreign_project, &head))["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_PROJECT_CONFLICT"
        );

        let mut foreign_head = head.clone();
        foreign_head.snapshot.byte_len += 1;
        let wrong_head = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PlanProjectBuildWirePayload {
                current_project_json: project.to_canonical_json().unwrap(),
                expected_head_json: serde_json::to_string(&foreign_head).unwrap(),
                root: store_root.display().to_string(),
            },
        })
        .unwrap();
        assert_eq!(
            execute(&wrong_head)["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_CONFLICT"
        );

        assert_eq!(
            execute(&format!(" {canonical}"))["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID"
        );
        let mut noncanonical_inner: Value = serde_json::from_str(&canonical).unwrap();
        noncanonical_inner["payload"]["expected_head_json"] =
            json!(format!(" {}", serde_json::to_string(&head).unwrap()));
        assert_eq!(
            execute(&serde_json::to_string(&noncanonical_inner).unwrap())["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_INVALID"
        );
        let mut missing: Value = serde_json::from_str(&canonical).unwrap();
        missing["payload"].as_object_mut().unwrap().remove("root");
        assert_eq!(
            execute(&serde_json::to_string(&missing).unwrap())["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID"
        );
        let mut wrong_type: Value = serde_json::from_str(&canonical).unwrap();
        wrong_type["payload"]["root"] = json!(7);
        assert_eq!(
            execute(&serde_json::to_string(&wrong_type).unwrap())["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID"
        );
        for forbidden in [
            "game_root",
            "output",
            "source",
            "path",
            "target",
            "artifact",
            "build",
            "deploy",
            "runtime",
            "publication",
        ] {
            let mut expanded: Value = serde_json::from_str(&canonical).unwrap();
            expanded["payload"][forbidden] = json!("forbidden");
            assert_eq!(
                execute(&serde_json::to_string(&expanded).unwrap())["error"]["code"],
                "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID",
                "forbidden authority field {forbidden} was accepted"
            );
        }

        let duplicate_command = canonical.replacen(
            "{\"command\":",
            &format!("{{\"command\":\"{COMMAND}\",\"command\":"),
            1,
        );
        assert_eq!(
            execute(&duplicate_command)["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID"
        );
        assert_eq!(
            plan_revision3_project_build_v1_raw(&canonical.replace(COMMAND, "wrong"))["error"]
                ["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_INVALID"
        );
    }

    #[test]
    fn transport_limits_and_signed_response_range_are_closed() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let mut value: Value =
            serde_json::from_str(&request(&store_root, &project, &head)).unwrap();
        value["payload"]["current_project_json"] = json!("");
        assert_eq!(
            execute(&serde_json::to_string(&value).unwrap())["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_LIMIT"
        );
        value["payload"]["current_project_json"] = json!(project.to_canonical_json().unwrap());
        value["payload"]["expected_head_json"] = json!("");
        assert_eq!(
            execute(&serde_json::to_string(&value).unwrap())["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_INVALID"
        );

        let oversized = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PlanProjectBuildWirePayload {
                current_project_json: "x".repeat(MAX_PROJECT_JSON_BYTES + 1),
                expected_head_json: serde_json::to_string(&head).unwrap(),
                root: store_root.display().to_string(),
            },
        })
        .unwrap();
        assert_eq!(
            plan_revision3_project_build_v1_raw(&oversized)["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_INPUT_LIMIT"
        );

        let high_project = empty_project(i64::MAX as u64 + 1);
        let high_temp = tempfile::tempdir().unwrap();
        let (high_root, high_head) = publish_project(high_temp.path(), &high_project);
        assert_eq!(
            execute(&request(&high_root, &high_project, &high_head))["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_RESPONSE_LIMIT"
        );
    }

    #[test]
    fn traversal_and_linked_store_roots_are_rejected_without_following() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let traversing = store_root.join("..").join("store");
        assert_eq!(
            execute(&request(&traversing, &project, &head))["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_UNAVAILABLE"
        );

        let alias_parent = tempfile::tempdir().unwrap();
        let alias = alias_parent.path().join("store-alias");
        if !make_test_dir_link(&store_root, &alias) {
            return;
        }
        let before = read_tree(&store_root);
        assert_eq!(
            execute(&request(&alias, &project, &head))["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_UNAVAILABLE"
        );
        assert_eq!(read_tree(&store_root), before);
        #[cfg(unix)]
        std::fs::remove_file(alias).unwrap();
        #[cfg(windows)]
        std::fs::remove_dir(alias).unwrap();
    }

    #[test]
    fn root_replacement_before_initial_store_open_dominates_the_early_store_error() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-before-open-store");
        let swapped = Cell::new(false);

        let outcome = plan_revision3_project_build_v1_inner_with_guards_and_planner(
            &input,
            |root| {
                // Windows retains every ancestor without delete sharing. If the rename is denied,
                // the platform has prevented the race rather than failed the assertion.
                if std::fs::rename(root, &displaced).is_err() {
                    return;
                }
                std::fs::create_dir(root).unwrap();
                swapped.set(true);
            },
            |_| {},
            plan_response,
        );
        if !swapped.get() {
            return;
        }

        assert_eq!(
            outcome.unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn transient_root_swap_and_restore_is_latched_as_root_changed() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-transient-store");
        let replacement = temp.path().join("replacement-transient-store");
        std::fs::create_dir(&replacement).unwrap();

        let outcome = plan_revision3_project_build_v1_inner_with_guard(&input, |root| {
            std::fs::rename(root, &displaced).unwrap();
            std::os::unix::fs::symlink(&replacement, root).unwrap();
            std::fs::remove_file(root).unwrap();
            std::fs::rename(&displaced, root).unwrap();
        });

        assert_eq!(
            outcome.unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn mount_binding_failure_maps_to_store_root_changed() {
        use gore_authoring::WorkingStoreLinuxMountId;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        std::fs::create_dir(&root).unwrap();
        let root_mount =
            WorkingStoreLinuxMountId::from_open_file(&std::fs::File::open(&root).unwrap()).unwrap();
        let foreign_mount =
            WorkingStoreLinuxMountId::from_open_file(&std::fs::File::open("/proc").unwrap())
                .unwrap();
        if root_mount == foreign_mount {
            return;
        }
        let error = WorkingProjectStore::open_existing_read_only_on_mount(
            &root,
            WorkingStoreLimits::default(),
            foreign_mount,
        )
        .unwrap_err();
        assert_eq!(
            super::map_store_error(error).code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[test]
    fn ordinary_closing_head_precedence_is_unchanged_without_a_root_change() {
        let failure = super::finalize_after_store_audits(
            super::InitialPlanning::StageFailure(super::stage_conflict()),
            Err(super::head_conflict()),
            Err(super::stage_conflict()),
        )
        .unwrap_err();

        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_CONFLICT"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detected_mount_change_dominates_head_and_stage_failures_from_every_audit() {
        use gore_authoring::WorkingStoreLinuxMountId;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        std::fs::create_dir(&root).unwrap();
        let root_mount =
            WorkingStoreLinuxMountId::from_open_file(&std::fs::File::open(&root).unwrap()).unwrap();
        let foreign_mount =
            WorkingStoreLinuxMountId::from_open_file(&std::fs::File::open("/proc").unwrap())
                .unwrap();
        if root_mount == foreign_mount {
            return;
        }
        let mount_failure = || {
            let error = WorkingProjectStore::open_existing_read_only_on_mount(
                &root,
                WorkingStoreLimits::default(),
                foreign_mount,
            )
            .unwrap_err();
            super::map_store_error(error)
        };

        let outcomes = [
            // Initial stage inspection observed the foreign mount; a later HEAD and stage error
            // must not erase that stronger evidence.
            super::finalize_after_store_audits(
                super::InitialPlanning::StageFailure(mount_failure()),
                Err(super::head_conflict()),
                Err(super::stage_conflict()),
            ),
            // The closing Store reopen observed it while both stage inspections were otherwise
            // invalid.
            super::finalize_after_store_audits(
                super::InitialPlanning::StageFailure(super::stage_conflict()),
                Err(mount_failure()),
                Err(super::stage_conflict()),
            ),
            // The closing stage inspection observed it concurrently with an ordinary closing
            // HEAD conflict and an earlier stage failure.
            super::finalize_after_store_audits(
                super::InitialPlanning::StageFailure(super::stage_conflict()),
                Err(super::head_conflict()),
                Err(mount_failure()),
            ),
        ];

        for outcome in outcomes {
            assert_eq!(
                outcome.unwrap_err().code,
                "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
            );
        }
    }

    #[test]
    fn later_head_publication_wins_over_the_planned_response() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let input = request(&store_root, &project, &head);
        let mut later = project.clone();
        later.revision += 1;
        later.meta.version = "later".into();

        let failure = plan_revision3_project_build_v1_inner_with_guard(&input, |root| {
            let store =
                WorkingProjectStore::open_existing(root, WorkingStoreLimits::default()).unwrap();
            let prepared = store
                .prepare_revision3_checkpoint(Some(&head), &later)
                .unwrap();
            std::fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
        })
        .unwrap_err();

        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_HEAD_CONFLICT"
        );
    }

    #[test]
    fn byte_identical_same_path_root_replacement_beats_a_deferred_planner_error() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-store");
        let swapped = Cell::new(false);

        let planner_failure = |_project: &ProjectRevision3,
                               _stages: &[gore_authoring::Revision3DataAssetStageViewV1],
                               _head: &WorkingHead| {
            Err(Failure::new(
                "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_PROJECT_INVALID",
                "injected planner failure",
            ))
        };
        assert_eq!(
            plan_revision3_project_build_v1_inner_with_guard_and_planner(
                &input,
                |_| {},
                planner_failure,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_PROJECT_INVALID"
        );

        let outcome = plan_revision3_project_build_v1_inner_with_guard_and_planner(
            &input,
            |root| {
                if std::fs::rename(root, &displaced).is_err() {
                    return;
                }
                copy_tree(&displaced, root);
                swapped.set(true);
            },
            planner_failure,
        );
        if !swapped.get() {
            return;
        }

        assert_eq!(
            outcome.unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[test]
    fn root_replacement_wins_even_when_the_replacement_publishes_another_head() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-head-store");
        let swapped = Cell::new(false);
        let mut later = project.clone();
        later.revision += 1;
        later.meta.version = "replacement-head".into();

        let outcome = plan_revision3_project_build_v1_inner_with_guard(&input, |root| {
            if std::fs::rename(root, &displaced).is_err() {
                return;
            }
            copy_tree(&displaced, root);
            let replacement =
                WorkingProjectStore::open_existing(root, WorkingStoreLimits::default()).unwrap();
            let prepared = replacement
                .prepare_revision3_checkpoint(Some(&head), &later)
                .unwrap();
            std::fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
            swapped.set(true);
        });
        if !swapped.get() {
            return;
        }

        assert_eq!(
            outcome.unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[test]
    fn root_replacement_wins_even_when_the_replacement_store_is_corrupt() {
        let temp = tempfile::tempdir().unwrap();
        let project = empty_project(7);
        let (store_root, head) = publish_project(temp.path(), &project);
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-corrupt-store");
        let swapped = Cell::new(false);

        let outcome = plan_revision3_project_build_v1_inner_with_guard(&input, |root| {
            if std::fs::rename(root, &displaced).is_err() {
                return;
            }
            copy_tree(&displaced, root);
            std::fs::write(root.join("gore-project.json"), b"not canonical head JSON").unwrap();
            swapped.set(true);
        });
        if !swapped.get() {
            return;
        }

        assert_eq!(
            outcome.unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[test]
    fn invalid_stage_registry_is_sanitized_read_only_and_still_root_audited() {
        let temp = tempfile::tempdir().unwrap();
        let store_root = temp.path().join("store");
        let store = WorkingProjectStore::at(&store_root, WorkingStoreLimits::default()).unwrap();
        let invalid_manifest = br#"{"private_path":"C:/private/stage.json"}"#;
        let manifest_digest = install_raw_asset(&store_root, invalid_manifest);
        let mut project = empty_project(7);
        project.asset_store.assets.insert(
            manifest_digest,
            AssetMeta {
                byte_len: invalid_manifest.len() as u64,
                media_type: DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1.into(),
            },
        );
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        std::fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        let input = request(&store_root, &project, &prepared.head);
        let before = read_tree(temp.path());

        let response = execute(&input);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STAGE_INVALID"
        );
        assert!(!response.to_string().contains("private"));
        assert!(!response
            .to_string()
            .contains(&store_root.display().to_string()));
        assert_eq!(read_tree(temp.path()), before);

        let displaced = temp.path().join("displaced-invalid-stage-store");
        let swapped = Cell::new(false);
        let outcome = plan_revision3_project_build_v1_inner_with_guard(&input, |root| {
            if std::fs::rename(root, &displaced).is_err() {
                return;
            }
            copy_tree(&displaced, root);
            swapped.set(true);
        });
        if !swapped.get() {
            return;
        }
        assert_eq!(
            outcome.unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_BUILD_PLAN_STORE_ROOT_CHANGED"
        );
    }
}
