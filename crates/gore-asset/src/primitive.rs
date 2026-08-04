//! Strict primitive payload codec for unversioned UObject properties.
//!
//! Only top-level property kinds with a schema-defined fixed width are decoded.
//! Variable-width and context-sensitive kinds are never skipped heuristically:
//! a non-zero value of such a kind returns [`PrimitiveError::UnsupportedType`].
//! A zero-mask entry is always safe because Unreal stores no payload bytes for
//! it, so even an otherwise unsupported complex property can be retained as
//! [`PropertyPayload::OmittedZero`].

use std::fmt;

use crate::{HeaderEntry, PropertySlot, UnversionedError, UnversionedHeader};
use thiserror::Error;
use usmap::PropertyInner;

/// Fixed-width top-level USMAP property kinds supported by this codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Byte,
    Bool,
    Int,
    Float,
    Double,
    UInt64,
    UInt32,
    UInt16,
    Int64,
    Int16,
    Int8,
}

impl PrimitiveKind {
    pub fn from_inner(inner: &PropertyInner) -> Option<Self> {
        match inner {
            PropertyInner::Byte => Some(Self::Byte),
            PropertyInner::Bool => Some(Self::Bool),
            PropertyInner::Int => Some(Self::Int),
            PropertyInner::Float => Some(Self::Float),
            PropertyInner::Double => Some(Self::Double),
            PropertyInner::UInt64 => Some(Self::UInt64),
            PropertyInner::UInt32 => Some(Self::UInt32),
            PropertyInner::UInt16 => Some(Self::UInt16),
            PropertyInner::Int64 => Some(Self::Int64),
            PropertyInner::Int16 => Some(Self::Int16),
            PropertyInner::Int8 => Some(Self::Int8),
            _ => None,
        }
    }

    pub fn width(self) -> usize {
        match self {
            Self::Byte | Self::Bool | Self::Int8 => 1,
            Self::UInt16 | Self::Int16 => 2,
            Self::Int | Self::Float | Self::UInt32 => 4,
            Self::Double | Self::UInt64 | Self::Int64 => 8,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Byte => "Byte",
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Float => "Float",
            Self::Double => "Double",
            Self::UInt64 => "UInt64",
            Self::UInt32 => "UInt32",
            Self::UInt16 => "UInt16",
            Self::Int64 => "Int64",
            Self::Int16 => "Int16",
            Self::Int8 => "Int8",
        }
    }
}

impl fmt::Display for PrimitiveKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One decoded fixed-width value.
///
/// Float equality is bitwise, retaining signed zeroes and NaN payloads across
/// edits to other properties.
#[derive(Debug, Clone, Copy)]
pub enum PrimitiveValue {
    Byte(u8),
    Bool(bool),
    Int(i32),
    Float(f32),
    Double(f64),
    UInt64(u64),
    UInt32(u32),
    UInt16(u16),
    Int64(i64),
    Int16(i16),
    Int8(i8),
}

impl PrimitiveValue {
    pub fn kind(self) -> PrimitiveKind {
        match self {
            Self::Byte(_) => PrimitiveKind::Byte,
            Self::Bool(_) => PrimitiveKind::Bool,
            Self::Int(_) => PrimitiveKind::Int,
            Self::Float(_) => PrimitiveKind::Float,
            Self::Double(_) => PrimitiveKind::Double,
            Self::UInt64(_) => PrimitiveKind::UInt64,
            Self::UInt32(_) => PrimitiveKind::UInt32,
            Self::UInt16(_) => PrimitiveKind::UInt16,
            Self::Int64(_) => PrimitiveKind::Int64,
            Self::Int16(_) => PrimitiveKind::Int16,
            Self::Int8(_) => PrimitiveKind::Int8,
        }
    }

    /// Materialize the logical value represented by a zero-mask entry when its
    /// schema kind is supported. This does not change the entry's omitted state.
    pub fn zero_for(inner: &PropertyInner) -> Option<Self> {
        Some(match PrimitiveKind::from_inner(inner)? {
            PrimitiveKind::Byte => Self::Byte(0),
            PrimitiveKind::Bool => Self::Bool(false),
            PrimitiveKind::Int => Self::Int(0),
            PrimitiveKind::Float => Self::Float(0.0),
            PrimitiveKind::Double => Self::Double(0.0),
            PrimitiveKind::UInt64 => Self::UInt64(0),
            PrimitiveKind::UInt32 => Self::UInt32(0),
            PrimitiveKind::UInt16 => Self::UInt16(0),
            PrimitiveKind::Int64 => Self::Int64(0),
            PrimitiveKind::Int16 => Self::Int16(0),
            PrimitiveKind::Int8 => Self::Int8(0),
        })
    }
}

impl PartialEq for PrimitiveValue {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Self::Byte(left), Self::Byte(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left.to_bits() == right.to_bits(),
            (Self::Double(left), Self::Double(right)) => left.to_bits() == right.to_bits(),
            (Self::UInt64(left), Self::UInt64(right)) => left == right,
            (Self::UInt32(left), Self::UInt32(right)) => left == right,
            (Self::UInt16(left), Self::UInt16(right)) => left == right,
            (Self::Int64(left), Self::Int64(right)) => left == right,
            (Self::Int16(left), Self::Int16(right)) => left == right,
            (Self::Int8(left), Self::Int8(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for PrimitiveValue {}

/// Serialized state of a selected schema property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPayload {
    /// The header's zero-mask bit is set and no payload bytes are present.
    OmittedZero,
    /// The header marks the property non-zero and these bytes are serialized.
    /// The contained numeric value is deliberately not auto-collapsed to zero.
    Value(PrimitiveValue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveProperty {
    pub schema_index: usize,
    pub payload: PropertyPayload,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrimitiveError {
    #[error(transparent)]
    Header(#[from] UnversionedError),
    #[error(
        "unsupported non-zero property {property_path:?} at schema slot {schema_index}: {property_type}; its payload length is not inferred"
    )]
    UnsupportedType {
        schema_index: usize,
        property_path: String,
        property_type: String,
    },
    #[error(
        "property {property_path:?} at schema slot {schema_index} is truncated at byte {byte_offset}: {property_type} needs {needed} bytes, only {available} remain"
    )]
    Truncated {
        schema_index: usize,
        property_path: String,
        property_type: String,
        byte_offset: usize,
        needed: usize,
        available: usize,
    },
    #[error(
        "property {property_path:?} at schema slot {schema_index} has invalid Bool byte {value} at byte {byte_offset}; expected 0 or 1"
    )]
    InvalidBool {
        schema_index: usize,
        property_path: String,
        byte_offset: usize,
        value: u8,
    },
    #[error(
        "property {property_path:?} at schema slot {schema_index} expects {expected}, but received {actual}"
    )]
    TypeMismatch {
        schema_index: usize,
        property_path: String,
        expected: PrimitiveKind,
        actual: PrimitiveKind,
    },
}

/// A fully decoded unversioned property block whose non-zero entries are all
/// fixed-width primitives.
///
/// Decoding retains the original header and payload bytes. [`Self::encode`]
/// returns them verbatim until a property is changed. Unsupported complex
/// properties are retained only when zero-masked; a non-zero complex value
/// fails before any length is guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitivePropertyBlock {
    schema_slot_count: usize,
    properties: Vec<PrimitiveProperty>,
    original_encoding: Option<Vec<u8>>,
}

impl PrimitivePropertyBlock {
    pub fn empty(schema_slot_count: usize) -> Self {
        Self {
            schema_slot_count,
            properties: Vec::new(),
            original_encoding: None,
        }
    }

    /// Decode a property block at the start of `bytes` using a flattened schema.
    /// The returned count is the exact first byte after the primitive payloads.
    pub fn decode(bytes: &[u8], slots: &[PropertySlot]) -> Result<(Self, usize), PrimitiveError> {
        let (header, header_len) = UnversionedHeader::decode(bytes, slots.len())?;
        let resolved = header.resolve_slots(slots)?;
        let mut offset = header_len;
        let mut properties = Vec::with_capacity(resolved.len());

        for entry in resolved {
            let payload = if entry.is_zero {
                PropertyPayload::OmittedZero
            } else {
                let (value, consumed) = decode_value(bytes, offset, entry.slot)?;
                offset += consumed;
                PropertyPayload::Value(value)
            };
            properties.push(PrimitiveProperty {
                schema_index: entry.schema_index,
                payload,
            });
        }

        Ok((
            Self {
                schema_slot_count: slots.len(),
                properties,
                original_encoding: Some(bytes[..offset].to_vec()),
            },
            offset,
        ))
    }

    pub fn schema_slot_count(&self) -> usize {
        self.schema_slot_count
    }

    pub fn properties(&self) -> &[PrimitiveProperty] {
        &self.properties
    }

    pub fn property(&self, schema_index: usize) -> Option<&PrimitiveProperty> {
        self.properties
            .binary_search_by_key(&schema_index, |property| property.schema_index)
            .ok()
            .map(|position| &self.properties[position])
    }

    /// Insert or replace a selected property. Complex schema kinds may only be
    /// inserted as [`PropertyPayload::OmittedZero`].
    pub fn set(
        &mut self,
        slots: &[PropertySlot],
        schema_index: usize,
        payload: PropertyPayload,
    ) -> Result<Option<PrimitiveProperty>, PrimitiveError> {
        validate_slots(self.schema_slot_count, slots)?;
        let slot = slots.get(schema_index).ok_or({
            PrimitiveError::Header(UnversionedError::SchemaIndexOutOfRange {
                schema_index,
                schema_slot_count: self.schema_slot_count,
            })
        })?;
        validate_payload(slot, payload)?;
        let replacement = PrimitiveProperty {
            schema_index,
            payload,
        };
        match self
            .properties
            .binary_search_by_key(&schema_index, |property| property.schema_index)
        {
            Ok(position) => {
                let previous = self.properties[position];
                if previous != replacement {
                    self.properties[position] = replacement;
                    self.original_encoding = None;
                }
                Ok(Some(previous))
            }
            Err(position) => {
                self.properties.insert(position, replacement);
                self.original_encoding = None;
                Ok(None)
            }
        }
    }

    pub fn remove(&mut self, schema_index: usize) -> Option<PrimitiveProperty> {
        let position = self
            .properties
            .binary_search_by_key(&schema_index, |property| property.schema_index)
            .ok()?;
        self.original_encoding = None;
        Some(self.properties.remove(position))
    }

    /// Encode this block against the same flattened schema shape.
    pub fn encode(&self, slots: &[PropertySlot]) -> Result<Vec<u8>, PrimitiveError> {
        validate_slots(self.schema_slot_count, slots)?;
        for property in &self.properties {
            let slot = &slots[property.schema_index];
            validate_payload(slot, property.payload)?;
        }
        if let Some(original) = &self.original_encoding {
            return Ok(original.clone());
        }

        let header = UnversionedHeader::from_entries(
            self.schema_slot_count,
            self.properties.iter().map(|property| HeaderEntry {
                schema_index: property.schema_index,
                is_zero: property.payload == PropertyPayload::OmittedZero,
            }),
        )?;
        let mut bytes = header.encode_canonical()?;
        for property in &self.properties {
            if let PropertyPayload::Value(value) = property.payload {
                encode_value(&mut bytes, &slots[property.schema_index], value)?;
            }
        }
        Ok(bytes)
    }
}

fn decode_value(
    bytes: &[u8],
    offset: usize,
    slot: &PropertySlot,
) -> Result<(PrimitiveValue, usize), PrimitiveError> {
    let kind = primitive_kind(slot)?;
    let width = kind.width();
    let raw = bytes
        .get(offset..offset.saturating_add(width))
        .ok_or_else(|| PrimitiveError::Truncated {
            schema_index: slot.schema_index,
            property_path: slot.path(),
            property_type: kind.to_string(),
            byte_offset: offset,
            needed: width,
            available: bytes.len().saturating_sub(offset),
        })?;

    let value = match kind {
        PrimitiveKind::Byte => PrimitiveValue::Byte(raw[0]),
        PrimitiveKind::Bool => match raw[0] {
            0 => PrimitiveValue::Bool(false),
            1 => PrimitiveValue::Bool(true),
            value => {
                return Err(PrimitiveError::InvalidBool {
                    schema_index: slot.schema_index,
                    property_path: slot.path(),
                    byte_offset: offset,
                    value,
                });
            }
        },
        PrimitiveKind::Int => PrimitiveValue::Int(i32::from_le_bytes(array(raw))),
        PrimitiveKind::Float => {
            PrimitiveValue::Float(f32::from_bits(u32::from_le_bytes(array(raw))))
        }
        PrimitiveKind::Double => {
            PrimitiveValue::Double(f64::from_bits(u64::from_le_bytes(array(raw))))
        }
        PrimitiveKind::UInt64 => PrimitiveValue::UInt64(u64::from_le_bytes(array(raw))),
        PrimitiveKind::UInt32 => PrimitiveValue::UInt32(u32::from_le_bytes(array(raw))),
        PrimitiveKind::UInt16 => PrimitiveValue::UInt16(u16::from_le_bytes(array(raw))),
        PrimitiveKind::Int64 => PrimitiveValue::Int64(i64::from_le_bytes(array(raw))),
        PrimitiveKind::Int16 => PrimitiveValue::Int16(i16::from_le_bytes(array(raw))),
        PrimitiveKind::Int8 => PrimitiveValue::Int8(i8::from_le_bytes(array(raw))),
    };
    Ok((value, width))
}

fn encode_value(
    out: &mut Vec<u8>,
    slot: &PropertySlot,
    value: PrimitiveValue,
) -> Result<(), PrimitiveError> {
    let expected = primitive_kind(slot)?;
    let actual = value.kind();
    if actual != expected {
        return Err(PrimitiveError::TypeMismatch {
            schema_index: slot.schema_index,
            property_path: slot.path(),
            expected,
            actual,
        });
    }
    match value {
        PrimitiveValue::Byte(value) => out.push(value),
        PrimitiveValue::Bool(value) => out.push(u8::from(value)),
        PrimitiveValue::Int(value) => out.extend_from_slice(&value.to_le_bytes()),
        PrimitiveValue::Float(value) => out.extend_from_slice(&value.to_bits().to_le_bytes()),
        PrimitiveValue::Double(value) => out.extend_from_slice(&value.to_bits().to_le_bytes()),
        PrimitiveValue::UInt64(value) => out.extend_from_slice(&value.to_le_bytes()),
        PrimitiveValue::UInt32(value) => out.extend_from_slice(&value.to_le_bytes()),
        PrimitiveValue::UInt16(value) => out.extend_from_slice(&value.to_le_bytes()),
        PrimitiveValue::Int64(value) => out.extend_from_slice(&value.to_le_bytes()),
        PrimitiveValue::Int16(value) => out.extend_from_slice(&value.to_le_bytes()),
        PrimitiveValue::Int8(value) => out.extend_from_slice(&value.to_le_bytes()),
    }
    Ok(())
}

fn primitive_kind(slot: &PropertySlot) -> Result<PrimitiveKind, PrimitiveError> {
    PrimitiveKind::from_inner(&slot.inner).ok_or_else(|| PrimitiveError::UnsupportedType {
        schema_index: slot.schema_index,
        property_path: slot.path(),
        property_type: format!("{:?}", slot.inner),
    })
}

fn validate_payload(slot: &PropertySlot, payload: PropertyPayload) -> Result<(), PrimitiveError> {
    let PropertyPayload::Value(value) = payload else {
        return Ok(());
    };
    let expected = primitive_kind(slot)?;
    let actual = value.kind();
    if actual != expected {
        return Err(PrimitiveError::TypeMismatch {
            schema_index: slot.schema_index,
            property_path: slot.path(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn validate_slots(expected_count: usize, slots: &[PropertySlot]) -> Result<(), PrimitiveError> {
    if slots.len() != expected_count {
        return Err(PrimitiveError::Header(
            UnversionedError::SlotCountMismatch {
                expected: expected_count,
                actual: slots.len(),
            },
        ));
    }
    for (position, slot) in slots.iter().enumerate() {
        if slot.schema_index != position {
            return Err(PrimitiveError::Header(
                UnversionedError::SlotLayoutMismatch {
                    position,
                    schema_index: slot.schema_index,
                },
            ));
        }
    }
    Ok(())
}

fn array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes.try_into().expect("caller checked primitive width")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots(inners: Vec<PropertyInner>) -> Vec<PropertySlot> {
        inners
            .into_iter()
            .enumerate()
            .map(|(schema_index, inner)| PropertySlot {
                schema_index,
                property_name: format!("P{schema_index}"),
                array_index: 0,
                array_dimension: 1,
                inner,
                declaring_schema_id: 0,
                declaring_schema_name: "Fixture".into(),
                declaring_module_path: Some("/Script/Test".into()),
            })
            .collect()
    }

    fn value(schema_index: usize, value: PrimitiveValue) -> PrimitiveProperty {
        PrimitiveProperty {
            schema_index,
            payload: PropertyPayload::Value(value),
        }
    }

    #[test]
    fn decodes_every_fixed_primitive_and_preserves_exact_bytes() {
        let slots = slots(vec![
            PropertyInner::Bool,
            PropertyInner::Byte,
            PropertyInner::Int,
            PropertyInner::Float,
            PropertyInner::Double,
            PropertyInner::UInt64,
            PropertyInner::UInt32,
            PropertyInner::UInt16,
            PropertyInner::Int64,
            PropertyInner::Int16,
            PropertyInner::Int8,
            PropertyInner::Struct {
                name: "ComplexZero".into(),
            },
        ]);
        // keep 12, last, has zero mask; bit 11 marks the complex entry zero.
        let mut bytes = vec![0x80, 0x19, 0x00, 0x08];
        bytes.push(1);
        bytes.push(0xab);
        bytes.extend_from_slice(&(-12_345i32).to_le_bytes());
        bytes.extend_from_slice(&0x7fc0_1234u32.to_le_bytes());
        bytes.extend_from_slice(&0x7ff8_0000_0000_1234u64.to_le_bytes());
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());
        bytes.extend_from_slice(&0x89ab_cdefu32.to_le_bytes());
        bytes.extend_from_slice(&0xbeefu16.to_le_bytes());
        bytes.extend_from_slice(&i64::MIN.to_le_bytes());
        bytes.extend_from_slice(&(-12_345i16).to_le_bytes());
        bytes.extend_from_slice(&(-101i8).to_le_bytes());
        let block_len = bytes.len();
        bytes.extend_from_slice(&[0xcc, 0xdd]);

        let (block, consumed) = PrimitivePropertyBlock::decode(&bytes, &slots).unwrap();
        assert_eq!(consumed, block_len);
        assert_eq!(
            block.properties(),
            [
                value(0, PrimitiveValue::Bool(true)),
                value(1, PrimitiveValue::Byte(0xab)),
                value(2, PrimitiveValue::Int(-12_345)),
                value(3, PrimitiveValue::Float(f32::from_bits(0x7fc0_1234))),
                value(
                    4,
                    PrimitiveValue::Double(f64::from_bits(0x7ff8_0000_0000_1234))
                ),
                value(5, PrimitiveValue::UInt64(u64::MAX)),
                value(6, PrimitiveValue::UInt32(0x89ab_cdef)),
                value(7, PrimitiveValue::UInt16(0xbeef)),
                value(8, PrimitiveValue::Int64(i64::MIN)),
                value(9, PrimitiveValue::Int16(-12_345)),
                value(10, PrimitiveValue::Int8(-101)),
                PrimitiveProperty {
                    schema_index: 11,
                    payload: PropertyPayload::OmittedZero,
                },
            ]
        );
        assert_eq!(block.encode(&slots).unwrap(), bytes[..block_len]);
    }

    #[test]
    fn edited_block_round_trips_and_keeps_explicit_zero_state() {
        let slots = slots(vec![
            PropertyInner::Bool,
            PropertyInner::Int,
            PropertyInner::Struct {
                name: "Opaque".into(),
            },
        ]);
        let mut block = PrimitivePropertyBlock::empty(slots.len());
        block
            .set(
                &slots,
                0,
                PropertyPayload::Value(PrimitiveValue::Bool(true)),
            )
            .unwrap();
        block
            .set(&slots, 1, PropertyPayload::Value(PrimitiveValue::Int(0)))
            .unwrap();
        block.set(&slots, 2, PropertyPayload::OmittedZero).unwrap();

        let encoded = block.encode(&slots).unwrap();
        let (decoded, consumed) = PrimitivePropertyBlock::decode(&encoded, &slots).unwrap();
        assert_eq!(consumed, encoded.len());
        // Int(0) remains an explicitly serialized value; it is not guessed into
        // the zero mask. The complex property remains safely zero-masked.
        assert_eq!(
            decoded.property(1).unwrap().payload,
            PropertyPayload::Value(PrimitiveValue::Int(0))
        );
        assert_eq!(
            decoded.property(2).unwrap().payload,
            PropertyPayload::OmittedZero
        );
    }

    #[test]
    fn editing_another_property_preserves_float_bits() {
        let slots = slots(vec![PropertyInner::Float, PropertyInner::Int]);
        // keep 2, last, no mask
        let mut source = vec![0x00, 0x05];
        source.extend_from_slice(&0x7fc0_1234u32.to_le_bytes());
        source.extend_from_slice(&1i32.to_le_bytes());
        let (mut block, _) = PrimitivePropertyBlock::decode(&source, &slots).unwrap();
        block
            .set(&slots, 1, PropertyPayload::Value(PrimitiveValue::Int(2)))
            .unwrap();

        let encoded = block.encode(&slots).unwrap();
        let (decoded, _) = PrimitivePropertyBlock::decode(&encoded, &slots).unwrap();
        assert_eq!(
            decoded.property(0).unwrap().payload,
            PropertyPayload::Value(PrimitiveValue::Float(f32::from_bits(0x7fc0_1234)))
        );
    }

    #[test]
    fn unsupported_nonzero_types_fail_without_guessing_a_length() {
        for inner in [
            PropertyInner::Name,
            PropertyInner::Str,
            PropertyInner::Object,
            PropertyInner::Array {
                inner: Box::new(PropertyInner::Int),
            },
            PropertyInner::Struct {
                name: "Vector".into(),
            },
        ] {
            let slots = slots(vec![inner]);
            // keep 1, last, unmasked, followed by bytes whose meaning is unknown.
            let error =
                PrimitivePropertyBlock::decode(&[0x00, 0x03, 1, 2, 3, 4], &slots).unwrap_err();
            assert!(matches!(
                error,
                PrimitiveError::UnsupportedType {
                    schema_index: 0,
                    ..
                }
            ));
        }
    }

    #[test]
    fn zero_masked_complex_type_is_losslessly_safe() {
        let slots = slots(vec![PropertyInner::Map {
            key: Box::new(PropertyInner::Name),
            value: Box::new(PropertyInner::Struct {
                name: "Anything".into(),
            }),
        }]);
        // keep 1, last, masked; bit 0 = zero. No property payload follows.
        let source = [0x80, 0x03, 0x01];
        let (block, consumed) = PrimitivePropertyBlock::decode(&source, &slots).unwrap();
        assert_eq!(consumed, source.len());
        assert_eq!(
            block.property(0).unwrap().payload,
            PropertyPayload::OmittedZero
        );
        assert_eq!(block.encode(&slots).unwrap(), source);
    }

    #[test]
    fn truncation_and_invalid_bool_report_exact_property_and_offset() {
        let double_slots = slots(vec![PropertyInner::Double]);
        let error =
            PrimitivePropertyBlock::decode(&[0x00, 0x03, 1, 2, 3, 4], &double_slots).unwrap_err();
        assert!(matches!(
            error,
            PrimitiveError::Truncated {
                schema_index: 0,
                byte_offset: 2,
                needed: 8,
                available: 4,
                ..
            }
        ));

        let bool_slots = slots(vec![PropertyInner::Bool]);
        assert!(matches!(
            PrimitivePropertyBlock::decode(&[0x00, 0x03, 2], &bool_slots),
            Err(PrimitiveError::InvalidBool {
                schema_index: 0,
                byte_offset: 2,
                value: 2,
                ..
            })
        ));
    }

    #[test]
    fn mutation_rejects_wrong_value_kind_and_nonzero_complex_value() {
        let slots = slots(vec![
            PropertyInner::UInt16,
            PropertyInner::Struct {
                name: "Opaque".into(),
            },
        ]);
        let mut block = PrimitivePropertyBlock::empty(slots.len());
        assert!(matches!(
            block.set(&slots, 0, PropertyPayload::Value(PrimitiveValue::Int16(7))),
            Err(PrimitiveError::TypeMismatch {
                expected: PrimitiveKind::UInt16,
                actual: PrimitiveKind::Int16,
                ..
            })
        ));
        assert!(matches!(
            block.set(&slots, 1, PropertyPayload::Value(PrimitiveValue::Int(7))),
            Err(PrimitiveError::UnsupportedType {
                schema_index: 1,
                ..
            })
        ));
    }

    #[test]
    fn zero_materialization_is_typed_but_does_not_cover_complex_kinds() {
        assert_eq!(
            PrimitiveValue::zero_for(&PropertyInner::Bool),
            Some(PrimitiveValue::Bool(false))
        );
        assert_eq!(
            PrimitiveValue::zero_for(&PropertyInner::Float),
            Some(PrimitiveValue::Float(0.0))
        );
        assert_eq!(PrimitiveValue::zero_for(&PropertyInner::Name), None);
    }
}
