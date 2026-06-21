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
use super::tables::{parse_tail_tables, N_TABLES};
use super::walk_modules::{module_count, module_names, module_region_end};
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

    // Per-table OldReference key-collision guard (keys resolve by name at load, so they only
    // need to be unique within the base table).
    for i in 0..N_TABLES {
        let base_keys: HashSet<i64> = base_tt.tables[i].keys.iter().copied().collect();
        if let Some(&k) = mini_tt.tables[i].keys.iter().find(|k| base_keys.contains(k)) {
            return Err(SpliceError::KeyCollision { table: i, key: k });
        }
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

    // Merged tables: for each of the 7, write (base+mini count) then base entries then mini entries.
    for i in 0..N_TABLES {
        let b = &base_tt.tables[i];
        let m = &mini_tt.tables[i];
        out.extend_from_slice(&(b.count + m.count).to_le_bytes());
        out.extend_from_slice(&base[b.entries_start..b.entries_end]);
        out.extend_from_slice(&mini[m.entries_start..m.entries_end]);
    }
    Ok(out)
}
