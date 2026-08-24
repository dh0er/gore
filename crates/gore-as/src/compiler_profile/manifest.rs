//! Digest-bound manifest for one completely captured G1R compiler profile.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

pub const COMPILER_PROFILE_SCHEMA: &str = "gore.as.compiler-profile";
pub const COMPILER_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const MAX_COMPILER_PROFILE_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROFILE_BLOB_PATH_BYTES: usize = 512;
const PROFILE_HASH_DOMAIN: &[u8] = b"gore-as-compiler-profile-v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedDigest<const N: usize>([u8; N]);

pub type Sha1Digest = FixedDigest<20>;
pub type Sha256Digest = FixedDigest<32>;

impl<const N: usize> FixedDigest<N> {
    pub const fn from_bytes(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    pub fn from_hex(value: &str) -> Result<Self, DigestParseError> {
        if value.len() != N * 2 {
            return Err(DigestParseError::Length {
                expected: N * 2,
                actual: value.len(),
            });
        }
        let mut out = [0u8; N];
        for (index, byte) in out.iter_mut().enumerate() {
            let at = index * 2;
            *byte = (hex_nibble(value.as_bytes()[at], at)? << 4)
                | hex_nibble(value.as_bytes()[at + 1], at + 1)?;
        }
        Ok(Self(out))
    }
}

impl<const N: usize> fmt::Display for FixedDigest<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl<const N: usize> Serialize for FixedDigest<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, const N: usize> Deserialize<'de> for FixedDigest<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(serde::de::Error::custom)
    }
}

fn hex_nibble(byte: u8, index: usize) -> Result<u8, DigestParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(DigestParseError::Character {
            index,
            character: byte as char,
        }),
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DigestParseError {
    #[error("digest must contain exactly {expected} hex characters, got {actual}")]
    Length { expected: usize, actual: usize },
    #[error("digest contains non-hex character {character:?} at index {index}")]
    Character { index: usize, character: char },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSealV1 {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_content_sha1: Option<Sha1Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedBlobV1 {
    /// Slash-separated path below the profile root. Absolute and traversal paths are rejected.
    pub path: String,
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerPlatformV1 {
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerArchitectureV1 {
    X86_64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerBuildConfigurationV1 {
    Shipping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerTargetV1 {
    pub steam_app_id: u32,
    pub steam_build_id: u64,
    pub depot_id: u32,
    pub depot_manifest_gid: u64,
    pub platform: CompilerPlatformV1,
    pub architecture: CompilerArchitectureV1,
    pub build_configuration: CompilerBuildConfigurationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeCodeViewV1 {
    pub guid: String,
    pub age: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerOracleV1 {
    pub executable: FileSealV1,
    pub binds_cache: FileSealV1,
    pub shipping_cache: FileSealV1,
    pub depot_manifest: FileSealV1,
    pub pe_codeview: PeCodeViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindsProfileV1 {
    pub wire_schema_version: u32,
    pub struct_count: u64,
    pub class_count: u64,
    pub method_count: u64,
    pub struct_property_count: u64,
    pub class_property_count: u64,
    pub canonical_database_sha256: Sha256Digest,
}

impl BindsProfileV1 {
    pub fn from_database(database: &super::binds::BindsDatabase) -> Self {
        Self {
            wire_schema_version: 1,
            struct_count: database.structs.len() as u64,
            class_count: database.classes.len() as u64,
            method_count: database.method_count() as u64,
            struct_property_count: database.struct_property_count() as u64,
            class_property_count: database.class_property_count() as u64,
            canonical_database_sha256: Sha256Digest::from_bytes(database.canonical_sha256()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineProfileV1 {
    pub as_create_version: u32,
    pub ordered_engine_properties: SealedBlobV1,
    pub registration_trace: SealedBlobV1,
    pub registration_trace_count: u64,
    pub post_bind_snapshot: SealedBlobV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnrealSemanticsProfileV1 {
    pub reflected_type_graph: SealedBlobV1,
    pub metadata_schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendProfileV1 {
    pub preprocessor_config: SealedBlobV1,
    pub class_generator_config: SealedBlobV1,
    pub compiler_options: SealedBlobV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BytecodeProfileV1 {
    pub opcode_table_version: String,
    pub opcode_table: SealedBlobV1,
    pub operand_schema: SealedBlobV1,
    pub codegen_probe_corpus: SealedBlobV1,
    pub expected_probe_results: SealedBlobV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheWriterProfileV1 {
    pub format_version: u32,
    pub serializer_schema: SealedBlobV1,
    pub build_identifier: u32,
    pub reference_table_order: SealedBlobV1,
    pub normalized_oracle_corpus: SealedBlobV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationProfileV1 {
    pub required_probe_suite_version: String,
    pub diagnostic_parity: SealedBlobV1,
    pub semantic_parity: SealedBlobV1,
    pub qualified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerProfileV1 {
    pub schema: String,
    pub schema_version: u32,
    pub target: CompilerTargetV1,
    pub oracle: CompilerOracleV1,
    pub binds: BindsProfileV1,
    pub engine: EngineProfileV1,
    pub unreal_semantics: UnrealSemanticsProfileV1,
    pub frontend: FrontendProfileV1,
    pub bytecode: BytecodeProfileV1,
    pub cache_writer: CacheWriterProfileV1,
    pub qualification: QualificationProfileV1,
    pub profile_sha256: Sha256Digest,
}

#[derive(Serialize)]
struct CompilerProfileHashPayloadV1<'a> {
    schema: &'a str,
    schema_version: u32,
    target: &'a CompilerTargetV1,
    oracle: &'a CompilerOracleV1,
    binds: &'a BindsProfileV1,
    engine: &'a EngineProfileV1,
    unreal_semantics: &'a UnrealSemanticsProfileV1,
    frontend: &'a FrontendProfileV1,
    bytecode: &'a BytecodeProfileV1,
    cache_writer: &'a CacheWriterProfileV1,
    qualification: &'a QualificationProfileV1,
}

impl CompilerProfileV1 {
    pub fn from_json(bytes: &[u8]) -> Result<Self, CompilerProfileError> {
        let profile = Self::parse_bounded_json(bytes)?;
        profile.validate_complete()?;
        Ok(profile)
    }

    /// Parse a materialized profile which is structurally complete but deliberately unqualified.
    ///
    /// This is a separate trust state, not a relaxation of [`Self::from_json`]. Product package
    /// resolution continues to call `from_json` and therefore rejects these artifacts.
    pub fn from_unqualified_json(bytes: &[u8]) -> Result<Self, CompilerProfileError> {
        let profile = Self::parse_bounded_json(bytes)?;
        profile.validate_unqualified_materialized()?;
        Ok(profile)
    }

    fn parse_bounded_json(bytes: &[u8]) -> Result<Self, CompilerProfileError> {
        if bytes.len() > MAX_COMPILER_PROFILE_JSON_BYTES {
            return Err(CompilerProfileError::JsonTooLarge {
                actual: bytes.len(),
                max: MAX_COMPILER_PROFILE_JSON_BYTES,
            });
        }
        Ok(serde_json::from_slice(bytes)?)
    }

    pub fn computed_sha256(&self) -> Result<Sha256Digest, CompilerProfileError> {
        let payload = CompilerProfileHashPayloadV1 {
            schema: &self.schema,
            schema_version: self.schema_version,
            target: &self.target,
            oracle: &self.oracle,
            binds: &self.binds,
            engine: &self.engine,
            unreal_semantics: &self.unreal_semantics,
            frontend: &self.frontend,
            bytecode: &self.bytecode,
            cache_writer: &self.cache_writer,
            qualification: &self.qualification,
        };
        let canonical = serde_json::to_vec(&payload)?;
        let mut hash = Sha256::new();
        hash.update(PROFILE_HASH_DOMAIN);
        hash.update(canonical);
        Ok(Sha256Digest::from_bytes(hash.finalize().into()))
    }

    pub fn seal(&mut self) -> Result<(), CompilerProfileError> {
        self.profile_sha256 = self.computed_sha256()?;
        Ok(())
    }

    pub fn validate_complete(&self) -> Result<(), CompilerProfileError> {
        self.validate_structure(QualificationStateV1::Qualified)
    }

    /// Validate the exact structure and canonical seal of a deliberately unqualified profile.
    /// Such a profile can be inspected and resumed by offline tooling but cannot enter the
    /// product resolver or mint qualified compiler authority.
    pub fn validate_unqualified_materialized(&self) -> Result<(), CompilerProfileError> {
        self.validate_structure(QualificationStateV1::Unqualified)
    }

    fn validate_structure(
        &self,
        qualification_state: QualificationStateV1,
    ) -> Result<(), CompilerProfileError> {
        if self.schema != COMPILER_PROFILE_SCHEMA {
            return Err(CompilerProfileError::Schema(self.schema.clone()));
        }
        if self.schema_version != COMPILER_PROFILE_SCHEMA_VERSION {
            return Err(CompilerProfileError::SchemaVersion(self.schema_version));
        }
        match (qualification_state, self.qualification.qualified) {
            (QualificationStateV1::Qualified, false) => {
                return Err(CompilerProfileError::NotQualified);
            }
            (QualificationStateV1::Unqualified, true) => {
                return Err(CompilerProfileError::UnexpectedQualifiedMaterialization);
            }
            _ => {}
        }
        if self.target.steam_app_id == 0
            || self.target.steam_build_id == 0
            || self.target.depot_id == 0
            || self.target.depot_manifest_gid == 0
        {
            return Err(CompilerProfileError::TargetIdentityMissing);
        }

        for (field, seal, steam_sha_required) in [
            ("oracle.executable", &self.oracle.executable, true),
            ("oracle.binds_cache", &self.oracle.binds_cache, true),
            ("oracle.shipping_cache", &self.oracle.shipping_cache, true),
            ("oracle.depot_manifest", &self.oracle.depot_manifest, false),
        ] {
            if seal.byte_len == 0 {
                return Err(CompilerProfileError::EmptyFileSeal { field });
            }
            if steam_sha_required && seal.steam_content_sha1.is_none() {
                return Err(CompilerProfileError::SteamContentSealMissing { field });
            }
        }

        if self.binds.wire_schema_version == 0
            || self.binds.struct_count == 0
            || self.binds.class_count == 0
            || self.binds.method_count == 0
            || self.binds.struct_property_count == 0
            || self.binds.class_property_count == 0
            || self.engine.as_create_version == 0
            || self.engine.registration_trace_count == 0
            || self.unreal_semantics.metadata_schema_version == 0
            || self.cache_writer.format_version == 0
        {
            return Err(CompilerProfileError::RequiredMeasurementMissing);
        }
        for (field, value) in [
            (
                "oracle.pe_codeview.guid",
                self.oracle.pe_codeview.guid.as_str(),
            ),
            (
                "bytecode.opcode_table_version",
                &self.bytecode.opcode_table_version,
            ),
            (
                "qualification.required_probe_suite_version",
                &self.qualification.required_probe_suite_version,
            ),
        ] {
            if value.trim().is_empty() {
                return Err(CompilerProfileError::RequiredStringMissing { field });
            }
        }

        let blobs = [
            (
                "engine.ordered_engine_properties",
                &self.engine.ordered_engine_properties,
            ),
            ("engine.registration_trace", &self.engine.registration_trace),
            ("engine.post_bind_snapshot", &self.engine.post_bind_snapshot),
            (
                "unreal_semantics.reflected_type_graph",
                &self.unreal_semantics.reflected_type_graph,
            ),
            (
                "frontend.preprocessor_config",
                &self.frontend.preprocessor_config,
            ),
            (
                "frontend.class_generator_config",
                &self.frontend.class_generator_config,
            ),
            ("frontend.compiler_options", &self.frontend.compiler_options),
            ("bytecode.opcode_table", &self.bytecode.opcode_table),
            ("bytecode.operand_schema", &self.bytecode.operand_schema),
            (
                "bytecode.codegen_probe_corpus",
                &self.bytecode.codegen_probe_corpus,
            ),
            (
                "bytecode.expected_probe_results",
                &self.bytecode.expected_probe_results,
            ),
            (
                "cache_writer.serializer_schema",
                &self.cache_writer.serializer_schema,
            ),
            (
                "cache_writer.reference_table_order",
                &self.cache_writer.reference_table_order,
            ),
            (
                "cache_writer.normalized_oracle_corpus",
                &self.cache_writer.normalized_oracle_corpus,
            ),
            (
                "qualification.diagnostic_parity",
                &self.qualification.diagnostic_parity,
            ),
            (
                "qualification.semantic_parity",
                &self.qualification.semantic_parity,
            ),
        ];
        let mut paths = BTreeMap::<String, (u64, Sha256Digest, &'static str)>::new();
        for (field, blob) in blobs {
            let path_key = validate_blob_path(field, &blob.path)?;
            if blob.byte_len == 0 {
                return Err(CompilerProfileError::EmptyBlob { field });
            }
            if let Some((byte_len, sha256, previous)) =
                paths.insert(path_key, (blob.byte_len, blob.sha256, field))
            {
                if byte_len != blob.byte_len || sha256 != blob.sha256 {
                    return Err(CompilerProfileError::ConflictingBlobPath {
                        field,
                        previous,
                        path: blob.path.clone(),
                    });
                }
            }
        }

        let computed = self.computed_sha256()?;
        if self.profile_sha256 != computed {
            return Err(CompilerProfileError::ProfileDigestMismatch {
                declared: self.profile_sha256,
                computed,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QualificationStateV1 {
    Qualified,
    Unqualified,
}

fn validate_blob_path(field: &'static str, path: &str) -> Result<String, CompilerProfileError> {
    let components_are_safe = path.split('/').all(|part| {
        let device_stem = part
            .split_once('.')
            .map_or(part, |(stem, _)| stem)
            .to_ascii_lowercase();
        let reserved_device = matches!(
            device_stem.as_str(),
            "con"
                | "prn"
                | "aux"
                | "nul"
                | "com1"
                | "com2"
                | "com3"
                | "com4"
                | "com5"
                | "com6"
                | "com7"
                | "com8"
                | "com9"
                | "lpt1"
                | "lpt2"
                | "lpt3"
                | "lpt4"
                | "lpt5"
                | "lpt6"
                | "lpt7"
                | "lpt8"
                | "lpt9"
        );
        !part.is_empty()
            && part != "."
            && part != ".."
            && part.len() <= 128
            && !part.ends_with('.')
            && !part.ends_with(' ')
            && !reserved_device
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    });
    if path.is_empty()
        || path.len() > MAX_PROFILE_BLOB_PATH_BYTES
        || !path.is_ascii()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
        || !components_are_safe
    {
        return Err(CompilerProfileError::UnsafeBlobPath {
            field,
            path: path.to_owned(),
        });
    }
    // The only qualified target is Windows, whose normal filesystem lookup is
    // case-insensitive. Use a canonical key so two manifest entries cannot
    // alias the same destination under different casing.
    Ok(path.to_ascii_lowercase())
}

#[derive(Debug, thiserror::Error)]
pub enum CompilerProfileError {
    #[error("compiler profile JSON is {actual} bytes; maximum is {max}")]
    JsonTooLarge { actual: usize, max: usize },
    #[error("compiler profile schema {0:?} is unsupported")]
    Schema(String),
    #[error("compiler profile schema version {0} is unsupported")]
    SchemaVersion(u32),
    #[error("compiler profile is not qualified")]
    NotQualified,
    #[error("unqualified materialization unexpectedly claims qualification")]
    UnexpectedQualifiedMaterialization,
    #[error("compiler profile target identity is incomplete")]
    TargetIdentityMissing,
    #[error("{field} has zero bytes")]
    EmptyFileSeal { field: &'static str },
    #[error("{field} has no Steam content SHA-1")]
    SteamContentSealMissing { field: &'static str },
    #[error("compiler profile is missing a required nonzero measurement")]
    RequiredMeasurementMissing,
    #[error("compiler profile field {field} is empty")]
    RequiredStringMissing { field: &'static str },
    #[error("compiler profile blob {field} has an unsafe path {path:?}")]
    UnsafeBlobPath { field: &'static str, path: String },
    #[error("compiler profile blob {field} has zero bytes")]
    EmptyBlob { field: &'static str },
    #[error("blob path {path:?} is used by {previous} and {field} with conflicting seals")]
    ConflictingBlobPath {
        field: &'static str,
        previous: &'static str,
        path: String,
    },
    #[error("compiler profile SHA-256 mismatch: declared {declared}, computed {computed}")]
    ProfileDigestMismatch {
        declared: Sha256Digest,
        computed: Sha256Digest,
    },
    #[error("invalid compiler profile JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha1(byte: u8) -> Sha1Digest {
        Sha1Digest::from_bytes([byte; 20])
    }

    fn sha256(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }

    fn file(byte: u8, steam: bool) -> FileSealV1 {
        FileSealV1 {
            byte_len: u64::from(byte) + 1,
            sha256: sha256(byte),
            steam_content_sha1: steam.then(|| sha1(byte)),
        }
    }

    fn blob(path: &str, byte: u8) -> SealedBlobV1 {
        SealedBlobV1 {
            path: path.to_owned(),
            byte_len: u64::from(byte) + 1,
            sha256: sha256(byte),
        }
    }

    fn complete_profile() -> CompilerProfileV1 {
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
                executable: file(1, true),
                binds_cache: file(2, true),
                shipping_cache: file(3, true),
                depot_manifest: file(4, false),
                pe_codeview: PeCodeViewV1 {
                    guid: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
                    age: 1,
                },
            },
            binds: BindsProfileV1 {
                wire_schema_version: 1,
                struct_count: 4_189,
                class_count: 7_007,
                method_count: 16_416,
                struct_property_count: 16_941,
                class_property_count: 14_966,
                canonical_database_sha256: sha256(5),
            },
            engine: EngineProfileV1 {
                as_create_version: 23_300,
                ordered_engine_properties: blob("engine/properties.json", 6),
                registration_trace: blob("engine/registrations.bin", 7),
                registration_trace_count: 1,
                post_bind_snapshot: blob("engine/post-bind.bin", 8),
            },
            unreal_semantics: UnrealSemanticsProfileV1 {
                reflected_type_graph: blob("unreal/type-graph.bin", 9),
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
                build_identifier: 0x9e37_7abe,
                reference_table_order: blob("cache/reference-order.json", 18),
                normalized_oracle_corpus: blob("cache/oracle-corpus.json", 19),
            },
            qualification: QualificationProfileV1 {
                required_probe_suite_version: "g1r-compiler-parity-v1".to_owned(),
                diagnostic_parity: blob("qualification/diagnostics.json", 20),
                semantic_parity: blob("qualification/semantic.json", 21),
                qualified: true,
            },
            profile_sha256: sha256(0),
        };
        profile.seal().unwrap();
        profile
    }

    #[test]
    fn complete_profile_round_trips_and_is_self_sealed() {
        let profile = complete_profile();
        profile.validate_complete().unwrap();
        let json = serde_json::to_vec_pretty(&profile).unwrap();
        assert_eq!(CompilerProfileV1::from_json(&json).unwrap(), profile);
        assert!(json
            .windows(64)
            .any(|window| window == profile.profile_sha256.to_string().as_bytes()));
    }

    #[test]
    fn qualification_and_self_digest_fail_closed() {
        let mut profile = complete_profile();
        profile.qualification.qualified = false;
        assert!(matches!(
            profile.validate_complete(),
            Err(CompilerProfileError::NotQualified)
        ));

        profile.seal().unwrap();
        let json = serde_json::to_vec(&profile).unwrap();
        assert_eq!(
            CompilerProfileV1::from_unqualified_json(&json).unwrap(),
            profile
        );
        assert!(matches!(
            CompilerProfileV1::from_json(&json),
            Err(CompilerProfileError::NotQualified)
        ));

        let qualified_json = serde_json::to_vec(&complete_profile()).unwrap();
        assert!(matches!(
            CompilerProfileV1::from_unqualified_json(&qualified_json),
            Err(CompilerProfileError::UnexpectedQualifiedMaterialization)
        ));

        let mut profile = complete_profile();
        profile.target.steam_build_id += 1;
        assert!(matches!(
            profile.validate_complete(),
            Err(CompilerProfileError::ProfileDigestMismatch { .. })
        ));
    }

    #[test]
    fn blob_paths_are_relative_and_seal_consistent() {
        let mut profile = complete_profile();
        profile.engine.registration_trace.path = "../registrations.bin".to_owned();
        profile.seal().unwrap();
        assert!(matches!(
            profile.validate_complete(),
            Err(CompilerProfileError::UnsafeBlobPath { .. })
        ));

        let mut profile = complete_profile();
        profile.engine.registration_trace.path = profile.engine.post_bind_snapshot.path.clone();
        profile.seal().unwrap();
        assert!(matches!(
            profile.validate_complete(),
            Err(CompilerProfileError::ConflictingBlobPath { .. })
        ));

        let mut profile = complete_profile();
        profile.engine.registration_trace.path =
            profile.engine.post_bind_snapshot.path.to_ascii_uppercase();
        profile.seal().unwrap();
        assert!(matches!(
            profile.validate_complete(),
            Err(CompilerProfileError::ConflictingBlobPath { .. })
        ));

        for unsafe_path in [
            "engine/NUL.json",
            "engine/trailing.",
            "engine/has space.json",
        ] {
            let mut profile = complete_profile();
            profile.engine.registration_trace.path = unsafe_path.to_owned();
            profile.seal().unwrap();
            assert!(matches!(
                profile.validate_complete(),
                Err(CompilerProfileError::UnsafeBlobPath { .. })
            ));
        }
    }

    #[test]
    fn unknown_manifest_fields_are_rejected() {
        let profile = complete_profile();
        let mut value = serde_json::to_value(profile).unwrap();
        value["unexpected"] = serde_json::json!(true);
        let error = CompilerProfileV1::from_json(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(matches!(error, CompilerProfileError::Json(_)));
        assert!(error.to_string().contains("unknown field"), "{error}");
    }
}
