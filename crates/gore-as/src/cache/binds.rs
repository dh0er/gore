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

/// This exact shipped Binds.Cache was audited together with the scalar default-site profile.
/// Exhaustive native field rows are never accepted from an unknown build.
const VERIFIED_DEFAULT_FIELD_BINDS_SHA256: [u8; 32] = [
    0x46, 0xe6, 0x62, 0x9a, 0xd5, 0xca, 0xcc, 0x11, 0x2b, 0x99, 0x22, 0xd4, 0x8a, 0x1a, 0xa9, 0x48,
    0xf4, 0x05, 0x72, 0xd7, 0x28, 0x57, 0x05, 0xb9, 0x81, 0xc3, 0xec, 0xa3, 0xdc, 0x61, 0x5f, 0xea,
];
/// Deterministic digest of the audited `(owner, field, value type)` mapping extracted from the
/// sealed Binds.Cache above. A parser change cannot silently alter mutation evidence.
const VERIFIED_DEFAULT_FIELD_MAP_SHA256: [u8; 32] = [
    0x5d, 0xdf, 0x7f, 0xa6, 0xdf, 0x36, 0xac, 0x00, 0xd0, 0x7b, 0xd0, 0x68, 0xfc, 0xf1, 0x9a, 0xd6,
    0x1a, 0x3f, 0x4b, 0x83, 0x61, 0x33, 0x51, 0x39, 0x66, 0xdc, 0x37, 0x9b, 0x24, 0x24, 0x17, 0x07,
];
/// Deterministic digest of every unambiguous `(AngelScript type, /Script/ path)` bridge found in
/// the sealed Binds file. Native-default ancestry uses this only together with a sealed USMAP.
const VERIFIED_DEFAULT_CLASS_PATH_MAP_SHA256: [u8; 32] = [
    0xcf, 0xfb, 0xce, 0x6f, 0xeb, 0x2f, 0x8c, 0x14, 0xdc, 0x5f, 0x25, 0x19, 0x37, 0x41, 0xf5, 0x89,
    0x51, 0xc1, 0x6f, 0x27, 0x0a, 0x76, 0x77, 0x31, 0x25, 0xd0, 0xe5, 0x07, 0xd3, 0x6e, 0x95, 0xc4,
];
/// Per-build GUID from the matching audited `PrecompiledScript_Shipping.Cache` header.
/// A sealed Binds file is not mutation evidence for any other script-cache build.
const VERIFIED_DEFAULT_SCRIPT_CACHE_GUID: [u8; 16] = [
    0x45, 0x0d, 0x65, 0xc0, 0x4f, 0x0c, 0x01, 0x4f, 0xbe, 0xc5, 0x68, 0x01, 0x63, 0x78, 0xe6, 0x9a,
];

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
        }
    }

    #[cfg(test)]
    pub(crate) fn from_test_field_types(
        generic: &[(&str, &str, &str)],
        verified: &[(&str, &str, &str)],
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
        }
    }

    /// Parse `Binds.Cache`. Returns `None` on any IO/parse failure (caller treats absence as
    /// "no data").
    pub fn load(path: &Path) -> Option<NativeApi> {
        let data = std::fs::read(path).ok()?;
        if data.len() < 8 {
            return None;
        }
        let (by_class, field_types) = parse_records(&data);
        let verified_default_field_types = verified_default_field_types(&data);
        let verified_default_class_paths = verified_default_class_paths(&data);
        let by_name = scan_by_name(&data);
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
            verified_default_field_types,
            verified_default_class_paths,
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
        if script_cache_guid != &VERIFIED_DEFAULT_SCRIPT_CACHE_GUID {
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
        (script_cache_guid == &VERIFIED_DEFAULT_SCRIPT_CACHE_GUID
            && !self.verified_default_class_paths.is_empty())
        .then_some(&self.verified_default_class_paths)
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
}

fn verified_default_field_types(data: &[u8]) -> HashMap<(String, String), String> {
    let source_sha256: [u8; 32] = Sha256::digest(data).into();
    if source_sha256 == VERIFIED_DEFAULT_FIELD_BINDS_SHA256 {
        let fields = scan_plain_field_types(data);
        if field_type_map_sha256(&fields) == VERIFIED_DEFAULT_FIELD_MAP_SHA256 {
            fields
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    }
}

fn verified_default_class_paths(data: &[u8]) -> HashMap<String, String> {
    let source_sha256: [u8; 32] = Sha256::digest(data).into();
    if source_sha256 != VERIFIED_DEFAULT_FIELD_BINDS_SHA256 {
        return HashMap::new();
    }
    let paths = scan_type_paths(data);
    if string_map_sha256(&paths) == VERIFIED_DEFAULT_CLASS_PATH_MAP_SHA256 {
        paths
    } else {
        HashMap::new()
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
    use super::*;

    const REAL_BINDS: &str =
        r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Script\Binds.Cache";

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

    #[test]
    fn mutation_fields_require_audited_script_guid_but_generic_fields_do_not() {
        let api = NativeApi::from_test_field_types(
            &[("UItemDefinition", "m_Value", "int")],
            &[("UItemDefinition", "m_Value", "int")],
        );
        let mut foreign_guid = VERIFIED_DEFAULT_SCRIPT_CACHE_GUID;
        foreign_guid[15] ^= 1;

        assert_eq!(
            api.verified_default_field_type(
                &VERIFIED_DEFAULT_SCRIPT_CACHE_GUID,
                "UItemDefinition",
                "m_Value",
            ),
            Some("int")
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

    /// Validation against the real shipped `Binds.Cache`. Skipped when the file is absent
    /// (so CI without the game install stays green).
    #[test]
    fn validate_against_real_binds_cache() {
        let path = Path::new(REAL_BINDS);
        if !path.exists() {
            eprintln!("skipping: {REAL_BINDS} not present");
            return;
        }
        let api = NativeApi::load(path).expect("load Binds.Cache");

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

    /// batch-25a: the in-crate `refs::native_field_type` table must MATCH the shipped
    /// Binds.Cache field decls — this is the verification path for the production source
    /// (the production emit runs without Binds, so the hardcoded table is load-bearing).
    /// Skipped when the game install is absent.
    #[test]
    fn validate_field_types_against_real_binds_cache() {
        let path = Path::new(REAL_BINDS);
        if !path.exists() {
            eprintln!("skipping: {REAL_BINDS} not present");
            return;
        }
        let api = NativeApi::load(path).expect("load Binds.Cache");
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

    #[test]
    fn validates_item_definition_fields_against_real_binds_cache() {
        let path = Path::new(REAL_BINDS);
        if !path.exists() {
            eprintln!("skipping: {REAL_BINDS} not present");
            return;
        }
        let api = NativeApi::load(path).expect("load Binds.Cache");
        assert_eq!(
            api.verified_default_field_type(
                &VERIFIED_DEFAULT_SCRIPT_CACHE_GUID,
                "UItemDefinition",
                "m_Value",
            ),
            Some("int")
        );
        assert_eq!(
            api.verified_default_field_type(
                &VERIFIED_DEFAULT_SCRIPT_CACHE_GUID,
                "UItemDefinition",
                "m_MaxStack",
            ),
            Some("int")
        );
        let mut foreign_guid = VERIFIED_DEFAULT_SCRIPT_CACHE_GUID;
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
        let path = Path::new(REAL_BINDS);
        if !path.exists() {
            eprintln!("skipping: {REAL_BINDS} not present");
            return;
        }
        let api = NativeApi::load(path).expect("load Binds.Cache");
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
