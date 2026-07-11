//! Read-only byte-span discovery for proven G1R UE5.4 property wire forms.
//!
//! This module deliberately separates structural discovery from editing. It
//! walks an unversioned property block with its exact USMAP schema and returns
//! borrowed slices for every recognized value. Bytes are never decoded into an
//! editable semantic model and no size is inferred for an unsupported type.

use std::collections::HashMap;

use thiserror::Error;
use usmap::PropertyInner;

use crate::{
    PropertySlot, SchemaDb, SchemaError, SchemaId, SchemaKind, UnversionedError, UnversionedHeader,
};

/// Resource bounds for one recursive span walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanLimits {
    /// Maximum nesting below the root property value.
    pub max_depth: usize,
    /// Maximum removed-key count or live-entry count in one map section.
    pub max_collection_elements: usize,
    /// Maximum number of blocks, properties, values, and map entries returned.
    pub max_total_nodes: usize,
}

impl Default for SpanLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_collection_elements: 1_000_000,
            max_total_nodes: 1_000_000,
        }
    }
}

/// A borrowed, opaque byte range relative to the root input slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceSpan<'a> {
    offset: usize,
    bytes: &'a [u8],
}

impl<'a> SliceSpan<'a> {
    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn end(&self) -> usize {
        self.offset
            .checked_add(self.bytes.len())
            .expect("span range was validated when constructed")
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// Fixed-size encodings proven for the G1R UE5.4 cooked-property profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedWireKind {
    Byte,
    Bool,
    Int32,
    Float32,
    PackageIndex,
    FName,
    Float64,
    UInt64,
    UInt32,
    UInt16,
    Int64,
    Int16,
    Int8,
    LinearColorF32x4,
    Vector4F64x4,
}

impl FixedWireKind {
    pub fn width(self) -> usize {
        match self {
            Self::Byte | Self::Bool | Self::Int8 => 1,
            Self::UInt16 | Self::Int16 => 2,
            Self::Int32 | Self::Float32 | Self::PackageIndex | Self::UInt32 => 4,
            Self::FName | Self::Float64 | Self::UInt64 | Self::Int64 => 8,
            Self::LinearColorF32x4 => 16,
            Self::Vector4F64x4 => 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedValueSpan<'a> {
    kind: FixedWireKind,
    span: SliceSpan<'a>,
}

impl<'a> FixedValueSpan<'a> {
    pub fn kind(&self) -> FixedWireKind {
        self.kind
    }

    pub fn span(&self) -> SliceSpan<'a> {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructValueSpan<'a> {
    struct_name: String,
    span: SliceSpan<'a>,
    properties: Box<PropertyBlockSpans<'a>>,
}

impl<'a> StructValueSpan<'a> {
    pub fn struct_name(&self) -> &str {
        &self.struct_name
    }

    pub fn span(&self) -> SliceSpan<'a> {
        self.span
    }

    pub fn properties(&self) -> &PropertyBlockSpans<'a> {
        &self.properties
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntrySpan<'a> {
    span: SliceSpan<'a>,
    key: ValueSpan<'a>,
    value: ValueSpan<'a>,
}

impl<'a> MapEntrySpan<'a> {
    pub fn span(&self) -> SliceSpan<'a> {
        self.span
    }

    pub fn key(&self) -> &ValueSpan<'a> {
        &self.key
    }

    pub fn value(&self) -> &ValueSpan<'a> {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapValueSpan<'a> {
    span: SliceSpan<'a>,
    key_type: PropertyInner,
    value_type: PropertyInner,
    removed_count: SliceSpan<'a>,
    removed_keys: Vec<ValueSpan<'a>>,
    entry_count: SliceSpan<'a>,
    entries: Vec<MapEntrySpan<'a>>,
}

impl<'a> MapValueSpan<'a> {
    pub fn span(&self) -> SliceSpan<'a> {
        self.span
    }

    pub fn key_type(&self) -> &PropertyInner {
        &self.key_type
    }

    pub fn value_type(&self) -> &PropertyInner {
        &self.value_type
    }

    /// Four-byte signed count field, retained without normalizing its bytes.
    pub fn removed_count_span(&self) -> SliceSpan<'a> {
        self.removed_count
    }

    pub fn removed_keys(&self) -> &[ValueSpan<'a>] {
        &self.removed_keys
    }

    /// Four-byte signed count field, retained without normalizing its bytes.
    pub fn entry_count_span(&self) -> SliceSpan<'a> {
        self.entry_count
    }

    pub fn entries(&self) -> &[MapEntrySpan<'a>] {
        &self.entries
    }
}

/// One recognized value. Every variant owns only metadata and borrowed bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueSpan<'a> {
    Fixed(FixedValueSpan<'a>),
    Struct(StructValueSpan<'a>),
    Map(MapValueSpan<'a>),
}

impl<'a> ValueSpan<'a> {
    pub fn span(&self) -> SliceSpan<'a> {
        match self {
            Self::Fixed(value) => value.span(),
            Self::Struct(value) => value.span(),
            Self::Map(value) => value.span(),
        }
    }
}

/// One header-selected property. Zero-masked properties have no value span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySpan<'a> {
    slot: PropertySlot,
    is_zero: bool,
    value: Option<ValueSpan<'a>>,
}

impl<'a> PropertySpan<'a> {
    pub fn slot(&self) -> &PropertySlot {
        &self.slot
    }

    pub fn is_zero(&self) -> bool {
        self.is_zero
    }

    pub fn value(&self) -> Option<&ValueSpan<'a>> {
        self.value.as_ref()
    }

    pub fn span(&self) -> Option<SliceSpan<'a>> {
        self.value.as_ref().map(ValueSpan::span)
    }
}

/// A complete schema-backed unversioned block and its exact consumed range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyBlockSpans<'a> {
    schema_id: SchemaId,
    schema_name: String,
    span: SliceSpan<'a>,
    header: SliceSpan<'a>,
    properties: Vec<PropertySpan<'a>>,
}

impl<'a> PropertyBlockSpans<'a> {
    pub fn schema_id(&self) -> SchemaId {
        self.schema_id
    }

    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    pub fn span(&self) -> SliceSpan<'a> {
        self.span
    }

    pub fn header(&self) -> SliceSpan<'a> {
        self.header
    }

    pub fn properties(&self) -> &[PropertySpan<'a>] {
        &self.properties
    }

    /// Number of bytes consumed from this block's own starting offset.
    pub fn consumed(&self) -> usize {
        self.span.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSection {
    RemovedKeys,
    Entries,
}

impl std::fmt::Display for MapSection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RemovedKeys => formatter.write_str("removed keys"),
            Self::Entries => formatter.write_str("entries"),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpanError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(
        "unversioned header for schema {schema_id} at byte {byte_offset} is invalid: {source}"
    )]
    Header {
        schema_id: SchemaId,
        byte_offset: usize,
        #[source]
        source: UnversionedError,
    },
    #[error("{wire_type} at byte {byte_offset} needs {needed} bytes, but only {available} remain")]
    Truncated {
        byte_offset: usize,
        needed: usize,
        available: usize,
        wire_type: String,
    },
    #[error("byte-range arithmetic overflow while walking {wire_type} at byte {byte_offset}")]
    OffsetOverflow {
        byte_offset: usize,
        wire_type: String,
    },
    #[error("bool payload at byte {byte_offset} is {value}; only 0 and 1 are valid")]
    InvalidBool { byte_offset: usize, value: u8 },
    #[error("unsupported non-zero wire type {property_type} at byte {byte_offset}")]
    UnsupportedType {
        byte_offset: usize,
        property_type: String,
    },
    #[error("struct {name:?} has no proven G1R UE5.4 wire form at byte {byte_offset}")]
    UnsupportedStruct { byte_offset: usize, name: String },
    #[error("fixed struct {name:?} does not match the proven CoreUObject schema: {reason}")]
    FixedStructSchema { name: String, reason: String },
    #[error("nested struct {name:?} resolves to {actual:?}, not a struct schema")]
    NestedSchemaKind { name: String, actual: SchemaKind },
    #[error("nested struct {name:?} resolves outside /Script/G1R: {actual:?}")]
    NestedSchemaModule {
        name: String,
        actual: Option<String>,
    },
    #[error("map {section} count at byte {byte_offset} is negative: {value}")]
    NegativeCollectionCount {
        byte_offset: usize,
        section: MapSection,
        value: i32,
    },
    #[error(
        "map {section} count {count} at byte {byte_offset} exceeds the configured limit {limit}"
    )]
    CollectionLimit {
        byte_offset: usize,
        section: MapSection,
        count: usize,
        limit: usize,
    },
    #[error(
        "wire nesting depth {depth} at byte {byte_offset} exceeds the limit {limit} ({wire_type})"
    )]
    DepthLimit {
        byte_offset: usize,
        wire_type: String,
        depth: usize,
        limit: usize,
    },
    #[error("span tree would contain {attempted} nodes at byte {byte_offset}; limit is {limit}")]
    NodeLimit {
        byte_offset: usize,
        attempted: usize,
        limit: usize,
    },
}

/// Read-only walker for the explicitly version-gated G1R UE5.4 wire profile.
#[derive(Debug)]
pub struct PropertySpanWalker<'db> {
    schemas: &'db SchemaDb,
    limits: SpanLimits,
}

impl<'db> PropertySpanWalker<'db> {
    pub fn g1r_ue5_4(schemas: &'db SchemaDb) -> Self {
        Self::g1r_ue5_4_with_limits(schemas, SpanLimits::default())
    }

    pub fn g1r_ue5_4_with_limits(schemas: &'db SchemaDb, limits: SpanLimits) -> Self {
        Self { schemas, limits }
    }

    pub fn limits(&self) -> SpanLimits {
        self.limits
    }

    /// Walk one schema-backed property stream from byte zero.
    ///
    /// `consumed()` is the exact property end. Any remaining input stays
    /// outside the result and may be retained as an opaque native suffix.
    pub fn walk<'bytes>(
        &self,
        bytes: &'bytes [u8],
        schema_id: SchemaId,
    ) -> Result<PropertyBlockSpans<'bytes>, SpanError> {
        let mut state = WalkState {
            schemas: self.schemas,
            limits: self.limits,
            bytes,
            nodes: 0,
            linear_color_validated: false,
            vector4_validated: false,
            slot_cache: HashMap::new(),
        };
        let (block, _) = state.walk_block(schema_id, 0, 0)?;
        Ok(block)
    }
}

struct WalkState<'db, 'bytes> {
    schemas: &'db SchemaDb,
    limits: SpanLimits,
    bytes: &'bytes [u8],
    nodes: usize,
    linear_color_validated: bool,
    vector4_validated: bool,
    slot_cache: HashMap<SchemaId, Vec<PropertySlot>>,
}

impl<'db, 'bytes> WalkState<'db, 'bytes> {
    fn walk_block(
        &mut self,
        schema_id: SchemaId,
        start: usize,
        depth: usize,
    ) -> Result<(PropertyBlockSpans<'bytes>, usize), SpanError> {
        self.ensure_depth(depth, start, "unversioned property block")?;
        self.charge_nodes(1, start)?;
        let schema_name = self.schemas.schema(schema_id)?.qualified_name();
        let slots = self.flattened_slots(schema_id)?;
        let remaining = self
            .bytes
            .get(start..)
            .ok_or_else(|| SpanError::Truncated {
                byte_offset: start,
                needed: 1,
                available: 0,
                wire_type: "unversioned property block".to_owned(),
            })?;
        let (header, header_len) =
            UnversionedHeader::decode(remaining, slots.len()).map_err(|source| {
                SpanError::Header {
                    schema_id,
                    byte_offset: start,
                    source,
                }
            })?;
        let header_end = self.checked_end(start, header_len, "unversioned header")?;
        let header_span = self.slice(start, header_end, "unversioned header")?;
        self.charge_nodes(header.entries().len(), start)?;

        let resolved = header
            .resolve_slots(&slots)
            .map_err(|source| SpanError::Header {
                schema_id,
                byte_offset: start,
                source,
            })?;
        let mut cursor = header_end;
        let mut properties = Vec::with_capacity(resolved.len());
        for entry in resolved {
            let value = if entry.is_zero {
                None
            } else {
                let (value, end) = self.walk_value(&entry.slot.inner, cursor, depth)?;
                cursor = end;
                Some(value)
            };
            properties.push(PropertySpan {
                slot: entry.slot.clone(),
                is_zero: entry.is_zero,
                value,
            });
        }

        let span = self.slice(start, cursor, "unversioned property block")?;
        Ok((
            PropertyBlockSpans {
                schema_id,
                schema_name,
                span,
                header: header_span,
                properties,
            },
            cursor,
        ))
    }

    fn walk_value(
        &mut self,
        inner: &PropertyInner,
        start: usize,
        depth: usize,
    ) -> Result<(ValueSpan<'bytes>, usize), SpanError> {
        self.ensure_depth(depth, start, &format!("{inner:?}"))?;
        self.charge_nodes(1, start)?;

        if let Some(kind) = fixed_wire_kind(inner) {
            let end = self.checked_end(start, kind.width(), fixed_wire_name(kind))?;
            let span = self.slice(start, end, fixed_wire_name(kind))?;
            if kind == FixedWireKind::Bool {
                let value = span.bytes()[0];
                if value > 1 {
                    return Err(SpanError::InvalidBool {
                        byte_offset: start,
                        value,
                    });
                }
            }
            return Ok((ValueSpan::Fixed(FixedValueSpan { kind, span }), end));
        }

        match inner {
            PropertyInner::Struct { name } => self.walk_struct(name, start, depth),
            PropertyInner::Map { key, value } => self.walk_map(key, value, start, depth),
            unsupported => Err(SpanError::UnsupportedType {
                byte_offset: start,
                property_type: format!("{unsupported:?}"),
            }),
        }
    }

    fn walk_struct(
        &mut self,
        name: &str,
        start: usize,
        depth: usize,
    ) -> Result<(ValueSpan<'bytes>, usize), SpanError> {
        match name {
            "LinearColor" => {
                if !self.linear_color_validated {
                    self.validate_fixed_struct_schema(
                        name,
                        &["R", "G", "B", "A"],
                        &PropertyInner::Float,
                    )?;
                    self.linear_color_validated = true;
                }
                self.walk_fixed_struct(start, FixedWireKind::LinearColorF32x4)
            }
            "Vector4" => {
                if !self.vector4_validated {
                    self.validate_fixed_struct_schema(
                        name,
                        &["X", "Y", "Z", "W"],
                        &PropertyInner::Double,
                    )?;
                    self.vector4_validated = true;
                }
                self.walk_fixed_struct(start, FixedWireKind::Vector4F64x4)
            }
            "BoneFeetData" | "BoneTrackedData" => {
                let child_depth = self.child_depth(depth, start, "nested unversioned struct")?;
                let schema_id = self.schemas.resolve(name)?;
                let schema = self.schemas.schema(schema_id)?;
                if schema.kind != SchemaKind::Struct {
                    return Err(SpanError::NestedSchemaKind {
                        name: name.to_owned(),
                        actual: schema.kind,
                    });
                }
                if !schema
                    .module_path
                    .as_deref()
                    .is_some_and(|module| module.eq_ignore_ascii_case("/Script/G1R"))
                {
                    return Err(SpanError::NestedSchemaModule {
                        name: name.to_owned(),
                        actual: schema.module_path.clone(),
                    });
                }
                let (properties, end) = self.walk_block(schema_id, start, child_depth)?;
                let span = self.slice(start, end, "nested unversioned struct")?;
                Ok((
                    ValueSpan::Struct(StructValueSpan {
                        struct_name: name.to_owned(),
                        span,
                        properties: Box::new(properties),
                    }),
                    end,
                ))
            }
            _ => Err(SpanError::UnsupportedStruct {
                byte_offset: start,
                name: name.to_owned(),
            }),
        }
    }

    fn validate_fixed_struct_schema(
        &self,
        name: &str,
        expected_names: &[&str],
        expected_inner: &PropertyInner,
    ) -> Result<(), SpanError> {
        let schema_id = self.schemas.resolve(name)?;
        let schema = self.schemas.schema(schema_id)?;
        if schema.kind != SchemaKind::Struct {
            return Err(SpanError::FixedStructSchema {
                name: name.to_owned(),
                reason: format!("schema kind is {:?}", schema.kind),
            });
        }
        if !schema
            .module_path
            .as_deref()
            .is_some_and(|module| module.eq_ignore_ascii_case("/Script/CoreUObject"))
        {
            return Err(SpanError::FixedStructSchema {
                name: name.to_owned(),
                reason: format!("module is {:?}", schema.module_path),
            });
        }
        let slots = self.schemas.flatten_slots(schema_id)?;
        if slots.len() != expected_names.len() {
            return Err(SpanError::FixedStructSchema {
                name: name.to_owned(),
                reason: format!(
                    "schema has {} slots, expected {}",
                    slots.len(),
                    expected_names.len()
                ),
            });
        }
        for (slot, expected_name) in slots.iter().zip(expected_names) {
            if slot.property_name != *expected_name
                || slot.array_dimension != 1
                || slot.array_index != 0
                || slot.inner != *expected_inner
            {
                return Err(SpanError::FixedStructSchema {
                    name: name.to_owned(),
                    reason: format!(
                        "slot {} is {}[{}] {:?}, expected {} {:?}",
                        slot.schema_index,
                        slot.property_name,
                        slot.array_index,
                        slot.inner,
                        expected_name,
                        expected_inner
                    ),
                });
            }
        }
        Ok(())
    }

    fn flattened_slots(&mut self, schema_id: SchemaId) -> Result<Vec<PropertySlot>, SpanError> {
        if let Some(slots) = self.slot_cache.get(&schema_id) {
            return Ok(slots.clone());
        }
        let slots = self.schemas.flatten_slots(schema_id)?;
        self.slot_cache.insert(schema_id, slots.clone());
        Ok(slots)
    }

    fn walk_fixed_struct(
        &mut self,
        start: usize,
        kind: FixedWireKind,
    ) -> Result<(ValueSpan<'bytes>, usize), SpanError> {
        let end = self.checked_end(start, kind.width(), fixed_wire_name(kind))?;
        let span = self.slice(start, end, fixed_wire_name(kind))?;
        Ok((ValueSpan::Fixed(FixedValueSpan { kind, span }), end))
    }

    fn walk_map(
        &mut self,
        key_type: &PropertyInner,
        value_type: &PropertyInner,
        start: usize,
        depth: usize,
    ) -> Result<(ValueSpan<'bytes>, usize), SpanError> {
        let child_depth = self.child_depth(depth, start, "map")?;
        let (removed_count, removed_count_span, mut cursor) =
            self.read_collection_count(start, MapSection::RemovedKeys)?;
        // Grow only after each child has charged the global node budget. This
        // avoids a large up-front allocation when a caller deliberately sets a
        // much smaller node limit than collection limit.
        let mut removed_keys = Vec::new();
        for _ in 0..removed_count {
            let (key, end) = self.walk_value(key_type, cursor, child_depth)?;
            cursor = end;
            removed_keys.push(key);
        }

        let (entry_count, entry_count_span, next) =
            self.read_collection_count(cursor, MapSection::Entries)?;
        cursor = next;
        self.charge_nodes(entry_count, cursor)?;
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let entry_start = cursor;
            let (key, key_end) = self.walk_value(key_type, cursor, child_depth)?;
            let (value, value_end) = self.walk_value(value_type, key_end, child_depth)?;
            cursor = value_end;
            entries.push(MapEntrySpan {
                span: self.slice(entry_start, cursor, "map entry")?,
                key,
                value,
            });
        }

        let span = self.slice(start, cursor, "map")?;
        Ok((
            ValueSpan::Map(MapValueSpan {
                span,
                key_type: key_type.clone(),
                value_type: value_type.clone(),
                removed_count: removed_count_span,
                removed_keys,
                entry_count: entry_count_span,
                entries,
            }),
            cursor,
        ))
    }

    fn read_collection_count(
        &self,
        start: usize,
        section: MapSection,
    ) -> Result<(usize, SliceSpan<'bytes>, usize), SpanError> {
        let end = self.checked_end(start, 4, "map count")?;
        let span = self.slice(start, end, "map count")?;
        let value = i32::from_le_bytes(
            span.bytes()
                .try_into()
                .expect("four-byte map count checked above"),
        );
        if value < 0 {
            return Err(SpanError::NegativeCollectionCount {
                byte_offset: start,
                section,
                value,
            });
        }
        let count = value as usize;
        if count > self.limits.max_collection_elements {
            return Err(SpanError::CollectionLimit {
                byte_offset: start,
                section,
                count,
                limit: self.limits.max_collection_elements,
            });
        }
        Ok((count, span, end))
    }

    fn ensure_depth(
        &self,
        depth: usize,
        byte_offset: usize,
        wire_type: &str,
    ) -> Result<(), SpanError> {
        if depth > self.limits.max_depth {
            return Err(SpanError::DepthLimit {
                byte_offset,
                wire_type: wire_type.to_owned(),
                depth,
                limit: self.limits.max_depth,
            });
        }
        Ok(())
    }

    fn child_depth(
        &self,
        depth: usize,
        byte_offset: usize,
        wire_type: &str,
    ) -> Result<usize, SpanError> {
        depth.checked_add(1).ok_or_else(|| SpanError::DepthLimit {
            byte_offset,
            wire_type: wire_type.to_owned(),
            depth,
            limit: self.limits.max_depth,
        })
    }

    fn charge_nodes(&mut self, count: usize, byte_offset: usize) -> Result<(), SpanError> {
        let attempted = self.nodes.checked_add(count).ok_or(SpanError::NodeLimit {
            byte_offset,
            attempted: usize::MAX,
            limit: self.limits.max_total_nodes,
        })?;
        if attempted > self.limits.max_total_nodes {
            return Err(SpanError::NodeLimit {
                byte_offset,
                attempted,
                limit: self.limits.max_total_nodes,
            });
        }
        self.nodes = attempted;
        Ok(())
    }

    fn checked_end(&self, start: usize, width: usize, wire_type: &str) -> Result<usize, SpanError> {
        start
            .checked_add(width)
            .ok_or_else(|| SpanError::OffsetOverflow {
                byte_offset: start,
                wire_type: wire_type.to_owned(),
            })
    }

    fn slice(
        &self,
        start: usize,
        end: usize,
        wire_type: &str,
    ) -> Result<SliceSpan<'bytes>, SpanError> {
        let needed = end
            .checked_sub(start)
            .ok_or_else(|| SpanError::OffsetOverflow {
                byte_offset: start,
                wire_type: wire_type.to_owned(),
            })?;
        let bytes = self
            .bytes
            .get(start..end)
            .ok_or_else(|| SpanError::Truncated {
                byte_offset: start,
                needed,
                available: self.bytes.len().saturating_sub(start),
                wire_type: wire_type.to_owned(),
            })?;
        Ok(SliceSpan {
            offset: start,
            bytes,
        })
    }
}

fn fixed_wire_kind(inner: &PropertyInner) -> Option<FixedWireKind> {
    match inner {
        PropertyInner::Byte => Some(FixedWireKind::Byte),
        PropertyInner::Bool => Some(FixedWireKind::Bool),
        PropertyInner::Int => Some(FixedWireKind::Int32),
        PropertyInner::Float => Some(FixedWireKind::Float32),
        PropertyInner::Object => Some(FixedWireKind::PackageIndex),
        PropertyInner::Name => Some(FixedWireKind::FName),
        PropertyInner::Double => Some(FixedWireKind::Float64),
        PropertyInner::UInt64 => Some(FixedWireKind::UInt64),
        PropertyInner::UInt32 => Some(FixedWireKind::UInt32),
        PropertyInner::UInt16 => Some(FixedWireKind::UInt16),
        PropertyInner::Int64 => Some(FixedWireKind::Int64),
        PropertyInner::Int16 => Some(FixedWireKind::Int16),
        PropertyInner::Int8 => Some(FixedWireKind::Int8),
        _ => None,
    }
}

fn fixed_wire_name(kind: FixedWireKind) -> &'static str {
    match kind {
        FixedWireKind::Byte => "byte",
        FixedWireKind::Bool => "bool",
        FixedWireKind::Int32 => "int32",
        FixedWireKind::Float32 => "float32",
        FixedWireKind::PackageIndex => "package index",
        FixedWireKind::FName => "FName",
        FixedWireKind::Float64 => "float64",
        FixedWireKind::UInt64 => "uint64",
        FixedWireKind::UInt32 => "uint32",
        FixedWireKind::UInt16 => "uint16",
        FixedWireKind::Int64 => "int64",
        FixedWireKind::Int16 => "int16",
        FixedWireKind::Int8 => "int8",
        FixedWireKind::LinearColorF32x4 => "LinearColor(float32 x4)",
        FixedWireKind::Vector4F64x4 => "Vector4(float64 x4)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usmap::{ExtEatr, ExtPpth, FlagsType, StructFlags};

    fn property(name: &str, index: u16, inner: PropertyInner) -> usmap::Property {
        usmap::Property {
            name: name.to_owned(),
            array_dim: 1,
            index,
            inner,
        }
    }

    fn schema(name: &str, properties: Vec<usmap::Property>) -> usmap::Struct {
        usmap::Struct {
            name: name.to_owned(),
            super_struct: None,
            properties,
        }
    }

    fn database(entries: Vec<(usmap::Struct, FlagsType)>) -> SchemaDb {
        database_in_module(entries, "/Script/G1R")
    }

    fn database_in_module(entries: Vec<(usmap::Struct, FlagsType)>, module: &str) -> SchemaDb {
        let mut tagged: Vec<_> = entries
            .into_iter()
            .map(|(schema, kind)| (schema, kind, module.to_owned()))
            .collect();
        tagged.push((
            schema(
                "LinearColor",
                ["R", "G", "B", "A"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, name)| property(name, index as u16, PropertyInner::Float))
                    .collect(),
            ),
            FlagsType::Struct,
            "/Script/CoreUObject".to_owned(),
        ));
        tagged.push((
            schema(
                "Vector4",
                ["X", "Y", "Z", "W"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, name)| property(name, index as u16, PropertyInner::Double))
                    .collect(),
            ),
            FlagsType::Struct,
            "/Script/CoreUObject".to_owned(),
        ));

        tagged_database(tagged)
    }

    fn tagged_database(tagged: Vec<(usmap::Struct, FlagsType, String)>) -> SchemaDb {
        let module_paths = tagged.iter().map(|(_, _, module)| module.clone()).collect();
        let struct_flags = tagged
            .iter()
            .map(|(_, type_, _)| StructFlags {
                type_: *type_,
                value: 0,
                prop_flags: Vec::new(),
            })
            .collect();
        SchemaDb::from_parsed(usmap::Usmap {
            enums: Vec::new(),
            structs: tagged.into_iter().map(|(schema, _, _)| schema).collect(),
            cext: None,
            ppth: Some(ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: module_paths,
            }),
            eatr: Some(ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags,
            }),
            envp: None,
        })
        .unwrap()
    }

    fn footstep_database() -> SchemaDb {
        database(vec![
            (
                schema(
                    "FootstepTag",
                    vec![
                        property(
                            "BoneData",
                            0,
                            PropertyInner::Struct {
                                name: "BoneFeetData".to_owned(),
                            },
                        ),
                        property(
                            "BonesToTrack",
                            1,
                            PropertyInner::Map {
                                key: Box::new(PropertyInner::Name),
                                value: Box::new(PropertyInner::Struct {
                                    name: "BoneTrackedData".to_owned(),
                                }),
                            },
                        ),
                    ],
                ),
                FlagsType::Class,
            ),
            (
                schema(
                    "BoneFeetData",
                    vec![
                        property(
                            "FeetTextureSize",
                            0,
                            PropertyInner::Struct {
                                name: "Vector4".to_owned(),
                            },
                        ),
                        property("Diffuse", 1, PropertyInner::Object),
                        property("Normal", 2, PropertyInner::Object),
                        property("AO", 3, PropertyInner::Object),
                    ],
                ),
                FlagsType::Struct,
            ),
            (
                schema(
                    "BoneTrackedData",
                    vec![property("InvertX", 0, PropertyInner::Bool)],
                ),
                FlagsType::Struct,
            ),
        ])
    }

    fn footstep_bytes() -> Vec<u8> {
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
        // BoneFeetData: final four-value header, FVector4d, three package refs.
        bytes.extend_from_slice(&[0x00, 0x09]);
        bytes.extend(0u8..32);
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&3i32.to_le_bytes());
        // TMap: no removed keys, two live entries.
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&11i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        // BoneTrackedData #1: zero-masked bool.
        bytes.extend_from_slice(&[0x80, 0x03, 0x01]);
        bytes.extend_from_slice(&22i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        // BoneTrackedData #2: explicit true bool.
        bytes.extend_from_slice(&[0x00, 0x03, 0x01]);
        assert_eq!(bytes.len(), 82);
        bytes
    }

    #[test]
    fn walks_nested_headers_maps_and_fixed_structs_with_absolute_spans() {
        let db = footstep_database();
        let schema_id = db.resolve_class("/Script/G1R.FootstepTag").unwrap();
        let mut bytes = footstep_bytes();
        bytes.extend_from_slice(&[0xaa, 0xbb]);

        let block = PropertySpanWalker::g1r_ue5_4(&db)
            .walk(&bytes, schema_id)
            .unwrap();
        assert_eq!(block.schema_name(), "/Script/G1R.FootstepTag");
        assert_eq!(block.consumed(), 82);
        assert_eq!(block.span().bytes(), &bytes[..82]);
        assert_eq!(block.header().offset(), 0);
        assert_eq!(block.header().bytes(), &bytes[..6]);
        assert_eq!(block.properties().len(), 2);

        let ValueSpan::Struct(bone_data) = block.properties()[0].value().unwrap() else {
            panic!("BoneData should be a nested struct");
        };
        assert_eq!(bone_data.span().offset(), 6);
        assert_eq!(bone_data.span().end(), 52);
        assert_eq!(bone_data.properties().header().bytes(), &bytes[6..8]);
        let bone_fields = bone_data.properties().properties();
        assert_eq!(bone_fields.len(), 4);
        let ValueSpan::Fixed(vector) = bone_fields[0].value().unwrap() else {
            panic!("FeetTextureSize should be fixed");
        };
        assert_eq!(vector.kind(), FixedWireKind::Vector4F64x4);
        assert_eq!(vector.span().offset(), 8);
        assert_eq!(vector.span().end(), 40);
        for (field, expected) in bone_fields[1..].iter().zip([40, 44, 48]) {
            let ValueSpan::Fixed(reference) = field.value().unwrap() else {
                panic!("texture reference should be fixed");
            };
            assert_eq!(reference.kind(), FixedWireKind::PackageIndex);
            assert_eq!(reference.span().offset(), expected);
            assert_eq!(reference.span().len(), 4);
        }

        let ValueSpan::Map(map) = block.properties()[1].value().unwrap() else {
            panic!("BonesToTrack should be a map");
        };
        assert_eq!(map.span().offset(), 52);
        assert_eq!(map.span().end(), 82);
        assert_eq!(map.removed_count_span().offset(), 52);
        assert!(map.removed_keys().is_empty());
        assert_eq!(map.entry_count_span().offset(), 56);
        assert_eq!(map.entries().len(), 2);
        assert_eq!(map.entries()[0].span().offset(), 60);
        assert_eq!(map.entries()[0].span().end(), 71);
        assert_eq!(map.entries()[1].span().offset(), 71);
        assert_eq!(map.entries()[1].span().end(), 82);
        let ValueSpan::Fixed(first_key) = map.entries()[0].key() else {
            panic!("map key should be FName");
        };
        assert_eq!(first_key.kind(), FixedWireKind::FName);
        assert_eq!(first_key.span().bytes(), &bytes[60..68]);

        let ValueSpan::Struct(first_value) = map.entries()[0].value() else {
            panic!("map value should be BoneTrackedData");
        };
        assert_eq!(first_value.properties().header().bytes(), &bytes[68..71]);
        assert!(first_value.properties().properties()[0].is_zero());
        assert!(first_value.properties().properties()[0].value().is_none());
        let ValueSpan::Struct(second_value) = map.entries()[1].value() else {
            panic!("map value should be BoneTrackedData");
        };
        let ValueSpan::Fixed(invert_x) = second_value.properties().properties()[0].value().unwrap()
        else {
            panic!("explicit bool should have a fixed span");
        };
        assert_eq!(invert_x.kind(), FixedWireKind::Bool);
        assert_eq!(invert_x.span().offset(), 81);
        assert_eq!(invert_x.span().bytes(), &[1]);
        assert_eq!(&bytes[block.consumed()..], &[0xaa, 0xbb]);
    }

    #[test]
    fn walks_object_to_linear_color_map_without_decoding_payloads() {
        let db = database(vec![(
            schema(
                "PhysicsFixture",
                vec![property(
                    "Colors",
                    0,
                    PropertyInner::Map {
                        key: Box::new(PropertyInner::Object),
                        value: Box::new(PropertyInner::Struct {
                            name: "LinearColor".to_owned(),
                        }),
                    },
                )],
            ),
            FlagsType::Class,
        )]);
        let schema_id = db.resolve_class("PhysicsFixture").unwrap();
        let mut bytes = vec![0x00, 0x03];
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&(-7i32).to_le_bytes());
        bytes.extend_from_slice(&[0x5a; 16]);
        bytes.push(0xcc);

        let block = PropertySpanWalker::g1r_ue5_4(&db)
            .walk(&bytes, schema_id)
            .unwrap();
        assert_eq!(block.consumed(), 30);
        let ValueSpan::Map(map) = block.properties()[0].value().unwrap() else {
            panic!("Colors should be a map");
        };
        assert_eq!(map.entries().len(), 1);
        let ValueSpan::Fixed(key) = map.entries()[0].key() else {
            panic!("map key should be a package reference");
        };
        assert_eq!(key.kind(), FixedWireKind::PackageIndex);
        let ValueSpan::Fixed(color) = map.entries()[0].value() else {
            panic!("LinearColor should be a fixed native struct");
        };
        assert_eq!(color.kind(), FixedWireKind::LinearColorF32x4);
        assert_eq!(color.span().bytes(), &[0x5a; 16]);
        assert_eq!(&bytes[block.consumed()..], &[0xcc]);
    }

    #[test]
    fn fixed_native_struct_width_requires_the_exact_core_schema() {
        let db = tagged_database(vec![
            (
                schema(
                    "PhysicsFixture",
                    vec![property(
                        "Colors",
                        0,
                        PropertyInner::Map {
                            key: Box::new(PropertyInner::Object),
                            value: Box::new(PropertyInner::Struct {
                                name: "LinearColor".to_owned(),
                            }),
                        },
                    )],
                ),
                FlagsType::Class,
                "/Script/G1R".to_owned(),
            ),
            (
                schema(
                    "LinearColor",
                    ["R", "G", "B"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, name)| property(name, index as u16, PropertyInner::Float))
                        .collect(),
                ),
                FlagsType::Struct,
                "/Script/CoreUObject".to_owned(),
            ),
        ]);
        let schema_id = db.resolve_class("PhysicsFixture").unwrap();
        let mut bytes = vec![0x00, 0x03];
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        assert!(matches!(
            PropertySpanWalker::g1r_ue5_4(&db).walk(&bytes, schema_id),
            Err(SpanError::FixedStructSchema { name, .. }) if name == "LinearColor"
        ));
    }

    #[test]
    fn unsupported_nonzero_types_fail_but_zero_masked_values_need_no_wire_form() {
        let db = database(vec![(
            schema(
                "UnsupportedFixture",
                vec![property(
                    "Values",
                    0,
                    PropertyInner::Array {
                        inner: Box::new(PropertyInner::Int),
                    },
                )],
            ),
            FlagsType::Class,
        )]);
        let schema_id = db.resolve_class("UnsupportedFixture").unwrap();
        let walker = PropertySpanWalker::g1r_ue5_4(&db);

        assert!(matches!(
            walker.walk(&[0x00, 0x03], schema_id),
            Err(SpanError::UnsupportedType { byte_offset: 2, .. })
        ));

        let block = walker.walk(&[0x80, 0x03, 0x01], schema_id).unwrap();
        assert_eq!(block.consumed(), 3);
        assert!(block.properties()[0].is_zero());
        assert!(block.properties()[0].span().is_none());
    }

    #[test]
    fn fixed_and_nested_truncation_and_invalid_bool_are_precise() {
        let object_db = database(vec![(
            schema(
                "ObjectFixture",
                vec![property("Reference", 0, PropertyInner::Object)],
            ),
            FlagsType::Class,
        )]);
        let object_id = object_db.resolve_class("ObjectFixture").unwrap();
        assert!(matches!(
            PropertySpanWalker::g1r_ue5_4(&object_db).walk(&[0x00, 0x03, 0xaa, 0xbb], object_id),
            Err(SpanError::Truncated {
                byte_offset: 2,
                needed: 4,
                available: 2,
                ..
            })
        ));

        let bool_db = database(vec![(
            schema(
                "BoolFixture",
                vec![property("Enabled", 0, PropertyInner::Bool)],
            ),
            FlagsType::Class,
        )]);
        let bool_id = bool_db.resolve_class("BoolFixture").unwrap();
        assert_eq!(
            PropertySpanWalker::g1r_ue5_4(&bool_db)
                .walk(&[0x00, 0x03, 2], bool_id)
                .unwrap_err(),
            SpanError::InvalidBool {
                byte_offset: 2,
                value: 2
            }
        );

        let nested_db = footstep_database();
        let nested_id = nested_db.resolve_class("FootstepTag").unwrap();
        let error = PropertySpanWalker::g1r_ue5_4(&nested_db)
            .walk(&[0x00, 0x05, 0x00], nested_id)
            .unwrap_err();
        assert!(matches!(
            error,
            SpanError::Header {
                byte_offset: 2,
                source: UnversionedError::Truncated {
                    offset: 0,
                    what: "fragment",
                    ..
                },
                ..
            }
        ));
    }

    fn map_database() -> SchemaDb {
        database(vec![(
            schema(
                "MapFixture",
                vec![property(
                    "Values",
                    0,
                    PropertyInner::Map {
                        key: Box::new(PropertyInner::Name),
                        value: Box::new(PropertyInner::Bool),
                    },
                )],
            ),
            FlagsType::Class,
        )])
    }

    #[test]
    fn map_counts_are_signed_bounded_and_truncation_checked() {
        let db = map_database();
        let schema_id = db.resolve_class("MapFixture").unwrap();
        let walker = PropertySpanWalker::g1r_ue5_4(&db);
        let mut negative = vec![0x00, 0x03];
        negative.extend_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(
            walker.walk(&negative, schema_id).unwrap_err(),
            SpanError::NegativeCollectionCount {
                byte_offset: 2,
                section: MapSection::RemovedKeys,
                value: -1
            }
        );

        assert!(matches!(
            walker.walk(&[0x00, 0x03, 0, 0, 0], schema_id),
            Err(SpanError::Truncated {
                byte_offset: 2,
                needed: 4,
                available: 3,
                ..
            })
        ));

        let limited = PropertySpanWalker::g1r_ue5_4_with_limits(
            &db,
            SpanLimits {
                max_collection_elements: 1,
                ..SpanLimits::default()
            },
        );
        let mut two_entries = vec![0x00, 0x03];
        two_entries.extend_from_slice(&0i32.to_le_bytes());
        two_entries.extend_from_slice(&2i32.to_le_bytes());
        assert_eq!(
            limited.walk(&two_entries, schema_id).unwrap_err(),
            SpanError::CollectionLimit {
                byte_offset: 6,
                section: MapSection::Entries,
                count: 2,
                limit: 1
            }
        );

        let tiny_tree = PropertySpanWalker::g1r_ue5_4_with_limits(
            &db,
            SpanLimits {
                max_total_nodes: 3,
                ..SpanLimits::default()
            },
        );
        let mut removed_keys = vec![0x00, 0x03];
        removed_keys.extend_from_slice(&2i32.to_le_bytes());
        assert_eq!(
            tiny_tree.walk(&removed_keys, schema_id).unwrap_err(),
            SpanError::NodeLimit {
                byte_offset: 6,
                attempted: 4,
                limit: 3
            }
        );
    }

    #[test]
    fn depth_and_total_node_limits_bound_recursive_work() {
        let db = footstep_database();
        let schema_id = db.resolve_class("FootstepTag").unwrap();
        let bytes = footstep_bytes();
        let shallow = PropertySpanWalker::g1r_ue5_4_with_limits(
            &db,
            SpanLimits {
                max_depth: 0,
                ..SpanLimits::default()
            },
        );
        assert!(matches!(
            shallow.walk(&bytes, schema_id),
            Err(SpanError::DepthLimit {
                byte_offset: 6,
                depth: 1,
                limit: 0,
                ..
            })
        ));

        let tiny_tree = PropertySpanWalker::g1r_ue5_4_with_limits(
            &db,
            SpanLimits {
                max_total_nodes: 1,
                ..SpanLimits::default()
            },
        );
        assert_eq!(
            tiny_tree.walk(&bytes, schema_id).unwrap_err(),
            SpanError::NodeLimit {
                byte_offset: 0,
                attempted: 3,
                limit: 1
            }
        );
    }

    #[test]
    fn unknown_structs_are_not_assigned_a_guessed_size() {
        let db = database(vec![(
            schema(
                "StructFixture",
                vec![property(
                    "Transform",
                    0,
                    PropertyInner::Struct {
                        name: "Transform".to_owned(),
                    },
                )],
            ),
            FlagsType::Class,
        )]);
        let schema_id = db.resolve_class("StructFixture").unwrap();
        assert_eq!(
            PropertySpanWalker::g1r_ue5_4(&db)
                .walk(&[0x00, 0x03], schema_id)
                .unwrap_err(),
            SpanError::UnsupportedStruct {
                byte_offset: 2,
                name: "Transform".to_owned()
            }
        );
    }

    #[test]
    fn named_nested_structs_must_resolve_to_the_proven_g1r_schema() {
        let db = database_in_module(
            vec![
                (
                    schema(
                        "Root",
                        vec![property(
                            "BoneData",
                            0,
                            PropertyInner::Struct {
                                name: "BoneFeetData".to_owned(),
                            },
                        )],
                    ),
                    FlagsType::Class,
                ),
                (schema("BoneFeetData", Vec::new()), FlagsType::Struct),
            ],
            "/Script/Foreign",
        );
        let schema_id = db.resolve_class("Root").unwrap();
        assert_eq!(
            PropertySpanWalker::g1r_ue5_4(&db)
                .walk(&[0x00, 0x03], schema_id)
                .unwrap_err(),
            SpanError::NestedSchemaModule {
                name: "BoneFeetData".to_owned(),
                actual: Some("/Script/Foreign".to_owned())
            }
        );
    }

    #[test]
    #[ignore = "requires GORE_USMAP and GORE_ASSET_FIXTURE_DIR for locally extracted G1R files"]
    fn walks_three_live_primary_dataasset_fixtures_to_the_proven_suffix() {
        use std::path::PathBuf;

        use crate::{LegacyPackageEnvelope, PackageCarrier, PackageLimits};

        let usmap = PathBuf::from(std::env::var_os("GORE_USMAP").expect("GORE_USMAP is required"));
        let fixture_dir = PathBuf::from(
            std::env::var_os("GORE_ASSET_FIXTURE_DIR").expect("GORE_ASSET_FIXTURE_DIR is required"),
        );
        let db = SchemaDb::from_usmap(&std::fs::read(usmap).unwrap()).unwrap();
        let walker = PropertySpanWalker::g1r_ue5_4(&db);
        let cases = [
            (
                "DA_PhysicsMaterialColor",
                "/Script/G1R.PhysicMaterialsColor",
                290,
                294,
            ),
            ("DA_HumanFootsteps", "/Script/G1R.FootstepTag", 82, 86),
            ("DA_WolfFootsteps", "/Script/G1R.FootstepTag", 82, 86),
        ];

        for (stem, schema_name, property_end, export_end) in cases {
            let carrier = PackageCarrier::load(
                fixture_dir.join(stem).join(format!("{stem}.uasset")),
                PackageLimits::default(),
            )
            .unwrap();
            let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).unwrap();
            let export = package.export(0).unwrap();
            assert_eq!(export.bytes().len(), export_end);
            assert_eq!(export.boundary().class_path(), schema_name);
            let schema_id = export.boundary().resolve_class_schema(&db).unwrap();
            let block = walker.walk(export.bytes(), schema_id).unwrap();
            assert_eq!(block.header().len(), 6);
            assert_eq!(block.consumed(), property_end);
            let segments = export.split_decoded_prefix(block.consumed()).unwrap();
            assert_eq!(segments.decoded_prefix, &export.bytes()[..property_end]);
            assert_eq!(segments.native_suffix, &[0, 0, 0, 0]);
        }
    }
}
