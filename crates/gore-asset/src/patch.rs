//! Snapshot-bound, compare-and-swap edits for proven fixed-width property leaves.
//!
//! A patch is planned from an exact legacy export, its exact USMAP schema, and
//! a leaf returned by [`PropertySpanWalker`]. The owned plan seals both package
//! components, then reparses and rewalks the package immediately before and
//! after mutation. It never accepts a caller-supplied absolute offset.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use usmap::PropertyInner;

use crate::{
    schema::{BoundedSchemaBudget, BoundedSchemaError, BoundedSchemaLimits, BoundedSchemaUsage},
    EnvelopeError, ExportEnvelope, ExportSchemaError, FixedValueSpan, FixedWireKind,
    LegacyPackageEnvelope, PackageCarrier, PackageComponent, PackageError, PropertyBlockSpans,
    PropertySlot, PropertySpanWalker, SchemaDb, SchemaError, SchemaId, SliceSpan, SpanError,
    SpanLimits, SpanWalkResourceLimits, SpanWalkUsage, ValueSpan,
};

/// On-disk/API version of [`FixedLeafSelector`].
pub const FIXED_LEAF_SELECTOR_FORMAT: u32 = 1;

/// Exact cooked-property profile understood by [`FixedLeafSelector`].
pub const FIXED_LEAF_SELECTOR_PROFILE: &str = "g1r_ue5_4";

/// A listed fixed-width leaf plus whether [`FixedLeafPatch`] can edit its role
/// and wire kind. The selector remains authoritative; `editable` is advisory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedLeafDescriptor {
    pub selector: FixedLeafSelector,
    pub editable: bool,
}

/// Per-export bounds for the additive one-walk descriptor surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedLeafInspectionLimits {
    pub span_limits: SpanLimits,
    pub max_descriptors_per_export: usize,
    pub max_selector_steps_per_leaf: usize,
}

impl Default for FixedLeafInspectionLimits {
    fn default() -> Self {
        Self {
            span_limits: SpanLimits::default(),
            max_descriptors_per_export: 10_000,
            max_selector_steps_per_leaf: 128,
        }
    }
}

/// Cross-export resource limits consumed by [`FixedLeafInspectionSession`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedLeafWorkLimits {
    pub max_work: usize,
    pub max_nodes: usize,
    pub max_collection_elements: usize,
    pub max_leaves: usize,
    pub max_selector_steps: usize,
    pub max_selector_bytes: usize,
    pub max_schema_string_bytes: usize,
    pub max_allocation_bytes: usize,
    pub max_byte_work: usize,
    pub max_hash_bytes: usize,
}

impl Default for FixedLeafWorkLimits {
    fn default() -> Self {
        Self {
            max_work: 2_000_000,
            max_nodes: 1_000_000,
            max_collection_elements: 1_000_000,
            max_leaves: 20_000,
            max_selector_steps: 1_000_000,
            max_selector_bytes: 8 * 1024 * 1024,
            max_schema_string_bytes: 32 * 1024 * 1024,
            max_allocation_bytes: 128 * 1024 * 1024,
            max_byte_work: 256 * 1024 * 1024,
            max_hash_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Mutable global budget shared by every export in one inspection request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedLeafWorkBudget {
    remaining_work: usize,
    remaining_nodes: usize,
    remaining_collection_elements: usize,
    remaining_leaves: usize,
    remaining_selector_steps: usize,
    remaining_selector_bytes: usize,
    remaining_schema_string_bytes: usize,
    remaining_allocation_bytes: usize,
    remaining_byte_work: usize,
    remaining_hash_bytes: usize,
}

impl FixedLeafWorkBudget {
    pub fn new(limits: FixedLeafWorkLimits) -> Self {
        Self {
            remaining_work: limits.max_work,
            remaining_nodes: limits.max_nodes,
            remaining_collection_elements: limits.max_collection_elements,
            remaining_leaves: limits.max_leaves,
            remaining_selector_steps: limits.max_selector_steps,
            remaining_selector_bytes: limits.max_selector_bytes,
            remaining_schema_string_bytes: limits.max_schema_string_bytes,
            remaining_allocation_bytes: limits.max_allocation_bytes,
            remaining_byte_work: limits.max_byte_work,
            remaining_hash_bytes: limits.max_hash_bytes,
        }
    }

    pub fn remaining_nodes(&self) -> usize {
        self.remaining_nodes
    }

    pub fn remaining_work(&self) -> usize {
        self.remaining_work
    }

    pub fn remaining_collection_elements(&self) -> usize {
        self.remaining_collection_elements
    }

    pub fn remaining_leaves(&self) -> usize {
        self.remaining_leaves
    }

    pub fn remaining_selector_bytes(&self) -> usize {
        self.remaining_selector_bytes
    }

    pub fn remaining_schema_string_bytes(&self) -> usize {
        self.remaining_schema_string_bytes
    }

    pub fn remaining_allocation_bytes(&self) -> usize {
        self.remaining_allocation_bytes
    }

    pub fn remaining_byte_work(&self) -> usize {
        self.remaining_byte_work
    }

    pub fn remaining_hash_bytes(&self) -> usize {
        self.remaining_hash_bytes
    }

    /// Tighten (never expand) the request-global selector allocation budget.
    pub fn cap_selector_bytes(&mut self, maximum: usize) {
        self.remaining_selector_bytes = self.remaining_selector_bytes.min(maximum);
    }

    fn charge(
        remaining: &mut usize,
        amount: usize,
        resource: &'static str,
    ) -> Result<(), FixedLeafInspectionError> {
        if amount > *remaining {
            return Err(FixedLeafInspectionError::ResourceLimit { resource });
        }
        *remaining -= amount;
        Ok(())
    }

    fn work(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_work, amount, "work")
    }

    fn nodes(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_nodes, amount, "nodes")
    }

    fn collection(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(
            &mut self.remaining_collection_elements,
            amount,
            "collection elements",
        )
    }

    fn leaf(&mut self) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_leaves, 1, "leaves")
    }

    fn selector_steps(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_selector_steps, amount, "selector steps")
    }

    fn selector_bytes(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_selector_bytes, amount, "selector bytes")?;
        Self::charge(&mut self.remaining_allocation_bytes, amount, "allocations")
    }

    fn schema_strings(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(
            &mut self.remaining_schema_string_bytes,
            amount,
            "schema strings",
        )
    }

    fn allocation(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_allocation_bytes, amount, "allocations")
    }

    fn byte_work(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_byte_work, amount, "byte work")
    }

    fn hash_bytes(&mut self, amount: usize) -> Result<(), FixedLeafInspectionError> {
        Self::charge(&mut self.remaining_hash_bytes, amount, "hash bytes")
    }

    fn debit_schema_usage(
        &mut self,
        usage: BoundedSchemaUsage,
    ) -> Result<(), FixedLeafInspectionError> {
        self.work(usage.work)?;
        self.nodes(usage.slots)?;
        self.schema_strings(usage.string_bytes)?;
        self.allocation(usage.allocation_bytes)?;
        self.byte_work(usage.byte_work)
    }

    fn debit_span_usage(&mut self, usage: SpanWalkUsage) -> Result<(), FixedLeafInspectionError> {
        self.nodes(usage.nodes)?;
        self.collection(usage.collection_elements)?;
        self.work(usage.work)?;
        self.schema_strings(usage.string_bytes)?;
        self.allocation(usage.allocation_bytes)?;
        self.byte_work(usage.byte_work)
    }
}

impl Default for FixedLeafWorkBudget {
    fn default() -> Self {
        Self::new(FixedLeafWorkLimits::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedLeafInspectionCounters {
    pub package_seal_captures: usize,
    pub usmap_sha256_captures: usize,
    pub span_walks: usize,
    pub schema_resolution_scans: usize,
    pub schema_cache_hits: usize,
}

/// One bounded, prewalked export result. The returned descriptors were created
/// directly from this single span tree; no hidden rewalk occurs.
#[derive(Debug)]
pub struct FixedLeafInspection {
    schema_name: String,
    property_bytes: usize,
    native_suffix_bytes: usize,
    descriptors: Vec<FixedLeafDescriptor>,
}

impl FixedLeafInspection {
    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    pub fn property_bytes(&self) -> usize {
        self.property_bytes
    }

    pub fn native_suffix_bytes(&self) -> usize {
        self.native_suffix_bytes
    }

    pub fn descriptors(&self) -> &[FixedLeafDescriptor] {
        &self.descriptors
    }

    pub fn into_descriptors(self) -> Vec<FixedLeafDescriptor> {
        self.descriptors
    }

    pub fn into_parts(self) -> (String, usize, usize, Vec<FixedLeafDescriptor>) {
        (
            self.schema_name,
            self.property_bytes,
            self.native_suffix_bytes,
            self.descriptors,
        )
    }
}

#[derive(Debug, Error)]
pub enum FixedLeafInspectionError {
    #[error(transparent)]
    Selector(#[from] FixedLeafSelectorError),
    #[error("fixed-leaf inspection exhausted its global {resource} budget")]
    ResourceLimit { resource: &'static str },
    #[error("fixed-leaf inspection could not reserve bounded descriptor storage")]
    Allocation,
    #[error("fixed-leaf inspection could not bind the export class ({reason})")]
    SchemaUnsupported { reason: &'static str },
}

impl FixedLeafInspectionError {
    pub fn is_resource_limit(&self) -> bool {
        matches!(
            self,
            Self::ResourceLimit { .. }
                | Self::Allocation
                | Self::Selector(FixedLeafSelectorError::Span(
                    SpanError::CollectionLimit { .. }
                        | SpanError::CollectionAggregateLimit { .. }
                        | SpanError::DepthLimit { .. }
                        | SpanError::NodeLimit { .. }
                        | SpanError::Allocation { .. }
                        | SpanError::ResourceLimit { .. }
                ))
                | Self::Selector(FixedLeafSelectorError::Span(SpanError::Header {
                    source: crate::UnversionedError::ResourceLimit { .. }
                        | crate::UnversionedError::Allocation,
                    ..
                }))
        )
    }
}

/// Request-scoped inspector which captures the package pair and USMAP hash once.
pub struct FixedLeafInspectionSession<'a> {
    carrier: &'a PackageCarrier,
    schemas: &'a SchemaDb,
    package_seal: PackagePairSeal,
    usmap_sha256: String,
    span_walks: Cell<usize>,
    schema_resolution_scans: Cell<usize>,
    schema_cache_hits: Cell<usize>,
    schema_cache: RefCell<HashMap<String, CachedSchemaResolution>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CachedSchemaResolution {
    Resolved(SchemaId),
    Missing,
    Ambiguous,
    WrongKind,
    Unsupported,
}

impl<'a> FixedLeafInspectionSession<'a> {
    pub fn new(
        carrier: &'a PackageCarrier,
        schemas: &'a SchemaDb,
    ) -> Result<Self, FixedLeafInspectionError> {
        let usmap = schemas
            .source_sha256()
            .ok_or(FixedLeafSelectorError::MissingUsmapSource)?;
        Ok(Self {
            carrier,
            schemas,
            package_seal: PackagePairSeal::capture(carrier),
            usmap_sha256: encode_hex_fallible(&usmap)?,
            span_walks: Cell::new(0),
            schema_resolution_scans: Cell::new(0),
            schema_cache_hits: Cell::new(0),
            schema_cache: RefCell::new(HashMap::new()),
        })
    }

    pub fn package_seal(&self) -> &PackagePairSeal {
        &self.package_seal
    }

    pub fn usmap_sha256(&self) -> &str {
        &self.usmap_sha256
    }

    pub fn counters(&self) -> FixedLeafInspectionCounters {
        FixedLeafInspectionCounters {
            package_seal_captures: 1,
            usmap_sha256_captures: 1,
            span_walks: self.span_walks.get(),
            schema_resolution_scans: self.schema_resolution_scans.get(),
            schema_cache_hits: self.schema_cache_hits.get(),
        }
    }

    pub fn inspect_export_bounded<'bytes>(
        &self,
        export: &ExportEnvelope<'bytes>,
        limits: FixedLeafInspectionLimits,
        budget: &mut FixedLeafWorkBudget,
    ) -> Result<FixedLeafInspection, FixedLeafInspectionError> {
        let boundary = export.boundary();
        let carrier_export = self
            .carrier
            .slice(boundary.component(), boundary.offset(), boundary.length())
            .map_err(|_| FixedLeafSelectorError::ForeignExport)?;
        if !same_slice(carrier_export, export.bytes()) {
            return Err(FixedLeafSelectorError::ForeignExport.into());
        }
        if budget.remaining_nodes == 0 {
            return Err(FixedLeafInspectionError::ResourceLimit {
                resource: "span tree",
            });
        }

        let schema_id = self.resolve_schema_cached(boundary.class_path(), budget)?;
        let span_limits = SpanLimits {
            max_depth: limits.span_limits.max_depth,
            max_collection_elements: limits
                .span_limits
                .max_collection_elements
                .min(budget.remaining_collection_elements),
            max_total_nodes: limits
                .span_limits
                .max_total_nodes
                .min(budget.remaining_nodes),
        };
        self.span_walks.set(self.span_walks.get().saturating_add(1));
        let walked = PropertySpanWalker::g1r_ue5_4_with_limits(self.schemas, span_limits)
            .walk_bounded_accounted(
                export.bytes(),
                schema_id,
                budget.remaining_collection_elements,
                SpanWalkResourceLimits {
                    max_work: budget.remaining_work(),
                    max_string_bytes: budget.remaining_schema_string_bytes(),
                    max_allocation_bytes: budget.remaining_allocation_bytes(),
                    max_byte_work: budget.remaining_byte_work(),
                    max_inheritance_depth: 128,
                    max_header_fragments: 65_536,
                },
            );
        let block = match walked {
            Ok((block, usage)) => {
                budget.debit_span_usage(usage)?;
                block
            }
            Err(failure) => {
                budget.debit_span_usage(failure.usage())?;
                return Err(FixedLeafSelectorError::Span(failure.into_source()).into());
            }
        };
        let native_suffix_bytes = export.bytes().len().checked_sub(block.consumed()).ok_or(
            FixedLeafInspectionError::ResourceLimit {
                resource: "property range",
            },
        )?;
        budget.hash_bytes(export.bytes().len())?;
        budget.allocation(64)?;
        let export_sha256 = encode_hex_fallible(&sha256(export.bytes()))?;

        let mut collector = BoundedDescriptorCollector {
            limits,
            budget,
            package_seal: &self.package_seal,
            usmap_sha256: &self.usmap_sha256,
            boundary,
            export_sha256: &export_sha256,
            path: Vec::new(),
            descriptors: Vec::new(),
            identity_cache: HashMap::new(),
        };
        collector.collect_block(&block, FixedLeafRole::PropertyValue, None)?;
        collector
            .budget
            .selector_bytes(json_string_upper_bound(block.schema_name()))?;
        let schema_name = clone_string_fallible(block.schema_name())?;
        Ok(FixedLeafInspection {
            schema_name,
            property_bytes: block.consumed(),
            native_suffix_bytes,
            descriptors: collector.descriptors,
        })
    }

    fn resolve_schema_cached(
        &self,
        query: &str,
        budget: &mut FixedLeafWorkBudget,
    ) -> Result<SchemaId, FixedLeafInspectionError> {
        budget.work(1)?;
        budget.byte_work(query.len())?;
        if let Some(cached) = self.schema_cache.borrow().get(query).copied() {
            self.schema_cache_hits
                .set(self.schema_cache_hits.get().saturating_add(1));
            return cached.into_result();
        }

        self.schema_resolution_scans
            .set(self.schema_resolution_scans.get().saturating_add(1));
        let mut schema_budget = BoundedSchemaBudget::new(BoundedSchemaLimits {
            max_work: budget.remaining_work(),
            max_slots: 0,
            max_string_bytes: budget.remaining_schema_string_bytes(),
            max_allocation_bytes: budget.remaining_allocation_bytes(),
            max_byte_work: budget.remaining_byte_work(),
            max_inheritance_depth: 128,
        });
        let resolved = self
            .schemas
            .resolve_class_compact_bounded(query, &mut schema_budget);
        budget.debit_schema_usage(schema_budget.usage())?;
        let cached = match resolved {
            Ok(id) => CachedSchemaResolution::Resolved(id),
            Err(BoundedSchemaError::Missing) => CachedSchemaResolution::Missing,
            Err(BoundedSchemaError::Ambiguous) => CachedSchemaResolution::Ambiguous,
            Err(BoundedSchemaError::WrongKind) => CachedSchemaResolution::WrongKind,
            Err(BoundedSchemaError::InheritanceCycle | BoundedSchemaError::InvalidLayout) => {
                CachedSchemaResolution::Unsupported
            }
            Err(BoundedSchemaError::ResourceLimit { resource }) => {
                return Err(FixedLeafInspectionError::ResourceLimit { resource });
            }
            Err(BoundedSchemaError::Allocation) => {
                return Err(FixedLeafInspectionError::Allocation);
            }
        };

        budget.work(1)?;
        budget.byte_work(query.len())?;
        budget.schema_strings(query.len())?;
        // Key bytes and value size are explicit. The extra 128 bytes per
        // insertion exceeds the current hashbrown bucket/control slack and
        // amortized rehash peak for this request-bounded cache.
        budget.allocation(
            query
                .len()
                .saturating_add(std::mem::size_of::<CachedSchemaResolution>())
                .saturating_add(128),
        )?;
        let mut cache = self.schema_cache.borrow_mut();
        cache
            .try_reserve(1)
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
        cache.insert(clone_string_fallible(query)?, cached);
        cached.into_result()
    }
}

impl CachedSchemaResolution {
    fn into_result(self) -> Result<SchemaId, FixedLeafInspectionError> {
        match self {
            Self::Resolved(id) => Ok(id),
            Self::Missing => Err(FixedLeafInspectionError::SchemaUnsupported {
                reason: "missing schema",
            }),
            Self::Ambiguous => Err(FixedLeafInspectionError::SchemaUnsupported {
                reason: "ambiguous schema",
            }),
            Self::WrongKind => Err(FixedLeafInspectionError::SchemaUnsupported {
                reason: "wrong schema kind",
            }),
            Self::Unsupported => Err(FixedLeafInspectionError::SchemaUnsupported {
                reason: "unsupported schema metadata",
            }),
        }
    }
}

/// Snapshot-specific, offset-free identity for one walked fixed-width leaf.
///
/// The selector binds the exact export identity and bytes, exact class, exact
/// schema-derived path, role, wire kind, and observed bytes. It intentionally
/// carries no byte offset. [`Self::resolve`] performs its own authoritative
/// rewalk with the exact source-bound schema database before returning a leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedLeafSelector {
    pub format: u32,
    pub profile: String,
    pub package_seal: PackagePairSeal,
    /// Lowercase SHA-256 of the exact `.usmap` bytes used to walk the export.
    /// This is sourced from [`SchemaDb::source_sha256`], never from the caller.
    pub usmap_sha256: String,
    pub export_index: usize,
    pub object_name: String,
    pub class_path: String,
    pub component: PackageComponent,
    /// Lowercase SHA-256 of the complete export bytes.
    pub export_sha256: String,
    pub role: FixedLeafRole,
    pub kind: FixedWireKind,
    pub path: Vec<FixedLeafSelectorStep>,
    /// Canonical lowercase hex of the exact observed leaf bytes.
    pub expected_hex: String,
}

/// Stable identity of one map key. Entry indices are deliberately omitted:
/// a unique key may move, while duplicate equal keys are ambiguous and fail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedLeafMapKeyIdentity {
    /// Fixed root key kind, or `None` for a schema-recursive struct/map key.
    pub kind: Option<FixedWireKind>,
    pub byte_length: usize,
    /// Lowercase SHA-256 of the complete serialized key.
    pub sha256: String,
}

/// Format-1's stable, local wire representation of a USMAP property type.
///
/// This deliberately does not serialize `usmap::PropertyInner` directly: the
/// dependency's serde layout is not part of this crate's persistence contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum FixedLeafWireType {
    #[serde(rename = "byte")]
    Byte {},
    #[serde(rename = "bool")]
    Bool {},
    #[serde(rename = "int")]
    Int {},
    #[serde(rename = "float")]
    Float {},
    #[serde(rename = "object")]
    Object {},
    #[serde(rename = "name")]
    Name {},
    #[serde(rename = "delegate")]
    Delegate {},
    #[serde(rename = "double")]
    Double {},
    #[serde(rename = "array")]
    Array { inner: Box<FixedLeafWireType> },
    #[serde(rename = "struct")]
    Struct { name: String },
    #[serde(rename = "string")]
    String {},
    #[serde(rename = "text")]
    Text {},
    #[serde(rename = "interface")]
    Interface {},
    #[serde(rename = "multicast_delegate")]
    MulticastDelegate {},
    #[serde(rename = "weak_object")]
    WeakObject {},
    #[serde(rename = "lazy_object")]
    LazyObject {},
    #[serde(rename = "asset_object")]
    AssetObject {},
    #[serde(rename = "soft_object")]
    SoftObject {},
    #[serde(rename = "uint64")]
    UInt64 {},
    #[serde(rename = "uint32")]
    UInt32 {},
    #[serde(rename = "uint16")]
    UInt16 {},
    #[serde(rename = "int64")]
    Int64 {},
    #[serde(rename = "int16")]
    Int16 {},
    #[serde(rename = "int8")]
    Int8 {},
    #[serde(rename = "map")]
    Map {
        key: Box<FixedLeafWireType>,
        value: Box<FixedLeafWireType>,
    },
    #[serde(rename = "set")]
    Set { key: Box<FixedLeafWireType> },
    #[serde(rename = "enum")]
    Enum {
        inner: Box<FixedLeafWireType>,
        name: String,
    },
    #[serde(rename = "field_path")]
    FieldPath {},
    #[serde(rename = "optional")]
    Optional { inner: Box<FixedLeafWireType> },
    #[serde(rename = "utf8_string")]
    Utf8String {},
    #[serde(rename = "ansi_string")]
    AnsiString {},
    #[serde(rename = "unknown")]
    Unknown {},
}

impl From<&PropertyInner> for FixedLeafWireType {
    fn from(inner: &PropertyInner) -> Self {
        match inner {
            PropertyInner::Byte => Self::Byte {},
            PropertyInner::Bool => Self::Bool {},
            PropertyInner::Int => Self::Int {},
            PropertyInner::Float => Self::Float {},
            PropertyInner::Object => Self::Object {},
            PropertyInner::Name => Self::Name {},
            PropertyInner::Delegate => Self::Delegate {},
            PropertyInner::Double => Self::Double {},
            PropertyInner::Array { inner } => Self::Array {
                inner: Box::new(Self::from(inner.as_ref())),
            },
            PropertyInner::Struct { name } => Self::Struct { name: name.clone() },
            PropertyInner::Str => Self::String {},
            PropertyInner::Text => Self::Text {},
            PropertyInner::Interface => Self::Interface {},
            PropertyInner::MulticastDelegate => Self::MulticastDelegate {},
            PropertyInner::WeakObject => Self::WeakObject {},
            PropertyInner::LazyObject => Self::LazyObject {},
            PropertyInner::AssetObject => Self::AssetObject {},
            PropertyInner::SoftObject => Self::SoftObject {},
            PropertyInner::UInt64 => Self::UInt64 {},
            PropertyInner::UInt32 => Self::UInt32 {},
            PropertyInner::UInt16 => Self::UInt16 {},
            PropertyInner::Int64 => Self::Int64 {},
            PropertyInner::Int16 => Self::Int16 {},
            PropertyInner::Int8 => Self::Int8 {},
            PropertyInner::Map { key, value } => Self::Map {
                key: Box::new(Self::from(key.as_ref())),
                value: Box::new(Self::from(value.as_ref())),
            },
            PropertyInner::Set { key } => Self::Set {
                key: Box::new(Self::from(key.as_ref())),
            },
            PropertyInner::Enum { inner, name } => Self::Enum {
                inner: Box::new(Self::from(inner.as_ref())),
                name: name.clone(),
            },
            PropertyInner::FieldPath => Self::FieldPath {},
            PropertyInner::Optional { inner } => Self::Optional {
                inner: Box::new(Self::from(inner.as_ref())),
            },
            PropertyInner::Utf8Str => Self::Utf8String {},
            PropertyInner::AnsiStr => Self::AnsiString {},
            PropertyInner::Unknown => Self::Unknown {},
        }
    }
}

/// One canonical semantic step from an export's class schema to a fixed leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case", deny_unknown_fields)]
pub enum FixedLeafSelectorStep {
    Property {
        schema_index: usize,
        property_name: String,
        array_index: usize,
        array_dimension: usize,
        declaring_schema_name: String,
        declaring_module_path: Option<String>,
        property_type: FixedLeafWireType,
    },
    Struct {
        name: String,
        /// Exact qualified schema selected for the nested unversioned block.
        schema_name: String,
    },
    Map {
        key_type: FixedLeafWireType,
        value_type: FixedLeafWireType,
    },
    MapEntryValue {
        key: FixedLeafMapKeyIdentity,
    },
    MapEntryKey {
        key: FixedLeafMapKeyIdentity,
    },
    RemovedMapKey {
        key: FixedLeafMapKeyIdentity,
    },
}

/// Failure to describe or resolve an offset-free fixed-leaf selector.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FixedLeafSelectorError {
    #[error(
        "unsupported fixed-leaf selector format {actual}; expected {FIXED_LEAF_SELECTOR_FORMAT}"
    )]
    UnsupportedFormat { actual: u32 },
    #[error(
        "unsupported fixed-leaf selector profile {actual:?}; expected {FIXED_LEAF_SELECTOR_PROFILE:?}"
    )]
    UnsupportedProfile { actual: String },
    #[error("fixed-leaf selector path is empty")]
    EmptyPath,
    #[error("{field} is not canonical lowercase hexadecimal")]
    NonCanonicalHex { field: &'static str },
    #[error("{field} has {actual} hex characters; expected {expected}")]
    HexLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("the export envelope is not backed by the supplied package carrier")]
    ForeignExport,
    #[error("fixed-leaf package pair drifted: expected {expected}, got {actual}")]
    PackageDrift {
        expected: Box<PackagePairSeal>,
        actual: Box<PackagePairSeal>,
    },
    #[error("the supplied schema database has no exact raw `.usmap` source identity")]
    MissingUsmapSource,
    #[error("fixed-leaf USMAP drifted: expected {expected}, got {actual}")]
    UsmapDrift { expected: String, actual: String },
    #[error(
        "fixed-leaf export identity field {field} drifted: expected {expected:?}, got {actual:?}"
    )]
    ExportIdentityDrift {
        field: &'static str,
        expected: String,
        actual: String,
    },
    #[error("fixed-leaf export bytes drifted: expected {expected}, got {actual}")]
    ExportBytesDrift { expected: String, actual: String },
    #[error(transparent)]
    ExportSchema(#[from] ExportSchemaError),
    #[error(transparent)]
    Span(#[from] SpanError),
    #[error("fixed-leaf selector semantic path was not found")]
    NotFound,
    #[error("fixed-leaf selector semantic path matched {matches} leaves")]
    Ambiguous { matches: usize },
    #[error(
        "fixed-leaf selector descends through a map key identity occurring {occurrences} times"
    )]
    DuplicateMapKeyAncestry { occurrences: usize },
    #[error("fixed-leaf wire kind drifted: expected {expected:?}, got {actual:?}")]
    KindDrift {
        expected: FixedWireKind,
        actual: FixedWireKind,
    },
    #[error("fixed-leaf bytes drifted: expected {expected}, got {actual}")]
    ExpectedDrift { expected: String, actual: String },
}

impl FixedLeafDescriptor {
    pub fn selector(&self) -> &FixedLeafSelector {
        &self.selector
    }
}

impl FixedLeafSelector {
    /// Decode the canonical expected bytes carried by this selector.
    pub fn expected_bytes(&self) -> Result<Vec<u8>, FixedLeafSelectorError> {
        decode_canonical_hex("expected_hex", &self.expected_hex, self.kind.width())
    }

    /// Resolve this selector against a freshly parsed and walked export.
    ///
    /// No offset is accepted. Package and USMAP seals, export identity/hash,
    /// semantic path uniqueness, role, kind, and expected bytes must all match.
    pub fn resolve<'bytes>(
        &self,
        carrier: &PackageCarrier,
        export: &ExportEnvelope<'bytes>,
        schemas: &SchemaDb,
    ) -> Result<FixedValueSpan<'bytes>, FixedLeafSelectorError> {
        if self.format != FIXED_LEAF_SELECTOR_FORMAT {
            return Err(FixedLeafSelectorError::UnsupportedFormat {
                actual: self.format,
            });
        }
        if self.profile != FIXED_LEAF_SELECTOR_PROFILE {
            return Err(FixedLeafSelectorError::UnsupportedProfile {
                actual: self.profile.clone(),
            });
        }
        if self.path.is_empty() {
            return Err(FixedLeafSelectorError::EmptyPath);
        }
        let expected = self.expected_bytes()?;
        let sealed_usmap = decode_canonical_hex("usmap_sha256", &self.usmap_sha256, 32)?;
        let sealed_export = decode_canonical_hex("export_sha256", &self.export_sha256, 32)?;

        let actual_package = PackagePairSeal::capture(carrier);
        if actual_package != self.package_seal {
            return Err(FixedLeafSelectorError::PackageDrift {
                expected: Box::new(self.package_seal.clone()),
                actual: Box::new(actual_package),
            });
        }
        let actual_usmap = schemas
            .source_sha256()
            .ok_or(FixedLeafSelectorError::MissingUsmapSource)?;
        if sealed_usmap.as_slice() != actual_usmap.as_slice() {
            return Err(FixedLeafSelectorError::UsmapDrift {
                expected: self.usmap_sha256.clone(),
                actual: encode_hex(&actual_usmap),
            });
        }
        let boundary = export.boundary();
        let carrier_export = carrier
            .slice(boundary.component(), boundary.offset(), boundary.length())
            .map_err(|_| FixedLeafSelectorError::ForeignExport)?;
        if !same_slice(carrier_export, export.bytes()) {
            return Err(FixedLeafSelectorError::ForeignExport);
        }
        require_identity(
            "export_index",
            self.export_index.to_string(),
            boundary.export_index().to_string(),
        )?;
        require_identity(
            "object_name",
            self.object_name.clone(),
            boundary.object_name(),
        )?;
        require_identity("class_path", self.class_path.clone(), boundary.class_path())?;
        require_identity(
            "component",
            self.component.to_string(),
            boundary.component().to_string(),
        )?;

        let actual_export_sha = sha256(export.bytes());
        if sealed_export.as_slice() != actual_export_sha.as_slice() {
            return Err(FixedLeafSelectorError::ExportBytesDrift {
                expected: self.export_sha256.clone(),
                actual: encode_hex(&actual_export_sha),
            });
        }
        let schema_id = boundary.resolve_class_schema(schemas)?;
        let block = PropertySpanWalker::g1r_ue5_4(schemas).walk(export.bytes(), schema_id)?;

        let matches: Vec<_> = selector_candidates(&block)
            .into_iter()
            .filter(|candidate| candidate.role == self.role && candidate.path == self.path)
            .collect();
        let [candidate] = matches.as_slice() else {
            return if matches.is_empty() {
                Err(FixedLeafSelectorError::NotFound)
            } else {
                Err(FixedLeafSelectorError::Ambiguous {
                    matches: matches.len(),
                })
            };
        };
        if let Some(occurrences) = candidate.duplicate_key_occurrences {
            return Err(FixedLeafSelectorError::DuplicateMapKeyAncestry { occurrences });
        }
        if candidate.leaf.kind() != self.kind {
            return Err(FixedLeafSelectorError::KindDrift {
                expected: self.kind,
                actual: candidate.leaf.kind(),
            });
        }
        let actual = candidate.leaf.span().bytes();
        if actual != expected.as_slice() {
            return Err(FixedLeafSelectorError::ExpectedDrift {
                expected: self.expected_hex.clone(),
                actual: encode_hex(actual),
            });
        }
        Ok(candidate.leaf.clone())
    }
}

/// List every fixed-width leaf in a freshly walked export without exposing or
/// accepting offsets. Traversal order is deterministic for identical bytes.
pub fn describe_fixed_leaves<'bytes>(
    carrier: &PackageCarrier,
    export: &ExportEnvelope<'bytes>,
    schemas: &SchemaDb,
) -> Result<Vec<FixedLeafDescriptor>, FixedLeafSelectorError> {
    let boundary = export.boundary();
    let carrier_export = carrier
        .slice(boundary.component(), boundary.offset(), boundary.length())
        .map_err(|_| FixedLeafSelectorError::ForeignExport)?;
    if !same_slice(carrier_export, export.bytes()) {
        return Err(FixedLeafSelectorError::ForeignExport);
    }
    let usmap_sha256 = encode_hex(
        &schemas
            .source_sha256()
            .ok_or(FixedLeafSelectorError::MissingUsmapSource)?,
    );
    let schema_id = boundary.resolve_class_schema(schemas)?;
    let block = PropertySpanWalker::g1r_ue5_4(schemas).walk(export.bytes(), schema_id)?;
    let package_seal = PackagePairSeal::capture(carrier);
    let export_sha256 = encode_hex(&sha256(export.bytes()));
    Ok(selector_candidates(&block)
        .into_iter()
        .map(|candidate| {
            let editable = selector_candidate_is_editable(&candidate);
            FixedLeafDescriptor {
                selector: FixedLeafSelector {
                    format: FIXED_LEAF_SELECTOR_FORMAT,
                    profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
                    package_seal: package_seal.clone(),
                    usmap_sha256: usmap_sha256.clone(),
                    export_index: boundary.export_index(),
                    object_name: boundary.object_name().to_owned(),
                    class_path: boundary.class_path().to_owned(),
                    component: boundary.component(),
                    export_sha256: export_sha256.clone(),
                    role: candidate.role,
                    kind: candidate.leaf.kind(),
                    path: candidate.path,
                    expected_hex: encode_hex(candidate.leaf.span().bytes()),
                },
                editable,
            }
        })
        .collect())
}

struct BoundedDescriptorCollector<'a, 'budget> {
    limits: FixedLeafInspectionLimits,
    budget: &'budget mut FixedLeafWorkBudget,
    package_seal: &'a PackagePairSeal,
    usmap_sha256: &'a str,
    boundary: &'a crate::ExportBoundary,
    export_sha256: &'a str,
    path: Vec<FixedLeafSelectorStep>,
    descriptors: Vec<FixedLeafDescriptor>,
    identity_cache: HashMap<(usize, usize, Option<u8>), FixedLeafMapKeyIdentity>,
}

impl<'a, 'budget> BoundedDescriptorCollector<'a, 'budget> {
    fn prepare_step(&mut self) -> Result<(), FixedLeafInspectionError> {
        let attempted =
            self.path
                .len()
                .checked_add(1)
                .ok_or(FixedLeafInspectionError::ResourceLimit {
                    resource: "selector path depth",
                })?;
        if attempted > self.limits.max_selector_steps_per_leaf {
            return Err(FixedLeafInspectionError::ResourceLimit {
                resource: "selector path depth",
            });
        }
        self.budget.work(1)?;
        reserve_one_path_step(&mut self.path, self.budget)
    }

    fn collect_block(
        &mut self,
        block: &PropertyBlockSpans<'_>,
        role: FixedLeafRole,
        duplicate_key_occurrences: Option<usize>,
    ) -> Result<(), FixedLeafInspectionError> {
        self.budget.work(1)?;
        for property in block.properties() {
            self.budget.work(1)?;
            let Some(value) = property.value() else {
                continue;
            };
            self.prepare_step()?;
            let slot = property.slot();
            self.budget.selector_bytes(
                256usize
                    .saturating_add(json_string_upper_bound(&slot.property_name))
                    .saturating_add(json_string_upper_bound(&slot.declaring_schema_name))
                    .saturating_add(
                        slot.declaring_module_path
                            .as_deref()
                            .map(json_string_upper_bound)
                            .unwrap_or(4),
                    )
                    .saturating_add(property_inner_dynamic_upper_bound(&slot.inner)),
            )?;
            self.path.push(FixedLeafSelectorStep::Property {
                schema_index: slot.schema_index,
                property_name: clone_string_fallible(&slot.property_name)?,
                array_index: slot.array_index,
                array_dimension: slot.array_dimension,
                declaring_schema_name: clone_string_fallible(&slot.declaring_schema_name)?,
                declaring_module_path: slot
                    .declaring_module_path
                    .as_deref()
                    .map(clone_string_fallible)
                    .transpose()?,
                property_type: FixedLeafWireType::from(&slot.inner),
            });
            let result = self.collect_value(value, role, duplicate_key_occurrences);
            self.path.pop();
            result?;
        }
        Ok(())
    }

    fn collect_value(
        &mut self,
        value: &ValueSpan<'_>,
        role: FixedLeafRole,
        duplicate_key_occurrences: Option<usize>,
    ) -> Result<(), FixedLeafInspectionError> {
        self.budget.work(1)?;
        match value {
            ValueSpan::Fixed(leaf) => self.emit_leaf(leaf, role, duplicate_key_occurrences),
            ValueSpan::Struct(value) => {
                self.prepare_step()?;
                self.budget.selector_bytes(
                    256usize
                        .saturating_add(json_string_upper_bound(value.struct_name()))
                        .saturating_add(json_string_upper_bound(value.properties().schema_name())),
                )?;
                self.path.push(FixedLeafSelectorStep::Struct {
                    name: clone_string_fallible(value.struct_name())?,
                    schema_name: clone_string_fallible(value.properties().schema_name())?,
                });
                let result =
                    self.collect_block(value.properties(), role, duplicate_key_occurrences);
                self.path.pop();
                result
            }
            ValueSpan::Map(value) => {
                self.prepare_step()?;
                self.budget.selector_bytes(
                    256usize
                        .saturating_add(property_inner_dynamic_upper_bound(value.key_type()))
                        .saturating_add(property_inner_dynamic_upper_bound(value.value_type())),
                )?;
                self.path.push(FixedLeafSelectorStep::Map {
                    key_type: FixedLeafWireType::from(value.key_type()),
                    value_type: FixedLeafWireType::from(value.value_type()),
                });
                let result = self.collect_map(value, role, duplicate_key_occurrences);
                self.path.pop();
                result
            }
        }
    }

    fn collect_map(
        &mut self,
        value: &crate::MapValueSpan<'_>,
        role: FixedLeafRole,
        duplicate_key_occurrences: Option<usize>,
    ) -> Result<(), FixedLeafInspectionError> {
        let mut removed_identities = Vec::new();
        self.budget.work(value.removed_keys().len())?;
        self.budget.allocation(
            value
                .removed_keys()
                .len()
                .checked_mul(std::mem::size_of::<FixedLeafMapKeyIdentity>())
                .ok_or(FixedLeafInspectionError::ResourceLimit {
                    resource: "allocations",
                })?,
        )?;
        removed_identities
            .try_reserve_exact(value.removed_keys().len())
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
        for key in value.removed_keys() {
            removed_identities.push(self.map_key_identity(key)?);
        }
        let removed_counts = map_key_identity_counts_fallible(&removed_identities, self.budget)?;
        for (key, identity) in value.removed_keys().iter().zip(&removed_identities) {
            let occurrences = map_key_identity_count(&removed_counts, identity);
            let branch_duplicate =
                duplicate_key_occurrences.or_else(|| (occurrences > 1).then_some(occurrences));
            self.prepare_step()?;
            self.budget.selector_bytes(512)?;
            self.path.push(FixedLeafSelectorStep::RemovedMapKey {
                key: identity.clone(),
            });
            let result = self.collect_value(key, role.removed_key_child(), branch_duplicate);
            self.path.pop();
            result?;
        }

        let mut entry_identities = Vec::new();
        self.budget.work(value.entries().len())?;
        self.budget.allocation(
            value
                .entries()
                .len()
                .checked_mul(std::mem::size_of::<FixedLeafMapKeyIdentity>())
                .ok_or(FixedLeafInspectionError::ResourceLimit {
                    resource: "allocations",
                })?,
        )?;
        entry_identities
            .try_reserve_exact(value.entries().len())
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
        for entry in value.entries() {
            entry_identities.push(self.map_key_identity(entry.key())?);
        }
        let entry_counts = map_key_identity_counts_fallible(&entry_identities, self.budget)?;
        for (entry, identity) in value.entries().iter().zip(&entry_identities) {
            let occurrences = map_key_identity_count(&entry_counts, identity);
            let branch_duplicate =
                duplicate_key_occurrences.or_else(|| (occurrences > 1).then_some(occurrences));

            self.prepare_step()?;
            self.budget.selector_bytes(512)?;
            self.path.push(FixedLeafSelectorStep::MapEntryKey {
                key: identity.clone(),
            });
            let key_result =
                self.collect_value(entry.key(), role.live_key_child(), branch_duplicate);
            self.path.pop();
            key_result?;

            self.prepare_step()?;
            self.budget.selector_bytes(512)?;
            self.path.push(FixedLeafSelectorStep::MapEntryValue {
                key: identity.clone(),
            });
            let value_result = self.collect_value(entry.value(), role, branch_duplicate);
            self.path.pop();
            value_result?;
        }
        Ok(())
    }

    fn map_key_identity(
        &mut self,
        key: &ValueSpan<'_>,
    ) -> Result<FixedLeafMapKeyIdentity, FixedLeafInspectionError> {
        self.budget.work(1)?;
        self.budget.selector_bytes(64)?;
        let span = key.span();
        let kind = match key {
            ValueSpan::Fixed(fixed) => Some(fixed.kind()),
            ValueSpan::Struct(_) | ValueSpan::Map(_) => None,
        };
        let cache_key = (span.offset(), span.len(), kind.map(fixed_wire_kind_tag));
        if let Some(identity) = self.identity_cache.get(&cache_key) {
            self.budget.allocation(
                std::mem::size_of::<FixedLeafMapKeyIdentity>()
                    .saturating_add(identity.sha256.len()),
            )?;
            return Ok(identity.clone());
        }
        self.budget.hash_bytes(span.len())?;
        self.budget.allocation(64)?;
        let identity = FixedLeafMapKeyIdentity {
            kind,
            byte_length: span.len(),
            sha256: encode_hex_fallible(&sha256(span.bytes()))?,
        };
        self.budget.work(1)?;
        // The identity and its second SHA string are explicit; 128 bytes of
        // per-entry margin covers hashbrown bucket/control slack and rehash.
        self.budget.allocation(
            std::mem::size_of::<((usize, usize, Option<u8>), FixedLeafMapKeyIdentity)>()
                .saturating_add(identity.sha256.len())
                .saturating_add(128),
        )?;
        self.identity_cache
            .try_reserve(1)
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
        self.identity_cache.insert(cache_key, identity.clone());
        Ok(identity)
    }

    fn emit_leaf(
        &mut self,
        leaf: &FixedValueSpan<'_>,
        role: FixedLeafRole,
        duplicate_key_occurrences: Option<usize>,
    ) -> Result<(), FixedLeafInspectionError> {
        if self.descriptors.len() >= self.limits.max_descriptors_per_export {
            return Err(FixedLeafInspectionError::ResourceLimit {
                resource: "descriptors per export",
            });
        }
        self.budget.leaf()?;
        self.budget.selector_steps(self.path.len())?;
        self.budget.work(self.path.len().saturating_add(1))?;
        let selector_bytes = selector_json_upper_bound(
            self.boundary.object_name(),
            self.boundary.class_path(),
            self.usmap_sha256,
            self.export_sha256,
            leaf,
            &self.path,
        );
        // selector_bytes is also charged to the allocation budget. Its fixed
        // 1KiB+ base plus every dynamic string/path byte dominates Descriptor
        // Vec slack, the exact owned-path reserve, and all selector clones.
        self.budget.selector_bytes(selector_bytes)?;
        self.descriptors
            .try_reserve(1)
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
        let mut owned_path = Vec::new();
        owned_path
            .try_reserve_exact(self.path.len())
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
        owned_path.extend(self.path.iter().cloned());
        let editable = duplicate_key_occurrences.is_none()
            && role == FixedLeafRole::PropertyValue
            && fixed_wire_kind_is_editable(leaf.kind())
            && !self.path.iter().any(|step| {
                matches!(
                    step,
                    FixedLeafSelectorStep::MapEntryValue {
                        key: FixedLeafMapKeyIdentity { kind: None, .. }
                    }
                )
            });
        self.descriptors.push(FixedLeafDescriptor {
            selector: FixedLeafSelector {
                format: FIXED_LEAF_SELECTOR_FORMAT,
                profile: clone_string_fallible(FIXED_LEAF_SELECTOR_PROFILE)?,
                package_seal: self.package_seal.clone(),
                usmap_sha256: clone_string_fallible(self.usmap_sha256)?,
                export_index: self.boundary.export_index(),
                object_name: clone_string_fallible(self.boundary.object_name())?,
                class_path: clone_string_fallible(self.boundary.class_path())?,
                component: self.boundary.component(),
                export_sha256: clone_string_fallible(self.export_sha256)?,
                role,
                kind: leaf.kind(),
                path: owned_path,
                expected_hex: encode_hex_fallible(leaf.span().bytes())?,
            },
            editable,
        });
        Ok(())
    }
}

fn reserve_one_path_step(
    path: &mut Vec<FixedLeafSelectorStep>,
    budget: &mut FixedLeafWorkBudget,
) -> Result<(), FixedLeafInspectionError> {
    if path.len() == path.capacity() {
        budget.allocation(
            std::mem::size_of::<FixedLeafSelectorStep>()
                .checked_add(64)
                .ok_or(FixedLeafInspectionError::ResourceLimit {
                    resource: "allocations",
                })?,
        )?;
        // Exact growth keeps the precharge proportional instead of relying on
        // Vec's geometric reserve policy.
        path.try_reserve_exact(1)
            .map_err(|_| FixedLeafInspectionError::Allocation)?;
    }
    Ok(())
}

fn map_key_identity_counts_fallible<'a>(
    identities: &'a [FixedLeafMapKeyIdentity],
    budget: &mut FixedLeafWorkBudget,
) -> Result<MapKeyIdentityCounts<'a>, FixedLeafInspectionError> {
    budget.work(identities.len())?;
    budget.byte_work(identities.len().saturating_mul(64))?;
    // A borrowed identity key is currently far below 160 bytes including
    // hashbrown controls/slack; charging 160 per item also covers rehash peak.
    budget.allocation(identities.len().checked_mul(160).ok_or(
        FixedLeafInspectionError::ResourceLimit {
            resource: "allocations",
        },
    )?)?;
    let mut counts = HashMap::new();
    counts
        .try_reserve(identities.len())
        .map_err(|_| FixedLeafInspectionError::Allocation)?;
    for identity in identities {
        *counts
            .entry((
                identity.kind.map(fixed_wire_kind_tag),
                identity.byte_length,
                identity.sha256.as_str(),
            ))
            .or_insert(0) += 1;
    }
    Ok(counts)
}

fn clone_string_fallible(value: &str) -> Result<String, FixedLeafInspectionError> {
    let mut out = String::new();
    out.try_reserve_exact(value.len())
        .map_err(|_| FixedLeafInspectionError::Allocation)?;
    out.push_str(value);
    Ok(out)
}

fn encode_hex_fallible(bytes: &[u8]) -> Result<String, FixedLeafInspectionError> {
    let capacity = bytes
        .len()
        .checked_mul(2)
        .ok_or(FixedLeafInspectionError::ResourceLimit {
            resource: "hex bytes",
        })?;
    let mut out = String::new();
    out.try_reserve_exact(capacity)
        .map_err(|_| FixedLeafInspectionError::Allocation)?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    Ok(out)
}

fn selector_json_upper_bound(
    object_name: &str,
    class_path: &str,
    usmap_sha256: &str,
    export_sha256: &str,
    leaf: &FixedValueSpan<'_>,
    path: &[FixedLeafSelectorStep],
) -> usize {
    let mut bytes = 1_024usize
        .saturating_add(json_string_upper_bound(object_name))
        .saturating_add(json_string_upper_bound(class_path))
        .saturating_add(json_string_upper_bound(usmap_sha256))
        .saturating_add(json_string_upper_bound(export_sha256))
        .saturating_add(leaf.span().len().saturating_mul(2));
    for step in path {
        bytes = bytes
            .saturating_add(256)
            .saturating_add(selector_step_dynamic_upper_bound(step));
    }
    bytes
}

fn json_string_upper_bound(value: &str) -> usize {
    value.len().saturating_mul(6).saturating_add(2)
}

fn selector_step_dynamic_upper_bound(step: &FixedLeafSelectorStep) -> usize {
    match step {
        FixedLeafSelectorStep::Property {
            property_name,
            declaring_schema_name,
            declaring_module_path,
            property_type,
            ..
        } => json_string_upper_bound(property_name)
            .saturating_add(json_string_upper_bound(declaring_schema_name))
            .saturating_add(
                declaring_module_path
                    .as_deref()
                    .map(json_string_upper_bound)
                    .unwrap_or(4),
            )
            .saturating_add(wire_type_dynamic_upper_bound(property_type)),
        FixedLeafSelectorStep::Struct { name, schema_name } => {
            json_string_upper_bound(name).saturating_add(json_string_upper_bound(schema_name))
        }
        FixedLeafSelectorStep::Map {
            key_type,
            value_type,
        } => wire_type_dynamic_upper_bound(key_type)
            .saturating_add(wire_type_dynamic_upper_bound(value_type)),
        FixedLeafSelectorStep::MapEntryValue { key }
        | FixedLeafSelectorStep::MapEntryKey { key }
        | FixedLeafSelectorStep::RemovedMapKey { key } => {
            json_string_upper_bound(&key.sha256).saturating_add(128)
        }
    }
}

fn wire_type_dynamic_upper_bound(wire: &FixedLeafWireType) -> usize {
    match wire {
        FixedLeafWireType::Array { inner }
        | FixedLeafWireType::Set { key: inner }
        | FixedLeafWireType::Optional { inner } => {
            64usize.saturating_add(wire_type_dynamic_upper_bound(inner))
        }
        FixedLeafWireType::Enum { inner, name } => 96usize
            .saturating_add(json_string_upper_bound(name))
            .saturating_add(wire_type_dynamic_upper_bound(inner)),
        FixedLeafWireType::Map { key, value } => 96usize
            .saturating_add(wire_type_dynamic_upper_bound(key))
            .saturating_add(wire_type_dynamic_upper_bound(value)),
        FixedLeafWireType::Struct { name } => 64usize.saturating_add(json_string_upper_bound(name)),
        _ => 64,
    }
}

fn property_inner_dynamic_upper_bound(inner: &PropertyInner) -> usize {
    match inner {
        PropertyInner::Array { inner }
        | PropertyInner::Set { key: inner }
        | PropertyInner::Optional { inner } => {
            64usize.saturating_add(property_inner_dynamic_upper_bound(inner))
        }
        PropertyInner::Enum { inner, name } => 96usize
            .saturating_add(json_string_upper_bound(name))
            .saturating_add(property_inner_dynamic_upper_bound(inner)),
        PropertyInner::Map { key, value } => 96usize
            .saturating_add(property_inner_dynamic_upper_bound(key))
            .saturating_add(property_inner_dynamic_upper_bound(value)),
        PropertyInner::Struct { name } => 64usize.saturating_add(json_string_upper_bound(name)),
        _ => 64,
    }
}

fn require_identity(
    field: &'static str,
    expected: String,
    actual: impl Into<String>,
) -> Result<(), FixedLeafSelectorError> {
    let actual = actual.into();
    if expected == actual {
        Ok(())
    } else {
        Err(FixedLeafSelectorError::ExportIdentityDrift {
            field,
            expected,
            actual,
        })
    }
}

#[derive(Debug, Clone)]
struct SelectorCandidate<'a> {
    leaf: FixedValueSpan<'a>,
    role: FixedLeafRole,
    path: Vec<FixedLeafSelectorStep>,
    duplicate_key_occurrences: Option<usize>,
}

fn selector_candidates<'a>(block: &PropertyBlockSpans<'a>) -> Vec<SelectorCandidate<'a>> {
    let mut path = Vec::new();
    let mut candidates = Vec::new();
    collect_selector_block(
        block,
        FixedLeafRole::PropertyValue,
        None,
        &mut path,
        &mut candidates,
    );
    candidates
}

fn collect_selector_block<'a>(
    block: &PropertyBlockSpans<'a>,
    role: FixedLeafRole,
    duplicate_key_occurrences: Option<usize>,
    path: &mut Vec<FixedLeafSelectorStep>,
    candidates: &mut Vec<SelectorCandidate<'a>>,
) {
    for property in block.properties() {
        let Some(value) = property.value() else {
            continue;
        };
        let slot = property.slot();
        path.push(FixedLeafSelectorStep::Property {
            schema_index: slot.schema_index,
            property_name: slot.property_name.clone(),
            array_index: slot.array_index,
            array_dimension: slot.array_dimension,
            declaring_schema_name: slot.declaring_schema_name.clone(),
            declaring_module_path: slot.declaring_module_path.clone(),
            property_type: FixedLeafWireType::from(&slot.inner),
        });
        collect_selector_value(value, role, duplicate_key_occurrences, path, candidates);
        path.pop();
    }
}

fn collect_selector_value<'a>(
    value: &ValueSpan<'a>,
    role: FixedLeafRole,
    duplicate_key_occurrences: Option<usize>,
    path: &mut Vec<FixedLeafSelectorStep>,
    candidates: &mut Vec<SelectorCandidate<'a>>,
) {
    match value {
        ValueSpan::Fixed(leaf) => candidates.push(SelectorCandidate {
            leaf: leaf.clone(),
            role,
            path: path.clone(),
            duplicate_key_occurrences,
        }),
        ValueSpan::Struct(value) => {
            path.push(FixedLeafSelectorStep::Struct {
                name: value.struct_name().to_owned(),
                schema_name: value.properties().schema_name().to_owned(),
            });
            collect_selector_block(
                value.properties(),
                role,
                duplicate_key_occurrences,
                path,
                candidates,
            );
            path.pop();
        }
        ValueSpan::Map(value) => {
            path.push(FixedLeafSelectorStep::Map {
                key_type: FixedLeafWireType::from(value.key_type()),
                value_type: FixedLeafWireType::from(value.value_type()),
            });
            let removed_identities: Vec<_> =
                value.removed_keys().iter().map(map_key_identity).collect();
            let removed_counts = map_key_identity_counts(&removed_identities);
            for (key, identity) in value.removed_keys().iter().zip(&removed_identities) {
                let occurrences = map_key_identity_count(&removed_counts, identity);
                let branch_duplicate =
                    duplicate_key_occurrences.or_else(|| (occurrences > 1).then_some(occurrences));
                path.push(FixedLeafSelectorStep::RemovedMapKey {
                    key: identity.clone(),
                });
                collect_selector_value(
                    key,
                    role.removed_key_child(),
                    branch_duplicate,
                    path,
                    candidates,
                );
                path.pop();
            }
            let entry_identities: Vec<_> = value
                .entries()
                .iter()
                .map(|entry| map_key_identity(entry.key()))
                .collect();
            let entry_counts = map_key_identity_counts(&entry_identities);
            for (entry, identity) in value.entries().iter().zip(&entry_identities) {
                let occurrences = map_key_identity_count(&entry_counts, identity);
                let branch_duplicate =
                    duplicate_key_occurrences.or_else(|| (occurrences > 1).then_some(occurrences));
                path.push(FixedLeafSelectorStep::MapEntryKey {
                    key: identity.clone(),
                });
                collect_selector_value(
                    entry.key(),
                    role.live_key_child(),
                    branch_duplicate,
                    path,
                    candidates,
                );
                path.pop();

                path.push(FixedLeafSelectorStep::MapEntryValue {
                    key: identity.clone(),
                });
                collect_selector_value(entry.value(), role, branch_duplicate, path, candidates);
                path.pop();
            }
            path.pop();
        }
    }
}

fn map_key_identity(key: &ValueSpan<'_>) -> FixedLeafMapKeyIdentity {
    let span = key.span();
    FixedLeafMapKeyIdentity {
        kind: match key {
            ValueSpan::Fixed(fixed) => Some(fixed.kind()),
            ValueSpan::Struct(_) | ValueSpan::Map(_) => None,
        },
        byte_length: span.len(),
        sha256: encode_hex(&sha256(span.bytes())),
    }
}

type MapKeyIdentityCounts<'a> = HashMap<(Option<u8>, usize, &'a str), usize>;

fn map_key_identity_counts(identities: &[FixedLeafMapKeyIdentity]) -> MapKeyIdentityCounts<'_> {
    let mut counts = HashMap::with_capacity(identities.len());
    for identity in identities {
        *counts
            .entry((
                identity.kind.map(fixed_wire_kind_tag),
                identity.byte_length,
                identity.sha256.as_str(),
            ))
            .or_insert(0) += 1;
    }
    counts
}

fn map_key_identity_count(
    counts: &MapKeyIdentityCounts<'_>,
    identity: &FixedLeafMapKeyIdentity,
) -> usize {
    counts
        .get(&(
            identity.kind.map(fixed_wire_kind_tag),
            identity.byte_length,
            identity.sha256.as_str(),
        ))
        .copied()
        .unwrap_or(0)
}

fn fixed_wire_kind_tag(kind: FixedWireKind) -> u8 {
    match kind {
        FixedWireKind::Byte => 0,
        FixedWireKind::Bool => 1,
        FixedWireKind::Int32 => 2,
        FixedWireKind::Float32 => 3,
        FixedWireKind::PackageIndex => 4,
        FixedWireKind::FName => 5,
        FixedWireKind::Float64 => 6,
        FixedWireKind::UInt64 => 7,
        FixedWireKind::UInt32 => 8,
        FixedWireKind::UInt16 => 9,
        FixedWireKind::Int64 => 10,
        FixedWireKind::Int16 => 11,
        FixedWireKind::Int8 => 12,
        FixedWireKind::LinearColorF32x4 => 13,
        FixedWireKind::Vector4F64x4 => 14,
    }
}

/// The single exhaustive policy decision for editable fixed wire kinds.
/// Adding a new kind fails compilation here until its edit safety is decided.
fn fixed_wire_kind_is_editable(kind: FixedWireKind) -> bool {
    match kind {
        FixedWireKind::Byte
        | FixedWireKind::Bool
        | FixedWireKind::Int32
        | FixedWireKind::Float32
        | FixedWireKind::Float64
        | FixedWireKind::UInt64
        | FixedWireKind::UInt32
        | FixedWireKind::UInt16
        | FixedWireKind::Int64
        | FixedWireKind::Int16
        | FixedWireKind::Int8
        | FixedWireKind::LinearColorF32x4
        | FixedWireKind::Vector4F64x4 => true,
        FixedWireKind::PackageIndex | FixedWireKind::FName => false,
    }
}

fn selector_candidate_is_editable(candidate: &SelectorCandidate<'_>) -> bool {
    candidate.duplicate_key_occurrences.is_none()
        && candidate.role == FixedLeafRole::PropertyValue
        && fixed_wire_kind_is_editable(candidate.leaf.kind())
        && !candidate.path.iter().any(|step| {
            matches!(
                step,
                FixedLeafSelectorStep::MapEntryValue {
                    key: FixedLeafMapKeyIdentity { kind: None, .. }
                }
            )
        })
}

fn decode_canonical_hex(
    field: &'static str,
    value: &str,
    expected_bytes: usize,
) -> Result<Vec<u8>, FixedLeafSelectorError> {
    let expected_chars = expected_bytes * 2;
    if value.len() != expected_chars {
        return Err(FixedLeafSelectorError::HexLength {
            field,
            expected: expected_chars,
            actual: value.len(),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FixedLeafSelectorError::NonCanonicalHex { field });
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("canonical hex was validated before decoding"),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

mod hex_32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&super::encode_hex(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        let value = String::deserialize(deserializer)?;
        let decoded =
            super::decode_canonical_hex("package seal", &value, 32).map_err(D::Error::custom)?;
        decoded
            .try_into()
            .map_err(|_| D::Error::custom("package seal must contain exactly 32 bytes"))
    }
}

/// SHA-256 identity of both components at one point in time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackagePairSeal {
    #[serde(with = "hex_32")]
    pub uasset_sha256: [u8; 32],
    #[serde(with = "hex_32")]
    pub uexp_sha256: [u8; 32],
}

impl PackagePairSeal {
    pub fn capture(carrier: &PackageCarrier) -> Self {
        Self {
            uasset_sha256: sha256(carrier.bytes(PackageComponent::Uasset)),
            uexp_sha256: sha256(carrier.bytes(PackageComponent::Uexp)),
        }
    }
}

impl fmt::Display for PackagePairSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("uasset=")?;
        write_hex(formatter, &self.uasset_sha256)?;
        formatter.write_str(",uexp=")?;
        write_hex(formatter, &self.uexp_sha256)
    }
}

/// One owned, snapshot-specific fixed-leaf replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedLeafPatch {
    before: PackagePairSeal,
    export_index: usize,
    object_name: String,
    class_path: String,
    component: PackageComponent,
    export_offset: usize,
    export_length: usize,
    schema_slots: Vec<PropertySlot>,
    block_consumed: usize,
    header_length: usize,
    leaf_path: Vec<FixedLeafPathStep>,
    leaf_offset: usize,
    absolute_offset: usize,
    kind: FixedWireKind,
    expected: Vec<u8>,
    replacement: Vec<u8>,
}

impl FixedLeafPatch {
    /// Plan an edit from one exact span-walker leaf.
    ///
    /// `block` and `leaf` must be borrowed from `export.bytes()` and `export`
    /// must in turn be borrowed from `carrier`. The plan owns all evidence, so
    /// those borrows can be dropped before [`Self::apply`] takes `&mut carrier`.
    /// Numeric, canonical Bool, `LinearColor`, and `Vector4` leaves are the
    /// editable subset. `FName` and package-index references, live or removed
    /// map keys, and values behind a schema-recursive struct/map key are
    /// refused. Expected and replacement bytes must differ and retain the exact
    /// fixed width.
    ///
    /// A plan is snapshot-specific: it seals the complete package pair and is
    /// normally single-use because a successful edit changes that seal.
    pub fn plan(
        carrier: &PackageCarrier,
        export: &ExportEnvelope<'_>,
        schemas: &SchemaDb,
        block: &PropertyBlockSpans<'_>,
        leaf: &FixedValueSpan<'_>,
        expected: &[u8],
        replacement: &[u8],
    ) -> Result<Self, FixedLeafPatchError> {
        let boundary = export.boundary();
        let carrier_export =
            carrier.slice(boundary.component(), boundary.offset(), boundary.length())?;
        if !same_slice(carrier_export, export.bytes()) {
            return Err(FixedLeafPatchError::ForeignExport);
        }

        let block_span = block.span();
        if block_span.offset() != 0
            || block_span.end() > export.bytes().len()
            || block_span.len() != block.consumed()
            || !same_prefix(block_span.bytes(), export.bytes())
        {
            return Err(FixedLeafPatchError::ForeignPropertyBlock);
        }

        let leaf_span = leaf.span();
        let supplied_matches = matching_fixed_leaves(block, leaf_span, leaf.kind());
        if supplied_matches.len() != 1
            || !same_slice(supplied_matches[0].span.bytes(), leaf_span.bytes())
            || leaf_span.end() > block_span.end()
            || !slice_at(export.bytes(), leaf_span.offset(), leaf_span.bytes())
        {
            return Err(FixedLeafPatchError::ForeignFixedLeaf {
                matching_leaves: supplied_matches.len(),
            });
        }

        // SchemaId is local to one SchemaDb. Rewalk with the supplied database
        // and require the caller's borrowed tree to describe the same semantic
        // property path, not merely the same numeric id and byte offset.
        let schema_id = boundary.resolve_class_schema(schemas)?;
        let authoritative =
            PropertySpanWalker::g1r_ue5_4(schemas).walk(export.bytes(), schema_id)?;
        let authoritative_matches = matching_fixed_leaves(&authoritative, leaf_span, leaf.kind());
        if authoritative_matches.len() != 1
            || supplied_matches[0].path != authoritative_matches[0].path
            || supplied_matches[0].role != authoritative_matches[0].role
            || block.schema_id() != authoritative.schema_id()
            || block.schema_name() != authoritative.schema_name()
            || block.consumed() != authoritative.consumed()
            || block.header().bytes() != authoritative.header().bytes()
        {
            return Err(FixedLeafPatchError::SemanticPathMismatch);
        }
        let authoritative_leaf = &authoritative_matches[0];
        if authoritative_leaf.role != FixedLeafRole::PropertyValue {
            return Err(FixedLeafPatchError::MapKeyEditUnsupported {
                section: authoritative_leaf.role.description(),
            });
        }
        if authoritative_leaf.path.iter().any(|step| {
            matches!(
                step,
                FixedLeafPathStep::MapEntryValue { key_kind: None, .. }
            )
        }) {
            return Err(FixedLeafPatchError::ComplexMapKeyUnsupported);
        }

        let schema_slots = schemas.flatten_slots(schema_id)?;

        validate_fixed_replacement(
            leaf.kind(),
            authoritative_leaf.span.bytes(),
            expected,
            replacement,
        )?;
        if expected == replacement {
            return Err(FixedLeafPatchError::NoChange);
        }

        let absolute_offset = boundary
            .offset()
            .checked_add(authoritative_leaf.span.offset())
            .ok_or(FixedLeafPatchError::RangeArithmetic)?;

        Ok(Self {
            before: PackagePairSeal::capture(carrier),
            export_index: boundary.export_index(),
            object_name: boundary.object_name().to_owned(),
            class_path: boundary.class_path().to_owned(),
            component: boundary.component(),
            export_offset: boundary.offset(),
            export_length: boundary.length(),
            schema_slots,
            block_consumed: authoritative.consumed(),
            header_length: authoritative.header().len(),
            leaf_path: authoritative_leaf.path.clone(),
            leaf_offset: authoritative_leaf.span.offset(),
            absolute_offset,
            kind: leaf.kind(),
            expected: expected.to_vec(),
            replacement: replacement.to_vec(),
        })
    }

    pub fn before(&self) -> &PackagePairSeal {
        &self.before
    }

    pub fn export_index(&self) -> usize {
        self.export_index
    }

    pub fn object_name(&self) -> &str {
        &self.object_name
    }

    pub fn class_path(&self) -> &str {
        &self.class_path
    }

    pub fn component(&self) -> PackageComponent {
        self.component
    }

    pub fn absolute_offset(&self) -> usize {
        self.absolute_offset
    }

    pub fn kind(&self) -> FixedWireKind {
        self.kind
    }

    pub fn expected(&self) -> &[u8] {
        &self.expected
    }

    pub fn replacement(&self) -> &[u8] {
        &self.replacement
    }

    /// Revalidate, apply one equal-length CAS, and revalidate the resulting
    /// structure. Any unexpected postcondition failure is rolled back with a
    /// second CAS and the original pair seal is verified.
    pub fn apply(
        &self,
        carrier: &mut PackageCarrier,
        schemas: &SchemaDb,
    ) -> Result<FixedLeafPatchReceipt, FixedLeafPatchError> {
        let actual_before = PackagePairSeal::capture(carrier);
        if actual_before != self.before {
            return Err(FixedLeafPatchError::PackageDrift {
                expected: Box::new(self.before.clone()),
                actual: Box::new(actual_before),
            });
        }

        self.validate_layout(carrier, schemas, &self.expected)?;
        carrier.replace_range_if_equal(
            self.component,
            self.absolute_offset,
            &self.expected,
            &self.replacement,
        )?;

        if let Err(postcondition) = self.validate_layout(carrier, schemas, &self.replacement) {
            let postcondition = postcondition.to_string();
            if let Err(rollback) = carrier.replace_range_if_equal(
                self.component,
                self.absolute_offset,
                &self.replacement,
                &self.expected,
            ) {
                return Err(FixedLeafPatchError::RollbackFailed {
                    postcondition,
                    rollback,
                });
            }
            let restored = PackagePairSeal::capture(carrier);
            if restored != self.before {
                return Err(FixedLeafPatchError::RollbackVerification {
                    postcondition,
                    expected: Box::new(self.before.clone()),
                    actual: Box::new(restored),
                });
            }
            return Err(FixedLeafPatchError::Postcondition { postcondition });
        }

        Ok(FixedLeafPatchReceipt {
            before: self.before.clone(),
            after: PackagePairSeal::capture(carrier),
            export_index: self.export_index,
            component: self.component,
            absolute_offset: self.absolute_offset,
            length: self.replacement.len(),
            kind: self.kind,
        })
    }

    fn validate_layout(
        &self,
        carrier: &PackageCarrier,
        schemas: &SchemaDb,
        target_bytes: &[u8],
    ) -> Result<(), FixedLeafPatchError> {
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(carrier)?;
        let boundary = package.exports().get(self.export_index).ok_or_else(|| {
            FixedLeafPatchError::LayoutDrift {
                reason: format!("export {} no longer exists", self.export_index),
            }
        })?;
        if boundary.object_name() != self.object_name
            || boundary.class_path() != self.class_path
            || boundary.component() != self.component
            || boundary.offset() != self.export_offset
            || boundary.length() != self.export_length
        {
            return Err(FixedLeafPatchError::LayoutDrift {
                reason: format!("export {} identity or boundary changed", self.export_index),
            });
        }

        let schema_id = boundary.resolve_class_schema(schemas)?;
        let current_slots = schemas.flatten_slots(schema_id)?;
        if current_slots != self.schema_slots {
            return Err(FixedLeafPatchError::SchemaLayoutDrift {
                class_path: self.class_path.clone(),
            });
        }

        let export = package.export(self.export_index)?;
        let block = PropertySpanWalker::g1r_ue5_4(schemas).walk(export.bytes(), schema_id)?;
        if block.consumed() != self.block_consumed
            || block.header().len() != self.header_length
            || block.span().offset() != 0
            || !same_prefix(block.span().bytes(), export.bytes())
        {
            return Err(FixedLeafPatchError::LayoutDrift {
                reason: format!(
                    "export {} property-block boundary changed",
                    self.export_index
                ),
            });
        }

        let leaf =
            follow_fixed_leaf_path(&block, &self.leaf_path, self.kind).map_err(|reason| {
                FixedLeafPatchError::LayoutDrift {
                    reason: format!("export {} target path changed: {reason}", self.export_index),
                }
            })?;
        if leaf.role != FixedLeafRole::PropertyValue {
            return Err(FixedLeafPatchError::LayoutDrift {
                reason: format!(
                    "export {} target moved into {}",
                    self.export_index,
                    leaf.role.description()
                ),
            });
        }
        let absolute_offset = boundary
            .offset()
            .checked_add(leaf.span.offset())
            .ok_or(FixedLeafPatchError::RangeArithmetic)?;
        if leaf.span.offset() != self.leaf_offset
            || absolute_offset != self.absolute_offset
            || leaf.span.bytes() != target_bytes
        {
            return Err(FixedLeafPatchError::TargetDrift {
                component: self.component,
                offset: self.absolute_offset,
                length: target_bytes.len(),
            });
        }
        Ok(())
    }
}

/// Evidence returned after a post-validated patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedLeafPatchReceipt {
    pub before: PackagePairSeal,
    pub after: PackagePairSeal,
    pub export_index: usize,
    pub component: PackageComponent,
    pub absolute_offset: usize,
    pub length: usize,
    pub kind: FixedWireKind,
}

#[derive(Debug, Error)]
pub enum FixedLeafPatchError {
    #[error("the export envelope is not backed by the supplied package carrier")]
    ForeignExport,
    #[error("the property block does not start at byte zero of the supplied export")]
    ForeignPropertyBlock,
    #[error(
        "the supplied property block and leaf do not match a fresh walk with the export's exact schema"
    )]
    SemanticPathMismatch,
    #[error(
        "the fixed leaf is not the unique matching leaf in the supplied property block (matches={matching_leaves})"
    )]
    ForeignFixedLeaf { matching_leaves: usize },
    #[error(
        "fixed wire kind {kind:?} has width {expected}, but its observed span has {actual} bytes"
    )]
    InvalidObservedWidth {
        kind: FixedWireKind,
        expected: usize,
        actual: usize,
    },
    #[error(
        "expected bytes have length {actual}, but fixed wire kind {kind:?} requires {expected}"
    )]
    ExpectedLength {
        kind: FixedWireKind,
        expected: usize,
        actual: usize,
    },
    #[error(
        "replacement bytes have length {actual}, but fixed wire kind {kind:?} requires {expected}"
    )]
    ReplacementLength {
        kind: FixedWireKind,
        expected: usize,
        actual: usize,
    },
    #[error(
        "expected bytes differ from the observed leaf at relative byte {mismatch_offset}: expected 0x{expected:02x}, got 0x{actual:02x}"
    )]
    ExpectedMismatch {
        mismatch_offset: usize,
        expected: u8,
        actual: u8,
    },
    #[error(
        "editing referential fixed wire kind {kind:?} is refused until package-map validation is available"
    )]
    ReferentialEditUnsupported { kind: FixedWireKind },
    #[error(
        "editing a fixed leaf inside {section} is refused; map key identity is not a value-only patch"
    )]
    MapKeyEditUnsupported { section: &'static str },
    #[error(
        "editing a map value with a schema-recursive struct or map key is refused until the key's semantic schema can be sealed"
    )]
    ComplexMapKeyUnsupported,
    #[error("Bool replacement byte must be 0 or 1, got {value}")]
    InvalidBool { value: u8 },
    #[error("expected and replacement bytes are identical")]
    NoChange,
    #[error("package pair drifted after planning: expected {expected}; got {actual}")]
    PackageDrift {
        expected: Box<PackagePairSeal>,
        actual: Box<PackagePairSeal>,
    },
    #[error("exact schema layout for {class_path} changed after planning")]
    SchemaLayoutDrift { class_path: String },
    #[error("package layout drifted after planning: {reason}")]
    LayoutDrift { reason: String },
    #[error("target range {component} {offset}..+{length} drifted during validation")]
    TargetDrift {
        component: PackageComponent,
        offset: usize,
        length: usize,
    },
    #[error("fixed-leaf offset arithmetic overflowed")]
    RangeArithmetic,
    #[error(
        "postcondition failed after mutation and the original bytes were restored: {postcondition}"
    )]
    Postcondition { postcondition: String },
    #[error("postcondition failed ({postcondition}) and rollback failed: {rollback}")]
    RollbackFailed {
        postcondition: String,
        rollback: PackageError,
    },
    #[error(
        "postcondition failed ({postcondition}); rollback completed but pair verification failed: expected {expected}; got {actual}"
    )]
    RollbackVerification {
        postcondition: String,
        expected: Box<PackagePairSeal>,
        actual: Box<PackagePairSeal>,
    },
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Envelope(#[from] EnvelopeError),
    #[error(transparent)]
    ExportSchema(#[from] ExportSchemaError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(transparent)]
    Span(#[from] SpanError),
}

fn validate_fixed_replacement(
    kind: FixedWireKind,
    observed: &[u8],
    expected: &[u8],
    replacement: &[u8],
) -> Result<(), FixedLeafPatchError> {
    let width = kind.width();
    if observed.len() != width {
        return Err(FixedLeafPatchError::InvalidObservedWidth {
            kind,
            expected: width,
            actual: observed.len(),
        });
    }
    if expected.len() != width {
        return Err(FixedLeafPatchError::ExpectedLength {
            kind,
            expected: width,
            actual: expected.len(),
        });
    }
    if replacement.len() != width {
        return Err(FixedLeafPatchError::ReplacementLength {
            kind,
            expected: width,
            actual: replacement.len(),
        });
    }
    if let Some((relative, (&actual, &expected_byte))) = observed
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (actual, expected))| actual != expected)
    {
        return Err(FixedLeafPatchError::ExpectedMismatch {
            mismatch_offset: relative,
            expected: expected_byte,
            actual,
        });
    }
    if !fixed_wire_kind_is_editable(kind) {
        return Err(FixedLeafPatchError::ReferentialEditUnsupported { kind });
    }
    if kind == FixedWireKind::Bool && !matches!(replacement, [0] | [1]) {
        return Err(FixedLeafPatchError::InvalidBool {
            value: replacement[0],
        });
    }
    Ok(())
}

fn matching_fixed_leaves<'a>(
    block: &PropertyBlockSpans<'a>,
    target: SliceSpan<'a>,
    kind: FixedWireKind,
) -> Vec<FixedLeafCandidate<'a>> {
    let mut matches = Vec::new();
    let mut path = Vec::new();
    collect_matching_block(
        block,
        target,
        kind,
        FixedLeafRole::PropertyValue,
        &mut path,
        &mut matches,
    );
    matches
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FixedLeafPathStep {
    Property(PropertySlot),
    Struct {
        name: String,
    },
    Map {
        key_type: PropertyInner,
        value_type: PropertyInner,
    },
    MapEntryValue {
        index: usize,
        key_kind: Option<FixedWireKind>,
        key_length: usize,
        key_sha256: [u8; 32],
    },
    MapEntryKey {
        index: usize,
    },
    RemovedMapKey {
        index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixedLeafRole {
    #[serde(rename = "property_value")]
    PropertyValue,
    #[serde(rename = "map_key")]
    MapKey,
    #[serde(rename = "removed_map_key")]
    RemovedMapKey,
}

impl FixedLeafRole {
    fn description(self) -> &'static str {
        match self {
            Self::PropertyValue => "a property value",
            Self::MapKey => "a live map key",
            Self::RemovedMapKey => "a removed-map-key record",
        }
    }

    fn live_key_child(self) -> Self {
        match self {
            Self::PropertyValue => Self::MapKey,
            unsafe_parent => unsafe_parent,
        }
    }

    fn removed_key_child(self) -> Self {
        match self {
            Self::PropertyValue => Self::RemovedMapKey,
            unsafe_parent => unsafe_parent,
        }
    }
}

#[derive(Debug, Clone)]
struct FixedLeafCandidate<'a> {
    span: SliceSpan<'a>,
    role: FixedLeafRole,
    path: Vec<FixedLeafPathStep>,
}

#[derive(Debug, Clone, Copy)]
struct FixedLeafMatch<'a> {
    span: SliceSpan<'a>,
    role: FixedLeafRole,
}

fn collect_matching_block<'a>(
    block: &PropertyBlockSpans<'a>,
    target: SliceSpan<'a>,
    kind: FixedWireKind,
    role: FixedLeafRole,
    path: &mut Vec<FixedLeafPathStep>,
    matches: &mut Vec<FixedLeafCandidate<'a>>,
) {
    for property in block.properties() {
        if let Some(value) = property.value() {
            path.push(FixedLeafPathStep::Property(property.slot().clone()));
            collect_matching_value(value, target, kind, role, path, matches);
            path.pop();
        }
    }
}

fn collect_matching_value<'a>(
    value: &ValueSpan<'a>,
    target: SliceSpan<'a>,
    kind: FixedWireKind,
    role: FixedLeafRole,
    path: &mut Vec<FixedLeafPathStep>,
    matches: &mut Vec<FixedLeafCandidate<'a>>,
) {
    match value {
        ValueSpan::Fixed(fixed) => {
            if fixed.kind() == kind && same_slice(fixed.span().bytes(), target.bytes()) {
                matches.push(FixedLeafCandidate {
                    span: fixed.span(),
                    role,
                    path: path.clone(),
                });
            }
        }
        ValueSpan::Struct(value) => {
            path.push(FixedLeafPathStep::Struct {
                name: value.struct_name().to_owned(),
            });
            collect_matching_block(value.properties(), target, kind, role, path, matches);
            path.pop();
        }
        ValueSpan::Map(value) => {
            path.push(FixedLeafPathStep::Map {
                key_type: value.key_type().clone(),
                value_type: value.value_type().clone(),
            });
            for (index, removed_key) in value.removed_keys().iter().enumerate() {
                path.push(FixedLeafPathStep::RemovedMapKey { index });
                collect_matching_value(
                    removed_key,
                    target,
                    kind,
                    role.removed_key_child(),
                    path,
                    matches,
                );
                path.pop();
            }
            for (index, entry) in value.entries().iter().enumerate() {
                path.push(FixedLeafPathStep::MapEntryKey { index });
                collect_matching_value(
                    entry.key(),
                    target,
                    kind,
                    role.live_key_child(),
                    path,
                    matches,
                );
                path.pop();
                let key = entry.key().span();
                let key_kind = match entry.key() {
                    ValueSpan::Fixed(fixed) => Some(fixed.kind()),
                    ValueSpan::Struct(_) | ValueSpan::Map(_) => None,
                };
                path.push(FixedLeafPathStep::MapEntryValue {
                    index,
                    key_kind,
                    key_length: key.len(),
                    key_sha256: sha256(key.bytes()),
                });
                collect_matching_value(entry.value(), target, kind, role, path, matches);
                path.pop();
            }
            path.pop();
        }
    }
}

fn follow_fixed_leaf_path<'a>(
    block: &PropertyBlockSpans<'a>,
    path: &[FixedLeafPathStep],
    kind: FixedWireKind,
) -> Result<FixedLeafMatch<'a>, String> {
    follow_block_path(block, path, kind, FixedLeafRole::PropertyValue)
}

fn follow_block_path<'a>(
    block: &PropertyBlockSpans<'a>,
    path: &[FixedLeafPathStep],
    kind: FixedWireKind,
    role: FixedLeafRole,
) -> Result<FixedLeafMatch<'a>, String> {
    let Some((FixedLeafPathStep::Property(slot), rest)) = path.split_first() else {
        return Err("expected a property step".to_owned());
    };
    let mut matching = block
        .properties()
        .iter()
        .filter(|property| property.slot() == slot);
    let property = matching
        .next()
        .ok_or_else(|| format!("property {:?} is missing", slot.path()))?;
    if matching.next().is_some() {
        return Err(format!("property {:?} is ambiguous", slot.path()));
    }
    let value = property
        .value()
        .ok_or_else(|| format!("property {:?} became zero-masked", slot.path()))?;
    follow_value_path(value, rest, kind, role)
}

fn follow_value_path<'a>(
    value: &ValueSpan<'a>,
    path: &[FixedLeafPathStep],
    kind: FixedWireKind,
    role: FixedLeafRole,
) -> Result<FixedLeafMatch<'a>, String> {
    if path.is_empty() {
        let ValueSpan::Fixed(fixed) = value else {
            return Err("path no longer ends at a fixed leaf".to_owned());
        };
        if fixed.kind() != kind {
            return Err(format!(
                "fixed kind changed from {kind:?} to {:?}",
                fixed.kind()
            ));
        }
        return Ok(FixedLeafMatch {
            span: fixed.span(),
            role,
        });
    }

    match (value, &path[0]) {
        (ValueSpan::Struct(value), FixedLeafPathStep::Struct { name }) => {
            if value.struct_name() != name {
                return Err(format!(
                    "struct changed from {name:?} to {:?}",
                    value.struct_name()
                ));
            }
            follow_block_path(value.properties(), &path[1..], kind, role)
        }
        (
            ValueSpan::Map(value),
            FixedLeafPathStep::Map {
                key_type,
                value_type,
            },
        ) => {
            if value.key_type() != key_type || value.value_type() != value_type {
                return Err("map key/value schema changed".to_owned());
            }
            follow_map_path(value, &path[1..], kind, role)
        }
        _ => Err("semantic leaf path changed value kind".to_owned()),
    }
}

fn follow_map_path<'a>(
    map: &crate::MapValueSpan<'a>,
    path: &[FixedLeafPathStep],
    kind: FixedWireKind,
    role: FixedLeafRole,
) -> Result<FixedLeafMatch<'a>, String> {
    let Some((branch, rest)) = path.split_first() else {
        return Err("map path has no key/value branch".to_owned());
    };
    match branch {
        FixedLeafPathStep::MapEntryValue {
            index,
            key_kind,
            key_length,
            key_sha256,
        } => {
            let entry = map
                .entries()
                .get(*index)
                .ok_or_else(|| format!("map entry {index} is missing"))?;
            let key = entry.key().span();
            if key.len() != *key_length || sha256(key.bytes()) != *key_sha256 {
                return Err(format!("map entry {index} key identity changed"));
            }
            let ValueSpan::Fixed(fixed_key) = entry.key() else {
                return Err(format!("map entry {index} key is no longer fixed-width"));
            };
            if Some(fixed_key.kind()) != *key_kind {
                return Err(format!("map entry {index} key wire kind changed"));
            }
            follow_value_path(entry.value(), rest, kind, role)
        }
        FixedLeafPathStep::MapEntryKey { index } => {
            let entry = map
                .entries()
                .get(*index)
                .ok_or_else(|| format!("map entry {index} is missing"))?;
            follow_value_path(entry.key(), rest, kind, role.live_key_child())
        }
        FixedLeafPathStep::RemovedMapKey { index } => {
            let key = map
                .removed_keys()
                .get(*index)
                .ok_or_else(|| format!("removed map key {index} is missing"))?;
            follow_value_path(key, rest, kind, role.removed_key_child())
        }
        _ => Err("map path has an invalid branch".to_owned()),
    }
}

fn same_slice(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && std::ptr::eq(left.as_ptr(), right.as_ptr())
}

fn same_prefix(prefix: &[u8], whole: &[u8]) -> bool {
    prefix.len() <= whole.len() && std::ptr::eq(prefix.as_ptr(), whole.as_ptr())
}

fn slice_at(whole: &[u8], offset: usize, candidate: &[u8]) -> bool {
    let Some(end) = offset.checked_add(candidate.len()) else {
        return false;
    };
    whole
        .get(offset..end)
        .is_some_and(|slice| same_slice(slice, candidate))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod selector_wire_tests {
    use super::*;

    fn format_1_selector() -> FixedLeafSelector {
        FixedLeafSelector {
            format: FIXED_LEAF_SELECTOR_FORMAT,
            profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
            package_seal: PackagePairSeal {
                uasset_sha256: [0x11; 32],
                uexp_sha256: [0x22; 32],
            },
            usmap_sha256: "33".repeat(32),
            export_index: 7,
            object_name: "Asset".to_owned(),
            class_path: "/Script/Test.Fixture".to_owned(),
            component: PackageComponent::Uexp,
            export_sha256: "44".repeat(32),
            role: FixedLeafRole::PropertyValue,
            kind: FixedWireKind::Bool,
            path: vec![FixedLeafSelectorStep::Property {
                schema_index: 2,
                property_name: "Enabled".to_owned(),
                array_index: 0,
                array_dimension: 1,
                declaring_schema_name: "Fixture".to_owned(),
                declaring_module_path: Some("/Script/Test".to_owned()),
                property_type: FixedLeafWireType::Map {
                    key: Box::new(FixedLeafWireType::Name {}),
                    value: Box::new(FixedLeafWireType::Optional {
                        inner: Box::new(FixedLeafWireType::Struct {
                            name: "State".to_owned(),
                        }),
                    }),
                },
            }],
            expected_hex: "01".to_owned(),
        }
    }

    #[test]
    fn format_1_selector_json_is_golden_and_round_trips() {
        let selector = format_1_selector();
        let json = serde_json::to_string(&selector).unwrap();
        let expected = concat!(
            "{\"format\":1,\"profile\":\"g1r_ue5_4\",\"package_seal\":{",
            "\"uasset_sha256\":\"1111111111111111111111111111111111111111111111111111111111111111\",",
            "\"uexp_sha256\":\"2222222222222222222222222222222222222222222222222222222222222222\"},",
            "\"usmap_sha256\":\"3333333333333333333333333333333333333333333333333333333333333333\",",
            "\"export_index\":7,\"object_name\":\"Asset\",\"class_path\":\"/Script/Test.Fixture\",",
            "\"component\":\"uexp\",\"export_sha256\":\"4444444444444444444444444444444444444444444444444444444444444444\",",
            "\"role\":\"property_value\",\"kind\":\"bool\",\"path\":[{\"step\":\"property\",",
            "\"schema_index\":2,\"property_name\":\"Enabled\",\"array_index\":0,\"array_dimension\":1,",
            "\"declaring_schema_name\":\"Fixture\",\"declaring_module_path\":\"/Script/Test\",",
            "\"property_type\":{\"type\":\"map\",\"key\":{\"type\":\"name\"},",
            "\"value\":{\"type\":\"optional\",\"inner\":{\"type\":\"struct\",\"name\":\"State\"}}}}],",
            "\"expected_hex\":\"01\"}"
        );
        assert_eq!(json, expected);
        assert_eq!(
            serde_json::from_str::<FixedLeafSelector>(&json).unwrap(),
            selector
        );
    }

    #[test]
    fn format_1_selector_rejects_unknown_top_level_and_wire_type_fields() {
        let json = serde_json::to_string(&format_1_selector()).unwrap();
        let top_level = json.replacen("{\"format\":1", "{\"future\":true,\"format\":1", 1);
        assert!(serde_json::from_str::<FixedLeafSelector>(&top_level).is_err());

        let nested = json.replacen(
            "{\"type\":\"name\"}",
            "{\"type\":\"name\",\"future\":true}",
            1,
        );
        assert!(serde_json::from_str::<FixedLeafSelector>(&nested).is_err());

        let deep_nested = json.replacen(
            "{\"type\":\"struct\",\"name\":\"State\"}",
            "{\"type\":\"struct\",\"name\":\"State\",\"future\":true}",
            1,
        );
        assert!(serde_json::from_str::<FixedLeafSelector>(&deep_nested).is_err());

        let nested_step = json.replacen(
            "\"schema_index\":2",
            "\"schema_index\":2,\"future\":true",
            1,
        );
        assert!(serde_json::from_str::<FixedLeafSelector>(&nested_step).is_err());

        let future_type = json.replacen("\"type\":\"name\"", "\"type\":\"future\"", 1);
        assert!(serde_json::from_str::<FixedLeafSelector>(&future_type).is_err());
    }

    #[test]
    fn format_1_round_trips_every_wire_type_variant() {
        let variants = vec![
            (FixedLeafWireType::Byte {}, r#"{"type":"byte"}"#),
            (FixedLeafWireType::Bool {}, r#"{"type":"bool"}"#),
            (FixedLeafWireType::Int {}, r#"{"type":"int"}"#),
            (FixedLeafWireType::Float {}, r#"{"type":"float"}"#),
            (FixedLeafWireType::Object {}, r#"{"type":"object"}"#),
            (FixedLeafWireType::Name {}, r#"{"type":"name"}"#),
            (FixedLeafWireType::Delegate {}, r#"{"type":"delegate"}"#),
            (FixedLeafWireType::Double {}, r#"{"type":"double"}"#),
            (
                FixedLeafWireType::Array {
                    inner: Box::new(FixedLeafWireType::Bool {}),
                },
                r#"{"type":"array","inner":{"type":"bool"}}"#,
            ),
            (
                FixedLeafWireType::Struct {
                    name: "State".to_owned(),
                },
                r#"{"type":"struct","name":"State"}"#,
            ),
            (FixedLeafWireType::String {}, r#"{"type":"string"}"#),
            (FixedLeafWireType::Text {}, r#"{"type":"text"}"#),
            (FixedLeafWireType::Interface {}, r#"{"type":"interface"}"#),
            (
                FixedLeafWireType::MulticastDelegate {},
                r#"{"type":"multicast_delegate"}"#,
            ),
            (
                FixedLeafWireType::WeakObject {},
                r#"{"type":"weak_object"}"#,
            ),
            (
                FixedLeafWireType::LazyObject {},
                r#"{"type":"lazy_object"}"#,
            ),
            (
                FixedLeafWireType::AssetObject {},
                r#"{"type":"asset_object"}"#,
            ),
            (
                FixedLeafWireType::SoftObject {},
                r#"{"type":"soft_object"}"#,
            ),
            (FixedLeafWireType::UInt64 {}, r#"{"type":"uint64"}"#),
            (FixedLeafWireType::UInt32 {}, r#"{"type":"uint32"}"#),
            (FixedLeafWireType::UInt16 {}, r#"{"type":"uint16"}"#),
            (FixedLeafWireType::Int64 {}, r#"{"type":"int64"}"#),
            (FixedLeafWireType::Int16 {}, r#"{"type":"int16"}"#),
            (FixedLeafWireType::Int8 {}, r#"{"type":"int8"}"#),
            (
                FixedLeafWireType::Map {
                    key: Box::new(FixedLeafWireType::Name {}),
                    value: Box::new(FixedLeafWireType::Int {}),
                },
                r#"{"type":"map","key":{"type":"name"},"value":{"type":"int"}}"#,
            ),
            (
                FixedLeafWireType::Set {
                    key: Box::new(FixedLeafWireType::Object {}),
                },
                r#"{"type":"set","key":{"type":"object"}}"#,
            ),
            (
                FixedLeafWireType::Enum {
                    inner: Box::new(FixedLeafWireType::Byte {}),
                    name: "Mode".to_owned(),
                },
                r#"{"type":"enum","inner":{"type":"byte"},"name":"Mode"}"#,
            ),
            (FixedLeafWireType::FieldPath {}, r#"{"type":"field_path"}"#),
            (
                FixedLeafWireType::Optional {
                    inner: Box::new(FixedLeafWireType::Double {}),
                },
                r#"{"type":"optional","inner":{"type":"double"}}"#,
            ),
            (
                FixedLeafWireType::Utf8String {},
                r#"{"type":"utf8_string"}"#,
            ),
            (
                FixedLeafWireType::AnsiString {},
                r#"{"type":"ansi_string"}"#,
            ),
            (FixedLeafWireType::Unknown {}, r#"{"type":"unknown"}"#),
        ];

        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
            assert_eq!(
                serde_json::from_str::<FixedLeafWireType>(&json).unwrap(),
                variant
            );
        }
    }

    #[test]
    fn format_1_round_trips_every_step_role_kind_and_map_key_identity() {
        let key = FixedLeafMapKeyIdentity {
            kind: Some(FixedWireKind::Int32),
            byte_length: 4,
            sha256: "aa".repeat(32),
        };
        let key_json = concat!(
            "{\"kind\":\"int32\",\"byte_length\":4,",
            "\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}"
        );
        assert_eq!(serde_json::to_string(&key).unwrap(), key_json);
        assert_eq!(
            serde_json::from_str::<FixedLeafMapKeyIdentity>(key_json).unwrap(),
            key
        );

        let steps = vec![
            (
                FixedLeafSelectorStep::Property {
                    schema_index: 3,
                    property_name: "Value".to_owned(),
                    array_index: 1,
                    array_dimension: 2,
                    declaring_schema_name: "Fixture".to_owned(),
                    declaring_module_path: Some("/Script/Test".to_owned()),
                    property_type: FixedLeafWireType::Bool {},
                },
                "property",
            ),
            (
                FixedLeafSelectorStep::Struct {
                    name: "BoneTrackedData".to_owned(),
                    schema_name: "/Script/G1R.BoneTrackedData".to_owned(),
                },
                "struct",
            ),
            (
                FixedLeafSelectorStep::Map {
                    key_type: FixedLeafWireType::Int {},
                    value_type: FixedLeafWireType::Bool {},
                },
                "map",
            ),
            (
                FixedLeafSelectorStep::MapEntryValue { key: key.clone() },
                "map_entry_value",
            ),
            (
                FixedLeafSelectorStep::MapEntryKey { key: key.clone() },
                "map_entry_key",
            ),
            (
                FixedLeafSelectorStep::RemovedMapKey { key: key.clone() },
                "removed_map_key",
            ),
        ];
        for (step, tag) in steps {
            let json = serde_json::to_string(&step).unwrap();
            assert!(json.starts_with(&format!(r#"{{"step":"{tag}""#)));
            assert_eq!(
                serde_json::from_str::<FixedLeafSelectorStep>(&json).unwrap(),
                step
            );
        }

        let roles = [
            (FixedLeafRole::PropertyValue, "property_value"),
            (FixedLeafRole::MapKey, "map_key"),
            (FixedLeafRole::RemovedMapKey, "removed_map_key"),
        ];
        for (role, name) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, format!(r#""{name}""#));
            assert_eq!(serde_json::from_str::<FixedLeafRole>(&json).unwrap(), role);
        }

        let kinds = [
            (FixedWireKind::Byte, "byte"),
            (FixedWireKind::Bool, "bool"),
            (FixedWireKind::Int32, "int32"),
            (FixedWireKind::Float32, "float32"),
            (FixedWireKind::PackageIndex, "package_index"),
            (FixedWireKind::FName, "fname"),
            (FixedWireKind::Float64, "float64"),
            (FixedWireKind::UInt64, "uint64"),
            (FixedWireKind::UInt32, "uint32"),
            (FixedWireKind::UInt16, "uint16"),
            (FixedWireKind::Int64, "int64"),
            (FixedWireKind::Int16, "int16"),
            (FixedWireKind::Int8, "int8"),
            (FixedWireKind::LinearColorF32x4, "linear_color_f32x4"),
            (FixedWireKind::Vector4F64x4, "vector4_f64x4"),
        ];
        for (kind, name) in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!(r#""{name}""#));
            assert_eq!(serde_json::from_str::<FixedWireKind>(&json).unwrap(), kind);
        }
    }

    #[test]
    fn editable_kind_whitelist_is_the_planner_authority() {
        let cases = [
            (FixedWireKind::Byte, true),
            (FixedWireKind::Bool, true),
            (FixedWireKind::Int32, true),
            (FixedWireKind::Float32, true),
            (FixedWireKind::PackageIndex, false),
            (FixedWireKind::FName, false),
            (FixedWireKind::Float64, true),
            (FixedWireKind::UInt64, true),
            (FixedWireKind::UInt32, true),
            (FixedWireKind::UInt16, true),
            (FixedWireKind::Int64, true),
            (FixedWireKind::Int16, true),
            (FixedWireKind::Int8, true),
            (FixedWireKind::LinearColorF32x4, true),
            (FixedWireKind::Vector4F64x4, true),
        ];
        for (kind, editable) in cases {
            assert_eq!(fixed_wire_kind_is_editable(kind), editable);
            let observed = vec![0; kind.width()];
            let mut replacement = observed.clone();
            replacement[0] = 1;
            let result = validate_fixed_replacement(kind, &observed, &observed, &replacement);
            assert_eq!(result.is_ok(), editable, "kind={kind:?}, result={result:?}");
        }
    }

    #[test]
    fn selector_path_growth_is_debited_before_vec_allocation() {
        let charge = std::mem::size_of::<FixedLeafSelectorStep>() + 64;
        let mut path = Vec::new();
        let mut zero = FixedLeafWorkBudget::new(FixedLeafWorkLimits {
            max_allocation_bytes: 0,
            ..FixedLeafWorkLimits::default()
        });
        assert!(matches!(
            reserve_one_path_step(&mut path, &mut zero),
            Err(FixedLeafInspectionError::ResourceLimit {
                resource: "allocations"
            })
        ));
        assert_eq!(path.capacity(), 0);

        let mut exact = FixedLeafWorkBudget::new(FixedLeafWorkLimits {
            max_allocation_bytes: charge,
            ..FixedLeafWorkLimits::default()
        });
        reserve_one_path_step(&mut path, &mut exact).unwrap();
        assert!(path.capacity() >= 1);
        assert_eq!(exact.remaining_allocation_bytes(), 0);
    }
}
