//! SHA-sealed native class ancestry for offline default mutation.
//!
//! The script cache contains script-to-script edges and the first native base name, but not the
//! native chain above that base. This profile joins three independently parsed, exact-build
//! witnesses: the script-cache GUID, the matching `Binds.Cache` AngelScript-to-Unreal name map,
//! and the matching USMAP class graph. Any missing identity or parser-output digest disables the
//! profile; ordinary direct-owner default discovery continues without it.

use std::collections::{HashMap, HashSet};

use gore_asset::schema::ExactDeclaredPropertyShape;
use gore_asset::{SchemaDb, SchemaError, SchemaId, SchemaKind};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::binds::NativeApi;
use super::default_fingerprint::{
    combined_default_cache_fingerprint, DefaultCacheFingerprint, DEFAULT_CACHE_FINGERPRINT_FORMAT,
};
use super::header::CacheHeader;

const VERIFIED_SCRIPT_CACHE_GUID: [u8; 16] = [
    0x45, 0x0d, 0x65, 0xc0, 0x4f, 0x0c, 0x01, 0x4f, 0xbe, 0xc5, 0x68, 0x01, 0x63, 0x78, 0xe6, 0x9a,
];
/// Full-cache identity after normalizing both audited scalar-default immediates and every exact,
/// reference-proven GameplayTag-to-float32 map operand. The version, digest, and both exact range
/// counts are atomic ancestry-profile identity components.
const VERIFIED_SCRIPT_CACHE_MUTATION_STABLE_SHA256: [u8; 32] = [
    0x01, 0xfe, 0x4e, 0x37, 0xcc, 0x3a, 0x5d, 0xee, 0x15, 0xc2, 0xbe, 0xb4, 0x9a, 0x3f, 0x40, 0x61,
    0x10, 0x77, 0x4b, 0x5e, 0x30, 0x0f, 0x2d, 0xe4, 0xad, 0x81, 0x1d, 0x0d, 0xf9, 0xad, 0xdd, 0x6b,
];
const VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT: usize = 26_339;
const VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT: usize = 1_432;
/// Identity of the complete atomic production evidence tuple: versioned combined cache
/// fingerprint and exact range counts, cache GUID, Binds bytes/bridge, and USMAP bytes/graphs.
pub const DEFAULT_NATIVE_ANCESTRY_PROFILE_ID: &str =
    "sha256:98da5430f213b0107bd7361fa3c78316bf5320fbd15a53a9258d50d8d3ac9ed5";
pub const DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID: &str =
    "sha256:f20ce5ce571f3d121046ac1942e0705cfb30c3761a3e390cd5d77ea2c16159cc";
const HOTFIX_24169431_SCRIPT_CACHE_GUID: [u8; 16] = [
    0x43, 0x52, 0x1b, 0x38, 0x49, 0x7e, 0x98, 0x4f, 0x8a, 0xbb, 0xc0, 0x35, 0xeb, 0x4c, 0xb1, 0xd7,
];
const HOTFIX_24169431_SCRIPT_CACHE_MUTATION_STABLE_SHA256: [u8; 32] = [
    0x21, 0x21, 0x11, 0x87, 0xec, 0xa2, 0x88, 0x9f, 0x04, 0xe2, 0xba, 0xf9, 0x5d, 0xa2, 0x2d, 0x4e,
    0x71, 0x88, 0x28, 0x73, 0x41, 0xb7, 0x6e, 0xde, 0xd1, 0x18, 0x8a, 0x1d, 0x08, 0x54, 0x34, 0xc5,
];
/// Exact native-ancestry evidence identity for Steam BuildID 24169431. This ID is derived from
/// the hotfix cache GUID and combined mutation-stable fingerprint plus the sealed Binds/USMAP
/// parser outputs; it is not a version-name or filename match.
pub const HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID: &str =
    "sha256:b7e13f7f3756e97a07194bdbd6ba6a1f2cb99179888d0d8e581f505be969b645";
pub const HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID: &str =
    "sha256:b56365eff74dc11610c0e2f08dcb41923773bbb6efc954403c6ea09c48239b8a";
const GAMEPLAY_TAG_FLOAT32_MAP_SEMANTIC_ID: &[u8] =
    b"usmap-class-declared-case-sensitive-array-dim-1:Map{key=Struct(GameplayTag),value=Float}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SealedDefaultNativeAncestryProfile {
    script_cache_guid: [u8; 16],
    script_cache_mutation_stable_sha256: [u8; 32],
    scalar_default_operand_count: usize,
    gameplay_tag_float32_operand_count: usize,
    profile_id: &'static str,
    gameplay_tag_float32_map_proof_id: &'static str,
}

const SEALED_DEFAULT_NATIVE_ANCESTRY_PROFILES: [SealedDefaultNativeAncestryProfile; 2] = [
    SealedDefaultNativeAncestryProfile {
        script_cache_guid: VERIFIED_SCRIPT_CACHE_GUID,
        script_cache_mutation_stable_sha256: VERIFIED_SCRIPT_CACHE_MUTATION_STABLE_SHA256,
        scalar_default_operand_count: VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT,
        gameplay_tag_float32_operand_count: VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT,
        profile_id: DEFAULT_NATIVE_ANCESTRY_PROFILE_ID,
        gameplay_tag_float32_map_proof_id: DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID,
    },
    SealedDefaultNativeAncestryProfile {
        script_cache_guid: HOTFIX_24169431_SCRIPT_CACHE_GUID,
        script_cache_mutation_stable_sha256: HOTFIX_24169431_SCRIPT_CACHE_MUTATION_STABLE_SHA256,
        scalar_default_operand_count: VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT,
        gameplay_tag_float32_operand_count: VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT,
        profile_id: HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID,
        gameplay_tag_float32_map_proof_id: HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID,
    },
];

fn sealed_profile_for_cache(
    script_cache_guid: &[u8; 16],
    fingerprint: &DefaultCacheFingerprint,
) -> Option<&'static SealedDefaultNativeAncestryProfile> {
    SEALED_DEFAULT_NATIVE_ANCESTRY_PROFILES
        .iter()
        .find(|profile| {
            script_cache_guid == &profile.script_cache_guid
                && fingerprint.sha256 == profile.script_cache_mutation_stable_sha256
                && fingerprint.scalar_operand_count == profile.scalar_default_operand_count
                && fingerprint.tag_operand_count == profile.gameplay_tag_float32_operand_count
        })
}

/// Return whether the two selector identities are one exact supported ancestry/map-proof pair.
/// Independently recognized IDs from different game generations are deliberately rejected.
pub fn is_supported_gameplay_tag_float32_proof_pair(
    ancestry_profile_id: &str,
    map_proof_id: &str,
) -> bool {
    SEALED_DEFAULT_NATIVE_ANCESTRY_PROFILES
        .iter()
        .any(|profile| {
            profile.profile_id == ancestry_profile_id
                && profile.gameplay_tag_float32_map_proof_id == map_proof_id
        })
}

const VERIFIED_USMAP_SHA256: [u8; 32] = [
    0x73, 0x55, 0x8c, 0x36, 0x89, 0x5c, 0xd1, 0xb0, 0xf0, 0xfd, 0x1b, 0x3c, 0xb4, 0x43, 0x05, 0xb2,
    0x40, 0xf8, 0xdb, 0xb9, 0x37, 0x30, 0xad, 0x03, 0xc8, 0x8d, 0x7b, 0x84, 0x78, 0xb7, 0xff, 0xca,
];
/// Digest of all 6,594 parsed class rows as `(qualified class, qualified direct parent or empty)`.
const VERIFIED_USMAP_CLASS_GRAPH_SHA256: [u8; 32] = [
    0x0e, 0x64, 0x32, 0x22, 0x22, 0xd3, 0xd3, 0x2c, 0x5c, 0xd4, 0x12, 0x54, 0x53, 0x2d, 0x51, 0x8b,
    0xe5, 0xfe, 0xb7, 0x22, 0xa2, 0x4e, 0xd0, 0x14, 0x22, 0x84, 0xfa, 0x4e, 0xc9, 0x1d, 0x67, 0x9d,
];
/// Digest of the 6,572 exact Binds-name to USMAP-class resolutions, including each direct parent.
const VERIFIED_RESOLVED_CLASS_PROFILE_SHA256: [u8; 32] = [
    0x17, 0x63, 0x37, 0x9b, 0xcb, 0x89, 0x81, 0x6d, 0x07, 0x27, 0x24, 0x51, 0x54, 0x75, 0xb2, 0x86,
    0x13, 0x03, 0x1e, 0xff, 0x3b, 0x86, 0x62, 0x44, 0xf2, 0x33, 0x1a, 0xc5, 0x9c, 0x40, 0x64, 0xfa,
];
/// Digest of the eight exact `(AngelScript owner, canonical USMAP class path, declared field)`
/// rows whose sealed property shape is `Map{Struct GameplayTag, Float}`.
const VERIFIED_GAMEPLAY_TAG_FLOAT32_MAP_PROFILE_SHA256: [u8; 32] = [
    0x5f, 0xa2, 0xe3, 0x56, 0x16, 0xcb, 0x6b, 0x04, 0xa3, 0x06, 0x02, 0x02, 0xe5, 0x5f, 0xf5, 0x75,
    0xd8, 0xe8, 0xae, 0xab, 0x5a, 0x25, 0x60, 0x2a, 0xed, 0xdc, 0x10, 0xb3, 0xad, 0x54, 0x27, 0x08,
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct DefaultNativeAncestryEvidence {
    script_cache_guid: [u8; 16],
    script_cache_fingerprint_format: &'static str,
    script_cache_mutation_stable_sha256: [u8; 32],
    scalar_default_operand_count: usize,
    gameplay_tag_float32_operand_count: usize,
    binds_source_sha256: [u8; 32],
    binds_bridge_sha256: [u8; 32],
    usmap_source_sha256: [u8; 32],
    usmap_graph_sha256: [u8; 32],
    resolved_profile_sha256: [u8; 32],
    profile_sha256: [u8; 32],
}

impl DefaultNativeAncestryEvidence {
    fn new(
        script_cache_guid: [u8; 16],
        script_cache_fingerprint: DefaultCacheFingerprint,
        binds_source_sha256: [u8; 32],
        binds_bridge_sha256: [u8; 32],
        usmap_source_sha256: [u8; 32],
        usmap_graph_sha256: [u8; 32],
        resolved_profile_sha256: [u8; 32],
    ) -> Self {
        let mut evidence = Self {
            script_cache_guid,
            script_cache_fingerprint_format: DEFAULT_CACHE_FINGERPRINT_FORMAT,
            script_cache_mutation_stable_sha256: script_cache_fingerprint.sha256,
            scalar_default_operand_count: script_cache_fingerprint.scalar_operand_count,
            gameplay_tag_float32_operand_count: script_cache_fingerprint.tag_operand_count,
            binds_source_sha256,
            binds_bridge_sha256,
            usmap_source_sha256,
            usmap_graph_sha256,
            resolved_profile_sha256,
            profile_sha256: [0; 32],
        };
        evidence.profile_sha256 = evidence.derived_profile_sha256();
        evidence
    }

    fn derived_profile_sha256(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update((self.script_cache_fingerprint_format.len() as u32).to_le_bytes());
        hash.update(self.script_cache_fingerprint_format.as_bytes());
        hash.update(self.script_cache_guid);
        hash.update(self.script_cache_mutation_stable_sha256);
        hash.update((self.scalar_default_operand_count as u64).to_le_bytes());
        hash.update((self.gameplay_tag_float32_operand_count as u64).to_le_bytes());
        hash.update(self.binds_source_sha256);
        hash.update(self.binds_bridge_sha256);
        hash.update(self.usmap_source_sha256);
        hash.update(self.usmap_graph_sha256);
        hash.update(self.resolved_profile_sha256);
        hash.finalize().into()
    }
}

#[derive(Debug, Error)]
pub enum DefaultAncestryError {
    #[error("script cache is invalid: {0}")]
    InvalidCache(String),
    #[error("script cache identity is not the sealed default-ancestry build")]
    UnsupportedCache,
    #[error("Binds.Cache has no sealed class-name profile for this script-cache GUID")]
    UnsupportedBinds,
    #[error("USMAP has no raw source identity")]
    MissingUsmapIdentity,
    #[error("USMAP identity is not the sealed default-ancestry build")]
    UnsupportedUsmap,
    #[error("USMAP class graph does not match its sealed parser-output profile")]
    UsmapGraphDrift,
    #[error(
        "Binds class {script_class} resolves ambiguously or invalidly through {path}: {error}"
    )]
    BridgeResolution {
        script_class: String,
        path: String,
        error: String,
    },
    #[error("two Binds class names resolve to USMAP schema {schema}")]
    DuplicateSchemaBridge { schema: String },
    #[error("resolved Binds/USMAP class profile does not match its sealed parser-output profile")]
    ResolvedProfileDrift,
    #[error("derived native-default evidence ID does not match its sealed production ID")]
    ProfileIdDrift,
    #[error("GameplayTag-to-float32 map field profile does not match its sealed parser output")]
    GameplayTagFloat32MapProfileDrift,
    #[error("derived GameplayTag-to-float32 map proof ID does not match its sealed production ID")]
    GameplayTagFloat32MapProofIdDrift,
    #[error("USMAP class hierarchy contains a cycle reachable from {class}")]
    CyclicHierarchy { class: String },
}

/// Opaque, exact-build proof for native class ancestry. Callers cannot construct arbitrary edges.
#[derive(Debug, Clone)]
pub struct DefaultNativeAncestry {
    sealed_profile: &'static SealedDefaultNativeAncestryProfile,
    evidence: DefaultNativeAncestryEvidence,
    class_ids: HashMap<String, SchemaId>,
    super_ids: Vec<Option<SchemaId>>,
    gameplay_tag_float32_maps: HashSet<(String, String)>,
    gameplay_tag_float32_map_profile_sha256: [u8; 32],
    gameplay_tag_float32_map_proof_sha256: [u8; 32],
}

impl DefaultNativeAncestry {
    /// Join a sealed Binds class-name map with a sealed, fully validated USMAP class graph.
    pub fn from_schema_db(
        native: &NativeApi,
        cache: &[u8],
        schemas: &SchemaDb,
    ) -> Result<Self, DefaultAncestryError> {
        let script_cache_guid = CacheHeader::parse(cache)
            .map_err(|error| DefaultAncestryError::InvalidCache(error.to_string()))?
            .hash;
        let script_cache_fingerprint = combined_default_cache_fingerprint(cache)
            .map_err(|error| DefaultAncestryError::InvalidCache(error.to_string()))?;
        let sealed_profile =
            sealed_profile_for_cache(&script_cache_guid, &script_cache_fingerprint)
                .ok_or(DefaultAncestryError::UnsupportedCache)?;
        let class_paths = native
            .verified_default_class_paths(&script_cache_guid)
            .ok_or(DefaultAncestryError::UnsupportedBinds)?;
        let (binds_source_sha256, binds_bridge_sha256) = native
            .verified_default_class_profile_digests(&script_cache_guid)
            .ok_or(DefaultAncestryError::UnsupportedBinds)?;
        let source_sha256 = schemas
            .source_sha256()
            .ok_or(DefaultAncestryError::MissingUsmapIdentity)?;
        if source_sha256 != VERIFIED_USMAP_SHA256 {
            return Err(DefaultAncestryError::UnsupportedUsmap);
        }

        let mut super_ids = vec![None; schemas.len()];
        let mut graph_rows = Vec::new();
        for record in schemas
            .schemas()
            .iter()
            .filter(|record| record.kind == SchemaKind::Class)
        {
            let parent = schemas
                .exact_class_super_schema_id(record.id)
                .map_err(|error| DefaultAncestryError::BridgeResolution {
                    script_class: "<USMAP graph>".into(),
                    path: record.qualified_name(),
                    error: error.to_string(),
                })?;
            super_ids[record.id] = parent;
            let parent_name = parent
                .map(|id| {
                    schemas
                        .schema(id)
                        .expect("resolved schema id")
                        .qualified_name()
                })
                .unwrap_or_default();
            graph_rows.push([record.qualified_name(), parent_name]);
        }
        let usmap_graph_sha256 = rows_sha256(&mut graph_rows);
        if usmap_graph_sha256 != VERIFIED_USMAP_CLASS_GRAPH_SHA256 {
            return Err(DefaultAncestryError::UsmapGraphDrift);
        }

        let mut class_ids = HashMap::new();
        let mut claimed_ids = HashSet::new();
        let mut resolved_rows = Vec::new();
        let mut gameplay_tag_float32_maps = HashSet::new();
        let mut gameplay_tag_float32_map_rows = Vec::new();
        for (script_class, path) in class_paths {
            let id = match schemas.resolve_class(path) {
                Ok(id) => id,
                // Binds also contains structs/enums and types absent from this USMAP. They are
                // not class ancestry evidence, and their exact omissions are sealed by the final
                // resolved-profile digest below.
                Err(SchemaError::SchemaNotFound { .. } | SchemaError::NotAClass(_)) => continue,
                Err(error) => {
                    return Err(DefaultAncestryError::BridgeResolution {
                        script_class: script_class.clone(),
                        path: path.clone(),
                        error: error.to_string(),
                    });
                }
            };
            let canonical_path = schemas
                .schema(id)
                .expect("resolved schema id")
                .qualified_name();
            if canonical_path != *path {
                return Err(DefaultAncestryError::BridgeResolution {
                    script_class: script_class.clone(),
                    path: path.clone(),
                    error: format!("non-canonical case; exact schema path is {canonical_path}"),
                });
            }
            let record = schemas.schema(id).expect("resolved schema id");
            for property in &record.properties {
                let shape = schemas
                    .exact_declared_property_shape(id, &property.name)
                    .map_err(|error| DefaultAncestryError::BridgeResolution {
                        script_class: script_class.clone(),
                        path: format!("{canonical_path}.{}", property.name),
                        error: error.to_string(),
                    })?;
                if shape == Some(ExactDeclaredPropertyShape::GameplayTagFloat32Map) {
                    gameplay_tag_float32_maps.insert((script_class.clone(), property.name.clone()));
                    gameplay_tag_float32_map_rows.push([
                        script_class.clone(),
                        canonical_path.clone(),
                        property.name.clone(),
                    ]);
                }
            }
            if !claimed_ids.insert(id) {
                return Err(DefaultAncestryError::DuplicateSchemaBridge {
                    schema: schemas
                        .schema(id)
                        .expect("resolved schema id")
                        .qualified_name(),
                });
            }
            class_ids.insert(script_class.clone(), id);
            let qualified = schemas
                .schema(id)
                .expect("resolved schema id")
                .qualified_name();
            let parent = super_ids[id]
                .map(|parent| {
                    schemas
                        .schema(parent)
                        .expect("resolved parent id")
                        .qualified_name()
                })
                .unwrap_or_default();
            resolved_rows.push([script_class.clone(), qualified, parent]);
        }
        let resolved_profile_sha256 = rows_sha256(&mut resolved_rows);
        if resolved_profile_sha256 != VERIFIED_RESOLVED_CLASS_PROFILE_SHA256 {
            return Err(DefaultAncestryError::ResolvedProfileDrift);
        }
        let gameplay_tag_float32_map_profile_sha256 =
            rows_sha256(&mut gameplay_tag_float32_map_rows);
        if gameplay_tag_float32_map_profile_sha256
            != VERIFIED_GAMEPLAY_TAG_FLOAT32_MAP_PROFILE_SHA256
        {
            return Err(DefaultAncestryError::GameplayTagFloat32MapProfileDrift);
        }

        let evidence = DefaultNativeAncestryEvidence::new(
            script_cache_guid,
            script_cache_fingerprint,
            binds_source_sha256,
            binds_bridge_sha256,
            source_sha256,
            usmap_graph_sha256,
            resolved_profile_sha256,
        );
        if expected_id_sha256(sealed_profile.profile_id) != Some(evidence.profile_sha256) {
            return Err(DefaultAncestryError::ProfileIdDrift);
        }
        let gameplay_tag_float32_map_proof_sha256 = map_proof_sha256(
            &evidence.profile_sha256,
            &gameplay_tag_float32_map_profile_sha256,
        );
        if expected_id_sha256(sealed_profile.gameplay_tag_float32_map_proof_id)
            != Some(gameplay_tag_float32_map_proof_sha256)
        {
            return Err(DefaultAncestryError::GameplayTagFloat32MapProofIdDrift);
        }

        let profile = Self {
            sealed_profile,
            evidence,
            class_ids,
            super_ids,
            gameplay_tag_float32_maps,
            gameplay_tag_float32_map_profile_sha256,
            gameplay_tag_float32_map_proof_sha256,
        };
        profile.validate_acyclic()?;
        Ok(profile)
    }

    pub fn class_count(&self) -> usize {
        self.class_ids.len()
    }

    pub fn profile_id(&self) -> &'static str {
        debug_assert_eq!(
            expected_id_sha256(self.sealed_profile.profile_id),
            Some(self.evidence.profile_sha256)
        );
        self.sealed_profile.profile_id
    }

    pub fn gameplay_tag_float32_map_profile_sha256(&self) -> [u8; 32] {
        self.gameplay_tag_float32_map_profile_sha256
    }

    pub fn gameplay_tag_float32_map_proof_id(&self) -> &'static str {
        debug_assert_eq!(
            self.gameplay_tag_float32_map_proof_sha256,
            map_proof_sha256(
                &self.evidence.profile_sha256,
                &self.gameplay_tag_float32_map_profile_sha256
            )
        );
        debug_assert_eq!(
            expected_id_sha256(self.sealed_profile.gameplay_tag_float32_map_proof_id),
            Some(self.gameplay_tag_float32_map_proof_sha256)
        );
        self.sealed_profile.gameplay_tag_float32_map_proof_id
    }

    pub(crate) fn supports_cache(
        &self,
        script_cache_guid: &[u8; 16],
        fingerprint: &DefaultCacheFingerprint,
    ) -> bool {
        script_cache_guid == &self.evidence.script_cache_guid
            && self.evidence.script_cache_fingerprint_format == DEFAULT_CACHE_FINGERPRINT_FORMAT
            && fingerprint.sha256 == self.evidence.script_cache_mutation_stable_sha256
            && fingerprint.scalar_operand_count == self.evidence.scalar_default_operand_count
            && fingerprint.tag_operand_count == self.evidence.gameplay_tag_float32_operand_count
    }

    pub(crate) fn proves_ancestry(&self, descendant: &str, ancestor: &str) -> bool {
        let (Some(&start), Some(&wanted)) =
            (self.class_ids.get(descendant), self.class_ids.get(ancestor))
        else {
            return false;
        };
        let mut seen = HashSet::new();
        let mut current = Some(start);
        while let Some(id) = current {
            if id == wanted {
                return true;
            }
            if !seen.insert(id) {
                return false;
            }
            current = self.super_ids.get(id).copied().flatten();
        }
        false
    }

    /// Return the sealed evidence identity only for an exact Binds-bridged owner and an exact,
    /// case-sensitive USMAP declaration of `TMap<FGameplayTag,float32>`.
    pub(crate) fn proves_gameplay_tag_float32_map(
        &self,
        owner: &str,
        field: &str,
    ) -> Option<&'static str> {
        self.gameplay_tag_float32_maps
            .contains(&(owner.to_owned(), field.to_owned()))
            .then_some(self.gameplay_tag_float32_map_proof_id())
    }

    fn validate_acyclic(&self) -> Result<(), DefaultAncestryError> {
        for (class, &start) in &self.class_ids {
            let mut seen = HashSet::new();
            let mut current = Some(start);
            while let Some(id) = current {
                if !seen.insert(id) {
                    return Err(DefaultAncestryError::CyclicHierarchy {
                        class: class.clone(),
                    });
                }
                current = self.super_ids.get(id).copied().flatten();
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn from_test_edges(edges: &[(&str, Option<&str>)]) -> Self {
        Self::from_test_edges_and_maps(edges, &[])
    }

    #[cfg(test)]
    pub(crate) fn from_test_edges_and_maps(
        edges: &[(&str, Option<&str>)],
        maps: &[(&str, &str)],
    ) -> Self {
        let class_ids: HashMap<_, _> = edges
            .iter()
            .enumerate()
            .map(|(id, (class, _))| ((*class).to_owned(), id))
            .collect();
        let super_ids = edges
            .iter()
            .map(|(_, parent)| parent.and_then(|parent| class_ids.get(parent).copied()))
            .collect();
        Self {
            sealed_profile: &SEALED_DEFAULT_NATIVE_ANCESTRY_PROFILES[0],
            evidence: DefaultNativeAncestryEvidence {
                script_cache_guid: VERIFIED_SCRIPT_CACHE_GUID,
                script_cache_fingerprint_format: DEFAULT_CACHE_FINGERPRINT_FORMAT,
                script_cache_mutation_stable_sha256: VERIFIED_SCRIPT_CACHE_MUTATION_STABLE_SHA256,
                scalar_default_operand_count: VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT,
                gameplay_tag_float32_operand_count: VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT,
                binds_source_sha256: [0; 32],
                binds_bridge_sha256: [0; 32],
                usmap_source_sha256: [0; 32],
                usmap_graph_sha256: [0; 32],
                resolved_profile_sha256: [0; 32],
                profile_sha256: expected_id_sha256(DEFAULT_NATIVE_ANCESTRY_PROFILE_ID)
                    .expect("production profile id"),
            },
            class_ids,
            super_ids,
            gameplay_tag_float32_maps: maps
                .iter()
                .map(|(owner, field)| ((*owner).to_owned(), (*field).to_owned()))
                .collect(),
            gameplay_tag_float32_map_profile_sha256:
                VERIFIED_GAMEPLAY_TAG_FLOAT32_MAP_PROFILE_SHA256,
            gameplay_tag_float32_map_proof_sha256: expected_id_sha256(
                DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID,
            )
            .expect("production map proof id"),
        }
    }
}

fn expected_id_sha256(id: &str) -> Option<[u8; 32]> {
    let hex = id.strip_prefix("sha256:")?;
    if hex.len() != 64 {
        return None;
    }
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&hex[offset..offset + 2], 16).ok()?;
    }
    Some(output)
}

fn map_proof_sha256(ancestry_profile: &[u8; 32], field_profile: &[u8; 32]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(ancestry_profile);
    hash.update(field_profile);
    hash.update(GAMEPLAY_TAG_FLOAT32_MAP_SEMANTIC_ID);
    hash.finalize().into()
}

fn rows_sha256<const N: usize>(rows: &mut Vec<[String; N]>) -> [u8; 32] {
    rows.sort_unstable();
    let mut hash = Sha256::new();
    for row in rows {
        for value in row {
            hash.update((value.len() as u32).to_le_bytes());
            hash.update(value.as_bytes());
        }
    }
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_tuple_and_native_chain_are_exact() {
        let profile = DefaultNativeAncestry::from_test_edges(&[
            ("UNativeLeaf", Some("UNativeBase")),
            ("UNativeBase", Some("UNativeRoot")),
            ("UNativeRoot", None),
        ]);
        let fingerprint = DefaultCacheFingerprint {
            sha256: VERIFIED_SCRIPT_CACHE_MUTATION_STABLE_SHA256,
            scalar_operand_count: VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT,
            tag_operand_count: VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT,
        };
        assert!(profile.supports_cache(&VERIFIED_SCRIPT_CACHE_GUID, &fingerprint));
        let mut wrong_guid = VERIFIED_SCRIPT_CACHE_GUID;
        wrong_guid[0] ^= 1;
        assert!(!profile.supports_cache(&wrong_guid, &fingerprint));
        let mut wrong_fingerprint = fingerprint;
        wrong_fingerprint.sha256[0] ^= 1;
        assert!(!profile.supports_cache(&VERIFIED_SCRIPT_CACHE_GUID, &wrong_fingerprint));
        let mut wrong_count = fingerprint;
        wrong_count.tag_operand_count -= 1;
        assert!(!profile.supports_cache(&VERIFIED_SCRIPT_CACHE_GUID, &wrong_count));
        assert!(profile.proves_ancestry("UNativeLeaf", "UNativeRoot"));
        assert!(!profile.proves_ancestry("UNativeRoot", "UNativeLeaf"));
        assert_eq!(
            expected_id_sha256(DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID),
            Some(map_proof_sha256(
                &expected_id_sha256(DEFAULT_NATIVE_ANCESTRY_PROFILE_ID)
                    .expect("production ancestry profile id"),
                &VERIFIED_GAMEPLAY_TAG_FLOAT32_MAP_PROFILE_SHA256
            ))
        );

        let map_profile = DefaultNativeAncestry::from_test_edges_and_maps(
            &[("UNativeLeaf", None)],
            &[("UNativeLeaf", "Damage")],
        );
        assert_eq!(
            map_profile.proves_gameplay_tag_float32_map("UNativeLeaf", "Damage"),
            Some(DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID)
        );
        assert_eq!(
            map_profile.proves_gameplay_tag_float32_map("UNativeLeaf", "damage"),
            None
        );
    }

    #[test]
    fn sealed_generation_tuples_and_selector_proof_pairs_do_not_cross() {
        let retained = DefaultCacheFingerprint {
            sha256: VERIFIED_SCRIPT_CACHE_MUTATION_STABLE_SHA256,
            scalar_operand_count: VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT,
            tag_operand_count: VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT,
        };
        let hotfix = DefaultCacheFingerprint {
            sha256: HOTFIX_24169431_SCRIPT_CACHE_MUTATION_STABLE_SHA256,
            scalar_operand_count: VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT,
            tag_operand_count: VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT,
        };

        assert_eq!(
            sealed_profile_for_cache(&VERIFIED_SCRIPT_CACHE_GUID, &retained)
                .map(|profile| profile.profile_id),
            Some(DEFAULT_NATIVE_ANCESTRY_PROFILE_ID)
        );
        assert_eq!(
            sealed_profile_for_cache(&HOTFIX_24169431_SCRIPT_CACHE_GUID, &hotfix)
                .map(|profile| profile.profile_id),
            Some(HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID)
        );
        assert!(sealed_profile_for_cache(&VERIFIED_SCRIPT_CACHE_GUID, &hotfix).is_none());
        assert!(sealed_profile_for_cache(&HOTFIX_24169431_SCRIPT_CACHE_GUID, &retained).is_none());

        let mut unknown_guid = HOTFIX_24169431_SCRIPT_CACHE_GUID;
        unknown_guid[0] ^= 1;
        assert!(sealed_profile_for_cache(&unknown_guid, &hotfix).is_none());
        let mut unknown_fingerprint = hotfix;
        unknown_fingerprint.sha256[0] ^= 1;
        assert!(
            sealed_profile_for_cache(&HOTFIX_24169431_SCRIPT_CACHE_GUID, &unknown_fingerprint)
                .is_none()
        );

        assert!(is_supported_gameplay_tag_float32_proof_pair(
            DEFAULT_NATIVE_ANCESTRY_PROFILE_ID,
            DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        ));
        assert!(is_supported_gameplay_tag_float32_proof_pair(
            HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID,
            HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        ));
        assert!(!is_supported_gameplay_tag_float32_proof_pair(
            DEFAULT_NATIVE_ANCESTRY_PROFILE_ID,
            HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        ));
        assert!(!is_supported_gameplay_tag_float32_proof_pair(
            HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID,
            DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        ));
        assert!(!is_supported_gameplay_tag_float32_proof_pair(
            "sha256:unknown",
            "sha256:unknown"
        ));
    }

    #[test]
    fn cyclic_native_graph_fails_closed() {
        let profile = DefaultNativeAncestry::from_test_edges(&[
            ("UNativeA", Some("UNativeB")),
            ("UNativeB", Some("UNativeA")),
        ]);
        assert!(matches!(
            profile.validate_acyclic(),
            Err(DefaultAncestryError::CyclicHierarchy { .. })
        ));
    }

    #[test]
    fn configured_production_profile_derives_its_evidence_ids() {
        let Some(cache_path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
            eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
            return;
        };
        let Some(usmap_path) = std::env::var_os("GORE_AS_DEFAULT_USMAP") else {
            eprintln!("skip: set GORE_AS_DEFAULT_USMAP");
            return;
        };
        let cache_path = std::path::PathBuf::from(cache_path);
        let cache = std::fs::read(&cache_path).expect("read configured cache");
        let binds = NativeApi::load(&cache_path.parent().unwrap().join("Binds.Cache"))
            .expect("load sibling Binds");
        let usmap = std::fs::read(usmap_path).expect("read configured USMAP");
        let schemas = SchemaDb::from_usmap(&usmap).expect("parse configured USMAP");
        let fingerprint = combined_default_cache_fingerprint(&cache)
            .expect("fingerprint configured production cache");
        let cache_guid = CacheHeader::parse(&cache)
            .expect("parse configured cache")
            .hash;
        let expected =
            sealed_profile_for_cache(&cache_guid, &fingerprint).expect("configured sealed tuple");
        let profile = DefaultNativeAncestry::from_schema_db(&binds, &cache, &schemas)
            .expect("derive configured profile");
        assert_eq!(profile.gameplay_tag_float32_maps.len(), 8);
        assert_eq!(
            profile.gameplay_tag_float32_map_profile_sha256(),
            VERIFIED_GAMEPLAY_TAG_FLOAT32_MAP_PROFILE_SHA256
        );
        assert_eq!(profile.profile_id(), expected.profile_id);
        assert_eq!(
            profile.gameplay_tag_float32_map_proof_id(),
            expected.gameplay_tag_float32_map_proof_id
        );
        assert_eq!(
            fingerprint,
            DefaultCacheFingerprint {
                sha256: expected.script_cache_mutation_stable_sha256,
                scalar_operand_count: expected.scalar_default_operand_count,
                tag_operand_count: expected.gameplay_tag_float32_operand_count,
            }
        );
        let references = super::super::default_tag_map::reference_proven_tag_map_sites(&cache)
            .expect("discover configured tag-map references");
        let changed_site = &references.sites[0];
        let mut replacement = changed_site.raw.expected;
        replacement[0] ^= 1;
        let mut mutated = cache.clone();
        mutated[changed_site.operand_range.clone()].copy_from_slice(&replacement);
        assert_ne!(
            super::super::default_patch::default_profile_cache_sha256(&mutated)
                .expect("legacy scalar-only fingerprint after tag edit"),
            super::super::default_patch::default_profile_cache_sha256(&cache)
                .expect("legacy scalar-only fingerprint before tag edit")
        );
        assert_eq!(
            combined_default_cache_fingerprint(&mutated)
                .expect("combined fingerprint after tag edit"),
            fingerprint
        );

        let reconstructed = DefaultNativeAncestry::from_schema_db(&binds, &mutated, &schemas)
            .expect("reconstruct ancestry after tag edit");
        assert_eq!(reconstructed.profile_id(), expected.profile_id);
        let tag_report =
            super::super::native_tag_map::inspect_native_tag_maps(&mutated, &reconstructed)
                .expect("rediscover native tag-map sites after tag edit");
        assert_eq!(
            tag_report.site_count(),
            VERIFIED_GAMEPLAY_TAG_FLOAT32_OPERAND_COUNT
        );
        assert_eq!(
            tag_report.fingerprint_format(),
            DEFAULT_CACHE_FINGERPRINT_FORMAT
        );
        assert_eq!(tag_report.fingerprint_sha256(), fingerprint.sha256);
        assert_eq!(
            tag_report.scalar_operand_count(),
            VERIFIED_SCALAR_DEFAULT_OPERAND_COUNT
        );
        assert_eq!(
            tag_report.tag_operand_count(),
            fingerprint.tag_operand_count
        );
        assert_eq!(tag_report.ancestry_profile_id(), expected.profile_id);
        assert_eq!(
            tag_report.map_proof_id(),
            expected.gameplay_tag_float32_map_proof_id
        );
        let rediscovered = tag_report
            .sites()
            .iter()
            .find(|site| site.operand_range() == changed_site.operand_range)
            .expect("edited tag-map operand remains uniquely rediscoverable");
        assert_eq!(rediscovered.expected(), replacement);

        let scalar_native = NativeApi::load(&cache_path.parent().unwrap().join("Binds.Cache"))
            .expect("reload sibling Binds for scalar discovery");
        let scalar_report = super::super::default_patch::default_sites_with_native_ancestry(
            &mutated,
            Some(scalar_native),
            Some(reconstructed),
        )
        .expect("retain scalar ancestry membership after tag edit");
        assert!(scalar_report.sites.iter().any(|site| {
            site.selector.ancestry_profile.as_deref() == Some(expected.profile_id)
        }));
    }
}
