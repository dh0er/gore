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

pub const TAG_FLAG_NATIVE_SERIALIZE: u8 = 0x08;
pub const TAG_FLAG_BOOL_TRUE: u8 = 0x10;

const MAX_DEPTH: usize = 96;

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
    pub name: String,
    pub type_name: String,
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
    /// StructProperty: (struct_type, package)
    pub struct_type: Option<(String, String)>,
    /// EnumProperty: (enum_type, package, underlying_type)
    pub enum_type: Option<(String, String, String)>,
    /// Array/Set inner type, Map key/value types (with nested descriptors).
    pub inner: Option<Box<InnerDescriptor>>,
    pub map: Option<Box<(InnerDescriptor, InnerDescriptor)>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InnerDescriptor {
    pub type_name: String,
    pub struct_type: Option<(String, String)>,
    pub enum_type: Option<(String, String, String)>,
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
    pub actual_type: String,
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
fn map_key_to_string(key: &PropertyValue) -> Option<String> {
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
    ctx: &mut SearchCtx,
) {
    for p in props {
        let display_len = display.len();
        if !display.is_empty() {
            display.push_str(" › ");
        }
        display.push_str(&p.name);
        path.push(p.name.clone());

        // Leaf value?
        if let Some(value_display) = scalar_display(&p.value) {
            if ctx.terms.iter().all(|t| display.to_lowercase().contains(t)) {
                ctx.record(PropertyHit {
                    path: path.clone(),
                    display: display.clone(),
                    type_name: p.type_name.clone(),
                    value_display,
                    editable: scalar_editable(&p.value),
                });
            }
        } else {
            walk_value_search(&p.value, path, display, ctx);
        }

        path.pop();
        display.truncate(display_len);
    }
}

fn walk_value_search(
    value: &PropertyValue,
    path: &mut Vec<String>,
    display: &mut String,
    ctx: &mut SearchCtx,
) {
    match value {
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            walk_search(inner, path, display, ctx);
        }
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
            walk_search(&i.properties, path, display, ctx);
        }
        PropertyValue::ObjectInstances(objs) => {
            for (idx, obj) in objs.iter().enumerate() {
                descend_indexed(idx, &obj.properties, path, display, ctx);
            }
        }
        PropertyValue::Map { entries, .. } => {
            for (key, val) in entries {
                let key_label = map_key_label(key);
                descend_value(&format!("{{{key_label}}}"), val, path, display, ctx);
            }
        }
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
            for (idx, el) in elements.iter().enumerate() {
                descend_value(&format!("[{idx}]"), el, path, display, ctx);
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
    ctx: &mut SearchCtx,
) {
    let display_len = display.len();
    let seg = format!("[{idx}]");
    display.push_str(&seg);
    path.push(seg);
    walk_search(props, path, display, ctx);
    path.pop();
    display.truncate(display_len);
}

fn descend_value(
    seg: &str,
    value: &PropertyValue,
    path: &mut Vec<String>,
    display: &mut String,
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
        walk_value_search(value, path, display, ctx);
    }
    path.pop();
    display.truncate(display_len);
}

fn map_key_label(key: &PropertyValue) -> String {
    // Delegate to the resolver's segment renderer so search-built paths always
    // round-trip through `resolve`. Unaddressable key types collapse to "?".
    map_key_to_string(key).unwrap_or_else(|| "?".to_string())
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
fn encode_fstring_value(value: &str) -> Vec<u8> {
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
/// [`patch_string`] can replace: Str/Name/Object/Enum properties, plus the
/// enum-as-FString form of ByteProperty (the plain one-byte form is a scalar).
pub fn string_patchable(property: &Property) -> bool {
    match property.type_name.as_str() {
        "StrProperty" | "NameProperty" | "ObjectProperty" | "EnumProperty" => true,
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

/// Parse a full decompressed private payload (strict: every byte accounted for).
pub fn parse_private_root(payload: &[u8]) -> Result<RootObject, CoreError> {
    let mut r = Reader::new(payload, 0);
    let root = read_object(&mut r, 0)?;
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "trailing bytes after root object: {} remaining",
            r.remaining()
        )));
    }
    Ok(root)
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
        let name = r.fstring()?;
        if name == "None" {
            return Ok(out);
        }
        let type_name = r.fstring()?;
        out.push(read_property(r, name, type_name, depth)?);
    }
}

fn read_property(
    r: &mut Reader,
    name: String,
    type_name: String,
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
            descriptor.struct_type = Some(read_struct_descriptor(r)?);
        }
        "EnumProperty" => {
            descriptor.enum_type = Some(read_enum_descriptor(r)?);
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

fn read_struct_descriptor(r: &mut Reader) -> Result<(String, String), CoreError> {
    let _count = r.u32()?;
    let struct_type = r.fstring()?;
    let _package_count = r.u32()?;
    let package = r.fstring()?;
    Ok((struct_type, package))
}

fn read_enum_descriptor(r: &mut Reader) -> Result<(String, String, String), CoreError> {
    let _count = r.u32()?;
    let enum_type = r.fstring()?;
    let _package_count = r.u32()?;
    let package = r.fstring()?;
    let _underlying_count = r.u32()?;
    let underlying = r.fstring()?;
    Ok((enum_type, package, underlying))
}

fn read_inner_descriptor(r: &mut Reader) -> Result<InnerDescriptor, CoreError> {
    let _count = r.u32()?;
    read_inner_descriptor_body(r)
}

fn read_inner_descriptor_body(r: &mut Reader) -> Result<InnerDescriptor, CoreError> {
    let type_name = r.fstring()?;
    let mut inner = InnerDescriptor {
        type_name: type_name.clone(),
        struct_type: None,
        enum_type: None,
    };
    match type_name.as_str() {
        "StructProperty" => inner.struct_type = Some(read_struct_descriptor(r)?),
        "EnumProperty" => inner.enum_type = Some(read_enum_descriptor(r)?),
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
        "ObjectProperty" => Ok(PropertyValue::Object(r.fstring()?)),
        "EnumProperty" => Ok(PropertyValue::Enum(r.fstring()?)),
        "SoftObjectProperty" => Ok(PropertyValue::SoftObject(read_soft_object_path(r)?)),
        "TextProperty" => Ok(PropertyValue::Opaque(r.read(r.remaining())?.to_vec())),
        "StructProperty" => {
            let (struct_type, _) = descriptor
                .struct_type
                .as_ref()
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
        other => Err(CoreError::Parse(format!(
            "unsupported property type {other:?} at 0x{:x}",
            r.abs_pos()
        ))),
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
                .as_ref()
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
            let actual_type = r.fstring()?;
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
    /// `GORESAVE_PAYLOAD_BIN=work/decompressed/G1R-001.host.bin cargo test -p goresave_core real_payload -- --ignored --nocapture`
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
