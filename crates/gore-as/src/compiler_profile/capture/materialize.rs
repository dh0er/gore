//! Fail-closed materialization of a decoded capture into an unqualified profile package.
//!
//! Product qualification is deliberately impossible here. The normal compiler-profile parser
//! rejects the resulting manifest until separate diagnostic and semantic parity qualification
//! replaces it with a newly sealed `qualified=true` manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[cfg(all(test, windows))]
use super::CAPTURE_TARGET_24878692;
use super::{
    capture_target_for_steam_build_id_v1, decode_capture_v1, CaptureDecodeError, CaptureTargetV1,
    DecodedCaptureV1, MAX_CAPTURE_BYTES_V1,
};
use crate::compiler_profile::frontend::validate_frontend_profile_payloads;
use crate::compiler_profile::manifest::{
    BindsProfileV1, BytecodeProfileV1, CacheWriterProfileV1, CompilerArchitectureV1,
    CompilerBuildConfigurationV1, CompilerOracleV1, CompilerPlatformV1, CompilerProfileError,
    CompilerProfileV1, CompilerTargetV1, EngineProfileV1, FrontendProfileV1,
    QualificationProfileV1, SealedBlobV1, Sha256Digest, UnrealSemanticsProfileV1,
    COMPILER_PROFILE_SCHEMA, COMPILER_PROFILE_SCHEMA_VERSION, MAX_COMPILER_PROFILE_JSON_BYTES,
};
use crate::compiler_profile::qualification::{
    validate_qualification_payloads, CompilerProbeCorpusV1, QualificationError,
    QualifiedSidecarIdentityV1,
};
use crate::compiler_profile::qualification_suite::{
    offline_artifact_authority_summary_from_manifest_json_v1,
    validate_canonical_full_qualification_corpus_v1, OfflineArtifactAuthoritySummaryV1,
    OfflineQualificationErrorV1, OfflineQualificationPromotionV1, FULL_QUALIFICATION_SUITE_ID_V1,
};
use crate::compiler_profile::registry::validate_engine_profile_payloads;

pub const STATIC_SUPPORT_MANIFEST_SCHEMA_V1: &str = "gore.as.unqualified-profile-static-support";
pub const STATIC_SUPPORT_MANIFEST_SCHEMA_VERSION_V1: u32 = 1;
pub const MATERIALIZATION_RECEIPT_SCHEMA_V1: &str = "gore.as.unqualified-profile-materialization";
pub const MATERIALIZATION_RECEIPT_SCHEMA_VERSION_V1: u32 = 1;
pub const PROFILE_MANIFEST_FILE_V1: &str = "compiler-profile.json";
pub const MATERIALIZATION_RECEIPT_FILE_V1: &str = "materialization-receipt.json";
pub const QUALIFIED_PROMOTION_RECEIPT_FILE_V1: &str = "qualification-promotion-receipt.json";
pub const EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1: &str =
    "embedded-qualification-artifacts.json";
pub const STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1: &str =
    "standalone-qualification-artifacts.json";
pub const QUALIFIED_PROMOTION_RECEIPT_SCHEMA_V1: &str = "gore.as.qualified-profile-promotion";
pub const QUALIFIED_PROMOTION_RECEIPT_SCHEMA_VERSION_V1: u32 = 1;

const MAX_SUPPORT_MANIFEST_BYTES_V1: u64 = 1024 * 1024;
const MAX_STATIC_SUPPORT_BLOB_BYTES_V1: u64 = 256 * 1024 * 1024;
const MAX_STATIC_SUPPORT_AGGREGATE_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_MATERIALIZED_AGGREGATE_BYTES_V1: u64 = 1024 * 1024 * 1024;
const RECEIPT_HASH_DOMAIN_V1: &[u8] = b"gore-as-unqualified-profile-materialization-v1\0";
const QUALIFIED_RECEIPT_HASH_DOMAIN_V1: &[u8] = b"gore-as-qualified-profile-promotion-v1\0";
const QUALIFIED_PROFILE_TREE_HASH_DOMAIN_V1: &[u8] = b"gore-as-qualified-profile-tree-v1\0";

const ENGINE_PROPERTIES_FILE: &str = "engine-properties.json";
const REGISTRATION_TRACE_FILE: &str = "registration-trace.json";
const POST_BIND_SNAPSHOT_FILE: &str = "post-bind-snapshot.json";
const PREPROCESSOR_CONFIG_FILE: &str = "preprocessor-config.json";
const CLASS_GENERATOR_CONFIG_FILE: &str = "class-generator-config.json";
const COMPILER_OPTIONS_FILE: &str = "compiler-options.json";

const PROFILE_PAYLOAD_FILES_V1: [&str; 16] = [
    ENGINE_PROPERTIES_FILE,
    REGISTRATION_TRACE_FILE,
    POST_BIND_SNAPSHOT_FILE,
    "reflected-type-graph.bin",
    PREPROCESSOR_CONFIG_FILE,
    CLASS_GENERATOR_CONFIG_FILE,
    COMPILER_OPTIONS_FILE,
    "opcode-table.bin",
    "operand-schema.bin",
    "codegen-probe-corpus.json",
    "expected-probe-results.json",
    "serializer-schema.bin",
    "reference-table-order.bin",
    "normalized-oracle-corpus.bin",
    "diagnostic-parity.json",
    "semantic-parity.json",
];

pub(crate) fn validate_supported_compiler_profile_target_v1(
    profile: &CompilerProfileV1,
) -> Result<(), ProfileMaterializationError> {
    supported_capture_target_for_compiler_target_v1(&profile.target)?;
    Ok(())
}

fn supported_capture_target_for_compiler_target_v1(
    compiler_target: &CompilerTargetV1,
) -> Result<&'static CaptureTargetV1, ProfileMaterializationError> {
    let target = capture_target_for_steam_build_id_v1(compiler_target.steam_build_id)
        .ok_or(ProfileMaterializationError::StaticTargetMismatch)?;
    if compiler_target.steam_app_id != target.steam_app_id
        || compiler_target.depot_id != target.depot_id
        || compiler_target.depot_manifest_gid != target.depot_manifest_gid
        || compiler_target.platform != CompilerPlatformV1::Windows
        || compiler_target.architecture != CompilerArchitectureV1::X86_64
        || compiler_target.build_configuration != CompilerBuildConfigurationV1::Shipping
    {
        return Err(ProfileMaterializationError::StaticTargetMismatch);
    }
    Ok(target)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticBlobKindV1 {
    ReflectedTypeGraph,
    OpcodeTable,
    OperandSchema,
    CodegenProbeCorpus,
    ExpectedProbeResults,
    SerializerSchema,
    ReferenceTableOrder,
    NormalizedOracleCorpus,
    DiagnosticParity,
    SemanticParity,
}

const STATIC_BLOB_FILES_V1: [(StaticBlobKindV1, &str); 10] = [
    (
        StaticBlobKindV1::ReflectedTypeGraph,
        "reflected-type-graph.bin",
    ),
    (StaticBlobKindV1::OpcodeTable, "opcode-table.bin"),
    (StaticBlobKindV1::OperandSchema, "operand-schema.bin"),
    (
        StaticBlobKindV1::CodegenProbeCorpus,
        "codegen-probe-corpus.json",
    ),
    (
        StaticBlobKindV1::ExpectedProbeResults,
        "expected-probe-results.json",
    ),
    (StaticBlobKindV1::SerializerSchema, "serializer-schema.bin"),
    (
        StaticBlobKindV1::ReferenceTableOrder,
        "reference-table-order.bin",
    ),
    (
        StaticBlobKindV1::NormalizedOracleCorpus,
        "normalized-oracle-corpus.bin",
    ),
    (StaticBlobKindV1::DiagnosticParity, "diagnostic-parity.json"),
    (StaticBlobKindV1::SemanticParity, "semantic-parity.json"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedSupportBlobV1 {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSupportPayloadSealsV1 {
    pub reflected_type_graph: PinnedSupportBlobV1,
    pub opcode_table: PinnedSupportBlobV1,
    pub operand_schema: PinnedSupportBlobV1,
    pub codegen_probe_corpus: PinnedSupportBlobV1,
    pub expected_probe_results: PinnedSupportBlobV1,
    pub serializer_schema: PinnedSupportBlobV1,
    pub reference_table_order: PinnedSupportBlobV1,
    pub normalized_oracle_corpus: PinnedSupportBlobV1,
    pub diagnostic_parity: PinnedSupportBlobV1,
    pub semantic_parity: PinnedSupportBlobV1,
}

impl StaticSupportPayloadSealsV1 {
    fn get(&self, kind: StaticBlobKindV1) -> PinnedSupportBlobV1 {
        match kind {
            StaticBlobKindV1::ReflectedTypeGraph => self.reflected_type_graph,
            StaticBlobKindV1::OpcodeTable => self.opcode_table,
            StaticBlobKindV1::OperandSchema => self.operand_schema,
            StaticBlobKindV1::CodegenProbeCorpus => self.codegen_probe_corpus,
            StaticBlobKindV1::ExpectedProbeResults => self.expected_probe_results,
            StaticBlobKindV1::SerializerSchema => self.serializer_schema,
            StaticBlobKindV1::ReferenceTableOrder => self.reference_table_order,
            StaticBlobKindV1::NormalizedOracleCorpus => self.normalized_oracle_corpus,
            StaticBlobKindV1::DiagnosticParity => self.diagnostic_parity,
            StaticBlobKindV1::SemanticParity => self.semantic_parity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticProfileSupportManifestV1 {
    pub schema: String,
    pub schema_version: u32,
    pub target: CompilerTargetV1,
    pub oracle: CompilerOracleV1,
    pub binds: BindsProfileV1,
    pub unreal_metadata_schema_version: u32,
    pub opcode_table_version: String,
    pub cache_format_version: u32,
    pub required_probe_suite_version: String,
    pub payloads: StaticSupportPayloadSealsV1,
}

impl StaticProfileSupportManifestV1 {
    pub fn from_json(bytes: &[u8]) -> Result<Self, ProfileMaterializationError> {
        if bytes.len() as u64 > MAX_SUPPORT_MANIFEST_BYTES_V1 {
            return Err(ProfileMaterializationError::InputTooLarge {
                label: "static support manifest",
                actual: bytes.len() as u64,
                max: MAX_SUPPORT_MANIFEST_BYTES_V1,
            });
        }
        let manifest: Self = serde_json::from_slice(bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json_pretty(&self) -> Result<Vec<u8>, ProfileMaterializationError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    pub fn validate(&self) -> Result<(), ProfileMaterializationError> {
        if self.schema != STATIC_SUPPORT_MANIFEST_SCHEMA_V1
            || self.schema_version != STATIC_SUPPORT_MANIFEST_SCHEMA_VERSION_V1
        {
            return Err(ProfileMaterializationError::StaticSupportSchema);
        }
        let target = supported_capture_target_for_compiler_target_v1(&self.target)?;
        if self.oracle.executable.byte_len != target.executable_bytes
            || self.oracle.executable.sha256 != Sha256Digest::from_bytes(target.executable_sha256)
            || self
                .oracle
                .executable
                .steam_content_sha1
                .is_none_or(|digest| digest.as_bytes().iter().all(|byte| *byte == 0))
            || self.oracle.binds_cache.byte_len == 0
            || self.oracle.binds_cache.sha256 == Sha256Digest::from_bytes([0; 32])
            || self
                .oracle
                .binds_cache
                .steam_content_sha1
                .is_none_or(|digest| digest.as_bytes().iter().all(|byte| *byte == 0))
            || self.oracle.shipping_cache.byte_len == 0
            || self.oracle.shipping_cache.sha256 == Sha256Digest::from_bytes([0; 32])
            || self
                .oracle
                .shipping_cache
                .steam_content_sha1
                .is_none_or(|digest| digest.as_bytes().iter().all(|byte| *byte == 0))
            || self.oracle.depot_manifest.byte_len == 0
            || self.oracle.depot_manifest.sha256 == Sha256Digest::from_bytes([0; 32])
            || !self
                .oracle
                .pe_codeview
                .guid
                .eq_ignore_ascii_case(target.codeview_guid)
            || self.oracle.pe_codeview.age != target.codeview_age
        {
            return Err(ProfileMaterializationError::StaticOracleMismatch);
        }
        if self.binds.wire_schema_version == 0
            || self.binds.struct_count == 0
            || self.binds.class_count == 0
            || self.binds.method_count == 0
            || self.binds.struct_property_count == 0
            || self.binds.class_property_count == 0
            || self.binds.canonical_database_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.unreal_metadata_schema_version == 0
            || self.cache_format_version == 0
        {
            return Err(ProfileMaterializationError::StaticMeasurementMissing);
        }
        for (label, value) in [
            ("opcode_table_version", self.opcode_table_version.as_str()),
            (
                "required_probe_suite_version",
                self.required_probe_suite_version.as_str(),
            ),
        ] {
            if value.is_empty() || value.len() > 128 || !value.is_ascii() {
                return Err(ProfileMaterializationError::InvalidStaticString(label));
            }
        }
        let mut total = 0u64;
        for (kind, _) in STATIC_BLOB_FILES_V1 {
            let seal = self.payloads.get(kind);
            if seal.byte_len == 0
                || seal.byte_len > MAX_STATIC_SUPPORT_BLOB_BYTES_V1
                || seal.sha256 == Sha256Digest::from_bytes([0; 32])
            {
                return Err(ProfileMaterializationError::InvalidSupportSeal);
            }
            total = total
                .checked_add(seal.byte_len)
                .ok_or(ProfileMaterializationError::SizeOverflow)?;
        }
        if total > MAX_STATIC_SUPPORT_AGGREGATE_BYTES_V1 {
            return Err(ProfileMaterializationError::InputTooLarge {
                label: "static support aggregate",
                actual: total,
                max: MAX_STATIC_SUPPORT_AGGREGATE_BYTES_V1,
            });
        }
        Ok(())
    }
}

/// Handle-pinned static support. Fields are private so callers cannot bypass byte/seal checks.
#[derive(Debug)]
pub struct PinnedStaticProfileSupportV1 {
    manifest: StaticProfileSupportManifestV1,
    manifest_sha256: Sha256Digest,
    payloads: BTreeMap<&'static str, Vec<u8>>,
    _manifest_file: File,
    _payload_files: Vec<File>,
    _directory_pin: File,
}

impl PinnedStaticProfileSupportV1 {
    pub fn manifest(&self) -> &StaticProfileSupportManifestV1 {
        &self.manifest
    }

    pub fn manifest_sha256(&self) -> Sha256Digest {
        self.manifest_sha256
    }

    fn bytes(&self, file_name: &'static str) -> &[u8] {
        self.payloads
            .get(file_name)
            .expect("all fixed support payloads are loaded")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedFileSealV1 {
    pub path: String,
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnqualifiedProfileMaterializationReceiptV1 {
    pub schema: String,
    pub schema_version: u32,
    pub qualified: bool,
    pub capture_stream_sha256: Sha256Digest,
    pub static_support_manifest_sha256: Sha256Digest,
    pub profile_sha256: Sha256Digest,
    pub files: Vec<MaterializedFileSealV1>,
    pub canonical_sha256: Sha256Digest,
}

#[derive(Serialize)]
struct ReceiptHashPayloadV1<'a> {
    schema: &'a str,
    schema_version: u32,
    qualified: bool,
    capture_stream_sha256: Sha256Digest,
    static_support_manifest_sha256: Sha256Digest,
    profile_sha256: Sha256Digest,
    files: &'a [MaterializedFileSealV1],
}

impl UnqualifiedProfileMaterializationReceiptV1 {
    fn computed_sha256(&self) -> Result<Sha256Digest, ProfileMaterializationError> {
        let payload = ReceiptHashPayloadV1 {
            schema: &self.schema,
            schema_version: self.schema_version,
            qualified: self.qualified,
            capture_stream_sha256: self.capture_stream_sha256,
            static_support_manifest_sha256: self.static_support_manifest_sha256,
            profile_sha256: self.profile_sha256,
            files: &self.files,
        };
        let bytes = serde_json::to_vec(&payload)?;
        let mut hash = Sha256::new();
        hash.update(RECEIPT_HASH_DOMAIN_V1);
        hash.update(bytes);
        Ok(Sha256Digest::from_bytes(hash.finalize().into()))
    }

    fn seal(&mut self) -> Result<(), ProfileMaterializationError> {
        self.canonical_sha256 = self.computed_sha256()?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ProfileMaterializationError> {
        if self.schema != MATERIALIZATION_RECEIPT_SCHEMA_V1
            || self.schema_version != MATERIALIZATION_RECEIPT_SCHEMA_VERSION_V1
            || self.qualified
            || self.capture_stream_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.static_support_manifest_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.profile_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.files.is_empty()
            || self.canonical_sha256 != self.computed_sha256()?
        {
            return Err(ProfileMaterializationError::InvalidMaterializationReceipt);
        }
        let mut previous: Option<&str> = None;
        let mut names = BTreeSet::new();
        for file in &self.files {
            if file.byte_len == 0
                || file.sha256 == Sha256Digest::from_bytes([0; 32])
                || !safe_fixed_file_name(&file.path)
                || previous.is_some_and(|name| name >= file.path.as_str())
                || !names.insert(file.path.to_ascii_lowercase())
            {
                return Err(ProfileMaterializationError::InvalidMaterializationReceipt);
            }
            previous = Some(&file.path);
        }
        Ok(())
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, ProfileMaterializationError> {
        if bytes.len() as u64 > MAX_SUPPORT_MANIFEST_BYTES_V1 {
            return Err(ProfileMaterializationError::InputTooLarge {
                label: "materialization receipt",
                actual: bytes.len() as u64,
                max: MAX_SUPPORT_MANIFEST_BYTES_V1,
            });
        }
        let receipt: Self = serde_json::from_slice(bytes)?;
        receipt.validate()?;
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedUnqualifiedProfileV1 {
    pub output_root: PathBuf,
    pub profile_sha256: Sha256Digest,
    pub capture_stream_sha256: Sha256Digest,
    pub static_support_manifest_sha256: Sha256Digest,
    pub qualified: bool,
    pub materialized_file_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedProfilePromotionReceiptV1 {
    pub schema: String,
    pub schema_version: u32,
    pub qualified: bool,
    pub source_profile_sha256: Sha256Digest,
    pub source_target: CompilerTargetV1,
    pub source_materialization_receipt_sha256: Sha256Digest,
    pub capture_stream_sha256: Sha256Digest,
    pub static_support_manifest_sha256: Sha256Digest,
    pub standalone_compiler: QualifiedSidecarIdentityV1,
    pub embedded_artifacts: OfflineArtifactAuthoritySummaryV1,
    pub standalone_artifacts: OfflineArtifactAuthoritySummaryV1,
    pub corpus_sha256: Sha256Digest,
    pub expected_results_sha256: Sha256Digest,
    pub diagnostic_parity_sha256: Sha256Digest,
    pub semantic_parity_sha256: Sha256Digest,
    pub profile_sha256: Sha256Digest,
    pub files: Vec<MaterializedFileSealV1>,
    pub canonical_sha256: Sha256Digest,
}

#[derive(Serialize)]
struct QualifiedReceiptHashPayloadV1<'a> {
    schema: &'a str,
    schema_version: u32,
    qualified: bool,
    source_profile_sha256: Sha256Digest,
    source_target: &'a CompilerTargetV1,
    source_materialization_receipt_sha256: Sha256Digest,
    capture_stream_sha256: Sha256Digest,
    static_support_manifest_sha256: Sha256Digest,
    standalone_compiler: QualifiedSidecarIdentityV1,
    embedded_artifacts: &'a OfflineArtifactAuthoritySummaryV1,
    standalone_artifacts: &'a OfflineArtifactAuthoritySummaryV1,
    corpus_sha256: Sha256Digest,
    expected_results_sha256: Sha256Digest,
    diagnostic_parity_sha256: Sha256Digest,
    semantic_parity_sha256: Sha256Digest,
    profile_sha256: Sha256Digest,
    files: &'a [MaterializedFileSealV1],
}

impl QualifiedProfilePromotionReceiptV1 {
    fn computed_sha256(&self) -> Result<Sha256Digest, ProfileMaterializationError> {
        let payload = QualifiedReceiptHashPayloadV1 {
            schema: &self.schema,
            schema_version: self.schema_version,
            qualified: self.qualified,
            source_profile_sha256: self.source_profile_sha256,
            source_target: &self.source_target,
            source_materialization_receipt_sha256: self.source_materialization_receipt_sha256,
            capture_stream_sha256: self.capture_stream_sha256,
            static_support_manifest_sha256: self.static_support_manifest_sha256,
            standalone_compiler: self.standalone_compiler,
            embedded_artifacts: &self.embedded_artifacts,
            standalone_artifacts: &self.standalone_artifacts,
            corpus_sha256: self.corpus_sha256,
            expected_results_sha256: self.expected_results_sha256,
            diagnostic_parity_sha256: self.diagnostic_parity_sha256,
            semantic_parity_sha256: self.semantic_parity_sha256,
            profile_sha256: self.profile_sha256,
            files: &self.files,
        };
        let bytes = serde_json::to_vec(&payload)?;
        let mut hash = Sha256::new();
        hash.update(QUALIFIED_RECEIPT_HASH_DOMAIN_V1);
        hash.update(bytes);
        Ok(Sha256Digest::from_bytes(hash.finalize().into()))
    }

    fn seal(&mut self) -> Result<(), ProfileMaterializationError> {
        self.canonical_sha256 = self.computed_sha256()?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ProfileMaterializationError> {
        self.standalone_compiler.validate()?;
        if self.schema != QUALIFIED_PROMOTION_RECEIPT_SCHEMA_V1
            || self.schema_version != QUALIFIED_PROMOTION_RECEIPT_SCHEMA_VERSION_V1
            || !self.qualified
            || self.source_profile_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.source_materialization_receipt_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.capture_stream_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.static_support_manifest_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.embedded_artifacts.backend
                != crate::compiler_profile::qualification_runner::CompilerProbeBackendKindV1::EmbeddedGame
            || self.standalone_artifacts.backend
                != crate::compiler_profile::qualification_runner::CompilerProbeBackendKindV1::Standalone
            || self.embedded_artifacts.suite_id.is_empty()
            || self.embedded_artifacts.suite_id != self.standalone_artifacts.suite_id
            || self.embedded_artifacts.source_profile_sha256 != self.source_profile_sha256
            || self.standalone_artifacts.source_profile_sha256 != self.source_profile_sha256
            || self.embedded_artifacts.source_target != self.source_target
            || self.standalone_artifacts.source_target != self.source_target
            || self.embedded_artifacts.standalone_compiler.is_some()
            || self.standalone_artifacts.standalone_compiler != Some(self.standalone_compiler)
            || self.embedded_artifacts.corpus_sha256 != self.corpus_sha256
            || self.standalone_artifacts.corpus_sha256 != self.corpus_sha256
            || self.embedded_artifacts.cache_seals.is_empty()
            || self.standalone_artifacts.cache_seals.is_empty()
            || self.embedded_artifacts.manifest_canonical_sha256
                == Sha256Digest::from_bytes([0; 32])
            || self.standalone_artifacts.manifest_canonical_sha256
                == Sha256Digest::from_bytes([0; 32])
            || self.corpus_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.expected_results_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.diagnostic_parity_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.semantic_parity_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.profile_sha256 == Sha256Digest::from_bytes([0; 32])
            || self.files.is_empty()
            || self.canonical_sha256 != self.computed_sha256()?
        {
            return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
        }
        let mut previous: Option<&str> = None;
        let mut names = BTreeSet::new();
        for file in &self.files {
            if file.byte_len == 0
                || file.sha256 == Sha256Digest::from_bytes([0; 32])
                || !safe_fixed_file_name(&file.path)
                || previous.is_some_and(|name| name >= file.path.as_str())
                || !names.insert(file.path.to_ascii_lowercase())
            {
                return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
            }
            previous = Some(&file.path);
        }
        Ok(())
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, ProfileMaterializationError> {
        if bytes.len() as u64 > MAX_SUPPORT_MANIFEST_BYTES_V1 {
            return Err(ProfileMaterializationError::InputTooLarge {
                label: "qualification promotion receipt",
                actual: bytes.len() as u64,
                max: MAX_SUPPORT_MANIFEST_BYTES_V1,
            });
        }
        let receipt: Self = serde_json::from_slice(bytes)?;
        receipt.validate()?;
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedQualifiedProfileV1 {
    pub output_root: PathBuf,
    pub source_profile_sha256: Sha256Digest,
    pub profile_sha256: Sha256Digest,
    pub qualification_receipt_sha256: Sha256Digest,
    pub qualified: bool,
    pub materialized_file_count: u32,
}

/// Handle-independent digest summary produced while every qualified-profile input is still
/// opened no-follow and the profile root is pinned. Release tooling uses this to prove that the
/// exact tree it copied is the tree which passed the Rust typed reload, rather than trusting a
/// self-declared `qualified` bit or profile id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedProfileVerificationV1 {
    profile: CompilerProfileV1,
    manifest_sha256: Sha256Digest,
    promotion_receipt_sha256: Sha256Digest,
    tree_sha256: Sha256Digest,
    file_count: u32,
}

impl QualifiedProfileVerificationV1 {
    pub fn profile(&self) -> &CompilerProfileV1 {
        &self.profile
    }

    pub fn manifest_sha256(&self) -> Sha256Digest {
        self.manifest_sha256
    }

    pub fn promotion_receipt_sha256(&self) -> Sha256Digest {
        self.promotion_receipt_sha256
    }

    pub fn tree_sha256(&self) -> Sha256Digest {
        self.tree_sha256
    }

    pub fn file_count(&self) -> u32 {
        self.file_count
    }

    pub fn into_profile(self) -> CompilerProfileV1 {
        self.profile
    }
}

struct PublishedFileV1 {
    name: &'static str,
    file: File,
    seal: SealedBlobV1,
}

/// Safely load a strict static-support manifest and all ten fixed support files.
pub fn load_pinned_static_profile_support_v1(
    support_manifest_path: &Path,
    support_root: &Path,
) -> Result<PinnedStaticProfileSupportV1, ProfileMaterializationError> {
    require_absolute_normalized(support_manifest_path, "static support manifest")?;
    require_absolute_normalized(support_root, "static support root")?;
    let mut manifest_file =
        open_regular_no_follow(support_manifest_path, "static support manifest")?;
    let manifest_bytes = read_bounded(
        &mut manifest_file,
        MAX_SUPPORT_MANIFEST_BYTES_V1,
        "static support manifest",
    )?;
    let manifest = StaticProfileSupportManifestV1::from_json(&manifest_bytes)?;
    let manifest_sha256 = sha256(&manifest_bytes);
    let directory_pin = open_directory_no_follow(support_root, "static support root")?;
    let mut payloads = BTreeMap::new();
    let mut payload_files = Vec::with_capacity(STATIC_BLOB_FILES_V1.len());
    for (kind, file_name) in STATIC_BLOB_FILES_V1 {
        let expected = manifest.payloads.get(kind);
        let path = support_root.join(file_name);
        let mut file = open_regular_no_follow(&path, file_name)?;
        let bytes = read_exact_sealed(&mut file, expected, file_name)?;
        payloads.insert(file_name, bytes);
        payload_files.push(file);
    }
    Ok(PinnedStaticProfileSupportV1 {
        manifest,
        manifest_sha256,
        payloads,
        _manifest_file: manifest_file,
        _payload_files: payload_files,
        _directory_pin: directory_pin,
    })
}

/// Decode one sealed capture and materialize a new unqualified package.
///
/// This is the CLI-oriented path. The inner materializer accepts only the successfully decoded
/// projection and the private handle-pinned support value.
pub fn materialize_unqualified_profile_package_from_paths_v1(
    capture_path: &Path,
    support_manifest_path: &Path,
    support_root: &Path,
    output_root: &Path,
) -> Result<MaterializedUnqualifiedProfileV1, ProfileMaterializationError> {
    require_absolute_normalized(capture_path, "capture")?;
    let mut capture_file = open_regular_no_follow(capture_path, "capture")?;
    let capture_bytes = read_bounded(&mut capture_file, MAX_CAPTURE_BYTES_V1 as u64, "capture")?;
    let decoded = decode_capture_v1(&capture_bytes)?;
    let support = load_pinned_static_profile_support_v1(support_manifest_path, support_root)?;
    materialize_unqualified_profile_package_v1(&decoded, &support, output_root)
}

/// Materialize a package only from an already validated capture projection and pinned support.
pub fn materialize_unqualified_profile_package_v1(
    decoded: &DecodedCaptureV1,
    support: &PinnedStaticProfileSupportV1,
    output_root: &Path,
) -> Result<MaterializedUnqualifiedProfileV1, ProfileMaterializationError> {
    support.manifest.validate()?;
    require_absolute_normalized(output_root, "output root")?;
    let decoded_target = decoded.header.target();
    let support_target = supported_capture_target_for_compiler_target_v1(&support.manifest.target)?;
    if decoded_target.generation != support_target.generation {
        return Err(ProfileMaterializationError::StaticTargetMismatch);
    }
    if decoded.build_jit.build_identifier != decoded_target.build_identifier
        || decoded.build_jit.as_reference_debugging
        || !decoded.build_jit.fork_opcode_table_201_212_present
        || decoded.build_jit.reference_debug_opcodes_emittable
        || decoded.build_jit.resolve_object_ptr_callback_registered
        || decoded
            .frontend_configs
            .preprocessor
            .external_hooks
            .process_chunks
            .bound
        || !decoded
            .frontend_configs
            .preprocessor
            .external_hooks
            .process_chunks
            .captures
            .is_empty()
        || decoded
            .frontend_configs
            .preprocessor
            .external_hooks
            .post_process_code
            .bound
        || !decoded
            .frontend_configs
            .preprocessor
            .external_hooks
            .post_process_code
            .captures
            .is_empty()
        || decoded.registration_trace.entries.is_empty()
        || decoded.frontend_boundaries.len() != 3
        || decoded.sealed_stream_sha256 == Sha256Digest::from_bytes([0; 32])
    {
        return Err(ProfileMaterializationError::DecodedProjectionIncomplete);
    }

    let engine_properties = decoded.ordered_engine_properties.to_json()?;
    let registration_trace = decoded.registration_trace.to_json()?;
    let post_bind_snapshot = decoded.post_bind_snapshot.to_json()?;
    let preprocessor_config = decoded.frontend_configs.preprocessor.to_json()?;
    let class_generator_config = decoded.frontend_configs.class_generator.to_json()?;
    let compiler_options = decoded.frontend_configs.compiler_options.to_json()?;

    let captured_blobs = [
        (ENGINE_PROPERTIES_FILE, engine_properties.as_slice()),
        (REGISTRATION_TRACE_FILE, registration_trace.as_slice()),
        (POST_BIND_SNAPSHOT_FILE, post_bind_snapshot.as_slice()),
        (PREPROCESSOR_CONFIG_FILE, preprocessor_config.as_slice()),
        (
            CLASS_GENERATOR_CONFIG_FILE,
            class_generator_config.as_slice(),
        ),
        (COMPILER_OPTIONS_FILE, compiler_options.as_slice()),
    ];
    let mut aggregate = captured_blobs.iter().try_fold(0u64, |total, (_, bytes)| {
        total
            .checked_add(bytes.len() as u64)
            .ok_or(ProfileMaterializationError::SizeOverflow)
    })?;
    for (_, file_name) in STATIC_BLOB_FILES_V1 {
        aggregate = aggregate
            .checked_add(support.bytes(file_name).len() as u64)
            .ok_or(ProfileMaterializationError::SizeOverflow)?;
    }
    if aggregate > MAX_MATERIALIZED_AGGREGATE_BYTES_V1 {
        return Err(ProfileMaterializationError::InputTooLarge {
            label: "materialized profile aggregate",
            actual: aggregate,
            max: MAX_MATERIALIZED_AGGREGATE_BYTES_V1,
        });
    }

    let (_output_parent_pin, output_directory_pin) = create_new_output_root(output_root)?;
    let mut published = Vec::with_capacity(18);
    let mut seals = BTreeMap::<&'static str, SealedBlobV1>::new();
    for (name, bytes) in captured_blobs {
        let artifact = write_new_output_file(output_root, name, bytes)?;
        seals.insert(name, artifact.seal.clone());
        published.push(artifact);
    }
    for (_, name) in STATIC_BLOB_FILES_V1 {
        let artifact = write_new_output_file(output_root, name, support.bytes(name))?;
        seals.insert(name, artifact.seal.clone());
        published.push(artifact);
    }

    let static_manifest = &support.manifest;
    let mut profile = CompilerProfileV1 {
        schema: COMPILER_PROFILE_SCHEMA.to_owned(),
        schema_version: COMPILER_PROFILE_SCHEMA_VERSION,
        target: static_manifest.target.clone(),
        oracle: static_manifest.oracle.clone(),
        binds: static_manifest.binds.clone(),
        engine: EngineProfileV1 {
            as_create_version: decoded_target.angelscript_version,
            ordered_engine_properties: seal(&seals, ENGINE_PROPERTIES_FILE)?,
            registration_trace: seal(&seals, REGISTRATION_TRACE_FILE)?,
            registration_trace_count: decoded.registration_trace.entries.len() as u64,
            post_bind_snapshot: seal(&seals, POST_BIND_SNAPSHOT_FILE)?,
        },
        unreal_semantics: UnrealSemanticsProfileV1 {
            reflected_type_graph: seal(&seals, "reflected-type-graph.bin")?,
            metadata_schema_version: static_manifest.unreal_metadata_schema_version,
        },
        frontend: FrontendProfileV1 {
            preprocessor_config: seal(&seals, PREPROCESSOR_CONFIG_FILE)?,
            class_generator_config: seal(&seals, CLASS_GENERATOR_CONFIG_FILE)?,
            compiler_options: seal(&seals, COMPILER_OPTIONS_FILE)?,
        },
        bytecode: BytecodeProfileV1 {
            opcode_table_version: static_manifest.opcode_table_version.clone(),
            opcode_table: seal(&seals, "opcode-table.bin")?,
            operand_schema: seal(&seals, "operand-schema.bin")?,
            codegen_probe_corpus: seal(&seals, "codegen-probe-corpus.json")?,
            expected_probe_results: seal(&seals, "expected-probe-results.json")?,
        },
        cache_writer: CacheWriterProfileV1 {
            format_version: static_manifest.cache_format_version,
            serializer_schema: seal(&seals, "serializer-schema.bin")?,
            build_identifier: decoded.build_jit.build_identifier,
            reference_table_order: seal(&seals, "reference-table-order.bin")?,
            normalized_oracle_corpus: seal(&seals, "normalized-oracle-corpus.bin")?,
        },
        qualification: QualificationProfileV1 {
            required_probe_suite_version: static_manifest.required_probe_suite_version.clone(),
            diagnostic_parity: seal(&seals, "diagnostic-parity.json")?,
            semantic_parity: seal(&seals, "semantic-parity.json")?,
            qualified: false,
        },
        profile_sha256: Sha256Digest::from_bytes([0; 32]),
    };
    profile.seal()?;
    profile.validate_unqualified_materialized()?;
    let profile_json = serde_json::to_vec_pretty(&profile)?;
    let profile_file = write_new_output_file(output_root, PROFILE_MANIFEST_FILE_V1, &profile_json)?;
    published.push(profile_file);

    let mut receipt_files = published
        .iter()
        .map(|artifact| MaterializedFileSealV1 {
            path: artifact.name.to_owned(),
            byte_len: artifact.seal.byte_len,
            sha256: artifact.seal.sha256,
        })
        .collect::<Vec<_>>();
    receipt_files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut receipt = UnqualifiedProfileMaterializationReceiptV1 {
        schema: MATERIALIZATION_RECEIPT_SCHEMA_V1.to_owned(),
        schema_version: MATERIALIZATION_RECEIPT_SCHEMA_VERSION_V1,
        qualified: false,
        capture_stream_sha256: decoded.sealed_stream_sha256,
        static_support_manifest_sha256: support.manifest_sha256,
        profile_sha256: profile.profile_sha256,
        files: receipt_files,
        canonical_sha256: Sha256Digest::from_bytes([0; 32]),
    };
    receipt.seal()?;
    receipt.validate()?;
    let receipt_json = serde_json::to_vec_pretty(&receipt)?;
    published.push(write_new_output_file(
        output_root,
        MATERIALIZATION_RECEIPT_FILE_V1,
        &receipt_json,
    )?);

    reload_published_package(&mut published, decoded, support, &profile, &receipt)?;
    drop(output_directory_pin);

    Ok(MaterializedUnqualifiedProfileV1 {
        output_root: output_root.to_path_buf(),
        profile_sha256: profile.profile_sha256,
        capture_stream_sha256: decoded.sealed_stream_sha256,
        static_support_manifest_sha256: support.manifest_sha256,
        qualified: false,
        materialized_file_count: published.len() as u32,
    })
}

/// Reopen and fully type/seal-check an already materialized unqualified package.
///
/// This is read-only and intentionally cannot upgrade qualification.
pub fn reload_unqualified_profile_package_v1(
    output_root: &Path,
) -> Result<CompilerProfileV1, ProfileMaterializationError> {
    require_absolute_normalized(output_root, "output root")?;
    let _directory_pin = open_directory_no_follow(output_root, "output root")?;
    let mut manifest_file = open_regular_no_follow(
        &output_root.join(PROFILE_MANIFEST_FILE_V1),
        "profile manifest",
    )?;
    let manifest_json = read_bounded(
        &mut manifest_file,
        MAX_COMPILER_PROFILE_JSON_BYTES as u64,
        "profile manifest",
    )?;
    let profile = CompilerProfileV1::from_unqualified_json(&manifest_json)?;
    validate_supported_compiler_profile_target_v1(&profile)?;
    if !matches!(
        CompilerProfileV1::from_json(&manifest_json),
        Err(CompilerProfileError::NotQualified)
    ) {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    let mut files = BTreeMap::new();
    let mut blob_pins = Vec::with_capacity(16);
    let mut observed_receipt_files = vec![MaterializedFileSealV1 {
        path: PROFILE_MANIFEST_FILE_V1.to_owned(),
        byte_len: manifest_json.len() as u64,
        sha256: sha256(&manifest_json),
    }];
    for blob in profile_blobs(&profile) {
        let mut file = open_regular_no_follow(&output_root.join(&blob.path), "profile blob")?;
        let bytes = read_exact_profile_blob(&mut file, blob, "profile blob")?;
        observed_receipt_files.push(MaterializedFileSealV1 {
            path: blob.path.clone(),
            byte_len: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
        files.insert(blob.path.clone(), bytes);
        blob_pins.push(file);
    }
    let mut receipt_file = open_regular_no_follow(
        &output_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
        "materialization receipt",
    )?;
    let receipt_json = read_bounded(
        &mut receipt_file,
        MAX_SUPPORT_MANIFEST_BYTES_V1,
        "materialization receipt",
    )?;
    let receipt = UnqualifiedProfileMaterializationReceiptV1::from_json(&receipt_json)?;
    observed_receipt_files.sort_by(|left, right| left.path.cmp(&right.path));
    if receipt.profile_sha256 != profile.profile_sha256 || receipt.files != observed_receipt_files {
        return Err(ProfileMaterializationError::InvalidMaterializationReceipt);
    }
    validate_typed_profile_payloads(&profile, &files)?;
    drop(blob_pins);
    Ok(profile)
}

/// Publish a new qualified package from one exact pinned unqualified package and a successful
/// in-memory promotion authority. The source is never modified and the destination must not
/// exist. All source blobs remain handle-pinned until the new package has been typed-reloaded.
pub fn promote_unqualified_profile_package_v1(
    source_root: &Path,
    output_root: &Path,
    corpus: &CompilerProbeCorpusV1,
    promotion: &OfflineQualificationPromotionV1,
) -> Result<MaterializedQualifiedProfileV1, ProfileMaterializationError> {
    require_absolute_normalized(source_root, "source profile root")?;
    require_absolute_normalized(output_root, "qualified output root")?;
    if source_root == output_root
        || source_root.starts_with(output_root)
        || output_root.starts_with(source_root)
    {
        return Err(ProfileMaterializationError::UnsafePathRelationship);
    }
    if !promotion.qualified() {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    validate_canonical_full_qualification_corpus_v1(corpus)?;

    let source_directory_pin = open_directory_no_follow(source_root, "source profile root")?;
    let mut source_manifest_file = open_regular_no_follow(
        &source_root.join(PROFILE_MANIFEST_FILE_V1),
        "source profile manifest",
    )?;
    let source_manifest_json = read_bounded(
        &mut source_manifest_file,
        MAX_COMPILER_PROFILE_JSON_BYTES as u64,
        "source profile manifest",
    )?;
    let source_profile = CompilerProfileV1::from_unqualified_json(&source_manifest_json)?;
    validate_supported_compiler_profile_target_v1(&source_profile)?;
    if source_profile.qualification.required_probe_suite_version != FULL_QUALIFICATION_SUITE_ID_V1 {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    if promotion.source_profile_sha256() != source_profile.profile_sha256
        || promotion.source_target() != &source_profile.target
    {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    require_fixed_materialized_profile_paths(&source_profile)?;

    let mut payloads = BTreeMap::<String, Vec<u8>>::new();
    let mut source_blob_pins = Vec::with_capacity(PROFILE_PAYLOAD_FILES_V1.len());
    let mut observed_source_files = vec![MaterializedFileSealV1 {
        path: PROFILE_MANIFEST_FILE_V1.to_owned(),
        byte_len: source_manifest_json.len() as u64,
        sha256: sha256(&source_manifest_json),
    }];
    for blob in profile_blobs(&source_profile) {
        let mut file =
            open_regular_no_follow(&source_root.join(&blob.path), "source profile payload")?;
        let bytes = read_exact_profile_blob(&mut file, blob, "source profile payload")?;
        observed_source_files.push(MaterializedFileSealV1 {
            path: blob.path.clone(),
            byte_len: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
        if payloads.insert(blob.path.clone(), bytes).is_some() {
            return Err(ProfileMaterializationError::QualificationBoundary);
        }
        source_blob_pins.push(file);
    }
    validate_typed_profile_payloads(&source_profile, &payloads)?;

    let mut source_receipt_file = open_regular_no_follow(
        &source_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
        "source materialization receipt",
    )?;
    let source_receipt_json = read_bounded(
        &mut source_receipt_file,
        MAX_SUPPORT_MANIFEST_BYTES_V1,
        "source materialization receipt",
    )?;
    let source_receipt =
        UnqualifiedProfileMaterializationReceiptV1::from_json(&source_receipt_json)?;
    observed_source_files.sort_by(|left, right| left.path.cmp(&right.path));
    if source_receipt.profile_sha256 != source_profile.profile_sha256
        || source_receipt.files != observed_source_files
    {
        return Err(ProfileMaterializationError::InvalidMaterializationReceipt);
    }

    let expected = promotion.expected_results();
    let differential = promotion.differential();
    let corpus_json = corpus.to_json()?;
    let expected_json = expected.to_json()?;
    let diagnostic_json = differential.diagnostic_parity.to_json()?;
    let semantic_json = differential.semantic_parity.to_json()?;
    let embedded_artifact_manifest_json = promotion.embedded_artifacts().manifest_json();
    let standalone_artifact_manifest_json = promotion.standalone_artifacts().manifest_json();
    let embedded_artifacts = promotion.embedded_artifacts().authority_summary()?;
    let standalone_artifacts = promotion.standalone_artifacts().authority_summary()?;
    for (path, bytes) in [
        (
            source_profile.bytecode.codegen_probe_corpus.path.clone(),
            corpus_json.as_slice(),
        ),
        (
            source_profile.bytecode.expected_probe_results.path.clone(),
            expected_json.as_slice(),
        ),
        (
            source_profile.qualification.diagnostic_parity.path.clone(),
            diagnostic_json.as_slice(),
        ),
        (
            source_profile.qualification.semantic_parity.path.clone(),
            semantic_json.as_slice(),
        ),
    ] {
        payloads.insert(path, bytes.to_vec());
    }

    let mut profile = source_profile.clone();
    profile.bytecode.codegen_probe_corpus =
        sealed_blob_for_existing_path(&profile.bytecode.codegen_probe_corpus.path, &corpus_json);
    profile.bytecode.expected_probe_results = sealed_blob_for_existing_path(
        &profile.bytecode.expected_probe_results.path,
        &expected_json,
    );
    profile.qualification.diagnostic_parity = sealed_blob_for_existing_path(
        &profile.qualification.diagnostic_parity.path,
        &diagnostic_json,
    );
    profile.qualification.semantic_parity =
        sealed_blob_for_existing_path(&profile.qualification.semantic_parity.path, &semantic_json);
    profile.qualification.required_probe_suite_version = corpus.suite_id.clone();
    profile.qualification.qualified = true;
    profile.seal()?;
    profile.validate_complete()?;
    validate_qualification_payloads(
        &profile.bytecode,
        &profile.qualification,
        &corpus_json,
        &expected_json,
        &diagnostic_json,
        &semantic_json,
    )?;

    let (_output_parent_pin, output_directory_pin) = create_new_output_root(output_root)?;
    let mut published = Vec::with_capacity(PROFILE_PAYLOAD_FILES_V1.len() + 4);
    for name in PROFILE_PAYLOAD_FILES_V1 {
        let bytes = payloads
            .get(name)
            .ok_or(ProfileMaterializationError::TypedReloadMissing(name))?;
        published.push(write_new_output_file(output_root, name, bytes)?);
    }
    published.push(write_new_output_file(
        output_root,
        EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        embedded_artifact_manifest_json,
    )?);
    published.push(write_new_output_file(
        output_root,
        STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        standalone_artifact_manifest_json,
    )?);
    let profile_json = serde_json::to_vec_pretty(&profile)?;
    published.push(write_new_output_file(
        output_root,
        PROFILE_MANIFEST_FILE_V1,
        &profile_json,
    )?);

    let mut receipt_files = published
        .iter()
        .map(|artifact| MaterializedFileSealV1 {
            path: artifact.name.to_owned(),
            byte_len: artifact.seal.byte_len,
            sha256: artifact.seal.sha256,
        })
        .collect::<Vec<_>>();
    receipt_files.sort_by(|left, right| left.path.cmp(&right.path));
    let standalone_compiler = promotion.standalone_compiler();
    let mut receipt = QualifiedProfilePromotionReceiptV1 {
        schema: QUALIFIED_PROMOTION_RECEIPT_SCHEMA_V1.to_owned(),
        schema_version: QUALIFIED_PROMOTION_RECEIPT_SCHEMA_VERSION_V1,
        qualified: true,
        source_profile_sha256: source_profile.profile_sha256,
        source_target: source_profile.target.clone(),
        source_materialization_receipt_sha256: sha256(&source_receipt_json),
        capture_stream_sha256: source_receipt.capture_stream_sha256,
        static_support_manifest_sha256: source_receipt.static_support_manifest_sha256,
        standalone_compiler,
        embedded_artifacts,
        standalone_artifacts,
        corpus_sha256: corpus.canonical_sha256,
        expected_results_sha256: expected.canonical_sha256,
        diagnostic_parity_sha256: differential.diagnostic_parity.canonical_sha256,
        semantic_parity_sha256: differential.semantic_parity.canonical_sha256,
        profile_sha256: profile.profile_sha256,
        files: receipt_files,
        canonical_sha256: Sha256Digest::from_bytes([0; 32]),
    };
    receipt.seal()?;
    receipt.validate()?;
    let receipt_json = serde_json::to_vec_pretty(&receipt)?;
    published.push(write_new_output_file(
        output_root,
        QUALIFIED_PROMOTION_RECEIPT_FILE_V1,
        &receipt_json,
    )?);

    reload_published_qualified_package(
        &mut published,
        &profile,
        &receipt,
        &corpus_json,
        &expected_json,
        &diagnostic_json,
        &semantic_json,
        embedded_artifact_manifest_json,
        standalone_artifact_manifest_json,
    )?;
    drop(output_directory_pin);
    drop(source_blob_pins);
    drop(source_receipt_file);
    drop(source_manifest_file);
    drop(source_directory_pin);

    Ok(MaterializedQualifiedProfileV1 {
        output_root: output_root.to_path_buf(),
        source_profile_sha256: source_profile.profile_sha256,
        profile_sha256: profile.profile_sha256,
        qualification_receipt_sha256: receipt.canonical_sha256,
        qualified: true,
        materialized_file_count: published.len() as u32,
    })
}

/// Reopen and fully validate a published qualified package, including both archived raw-artifact
/// manifests and their per-case cache/supplemental authority summaries in the promotion receipt.
pub fn reload_qualified_profile_package_v1(
    output_root: &Path,
) -> Result<CompilerProfileV1, ProfileMaterializationError> {
    verify_qualified_profile_package_v1(output_root)
        .map(QualifiedProfileVerificationV1::into_profile)
}

/// Fully validate one published qualified package and return a digest of every authoritative
/// file while the directory and all inputs remain pinned. The digest covers the manifest, all 16
/// typed payloads, both raw-artifact manifests, and the promotion receipt.
pub fn verify_qualified_profile_package_v1(
    output_root: &Path,
) -> Result<QualifiedProfileVerificationV1, ProfileMaterializationError> {
    require_absolute_normalized(output_root, "qualified profile root")?;
    let _directory_pin = open_directory_no_follow(output_root, "qualified profile root")?;
    let mut manifest_file = open_regular_no_follow(
        &output_root.join(PROFILE_MANIFEST_FILE_V1),
        "qualified profile manifest",
    )?;
    let manifest_json = read_bounded(
        &mut manifest_file,
        MAX_COMPILER_PROFILE_JSON_BYTES as u64,
        "qualified profile manifest",
    )?;
    let profile = CompilerProfileV1::from_json(&manifest_json)?;
    validate_supported_compiler_profile_target_v1(&profile)?;
    require_fixed_materialized_profile_paths(&profile)?;
    let mut bytes = BTreeMap::new();
    let mut pins = Vec::with_capacity(PROFILE_PAYLOAD_FILES_V1.len() + 3);
    let mut observed_files = vec![MaterializedFileSealV1 {
        path: PROFILE_MANIFEST_FILE_V1.to_owned(),
        byte_len: manifest_json.len() as u64,
        sha256: sha256(&manifest_json),
    }];
    for blob in profile_blobs(&profile) {
        let mut file =
            open_regular_no_follow(&output_root.join(&blob.path), "qualified profile payload")?;
        let payload = read_exact_profile_blob(&mut file, blob, "qualified profile payload")?;
        observed_files.push(MaterializedFileSealV1 {
            path: blob.path.clone(),
            byte_len: payload.len() as u64,
            sha256: sha256(&payload),
        });
        bytes.insert(blob.path.clone(), payload);
        pins.push(file);
    }
    for name in [
        EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
    ] {
        let mut file = open_regular_no_follow(&output_root.join(name), "artifact archive")?;
        let payload = read_bounded(
            &mut file,
            MAX_STATIC_SUPPORT_BLOB_BYTES_V1,
            "artifact archive",
        )?;
        observed_files.push(MaterializedFileSealV1 {
            path: name.to_owned(),
            byte_len: payload.len() as u64,
            sha256: sha256(&payload),
        });
        bytes.insert(name.to_owned(), payload);
        pins.push(file);
    }
    let mut receipt_file = open_regular_no_follow(
        &output_root.join(QUALIFIED_PROMOTION_RECEIPT_FILE_V1),
        "qualification promotion receipt",
    )?;
    let receipt_json = read_bounded(
        &mut receipt_file,
        MAX_SUPPORT_MANIFEST_BYTES_V1,
        "qualification promotion receipt",
    )?;
    let receipt = QualifiedProfilePromotionReceiptV1::from_json(&receipt_json)?;
    observed_files.sort_by(|left, right| left.path.cmp(&right.path));
    if receipt.profile_sha256 != profile.profile_sha256
        || receipt.source_target != profile.target
        || receipt.files != observed_files
    {
        return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
    }
    if receipt.embedded_artifacts.suite_id != profile.qualification.required_probe_suite_version {
        return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
    }
    let embedded_archive = bytes
        .get(EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1)
        .ok_or(ProfileMaterializationError::TypedReloadMissing(
            EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        ))?;
    let standalone_archive = bytes
        .get(STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1)
        .ok_or(ProfileMaterializationError::TypedReloadMissing(
            STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        ))?;
    if offline_artifact_authority_summary_from_manifest_json_v1(embedded_archive)?
        != receipt.embedded_artifacts
        || offline_artifact_authority_summary_from_manifest_json_v1(standalone_archive)?
            != receipt.standalone_artifacts
    {
        return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
    }
    validate_typed_profile_payloads(&profile, &bytes)?;
    let qualification = validate_qualification_payloads(
        &profile.bytecode,
        &profile.qualification,
        bytes[&profile.bytecode.codegen_probe_corpus.path].as_slice(),
        bytes[&profile.bytecode.expected_probe_results.path].as_slice(),
        bytes[&profile.qualification.diagnostic_parity.path].as_slice(),
        bytes[&profile.qualification.semantic_parity.path].as_slice(),
    )?;
    if qualification.standalone_compiler() != receipt.standalone_compiler {
        return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
    }
    let manifest_sha256 = sha256(&manifest_json);
    let promotion_receipt_sha256 = sha256(&receipt_json);
    let mut tree_files = observed_files;
    tree_files.push(MaterializedFileSealV1 {
        path: QUALIFIED_PROMOTION_RECEIPT_FILE_V1.to_owned(),
        byte_len: receipt_json.len() as u64,
        sha256: promotion_receipt_sha256,
    });
    tree_files.sort_by(|left, right| left.path.cmp(&right.path));
    let tree_sha256 = qualified_profile_tree_sha256_v1(&tree_files);
    let file_count = u32::try_from(tree_files.len())
        .map_err(|_| ProfileMaterializationError::InvalidQualifiedPromotionReceipt)?;
    drop(receipt_file);
    drop(pins);
    Ok(QualifiedProfileVerificationV1 {
        profile,
        manifest_sha256,
        promotion_receipt_sha256,
        tree_sha256,
        file_count,
    })
}

fn qualified_profile_tree_sha256_v1(files: &[MaterializedFileSealV1]) -> Sha256Digest {
    let mut hash = Sha256::new();
    hash.update(QUALIFIED_PROFILE_TREE_HASH_DOMAIN_V1);
    hash.update((files.len() as u64).to_le_bytes());
    for file in files {
        hash.update((file.path.len() as u64).to_le_bytes());
        hash.update(file.path.as_bytes());
        hash.update(file.byte_len.to_le_bytes());
        hash.update(file.sha256.as_bytes());
    }
    Sha256Digest::from_bytes(hash.finalize().into())
}

fn sealed_blob_for_existing_path(path: &str, bytes: &[u8]) -> SealedBlobV1 {
    SealedBlobV1 {
        path: path.to_owned(),
        byte_len: bytes.len() as u64,
        sha256: sha256(bytes),
    }
}

fn require_fixed_materialized_profile_paths(
    profile: &CompilerProfileV1,
) -> Result<(), ProfileMaterializationError> {
    let actual: BTreeSet<_> = profile_blobs(profile)
        .into_iter()
        .map(|blob| blob.path.as_str())
        .collect();
    let expected: BTreeSet<_> = PROFILE_PAYLOAD_FILES_V1.into_iter().collect();
    if actual != expected || actual.len() != PROFILE_PAYLOAD_FILES_V1.len() {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    Ok(())
}

fn reload_published_qualified_package(
    published: &mut [PublishedFileV1],
    expected_profile: &CompilerProfileV1,
    expected_receipt: &QualifiedProfilePromotionReceiptV1,
    corpus_json: &[u8],
    expected_json: &[u8],
    diagnostic_json: &[u8],
    semantic_json: &[u8],
    embedded_artifact_manifest_json: &[u8],
    standalone_artifact_manifest_json: &[u8],
) -> Result<(), ProfileMaterializationError> {
    let mut bytes = BTreeMap::<String, Vec<u8>>::new();
    for artifact in published.iter_mut() {
        let actual = read_exact_profile_blob(&mut artifact.file, &artifact.seal, artifact.name)?;
        bytes.insert(artifact.name.to_owned(), actual);
    }
    let manifest_json = bytes.get(PROFILE_MANIFEST_FILE_V1).ok_or(
        ProfileMaterializationError::TypedReloadMissing(PROFILE_MANIFEST_FILE_V1),
    )?;
    let profile = CompilerProfileV1::from_json(manifest_json)?;
    if &profile != expected_profile {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    let receipt_json = bytes.get(QUALIFIED_PROMOTION_RECEIPT_FILE_V1).ok_or(
        ProfileMaterializationError::TypedReloadMissing(QUALIFIED_PROMOTION_RECEIPT_FILE_V1),
    )?;
    let receipt = QualifiedProfilePromotionReceiptV1::from_json(receipt_json)?;
    if &receipt != expected_receipt || receipt.profile_sha256 != profile.profile_sha256 {
        return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
    }
    let embedded_archive = bytes
        .get(EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1)
        .ok_or(ProfileMaterializationError::TypedReloadMissing(
            EMBEDDED_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        ))?;
    let standalone_archive = bytes
        .get(STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1)
        .ok_or(ProfileMaterializationError::TypedReloadMissing(
            STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1,
        ))?;
    if embedded_archive.as_slice() != embedded_artifact_manifest_json
        || standalone_archive.as_slice() != standalone_artifact_manifest_json
        || offline_artifact_authority_summary_from_manifest_json_v1(embedded_archive)?
            != receipt.embedded_artifacts
        || offline_artifact_authority_summary_from_manifest_json_v1(standalone_archive)?
            != receipt.standalone_artifacts
    {
        return Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt);
    }
    validate_typed_profile_payloads(&profile, &bytes)?;
    validate_qualification_payloads(
        &profile.bytecode,
        &profile.qualification,
        corpus_json,
        expected_json,
        diagnostic_json,
        semantic_json,
    )?;
    Ok(())
}

fn reload_published_package(
    published: &mut [PublishedFileV1],
    decoded: &DecodedCaptureV1,
    support: &PinnedStaticProfileSupportV1,
    expected_profile: &CompilerProfileV1,
    expected_receipt: &UnqualifiedProfileMaterializationReceiptV1,
) -> Result<(), ProfileMaterializationError> {
    let mut bytes = BTreeMap::<String, Vec<u8>>::new();
    for artifact in published.iter_mut() {
        let actual = read_exact_profile_blob(&mut artifact.file, &artifact.seal, artifact.name)?;
        bytes.insert(artifact.name.to_owned(), actual);
    }
    let manifest_json = bytes.get(PROFILE_MANIFEST_FILE_V1).ok_or(
        ProfileMaterializationError::TypedReloadMissing(PROFILE_MANIFEST_FILE_V1),
    )?;
    let profile = CompilerProfileV1::from_unqualified_json(manifest_json)?;
    if &profile != expected_profile
        || !matches!(
            CompilerProfileV1::from_json(manifest_json),
            Err(CompilerProfileError::NotQualified)
        )
    {
        return Err(ProfileMaterializationError::QualificationBoundary);
    }
    let receipt_json = bytes.get(MATERIALIZATION_RECEIPT_FILE_V1).ok_or(
        ProfileMaterializationError::TypedReloadMissing(MATERIALIZATION_RECEIPT_FILE_V1),
    )?;
    let receipt = UnqualifiedProfileMaterializationReceiptV1::from_json(receipt_json)?;
    if &receipt != expected_receipt
        || receipt.capture_stream_sha256 != decoded.sealed_stream_sha256
        || receipt.static_support_manifest_sha256 != support.manifest_sha256
        || receipt.profile_sha256 != profile.profile_sha256
    {
        return Err(ProfileMaterializationError::InvalidMaterializationReceipt);
    }
    validate_typed_profile_payloads(&profile, &bytes)?;
    Ok(())
}

fn validate_typed_profile_payloads(
    profile: &CompilerProfileV1,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ProfileMaterializationError> {
    let get = |blob: &SealedBlobV1| {
        files
            .get(&blob.path)
            .map(Vec::as_slice)
            .ok_or_else(|| ProfileMaterializationError::TypedReloadPath(blob.path.clone()))
    };
    validate_engine_profile_payloads(
        &profile.engine,
        get(&profile.engine.ordered_engine_properties)?,
        get(&profile.engine.registration_trace)?,
        get(&profile.engine.post_bind_snapshot)?,
    )?;
    validate_frontend_profile_payloads(
        &profile.frontend,
        get(&profile.frontend.preprocessor_config)?,
        get(&profile.frontend.class_generator_config)?,
        get(&profile.frontend.compiler_options)?,
    )?;
    for blob in profile_blobs(profile) {
        let bytes = get(blob)?;
        if blob.byte_len != bytes.len() as u64 || blob.sha256 != sha256(bytes) {
            return Err(ProfileMaterializationError::TypedReloadSeal(
                blob.path.clone(),
            ));
        }
    }
    Ok(())
}

fn profile_blobs(profile: &CompilerProfileV1) -> [&SealedBlobV1; 16] {
    [
        &profile.engine.ordered_engine_properties,
        &profile.engine.registration_trace,
        &profile.engine.post_bind_snapshot,
        &profile.unreal_semantics.reflected_type_graph,
        &profile.frontend.preprocessor_config,
        &profile.frontend.class_generator_config,
        &profile.frontend.compiler_options,
        &profile.bytecode.opcode_table,
        &profile.bytecode.operand_schema,
        &profile.bytecode.codegen_probe_corpus,
        &profile.bytecode.expected_probe_results,
        &profile.cache_writer.serializer_schema,
        &profile.cache_writer.reference_table_order,
        &profile.cache_writer.normalized_oracle_corpus,
        &profile.qualification.diagnostic_parity,
        &profile.qualification.semantic_parity,
    ]
}

fn seal(
    seals: &BTreeMap<&'static str, SealedBlobV1>,
    name: &'static str,
) -> Result<SealedBlobV1, ProfileMaterializationError> {
    seals
        .get(name)
        .cloned()
        .ok_or(ProfileMaterializationError::TypedReloadMissing(name))
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn safe_fixed_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.is_ascii()
        && !name
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':' | '\0'))
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn require_absolute_normalized(
    path: &Path,
    label: &'static str,
) -> Result<(), ProfileMaterializationError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProfileMaterializationError::UnsafePath(label));
    }
    Ok(())
}

fn read_bounded(
    file: &mut File,
    max: u64,
    label: &'static str,
) -> Result<Vec<u8>, ProfileMaterializationError> {
    let length = file.metadata()?.len();
    if length == 0 || length > max {
        return Err(ProfileMaterializationError::InputTooLarge {
            label,
            actual: length,
            max,
        });
    }
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != length {
        return Err(ProfileMaterializationError::InputChanged(label));
    }
    Ok(bytes)
}

fn read_exact_sealed(
    file: &mut File,
    expected: PinnedSupportBlobV1,
    label: &'static str,
) -> Result<Vec<u8>, ProfileMaterializationError> {
    let bytes = read_bounded(file, MAX_STATIC_SUPPORT_BLOB_BYTES_V1, label)?;
    if bytes.len() as u64 != expected.byte_len || sha256(&bytes) != expected.sha256 {
        return Err(ProfileMaterializationError::StaticSupportSealMismatch(
            label,
        ));
    }
    Ok(bytes)
}

fn read_exact_profile_blob(
    file: &mut File,
    expected: &SealedBlobV1,
    label: &'static str,
) -> Result<Vec<u8>, ProfileMaterializationError> {
    let bytes = read_bounded(file, MAX_STATIC_SUPPORT_BLOB_BYTES_V1, label)?;
    if bytes.len() as u64 != expected.byte_len || sha256(&bytes) != expected.sha256 {
        return Err(ProfileMaterializationError::TypedReloadSeal(
            expected.path.clone(),
        ));
    }
    Ok(bytes)
}

#[cfg(windows)]
fn open_regular_no_follow(
    path: &Path,
    label: &'static str,
) -> Result<File, ProfileMaterializationError> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|source| ProfileMaterializationError::InputOpen { label, source })?;
    validate_regular_single_link(&file, label)?;
    Ok(file)
}

#[cfg(not(windows))]
fn open_regular_no_follow(
    _path: &Path,
    _label: &'static str,
) -> Result<File, ProfileMaterializationError> {
    Err(ProfileMaterializationError::UnsupportedPlatform)
}

#[cfg(windows)]
fn open_directory_no_follow(
    path: &Path,
    label: &'static str,
) -> Result<File, ProfileMaterializationError> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_SHARE_READ,
    };
    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|_| ProfileMaterializationError::UnsafePath(label))?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata_is_reparse(&metadata) {
        return Err(ProfileMaterializationError::UnsafePath(label));
    }
    Ok(file)
}

#[cfg(not(windows))]
fn open_directory_no_follow(
    _path: &Path,
    _label: &'static str,
) -> Result<File, ProfileMaterializationError> {
    Err(ProfileMaterializationError::UnsupportedPlatform)
}

#[cfg(windows)]
fn validate_regular_single_link(
    file: &File,
    label: &'static str,
) -> Result<(), ProfileMaterializationError> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };
    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the duration of the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0
        || info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        || info.nNumberOfLinks != 1
    {
        return Err(ProfileMaterializationError::UnsafePath(label));
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(windows)]
fn create_new_output_root(output_root: &Path) -> Result<(File, File), ProfileMaterializationError> {
    let parent = output_root
        .parent()
        .ok_or(ProfileMaterializationError::UnsafePath("output root"))?;
    let parent_pin = open_directory_no_follow(parent, "output parent")?;
    match std::fs::create_dir(output_root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ProfileMaterializationError::OutputExists);
        }
        Err(error) => return Err(ProfileMaterializationError::Io(error)),
    }
    let output_pin = open_directory_no_follow(output_root, "new output root")?;
    Ok((parent_pin, output_pin))
}

#[cfg(not(windows))]
fn create_new_output_root(
    _output_root: &Path,
) -> Result<(File, File), ProfileMaterializationError> {
    Err(ProfileMaterializationError::UnsupportedPlatform)
}

#[cfg(windows)]
fn write_new_output_file(
    output_root: &Path,
    name: &'static str,
    bytes: &[u8],
) -> Result<PublishedFileV1, ProfileMaterializationError> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    if !safe_fixed_file_name(name) || bytes.is_empty() {
        return Err(ProfileMaterializationError::UnsafeOutputFile(name));
    }
    let path = output_root.join(name);
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ProfileMaterializationError::OutputFileExists(name)
            } else {
                ProfileMaterializationError::Io(error)
            }
        })?;
    validate_regular_single_link(&file, name)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    validate_regular_single_link(&file, name)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(PublishedFileV1 {
        name,
        file,
        seal: SealedBlobV1 {
            path: name.to_owned(),
            byte_len: bytes.len() as u64,
            sha256: sha256(bytes),
        },
    })
}

#[cfg(not(windows))]
fn write_new_output_file(
    _output_root: &Path,
    _name: &'static str,
    _bytes: &[u8],
) -> Result<PublishedFileV1, ProfileMaterializationError> {
    Err(ProfileMaterializationError::UnsupportedPlatform)
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileMaterializationError {
    #[error("profile materialization is supported only on Windows")]
    UnsupportedPlatform,
    #[error("unsafe or unavailable {0} path")]
    UnsafePath(&'static str),
    #[error("cannot safely open {label}: {source}")]
    InputOpen {
        label: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("source and output profile roots must be distinct and non-nested")]
    UnsafePathRelationship,
    #[error("{label} is {actual} bytes; maximum is {max}")]
    InputTooLarge {
        label: &'static str,
        actual: u64,
        max: u64,
    },
    #[error("{0} changed while held open")]
    InputChanged(&'static str),
    #[error("static support manifest schema is unsupported")]
    StaticSupportSchema,
    #[error("static support target does not match a supported capture generation")]
    StaticTargetMismatch,
    #[error("static support oracle identity does not match the selected capture generation")]
    StaticOracleMismatch,
    #[error("static support omits a required nonzero measurement")]
    StaticMeasurementMissing,
    #[error("static support string {0} is empty, non-ASCII, or too long")]
    InvalidStaticString(&'static str),
    #[error("static support contains an invalid blob seal")]
    InvalidSupportSeal,
    #[error("static support payload {0} does not match its pinned seal")]
    StaticSupportSealMismatch(&'static str),
    #[error("decoded capture projection is incomplete")]
    DecodedProjectionIncomplete,
    #[error("size arithmetic overflow")]
    SizeOverflow,
    #[error("output root already exists")]
    OutputExists,
    #[error("output file {0} already exists")]
    OutputFileExists(&'static str),
    #[error("unsafe output file {0}")]
    UnsafeOutputFile(&'static str),
    #[error("typed reload is missing {0}")]
    TypedReloadMissing(&'static str),
    #[error("typed reload has unknown path {0:?}")]
    TypedReloadPath(String),
    #[error("typed reload seal mismatch for {0:?}")]
    TypedReloadSeal(String),
    #[error("materialized profile crossed the qualification boundary")]
    QualificationBoundary,
    #[error("materialization receipt is invalid")]
    InvalidMaterializationReceipt,
    #[error("qualified profile promotion receipt is invalid")]
    InvalidQualifiedPromotionReceipt,
    #[error("capture decode failed: {0}")]
    Capture(#[from] CaptureDecodeError),
    #[error("compiler profile is invalid: {0}")]
    Profile(#[from] CompilerProfileError),
    #[error("qualification payload is invalid: {0}")]
    Qualification(#[from] QualificationError),
    #[error("offline qualification artifact is invalid: {0}")]
    OfflineQualification(#[from] OfflineQualificationErrorV1),
    #[error("registry projection is invalid: {0}")]
    Registry(#[from] crate::compiler_profile::registry::RegistryProfileError),
    #[error("frontend projection is invalid: {0}")]
    Frontend(#[from] crate::compiler_profile::frontend::FrontendProfileError),
    #[error("invalid materialization JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("materialization I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::compiler_profile::manifest::{FileSealV1, PeCodeViewV1, Sha1Digest, Sha256Digest};

    fn file_seal(bytes: &[u8], steam: bool) -> FileSealV1 {
        FileSealV1 {
            byte_len: bytes.len() as u64,
            sha256: sha256(bytes),
            steam_content_sha1: steam.then(|| Sha1Digest::from_bytes([0x51; 20])),
        }
    }

    fn support_manifest(payload: PinnedSupportBlobV1) -> StaticProfileSupportManifestV1 {
        let target = &CAPTURE_TARGET_24878692;
        StaticProfileSupportManifestV1 {
            schema: STATIC_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
            schema_version: STATIC_SUPPORT_MANIFEST_SCHEMA_VERSION_V1,
            target: CompilerTargetV1 {
                steam_app_id: target.steam_app_id,
                steam_build_id: target.steam_build_id,
                depot_id: target.depot_id,
                depot_manifest_gid: target.depot_manifest_gid,
                platform: CompilerPlatformV1::Windows,
                architecture: CompilerArchitectureV1::X86_64,
                build_configuration: CompilerBuildConfigurationV1::Shipping,
            },
            oracle: CompilerOracleV1 {
                executable: FileSealV1 {
                    byte_len: target.executable_bytes,
                    sha256: Sha256Digest::from_bytes(target.executable_sha256),
                    steam_content_sha1: Some(Sha1Digest::from_bytes([1; 20])),
                },
                binds_cache: file_seal(b"binds", true),
                shipping_cache: file_seal(b"shipping", true),
                depot_manifest: file_seal(b"manifest", false),
                pe_codeview: PeCodeViewV1 {
                    guid: target.codeview_guid.to_owned(),
                    age: target.codeview_age,
                },
            },
            binds: BindsProfileV1 {
                wire_schema_version: 1,
                struct_count: 1,
                class_count: 1,
                method_count: 1,
                struct_property_count: 1,
                class_property_count: 1,
                canonical_database_sha256: Sha256Digest::from_bytes([0x61; 32]),
            },
            unreal_metadata_schema_version: 1,
            opcode_table_version: "synthetic-opcodes-v1".to_owned(),
            cache_format_version: 1,
            required_probe_suite_version: FULL_QUALIFICATION_SUITE_ID_V1.to_owned(),
            payloads: StaticSupportPayloadSealsV1 {
                reflected_type_graph: payload,
                opcode_table: payload,
                operand_schema: payload,
                codegen_probe_corpus: payload,
                expected_probe_results: payload,
                serializer_schema: payload,
                reference_table_order: payload,
                normalized_oracle_corpus: payload,
                diagnostic_parity: payload,
                semantic_parity: payload,
            },
        }
    }

    #[test]
    fn static_support_accepts_each_generation_and_rejects_cross_generation_oracle_identity() {
        let payload = PinnedSupportBlobV1 {
            byte_len: 1,
            sha256: Sha256Digest::from_bytes([0x41; 32]),
        };
        let mut historical = support_manifest(payload);
        let target = &super::super::CAPTURE_TARGET_24539464;
        historical.target.steam_app_id = target.steam_app_id;
        historical.target.steam_build_id = target.steam_build_id;
        historical.target.depot_id = target.depot_id;
        historical.target.depot_manifest_gid = target.depot_manifest_gid;
        historical.oracle.executable.byte_len = target.executable_bytes;
        historical.oracle.executable.sha256 = Sha256Digest::from_bytes(target.executable_sha256);
        historical.oracle.pe_codeview.guid = target.codeview_guid.to_owned();
        historical.oracle.pe_codeview.age = target.codeview_age;
        historical.validate().unwrap();

        historical.target.depot_manifest_gid = CAPTURE_TARGET_24878692.depot_manifest_gid;
        assert!(matches!(
            historical.validate(),
            Err(ProfileMaterializationError::StaticTargetMismatch)
        ));
        historical.target.depot_manifest_gid = target.depot_manifest_gid;

        historical.oracle.executable.sha256 =
            Sha256Digest::from_bytes(CAPTURE_TARGET_24878692.executable_sha256);
        assert!(matches!(
            historical.validate(),
            Err(ProfileMaterializationError::StaticOracleMismatch)
        ));
    }

    fn prepare_support(root: &Path) -> (PathBuf, PathBuf) {
        let support_root = root.join("support");
        std::fs::create_dir(&support_root).unwrap();
        let payload_bytes = br#"{"synthetic":true,"qualified":false}"#;
        for (_, name) in STATIC_BLOB_FILES_V1 {
            std::fs::write(support_root.join(name), payload_bytes).unwrap();
        }
        let seal = PinnedSupportBlobV1 {
            byte_len: payload_bytes.len() as u64,
            sha256: sha256(payload_bytes),
        };
        let manifest_path = root.join("static-support.json");
        std::fs::write(
            &manifest_path,
            support_manifest(seal).to_json_pretty().unwrap(),
        )
        .unwrap();
        (manifest_path, support_root)
    }

    fn promotion_fixture(
        source_profile: &CompilerProfileV1,
    ) -> (CompilerProbeCorpusV1, OfflineQualificationPromotionV1) {
        crate::compiler_profile::qualification_suite::tests::canonical_full_promotion_fixture_v1(
            source_profile.profile_sha256,
            source_profile.target.clone(),
        )
    }

    #[test]
    fn decoded_capture_materializes_unqualified_and_reload_detects_corruption() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let capture_path = root.join("fixture.capture");
        std::fs::write(
            &capture_path,
            super::super::decode::tests::complete_capture_fixture(),
        )
        .unwrap();
        let (support_manifest_path, support_root) = prepare_support(&root);
        let output_root = root.join("materialized-profile");

        let mut forged_bound_graph_hook =
            decode_capture_v1(&std::fs::read(&capture_path).unwrap()).unwrap();
        forged_bound_graph_hook
            .frontend_configs
            .preprocessor
            .external_hooks
            .process_chunks
            .bound = true;
        let pinned_support =
            load_pinned_static_profile_support_v1(&support_manifest_path, &support_root).unwrap();
        assert!(matches!(
            materialize_unqualified_profile_package_v1(
                &forged_bound_graph_hook,
                &pinned_support,
                &root.join("forged-bound-graph-hook"),
            ),
            Err(ProfileMaterializationError::DecodedProjectionIncomplete)
        ));

        let result = materialize_unqualified_profile_package_from_paths_v1(
            &capture_path,
            &support_manifest_path,
            &support_root,
            &output_root,
        )
        .unwrap();
        assert!(!result.qualified);
        assert_eq!(result.materialized_file_count, 18);

        let profile_json = std::fs::read(output_root.join(PROFILE_MANIFEST_FILE_V1)).unwrap();
        assert!(matches!(
            CompilerProfileV1::from_json(&profile_json),
            Err(CompilerProfileError::NotQualified)
        ));
        let profile = CompilerProfileV1::from_unqualified_json(&profile_json).unwrap();
        assert_eq!(profile.profile_sha256, result.profile_sha256);
        assert_eq!(
            reload_unqualified_profile_package_v1(&output_root).unwrap(),
            profile
        );

        let receipt = UnqualifiedProfileMaterializationReceiptV1::from_json(
            &std::fs::read(output_root.join(MATERIALIZATION_RECEIPT_FILE_V1)).unwrap(),
        )
        .unwrap();
        assert!(!receipt.qualified);
        assert_eq!(receipt.profile_sha256, profile.profile_sha256);

        for drift in 0..4 {
            let mut foreign_profile = profile.clone();
            match drift {
                0 => foreign_profile.target.steam_app_id += 1,
                1 => foreign_profile.target.steam_build_id += 1,
                2 => foreign_profile.target.depot_id += 1,
                3 => foreign_profile.target.depot_manifest_gid += 1,
                _ => unreachable!(),
            }
            foreign_profile.seal().unwrap();
            let foreign_manifest_json = serde_json::to_vec_pretty(&foreign_profile).unwrap();
            let mut foreign_receipt = receipt.clone();
            foreign_receipt.profile_sha256 = foreign_profile.profile_sha256;
            let manifest_seal = foreign_receipt
                .files
                .iter_mut()
                .find(|file| file.path == PROFILE_MANIFEST_FILE_V1)
                .unwrap();
            manifest_seal.byte_len = foreign_manifest_json.len() as u64;
            manifest_seal.sha256 = sha256(&foreign_manifest_json);
            foreign_receipt.seal().unwrap();
            std::fs::write(
                output_root.join(PROFILE_MANIFEST_FILE_V1),
                foreign_manifest_json,
            )
            .unwrap();
            std::fs::write(
                output_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
                serde_json::to_vec_pretty(&foreign_receipt).unwrap(),
            )
            .unwrap();
            assert!(matches!(
                reload_unqualified_profile_package_v1(&output_root),
                Err(ProfileMaterializationError::StaticTargetMismatch)
            ));
        }
        std::fs::write(output_root.join(PROFILE_MANIFEST_FILE_V1), &profile_json).unwrap();
        std::fs::write(
            output_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();

        let property_path = output_root.join(ENGINE_PROPERTIES_FILE);
        let original_property = std::fs::read(&property_path).unwrap();
        let mut corrupted = original_property.clone();
        corrupted[0] ^= 1;
        std::fs::write(&property_path, corrupted).unwrap();
        assert!(matches!(
            reload_unqualified_profile_package_v1(&output_root),
            Err(ProfileMaterializationError::TypedReloadSeal(_))
                | Err(ProfileMaterializationError::Registry(_))
        ));

        std::fs::write(&property_path, original_property).unwrap();
        let mut forged_receipt = receipt;
        forged_receipt.files[0].byte_len += 1;
        forged_receipt.seal().unwrap();
        std::fs::write(
            output_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
            serde_json::to_vec_pretty(&forged_receipt).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            reload_unqualified_profile_package_v1(&output_root),
            Err(ProfileMaterializationError::InvalidMaterializationReceipt)
        ));
    }

    #[test]
    fn support_drift_and_output_collision_fail_before_clobbering() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let capture_path = root.join("fixture.capture");
        std::fs::write(
            &capture_path,
            super::super::decode::tests::complete_capture_fixture(),
        )
        .unwrap();
        let (support_manifest_path, support_root) = prepare_support(&root);
        std::fs::write(support_root.join("opcode-table.bin"), b"drift").unwrap();
        let refused_output = root.join("refused-profile");
        assert!(matches!(
            materialize_unqualified_profile_package_from_paths_v1(
                &capture_path,
                &support_manifest_path,
                &support_root,
                &refused_output,
            ),
            Err(ProfileMaterializationError::StaticSupportSealMismatch(
                "opcode-table.bin"
            ))
        ));
        assert!(!refused_output.exists());

        let second = root.join("second");
        std::fs::create_dir(&second).unwrap();
        let (support_manifest_path, support_root) = prepare_support(&second);
        let output_root = root.join("existing-profile");
        std::fs::create_dir(&output_root).unwrap();
        std::fs::write(output_root.join("keep.txt"), b"keep").unwrap();
        assert!(matches!(
            materialize_unqualified_profile_package_from_paths_v1(
                &capture_path,
                &support_manifest_path,
                &support_root,
                &output_root,
            ),
            Err(ProfileMaterializationError::OutputExists)
        ));
        assert_eq!(
            std::fs::read(output_root.join("keep.txt")).unwrap(),
            b"keep"
        );
    }

    #[test]
    fn create_new_promotion_publishes_and_typed_reloads_qualified_package() {
        let tree_vector = vec![
            MaterializedFileSealV1 {
                path: "a.json".to_owned(),
                byte_len: 5,
                sha256: sha256(b"alpha"),
            },
            MaterializedFileSealV1 {
                path: "z.bin".to_owned(),
                byte_len: 3,
                sha256: sha256(&[0, 1, 2]),
            },
        ];
        assert_eq!(
            qualified_profile_tree_sha256_v1(&tree_vector).to_string(),
            "9792a24a47c6628dd81200843908c370b162b65a5caf7b1d45d175c5fca365a6"
        );
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let capture_path = root.join("fixture.capture");
        std::fs::write(
            &capture_path,
            super::super::decode::tests::complete_capture_fixture(),
        )
        .unwrap();
        let (support_manifest_path, support_root) = prepare_support(&root);
        let source_root = root.join("unqualified-source");
        materialize_unqualified_profile_package_from_paths_v1(
            &capture_path,
            &support_manifest_path,
            &support_root,
            &source_root,
        )
        .unwrap();
        let source_manifest_before =
            std::fs::read(source_root.join(PROFILE_MANIFEST_FILE_V1)).unwrap();
        let source_receipt_before =
            std::fs::read(source_root.join(MATERIALIZATION_RECEIPT_FILE_V1)).unwrap();
        let source_profile =
            CompilerProfileV1::from_unqualified_json(&source_manifest_before).unwrap();
        let (corpus, promotion) = promotion_fixture(&source_profile);

        let mut replay_target = source_profile.clone();
        replay_target.oracle.depot_manifest.sha256 = Sha256Digest::from_bytes([0x77; 32]);
        replay_target.seal().unwrap();
        assert_ne!(replay_target.profile_sha256, source_profile.profile_sha256);
        let replay_manifest = serde_json::to_vec_pretty(&replay_target).unwrap();
        let mut replay_receipt =
            UnqualifiedProfileMaterializationReceiptV1::from_json(&source_receipt_before).unwrap();
        replay_receipt.profile_sha256 = replay_target.profile_sha256;
        let replay_manifest_seal = replay_receipt
            .files
            .iter_mut()
            .find(|file| file.path == PROFILE_MANIFEST_FILE_V1)
            .unwrap();
        replay_manifest_seal.byte_len = replay_manifest.len() as u64;
        replay_manifest_seal.sha256 = sha256(&replay_manifest);
        replay_receipt.seal().unwrap();
        std::fs::write(source_root.join(PROFILE_MANIFEST_FILE_V1), replay_manifest).unwrap();
        std::fs::write(
            source_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
            serde_json::to_vec_pretty(&replay_receipt).unwrap(),
        )
        .unwrap();
        assert_eq!(
            reload_unqualified_profile_package_v1(&source_root).unwrap(),
            replay_target
        );
        let replay_output = root.join("replayed-profile-output");
        assert!(matches!(
            promote_unqualified_profile_package_v1(
                &source_root,
                &replay_output,
                &corpus,
                &promotion,
            ),
            Err(ProfileMaterializationError::QualificationBoundary)
        ));
        assert!(!replay_output.exists());
        std::fs::write(
            source_root.join(PROFILE_MANIFEST_FILE_V1),
            &source_manifest_before,
        )
        .unwrap();
        std::fs::write(
            source_root.join(MATERIALIZATION_RECEIPT_FILE_V1),
            &source_receipt_before,
        )
        .unwrap();
        let output_root = root.join("qualified-output");
        let result =
            promote_unqualified_profile_package_v1(&source_root, &output_root, &corpus, &promotion)
                .unwrap();
        assert!(result.qualified);
        assert_eq!(result.materialized_file_count, 20);
        let profile = CompilerProfileV1::from_json(
            &std::fs::read(output_root.join(PROFILE_MANIFEST_FILE_V1)).unwrap(),
        )
        .unwrap();
        assert!(profile.qualification.qualified);
        assert_eq!(profile.profile_sha256, result.profile_sha256);
        let receipt = QualifiedProfilePromotionReceiptV1::from_json(
            &std::fs::read(output_root.join(QUALIFIED_PROMOTION_RECEIPT_FILE_V1)).unwrap(),
        )
        .unwrap();
        assert!(receipt.qualified);
        assert_eq!(receipt.profile_sha256, profile.profile_sha256);
        assert_eq!(
            reload_qualified_profile_package_v1(&output_root).unwrap(),
            profile
        );
        let verification = verify_qualified_profile_package_v1(&output_root).unwrap();
        assert_eq!(verification.profile(), &profile);
        assert_eq!(verification.file_count(), 20);
        assert_eq!(
            verification.manifest_sha256(),
            sha256(&std::fs::read(output_root.join(PROFILE_MANIFEST_FILE_V1)).unwrap())
        );
        assert_eq!(
            verification.promotion_receipt_sha256(),
            sha256(&std::fs::read(output_root.join(QUALIFIED_PROMOTION_RECEIPT_FILE_V1)).unwrap())
        );
        assert_ne!(
            verification.tree_sha256(),
            Sha256Digest::from_bytes([0; 32])
        );
        assert_eq!(
            std::fs::read(source_root.join(PROFILE_MANIFEST_FILE_V1)).unwrap(),
            source_manifest_before
        );

        assert!(matches!(
            promote_unqualified_profile_package_v1(&source_root, &output_root, &corpus, &promotion,),
            Err(ProfileMaterializationError::OutputExists)
        ));

        let artifact_path = output_root.join(STANDALONE_QUALIFICATION_ARTIFACT_MANIFEST_FILE_V1);
        let mut substituted = std::fs::read(&artifact_path).unwrap();
        let last = substituted.len() - 1;
        substituted[last] ^= 1;
        std::fs::write(&artifact_path, substituted).unwrap();
        assert!(matches!(
            reload_qualified_profile_package_v1(&output_root),
            Err(ProfileMaterializationError::InvalidQualifiedPromotionReceipt)
                | Err(ProfileMaterializationError::OfflineQualification(_))
        ));
    }
}
