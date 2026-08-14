//! Splice one game-compiled module into the shipped cache (Weg B).
//!
//! Per `work/reversing/gore-as/findings/container-splice.md` §5: insert the mini-cache's
//! single module entry immediately before the base cache's global tail tables, and bump
//! the `Modules` count at 0x14 by one. The runtime may not enforce the FGuid, but GORE does:
//! prepared minis are bound to the exact base generation before composition.
//!
//! [`splice`] is the strict **case-(b)** fast path: the mini module references no global
//! types (its 7 tail tables are 28 zero bytes). [`splice_auto`] also supports **case-(a)**
//! class-/native-reference-bearing modules by merging their minimal tail-table rows with
//! collision checks.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use super::header::CacheHeader;
use super::tables::{parse_tail_tables, TailTables, N_TABLES};
use super::walk_modules::{module_count, module_names, module_ranges, module_region_end};
use super::wire::{Cursor, WireError};
use thiserror::Error;

// One Manager loadout may enable 1,000 mods, but a production AngelScript loadout with hundreds
// of independently compiled modules is already pathological. Keep the repeated validation work
// firmly bounded while leaving two orders of magnitude above current real fixtures.
const MAX_SEQUENTIAL_MINIS: u64 = 256;
const MAX_SEQUENTIAL_KEYED_ROWS: u64 = 131_072;
const MAX_SEQUENTIAL_KEYED_ROW_BYTES: u64 = 64 * 1024 * 1024;
// The pristine Shipping cache currently has ~180k keyed tail rows. Base preflight needs a wider
// envelope than the much smaller cumulative contribution budget for independently compiled minis.
const MAX_BASE_KEYED_ROWS: u64 = 1_000_000;
const MAX_BASE_KEYED_ROW_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SEQUENTIAL_T6_NAMES: u64 = 65_536;
const MAX_SEQUENTIAL_T6_NAME_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPOSED_CACHE_BYTES: u64 = 1024 * 1024 * 1024;
// Each guarded composition currently performs two full prospective-output validation walks (the
// low-level mechanical pass and the generation-bound declaration pass). Bound their cumulative
// bytes so 256 tiny minis cannot force hundreds of GiB of repeated scans over a near-GiB cache.
const MAX_SEQUENTIAL_COMPOSED_SCAN_BYTES: u64 = 16 * 1024 * 1024 * 1024;
// Validation currently materializes each function's bytecode once for disassembly. Cap both one
// function and the complete mini well below the Manager's 1-GiB archive ceiling so worst-case input
// cannot create a second near-GiB allocation. The shipped cache is ~124 MiB in total; prepared
// one-module minis are orders of magnitude smaller.
const MAX_MINI_FUNCTION_BYTECODE_DWORDS: u64 = 4 * 1024 * 1024;
const MAX_MINI_TOTAL_BYTECODE_DWORDS: u64 = 16 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum SpliceError {
    #[error("cache header error: {0}")]
    Header(#[from] super::header::HeaderError),
    #[error("walk error: {0}")]
    Wire(#[from] WireError),
    #[error("mini-cache must contain exactly 1 module, found {0}")]
    MiniNotSingle(u32),
    #[error(
        "mini-cache has non-empty global tables ({0} trailing bytes): strict splice only accepts referenceless modules; use splice_auto for a remapped class/function-bearing mini"
    )]
    MiniHasGlobalRefs(usize),
    #[error("module name {0:?} already exists in the base cache (splicing would overwrite it)")]
    NameCollision(String),
    #[error(
        "inner module identity {0:?} already exists in the base cache (the runtime would overwrite it)"
    )]
    InnerNameCollision(String),
    #[error("module-map key {0:?} occurs more than once in the cache")]
    ModuleKeyCollision(String),
    #[error("module name {0:?} not found in the base cache (nothing to replace)")]
    NameNotFound(String),
    #[error(
        "module name {0:?} is ambiguous: it matches one module's TMap key and a different module's inner name — pass the exact TMap key"
    )]
    AmbiguousTarget(String),
    #[error(
        "tail tables of {which} don't end at EOF (ended {got:#x}, len {len:#x}) — parse desync"
    )]
    TailNotAtEof {
        which: &'static str,
        got: usize,
        len: usize,
    },
    #[error("OldReference key collision in table {table} ({key:#x}) — would need rekeying")]
    KeyCollision { table: usize, key: i64 },
    #[error(
        "cannot compose mini-caches: tail table {table} key {key:#x} was already contributed by an earlier mini"
    )]
    SequentialKeyCollision { table: usize, key: i64 },
    #[error("cannot rebase sequential StaticNames: {0}")]
    StaticNameRebase(#[from] super::remap::RemapError),
    #[error(
        "mini-cache GUID {mini:?} does not match target base GUID {base:?}; remap the module against this exact game cache before applying it"
    )]
    MiniGuidMismatch { base: [u8; 16], mini: [u8; 16] },
    #[error("mini-cache reference validation failed: {0}")]
    MiniReference(#[source] super::remap::RemapError),
    #[error("composed cache record validation failed: {0}")]
    ComposedModule(#[source] super::remap::RemapError),
    #[error("loadout-wide script ID planning failed: {0}")]
    LoadoutPlan(#[source] super::remap::RemapError),
    #[error("standalone script cache byte limit exceeded: {actual} > {limit}")]
    StandaloneCacheTooLarge { actual: u64, limit: u64 },
    #[error(
        "running cache is not the exact state expected by this sequential guard; restart composition from its pristine base"
    )]
    RunningStateMismatch,
    #[error("sequential mini-cache {resource} limit exceeded: {actual} > {limit}")]
    SequentialLimitExceeded {
        resource: &'static str,
        actual: u64,
        limit: u64,
    },
}

/// Streaming first pass for assigning one collision-stable pointer/engine-ID mapping to every
/// independently prepared mini-cache in a loadout. `inspect` retains no mini bytes, so callers may
/// read and drop one artifact at a time.
pub struct LoadoutScriptIdPlanBuilder(super::remap::LoadoutScriptIdPlanBuilder);

/// Opaque, immutable canonical assignment plan produced by [`LoadoutScriptIdPlanBuilder`].
///
/// The plan retains a parsed pristine-base context. Callers that also need a
/// [`SequentialMiniGuard`] should canonicalize their inspected minis first, drop this plan, and
/// only then construct the guard so both large contexts are never resident together.
pub struct LoadoutScriptIdPlan(super::remap::LoadoutScriptIdPlan);

fn loadout_splice_error(error: super::remap::RemapError) -> SpliceError {
    match error {
        super::remap::RemapError::LoadoutPlanGuidMismatch { pristine, mini } => {
            SpliceError::MiniGuidMismatch {
                base: pristine,
                mini,
            }
        }
        super::remap::RemapError::NotSingle(count) => SpliceError::MiniNotSingle(count),
        other => SpliceError::LoadoutPlan(other),
    }
}

impl LoadoutScriptIdPlanBuilder {
    /// Parse the unchanged pristine base once and start a loadout-wide identity inventory.
    pub fn new(pristine_base: &[u8]) -> Result<Self, SpliceError> {
        super::remap::LoadoutScriptIdPlanBuilder::new(pristine_base)
            .map(Self)
            .map_err(loadout_splice_error)
    }

    /// Inspect one exact mini atomically without retaining its bytes.
    pub fn inspect(&mut self, mini: &[u8]) -> Result<(), SpliceError> {
        self.0.inspect(mini).map_err(loadout_splice_error)
    }

    /// Freeze the portable-identity union and allocate its deterministic canonical mapping.
    pub fn finish(self) -> Result<LoadoutScriptIdPlan, SpliceError> {
        self.0
            .finish()
            .map(LoadoutScriptIdPlan)
            .map_err(loadout_splice_error)
    }
}

/// Canonicalize one exact mini previously inspected by `plan` against the same pristine base.
/// Both byte inputs are SHA-256-bound by the plan; the returned cache is still validated by
/// [`SequentialMiniGuard`] when it is composed.
pub fn remap_module_to_base_with_loadout_plan(
    mini: &[u8],
    pristine_base: &[u8],
    plan: &LoadoutScriptIdPlan,
) -> Result<Vec<u8>, SpliceError> {
    super::remap::remap_module_to_base_with_loadout_plan(mini, pristine_base, &plan.0)
        .map(|(bytes, _counts)| bytes)
        .map_err(loadout_splice_error)
}

fn check_bytecode_work(work: super::remap::ModuleWorkSummary) -> Result<(), SpliceError> {
    let max_function = u64::try_from(work.max_function_bytecode_dwords).unwrap_or(u64::MAX);
    if max_function > MAX_MINI_FUNCTION_BYTECODE_DWORDS {
        return Err(SpliceError::SequentialLimitExceeded {
            resource: "mini function bytecode dwords",
            actual: max_function,
            limit: MAX_MINI_FUNCTION_BYTECODE_DWORDS,
        });
    }
    if work.total_bytecode_dwords > MAX_MINI_TOTAL_BYTECODE_DWORDS {
        return Err(SpliceError::SequentialLimitExceeded {
            resource: "mini total bytecode dwords",
            actual: work.total_bytecode_dwords,
            limit: MAX_MINI_TOTAL_BYTECODE_DWORDS,
        });
    }
    Ok(())
}

fn finish_composition(out: Vec<u8>) -> Result<Vec<u8>, SpliceError> {
    match super::remap::validate_composed_module_records(&out) {
        Err(super::remap::RemapError::ModuleNameCollision { name }) => {
            return Err(SpliceError::InnerNameCollision(name));
        }
        Err(error) => return Err(SpliceError::ComposedModule(error)),
        Ok(()) => {}
    }
    Ok(out)
}

/// Validate a complete `PrecompiledScript*.Cache` replacement without interpreting its runtime
/// identities. This is deliberately narrower than [`SequentialMiniGuard`]: a standalone full
/// replacement owns its GUID and reference universe, so GORE only validates the supported header,
/// the complete module/tail wire shape, finite parser work, and exact EOF.
///
/// The input is never rewritten. Callers that publish it must retain and copy the original bytes.
pub fn validate_standalone_script_cache(bytes: &[u8]) -> Result<(), SpliceError> {
    CacheHeader::parse(bytes)?;
    let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual > MAX_COMPOSED_CACHE_BYTES {
        return Err(SpliceError::StandaloneCacheTooLarge {
            actual,
            limit: MAX_COMPOSED_CACHE_BYTES,
        });
    }
    super::remap::preflight_cache_module_work(bytes)?;
    // The tail preflight re-walks the module region, bounds all seven streaming table shapes and
    // their projected allocation work, and rejects any trailing byte after the final T7 row.
    super::remap::preflight_tail_tables(bytes)?;
    // Every non-T6 tail container is a TMap. Duplicate keys are malformed even when this cache
    // owns an unrelated GUID/reference universe: Unreal cannot deserialize two distinct entries
    // under one map key. T1/T3/T5 OldReference keys and derived T7 property keys also share the
    // runtime key domain enforced by the sequential overlay guard, so a standalone replacement
    // must not be a weaker way to publish the same collision.
    let tail = module_region_end(bytes)?;
    let tables = parse_tail_tables(bytes, tail)?;
    let mut pointer_domain = HashSet::new();
    for (table, rows) in tables.tables.iter().enumerate() {
        if table == 5 {
            continue;
        }
        let mut within_table = HashSet::new();
        for &key in &rows.keys {
            if !within_table.insert(key)
                || (matches!(table, 0 | 2 | 4 | 6) && !pointer_domain.insert(key))
            {
                return Err(SpliceError::KeyCollision { table, key });
            }
        }
    }
    // `Modules` is serialized as a TMap. Duplicate outer keys cannot survive deserialization as
    // two distinct entries, so reject that malformed container shape without imposing the
    // generation-specific declaration rules used for GORE-composed caches.
    let mut module_keys = HashSet::new();
    for name in module_names(bytes)? {
        if !module_keys.insert(name.clone()) {
            return Err(SpliceError::ModuleKeyCollision(name));
        }
    }
    Ok(())
}

/// Stateful preflight for folding independently compiled mini-caches onto one running cache.
///
/// Each mini was remapped against a pristine base. A later mini must therefore never reuse a
/// keyed T1–T5/T7 row contributed by an earlier mini: `mini wins` would retarget the earlier
/// module. T6 is identity-by-text, so later pools are instead deduplicated and their absolute
/// bytecode indices are rebased onto the names contributed by preceding minis. All validation is
/// completed before the guard records a mini, keeping retry state atomic on error.
#[derive(Debug)]
struct SequentialMiniBase {
    base_guid: [u8; 16],
    reference_context: super::remap::EffectiveReferenceBase,
    /// Exact base rows by key. A mini may repeat one byte-for-byte, but may never use `mini wins`
    /// to retarget existing base users of that key.
    base_rows: [HashMap<i64, Vec<u8>>; N_TABLES],
    base_ptr_tables: HashMap<i64, usize>,
    static_context: super::remap::StaticNameRebaseContext,
}

#[derive(Debug)]
pub struct SequentialMiniGuard {
    /// Large pristine-cache indexes are immutable and shared by staged compositions.
    base: Arc<SequentialMiniBase>,
    reference_state: super::remap::EffectiveReferenceState,
    /// Key -> canonical serialized row. An exact repeat is the same identity with the same fully
    /// remapped dependencies and is safe to deduplicate; a different row at that key is a hard
    /// hash/raw collision.
    contributed_rows: [HashMap<i64, Vec<u8>>; N_TABLES],
    /// T1/T3/T5 OldReference keys and derived T7 property keys share one runtime-pointer domain
    /// even though they live in separate serialized TMaps. A cross-table reuse is therefore just
    /// as unsafe as a collision within one table.
    contributed_ptr_tables: HashMap<i64, usize>,
    contributed_static_names: Vec<String>,
    /// Collision-resistant binding to the pristine base or the last cache returned by compose_*.
    expected_running_sha256: [u8; 32],
    usage: SequentialUsage,
    composition_state_valid: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct SequentialUsage {
    minis: u64,
    keyed_rows: u64,
    keyed_row_bytes: u64,
    static_names: u64,
    static_name_bytes: u64,
    composed_scan_bytes: u64,
}

struct SequentialMiniDelta {
    rows: [HashMap<i64, Vec<u8>>; N_TABLES],
    ptr_tables: HashMap<i64, usize>,
    static_names: Vec<String>,
    reference: super::remap::ReferenceContribution,
    usage: SequentialUsage,
}

fn checked_usage_add(
    current: u64,
    added: u64,
    resource: &'static str,
    limit: u64,
) -> Result<u64, SpliceError> {
    let actual = current.checked_add(added).unwrap_or(u64::MAX);
    if actual > limit {
        return Err(SpliceError::SequentialLimitExceeded {
            resource,
            actual,
            limit,
        });
    }
    Ok(actual)
}

fn checked_composed_capacity(parts: &[usize]) -> Result<usize, SpliceError> {
    let actual = parts
        .iter()
        .try_fold(0usize, |sum, &part| sum.checked_add(part))
        .and_then(|sum| u64::try_from(sum).ok())
        .unwrap_or(u64::MAX);
    if actual > MAX_COMPOSED_CACHE_BYTES {
        return Err(SpliceError::SequentialLimitExceeded {
            resource: "composed cache bytes",
            actual,
            limit: MAX_COMPOSED_CACHE_BYTES,
        });
    }
    Ok(actual as usize)
}

impl SequentialUsage {
    fn with_static_names(mut self, names: &[String]) -> Result<Self, SpliceError> {
        self.static_names = u64::try_from(names.len()).unwrap_or(u64::MAX);
        self.static_name_bytes = names.iter().try_fold(0u64, |total, name| {
            checked_usage_add(
                total,
                u64::try_from(name.len()).unwrap_or(u64::MAX),
                "StaticNames bytes",
                MAX_SEQUENTIAL_T6_NAME_BYTES,
            )
        })?;
        Ok(self)
    }

    fn project(self, delta: Self) -> Result<Self, SpliceError> {
        Ok(Self {
            minis: checked_usage_add(self.minis, delta.minis, "mini count", MAX_SEQUENTIAL_MINIS)?,
            keyed_rows: checked_usage_add(
                self.keyed_rows,
                delta.keyed_rows,
                "keyed row count",
                MAX_SEQUENTIAL_KEYED_ROWS,
            )?,
            keyed_row_bytes: checked_usage_add(
                self.keyed_row_bytes,
                delta.keyed_row_bytes,
                "keyed row bytes",
                MAX_SEQUENTIAL_KEYED_ROW_BYTES,
            )?,
            static_names: checked_usage_add(
                self.static_names,
                delta.static_names,
                "StaticNames count",
                MAX_SEQUENTIAL_T6_NAMES,
            )?,
            static_name_bytes: checked_usage_add(
                self.static_name_bytes,
                delta.static_name_bytes,
                "StaticNames bytes",
                MAX_SEQUENTIAL_T6_NAME_BYTES,
            )?,
            composed_scan_bytes: checked_usage_add(
                self.composed_scan_bytes,
                delta.composed_scan_bytes,
                "composed validation scan bytes",
                MAX_SEQUENTIAL_COMPOSED_SCAN_BYTES,
            )?,
        })
    }
}

/// Bound table work before the allocating parser builds key/offset vectors. Prepared minis are
/// intentionally minimal, so a single mini never needs more rows/names than the whole sequential
/// state can retain.
fn preflight_sequential_tail_with_limits(
    bytes: &[u8],
    start: usize,
    max_keyed_rows: u64,
    max_keyed_row_bytes: u64,
) -> Result<(), SpliceError> {
    let mut c = Cursor::at(bytes, start);
    let mut keyed_rows = 0u64;
    let mut keyed_bytes = 0u64;

    let mut keyed_table = |c: &mut Cursor<'_>,
                           field: &'static str,
                           key_bytes: usize,
                           read_value: fn(&mut Cursor<'_>) -> Result<(), WireError>|
     -> Result<(), SpliceError> {
        let count = c.read_count(field)?;
        keyed_rows = checked_usage_add(
            keyed_rows,
            u64::try_from(count).unwrap_or(u64::MAX),
            "keyed row count",
            max_keyed_rows,
        )?;
        let entries_start = c.pos();
        for _ in 0..count {
            c.skip(key_bytes)?;
            read_value(c)?;
        }
        keyed_bytes = checked_usage_add(
            keyed_bytes,
            u64::try_from(c.pos() - entries_start).unwrap_or(u64::MAX),
            "keyed row bytes",
            max_keyed_row_bytes,
        )?;
        Ok(())
    };

    keyed_table(&mut c, "TypeReferences", 8, preflight_type_reference)?;
    keyed_table(&mut c, "TypeIdReferenceToPointer", 4, |c| c.skip(8))?;
    keyed_table(
        &mut c,
        "FunctionReferences",
        8,
        preflight_function_reference,
    )?;
    keyed_table(&mut c, "FunctionIdReferenceToPointer", 4, |c| c.skip(8))?;
    keyed_table(&mut c, "GlobalReferences", 8, preflight_global_reference)?;

    let static_count = c.read_count("StaticNames")?;
    checked_usage_add(
        0,
        u64::try_from(static_count).unwrap_or(u64::MAX),
        "StaticNames count",
        MAX_SEQUENTIAL_T6_NAMES,
    )?;
    let mut static_bytes = 0u64;
    for _ in 0..static_count {
        let value = c.read_sia()?;
        static_bytes = checked_usage_add(
            static_bytes,
            u64::try_from(value.len()).unwrap_or(u64::MAX),
            "StaticNames bytes",
            MAX_SEQUENTIAL_T6_NAME_BYTES,
        )?;
    }
    keyed_table(
        &mut c,
        "PropertyReferences",
        8,
        preflight_property_reference,
    )?;
    Ok(())
}

fn preflight_sequential_tail(bytes: &[u8], start: usize) -> Result<(), SpliceError> {
    preflight_sequential_tail_with_limits(
        bytes,
        start,
        MAX_SEQUENTIAL_KEYED_ROWS,
        MAX_SEQUENTIAL_KEYED_ROW_BYTES,
    )
}

fn preflight_type_reference(c: &mut Cursor<'_>) -> Result<(), WireError> {
    c.read_sia()?;
    c.read_sia()?;
    c.read_sia()?;
    c.skip_tarray_fixed(36, "TypeRef.SubTypes")
}

fn preflight_function_reference(c: &mut Cursor<'_>) -> Result<(), WireError> {
    c.read_sia()?;
    c.read_sia()?;
    c.read_sia()?;
    c.skip(20)?;
    c.skip_tarray_fixed(36, "FuncRef.ParameterTypes")?;
    c.skip(36)
}

fn preflight_global_reference(c: &mut Cursor<'_>) -> Result<(), WireError> {
    let name_pos = c.pos();
    let name = c.read_sia_bytes()?;
    c.read_sia()?;
    c.read_sia()?;
    if c.read_bool4()? {
        name.decode_utf8(name_pos)?;
    }
    Ok(())
}

fn preflight_property_reference(c: &mut Cursor<'_>) -> Result<(), WireError> {
    c.read_sia()?;
    c.skip(4)
}

impl SequentialMiniGuard {
    /// Bind the guard to the exact base all incoming minis were independently remapped against.
    pub fn new(base: &[u8]) -> Result<Self, SpliceError> {
        let header = CacheHeader::parse(base)?;
        // A raw-file component can replace the effective script base before Manager composition.
        // Reject an oversized or record-amplified base before any StaticName, identity, module-name,
        // or tail-row collections are materialized.
        checked_composed_capacity(&[base.len()])?;
        super::remap::preflight_cache_module_work(base)?;
        let base_tail = module_region_end(base)?;
        preflight_sequential_tail_with_limits(
            base,
            base_tail,
            MAX_BASE_KEYED_ROWS,
            MAX_BASE_KEYED_ROW_BYTES,
        )?;
        let static_context = match super::remap::StaticNameRebaseContext::build(base) {
            Err(super::remap::RemapError::ModuleNameCollision { name }) => {
                return Err(SpliceError::InnerNameCollision(name));
            }
            Err(error) => return Err(SpliceError::StaticNameRebase(error)),
            Ok(context) => context,
        };
        let reference_context = match super::remap::EffectiveReferenceBase::build(base) {
            Err(super::remap::RemapError::ModuleNameCollision { name }) => {
                return Err(SpliceError::InnerNameCollision(name));
            }
            Err(error) => return Err(SpliceError::StaticNameRebase(error)),
            Ok(context) => context,
        };
        let base_guid = header.hash;
        let base_tables = parse_tail_tables(base, base_tail)?;
        if base_tables.end != base.len() {
            return Err(SpliceError::TailNotAtEof {
                which: "base",
                got: base_tables.end,
                len: base.len(),
            });
        }
        let mut base_rows: [HashMap<i64, Vec<u8>>; N_TABLES] =
            std::array::from_fn(|_| HashMap::new());
        let mut base_ptr_tables = HashMap::new();
        for (table, rows) in base_tables.tables.iter().enumerate() {
            if table == 5 {
                continue;
            }
            for (row_index, &key) in rows.keys.iter().enumerate() {
                let start = rows.entry_starts[row_index];
                let end = rows
                    .entry_starts
                    .get(row_index + 1)
                    .copied()
                    .unwrap_or(rows.entries_end);
                if base_rows[table]
                    .insert(key, base[start..end].to_vec())
                    .is_some()
                {
                    return Err(SpliceError::KeyCollision { table, key });
                }
                if matches!(table, 0 | 2 | 4 | 6) && base_ptr_tables.insert(key, table).is_some() {
                    return Err(SpliceError::KeyCollision { table, key });
                }
            }
        }
        Ok(Self {
            base: Arc::new(SequentialMiniBase {
                base_guid,
                reference_context,
                base_rows,
                base_ptr_tables,
                static_context,
            }),
            reference_state: super::remap::EffectiveReferenceState::default(),
            contributed_rows: std::array::from_fn(|_| HashMap::new()),
            contributed_ptr_tables: HashMap::new(),
            contributed_static_names: Vec::new(),
            expected_running_sha256: Sha256::digest(base).into(),
            usage: SequentialUsage::default(),
            composition_state_valid: true,
        })
    }

    /// Validate `mini`, return the StaticNames-rebased bytes, and record its rows atomically.
    ///
    /// This advances validation history only; callers publishing a composed cache should prefer
    /// [`Self::compose_add`] or [`Self::compose_edit`], which commit history only after the entire
    /// prospective cache passes structural validation.
    pub fn check_and_record(&mut self, mini: &[u8]) -> Result<Vec<u8>, SpliceError> {
        let (prepared, delta) = self.stage(mini)?;
        self.commit(delta);
        self.composition_state_valid = false;
        Ok(prepared)
    }

    /// Validate and append one mini to `running`, then atomically advance guard history.
    pub fn compose_add(&mut self, running: &[u8], mini: &[u8]) -> Result<Vec<u8>, SpliceError> {
        self.require_running_state(running)?;
        // Refuse an oversized prospective cache before `stage` parses bytecode or materializes
        // any per-function `Vec<i32>`. The exact low-level splice checks again after parsing; this
        // conservative bound is intentionally early and cannot under-estimate the output.
        let prospective = checked_composed_capacity(&[running.len(), mini.len()])?;
        let scan_bytes = u64::try_from(prospective)
            .unwrap_or(u64::MAX)
            .saturating_mul(2);
        checked_usage_add(
            self.usage.composed_scan_bytes,
            scan_bytes,
            "composed validation scan bytes",
            MAX_SEQUENTIAL_COMPOSED_SCAN_BYTES,
        )?;
        let (prepared, delta) = self.stage(mini)?;
        let composed = splice_auto(running, &prepared)?;
        self.base
            .reference_context
            .validate_composed_declarations(&composed)
            .map_err(SpliceError::ComposedModule)?;
        let mut delta = delta;
        delta.usage.composed_scan_bytes = scan_bytes;
        self.commit(delta);
        self.expected_running_sha256 = Sha256::digest(&composed).into();
        Ok(composed)
    }

    /// Validate and replace `target` in `running`, then atomically advance guard history.
    pub fn compose_edit(
        &mut self,
        running: &[u8],
        mini: &[u8],
        target: &str,
    ) -> Result<Vec<u8>, SpliceError> {
        self.require_running_state(running)?;
        // See `compose_add`: edits may replace bytes, but `running + mini` is a safe early upper
        // bound and prevents a near-limit mini from allocating a second near-GiB bytecode vector.
        let prospective = checked_composed_capacity(&[running.len(), mini.len()])?;
        let scan_bytes = u64::try_from(prospective)
            .unwrap_or(u64::MAX)
            .saturating_mul(2);
        checked_usage_add(
            self.usage.composed_scan_bytes,
            scan_bytes,
            "composed validation scan bytes",
            MAX_SEQUENTIAL_COMPOSED_SCAN_BYTES,
        )?;
        let (prepared, delta) = self.stage(mini)?;
        let composed = replace_module(running, &prepared, target)?;
        self.base
            .reference_context
            .validate_composed_declarations(&composed)
            .map_err(SpliceError::ComposedModule)?;
        let mut delta = delta;
        delta.usage.composed_scan_bytes = scan_bytes;
        self.commit(delta);
        self.expected_running_sha256 = Sha256::digest(&composed).into();
        Ok(composed)
    }

    fn require_running_state(&self, running: &[u8]) -> Result<(), SpliceError> {
        if !self.composition_state_valid
            || <[u8; 32]>::from(Sha256::digest(running)) != self.expected_running_sha256
        {
            return Err(SpliceError::RunningStateMismatch);
        }
        Ok(())
    }

    fn stage(&self, mini: &[u8]) -> Result<(Vec<u8>, SequentialMiniDelta), SpliceError> {
        // `check_and_record` reaches stage directly, so enforce the public one-cache size ceiling
        // before StaticName rebasing clones the complete mini.
        checked_composed_capacity(&[mini.len()])?;
        checked_usage_add(self.usage.minis, 1, "mini count", MAX_SEQUENTIAL_MINIS)?;
        let header = CacheHeader::parse(mini)?;
        let count = header.type_count;
        if count != 1 {
            return Err(SpliceError::MiniNotSingle(count));
        }
        let mini_guid = header.hash;
        if mini_guid != self.base.base_guid {
            return Err(SpliceError::MiniGuidMismatch {
                base: self.base.base_guid,
                mini: mini_guid,
            });
        }
        // This streaming pass runs before any detailed record or bytecode vectors are built. It
        // bounds both the largest function and the complete mini-cache bytecode payload so a
        // near-limit archive cannot force a second near-limit allocation during validation.
        check_bytecode_work(super::remap::preflight_mini_module_work(mini)?)?;
        let tail = module_region_end(mini)?;
        preflight_sequential_tail(mini, tail)?;
        let tables = parse_tail_tables(mini, tail)?;
        if tables.end != mini.len() {
            return Err(SpliceError::TailNotAtEof {
                which: "mini",
                got: tables.end,
                len: mini.len(),
            });
        }

        // Preflight without mutating state so a rejected mini does not poison a retry.
        const STATIC_NAMES: usize = 5;
        let mut within_ptr_domain = HashMap::new();
        for (table, rows) in tables.tables.iter().enumerate() {
            if table == STATIC_NAMES {
                continue;
            }
            let mut within_mini = HashSet::new();
            for (row_index, &key) in rows.keys.iter().enumerate() {
                if !within_mini.insert(key) {
                    return Err(SpliceError::SequentialKeyCollision { table, key });
                }
                if matches!(table, 0 | 2 | 4 | 6) {
                    let cross_table_within = within_ptr_domain
                        .insert(key, table)
                        .is_some_and(|prior| prior != table);
                    let cross_table_prior = self
                        .contributed_ptr_tables
                        .get(&key)
                        .is_some_and(|&prior| prior != table);
                    let cross_table_base = self
                        .base
                        .base_ptr_tables
                        .get(&key)
                        .is_some_and(|&prior| prior != table);
                    if cross_table_within || cross_table_prior {
                        return Err(SpliceError::SequentialKeyCollision { table, key });
                    }
                    if cross_table_base {
                        return Err(SpliceError::KeyCollision { table, key });
                    }
                }
                let start = rows.entry_starts[row_index];
                let end = rows
                    .entry_starts
                    .get(row_index + 1)
                    .copied()
                    .unwrap_or(rows.entries_end);
                if self.base.base_rows[table]
                    .get(&key)
                    .is_some_and(|base_row| base_row.as_slice() != &mini[start..end])
                {
                    return Err(SpliceError::KeyCollision { table, key });
                }
                if let Some(prior) = self.contributed_rows[table].get(&key) {
                    if prior.as_slice() != &mini[start..end] {
                        return Err(SpliceError::SequentialKeyCollision { table, key });
                    }
                }
            }
        }

        let reference_contribution = self
            .base
            .reference_context
            .validate(&self.reference_state, mini)
            .map_err(SpliceError::MiniReference)?;

        // Rebase before mutating either collision state or the accumulated T6 pool. A malformed
        // operand therefore leaves this guard reusable for a corrected mini.
        let (rebased, appended_names) = super::remap::rebase_static_names_for_composition(
            mini,
            &self.base.static_context,
            &self.contributed_static_names,
        )?;

        let mut delta_rows: [HashMap<i64, Vec<u8>>; N_TABLES] =
            std::array::from_fn(|_| HashMap::new());
        let mut delta_ptr_tables = HashMap::new();
        let mut delta_usage = SequentialUsage {
            minis: 1,
            ..SequentialUsage::default()
        };
        for (table, rows) in tables.tables.iter().enumerate() {
            if table != STATIC_NAMES {
                for (row_index, &key) in rows.keys.iter().enumerate() {
                    let start = rows.entry_starts[row_index];
                    let end = rows
                        .entry_starts
                        .get(row_index + 1)
                        .copied()
                        .unwrap_or(rows.entries_end);
                    // Exact base/prior repeats were already proven byte-identical and add no
                    // state. Keeping only novel rows makes commit linear in this mini's delta.
                    if self.base.base_rows[table].contains_key(&key)
                        || self.contributed_rows[table].contains_key(&key)
                    {
                        continue;
                    }
                    delta_rows[table].insert(key, mini[start..end].to_vec());
                    delta_usage.keyed_rows = checked_usage_add(
                        delta_usage.keyed_rows,
                        1,
                        "keyed row count",
                        MAX_SEQUENTIAL_KEYED_ROWS,
                    )?;
                    delta_usage.keyed_row_bytes = checked_usage_add(
                        delta_usage.keyed_row_bytes,
                        u64::try_from(end - start).unwrap_or(u64::MAX),
                        "keyed row bytes",
                        MAX_SEQUENTIAL_KEYED_ROW_BYTES,
                    )?;
                    if matches!(table, 0 | 2 | 4 | 6) {
                        delta_ptr_tables.insert(key, table);
                    }
                }
            }
        }
        let delta_usage = delta_usage.with_static_names(&appended_names)?;
        self.usage.project(delta_usage)?;
        Ok((
            rebased,
            SequentialMiniDelta {
                rows: delta_rows,
                ptr_tables: delta_ptr_tables,
                static_names: appended_names,
                reference: reference_contribution,
                usage: delta_usage,
            },
        ))
    }

    fn commit(&mut self, delta: SequentialMiniDelta) {
        let usage = self
            .usage
            .project(delta.usage)
            .expect("validated sequential usage cannot overflow while recording");
        for (target, rows) in self.contributed_rows.iter_mut().zip(delta.rows) {
            target.extend(rows);
        }
        self.contributed_ptr_tables.extend(delta.ptr_tables);
        self.contributed_static_names.extend(delta.static_names);
        self.reference_state.record(delta.reference);
        self.usage = usage;
    }
}

/// Append `mini`'s single referenceless module to `base`, returning the new cache bytes.
///
/// This is a low-level composition primitive. It validates the mechanical container shape but
/// does not establish generation binding or executable-reference authority. Publishing callers
/// must use [`SequentialMiniGuard::compose_add`] instead.
pub fn splice(base: &[u8], mini: &[u8]) -> Result<Vec<u8>, SpliceError> {
    let mini_n = module_count(mini);
    if mini_n != 1 {
        return Err(SpliceError::MiniNotSingle(mini_n));
    }

    let mini_tail = module_region_end(mini)?;
    let trailing = &mini[mini_tail..];
    if trailing.iter().any(|&x| x != 0) {
        return Err(SpliceError::MiniHasGlobalRefs(trailing.len()));
    }

    // The mini's module entry = TMap (FString key + module value) at [0x18 .. mini_tail].
    let mod_bytes = &mini[CacheHeader::SIZE..mini_tail];

    let new_name = module_names(mini)?.into_iter().next().unwrap_or_default();
    if module_names(base)?.iter().any(|n| n == &new_name) {
        return Err(SpliceError::NameCollision(new_name));
    }

    let base_tail = module_region_end(base)?;
    let base_count = module_count(base);

    let capacity = checked_composed_capacity(&[base.len(), mod_bytes.len()])?;
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(&base[..0x14]); // FGuid + magic
    out.extend_from_slice(&(base_count + 1).to_le_bytes()); // bumped Modules count
    out.extend_from_slice(&base[CacheHeader::SIZE..base_tail]); // all existing modules
    out.extend_from_slice(mod_bytes); // the new module, before the tail tables
    out.extend_from_slice(&base[base_tail..]); // global tail tables, unchanged
    finish_composition(out)
}

/// Auto-select the splice path: case-(b) fast append for a referenceless mini, else the
/// case-(a) global-table merge for a class/native-ref-bearing mini.
///
/// This is a low-level composition primitive. Before calling it, validate the mini with a
/// [`SequentialMiniGuard`] bound to the pristine base and compose the prepared bytes returned by
/// the guard. The caller owns sequential guard history; this function deliberately does not
/// construct or advance a guard itself.
pub fn splice_auto(base: &[u8], mini: &[u8]) -> Result<Vec<u8>, SpliceError> {
    let mini_n = module_count(mini);
    if mini_n != 1 {
        return Err(SpliceError::MiniNotSingle(mini_n));
    }
    let mini_tail = module_region_end(mini)?;
    if mini[mini_tail..].iter().all(|&x| x == 0) {
        splice(base, mini)
    } else {
        splice_case_a(base, mini)
    }
}

/// Case-(a): append the mini's module AND merge its 7 global tail-table entries into the
/// base cache (per `case-a-tables-and-exec.md` §3). Used when the mini references native
/// types/functions (its tail tables are non-empty), e.g. a class or a PrintString call.
///
/// This is a low-level composition primitive. Publishing callers must use
/// [`SequentialMiniGuard::compose_add`] so the mini is validated against its pristine base before
/// any rows are merged.
pub fn splice_case_a(base: &[u8], mini: &[u8]) -> Result<Vec<u8>, SpliceError> {
    let mini_n = module_count(mini);
    if mini_n != 1 {
        return Err(SpliceError::MiniNotSingle(mini_n));
    }

    let base_tail = module_region_end(base)?;
    let mini_tail = module_region_end(mini)?;

    let base_tt = parse_tail_tables(base, base_tail)?;
    if base_tt.end != base.len() {
        return Err(SpliceError::TailNotAtEof {
            which: "base",
            got: base_tt.end,
            len: base.len(),
        });
    }
    let mini_tt = parse_tail_tables(mini, mini_tail)?;
    if mini_tt.end != mini.len() {
        return Err(SpliceError::TailNotAtEof {
            which: "mini",
            got: mini_tt.end,
            len: mini.len(),
        });
    }

    // Module name collision.
    let new_name = module_names(mini)?.into_iter().next().unwrap_or_default();
    if module_names(base)?.iter().any(|n| n == &new_name) {
        return Err(SpliceError::NameCollision(new_name));
    }

    let mod_bytes = &mini[CacheHeader::SIZE..mini_tail];
    let base_count = module_count(base);

    let capacity =
        checked_composed_capacity(&[base.len(), mod_bytes.len(), mini.len() - mini_tail])?;
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(&base[..0x14]);
    out.extend_from_slice(&(base_count + 1).to_le_bytes());
    out.extend_from_slice(&base[CacheHeader::SIZE..base_tail]); // existing modules
    out.extend_from_slice(mod_bytes); // new module, before tables

    append_merged_tables(&mut out, base, &base_tt, mini, &mini_tt);
    finish_composition(out)
}

/// Append the 7 merged tail tables (base ++ mini, deduped) to `out`.
///
/// Every keyed table is `TMap<key, V>` whose KEY (an int32 engine id for tables 1 & 3, an
/// int64 `OldReference` original-pointer-id for tables 0/2/4/6) is DETERMINISTIC for a given
/// build (see `container-splice.md` §4). A recompiled mini re-exports references the edited
/// module touches — types, functions, globals, properties — some of which already exist in
/// the base. Concatenating verbatim would emit duplicate TMap keys; keeping the BASE row on a
/// collision would discard the mini's freshly compiled metadata for refs the new bytecode
/// actually expects. So on a key collision the MINI wins: drop the base's colliding row and
/// take the mini's, which both removes the duplicate key and keeps the up-to-date payload.
/// Table 5 (StaticNames) is an unkeyed `TArray<FString>` where duplicates are harmless →
/// append verbatim.
fn append_merged_tables(
    out: &mut Vec<u8>,
    base: &[u8],
    base_tt: &TailTables,
    mini: &[u8],
    mini_tt: &TailTables,
) {
    const STATIC_NAMES: usize = 5;
    for i in 0..N_TABLES {
        let b = &base_tt.tables[i];
        let m = &mini_tt.tables[i];
        if i == STATIC_NAMES {
            out.extend_from_slice(&(b.count + m.count).to_le_bytes());
            out.extend_from_slice(&base[b.entries_start..b.entries_end]);
            out.extend_from_slice(&mini[m.entries_start..m.entries_end]);
            continue;
        }
        // Keep base rows the mini does NOT redefine, then append every mini row (mini wins on
        // collision, replacing stale base metadata while avoiding a duplicate key).
        let mini_keys: HashSet<i64> = m.keys.iter().copied().collect();
        let mut kept = Vec::new();
        let mut kept_count = 0u32;
        for (j, &k) in b.keys.iter().enumerate() {
            if mini_keys.contains(&k) {
                continue;
            }
            let start = b.entry_starts[j];
            let end = b.entry_starts.get(j + 1).copied().unwrap_or(b.entries_end);
            kept.extend_from_slice(&base[start..end]);
            kept_count += 1;
        }
        out.extend_from_slice(&(kept_count + m.count).to_le_bytes());
        out.extend_from_slice(&kept);
        out.extend_from_slice(&mini[m.entries_start..m.entries_end]);
    }
}

/// Extract one module from `cache` into a standalone 1-module mini-cache: its `Modules`
/// TMap entry followed by the cache's FULL global tail tables. Used to pull a dependency-
/// heavy module (which can't be compiled standalone) out of a full-tree regen so it can be
/// [`replace_module`]'d into the vanilla base. Extraction retains the source cache's FGuid; use
/// [`remap_module_to_base`] before targeting a different base. The full tail tables guarantee
/// every ref the module's bytecode uses is present, and `replace_module`'s merge folds them into
/// the base.
pub fn extract_module(cache: &[u8], target_name: &str) -> Result<Vec<u8>, SpliceError> {
    super::remap::preflight_cache_module_work(cache)?;
    super::remap::preflight_tail_tables(cache)?;
    let ranges = module_ranges(cache)?;
    let matches: Vec<_> = ranges
        .iter()
        .filter(|(n, _, _)| n == target_name)
        .cloned()
        .collect();
    let (_, start, end) = match matches.as_slice() {
        [entry] => entry.clone(),
        [] => return Err(SpliceError::NameNotFound(target_name.to_string())),
        _ => return Err(SpliceError::AmbiguousTarget(target_name.to_string())),
    };
    let tail_off = module_region_end(cache)?;
    let mut out = Vec::with_capacity(0x18 + (end - start) + (cache.len() - tail_off));
    out.extend_from_slice(&cache[..0x14]); // FGuid + magic
    out.extend_from_slice(&1u32.to_le_bytes()); // Modules count = 1
    out.extend_from_slice(&cache[start..end]); // the one module's TMap entry
    out.extend_from_slice(&cache[tail_off..]); // full global tail tables
    Ok(out)
}

/// Rewrite an extracted (regen-tables) 1-module mini's bytecode refs to the VANILLA `base`'s
/// keys/ids and return a NEW 1-module mini with EMPTY tail tables (28 zero bytes), so it can be
/// [`replace_module`]'d into `base` without appending any tail-table rows.
///
/// This is the REF-REMAPPING step (`work/reversing/gore-as/specs/ref-remap.md`): the 7 global
/// tail tables are keyed by runtime pointers/ids captured at serialization, so a full-tree regen
/// assigns DIFFERENT keys than vanilla for the SAME symbols. Splicing the regen mini verbatim
/// would append every regen row (cache grows ~22 MB → boot crash). Here we resolve each ref
/// operand by SYMBOL IDENTITY (name + module + namespace + signature) to the base's key, then
/// ship empty tables so the merge adds nothing and the cache stays vanilla-sized.
///
/// Returns the remapped mini bytes. Any ref whose symbol is not present in `base` is a HARD
/// ERROR; use [`remap_module_to_base_with_options`] for the explicit minimal-row opt-in.
pub fn remap_module_to_base(
    extracted_mini: &[u8],
    base: &[u8],
) -> Result<Vec<u8>, super::remap::RemapError> {
    super::remap::remap_module_to_base(extracted_mini, base).map(|(bytes, _counts)| bytes)
}

/// Option-bearing variant of [`remap_module_to_base`]. See
/// [`super::remap::RemapOptions::allow_new_symbols`] for the explicit minimal-row opt-in.
pub fn remap_module_to_base_with_options(
    extracted_mini: &[u8],
    base: &[u8],
    options: super::remap::RemapOptions,
) -> Result<Vec<u8>, super::remap::RemapError> {
    super::remap::remap_module_to_base_with_options(extracted_mini, base, options)
        .map(|(bytes, _counts)| bytes)
}

/// Replace an existing module in `base` with `new_mini`'s single module, keeping the
/// `Modules` count UNCHANGED. The new module's tail-table entries are merged into `base`'s
/// the same way [`splice_case_a`] does (per `case-a-tables-and-exec.md` §3): see
/// [`append_merged_tables`] — on a key collision the mini's row wins; StaticNames appends.
///
/// This is the decompiler edit loop: decompile a module → edit the `.as` → the game
/// recompiles it to a mini-cache → `replace_module` swaps it in.
///
/// NOTE: the OLD module's now-orphaned tail-table entries are NOT removed — they are
/// name-resolved at load and harmless (per `container-splice.md` §4); only the new
/// entries are appended.
///
/// This is a low-level composition primitive. Before calling it, validate `new_mini` with a
/// [`SequentialMiniGuard`] bound to the pristine base and replace with the prepared bytes returned
/// by the guard. The caller owns sequential guard history; this function deliberately does not
/// construct or advance a guard itself.
pub fn replace_module(
    base: &[u8],
    new_mini: &[u8],
    target_name: &str,
) -> Result<Vec<u8>, SpliceError> {
    let mini_n = module_count(new_mini);
    if mini_n != 1 {
        return Err(SpliceError::MiniNotSingle(mini_n));
    }

    // Locate the target module's whole TMap-entry byte range in the base. `module_ranges`
    // keys by the `Modules` TMap key; `emit` labels output by the inner `ModuleName`, which
    // can differ. Match the key first, then fall back to the inner name so a modder using the
    // emitted module name as `target` doesn't get a spurious NameNotFound.
    let ranges = module_ranges(base)?;
    let inner_names = module_inner_names(base, &ranges)?;
    let outer_matches: Vec<usize> = ranges
        .iter()
        .enumerate()
        .filter_map(|(index, (name, _, _))| (name == target_name).then_some(index))
        .collect();
    let idx = match outer_matches.as_slice() {
        [i] => {
            let i = *i;
            // The TMap key matched. Guard against the pathological case where `target` ALSO
            // equals a DIFFERENT module's inner name: silently replacing the key match could
            // be the wrong module, so refuse and require the exact key.
            let collides = inner_names
                .iter()
                .enumerate()
                .any(|(j, name)| j != i && name == target_name);
            if collides {
                return Err(SpliceError::AmbiguousTarget(target_name.to_string()));
            }
            i
        }
        [] => {
            // Fall back to the inner `ModuleName`, but ONLY if it's unambiguous: if several
            // entries share that inner name (different TMap keys), we can't tell which to
            // replace, so refuse rather than corrupt the wrong byte range. Propagate a
            // base-parse failure instead of masking it as NameNotFound.
            let inner: Vec<usize> = inner_names
                .iter()
                .enumerate()
                .filter(|(i, name)| *i < ranges.len() && *name == target_name)
                .map(|(i, _)| i)
                .collect();
            match inner.as_slice() {
                [i] => *i,
                [] => return Err(SpliceError::NameNotFound(target_name.to_string())),
                _ => return Err(SpliceError::AmbiguousTarget(target_name.to_string())),
            }
        }
        _ => return Err(SpliceError::AmbiguousTarget(target_name.to_string())),
    };
    let (_, target_start, target_end) = ranges[idx].clone();

    // Renaming onto an already-occupied key would write two entries under one module name
    // while leaving the count unchanged — an ambiguous TMap. Reject a replacement whose name
    // collides with a DIFFERENT base module. Exclude the module being replaced BY INDEX (its
    // own key matching is an in-place replace) — `target_name` may be the inner ModuleName,
    // not the TMap key, so comparing against it would miss the self-match. Mirrors the
    // `splice_case_a` collision guard.
    let new_name = module_names(new_mini)?
        .into_iter()
        .next()
        .unwrap_or_default();
    if module_names(base)?
        .iter()
        .enumerate()
        .any(|(i, n)| i != idx && n == &new_name)
    {
        return Err(SpliceError::NameCollision(new_name));
    }

    let base_tail = module_region_end(base)?;
    let base_tt = parse_tail_tables(base, base_tail)?;
    if base_tt.end != base.len() {
        return Err(SpliceError::TailNotAtEof {
            which: "base",
            got: base_tt.end,
            len: base.len(),
        });
    }

    let mini_tail = module_region_end(new_mini)?;
    let mini_tt = parse_tail_tables(new_mini, mini_tail)?;
    if mini_tt.end != new_mini.len() {
        return Err(SpliceError::TailNotAtEof {
            which: "mini",
            got: mini_tt.end,
            len: new_mini.len(),
        });
    }

    // The new module entry = TMap (FString key + module value) at [0x18 .. mini_tail].
    let mod_bytes = &new_mini[CacheHeader::SIZE..mini_tail];

    // out = base[0..target_start] ++ MOD_BYTES ++ base[target_end..base_tail] ++ MERGED.
    // Header (incl. Modules count @0x14) stays byte-identical: count is unchanged.
    let capacity =
        checked_composed_capacity(&[base.len(), mod_bytes.len(), new_mini.len() - mini_tail])?;
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(&base[..target_start]); // header + modules before target
    out.extend_from_slice(mod_bytes); // replacement module
    out.extend_from_slice(&base[target_end..base_tail]); // modules after target
    append_merged_tables(&mut out, base, &base_tt, new_mini, &mini_tt);
    finish_composition(out)
}

/// Read only each module's inner runtime identity from the already located TMap-entry ranges.
/// This avoids `model::parse_modules`, which would clone every function bytecode array merely to
/// resolve an edit target in a potentially hundreds-of-megabytes cache.
fn module_inner_names(
    bytes: &[u8],
    ranges: &[(String, usize, usize)],
) -> Result<Vec<String>, WireError> {
    ranges
        .iter()
        .map(|(_, start, _)| {
            let mut cursor = Cursor::at(bytes, *start);
            cursor.read_fstring()?;
            cursor.read_sia()
        })
        .collect()
}

#[cfg(test)]
mod resource_limit_tests {
    use super::*;

    #[test]
    fn composed_capacity_accepts_the_limit_and_rejects_larger_or_overflowing_inputs() {
        assert_eq!(
            checked_composed_capacity(&[MAX_COMPOSED_CACHE_BYTES as usize]).unwrap(),
            MAX_COMPOSED_CACHE_BYTES as usize
        );

        assert!(matches!(
            checked_composed_capacity(&[MAX_COMPOSED_CACHE_BYTES as usize, 1]),
            Err(SpliceError::SequentialLimitExceeded {
                resource: "composed cache bytes",
                actual,
                limit: MAX_COMPOSED_CACHE_BYTES,
            }) if actual == MAX_COMPOSED_CACHE_BYTES + 1
        ));

        assert!(matches!(
            checked_composed_capacity(&[usize::MAX, 1]),
            Err(SpliceError::SequentialLimitExceeded {
                resource: "composed cache bytes",
                actual: u64::MAX,
                limit: MAX_COMPOSED_CACHE_BYTES,
            })
        ));
    }
}
