use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::usmap_preflight::{preflight_bounded_usmap, UsmapLimits, UsmapPreflightError};

pub type SchemaId = usize;

const MAX_USMAP_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_USMAP_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Unknown,
    Struct,
    Class,
}

/// Narrow, mutation-safe declared-property shapes exposed without leaking the USMAP parser's
/// dependency type across crate boundaries. Variants are added only after an exact wire/profile
/// audit; unknown or merely similar shapes are represented by `None` at lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactDeclaredPropertyShape {
    GameplayTagFloat32Map,
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

/// Closed budget used only by additive bounded schema lookup/flattening APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedSchemaLimits {
    pub max_work: usize,
    pub max_slots: usize,
    pub max_string_bytes: usize,
    pub max_allocation_bytes: usize,
    pub max_byte_work: usize,
    pub max_inheritance_depth: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BoundedSchemaUsage {
    pub work: usize,
    pub slots: usize,
    pub string_bytes: usize,
    pub allocation_bytes: usize,
    pub byte_work: usize,
}

#[derive(Debug)]
pub struct BoundedSchemaBudget {
    limits: BoundedSchemaLimits,
    usage: BoundedSchemaUsage,
}

impl BoundedSchemaBudget {
    pub fn new(limits: BoundedSchemaLimits) -> Self {
        Self {
            limits,
            usage: BoundedSchemaUsage::default(),
        }
    }

    pub fn usage(&self) -> BoundedSchemaUsage {
        self.usage
    }

    fn charge(
        used: &mut usize,
        amount: usize,
        limit: usize,
        resource: &'static str,
    ) -> Result<(), BoundedSchemaError> {
        let attempted = used
            .checked_add(amount)
            .ok_or(BoundedSchemaError::ResourceLimit { resource })?;
        if attempted > limit {
            return Err(BoundedSchemaError::ResourceLimit { resource });
        }
        *used = attempted;
        Ok(())
    }

    fn work(&mut self, amount: usize) -> Result<(), BoundedSchemaError> {
        Self::charge(
            &mut self.usage.work,
            amount,
            self.limits.max_work,
            "schema work",
        )
    }

    fn slots(&mut self, amount: usize) -> Result<(), BoundedSchemaError> {
        Self::charge(
            &mut self.usage.slots,
            amount,
            self.limits.max_slots,
            "flattened slots",
        )
    }

    fn strings(&mut self, amount: usize) -> Result<(), BoundedSchemaError> {
        Self::charge(
            &mut self.usage.string_bytes,
            amount,
            self.limits.max_string_bytes,
            "schema strings",
        )
    }

    fn allocation(&mut self, amount: usize) -> Result<(), BoundedSchemaError> {
        Self::charge(
            &mut self.usage.allocation_bytes,
            amount,
            self.limits.max_allocation_bytes,
            "schema allocations",
        )
    }

    fn bytes(&mut self, amount: usize) -> Result<(), BoundedSchemaError> {
        Self::charge(
            &mut self.usage.byte_work,
            amount,
            self.limits.max_byte_work,
            "schema byte work",
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BoundedSchemaError {
    #[error("bounded schema lookup did not find a compatible schema")]
    Missing,
    #[error("bounded schema lookup is ambiguous")]
    Ambiguous,
    #[error("bounded schema lookup found the wrong schema kind")]
    WrongKind,
    #[error("bounded schema inheritance cycles")]
    InheritanceCycle,
    #[error("bounded schema property layout is invalid")]
    InvalidLayout,
    #[error("bounded schema operation exhausted {resource}")]
    ResourceLimit { resource: &'static str },
    #[error("bounded schema operation could not reserve proven storage")]
    Allocation,
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
    #[error(transparent)]
    BoundedPreflight(#[from] UsmapPreflightError),
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
    #[error("schema {schema} declares property {property:?} {count} times")]
    DuplicateDeclaredProperty {
        schema: String,
        property: String,
        count: usize,
    },
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

    /// Parse an exact `.usmap` only after a complete allocation-bounded structural preflight.
    ///
    /// Unlike [`Self::from_usmap`]'s legacy header-only guard, this validates the decompressed
    /// name map, every enum/schema/property record, recursive property-type depth, and every
    /// supported extension aggregate before the upstream parser can allocate from wire counts.
    pub fn from_usmap_bounded(bytes: &[u8], limits: UsmapLimits) -> Result<Self, SchemaError> {
        Self::from_usmap_bounded_with(bytes, limits, |bytes| {
            usmap::Usmap::read(&mut Cursor::new(bytes)).map_err(|error| error.to_string())
        })
    }

    fn from_usmap_bounded_with<F>(
        bytes: &[u8],
        limits: UsmapLimits,
        parse: F,
    ) -> Result<Self, SchemaError>
    where
        F: FnOnce(&[u8]) -> Result<usmap::Usmap, String>,
    {
        preflight_bounded_usmap(bytes, limits)?;
        let parsed = catch_unwind(AssertUnwindSafe(|| parse(bytes)))
            .map_err(|panic| SchemaError::ParserPanic(panic_message(panic)))?
            .map_err(SchemaError::Parse)?;
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

    /// Resolve one property declared directly on an exact Class schema.
    ///
    /// The lookup is case-sensitive, never walks parents, rejects duplicate declarations, and
    /// recognizes only scalar (`array_dim == 1`) `TMap<FGameplayTag,float32>` encoded by USMAP as
    /// `Map { key: Struct { name: "GameplayTag" }, value: Float }`.
    pub fn exact_declared_property_shape(
        &self,
        id: SchemaId,
        property_name: &str,
    ) -> Result<Option<ExactDeclaredPropertyShape>, SchemaError> {
        let schema = self.schema(id)?;
        if schema.kind != SchemaKind::Class {
            return Err(SchemaError::NotAClass(schema.qualified_name()));
        }
        let matches: Vec<_> = schema
            .properties
            .iter()
            .filter(|property| property.name == property_name)
            .collect();
        let property = match matches.as_slice() {
            [] => return Ok(None),
            [property] => *property,
            many => {
                return Err(SchemaError::DuplicateDeclaredProperty {
                    schema: schema.qualified_name(),
                    property: property_name.to_owned(),
                    count: many.len(),
                });
            }
        };
        if property.array_dim != 1 {
            return Ok(None);
        }
        let exact = matches!(
            &property.inner,
            usmap::PropertyInner::Map { key, value }
                if matches!(key.as_ref(), usmap::PropertyInner::Struct { name } if name == "GameplayTag")
                    && matches!(value.as_ref(), usmap::PropertyInner::Float)
        );
        Ok(exact.then_some(ExactDeclaredPropertyShape::GameplayTagFloat32Map))
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

    /// Resolve a class without materializing candidate IDs or diagnostic names.
    pub fn resolve_class_compact_bounded(
        &self,
        query: &str,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<SchemaId, BoundedSchemaError> {
        self.resolve_compact_bounded_kind(query, Some(SchemaKind::Class), budget)
    }

    /// Resolve any unique schema without materializing candidate diagnostics.
    pub fn resolve_compact_bounded(
        &self,
        query: &str,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<SchemaId, BoundedSchemaError> {
        self.resolve_compact_bounded_kind(query, None, budget)
    }

    fn resolve_compact_bounded_kind(
        &self,
        query: &str,
        required_kind: Option<SchemaKind>,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<SchemaId, BoundedSchemaError> {
        let query = query.trim();
        let (module, name) = split_qualified(query)
            .map(|(module, name)| (Some(module), name))
            .unwrap_or((None, query));
        let folded = fold_bounded(name, budget)?;
        let candidates = self.by_name.get(&folded).map(Vec::as_slice).unwrap_or(&[]);
        let mut compatible_count = 0usize;
        let mut compatible = 0usize;
        let mut any_count = 0usize;
        for id in candidates {
            budget.work(1)?;
            let candidate = &self.schemas[*id];
            if let Some(module) = module {
                let candidate_module = candidate.module_path.as_deref().unwrap_or("");
                budget.bytes(module.len().checked_add(candidate_module.len()).ok_or(
                    BoundedSchemaError::ResourceLimit {
                        resource: "schema byte work",
                    },
                )?)?;
                if !candidate_module.eq_ignore_ascii_case(module) {
                    continue;
                }
            }
            any_count += 1;
            if required_kind.is_none_or(|kind| candidate.kind == kind) {
                compatible_count += 1;
                compatible = *id;
            }
        }
        match compatible_count {
            1 => Ok(compatible),
            count if count > 1 => Err(BoundedSchemaError::Ambiguous),
            _ if any_count == 1 && required_kind.is_some() => Err(BoundedSchemaError::WrongKind),
            _ => Err(BoundedSchemaError::Missing),
        }
    }

    /// Flatten derived-to-base slots with every vector and recursive clone
    /// charged before allocation. Errors remain compact and allocation-free.
    pub fn flatten_slots_bounded(
        &self,
        id: SchemaId,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<Vec<PropertySlot>, BoundedSchemaError> {
        if id >= self.schemas.len() {
            return Err(BoundedSchemaError::Missing);
        }
        let mut chain = Vec::new();
        let mut current = id;

        loop {
            if chain.len() >= budget.limits.max_inheritance_depth {
                return Err(BoundedSchemaError::ResourceLimit {
                    resource: "inheritance depth",
                });
            }
            budget.work(chain.len().saturating_add(1))?;
            if chain.contains(&current) {
                return Err(BoundedSchemaError::InheritanceCycle);
            }
            reserve_schema_chain_entry(&mut chain, budget)?;
            chain.push(current);

            let schema = &self.schemas[current];
            let Some(super_name) = schema.super_name.as_deref() else {
                break;
            };
            current = self.resolve_super_compact_bounded(schema, super_name, budget)?;
        }

        budget.allocation(
            chain
                .len()
                .checked_mul(std::mem::size_of::<usize>())
                .ok_or(BoundedSchemaError::ResourceLimit {
                    resource: "schema allocations",
                })?,
        )?;
        let mut local_counts = Vec::new();
        local_counts
            .try_reserve_exact(chain.len())
            .map_err(|_| BoundedSchemaError::Allocation)?;
        let mut total_slots = 0usize;
        for schema_id in &chain {
            let local_count = self.local_slot_count_bounded(&self.schemas[*schema_id], budget)?;
            total_slots = total_slots
                .checked_add(local_count)
                .ok_or(BoundedSchemaError::InvalidLayout)?;
            local_counts.push(local_count);
        }
        budget.slots(total_slots)?;

        budget.allocation(
            chain
                .len()
                .checked_mul(std::mem::size_of::<Vec<Option<PropertySlot>>>())
                .ok_or(BoundedSchemaError::ResourceLimit {
                    resource: "schema allocations",
                })?,
        )?;
        let mut levels = Vec::new();
        levels
            .try_reserve_exact(chain.len())
            .map_err(|_| BoundedSchemaError::Allocation)?;
        let mut absolute_base = 0usize;
        for (schema_id, local_count) in chain.iter().zip(local_counts) {
            levels.push(self.build_local_slots_bounded(
                &self.schemas[*schema_id],
                absolute_base,
                local_count,
                budget,
            )?);
            absolute_base = absolute_base
                .checked_add(local_count)
                .ok_or(BoundedSchemaError::InvalidLayout)?;
        }

        budget.allocation(
            total_slots
                .checked_mul(std::mem::size_of::<PropertySlot>())
                .ok_or(BoundedSchemaError::ResourceLimit {
                    resource: "schema allocations",
                })?,
        )?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(total_slots)
            .map_err(|_| BoundedSchemaError::Allocation)?;
        budget.work(total_slots)?;
        for level in levels {
            for slot in level {
                slots.push(slot.ok_or(BoundedSchemaError::InvalidLayout)?);
            }
        }
        Ok(slots)
    }

    fn local_slot_count_bounded(
        &self,
        schema: &SchemaRecord,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<usize, BoundedSchemaError> {
        let mut local_count = 0usize;
        for property in &schema.properties {
            budget.work(1)?;
            let dim = usize::from(property.array_dim);
            if dim == 0 {
                return Err(BoundedSchemaError::InvalidLayout);
            }
            local_count = local_count
                .checked_add(dim)
                .ok_or(BoundedSchemaError::InvalidLayout)?;
        }
        Ok(local_count)
    }

    fn build_local_slots_bounded(
        &self,
        schema: &SchemaRecord,
        absolute_base: usize,
        local_count: usize,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<Vec<Option<PropertySlot>>, BoundedSchemaError> {
        budget.allocation(
            local_count
                .checked_mul(std::mem::size_of::<Option<PropertySlot>>())
                .ok_or(BoundedSchemaError::ResourceLimit {
                    resource: "schema allocations",
                })?,
        )?;
        let mut local = Vec::new();
        local
            .try_reserve_exact(local_count)
            .map_err(|_| BoundedSchemaError::Allocation)?;
        local.resize_with(local_count, || None);

        for property in &schema.properties {
            let dim = usize::from(property.array_dim);
            let start = usize::from(property.index);
            for array_index in 0..dim {
                budget.work(1)?;
                let local_index = start
                    .checked_add(array_index)
                    .ok_or(BoundedSchemaError::InvalidLayout)?;
                if local_index >= local_count || local[local_index].is_some() {
                    return Err(BoundedSchemaError::InvalidLayout);
                }
                let schema_index = absolute_base
                    .checked_add(local_index)
                    .ok_or(BoundedSchemaError::InvalidLayout)?;
                let property_name = clone_string_bounded(&property.name, budget)?;
                let declaring_schema_name = clone_string_bounded(&schema.name, budget)?;
                let declaring_module_path = schema
                    .module_path
                    .as_deref()
                    .map(|value| clone_string_bounded(value, budget))
                    .transpose()?;
                let inner = clone_property_inner_bounded(&property.inner, budget)?;
                local[local_index] = Some(PropertySlot {
                    schema_index,
                    property_name,
                    array_index,
                    array_dimension: dim,
                    inner,
                    declaring_schema_id: schema.id,
                    declaring_schema_name,
                    declaring_module_path,
                });
            }
        }
        budget.work(local_count)?;
        if local.iter().any(Option::is_none) {
            return Err(BoundedSchemaError::InvalidLayout);
        }
        Ok(local)
    }

    fn resolve_super_compact_bounded(
        &self,
        schema: &SchemaRecord,
        super_name: &str,
        budget: &mut BoundedSchemaBudget,
    ) -> Result<SchemaId, BoundedSchemaError> {
        let folded = fold_bounded(super_name, budget)?;
        let candidates = self.by_name.get(&folded).map(Vec::as_slice).unwrap_or(&[]);
        let mut all_count = 0usize;
        let mut all_id = 0usize;
        let mut same_count = 0usize;
        let mut same_id = 0usize;
        for id in candidates {
            budget.work(1)?;
            let candidate = &self.schemas[*id];
            let compatible = schema.kind == SchemaKind::Unknown
                || candidate.kind == SchemaKind::Unknown
                || candidate.kind == schema.kind;
            if !compatible {
                continue;
            }
            all_count += 1;
            all_id = *id;
            if let Some(module) = schema.module_path.as_deref() {
                let candidate_module = candidate.module_path.as_deref().unwrap_or("");
                budget.bytes(module.len().checked_add(candidate_module.len()).ok_or(
                    BoundedSchemaError::ResourceLimit {
                        resource: "schema byte work",
                    },
                )?)?;
                if candidate_module.eq_ignore_ascii_case(module) {
                    same_count += 1;
                    same_id = *id;
                }
            }
        }
        match same_count {
            1 => Ok(same_id),
            count if count > 1 => Err(BoundedSchemaError::Ambiguous),
            _ => match all_count {
                0 => Err(BoundedSchemaError::Missing),
                1 => Ok(all_id),
                _ => Err(BoundedSchemaError::Ambiguous),
            },
        }
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

fn fold_bounded(
    value: &str,
    budget: &mut BoundedSchemaBudget,
) -> Result<String, BoundedSchemaError> {
    budget.bytes(value.len())?;
    let mut folded = clone_string_bounded(value, budget)?;
    folded.make_ascii_lowercase();
    Ok(folded)
}

fn clone_string_bounded(
    value: &str,
    budget: &mut BoundedSchemaBudget,
) -> Result<String, BoundedSchemaError> {
    budget.strings(value.len())?;
    budget.allocation(value.len())?;
    let mut cloned = String::new();
    cloned
        .try_reserve_exact(value.len())
        .map_err(|_| BoundedSchemaError::Allocation)?;
    cloned.push_str(value);
    Ok(cloned)
}

fn reserve_schema_chain_entry(
    chain: &mut Vec<SchemaId>,
    budget: &mut BoundedSchemaBudget,
) -> Result<(), BoundedSchemaError> {
    if chain.len() == chain.capacity() {
        let new_len = chain
            .len()
            .checked_add(1)
            .ok_or(BoundedSchemaError::ResourceLimit {
                resource: "schema allocations",
            })?;
        budget.work(chain.len())?;
        budget.allocation(new_len.checked_mul(std::mem::size_of::<SchemaId>()).ok_or(
            BoundedSchemaError::ResourceLimit {
                resource: "schema allocations",
            },
        )?)?;
        chain
            .try_reserve_exact(1)
            .map_err(|_| BoundedSchemaError::Allocation)?;
    }
    Ok(())
}

pub(crate) fn clone_property_inner_bounded(
    inner: &usmap::PropertyInner,
    budget: &mut BoundedSchemaBudget,
) -> Result<usmap::PropertyInner, BoundedSchemaError> {
    budget.work(1)?;
    use usmap::PropertyInner;
    Ok(match inner {
        PropertyInner::Byte => PropertyInner::Byte,
        PropertyInner::Bool => PropertyInner::Bool,
        PropertyInner::Int => PropertyInner::Int,
        PropertyInner::Float => PropertyInner::Float,
        PropertyInner::Object => PropertyInner::Object,
        PropertyInner::Name => PropertyInner::Name,
        PropertyInner::Delegate => PropertyInner::Delegate,
        PropertyInner::Double => PropertyInner::Double,
        PropertyInner::Array { inner } => {
            budget.allocation(std::mem::size_of::<PropertyInner>())?;
            PropertyInner::Array {
                inner: Box::new(clone_property_inner_bounded(inner, budget)?),
            }
        }
        PropertyInner::Struct { name } => PropertyInner::Struct {
            name: clone_string_bounded(name, budget)?,
        },
        PropertyInner::Str => PropertyInner::Str,
        PropertyInner::Text => PropertyInner::Text,
        PropertyInner::Interface => PropertyInner::Interface,
        PropertyInner::MulticastDelegate => PropertyInner::MulticastDelegate,
        PropertyInner::WeakObject => PropertyInner::WeakObject,
        PropertyInner::LazyObject => PropertyInner::LazyObject,
        PropertyInner::AssetObject => PropertyInner::AssetObject,
        PropertyInner::SoftObject => PropertyInner::SoftObject,
        PropertyInner::UInt64 => PropertyInner::UInt64,
        PropertyInner::UInt32 => PropertyInner::UInt32,
        PropertyInner::UInt16 => PropertyInner::UInt16,
        PropertyInner::Int64 => PropertyInner::Int64,
        PropertyInner::Int16 => PropertyInner::Int16,
        PropertyInner::Int8 => PropertyInner::Int8,
        PropertyInner::Map { key, value } => {
            budget.allocation(std::mem::size_of::<PropertyInner>().checked_mul(2).ok_or(
                BoundedSchemaError::ResourceLimit {
                    resource: "schema allocations",
                },
            )?)?;
            PropertyInner::Map {
                key: Box::new(clone_property_inner_bounded(key, budget)?),
                value: Box::new(clone_property_inner_bounded(value, budget)?),
            }
        }
        PropertyInner::Set { key } => {
            budget.allocation(std::mem::size_of::<PropertyInner>())?;
            PropertyInner::Set {
                key: Box::new(clone_property_inner_bounded(key, budget)?),
            }
        }
        PropertyInner::Enum { inner, name } => {
            budget.allocation(std::mem::size_of::<PropertyInner>())?;
            PropertyInner::Enum {
                inner: Box::new(clone_property_inner_bounded(inner, budget)?),
                name: clone_string_bounded(name, budget)?,
            }
        }
        PropertyInner::FieldPath => PropertyInner::FieldPath,
        PropertyInner::Optional { inner } => {
            budget.allocation(std::mem::size_of::<PropertyInner>())?;
            PropertyInner::Optional {
                inner: Box::new(clone_property_inner_bounded(inner, budget)?),
            }
        }
        PropertyInner::Utf8Str => PropertyInner::Utf8Str,
        PropertyInner::AnsiStr => PropertyInner::AnsiStr,
        PropertyInner::Unknown => PropertyInner::Unknown,
    })
}

/// Clone one already flattened slot while charging every dynamic byte and
/// recursive `PropertyInner` allocation before it is materialized.
pub(crate) fn clone_property_slot_bounded(
    slot: &PropertySlot,
    budget: &mut BoundedSchemaBudget,
) -> Result<PropertySlot, BoundedSchemaError> {
    budget.work(1)?;
    budget.allocation(std::mem::size_of::<PropertySlot>())?;
    Ok(PropertySlot {
        schema_index: slot.schema_index,
        property_name: clone_string_bounded(&slot.property_name, budget)?,
        array_index: slot.array_index,
        array_dimension: slot.array_dimension,
        inner: clone_property_inner_bounded(&slot.inner, budget)?,
        declaring_schema_id: slot.declaring_schema_id,
        declaring_schema_name: clone_string_bounded(&slot.declaring_schema_name, budget)?,
        declaring_module_path: slot
            .declaring_module_path
            .as_deref()
            .map(|value| clone_string_bounded(value, budget))
            .transpose()?,
    })
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
    use std::cell::Cell;

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
    fn bounded_preflight_runs_before_the_injected_upstream_parser() {
        let empty = usmap::Usmap {
            enums: Vec::new(),
            structs: Vec::new(),
            cext: None,
            ppth: None,
            eatr: None,
            envp: None,
        };
        let mut bytes = Vec::new();
        empty.write(&mut bytes).unwrap();
        let payload_offset = 16;
        assert_eq!(&bytes[..3], &[0xc4, 0x30, 4]);

        let mut hostile = bytes.clone();
        hostile[payload_offset..payload_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let called = Cell::new(0usize);
        let limits = UsmapLimits {
            max_names: 0,
            ..UsmapLimits::default()
        };
        let result = SchemaDb::from_usmap_bounded_with(&hostile, limits, |_| {
            called.set(called.get() + 1);
            Ok(empty.clone())
        });
        assert!(matches!(
            result,
            Err(SchemaError::BoundedPreflight(UsmapPreflightError::Limit {
                resource: "names",
                ..
            }))
        ));
        assert_eq!(called.get(), 0);

        let parsed = SchemaDb::from_usmap_bounded_with(&bytes, UsmapLimits::default(), |_| {
            called.set(called.get() + 1);
            Ok(empty)
        })
        .unwrap();
        assert!(parsed.is_empty());
        assert_eq!(called.get(), 1);
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
    fn exact_declared_gameplay_tag_float_map_is_case_unique_class_and_scalar_only() {
        let exact_map = || usmap::Property {
            name: "Damage".into(),
            array_dim: 1,
            index: 3,
            inner: usmap::PropertyInner::Map {
                key: Box::new(usmap::PropertyInner::Struct {
                    name: "GameplayTag".into(),
                }),
                value: Box::new(usmap::PropertyInner::Float),
            },
        };
        let mut map = fixture();
        map.structs[0].properties.push(exact_map());
        let db = SchemaDb::from_parsed(map).unwrap();
        let derived = db.resolve_class("/Script/Game.Derived").unwrap();
        assert_eq!(
            db.exact_declared_property_shape(derived, "Damage").unwrap(),
            Some(ExactDeclaredPropertyShape::GameplayTagFloat32Map)
        );
        assert_eq!(
            db.exact_declared_property_shape(derived, "damage").unwrap(),
            None,
            "case folding is forbidden"
        );
        assert_eq!(
            db.exact_declared_property_shape(derived, "Count").unwrap(),
            None,
            "declared-only lookup must not walk Base"
        );

        let mut wide = fixture();
        let mut property = exact_map();
        property.array_dim = 2;
        wide.structs[0].properties.push(property);
        let db = SchemaDb::from_parsed(wide).unwrap();
        let derived = db.resolve_class("Derived").unwrap();
        assert_eq!(
            db.exact_declared_property_shape(derived, "Damage").unwrap(),
            None
        );

        let mut wrong_key = fixture();
        let mut property = exact_map();
        let usmap::PropertyInner::Map { key, .. } = &mut property.inner else {
            unreachable!()
        };
        **key = usmap::PropertyInner::Struct {
            name: "FGameplayTag".into(),
        };
        wrong_key.structs[0].properties.push(property);
        let db = SchemaDb::from_parsed(wrong_key).unwrap();
        let derived = db.resolve_class("Derived").unwrap();
        assert_eq!(
            db.exact_declared_property_shape(derived, "Damage").unwrap(),
            None
        );

        let mut duplicate = fixture();
        duplicate.structs[0]
            .properties
            .extend([exact_map(), exact_map()]);
        let db = SchemaDb::from_parsed(duplicate).unwrap();
        let derived = db.resolve_class("Derived").unwrap();
        assert!(matches!(
            db.exact_declared_property_shape(derived, "Damage"),
            Err(SchemaError::DuplicateDeclaredProperty { count: 2, .. })
        ));

        let mut not_class = fixture();
        not_class.eatr.as_mut().unwrap().struct_flags[0] = flags(usmap::FlagsType::Struct);
        let db = SchemaDb::from_parsed(not_class).unwrap();
        let derived = db.resolve("Derived").unwrap();
        assert!(matches!(
            db.exact_declared_property_shape(derived, "Damage"),
            Err(SchemaError::NotAClass(_))
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
    fn bounded_flatten_is_identical_and_every_limit_is_exact() {
        let db = SchemaDb::from_parsed(fixture()).unwrap();
        let id = db.resolve_class("Derived").unwrap();
        let legacy = db.flatten_slots(id).unwrap();
        let generous = BoundedSchemaLimits {
            max_work: 10_000,
            max_slots: 10_000,
            max_string_bytes: 10_000,
            max_allocation_bytes: 1_000_000,
            max_byte_work: 10_000,
            max_inheritance_depth: 2,
        };
        let mut budget = BoundedSchemaBudget::new(generous);
        let bounded = db.flatten_slots_bounded(id, &mut budget).unwrap();
        assert_eq!(bounded, legacy);
        let usage = budget.usage();

        for tightened in [
            BoundedSchemaLimits {
                max_work: usage.work - 1,
                ..generous
            },
            BoundedSchemaLimits {
                max_slots: usage.slots - 1,
                ..generous
            },
            BoundedSchemaLimits {
                max_string_bytes: usage.string_bytes - 1,
                ..generous
            },
            BoundedSchemaLimits {
                max_allocation_bytes: usage.allocation_bytes - 1,
                ..generous
            },
            BoundedSchemaLimits {
                max_byte_work: usage.byte_work - 1,
                ..generous
            },
            BoundedSchemaLimits {
                max_inheritance_depth: 1,
                ..generous
            },
        ] {
            let mut limited = BoundedSchemaBudget::new(tightened);
            assert!(matches!(
                db.flatten_slots_bounded(id, &mut limited),
                Err(BoundedSchemaError::ResourceLimit { .. })
            ));
        }

        let exact = BoundedSchemaLimits {
            max_work: usage.work,
            max_slots: usage.slots,
            max_string_bytes: usage.string_bytes,
            max_allocation_bytes: usage.allocation_bytes,
            max_byte_work: usage.byte_work,
            max_inheritance_depth: 2,
        };
        let mut exact_budget = BoundedSchemaBudget::new(exact);
        assert_eq!(
            db.flatten_slots_bounded(id, &mut exact_budget).unwrap(),
            legacy
        );
    }

    #[test]
    fn inheritance_chain_growth_is_debited_before_exact_reserve() {
        let charge = std::mem::size_of::<SchemaId>();
        let limits = |max_allocation_bytes| BoundedSchemaLimits {
            max_work: 0,
            max_slots: 0,
            max_string_bytes: 0,
            max_allocation_bytes,
            max_byte_work: 0,
            max_inheritance_depth: 1,
        };
        let mut chain = Vec::new();
        let mut short = BoundedSchemaBudget::new(limits(charge - 1));
        assert!(matches!(
            reserve_schema_chain_entry(&mut chain, &mut short),
            Err(BoundedSchemaError::ResourceLimit {
                resource: "schema allocations"
            })
        ));
        assert_eq!(chain.capacity(), 0);

        let mut exact = BoundedSchemaBudget::new(limits(charge));
        reserve_schema_chain_entry(&mut chain, &mut exact).unwrap();
        assert!(chain.capacity() >= 1);
        assert_eq!(exact.usage().allocation_bytes, charge);
    }

    #[test]
    fn bounded_flatten_reserves_one_final_output_for_deep_wide_inheritance() {
        const DEPTH: usize = 32;
        const WIDTH: usize = 8;
        let structs: Vec<_> = (0..DEPTH)
            .map(|index| usmap::Struct {
                name: format!("Level{index}"),
                super_struct: (index + 1 < DEPTH).then(|| format!("Level{}", index + 1)),
                properties: vec![usmap::Property {
                    name: format!("P{index}"),
                    array_dim: WIDTH as u8,
                    index: 0,
                    inner: usmap::PropertyInner::Int,
                }],
            })
            .collect();
        let db = SchemaDb::from_parsed(usmap::Usmap {
            enums: Vec::new(),
            ppth: Some(usmap::ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: vec!["/Script/Deep".into(); DEPTH],
            }),
            eatr: Some(usmap::ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags: vec![flags(usmap::FlagsType::Class); DEPTH],
            }),
            structs,
            cext: None,
            envp: None,
        })
        .unwrap();
        let id = db.resolve_class("Level0").unwrap();
        let generous = BoundedSchemaLimits {
            max_work: 1_000_000,
            max_slots: 1_000_000,
            max_string_bytes: 1_000_000,
            max_allocation_bytes: 64 * 1024 * 1024,
            max_byte_work: 1_000_000,
            max_inheritance_depth: DEPTH,
        };
        let mut measured = BoundedSchemaBudget::new(generous);
        let slots = db.flatten_slots_bounded(id, &mut measured).unwrap();
        assert_eq!(slots, db.flatten_slots(id).unwrap());
        assert_eq!(slots.len(), DEPTH * WIDTH);
        let usage = measured.usage();
        let minimum_slot_storage = slots.len()
            * (std::mem::size_of::<Option<PropertySlot>>() + std::mem::size_of::<PropertySlot>());
        assert!(usage.allocation_bytes >= minimum_slot_storage);
        assert!(usage.work >= slots.len() * 3 + (DEPTH * (DEPTH + 1)) / 2);

        let exact = BoundedSchemaLimits {
            max_work: usage.work,
            max_slots: usage.slots,
            max_string_bytes: usage.string_bytes,
            max_allocation_bytes: usage.allocation_bytes,
            max_byte_work: usage.byte_work,
            max_inheritance_depth: DEPTH,
        };
        let mut exact_budget = BoundedSchemaBudget::new(exact);
        assert_eq!(
            db.flatten_slots_bounded(id, &mut exact_budget).unwrap(),
            slots
        );
        for limited in [
            BoundedSchemaLimits {
                max_work: usage.work - 1,
                ..exact
            },
            BoundedSchemaLimits {
                max_allocation_bytes: usage.allocation_bytes - 1,
                ..exact
            },
        ] {
            let mut budget = BoundedSchemaBudget::new(limited);
            assert!(matches!(
                db.flatten_slots_bounded(id, &mut budget),
                Err(BoundedSchemaError::ResourceLimit { .. })
            ));
        }
    }

    #[test]
    fn compact_resolution_bounds_large_duplicate_sets_without_diagnostics() {
        let mut map = fixture();
        for index in 0..2_048 {
            map.structs.push(usmap::Struct {
                name: "Duplicate".into(),
                super_struct: None,
                properties: Vec::new(),
            });
            map.ppth
                .as_mut()
                .unwrap()
                .structs
                .push(format!("/Script/Duplicate{index}"));
            map.eatr
                .as_mut()
                .unwrap()
                .struct_flags
                .push(flags(usmap::FlagsType::Class));
        }
        let db = SchemaDb::from_parsed(map).unwrap();
        let mut limited = BoundedSchemaBudget::new(BoundedSchemaLimits {
            max_work: 100,
            max_slots: 0,
            max_string_bytes: 128,
            max_allocation_bytes: 128,
            max_byte_work: 128,
            max_inheritance_depth: 1,
        });
        assert!(matches!(
            db.resolve_class_compact_bounded("Duplicate", &mut limited),
            Err(BoundedSchemaError::ResourceLimit {
                resource: "schema work"
            })
        ));
        assert_eq!(limited.usage().work, 100);
        assert!(limited.usage().allocation_bytes < 128);
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
