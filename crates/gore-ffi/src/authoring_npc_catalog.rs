//! Native, read-only construction of one exact pinned NPC archetype catalog.
//!
//! The client supplies only a game root. Executable and Binds paths are fixed natively and the
//! Shipping cache is selected exclusively through `gore_mod::pristine_script_cache`. Every
//! success remains runtime-unqualified and exposes no build, deploy, or publication operation.

use std::fs::{File, OpenOptions};
use std::io::{self, Read as _, Take};
use std::path::{Path, PathBuf};

use gore_npc_catalog::{
    build_npc_archetype_catalog, NpcArchetypeCatalogFile, NpcCatalogError, MAX_CATALOG_JSON_BYTES,
};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, CatalogError, ContentSeal, GenerationInputLimits,
    StoryCatalogFile,
};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

const MAX_GAME_ROOT_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 24 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const REQUEST_BINDING_DOMAIN: &[u8] =
    b"gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\0";

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

pub(super) fn build_for_game_root_v1(payload: Value) -> Value {
    build_for_game_root_v1_inner(&payload, MAX_RESPONSE_BYTES).unwrap_or_else(Failure::response)
}

fn build_for_game_root_v1_inner(payload: &Value, response_limit: usize) -> Result<Value, Failure> {
    let object = exact_payload(payload)?;
    let game_root_wire = bounded_game_root(object)?;
    let request_binding_sha256 = request_binding(game_root_wire);
    let game_root = PathBuf::from(game_root_wire);
    let g1r = if game_root.file_name().is_some_and(|name| name == "G1R") {
        game_root.clone()
    } else {
        game_root.join("G1R")
    };
    let executable = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let binds_path = g1r.join("Script").join("Binds.Cache");

    // This is the only Shipping-cache selector. Clients never choose a live cache, backup, or
    // deployment record and never receive the derived native paths.
    let shipping = gore_mod::pristine_script_cache(&game_root).map_err(map_pristine_error)?;
    let limits = GenerationInputLimits::default();
    let story_catalog =
        build_known_catalog_with_shipping_snapshot(&executable, &shipping, &binds_path, limits)
            .map_err(map_story_catalog_error)?;
    story_catalog
        .revalidate_generation_inputs()
        .map_err(map_story_catalog_error)?;
    let binds = read_bounded_regular(&binds_path, limits.max_binds_cache_bytes)?;
    revalidate_sources(&story_catalog, &game_root)?;

    let npc_catalog = build_npc_archetype_catalog(&story_catalog, &shipping, &binds)
        .map_err(map_npc_catalog_error)?;

    // Close the mutable native-input window on both sides of canonical catalog serialization.
    revalidate_sources(&story_catalog, &game_root)?;
    let catalog_bytes = npc_catalog
        .to_canonical_json()
        .map_err(map_npc_catalog_error)?;
    if catalog_bytes.len() > MAX_CATALOG_JSON_BYTES {
        return Err(catalog_limit_failure());
    }
    revalidate_sources(&story_catalog, &game_root)?;
    let catalog_json = String::from_utf8(catalog_bytes).map_err(|_| catalog_build_failure())?;

    let response = catalog_response(&npc_catalog, request_binding_sha256, catalog_json);
    // Serialize once under the command-specific budget and revalidate the guarded paths and
    // deployment-aware pristine selection immediately before and after that serialization.
    revalidate_sources(&story_catalog, &game_root)?;
    let response = enforce_response_budget(response, response_limit)?;
    revalidate_sources(&story_catalog, &game_root)?;
    Ok(response)
}

fn catalog_response(
    catalog: &NpcArchetypeCatalogFile,
    request_binding_sha256: String,
    catalog_json: String,
) -> Value {
    json!({
        "ok": true,
        "request_binding_sha256": request_binding_sha256,
        "catalog_json": catalog_json,
        "generation": catalog.generation(),
        "catalog_seal": catalog.catalog_seal(),
        "source": catalog.source(),
        "payload_seal": catalog.payload_seal(),
        "record_count": catalog.records().len(),
        "rejection_count": catalog.rejections().len(),
        "qualification": catalog.qualification(),
    })
}

fn exact_payload(payload: &Value) -> Result<&Map<String, Value>, Failure> {
    let object = payload.as_object().ok_or_else(invalid_request)?;
    if object.len() != 1 || !object.contains_key("game_root") {
        return Err(invalid_request());
    }
    Ok(object)
}

fn bounded_game_root(object: &Map<String, Value>) -> Result<&str, Failure> {
    object
        .get("game_root")
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty() && value.len() <= MAX_GAME_ROOT_BYTES && !value.contains('\0')
        })
        .ok_or_else(invalid_request)
}

fn request_binding(game_root: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_BINDING_DOMAIN);
    let bytes = game_root.as_bytes();
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hex_digest(hasher.finalize())
}

fn revalidate_sources(catalog: &StoryCatalogFile, game_root: &Path) -> Result<(), Failure> {
    catalog
        .revalidate_generation_inputs()
        .map_err(map_story_catalog_error)?;
    let current = gore_mod::pristine_script_cache(game_root).map_err(map_pristine_error)?;
    verify_snapshot(&current, &catalog.generation().shipping_cache)?;
    catalog
        .revalidate_generation_inputs()
        .map_err(map_story_catalog_error)
}

fn verify_snapshot(bytes: &[u8], expected: &ContentSeal) -> Result<(), Failure> {
    let byte_len = u64::try_from(bytes.len()).map_err(|_| input_limit_failure())?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    if byte_len != expected.byte_len || digest.as_slice() != expected.sha256.as_bytes() {
        return Err(input_changed_failure());
    }
    Ok(())
}

fn read_bounded_regular(path: &Path, max_bytes: u64) -> Result<Vec<u8>, Failure> {
    let mut file = open_regular_no_follow(path).map_err(map_input_io)?;
    let before = file.metadata().map_err(|_| input_unavailable_failure())?;
    if !before.is_file() {
        return Err(unsafe_input_failure());
    }
    if before.len() == 0 || before.len() > max_bytes {
        return Err(input_limit_failure());
    }
    let capacity = usize::try_from(before.len()).map_err(|_| input_limit_failure())?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut reader: Take<&mut File> = file.by_ref().take(max_bytes.saturating_add(1));
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| input_unavailable_failure())?;
    let after = file.metadata().map_err(|_| input_unavailable_failure())?;
    if bytes.len() as u64 != before.len()
        || bytes.len() as u64 > max_bytes
        || after.len() != before.len()
    {
        return Err(input_changed_failure());
    }
    Ok(bytes)
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path)?;
    use std::os::windows::fs::MetadataExt as _;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "reparse input rejected",
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(not(any(windows, unix)))]
fn open_regular_no_follow(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| response_limit_failure())?;
    if bytes.len() > limit {
        return Err(response_limit_failure());
    }
    Ok(response)
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_REQUEST_INVALID",
        "payload must contain exactly one non-empty bounded game_root path",
    )
}

fn catalog_build_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_BUILD_FAILED",
        "the closed NPC archetype catalog could not be built",
    )
}

fn catalog_limit_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_LIMIT",
        "the NPC archetype catalog exceeds its bounded resource budget",
    )
}

fn response_limit_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_RESPONSE_LIMIT",
        "the NPC archetype catalog response exceeds its bounded transport budget",
    )
}

fn input_unavailable_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_INPUT_UNAVAILABLE",
        "a required fixed native generation input is unavailable",
    )
}

fn input_limit_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_INPUT_LIMIT",
        "a fixed native generation input exceeds its resource limit",
    )
}

fn input_changed_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_INPUT_CHANGED",
        "the native game generation changed while the NPC catalog was being built",
    )
}

fn unsafe_input_failure() -> Failure {
    Failure::new(
        "AUTHORING_NPC_CATALOG_UNSAFE_INPUT",
        "a fixed native generation input is unsafe",
    )
}

fn map_input_io(error: io::Error) -> Failure {
    if error.kind() == io::ErrorKind::InvalidInput {
        unsafe_input_failure()
    } else {
        input_unavailable_failure()
    }
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_NPC_CATALOG_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before NPC catalog inspection",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return input_limit_failure();
    }
    if message.contains("not a regular non-link file") {
        return unsafe_input_failure();
    }
    Failure::new(
        "AUTHORING_NPC_CATALOG_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_story_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => {
            input_limit_failure()
        }
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => {
            unsafe_input_failure()
        }
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => {
            input_changed_failure()
        }
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_NPC_CATALOG_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        _ => input_unavailable_failure(),
    }
}

fn map_npc_catalog_error(error: NpcCatalogError) -> Failure {
    match error {
        NpcCatalogError::LimitExceeded { .. } => catalog_limit_failure(),
        NpcCatalogError::UnsupportedGeneration => Failure::new(
            "AUTHORING_NPC_CATALOG_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        NpcCatalogError::GenerationInputMismatch { .. } => input_changed_failure(),
        _ => catalog_build_failure(),
    }
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
    use gore_story_catalog::{known_generation_v1, known_generation_v2, Sha256Digest};
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn request_is_exact_bounded_and_cannot_select_provenance() {
        assert!(exact_payload(&json!({"game_root": "C:/Games/Gothic"})).is_ok());
        for invalid in [
            Value::Null,
            json!({}),
            json!({"game_root": ""}),
            json!({"game_root": 1}),
            json!({"game_root": "x\0y"}),
            json!({"game_root": "x".repeat(MAX_GAME_ROOT_BYTES + 1)}),
            json!({"game_root": "x", "shipping_cache": "client-choice"}),
            json!({"game_root": "x", "binds_cache": "client-choice"}),
            json!({"game_root": "x", "executable": "client-choice"}),
            json!({"game_root": "x", "catalog_json": "forged"}),
        ] {
            assert_eq!(
                build_for_game_root_v1(invalid)["error"]["code"],
                "AUTHORING_NPC_CATALOG_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn request_binding_is_domain_separated_and_covers_exact_root_bytes() {
        let root = "C:/Games/Gothic";
        let binding = request_binding(root);
        assert_eq!(binding.len(), 64);
        assert_ne!(binding, request_binding("C:/Games/gothic"));
        assert_ne!(binding, request_binding("C:/Games/Gothic/"));
        assert_ne!(binding, hex_digest(Sha256::digest(root.as_bytes())));
    }

    #[test]
    fn snapshot_drift_and_response_budget_fail_closed() {
        let bytes = b"pristine";
        let expected = ContentSeal {
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
        };
        verify_snapshot(bytes, &expected).unwrap();
        assert_eq!(
            verify_snapshot(b"drifted!", &expected).unwrap_err().code,
            "AUTHORING_NPC_CATALOG_INPUT_CHANGED"
        );
        assert_eq!(
            enforce_response_budget(json!({"ok": true, "large": "x".repeat(64)}), 8)
                .unwrap_err()
                .code,
            "AUTHORING_NPC_CATALOG_RESPONSE_LIMIT"
        );
    }

    #[test]
    fn recovery_and_missing_paths_are_sanitized() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("private-game-root");
        let script = game.join("G1R/Script");
        fs::create_dir_all(&script).unwrap();
        let live = script.join("PrecompiledScript_Shipping.Cache");
        let backup = PathBuf::from(format!("{}.gore-bak", live.display()));
        let pristine = b"pristine cache";
        let deployed = b"deployed cache";
        fs::write(&live, deployed).unwrap();
        fs::write(&backup, pristine).unwrap();
        let record = gore_mod::DeployRecord {
            mod_name: "fixture".to_owned(),
            phase: gore_mod::DeployPhase::RecoveryRequired,
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(live.display().to_string(), fnv1a64_hex(deployed))]),
            backup_hashes: BTreeMap::from([(
                backup.display().to_string(),
                format!("sha256:{}", hex_digest(Sha256::digest(pristine))),
            )]),
            ..Default::default()
        };
        fs::write(
            game.join("gore-mod.deployed.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        let failure = map_pristine_error(gore_mod::pristine_script_cache(&game).unwrap_err());
        assert_eq!(failure.code, "AUTHORING_NPC_CATALOG_RECOVERY_REQUIRED");

        let missing = game.join("missing-secret");
        let response = build_for_game_root_v1(json!({"game_root": missing.to_string_lossy()}));
        assert_eq!(response["ok"], false);
        assert!(!response
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn command_is_sorted_advertised_and_dispatches_without_path_leaks() {
        let info: Value = serde_json::from_str(&crate::execute_json(
            r#"{"command":"core_info","payload":{}}"#,
        ))
        .unwrap();
        let commands = info["commands"].as_array().unwrap();
        assert!(commands
            .windows(2)
            .all(|pair| pair[0].as_str() < pair[1].as_str()));
        assert!(commands.contains(&json!(
            "authoring_npc_archetype_catalog_v1_build_for_game_root"
        )));

        let private = "C:/private/missing-npc-game";
        let request = json!({
            "command": "authoring_npc_archetype_catalog_v1_build_for_game_root",
            "payload": {"game_root": private},
        });
        let response: Value =
            serde_json::from_str(&crate::execute_json(&request.to_string())).unwrap();
        assert_eq!(response["ok"], false);
        assert!(!response.to_string().contains(private));
    }

    #[test]
    #[ignore = "requires the explicitly configured pinned game generation"]
    fn configured_real_game_catalog_golden_is_stable() {
        let game_root = std::env::var("GORE_STORY_GAME_ROOT")
            .expect("set GORE_STORY_GAME_ROOT to the pinned game installation");
        let response =
            build_for_game_root_v1_inner(&json!({"game_root": game_root}), MAX_RESPONSE_BYTES)
                .expect("build pinned native NPC catalog");
        let generation = &response["generation"];
        let (source_pair_sha256, payload_sha256, catalog_sha256) =
            if generation == &json!(known_generation_v1()) {
                (
                    "aaeabcbee66bfd7402d88282827e76393fbbcb03d9a9e8f8f8eae4d38c056dd4",
                    "bc84dd8023a2df28e280e385e363748884fe5a49a94e78c990aacfe6271c6d7d",
                    "b7f1f08f1c10b38a461af45724d9e722c670e67cad49e00356851a85cda46ec1",
                )
            } else if generation == &json!(known_generation_v2()) {
                (
                    "42a6794e68610572f91ef1c41d5e8a661107fa689e7f0a41c65c302a784d665b",
                    "d11c6025e2ced4e376adb0ffdcafe8f5a7f9efd2ec6bf800f271be047b6fb9f8",
                    "342ec6bc1b1acefdd4f34ae652b141ee89a5e20d94679f10c3001b7e77f04946",
                )
            } else {
                panic!("real NPC golden returned an unregistered generation")
            };
        assert_eq!(response["ok"], true);
        assert_eq!(response["record_count"], 634);
        assert_eq!(response["rejection_count"], 416);
        assert_eq!(
            response["qualification"],
            json!({
                "linkage": "sealed_linkage_verified",
                "runtime": "runtime_unqualified",
                "build": "not_supported",
                "deploy": "not_supported",
                "publication": "not_supported",
            })
        );
        assert_eq!(response["source"]["source_pair_seal"]["byte_len"], 228);
        assert_eq!(
            response["source"]["source_pair_seal"]["sha256"],
            source_pair_sha256
        );
        assert_eq!(response["payload_seal"]["byte_len"], 1_806_762);
        assert_eq!(response["payload_seal"]["sha256"], payload_sha256);
        assert_eq!(response["catalog_seal"]["byte_len"], 1_807_892);
        assert_eq!(response["catalog_seal"]["sha256"], catalog_sha256);
        assert_eq!(response["catalog_json"].as_str().unwrap().len(), 1_808_069);
        assert!(serde_json::to_vec(&response).unwrap().len() <= MAX_RESPONSE_BYTES);
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
