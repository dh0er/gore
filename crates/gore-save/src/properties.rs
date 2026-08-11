//! Typed parser for the G1R decompressed private payload property stream.
//!
//! Implements the byte-exact grammar documented in
//! `work/docs/ue-property-structure.md` and proven against full real saves
//! (G1R-001/002/005 + Profile_0_Screenshots) by `work/tools/validate_coverage.py`:
//!
//! Root object:
//! ```text
//! FString class | u8 flag | PropertyList ("None"-terminated) | u32 footer
//! ```
//!
//! Property tag (G1R variant of FPropertyTag):
//! ```text
//! FString name      "None" => terminator, nothing follows
//! FString type
//! [type descriptors]
//! u32 array_index   observed 0 everywhere
//! u32 size          payload byte count (BoolProperty: 0)
//! u8  tag_flags     EPropertyTagFlags: 0x08 native-serialize, 0x10 bool-true
//! [size bytes payload]
//! ```
//!
//! Struct payloads are tagged property lists unless `tag_flags & 0x08`, in
//! which case they use a native binary layout (Vector/Quat/Guid/DateTime/
//! GameplayTagContainer/InstancedStruct/...). FGameplayTag standalone is a
//! property list (`TagName: NameProperty`), only the container is packed.

use crate::{CoreError, Reader};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

pub const TAG_FLAG_NATIVE_SERIALIZE: u8 = 0x08;
pub const TAG_FLAG_BOOL_TRUE: u8 = 0x10;

const MAX_DEPTH: usize = 96;

/// An interned property or type name.
///
/// A real save's payload parses into ~1.4 million properties, and the names on them
/// are drawn from a few thousand distinct strings — `"IntProperty"` alone appears
/// hundreds of thousands of times. Allocating one `String` per occurrence made the
/// allocator the parser's dominant cost, so names are shared instead: equal text is
/// stored once per thread and handed out as a cheap pointer clone.
///
/// It compares and reads like a `&str` (`Deref`, `PartialEq<&str>`, `Display`), so
/// call sites treat it as the string it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PropStr(Arc<str>);

/// A small non-cryptographic hasher for the name table.
///
/// The table is looked up several million times per parse and holds nothing but
/// short, internal, non-attacker-chosen strings, so the default SipHash costs more
/// than the allocation interning is there to avoid. This is the usual
/// multiply-and-rotate word hash.
///
/// The names come out of the save file, so they are chosen by whoever wrote it —
/// and a fixed hash that anyone can read here would let a crafted save drop
/// thousands of names into one bucket and turn the millions of lookups a parse
/// makes into a quadratic hang, long before the size caps could bite. The state is
/// therefore started from a value drawn once per process, which no save can know.
/// This is not a cryptographic hash and does not pretend to be; it is that unknown
/// starting point that makes collisions impossible to work out ahead of time.
#[derive(Clone, Copy)]
struct NameHasher(u64);

/// Drawn once per process from the standard library's randomly seeded hasher, so
/// no dependency is needed to get an unpredictable value.
fn name_hash_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    static SEED: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SEED.get_or_init(|| {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u8(0);
        hasher.finish()
    })
}

impl std::hash::Hasher for NameHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let mut hash = self.0;
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let word = u64::from_le_bytes(chunk.try_into().expect("chunks_exact(8)"));
            hash = (hash.rotate_left(5) ^ word).wrapping_mul(SEED);
        }
        let mut tail = 0u64;
        for (index, byte) in chunks.remainder().iter().enumerate() {
            tail |= u64::from(*byte) << (index * 8);
        }
        self.0 = (hash.rotate_left(5) ^ tail).wrapping_mul(SEED);
    }
}

#[derive(Clone, Copy)]
struct NameHasherBuilder(u64);

impl NameHasherBuilder {
    fn new() -> Self {
        NameHasherBuilder(name_hash_seed())
    }
}

impl std::hash::BuildHasher for NameHasherBuilder {
    type Hasher = NameHasher;
    fn build_hasher(&self) -> NameHasher {
        NameHasher(self.0)
    }
}

thread_local! {
    /// Names seen on this thread. Bounded so a save full of unique strings cannot
    /// grow it without limit; past the cap new names simply are not shared.
    static INTERNED_NAMES: std::cell::RefCell<HashSet<Arc<str>, NameHasherBuilder>> =
        std::cell::RefCell::new(HashSet::with_hasher(NameHasherBuilder::new()));
    /// Bytes of text the table above is holding on to.
    static INTERNED_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What the shared table may keep. A count alone does not bound memory: the table
/// lives for the life of the thread, and a malformed or modded save could fill it
/// with tens of thousands of very long names and pin them there long after the save
/// was closed. Real property names are short and few, so a name that busts either
/// limit is simply not shared — it is still returned, and it is released with the
/// tree that holds it, exactly as before interning existed.
const MAX_INTERNED_NAMES: usize = 1 << 16;
const MAX_INTERNED_BYTES: usize = 4 << 20;
const MAX_INTERNED_NAME_LEN: usize = 256;

impl PropStr {
    /// Intern `text`, reusing the copy this thread already holds when there is one.
    pub fn new(text: &str) -> Self {
        INTERNED_NAMES.with(|names| {
            let mut names = names.borrow_mut();
            if let Some(shared) = names.get(text) {
                return PropStr(shared.clone());
            }
            let shared: Arc<str> = Arc::from(text);
            if text.len() <= MAX_INTERNED_NAME_LEN && names.len() < MAX_INTERNED_NAMES {
                INTERNED_BYTES.with(|held| {
                    let next = held.get().saturating_add(text.len());
                    if next <= MAX_INTERNED_BYTES {
                        held.set(next);
                        names.insert(shared.clone());
                    }
                });
            }
            PropStr(shared)
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for PropStr {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PropStr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for PropStr {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PropStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for PropStr {
    fn from(value: &str) -> Self {
        PropStr::new(value)
    }
}

impl From<String> for PropStr {
    fn from(value: String) -> Self {
        PropStr::new(&value)
    }
}

impl From<&PropStr> for String {
    fn from(value: &PropStr) -> Self {
        value.0.to_string()
    }
}

impl PartialEq<str> for PropStr {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for PropStr {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for PropStr {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl PartialEq<PropStr> for str {
    fn eq(&self, other: &PropStr) -> bool {
        self == &*other.0
    }
}

impl PartialEq<PropStr> for &str {
    fn eq(&self, other: &PropStr) -> bool {
        *self == &*other.0
    }
}

impl PartialEq<PropStr> for String {
    fn eq(&self, other: &PropStr) -> bool {
        self.as_str() == &*other.0
    }
}

impl serde::Serialize for PropStr {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RootObject {
    pub class: String,
    pub flag: u8,
    pub properties: Vec<Property>,
    pub footer: u32,
    /// Total bytes consumed (== payload length when fully parsed).
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: PropStr,
    pub type_name: PropStr,
    pub descriptor: Descriptor,
    pub array_index: u32,
    pub tag_flags: u8,
    /// Absolute offset of the sized value payload within the parsed buffer.
    pub value_offset: usize,
    pub value_size: usize,
    pub value: PropertyValue,
}

impl Property {
    /// Absolute offset of the tag's u32 `size` field. The value payload is
    /// always preceded by `u32 size | u8 tag_flags`, so the size field sits
    /// five bytes before the recorded value offset.
    pub fn size_field_offset(&self) -> usize {
        self.value_offset - 5
    }
}

/// Type descriptors serialized between the property type and the value header.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Descriptor {
    /// StructProperty: (struct_type, package). Boxed like the others: a payload
    /// parses into over a million properties and most carry no descriptor at all,
    /// so what this costs when absent is what matters.
    pub struct_type: Option<Box<(PropStr, PropStr)>>,
    /// EnumProperty: (enum_type, package, underlying_type)
    pub enum_type: Option<Box<(PropStr, PropStr, PropStr)>>,
    /// Array/Set inner type, Map key/value types (with nested descriptors).
    pub inner: Option<Box<InnerDescriptor>>,
    pub map: Option<Box<(InnerDescriptor, InnerDescriptor)>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InnerDescriptor {
    pub type_name: PropStr,
    pub struct_type: Option<Box<(PropStr, PropStr)>>,
    pub enum_type: Option<Box<(PropStr, PropStr, PropStr)>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Int(i32),
    UInt32(u32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    Byte(u8),
    Str(String),
    Name(String),
    Object(String),
    Enum(String),
    SoftObject(SoftObjectPath),
    Struct(StructValue),
    Array {
        elements: Vec<PropertyValue>,
    },
    /// ArrayProperty<ObjectProperty> whose elements are inline-serialized
    /// objects rather than bare paths.
    ObjectInstances(Vec<ObjectInstance>),
    Set {
        num_to_remove: u32,
        elements: Vec<PropertyValue>,
    },
    Map {
        num_to_remove: u32,
        entries: Vec<(PropertyValue, PropertyValue)>,
    },
    /// TextProperty and other opaque payloads kept as raw bytes.
    Opaque(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoftObjectPath {
    pub package_name: String,
    pub asset_name: String,
    pub sub_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInstance {
    pub class: String,
    pub flag: u8,
    pub properties: Vec<Property>,
    pub footer: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructValue {
    Vector3 {
        x: f64,
        y: f64,
        z: f64,
    },
    Vector3f {
        x: f32,
        y: f32,
        z: f32,
    },
    Vector4 {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    },
    Vector2 {
        x: f64,
        y: f64,
    },
    Guid([u8; 16]),
    DateTime(i64),
    GameplayTagContainer(Vec<String>),
    /// FInstancedStruct; `None` when unset (empty type, zero size).
    Instanced(Option<InstancedStruct>),
    Properties(Vec<Property>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstancedStruct {
    pub actual_type: PropStr,
    /// Absolute offset of the u32 `data_size` field preceding the body.
    /// Length-changing edits inside this struct must adjust it.
    pub data_size_offset: usize,
    pub properties: Vec<Property>,
}

/// One step of a typed-property path.
///
/// String form (used by the `private.typed.setValue` edit):
/// - `name`      — property by name in the current property list
/// - `{key}`     — map entry by stringified key (Str/Name/Enum/Object keys)
/// - `[3]`       — array/set element or instanced-object index
///
/// Struct property lists and InstancedStruct wrappers are descended through
/// the property segment itself; map values that are InstancedStructs are
/// unwrapped transparently after a `{key}` segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSeg {
    Name(String),
    MapKey(String),
    Index(usize),
}

pub fn parse_path(segments: &[String]) -> Result<Vec<PathSeg>, CoreError> {
    segments
        .iter()
        .map(|raw| {
            if let Some(key) = raw.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                Ok(PathSeg::MapKey(key.to_string()))
            } else if let Some(idx) = raw.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                idx.parse::<usize>().map(PathSeg::Index).map_err(|_| {
                    CoreError::InvalidRequest(format!("invalid index segment {raw:?}"))
                })
            } else if raw.is_empty() {
                Err(CoreError::InvalidRequest("empty path segment".to_string()))
            } else {
                Ok(PathSeg::Name(raw.clone()))
            }
        })
        .collect()
}

/// Render a map key as the `{mapKey}` path segment used by both the typed
/// search (to build paths) and `resolve` (to match them). The two must stay in
/// lockstep: any key type search can label must also be resolvable here, or a
/// nested scalar would surface as editable with a path `setValue` cannot find.
/// Returns `None` for key types that cannot be addressed as a path segment.
pub(crate) fn map_key_to_string(key: &PropertyValue) -> Option<String> {
    match key {
        PropertyValue::Str(s)
        | PropertyValue::Name(s)
        | PropertyValue::Enum(s)
        | PropertyValue::Object(s) => Some(s.clone()),
        PropertyValue::Int(i) => Some(i.to_string()),
        PropertyValue::Struct(StructValue::Guid(raw)) => Some(hex_guid(raw)),
        _ => None,
    }
}

/// A resolved typed path plus the absolute offsets of every enclosing u32
/// size field crossed on the way to the target (outermost first): the tag
/// `size` of each ancestor property and the `data_size` of each
/// InstancedStruct wrapper. Length-changing edits must add their byte delta
/// to each of these fields; the target's own size field is not included.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedChain<'a> {
    pub target: &'a Property,
    pub enclosing_size_fields: Vec<usize>,
}

/// Resolve a path to a tagged property within a parsed tree. The target must
/// be a `Property` (it carries the absolute value offset needed for patching).
pub fn resolve<'a>(
    properties: &'a [Property],
    path: &[PathSeg],
) -> Result<&'a Property, CoreError> {
    resolve_chain(properties, path).map(|chain| chain.target)
}

/// Like [`resolve`], but also collects the enclosing size-field offsets needed
/// to apply a length-changing patch at the target.
pub fn resolve_chain<'a>(
    properties: &'a [Property],
    path: &[PathSeg],
) -> Result<ResolvedChain<'a>, CoreError> {
    let mut enclosing_size_fields = Vec::new();
    let target = resolve_in_properties(properties, path, &mut enclosing_size_fields)?;
    Ok(ResolvedChain {
        target,
        enclosing_size_fields,
    })
}

fn resolve_in_properties<'a>(
    properties: &'a [Property],
    path: &[PathSeg],
    sizes: &mut Vec<usize>,
) -> Result<&'a Property, CoreError> {
    let Some((first, rest)) = path.split_first() else {
        return Err(CoreError::InvalidRequest("empty typed path".to_string()));
    };
    let PathSeg::Name(name) = first else {
        return Err(CoreError::InvalidRequest(format!(
            "path must start with a property name, got {first:?}"
        )));
    };
    let property = properties
        .iter()
        .find(|p| &p.name == name)
        .ok_or_else(|| CoreError::Parse(format!("property {name:?} not found")))?;
    if rest.is_empty() {
        return Ok(property);
    }
    sizes.push(property.size_field_offset());
    resolve_in_value(&property.value, rest, sizes)
}

fn resolve_in_value<'a>(
    value: &'a PropertyValue,
    path: &[PathSeg],
    sizes: &mut Vec<usize>,
) -> Result<&'a Property, CoreError> {
    let (seg, rest) = path.split_first().expect("path checked non-empty");
    match (value, seg) {
        (PropertyValue::Struct(StructValue::Properties(inner)), PathSeg::Name(_)) => {
            resolve_in_properties(inner, path, sizes)
        }
        (PropertyValue::Struct(StructValue::Instanced(Some(instanced))), PathSeg::Name(_)) => {
            sizes.push(instanced.data_size_offset);
            resolve_in_properties(&instanced.properties, path, sizes)
        }
        (PropertyValue::Map { entries, .. }, PathSeg::MapKey(wanted)) => {
            let entry = entries
                .iter()
                .find(|(k, _)| map_key_to_string(k).as_deref() == Some(wanted.as_str()))
                .ok_or_else(|| CoreError::Parse(format!("map key {wanted:?} not found")))?;
            if rest.is_empty() {
                return Err(CoreError::InvalidRequest(
                    "path may not end on a map entry; address a property inside it".to_string(),
                ));
            }
            resolve_in_value(&entry.1, rest, sizes)
        }
        (PropertyValue::Array { elements }, PathSeg::Index(i))
        | (PropertyValue::Set { elements, .. }, PathSeg::Index(i)) => {
            let element = elements
                .get(*i)
                .ok_or_else(|| CoreError::Parse(format!("index {i} out of bounds")))?;
            if rest.is_empty() {
                return Err(CoreError::InvalidRequest(
                    "path may not end on a container element; address a property inside it"
                        .to_string(),
                ));
            }
            resolve_in_value(element, rest, sizes)
        }
        (PropertyValue::ObjectInstances(instances), PathSeg::Index(i)) => {
            let instance = instances
                .get(*i)
                .ok_or_else(|| CoreError::Parse(format!("object index {i} out of bounds")))?;
            if rest.is_empty() {
                return Err(CoreError::InvalidRequest(
                    "path may not end on an object instance; address a property inside it"
                        .to_string(),
                ));
            }
            resolve_in_properties(&instance.properties, rest, sizes)
        }
        (other, seg) => Err(CoreError::InvalidRequest(format!(
            "segment {seg:?} cannot descend into {}",
            value_kind(other)
        ))),
    }
}

fn value_kind(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Int(_) => "Int",
        PropertyValue::UInt32(_) => "UInt32",
        PropertyValue::Int64(_) => "Int64",
        PropertyValue::Float(_) => "Float",
        PropertyValue::Double(_) => "Double",
        PropertyValue::Bool(_) => "Bool",
        PropertyValue::Byte(_) => "Byte",
        PropertyValue::Str(_) => "Str",
        PropertyValue::Name(_) => "Name",
        PropertyValue::Object(_) => "Object",
        PropertyValue::Enum(_) => "Enum",
        PropertyValue::SoftObject(_) => "SoftObject",
        PropertyValue::Struct(StructValue::Properties(_)) => "Struct",
        PropertyValue::Struct(StructValue::Instanced(_)) => "InstancedStruct",
        PropertyValue::Struct(_) => "NativeStruct",
        PropertyValue::Array { .. } => "Array",
        PropertyValue::ObjectInstances(_) => "ObjectInstances",
        PropertyValue::Set { .. } => "Set",
        PropertyValue::Map { .. } => "Map",
        PropertyValue::Opaque(_) => "Opaque",
    }
}

/// A single property surfaced by a typed search, addressable for editing.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyHit {
    /// setValue-compatible path segments (name / `{mapKey}` / `[index]`).
    pub path: Vec<String>,
    /// Human-readable dotted path for display.
    pub display: String,
    pub type_name: String,
    /// Formatted current value.
    pub value_display: String,
    /// True for values `private.typed.setValue` can patch: fixed-size scalars
    /// and Str/Name strings (length-changing, size chain fixed up on write).
    pub editable: bool,
}

/// One node in the exhaustive property browser. Unlike [`PropertyHit`], this
/// represents containers, native structs, inline container elements and opaque
/// payloads as well as scalar leaves. `edit_value` is the lossless JSON shape
/// accepted by `private.typed.setValue`; it is only populated for nodes that
/// are actually addressable and editable.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyNode {
    /// Deterministic DFS ordinal, stable across queries/filters for one tree.
    pub ordinal: usize,
    pub path: Vec<String>,
    pub display: String,
    pub type_name: String,
    pub struct_type: Option<String>,
    pub kind: String,
    pub value_display: String,
    pub edit_value: Option<Value>,
    pub editable: bool,
    pub child_count: usize,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyBrowseOptions<'a> {
    pub query: &'a str,
    pub type_filter: Option<&'a str>,
    pub kind_filter: Option<&'a str>,
    pub editable_filter: Option<bool>,
    pub offset: usize,
    pub limit: usize,
    /// PUBLIC properties share the grammar but are intentionally read-only.
    pub allow_edits: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBrowseResult {
    pub nodes: Vec<PropertyNode>,
    pub total: usize,
    pub editable: usize,
    pub read_only: usize,
    pub kind_counts: BTreeMap<String, usize>,
    pub type_counts: BTreeMap<String, usize>,
}

/// Walk the whole property tree and collect properties whose display path
/// contains every whitespace-separated term in `query` (case-insensitive). An
/// empty query matches everything. Returns the `[offset, offset+limit)` page of
/// matches plus the total match count (the whole tree is walked either way, so
/// the total supports last-page navigation).
pub fn search_properties(
    root: &RootObject,
    query: &str,
    offset: usize,
    limit: usize,
) -> (Vec<PropertyHit>, usize) {
    let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    let mut hits = Vec::new();
    let mut total = 0usize;
    let mut ctx = SearchCtx {
        terms: &terms,
        offset,
        limit,
        total: &mut total,
        hits: &mut hits,
    };
    walk_search(
        &root.properties,
        &mut Vec::new(),
        &mut String::new(),
        true,
        &mut ctx,
    );
    (hits, total)
}

struct SearchCtx<'a> {
    terms: &'a [String],
    offset: usize,
    limit: usize,
    total: &'a mut usize,
    hits: &'a mut Vec<PropertyHit>,
}

impl SearchCtx<'_> {
    /// Record a match: count it toward the total and push it if it falls inside
    /// the requested page window.
    fn record(&mut self, hit: PropertyHit) {
        let index = *self.total;
        *self.total += 1;
        if index >= self.offset && self.hits.len() < self.limit {
            self.hits.push(hit);
        }
    }
}

fn scalar_editable(value: &PropertyValue) -> bool {
    matches!(
        value,
        PropertyValue::Int(_)
            | PropertyValue::UInt32(_)
            | PropertyValue::Int64(_)
            | PropertyValue::Float(_)
            | PropertyValue::Double(_)
            | PropertyValue::Bool(_)
            | PropertyValue::Byte(_)
            | PropertyValue::Str(_)
            | PropertyValue::Name(_)
            | PropertyValue::Object(_)
            | PropertyValue::Enum(_)
    )
}

fn scalar_display(value: &PropertyValue) -> Option<String> {
    Some(match value {
        PropertyValue::Int(v) => v.to_string(),
        PropertyValue::UInt32(v) => v.to_string(),
        PropertyValue::Int64(v) => v.to_string(),
        PropertyValue::Float(v) => v.to_string(),
        PropertyValue::Double(v) => v.to_string(),
        PropertyValue::Bool(v) => v.to_string(),
        PropertyValue::Byte(v) => v.to_string(),
        PropertyValue::Str(s)
        | PropertyValue::Name(s)
        | PropertyValue::Object(s)
        | PropertyValue::Enum(s) => s.clone(),
        PropertyValue::SoftObject(p) => p.package_name.clone(),
        _ => return None,
    })
}

fn walk_search(
    props: &[Property],
    path: &mut Vec<String>,
    display: &mut String,
    ancestors_addressable: bool,
    ctx: &mut SearchCtx,
) {
    let mut name_counts = HashMap::<&str, usize>::new();
    for property in props {
        *name_counts.entry(property.name.as_str()).or_default() += 1;
    }
    for p in props {
        let display_len = display.len();
        if !display.is_empty() {
            display.push_str(" › ");
        }
        display.push_str(&p.name);
        path.push(p.name.to_string());
        let addressable =
            ancestors_addressable && name_counts.get(p.name.as_str()).copied() == Some(1);

        // Leaf value?
        if let Some(value_display) = scalar_display(&p.value) {
            if ctx.terms.iter().all(|t| display.to_lowercase().contains(t)) {
                ctx.record(PropertyHit {
                    path: path.clone(),
                    display: display.clone(),
                    type_name: p.type_name.to_string(),
                    value_display,
                    editable: addressable && scalar_editable(&p.value),
                });
            }
        } else {
            walk_value_search(&p.value, path, display, addressable, ctx);
        }

        path.pop();
        display.truncate(display_len);
    }
}

fn walk_value_search(
    value: &PropertyValue,
    path: &mut Vec<String>,
    display: &mut String,
    ancestors_addressable: bool,
    ctx: &mut SearchCtx,
) {
    match value {
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            walk_search(inner, path, display, ancestors_addressable, ctx);
        }
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
            walk_search(&i.properties, path, display, ancestors_addressable, ctx);
        }
        PropertyValue::ObjectInstances(objs) => {
            for (idx, obj) in objs.iter().enumerate() {
                descend_indexed(
                    idx,
                    &obj.properties,
                    path,
                    display,
                    ancestors_addressable,
                    ctx,
                );
            }
        }
        PropertyValue::Map { entries, .. } => {
            let labels = entries
                .iter()
                .map(|(key, _)| map_key_to_string(key))
                .collect::<Vec<_>>();
            let mut counts = HashMap::<&str, usize>::new();
            for label in labels.iter().flatten() {
                *counts.entry(label.as_str()).or_default() += 1;
            }
            for (index, ((_, value), label)) in entries.iter().zip(labels.iter()).enumerate() {
                let unique = label
                    .as_deref()
                    .is_some_and(|label| counts.get(label).copied() == Some(1));
                let segment = match label {
                    Some(label) if unique => format!("{{{label}}}"),
                    Some(label) => format!("{{{label}}} [#{index}]"),
                    None => format!("{{? #{index}}}"),
                };
                descend_value(
                    &segment,
                    value,
                    path,
                    display,
                    ancestors_addressable && unique,
                    ctx,
                );
            }
        }
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
            for (idx, el) in elements.iter().enumerate() {
                descend_value(
                    &format!("[{idx}]"),
                    el,
                    path,
                    display,
                    ancestors_addressable,
                    ctx,
                );
            }
        }
        _ => {}
    }
}

fn descend_indexed(
    idx: usize,
    props: &[Property],
    path: &mut Vec<String>,
    display: &mut String,
    descendants_addressable: bool,
    ctx: &mut SearchCtx,
) {
    let display_len = display.len();
    let seg = format!("[{idx}]");
    display.push_str(&seg);
    path.push(seg);
    walk_search(props, path, display, descendants_addressable, ctx);
    path.pop();
    display.truncate(display_len);
}

fn descend_value(
    seg: &str,
    value: &PropertyValue,
    path: &mut Vec<String>,
    display: &mut String,
    descendants_addressable: bool,
    ctx: &mut SearchCtx,
) {
    let display_len = display.len();
    display.push_str(seg);
    path.push(seg.to_string());
    if let Some(value_display) = scalar_display(value) {
        if ctx.terms.iter().all(|t| display.to_lowercase().contains(t)) {
            ctx.record(PropertyHit {
                path: path.clone(),
                display: display.clone(),
                type_name: container_value_type(value).to_string(),
                value_display,
                // This hit's path ends on a `{mapKey}` or `[index]` segment.
                // `setValue` only resolves to tagged Property nodes and rejects
                // paths ending on a container element, so such scalars are not
                // editable even though they are fixed-size.
                editable: false,
            });
        }
    } else {
        walk_value_search(value, path, display, descendants_addressable, ctx);
    }
    path.pop();
    display.truncate(display_len);
}

fn hex_guid(raw: &[u8; 16]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

fn container_value_type(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Int(_) => "IntProperty",
        PropertyValue::UInt32(_) => "UInt32Property",
        PropertyValue::Int64(_) => "Int64Property",
        PropertyValue::Float(_) => "FloatProperty",
        PropertyValue::Double(_) => "DoubleProperty",
        PropertyValue::Bool(_) => "BoolProperty",
        PropertyValue::Byte(_) => "ByteProperty",
        PropertyValue::Str(_) => "StrProperty",
        PropertyValue::Name(_) => "NameProperty",
        PropertyValue::Object(_) => "ObjectProperty",
        PropertyValue::Enum(_) => "EnumProperty",
        PropertyValue::SoftObject(_) => "SoftObjectProperty",
        _ => "StructProperty",
    }
}

/// Exhaustive, filterable view of a typed property tree. The traversal always
/// counts the full matching set for stable pagination, but allocates/clones a
/// [`PropertyNode`] only for the requested page. That distinction matters for
/// real saves with more than a million nodes.
pub fn browse_properties(
    root: &RootObject,
    options: &PropertyBrowseOptions<'_>,
) -> PropertyBrowseResult {
    let terms = options
        .query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let mut ctx = BrowseCtx {
        terms,
        type_filter: options
            .type_filter
            .filter(|value| !value.is_empty())
            .map(str::to_lowercase),
        kind_filter: options
            .kind_filter
            .filter(|value| !value.is_empty() && *value != "all")
            .map(str::to_lowercase),
        editable_filter: options.editable_filter,
        allow_edits: options.allow_edits,
        offset: options.offset,
        limit: options.limit,
        visited: 0,
        total: 0,
        editable: 0,
        read_only: 0,
        kind_counts: BTreeMap::new(),
        type_counts: BTreeMap::new(),
        nodes: Vec::with_capacity(options.limit.min(1000)),
    };
    walk_browse_properties(
        &root.properties,
        &mut Vec::new(),
        &mut String::new(),
        true,
        &mut ctx,
    );
    PropertyBrowseResult {
        nodes: ctx.nodes,
        total: ctx.total,
        editable: ctx.editable,
        read_only: ctx.read_only,
        kind_counts: ctx.kind_counts,
        type_counts: ctx.type_counts,
    }
}

struct BrowseCtx {
    terms: Vec<String>,
    type_filter: Option<String>,
    kind_filter: Option<String>,
    editable_filter: Option<bool>,
    allow_edits: bool,
    offset: usize,
    limit: usize,
    /// Counts every visible tree node before filtering. Used as a stable id.
    visited: usize,
    total: usize,
    editable: usize,
    read_only: usize,
    kind_counts: BTreeMap<String, usize>,
    type_counts: BTreeMap<String, usize>,
    nodes: Vec<PropertyNode>,
}

impl BrowseCtx {
    /// Return the stable node ordinal when this match belongs to the requested
    /// page, or `None` when it is filtered/outside the page.
    #[allow(clippy::too_many_arguments)]
    fn slot(
        &mut self,
        display: &str,
        type_name: &str,
        struct_type: Option<&str>,
        kind: &str,
        scalar_map_entry: bool,
        value_display: &str,
        addressable_editable: bool,
    ) -> Option<usize> {
        let ordinal = self.visited;
        self.visited += 1;
        let editable = self.allow_edits && addressable_editable;
        if self
            .editable_filter
            .is_some_and(|wanted| wanted != editable)
        {
            return None;
        }
        if self.type_filter.as_ref().is_some_and(|wanted| {
            !type_name.to_lowercase().contains(wanted)
                && !struct_type.is_some_and(|value| value.to_lowercase().contains(wanted))
        }) {
            return None;
        }
        if self
            .kind_filter
            .as_deref()
            .is_some_and(|wanted| !kind_filter_matches(wanted, kind, scalar_map_entry))
        {
            return None;
        }
        if !self.terms.is_empty() {
            let display = display.to_lowercase();
            let type_name = type_name.to_lowercase();
            let struct_type = struct_type.unwrap_or_default().to_lowercase();
            let kind = kind.to_lowercase();
            let value = value_display.to_lowercase();
            if !self.terms.iter().all(|term| {
                display.contains(term)
                    || type_name.contains(term)
                    || struct_type.contains(term)
                    || kind.contains(term)
                    || value.contains(term)
            }) {
                return None;
            }
        }
        if editable {
            self.editable += 1;
        } else {
            self.read_only += 1;
        }
        increment_count(&mut self.kind_counts, kind);
        increment_count(&mut self.type_counts, type_name);
        let match_index = self.total;
        self.total += 1;
        (match_index >= self.offset && self.nodes.len() < self.limit).then_some(ordinal)
    }
}

fn increment_count(counts: &mut BTreeMap<String, usize>, key: &str) {
    if let Some(count) = counts.get_mut(key) {
        *count += 1;
    } else {
        counts.insert(key.to_string(), 1);
    }
}

fn kind_filter_matches(filter: &str, kind: &str, scalar_map_entry: bool) -> bool {
    match filter {
        "container" => matches!(
            kind,
            "array"
                | "map"
                | "set"
                | "objectArray"
                | "arrayElement"
                | "setElement"
                | "mapEntry"
                | "objectInstance"
        ),
        "struct" => matches!(kind, "struct" | "nativeStruct" | "instancedStruct"),
        // A map entry remains a `mapEntry` node (and therefore read-only), but
        // when its value has no children it is also a scalar leaf for filtering
        // purposes. This makes primitive TMap values discoverable alongside
        // tagged scalars without misrepresenting their kind or writability.
        "scalar" => kind == "scalar" || scalar_map_entry,
        other => kind.eq_ignore_ascii_case(other),
    }
}

fn walk_browse_properties(
    props: &[Property],
    path: &mut Vec<String>,
    display: &mut String,
    ancestors_addressable: bool,
    ctx: &mut BrowseCtx,
) {
    // `resolve_chain` selects the first property with a matching name. Mark
    // duplicate-name siblings read-only so the UI never promises a path that
    // could silently resolve to the wrong occurrence.
    let mut name_counts = HashMap::<&str, usize>::new();
    for property in props {
        *name_counts.entry(property.name.as_str()).or_default() += 1;
    }
    for property in props {
        let display_len = display.len();
        if !display.is_empty() {
            display.push_str(" › ");
        }
        display.push_str(&property.name);
        path.push(property.name.to_string());
        let addressable =
            ancestors_addressable && name_counts.get(property.name.as_str()).copied() == Some(1);
        let kind = property_kind(&property.value);
        let struct_type = property
            .descriptor
            .struct_type
            .as_deref()
            .map(|(name, _)| name.as_str());
        let value_display = browse_value_preview(&property.value);
        let addressable_editable = addressable && browse_value_editable(&property.value);
        let child_count = browse_child_count(&property.value);
        if let Some(ordinal) = ctx.slot(
            display,
            &property.type_name,
            struct_type,
            kind,
            false,
            &value_display,
            addressable_editable,
        ) {
            ctx.nodes.push(PropertyNode {
                ordinal,
                path: path.clone(),
                display: display.clone(),
                type_name: property.type_name.to_string(),
                struct_type: struct_type.map(str::to_string),
                kind: kind.to_string(),
                value_display,
                edit_value: addressable_editable
                    .then(|| browse_value_json(&property.value))
                    .flatten(),
                editable: ctx.allow_edits && addressable_editable,
                child_count,
                depth: path.len().saturating_sub(1),
            });
        }
        walk_browse_value_children(&property.value, path, display, addressable, ctx);
        path.pop();
        display.truncate(display_len);
    }
}

fn walk_browse_value_children(
    value: &PropertyValue,
    path: &mut Vec<String>,
    display: &mut String,
    ancestors_addressable: bool,
    ctx: &mut BrowseCtx,
) {
    match value {
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            walk_browse_properties(inner, path, display, ancestors_addressable, ctx);
        }
        PropertyValue::Struct(StructValue::Instanced(Some(instance))) => {
            walk_browse_properties(
                &instance.properties,
                path,
                display,
                ancestors_addressable,
                ctx,
            );
        }
        PropertyValue::ObjectInstances(instances) => {
            for (index, instance) in instances.iter().enumerate() {
                let segment = format!("[{index}]");
                descend_browse_inline(
                    &segment,
                    "ObjectInstance",
                    "objectInstance",
                    &format!(
                        "{} · {} properties",
                        instance.class,
                        instance.properties.len()
                    ),
                    instance.properties.len(),
                    false,
                    None,
                    path,
                    display,
                    ancestors_addressable,
                    ctx,
                    |path, display, addressable, ctx| {
                        walk_browse_properties(
                            &instance.properties,
                            path,
                            display,
                            addressable,
                            ctx,
                        )
                    },
                );
            }
        }
        PropertyValue::Map { entries, .. } => {
            let labels = entries
                .iter()
                .map(|(key, _)| map_key_to_string(key))
                .collect::<Vec<_>>();
            let mut counts = HashMap::<&str, usize>::new();
            for label in labels.iter().flatten() {
                *counts.entry(label.as_str()).or_default() += 1;
            }
            for (index, ((key, entry_value), label)) in
                entries.iter().zip(labels.iter()).enumerate()
            {
                let unique = label
                    .as_deref()
                    .is_some_and(|value| counts.get(value).copied() == Some(1));
                let segment = match label {
                    Some(label) if unique => format!("{{{label}}}"),
                    Some(label) => format!("{{{label}}} [#{index}]"),
                    None => format!("{{? #{index}}}"),
                };
                let key_preview = browse_value_preview(key);
                let value_preview = browse_value_preview(entry_value);
                let preview = format!("{key_preview} → {value_preview}");
                let child_count = browse_child_count(entry_value);
                descend_browse_inline(
                    &segment,
                    container_value_type(entry_value),
                    "mapEntry",
                    &preview,
                    child_count,
                    scalar_display(entry_value).is_some(),
                    inline_struct_type(entry_value),
                    path,
                    display,
                    ancestors_addressable && unique,
                    ctx,
                    |path, display, addressable, ctx| {
                        walk_browse_value_children(entry_value, path, display, addressable, ctx)
                    },
                );
            }
        }
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
            let is_set = matches!(value, PropertyValue::Set { .. });
            for (index, element) in elements.iter().enumerate() {
                let segment = format!("[{index}]");
                let preview = browse_value_preview(element);
                descend_browse_inline(
                    &segment,
                    container_value_type(element),
                    if is_set { "setElement" } else { "arrayElement" },
                    &preview,
                    browse_child_count(element),
                    false,
                    inline_struct_type(element),
                    path,
                    display,
                    ancestors_addressable,
                    ctx,
                    |path, display, addressable, ctx| {
                        walk_browse_value_children(element, path, display, addressable, ctx)
                    },
                );
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn descend_browse_inline<F>(
    segment: &str,
    type_name: &str,
    kind: &str,
    value_display: &str,
    child_count: usize,
    scalar_map_entry: bool,
    struct_type: Option<&str>,
    path: &mut Vec<String>,
    display: &mut String,
    descendants_addressable: bool,
    ctx: &mut BrowseCtx,
    descend: F,
) where
    F: FnOnce(&mut Vec<String>, &mut String, bool, &mut BrowseCtx),
{
    let display_len = display.len();
    display.push_str(segment);
    path.push(segment.to_string());
    if let Some(ordinal) = ctx.slot(
        display,
        type_name,
        struct_type,
        kind,
        scalar_map_entry,
        value_display,
        false,
    ) {
        ctx.nodes.push(PropertyNode {
            ordinal,
            path: path.clone(),
            display: display.clone(),
            type_name: type_name.to_string(),
            struct_type: struct_type.map(str::to_string),
            kind: kind.to_string(),
            value_display: value_display.to_string(),
            edit_value: None,
            editable: false,
            child_count,
            depth: path.len().saturating_sub(1),
        });
    }
    descend(path, display, descendants_addressable, ctx);
    path.pop();
    display.truncate(display_len);
}

fn property_kind(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Struct(StructValue::Properties(_)) => "struct",
        PropertyValue::Struct(StructValue::Instanced(_)) => "instancedStruct",
        PropertyValue::Struct(_) => "nativeStruct",
        PropertyValue::Array { .. } => "array",
        PropertyValue::ObjectInstances(_) => "objectArray",
        PropertyValue::Set { .. } => "set",
        PropertyValue::Map { .. } => "map",
        PropertyValue::Opaque(_) => "opaque",
        _ => "scalar",
    }
}

fn inline_struct_type(value: &PropertyValue) -> Option<&'static str> {
    match value {
        PropertyValue::Struct(StructValue::Vector3 { .. }) => Some("Vector"),
        PropertyValue::Struct(StructValue::Vector3f { .. }) => Some("Vector3f"),
        PropertyValue::Struct(StructValue::Vector4 { .. }) => Some("Vector4/Quat"),
        PropertyValue::Struct(StructValue::Vector2 { .. }) => Some("Vector2D"),
        PropertyValue::Struct(StructValue::Guid(_)) => Some("Guid"),
        PropertyValue::Struct(StructValue::DateTime(_)) => Some("DateTime"),
        PropertyValue::Struct(StructValue::GameplayTagContainer(_)) => Some("GameplayTagContainer"),
        PropertyValue::Struct(StructValue::Instanced(_)) => Some("InstancedStruct"),
        PropertyValue::Struct(StructValue::Properties(_)) => Some("Struct"),
        _ => None,
    }
}

fn browse_child_count(value: &PropertyValue) -> usize {
    match value {
        PropertyValue::Struct(StructValue::Properties(inner)) => inner.len(),
        PropertyValue::Struct(StructValue::Instanced(Some(instance))) => instance.properties.len(),
        PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => tags.len(),
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => elements.len(),
        PropertyValue::ObjectInstances(instances) => instances.len(),
        PropertyValue::Map { entries, .. } => entries.len(),
        _ => 0,
    }
}

fn browse_value_editable(value: &PropertyValue) -> bool {
    scalar_editable(value)
        || matches!(
            value,
            PropertyValue::Struct(
                StructValue::Vector2 { .. }
                    | StructValue::Vector3 { .. }
                    | StructValue::Vector3f { .. }
                    | StructValue::Vector4 { .. }
                    | StructValue::Guid(_)
                    | StructValue::DateTime(_)
                    | StructValue::GameplayTagContainer(_)
            )
        )
}

fn browse_value_preview(value: &PropertyValue) -> String {
    match value {
        PropertyValue::SoftObject(path) => {
            let mut value = format!("{}.{}", path.package_name, path.asset_name);
            if !path.sub_path.is_empty() {
                value.push(':');
                value.push_str(&path.sub_path);
            }
            value
        }
        PropertyValue::Struct(StructValue::Vector3 { x, y, z }) => {
            format!("x: {x}, y: {y}, z: {z}")
        }
        PropertyValue::Struct(StructValue::Vector3f { x, y, z }) => {
            format!("x: {x}, y: {y}, z: {z}")
        }
        PropertyValue::Struct(StructValue::Vector4 { x, y, z, w }) => {
            format!("x: {x}, y: {y}, z: {z}, w: {w}")
        }
        PropertyValue::Struct(StructValue::Vector2 { x, y }) => format!("x: {x}, y: {y}"),
        PropertyValue::Struct(StructValue::Guid(raw)) => hex_guid(raw),
        PropertyValue::Struct(StructValue::DateTime(ticks)) => ticks.to_string(),
        PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => tags.join(", "),
        PropertyValue::Struct(StructValue::Instanced(Some(instance))) => format!(
            "{} · {} properties",
            instance.actual_type,
            instance.properties.len()
        ),
        PropertyValue::Struct(StructValue::Instanced(None)) => "empty".to_string(),
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            format!("{} properties", inner.len())
        }
        PropertyValue::Array { elements } => format!("{} elements", elements.len()),
        PropertyValue::ObjectInstances(instances) => format!("{} objects", instances.len()),
        PropertyValue::Set { elements, .. } => format!("{} elements", elements.len()),
        PropertyValue::Map { entries, .. } => format!("{} entries", entries.len()),
        PropertyValue::Opaque(bytes) => opaque_preview(bytes),
        other => scalar_display(other).unwrap_or_default(),
    }
}

fn opaque_preview(bytes: &[u8]) -> String {
    let preview = bytes
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > 16 {
        format!("{} bytes · {preview} …", bytes.len())
    } else {
        format!("{} bytes · {preview}", bytes.len())
    }
}

fn browse_value_json(value: &PropertyValue) -> Option<Value> {
    Some(match value {
        PropertyValue::Int(value) => json!(value),
        PropertyValue::UInt32(value) => json!(value),
        PropertyValue::Int64(value) => json!(value),
        PropertyValue::Float(value) => json!(value),
        PropertyValue::Double(value) => json!(value),
        PropertyValue::Bool(value) => json!(value),
        PropertyValue::Byte(value) => json!(value),
        PropertyValue::Str(value)
        | PropertyValue::Name(value)
        | PropertyValue::Object(value)
        | PropertyValue::Enum(value) => json!(value),
        PropertyValue::Struct(StructValue::Vector3 { x, y, z }) => {
            json!({ "x": x, "y": y, "z": z })
        }
        PropertyValue::Struct(StructValue::Vector3f { x, y, z }) => {
            json!({ "x": x, "y": y, "z": z })
        }
        PropertyValue::Struct(StructValue::Vector4 { x, y, z, w }) => {
            json!({ "x": x, "y": y, "z": z, "w": w })
        }
        PropertyValue::Struct(StructValue::Vector2 { x, y }) => json!({ "x": x, "y": y }),
        PropertyValue::Struct(StructValue::Guid(raw)) => json!(hex_guid(raw)),
        PropertyValue::Struct(StructValue::DateTime(ticks)) => json!(ticks),
        PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => json!(tags),
        _ => return None,
    })
}

/// Depth-first search for the first property named `name` anywhere in the
/// tree. Returns the setValue-addressable path segments leading to it
/// (inclusive) plus the property. Map entries whose keys cannot be rendered as
/// path segments are skipped (a hit behind such a key would not be
/// addressable anyway).
pub fn find_property_by_name<'a>(
    root: &'a RootObject,
    name: &str,
) -> Option<(Vec<String>, &'a Property)> {
    find_path_in_properties(&root.properties, name)
}

/// Like [`find_property_by_name`] but rooted at an arbitrary property slice
/// (e.g. one profile's properties inside `m_Profiles`). The returned path is
/// RELATIVE to that slice. Used to scope a by-name lookup to a sub-tree.
pub fn find_path_in_properties<'a>(
    properties: &'a [Property],
    name: &str,
) -> Option<(Vec<String>, &'a Property)> {
    fn in_props<'a>(
        props: &'a [Property],
        name: &str,
        path: &mut Vec<String>,
    ) -> Option<&'a Property> {
        for p in props {
            path.push(p.name.to_string());
            if p.name == name {
                return Some(p);
            }
            if let Some(found) = in_value(&p.value, name, path) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    fn in_value<'a>(
        value: &'a PropertyValue,
        name: &str,
        path: &mut Vec<String>,
    ) -> Option<&'a Property> {
        match value {
            PropertyValue::Struct(StructValue::Properties(inner)) => in_props(inner, name, path),
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                in_props(&i.properties, name, path)
            }
            PropertyValue::Map { entries, .. } => {
                for (key, val) in entries {
                    let Some(key) = map_key_to_string(key) else {
                        continue;
                    };
                    path.push(format!("{{{key}}}"));
                    if let Some(found) = in_value(val, name, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                for (i, e) in elements.iter().enumerate() {
                    path.push(format!("[{i}]"));
                    if let Some(found) = in_value(e, name, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::ObjectInstances(objs) => {
                for (i, o) in objs.iter().enumerate() {
                    path.push(format!("[{i}]"));
                    if let Some(found) = in_props(&o.properties, name, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            _ => None,
        }
    }
    let mut path = Vec::new();
    let target = in_props(properties, name, &mut path)?;
    Some((path, target))
}

/// Fixed-size scalar replacement value for in-place typed patching.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarValue {
    Int(i32),
    UInt32(u32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    /// Plain one-byte ByteProperty only; the enum-as-FString form goes
    /// through `patch_string`.
    Byte(u8),
}

/// Patch a resolved property's value in place. Only fixed-size scalars are
/// supported — the payload length never changes, so all recorded offsets in
/// the parsed tree stay valid across consecutive patches.
pub fn patch_scalar(
    payload: &mut [u8],
    property: &Property,
    value: ScalarValue,
) -> Result<(), CoreError> {
    fn write(
        payload: &mut [u8],
        offset: usize,
        size: usize,
        bytes: &[u8],
    ) -> Result<(), CoreError> {
        if size != bytes.len() || offset + size > payload.len() {
            return Err(CoreError::Parse(format!(
                "patch target out of bounds: offset {offset}, size {size}"
            )));
        }
        payload[offset..offset + size].copy_from_slice(bytes);
        Ok(())
    }
    let mismatch = || {
        CoreError::InvalidRequest(format!(
            "value {value:?} does not match property type {}",
            property.type_name
        ))
    };
    match (property.type_name.as_str(), value) {
        ("IntProperty", ScalarValue::Int(v)) => write(
            payload,
            property.value_offset,
            property.value_size,
            &v.to_le_bytes(),
        ),
        ("UInt32Property", ScalarValue::UInt32(v)) => write(
            payload,
            property.value_offset,
            property.value_size,
            &v.to_le_bytes(),
        ),
        ("Int64Property", ScalarValue::Int64(v)) => write(
            payload,
            property.value_offset,
            property.value_size,
            &v.to_le_bytes(),
        ),
        ("FloatProperty", ScalarValue::Float(v)) => write(
            payload,
            property.value_offset,
            property.value_size,
            &v.to_le_bytes(),
        ),
        ("DoubleProperty", ScalarValue::Double(v)) => write(
            payload,
            property.value_offset,
            property.value_size,
            &v.to_le_bytes(),
        ),
        ("ByteProperty", ScalarValue::Byte(v)) => {
            // Only the plain one-byte form; enum-as-byte payloads are FStrings
            // and longer than one byte, which the size check below rejects.
            if !matches!(property.value, PropertyValue::Byte(_)) {
                return Err(mismatch());
            }
            write(payload, property.value_offset, property.value_size, &[v])
        }
        ("BoolProperty", ScalarValue::Bool(v)) => {
            // No payload; the value is tag_flags bit 0x10, one byte before the
            // recorded value offset.
            let flag_offset = property
                .value_offset
                .checked_sub(1)
                .ok_or_else(|| CoreError::Parse("bool tag offset underflow".to_string()))?;
            if flag_offset >= payload.len() {
                return Err(CoreError::Parse(
                    "bool tag offset out of bounds".to_string(),
                ));
            }
            if v {
                payload[flag_offset] |= TAG_FLAG_BOOL_TRUE;
            } else {
                payload[flag_offset] &= !TAG_FLAG_BOOL_TRUE;
            }
            Ok(())
        }
        _ => Err(mismatch()),
    }
}

/// Serialized FString payload for a replacement value, mirroring the formats
/// `Reader::fstring` accepts: empty => bare zero length, ASCII => 8-bit chars
/// with a NUL terminator, otherwise UTF-16LE with a negative character count.
pub(crate) fn encode_fstring_value(value: &str) -> Vec<u8> {
    if value.is_empty() {
        return 0i32.to_le_bytes().to_vec();
    }
    if value.is_ascii() {
        let mut out = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        return out;
    }
    let units: Vec<u16> = value.encode_utf16().collect();
    let count = -((units.len() + 1) as i32);
    let mut out = count.to_le_bytes().to_vec();
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out.extend_from_slice(&[0, 0]);
    out
}

/// True when a tagged property's whole value payload is a single FString that
/// [`patch_string`] can replace: Str/Name/Object/Class/Enum properties, plus the
/// enum-as-FString form of ByteProperty (the plain one-byte form is a scalar).
pub fn string_patchable(property: &Property) -> bool {
    match property.type_name.as_str() {
        "StrProperty" | "NameProperty" | "ObjectProperty" | "ClassProperty" | "EnumProperty" => {
            true
        }
        "ByteProperty" => matches!(property.value, PropertyValue::Enum(_)),
        _ => false,
    }
}

/// Replace a string-valued property's FString payload. The new bytes may
/// differ in length: the property's own tag size and every enclosing size
/// field in `enclosing_size_fields` (from [`resolve_chain`]) are adjusted by
/// the byte delta. All writes are validated before the first mutation, so a
/// failed patch leaves the payload untouched. Offsets recorded in the parsed
/// tree are stale after a successful patch — re-parse before further edits.
pub fn patch_string(
    payload: &mut Vec<u8>,
    target: &Property,
    enclosing_size_fields: &[usize],
    new_value: &str,
) -> Result<(), CoreError> {
    if !string_patchable(target) {
        return Err(CoreError::InvalidRequest(format!(
            "string value does not match property type {}",
            target.type_name
        )));
    }
    let value_end = target
        .value_offset
        .checked_add(target.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| {
            CoreError::Parse(format!(
                "patch target out of bounds: offset {}, size {}",
                target.value_offset, target.value_size
            ))
        })?;
    let encoded = encode_fstring_value(new_value);
    let new_size = u32::try_from(encoded.len())
        .map_err(|_| CoreError::InvalidRequest("replacement string too long".to_string()))?;
    let delta = encoded.len() as i64 - target.value_size as i64;

    // Compute every size-field rewrite up front; mutate only once all are valid.
    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 1);
    if target.value_offset < 5 {
        return Err(CoreError::Parse("string tag offset underflow".to_string()));
    }
    writes.push((target.size_field_offset(), new_size));
    for &offset in enclosing_size_fields {
        // Enclosing headers always precede the value they wrap, so they are
        // unaffected by the splice below.
        if offset + 4 > target.value_offset {
            return Err(CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} does not precede the patch target"
            )));
        }
        let old = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        let updated = u32::try_from(i64::from(old) + delta).map_err(|_| {
            CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} would leave the u32 range"
            ))
        })?;
        writes.push((offset, updated));
    }
    for (offset, value) in writes {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    payload.splice(target.value_offset..value_end, encoded);
    Ok(())
}

/// Replace a property's entire serialized value with `new_bytes` (any length),
/// fixing the property's own size field and every enclosing size field by the
/// length delta. The generic sibling of [`patch_string`]: no type check and no
/// FString framing — `new_bytes` must be a schema-valid serialized value body
/// for `target`'s type, which the caller proves with a re-parse afterwards.
pub fn patch_value_bytes(
    payload: &mut Vec<u8>,
    target: &Property,
    enclosing_size_fields: &[usize],
    new_bytes: &[u8],
) -> Result<(), CoreError> {
    let value_end = target
        .value_offset
        .checked_add(target.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| {
            CoreError::Parse(format!(
                "patch target out of bounds: offset {}, size {}",
                target.value_offset, target.value_size
            ))
        })?;
    let new_size = u32::try_from(new_bytes.len())
        .map_err(|_| CoreError::InvalidRequest("replacement value too long".to_string()))?;
    let delta = new_bytes.len() as i64 - target.value_size as i64;
    if target.value_offset < 5 {
        return Err(CoreError::Parse("value tag offset underflow".to_string()));
    }
    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 1);
    writes.push((target.size_field_offset(), new_size));
    for &offset in enclosing_size_fields {
        if offset + 4 > target.value_offset {
            return Err(CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} does not precede the patch target"
            )));
        }
        let old = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        let updated = u32::try_from(i64::from(old) + delta).map_err(|_| {
            CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} would leave the u32 range"
            ))
        })?;
        writes.push((offset, updated));
    }
    for (offset, value) in writes {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    payload.splice(target.value_offset..value_end, new_bytes.iter().copied());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Array,
    Set,
}

/// Byte layout of a Set/Array property's value: where the element-count field
/// sits and the absolute byte range of every element. Computed by re-reading
/// the container body, since the parsed tree does not record inline element
/// offsets.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerLayout {
    pub kind: ContainerKind,
    pub inner_type: String,
    /// Absolute offset of the u32 element-count field.
    pub count_offset: usize,
    pub count: usize,
    /// Absolute byte range of each element within the payload.
    pub element_ranges: Vec<core::ops::Range<usize>>,
}

pub fn container_layout(payload: &[u8], property: &Property) -> Result<ContainerLayout, CoreError> {
    let kind = match property.type_name.as_str() {
        "ArrayProperty" => ContainerKind::Array,
        "SetProperty" => ContainerKind::Set,
        other => {
            return Err(CoreError::InvalidRequest(format!(
                "container edits require an ArrayProperty or SetProperty target, got {other}"
            )));
        }
    };
    // Instanced-object arrays interleave full object streams; element-level
    // splicing is not supported for them.
    if matches!(property.value, PropertyValue::ObjectInstances(_)) {
        return Err(CoreError::UnsupportedEdit(
            "container edits do not support instanced-object arrays".to_string(),
        ));
    }
    let inner =
        property.descriptor.inner.as_deref().ok_or_else(|| {
            CoreError::Parse("container property missing inner descriptor".into())
        })?;
    let end = property
        .value_offset
        .checked_add(property.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| CoreError::Parse("container value out of bounds".to_string()))?;
    let mut r = Reader::new(&payload[property.value_offset..end], property.value_offset);
    if kind == ContainerKind::Set {
        let _num_to_remove = r.u32()?;
    }
    let count_offset = r.abs_pos();
    let count = r.u32()? as usize;
    let mut element_ranges = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        let start = r.abs_pos();
        read_inline_value(&mut r, inner, 0)?;
        element_ranges.push(start..r.abs_pos());
    }
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "container body left {} bytes after {count} elements",
            r.remaining()
        )));
    }
    Ok(ContainerLayout {
        kind,
        inner_type: inner.type_name.to_string(),
        count_offset,
        count,
        element_ranges,
    })
}

/// Byte layout of a native `GameplayTagContainer` StructProperty value: the
/// `u32` count-field offset, the parsed tags, and the absolute byte range of
/// every tag FString. Mirrors `container_layout`, but the value is serialized
/// natively (`u32 count` followed by `count` FStrings) rather than as a
/// generic container body. Used to splice individual tags in/out.
#[derive(Debug, Clone, PartialEq)]
pub struct TagContainerLayout {
    /// Absolute offset of the u32 tag-count field.
    pub count_offset: usize,
    pub count: usize,
    pub tags: Vec<String>,
    /// Absolute byte range of each tag FString within the payload.
    pub element_ranges: Vec<core::ops::Range<usize>>,
}

pub fn tag_container_layout(
    payload: &[u8],
    property: &Property,
) -> Result<TagContainerLayout, CoreError> {
    if property.type_name != "StructProperty" {
        return Err(CoreError::InvalidRequest(format!(
            "tag_container_layout requires a GameplayTagContainer StructProperty target, got {}",
            property.type_name
        )));
    }
    let struct_type = property
        .descriptor
        .struct_type
        .as_deref()
        .map(|(name, _)| name.as_str());
    if struct_type != Some("GameplayTagContainer") {
        return Err(CoreError::InvalidRequest(format!(
            "tag_container_layout requires a GameplayTagContainer struct, got {}",
            struct_type.unwrap_or("<unknown>")
        )));
    }
    let end = property
        .value_offset
        .checked_add(property.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| CoreError::Parse("tag container value out of bounds".to_string()))?;
    let mut r = Reader::new(&payload[property.value_offset..end], property.value_offset);
    let count_offset = r.abs_pos();
    let count = r.u32()? as usize;
    let mut tags = Vec::with_capacity(count.min(1 << 16));
    let mut element_ranges = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        let start = r.abs_pos();
        tags.push(r.fstring()?);
        element_ranges.push(start..r.abs_pos());
    }
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "tag container body left {} bytes after {count} tags",
            r.remaining()
        )));
    }
    Ok(TagContainerLayout {
        count_offset,
        count,
        tags,
        element_ranges,
    })
}

/// Byte layout of a MapProperty value: the count-field offset and the absolute
/// byte range of every (key+value) entry. Mirrors `container_layout` for maps,
/// which `container_layout` rejects (maps have inline key/value pairs).
#[derive(Debug, Clone, PartialEq)]
pub struct MapLayout {
    pub count_offset: usize,
    pub count: usize,
    pub entry_ranges: Vec<core::ops::Range<usize>>,
}

pub fn map_layout(payload: &[u8], property: &Property) -> Result<MapLayout, CoreError> {
    if property.type_name != "MapProperty" {
        return Err(CoreError::InvalidRequest(format!(
            "map_layout requires a MapProperty target, got {}",
            property.type_name
        )));
    }
    let (key, value) = property
        .descriptor
        .map
        .as_deref()
        .ok_or_else(|| CoreError::Parse("MapProperty missing descriptor".into()))?;
    let end = property
        .value_offset
        .checked_add(property.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| CoreError::Parse("map value out of bounds".to_string()))?;
    let mut r = Reader::new(&payload[property.value_offset..end], property.value_offset);
    let _num_to_remove = r.u32()?;
    let count_offset = r.abs_pos();
    let count = r.u32()? as usize;
    let mut entry_ranges = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        let start = r.abs_pos();
        read_inline_value(&mut r, key, 0)?;
        read_inline_value(&mut r, value, 0)?;
        entry_ranges.push(start..r.abs_pos());
    }
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "map body left {} bytes after {count} entries",
            r.remaining()
        )));
    }
    Ok(MapLayout {
        count_offset,
        count,
        entry_ranges,
    })
}

/// Structural container edit applied by `patch_container`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerEdit {
    /// Append a Name/Str element to a SetProperty (rejects duplicates).
    SetAdd(String),
    /// Remove a Name/Str element from a SetProperty by value.
    SetRemove(String),
    /// Remove an ArrayProperty element by index.
    ArrayRemove(usize),
    /// Remove several ArrayProperty elements in one splice pass. The indices are
    /// resolved against ONE layout, so a caller stripping many elements neither
    /// re-parses between them nor has to reason about how each removal shifts the
    /// next. Duplicates are ignored; order does not matter.
    ArrayRemoveMany(Vec<usize>),
    /// Duplicate an ArrayProperty element in place (copy inserted right after
    /// the source element).
    ArrayDuplicate(usize),
    /// Append raw element bytes to an ArrayProperty (works on an empty array).
    /// The bytes must be a single, schema-valid element for this array's inner
    /// type; the caller is responsible for that (it is validated by the
    /// re-parse the caller performs afterwards).
    ArrayInsertBytes(Vec<u8>),
    /// Append a pre-built (inline key ++ inline value) entry to a MapProperty.
    /// The bytes must be schema-valid for this map's key/value descriptors; the
    /// caller validates via the re-parse it performs afterwards.
    MapInsert { entry_bytes: Vec<u8> },
    /// Remove several MapProperty entries in one splice pass, like
    /// [`ContainerEdit::ArrayRemoveMany`].
    MapRemoveMany(Vec<usize>),
    /// Remove the (key+value) entry at `entry_index` from a MapProperty (the
    /// entry's whole byte range is spliced out, the count decremented). The index
    /// is into [`map_layout`]'s `entry_ranges` (entry order == on-disk order).
    MapRemove { entry_index: usize },
}

fn set_string_elements(target: &Property) -> Option<&[PropertyValue]> {
    match &target.value {
        PropertyValue::Set { elements, .. } => Some(elements),
        _ => None,
    }
}

/// Locate a string element in a set. `fold_case` follows UE FName semantics:
/// Name sets compare case-insensitively, Str sets hold regular strings where
/// case-only variants are distinct values.
fn set_element_position(elements: &[PropertyValue], value: &str, fold_case: bool) -> Option<usize> {
    elements.iter().position(|e| match e {
        PropertyValue::Name(s) | PropertyValue::Str(s) => {
            if fold_case {
                s.eq_ignore_ascii_case(value)
            } else {
                s == value
            }
        }
        _ => false,
    })
}

/// Apply a structural set/array edit to a resolved container property. The
/// element count, the property's own tag size, and every enclosing size field
/// (from [`resolve_chain`]) are adjusted by the byte delta; all writes are
/// validated before the first mutation, so a failed patch leaves the payload
/// untouched. Offsets recorded in the parsed tree are stale after a successful
/// patch — re-parse before further edits.
pub fn patch_container(
    payload: &mut Vec<u8>,
    target: &Property,
    enclosing_size_fields: &[usize],
    edit: &ContainerEdit,
) -> Result<(), CoreError> {
    // Map inserts are handled inline (key/value pairs); `container_layout`
    // rejects MapProperty, so resolve the Array/Set layout lazily and skip it
    // for the map path. The shared size-chain fixup below runs for both.
    let layout = match edit {
        ContainerEdit::MapInsert { .. }
        | ContainerEdit::MapRemove { .. }
        | ContainerEdit::MapRemoveMany(_) => None,
        _ => Some(container_layout(payload, target)?),
    };
    let require_kind = |wanted: ContainerKind, op: &str| {
        // Only reachable for Array/Set edits, where `layout` is always Some.
        let layout = layout.as_ref().expect("non-map edit resolves a layout");
        if layout.kind == wanted {
            Ok(())
        } else {
            Err(CoreError::InvalidRequest(format!(
                "{op} requires a {wanted:?} target, got {:?}",
                layout.kind
            )))
        }
    };
    // Each edit is one splice pass: either remove byte ranges or insert bytes at a
    // position. Removals are a LIST so an edit can drop several elements resolved
    // from one layout; they are spliced back to front below, which leaves every
    // range before the one being removed exactly where it was.
    let (remove_ranges, insert_at, insert_bytes, count_delta): (
        Vec<core::ops::Range<usize>>,
        usize,
        Vec<u8>,
        i64,
    ) = match edit {
        ContainerEdit::SetAdd(value) => {
            require_kind(ContainerKind::Set, "setAdd")?;
            let layout = layout.as_ref().expect("set edit resolves a layout");
            if !matches!(layout.inner_type.as_str(), "NameProperty" | "StrProperty") {
                return Err(CoreError::UnsupportedEdit(format!(
                    "setAdd supports Name/Str sets; this set holds {}",
                    layout.inner_type
                )));
            }
            let elements = set_string_elements(target)
                .ok_or_else(|| CoreError::Parse("set value not parsed as a set".into()))?;
            let fold_case = layout.inner_type == "NameProperty";
            if set_element_position(elements, value, fold_case).is_some() {
                return Err(CoreError::InvalidRequest(format!(
                    "set already contains {value:?}"
                )));
            }
            let end = target.value_offset + target.value_size;
            (Vec::new(), end, encode_fstring_value(value), 1)
        }
        ContainerEdit::SetRemove(value) => {
            require_kind(ContainerKind::Set, "setRemove")?;
            let layout = layout.as_ref().expect("set edit resolves a layout");
            let elements = set_string_elements(target)
                .ok_or_else(|| CoreError::Parse("set value not parsed as a set".into()))?;
            let fold_case = layout.inner_type == "NameProperty";
            let index = set_element_position(elements, value, fold_case)
                .ok_or_else(|| CoreError::Parse(format!("set does not contain {value:?}")))?;
            let range = layout.element_ranges[index].clone();
            (vec![range.clone()], range.start, Vec::new(), -1)
        }
        ContainerEdit::ArrayRemove(index) => {
            require_kind(ContainerKind::Array, "arrayRemove")?;
            let layout = layout.as_ref().expect("array edit resolves a layout");
            let range = layout.element_ranges.get(*index).cloned().ok_or_else(|| {
                CoreError::InvalidRequest(format!(
                    "array index {index} out of bounds ({} elements)",
                    layout.count
                ))
            })?;
            (vec![range.clone()], range.start, Vec::new(), -1)
        }
        ContainerEdit::ArrayDuplicate(index) => {
            require_kind(ContainerKind::Array, "arrayDuplicate")?;
            let layout = layout.as_ref().expect("array edit resolves a layout");
            let range = layout.element_ranges.get(*index).cloned().ok_or_else(|| {
                CoreError::InvalidRequest(format!(
                    "array index {index} out of bounds ({} elements)",
                    layout.count
                ))
            })?;
            let bytes = payload[range.clone()].to_vec();
            (Vec::new(), range.end, bytes, 1)
        }
        ContainerEdit::ArrayInsertBytes(bytes) => {
            require_kind(ContainerKind::Array, "arrayInsertBytes")?;
            // Append after the last element; for an empty array this is the end
            // of the value (right after the count u32).
            let end = target.value_offset + target.value_size;
            (Vec::new(), end, bytes.clone(), 1)
        }
        ContainerEdit::ArrayRemoveMany(indices) => {
            require_kind(ContainerKind::Array, "arrayRemoveMany")?;
            let layout = layout.as_ref().expect("array edit resolves a layout");
            let ranges = distinct_element_ranges(indices, &layout.element_ranges, layout.count)?;
            let start = ranges.first().map_or(target.value_offset, |range| range.start);
            let removed = ranges.len() as i64;
            (ranges, start, Vec::new(), -removed)
        }
        ContainerEdit::MapRemoveMany(indices) => {
            if target.type_name != "MapProperty" {
                return Err(CoreError::InvalidRequest(format!(
                    "mapRemoveMany requires a MapProperty target, got {}",
                    target.type_name
                )));
            }
            let map = map_layout(payload, target)?;
            let ranges = distinct_element_ranges(indices, &map.entry_ranges, map.count)?;
            let start = ranges.first().map_or(target.value_offset, |range| range.start);
            let removed = ranges.len() as i64;
            (ranges, start, Vec::new(), -removed)
        }
        ContainerEdit::MapInsert { entry_bytes } => {
            if target.type_name != "MapProperty" {
                return Err(CoreError::InvalidRequest(format!(
                    "mapInsert requires a MapProperty target, got {}",
                    target.type_name
                )));
            }
            let insert_at = target.value_offset + target.value_size; // end of map body
            (Vec::new(), insert_at, entry_bytes.clone(), 1)
        }
        ContainerEdit::MapRemove { entry_index } => {
            if target.type_name != "MapProperty" {
                return Err(CoreError::InvalidRequest(format!(
                    "mapRemove requires a MapProperty target, got {}",
                    target.type_name
                )));
            }
            let map = map_layout(payload, target)?;
            let range = map.entry_ranges.get(*entry_index).cloned().ok_or_else(|| {
                CoreError::InvalidRequest(format!(
                    "map entry index {entry_index} out of bounds ({} entries)",
                    map.count
                ))
            })?;
            (vec![range.clone()], range.start, Vec::new(), -1)
        }
    };
    // The entry/element count lives at `count_offset`; its current value comes
    // from the layout for Array/Set, or from `map_layout` for the map path.
    // Both are computed before any mutation (offsets stay valid until the
    // splice), preserving the "failed patch leaves payload untouched" rule.
    let (count, count_offset) = match edit {
        ContainerEdit::MapInsert { .. }
        | ContainerEdit::MapRemove { .. }
        | ContainerEdit::MapRemoveMany(_) => {
            let map = map_layout(payload, target)?;
            (map.count, map.count_offset)
        }
        _ => {
            let layout = layout.as_ref().expect("non-map edit resolves a layout");
            (layout.count, layout.count_offset)
        }
    };
    let removed: usize = remove_ranges.iter().map(|range| range.len()).sum();
    let delta = insert_bytes.len() as i64 - removed as i64;
    let new_count = u32::try_from(count as i64 + count_delta)
        .map_err(|_| CoreError::Parse("container count underflow".to_string()))?;
    let new_size = u32::try_from(target.value_size as i64 + delta)
        .map_err(|_| CoreError::Parse("container size would leave the u32 range".to_string()))?;

    // Compute every size-field rewrite up front; mutate only once all are
    // valid (same discipline as patch_string).
    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 2);
    if target.value_offset < 5 {
        return Err(CoreError::Parse(
            "container tag offset underflow".to_string(),
        ));
    }
    writes.push((target.size_field_offset(), new_size));
    for &offset in enclosing_size_fields {
        if offset + 4 > target.value_offset {
            return Err(CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} does not precede the patch target"
            )));
        }
        let old = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        let updated = u32::try_from(i64::from(old) + delta).map_err(|_| {
            CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} would leave the u32 range"
            ))
        })?;
        writes.push((offset, updated));
    }
    // The count field lives inside the value payload but always precedes the
    // splice position (elements follow the count), so writing it before the
    // splice is safe.
    writes.push((count_offset, new_count));
    for (offset, value) in writes {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    if remove_ranges.is_empty() {
        payload.splice(insert_at..insert_at, insert_bytes);
    } else {
        // Back to front: removing a later range cannot move an earlier one.
        for range in remove_ranges.into_iter().rev() {
            payload.splice(range, core::iter::empty());
        }
    }
    Ok(())
}

/// The byte ranges of `indices` within `ranges`, de-duplicated and sorted ascending
/// so a caller can splice them back to front.
fn distinct_element_ranges(
    indices: &[usize],
    ranges: &[core::ops::Range<usize>],
    count: usize,
) -> Result<Vec<core::ops::Range<usize>>, CoreError> {
    let mut wanted = indices.to_vec();
    wanted.sort_unstable();
    wanted.dedup();
    wanted
        .into_iter()
        .map(|index| {
            ranges.get(index).cloned().ok_or_else(|| {
                CoreError::InvalidRequest(format!(
                    "container index {index} out of bounds ({count} elements)"
                ))
            })
        })
        .collect()
}



/// Add or remove one tag in a native `GameplayTagContainer` value, applied by
/// [`patch_tag_container`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagEdit {
    /// Append a tag FString (rejects an already-present tag).
    Add(String),
    /// Remove a tag FString by value (errors if not present).
    Remove(String),
}

/// Add or remove a single tag in a native `GameplayTagContainer` StructProperty
/// value. The container's `u32` count, the struct's own tag size, and every
/// enclosing size field (from [`resolve_chain`]) are adjusted by the byte
/// delta; all writes are validated before the first mutation, so a failed patch
/// leaves the payload untouched. Offsets recorded in the parsed tree are stale
/// after a successful patch — re-parse before further edits.
pub fn patch_tag_container(
    payload: &mut Vec<u8>,
    target: &Property,
    enclosing_size_fields: &[usize],
    edit: &TagEdit,
) -> Result<(), CoreError> {
    let layout = tag_container_layout(payload, target)?;
    // Each edit is one splice: either remove a tag's byte range or insert tag
    // bytes at a position. `count_delta` is +1 or -1.
    let (remove_range, insert_at, insert_bytes, count_delta): (
        Option<core::ops::Range<usize>>,
        usize,
        Vec<u8>,
        i64,
    ) = match edit {
        TagEdit::Add(tag) => {
            if layout.tags.iter().any(|t| t == tag) {
                return Err(CoreError::InvalidRequest(format!(
                    "tag container already contains {tag:?}"
                )));
            }
            // Append after the last tag, i.e. at the end of the value.
            let end = target.value_offset + target.value_size;
            (None, end, encode_fstring_value(tag), 1)
        }
        TagEdit::Remove(tag) => {
            let index = layout.tags.iter().position(|t| t == tag).ok_or_else(|| {
                CoreError::Parse(format!("tag container does not contain {tag:?}"))
            })?;
            let range = layout.element_ranges[index].clone();
            (Some(range.clone()), range.start, Vec::new(), -1)
        }
    };
    let removed = remove_range.as_ref().map_or(0, |r| r.len());
    let delta = insert_bytes.len() as i64 - removed as i64;
    let new_count = u32::try_from(layout.count as i64 + count_delta)
        .map_err(|_| CoreError::Parse("tag container count underflow".to_string()))?;
    let new_size = u32::try_from(target.value_size as i64 + delta).map_err(|_| {
        CoreError::Parse("tag container size would leave the u32 range".to_string())
    })?;

    // Compute every size-field rewrite up front; mutate only once all are valid
    // (same discipline as patch_container).
    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 2);
    if target.value_offset < 5 {
        return Err(CoreError::Parse(
            "tag container tag offset underflow".to_string(),
        ));
    }
    writes.push((target.size_field_offset(), new_size));
    for &offset in enclosing_size_fields {
        if offset + 4 > target.value_offset {
            return Err(CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} does not precede the patch target"
            )));
        }
        let old = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        let updated = u32::try_from(i64::from(old) + delta).map_err(|_| {
            CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} would leave the u32 range"
            ))
        })?;
        writes.push((offset, updated));
    }
    // The count field lives inside the value payload but always precedes the
    // splice position (tags follow the count), so writing it before the splice
    // is safe.
    writes.push((layout.count_offset, new_count));
    for (offset, value) in writes {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    match remove_range {
        Some(range) => {
            payload.splice(range, core::iter::empty());
        }
        None => {
            payload.splice(insert_at..insert_at, insert_bytes);
        }
    }
    Ok(())
}

/// Remove a single tag from a native `GameplayTagContainer` that is the *value*
/// of an entry in a MapProperty (e.g. `LooseTagsByGlobalId[<GlobalId>]`).
///
/// A map-value tag container is serialized inline as `u32 count` + `count`
/// FStrings — it has NO `u32 size | u8 tag_flags` header of its own (unlike a
/// tagged StructProperty), so [`patch_tag_container`] cannot be used: that
/// primitive rewrites the (nonexistent here) per-value size field at
/// `value_offset - 5`, which for a map value would clobber the preceding key
/// bytes. This primitive instead splices the tag FString out of the value body
/// and adjusts the value's own count, the enclosing MapProperty's size field,
/// and every ancestor size field (from [`resolve_chain`]) by the byte delta.
///
/// `map_property` is the resolved `MapProperty`; `entry_index` indexes its
/// on-disk entries ([`map_layout`] order). No-op success if the tag is absent.
/// All writes are validated before the first mutation, so a failed patch leaves
/// the payload untouched. Offsets in the parsed tree are stale afterwards —
/// re-parse before further edits.
pub fn patch_map_value_tag_container(
    payload: &mut Vec<u8>,
    map_property: &Property,
    enclosing_size_fields: &[usize],
    entry_index: usize,
    tags: &[&str],
) -> Result<usize, CoreError> {
    if map_property.type_name != "MapProperty" {
        return Err(CoreError::InvalidRequest(format!(
            "patch_map_value_tag_container requires a MapProperty target, got {}",
            map_property.type_name
        )));
    }
    let (_key_desc, value_desc) = map_property
        .descriptor
        .map
        .as_deref()
        .ok_or_else(|| CoreError::Parse("MapProperty missing descriptor".into()))?;
    let value_is_tag_container = value_desc.type_name == "StructProperty"
        && value_desc
            .struct_type
            .as_deref()
            .map(|(name, _)| name.as_str())
            == Some("GameplayTagContainer");
    if !value_is_tag_container {
        return Err(CoreError::InvalidRequest(
            "map value is not a GameplayTagContainer".to_string(),
        ));
    }

    // Locate the entry's value sub-range: parse the key, the value starts after.
    let layout = map_layout(payload, map_property)?;
    let entry_range = layout
        .entry_ranges
        .get(entry_index)
        .cloned()
        .ok_or_else(|| {
            CoreError::InvalidRequest(format!(
                "map entry index {entry_index} out of bounds ({} entries)",
                layout.count
            ))
        })?;
    let (key_desc, _value_desc) = map_property
        .descriptor
        .map
        .as_deref()
        .expect("map descriptor present");
    let value_start = {
        let entry = &payload[entry_range.clone()];
        let mut r = Reader::new(entry, entry_range.start);
        read_inline_value(&mut r, key_desc, 0)?;
        r.abs_pos()
    };
    // The value body is the native tag container: u32 count + count FStrings.
    let count_offset = value_start;
    let (count, tag_ranges) = {
        let body = &payload[value_start..entry_range.end];
        let mut r = Reader::new(body, value_start);
        let count = r.u32()? as usize;
        let mut ranges = Vec::with_capacity(count.min(1 << 16));
        let mut tags = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            let start = r.abs_pos();
            tags.push(r.fstring()?);
            ranges.push((tags.last().cloned().unwrap(), start..r.abs_pos()));
        }
        (count, ranges)
    };
    // Every requested tag that is present, in container order, so they can be
    // spliced back to front from this one view of the container.
    let ranges: Vec<core::ops::Range<usize>> = tag_ranges
        .into_iter()
        .filter(|(name, _)| tags.contains(&name.as_str()))
        .map(|(_, range)| range)
        .collect();
    if ranges.is_empty() {
        return Ok(0); // none of them present => nothing to remove
    }
    let first_start = ranges[0].start;
    let delta = -(ranges.iter().map(|range| range.len()).sum::<usize>() as i64);
    let new_count = u32::try_from(count as i64 - ranges.len() as i64)
        .map_err(|_| CoreError::Parse("tag container count underflow".to_string()))?;

    // Compute every size-field rewrite up front; mutate only once all are valid.
    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 2);
    // The MapProperty's own size field shrinks too.
    writes.push((
        map_property.size_field_offset(),
        u32::try_from(map_property.value_size as i64 + delta)
            .map_err(|_| CoreError::Parse("map size would leave the u32 range".to_string()))?,
    ));
    for &offset in enclosing_size_fields {
        if offset + 4 > first_start {
            return Err(CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} does not precede the patch target"
            )));
        }
        let old = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        let updated = u32::try_from(i64::from(old) + delta).map_err(|_| {
            CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} would leave the u32 range"
            ))
        })?;
        writes.push((offset, updated));
    }
    // The count field precedes every spliced range, so writing it first is safe.
    writes.push((count_offset, new_count));
    for (offset, value) in writes {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let removed = ranges.len();
    // Back to front: removing a later tag cannot move an earlier one.
    for range in ranges.into_iter().rev() {
        payload.splice(range, core::iter::empty());
    }
    Ok(removed)
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PropertyCounts {
    pub total: usize,
    pub max_depth: usize,
}

/// Recursively count every property reachable through structs, instanced
/// structs, instanced object arrays, and container values, with max nesting depth.
pub fn count_properties(props: &[Property]) -> PropertyCounts {
    fn walk(props: &[Property], depth: usize, acc: &mut PropertyCounts) {
        acc.max_depth = acc.max_depth.max(depth);
        for p in props {
            acc.total += 1;
            walk_value(&p.value, depth, acc);
        }
    }
    fn walk_value(value: &PropertyValue, depth: usize, acc: &mut PropertyCounts) {
        match value {
            PropertyValue::Struct(StructValue::Properties(inner)) => walk(inner, depth + 1, acc),
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                walk(&i.properties, depth + 1, acc)
            }
            PropertyValue::ObjectInstances(objs) => {
                for o in objs {
                    walk(&o.properties, depth + 1, acc);
                }
            }
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                for e in elements {
                    walk_value(e, depth + 1, acc);
                }
            }
            PropertyValue::Map { entries, .. } => {
                for (k, v) in entries {
                    walk_value(k, depth + 1, acc);
                    walk_value(v, depth + 1, acc);
                }
            }
            _ => {}
        }
    }
    let mut acc = PropertyCounts::default();
    walk(props, 1, &mut acc);
    acc
}

thread_local! {
    /// How many whole-payload parses this thread has done.
    ///
    /// A parse costs about half a second on a real save and allocates a tree of a
    /// few hundred megabytes, so how often an edit re-parses is the number that
    /// decides how long a write takes — and unlike wall-clock time it is exactly
    /// reproducible. `crates/gore-save/examples/json_timer.rs` reports it beside the
    /// elapsed time. Counting costs one thread-local increment per parse.
    pub static PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Parse a full decompressed private payload (strict: every byte accounted for).
pub fn parse_private_root(payload: &[u8]) -> Result<RootObject, CoreError> {
    PARSE_COUNT.with(|c| c.set(c.get() + 1));
    parse_private_root_at(payload, 0)
}

/// Like [`parse_private_root`] but the object begins at absolute offset `start`
/// within `payload` (the rest of `payload` before `start` is a header the
/// object body does not include). Recorded offsets are ABSOLUTE within
/// `payload`, so `patch_string` / `patch_scalar` can splice directly into the
/// same buffer. `consumed` is the absolute end of the object.
pub fn parse_private_root_at(payload: &[u8], start: usize) -> Result<RootObject, CoreError> {
    if start > payload.len() {
        return Err(CoreError::Parse("object start past end of payload".into()));
    }
    // base_offset = start makes Reader::abs_pos() report offsets within the
    // whole `payload`, while it reads only the object slice.
    let mut r = Reader::new(&payload[start..], start);
    let root = read_object(&mut r, 0)?;
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "trailing bytes after root object: {} remaining",
            r.remaining()
        )));
    }
    Ok(root)
}

/// Parse a bare GVAS property list (no `class`/flag object framing, no footer)
/// that begins at offset 0 and is terminated by a `None` name. This is the
/// shape of a save's uncompressed PUBLIC payload (`SaveGamePublicData`), which
/// starts directly at the first property (e.g. `CustomPayload`) rather than at
/// an object header. The returned [`RootObject`] reuses the same property tree
/// (so `resolve_chain` / `patch_string` apply unchanged); `class` is empty,
/// `footer` is `0`, and `consumed` is the absolute end of the property list
/// including the closing `None`.
pub fn parse_property_list_root(payload: &[u8]) -> Result<RootObject, CoreError> {
    parse_property_list_root_at(payload, 0)
}

/// Like [`parse_property_list_root`] but the property list begins at absolute
/// offset `start` within `payload` (the bytes before `start` are a header the
/// list does not include). Recorded offsets are ABSOLUTE within `payload`. This
/// is the shape of a standard GVAS save-game file, whose variable-length header
/// ends with the save-game class name and is followed DIRECTLY by the object's
/// property list (no nested `class`/flag object framing, no footer).
pub fn parse_property_list_root_at(payload: &[u8], start: usize) -> Result<RootObject, CoreError> {
    if start > payload.len() {
        return Err(CoreError::Parse("list start past end of payload".into()));
    }
    let mut r = Reader::new(&payload[start..], start);
    let properties = read_property_list(&mut r, 0)?;
    Ok(RootObject {
        class: String::new(),
        flag: 0,
        properties,
        footer: 0,
        consumed: r.abs_pos(),
    })
}

fn read_object(r: &mut Reader, depth: usize) -> Result<RootObject, CoreError> {
    let class = r.fstring()?;
    let flag = r.u8()?;
    let properties = read_property_list(r, depth)?;
    let footer = r.u32()?;
    Ok(RootObject {
        class,
        flag,
        properties,
        footer,
        consumed: r.abs_pos(),
    })
}

pub(crate) fn read_property_list(r: &mut Reader, depth: usize) -> Result<Vec<Property>, CoreError> {
    if depth > MAX_DEPTH {
        return Err(CoreError::Parse(format!(
            "property nesting exceeds {MAX_DEPTH}"
        )));
    }
    let mut out = Vec::new();
    loop {
        let name = r.prop_str()?;
        if name == "None" {
            return Ok(out);
        }
        let type_name = r.prop_str()?;
        out.push(read_property(r, name, type_name, depth)?);
    }
}

fn read_property(
    r: &mut Reader,
    name: PropStr,
    type_name: PropStr,
    depth: usize,
) -> Result<Property, CoreError> {
    let descriptor = read_descriptor(r, &type_name)?;
    let array_index = r.u32()?;
    let size = r.u32()? as usize;
    let tag_flags = r.u8()?;
    let value_offset = r.abs_pos();

    if type_name == "BoolProperty" {
        // Bool carries no payload; value lives in tag_flags bit 0x10 and the
        // u32 read as `size` above is the bool's (always zero) size field.
        return Ok(Property {
            name,
            type_name,
            descriptor,
            array_index,
            tag_flags,
            value_offset,
            value_size: 0,
            value: PropertyValue::Bool(tag_flags & TAG_FLAG_BOOL_TRUE != 0),
        });
    }

    let body = r.read(size)?;
    let mut sub = Reader::new(body, value_offset);
    let value = read_sized_value(&mut sub, &type_name, &descriptor, tag_flags, depth)?;
    if sub.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "property {name} ({type_name}) left {} of {} payload bytes at 0x{:x}",
            sub.remaining(),
            size,
            sub.abs_pos(),
        )));
    }
    Ok(Property {
        name,
        type_name,
        descriptor,
        array_index,
        tag_flags,
        value_offset,
        value_size: size,
        value,
    })
}

fn read_descriptor(r: &mut Reader, type_name: &str) -> Result<Descriptor, CoreError> {
    let mut descriptor = Descriptor::default();
    match type_name {
        "StructProperty" => {
            descriptor.struct_type = Some(Box::new(read_struct_descriptor(r)?));
        }
        "EnumProperty" => {
            descriptor.enum_type = Some(Box::new(read_enum_descriptor(r)?));
        }
        "ArrayProperty" | "SetProperty" => {
            descriptor.inner = Some(Box::new(read_inner_descriptor(r)?));
        }
        "MapProperty" => {
            let _count = r.u32()?;
            let key = read_inner_descriptor_body(r)?;
            // key_flags u32 separates the key descriptor from the value type.
            let _key_flags = r.u32()?;
            let value = read_inner_descriptor_body(r)?;
            descriptor.map = Some(Box::new((key, value)));
        }
        _ => {}
    }
    Ok(descriptor)
}

fn read_struct_descriptor(r: &mut Reader) -> Result<(PropStr, PropStr), CoreError> {
    let _count = r.u32()?;
    let struct_type = r.prop_str()?;
    let _package_count = r.u32()?;
    let package = r.prop_str()?;
    Ok((struct_type, package))
}

fn read_enum_descriptor(r: &mut Reader) -> Result<(PropStr, PropStr, PropStr), CoreError> {
    let _count = r.u32()?;
    let enum_type = r.prop_str()?;
    let _package_count = r.u32()?;
    let package = r.prop_str()?;
    let _underlying_count = r.u32()?;
    let underlying = r.prop_str()?;
    Ok((enum_type, package, underlying))
}

fn read_inner_descriptor(r: &mut Reader) -> Result<InnerDescriptor, CoreError> {
    let _count = r.u32()?;
    read_inner_descriptor_body(r)
}

fn read_inner_descriptor_body(r: &mut Reader) -> Result<InnerDescriptor, CoreError> {
    let type_name = r.prop_str()?;
    let mut inner = InnerDescriptor {
        type_name: type_name.clone(),
        struct_type: None,
        enum_type: None,
    };
    match type_name.as_str() {
        "StructProperty" => inner.struct_type = Some(Box::new(read_struct_descriptor(r)?)),
        "EnumProperty" => inner.enum_type = Some(Box::new(read_enum_descriptor(r)?)),
        _ => {}
    }
    Ok(inner)
}

fn read_sized_value(
    r: &mut Reader,
    type_name: &str,
    descriptor: &Descriptor,
    tag_flags: u8,
    depth: usize,
) -> Result<PropertyValue, CoreError> {
    match type_name {
        "IntProperty" => Ok(PropertyValue::Int(r.i32()?)),
        "UInt32Property" => Ok(PropertyValue::UInt32(r.u32()?)),
        "Int64Property" => Ok(PropertyValue::Int64(r.i64()?)),
        "FloatProperty" => Ok(PropertyValue::Float(r.f32()?)),
        "DoubleProperty" => Ok(PropertyValue::Double(r.f64()?)),
        "ByteProperty" => {
            // enum-as-byte serializes as FString; plain byte as u8
            if r.remaining() == 1 {
                Ok(PropertyValue::Byte(r.u8()?))
            } else {
                Ok(PropertyValue::Enum(r.fstring()?))
            }
        }
        "StrProperty" => Ok(PropertyValue::Str(r.fstring()?)),
        "NameProperty" => Ok(PropertyValue::Name(r.fstring()?)),
        "ObjectProperty" | "ClassProperty" => Ok(PropertyValue::Object(r.fstring()?)),
        "EnumProperty" => Ok(PropertyValue::Enum(r.fstring()?)),
        "SoftObjectProperty" => Ok(PropertyValue::SoftObject(read_soft_object_path(r)?)),
        // FFieldPath (TArray<FName> + owner) has no scalar/string editing story,
        // so keep its payload opaque like TextProperty rather than aborting the
        // whole typed parse. Saves carrying one would otherwise fail every
        // typed-parse-gated tab (All data, Progression, typed editing).
        "TextProperty" | "FieldPathProperty" => {
            Ok(PropertyValue::Opaque(r.read(r.remaining())?.to_vec()))
        }
        "StructProperty" => {
            let (struct_type, _) = descriptor
                .struct_type
                .as_deref()
                .ok_or_else(|| CoreError::Parse("StructProperty missing descriptor".into()))?;
            Ok(PropertyValue::Struct(read_struct_value(
                r,
                struct_type,
                tag_flags & TAG_FLAG_NATIVE_SERIALIZE != 0,
                depth,
            )?))
        }
        "ArrayProperty" => {
            let inner = descriptor
                .inner
                .as_ref()
                .ok_or_else(|| CoreError::Parse("ArrayProperty missing descriptor".into()))?;
            read_array_value(r, inner, depth)
        }
        "SetProperty" => {
            let inner = descriptor
                .inner
                .as_ref()
                .ok_or_else(|| CoreError::Parse("SetProperty missing descriptor".into()))?;
            let num_to_remove = r.u32()?;
            let count = r.u32()? as usize;
            let mut elements = Vec::with_capacity(count.min(1 << 16));
            for _ in 0..count {
                elements.push(read_inline_value(r, inner, depth)?);
            }
            Ok(PropertyValue::Set {
                num_to_remove,
                elements,
            })
        }
        "MapProperty" => {
            let (key, value) = descriptor
                .map
                .as_deref()
                .ok_or_else(|| CoreError::Parse("MapProperty missing descriptor".into()))?;
            let num_to_remove = r.u32()?;
            let count = r.u32()? as usize;
            let mut entries = Vec::with_capacity(count.min(1 << 16));
            for _ in 0..count {
                let k = read_inline_value(r, key, depth)?;
                let v = read_inline_value(r, value, depth)?;
                entries.push((k, v));
            }
            Ok(PropertyValue::Map {
                num_to_remove,
                entries,
            })
        }
        // Any leaf type we don't model: keep its sized payload opaque (read-only,
        // round-tripped unchanged) rather than aborting. read_property bounds the
        // sub-reader to exactly `size` bytes, so this consumes the whole payload
        // byte-exact. Aborting here would fail the entire typed parse and gate off
        // every typed-parse tab (All data, Progression, Inventory add/remove,
        // Player editing) — the breakage the Text/FieldPath opaque arms each fixed
        // one type at a time. Container types (Struct/Array/Set/Map/Enum) are
        // matched above and still error if their descriptor is missing, so a real
        // stream desync on a known type surfaces rather than being swallowed.
        _ => Ok(PropertyValue::Opaque(r.read(r.remaining())?.to_vec())),
    }
}

fn read_soft_object_path(r: &mut Reader) -> Result<SoftObjectPath, CoreError> {
    Ok(SoftObjectPath {
        package_name: r.fstring()?,
        asset_name: r.fstring()?,
        sub_path: r.fstring()?,
    })
}

/// Inline (headerless) value inside an array/set/map body.
fn read_inline_value(
    r: &mut Reader,
    inner: &InnerDescriptor,
    depth: usize,
) -> Result<PropertyValue, CoreError> {
    match inner.type_name.as_str() {
        "IntProperty" => Ok(PropertyValue::Int(r.i32()?)),
        "UInt32Property" => Ok(PropertyValue::UInt32(r.u32()?)),
        "Int64Property" => Ok(PropertyValue::Int64(r.i64()?)),
        "FloatProperty" => Ok(PropertyValue::Float(r.f32()?)),
        "DoubleProperty" => Ok(PropertyValue::Double(r.f64()?)),
        "BoolProperty" => Ok(PropertyValue::Bool(r.u8()? != 0)),
        "ByteProperty" => Ok(PropertyValue::Byte(r.u8()?)),
        "StrProperty" => Ok(PropertyValue::Str(r.fstring()?)),
        "NameProperty" => Ok(PropertyValue::Name(r.fstring()?)),
        "ObjectProperty" => Ok(PropertyValue::Object(r.fstring()?)),
        "EnumProperty" => Ok(PropertyValue::Enum(r.fstring()?)),
        "SoftObjectProperty" => Ok(PropertyValue::SoftObject(read_soft_object_path(r)?)),
        "StructProperty" => {
            let (struct_type, _) = inner
                .struct_type
                .as_deref()
                .ok_or_else(|| CoreError::Parse("inline struct missing descriptor".into()))?;
            // Inline structs carry no tag_flags; decide native-vs-proplist by type.
            Ok(PropertyValue::Struct(read_struct_value(
                r,
                struct_type,
                is_native_struct_type(struct_type),
                depth,
            )?))
        }
        other => Err(CoreError::Parse(format!(
            "unsupported inline value type {other:?} at 0x{:x}",
            r.abs_pos()
        ))),
    }
}

/// Native-serialize struct types observed with tag_flags 0x08. Used for inline
/// container elements, which carry no per-element flags.
fn is_native_struct_type(struct_type: &str) -> bool {
    matches!(
        struct_type,
        "Vector"
            | "Rotator"
            | "Quat"
            | "Vector4"
            | "Vector2D"
            | "Guid"
            | "DateTime"
            | "GameplayTagContainer"
            | "InstancedStruct"
    )
}

fn read_struct_value(
    r: &mut Reader,
    struct_type: &str,
    native: bool,
    depth: usize,
) -> Result<StructValue, CoreError> {
    if !native {
        return Ok(StructValue::Properties(read_property_list(r, depth + 1)?));
    }
    match struct_type {
        "Vector" | "Rotator" => {
            if r.remaining() >= 24 {
                Ok(StructValue::Vector3 {
                    x: r.f64()?,
                    y: r.f64()?,
                    z: r.f64()?,
                })
            } else {
                Ok(StructValue::Vector3f {
                    x: r.f32()?,
                    y: r.f32()?,
                    z: r.f32()?,
                })
            }
        }
        "Quat" | "Vector4" => Ok(StructValue::Vector4 {
            x: r.f64()?,
            y: r.f64()?,
            z: r.f64()?,
            w: r.f64()?,
        }),
        "Vector2D" => Ok(StructValue::Vector2 {
            x: r.f64()?,
            y: r.f64()?,
        }),
        "Guid" => {
            let mut raw = [0u8; 16];
            raw.copy_from_slice(r.read(16)?);
            Ok(StructValue::Guid(raw))
        }
        "DateTime" => Ok(StructValue::DateTime(r.i64()?)),
        "GameplayTagContainer" => {
            let count = r.u32()? as usize;
            let mut tags = Vec::with_capacity(count.min(1 << 16));
            for _ in 0..count {
                tags.push(r.fstring()?);
            }
            Ok(StructValue::GameplayTagContainer(tags))
        }
        "InstancedStruct" => {
            let actual_type = r.prop_str()?;
            let data_size_offset = r.abs_pos();
            let data_size = r.u32()? as usize;
            let body_base = r.abs_pos();
            let body = r.read(data_size)?;
            if data_size == 0 {
                return Ok(StructValue::Instanced(None));
            }
            let mut sub = Reader::new(body, body_base);
            let properties = read_property_list(&mut sub, depth + 1)?;
            if sub.remaining() != 0 {
                return Err(CoreError::Parse(format!(
                    "InstancedStruct {actual_type} left {} of {data_size} bytes",
                    sub.remaining()
                )));
            }
            Ok(StructValue::Instanced(Some(InstancedStruct {
                actual_type,
                data_size_offset,
                properties,
            })))
        }
        other => Err(CoreError::Parse(format!(
            "native struct {other:?} has no decoder"
        ))),
    }
}

/// ArrayProperty body. Object arrays come in two shapes (bare paths vs inline
/// instanced objects); try plain first and fall back, exactly like the proven
/// Python validator.
fn read_array_value(
    r: &mut Reader,
    inner: &InnerDescriptor,
    depth: usize,
) -> Result<PropertyValue, CoreError> {
    let body_base = r.abs_pos();
    let body = r.read(r.remaining())?;
    let mut plain = Reader::new(body, body_base);
    let plain_result = (|| -> Result<Vec<PropertyValue>, CoreError> {
        let count = plain.u32()? as usize;
        let mut elements = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            elements.push(read_inline_value(&mut plain, inner, depth)?);
        }
        if plain.remaining() != 0 {
            return Err(CoreError::Parse(format!(
                "array left {} bytes",
                plain.remaining()
            )));
        }
        Ok(elements)
    })();
    match plain_result {
        Ok(elements) => Ok(PropertyValue::Array { elements }),
        Err(plain_err) => {
            if inner.type_name != "ObjectProperty" {
                return Err(plain_err);
            }
            let mut inst = Reader::new(body, body_base);
            let count = inst.u32()? as usize;
            let mut instances = Vec::with_capacity(count.min(1 << 16));
            for _ in 0..count {
                let class = inst.fstring()?;
                let flag = inst.u8()?;
                let properties = read_property_list(&mut inst, depth + 1)?;
                let footer = inst.u32()?;
                instances.push(ObjectInstance {
                    class,
                    flag,
                    properties,
                    footer,
                });
            }
            if inst.remaining() != 0 {
                return Err(CoreError::Parse(format!(
                    "instanced object array left {} bytes",
                    inst.remaining()
                )));
            }
            Ok(PropertyValue::ObjectInstances(instances))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names being hashed come out of the save file, so the table has to be
    /// keyed by something the file's author cannot know: without that, collisions
    /// can be worked out in advance and a crafted save turns every lookup of a
    /// parse into a linear scan. The seed is what supplies that, so this pins that
    /// it genuinely reaches the hash rather than sitting unused beside it.
    #[test]
    fn the_name_hash_depends_on_the_per_process_seed() {
        use std::hash::{BuildHasher, Hash, Hasher};

        fn hash_with(seed: u64, text: &str) -> u64 {
            let mut hasher = NameHasherBuilder(seed).build_hasher();
            text.hash(&mut hasher);
            hasher.finish()
        }

        let name = "GameplayTagContainer";
        assert_eq!(
            hash_with(1, name),
            hash_with(1, name),
            "the same seed has to keep hashing a name the same way"
        );
        assert_ne!(
            hash_with(1, name),
            hash_with(2, name),
            "a different seed has to move the name somewhere else"
        );
        assert_ne!(
            name_hash_seed(),
            0,
            "the drawn seed has to be a real value, not a zero standing in for one"
        );
        assert_eq!(
            name_hash_seed(),
            name_hash_seed(),
            "the seed is drawn once, so the table cannot lose track of its own keys"
        );
    }

    /// The shared name table lives as long as the thread, so it must not be able to
    /// pin an unbounded amount of text: a name past the length limit is handed back
    /// like any other, but is not retained for sharing.
    #[test]
    fn the_name_table_does_not_retain_outsized_names() {
        let short = "IntProperty";
        assert_eq!(
            PropStr::new(short).as_str().as_ptr(),
            PropStr::new(short).as_str().as_ptr(),
            "an ordinary name is stored once and shared"
        );

        let outsized = "X".repeat(MAX_INTERNED_NAME_LEN + 1);
        assert_eq!(PropStr::new(&outsized).as_str(), outsized);
        assert_ne!(
            PropStr::new(&outsized).as_str().as_ptr(),
            PropStr::new(&outsized).as_str().as_ptr(),
            "an outsized name is not kept in the table"
        );
    }

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    fn tag(name: &str, type_name: &str) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring(type_name));
        out
    }

    fn header(size: u32, flags: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&size.to_le_bytes());
        out.push(flags);
        out
    }

    fn int_property(name: &str, value: i32) -> Vec<u8> {
        let mut out = tag(name, "IntProperty");
        out.extend_from_slice(&header(4, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn str_property(name: &str, value: &str) -> Vec<u8> {
        let payload = fstring(value);
        let mut out = tag(name, "StrProperty");
        out.extend_from_slice(&header(payload.len() as u32, 0));
        out.extend_from_slice(&payload);
        out
    }

    fn root(class: &str, props: &[u8]) -> Vec<u8> {
        let mut out = fstring(class);
        out.push(0); // object flag
        out.extend_from_slice(props);
        out.extend_from_slice(&fstring("None"));
        out.extend_from_slice(&0u32.to_le_bytes()); // footer
        out
    }

    /// A non-native (property-list) StructProperty named `name`, whose body is
    /// the given IntProperty members closed by the `None` terminator. Its
    /// header size field spans the whole member list including the terminator.
    fn struct_property(name: &str, members: &[(&str, i32)]) -> Vec<u8> {
        let mut struct_body = Vec::new();
        for (member, value) in members {
            struct_body.extend_from_slice(&int_property(member, *value));
        }
        struct_body.extend_from_slice(&fstring("None")); // close the property list

        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("InventoryData"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/Test"));
        out.extend_from_slice(&header(struct_body.len() as u32, 0));
        out.extend_from_slice(&struct_body);
        out
    }

    /// A root object holding just the StructProperty from [`struct_property`], so
    /// `resolve(root, [name])` yields it. Used as the byte donor whose full value
    /// body is lifted and spliced elsewhere.
    fn single_struct_property_payload(name: &str, members: &[(&str, i32)]) -> Vec<u8> {
        root("/Script/Test.Save", &struct_property(name, members))
    }

    /// Wrap `m_Inventory` inside a non-native "Parent" struct, followed by a
    /// trailing sibling `m_After` int, so patching `m_Inventory` exercises an
    /// enclosing size field (Parent's) AND shifts `m_After` unless every
    /// enclosing/sibling offset is fixed up. Modeled on
    /// `nested_tag_container_payload`. Layout:
    /// `Parent: StructProperty { m_Inventory: InventoryData, m_After: Int }`.
    fn parent_with_inventory_payload(members: &[(&str, i32)]) -> Vec<u8> {
        let mut struct_body = struct_property("m_Inventory", members);
        struct_body.extend_from_slice(&int_property("m_After", 9));
        struct_body.extend_from_slice(&fstring("None")); // close Parent's property list

        let mut props = tag("Parent", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("NPCData"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/Test"));
        props.extend_from_slice(&header(struct_body.len() as u32, 0));
        props.extend_from_slice(&struct_body);
        root("/Script/Test.Save", &props)
    }

    #[test]
    fn patch_value_bytes_replaces_struct_body_and_reparses() {
        // A nested StructProperty whose body we overwrite with a longer, still-
        // valid body lifted from a second parse of the same shape. Nesting under
        // "Parent" (and the trailing m_After sibling) forces the enclosing-size-
        // field fixup and sibling-offset consistency to be exercised.
        let mut payload = parent_with_inventory_payload(&[("m_Count", 1i32)]);
        let donor =
            single_struct_property_payload("m_Inventory", &[("m_Count", 7i32), ("m_Extra", 9i32)]);

        let donor_root = parse_private_root(&donor).unwrap();
        let donor_target = resolve(
            &donor_root.properties,
            &parse_path(&["m_Inventory".into()]).unwrap(),
        )
        .unwrap();
        let donor_bytes = donor
            [donor_target.value_offset..donor_target.value_offset + donor_target.value_size]
            .to_vec();

        let inventory_path = parse_path(&["Parent".into(), "m_Inventory".into()]).unwrap();
        let root = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&root.properties, &inventory_path).unwrap();
        // Lock in that the target is genuinely nested: the enclosing-size-field
        // fixup loop in patch_value_bytes must have work to do.
        assert!(!chain.enclosing_size_fields.is_empty());
        patch_value_bytes(
            &mut payload,
            chain.target,
            &chain.enclosing_size_fields,
            &donor_bytes,
        )
        .unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(reparsed.consumed, payload.len());
        // (a) the patched value now carries the donor body verbatim.
        let inv = resolve(&reparsed.properties, &inventory_path).unwrap();
        assert_eq!(inv.value_size, donor_bytes.len());
        // (b) the trailing sibling survives intact: a missed/wrong enclosing-size
        // fixup would shift m_After and break this resolve (or the re-parse).
        let after = resolve(
            &reparsed.properties,
            &parse_path(&["Parent".into(), "m_After".into()]).unwrap(),
        )
        .unwrap();
        assert_eq!(after.value, PropertyValue::Int(9));
    }

    #[test]
    fn parses_root_with_scalars() {
        let mut props = int_property("m_SaveVersionNumber", 17);
        props.extend_from_slice(&str_property("m_ProfileName", "0"));
        let payload = root("/Script/Angelscript.GothicFinalDataGame", &props);

        let parsed = parse_private_root(&payload).unwrap();
        assert_eq!(parsed.class, "/Script/Angelscript.GothicFinalDataGame");
        assert_eq!(parsed.properties.len(), 2);
        assert_eq!(parsed.properties[0].value, PropertyValue::Int(17));
        assert_eq!(
            parsed.properties[1].value,
            PropertyValue::Str("0".to_string())
        );
        assert_eq!(parsed.consumed, payload.len());
    }

    #[test]
    fn bool_value_lives_in_tag_flags() {
        let mut props = tag("bIsHero", "BoolProperty");
        props.extend_from_slice(&header(0, TAG_FLAG_BOOL_TRUE));
        let mut props2 = tag("bIsOrc", "BoolProperty");
        props2.extend_from_slice(&header(0, 0));
        props.extend_from_slice(&props2);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        assert_eq!(parsed.properties[0].value, PropertyValue::Bool(true));
        assert_eq!(parsed.properties[1].value, PropertyValue::Bool(false));
    }

    #[test]
    fn field_path_property_kept_opaque() {
        // FFieldPath has no scalar editing story; the typed parser must keep its
        // payload as opaque bytes (like TextProperty) instead of aborting the
        // whole save parse with "unsupported property type".
        let body = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut props = tag("MyFieldPath", "FieldPathProperty");
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            parsed.properties[0].value,
            PropertyValue::Opaque(vec![1, 2, 3, 4, 5, 6, 7, 8])
        );
        assert_eq!(parsed.properties[0].type_name, "FieldPathProperty");
    }

    #[test]
    fn unknown_leaf_property_kept_opaque() {
        // Any leaf property type the parser does not model must keep its sized
        // payload as opaque bytes instead of aborting the whole typed parse.
        // A single unmodelled type would otherwise leave privateTypedVerified
        // false and gate off every typed-parse tab (All data, Progression,
        // Inventory add/remove, Player editing) — the recurring breakage the
        // FieldPathProperty/TextProperty opaque arms were each added to fix.
        let body = vec![9u8, 8, 7, 6, 5];
        let mut props = tag("SomeFuture", "WeakObjectProperty");
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            parsed.properties[0].value,
            PropertyValue::Opaque(vec![9, 8, 7, 6, 5])
        );
        assert_eq!(parsed.properties[0].type_name, "WeakObjectProperty");
    }

    #[test]
    fn gameplay_tag_container_is_native() {
        let mut body = 2u32.to_le_bytes().to_vec();
        body.extend_from_slice(&fstring("Guild.Orc.Scout"));
        body.extend_from_slice(&fstring("Memory.Guild.Joined"));

        let mut props = tag("EventTags", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("GameplayTagContainer"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/GameplayTags"));
        props.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => {
                assert_eq!(
                    tags,
                    &vec![
                        "Guild.Orc.Scout".to_string(),
                        "Memory.Guild.Joined".to_string()
                    ]
                );
            }
            other => panic!("unexpected value {other:?}"),
        }
    }

    fn decode_fstring_at(payload: &[u8], offset: usize) -> String {
        Reader::new(&payload[offset..], offset).fstring().unwrap()
    }

    #[test]
    fn tag_container_layout_reports_count_and_ranges() {
        let mut body = 2u32.to_le_bytes().to_vec();
        body.extend_from_slice(&fstring("State.Dead"));
        body.extend_from_slice(&fstring("State.KillBountyGranted"));

        let mut props = tag("CapturedActorTags", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("GameplayTagContainer"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/GameplayTags"));
        props.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let property = &parsed.properties[0];

        let layout = tag_container_layout(&payload, property).unwrap();
        assert_eq!(layout.count, 2);
        assert_eq!(
            layout.tags,
            vec![
                "State.Dead".to_string(),
                "State.KillBountyGranted".to_string()
            ]
        );
        assert_eq!(layout.element_ranges.len(), 2);
        let second = &layout.element_ranges[1];
        assert_eq!(
            decode_fstring_at(&payload, second.start),
            "State.KillBountyGranted"
        );
    }

    /// Build a native `GameplayTagContainer` StructProperty body (`u32 count`
    /// followed by `count` FString tags), wrapped in the property tag.
    fn tag_container_property(name: &str, tags: &[&str]) -> Vec<u8> {
        let mut body = (tags.len() as u32).to_le_bytes().to_vec();
        for t in tags {
            body.extend_from_slice(&fstring(t));
        }
        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("GameplayTagContainer"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/GameplayTags"));
        out.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        out.extend_from_slice(&body);
        out
    }

    /// Wrap a tag container inside a non-native (property-list) parent
    /// StructProperty so the patch exercises an enclosing size field. Layout:
    /// `Parent: StructProperty { CapturedActorTags: GameplayTagContainer }`.
    fn nested_tag_container_payload(tags: &[&str]) -> Vec<u8> {
        let mut struct_body = tag_container_property("CapturedActorTags", tags);
        struct_body.extend_from_slice(&fstring("None")); // close the property list

        let mut props = tag("Parent", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("NPCData"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/Test"));
        props.extend_from_slice(&header(struct_body.len() as u32, 0));
        props.extend_from_slice(&struct_body);
        // A trailing root int: a missed size fixup shifts it and breaks re-parse.
        props.extend_from_slice(&int_property("m_After", 9));
        root("/Script/Test.Save", &props)
    }

    fn nested_tags_path() -> Vec<PathSeg> {
        parse_path(&["Parent".into(), "CapturedActorTags".into()]).unwrap()
    }

    /// Re-parse strictly and return the tag container's tags, asserting the
    /// trailing int survived (proof every enclosing size stayed consistent).
    fn reparse_nested_tags(payload: &[u8]) -> Vec<String> {
        let reparsed = parse_private_root(payload).unwrap();
        assert_eq!(reparsed.consumed, payload.len());
        let after = resolve(
            &reparsed.properties,
            &parse_path(&["m_After".into()]).unwrap(),
        )
        .unwrap();
        assert_eq!(after.value, PropertyValue::Int(9));
        let target = resolve(&reparsed.properties, &nested_tags_path()).unwrap();
        match &target.value {
            PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => tags.clone(),
            other => panic!("unexpected value {other:?}"),
        }
    }

    #[test]
    fn patch_tag_container_removes_tag_and_fixes_size_chain() {
        let mut payload = nested_tag_container_payload(&["State.Dead", "State.KillBountyGranted"]);
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &nested_tags_path()).unwrap();
        assert_eq!(chain.enclosing_size_fields.len(), 1);
        let target = chain.target.clone();
        patch_tag_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &TagEdit::Remove("State.Dead".to_string()),
        )
        .unwrap();

        assert_eq!(
            reparse_nested_tags(&payload),
            vec!["State.KillBountyGranted"]
        );
    }

    #[test]
    fn patch_tag_container_remove_missing_tag_errors_and_leaves_payload() {
        let mut payload = nested_tag_container_payload(&["State.Dead"]);
        let before = payload.clone();
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &nested_tags_path()).unwrap();
        let target = chain.target.clone();
        let err = patch_tag_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &TagEdit::Remove("State.KillBountyGranted".to_string()),
        );
        assert!(err.is_err());
        assert_eq!(payload, before);
    }

    #[test]
    fn patch_tag_container_adds_tag_and_fixes_size_chain() {
        let mut payload = nested_tag_container_payload(&["State.Dead"]);
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &nested_tags_path()).unwrap();
        let target = chain.target.clone();
        patch_tag_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &TagEdit::Add("State.KillBountyGranted".to_string()),
        )
        .unwrap();

        assert_eq!(
            reparse_nested_tags(&payload),
            vec!["State.Dead", "State.KillBountyGranted"]
        );
    }

    /// A `LooseTagsByGlobalId`-style MapProperty<Str, Struct(GameplayTagContainer)>:
    /// `entries` are `(id, &[tag])` and each value is an INLINE native tag container
    /// (`u32 count` + count FStrings), exactly as a struct-typed map value serializes
    /// (no per-value size header). Followed by a trailing root int to catch a missed
    /// size fixup.
    fn loose_tags_map_payload(entries: &[(&str, &[&str])]) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&(entries.len() as u32).to_le_bytes()); // count
        for (id, tags) in entries {
            map_body.extend_from_slice(&fstring(id));
            map_body.extend_from_slice(&(tags.len() as u32).to_le_bytes());
            for t in *tags {
                map_body.extend_from_slice(&fstring(t));
            }
        }
        let mut props = tag("LooseTagsByGlobalId", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("StrProperty")); // key type
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty")); // value type
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("GameplayTagContainer")); // value struct type
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/GameplayTags"));
        props.extend_from_slice(&header(map_body.len() as u32, 0));
        props.extend_from_slice(&map_body);
        props.extend_from_slice(&int_property("m_After", 9));
        root("/Script/Test.Save", &props)
    }

    fn reparse_loose_tags(payload: &[u8], id: &str) -> Vec<String> {
        let reparsed = parse_private_root(payload).unwrap();
        assert_eq!(reparsed.consumed, payload.len(), "byte-clean re-parse");
        let after = resolve(
            &reparsed.properties,
            &parse_path(&["m_After".into()]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            after.value,
            PropertyValue::Int(9),
            "trailing int survives size fixup"
        );
        let PropertyValue::Map { entries, .. } = &reparsed.properties[0].value else {
            panic!("LooseTagsByGlobalId not a map");
        };
        let (_k, v) = entries
            .iter()
            .find(|(k, _)| map_key_to_string(k).as_deref() == Some(id))
            .expect("entry present");
        match v {
            PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => tags.clone(),
            other => panic!("unexpected value {other:?}"),
        }
    }

    #[test]
    fn patch_map_value_tag_container_removes_tag_and_fixes_sizes() {
        // Two entries; remove State.Dead from the second, leaving the first untouched
        // and the second's other tags intact.
        let mut payload = loose_tags_map_payload(&[
            ("Npc-A", &["State.Aggro"]),
            (
                "Npc-B",
                &[
                    "State.KillBountyGranted",
                    "State.Dead",
                    "State.ExecutedBountyGranted",
                ],
            ),
        ]);
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(
            &parsed.properties,
            &parse_path(&["LooseTagsByGlobalId".into()]).unwrap(),
        )
        .unwrap();
        // Top-level map => no ancestor size fields (its own size field is handled
        // inside the primitive).
        assert!(chain.enclosing_size_fields.is_empty());
        let target = chain.target.clone();
        let removed = patch_map_value_tag_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            1, // Npc-B
            &["State.Dead"],
        )
        .unwrap();
        assert_eq!(removed, 1);

        assert_eq!(reparse_loose_tags(&payload, "Npc-A"), vec!["State.Aggro"]);
        assert_eq!(
            reparse_loose_tags(&payload, "Npc-B"),
            vec!["State.KillBountyGranted", "State.ExecutedBountyGranted"]
        );
    }

    #[test]
    fn patch_map_value_tag_container_missing_tag_is_noop_false() {
        let mut payload = loose_tags_map_payload(&[("Npc-A", &["State.Aggro"])]);
        let before = payload.clone();
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(
            &parsed.properties,
            &parse_path(&["LooseTagsByGlobalId".into()]).unwrap(),
        )
        .unwrap();
        let target = chain.target.clone();
        let removed = patch_map_value_tag_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            0,
            &["State.Dead"],
        )
        .unwrap();
        assert_eq!(removed, 0, "tag absent => no removal");
        assert_eq!(payload, before, "no-op leaves payload byte-identical");
    }

    #[test]
    fn gameplay_tag_standalone_is_property_list() {
        let mut body = tag("TagName", "NameProperty");
        let name_payload = fstring("CrimeLocation.OldCamp");
        body.extend_from_slice(&header(name_payload.len() as u32, 0));
        body.extend_from_slice(&name_payload);
        body.extend_from_slice(&fstring("None"));

        let mut props = tag("LocationTag", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("GameplayTag"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/GameplayTags"));
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::Struct(StructValue::Properties(inner)) => {
                assert_eq!(inner[0].name, "TagName");
                assert_eq!(
                    inner[0].value,
                    PropertyValue::Name("CrimeLocation.OldCamp".into())
                );
            }
            other => panic!("unexpected value {other:?}"),
        }
    }

    #[test]
    fn instanced_struct_roundtrips_nested_properties() {
        let nested = {
            let mut n = int_property("m_ItemCount", 5);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut body = fstring("/Script/G1R.ItemData");
        body.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        body.extend_from_slice(&nested);

        let mut props = tag("m_Profile", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::Struct(StructValue::Instanced(Some(instanced))) => {
                assert_eq!(instanced.actual_type, "/Script/G1R.ItemData");
                assert_eq!(instanced.properties[0].value, PropertyValue::Int(5));
            }
            other => panic!("unexpected value {other:?}"),
        }
    }

    #[test]
    fn empty_instanced_struct_has_no_terminator() {
        let mut body = fstring(""); // empty type: i32 0, no bytes
        body.extend_from_slice(&0u32.to_le_bytes()); // data_size 0

        let mut props = tag("Payload", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            parsed.properties[0].value,
            PropertyValue::Struct(StructValue::Instanced(None))
        );
    }

    #[test]
    fn map_and_set_have_num_to_remove() {
        // Map<StrProperty, IntProperty> with one entry
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("Gold"));
        map_body.extend_from_slice(&42i32.to_le_bytes());

        let mut props = tag("m_GenericData", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("StrProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("IntProperty"));
        props.extend_from_slice(&header(map_body.len() as u32, 0));
        props.extend_from_slice(&map_body);

        // Set<NameProperty> with two elements
        let mut set_body = 0u32.to_le_bytes().to_vec();
        set_body.extend_from_slice(&2u32.to_le_bytes());
        set_body.extend_from_slice(&fstring("Lock_A"));
        set_body.extend_from_slice(&fstring("Lock_B"));

        props.extend_from_slice(&tag("m_UnlockedLocks", "SetProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("NameProperty"));
        props.extend_from_slice(&header(set_body.len() as u32, 0));
        props.extend_from_slice(&set_body);

        let payload = root("/Script/Test.Save", &props);
        let parsed = parse_private_root(&payload).unwrap();

        match &parsed.properties[0].value {
            PropertyValue::Map {
                num_to_remove,
                entries,
            } => {
                assert_eq!(*num_to_remove, 0);
                assert_eq!(
                    entries[0],
                    (PropertyValue::Str("Gold".into()), PropertyValue::Int(42))
                );
            }
            other => panic!("unexpected map {other:?}"),
        }
        match &parsed.properties[1].value {
            PropertyValue::Set { elements, .. } => assert_eq!(elements.len(), 2),
            other => panic!("unexpected set {other:?}"),
        }
    }

    #[test]
    fn guid_keyed_map_reads_raw_guids() {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&[0xAA; 16]); // guid key
        body.extend_from_slice(&7i32.to_le_bytes()); // int value

        let mut props = tag("m_InteractiveData", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("Guid"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/CoreUObject"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("IntProperty"));
        props.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::Map { entries, .. } => {
                assert_eq!(
                    entries[0].0,
                    PropertyValue::Struct(StructValue::Guid([0xAA; 16]))
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn object_array_plain_paths() {
        let mut body = 2u32.to_le_bytes().to_vec();
        body.extend_from_slice(&fstring("/Script/A.B"));
        body.extend_from_slice(&fstring("/Script/C.D"));

        let mut props = tag("Owners", "ArrayProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("ObjectProperty"));
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::Array { elements } => assert_eq!(elements.len(), 2),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn object_array_instanced_objects() {
        // one element: path + flag + {m_ItemCount: 3} + footer
        let mut elem = fstring("/Script/Angelscript.AIValueSet_Guide");
        elem.push(0);
        elem.extend_from_slice(&int_property("m_ItemCount", 3));
        elem.extend_from_slice(&fstring("None"));
        elem.extend_from_slice(&0u32.to_le_bytes());

        let mut body = 1u32.to_le_bytes().to_vec();
        body.extend_from_slice(&elem);

        let mut props = tag("PersistantStorage", "ArrayProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("ObjectProperty"));
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::ObjectInstances(instances) => {
                assert_eq!(instances.len(), 1);
                assert_eq!(instances[0].class, "/Script/Angelscript.AIValueSet_Guide");
                assert_eq!(instances[0].properties[0].value, PropertyValue::Int(3));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn soft_object_path_has_three_strings() {
        let mut body = fstring("/Game/Maps/MainMap/MainMap");
        body.extend_from_slice(&fstring("MainMap"));
        body.extend_from_slice(&fstring("PersistentLevel.Region_0"));

        let mut props = tag("Region", "SoftObjectProperty");
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        match &parsed.properties[0].value {
            PropertyValue::SoftObject(path) => {
                assert_eq!(path.package_name, "/Game/Maps/MainMap/MainMap");
                assert_eq!(path.asset_name, "MainMap");
                assert_eq!(path.sub_path, "PersistentLevel.Region_0");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn rejects_trailing_garbage() {
        let payload = {
            let mut p = root("/Script/Test.Save", &int_property("x", 1));
            p.extend_from_slice(&[0xFF, 0xFF]);
            p
        };
        assert!(parse_private_root(&payload).is_err());
    }

    /// Strict full parse of real payload dumps. Run manually:
    /// `GORESAVE_PAYLOAD_BIN=work/decompressed/G1R-001.host.bin cargo test -p gore-save real_payload -- --ignored --nocapture`
    /// Accepts a single file or a `;`-separated list.
    #[test]
    #[ignore = "needs a local payload dump (GORESAVE_PAYLOAD_BIN)"]
    fn real_payload_parses_byte_exact() {
        let Ok(paths) = std::env::var("GORESAVE_PAYLOAD_BIN") else {
            panic!("set GORESAVE_PAYLOAD_BIN to a decompressed payload dump");
        };
        for path in paths.split(';').filter(|p| !p.is_empty()) {
            let payload = std::fs::read(path).unwrap();
            let parsed = parse_private_root(&payload).unwrap_or_else(|err| panic!("{path}: {err}"));
            assert_eq!(parsed.consumed, payload.len(), "{path}: incomplete parse");
            fn count(props: &[Property]) -> usize {
                props
                    .iter()
                    .map(|p| {
                        1 + match &p.value {
                            PropertyValue::Struct(StructValue::Properties(inner)) => count(inner),
                            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                                count(&i.properties)
                            }
                            PropertyValue::ObjectInstances(objs) => {
                                objs.iter().map(|o| count(&o.properties)).sum()
                            }
                            _ => 0,
                        }
                    })
                    .sum()
            }
            println!(
                "{path}: class={} bytes={} top_level_props={} reachable_props>={}",
                parsed.class,
                payload.len(),
                parsed.properties.len(),
                count(&parsed.properties),
            );
        }
    }

    /// Length-changing string patch against real payload dumps: grow the first
    /// addressable nested string by four bytes, then prove the strict re-parse
    /// still consumes every byte. Run manually like `real_payload_parses_byte_exact`.
    #[test]
    #[ignore = "needs a local payload dump (GORESAVE_PAYLOAD_BIN)"]
    fn real_payload_string_patch_reparses_byte_exact() {
        let Ok(paths) = std::env::var("GORESAVE_PAYLOAD_BIN") else {
            panic!("set GORESAVE_PAYLOAD_BIN to a decompressed payload dump");
        };
        for path in paths.split(';').filter(|p| !p.is_empty()) {
            let mut payload = std::fs::read(path).unwrap();
            let parsed = parse_private_root(&payload).unwrap();
            let (hits, _) = search_properties(&parsed, "", 0, 100_000);
            // Deepest addressable string = longest enclosing size chain.
            let Some(hit) = hits
                .iter()
                .filter(|h| {
                    h.editable && (h.type_name == "StrProperty" || h.type_name == "NameProperty")
                })
                .max_by_key(|h| h.path.len())
            else {
                println!("{path}: no addressable string property, skipping");
                continue;
            };
            let segs = parse_path(&hit.path).unwrap();
            let chain = resolve_chain(&parsed.properties, &segs).unwrap();
            let target = chain.target.clone();
            let new_value = format!("{}_ABC", hit.value_display);
            patch_string(
                &mut payload,
                &target,
                &chain.enclosing_size_fields,
                &new_value,
            )
            .unwrap();

            let reparsed = parse_private_root(&payload)
                .unwrap_or_else(|err| panic!("{path}: re-parse after string patch failed: {err}"));
            assert_eq!(
                reparsed.consumed,
                payload.len(),
                "{path}: incomplete re-parse"
            );
            let after = resolve(&reparsed.properties, &segs).unwrap();
            let value = match &after.value {
                PropertyValue::Str(s) | PropertyValue::Name(s) => s.clone(),
                other => panic!("{path}: unexpected value {other:?}"),
            };
            assert_eq!(value, new_value);
            println!(
                "{path}: patched {} ({} enclosing size fields) -> {new_value:?}",
                hit.display,
                chain.enclosing_size_fields.len(),
            );
        }
    }

    #[test]
    fn property_records_value_offsets_for_patching() {
        let props = int_property("m_ItemCount", 9);
        let payload = root("/Script/Test.Save", &props);
        let parsed = parse_private_root(&payload).unwrap();
        let p = &parsed.properties[0];
        assert_eq!(p.value_size, 4);
        let raw = &payload[p.value_offset..p.value_offset + 4];
        assert_eq!(i32::from_le_bytes(raw.try_into().unwrap()), 9);
    }

    /// Payload shaped like the real save: root → MapProperty<Str, InstancedStruct>
    /// → property list with an int and a bool.
    fn map_of_instanced_payload() -> Vec<u8> {
        let nested = {
            let mut n = int_property("m_ItemCount", 5);
            n.extend_from_slice(&tag("bLooted", "BoolProperty"));
            n.extend_from_slice(&header(0, 0)); // bool false
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/G1R.ChestData");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("ChestStates")); // key
        map_body.extend_from_slice(&instanced); // value: InstancedStruct inline

        let mut props = tag("m_GenericData", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("StrProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        root("/Script/Test.Save", &props)
    }

    fn float_property(name: &str, value: f32) -> Vec<u8> {
        let mut out = tag(name, "FloatProperty");
        out.extend_from_slice(&header(4, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn map_of_object_keyed_instanced_payload() -> Vec<u8> {
        let nested = {
            let mut n = float_property("BaseValue", 64.0);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/G1R.GameplayAttributeData");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("/Script/G1R.AttributeSet_Health")); // ObjectProperty key
        map_body.extend_from_slice(&instanced); // value: InstancedStruct inline

        let mut props = tag("AttributeSetsByClass", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("ObjectProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        root("/Script/Test.Save", &props)
    }

    #[test]
    fn search_addresses_object_map_keys() {
        let payload = map_of_object_keyed_instanced_payload();
        let root = parse_private_root(&payload).unwrap();
        let (hits, total) = search_properties(&root, "basevalue", 0, 100);
        assert_eq!(total, 1);
        let hit = &hits[0];
        assert_eq!(
            hit.path,
            vec![
                "AttributeSetsByClass",
                "{/Script/G1R.AttributeSet_Health}",
                "BaseValue"
            ]
        );
        assert_eq!(hit.type_name, "FloatProperty");
        assert!(hit.editable);
        // The search-built path must round-trip through resolve().
        let segs = parse_path(&hit.path).unwrap();
        assert_eq!(
            resolve(&root.properties, &segs).unwrap().value,
            PropertyValue::Float(64.0)
        );
    }

    #[test]
    fn exhaustive_browser_marks_unaddressable_and_duplicate_map_keys_read_only() {
        fn leaf(name: &str, value: i32) -> PropertyValue {
            PropertyValue::Struct(StructValue::Properties(vec![Property {
                name: name.into(),
                type_name: "IntProperty".into(),
                descriptor: Descriptor::default(),
                array_index: 0,
                tag_flags: 0,
                value_offset: 32,
                value_size: 4,
                value: PropertyValue::Int(value),
            }]))
        }
        let root = RootObject {
            class: "/Script/Test.Save".to_string(),
            flag: 0,
            properties: vec![Property {
                name: "Values".into(),
                type_name: "MapProperty".into(),
                descriptor: Descriptor::default(),
                array_index: 0,
                tag_flags: 0,
                value_offset: 16,
                value_size: 64,
                value: PropertyValue::Map {
                    num_to_remove: 0,
                    entries: vec![
                        (PropertyValue::Float(1.0), leaf("FloatKeyLeaf", 1)),
                        (
                            PropertyValue::Name("Same".to_string()),
                            leaf("DuplicateLeaf", 2),
                        ),
                        (
                            PropertyValue::Name("Same".to_string()),
                            leaf("DuplicateLeaf", 3),
                        ),
                    ],
                },
            }],
            footer: 0,
            consumed: 0,
        };
        let result = browse_properties(
            &root,
            &PropertyBrowseOptions {
                query: "Leaf",
                type_filter: None,
                kind_filter: None,
                editable_filter: None,
                offset: 0,
                limit: 100,
                allow_edits: true,
            },
        );
        let leaves = result
            .nodes
            .iter()
            .filter(|node| node.kind == "scalar")
            .collect::<Vec<_>>();
        assert_eq!(leaves.len(), 3);
        assert!(leaves.iter().all(|node| !node.editable));
        assert!(leaves.iter().all(|node| node.edit_value.is_none()));
        assert!(
            leaves
                .iter()
                .any(|node| node.path.iter().any(|segment| segment.contains("? #0")))
        );
        assert_eq!(
            leaves
                .iter()
                .filter(|node| node.display.contains("Same"))
                .count(),
            2
        );
        let (legacy, total) = search_properties(&root, "Leaf", 0, 100);
        assert_eq!(total, 3);
        assert!(legacy.iter().all(|hit| !hit.editable));
    }

    #[test]
    fn search_finds_nested_scalar_with_addressable_path() {
        let payload = map_of_instanced_payload();
        let root = parse_private_root(&payload).unwrap();
        let (hits, total) = search_properties(&root, "itemcount", 0, 100);
        assert_eq!(total, 1);
        assert_eq!(hits.len(), 1);
        let hit = &hits[0];
        assert_eq!(
            hit.path,
            vec!["m_GenericData", "{ChestStates}", "m_ItemCount"]
        );
        assert_eq!(hit.type_name, "IntProperty");
        assert_eq!(hit.value_display, "5");
        assert!(hit.editable);
        // path round-trips through resolve()
        let segs = parse_path(&hit.path).unwrap();
        assert_eq!(
            resolve(&root.properties, &segs).unwrap().value,
            PropertyValue::Int(5)
        );
    }

    #[test]
    fn search_empty_query_lists_all_and_truncates() {
        let payload = map_of_instanced_payload();
        let root = parse_private_root(&payload).unwrap();
        let (all, total) = search_properties(&root, "", 0, 100);
        // m_ItemCount + bLooted are the two leaf scalars
        assert_eq!(total, 2);
        assert_eq!(all.len(), 2);
        // page size 1 returns one entry but still reports the full total
        let (page0, total0) = search_properties(&root, "", 0, 1);
        assert_eq!(page0.len(), 1);
        assert_eq!(total0, 2);
        // second page returns the next entry, no overlap
        let (page1, _) = search_properties(&root, "", 1, 1);
        assert_eq!(page1.len(), 1);
        assert_ne!(page0[0].display, page1[0].display);
        // offset past the end yields an empty page with the real total
        let (empty, total_end) = search_properties(&root, "", 99, 10);
        assert!(empty.is_empty());
        assert_eq!(total_end, 2);
    }

    #[test]
    fn search_marks_strings_editable() {
        // root class string is not a property; build a payload with a StrProperty
        let mut props = str_property("m_ProfileName", "Hero");
        props.extend_from_slice(&int_property("m_Gold", 250));
        let payload = root("/Script/Test.Save", &props);
        let parsed = parse_private_root(&payload).unwrap();
        let (hits, _) = search_properties(&parsed, "m_", 0, 100);
        let name_hit = hits.iter().find(|h| h.display == "m_ProfileName").unwrap();
        assert!(name_hit.editable);
        let gold_hit = hits.iter().find(|h| h.display == "m_Gold").unwrap();
        assert!(gold_hit.editable);
    }

    #[test]
    fn search_keeps_container_element_strings_non_editable() {
        // Set<NameProperty> elements surface as hits whose path ends on an
        // `[index]` segment, which setValue cannot resolve — stay read-only.
        let mut set_body = 0u32.to_le_bytes().to_vec();
        set_body.extend_from_slice(&1u32.to_le_bytes());
        set_body.extend_from_slice(&fstring("Lock_A"));
        let mut props = tag("m_UnlockedLocks", "SetProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("NameProperty"));
        props.extend_from_slice(&header(set_body.len() as u32, 0));
        props.extend_from_slice(&set_body);
        let payload = root("/Script/Test.Save", &props);
        let parsed = parse_private_root(&payload).unwrap();
        let (hits, _) = search_properties(&parsed, "lock", 0, 100);
        let hit = hits.iter().find(|h| h.value_display == "Lock_A").unwrap();
        assert!(!hit.editable);
    }

    #[test]
    fn resolves_path_through_map_and_instanced_struct() {
        let payload = map_of_instanced_payload();
        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&[
            "m_GenericData".into(),
            "{ChestStates}".into(),
            "m_ItemCount".into(),
        ])
        .unwrap();
        let target = resolve(&parsed.properties, &path).unwrap();
        assert_eq!(target.value, PropertyValue::Int(5));
    }

    #[test]
    fn patch_scalar_updates_int_in_place() {
        let mut payload = map_of_instanced_payload();
        let original_len = payload.len();
        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&[
            "m_GenericData".into(),
            "{ChestStates}".into(),
            "m_ItemCount".into(),
        ])
        .unwrap();
        let target = resolve(&parsed.properties, &path).unwrap().clone();
        patch_scalar(&mut payload, &target, ScalarValue::Int(99)).unwrap();

        assert_eq!(payload.len(), original_len);
        let reparsed = parse_private_root(&payload).unwrap();
        let after = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(after.value, PropertyValue::Int(99));
    }

    #[test]
    fn patch_scalar_flips_bool_via_tag_flags() {
        let mut payload = map_of_instanced_payload();
        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&[
            "m_GenericData".into(),
            "{ChestStates}".into(),
            "bLooted".into(),
        ])
        .unwrap();
        let target = resolve(&parsed.properties, &path).unwrap().clone();
        assert_eq!(target.value, PropertyValue::Bool(false));
        patch_scalar(&mut payload, &target, ScalarValue::Bool(true)).unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        let after = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(after.value, PropertyValue::Bool(true));
    }

    #[test]
    fn patch_scalar_rejects_type_mismatch() {
        let mut payload = map_of_instanced_payload();
        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&[
            "m_GenericData".into(),
            "{ChestStates}".into(),
            "m_ItemCount".into(),
        ])
        .unwrap();
        let target = resolve(&parsed.properties, &path).unwrap().clone();
        assert!(patch_scalar(&mut payload, &target, ScalarValue::Float(1.0)).is_err());
    }

    #[test]
    fn resolve_rejects_unknown_key_and_truncated_paths() {
        let payload = map_of_instanced_payload();
        let parsed = parse_private_root(&payload).unwrap();
        let missing = parse_path(&["m_GenericData".into(), "{Nope}".into(), "x".into()]).unwrap();
        assert!(resolve(&parsed.properties, &missing).is_err());
        let dangling = parse_path(&["m_GenericData".into(), "{ChestStates}".into()]).unwrap();
        assert!(resolve(&parsed.properties, &dangling).is_err());
    }

    #[test]
    fn nested_offsets_are_absolute_through_instanced_struct() {
        let nested = {
            let mut n = int_property("m_ItemCount", 5);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut body = fstring("/Script/G1R.ItemData");
        body.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        body.extend_from_slice(&nested);

        let mut props = tag("m_Profile", "StructProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let PropertyValue::Struct(StructValue::Instanced(Some(instanced))) =
            &parsed.properties[0].value
        else {
            panic!("expected instanced struct");
        };
        let inner = &instanced.properties[0];
        // The recorded offset must index the WHOLE payload, not the inner body.
        let raw = &payload[inner.value_offset..inner.value_offset + inner.value_size];
        assert_eq!(i32::from_le_bytes(raw.try_into().unwrap()), 5);
    }

    /// Real-save shape with a string leaf: root → MapProperty<Str,
    /// InstancedStruct> → { m_PlayerName: Str, m_ItemCount: Int }, plus a
    /// root-level int after the map so a missed size-chain fixup shifts it
    /// and breaks the strict re-parse.
    fn nested_string_payload(player_name: &str) -> Vec<u8> {
        let nested = {
            let mut n = str_property("m_PlayerName", player_name);
            n.extend_from_slice(&int_property("m_ItemCount", 5));
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/G1R.PlayerCharacter");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("CharacterStates"));
        map_body.extend_from_slice(&instanced);

        let mut props = tag("m_GenericData", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("StrProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        props.extend_from_slice(&int_property("m_AfterMap", 7));
        root("/Script/Test.Save", &props)
    }

    fn player_name_path() -> Vec<PathSeg> {
        parse_path(&[
            "m_GenericData".into(),
            "{CharacterStates}".into(),
            "m_PlayerName".into(),
        ])
        .unwrap()
    }

    /// Re-parse strictly and assert the string took the new value while the
    /// nested int and the root-level property after the map stayed intact.
    fn assert_nested_string_patch(payload: &[u8], expected_name: &str) {
        let reparsed = parse_private_root(payload).unwrap();
        let name = resolve(&reparsed.properties, &player_name_path()).unwrap();
        assert_eq!(name.value, PropertyValue::Str(expected_name.to_string()));
        let count_path = parse_path(&[
            "m_GenericData".into(),
            "{CharacterStates}".into(),
            "m_ItemCount".into(),
        ])
        .unwrap();
        let count = resolve(&reparsed.properties, &count_path).unwrap();
        assert_eq!(count.value, PropertyValue::Int(5));
        let after = parse_path(&["m_AfterMap".into()]).unwrap();
        let after = resolve(&reparsed.properties, &after).unwrap();
        assert_eq!(after.value, PropertyValue::Int(7));
    }

    #[test]
    fn resolve_chain_collects_enclosing_size_fields() {
        let payload = nested_string_payload("Hero");
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &player_name_path()).unwrap();
        assert_eq!(chain.target.type_name, "StrProperty");

        let map_property = &parsed.properties[0];
        let PropertyValue::Map { entries, .. } = &map_property.value else {
            panic!("expected map");
        };
        let PropertyValue::Struct(StructValue::Instanced(Some(instanced))) = &entries[0].1 else {
            panic!("expected instanced struct");
        };
        // Outermost first: the map tag's size, then the instanced data_size.
        assert_eq!(
            chain.enclosing_size_fields,
            vec![map_property.size_field_offset(), instanced.data_size_offset]
        );
        // Both offsets index the size fields the parser consumed.
        let map_size = u32::from_le_bytes(
            payload[map_property.size_field_offset()..map_property.size_field_offset() + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(map_size as usize, map_property.value_size);
    }

    #[test]
    fn patch_string_grows_nested_string_and_fixes_size_chain() {
        let mut payload = nested_string_payload("Hero");
        let original_len = payload.len();
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &player_name_path()).unwrap();
        let target = chain.target.clone();
        patch_string(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            "Nameless",
        )
        .unwrap();

        assert_eq!(payload.len(), original_len + 4);
        assert_nested_string_patch(&payload, "Nameless");
    }

    #[test]
    fn patch_string_shrinks_nested_string_and_fixes_size_chain() {
        let mut payload = nested_string_payload("Hero");
        let original_len = payload.len();
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &player_name_path()).unwrap();
        let target = chain.target.clone();
        patch_string(&mut payload, &target, &chain.enclosing_size_fields, "Po").unwrap();

        assert_eq!(payload.len(), original_len - 2);
        assert_nested_string_patch(&payload, "Po");
    }

    /// Difficulty-shaped synthetic regression: an asset-path `ObjectProperty`
    /// (`m_customResourcesSettings`) nested inside an `InstancedStruct` inside a
    /// `MapProperty` — exactly the shape that corrupted real saves. Shrinking the
    /// asset path must update BOTH the InstancedStruct `data_size` and the Map
    /// tag size, and the tree must strictly re-parse (a stale enclosing size
    /// would desync the byte-exact parser, like the game's loader did).
    #[test]
    fn patch_object_difficulty_in_map_instanced_struct_fixes_size_chain() {
        // InstancedStruct body: one ObjectProperty asset path + None.
        let resources = fstring("/Script/Angelscript.ResourcesDifficultySettings_Standard");
        let nested = {
            let mut n = tag("m_customResourcesSettings", "ObjectProperty");
            n.extend_from_slice(&header(resources.len() as u32, 0));
            n.extend_from_slice(&resources);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/G1R.SaveDataPayload");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        // Map<Object, InstancedStruct> with one entry.
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("/Script/G1R.SaveDataPayload")); // Object key
        map_body.extend_from_slice(&instanced);

        let mut props = tag("CustomPayload", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes()); // descriptor count
        props.extend_from_slice(&fstring("ObjectProperty")); // key type
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty")); // value type
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        // Trailing root-level int: a missed size fixup shifts it and breaks parse.
        props.extend_from_slice(&int_property("m_AfterMap", 7));
        let mut payload = root("/Script/Test.Save", &props);
        let original_len = payload.len();

        let path = parse_path(&[
            "CustomPayload".into(),
            "{/Script/G1R.SaveDataPayload}".into(),
            "m_customResourcesSettings".into(),
        ])
        .unwrap();
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        // Two enclosing size fields: the Map tag size and the InstancedStruct
        // data_size (outermost first).
        assert_eq!(chain.enclosing_size_fields.len(), 2);
        let map_property = &parsed.properties[0];
        assert_eq!(
            chain.enclosing_size_fields[0],
            map_property.size_field_offset()
        );
        let target = chain.target.clone();
        let enclosing = chain.enclosing_size_fields.clone();

        // Shrink _Standard -> _Easy (-4 bytes).
        patch_string(
            &mut payload,
            &target,
            &enclosing,
            "/Script/Angelscript.ResourcesDifficultySettings_Easy",
        )
        .unwrap();
        assert_eq!(payload.len(), original_len - 4);

        // Strict re-parse must succeed and consume every byte.
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(reparsed.consumed, payload.len());
        let after = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(
            after.value,
            PropertyValue::Object("/Script/Angelscript.ResourcesDifficultySettings_Easy".into()),
        );
        // The trailing int survived (no desync).
        let after_int = resolve(
            &reparsed.properties,
            &parse_path(&["m_AfterMap".into()]).unwrap(),
        )
        .unwrap();
        assert_eq!(after_int.value, PropertyValue::Int(7));
    }

    #[test]
    fn patch_string_handles_same_length_and_name_property() {
        let nested = {
            let mut n = tag("m_QuestTag", "NameProperty");
            let value = fstring("Quest.A");
            n.extend_from_slice(&header(value.len() as u32, 0));
            n.extend_from_slice(&value);
            n
        };
        let mut payload = root("/Script/Test.Save", &nested);
        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&["m_QuestTag".into()]).unwrap();
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        assert!(chain.enclosing_size_fields.is_empty());
        let target = chain.target.clone();
        patch_string(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            "Quest.B",
        )
        .unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        let after = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(after.value, PropertyValue::Name("Quest.B".to_string()));
    }

    #[test]
    fn patch_string_writes_utf16_for_non_ascii() {
        let mut payload = nested_string_payload("Hero");
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &player_name_path()).unwrap();
        let target = chain.target.clone();
        patch_string(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            "Hörnchen",
        )
        .unwrap();

        assert_nested_string_patch(&payload, "Hörnchen");
        // The new payload is UTF-16: negative character count incl. terminator.
        let reparsed = parse_private_root(&payload).unwrap();
        let after = resolve(&reparsed.properties, &player_name_path()).unwrap();
        let count = i32::from_le_bytes(
            payload[after.value_offset..after.value_offset + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(count, -9);
        assert_eq!(after.value_size, 4 + 9 * 2);
    }

    #[test]
    fn patch_string_supports_empty_replacement() {
        let mut payload = nested_string_payload("Hero");
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &player_name_path()).unwrap();
        let target = chain.target.clone();
        patch_string(&mut payload, &target, &chain.enclosing_size_fields, "").unwrap();

        assert_nested_string_patch(&payload, "");
    }

    #[test]
    fn patch_string_rejects_non_string_target() {
        let mut payload = nested_string_payload("Hero");
        let original = payload.clone();
        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&[
            "m_GenericData".into(),
            "{CharacterStates}".into(),
            "m_ItemCount".into(),
        ])
        .unwrap();
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();
        let err = patch_string(&mut payload, &target, &chain.enclosing_size_fields, "oops");
        assert!(err.is_err());
        assert_eq!(
            payload, original,
            "failed patches must not mutate the payload"
        );
    }

    /// Tagged property whose value payload is a single FString.
    fn fstring_property(name: &str, type_name: &str, value: &str) -> Vec<u8> {
        let payload = fstring(value);
        let mut out = tag(name, type_name);
        out.extend_from_slice(&header(payload.len() as u32, 0));
        out.extend_from_slice(&payload);
        out
    }

    fn enum_property(name: &str, enum_type: &str, value: &str) -> Vec<u8> {
        let payload = fstring(value);
        let mut out = tag(name, "EnumProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(enum_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/Test"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("ByteProperty"));
        out.extend_from_slice(&header(payload.len() as u32, 0));
        out.extend_from_slice(&payload);
        out
    }

    fn plain_byte_property(name: &str, value: u8) -> Vec<u8> {
        let mut out = tag(name, "ByteProperty");
        out.extend_from_slice(&header(1, 0));
        out.push(value);
        out
    }

    #[test]
    fn patch_string_updates_object_and_enum_properties() {
        let mut props = fstring_property("m_Weapon", "ObjectProperty", "/Script/G1R.ItMw_Sword");
        props.extend_from_slice(&enum_property("m_Guild", "EGuild", "EGuild::Old"));
        props.extend_from_slice(&int_property("m_After", 7));
        let mut payload = root("/Script/Test.Save", &props);

        for (name, new_value) in [
            ("m_Weapon", "/Script/G1R.ItMw_Axe_TwoHanded"),
            ("m_Guild", "EGuild::None"),
        ] {
            let parsed = parse_private_root(&payload).unwrap();
            let path = parse_path(&[name.to_string()]).unwrap();
            let chain = resolve_chain(&parsed.properties, &path).unwrap();
            let target = chain.target.clone();
            patch_string(
                &mut payload,
                &target,
                &chain.enclosing_size_fields,
                new_value,
            )
            .unwrap();

            let reparsed = parse_private_root(&payload).unwrap();
            let after = resolve(&reparsed.properties, &path).unwrap();
            let read = match &after.value {
                PropertyValue::Object(s) | PropertyValue::Enum(s) => s.clone(),
                other => panic!("unexpected value {other:?}"),
            };
            assert_eq!(read, new_value);
            let after_int = resolve(
                &reparsed.properties,
                &parse_path(&["m_After".into()]).unwrap(),
            )
            .unwrap();
            assert_eq!(after_int.value, PropertyValue::Int(7));
        }
    }

    #[test]
    fn patch_string_updates_enum_as_byte_property() {
        // enum-as-byte: ByteProperty whose payload is an FString enum name
        let mut props = fstring_property("m_Rank", "ByteProperty", "ERank::Novice");
        props.extend_from_slice(&int_property("m_After", 7));
        let mut payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let path = parse_path(&["m_Rank".into()]).unwrap();
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        assert_eq!(
            chain.target.value,
            PropertyValue::Enum("ERank::Novice".into())
        );
        let target = chain.target.clone();
        patch_string(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            "ERank::Master",
        )
        .unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        let after = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(after.value, PropertyValue::Enum("ERank::Master".into()));
        let after_int = resolve(
            &reparsed.properties,
            &parse_path(&["m_After".into()]).unwrap(),
        )
        .unwrap();
        assert_eq!(after_int.value, PropertyValue::Int(7));
    }

    #[test]
    fn patch_scalar_updates_plain_byte_and_rejects_enum_form() {
        let mut props = plain_byte_property("m_Level", 3);
        props.extend_from_slice(&fstring_property("m_Rank", "ByteProperty", "ERank::Novice"));
        let mut payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let level = resolve(
            &parsed.properties,
            &parse_path(&["m_Level".into()]).unwrap(),
        )
        .unwrap()
        .clone();
        assert_eq!(level.value, PropertyValue::Byte(3));
        patch_scalar(&mut payload, &level, ScalarValue::Byte(42)).unwrap();
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(reparsed.properties[0].value, PropertyValue::Byte(42));

        // Scalar byte write on the enum-as-byte form must be rejected.
        let rank = resolve(
            &reparsed.properties,
            &parse_path(&["m_Rank".into()]).unwrap(),
        )
        .unwrap()
        .clone();
        assert!(patch_scalar(&mut payload, &rank, ScalarValue::Byte(1)).is_err());
        // And the plain-byte form must reject a string patch.
        assert!(patch_string(&mut payload, &level, &[], "oops").is_err());
    }

    fn name_set_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for v in values {
            body.extend_from_slice(&fstring(v));
        }
        let mut out = tag(name, "SetProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// Wrap a single tagged property in a parseable private-root payload.
    fn private_root_with_property(prop: &[u8]) -> Vec<u8> {
        root("/Script/Test.Save", prop)
    }

    /// A MapProperty<NameProperty, StructProperty(KnowledgeSet)> with `chars`
    /// entries, each an empty Knowledge set. Returns a full tagged property.
    fn knowledge_map_property(chars: &[&str]) -> Vec<u8> {
        let empty_value = || {
            // Inline struct value: a property list holding one (empty) Name set
            // named "Knowledge", terminated by "None".
            let mut v = name_set_property("Knowledge", &[]);
            v.extend_from_slice(&fstring("None"));
            v
        };
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(chars.len() as u32).to_le_bytes()); // count
        for c in chars {
            body.extend_from_slice(&fstring(c)); // inline Name key
            body.extend_from_slice(&empty_value()); // inline struct value
        }
        let mut out = tag("CharacterKnowledgeByUniqueName", "MapProperty");
        out.extend_from_slice(&2u32.to_le_bytes()); // descriptor count
        out.extend_from_slice(&fstring("NameProperty")); // key type
        out.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        out.extend_from_slice(&fstring("StructProperty")); // value type
        out.extend_from_slice(&1u32.to_le_bytes()); // struct descriptor count
        out.extend_from_slice(&fstring("KnowledgeSet")); // value struct type
        out.extend_from_slice(&1u32.to_le_bytes()); // package count
        out.extend_from_slice(&fstring("/Script/G1R")); // package
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn map_layout_reports_count_and_entry_ranges() {
        let payload = private_root_with_property(&knowledge_map_property(&["A", "BB"]));
        let root = parse_private_root(&payload).unwrap();
        let (_, prop) = find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
        let layout = map_layout(&payload, prop).unwrap();
        assert_eq!(layout.count, 2);
        assert_eq!(layout.entry_ranges.len(), 2);
        assert_eq!(layout.entry_ranges[0].end, layout.entry_ranges[1].start);
    }

    #[test]
    fn map_insert_appends_entry_and_fixes_sizes() {
        let mut payload = private_root_with_property(&knowledge_map_property(&["A"]));
        let root = parse_private_root(&payload).unwrap();
        let (_, prop) = find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
        let enclosing: Vec<usize> = Vec::new(); // top-level property; no enclosing size fields

        // New entry bytes: inline Name key "ZZ" + empty KnowledgeSet value.
        let mut entry = fstring("ZZ");
        let mut val = name_set_property("Knowledge", &[]);
        val.extend_from_slice(&fstring("None"));
        entry.extend_from_slice(&val);

        let prop_owned = prop.clone();
        drop(root);
        patch_container(
            &mut payload,
            &prop_owned,
            &enclosing,
            &ContainerEdit::MapInsert { entry_bytes: entry },
        )
        .unwrap();

        let root2 = parse_private_root(&payload).unwrap();
        let (_, prop2) = find_property_by_name(&root2, "CharacterKnowledgeByUniqueName").unwrap();
        let PropertyValue::Map { entries, .. } = &prop2.value else {
            panic!("not a map")
        };
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .any(|(k, _)| matches!(k, PropertyValue::Name(s) if s == "ZZ"))
        );
        assert_eq!(root2.consumed, payload.len()); // proves size fields are consistent
    }

    #[test]
    fn map_remove_splices_entry_and_fixes_sizes() {
        // Three entries; remove the middle one — the other two must survive and
        // the payload must re-parse byte-clean (size cascade consistent).
        let mut payload = private_root_with_property(&knowledge_map_property(&["A", "BB", "C"]));
        let root = parse_private_root(&payload).unwrap();
        let (_, prop) = find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
        let layout = map_layout(&payload, prop).unwrap();
        // Locate "BB" by stringified key.
        let PropertyValue::Map { entries, .. } = &prop.value else {
            panic!("not a map")
        };
        let idx = entries
            .iter()
            .position(|(k, _)| map_key_to_string(k).as_deref() == Some("BB"))
            .unwrap();
        assert_eq!(layout.count, 3);
        let enclosing: Vec<usize> = Vec::new(); // top-level property
        let prop_owned = prop.clone();
        drop(root);
        patch_container(
            &mut payload,
            &prop_owned,
            &enclosing,
            &ContainerEdit::MapRemove { entry_index: idx },
        )
        .unwrap();

        let root2 = parse_private_root(&payload).unwrap();
        assert_eq!(root2.consumed, payload.len()); // proves size fields are consistent
        let (_, prop2) = find_property_by_name(&root2, "CharacterKnowledgeByUniqueName").unwrap();
        let PropertyValue::Map { entries, .. } = &prop2.value else {
            panic!("not a map")
        };
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .any(|(k, _)| matches!(k, PropertyValue::Name(s) if s == "A"))
        );
        assert!(
            entries
                .iter()
                .any(|(k, _)| matches!(k, PropertyValue::Name(s) if s == "C"))
        );
        assert!(
            !entries
                .iter()
                .any(|(k, _)| matches!(k, PropertyValue::Name(s) if s == "BB"))
        );
    }

    #[test]
    fn map_remove_rejects_out_of_bounds_index() {
        let mut payload = private_root_with_property(&knowledge_map_property(&["A"]));
        let root = parse_private_root(&payload).unwrap();
        let (_, prop) = find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
        let prop_owned = prop.clone();
        let snapshot = payload.clone();
        drop(root);
        let err = patch_container(
            &mut payload,
            &prop_owned,
            &[],
            &ContainerEdit::MapRemove { entry_index: 5 },
        );
        assert!(err.is_err());
        assert_eq!(
            payload, snapshot,
            "a failed map remove must not mutate the payload"
        );
    }

    fn int_array_property(name: &str, values: &[i32]) -> Vec<u8> {
        let mut body = (values.len() as u32).to_le_bytes().to_vec();
        for v in values {
            body.extend_from_slice(&v.to_le_bytes());
        }
        let mut out = tag(name, "ArrayProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("IntProperty"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn container_layout_reports_set_element_ranges() {
        let payload = root(
            "/Script/Test.Save",
            &name_set_property("Knowledge", &["Voiceline_A", "ChoiceB"]),
        );
        let parsed = parse_private_root(&payload).unwrap();
        let target = &parsed.properties[0];

        let layout = container_layout(&payload, target).unwrap();
        assert_eq!(layout.kind, ContainerKind::Set);
        assert_eq!(layout.inner_type, "NameProperty");
        assert_eq!(layout.count, 2);
        // count field sits after the u32 num_to_remove
        assert_eq!(layout.count_offset, target.value_offset + 4);
        assert_eq!(layout.element_ranges.len(), 2);
        // elements are FStrings: 4-byte length + chars + NUL
        let first = &layout.element_ranges[0];
        assert_eq!(first.start, target.value_offset + 8);
        assert_eq!(first.len(), 4 + "Voiceline_A".len() + 1);
        let second = &layout.element_ranges[1];
        assert_eq!(second.start, first.end);
        assert_eq!(second.end, target.value_offset + target.value_size);
    }

    #[test]
    fn container_layout_reports_array_element_ranges() {
        let payload = root("/Script/Test.Save", &int_array_property("Nums", &[7, 8, 9]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = &parsed.properties[0];

        let layout = container_layout(&payload, target).unwrap();
        assert_eq!(layout.kind, ContainerKind::Array);
        assert_eq!(layout.count, 3);
        assert_eq!(layout.count_offset, target.value_offset);
        assert_eq!(
            layout.element_ranges,
            vec![
                target.value_offset + 4..target.value_offset + 8,
                target.value_offset + 8..target.value_offset + 12,
                target.value_offset + 12..target.value_offset + 16,
            ]
        );
    }

    #[test]
    fn container_layout_rejects_non_container_targets() {
        let payload = root("/Script/Test.Save", &int_property("m_X", 1));
        let parsed = parse_private_root(&payload).unwrap();
        assert!(container_layout(&payload, &parsed.properties[0]).is_err());
    }

    fn struct_wrapping(name: &str, struct_type: &str, inner_props: &[u8]) -> Vec<u8> {
        let mut body = inner_props.to_vec();
        body.extend_from_slice(&fstring("None"));
        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/Test"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    fn resolve_set_target(payload: &[u8]) -> (RootObject, Vec<PathSeg>) {
        let parsed = parse_private_root(payload).unwrap();
        let path = parse_path(&["KnowledgeSet".to_string(), "Knowledge".to_string()]).unwrap();
        (parsed, path)
    }

    #[test]
    fn patch_container_set_add_appends_and_fixes_sizes() {
        let mut payload = root(
            "/Script/Test.Save",
            &struct_wrapping(
                "KnowledgeSet",
                "KnowledgeSet",
                &name_set_property("Knowledge", &["Voiceline_A"]),
            ),
        );
        let (parsed, path) = resolve_set_target(&payload);
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();

        patch_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &ContainerEdit::SetAdd("ChoiceB".to_string()),
        )
        .unwrap();

        // Strict re-parse proves every size field (set tag + wrapping struct
        // tag) was adjusted.
        let reparsed = parse_private_root(&payload).unwrap();
        let set = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(
            set.value,
            PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    PropertyValue::Name("Voiceline_A".to_string()),
                    PropertyValue::Name("ChoiceB".to_string()),
                ],
            }
        );
    }

    #[test]
    fn patch_container_set_add_rejects_duplicates_without_mutation() {
        let mut payload = root(
            "/Script/Test.Save",
            &name_set_property("Knowledge", &["Voiceline_A"]),
        );
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        let copy = payload.clone();
        assert!(
            patch_container(
                &mut payload,
                &target,
                &[],
                &ContainerEdit::SetAdd("Voiceline_A".to_string()),
            )
            .is_err()
        );
        assert_eq!(payload, copy);

        // UE FNames are case-insensitive: a case-variant is a duplicate too.
        assert!(
            patch_container(
                &mut payload,
                &target,
                &[],
                &ContainerEdit::SetAdd("VOICELINE_a".to_string()),
            )
            .is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_name_set_remove_folds_case() {
        let mut payload = root(
            "/Script/Test.Save",
            &name_set_property("Knowledge", &["Voiceline_A", "ChoiceB"]),
        );
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        // FName semantics: a case-variant removes the existing member.
        patch_container(
            &mut payload,
            &target,
            &[],
            &ContainerEdit::SetRemove("VOICELINE_a".to_string()),
        )
        .unwrap();
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            reparsed.properties[0].value,
            PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![PropertyValue::Name("ChoiceB".to_string())],
            }
        );
    }

    #[test]
    fn patch_container_str_set_add_keeps_case_sensitivity() {
        // Str sets hold regular strings: case-only variants are distinct.
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&fstring("foo"));
        let mut props = tag("Tags", "SetProperty");
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("StrProperty"));
        props.extend_from_slice(&header(body.len() as u32, 0));
        props.extend_from_slice(&body);
        let mut payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        patch_container(
            &mut payload,
            &target,
            &[],
            &ContainerEdit::SetAdd("Foo".to_string()),
        )
        .unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            reparsed.properties[0].value,
            PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    PropertyValue::Str("foo".to_string()),
                    PropertyValue::Str("Foo".to_string()),
                ],
            }
        );

        // Exact duplicates are still rejected.
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        let copy = payload.clone();
        assert!(
            patch_container(
                &mut payload,
                &target,
                &[],
                &ContainerEdit::SetAdd("Foo".to_string()),
            )
            .is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_set_remove_splices_element_out() {
        let mut payload = root(
            "/Script/Test.Save",
            &struct_wrapping(
                "KnowledgeSet",
                "KnowledgeSet",
                &name_set_property("Knowledge", &["Voiceline_A", "ChoiceB", "Voiceline_C"]),
            ),
        );
        let (parsed, path) = resolve_set_target(&payload);
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();

        patch_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &ContainerEdit::SetRemove("ChoiceB".to_string()),
        )
        .unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        let set = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(
            set.value,
            PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    PropertyValue::Name("Voiceline_A".to_string()),
                    PropertyValue::Name("Voiceline_C".to_string()),
                ],
            }
        );

        // Removing a value that is not present fails without mutation.
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();
        let copy = payload.clone();
        assert!(
            patch_container(
                &mut payload,
                &target,
                &chain.enclosing_size_fields,
                &ContainerEdit::SetRemove("ChoiceB".to_string()),
            )
            .is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_array_remove_and_duplicate() {
        let mut payload = root("/Script/Test.Save", &int_array_property("Nums", &[7, 8, 9]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();

        patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayRemove(1)).unwrap();
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            reparsed.properties[0].value,
            PropertyValue::Array {
                elements: vec![PropertyValue::Int(7), PropertyValue::Int(9)],
            }
        );

        let target = reparsed.properties[0].clone();
        patch_container(
            &mut payload,
            &target,
            &[],
            &ContainerEdit::ArrayDuplicate(0),
        )
        .unwrap();
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            reparsed.properties[0].value,
            PropertyValue::Array {
                elements: vec![
                    PropertyValue::Int(7),
                    PropertyValue::Int(7),
                    PropertyValue::Int(9),
                ],
            }
        );

        // Out-of-bounds index fails without mutation.
        let target = reparsed.properties[0].clone();
        let copy = payload.clone();
        assert!(
            patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayRemove(3)).is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_rejects_kind_mismatch() {
        let mut payload = root("/Script/Test.Save", &int_array_property("Nums", &[7]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        // set ops on an array
        assert!(
            patch_container(
                &mut payload,
                &target,
                &[],
                &ContainerEdit::SetAdd("X".into())
            )
            .is_err()
        );
        // array ops on a set
        let mut payload = root("/Script/Test.Save", &name_set_property("S", &["A"]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        assert!(
            patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayRemove(0)).is_err()
        );
    }

    #[test]
    fn find_property_by_name_returns_addressable_path() {
        // Map { "CharacterStates" => InstancedStruct { Knowledge: Set } }
        let nested = {
            let mut n = name_set_property("Knowledge", &["Voiceline_A"]);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/Test.CharacterStates");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        let mut map_body = 0u32.to_le_bytes().to_vec();
        map_body.extend_from_slice(&1u32.to_le_bytes());
        map_body.extend_from_slice(&fstring("CharacterStates")); // Name key
        map_body.extend_from_slice(&instanced);

        let mut props = tag("m_GenericData", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("NameProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let (path, prop) = find_property_by_name(&parsed, "Knowledge").unwrap();
        assert_eq!(
            path,
            vec!["m_GenericData", "{CharacterStates}", "Knowledge"]
        );
        assert!(matches!(prop.value, PropertyValue::Set { .. }));
        // The returned path round-trips through resolve().
        let segs = parse_path(&path).unwrap();
        assert_eq!(
            resolve(&parsed.properties, &segs).unwrap().name,
            "Knowledge"
        );

        assert!(find_property_by_name(&parsed, "DoesNotExist").is_none());
    }

    #[test]
    fn search_marks_object_enum_and_byte_editable() {
        let mut props = fstring_property("m_Weapon", "ObjectProperty", "/Script/G1R.ItMw_Sword");
        props.extend_from_slice(&enum_property("m_Guild", "EGuild", "EGuild::Old"));
        props.extend_from_slice(&fstring_property("m_Rank", "ByteProperty", "ERank::Novice"));
        props.extend_from_slice(&plain_byte_property("m_Level", 3));
        let payload = root("/Script/Test.Save", &props);
        let parsed = parse_private_root(&payload).unwrap();
        let (hits, _) = search_properties(&parsed, "m_", 0, 100);
        for name in ["m_Weapon", "m_Guild", "m_Rank", "m_Level"] {
            let hit = hits.iter().find(|h| h.display == name).unwrap();
            assert!(hit.editable, "{name} should be editable");
        }
    }
}
