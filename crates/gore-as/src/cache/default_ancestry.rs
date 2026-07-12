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
use super::default_patch::default_profile_cache_sha256;
use super::header::CacheHeader;

const VERIFIED_SCRIPT_CACHE_GUID: [u8; 16] = [
    0x45, 0x0d, 0x65, 0xc0, 0x4f, 0x0c, 0x01, 0x4f, 0xbe, 0xc5, 0x68, 0x01, 0x63, 0x78, 0xe6, 0x9a,
];
const VERIFIED_SCRIPT_CACHE_SEMANTIC_SHA256: [u8; 32] = [
    0xc1, 0xb3, 0x8e, 0x08, 0x3f, 0xde, 0xcc, 0x93, 0xd1, 0xc4, 0xa5, 0x39, 0x53, 0xe2, 0xfb, 0x90,
    0x16, 0x96, 0x3c, 0x04, 0x2c, 0x3d, 0xb8, 0x6d, 0xdf, 0xda, 0x6a, 0x40, 0x82, 0x30, 0x46, 0x8b,
];
/// Identity of the complete immutable evidence tuple: cache GUID and semantic fingerprint,
/// Binds bytes and canonical bridge, plus USMAP bytes, Class graph, and resolved join.
pub const DEFAULT_NATIVE_ANCESTRY_PROFILE_ID: &str =
    "sha256:3f53ee63723e6eb0c1ed7212c76d17976592dff30921c7fb2be729f2aef61cd1";
pub const DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID: &str =
    "sha256:c1b9f5e3e85e0637dc56c2228d1b38c7fdf9fc8d7aa96342cd56460c936d9b71";
const GAMEPLAY_TAG_FLOAT32_MAP_SEMANTIC_ID: &[u8] =
    b"usmap-class-declared-case-sensitive-array-dim-1:Map{key=Struct(GameplayTag),value=Float}";

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
    script_cache_semantic_sha256: [u8; 32],
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
        script_cache_semantic_sha256: [u8; 32],
        binds_source_sha256: [u8; 32],
        binds_bridge_sha256: [u8; 32],
        usmap_source_sha256: [u8; 32],
        usmap_graph_sha256: [u8; 32],
        resolved_profile_sha256: [u8; 32],
    ) -> Self {
        let mut evidence = Self {
            script_cache_guid,
            script_cache_semantic_sha256,
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
        hash.update(self.script_cache_guid);
        hash.update(self.script_cache_semantic_sha256);
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
        let script_cache_semantic_sha256 = default_profile_cache_sha256(cache)
            .map_err(|error| DefaultAncestryError::InvalidCache(error.to_string()))?;
        if script_cache_guid != VERIFIED_SCRIPT_CACHE_GUID
            || script_cache_semantic_sha256 != VERIFIED_SCRIPT_CACHE_SEMANTIC_SHA256
        {
            return Err(DefaultAncestryError::UnsupportedCache);
        }
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
            script_cache_semantic_sha256,
            binds_source_sha256,
            binds_bridge_sha256,
            source_sha256,
            usmap_graph_sha256,
            resolved_profile_sha256,
        );
        if expected_profile_sha256() != Some(evidence.profile_sha256) {
            return Err(DefaultAncestryError::ProfileIdDrift);
        }
        let gameplay_tag_float32_map_proof_sha256 = map_proof_sha256(
            &evidence.profile_sha256,
            &gameplay_tag_float32_map_profile_sha256,
        );
        if expected_id_sha256(DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID)
            != Some(gameplay_tag_float32_map_proof_sha256)
        {
            return Err(DefaultAncestryError::GameplayTagFloat32MapProofIdDrift);
        }

        let profile = Self {
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
            expected_profile_sha256(),
            Some(self.evidence.profile_sha256)
        );
        DEFAULT_NATIVE_ANCESTRY_PROFILE_ID
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
            expected_id_sha256(DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID),
            Some(self.gameplay_tag_float32_map_proof_sha256)
        );
        DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
    }

    pub(crate) fn supports_cache(
        &self,
        script_cache_guid: &[u8; 16],
        semantic_sha256: &[u8; 32],
    ) -> bool {
        script_cache_guid == &self.evidence.script_cache_guid
            && semantic_sha256 == &self.evidence.script_cache_semantic_sha256
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
            evidence: DefaultNativeAncestryEvidence {
                script_cache_guid: VERIFIED_SCRIPT_CACHE_GUID,
                script_cache_semantic_sha256: VERIFIED_SCRIPT_CACHE_SEMANTIC_SHA256,
                binds_source_sha256: [0; 32],
                binds_bridge_sha256: [0; 32],
                usmap_source_sha256: [0; 32],
                usmap_graph_sha256: [0; 32],
                resolved_profile_sha256: [0; 32],
                profile_sha256: expected_profile_sha256().expect("production profile id"),
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

fn expected_profile_sha256() -> Option<[u8; 32]> {
    expected_id_sha256(DEFAULT_NATIVE_ANCESTRY_PROFILE_ID)
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
        assert!(profile.supports_cache(
            &VERIFIED_SCRIPT_CACHE_GUID,
            &VERIFIED_SCRIPT_CACHE_SEMANTIC_SHA256
        ));
        let mut wrong_guid = VERIFIED_SCRIPT_CACHE_GUID;
        wrong_guid[0] ^= 1;
        assert!(!profile.supports_cache(&wrong_guid, &VERIFIED_SCRIPT_CACHE_SEMANTIC_SHA256));
        let mut wrong_semantic = VERIFIED_SCRIPT_CACHE_SEMANTIC_SHA256;
        wrong_semantic[0] ^= 1;
        assert!(!profile.supports_cache(&VERIFIED_SCRIPT_CACHE_GUID, &wrong_semantic));
        assert!(profile.proves_ancestry("UNativeLeaf", "UNativeRoot"));
        assert!(!profile.proves_ancestry("UNativeRoot", "UNativeLeaf"));
        assert_eq!(
            expected_id_sha256(DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID),
            Some(map_proof_sha256(
                &expected_profile_sha256().expect("production ancestry profile id"),
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
        let profile = DefaultNativeAncestry::from_schema_db(&binds, &cache, &schemas)
            .expect("derive configured profile");
        assert_eq!(profile.gameplay_tag_float32_maps.len(), 8);
        assert_eq!(
            profile.gameplay_tag_float32_map_profile_sha256(),
            VERIFIED_GAMEPLAY_TAG_FLOAT32_MAP_PROFILE_SHA256
        );
        assert_eq!(profile.profile_id(), DEFAULT_NATIVE_ANCESTRY_PROFILE_ID);
        assert_eq!(
            profile.gameplay_tag_float32_map_proof_id(),
            DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        );
    }
}
