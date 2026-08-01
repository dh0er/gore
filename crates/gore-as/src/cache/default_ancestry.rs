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
use gore_generation::{expected_id_sha256, map_proof_sha256, GenerationRow, ProfileComponents};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::binds::NativeApi;
use super::default_fingerprint::{
    combined_default_cache_fingerprint, DefaultCacheFingerprint, DEFAULT_CACHE_FINGERPRINT_FORMAT,
};
use super::header::CacheHeader;

/// Aliases for the first two audited generations, kept because they are named directly by the CLI,
/// by `default_patch`'s tests and by two integration suites. A third generation deliberately gets
/// no alias: new call sites read `gore_generation::rows()` instead of adding a pair of constants
/// per build, which is the whole point of the table.
pub const DEFAULT_NATIVE_ANCESTRY_PROFILE_ID: &str =
    gore_generation::ROW_G1R_1_0_3.native_ancestry_profile_id;
pub const DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID: &str =
    gore_generation::ROW_G1R_1_0_3.gameplay_tag_float32_map_proof_id;
pub const HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID: &str =
    gore_generation::ROW_G1R_24169431.native_ancestry_profile_id;
pub const HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID: &str =
    gore_generation::ROW_G1R_24169431.gameplay_tag_float32_map_proof_id;

fn cache_fingerprint(fingerprint: &DefaultCacheFingerprint) -> gore_generation::CacheFingerprint {
    gore_generation::CacheFingerprint {
        sha256: fingerprint.sha256,
        scalar_operand_count: fingerprint.scalar_operand_count,
        tag_operand_count: fingerprint.tag_operand_count,
    }
}

fn sealed_profile_for_cache(
    script_cache_guid: &[u8; 16],
    fingerprint: &DefaultCacheFingerprint,
) -> Option<&'static GenerationRow> {
    gore_generation::row_for_script_cache(script_cache_guid, &cache_fingerprint(fingerprint))
}

/// Return whether the two selector identities are one exact supported ancestry/map-proof pair.
/// Independently recognized IDs from different game generations are deliberately rejected.
pub fn is_supported_gameplay_tag_float32_proof_pair(
    ancestry_profile_id: &str,
    map_proof_id: &str,
) -> bool {
    gore_generation::is_supported_tag_proof_pair(ancestry_profile_id, map_proof_id)
}

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

    /// The same components a table row publishes, but read off the bytes actually on disk. Both
    /// sides go through `gore_generation::derived_profile_sha256`, so the ID a row claims and the
    /// ID an install derives cannot be computed two different ways.
    fn components(&self) -> ProfileComponents {
        ProfileComponents {
            fingerprint_format: self.script_cache_fingerprint_format,
            script_cache_guid: self.script_cache_guid,
            script_cache_mutation_stable_sha256: self.script_cache_mutation_stable_sha256,
            scalar_default_operand_count: self.scalar_default_operand_count,
            gameplay_tag_float32_operand_count: self.gameplay_tag_float32_operand_count,
            binds_source_sha256: self.binds_source_sha256,
            binds_bridge_sha256: self.binds_bridge_sha256,
            usmap_source_sha256: self.usmap_source_sha256,
            usmap_graph_sha256: self.usmap_graph_sha256,
            resolved_profile_sha256: self.resolved_profile_sha256,
        }
    }

    fn derived_profile_sha256(&self) -> [u8; 32] {
        gore_generation::derived_profile_sha256(&self.components())
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
    sealed_profile: &'static GenerationRow,
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
        if source_sha256 != sealed_profile.usmap.sha256 {
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
        if usmap_graph_sha256 != sealed_profile.usmap_class_graph_sha256 {
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
        if resolved_profile_sha256 != sealed_profile.resolved_class_profile_sha256 {
            return Err(DefaultAncestryError::ResolvedProfileDrift);
        }
        let gameplay_tag_float32_map_profile_sha256 =
            rows_sha256(&mut gameplay_tag_float32_map_rows);
        if gameplay_tag_float32_map_profile_sha256
            != sealed_profile.gameplay_tag_float32_map_profile_sha256
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
        if expected_id_sha256(sealed_profile.native_ancestry_profile_id)
            != Some(evidence.profile_sha256)
        {
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
            expected_id_sha256(self.sealed_profile.native_ancestry_profile_id),
            Some(self.evidence.profile_sha256)
        );
        self.sealed_profile.native_ancestry_profile_id
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
        let sealed_profile = &gore_generation::GENERATION_ROWS[0];
        Self {
            sealed_profile,
            evidence: DefaultNativeAncestryEvidence {
                script_cache_guid: sealed_profile.script_cache_guid,
                script_cache_fingerprint_format: DEFAULT_CACHE_FINGERPRINT_FORMAT,
                script_cache_mutation_stable_sha256: sealed_profile
                    .script_cache_mutation_stable_sha256,
                scalar_default_operand_count: sealed_profile.scalar_default_operand_count,
                gameplay_tag_float32_operand_count: sealed_profile
                    .gameplay_tag_float32_operand_count,
                binds_source_sha256: [0; 32],
                binds_bridge_sha256: [0; 32],
                usmap_source_sha256: [0; 32],
                usmap_graph_sha256: [0; 32],
                resolved_profile_sha256: [0; 32],
                profile_sha256: expected_id_sha256(sealed_profile.native_ancestry_profile_id)
                    .expect("production profile id"),
            },
            class_ids,
            super_ids,
            gameplay_tag_float32_maps: maps
                .iter()
                .map(|(owner, field)| ((*owner).to_owned(), (*field).to_owned()))
                .collect(),
            gameplay_tag_float32_map_profile_sha256: sealed_profile
                .gameplay_tag_float32_map_profile_sha256,
            gameplay_tag_float32_map_proof_sha256: expected_id_sha256(
                sealed_profile.gameplay_tag_float32_map_proof_id,
            )
            .expect("production map proof id"),
        }
    }
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
        let row = &gore_generation::GENERATION_ROWS[0];
        let fingerprint = DefaultCacheFingerprint {
            sha256: row.script_cache_mutation_stable_sha256,
            scalar_operand_count: row.scalar_default_operand_count,
            tag_operand_count: row.gameplay_tag_float32_operand_count,
        };
        assert!(profile.supports_cache(&row.script_cache_guid, &fingerprint));
        let mut wrong_guid = row.script_cache_guid;
        wrong_guid[0] ^= 1;
        assert!(!profile.supports_cache(&wrong_guid, &fingerprint));
        let mut wrong_fingerprint = fingerprint;
        wrong_fingerprint.sha256[0] ^= 1;
        assert!(!profile.supports_cache(&row.script_cache_guid, &wrong_fingerprint));
        let mut wrong_count = fingerprint;
        wrong_count.tag_operand_count -= 1;
        assert!(!profile.supports_cache(&row.script_cache_guid, &wrong_count));
        assert!(profile.proves_ancestry("UNativeLeaf", "UNativeRoot"));
        assert!(!profile.proves_ancestry("UNativeRoot", "UNativeLeaf"));

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
    fn every_audited_generation_is_reachable_through_this_crates_fingerprint_type() {
        // What is left of the old hardcoded V1/V2 non-crossing check once `gore-generation`'s
        // `every_row_is_reachable_through_every_gate` covers every pair: the seam.
        // `DefaultCacheFingerprint` is a different type from the table's `CacheFingerprint` and
        // the conversion is three fields that are easy to transpose — the two operand counts
        // differ, so a swap fails here instead of quietly refusing an audited install.
        assert_eq!(
            DEFAULT_CACHE_FINGERPRINT_FORMAT,
            gore_generation::CACHE_FINGERPRINT_FORMAT,
            "the fingerprint format is one of the nine components of every sealed profile ID, so \
             the table and the pass that computes it must never name different versions"
        );
        for row in gore_generation::rows() {
            let fingerprint = DefaultCacheFingerprint {
                sha256: row.script_cache_mutation_stable_sha256,
                scalar_operand_count: row.scalar_default_operand_count,
                tag_operand_count: row.gameplay_tag_float32_operand_count,
            };
            assert_eq!(
                sealed_profile_for_cache(&row.script_cache_guid, &fingerprint)
                    .map(|found| found.native_ancestry_profile_id),
                Some(row.native_ancestry_profile_id),
                "{} is audited but this crate cannot reach it",
                row.id
            );
            let mut wrong_count = fingerprint;
            wrong_count.scalar_operand_count += 1;
            assert!(sealed_profile_for_cache(&row.script_cache_guid, &wrong_count).is_none());
            let mut wrong_fingerprint = fingerprint;
            wrong_fingerprint.sha256[0] ^= 1;
            assert!(sealed_profile_for_cache(&row.script_cache_guid, &wrong_fingerprint).is_none());
            assert!(is_supported_gameplay_tag_float32_proof_pair(
                row.native_ancestry_profile_id,
                row.gameplay_tag_float32_map_proof_id
            ));
        }
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
            expected.gameplay_tag_float32_map_profile_sha256
        );
        assert_eq!(profile.profile_id(), expected.native_ancestry_profile_id);
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
        assert_eq!(
            reconstructed.profile_id(),
            expected.native_ancestry_profile_id
        );
        let tag_report =
            super::super::native_tag_map::inspect_native_tag_maps(&mutated, &reconstructed)
                .expect("rediscover native tag-map sites after tag edit");
        assert_eq!(
            tag_report.site_count(),
            expected.gameplay_tag_float32_operand_count
        );
        assert_eq!(
            tag_report.fingerprint_format(),
            DEFAULT_CACHE_FINGERPRINT_FORMAT
        );
        assert_eq!(tag_report.fingerprint_sha256(), fingerprint.sha256);
        assert_eq!(
            tag_report.scalar_operand_count(),
            expected.scalar_default_operand_count
        );
        assert_eq!(
            tag_report.tag_operand_count(),
            fingerprint.tag_operand_count
        );
        assert_eq!(
            tag_report.ancestry_profile_id(),
            expected.native_ancestry_profile_id
        );
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
            site.selector.ancestry_profile.as_deref() == Some(expected.native_ancestry_profile_id)
        }));
    }
}
