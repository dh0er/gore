//! Allocation-bounded structural preflight for the upstream USMAP parser.
//!
//! The upstream wire reader recursively builds `PropertyInner` boxes and grows several vectors
//! directly from untrusted counts. This module validates the complete decompressed wire first,
//! using only one fallibly allocated name-length table and (for Zstd) one fallibly allocated
//! output buffer. Only a successful preflight may reach `usmap::Usmap::read`.

use std::str;

use thiserror::Error;

/// Closed resource contract for one USMAP parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsmapLimits {
    pub max_file_bytes: usize,
    pub max_decompressed_bytes: usize,
    pub max_custom_versions: usize,
    pub max_names: usize,
    pub max_name_bytes: usize,
    pub max_total_name_bytes: usize,
    pub max_string_references: usize,
    pub max_materialized_string_bytes: usize,
    pub max_enums: usize,
    pub max_enum_entries: usize,
    pub max_schemas: usize,
    pub max_properties: usize,
    pub max_property_inner_depth: usize,
    pub max_property_inner_nodes: usize,
    pub max_extension_blocks: usize,
    pub max_extension_entries: usize,
}

impl Default for UsmapLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 128 * 1024 * 1024,
            max_decompressed_bytes: 128 * 1024 * 1024,
            max_custom_versions: 4_096,
            max_names: 250_000,
            max_name_bytes: 1_024,
            max_total_name_bytes: 16 * 1024 * 1024,
            max_string_references: 2_000_000,
            max_materialized_string_bytes: 64 * 1024 * 1024,
            max_enums: 100_000,
            max_enum_entries: 500_000,
            max_schemas: 100_000,
            max_properties: 500_000,
            max_property_inner_depth: 64,
            max_property_inner_nodes: 1_000_000,
            max_extension_blocks: 16,
            max_extension_entries: 1_000_000,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UsmapPreflightError {
    #[error("USMAP is truncated while reading {0}")]
    Truncated(&'static str),
    #[error("invalid bounded USMAP structure: {0}")]
    Invalid(&'static str),
    #[error("unsupported bounded USMAP version {0}")]
    UnsupportedVersion(u8),
    #[error("unsupported bounded USMAP compression method {0}")]
    UnsupportedCompression(u8),
    #[error("USMAP {resource} count/size {actual} exceeds limit {limit}")]
    Limit {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("USMAP allocation failed for {0}")]
    Allocation(&'static str),
    #[error("USMAP Zstd decompression failed or disagreed with the declared size")]
    Decompression,
    #[error("USMAP size arithmetic overflowed while reading {0}")]
    Arithmetic(&'static str),
}

pub(crate) fn preflight_bounded_usmap(
    bytes: &[u8],
    limits: UsmapLimits,
) -> Result<(), UsmapPreflightError> {
    ensure_limit("file bytes", bytes.len(), limits.max_file_bytes)?;
    let header = Header::parse(bytes, limits)?;
    let compressed = bytes
        .get(header.payload_offset..)
        .ok_or(UsmapPreflightError::Truncated("compressed payload"))?;

    if header.compression == 0 {
        if header.decompressed_size != compressed.len() {
            return Err(UsmapPreflightError::Invalid(
                "uncompressed payload size disagrees with its header",
            ));
        }
        return PayloadPreflight::new(compressed, header.version, limits).run();
    }

    let mut decompressed = Vec::new();
    decompressed
        .try_reserve_exact(header.decompressed_size)
        .map_err(|_| UsmapPreflightError::Allocation("decompressed bytes"))?;
    decompressed.resize(header.decompressed_size, 0);
    let actual = zstd::bulk::decompress_to_buffer(compressed, &mut decompressed)
        .map_err(|_| UsmapPreflightError::Decompression)?;
    if actual != header.decompressed_size {
        return Err(UsmapPreflightError::Decompression);
    }
    PayloadPreflight::new(&decompressed, header.version, limits).run()
}

#[derive(Debug, Clone, Copy)]
struct Header {
    version: u8,
    compression: u8,
    decompressed_size: usize,
    payload_offset: usize,
}

impl Header {
    fn parse(bytes: &[u8], limits: UsmapLimits) -> Result<Self, UsmapPreflightError> {
        let mut cursor = WireCursor::new(bytes);
        if cursor.u16("magic")? != 0x30c4 {
            return Err(UsmapPreflightError::Invalid("magic"));
        }
        let version = cursor.u8("version")?;
        if version > 4 {
            return Err(UsmapPreflightError::UnsupportedVersion(version));
        }
        if version >= 1 {
            let has_versioning = cursor.i32("package-version flag")? > 0;
            if has_versioning {
                cursor.skip(8, "package versions")?;
                let count = cursor.u32_usize("custom-version count")?;
                ensure_limit("custom versions", count, limits.max_custom_versions)?;
                let bytes = checked_mul(count, 24, "custom versions")?;
                cursor.skip(bytes, "custom versions")?;
                cursor.skip(4, "network changelist")?;
            }
        }
        let compression = cursor.u8("compression method")?;
        if !matches!(compression, 0 | 3) {
            return Err(UsmapPreflightError::UnsupportedCompression(compression));
        }
        let compressed_size = cursor.u32_usize("compressed size")?;
        let decompressed_size = cursor.u32_usize("decompressed size")?;
        ensure_limit(
            "decompressed bytes",
            decompressed_size,
            limits.max_decompressed_bytes,
        )?;
        if compressed_size != cursor.remaining() {
            return Err(UsmapPreflightError::Invalid(
                "compressed payload size disagrees with its header",
            ));
        }
        Ok(Self {
            version,
            compression,
            decompressed_size,
            payload_offset: cursor.position(),
        })
    }
}

struct PayloadPreflight<'a> {
    cursor: WireCursor<'a>,
    version: u8,
    limits: UsmapLimits,
    name_lengths: Vec<usize>,
    total_name_bytes: usize,
    string_references: usize,
    materialized_string_bytes: usize,
    enum_entries: usize,
    properties: usize,
    property_inner_nodes: usize,
    extension_blocks: usize,
    extension_entries: usize,
    seen_extensions: u8,
}

impl<'a> PayloadPreflight<'a> {
    fn new(bytes: &'a [u8], version: u8, limits: UsmapLimits) -> Self {
        Self {
            cursor: WireCursor::new(bytes),
            version,
            limits,
            name_lengths: Vec::new(),
            total_name_bytes: 0,
            string_references: 0,
            materialized_string_bytes: 0,
            enum_entries: 0,
            properties: 0,
            property_inner_nodes: 0,
            extension_blocks: 0,
            extension_entries: 0,
            seen_extensions: 0,
        }
    }

    fn run(mut self) -> Result<(), UsmapPreflightError> {
        self.names()?;
        self.enums()?;
        self.schemas()?;
        self.extensions()?;
        Ok(())
    }

    fn names(&mut self) -> Result<(), UsmapPreflightError> {
        let count = self.cursor.u32_usize("name count")?;
        ensure_limit("names", count, self.limits.max_names)?;
        self.name_lengths
            .try_reserve_exact(count)
            .map_err(|_| UsmapPreflightError::Allocation("name-length table"))?;
        for _ in 0..count {
            let raw_length = if self.version >= 2 {
                usize::from(self.cursor.u16("name length")?)
            } else {
                usize::from(self.cursor.u8("name length")?)
            };
            ensure_limit("single name bytes", raw_length, self.limits.max_name_bytes)?;
            self.total_name_bytes = charge(
                "total name bytes",
                self.total_name_bytes,
                raw_length,
                self.limits.max_total_name_bytes,
            )?;
            let raw = self.cursor.take(raw_length, "name bytes")?;
            let string_bytes = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
            str::from_utf8(&raw[..string_bytes])
                .map_err(|_| UsmapPreflightError::Invalid("name is not UTF-8"))?;
            self.materialized_string_bytes = charge(
                "materialized string bytes",
                self.materialized_string_bytes,
                checked_mul(string_bytes, 2, "name map string copies")?,
                self.limits.max_materialized_string_bytes,
            )?;
            self.name_lengths.push(string_bytes);
        }
        Ok(())
    }

    fn enums(&mut self) -> Result<(), UsmapPreflightError> {
        let count = self.cursor.u32_usize("enum count")?;
        ensure_limit("enums", count, self.limits.max_enums)?;
        for _ in 0..count {
            self.name_ref("enum name")?;
            let entries = if self.version >= 3 {
                usize::from(self.cursor.u16("enum entry count")?)
            } else {
                usize::from(self.cursor.u8("enum entry count")?)
            };
            self.enum_entries = charge(
                "enum entries",
                self.enum_entries,
                entries,
                self.limits.max_enum_entries,
            )?;
            for _ in 0..entries {
                if self.version >= 4 {
                    self.cursor.skip(8, "explicit enum value")?;
                }
                self.name_ref("enum entry name")?;
            }
        }
        Ok(())
    }

    fn schemas(&mut self) -> Result<(), UsmapPreflightError> {
        let count = self.cursor.u32_usize("schema count")?;
        ensure_limit("schemas", count, self.limits.max_schemas)?;
        for _ in 0..count {
            self.name_ref("schema name")?;
            self.optional_name_ref("super schema name")?;
            let property_slots = usize::from(self.cursor.u16("property slot count")?);
            let serializable = usize::from(self.cursor.u16("property count")?);
            self.properties = charge(
                "properties",
                self.properties,
                serializable,
                self.limits.max_properties,
            )?;
            let mut computed_slots = 0usize;
            for _ in 0..serializable {
                self.cursor.skip(2, "property index")?;
                let array_dim = usize::from(self.cursor.u8("property array dimension")?);
                computed_slots = computed_slots
                    .checked_add(array_dim)
                    .ok_or(UsmapPreflightError::Arithmetic("property slots"))?;
                self.name_ref("property name")?;
                self.property_inner(1)?;
            }
            if computed_slots != property_slots {
                return Err(UsmapPreflightError::Invalid(
                    "property slot count disagrees with array dimensions",
                ));
            }
        }
        Ok(())
    }

    fn property_inner(&mut self, depth: usize) -> Result<(), UsmapPreflightError> {
        ensure_limit(
            "property-inner depth",
            depth,
            self.limits.max_property_inner_depth,
        )?;
        self.property_inner_nodes = charge(
            "property-inner nodes",
            self.property_inner_nodes,
            1,
            self.limits.max_property_inner_nodes,
        )?;
        match self.cursor.u8("property type")? {
            8 => self.property_inner(depth + 1)?,
            9 => self.name_ref("struct property type")?,
            24 => {
                self.property_inner(depth + 1)?;
                self.property_inner(depth + 1)?;
            }
            25 => self.property_inner(depth + 1)?,
            26 => {
                self.property_inner(depth + 1)?;
                self.name_ref("enum property type")?;
            }
            28 => self.property_inner(depth + 1)?,
            0..=7 | 10..=23 | 27 | 29 | 30 | 0xff => {}
            _ => return Err(UsmapPreflightError::Invalid("unknown property type")),
        }
        Ok(())
    }

    fn extensions(&mut self) -> Result<(), UsmapPreflightError> {
        while self.cursor.remaining() != 0 {
            let tag: [u8; 4] = self
                .cursor
                .take(4, "extension tag")?
                .try_into()
                .expect("fixed extension tag length");
            self.extension_blocks = charge(
                "extension blocks",
                self.extension_blocks,
                1,
                self.limits.max_extension_blocks,
            )?;
            let (bit, sized) = match &tag {
                b"CEXT" => (1, false),
                b"PPTH" => (2, true),
                b"EATR" => (4, true),
                b"ENVP" => (8, true),
                _ => return Err(UsmapPreflightError::Invalid("unknown extension tag")),
            };
            if self.seen_extensions & bit != 0 {
                return Err(UsmapPreflightError::Invalid("duplicate extension tag"));
            }
            self.seen_extensions |= bit;
            if sized {
                let size = self.cursor.u32_usize("extension byte size")?;
                let section = self.cursor.take(size, "extension body")?;
                let mut nested = WireCursor::new(section);
                match &tag {
                    b"PPTH" => self.ppth(&mut nested)?,
                    b"EATR" => self.eatr(&mut nested)?,
                    b"ENVP" => self.envp(&mut nested)?,
                    _ => unreachable!("sized extension set is closed"),
                }
                if nested.remaining() != 0 {
                    return Err(UsmapPreflightError::Invalid(
                        "extension size disagrees with its fields",
                    ));
                }
            } else {
                self.cursor.skip(1, "CEXT version")?;
                let entries = self.cursor.u32_usize("CEXT entry count")?;
                self.charge_extension_entries(entries)?;
            }
        }
        Ok(())
    }

    fn ppth(&mut self, cursor: &mut WireCursor<'_>) -> Result<(), UsmapPreflightError> {
        cursor.skip(1, "PPTH version")?;
        let enums = cursor.u32_usize("PPTH enum count")?;
        self.charge_extension_entries(enums)?;
        for _ in 0..enums {
            self.name_ref_from(cursor, "PPTH enum path")?;
        }
        let schemas = cursor.u32_usize("PPTH schema count")?;
        self.charge_extension_entries(schemas)?;
        for _ in 0..schemas {
            self.name_ref_from(cursor, "PPTH schema path")?;
        }
        Ok(())
    }

    fn eatr(&mut self, cursor: &mut WireCursor<'_>) -> Result<(), UsmapPreflightError> {
        cursor.skip(1, "EATR version")?;
        let enums = cursor.u32_usize("EATR enum count")?;
        self.charge_extension_entries(enums)?;
        cursor.skip(checked_mul(enums, 4, "EATR enum flags")?, "EATR enum flags")?;
        let schemas = cursor.u32_usize("EATR schema count")?;
        self.charge_extension_entries(schemas)?;
        for _ in 0..schemas {
            if cursor.u8("EATR schema kind")? > 2 {
                return Err(UsmapPreflightError::Invalid("unknown EATR schema kind"));
            }
            cursor.skip(4, "EATR schema flags")?;
            let properties = cursor.u32_usize("EATR property flag count")?;
            self.charge_extension_entries(properties)?;
            cursor.skip(
                checked_mul(properties, 8, "EATR property flags")?,
                "EATR property flags",
            )?;
        }
        Ok(())
    }

    fn envp(&mut self, cursor: &mut WireCursor<'_>) -> Result<(), UsmapPreflightError> {
        cursor.skip(1, "ENVP version")?;
        let groups = cursor.u32_usize("ENVP group count")?;
        self.charge_extension_entries(groups)?;
        for _ in 0..groups {
            let pairs = cursor.u32_usize("ENVP pair count")?;
            self.charge_extension_entries(pairs)?;
            for _ in 0..pairs {
                self.name_ref_from(cursor, "ENVP name")?;
                cursor.skip(8, "ENVP value")?;
            }
        }
        Ok(())
    }

    fn charge_extension_entries(&mut self, count: usize) -> Result<(), UsmapPreflightError> {
        self.extension_entries = charge(
            "extension entries",
            self.extension_entries,
            count,
            self.limits.max_extension_entries,
        )?;
        Ok(())
    }

    fn name_ref(&mut self, what: &'static str) -> Result<(), UsmapPreflightError> {
        let index = self.cursor.u32(what)?;
        self.charge_name_index(index, false)
    }

    fn optional_name_ref(&mut self, what: &'static str) -> Result<(), UsmapPreflightError> {
        let index = self.cursor.u32(what)?;
        self.charge_name_index(index, true)
    }

    fn name_ref_from(
        &mut self,
        cursor: &mut WireCursor<'_>,
        what: &'static str,
    ) -> Result<(), UsmapPreflightError> {
        let index = cursor.u32(what)?;
        self.charge_name_index(index, false)
    }

    fn charge_name_index(&mut self, index: u32, optional: bool) -> Result<(), UsmapPreflightError> {
        if optional && index == u32::MAX {
            return Ok(());
        }
        let length = *self
            .name_lengths
            .get(index as usize)
            .ok_or(UsmapPreflightError::Invalid("name index is out of range"))?;
        self.string_references = charge(
            "string references",
            self.string_references,
            1,
            self.limits.max_string_references,
        )?;
        self.materialized_string_bytes = charge(
            "materialized string bytes",
            self.materialized_string_bytes,
            length,
            self.limits.max_materialized_string_bytes,
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct WireCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> WireCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    const fn position(self) -> usize {
        self.position
    }

    fn remaining(self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    fn take(&mut self, length: usize, what: &'static str) -> Result<&'a [u8], UsmapPreflightError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(UsmapPreflightError::Arithmetic(what))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(UsmapPreflightError::Truncated(what))?;
        self.position = end;
        Ok(value)
    }

    fn skip(&mut self, length: usize, what: &'static str) -> Result<(), UsmapPreflightError> {
        self.take(length, what).map(|_| ())
    }

    fn u8(&mut self, what: &'static str) -> Result<u8, UsmapPreflightError> {
        Ok(self.take(1, what)?[0])
    }

    fn u16(&mut self, what: &'static str) -> Result<u16, UsmapPreflightError> {
        Ok(u16::from_le_bytes(
            self.take(2, what)?.try_into().expect("checked u16 length"),
        ))
    }

    fn u32(&mut self, what: &'static str) -> Result<u32, UsmapPreflightError> {
        Ok(u32::from_le_bytes(
            self.take(4, what)?.try_into().expect("checked u32 length"),
        ))
    }

    fn i32(&mut self, what: &'static str) -> Result<i32, UsmapPreflightError> {
        Ok(self.u32(what)? as i32)
    }

    fn u32_usize(&mut self, what: &'static str) -> Result<usize, UsmapPreflightError> {
        usize::try_from(self.u32(what)?).map_err(|_| UsmapPreflightError::Arithmetic(what))
    }
}

fn ensure_limit(
    resource: &'static str,
    actual: usize,
    limit: usize,
) -> Result<(), UsmapPreflightError> {
    if actual > limit {
        Err(UsmapPreflightError::Limit {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn charge(
    resource: &'static str,
    current: usize,
    added: usize,
    limit: usize,
) -> Result<usize, UsmapPreflightError> {
    let actual = current
        .checked_add(added)
        .ok_or(UsmapPreflightError::Arithmetic(resource))?;
    ensure_limit(resource, actual, limit)?;
    Ok(actual)
}

fn checked_mul(
    value: usize,
    width: usize,
    what: &'static str,
) -> Result<usize, UsmapPreflightError> {
    value
        .checked_mul(width)
        .ok_or(UsmapPreflightError::Arithmetic(what))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(payload: &[u8], compression: u8) -> Vec<u8> {
        let encoded = if compression == 3 {
            zstd::bulk::compress(payload, 1).unwrap()
        } else {
            payload.to_vec()
        };
        let mut bytes = vec![0xc4, 0x30, 4];
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.push(compression);
        bytes.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&encoded);
        bytes
    }

    fn empty_payload() -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload
    }

    fn nested_property_payload(depth: usize) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&2u32.to_le_bytes());
        for name in [b"Schema".as_slice(), b"Value".as_slice()] {
            payload.extend_from_slice(&(name.len() as u16).to_le_bytes());
            payload.extend_from_slice(name);
        }
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&u32::MAX.to_le_bytes());
        payload.extend_from_slice(&1u16.to_le_bytes());
        payload.extend_from_slice(&1u16.to_le_bytes());
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.push(1);
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend(std::iter::repeat_n(28, depth.saturating_sub(1)));
        payload.push(1);
        payload
    }

    #[test]
    fn exact_limits_and_zstd_are_accepted() {
        let payload = nested_property_payload(3);
        let limits = UsmapLimits {
            max_names: 2,
            max_schemas: 1,
            max_properties: 1,
            max_property_inner_depth: 3,
            max_property_inner_nodes: 3,
            max_total_name_bytes: 11,
            ..UsmapLimits::default()
        };
        for compression in [0, 3] {
            preflight_bounded_usmap(&wrap(&payload, compression), limits).unwrap();
        }
    }

    #[test]
    fn every_count_limit_fails_before_count_driven_work() {
        let limits = UsmapLimits {
            max_names: 0,
            ..UsmapLimits::default()
        };
        let mut payload = empty_payload();
        payload[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), limits),
            Err(UsmapPreflightError::Limit {
                resource: "names",
                ..
            })
        ));

        let mut payload = empty_payload();
        payload[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        let limits = UsmapLimits {
            max_enums: 0,
            ..UsmapLimits::default()
        };
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), limits),
            Err(UsmapPreflightError::Limit {
                resource: "enums",
                ..
            })
        ));

        let mut payload = empty_payload();
        payload[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        let limits = UsmapLimits {
            max_schemas: 0,
            ..UsmapLimits::default()
        };
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), limits),
            Err(UsmapPreflightError::Limit {
                resource: "schemas",
                ..
            })
        ));
    }

    #[test]
    fn name_and_recursive_property_limit_plus_one_fail_closed() {
        let payload = nested_property_payload(4);
        let limits = UsmapLimits {
            max_property_inner_depth: 3,
            ..UsmapLimits::default()
        };
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), limits),
            Err(UsmapPreflightError::Limit {
                resource: "property-inner depth",
                actual: 4,
                limit: 3,
            })
        ));

        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&4u16.to_le_bytes());
        payload.extend_from_slice(b"name");
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        let limits = UsmapLimits {
            max_name_bytes: 3,
            ..UsmapLimits::default()
        };
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), limits),
            Err(UsmapPreflightError::Limit {
                resource: "single name bytes",
                actual: 4,
                limit: 3,
            })
        ));
    }

    #[test]
    fn extension_aggregates_and_declared_sizes_are_bounded() {
        let mut payload = empty_payload();
        payload.extend_from_slice(b"CEXT");
        payload.push(0);
        payload.extend_from_slice(&2u32.to_le_bytes());
        let limits = UsmapLimits {
            max_extension_entries: 1,
            ..UsmapLimits::default()
        };
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), limits),
            Err(UsmapPreflightError::Limit {
                resource: "extension entries",
                actual: 2,
                limit: 1,
            })
        ));

        let mut payload = empty_payload();
        payload.extend_from_slice(b"PPTH");
        payload.extend_from_slice(&100u32.to_le_bytes());
        assert!(matches!(
            preflight_bounded_usmap(&wrap(&payload, 0), UsmapLimits::default()),
            Err(UsmapPreflightError::Truncated("extension body"))
        ));
    }
}
