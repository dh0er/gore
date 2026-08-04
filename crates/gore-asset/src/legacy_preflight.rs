//! Allocation-free bounds proof for the fixed G1R UE5.4 legacy header profile.
//!
//! `retoc` remains the authoritative decoder. This module only proves that every
//! count, offset, string, and aggregate that can drive one of its allocations is
//! within an application-selected bound before the decoder is entered.

use thiserror::Error;

const PACKAGE_TAG: u32 = 0x9e2a_83c1;
const FILTER_EDITOR_ONLY_FLAG: u32 = 0x8000_0000;

/// Resource limits for the fixed UE5.4 legacy-package header preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyHeaderLimits {
    pub max_names: usize,
    pub max_imports: usize,
    pub max_exports: usize,
    pub max_cell_imports: usize,
    pub max_cell_exports: usize,
    pub max_preload_dependencies: usize,
    pub max_data_resources: usize,
    pub max_summary_array_elements: usize,
    pub max_string_bytes: usize,
    pub max_total_string_bytes: usize,
    /// Aggregate bytes retained or temporarily allocated while deriving export
    /// object names and class paths after the allocation-free header proof.
    pub max_derived_metadata_bytes: usize,
    /// Maximum fully qualified class path retained for one export.
    pub max_class_path_bytes: usize,
    /// Aggregate nodes/string copies visited while deriving export metadata.
    pub max_derived_work: usize,
}

impl Default for LegacyHeaderLimits {
    fn default() -> Self {
        Self {
            max_names: 1_000_000,
            max_imports: 1_000_000,
            max_exports: 100_000,
            max_cell_imports: 100_000,
            max_cell_exports: 100_000,
            max_preload_dependencies: 1_000_000,
            max_data_resources: 100_000,
            max_summary_array_elements: 1_000_000,
            max_string_bytes: 1024 * 1024,
            max_total_string_bytes: 32 * 1024 * 1024,
            max_derived_metadata_bytes: 128 * 1024 * 1024,
            max_class_path_bytes: 1024 * 1024,
            max_derived_work: 2_000_000,
        }
    }
}

/// A malformed or over-budget header rejected before `retoc` is called.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LegacyHeaderPreflightError {
    #[error(
        "legacy package header is truncated at byte {offset}; need {needed} bytes, have {available}"
    )]
    Truncated {
        offset: usize,
        needed: usize,
        available: usize,
    },
    #[error("legacy package magic is invalid")]
    Magic,
    #[error("legacy package is not the fixed unversioned UE5 profile")]
    VersionProfile,
    #[error("legacy package is not cooked with unversioned properties")]
    PackageFlags,
    #[error("legacy package advertised header size {advertised}, actual {actual}")]
    HeaderLength { advertised: i32, actual: usize },
    #[error("{field} count {value} is negative")]
    NegativeCount { field: &'static str, value: i32 },
    #[error("{field} count {actual} exceeds limit {limit}")]
    CountLimit {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("{field} offset {value} is negative")]
    NegativeOffset { field: &'static str, value: i32 },
    #[error("{field} range {offset}..{end} exceeds {length}-byte header")]
    Range {
        field: &'static str,
        offset: usize,
        end: usize,
        length: usize,
    },
    #[error("{field} arithmetic overflowed")]
    Arithmetic { field: &'static str },
    #[error("one {field} string uses {actual} bytes, exceeding limit {limit}")]
    StringLimit {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("legacy header strings use {actual} bytes, exceeding aggregate limit {limit}")]
    StringAggregate { actual: usize, limit: usize },
    #[error("{field} has an invalid value {value}")]
    InvalidValue { field: &'static str, value: i64 },
}

#[derive(Debug, Clone, Copy)]
struct CountOffset {
    count: usize,
    offset: usize,
}

/// Prove every allocation-driving field for the fixed UE5.4 fallback layout.
pub(crate) fn preflight_g1r_ue5_4(
    bytes: &[u8],
    limits: LegacyHeaderLimits,
) -> Result<(), LegacyHeaderPreflightError> {
    let mut cursor = WireCursor::new(bytes, limits);
    if cursor.u32()? != PACKAGE_TAG {
        return Err(LegacyHeaderPreflightError::Magic);
    }

    let legacy_version = cursor.i32()?;
    if legacy_version > -8 {
        return Err(LegacyHeaderPreflightError::VersionProfile);
    }
    let ue3 = cursor.i32()?;
    let ue4 = cursor.i32()?;
    let ue5 = if legacy_version <= -8 {
        cursor.i32()?
    } else {
        0
    };
    let licensee = cursor.i32()?;
    let mut early_header_size = None;
    if legacy_version <= -9 {
        cursor.skip(20, "saved package hash")?;
        early_header_size = Some(cursor.i32()?);
    }
    let custom_versions = cursor.u32_count("custom versions", 0)?;
    if custom_versions != 0 || ue3 != 0 || ue4 != 0 || ue5 != 0 || licensee != 0 {
        return Err(LegacyHeaderPreflightError::VersionProfile);
    }
    let header_size = match early_header_size {
        Some(value) => value,
        None => cursor.i32()?,
    };
    if usize::try_from(header_size).ok() != Some(bytes.len()) {
        return Err(LegacyHeaderPreflightError::HeaderLength {
            advertised: header_size,
            actual: bytes.len(),
        });
    }

    cursor.fstring("package name")?;
    let package_flags = cursor.u32()?;
    let filtered = package_flags & FILTER_EDITOR_ONLY_FLAG != 0;

    let names = cursor.count_offset("names", limits.max_names)?;
    let soft_object_paths =
        cursor.count_offset("soft object paths", limits.max_summary_array_elements)?;
    if !filtered {
        cursor.fstring("localization id")?;
    }
    let gatherable =
        cursor.count_offset("gatherable text data", limits.max_summary_array_elements)?;
    let exports = cursor.count_offset("exports", limits.max_exports)?;
    let imports = cursor.count_offset("imports", limits.max_imports)?;

    // The fixed UE5.4 fallback predates VerseCells. These explicit zero values
    // still flow through the common validator so future layout additions cannot
    // accidentally omit either cap.
    validate_count("cell imports", 0, limits.max_cell_imports)?;
    validate_count("cell exports", 0, limits.max_cell_exports)?;

    let depends_offset = cursor.nonnegative_offset("depends")?;
    let soft_package_references =
        cursor.count_offset("soft package references", limits.max_summary_array_elements)?;
    cursor.nonnegative_or_sentinel_offset("searchable names")?;
    cursor.nonnegative_or_sentinel_offset("thumbnail table")?;
    // UE5.4 has no import-type-hierarchy fields. It does still serialize the
    // package GUID; that field was removed only by the later UE5.6 layout.
    cursor.skip(16, "package guid")?;
    if !filtered {
        cursor.skip(16, "persistent package guid")?;
    }
    let generation_count =
        cursor.u32_count("package generations", limits.max_summary_array_elements)?;
    cursor.skip_product(generation_count, 8, "package generations")?;
    cursor.engine_version("saved engine branch")?;
    cursor.engine_version("compatible engine branch")?;
    let compression_flags = cursor.u32()?;
    let compressed_chunks = cursor.i32()?;
    if compression_flags != 0 || compressed_chunks != 0 {
        return Err(LegacyHeaderPreflightError::InvalidValue {
            field: "legacy compression",
            value: i64::from(compressed_chunks),
        });
    }
    cursor.skip(4, "package source")?;
    let additional_count =
        cursor.u32_count("additional packages", limits.max_summary_array_elements)?;
    for _ in 0..additional_count {
        cursor.fstring("additional package")?;
    }
    cursor.nonnegative_or_sentinel_offset("asset registry data")?;
    cursor.skip(8, "bulk data start")?;
    cursor.nonnegative_or_sentinel_offset("world tile info")?;
    let chunk_count = cursor.u32_count("chunk ids", limits.max_summary_array_elements)?;
    cursor.skip_product(chunk_count, 4, "chunk ids")?;
    let preload = cursor.count_offset("preload dependencies", limits.max_preload_dependencies)?;
    let referenced_names = cursor.nonnegative_count("referenced names", limits.max_names)?;
    if referenced_names > names.count {
        return Err(LegacyHeaderPreflightError::CountLimit {
            field: "referenced names",
            actual: referenced_names,
            limit: names.count,
        });
    }
    cursor.skip(8, "payload toc offset")?;
    let data_resource_offset = cursor.i32()?;

    validate_fixed_range(bytes, "imports", imports, if filtered { 32 } else { 40 })?;
    validate_fixed_range(bytes, "exports", exports, 96)?;
    validate_fixed_range(bytes, "preload dependencies", preload, 4)?;
    validate_empty_or_offset(bytes, "soft object paths", soft_object_paths)?;
    validate_empty_or_offset(bytes, "gatherable text data", gatherable)?;
    validate_empty_or_offset(bytes, "soft package references", soft_package_references)?;
    if depends_offset > bytes.len() {
        return Err(LegacyHeaderPreflightError::Range {
            field: "depends",
            offset: depends_offset,
            end: depends_offset,
            length: bytes.len(),
        });
    }

    let mut names_cursor = WireCursor::at(bytes, limits, names.offset)?;
    for _ in 0..names.count {
        names_cursor.fstring("name map")?;
        names_cursor.skip(4, "name hashes")?;
    }
    cursor.absorb_strings(&names_cursor)?;

    if data_resource_offset > 0 {
        let offset = usize::try_from(data_resource_offset).map_err(|_| {
            LegacyHeaderPreflightError::NegativeOffset {
                field: "data resources",
                value: data_resource_offset,
            }
        })?;
        let mut resources = WireCursor::at(bytes, limits, offset)?;
        let version = resources.u32()?;
        if version > 2 {
            return Err(LegacyHeaderPreflightError::InvalidValue {
                field: "data resource version",
                value: i64::from(version),
            });
        }
        let count = resources.nonnegative_count("data resources", limits.max_data_resources)?;
        let width = if version >= 2 { 45 } else { 44 };
        resources.skip_product(count, width, "data resources")?;
    } else if data_resource_offset < -1 {
        return Err(LegacyHeaderPreflightError::NegativeOffset {
            field: "data resources",
            value: data_resource_offset,
        });
    }

    Ok(())
}

fn validate_count(
    field: &'static str,
    value: i32,
    limit: usize,
) -> Result<usize, LegacyHeaderPreflightError> {
    let count = usize::try_from(value)
        .map_err(|_| LegacyHeaderPreflightError::NegativeCount { field, value })?;
    if count > limit {
        return Err(LegacyHeaderPreflightError::CountLimit {
            field,
            actual: count,
            limit,
        });
    }
    Ok(count)
}

fn validate_fixed_range(
    bytes: &[u8],
    field: &'static str,
    pair: CountOffset,
    width: usize,
) -> Result<(), LegacyHeaderPreflightError> {
    let byte_len = pair
        .count
        .checked_mul(width)
        .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?;
    let end = pair
        .offset
        .checked_add(byte_len)
        .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?;
    if end > bytes.len() {
        return Err(LegacyHeaderPreflightError::Range {
            field,
            offset: pair.offset,
            end,
            length: bytes.len(),
        });
    }
    Ok(())
}

fn validate_empty_or_offset(
    bytes: &[u8],
    field: &'static str,
    pair: CountOffset,
) -> Result<(), LegacyHeaderPreflightError> {
    if pair.count != 0 || pair.offset > bytes.len() {
        return Err(LegacyHeaderPreflightError::Range {
            field,
            offset: pair.offset,
            end: pair.offset,
            length: bytes.len(),
        });
    }
    Ok(())
}

struct WireCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    limits: LegacyHeaderLimits,
    string_bytes: usize,
}

impl<'a> WireCursor<'a> {
    fn new(bytes: &'a [u8], limits: LegacyHeaderLimits) -> Self {
        Self {
            bytes,
            offset: 0,
            limits,
            string_bytes: 0,
        }
    }

    fn at(
        bytes: &'a [u8],
        limits: LegacyHeaderLimits,
        offset: usize,
    ) -> Result<Self, LegacyHeaderPreflightError> {
        if offset > bytes.len() {
            return Err(LegacyHeaderPreflightError::Range {
                field: "table offset",
                offset,
                end: offset,
                length: bytes.len(),
            });
        }
        Ok(Self {
            bytes,
            offset,
            limits,
            string_bytes: 0,
        })
    }

    fn read<const N: usize>(&mut self) -> Result<[u8; N], LegacyHeaderPreflightError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(LegacyHeaderPreflightError::Arithmetic {
                field: "header cursor",
            })?;
        let available = self.bytes.len().saturating_sub(self.offset);
        let slice =
            self.bytes
                .get(self.offset..end)
                .ok_or(LegacyHeaderPreflightError::Truncated {
                    offset: self.offset,
                    needed: N,
                    available,
                })?;
        self.offset = end;
        Ok(slice.try_into().expect("fixed-size slice checked above"))
    }

    fn u32(&mut self) -> Result<u32, LegacyHeaderPreflightError> {
        Ok(u32::from_le_bytes(self.read()?))
    }

    fn i32(&mut self) -> Result<i32, LegacyHeaderPreflightError> {
        Ok(i32::from_le_bytes(self.read()?))
    }

    fn skip(
        &mut self,
        count: usize,
        field: &'static str,
    ) -> Result<(), LegacyHeaderPreflightError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?;
        if end > self.bytes.len() {
            return Err(LegacyHeaderPreflightError::Truncated {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        self.offset = end;
        Ok(())
    }

    fn skip_product(
        &mut self,
        count: usize,
        width: usize,
        field: &'static str,
    ) -> Result<(), LegacyHeaderPreflightError> {
        let bytes = count
            .checked_mul(width)
            .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?;
        self.skip(bytes, field)
    }

    fn u32_count(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<usize, LegacyHeaderPreflightError> {
        let count = usize::try_from(self.u32()?)
            .map_err(|_| LegacyHeaderPreflightError::Arithmetic { field })?;
        if count > limit {
            return Err(LegacyHeaderPreflightError::CountLimit {
                field,
                actual: count,
                limit,
            });
        }
        Ok(count)
    }

    fn nonnegative_count(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<usize, LegacyHeaderPreflightError> {
        validate_count(field, self.i32()?, limit)
    }

    fn nonnegative_offset(
        &mut self,
        field: &'static str,
    ) -> Result<usize, LegacyHeaderPreflightError> {
        let value = self.i32()?;
        usize::try_from(value)
            .map_err(|_| LegacyHeaderPreflightError::NegativeOffset { field, value })
    }

    fn nonnegative_or_sentinel_offset(
        &mut self,
        field: &'static str,
    ) -> Result<(), LegacyHeaderPreflightError> {
        let value = self.i32()?;
        if value < -1 {
            return Err(LegacyHeaderPreflightError::NegativeOffset { field, value });
        }
        Ok(())
    }

    fn count_offset(
        &mut self,
        field: &'static str,
        limit: usize,
    ) -> Result<CountOffset, LegacyHeaderPreflightError> {
        let count = self.nonnegative_count(field, limit)?;
        let offset = self.nonnegative_offset(field)?;
        if offset > self.bytes.len() {
            return Err(LegacyHeaderPreflightError::Range {
                field,
                offset,
                end: offset,
                length: self.bytes.len(),
            });
        }
        Ok(CountOffset { count, offset })
    }

    fn fstring(&mut self, field: &'static str) -> Result<(), LegacyHeaderPreflightError> {
        let signed = self.i32()?;
        let bytes = if signed < 0 {
            let units = signed
                .checked_abs()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?;
            units
                .checked_mul(2)
                .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?
        } else {
            usize::try_from(signed).map_err(|_| LegacyHeaderPreflightError::Arithmetic { field })?
        };
        if bytes > self.limits.max_string_bytes {
            return Err(LegacyHeaderPreflightError::StringLimit {
                field,
                actual: bytes,
                limit: self.limits.max_string_bytes,
            });
        }
        let total = self
            .string_bytes
            .checked_add(bytes)
            .ok_or(LegacyHeaderPreflightError::Arithmetic { field })?;
        if total > self.limits.max_total_string_bytes {
            return Err(LegacyHeaderPreflightError::StringAggregate {
                actual: total,
                limit: self.limits.max_total_string_bytes,
            });
        }
        self.string_bytes = total;
        self.skip(bytes, field)
    }

    fn engine_version(&mut self, field: &'static str) -> Result<(), LegacyHeaderPreflightError> {
        self.skip(10, "engine version")?;
        self.fstring(field)
    }

    fn absorb_strings(&mut self, other: &Self) -> Result<(), LegacyHeaderPreflightError> {
        let total = self.string_bytes.checked_add(other.string_bytes).ok_or(
            LegacyHeaderPreflightError::Arithmetic {
                field: "string aggregate",
            },
        )?;
        if total > self.limits.max_total_string_bytes {
            return Err(LegacyHeaderPreflightError::StringAggregate {
                actual: total,
                limit: self.limits.max_total_string_bytes,
            });
        }
        self.string_bytes = total;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_count_validator_rejects_negative_and_i32_max_for_every_retoc_table() {
        let limits = LegacyHeaderLimits {
            max_names: 3,
            max_imports: 3,
            max_exports: 3,
            max_cell_imports: 3,
            max_cell_exports: 3,
            max_preload_dependencies: 3,
            max_data_resources: 3,
            ..LegacyHeaderLimits::default()
        };
        for (field, limit) in [
            ("names", limits.max_names),
            ("imports", limits.max_imports),
            ("exports", limits.max_exports),
            ("cell imports", limits.max_cell_imports),
            ("cell exports", limits.max_cell_exports),
            ("preload dependencies", limits.max_preload_dependencies),
            ("data resources", limits.max_data_resources),
        ] {
            assert!(matches!(
                validate_count(field, -1, limit),
                Err(LegacyHeaderPreflightError::NegativeCount { .. })
            ));
            assert!(matches!(
                validate_count(field, i32::MAX, limit),
                Err(LegacyHeaderPreflightError::CountLimit { .. })
            ));
        }
    }

    #[test]
    fn string_aggregate_and_table_ranges_are_exact_and_one_over_safe() {
        let limits = LegacyHeaderLimits {
            max_string_bytes: 3,
            max_total_string_bytes: 6,
            ..LegacyHeaderLimits::default()
        };
        let mut exact = WireCursor::new(&[3, 0, 0, 0, b'a', b'b', 0], limits);
        exact.fstring("exact").unwrap();

        let mut individual_over = WireCursor::new(&[4, 0, 0, 0, b'a', b'b', b'c', 0], limits);
        assert!(matches!(
            individual_over.fstring("over"),
            Err(LegacyHeaderPreflightError::StringLimit { actual: 4, .. })
        ));

        let bytes = [3, 0, 0, 0, b'a', b'b', 0, 3, 0, 0, 0, b'c', b'd', 0];
        let mut aggregate_exact = WireCursor::new(&bytes, limits);
        aggregate_exact.fstring("first").unwrap();
        aggregate_exact.fstring("second").unwrap();
        let mut aggregate_over = WireCursor::new(
            &[3, 0, 0, 0, b'a', b'b', 0, 4, 0, 0, 0, b'c', b'd', b'e', 0],
            LegacyHeaderLimits {
                max_string_bytes: 4,
                max_total_string_bytes: 6,
                ..LegacyHeaderLimits::default()
            },
        );
        aggregate_over.fstring("first").unwrap();
        assert!(matches!(
            aggregate_over.fstring("second"),
            Err(LegacyHeaderPreflightError::StringAggregate { actual: 7, .. })
        ));

        let pair = CountOffset {
            count: 2,
            offset: 2,
        };
        validate_fixed_range(&[0; 10], "table", pair, 4).unwrap();
        assert!(matches!(
            validate_fixed_range(&[0; 9], "table", pair, 4),
            Err(LegacyHeaderPreflightError::Range { end: 10, .. })
        ));
    }
}
