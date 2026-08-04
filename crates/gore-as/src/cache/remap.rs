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

use std::collections::{HashMap, HashSet};

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

/// Field separator for composing a stable symbol identity string (unlikely in any name).
const SEP: char = '\u{1f}';

/// Field separator INSIDE a namespace field's `::`-qualified segments (regen drops leading
/// `Ns::` segments — see [`ns_drift_ok`]).
const NS_SEP: &str = "::";

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
struct SymTables {
    /// T1: type ptr key -> identity ; identity -> ptr key.
    type_id_of_ptr: HashMap<i64, String>,
    type_ptr_of_id: HashMap<String, Vec<i64>>,
    /// T1: type ptr key -> the oracle [`Ident`] (full + ns-stripped skeleton + namespace list).
    /// PARALLEL to `type_id_of_ptr` (whose full string the remapper's key→key splice keeps).
    type_ident_of_ptr: HashMap<i64, Ident>,
    /// T2: type-id (i32, raw operand) -> type ptr.
    typeid_to_ptr: HashMap<i32, i64>,
    /// inverse of T2: type ptr -> type-id (raw i32 operand).
    ptr_to_typeid: HashMap<i64, i32>,
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
    /// EVERY int64 ptr-key that appears as a key in this cache's tail tables: T1 type ptrs,
    /// T3 func ptrs, T5 global ptrs, and the ptr values in T2/T4 (id->ptr). Used by the
    /// post-condition scan to assert no regen ptr-key survives in a remapped module's bytes.
    /// (T7 PropertyReferences keys are DERIVED `(tid<<1)|(off<<33)|1` — not raw ptrs — and are
    /// never an operand/embedded field, so they are excluded here; the type-id remap handles them.)
    all_ptr_keys: HashSet<i64>,
}

/// Read a DataType's stable identity contribution: token + (for object/value types) the
/// resolved TYPE IDENTITY of its `type_info` ptr (the build-specific ptr resolved to a portable
/// identity that includes the type's name + template subtypes — so `TSubclassOf<AFoo>` and
/// `TSubclassOf<ABar>` are distinguished, which matters for conversion-operator overloads).
///
/// Returns the oracle [`Ident`] triple: the nested type's namespace fields (which drift, GAP-A)
/// are carried through into both the stripped skeleton and the namespace list, so a func/global
/// identity that embeds this DataType composes correctly for [`Ident::oracle_eq`].
fn datatype_identity(
    c: &mut Cursor,
    type_ident_of_ptr: &HashMap<i64, Ident>,
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
            .cloned()
            .unwrap_or_default()
    } else {
        Ident::default()
    };
    let prefix = format!(
        "{}{}{}{}{}{}:{token}:",
        b0 as u8, b1 as u8, b2 as u8, b3 as u8, b4 as u8, b5 as u8
    );
    Ok(Ident {
        full: format!("{prefix}{}", tident.full),
        ns_stripped: format!("{prefix}{}", tident.ns_stripped),
        namespaces: tident.namespaces,
    })
}

impl SymTables {
    /// Parse the 7 tail tables of `bytes` into identity maps. Two passes over T3 are avoided
    /// by parsing T1 first (so func owner/param type ptrs resolve to names).
    fn build(bytes: &[u8]) -> Result<Self, WireError> {
        let tail = module_region_end(bytes)?;
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
        struct RawType {
            key: i64,
            name: String,
            module: String,
            namespace: String,
            /// (token, subtype_type_info_ptr) per subtype.
            subs: Vec<(i32, i64)>,
        }
        let ntypes = c.read_count("TypeReferences")?;
        let mut raw_types = Vec::with_capacity(ntypes);
        for _ in 0..ntypes {
            let key = c.read_i64()?;
            all_ptr_keys.insert(key);
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let nsub = c.read_count("TypeRef.SubTypes")?;
            let mut subs = Vec::with_capacity(nsub);
            for _ in 0..nsub {
                // DataType (36 B): 24 bools + i64 type_info + i32 token.
                let base = c.pos();
                let type_info = i64::from_le_bytes(bytes[base + 24..base + 32].try_into().unwrap());
                let token = i32::from_le_bytes(bytes[base + 32..base + 36].try_into().unwrap());
                subs.push((token, type_info));
                c.skip(DATA_TYPE_SIZE)?;
            }
            type_name_of_ptr.insert(key, name.clone());
            raw_types.push(RawType {
                key,
                name,
                module,
                namespace,
                subs,
            });
        }
        // PASS 2: compose identities using resolved subtype names. Subtype names are the bare
        // T1 Name (no module/namespace), so a subtype contributes NO namespace field — the only
        // namespace in a type identity is the type's OWN (field index 1). The oracle skeleton
        // drops it; the namespace list carries it (GAP-A).
        for rt in &raw_types {
            let mut sub_ident = String::new();
            for (token, sub_ptr) in &rt.subs {
                let sub_name = if *token == 5 {
                    type_name_of_ptr.get(sub_ptr).cloned().unwrap_or_default()
                } else {
                    String::new()
                };
                sub_ident.push_str(&format!("{token}:{sub_name},"));
            }
            let identity = format!(
                "{}{SEP}{}{SEP}{}{SEP}{}:{sub_ident}",
                rt.module,
                rt.namespace,
                rt.name,
                rt.subs.len()
            );
            // ns-stripped skeleton: identical but with the namespace field blanked.
            let ns_stripped = format!(
                "{}{SEP}{SEP}{}{SEP}{}:{sub_ident}",
                rt.module,
                rt.name,
                rt.subs.len()
            );
            type_ident_of_ptr.insert(
                rt.key,
                Ident {
                    full: identity.clone(),
                    ns_stripped,
                    namespaces: vec![rt.namespace.clone()],
                },
            );
            type_id_of_ptr.insert(rt.key, identity.clone());
            type_ptr_of_id.entry(identity).or_default().push(rt.key);
        }

        // T2 TypeIdReferenceToPointer: i32 id -> i64 ptr.
        let mut typeid_to_ptr = HashMap::new();
        let mut ptr_to_typeid = HashMap::new();
        for _ in 0..c.read_count("TypeIdRef")? {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            all_ptr_keys.insert(ptr);
            typeid_to_ptr.insert(id, ptr);
            ptr_to_typeid.insert(ptr, id);
        }

        // T3 FunctionReferences: i64 key + (Name, Module, Namespace, 3 bool, i64 ObjectType,
        // TArray<DataType> params, DataType ret).
        let mut func_id_of_ptr = HashMap::new();
        let mut func_ptr_of_id: HashMap<String, Vec<i64>> = HashMap::new();
        let mut func_ident_of_ptr: HashMap<i64, Ident> = HashMap::new();
        let mut func_name_of_ptr = HashMap::new();
        for _ in 0..c.read_count("FunctionReferences")? {
            let key = c.read_i64()?;
            all_ptr_keys.insert(key);
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let _is_const = c.read_bool4()?;
            let _is_imported = c.read_bool4()?;
            let is_method = c.read_bool4()?;
            let objtype = c.read_i64()?;
            // Use the owner's FULL type identity (name + template subtypes), not just its name,
            // so e.g. `TSubclassOf<AFoo>::opImplConv` and `TSubclassOf<ABar>::opImplConv` (which
            // share the bare owner name `TSubclassOf`) are distinguished. The owner + each param +
            // ret are type identities that carry their OWN (drifting) namespace fields, so compose
            // all three oracle forms in lockstep (GAP-A: a func's own ns is field index 1, then the
            // owner's ns, then each param's, then ret's — traversal order preserved in the list).
            let owner = type_ident_of_ptr.get(&objtype).cloned().unwrap_or_default();
            let nparams = c.read_count("FuncRef.Params")?;
            let mut params_full = String::new();
            let mut params_stripped = String::new();
            let mut param_ns: Vec<String> = Vec::new();
            for _ in 0..nparams {
                let p = datatype_identity(&mut c, &type_ident_of_ptr)?;
                params_full.push_str(&p.full);
                params_full.push(',');
                params_stripped.push_str(&p.ns_stripped);
                params_stripped.push(',');
                param_ns.extend(p.namespaces);
            }
            let ret = datatype_identity(&mut c, &type_ident_of_ptr)?;
            let identity = format!(
                "{module}{SEP}{namespace}{SEP}{}{SEP}{name}{SEP}{}{SEP}{params_full}{SEP}{}",
                owner.full, is_method as u8, ret.full
            );
            let ns_stripped = format!(
                "{module}{SEP}{SEP}{}{SEP}{name}{SEP}{}{SEP}{params_stripped}{SEP}{}",
                owner.ns_stripped, is_method as u8, ret.ns_stripped
            );
            let mut namespaces = Vec::with_capacity(2 + param_ns.len() + ret.namespaces.len());
            namespaces.push(namespace.clone()); // the func's own namespace (field index 1)
            namespaces.extend(owner.namespaces.iter().cloned());
            namespaces.extend(param_ns);
            namespaces.extend(ret.namespaces.iter().cloned());
            func_ident_of_ptr.insert(
                key,
                Ident {
                    full: identity.clone(),
                    ns_stripped,
                    namespaces,
                },
            );
            func_id_of_ptr.insert(key, identity.clone());
            func_ptr_of_id.entry(identity).or_default().push(key);
            func_name_of_ptr.insert(key, name);
        }

        // T4 FunctionIdReferenceToPointer: i32 id -> i64 ptr.
        let mut funcid_to_ptr = HashMap::new();
        let mut ptr_to_funcid = HashMap::new();
        for _ in 0..c.read_count("FuncIdRef")? {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            all_ptr_keys.insert(ptr);
            funcid_to_ptr.insert(id, ptr);
            ptr_to_funcid.insert(ptr, id);
        }

        // T5 GlobalReferences: i64 key + (Name, Module, Namespace, i32 bIsString).
        let mut global_id_of_ptr = HashMap::new();
        let mut global_ptr_of_id: HashMap<String, Vec<i64>> = HashMap::new();
        let mut global_ident_of_ptr: HashMap<i64, Ident> = HashMap::new();
        let mut global_name_of_ptr = HashMap::new();
        for _ in 0..c.read_count("GlobalReferences")? {
            let key = c.read_i64()?;
            all_ptr_keys.insert(key);
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let is_string = c.read_bool4()?;
            let identity = format!(
                "{module}{SEP}{namespace}{SEP}{name}{SEP}{}",
                is_string as u8
            );
            // ns-stripped skeleton: namespace field (index 1) blanked (GAP-A).
            let ns_stripped = format!("{module}{SEP}{SEP}{name}{SEP}{}", is_string as u8);
            global_ident_of_ptr.insert(
                key,
                Ident {
                    full: identity.clone(),
                    ns_stripped,
                    namespaces: vec![namespace.clone()],
                },
            );
            global_id_of_ptr.insert(key, identity.clone());
            global_ptr_of_id.entry(identity).or_default().push(key);
            global_name_of_ptr.insert(key, name);
        }
        // T6 StaticNames + T7 PropertyReferences are not operand-referenced (member ops carry a
        // TYPE-ID, not a prop key — the prop key is derived from typeid+offset), so skip them.

        Ok(SymTables {
            type_id_of_ptr,
            type_ptr_of_id,
            type_ident_of_ptr,
            typeid_to_ptr,
            ptr_to_typeid,
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
            all_ptr_keys,
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

// -------------------------------------------------------------------------------------------------
// Raw tail-row metadata used only by the explicit new-symbol path. The strict/default remapper
// above deliberately keeps its historical parsing and output behavior unchanged.
// -------------------------------------------------------------------------------------------------

#[derive(Clone)]
struct TypeRowMeta {
    start: usize,
    end: usize,
    key: i64,
    module: String,
    /// Absolute byte offsets + raw values of DataType.TypeInfo.OldReference fields in SubTypes.
    type_deps: Vec<(usize, i64)>,
}

#[derive(Clone)]
struct FuncRowMeta {
    start: usize,
    end: usize,
    key: i64,
    name: String,
    module: String,
    /// ObjectType plus every parameter/return DataType.TypeInfo.OldReference.
    type_deps: Vec<(usize, i64)>,
}

#[derive(Clone)]
struct GlobalRowMeta {
    start: usize,
    end: usize,
    key: i64,
    module: String,
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

struct TailMetadata {
    types: Vec<TypeRowMeta>,
    type_ids: Vec<IdPtrRowMeta>,
    funcs: Vec<FuncRowMeta>,
    func_ids: Vec<IdPtrRowMeta>,
    globals: Vec<GlobalRowMeta>,
    static_names: Vec<StaticRowMeta>,
    properties: Vec<PropertyRowMeta>,
}

/// Consume one inline DataType and return its TypeInfo.OldReference field location/value.
fn read_datatype_dep(c: &mut Cursor) -> Result<(usize, i64), WireError> {
    c.skip(24)?; // six archive bools
    let off = c.pos();
    let ptr = c.read_i64()?;
    c.skip(4)?; // token
    Ok((off, ptr))
}

impl TailMetadata {
    fn build(bytes: &[u8]) -> Result<Self, RemapError> {
        let tail = module_region_end(bytes)?;
        let mut c = Cursor::at(bytes, tail);

        let mut types = Vec::new();
        for _ in 0..c.read_count("TypeReferences")? {
            let start = c.pos();
            let key = c.read_i64()?;
            c.read_sia()?; // Name
            let module = c.read_sia()?;
            c.read_sia()?; // Namespace
            let mut type_deps = Vec::new();
            for _ in 0..c.read_count("TypeRef.SubTypes")? {
                type_deps.push(read_datatype_dep(&mut c)?);
            }
            types.push(TypeRowMeta {
                start,
                end: c.pos(),
                key,
                module,
                type_deps,
            });
        }

        let mut type_ids = Vec::new();
        for _ in 0..c.read_count("TypeIdRef")? {
            let start = c.pos();
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            type_ids.push(IdPtrRowMeta {
                start,
                end: c.pos(),
                id,
                ptr,
            });
        }

        let mut funcs = Vec::new();
        for _ in 0..c.read_count("FunctionReferences")? {
            let start = c.pos();
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            c.read_sia()?; // Namespace
            c.skip(12)?; // const/imported/method
            let owner_off = c.pos();
            let owner = c.read_i64()?;
            let mut type_deps = vec![(owner_off, owner)];
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
                type_deps,
            });
        }

        let mut func_ids = Vec::new();
        for _ in 0..c.read_count("FuncIdRef")? {
            let start = c.pos();
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            func_ids.push(IdPtrRowMeta {
                start,
                end: c.pos(),
                id,
                ptr,
            });
        }

        let mut globals = Vec::new();
        for _ in 0..c.read_count("GlobalReferences")? {
            let start = c.pos();
            let key = c.read_i64()?;
            c.read_sia()?; // Name
            let module = c.read_sia()?;
            c.read_sia()?; // Namespace
            c.skip(4)?; // bIsString
            globals.push(GlobalRowMeta {
                start,
                end: c.pos(),
                key,
                module,
            });
        }

        let mut static_names = Vec::new();
        for index in 0..c.read_count("StaticNames")? {
            let start = c.pos();
            let name = c.read_sia()?;
            static_names.push(StaticRowMeta {
                index,
                start,
                end: c.pos(),
                name,
            });
        }

        let mut properties = Vec::new();
        for index in 0..c.read_count("PropertyReferences")? {
            let start = c.pos();
            let key = c.read_i64()?;
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
            type_ids,
            funcs,
            func_ids,
            globals,
            static_names,
            properties,
        })
    }

    fn type_row(&self, key: i64) -> Option<&TypeRowMeta> {
        self.types.iter().find(|r| r.key == key)
    }

    fn func_row(&self, key: i64) -> Option<&FuncRowMeta> {
        self.funcs.iter().find(|r| r.key == key)
    }
}

/// Immutable part of sequential StaticNames composition. Independently remapped minis encode
/// their first private T6 row at `pristine_names.len()`, so a later mini cannot be interpreted
/// from the already-grown accumulator alone. Build this once from the exact base all minis were
/// remapped against, then carry only the small name pool and `__STATIC_NAME` lookup sets.
#[derive(Debug)]
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

fn split_type_id_operand(id: i32) -> (i32, u32) {
    let raw = id as u32;
    ((raw & TYPE_ID_CORE_MASK) as i32, raw & !TYPE_ID_CORE_MASK)
}

fn apply_type_id_operand_flags(core: i32, flags: u32) -> i32 {
    ((core as u32 & TYPE_ID_CORE_MASK) | flags) as i32
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
                    let new_id =
                        *base
                            .ptr_to_funcid
                            .get(&nptr)
                            .ok_or_else(|| RemapError::Unresolved {
                                kind: "function-id(no base id)",
                                op: ins.op.name,
                                key: nptr,
                                name: base
                                    .func_name_of_ptr
                                    .get(&nptr)
                                    .cloned()
                                    .unwrap_or_default(),
                            })?;
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
                    let new_id =
                        *base
                            .ptr_to_typeid
                            .get(&nptr)
                            .ok_or_else(|| RemapError::Unresolved {
                                kind: "type-id(no base id)",
                                op: ins.op.name,
                                key: nptr,
                                name: base
                                    .type_name_of_ptr
                                    .get(&nptr)
                                    .cloned()
                                    .unwrap_or_default(),
                            })?;
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
    TypePtr,
    FuncId,
}
struct EmbedRef {
    byte_off: usize,
    kind: EmbedKind,
}

/// Everything the byte-walker collects from the single module entry.
#[derive(Default)]
struct ModuleSpans {
    code: Vec<CodeSpan>,
    embeds: Vec<EmbedRef>,
}

/// Walk the single module entry in `mini` (TMap key + module value) and collect every
/// function's ByteCode span + every embedded int64 ref. Mirrors `walk_modules::read_module_c`
/// but records byte offsets.
fn collect_module_spans(mini: &[u8]) -> Result<ModuleSpans, WireError> {
    let mut c = Cursor::at(mini, CacheHeader::SIZE);
    c.read_fstring()?; // TMap key
    let mut spans = ModuleSpans::default();
    read_module_spans(&mut c, mini, &mut spans)?;
    Ok(spans)
}

/// A `FAngelscriptPrecompiledDataType` (36 B): 6×bool(24) + int64 TypeInfo.OldReference(+24) +
/// int32 Token(+32). The TypeInfo.OldReference is a T1 TYPE ptr (0 for primitives). Record it as
/// an embedded type-ptr ref (skipped at remap time when 0), then advance past the 36 bytes.
/// CONFIRMED: container-splice.md §0/§3 (void ReturnType has TypeInfo.Old=0). This is the field
/// that carried the surviving regen-keys — every DataType in function signatures / property /
/// global / import records embeds one, and the prior remap skipped them wholesale.
fn embed_datatype(c: &mut Cursor, out: &mut ModuleSpans) -> Result<(), WireError> {
    let dt_start = c.pos();
    out.embeds.push(EmbedRef {
        byte_off: dt_start + 24,
        kind: EmbedKind::TypePtr,
    });
    c.skip(DATA_TYPE_SIZE)?;
    Ok(())
}

fn read_function_spans(
    c: &mut Cursor,
    bytes: &[u8],
    out: &mut ModuleSpans,
) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    embed_datatype(c, out)?; // ReturnType
    let nptypes = c.read_count("ParameterTypes")?;
    for _ in 0..nptypes {
        embed_datatype(c, out)?;
    }
    c.skip_tarray_sia("ParameterNames")?;
    c.skip_tarray_fixed(4, "ParameterFlags")?;
    c.skip_tarray_sia("ParameterDefaultArgs")?;
    c.skip(4)?; // FunctionTraits
                // ByteCode TArray<int32>: record the span.
    let count = c.read_count("ByteCode")?;
    let data_off = c.pos();
    out.code.push(CodeSpan { data_off, count });
    c.skip(count * 4)?;
    let _ = bytes; // (bytes used only for span math; offsets are absolute)
    c.skip_tarray_fixed(4, "ByteCodeReferences")?;
    c.skip(4)?; // VariableSpace
                // ObjVariableTypes: TArray<int64> of TYPE ptrs (T1) per object local slot — remap each.
    let nobj = c.read_count("ObjVariableTypes")?;
    for _ in 0..nobj {
        out.embeds.push(EmbedRef {
            byte_off: c.pos(),
            kind: EmbedKind::TypePtr,
        });
        c.skip(8)?;
    }
    c.skip_tarray_fixed(4, "ObjVariablePos")?;
    c.skip(4)?; // ObjVariablesOnHeap
    c.skip_tarray_fixed(4, "VariableInfoProgramPos")?;
    c.skip_tarray_fixed(4, "VariableInfoOffset")?;
    c.skip_tarray_fixed(4, "VariableInfoOption")?;
    c.skip(4)?; // StackNeeded
    c.skip(4)?; // Id
    c.skip(4)?; // DeclaredAt
    c.skip_tarray_fixed(4, "LineNumbers")?;
    if c.read_bool4()? {
        c.read_sia()?; // UnrealFunctionName
        c.skip_tarray_sia("UF.MetaSpec")?;
        c.skip_tarray_sia("UF.MetaValues")?;
        c.skip(18 * 4)?;
    }
    Ok(())
}

fn read_property_spans(c: &mut Cursor, out: &mut ModuleSpans) -> Result<(), WireError> {
    c.read_sia()?; // Name
    embed_datatype(c, out)?; // Type
    c.skip(4)?; // bIsPrivate
    c.skip(4)?; // bIsProtected
    if c.read_bool4()? {
        c.skip_tarray_sia("UP.MetaSpec")?;
        c.skip_tarray_sia("UP.MetaValues")?;
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

fn read_class_spans(c: &mut Cursor, bytes: &[u8], out: &mut ModuleSpans) -> Result<(), WireError> {
    c.read_sia()?; // ClassName
    c.read_sia()?; // Namespace
    c.skip(4)?; // Flags
    let nprops = c.read_count("Class.Properties")?;
    for _ in 0..nprops {
        read_property_spans(c, out)?;
    }
    let nmethods = c.read_count("Class.Methods")?;
    for _ in 0..nmethods {
        read_function_spans(c, bytes, out)?;
    }
    c.skip_tarray_fixed(4, "Class.MethodTable")?;
    // DerivedFrom + ShadowType: int64 TYPE ptrs (T1). Value 0 = none (skipped at remap time).
    out.embeds.push(EmbedRef {
        byte_off: c.pos(),
        kind: EmbedKind::TypePtr,
    });
    c.skip(8)?; // DerivedFrom
    out.embeds.push(EmbedRef {
        byte_off: c.pos(),
        kind: EmbedKind::TypePtr,
    });
    c.skip(8)?; // ShadowType
    let nctors = c.read_count("Class.Constructors")?;
    for _ in 0..nctors {
        read_function_spans(c, bytes, out)?;
    }
    // FactoryRefs + BehaviorRefs: TArray<int64> of FUNC ids (T4); 0/non-id values are
    // sentinels/behavior-type tags (remap skips anything not in the regen funcid table).
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
    let nbehav = c.read_count("Class.BehaviorFunctions")?;
    for _ in 0..nbehav {
        read_function_spans(c, bytes, out)?;
    }
    c.skip_tarray_fixed(4, "Class.BehaviorFunctionTypes")?;
    if c.read_bool4()? {
        c.read_sia()?; // SuperClass
        c.read_sia()?; // CodeSuperClass
        c.skip(8 * 4)?;
        c.read_sia()?; // StaticClassGVName
        c.skip(4)?; // bPlaceable
        c.skip_tarray_sia("Class.MetaSpec")?;
        c.skip_tarray_sia("Class.MetaValues")?;
        c.read_sia()?; // ComposeOntoClassName
    }
    Ok(())
}

fn read_enum_spans(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    c.skip_tarray_sia("Enum.Names")?;
    c.skip_tarray_fixed(4, "Enum.Values")?;
    Ok(())
}

fn read_global_spans(c: &mut Cursor, bytes: &[u8], out: &mut ModuleSpans) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    embed_datatype(c, out)?; // Type
    if !c.read_bool4()? {
        // !bIsDefaultInit
        if c.read_bool4()? {
            c.skip(8)?; // PureConstantValue
        } else if c.read_bool4()? {
            read_function_spans(c, bytes, out)?; // InitFunc carries bytecode too
        }
    }
    Ok(())
}

fn read_function_import_spans(c: &mut Cursor, out: &mut ModuleSpans) -> Result<(), WireError> {
    c.read_sia()?; // ImportedFromModule
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    let nptypes = c.read_count("Import.ParameterTypes")?;
    for _ in 0..nptypes {
        embed_datatype(c, out)?;
    }
    c.skip_tarray_fixed(4, "Import.ParameterFlags")?;
    c.skip_tarray_sia("Import.ParameterDefaultArgs")?;
    embed_datatype(c, out)?; // ReturnType
    Ok(())
}

fn read_module_spans(c: &mut Cursor, bytes: &[u8], out: &mut ModuleSpans) -> Result<(), WireError> {
    c.read_sia()?; // ModuleName
    let nfns = c.read_count("Module.Functions")?;
    for _ in 0..nfns {
        read_function_spans(c, bytes, out)?;
    }
    let nclasses = c.read_count("Module.Classes")?;
    for _ in 0..nclasses {
        read_class_spans(c, bytes, out)?;
    }
    let nenums = c.read_count("Module.Enums")?;
    for _ in 0..nenums {
        read_enum_spans(c)?;
    }
    let nglobals = c.read_count("Module.GlobalVariables")?;
    for _ in 0..nglobals {
        read_global_spans(c, bytes, out)?;
    }
    let nimports = c.read_count("Module.FunctionImports")?;
    for _ in 0..nimports {
        read_function_import_spans(c, out)?;
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
    used_static_indices: HashSet<i64>,
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
    let mut matches = base_ident_of_ptr
        .iter()
        .filter_map(|(&key, ident)| regen_ident.oracle_eq(ident).then_some(key));
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        let n = 2 + matches.count();
        return Err(RemapError::Ambiguous {
            kind,
            op,
            name: regen_name_of_ptr
                .get(&regen_key)
                .cloned()
                .unwrap_or_default(),
            n,
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
) -> Result<(), RemapError> {
    if key == 0 || plan.type_ptrs.contains_key(&key) || plan.new_types.contains(&key) {
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
) -> Result<(), RemapError> {
    if key == 0 || plan.func_ptrs.contains_key(&key) || plan.new_funcs.contains(&key) {
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
) -> Result<(), RemapError> {
    if key == 0 || plan.global_ptrs.contains_key(&key) || plan.new_globals.contains(&key) {
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

fn callee_name_from_regen<'a>(
    ins: &super::disasm::Instr,
    code: &[i32],
    regen: &'a SymTables,
) -> Option<&'a str> {
    match ins.op.name {
        "CALLSYS" | "FuncPtr" | "Thiscall1" => regen
            .func_name_of_ptr
            .get(&read_qw(code, ins.offset_dw + 1))
            .map(String::as_str),
        "CALL" | "CALLBND" | "CALLINTF" => regen
            .funcid_to_ptr
            .get(&code[ins.offset_dw + 1])
            .and_then(|ptr| regen.func_name_of_ptr.get(ptr))
            .map(String::as_str),
        _ => None,
    }
}

fn analyze_bytecode_for_new_symbols(
    code: &[i32],
    plan: &mut NewSymbolPlan,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    let instrs = disassemble(code).map_err(|e| RemapError::Disasm(e.to_string()))?;
    for (pos, ins) in instrs.iter().enumerate() {
        for site in ref_sites(ins.op.name) {
            let off = ins.offset_dw + site.dw_index;
            match site.kind {
                RefKind::GlobalPtr => {
                    declare_global(plan, read_qw(code, off), ins.op.name, regen, base)?
                }
                RefKind::FuncPtr => {
                    declare_func(plan, read_qw(code, off), ins.op.name, regen, base)?
                }
                RefKind::TypePtr => {
                    declare_type(plan, read_qw(code, off), ins.op.name, regen, base)?
                }
                RefKind::FuncId => {
                    if let Some(&ptr) = regen.funcid_to_ptr.get(&code[off]) {
                        declare_func(plan, ptr, ins.op.name, regen, base)?;
                    }
                }
                RefKind::TypeId => {
                    let (core, _) = split_type_id_operand(code[off]);
                    if let Some(&ptr) = regen.typeid_to_ptr.get(&core) {
                        declare_type(plan, ptr, ins.op.name, regen, base)?;
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
                .and_then(|next| callee_name_from_regen(next, code, regen))
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
    let key = c.read_fstring()?;
    let inner = c.read_sia()?;
    Ok([key, inner].into_iter().filter(|s| !s.is_empty()).collect())
}

fn close_type_dependencies(
    plan: &mut NewSymbolPlan,
    meta: &TailMetadata,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    loop {
        let before = plan.new_types.len() + plan.type_ptrs.len();
        let new_types: Vec<i64> = plan.new_types.iter().copied().collect();
        for key in new_types {
            let row = meta
                .type_row(key)
                .ok_or(RemapError::MissingNewRow { kind: "type", key })?;
            for &(_, dep) in &row.type_deps {
                if dep != 0 {
                    declare_type(plan, dep, "TypeRef.SubTypes", regen, base)?;
                }
            }
        }
        let new_funcs: Vec<i64> = plan.new_funcs.iter().copied().collect();
        for key in new_funcs {
            let row = meta.func_row(key).ok_or(RemapError::MissingNewRow {
                kind: "function",
                key,
            })?;
            for &(_, dep) in &row.type_deps {
                if dep != 0 {
                    declare_type(plan, dep, "FunctionReference.DataType", regen, base)?;
                }
            }
        }
        if before == plan.new_types.len() + plan.type_ptrs.len() {
            break;
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
) -> Result<(), RemapError> {
    for row in &meta.types {
        if targets.contains(&row.module) {
            declare_type(plan, row.key, "target module TypeReferences", regen, base)?;
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
            )?;
        }
    }
    for row in &meta.globals {
        if targets.contains(&row.module) {
            declare_global(plan, row.key, "target module GlobalReferences", regen, base)?;
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

fn allocate_synthetic_ptr(
    kind: u8,
    identity: &str,
    used: &HashSet<i64>,
) -> Result<i64, RemapError> {
    // OldReference is an opaque serialized lookup key. Keep synthetic keys in a stable, positive
    // high range that real Win64 heap pointers do not occupy, then linear-probe deterministically.
    let mut candidate =
        0x6000_0000_0000_0000u64 | (stable_hash64(kind, identity) & 0x0fff_ffff_ffff_ffff);
    let start = candidate;
    loop {
        let signed = candidate as i64;
        if signed != 0 && !used.contains(&signed) {
            return Ok(signed);
        }
        candidate = 0x6000_0000_0000_0000 | ((candidate + 1) & 0x0fff_ffff_ffff_ffff);
        if candidate == start {
            return Err(RemapError::KeySpaceExhausted {
                kind: "OldReference",
            });
        }
    }
}

fn allocate_new_pointer_keys(
    plan: &mut NewSymbolPlan,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    let mut symbols: Vec<(u8, String, i64)> = Vec::new();
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
        symbols.push((0, identity, key));
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
        symbols.push((1, identity, key));
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
        symbols.push((2, identity, key));
    }
    symbols.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));

    let mut used = base.all_ptr_keys.clone();
    for (kind, identity, raw) in symbols {
        // Never retain a compiler-run-local raw OldReference merely because it is free in the
        // pristine base. Independent compiler runs recycle those first-free pointer values for
        // different symbols. Identity-derived allocation makes each mini converge on the same
        // key for the same symbol, independent of regen order or raw address assignment.
        let final_key = allocate_synthetic_ptr(kind, &identity, &used)?;
        used.insert(final_key);
        match kind {
            0 => {
                plan.type_ptrs.insert(raw, final_key);
            }
            1 => {
                plan.func_ptrs.insert(raw, final_key);
            }
            _ => {
                plan.global_ptrs.insert(raw, final_key);
            }
        }
    }
    Ok(())
}

fn allocate_type_id(raw: i32, identity: &str, used: &HashSet<i32>) -> Result<i32, RemapError> {
    // AngelScript reserves the upper six bits for object-kind flags and the lower 26 for the
    // sequence number. Preserve the regen symbol's runtime-kind flags, but derive the sequence
    // from portable identity: raw sequences are first-free values recycled by independent runs.
    const SEQ_MASK: u32 = 0x03ff_ffff;
    let bits = raw as u32;
    let flags = bits & !SEQ_MASK;
    let start_seq = stable_hash64(3, identity) as u32 & SEQ_MASK;
    let mut seq = start_seq;
    loop {
        let candidate = (flags | seq) as i32;
        if !used.contains(&candidate) {
            return Ok(candidate);
        }
        seq = (seq + 1) & SEQ_MASK;
        if seq == start_seq {
            return Err(RemapError::KeySpaceExhausted { kind: "type-id" });
        }
    }
}

fn allocate_function_id(identity: &str, used: &HashSet<i32>) -> Result<i32, RemapError> {
    // Serialized function ids are non-negative i32 lookup keys (0 is a sentinel in several
    // module-record arrays). Keep synthetic ids in the positive domain and probe deterministically
    // only against the pristine base plus earlier symbols in this mini.
    const ID_MASK: u64 = i32::MAX as u64;
    let mut candidate = stable_hash64(4, identity) & ID_MASK;
    if candidate == 0 {
        candidate = 1;
    }
    let start = candidate;
    loop {
        let id = candidate as i32;
        if !used.contains(&id) {
            return Ok(id);
        }
        candidate = if candidate == ID_MASK {
            1
        } else {
            candidate + 1
        };
        if candidate == start {
            return Err(RemapError::KeySpaceExhausted {
                kind: "function-id",
            });
        }
    }
}

fn allocate_engine_ids(
    plan: &mut NewSymbolPlan,
    meta: &TailMetadata,
    regen: &SymTables,
    base: &SymTables,
) -> Result<(), RemapError> {
    let mut used_type_ids: HashSet<i32> = base.typeid_to_ptr.keys().copied().collect();
    for &ptr in &plan.new_types {
        if !meta.type_ids.iter().any(|row| row.ptr == ptr) {
            return Err(RemapError::MissingNewRow {
                kind: "type-id",
                key: ptr,
            });
        }
    }
    for row in meta
        .type_ids
        .iter()
        .filter(|row| plan.new_types.contains(&row.ptr))
    {
        let identity = regen
            .type_id_of_ptr
            .get(&row.ptr)
            .ok_or(RemapError::MissingNewRow {
                kind: "type identity",
                key: row.ptr,
            })?;
        let final_id = allocate_type_id(row.id, identity, &used_type_ids)?;
        used_type_ids.insert(final_id);
        plan.type_ids.insert(row.id, final_id);
    }

    let mut used_func_ids: HashSet<i32> = base.funcid_to_ptr.keys().copied().collect();
    for &ptr in &plan.new_funcs {
        if !meta.func_ids.iter().any(|row| row.ptr == ptr) {
            return Err(RemapError::MissingNewRow {
                kind: "function-id",
                key: ptr,
            });
        }
    }
    for row in meta
        .func_ids
        .iter()
        .filter(|row| plan.new_funcs.contains(&row.ptr))
    {
        let identity = regen
            .func_id_of_ptr
            .get(&row.ptr)
            .ok_or(RemapError::MissingNewRow {
                kind: "function identity",
                key: row.ptr,
            })?;
        let final_id = allocate_function_id(identity, &used_func_ids)?;
        used_func_ids.insert(final_id);
        plan.func_ids.insert(row.id, final_id);
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
    let final_core =
        base.ptr_to_typeid
            .get(&final_ptr)
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
    if let Some(&id) = plan.func_ids.get(&raw) {
        return Ok(id);
    }
    let Some(&regen_ptr) = regen.funcid_to_ptr.get(&raw) else {
        return Ok(raw); // sentinel / behavior tag
    };
    let final_ptr = mapped_func_ptr(plan, regen_ptr)?;
    base.ptr_to_funcid
        .get(&final_ptr)
        .copied()
        .ok_or(RemapError::MissingNewRow {
            kind: "base function-id",
            key: final_ptr,
        })
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
        let Some(&owner_ptr) = regen.typeid_to_ptr.get(&row.old_type_id) else {
            continue;
        };
        let owner_is_new = plan.new_types.contains(&owner_ptr);
        let owner_is_target = type_module
            .get(&owner_ptr)
            .is_some_and(|module| targets.contains(*module));
        if !owner_is_new && !owner_is_target {
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
                .and_then(|next| callee_name_from_regen(next, &original, regen))
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
                    counts.func_id += usize::from(regen.funcid_to_ptr.contains_key(&original[off]));
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
        for &(off, dep) in &row.type_deps {
            patch_i64_at(&mut bytes, row.start, off, mapped_type_ptr(plan, dep)?);
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
        for &(off, dep) in &row.type_deps {
            patch_i64_at(&mut bytes, row.start, off, mapped_type_ptr(plan, dep)?);
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

    let selected_globals: Vec<&GlobalRowMeta> = meta
        .globals
        .iter()
        .filter(|row| plan.new_globals.contains(&row.key))
        .collect();
    out.extend_from_slice(&(selected_globals.len() as u32).to_le_bytes());
    for row in selected_globals {
        let mut bytes = source[row.start..row.end].to_vec();
        patch_i64_at(
            &mut bytes,
            row.start,
            row.start,
            mapped_global_ptr(plan, row.key)?,
        );
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

fn remap_module_allow_new(
    extracted_mini: &[u8],
    base: &[u8],
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
    let mini_n = super::walk_modules::module_count(extracted_mini);
    if mini_n != 1 {
        return Err(RemapError::NotSingle(mini_n));
    }

    let regen = SymTables::build(extracted_mini)?;
    let base_syms = SymTables::build(base)?;
    let regen_meta = TailMetadata::build(extracted_mini)?;
    let base_meta = TailMetadata::build(base)?;
    let spans = collect_module_spans(extracted_mini)?;
    let targets = target_module_names(extracted_mini)?;

    let mod_start = CacheHeader::SIZE;
    let mod_end = module_region_end(extracted_mini)?;
    let mut module_bytes = extracted_mini[mod_start..mod_end].to_vec();
    let mut plan = NewSymbolPlan::default();

    // A declaration can be new even when no bytecode calls it yet. Seed every row declared by the
    // edited module, then add all directly referenced symbols and recursively close type deps.
    seed_target_module_symbols(&mut plan, &regen_meta, &targets, &regen, &base_syms)?;
    for span in &spans.code {
        let rel = span.data_off - mod_start;
        let code: Vec<i32> = (0..span.count)
            .map(|k| {
                let off = rel + k * 4;
                i32::from_le_bytes(module_bytes[off..off + 4].try_into().unwrap())
            })
            .collect();
        analyze_bytecode_for_new_symbols(&code, &mut plan, &regen, &base_syms)?;
    }
    for embed in &spans.embeds {
        let rel = embed.byte_off - mod_start;
        let raw = i64::from_le_bytes(module_bytes[rel..rel + 8].try_into().unwrap());
        if raw == 0 {
            continue;
        }
        match embed.kind {
            EmbedKind::TypePtr => {
                declare_type(&mut plan, raw, "embedded DataType", &regen, &base_syms)?
            }
            EmbedKind::FuncId => {
                if let Some(&ptr) = regen.funcid_to_ptr.get(&(raw as i32)) {
                    declare_func(&mut plan, ptr, "Factory/BehaviorRefs", &regen, &base_syms)?;
                }
            }
        }
    }
    close_type_dependencies(&mut plan, &regen_meta, &regen, &base_syms)?;
    allocate_new_pointer_keys(&mut plan, &regen, &base_syms)?;
    allocate_engine_ids(&mut plan, &regen_meta, &regen, &base_syms)?;
    plan_static_names(&mut plan, &regen_meta, &base_meta)?;
    plan_properties(
        &mut plan,
        &regen_meta,
        &base_meta,
        &targets,
        &regen,
        &base_syms,
    )?;

    let mut total = RemapCounts::default();
    for span in &spans.code {
        let rel = span.data_off - mod_start;
        let mut code: Vec<i32> = (0..span.count)
            .map(|k| {
                let off = rel + k * 4;
                i32::from_le_bytes(module_bytes[off..off + 4].try_into().unwrap())
            })
            .collect();
        let counts = patch_bytecode_with_new_symbols(&mut code, &plan, &regen, &base_syms)?;
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
            EmbedKind::TypePtr => {
                module_bytes[rel..rel + 8]
                    .copy_from_slice(&mapped_type_ptr(&plan, raw)?.to_le_bytes());
                total.embed_type_ptr += 1;
            }
            EmbedKind::FuncId => {
                let raw_id = raw as i32;
                if regen.funcid_to_ptr.contains_key(&raw_id) {
                    let mapped = mapped_func_id(&plan, raw_id, &regen, &base_syms)?;
                    let value = (raw & !0xffff_ffffi64) | mapped as u32 as i64;
                    module_bytes[rel..rel + 8].copy_from_slice(&value.to_le_bytes());
                    total.embed_func_id += 1;
                }
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
    let surviving: Vec<SurvivingKey> = scan_surviving_regen_keys(&module_bytes, &regen, &base_syms)
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

    let tail = emit_minimal_new_symbol_tail(extracted_mini, &regen_meta, &plan)?;
    let mut out = Vec::with_capacity(CacheHeader::SIZE + module_bytes.len() + tail.len());
    out.extend_from_slice(&extracted_mini[..0x14]);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&module_bytes);
    out.extend_from_slice(&tail);
    Ok((out, total))
}

/// Public entry: rewrite `extracted_mini`'s module bytecode refs to `base`'s keys, returning a
/// new 1-module mini whose tail tables are EMPTY (28 zero bytes). See module docs.
pub fn remap_module_to_base(
    extracted_mini: &[u8],
    base: &[u8],
) -> Result<(Vec<u8>, RemapCounts), RemapError> {
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
            EmbedKind::TypePtr => {
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
                // FactoryRefs/BehaviorRefs hold func-IDS in int64 slots, interleaved with
                // sentinels/behavior-type tags. A value present in the regen func-id table IS a
                // real func ref to remap; anything else (a behavior-type enum, a small tag) is
                // NOT and is left untouched.
                let regen_id = regen_key as i32;
                let Some(&regen_ptr) = regen.funcid_to_ptr.get(&regen_id) else {
                    continue;
                };
                let nptr = remap_ptr(
                    "function-id(embed)",
                    "Factory/BehaviorRefs",
                    regen_ptr,
                    &regen.func_id_of_ptr,
                    &regen.func_name_of_ptr,
                    &base_syms.func_ptr_of_id,
                )?;
                let new_id =
                    *base_syms
                        .ptr_to_funcid
                        .get(&nptr)
                        .ok_or_else(|| RemapError::Unresolved {
                            kind: "function-id(embed,no base id)",
                            op: "Factory/BehaviorRefs",
                            key: nptr,
                            name: base_syms
                                .func_name_of_ptr
                                .get(&nptr)
                                .cloned()
                                .unwrap_or_default(),
                        })?;
                // Preserve the high 32 bits (the regen slot stored the id in the low dword; the
                // high dword was 0 for a real id — keep whatever was there for safety on tags).
                let new_val = (regen_key & !0xFFFF_FFFFi64) | (new_id as u32 as i64);
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
    out.extend_from_slice(&extracted_mini[..0x14]); // FGuid + magic
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
    /// The identity string embeds unit-separator chars; render them as `::`-ish for readability.
    pub fn display(&self) -> String {
        match self {
            OperandId::Named { ident, .. } => ident.full.replace(SEP, " » "),
            OperandId::Primitive(id) => format!("prim#{id}"),
            OperandId::RawPtr(p) => format!("<unresolved-ptr {p:#x}>"),
            OperandId::RawId(i) => format!("<unresolved-id {i}>"),
        }
    }

    /// For a resolved FUNCTION identity (a `CALLSYS`/`CALL` callee), return the callee's
    /// (owner-type-name, method-name) as borrowed slices of the composed `Ident.full`.
    ///
    /// A T3 FunctionReference identity is composed (see `SymTables::build`) as the SEP-joined
    /// fields `module | namespace | <owner.full> | name | is_method | params | ret`, where
    /// `<owner.full>` is itself a TYPE identity of EXACTLY 4 SEP-fields
    /// (`owner_module | owner_ns | owner_name | Nsub:subs`). So on a raw `split(SEP)` the layout is
    /// positionally fixed regardless of field CONTENT:
    ///   [0]=module [1]=ns [2]=owner_module [3]=owner_ns [4]=owner_name [5]=owner_subs
    ///   [6]=name(the method) [7]=is_method [8..]=params/ret.
    /// This is the cache-INDEPENDENT keying the scope-strip needs (the raw CALLSYS func-PTR drifts
    /// across builds, but the resolved owner+method identity does not). Returns `None` for a
    /// non-`Named` operand or a malformed identity with too few fields (defensive).
    pub fn func_owner_method(&self) -> Option<(&str, &str)> {
        match self {
            OperandId::Named {
                kind: RefKind::FuncPtr | RefKind::FuncId,
                ident,
            } => {
                let mut it = ident.full.split(SEP);
                let owner = it.nth(4)?; // fields 0..=4, leaving cursor after field 4
                let method = it.nth(1)?; // field 6 (skip field 5 = owner_subs)
                Some((owner, method))
            }
            _ => None,
        }
    }

    /// TEST-ONLY constructor: build a `Named` FUNC-ptr identity from an owner-type name + method
    /// name, composing the exact SEP layout `SEP SEP SEP SEP owner SEP 0: SEP method SEP 1` so
    /// [`func_owner_method`](Self::func_owner_method) round-trips. Used by the bytediff N5 unit
    /// tests (which construct synthetic `NormInstr` CALLSYS callees).
    #[doc(hidden)]
    pub fn named_func_for_test(owner: &str, method: &str) -> OperandId {
        let full = format!("{SEP}{SEP}{SEP}{SEP}{owner}{SEP}0:{SEP}{method}{SEP}1");
        OperandId::Named {
            kind: RefKind::FuncPtr,
            ident: Ident {
                full: full.clone(),
                ns_stripped: full,
                namespaces: vec![],
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
        }
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
            },
        };
        assert_eq!(short.func_owner_method(), None);
    }
}
