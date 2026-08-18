//! REF-REMAPPING: rewrite a regen-extracted module's bytecode ref operands from REGEN
//! keys/ids to the equivalent VANILLA (base) keys/ids, by SYMBOL IDENTITY (name + module +
//! namespace + signature), so the module can be spliced into the vanilla base WITHOUT
//! appending any tail-table rows (the module now references vanilla's existing rows).
//!
//! Why this is needed (`work/reversing/gore-as/specs/ref-remap.md`): the 7 global tail
//! tables are keyed by RUNTIME POINTERS / engine ids captured at serialization. A full-tree
//! regen assigns DIFFERENT keys than vanilla for the SAME symbols. `replace_module` merges
//! the mini's tables into the base on key-collision; with non-colliding regen keys EVERY row
//! would be appended (cache grows ~22 MB, duplicate type registration, boot crash). The fix:
//! rewrite the module's bytecode operands to vanilla keys, then ship EMPTY tail tables so the
//! merge adds nothing.
//!
//! Operand classification is the authoritative table from `findings/decompile-refs.md §3`
//! (verbatim from the engine `FAngelscriptBytecodeReferencer` Store/Load switch). See
//! `OP_REFS` below. Remap is SIZE-PRESERVING (i64 key->i64 key, i32 id->i32 id) so operand
//! dwords are patched in place; no resize.

use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use super::disasm::disassemble;
use super::header::CacheHeader;
use super::types::DATA_TYPE_SIZE;
use super::walk_modules::module_region_end;
use super::wire::{Cursor, WireError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemapError {
    #[error("wire error: {0}")]
    Wire(#[from] WireError),
    #[error("disasm error: {0}")]
    Disasm(String),
    #[error("mini-cache must contain exactly 1 module, found {0}")]
    NotSingle(u32),
    #[error(
        "unresolved {kind} ref in op {op} (regen key {key:#x}, name {name:?}): \
         no matching symbol in the base cache. This module introduces a NEW symbol the base \
         lacks — opt in with RemapOptions::allow_new_symbols to retain its minimal row."
    )]
    Unresolved {
        kind: &'static str,
        op: &'static str,
        key: i64,
        name: String,
    },
    #[error(
        "ambiguous {kind} ref in op {op} (name {name:?}): matches {n} distinct base keys — \
         identity is not unique enough to remap safely."
    )]
    Ambiguous {
        kind: &'static str,
        op: &'static str,
        name: String,
        n: usize,
    },
    #[error(
        "POST-CONDITION FAILED: {n} regen tail-table key(s) SURVIVED in the remapped module's \
         bytes (a regen ptr-key that resolves to null in vanilla → boot crash). These live in a \
         module-record field the remap does not yet cover. First {shown}: {detail}"
    )]
    SurvivingRegenKeys {
        n: usize,
        shown: usize,
        detail: String,
    },
    #[error("regen tail tables end at {got:#x}, but the cache is {len:#x} bytes long")]
    TailNotAtEof { got: usize, len: usize },
    #[error("new-symbol remap could not find the required {kind} row for {key:#x}")]
    MissingNewRow { kind: &'static str, key: i64 },
    #[error("new-symbol remap exhausted the collision-free {kind} key space")]
    KeySpaceExhausted { kind: &'static str },
    #[error(
        "new property {name:?} would collide with an unrelated base PropertyReferences row at {key:#x}"
    )]
    PropertyCollision { name: String, key: i64 },
    #[error("StaticNames index {0} is referenced by the module but absent from the regen cache")]
    MissingStaticName(i64),
    #[error("StaticNames index {0} does not fit the bytecode operand that references it")]
    StaticNameIndexOverflow(i64),
    #[error(
        "mini-cache contains an unresolved {kind} reference in {op} ({key:#x}); the key/id is absent from both the target base and the mini's retained tail tables"
    )]
    UnresolvedEffectiveReference {
        kind: &'static str,
        op: &'static str,
        key: i64,
    },
    #[error(
        "mini-cache tail table {table} row {row_key:#x} has an unresolved {kind} dependency {dependency:#x}; the dependency is absent from both the target base and the mini's retained tail tables"
    )]
    UnresolvedTailDependency {
        table: usize,
        row_key: i64,
        kind: &'static str,
        dependency: i64,
    },
    #[error(
        "mini-cache tail table {table} row {row_key:#x} violates the {kind} invariant ({detail})"
    )]
    InvalidTailRow {
        table: usize,
        row_key: i64,
        kind: &'static str,
        detail: String,
    },
    #[error("module {module:?} violates the {field} cache invariant ({detail})")]
    InvalidModuleStructure {
        module: String,
        field: &'static str,
        detail: String,
    },
    #[error(
        "cache-wide function Id collision {id:#x}: module {first_module:?} and module {second_module:?}"
    )]
    FunctionIdCollision {
        id: i32,
        first_module: String,
        second_module: String,
    },
    #[error("inner module identity {name:?} is registered more than once")]
    ModuleNameCollision { name: String },
    #[error("loadout Script-ID plan was built for a different pristine base cache")]
    LoadoutPlanBaseMismatch,
    #[error("loadout Script-ID plan did not inspect this mini-cache")]
    LoadoutPlanMiniNotInspected,
    #[error("loadout Script-ID plan's bound novel-identity set does not match this mini-cache")]
    LoadoutPlanIdentityMismatch,
    #[error("loadout Script-ID plan has no {kind} assignment for identity {identity:?}")]
    LoadoutPlanMissingAssignment {
        kind: &'static str,
        identity: String,
    },
    #[error("loadout Script-ID plan exceeds {resource}: {actual} > {limit}")]
    LoadoutPlanResourceLimit {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("loadout Script-ID {artifact} header is invalid: {detail}")]
    LoadoutPlanInvalidHeader {
        artifact: &'static str,
        detail: String,
    },
    #[error("loadout Script-ID mini GUID {mini:?} does not match pristine GUID {pristine:?}")]
    LoadoutPlanGuidMismatch { pristine: [u8; 16], mini: [u8; 16] },
    #[error(
        "portable type identity {identity:?} has conflicting T2 object kinds {first:#x} and {second:#x}"
    )]
    LoadoutPlanTypeKindConflict {
        identity: String,
        first: u32,
        second: u32,
    },
}

/// Opt-in behavior for [`remap_module_to_base_with_options`]. The default is intentionally
/// strict and byte-for-byte identical to the historical remapper: every referenced symbol must
/// already exist in `base`, and the emitted mini has seven empty tail tables.
#[derive(Debug, Clone, Copy, Default)]
pub struct RemapOptions {
    /// Carry minimal tail-table rows for symbols genuinely absent from `base`. Existing symbols
    /// still map to base rows by identity; every new pointer/id is synthesized from portable
    /// identity so independently remapped minis compose without first-free allocator collisions.
    pub allow_new_symbols: bool,
}

/// Legacy separator used only by the compact bytediff unit-test identity fixtures. Production
/// identities are length-framed because serialized names may contain any delimiter byte.
const SEP: char = '\u{1f}';

/// Field separator INSIDE a namespace field's `::`-qualified segments (regen drops leading
/// `Ns::` segments — see [`ns_drift_ok`]).
const NS_SEP: &str = "::";

/// Shipped caches currently nest T1 template dependencies at most three rows deep. Keep a
/// generous semantic ceiling so an externally supplied acyclic chain cannot overflow the stack.
const MAX_TYPE_IDENTITY_DEPTH: usize = 64;

/// Recursive portable identities are derived from cache rows not yet semantically admitted. The
/// largest identity is below 1.3 KiB; these generous limits prevent DAG/string amplification
/// without constraining real symbols.
const MAX_IDENTITY_BYTES: usize = 64 * 1024;
const MAX_IDENTITY_NAMESPACES: usize = 4096;
const MIN_IDENTITY_BUDGET: usize = 64 * 1024;
// The current 124 MiB Shipping cache's framed T1/T3/T5 footprint is above 128 MiB once owned-map
// and diagnostic strings are counted. Keep a hard ceiling, but leave broad headroom above that
// real generation while the 4x-input rule remains the tighter bound for normal/minimal caches.
const MAX_IDENTITY_BUDGET: usize = 256 * 1024 * 1024;

/// Namespace-tolerant identity equality is deliberately pairwise and non-transitive. Bound the
/// worst-case bytes each comparison may inspect to a small multiple of the effective plus incoming
/// identity footprint, with an absolute ceiling, so one huge same-skeleton bucket cannot force
/// quadratic preflight time even when its strings differ only near the end.
const IDENTITY_COMPARISON_WORK_MULTIPLIER: usize = 4;
const MIN_IDENTITY_COMPARISON_WORK: usize = 64 * 1024;
const MAX_IDENTITY_COMPARISON_WORK: usize = 256 * 1024 * 1024;

// Declaration authority is queried only for keyed T1/T5/T7 rows. Keep the transient query set at
// the same production envelope as sequential keyed-row composition, while allowing the compact
// pristine declaration index broad headroom for the complete Shipping cache.
// Final composed validation scans T1+T5+T7. The measured Shipping fixture already has 184,315
// such rows, so this must use the complete-cache envelope rather than the much smaller one-mini
// contribution envelope.
const MAX_DECLARATION_QUERY_ROWS: usize = 1_000_000;
const MAX_DECLARATION_QUERY_BYTES: usize = 64 * 1024 * 1024;
const MAX_DECLARATION_AUTHORITY_ROWS: usize = 1_000_000;
// Function declarations retain full runtime-effective DataType identities plus compact lookup
// indexes. The real Shipping declaration inventory exceeds 128 MiB under that conservative heap
// accounting, so share the established complete-cache 256-MiB identity envelope.
const MAX_DECLARATION_AUTHORITY_BYTES: usize = 256 * 1024 * 1024;
const MAX_TAIL_PREFLIGHT_ROWS: usize = 1_000_000;
const MAX_TAIL_PREFLIGHT_BYTES: usize = 256 * 1024 * 1024;
const MAX_TAIL_PREFLIGHT_STATIC_NAMES: usize = 1_000_000;
const MAX_TAIL_PREFLIGHT_STATIC_NAME_BYTES: usize = 128 * 1024 * 1024;
// Windows-1252 input bytes may expand to three UTF-8 bytes when decoded into Rust Strings. Bound
// that projected owned payload before SymTables/TailMetadata perform their first allocation-heavy
// pass; the raw serialized-byte cap alone is not enough to prevent several copies from nearing a
// gigabyte on worst-case high-bit FString payloads.
const MAX_TAIL_PREFLIGHT_DECODED_STRING_BYTES: usize = 256 * 1024 * 1024;

// Before any detailed module walker materializes CodeSpan/embed/record vectors, a streaming pass
// bounds both the number of function-like records and every variable-width record array it sees.
// The caller supplies the stricter per-function and aggregate ByteCode limits.
const MAX_MINI_STREAMED_FUNCTIONS: usize = 131_072;
// ByteCode dwords themselves are charged as work, but splice owns the stable 16M aggregate
// ByteCode error. Leave another 16M for record/metadata arrays so the bytecode-specific limit
// wins at its exact boundary while total materialized work remains firmly bounded.
const MAX_MINI_MODULE_WORK_ITEMS: usize = 32 * 1024 * 1024;
const MAX_CACHE_MODULE_WORK_ITEMS: usize = 50_000_000;

#[derive(Debug)]
struct IdentityBudget {
    remaining: usize,
    max: usize,
    charged: usize,
}

impl IdentityBudget {
    fn for_composed_input(
        total_input_len: usize,
        already_charged: usize,
    ) -> Result<Self, WireError> {
        let max = total_input_len
            .checked_mul(4)
            .unwrap_or(usize::MAX)
            .clamp(MIN_IDENTITY_BUDGET, MAX_IDENTITY_BUDGET);
        let remaining = max
            .checked_sub(already_charged)
            .ok_or(WireError::IdentityBudgetExceeded { max })?;
        Ok(Self {
            remaining,
            max,
            charged: 0,
        })
    }

    fn charge(&mut self, key: i64, identity: &Ident) -> Result<(), WireError> {
        let logical = identity_footprint(key, identity)?;
        self.remaining = self
            .remaining
            .checked_sub(logical)
            .ok_or(WireError::IdentityBudgetExceeded { max: self.max })?;
        self.charged = self
            .charged
            .checked_add(logical)
            .ok_or(WireError::IdentityBudgetExceeded { max: self.max })?;
        Ok(())
    }
}

fn identity_footprint(key: i64, identity: &Ident) -> Result<usize, WireError> {
    let logical = std::mem::size_of::<Ident>()
        .checked_add(identity.full.len())
        .and_then(|n| n.checked_add(identity.ns_stripped.len()))
        .and_then(|n| n.checked_add(identity.display.len()))
        .and_then(|n| {
            identity.namespaces.iter().try_fold(n, |sum, namespace| {
                sum.checked_add(std::mem::size_of::<String>())?
                    .checked_add(namespace.len())
            })
        })
        .and_then(|n| n.checked_add(identity.function_owner.as_ref().map_or(0, String::len)))
        .and_then(|n| n.checked_add(identity.function_name.as_ref().map_or(0, String::len)))
        .ok_or(WireError::IdentityTooLarge {
            key,
            max: MAX_IDENTITY_BYTES,
        })?;
    if logical > MAX_IDENTITY_BYTES || identity.namespaces.len() > MAX_IDENTITY_NAMESPACES {
        return Err(WireError::IdentityTooLarge {
            key,
            max: MAX_IDENTITY_BYTES,
        });
    }
    Ok(logical)
}

fn checked_identity_append(key: i64, target: &mut String, value: &str) -> Result<(), WireError> {
    if target
        .len()
        .checked_add(value.len())
        .is_none_or(|len| len > MAX_IDENTITY_BYTES)
    {
        return Err(WireError::IdentityTooLarge {
            key,
            max: MAX_IDENTITY_BYTES,
        });
    }
    target.push_str(value);
    Ok(())
}

/// Length-prefix every field. Cache strings may contain any byte accepted by FString, including
/// our display separator, so delimiter-only concatenation is not an injective symbol identity.
struct IdentityEncoder {
    key: i64,
    out: String,
}

impl IdentityEncoder {
    fn new(key: i64) -> Self {
        Self {
            key,
            out: String::new(),
        }
    }

    fn field(&mut self, value: &str) -> Result<(), WireError> {
        let len = value.len().to_string();
        checked_identity_append(self.key, &mut self.out, &len)?;
        checked_identity_append(self.key, &mut self.out, ":")?;
        checked_identity_append(self.key, &mut self.out, value)
    }

    fn number(&mut self, value: impl std::fmt::Display) -> Result<(), WireError> {
        self.field(&value.to_string())
    }

    fn finish(self) -> String {
        self.out
    }
}

/// Incremental namespace collector. Re-summing the accumulated vector on every append turns one
/// wide template/function signature into quadratic work; this tracks the exact owned footprint
/// once and inspects only newly appended namespace fields.
struct NamespaceAccumulator {
    values: Vec<String>,
    bytes: usize,
}

impl NamespaceAccumulator {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            bytes: 0,
        }
    }

    fn push(&mut self, key: i64, value: &str) -> Result<(), WireError> {
        let count = self
            .values
            .len()
            .checked_add(1)
            .ok_or(WireError::IdentityTooLarge {
                key,
                max: MAX_IDENTITY_BYTES,
            })?;
        let bytes = self
            .bytes
            .checked_add(std::mem::size_of::<String>())
            .and_then(|bytes| bytes.checked_add(value.len()));
        if count > MAX_IDENTITY_NAMESPACES || bytes.is_none_or(|bytes| bytes > MAX_IDENTITY_BYTES) {
            return Err(WireError::IdentityTooLarge {
                key,
                max: MAX_IDENTITY_BYTES,
            });
        }
        self.bytes = bytes.expect("checked above");
        self.values.push(value.to_owned());
        Ok(())
    }

    fn extend(&mut self, key: i64, value: &[String]) -> Result<(), WireError> {
        let count =
            self.values
                .len()
                .checked_add(value.len())
                .ok_or(WireError::IdentityTooLarge {
                    key,
                    max: MAX_IDENTITY_BYTES,
                })?;
        let added = value.iter().try_fold(0usize, |sum, namespace| {
            sum.checked_add(std::mem::size_of::<String>())?
                .checked_add(namespace.len())
        });
        let bytes = added.and_then(|added| self.bytes.checked_add(added));
        if count > MAX_IDENTITY_NAMESPACES || bytes.is_none_or(|bytes| bytes > MAX_IDENTITY_BYTES) {
            return Err(WireError::IdentityTooLarge {
                key,
                max: MAX_IDENTITY_BYTES,
            });
        }
        self.bytes = bytes.expect("checked above");
        self.values.extend(value.iter().cloned());
        Ok(())
    }

    fn finish(self) -> Vec<String> {
        self.values
    }
}

fn checked_display(key: i64, parts: &[&str]) -> Result<String, WireError> {
    let mut display = String::new();
    for part in parts {
        checked_identity_append(key, &mut display, part)?;
    }
    Ok(display)
}

/// A symbol identity in three parallel forms. `full` is the display/exact-match string
/// (namespaces embedded); `ns_stripped` is the same string with every namespace field replaced
/// by empty (the structural skeleton — module/name/subtypes/signature only); `namespaces` lists
/// the namespace-field values in traversal order. GAP-A (batch-38): our emitter never writes
/// `namespace X { }` blocks, so a vanilla symbol carries a namespace where the regen has none (or
/// a `::`-suffix of it); the binding is unchanged (module+name+subtypes+signature pin the symbol).
/// Two identities match (see [`Ident::oracle_eq`]) when their skeletons are equal AND every
/// namespace-field pair is a benign drift (equal / one-empty / one a `::`-suffix of the other),
/// which collapses the ~26.8k drift diffs while KEEPING the ~39 real `Foo::Bar` vs `Baz::Bar`
/// namespace-collisions SEMANTIC (both-nonempty-non-suffix → not a match).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ident {
    full: String,
    ns_stripped: String,
    namespaces: Vec<String>,
    display: String,
    /// Present only for T3 identities. Avoid reparsing the recursively framed owner identity to
    /// recover the stable owner/method key used by the bytediff scope normalizer.
    function_owner: Option<String>,
    function_name: Option<String>,
}

impl Ident {
    /// Oracle equality with benign namespace-drift tolerance (GAP-A). Exact-string-equal always
    /// matches (self-identity, and the common no-drift case). Otherwise require identical
    /// structural skeletons AND every namespace-field pair to be a benign drift.
    fn oracle_eq(&self, other: &Ident) -> bool {
        if self.full == other.full {
            return true;
        }
        if self.ns_stripped != other.ns_stripped || self.namespaces.len() != other.namespaces.len()
        {
            return false;
        }
        self.namespaces
            .iter()
            .zip(&other.namespaces)
            .all(|(a, b)| ns_drift_ok(a, b))
    }
}

/// True if two namespace-field values differ only by the benign drift our emitter introduces:
/// they are equal, one is empty (the pure-global drop), or one is a `::`-suffix of the other
/// (the enclosing `namespace G1R { }` block dropped, e.g. `G1R::UStoryG1R` vs `UStoryG1R`).
/// A `Foo::Bar` vs `Baz::Bar` pair (both non-empty, neither a `::`-suffix of the other) is a
/// REAL namespace-collision and returns false → stays SEMANTIC (the 39-collision guard, spec §1.1).
fn ns_drift_ok(a: &str, b: &str) -> bool {
    if a == b || a.is_empty() || b.is_empty() {
        return true;
    }
    is_ns_suffix(a, b) || is_ns_suffix(b, a)
}

/// True if `short` equals `long` with ≥1 leading `Seg::` namespace segment removed (i.e. `short`
/// is a proper `::`-delimited suffix of `long`). `"UStoryG1R"` is a suffix of `"G1R::UStoryG1R"`;
/// `"Bar"` is NOT a suffix of `"BazBar"` (segment-boundary required, not a raw substring).
fn is_ns_suffix(long: &str, short: &str) -> bool {
    long.len() > short.len()
        && long.ends_with(short)
        && long[..long.len() - short.len()].ends_with(NS_SEP)
}

/// Symbol identity → key inverse maps for one cache's tail tables, plus the forward key→name
/// maps needed to compose function/global identities and to report unresolved refs.
#[derive(Clone, Debug)]
struct SymTables {
    /// T1: type ptr key -> identity ; identity -> ptr key.
    type_id_of_ptr: HashMap<i64, String>,
    type_ptr_of_id: HashMap<String, Vec<i64>>,
    /// T1: type ptr key -> the oracle [`Ident`] (full + ns-stripped skeleton + namespace list).
    /// PARALLEL to `type_id_of_ptr` (whose full string the remapper's key→key splice keeps).
    type_ident_of_ptr: HashMap<i64, Ident>,
    /// T2: type-id (i32, raw operand) -> type ptr.
    typeid_to_ptr: HashMap<i32, i64>,
    /// Alias-aware inverse of T2: (type ptr, object-kind bits) -> canonical raw type-id.
    ptr_kind_to_typeid: HashMap<(i64, u32), i32>,
    /// T3: func ptr key -> identity ; identity -> ptr key.
    func_id_of_ptr: HashMap<i64, String>,
    func_ptr_of_id: HashMap<String, Vec<i64>>,
    /// T3: func ptr key -> the oracle [`Ident`] (parallel to `func_id_of_ptr`).
    func_ident_of_ptr: HashMap<i64, Ident>,
    /// forward Name (for error messages) per func ptr.
    func_name_of_ptr: HashMap<i64, String>,
    type_name_of_ptr: HashMap<i64, String>,
    global_name_of_ptr: HashMap<i64, String>,
    /// T4: func-id (i32, raw operand) -> func ptr.
    funcid_to_ptr: HashMap<i32, i64>,
    ptr_to_funcid: HashMap<i64, i32>,
    /// T5: global ptr key -> identity ; identity -> ptr key.
    global_id_of_ptr: HashMap<i64, String>,
    global_ptr_of_id: HashMap<String, Vec<i64>>,
    /// T5: global ptr key -> the oracle [`Ident`] (parallel to `global_id_of_ptr`).
    global_ident_of_ptr: HashMap<i64, Ident>,
    /// T5 string literals are runtime-created values, so equal literal identities may legally
    /// appear under many raw keys. Non-string globals remain unique declarations.
    global_is_string_of_ptr: HashMap<i64, bool>,
    /// EVERY int64 ptr-key that appears as a key in this cache's tail tables: T1 type ptrs,
    /// T3 func ptrs, T5 global ptrs, and the ptr values in T2/T4 (id->ptr). Used by the
    /// post-condition scan to assert no regen ptr-key survives in a remapped module's bytes.
    /// (T7 PropertyReferences keys are DERIVED `(tid<<1)|(off<<33)|1` — not raw ptrs — and are
    /// never an operand/embedded field, so they are excluded here; the type-id remap handles them.)
    all_ptr_keys: HashSet<i64>,
    /// Logical heap footprint charged while constructing portable T1/T3/T5 identities.
    identity_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TailPreflightSummary {
    tail: usize,
}

fn charge_projected_tail_string(
    projected: &mut usize,
    raw_len: usize,
    pos: usize,
) -> Result<(), WireError> {
    let decoded = raw_len.checked_mul(3).ok_or(WireError::BadLen {
        pos,
        len: i64::MAX,
        field: "tail projected decoded string bytes",
    })?;
    *projected = projected.checked_add(decoded).ok_or(WireError::BadLen {
        pos,
        len: i64::MAX,
        field: "tail projected decoded string bytes",
    })?;
    if *projected > MAX_TAIL_PREFLIGHT_DECODED_STRING_BYTES {
        return Err(WireError::BadLen {
            pos,
            len: (*projected).min(i64::MAX as usize) as i64,
            field: "tail projected decoded string bytes",
        });
    }
    Ok(())
}

fn preflight_keyed_tail_table(
    c: &mut Cursor<'_>,
    field: &'static str,
    key_bytes: usize,
    keyed_rows: &mut usize,
    keyed_bytes: &mut usize,
    projected_string_bytes: &mut usize,
    read_value: fn(&mut Cursor<'_>, &mut usize) -> Result<(), WireError>,
) -> Result<(), WireError> {
    let count_pos = c.pos();
    let count = c.read_count(field)?;
    *keyed_rows = keyed_rows.checked_add(count).ok_or(WireError::BadLen {
        pos: count_pos,
        len: i64::MAX,
        field: "tail keyed rows",
    })?;
    if *keyed_rows > MAX_TAIL_PREFLIGHT_ROWS {
        return Err(WireError::BadLen {
            pos: count_pos,
            len: *keyed_rows as i64,
            field: "tail keyed rows",
        });
    }
    let entries_start = c.pos();
    for _ in 0..count {
        c.skip(key_bytes)?;
        read_value(c, projected_string_bytes)?;
    }
    *keyed_bytes = keyed_bytes
        .checked_add(c.pos().saturating_sub(entries_start))
        .ok_or(WireError::BadLen {
            pos: entries_start,
            len: i64::MAX,
            field: "tail keyed bytes",
        })?;
    if *keyed_bytes > MAX_TAIL_PREFLIGHT_BYTES {
        return Err(WireError::BadLen {
            pos: entries_start,
            len: *keyed_bytes as i64,
            field: "tail keyed bytes",
        });
    }
    Ok(())
}

pub(super) fn preflight_tail_tables(bytes: &[u8]) -> Result<TailPreflightSummary, WireError> {
    let tail = module_region_end(bytes)?;
    let mut c = Cursor::at(bytes, tail);
    let mut keyed_rows = 0usize;
    let mut keyed_bytes = 0usize;
    let mut projected_string_bytes = 0usize;
    preflight_keyed_tail_table(
        &mut c,
        "TypeReferences",
        8,
        &mut keyed_rows,
        &mut keyed_bytes,
        &mut projected_string_bytes,
        |c, projected| {
            for _ in 0..3 {
                let pos = c.pos();
                let raw = c.read_sia_bytes()?;
                charge_projected_tail_string(projected, raw.len(), pos)?;
            }
            c.skip_tarray_fixed(DATA_TYPE_SIZE, "TypeRef.SubTypes")
        },
    )?;
    preflight_keyed_tail_table(
        &mut c,
        "TypeIdReferenceToPointer",
        4,
        &mut keyed_rows,
        &mut keyed_bytes,
        &mut projected_string_bytes,
        |c, _| c.skip(8),
    )?;
    preflight_keyed_tail_table(
        &mut c,
        "FunctionReferences",
        8,
        &mut keyed_rows,
        &mut keyed_bytes,
        &mut projected_string_bytes,
        |c, projected| {
            for _ in 0..3 {
                let pos = c.pos();
                let raw = c.read_sia_bytes()?;
                charge_projected_tail_string(projected, raw.len(), pos)?;
            }
            c.skip(20)?;
            c.skip_tarray_fixed(DATA_TYPE_SIZE, "FuncRef.ParameterTypes")?;
            c.skip(DATA_TYPE_SIZE)
        },
    )?;
    preflight_keyed_tail_table(
        &mut c,
        "FunctionIdReferenceToPointer",
        4,
        &mut keyed_rows,
        &mut keyed_bytes,
        &mut projected_string_bytes,
        |c, _| c.skip(8),
    )?;
    preflight_keyed_tail_table(
        &mut c,
        "GlobalReferences",
        8,
        &mut keyed_rows,
        &mut keyed_bytes,
        &mut projected_string_bytes,
        |c, projected| {
            let name_pos = c.pos();
            let name = c.read_sia_bytes()?;
            charge_projected_tail_string(projected, name.len(), name_pos)?;
            for _ in 0..2 {
                let pos = c.pos();
                let raw = c.read_sia_bytes()?;
                charge_projected_tail_string(projected, raw.len(), pos)?;
            }
            if c.read_bool4()? {
                name.decode_utf8(name_pos)?;
            }
            Ok(())
        },
    )?;

    let static_pos = c.pos();
    let static_count = c.read_count("StaticNames")?;
    if static_count > MAX_TAIL_PREFLIGHT_STATIC_NAMES {
        return Err(WireError::BadLen {
            pos: static_pos,
            len: static_count as i64,
            field: "StaticNames count",
        });
    }
    let mut static_bytes = 0usize;
    for _ in 0..static_count {
        let name_pos = c.pos();
        let raw = c.read_sia_bytes()?;
        charge_projected_tail_string(&mut projected_string_bytes, raw.len(), name_pos)?;
        static_bytes = static_bytes
            .checked_add(raw.len())
            .ok_or(WireError::BadLen {
                pos: static_pos,
                len: i64::MAX,
                field: "StaticNames bytes",
            })?;
        if static_bytes > MAX_TAIL_PREFLIGHT_STATIC_NAME_BYTES {
            return Err(WireError::BadLen {
                pos: static_pos,
                len: static_bytes as i64,
                field: "StaticNames bytes",
            });
        }
    }
    preflight_keyed_tail_table(
        &mut c,
        "PropertyReferences",
        8,
        &mut keyed_rows,
        &mut keyed_bytes,
        &mut projected_string_bytes,
        |c, projected| {
            let pos = c.pos();
            let raw = c.read_sia_bytes()?;
            charge_projected_tail_string(projected, raw.len(), pos)?;
            c.skip(4)
        },
    )?;
    if c.pos() != bytes.len() {
        return Err(WireError::BadLen {
            pos: c.pos(),
            len: bytes.len().saturating_sub(c.pos()) as i64,
            field: "tail trailing bytes",
        });
    }
    Ok(TailPreflightSummary { tail })
}

#[derive(Clone, Debug)]
struct RawTypeIdentityDataType {
    flags: [bool; 6],
    type_info: i64,
    token: i32,
}

#[derive(Clone, Debug)]
struct RawTypeIdentityRow {
    key: i64,
    name: String,
    module: String,
    namespace: String,
    subs: Vec<RawTypeIdentityDataType>,
}

fn datatype_identity(
    key: i64,
    flags: [bool; 6],
    token: i32,
    type_info: i64,
    nested: &Ident,
) -> Result<Ident, WireError> {
    let [is_ref, is_object_const, is_object_handle, is_const_handle, is_auto, _if_handle_const] =
        flags;
    let (tag, effective_flags, nested_identity) = if is_auto {
        if token != 5 || type_info != 0 {
            return Err(WireError::InvalidDataType {
                key,
                detail: "auto requires token 5 and a null TypeInfo",
            });
        }
        (
            "auto",
            [
                is_ref,
                is_object_const,
                is_object_handle,
                is_object_handle && is_const_handle,
            ],
            None,
        )
    } else if token == 5 {
        if type_info == 0 || nested.full.is_empty() {
            return Err(WireError::InvalidDataType {
                key,
                detail: "identifier requires a concrete TypeReference",
            });
        }
        (
            "object",
            [
                is_ref,
                is_object_const,
                is_object_handle,
                is_object_handle && is_const_handle,
            ],
            Some(nested),
        )
    } else if matches!(
        token,
        0x3b | 0x41 | 0x44 | 0x45 | 0x46 | 0x47 | 0x4b | 0x4c | 0x4d | 0x4e | 0x50 | 0x51 | 0x52
    ) && type_info == 0
    {
        ("primitive", [is_ref, is_object_const, false, false], None)
    } else {
        return Err(WireError::InvalidDataType {
            key,
            detail: "unsupported token/pointer shape",
        });
    };

    let mut full = IdentityEncoder::new(key);
    full.field(tag)?;
    full.number(token)?;
    for flag in effective_flags {
        full.number(flag as u8)?;
    }
    if let Some(nested) = nested_identity {
        full.field(&nested.full)?;
    }
    let mut stripped = IdentityEncoder::new(key);
    stripped.field(tag)?;
    stripped.number(token)?;
    for flag in effective_flags {
        stripped.number(flag as u8)?;
    }
    if let Some(nested) = nested_identity {
        stripped.field(&nested.ns_stripped)?;
    }
    Ok(Ident {
        full: full.finish(),
        ns_stripped: stripped.finish(),
        namespaces: nested_identity
            .map(|identity| identity.namespaces.clone())
            .unwrap_or_default(),
        display: checked_display(key, &[tag, "(", &token.to_string(), ") ", &nested.display])?,
        function_owner: None,
        function_name: None,
    })
}

/// Resolve a complete, build-portable T1 identity. Nested template arguments are themselves
/// full DataTypes; flattening them to an immediate bare name conflates distinct nested types.
fn resolve_type_identity(
    key: i64,
    rows: &[RawTypeIdentityRow],
    row_by_key: &HashMap<i64, usize>,
    fallback: Option<&SymTables>,
    memo: &mut HashMap<i64, Ident>,
    visiting: &mut HashSet<i64>,
    depth: usize,
) -> Result<Ident, WireError> {
    if let Some(identity) = memo.get(&key) {
        return Ok(identity.clone());
    }
    let Some(&index) = row_by_key.get(&key) else {
        return Ok(fallback
            .and_then(|prior| prior.type_ident_of_ptr.get(&key))
            .cloned()
            .unwrap_or_default());
    };
    if depth > MAX_TYPE_IDENTITY_DEPTH {
        return Err(WireError::TypeReferenceDepth {
            key,
            max: MAX_TYPE_IDENTITY_DEPTH,
        });
    }
    if !visiting.insert(key) {
        // Runtime keys are generation-local, so they cannot safely anchor a portable cycle
        // identity. Shipped caches are acyclic; reject an adversarial cycle instead.
        return Err(WireError::CyclicTypeReference { key });
    }

    let row = &rows[index];
    let mut subs_full = String::new();
    let mut subs_stripped = String::new();
    let mut namespaces = NamespaceAccumulator::new();
    namespaces.push(key, &row.namespace)?;
    for subtype in &row.subs {
        let nested = if subtype.token == 5 {
            resolve_type_identity(
                subtype.type_info,
                rows,
                row_by_key,
                fallback,
                memo,
                visiting,
                depth + 1,
            )?
        } else {
            Ident::default()
        };
        let datatype = datatype_identity(
            key,
            subtype.flags,
            subtype.token,
            subtype.type_info,
            &nested,
        )?;
        let mut framed = IdentityEncoder::new(key);
        framed.field(&datatype.full)?;
        checked_identity_append(key, &mut subs_full, &framed.finish())?;
        let mut framed = IdentityEncoder::new(key);
        framed.field(&datatype.ns_stripped)?;
        checked_identity_append(key, &mut subs_stripped, &framed.finish())?;
        namespaces.extend(key, &datatype.namespaces)?;
    }
    visiting.remove(&key);

    let sentinel_template_subtype = row.module == "$__T__";
    if sentinel_template_subtype && (row.namespace.is_empty() || !row.subs.is_empty()) {
        return Err(WireError::InvalidTypeReference {
            key,
            detail: "$__T__ rows require a namespace and no SubTypes",
        });
    }
    // Runtime template-instance lookup ignores the declaring module. The `$__T__` subtype
    // sentinel and ordinary non-template script types use separate branches where it matters.
    let effective_module = if !row.subs.is_empty() && !sentinel_template_subtype {
        ""
    } else {
        &row.module
    };
    let mut full = IdentityEncoder::new(key);
    full.field(effective_module)?;
    full.field(&row.namespace)?;
    full.field(&row.name)?;
    full.number(row.subs.len())?;
    full.field(&subs_full)?;
    let mut ns_stripped = IdentityEncoder::new(key);
    ns_stripped.field(effective_module)?;
    ns_stripped.field("")?;
    ns_stripped.field(&row.name)?;
    ns_stripped.number(row.subs.len())?;
    ns_stripped.field(&subs_stripped)?;
    let identity = Ident {
        full: full.finish(),
        ns_stripped: ns_stripped.finish(),
        namespaces: namespaces.finish(),
        display: checked_display(key, &[&row.module, "::", &row.name])?,
        function_owner: None,
        function_name: None,
    };
    memo.insert(key, identity.clone());
    Ok(identity)
}

/// Read a DataType's stable identity contribution: token + (for object/value types) the
/// resolved TYPE IDENTITY of its `type_info` ptr (the build-specific ptr resolved to a portable
/// identity that includes the type's name + template subtypes — so `TSubclassOf<AFoo>` and
/// `TSubclassOf<ABar>` are distinguished, which matters for conversion-operator overloads).
///
/// Returns the oracle [`Ident`] triple: the nested type's namespace fields (which drift, GAP-A)
/// are carried through into both the stripped skeleton and the namespace list, so a func/global
/// identity that embeds this DataType composes correctly for [`Ident::oracle_eq`].
fn datatype_identity_with_fallback(
    key: i64,
    c: &mut Cursor,
    type_ident_of_ptr: &HashMap<i64, Ident>,
    fallback: Option<&HashMap<i64, Ident>>,
) -> Result<Ident, WireError> {
    // 6 bools, i64 type_info, i32 token (mirror DataType::read order).
    let b0 = c.read_bool4()?;
    let b1 = c.read_bool4()?;
    let b2 = c.read_bool4()?;
    let b3 = c.read_bool4()?;
    let b4 = c.read_bool4()?;
    let b5 = c.read_bool4()?;
    let type_info = c.read_i64()?;
    let token = c.read_i32()?;
    let tident = if token == 5 {
        type_ident_of_ptr
            .get(&type_info)
            .or_else(|| fallback.and_then(|prior| prior.get(&type_info)))
            .cloned()
            .unwrap_or_default()
    } else {
        Ident::default()
    };
    datatype_identity(key, [b0, b1, b2, b3, b4, b5], token, type_info, &tident)
}

fn function_declaration_identity(
    key: i64,
    branch: &'static str,
    module: &str,
    namespace: &str,
    owner: Option<&Ident>,
    name: &str,
    is_const: bool,
    params: &[Ident],
    ret: &Ident,
) -> Result<FunctionDeclarationIdentity, WireError> {
    let owner = owner.cloned().unwrap_or_default();
    let mut params_full = String::new();
    let mut params_stripped = String::new();
    let mut param_namespaces = NamespaceAccumulator::new();
    for param in params {
        checked_identity_append(key, &mut params_full, &param.full)?;
        checked_identity_append(key, &mut params_full, ",")?;
        checked_identity_append(key, &mut params_stripped, &param.ns_stripped)?;
        checked_identity_append(key, &mut params_stripped, ",")?;
        param_namespaces.extend(key, &param.namespaces)?;
    }
    let effective_namespace = if branch == "global" { namespace } else { "" };
    let effective_module = if branch == "method" { "" } else { module };
    let effective_owner = if branch == "method" {
        owner
    } else {
        Ident::default()
    };
    let mut full = IdentityEncoder::new(key);
    full.field(branch)?;
    full.field(effective_module)?;
    full.field(effective_namespace)?;
    full.field(&effective_owner.full)?;
    full.field(name)?;
    full.number(is_const as u8)?;
    full.field(&params_full)?;
    full.field(&ret.full)?;
    let mut stripped = IdentityEncoder::new(key);
    stripped.field(branch)?;
    stripped.field(effective_module)?;
    stripped.field("")?;
    stripped.field(&effective_owner.ns_stripped)?;
    stripped.field(name)?;
    stripped.number(is_const as u8)?;
    stripped.field(&params_stripped)?;
    stripped.field(&ret.ns_stripped)?;
    let mut namespaces = NamespaceAccumulator::new();
    if branch == "global" {
        namespaces.push(key, effective_namespace)?;
    }
    namespaces.extend(key, &effective_owner.namespaces)?;
    namespaces.extend(key, &param_namespaces.finish())?;
    namespaces.extend(key, &ret.namespaces)?;
    Ok(FunctionDeclarationIdentity {
        identity: Ident {
            full: full.finish(),
            ns_stripped: stripped.finish(),
            namespaces: namespaces.finish(),
            display: checked_display(key, &[module, "::", name])?,
            function_owner: None,
            function_name: Some(name.to_owned()),
        },
        footprint: 0,
    })
}

fn duplicate_tail_key_wire(pos: usize, table: usize) -> WireError {
    let field = match table {
        0 => "duplicate TypeReferences key",
        1 => "duplicate TypeIdReferenceToPointer key",
        2 => "duplicate FunctionReferences key",
        3 => "duplicate FunctionIdReferenceToPointer key",
        4 => "duplicate GlobalReferences key",
        _ => "duplicate tail-table key",
    };
    WireError::BadLen { pos, len: 2, field }
}

impl SymTables {
    /// Parse the 7 tail tables of `bytes` into identity maps. Two passes over T3 are avoided
    /// by parsing T1 first (so func owner/param type ptrs resolve to names).
    fn build(bytes: &[u8]) -> Result<Self, WireError> {
        Self::build_inner(bytes, None, bytes.len(), 0)
    }

    /// Parse only `bytes`' own rows, using `fallback` solely to name T1 dependencies omitted from
    /// a minimal independently-remapped mini. The fallback never makes a missing executable ref
    /// valid; it only makes portable identity comparison complete. The composed source/identity
    /// budget prevents a sequence of individually-small minis from growing retained state without
    /// bound.
    fn build_with_type_fallback_and_budget(
        bytes: &[u8],
        fallback: &SymTables,
        total_source_bytes: usize,
        already_charged: usize,
    ) -> Result<Self, WireError> {
        Self::build_inner(bytes, Some(fallback), total_source_bytes, already_charged)
    }

    fn build_inner(
        bytes: &[u8],
        fallback: Option<&SymTables>,
        total_source_bytes: usize,
        already_charged: usize,
    ) -> Result<Self, WireError> {
        let tail = preflight_tail_tables(bytes)?.tail;
        let mut c = Cursor::at(bytes, tail);

        let mut all_ptr_keys: HashSet<i64> = HashSet::new();
        let mut type_id_of_ptr = HashMap::new();
        let mut type_ptr_of_id: HashMap<String, Vec<i64>> = HashMap::new();
        let mut type_ident_of_ptr: HashMap<i64, Ident> = HashMap::new();
        let mut type_name_of_ptr = HashMap::new();

        // T1 TypeReferences: i64 key + (Name, Module, Namespace, TArray<DataType> SubTypes).
        // PASS 1: collect key -> (Name, Module, Namespace, raw subtype DataType bytes). The
        // identity must include each subtype's RESOLVED NAME (so `TSubclassOf<AFoo>` differs
        // from `TSubclassOf<ABar>` — they share Name `TSubclassOf` but distinct subtype ptrs),
        // and subtype ptrs may forward-reference other T1 rows, so build names first.
        let ntypes = c.read_count("TypeReferences")?;
        c.ensure_minimum_remaining(ntypes, 24, "TypeReferences")?;
        let mut raw_types = Vec::with_capacity(ntypes);
        let mut row_by_key = HashMap::with_capacity(ntypes);
        for _ in 0..ntypes {
            let key_pos = c.pos();
            let key = c.read_i64()?;
            if row_by_key.insert(key, raw_types.len()).is_some() {
                return Err(duplicate_tail_key_wire(key_pos, 0));
            }
            all_ptr_keys.insert(key);
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let nsub = c.read_count("TypeRef.SubTypes")?;
            c.ensure_minimum_remaining(nsub, DATA_TYPE_SIZE, "TypeRef.SubTypes")?;
            let mut subs = Vec::with_capacity(nsub);
            for _ in 0..nsub {
                let mut flags = [false; 6];
                for flag in &mut flags {
                    *flag = c.read_bool4()?;
                }
                subs.push(RawTypeIdentityDataType {
                    flags,
                    type_info: c.read_i64()?,
                    token: c.read_i32()?,
                });
            }
            raw_types.push(RawTypeIdentityRow {
                key,
                name,
                module,
                namespace,
                subs,
            });
        }
        // PASS 2: recursively compose complete subtype identities. The old bare-name form
        // T1 Name (no module/namespace), so a subtype contributes NO namespace field — the only
        // collapsed nested template arguments; nested skeletons and namespaces now travel too.
        let mut memo = HashMap::new();
        let mut identity_budget =
            IdentityBudget::for_composed_input(total_source_bytes, already_charged)?;
        for rt in &raw_types {
            let identity = resolve_type_identity(
                rt.key,
                &raw_types,
                &row_by_key,
                fallback,
                &mut memo,
                &mut HashSet::new(),
                0,
            )?;
            identity_budget.charge(rt.key, &identity)?;
            type_name_of_ptr.insert(rt.key, rt.name.clone());
            type_ident_of_ptr.insert(rt.key, identity.clone());
            type_id_of_ptr.insert(rt.key, identity.full.clone());
            type_ptr_of_id
                .entry(identity.full)
                .or_default()
                .push(rt.key);
        }

        // T2 TypeIdReferenceToPointer: i32 id -> i64 ptr.
        let mut typeid_to_ptr = HashMap::new();
        let mut ptr_kind_to_typeid: HashMap<(i64, u32), i32> = HashMap::new();
        for _ in 0..c.read_count("TypeIdRef")? {
            let key_pos = c.pos();
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            all_ptr_keys.insert(ptr);
            if typeid_to_ptr.insert(id, ptr).is_some() {
                return Err(duplicate_tail_key_wire(key_pos, 1));
            }
            ptr_kind_to_typeid
                .entry((ptr, id as u32 & TYPE_ID_OBJECT_MASK))
                .and_modify(|canonical| *canonical = (*canonical).min(id))
                .or_insert(id);
        }

        // T3 FunctionReferences: i64 key + (Name, Module, Namespace, 3 bool, i64 ObjectType,
        // TArray<DataType> params, DataType ret).
        let mut func_id_of_ptr = HashMap::new();
        let mut func_ptr_of_id: HashMap<String, Vec<i64>> = HashMap::new();
        let mut func_ident_of_ptr: HashMap<i64, Ident> = HashMap::new();
        let mut func_name_of_ptr = HashMap::new();
        for _ in 0..c.read_count("FunctionReferences")? {
            let key_pos = c.pos();
            let key = c.read_i64()?;
            if func_id_of_ptr.contains_key(&key) {
                return Err(duplicate_tail_key_wire(key_pos, 2));
            }
            all_ptr_keys.insert(key);
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let is_const = c.read_bool4()?;
            let is_imported = c.read_bool4()?;
            let is_method = c.read_bool4()?;
            let objtype = c.read_i64()?;
            // Use the owner's FULL type identity (name + template subtypes), not just its name,
            // so e.g. `TSubclassOf<AFoo>::opImplConv` and `TSubclassOf<ABar>::opImplConv` (which
            // share the bare owner name `TSubclassOf`) are distinguished. The owner + each param +
            // ret are type identities that carry their OWN (drifting) namespace fields, so compose
            // all three oracle forms in lockstep (GAP-A: a func's own ns is field index 1, then the
            // owner's ns, then each param's, then ret's — traversal order preserved in the list).
            let type_identity = |ptr: &i64| {
                type_ident_of_ptr
                    .get(ptr)
                    .or_else(|| fallback.and_then(|prior| prior.type_ident_of_ptr.get(ptr)))
                    .cloned()
            };
            let owner = type_identity(&objtype).unwrap_or_default();
            let nparams = c.read_count("FuncRef.Params")?;
            let mut params_full = String::new();
            let mut params_stripped = String::new();
            let mut param_ns = NamespaceAccumulator::new();
            for _ in 0..nparams {
                let p = datatype_identity_with_fallback(
                    key,
                    &mut c,
                    &type_ident_of_ptr,
                    fallback.map(|prior| &prior.type_ident_of_ptr),
                )?;
                checked_identity_append(key, &mut params_full, &p.full)?;
                checked_identity_append(key, &mut params_full, ",")?;
                checked_identity_append(key, &mut params_stripped, &p.ns_stripped)?;
                checked_identity_append(key, &mut params_stripped, ",")?;
                param_ns.extend(key, &p.namespaces)?;
            }
            let ret = datatype_identity_with_fallback(
                key,
                &mut c,
                &type_ident_of_ptr,
                fallback.map(|prior| &prior.type_ident_of_ptr),
            )?;
            if is_imported && (is_method || objtype != 0) {
                return Err(WireError::InvalidFunctionReference {
                    key,
                    detail: "imported declarations cannot have a method owner",
                });
            }
            // Project exactly the fields consumed by the runtime lookup branch. Imported
            // declarations ignore Namespace/owner; methods ignore Module/Namespace; globals use
            // Module+Namespace. This prevents byte-distinct rows from aliasing at runtime.
            let (branch, effective_module, effective_namespace, effective_owner) = if is_imported {
                ("imported", module.as_str(), "", Ident::default())
            } else if is_method {
                ("method", "", "", owner.clone())
            } else {
                (
                    "global",
                    module.as_str(),
                    namespace.as_str(),
                    Ident::default(),
                )
            };
            let mut identity = IdentityEncoder::new(key);
            identity.field(branch)?;
            identity.field(effective_module)?;
            identity.field(effective_namespace)?;
            identity.field(&effective_owner.full)?;
            identity.field(&name)?;
            identity.number(is_const as u8)?;
            identity.field(&params_full)?;
            identity.field(&ret.full)?;
            let identity = identity.finish();
            let mut ns_stripped = IdentityEncoder::new(key);
            ns_stripped.field(branch)?;
            ns_stripped.field(effective_module)?;
            ns_stripped.field("")?;
            ns_stripped.field(&effective_owner.ns_stripped)?;
            ns_stripped.field(&name)?;
            ns_stripped.number(is_const as u8)?;
            ns_stripped.field(&params_stripped)?;
            ns_stripped.field(&ret.ns_stripped)?;
            let ns_stripped = ns_stripped.finish();
            let mut namespaces = NamespaceAccumulator::new();
            if !is_imported && !is_method {
                namespaces.push(key, effective_namespace)?;
            }
            namespaces.extend(key, &effective_owner.namespaces)?;
            namespaces.extend(key, &param_ns.finish())?;
            namespaces.extend(key, &ret.namespaces)?;
            let function_identity = Ident {
                full: identity.clone(),
                ns_stripped,
                namespaces: namespaces.finish(),
                display: checked_display(key, &[&module, "::", &name])?,
                function_owner: is_method.then(|| {
                    type_name_of_ptr
                        .get(&objtype)
                        .or_else(|| fallback.and_then(|prior| prior.type_name_of_ptr.get(&objtype)))
                        .cloned()
                        .unwrap_or_default()
                }),
                function_name: Some(name.clone()),
            };
            identity_budget.charge(key, &function_identity)?;
            func_ident_of_ptr.insert(key, function_identity);
            func_id_of_ptr.insert(key, identity.clone());
            func_ptr_of_id.entry(identity).or_default().push(key);
            func_name_of_ptr.insert(key, name);
        }

        // T4 FunctionIdReferenceToPointer: i32 id -> i64 ptr.
        let mut funcid_to_ptr = HashMap::new();
        let mut ptr_to_funcid: HashMap<i64, i32> = HashMap::new();
        for _ in 0..c.read_count("FuncIdRef")? {
            let key_pos = c.pos();
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            all_ptr_keys.insert(ptr);
            if funcid_to_ptr.insert(id, ptr).is_some() {
                return Err(duplicate_tail_key_wire(key_pos, 3));
            }
            ptr_to_funcid
                .entry(ptr)
                .and_modify(|canonical| {
                    // Zero is the null operand sentinel. A real pointer may nevertheless have
                    // a historical T4 alias at zero, so prefer any non-zero alias and otherwise
                    // keep the deterministic smallest wire id.
                    if *canonical == 0 || (id != 0 && id < *canonical) {
                        *canonical = id;
                    }
                })
                .or_insert(id);
        }

        // T5 GlobalReferences: i64 key + (Name, Module, Namespace, i32 bIsString).
        let mut global_id_of_ptr = HashMap::new();
        let mut global_ptr_of_id: HashMap<String, Vec<i64>> = HashMap::new();
        let mut global_ident_of_ptr: HashMap<i64, Ident> = HashMap::new();
        let mut global_is_string_of_ptr = HashMap::new();
        let mut global_name_of_ptr = HashMap::new();
        for _ in 0..c.read_count("GlobalReferences")? {
            let key_pos = c.pos();
            let key = c.read_i64()?;
            if global_id_of_ptr.contains_key(&key) {
                return Err(duplicate_tail_key_wire(key_pos, 4));
            }
            all_ptr_keys.insert(key);
            let name_pos = c.pos();
            let name = c.read_sia_bytes()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let is_string = c.read_bool4()?;
            let name = if is_string {
                name.decode_utf8(name_pos)?
            } else {
                name.decode_ansi()
            };
            let effective_module = if is_string { "" } else { module.as_str() };
            let effective_namespace = if is_string { "" } else { namespace.as_str() };
            let mut identity = IdentityEncoder::new(key);
            identity.field(effective_module)?;
            identity.field(effective_namespace)?;
            identity.field(&name)?;
            identity.number(is_string as u8)?;
            let identity = identity.finish();
            let mut ns_stripped = IdentityEncoder::new(key);
            ns_stripped.field(effective_module)?;
            ns_stripped.field("")?;
            ns_stripped.field(&name)?;
            ns_stripped.number(is_string as u8)?;
            let ns_stripped = ns_stripped.finish();
            let mut global_namespaces = NamespaceAccumulator::new();
            if !is_string {
                global_namespaces.push(key, &namespace)?;
            }
            let global_identity = Ident {
                full: identity.clone(),
                ns_stripped,
                namespaces: global_namespaces.finish(),
                display: checked_display(key, &[&module, "::", &name])?,
                function_owner: None,
                function_name: None,
            };
            identity_budget.charge(key, &global_identity)?;
            global_ident_of_ptr.insert(key, global_identity);
            global_is_string_of_ptr.insert(key, is_string);
            global_id_of_ptr.insert(key, identity.clone());
            global_ptr_of_id.entry(identity).or_default().push(key);
            global_name_of_ptr.insert(key, name);
        }
        for keys in global_ptr_of_id.values_mut() {
            keys.sort_unstable();
        }
        // T6 StaticNames + T7 PropertyReferences are not operand-referenced (member ops carry a
        // TYPE-ID, not a prop key — the prop key is derived from typeid+offset), so skip them.

        let identity_bytes = identity_budget.charged;
        Ok(SymTables {
            type_id_of_ptr,
            type_ptr_of_id,
            type_ident_of_ptr,
            typeid_to_ptr,
            ptr_kind_to_typeid,
            func_id_of_ptr,
            func_ptr_of_id,
            func_ident_of_ptr,
            func_name_of_ptr,
            type_name_of_ptr,
            global_name_of_ptr,
            funcid_to_ptr,
            ptr_to_funcid,
            global_id_of_ptr,
            global_ptr_of_id,
            global_ident_of_ptr,
            global_is_string_of_ptr,
            all_ptr_keys,
            identity_bytes,
        })
    }

    /// Resolve a regen ptr-key to a human name (type/func/global) for diagnostic reporting.
    fn name_of_key(&self, key: i64) -> String {
        if let Some(n) = self.type_name_of_ptr.get(&key) {
            return format!("type {n:?}");
        }
        if let Some(n) = self.func_name_of_ptr.get(&key) {
            return format!("func {n:?}");
        }
        if let Some(n) = self.global_name_of_ptr.get(&key) {
            return format!("global {n:?}");
        }
        "<id-table ptr (T2/T4) with no direct T1/T3/T5 row>".to_string()
    }
}

#[derive(Debug)]
struct IdentityReverseSummary {
    by_skeleton: HashMap<String, Vec<(i64, usize)>>,
    identity_bytes: usize,
}

#[derive(Debug)]
struct SymbolIdentitySummaries {
    types: IdentityReverseSummary,
    functions: IdentityReverseSummary,
    globals: IdentityReverseSummary,
}

impl IdentityReverseSummary {
    fn build(rows: &HashMap<i64, Ident>) -> Result<Self, WireError> {
        Self::build_filtered(rows, |_| true)
    }

    fn build_filtered(
        rows: &HashMap<i64, Ident>,
        include: impl Fn(i64) -> bool,
    ) -> Result<Self, WireError> {
        let mut by_skeleton: HashMap<String, Vec<(i64, usize)>> = HashMap::new();
        let mut identity_bytes = 0usize;
        for (&key, identity) in rows {
            if !include(key) {
                continue;
            }
            let footprint = identity_footprint(key, identity)?;
            identity_bytes = identity_bytes.checked_add(footprint).ok_or(
                WireError::IdentityComparisonBudgetExceeded {
                    max: MAX_IDENTITY_COMPARISON_WORK,
                },
            )?;
            by_skeleton
                .entry(identity.ns_stripped.clone())
                .or_default()
                .push((key, footprint));
        }
        Ok(Self {
            by_skeleton,
            identity_bytes,
        })
    }
}

#[derive(Debug)]
struct IdPointerSummary {
    min_id_by_ptr: HashMap<i64, i32>,
    count_by_ptr: HashMap<i64, usize>,
}

impl IdPointerSummary {
    fn build(rows: &HashMap<i32, i64>) -> Self {
        let mut min_id_by_ptr = HashMap::new();
        let mut count_by_ptr = HashMap::new();
        for (&id, &ptr) in rows {
            min_id_by_ptr
                .entry(ptr)
                .and_modify(|prior: &mut i32| *prior = (*prior).min(id))
                .or_insert(id);
            *count_by_ptr.entry(ptr).or_default() += 1;
        }
        Self {
            min_id_by_ptr,
            count_by_ptr,
        }
    }

    /// Summarize only rows whose id is not already present in `base`. Exact base-row repeats in a
    /// prepared mini are one effective TMap entry, not a second reverse mapping.
    fn build_additional(base: &HashMap<i32, i64>, rows: &HashMap<i32, i64>) -> Self {
        let mut min_id_by_ptr = HashMap::new();
        let mut count_by_ptr = HashMap::new();
        for (&id, &ptr) in rows {
            if base.contains_key(&id) {
                continue;
            }
            min_id_by_ptr
                .entry(ptr)
                .and_modify(|prior: &mut i32| *prior = (*prior).min(id))
                .or_insert(id);
            *count_by_ptr.entry(ptr).or_default() += 1;
        }
        Self {
            min_id_by_ptr,
            count_by_ptr,
        }
    }

    fn count(&self, ptr: i64) -> usize {
        self.count_by_ptr.get(&ptr).copied().unwrap_or_default()
    }

    fn unique_id_with(&self, additional: &Self, ptr: i64) -> Option<i32> {
        match (self.count(ptr), additional.count(ptr)) {
            (1, 0) => self.min_id_by_ptr.get(&ptr).copied(),
            (0, 1) => additional.min_id_by_ptr.get(&ptr).copied(),
            _ => None,
        }
    }
}

fn ensure_unique_symbol_identities(
    table: usize,
    base: &HashMap<i64, Ident>,
    base_summary: &IdentityReverseSummary,
    accepted: &HashMap<i64, Ident>,
    mini: &HashMap<i64, Ident>,
) -> Result<(), RemapError> {
    ensure_unique_symbol_identities_filtered(table, base, base_summary, accepted, mini, |_| true)
}

fn ensure_unique_symbol_identities_filtered(
    table: usize,
    base: &HashMap<i64, Ident>,
    base_summary: &IdentityReverseSummary,
    accepted: &HashMap<i64, Ident>,
    mini: &HashMap<i64, Ident>,
    include_mini: impl Fn(i64) -> bool,
) -> Result<(), RemapError> {
    let mut accepted_by_skeleton: HashMap<&str, Vec<(i64, &Ident, usize)>> = HashMap::new();
    let accepted_identity_bytes = accepted.iter().try_fold(
        0usize,
        |total, (&key, identity)| -> Result<usize, WireError> {
            let footprint = identity_footprint(key, identity)?;
            accepted_by_skeleton
                .entry(identity.ns_stripped.as_str())
                .or_default()
                .push((key, identity, footprint));
            total
                .checked_add(footprint)
                .ok_or(WireError::IdentityComparisonBudgetExceeded {
                    max: MAX_IDENTITY_COMPARISON_WORK,
                })
        },
    )?;
    let mut rows: Vec<(i64, &Ident, usize)> = mini
        .iter()
        .filter(|(key, _)| include_mini(**key))
        .map(|(&key, identity)| Ok((key, identity, identity_footprint(key, identity)?)))
        .collect::<Result<_, WireError>>()?;
    let mini_identity_bytes = rows.iter().try_fold(0usize, |total, (_, _, footprint)| {
        total
            .checked_add(*footprint)
            .ok_or(WireError::IdentityComparisonBudgetExceeded {
                max: MAX_IDENTITY_COMPARISON_WORK,
            })
    })?;
    let identity_bytes = base_summary
        .identity_bytes
        .checked_add(accepted_identity_bytes)
        .and_then(|bytes| bytes.checked_add(mini_identity_bytes))
        .ok_or(WireError::IdentityComparisonBudgetExceeded {
            max: MAX_IDENTITY_COMPARISON_WORK,
        })?;
    let max_comparison_work = identity_bytes
        .checked_mul(IDENTITY_COMPARISON_WORK_MULTIPLIER)
        .unwrap_or(MAX_IDENTITY_COMPARISON_WORK)
        .clamp(MIN_IDENTITY_COMPARISON_WORK, MAX_IDENTITY_COMPARISON_WORK);
    let mut remaining_comparison_work = max_comparison_work;
    rows.sort_by_key(|(key, _, _)| *key);
    let mut mini_by_skeleton: HashMap<&str, Vec<(i64, &Ident, usize)>> = HashMap::new();
    for (key, identity, footprint) in rows {
        // A base/prior key is grandfathered. The collision layer has already required its row
        // bytes to be identical; do not reinterpret historical identity ambiguity here.
        if base.contains_key(&key) || accepted.contains_key(&key) {
            continue;
        }
        let mut matching_prior = None;
        if let Some(candidates) = base_summary.by_skeleton.get(identity.ns_stripped.as_str()) {
            for &(prior_key, prior_footprint) in candidates {
                if prior_key == key {
                    continue;
                }
                let comparison_work = footprint.checked_add(prior_footprint).ok_or(
                    WireError::IdentityComparisonBudgetExceeded {
                        max: max_comparison_work,
                    },
                )?;
                remaining_comparison_work = remaining_comparison_work
                    .checked_sub(comparison_work)
                    .ok_or(WireError::IdentityComparisonBudgetExceeded {
                        max: max_comparison_work,
                    })?;
                let prior = base
                    .get(&prior_key)
                    .expect("base identity summary points to its source row");
                if identity.oracle_eq(prior) {
                    matching_prior = Some(
                        matching_prior.map_or(prior_key, |current: i64| current.min(prior_key)),
                    );
                }
            }
        }
        for candidates in [
            accepted_by_skeleton.get(identity.ns_stripped.as_str()),
            mini_by_skeleton.get(identity.ns_stripped.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            for &(prior_key, prior, prior_footprint) in candidates {
                if prior_key == key {
                    continue;
                }
                let comparison_work = footprint.checked_add(prior_footprint).ok_or(
                    WireError::IdentityComparisonBudgetExceeded {
                        max: max_comparison_work,
                    },
                )?;
                remaining_comparison_work = remaining_comparison_work
                    .checked_sub(comparison_work)
                    .ok_or(WireError::IdentityComparisonBudgetExceeded {
                        max: max_comparison_work,
                    })?;
                if identity.oracle_eq(prior) {
                    matching_prior = Some(
                        matching_prior.map_or(prior_key, |current: i64| current.min(prior_key)),
                    );
                }
            }
        }
        if let Some(prior_key) = matching_prior {
            return Err(RemapError::InvalidTailRow {
                table,
                row_key: key,
                kind: "symbol identity",
                detail: format!(
                    "the same portable identity is already registered at key {prior_key:#x}"
                ),
            });
        }
        mini_by_skeleton
            .entry(identity.ns_stripped.as_str())
            .or_default()
            .push((key, identity, footprint));
    }
    Ok(())
}

fn ensure_unique_id_pointers(
    table: usize,
    base: &HashMap<i32, i64>,
    base_summary: &IdPointerSummary,
    accepted: &HashMap<i32, i64>,
    mini: &HashMap<i32, i64>,
) -> Result<(), RemapError> {
    let mut by_ptr: HashMap<i64, i32> = HashMap::new();
    for (&id, &ptr) in accepted {
        by_ptr
            .entry(ptr)
            .and_modify(|prior| *prior = (*prior).min(id))
            .or_insert(id);
    }
    let mut rows: Vec<(i32, i64)> = mini.iter().map(|(&id, &ptr)| (id, ptr)).collect();
    rows.sort_unstable();
    for (id, ptr) in rows {
        if base.contains_key(&id) || accepted.contains_key(&id) {
            continue;
        }
        let prior = base_summary
            .min_id_by_ptr
            .get(&ptr)
            .into_iter()
            .chain(by_ptr.get(&ptr))
            .copied()
            .min();
        if let Some(prior) = prior {
            return Err(RemapError::InvalidTailRow {
                table,
                row_key: id as i64,
                kind: "pointer-to-id mapping",
                detail: format!("pointer {ptr:#x} is already mapped by id {prior:#x}"),
            });
        }
        by_ptr.insert(ptr, id);
    }
    Ok(())
}

/// Count only mappings added after the pristine base. Keeping this delta separate lets callers
/// query the precomputed base summary without cloning its full T2/T4 map for every mini.
fn additional_pointer_id_counts(
    base: &HashMap<i32, i64>,
    accepted: &HashMap<i32, i64>,
    mini: &HashMap<i32, i64>,
) -> HashMap<i64, usize> {
    let mut counts = HashMap::new();
    for (&id, &ptr) in accepted {
        if !base.contains_key(&id) {
            *counts.entry(ptr).or_default() += 1;
        }
    }
    for (&id, &ptr) in mini {
        if !base.contains_key(&id) && !accepted.contains_key(&id) {
            *counts.entry(ptr).or_default() += 1;
        }
    }
    counts
}

// -------------------------------------------------------------------------------------------------
// Raw tail-row metadata used only by the explicit new-symbol path. The strict/default remapper
// above deliberately keeps its historical parsing and output behavior unchanged.
// -------------------------------------------------------------------------------------------------

#[derive(Clone)]
struct TypeRowMeta {
    start: usize,
    end: usize,
    key: i64,
    name: String,
    module: String,
    namespace: String,
    /// DataType fields embedded by this row.
    type_deps: Vec<DataTypeDep>,
}

#[derive(Clone)]
struct FuncRowMeta {
    start: usize,
    end: usize,
    key: i64,
    name: String,
    module: String,
    namespace: String,
    /// The optional ObjectType plus every parameter/return DataType.
    owner_dep: (usize, i64),
    is_imported: bool,
    is_method: bool,
    type_deps: Vec<DataTypeDep>,
}

#[derive(Clone)]
struct GlobalRowMeta {
    start: usize,
    end: usize,
    key: i64,
    name: String,
    module: String,
    namespace: String,
    is_string: bool,
}

#[derive(Clone)]
struct IdPtrRowMeta {
    start: usize,
    end: usize,
    id: i32,
    ptr: i64,
}

#[derive(Clone)]
struct StaticRowMeta {
    index: usize,
    start: usize,
    end: usize,
    name: String,
}

#[derive(Clone)]
struct PropertyRowMeta {
    index: usize,
    start: usize,
    end: usize,
    key: i64,
    name: String,
    old_type_id: i32,
    /// The member byte offset encoded in key bits 33+.
    member_offset: i32,
}

#[derive(Clone, Copy)]
struct DataTypeDep {
    off: usize,
    ptr: i64,
    token: i32,
    is_auto: bool,
}

#[derive(Clone, Copy)]
enum UniqueRowIndex {
    Unique(usize),
    Ambiguous,
}

fn record_ptr_row(index: &mut HashMap<i64, UniqueRowIndex>, ptr: i64, row: usize) {
    index
        .entry(ptr)
        .and_modify(|state| *state = UniqueRowIndex::Ambiguous)
        .or_insert(UniqueRowIndex::Unique(row));
}

fn duplicate_tail_row(table: usize, row_key: i64, pos: usize) -> RemapError {
    RemapError::InvalidTailRow {
        table,
        row_key,
        kind: "duplicate key",
        detail: format!("the keyed table contains another row for this key (at {pos:#x})"),
    }
}

struct TailMetadata {
    types: Vec<TypeRowMeta>,
    type_by_key: HashMap<i64, usize>,
    type_ids: Vec<IdPtrRowMeta>,
    type_id_by_ptr: HashMap<i64, UniqueRowIndex>,
    funcs: Vec<FuncRowMeta>,
    func_by_key: HashMap<i64, usize>,
    func_ids: Vec<IdPtrRowMeta>,
    func_id_by_ptr: HashMap<i64, UniqueRowIndex>,
    globals: Vec<GlobalRowMeta>,
    static_names: Vec<StaticRowMeta>,
    properties: Vec<PropertyRowMeta>,
    property_by_key: HashMap<i64, usize>,
}

/// Consume one inline DataType and retain the pointer plus the token that determines whether it
/// must be a concrete T1 reference (`ttIdentifier == 5`) or the exact null primitive sentinel.
fn read_datatype_dep(c: &mut Cursor) -> Result<DataTypeDep, WireError> {
    c.skip(8)?; // reference + object-const
    c.skip(4)?; // object-handle (identity normalization handles this in SymTables)
    c.skip(4)?; // read-only/const-handle
    let is_auto = c.read_bool4()?;
    c.skip(4)?; // if-handle-then-const
    let off = c.pos();
    let ptr = c.read_i64()?;
    let token = c.read_i32()?;
    Ok(DataTypeDep {
        off,
        ptr,
        token,
        is_auto,
    })
}

impl TailMetadata {
    fn build(bytes: &[u8]) -> Result<Self, RemapError> {
        let tail = preflight_tail_tables(bytes)?.tail;
        let mut c = Cursor::at(bytes, tail);

        let type_count = c.read_count("TypeReferences")?;
        let mut types = Vec::with_capacity(type_count);
        let mut type_by_key = HashMap::with_capacity(type_count);
        for _ in 0..type_count {
            let start = c.pos();
            let key = c.read_i64()?;
            if type_by_key.insert(key, types.len()).is_some() {
                return Err(duplicate_tail_row(0, key, start));
            }
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let mut type_deps = Vec::new();
            for _ in 0..c.read_count("TypeRef.SubTypes")? {
                type_deps.push(read_datatype_dep(&mut c)?);
            }
            types.push(TypeRowMeta {
                start,
                end: c.pos(),
                key,
                name,
                module,
                namespace,
                type_deps,
            });
        }

        let type_id_count = c.read_count("TypeIdRef")?;
        let mut type_ids = Vec::with_capacity(type_id_count);
        let mut type_id_keys = HashSet::with_capacity(type_id_count);
        let mut type_id_by_ptr = HashMap::with_capacity(type_id_count);
        for _ in 0..type_id_count {
            let start = c.pos();
            let id = c.read_i32()?;
            if !type_id_keys.insert(id) {
                return Err(duplicate_tail_row(1, id as i64, start));
            }
            let ptr = c.read_i64()?;
            record_ptr_row(&mut type_id_by_ptr, ptr, type_ids.len());
            type_ids.push(IdPtrRowMeta {
                start,
                end: c.pos(),
                id,
                ptr,
            });
        }

        let func_count = c.read_count("FunctionReferences")?;
        let mut funcs = Vec::with_capacity(func_count);
        let mut func_by_key = HashMap::with_capacity(func_count);
        for _ in 0..func_count {
            let start = c.pos();
            let key = c.read_i64()?;
            if func_by_key.insert(key, funcs.len()).is_some() {
                return Err(duplicate_tail_row(2, key, start));
            }
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            c.read_bool4()?; // const (projected into SymTables' runtime identity)
            let is_imported = c.read_bool4()?;
            let is_method = c.read_bool4()?;
            let owner_off = c.pos();
            let owner = c.read_i64()?;
            let owner_dep = (owner_off, owner);
            let mut type_deps = Vec::new();
            for _ in 0..c.read_count("FuncRef.Params")? {
                type_deps.push(read_datatype_dep(&mut c)?);
            }
            type_deps.push(read_datatype_dep(&mut c)?); // return type
            funcs.push(FuncRowMeta {
                start,
                end: c.pos(),
                key,
                name,
                module,
                namespace,
                owner_dep,
                is_imported,
                is_method,
                type_deps,
            });
        }

        let func_id_count = c.read_count("FuncIdRef")?;
        let mut func_ids = Vec::with_capacity(func_id_count);
        let mut func_id_keys = HashSet::with_capacity(func_id_count);
        let mut func_id_by_ptr = HashMap::with_capacity(func_id_count);
        for _ in 0..func_id_count {
            let start = c.pos();
            let id = c.read_i32()?;
            if !func_id_keys.insert(id) {
                return Err(duplicate_tail_row(3, id as i64, start));
            }
            let ptr = c.read_i64()?;
            record_ptr_row(&mut func_id_by_ptr, ptr, func_ids.len());
            func_ids.push(IdPtrRowMeta {
                start,
                end: c.pos(),
                id,
                ptr,
            });
        }

        let global_count = c.read_count("GlobalReferences")?;
        let mut globals = Vec::with_capacity(global_count);
        let mut global_keys = HashSet::with_capacity(global_count);
        for _ in 0..global_count {
            let start = c.pos();
            let key = c.read_i64()?;
            if !global_keys.insert(key) {
                return Err(duplicate_tail_row(4, key, start));
            }
            let name_pos = c.pos();
            let name = c.read_sia_bytes()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let is_string = c.read_bool4()?;
            let name = if is_string {
                name.decode_utf8(name_pos)?
            } else {
                name.decode_ansi()
            };
            globals.push(GlobalRowMeta {
                start,
                end: c.pos(),
                key,
                name,
                module,
                namespace,
                is_string,
            });
        }

        let static_count = c.read_count("StaticNames")?;
        let mut static_names = Vec::with_capacity(static_count);
        for index in 0..static_count {
            let start = c.pos();
            let name = c.read_sia()?;
            static_names.push(StaticRowMeta {
                index,
                start,
                end: c.pos(),
                name,
            });
        }

        let property_count = c.read_count("PropertyReferences")?;
        let mut properties = Vec::with_capacity(property_count);
        let mut property_by_key = HashMap::with_capacity(property_count);
        for index in 0..property_count {
            let start = c.pos();
            let key = c.read_i64()?;
            property_by_key.entry(key).or_insert(index);
            let name = c.read_sia()?;
            let old_type_id = c.read_i32()?;
            // Exact inverse of refs.rs: (type_id << 1) | (offset << 33) | 1.
            let member_offset = ((key as u64) >> 33) as u32 as i32;
            properties.push(PropertyRowMeta {
                index,
                start,
                end: c.pos(),
                key,
                name,
                old_type_id,
                member_offset,
            });
        }

        if c.pos() != bytes.len() {
            return Err(RemapError::TailNotAtEof {
                got: c.pos(),
                len: bytes.len(),
            });
        }
        Ok(TailMetadata {
            types,
            type_by_key,
            type_ids,
            type_id_by_ptr,
            funcs,
            func_by_key,
            func_ids,
            func_id_by_ptr,
            globals,
            static_names,
            properties,
            property_by_key,
        })
    }

    fn type_row(&self, key: i64) -> Option<&TypeRowMeta> {
        self.type_by_key
            .get(&key)
            .and_then(|&index| self.types.get(index))
    }

    fn func_row(&self, key: i64) -> Option<&FuncRowMeta> {
        self.func_by_key
            .get(&key)
            .and_then(|&index| self.funcs.get(index))
    }
}

/// Collect the authoritative module identities used by runtime symbol lookup. `Modules` is a
/// TMap, but its outer FString key is only a container key/alias; tail rows bind to the inner
/// `FAngelscriptPrecompiledModule::ModuleName` instead.
fn inner_module_names(bytes: &[u8]) -> Result<HashSet<String>, RemapError> {
    let mut names = HashSet::new();
    let mut outer_names = HashSet::new();
    for (outer, start, _) in super::walk_modules::module_ranges(bytes)? {
        if !outer_names.insert(outer.clone()) {
            return Err(RemapError::ModuleNameCollision { name: outer });
        }
        let mut cursor = Cursor::at(bytes, start);
        cursor.read_fstring()?; // outer TMap key: deliberately not authority
        let inner = cursor.read_sia()?;
        if !inner.is_empty() && !names.insert(inner.clone()) {
            return Err(RemapError::ModuleNameCollision { name: inner });
        }
    }
    Ok(names)
}

fn missing_module_authority(
    table: usize,
    row_key: i64,
    declaration: &str,
    module: &str,
) -> RemapError {
    RemapError::InvalidTailRow {
        table,
        row_key,
        kind: "module authority",
        detail: format!(
            "{declaration} names module {module:?}, which is absent from both the pristine base and the current mini"
        ),
    }
}

fn missing_declaration_membership(
    table: usize,
    row_key: i64,
    detail: impl Into<String>,
) -> RemapError {
    RemapError::InvalidTailRow {
        table,
        row_key,
        kind: "declaration membership",
        detail: detail.into(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeDeclarationKind {
    ScriptLeaf,
    EngineLeaf,
    Template,
    TemplateSentinel,
}

#[derive(Clone, Debug)]
struct TypeDeclarationDescriptor {
    identity: DeclarationIdentity,
    kind: TypeDeclarationKind,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PristinePropertyIdentity {
    owner_identity: String,
    name: String,
    member_offset: i32,
}

#[derive(Debug)]
struct PristineDeclarationAuthority {
    declarations: DeclarationInventory,
    types_by_ptr: HashMap<i64, TypeDeclarationDescriptor>,
    template_bases: HashMap<(String, usize), HashSet<String>>,
    properties: HashSet<PristinePropertyIdentity>,
    orphan_functions: HashSet<i64>,
    script_owners: ScriptOwnerIndex,
}

#[derive(Debug, Default)]
struct ScriptOwnerIndex {
    buckets: HashMap<DeclarationSkeleton, HashMap<String, Ident>>,
}

impl ScriptOwnerIndex {
    fn insert(
        &mut self,
        declaration: &DeclarationIdentity,
        identity: &Ident,
        pos: usize,
    ) -> Result<(), WireError> {
        let bucket = self
            .buckets
            .entry(DeclarationSkeleton::from(declaration))
            .or_default();
        if let Some(prior) = bucket.get(&declaration.namespace) {
            if prior.full == identity.full {
                return Ok(());
            }
            return Err(WireError::BadLen {
                pos,
                len: 2,
                field: "ambiguous script class declaration",
            });
        }
        bucket.insert(declaration.namespace.clone(), identity.clone());
        Ok(())
    }

    fn exact(&self, declaration: &DeclarationIdentity) -> Option<&Ident> {
        self.buckets
            .get(&DeclarationSkeleton::from(declaration))?
            .get(&declaration.namespace)
    }
}

fn type_declaration_descriptor(row: &TypeRowMeta) -> TypeDeclarationDescriptor {
    let kind = if !row.type_deps.is_empty() {
        TypeDeclarationKind::Template
    } else if row.module == "$__T__" {
        TypeDeclarationKind::TemplateSentinel
    } else if row.module.is_empty() {
        TypeDeclarationKind::EngineLeaf
    } else {
        TypeDeclarationKind::ScriptLeaf
    };
    TypeDeclarationDescriptor {
        identity: DeclarationIdentity {
            module: row.module.clone(),
            namespace: row.namespace.clone(),
            name: row.name.clone(),
        },
        kind,
    }
}

impl PristineDeclarationAuthority {
    fn build(
        meta: &TailMetadata,
        syms: &SymTables,
        declarations: DeclarationInventory,
        comparison_budget: &mut IdentityComparisonBudget,
    ) -> Result<Self, WireError> {
        let mut authority_rows = declarations.rows;
        let mut authority_bytes = declarations.bytes;
        let mut charge = |rows: usize, bytes: usize, pos: usize| -> Result<(), WireError> {
            authority_rows = authority_rows.checked_add(rows).ok_or(WireError::BadLen {
                pos,
                len: i64::MAX,
                field: "pristine declaration authority rows",
            })?;
            if authority_rows > MAX_DECLARATION_AUTHORITY_ROWS {
                return Err(WireError::BadLen {
                    pos,
                    len: authority_rows as i64,
                    field: "pristine declaration authority rows",
                });
            }
            authority_bytes = authority_bytes
                .checked_add(bytes)
                .ok_or(WireError::BadLen {
                    pos,
                    len: i64::MAX,
                    field: "pristine declaration authority bytes",
                })?;
            if authority_bytes > MAX_DECLARATION_AUTHORITY_BYTES {
                return Err(WireError::BadLen {
                    pos,
                    len: authority_bytes as i64,
                    field: "pristine declaration authority bytes",
                });
            }
            Ok(())
        };
        let mut types_by_ptr = HashMap::new();
        let mut template_bases: HashMap<(String, usize), HashSet<String>> = HashMap::new();
        let mut script_owners = ScriptOwnerIndex::default();
        for row in &meta.types {
            let descriptor = type_declaration_descriptor(row);
            let mut bytes = declaration_identity_bytes(&descriptor.identity);
            if descriptor.kind == TypeDeclarationKind::Template {
                bytes = bytes
                    .checked_add(row.name.len())
                    .and_then(|value| value.checked_add(row.namespace.len()))
                    .ok_or(WireError::BadLen {
                        pos: row.start,
                        len: i64::MAX,
                        field: "pristine declaration authority bytes",
                    })?;
            }
            charge(1, bytes, row.start)?;
            if descriptor.kind == TypeDeclarationKind::Template {
                let namespaces = template_bases
                    .entry((row.name.clone(), row.type_deps.len()))
                    .or_default();
                namespaces.insert(row.namespace.clone());
            }
            if descriptor.kind == TypeDeclarationKind::ScriptLeaf {
                if let Some(identity) = syms.type_ident_of_ptr.get(&row.key) {
                    script_owners.insert(&descriptor.identity, identity, row.start)?;
                }
            }
            types_by_ptr.insert(row.key, descriptor);
        }

        let mut properties = HashSet::new();
        for row in &meta.properties {
            let Some(owner_ptr) = syms.typeid_to_ptr.get(&row.old_type_id) else {
                continue;
            };
            let Some(owner_identity) = syms.type_id_of_ptr.get(owner_ptr) else {
                continue;
            };
            let bytes = owner_identity
                .len()
                .checked_add(row.name.len())
                .and_then(|value| {
                    value.checked_add(std::mem::size_of::<PristinePropertyIdentity>())
                })
                .ok_or(WireError::BadLen {
                    pos: row.start,
                    len: i64::MAX,
                    field: "pristine declaration authority bytes",
                })?;
            charge(1, bytes, row.start)?;
            properties.insert(PristinePropertyIdentity {
                owner_identity: owner_identity.clone(),
                name: row.name.clone(),
                member_offset: row.member_offset,
            });
        }
        let mut orphan_functions = HashSet::new();
        for row in &meta.funcs {
            let Some(identity) = syms.func_ident_of_ptr.get(&row.key) else {
                continue;
            };
            match match_function_declarations(&[&declarations], identity, comparison_budget)? {
                FunctionDeclarationMatch::Missing => {
                    orphan_functions.insert(row.key);
                }
                FunctionDeclarationMatch::Unique => {}
                FunctionDeclarationMatch::Ambiguous => {
                    return Err(WireError::BadLen {
                        pos: row.start,
                        len: 2,
                        field: "ambiguous function declaration membership",
                    });
                }
            }
        }
        Ok(Self {
            declarations,
            types_by_ptr,
            template_bases,
            properties,
            orphan_functions,
            script_owners,
        })
    }
}

fn match_template_base(
    pristine: &PristineDeclarationAuthority,
    name: &str,
    namespace: &str,
    arity: usize,
    budget: &mut IdentityComparisonBudget,
) -> Result<FunctionDeclarationMatch, WireError> {
    let Some(candidates) = pristine.template_bases.get(&(name.to_owned(), arity)) else {
        return Ok(FunctionDeclarationMatch::Missing);
    };
    if candidates.contains(namespace) {
        return Ok(FunctionDeclarationMatch::Unique);
    }
    Ok(match_unique_namespace(namespace, candidates.iter().map(String::as_str), budget)?.into())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FunctionDeclarationMatch {
    Missing,
    Unique,
    Ambiguous,
}

#[derive(Debug)]
struct IdentityComparisonBudget {
    remaining: usize,
    max: usize,
}

impl IdentityComparisonBudget {
    /// Create one cumulative oracle-work budget for a complete validation/planning phase. Exact
    /// hash-index hits are free; every namespace-tolerant candidate comparison shares this pool.
    fn new(phase_bytes: usize) -> Self {
        let max = phase_bytes
            .saturating_mul(IDENTITY_COMPARISON_WORK_MULTIPLIER)
            .clamp(MIN_IDENTITY_COMPARISON_WORK, MAX_IDENTITY_COMPARISON_WORK);
        Self {
            remaining: max,
            max,
        }
    }

    fn charge(&mut self, query: usize, candidate: usize) -> Result<(), WireError> {
        let work = query
            .checked_add(candidate)
            .ok_or(WireError::IdentityComparisonBudgetExceeded { max: self.max })?;
        self.remaining = self
            .remaining
            .checked_sub(work)
            .ok_or(WireError::IdentityComparisonBudgetExceeded { max: self.max })?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NamespaceMatch<'a> {
    Missing,
    Unique(&'a str),
    Ambiguous,
}

impl From<NamespaceMatch<'_>> for FunctionDeclarationMatch {
    fn from(value: NamespaceMatch<'_>) -> Self {
        match value {
            NamespaceMatch::Missing => Self::Missing,
            NamespaceMatch::Unique(_) => Self::Unique,
            NamespaceMatch::Ambiguous => Self::Ambiguous,
        }
    }
}

fn match_unique_namespace<'a>(
    query: &str,
    candidates: impl IntoIterator<Item = &'a str>,
    budget: &mut IdentityComparisonBudget,
) -> Result<NamespaceMatch<'a>, WireError> {
    let query_footprint = query.len().saturating_add(std::mem::size_of::<String>());
    let mut matched = None;
    for candidate in candidates {
        let candidate_footprint = candidate
            .len()
            .saturating_add(std::mem::size_of::<String>());
        budget.charge(query_footprint, candidate_footprint)?;
        if !ns_drift_ok(query, candidate) {
            continue;
        }
        match matched {
            None => matched = Some(candidate),
            Some(prior) if prior == candidate => {}
            Some(_) => return Ok(NamespaceMatch::Ambiguous),
        }
    }
    Ok(matched.map_or(NamespaceMatch::Missing, NamespaceMatch::Unique))
}

fn has_exact_function_declaration(declarations: &DeclarationInventory, identity: &Ident) -> bool {
    declarations
        .function_exact
        .get(&function_identity_hash(&identity.full))
        .is_some_and(|indices| {
            indices
                .iter()
                .any(|&index| declarations.functions[index].identity.full == identity.full)
        })
}

/// Exact full identity wins across the entire pristine/current union. Without an exact match,
/// benign namespace drift is accepted only when it identifies one distinct declaration; the same
/// exact declaration repeated in pristine/current is deduplicated, while two drift candidates are
/// fail-closed as ambiguous.
fn match_function_declarations(
    inventories: &[&DeclarationInventory],
    identity: &Ident,
    budget: &mut IdentityComparisonBudget,
) -> Result<FunctionDeclarationMatch, WireError> {
    if inventories
        .iter()
        .any(|inventory| has_exact_function_declaration(inventory, identity))
    {
        return Ok(FunctionDeclarationMatch::Unique);
    }
    let query_footprint = identity_footprint(0, identity)?;
    let mut matched_full: Option<&str> = None;
    for inventory in inventories {
        let Some(indices) = inventory.function_buckets.get(&identity.ns_stripped) else {
            continue;
        };
        for &index in indices {
            let candidate = &inventory.functions[index];
            budget.charge(query_footprint, candidate.footprint)?;
            if !identity.oracle_eq(&candidate.identity) {
                continue;
            }
            match matched_full {
                None => matched_full = Some(candidate.identity.full.as_str()),
                Some(full) if full == candidate.identity.full => {}
                Some(_) => return Ok(FunctionDeclarationMatch::Ambiguous),
            }
        }
    }
    Ok(if matched_full.is_some() {
        FunctionDeclarationMatch::Unique
    } else {
        FunctionDeclarationMatch::Missing
    })
}

#[derive(Clone, Copy)]
enum DeclarationSetKind {
    Type,
    Global,
}

fn match_declaration_identities(
    inventories: &[&DeclarationInventory],
    identity: &DeclarationIdentity,
    kind: DeclarationSetKind,
    budget: &mut IdentityComparisonBudget,
) -> Result<FunctionDeclarationMatch, WireError> {
    let exact = |inventory: &DeclarationInventory| match kind {
        DeclarationSetKind::Type => inventory.types.contains(identity),
        DeclarationSetKind::Global => inventory.globals.contains(identity),
    };
    if inventories.iter().any(|inventory| exact(inventory)) {
        return Ok(FunctionDeclarationMatch::Unique);
    }
    let skeleton = DeclarationSkeleton::from(identity);
    let candidates = inventories.iter().flat_map(|inventory| {
        match kind {
            DeclarationSetKind::Type => inventory.type_buckets.get(&skeleton),
            DeclarationSetKind::Global => inventory.global_buckets.get(&skeleton),
        }
        .into_iter()
        .flat_map(HashSet::iter)
        .map(String::as_str)
    });
    Ok(match_unique_namespace(&identity.namespace, candidates, budget)?.into())
}

fn match_property_declarations(
    inventories: &[&DeclarationInventory],
    identity: &PropertyDeclarationIdentity,
    budget: &mut IdentityComparisonBudget,
) -> Result<FunctionDeclarationMatch, WireError> {
    if inventories
        .iter()
        .any(|inventory| inventory.properties.contains(identity))
    {
        return Ok(FunctionDeclarationMatch::Unique);
    }
    let skeleton = PropertyDeclarationSkeleton::from(identity);
    let candidates = inventories.iter().flat_map(|inventory| {
        inventory
            .property_buckets
            .get(&skeleton)
            .into_iter()
            .flat_map(HashSet::iter)
            .map(String::as_str)
    });
    Ok(match_unique_namespace(&identity.owner.namespace, candidates, budget)?.into())
}

/// Require every genuinely new declaration to be owned by either the pristine cache or the
/// module currently being admitted. Previously accepted minis are collision/novelty history,
/// never a source of module authority for a later mini.
fn validate_novel_declaration_membership(
    meta: &TailMetadata,
    syms: &SymTables,
    pristine_syms: &SymTables,
    pristine: &PristineDeclarationAuthority,
    current: &DeclarationInventory,
    has_authority: impl Fn(&str) -> bool,
    is_novel_type: impl Fn(i64) -> bool,
    is_novel_func: impl Fn(i64) -> bool,
    is_novel_global: impl Fn(i64) -> bool,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    let inventories = [&pristine.declarations, current];
    for row in &meta.types {
        if !is_novel_type(row.key) {
            continue;
        }
        let exact_pristine = syms
            .type_id_of_ptr
            .get(&row.key)
            .is_some_and(|identity| pristine_syms.type_ptr_of_id.contains_key(identity));
        if !row.type_deps.is_empty() {
            match match_template_base(
                pristine,
                &row.name,
                &row.namespace,
                row.type_deps.len(),
                comparison_budget,
            )? {
                FunctionDeclarationMatch::Unique => {}
                FunctionDeclarationMatch::Missing => {
                    return Err(missing_declaration_membership(
                        0,
                        row.key,
                        format!(
                            "template instance {}::{} with arity {} has no matching pristine template TypeReferences declaration",
                            row.namespace,
                            row.name,
                            row.type_deps.len()
                        ),
                    ));
                }
                FunctionDeclarationMatch::Ambiguous => {
                    return Err(missing_declaration_membership(
                        0,
                        row.key,
                        "template instance matches more than one namespace-tolerant pristine template declaration",
                    ));
                }
            }
        } else if row.module.is_empty() || row.module == "$__T__" {
            if !exact_pristine {
                return Err(missing_declaration_membership(
                    0,
                    row.key,
                    "engine/native and $__T__ type rows require an exact pristine TypeReferences identity",
                ));
            }
        } else {
            let identity = DeclarationIdentity {
                module: row.module.clone(),
                namespace: row.namespace.clone(),
                name: row.name.clone(),
            };
            match match_declaration_identities(
                &inventories,
                &identity,
                DeclarationSetKind::Type,
                comparison_budget,
            )? {
                FunctionDeclarationMatch::Unique => {}
                FunctionDeclarationMatch::Missing => {
                    return Err(missing_declaration_membership(
                        0,
                        row.key,
                        format!(
                            "script type {}::{} in module {:?} is absent from Classes and Enums in both the pristine base and current mini",
                            row.namespace, row.name, row.module
                        ),
                    ));
                }
                FunctionDeclarationMatch::Ambiguous => {
                    return Err(missing_declaration_membership(
                        0,
                        row.key,
                        "script type matches more than one namespace-tolerant declaration",
                    ));
                }
            }
        }
    }

    for row in &meta.funcs {
        if !is_novel_func(row.key) {
            continue;
        }
        let identity = syms.func_ident_of_ptr.get(&row.key).ok_or_else(|| {
            missing_declaration_membership(
                2,
                row.key,
                "function row has no runtime-effective portable identity",
            )
        })?;
        if row.is_imported {
            // FunctionImports are inventoried so final validation catches deletion of a pristine
            // import. Admission of genuinely novel imports remains fail-closed: their cross-module
            // binding needs authority beyond a self-asserted current record.
            return Err(missing_declaration_membership(
                2,
                row.key,
                format!(
                    "missing import declaration membership for {}::{} from {:?}: an exact current/pristine Module.FunctionImports runtime signature is required",
                    row.namespace, row.name, row.module
                ),
            ));
        }
        if row.is_method || row.module.is_empty() {
            let exact_pristine = syms
                .func_id_of_ptr
                .get(&row.key)
                .is_some_and(|identity| pristine_syms.func_ptr_of_id.contains_key(identity));
            let declaration_match =
                match_function_declarations(&inventories, identity, comparison_budget)?;
            if declaration_match == FunctionDeclarationMatch::Ambiguous {
                return Err(missing_declaration_membership(
                    2,
                    row.key,
                    "method/native function matches more than one namespace-tolerant declaration",
                ));
            }
            if declaration_match == FunctionDeclarationMatch::Missing && !exact_pristine {
                return Err(missing_declaration_membership(
                    2,
                    row.key,
                    "method/native function has neither an exact current/pristine function record nor an exact pristine FunctionReferences identity",
                ));
            }
        } else {
            if !has_authority(&row.module) {
                return Err(missing_module_authority(
                    2,
                    row.key,
                    "global function declaration",
                    &row.module,
                ));
            }
            let declaration_match =
                match_function_declarations(&inventories, identity, comparison_budget)?;
            if declaration_match == FunctionDeclarationMatch::Ambiguous {
                return Err(missing_declaration_membership(
                    2,
                    row.key,
                    "global function matches more than one namespace-tolerant declaration",
                ));
            }
            if declaration_match == FunctionDeclarationMatch::Missing {
                return Err(missing_declaration_membership(
                    2,
                    row.key,
                    format!(
                        "global function {}::{} in module {:?} has no exact current/pristine function record with the same runtime signature",
                        row.namespace, row.name, row.module
                    ),
                ));
            }
        }
    }

    for row in &meta.globals {
        // A string row's Name is the literal value; Module is not consulted for that lookup.
        if !is_novel_global(row.key) || row.is_string {
            continue;
        }
        if row.module.is_empty() {
            let exact_pristine = syms
                .global_id_of_ptr
                .get(&row.key)
                .is_some_and(|identity| pristine_syms.global_ptr_of_id.contains_key(identity));
            if !exact_pristine {
                return Err(missing_declaration_membership(
                    4,
                    row.key,
                    "native global rows require an exact pristine GlobalReferences identity",
                ));
            }
        } else {
            let identity = DeclarationIdentity {
                module: row.module.clone(),
                namespace: row.namespace.clone(),
                name: row.name.clone(),
            };
            match match_declaration_identities(
                &inventories,
                &identity,
                DeclarationSetKind::Global,
                comparison_budget,
            )? {
                FunctionDeclarationMatch::Unique => {}
                FunctionDeclarationMatch::Missing => {
                    return Err(missing_declaration_membership(
                        4,
                        row.key,
                        format!(
                            "global {}::{} in module {:?} is absent from GlobalVariables in both the pristine base and current mini",
                            row.namespace, row.name, row.module
                        ),
                    ));
                }
                FunctionDeclarationMatch::Ambiguous => {
                    return Err(missing_declaration_membership(
                        4,
                        row.key,
                        "global matches more than one namespace-tolerant declaration",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_novel_property_membership(
    meta: &TailMetadata,
    syms: &SymTables,
    pristine_syms: &SymTables,
    pristine: &PristineDeclarationAuthority,
    current: &DeclarationInventory,
    is_novel_property: impl Fn(&PropertyRowMeta) -> bool,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    for row in &meta.properties {
        if !is_novel_property(row) {
            continue;
        }
        let owner_ptr = syms
            .typeid_to_ptr
            .get(&row.old_type_id)
            .or_else(|| pristine_syms.typeid_to_ptr.get(&row.old_type_id))
            .copied()
            .ok_or_else(|| {
                missing_declaration_membership(
                    6,
                    row.key,
                    format!(
                        "property owner type id {:#x} is unresolved",
                        row.old_type_id
                    ),
                )
            })?;
        let descriptor = meta
            .type_row(owner_ptr)
            .map(type_declaration_descriptor)
            .or_else(|| pristine.types_by_ptr.get(&owner_ptr).cloned());
        let Some(descriptor) = descriptor else {
            return Err(missing_declaration_membership(
                6,
                row.key,
                format!("property owner pointer {owner_ptr:#x} has no T1 declaration"),
            ));
        };
        if descriptor.kind == TypeDeclarationKind::ScriptLeaf {
            let property = PropertyDeclarationIdentity {
                owner: descriptor.identity,
                name: row.name.clone(),
            };
            match match_property_declarations(
                &[&pristine.declarations, current],
                &property,
                comparison_budget,
            )? {
                FunctionDeclarationMatch::Unique => {}
                FunctionDeclarationMatch::Missing => {
                    return Err(missing_declaration_membership(
                        6,
                        row.key,
                        format!(
                            "property {:?} is absent from the declaring class in both the pristine base and current mini",
                            row.name
                        ),
                    ));
                }
                FunctionDeclarationMatch::Ambiguous => {
                    return Err(missing_declaration_membership(
                        6,
                        row.key,
                        "property matches more than one namespace-tolerant class declaration",
                    ));
                }
            }
        } else {
            let owner_identity = syms
                .type_id_of_ptr
                .get(&owner_ptr)
                .or_else(|| pristine_syms.type_id_of_ptr.get(&owner_ptr));
            let exact = owner_identity.is_some_and(|owner_identity| {
                pristine.properties.contains(&PristinePropertyIdentity {
                    owner_identity: owner_identity.clone(),
                    name: row.name.clone(),
                    member_offset: row.member_offset,
                })
            });
            if !exact {
                return Err(missing_declaration_membership(
                    6,
                    row.key,
                    "engine/template properties require an exact pristine property row",
                ));
            }
        }
    }
    Ok(())
}

fn has_pristine_property_identity(
    row: &PropertyRowMeta,
    syms: &SymTables,
    pristine_syms: &SymTables,
    pristine: &PristineDeclarationAuthority,
) -> bool {
    syms.typeid_to_ptr
        .get(&row.old_type_id)
        .or_else(|| pristine_syms.typeid_to_ptr.get(&row.old_type_id))
        .and_then(|owner_ptr| {
            syms.type_id_of_ptr
                .get(owner_ptr)
                .or_else(|| pristine_syms.type_id_of_ptr.get(owner_ptr))
        })
        .is_some_and(|owner_identity| {
            pristine.properties.contains(&PristinePropertyIdentity {
                owner_identity: owner_identity.clone(),
                name: row.name.clone(),
                member_offset: row.member_offset,
            })
        })
}

/// Immutable part of sequential StaticNames composition. Independently remapped minis encode
/// their first private T6 row at `pristine_names.len()`, so a later mini cannot be interpreted
/// from the already-grown accumulator alone. Build this once from the exact base all minis were
/// remapped against, then carry only the small name pool and `__STATIC_NAME` lookup sets.
#[derive(Clone, Debug)]
pub(super) struct StaticNameRebaseContext {
    pristine_names: Vec<String>,
    static_accessor_ptrs: HashSet<i64>,
    static_accessor_ids: HashSet<i32>,
}

impl StaticNameRebaseContext {
    pub(super) fn build(base: &[u8]) -> Result<Self, RemapError> {
        let meta = TailMetadata::build(base)?;
        let static_accessor_ptrs: HashSet<i64> = meta
            .funcs
            .iter()
            .filter_map(|row| (row.name == "__STATIC_NAME").then_some(row.key))
            .collect();
        let static_accessor_ids = meta
            .func_ids
            .iter()
            .filter_map(|row| static_accessor_ptrs.contains(&row.ptr).then_some(row.id))
            .collect();
        Ok(Self {
            pristine_names: meta.static_names.into_iter().map(|row| row.name).collect(),
            static_accessor_ptrs,
            static_accessor_ids,
        })
    }

    fn next_is_static_accessor(&self, ins: &super::disasm::Instr, code: &[i32]) -> bool {
        match ins.op.name {
            "CALLSYS" | "FuncPtr" | "Thiscall1" => self
                .static_accessor_ptrs
                .contains(&read_qw(code, ins.offset_dw + 1)),
            "CALL" | "CALLBND" | "CALLINTF" => {
                self.static_accessor_ids.contains(&code[ins.offset_dw + 1])
            }
            _ => false,
        }
    }
}

/// Rebase one independently-remapped mini's absolute StaticNames operands onto the pool produced
/// by earlier minis. Returns the rewritten mini plus exactly the new names it contributes.
///
/// T6 is identity-by-text. Existing names (including duplicates inside this mini) are therefore
/// safely deduplicated; genuinely new rows retain their source order. Bytecode is patched in every
/// function-like record collected by [`collect_module_spans`] (`Functions`, methods, constructors,
/// behavior functions, and global init functions).
pub(super) fn rebase_static_names_for_composition(
    mini: &[u8],
    context: &StaticNameRebaseContext,
    prior_contributions: &[String],
) -> Result<(Vec<u8>, Vec<String>), RemapError> {
    let mini_n = super::walk_modules::module_count(mini);
    if mini_n != 1 {
        return Err(RemapError::NotSingle(mini_n));
    }

    let meta = TailMetadata::build(mini)?;
    // Mirror `plan_static_names`: if a malformed/prior cache already contains duplicate text,
    // the last base occurrence is the canonical one. Contributions are unique by construction.
    let pristine_len = context.pristine_names.len();
    let mut current_by_name = HashMap::<String, i64>::new();
    for (index, name) in context.pristine_names.iter().enumerate() {
        current_by_name.insert(name.clone(), index as i64);
    }
    for (index, name) in prior_contributions.iter().enumerate() {
        current_by_name
            .entry(name.clone())
            .or_insert((pristine_len + index) as i64);
    }

    // Raw operands in this mini address its local T6 rows as if they immediately followed the
    // pristine pool. Plan destination indices in serialized row order, independent of hash order
    // and bytecode traversal order.
    let mut source_to_final = HashMap::<i64, i64>::new();
    let mut appended_names = Vec::<String>::new();
    let mut appended_rows = Vec::<usize>::new();
    for row in &meta.static_names {
        let source = (pristine_len + row.index) as i64;
        let final_index = if let Some(&existing) = current_by_name.get(&row.name) {
            existing
        } else {
            let index = (pristine_len + prior_contributions.len() + appended_names.len()) as i64;
            current_by_name.insert(row.name.clone(), index);
            appended_names.push(row.name.clone());
            appended_rows.push(row.index);
            index
        };
        source_to_final.insert(source, final_index);
    }

    let mod_start = CacheHeader::SIZE;
    let mod_end = module_region_end(mini)?;
    let spans = collect_module_spans(mini)?;
    let mut module_bytes = mini[mod_start..mod_end].to_vec();
    for span in &spans.code {
        let rel = span.data_off - mod_start;
        let mut code: Vec<i32> = (0..span.count)
            .map(|k| {
                let off = rel + k * 4;
                i32::from_le_bytes(module_bytes[off..off + 4].try_into().unwrap())
            })
            .collect();
        let original = code.clone();
        let instrs = disassemble(&original).map_err(|e| RemapError::Disasm(e.to_string()))?;
        for (pos, ins) in instrs.iter().enumerate() {
            if ins.op.name == "STR" {
                let raw = ((original[ins.offset_dw] as u32 >> 16) & 0xffff) as i64;
                if raw >= pristine_len as i64 {
                    let mapped = source_to_final
                        .get(&raw)
                        .copied()
                        .ok_or(RemapError::MissingStaticName(raw))?;
                    let mapped = u16::try_from(mapped)
                        .map_err(|_| RemapError::StaticNameIndexOverflow(mapped))?;
                    let low = code[ins.offset_dw] as u32 & 0x0000_ffff;
                    code[ins.offset_dw] = (low | (u32::from(mapped) << 16)) as i32;
                }
            } else if ins.op.name == "PshC4"
                && instrs
                    .get(pos + 1)
                    .is_some_and(|next| context.next_is_static_accessor(next, &original))
            {
                let raw = original[ins.offset_dw + 1] as i64;
                if raw >= pristine_len as i64 {
                    let mapped = source_to_final
                        .get(&raw)
                        .copied()
                        .ok_or(RemapError::MissingStaticName(raw))?;
                    code[ins.offset_dw + 1] = i32::try_from(mapped)
                        .map_err(|_| RemapError::StaticNameIndexOverflow(mapped))?;
                }
            }
        }
        for (k, &dw) in code.iter().enumerate() {
            let off = rel + k * 4;
            module_bytes[off..off + 4].copy_from_slice(&dw.to_le_bytes());
        }
    }

    // Rebuild only T6. All keyed tables and their byte offsets within the tail remain byte-exact;
    // module bytecode patching is size-preserving.
    let selected: HashSet<usize> = appended_rows.into_iter().collect();
    let tables = super::tables::parse_tail_tables(mini, mod_end)?;
    let static_table = &tables.tables[5];
    let count_pos = static_table.entries_start - 4;
    let mut out = Vec::with_capacity(mini.len());
    out.extend_from_slice(&mini[..mod_start]);
    out.extend_from_slice(&module_bytes);
    out.extend_from_slice(&mini[mod_end..count_pos]);
    out.extend_from_slice(&(appended_names.len() as u32).to_le_bytes());
    for row in &meta.static_names {
        if selected.contains(&row.index) {
            out.extend_from_slice(&mini[row.start..row.end]);
        }
    }
    out.extend_from_slice(&mini[static_table.entries_end..]);
    Ok((out, appended_names))
}

/// One surviving regen-key found in the remapped module's bytes.
#[derive(Debug, Clone)]
pub struct SurvivingKey {
    /// Byte offset of the 8-byte LE value WITHIN the module entry (mod_start-relative).
    pub byte_off: usize,
    pub value: i64,
    /// Human description (symbol the regen-key maps to).
    pub name: String,
}

/// HARD POST-CONDITION: scan the whole module-entry byte range for any 8-byte little-endian
/// int64 that is a REGEN tail-table key but NOT a VANILLA key — i.e. a surviving regen-key
/// that the remap failed to rewrite. Such a value resolves to a null object in vanilla's
/// context and is dereferenced by the engine → boot crash. The invariant is ZERO hits.
///
/// Disambiguation against false positives: regen ptr-keys are large heap pointers
/// (`~0x0000_2xxx_xxxx_xxxx`). A coincidental int64 in non-ref data (e.g. a `double`/`int64`
/// constant baked into bytecode) could in principle equal a regen-key. We suppress that class
/// of false positive two ways: (1) a value that is ALSO a vanilla key is, by construction,
/// already correct (it points at the right vanilla symbol) and is skipped; (2) we require the
/// value to be a real regen TABLE KEY (present in `all_ptr_keys`) — random immediates almost
/// never collide with an actual 48-bit heap pointer that was a live type/func/global at regen
/// time. Any residual hit is reported (offset+value+name) so it can be field-classified rather
/// than silently ignored — correctness over convenience.
fn scan_surviving_regen_keys(
    module_bytes: &[u8],
    regen: &SymTables,
    base: &SymTables,
) -> Vec<SurvivingKey> {
    let mut hits = Vec::new();
    if module_bytes.len() < 8 {
        return hits;
    }
    // Slide an 8-byte window over every byte offset (unaligned: a qword operand sits at a
    // dword boundary, embedded int64s at varying offsets — scan every byte to miss nothing).
    for off in 0..=module_bytes.len() - 8 {
        let v = i64::from_le_bytes(module_bytes[off..off + 8].try_into().unwrap());
        if v == 0 {
            continue;
        }
        if regen.all_ptr_keys.contains(&v) && !base.all_ptr_keys.contains(&v) {
            hits.push(SurvivingKey {
                byte_off: off,
                value: v,
                name: regen.name_of_key(v),
            });
        }
    }
    hits
}

/// Where a ref operand lives within an instruction + which table it keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    GlobalPtr,
    FuncPtr,
    TypePtr,
    FuncId,
    TypeId,
}

/// One ref operand site: the dword index within the instruction + its kind.
pub struct RefSite {
    /// First operand dword index within the instruction (the low dword for a qword).
    pub dw_index: usize,
    pub is_qword: bool,
    pub kind: RefKind,
}

/// Operand sites per opcode. Empty for non-ref ops. The authoritative classification from
/// `findings/decompile-refs.md §3`. ALLOC carries TWO ref operands (type ptr + ctor func id).
/// Shared by the ref-remapper (key->key rewrite) and the bytediff oracle (key->identity N1
/// canonicalization) so both use the SAME op->table map — the make-or-break for a build-portable
/// bytecode compare (`specs/semantic-oracle.md §3.1`).
pub fn ref_sites(op: &str) -> Vec<RefSite> {
    use RefKind::*;
    match op {
        // global ptr (QW @ dword 1)
        "PshGPtr" | "PshG4" | "PGA" | "LDG" => vec![RefSite {
            dw_index: 1,
            is_qword: true,
            kind: GlobalPtr,
        }],
        // LdGRdR4 / CpyGtoV4: wW_QW (QW @ dword 1). CpyVtoG4: rW_QW (QW @ dword 1).
        "LdGRdR4" | "CpyGtoV4" | "CpyVtoG4" => vec![RefSite {
            dw_index: 1,
            is_qword: true,
            kind: GlobalPtr,
        }],
        // SetG4: QW_DW (QW @ dword 1 = global ptr; DW @ dword 3 = literal value, NOT a ref).
        "SetG4" => vec![RefSite {
            dw_index: 1,
            is_qword: true,
            kind: GlobalPtr,
        }],
        // func ptr (QW @ dword 1)
        "CALLSYS" | "FuncPtr" | "Thiscall1" => vec![RefSite {
            dw_index: 1,
            is_qword: true,
            kind: FuncPtr,
        }],
        // type ptr (QW @ dword 1)
        "OBJTYPE" | "FREE" | "FinConstruct" | "CopyScript" => {
            vec![RefSite {
                dw_index: 1,
                is_qword: true,
                kind: TypePtr,
            }]
        }
        // DestructScript: rW_QW (QW @ dword 1 = type ptr).
        "DestructScript" => vec![RefSite {
            dw_index: 1,
            is_qword: true,
            kind: TypePtr,
        }],
        // func id (DW @ dword 1)
        "CALL" | "CALLBND" | "CALLINTF" => vec![RefSite {
            dw_index: 1,
            is_qword: false,
            kind: FuncId,
        }],
        // type id (DW @ dword 1)
        "TYPEID" | "Cast" => vec![RefSite {
            dw_index: 1,
            is_qword: false,
            kind: TypeId,
        }],
        // COPY: W_DW (DW @ dword 1 = type id).
        "COPY" => vec![RefSite {
            dw_index: 1,
            is_qword: false,
            kind: TypeId,
        }],
        // SetListType: rW_DW_DW (type id = INTARG(bc+1) = DW @ dword 2).
        "SetListType" => vec![RefSite {
            dw_index: 2,
            is_qword: false,
            kind: TypeId,
        }],
        // member type-id: ADDSi/LoadThisR (W_DW, DW @ dword 1); LoadRObjR/LoadVObjR (rW_W_DW, DW @ dword 2).
        "ADDSi" | "LoadThisR" => vec![RefSite {
            dw_index: 1,
            is_qword: false,
            kind: TypeId,
        }],
        "LoadRObjR" | "LoadVObjR" => vec![RefSite {
            dw_index: 2,
            is_qword: false,
            kind: TypeId,
        }],
        // ALLOC: QW_DW (type ptr @ dword 1; ctor func id @ dword 3).
        "ALLOC" => vec![
            RefSite {
                dw_index: 1,
                is_qword: true,
                kind: TypePtr,
            },
            RefSite {
                dw_index: 3,
                is_qword: false,
                kind: FuncId,
            },
        ],
        _ => Vec::new(),
    }
}

/// Result of remapping one function's bytecode: per-table count of operands rewritten.
#[derive(Default, Debug, Clone, Copy)]
pub struct RemapCounts {
    pub global_ptr: usize,
    pub func_ptr: usize,
    pub type_ptr: usize,
    pub func_id: usize,
    pub type_id: usize,
    /// Embedded module-record int64 refs (ObjVariableTypes/DerivedFrom/ShadowType/Factory/Behavior).
    pub embed_type_ptr: usize,
    pub embed_func_id: usize,
}

impl RemapCounts {
    fn add(&mut self, other: &RemapCounts) {
        self.global_ptr += other.global_ptr;
        self.func_ptr += other.func_ptr;
        self.type_ptr += other.type_ptr;
        self.func_id += other.func_id;
        self.type_id += other.type_id;
        self.embed_type_ptr += other.embed_type_ptr;
        self.embed_func_id += other.embed_func_id;
    }
    pub fn total(&self) -> usize {
        self.global_ptr
            + self.func_ptr
            + self.type_ptr
            + self.func_id
            + self.type_id
            + self.embed_type_ptr
            + self.embed_func_id
    }
}

/// Resolve a regen ptr-key to the equivalent base ptr-key by identity. `None` if the regen
/// ptr isn't in the regen table (e.g. an id that didn't index a ptr — caller handles).
fn remap_ptr(
    kind: &'static str,
    op: &'static str,
    regen_key: i64,
    regen_id_of_ptr: &HashMap<i64, String>,
    regen_name_of_ptr: &HashMap<i64, String>,
    base_ptr_of_id: &HashMap<String, Vec<i64>>,
) -> Result<i64, RemapError> {
    let identity = regen_id_of_ptr
        .get(&regen_key)
        .ok_or_else(|| RemapError::Unresolved {
            kind,
            op,
            key: regen_key,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
        })?;
    match base_ptr_of_id.get(identity).map(|v| v.as_slice()) {
        Some([k]) => Ok(*k),
        Some([]) | None => Err(RemapError::Unresolved {
            kind,
            op,
            key: regen_key,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
        }),
        Some(many) => Err(RemapError::Ambiguous {
            kind,
            op,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
            n: many.len(),
        }),
    }
}

/// Flags that bytecode may OR onto a core AngelScript type-id. T2
/// (`TypeIdReferenceToPointer`) is keyed only by `(MASK_SEQNBR | MASK_OBJECT)`; handle/const
/// qualifiers are operand-local and must survive a remap unchanged.
const TYPE_ID_CORE_MASK: u32 = 0x1fff_ffff; // MASK_SEQNBR | MASK_OBJECT
const TYPE_ID_OBJECT_MASK: u32 = 0x1c00_0000;
const TYPE_ID_QUALIFIER_MASK: u32 = 0x6000_0000; // OBJHANDLE | HANDLETOCONST
const TYPE_ID_SEQUENCE_MASK: u32 = 0x03ff_ffff;
const LAST_PRIMITIVE_TYPE_ID: i32 = 11;

fn valid_type_id_core(id: i32) -> bool {
    if (id as u32) & TYPE_ID_SEQUENCE_MASK <= LAST_PRIMITIVE_TYPE_ID as u32
        || (id as u32) & !TYPE_ID_CORE_MASK != 0
    {
        return false;
    }
    matches!(
        (id as u32) & TYPE_ID_OBJECT_MASK,
        0 | 0x0400_0000 | 0x0800_0000 | 0x1000_0000
    )
}

fn valid_datatype_pointer(dep: DataTypeDep, has_concrete_type_ptr: impl Fn(i64) -> bool) -> bool {
    if dep.is_auto {
        return dep.token == 5 && dep.ptr == 0;
    }
    match dep.token {
        5 => has_concrete_type_ptr(dep.ptr),
        // Every primitive/void/? keyword is table-independent.
        0x3b | 0x41 | 0x44 | 0x45 | 0x46 | 0x47 | 0x4b | 0x4c | 0x4d | 0x4e | 0x50 | 0x51
        | 0x52 => dep.ptr == 0,
        _ => false,
    }
}

fn split_type_id_operand(id: i32) -> (i32, u32) {
    let raw = id as u32;
    ((raw & TYPE_ID_CORE_MASK) as i32, raw & !TYPE_ID_CORE_MASK)
}

fn apply_type_id_operand_flags(core: i32, flags: u32) -> i32 {
    ((core as u32 & TYPE_ID_CORE_MASK) | flags) as i32
}

/// Validate a prepared mini against the exact cache it will be composed with.
///
/// A remapped mini may reference either an existing base symbol or a genuinely new symbol whose
/// minimal row it retains. Anything else is a stale compiler-generation key which the engine
/// resolves as null. Validate both executable/module-record operands and dependencies retained in
/// T1-T4/T7 before the sequential guard records any state.
pub(super) struct EffectiveReferenceBase {
    /// Shared with the allow-new/loadout analyzer so the pristine cache's large identity and
    /// declaration maps are constructed and retained only once.
    base: Arc<AllowNewBaseContext>,
    base_property_keys: HashSet<i64>,
    base_type_ids: IdPointerSummary,
    base_func_ids: IdPointerSummary,
}

impl std::fmt::Debug for EffectiveReferenceBase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectiveReferenceBase")
            .field("base_source_bytes", &self.base.source_bytes)
            .field("base_property_keys", &self.base_property_keys.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct EffectiveReferenceState {
    accepted_source_bytes: usize,
    accepted_identity_bytes: usize,
    accepted_type_identities: HashMap<i64, Ident>,
    accepted_func_identities: HashMap<i64, Ident>,
    accepted_global_identities: HashMap<i64, Ident>,
    accepted_type_ids: HashMap<i32, i64>,
    accepted_func_ids: HashMap<i32, i64>,
}

pub(super) struct ReferenceContribution {
    type_identities: HashMap<i64, Ident>,
    func_identities: HashMap<i64, Ident>,
    global_identities: HashMap<i64, Ident>,
    type_ids: HashMap<i32, i64>,
    func_ids: HashMap<i32, i64>,
    source_bytes: usize,
    identity_bytes: usize,
}

impl EffectiveReferenceBase {
    pub(super) fn build(base: &[u8]) -> Result<Self, RemapError> {
        preflight_cache_module_work(base)?;
        Self::from_allow_new_base(Arc::new(build_allow_new_base_context(base)?))
    }

    fn from_allow_new_base(base: Arc<AllowNewBaseContext>) -> Result<Self, RemapError> {
        let base_property_keys = base.meta.properties.iter().map(|row| row.key).collect();
        let base_type_ids = IdPointerSummary::build(&base.syms.typeid_to_ptr);
        let base_func_ids = IdPointerSummary::build(&base.syms.funcid_to_ptr);
        Ok(Self {
            base,
            base_property_keys,
            base_type_ids,
            base_func_ids,
        })
    }

    pub(super) fn validate(
        &self,
        state: &EffectiveReferenceState,
        mini: &[u8],
    ) -> Result<ReferenceContribution, RemapError> {
        preflight_mini_module_work(mini)?;
        let base_syms = &self.base.syms;
        let total_source_bytes = self
            .base
            .source_bytes
            .checked_add(state.accepted_source_bytes)
            .and_then(|bytes| bytes.checked_add(mini.len()))
            .ok_or(WireError::IdentityBudgetExceeded {
                max: MAX_IDENTITY_BUDGET,
            })?;
        let already_charged = base_syms
            .identity_bytes
            .checked_add(state.accepted_identity_bytes)
            .ok_or(WireError::IdentityBudgetExceeded {
                max: MAX_IDENTITY_BUDGET,
            })?;
        let mini_syms = SymTables::build_with_type_fallback_and_budget(
            mini,
            base_syms,
            total_source_bytes,
            already_charged,
        )?;
        let meta = TailMetadata::build(mini)?;
        let current_module_authorities = inner_module_names(mini)?;
        let mut comparison_budget = IdentityComparisonBudget::new(
            total_source_bytes
                .saturating_add(already_charged)
                .saturating_add(self.base.declarations.declarations.bytes),
        );
        let current_declarations = collect_declaration_inventory(
            mini,
            &mini_syms,
            Some(base_syms),
            Some(&self.base.declarations.script_owners),
            &meta,
            &mut comparison_budget,
        )?
        .declarations;
        validate_novel_declaration_membership(
            &meta,
            &mini_syms,
            base_syms,
            &self.base.declarations,
            &current_declarations,
            |module| {
                self.base.module_authorities.contains(module)
                    || current_module_authorities.contains(module)
            },
            |key| !base_syms.type_ident_of_ptr.contains_key(&key),
            |key| !base_syms.func_ident_of_ptr.contains_key(&key),
            |key| !base_syms.global_ident_of_ptr.contains_key(&key),
            &mut comparison_budget,
        )?;
        validate_novel_property_membership(
            &meta,
            &mini_syms,
            base_syms,
            &self.base.declarations,
            &current_declarations,
            |row| !self.base_property_keys.contains(&row.key),
            &mut comparison_budget,
        )?;

        // A retained row may repeat an existing key byte-for-byte (the collision layer proves
        // that), but it may not register an already-known portable symbol identity under
        // a second key. Non-colliding duplicate registrations are just as fatal as stale refs.
        ensure_unique_symbol_identities(
            0,
            &base_syms.type_ident_of_ptr,
            &self.base.identity_summaries.types,
            &state.accepted_type_identities,
            &mini_syms.type_ident_of_ptr,
        )?;
        ensure_unique_symbol_identities(
            2,
            &base_syms.func_ident_of_ptr,
            &self.base.identity_summaries.functions,
            &state.accepted_func_identities,
            &mini_syms.func_ident_of_ptr,
        )?;
        ensure_unique_symbol_identities_filtered(
            4,
            &base_syms.global_ident_of_ptr,
            &self.base.identity_summaries.globals,
            &state.accepted_global_identities,
            &mini_syms.global_ident_of_ptr,
            |key| {
                !mini_syms
                    .global_is_string_of_ptr
                    .get(&key)
                    .copied()
                    .unwrap_or(false)
            },
        )?;
        ensure_unique_id_pointers(
            1,
            &base_syms.typeid_to_ptr,
            &self.base_type_ids,
            &state.accepted_type_ids,
            &mini_syms.typeid_to_ptr,
        )?;
        ensure_unique_id_pointers(
            3,
            &base_syms.funcid_to_ptr,
            &self.base_func_ids,
            &state.accepted_func_ids,
            &mini_syms.funcid_to_ptr,
        )?;

        let has_type_ptr = |key: i64| {
            key == 0
                || base_syms.type_id_of_ptr.contains_key(&key)
                || mini_syms.type_id_of_ptr.contains_key(&key)
        };
        let has_concrete_type_ptr = |key: i64| {
            key != 0
                && (base_syms.type_id_of_ptr.contains_key(&key)
                    || mini_syms.type_id_of_ptr.contains_key(&key))
        };
        let has_func_ptr = |key: i64| {
            key != 0
                && (base_syms.func_id_of_ptr.contains_key(&key)
                    || mini_syms.func_id_of_ptr.contains_key(&key))
        };
        let has_global_ptr = |key: i64| {
            key != 0
                && (base_syms.global_id_of_ptr.contains_key(&key)
                    || mini_syms.global_id_of_ptr.contains_key(&key))
        };
        let has_type_id = |id: i32| {
            base_syms.typeid_to_ptr.contains_key(&id) || mini_syms.typeid_to_ptr.contains_key(&id)
        };
        // T4[0] may define a real function, but raw function-id references use zero as null.
        let has_func_id = |id: i32| {
            id != 0
                && (base_syms.funcid_to_ptr.contains_key(&id)
                    || mini_syms.funcid_to_ptr.contains_key(&id))
        };
        let has_property_key = |key: i64| {
            self.base_property_keys.contains(&key) || meta.property_by_key.contains_key(&key)
        };
        let additional_type_ids =
            IdPointerSummary::build_additional(&base_syms.typeid_to_ptr, &mini_syms.typeid_to_ptr);

        let tail_dependency = |table, row_key, kind, dependency| {
            Err(RemapError::UnresolvedTailDependency {
                table,
                row_key,
                kind,
                dependency,
            })
        };
        let invalid_tail = |table, row_key, kind, detail: String| {
            Err(RemapError::InvalidTailRow {
                table,
                row_key,
                kind,
                detail,
            })
        };

        for (table, rows) in [
            (0, &meta.types.iter().map(|row| row.key).collect::<Vec<_>>()),
            (2, &meta.funcs.iter().map(|row| row.key).collect::<Vec<_>>()),
            (
                4,
                &meta.globals.iter().map(|row| row.key).collect::<Vec<_>>(),
            ),
        ] {
            if rows.contains(&0) {
                return invalid_tail(
                    table,
                    0,
                    "OldReference key",
                    "zero is the null sentinel".to_owned(),
                );
            }
        }
        for row in &meta.types {
            for &dependency in &row.type_deps {
                if !valid_datatype_pointer(dependency, has_concrete_type_ptr) {
                    return tail_dependency(0, row.key, "DataType pointer", dependency.ptr);
                }
            }
        }
        for row in &meta.type_ids {
            if !valid_type_id_core(row.id) {
                return invalid_tail(
                    1,
                    row.id as i64,
                    "type-id key",
                    "T2 keys must be unqualified non-primitive core ids".to_owned(),
                );
            }
            // T2 is the runtime object/handle TypeId map. Unlike a primitive DataType's optional
            // `OldReference`, its target can never be the null sentinel.
            if !has_concrete_type_ptr(row.ptr) {
                return tail_dependency(1, row.id as i64, "type pointer", row.ptr);
            }
        }
        for row in &meta.funcs {
            if row.is_method != (row.owner_dep.1 != 0) {
                return invalid_tail(
                    2,
                    row.key,
                    "method owner",
                    "bIsMethod must exactly match whether ObjectType is concrete".to_owned(),
                );
            }
            if row.owner_dep.1 != 0 && !has_concrete_type_ptr(row.owner_dep.1) {
                return tail_dependency(2, row.key, "owner type pointer", row.owner_dep.1);
            }
            for &dependency in &row.type_deps {
                if !valid_datatype_pointer(dependency, has_concrete_type_ptr) {
                    return tail_dependency(2, row.key, "DataType pointer", dependency.ptr);
                }
            }
        }
        for row in &meta.func_ids {
            if !has_func_ptr(row.ptr) {
                return tail_dependency(3, row.id as i64, "function pointer", row.ptr);
            }
        }

        let additional_type_id_count = additional_pointer_id_counts(
            &base_syms.typeid_to_ptr,
            &state.accepted_type_ids,
            &mini_syms.typeid_to_ptr,
        );
        for row in &meta.types {
            // Historical/base rows are grandfathered. Their keyed bytes already matched the
            // pristine cache exactly, whose reverse-map ambiguity is not this mini's doing.
            if base_syms.type_ident_of_ptr.contains_key(&row.key)
                || state.accepted_type_identities.contains_key(&row.key)
            {
                continue;
            }
            let count = self.base_type_ids.count(row.key).saturating_add(
                additional_type_id_count
                    .get(&row.key)
                    .copied()
                    .unwrap_or_default(),
            );
            if count != 1 {
                return invalid_tail(
                    0,
                    row.key,
                    "reverse type-id mapping",
                    "every T1 row must have exactly one effective T2 id".to_owned(),
                );
            }
        }
        let additional_func_id_count = additional_pointer_id_counts(
            &base_syms.funcid_to_ptr,
            &state.accepted_func_ids,
            &mini_syms.funcid_to_ptr,
        );
        for row in &meta.funcs {
            if base_syms.func_ident_of_ptr.contains_key(&row.key)
                || state.accepted_func_identities.contains_key(&row.key)
            {
                continue;
            }
            let count = self.base_func_ids.count(row.key).saturating_add(
                additional_func_id_count
                    .get(&row.key)
                    .copied()
                    .unwrap_or_default(),
            );
            if count != 1 {
                return invalid_tail(
                    2,
                    row.key,
                    "reverse function-id mapping",
                    "every T3 row must have exactly one effective T4 id".to_owned(),
                );
            }
        }
        for row in &meta.properties {
            if !has_type_id(row.old_type_id) {
                return tail_dependency(6, row.key, "type id", row.old_type_id as i64);
            }
            let expected = property_key(row.old_type_id, row.member_offset);
            if row.key != expected {
                return invalid_tail(
                    6,
                    row.key,
                    "property key",
                    format!("expected {expected:#x} from its type id and member offset"),
                );
            }
        }

        let spans = collect_module_spans(mini)?;
        for span in &spans.code {
            let code: Vec<i32> = (0..span.count)
                .map(|index| {
                    let off = span.data_off + index * 4;
                    i32::from_le_bytes(mini[off..off + 4].try_into().unwrap())
                })
                .collect();
            let instructions =
                disassemble(&code).map_err(|error| RemapError::Disasm(error.to_string()))?;
            for instruction in &instructions {
                if matches!(
                    instruction.op.name,
                    "ADDSi" | "LoadThisR" | "LoadRObjR" | "LoadVObjR"
                ) {
                    let Some(&raw_type_id) = instruction.dwords.first() else {
                        return Err(RemapError::Disasm(format!(
                            "{} is missing its owner type-id operand",
                            instruction.op.name
                        )));
                    };
                    let type_id = raw_type_id as i32;
                    let member_offset = instruction
                        .words
                        .last()
                        .copied()
                        .map(|word| i32::from(word as i16))
                        .ok_or_else(|| {
                            RemapError::Disasm(format!(
                                "{} is missing its member-offset operand",
                                instruction.op.name
                            ))
                        })?;
                    if !has_type_id(type_id)
                        || !has_property_key(property_key(type_id, member_offset))
                    {
                        return Err(RemapError::UnresolvedEffectiveReference {
                            kind: "property reference",
                            op: instruction.op.name,
                            key: property_key(type_id, member_offset),
                        });
                    }
                }
                for site in ref_sites(instruction.op.name) {
                    let off = instruction.offset_dw + site.dw_index;
                    let (valid, key, kind) = match site.kind {
                        RefKind::GlobalPtr => {
                            let key = read_qw(&code, off);
                            (has_global_ptr(key), key, "global pointer")
                        }
                        RefKind::FuncPtr => {
                            let key = read_qw(&code, off);
                            (has_func_ptr(key), key, "function pointer")
                        }
                        RefKind::TypePtr => {
                            let key = read_qw(&code, off);
                            (has_concrete_type_ptr(key), key, "type pointer")
                        }
                        RefKind::FuncId => {
                            let id = code[off];
                            let native_alloc_without_ctor =
                                if id == 0 && instruction.op.name == "ALLOC" {
                                    let type_ptr = read_qw(&code, instruction.offset_dw + 1);
                                    self.base_type_ids
                                        .unique_id_with(&additional_type_ids, type_ptr)
                                        .is_some_and(|core| {
                                            valid_type_id_core(core)
                                                && matches!(
                                                    core as u32 & TYPE_ID_OBJECT_MASK,
                                                    0x0400_0000 | 0x1000_0000
                                                )
                                        })
                                } else {
                                    false
                                };
                            (
                                has_func_id(id) || native_alloc_without_ctor,
                                id as i64,
                                "function id",
                            )
                        }
                        RefKind::TypeId => {
                            let (id, flags) = split_type_id_operand(code[off]);
                            let qualifiers_valid = flags & !TYPE_ID_QUALIFIER_MASK == 0;
                            let qualified_object =
                                flags == 0 || (id as u32 & TYPE_ID_OBJECT_MASK) != 0;
                            (
                                (qualifiers_valid && qualified_object && has_type_id(id))
                                    || (flags == 0 && (0..=LAST_PRIMITIVE_TYPE_ID).contains(&id)),
                                id as i64,
                                "type id",
                            )
                        }
                    };
                    if !valid {
                        return Err(RemapError::UnresolvedEffectiveReference {
                            kind,
                            op: instruction.op.name,
                            key,
                        });
                    }
                }
            }
        }

        for embed in &spans.embeds {
            let raw =
                i64::from_le_bytes(mini[embed.byte_off..embed.byte_off + 8].try_into().unwrap());
            match embed.kind {
                EmbedKind::TypePtr(rule)
                    if match rule {
                        TypePtrRule::Concrete => !has_concrete_type_ptr(raw),
                        TypePtrRule::Optional => !has_type_ptr(raw),
                        TypePtrRule::Null => raw != 0,
                        TypePtrRule::Invalid => true,
                    } =>
                {
                    return Err(RemapError::UnresolvedEffectiveReference {
                        kind: "embedded type pointer",
                        op: "module record",
                        key: raw,
                    });
                }
                EmbedKind::FuncId => {
                    if raw == 0 {
                        continue;
                    }
                    let Ok(id) = i32::try_from(raw) else {
                        return Err(RemapError::UnresolvedEffectiveReference {
                            kind: "embedded function id",
                            op: "Factory/BehaviorRefs",
                            key: raw,
                        });
                    };
                    if has_func_id(id) {
                        let ptr = mini_syms
                            .funcid_to_ptr
                            .get(&id)
                            .or_else(|| base_syms.funcid_to_ptr.get(&id))
                            .copied()
                            .unwrap();
                        if !has_func_ptr(ptr) {
                            return Err(RemapError::UnresolvedEffectiveReference {
                                kind: "embedded function id",
                                op: "Factory/BehaviorRefs",
                                key: raw,
                            });
                        }
                    } else {
                        return Err(RemapError::UnresolvedEffectiveReference {
                            kind: "embedded function id",
                            op: "Factory/BehaviorRefs",
                            key: raw,
                        });
                    }
                }
                EmbedKind::TypePtr(_) => {}
            }
        }
        let mut persistent_identity_bytes = 0usize;
        for (&key, identity) in &mini_syms.type_ident_of_ptr {
            if !base_syms.type_ident_of_ptr.contains_key(&key)
                && !state.accepted_type_identities.contains_key(&key)
            {
                persistent_identity_bytes = persistent_identity_bytes
                    .checked_add(identity_footprint(key, identity)?)
                    .ok_or(WireError::IdentityBudgetExceeded {
                        max: MAX_IDENTITY_BUDGET,
                    })?;
            }
        }
        for (&key, identity) in &mini_syms.func_ident_of_ptr {
            if !base_syms.func_ident_of_ptr.contains_key(&key)
                && !state.accepted_func_identities.contains_key(&key)
            {
                persistent_identity_bytes = persistent_identity_bytes
                    .checked_add(identity_footprint(key, identity)?)
                    .ok_or(WireError::IdentityBudgetExceeded {
                        max: MAX_IDENTITY_BUDGET,
                    })?;
            }
        }
        for (&key, identity) in &mini_syms.global_ident_of_ptr {
            if !mini_syms
                .global_is_string_of_ptr
                .get(&key)
                .copied()
                .unwrap_or(false)
                && !base_syms.global_ident_of_ptr.contains_key(&key)
                && !state.accepted_global_identities.contains_key(&key)
            {
                persistent_identity_bytes = persistent_identity_bytes
                    .checked_add(identity_footprint(key, identity)?)
                    .ok_or(WireError::IdentityBudgetExceeded {
                        max: MAX_IDENTITY_BUDGET,
                    })?;
            }
        }
        // Retain only genuinely new state. Exact base/prior repeats were already proven
        // byte-identical and need no second owned map entry (or uncharged duplicate heap copy).
        let type_identities = mini_syms
            .type_ident_of_ptr
            .into_iter()
            .filter(|(key, _)| {
                !base_syms.type_ident_of_ptr.contains_key(key)
                    && !state.accepted_type_identities.contains_key(key)
            })
            .collect();
        let func_identities = mini_syms
            .func_ident_of_ptr
            .into_iter()
            .filter(|(key, _)| {
                !base_syms.func_ident_of_ptr.contains_key(key)
                    && !state.accepted_func_identities.contains_key(key)
            })
            .collect();
        let global_is_string_of_ptr = &mini_syms.global_is_string_of_ptr;
        let global_identities = mini_syms
            .global_ident_of_ptr
            .into_iter()
            .filter(|(key, _)| {
                !global_is_string_of_ptr.get(key).copied().unwrap_or(false)
                    && !base_syms.global_ident_of_ptr.contains_key(key)
                    && !state.accepted_global_identities.contains_key(key)
            })
            .collect();
        let type_ids = mini_syms
            .typeid_to_ptr
            .into_iter()
            .filter(|(id, _)| {
                !base_syms.typeid_to_ptr.contains_key(id)
                    && !state.accepted_type_ids.contains_key(id)
            })
            .collect();
        let func_ids = mini_syms
            .funcid_to_ptr
            .into_iter()
            .filter(|(id, _)| {
                !base_syms.funcid_to_ptr.contains_key(id)
                    && !state.accepted_func_ids.contains_key(id)
            })
            .collect();
        Ok(ReferenceContribution {
            type_identities,
            func_identities,
            global_identities,
            type_ids,
            func_ids,
            source_bytes: mini.len(),
            identity_bytes: persistent_identity_bytes,
        })
    }

    pub(super) fn validate_composed_declarations(&self, bytes: &[u8]) -> Result<(), RemapError> {
        validate_composed_module_records_with_pristine(bytes, Some(&self.base.declarations))
    }
}

impl EffectiveReferenceState {
    pub(super) fn record(&mut self, contribution: ReferenceContribution) {
        self.accepted_source_bytes = self
            .accepted_source_bytes
            .checked_add(contribution.source_bytes)
            .expect("validated reference-source budget cannot overflow while recording");
        self.accepted_identity_bytes = self
            .accepted_identity_bytes
            .checked_add(contribution.identity_bytes)
            .expect("validated identity budget cannot overflow while recording");
        self.accepted_type_identities
            .extend(contribution.type_identities);
        self.accepted_func_identities
            .extend(contribution.func_identities);
        self.accepted_global_identities
            .extend(contribution.global_identities);
        self.accepted_type_ids.extend(contribution.type_ids);
        self.accepted_func_ids.extend(contribution.func_ids);
    }
}

/// Remap one function's bytecode dwords IN PLACE. `code` is the function's `Vec<i32>`.
fn remap_bytecode(
    code: &mut [i32],
    regen: &SymTables,
    base: &SymTables,
) -> Result<RemapCounts, RemapError> {
    let instrs = disassemble(code).map_err(|e| RemapError::Disasm(e.to_string()))?;
    let mut counts = RemapCounts::default();
    for ins in &instrs {
        for site in ref_sites(ins.op.name) {
            let base_off = ins.offset_dw + site.dw_index;
            match site.kind {
                RefKind::GlobalPtr => {
                    let regen_key = read_qw(code, base_off);
                    let nk = remap_ptr(
                        "global",
                        ins.op.name,
                        regen_key,
                        &regen.global_id_of_ptr,
                        &regen.global_name_of_ptr,
                        &base.global_ptr_of_id,
                    )?;
                    write_qw(code, base_off, nk);
                    counts.global_ptr += 1;
                }
                RefKind::FuncPtr => {
                    let regen_key = read_qw(code, base_off);
                    let nk = remap_ptr(
                        "function",
                        ins.op.name,
                        regen_key,
                        &regen.func_id_of_ptr,
                        &regen.func_name_of_ptr,
                        &base.func_ptr_of_id,
                    )?;
                    write_qw(code, base_off, nk);
                    counts.func_ptr += 1;
                }
                RefKind::TypePtr => {
                    let regen_key = read_qw(code, base_off);
                    let nk = remap_ptr(
                        "type",
                        ins.op.name,
                        regen_key,
                        &regen.type_id_of_ptr,
                        &regen.type_name_of_ptr,
                        &base.type_ptr_of_id,
                    )?;
                    write_qw(code, base_off, nk);
                    counts.type_ptr += 1;
                }
                RefKind::FuncId => {
                    let regen_id = code[base_off];
                    if regen_id == 0 {
                        continue; // null reference sentinel, even when T4 has a real row at 0.
                    }
                    // id -> regen ptr. If absent, the id isn't a real func ref (defensive) — skip.
                    let Some(&regen_ptr) = regen.funcid_to_ptr.get(&regen_id) else {
                        continue;
                    };
                    let nptr = remap_ptr(
                        "function-id",
                        ins.op.name,
                        regen_ptr,
                        &regen.func_id_of_ptr,
                        &regen.func_name_of_ptr,
                        &base.func_ptr_of_id,
                    )?;
                    // base ptr -> base id (the operand is the id, not the ptr).
                    let new_id = if base.funcid_to_ptr.get(&regen_id) == Some(&nptr) {
                        regen_id
                    } else {
                        *base
                            .ptr_to_funcid
                            .get(&nptr)
                            .filter(|&&id| id != 0)
                            .ok_or_else(|| RemapError::Unresolved {
                                kind: "function-id(no base id)",
                                op: ins.op.name,
                                key: nptr,
                                name: base
                                    .func_name_of_ptr
                                    .get(&nptr)
                                    .cloned()
                                    .unwrap_or_default(),
                            })?
                    };
                    code[base_off] = new_id;
                    counts.func_id += 1;
                }
                RefKind::TypeId => {
                    let regen_id = code[base_off];
                    // Primitive type-ids (<= LAST_PRIMITIVE) are not in T2 — they resolve to
                    // themselves and need no remap. Skip silently (decompile-refs.md §2.5).
                    // Object-handle qualifiers are operand flags, not part of the T2 key.
                    let (regen_core, flags) = split_type_id_operand(regen_id);
                    let Some(&regen_ptr) = regen.typeid_to_ptr.get(&regen_core) else {
                        continue;
                    };
                    let nptr = remap_ptr(
                        "type-id",
                        ins.op.name,
                        regen_ptr,
                        &regen.type_id_of_ptr,
                        &regen.type_name_of_ptr,
                        &base.type_ptr_of_id,
                    )?;
                    let new_id = if base.typeid_to_ptr.get(&regen_core) == Some(&nptr) {
                        regen_core
                    } else {
                        *base
                            .ptr_kind_to_typeid
                            .get(&(nptr, regen_core as u32 & TYPE_ID_OBJECT_MASK))
                            .ok_or_else(|| RemapError::Unresolved {
                                kind: "type-id(no base id)",
                                op: ins.op.name,
                                key: nptr,
                                name: base
                                    .type_name_of_ptr
                                    .get(&nptr)
                                    .cloned()
                                    .unwrap_or_default(),
                            })?
                    };
                    code[base_off] = apply_type_id_operand_flags(new_id, flags);
                    counts.type_id += 1;
                }
            }
        }
    }
    Ok(counts)
}

fn read_qw(code: &[i32], dw: usize) -> i64 {
    let lo = code[dw] as u32 as u64;
    let hi = code[dw + 1] as u32 as u64;
    (lo | (hi << 32)) as i64
}

fn write_qw(code: &mut [i32], dw: usize, val: i64) {
    let v = val as u64;
    code[dw] = (v & 0xFFFF_FFFF) as u32 as i32;
    code[dw + 1] = ((v >> 32) & 0xFFFF_FFFF) as u32 as i32;
}

// ---------------------------------------------------------------------------------------------
// Module-entry byte walker that records each function's ByteCode TArray byte span, so the
// remapped dwords can be written back in place (the rest of the module entry is copied verbatim).
// ---------------------------------------------------------------------------------------------

/// Byte location of one function's `ByteCode TArray<int32>` DATA (after the count prefix).
struct CodeSpan {
    /// Byte offset of the first bytecode dword (just after the int32 count).
    data_off: usize,
    /// Number of int32 dwords.
    count: usize,
}

/// An embedded module-record int64 reference field (NOT in the bytecode stream): its absolute
/// byte offset + which table it keys. These carry regen ptr/id keys too and must be remapped.
#[derive(Clone, Copy)]
enum EmbedKind {
    TypePtr(TypePtrRule),
    FuncId,
}
#[derive(Clone, Copy)]
enum TypePtrRule {
    Concrete,
    Optional,
    Null,
    Invalid,
}
struct EmbedRef {
    byte_off: usize,
    kind: EmbedKind,
}

#[derive(Clone, Debug)]
struct ModuleFunctionIdSite {
    byte_off: usize,
    identity: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DeclarationIdentity {
    module: String,
    namespace: String,
    name: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PropertyDeclarationIdentity {
    owner: DeclarationIdentity,
    name: String,
}

#[derive(Clone, Debug)]
struct FunctionDeclarationIdentity {
    identity: Ident,
    footprint: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DeclarationSkeleton {
    module: String,
    name: String,
}

impl From<&DeclarationIdentity> for DeclarationSkeleton {
    fn from(identity: &DeclarationIdentity) -> Self {
        Self {
            module: identity.module.clone(),
            name: identity.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PropertyDeclarationSkeleton {
    owner: DeclarationSkeleton,
    name: String,
}

impl From<&PropertyDeclarationIdentity> for PropertyDeclarationSkeleton {
    fn from(identity: &PropertyDeclarationIdentity) -> Self {
        Self {
            owner: DeclarationSkeleton::from(&identity.owner),
            name: identity.name.clone(),
        }
    }
}

fn function_identity_hash(full: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    full.hash(&mut hasher);
    hasher.finish()
}

/// Compact declaration-only inventory. It deliberately excludes function bodies and all other
/// module records; the hard row/string budgets are charged before an entry is retained.
#[derive(Debug, Default)]
struct DeclarationInventory {
    types: HashSet<DeclarationIdentity>,
    type_buckets: HashMap<DeclarationSkeleton, HashSet<String>>,
    functions: Vec<FunctionDeclarationIdentity>,
    function_exact: HashMap<u64, Vec<usize>>,
    function_buckets: HashMap<String, Vec<usize>>,
    globals: HashSet<DeclarationIdentity>,
    global_buckets: HashMap<DeclarationSkeleton, HashSet<String>>,
    properties: HashSet<PropertyDeclarationIdentity>,
    property_buckets: HashMap<PropertyDeclarationSkeleton, HashSet<String>>,
    rows: usize,
    bytes: usize,
}

impl DeclarationInventory {
    fn charge(&mut self, bytes: usize, pos: usize) -> Result<(), WireError> {
        let rows = self.rows.checked_add(1).ok_or(WireError::BadLen {
            pos,
            len: i64::MAX,
            field: "module declaration rows",
        })?;
        if rows > MAX_DECLARATION_AUTHORITY_ROWS {
            return Err(WireError::BadLen {
                pos,
                len: rows as i64,
                field: "module declaration rows",
            });
        }
        let total = self.bytes.checked_add(bytes).ok_or(WireError::BadLen {
            pos,
            len: i64::MAX,
            field: "module declaration bytes",
        })?;
        if total > MAX_DECLARATION_AUTHORITY_BYTES {
            return Err(WireError::BadLen {
                pos,
                len: total as i64,
                field: "module declaration bytes",
            });
        }
        self.rows = rows;
        self.bytes = total;
        Ok(())
    }

    fn insert_type(&mut self, identity: DeclarationIdentity, pos: usize) -> Result<(), WireError> {
        if self.types.contains(&identity) {
            return Err(WireError::BadLen {
                pos,
                len: 2,
                field: "duplicate type declaration",
            });
        }
        let bytes =
            declaration_identity_bytes(&identity).saturating_add(std::mem::size_of::<String>());
        self.charge(bytes, pos)?;
        self.type_buckets
            .entry(DeclarationSkeleton::from(&identity))
            .or_default()
            .insert(identity.namespace.clone());
        self.types.insert(identity);
        Ok(())
    }

    fn insert_global(
        &mut self,
        identity: DeclarationIdentity,
        pos: usize,
    ) -> Result<(), WireError> {
        if self.globals.contains(&identity) {
            return Err(WireError::BadLen {
                pos,
                len: 2,
                field: "duplicate global declaration",
            });
        }
        let bytes =
            declaration_identity_bytes(&identity).saturating_add(std::mem::size_of::<String>());
        self.charge(bytes, pos)?;
        self.global_buckets
            .entry(DeclarationSkeleton::from(&identity))
            .or_default()
            .insert(identity.namespace.clone());
        self.globals.insert(identity);
        Ok(())
    }

    fn insert_function(
        &mut self,
        mut identity: FunctionDeclarationIdentity,
        pos: usize,
    ) -> Result<(), WireError> {
        let exact_hash = function_identity_hash(&identity.identity.full);
        if self.function_exact.get(&exact_hash).is_some_and(|indices| {
            indices
                .iter()
                .any(|&index| self.functions[index].identity.full == identity.identity.full)
        }) {
            return Err(WireError::BadLen {
                pos,
                len: 2,
                field: "duplicate function declaration",
            });
        }
        identity.footprint = identity_footprint(pos as i64, &identity.identity)?;
        // Account for the arena entry and both compact index vectors. The skeleton map owns at
        // most one additional copy of each distinct skeleton; conservatively charging it per row
        // keeps the actual heap below the authority budget without another allocation-heavy pass.
        let bytes = identity
            .footprint
            .saturating_add(identity.identity.ns_stripped.len())
            .saturating_add(2 * std::mem::size_of::<usize>());
        self.charge(bytes, pos)?;
        let index = self.functions.len();
        let skeleton = identity.identity.ns_stripped.clone();
        self.functions.push(identity);
        self.function_exact
            .entry(exact_hash)
            .or_default()
            .push(index);
        self.function_buckets
            .entry(skeleton)
            .or_default()
            .push(index);
        Ok(())
    }

    fn insert_property(
        &mut self,
        identity: PropertyDeclarationIdentity,
        pos: usize,
    ) -> Result<(), WireError> {
        if self.properties.contains(&identity) {
            return Err(WireError::BadLen {
                pos,
                len: 2,
                field: "duplicate property declaration",
            });
        }
        let bytes = declaration_identity_bytes(&identity.owner)
            .checked_add(identity.name.len())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<String>()))
            .ok_or(WireError::BadLen {
                pos,
                len: i64::MAX,
                field: "module declaration bytes",
            })?;
        self.charge(bytes, pos)?;
        self.property_buckets
            .entry(PropertyDeclarationSkeleton::from(&identity))
            .or_default()
            .insert(identity.owner.namespace.clone());
        self.properties.insert(identity);
        Ok(())
    }

    fn merge(&mut self, other: DeclarationInventory, pos: usize) -> Result<(), WireError> {
        for identity in other.types {
            self.insert_type(identity, pos)?;
        }
        for identity in other.functions {
            self.insert_function(identity, pos)?;
        }
        for identity in other.globals {
            self.insert_global(identity, pos)?;
        }
        for identity in other.properties {
            self.insert_property(identity, pos)?;
        }
        Ok(())
    }
}

fn declaration_identity_bytes(identity: &DeclarationIdentity) -> usize {
    identity
        .module
        .len()
        .saturating_add(identity.namespace.len())
        .saturating_add(identity.name.len())
}

/// Everything the byte-walker collects from the single module entry.
#[derive(Default)]
struct ModuleSpans {
    module: String,
    inner_module: String,
    code: Vec<CodeSpan>,
    embeds: Vec<EmbedRef>,
    function_ids: Vec<i32>,
    function_id_sites: Vec<ModuleFunctionIdSite>,
    function_id_identity_bytes: usize,
    capture_function_identities: bool,
    declarations: Option<DeclarationInventory>,
    structural_violation: Option<(&'static str, String)>,
}

struct DeclarationTypeContext<'a> {
    primary: &'a HashMap<i64, Ident>,
    fallback: Option<&'a HashMap<i64, Ident>>,
    script_owners: ScriptOwnerIndex,
    fallback_script_owners: Option<&'a ScriptOwnerIndex>,
}

impl DeclarationTypeContext<'_> {
    fn datatype(&self, key: i64, datatype: ModuleDataType) -> Result<Ident, WireError> {
        let nested = if datatype.token == 5 {
            self.primary
                .get(&datatype.type_info)
                .or_else(|| {
                    self.fallback
                        .and_then(|fallback| fallback.get(&datatype.type_info))
                })
                .cloned()
                .unwrap_or_default()
        } else {
            Ident::default()
        };
        datatype_identity(
            key,
            datatype.flags,
            datatype.token,
            datatype.type_info,
            &nested,
        )
    }

    fn object_type(&self, key: i64, datatype: ModuleDataType) -> Result<Ident, WireError> {
        // Validate the complete return DataType first, then project exactly GetTypeInfo(): factory
        // T3 ObjectType is the nested T1 identity, not the flags+token DataType identity.
        self.datatype(key, datatype)?;
        if datatype.token != 5 || datatype.flags[4] || datatype.type_info == 0 {
            return Err(WireError::InvalidDataType {
                key,
                detail: "factory return requires a concrete object TypeReference",
            });
        }
        self.primary
            .get(&datatype.type_info)
            .or_else(|| {
                self.fallback
                    .and_then(|fallback| fallback.get(&datatype.type_info))
            })
            .cloned()
            .ok_or(WireError::InvalidDataType {
                key,
                detail: "factory return TypeReference is unresolved",
            })
    }

    fn script_owner(
        &self,
        declaration: &DeclarationIdentity,
        comparison_budget: &mut IdentityComparisonBudget,
    ) -> Result<Option<&Ident>, WireError> {
        if let Some(identity) = self.script_owners.exact(declaration) {
            return Ok(Some(identity));
        }
        if let Some(identity) = self
            .fallback_script_owners
            .and_then(|fallback| fallback.exact(declaration))
        {
            return Ok(Some(identity));
        }
        let skeleton = DeclarationSkeleton::from(declaration);
        let indices = [Some(&self.script_owners), self.fallback_script_owners];
        let candidates = indices
            .into_iter()
            .flatten()
            .flat_map(|index| index.buckets.get(&skeleton).into_iter())
            .flat_map(HashMap::keys)
            .map(String::as_str);
        let matched =
            match match_unique_namespace(&declaration.namespace, candidates, comparison_budget)? {
                NamespaceMatch::Missing => return Ok(None),
                NamespaceMatch::Ambiguous => {
                    return Err(WireError::BadLen {
                        pos: 0,
                        len: 2,
                        field: "ambiguous script class declaration",
                    });
                }
                NamespaceMatch::Unique(namespace) => namespace,
            };
        for index in [Some(&self.script_owners), self.fallback_script_owners]
            .into_iter()
            .flatten()
        {
            if let Some(identity) = index
                .buckets
                .get(&skeleton)
                .and_then(|bucket| bucket.get(matched))
            {
                return Ok(Some(identity));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Copy)]
enum FunctionDeclarationScope<'a> {
    None,
    Global,
    Method(&'a Ident),
}

struct FunctionDeclarationRecord {
    identity: FunctionDeclarationIdentity,
    pos: usize,
}

fn insert_function_declaration(
    out: &mut ModuleSpans,
    record: Option<FunctionDeclarationRecord>,
) -> Result<(), WireError> {
    if let (Some(declarations), Some(record)) = (out.declarations.as_mut(), record) {
        declarations.insert_function(record.identity, record.pos)?;
    }
    Ok(())
}

fn record_structural_violation(out: &mut ModuleSpans, field: &'static str, detail: String) {
    if out.structural_violation.is_none() {
        out.structural_violation = Some((field, detail));
    }
}

fn require_equal_counts(
    out: &mut ModuleSpans,
    field: &'static str,
    left_name: &'static str,
    left: usize,
    right_name: &'static str,
    right: usize,
) {
    if left != right {
        record_structural_violation(
            out,
            field,
            format!("{left_name} has {left} entries but {right_name} has {right}"),
        );
    }
}

fn validate_collected_module_structure(spans: &ModuleSpans) -> Result<(), RemapError> {
    if let Some((field, detail)) = spans.structural_violation.as_ref() {
        return Err(RemapError::InvalidModuleStructure {
            module: spans.module.clone(),
            field,
            detail: detail.clone(),
        });
    }
    let mut ids = HashSet::new();
    for &id in &spans.function_ids {
        if !ids.insert(id) {
            return Err(RemapError::FunctionIdCollision {
                id,
                first_module: spans.module.clone(),
                second_module: spans.module.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug)]
struct FinalTypeDeclarationQuery {
    row_key: i64,
    descriptor: TypeDeclarationDescriptor,
}

#[derive(Debug)]
struct FinalGlobalDeclarationQuery {
    row_key: i64,
    identity: DeclarationIdentity,
}

#[derive(Debug)]
struct FinalPropertyDeclarationQuery {
    row_key: i64,
    name: String,
    old_type_id: i32,
}

#[derive(Debug, Default)]
struct FinalDeclarationQueries {
    types: Vec<FinalTypeDeclarationQuery>,
    globals: Vec<FinalGlobalDeclarationQuery>,
    properties: Vec<FinalPropertyDeclarationQuery>,
    type_ids: HashMap<i32, i64>,
    rows: usize,
    bytes: usize,
}

impl FinalDeclarationQueries {
    fn charge_rows(&mut self, count: usize, pos: usize) -> Result<(), WireError> {
        let rows = self.rows.checked_add(count).ok_or(WireError::BadLen {
            pos,
            len: i64::MAX,
            field: "declaration query rows",
        })?;
        if rows > MAX_DECLARATION_QUERY_ROWS {
            return Err(WireError::BadLen {
                pos,
                len: rows as i64,
                field: "declaration query rows",
            });
        }
        self.rows = rows;
        Ok(())
    }

    fn charge_bytes(&mut self, count: usize, pos: usize) -> Result<(), WireError> {
        let bytes = self.bytes.checked_add(count).ok_or(WireError::BadLen {
            pos,
            len: i64::MAX,
            field: "declaration query bytes",
        })?;
        if bytes > MAX_DECLARATION_QUERY_BYTES {
            return Err(WireError::BadLen {
                pos,
                len: bytes as i64,
                field: "declaration query bytes",
            });
        }
        self.bytes = bytes;
        Ok(())
    }

    fn read_sia(&mut self, c: &mut Cursor) -> Result<String, WireError> {
        let pos = c.pos();
        let raw = c.read_sia_bytes()?;
        self.charge_bytes(raw.len(), pos)?;
        Ok(raw.decode_ansi())
    }

    fn skip_fixed(
        c: &mut Cursor,
        count: usize,
        width: usize,
        field: &'static str,
    ) -> Result<(), WireError> {
        c.ensure_minimum_remaining(count, width, field)?;
        c.skip(count.checked_mul(width).ok_or(WireError::BadLen {
            pos: c.pos(),
            len: count as i64,
            field,
        })?)
    }

    /// Read only the T1/T5/T7 fields needed for declaration membership. T2 is revisited in a
    /// second pass and retains mappings only for the bounded set of T7 owner ids.
    fn build(bytes: &[u8]) -> Result<Self, RemapError> {
        let tail = preflight_tail_tables(bytes)?.tail;
        let mut c = Cursor::at(bytes, tail);
        let mut out = Self::default();

        let ntypes = c.read_count("TypeReferences")?;
        out.charge_rows(ntypes, c.pos().saturating_sub(4))?;
        c.ensure_minimum_remaining(ntypes, 24, "TypeReferences")?;
        for _ in 0..ntypes {
            let row_key = c.read_i64()?;
            let name = out.read_sia(&mut c)?;
            let module = out.read_sia(&mut c)?;
            let namespace = out.read_sia(&mut c)?;
            let nsub = c.read_count("TypeRef.SubTypes")?;
            Self::skip_fixed(&mut c, nsub, DATA_TYPE_SIZE, "TypeRef.SubTypes")?;
            let kind = if nsub != 0 {
                TypeDeclarationKind::Template
            } else if module == "$__T__" {
                TypeDeclarationKind::TemplateSentinel
            } else if module.is_empty() {
                TypeDeclarationKind::EngineLeaf
            } else {
                TypeDeclarationKind::ScriptLeaf
            };
            out.types.push(FinalTypeDeclarationQuery {
                row_key,
                descriptor: TypeDeclarationDescriptor {
                    identity: DeclarationIdentity {
                        module,
                        namespace,
                        name,
                    },
                    kind,
                },
            });
        }

        let type_ids_pos = c.pos();
        let ntypeids = c.read_count("TypeIdRef")?;
        Self::skip_fixed(&mut c, ntypeids, 12, "TypeIdRef")?;

        let nfuncs = c.read_count("FunctionReferences")?;
        c.ensure_minimum_remaining(nfuncs, 76, "FunctionReferences")?;
        for _ in 0..nfuncs {
            c.skip(8)?;
            c.read_sia_bytes()?;
            c.read_sia_bytes()?;
            c.read_sia_bytes()?;
            c.skip(3 * 4 + 8)?;
            let nparams = c.read_count("FuncRef.Params")?;
            Self::skip_fixed(&mut c, nparams, DATA_TYPE_SIZE, "FuncRef.Params")?;
            c.skip(DATA_TYPE_SIZE)?;
        }

        let nfuncids = c.read_count("FuncIdRef")?;
        Self::skip_fixed(&mut c, nfuncids, 12, "FuncIdRef")?;

        let nglobals = c.read_count("GlobalReferences")?;
        out.charge_rows(nglobals, c.pos().saturating_sub(4))?;
        c.ensure_minimum_remaining(nglobals, 24, "GlobalReferences")?;
        for _ in 0..nglobals {
            let row_key = c.read_i64()?;
            let name_pos = c.pos();
            let name = c.read_sia_bytes()?;
            out.charge_bytes(name.len(), name_pos)?;
            let module = out.read_sia(&mut c)?;
            let namespace = out.read_sia(&mut c)?;
            let is_string = c.read_bool4()?;
            if is_string {
                name.decode_utf8(name_pos)?;
            } else if !module.is_empty() {
                out.globals.push(FinalGlobalDeclarationQuery {
                    row_key,
                    identity: DeclarationIdentity {
                        module,
                        namespace,
                        name: name.decode_ansi(),
                    },
                });
            }
        }

        let nstatic = c.read_count("StaticNames")?;
        for _ in 0..nstatic {
            c.read_sia_bytes()?;
        }

        let nproperties = c.read_count("PropertyReferences")?;
        out.charge_rows(nproperties, c.pos().saturating_sub(4))?;
        c.ensure_minimum_remaining(nproperties, 16, "PropertyReferences")?;
        for _ in 0..nproperties {
            let row_key = c.read_i64()?;
            let name = out.read_sia(&mut c)?;
            let old_type_id = c.read_i32()?;
            out.properties.push(FinalPropertyDeclarationQuery {
                row_key,
                name,
                old_type_id,
            });
        }
        if c.pos() != bytes.len() {
            return Err(RemapError::TailNotAtEof {
                got: c.pos(),
                len: bytes.len(),
            });
        }

        let wanted_owner_ids: HashSet<i32> =
            out.properties.iter().map(|row| row.old_type_id).collect();
        let mut type_ids = Cursor::at(bytes, type_ids_pos);
        let count = type_ids.read_count("TypeIdRef")?;
        for _ in 0..count {
            let id = type_ids.read_i32()?;
            let ptr = type_ids.read_i64()?;
            if wanted_owner_ids.contains(&id) {
                out.type_ids.insert(id, ptr);
            }
        }
        Ok(out)
    }

    fn validate(
        self,
        declarations: &DeclarationInventory,
        syms: &SymTables,
        pristine: Option<&PristineDeclarationAuthority>,
        comparison_budget: &mut IdentityComparisonBudget,
    ) -> Result<(), RemapError> {
        let type_by_ptr: HashMap<i64, &TypeDeclarationDescriptor> = self
            .types
            .iter()
            .map(|row| (row.row_key, &row.descriptor))
            .collect();
        for row in &self.types {
            if row.descriptor.kind == TypeDeclarationKind::ScriptLeaf
                && match_declaration_identities(
                    &[declarations],
                    &row.descriptor.identity,
                    DeclarationSetKind::Type,
                    comparison_budget,
                )? != FunctionDeclarationMatch::Unique
            {
                return Err(missing_declaration_membership(
                    0,
                    row.row_key,
                    format!(
                        "final module output has no Class or Enum for {}::{} in module {:?}",
                        row.descriptor.identity.namespace,
                        row.descriptor.identity.name,
                        row.descriptor.identity.module
                    ),
                ));
            }
        }
        for row in &self.globals {
            if match_declaration_identities(
                &[declarations],
                &row.identity,
                DeclarationSetKind::Global,
                comparison_budget,
            )? != FunctionDeclarationMatch::Unique
            {
                return Err(missing_declaration_membership(
                    4,
                    row.row_key,
                    format!(
                        "final module output has no GlobalVariables declaration for {}::{} in module {:?}",
                        row.identity.namespace, row.identity.name, row.identity.module
                    ),
                ));
            }
        }
        if let Some(pristine) = pristine {
            for (&row_key, identity) in &syms.func_ident_of_ptr {
                let declaration_match =
                    match_function_declarations(&[declarations], identity, comparison_budget)?;
                if declaration_match == FunctionDeclarationMatch::Unique
                    || (declaration_match == FunctionDeclarationMatch::Missing
                        && pristine.orphan_functions.contains(&row_key))
                {
                    continue;
                }
                return Err(missing_declaration_membership(
                    2,
                    row_key,
                    format!(
                        "final module output has no exact function record for runtime signature {}",
                        identity.display
                    ),
                ));
            }
        }
        for row in &self.properties {
            let Some(owner_ptr) = self.type_ids.get(&row.old_type_id) else {
                return Err(missing_declaration_membership(
                    6,
                    row.row_key,
                    format!(
                        "final property owner type id {:#x} is unresolved",
                        row.old_type_id
                    ),
                ));
            };
            let Some(owner) = type_by_ptr.get(owner_ptr) else {
                return Err(missing_declaration_membership(
                    6,
                    row.row_key,
                    format!("final property owner pointer {owner_ptr:#x} has no T1 row"),
                ));
            };
            if owner.kind != TypeDeclarationKind::ScriptLeaf {
                continue;
            }
            let property = PropertyDeclarationIdentity {
                owner: owner.identity.clone(),
                name: row.name.clone(),
            };
            if match_property_declarations(&[declarations], &property, comparison_budget)?
                != FunctionDeclarationMatch::Unique
            {
                return Err(missing_declaration_membership(
                    6,
                    row.row_key,
                    format!("final declaring class has no property named {:?}", row.name),
                ));
            }
        }
        Ok(())
    }
}

/// Validate the runtime-indexed records of a fully composed cache before any caller can publish
/// it. Doing this on the prospective output (rather than on a mini in isolation) naturally lets
/// an edit reuse ids owned by the module it replaced, while still rejecting collisions with every
/// untouched or previously composed module.
fn validate_composed_module_records_with_pristine(
    bytes: &[u8],
    pristine: Option<&PristineDeclarationAuthority>,
) -> Result<(), RemapError> {
    if bytes.len() < CacheHeader::SIZE {
        return Err(WireError::Eof {
            pos: 0,
            need: CacheHeader::SIZE,
            have: bytes.len(),
        }
        .into());
    }
    preflight_cache_module_work(bytes)?;
    let declaration_queries = FinalDeclarationQueries::build(bytes)?;
    let syms = SymTables::build(bytes)?;
    let meta = TailMetadata::build(bytes)?;
    let mut comparison_budget = IdentityComparisonBudget::new(
        bytes
            .len()
            .saturating_add(syms.identity_bytes)
            .saturating_add(declaration_queries.bytes),
    );
    let declaration_types = declaration_type_context(&meta, &syms, None, None)?;
    let mut cursor = Cursor::at(bytes, CacheHeader::SIZE);
    let count = super::walk_modules::module_count(bytes) as usize;
    cursor.ensure_minimum_remaining(count, 60, "Modules")?;
    let mut function_ids = HashMap::<i32, String>::new();
    let mut outer_module_names = HashSet::new();
    let mut inner_module_names = HashSet::new();
    let mut declarations = DeclarationInventory::default();
    for _ in 0..count {
        let module = cursor.read_fstring()?;
        if !outer_module_names.insert(module.clone()) {
            return Err(RemapError::ModuleNameCollision { name: module });
        }
        let mut spans = ModuleSpans {
            module: module.clone(),
            declarations: Some(DeclarationInventory::default()),
            ..ModuleSpans::default()
        };
        read_module_spans(
            &mut cursor,
            bytes,
            &mut spans,
            Some(&declaration_types),
            Some(&mut comparison_budget),
        )?;
        if !inner_module_names.insert(spans.inner_module.clone()) {
            return Err(RemapError::ModuleNameCollision {
                name: spans.inner_module,
            });
        }
        validate_collected_module_structure(&spans)?;
        declarations.merge(spans.declarations.take().unwrap_or_default(), cursor.pos())?;
        for id in spans.function_ids {
            if let Some(first_module) = function_ids.insert(id, module.clone()) {
                return Err(RemapError::FunctionIdCollision {
                    id,
                    first_module,
                    second_module: module,
                });
            }
        }
    }
    declaration_queries.validate(&declarations, &syms, pristine, &mut comparison_budget)?;
    Ok(())
}

pub(super) fn validate_composed_module_records(bytes: &[u8]) -> Result<(), RemapError> {
    // Low-level splice helpers have no generation-bound pristine authority. They still validate
    // structural records and T1/T5/T7 declaration closure; the publishing SequentialMiniGuard
    // performs the additional exact T3/orphan-aware pass before committing its staged state.
    validate_composed_module_records_with_pristine(bytes, None)
}

/// Walk the single module entry in `mini` (TMap key + module value) and collect every
/// function's ByteCode span + every embedded int64 ref. Mirrors `walk_modules::read_module_c`
/// but records byte offsets.
fn collect_module_spans(mini: &[u8]) -> Result<ModuleSpans, WireError> {
    preflight_mini_module_work(mini)?;
    let mut c = Cursor::at(mini, CacheHeader::SIZE);
    let module = c.read_fstring()?; // TMap key
    let mut spans = ModuleSpans {
        module,
        ..ModuleSpans::default()
    };
    read_module_spans(&mut c, mini, &mut spans, None, None)?;
    Ok(spans)
}

/// Stream each module independently and retain only its bounded declaration inventory. The
/// detailed walker remains the single format oracle; callers run the allocation-light work
/// preflight before this helper so its transient span vectors are bounded as well.
fn declaration_type_context<'a>(
    meta: &TailMetadata,
    primary: &'a SymTables,
    fallback: Option<&'a SymTables>,
    fallback_script_owners: Option<&'a ScriptOwnerIndex>,
) -> Result<DeclarationTypeContext<'a>, WireError> {
    let mut script_owners = ScriptOwnerIndex::default();
    for row in &meta.types {
        let descriptor = type_declaration_descriptor(row);
        if descriptor.kind != TypeDeclarationKind::ScriptLeaf {
            continue;
        }
        if let Some(identity) = primary
            .type_ident_of_ptr
            .get(&row.key)
            .or_else(|| fallback.and_then(|fallback| fallback.type_ident_of_ptr.get(&row.key)))
        {
            script_owners.insert(&descriptor.identity, identity, row.start)?;
        }
    }
    Ok(DeclarationTypeContext {
        primary: &primary.type_ident_of_ptr,
        fallback: fallback.map(|fallback| &fallback.type_ident_of_ptr),
        script_owners,
        fallback_script_owners,
    })
}

struct CollectedDeclarationInventory {
    declarations: DeclarationInventory,
    function_id_sites: Vec<ModuleFunctionIdSite>,
}

fn collect_declaration_inventory(
    bytes: &[u8],
    primary: &SymTables,
    fallback: Option<&SymTables>,
    fallback_script_owners: Option<&ScriptOwnerIndex>,
    meta: &TailMetadata,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<CollectedDeclarationInventory, WireError> {
    if bytes.len() < CacheHeader::SIZE {
        return Err(WireError::Eof {
            pos: 0,
            need: CacheHeader::SIZE,
            have: bytes.len(),
        });
    }
    let mut cursor = Cursor::at(bytes, CacheHeader::SIZE);
    let declaration_types =
        declaration_type_context(meta, primary, fallback, fallback_script_owners)?;
    let count = super::walk_modules::module_count(bytes) as usize;
    cursor.ensure_minimum_remaining(count, 60, "Modules")?;
    let mut inventory = DeclarationInventory::default();
    let mut function_id_sites = Vec::new();
    let mut function_id_identity_bytes = 0usize;
    for _ in 0..count {
        let module = cursor.read_fstring()?;
        let mut spans = ModuleSpans {
            module,
            declarations: Some(DeclarationInventory::default()),
            capture_function_identities: true,
            ..ModuleSpans::default()
        };
        read_module_spans(
            &mut cursor,
            bytes,
            &mut spans,
            Some(&declaration_types),
            Some(&mut *comparison_budget),
        )?;
        inventory.merge(spans.declarations.take().unwrap_or_default(), cursor.pos())?;
        function_id_identity_bytes = function_id_identity_bytes
            .checked_add(spans.function_id_identity_bytes)
            .filter(|&bytes| bytes <= MAX_DECLARATION_AUTHORITY_BYTES)
            .ok_or(WireError::BadLen {
                pos: cursor.pos(),
                len: i64::MAX,
                field: "module Function.Id identity bytes",
            })?;
        function_id_sites.extend(spans.function_id_sites);
    }
    Ok(CollectedDeclarationInventory {
        declarations: inventory,
        function_id_sites,
    })
}

/// Allocation-light summary produced before the detailed module walker constructs any bytecode,
/// embed, object-position, variable-info, or record vectors.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ModuleWorkSummary {
    pub(super) function_count: usize,
    pub(super) max_function_bytecode_dwords: usize,
    pub(super) total_bytecode_dwords: u64,
    pub(super) work_items: usize,
    pub(super) module_authority_bytes: usize,
}

struct ModuleWorkBudget {
    summary: ModuleWorkSummary,
    max_functions: usize,
    max_work_items: usize,
}

impl ModuleWorkBudget {
    fn charge(&mut self, count: usize, pos: usize, field: &'static str) -> Result<(), WireError> {
        let total = self
            .summary
            .work_items
            .checked_add(count)
            .ok_or(WireError::BadLen {
                pos,
                len: i64::MAX,
                field,
            })?;
        if total > self.max_work_items {
            return Err(WireError::BadLen {
                pos,
                len: total as i64,
                field: "module work items",
            });
        }
        self.summary.work_items = total;
        Ok(())
    }

    fn function(&mut self, pos: usize) -> Result<(), WireError> {
        let count = self
            .summary
            .function_count
            .checked_add(1)
            .ok_or(WireError::BadLen {
                pos,
                len: i64::MAX,
                field: "module function records",
            })?;
        if count > self.max_functions {
            return Err(WireError::BadLen {
                pos,
                len: count as i64,
                field: "module function records",
            });
        }
        self.summary.function_count = count;
        self.charge(1, pos, "module function records")
    }

    fn bytecode(&mut self, count: usize, pos: usize) -> Result<(), WireError> {
        self.summary.max_function_bytecode_dwords =
            self.summary.max_function_bytecode_dwords.max(count);
        self.summary.total_bytecode_dwords = self
            .summary
            .total_bytecode_dwords
            .checked_add(count as u64)
            .ok_or(WireError::BadLen {
                pos,
                len: i64::MAX,
                field: "total ByteCode dwords",
            })?;
        self.charge(count, pos, "ByteCode")
    }

    fn module_authority_names(
        &mut self,
        outer_bytes: usize,
        inner_bytes: usize,
        pos: usize,
    ) -> Result<(), WireError> {
        let total = self
            .summary
            .module_authority_bytes
            .checked_add(outer_bytes)
            .and_then(|bytes| bytes.checked_add(inner_bytes))
            .ok_or(WireError::BadLen {
                pos,
                len: i64::MAX,
                field: "module authority name bytes",
            })?;
        if total > MAX_DECLARATION_AUTHORITY_BYTES {
            return Err(WireError::BadLen {
                pos,
                len: total as i64,
                field: "module authority name bytes",
            });
        }
        self.summary.module_authority_bytes = total;
        Ok(())
    }
}

fn module_work_count(
    c: &mut Cursor,
    budget: &mut ModuleWorkBudget,
    field: &'static str,
    minimum_width: usize,
) -> Result<usize, WireError> {
    let pos = c.pos();
    let count = c.read_count(field)?;
    c.ensure_minimum_remaining(count, minimum_width, field)?;
    budget.charge(count, pos, field)?;
    Ok(count)
}

fn module_work_skip_fixed(
    c: &mut Cursor,
    budget: &mut ModuleWorkBudget,
    field: &'static str,
    width: usize,
) -> Result<usize, WireError> {
    let count = module_work_count(c, budget, field, width)?;
    c.skip(count.checked_mul(width).ok_or(WireError::BadLen {
        pos: c.pos(),
        len: count as i64,
        field,
    })?)?;
    Ok(count)
}

fn module_work_skip_sia_array(
    c: &mut Cursor,
    budget: &mut ModuleWorkBudget,
    field: &'static str,
) -> Result<usize, WireError> {
    let count = module_work_count(c, budget, field, 4)?;
    for _ in 0..count {
        c.read_sia_bytes()?;
    }
    Ok(count)
}

fn module_work_function(c: &mut Cursor, budget: &mut ModuleWorkBudget) -> Result<(), WireError> {
    budget.function(c.pos())?;
    c.read_sia_bytes()?; // Name
    c.read_sia_bytes()?; // Namespace
    c.skip(DATA_TYPE_SIZE)?; // ReturnType
    module_work_skip_fixed(c, budget, "ParameterTypes", DATA_TYPE_SIZE)?;
    module_work_skip_sia_array(c, budget, "ParameterNames")?;
    module_work_skip_fixed(c, budget, "ParameterFlags", 4)?;
    module_work_skip_sia_array(c, budget, "ParameterDefaultArgs")?;
    c.skip(4)?; // FunctionTraits
    let bytecode_pos = c.pos();
    let bytecode = c.read_count("ByteCode")?;
    c.ensure_minimum_remaining(bytecode, 4, "ByteCode")?;
    budget.bytecode(bytecode, bytecode_pos)?;
    c.skip(bytecode.checked_mul(4).ok_or(WireError::BadLen {
        pos: bytecode_pos,
        len: bytecode as i64,
        field: "ByteCode",
    })?)?;
    module_work_skip_fixed(c, budget, "ByteCodeReferences", 4)?;
    c.skip(4)?; // VariableSpace
    module_work_skip_fixed(c, budget, "ObjVariableTypes", 8)?;
    module_work_skip_fixed(c, budget, "ObjVariablePos", 4)?;
    c.skip(4)?; // ObjVariablesOnHeap
    module_work_skip_fixed(c, budget, "VariableInfoProgramPos", 4)?;
    module_work_skip_fixed(c, budget, "VariableInfoOffset", 4)?;
    module_work_skip_fixed(c, budget, "VariableInfoOption", 4)?;
    c.skip(4 * 3)?; // StackNeeded + Id + DeclaredAt
    module_work_skip_fixed(c, budget, "LineNumbers", 4)?;
    if c.read_bool4()? {
        c.read_sia_bytes()?; // UnrealFunctionName
        module_work_skip_sia_array(c, budget, "UF.MetaSpec")?;
        module_work_skip_sia_array(c, budget, "UF.MetaValues")?;
        c.skip(18 * 4)?;
    }
    Ok(())
}

fn module_work_property(c: &mut Cursor, budget: &mut ModuleWorkBudget) -> Result<(), WireError> {
    c.read_sia_bytes()?;
    c.skip(DATA_TYPE_SIZE + 2 * 4)?;
    if c.read_bool4()? {
        module_work_skip_sia_array(c, budget, "UP.MetaSpec")?;
        module_work_skip_sia_array(c, budget, "UP.MetaValues")?;
        c.skip(9 * 4)?;
        let replicated = c.read_bool4()?;
        c.skip(3 * 4)?;
        if replicated {
            c.skip(2 * 4)?;
        }
        c.skip(3 * 4)?;
    }
    Ok(())
}

fn module_work_class(c: &mut Cursor, budget: &mut ModuleWorkBudget) -> Result<(), WireError> {
    c.read_sia_bytes()?;
    c.read_sia_bytes()?;
    c.skip(4)?;
    let properties = module_work_count(c, budget, "Class.Properties", 52)?;
    for _ in 0..properties {
        module_work_property(c, budget)?;
    }
    let methods = module_work_count(c, budget, "Class.Methods", 120)?;
    for _ in 0..methods {
        module_work_function(c, budget)?;
    }
    module_work_skip_fixed(c, budget, "Class.MethodTable", 4)?;
    c.skip(16)?;
    let constructors = module_work_count(c, budget, "Class.Constructors", 120)?;
    for _ in 0..constructors {
        module_work_function(c, budget)?;
    }
    module_work_skip_fixed(c, budget, "Class.FactoryRefs", 8)?;
    module_work_skip_fixed(c, budget, "Class.BehaviorRefs", 8)?;
    let behaviors = module_work_count(c, budget, "Class.BehaviorFunctions", 120)?;
    for _ in 0..behaviors {
        module_work_function(c, budget)?;
    }
    module_work_skip_fixed(c, budget, "Class.BehaviorFunctionTypes", 4)?;
    if c.read_bool4()? {
        c.read_sia_bytes()?;
        c.read_sia_bytes()?;
        c.skip(8 * 4)?;
        c.read_sia_bytes()?;
        c.skip(4)?;
        module_work_skip_sia_array(c, budget, "Class.MetaSpec")?;
        module_work_skip_sia_array(c, budget, "Class.MetaValues")?;
        c.read_sia_bytes()?;
    }
    Ok(())
}

fn module_work_enum(c: &mut Cursor, budget: &mut ModuleWorkBudget) -> Result<(), WireError> {
    c.read_sia_bytes()?;
    c.read_sia_bytes()?;
    module_work_skip_sia_array(c, budget, "Enum.Names")?;
    module_work_skip_fixed(c, budget, "Enum.Values", 4)?;
    Ok(())
}

fn module_work_global(c: &mut Cursor, budget: &mut ModuleWorkBudget) -> Result<(), WireError> {
    c.read_sia_bytes()?;
    c.read_sia_bytes()?;
    c.skip(DATA_TYPE_SIZE)?;
    if !c.read_bool4()? {
        if c.read_bool4()? {
            c.skip(8)?;
        } else if c.read_bool4()? {
            module_work_function(c, budget)?;
        }
    }
    Ok(())
}

fn module_work_import(c: &mut Cursor, budget: &mut ModuleWorkBudget) -> Result<(), WireError> {
    c.read_sia_bytes()?;
    c.read_sia_bytes()?;
    c.read_sia_bytes()?;
    module_work_skip_fixed(c, budget, "Import.ParameterTypes", DATA_TYPE_SIZE)?;
    module_work_skip_fixed(c, budget, "Import.ParameterFlags", 4)?;
    module_work_skip_sia_array(c, budget, "Import.ParameterDefaultArgs")?;
    c.skip(DATA_TYPE_SIZE)?;
    Ok(())
}

fn stream_module_work_with_limits(
    bytes: &[u8],
    max_functions: usize,
    max_work_items: usize,
) -> Result<ModuleWorkSummary, WireError> {
    if bytes.len() < CacheHeader::SIZE {
        return Err(WireError::Eof {
            pos: 0,
            need: CacheHeader::SIZE,
            have: bytes.len(),
        });
    }
    let mut c = Cursor::at(bytes, CacheHeader::SIZE);
    let modules = super::walk_modules::module_count(bytes) as usize;
    c.ensure_minimum_remaining(modules, 60, "Modules")?;
    if modules > MAX_DECLARATION_AUTHORITY_ROWS {
        return Err(WireError::BadLen {
            pos: 0x14,
            len: modules as i64,
            field: "module authority Modules",
        });
    }
    let mut budget = ModuleWorkBudget {
        summary: ModuleWorkSummary::default(),
        max_functions,
        max_work_items,
    };
    budget.charge(modules, c.pos(), "Modules")?;
    for _ in 0..modules {
        let name_pos = c.pos();
        let outer = c.read_fstring()?;
        let inner = c.read_sia_bytes()?;
        budget.module_authority_names(outer.len(), inner.len(), name_pos)?;
        let functions = module_work_count(&mut c, &mut budget, "Module.Functions", 120)?;
        for _ in 0..functions {
            module_work_function(&mut c, &mut budget)?;
        }
        let classes = module_work_count(&mut c, &mut budget, "Module.Classes", 64)?;
        for _ in 0..classes {
            module_work_class(&mut c, &mut budget)?;
        }
        let enums = module_work_count(&mut c, &mut budget, "Module.Enums", 16)?;
        for _ in 0..enums {
            module_work_enum(&mut c, &mut budget)?;
        }
        let globals = module_work_count(&mut c, &mut budget, "Module.GlobalVariables", 48)?;
        for _ in 0..globals {
            module_work_global(&mut c, &mut budget)?;
        }
        let imports = module_work_count(&mut c, &mut budget, "Module.FunctionImports", 60)?;
        for _ in 0..imports {
            module_work_import(&mut c, &mut budget)?;
        }
        c.skip(8)?;
        module_work_skip_sia_array(&mut c, &mut budget, "Module.ImportedModules")?;
        c.read_sia_bytes()?;
        module_work_skip_sia_array(&mut c, &mut budget, "Module.DeclaredEvents")?;
        module_work_skip_sia_array(&mut c, &mut budget, "Module.DeclaredDelegates")?;
        c.read_sia_bytes()?;
        module_work_skip_sia_array(&mut c, &mut budget, "Module.PostInitFunctions")?;
    }
    Ok(budget.summary)
}

/// Scan a prepared one-module mini without materializing any record arrays. The sequential guard
/// uses the returned maxima/totals to apply its public SpliceError bytecode limits before
/// reference validation; this helper itself enforces the fixed function/work-allocation caps.
pub(super) fn preflight_mini_module_work(bytes: &[u8]) -> Result<ModuleWorkSummary, WireError> {
    stream_module_work_with_limits(
        bytes,
        MAX_MINI_STREAMED_FUNCTIONS,
        MAX_MINI_MODULE_WORK_ITEMS,
    )
}

pub(super) fn preflight_cache_module_work(bytes: &[u8]) -> Result<ModuleWorkSummary, WireError> {
    stream_module_work_with_limits(
        bytes,
        MAX_DECLARATION_AUTHORITY_ROWS,
        MAX_CACHE_MODULE_WORK_ITEMS,
    )
}

/// A `FAngelscriptPrecompiledDataType` (36 B): 6×bool(24) + int64 TypeInfo.OldReference(+24) +
/// int32 Token(+32). An identifier DataType must carry a concrete T1 pointer; every primitive
/// DataType must carry the exact null sentinel. Record that distinction for final validation.
/// CONFIRMED: container-splice.md §0/§3 (void ReturnType has TypeInfo.Old=0). This is the field
/// that carried the surviving regen-keys — every DataType in function signatures / property /
/// global / import records embeds one, and the prior remap skipped them wholesale.
#[derive(Clone, Copy)]
struct ModuleDataType {
    flags: [bool; 6],
    type_info: i64,
    token: i32,
}

fn embed_datatype(c: &mut Cursor, out: &mut ModuleSpans) -> Result<ModuleDataType, WireError> {
    let dt_start = c.pos();
    let mut flags = [false; 6];
    for flag in &mut flags {
        *flag = c.read_bool4()?;
    }
    let type_info = c.read_i64()?;
    let token = c.read_i32()?;
    let is_auto = flags[4];
    let rule = match (is_auto, token) {
        (true, 5) => TypePtrRule::Null,
        (true, _) => TypePtrRule::Invalid,
        (false, 5) => TypePtrRule::Concrete,
        (
            false,
            0x3b | 0x41 | 0x44 | 0x45 | 0x46 | 0x47 | 0x4b | 0x4c | 0x4d | 0x4e | 0x50 | 0x51
            | 0x52,
        ) => TypePtrRule::Null,
        _ => TypePtrRule::Invalid,
    };
    out.embeds.push(EmbedRef {
        byte_off: dt_start + 24,
        kind: EmbedKind::TypePtr(rule),
    });
    Ok(ModuleDataType {
        flags,
        type_info,
        token,
    })
}

fn read_function_spans(
    c: &mut Cursor,
    bytes: &[u8],
    out: &mut ModuleSpans,
    declaration_types: Option<&DeclarationTypeContext<'_>>,
    declaration_scope: FunctionDeclarationScope<'_>,
) -> Result<Option<FunctionDeclarationRecord>, WireError> {
    let declaration_pos = c.pos();
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    let return_type = embed_datatype(c, out)?;
    let nptypes = c.read_count("ParameterTypes")?;
    let mut parameter_types = Vec::with_capacity(nptypes);
    for _ in 0..nptypes {
        parameter_types.push(embed_datatype(c, out)?);
    }
    let npnames = c.read_count("ParameterNames")?;
    for _ in 0..npnames {
        c.read_sia_bytes()?;
    }
    let npflags = c.read_count("ParameterFlags")?;
    c.skip(npflags * 4)?;
    let npdefaults = c.read_count("ParameterDefaultArgs")?;
    for _ in 0..npdefaults {
        c.read_sia()?;
    }
    require_equal_counts(
        out,
        "Function.Parameters",
        "ParameterTypes",
        nptypes,
        "ParameterNames",
        npnames,
    );
    require_equal_counts(
        out,
        "Function.Parameters",
        "ParameterTypes",
        nptypes,
        "ParameterFlags",
        npflags,
    );
    require_equal_counts(
        out,
        "Function.Parameters",
        "ParameterTypes",
        nptypes,
        "ParameterDefaultArgs",
        npdefaults,
    );
    let traits = c.read_i32()?;
    let (declaration_record, runtime_identity) = if let (true, Some(types)) = (
        out.declarations.is_some() || out.capture_function_identities,
        declaration_types,
    ) {
        let key = declaration_pos as i64;
        let special_owner = if matches!(declaration_scope, FunctionDeclarationScope::Global)
            && matches!(name.as_str(), "$fact" | "$beh3")
        {
            Some(types.object_type(key, return_type)?)
        } else {
            None
        };
        let branch = match declaration_scope {
            FunctionDeclarationScope::None => None,
            FunctionDeclarationScope::Global => special_owner
                .as_ref()
                .map_or(Some(("global", None)), |owner| {
                    Some(("method", Some(owner)))
                }),
            FunctionDeclarationScope::Method(owner) => Some(("method", Some(owner))),
        };
        let params = parameter_types
            .iter()
            .copied()
            .map(|datatype| types.datatype(key, datatype))
            .collect::<Result<Vec<_>, _>>()?;
        let ret = types.datatype(key, return_type)?;
        if let Some((branch, owner)) = branch {
            let identity = function_declaration_identity(
                key,
                branch,
                &out.inner_module,
                &namespace,
                owner,
                &name,
                traits & 4 != 0,
                &params,
                &ret,
            )?;
            let runtime_identity = identity.identity.full.clone();
            (
                Some(FunctionDeclarationRecord {
                    identity,
                    pos: declaration_pos,
                }),
                Some(runtime_identity),
            )
        } else {
            // Global initializer functions have no T3 declaration of their own. Bind their
            // runtime id to a portable signature plus the stable record ordinal within the
            // module so independent compiler runs cannot collide across a loadout.
            let identity = function_declaration_identity(
                key,
                "internal",
                &out.inner_module,
                &namespace,
                None,
                &name,
                traits & 4 != 0,
                &params,
                &ret,
            )?;
            let mut encoded = IdentityEncoder::new(key);
            encoded.field("module-function")?;
            encoded.field(&identity.identity.full)?;
            encoded.number(out.function_ids.len())?;
            (None, Some(encoded.finish()))
        }
    } else {
        (None, None)
    };
    // ByteCode TArray<int32>: record the span.
    let count = c.read_count("ByteCode")?;
    let data_off = c.pos();
    out.code.push(CodeSpan { data_off, count });
    c.skip(count * 4)?;
    let _ = bytes; // (bytes used only for span math; offsets are absolute)
    c.skip_tarray_fixed(4, "ByteCodeReferences")?;
    let variable_space = c.read_i32()?;
    if variable_space < 0 {
        record_structural_violation(
            out,
            "Function.VariableSpace",
            format!("VariableSpace must be non-negative, found {variable_space}"),
        );
    }
    // ObjVariableTypes: TArray<int64> of TYPE ptrs (T1) per object local slot — remap each.
    let nobj = c.read_count("ObjVariableTypes")?;
    for _ in 0..nobj {
        out.embeds.push(EmbedRef {
            byte_off: c.pos(),
            kind: EmbedKind::TypePtr(TypePtrRule::Concrete),
        });
        c.skip(8)?;
    }
    let nobjpos = c.read_count("ObjVariablePos")?;
    c.ensure_minimum_remaining(nobjpos, 4, "ObjVariablePos")?;
    let mut object_positions = Vec::with_capacity(nobjpos);
    for _ in 0..nobjpos {
        object_positions.push(c.read_i32()?);
    }
    require_equal_counts(
        out,
        "Function.ObjectVariables",
        "ObjVariableTypes",
        nobj,
        "ObjVariablePos",
        nobjpos,
    );
    for &position in &object_positions {
        if position <= 0 || position > variable_space {
            record_structural_violation(
                out,
                "Function.ObjectVariables",
                format!(
                    "object position {position} must be within 1..={variable_space} VariableSpace"
                ),
            );
        }
    }
    let object_position_set: HashSet<i32> = object_positions.iter().copied().collect();
    let object_variables_on_heap = c.read_i32()?;
    if object_variables_on_heap < 0 || object_variables_on_heap as usize > nobjpos {
        record_structural_violation(
            out,
            "Function.ObjVariablesOnHeap",
            format!(
                "heap prefix {object_variables_on_heap} must be within 0..={nobjpos} object variables"
            ),
        );
    }
    let nvar_program = c.read_count("VariableInfoProgramPos")?;
    c.ensure_minimum_remaining(nvar_program, 4, "VariableInfoProgramPos")?;
    let mut variable_program_positions = Vec::with_capacity(nvar_program);
    for _ in 0..nvar_program {
        variable_program_positions.push(c.read_i32()?);
    }
    let nvar_offset = c.read_count("VariableInfoOffset")?;
    c.ensure_minimum_remaining(nvar_offset, 4, "VariableInfoOffset")?;
    let mut variable_offsets = Vec::with_capacity(nvar_offset);
    for _ in 0..nvar_offset {
        variable_offsets.push(c.read_i32()?);
    }
    let nvar_option = c.read_count("VariableInfoOption")?;
    c.ensure_minimum_remaining(nvar_option, 4, "VariableInfoOption")?;
    let mut variable_options = Vec::with_capacity(nvar_option);
    for _ in 0..nvar_option {
        variable_options.push(c.read_i32()?);
    }
    require_equal_counts(
        out,
        "Function.VariableInfo",
        "ProgramPos",
        nvar_program,
        "Offset",
        nvar_offset,
    );
    require_equal_counts(
        out,
        "Function.VariableInfo",
        "ProgramPos",
        nvar_program,
        "Option",
        nvar_option,
    );
    if nvar_program == nvar_offset && nvar_program == nvar_option {
        let mut previous_program_position = None;
        let mut block_depth = 0usize;
        for (index, ((&program_position, &variable_offset), &option)) in variable_program_positions
            .iter()
            .zip(&variable_offsets)
            .zip(&variable_options)
            .enumerate()
        {
            if program_position < 0 || program_position as usize > count {
                record_structural_violation(
                    out,
                    "Function.VariableInfo",
                    format!(
                        "entry {index} ProgramPos {program_position} must be within 0..={count} ByteCode dwords"
                    ),
                );
            }
            if previous_program_position.is_some_and(|previous| program_position < previous) {
                record_structural_violation(
                    out,
                    "Function.VariableInfo",
                    format!(
                        "entry {index} ProgramPos {program_position} is before the previous position"
                    ),
                );
            }
            previous_program_position = Some(program_position);
            match option {
                0 | 1 => {
                    if !object_position_set.contains(&variable_offset) {
                        record_structural_violation(
                            out,
                            "Function.VariableInfo",
                            format!(
                                "entry {index} object offset {variable_offset} is absent from ObjVariablePos"
                            ),
                        );
                    }
                }
                2 => {
                    if variable_offset != 0 {
                        record_structural_violation(
                            out,
                            "Function.VariableInfo",
                            format!(
                                "entry {index} BLOCK_BEGIN must carry offset 0, found {variable_offset}"
                            ),
                        );
                    }
                    block_depth += 1;
                }
                3 => {
                    if variable_offset != 0 {
                        record_structural_violation(
                            out,
                            "Function.VariableInfo",
                            format!(
                                "entry {index} BLOCK_END must carry offset 0, found {variable_offset}"
                            ),
                        );
                    }
                    match block_depth.checked_sub(1) {
                        Some(depth) => block_depth = depth,
                        None => record_structural_violation(
                            out,
                            "Function.VariableInfo",
                            format!("entry {index} closes a block before any block begins"),
                        ),
                    }
                }
                _ => record_structural_violation(
                    out,
                    "Function.VariableInfo",
                    format!("entry {index} has unknown option {option}; expected 0..=3"),
                ),
            }
        }
        if block_depth != 0 {
            record_structural_violation(
                out,
                "Function.VariableInfo",
                format!("{block_depth} variable-info block(s) remain unclosed"),
            );
        }
    }
    let stack_needed = c.read_i32()?;
    if stack_needed < variable_space.max(0) {
        record_structural_violation(
            out,
            "Function.StackNeeded",
            format!(
                "StackNeeded {stack_needed} must be at least VariableSpace {}",
                variable_space.max(0)
            ),
        );
    }
    let id_off = c.pos();
    out.function_ids.push(c.read_i32()?); // Id
    if out.capture_function_identities {
        let identity = runtime_identity.expect("typed function-id scan has a portable identity");
        out.function_id_identity_bytes = out
            .function_id_identity_bytes
            .checked_add(identity.len())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<ModuleFunctionIdSite>()))
            .filter(|&bytes| bytes <= MAX_DECLARATION_AUTHORITY_BYTES)
            .ok_or(WireError::BadLen {
                pos: id_off,
                len: i64::MAX,
                field: "module Function.Id identity bytes",
            })?;
        out.function_id_sites.push(ModuleFunctionIdSite {
            byte_off: id_off,
            identity,
        });
    }
    c.skip(4)?; // DeclaredAt
    c.skip_tarray_fixed(4, "LineNumbers")?;
    if c.read_bool4()? {
        c.read_sia()?; // UnrealFunctionName
        let nspec = c.read_count("UF.MetaSpec")?;
        for _ in 0..nspec {
            c.read_sia()?;
        }
        let nvalues = c.read_count("UF.MetaValues")?;
        for _ in 0..nvalues {
            c.read_sia()?;
        }
        require_equal_counts(
            out,
            "UFunction.Metadata",
            "MetaSpec",
            nspec,
            "MetaValues",
            nvalues,
        );
        c.skip(18 * 4)?;
    }
    Ok(declaration_record)
}

fn read_property_spans(
    c: &mut Cursor,
    out: &mut ModuleSpans,
    owner: Option<&DeclarationIdentity>,
) -> Result<(), WireError> {
    let declaration_pos = c.pos();
    let name = c.read_sia()?;
    if let (Some(declarations), Some(owner)) = (out.declarations.as_mut(), owner) {
        declarations.insert_property(
            PropertyDeclarationIdentity {
                owner: owner.clone(),
                name,
            },
            declaration_pos,
        )?;
    }
    embed_datatype(c, out)?; // Type
    c.skip(4)?; // bIsPrivate
    c.skip(4)?; // bIsProtected
    if c.read_bool4()? {
        let nspec = c.read_count("UP.MetaSpec")?;
        for _ in 0..nspec {
            c.read_sia()?;
        }
        let nvalues = c.read_count("UP.MetaValues")?;
        for _ in 0..nvalues {
            c.read_sia()?;
        }
        require_equal_counts(
            out,
            "UProperty.Metadata",
            "MetaSpec",
            nspec,
            "MetaValues",
            nvalues,
        );
        c.skip(9 * 4)?;
        let replicated = c.read_bool4()?;
        c.skip(4)?; // bSkipReplication
        c.skip(4)?; // bSkipSerialization
        c.skip(4)?; // bSaveGame
        if replicated {
            c.skip(4)?; // ReplicationCondition
            c.skip(4)?; // bRepNotify
        }
        c.skip(4)?; // bConfig
        c.skip(4)?; // bInterp
        c.skip(4)?; // bAssetRegistrySearchable
    }
    Ok(())
}

fn read_class_spans(
    c: &mut Cursor,
    bytes: &[u8],
    out: &mut ModuleSpans,
    declaration_types: Option<&DeclarationTypeContext<'_>>,
    comparison_budget: Option<&mut IdentityComparisonBudget>,
) -> Result<(), WireError> {
    let declaration_pos = c.pos();
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    let declaration = DeclarationIdentity {
        module: out.inner_module.clone(),
        namespace,
        name,
    };
    if let Some(declarations) = out.declarations.as_mut() {
        declarations.insert_type(declaration.clone(), declaration_pos)?;
    }
    let owner = match declaration_types {
        Some(types) => Some(types.script_owner(
            &declaration,
            comparison_budget.expect("declaration scan has a comparison budget"),
        )?)
        .flatten(),
        None => None,
    };
    let flags = c.read_i32()?;
    let nprops = c.read_count("Class.Properties")?;
    for _ in 0..nprops {
        read_property_spans(c, out, Some(&declaration))?;
    }
    let nmethods = c.read_count("Class.Methods")?;
    let mut method_declarations = Vec::with_capacity(nmethods);
    for _ in 0..nmethods {
        method_declarations.push(read_function_spans(
            c,
            bytes,
            out,
            declaration_types,
            owner.map_or(
                FunctionDeclarationScope::None,
                FunctionDeclarationScope::Method,
            ),
        )?);
    }
    let nmethod_table = c.read_count("Class.MethodTable")?;
    let mut seen_method_indices = HashSet::new();
    let mut method_table_valid = true;
    for slot in 0..nmethod_table {
        let index = c.read_i32()?;
        if index == -1 {
            continue;
        }
        if index < 0 || index as usize >= nmethods || !seen_method_indices.insert(index) {
            method_table_valid = false;
            record_structural_violation(
                out,
                "Class.MethodTable",
                format!(
                    "slot {slot} references invalid or duplicate local method index {index} (Methods.Num={nmethods})"
                ),
            );
        }
    }
    let is_value_class = flags & 0x2 != 0;
    if is_value_class && nmethod_table != 0 {
        method_table_valid = false;
        record_structural_violation(
            out,
            "Class.MethodTable",
            format!(
                "value class Methods are created directly and require an empty MethodTable, found {nmethod_table} entries"
            ),
        );
    } else if !is_value_class && seen_method_indices.len() != nmethods {
        method_table_valid = false;
        record_structural_violation(
            out,
            "Class.MethodTable",
            format!(
                "reference class MethodTable covers {} of {nmethods} local Methods; every local method must appear exactly once",
                seen_method_indices.len()
            ),
        );
    }
    if method_table_valid {
        for declaration in method_declarations {
            insert_function_declaration(out, declaration)?;
        }
    }
    // DerivedFrom + ShadowType: int64 TYPE ptrs (T1). Value 0 = none (skipped at remap time).
    out.embeds.push(EmbedRef {
        byte_off: c.pos(),
        kind: EmbedKind::TypePtr(TypePtrRule::Optional),
    });
    c.skip(8)?; // DerivedFrom
    out.embeds.push(EmbedRef {
        byte_off: c.pos(),
        kind: EmbedKind::TypePtr(TypePtrRule::Optional),
    });
    c.skip(8)?; // ShadowType
    let nctors = c.read_count("Class.Constructors")?;
    for _ in 0..nctors {
        let declaration = read_function_spans(
            c,
            bytes,
            out,
            declaration_types,
            owner.map_or(
                FunctionDeclarationScope::None,
                FunctionDeclarationScope::Method,
            ),
        )?;
        insert_function_declaration(out, declaration)?;
    }
    // FactoryRefs + BehaviorRefs: TArray<int64> of FUNC ids (T4); zero is the only observed and
    // admitted sentinel. Behavior type tags live in the separate BehaviorFunctionTypes array.
    let nfact = c.read_count("Class.FactoryRefs")?;
    for _ in 0..nfact {
        out.embeds.push(EmbedRef {
            byte_off: c.pos(),
            kind: EmbedKind::FuncId,
        });
        c.skip(8)?;
    }
    let nbeh = c.read_count("Class.BehaviorRefs")?;
    for _ in 0..nbeh {
        out.embeds.push(EmbedRef {
            byte_off: c.pos(),
            kind: EmbedKind::FuncId,
        });
        c.skip(8)?;
    }
    if nbeh != 7 {
        record_structural_violation(
            out,
            "Class.BehaviorRefs",
            format!("runtime requires exactly 7 entries, found {nbeh}"),
        );
    }
    let nbehav = c.read_count("Class.BehaviorFunctions")?;
    if nbehav > 1 {
        record_structural_violation(
            out,
            "Class.BehaviorFunctions",
            format!("runtime supports at most one synthesized destructor behavior, found {nbehav}"),
        );
    }
    for _ in 0..nbehav {
        let declaration = read_function_spans(
            c,
            bytes,
            out,
            declaration_types,
            owner.map_or(
                FunctionDeclarationScope::None,
                FunctionDeclarationScope::Method,
            ),
        )?;
        insert_function_declaration(out, declaration)?;
    }
    let nbehav_types = c.read_count("Class.BehaviorFunctionTypes")?;
    for index in 0..nbehav_types {
        let behavior_type = c.read_i32()?;
        if behavior_type != 2 {
            record_structural_violation(
                out,
                "Class.BehaviorFunctionTypes",
                format!(
                    "entry {index} has behavior type {behavior_type}; only asBEHAVE_DESTRUCT (2) is supported"
                ),
            );
        }
    }
    require_equal_counts(
        out,
        "Class.Behaviors",
        "BehaviorFunctions",
        nbehav,
        "BehaviorFunctionTypes",
        nbehav_types,
    );
    if c.read_bool4()? {
        c.read_sia()?; // SuperClass
        c.read_sia()?; // CodeSuperClass
        c.skip(8 * 4)?;
        c.read_sia()?; // StaticClassGVName
        c.skip(4)?; // bPlaceable
        let nspec = c.read_count("Class.MetaSpec")?;
        for _ in 0..nspec {
            c.read_sia()?;
        }
        let nvalues = c.read_count("Class.MetaValues")?;
        for _ in 0..nvalues {
            c.read_sia()?;
        }
        require_equal_counts(
            out,
            "Class.Metadata",
            "MetaSpec",
            nspec,
            "MetaValues",
            nvalues,
        );
        c.read_sia()?; // ComposeOntoClassName
    }
    Ok(())
}

fn read_enum_spans(c: &mut Cursor, out: &mut ModuleSpans) -> Result<(), WireError> {
    let declaration_pos = c.pos();
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    if let Some(declarations) = out.declarations.as_mut() {
        declarations.insert_type(
            DeclarationIdentity {
                module: out.inner_module.clone(),
                namespace,
                name,
            },
            declaration_pos,
        )?;
    }
    let nnames = c.read_count("Enum.Names")?;
    for _ in 0..nnames {
        c.read_sia()?;
    }
    let nvalues = c.read_count("Enum.Values")?;
    c.skip(nvalues * 4)?;
    require_equal_counts(out, "Enum.Entries", "Names", nnames, "Values", nvalues);
    Ok(())
}

fn read_global_spans(
    c: &mut Cursor,
    bytes: &[u8],
    out: &mut ModuleSpans,
    declaration_types: Option<&DeclarationTypeContext<'_>>,
) -> Result<(), WireError> {
    let declaration_pos = c.pos();
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    if let Some(declarations) = out.declarations.as_mut() {
        declarations.insert_global(
            DeclarationIdentity {
                module: out.inner_module.clone(),
                namespace,
                name,
            },
            declaration_pos,
        )?;
    }
    embed_datatype(c, out)?; // Type
    if !c.read_bool4()? {
        // !bIsDefaultInit
        if c.read_bool4()? {
            c.skip(8)?; // PureConstantValue
        } else if c.read_bool4()? {
            read_function_spans(
                c,
                bytes,
                out,
                declaration_types,
                FunctionDeclarationScope::None,
            )?; // InitFunc carries bytecode but is not a T3 declaration
        }
    }
    Ok(())
}

fn read_function_import_spans(
    c: &mut Cursor,
    out: &mut ModuleSpans,
    declaration_types: Option<&DeclarationTypeContext<'_>>,
) -> Result<(), WireError> {
    let declaration_pos = c.pos();
    c.read_sia()?; // ImportedFromModule
    let name = c.read_sia()?;
    c.read_sia()?; // Namespace
    let nptypes = c.read_count("Import.ParameterTypes")?;
    let mut parameter_types = Vec::with_capacity(nptypes);
    for _ in 0..nptypes {
        parameter_types.push(embed_datatype(c, out)?);
    }
    let npflags = c.read_count("Import.ParameterFlags")?;
    c.skip(npflags * 4)?;
    let npdefaults = c.read_count("Import.ParameterDefaultArgs")?;
    for _ in 0..npdefaults {
        c.read_sia()?;
    }
    require_equal_counts(
        out,
        "Import.Parameters",
        "ParameterTypes",
        nptypes,
        "ParameterFlags",
        npflags,
    );
    require_equal_counts(
        out,
        "Import.Parameters",
        "ParameterTypes",
        nptypes,
        "ParameterDefaultArgs",
        npdefaults,
    );
    let return_type = embed_datatype(c, out)?;
    if let (Some(types), true) = (declaration_types, out.declarations.is_some()) {
        let key = declaration_pos as i64;
        let params = parameter_types
            .into_iter()
            .map(|datatype| types.datatype(key, datatype))
            .collect::<Result<Vec<_>, _>>()?;
        let ret = types.datatype(key, return_type)?;
        insert_function_declaration(
            out,
            Some(FunctionDeclarationRecord {
                identity: function_declaration_identity(
                    key,
                    "imported",
                    &out.inner_module,
                    "",
                    None,
                    &name,
                    false,
                    &params,
                    &ret,
                )?,
                pos: declaration_pos,
            }),
        )?;
    }
    Ok(())
}

fn read_module_spans(
    c: &mut Cursor,
    bytes: &[u8],
    out: &mut ModuleSpans,
    declaration_types: Option<&DeclarationTypeContext<'_>>,
    mut comparison_budget: Option<&mut IdentityComparisonBudget>,
) -> Result<(), WireError> {
    out.inner_module = c.read_sia()?; // ModuleName
    let nfns = c.read_count("Module.Functions")?;
    for _ in 0..nfns {
        let declaration = read_function_spans(
            c,
            bytes,
            out,
            declaration_types,
            FunctionDeclarationScope::Global,
        )?;
        insert_function_declaration(out, declaration)?;
    }
    let nclasses = c.read_count("Module.Classes")?;
    for _ in 0..nclasses {
        read_class_spans(
            c,
            bytes,
            out,
            declaration_types,
            comparison_budget.as_deref_mut(),
        )?;
    }
    let nenums = c.read_count("Module.Enums")?;
    for _ in 0..nenums {
        read_enum_spans(c, out)?;
    }
    let nglobals = c.read_count("Module.GlobalVariables")?;
    for _ in 0..nglobals {
        read_global_spans(c, bytes, out, declaration_types)?;
    }
    let nimports = c.read_count("Module.FunctionImports")?;
    for _ in 0..nimports {
        read_function_import_spans(c, out, declaration_types)?;
    }
    c.skip(8)?; // CodeHash
    c.skip_tarray_sia("Module.ImportedModules")?;
    c.read_sia()?; // StaticsClassName
    c.skip_tarray_sia("Module.DeclaredEvents")?;
    c.skip_tarray_sia("Module.DeclaredDelegates")?;
    c.read_sia()?; // ScriptRelativeFilename
    c.skip_tarray_sia("Module.PostInitFunctions")?;
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Opt-in new-symbol planner.
// -------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum NovelPointerKind {
    Type,
    Function,
    Global,
}

impl NovelPointerKind {
    const fn hash_domain(self) -> u8 {
        match self {
            Self::Type => 0,
            Self::Function => 1,
            Self::Global => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[allow(dead_code)] // consumed by the next, splice-level loadout integration tranche
struct NovelPointerIdentity {
    kind: NovelPointerKind,
    identity: String,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[allow(dead_code)]
struct NovelTypeIdIdentity {
    object_kind: u32,
    identity: String,
}

/// Exact portable identities observed while validating one input mini. The builder merges these
/// into one owned union; each mini retains only a compact canonical fingerprint beside its SHA.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(dead_code)]
struct NovelIdentitySet {
    pointers: BTreeSet<NovelPointerIdentity>,
    type_ids: BTreeMap<String, u32>,
    function_ids: BTreeSet<String>,
    module_function_ids: BTreeSet<String>,
}

#[allow(dead_code)]
impl NovelIdentitySet {
    fn additional_usage(
        &self,
        inventory: &Self,
        limits: LoadoutPlanLimits,
    ) -> Result<(usize, usize), RemapError> {
        let mut entries = 0usize;
        let mut bytes = 0usize;
        for identity in &self.pointers {
            if !inventory.pointers.contains(identity) {
                entries = entries
                    .checked_add(1)
                    .ok_or(RemapError::LoadoutPlanResourceLimit {
                        resource: "novel assignments",
                        actual: usize::MAX,
                        limit: limits.max_assignments,
                    })?;
                bytes = bytes.checked_add(identity.identity.len()).ok_or(
                    RemapError::LoadoutPlanResourceLimit {
                        resource: "identity bytes",
                        actual: usize::MAX,
                        limit: limits.max_identity_bytes,
                    },
                )?;
            }
        }
        for (identity, object_kind) in &self.type_ids {
            if let Some(&prior_kind) = inventory.type_ids.get(identity) {
                if prior_kind != *object_kind {
                    return Err(RemapError::LoadoutPlanTypeKindConflict {
                        identity: identity.clone(),
                        first: prior_kind,
                        second: *object_kind,
                    });
                }
            } else {
                entries = entries
                    .checked_add(1)
                    .ok_or(RemapError::LoadoutPlanResourceLimit {
                        resource: "novel assignments",
                        actual: usize::MAX,
                        limit: limits.max_assignments,
                    })?;
                bytes = bytes.checked_add(identity.len()).ok_or(
                    RemapError::LoadoutPlanResourceLimit {
                        resource: "identity bytes",
                        actual: usize::MAX,
                        limit: limits.max_identity_bytes,
                    },
                )?;
            }
        }
        for identity in &self.function_ids {
            if !inventory.function_ids.contains(identity) {
                entries = entries
                    .checked_add(1)
                    .ok_or(RemapError::LoadoutPlanResourceLimit {
                        resource: "novel assignments",
                        actual: usize::MAX,
                        limit: limits.max_assignments,
                    })?;
                bytes = bytes.checked_add(identity.len()).ok_or(
                    RemapError::LoadoutPlanResourceLimit {
                        resource: "identity bytes",
                        actual: usize::MAX,
                        limit: limits.max_identity_bytes,
                    },
                )?;
            }
        }
        for identity in &self.module_function_ids {
            if !inventory.module_function_ids.contains(identity) {
                entries = entries
                    .checked_add(1)
                    .ok_or(RemapError::LoadoutPlanResourceLimit {
                        resource: "novel assignments",
                        actual: usize::MAX,
                        limit: limits.max_assignments,
                    })?;
                bytes = bytes.checked_add(identity.len()).ok_or(
                    RemapError::LoadoutPlanResourceLimit {
                        resource: "identity bytes",
                        actual: usize::MAX,
                        limit: limits.max_identity_bytes,
                    },
                )?;
            }
        }
        Ok((entries, bytes))
    }

    fn merge_into(self, inventory: &mut Self) {
        inventory.pointers.extend(self.pointers);
        inventory.type_ids.extend(self.type_ids);
        inventory.function_ids.extend(self.function_ids);
        inventory
            .module_function_ids
            .extend(self.module_function_ids);
    }

    fn fingerprint(&self) -> [u8; 32] {
        fn field(hasher: &mut Sha256, tag: u8, value: &str) {
            hasher.update([tag]);
            hasher.update((value.len() as u64).to_le_bytes());
            hasher.update(value.as_bytes());
        }

        let mut hasher = Sha256::new();
        hasher.update(b"gore-loadout-script-id-identities-v1");
        for identity in &self.pointers {
            hasher.update([0, identity.kind.hash_domain()]);
            field(&mut hasher, 1, &identity.identity);
        }
        for (identity, object_kind) in &self.type_ids {
            hasher.update([2]);
            hasher.update(object_kind.to_le_bytes());
            field(&mut hasher, 3, identity);
        }
        for identity in &self.function_ids {
            field(&mut hasher, 4, identity);
        }
        for identity in &self.module_function_ids {
            field(&mut hasher, 5, identity);
        }
        hasher.finalize().into()
    }
}

#[allow(dead_code)]
const MAX_LOADOUT_PLAN_MINIS: usize = 256;
#[allow(dead_code)]
const MAX_LOADOUT_PLAN_ASSIGNMENTS: usize = 131_072;
#[allow(dead_code)]
const MAX_LOADOUT_PLAN_IDENTITY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct LoadoutPlanLimits {
    max_minis: usize,
    max_assignments: usize,
    max_identity_bytes: usize,
}

#[allow(dead_code)]
const PRODUCTION_LOADOUT_PLAN_LIMITS: LoadoutPlanLimits = LoadoutPlanLimits {
    max_minis: MAX_LOADOUT_PLAN_MINIS,
    max_assignments: MAX_LOADOUT_PLAN_ASSIGNMENTS,
    max_identity_bytes: MAX_LOADOUT_PLAN_IDENTITY_BYTES,
};

/// Canonical Script-ID assignments for one loadout, always derived from the unchanged pristine
/// base plus the union of every inspected mini's portable novel identities. This is deliberately
/// crate-internal until the composer owns the two-pass orchestration.
#[allow(dead_code)]
pub(super) struct LoadoutScriptIdPlan {
    pristine_base_sha256: [u8; 32],
    pristine_guid: [u8; 16],
    pristine_header: [u8; 0x14],
    base: Arc<AllowNewBaseContext>,
    effective_base: Arc<EffectiveReferenceBase>,
    inspected_minis: HashMap<[u8; 32], [u8; 32]>,
    pointer_assignments: BTreeMap<NovelPointerIdentity, i64>,
    type_id_assignments: BTreeMap<NovelTypeIdIdentity, i32>,
    function_id_assignments: BTreeMap<String, i32>,
    module_function_id_assignments: BTreeMap<String, i32>,
}

/// Streaming first pass for a loadout. `inspect` retains no mini bytes, so callers may read,
/// inspect, and drop each artifact before opening the next one.
#[allow(dead_code)]
pub(super) struct LoadoutScriptIdPlanBuilder {
    pristine_base_sha256: [u8; 32],
    pristine_guid: [u8; 16],
    pristine_header: [u8; 0x14],
    base: Arc<AllowNewBaseContext>,
    effective_base: Arc<EffectiveReferenceBase>,
    inspected_count: usize,
    inspected_minis: HashMap<[u8; 32], [u8; 32]>,
    inventory: NovelIdentitySet,
    assignment_entries: usize,
    identity_bytes: usize,
    limits: LoadoutPlanLimits,
    domains: CanonicalAllocationDomains,
}

#[derive(Default)]
struct NewSymbolPlan {
    new_types: HashSet<i64>,
    new_funcs: HashSet<i64>,
    new_globals: HashSet<i64>,
    /// Regen ptr -> final ptr. Existing symbols point at vanilla; new symbols are filled after
    /// deterministic identity allocation.
    type_ptrs: HashMap<i64, i64>,
    func_ptrs: HashMap<i64, i64>,
    global_ptrs: HashMap<i64, i64>,
    /// Regen engine id -> final engine id (new rows only; existing ids resolve through ptr maps).
    type_ids: HashMap<i32, i32>,
    func_ids: HashMap<i32, i32>,
    module_function_ids: HashMap<String, i32>,
    used_static_indices: HashSet<i64>,
    /// Exact member sites used by target bytecode, in the regen generation's core type-id space.
    /// A pristine script class may declare a property that vanilla bytecode never referenced, so
    /// its T7 row can be absent from `base` even though the declaration is valid authority.
    used_property_sites: HashSet<(i32, i32)>,
    static_indices: HashMap<i64, i64>,
    selected_static_rows: Vec<usize>,
    selected_properties: Vec<SelectedProperty>,
}

struct SelectedProperty {
    index: usize,
    key: i64,
    type_id: i32,
}

fn match_base_ptr(
    kind: &'static str,
    op: &'static str,
    regen_key: i64,
    regen_id_of_ptr: &HashMap<i64, String>,
    regen_ident_of_ptr: &HashMap<i64, Ident>,
    regen_name_of_ptr: &HashMap<i64, String>,
    base_ptr_of_id: &HashMap<String, Vec<i64>>,
    base_ident_of_ptr: &HashMap<i64, Ident>,
    base_summary: &IdentityReverseSummary,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<Option<i64>, RemapError> {
    let identity = regen_id_of_ptr
        .get(&regen_key)
        .ok_or_else(|| RemapError::Unresolved {
            kind,
            op,
            key: regen_key,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
        })?;
    match base_ptr_of_id.get(identity).map(Vec::as_slice) {
        Some([key]) => return Ok(Some(*key)),
        Some([]) | None => {}
        Some(many) => {
            return Err(RemapError::Ambiguous {
                kind,
                op,
                name: regen_name_of_ptr
                    .get(&regen_key)
                    .cloned()
                    .unwrap_or_default(),
                n: many.len(),
            });
        }
    }

    // The emitter can drop namespace blocks (GAP-A), so an exact identity miss does not prove
    // that this is a genuinely new symbol. Reuse the semantic oracle's deliberately narrow
    // namespace tolerance, but only accept a unique base row. `oracle_eq` is pairwise and not
    // transitive when an empty namespace bridges two real namespaces; treating that case as
    // ambiguous prevents the allow-new path from silently choosing the wrong existing symbol.
    let regen_ident = regen_ident_of_ptr
        .get(&regen_key)
        .ok_or_else(|| RemapError::Unresolved {
            kind,
            op,
            key: regen_key,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
        })?;
    let regen_footprint = identity_footprint(regen_key, regen_ident)?;
    let mut first = None;
    let mut matches = 0usize;
    for &(key, candidate_footprint) in base_summary
        .by_skeleton
        .get(&regen_ident.ns_stripped)
        .into_iter()
        .flatten()
    {
        comparison_budget.charge(regen_footprint, candidate_footprint)?;
        let candidate = base_ident_of_ptr
            .get(&key)
            .expect("base identity summary points to its source row");
        if !regen_ident.oracle_eq(candidate) {
            continue;
        }
        matches = matches.saturating_add(1);
        if first.is_none() {
            first = Some(key);
        }
    }
    let Some(first) = first else {
        return Ok(None);
    };
    if matches > 1 {
        return Err(RemapError::Ambiguous {
            kind,
            op,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
            n: matches,
        });
    }
    Ok(Some(first))
}

fn declare_type(
    plan: &mut NewSymbolPlan,
    key: i64,
    op: &'static str,
    regen: &SymTables,
    base: &SymTables,
    base_summaries: &SymbolIdentitySummaries,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    if key == 0 || plan.type_ptrs.contains_key(&key) || plan.new_types.contains(&key) {
        return Ok(());
    }
    // Exact key repeats are already unambiguous even when the pristine cache contains several
    // historical aliases for the same portable identity. This mirrors the sequential guard's
    // grandfathering rule without allowing a raw-key collision with a different declaration.
    if regen.type_id_of_ptr.get(&key) == base.type_id_of_ptr.get(&key)
        && regen.type_id_of_ptr.contains_key(&key)
    {
        plan.type_ptrs.insert(key, key);
        return Ok(());
    }
    // A minimal prepared mini intentionally omits pristine T1 rows while its signatures retain
    // the already-final pristine pointer. Only accept that direct key when the mini does not
    // define a competing row of its own; raw-key collisions in a full regen must still resolve by
    // portable identity.
    if !regen.type_id_of_ptr.contains_key(&key) && base.type_id_of_ptr.contains_key(&key) {
        plan.type_ptrs.insert(key, key);
        return Ok(());
    }
    match match_base_ptr(
        "type",
        op,
        key,
        &regen.type_id_of_ptr,
        &regen.type_ident_of_ptr,
        &regen.type_name_of_ptr,
        &base.type_ptr_of_id,
        &base.type_ident_of_ptr,
        &base_summaries.types,
        comparison_budget,
    )? {
        Some(vanilla) => {
            plan.type_ptrs.insert(key, vanilla);
        }
        None => {
            plan.new_types.insert(key);
        }
    }
    Ok(())
}

fn declare_func(
    plan: &mut NewSymbolPlan,
    key: i64,
    op: &'static str,
    regen: &SymTables,
    base: &SymTables,
    base_summaries: &SymbolIdentitySummaries,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    if key == 0 || plan.func_ptrs.contains_key(&key) || plan.new_funcs.contains(&key) {
        return Ok(());
    }
    if regen.func_id_of_ptr.get(&key) == base.func_id_of_ptr.get(&key)
        && regen.func_id_of_ptr.contains_key(&key)
    {
        plan.func_ptrs.insert(key, key);
        return Ok(());
    }
    if !regen.func_id_of_ptr.contains_key(&key) && base.func_id_of_ptr.contains_key(&key) {
        plan.func_ptrs.insert(key, key);
        return Ok(());
    }
    match match_base_ptr(
        "function",
        op,
        key,
        &regen.func_id_of_ptr,
        &regen.func_ident_of_ptr,
        &regen.func_name_of_ptr,
        &base.func_ptr_of_id,
        &base.func_ident_of_ptr,
        &base_summaries.functions,
        comparison_budget,
    )? {
        Some(vanilla) => {
            plan.func_ptrs.insert(key, vanilla);
        }
        None => {
            plan.new_funcs.insert(key);
        }
    }
    Ok(())
}

fn declare_global(
    plan: &mut NewSymbolPlan,
    key: i64,
    op: &'static str,
    regen: &SymTables,
    base: &SymTables,
    base_summaries: &SymbolIdentitySummaries,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    if key == 0 || plan.global_ptrs.contains_key(&key) || plan.new_globals.contains(&key) {
        return Ok(());
    }
    if regen
        .global_is_string_of_ptr
        .get(&key)
        .copied()
        .unwrap_or(false)
    {
        let identity = regen
            .global_id_of_ptr
            .get(&key)
            .ok_or(RemapError::MissingNewRow {
                kind: "string global identity",
                key,
            })?;
        if let Some(existing) = base
            .global_ptr_of_id
            .get(identity)
            .and_then(|keys| keys.first().copied())
        {
            // Equal string-literal T5 rows are runtime-equivalent. Shipping contains thousands
            // of aliases, so canonicalize a regenerated key to the stable smallest base key.
            plan.global_ptrs.insert(key, existing);
        } else {
            plan.new_globals.insert(key);
        }
        return Ok(());
    }
    if regen.global_id_of_ptr.get(&key) == base.global_id_of_ptr.get(&key)
        && regen.global_id_of_ptr.contains_key(&key)
    {
        plan.global_ptrs.insert(key, key);
        return Ok(());
    }
    if !regen.global_id_of_ptr.contains_key(&key) && base.global_id_of_ptr.contains_key(&key) {
        plan.global_ptrs.insert(key, key);
        return Ok(());
    }
    match match_base_ptr(
        "global",
        op,
        key,
        &regen.global_id_of_ptr,
        &regen.global_ident_of_ptr,
        &regen.global_name_of_ptr,
        &base.global_ptr_of_id,
        &base.global_ident_of_ptr,
        &base_summaries.globals,
        comparison_budget,
    )? {
        Some(vanilla) => {
            plan.global_ptrs.insert(key, vanilla);
        }
        None => {
            plan.new_globals.insert(key);
        }
    }
    Ok(())
}

fn callee_name_from_effective<'a>(
    ins: &super::disasm::Instr,
    code: &[i32],
    regen: &'a SymTables,
    base: &'a SymTables,
) -> Option<&'a str> {
    match ins.op.name {
        "CALLSYS" | "FuncPtr" | "Thiscall1" => {
            let ptr = read_qw(code, ins.offset_dw + 1);
            regen
                .func_name_of_ptr
                .get(&ptr)
                .or_else(|| base.func_name_of_ptr.get(&ptr))
                .map(String::as_str)
        }
        "CALL" | "CALLBND" | "CALLINTF" => {
            let id = code[ins.offset_dw + 1];
            (id != 0)
                .then_some(id)
                .and_then(|id| {
                    regen
                        .funcid_to_ptr
                        .get(&id)
                        .or_else(|| base.funcid_to_ptr.get(&id))
                })
                .and_then(|ptr| {
                    regen
                        .func_name_of_ptr
                        .get(ptr)
                        .or_else(|| base.func_name_of_ptr.get(ptr))
                })
                .map(String::as_str)
        }
        _ => None,
    }
}

fn analyze_bytecode_for_new_symbols(
    code: &[i32],
    plan: &mut NewSymbolPlan,
    regen: &SymTables,
    base: &SymTables,
    base_summaries: &SymbolIdentitySummaries,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    let instrs = disassemble(code).map_err(|e| RemapError::Disasm(e.to_string()))?;
    for (pos, ins) in instrs.iter().enumerate() {
        if matches!(
            ins.op.name,
            "ADDSi" | "LoadThisR" | "LoadRObjR" | "LoadVObjR"
        ) {
            let raw_type_id = ins.dwords.first().copied().ok_or_else(|| {
                RemapError::Disasm(format!(
                    "{} is missing its owner type-id operand",
                    ins.op.name
                ))
            })? as i32;
            let member_offset = ins
                .words
                .last()
                .copied()
                .map(|word| i32::from(word as i16))
                .ok_or_else(|| {
                    RemapError::Disasm(format!(
                        "{} is missing its member-offset operand",
                        ins.op.name
                    ))
                })?;
            let (owner_core_id, _) = split_type_id_operand(raw_type_id);
            plan.used_property_sites
                .insert((owner_core_id, member_offset));
        }
        for site in ref_sites(ins.op.name) {
            let off = ins.offset_dw + site.dw_index;
            match site.kind {
                RefKind::GlobalPtr => declare_global(
                    plan,
                    read_qw(code, off),
                    ins.op.name,
                    regen,
                    base,
                    base_summaries,
                    comparison_budget,
                )?,
                RefKind::FuncPtr => declare_func(
                    plan,
                    read_qw(code, off),
                    ins.op.name,
                    regen,
                    base,
                    base_summaries,
                    comparison_budget,
                )?,
                RefKind::TypePtr => declare_type(
                    plan,
                    read_qw(code, off),
                    ins.op.name,
                    regen,
                    base,
                    base_summaries,
                    comparison_budget,
                )?,
                RefKind::FuncId => {
                    let id = code[off];
                    if id != 0 {
                        if let Some(&ptr) = regen.funcid_to_ptr.get(&id) {
                            declare_func(
                                plan,
                                ptr,
                                ins.op.name,
                                regen,
                                base,
                                base_summaries,
                                comparison_budget,
                            )?;
                        }
                    }
                }
                RefKind::TypeId => {
                    let (core, _) = split_type_id_operand(code[off]);
                    if let Some(&ptr) = regen.typeid_to_ptr.get(&core) {
                        declare_type(
                            plan,
                            ptr,
                            ins.op.name,
                            regen,
                            base,
                            base_summaries,
                            comparison_budget,
                        )?;
                    }
                }
            }
        }

        // StaticNames has two observed operand forms. STR stores a u16 index in dword 0's high
        // word. An n"..." literal stores an i32 index in PshC4 immediately before the native
        // __STATIC_NAME accessor. Record both by text later, after all refs are classified.
        if ins.op.name == "STR" {
            let idx = ((code[ins.offset_dw] as u32 >> 16) & 0xffff) as i64;
            plan.used_static_indices.insert(idx);
        } else if ins.op.name == "PshC4"
            && instrs
                .get(pos + 1)
                .and_then(|next| callee_name_from_effective(next, code, regen, base))
                == Some("__STATIC_NAME")
        {
            plan.used_static_indices
                .insert(code[ins.offset_dw + 1] as i64);
        }
    }
    Ok(())
}

fn target_module_names(mini: &[u8]) -> Result<HashSet<String>, WireError> {
    let mut c = Cursor::at(mini, CacheHeader::SIZE);
    c.read_fstring()?; // outer TMap key is not a declaring-module identity
    let inner = c.read_sia()?;
    Ok([inner].into_iter().filter(|s| !s.is_empty()).collect())
}

fn close_type_dependencies(
    plan: &mut NewSymbolPlan,
    meta: &TailMetadata,
    regen: &SymTables,
    base: &SymTables,
    base_summaries: &SymbolIdentitySummaries,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    let mut pending_types: VecDeque<i64> = plan.new_types.iter().copied().collect();
    let new_funcs: Vec<i64> = plan.new_funcs.iter().copied().collect();
    for key in new_funcs {
        let row = meta.func_row(key).ok_or(RemapError::MissingNewRow {
            kind: "function",
            key,
        })?;
        if row.owner_dep.1 != 0 {
            let dependency = row.owner_dep.1;
            let was_new = plan.new_types.contains(&dependency);
            declare_type(
                plan,
                dependency,
                "FunctionReference.ObjectType",
                regen,
                base,
                base_summaries,
                comparison_budget,
            )?;
            if !was_new && plan.new_types.contains(&dependency) {
                pending_types.push_back(dependency);
            }
        }
        for dependency in row
            .type_deps
            .iter()
            .map(|dep| dep.ptr)
            .filter(|&ptr| ptr != 0)
        {
            let was_new = plan.new_types.contains(&dependency);
            declare_type(
                plan,
                dependency,
                "FunctionReference.DataType",
                regen,
                base,
                base_summaries,
                comparison_budget,
            )?;
            if !was_new && plan.new_types.contains(&dependency) {
                pending_types.push_back(dependency);
            }
        }
    }

    while let Some(key) = pending_types.pop_front() {
        let row = meta
            .type_row(key)
            .ok_or(RemapError::MissingNewRow { kind: "type", key })?;
        for dependency in row
            .type_deps
            .iter()
            .map(|dep| dep.ptr)
            .filter(|&ptr| ptr != 0)
        {
            let was_new = plan.new_types.contains(&dependency);
            declare_type(
                plan,
                dependency,
                "TypeRef.SubTypes",
                regen,
                base,
                base_summaries,
                comparison_budget,
            )?;
            if !was_new && plan.new_types.contains(&dependency) {
                pending_types.push_back(dependency);
            }
        }
    }
    Ok(())
}

fn seed_target_module_symbols(
    plan: &mut NewSymbolPlan,
    meta: &TailMetadata,
    targets: &HashSet<String>,
    regen: &SymTables,
    base: &SymTables,
    base_summaries: &SymbolIdentitySummaries,
    comparison_budget: &mut IdentityComparisonBudget,
) -> Result<(), RemapError> {
    for row in &meta.types {
        if targets.contains(&row.module) {
            declare_type(
                plan,
                row.key,
                "target module TypeReferences",
                regen,
                base,
                base_summaries,
                comparison_budget,
            )?;
        }
    }
    for row in &meta.funcs {
        if targets.contains(&row.module) {
            declare_func(
                plan,
                row.key,
                "target module FunctionReferences",
                regen,
                base,
                base_summaries,
                comparison_budget,
            )?;
        }
    }
    for row in &meta.globals {
        if targets.contains(&row.module) {
            declare_global(
                plan,
                row.key,
                "target module GlobalReferences",
                regen,
                base,
                base_summaries,
                comparison_budget,
            )?;
        }
    }
    Ok(())
}

fn stable_hash64(kind: u8, identity: &str) -> u64 {
    // Fixed FNV-1a (not RandomState/SipHash): identical caches always get identical rekeys.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    h ^= kind as u64;
    h = h.wrapping_mul(0x0000_0100_0000_01b3);
    for &b in identity.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

const SYNTHETIC_POINTER_HIGH: u64 = 0x6000_0000_0000_0000;
const SYNTHETIC_POINTER_LOW_MASK: u64 = 0x0fff_ffff_ffff_fff8;
const SYNTHETIC_POINTER_SLOT_HIGH: u64 = SYNTHETIC_POINTER_LOW_MASK >> 3;

#[derive(Clone, Copy)]
struct CanonicalAllocationDomains {
    pointer_slot_high: u64,
    type_sequence_high: u32,
    function_id_high: u64,
}

const PRODUCTION_ALLOCATION_DOMAINS: CanonicalAllocationDomains = CanonicalAllocationDomains {
    pointer_slot_high: SYNTHETIC_POINTER_SLOT_HIGH,
    type_sequence_high: TYPE_ID_SEQUENCE_MASK,
    function_id_high: i32::MAX as u64,
};

/// Occupied inclusive intervals over one finite integer domain. A lookup jumps directly to the
/// end of the containing interval, so collision chains and wrap-around stay O(log n) instead of
/// probing one occupied slot at a time.
#[derive(Clone, Debug)]
struct SuccessorAllocator {
    low: u64,
    high: u64,
    occupied: BTreeMap<u64, u64>,
    occupied_count: u128,
}

impl SuccessorAllocator {
    fn new(low: u64, high: u64, occupied: impl IntoIterator<Item = u64>) -> Self {
        assert!(low <= high, "invalid successor-allocation domain");
        let mut allocator = Self {
            low,
            high,
            occupied: BTreeMap::new(),
            occupied_count: 0,
        };
        for value in occupied {
            allocator.occupy(value);
        }
        allocator
    }

    fn domain_len(&self) -> u128 {
        u128::from(self.high) - u128::from(self.low) + 1
    }

    fn occupy(&mut self, value: u64) {
        if value < self.low || value > self.high {
            return;
        }
        let left = self
            .occupied
            .range(..=value)
            .next_back()
            .map(|(&start, &end)| (start, end));
        if left.is_some_and(|(_, end)| value <= end) {
            return;
        }
        let right = self
            .occupied
            .range(value..)
            .next()
            .map(|(&start, &end)| (start, end));
        let joins_left = left.is_some_and(|(_, end)| end.checked_add(1) == Some(value));
        let joins_right = right.is_some_and(|(start, _)| value.checked_add(1) == Some(start));

        match (left, right, joins_left, joins_right) {
            (Some((left_start, _)), Some((right_start, right_end)), true, true) => {
                self.occupied.remove(&right_start);
                *self
                    .occupied
                    .get_mut(&left_start)
                    .expect("left interval remains present") = right_end;
            }
            (Some((left_start, _)), _, true, false) => {
                *self
                    .occupied
                    .get_mut(&left_start)
                    .expect("left interval remains present") = value;
            }
            (_, Some((right_start, right_end)), false, true) => {
                self.occupied.remove(&right_start);
                self.occupied.insert(value, right_end);
            }
            _ => {
                self.occupied.insert(value, value);
            }
        }
        self.occupied_count += 1;
    }

    fn first_free_at_or_after(&self, candidate: u64) -> Option<u64> {
        let containing = self
            .occupied
            .range(..=candidate)
            .next_back()
            .map(|(&start, &end)| (start, end));
        match containing {
            Some((_, end)) if candidate <= end => {
                end.checked_add(1).filter(|&next| next <= self.high)
            }
            _ => Some(candidate),
        }
    }

    fn allocate_from(&mut self, start: u64) -> Option<u64> {
        if self.occupied_count >= self.domain_len() {
            return None;
        }
        let start = start.clamp(self.low, self.high);
        let candidate = self
            .first_free_at_or_after(start)
            .or_else(|| self.first_free_at_or_after(self.low))?;
        self.occupy(candidate);
        Some(candidate)
    }
}

fn synthetic_pointer_slot(value: i64, slot_high: u64) -> Option<u64> {
    let raw = value as u64;
    if raw & !SYNTHETIC_POINTER_LOW_MASK != SYNTHETIC_POINTER_HIGH {
        return None;
    }
    let slot = (raw & SYNTHETIC_POINTER_LOW_MASK) >> 3;
    (slot <= slot_high).then_some(slot)
}

fn synthetic_pointer_start(kind: NovelPointerKind, identity: &str, slot_high: u64) -> u64 {
    let hashed = stable_hash64(kind.hash_domain(), identity) >> 3;
    if slot_high == SYNTHETIC_POINTER_SLOT_HIGH {
        hashed & SYNTHETIC_POINTER_SLOT_HIGH
    } else {
        hashed % (slot_high + 1)
    }
}

fn synthetic_pointer_from_slot(slot: u64) -> i64 {
    (SYNTHETIC_POINTER_HIGH | (slot << 3)) as i64
}

fn allocate_synthetic_ptr(
    kind: NovelPointerKind,
    identity: &str,
    allocator: &mut SuccessorAllocator,
    domains: CanonicalAllocationDomains,
) -> Result<i64, RemapError> {
    // OldReference is an opaque serialized lookup key. Keep synthetic keys in a stable, positive
    // high range that real Win64 heap pointers do not occupy, then select the same next free slot
    // as the historical linear probe through the interval-backed successor allocator.
    // T1/T3/T5 and the derived T7 property keys share one runtime pointer map. T7 keys are
    // always odd; preserve real Win64 pointer alignment so synthetic keys cannot enter that
    // derived-key domain.
    let start = synthetic_pointer_start(kind, identity, domains.pointer_slot_high);
    allocator
        .allocate_from(start)
        .map(synthetic_pointer_from_slot)
        .ok_or(RemapError::KeySpaceExhausted {
            kind: "OldReference",
        })
}

fn allocate_new_pointer_keys(
    plan: &mut NewSymbolPlan,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    let mut symbols: Vec<(NovelPointerKind, String, i64)> = Vec::new();
    for &key in &plan.new_types {
        let identity =
            regen
                .type_id_of_ptr
                .get(&key)
                .cloned()
                .ok_or(RemapError::MissingNewRow {
                    kind: "type identity",
                    key,
                })?;
        symbols.push((NovelPointerKind::Type, identity, key));
    }
    for &key in &plan.new_funcs {
        let identity =
            regen
                .func_id_of_ptr
                .get(&key)
                .cloned()
                .ok_or(RemapError::MissingNewRow {
                    kind: "function identity",
                    key,
                })?;
        symbols.push((NovelPointerKind::Function, identity, key));
    }
    for &key in &plan.new_globals {
        let identity =
            regen
                .global_id_of_ptr
                .get(&key)
                .cloned()
                .ok_or(RemapError::MissingNewRow {
                    kind: "global identity",
                    key,
                })?;
        symbols.push((NovelPointerKind::Global, identity, key));
    }
    symbols.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));

    let mut allocator = SuccessorAllocator::new(
        0,
        PRODUCTION_ALLOCATION_DOMAINS.pointer_slot_high,
        base.all_ptr_keys.iter().filter_map(|&key| {
            synthetic_pointer_slot(key, PRODUCTION_ALLOCATION_DOMAINS.pointer_slot_high)
        }),
    );
    let mut string_global_assignments = HashMap::<String, i64>::new();
    for (kind, identity, raw) in symbols {
        if kind == NovelPointerKind::Global
            && regen
                .global_is_string_of_ptr
                .get(&raw)
                .copied()
                .unwrap_or(false)
        {
            if let Some(&assigned) = string_global_assignments.get(&identity) {
                plan.global_ptrs.insert(raw, assigned);
                continue;
            }
        }
        // Never retain a compiler-run-local raw OldReference merely because it is free in the
        // pristine base. Independent compiler runs recycle those first-free pointer values for
        // different symbols. Identity-derived allocation makes each mini converge on the same
        // key for the same symbol, independent of regen order or raw address assignment.
        let final_key = allocate_synthetic_ptr(
            kind,
            &identity,
            &mut allocator,
            PRODUCTION_ALLOCATION_DOMAINS,
        )?;
        match kind {
            NovelPointerKind::Type => {
                plan.type_ptrs.insert(raw, final_key);
            }
            NovelPointerKind::Function => {
                plan.func_ptrs.insert(raw, final_key);
            }
            NovelPointerKind::Global => {
                plan.global_ptrs.insert(raw, final_key);
                if regen
                    .global_is_string_of_ptr
                    .get(&raw)
                    .copied()
                    .unwrap_or(false)
                {
                    string_global_assignments.insert(identity, final_key);
                }
            }
        }
    }
    Ok(())
}

fn type_sequence_start(identity: &str, sequence_high: u32) -> u64 {
    let hashed = stable_hash64(3, identity) as u32 & TYPE_ID_SEQUENCE_MASK;
    if sequence_high == TYPE_ID_SEQUENCE_MASK {
        hashed.max(LAST_PRIMITIVE_TYPE_ID as u32 + 1) as u64
    } else {
        let low = LAST_PRIMITIVE_TYPE_ID as u32 + 1;
        u64::from(low + hashed % (sequence_high - low + 1))
    }
}

fn allocate_type_id(
    raw: i32,
    identity: &str,
    allocator: &mut SuccessorAllocator,
    domains: CanonicalAllocationDomains,
) -> Result<i32, RemapError> {
    // T2 stores an unqualified core id: object-kind bits plus the lower 26-bit sequence. Handle
    // qualifiers are operand-local and a qualified/negative T2 key is malformed.
    let bits = raw as u32;
    if !valid_type_id_core(raw) {
        return Err(RemapError::InvalidTailRow {
            table: 1,
            row_key: raw as i64,
            kind: "type-id key",
            detail: "T2 keys must be unqualified non-primitive core ids".to_owned(),
        });
    }
    let flags = bits & TYPE_ID_OBJECT_MASK;
    let start = type_sequence_start(identity, domains.type_sequence_high);
    let sequence = allocator
        .allocate_from(start)
        .ok_or(RemapError::KeySpaceExhausted { kind: "type-id" })?;
    Ok((flags | sequence as u32) as i32)
}

fn function_id_start(identity: &str, id_high: u64) -> u64 {
    let hashed = stable_hash64(4, identity) & i32::MAX as u64;
    if id_high == i32::MAX as u64 {
        hashed.max(1)
    } else {
        1 + hashed % id_high
    }
}

fn allocate_function_id(
    identity: &str,
    allocator: &mut SuccessorAllocator,
    domains: CanonicalAllocationDomains,
) -> Result<i32, RemapError> {
    // Serialized function ids are non-negative i32 lookup keys (0 is a sentinel in several
    // module-record arrays). Keep synthetic ids in the positive domain and select the same next
    // free id as the historical linear probe.
    let start = function_id_start(identity, domains.function_id_high);
    allocator
        .allocate_from(start)
        .map(|id| id as i32)
        .ok_or(RemapError::KeySpaceExhausted {
            kind: "function-id",
        })
}

fn allocate_module_function_id(
    identity: &str,
    allocator: &mut SuccessorAllocator,
    domains: CanonicalAllocationDomains,
) -> Result<i32, RemapError> {
    let hashed = stable_hash64(5, identity) & i32::MAX as u64;
    let start = if domains.function_id_high == i32::MAX as u64 {
        hashed.max(1)
    } else {
        1 + hashed % domains.function_id_high
    };
    allocator
        .allocate_from(start)
        .map(|id| id as i32)
        .ok_or(RemapError::KeySpaceExhausted {
            kind: "module Function.Id",
        })
}

fn type_id_allocator(
    object_kind: u32,
    base: &SymTables,
    domains: CanonicalAllocationDomains,
) -> SuccessorAllocator {
    SuccessorAllocator::new(
        u64::from(LAST_PRIMITIVE_TYPE_ID as u32 + 1),
        u64::from(domains.type_sequence_high),
        base.typeid_to_ptr.keys().filter_map(|&id| {
            let raw = id as u32;
            ((raw & TYPE_ID_OBJECT_MASK) == object_kind)
                .then_some(u64::from(raw & TYPE_ID_SEQUENCE_MASK))
        }),
    )
}

fn function_id_allocator(
    base: &SymTables,
    domains: CanonicalAllocationDomains,
) -> SuccessorAllocator {
    SuccessorAllocator::new(
        1,
        domains.function_id_high,
        base.funcid_to_ptr
            .keys()
            .filter_map(|&id| u64::try_from(id).ok().filter(|&id| id != 0)),
    )
}

fn allocate_engine_ids(
    plan: &mut NewSymbolPlan,
    meta: &TailMetadata,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    let mut type_allocators = BTreeMap::<u32, SuccessorAllocator>::new();
    let mut selected_type_rows = Vec::with_capacity(plan.new_types.len());
    for &ptr in &plan.new_types {
        match meta.type_id_by_ptr.get(&ptr) {
            Some(UniqueRowIndex::Unique(index)) => selected_type_rows.push(*index),
            Some(UniqueRowIndex::Ambiguous) => {
                return Err(RemapError::InvalidTailRow {
                    table: 1,
                    row_key: ptr,
                    kind: "reverse type-id mapping",
                    detail: "a new type pointer must have exactly one T2 row".to_owned(),
                });
            }
            None => {
                return Err(RemapError::MissingNewRow {
                    kind: "type-id",
                    key: ptr,
                });
            }
        }
    }
    // Preserve serialized T2 order so this resource fix does not change allocator collision
    // semantics; allocator redesign is a separate loadout-wide tranche.
    selected_type_rows.sort_unstable();
    for index in selected_type_rows {
        let row = &meta.type_ids[index];
        let identity = regen
            .type_id_of_ptr
            .get(&row.ptr)
            .ok_or(RemapError::MissingNewRow {
                kind: "type identity",
                key: row.ptr,
            })?;
        let object_kind = row.id as u32 & TYPE_ID_OBJECT_MASK;
        let allocator = type_allocators
            .entry(object_kind)
            .or_insert_with(|| type_id_allocator(object_kind, base, PRODUCTION_ALLOCATION_DOMAINS));
        let final_id =
            allocate_type_id(row.id, identity, allocator, PRODUCTION_ALLOCATION_DOMAINS)?;
        plan.type_ids.insert(row.id, final_id);
    }

    let mut func_allocator = function_id_allocator(base, PRODUCTION_ALLOCATION_DOMAINS);
    let mut selected_func_rows = Vec::with_capacity(plan.new_funcs.len());
    for &ptr in &plan.new_funcs {
        match meta.func_id_by_ptr.get(&ptr) {
            Some(UniqueRowIndex::Unique(index)) => selected_func_rows.push(*index),
            Some(UniqueRowIndex::Ambiguous) => {
                return Err(RemapError::InvalidTailRow {
                    table: 3,
                    row_key: ptr,
                    kind: "reverse function-id mapping",
                    detail: "a new function pointer must have exactly one T4 row".to_owned(),
                });
            }
            None => {
                return Err(RemapError::MissingNewRow {
                    kind: "function-id",
                    key: ptr,
                });
            }
        }
    }
    selected_func_rows.sort_unstable();
    for index in selected_func_rows {
        let row = &meta.func_ids[index];
        let identity = regen
            .func_id_of_ptr
            .get(&row.ptr)
            .ok_or(RemapError::MissingNewRow {
                kind: "function identity",
                key: row.ptr,
            })?;
        let final_id =
            allocate_function_id(identity, &mut func_allocator, PRODUCTION_ALLOCATION_DOMAINS)?;
        plan.func_ids.insert(row.id, final_id);
    }
    Ok(())
}

#[allow(dead_code)]
fn unique_type_id_row<'a>(
    meta: &'a TailMetadata,
    ptr: i64,
) -> Result<&'a IdPtrRowMeta, RemapError> {
    match meta.type_id_by_ptr.get(&ptr) {
        Some(UniqueRowIndex::Unique(index)) => Ok(&meta.type_ids[*index]),
        Some(UniqueRowIndex::Ambiguous) => Err(RemapError::InvalidTailRow {
            table: 1,
            row_key: ptr,
            kind: "reverse type-id mapping",
            detail: "a new type pointer must have exactly one T2 row".to_owned(),
        }),
        None => Err(RemapError::MissingNewRow {
            kind: "type-id",
            key: ptr,
        }),
    }
}

#[allow(dead_code)]
fn unique_function_id_row<'a>(
    meta: &'a TailMetadata,
    ptr: i64,
) -> Result<&'a IdPtrRowMeta, RemapError> {
    match meta.func_id_by_ptr.get(&ptr) {
        Some(UniqueRowIndex::Unique(index)) => Ok(&meta.func_ids[*index]),
        Some(UniqueRowIndex::Ambiguous) => Err(RemapError::InvalidTailRow {
            table: 3,
            row_key: ptr,
            kind: "reverse function-id mapping",
            detail: "a new function pointer must have exactly one T4 row".to_owned(),
        }),
        None => Err(RemapError::MissingNewRow {
            kind: "function-id",
            key: ptr,
        }),
    }
}

#[allow(dead_code)]
fn novel_identity_set(
    plan: &NewSymbolPlan,
    meta: &TailMetadata,
    regen: &SymTables,
    spans: &ModuleSpans,
) -> Result<NovelIdentitySet, RemapError> {
    let mut identities = NovelIdentitySet::default();
    for &ptr in &plan.new_types {
        let identity =
            regen
                .type_id_of_ptr
                .get(&ptr)
                .cloned()
                .ok_or(RemapError::MissingNewRow {
                    kind: "type identity",
                    key: ptr,
                })?;
        let row = unique_type_id_row(meta, ptr)?;
        if !valid_type_id_core(row.id) {
            return Err(RemapError::InvalidTailRow {
                table: 1,
                row_key: row.id as i64,
                kind: "type-id key",
                detail: "T2 keys must be unqualified non-primitive core ids".to_owned(),
            });
        }
        identities.pointers.insert(NovelPointerIdentity {
            kind: NovelPointerKind::Type,
            identity: identity.clone(),
        });
        let object_kind = row.id as u32 & TYPE_ID_OBJECT_MASK;
        if let Some(first) = identities.type_ids.insert(identity.clone(), object_kind) {
            if first != object_kind {
                return Err(RemapError::LoadoutPlanTypeKindConflict {
                    identity,
                    first,
                    second: object_kind,
                });
            }
        }
    }
    for &ptr in &plan.new_funcs {
        let identity =
            regen
                .func_id_of_ptr
                .get(&ptr)
                .cloned()
                .ok_or(RemapError::MissingNewRow {
                    kind: "function identity",
                    key: ptr,
                })?;
        unique_function_id_row(meta, ptr)?;
        identities.pointers.insert(NovelPointerIdentity {
            kind: NovelPointerKind::Function,
            identity: identity.clone(),
        });
        identities.function_ids.insert(identity);
    }
    for &ptr in &plan.new_globals {
        let identity =
            regen
                .global_id_of_ptr
                .get(&ptr)
                .cloned()
                .ok_or(RemapError::MissingNewRow {
                    kind: "global identity",
                    key: ptr,
                })?;
        identities.pointers.insert(NovelPointerIdentity {
            kind: NovelPointerKind::Global,
            identity,
        });
    }
    identities.module_function_ids.extend(
        spans
            .function_id_sites
            .iter()
            .map(|site| site.identity.clone()),
    );
    Ok(identities)
}

#[allow(dead_code)]
fn allocate_loadout_assignments(
    base: &AllowNewBaseContext,
    inventory: NovelIdentitySet,
    domains: CanonicalAllocationDomains,
) -> Result<
    (
        BTreeMap<NovelPointerIdentity, i64>,
        BTreeMap<NovelTypeIdIdentity, i32>,
        BTreeMap<String, i32>,
        BTreeMap<String, i32>,
    ),
    RemapError,
> {
    if domains.type_sequence_high < LAST_PRIMITIVE_TYPE_ID as u32 + 1 {
        return Err(RemapError::KeySpaceExhausted { kind: "type-id" });
    }
    if domains.function_id_high == 0 {
        return Err(RemapError::KeySpaceExhausted {
            kind: "function-id",
        });
    }

    let NovelIdentitySet {
        pointers,
        type_ids,
        function_ids,
        module_function_ids,
    } = inventory;

    let mut pointer_allocator = SuccessorAllocator::new(
        0,
        domains.pointer_slot_high,
        base.syms
            .all_ptr_keys
            .iter()
            .filter_map(|&key| synthetic_pointer_slot(key, domains.pointer_slot_high)),
    );
    let mut pointer_assignments = BTreeMap::new();
    for identity in pointers {
        let assigned = allocate_synthetic_ptr(
            identity.kind,
            &identity.identity,
            &mut pointer_allocator,
            domains,
        )?;
        pointer_assignments.insert(identity, assigned);
    }

    let mut type_allocators = BTreeMap::<u32, SuccessorAllocator>::new();
    let mut type_id_assignments = BTreeMap::new();
    let mut type_identities: Vec<NovelTypeIdIdentity> = type_ids
        .into_iter()
        .map(|(identity, object_kind)| NovelTypeIdIdentity {
            object_kind,
            identity,
        })
        .collect();
    type_identities.sort();
    for identity in type_identities {
        let allocator = type_allocators
            .entry(identity.object_kind)
            .or_insert_with(|| type_id_allocator(identity.object_kind, &base.syms, domains));
        let raw = (identity.object_kind | (LAST_PRIMITIVE_TYPE_ID as u32 + 1)) as i32;
        let assigned = allocate_type_id(raw, &identity.identity, allocator, domains)?;
        type_id_assignments.insert(identity, assigned);
    }

    let mut function_allocator = function_id_allocator(&base.syms, domains);
    let mut function_id_assignments = BTreeMap::new();
    for identity in function_ids {
        let assigned = allocate_function_id(&identity, &mut function_allocator, domains)?;
        function_id_assignments.insert(identity, assigned);
    }

    // Module-record Function.Id is a separate AngelScript engine namespace from T4. Shipping has
    // more than 100k declared module ids whose values have no T4 row, so allocate it separately
    // while preserving an exact pristine declaration when editing a module.
    let mut module_function_allocator = SuccessorAllocator::new(
        1,
        domains.function_id_high,
        base.occupied_module_function_ids
            .iter()
            .filter_map(|&id| u64::try_from(id).ok().filter(|&id| id != 0)),
    );
    let mut module_function_id_assignments = BTreeMap::new();
    for identity in module_function_ids {
        let assigned = base
            .module_function_ids
            .get(&identity)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| {
                allocate_module_function_id(&identity, &mut module_function_allocator, domains)
            })?;
        module_function_id_assignments.insert(identity, assigned);
    }

    Ok((
        pointer_assignments,
        type_id_assignments,
        function_id_assignments,
        module_function_id_assignments,
    ))
}

#[allow(dead_code)]
fn loadout_missing_assignment(kind: &'static str, identity: &str) -> RemapError {
    RemapError::LoadoutPlanMissingAssignment {
        kind,
        identity: identity.to_owned(),
    }
}

#[allow(dead_code)]
fn apply_loadout_assignments(
    local: &mut NewSymbolPlan,
    meta: &TailMetadata,
    regen: &SymTables,
    spans: &ModuleSpans,
    loadout: &LoadoutScriptIdPlan,
) -> Result<(), RemapError> {
    for &raw in &local.new_types {
        let identity = regen
            .type_id_of_ptr
            .get(&raw)
            .ok_or(RemapError::MissingNewRow {
                kind: "type identity",
                key: raw,
            })?;
        let pointer_identity = NovelPointerIdentity {
            kind: NovelPointerKind::Type,
            identity: identity.clone(),
        };
        let pointer = loadout
            .pointer_assignments
            .get(&pointer_identity)
            .copied()
            .ok_or_else(|| loadout_missing_assignment("type pointer", identity))?;
        local.type_ptrs.insert(raw, pointer);

        let row = unique_type_id_row(meta, raw)?;
        let type_identity = NovelTypeIdIdentity {
            object_kind: row.id as u32 & TYPE_ID_OBJECT_MASK,
            identity: identity.clone(),
        };
        let type_id = loadout
            .type_id_assignments
            .get(&type_identity)
            .copied()
            .ok_or_else(|| loadout_missing_assignment("type-id", identity))?;
        local.type_ids.insert(row.id, type_id);
    }

    for &raw in &local.new_funcs {
        let identity = regen
            .func_id_of_ptr
            .get(&raw)
            .ok_or(RemapError::MissingNewRow {
                kind: "function identity",
                key: raw,
            })?;
        let pointer_identity = NovelPointerIdentity {
            kind: NovelPointerKind::Function,
            identity: identity.clone(),
        };
        let pointer = loadout
            .pointer_assignments
            .get(&pointer_identity)
            .copied()
            .ok_or_else(|| loadout_missing_assignment("function pointer", identity))?;
        local.func_ptrs.insert(raw, pointer);

        let row = unique_function_id_row(meta, raw)?;
        let function_id = loadout
            .function_id_assignments
            .get(identity)
            .copied()
            .ok_or_else(|| loadout_missing_assignment("function-id", identity))?;
        local.func_ids.insert(row.id, function_id);
    }

    for &raw in &local.new_globals {
        let identity = regen
            .global_id_of_ptr
            .get(&raw)
            .ok_or(RemapError::MissingNewRow {
                kind: "global identity",
                key: raw,
            })?;
        let pointer_identity = NovelPointerIdentity {
            kind: NovelPointerKind::Global,
            identity: identity.clone(),
        };
        let pointer = loadout
            .pointer_assignments
            .get(&pointer_identity)
            .copied()
            .ok_or_else(|| loadout_missing_assignment("global pointer", identity))?;
        local.global_ptrs.insert(raw, pointer);
    }
    for site in &spans.function_id_sites {
        let assigned = loadout
            .module_function_id_assignments
            .get(&site.identity)
            .copied()
            .ok_or_else(|| loadout_missing_assignment("module Function.Id", &site.identity))?;
        local
            .module_function_ids
            .insert(site.identity.clone(), assigned);
    }
    Ok(())
}

fn mapped_type_ptr(plan: &NewSymbolPlan, key: i64) -> Result<i64, RemapError> {
    if key == 0 {
        return Ok(0);
    }
    plan.type_ptrs
        .get(&key)
        .copied()
        .ok_or(RemapError::MissingNewRow {
            kind: "mapped type",
            key,
        })
}

fn mapped_func_ptr(plan: &NewSymbolPlan, key: i64) -> Result<i64, RemapError> {
    plan.func_ptrs
        .get(&key)
        .copied()
        .ok_or(RemapError::MissingNewRow {
            kind: "mapped function",
            key,
        })
}

fn mapped_global_ptr(plan: &NewSymbolPlan, key: i64) -> Result<i64, RemapError> {
    plan.global_ptrs
        .get(&key)
        .copied()
        .ok_or(RemapError::MissingNewRow {
            kind: "mapped global",
            key,
        })
}

fn mapped_type_id(
    plan: &NewSymbolPlan,
    raw: i32,
    regen: &SymTables,
    base: &SymTables,
) -> Result<i32, RemapError> {
    let (core, flags) = split_type_id_operand(raw);
    if let Some(&id) = plan.type_ids.get(&core) {
        return Ok(apply_type_id_operand_flags(id, flags));
    }
    let Some(&regen_ptr) = regen.typeid_to_ptr.get(&core) else {
        return Ok(raw); // primitive / non-reference
    };
    let final_ptr = mapped_type_ptr(plan, regen_ptr)?;
    if base.typeid_to_ptr.get(&core) == Some(&final_ptr) {
        return Ok(apply_type_id_operand_flags(core, flags));
    }
    let final_core = base
        .ptr_kind_to_typeid
        .get(&(final_ptr, core as u32 & TYPE_ID_OBJECT_MASK))
        .copied()
        .ok_or(RemapError::MissingNewRow {
            kind: "base type-id",
            key: final_ptr,
        })?;
    Ok(apply_type_id_operand_flags(final_core, flags))
}

fn mapped_func_id(
    plan: &NewSymbolPlan,
    raw: i32,
    regen: &SymTables,
    base: &SymTables,
) -> Result<i32, RemapError> {
    if raw == 0 {
        return Ok(0); // null reference sentinel; a real T4 definition may still use key zero.
    }
    if let Some(&id) = plan.func_ids.get(&raw) {
        return Ok(id);
    }
    let Some(&regen_ptr) = regen.funcid_to_ptr.get(&raw) else {
        return Ok(raw); // sentinel / behavior tag
    };
    let final_ptr = mapped_func_ptr(plan, regen_ptr)?;
    if base.funcid_to_ptr.get(&raw) == Some(&final_ptr) {
        return Ok(raw);
    }
    base.ptr_to_funcid
        .get(&final_ptr)
        .filter(|&&id| id != 0)
        .copied()
        .ok_or(RemapError::MissingNewRow {
            kind: "base function-id",
            key: final_ptr,
        })
}

fn resolve_embedded_regen_func_id(raw: i64, regen: &SymTables) -> Result<(i32, i64), RemapError> {
    let id = i32::try_from(raw).map_err(|_| RemapError::UnresolvedEffectiveReference {
        kind: "embedded function id",
        op: "Factory/BehaviorRefs",
        key: raw,
    })?;
    let ptr =
        regen
            .funcid_to_ptr
            .get(&id)
            .copied()
            .ok_or(RemapError::UnresolvedEffectiveReference {
                kind: "embedded function id",
                op: "Factory/BehaviorRefs",
                key: raw,
            })?;
    Ok((id, ptr))
}

fn resolve_embedded_effective_func_id(
    raw: i64,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(i32, i64), RemapError> {
    let id = i32::try_from(raw).map_err(|_| RemapError::UnresolvedEffectiveReference {
        kind: "embedded function id",
        op: "Factory/BehaviorRefs",
        key: raw,
    })?;
    let ptr = regen
        .funcid_to_ptr
        .get(&id)
        .or_else(|| base.funcid_to_ptr.get(&id))
        .copied()
        .ok_or(RemapError::UnresolvedEffectiveReference {
            kind: "embedded function id",
            op: "Factory/BehaviorRefs",
            key: raw,
        })?;
    Ok((id, ptr))
}

fn plan_static_names(
    plan: &mut NewSymbolPlan,
    regen: &TailMetadata,
    base: &TailMetadata,
) -> Result<(), RemapError> {
    let base_by_name: HashMap<&str, i64> = base
        .static_names
        .iter()
        .map(|row| (row.name.as_str(), row.index as i64))
        .collect();
    let mut new_by_name: HashMap<String, i64> = HashMap::new();
    let mut used: Vec<i64> = plan.used_static_indices.iter().copied().collect();
    used.sort_unstable();
    for raw in used {
        let row = usize::try_from(raw)
            .ok()
            .and_then(|i| regen.static_names.get(i))
            .ok_or(RemapError::MissingStaticName(raw))?;
        let final_index = if let Some(&base_index) = base_by_name.get(row.name.as_str()) {
            base_index
        } else if let Some(&selected) = new_by_name.get(&row.name) {
            selected
        } else {
            let index = base.static_names.len() as i64 + plan.selected_static_rows.len() as i64;
            plan.selected_static_rows.push(row.index);
            new_by_name.insert(row.name.clone(), index);
            index
        };
        plan.static_indices.insert(raw, final_index);
    }
    Ok(())
}

/// Prepared minis already encode their private StaticNames rows as absolute indices immediately
/// after the pristine pool. The loadout-wide second pass must therefore resolve
/// `base.static_names.len() + local_row`, not reinterpret that absolute operand as a local row in
/// the mini's compact T6 table.
fn plan_prepared_static_names(
    plan: &mut NewSymbolPlan,
    prepared: &TailMetadata,
    base: &TailMetadata,
) -> Result<(), RemapError> {
    let base_len = base.static_names.len() as i64;
    let base_by_name: HashMap<&str, i64> = base
        .static_names
        .iter()
        .map(|row| (row.name.as_str(), row.index as i64))
        .collect();
    let prepared_by_source: HashMap<i64, &StaticRowMeta> = prepared
        .static_names
        .iter()
        .map(|row| (base_len + row.index as i64, row))
        .collect();
    let mut new_by_name: HashMap<String, i64> = HashMap::new();
    let mut used: Vec<i64> = plan.used_static_indices.iter().copied().collect();
    used.sort_unstable();
    for raw in used {
        if raw >= 0 && raw < base_len {
            plan.static_indices.insert(raw, raw);
            continue;
        }
        let row = prepared_by_source
            .get(&raw)
            .copied()
            .ok_or(RemapError::MissingStaticName(raw))?;
        let final_index = if let Some(&base_index) = base_by_name.get(row.name.as_str()) {
            base_index
        } else if let Some(&selected) = new_by_name.get(&row.name) {
            selected
        } else {
            let index = base_len + plan.selected_static_rows.len() as i64;
            plan.selected_static_rows.push(row.index);
            new_by_name.insert(row.name.clone(), index);
            index
        };
        plan.static_indices.insert(raw, final_index);
    }
    Ok(())
}

fn property_key(type_id: i32, member_offset: i32) -> i64 {
    ((type_id as i64) << 1) | ((member_offset as i64) << 33) | 1
}

fn plan_properties(
    plan: &mut NewSymbolPlan,
    regen_meta: &TailMetadata,
    base_meta: &TailMetadata,
    targets: &HashSet<String>,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    let type_module: HashMap<i64, &str> = regen_meta
        .types
        .iter()
        .map(|row| (row.key, row.module.as_str()))
        .collect();
    let base_by_key: HashMap<i64, &PropertyRowMeta> = base_meta
        .properties
        .iter()
        .map(|row| (row.key, row))
        .collect();
    let mut selected_keys = HashSet::new();

    for row in &regen_meta.properties {
        let owner_ptr = regen.typeid_to_ptr.get(&row.old_type_id).copied();
        let owner_is_new = owner_ptr.is_some_and(|ptr| plan.new_types.contains(&ptr));
        let owner_is_target = owner_ptr
            .and_then(|ptr| type_module.get(&ptr))
            .is_some_and(|module| targets.contains(*module));
        let property_is_used = plan
            .used_property_sites
            .contains(&(row.old_type_id, row.member_offset));
        if !owner_is_new && !owner_is_target && !property_is_used {
            continue;
        }
        let final_id = mapped_type_id(plan, row.old_type_id, regen, base)?;
        let final_key = property_key(final_id, row.member_offset);
        if let Some(existing) = base_by_key.get(&final_key) {
            if existing.name == row.name && existing.old_type_id == final_id {
                continue; // the vanilla row already describes this exact property.
            }
            return Err(RemapError::PropertyCollision {
                name: row.name.clone(),
                key: final_key,
            });
        }
        if !selected_keys.insert(final_key) {
            return Err(RemapError::PropertyCollision {
                name: row.name.clone(),
                key: final_key,
            });
        }
        plan.selected_properties.push(SelectedProperty {
            index: row.index,
            key: final_key,
            type_id: final_id,
        });
    }
    for &(owner_type_id, member_offset) in &plan.used_property_sites {
        let final_id = mapped_type_id(plan, owner_type_id, regen, base)?;
        let final_key = property_key(final_id, member_offset);
        if !base_by_key.contains_key(&final_key) && !selected_keys.contains(&final_key) {
            return Err(RemapError::MissingNewRow {
                kind: "property",
                key: property_key(owner_type_id, member_offset),
            });
        }
    }
    Ok(())
}

fn patch_bytecode_with_new_symbols(
    code: &mut [i32],
    plan: &NewSymbolPlan,
    regen: &SymTables,
    base: &SymTables,
) -> Result<RemapCounts, RemapError> {
    // Keep an immutable copy for look-ahead classification: call refs are rewritten below, but
    // PshC4 -> __STATIC_NAME recognition must use the original regen key/id.
    let original = code.to_vec();
    let instrs = disassemble(&original).map_err(|e| RemapError::Disasm(e.to_string()))?;
    let mut counts = RemapCounts::default();

    for (pos, ins) in instrs.iter().enumerate() {
        if ins.op.name == "STR" {
            let raw = ((original[ins.offset_dw] as u32 >> 16) & 0xffff) as i64;
            if let Some(&mapped) = plan.static_indices.get(&raw) {
                let mapped = u16::try_from(mapped)
                    .map_err(|_| RemapError::StaticNameIndexOverflow(mapped))?;
                let low = code[ins.offset_dw] as u32 & 0x0000_ffff;
                code[ins.offset_dw] = (low | ((mapped as u32) << 16)) as i32;
            }
        } else if ins.op.name == "PshC4"
            && instrs
                .get(pos + 1)
                .and_then(|next| callee_name_from_effective(next, &original, regen, base))
                == Some("__STATIC_NAME")
        {
            let raw = original[ins.offset_dw + 1] as i64;
            if let Some(&mapped) = plan.static_indices.get(&raw) {
                code[ins.offset_dw + 1] = i32::try_from(mapped)
                    .map_err(|_| RemapError::StaticNameIndexOverflow(mapped))?;
            }
        }

        for site in ref_sites(ins.op.name) {
            let off = ins.offset_dw + site.dw_index;
            match site.kind {
                RefKind::GlobalPtr => {
                    let raw = read_qw(&original, off);
                    write_qw(code, off, mapped_global_ptr(plan, raw)?);
                    counts.global_ptr += 1;
                }
                RefKind::FuncPtr => {
                    let raw = read_qw(&original, off);
                    write_qw(code, off, mapped_func_ptr(plan, raw)?);
                    counts.func_ptr += 1;
                }
                RefKind::TypePtr => {
                    let raw = read_qw(&original, off);
                    write_qw(code, off, mapped_type_ptr(plan, raw)?);
                    counts.type_ptr += 1;
                }
                RefKind::FuncId => {
                    code[off] = mapped_func_id(plan, original[off], regen, base)?;
                    counts.func_id += usize::from(
                        original[off] != 0 && regen.funcid_to_ptr.contains_key(&original[off]),
                    );
                }
                RefKind::TypeId => {
                    code[off] = mapped_type_id(plan, original[off], regen, base)?;
                    let (core, _) = split_type_id_operand(original[off]);
                    counts.type_id += usize::from(regen.typeid_to_ptr.contains_key(&core));
                }
            }
        }
    }
    Ok(counts)
}

fn patch_i64_at(row: &mut [u8], row_start: usize, absolute: usize, value: i64) {
    let rel = absolute - row_start;
    row[rel..rel + 8].copy_from_slice(&value.to_le_bytes());
}

fn append_canonical_sia(out: &mut Vec<u8>, value: &str) -> Result<(), RemapError> {
    if value.is_empty() {
        out.extend_from_slice(&0i32.to_le_bytes());
        return Ok(());
    }
    let length = i32::try_from(value.len()).map_err(|_| WireError::BadLen {
        pos: 0,
        len: i64::MAX,
        field: "canonical string-global identity",
    })?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    Ok(())
}

fn append_canonical_string_global(
    out: &mut Vec<u8>,
    key: i64,
    value: &str,
) -> Result<(), RemapError> {
    out.extend_from_slice(&key.to_le_bytes());
    append_canonical_sia(out, value)?;
    append_canonical_sia(out, "")?; // runtime ignores Module for bIsString
    append_canonical_sia(out, "")?; // Namespace is not part of literal identity
    out.extend_from_slice(&1i32.to_le_bytes());
    Ok(())
}

fn emit_minimal_new_symbol_tail(
    source: &[u8],
    meta: &TailMetadata,
    plan: &NewSymbolPlan,
) -> Result<Vec<u8>, RemapError> {
    let mut out = Vec::new();

    let selected_types: Vec<&TypeRowMeta> = meta
        .types
        .iter()
        .filter(|row| plan.new_types.contains(&row.key))
        .collect();
    out.extend_from_slice(&(selected_types.len() as u32).to_le_bytes());
    for row in selected_types {
        let mut bytes = source[row.start..row.end].to_vec();
        patch_i64_at(
            &mut bytes,
            row.start,
            row.start,
            mapped_type_ptr(plan, row.key)?,
        );
        for &dep in &row.type_deps {
            patch_i64_at(
                &mut bytes,
                row.start,
                dep.off,
                mapped_type_ptr(plan, dep.ptr)?,
            );
        }
        out.extend_from_slice(&bytes);
    }

    let selected_type_ids: Vec<&IdPtrRowMeta> = meta
        .type_ids
        .iter()
        .filter(|row| plan.new_types.contains(&row.ptr))
        .collect();
    out.extend_from_slice(&(selected_type_ids.len() as u32).to_le_bytes());
    for row in selected_type_ids {
        let mut bytes = source[row.start..row.end].to_vec();
        bytes[..4].copy_from_slice(
            &plan
                .type_ids
                .get(&row.id)
                .copied()
                .ok_or(RemapError::MissingNewRow {
                    kind: "type-id mapping",
                    key: row.id as i64,
                })?
                .to_le_bytes(),
        );
        bytes[4..12].copy_from_slice(&mapped_type_ptr(plan, row.ptr)?.to_le_bytes());
        out.extend_from_slice(&bytes);
    }

    let selected_funcs: Vec<&FuncRowMeta> = meta
        .funcs
        .iter()
        .filter(|row| plan.new_funcs.contains(&row.key))
        .collect();
    out.extend_from_slice(&(selected_funcs.len() as u32).to_le_bytes());
    for row in selected_funcs {
        let mut bytes = source[row.start..row.end].to_vec();
        patch_i64_at(
            &mut bytes,
            row.start,
            row.start,
            mapped_func_ptr(plan, row.key)?,
        );
        patch_i64_at(
            &mut bytes,
            row.start,
            row.owner_dep.0,
            mapped_type_ptr(plan, row.owner_dep.1)?,
        );
        for &dep in &row.type_deps {
            patch_i64_at(
                &mut bytes,
                row.start,
                dep.off,
                mapped_type_ptr(plan, dep.ptr)?,
            );
        }
        out.extend_from_slice(&bytes);
    }

    let selected_func_ids: Vec<&IdPtrRowMeta> = meta
        .func_ids
        .iter()
        .filter(|row| plan.new_funcs.contains(&row.ptr))
        .collect();
    out.extend_from_slice(&(selected_func_ids.len() as u32).to_le_bytes());
    for row in selected_func_ids {
        let mut bytes = source[row.start..row.end].to_vec();
        bytes[..4].copy_from_slice(
            &plan
                .func_ids
                .get(&row.id)
                .copied()
                .ok_or(RemapError::MissingNewRow {
                    kind: "function-id mapping",
                    key: row.id as i64,
                })?
                .to_le_bytes(),
        );
        bytes[4..12].copy_from_slice(&mapped_func_ptr(plan, row.ptr)?.to_le_bytes());
        out.extend_from_slice(&bytes);
    }

    let mut selected_globals = Vec::<(&GlobalRowMeta, i64)>::new();
    let mut selected_string_keys = HashSet::new();
    for row in &meta.globals {
        if !plan.new_globals.contains(&row.key) {
            continue;
        }
        let key = mapped_global_ptr(plan, row.key)?;
        if row.is_string && !selected_string_keys.insert(key) {
            continue;
        }
        selected_globals.push((row, key));
    }
    out.extend_from_slice(&(selected_globals.len() as u32).to_le_bytes());
    for (row, key) in selected_globals {
        if row.is_string {
            append_canonical_string_global(&mut out, key, &row.name)?;
            continue;
        }
        let mut bytes = source[row.start..row.end].to_vec();
        patch_i64_at(&mut bytes, row.start, row.start, key);
        out.extend_from_slice(&bytes);
    }

    let selected_static: HashSet<usize> = plan.selected_static_rows.iter().copied().collect();
    let static_rows: Vec<&StaticRowMeta> = meta
        .static_names
        .iter()
        .filter(|row| selected_static.contains(&row.index))
        .collect();
    out.extend_from_slice(&(static_rows.len() as u32).to_le_bytes());
    for row in static_rows {
        out.extend_from_slice(&source[row.start..row.end]);
    }

    let selected_properties: HashMap<usize, &SelectedProperty> = plan
        .selected_properties
        .iter()
        .map(|p| (p.index, p))
        .collect();
    let property_rows: Vec<(&PropertyRowMeta, &SelectedProperty)> = meta
        .properties
        .iter()
        .filter_map(|row| {
            selected_properties
                .get(&row.index)
                .map(|selected| (row, *selected))
        })
        .collect();
    out.extend_from_slice(&(property_rows.len() as u32).to_le_bytes());
    for (row, selected) in property_rows {
        let mut bytes = source[row.start..row.end].to_vec();
        bytes[..8].copy_from_slice(&selected.key.to_le_bytes());
        let id_pos = bytes.len() - 4;
        bytes[id_pos..].copy_from_slice(&selected.type_id.to_le_bytes());
        out.extend_from_slice(&bytes);
    }

    Ok(out)
}

struct AllowNewBaseContext {
    syms: SymTables,
    meta: TailMetadata,
    identity_summaries: SymbolIdentitySummaries,
    module_authorities: HashSet<String>,
    declarations: PristineDeclarationAuthority,
    module_function_ids: HashMap<String, i32>,
    occupied_module_function_ids: HashSet<i32>,
    source_bytes: usize,
}

fn build_allow_new_base_context(base: &[u8]) -> Result<AllowNewBaseContext, RemapError> {
    let syms = SymTables::build(base)?;
    let meta = TailMetadata::build(base)?;
    let identity_summaries = SymbolIdentitySummaries {
        types: IdentityReverseSummary::build(&syms.type_ident_of_ptr)?,
        functions: IdentityReverseSummary::build(&syms.func_ident_of_ptr)?,
        globals: IdentityReverseSummary::build_filtered(&syms.global_ident_of_ptr, |key| {
            !syms
                .global_is_string_of_ptr
                .get(&key)
                .copied()
                .unwrap_or(false)
        })?,
    };
    let module_authorities = inner_module_names(base)?;
    let mut comparison_budget =
        IdentityComparisonBudget::new(base.len().saturating_add(syms.identity_bytes));
    let collected =
        collect_declaration_inventory(base, &syms, None, None, &meta, &mut comparison_budget)?;
    let CollectedDeclarationInventory {
        declarations: declaration_inventory,
        function_id_sites,
    } = collected;
    let mut module_function_ids = HashMap::new();
    let mut occupied_module_function_ids = HashSet::new();
    for site in function_id_sites {
        let id = i32::from_le_bytes(base[site.byte_off..site.byte_off + 4].try_into().unwrap());
        module_function_ids.insert(site.identity, id);
        occupied_module_function_ids.insert(id);
    }
    let declarations = PristineDeclarationAuthority::build(
        &meta,
        &syms,
        declaration_inventory,
        &mut comparison_budget,
    )?;
    Ok(AllowNewBaseContext {
        syms,
        meta,
        identity_summaries,
        module_authorities,
        declarations,
        module_function_ids,
        occupied_module_function_ids,
        source_bytes: base.len(),
    })
}

struct AnalyzedNewSymbolMini {
    regen: SymTables,
    meta: TailMetadata,
    spans: ModuleSpans,
    targets: HashSet<String>,
    current_declarations: DeclarationInventory,
    plan: NewSymbolPlan,
    comparison_budget: IdentityComparisonBudget,
}

fn analyze_new_symbol_mini(
    extracted_mini: &[u8],
    base: &AllowNewBaseContext,
) -> Result<AnalyzedNewSymbolMini, RemapError> {
    let total_source_bytes = base.source_bytes.checked_add(extracted_mini.len()).ok_or(
        WireError::IdentityBudgetExceeded {
            max: MAX_IDENTITY_BUDGET,
        },
    )?;
    let regen = SymTables::build_with_type_fallback_and_budget(
        extracted_mini,
        &base.syms,
        total_source_bytes,
        base.syms.identity_bytes,
    )?;
    let meta = TailMetadata::build(extracted_mini)?;
    let mut comparison_budget = IdentityComparisonBudget::new(
        total_source_bytes
            .saturating_add(regen.identity_bytes)
            .saturating_add(base.declarations.declarations.bytes),
    );
    let collected = collect_declaration_inventory(
        extracted_mini,
        &regen,
        Some(&base.syms),
        Some(&base.declarations.script_owners),
        &meta,
        &mut comparison_budget,
    )?;
    let current_declarations = collected.declarations;
    let mut spans = collect_module_spans(extracted_mini)?;
    spans.function_id_sites = collected.function_id_sites;
    let targets = target_module_names(extracted_mini)?;
    let mut plan = NewSymbolPlan::default();

    // A declaration can be new even when no bytecode calls it yet. Seed every row declared by the
    // edited module, then add all directly referenced symbols and recursively close type deps.
    seed_target_module_symbols(
        &mut plan,
        &meta,
        &targets,
        &regen,
        &base.syms,
        &base.identity_summaries,
        &mut comparison_budget,
    )?;
    for span in &spans.code {
        let code: Vec<i32> = (0..span.count)
            .map(|k| {
                let off = span.data_off + k * 4;
                i32::from_le_bytes(extracted_mini[off..off + 4].try_into().unwrap())
            })
            .collect();
        analyze_bytecode_for_new_symbols(
            &code,
            &mut plan,
            &regen,
            &base.syms,
            &base.identity_summaries,
            &mut comparison_budget,
        )?;
    }
    for embed in &spans.embeds {
        let raw = i64::from_le_bytes(
            extracted_mini[embed.byte_off..embed.byte_off + 8]
                .try_into()
                .unwrap(),
        );
        if raw == 0 {
            continue;
        }
        match embed.kind {
            EmbedKind::TypePtr(_) => declare_type(
                &mut plan,
                raw,
                "embedded DataType",
                &regen,
                &base.syms,
                &base.identity_summaries,
                &mut comparison_budget,
            )?,
            EmbedKind::FuncId => {
                let (_, ptr) = resolve_embedded_effective_func_id(raw, &regen, &base.syms)?;
                declare_func(
                    &mut plan,
                    ptr,
                    "Factory/BehaviorRefs",
                    &regen,
                    &base.syms,
                    &base.identity_summaries,
                    &mut comparison_budget,
                )?;
            }
        }
    }
    close_type_dependencies(
        &mut plan,
        &meta,
        &regen,
        &base.syms,
        &base.identity_summaries,
        &mut comparison_budget,
    )?;
    validate_novel_declaration_membership(
        &meta,
        &regen,
        &base.syms,
        &base.declarations,
        &current_declarations,
        |module| base.module_authorities.contains(module) || targets.contains(module),
        |key| plan.new_types.contains(&key),
        |key| plan.new_funcs.contains(&key),
        |key| plan.new_globals.contains(&key),
        &mut comparison_budget,
    )?;
    Ok(AnalyzedNewSymbolMini {
        regen,
        meta,
        spans,
        targets,
        current_declarations,
        plan,
        comparison_budget,
    })
}

fn finish_new_symbol_remap(
    extracted_mini: &[u8],
    pristine_header: &[u8],
    base: &AllowNewBaseContext,
    analyzed: AnalyzedNewSymbolMini,
    prepared_static_names: bool,
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
    let AnalyzedNewSymbolMini {
        regen,
        meta,
        spans,
        targets,
        current_declarations,
        mut plan,
        mut comparison_budget,
    } = analyzed;

    if prepared_static_names {
        plan_prepared_static_names(&mut plan, &meta, &base.meta)?;
    } else {
        plan_static_names(&mut plan, &meta, &base.meta)?;
    }
    plan_properties(&mut plan, &meta, &base.meta, &targets, &regen, &base.syms)?;
    let selected_property_indices: HashSet<usize> = plan
        .selected_properties
        .iter()
        .map(|row| row.index)
        .collect();
    validate_novel_property_membership(
        &meta,
        &regen,
        &base.syms,
        &base.declarations,
        &current_declarations,
        |row| {
            selected_property_indices.contains(&row.index)
                && !has_pristine_property_identity(row, &regen, &base.syms, &base.declarations)
        },
        &mut comparison_budget,
    )?;

    let mod_start = CacheHeader::SIZE;
    let mod_end = module_region_end(extracted_mini)?;
    let mut module_bytes = extracted_mini[mod_start..mod_end].to_vec();
    let mut total = RemapCounts::default();
    for site in &spans.function_id_sites {
        let Some(&assigned) = plan.module_function_ids.get(&site.identity) else {
            continue;
        };
        let rel = site.byte_off - mod_start;
        module_bytes[rel..rel + 4].copy_from_slice(&assigned.to_le_bytes());
    }
    for span in &spans.code {
        let rel = span.data_off - mod_start;
        let mut code: Vec<i32> = (0..span.count)
            .map(|k| {
                let off = rel + k * 4;
                i32::from_le_bytes(module_bytes[off..off + 4].try_into().unwrap())
            })
            .collect();
        let counts = patch_bytecode_with_new_symbols(&mut code, &plan, &regen, &base.syms)?;
        total.add(&counts);
        for (k, &dw) in code.iter().enumerate() {
            let off = rel + k * 4;
            module_bytes[off..off + 4].copy_from_slice(&dw.to_le_bytes());
        }
    }

    for embed in &spans.embeds {
        let rel = embed.byte_off - mod_start;
        let raw = i64::from_le_bytes(module_bytes[rel..rel + 8].try_into().unwrap());
        if raw == 0 {
            continue;
        }
        match embed.kind {
            EmbedKind::TypePtr(_) => {
                module_bytes[rel..rel + 8]
                    .copy_from_slice(&mapped_type_ptr(&plan, raw)?.to_le_bytes());
                total.embed_type_ptr += 1;
            }
            EmbedKind::FuncId => {
                let (raw_id, _) = resolve_embedded_effective_func_id(raw, &regen, &base.syms)?;
                let mapped = mapped_func_id(&plan, raw_id, &regen, &base.syms)?;
                module_bytes[rel..rel + 8].copy_from_slice(&i64::from(mapped).to_le_bytes());
                total.embed_func_id += 1;
            }
        }
    }

    // Preserve the hard invariant. Only raw keys of declared-new symbols that remained
    // collision-free are allowed to survive; re-keyed symbols must use their replacement key.
    let mut allowed_new_raw = HashSet::new();
    for &raw in &plan.new_types {
        if plan.type_ptrs.get(&raw) == Some(&raw) {
            allowed_new_raw.insert(raw);
        }
    }
    for &raw in &plan.new_funcs {
        if plan.func_ptrs.get(&raw) == Some(&raw) {
            allowed_new_raw.insert(raw);
        }
    }
    for &raw in &plan.new_globals {
        if plan.global_ptrs.get(&raw) == Some(&raw) {
            allowed_new_raw.insert(raw);
        }
    }
    let surviving: Vec<SurvivingKey> = scan_surviving_regen_keys(&module_bytes, &regen, &base.syms)
        .into_iter()
        .filter(|hit| !allowed_new_raw.contains(&hit.value))
        .collect();
    if !surviving.is_empty() {
        let shown = surviving.len().min(12);
        let detail = surviving[..shown]
            .iter()
            .map(|s| format!("@+{:#x}={:#x} ({})", s.byte_off, s.value, s.name))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RemapError::SurvivingRegenKeys {
            n: surviving.len(),
            shown,
            detail,
        });
    }

    let tail = emit_minimal_new_symbol_tail(extracted_mini, &meta, &plan)?;
    let mut out = Vec::with_capacity(CacheHeader::SIZE + module_bytes.len() + tail.len());
    // A prepared mini is generation-bound to the exact target cache. This also makes the
    // low-level extract-remap output directly consumable by the strict sequential guard.
    out.extend_from_slice(&pristine_header[..0x14]);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&module_bytes);
    out.extend_from_slice(&tail);
    Ok((out, total))
}

#[allow(dead_code)]
fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[allow(dead_code)]
fn loadout_cache_header(bytes: &[u8], artifact: &'static str) -> Result<CacheHeader, RemapError> {
    CacheHeader::parse(bytes).map_err(|error| RemapError::LoadoutPlanInvalidHeader {
        artifact,
        detail: error.to_string(),
    })
}

#[allow(dead_code)]
impl LoadoutScriptIdPlanBuilder {
    /// Parse and retain the pristine base context exactly once. Mini bytes are never retained by
    /// the builder and may be dropped immediately after [`Self::inspect`] returns.
    pub(super) fn new(pristine_base: &[u8]) -> Result<Self, RemapError> {
        Self::new_with_config(
            pristine_base,
            PRODUCTION_LOADOUT_PLAN_LIMITS,
            PRODUCTION_ALLOCATION_DOMAINS,
        )
    }

    fn new_with_config(
        pristine_base: &[u8],
        limits: LoadoutPlanLimits,
        domains: CanonicalAllocationDomains,
    ) -> Result<Self, RemapError> {
        let header = loadout_cache_header(pristine_base, "pristine base")?;
        preflight_cache_module_work(pristine_base)?;
        let mut pristine_header = [0u8; 0x14];
        pristine_header.copy_from_slice(&pristine_base[..0x14]);
        let base = Arc::new(build_allow_new_base_context(pristine_base)?);
        let effective_base = Arc::new(EffectiveReferenceBase::from_allow_new_base(Arc::clone(
            &base,
        ))?);
        Ok(Self {
            pristine_base_sha256: sha256_bytes(pristine_base),
            pristine_guid: header.hash,
            pristine_header,
            base,
            effective_base,
            inspected_count: 0,
            inspected_minis: HashMap::new(),
            inventory: NovelIdentitySet::default(),
            assignment_entries: 0,
            identity_bytes: 0,
            limits,
            domains,
        })
    }

    /// Validate and add one mini atomically. No builder state changes until the complete semantic
    /// inspection and all cumulative resource checks have succeeded.
    pub(super) fn inspect(&mut self, mini: &[u8]) -> Result<(), RemapError> {
        let header = loadout_cache_header(mini, "mini")?;
        if header.hash != self.pristine_guid {
            return Err(RemapError::LoadoutPlanGuidMismatch {
                pristine: self.pristine_guid,
                mini: header.hash,
            });
        }
        if header.type_count != 1 {
            return Err(RemapError::NotSingle(header.type_count));
        }
        let next_minis =
            self.inspected_count
                .checked_add(1)
                .ok_or(RemapError::LoadoutPlanResourceLimit {
                    resource: "inspected minis",
                    actual: usize::MAX,
                    limit: self.limits.max_minis,
                })?;
        if next_minis > self.limits.max_minis {
            return Err(RemapError::LoadoutPlanResourceLimit {
                resource: "inspected minis",
                actual: next_minis,
                limit: self.limits.max_minis,
            });
        }
        let digest = sha256_bytes(mini);
        if self.inspected_minis.contains_key(&digest) {
            // Every manifest entry consumes one mini slot, while exact bytes only need semantic
            // inspection once. Commit the count only after all validation and limit checks pass.
            self.inspected_count = next_minis;
            return Ok(());
        }

        // Validate executable/module-record references against exactly pristine + this mini.
        // Prior inspected minis are deliberately absent: the assignment union cannot authorize
        // a cross-mini dependency.
        self.effective_base
            .validate(&EffectiveReferenceState::default(), mini)?;
        let analyzed = analyze_new_symbol_mini(mini, &self.base)?;
        let identities = novel_identity_set(
            &analyzed.plan,
            &analyzed.meta,
            &analyzed.regen,
            &analyzed.spans,
        )?;
        let fingerprint = identities.fingerprint();
        let (additional_entries, additional_bytes) =
            identities.additional_usage(&self.inventory, self.limits)?;
        let next_entries = self
            .assignment_entries
            .checked_add(additional_entries)
            .ok_or(RemapError::LoadoutPlanResourceLimit {
                resource: "novel assignments",
                actual: usize::MAX,
                limit: self.limits.max_assignments,
            })?;
        if next_entries > self.limits.max_assignments {
            return Err(RemapError::LoadoutPlanResourceLimit {
                resource: "novel assignments",
                actual: next_entries,
                limit: self.limits.max_assignments,
            });
        }
        let next_identity_bytes = self.identity_bytes.checked_add(additional_bytes).ok_or(
            RemapError::LoadoutPlanResourceLimit {
                resource: "identity bytes",
                actual: usize::MAX,
                limit: self.limits.max_identity_bytes,
            },
        )?;
        if next_identity_bytes > self.limits.max_identity_bytes {
            return Err(RemapError::LoadoutPlanResourceLimit {
                resource: "identity bytes",
                actual: next_identity_bytes,
                limit: self.limits.max_identity_bytes,
            });
        }

        identities.merge_into(&mut self.inventory);
        self.inspected_count = next_minis;
        self.inspected_minis.insert(digest, fingerprint);
        self.assignment_entries = next_entries;
        self.identity_bytes = next_identity_bytes;
        Ok(())
    }

    /// Freeze the stable identity union and allocate one canonical mapping for all inspected
    /// minis. The retained base context moves into the finished plan for O(1) reuse per rewrite.
    pub(super) fn finish(self) -> Result<LoadoutScriptIdPlan, RemapError> {
        let Self {
            pristine_base_sha256,
            pristine_guid,
            pristine_header,
            base,
            effective_base,
            inspected_minis,
            inventory,
            domains,
            ..
        } = self;
        let (
            pointer_assignments,
            type_id_assignments,
            function_id_assignments,
            module_function_id_assignments,
        ) = allocate_loadout_assignments(&base, inventory, domains)?;
        Ok(LoadoutScriptIdPlan {
            pristine_base_sha256,
            pristine_guid,
            pristine_header,
            base,
            effective_base,
            inspected_minis,
            pointer_assignments,
            type_id_assignments,
            function_id_assignments,
            module_function_id_assignments,
        })
    }
}

/// Convenience wrapper for small in-memory callers. Manager integration should use
/// [`LoadoutScriptIdPlanBuilder`] directly so each mini can be dropped after inspection.
#[allow(dead_code)]
pub(super) fn build_loadout_script_id_plan(
    pristine_base: &[u8],
    minis: &[&[u8]],
) -> Result<LoadoutScriptIdPlan, RemapError> {
    let mut builder = LoadoutScriptIdPlanBuilder::new(pristine_base)?;
    for &mini in minis {
        builder.inspect(mini)?;
    }
    builder.finish()
}

/// Apply a previously inspected loadout assignment to one exact mini. Both the pristine base and
/// mini bytes are SHA-bound; the independently recomputed portable identity set must also match
/// before any rewritten output is materialized.
#[allow(dead_code)]
pub(super) fn remap_module_to_base_with_loadout_plan(
    extracted_mini: &[u8],
    pristine_base: &[u8],
    loadout: &LoadoutScriptIdPlan,
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
    let base_header = loadout_cache_header(pristine_base, "pristine base")?;
    if base_header.hash != loadout.pristine_guid
        || sha256_bytes(pristine_base) != loadout.pristine_base_sha256
    {
        return Err(RemapError::LoadoutPlanBaseMismatch);
    }
    let mini_header = loadout_cache_header(extracted_mini, "mini")?;
    if mini_header.hash != loadout.pristine_guid {
        return Err(RemapError::LoadoutPlanGuidMismatch {
            pristine: loadout.pristine_guid,
            mini: mini_header.hash,
        });
    }
    if mini_header.type_count != 1 {
        return Err(RemapError::NotSingle(mini_header.type_count));
    }
    let bound_identity_fingerprint = loadout
        .inspected_minis
        .get(&sha256_bytes(extracted_mini))
        .ok_or(RemapError::LoadoutPlanMiniNotInspected)?;
    loadout
        .effective_base
        .validate(&EffectiveReferenceState::default(), extracted_mini)?;

    let base = &loadout.base;
    let mut analyzed = analyze_new_symbol_mini(extracted_mini, base)?;
    let actual_identities = novel_identity_set(
        &analyzed.plan,
        &analyzed.meta,
        &analyzed.regen,
        &analyzed.spans,
    )?;
    if actual_identities.fingerprint() != *bound_identity_fingerprint {
        return Err(RemapError::LoadoutPlanIdentityMismatch);
    }
    apply_loadout_assignments(
        &mut analyzed.plan,
        &analyzed.meta,
        &analyzed.regen,
        &analyzed.spans,
        loadout,
    )?;
    finish_new_symbol_remap(
        extracted_mini,
        &loadout.pristine_header,
        base,
        analyzed,
        true,
    )
}

fn remap_module_allow_new(
    extracted_mini: &[u8],
    base: &[u8],
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
    let mini_n = super::walk_modules::module_count(extracted_mini);
    if mini_n != 1 {
        return Err(RemapError::NotSingle(mini_n));
    }
    preflight_mini_module_work(extracted_mini)?;
    preflight_cache_module_work(base)?;

    let base_context = build_allow_new_base_context(base)?;
    let mut analyzed = analyze_new_symbol_mini(extracted_mini, &base_context)?;
    allocate_new_pointer_keys(&mut analyzed.plan, &analyzed.regen, &base_context.syms)?;
    allocate_engine_ids(
        &mut analyzed.plan,
        &analyzed.meta,
        &analyzed.regen,
        &base_context.syms,
    )?;
    finish_new_symbol_remap(extracted_mini, base, &base_context, analyzed, false)
}

/// Public entry: rewrite `extracted_mini`'s module bytecode refs to `base`'s keys, returning a
/// new 1-module mini whose tail tables are EMPTY (28 zero bytes). See module docs.
pub fn remap_module_to_base(
    extracted_mini: &[u8],
    base: &[u8],
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
    preflight_mini_module_work(extracted_mini)?;
    preflight_cache_module_work(base)?;
    let mini_n = super::walk_modules::module_count(extracted_mini);
    if mini_n != 1 {
        return Err(RemapError::NotSingle(mini_n));
    }

    let regen = SymTables::build(extracted_mini)?;
    let base_syms = SymTables::build(base)?;

    // The module entry occupies [CacheHeader::SIZE .. module_region_end]. Copy it out so we can
    // patch bytecode dwords in place, then emit header(count=1) + module + 28 zero bytes.
    let mod_start = CacheHeader::SIZE;
    let mod_end = module_region_end(extracted_mini)?;
    let mut module_bytes = extracted_mini[mod_start..mod_end].to_vec();

    // Spans are absolute offsets into `extracted_mini`; translate to module_bytes-relative.
    let spans = collect_module_spans(extracted_mini)?;
    let mut total = RemapCounts::default();
    for span in &spans.code {
        let rel = span.data_off - mod_start;
        // Read the bytecode dwords into a Vec<i32>.
        let mut code: Vec<i32> = (0..span.count)
            .map(|k| {
                let o = rel + k * 4;
                i32::from_le_bytes(module_bytes[o..o + 4].try_into().unwrap())
            })
            .collect();
        let counts = remap_bytecode(&mut code, &regen, &base_syms)?;
        total.add(&counts);
        // Write patched dwords back.
        for (k, &dw) in code.iter().enumerate() {
            let o = rel + k * 4;
            module_bytes[o..o + 4].copy_from_slice(&dw.to_le_bytes());
        }
    }

    // Embedded module-record int64 refs (outside the bytecode stream).
    for em in &spans.embeds {
        let o = em.byte_off - mod_start;
        let regen_key = i64::from_le_bytes(module_bytes[o..o + 8].try_into().unwrap());
        if regen_key == 0 {
            continue; // null / none sentinel — leave as-is.
        }
        match em.kind {
            EmbedKind::TypePtr(_) => {
                let nk = remap_ptr(
                    "type(embed)",
                    "ObjVar/DerivedFrom/ShadowType",
                    regen_key,
                    &regen.type_id_of_ptr,
                    &regen.type_name_of_ptr,
                    &base_syms.type_ptr_of_id,
                )?;
                module_bytes[o..o + 8].copy_from_slice(&nk.to_le_bytes());
                total.embed_type_ptr += 1;
            }
            EmbedKind::FuncId => {
                // FactoryRefs/BehaviorRefs hold sign-extended int32 T4 ids in int64 slots; zero is
                // their only sentinel. Behavior type tags live in BehaviorFunctionTypes.
                let (regen_id, regen_ptr) = resolve_embedded_regen_func_id(regen_key, &regen)?;
                let nptr = remap_ptr(
                    "function-id(embed)",
                    "Factory/BehaviorRefs",
                    regen_ptr,
                    &regen.func_id_of_ptr,
                    &regen.func_name_of_ptr,
                    &base_syms.func_ptr_of_id,
                )?;
                let new_id = if base_syms.funcid_to_ptr.get(&regen_id) == Some(&nptr) {
                    regen_id
                } else {
                    *base_syms
                        .ptr_to_funcid
                        .get(&nptr)
                        .filter(|&&id| id != 0)
                        .ok_or_else(|| RemapError::Unresolved {
                            kind: "function-id(embed,no base id)",
                            op: "Factory/BehaviorRefs",
                            key: nptr,
                            name: base_syms
                                .func_name_of_ptr
                                .get(&nptr)
                                .cloned()
                                .unwrap_or_default(),
                        })?
                };
                let new_val = i64::from(new_id);
                module_bytes[o..o + 8].copy_from_slice(&new_val.to_le_bytes());
                total.embed_func_id += 1;
            }
        }
    }

    // HARD POST-CONDITION: no regen tail-table key may survive anywhere in the remapped module
    // bytes. If one does, it lives in a module-record field the remap doesn't cover yet — fail
    // loudly (with offsets+names) instead of shipping a cache that null-derefs on boot.
    let surviving = scan_surviving_regen_keys(&module_bytes, &regen, &base_syms);
    if !surviving.is_empty() {
        let shown = surviving.len().min(12);
        let detail = surviving[..shown]
            .iter()
            .map(|s| format!("@+{:#x}={:#x} ({})", s.byte_off, s.value, s.name))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RemapError::SurvivingRegenKeys {
            n: surviving.len(),
            shown,
            detail,
        });
    }

    // Emit: FGuid+magic (from mini) + Modules count=1 + module bytes + 7 empty tables.
    let mut out = Vec::with_capacity(CacheHeader::SIZE + module_bytes.len() + 28);
    // Canonicalize the prepared artifact to the exact target generation. Callers that use this
    // low-level API directly therefore receive the same generation binding as compile-module.
    out.extend_from_slice(&base[..0x14]); // target FGuid + magic
    out.extend_from_slice(&1u32.to_le_bytes()); // Modules count = 1
    out.extend_from_slice(&module_bytes);
    out.extend_from_slice(&[0u8; 28]); // 7 tables × int32 count 0
    Ok((out, total))
}

/// Rewrite a one-module regen mini against `base`, with explicit opt-in support for symbols that
/// do not exist in the base cache. With [`RemapOptions::allow_new_symbols`] disabled this calls the
/// historical strict implementation directly, preserving its exact output and failure behavior.
///
/// In opt-in mode, existing refs still map by identity to vanilla keys. Rows for genuinely new
/// types/functions/globals declared or referenced by the module are selected from the regen tail,
/// their T1/T3 DataType dependencies and T2/T4 id rows are carried, required StaticNames/T7 rows
/// are retained, and every new key/id is deterministically synthesized from portable identity
/// before emission (never inherited from one compiler run's first-free allocation).
pub fn remap_module_to_base_with_options(
    extracted_mini: &[u8],
    base: &[u8],
    options: RemapOptions,
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
    if options.allow_new_symbols {
        remap_module_allow_new(extracted_mini, base)
    } else {
        remap_module_to_base(extracted_mini, base)
    }
}

// ---------------------------------------------------------------------------------------------
// PUBLIC N1 API for the bytediff oracle (`specs/semantic-oracle.md §3.1`): resolve a raw
// bytecode ref operand to a build-PORTABLE identity string, reusing the exact `SymTables`
// classification the remapper uses. Where `remap.rs` maps key->key (size-preserving splice),
// bytediff needs key->identity (a strict subset: `SymTables` already builds the forward
// `*_id_of_ptr` identity maps). No new RE.
// ---------------------------------------------------------------------------------------------

/// One cache's tail-table identity resolver for bytecode ref operands. Build once per cache;
/// call [`Self::resolve_operand`] on each ref operand of each disassembled instruction.
pub struct RefIdentity {
    syms: SymTables,
}

/// A resolved ref operand: either a portable identity (name+module+ns+signature — comparable
/// across builds) or, when the operand keys nothing in the tables (a primitive type-id, or a
/// key genuinely absent from the tail tables), a raw fallback that still compares equal to an
/// identical raw operand on the other side.
///
/// Equality is CUSTOM ([`PartialEq`] below): two `Named` operands compare via
/// [`Ident::oracle_eq`], which tolerates benign namespace-drift (GAP-A). This relation is NOT
/// transitive (`Foo::X` ~ `X` ~ `Baz::X`, yet `Foo::X` ≁ `Baz::X`), so `OperandId` is
/// deliberately NOT `Eq`; the oracle only ever compares operand PAIRS, never keys a map/set by
/// one, so a full equivalence relation is not required.
#[derive(Debug, Clone)]
pub enum OperandId {
    /// Portable identity resolved via the tail tables (the normal cross-referencing case).
    Named {
        kind: RefKind,
        ident: Ident,
    },
    /// Primitive type-id (<= LAST_PRIMITIVE, not in T2) — resolves to itself. Compared by value.
    Primitive(i32),
    /// A key/id present as an operand but absent from this cache's tables (defensive: a null
    /// sentinel, or a table gap). Compared by raw value so two identical raws still match.
    RawPtr(i64),
    RawId(i32),
}

impl PartialEq for OperandId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                OperandId::Named {
                    kind: ka,
                    ident: ia,
                },
                OperandId::Named {
                    kind: kb,
                    ident: ib,
                },
            ) => ka == kb && ia.oracle_eq(ib),
            (OperandId::Primitive(a), OperandId::Primitive(b)) => a == b,
            (OperandId::RawPtr(a), OperandId::RawPtr(b)) => a == b,
            (OperandId::RawId(a), OperandId::RawId(b)) => a == b,
            _ => false,
        }
    }
}

impl OperandId {
    /// Human-readable form for the SEMANTIC-DIFF report (e.g. `CALLSYS Story::GiveXP`).
    pub fn display(&self) -> String {
        match self {
            OperandId::Named { ident, .. } => ident.display.clone(),
            OperandId::Primitive(id) => format!("prim#{id}"),
            OperandId::RawPtr(p) => format!("<unresolved-ptr {p:#x}>"),
            OperandId::RawId(i) => format!("<unresolved-id {i}>"),
        }
    }

    /// For a resolved FUNCTION identity (a `CALLSYS`/`CALL` callee), return the callee's
    /// (owner-type-name, method-name) as borrowed slices of the composed `Ident.full`.
    ///
    /// Recursive template identities contain nested separators, so the owner and method are
    /// retained as structured metadata when T3 is parsed rather than recovered positionally from
    /// the display identity. This remains cache-independent while avoiding delimiter ambiguity.
    pub fn func_owner_method(&self) -> Option<(&str, &str)> {
        match self {
            OperandId::Named {
                kind: RefKind::FuncPtr | RefKind::FuncId,
                ident,
            } => ident
                .function_owner
                .as_deref()
                .zip(ident.function_name.as_deref()),
            _ => None,
        }
    }

    /// TEST-ONLY constructor for the bytediff N5 unit tests.
    #[doc(hidden)]
    pub fn named_func_for_test(owner: &str, method: &str) -> OperandId {
        let full = format!("{SEP}{SEP}{SEP}{SEP}{owner}{SEP}0:{SEP}{method}{SEP}1");
        OperandId::Named {
            kind: RefKind::FuncPtr,
            ident: Ident {
                full: full.clone(),
                ns_stripped: full,
                namespaces: vec![],
                display: format!("{owner}::{method}"),
                function_owner: Some(owner.to_owned()),
                function_name: Some(method.to_owned()),
            },
        }
    }

    /// True if this is a large runtime object type-id resolved as [`OperandId::Primitive`] (an
    /// `asCTypeInfo` id NOT in T2 that has the AngelScript object-mask bits set). Such an id is
    /// build-specific and drifts across recompiles; GAP-C (batch-38) treats a lone diff of one as
    /// benign when it feeds an `opCast`/`Cast` whose callee identity matches on both sides.
    /// Genuine primitive type-ids (bool/int/float — fixed engine constants, mask bits clear)
    /// return false and keep comparing by raw value.
    pub fn is_runtime_object_typeid(&self) -> bool {
        match self {
            // asTYPEID_MASK_OBJECT = 0x1C00_0000 (APPOBJECT|SCRIPTOBJECT|TEMPLATE). A primitive
            // (void/bool/int*/float/double) has none of these set; a runtime class type-id does.
            OperandId::Primitive(id) => (*id as u32) & 0x1C00_0000 != 0,
            _ => false,
        }
    }
}

impl RefIdentity {
    /// Build the identity resolver from a full cache's tail tables.
    pub fn build(bytes: &[u8]) -> Result<Self, WireError> {
        Ok(RefIdentity {
            syms: SymTables::build(bytes)?,
        })
    }

    /// Resolve a QWORD ptr operand (global/func/type ptr) to a portable identity.
    pub fn resolve_ptr(&self, kind: RefKind, key: i64) -> OperandId {
        let map = match kind {
            RefKind::GlobalPtr => &self.syms.global_ident_of_ptr,
            RefKind::FuncPtr => &self.syms.func_ident_of_ptr,
            RefKind::TypePtr => &self.syms.type_ident_of_ptr,
            // FuncId/TypeId are DW operands, not ptr — never routed here.
            RefKind::FuncId | RefKind::TypeId => return OperandId::RawPtr(key),
        };
        match map.get(&key) {
            Some(ident) => OperandId::Named {
                kind,
                ident: ident.clone(),
            },
            None => OperandId::RawPtr(key),
        }
    }

    /// Resolve a DWORD id operand (func-id via T4->T3, type-id via T2->T1) to a portable
    /// identity. A type-id absent from T2 is a PRIMITIVE (int/bool/float32/...) that resolves to
    /// itself (verbatim copy of the remapper's primitive-passthrough rule, `ref-remap.md §2.5`).
    pub fn resolve_id(&self, kind: RefKind, id: i32) -> OperandId {
        match kind {
            RefKind::FuncId if id == 0 => OperandId::RawId(0),
            RefKind::FuncId => match self.syms.funcid_to_ptr.get(&id) {
                Some(ptr) => match self.syms.func_ident_of_ptr.get(ptr) {
                    Some(ident) => OperandId::Named {
                        kind,
                        ident: ident.clone(),
                    },
                    None => OperandId::RawPtr(*ptr),
                },
                // Not a real func-id in this cache: defensive, compare raw.
                None => OperandId::RawId(id),
            },
            RefKind::TypeId => match self.syms.typeid_to_ptr.get(&id) {
                Some(ptr) => match self.syms.type_ident_of_ptr.get(ptr) {
                    Some(ident) => OperandId::Named {
                        kind,
                        ident: ident.clone(),
                    },
                    None => OperandId::RawPtr(*ptr),
                },
                // Absent from T2 => primitive type-id, resolves to itself.
                None => OperandId::Primitive(id),
            },
            RefKind::GlobalPtr | RefKind::FuncPtr | RefKind::TypePtr => OperandId::RawId(id),
        }
    }
}

#[cfg(test)]
#[path = "remap_loadout_plan_tests.rs"]
mod loadout_plan_tests;

#[cfg(test)]
mod bytediff_n1_tests {
    use super::*;

    /// N1: a CALLSYS func-ptr operand resolves to a portable identity string that embeds the
    /// function name — the exact make-or-break for the bytediff oracle. Uses the richtest sample.
    #[test]
    fn n1_resolves_callsys_ptr_to_named_identity() {
        // Skip on CI / any checkout without the gitignored `work/` scratch tree (mirrors the
        // bytediff sample gates): the richtest sample lives under work/reversing/gore-as/samples.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../work/reversing/gore-as/samples/PrecompiledScript.richtest.Cache"
        );
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("[skip] RE sample not present at {path}");
                return;
            }
        };
        let ident = RefIdentity::build(&bytes).expect("build RefIdentity");
        // Find any T3 func ptr key and confirm resolve_ptr yields a Named identity containing
        // the function's Name (the identity is module|ns|owner|name|is_method|params|ret).
        let (&ptr, name) = ident
            .syms
            .func_name_of_ptr
            .iter()
            .next()
            .expect("at least one func ref");
        let resolved = ident.resolve_ptr(RefKind::FuncPtr, ptr);
        match &resolved {
            OperandId::Named { kind, ident } => {
                assert_eq!(*kind, RefKind::FuncPtr);
                assert!(
                    ident.full.contains(name.as_str()),
                    "identity {:?} should contain func name {name:?}",
                    ident.full
                );
                // The ns-stripped skeleton must have the SAME structure (same SEP-field count)
                // as the full identity — it only blanks namespace fields, never adds/drops SEPs.
                assert_eq!(
                    ident.full.matches(SEP).count(),
                    ident.ns_stripped.matches(SEP).count(),
                    "ns-stripped skeleton must preserve SEP structure"
                );
            }
            other => panic!("expected Named identity, got {other:?}"),
        }
        // An unknown ptr resolves to a RawPtr (defensive), NOT a panic.
        assert!(matches!(
            ident.resolve_ptr(RefKind::FuncPtr, 0x7fff_dead_beef),
            OperandId::RawPtr(_)
        ));
        // A primitive type-id (bool == not-in-T2, small id) resolves to itself.
        assert!(matches!(
            ident.resolve_id(RefKind::TypeId, 0x41),
            OperandId::Primitive(0x41)
        ));
    }

    // ---- GAP-A namespace-drift unit tests (batch-38) ----

    fn ident(full: &str, ns_stripped: &str, namespaces: &[&str]) -> Ident {
        Ident {
            full: full.to_string(),
            ns_stripped: ns_stripped.to_string(),
            namespaces: namespaces.iter().map(|s| s.to_string()).collect(),
            display: full.to_string(),
            function_owner: None,
            function_name: None,
        }
    }

    fn fixed_comparison_budget(max: usize) -> IdentityComparisonBudget {
        IdentityComparisonBudget {
            remaining: max,
            max,
        }
    }

    #[test]
    fn base_pointer_lookup_is_exact_first_and_skeleton_bounded() {
        let regen_key = 99;
        let regen_identity = ident("M|::Thing", "M|Thing", &[""]);
        let base_identity = ident("M|Outer::Thing", "M|Thing", &["Outer"]);
        let unrelated = ident("Other|Unrelated", "Other|Unrelated", &["Huge"]);
        let regen_id_of_ptr = HashMap::from([(regen_key, regen_identity.full.clone())]);
        let regen_ident_of_ptr = HashMap::from([(regen_key, regen_identity.clone())]);
        let regen_names = HashMap::from([(regen_key, "Thing".to_owned())]);
        let mut base_ident_of_ptr = HashMap::from([(7, base_identity.clone())]);
        for key in 1000..2000 {
            base_ident_of_ptr.insert(key, unrelated.clone());
        }
        let base_summary = IdentityReverseSummary::build(&base_ident_of_ptr).unwrap();

        let work = identity_footprint(regen_key, &regen_identity)
            .unwrap()
            .checked_add(identity_footprint(7, &base_identity).unwrap())
            .unwrap();
        let mut drift_budget = fixed_comparison_budget(work);
        assert_eq!(
            match_base_ptr(
                "type",
                "test",
                regen_key,
                &regen_id_of_ptr,
                &regen_ident_of_ptr,
                &regen_names,
                &HashMap::new(),
                &base_ident_of_ptr,
                &base_summary,
                &mut drift_budget,
            )
            .unwrap(),
            Some(7)
        );
        assert_eq!(drift_budget.remaining, 0);

        // An exact full identity bypasses namespace-oracle work entirely, even when the phase's
        // comparison budget has already been consumed.
        let exact_ptrs = HashMap::from([(regen_identity.full.clone(), vec![7])]);
        let mut exhausted = fixed_comparison_budget(0);
        assert_eq!(
            match_base_ptr(
                "type",
                "test",
                regen_key,
                &regen_id_of_ptr,
                &regen_ident_of_ptr,
                &regen_names,
                &exact_ptrs,
                &base_ident_of_ptr,
                &base_summary,
                &mut exhausted,
            )
            .unwrap(),
            Some(7)
        );
    }

    #[test]
    fn base_pointer_lookup_rejects_drift_ambiguity_and_bounded_work() {
        let regen_key = 99;
        let regen_identity = ident("M|::Thing", "M|Thing", &[""]);
        let first = ident("M|One::Thing", "M|Thing", &["One"]);
        let second = ident("M|Two::Thing", "M|Thing", &["Two"]);
        let regen_id_of_ptr = HashMap::from([(regen_key, regen_identity.full.clone())]);
        let regen_ident_of_ptr = HashMap::from([(regen_key, regen_identity)]);
        let regen_names = HashMap::from([(regen_key, "Thing".to_owned())]);
        let base_ident_of_ptr = HashMap::from([(7, first), (8, second)]);
        let base_summary = IdentityReverseSummary::build(&base_ident_of_ptr).unwrap();
        let mut budget = fixed_comparison_budget(64 * 1024);
        assert!(matches!(
            match_base_ptr(
                "type",
                "test",
                regen_key,
                &regen_id_of_ptr,
                &regen_ident_of_ptr,
                &regen_names,
                &HashMap::new(),
                &base_ident_of_ptr,
                &base_summary,
                &mut budget,
            ),
            Err(RemapError::Ambiguous { n: 2, .. })
        ));

        let mut tiny_budget = fixed_comparison_budget(1);
        assert!(matches!(
            match_base_ptr(
                "type",
                "test",
                regen_key,
                &regen_id_of_ptr,
                &regen_ident_of_ptr,
                &regen_names,
                &HashMap::new(),
                &base_ident_of_ptr,
                &base_summary,
                &mut tiny_budget,
            ),
            Err(RemapError::Wire(
                WireError::IdentityComparisonBudgetExceeded { max: 1 }
            ))
        ));
    }

    #[test]
    fn declaration_and_property_namespace_matching_is_exact_first_and_bounded() {
        let mut first = DeclarationInventory::default();
        let mut second = DeclarationInventory::default();
        let exact = DeclarationIdentity {
            module: "M".to_owned(),
            namespace: "Owner".to_owned(),
            name: "Thing".to_owned(),
        };
        let drift = DeclarationIdentity {
            module: "M".to_owned(),
            namespace: "Outer::Owner".to_owned(),
            name: "Thing".to_owned(),
        };
        first.insert_type(exact.clone(), 0).unwrap();
        first.insert_global(exact.clone(), 0).unwrap();
        first
            .insert_property(
                PropertyDeclarationIdentity {
                    owner: exact.clone(),
                    name: "Value".to_owned(),
                },
                0,
            )
            .unwrap();
        second.insert_type(drift.clone(), 0).unwrap();
        second.insert_global(drift.clone(), 0).unwrap();
        second
            .insert_property(
                PropertyDeclarationIdentity {
                    owner: drift,
                    name: "Value".to_owned(),
                },
                0,
            )
            .unwrap();

        // Exact identity wins across the union and consumes no namespace comparison work.
        let inventories = [&first, &second];
        let mut exact_budget = fixed_comparison_budget(0);
        assert_eq!(
            match_declaration_identities(
                &inventories,
                &exact,
                DeclarationSetKind::Type,
                &mut exact_budget,
            )
            .unwrap(),
            FunctionDeclarationMatch::Unique
        );
        assert_eq!(
            match_declaration_identities(
                &inventories,
                &exact,
                DeclarationSetKind::Global,
                &mut exact_budget,
            )
            .unwrap(),
            FunctionDeclarationMatch::Unique
        );
        assert_eq!(
            match_property_declarations(
                &inventories,
                &PropertyDeclarationIdentity {
                    owner: exact.clone(),
                    name: "Value".to_owned(),
                },
                &mut exact_budget,
            )
            .unwrap(),
            FunctionDeclarationMatch::Unique
        );

        let missing_namespace = DeclarationIdentity {
            namespace: String::new(),
            ..exact.clone()
        };
        let mut ambiguity_budget = fixed_comparison_budget(64 * 1024);
        assert_eq!(
            match_declaration_identities(
                &inventories,
                &missing_namespace,
                DeclarationSetKind::Type,
                &mut ambiguity_budget,
            )
            .unwrap(),
            FunctionDeclarationMatch::Ambiguous
        );
        let mut tiny_budget = fixed_comparison_budget(1);
        assert!(matches!(
            match_property_declarations(
                &inventories,
                &PropertyDeclarationIdentity {
                    owner: missing_namespace,
                    name: "Value".to_owned(),
                },
                &mut tiny_budget,
            ),
            Err(WireError::IdentityComparisonBudgetExceeded { max: 1 })
        ));
    }

    #[test]
    fn template_and_script_owner_indexes_share_namespace_budget() {
        let mut current_owners = ScriptOwnerIndex::default();
        let mut fallback_owners = ScriptOwnerIndex::default();
        let owner_one = DeclarationIdentity {
            module: "M".to_owned(),
            namespace: "One".to_owned(),
            name: "Class".to_owned(),
        };
        let owner_two = DeclarationIdentity {
            namespace: "Two".to_owned(),
            ..owner_one.clone()
        };
        let one_identity = ident("M|One|Class", "M||Class", &["One"]);
        let two_identity = ident("M|Two|Class", "M||Class", &["Two"]);
        current_owners.insert(&owner_one, &one_identity, 0).unwrap();
        fallback_owners
            .insert(&owner_two, &two_identity, 0)
            .unwrap();
        let primary = HashMap::new();
        let context = DeclarationTypeContext {
            primary: &primary,
            fallback: None,
            script_owners: current_owners,
            fallback_script_owners: Some(&fallback_owners),
        };

        let mut exact_budget = fixed_comparison_budget(0);
        assert_eq!(
            context
                .script_owner(&owner_one, &mut exact_budget)
                .unwrap()
                .unwrap()
                .full,
            one_identity.full
        );
        let missing_namespace = DeclarationIdentity {
            namespace: String::new(),
            ..owner_one
        };
        let mut owner_budget = fixed_comparison_budget(64 * 1024);
        assert!(matches!(
            context.script_owner(&missing_namespace, &mut owner_budget),
            Err(WireError::BadLen {
                field: "ambiguous script class declaration",
                ..
            })
        ));

        let pristine = PristineDeclarationAuthority {
            declarations: DeclarationInventory::default(),
            types_by_ptr: HashMap::new(),
            template_bases: HashMap::from([(
                ("Array".to_owned(), 1),
                HashSet::from(["One".to_owned(), "Two".to_owned()]),
            )]),
            properties: HashSet::new(),
            orphan_functions: HashSet::new(),
            script_owners: ScriptOwnerIndex::default(),
        };
        let mut template_budget = fixed_comparison_budget(64 * 1024);
        assert_eq!(
            match_template_base(&pristine, "Array", "", 1, &mut template_budget).unwrap(),
            FunctionDeclarationMatch::Ambiguous
        );
        let mut exact_template_budget = fixed_comparison_budget(0);
        assert_eq!(
            match_template_base(&pristine, "Array", "One", 1, &mut exact_template_budget,).unwrap(),
            FunctionDeclarationMatch::Unique
        );
    }

    /// GAP-A: a vanilla symbol WITH a namespace and a regen symbol with an EMPTY namespace but
    /// otherwise identical module/name/subtypes = MATCH (benign drift). Direction-symmetric.
    #[test]
    fn gap_a_empty_namespace_matches() {
        let sep = SEP;
        // T5 global `__StaticType_X`: vanilla ns=`G1R::GenericVoiceline`, regen ns=``.
        let van = ident(
            &format!("Story.G1R{sep}G1R::GenericVoiceline{sep}__StaticType_X{sep}0"),
            &format!("Story.G1R{sep}{sep}__StaticType_X{sep}0"),
            &["G1R::GenericVoiceline"],
        );
        let reg = ident(
            &format!("Story.G1R{sep}{sep}__StaticType_X{sep}0"),
            &format!("Story.G1R{sep}{sep}__StaticType_X{sep}0"),
            &[""],
        );
        assert!(
            van.oracle_eq(&reg),
            "empty-vs-nonempty namespace must match"
        );
        assert!(reg.oracle_eq(&van), "match is symmetric");
        // As full OperandId operands (same kind) they compare equal too.
        let a = OperandId::Named {
            kind: RefKind::GlobalPtr,
            ident: van,
        };
        let b = OperandId::Named {
            kind: RefKind::GlobalPtr,
            ident: reg,
        };
        assert_eq!(a, b);
    }

    /// GAP-A drift: the enclosing `namespace G1R { }` block is dropped, leaving a `::`-suffix
    /// (`G1R::UStoryG1R` vs `UStoryG1R`) — benign.
    #[test]
    fn gap_a_namespace_suffix_matches() {
        let sep = SEP;
        let van = ident(
            &format!("Story.G1R{sep}G1R::UStoryG1R{sep}{sep}Get{sep}0"),
            &format!("Story.G1R{sep}{sep}{sep}Get{sep}0"),
            &["G1R::UStoryG1R"],
        );
        let reg = ident(
            &format!("Story.G1R{sep}UStoryG1R{sep}{sep}Get{sep}0"),
            &format!("Story.G1R{sep}{sep}{sep}Get{sep}0"),
            &["UStoryG1R"],
        );
        assert!(
            van.oracle_eq(&reg),
            "namespace `::`-suffix drift must match"
        );
    }

    /// GAP-A GUARD: two genuinely different symbols distinguished ONLY by namespace
    /// (`Foo::Bar` vs `Baz::Bar`, both non-empty, neither a `::`-suffix of the other) must STAY
    /// distinct (SEMANTIC) — a real collision the fix must not collapse.
    #[test]
    fn gap_a_real_collision_kept_semantic() {
        let sep = SEP;
        let foo = ident(
            &format!("M{sep}Foo{sep}Bar{sep}0"),
            &format!("M{sep}{sep}Bar{sep}0"),
            &["Foo"],
        );
        let baz = ident(
            &format!("M{sep}Baz{sep}Bar{sep}0"),
            &format!("M{sep}{sep}Bar{sep}0"),
            &["Baz"],
        );
        assert!(
            !foo.oracle_eq(&baz),
            "Foo::Bar vs Baz::Bar is a real collision, must NOT match"
        );
        assert!(!baz.oracle_eq(&foo));
        let a = OperandId::Named {
            kind: RefKind::GlobalPtr,
            ident: foo,
        };
        let b = OperandId::Named {
            kind: RefKind::GlobalPtr,
            ident: baz,
        };
        assert_ne!(a, b);
    }

    /// GAP-A GUARD: a difference in a NON-namespace field (the name itself) is never collapsed,
    /// even when the namespace fields would match — the skeleton differs.
    #[test]
    fn gap_a_different_name_kept_semantic() {
        let sep = SEP;
        let a = ident(
            &format!("M{sep}G1R{sep}Alpha{sep}0"),
            &format!("M{sep}{sep}Alpha{sep}0"),
            &["G1R"],
        );
        let b = ident(
            &format!("M{sep}{sep}Beta{sep}0"),
            &format!("M{sep}{sep}Beta{sep}0"),
            &[""],
        );
        assert!(
            !a.oracle_eq(&b),
            "different name (skeleton differs) must not match"
        );
    }

    /// `is_ns_suffix` requires a `::` segment boundary, not a raw substring.
    #[test]
    fn ns_suffix_requires_segment_boundary() {
        assert!(is_ns_suffix("G1R::UStoryG1R", "UStoryG1R"));
        assert!(is_ns_suffix("A::B::C", "C"));
        assert!(is_ns_suffix("A::B::C", "B::C"));
        assert!(!is_ns_suffix("BazBar", "Bar")); // no `::` boundary
        assert!(!is_ns_suffix("Bar", "Bar")); // not proper (equal length)
        assert!(!is_ns_suffix("Foo::Bar", "Baz")); // not a suffix
    }

    /// GAP-C: the object-mask discriminator separates genuine primitive type-ids (mask clear,
    /// compared by raw value) from large runtime `asCTypeInfo` ids (mask set, opCast-gated).
    #[test]
    fn gap_c_runtime_object_typeid_discriminator() {
        // 0x48003464 (1207972964) has asTYPEID_SCRIPTOBJECT (0x08000000) set → runtime.
        assert!(OperandId::Primitive(1207972964).is_runtime_object_typeid());
        assert!(OperandId::Primitive(1207972931).is_runtime_object_typeid());
        // Genuine primitives: mask bits clear.
        assert!(!OperandId::Primitive(0x41).is_runtime_object_typeid());
        assert!(!OperandId::Primitive(0).is_runtime_object_typeid());
        assert!(!OperandId::Primitive(10).is_runtime_object_typeid());
        // Non-primitive variants are never runtime type-ids.
        assert!(!OperandId::RawId(1207972964).is_runtime_object_typeid());
    }

    /// `func_owner_method` extracts owner-type-name (field 4) + method-name (field 6) from a
    /// composed T3 function identity, positionally fixed because the embedded owner is EXACTLY 4
    /// SEP-fields. This is the cache-independent key the n5 scope-strip uses.
    #[test]
    fn func_owner_method_splits_composed_identity() {
        let sep = SEP;
        // Mirror the exact composition for a RAII scope-counter ctor `FScopeCycleCounter::$beh0`:
        // module="" ns="" owner=(""|""|"FScopeCycleCounter"|"0:") name="$beh0" is_method="1" ...
        let full = format!(
            "{sep}{sep}{sep}{sep}FScopeCycleCounter{sep}0:{sep}$beh0{sep}1{sep}110100:5:{sep}{sep}FStatID{sep}0:,{sep}000000:82:"
        );
        let stripped = full.clone(); // ns fields already empty here
        let id = OperandId::Named {
            kind: RefKind::FuncPtr,
            ident: Ident {
                full,
                ns_stripped: stripped,
                namespaces: vec![],
                display: "FScopeCycleCounter::$beh0".to_owned(),
                function_owner: Some("FScopeCycleCounter".to_owned()),
                function_name: Some("$beh0".to_owned()),
            },
        };
        assert_eq!(
            id.func_owner_method(),
            Some(("FScopeCycleCounter", "$beh0"))
        );

        // FStatID temp dtor `FStatID::$beh2`.
        let full2 =
            format!("{sep}{sep}{sep}{sep}FStatID{sep}0:{sep}$beh2{sep}1{sep}{sep}000000:82:");
        let id2 = OperandId::Named {
            kind: RefKind::FuncPtr,
            ident: Ident {
                full: full2.clone(),
                ns_stripped: full2,
                namespaces: vec![],
                display: "FStatID::$beh2".to_owned(),
                function_owner: Some("FStatID".to_owned()),
                function_name: Some("$beh2".to_owned()),
            },
        };
        assert_eq!(id2.func_owner_method(), Some(("FStatID", "$beh2")));

        // A callee WITH non-empty module/namespace/owner-namespace still indexes correctly (the
        // owner is still exactly 4 fields).
        let full3 = format!(
            "GAS.Mixins{sep}NS{sep}OMod{sep}ONs{sep}AGothicCharacterState{sep}0:{sep}IsTrulyPartOfGuild{sep}1{sep}p{sep}r"
        );
        let id3 = OperandId::Named {
            kind: RefKind::FuncPtr,
            ident: Ident {
                full: full3.clone(),
                ns_stripped: full3,
                namespaces: vec![],
                display: "AGothicCharacterState::IsTrulyPartOfGuild".to_owned(),
                function_owner: Some("AGothicCharacterState".to_owned()),
                function_name: Some("IsTrulyPartOfGuild".to_owned()),
            },
        };
        assert_eq!(
            id3.func_owner_method(),
            Some(("AGothicCharacterState", "IsTrulyPartOfGuild"))
        );

        // Non-function operands / malformed identities return None.
        assert_eq!(OperandId::Primitive(3).func_owner_method(), None);
        let short = OperandId::Named {
            kind: RefKind::FuncPtr,
            ident: Ident {
                full: format!("a{sep}b"),
                ns_stripped: String::new(),
                namespaces: vec![],
                display: "a::b".to_owned(),
                function_owner: None,
                function_name: None,
            },
        };
        assert_eq!(short.func_owner_method(), None);
    }
}
