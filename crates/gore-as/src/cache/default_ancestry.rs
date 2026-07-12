//! SHA-sealed native class ancestry for offline default mutation.
//!
//! The script cache contains script-to-script edges and the first native base name, but not the
//! native chain above that base. This profile joins three independently parsed, exact-build
//! witnesses: the script-cache GUID, the matching `Binds.Cache` AngelScript-to-Unreal name map,
//! and the matching USMAP class graph. Any missing identity or parser-output digest disables the
//! profile; ordinary direct-owner default discovery continues without it.

use std::collections::{HashMap, HashSet};

use gore_asset::{SchemaDb, SchemaError, SchemaId, SchemaKind};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::binds::NativeApi;

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

#[derive(Debug, Error)]
pub enum DefaultAncestryError {
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
    #[error("USMAP class hierarchy contains a cycle reachable from {class}")]
    CyclicHierarchy { class: String },
}

/// Opaque, exact-build proof for native class ancestry. Callers cannot construct arbitrary edges.
#[derive(Debug, Clone)]
pub struct DefaultNativeAncestry {
    script_cache_guid: [u8; 16],
    class_ids: HashMap<String, SchemaId>,
    super_ids: Vec<Option<SchemaId>>,
}

impl DefaultNativeAncestry {
    /// Join a sealed Binds class-name map with a sealed, fully validated USMAP class graph.
    pub fn from_schema_db(
        native: &NativeApi,
        script_cache_guid: &[u8; 16],
        schemas: &SchemaDb,
    ) -> Result<Self, DefaultAncestryError> {
        let class_paths = native
            .verified_default_class_paths(script_cache_guid)
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
            let parent = schemas.super_schema_id(record.id).map_err(|error| {
                DefaultAncestryError::BridgeResolution {
                    script_class: "<USMAP graph>".into(),
                    path: record.qualified_name(),
                    error: error.to_string(),
                }
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
        if rows_sha256(&mut graph_rows) != VERIFIED_USMAP_CLASS_GRAPH_SHA256 {
            return Err(DefaultAncestryError::UsmapGraphDrift);
        }

        let mut class_ids = HashMap::new();
        let mut claimed_ids = HashSet::new();
        let mut resolved_rows = Vec::new();
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
        if rows_sha256(&mut resolved_rows) != VERIFIED_RESOLVED_CLASS_PROFILE_SHA256 {
            return Err(DefaultAncestryError::ResolvedProfileDrift);
        }

        let profile = Self {
            script_cache_guid: *script_cache_guid,
            class_ids,
            super_ids,
        };
        profile.validate_acyclic()?;
        Ok(profile)
    }

    pub fn class_count(&self) -> usize {
        self.class_ids.len()
    }

    pub(crate) fn supports_cache(&self, script_cache_guid: &[u8; 16]) -> bool {
        script_cache_guid == &self.script_cache_guid
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
