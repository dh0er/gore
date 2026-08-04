//! `Binds.Cache` native-API arity table.
//!
//! `Binds.Cache` is the Hazelight UnrealEngine-Angelscript *bind database* dumped for
//! a specific shipped build (see `work/reversing/gore-as/findings/binds-cache.md`). It is
//! a flat record stream — NOT the `FGuid`+magic precompiled-cache container:
//!
//! ```text
//! file    := u32 count, record[]
//! record  := u32 typeNameLen, typeName(NUL-term)        e.g. "FGuid\0"
//!            u32 pathLen,     scriptPath(NUL-term)       e.g. "/Script/CoreUObject.Guid\0"
//!            u32 fieldCount
//!            field[fieldCount]
//! field   := u32 declLen, declString(NUL-term)  "<TYPE> <Name>" OR a full "<ret> Name(params) const"
//!            u32 nameLen, name(NUL-term)         bare member/method name
//!            <variable-width metadata slot>      (see below)
//! ```
//!
//! The per-field metadata slot is the awkward part. For plain *struct* fields it is a fixed
//! 32-byte run of `0x00`. For *function*/library fields it carries a small binding record
//! (flags, a `0xffff` marker, and an embedded length-prefixed `"Script"` string) of varying
//! width. Because the slot width is not uniform, this parser does NOT trust a fixed stride.
//! It locates record boundaries with the type/path signature, then scans each bounded record
//! region for an adjacent declaration/name pair whose identifiers agree.
//!
//! Three independent passes populate the tables:
//! 1. A conservative **record parse** locates record boundaries via the strong
//!    `(typeNameLen, typeName, pathLen, "/Script/…")` signature, then walks fields to build the
//!    `(class, name) -> arity` map.
//! 2. For a SHA-256-sealed audited build only, an exhaustive, declaration/name-agreeing
//!    **plain-field scan** builds `(class, field) -> type` mutation witnesses.
//! 3. A class-agnostic **printable-run scan** collects every `[\x20-\x7e]{4,}` run containing
//!    `(` and `)` to build the `name -> arity` map. This is the high-coverage backstop (the
//!    record parse stays in sync for most but not every record).
//!
//! A *signature* is any decl string containing both `(` and `)`. Arity = number of top-level
//! comma-separated parameters inside the OUTERMOST parens, ignoring commas nested in `<…>`
//! template brackets or inner `(…)`. Empty parens => 0. Defaulted params (`= nullptr`) count.

use std::collections::HashMap;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Does the generation this script-cache GUID names seal the exact `Binds.Cache` these tables were
/// built from?
///
/// Existing anywhere in the table is not enough, and the comment that used to sit here asserted a
/// property the code did not have. Generations do not all carry the same Binds file — one build
/// moved it — and their sealed field-map digests differ accordingly. A GUID-only gate therefore
/// handed one generation's field map to another generation's cache as mutation evidence, which is
/// reachable in practice because the loader takes the Binds path from `GORE_AS_BINDS` or from
/// beside the cache, and neither is required to belong to the same install as the cache itself.
///
/// `loaded` is `None` when the bytes these tables came from matched no sealed Binds file at all, in
/// which case the maps are empty anyway and nothing can be admitted.
fn is_verified_default_pairing(
    script_cache_guid: &[u8; 16],
    loaded: Option<&[u8; 32]>,
) -> bool {
    let Some(loaded) = loaded else { return false };
    gore_generation::row_for_script_cache_guid(script_cache_guid)
        .is_some_and(|row| row.binds_cache.sha256 == *loaded)
}

type VerifiedDefaultClassProfileDigests = ([u8; 32], [u8; 32]);

/// Native AngelScript method/function arities extracted from `Binds.Cache`.
#[derive(Debug)]
pub struct NativeApi {
    /// Exact `(class, method) -> arity`. Built from the record parse; entries with conflicting
    /// arities for the same key are removed, so a present entry is unambiguous.
    by_class: HashMap<(String, String), usize>,
    /// `name -> Some(arity)` when every signature with that name shares one arity, else `None`.
    /// Built from the printable-run scan (full coverage, class-agnostic).
    by_name: HashMap<String, Option<usize>>,
    /// Plain STRUCT-FIELD value types: `(class, field) -> type` from the two-token
    /// `"<TYPE> <Name>"` decls (no parens = not a signature). Conflicting keys are dropped,
    /// same policy as the arity map. batch-25a (specs/batch23-cantconvert.md G2): resolves
    /// native-struct enum field types the script cache cannot (PropertyReferences.OldTypeId
    /// is the OWNER struct, not the field's value type).
    field_types: HashMap<(String, String), String>,
    /// High-coverage native field rows are a mutation witness only for a sealed, audited
    /// Binds.Cache identity. Unknown versions keep this empty and therefore fail closed.
    verified_default_field_types: HashMap<(String, String), String>,
    /// Exact AngelScript type name to qualified Unreal class/struct path bridge. Populated only
    /// for the sealed Binds bytes and sealed parser-output digest; a USMAP profile independently
    /// filters this to classes before it can become ancestry evidence.
    verified_default_class_paths: HashMap<String, String>,
    /// Actual digests observed while constructing the sealed class bridge. Stored rather than
    /// inferred from hard-coded constants so downstream evidence IDs bind the live verified
    /// tuple that produced this instance.
    verified_default_class_profile_digests: Option<VerifiedDefaultClassProfileDigests>,
    /// SHA-256 of the `Binds.Cache` bytes these sealed tables were admitted from, when they were.
    /// Kept because the identity is what makes them evidence: without it, "this map is sealed" and
    /// "this map is sealed *for the cache in front of us*" are the same sentence, and only the
    /// second one is true.
    verified_default_binds_sha256: Option<[u8; 32]>,
}

impl NativeApi {
    #[cfg(test)]
    pub(crate) fn from_test_arities(
        exact: &[(&str, &str, usize)],
        by_name: &[(&str, Option<usize>)],
    ) -> NativeApi {
        NativeApi {
            by_class: exact
                .iter()
                .map(|(class, name, arity)| ((class.to_string(), name.to_string()), *arity))
                .collect(),
            by_name: by_name
                .iter()
                .map(|(name, arity)| (name.to_string(), *arity))
                .collect(),
            field_types: HashMap::new(),
            verified_default_field_types: HashMap::new(),
            verified_default_class_paths: HashMap::new(),
            verified_default_class_profile_digests: None,
            verified_default_binds_sha256: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_test_field_types(
        generic: &[(&str, &str, &str)],
        verified: &[(&str, &str, &str)],
        // The `Binds.Cache` identity the verified map is supposed to have come from. `None` builds
        // the shape a file matching no seal produces, where nothing is admitted for any GUID.
        sealed_for: Option<[u8; 32]>,
    ) -> NativeApi {
        let fields = |rows: &[(&str, &str, &str)]| {
            rows.iter()
                .map(|(class, field, value_type)| {
                    (
                        (class.to_string(), field.to_string()),
                        value_type.to_string(),
                    )
                })
                .collect()
        };
        NativeApi {
            by_class: HashMap::new(),
            by_name: HashMap::new(),
            field_types: fields(generic),
            verified_default_field_types: fields(verified),
            verified_default_class_paths: HashMap::new(),
            verified_default_class_profile_digests: None,
            verified_default_binds_sha256: sealed_for,
        }
    }

    /// Parse `Binds.Cache`. Returns `None` on any IO/parse failure (caller treats absence as
    /// "no data").
    pub fn load(path: &Path) -> Option<NativeApi> {
        let data = std::fs::read(path).ok()?;
        Self::from_bytes(&data)
    }

    /// Parse already bounded `Binds.Cache` bytes. The caller is responsible for applying an input
    /// size limit. CLI mutation paths use this entry point so the exact buffer they size-check and
    /// hash is also the buffer that supplies sealed evidence.
    pub fn from_bytes(data: &[u8]) -> Option<NativeApi> {
        if data.len() < 8 {
            return None;
        }
        let (by_class, field_types) = parse_records(data);
        // The identity of the bytes, not merely the fact that they were readable. Everything sealed
        // below is evidence only for the generation that ships exactly this file.
        let source_sha256: [u8; 32] = Sha256::digest(data).into();
        let verified_default_field_types = verified_default_field_types(data);
        let (verified_default_class_paths, verified_default_class_profile_digests) =
            verified_default_class_paths(data);
        let by_name = scan_by_name(data);
        // A partially readable cache may populate only one table; keep any useful table.
        if by_name.is_empty()
            && by_class.is_empty()
            && field_types.is_empty()
            && verified_default_field_types.is_empty()
            && verified_default_class_paths.is_empty()
        {
            return None;
        }
        Some(NativeApi {
            by_class,
            by_name,
            field_types,
            verified_default_binds_sha256: (!verified_default_field_types.is_empty()
                || !verified_default_class_paths.is_empty())
            .then_some(source_sha256),
            verified_default_field_types,
            verified_default_class_paths,
            verified_default_class_profile_digests,
        })
    }

    /// Exact `(class, name)` arity if known and unambiguous.
    pub fn arity(&self, class: &str, name: &str) -> Option<usize> {
        self.by_class
            .get(&(class.to_string(), name.to_string()))
            .copied()
    }

    /// Name-only arity, returned ONLY if every signature with that name shares a single arity
    /// (unambiguous across overloads). `None` if overloaded with differing arities or unknown.
    pub fn arity_by_name(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied().flatten()
    }

    /// True if `name` appears as ANY native function/method signature name in the printable-run
    /// scan — `contains_key`, NOT `arity_by_name`: ambiguous-arity overloads still count as
    /// "exists". Batch-24b shadow gate (a script global sharing a name with any native member
    /// must be `::`-qualified inside classes).
    pub fn has_name(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Plain struct-field VALUE type for an exact `(class, field)` key, if unambiguous.
    /// batch-25a: `("FWidgetAlignment", "VerticalAlignment") -> "EVerticalAlignment"`.
    pub fn field_type(&self, class: &str, field: &str) -> Option<&str> {
        self.field_types
            .get(&(class.to_string(), field.to_string()))
            .map(|s| s.as_str())
    }

    /// Exact native field type usable as mutation evidence. The table exists only when the
    /// source Binds.Cache and extracted field map matched their audited SHA-256 profiles, and an
    /// entry is exposed only for the matching audited script-cache GUID. Generic decompiler field
    /// types remain available through [`Self::field_type`] independently of this mutation gate.
    pub fn verified_default_field_type(
        &self,
        script_cache_guid: &[u8; 16],
        class: &str,
        field: &str,
    ) -> Option<&str> {
        if !is_verified_default_pairing(script_cache_guid, self.verified_default_binds_sha256.as_ref())
        {
            return None;
        }
        self.verified_default_field_types
            .get(&(class.to_string(), field.to_string()))
            .map(String::as_str)
    }

    /// Sealed AngelScript type-to-Unreal path map for native default ancestry. The full map is
    /// available only for the audited script-cache GUID; callers must still join it with the
    /// independently sealed USMAP class graph.
    pub(crate) fn verified_default_class_paths(
        &self,
        script_cache_guid: &[u8; 16],
    ) -> Option<&HashMap<String, String>> {
        (is_verified_default_pairing(script_cache_guid, self.verified_default_binds_sha256.as_ref())
            && !self.verified_default_class_paths.is_empty())
        .then_some(&self.verified_default_class_paths)
    }

    /// Actual `(Binds bytes, canonical class bridge)` digests paired with
    /// [`Self::verified_default_class_paths`]. Unknown/unsealed builds expose neither half.
    pub(crate) fn verified_default_class_profile_digests(
        &self,
        script_cache_guid: &[u8; 16],
    ) -> Option<VerifiedDefaultClassProfileDigests> {
        (is_verified_default_pairing(script_cache_guid, self.verified_default_binds_sha256.as_ref())
            && !self.verified_default_class_paths.is_empty())
        .then_some(self.verified_default_class_profile_digests)
        .flatten()
    }

    /// Number of distinct `(class, field)` plain-field type entries (diagnostic).
    pub fn field_type_count(&self) -> usize {
        self.field_types.len()
    }

    /// Number of distinct names in the by-name table (diagnostic).
    pub fn name_count(&self) -> usize {
        self.by_name.len()
    }

    /// Number of distinct `(class, name)` entries (diagnostic).
    pub fn class_name_count(&self) -> usize {
        self.by_class.len()
    }

    /// Every native callable leaf found by the high-coverage printable-signature scan.
    ///
    /// Kept crate-private because bare callable names deliberately collapse method, free-function,
    /// namespace, and overload domains. The collision-inventory collector uses that conservative
    /// union; normal decompiler callers should continue to use the scoped lookup APIs above.
    #[cfg(test)]
    pub(super) fn collision_callable_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }
}

pub(super) enum CollisionNameVisitError<E> {
    InvalidBinds,
    Visitor(E),
}

/// Visit every native type/callable collision candidate without materializing a candidate table.
///
/// Callers must first bind `data` to a trusted generation seal. The format's variable-width
/// metadata slots do not admit an exact generic record walk, so the seal supplies exact byte
/// identity while this two-pass scan supplies a structural floor and deterministic candidate
/// order. Conflicting Unreal path aliases remain collision names rather than being discarded.
pub(super) fn visit_collision_names<E>(
    data: &[u8],
    mut visitor: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), CollisionNameVisitError<E>> {
    let declared = read_u32(data, 0).ok_or(CollisionNameVisitError::InvalidBinds)? as usize;
    if declared == 0 || declared > 200_000 {
        return Err(CollisionNameVisitError::InvalidBinds);
    }

    let mut strong_headers = 0usize;
    let mut first_header = None;
    for offset in 4..data.len().saturating_sub(8) {
        if strong_record_type_name(data, offset).is_some() {
            first_header.get_or_insert(offset);
            strong_headers = strong_headers.saturating_add(1);
        }
    }
    if first_header != Some(4) || strong_headers < declared {
        return Err(CollisionNameVisitError::InvalidBinds);
    }

    for offset in 4..data.len().saturating_sub(8) {
        if let Some(name) = strong_record_type_name(data, offset) {
            visitor(name).map_err(CollisionNameVisitError::Visitor)?;
        }
    }

    let mut offset = 0usize;
    while offset < data.len() {
        if !(0x20..0x7f).contains(&data[offset]) {
            offset += 1;
            continue;
        }
        let start = offset;
        while offset < data.len() && (0x20..0x7f).contains(&data[offset]) {
            offset += 1;
        }
        if offset - start < 4 {
            continue;
        }
        let run = &data[start..offset];
        if !run.contains(&b'(') || !run.contains(&b')') {
            continue;
        }
        let signature =
            std::str::from_utf8(run).map_err(|_| CollisionNameVisitError::InvalidBinds)?;
        if let (Some(name), Some(_)) = (name_before_paren(signature), arity_of(signature)) {
            visitor(name).map_err(CollisionNameVisitError::Visitor)?;
        }
    }
    Ok(())
}

fn strong_record_type_name(data: &[u8], offset: usize) -> Option<&str> {
    let type_len = read_u32(data, offset)? as usize;
    if !(2..=256).contains(&type_len)
        || !is_cstr(data, offset.checked_add(4)?, type_len)
        || !is_script_path(data, offset.checked_add(4)?.checked_add(type_len)?)
    {
        return None;
    }
    let start = offset.checked_add(4)?;
    let end = start.checked_add(type_len)?.checked_sub(1)?;
    let name = std::str::from_utf8(data.get(start..end)?).ok()?;
    (!name.is_empty()).then_some(name)
}

fn verified_default_field_types(data: &[u8]) -> HashMap<(String, String), String> {
    let source_sha256: [u8; 32] = Sha256::digest(data).into();
    let Some((field_map_sha256, _)) = gore_generation::binds_digests_for_sha256(&source_sha256)
    else {
        return HashMap::new();
    };
    let fields = scan_plain_field_types(data);
    if field_type_map_sha256(&fields) == field_map_sha256 {
        fields
    } else {
        HashMap::new()
    }
}

fn verified_default_class_paths(
    data: &[u8],
) -> (
    HashMap<String, String>,
    Option<VerifiedDefaultClassProfileDigests>,
) {
    let source_sha256: [u8; 32] = Sha256::digest(data).into();
    let Some((_, class_path_map_sha256)) = gore_generation::binds_digests_for_sha256(&source_sha256)
    else {
        return (HashMap::new(), None);
    };
    let paths = scan_type_paths(data);
    let bridge_sha256 = string_map_sha256(&paths);
    if bridge_sha256 == class_path_map_sha256 {
        (paths, Some((source_sha256, bridge_sha256)))
    } else {
        (HashMap::new(), None)
    }
}

/// Everything a `Binds.Cache` says about itself that a generation row seals, derived from any file
/// rather than only from a sealed one.
///
/// [`BindsProfile::class_paths`] is the same map [`NativeApi::verified_default_class_paths`] hands
/// to the ancestry join — but that accessor answers for an audited script-cache GUID only, and
/// this one answers for anything. Both are true statements about the bytes; they differ in what
/// they license. Nothing built from this may mutate a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindsProfile {
    pub field_map_sha256: [u8; 32],
    pub class_path_map_sha256: [u8; 32],
    /// Exact AngelScript type name to qualified Unreal class/struct path.
    pub class_paths: HashMap<String, String>,
    pub field_row_count: usize,
    pub class_path_row_count: usize,
}

/// Derive the two parser-output digests a generation row pins, and the row counts that are the only
/// evidence a parser did not silently stop recognising a record shape.
///
/// This exists so that qualifying a new build reads the digests out of the parser that produces
/// them rather than out of a second implementation written to compute the same hash faster: a
/// reimplementation that is subtly wrong mints a seal that is perfectly self-consistent and
/// describes nothing (`docs/reference/game-updates.md` step 6). It runs the same
/// [`scan_plain_field_types`] and [`scan_type_paths`] passes the sealed accessors run, through the
/// same two digest functions, and differs from them in exactly one way: it does not ask the
/// generation table for permission first.
pub fn derive_binds_profile(data: &[u8]) -> BindsProfile {
    let fields = scan_plain_field_types(data);
    let class_paths = scan_type_paths(data);
    BindsProfile {
        field_map_sha256: field_type_map_sha256(&fields),
        class_path_map_sha256: string_map_sha256(&class_paths),
        field_row_count: fields.len(),
        class_path_row_count: class_paths.len(),
        class_paths,
    }
}

fn string_map_sha256(values: &HashMap<String, String>) -> [u8; 32] {
    let mut rows: Vec<_> = values.iter().collect();
    rows.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let mut hash = Sha256::new();
    for (key, value) in rows {
        for value in [key, value] {
            hash.update((value.len() as u32).to_le_bytes());
            hash.update(value.as_bytes());
        }
    }
    hash.finalize().into()
}

fn field_type_map_sha256(fields: &HashMap<(String, String), String>) -> [u8; 32] {
    let mut rows: Vec<_> = fields.iter().collect();
    rows.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let mut hash = Sha256::new();
    for ((class, field), value_type) in rows {
        for value in [class, field, value_type] {
            hash.update((value.len() as u32).to_le_bytes());
            hash.update(value.as_bytes());
        }
    }
    hash.finalize().into()
}

#[inline]
fn read_u32(data: &[u8], off: usize) -> Option<u32> {
    data.get(off..off.checked_add(4)?)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Is `data[off..off+len]` a NUL-terminated ASCII-printable C string?
fn is_cstr(data: &[u8], off: usize, len: usize) -> bool {
    let Some(end) = off.checked_add(len) else {
        return false;
    };
    if len == 0 || len > 8192 || end > data.len() {
        return false;
    }
    let s = &data[off..end];
    s[len - 1] == 0 && s[..len - 1].iter().all(|&b| (0x20..0x7f).contains(&b))
}

/// Does a length-prefixed string at `lenoff` (u32 len then bytes) start with "/Script/"?
fn is_script_path(data: &[u8], lenoff: usize) -> bool {
    match read_u32(data, lenoff) {
        Some(l) => {
            let l = l as usize;
            let Some(start) = lenoff.checked_add(4) else {
                return false;
            };
            is_cstr(data, start, l)
                && start
                    .checked_add(8)
                    .and_then(|end| data.get(start..end))
                    .is_some_and(|p| p == b"/Script/")
        }
        None => false,
    }
}

/// Is there a plausible field decl (u32 len + printable C string) at `p`, within `[.., end)`?
fn looks_field_decl(data: &[u8], p: usize, end: usize) -> bool {
    match read_u32(data, p) {
        Some(l) => {
            let l = l as usize;
            let Some(start) = p.checked_add(4) else {
                return false;
            };
            is_cstr(data, start, l) && start.checked_add(l).is_some_and(|value| value <= end)
        }
        None => false,
    }
}

/// Number of top-level params inside the outermost `(...)` of a signature decl.
/// Commas inside `<...>` templates or nested `(...)` are not counted.
fn arity_of(decl: &str) -> Option<usize> {
    let bytes = decl.as_bytes();
    let open = decl.find('(')?;
    // Find the matching close paren.
    let mut depth = 0i32;
    let mut close = None;
    for (k, &c) in bytes.iter().enumerate().skip(open) {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(k);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let inner = &decl[open + 1..close];
    if inner.trim().is_empty() {
        return Some(0);
    }
    let mut ad = 0i32; // angle/paren nesting depth
    let mut count = 1usize;
    for &c in inner.as_bytes() {
        match c {
            b'<' | b'(' => ad += 1,
            b'>' | b')' => ad -= 1,
            b',' if ad == 0 => count += 1,
            _ => {}
        }
    }
    Some(count)
}

/// The bare method name = the identifier immediately before the outermost `(`.
fn name_before_paren(decl: &str) -> Option<&str> {
    let open = decl.find('(')?;
    let pre = decl[..open].trim_end();
    let start = pre
        .char_indices()
        .rev()
        .take_while(|&(_, c)| c == '_' || c.is_ascii_alphanumeric())
        .last()
        .map(|(i, _)| i)?;
    let ident = &pre[start..];
    if ident.is_empty() || ident.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(ident)
}

/// Locate record starts by the strong `(typeLen, typeName, pathLen, "/Script/…")` signature.
fn find_record_starts_exhaustive(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let mut starts = Vec::new();
    let mut o = 4usize; // skip the leading u32 count
    while o + 8 < n {
        if let Some(tl) = read_u32(data, o) {
            let tl = tl as usize;
            if tl > 1 && tl <= 256 && is_cstr(data, o + 4, tl) && is_script_path(data, o + 4 + tl) {
                starts.push(o);
            }
        }
        o += 1;
    }
    starts
}

/// Extract exact type/path header pairs from every strong Binds header candidate. Both directions
/// must be one-to-one: conflicting type aliases or path aliases are removed before the sealed map
/// digest is checked.
fn scan_type_paths(data: &[u8]) -> HashMap<String, String> {
    let mut by_type = HashMap::new();
    let mut type_conflicts = std::collections::HashSet::new();
    let mut by_path: HashMap<String, String> = HashMap::new();
    let mut path_conflicts = std::collections::HashSet::new();

    for offset in find_record_starts_exhaustive(data) {
        let Some(type_len) = read_u32(data, offset).map(|value| value as usize) else {
            continue;
        };
        let Some(type_start) = offset.checked_add(4) else {
            continue;
        };
        let Some(type_end) = type_start.checked_add(type_len) else {
            continue;
        };
        let Some(path_len) = read_u32(data, type_end).map(|value| value as usize) else {
            continue;
        };
        let Some(path_start) = type_end.checked_add(4) else {
            continue;
        };
        let Some(path_end) = path_start.checked_add(path_len) else {
            continue;
        };
        let (Some(type_bytes), Some(path_bytes)) = (
            data.get(type_start..type_end.saturating_sub(1)),
            data.get(path_start..path_end.saturating_sub(1)),
        ) else {
            continue;
        };
        let (Ok(type_name), Ok(path)) = (
            std::str::from_utf8(type_bytes),
            std::str::from_utf8(path_bytes),
        ) else {
            continue;
        };
        let type_name = type_name.to_owned();
        let path = path.to_owned();

        if by_type
            .get(&type_name)
            .is_some_and(|previous| previous != &path)
        {
            type_conflicts.insert(type_name.clone());
        } else {
            by_type.insert(type_name.clone(), path.clone());
        }
        if by_path
            .get(&path)
            .is_some_and(|previous| previous != &type_name)
        {
            path_conflicts.insert(path.clone());
        } else {
            by_path.insert(path, type_name);
        }
    }

    by_type.retain(|type_name, path| {
        !type_conflicts.contains(type_name) && !path_conflicts.contains(path)
    });
    by_type
}

/// Best-effort end of one record, used only to keep the method-arity parser from treating
/// embedded type/path metadata as record headers. Plain-field coverage uses the exhaustive
/// scanner because this tolerant walk can skip a later record after a very wide metadata slot.
fn record_end(data: &[u8], offset: usize) -> Option<usize> {
    let n = data.len();
    let type_len = read_u32(data, offset)? as usize;
    let mut cursor = offset.checked_add(4)?.checked_add(type_len)?;
    let path_len = read_u32(data, cursor)? as usize;
    cursor = cursor.checked_add(4)?.checked_add(path_len)?;
    let field_count = read_u32(data, cursor)? as usize;
    if field_count > 200_000 {
        return None;
    }
    cursor = cursor.checked_add(4)?;
    for _ in 0..field_count {
        if !looks_field_decl(data, cursor, n) {
            break;
        }
        let decl_len = read_u32(data, cursor)? as usize;
        let mut after = cursor.checked_add(4)?.checked_add(decl_len)?;
        let name_len = read_u32(data, after)? as usize;
        if !is_cstr(data, after + 4, name_len) {
            break;
        }
        after = after.checked_add(4)?.checked_add(name_len)?;
        let limit = after.saturating_add(512).min(n);
        let mut next = after;
        while next < limit && !looks_field_decl(data, next, n) {
            next += 1;
        }
        if next == limit {
            cursor = after;
            break;
        }
        cursor = next;
    }
    Some(cursor)
}

/// Locate a conservative record sequence for method arities.
fn find_record_starts(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let mut starts = Vec::new();
    let mut offset = 4usize;
    while offset + 8 < n {
        if let Some(type_len) = read_u32(data, offset).map(|value| value as usize) {
            if type_len > 1
                && type_len <= 256
                && is_cstr(data, offset + 4, type_len)
                && is_script_path(data, offset + 4 + type_len)
            {
                starts.push(offset);
                if let Some(end) = record_end(data, offset).filter(|end| *end > offset) {
                    offset = end;
                    continue;
                }
            }
        }
        offset += 1;
    }
    starts
}

/// Tolerant record parse -> (`(class, name) -> arity`, `(class, field) -> plain-field type`),
/// dropping any conflicting keys from either map.
type NativeRecordMaps = (
    HashMap<(String, String), usize>,
    HashMap<(String, String), String>,
);

fn parse_records(data: &[u8]) -> NativeRecordMaps {
    let n = data.len();
    let starts = find_record_starts(data);
    let start_set: std::collections::HashSet<usize> = starts.iter().copied().collect();
    let mut map: HashMap<(String, String), usize> = HashMap::new();
    let mut conflicting: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    let mut ftypes: HashMap<(String, String), String> = HashMap::new();
    let mut ftype_conflicts: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    for (ri, &o) in starts.iter().enumerate() {
        let end = if ri + 1 < starts.len() {
            starts[ri + 1]
        } else {
            n
        };
        let tl = match read_u32(data, o) {
            Some(v) => v as usize,
            None => continue,
        };
        let mut p = o + 4;
        let tn = String::from_utf8_lossy(&data[p..p + tl])
            .trim_end_matches('\0')
            .to_string();
        p += tl;
        let pl = match read_u32(data, p) {
            Some(v) => v as usize,
            None => continue,
        };
        p += 4 + pl;
        let fc = match read_u32(data, p) {
            Some(v) => v as usize,
            None => continue,
        };
        if fc > 200_000 {
            continue;
        }
        p += 4;

        let mut fi = 0usize;
        while fi < fc && p < end {
            if !looks_field_decl(data, p, end) {
                break;
            }
            let dl = read_u32(data, p).unwrap() as usize;
            let decl = String::from_utf8_lossy(&data[p + 4..p + 4 + dl])
                .trim_end_matches('\0')
                .to_string();
            let mut q = p + 4 + dl;
            // bare name
            let nl = match read_u32(data, q) {
                Some(v) => v as usize,
                None => break,
            };
            if !is_cstr(data, q + 4, nl) {
                break;
            }
            let name = String::from_utf8_lossy(&data[q + 4..q + 4 + nl])
                .trim_end_matches('\0')
                .to_string();
            q += 4 + nl;

            if decl.contains('(') && decl.contains(')') {
                if let Some(a) = arity_of(&decl) {
                    let key = (tn.clone(), name.clone());
                    match map.get(&key) {
                        Some(&prev) if prev != a => {
                            conflicting.insert(key);
                        }
                        _ => {
                            map.insert(key, a);
                        }
                    }
                }
            } else {
                // Plain STRUCT-FIELD decl: strictly two whitespace-separated tokens with the
                // second token equal to the bare field name -> the first token is the value
                // type (`"EVerticalAlignment VerticalAlignment"`). Anything wider (const,
                // templates with spaces, property accessors) is skipped — strict two-token
                // keeps the map free of misparses (batch-25a).
                let mut toks = decl.split_whitespace();
                if let (Some(ty), Some(fname), None) = (toks.next(), toks.next(), toks.next()) {
                    if fname == name && !ty.is_empty() {
                        let key = (tn.clone(), name.clone());
                        match ftypes.get(&key) {
                            Some(prev) if prev != ty => {
                                ftype_conflicts.insert(key);
                            }
                            _ => {
                                ftypes.insert(key, ty.to_string());
                            }
                        }
                    }
                }
            }

            let limit = q.saturating_add(512).min(end);
            let mut next = q;
            let mut found = None;
            while next < limit {
                if start_set.contains(&next) {
                    break;
                }
                if looks_field_decl(data, next, end) {
                    found = Some(next);
                    break;
                }
                next += 1;
            }
            match found {
                Some(next) => {
                    p = next;
                    fi += 1;
                }
                None => break,
            }
        }
    }

    for key in conflicting {
        map.remove(&key);
    }
    for key in ftype_conflicts {
        ftypes.remove(&key);
    }
    (map, ftypes)
}

/// High-coverage plain-field pass. Unlike method arities, a row is accepted only when its
/// declaration is exactly `<TYPE> <Name>` and the adjacent bare name agrees, so scanning every
/// strong header candidate cannot turn arbitrary metadata into a field type.
fn scan_plain_field_types(data: &[u8]) -> HashMap<(String, String), String> {
    let starts = find_record_starts_exhaustive(data);
    let mut fields = HashMap::new();
    let mut conflicts = std::collections::HashSet::new();

    for (index, &offset) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(data.len());
        let Some(type_len) = read_u32(data, offset).map(|value| value as usize) else {
            continue;
        };
        let type_start = match offset.checked_add(4) {
            Some(value) => value,
            None => continue,
        };
        let Some(type_bytes) = data.get(type_start..type_start.saturating_add(type_len)) else {
            continue;
        };
        let class = String::from_utf8_lossy(type_bytes)
            .trim_end_matches('\0')
            .to_owned();
        let path_offset = type_start + type_len;
        let Some(path_len) = read_u32(data, path_offset).map(|value| value as usize) else {
            continue;
        };
        let count_offset = match path_offset
            .checked_add(4)
            .and_then(|value| value.checked_add(path_len))
        {
            Some(value) => value,
            None => continue,
        };
        let Some(field_count) = read_u32(data, count_offset).map(|value| value as usize) else {
            continue;
        };
        if field_count > 200_000 {
            continue;
        }
        let mut cursor = count_offset + 4;
        while cursor < end {
            if !looks_field_decl(data, cursor, end) {
                cursor += 1;
                continue;
            }
            let decl_len = read_u32(data, cursor).unwrap() as usize;
            let decl_start = cursor + 4;
            let decl_end = decl_start + decl_len;
            let decl = String::from_utf8_lossy(&data[decl_start..decl_end])
                .trim_end_matches('\0')
                .to_owned();
            let Some(name_len) = read_u32(data, decl_end).map(|value| value as usize) else {
                cursor += 1;
                continue;
            };
            let name_start = decl_end + 4;
            if !is_cstr(data, name_start, name_len) || name_start + name_len > end {
                cursor += 1;
                continue;
            }
            let name = String::from_utf8_lossy(&data[name_start..name_start + name_len])
                .trim_end_matches('\0')
                .to_owned();
            cursor = name_start + name_len;

            if decl.contains('(') || decl.contains(')') {
                continue;
            }
            let mut tokens = decl.split_whitespace();
            let (Some(value_type), Some(field_name), None) =
                (tokens.next(), tokens.next(), tokens.next())
            else {
                continue;
            };
            if field_name != name || value_type.is_empty() {
                continue;
            }
            let key = (class.clone(), name);
            match fields.get(&key) {
                Some(previous) if previous != value_type => {
                    conflicts.insert(key);
                }
                _ => {
                    fields.insert(key, value_type.to_owned());
                }
            }
        }
    }
    for key in conflicts {
        fields.remove(&key);
    }
    fields
}

/// Class-agnostic printable-run scan -> `name -> Some(arity)|None(conflict)`.
fn scan_by_name(data: &[u8]) -> HashMap<String, Option<usize>> {
    // Collect arities seen per name; resolve to Some(unique) or None(conflict) at the end.
    let mut seen: HashMap<String, std::collections::HashSet<usize>> = HashMap::new();

    let n = data.len();
    let mut i = 0usize;
    while i < n {
        // Find start of a printable run.
        if !(0x20..0x7f).contains(&data[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && (0x20..0x7f).contains(&data[i]) {
            i += 1;
        }
        if i - start < 4 {
            continue;
        }
        let run = &data[start..i];
        if !run.contains(&b'(') || !run.contains(&b')') {
            continue;
        }
        // Safe: run is ASCII-printable by construction.
        let s = std::str::from_utf8(run).unwrap_or("");
        if let (Some(name), Some(a)) = (name_before_paren(s), arity_of(s)) {
            seen.entry(name.to_string()).or_default().insert(a);
        }
    }

    seen.into_iter()
        .map(|(name, arities)| {
            let v = if arities.len() == 1 {
                arities.into_iter().next()
            } else {
                None
            };
            (name, v)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The default Steam layout. Only a fallback: it is where the file sits on the machine these
    /// tests were written on, and keeping it means a developer with that layout still needs no
    /// setup.
    const DEFAULT_REAL_BINDS: &str =
        r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Script\Binds.Cache";

    /// `GORE_AS_BINDS` is the override production already reads (`gore/src/cmd/as_cache.rs`,
    /// `gore-ffi/src/lib.rs`), so the tests read the same one rather than inventing a second way
    /// to name the same file. Without it these tests re-audit whatever Steam last wrote to one
    /// hardcoded drive letter, which is how a background game update arrives looking like a
    /// parser failure.
    fn real_binds_path() -> PathBuf {
        std::env::var_os("GORE_AS_BINDS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_BINDS))
    }

    #[test]
    fn arity_of_handles_templates_and_defaults() {
        assert_eq!(arity_of("void Foo()"), Some(0));
        assert_eq!(arity_of("int Bar(int a)"), Some(1));
        assert_eq!(arity_of("int Baz(int a, int b)"), Some(2));
        // template comma must NOT split a param
        assert_eq!(
            arity_of("int F(TArray<TPair<int, float>> a, int b)"),
            Some(2)
        );
        // default values still count
        assert_eq!(
            arity_of("int G(const TSubclassOf<UInterface> T = nullptr)"),
            Some(1)
        );
        assert_eq!(arity_of("bool H(const FHandle& h) const"), Some(1));
    }

    #[test]
    fn name_before_paren_picks_method_name() {
        assert_eq!(name_before_paren("FString GetSelf()"), Some("GetSelf"));
        assert_eq!(
            name_before_paren("bool AllowSelectionModifiers(const FX& h) const"),
            Some("AllowSelectionModifiers")
        );
    }

    #[test]
    fn exhaustive_field_scan_crosses_wide_metadata_gaps() {
        fn push_cstr(data: &mut Vec<u8>, value: &str) {
            let len = u32::try_from(value.len() + 1).unwrap();
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(value.as_bytes());
            data.push(0);
        }

        let mut data = 1u32.to_le_bytes().to_vec();
        push_cstr(&mut data, "UItemDefinition");
        push_cstr(&mut data, "/Script/G1R.ItemDefinition");
        data.extend_from_slice(&2u32.to_le_bytes());
        push_cstr(&mut data, "void First()");
        push_cstr(&mut data, "First");
        data.extend(std::iter::repeat_n(0u8, 700));
        push_cstr(&mut data, "int m_Value");
        push_cstr(&mut data, "m_Value");
        data.extend(std::iter::repeat_n(0u8, 32));

        let fields = scan_plain_field_types(&data);
        assert_eq!(
            fields
                .get(&("UItemDefinition".into(), "m_Value".into()))
                .map(String::as_str),
            Some("int")
        );
        assert!(
            verified_default_field_types(&data).is_empty(),
            "heuristic rows from an unknown Binds identity must never become mutation evidence"
        );
    }

    /// A syntactically valid `Binds.Cache` that no generation seals: two records, one plain field
    /// each, with the wide zero metadata slot the real file carries after a field.
    fn unsealed_binds_fixture() -> Vec<u8> {
        fn push_cstr(data: &mut Vec<u8>, value: &str) {
            let len = u32::try_from(value.len() + 1).unwrap();
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(value.as_bytes());
            data.push(0);
        }

        let mut data = 2u32.to_le_bytes().to_vec();
        for (class, path, field, value_type) in [
            (
                "UItemDefinition",
                "/Script/G1R.ItemDefinition",
                "m_Value",
                "int",
            ),
            (
                "UWeaponDefinition",
                "/Script/G1R.WeaponDefinition",
                "m_CriticalMultiplier",
                "float32",
            ),
        ] {
            push_cstr(&mut data, class);
            push_cstr(&mut data, path);
            data.extend_from_slice(&1u32.to_le_bytes());
            push_cstr(&mut data, &format!("{value_type} {field}"));
            push_cstr(&mut data, field);
            data.extend(std::iter::repeat_n(0u8, 32));
        }
        data
    }

    #[test]
    fn an_unsealed_binds_file_is_fully_described_and_still_admits_nothing() {
        // The line this file has to hold, asserted on one buffer at one moment so that no reading
        // of it can be charitable. `gore as qualify` must be able to read everything a brand new
        // Binds.Cache produces — that is what qualifying one *is* — while every accessor that can
        // reach a mutation goes on answering "unknown" for those same bytes. Widening the second
        // into the first is one `pub` away and the damage would be invisible: an unaudited file
        // would supply field types and a class bridge that agree with themselves perfectly and
        // describe a build nobody looked at.
        let data = unsealed_binds_fixture();
        let source_sha256: [u8; 32] = Sha256::digest(&data).into();
        assert!(
            gore_generation::binds_digests_for_sha256(&source_sha256).is_none(),
            "the fixture has to be a file the table does not seal, or this test proves nothing"
        );

        let profile = derive_binds_profile(&data);
        assert_eq!(profile.field_row_count, 2);
        assert_eq!(profile.class_path_row_count, 2);
        assert_eq!(
            profile.class_paths.get("UItemDefinition").map(String::as_str),
            Some("/Script/G1R.ItemDefinition"),
            "the class bridge a qualification run reads is the whole map, not a sealed subset"
        );
        assert_ne!(profile.field_map_sha256, [0; 32]);
        assert_ne!(profile.class_path_map_sha256, [0; 32]);

        assert!(
            verified_default_field_types(&data).is_empty(),
            "heuristic rows from an unknown Binds identity must never become mutation evidence"
        );
        assert_eq!(
            verified_default_class_paths(&data),
            (HashMap::new(), None),
            "an unsealed class bridge must not reach the ancestry join, digests or map"
        );
        let api = NativeApi::from_bytes(&data).expect("an unsealed file still parses");
        for row in gore_generation::rows() {
            assert_eq!(
                api.verified_default_field_type(&row.script_cache_guid, "UItemDefinition", "m_Value"),
                None,
                "{} admitted a field type from a file it does not seal",
                row.id
            );
            assert!(
                api.verified_default_class_paths(&row.script_cache_guid).is_none(),
                "{} admitted a class bridge from a file it does not seal",
                row.id
            );
            assert!(
                api.verified_default_class_profile_digests(&row.script_cache_guid)
                    .is_none(),
                "{} admitted profile digests from a file it does not seal",
                row.id
            );
        }
        assert_eq!(
            api.field_type("UItemDefinition", "m_Value"),
            Some("int"),
            "the gate must not narrow generic decompiler field evidence either"
        );
    }

    #[test]
    fn mutation_fields_require_the_generation_that_seals_this_binds_file() {
        // A sealed map is evidence for the generation that ships exactly these Binds bytes, and for
        // no other. The gate used to ask only whether the script-cache GUID appeared anywhere in
        // the table, so pointing GORE_AS_BINDS at an archived Binds file — which the loader permits,
        // since the path comes from the environment or from beside the cache — handed one
        // generation's field map to another generation's cache as mutation evidence. The two do
        // differ: their sealed field-map digests are not the same hash.
        let mut foreign_guid = gore_generation::GENERATION_ROWS[0].script_cache_guid;
        foreign_guid[15] ^= 1;

        for row in gore_generation::rows() {
            let api = NativeApi::from_test_field_types(
                &[("UItemDefinition", "m_Value", "int")],
                &[("UItemDefinition", "m_Value", "int")],
                Some(row.binds_cache.sha256),
            );

            assert_eq!(
                api.verified_default_field_type(
                    &row.script_cache_guid,
                    "UItemDefinition",
                    "m_Value",
                ),
                Some("int"),
                "{} must read the map sealed for its own Binds file",
                row.id
            );

            // Every other generation whose Binds file differs must be refused this map.
            for other in gore_generation::rows() {
                if other.binds_cache.sha256 == row.binds_cache.sha256 {
                    continue;
                }
                assert_eq!(
                    api.verified_default_field_type(
                        &other.script_cache_guid,
                        "UItemDefinition",
                        "m_Value",
                    ),
                    None,
                    "{} was handed the field map sealed for {}",
                    other.id,
                    row.id
                );
            }
        }

        let api = NativeApi::from_test_field_types(
            &[("UItemDefinition", "m_Value", "int")],
            &[("UItemDefinition", "m_Value", "int")],
            Some(gore_generation::GENERATION_ROWS[0].binds_cache.sha256),
        );
        assert_eq!(
            api.verified_default_field_type(&foreign_guid, "UItemDefinition", "m_Value"),
            None,
            "an unknown script-cache GUID must expose no mutation evidence"
        );
        assert_eq!(
            api.field_type("UItemDefinition", "m_Value"),
            Some("int"),
            "the GUID gate must not narrow generic decompiler field evidence"
        );
    }

    #[test]
    fn length_helpers_fail_closed_on_overflow() {
        assert!(read_u32(&[], usize::MAX).is_none());
        assert!(!is_cstr(&[], usize::MAX, 8));
        assert!(!looks_field_decl(&[0; 8], usize::MAX, usize::MAX));
    }

    /// Coverage of the real shipped `Binds.Cache`, whichever one `GORE_AS_BINDS` names. Skipped
    /// when no such file is present (so CI without the game install stays green).
    #[test]
    fn the_real_binds_cache_parses_with_the_coverage_the_decompiler_relies_on() {
        // Why this is no longer one test with the digest seal below. Everything asserted here is
        // a statement about the *parser* and holds for any generation of the file; the seal is a
        // statement about *which game build was audited*. They shared one test, with the seal
        // checked first — so when Steam shipped build 24340829 and `Binds.Cache` grew from
        // 5,903,938 to 5,908,587 bytes, the `.expect` on the sealed digests aborted before a
        // single arity was reached. A routine game patch had silently disabled the only real-file
        // coverage check on the code path the decompiler actually uses, which is the opposite of
        // what a seal is for. A seal going stale must never take a live check down with it.
        let path = real_binds_path();
        if !path.exists() {
            eprintln!(
                "skipping: {} not present (set GORE_AS_BINDS)",
                path.display()
            );
            return;
        }
        let api = NativeApi::load(&path).expect("load Binds.Cache");
        let bytes = std::fs::read(&path).expect("read Binds.Cache for from_bytes parity");
        let from_bytes = NativeApi::from_bytes(&bytes).expect("parse identical Binds bytes");
        assert_eq!(api.class_name_count(), from_bytes.class_name_count());
        assert_eq!(api.name_count(), from_bytes.name_count());
        assert_eq!(api.field_type_count(), from_bytes.field_type_count());
        // Both construction routes must agree about the sealed state too — including agreeing
        // that there is none, which is what an unsealed generation looks like from here.
        for row in gore_generation::rows() {
            assert_eq!(
                api.verified_default_class_profile_digests(&row.script_cache_guid),
                from_bytes.verified_default_class_profile_digests(&row.script_cache_guid),
                "the two construction routes disagree about {}",
                row.id
            );
        }

        eprintln!("distinct (class,name) entries : {}", api.class_name_count());
        eprintln!("distinct by-name entries       : {}", api.name_count());
        for nm in [
            "GetCharacterState",
            "GetSelf",
            "GetAI",
            "HasGameplayTag",
            "AssessEvent",
            "GetAvatarActorFromActorInfo",
        ] {
            eprintln!("  arity_by_name({nm:?}) = {:?}", api.arity_by_name(nm));
        }

        // by-name expectations (validated against the Python prototype).
        assert_eq!(api.arity_by_name("GetSelf"), Some(0));
        assert_eq!(api.arity_by_name("AssessEvent"), Some(2));
        assert_eq!(api.arity_by_name("GetAvatarActorFromActorInfo"), Some(0));
        // overloaded with differing arities => None
        assert_eq!(api.arity_by_name("GetCharacterState"), None);
        assert_eq!(api.arity_by_name("GetAI"), None);
        assert_eq!(api.arity_by_name("HasGameplayTag"), None);

        // ~14k distinct names expected.
        assert!(
            api.name_count() > 13_000,
            "by-name coverage too low: {}",
            api.name_count()
        );

        // (class,name) exact lookups from the record parse.
        assert_eq!(
            api.arity("UGameplayAbility_CharacterAI", "AssessEvent"),
            Some(2)
        );
        assert_eq!(
            api.arity("UGameplayAbility", "GetAvatarActorFromActorInfo"),
            Some(0)
        );
    }

    /// The provenance record for `binds_cache.sha256` and `binds_class_path_map_sha256` in the
    /// generation table: the digests those rows pin are the ones the audited shipped bytes
    /// actually produce. Point `GORE_AS_BINDS` at one generation's `Binds.Cache` and run with
    /// `--ignored`.
    #[test]
    #[ignore = "sealed provenance; set GORE_AS_BINDS to the audited generation's Binds.Cache"]
    fn the_sealed_class_profile_digests_match_the_audited_binds_generation() {
        // Why this is ignored rather than run by default: a seal names one game build, and Steam
        // replaces `Binds.Cache` whenever it likes. When it does, these digests are absent by
        // design, not wrong — the honest reading of a failure here is "the game installed on this
        // machine is not a generation this build supports", which `gore story-catalog` and
        // `load_default_mutation_evidence` already say, on the real command, with a far better
        // message than a panicking unit test. Re-sealing is a deliberate multi-file audit (a new
        // script-cache GUID, ancestry fingerprints, operand counts, USMAP graph seals), never a
        // test fix, so this stays runnable on demand and stays out of the default suite.
        let path = real_binds_path();
        if !path.exists() {
            eprintln!(
                "skipping: {} not present (set GORE_AS_BINDS)",
                path.display()
            );
            return;
        }
        let api = NativeApi::load(&path).expect("load Binds.Cache");
        let bytes = std::fs::read(&path).expect("read the configured Binds.Cache");
        let file_sha256: [u8; 32] = Sha256::digest(&bytes).into();
        let mut audited = false;
        for row in gore_generation::rows_for_binds_sha256(&file_sha256) {
            let (source_sha256, bridge_sha256) = api
                .verified_default_class_profile_digests(&row.script_cache_guid)
                .expect("sealed Binds class-profile digests");
            assert_eq!(source_sha256, row.binds_cache.sha256, "{}", row.id);
            assert_eq!(bridge_sha256, row.binds_class_path_map_sha256, "{}", row.id);
            audited = true;
        }
        assert!(
            audited,
            "the configured Binds.Cache belongs to no audited generation, so it seals nothing"
        );
    }

    /// batch-25a: the in-crate `refs::native_field_type` table must MATCH the shipped
    /// Binds.Cache field decls — this is the verification path for the production source
    /// (the production emit runs without Binds, so the hardcoded table is load-bearing).
    /// Skipped when the game install is absent.
    #[test]
    fn validate_field_types_against_real_binds_cache() {
        let path = real_binds_path();
        if !path.exists() {
            eprintln!(
                "skipping: {} not present (set GORE_AS_BINDS)",
                path.display()
            );
            return;
        }
        let api = NativeApi::load(&path).expect("load Binds.Cache");
        eprintln!(
            "distinct (class,field) type entries: {}",
            api.field_type_count()
        );
        // mirror of refs.rs KNOWN_NATIVE_FIELD_TYPES (keep in sync)
        for (cls, field, want) in [
            (
                "FWidgetAlignment",
                "VerticalAlignment",
                "EVerticalAlignment",
            ),
            (
                "FWidgetAlignment",
                "HorizontalAlignment",
                "EHorizontalAlignment",
            ),
            ("FPerceivedAgent", "Relationship", "ERelationship"),
            ("FPerceivedAgent", "Hostility", "ERelationshipHostility"),
            (
                "FPerceivedAgent",
                "RelativeRank",
                "ERelationshipRelativeRank",
            ),
            (
                "FFXPerceptionSoundArea",
                "PerceptionLoudness",
                "EPerceptionNoiseLoudness",
            ),
            (
                "FALoadingScreenSettings",
                "Layout",
                "EAsyncLoadingScreenLayout",
            ),
            (
                "FALoadingScreenSettings",
                "PlaybackType",
                "EMoviePlaybackType",
            ),
            ("FTextAppearance", "Justification", "ETextJustify"),
            (
                "FInteractionAnimTransition",
                "TransitionKind",
                "EInteractionInputKind",
            ),
            ("FWeatherSaveGame", "CurrentWeather", "EWeather"),
            (
                "FCrimeVictimPersonHandle",
                "RelationshipTowardsPerson",
                "ERelationship",
            ),
            (
                "FCrimeVictimPersonHandle",
                "RelativeRankTowardsPerson",
                "ERelationshipRelativeRank",
            ),
            (
                "FCrimeVictimGuildHandle",
                "RelationshipTowardsGuild",
                "ERelationship",
            ),
            (
                "FCrimeVictimGuildHandle",
                "RelativeRankTowardsGuild",
                "ERelationshipRelativeRank",
            ),
            (
                "FLetterboxLayoutSettings",
                "VerticalLoadingWidgetPosition",
                "EVerticalAlignment",
            ),
            (
                "FLetterboxLayoutSettings",
                "VerticalTipWidgetPosition",
                "EVerticalAlignment",
            ),
        ] {
            assert_eq!(
                api.field_type(cls, field),
                Some(want),
                "table entry ({cls}, {field}) disagrees with the shipped Binds.Cache"
            );
        }
    }

    /// The offline provenance for `AUDITED_ITEM_FIELD_MANIFEST_SHA256`
    /// (`gore-ffi/src/authoring_item_patch_revision3.rs` names this test as its witness): the
    /// scalar types baked into the embedded item catalog are the ones the audited shipped
    /// `Binds.Cache` declares. Point `GORE_AS_BINDS` at that generation's `Binds.Cache` and run
    /// with `--ignored`.
    #[test]
    #[ignore = "sealed provenance; set GORE_AS_BINDS to the audited generation's Binds.Cache"]
    fn validates_item_authoring_field_types_against_real_binds_cache() {
        // Same reason the class-profile digests above are ignored, and the same evidence: every
        // row here goes through `verified_default_field_type`, which is gated on the sealed file
        // identity, so on any other game build all 20 fields × 2 GUIDs lose their evidence at
        // once and none of it says anything about the parser — the unsealed field-type tests
        // either side of this one go on proving those rows are still readable out of the new
        // bytes. The seal must stay sealed (it is what makes the embedded manifest evidence
        // rather than a guess) and must stay runnable, so it moves out of the default suite
        // instead of being re-pointed at whatever Steam installed most recently.
        let path = real_binds_path();
        if !path.exists() {
            eprintln!(
                "skipping: {} not present (set GORE_AS_BINDS)",
                path.display()
            );
            return;
        }
        let api = NativeApi::load(&path).expect("load Binds.Cache");
        let expected = [
            ("UItemDefinition", "m_Value", "int"),
            ("UItemDefinition", "m_MaxStack", "int"),
            ("UItemDefinition", "m_Weight", "float32"),
            ("UItemDefinition", "m_Mass", "float32"),
            ("UItemDefinition", "m_Buoyancy", "float32"),
            ("UItemDefinition", "m_AutoTarget", "bool"),
            ("UProjectileDefinition", "m_ArcParam", "float32"),
            ("UProjectileDefinition", "m_Radius", "float32"),
            (
                "UWeaponArcheryDefinition",
                "m_ArrowGravityModifier",
                "float32",
            ),
            (
                "UWeaponRangedDefinition",
                "m_ArrowGravityModifier",
                "float32",
            ),
            ("UWeaponArcheryDefinition", "m_MaxRange", "float32"),
            (
                "UWeaponMeleeDefinition",
                "m_BlockSuperArmorMultiplier",
                "float32",
            ),
            ("UWeaponMeleeDefinition", "m_DamageReduction", "float32"),
            ("UWeaponMeleeDefinition", "m_HpRegenerateTick", "float32"),
            ("UWeaponMeleeDefinition", "m_StartRegenerateSc", "float32"),
            ("UWeaponDefinition", "m_CriticalMultiplier", "float32"),
            ("UWeaponDefinition", "m_SuperArmorDamageBase", "float32"),
            ("URuneSpellContainer", "m_CanEquipAfterUse", "bool"),
            (
                "URuneSpellContainer",
                "m_IsTargetingIndicatorEnabled",
                "bool",
            ),
            ("URuneSpellContainer", "RequiredMagicCircleLevel", "int"),
        ];
        for row in gore_generation::rows() {
            for (owner, field, value_type) in expected {
                assert_eq!(
                    api.verified_default_field_type(&row.script_cache_guid, owner, field),
                    Some(value_type),
                    "sealed item field {owner}.{field} for generation {}",
                    row.id
                );
            }
        }
        let mut foreign_guid = gore_generation::GENERATION_ROWS[0].script_cache_guid;
        foreign_guid[0] ^= 1;
        assert_eq!(
            api.verified_default_field_type(&foreign_guid, "UItemDefinition", "m_Value"),
            None,
            "sealed Binds evidence must not cross script-cache builds"
        );
    }

    /// batch-40b: the in-crate `refs` `KNOWN_NATIVE_FLOAT_FIELDS` table must MATCH the shipped
    /// Binds.Cache field decls. Same verification path as
    /// `validate_field_types_against_real_binds_cache` above (the production emit runs without
    /// Binds, so the hardcoded float-field table is load-bearing). This proves the baked-in
    /// float types are authoritative rather than guessed. Skipped when the game install is absent.
    #[test]
    fn validate_float_field_types_against_real_binds_cache() {
        let path = real_binds_path();
        if !path.exists() {
            eprintln!(
                "skipping: {} not present (set GORE_AS_BINDS)",
                path.display()
            );
            return;
        }
        let api = NativeApi::load(&path).expect("load Binds.Cache");
        // mirror of refs.rs KNOWN_NATIVE_FLOAT_FIELDS (keep in sync)
        for (cls, field, want) in [
            (
                "FALoadingScreenSettings",
                "MinimumLoadingScreenDisplayTime",
                "float32",
            ),
            ("FAlphaBlendArgs", "BlendTime", "float32"),
            ("FCameraBehaviour", "m_ArmLength", "float32"),
            ("FCameraBehaviour", "m_LagSpeed", "float32"),
            ("FCameraBehaviour", "m_SpellPitchLimit", "float32"),
            ("FCameraBehaviour", "m_SpellYawLimit", "float32"),
            ("FDodgeData", "m_SuperArmorResistanceMultiplier", "float32"),
            ("FFreezeParams", "m_BlendOutDuration", "float32"),
            ("FFreezeParams", "m_CustomTimeDilation", "float32"),
            ("FFreezeParams", "m_FreezeDuration", "float32"),
            ("FGameplayCueParameters", "NormalizedMagnitude", "float32"),
            ("FGameplayCueParameters", "RawMagnitude", "float32"),
            (
                "FGameplayEffectContext_HitResponse",
                "BowStretch",
                "float32",
            ),
            (
                "FGameplayEffectContext_HitResponse",
                "MultiplierSuperArmor",
                "float32",
            ),
            (
                "FGothicFlyDiveSettings",
                "AdaptToCollisionSampleZDistance",
                "float32",
            ),
            (
                "FGothicFlyDiveSettings",
                "CharacterZDivergeOffset",
                "float32",
            ),
            (
                "FGothicFlyDiveSettings",
                "GroundedMoveBeforeGoalDistance",
                "float32",
            ),
            ("FGothicFlyDiveSettings", "UseFlyDiveMinDistance", "float32"),
            (
                "FGothicPathfollowSettings",
                "AgentRadiusMultiplier",
                "float32",
            ),
            (
                "FGothicPathfollowSettings",
                "CrowdAgentRadiusMultiplier",
                "float32",
            ),
            (
                "FGothicPathfollowSettings",
                "CrowdAgentSeparationWeight",
                "float32",
            ),
            (
                "FInteractionAnimTransition",
                "BlockOtherTransitionsForSeconds",
                "float32",
            ),
            ("FInteractionAnimTransition", "CooldownSeconds", "float32"),
            ("FInteractionAnimTransition", "Probability", "float32"),
            ("FInteractionAnimTransition", "Weight", "float32"),
            ("FLightSet", "BarnDoorAngle", "float32"),
            ("FLightSet", "BarnDoorLength", "float32"),
            ("FLightSet", "IndirectLightingIntensity", "float32"),
            ("FLightSet", "VolumetricScatteringIntensity", "float32"),
            ("FLightValues", "AttenuationRadius", "float32"),
            ("FLightValues", "SourceHeight", "float32"),
            ("FLightValues", "SourceWidth", "float32"),
            ("FMemorizedEvent", "Magnitude", "float32"),
            (
                "FPathfollowModifyAvoidVelocitySettings",
                "FastSpeedVelocityMultiplier",
                "float32",
            ),
            (
                "FPathfollowModifyAvoidVelocitySettings",
                "MediumRangeVelocityMultiplier",
                "float32",
            ),
            (
                "FPathfollowModifyAvoidVelocitySettings",
                "ShortRangeVelocityMultiplier",
                "float32",
            ),
            (
                "FPathfollowMoveFocusSettings",
                "FocalPointHeightMultiplier",
                "float32",
            ),
            ("FPerceptionHandler", "DelaySeconds", "float32"),
            ("FRelativeCrimeDataEntry", "BaseSeverity", "float32"),
            ("FRememberedPerception", "Magnitude", "float32"),
            ("FRememberedPerception", "TimeUpdated", "float32"),
            ("FScalableFloat", "Value", "float32"),
            ("FScoredItemAction", "Score", "float32"),
            ("FSlateFontInfo", "Size", "float32"),
            ("FTipSettings", "TipSwapTime", "float32"),
            ("FTipSettings", "TipWrapAt", "float32"),
        ] {
            assert_eq!(
                api.field_type(cls, field),
                Some(want),
                "float-field table entry ({cls}, {field}) disagrees with the shipped Binds.Cache"
            );
        }
    }
}
