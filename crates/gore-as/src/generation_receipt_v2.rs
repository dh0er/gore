//! Product-authoritative receipt for one retained full-graph compiler artifact.
//!
//! V1 receipts intentionally describe one base-GUID mini-cache. This V2 contract describes a
//! complete final graph, allows the compiler to emit a new deterministic cache GUID, binds every
//! Add/Edit/Delete decision, and accepts authority only from the embedded product-catalog
//! resolver. It is evidence, never a deployment capability.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::cache::{emit_all, header::CacheHeader, model};
use crate::compile::{
    FullGraphCompileArtifactV1, FullGraphDeletedModuleV1, FullGraphSourceDispositionV1,
    FullGraphSourceManifestEntryV1, MAX_FULL_GRAPH_COMPILE_CHANGES_V1,
    MAX_FULL_GRAPH_FINAL_MODULES_V1, MAX_FULL_GRAPH_SOURCE_BYTES_V1,
};
use crate::compiler_profile::manifest::Sha256Digest;
use crate::compiler_profile::qualification::{
    QUALIFIED_SIDECAR_REQUEST_VERSION_V1, QUALIFIED_SIDECAR_REQUEST_VERSION_V2,
    QUALIFIED_SIDECAR_RESPONSE_VERSION_V1,
};
use crate::generation_receipt::{
    open_regular_read_no_follow, publish_bytes_atomic_no_clobber_v1, read_bounded_file,
    ArtifactSealV1, GenerationReceiptError, ReceiptBackendSelectionV1,
};
use crate::standalone_package::ProductStandaloneCompilerTargetV1;
use crate::standalone_package_resolver::{
    ProductStandaloneCompilerPackageIdentityV1, ProductStandaloneCompilerReceiptAuthorityV1,
};

pub const GENERATION_RECEIPT_SCHEMA_V2: &str = "gore.as.generation-receipt";
pub const GENERATION_RECEIPT_VERSION_V2: u32 = 2;
pub const MAX_GENERATION_RECEIPT_JSON_BYTES_V2: usize = 8 * 1024 * 1024;
pub const MAX_GENERATION_OUTPUT_BYTES_V2: u64 = 512 * 1024 * 1024;

const MAX_IDENTITY_BYTES_V2: usize = 4 * 1024;
const RECEIPT_HASH_DOMAIN_V2: &[u8] = b"gore-as-generation-receipt-v2\0";
const RECEIPT_TEMP_PREFIX_V2: &str = ".gore-as-receipt-v2-tmp-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationArtifactKindV2 {
    FullGraph,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductQualifiedPackageIdentityV2 {
    pub catalog_sha256: Sha256Digest,
    pub sidecar: ArtifactSealV1,
    pub compatibility_id: String,
    pub request_version: u32,
    pub response_version: u32,
    pub profile_manifest: ArtifactSealV1,
    pub profile_sha256: Sha256Digest,
    pub target: ProductStandaloneCompilerTargetV1,
}

impl ProductQualifiedPackageIdentityV2 {
    fn from_product(identity: &ProductStandaloneCompilerPackageIdentityV1) -> Self {
        Self {
            catalog_sha256: identity.catalog_sha256(),
            sidecar: ArtifactSealV1 {
                byte_len: identity.sidecar_byte_len(),
                sha256: identity.sidecar_sha256(),
            },
            compatibility_id: identity.compatibility_id().to_owned(),
            request_version: identity.request_version(),
            response_version: identity.response_version(),
            profile_manifest: ArtifactSealV1 {
                byte_len: identity.manifest_byte_len(),
                sha256: identity.manifest_sha256(),
            },
            profile_sha256: identity.profile_sha256(),
            target: identity.target().clone(),
        }
    }

    fn validate(&self) -> Result<(), GenerationReceiptError> {
        if is_zero(self.catalog_sha256)
            || is_zero(self.sidecar.sha256)
            || is_zero(self.profile_manifest.sha256)
            || is_zero(self.profile_sha256)
            || self.sidecar.byte_len == 0
            || self.sidecar.byte_len > MAX_GENERATION_OUTPUT_BYTES_V2
            || self.profile_manifest.byte_len == 0
            || self.profile_manifest.byte_len > MAX_GENERATION_RECEIPT_JSON_BYTES_V2 as u64
        {
            return invalid(
                "qualified_package",
                "contains an empty, zero-digest, or oversized product identity",
            );
        }
        if !matches!(
            self.request_version,
            QUALIFIED_SIDECAR_REQUEST_VERSION_V1 | QUALIFIED_SIDECAR_REQUEST_VERSION_V2
        ) || self.response_version != QUALIFIED_SIDECAR_RESPONSE_VERSION_V1
        {
            return invalid(
                "qualified_package.protocol",
                "does not match the qualified standalone protocol",
            );
        }
        crate::standalone_package::validate_standalone_compiler_compatibility_id_v1(
            "qualified_package.compatibility_id",
            &self.compatibility_id,
        )
        .map_err(|error| GenerationReceiptError::Invalid {
            field: "qualified_package.compatibility_id",
            reason: error.to_string(),
        })?;
        ProductStandaloneCompilerTargetV1::try_new(
            self.target.target().clone(),
            self.target.pe_codeview().clone(),
        )
        .map_err(|error| GenerationReceiptError::Invalid {
            field: "qualified_package.target",
            reason: error.to_string(),
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FullGraphSourceDispositionReceiptV2 {
    Base,
    Added,
    Edited,
}

impl From<FullGraphSourceDispositionV1> for FullGraphSourceDispositionReceiptV2 {
    fn from(value: FullGraphSourceDispositionV1) -> Self {
        match value {
            FullGraphSourceDispositionV1::Base => Self::Base,
            FullGraphSourceDispositionV1::Added => Self::Added,
            FullGraphSourceDispositionV1::Edited => Self::Edited,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullGraphSourceManifestReceiptEntryV2 {
    pub module_name: String,
    pub relative_path: String,
    pub disposition: FullGraphSourceDispositionReceiptV2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ArtifactSealV1>,
}

impl From<&FullGraphSourceManifestEntryV1> for FullGraphSourceManifestReceiptEntryV2 {
    fn from(value: &FullGraphSourceManifestEntryV1) -> Self {
        Self {
            module_name: value.module_name.clone(),
            relative_path: value.relative_path.clone(),
            disposition: value.disposition.into(),
            source: value
                .source_byte_len
                .zip(value.source_sha256)
                .map(|(byte_len, sha256)| ArtifactSealV1 { byte_len, sha256 }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullGraphDeletedModuleReceiptV2 {
    pub module_name: String,
    pub relative_path: String,
}

impl From<&FullGraphDeletedModuleV1> for FullGraphDeletedModuleReceiptV2 {
    fn from(value: &FullGraphDeletedModuleV1) -> Self {
        Self {
            module_name: value.module_name.clone(),
            relative_path: value.relative_path.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullGraphGenerationInputsV2 {
    pub base_cache: ArtifactSealV1,
    pub binds_cache: ArtifactSealV1,
    pub final_manifest: Vec<FullGraphSourceManifestReceiptEntryV2>,
    pub deleted_modules: Vec<FullGraphDeletedModuleReceiptV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullGraphGenerationOutputV2 {
    pub artifact: ArtifactSealV1,
    pub cache_guid: String,
    pub build_identifier: u32,
    pub module_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationReceiptV2 {
    pub schema: String,
    pub schema_version: u32,
    pub artifact_kind: GenerationArtifactKindV2,
    pub qualified_package: ProductQualifiedPackageIdentityV2,
    pub inputs: FullGraphGenerationInputsV2,
    pub output: FullGraphGenerationOutputV2,
    pub backend: ReceiptBackendSelectionV1,
    pub canonical_sha256: Sha256Digest,
}

impl GenerationReceiptV2 {
    /// Seal a retained full-graph result against an authority minted only by the embedded catalog
    /// resolver. The target handles have already moved through the compiler transaction; callers
    /// must invoke this only after the report confirms exact restore and no recovery requirement.
    pub fn build_for_full_graph_artifact(
        authority: &ProductStandaloneCompilerReceiptAuthorityV1,
        base_cache: &[u8],
        binds_cache: &[u8],
        artifact: &FullGraphCompileArtifactV1,
        backend: ReceiptBackendSelectionV1,
    ) -> Result<Self, GenerationReceiptError> {
        artifact
            .validate_retained_artifact()
            .map_err(GenerationReceiptError::RetainedOutput)?;
        let output_bytes = read_retained_full_graph(artifact)?;
        let profile = authority.profile_package().profile();
        profile
            .validate_complete()
            .map_err(|error| GenerationReceiptError::Profile(error.to_string()))?;
        backend.validate()?;

        let base_seal = ArtifactSealV1::from_bytes(base_cache);
        let binds_seal = ArtifactSealV1::from_bytes(binds_cache);
        if !base_seal.matches_target_seal(authority.shipping_cache_seal())
            || !binds_seal.matches_target_seal(authority.binds_cache_seal())
        {
            return invalid(
                "inputs",
                "base/Binds bytes do not belong to the product-qualified profile",
            );
        }
        if artifact.base_cache_sha256() != base_seal.sha256 {
            return invalid(
                "inputs.base_cache",
                "does not match the base cache used by the full-graph compiler",
            );
        }

        let output_header = CacheHeader::parse(&output_bytes).map_err(|error| {
            GenerationReceiptError::CacheHeader {
                field: "output cache",
                reason: error.to_string(),
            }
        })?;
        if output_header.magic != profile.cache_writer.build_identifier {
            return invalid(
                "output.build_identifier",
                "does not match the product-qualified writer profile",
            );
        }

        let mut receipt = Self {
            schema: GENERATION_RECEIPT_SCHEMA_V2.into(),
            schema_version: GENERATION_RECEIPT_VERSION_V2,
            artifact_kind: GenerationArtifactKindV2::FullGraph,
            qualified_package: ProductQualifiedPackageIdentityV2::from_product(
                authority.identity(),
            ),
            inputs: FullGraphGenerationInputsV2 {
                base_cache: base_seal,
                binds_cache: binds_seal,
                final_manifest: artifact.final_manifest().iter().map(Into::into).collect(),
                deleted_modules: artifact.deleted_modules().iter().map(Into::into).collect(),
            },
            output: FullGraphGenerationOutputV2 {
                artifact: ArtifactSealV1::from_bytes(&output_bytes),
                cache_guid: hex_lower(&output_header.hash),
                build_identifier: output_header.magic,
                module_count: output_header.type_count,
            },
            backend,
            canonical_sha256: zero_digest(),
        };
        receipt.canonical_sha256 = receipt.computed_digest()?;
        receipt.validate_against(authority, base_cache, binds_cache, artifact)?;
        Ok(receipt)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, GenerationReceiptError> {
        if bytes.len() > MAX_GENERATION_RECEIPT_JSON_BYTES_V2 {
            return Err(GenerationReceiptError::InputTooLarge {
                field: "generation receipt V2 JSON",
                actual: bytes.len() as u64,
                max: MAX_GENERATION_RECEIPT_JSON_BYTES_V2 as u64,
            });
        }
        let receipt: Self = serde_json::from_slice(bytes)?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, GenerationReceiptError> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() > MAX_GENERATION_RECEIPT_JSON_BYTES_V2 {
            return Err(GenerationReceiptError::InputTooLarge {
                field: "generation receipt V2 JSON",
                actual: bytes.len() as u64,
                max: MAX_GENERATION_RECEIPT_JSON_BYTES_V2 as u64,
            });
        }
        Ok(bytes)
    }

    /// Strict schema/canonical integrity only. Product authority is established exclusively by
    /// [`Self::validate_against`], never by deserialising this receipt.
    pub fn validate(&self) -> Result<(), GenerationReceiptError> {
        if self.schema != GENERATION_RECEIPT_SCHEMA_V2
            || self.schema_version != GENERATION_RECEIPT_VERSION_V2
            || self.artifact_kind != GenerationArtifactKindV2::FullGraph
        {
            return invalid("schema", "expected gore.as.generation-receipt@2/full_graph");
        }
        self.qualified_package.validate()?;
        self.backend.validate()?;
        validate_artifact_seal(
            "inputs.base_cache",
            &self.inputs.base_cache,
            MAX_GENERATION_OUTPUT_BYTES_V2,
            false,
        )?;
        validate_artifact_seal(
            "inputs.binds_cache",
            &self.inputs.binds_cache,
            MAX_GENERATION_OUTPUT_BYTES_V2,
            false,
        )?;
        validate_artifact_seal(
            "output.artifact",
            &self.output.artifact,
            MAX_GENERATION_OUTPUT_BYTES_V2,
            false,
        )?;
        let guid = parse_guid_hex(&self.output.cache_guid)?;
        if guid == [0; 16] {
            return invalid("output.cache_guid", "must not be the zero GUID");
        }
        if self.output.module_count as usize != self.inputs.final_manifest.len() {
            return invalid(
                "output.module_count",
                "does not equal the complete final manifest length",
            );
        }
        validate_manifests(&self.inputs)?;
        if self.computed_digest()? != self.canonical_sha256 {
            return Err(GenerationReceiptError::DigestMismatch {
                expected: self.canonical_sha256,
                actual: self.computed_digest()?,
            });
        }
        Ok(())
    }

    pub fn validate_against(
        &self,
        authority: &ProductStandaloneCompilerReceiptAuthorityV1,
        base_cache: &[u8],
        binds_cache: &[u8],
        artifact: &FullGraphCompileArtifactV1,
    ) -> Result<(), GenerationReceiptError> {
        self.validate()?;
        let profile_package = authority.profile_package();
        let profile = profile_package.profile();
        profile
            .validate_complete()
            .map_err(|error| GenerationReceiptError::Profile(error.to_string()))?;
        if self.qualified_package
            != ProductQualifiedPackageIdentityV2::from_product(authority.identity())
            || self.qualified_package.profile_sha256 != profile.profile_sha256
            || authority.identity().profile_sha256() != profile.profile_sha256
        {
            return invalid(
                "qualified_package",
                "does not match the embedded-catalog receipt authority",
            );
        }
        let qualified_sidecar = profile_package.standalone_compiler_identity();
        if qualified_sidecar.request_version != self.qualified_package.request_version
            || qualified_sidecar.response_version != self.qualified_package.response_version
        {
            return invalid(
                "qualified_package.sidecar",
                "does not use the wire protocol exercised by differential qualification",
            );
        }
        if profile.target != *self.qualified_package.target.target()
            || profile.oracle.pe_codeview != *self.qualified_package.target.pe_codeview()
        {
            return invalid(
                "qualified_package.target",
                "does not match the qualified compiler profile",
            );
        }

        let base_seal = ArtifactSealV1::from_bytes(base_cache);
        let binds_seal = ArtifactSealV1::from_bytes(binds_cache);
        if self.inputs.base_cache != base_seal
            || self.inputs.binds_cache != binds_seal
            || !base_seal.matches_target_seal(authority.shipping_cache_seal())
            || !binds_seal.matches_target_seal(authority.binds_cache_seal())
            || artifact.base_cache_sha256() != base_seal.sha256
        {
            return invalid("inputs", "base/Binds/profile/compiler seals changed");
        }

        artifact
            .validate_retained_artifact()
            .map_err(GenerationReceiptError::RetainedOutput)?;
        let output_bytes = read_retained_full_graph(artifact)?;
        let output_seal = ArtifactSealV1::from_bytes(&output_bytes);
        if self.output.artifact != output_seal
            || artifact.byte_len() != output_seal.byte_len
            || artifact.sha256() != output_seal.sha256
        {
            return invalid(
                "output.artifact",
                "retained output identity or bytes changed",
            );
        }
        let header = CacheHeader::parse(&output_bytes).map_err(|error| {
            GenerationReceiptError::CacheHeader {
                field: "output cache",
                reason: error.to_string(),
            }
        })?;
        if self.output.cache_guid != hex_lower(&header.hash)
            || self.output.build_identifier != header.magic
            || header.magic != profile.cache_writer.build_identifier
            || self.output.module_count != header.type_count
            || artifact.module_count() != header.type_count
        {
            return invalid(
                "output",
                "cache header, artifact, receipt and profile disagree",
            );
        }

        let expected_manifest = artifact
            .final_manifest()
            .iter()
            .map(FullGraphSourceManifestReceiptEntryV2::from)
            .collect::<Vec<_>>();
        let expected_deleted = artifact
            .deleted_modules()
            .iter()
            .map(FullGraphDeletedModuleReceiptV2::from)
            .collect::<Vec<_>>();
        if self.inputs.final_manifest != expected_manifest
            || self.inputs.deleted_modules != expected_deleted
        {
            return invalid(
                "inputs.manifest",
                "does not match the compiler-validated final graph",
            );
        }
        validate_output_manifest(&output_bytes, &self.inputs.final_manifest)?;
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, GenerationReceiptError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        let bytes = serde_json::to_vec(&canonical)?;
        let mut digest = Sha256::new();
        digest.update(RECEIPT_HASH_DOMAIN_V2);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
        Ok(Sha256Digest::from_bytes(digest.finalize().into()))
    }
}

pub fn publish_generation_receipt_v2(
    destination: &Path,
    receipt: &GenerationReceiptV2,
) -> Result<PathBuf, GenerationReceiptError> {
    let bytes = receipt.to_json()?;
    publish_bytes_atomic_no_clobber_v1(
        destination,
        &bytes,
        MAX_GENERATION_RECEIPT_JSON_BYTES_V2 as u64,
        RECEIPT_TEMP_PREFIX_V2,
        "generation receipt V2",
    )
}

pub fn read_generation_receipt_v2(
    path: &Path,
) -> Result<GenerationReceiptV2, GenerationReceiptError> {
    let file = open_regular_read_no_follow(path).map_err(GenerationReceiptError::Publication)?;
    let bytes = read_bounded_file(
        file,
        MAX_GENERATION_RECEIPT_JSON_BYTES_V2 as u64,
        "generation receipt V2",
    )
    .map_err(GenerationReceiptError::Publication)?;
    GenerationReceiptV2::from_json(&bytes)
}

fn validate_manifests(inputs: &FullGraphGenerationInputsV2) -> Result<(), GenerationReceiptError> {
    if inputs.final_manifest.is_empty()
        || inputs.final_manifest.len() > MAX_FULL_GRAPH_FINAL_MODULES_V1
        || inputs.deleted_modules.len() > MAX_FULL_GRAPH_FINAL_MODULES_V1
    {
        return Err(GenerationReceiptError::CountTooLarge {
            field: "full-graph manifest",
            actual: inputs
                .final_manifest
                .len()
                .max(inputs.deleted_modules.len()),
            max: MAX_FULL_GRAPH_FINAL_MODULES_V1,
        });
    }
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut previous = None;
    let mut source_bytes = 0u64;
    let mut changed_modules = inputs.deleted_modules.len();
    for entry in &inputs.final_manifest {
        validate_identity(&entry.module_name, &entry.relative_path)?;
        let key = canonical_identity(&entry.module_name, &entry.relative_path);
        if previous.as_ref().is_some_and(|value| value >= &key) {
            return invalid(
                "inputs.final_manifest",
                "must be canonically sorted and unique",
            );
        }
        previous = Some(key);
        if !names.insert(fold(&entry.module_name)) || !paths.insert(fold(&entry.relative_path)) {
            return invalid(
                "inputs.final_manifest",
                "contains a Windows-case-colliding module or path",
            );
        }
        match (entry.disposition, &entry.source) {
            (FullGraphSourceDispositionReceiptV2::Base, None) => {}
            (
                FullGraphSourceDispositionReceiptV2::Added
                | FullGraphSourceDispositionReceiptV2::Edited,
                Some(source),
            ) => {
                changed_modules = changed_modules.saturating_add(1);
                validate_artifact_seal(
                    "inputs.final_manifest.source",
                    source,
                    MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64,
                    true,
                )?;
                source_bytes = source_bytes.checked_add(source.byte_len).ok_or_else(|| {
                    GenerationReceiptError::InputTooLarge {
                        field: "full-graph source bytes",
                        actual: u64::MAX,
                        max: MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64,
                    }
                })?;
            }
            _ => {
                return invalid(
                    "inputs.final_manifest.source",
                    "base entries omit source seals; Add/Edit entries require them",
                )
            }
        }
    }
    if source_bytes > MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64 {
        return Err(GenerationReceiptError::InputTooLarge {
            field: "full-graph source bytes",
            actual: source_bytes,
            max: MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64,
        });
    }
    if changed_modules > MAX_FULL_GRAPH_COMPILE_CHANGES_V1 {
        return Err(GenerationReceiptError::CountTooLarge {
            field: "full-graph changes",
            actual: changed_modules,
            max: MAX_FULL_GRAPH_COMPILE_CHANGES_V1,
        });
    }

    previous = None;
    let mut deleted_names = BTreeSet::new();
    let mut deleted_paths = BTreeSet::new();
    for deleted in &inputs.deleted_modules {
        validate_identity(&deleted.module_name, &deleted.relative_path)?;
        let key = canonical_identity(&deleted.module_name, &deleted.relative_path);
        if previous.as_ref().is_some_and(|value| value >= &key) {
            return invalid(
                "inputs.deleted_modules",
                "must be canonically sorted and unique",
            );
        }
        previous = Some(key);
        if !deleted_names.insert(fold(&deleted.module_name))
            || !deleted_paths.insert(fold(&deleted.relative_path))
        {
            return invalid(
                "inputs.deleted_modules",
                "contains a Windows-case-colliding module or path",
            );
        }
        if names.contains(&fold(&deleted.module_name))
            || paths.contains(&fold(&deleted.relative_path))
        {
            return invalid(
                "inputs.deleted_modules",
                "deleted identities must be absent from the final graph",
            );
        }
    }
    Ok(())
}

fn validate_output_manifest(
    bytes: &[u8],
    expected: &[FullGraphSourceManifestReceiptEntryV2],
) -> Result<(), GenerationReceiptError> {
    let modules = model::parse_modules(bytes).map_err(|error| GenerationReceiptError::Invalid {
        field: "output.modules",
        reason: error.to_string(),
    })?;
    let actual = emit_all::validated_module_identities(&modules).map_err(|error| {
        GenerationReceiptError::Invalid {
            field: "output.modules",
            reason: error,
        }
    })?;
    let actual = actual
        .iter()
        .map(|entry| {
            (
                fold(&entry.module_name),
                (entry.module_name.as_str(), entry.relative_path.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected = expected
        .iter()
        .map(|entry| {
            (
                fold(&entry.module_name),
                (entry.module_name.as_str(), entry.relative_path.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if actual != expected {
        return invalid(
            "output.modules",
            "does not exactly match the receipt final manifest",
        );
    }
    Ok(())
}

fn read_retained_full_graph(
    artifact: &FullGraphCompileArtifactV1,
) -> Result<Vec<u8>, GenerationReceiptError> {
    if artifact.byte_len() == 0 || artifact.byte_len() > MAX_GENERATION_OUTPUT_BYTES_V2 {
        return Err(GenerationReceiptError::InputTooLarge {
            field: "retained full-graph output",
            actual: artifact.byte_len(),
            max: MAX_GENERATION_OUTPUT_BYTES_V2,
        });
    }
    let bytes = artifact
        .read_retained_artifact_bytes(MAX_GENERATION_OUTPUT_BYTES_V2)
        .map_err(GenerationReceiptError::RetainedOutput)?;
    if bytes.len() as u64 != artifact.byte_len()
        || Sha256Digest::from_bytes(Sha256::digest(&bytes).into()) != artifact.sha256()
    {
        return Err(GenerationReceiptError::RetainedOutput(
            "retained full-graph output changed while being sealed".into(),
        ));
    }
    artifact
        .validate_retained_artifact()
        .map_err(GenerationReceiptError::RetainedOutput)?;
    Ok(bytes)
}

/// Read a published full-graph cache through its exact retained creation handle.
///
/// This is the only safe post-publication byte-read seam for external `gore-as` consumers: it
/// never reopens [`FullGraphCompileArtifactV1::path`], and it rechecks the retained length and
/// SHA-256 seal before returning the bounded bytes.
pub fn read_full_graph_compile_output_bytes_v2(
    artifact: &FullGraphCompileArtifactV1,
) -> Result<Vec<u8>, GenerationReceiptError> {
    read_retained_full_graph(artifact)
}

fn validate_artifact_seal(
    field: &'static str,
    seal: &ArtifactSealV1,
    max: u64,
    allow_empty: bool,
) -> Result<(), GenerationReceiptError> {
    if (!allow_empty && seal.byte_len == 0) || seal.byte_len > max || is_zero(seal.sha256) {
        return invalid(field, "contains an empty, oversized, or zero-digest seal");
    }
    Ok(())
}

fn validate_identity(module_name: &str, relative_path: &str) -> Result<(), GenerationReceiptError> {
    validate_text("module_name", module_name)?;
    validate_text("relative_path", relative_path)?;
    let path = Path::new(relative_path);
    if path.is_absolute()
        || relative_path.contains('\\')
        || relative_path.contains(':')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || relative_path.split('/').any(|component| {
            component.is_empty()
                || component.ends_with([' ', '.'])
                || component
                    .chars()
                    .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
                || windows_reserved_component(component)
        })
    {
        return invalid(
            "relative_path",
            "must be a safe slash-separated relative path",
        );
    }
    Ok(())
}

fn windows_reserved_component(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
}

fn validate_text(field: &'static str, value: &str) -> Result<(), GenerationReceiptError> {
    if value.is_empty()
        || value.len() > MAX_IDENTITY_BYTES_V2
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return invalid(field, "must be bounded nonempty UTF-8 without controls");
    }
    Ok(())
}

fn canonical_identity(module_name: &str, relative_path: &str) -> (String, String, String, String) {
    (
        fold(module_name),
        fold(relative_path),
        module_name.to_owned(),
        relative_path.to_owned(),
    )
}

fn fold(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

fn parse_guid_hex(value: &str) -> Result<[u8; 16], GenerationReceiptError> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(
            "output.cache_guid",
            "must be exactly 32 lowercase hexadecimal characters",
        );
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .expect("validated hexadecimal cache GUID");
    }
    Ok(bytes)
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

fn is_zero(value: Sha256Digest) -> bool {
    value == zero_digest()
}

fn invalid<T>(field: &'static str, reason: &'static str) -> Result<T, GenerationReceiptError> {
    Err(GenerationReceiptError::Invalid {
        field,
        reason: reason.into(),
    })
}

#[cfg(all(test, windows))]
mod tests {
    use sha2::{Digest as _, Sha256};

    use super::*;
    use crate::compile::{
        bind_full_graph_artifact_for_test, build_full_graph_cache_for_test, CompilerBackendModeV1,
        CompilerBackendNameV1, FullGraphSourceDispositionV1, FullGraphSourceManifestEntryV1,
    };
    use crate::standalone_package_resolver::test_support::SyntheticProductPackageFixtureV1;

    fn digest(bytes: &[u8]) -> Sha256Digest {
        Sha256Digest::from_bytes(Sha256::digest(bytes).into())
    }

    #[test]
    fn product_authority_retained_artifact_and_receipt_v2_are_one_closed_chain() {
        let package = SyntheticProductPackageFixtureV1::create();
        let (base_cache, binds_cache) = package.install_compatible_target_variants();
        let authority = package.receipt_authority();
        assert_ne!(
            ArtifactSealV1::from_bytes(&base_cache).sha256,
            authority
                .profile_package()
                .profile()
                .oracle
                .shipping_cache
                .sha256
        );
        assert_ne!(
            ArtifactSealV1::from_bytes(&binds_cache).sha256,
            authority
                .profile_package()
                .profile()
                .oracle
                .binds_cache
                .sha256
        );
        let authored_source = b"void Test() {}\n";
        let final_manifest = vec![FullGraphSourceManifestEntryV1 {
            module_name: "Test.Module".into(),
            relative_path: "Test/Module.as".into(),
            disposition: FullGraphSourceDispositionV1::Added,
            source_byte_len: Some(authored_source.len() as u64),
            source_sha256: Some(digest(authored_source)),
        }];
        let output_bytes = build_full_graph_cache_for_test(
            crate::cache::header::CACHE_MAGIC,
            [0x42; 16],
            &[("Test.Module", "Test/Module.as")],
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let output_path = temp.path().join("compiled.cache");
        std::fs::write(&output_path, output_bytes).unwrap();
        let artifact = bind_full_graph_artifact_for_test(
            output_path,
            digest(&base_cache),
            final_manifest,
            Vec::new(),
        )
        .unwrap();
        let backend = ReceiptBackendSelectionV1::from_compile_selection(
            CompilerBackendModeV1::Standalone,
            CompilerBackendNameV1::Standalone,
            None,
        )
        .unwrap();

        let receipt = GenerationReceiptV2::build_for_full_graph_artifact(
            &authority,
            &base_cache,
            &binds_cache,
            &artifact,
            backend,
        )
        .unwrap();
        receipt
            .validate_against(&authority, &base_cache, &binds_cache, &artifact)
            .unwrap();

        let receipt_path = temp.path().join("compiled.receipt.json");
        publish_generation_receipt_v2(&receipt_path, &receipt).unwrap();
        let loaded = read_generation_receipt_v2(&receipt_path).unwrap();
        assert_eq!(loaded, receipt);
        loaded
            .validate_against(&authority, &base_cache, &binds_cache, &artifact)
            .unwrap();
        assert!(publish_generation_receipt_v2(&receipt_path, &receipt).is_err());

        let mut protocol_v2 = receipt.clone();
        protocol_v2.qualified_package.request_version = QUALIFIED_SIDECAR_REQUEST_VERSION_V2;
        protocol_v2.canonical_sha256 = zero_digest();
        protocol_v2.canonical_sha256 = protocol_v2.computed_digest().unwrap();
        protocol_v2.validate().unwrap();

        let mut tampered = receipt.clone();
        tampered.output.module_count += 1;
        assert!(tampered.validate().is_err());
        assert!(GenerationReceiptV2::build_for_full_graph_artifact(
            &authority,
            &base_cache,
            b"wrong-binds",
            &artifact,
            ReceiptBackendSelectionV1::from_compile_selection(
                CompilerBackendModeV1::Standalone,
                CompilerBackendNameV1::Standalone,
                None,
            )
            .unwrap(),
        )
        .is_err());
    }
}
