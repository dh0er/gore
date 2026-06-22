//! Splice one game-compiled module into the shipped cache (Weg B).
//!
//! Per `work/reversing/gore-as/findings/container-splice.md` §5: insert the mini-cache's
//! single module entry immediately before the base cache's global tail tables, and bump
//! the `Modules` count at 0x14 by one. The load path has no integrity check (§6), so no
//! checksum/GUID fix is needed.
//!
//! This implements the **case-(b)** path only: the mini module must reference no global
//! types (its 7 tail tables empty = 28 zero bytes). Class-bearing / type-referencing
//! modules need the case-(a) global-table merge (§9.7) — not yet implemented.

use std::collections::HashSet;

use super::header::CacheHeader;
use super::tables::{parse_tail_tables, TailTables, N_TABLES};
use super::walk_modules::{module_count, module_names, module_ranges, module_region_end};
use super::wire::WireError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpliceError {
    #[error("walk error: {0}")]
    Wire(#[from] WireError),
    #[error("mini-cache must contain exactly 1 module, found {0}")]
    MiniNotSingle(u32),
    #[error("mini-cache has non-empty global tables ({0} trailing bytes): it references global types/functions — needs the case-(a) table merge (not yet implemented). Keep the seed script to free, primitive-only functions (no classes).")]
    MiniHasGlobalRefs(usize),
    #[error("module name {0:?} already exists in the base cache (splicing would overwrite it)")]
    NameCollision(String),
    #[error("module name {0:?} not found in the base cache (nothing to replace)")]
    NameNotFound(String),
    #[error("module name {0:?} is ambiguous: it matches one module's TMap key and a different module's inner name — pass the exact TMap key")]
    AmbiguousTarget(String),
    #[error("tail tables of {which} don't end at EOF (ended {got:#x}, len {len:#x}) — parse desync")]
    TailNotAtEof {
        which: &'static str,
        got: usize,
        len: usize,
    },
    #[error("OldReference key collision in table {table} ({key:#x}) — would need rekeying")]
    KeyCollision { table: usize, key: i64 },
}

/// Append `mini`'s single (primitive-only) module to `base`, returning the new cache bytes.
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

    let mut out = Vec::with_capacity(base.len() + mod_bytes.len());
    out.extend_from_slice(&base[..0x14]); // FGuid + magic
    out.extend_from_slice(&(base_count + 1).to_le_bytes()); // bumped Modules count
    out.extend_from_slice(&base[CacheHeader::SIZE..base_tail]); // all existing modules
    out.extend_from_slice(mod_bytes); // the new module, before the tail tables
    out.extend_from_slice(&base[base_tail..]); // global tail tables, unchanged
    Ok(out)
}

/// Auto-select the splice path: case-(b) fast append for a referenceless mini, else the
/// case-(a) global-table merge for a class/native-ref-bearing mini.
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

    let mut out = Vec::with_capacity(base.len() + mod_bytes.len() + (mini.len() - mini_tail));
    out.extend_from_slice(&base[..0x14]);
    out.extend_from_slice(&(base_count + 1).to_le_bytes());
    out.extend_from_slice(&base[CacheHeader::SIZE..base_tail]); // existing modules
    out.extend_from_slice(mod_bytes); // new module, before tables

    append_merged_tables(&mut out, base, &base_tt, mini, &mini_tt);
    Ok(out)
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
    let idx = match ranges.iter().position(|(name, _, _)| name == target_name) {
        Some(i) => {
            // The TMap key matched. Guard against the pathological case where `target` ALSO
            // equals a DIFFERENT module's inner name: silently replacing the key match could
            // be the wrong module, so refuse and require the exact key.
            let collides = super::model::parse_modules(base)?
                .iter()
                .enumerate()
                .any(|(j, m)| j != i && j < ranges.len() && m.name == target_name);
            if collides {
                return Err(SpliceError::AmbiguousTarget(target_name.to_string()));
            }
            i
        }
        None => {
            // Fall back to the inner `ModuleName`, but ONLY if it's unambiguous: if several
            // entries share that inner name (different TMap keys), we can't tell which to
            // replace, so refuse rather than corrupt the wrong byte range. Propagate a
            // base-parse failure instead of masking it as NameNotFound.
            let mods = super::model::parse_modules(base)?;
            let inner: Vec<usize> = mods
                .iter()
                .enumerate()
                .filter(|(i, m)| *i < ranges.len() && m.name == target_name)
                .map(|(i, _)| i)
                .collect();
            match inner.as_slice() {
                [i] => *i,
                _ => return Err(SpliceError::NameNotFound(target_name.to_string())),
            }
        }
    };
    let (_, target_start, target_end) = ranges[idx].clone();

    // Renaming onto an already-occupied key would write two entries under one module name
    // while leaving the count unchanged — an ambiguous TMap. Reject a replacement whose name
    // collides with a DIFFERENT base module. Exclude the module being replaced BY INDEX (its
    // own key matching is an in-place replace) — `target_name` may be the inner ModuleName,
    // not the TMap key, so comparing against it would miss the self-match. Mirrors the
    // `splice_case_a` collision guard.
    let new_name = module_names(new_mini)?.into_iter().next().unwrap_or_default();
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
    let mut out =
        Vec::with_capacity(base.len() + mod_bytes.len() + (new_mini.len() - mini_tail));
    out.extend_from_slice(&base[..target_start]); // header + modules before target
    out.extend_from_slice(mod_bytes); // replacement module
    out.extend_from_slice(&base[target_end..base_tail]); // modules after target
    append_merged_tables(&mut out, base, &base_tt, new_mini, &mini_tt);
    Ok(out)
}
