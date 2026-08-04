//! Codec for Unreal's unversioned-property header.
//!
//! The header is a stream of little-endian 16-bit skip/value fragments followed
//! by one packed zero mask. Property payloads immediately follow the header;
//! entries marked zero consume no payload bytes. Decoding therefore gives a
//! generic asset editor the schema slots it must visit and the exact payload
//! start without interpreting any property values yet.
//!
//! Empty non-final fragments are legal. UE5.4 can emit them at the beginning of
//! real UObject property streams; they still advance the input by two bytes and
//! are bounded by the fragment-count limit.

use crate::schema::PropertySlot;
use thiserror::Error;

const SKIP_MASK: u16 = 0x007f;
const HAS_ZERO_MASK: u16 = 0x0080;
const IS_LAST_MASK: u16 = 0x0100;
const VALUE_SHIFT: u32 = 9;
const FRAGMENT_FIELD_MAX: usize = 127;

// A real reflected class is many orders of magnitude smaller. This prevents a
// malicious stream made entirely of tiny fragments from causing unbounded work.
const MAX_HEADER_FRAGMENTS: usize = 1 << 16;
const MAX_HEADER_ENTRIES: usize = 1 << 20;

/// A schema slot selected by an unversioned-property header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderEntry {
    pub schema_index: usize,
    /// Unreal omits the property payload when its zero-mask bit is set.
    pub is_zero: bool,
}

/// A header entry joined to the flattened USMAP slot it addresses.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedHeaderEntry<'a> {
    pub schema_index: usize,
    pub is_zero: bool,
    pub slot: &'a PropertySlot,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UnversionedError {
    #[error(
        "unversioned header is truncated at byte {offset} while reading {what}: need {needed} bytes, have {available}"
    )]
    Truncated {
        offset: usize,
        needed: usize,
        available: usize,
        what: &'static str,
    },
    #[error("unversioned header exceeds the {limit}-fragment safety limit")]
    TooManyFragments { limit: usize },
    #[error("unversioned header exceeds the {limit}-entry safety limit")]
    TooManyEntries { limit: usize },
    #[error("unversioned fragment {fragment} overflows its schema index")]
    SchemaIndexOverflow { fragment: usize },
    #[error(
        "unversioned fragment {fragment} reaches schema slot {end}, but the schema has only {schema_slot_count} slots"
    )]
    FragmentOutOfRange {
        fragment: usize,
        end: usize,
        schema_slot_count: usize,
    },
    #[error(
        "unversioned header references schema slot {schema_index}, but the schema has only {schema_slot_count} slots"
    )]
    SchemaIndexOutOfRange {
        schema_index: usize,
        schema_slot_count: usize,
    },
    #[error("unversioned header contains schema slot {schema_index} more than once")]
    DuplicateSchemaIndex { schema_index: usize },
    #[error("unversioned zero-mask size overflow")]
    ZeroMaskSizeOverflow,
    #[error(
        "flattened schema has {actual} slots, but this unversioned header was decoded for {expected} slots"
    )]
    SlotCountMismatch { expected: usize, actual: usize },
    #[error(
        "flattened schema position {position} contains schema index {schema_index}; expected {position}"
    )]
    SlotLayoutMismatch {
        position: usize,
        schema_index: usize,
    },
    #[error("bounded unversioned-header decoding exhausted {resource}")]
    ResourceLimit { resource: &'static str },
    #[error("bounded unversioned-header decoding could not reserve proven storage")]
    Allocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderDecodeLimits {
    pub max_fragments: usize,
    pub max_entries: usize,
    pub max_work: usize,
    pub max_allocation_bytes: usize,
    pub max_byte_work: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeaderDecodeUsage {
    pub fragments: usize,
    pub entries: usize,
    pub work: usize,
    pub allocation_bytes: usize,
    pub byte_work: usize,
}

#[derive(Debug)]
pub struct HeaderDecodeBudget {
    limits: HeaderDecodeLimits,
    usage: HeaderDecodeUsage,
}

impl HeaderDecodeBudget {
    pub fn new(limits: HeaderDecodeLimits) -> Self {
        Self {
            limits,
            usage: HeaderDecodeUsage::default(),
        }
    }

    pub fn usage(&self) -> HeaderDecodeUsage {
        self.usage
    }

    fn charge(
        used: &mut usize,
        amount: usize,
        limit: usize,
        resource: &'static str,
    ) -> Result<(), UnversionedError> {
        let attempted = used
            .checked_add(amount)
            .ok_or(UnversionedError::ResourceLimit { resource })?;
        if attempted > limit {
            return Err(UnversionedError::ResourceLimit { resource });
        }
        *used = attempted;
        Ok(())
    }

    fn fragment(&mut self) -> Result<(), UnversionedError> {
        Self::charge(
            &mut self.usage.fragments,
            1,
            self.limits.max_fragments,
            "header fragments",
        )?;
        self.work(1)
    }

    fn entries(&mut self, amount: usize) -> Result<(), UnversionedError> {
        Self::charge(
            &mut self.usage.entries,
            amount,
            self.limits.max_entries,
            "header entries",
        )?;
        self.work(amount)
    }

    fn work(&mut self, amount: usize) -> Result<(), UnversionedError> {
        Self::charge(
            &mut self.usage.work,
            amount,
            self.limits.max_work,
            "header work",
        )
    }

    fn allocation(&mut self, amount: usize) -> Result<(), UnversionedError> {
        Self::charge(
            &mut self.usage.allocation_bytes,
            amount,
            self.limits.max_allocation_bytes,
            "header allocations",
        )
    }

    fn bytes(&mut self, amount: usize) -> Result<(), UnversionedError> {
        Self::charge(
            &mut self.usage.byte_work,
            amount,
            self.limits.max_byte_work,
            "header byte work",
        )
    }
}

/// Semantic contents of an unversioned-property header.
///
/// A decoded header retains its original bytes. [`Self::encode`] returns those
/// bytes verbatim until a semantic mutation is made, which lets callers inspect
/// and rewrite unrelated payload bytes without normalizing the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnversionedHeader {
    schema_slot_count: usize,
    entries: Vec<HeaderEntry>,
    original_encoding: Option<Vec<u8>>,
}

impl UnversionedHeader {
    pub fn empty(schema_slot_count: usize) -> Self {
        Self {
            schema_slot_count,
            entries: Vec::new(),
            original_encoding: None,
        }
    }

    /// Construct a header from property states. Entries may be unordered, but
    /// duplicate or out-of-range schema indices are rejected.
    pub fn from_entries(
        schema_slot_count: usize,
        source_entries: impl IntoIterator<Item = HeaderEntry>,
    ) -> Result<Self, UnversionedError> {
        let mut entries: Vec<HeaderEntry> = Vec::new();
        for entry in source_entries {
            if entries.len() >= MAX_HEADER_ENTRIES {
                return Err(UnversionedError::TooManyEntries {
                    limit: MAX_HEADER_ENTRIES,
                });
            }
            entries.push(entry);
        }
        entries.sort_unstable_by_key(|entry| entry.schema_index);
        validate_entries(schema_slot_count, &entries)?;
        Ok(Self {
            schema_slot_count,
            entries,
            original_encoding: None,
        })
    }

    /// Decode a header at the start of `bytes`.
    ///
    /// The returned byte count is the exact offset at which non-zero property
    /// payloads begin. `schema_slot_count` should be the length returned by
    /// [`crate::SchemaDb::flatten_slots`].
    pub fn decode(
        bytes: &[u8],
        schema_slot_count: usize,
    ) -> Result<(Self, usize), UnversionedError> {
        let mut offset = 0usize;
        let mut cursor = 0usize;
        let mut fragments = Vec::new();
        let mut zero_mask_bits = 0usize;
        let mut entry_count = 0usize;

        loop {
            if fragments.len() >= MAX_HEADER_FRAGMENTS {
                return Err(UnversionedError::TooManyFragments {
                    limit: MAX_HEADER_FRAGMENTS,
                });
            }
            let word = read_u16(bytes, offset, "fragment")?;
            offset += 2;

            let skip_num = (word & SKIP_MASK) as usize;
            let value_num = (word >> VALUE_SHIFT) as usize;
            let has_zero_mask = word & HAS_ZERO_MASK != 0;
            let is_last = word & IS_LAST_MASK != 0;
            let fragment_index = fragments.len();

            let first_index =
                cursor
                    .checked_add(skip_num)
                    .ok_or(UnversionedError::SchemaIndexOverflow {
                        fragment: fragment_index,
                    })?;
            let end = first_index.checked_add(value_num).ok_or(
                UnversionedError::SchemaIndexOverflow {
                    fragment: fragment_index,
                },
            )?;
            if end > schema_slot_count {
                return Err(UnversionedError::FragmentOutOfRange {
                    fragment: fragment_index,
                    end,
                    schema_slot_count,
                });
            }
            if has_zero_mask {
                zero_mask_bits = zero_mask_bits
                    .checked_add(value_num)
                    .ok_or(UnversionedError::ZeroMaskSizeOverflow)?;
            }
            entry_count = entry_count.saturating_add(value_num);
            if entry_count > MAX_HEADER_ENTRIES {
                return Err(UnversionedError::TooManyEntries {
                    limit: MAX_HEADER_ENTRIES,
                });
            }

            fragments.push(DecodedFragment {
                first_index,
                value_num,
                has_zero_mask,
            });
            cursor = end;
            if is_last {
                break;
            }
        }

        let mask_len = zero_mask_byte_len(zero_mask_bits)?;
        let mask_end = offset
            .checked_add(mask_len)
            .ok_or(UnversionedError::ZeroMaskSizeOverflow)?;
        let zero_mask = bytes
            .get(offset..mask_end)
            .ok_or_else(|| UnversionedError::Truncated {
                offset,
                needed: mask_len,
                available: bytes.len().saturating_sub(offset),
                what: "zero mask",
            })?;

        let mut entries = Vec::new();
        let mut mask_index = 0usize;
        for fragment in fragments {
            for relative_index in 0..fragment.value_num {
                let is_zero = if fragment.has_zero_mask {
                    let value = mask_bit(zero_mask, mask_index);
                    mask_index += 1;
                    value
                } else {
                    false
                };
                entries.push(HeaderEntry {
                    schema_index: fragment.first_index + relative_index,
                    is_zero,
                });
            }
        }

        let consumed = mask_end;
        Ok((
            Self {
                schema_slot_count,
                entries,
                original_encoding: Some(bytes[..consumed].to_vec()),
            },
            consumed,
        ))
    }

    /// Two-pass bounded decoder used by the additive hostile-input walker.
    /// It never materializes a fragment vector and updates `budget` before all
    /// proportional work and allocation, including error paths.
    pub fn decode_bounded(
        bytes: &[u8],
        schema_slot_count: usize,
        budget: &mut HeaderDecodeBudget,
    ) -> Result<(Self, usize), UnversionedError> {
        let mut offset = 0usize;
        let mut cursor = 0usize;
        let mut zero_mask_bits = 0usize;
        loop {
            if budget.usage.fragments >= MAX_HEADER_FRAGMENTS {
                return Err(UnversionedError::TooManyFragments {
                    limit: MAX_HEADER_FRAGMENTS,
                });
            }
            budget.fragment()?;
            budget.bytes(2)?;
            let word = read_u16(bytes, offset, "fragment")?;
            offset += 2;
            let skip_num = (word & SKIP_MASK) as usize;
            let value_num = (word >> VALUE_SHIFT) as usize;
            let has_zero_mask = word & HAS_ZERO_MASK != 0;
            let is_last = word & IS_LAST_MASK != 0;
            let fragment_index = budget.usage.fragments - 1;
            let first_index =
                cursor
                    .checked_add(skip_num)
                    .ok_or(UnversionedError::SchemaIndexOverflow {
                        fragment: fragment_index,
                    })?;
            let end = first_index.checked_add(value_num).ok_or(
                UnversionedError::SchemaIndexOverflow {
                    fragment: fragment_index,
                },
            )?;
            if end > schema_slot_count {
                return Err(UnversionedError::FragmentOutOfRange {
                    fragment: fragment_index,
                    end,
                    schema_slot_count,
                });
            }
            if has_zero_mask {
                zero_mask_bits = zero_mask_bits
                    .checked_add(value_num)
                    .ok_or(UnversionedError::ZeroMaskSizeOverflow)?;
            }
            budget.entries(value_num)?;
            if budget.usage.entries > MAX_HEADER_ENTRIES {
                return Err(UnversionedError::TooManyEntries {
                    limit: MAX_HEADER_ENTRIES,
                });
            }
            cursor = end;
            if is_last {
                break;
            }
        }

        let mask_len = zero_mask_byte_len(zero_mask_bits)?;
        budget.bytes(mask_len)?;
        let mask_end = offset
            .checked_add(mask_len)
            .ok_or(UnversionedError::ZeroMaskSizeOverflow)?;
        let zero_mask = bytes
            .get(offset..mask_end)
            .ok_or_else(|| UnversionedError::Truncated {
                offset,
                needed: mask_len,
                available: bytes.len().saturating_sub(offset),
                what: "zero mask",
            })?;

        budget.allocation(
            budget
                .usage
                .entries
                .checked_mul(std::mem::size_of::<HeaderEntry>())
                .ok_or(UnversionedError::ResourceLimit {
                    resource: "header allocations",
                })?,
        )?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(budget.usage.entries)
            .map_err(|_| UnversionedError::Allocation)?;
        // The second pass rereads every fragment and visits every selected
        // entry. Charge it separately; the first-pass proof is not a licence
        // for an unmetered replay.
        budget.work(
            budget
                .usage
                .fragments
                .checked_add(budget.usage.entries)
                .ok_or(UnversionedError::ResourceLimit {
                    resource: "header work",
                })?,
        )?;
        budget.bytes(offset)?;
        let mut fragment_offset = 0usize;
        let mut schema_cursor = 0usize;
        let mut mask_index = 0usize;
        loop {
            let word = read_u16(bytes, fragment_offset, "fragment")?;
            fragment_offset += 2;
            let first_index = schema_cursor + usize::from(word & SKIP_MASK);
            let value_num = usize::from(word >> VALUE_SHIFT);
            let has_zero_mask = word & HAS_ZERO_MASK != 0;
            for relative_index in 0..value_num {
                let is_zero = if has_zero_mask {
                    let value = mask_bit(zero_mask, mask_index);
                    mask_index += 1;
                    value
                } else {
                    false
                };
                entries.push(HeaderEntry {
                    schema_index: first_index + relative_index,
                    is_zero,
                });
            }
            schema_cursor = first_index + value_num;
            if word & IS_LAST_MASK != 0 {
                break;
            }
        }

        budget.allocation(mask_end)?;
        budget.bytes(mask_end)?;
        let mut original = Vec::new();
        original
            .try_reserve_exact(mask_end)
            .map_err(|_| UnversionedError::Allocation)?;
        original.extend_from_slice(&bytes[..mask_end]);
        Ok((
            Self {
                schema_slot_count,
                entries,
                original_encoding: Some(original),
            },
            mask_end,
        ))
    }

    pub fn schema_slot_count(&self) -> usize {
        self.schema_slot_count
    }

    pub fn entries(&self) -> &[HeaderEntry] {
        &self.entries
    }

    pub fn entry(&self, schema_index: usize) -> Option<&HeaderEntry> {
        self.entries
            .binary_search_by_key(&schema_index, |entry| entry.schema_index)
            .ok()
            .map(|position| &self.entries[position])
    }

    pub fn has_non_zero_values(&self) -> bool {
        self.entries.iter().any(|entry| !entry.is_zero)
    }

    /// Insert or update a property state. Returns the previous entry, if any.
    /// A no-op update keeps the original byte-preserving encoding intact.
    pub fn set(
        &mut self,
        schema_index: usize,
        is_zero: bool,
    ) -> Result<Option<HeaderEntry>, UnversionedError> {
        validate_index(self.schema_slot_count, schema_index)?;
        let replacement = HeaderEntry {
            schema_index,
            is_zero,
        };
        match self
            .entries
            .binary_search_by_key(&schema_index, |entry| entry.schema_index)
        {
            Ok(position) => {
                let previous = self.entries[position];
                if previous != replacement {
                    self.entries[position] = replacement;
                    self.original_encoding = None;
                }
                Ok(Some(previous))
            }
            Err(position) => {
                if self.entries.len() >= MAX_HEADER_ENTRIES {
                    return Err(UnversionedError::TooManyEntries {
                        limit: MAX_HEADER_ENTRIES,
                    });
                }
                self.entries.insert(position, replacement);
                self.original_encoding = None;
                Ok(None)
            }
        }
    }

    pub fn remove(&mut self, schema_index: usize) -> Option<HeaderEntry> {
        let position = self
            .entries
            .binary_search_by_key(&schema_index, |entry| entry.schema_index)
            .ok()?;
        self.original_encoding = None;
        Some(self.entries.remove(position))
    }

    /// Join header entries to a flattened USMAP schema, preserving header order.
    pub fn resolve_slots<'a>(
        &self,
        slots: &'a [PropertySlot],
    ) -> Result<Vec<ResolvedHeaderEntry<'a>>, UnversionedError> {
        if slots.len() != self.schema_slot_count {
            return Err(UnversionedError::SlotCountMismatch {
                expected: self.schema_slot_count,
                actual: slots.len(),
            });
        }
        for (position, slot) in slots.iter().enumerate() {
            if slot.schema_index != position {
                return Err(UnversionedError::SlotLayoutMismatch {
                    position,
                    schema_index: slot.schema_index,
                });
            }
        }
        Ok(self
            .entries
            .iter()
            .map(|entry| ResolvedHeaderEntry {
                schema_index: entry.schema_index,
                is_zero: entry.is_zero,
                slot: &slots[entry.schema_index],
            })
            .collect())
    }

    /// Encode the header. An unmodified decoded header is returned byte-for-byte;
    /// constructed or edited headers use Unreal's canonical compact fragments.
    pub fn encode(&self) -> Result<Vec<u8>, UnversionedError> {
        if let Some(original) = &self.original_encoding {
            return Ok(original.clone());
        }
        self.encode_canonical()
    }

    /// Encode a canonical compact representation, ignoring retained source bytes.
    pub fn encode_canonical(&self) -> Result<Vec<u8>, UnversionedError> {
        validate_entries(self.schema_slot_count, &self.entries)?;
        let fragments = build_fragments(&self.entries)?;
        let zero_mask_bits: usize = fragments
            .iter()
            .filter(|fragment| fragment.has_zero_mask)
            .map(|fragment| fragment.value_num)
            .sum();
        let mask_len = zero_mask_byte_len(zero_mask_bits)?;
        let header_len = fragments
            .len()
            .checked_mul(2)
            .and_then(|len| len.checked_add(mask_len))
            .ok_or(UnversionedError::ZeroMaskSizeOverflow)?;
        let mut bytes = Vec::with_capacity(header_len);
        let mut mask = vec![0u8; mask_len];
        let mut mask_index = 0usize;

        for (index, fragment) in fragments.iter().enumerate() {
            let is_last = index + 1 == fragments.len();
            let mut word = fragment.skip_num as u16;
            if fragment.has_zero_mask {
                word |= HAS_ZERO_MASK;
            }
            if is_last {
                word |= IS_LAST_MASK;
            }
            word |= (fragment.value_num as u16) << VALUE_SHIFT;
            bytes.extend_from_slice(&word.to_le_bytes());

            if fragment.has_zero_mask {
                for entry in &self.entries[fragment.entry_start..fragment.entry_end] {
                    if entry.is_zero {
                        set_mask_bit(&mut mask, mask_index);
                    }
                    mask_index += 1;
                }
            }
        }
        bytes.extend_from_slice(&mask);
        Ok(bytes)
    }
}

#[derive(Debug)]
struct DecodedFragment {
    first_index: usize,
    value_num: usize,
    has_zero_mask: bool,
}

#[derive(Debug)]
struct EncodedFragment {
    skip_num: usize,
    value_num: usize,
    has_zero_mask: bool,
    entry_start: usize,
    entry_end: usize,
}

fn build_fragments(entries: &[HeaderEntry]) -> Result<Vec<EncodedFragment>, UnversionedError> {
    if entries.is_empty() {
        return Ok(vec![EncodedFragment {
            skip_num: 0,
            value_num: 0,
            has_zero_mask: false,
            entry_start: 0,
            entry_end: 0,
        }]);
    }

    let mut fragments = Vec::new();
    let mut cursor = 0usize;
    let mut entry_start = 0usize;
    while entry_start < entries.len() {
        let first_index = entries[entry_start].schema_index;
        let mut gap = first_index - cursor;
        while gap > FRAGMENT_FIELD_MAX {
            push_fragment(
                &mut fragments,
                EncodedFragment {
                    skip_num: FRAGMENT_FIELD_MAX,
                    value_num: 0,
                    has_zero_mask: false,
                    entry_start,
                    entry_end: entry_start,
                },
            )?;
            gap -= FRAGMENT_FIELD_MAX;
        }

        let mut entry_end = entry_start + 1;
        while entry_end < entries.len()
            && entry_end - entry_start < FRAGMENT_FIELD_MAX
            && entries[entry_end].schema_index == first_index + (entry_end - entry_start)
        {
            entry_end += 1;
        }
        let has_zero_mask = entries[entry_start..entry_end]
            .iter()
            .any(|entry| entry.is_zero);
        let value_num = entry_end - entry_start;
        push_fragment(
            &mut fragments,
            EncodedFragment {
                skip_num: gap,
                value_num,
                has_zero_mask,
                entry_start,
                entry_end,
            },
        )?;
        cursor = first_index + value_num;
        entry_start = entry_end;
    }
    Ok(fragments)
}

fn push_fragment(
    fragments: &mut Vec<EncodedFragment>,
    fragment: EncodedFragment,
) -> Result<(), UnversionedError> {
    if fragments.len() >= MAX_HEADER_FRAGMENTS {
        return Err(UnversionedError::TooManyFragments {
            limit: MAX_HEADER_FRAGMENTS,
        });
    }
    debug_assert!(fragment.skip_num <= FRAGMENT_FIELD_MAX);
    debug_assert!(fragment.value_num <= FRAGMENT_FIELD_MAX);
    fragments.push(fragment);
    Ok(())
}

fn validate_entries(
    schema_slot_count: usize,
    entries: &[HeaderEntry],
) -> Result<(), UnversionedError> {
    if entries.len() > MAX_HEADER_ENTRIES {
        return Err(UnversionedError::TooManyEntries {
            limit: MAX_HEADER_ENTRIES,
        });
    }
    let mut previous = None;
    for entry in entries {
        validate_index(schema_slot_count, entry.schema_index)?;
        if previous == Some(entry.schema_index) {
            return Err(UnversionedError::DuplicateSchemaIndex {
                schema_index: entry.schema_index,
            });
        }
        previous = Some(entry.schema_index);
    }
    Ok(())
}

fn validate_index(schema_slot_count: usize, schema_index: usize) -> Result<(), UnversionedError> {
    if schema_index >= schema_slot_count {
        return Err(UnversionedError::SchemaIndexOutOfRange {
            schema_index,
            schema_slot_count,
        });
    }
    Ok(())
}

fn zero_mask_byte_len(num_bits: usize) -> Result<usize, UnversionedError> {
    match num_bits {
        0 => Ok(0),
        1..=8 => Ok(1),
        9..=16 => Ok(2),
        _ => num_bits
            .checked_add(31)
            .map(|rounded| (rounded / 32) * 4)
            .ok_or(UnversionedError::ZeroMaskSizeOverflow),
    }
}

fn read_u16(bytes: &[u8], offset: usize, what: &'static str) -> Result<u16, UnversionedError> {
    let raw =
        bytes
            .get(offset..offset.saturating_add(2))
            .ok_or_else(|| UnversionedError::Truncated {
                offset,
                needed: 2,
                available: bytes.len().saturating_sub(offset),
                what,
            })?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn mask_bit(mask: &[u8], index: usize) -> bool {
    mask[index / 8] & (1 << (index % 8)) != 0
}

fn set_mask_bit(mask: &mut [u8], index: usize) {
    mask[index / 8] |= 1 << (index % 8);
}

#[cfg(test)]
mod tests {
    use super::*;
    use usmap::PropertyInner;

    fn entry(schema_index: usize, is_zero: bool) -> HeaderEntry {
        HeaderEntry {
            schema_index,
            is_zero,
        }
    }

    #[test]
    fn decodes_fragment_stream_and_zero_mask() {
        // skip 2, keep 3 (middle value zero); then skip 1, keep 2, last.
        let bytes = [0x82, 0x06, 0x01, 0x05, 0x02, 0xaa, 0xbb];
        let (header, consumed) = UnversionedHeader::decode(&bytes, 8).unwrap();
        assert_eq!(consumed, 5);
        assert_eq!(
            header.entries(),
            [
                entry(2, false),
                entry(3, true),
                entry(4, false),
                entry(6, false),
                entry(7, false),
            ]
        );
        assert!(header.has_non_zero_values());
        assert_eq!(header.encode().unwrap(), bytes[..consumed]);
    }

    #[test]
    fn bounded_decode_is_identical_exact_and_accounts_failed_prefixes() {
        let bytes = [0x82, 0x06, 0x01, 0x05, 0x02, 0xaa, 0xbb];
        let legacy = UnversionedHeader::decode(&bytes, 8).unwrap();
        let generous = HeaderDecodeLimits {
            max_fragments: 100,
            max_entries: 100,
            max_work: 1_000,
            max_allocation_bytes: 1_000,
            max_byte_work: 1_000,
        };
        let mut budget = HeaderDecodeBudget::new(generous);
        let bounded = UnversionedHeader::decode_bounded(&bytes, 8, &mut budget).unwrap();
        assert_eq!(bounded, legacy);
        let usage = budget.usage();
        let exact = HeaderDecodeLimits {
            max_fragments: usage.fragments,
            max_entries: usage.entries,
            max_work: usage.work,
            max_allocation_bytes: usage.allocation_bytes,
            max_byte_work: usage.byte_work,
        };
        let mut exact_budget = HeaderDecodeBudget::new(exact);
        assert_eq!(
            UnversionedHeader::decode_bounded(&bytes, 8, &mut exact_budget).unwrap(),
            legacy
        );

        let hostile = vec![0u8; 10];
        let mut failed = HeaderDecodeBudget::new(HeaderDecodeLimits {
            max_fragments: 4,
            max_entries: 0,
            max_work: 100,
            max_allocation_bytes: 0,
            max_byte_work: 100,
        });
        assert!(matches!(
            UnversionedHeader::decode_bounded(&hostile, 0, &mut failed),
            Err(UnversionedError::ResourceLimit {
                resource: "header fragments"
            })
        ));
        assert_eq!(failed.usage().fragments, 4);
        assert_eq!(failed.usage().byte_work, 8);
    }

    #[test]
    fn canonical_round_trip_splits_long_skips_runs_and_masks() {
        let entries: Vec<_> = (128..=299)
            .chain(500..=520)
            .map(|schema_index| entry(schema_index, schema_index % 11 == 0))
            .collect();
        let header = UnversionedHeader::from_entries(521, entries.clone()).unwrap();
        let encoded = header.encode().unwrap();
        let (decoded, consumed) = UnversionedHeader::decode(&encoded, 521).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.entries(), entries);
        assert_eq!(decoded.encode().unwrap(), encoded);
    }

    #[test]
    fn zero_mask_uses_whole_words_above_sixteen_bits() {
        let entries: Vec<_> = (0..80)
            .map(|schema_index| entry(schema_index, schema_index == 0 || schema_index == 79))
            .collect();
        let header = UnversionedHeader::from_entries(80, entries.clone()).unwrap();
        let encoded = header.encode().unwrap();
        // One fragment (2 bytes), then ceil(80/32) * 4 zero-mask bytes.
        assert_eq!(encoded.len(), 14);
        let (decoded, _) = UnversionedHeader::decode(&encoded, 80).unwrap();
        assert_eq!(decoded.entries(), entries);
    }

    #[test]
    fn malformed_and_out_of_range_headers_fail_cleanly() {
        assert!(matches!(
            UnversionedHeader::decode(&[0x00], 1),
            Err(UnversionedError::Truncated {
                what: "fragment",
                ..
            })
        ));
        // A zero non-final fragment is legal and still advances the byte stream;
        // without a following fragment this therefore fails as truncation.
        assert!(matches!(
            UnversionedHeader::decode(&[0x00, 0x00], 1),
            Err(UnversionedError::Truncated {
                what: "fragment",
                ..
            })
        ));
        // last, skip 5, keep 1 against a five-slot schema
        assert!(matches!(
            UnversionedHeader::decode(&[0x05, 0x03], 5),
            Err(UnversionedError::FragmentOutOfRange { .. })
        ));
        // last, masked, keep 1, but no following zero-mask byte
        assert!(matches!(
            UnversionedHeader::decode(&[0x80, 0x03], 1),
            Err(UnversionedError::Truncated {
                what: "zero mask",
                ..
            })
        ));
    }

    #[test]
    fn legal_empty_fragments_are_preserved_before_final_values() {
        // Real UE5.4 G1R UObject exports can begin with two empty, non-final
        // fragments. They are semantically redundant but are part of the
        // original unversioned header and must not be mistaken for a four-byte
        // envelope prefix. The fragment-count limit bounds malicious streams.
        let source = [0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
        let (header, consumed) = UnversionedHeader::decode(&source, 3).unwrap();
        assert_eq!(consumed, source.len());
        assert_eq!(header.entries(), [entry(0, false), entry(1, false)]);
        assert_eq!(header.encode().unwrap(), source);
        assert_eq!(header.encode_canonical().unwrap(), [0x00, 0x05]);

        // A different real export begins immediately with its meaningful final
        // fragment. This disproves any generic four-byte pre-header prefix.
        let direct = [0x03, 0x01]; // last, skip three, no serialized values
        let (header, consumed) = UnversionedHeader::decode(&direct, 3).unwrap();
        assert_eq!(consumed, direct.len());
        assert!(header.entries().is_empty());
        assert_eq!(header.encode().unwrap(), direct);
    }

    #[test]
    fn unmodified_decode_preserves_noncanonical_fragment_bytes() {
        // Two fragments encode adjacent non-zero entries; canonical form uses one.
        let source = [0x00, 0x02, 0x00, 0x03];
        let (mut header, _) = UnversionedHeader::decode(&source, 2).unwrap();
        assert_eq!(header.encode().unwrap(), source);
        assert_eq!(header.encode_canonical().unwrap(), [0x00, 0x05]);

        // A semantic no-op retains bytes; a real mutation switches to canonical.
        header.set(0, false).unwrap();
        assert_eq!(header.encode().unwrap(), source);
        header.set(0, true).unwrap();
        assert_ne!(header.encode().unwrap(), source);
        let (round_trip, _) = UnversionedHeader::decode(&header.encode().unwrap(), 2).unwrap();
        assert_eq!(round_trip.entry(0), Some(&entry(0, true)));
    }

    #[test]
    fn constructed_headers_sort_reject_duplicates_and_support_removal() {
        let mut header =
            UnversionedHeader::from_entries(4, [entry(3, false), entry(1, true), entry(0, false)])
                .unwrap();
        assert_eq!(
            header
                .entries()
                .iter()
                .map(|entry| entry.schema_index)
                .collect::<Vec<_>>(),
            [0, 1, 3]
        );
        assert_eq!(header.remove(1), Some(entry(1, true)));
        assert_eq!(header.remove(1), None);
        assert!(matches!(
            UnversionedHeader::from_entries(4, [entry(2, false), entry(2, true)]),
            Err(UnversionedError::DuplicateSchemaIndex { schema_index: 2 })
        ));
        assert!(matches!(
            header.set(4, false),
            Err(UnversionedError::SchemaIndexOutOfRange { .. })
        ));
    }

    #[test]
    fn resolves_entries_to_flattened_schema_slots() {
        let slots: Vec<_> = (0..3)
            .map(|schema_index| PropertySlot {
                schema_index,
                property_name: format!("P{schema_index}"),
                array_index: 0,
                array_dimension: 1,
                inner: PropertyInner::Int,
                declaring_schema_id: 0,
                declaring_schema_name: "Fixture".into(),
                declaring_module_path: Some("/Script/Test".into()),
            })
            .collect();
        let header = UnversionedHeader::from_entries(3, [entry(0, false), entry(2, true)]).unwrap();
        let resolved = header.resolve_slots(&slots).unwrap();
        assert_eq!(resolved[0].slot.property_name, "P0");
        assert_eq!(resolved[1].slot.property_name, "P2");
        assert!(resolved[1].is_zero);

        assert!(matches!(
            header.resolve_slots(&slots[..2]),
            Err(UnversionedError::SlotCountMismatch { .. })
        ));
    }

    #[test]
    fn empty_header_has_a_valid_last_fragment() {
        let encoded = UnversionedHeader::empty(0).encode().unwrap();
        assert_eq!(encoded, [0x00, 0x01]);
        let (decoded, consumed) = UnversionedHeader::decode(&encoded, 0).unwrap();
        assert_eq!(consumed, 2);
        assert!(decoded.entries().is_empty());
        assert!(!decoded.has_non_zero_values());
    }
}
