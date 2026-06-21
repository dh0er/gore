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

use super::header::CacheHeader;
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
