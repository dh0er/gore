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
//! width. Because the slot width is not uniform, this parser does NOT trust a fixed stride:
//! it re-syncs after every field by scanning forward to the next plausible field decl or the
//! next record start. This keeps the (class, name) extraction robust without panicking.
//!
//! Two independent passes populate the tables:
//! 1. A tolerant **record parse** locates record boundaries via the strong
//!    `(typeNameLen, typeName, pathLen, "/Script/…")` signature, then walks fields to build the
//!    `(class, name) -> arity` map.
//! 2. A class-agnostic **printable-run scan** collects every `[\x20-\x7e]{4,}` run containing
//!    `(` and `)` to build the `name -> arity` map. This is the high-coverage backstop (the
//!    record parse stays in sync for most but not every record).
//!
//! A *signature* is any decl string containing both `(` and `)`. Arity = number of top-level
//! comma-separated parameters inside the OUTERMOST parens, ignoring commas nested in `<…>`
//! template brackets or inner `(…)`. Empty parens => 0. Defaulted params (`= nullptr`) count.

use std::collections::HashMap;
use std::path::Path;

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
}

impl NativeApi {
    /// Parse `Binds.Cache`. Returns `None` on any IO/parse failure (caller treats absence as
    /// "no data").
    pub fn load(path: &Path) -> Option<NativeApi> {
        let data = std::fs::read(path).ok()?;
        if data.len() < 8 {
            return None;
        }
        let (by_class, field_types) = parse_records(&data);
        let by_name = scan_by_name(&data);
        // A partially readable cache may populate only one table; keep it if either has data.
        if by_name.is_empty() && by_class.is_empty() {
            return None;
        }
        Some(NativeApi { by_class, by_name, field_types })
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

#[inline]
fn read_u32(data: &[u8], off: usize) -> Option<u32> {
    data.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Is `data[off..off+len]` a NUL-terminated ASCII-printable C string?
fn is_cstr(data: &[u8], off: usize, len: usize) -> bool {
    if len == 0 || len > 8192 || off + len > data.len() {
        return false;
    }
    let s = &data[off..off + len];
    s[len - 1] == 0 && s[..len - 1].iter().all(|&b| (0x20..0x7f).contains(&b))
}

/// Does a length-prefixed string at `lenoff` (u32 len then bytes) start with "/Script/"?
fn is_script_path(data: &[u8], lenoff: usize) -> bool {
    match read_u32(data, lenoff) {
        Some(l) => {
            let l = l as usize;
            is_cstr(data, lenoff + 4, l)
                && data
                    .get(lenoff + 4..lenoff + 4 + 8)
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
            is_cstr(data, p + 4, l) && p + 4 + l <= end
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

/// Offset just past the whole record at `o` (header + its `fc` fields). Walking the fields
/// (rather than only skipping the type/path header) means the scanner resumes AFTER the
/// record instead of inside its field region, where a field's bytes can coincidentally match
/// the record-start signature and create a spurious boundary that truncates the next record's
/// field walk. Returns `None` if the header/field-count looks implausible.
fn record_end(data: &[u8], o: usize) -> Option<usize> {
    let n = data.len();
    let tl = read_u32(data, o)? as usize;
    let mut p = o + 4 + tl; // past type name
    let pl = read_u32(data, p)? as usize;
    p += 4 + pl; // past script path
    let fc = read_u32(data, p)? as usize;
    if fc > 200_000 {
        return None;
    }
    p += 4; // past fieldCount
    let mut fi = 0usize;
    while fi < fc {
        if !looks_field_decl(data, p, n) {
            break;
        }
        let dl = read_u32(data, p)? as usize;
        let mut q = p + 4 + dl; // past decl string
        let nl = read_u32(data, q)? as usize;
        if !is_cstr(data, q + 4, nl) {
            break;
        }
        q += 4 + nl; // past bare name
        // Re-sync over the variable-width metadata slot to the next field decl. For the last
        // field this lands on the next record's type name (also a u32-len + cstr), i.e. the
        // record end.
        let limit = (q + 512).min(n);
        let mut nxt = q;
        let mut found = None;
        while nxt < limit {
            if looks_field_decl(data, nxt, n) {
                found = Some(nxt);
                break;
            }
            nxt += 1;
        }
        match found {
            Some(f) => {
                p = f;
                fi += 1;
            }
            None => {
                p = q;
                break;
            }
        }
    }
    Some(p)
}

/// Locate record starts by the strong `(typeLen, typeName, pathLen, "/Script/…")` signature.
fn find_record_starts(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let mut starts = Vec::new();
    let mut o = 4usize; // skip the leading u32 count
    while o + 8 < n {
        if let Some(tl) = read_u32(data, o) {
            let tl = tl as usize;
            if tl > 1 && tl <= 256 && is_cstr(data, o + 4, tl) && is_script_path(data, o + 4 + tl) {
                starts.push(o);
                // Advance past the WHOLE record so an in-record byte run can't be mistaken
                // for the next record start; fall back to the header skip if the walk fails.
                match record_end(data, o) {
                    Some(e) if e > o => o = e,
                    _ => o = o + 4 + tl + 4 + read_u32(data, o + 4 + tl).unwrap_or(0) as usize,
                }
                continue;
            }
        }
        o += 1;
    }
    starts
}

/// Tolerant record parse -> (`(class, name) -> arity`, `(class, field) -> plain-field type`),
/// dropping any conflicting keys from either map.
fn parse_records(
    data: &[u8],
) -> (HashMap<(String, String), usize>, HashMap<(String, String), String>) {
    let n = data.len();
    let starts = find_record_starts(data);
    let start_set: std::collections::HashSet<usize> = starts.iter().copied().collect();

    let mut map: HashMap<(String, String), usize> = HashMap::new();
    let mut conflicting: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    // plain two-token field decls `"<TYPE> <Name>"` (no parens): value type per (class, field).
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
        let tn = String::from_utf8_lossy(&data[p..p + tl]).trim_end_matches('\0').to_string();
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

            // Re-sync over the variable-width metadata slot: scan forward to the next
            // field decl, bailing if we cross into the next record.
            let limit = (q + 512).min(end);
            let mut nxt = q;
            let mut found = None;
            while nxt < limit {
                if start_set.contains(&nxt) {
                    break;
                }
                if looks_field_decl(data, nxt, end) {
                    found = Some(nxt);
                    break;
                }
                nxt += 1;
            }
            match found {
                Some(f) => {
                    p = f;
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
        assert_eq!(api.arity("UGameplayAbility_CharacterAI", "AssessEvent"), Some(2));
        assert_eq!(api.arity("UGameplayAbility", "GetAvatarActorFromActorInfo"), Some(0));
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
        eprintln!("distinct (class,field) type entries: {}", api.field_type_count());
        // mirror of refs.rs KNOWN_NATIVE_FIELD_TYPES (keep in sync)
        for (cls, field, want) in [
            ("FWidgetAlignment", "VerticalAlignment", "EVerticalAlignment"),
            ("FWidgetAlignment", "HorizontalAlignment", "EHorizontalAlignment"),
            ("FPerceivedAgent", "Relationship", "ERelationship"),
            ("FPerceivedAgent", "Hostility", "ERelationshipHostility"),
            ("FPerceivedAgent", "RelativeRank", "ERelationshipRelativeRank"),
            ("FFXPerceptionSoundArea", "PerceptionLoudness", "EPerceptionNoiseLoudness"),
            ("FALoadingScreenSettings", "Layout", "EAsyncLoadingScreenLayout"),
            ("FALoadingScreenSettings", "PlaybackType", "EMoviePlaybackType"),
            ("FTextAppearance", "Justification", "ETextJustify"),
            ("FInteractionAnimTransition", "TransitionKind", "EInteractionInputKind"),
            ("FWeatherSaveGame", "CurrentWeather", "EWeather"),
        ] {
            assert_eq!(
                api.field_type(cls, field),
                Some(want),
                "table entry ({cls}, {field}) disagrees with the shipped Binds.Cache"
            );
        }
    }
}
