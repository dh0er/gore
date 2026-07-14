//! Exact-basis, build-only lowering for managed revision-3 Voice content.
//!
//! The command reads every selected Ogg through the verified managed Store boundary, assembles a
//! sealed existing-member bundle from owned bytes, writes and verifies it in an owned sibling
//! staging directory, and atomically promotes it without replacing a racing final target. The
//! receipt remains bound to the fully verified project/head snapshot observed after the last Store
//! asset read; a later independent head publication does not reinterpret those immutable bytes as
//! a different build. It never deploys or writes the game.

use std::fs;
use std::path::{Component, Path, PathBuf};

use gore_asset::dataasset_workflow::{read_verified_file_bounded, MAX_GAME_EXECUTABLE_BYTES};
use gore_authoring::model_revision3::VoiceMemberProof as AuthoringVoiceMemberProof;
use gore_authoring::{
    plan_revision3_voice_build_v1, AssetVerification, Revision3VoiceBuildPlanEvaluationV1,
    Revision3VoiceBuildPlanV1, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_mod::{
    build_sealed_voice_bundle, seal_voice_bundle_disk_tree, semantic_install_root,
    verify_sealed_voice_bundle, write_voice_bundle_staged_new, ModMeta, SealedVoiceArchiveReplace,
    StagedVoiceBundle, VoiceArchiveObservation, VoiceBundleStagingError,
    VoiceBundleStagingErrorKind, VoiceExecutableGenerationSeal, VoiceMemberProof,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_build_revision3_voice_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize =
    MAX_PROJECT_JSON_BYTES * 2 + MAX_HEAD_JSON_BYTES * 2 + MAX_PATH_BYTES * 6 + 4096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildVoiceWirePayload {
    current_project_json: String,
    expected_head_json: String,
    game_root: String,
    output: String,
    root: String,
}

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BuildRootGuard {
    store: PathBuf,
    game: PathBuf,
    output_parent: PathBuf,
    output_target: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildOutputPhase {
    BeforeStagingWrite,
    AfterStagingWrite,
    BeforePromotion,
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

pub(super) fn build_revision3_voice_v1_raw(input: &str) -> Value {
    build_revision3_voice_v1_inner(input).unwrap_or_else(Failure::response)
}

fn build_revision3_voice_v1_inner(input: &str) -> Result<Value, Failure> {
    build_revision3_voice_v1_inner_with_output_guard(input, |_, _| {})
}

fn build_revision3_voice_v1_inner_with_output_guard<F>(
    input: &str,
    mut output_guard: F,
) -> Result<Value, Failure>
where
    F: FnMut(BuildOutputPhase, &Path),
{
    let payload: BuildVoiceWirePayload = parse_exact_wire(input)?;
    for (value, label) in [
        (&payload.root, "managed Store root"),
        (&payload.game_root, "game installation root"),
        (&payload.output, "voice bundle output"),
    ] {
        validate_path(value, label)?;
    }
    if payload.current_project_json.is_empty()
        || payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES
    {
        return Err(invalid_request());
    }
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let root_guard = ensure_build_roots_are_disjoint(
        Path::new(&payload.root),
        Path::new(&payload.game_root),
        Path::new(&payload.output),
    )?;

    // From this point on, use only the canonical paths captured by the root guard. The caller's
    // spellings may be relative aliases and are retained solely for a later identity recheck.
    let store = WorkingProjectStore::open_existing(&root_guard.store, ffi_store_limits())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if basis.head != expected_head {
        return Err(head_conflict());
    }
    let canonical_basis = basis.project.to_canonical_json().map_err(|_| invariant())?;
    if canonical_basis != payload.current_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    validate_signed_wire_values(&basis.project)?;
    require_installed_executable_generation(&root_guard.game, &basis.project.target)?;

    let evaluation = plan_revision3_voice_build_v1(&basis.project).map_err(|error| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_PROJECT_INVALID",
            error.to_string(),
        )
    })?;
    let plan = match evaluation {
        Revision3VoiceBuildPlanEvaluationV1::Blocked { report } => {
            let after_plan = store
                .open_current_revision3(AssetVerification::Full)
                .map_err(map_store_error)?;
            if after_plan.head != basis.head || after_plan.project != basis.project {
                return Err(head_conflict());
            }
            let head_json = canonical_head_json(&basis.head)?;
            return enforce_response_budget(json!({
                "ok": true,
                "outcome": "blocked",
                "basis_head_json": head_json,
                "project_id": basis.project.project_id.to_string(),
                "project_revision": basis.project.revision,
                "report": report,
                "build_authority": "not_granted",
                "deployment_status": "not_performed",
            }));
        }
        Revision3VoiceBuildPlanEvaluationV1::Ready { plan } => plan,
    };

    let edits = read_plan_edits(&store, &plan)?;
    // Close the mutable Store-read window before creating any external output.
    let after_read = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after_read.head != basis.head || after_read.project != basis.project {
        return Err(head_conflict());
    }
    require_installed_executable_generation(&root_guard.game, &basis.project.target)?;
    revalidate_build_root_guard(
        &root_guard,
        Path::new(&payload.root),
        Path::new(&payload.game_root),
        Path::new(&payload.output),
    )?;

    let bundle = build_sealed_voice_bundle(
        ModMeta {
            name: plan.meta.name.clone(),
            version: plan.meta.version.clone(),
            author: plan.meta.author.clone(),
        },
        VoiceExecutableGenerationSeal {
            byte_len: basis.project.target.executable.byte_len,
            sha256: basis.project.target.executable.sha256.to_string(),
        },
        edits,
    )
    .map_err(map_bundle_error)?;
    let (bundle_bytes, bundle_sha256) = bundle_content_seal(&bundle.files)?;
    // Construct and bound the complete success receipt before the atomic commit. Once staging is
    // promoted, this function must have no remaining fallible work that could report failure while
    // leaving a valid final output behind.
    let head_json = canonical_head_json(&basis.head)?;
    let receipt = enforce_response_budget(json!({
        "ok": true,
        "outcome": "built",
        "basis_head_json": head_json,
        "project_id": plan.project_id.to_string(),
        "project_revision": plan.project_revision,
        // Preserve the exact caller-bound output spelling in the receipt. All filesystem work
        // below uses the canonical guarded target; the Dart boundary also verifies this field
        // byte-for-byte against the request and must not reinterpret it as another build.
        "output": payload.output.clone(),
        "edit_count": plan.edits.len(),
        "file_count": bundle.files.len(),
        "bundle_bytes": bundle_bytes,
        "bundle_sha256": bundle_sha256,
        "build_authority": "generation_sealed_existing_member_bundle_v1",
        "deployment_status": "not_performed",
    }))?;
    // Bundle lowering can validate and hash a large, bounded set of Ogg bytes. Re-authenticate the
    // game generation and all three roots immediately around the only external write boundary.
    output_guard(
        BuildOutputPhase::BeforeStagingWrite,
        &root_guard.output_target,
    );
    require_installed_executable_generation(&root_guard.game, &basis.project.target)?;
    revalidate_build_root_guard(
        &root_guard,
        Path::new(&payload.root),
        Path::new(&payload.game_root),
        Path::new(&payload.output),
    )?;
    let staging = write_voice_bundle_staged_new(&root_guard.output_target, &bundle)
        .map_err(map_bundle_write_error)?;
    output_guard(BuildOutputPhase::AfterStagingWrite, staging.path());
    let verified = (|| -> Result<(), Failure> {
        verify_sealed_voice_bundle(staging.path()).map_err(map_bundle_verify_error)?;
        let disk_seal =
            seal_voice_bundle_disk_tree(staging.path()).map_err(map_bundle_verify_error)?;
        if disk_seal.byte_len != bundle_bytes || disk_seal.sha256 != bundle_sha256 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED",
                "the verified on-disk Voice bundle differs from its exact in-memory bundle",
            ));
        }
        Ok(())
    })();
    if let Err(primary) = verified {
        return Err(abort_staged_voice_bundle(staging, primary));
    }

    output_guard(BuildOutputPhase::BeforePromotion, staging.path());
    let still_authorized = (|| -> Result<(), Failure> {
        require_installed_executable_generation(&root_guard.game, &basis.project.target)?;
        revalidate_build_root_guard(
            &root_guard,
            Path::new(&payload.root),
            Path::new(&payload.game_root),
            Path::new(&payload.output),
        )?;
        Ok(())
    })();
    if let Err(primary) = still_authorized {
        return Err(abort_staged_voice_bundle(staging, primary));
    }

    staging.promote_new().map_err(map_bundle_promotion_error)?;
    Ok(receipt)
}

fn read_plan_edits(
    store: &WorkingProjectStore,
    plan: &Revision3VoiceBuildPlanV1,
) -> Result<Vec<SealedVoiceArchiveReplace>, Failure> {
    let mut edits = Vec::with_capacity(plan.edits.len());
    for edit in &plan.edits {
        let ogg = store
            .read_verified_ogg_asset(&edit.asset)
            .map_err(map_store_error)?;
        let member_proof = match &edit.target.member_proof {
            AuthoringVoiceMemberProof::Present {
                uncompressed_size,
                crc32,
            } => VoiceMemberProof::Present {
                uncompressed_size: *uncompressed_size,
                crc32: *crc32,
            },
            AuthoringVoiceMemberProof::Absent => {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_VOICE_BUILD_PROJECT_INVALID",
                    "managed Voice build plan contains an additive member proof",
                ));
            }
        };
        edits.push(SealedVoiceArchiveReplace {
            archive: edit.target.archive.clone(),
            archive_path: edit.target.member.clone(),
            ogg,
            observation: VoiceArchiveObservation {
                archive_size: edit.target.archive_seal.byte_len,
                archive_sha256: edit.target.archive_seal.sha256.to_string(),
                member_proof,
            },
        });
    }
    Ok(edits)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_INPUT_LIMIT",
            "revision-3 Voice build request exceeds its bounded wire limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invariant())?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_path(path: &str, label: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_INPUT_INVALID",
            format!("{label} is empty, unsafe, or exceeds its bounded path limit"),
        ));
    }
    Ok(())
}

fn ensure_build_roots_are_disjoint(
    store_root: &Path,
    game_root: &Path,
    output: &Path,
) -> Result<BuildRootGuard, Failure> {
    let canonical_store = canonical_existing_directory_no_reparse(
        store_root,
        "AUTHORING_REVISION3_VOICE_BUILD_STORE_UNAVAILABLE",
        "managed Store root",
    )
    .map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_UNAVAILABLE",
            "managed Store root could not be resolved safely",
        )
    })?;
    let canonical_game = canonical_existing_directory_no_reparse(
        &semantic_install_root(game_root),
        "AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE",
        "game installation root",
    )
    .map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE",
            "game installation root could not be resolved safely",
        )
    })?;
    if canonical_store.starts_with(&canonical_game) || canonical_game.starts_with(&canonical_store)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_GAME_ALIAS",
            "game installation and managed Store roots must be disjoint",
        ));
    }
    let parent = output
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = canonical_existing_directory_no_reparse(
        parent,
        "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE",
        "voice bundle output parent",
    )
    .map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE",
            "voice bundle output parent must already exist without symbolic-link or reparse traversal",
        )
    })?;
    let name = output.file_name().ok_or_else(invalid_request)?;
    let canonical_target: PathBuf = canonical_parent.join(name);
    if canonical_target.starts_with(&canonical_store)
        || canonical_store.starts_with(&canonical_target)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_OUTPUT_ALIAS",
            "voice bundle output and managed Store roots must be disjoint",
        ));
    }
    if canonical_target.starts_with(&canonical_game)
        || canonical_game.starts_with(&canonical_target)
        || has_recognizable_game_layout_ancestor(&canonical_target)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS",
            "voice bundle output and game installation roots must be disjoint",
        ));
    }
    Ok(BuildRootGuard {
        store: canonical_store,
        game: canonical_game,
        output_parent: canonical_parent,
        output_target: canonical_target,
    })
}

fn has_recognizable_game_layout_ancestor(path: &Path) -> bool {
    let start = path.parent().unwrap_or(path);
    start.ancestors().any(|ancestor| {
        let direct_g1r = ancestor
            .file_name()
            .is_some_and(|name| name.as_encoded_bytes().eq_ignore_ascii_case(b"G1R"));
        let executable = if direct_g1r {
            ancestor
                .join("Binaries")
                .join("Win64")
                .join("G1R-Win64-Shipping.exe")
        } else {
            ancestor
                .join("G1R")
                .join("Binaries")
                .join("Win64")
                .join("G1R-Win64-Shipping.exe")
        };
        fs::symlink_metadata(executable).is_ok()
    })
}

fn revalidate_build_root_guard(
    expected: &BuildRootGuard,
    store_root: &Path,
    game_root: &Path,
    output: &Path,
) -> Result<(), Failure> {
    let actual = ensure_build_roots_are_disjoint(store_root, game_root, output)
        .map_err(map_root_revalidation_failure)?;
    require_same_build_root_guard(expected, &actual)
}

fn map_root_revalidation_failure(failure: Failure) -> Failure {
    match failure.code {
        "AUTHORING_REVISION3_VOICE_BUILD_STORE_UNAVAILABLE" => Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED",
            "the managed Store root became unavailable or changed identity during Voice build",
        ),
        "AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE" => Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED",
            "the game installation root became unavailable or changed identity during Voice build",
        ),
        "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE" => Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED",
            "the Voice output parent became unavailable or changed identity during Voice build",
        ),
        _ => failure,
    }
}

fn require_same_build_root_guard(
    expected: &BuildRootGuard,
    actual: &BuildRootGuard,
) -> Result<(), Failure> {
    if actual.store != expected.store {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED",
            "the managed Store root changed identity during Voice build",
        ));
    }
    if actual.game != expected.game {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED",
            "the game installation root changed identity during Voice build",
        ));
    }
    if actual.output_parent != expected.output_parent
        || actual.output_target != expected.output_target
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED",
            "the Voice output parent or target spelling changed identity during Voice build",
        ));
    }
    Ok(())
}

fn canonical_existing_directory_no_reparse(
    path: &Path,
    code: &'static str,
    label: &str,
) -> Result<PathBuf, Failure> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(Failure::new(
            code,
            format!("{label} must not contain '..' traversal"),
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| Failure::new(code, format!("{label} could not be resolved")))?
            .join(path)
    };
    for ancestor in absolute.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| {
            Failure::new(code, format!("{label} has an unavailable path component"))
        })?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            return Err(Failure::new(
                code,
                format!("{label} crosses a symbolic link, reparse point, or non-directory"),
            ));
        }
    }
    fs::canonicalize(&absolute)
        .map_err(|_| Failure::new(code, format!("{label} could not be canonicalized")))
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn require_installed_executable_generation(
    install_root: &Path,
    expected: &gore_authoring::GameGenerationAnchor,
) -> Result<(), Failure> {
    let executable = install_root
        .join("G1R")
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let verified = read_verified_file_bounded(
        &executable,
        MAX_GAME_EXECUTABLE_BYTES,
        "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE",
    )
    .map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_UNAVAILABLE",
            "the installed game executable could not be read and sealed safely",
        )
    })?;
    if verified.length() != expected.executable.byte_len
        || verified.sha256() != expected.executable.sha256.as_bytes()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH",
            "the installed game executable does not match the project's exact generation",
        ));
    }
    Ok(())
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    if canonical_head_json(&head)? != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    serde_json::to_string(head).map_err(|_| invariant())
}

fn validate_signed_wire_values(project: &gore_authoring::ProjectRevision3) -> Result<(), Failure> {
    for value in [project.revision, project.target.executable.byte_len] {
        if value > i64::MAX as u64 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_BUILD_RESPONSE_LIMIT",
                "revision-3 Voice build contains an integer outside the signed wire range",
            ));
        }
    }
    Ok(())
}

fn bundle_content_seal(files: &gore_mod::Files) -> Result<(u64, String), Failure> {
    let mut total = 0u64;
    let mut digest = Sha256::new();
    digest.update(b"gore-mod.voice-bundle-tree.v1\0");
    for (path, bytes) in files {
        let path_len = u64::try_from(path.len()).map_err(|_| invariant())?;
        let byte_len = u64::try_from(bytes.len()).map_err(|_| invariant())?;
        total = total.checked_add(byte_len).ok_or_else(invariant)?;
        digest.update(path_len.to_be_bytes());
        digest.update(path.as_bytes());
        digest.update(byte_len.to_be_bytes());
        digest.update(bytes);
    }
    Ok((total, format!("{:x}", digest.finalize())))
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| invariant())?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_RESPONSE_LIMIT",
            "revision-3 Voice build response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits::default()
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    use WorkingStoreError::*;
    let code = match error {
        HeadConflict { .. } => "AUTHORING_REVISION3_VOICE_BUILD_HEAD_CONFLICT",
        MissingRoot(_) | MissingObject(_) => "AUTHORING_REVISION3_VOICE_BUILD_STORE_MISSING",
        UnsafePath { .. } => "AUTHORING_REVISION3_VOICE_BUILD_STORE_UNSAFE",
        LimitExceeded { .. } | InvalidLimits { .. } => {
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_LIMIT"
        }
        _ => "AUTHORING_REVISION3_VOICE_BUILD_STORE_INVARIANT",
    };
    Failure::new(code, error.to_string())
}

fn map_bundle_error(error: gore_mod::ModError) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BUILD_BUNDLE_INVALID",
        error.to_string(),
    )
}

fn map_bundle_write_error(error: gore_mod::ModError) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_FAILED",
        error.to_string(),
    )
}

fn map_bundle_verify_error(error: gore_mod::ModError) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED",
        error.to_string(),
    )
}

fn abort_staged_voice_bundle(staging: StagedVoiceBundle, primary: Failure) -> Failure {
    let staging_path = staging.path().display().to_string();
    match staging.abort() {
        Ok(()) => primary,
        Err(cleanup) => Failure::new(
            "AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED",
            format!(
                "{} ({}) and the owned Voice staging tree at {staging_path:?} could not be fully cleaned: {cleanup}",
                primary.message, primary.code
            ),
        ),
    }
}

fn map_bundle_promotion_error(error: VoiceBundleStagingError) -> Failure {
    let code = bundle_promotion_failure_code(error.kind(), error.cleanup_confirmed());
    Failure::new(code, error.to_string())
}

fn bundle_promotion_failure_code(
    kind: VoiceBundleStagingErrorKind,
    cleanup_confirmed: bool,
) -> &'static str {
    match kind {
        VoiceBundleStagingErrorKind::OperationFailed if cleanup_confirmed => {
            "AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED"
        }
        VoiceBundleStagingErrorKind::OperationFailed
        | VoiceBundleStagingErrorKind::CleanupFailed => {
            "AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED"
        }
        VoiceBundleStagingErrorKind::PublishedButUnconfirmed => {
            "AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED"
        }
    }
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BUILD_HEAD_CONFLICT",
        "the published revision-3 project changed during Voice build",
    )
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BUILD_INPUT_INVALID",
        "request must be exact canonical JSON containing command and exactly current_project_json, expected_head_json, game_root, output, and root",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_BUILD_INVARIANT",
        "revision-3 Voice build could not preserve its exact internal contract",
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
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry,
        OggCodec as ModelOggCodec, OggMetadata as ModelOggMetadata, OriginRef, SchemaRevisionV3,
        TypedRef, VoiceMemberProof as ModelVoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
        VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
    };
    use gore_authoring::{
        ArchiveSeal, AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2,
        GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta, ProjectRevision3, Sha256Digest,
        WorkingHead, WorkingProjectStore, WorkingStoreLimits,
    };
    use serde_json::Value;
    use sha2::{Digest as _, Sha256};

    use super::{
        build_revision3_voice_v1_inner_with_output_guard, bundle_promotion_failure_code,
        map_root_revalidation_failure, require_same_build_root_guard, BuildOutputPhase,
        BuildRootGuard, BuildVoiceWirePayload, ExactWireRequest, Failure, COMMAND,
    };

    const EXECUTABLE_BYTES: &[u8] = b"fixture managed Voice executable";

    #[test]
    fn published_but_unconfirmed_has_a_distinct_terminal_error_code() {
        assert_eq!(
            bundle_promotion_failure_code(
                gore_mod::VoiceBundleStagingErrorKind::PublishedButUnconfirmed,
                false,
            ),
            "AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED"
        );
    }

    #[test]
    fn root_revalidation_reports_the_exact_changed_authority() {
        let expected = BuildRootGuard {
            store: PathBuf::from("store-a"),
            game: PathBuf::from("game-a"),
            output_parent: PathBuf::from("output-parent-a"),
            output_target: PathBuf::from("output-parent-a/bundle"),
        };
        let cases = [
            (
                BuildRootGuard {
                    store: PathBuf::from("store-b"),
                    ..expected.clone()
                },
                "AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED",
            ),
            (
                BuildRootGuard {
                    game: PathBuf::from("game-b"),
                    ..expected.clone()
                },
                "AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED",
            ),
            (
                BuildRootGuard {
                    output_parent: PathBuf::from("output-parent-b"),
                    output_target: PathBuf::from("output-parent-b/bundle"),
                    ..expected.clone()
                },
                "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED",
            ),
        ];
        for (actual, expected_code) in cases {
            let failure = require_same_build_root_guard(&expected, &actual).unwrap_err();
            assert_eq!(failure.code, expected_code);
        }

        let unavailable_cases = [
            (
                "AUTHORING_REVISION3_VOICE_BUILD_STORE_UNAVAILABLE",
                "AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED",
            ),
            (
                "AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE",
                "AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED",
            ),
            (
                "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE",
                "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED",
            ),
        ];
        for (unavailable_code, expected_code) in unavailable_cases {
            let failure = map_root_revalidation_failure(Failure::new(unavailable_code, "fixture"));
            assert_eq!(failure.code, expected_code);
        }
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x31; 16])
    }

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn executable_seal() -> ContentSeal {
        ContentSeal {
            byte_len: EXECUTABLE_BYTES.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(EXECUTABLE_BYTES).into()),
        }
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn new_origin(label: &str) -> OriginRef {
        OriginRef::New {
            authored_runtime_id: label.to_owned(),
        }
    }

    fn voice_project(
        imported: &gore_authoring::ImportedOgg,
        target_resolution: VoiceTargetResolution,
        select_take: bool,
    ) -> ProjectRevision3 {
        let project_id = project_id();
        let localization_id = entity_id(1);
        let line_id = entity_id(2);
        let slot_id = entity_id(3);
        let take_id = entity_id(4);
        let locale = locale();
        let authored_ref = |id, kind| TypedRef::new(project_id, id, kind);
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id,
            revision: 5,
            meta: ProjectMeta {
                name: "ManagedVoiceE2E".into(),
                version: "1.0.0".into(),
                author: "gore-ffi tests".into(),
            },
            target: GameGenerationAnchor {
                executable: executable_seal(),
            },
            authoring_locales: BTreeSet::from([locale.clone()]),
            entities: BTreeMap::from([
                (
                    localization_id,
                    Entity {
                        id: localization_id,
                        display_name: "Asghan line".into(),
                        origin: new_origin("loc:asghan:e2e"),
                        revision: 1,
                        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: "GRD_263_ASGHAN_E2E".into(),
                            texts: BTreeMap::from([(locale.clone(), "Geh weiter.".into())]),
                        }),
                    },
                ),
                (
                    line_id,
                    Entity {
                        id: line_id,
                        display_name: "Asghan greeting".into(),
                        origin: new_origin("dialog:asghan:e2e"),
                        revision: 1,
                        payload: EntityPayload::DialogLine(DialogLine {
                            localization: authored_ref(
                                localization_id,
                                EntityKind::LocalizationEntry,
                            ),
                            speaker_hint: Some("Asghan".into()),
                            voice_slots: BTreeMap::from([(
                                locale.clone(),
                                authored_ref(slot_id, EntityKind::VoiceSlot),
                            )]),
                        }),
                    },
                ),
                (
                    slot_id,
                    Entity {
                        id: slot_id,
                        display_name: "Asghan DE".into(),
                        origin: new_origin("voice-slot:asghan:e2e:de"),
                        revision: 1,
                        payload: EntityPayload::VoiceSlot(VoiceSlot {
                            locale: locale.clone(),
                            target_resolution,
                            candidates: vec![authored_ref(take_id, EntityKind::VoiceTake)],
                            selected: select_take
                                .then(|| authored_ref(take_id, EntityKind::VoiceTake)),
                        }),
                    },
                ),
                (
                    take_id,
                    Entity {
                        id: take_id,
                        display_name: "Approved Asghan take".into(),
                        origin: OriginRef::Imported {
                            importer: "gore-ffi-e2e".into(),
                            source_seal: ContentSeal {
                                byte_len: imported.asset.byte_len,
                                sha256: imported.asset.sha256,
                            },
                            external_identity: None,
                        },
                        revision: 1,
                        payload: EntityPayload::VoiceTake(VoiceTake {
                            locale,
                            asset: imported.asset.clone(),
                            ogg: ModelOggMetadata {
                                codec: match imported.ogg.codec {
                                    gore_authoring::OggCodec::Vorbis => ModelOggCodec::Vorbis,
                                    gore_authoring::OggCodec::Opus => ModelOggCodec::Opus,
                                },
                                channels: imported.ogg.channels,
                                sample_rate: imported.ogg.sample_rate,
                                pages: imported.ogg.pages,
                                logical_streams: imported.ogg.logical_streams,
                            },
                            status: VoiceTakeStatus::Approved,
                        }),
                    },
                ),
            ]),
            asset_store: AssetStoreIndex {
                assets: BTreeMap::from([(
                    imported.asset.sha256,
                    AssetMeta {
                        byte_len: imported.asset.byte_len,
                        media_type: "audio/ogg".into(),
                    },
                )]),
            },
        }
    }

    fn resolved_target() -> VoiceTargetResolution {
        VoiceTargetResolution::Resolved {
            target: VoiceTarget {
                archive: "german_new.zip".into(),
                member: "NPC/Asghan/GRD_263_ASGHAN_E2E.ogg".into(),
                operation: VoiceOperation::Replace,
                archive_seal: ArchiveSeal {
                    byte_len: 4096,
                    sha256: digest(0x55),
                },
                member_proof: ModelVoiceMemberProof::Present {
                    uncompressed_size: 1024,
                    crc32: 0x1234_5678,
                },
            },
        }
    }

    fn publish_fixture(
        parent: &Path,
        target_resolution: VoiceTargetResolution,
        select_take: bool,
    ) -> (PathBuf, ProjectRevision3, WorkingHead) {
        let store_root = parent.join("store");
        let source = parent.join("take.ogg");
        std::fs::write(
            &source,
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        )
        .unwrap();
        let store = WorkingProjectStore::at(&store_root, WorkingStoreLimits::default()).unwrap();
        let imported = store.import_ogg(&source, "asghan-e2e.ogg", None).unwrap();
        assert_eq!(imported.ogg.codec, gore_authoring::OggCodec::Vorbis);
        let project = voice_project(&imported, target_resolution, select_take);
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        std::fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        (store_root, project, prepared.head)
    }

    fn matching_game_root(parent: &Path) -> PathBuf {
        matching_game_root_named(parent, "game")
    }

    fn matching_game_root_named(parent: &Path, name: &str) -> PathBuf {
        let game_root = parent.join(name);
        let executable = game_root
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(executable, EXECUTABLE_BYTES).unwrap();
        game_root
    }

    fn request(
        store_root: &Path,
        game_root: &Path,
        output: &Path,
        project: &ProjectRevision3,
        head: &WorkingHead,
    ) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: BuildVoiceWirePayload {
                current_project_json: project.to_canonical_json().unwrap(),
                expected_head_json: serde_json::to_string(head).unwrap(),
                game_root: game_root.to_str().unwrap().to_owned(),
                output: output.to_str().unwrap().to_owned(),
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

    fn top_level_entry_names(root: &Path) -> BTreeSet<std::ffi::OsString> {
        std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect()
    }

    #[test]
    fn store_voice_build_e2e_writes_verified_sealed_bundle_and_never_clobbers() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let output = temp.path().join("voice-bundle");
        let input = request(&store_root, &game_root, &output, &project, &head);
        let before = top_level_entry_names(temp.path());

        let built = execute(&input);
        assert_eq!(built["ok"], true);
        assert_eq!(built["outcome"], "built");
        assert_eq!(built["edit_count"], 1);
        assert_eq!(built["file_count"], 3);
        assert_eq!(
            built["build_authority"],
            "generation_sealed_existing_member_bundle_v1"
        );
        assert_eq!(built["deployment_status"], "not_performed");
        assert_eq!(built["bundle_sha256"].as_str().unwrap().len(), 64);
        gore_mod::verify_sealed_voice_bundle(&output).unwrap();
        let disk_seal = gore_mod::seal_voice_bundle_disk_tree(&output).unwrap();
        assert_eq!(built["bundle_bytes"], disk_seal.byte_len);
        assert_eq!(built["bundle_sha256"], disk_seal.sha256);
        let voice_manifest: gore_mod::VoicePatchManifest =
            serde_json::from_slice(&std::fs::read(output.join("voice/manifest.json")).unwrap())
                .unwrap();
        assert_eq!(voice_manifest.format, 3);
        assert_eq!(
            voice_manifest.executable_generation,
            Some(gore_mod::VoiceExecutableGenerationSeal {
                byte_len: EXECUTABLE_BYTES.len() as u64,
                sha256: format!("{:x}", Sha256::digest(EXECUTABLE_BYTES)),
            })
        );
        let exact_first_tree = read_tree(&output);
        let mut expected_after = before;
        expected_after.insert(output.file_name().unwrap().to_os_string());
        assert_eq!(top_level_entry_names(temp.path()), expected_after);

        let refused = execute(&input);
        assert_eq!(refused["ok"], false);
        assert_eq!(
            refused["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED"
        );
        assert!(refused["error"]["message"]
            .as_str()
            .unwrap()
            .contains("already exists"));
        assert_eq!(read_tree(&output), exact_first_tree);
        gore_mod::verify_sealed_voice_bundle(&output).unwrap();
    }

    #[test]
    fn unresolved_store_voice_build_is_blocked_without_creating_output() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) =
            publish_fixture(temp.path(), VoiceTargetResolution::Unresolved, false);
        let game_root = matching_game_root(temp.path());
        let output = temp.path().join("must-not-exist");

        let blocked = execute(&request(&store_root, &game_root, &output, &project, &head));
        assert_eq!(blocked["ok"], true);
        assert_eq!(blocked["outcome"], "blocked");
        assert_eq!(blocked["build_authority"], "not_granted");
        assert_eq!(blocked["deployment_status"], "not_performed");
        assert_eq!(blocked["report"]["total_slots"], 1);
        assert_eq!(blocked["report"]["ready_slots"], 0);
        let reasons: Vec<_> = blocked["report"]["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|blocker| blocker["reason"].as_str().unwrap())
            .collect();
        assert_eq!(reasons, ["unresolved_target", "missing_selected_take"]);
        assert!(!output.exists());
    }

    #[test]
    fn configured_game_root_must_carry_the_exact_project_executable_generation() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let fake_game = temp.path().join("fake-game");
        std::fs::create_dir(&fake_game).unwrap();
        let output = temp.path().join("must-not-exist");

        let unavailable = execute(&request(&store_root, &fake_game, &output, &project, &head));
        assert_eq!(
            unavailable["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_UNAVAILABLE"
        );
        assert!(!output.exists());

        let executable = fake_game
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(executable, b"wrong executable generation").unwrap();
        let mismatch = execute(&request(&store_root, &fake_game, &output, &project, &head));
        assert_eq!(
            mismatch["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH"
        );
        assert!(!output.exists());
    }

    #[test]
    fn executable_generation_is_rechecked_immediately_before_output() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let executable = game_root
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe");
        let output = temp.path().join("must-not-exist-after-generation-drift");
        let input = request(&store_root, &game_root, &output, &project, &head);

        let response = build_revision3_voice_v1_inner_with_output_guard(&input, |phase, _| {
            if phase == BuildOutputPhase::BeforeStagingWrite {
                std::fs::write(&executable, b"drifted after all managed Store reads").unwrap();
            }
        })
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH"
        );
        assert!(!output.exists());
    }

    #[test]
    fn executable_generation_drift_after_staging_removes_only_owned_staging() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let executable = game_root
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe");
        let output = temp.path().join("must-not-exist-after-post-write-drift");
        let input = request(&store_root, &game_root, &output, &project, &head);
        let before = top_level_entry_names(temp.path());

        let response =
            build_revision3_voice_v1_inner_with_output_guard(&input, |phase, _staging| {
                if phase == BuildOutputPhase::BeforePromotion {
                    std::fs::write(&executable, b"drifted after staging was verified").unwrap();
                }
            })
            .unwrap_err()
            .response();

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH"
        );
        assert!(!output.exists());
        assert_eq!(top_level_entry_names(temp.path()), before);
    }

    #[test]
    fn staging_verification_failure_cleans_staging_and_leaves_no_final_output() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let output = temp.path().join("must-not-exist-after-verify-failure");
        let input = request(&store_root, &game_root, &output, &project, &head);
        let before = top_level_entry_names(temp.path());

        let response =
            build_revision3_voice_v1_inner_with_output_guard(&input, |phase, staging| {
                if phase == BuildOutputPhase::AfterStagingWrite {
                    std::fs::write(staging.join("voice/payload/0.ogg"), b"tampered").unwrap();
                }
            })
            .unwrap_err()
            .response();

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED"
        );
        assert!(!output.exists());
        assert_eq!(top_level_entry_names(temp.path()), before);
    }

    #[test]
    fn foreign_staging_entry_is_never_deleted_and_reports_cleanup_failure() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let output = temp.path().join("must-not-exist-after-foreign-stage-entry");
        let input = request(&store_root, &game_root, &output, &project, &head);
        let staged_path = std::cell::RefCell::new(None);
        let foreign = b"not created by the Voice bundle writer";

        let response =
            build_revision3_voice_v1_inner_with_output_guard(&input, |phase, staging| {
                if phase == BuildOutputPhase::AfterStagingWrite {
                    staged_path.replace(Some(staging.to_path_buf()));
                    std::fs::write(staging.join("foreign.keep"), foreign).unwrap();
                }
            })
            .unwrap_err()
            .response();

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED"
        );
        assert!(!output.exists());
        let staged_path = staged_path.into_inner().unwrap();
        assert_eq!(
            std::fs::read(staged_path.join("foreign.keep")).unwrap(),
            foreign
        );
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&staged_path.display().to_string()));
    }

    #[test]
    fn racing_final_creator_is_preserved_and_owned_staging_is_removed() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let output = temp.path().join("foreign-racing-final");
        let input = request(&store_root, &game_root, &output, &project, &head);
        let before = top_level_entry_names(temp.path());
        let foreign = b"foreign final creator wins";

        let response =
            build_revision3_voice_v1_inner_with_output_guard(&input, |phase, _staging| {
                if phase == BuildOutputPhase::BeforePromotion {
                    std::fs::create_dir(&output).unwrap();
                    std::fs::write(output.join("foreign.txt"), foreign).unwrap();
                }
            })
            .unwrap_err()
            .response();

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED"
        );
        assert_eq!(std::fs::read(output.join("foreign.txt")).unwrap(), foreign);
        let mut expected_after = before;
        expected_after.insert(output.file_name().unwrap().to_os_string());
        assert_eq!(top_level_entry_names(temp.path()), expected_after);
    }

    #[test]
    fn later_published_head_does_not_relabel_the_owned_exact_basis_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let output = temp.path().join("exact-basis-voice-bundle");
        let input = request(&store_root, &game_root, &output, &project, &head);
        let mut later_project = project.clone();
        later_project.revision += 1;
        later_project.meta.version = "later-independent-head".to_owned();
        let later_head = std::cell::RefCell::new(None);

        let built = build_revision3_voice_v1_inner_with_output_guard(&input, |phase, _| {
            if phase == BuildOutputPhase::BeforeStagingWrite {
                let store =
                    WorkingProjectStore::open_existing(&store_root, WorkingStoreLimits::default())
                        .unwrap();
                let prepared = store
                    .prepare_revision3_checkpoint(Some(&head), &later_project)
                    .unwrap();
                std::fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
                later_head.replace(Some(prepared.head));
            }
        })
        .unwrap();

        assert_eq!(built["ok"], true, "{built}");
        assert_eq!(built["outcome"], "built");
        assert_eq!(built["project_revision"], project.revision);
        assert_eq!(
            built["basis_head_json"],
            serde_json::to_string(&head).unwrap()
        );
        let current =
            WorkingProjectStore::open_existing(&store_root, WorkingStoreLimits::default())
                .unwrap()
                .open_current_revision3(gore_authoring::AssetVerification::Full)
                .unwrap();
        assert_eq!(current.head, later_head.into_inner().unwrap());
        assert_eq!(current.project, later_project);
        gore_mod::verify_sealed_voice_bundle(&output).unwrap();
    }

    #[test]
    fn output_inside_a_different_recognizable_game_layout_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let configured_game = matching_game_root_named(temp.path(), "configured-game");
        let other_game = matching_game_root_named(temp.path(), "other-game");
        let output = other_game.join("must-not-be-created");

        let response = execute(&request(
            &store_root,
            &configured_game,
            &output,
            &project,
            &head,
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS"
        );
        assert!(!output.exists());
    }

    #[test]
    fn output_inside_game_install_is_rejected_before_any_write() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let output = game_root.join("must-not-be-created");

        let response = execute(&request(&store_root, &game_root, &output, &project, &head));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS"
        );
        assert!(!output.exists());
    }

    #[test]
    fn direct_g1r_mixed_case_root_builds_against_the_shared_semantic_install() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());
        let direct_g1r = game_root.join("g1R");
        let output = temp.path().join("direct-g1r-voice-bundle");

        let response = execute(&request(&store_root, &direct_g1r, &output, &project, &head));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "built");
        assert_eq!(response["output"], output.display().to_string());
        gore_mod::verify_sealed_voice_bundle(&output).unwrap();
    }

    #[test]
    fn malformed_and_unsafe_build_roots_fail_before_output() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let game_root = matching_game_root(temp.path());

        let noncanonical_output = temp.path().join("noncanonical-must-not-exist");
        let canonical = request(
            &store_root,
            &game_root,
            &noncanonical_output,
            &project,
            &head,
        );
        let noncanonical = execute(&format!(" {canonical}"));
        assert_eq!(
            noncanonical["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_INPUT_INVALID"
        );
        assert!(!noncanonical_output.exists());

        let missing_parent_output = temp.path().join("missing-parent").join("bundle");
        let missing_parent = execute(&request(
            &store_root,
            &game_root,
            &missing_parent_output,
            &project,
            &head,
        ));
        assert_eq!(
            missing_parent["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE"
        );
        assert!(!missing_parent_output.exists());

        let store_output = store_root.join("must-not-be-created");
        let aliased = execute(&request(
            &store_root,
            &game_root,
            &store_output,
            &project,
            &head,
        ));
        assert_eq!(
            aliased["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_STORE_OUTPUT_ALIAS"
        );
        assert!(!store_output.exists());

        let missing_game = temp.path().join("missing-game");
        let missing_game_output = temp.path().join("missing-game-must-not-exist");
        let unavailable = execute(&request(
            &store_root,
            &missing_game,
            &missing_game_output,
            &project,
            &head,
        ));
        assert_eq!(
            unavailable["error"]["code"],
            "AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE"
        );
        assert!(!missing_game_output.exists());
    }
}
