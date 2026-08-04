//! Exact-current, read-only installed DataAsset package-candidate audit for revision-3 projects.
//!
//! The client supplies only one managed Store root, its exact canonical head, and one game root.
//! Native code fully reopens the Store, derives the executable anchor from the project, and uses
//! `gore-tex`'s safe installed-package snapshot. The response exposes semantic `/Game` candidates
//! and path-free seals only. It never returns project JSON, local paths, package bytes, selectors,
//! offsets, file identities, or any mutation/build/runtime/publication authority.

use std::io::{self, Write};
use std::path::Path;

use gore_authoring::{
    AssetVerification, ProjectRevision3, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_tex::installed_package_index::{
    inspect_installed_package_index_v1, ExpectedInstalledExecutableV1,
    InstalledPackageContentSealV1, InstalledPackageIndexErrorV1, VerifiedInstalledPackageIndexV1,
};
use gore_tex::package_index::{PackageIndexError, PackageIndexStatus};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_read_revision3_dataasset_package_index_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = (MAX_PATH_BYTES * 2 + MAX_HEAD_JSON_BYTES) * 6 + 4 * 1024;
const MAX_RESPONSE_BYTES: usize = crate::transport::MAX_TRANSPORT_RESPONSE_BYTES;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadPackageIndexWirePayload {
    expected_head_json: String,
    game_root: String,
    root: String,
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

pub(super) fn read_revision3_dataasset_package_index_v1_raw(input: &str) -> Value {
    read_revision3_dataasset_package_index_v1_inner(input, MAX_RESPONSE_BYTES)
        .unwrap_or_else(Failure::response)
}

fn read_revision3_dataasset_package_index_v1_inner(
    input: &str,
    response_limit: usize,
) -> Result<Value, Failure> {
    let payload: ReadPackageIndexWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(head_conflict());
    }
    validate_project(&before.project)?;
    let head_json = serde_json::to_string(&before.head).map_err(|_| invariant_failure())?;
    if head_json != payload.expected_head_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_INVALID",
            "expected_head_json is not in exact canonical form",
        ));
    }

    let executable_anchor = ExpectedInstalledExecutableV1 {
        byte_len: before.project.target.executable.byte_len,
        sha256: *before.project.target.executable.sha256.as_bytes(),
    };
    let snapshot =
        inspect_installed_package_index_v1(Path::new(&payload.game_root), executable_anchor)
            .map_err(map_snapshot_error)?;
    validate_snapshot(&snapshot, &before.project)?;
    snapshot.revalidate().map_err(map_snapshot_error)?;

    let package_index_status = match snapshot.index().status {
        PackageIndexStatus::CompleteIndex => "complete_index",
        PackageIndexStatus::PartialIndex => "partial_index",
    };
    let candidate_count =
        u64::try_from(snapshot.index().candidates.len()).map_err(|_| invariant_failure())?;
    let response_result = enforce_response_budget(
        json!({
            "ok": true,
            "outcome": "audit_only",
            "head_json": head_json,
            "project_id": before.project.project_id.to_string(),
            "project_revision": before.project.revision,
            "package_index_json": snapshot.index_json(),
            "package_index_status": package_index_status,
            "candidate_count": candidate_count,
            "target_executable_seal": snapshot.target_executable(),
            "mount_inventory_entry_count": snapshot.mount_inventory_entry_count(),
            "mount_inventory_seal": snapshot.mount_inventory_seal(),
            "package_index_seal": snapshot.index_seal(),
            "source_snapshot_seal": snapshot.source_snapshot_seal(),
            "scope": "installed_dataasset_package_candidates_only",
            "content_status": "metadata_candidates_only",
            "export_bundle_payload_status": "not_read",
            "mutation_status": "not_supported",
            "build_status": "not_evaluated",
            "runtime_status": "runtime_unqualified",
            "publication_status": "not_supported",
            "authority_status": "not_granted",
        }),
        response_limit,
    );

    // Close the response-construction window for both sources even if the local response budget
    // was exceeded. Security drift wins over a transport-size failure.
    let snapshot_after = snapshot.revalidate().map_err(map_snapshot_error);
    let store_after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error);
    snapshot_after?;
    let after = store_after?;
    if after.head != expected_head || after.project != before.project {
        return Err(head_conflict());
    }

    response_result
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INPUT_LIMIT",
            format!("DataAsset package-index request exceeds the {MAX_WIRE_BYTES}-byte limit"),
        ));
    }
    let request: ExactWireRequest<P> =
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

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_INVALID",
            "expected_head_json is not one closed revision-3 working head",
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| invariant_failure())?;
    if canonical != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn validate_project(project: &ProjectRevision3) -> Result<(), Failure> {
    signed_wire_u64(project.revision)?;
    signed_wire_u64(project.target.executable.byte_len)?;
    if project.target.executable.byte_len == 0 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_TARGET_INVALID",
            "the exact project executable anchor is invalid",
        ));
    }
    Ok(())
}

fn validate_snapshot(
    snapshot: &VerifiedInstalledPackageIndexV1,
    project: &ProjectRevision3,
) -> Result<(), Failure> {
    let executable = snapshot.target_executable();
    if executable.byte_len != project.target.executable.byte_len
        || executable.sha256 != project.target.executable.sha256.to_string()
    {
        return Err(invariant_failure());
    }
    for seal in [
        executable,
        snapshot.mount_inventory_seal(),
        snapshot.index_seal(),
        snapshot.source_snapshot_seal(),
    ] {
        validate_seal(seal)?;
    }
    signed_wire_u64(snapshot.mount_inventory_entry_count())?;
    let index = snapshot.index();
    for value in [
        index.physical_chunk_count,
        index.winning_export_bundle_count,
        index.directory_indexed_export_bundle_count,
        index.out_of_scope_export_bundle_count,
        u64::try_from(index.candidates.len()).map_err(|_| invariant_failure())?,
        u64::try_from(index.partial_reasons.len()).map_err(|_| invariant_failure())?,
    ] {
        signed_wire_u64(value)?;
    }
    for reason in &index.partial_reasons {
        signed_wire_u64(reason.count)?;
    }

    let canonical_index = serde_json::to_string(index).map_err(|_| invariant_failure())?;
    let canonical_index_len =
        u64::try_from(canonical_index.len()).map_err(|_| invariant_failure())?;
    if canonical_index != snapshot.index_json()
        || snapshot.index_seal().byte_len != canonical_index_len
        || snapshot.index_seal().sha256 != hex_digest(&Sha256::digest(canonical_index.as_bytes()))
    {
        return Err(invariant_failure());
    }
    Ok(())
}

fn validate_seal(seal: &InstalledPackageContentSealV1) -> Result<(), Failure> {
    signed_wire_u64(seal.byte_len)?;
    if seal.sha256.len() != 64
        || !seal
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invariant_failure());
    }
    Ok(())
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_RESPONSE_LIMIT",
            "DataAsset package-index evidence contains an integer outside the signed wire range",
        ));
    }
    Ok(())
}

struct BoundedResponseCounter {
    bytes: usize,
    limit: usize,
    exceeded: bool,
}

impl Write for BoundedResponseCounter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let Some(next) = self.bytes.checked_add(buffer.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("response counter overflow"));
        };
        if next > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("response budget exceeded"));
        }
        self.bytes = next;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let mut counter = BoundedResponseCounter {
        bytes: 0,
        limit,
        exceeded: false,
    };
    if serde_json::to_writer(&mut counter, &response).is_err() {
        return if counter.exceeded {
            Err(Failure::new(
                "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_RESPONSE_LIMIT",
                "DataAsset package-index response exceeds its bounded transport budget",
            ))
        } else {
            Err(invariant_failure())
        };
    }
    Ok(response)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly expected_head_json, game_root, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INVARIANT",
        "the native DataAsset package-index audit failed an internal invariant",
    )
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_MISSING"
        }
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 working Store audit failed")
}

fn map_snapshot_error(error: InstalledPackageIndexErrorV1) -> Failure {
    use InstalledPackageIndexErrorV1 as E;
    let code = match error {
        E::InvalidExpectedExecutable => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_TARGET_INVALID"
        }
        E::ParentTraversal
        | E::PathContainsNul
        | E::UnsafePath { .. }
        | E::NonUtf8TreeEntry
        | E::UnsafeTreeEntry => "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_PATH_UNSAFE",
        E::NestedMountable
        | E::NoncanonicalMountName { .. }
        | E::MountNameCollision
        | E::MainContainerMissing
        | E::MountCompanionMissing { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LAYOUT_INVALID"
        }
        E::TreeEntryLimit { .. }
        | E::TreeDepthLimit { .. }
        | E::TreePathLimit { .. }
        | E::AggregateTreePathLimit { .. }
        | E::DirectMountLimit { .. }
        | E::FileLengthLimit { .. }
        | E::AggregateHashedMountLimit { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LIMIT"
        }
        E::ExecutableMismatch => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_GENERATION_MISMATCH"
        }
        E::SourceChanged { .. } | E::TreeChanged | E::OpenedContainerSetChanged => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_CHANGED"
        }
        E::ContainerPriority(error) | E::PackageIndex(error) => {
            return map_package_index_error(error);
        }
        E::IoStoreOpen => "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_IOSTORE_OPEN_FAILED",
        E::IndexJsonLimit { .. } => "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_RESPONSE_LIMIT",
        E::IndexSerialization | E::CounterOverflow => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INVARIANT"
        }
        E::Filesystem { .. } => "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_IO",
        E::UnsupportedPlatform => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PLATFORM_UNSUPPORTED"
        }
    };
    Failure::new(code, "the installed DataAsset package-index audit failed")
}

fn map_package_index_error(error: PackageIndexError) -> Failure {
    use PackageIndexError as E;
    let code = match error {
        E::InvalidLimits(_) | E::InvalidLimit { .. } | E::CounterOverflow => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INVARIANT"
        }
        E::AmbiguousContainerPriority { .. } | E::ContainerPriorityVersionOverflow { .. } => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LAYOUT_INVALID"
        }
        E::ChildContainerLimit { .. }
        | E::ContainerPriorityNameLimit { .. }
        | E::AggregateContainerPriorityNameLimit { .. }
        | E::ChunkScanLimit { .. }
        | E::WinningExportBundleLimit { .. }
        | E::DirectoryPathLimit { .. }
        | E::AggregateDirectoryPathLimit { .. }
        | E::CandidateLimit { .. } => "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LIMIT",
        E::ContainerVersionUnavailable => {
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PACKAGE_INDEX_FAILED"
        }
    };
    Failure::new(code, "the installed DataAsset package-index audit failed")
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
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

#[cfg(all(test, windows))]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::{self, OpenOptions};
    use std::io::{Seek as _, SeekFrom, Write as _};
    use std::path::{Path, PathBuf};

    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        ProjectRevision3, SchemaRevisionV3, Sha256Digest,
    };
    use retoc::iostore_writer::IoStoreWriter;
    use retoc::version::EngineVersion;
    use retoc::{EIoChunkType, FIoChunkId, FIoContainerId, FPackageId, UEPath, UEPathBuf};
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;

    const EXE_BYTES: &[u8] = b"gore-ffi package index executable fixture v1";
    const TARGET: &str = "/Game/Characters/DA_Asghan";
    const PRIVATE_PROJECT_MARKER: &str = "PRIVATE PROJECT JSON MUST NOT ESCAPE";

    struct Fixture {
        _temp: TempDir,
        store_root: PathBuf,
        game_root: PathBuf,
        paks: PathBuf,
        head_json: String,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = TempDir::new().unwrap();
            let store_root = temp.path().join("private-store-root");
            let game_root = temp.path().join("private-game-root");
            let g1r = game_root.join("G1R");
            let paks = g1r.join("Content/Paks");
            let executable = g1r.join("Binaries/Win64/G1R-Win64-Shipping.exe");
            fs::create_dir_all(&paks).unwrap();
            fs::create_dir_all(executable.parent().unwrap()).unwrap();
            fs::write(&executable, EXE_BYTES).unwrap();
            write_container(&paks.join("G1R-Windows.utoc"));

            let project = project();
            let store = WorkingProjectStore::at(&store_root, ffi_store_limits()).unwrap();
            let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
            fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
            let head_json = String::from_utf8(prepared.head_bytes).unwrap();
            Self {
                _temp: temp,
                store_root,
                game_root,
                paks,
                head_json,
            }
        }

        fn raw_request(&self) -> String {
            serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": self.head_json,
                    "game_root": self.game_root.to_string_lossy(),
                    "root": self.store_root.to_string_lossy(),
                },
            }))
            .unwrap()
        }

        fn call(&self) -> Value {
            serde_json::from_str(&crate::execute_json(&self.raw_request())).unwrap()
        }
    }

    fn project() -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x44; 16]),
            revision: 7,
            meta: ProjectMeta {
                name: PRIVATE_PROJECT_MARKER.to_owned(),
                version: "0.1.0-private".to_owned(),
                author: "private-author".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: EXE_BYTES.len() as u64,
                    sha256: Sha256Digest::from_bytes(Sha256::digest(EXE_BYTES).into()),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn package_id(target: &str) -> FPackageId {
        FPackageId(FIoContainerId::from_name(target).0)
    }

    fn write_container(path: &Path) {
        let version = EngineVersion::UE5_4;
        let mut writer = IoStoreWriter::new(
            path,
            version.toc_version(),
            None,
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        writer
            .write_chunk(
                FIoChunkId::from_package_id(package_id(TARGET), 0, EIoChunkType::ExportBundleData),
                Some(UEPath::new(
                    "../../../G1R/Content/Characters/DA_Asghan.uasset",
                )),
                b"export payload must remain unread",
            )
            .unwrap();
        writer.finalize().unwrap();
    }

    fn tree_bytes(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        let mut output = BTreeMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.is_dir() {
                    output.insert(relative, None);
                    pending.push(path);
                } else {
                    output.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        output
    }

    fn overwrite_same_length(path: &Path, byte: u8) {
        let length = fs::metadata(path).unwrap().len();
        let mut file = OpenOptions::new().write(true).open(path).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let block = [byte; 64 * 1024];
        let mut remaining = length;
        while remaining > 0 {
            let count = remaining.min(block.len() as u64) as usize;
            file.write_all(&block[..count]).unwrap();
            remaining -= count as u64;
        }
        file.flush().unwrap();
        assert_eq!(fs::metadata(path).unwrap().len(), length);
    }

    #[test]
    fn exact_current_store_returns_one_closed_path_free_audit_without_writes() {
        let fixture = Fixture::new();
        let store_before = tree_bytes(&fixture.store_root);
        let game_before = tree_bytes(&fixture.game_root);

        let response = fixture.call();

        assert_eq!(response["ok"], true);
        let ordered_keys = response.as_object().unwrap().keys().collect::<Vec<_>>();
        assert!(ordered_keys.windows(2).all(|pair| pair[0] < pair[1]));
        let keys = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            keys,
            BTreeSet::from([
                "authority_status",
                "build_status",
                "candidate_count",
                "content_status",
                "export_bundle_payload_status",
                "head_json",
                "mount_inventory_entry_count",
                "mount_inventory_seal",
                "mutation_status",
                "ok",
                "outcome",
                "package_index_json",
                "package_index_seal",
                "package_index_status",
                "project_id",
                "project_revision",
                "publication_status",
                "runtime_status",
                "scope",
                "source_snapshot_seal",
                "target_executable_seal",
            ])
        );
        assert_eq!(response["outcome"], "audit_only");
        assert_eq!(response["head_json"], fixture.head_json);
        assert_eq!(response["project_id"], "44".repeat(16));
        assert_eq!(response["project_revision"], 7);
        assert_eq!(response["package_index_status"], "complete_index");
        assert_eq!(response["candidate_count"], 1);
        assert_eq!(response["mount_inventory_entry_count"], 2);
        assert_eq!(
            response["scope"],
            "installed_dataasset_package_candidates_only"
        );
        assert_eq!(response["content_status"], "metadata_candidates_only");
        assert_eq!(response["export_bundle_payload_status"], "not_read");
        assert_eq!(response["mutation_status"], "not_supported");
        assert_eq!(response["build_status"], "not_evaluated");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(response["authority_status"], "not_granted");

        let index: Value =
            serde_json::from_str(response["package_index_json"].as_str().unwrap()).unwrap();
        assert_eq!(index["status"], "complete_index");
        assert_eq!(index["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(index["candidates"][0]["target_path"], TARGET);
        for key in [
            "target_executable_seal",
            "mount_inventory_seal",
            "package_index_seal",
            "source_snapshot_seal",
        ] {
            assert_eq!(response[key].as_object().unwrap().len(), 2);
            assert!(response[key]["byte_len"].as_u64().is_some());
            let digest = response[key]["sha256"].as_str().unwrap();
            assert_eq!(digest.len(), 64);
            assert!(digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')));
        }

        let encoded = serde_json::to_string(&response).unwrap();
        assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
        assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
        assert!(!encoded.contains(PRIVATE_PROJECT_MARKER));
        assert!(!encoded.contains("project_json"));
        assert!(!encoded.contains("request_binding_sha256"));
        assert_eq!(tree_bytes(&fixture.store_root), store_before);
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[test]
    fn exact_project_executable_anchor_rejects_another_generation() {
        let fixture = Fixture::new();
        fs::write(
            fixture
                .game_root
                .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe"),
            vec![0x5a; EXE_BYTES.len()],
        )
        .unwrap();

        let response = fixture.call();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_GENERATION_MISMATCH"
        );
        let encoded = serde_json::to_string(&response).unwrap();
        assert!(!encoded.contains(&fixture.game_root.to_string_lossy().to_string()));
        assert!(!encoded.contains(PRIVATE_PROJECT_MARKER));
    }

    #[test]
    fn corrupt_export_bundle_ucas_remains_unread() {
        let fixture = Fixture::new();
        overwrite_same_length(&fixture.paks.join("G1R-Windows.ucas"), 0xa5);
        let response = fixture.call();
        assert_eq!(response["ok"], true);
        assert_eq!(response["export_bundle_payload_status"], "not_read");
        assert_eq!(response["candidate_count"], 1);
    }

    #[test]
    fn stale_noncanonical_or_duplicate_head_fails_closed() {
        let fixture = Fixture::new();
        let request_with_head = |head: String| {
            serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": head,
                    "game_root": fixture.game_root.to_string_lossy(),
                    "root": fixture.store_root.to_string_lossy(),
                },
            }))
            .unwrap()
        };
        let stale = fixture
            .head_json
            .replace("\"byte_len\":", "\"byte_len\":999");
        assert_eq!(
            read_revision3_dataasset_package_index_v1_raw(&request_with_head(stale))["error"]
                ["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_CONFLICT"
        );
        assert_eq!(
            read_revision3_dataasset_package_index_v1_raw(&request_with_head(format!(
                " {}",
                fixture.head_json
            )))["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_INVALID"
        );
        let duplicate = fixture.head_json.replacen('{', "{\"store_format\":1,", 1);
        assert_eq!(
            read_revision3_dataasset_package_index_v1_raw(&request_with_head(duplicate))["error"]
                ["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_INVALID"
        );
    }

    #[test]
    fn raw_wire_requires_exact_duplicate_free_three_field_payload() {
        let fixture = Fixture::new();
        let head = serde_json::to_string(&fixture.head_json).unwrap();
        let game = serde_json::to_string(&fixture.game_root.to_string_lossy()).unwrap();
        let root = serde_json::to_string(&fixture.store_root.to_string_lossy()).unwrap();
        for raw in [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":{head},\"game_root\":{game},\"root\":{root}}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":{head},\"game_root\":{game},\"game_root\":{game},\"root\":{root}}}}}"
            ),
            serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": fixture.head_json,
                    "game_root": fixture.game_root.to_string_lossy(),
                    "root": fixture.store_root.to_string_lossy(),
                    "project_json": "forged",
                },
            }))
            .unwrap(),
            serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": fixture.head_json,
                    "game_root": 7,
                    "root": fixture.store_root.to_string_lossy(),
                },
            }))
            .unwrap(),
            serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": fixture.head_json,
                    "game_root": fixture.game_root.to_string_lossy(),
                },
            }))
            .unwrap(),
            serde_json::to_string(&json!({
                "command": "wrong",
                "payload": {
                    "expected_head_json": fixture.head_json,
                    "game_root": fixture.game_root.to_string_lossy(),
                    "root": fixture.store_root.to_string_lossy(),
                },
            }))
            .unwrap(),
        ] {
            assert_eq!(
                read_revision3_dataasset_package_index_v1_raw(&raw)["error"]["code"],
                "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_REQUEST_INVALID"
            );
        }
        assert_eq!(
            read_revision3_dataasset_package_index_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))["error"]
                ["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INPUT_LIMIT"
        );
    }

    #[test]
    fn dispatch_routes_oversize_wire_to_the_command_local_cap() {
        let raw = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"padding\":\"{}\"}}}}",
            "x".repeat(MAX_WIRE_BYTES + 1)
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&raw)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INPUT_LIMIT"
        );
    }

    #[test]
    fn bounded_response_failure_still_leaves_store_and_game_unchanged() {
        let fixture = Fixture::new();
        let store_before = tree_bytes(&fixture.store_root);
        let game_before = tree_bytes(&fixture.game_root);
        let failure = read_revision3_dataasset_package_index_v1_inner(&fixture.raw_request(), 128)
            .unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_RESPONSE_LIMIT"
        );
        assert_eq!(tree_bytes(&fixture.store_root), store_before);
        assert_eq!(tree_bytes(&fixture.game_root), game_before);
    }

    #[test]
    fn game_and_store_failures_never_echo_private_paths() {
        let fixture = Fixture::new();
        let private_missing = fixture
            ._temp
            .path()
            .join("PRIVATE-MISSING-GAME-PATH-DO-NOT-ECHO");
        let raw = serde_json::to_string(&json!({
            "command": COMMAND,
            "payload": {
                "expected_head_json": fixture.head_json,
                "game_root": private_missing.to_string_lossy(),
                "root": fixture.store_root.to_string_lossy(),
            },
        }))
        .unwrap();
        let response = read_revision3_dataasset_package_index_v1_raw(&raw);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_IO"
        );
        let encoded = serde_json::to_string(&response).unwrap();
        assert!(!encoded.contains("PRIVATE-MISSING-GAME-PATH-DO-NOT-ECHO"));
        assert!(!encoded.contains(&fixture.store_root.to_string_lossy().to_string()));
    }

    #[test]
    fn snapshot_error_categories_are_closed_and_sanitized() {
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::UnsupportedPlatform).code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PLATFORM_UNSUPPORTED"
        );
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::ParentTraversal).code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_PATH_UNSAFE"
        );
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::MainContainerMissing).code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LAYOUT_INVALID"
        );
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::TreeEntryLimit {
                actual: 2,
                limit: 1,
            })
            .code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LIMIT"
        );
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::OpenedContainerSetChanged).code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_CHANGED"
        );
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::ContainerPriority(
                PackageIndexError::AmbiguousContainerPriority {
                    first: "private-a".to_owned(),
                    second: "private-b".to_owned(),
                },
            ))
            .code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LAYOUT_INVALID"
        );
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::Path;

    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        ProjectRevision3, SchemaRevisionV3, Sha256Digest, WorkingProjectStore,
    };
    use tempfile::TempDir;

    use super::*;

    fn tree_bytes(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        let mut output = BTreeMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.is_dir() {
                    output.insert(relative, None);
                    pending.push(path);
                } else {
                    output.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        output
    }

    #[test]
    fn unix_maps_platform_unsupported_without_accessing_the_game_tree_or_writing_the_store() {
        let temp = TempDir::new().unwrap();
        let store_root = temp.path().join("store");
        let missing_game_root = temp.path().join("game-tree-must-stay-missing");
        let project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x77; 16]),
            revision: 3,
            meta: ProjectMeta {
                name: "unix platform gate".to_owned(),
                version: "0.1.0".to_owned(),
                author: "test".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 1,
                    sha256: Sha256Digest::from_bytes([0x55; 32]),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        };
        let store = WorkingProjectStore::at(&store_root, ffi_store_limits()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        let head_json = String::from_utf8(prepared.head_bytes).unwrap();
        let store_before = tree_bytes(&store_root);
        assert!(!missing_game_root.exists());

        let raw = serde_json::to_string(&json!({
            "command": COMMAND,
            "payload": {
                "expected_head_json": head_json,
                "game_root": missing_game_root.to_string_lossy(),
                "root": store_root.to_string_lossy(),
            },
        }))
        .unwrap();
        let response = read_revision3_dataasset_package_index_v1_raw(&raw);

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PLATFORM_UNSUPPORTED"
        );
        assert!(!missing_game_root.exists());
        assert_eq!(tree_bytes(&store_root), store_before);
        assert_eq!(
            map_snapshot_error(InstalledPackageIndexErrorV1::UnsupportedPlatform).code,
            "AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PLATFORM_UNSUPPORTED"
        );
    }
}
