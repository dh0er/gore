//! Raw discovery of the generated GameplayTag-to-float32 map-default bytecode shape.
//!
//! This module deliberately performs no semantic resolution and no mutation. A returned window
//! is only raw evidence: later code must still prove the initializer, target class ancestry,
//! declaring field and container schema, GameplayTag global, and exact `TMap::Add` signature
//! before it may expose a selector or change any bytes.

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::default_ancestry::DefaultNativeAncestry;
use super::disasm::{disassemble, DisasmError, Instr};
use super::walk_modules::{
    collect_function_bytecode_spans, module_region_end, FuncCodeKind, FuncCodeSpan,
};
use super::wire::{Cursor, WireError};

/// Exact encoded size of the only admitted raw window.
pub const RAW_TAG_MAP_WINDOW_DWORDS: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExactTypeIdentity {
    pub name: String,
    pub module: String,
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactDataType {
    pub is_reference: bool,
    pub is_object_const: bool,
    pub is_object_handle: bool,
    pub is_read_only: bool,
    pub is_auto: bool,
    pub if_handle_then_const: bool,
    pub type_info: i64,
    pub token: i32,
}

impl ExactDataType {
    fn read(c: &mut Cursor<'_>) -> Result<Self, ExactReferenceError> {
        Ok(Self {
            is_reference: read_canonical_bool(c, "DataType.bIsReference")?,
            is_object_const: read_canonical_bool(c, "DataType.bIsObjectConst")?,
            is_object_handle: read_canonical_bool(c, "DataType.bIsObjectHandle")?,
            is_read_only: read_canonical_bool(c, "DataType.bIsReadOnly")?,
            is_auto: read_canonical_bool(c, "DataType.bIsAuto")?,
            if_handle_then_const: read_canonical_bool(c, "DataType.bIfHandleThenConst")?,
            type_info: c.read_i64()?,
            token: c.read_i32()?,
        })
    }

    fn is_plain_void(&self) -> bool {
        !self.is_reference
            && !self.is_object_const
            && !self.is_object_handle
            && !self.is_read_only
            && !self.is_auto
            && !self.if_handle_then_const
            && self.type_info == 0
            && self.token == 0x52
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactTypeReference {
    pub identity: ExactTypeIdentity,
    pub subtypes: Vec<ExactDataType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactGlobalReference {
    pub name: String,
    pub module: String,
    pub namespace: String,
    pub is_string: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactFunctionReference {
    pub name: String,
    pub module: String,
    pub namespace: String,
    pub is_const: bool,
    pub is_imported_decl: bool,
    pub is_method: bool,
    pub object_type: i64,
    pub params: Vec<ExactDataType>,
    pub ret: ExactDataType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactPropertyReference {
    pub name: String,
    pub old_type_id: i32,
}

#[derive(Debug, Error)]
pub enum ExactReferenceError {
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error("non-canonical serialized bool {field}={value}")]
    NonCanonicalBool { field: &'static str, value: i32 },
    #[error("non-canonical string {field} at {offset:#x}: {reason}")]
    NonCanonicalString {
        field: &'static str,
        offset: usize,
        reason: String,
    },
    #[error("duplicate key {key:#x} in exact reference table {table}")]
    DuplicateKey { table: &'static str, key: i64 },
    #[error("exact reference tail ends at {end:#x}, not EOF {len:#x}")]
    TailNotAtEof { end: usize, len: usize },
}

/// Lossless mutation-oriented view of the tail identities used by the raw window.
///
/// Unlike `RefResolver`, this retains GlobalReference.Module, all three function flags, the
/// full owner TypeReference, and every raw DataType flag. It intentionally exposes no fuzzy or
/// name-only lookup.
#[derive(Debug, Clone)]
pub struct ExactReferenceIndex {
    types: HashMap<i64, ExactTypeReference>,
    type_ids: HashMap<i32, i64>,
    functions: HashMap<i64, ExactFunctionReference>,
    globals: HashMap<i64, ExactGlobalReference>,
    properties: HashMap<i64, ExactPropertyReference>,
}

impl ExactReferenceIndex {
    pub fn build(cache: &[u8]) -> Result<Self, ExactReferenceError> {
        let tail = module_region_end(cache)?;
        Self::parse_tail(cache, tail)
    }

    fn parse_tail(cache: &[u8], tail: usize) -> Result<Self, ExactReferenceError> {
        let mut c = Cursor::at(cache, tail);
        let mut types = HashMap::new();
        let type_count = c.read_count("TypeReferences")?;
        c.ensure_minimum_remaining(type_count, 24, "TypeReferences")?;
        for _ in 0..type_count {
            let key = c.read_i64()?;
            let identity = ExactTypeIdentity {
                name: read_exact_sia(&mut c, cache, "TypeReference.Name")?,
                module: read_exact_sia(&mut c, cache, "TypeReference.Module")?,
                namespace: read_exact_sia(&mut c, cache, "TypeReference.Namespace")?,
            };
            let subtype_count = c.read_count("TypeRef.SubTypes")?;
            c.ensure_minimum_remaining(subtype_count, 36, "TypeRef.SubTypes")?;
            let mut subtypes = Vec::with_capacity(subtype_count);
            for _ in 0..subtype_count {
                subtypes.push(ExactDataType::read(&mut c)?);
            }
            insert_unique(
                &mut types,
                key,
                ExactTypeReference { identity, subtypes },
                "TypeReferences",
            )?;
        }

        let mut type_ids = HashMap::new();
        let type_id_count = c.read_count("TypeIdReferenceToPointer")?;
        c.ensure_minimum_remaining(type_id_count, 12, "TypeIdReferenceToPointer")?;
        for _ in 0..type_id_count {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            insert_unique(&mut type_ids, id, ptr, "TypeIdReferenceToPointer")?;
        }

        let mut functions = HashMap::new();
        let function_count = c.read_count("FunctionReferences")?;
        c.ensure_minimum_remaining(function_count, 80, "FunctionReferences")?;
        for _ in 0..function_count {
            let key = c.read_i64()?;
            let name = read_exact_sia(&mut c, cache, "FunctionReference.Name")?;
            let module = read_exact_sia(&mut c, cache, "FunctionReference.Module")?;
            let namespace = read_exact_sia(&mut c, cache, "FunctionReference.Namespace")?;
            let is_const = read_canonical_bool(&mut c, "FunctionReference.bIsConst")?;
            let is_imported_decl =
                read_canonical_bool(&mut c, "FunctionReference.bIsImportedDecl")?;
            let is_method = read_canonical_bool(&mut c, "FunctionReference.bIsMethod")?;
            let object_type = c.read_i64()?;
            let param_count = c.read_count("FunctionReference.ParameterTypes")?;
            c.ensure_minimum_remaining(param_count, 36, "FunctionReference.ParameterTypes")?;
            let mut params = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                params.push(ExactDataType::read(&mut c)?);
            }
            let ret = ExactDataType::read(&mut c)?;
            insert_unique(
                &mut functions,
                key,
                ExactFunctionReference {
                    name,
                    module,
                    namespace,
                    is_const,
                    is_imported_decl,
                    is_method,
                    object_type,
                    params,
                    ret,
                },
                "FunctionReferences",
            )?;
        }

        let function_id_count = c.read_count("FunctionIdReferenceToPointer")?;
        c.ensure_minimum_remaining(function_id_count, 12, "FunctionIdReferenceToPointer")?;
        let mut function_ids = HashSet::with_capacity(function_id_count);
        for _ in 0..function_id_count {
            let id = c.read_i32()?;
            c.read_i64()?;
            if !function_ids.insert(id) {
                return Err(ExactReferenceError::DuplicateKey {
                    table: "FunctionIdReferenceToPointer",
                    key: i64::from(id),
                });
            }
        }

        let mut globals = HashMap::new();
        let global_count = c.read_count("GlobalReferences")?;
        c.ensure_minimum_remaining(global_count, 24, "GlobalReferences")?;
        for _ in 0..global_count {
            let key = c.read_i64()?;
            let global = ExactGlobalReference {
                name: read_exact_sia(&mut c, cache, "GlobalReference.Name")?,
                module: read_exact_sia(&mut c, cache, "GlobalReference.Module")?,
                namespace: read_exact_sia(&mut c, cache, "GlobalReference.Namespace")?,
                is_string: read_canonical_bool(&mut c, "GlobalReference.bIsString")?,
            };
            insert_unique(&mut globals, key, global, "GlobalReferences")?;
        }

        let static_name_count = c.read_count("StaticNames")?;
        c.ensure_minimum_remaining(static_name_count, 4, "StaticNames")?;
        for _ in 0..static_name_count {
            read_exact_sia(&mut c, cache, "StaticNames.Name")?;
        }

        let mut properties = HashMap::new();
        let property_count = c.read_count("PropertyReferences")?;
        c.ensure_minimum_remaining(property_count, 16, "PropertyReferences")?;
        for _ in 0..property_count {
            let key = c.read_i64()?;
            let property = ExactPropertyReference {
                name: read_exact_sia(&mut c, cache, "PropertyReference.Name")?,
                old_type_id: c.read_i32()?,
            };
            insert_unique(&mut properties, key, property, "PropertyReferences")?;
        }
        if c.pos() != cache.len() {
            return Err(ExactReferenceError::TailNotAtEof {
                end: c.pos(),
                len: cache.len(),
            });
        }
        Ok(Self {
            types,
            type_ids,
            functions,
            globals,
            properties,
        })
    }

    pub fn type_by_id(&self, id: i32) -> Option<&ExactTypeReference> {
        self.type_ids.get(&id).and_then(|ptr| self.types.get(ptr))
    }

    pub fn type_by_ptr(&self, ptr: i64) -> Option<&ExactTypeReference> {
        self.types.get(&ptr)
    }

    pub fn function_by_ptr(&self, ptr: i64) -> Option<&ExactFunctionReference> {
        self.functions.get(&ptr)
    }

    pub fn global_by_ptr(&self, ptr: i64) -> Option<&ExactGlobalReference> {
        self.globals.get(&ptr)
    }

    pub fn property(
        &self,
        owner_type_id: i32,
        member_offset: i32,
    ) -> Option<&ExactPropertyReference> {
        let key = ((owner_type_id as i64) << 1) | ((member_offset as i64) << 33) | 1;
        self.properties.get(&key)
    }
}

fn read_canonical_bool(
    c: &mut Cursor<'_>,
    field: &'static str,
) -> Result<bool, ExactReferenceError> {
    let value = c.read_i32()?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ExactReferenceError::NonCanonicalBool { field, value }),
    }
}

/// Validate the raw SIA encoding after the shared cursor advances. The general decoder is
/// intentionally tolerant and lossy for decompilation; mutation evidence must reject malformed
/// UTF-8/UTF-16, embedded terminators, and nonzero terminators instead of normalizing them.
fn read_exact_sia(
    c: &mut Cursor<'_>,
    source: &[u8],
    field: &'static str,
) -> Result<String, ExactReferenceError> {
    let start = c.pos();
    let decoded = c.read_sia()?;
    let end = c.pos();
    let raw = source.get(start..end).ok_or(WireError::Eof {
        pos: start,
        need: end.saturating_sub(start),
        have: source.len().saturating_sub(start),
    })?;
    let length_bytes = raw.get(..4).ok_or(WireError::Eof {
        pos: start,
        need: 4,
        have: raw.len(),
    })?;
    let length = i32::from_le_bytes(length_bytes.try_into().expect("four-byte length"));
    let invalid = |reason: &str| ExactReferenceError::NonCanonicalString {
        field,
        offset: start,
        reason: reason.to_owned(),
    };
    let exact = match length.cmp(&0) {
        std::cmp::Ordering::Equal => {
            if raw.len() != 4 {
                return Err(invalid("zero-length encoding has trailing bytes"));
            }
            String::new()
        }
        std::cmp::Ordering::Greater => {
            let payload = raw
                .get(4..raw.len().saturating_sub(1))
                .ok_or_else(|| invalid("positive encoding is truncated"))?;
            if raw.last() != Some(&0) {
                return Err(invalid("positive encoding has no zero terminator"));
            }
            if payload.contains(&0) {
                return Err(invalid("positive encoding contains an embedded zero"));
            }
            std::str::from_utf8(payload)
                .map_err(|_| invalid("positive encoding is not valid UTF-8"))?
                .to_owned()
        }
        std::cmp::Ordering::Less => {
            let payload = raw
                .get(4..raw.len().saturating_sub(2))
                .ok_or_else(|| invalid("wide encoding is truncated"))?;
            if raw.get(raw.len().saturating_sub(2)..) != Some(&[0, 0][..]) {
                return Err(invalid("wide encoding has no zero terminator"));
            }
            let units: Vec<_> = payload
                .chunks_exact(2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                .collect();
            if units.contains(&0) {
                return Err(invalid("wide encoding contains an embedded zero"));
            }
            String::from_utf16(&units).map_err(|_| invalid("wide encoding is not valid UTF-16"))?
        }
    };
    if exact != decoded {
        return Err(invalid("tolerant decoder did not preserve the exact text"));
    }
    Ok(exact)
}

fn insert_unique<K, V>(
    values: &mut HashMap<K, V>,
    key: K,
    value: V,
    table: &'static str,
) -> Result<(), ExactReferenceError>
where
    K: std::hash::Hash + Eq + Copy + Into<i64>,
{
    if values.insert(key, value).is_some() {
        return Err(ExactReferenceError::DuplicateKey {
            table,
            key: key.into(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagMapReferenceReject {
    MissingOwnerType,
    MissingProperty,
    PropertyOwnerMismatch,
    MissingTagGlobal,
    NonGameplayTagGlobal,
    MissingCallee,
    NonExactTMapAdd,
}

/// A raw candidate whose cache-local references and initializer shape are exact.
///
/// This is still not mutation evidence. A later layer must additionally prove that `function`
/// belongs to one unique parsed class, that the target derives from `field_owner`, and that the
/// declaring field's sealed schema is exactly `TMap<FGameplayTag,float32>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceProvenTagMapSite {
    pub function: String,
    pub raw: RawTagMapWindow,
    pub field_owner: ExactTypeIdentity,
    pub field: String,
    pub tag: ExactGlobalReference,
}

/// A reference-proven native field whose exact declared USMAP shape is additionally sealed.
/// Target-class identity and target-to-owner ancestry remain separate mandatory gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFieldProvenTagMapSite {
    pub reference: ReferenceProvenTagMapSite,
    pub field_schema_proof_id: &'static str,
}

pub fn prove_native_tag_map_field_schema(
    ancestry: &DefaultNativeAncestry,
    site: &ReferenceProvenTagMapSite,
) -> Option<NativeFieldProvenTagMapSite> {
    if !site.field_owner.module.is_empty() || !site.field_owner.namespace.is_empty() {
        return None;
    }
    let proof_id = ancestry.proves_gameplay_tag_float32_map(&site.field_owner.name, &site.field)?;
    Some(NativeFieldProvenTagMapSite {
        reference: site.clone(),
        field_schema_proof_id: proof_id,
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagMapReferenceStats {
    pub init_functions: usize,
    pub branched_init_functions: usize,
    pub raw_windows: usize,
    pub reference_proven_windows: usize,
    pub missing_owner_types: usize,
    pub missing_properties: usize,
    pub property_owner_mismatches: usize,
    pub missing_tag_globals: usize,
    pub non_gameplay_tag_globals: usize,
    pub missing_callees: usize,
    pub non_exact_tmap_add_callees: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagMapReferenceReport {
    pub sites: Vec<ReferenceProvenTagMapSite>,
    pub stats: TagMapReferenceStats,
}

#[derive(Debug, Error)]
pub enum TagMapReferenceScanError {
    #[error(transparent)]
    ExactReferences(#[from] ExactReferenceError),
    #[error("failed to walk function bytecode spans: {0}")]
    Walk(String),
    #[error("failed to disassemble {function}: {error}")]
    Disasm { function: String, error: String },
}

/// Prove exact initializer and reference identities across a whole cache.
///
/// This deliberately stops before class ancestry and field-schema proof, so callers cannot use
/// the result as a mutation selector by itself.
pub fn reference_proven_tag_map_sites(
    cache: &[u8],
) -> Result<TagMapReferenceReport, TagMapReferenceScanError> {
    let refs = ExactReferenceIndex::build(cache)?;
    let spans = collect_function_bytecode_spans(cache)
        .map_err(|error| TagMapReferenceScanError::Walk(error.to_string()))?;
    let mut sites = Vec::new();
    let mut stats = TagMapReferenceStats::default();

    for span in &spans {
        if !is_initializer_metadata(span) {
            continue;
        }
        stats.init_functions += 1;
        let instrs =
            disassemble(&span.code.bytecode).map_err(|error| TagMapReferenceScanError::Disasm {
                function: span.code.func.clone(),
                error: error.to_string(),
            })?;
        if instrs
            .iter()
            .any(|instruction| is_branch(instruction.op.name))
        {
            stats.branched_init_functions += 1;
        }
        if !is_reachable_linear_initializer(&instrs) {
            continue;
        }
        let windows = raw_tag_map_windows(&span.code.bytecode, &instrs);
        stats.raw_windows += windows.len();
        for raw in windows {
            match prove_reference_window(&refs, &raw) {
                Ok((field_owner, field, tag)) => {
                    stats.reference_proven_windows += 1;
                    sites.push(ReferenceProvenTagMapSite {
                        function: span.code.func.clone(),
                        raw,
                        field_owner,
                        field,
                        tag,
                    });
                }
                Err(TagMapReferenceReject::MissingOwnerType) => stats.missing_owner_types += 1,
                Err(TagMapReferenceReject::MissingProperty) => stats.missing_properties += 1,
                Err(TagMapReferenceReject::PropertyOwnerMismatch) => {
                    stats.property_owner_mismatches += 1
                }
                Err(TagMapReferenceReject::MissingTagGlobal) => stats.missing_tag_globals += 1,
                Err(TagMapReferenceReject::NonGameplayTagGlobal) => {
                    stats.non_gameplay_tag_globals += 1
                }
                Err(TagMapReferenceReject::MissingCallee) => stats.missing_callees += 1,
                Err(TagMapReferenceReject::NonExactTMapAdd) => {
                    stats.non_exact_tmap_add_callees += 1
                }
            }
        }
    }
    Ok(TagMapReferenceReport { sites, stats })
}

fn prove_reference_window(
    refs: &ExactReferenceIndex,
    raw: &RawTagMapWindow,
) -> Result<(ExactTypeIdentity, String, ExactGlobalReference), TagMapReferenceReject> {
    let owner = refs
        .type_by_id(raw.owner_type_id)
        .ok_or(TagMapReferenceReject::MissingOwnerType)?;
    let property = refs
        .property(raw.owner_type_id, raw.member_offset)
        .ok_or(TagMapReferenceReject::MissingProperty)?;
    if property.old_type_id != raw.owner_type_id {
        return Err(TagMapReferenceReject::PropertyOwnerMismatch);
    }
    let tag = refs
        .global_by_ptr(raw.tag_global_ptr)
        .ok_or(TagMapReferenceReject::MissingTagGlobal)?;
    if tag.name.is_empty()
        || !tag.module.is_empty()
        || tag.namespace != "GameplayTag"
        || tag.is_string
    {
        return Err(TagMapReferenceReject::NonGameplayTagGlobal);
    }
    let callee = refs
        .function_by_ptr(raw.callee_func_ptr)
        .ok_or(TagMapReferenceReject::MissingCallee)?;
    if !is_exact_tmap_gameplay_tag_float32_add(refs, callee) {
        return Err(TagMapReferenceReject::NonExactTMapAdd);
    }
    Ok((owner.identity.clone(), property.name.clone(), tag.clone()))
}

fn is_exact_tmap_gameplay_tag_float32_add(
    refs: &ExactReferenceIndex,
    callee: &ExactFunctionReference,
) -> bool {
    if callee.name != "Add"
        || !callee.module.is_empty()
        || !callee.namespace.is_empty()
        || callee.is_const
        || callee.is_imported_decl
        || !callee.is_method
        || !callee.ret.is_plain_void()
    {
        return false;
    }
    let Some(owner) = refs.type_by_ptr(callee.object_type) else {
        return false;
    };
    if owner.identity
        != (ExactTypeIdentity {
            name: "TMap".into(),
            module: String::new(),
            namespace: String::new(),
        })
        || owner.subtypes.len() != 2
        || callee.params.len() != 2
    {
        return false;
    }
    let key = &owner.subtypes[0];
    let value = &owner.subtypes[1];
    let key_param = &callee.params[0];
    let value_param = &callee.params[1];
    is_plain_gameplay_tag(refs, key)
        && is_plain_float32(value)
        && is_const_reference_to(key_param, key)
        && is_const_reference_to(value_param, value)
}

fn is_plain_gameplay_tag(refs: &ExactReferenceIndex, value: &ExactDataType) -> bool {
    is_plain_value(value)
        && value.token == 5
        && refs.type_by_ptr(value.type_info).is_some_and(|reference| {
            reference.identity
                == (ExactTypeIdentity {
                    name: "FGameplayTag".into(),
                    module: String::new(),
                    namespace: String::new(),
                })
                && reference.subtypes.is_empty()
        })
}

fn is_plain_float32(value: &ExactDataType) -> bool {
    is_plain_value(value) && value.type_info == 0 && value.token == 0x50
}

fn is_plain_value(value: &ExactDataType) -> bool {
    !value.is_reference
        && !value.is_object_const
        && !value.is_object_handle
        && !value.is_read_only
        && !value.is_auto
        && !value.if_handle_then_const
}

fn is_const_reference_to(reference: &ExactDataType, value: &ExactDataType) -> bool {
    reference.is_reference
        && reference.is_object_const
        && !reference.is_object_handle
        && reference.is_read_only
        && !reference.is_auto
        && !reference.if_handle_then_const
        && reference.type_info == value.type_info
        && reference.token == value.token
}

fn is_initializer_metadata(span: &FuncCodeSpan) -> bool {
    span.kind == FuncCodeKind::ClassMethod
        && span.method_table_valid
        && span.in_method_table
        && matches!(span.function_traits, 0 | 0x20)
        && span.code.is_method
        && span.code.func.ends_with("::__InitDefaults")
        && span.code.param_types.is_empty()
        && is_plain_model_void(&span.code.ret)
}

fn is_plain_model_void(value: &super::types::DataType) -> bool {
    !value.is_reference
        && !value.is_object_const
        && !value.is_object_handle
        && !value.is_read_only
        && !value.is_auto
        && !value.if_handle_then_const
        && value.type_info == 0
        && value.token == 0x52
}

fn is_branch(name: &str) -> bool {
    matches!(
        name,
        "JMP" | "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JMPP" | "JLowZ" | "JLowNZ"
    )
}

fn is_reachable_linear_initializer(instrs: &[Instr]) -> bool {
    if instrs
        .iter()
        .any(|instruction| is_branch(instruction.op.name))
    {
        return false;
    }
    let Some((last, prefix)) = instrs.split_last() else {
        return false;
    };
    last.op.name == "RET"
        && prefix
            .iter()
            .all(|instruction| !matches!(instruction.op.name, "RET" | "ThrowException"))
}

/// One exact, contiguous raw candidate:
///
/// `SetV4 value, immediate; PSF value; PshGPtr tag; PshVPtr this;`
/// `ADDSi member_offset, owner_type_id; CALLSYS callee`
///
/// Offsets, ids, and pointers are provenance only. None of them is a semantic selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTagMapWindow {
    pub instruction_index: usize,
    pub instruction_offset_dw: usize,
    /// Dword containing the complete four-byte `SetV4` immediate.
    pub operand_offset_dw: usize,
    pub value_slot: u16,
    pub expected: [u8; 4],
    pub owner_type_id: i32,
    pub member_offset: i32,
    pub tag_global_ptr: i64,
    pub callee_func_ptr: i64,
    /// SHA-256 of all six instructions with only the four-byte immediate zeroed.
    pub context_sha256: String,
}

/// Disassemble one function and return only exact raw GameplayTag-map candidates.
pub fn scan_raw_tag_map_windows(bytecode: &[i32]) -> Result<Vec<RawTagMapWindow>, DisasmError> {
    let instrs = disassemble(bytecode)?;
    Ok(raw_tag_map_windows(bytecode, &instrs))
}

/// Match instructions produced from this same bytecode against the exact six-opcode wire shape.
/// Kept private so callers cannot pair an unrelated instruction list with different raw bytes.
fn raw_tag_map_windows(bytecode: &[i32], instrs: &[Instr]) -> Vec<RawTagMapWindow> {
    let mut result = Vec::new();
    for (instruction_index, six) in instrs.windows(6).enumerate() {
        let [set, value_address, tag, receiver, member, call] = six else {
            unreachable!("windows(6) always yields six instructions")
        };
        if set.op.name != "SetV4"
            || value_address.op.name != "PSF"
            || tag.op.name != "PshGPtr"
            || receiver.op.name != "PshVPtr"
            || member.op.name != "ADDSi"
            || call.op.name != "CALLSYS"
        {
            continue;
        }

        let (
            [value_slot],
            [immediate],
            [address_slot],
            [tag_global_ptr],
            [receiver_slot],
            [member_offset],
            [owner_type_id],
            [callee_func_ptr],
        ) = (
            set.words.as_slice(),
            set.dwords.as_slice(),
            value_address.words.as_slice(),
            tag.qwords.as_slice(),
            receiver.words.as_slice(),
            member.words.as_slice(),
            member.dwords.as_slice(),
            call.qwords.as_slice(),
        )
        else {
            continue;
        };
        if value_slot != address_slot || *receiver_slot != 0 {
            continue;
        }

        // Require the exact instruction sizes and contiguity, not merely six matching decoded
        // names. This also seals the relative immediate position used by a later CAS patcher.
        let Some(psf_offset) = set.offset_dw.checked_add(2) else {
            continue;
        };
        let Some(tag_offset) = set.offset_dw.checked_add(3) else {
            continue;
        };
        let Some(receiver_offset) = set.offset_dw.checked_add(6) else {
            continue;
        };
        let Some(member_offset_dw) = set.offset_dw.checked_add(7) else {
            continue;
        };
        let Some(call_offset) = set.offset_dw.checked_add(9) else {
            continue;
        };
        let Some(window_end) = set.offset_dw.checked_add(RAW_TAG_MAP_WINDOW_DWORDS) else {
            continue;
        };
        if value_address.offset_dw != psf_offset
            || tag.offset_dw != tag_offset
            || receiver.offset_dw != receiver_offset
            || member.offset_dw != member_offset_dw
            || call.offset_dw != call_offset
            || call.offset_dw.checked_add(call.op.size_dwords as usize) != Some(window_end)
        {
            continue;
        }

        let Some(operand_offset_dw) = set.offset_dw.checked_add(1) else {
            continue;
        };
        let Some(context_sha256) = context_hash(bytecode, set.offset_dw, operand_offset_dw) else {
            continue;
        };
        result.push(RawTagMapWindow {
            instruction_index,
            instruction_offset_dw: set.offset_dw,
            operand_offset_dw,
            value_slot: *value_slot,
            expected: immediate.to_le_bytes(),
            owner_type_id: *owner_type_id as i32,
            member_offset: i32::from(*member_offset),
            tag_global_ptr: *tag_global_ptr as i64,
            callee_func_ptr: *callee_func_ptr as i64,
            context_sha256,
        });
    }
    result
}

fn context_hash(bytecode: &[i32], start_dw: usize, operand_offset_dw: usize) -> Option<String> {
    let end_dw = start_dw.checked_add(RAW_TAG_MAP_WINDOW_DWORDS)?;
    let words = bytecode.get(start_dw..end_dw)?;
    let mut bytes = Vec::with_capacity(RAW_TAG_MAP_WINDOW_DWORDS * 4);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    let relative = operand_offset_dw.checked_sub(start_dw)?.checked_mul(4)?;
    bytes.get_mut(relative..relative.checked_add(4)?)?.fill(0);
    Some(encode_hex(&Sha256::digest(bytes)))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::types::DataType;
    use super::super::walk_modules::FuncCode;
    use super::*;

    const TAG_PTR: u64 = 0x024b_dd09_8bc8;
    const CALLEE_PTR: u64 = 0x024b_ef73_8200;
    const OWNER_TYPE_ID: u32 = 0x0400_1269;

    fn op(opcode: u8, word: u16) -> i32 {
        (u32::from(opcode) | (u32::from(word) << 16)) as i32
    }

    fn qword_op(opcode: u8, value: u64, out: &mut Vec<i32>) {
        out.extend([
            i32::from(opcode),
            value as u32 as i32,
            (value >> 32) as u32 as i32,
        ]);
    }

    fn exact_window() -> Vec<i32> {
        let mut code = vec![op(77, 2), 0x4120_0000]; // SetV4 w2, 10.0f
        code.push(op(4, 2)); // PSF w2
        qword_op(1, TAG_PTR, &mut code); // PshGPtr tag
        code.push(op(48, 0)); // PshVPtr this
        code.extend([op(79, 800), OWNER_TYPE_ID as i32]); // ADDSi field, owner
        qword_op(61, CALLEE_PTR, &mut code); // CALLSYS callee
        code
    }

    fn plain_identifier(type_info: i64) -> ExactDataType {
        ExactDataType {
            type_info,
            token: 5,
            ..plain_void()
        }
    }

    fn plain_float32() -> ExactDataType {
        ExactDataType {
            token: 0x50,
            ..plain_void()
        }
    }

    fn plain_void() -> ExactDataType {
        ExactDataType {
            is_reference: false,
            is_object_const: false,
            is_object_handle: false,
            is_read_only: false,
            is_auto: false,
            if_handle_then_const: false,
            type_info: 0,
            token: 0x52,
        }
    }

    fn const_reference_to(value: &ExactDataType) -> ExactDataType {
        ExactDataType {
            is_reference: true,
            is_object_const: true,
            is_read_only: true,
            type_info: value.type_info,
            token: value.token,
            ..plain_void()
        }
    }

    fn exact_reference_index() -> ExactReferenceIndex {
        const OWNER_PTR: i64 = 0x1000;
        const TAG_TYPE_PTR: i64 = 0x2000;
        const TMAP_PTR: i64 = 0x3000;
        let key = plain_identifier(TAG_TYPE_PTR);
        let value = plain_float32();
        ExactReferenceIndex {
            types: HashMap::from([
                (
                    OWNER_PTR,
                    ExactTypeReference {
                        identity: ExactTypeIdentity {
                            name: "UWeaponDefinition".into(),
                            module: String::new(),
                            namespace: String::new(),
                        },
                        subtypes: Vec::new(),
                    },
                ),
                (
                    TAG_TYPE_PTR,
                    ExactTypeReference {
                        identity: ExactTypeIdentity {
                            name: "FGameplayTag".into(),
                            module: String::new(),
                            namespace: String::new(),
                        },
                        subtypes: Vec::new(),
                    },
                ),
                (
                    TMAP_PTR,
                    ExactTypeReference {
                        identity: ExactTypeIdentity {
                            name: "TMap".into(),
                            module: String::new(),
                            namespace: String::new(),
                        },
                        subtypes: vec![key.clone(), value.clone()],
                    },
                ),
            ]),
            type_ids: HashMap::from([(OWNER_TYPE_ID as i32, OWNER_PTR)]),
            functions: HashMap::from([(
                CALLEE_PTR as i64,
                ExactFunctionReference {
                    name: "Add".into(),
                    module: String::new(),
                    namespace: String::new(),
                    is_const: false,
                    is_imported_decl: false,
                    is_method: true,
                    object_type: TMAP_PTR,
                    params: vec![const_reference_to(&key), const_reference_to(&value)],
                    ret: plain_void(),
                },
            )]),
            globals: HashMap::from([(
                TAG_PTR as i64,
                ExactGlobalReference {
                    name: "Item_Damage_Physical_Edge".into(),
                    module: String::new(),
                    namespace: "GameplayTag".into(),
                    is_string: false,
                },
            )]),
            properties: HashMap::from([(
                ((OWNER_TYPE_ID as i64) << 1) | (800i64 << 33) | 1,
                ExactPropertyReference {
                    name: "m_DamageBase".into(),
                    old_type_id: OWNER_TYPE_ID as i32,
                },
            )]),
        }
    }

    fn raw_window() -> RawTagMapWindow {
        scan_raw_tag_map_windows(&exact_window())
            .unwrap()
            .pop()
            .unwrap()
    }

    fn push_sia(out: &mut Vec<u8>, value: &str) {
        if value.is_empty() {
            out.extend_from_slice(&0i32.to_le_bytes());
        } else {
            out.extend_from_slice(&(value.len() as i32).to_le_bytes());
            out.extend_from_slice(value.as_bytes());
            out.push(0);
        }
    }

    fn push_data_type(out: &mut Vec<u8>, value: &ExactDataType) {
        for flag in [
            value.is_reference,
            value.is_object_const,
            value.is_object_handle,
            value.is_read_only,
            value.is_auto,
            value.if_handle_then_const,
        ] {
            out.extend_from_slice(&i32::from(flag).to_le_bytes());
        }
        out.extend_from_slice(&value.type_info.to_le_bytes());
        out.extend_from_slice(&value.token.to_le_bytes());
    }

    fn exact_tail_fixture(function_const: i32) -> Vec<u8> {
        exact_tail_fixture_with_duplicate_type(function_const, false)
    }

    fn exact_tail_fixture_with_duplicate_type(
        function_const: i32,
        duplicate_type: bool,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(if duplicate_type { 2i32 } else { 1i32 }).to_le_bytes());
        let mut type_entry = Vec::new();
        type_entry.extend_from_slice(&0x1000i64.to_le_bytes());
        push_sia(&mut type_entry, "UOwner");
        push_sia(&mut type_entry, "OwnerModule");
        push_sia(&mut type_entry, "OwnerNamespace");
        type_entry.extend_from_slice(&0i32.to_le_bytes()); // subtypes
        out.extend_from_slice(&type_entry);
        if duplicate_type {
            out.extend_from_slice(&type_entry);
        }

        out.extend_from_slice(&1i32.to_le_bytes()); // TypeIdReferenceToPointer
        out.extend_from_slice(&7i32.to_le_bytes());
        out.extend_from_slice(&0x1000i64.to_le_bytes());

        out.extend_from_slice(&1i32.to_le_bytes()); // FunctionReferences
        out.extend_from_slice(&0x2000i64.to_le_bytes());
        push_sia(&mut out, "Call");
        push_sia(&mut out, "FunctionModule");
        push_sia(&mut out, "FunctionNamespace");
        out.extend_from_slice(&function_const.to_le_bytes());
        out.extend_from_slice(&1i32.to_le_bytes()); // imported
        out.extend_from_slice(&1i32.to_le_bytes()); // method
        out.extend_from_slice(&0x1000i64.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // params
        push_data_type(&mut out, &plain_void());

        out.extend_from_slice(&0i32.to_le_bytes()); // FunctionIdReferenceToPointer

        out.extend_from_slice(&1i32.to_le_bytes()); // GlobalReferences
        out.extend_from_slice(&0x3000i64.to_le_bytes());
        push_sia(&mut out, "Tag");
        push_sia(&mut out, "GlobalModule");
        push_sia(&mut out, "GlobalNamespace");
        out.extend_from_slice(&0i32.to_le_bytes()); // non-string

        out.extend_from_slice(&0i32.to_le_bytes()); // StaticNames

        out.extend_from_slice(&1i32.to_le_bytes()); // PropertyReferences
        out.extend_from_slice(&0x4000i64.to_le_bytes());
        push_sia(&mut out, "Field");
        out.extend_from_slice(&7i32.to_le_bytes());
        out
    }

    fn initializer_span(bytecode: Vec<i32>) -> FuncCodeSpan {
        FuncCodeSpan {
            code: FuncCode {
                func: "Items.UFixture::__InitDefaults".into(),
                is_method: true,
                param_names: Vec::new(),
                param_types: Vec::new(),
                ret: DataType {
                    token: 0x52,
                    ..DataType::default()
                },
                bytecode,
            },
            kind: FuncCodeKind::ClassMethod,
            function_traits: 0x20,
            method_table_valid: true,
            in_method_table: true,
            bytecode_offset: 0,
        }
    }

    #[test]
    fn finds_exact_sword_shaped_raw_window() {
        let windows = scan_raw_tag_map_windows(&exact_window()).unwrap();
        assert_eq!(windows.len(), 1);
        let window = &windows[0];
        assert_eq!(window.instruction_index, 0);
        assert_eq!(window.instruction_offset_dw, 0);
        assert_eq!(window.operand_offset_dw, 1);
        assert_eq!(window.value_slot, 2);
        assert_eq!(window.expected, 10.0f32.to_le_bytes());
        assert_eq!(window.owner_type_id, OWNER_TYPE_ID as i32);
        assert_eq!(window.member_offset, 800);
        assert_eq!(window.tag_global_ptr, TAG_PTR as i64);
        assert_eq!(window.callee_func_ptr, CALLEE_PTR as i64);
        assert_eq!(
            window.context_sha256,
            "d02d0b0a7bd68cdae2d2e04b530fa959a94c2270cf178d406f64c474f1840312"
        );
    }

    #[test]
    fn preserves_arbitrary_raw_ids_without_claiming_semantics() {
        let mut code = exact_window();
        code[4] = 0x5566_7788;
        code[5] = 0x1122_3344;
        code[8] = 0x7654_3210;
        code[10] = 0x0bad_f00d;
        code[11] = 0x0123_4567;
        let window = scan_raw_tag_map_windows(&code).unwrap().pop().unwrap();
        assert_eq!(window.tag_global_ptr as u64, 0x1122_3344_5566_7788);
        assert_eq!(window.owner_type_id as u32, 0x7654_3210);
        assert_eq!(window.callee_func_ptr as u64, 0x0123_4567_0bad_f00d);
    }

    #[test]
    fn exact_tail_parser_retains_modules_flags_and_full_type_identity() {
        let tail = exact_tail_fixture(0);
        let refs = ExactReferenceIndex::parse_tail(&tail, 0).unwrap();
        assert_eq!(
            refs.type_by_id(7).unwrap().identity,
            ExactTypeIdentity {
                name: "UOwner".into(),
                module: "OwnerModule".into(),
                namespace: "OwnerNamespace".into(),
            }
        );
        let function = refs.function_by_ptr(0x2000).unwrap();
        assert_eq!(function.module, "FunctionModule");
        assert_eq!(function.namespace, "FunctionNamespace");
        assert!(!function.is_const);
        assert!(function.is_imported_decl);
        assert!(function.is_method);
        let global = refs.global_by_ptr(0x3000).unwrap();
        assert_eq!(global.module, "GlobalModule");
        assert_eq!(global.namespace, "GlobalNamespace");
    }

    #[test]
    fn exact_tail_parser_rejects_noncanonical_bool() {
        assert!(matches!(
            ExactReferenceIndex::parse_tail(&exact_tail_fixture(2), 0),
            Err(ExactReferenceError::NonCanonicalBool {
                field: "FunctionReference.bIsConst",
                value: 2
            })
        ));
    }

    #[test]
    fn exact_tail_parser_rejects_lossy_string_identity() {
        let mut tail = exact_tail_fixture(0);
        // count(4) + key(8) + SIA length(4) => first TypeReference.Name payload byte.
        tail[16] = 0xff;
        assert!(matches!(
            ExactReferenceIndex::parse_tail(&tail, 0),
            Err(ExactReferenceError::NonCanonicalString {
                field: "TypeReference.Name",
                ..
            })
        ));
    }

    #[test]
    fn exact_tail_parser_rejects_duplicate_keys_and_trailing_bytes() {
        assert!(matches!(
            ExactReferenceIndex::parse_tail(&exact_tail_fixture_with_duplicate_type(0, true), 0),
            Err(ExactReferenceError::DuplicateKey {
                table: "TypeReferences",
                key: 0x1000
            })
        ));
        let mut trailing = exact_tail_fixture(0);
        trailing.push(0);
        assert!(matches!(
            ExactReferenceIndex::parse_tail(&trailing, 0),
            Err(ExactReferenceError::TailNotAtEof { .. })
        ));
    }

    #[test]
    fn exact_reference_proof_accepts_only_the_complete_identity() {
        let raw = raw_window();
        let refs = exact_reference_index();
        let (owner, field, tag) = prove_reference_window(&refs, &raw).unwrap();
        assert_eq!(owner.name, "UWeaponDefinition");
        assert_eq!(field, "m_DamageBase");
        assert_eq!(tag.name, "Item_Damage_Physical_Edge");

        let mut drifted = refs.clone();
        drifted.globals.get_mut(&(TAG_PTR as i64)).unwrap().module = "DifferentModule".into();
        assert_eq!(
            prove_reference_window(&drifted, &raw),
            Err(TagMapReferenceReject::NonGameplayTagGlobal)
        );

        let mut drifted = refs.clone();
        drifted
            .functions
            .get_mut(&(CALLEE_PTR as i64))
            .unwrap()
            .is_imported_decl = true;
        assert_eq!(
            prove_reference_window(&drifted, &raw),
            Err(TagMapReferenceReject::NonExactTMapAdd)
        );

        let mut drifted = refs.clone();
        drifted
            .functions
            .get_mut(&(CALLEE_PTR as i64))
            .unwrap()
            .params[1]
            .token = 0x44;
        assert_eq!(
            prove_reference_window(&drifted, &raw),
            Err(TagMapReferenceReject::NonExactTMapAdd)
        );

        let mut drifted = refs;
        drifted.properties.values_mut().next().unwrap().old_type_id += 1;
        assert_eq!(
            prove_reference_window(&drifted, &raw),
            Err(TagMapReferenceReject::PropertyOwnerMismatch)
        );
    }

    #[test]
    fn opaque_native_field_proof_binds_exact_owner_field_and_map_profile() {
        let raw = raw_window();
        let refs = exact_reference_index();
        let (field_owner, field, tag) = prove_reference_window(&refs, &raw).unwrap();
        let site = ReferenceProvenTagMapSite {
            function: "Items.UFixture::__InitDefaults".into(),
            raw,
            field_owner,
            field,
            tag,
        };
        let profile = DefaultNativeAncestry::from_test_edges_and_maps(
            &[("UWeaponDefinition", None)],
            &[("UWeaponDefinition", "m_DamageBase")],
        );
        let proven = prove_native_tag_map_field_schema(&profile, &site).unwrap();
        assert_eq!(
            proven.field_schema_proof_id,
            super::super::default_ancestry::DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        );

        let mut wrong_case = site.clone();
        wrong_case.field = "m_damageBase".into();
        assert!(prove_native_tag_map_field_schema(&profile, &wrong_case).is_none());
        let mut wrong_module = site;
        wrong_module.field_owner.module = "Foreign".into();
        assert!(prove_native_tag_map_field_schema(&profile, &wrong_module).is_none());
    }

    #[test]
    fn initializer_gate_rejects_metadata_branches_early_ret_and_throw() {
        let mut linear = exact_window();
        linear.push(op(10, 0)); // final RET
        let span = initializer_span(linear.clone());
        assert!(is_initializer_metadata(&span));
        assert!(is_reachable_linear_initializer(
            &disassemble(&linear).unwrap()
        ));

        let mut wrong_traits = span.clone();
        wrong_traits.function_traits = 1;
        assert!(!is_initializer_metadata(&wrong_traits));
        let mut invalid_table = span.clone();
        invalid_table.method_table_valid = false;
        assert!(!is_initializer_metadata(&invalid_table));

        let mut early_ret = exact_window();
        early_ret.push(op(10, 0));
        early_ret.push(op(10, 0));
        assert!(!is_reachable_linear_initializer(
            &disassemble(&early_ret).unwrap()
        ));

        let branched = vec![11, 0, op(10, 0)]; // JMP then RET
        assert!(!is_reachable_linear_initializer(
            &disassemble(&branched).unwrap()
        ));
        let thrown = vec![op(212, 0), op(10, 0)];
        assert!(!is_reachable_linear_initializer(
            &disassemble(&thrown).unwrap()
        ));
    }

    #[test]
    fn configured_shipping_reference_census_is_stable() {
        let Some(path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
            eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
            return;
        };
        let path = std::path::PathBuf::from(path);
        let cache = std::fs::read(&path).expect("read configured Shipping cache");
        let report = reference_proven_tag_map_sites(&cache).expect("reference-proven tag maps");
        assert_eq!(report.stats.init_functions, 29_951);
        assert_eq!(report.stats.branched_init_functions, 1);
        assert_eq!(report.stats.raw_windows, 1_432);
        assert_eq!(report.stats.reference_proven_windows, 1_432);
        assert_eq!(report.stats.missing_owner_types, 0);
        assert_eq!(report.stats.missing_properties, 0);
        assert_eq!(report.stats.property_owner_mismatches, 0);
        assert_eq!(report.stats.missing_tag_globals, 0);
        assert_eq!(report.stats.non_gameplay_tag_globals, 0);
        assert_eq!(report.stats.missing_callees, 0);
        assert_eq!(report.stats.non_exact_tmap_add_callees, 0);
        let sword: Vec<_> = report
            .sites
            .iter()
            .filter(|site| {
                site.function
                    .ends_with("UItMw_1H_Sword_Old_01::__InitDefaults")
                    && site.field == "m_DamageBase"
                    && site.tag.name == "Item_Damage_Physical_Edge"
            })
            .collect();
        assert_eq!(sword.len(), 1);
        assert_eq!(sword[0].raw.expected, 10.0f32.to_le_bytes());

        let Some(usmap_path) = std::env::var_os("GORE_AS_DEFAULT_USMAP") else {
            eprintln!("skip native field proof: set GORE_AS_DEFAULT_USMAP");
            return;
        };
        let native = super::super::binds::NativeApi::load(
            &path.parent().expect("Script directory").join("Binds.Cache"),
        )
        .expect("load sibling Binds");
        let usmap = std::fs::read(usmap_path).expect("read configured USMAP");
        let schemas = gore_asset::SchemaDb::from_usmap(&usmap).expect("parse configured USMAP");
        let profile = DefaultNativeAncestry::from_schema_db(&native, &cache, &schemas)
            .expect("build sealed ancestry and field profile");
        let field_proven: Vec<_> = report
            .sites
            .iter()
            .filter_map(|site| prove_native_tag_map_field_schema(&profile, site))
            .collect();
        assert_eq!(field_proven.len(), 1_432);
        assert!(field_proven.iter().any(|site| {
            site.reference
                .function
                .ends_with("UItMw_1H_Sword_Old_01::__InitDefaults")
                && site.reference.field == "m_DamageBase"
                && site.reference.tag.name == "Item_Damage_Physical_Edge"
                && site.field_schema_proof_id
                    == super::super::default_ancestry::DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
        }));
    }

    #[test]
    fn rejects_slot_receiver_opcode_and_contiguity_drift() {
        let mut cases = Vec::new();

        let mut slot_mismatch = exact_window();
        slot_mismatch[2] = op(4, 3);
        cases.push(("slot mismatch", slot_mismatch));

        let mut non_this_receiver = exact_window();
        non_this_receiver[6] = op(48, 1);
        cases.push(("non-this receiver", non_this_receiver));

        let mut non_global_tag = exact_window();
        non_global_tag[3] = 7; // PshG4, same qword wire width
        cases.push(("non-pointer tag push", non_global_tag));

        let mut non_callsys = exact_window();
        non_callsys[9] = 200; // Thiscall1, same qword wire width
        cases.push(("non-CALLSYS callee", non_callsys));

        let mut interrupted = exact_window();
        interrupted.insert(2, 0); // PopPtr between SetV4 and PSF
        cases.push(("interrupted window", interrupted));

        for (name, code) in cases {
            assert!(
                scan_raw_tag_map_windows(&code).unwrap().is_empty(),
                "{name} must fail closed"
            );
        }
    }

    #[test]
    fn rejects_truncated_bytecode_before_scanning() {
        let mut code = exact_window();
        code.pop();
        assert!(matches!(
            scan_raw_tag_map_windows(&code),
            Err(DisasmError::Truncated { .. })
        ));
    }
}
