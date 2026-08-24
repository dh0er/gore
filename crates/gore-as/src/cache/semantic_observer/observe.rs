use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use sha2::{Digest, Sha256};

use super::model::{
    self, Cache, Class, DataType, Enum, Function, FunctionImport, FunctionReference, Global,
    GlobalInitializer, Module, Property,
};
use super::SemanticObserverError;
use crate::cache::disasm::{disassemble, Instr};
use crate::cache::remap::{ref_sites, RefKind};

const OBSERVER_DOMAIN: &[u8] = b"GORE\0AS\0WHOLE-CACHE-SEMANTIC-OBSERVER\0V1\0";
const IDENTITY_DOMAIN: &[u8] = b"GORE\0AS\0CACHE-REFERENCE-IDENTITY\0V1\0";
const TYPE_ID_CORE_MASK: u32 = 0x1fff_ffff;
const TYPE_ID_QUALIFIER_MASK: u32 = 0x6000_0000;
const TYPE_ID_OBJECT_MASK: u32 = 0x1c00_0000;
const LAST_PRIMITIVE_TYPE_ID: i32 = 11;
const MAX_IDENTITY_BYTES: usize = 1024 * 1024;
const MAX_IDENTITY_TOTAL_BYTES: usize = 256 * 1024 * 1024;
const MAX_RESOLVER_ROWS: usize = 8_000_000;
const MAX_TYPE_IDENTITY_DEPTH: usize = 128;
const MAX_INVOKE_DEPTH: usize = 64;
const MAX_INVOKE_NODES: usize = 1_000_000;
const MAX_INVOKE_BYTES: usize = 64 * 1024 * 1024;

/// A backend-neutral invocation value. Floating-point observations are supplied as their exact
/// IEEE bit patterns, avoiding host formatting, NaN, and signed-zero ambiguity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalInvokeValueV1 {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F32Bits(u32),
    F64Bits(u64),
    Utf8(String),
    Bytes(Vec<u8>),
    Sequence(Vec<CanonicalInvokeValueV1>),
    /// Named fields. Field order is not significant; duplicate names are rejected.
    Record(Vec<(String, CanonicalInvokeValueV1)>),
}

/// Optional result of invoking a qualification probe after compiling it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalInvokeReturnV1 {
    type_identity: String,
    value: CanonicalInvokeValueV1,
}

impl CanonicalInvokeReturnV1 {
    pub fn new(type_identity: impl Into<String>, value: CanonicalInvokeValueV1) -> Self {
        Self {
            type_identity: type_identity.into(),
            value,
        }
    }

    pub fn type_identity(&self) -> &str {
        &self.type_identity
    }

    pub fn value(&self) -> &CanonicalInvokeValueV1 {
        &self.value
    }
}

/// Stable digest and bounded coverage counters produced by the observer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedCacheModuleIdentityV1 {
    map_key: String,
    name: String,
    semantic_sha256: [u8; 32],
}

impl ObservedCacheModuleIdentityV1 {
    pub fn map_key(&self) -> &str {
        &self.map_key
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Domain-separated digest of this complete normalized module record.
    pub fn semantic_sha256(&self) -> &[u8; 32] {
        &self.semantic_sha256
    }
}

/// Exact property identity needed by corpus-specific metadata witnesses. The
/// complete property record remains part of the canonical cache/module digest;
/// this projection only makes a closed subset queryable without reparsing raw
/// bytes in qualification policy code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedCachePropertyIdentityV1 {
    module: String,
    class_name: String,
    property_name: String,
    unreal_property: bool,
    transient: Option<bool>,
}

impl ObservedCachePropertyIdentityV1 {
    pub fn module(&self) -> &str {
        &self.module
    }

    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    pub fn property_name(&self) -> &str {
        &self.property_name
    }

    pub fn unreal_property(&self) -> bool {
        self.unreal_property
    }

    pub fn transient(&self) -> Option<bool> {
        self.transient
    }
}

/// Stable digest and bounded coverage counters produced by the observer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WholeCacheSemanticObservationV1 {
    sha256: [u8; 32],
    module_count: u32,
    function_count: u64,
    opcode_counts: [u64; 213],
    class_count: u64,
    behaviour_function_count: u64,
    property_count: u64,
    global_count: u64,
    initializer_function_count: u64,
    string_global_reference_count: u32,
    static_names: Vec<String>,
    module_identities: Vec<ObservedCacheModuleIdentityV1>,
    property_identities: Vec<ObservedCachePropertyIdentityV1>,
    tail_table_counts: [u32; 7],
    invoke_return_included: bool,
}

impl WholeCacheSemanticObservationV1 {
    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }

    pub fn sha256_hex(&self) -> String {
        self.sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn module_count(&self) -> u32 {
        self.module_count
    }

    pub fn function_count(&self) -> u64 {
        self.function_count
    }

    /// Exact instruction counts for serialized opcodes `0..=212`.
    pub fn opcode_counts(&self) -> &[u64; 213] {
        &self.opcode_counts
    }

    pub fn opcode_count(&self, opcode: u8) -> Option<u64> {
        self.opcode_counts.get(opcode as usize).copied()
    }

    /// Resolve through the canonical fork opcode table rather than a caller-maintained number.
    pub fn opcode_count_named(&self, name: &str) -> Option<u64> {
        let opcode = crate::cache::isa::OPCODES
            .iter()
            .find(|opcode| opcode.name == name)?
            .opcode;
        self.opcode_count(opcode)
    }

    pub fn class_count(&self) -> u64 {
        self.class_count
    }

    pub fn behaviour_function_count(&self) -> u64 {
        self.behaviour_function_count
    }

    pub fn property_count(&self) -> u64 {
        self.property_count
    }

    pub fn global_count(&self) -> u64 {
        self.global_count
    }

    pub fn initializer_function_count(&self) -> u64 {
        self.initializer_function_count
    }

    pub fn string_global_reference_count(&self) -> u32 {
        self.string_global_reference_count
    }

    /// Static FName spellings in exact decoded cache order. The complete table
    /// already participates in the canonical digest; this bounded projection
    /// lets qualification policy assert target-specific canonical spellings.
    pub fn static_names(&self) -> &[String] {
        &self.static_names
    }

    /// Ordered canonical module-map identities actually decoded from the cache.
    pub fn module_identities(&self) -> &[ObservedCacheModuleIdentityV1] {
        &self.module_identities
    }

    /// Ordered module/class/property identities decoded from the cache.
    pub fn property_identities(&self) -> &[ObservedCachePropertyIdentityV1] {
        &self.property_identities
    }

    /// Counts in wire order: T1 types, T2 type ids, T3 functions, T4 function ids,
    /// T5 globals, T6 static names, T7 properties.
    pub fn tail_table_counts(&self) -> &[u32; 7] {
        &self.tail_table_counts
    }

    pub fn invoke_return_included(&self) -> bool {
        self.invoke_return_included
    }
}

trait Sink {
    fn put(&mut self, bytes: &[u8]);
}

impl Sink for Vec<u8> {
    fn put(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

impl Sink for Sha256 {
    fn put(&mut self, bytes: &[u8]) {
        Digest::update(self, bytes);
    }
}

struct TeeSink<'a> {
    whole_cache: &'a mut Sha256,
    module: &'a mut Sha256,
}

impl Sink for TeeSink<'_> {
    fn put(&mut self, bytes: &[u8]) {
        self.whole_cache.put(bytes);
        self.module.put(bytes);
    }
}

struct IdentityBuffer {
    bytes: Vec<u8>,
    overflow: bool,
}

impl IdentityBuffer {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            overflow: false,
        }
    }

    fn finish(self, resource: &'static str) -> Result<Vec<u8>, SemanticObserverError> {
        if self.overflow {
            return Err(SemanticObserverError::ResourceLimit {
                resource,
                actual: MAX_IDENTITY_BYTES.saturating_add(1),
                limit: MAX_IDENTITY_BYTES,
            });
        }
        Ok(self.bytes)
    }
}

impl Sink for IdentityBuffer {
    fn put(&mut self, bytes: &[u8]) {
        if self.overflow {
            return;
        }
        let Some(total) = self.bytes.len().checked_add(bytes.len()) else {
            self.overflow = true;
            return;
        };
        if total > MAX_IDENTITY_BYTES || self.bytes.try_reserve(bytes.len()).is_err() {
            self.overflow = true;
            return;
        }
        self.bytes.extend_from_slice(bytes);
    }
}

fn tag(sink: &mut impl Sink, value: &'static [u8]) {
    u64v(sink, value.len() as u64);
    sink.put(value);
}

fn bytes(sink: &mut impl Sink, value: &[u8]) {
    u64v(sink, value.len() as u64);
    sink.put(value);
}

fn string(sink: &mut impl Sink, value: &str) {
    bytes(sink, value.as_bytes());
}

fn boolv(sink: &mut impl Sink, value: bool) {
    sink.put(&[u8::from(value)]);
}

fn i32v(sink: &mut impl Sink, value: i32) {
    sink.put(&value.to_le_bytes());
}

fn u32v(sink: &mut impl Sink, value: u32) {
    sink.put(&value.to_le_bytes());
}

fn i64v(sink: &mut impl Sink, value: i64) {
    sink.put(&value.to_le_bytes());
}

fn u64v(sink: &mut impl Sink, value: u64) {
    sink.put(&value.to_le_bytes());
}

fn count(sink: &mut impl Sink, value: usize) {
    u64v(sink, value as u64);
}

fn hash_i32s(sink: &mut impl Sink, values: &[i32]) {
    count(sink, values.len());
    for &value in values {
        i32v(sink, value);
    }
}

fn hash_strings(sink: &mut impl Sink, values: &[String]) {
    count(sink, values.len());
    for value in values {
        string(sink, value);
    }
}

struct Resolver<'a> {
    cache: &'a Cache,
    type_by_ptr: HashMap<i64, &'a model::TypeReference>,
    type_ptr_by_id: HashMap<i32, i64>,
    function_by_ptr: HashMap<i64, &'a FunctionReference>,
    function_ptr_by_id: HashMap<i32, i64>,
    global_by_ptr: HashMap<i64, &'a model::GlobalReference>,
    property_by_key: HashMap<i64, &'a model::PropertyReference>,
    type_identity: HashMap<i64, Vec<u8>>,
    function_identity: HashMap<i64, Vec<u8>>,
    global_identity: HashMap<i64, Vec<u8>>,
    property_identity: HashMap<i64, Vec<u8>>,
    identity_bytes: usize,
}

impl<'a> Resolver<'a> {
    fn build(cache: &'a Cache) -> Result<Self, SemanticObserverError> {
        let resolver_rows = [
            cache.type_references.len(),
            cache.type_ids.len(),
            cache.function_references.len(),
            cache.function_ids.len(),
            cache.global_references.len(),
            cache.property_references.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
        .ok_or(SemanticObserverError::ResourceLimit {
            resource: "reference resolver rows",
            actual: usize::MAX,
            limit: MAX_RESOLVER_ROWS,
        })?;
        if resolver_rows > MAX_RESOLVER_ROWS {
            return Err(SemanticObserverError::ResourceLimit {
                resource: "reference resolver rows",
                actual: resolver_rows,
                limit: MAX_RESOLVER_ROWS,
            });
        }
        let mut resolver = Self {
            cache,
            type_by_ptr: HashMap::new(),
            type_ptr_by_id: HashMap::new(),
            function_by_ptr: HashMap::new(),
            function_ptr_by_id: HashMap::new(),
            global_by_ptr: HashMap::new(),
            property_by_key: HashMap::new(),
            type_identity: HashMap::new(),
            function_identity: HashMap::new(),
            global_identity: HashMap::new(),
            property_identity: HashMap::new(),
            identity_bytes: 0,
        };
        try_reserve_map(
            &mut resolver.type_by_ptr,
            cache.type_references.len(),
            "type pointer resolver",
        )?;
        try_reserve_map(
            &mut resolver.type_ptr_by_id,
            cache.type_ids.len(),
            "type id resolver",
        )?;
        try_reserve_map(
            &mut resolver.function_by_ptr,
            cache.function_references.len(),
            "function pointer resolver",
        )?;
        try_reserve_map(
            &mut resolver.function_ptr_by_id,
            cache.function_ids.len(),
            "function id resolver",
        )?;
        try_reserve_map(
            &mut resolver.global_by_ptr,
            cache.global_references.len(),
            "global pointer resolver",
        )?;
        try_reserve_map(
            &mut resolver.property_by_key,
            cache.property_references.len(),
            "property resolver",
        )?;
        try_reserve_map(
            &mut resolver.type_identity,
            cache.type_references.len(),
            "type identity resolver",
        )?;
        try_reserve_map(
            &mut resolver.function_identity,
            cache.function_references.len(),
            "function identity resolver",
        )?;
        try_reserve_map(
            &mut resolver.global_identity,
            cache.global_references.len(),
            "global identity resolver",
        )?;
        try_reserve_map(
            &mut resolver.property_identity,
            cache.property_references.len(),
            "property identity resolver",
        )?;
        let mut pointer_keys = HashSet::new();
        pointer_keys
            .try_reserve(
                cache.type_references.len()
                    + cache.function_references.len()
                    + cache.global_references.len(),
            )
            .map_err(|_| SemanticObserverError::AllocationFailed {
                resource: "pointer-key uniqueness set",
            })?;
        for row in &cache.type_references {
            if row.raw_key == 0 || !pointer_keys.insert(row.raw_key) {
                return Err(SemanticObserverError::DuplicateKey {
                    kind: "type/pointer",
                    key: format!("{:#x}", row.raw_key),
                });
            }
            resolver.type_by_ptr.insert(row.raw_key, row);
        }
        for &(id, ptr) in &cache.type_ids {
            if id <= LAST_PRIMITIVE_TYPE_ID || resolver.type_ptr_by_id.insert(id, ptr).is_some() {
                return Err(SemanticObserverError::DuplicateKey {
                    kind: "type id",
                    key: format!("{id:#x}"),
                });
            }
            if !resolver.type_by_ptr.contains_key(&ptr) {
                return Err(unresolved("TypeIdReferenceToPointer", "type pointer", ptr));
            }
        }
        for row in &cache.function_references {
            if row.raw_key == 0 || !pointer_keys.insert(row.raw_key) {
                return Err(SemanticObserverError::DuplicateKey {
                    kind: "function/pointer",
                    key: format!("{:#x}", row.raw_key),
                });
            }
            resolver.function_by_ptr.insert(row.raw_key, row);
        }
        for &(id, ptr) in &cache.function_ids {
            if id == 0 || resolver.function_ptr_by_id.insert(id, ptr).is_some() {
                return Err(SemanticObserverError::DuplicateKey {
                    kind: "function id",
                    key: format!("{id:#x}"),
                });
            }
            if !resolver.function_by_ptr.contains_key(&ptr) {
                return Err(unresolved(
                    "FunctionIdReferenceToPointer",
                    "function pointer",
                    ptr,
                ));
            }
        }
        for row in &cache.global_references {
            if row.raw_key == 0 || !pointer_keys.insert(row.raw_key) {
                return Err(SemanticObserverError::DuplicateKey {
                    kind: "global/pointer",
                    key: format!("{:#x}", row.raw_key),
                });
            }
            resolver.global_by_ptr.insert(row.raw_key, row);
        }
        for row in &cache.property_references {
            if row.raw_key == 0 || resolver.property_by_key.insert(row.raw_key, row).is_some() {
                return Err(SemanticObserverError::DuplicateKey {
                    kind: "property",
                    key: format!("{:#x}", row.raw_key),
                });
            }
        }

        for row in &cache.type_references {
            resolver.build_type_identity(row.raw_key, &mut Vec::new())?;
        }
        for source_row in &cache.function_references {
            let key = source_row.raw_key;
            let row = resolver.function_by_ptr[&key];
            if row.is_method && row.object_type == 0 {
                return Err(SemanticObserverError::UnresolvedReference {
                    context: "FunctionReference.ObjectType".to_owned(),
                    kind: "method owner type pointer",
                    value: 0,
                });
            }
            let mut identity = IdentityBuffer::new();
            identity.put(IDENTITY_DOMAIN);
            tag(&mut identity, b"function-reference");
            string(&mut identity, &row.name);
            string(&mut identity, &row.module);
            string(&mut identity, &row.namespace);
            boolv(&mut identity, row.is_const);
            boolv(&mut identity, row.is_imported_decl);
            boolv(&mut identity, row.is_method);
            resolver.hash_type_ptr(
                &mut identity,
                row.object_type,
                true,
                "FunctionReference.ObjectType",
            )?;
            count(&mut identity, row.parameter_types.len());
            for data_type in &row.parameter_types {
                resolver.hash_data_type(
                    &mut identity,
                    data_type,
                    "FunctionReference.ParameterTypes",
                )?;
            }
            resolver.hash_data_type(
                &mut identity,
                &row.return_type,
                "FunctionReference.ReturnType",
            )?;
            let identity = identity.finish("function reference identity")?;
            resolver.store_identity("function", key, identity, IdentityClass::Function)?;
        }
        for source_row in &cache.global_references {
            let key = source_row.raw_key;
            let row = resolver.global_by_ptr[&key];
            let mut identity = IdentityBuffer::new();
            identity.put(IDENTITY_DOMAIN);
            tag(&mut identity, b"global-reference");
            string(&mut identity, &row.name);
            string(&mut identity, &row.module);
            string(&mut identity, &row.namespace);
            boolv(&mut identity, row.is_string);
            let identity = identity.finish("global reference identity")?;
            resolver.store_identity("global", key, identity, IdentityClass::Global)?;
        }
        for source_row in &cache.property_references {
            let key = source_row.raw_key;
            let row = resolver.property_by_key[&key];
            if key as u64 & 1 == 0 {
                return Err(SemanticObserverError::InvalidStructure {
                    context: "PropertyReferences",
                    detail: "property key is missing its canonical discriminator bit",
                });
            }
            let key_owner_type_id = ((key as u64 >> 1) & u32::MAX as u64) as u32 as i32;
            let mut identity = IdentityBuffer::new();
            identity.put(IDENTITY_DOMAIN);
            tag(&mut identity, b"property-reference");
            // The raw key also carries a build-specific member offset. The name is its portable
            // member identity; the key's owner id and the row's saved owner/type id are both
            // resolved so a malformed cross-owner row cannot collapse onto a valid one.
            resolver.hash_type_id(
                &mut identity,
                key_owner_type_id,
                "PropertyReferences.Key.OwnerTypeId",
            )?;
            string(&mut identity, &row.name);
            resolver.hash_type_id(
                &mut identity,
                row.old_type_id,
                "PropertyReference.OldTypeId",
            )?;
            let identity = identity.finish("property reference identity")?;
            resolver.store_identity("property", key, identity, IdentityClass::Property)?;
        }
        Ok(resolver)
    }

    fn store_identity(
        &mut self,
        kind: &'static str,
        key: i64,
        identity: Vec<u8>,
        class: IdentityClass,
    ) -> Result<(), SemanticObserverError> {
        if identity.len() > MAX_IDENTITY_BYTES {
            return Err(SemanticObserverError::ResourceLimit {
                resource: "one semantic reference identity",
                actual: identity.len(),
                limit: MAX_IDENTITY_BYTES,
            });
        }
        self.identity_bytes = self.identity_bytes.checked_add(identity.len()).ok_or(
            SemanticObserverError::ResourceLimit {
                resource: "semantic reference identities",
                actual: usize::MAX,
                limit: MAX_IDENTITY_TOTAL_BYTES,
            },
        )?;
        if self.identity_bytes > MAX_IDENTITY_TOTAL_BYTES {
            return Err(SemanticObserverError::ResourceLimit {
                resource: "semantic reference identities",
                actual: self.identity_bytes,
                limit: MAX_IDENTITY_TOTAL_BYTES,
            });
        }
        let replaced = match class {
            IdentityClass::Type => self.type_identity.insert(key, identity),
            IdentityClass::Function => self.function_identity.insert(key, identity),
            IdentityClass::Global => self.global_identity.insert(key, identity),
            IdentityClass::Property => self.property_identity.insert(key, identity),
        };
        if replaced.is_some() {
            return Err(SemanticObserverError::AmbiguousIdentity { kind });
        }
        Ok(())
    }

    fn build_type_identity(
        &mut self,
        key: i64,
        stack: &mut Vec<i64>,
    ) -> Result<Vec<u8>, SemanticObserverError> {
        if let Some(identity) = self.type_identity.get(&key) {
            return Ok(identity.clone());
        }
        if stack.contains(&key) {
            return Err(SemanticObserverError::InvalidStructure {
                context: "TypeReferences",
                detail: "cyclic subtype reference",
            });
        }
        if stack.len() >= MAX_TYPE_IDENTITY_DEPTH {
            return Err(SemanticObserverError::ResourceLimit {
                resource: "type identity nesting",
                actual: stack.len() + 1,
                limit: MAX_TYPE_IDENTITY_DEPTH,
            });
        }
        let row = *self
            .type_by_ptr
            .get(&key)
            .ok_or_else(|| unresolved("TypeReferences", "type pointer", key))?;
        stack.push(key);
        let mut identity = IdentityBuffer::new();
        identity.put(IDENTITY_DOMAIN);
        tag(&mut identity, b"type-reference");
        string(&mut identity, &row.name);
        string(&mut identity, &row.module);
        string(&mut identity, &row.namespace);
        count(&mut identity, row.sub_types.len());
        for subtype in &row.sub_types {
            for flag in subtype.flags {
                boolv(&mut identity, flag);
            }
            i32v(&mut identity, subtype.token);
            if subtype.type_info == 0 {
                tag(&mut identity, b"null-type-info");
            } else {
                let nested = self.build_type_identity(subtype.type_info, stack)?;
                bytes(&mut identity, &nested);
            }
        }
        stack.pop();
        let identity = identity.finish("type reference identity")?;
        self.store_identity("type", key, identity.clone(), IdentityClass::Type)?;
        Ok(identity)
    }

    fn hash_data_type(
        &self,
        sink: &mut impl Sink,
        value: &DataType,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        tag(sink, b"data-type");
        for flag in value.flags {
            boolv(sink, flag);
        }
        i32v(sink, value.token);
        self.hash_type_ptr(sink, value.type_info, true, context)
    }

    fn hash_type_ptr(
        &self,
        sink: &mut impl Sink,
        ptr: i64,
        nullable: bool,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        if ptr == 0 && nullable {
            tag(sink, b"null-type-pointer");
            return Ok(());
        }
        let identity = self
            .type_identity
            .get(&ptr)
            .ok_or_else(|| unresolved(context, "type pointer", ptr))?;
        tag(sink, b"type-pointer-identity");
        bytes(sink, identity);
        Ok(())
    }

    fn hash_function_ptr(
        &self,
        sink: &mut impl Sink,
        ptr: i64,
        nullable: bool,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        if ptr == 0 && nullable {
            tag(sink, b"null-function-pointer");
            return Ok(());
        }
        let identity = self
            .function_identity
            .get(&ptr)
            .ok_or_else(|| unresolved(context, "function pointer", ptr))?;
        tag(sink, b"function-pointer-identity");
        bytes(sink, identity);
        Ok(())
    }

    fn hash_global_ptr(
        &self,
        sink: &mut impl Sink,
        ptr: i64,
        nullable: bool,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        if ptr == 0 && nullable {
            tag(sink, b"null-global-pointer");
            return Ok(());
        }
        let identity = self
            .global_identity
            .get(&ptr)
            .ok_or_else(|| unresolved(context, "global pointer", ptr))?;
        tag(sink, b"global-pointer-identity");
        bytes(sink, identity);
        Ok(())
    }

    fn hash_type_id(
        &self,
        sink: &mut impl Sink,
        raw: i32,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        if raw == -1 {
            tag(sink, b"no-type-id");
            return Ok(());
        }
        let raw_bits = raw as u32;
        let core = (raw_bits & TYPE_ID_CORE_MASK) as i32;
        let qualifiers = raw_bits & !TYPE_ID_CORE_MASK;
        if (0..=LAST_PRIMITIVE_TYPE_ID).contains(&raw) {
            tag(sink, b"primitive-type-id");
            i32v(sink, raw);
            return Ok(());
        }
        if qualifiers & !TYPE_ID_QUALIFIER_MASK != 0 || core <= LAST_PRIMITIVE_TYPE_ID {
            return Err(unresolved(context, "runtime type id", raw as i64));
        }
        let ptr = self
            .type_ptr_by_id
            .get(&core)
            .ok_or_else(|| unresolved(context, "runtime type id", raw as i64))?;
        let identity = self
            .type_identity
            .get(ptr)
            .ok_or_else(|| unresolved(context, "type pointer", *ptr))?;
        tag(sink, b"type-id-identity");
        u32v(sink, raw_bits & TYPE_ID_OBJECT_MASK);
        u32v(sink, qualifiers);
        bytes(sink, identity);
        Ok(())
    }

    fn hash_function_id(
        &self,
        sink: &mut impl Sink,
        raw: i64,
        nullable: bool,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        if raw == 0 && nullable {
            tag(sink, b"null-function-id");
            return Ok(());
        }
        let id = i32::try_from(raw).map_err(|_| unresolved(context, "function id", raw))?;
        let ptr = self
            .function_ptr_by_id
            .get(&id)
            .ok_or_else(|| unresolved(context, "function id", raw))?;
        self.hash_function_ptr(sink, *ptr, false, context)
    }

    fn hash_property_key(
        &self,
        sink: &mut impl Sink,
        key: i64,
        context: &str,
    ) -> Result<(), SemanticObserverError> {
        let identity = self
            .property_identity
            .get(&key)
            .ok_or_else(|| unresolved(context, "property key", key))?;
        tag(sink, b"property-reference-identity");
        bytes(sink, identity);
        Ok(())
    }

    fn function_name_by_id(&self, id: i32) -> Option<&str> {
        self.function_ptr_by_id
            .get(&id)
            .and_then(|ptr| self.function_by_ptr.get(ptr))
            .map(|row| row.name.as_str())
    }

    fn function_name_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.function_by_ptr.get(&ptr).map(|row| row.name.as_str())
    }
}

enum IdentityClass {
    Type,
    Function,
    Global,
    Property,
}

fn try_reserve_map<K: Eq + Hash, V>(
    map: &mut HashMap<K, V>,
    additional: usize,
    resource: &'static str,
) -> Result<(), SemanticObserverError> {
    map.try_reserve(additional)
        .map_err(|_| SemanticObserverError::AllocationFailed { resource })
}

fn unresolved(context: &str, kind: &'static str, value: i64) -> SemanticObserverError {
    SemanticObserverError::UnresolvedReference {
        context: context.to_owned(),
        kind,
        value,
    }
}

struct Stats {
    functions: u64,
    opcode_counts: [u64; 213],
    classes: u64,
    behaviour_functions: u64,
    properties: u64,
    globals: u64,
    initializer_functions: u64,
    function_ids: HashSet<u32>,
    declaration_identities: HashSet<Vec<u8>>,
    declaration_identity_bytes: usize,
}

impl Stats {
    fn new(function_count: usize) -> Result<Self, SemanticObserverError> {
        if function_count > MAX_RESOLVER_ROWS {
            return Err(SemanticObserverError::ResourceLimit {
                resource: "function records",
                actual: function_count,
                limit: MAX_RESOLVER_ROWS,
            });
        }
        let mut value = Self {
            functions: 0,
            opcode_counts: [0; 213],
            classes: 0,
            behaviour_functions: 0,
            properties: 0,
            globals: 0,
            initializer_functions: 0,
            function_ids: HashSet::new(),
            declaration_identities: HashSet::new(),
            declaration_identity_bytes: 0,
        };
        value
            .function_ids
            .try_reserve(function_count)
            .map_err(|_| SemanticObserverError::AllocationFailed {
                resource: "function id uniqueness set",
            })?;
        value
            .declaration_identities
            .try_reserve(function_count)
            .map_err(|_| SemanticObserverError::AllocationFailed {
                resource: "declared function identity set",
            })?;
        Ok(value)
    }
}

/// Decode and canonically observe one complete cache plus an optional invocation result.
///
/// The 16-byte `DataGuid` is deliberately represented by a domain marker instead of its random
/// value: it is a generation nonce, not runtime program semantics. `BuildIdentifier` remains in
/// the digest. Module/list order remains significant. TMap tail rows are sorted by complete
/// semantic identity after their raw pointer/id keys have been resolved.
pub fn observe_whole_cache_semantics_v1(
    cache_bytes: &[u8],
    invoke_return: Option<&CanonicalInvokeReturnV1>,
) -> Result<WholeCacheSemanticObservationV1, SemanticObserverError> {
    let cache = model::decode(cache_bytes)?;
    let resolver = Resolver::build(&cache)?;
    validate_module_keys(&cache)?;
    let mut digest = Sha256::new();
    digest.put(OBSERVER_DOMAIN);
    tag(&mut digest, b"normalized-random-data-guid");
    i32v(&mut digest, cache.build_identifier);
    count(&mut digest, cache.modules.len());
    let mut stats = Stats::new(total_function_records(&cache)?)?;
    let mut module_identities = Vec::new();
    module_identities
        .try_reserve(cache.modules.len())
        .map_err(|_| SemanticObserverError::AllocationFailed {
            resource: "module semantic identities",
        })?;
    for module in &cache.modules {
        let mut module_digest = Sha256::new();
        module_digest.put(b"gore.as.cache.semantic-observer.module.v1\0");
        hash_module(
            &mut TeeSink {
                whole_cache: &mut digest,
                module: &mut module_digest,
            },
            module,
            &resolver,
            &mut stats,
        )?;
        module_identities.push(ObservedCacheModuleIdentityV1 {
            map_key: module.map_key.clone(),
            name: module.name.clone(),
            semantic_sha256: module_digest.finalize().into(),
        });
    }
    let mut property_identities = Vec::new();
    property_identities
        .try_reserve(stats.properties as usize)
        .map_err(|_| SemanticObserverError::AllocationFailed {
            resource: "property semantic identities",
        })?;
    for module in &cache.modules {
        for class in &module.classes {
            for property in &class.properties {
                property_identities.push(ObservedCachePropertyIdentityV1 {
                    module: module.name.clone(),
                    class_name: class.name.clone(),
                    property_name: property.name.clone(),
                    unreal_property: property.unreal.is_some(),
                    transient: property
                        .unreal
                        .as_ref()
                        .map(|unreal| unreal.flags_before_replication[8]),
                });
            }
        }
    }
    hash_tail_tables(&mut digest, &cache, &resolver)?;
    match invoke_return {
        Some(value) => {
            tag(&mut digest, b"invoke-return-present");
            let mut budget = InvokeBudget::default();
            charge_invoke_bytes(&mut budget, value.type_identity.len())?;
            string(&mut digest, &value.type_identity);
            hash_invoke_value(&mut digest, &value.value, 0, &mut budget)?;
        }
        None => tag(&mut digest, b"invoke-return-absent"),
    }
    let sha256: [u8; 32] = digest.finalize().into();
    Ok(WholeCacheSemanticObservationV1 {
        sha256,
        module_count: cache.modules.len() as u32,
        function_count: stats.functions,
        opcode_counts: stats.opcode_counts,
        class_count: stats.classes,
        behaviour_function_count: stats.behaviour_functions,
        property_count: stats.properties,
        global_count: stats.globals,
        initializer_function_count: stats.initializer_functions,
        string_global_reference_count: cache
            .global_references
            .iter()
            .filter(|row| row.is_string)
            .count() as u32,
        static_names: cache.static_names.clone(),
        module_identities,
        property_identities,
        tail_table_counts: [
            cache.type_references.len() as u32,
            cache.type_ids.len() as u32,
            cache.function_references.len() as u32,
            cache.function_ids.len() as u32,
            cache.global_references.len() as u32,
            cache.static_names.len() as u32,
            cache.property_references.len() as u32,
        ],
        invoke_return_included: invoke_return.is_some(),
    })
}

fn total_function_records(cache: &Cache) -> Result<usize, SemanticObserverError> {
    let mut total = 0usize;
    let mut add = |value: usize| -> Result<(), SemanticObserverError> {
        total = total
            .checked_add(value)
            .ok_or(SemanticObserverError::ResourceLimit {
                resource: "function records",
                actual: usize::MAX,
                limit: MAX_RESOLVER_ROWS,
            })?;
        Ok(())
    };
    for module in &cache.modules {
        add(module.functions.len())?;
        for class in &module.classes {
            add(class.methods.len())?;
            add(class.constructors.len())?;
            add(class.behaviour_functions.len())?;
        }
        for global in &module.globals {
            if matches!(global.initializer, GlobalInitializer::Function { .. }) {
                add(1)?;
            }
        }
    }
    Ok(total)
}

fn validate_module_keys(cache: &Cache) -> Result<(), SemanticObserverError> {
    let mut keys = HashSet::new();
    let mut names = HashSet::new();
    for module in &cache.modules {
        if !keys.insert(module.map_key.as_str()) {
            return Err(SemanticObserverError::DuplicateKey {
                kind: "module map",
                key: module.map_key.clone(),
            });
        }
        if !names.insert(module.name.as_str()) {
            return Err(SemanticObserverError::DuplicateKey {
                kind: "module name",
                key: module.name.clone(),
            });
        }
    }
    Ok(())
}

fn hash_module(
    sink: &mut impl Sink,
    module: &Module,
    resolver: &Resolver<'_>,
    stats: &mut Stats,
) -> Result<(), SemanticObserverError> {
    tag(sink, b"module");
    string(sink, &module.map_key);
    string(sink, &module.name);
    count(sink, module.functions.len());
    for (index, function) in module.functions.iter().enumerate() {
        hash_function(
            sink,
            function,
            resolver,
            stats,
            &format!("{}::function[{index}]", module.name),
            &module.name,
        )?;
    }
    count(sink, module.classes.len());
    for (index, class) in module.classes.iter().enumerate() {
        stats.classes += 1;
        hash_class(sink, class, resolver, stats, &module.name, index)?;
    }
    count(sink, module.enums.len());
    for value in &module.enums {
        hash_enum(sink, value);
    }
    count(sink, module.globals.len());
    for (index, global) in module.globals.iter().enumerate() {
        stats.globals += 1;
        hash_global(sink, global, resolver, stats, &module.name, index)?;
    }
    count(sink, module.function_imports.len());
    for import in &module.function_imports {
        hash_import(sink, import, resolver)?;
    }
    i64v(sink, module.code_hash);
    hash_strings(sink, &module.imported_modules);
    string(sink, &module.statics_class_name);
    hash_strings(sink, &module.declared_events);
    hash_strings(sink, &module.declared_delegates);
    string(sink, &module.script_relative_filename);
    hash_strings(sink, &module.post_init_functions);
    Ok(())
}

fn hash_function(
    sink: &mut impl Sink,
    function: &Function,
    resolver: &Resolver<'_>,
    stats: &mut Stats,
    context: &str,
    _owner_identity: &str,
) -> Result<(), SemanticObserverError> {
    stats.functions += 1;
    if function.id == 0 || !stats.function_ids.insert(function.id) {
        return Err(SemanticObserverError::DuplicateKey {
            kind: "declared function id",
            key: format!("{:#x}", function.id),
        });
    }
    // The owning semantic path (including function role/list position) is required: the fork can
    // legitimately serialize signature-identical wrappers in different behaviour/initializer
    // slots. Treating the bare signature as an identity would make those valid declarations
    // ambiguous.
    let declaration = function_declaration_identity(function, resolver, context)?;
    stats.declaration_identity_bytes = stats
        .declaration_identity_bytes
        .checked_add(declaration.len())
        .ok_or(SemanticObserverError::ResourceLimit {
            resource: "declared function identities",
            actual: usize::MAX,
            limit: MAX_IDENTITY_TOTAL_BYTES,
        })?;
    if stats.declaration_identity_bytes > MAX_IDENTITY_TOTAL_BYTES {
        return Err(SemanticObserverError::ResourceLimit {
            resource: "declared function identities",
            actual: stats.declaration_identity_bytes,
            limit: MAX_IDENTITY_TOTAL_BYTES,
        });
    }
    if !stats.declaration_identities.insert(declaration.clone()) {
        return Err(SemanticObserverError::AmbiguousIdentity {
            kind: "declared function",
        });
    }
    tag(sink, b"function");
    bytes(sink, &declaration); // Canonical representation of Function.Id.
    string(sink, &function.name);
    string(sink, &function.namespace);
    resolver.hash_data_type(sink, &function.return_type, context)?;
    count(sink, function.parameter_types.len());
    for data_type in &function.parameter_types {
        resolver.hash_data_type(sink, data_type, context)?;
    }
    hash_strings(sink, &function.parameter_names);
    hash_i32s(sink, &function.parameter_flags);
    hash_strings(sink, &function.parameter_default_args);
    i32v(sink, function.traits);
    hash_bytecode(sink, &function.bytecode, resolver, stats, context)?;
    if !function.bytecode_references.is_empty() {
        return Err(SemanticObserverError::UnsupportedByteCodeReferences {
            context: context.to_owned(),
        });
    }
    count(sink, 0); // ByteCodeReferences is covered under its explicit empty-only contract.
    i32v(sink, function.variable_space);
    count(sink, function.object_variable_types.len());
    for &ptr in &function.object_variable_types {
        resolver.hash_type_ptr(sink, ptr, false, context)?;
    }
    hash_i32s(sink, &function.object_variable_positions);
    i32v(sink, function.object_variables_on_heap);
    hash_i32s(sink, &function.var_info_program_positions);
    hash_i32s(sink, &function.var_info_offsets);
    hash_i32s(sink, &function.var_info_options);
    i32v(sink, function.stack_needed);
    i32v(sink, function.declared_at);
    hash_i32s(sink, &function.line_numbers);
    match &function.unreal {
        Some(unreal) => {
            boolv(sink, true);
            string(sink, &unreal.unreal_name);
            hash_strings(sink, &unreal.metadata_specifiers);
            hash_strings(sink, &unreal.metadata_values);
            for flag in unreal.flags {
                boolv(sink, flag);
            }
        }
        None => boolv(sink, false),
    }
    Ok(())
}

fn function_declaration_identity(
    function: &Function,
    resolver: &Resolver<'_>,
    owner_identity: &str,
) -> Result<Vec<u8>, SemanticObserverError> {
    let mut identity = IdentityBuffer::new();
    identity.put(IDENTITY_DOMAIN);
    tag(&mut identity, b"declared-function");
    string(&mut identity, owner_identity);
    string(&mut identity, &function.namespace);
    string(&mut identity, &function.name);
    count(&mut identity, function.parameter_types.len());
    for data_type in &function.parameter_types {
        resolver.hash_data_type(&mut identity, data_type, "declared function identity")?;
    }
    resolver.hash_data_type(
        &mut identity,
        &function.return_type,
        "declared function identity",
    )?;
    i32v(&mut identity, function.traits);
    identity.finish("declared function identity")
}

fn hash_property(
    sink: &mut impl Sink,
    property: &Property,
    resolver: &Resolver<'_>,
    context: &str,
) -> Result<(), SemanticObserverError> {
    tag(sink, b"property");
    string(sink, &property.name);
    resolver.hash_data_type(sink, &property.data_type, context)?;
    boolv(sink, property.is_private);
    boolv(sink, property.is_protected);
    match &property.unreal {
        Some(unreal) => {
            boolv(sink, true);
            hash_strings(sink, &unreal.metadata_specifiers);
            hash_strings(sink, &unreal.metadata_values);
            for flag in unreal.flags_before_replication {
                boolv(sink, flag);
            }
            boolv(sink, unreal.replicated);
            boolv(sink, unreal.skip_replication);
            boolv(sink, unreal.skip_serialization);
            boolv(sink, unreal.save_game);
            match unreal.replication {
                Some((condition, notify)) => {
                    boolv(sink, true);
                    i32v(sink, condition);
                    boolv(sink, notify);
                }
                None => boolv(sink, false),
            }
            boolv(sink, unreal.config);
            boolv(sink, unreal.interp);
            boolv(sink, unreal.asset_registry_searchable);
        }
        None => boolv(sink, false),
    }
    Ok(())
}

fn hash_class(
    sink: &mut impl Sink,
    class: &Class,
    resolver: &Resolver<'_>,
    stats: &mut Stats,
    module_name: &str,
    class_index: usize,
) -> Result<(), SemanticObserverError> {
    let context = format!("{module_name}::class[{class_index}]::{}", class.name);
    let owner = format!("{module_name}::{}::{}", class.namespace, class.name);
    tag(sink, b"class");
    string(sink, &class.name);
    string(sink, &class.namespace);
    i32v(sink, class.flags);
    count(sink, class.properties.len());
    for property in &class.properties {
        stats.properties += 1;
        hash_property(sink, property, resolver, &context)?;
    }
    count(sink, class.methods.len());
    for (index, function) in class.methods.iter().enumerate() {
        hash_function(
            sink,
            function,
            resolver,
            stats,
            &format!("{context}::method[{index}]"),
            &owner,
        )?;
    }
    hash_i32s(sink, &class.method_table);
    for &index in &class.method_table {
        if index < -1 || index as usize >= class.methods.len() && index != -1 {
            return Err(SemanticObserverError::InvalidStructure {
                context: "Class.MethodTable",
                detail: "method index is outside Class.Methods",
            });
        }
    }
    resolver.hash_type_ptr(sink, class.derived_from, true, &context)?;
    resolver.hash_type_ptr(sink, class.shadow_type, true, &context)?;
    count(sink, class.constructors.len());
    for (index, function) in class.constructors.iter().enumerate() {
        hash_function(
            sink,
            function,
            resolver,
            stats,
            &format!("{context}::constructor[{index}]"),
            &owner,
        )?;
    }
    count(sink, class.factory_references.len());
    for &id in &class.factory_references {
        resolver.hash_function_id(sink, id, true, &context)?;
    }
    count(sink, class.behaviour_references.len());
    for &id in &class.behaviour_references {
        resolver.hash_function_id(sink, id, true, &context)?;
    }
    count(sink, class.behaviour_functions.len());
    stats.behaviour_functions += class.behaviour_functions.len() as u64;
    for (index, function) in class.behaviour_functions.iter().enumerate() {
        hash_function(
            sink,
            function,
            resolver,
            stats,
            &format!("{context}::behaviour[{index}]"),
            &owner,
        )?;
    }
    hash_i32s(sink, &class.behaviour_function_types);
    match &class.preprocessor {
        Some(value) => {
            boolv(sink, true);
            string(sink, &value.super_class);
            string(sink, &value.code_super_class);
            for flag in value.flags {
                boolv(sink, flag);
            }
            string(sink, &value.config_name);
            string(sink, &value.static_class_global_variable_name);
            boolv(sink, value.placeable);
            hash_strings(sink, &value.metadata_specifiers);
            hash_strings(sink, &value.metadata_values);
            string(sink, &value.compose_onto_class_name);
        }
        None => boolv(sink, false),
    }
    Ok(())
}

fn hash_enum(sink: &mut impl Sink, value: &Enum) {
    tag(sink, b"enum");
    string(sink, &value.name);
    string(sink, &value.namespace);
    hash_strings(sink, &value.names);
    hash_i32s(sink, &value.values);
}

fn hash_global(
    sink: &mut impl Sink,
    global: &Global,
    resolver: &Resolver<'_>,
    stats: &mut Stats,
    module_name: &str,
    index: usize,
) -> Result<(), SemanticObserverError> {
    let context = format!("{module_name}::global[{index}]::{}", global.name);
    tag(sink, b"global");
    string(sink, &global.name);
    string(sink, &global.namespace);
    resolver.hash_data_type(sink, &global.data_type, &context)?;
    match &global.initializer {
        GlobalInitializer::Default => tag(sink, b"default-initializer"),
        GlobalInitializer::PureConstant(value) => {
            tag(sink, b"pure-constant");
            u64v(sink, *value);
        }
        GlobalInitializer::Function { present, function } => {
            stats.initializer_functions += 1;
            tag(sink, b"function-initializer");
            boolv(sink, *present);
            hash_function(sink, function, resolver, stats, &context, module_name)?;
        }
    }
    Ok(())
}

fn hash_import(
    sink: &mut impl Sink,
    import: &FunctionImport,
    resolver: &Resolver<'_>,
) -> Result<(), SemanticObserverError> {
    tag(sink, b"function-import");
    string(sink, &import.imported_from_module);
    string(sink, &import.signature.name);
    string(sink, &import.signature.namespace);
    count(sink, import.signature.parameter_types.len());
    for value in &import.signature.parameter_types {
        resolver.hash_data_type(sink, value, "FunctionImport.ParameterTypes")?;
    }
    hash_i32s(sink, &import.signature.parameter_flags);
    hash_strings(sink, &import.signature.parameter_default_args);
    resolver.hash_data_type(
        sink,
        &import.signature.return_type,
        "FunctionImport.ReturnType",
    )?;
    Ok(())
}

fn hash_tail_tables(
    sink: &mut impl Sink,
    cache: &Cache,
    resolver: &Resolver<'_>,
) -> Result<(), SemanticObserverError> {
    tag(sink, b"seven-tail-tables");
    let mut identities: Vec<&Vec<u8>> = resolver.type_identity.values().collect();
    identities.sort();
    tag(sink, b"T1-TypeReferences");
    count(sink, identities.len());
    for identity in identities {
        bytes(sink, identity);
    }

    let mut rows = Vec::with_capacity(cache.type_ids.len());
    for &(id, _) in &cache.type_ids {
        let mut row = Vec::new();
        resolver.hash_type_id(&mut row, id, "TypeIdReferenceToPointer")?;
        rows.push(row);
    }
    rows.sort();
    hash_sorted_rows(sink, b"T2-TypeIdReferenceToPointer", &rows);

    let mut identities: Vec<&Vec<u8>> = resolver.function_identity.values().collect();
    identities.sort();
    tag(sink, b"T3-FunctionReferences");
    count(sink, identities.len());
    for identity in identities {
        bytes(sink, identity);
    }

    let mut rows = Vec::with_capacity(cache.function_ids.len());
    for &(id, _) in &cache.function_ids {
        let mut row = Vec::new();
        resolver.hash_function_id(&mut row, id as i64, false, "FunctionIdReferenceToPointer")?;
        rows.push(row);
    }
    rows.sort();
    hash_sorted_rows(sink, b"T4-FunctionIdReferenceToPointer", &rows);

    let mut identities: Vec<&Vec<u8>> = resolver.global_identity.values().collect();
    identities.sort();
    tag(sink, b"T5-GlobalReferences");
    count(sink, identities.len());
    for identity in identities {
        bytes(sink, identity);
    }

    let mut names: Vec<&String> = cache.static_names.iter().collect();
    names.sort();
    tag(sink, b"T6-StaticNames");
    count(sink, names.len());
    for name in names {
        string(sink, name);
    }

    let mut identities: Vec<&Vec<u8>> = resolver.property_identity.values().collect();
    identities.sort();
    tag(sink, b"T7-PropertyReferences");
    count(sink, identities.len());
    for identity in identities {
        bytes(sink, identity);
    }
    Ok(())
}

fn hash_sorted_rows(sink: &mut impl Sink, table: &'static [u8], rows: &[Vec<u8>]) {
    tag(sink, table);
    count(sink, rows.len());
    for row in rows {
        bytes(sink, row);
    }
}

fn read_qword(code: &[i32], offset: usize) -> i64 {
    let low = code[offset] as u32 as u64;
    let high = code[offset + 1] as u32 as u64;
    (low | high << 32) as i64
}

fn next_callee_name<'a>(
    instructions: &[Instr],
    index: usize,
    code: &[i32],
    resolver: &'a Resolver<'_>,
) -> Option<&'a str> {
    let next = instructions.get(index + 1)?;
    match next.op.name {
        "CALLSYS" | "FuncPtr" | "Thiscall1" => {
            resolver.function_name_by_ptr(read_qword(code, next.offset_dw + 1))
        }
        "CALL" | "CALLBND" | "CALLINTF" => resolver.function_name_by_id(code[next.offset_dw + 1]),
        _ => None,
    }
}

fn hash_bytecode(
    sink: &mut impl Sink,
    code: &[i32],
    resolver: &Resolver<'_>,
    stats: &mut Stats,
    context: &str,
) -> Result<(), SemanticObserverError> {
    let instructions =
        disassemble(code).map_err(|error| SemanticObserverError::InvalidBytecode {
            context: context.to_owned(),
            detail: error.to_string(),
        })?;
    tag(sink, b"aligned-bytecode");
    count(sink, instructions.len());
    for (instruction_index, instruction) in instructions.iter().enumerate() {
        stats.opcode_counts[instruction.op.opcode as usize] += 1;
        let start = instruction.offset_dw;
        let size = instruction.op.size_dwords as usize;
        let mut raw: Vec<i32> = code[start..start + size].to_vec();
        let mut references = Vec::<Vec<u8>>::new();
        let member = matches!(
            instruction.op.name,
            "ADDSi" | "LoadThisR" | "LoadRObjR" | "LoadVObjR"
        );
        for site in ref_sites(instruction.op.name) {
            let absolute = start + site.dw_index;
            let mut encoded = Vec::new();
            match site.kind {
                RefKind::GlobalPtr => resolver.hash_global_ptr(
                    &mut encoded,
                    read_qword(code, absolute),
                    true,
                    context,
                )?,
                RefKind::FuncPtr => resolver.hash_function_ptr(
                    &mut encoded,
                    read_qword(code, absolute),
                    true,
                    context,
                )?,
                RefKind::TypePtr => resolver.hash_type_ptr(
                    &mut encoded,
                    read_qword(code, absolute),
                    true,
                    context,
                )?,
                RefKind::FuncId => {
                    resolver.hash_function_id(&mut encoded, code[absolute] as i64, true, context)?
                }
                RefKind::TypeId => resolver.hash_type_id(&mut encoded, code[absolute], context)?,
            }
            let local = site.dw_index;
            raw[local] = 0;
            if site.is_qword {
                raw[local + 1] = 0;
            }
            references.push(encoded);
        }
        if member {
            let (raw_type_id, offset) = match instruction.op.name {
                "ADDSi" | "LoadThisR" => {
                    raw[0] = (raw[0] as u32 & 0x0000_ffff) as i32;
                    (code[start + 1], (code[start] >> 16) as i16 as i32)
                }
                _ => {
                    raw[1] = (raw[1] as u32 & 0xffff_0000) as i32;
                    (code[start + 2], code[start + 1] as i16 as i32)
                }
            };
            let key_bits = ((raw_type_id as u32 as u64) << 1) | ((offset as u32 as u64) << 33) | 1;
            let mut encoded = Vec::new();
            resolver.hash_property_key(&mut encoded, key_bits as i64, context)?;
            references.push(encoded);
        }
        if instruction.op.name == "STR" {
            let index = ((code[start] as u32 >> 16) & 0xffff) as usize;
            let name = resolver
                .cache
                .static_names
                .get(index)
                .ok_or_else(|| unresolved(context, "StaticNames index", index as i64))?;
            raw[0] = (raw[0] as u32 & 0x0000_ffff) as i32;
            let mut encoded = Vec::new();
            tag(&mut encoded, b"static-name-identity");
            string(&mut encoded, name);
            references.push(encoded);
        } else if instruction.op.name == "PshC4"
            && next_callee_name(&instructions, instruction_index, code, resolver)
                == Some("__STATIC_NAME")
        {
            let index = code[start + 1];
            let name = usize::try_from(index)
                .ok()
                .and_then(|index| resolver.cache.static_names.get(index))
                .ok_or_else(|| unresolved(context, "StaticNames index", index as i64))?;
            raw[1] = 0;
            let mut encoded = Vec::new();
            tag(&mut encoded, b"static-name-identity");
            string(&mut encoded, name);
            references.push(encoded);
        }
        tag(sink, b"instruction");
        u32v(sink, instruction.op.opcode as u32);
        count(sink, raw.len());
        for value in raw {
            i32v(sink, value);
        }
        count(sink, references.len());
        for reference in references {
            bytes(sink, &reference);
        }
    }
    Ok(())
}

#[derive(Default)]
struct InvokeBudget {
    nodes: usize,
    bytes: usize,
}

fn hash_invoke_value(
    sink: &mut impl Sink,
    value: &CanonicalInvokeValueV1,
    depth: usize,
    budget: &mut InvokeBudget,
) -> Result<(), SemanticObserverError> {
    if depth > MAX_INVOKE_DEPTH {
        return Err(SemanticObserverError::InvalidInvoke(format!(
            "nesting exceeds {MAX_INVOKE_DEPTH}"
        )));
    }
    budget.nodes += 1;
    if budget.nodes > MAX_INVOKE_NODES {
        return Err(SemanticObserverError::InvalidInvoke(format!(
            "node count exceeds {MAX_INVOKE_NODES}"
        )));
    }
    match value {
        CanonicalInvokeValueV1::Null => tag(sink, b"null"),
        CanonicalInvokeValueV1::Bool(value) => {
            tag(sink, b"bool");
            boolv(sink, *value);
        }
        CanonicalInvokeValueV1::I64(value) => {
            tag(sink, b"i64");
            i64v(sink, *value);
        }
        CanonicalInvokeValueV1::U64(value) => {
            tag(sink, b"u64");
            u64v(sink, *value);
        }
        CanonicalInvokeValueV1::F32Bits(value) => {
            tag(sink, b"f32-bits");
            u32v(sink, *value);
        }
        CanonicalInvokeValueV1::F64Bits(value) => {
            tag(sink, b"f64-bits");
            u64v(sink, *value);
        }
        CanonicalInvokeValueV1::Utf8(value) => {
            charge_invoke_bytes(budget, value.len())?;
            tag(sink, b"utf8");
            string(sink, value);
        }
        CanonicalInvokeValueV1::Bytes(value) => {
            charge_invoke_bytes(budget, value.len())?;
            tag(sink, b"bytes");
            bytes(sink, value);
        }
        CanonicalInvokeValueV1::Sequence(values) => {
            tag(sink, b"sequence");
            count(sink, values.len());
            for value in values {
                hash_invoke_value(sink, value, depth + 1, budget)?;
            }
        }
        CanonicalInvokeValueV1::Record(fields) => {
            let mut fields: Vec<_> = fields.iter().collect();
            fields.sort_by(|left, right| left.0.cmp(&right.0));
            for pair in fields.windows(2) {
                if pair[0].0 == pair[1].0 {
                    return Err(SemanticObserverError::InvalidInvoke(format!(
                        "duplicate record field {:?}",
                        pair[0].0
                    )));
                }
            }
            tag(sink, b"record");
            count(sink, fields.len());
            for (name, value) in fields {
                charge_invoke_bytes(budget, name.len())?;
                string(sink, name);
                hash_invoke_value(sink, value, depth + 1, budget)?;
            }
        }
    }
    Ok(())
}

fn charge_invoke_bytes(
    budget: &mut InvokeBudget,
    additional: usize,
) -> Result<(), SemanticObserverError> {
    budget.bytes = budget
        .bytes
        .checked_add(additional)
        .ok_or_else(|| SemanticObserverError::InvalidInvoke("byte count overflow".to_owned()))?;
    if budget.bytes > MAX_INVOKE_BYTES {
        return Err(SemanticObserverError::InvalidInvoke(format!(
            "payload exceeds {MAX_INVOKE_BYTES} bytes"
        )));
    }
    Ok(())
}
