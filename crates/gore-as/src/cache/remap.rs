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
         lacks — minimal-row fallback not yet implemented."
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
}

/// Field separator for composing a stable symbol identity string (unlikely in any name).
const SEP: char = '\u{1f}';

/// Symbol identity → key inverse maps for one cache's tail tables, plus the forward key→name
/// maps needed to compose function/global identities and to report unresolved refs.
struct SymTables {
    /// T1: type ptr key -> identity ; identity -> ptr key.
    type_id_of_ptr: HashMap<i64, String>,
    type_ptr_of_id: HashMap<String, Vec<i64>>,
    /// T2: type-id (i32, raw operand) -> type ptr.
    typeid_to_ptr: HashMap<i32, i64>,
    /// inverse of T2: type ptr -> type-id (raw i32 operand).
    ptr_to_typeid: HashMap<i64, i32>,
    /// T3: func ptr key -> identity ; identity -> ptr key.
    func_id_of_ptr: HashMap<i64, String>,
    func_ptr_of_id: HashMap<String, Vec<i64>>,
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
fn datatype_identity(c: &mut Cursor, type_id_of_ptr: &HashMap<i64, String>) -> Result<String, WireError> {
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
        type_id_of_ptr.get(&type_info).cloned().unwrap_or_default()
    } else {
        String::new()
    };
    Ok(format!(
        "{}{}{}{}{}{}:{token}:{tident}",
        b0 as u8, b1 as u8, b2 as u8, b3 as u8, b4 as u8, b5 as u8
    ))
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
            raw_types.push(RawType { key, name, module, namespace, subs });
        }
        // PASS 2: compose identities using resolved subtype names.
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
                rt.module, rt.namespace, rt.name, rt.subs.len()
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
            // share the bare owner name `TSubclassOf`) are distinguished.
            let owner = type_id_of_ptr.get(&objtype).cloned().unwrap_or_default();
            let nparams = c.read_count("FuncRef.Params")?;
            let mut params = String::new();
            for _ in 0..nparams {
                params.push_str(&datatype_identity(&mut c, &type_id_of_ptr)?);
                params.push(',');
            }
            let ret = datatype_identity(&mut c, &type_id_of_ptr)?;
            let identity = format!(
                "{module}{SEP}{namespace}{SEP}{owner}{SEP}{name}{SEP}{}{SEP}{params}{SEP}{ret}",
                is_method as u8
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
        let mut global_name_of_ptr = HashMap::new();
        for _ in 0..c.read_count("GlobalReferences")? {
            let key = c.read_i64()?;
            all_ptr_keys.insert(key);
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let is_string = c.read_bool4()?;
            let identity = format!("{module}{SEP}{namespace}{SEP}{name}{SEP}{}", is_string as u8);
            global_id_of_ptr.insert(key, identity.clone());
            global_ptr_of_id.entry(identity).or_default().push(key);
            global_name_of_ptr.insert(key, name);
        }
        // T6 StaticNames + T7 PropertyReferences are not operand-referenced (member ops carry a
        // TYPE-ID, not a prop key — the prop key is derived from typeid+offset), so skip them.

        Ok(SymTables {
            type_id_of_ptr,
            type_ptr_of_id,
            typeid_to_ptr,
            ptr_to_typeid,
            func_id_of_ptr,
            func_ptr_of_id,
            func_name_of_ptr,
            type_name_of_ptr,
            global_name_of_ptr,
            funcid_to_ptr,
            ptr_to_funcid,
            global_id_of_ptr,
            global_ptr_of_id,
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
        "PshGPtr" | "PshG4" | "PGA" | "LDG" => vec![RefSite { dw_index: 1, is_qword: true, kind: GlobalPtr }],
        // LdGRdR4 / CpyGtoV4: wW_QW (QW @ dword 1). CpyVtoG4: rW_QW (QW @ dword 1).
        "LdGRdR4" | "CpyGtoV4" | "CpyVtoG4" => vec![RefSite { dw_index: 1, is_qword: true, kind: GlobalPtr }],
        // SetG4: QW_DW (QW @ dword 1 = global ptr; DW @ dword 3 = literal value, NOT a ref).
        "SetG4" => vec![RefSite { dw_index: 1, is_qword: true, kind: GlobalPtr }],
        // func ptr (QW @ dword 1)
        "CALLSYS" | "FuncPtr" | "Thiscall1" => vec![RefSite { dw_index: 1, is_qword: true, kind: FuncPtr }],
        // type ptr (QW @ dword 1)
        "OBJTYPE" | "FREE" | "FinConstruct" | "CopyScript" => {
            vec![RefSite { dw_index: 1, is_qword: true, kind: TypePtr }]
        }
        // DestructScript: rW_QW (QW @ dword 1 = type ptr).
        "DestructScript" => vec![RefSite { dw_index: 1, is_qword: true, kind: TypePtr }],
        // func id (DW @ dword 1)
        "CALL" | "CALLBND" | "CALLINTF" => vec![RefSite { dw_index: 1, is_qword: false, kind: FuncId }],
        // type id (DW @ dword 1)
        "TYPEID" | "Cast" => vec![RefSite { dw_index: 1, is_qword: false, kind: TypeId }],
        // COPY: W_DW (DW @ dword 1 = type id).
        "COPY" => vec![RefSite { dw_index: 1, is_qword: false, kind: TypeId }],
        // SetListType: rW_DW_DW (type id = INTARG(bc+1) = DW @ dword 2).
        "SetListType" => vec![RefSite { dw_index: 2, is_qword: false, kind: TypeId }],
        // member type-id: ADDSi/LoadThisR (W_DW, DW @ dword 1); LoadRObjR/LoadVObjR (rW_W_DW, DW @ dword 2).
        "ADDSi" | "LoadThisR" => vec![RefSite { dw_index: 1, is_qword: false, kind: TypeId }],
        "LoadRObjR" | "LoadVObjR" => vec![RefSite { dw_index: 2, is_qword: false, kind: TypeId }],
        // ALLOC: QW_DW (type ptr @ dword 1; ctor func id @ dword 3).
        "ALLOC" => vec![
            RefSite { dw_index: 1, is_qword: true, kind: TypePtr },
            RefSite { dw_index: 3, is_qword: false, kind: FuncId },
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
        self.global_ptr + self.func_ptr + self.type_ptr + self.func_id + self.type_id
            + self.embed_type_ptr + self.embed_func_id
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
    let identity = regen_id_of_ptr.get(&regen_key).ok_or_else(|| RemapError::Unresolved {
        kind,
        op,
        key: regen_key,
        name: regen_name_of_ptr.get(&regen_key).cloned().unwrap_or_default(),
    })?;
    match base_ptr_of_id.get(identity).map(|v| v.as_slice()) {
        Some([k]) => Ok(*k),
        Some([]) | None => Err(RemapError::Unresolved {
            kind,
            op,
            key: regen_key,
            name: regen_name_of_ptr.get(&regen_key).cloned().unwrap_or_default(),
        }),
        Some(many) => Err(RemapError::Ambiguous {
            kind,
            op,
            name: regen_name_of_ptr.get(&regen_key).cloned().unwrap_or_default(),
            n: many.len(),
        }),
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
                        "global", ins.op.name, regen_key,
                        &regen.global_id_of_ptr, &regen.global_name_of_ptr, &base.global_ptr_of_id,
                    )?;
                    write_qw(code, base_off, nk);
                    counts.global_ptr += 1;
                }
                RefKind::FuncPtr => {
                    let regen_key = read_qw(code, base_off);
                    let nk = remap_ptr(
                        "function", ins.op.name, regen_key,
                        &regen.func_id_of_ptr, &regen.func_name_of_ptr, &base.func_ptr_of_id,
                    )?;
                    write_qw(code, base_off, nk);
                    counts.func_ptr += 1;
                }
                RefKind::TypePtr => {
                    let regen_key = read_qw(code, base_off);
                    let nk = remap_ptr(
                        "type", ins.op.name, regen_key,
                        &regen.type_id_of_ptr, &regen.type_name_of_ptr, &base.type_ptr_of_id,
                    )?;
                    write_qw(code, base_off, nk);
                    counts.type_ptr += 1;
                }
                RefKind::FuncId => {
                    let regen_id = code[base_off];
                    // id -> regen ptr. If absent, the id isn't a real func ref (defensive) — skip.
                    let Some(&regen_ptr) = regen.funcid_to_ptr.get(&regen_id) else { continue };
                    let nptr = remap_ptr(
                        "function-id", ins.op.name, regen_ptr,
                        &regen.func_id_of_ptr, &regen.func_name_of_ptr, &base.func_ptr_of_id,
                    )?;
                    // base ptr -> base id (the operand is the id, not the ptr).
                    let new_id = *base.ptr_to_funcid.get(&nptr).ok_or_else(|| RemapError::Unresolved {
                        kind: "function-id(no base id)", op: ins.op.name, key: nptr,
                        name: base.func_name_of_ptr.get(&nptr).cloned().unwrap_or_default(),
                    })?;
                    code[base_off] = new_id;
                    counts.func_id += 1;
                }
                RefKind::TypeId => {
                    let regen_id = code[base_off];
                    // Primitive type-ids (<= LAST_PRIMITIVE) are not in T2 — they resolve to
                    // themselves and need no remap. Skip silently (decompile-refs.md §2.5).
                    let Some(&regen_ptr) = regen.typeid_to_ptr.get(&regen_id) else { continue };
                    let nptr = remap_ptr(
                        "type-id", ins.op.name, regen_ptr,
                        &regen.type_id_of_ptr, &regen.type_name_of_ptr, &base.type_ptr_of_id,
                    )?;
                    let new_id = *base.ptr_to_typeid.get(&nptr).ok_or_else(|| RemapError::Unresolved {
                        kind: "type-id(no base id)", op: ins.op.name, key: nptr,
                        name: base.type_name_of_ptr.get(&nptr).cloned().unwrap_or_default(),
                    })?;
                    code[base_off] = new_id;
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
    out.embeds.push(EmbedRef { byte_off: dt_start + 24, kind: EmbedKind::TypePtr });
    c.skip(DATA_TYPE_SIZE)?;
    Ok(())
}

fn read_function_spans(c: &mut Cursor, bytes: &[u8], out: &mut ModuleSpans) -> Result<(), WireError> {
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
        out.embeds.push(EmbedRef { byte_off: c.pos(), kind: EmbedKind::TypePtr });
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
    out.embeds.push(EmbedRef { byte_off: c.pos(), kind: EmbedKind::TypePtr });
    c.skip(8)?; // DerivedFrom
    out.embeds.push(EmbedRef { byte_off: c.pos(), kind: EmbedKind::TypePtr });
    c.skip(8)?; // ShadowType
    let nctors = c.read_count("Class.Constructors")?;
    for _ in 0..nctors {
        read_function_spans(c, bytes, out)?;
    }
    // FactoryRefs + BehaviorRefs: TArray<int64> of FUNC ids (T4); 0/non-id values are
    // sentinels/behavior-type tags (remap skips anything not in the regen funcid table).
    let nfact = c.read_count("Class.FactoryRefs")?;
    for _ in 0..nfact {
        out.embeds.push(EmbedRef { byte_off: c.pos(), kind: EmbedKind::FuncId });
        c.skip(8)?;
    }
    let nbeh = c.read_count("Class.BehaviorRefs")?;
    for _ in 0..nbeh {
        out.embeds.push(EmbedRef { byte_off: c.pos(), kind: EmbedKind::FuncId });
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

/// Public entry: rewrite `extracted_mini`'s module bytecode refs to `base`'s keys, returning a
/// new 1-module mini whose tail tables are EMPTY (28 zero bytes). See module docs.
pub fn remap_module_to_base(extracted_mini: &[u8], base: &[u8]) -> Result<(Vec<u8>, RemapCounts), RemapError> {
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
                    "type(embed)", "ObjVar/DerivedFrom/ShadowType", regen_key,
                    &regen.type_id_of_ptr, &regen.type_name_of_ptr, &base_syms.type_ptr_of_id,
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
                let Some(&regen_ptr) = regen.funcid_to_ptr.get(&regen_id) else { continue };
                let nptr = remap_ptr(
                    "function-id(embed)", "Factory/BehaviorRefs", regen_ptr,
                    &regen.func_id_of_ptr, &regen.func_name_of_ptr, &base_syms.func_ptr_of_id,
                )?;
                let new_id = *base_syms.ptr_to_funcid.get(&nptr).ok_or_else(|| RemapError::Unresolved {
                    kind: "function-id(embed,no base id)", op: "Factory/BehaviorRefs", key: nptr,
                    name: base_syms.func_name_of_ptr.get(&nptr).cloned().unwrap_or_default(),
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
        return Err(RemapError::SurvivingRegenKeys { n: surviving.len(), shown, detail });
    }

    // Emit: FGuid+magic (from mini) + Modules count=1 + module bytes + 7 empty tables.
    let mut out = Vec::with_capacity(CacheHeader::SIZE + module_bytes.len() + 28);
    out.extend_from_slice(&extracted_mini[..0x14]); // FGuid + magic
    out.extend_from_slice(&1u32.to_le_bytes()); // Modules count = 1
    out.extend_from_slice(&module_bytes);
    out.extend_from_slice(&[0u8; 28]); // 7 tables × int32 count 0
    Ok((out, total))
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperandId {
    /// Portable identity resolved via the tail tables (the normal cross-referencing case).
    Named { kind: RefKind, identity: String },
    /// Primitive type-id (<= LAST_PRIMITIVE, not in T2) — resolves to itself. Compared by value.
    Primitive(i32),
    /// A key/id present as an operand but absent from this cache's tables (defensive: a null
    /// sentinel, or a table gap). Compared by raw value so two identical raws still match.
    RawPtr(i64),
    RawId(i32),
}

impl OperandId {
    /// Human-readable form for the SEMANTIC-DIFF report (e.g. `CALLSYS Story::GiveXP`).
    /// The identity string embeds unit-separator chars; render them as `::`-ish for readability.
    pub fn display(&self) -> String {
        match self {
            OperandId::Named { identity, .. } => identity.replace(SEP, " » "),
            OperandId::Primitive(id) => format!("prim#{id}"),
            OperandId::RawPtr(p) => format!("<unresolved-ptr {p:#x}>"),
            OperandId::RawId(i) => format!("<unresolved-id {i}>"),
        }
    }
}

impl RefIdentity {
    /// Build the identity resolver from a full cache's tail tables.
    pub fn build(bytes: &[u8]) -> Result<Self, WireError> {
        Ok(RefIdentity { syms: SymTables::build(bytes)? })
    }

    /// Resolve a QWORD ptr operand (global/func/type ptr) to a portable identity.
    pub fn resolve_ptr(&self, kind: RefKind, key: i64) -> OperandId {
        let map = match kind {
            RefKind::GlobalPtr => &self.syms.global_id_of_ptr,
            RefKind::FuncPtr => &self.syms.func_id_of_ptr,
            RefKind::TypePtr => &self.syms.type_id_of_ptr,
            // FuncId/TypeId are DW operands, not ptr — never routed here.
            RefKind::FuncId | RefKind::TypeId => return OperandId::RawPtr(key),
        };
        match map.get(&key) {
            Some(id) => OperandId::Named { kind, identity: id.clone() },
            None => OperandId::RawPtr(key),
        }
    }

    /// Resolve a DWORD id operand (func-id via T4->T3, type-id via T2->T1) to a portable
    /// identity. A type-id absent from T2 is a PRIMITIVE (int/bool/float32/...) that resolves to
    /// itself (verbatim copy of the remapper's primitive-passthrough rule, `ref-remap.md §2.5`).
    pub fn resolve_id(&self, kind: RefKind, id: i32) -> OperandId {
        match kind {
            RefKind::FuncId => match self.syms.funcid_to_ptr.get(&id) {
                Some(ptr) => match self.syms.func_id_of_ptr.get(ptr) {
                    Some(ident) => OperandId::Named { kind, identity: ident.clone() },
                    None => OperandId::RawPtr(*ptr),
                },
                // Not a real func-id in this cache: defensive, compare raw.
                None => OperandId::RawId(id),
            },
            RefKind::TypeId => match self.syms.typeid_to_ptr.get(&id) {
                Some(ptr) => match self.syms.type_id_of_ptr.get(ptr) {
                    Some(ident) => OperandId::Named { kind, identity: ident.clone() },
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
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../work/reversing/gore-as/samples/PrecompiledScript.richtest.Cache"
        ))
        .expect("read richtest sample");
        let ident = RefIdentity::build(&bytes).expect("build RefIdentity");
        // Find any T3 func ptr key and confirm resolve_ptr yields a Named identity containing
        // the function's Name (the identity is module|ns|owner|name|is_method|params|ret).
        let (&ptr, name) =
            ident.syms.func_name_of_ptr.iter().next().expect("at least one func ref");
        let resolved = ident.resolve_ptr(RefKind::FuncPtr, ptr);
        match &resolved {
            OperandId::Named { kind, identity } => {
                assert_eq!(*kind, RefKind::FuncPtr);
                assert!(
                    identity.contains(name.as_str()),
                    "identity {identity:?} should contain func name {name:?}"
                );
            }
            other => panic!("expected Named identity, got {other:?}"),
        }
        // An unknown ptr resolves to a RawPtr (defensive), NOT a panic.
        assert!(matches!(ident.resolve_ptr(RefKind::FuncPtr, 0x7fff_dead_beef), OperandId::RawPtr(_)));
        // A primitive type-id (bool == not-in-T2, small id) resolves to itself.
        assert!(matches!(ident.resolve_id(RefKind::TypeId, 0x41), OperandId::Primitive(0x41)));
    }
}
