use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use sha2::{Digest, Sha256};
use thiserror::Error;

pub type SchemaId = usize;

const MAX_USMAP_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_USMAP_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Unknown,
    Struct,
    Class,
}

#[derive(Debug, Clone)]
pub struct SchemaRecord {
    pub id: SchemaId,
    pub name: String,
    pub module_path: Option<String>,
    pub kind: SchemaKind,
    pub super_name: Option<String>,
    pub properties: Vec<usmap::Property>,
}

impl SchemaRecord {
    pub fn qualified_name(&self) -> String {
        match &self.module_path {
            Some(module) if !module.is_empty() => format!("{module}.{}", self.name),
            _ => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySlot {
    /// Absolute index consumed by the unversioned-property header.
    pub schema_index: usize,
    pub property_name: String,
    pub array_index: usize,
    pub array_dimension: usize,
    pub inner: usmap::PropertyInner,
    pub declaring_schema_id: SchemaId,
    pub declaring_schema_name: String,
    pub declaring_module_path: Option<String>,
}

impl PropertySlot {
    pub fn path(&self) -> String {
        if self.array_dimension > 1 {
            format!("{}[{}]", self.property_name, self.array_index)
        } else {
            self.property_name.clone()
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchemaError {
    #[error("USMAP input is empty or truncated while reading {0}")]
    Truncated(&'static str),
    #[error("USMAP input is {actual} bytes; the safety limit is {limit} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid USMAP magic 0x{0:04x}")]
    InvalidMagic(u16),
    #[error("unsupported USMAP version {0}")]
    UnsupportedVersion(u8),
    #[error("unsupported USMAP compression method {0}")]
    UnsupportedCompression(u8),
    #[error("USMAP advertises {actual} decompressed bytes; the safety limit is {limit} bytes")]
    DecompressedTooLarge { actual: usize, limit: usize },
    #[error("USMAP payload length mismatch: header says {advertised}, file contains {actual}")]
    PayloadLengthMismatch { advertised: usize, actual: usize },
    #[error("USMAP parser failed: {0}")]
    Parse(String),
    #[error("USMAP parser panicked: {0}")]
    ParserPanic(String),
    #[error("USMAP {extension} metadata has {actual} entries for {schemas} schemas")]
    MetadataLength {
        extension: &'static str,
        schemas: usize,
        actual: usize,
    },
    #[error("schema {query:?} was not found")]
    SchemaNotFound { query: String },
    #[error("schema {query:?} is ambiguous: {candidates:?}")]
    SchemaAmbiguous {
        query: String,
        candidates: Vec<String>,
    },
    #[error("schema {0} is not a class")]
    NotAClass(String),
    #[error("schema id {0} is out of range")]
    InvalidSchemaId(SchemaId),
    #[error("super schema {super_name:?} for {schema} was not found")]
    SuperNotFound { schema: String, super_name: String },
    #[error("super schema {super_name:?} for {schema} is ambiguous: {candidates:?}")]
    SuperAmbiguous {
        schema: String,
        super_name: String,
        candidates: Vec<String>,
    },
    #[error("inheritance cycle detected: {0:?}")]
    InheritanceCycle(Vec<String>),
    #[error("property {property:?} in {schema} has array_dim=0")]
    ZeroArrayDimension { schema: String, property: String },
    #[error("property layout size overflow in {0}")]
    PropertyCountOverflow(String),
    #[error(
        "property {property:?} in {schema} occupies local slot {slot}, outside the {count}-slot schema"
    )]
    PropertyOutOfRange {
        schema: String,
        property: String,
        slot: usize,
        count: usize,
    },
    #[error("properties {first:?} and {second:?} overlap at local slot {slot} in {schema}")]
    PropertyOverlap {
        schema: String,
        first: String,
        second: String,
        slot: usize,
    },
    #[error("schema {schema} has no serializable property at local slot {slot}")]
    PropertyGap { schema: String, slot: usize },
}

#[derive(Debug, Clone)]
pub struct SchemaDb {
    schemas: Vec<SchemaRecord>,
    by_name: HashMap<String, Vec<SchemaId>>,
    by_qualified: HashMap<(String, String), Vec<SchemaId>>,
    source_sha256: Option<[u8; 32]>,
}

impl SchemaDb {
    /// Parse a complete `.usmap` file after a small bounded header preflight.
    ///
    /// The upstream parser currently contains a few assertions for malformed
    /// maps; catch them here so corrupt input becomes an ordinary error.
    pub fn from_usmap(bytes: &[u8]) -> Result<Self, SchemaError> {
        preflight_usmap(bytes)?;
        let parsed = catch_unwind(AssertUnwindSafe(|| {
            usmap::Usmap::read(&mut Cursor::new(bytes))
        }))
        .map_err(|panic| SchemaError::ParserPanic(panic_message(panic)))?
        .map_err(|error| SchemaError::Parse(error.to_string()))?;
        let mut db = Self::from_parsed(parsed)?;
        db.source_sha256 = Some(Sha256::digest(bytes).into());
        Ok(db)
    }

    /// Build an index from an already parsed map. Schema vector order is kept
    /// because PPTH/EATR metadata is positional and duplicate names are legal.
    pub fn from_parsed(parsed: usmap::Usmap) -> Result<Self, SchemaError> {
        let usmap::Usmap {
            structs,
            ppth,
            eatr,
            ..
        } = parsed;

        let schema_count = structs.len();
        let module_paths = match ppth {
            Some(extension) => {
                if extension.structs.len() != schema_count {
                    return Err(SchemaError::MetadataLength {
                        extension: "PPTH",
                        schemas: schema_count,
                        actual: extension.structs.len(),
                    });
                }
                extension.structs.into_iter().map(nonempty).collect()
            }
            None => vec![None; schema_count],
        };
        let kinds = match eatr {
            Some(extension) => {
                if extension.struct_flags.len() != schema_count {
                    return Err(SchemaError::MetadataLength {
                        extension: "EATR",
                        schemas: schema_count,
                        actual: extension.struct_flags.len(),
                    });
                }
                extension
                    .struct_flags
                    .into_iter()
                    .map(|flags| match flags.type_ {
                        usmap::FlagsType::Class => SchemaKind::Class,
                        usmap::FlagsType::Struct => SchemaKind::Struct,
                        usmap::FlagsType::Unknown => SchemaKind::Unknown,
                    })
                    .collect()
            }
            None => vec![SchemaKind::Unknown; schema_count],
        };

        let schemas: Vec<_> = structs
            .into_iter()
            .zip(module_paths)
            .zip(kinds)
            .enumerate()
            .map(|(id, ((schema, module_path), kind))| SchemaRecord {
                id,
                name: schema.name,
                module_path,
                kind,
                super_name: schema.super_struct.and_then(nonempty),
                properties: schema.properties,
            })
            .collect();

        let mut by_name: HashMap<String, Vec<SchemaId>> = HashMap::new();
        let mut by_qualified: HashMap<(String, String), Vec<SchemaId>> = HashMap::new();
        for schema in &schemas {
            by_name
                .entry(fold(&schema.name))
                .or_default()
                .push(schema.id);
            if let Some(module) = &schema.module_path {
                by_qualified
                    .entry((fold(module), fold(&schema.name)))
                    .or_default()
                    .push(schema.id);
            }
        }

        Ok(Self {
            schemas,
            by_name,
            by_qualified,
            source_sha256: None,
        })
    }

    /// SHA-256 of the exact `.usmap` bytes passed to [`Self::from_usmap`].
    /// Synthetic databases built with [`Self::from_parsed`] have no raw source.
    pub fn source_sha256(&self) -> Option<[u8; 32]> {
        self.source_sha256
    }

    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    pub fn schemas(&self) -> &[SchemaRecord] {
        &self.schemas
    }

    pub fn schema(&self, id: SchemaId) -> Result<&SchemaRecord, SchemaError> {
        self.schemas.get(id).ok_or(SchemaError::InvalidSchemaId(id))
    }

    /// Resolve the direct parent of one schema using the same strict module-aware rules as
    /// [`Self::flatten_slots`]. A missing, ambiguous, or differently-kinded parent is an error;
    /// callers never receive a guessed inheritance edge.
    pub fn super_schema_id(&self, id: SchemaId) -> Result<Option<SchemaId>, SchemaError> {
        let schema = self.schema(id)?;
        schema
            .super_name
            .as_deref()
            .map(|super_name| self.resolve_super(schema, super_name))
            .transpose()
    }

    /// Resolve a direct inheritance edge for mutation evidence. Unlike the general schema
    /// resolver this is case-sensitive and accepts only canonical Class -> Class edges; Unknown
    /// and Struct records are never compatible fallbacks.
    pub fn exact_class_super_schema_id(
        &self,
        id: SchemaId,
    ) -> Result<Option<SchemaId>, SchemaError> {
        let schema = self.schema(id)?;
        if schema.kind != SchemaKind::Class {
            return Err(SchemaError::NotAClass(schema.qualified_name()));
        }
        let Some(super_name) = schema.super_name.as_deref() else {
            return Ok(None);
        };
        let candidates: Vec<_> = self
            .schemas
            .iter()
            .filter(|candidate| candidate.kind == SchemaKind::Class && candidate.name == super_name)
            .map(|candidate| candidate.id)
            .collect();
        if let Some(module) = schema.module_path.as_deref() {
            let same_module: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|candidate| self.schemas[*candidate].module_path.as_deref() == Some(module))
                .collect();
            match same_module.as_slice() {
                [parent] => return Ok(Some(*parent)),
                many if !many.is_empty() => {
                    return Err(self.super_ambiguous(schema, super_name, same_module));
                }
                _ => {}
            }
        }
        match candidates.as_slice() {
            [] => Err(SchemaError::SuperNotFound {
                schema: schema.qualified_name(),
                super_name: super_name.to_string(),
            }),
            [parent] => Ok(Some(*parent)),
            _ => Err(self.super_ambiguous(schema, super_name, candidates)),
        }
    }

    /// Resolve either a short name or `/Script/Module.Name`. Short names must
    /// be unique; duplicate schemas are never silently collapsed.
    pub fn resolve(&self, query: &str) -> Result<SchemaId, SchemaError> {
        let query = query.trim();
        let ids = if let Some((module, name)) = split_qualified(query) {
            self.by_qualified
                .get(&(fold(module), fold(name)))
                .cloned()
                .unwrap_or_default()
        } else {
            self.by_name.get(&fold(query)).cloned().unwrap_or_default()
        };
        self.unique(query, ids)
    }

    pub fn resolve_class(&self, query: &str) -> Result<SchemaId, SchemaError> {
        let query = query.trim();
        let ids = if let Some((module, name)) = split_qualified(query) {
            self.by_qualified
                .get(&(fold(module), fold(name)))
                .cloned()
                .unwrap_or_default()
        } else {
            self.by_name.get(&fold(query)).cloned().unwrap_or_default()
        };
        let class_ids: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| self.schemas[*id].kind == SchemaKind::Class)
            .collect();
        if class_ids.is_empty() && ids.len() == 1 {
            return Err(SchemaError::NotAClass(
                self.schemas[ids[0]].qualified_name(),
            ));
        }
        self.unique(query, class_ids)
    }

    /// Flatten derived schema first, then each parent. This is the index order
    /// used by Unreal's unversioned-property header.
    pub fn flatten_slots(&self, id: SchemaId) -> Result<Vec<PropertySlot>, SchemaError> {
        let mut slots = Vec::new();
        let mut seen = HashSet::new();
        let mut chain = Vec::new();
        let mut current = id;
        let mut absolute_base = 0usize;

        loop {
            let schema = self.schema(current)?;
            if !seen.insert(current) {
                chain.push(schema.qualified_name());
                return Err(SchemaError::InheritanceCycle(chain));
            }
            chain.push(schema.qualified_name());
            absolute_base = self.append_local_slots(schema, absolute_base, &mut slots)?;

            let Some(super_name) = schema.super_name.as_deref() else {
                break;
            };
            current = self.resolve_super(schema, super_name)?;
        }

        Ok(slots)
    }

    fn append_local_slots(
        &self,
        schema: &SchemaRecord,
        absolute_base: usize,
        out: &mut Vec<PropertySlot>,
    ) -> Result<usize, SchemaError> {
        let schema_name = schema.qualified_name();
        let mut local_count = 0usize;
        for property in &schema.properties {
            let dim = property.array_dim as usize;
            if dim == 0 {
                return Err(SchemaError::ZeroArrayDimension {
                    schema: schema_name,
                    property: property.name.clone(),
                });
            }
            local_count = local_count
                .checked_add(dim)
                .ok_or_else(|| SchemaError::PropertyCountOverflow(schema.qualified_name()))?;
        }

        let mut local: Vec<Option<PropertySlot>> = vec![None; local_count];
        for property in &schema.properties {
            let dim = property.array_dim as usize;
            let start = property.index as usize;
            for array_index in 0..dim {
                let local_index = start
                    .checked_add(array_index)
                    .ok_or_else(|| SchemaError::PropertyCountOverflow(schema.qualified_name()))?;
                if local_index >= local_count {
                    return Err(SchemaError::PropertyOutOfRange {
                        schema: schema.qualified_name(),
                        property: property.name.clone(),
                        slot: local_index,
                        count: local_count,
                    });
                }
                if let Some(existing) = &local[local_index] {
                    return Err(SchemaError::PropertyOverlap {
                        schema: schema.qualified_name(),
                        first: existing.property_name.clone(),
                        second: property.name.clone(),
                        slot: local_index,
                    });
                }
                let schema_index = absolute_base
                    .checked_add(local_index)
                    .ok_or_else(|| SchemaError::PropertyCountOverflow(schema.qualified_name()))?;
                local[local_index] = Some(PropertySlot {
                    schema_index,
                    property_name: property.name.clone(),
                    array_index,
                    array_dimension: dim,
                    inner: property.inner.clone(),
                    declaring_schema_id: schema.id,
                    declaring_schema_name: schema.name.clone(),
                    declaring_module_path: schema.module_path.clone(),
                });
            }
        }

        for (slot, property) in local.into_iter().enumerate() {
            out.push(property.ok_or_else(|| SchemaError::PropertyGap {
                schema: schema.qualified_name(),
                slot,
            })?);
        }
        absolute_base
            .checked_add(local_count)
            .ok_or_else(|| SchemaError::PropertyCountOverflow(schema.qualified_name()))
    }

    fn resolve_super(
        &self,
        schema: &SchemaRecord,
        super_name: &str,
    ) -> Result<SchemaId, SchemaError> {
        let all = self
            .by_name
            .get(&fold(super_name))
            .cloned()
            .unwrap_or_default();
        let compatible = |id: &SchemaId| {
            let candidate = &self.schemas[*id];
            schema.kind == SchemaKind::Unknown
                || candidate.kind == SchemaKind::Unknown
                || candidate.kind == schema.kind
        };
        let all: Vec<_> = all.into_iter().filter(compatible).collect();

        if let Some(module) = schema.module_path.as_deref() {
            let same_module: Vec<_> = all
                .iter()
                .copied()
                .filter(|id| {
                    self.schemas[*id]
                        .module_path
                        .as_deref()
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(module))
                })
                .collect();
            if same_module.len() == 1 {
                return Ok(same_module[0]);
            }
            if same_module.len() > 1 {
                return Err(self.super_ambiguous(schema, super_name, same_module));
            }
        }

        match all.len() {
            0 => Err(SchemaError::SuperNotFound {
                schema: schema.qualified_name(),
                super_name: super_name.to_string(),
            }),
            1 => Ok(all[0]),
            _ => Err(self.super_ambiguous(schema, super_name, all)),
        }
    }

    fn unique(&self, query: &str, ids: Vec<SchemaId>) -> Result<SchemaId, SchemaError> {
        match ids.len() {
            0 => Err(SchemaError::SchemaNotFound {
                query: query.to_string(),
            }),
            1 => Ok(ids[0]),
            _ => Err(SchemaError::SchemaAmbiguous {
                query: query.to_string(),
                candidates: self.candidate_names(ids),
            }),
        }
    }

    fn super_ambiguous(
        &self,
        schema: &SchemaRecord,
        super_name: &str,
        ids: Vec<SchemaId>,
    ) -> SchemaError {
        SchemaError::SuperAmbiguous {
            schema: schema.qualified_name(),
            super_name: super_name.to_string(),
            candidates: self.candidate_names(ids),
        }
    }

    fn candidate_names(&self, ids: Vec<SchemaId>) -> Vec<String> {
        let mut names: Vec<_> = ids
            .into_iter()
            .map(|id| self.schemas[id].qualified_name())
            .collect();
        names.sort();
        names
    }
}

fn preflight_usmap(bytes: &[u8]) -> Result<(), SchemaError> {
    if bytes.len() > MAX_USMAP_FILE_BYTES {
        return Err(SchemaError::InputTooLarge {
            actual: bytes.len(),
            limit: MAX_USMAP_FILE_BYTES,
        });
    }
    let magic = read_u16(bytes, 0, "magic")?;
    if magic != 0x30c4 {
        return Err(SchemaError::InvalidMagic(magic));
    }
    let version = *bytes.get(2).ok_or(SchemaError::Truncated("version"))?;
    if version > 4 {
        return Err(SchemaError::UnsupportedVersion(version));
    }

    let mut offset = 3usize;
    if version >= 1 {
        let has_versioning = read_i32(bytes, offset, "package-version flag")?;
        offset += 4;
        if has_versioning > 0 {
            checked_advance(bytes, &mut offset, 8, "package versions")?;
            let custom_count = read_u32(bytes, offset, "custom-version count")? as usize;
            offset += 4;
            let custom_bytes = custom_count
                .checked_mul(24)
                .ok_or(SchemaError::Truncated("custom versions"))?;
            checked_advance(bytes, &mut offset, custom_bytes, "custom versions")?;
            checked_advance(bytes, &mut offset, 4, "network changelist")?;
        }
    }

    let compression = *bytes
        .get(offset)
        .ok_or(SchemaError::Truncated("compression method"))?;
    offset += 1;
    if !matches!(compression, 0 | 3) {
        return Err(SchemaError::UnsupportedCompression(compression));
    }
    let compressed_size = read_u32(bytes, offset, "compressed size")? as usize;
    offset += 4;
    let decompressed_size = read_u32(bytes, offset, "decompressed size")? as usize;
    offset += 4;
    if decompressed_size > MAX_USMAP_DECOMPRESSED_BYTES {
        return Err(SchemaError::DecompressedTooLarge {
            actual: decompressed_size,
            limit: MAX_USMAP_DECOMPRESSED_BYTES,
        });
    }
    let actual = bytes.len().saturating_sub(offset);
    if compressed_size != actual {
        return Err(SchemaError::PayloadLengthMismatch {
            advertised: compressed_size,
            actual,
        });
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize, what: &'static str) -> Result<u16, SchemaError> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .ok_or(SchemaError::Truncated(what))?
        .try_into()
        .expect("checked length");
    Ok(u16::from_le_bytes(raw))
}

fn read_u32(bytes: &[u8], offset: usize, what: &'static str) -> Result<u32, SchemaError> {
    let raw: [u8; 4] = bytes
        .get(offset..offset + 4)
        .ok_or(SchemaError::Truncated(what))?
        .try_into()
        .expect("checked length");
    Ok(u32::from_le_bytes(raw))
}

fn read_i32(bytes: &[u8], offset: usize, what: &'static str) -> Result<i32, SchemaError> {
    Ok(read_u32(bytes, offset, what)? as i32)
}

fn checked_advance(
    bytes: &[u8],
    offset: &mut usize,
    len: usize,
    what: &'static str,
) -> Result<(), SchemaError> {
    let end = offset
        .checked_add(len)
        .ok_or(SchemaError::Truncated(what))?;
    if end > bytes.len() {
        return Err(SchemaError::Truncated(what));
    }
    *offset = end;
    Ok(())
}

fn split_qualified(query: &str) -> Option<(&str, &str)> {
    let (module, name) = query.rsplit_once('.')?;
    (!module.is_empty() && !name.is_empty()).then_some((module, name))
}

fn fold(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(kind: usmap::FlagsType) -> usmap::StructFlags {
        usmap::StructFlags {
            type_: kind,
            value: 0,
            prop_flags: Vec::new(),
        }
    }

    fn fixture() -> usmap::Usmap {
        usmap::Usmap {
            enums: Vec::new(),
            structs: vec![
                usmap::Struct {
                    name: "Derived".into(),
                    super_struct: Some("Base".into()),
                    properties: vec![
                        usmap::Property {
                            name: "Weights".into(),
                            array_dim: 2,
                            index: 1,
                            inner: usmap::PropertyInner::Float,
                        },
                        usmap::Property {
                            name: "Mode".into(),
                            array_dim: 1,
                            index: 0,
                            inner: usmap::PropertyInner::Byte,
                        },
                    ],
                },
                usmap::Struct {
                    name: "Base".into(),
                    super_struct: None,
                    properties: vec![
                        usmap::Property {
                            name: "Count".into(),
                            array_dim: 1,
                            index: 0,
                            inner: usmap::PropertyInner::Int,
                        },
                        usmap::Property {
                            name: "Level".into(),
                            array_dim: 1,
                            index: 1,
                            inner: usmap::PropertyInner::UInt16,
                        },
                    ],
                },
            ],
            cext: None,
            ppth: Some(usmap::ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: vec!["/Script/Game".into(), "/Script/Game".into()],
            }),
            eatr: Some(usmap::ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags: vec![
                    flags(usmap::FlagsType::Class),
                    flags(usmap::FlagsType::Class),
                ],
            }),
            envp: None,
        }
    }

    #[test]
    fn resolves_qualified_and_case_insensitive_short_names() {
        let db = SchemaDb::from_parsed(fixture()).unwrap();
        let short = db.resolve_class("derived").unwrap();
        let qualified = db.resolve_class("/script/game.DERIVED").unwrap();
        assert_eq!(short, qualified);
        assert_eq!(
            db.schema(short).unwrap().qualified_name(),
            "/Script/Game.Derived"
        );
        let parent = db.super_schema_id(short).unwrap().expect("Derived parent");
        assert_eq!(
            db.schema(parent).unwrap().qualified_name(),
            "/Script/Game.Base"
        );
        assert_eq!(db.super_schema_id(parent).unwrap(), None);
        assert_eq!(db.exact_class_super_schema_id(short).unwrap(), Some(parent));
        assert_eq!(db.exact_class_super_schema_id(parent).unwrap(), None);
    }

    #[test]
    fn exact_class_super_rejects_case_struct_unknown_and_ambiguity() {
        let mut wrong_case = fixture();
        wrong_case.structs[0].super_struct = Some("base".into());
        let db = SchemaDb::from_parsed(wrong_case).unwrap();
        let derived = db.resolve_class("/Script/Game.Derived").unwrap();
        assert!(matches!(
            db.exact_class_super_schema_id(derived),
            Err(SchemaError::SuperNotFound { .. })
        ));

        for non_class_kind in [usmap::FlagsType::Struct, usmap::FlagsType::Unknown] {
            let mut map = fixture();
            map.eatr.as_mut().unwrap().struct_flags[1] = flags(non_class_kind);
            let db = SchemaDb::from_parsed(map).unwrap();
            let derived = db.resolve_class("/Script/Game.Derived").unwrap();
            assert!(matches!(
                db.exact_class_super_schema_id(derived),
                Err(SchemaError::SuperNotFound { .. })
            ));
        }

        let mut ambiguous = fixture();
        ambiguous.ppth.as_mut().unwrap().structs = vec![
            "/Script/Third".into(),
            "/Script/First".into(),
            "/Script/Second".into(),
        ];
        ambiguous.structs.push(usmap::Struct {
            name: "Base".into(),
            super_struct: None,
            properties: Vec::new(),
        });
        ambiguous
            .eatr
            .as_mut()
            .unwrap()
            .struct_flags
            .push(flags(usmap::FlagsType::Class));
        let db = SchemaDb::from_parsed(ambiguous).unwrap();
        let derived = db.resolve_class("/Script/Third.Derived").unwrap();
        assert!(matches!(
            db.exact_class_super_schema_id(derived),
            Err(SchemaError::SuperAmbiguous { .. })
        ));
    }

    #[test]
    fn reader_round_trip_preserves_positional_metadata() {
        let mut bytes = Vec::new();
        fixture().write(&mut bytes).unwrap();
        let db = SchemaDb::from_usmap(&bytes).unwrap();
        let expected_sha256: [u8; 32] = Sha256::digest(&bytes).into();
        assert_eq!(db.source_sha256(), Some(expected_sha256));
        assert_eq!(db.len(), 2);
        assert_eq!(db.schema(0).unwrap().kind, SchemaKind::Class);
        assert_eq!(
            db.schema(1).unwrap().module_path.as_deref(),
            Some("/Script/Game")
        );
        assert_eq!(
            SchemaDb::from_parsed(fixture()).unwrap().source_sha256(),
            None
        );
    }

    #[test]
    fn flatten_is_derived_to_base_and_expands_fixed_arrays() {
        let db = SchemaDb::from_parsed(fixture()).unwrap();
        let id = db.resolve_class("Derived").unwrap();
        let slots = db.flatten_slots(id).unwrap();
        assert_eq!(slots.len(), 5);
        assert_eq!(
            slots.iter().map(PropertySlot::path).collect::<Vec<_>>(),
            ["Mode", "Weights[0]", "Weights[1]", "Count", "Level"]
        );
        assert_eq!(
            slots
                .iter()
                .map(|slot| slot.schema_index)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4]
        );
        assert_eq!(slots[2].declaring_schema_name, "Derived");
        assert_eq!(slots[3].declaring_schema_name, "Base");
    }

    #[test]
    fn same_module_parent_wins_over_same_named_foreign_schema() {
        let mut map = fixture();
        map.structs.push(usmap::Struct {
            name: "Base".into(),
            super_struct: None,
            properties: Vec::new(),
        });
        map.ppth
            .as_mut()
            .unwrap()
            .structs
            .push("/Script/Other".into());
        map.eatr
            .as_mut()
            .unwrap()
            .struct_flags
            .push(flags(usmap::FlagsType::Class));
        let db = SchemaDb::from_parsed(map).unwrap();
        let slots = db
            .flatten_slots(db.resolve_class("/Script/Game.Derived").unwrap())
            .unwrap();
        assert_eq!(
            slots.last().unwrap().declaring_module_path.as_deref(),
            Some("/Script/Game")
        );
    }

    #[test]
    fn duplicate_short_names_are_ambiguous() {
        let mut map = fixture();
        map.structs.push(usmap::Struct {
            name: "Derived".into(),
            super_struct: None,
            properties: Vec::new(),
        });
        map.ppth
            .as_mut()
            .unwrap()
            .structs
            .push("/Script/Other".into());
        map.eatr
            .as_mut()
            .unwrap()
            .struct_flags
            .push(flags(usmap::FlagsType::Class));
        let db = SchemaDb::from_parsed(map).unwrap();
        assert!(matches!(
            db.resolve_class("Derived"),
            Err(SchemaError::SchemaAmbiguous { .. })
        ));
        assert!(db.resolve_class("/Script/Other.Derived").is_ok());
    }

    #[test]
    fn inheritance_cycles_are_rejected() {
        let mut map = fixture();
        map.structs[1].super_struct = Some("Derived".into());
        let db = SchemaDb::from_parsed(map).unwrap();
        assert!(matches!(
            db.flatten_slots(db.resolve_class("Derived").unwrap()),
            Err(SchemaError::InheritanceCycle(_))
        ));
    }

    #[test]
    fn overlaps_gaps_and_zero_arrays_are_rejected() {
        let mut overlap = fixture();
        overlap.structs[0].properties[0].index = 0;
        let db = SchemaDb::from_parsed(overlap).unwrap();
        assert!(matches!(
            db.flatten_slots(db.resolve_class("Derived").unwrap()),
            Err(SchemaError::PropertyOverlap { .. })
        ));

        let mut gap = fixture();
        gap.structs[0].properties[0].index = 2;
        let db = SchemaDb::from_parsed(gap).unwrap();
        assert!(matches!(
            db.flatten_slots(db.resolve_class("Derived").unwrap()),
            Err(SchemaError::PropertyOutOfRange { .. } | SchemaError::PropertyGap { .. })
        ));

        let mut zero = fixture();
        zero.structs[0].properties[0].array_dim = 0;
        let db = SchemaDb::from_parsed(zero).unwrap();
        assert!(matches!(
            db.flatten_slots(db.resolve_class("Derived").unwrap()),
            Err(SchemaError::ZeroArrayDimension { .. })
        ));
    }

    #[test]
    fn positional_extension_mismatch_is_rejected() {
        let mut map = fixture();
        map.ppth.as_mut().unwrap().structs.pop();
        assert!(matches!(
            SchemaDb::from_parsed(map),
            Err(SchemaError::MetadataLength {
                extension: "PPTH",
                ..
            })
        ));
    }

    #[test]
    fn malformed_headers_fail_without_entering_the_upstream_parser() {
        assert!(matches!(
            SchemaDb::from_usmap(&[0xc4, 0x30]),
            Err(SchemaError::Truncated("version"))
        ));
        assert!(matches!(
            SchemaDb::from_usmap(&[0, 0, 4]),
            Err(SchemaError::InvalidMagic(_))
        ));
    }

    #[test]
    #[ignore = "requires GORE_USMAP to point at an installed game's mappings file"]
    fn parses_live_g1r_schema_map() {
        let path = std::env::var("GORE_USMAP").expect("set GORE_USMAP");
        let bytes = std::fs::read(path).unwrap();
        let db = SchemaDb::from_usmap(&bytes).unwrap();
        assert!(db.len() > 10_000);
        assert!(db.resolve_class("GothicWeatherSettings").is_ok());
    }
}
