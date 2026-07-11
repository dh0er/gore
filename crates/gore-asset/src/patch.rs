//! Snapshot-bound, compare-and-swap edits for proven fixed-width property leaves.
//!
//! A patch is planned from an exact legacy export, its exact USMAP schema, and
//! a leaf returned by [`PropertySpanWalker`]. The owned plan seals both package
//! components, then reparses and rewalks the package immediately before and
//! after mutation. It never accepts a caller-supplied absolute offset.

use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;
use usmap::PropertyInner;

use crate::{
    EnvelopeError, ExportEnvelope, ExportSchemaError, FixedValueSpan, FixedWireKind,
    LegacyPackageEnvelope, PackageCarrier, PackageComponent, PackageError, PropertyBlockSpans,
    PropertySlot, PropertySpanWalker, SchemaDb, SchemaError, SliceSpan, SpanError, ValueSpan,
};

/// SHA-256 identity of both components at one point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePairSeal {
    pub uasset_sha256: [u8; 32],
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
    #[error("the supplied property block and leaf do not match a fresh walk with the export's exact schema")]
    SemanticPathMismatch,
    #[error("the fixed leaf is not the unique matching leaf in the supplied property block (matches={matching_leaves})")]
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
    #[error("expected bytes differ from the observed leaf at relative byte {mismatch_offset}: expected 0x{expected:02x}, got 0x{actual:02x}")]
    ExpectedMismatch {
        mismatch_offset: usize,
        expected: u8,
        actual: u8,
    },
    #[error("editing referential fixed wire kind {kind:?} is refused until package-map validation is available")]
    ReferentialEditUnsupported { kind: FixedWireKind },
    #[error("editing a fixed leaf inside {section} is refused; map key identity is not a value-only patch")]
    MapKeyEditUnsupported { section: &'static str },
    #[error("editing a map value with a schema-recursive struct or map key is refused until the key's semantic schema can be sealed")]
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
    #[error("postcondition failed ({postcondition}); rollback completed but pair verification failed: expected {expected}; got {actual}")]
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
    if matches!(kind, FixedWireKind::FName | FixedWireKind::PackageIndex) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixedLeafRole {
    PropertyValue,
    MapKey,
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
