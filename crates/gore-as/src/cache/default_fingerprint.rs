//! Versioned, cache-only fingerprinting for supported default-value mutation families.
//!
//! The fingerprint is a build-membership primitive, never selector or mutation authority. It
//! hashes the full cache exactly once after validating, sorting, overlap-checking, and zeroing the
//! shared canonical direct-scalar ranges plus exact reference-proven GameplayTag-map ranges.

use std::collections::HashSet;
use std::ops::Range;

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::default_patterns::{
    direct_default_windows, immediate_bytes, is_canonical_initializer_metadata,
    is_reachable_linear_initializer,
};
use super::default_tag_map::{
    normalize_reference_proven_tag_map_operands, TagMapReferenceScanError,
};
use super::disasm::disassemble;
use super::header::CacheHeader;
use super::model::parse_modules;
use super::tables::parse_tail_tables;
use super::walk_modules::{collect_function_bytecode_spans, module_region_end};

pub const DEFAULT_CACHE_FINGERPRINT_FORMAT: &str =
    "gore-as-default-cache-fingerprint-v2-scalar-tag";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DefaultCacheFingerprint {
    pub(crate) sha256: [u8; 32],
    pub(crate) scalar_operand_count: usize,
    pub(crate) tag_operand_count: usize,
}

#[derive(Debug, Error)]
pub(crate) enum DefaultFingerprintError {
    #[error("invalid cache header: {0}")]
    Header(String),
    #[error("invalid cache structure: {0}")]
    Wire(String),
    #[error("cache tail tables end at {end:#x}, not EOF {len:#x}")]
    TailNotAtEof { end: usize, len: usize },
    #[error("duplicate key {key:#x} in tail table {table}")]
    DuplicateTailKey { table: &'static str, key: i64 },
    #[error(transparent)]
    TagMap(#[from] TagMapReferenceScanError),
    #[error("failed to disassemble {function}: {error}")]
    Disasm { function: String, error: String },
    #[error("default operand range overflows or is outside the cache in {function}")]
    OperandRange { function: String },
    #[error("default operand provenance mismatch in {function} at cache offset {offset:#x}")]
    OperandMismatch { function: String, offset: usize },
    #[error(
        "normalized default operand ranges overlap at {offset:#x}: {first_family} in {first_function} and {second_family} in {second_function}"
    )]
    OperandOverlap {
        first_family: &'static str,
        first_function: String,
        second_family: &'static str,
        second_function: String,
        offset: usize,
    },
}

#[derive(Debug, Clone)]
struct NormalizedRange {
    family: &'static str,
    function: String,
    range: Range<usize>,
}

pub(crate) fn combined_default_cache_fingerprint(
    cache: &[u8],
) -> Result<DefaultCacheFingerprint, DefaultFingerprintError> {
    let (mut normalized, tag_report) = normalize_reference_proven_tag_map_operands(cache)?;
    let scalar_ranges = scalar_default_operand_ranges(&normalized)?;
    let scalar_operand_count = scalar_ranges.len();
    let tag_operand_count = tag_report.sites.len();

    let mut ranges = scalar_ranges;
    ranges.extend(tag_report.sites.iter().map(|site| NormalizedRange {
        family: "gameplay-tag-float32",
        function: site.function.clone(),
        range: site.operand_range.clone(),
    }));
    validate_non_overlapping(&mut ranges)?;

    for operand in ranges
        .iter()
        .filter(|operand| operand.family == "direct-scalar")
    {
        normalized
            .get_mut(operand.range.clone())
            .ok_or_else(|| DefaultFingerprintError::OperandRange {
                function: operand.function.clone(),
            })?
            .fill(0);
    }

    let mut hash = Sha256::new();
    hash.update((DEFAULT_CACHE_FINGERPRINT_FORMAT.len() as u32).to_le_bytes());
    hash.update(DEFAULT_CACHE_FINGERPRINT_FORMAT.as_bytes());
    hash.update((scalar_operand_count as u64).to_le_bytes());
    hash.update((tag_operand_count as u64).to_le_bytes());
    hash.update(&normalized);
    Ok(DefaultCacheFingerprint {
        sha256: hash.finalize().into(),
        scalar_operand_count,
        tag_operand_count,
    })
}

/// Legacy scalar-only digest retained at the existing public API path. New native evidence uses
/// `combined_default_cache_fingerprint` above.
pub(crate) fn scalar_default_cache_sha256(
    cache: &[u8],
) -> Result<[u8; 32], DefaultFingerprintError> {
    let mut normalized = cache.to_vec();
    let mut ranges = scalar_default_operand_ranges(cache)?;
    validate_non_overlapping(&mut ranges)?;
    for operand in ranges {
        normalized
            .get_mut(operand.range)
            .ok_or(DefaultFingerprintError::OperandRange {
                function: operand.function,
            })?
            .fill(0);
    }
    Ok(Sha256::digest(&normalized).into())
}

fn scalar_default_operand_ranges(
    cache: &[u8],
) -> Result<Vec<NormalizedRange>, DefaultFingerprintError> {
    validate_cache(cache)?;
    let spans = collect_function_bytecode_spans(cache)
        .map_err(|error| DefaultFingerprintError::Wire(error.to_string()))?;
    let mut ranges = Vec::new();
    for span in &spans {
        if !is_canonical_initializer_metadata(span) {
            continue;
        }
        let instrs =
            disassemble(&span.code.bytecode).map_err(|error| DefaultFingerprintError::Disasm {
                function: span.code.func.clone(),
                error: error.to_string(),
            })?;
        if !is_reachable_linear_initializer(&instrs) {
            continue;
        }
        for window in direct_default_windows(&span.code.bytecode, &instrs) {
            let start = span
                .bytecode_offset
                .checked_add(window.operand_offset_dw.checked_mul(4).ok_or_else(|| {
                    DefaultFingerprintError::OperandRange {
                        function: span.code.func.clone(),
                    }
                })?)
                .ok_or_else(|| DefaultFingerprintError::OperandRange {
                    function: span.code.func.clone(),
                })?;
            let end = start
                .checked_add(window.pattern.operand_width())
                .ok_or_else(|| DefaultFingerprintError::OperandRange {
                    function: span.code.func.clone(),
                })?;
            let actual =
                cache
                    .get(start..end)
                    .ok_or_else(|| DefaultFingerprintError::OperandRange {
                        function: span.code.func.clone(),
                    })?;
            let expected = immediate_bytes(
                &span.code.bytecode,
                window.operand_offset_dw,
                window.pattern.operand_width(),
            )
            .ok_or_else(|| DefaultFingerprintError::OperandRange {
                function: span.code.func.clone(),
            })?;
            if actual != expected {
                return Err(DefaultFingerprintError::OperandMismatch {
                    function: span.code.func.clone(),
                    offset: start,
                });
            }
            ranges.push(NormalizedRange {
                family: "direct-scalar",
                function: span.code.func.clone(),
                range: start..end,
            });
        }
    }
    Ok(ranges)
}

fn validate_non_overlapping(ranges: &mut [NormalizedRange]) -> Result<(), DefaultFingerprintError> {
    ranges.sort_by_key(|operand| operand.range.start);
    for pair in ranges.windows(2) {
        let [first, second] = pair else {
            unreachable!("windows(2) always yields two ranges")
        };
        if first.range.end > second.range.start {
            return Err(DefaultFingerprintError::OperandOverlap {
                first_family: first.family,
                first_function: first.function.clone(),
                second_family: second.family,
                second_function: second.function.clone(),
                offset: second.range.start,
            });
        }
    }
    Ok(())
}

fn validate_cache(cache: &[u8]) -> Result<(), DefaultFingerprintError> {
    CacheHeader::parse(cache)
        .map_err(|error| DefaultFingerprintError::Header(error.to_string()))?;
    let tail = module_region_end(cache)
        .map_err(|error| DefaultFingerprintError::Wire(error.to_string()))?;
    let tables = parse_tail_tables(cache, tail)
        .map_err(|error| DefaultFingerprintError::Wire(error.to_string()))?;
    if tables.end != cache.len() {
        return Err(DefaultFingerprintError::TailNotAtEof {
            end: tables.end,
            len: cache.len(),
        });
    }
    const TABLE_NAMES: [&str; 7] = [
        "TypeReferences",
        "TypeIdReferenceToPointer",
        "FunctionReferences",
        "FunctionIdReferenceToPointer",
        "GlobalReferences",
        "StaticNames",
        "PropertyReferences",
    ];
    for (index, table) in tables.tables.iter().enumerate() {
        let mut keys = HashSet::with_capacity(table.keys.len());
        for key in &table.keys {
            if !keys.insert(*key) {
                return Err(DefaultFingerprintError::DuplicateTailKey {
                    table: TABLE_NAMES[index],
                    key: *key,
                });
            }
        }
    }
    parse_modules(cache).map_err(|error| DefaultFingerprintError::Wire(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_combined_fingerprint_is_stable_under_tag_edits() {
        let Some(path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
            eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
            return;
        };
        let cache = std::fs::read(path).expect("read configured cache");
        let fingerprint = combined_default_cache_fingerprint(&cache).expect("combined fingerprint");
        assert_eq!(
            fingerprint,
            DefaultCacheFingerprint {
                sha256: [
                    0x01, 0xfe, 0x4e, 0x37, 0xcc, 0x3a, 0x5d, 0xee, 0x15, 0xc2, 0xbe, 0xb4, 0x9a,
                    0x3f, 0x40, 0x61, 0x10, 0x77, 0x4b, 0x5e, 0x30, 0x0f, 0x2d, 0xe4, 0xad, 0x81,
                    0x1d, 0x0d, 0xf9, 0xad, 0xdd, 0x6b,
                ],
                scalar_operand_count: 26_339,
                tag_operand_count: 1_432,
            }
        );

        let report = super::super::default_tag_map::reference_proven_tag_map_sites(&cache)
            .expect("tag report");
        let mut changed = cache.clone();
        changed[report.sites[0].operand_range.start] ^= 0x01;
        assert_eq!(
            combined_default_cache_fingerprint(&changed).expect("changed fingerprint"),
            fingerprint
        );
        assert_ne!(
            scalar_default_cache_sha256(&changed).expect("changed scalar digest"),
            scalar_default_cache_sha256(&cache).expect("original scalar digest")
        );
    }
}
