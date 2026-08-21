//! Deterministic, qualified evidence for one standalone/game compiler output.
//!
//! A receipt is not a deployment capability. It binds the exact retained compiler-output handle
//! to a qualified compiler profile, its generation identity, exact source/base/Binds inputs, the
//! deterministic cache header and the explicit backend selection. Publication is atomic and
//! no-clobber; no game or installation path is opened by this module.

use std::collections::BTreeSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::cache::header::{CacheHeader, CACHE_MAGIC};
use crate::compile::CompileOutput;
use crate::compiler_backend::{
    CompilerBackendFailureKindV1, CompilerBackendFallbackReasonV1, CompilerBackendModeV1,
    CompilerBackendNameV1,
};
use crate::compiler_profile::manifest::{CompilerProfileV1, FileSealV1, Sha256Digest};
use crate::standalone_sidecar::ValidatedCompilerProfilePackageV1;

pub const GENERATION_RECEIPT_SCHEMA_V1: &str = "gore.as.generation-receipt";
pub const GENERATION_RECEIPT_VERSION_V1: u32 = 1;
/// Receipt V1 is intentionally pinned to the currently qualified cache writer/header.
/// A profile with a new BuildIdentifier requires a new parser and receipt schema version.
pub const GENERATION_RECEIPT_BUILD_IDENTIFIER_V1: u32 = CACHE_MAGIC;
pub const MAX_GENERATION_RECEIPT_JSON_BYTES_V1: usize = 256 * 1024;
pub const MAX_GENERATION_SOURCE_FILES_V1: usize = 4_096;
pub const MAX_GENERATION_SOURCE_FILE_BYTES_V1: usize = 16 * 1024 * 1024;
pub const MAX_GENERATION_SOURCE_BYTES_V1: usize = 256 * 1024 * 1024;
pub const MAX_GENERATION_OUTPUT_BYTES_V1: u64 = 512 * 1024 * 1024;

const MAX_SOURCE_PATH_BYTES_V1: usize = 4 * 1024;
const MAX_MODULE_NAME_BYTES_V1: usize = 4 * 1024;
const MAX_BACKEND_DETAIL_BYTES_V1: usize = 8 * 1024;
const RECEIPT_HASH_DOMAIN_V1: &[u8] = b"gore-as-generation-receipt-v1\0";
const SOURCE_TREE_HASH_DOMAIN_V1: &[u8] = b"gore-as-generation-source-tree-v1\0";
const TEMP_PREFIX: &str = ".gore-as-receipt-v1-tmp-";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactSealV1 {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

impl ArtifactSealV1 {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
        }
    }

    fn matches_profile_file(&self, expected: &FileSealV1) -> bool {
        self.byte_len == expected.byte_len && self.sha256 == expected.sha256
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationSourceFileV1<'a> {
    pub relative_path: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSourceTreeSealV1 {
    pub file_count: u32,
    pub byte_len: u64,
    pub canonical_sha256: Sha256Digest,
}

impl GenerationSourceTreeSealV1 {
    pub fn from_files(
        files: &[GenerationSourceFileV1<'_>],
    ) -> Result<Self, GenerationReceiptError> {
        if files.is_empty() {
            return invalid(
                "inputs.source_tree",
                "must contain at least one authored source",
            );
        }
        check_count(
            "inputs.source_tree.files",
            files.len(),
            MAX_GENERATION_SOURCE_FILES_V1,
        )?;
        let mut normalized = Vec::with_capacity(files.len());
        let mut identities = BTreeSet::new();
        let mut total = 0usize;
        for file in files {
            let path = validate_source_path(file.relative_path)?;
            if file.bytes.len() > MAX_GENERATION_SOURCE_FILE_BYTES_V1 {
                return Err(GenerationReceiptError::InputTooLarge {
                    field: "source file",
                    actual: file.bytes.len() as u64,
                    max: MAX_GENERATION_SOURCE_FILE_BYTES_V1 as u64,
                });
            }
            total = total.checked_add(file.bytes.len()).ok_or_else(|| {
                GenerationReceiptError::InputTooLarge {
                    field: "source tree",
                    actual: u64::MAX,
                    max: MAX_GENERATION_SOURCE_BYTES_V1 as u64,
                }
            })?;
            if total > MAX_GENERATION_SOURCE_BYTES_V1 {
                return Err(GenerationReceiptError::InputTooLarge {
                    field: "source tree",
                    actual: total as u64,
                    max: MAX_GENERATION_SOURCE_BYTES_V1 as u64,
                });
            }
            let identity = path.to_ascii_lowercase();
            if !identities.insert(identity) {
                return invalid(
                    "inputs.source_tree.files",
                    "contains a duplicate Windows path identity",
                );
            }
            normalized.push((path, file.bytes));
        }
        normalized.sort_by(|left, right| left.0.cmp(&right.0));

        let mut hash = Sha256::new();
        hash.update(SOURCE_TREE_HASH_DOMAIN_V1);
        hash.update((normalized.len() as u64).to_le_bytes());
        for (path, bytes) in normalized {
            hash.update((path.len() as u64).to_le_bytes());
            hash.update(path.as_bytes());
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
        Ok(Self {
            file_count: files.len() as u32,
            byte_len: total as u64,
            canonical_sha256: Sha256Digest::from_bytes(hash.finalize().into()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedProfileIdentityV1 {
    pub profile_schema_version: u32,
    pub profile_sha256: Sha256Digest,
    pub steam_app_id: u32,
    pub steam_build_id: u64,
    pub depot_id: u32,
    pub depot_manifest_gid: u64,
    pub executable_codeview_guid: String,
    pub executable_codeview_age: u32,
}

impl QualifiedProfileIdentityV1 {
    fn from_profile(profile: &CompilerProfileV1) -> Self {
        Self {
            profile_schema_version: profile.schema_version,
            profile_sha256: profile.profile_sha256,
            steam_app_id: profile.target.steam_app_id,
            steam_build_id: profile.target.steam_build_id,
            depot_id: profile.target.depot_id,
            depot_manifest_gid: profile.target.depot_manifest_gid,
            executable_codeview_guid: profile.oracle.pe_codeview.guid.clone(),
            executable_codeview_age: profile.oracle.pe_codeview.age,
        }
    }

    fn matches_profile(&self, profile: &CompilerProfileV1) -> bool {
        self == &Self::from_profile(profile)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptBackendModeV1 {
    Standalone,
    Game,
    StandaloneThenGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptBackendNameV1 {
    Standalone,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptBackendFailureKindV1 {
    Preflight,
    Unavailable,
    Unsupported,
    Rejected,
    InvalidOutput,
    Internal,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptFallbackReasonV1 {
    pub failed_backend: ReceiptBackendNameV1,
    pub failure_kind: ReceiptBackendFailureKindV1,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptBackendSelectionV1 {
    pub requested_mode: ReceiptBackendModeV1,
    pub used_backend: ReceiptBackendNameV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<ReceiptFallbackReasonV1>,
}

impl ReceiptBackendSelectionV1 {
    pub fn from_compile_selection(
        requested_mode: CompilerBackendModeV1,
        used_backend: CompilerBackendNameV1,
        fallback_reason: Option<&CompilerBackendFallbackReasonV1>,
    ) -> Result<Self, GenerationReceiptError> {
        let selection = Self {
            requested_mode: requested_mode.into(),
            used_backend: used_backend.into(),
            fallback_reason: fallback_reason.map(|reason| ReceiptFallbackReasonV1 {
                failed_backend: reason.failed_backend().into(),
                failure_kind: reason.failure_kind().into(),
                detail: reason.detail().to_owned(),
            }),
        };
        selection.validate()?;
        Ok(selection)
    }

    fn validate(&self) -> Result<(), GenerationReceiptError> {
        if let Some(reason) = &self.fallback_reason {
            validate_bounded_text(
                "backend.fallback_reason.detail",
                &reason.detail,
                MAX_BACKEND_DETAIL_BYTES_V1,
            )?;
            if reason.failed_backend != ReceiptBackendNameV1::Standalone {
                return invalid(
                    "backend.fallback_reason.failed_backend",
                    "V1 fallback can only originate from standalone",
                );
            }
        }
        let valid = match (
            self.requested_mode,
            self.used_backend,
            &self.fallback_reason,
        ) {
            (ReceiptBackendModeV1::Standalone, ReceiptBackendNameV1::Standalone, None)
            | (ReceiptBackendModeV1::Game, ReceiptBackendNameV1::Game, None)
            | (ReceiptBackendModeV1::StandaloneThenGame, ReceiptBackendNameV1::Standalone, None)
            | (ReceiptBackendModeV1::StandaloneThenGame, ReceiptBackendNameV1::Game, Some(_)) => {
                true
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            invalid(
                "backend",
                "used backend and fallback do not match the explicit requested mode",
            )
        }
    }
}

impl From<CompilerBackendModeV1> for ReceiptBackendModeV1 {
    fn from(value: CompilerBackendModeV1) -> Self {
        match value {
            CompilerBackendModeV1::Standalone => Self::Standalone,
            CompilerBackendModeV1::Game => Self::Game,
            CompilerBackendModeV1::StandaloneThenGame => Self::StandaloneThenGame,
        }
    }
}

impl From<CompilerBackendNameV1> for ReceiptBackendNameV1 {
    fn from(value: CompilerBackendNameV1) -> Self {
        match value {
            CompilerBackendNameV1::Standalone => Self::Standalone,
            CompilerBackendNameV1::Game => Self::Game,
        }
    }
}

impl From<CompilerBackendFailureKindV1> for ReceiptBackendFailureKindV1 {
    fn from(value: CompilerBackendFailureKindV1) -> Self {
        match value {
            CompilerBackendFailureKindV1::Preflight => Self::Preflight,
            CompilerBackendFailureKindV1::Unavailable => Self::Unavailable,
            CompilerBackendFailureKindV1::Unsupported => Self::Unsupported,
            CompilerBackendFailureKindV1::Rejected => Self::Rejected,
            CompilerBackendFailureKindV1::InvalidOutput => Self::InvalidOutput,
            CompilerBackendFailureKindV1::Internal => Self::Internal,
            CompilerBackendFailureKindV1::RecoveryRequired => Self::RecoveryRequired,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationInputsV1 {
    pub source_tree: GenerationSourceTreeSealV1,
    pub base_cache: ArtifactSealV1,
    pub binds_cache: ArtifactSealV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationOutputV1 {
    pub artifact: ArtifactSealV1,
    pub cache_guid: String,
    pub build_identifier: u32,
    pub module_count: u32,
    pub module_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationReceiptV1 {
    pub schema: String,
    pub schema_version: u32,
    pub qualified_profile: QualifiedProfileIdentityV1,
    pub inputs: GenerationInputsV1,
    pub output: GenerationOutputV1,
    pub backend: ReceiptBackendSelectionV1,
    pub canonical_sha256: Sha256Digest,
}

impl GenerationReceiptV1 {
    pub fn build_from_bytes(
        profile_package: &ValidatedCompilerProfilePackageV1,
        sources: &[GenerationSourceFileV1<'_>],
        base_cache: &[u8],
        binds_cache: &[u8],
        output_cache: &[u8],
        module_name: &str,
        backend: ReceiptBackendSelectionV1,
    ) -> Result<Self, GenerationReceiptError> {
        Self::build_from_profile_bytes(
            profile_package.profile(),
            sources,
            base_cache,
            binds_cache,
            output_cache,
            module_name,
            backend,
        )
    }

    fn build_from_profile_bytes(
        profile: &CompilerProfileV1,
        sources: &[GenerationSourceFileV1<'_>],
        base_cache: &[u8],
        binds_cache: &[u8],
        output_cache: &[u8],
        module_name: &str,
        backend: ReceiptBackendSelectionV1,
    ) -> Result<Self, GenerationReceiptError> {
        if output_cache.len() as u64 > MAX_GENERATION_OUTPUT_BYTES_V1 {
            return Err(GenerationReceiptError::InputTooLarge {
                field: "output cache",
                actual: output_cache.len() as u64,
                max: MAX_GENERATION_OUTPUT_BYTES_V1,
            });
        }
        profile
            .validate_complete()
            .map_err(|error| GenerationReceiptError::Profile(error.to_string()))?;
        if profile.cache_writer.build_identifier != GENERATION_RECEIPT_BUILD_IDENTIFIER_V1 {
            return invalid(
                "qualified_profile.cache_writer.build_identifier",
                "receipt V1 supports only the current 0x9e377abe cache header",
            );
        }
        backend.validate()?;
        validate_bounded_text("output.module_name", module_name, MAX_MODULE_NAME_BYTES_V1)?;
        let base_header = CacheHeader::parse(base_cache).map_err(|error| {
            GenerationReceiptError::CacheHeader {
                field: "base cache",
                reason: error.to_string(),
            }
        })?;
        let output_header = CacheHeader::parse(output_cache).map_err(|error| {
            GenerationReceiptError::CacheHeader {
                field: "output cache",
                reason: error.to_string(),
            }
        })?;
        let base_seal = ArtifactSealV1::from_bytes(base_cache);
        let binds_seal = ArtifactSealV1::from_bytes(binds_cache);
        if !base_seal.matches_profile_file(&profile.oracle.shipping_cache) {
            return invalid(
                "inputs.base_cache",
                "does not match the qualified profile's Shipping cache",
            );
        }
        if !binds_seal.matches_profile_file(&profile.oracle.binds_cache) {
            return invalid(
                "inputs.binds_cache",
                "does not match the qualified profile's Binds.Cache",
            );
        }
        if base_header.hash != output_header.hash {
            return invalid(
                "output.cache_guid",
                "is not the deterministic base-cache GUID",
            );
        }
        if base_header.magic != output_header.magic
            || output_header.magic != profile.cache_writer.build_identifier
        {
            return invalid(
                "output.build_identifier",
                "does not match both the base cache and qualified writer profile",
            );
        }
        let mut receipt = Self {
            schema: GENERATION_RECEIPT_SCHEMA_V1.to_owned(),
            schema_version: GENERATION_RECEIPT_VERSION_V1,
            qualified_profile: QualifiedProfileIdentityV1::from_profile(profile),
            inputs: GenerationInputsV1 {
                source_tree: GenerationSourceTreeSealV1::from_files(sources)?,
                base_cache: base_seal,
                binds_cache: binds_seal,
            },
            output: GenerationOutputV1 {
                artifact: ArtifactSealV1::from_bytes(output_cache),
                cache_guid: hex_lower(&output_header.hash),
                build_identifier: output_header.magic,
                module_count: output_header.type_count,
                module_name: module_name.to_owned(),
            },
            backend,
            canonical_sha256: zero_digest(),
        };
        receipt.canonical_sha256 = receipt.computed_digest()?;
        receipt.validate_against_profile(
            profile,
            sources,
            base_cache,
            binds_cache,
            output_cache,
        )?;
        Ok(receipt)
    }

    pub fn build_for_compile_output(
        profile_package: &ValidatedCompilerProfilePackageV1,
        sources: &[GenerationSourceFileV1<'_>],
        base_cache: &[u8],
        binds_cache: &[u8],
        output: &CompileOutput,
        backend: ReceiptBackendSelectionV1,
    ) -> Result<Self, GenerationReceiptError> {
        Self::build_for_compile_output_profile(
            profile_package.profile(),
            sources,
            base_cache,
            binds_cache,
            output,
            backend,
        )
    }

    fn build_for_compile_output_profile(
        profile: &CompilerProfileV1,
        sources: &[GenerationSourceFileV1<'_>],
        base_cache: &[u8],
        binds_cache: &[u8],
        output: &CompileOutput,
        backend: ReceiptBackendSelectionV1,
    ) -> Result<Self, GenerationReceiptError> {
        let output_bytes = read_retained_output(output)?;
        Self::build_from_profile_bytes(
            profile,
            sources,
            base_cache,
            binds_cache,
            &output_bytes,
            &output.module_name,
            backend,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, GenerationReceiptError> {
        if bytes.len() > MAX_GENERATION_RECEIPT_JSON_BYTES_V1 {
            return Err(GenerationReceiptError::InputTooLarge {
                field: "generation receipt JSON",
                actual: bytes.len() as u64,
                max: MAX_GENERATION_RECEIPT_JSON_BYTES_V1 as u64,
            });
        }
        let receipt: Self = serde_json::from_slice(bytes)?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, GenerationReceiptError> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() > MAX_GENERATION_RECEIPT_JSON_BYTES_V1 {
            return Err(GenerationReceiptError::InputTooLarge {
                field: "generation receipt JSON",
                actual: bytes.len() as u64,
                max: MAX_GENERATION_RECEIPT_JSON_BYTES_V1 as u64,
            });
        }
        Ok(bytes)
    }

    /// Validate the receipt's strict schema and canonical self-integrity.
    ///
    /// This does not establish compiler qualification authority. Consumers that need to trust the
    /// qualified-profile claim must call [`Self::validate_against`] with an opaque package loaded
    /// by [`ValidatedCompilerProfilePackageV1::load`].
    pub fn validate(&self) -> Result<(), GenerationReceiptError> {
        if self.schema != GENERATION_RECEIPT_SCHEMA_V1
            || self.schema_version != GENERATION_RECEIPT_VERSION_V1
        {
            return Err(GenerationReceiptError::Schema {
                actual: format!("{}@{}", self.schema, self.schema_version),
            });
        }
        self.backend.validate()?;
        validate_bounded_text(
            "output.module_name",
            &self.output.module_name,
            MAX_MODULE_NAME_BYTES_V1,
        )?;
        parse_guid_hex(&self.output.cache_guid)?;
        let computed = self.computed_digest()?;
        if computed != self.canonical_sha256 {
            return Err(GenerationReceiptError::DigestMismatch {
                expected: self.canonical_sha256,
                actual: computed,
            });
        }
        Ok(())
    }

    pub fn validate_against(
        &self,
        profile_package: &ValidatedCompilerProfilePackageV1,
        sources: &[GenerationSourceFileV1<'_>],
        base_cache: &[u8],
        binds_cache: &[u8],
        output_cache: &[u8],
    ) -> Result<(), GenerationReceiptError> {
        self.validate_against_profile(
            profile_package.profile(),
            sources,
            base_cache,
            binds_cache,
            output_cache,
        )
    }

    fn validate_against_profile(
        &self,
        profile: &CompilerProfileV1,
        sources: &[GenerationSourceFileV1<'_>],
        base_cache: &[u8],
        binds_cache: &[u8],
        output_cache: &[u8],
    ) -> Result<(), GenerationReceiptError> {
        self.validate()?;
        profile
            .validate_complete()
            .map_err(|error| GenerationReceiptError::Profile(error.to_string()))?;
        if !self.qualified_profile.matches_profile(profile) {
            return invalid(
                "qualified_profile",
                "does not match the exact qualified compiler profile",
            );
        }
        if self.inputs.source_tree != GenerationSourceTreeSealV1::from_files(sources)?
            || self.inputs.base_cache != ArtifactSealV1::from_bytes(base_cache)
            || self.inputs.binds_cache != ArtifactSealV1::from_bytes(binds_cache)
            || self.output.artifact != ArtifactSealV1::from_bytes(output_cache)
        {
            return invalid("artifacts", "one or more exact artifact seals changed");
        }
        if !self
            .inputs
            .base_cache
            .matches_profile_file(&profile.oracle.shipping_cache)
            || !self
                .inputs
                .binds_cache
                .matches_profile_file(&profile.oracle.binds_cache)
        {
            return invalid(
                "inputs",
                "base/Binds seals do not belong to the qualified profile",
            );
        }
        let base_header = CacheHeader::parse(base_cache).map_err(|error| {
            GenerationReceiptError::CacheHeader {
                field: "base cache",
                reason: error.to_string(),
            }
        })?;
        let output_header = CacheHeader::parse(output_cache).map_err(|error| {
            GenerationReceiptError::CacheHeader {
                field: "output cache",
                reason: error.to_string(),
            }
        })?;
        if base_header.hash != output_header.hash
            || self.output.cache_guid != hex_lower(&base_header.hash)
        {
            return invalid(
                "output.cache_guid",
                "does not match the deterministic base-cache GUID",
            );
        }
        if base_header.magic != output_header.magic
            || self.output.build_identifier != output_header.magic
            || output_header.magic != profile.cache_writer.build_identifier
        {
            return invalid(
                "output.build_identifier",
                "does not match base, output and qualified writer profile",
            );
        }
        if self.output.module_count != output_header.type_count {
            return invalid(
                "output.module_count",
                "does not match the output cache header",
            );
        }
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, GenerationReceiptError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        let bytes = serde_json::to_vec(&canonical)?;
        let mut hash = Sha256::new();
        hash.update(RECEIPT_HASH_DOMAIN_V1);
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
        Ok(Sha256Digest::from_bytes(hash.finalize().into()))
    }
}

/// Publish a fully validated receipt with atomic visibility and no overwrite semantics.
///
/// The hard-link publication step is atomic and fails if `destination` already exists. The
/// same-directory temporary file is fully written and synced before it becomes visible there.
pub fn publish_generation_receipt_v1(
    destination: &Path,
    receipt: &GenerationReceiptV1,
) -> Result<PathBuf, GenerationReceiptError> {
    let bytes = receipt.to_json()?;
    publish_bytes_atomic_no_clobber_v1(
        destination,
        &bytes,
        MAX_GENERATION_RECEIPT_JSON_BYTES_V1 as u64,
        TEMP_PREFIX,
        "generation receipt",
    )
}

/// Publish exact compiler-output bytes with atomic visibility and no overwrite semantics.
pub fn publish_generation_output_v1(
    destination: &Path,
    bytes: &[u8],
) -> Result<PathBuf, GenerationReceiptError> {
    if bytes.len() as u64 > MAX_GENERATION_OUTPUT_BYTES_V1 {
        return Err(GenerationReceiptError::InputTooLarge {
            field: "generation output",
            actual: bytes.len() as u64,
            max: MAX_GENERATION_OUTPUT_BYTES_V1,
        });
    }
    publish_bytes_atomic_no_clobber_v1(
        destination,
        bytes,
        MAX_GENERATION_OUTPUT_BYTES_V1,
        ".gore-as-output-v1-tmp-",
        "generation output",
    )
}

fn publish_bytes_atomic_no_clobber_v1(
    destination: &Path,
    bytes: &[u8],
    max_bytes: u64,
    temp_prefix: &str,
    label: &'static str,
) -> Result<PathBuf, GenerationReceiptError> {
    let file_name = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or(GenerationReceiptError::UnsafePublicationPath)?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    validate_publication_parent(parent)?;
    match std::fs::symlink_metadata(destination) {
        Ok(_) => {
            return Err(GenerationReceiptError::AlreadyExists(
                destination.to_path_buf(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(GenerationReceiptError::Publication(format!(
                "inspecting {label} destination: {error}"
            )));
        }
    }

    let mut last_collision = None;
    for _ in 0..128 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            "{temp_prefix}{}-{sequence}-{}",
            std::process::id(),
            file_name.to_string_lossy()
        ));
        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_collision = Some(error);
                continue;
            }
            Err(error) => {
                return Err(GenerationReceiptError::Publication(format!(
                    "creating {label} staging file: {error}"
                )));
            }
        };
        if let Err(error) = validate_regular_non_reparse(&file)
            .and_then(|_| file.write_all(&bytes).map_err(|error| error.to_string()))
            .and_then(|_| file.sync_all().map_err(|error| error.to_string()))
        {
            drop(file);
            let _ = std::fs::remove_file(&temp);
            return Err(GenerationReceiptError::Publication(format!(
                "initializing {label} staging file: {error}"
            )));
        }
        drop(file);
        match std::fs::hard_link(&temp, destination) {
            Ok(()) => {
                let final_result = (|| {
                    let final_file = open_regular_read_no_follow(destination)?;
                    validate_regular_non_reparse(&final_file)?;
                    let final_bytes = read_bounded_file(final_file, max_bytes, label)?;
                    if final_bytes != bytes {
                        return Err("published receipt bytes changed".to_owned());
                    }
                    Ok(())
                })();
                let cleanup = std::fs::remove_file(&temp);
                if let Err(error) = final_result {
                    return Err(GenerationReceiptError::PublicationUncertain {
                        path: destination.to_path_buf(),
                        reason: error,
                    });
                }
                if let Err(error) = cleanup {
                    return Err(GenerationReceiptError::PublicationUncertain {
                        path: destination.to_path_buf(),
                        reason: format!("removing {label} staging link: {error}"),
                    });
                }
                sync_parent_directory(parent).map_err(|reason| {
                    GenerationReceiptError::PublicationUncertain {
                        path: destination.to_path_buf(),
                        reason,
                    }
                })?;
                return Ok(destination.to_path_buf());
            }
            Err(error) => {
                let _ = std::fs::remove_file(&temp);
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(GenerationReceiptError::AlreadyExists(
                        destination.to_path_buf(),
                    ));
                }
                return Err(GenerationReceiptError::Publication(format!(
                    "atomically linking {label} into place: {error}"
                )));
            }
        }
    }
    Err(GenerationReceiptError::Publication(format!(
        "could not reserve a {label} staging name: {}",
        last_collision
            .map(|error| error.to_string())
            .unwrap_or_else(|| "name space exhausted".to_owned())
    )))
}

pub fn read_generation_receipt_v1(
    path: &Path,
) -> Result<GenerationReceiptV1, GenerationReceiptError> {
    let file = open_regular_read_no_follow(path).map_err(GenerationReceiptError::Publication)?;
    let bytes = read_bounded_file(
        file,
        MAX_GENERATION_RECEIPT_JSON_BYTES_V1 as u64,
        "generation receipt",
    )
    .map_err(GenerationReceiptError::Publication)?;
    GenerationReceiptV1::from_json(&bytes)
}

/// Read the exact retained compiler output through its create-new handle.
///
/// Product callers use this for a final copy after the same handle has been sealed into a
/// receipt. It never reopens `CompileOutput::mini_path`.
pub fn read_compile_output_bytes_v1(
    output: &CompileOutput,
) -> Result<Vec<u8>, GenerationReceiptError> {
    read_retained_output(output)
}

fn read_retained_output(output: &CompileOutput) -> Result<Vec<u8>, GenerationReceiptError> {
    let file = output
        .clone_retained_artifact_file()
        .map_err(GenerationReceiptError::RetainedOutput)?;
    read_bounded_file(
        file,
        MAX_GENERATION_OUTPUT_BYTES_V1,
        "retained compiler output",
    )
    .map_err(GenerationReceiptError::RetainedOutput)
}

fn read_bounded_file(
    mut file: std::fs::File,
    max: u64,
    label: &'static str,
) -> Result<Vec<u8>, String> {
    validate_regular_non_reparse(&file)?;
    let before = file
        .metadata()
        .map_err(|error| format!("inspecting {label}: {error}"))?;
    if before.len() > max {
        return Err(format!(
            "{label} is {} bytes; maximum is {max}",
            before.len()
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("seeking {label}: {error}"))?;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&mut file)
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("reading {label}: {error}"))?;
    if bytes.len() as u64 > max {
        return Err(format!("{label} exceeded maximum {max} while reading"));
    }
    let after = file
        .metadata()
        .map_err(|error| format!("rechecking {label}: {error}"))?;
    if before.len() != after.len() || after.len() != bytes.len() as u64 {
        return Err(format!("{label} changed while it was being sealed"));
    }
    Ok(bytes)
}

fn validate_publication_parent(parent: &Path) -> Result<(), GenerationReceiptError> {
    let metadata = std::fs::symlink_metadata(parent).map_err(|error| {
        GenerationReceiptError::Publication(format!("inspecting receipt parent: {error}"))
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) {
        return Err(GenerationReceiptError::UnsafePublicationPath);
    }
    Ok(())
}

fn validate_regular_non_reparse(file: &std::fs::File) -> Result<(), String> {
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspecting file handle: {error}"))?;
    if !metadata.is_file() || metadata_is_reparse(&metadata) {
        return Err("file handle is not a regular non-reparse file".to_owned());
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(_: &std::fs::Metadata) -> bool {
    false
}

#[cfg(windows)]
fn open_regular_read_no_follow(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path).map_err(|error| error.to_string())?;
    validate_regular_non_reparse(&file)?;
    Ok(file)
}

#[cfg(unix)]
fn open_regular_read_no_follow(path: &Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options.open(path).map_err(|error| error.to_string())?;
    validate_regular_non_reparse(&file)?;
    Ok(file)
}

#[cfg(not(any(windows, unix)))]
fn open_regular_read_no_follow(path: &Path) -> Result<std::fs::File, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("path is not a regular file".to_owned());
    }
    std::fs::File::open(path).map_err(|error| error.to_string())
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> Result<(), String> {
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("syncing receipt parent directory: {error}"))
}

#[cfg(not(unix))]
fn sync_parent_directory(_: &Path) -> Result<(), String> {
    Ok(())
}

fn validate_source_path(value: &str) -> Result<String, GenerationReceiptError> {
    if value.is_empty()
        || value.len() > MAX_SOURCE_PATH_BYTES_V1
        || value.contains('\\')
        || value.contains('\0')
        || value.chars().any(|character| character.is_control())
    {
        return invalid(
            "inputs.source_tree.relative_path",
            "must be bounded slash-separated UTF-8 without control characters",
        );
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return invalid(
            "inputs.source_tree.relative_path",
            "must contain only relative normal components",
        );
    }
    Ok(value.to_owned())
}

fn validate_bounded_text(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), GenerationReceiptError> {
    if value.is_empty()
        || value.len() > max
        || value.contains('\0')
        || value.chars().any(|character| character.is_control())
    {
        invalid(
            field,
            "must be non-empty bounded text without control characters",
        )
    } else {
        Ok(())
    }
}

fn parse_guid_hex(value: &str) -> Result<[u8; 16], GenerationReceiptError> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(
            "output.cache_guid",
            "must be exactly 32 lowercase hex characters",
        );
    }
    let mut out = [0u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .expect("validated lowercase hex");
    }
    Ok(out)
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(text, "{byte:02x}").expect("writing to String cannot fail");
    }
    text
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

fn check_count(
    field: &'static str,
    actual: usize,
    max: usize,
) -> Result<(), GenerationReceiptError> {
    if actual > max {
        Err(GenerationReceiptError::CountTooLarge { field, actual, max })
    } else {
        Ok(())
    }
}

fn invalid<T>(field: &'static str, reason: &'static str) -> Result<T, GenerationReceiptError> {
    Err(GenerationReceiptError::Invalid {
        field,
        reason: reason.to_owned(),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationReceiptError {
    #[error(
        "generation receipt schema mismatch: expected gore.as.generation-receipt@1, got {actual}"
    )]
    Schema { actual: String },
    #[error("qualified compiler profile is invalid: {0}")]
    Profile(String),
    #[error("{field} is invalid: {reason}")]
    Invalid { field: &'static str, reason: String },
    #[error("{field} count {actual} exceeds maximum {max}")]
    CountTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("{field} is {actual} bytes; maximum accepted size is {max}")]
    InputTooLarge {
        field: &'static str,
        actual: u64,
        max: u64,
    },
    #[error("invalid {field} header: {reason}")]
    CacheHeader { field: &'static str, reason: String },
    #[error("receipt digest mismatch: expected {expected}, got {actual}")]
    DigestMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("retained compiler output cannot be sealed: {0}")]
    RetainedOutput(String),
    #[error("generation receipt JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("receipt publication path is unsafe")]
    UnsafePublicationPath,
    #[error("receipt destination already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("receipt publication failed: {0}")]
    Publication(String),
    #[error("receipt publication at {path} is uncertain: {reason}")]
    PublicationUncertain { path: PathBuf, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::header::CACHE_MAGIC;
    use crate::compiler_profile::frontend::{
        ClassGeneratorConfigV1, CompilerOptionsV1, EffectivePreprocessorFlagV1,
        PreprocessorConfigV1, PropertyBlueprintSpecifierV1, PropertyEditSpecifierV1,
        StaticClassModeV1, CLASS_GENERATOR_CONFIG_SCHEMA, COMPILER_OPTIONS_SCHEMA,
        FRONTEND_SCHEMA_VERSION, PREPROCESSOR_CONFIG_SCHEMA,
    };
    use crate::compiler_profile::manifest::{
        BindsProfileV1, BytecodeProfileV1, CacheWriterProfileV1, CompilerArchitectureV1,
        CompilerBuildConfigurationV1, CompilerOracleV1, CompilerPlatformV1, CompilerTargetV1,
        EngineProfileV1, FrontendProfileV1, PeCodeViewV1, QualificationProfileV1, SealedBlobV1,
        Sha1Digest, UnrealSemanticsProfileV1, COMPILER_PROFILE_SCHEMA,
        COMPILER_PROFILE_SCHEMA_VERSION,
    };
    use crate::compiler_profile::qualification::{
        CompilerProbeCaseV1, CompilerProbeCorpusV1, DiagnosticParityEntryV1,
        DiagnosticParityReportV1, ExpectedProbeResultV1, ExpectedProbeResultsV1, ProbeModeV1,
        ProbeOutcomeV1, ProbeSourceSectionV1, SemanticParityEntryV1, SemanticParityReportV1,
        DIAGNOSTIC_PARITY_SCHEMA, EXPECTED_RESULTS_SCHEMA, PROBE_CORPUS_SCHEMA,
        QUALIFICATION_SCHEMA_VERSION, SEMANTIC_PARITY_SCHEMA,
    };
    use crate::compiler_profile::registry::{
        DynamicScriptTypeOperationsV1, EnginePropertySettingV1, EnginePropertyV1,
        FixedTypeOperationsV1, OrderedEnginePropertiesV1, PostBindEntryV1, PostBindResultV1,
        PostBindSnapshotV1, PrimitiveTypeOperationsV1, PrimitiveTypeV1, RegistrationContextV1,
        RegistrationEntryV1, RegistrationTraceV1, TypeOperationsV1, ENGINE_PROPERTIES_SCHEMA,
        POST_BIND_SNAPSHOT_SCHEMA, REGISTRATION_TRACE_SCHEMA,
    };

    fn cache(guid: [u8; 16], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&guid);
        bytes.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn file_seal(bytes: &[u8], steam: bool) -> FileSealV1 {
        FileSealV1 {
            byte_len: bytes.len() as u64,
            sha256: ArtifactSealV1::from_bytes(bytes).sha256,
            steam_content_sha1: steam.then(|| Sha1Digest::from_bytes([0x5a; 20])),
        }
    }

    fn qualified_profile_manifest(base: &[u8], binds: &[u8]) -> CompilerProfileV1 {
        let blob = |path: &str, byte: u8| SealedBlobV1 {
            path: path.to_owned(),
            byte_len: u64::from(byte) + 1,
            sha256: Sha256Digest::from_bytes([byte; 32]),
        };
        let mut profile = CompilerProfileV1 {
            schema: COMPILER_PROFILE_SCHEMA.to_owned(),
            schema_version: COMPILER_PROFILE_SCHEMA_VERSION,
            target: CompilerTargetV1 {
                steam_app_id: 1_297_900,
                steam_build_id: 24_539_464,
                depot_id: 1_297_901,
                depot_manifest_gid: 1_585_071_322_101_748_861,
                platform: CompilerPlatformV1::Windows,
                architecture: CompilerArchitectureV1::X86_64,
                build_configuration: CompilerBuildConfigurationV1::Shipping,
            },
            oracle: CompilerOracleV1 {
                executable: file_seal(b"shipping-exe", true),
                binds_cache: file_seal(binds, true),
                shipping_cache: file_seal(base, true),
                depot_manifest: file_seal(b"depot-manifest", false),
                pe_codeview: PeCodeViewV1 {
                    guid: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
                    age: 1,
                },
            },
            binds: BindsProfileV1 {
                wire_schema_version: 1,
                struct_count: 1,
                class_count: 1,
                method_count: 1,
                struct_property_count: 1,
                class_property_count: 1,
                canonical_database_sha256: Sha256Digest::from_bytes([5; 32]),
            },
            engine: EngineProfileV1 {
                as_create_version: 23_300,
                ordered_engine_properties: blob("engine/properties.json", 6),
                registration_trace: blob("engine/registrations.json", 7),
                registration_trace_count: 1,
                post_bind_snapshot: blob("engine/post-bind.json", 8),
            },
            unreal_semantics: UnrealSemanticsProfileV1 {
                reflected_type_graph: blob("unreal/type-graph.json", 9),
                metadata_schema_version: 1,
            },
            frontend: FrontendProfileV1 {
                preprocessor_config: blob("frontend/preprocessor.json", 10),
                class_generator_config: blob("frontend/class-generator.json", 11),
                compiler_options: blob("frontend/options.json", 12),
            },
            bytecode: BytecodeProfileV1 {
                opcode_table_version: "g1r-as-opcodes-v1".to_owned(),
                opcode_table: blob("bytecode/opcodes.json", 13),
                operand_schema: blob("bytecode/operands.json", 14),
                codegen_probe_corpus: blob("bytecode/probes.json", 15),
                expected_probe_results: blob("bytecode/probe-results.bin", 16),
            },
            cache_writer: CacheWriterProfileV1 {
                format_version: 1,
                serializer_schema: blob("cache/serializer.json", 17),
                build_identifier: CACHE_MAGIC,
                reference_table_order: blob("cache/reference-order.json", 18),
                normalized_oracle_corpus: blob("cache/oracle-corpus.json", 19),
            },
            qualification: QualificationProfileV1 {
                required_probe_suite_version: "g1r-compiler-parity-v1".to_owned(),
                diagnostic_parity: blob("qualification/diagnostics.json", 20),
                semantic_parity: blob("qualification/semantic.json", 21),
                qualified: true,
            },
            profile_sha256: zero_digest(),
        };
        profile.seal().unwrap();
        profile
    }

    struct QualifiedFixture {
        root: PathBuf,
        package: ValidatedCompilerProfilePackageV1,
    }

    impl QualifiedFixture {
        fn create(label: &str, base: &[u8], binds: &[u8]) -> Self {
            let root = unique_root(label);
            let profile_root = root.join("profile");
            std::fs::create_dir_all(&profile_root).unwrap();

            let write_blob = |path: &str, bytes: &[u8]| {
                let destination = profile_root.join(path);
                std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
                std::fs::write(&destination, bytes).unwrap();
                SealedBlobV1 {
                    path: path.to_owned(),
                    byte_len: bytes.len() as u64,
                    sha256: ArtifactSealV1::from_bytes(bytes).sha256,
                }
            };

            let mut properties = OrderedEnginePropertiesV1 {
                schema: ENGINE_PROPERTIES_SCHEMA.to_owned(),
                schema_version: 1,
                settings: vec![EnginePropertySettingV1 {
                    ordinal: 0,
                    property: EnginePropertyV1::OptimizeBytecode,
                    value: 1,
                }],
                canonical_sha256: zero_digest(),
            };
            properties.seal().unwrap();
            let properties_blob =
                write_blob("engine/properties.json", &properties.to_json().unwrap());

            let primitive_layouts = [
                (PrimitiveTypeV1::Bool, 1, 1),
                (PrimitiveTypeV1::Int8, 1, 1),
                (PrimitiveTypeV1::Int16, 2, 2),
                (PrimitiveTypeV1::Int32, 4, 4),
                (PrimitiveTypeV1::Int64, 8, 8),
                (PrimitiveTypeV1::Uint8, 1, 1),
                (PrimitiveTypeV1::Uint16, 2, 2),
                (PrimitiveTypeV1::Uint32, 4, 4),
                (PrimitiveTypeV1::Uint64, 8, 8),
                (PrimitiveTypeV1::Float32, 4, 4),
                (PrimitiveTypeV1::Float64, 8, 8),
            ];
            let fixed_operations = |value_size, value_alignment| FixedTypeOperationsV1 {
                can_be_template_subtype: true,
                can_construct: true,
                need_construct: false,
                can_destruct: true,
                need_destruct: false,
                can_copy: true,
                need_copy: false,
                can_compare: true,
                can_hash_value: true,
                value_size,
                value_alignment,
                is_object_pointer: false,
            };
            let mut trace = RegistrationTraceV1 {
                schema: REGISTRATION_TRACE_SCHEMA.to_owned(),
                schema_version: 1,
                host_stubs: vec![],
                primitive_operations: primitive_layouts
                    .into_iter()
                    .enumerate()
                    .map(
                        |(ordinal, (primitive, size, alignment))| PrimitiveTypeOperationsV1 {
                            ordinal: ordinal as u32,
                            primitive,
                            operations: fixed_operations(size, alignment),
                        },
                    )
                    .collect(),
                dynamic_script_operations: DynamicScriptTypeOperationsV1 {
                    delegate: fixed_operations(16, 8),
                    multicast_delegate: fixed_operations(16, 8),
                },
                entries: vec![RegistrationEntryV1::Enum {
                    ordinal: 0,
                    registration_id: 0,
                    context: RegistrationContextV1 {
                        namespace: String::new(),
                        config_group: None,
                        access_mask: u32::MAX,
                    },
                    type_id: 1,
                    declaration: "ETest".to_owned(),
                    type_operations: TypeOperationsV1::Fixed {
                        operations: fixed_operations(1, 1),
                    },
                }],
                canonical_sha256: zero_digest(),
            };
            trace.seal().unwrap();
            let trace_blob = write_blob("engine/registrations.json", &trace.to_json().unwrap());

            let mut snapshot = PostBindSnapshotV1 {
                schema: POST_BIND_SNAPSHOT_SCHEMA.to_owned(),
                schema_version: 1,
                engine_properties_sha256: properties.canonical_sha256,
                registration_trace_sha256: trace.canonical_sha256,
                entries: vec![PostBindEntryV1 {
                    ordinal: 0,
                    trace_registration_id: 0,
                    result: PostBindResultV1::Enum { engine_type_id: 1 },
                }],
                final_states: vec![],
                canonical_sha256: zero_digest(),
            };
            snapshot.seal().unwrap();
            let snapshot_blob = write_blob("engine/post-bind.json", &snapshot.to_json().unwrap());

            let mut preprocessor = PreprocessorConfigV1 {
                schema: PREPROCESSOR_CONFIG_SCHEMA.to_owned(),
                schema_version: FRONTEND_SCHEMA_VERSION,
                automatic_imports: true,
                warn_on_manual_import_statements: true,
                use_editor_scripts: false,
                effective_flags: [
                    ("COOK_COMMANDLET", false),
                    ("EDITOR", false),
                    ("EDITORONLY_DATA", false),
                    ("RELEASE", true),
                    ("TEST", false),
                    ("WITH_SERVER_CODE", true),
                ]
                .into_iter()
                .enumerate()
                .map(|(ordinal, (name, value))| EffectivePreprocessorFlagV1 {
                    ordinal: ordinal as u32,
                    name: name.to_owned(),
                    value,
                })
                .collect(),
                default_function_blueprint_callable: true,
                default_property_edit_specifier: PropertyEditSpecifierV1::EditAnywhere,
                default_property_edit_specifier_for_structs: PropertyEditSpecifierV1::EditAnywhere,
                default_property_blueprint_specifier:
                    PropertyBlueprintSpecifierV1::BlueprintReadWrite,
                static_class_mode: StaticClassModeV1::Allowed,
                script_float_is_float64: true,
                angelscript_haze: false,
                enforce_server_rpc_validation: false,
                blueprint_event_argument_specializations: vec![
                    "FName".to_owned(),
                    "int32".to_owned(),
                ],
                native_super_types: vec![],
                canonical_sha256: zero_digest(),
            };
            preprocessor.seal().unwrap();
            let preprocessor_blob = write_blob(
                "frontend/preprocessor.json",
                &preprocessor.to_json().unwrap(),
            );
            let mut class_generator = ClassGeneratorConfigV1 {
                schema: CLASS_GENERATOR_CONFIG_SCHEMA.to_owned(),
                schema_version: FRONTEND_SCHEMA_VERSION,
                mark_non_uproperty_properties_as_transient: false,
                canonical_sha256: zero_digest(),
            };
            class_generator.seal().unwrap();
            let class_generator_blob = write_blob(
                "frontend/class-generator.json",
                &class_generator.to_json().unwrap(),
            );
            let mut compiler_options = CompilerOptionsV1 {
                schema: COMPILER_OPTIONS_SCHEMA.to_owned(),
                schema_version: FRONTEND_SCHEMA_VERSION,
                error_on_incorrect_editor_only_code: true,
                warn_on_divergent_comparison_operator_overloads: true,
                warn_on_implicit_signed_unsigned_conversion: true,
                warn_on_increment_decrement_in_complex_expression: true,
                warn_on_unused_return_value_for_const_methods: true,
                canonical_sha256: zero_digest(),
            };
            compiler_options.seal().unwrap();
            let compiler_options_blob = write_blob(
                "frontend/compiler-options.json",
                &compiler_options.to_json().unwrap(),
            );

            let source_text = "void Test() {}\n";
            let mut corpus = CompilerProbeCorpusV1 {
                schema: PROBE_CORPUS_SCHEMA.to_owned(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: "receipt-test-v1".to_owned(),
                cases: vec![CompilerProbeCaseV1 {
                    ordinal: 0,
                    case_id: "positive.compile".to_owned(),
                    category: "smoke".to_owned(),
                    expected_outcome: ProbeOutcomeV1::Accepted,
                    mode: ProbeModeV1::CompileOnly,
                    sections: vec![ProbeSourceSectionV1 {
                        ordinal: 0,
                        module: "Module".to_owned(),
                        relative_path: "Module.as".to_owned(),
                        source_utf8: source_text.to_owned(),
                        source_sha256: ArtifactSealV1::from_bytes(source_text.as_bytes()).sha256,
                    }],
                }],
                canonical_sha256: zero_digest(),
            };
            corpus.seal().unwrap();
            let semantic_sha256 = ArtifactSealV1::from_bytes(b"normalized-result").sha256;
            let mut expected = ExpectedProbeResultsV1 {
                schema: EXPECTED_RESULTS_SCHEMA.to_owned(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: corpus.suite_id.clone(),
                corpus_sha256: corpus.canonical_sha256,
                results: vec![ExpectedProbeResultV1 {
                    ordinal: 0,
                    case_id: corpus.cases[0].case_id.clone(),
                    outcome: ProbeOutcomeV1::Accepted,
                    diagnostics: vec![],
                    semantic_sha256: Some(semantic_sha256),
                }],
                canonical_sha256: zero_digest(),
            };
            expected.seal().unwrap();
            let diagnostics_sha256 = expected.results[0].diagnostics_sha256().unwrap();
            let mut diagnostics = DiagnosticParityReportV1 {
                schema: DIAGNOSTIC_PARITY_SCHEMA.to_owned(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: corpus.suite_id.clone(),
                corpus_sha256: corpus.canonical_sha256,
                expected_results_sha256: expected.canonical_sha256,
                entries: vec![DiagnosticParityEntryV1 {
                    ordinal: 0,
                    case_id: corpus.cases[0].case_id.clone(),
                    expected_sha256: diagnostics_sha256,
                    embedded_sha256: diagnostics_sha256,
                    standalone_sha256: diagnostics_sha256,
                }],
                canonical_sha256: zero_digest(),
            };
            diagnostics.seal().unwrap();
            let mut semantics = SemanticParityReportV1 {
                schema: SEMANTIC_PARITY_SCHEMA.to_owned(),
                schema_version: QUALIFICATION_SCHEMA_VERSION,
                suite_id: corpus.suite_id.clone(),
                corpus_sha256: corpus.canonical_sha256,
                expected_results_sha256: expected.canonical_sha256,
                entries: vec![SemanticParityEntryV1 {
                    ordinal: 0,
                    case_id: corpus.cases[0].case_id.clone(),
                    expected_sha256: semantic_sha256,
                    embedded_sha256: semantic_sha256,
                    standalone_sha256: semantic_sha256,
                }],
                unexplained_differences: vec![],
                qualified: true,
                canonical_sha256: zero_digest(),
            };
            semantics.seal().unwrap();
            let corpus_blob = write_blob("qualification/corpus.json", &corpus.to_json().unwrap());
            let expected_blob =
                write_blob("qualification/expected.json", &expected.to_json().unwrap());
            let diagnostics_blob = write_blob(
                "qualification/diagnostics.json",
                &diagnostics.to_json().unwrap(),
            );
            let semantics_blob = write_blob(
                "qualification/semantics.json",
                &semantics.to_json().unwrap(),
            );

            let mut profile = qualified_profile_manifest(base, binds);
            profile.engine.ordered_engine_properties = properties_blob;
            profile.engine.registration_trace = trace_blob;
            profile.engine.post_bind_snapshot = snapshot_blob;
            profile.frontend.preprocessor_config = preprocessor_blob;
            profile.frontend.class_generator_config = class_generator_blob;
            profile.frontend.compiler_options = compiler_options_blob;
            profile.bytecode.codegen_probe_corpus = corpus_blob;
            profile.bytecode.expected_probe_results = expected_blob;
            profile.qualification.diagnostic_parity = diagnostics_blob;
            profile.qualification.semantic_parity = semantics_blob;
            profile.qualification.required_probe_suite_version = corpus.suite_id.clone();
            profile.seal().unwrap();
            let manifest = profile_root.join("profile.json");
            std::fs::write(&manifest, serde_json::to_vec(&profile).unwrap()).unwrap();
            let package =
                ValidatedCompilerProfilePackageV1::load(&manifest, &profile_root).unwrap();
            Self { root, package }
        }
    }

    impl Drop for QualifiedFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    fn selection() -> ReceiptBackendSelectionV1 {
        ReceiptBackendSelectionV1::from_compile_selection(
            CompilerBackendModeV1::Standalone,
            CompilerBackendNameV1::Standalone,
            None,
        )
        .unwrap()
    }

    fn unique_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "gore-as-generation-receipt-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn receipt_round_trip_binds_profile_generation_inputs_output_and_backend() {
        let guid = [0x42; 16];
        let base = cache(guid, b"qualified base");
        let binds = b"qualified binds".to_vec();
        let output = cache(guid, b"deterministic output");
        let fixture = QualifiedFixture::create("round-trip", &base, &binds);
        let sources = [
            GenerationSourceFileV1 {
                relative_path: "Game/B.as",
                bytes: b"void B() {}",
            },
            GenerationSourceFileV1 {
                relative_path: "Game/A.as",
                bytes: b"void A() {}",
            },
        ];

        let receipt = GenerationReceiptV1::build_from_bytes(
            &fixture.package,
            &sources,
            &base,
            &binds,
            &output,
            "GameModule",
            selection(),
        )
        .unwrap();
        let json = receipt.to_json().unwrap();
        let decoded = GenerationReceiptV1::from_json(&json).unwrap();
        decoded
            .validate_against(&fixture.package, &sources, &base, &binds, &output)
            .unwrap();
        assert_eq!(decoded.output.cache_guid, "42".repeat(16));
        assert_eq!(decoded.output.build_identifier, CACHE_MAGIC);
        assert_eq!(decoded.qualified_profile.steam_build_id, 24_539_464);
        assert_eq!(decoded.qualified_profile.depot_id, 1_297_901);
        assert_eq!(
            decoded.backend.used_backend,
            ReceiptBackendNameV1::Standalone
        );
    }

    #[test]
    fn receipt_rejects_unqualified_or_drifting_inputs_header_and_silent_fallback() {
        let guid = [0x24; 16];
        let base = cache(guid, b"base");
        let binds = b"binds".to_vec();
        let output = cache(guid, b"output");
        let fixture = QualifiedFixture::create("negative", &base, &binds);
        let sources = [GenerationSourceFileV1 {
            relative_path: "Module.as",
            bytes: b"void Main() {}",
        }];

        let wrong_guid_output = cache([0x99; 16], b"output");
        assert!(matches!(
            GenerationReceiptV1::build_from_bytes(
                &fixture.package,
                &sources,
                &base,
                &binds,
                &wrong_guid_output,
                "Module",
                selection()
            ),
            Err(GenerationReceiptError::Invalid {
                field: "output.cache_guid",
                ..
            })
        ));
        assert!(ReceiptBackendSelectionV1::from_compile_selection(
            CompilerBackendModeV1::StandaloneThenGame,
            CompilerBackendNameV1::Game,
            None,
        )
        .is_err());

        let receipt = GenerationReceiptV1::build_from_bytes(
            &fixture.package,
            &sources,
            &base,
            &binds,
            &output,
            "Module",
            selection(),
        )
        .unwrap();
        let mut drifted = output.clone();
        drifted.push(0xff);
        assert!(receipt
            .validate_against(&fixture.package, &sources, &base, &binds, &drifted)
            .is_err());
    }

    #[test]
    fn exact_handle_receipt_publication_is_atomic_no_clobber_and_strict() {
        let root = unique_root("publication");
        std::fs::create_dir(&root).unwrap();
        let guid = [0x73; 16];
        let base = cache(guid, b"base");
        let binds = b"binds".to_vec();
        let output_bytes = cache(guid, b"output");
        let output_path = root.join("module.cache");
        std::fs::write(&output_path, &output_bytes).unwrap();
        let output = CompileOutput::bind_existing(output_path, "Module".to_owned()).unwrap();
        let fixture = QualifiedFixture::create("publication-profile", &base, &binds);
        let sources = [GenerationSourceFileV1 {
            relative_path: "Module.as",
            bytes: b"void Main() {}",
        }];
        let receipt = GenerationReceiptV1::build_for_compile_output(
            &fixture.package,
            &sources,
            &base,
            &binds,
            &output,
            selection(),
        )
        .unwrap();
        let destination = root.join("module.gore-as-receipt.json");
        publish_generation_receipt_v1(&destination, &receipt).unwrap();
        assert_eq!(read_generation_receipt_v1(&destination).unwrap(), receipt);
        assert!(matches!(
            publish_generation_receipt_v1(&destination, &receipt),
            Err(GenerationReceiptError::AlreadyExists(_))
        ));
        let published_output = root.join("published.cache");
        publish_generation_output_v1(&published_output, &output_bytes).unwrap();
        assert_eq!(std::fs::read(&published_output).unwrap(), output_bytes);
        assert!(matches!(
            publish_generation_output_v1(&published_output, &output_bytes),
            Err(GenerationReceiptError::AlreadyExists(_))
        ));

        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
        value["future_field"] = serde_json::json!(true);
        assert!(matches!(
            GenerationReceiptV1::from_json(&serde_json::to_vec(&value).unwrap()),
            Err(GenerationReceiptError::Json(_))
        ));
        drop(output);
        std::fs::remove_dir_all(root).unwrap();
    }
}
