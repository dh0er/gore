//! Fail-closed discovery and copy-on-write patching of directly proven class defaults.
//!
//! The only accepted v1 shape is an exact, contiguous AngelScript bytecode window:
//!
//! `SetV{1,2,4,8} slot, immediate; LoadThisR member_offset, owner_type_id; WRTV{same} slot`
//!
//! Offsets are output provenance only. Patches re-resolve a semantic selector, compare the
//! complete serialized immediate (CAS), preserve the cache length, and validate the entire
//! cache again before returning any bytes.

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::binds::NativeApi;
use super::disasm::{disassemble, Instr};
use super::emit_all::prepare_resolver_semantics;
use super::header::CacheHeader;
use super::model::{parse_modules, Module};
use super::refs::{RefResolver, TypeIdentity};
use super::tables::parse_tail_tables;
use super::walk_modules::FuncCodeKind;
use super::walk_modules::{collect_function_bytecode_spans, module_region_end, FuncCodeSpan};

pub const DEFAULT_SITE_SELECTOR_FORMAT: &str = "gore-as-default-site-v3";
pub const DEFAULT_SITES_REPORT_FORMAT: &str = "gore-as-default-sites-v1";
pub const DEFAULT_PATCH_REPORT_FORMAT: &str = "gore-as-default-patch-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefaultPattern {
    SetV1LoadThisWrtV1,
    SetV2LoadThisWrtV2,
    SetV4LoadThisWrtV4,
    SetV8LoadThisWrtV8,
}

impl DefaultPattern {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SetV1LoadThisWrtV1 => "set_v1_load_this_wrt_v1",
            Self::SetV2LoadThisWrtV2 => "set_v2_load_this_wrt_v2",
            Self::SetV4LoadThisWrtV4 => "set_v4_load_this_wrt_v4",
            Self::SetV8LoadThisWrtV8 => "set_v8_load_this_wrt_v8",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "set_v1_load_this_wrt_v1" => Some(Self::SetV1LoadThisWrtV1),
            "set_v2_load_this_wrt_v2" => Some(Self::SetV2LoadThisWrtV2),
            "set_v4_load_this_wrt_v4" => Some(Self::SetV4LoadThisWrtV4),
            "set_v8_load_this_wrt_v8" => Some(Self::SetV8LoadThisWrtV8),
            _ => None,
        }
    }

    pub const fn value_width(self) -> usize {
        match self {
            Self::SetV1LoadThisWrtV1 => 1,
            Self::SetV2LoadThisWrtV2 => 2,
            Self::SetV4LoadThisWrtV4 => 4,
            Self::SetV8LoadThisWrtV8 => 8,
        }
    }

    /// The complete serialized immediate width. SetV1/SetV2 still carry a full dword.
    pub const fn operand_width(self) -> usize {
        match self {
            Self::SetV8LoadThisWrtV8 => 8,
            _ => 4,
        }
    }

    const fn set_name(self) -> &'static str {
        match self {
            Self::SetV1LoadThisWrtV1 => "SetV1",
            Self::SetV2LoadThisWrtV2 => "SetV2",
            Self::SetV4LoadThisWrtV4 => "SetV4",
            Self::SetV8LoadThisWrtV8 => "SetV8",
        }
    }

    const fn write_name(self) -> &'static str {
        match self {
            Self::SetV1LoadThisWrtV1 => "WRTV1",
            Self::SetV2LoadThisWrtV2 => "WRTV2",
            Self::SetV4LoadThisWrtV4 => "WRTV4",
            Self::SetV8LoadThisWrtV8 => "WRTV8",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawEncoding {
    LeU32,
    LeU64,
}

impl RawEncoding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LeU32 => "le_u32",
            Self::LeU64 => "le_u64",
        }
    }

    pub const fn width(self) -> usize {
        match self {
            Self::LeU32 => 4,
            Self::LeU64 => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultSiteSelector {
    pub module: String,
    pub class: String,
    /// Declaring owner resolved from the `LoadThisR` type id. This is semantic selector input,
    /// not incidental provenance: inherited and shadowed same-name fields are distinct sites.
    pub field_owner: String,
    pub field: String,
    /// Canonical field value type. Binding this semantic meaning prevents a stale raw CAS from
    /// crossing (for example) an `int32` -> `float32` hotfix with identical operand bytes.
    pub value_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultSite {
    pub selector: DefaultSiteSelector,
    pub field_owner: String,
    pub owner_type_id: i32,
    pub member_offset: i32,
    pub value_type: String,
    pub pattern: DefaultPattern,
    /// SHA-256 of the three-instruction window with only the value operand zeroed.
    /// This is provenance, not selector input, so recompilation drift cannot redirect a patch.
    pub context_sha256: String,
    pub expected: Vec<u8>,
    pub display_value: String,
    pub encoding: RawEncoding,
    pub function: String,
    pub opcode: &'static str,
    pub instruction_index: usize,
    pub instruction_offset_dw: usize,
    /// Absolute cache byte offset. Provenance only; never accepted as patch input.
    pub operand_offset: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultSiteStats {
    pub init_functions: usize,
    pub branched_init_functions: usize,
    pub direct_windows: usize,
    pub unresolved_fields: usize,
    pub unresolved_types: usize,
    pub unsupported_types: usize,
    pub ambiguous_fields: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultSiteReport {
    pub cache_len: usize,
    pub cache_sha256: String,
    pub sites: Vec<DefaultSite>,
    pub stats: DefaultSiteStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultPatch {
    pub bytes: Vec<u8>,
    pub before: DefaultSite,
    pub after: DefaultSite,
}

#[derive(Debug, Error)]
pub enum DefaultSiteError {
    #[error("invalid cache header: {0}")]
    Header(String),
    #[error("invalid cache structure: {0}")]
    Wire(String),
    #[error("cache tail tables end at {end:#x}, not EOF {len:#x}")]
    TailNotAtEof { end: usize, len: usize },
    #[error("duplicate key {key:#x} in tail table {table}")]
    DuplicateTailKey { table: &'static str, key: i64 },
    #[error("failed to disassemble {function}: {error}")]
    Disasm { function: String, error: String },
    #[error("duplicate class identity {module}.{class} in module model")]
    DuplicateClass { module: String, class: String },
    #[error("duplicate bare class identity {class} in modules {first_module} and {second_module}")]
    DuplicateBareClass {
        class: String,
        first_module: String,
        second_module: String,
    },
    #[error("class {module}.{class} declares field {field} more than once")]
    DuplicateDirectField {
        module: String,
        class: String,
        field: String,
    },
    #[error("class hierarchy contains a cycle reachable from {class}")]
    CyclicClassHierarchy { class: String },
    #[error("class {module}.{class} has {count} generated void __InitDefaults methods")]
    AmbiguousInitializer {
        module: String,
        class: String,
        count: usize,
    },
    #[error("bytecode range overflow in {function}")]
    RangeOverflow { function: String },
    #[error("bytecode provenance mismatch in {function} at cache offset {offset:#x}")]
    ProvenanceMismatch { function: String, offset: usize },
}

#[derive(Debug, Error)]
pub enum DefaultPatchError {
    #[error(transparent)]
    Inspect(#[from] DefaultSiteError),
    #[error("default-site selector was not found or is not uniquely editable")]
    SelectorNotFound,
    #[error("default-site selector matched {matches} editable sites")]
    SelectorAmbiguous { matches: usize },
    #[error("expected operand has {got} bytes; selector requires {required}")]
    ExpectedWidth { required: usize, got: usize },
    #[error("replacement operand has {got} bytes; selector requires {required}")]
    ReplacementWidth { required: usize, got: usize },
    #[error("expected operand drifted: expected {expected}, got {actual}")]
    CasMismatch { expected: String, actual: String },
    #[error("expected and replacement operands are identical")]
    NoChange,
    #[error("replacement is not a valid raw value for {value_type}: {reason}")]
    InvalidReplacement { value_type: String, reason: String },
    #[error("patch postcondition failed: {0}")]
    Postcondition(String),
}

#[derive(Debug, Clone)]
struct ClassIdentity {
    module: String,
    class: String,
}

#[derive(Debug)]
struct InspectionContext {
    refs: RefResolver,
    identities: HashMap<String, ClassIdentity>,
    hierarchy: ClassHierarchy,
    /// Exact direct script fields whose serialized `DataType.type_info` resolves to one parsed
    /// script enum's full module/namespace/name identity.
    script_enum_fields: HashMap<String, HashMap<String, TypeIdentity>>,
    script_cache_guid: [u8; 16],
}

#[derive(Debug, Clone)]
struct ClassHierarchy {
    /// Every script class appears exactly once. `None` means no declared super; a non-script
    /// parent remains a valid terminal name so direct native ownership can still be proven.
    supers: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
struct RawWindow {
    pattern: DefaultPattern,
    instruction_index: usize,
    instruction_offset_dw: usize,
    operand_offset_dw: usize,
    owner_type_id: i32,
    member_offset: i32,
    context_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    Bool,
    Signed,
    Unsigned,
    Float32,
    Float64,
    Enum,
}

/// Discover every uniquely editable direct-member default site in a cache.
pub fn default_sites(
    cache: &[u8],
    native: Option<NativeApi>,
) -> Result<DefaultSiteReport, DefaultSiteError> {
    let context = InspectionContext::build(cache, native)?;
    context.inspect(cache)
}

/// Apply one semantic, compare-and-swap default patch to a cloned cache buffer.
pub fn patch_default(
    cache: &[u8],
    native: Option<NativeApi>,
    selector: &DefaultSiteSelector,
    expected: &[u8],
    replacement: &[u8],
) -> Result<DefaultPatch, DefaultPatchError> {
    let context = InspectionContext::build(cache, native)?;
    let before_report = context.inspect(cache)?;
    let matches: Vec<_> = before_report
        .sites
        .iter()
        .filter(|site| &site.selector == selector)
        .collect();
    let before = match matches.as_slice() {
        [] => return Err(DefaultPatchError::SelectorNotFound),
        [site] => (*site).clone(),
        many => {
            return Err(DefaultPatchError::SelectorAmbiguous {
                matches: many.len(),
            })
        }
    };

    let required = before.encoding.width();
    if expected.len() != required {
        return Err(DefaultPatchError::ExpectedWidth {
            required,
            got: expected.len(),
        });
    }
    if replacement.len() != required {
        return Err(DefaultPatchError::ReplacementWidth {
            required,
            got: replacement.len(),
        });
    }
    if before.expected != expected {
        return Err(DefaultPatchError::CasMismatch {
            expected: encode_hex(expected),
            actual: encode_hex(&before.expected),
        });
    }
    if expected == replacement {
        return Err(DefaultPatchError::NoChange);
    }
    validate_raw_value(
        &before.value_type,
        before.pattern.value_width(),
        replacement,
        context.proves_script_enum(
            &before.field_owner,
            &before.selector.field,
            &before.value_type,
        ),
    )
    .map_err(|reason| DefaultPatchError::InvalidReplacement {
        value_type: before.value_type.clone(),
        reason,
    })?;

    let end = before
        .operand_offset
        .checked_add(required)
        .ok_or_else(|| DefaultPatchError::Postcondition("operand range overflow".into()))?;
    if cache.get(before.operand_offset..end) != Some(expected) {
        return Err(DefaultPatchError::CasMismatch {
            expected: encode_hex(expected),
            actual: cache
                .get(before.operand_offset..end)
                .map(encode_hex)
                .unwrap_or_else(|| "<out-of-range>".into()),
        });
    }

    let mut output = cache.to_vec();
    output[before.operand_offset..end].copy_from_slice(replacement);
    validate_cache(&output)?;
    verify_only_range_changed(cache, &output, before.operand_offset, end)?;

    let after_report = context.inspect(&output)?;
    let after_matches: Vec<_> = after_report
        .sites
        .iter()
        .filter(|site| &site.selector == selector)
        .collect();
    let after = match after_matches.as_slice() {
        [site] => (*site).clone(),
        other => {
            return Err(DefaultPatchError::Postcondition(format!(
                "selector rediscovered {} times after patch",
                other.len()
            )))
        }
    };
    if after.operand_offset != before.operand_offset || after.expected != replacement {
        return Err(DefaultPatchError::Postcondition(
            "rediscovered site does not prove the replacement at the original operand".into(),
        ));
    }

    Ok(DefaultPatch {
        bytes: output,
        before,
        after,
    })
}

impl InspectionContext {
    fn build(cache: &[u8], native: Option<NativeApi>) -> Result<Self, DefaultSiteError> {
        validate_cache(cache)?;
        let script_cache_guid = CacheHeader::parse(cache)
            .map_err(|error| DefaultSiteError::Header(error.to_string()))?
            .hash;
        let modules = parse_modules(cache).map_err(wire_error)?;
        // Validate bare class identity and inheritance before the shared resolver flattens both
        // maps by class name. Otherwise a later duplicate could silently replace type evidence.
        let hierarchy = ClassHierarchy::build(&modules)?;
        let mut refs = RefResolver::build(cache).map_err(wire_error)?;
        let script_enum_fields = proven_script_enum_fields(&modules, &refs);
        prepare_resolver_semantics(&modules, &mut refs, native);
        let identities = class_identities(&modules)?;
        Ok(Self {
            refs,
            identities,
            hierarchy,
            script_enum_fields,
            script_cache_guid,
        })
    }

    fn inspect(&self, cache: &[u8]) -> Result<DefaultSiteReport, DefaultSiteError> {
        validate_cache(cache)?;
        let spans = collect_function_bytecode_spans(cache).map_err(wire_error)?;
        let mut stats = DefaultSiteStats::default();
        let mut provisional = Vec::new();
        let mut init_counts: HashMap<&str, usize> = HashMap::new();

        for span in &spans {
            let Some(identity) = self.identities.get(&span.code.func) else {
                continue;
            };
            if span.kind != FuncCodeKind::ClassMethod
                || !span.method_table_valid
                || !span.in_method_table
                || !is_initializer_traits(span.function_traits)
                || !span.code.is_method
                || !span.code.param_types.is_empty()
                || !is_plain_void(&span.code.ret)
            {
                continue;
            }
            stats.init_functions += 1;
            *init_counts.entry(&span.code.func).or_default() += 1;
            let instrs =
                disassemble(&span.code.bytecode).map_err(|error| DefaultSiteError::Disasm {
                    function: span.code.func.clone(),
                    error: error.to_string(),
                })?;
            let has_branch = instrs
                .iter()
                .any(|instruction| is_branch(instruction.op.name));
            if has_branch {
                stats.branched_init_functions += 1;
            }
            // v1 mutation sites are admitted only on one reachable linear path. Merely banning
            // jumps is insufficient: a pattern after an early RET is unreachable dead bytecode.
            if !is_reachable_linear_initializer(&instrs) {
                continue;
            }
            let windows = direct_windows(&span.code.bytecode, &instrs);
            stats.direct_windows += windows.len();
            for window in windows {
                match self.resolve_site(cache, span, identity, window)? {
                    Resolved::Site(site) => provisional.push(*site),
                    Resolved::UnresolvedField => stats.unresolved_fields += 1,
                    Resolved::UnresolvedType => stats.unresolved_types += 1,
                    Resolved::UnsupportedType => stats.unsupported_types += 1,
                }
            }
        }

        // A duplicated initializer or repeated direct assignment to the same semantic field is
        // deliberately not addressable in v1. Removing every member of such a group prevents a
        // selector from silently choosing an occurrence by byte offset or incidental order.
        let mut semantic_counts: HashMap<DefaultSiteSelector, usize> = HashMap::new();
        for site in &provisional {
            *semantic_counts.entry(site.selector.clone()).or_default() += 1;
        }
        stats.ambiguous_fields = semantic_counts
            .values()
            .filter(|count| **count != 1)
            .count();
        let mut sites = Vec::new();
        for site in provisional {
            let duplicated_function = init_counts
                .get(site.function.as_str())
                .copied()
                .unwrap_or_default()
                != 1;
            let duplicated_field = semantic_counts
                .get(&site.selector)
                .copied()
                .unwrap_or_default()
                != 1;
            if !duplicated_function && !duplicated_field {
                sites.push(site);
            }
        }
        sites.sort_by(|a, b| a.selector.cmp(&b.selector));

        Ok(DefaultSiteReport {
            cache_len: cache.len(),
            cache_sha256: sha256_hex(cache),
            sites,
            stats,
        })
    }

    fn resolve_site(
        &self,
        cache: &[u8],
        span: &FuncCodeSpan,
        identity: &ClassIdentity,
        window: RawWindow,
    ) -> Result<Resolved, DefaultSiteError> {
        let Some(field) = self
            .refs
            .member(window.owner_type_id, window.member_offset)
            .map(str::to_owned)
        else {
            return Ok(Resolved::UnresolvedField);
        };
        let Some(field_owner) = self
            .refs
            .type_by_id(window.owner_type_id)
            .map(str::to_owned)
        else {
            return Ok(Resolved::UnresolvedField);
        };
        if !self
            .hierarchy
            .proves_ancestry(&identity.class, &field_owner)
        {
            // The operand may name a real field, but without a target->owner inheritance proof it
            // is not a field of this initializer's class and must not inherit that class's selector.
            return Ok(Resolved::UnresolvedField);
        }

        let mut type_evidence = HashSet::new();
        // LoadThisR's type id is the declaring/owning type. A same-named field on the
        // concrete target class is unrelated and must never supply missing type evidence.
        for value_type in [
            self.refs.own_field_type_by_class(&field_owner, &field),
            self.refs.verified_native_default_field_type(
                &self.script_cache_guid,
                &field_owner,
                &field,
            ),
        ]
        .into_iter()
        .flatten()
        {
            type_evidence.insert(normalize_type_name(value_type));
        }
        if type_evidence.len() != 1 {
            return Ok(Resolved::UnresolvedType);
        }
        let raw_value_type = type_evidence.into_iter().next().unwrap();
        let enum_identity = self
            .script_enum_fields
            .get(&field_owner)
            .and_then(|fields| fields.get(&field));
        if enum_identity.is_some_and(|identity| identity.name != raw_value_type) {
            return Ok(Resolved::UnresolvedType);
        }
        let (value_type, enum_proven) = match enum_identity {
            Some(identity) => (canonical_script_enum_type(identity), true),
            None => (raw_value_type, false),
        };
        let Some((kind, type_width)) = classify_value_type(&value_type, enum_proven) else {
            return Ok(Resolved::UnsupportedType);
        };
        let value_width = window.pattern.value_width();
        if type_width.is_some_and(|width| width != value_width)
            || (kind == ValueKind::Enum && !matches!(value_width, 1 | 2 | 4))
        {
            return Ok(Resolved::UnsupportedType);
        }

        let operand_offset = span
            .bytecode_offset
            .checked_add(window.operand_offset_dw.checked_mul(4).ok_or_else(|| {
                DefaultSiteError::RangeOverflow {
                    function: span.code.func.clone(),
                }
            })?)
            .ok_or_else(|| DefaultSiteError::RangeOverflow {
                function: span.code.func.clone(),
            })?;
        let operand_end = operand_offset
            .checked_add(window.pattern.operand_width())
            .ok_or_else(|| DefaultSiteError::RangeOverflow {
                function: span.code.func.clone(),
            })?;
        let Some(expected) = cache.get(operand_offset..operand_end).map(<[u8]>::to_vec) else {
            return Err(DefaultSiteError::RangeOverflow {
                function: span.code.func.clone(),
            });
        };
        let bytecode_expected = immediate_bytes(
            &span.code.bytecode,
            window.operand_offset_dw,
            window.pattern.operand_width(),
        )
        .ok_or_else(|| DefaultSiteError::RangeOverflow {
            function: span.code.func.clone(),
        })?;
        if expected != bytecode_expected {
            return Err(DefaultSiteError::ProvenanceMismatch {
                function: span.code.func.clone(),
                offset: operand_offset,
            });
        }
        if validate_raw_value(&value_type, value_width, &expected, enum_proven).is_err() {
            return Ok(Resolved::UnsupportedType);
        }
        let Some(display_value) =
            display_raw_value(&value_type, value_width, &expected, enum_proven)
        else {
            return Ok(Resolved::UnsupportedType);
        };

        let encoding = if window.pattern.operand_width() == 8 {
            RawEncoding::LeU64
        } else {
            RawEncoding::LeU32
        };
        Ok(Resolved::Site(Box::new(DefaultSite {
            selector: DefaultSiteSelector {
                module: identity.module.clone(),
                class: identity.class.clone(),
                field_owner: field_owner.clone(),
                field,
                value_type: value_type.clone(),
            },
            field_owner,
            owner_type_id: window.owner_type_id,
            member_offset: window.member_offset,
            value_type,
            pattern: window.pattern,
            context_sha256: window.context_sha256,
            expected,
            display_value,
            encoding,
            function: span.code.func.clone(),
            opcode: window.pattern.set_name(),
            instruction_index: window.instruction_index,
            instruction_offset_dw: window.instruction_offset_dw,
            operand_offset,
        })))
    }

    fn proves_script_enum(&self, field_owner: &str, field: &str, value_type: &str) -> bool {
        self.script_enum_fields
            .get(field_owner)
            .and_then(|fields| fields.get(field))
            .is_some_and(|identity| canonical_script_enum_type(identity) == value_type)
    }
}

enum Resolved {
    Site(Box<DefaultSite>),
    UnresolvedField,
    UnresolvedType,
    UnsupportedType,
}

fn validate_cache(cache: &[u8]) -> Result<(), DefaultSiteError> {
    CacheHeader::parse(cache).map_err(|error| DefaultSiteError::Header(error.to_string()))?;
    let tail = module_region_end(cache).map_err(wire_error)?;
    let tables = parse_tail_tables(cache, tail).map_err(wire_error)?;
    if tables.end != cache.len() {
        return Err(DefaultSiteError::TailNotAtEof {
            end: tables.end,
            len: cache.len(),
        });
    }
    const TABLE_NAMES: [&str; 7] = [
        "TypeReferences",
        "TypeIdReferenceToPointer",
        "FunctionReferences",
        "FunctionIdReferenceToPointer",
        "GlobalReferences",
        "StaticNames",
        "PropertyReferences",
    ];
    for (index, table) in tables.tables.iter().enumerate() {
        let mut keys = HashSet::with_capacity(table.keys.len());
        for key in &table.keys {
            if !keys.insert(*key) {
                return Err(DefaultSiteError::DuplicateTailKey {
                    table: TABLE_NAMES[index],
                    key: *key,
                });
            }
        }
    }
    parse_modules(cache).map_err(wire_error)?;
    Ok(())
}

impl ClassHierarchy {
    fn build(modules: &[Module]) -> Result<Self, DefaultSiteError> {
        let mut supers = HashMap::new();
        let mut defining_modules: HashMap<String, String> = HashMap::new();
        for module in modules {
            for class in &module.classes {
                if let Some(first_module) = defining_modules.get(&class.name) {
                    return Err(DefaultSiteError::DuplicateBareClass {
                        class: class.name.clone(),
                        first_module: first_module.clone(),
                        second_module: module.name.clone(),
                    });
                }
                defining_modules.insert(class.name.clone(), module.name.clone());
                let mut direct_fields = HashSet::new();
                for field in &class.fields {
                    if !direct_fields.insert(field.name.as_str()) {
                        return Err(DefaultSiteError::DuplicateDirectField {
                            module: module.name.clone(),
                            class: class.name.clone(),
                            field: field.name.clone(),
                        });
                    }
                }
                supers.insert(
                    class.name.clone(),
                    class.super_class.clone().filter(|name| !name.is_empty()),
                );
            }
        }

        let hierarchy = Self { supers };
        for class in hierarchy.supers.keys() {
            hierarchy.validate_chain(class)?;
        }
        Ok(hierarchy)
    }

    fn validate_chain(&self, start: &str) -> Result<(), DefaultSiteError> {
        let mut seen = HashSet::new();
        let mut current = start;
        while let Some(Some(parent)) = self.supers.get(current) {
            if !seen.insert(current) {
                return Err(DefaultSiteError::CyclicClassHierarchy {
                    class: start.to_owned(),
                });
            }
            // An unparsed parent is a native terminal. Its name is still usable as the direct
            // owner proof, but ancestry above it is deliberately unknown.
            if !self.supers.contains_key(parent) {
                return Ok(());
            }
            current = parent;
        }
        Ok(())
    }

    fn proves_ancestry(&self, target: &str, owner: &str) -> bool {
        if target == owner {
            return self.supers.contains_key(target);
        }
        let mut seen = HashSet::new();
        let mut current = target;
        while let Some(Some(parent)) = self.supers.get(current) {
            if !seen.insert(current) {
                return false;
            }
            if parent == owner {
                return true;
            }
            if !self.supers.contains_key(parent) {
                return false;
            }
            current = parent;
        }
        false
    }
}

fn proven_script_enum_fields(
    modules: &[Module],
    refs: &RefResolver,
) -> HashMap<String, HashMap<String, TypeIdentity>> {
    let class_names: HashSet<_> = modules
        .iter()
        .flat_map(|module| module.classes.iter())
        .map(|class| class.name.as_str())
        .collect();
    let mut enum_counts = HashMap::new();
    for module in modules {
        for enum_def in &module.enums {
            let identity = TypeIdentity {
                name: enum_def.name.clone(),
                module: module.name.clone(),
                namespace: enum_def.namespace.clone(),
            };
            *enum_counts.entry(identity).or_insert(0usize) += 1;
        }
    }
    let enums: HashSet<_> = enum_counts
        .into_iter()
        .filter_map(|(identity, count)| {
            (count == 1 && !class_names.contains(identity.name.as_str())).then_some(identity)
        })
        .collect();

    let mut fields: HashMap<String, HashMap<String, TypeIdentity>> = HashMap::new();
    for module in modules {
        for class in &module.classes {
            for field in &class.fields {
                if !is_plain_identifier_value(&field.ty)
                    || refs.type_subtypes(field.ty.type_info).is_some()
                {
                    continue;
                }
                let Some(identity) = refs.type_identity_by_ptr(field.ty.type_info) else {
                    continue;
                };
                if enums.contains(identity) {
                    fields
                        .entry(class.name.clone())
                        .or_default()
                        .insert(field.name.clone(), identity.clone());
                }
            }
        }
    }
    fields
}

fn is_plain_identifier_value(value: &super::types::DataType) -> bool {
    !value.is_reference
        && !value.is_object_const
        && !value.is_object_handle
        && !value.is_read_only
        && !value.is_auto
        && !value.if_handle_then_const
        && value.type_info != 0
        && value.token == 5
}

fn canonical_script_enum_type(identity: &TypeIdentity) -> String {
    format!(
        "script-enum:{}:{}:{}:{}:{}:{}",
        identity.module.len(),
        identity.module,
        identity.namespace.len(),
        identity.namespace,
        identity.name.len(),
        identity.name
    )
}

fn class_identities(
    modules: &[Module],
) -> Result<HashMap<String, ClassIdentity>, DefaultSiteError> {
    let mut result = HashMap::new();
    for module in modules {
        for class in &module.classes {
            let initializer_count = class
                .methods
                .iter()
                .filter(|function| {
                    function.name == "__InitDefaults"
                        && is_initializer_traits(function.traits)
                        && function.params.is_empty()
                        && is_plain_void(&function.ret)
                })
                .count();
            if initializer_count == 0 {
                continue;
            }
            if initializer_count != 1 {
                return Err(DefaultSiteError::AmbiguousInitializer {
                    module: module.name.clone(),
                    class: class.name.clone(),
                    count: initializer_count,
                });
            }
            let function = format!("{}.{}::__InitDefaults", module.name, class.name);
            if result
                .insert(
                    function,
                    ClassIdentity {
                        module: module.name.clone(),
                        class: class.name.clone(),
                    },
                )
                .is_some()
            {
                return Err(DefaultSiteError::DuplicateClass {
                    module: module.name.clone(),
                    class: class.name.clone(),
                });
            }
        }
    }
    Ok(result)
}

fn is_initializer_traits(traits: i32) -> bool {
    // Shipping contains exactly two compiler-emitted shapes: ordinary and final.
    matches!(traits, 0 | 0x20)
}

fn is_plain_void(value: &super::types::DataType) -> bool {
    !value.is_reference
        && !value.is_object_const
        && !value.is_object_handle
        && !value.is_read_only
        && !value.is_auto
        && !value.if_handle_then_const
        && value.type_info == 0
        && value.token == 0x52
}

fn direct_windows(bytecode: &[i32], instrs: &[Instr]) -> Vec<RawWindow> {
    let mut result = Vec::new();
    for (instruction_index, triple) in instrs.windows(3).enumerate() {
        let set = &triple[0];
        let load = &triple[1];
        let write = &triple[2];
        let Some(pattern) = pattern_for_set(set.op.name) else {
            continue;
        };
        if load.op.name != "LoadThisR" || write.op.name != pattern.write_name() {
            continue;
        }
        let (Some(&set_slot), Some(&write_slot)) = (set.words.first(), write.words.first()) else {
            continue;
        };
        if set_slot != write_slot {
            continue;
        }
        let (Some(&member_offset), Some(&owner_type_id)) =
            (load.words.first(), load.dwords.first())
        else {
            continue;
        };
        let operand_offset_dw = set.offset_dw + 1;
        let Some(context_sha256) = context_hash(bytecode, triple, operand_offset_dw, pattern)
        else {
            continue;
        };
        result.push(RawWindow {
            pattern,
            instruction_index,
            instruction_offset_dw: set.offset_dw,
            operand_offset_dw,
            owner_type_id: owner_type_id as i32,
            member_offset: member_offset as i32,
            context_sha256,
        });
    }
    result
}

fn pattern_for_set(name: &str) -> Option<DefaultPattern> {
    match name {
        "SetV1" => Some(DefaultPattern::SetV1LoadThisWrtV1),
        "SetV2" => Some(DefaultPattern::SetV2LoadThisWrtV2),
        "SetV4" => Some(DefaultPattern::SetV4LoadThisWrtV4),
        "SetV8" => Some(DefaultPattern::SetV8LoadThisWrtV8),
        _ => None,
    }
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

fn context_hash(
    bytecode: &[i32],
    triple: &[Instr],
    operand_offset_dw: usize,
    pattern: DefaultPattern,
) -> Option<String> {
    let start = triple.first()?.offset_dw;
    let last = triple.last()?;
    let end = last.offset_dw.checked_add(last.op.size_dwords as usize)?;
    let mut bytes = bytecode_words(bytecode.get(start..end)?)?;
    let relative = operand_offset_dw.checked_sub(start)?.checked_mul(4)?;
    let value_end = relative.checked_add(pattern.operand_width())?;
    bytes.get_mut(relative..value_end)?.fill(0);
    Some(sha256_hex(&bytes))
}

fn immediate_bytes(bytecode: &[i32], offset_dw: usize, width: usize) -> Option<Vec<u8>> {
    if !width.is_multiple_of(4) {
        return None;
    }
    let words = bytecode.get(offset_dw..offset_dw.checked_add(width / 4)?)?;
    bytecode_words(words)
}

fn bytecode_words(words: &[i32]) -> Option<Vec<u8>> {
    let capacity = words.len().checked_mul(4)?;
    let mut bytes = Vec::with_capacity(capacity);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    Some(bytes)
}

fn normalize_type_name(value: &str) -> String {
    value
        .trim()
        .strip_prefix("const ")
        .unwrap_or(value.trim())
        .trim_end_matches('@')
        .to_owned()
}

fn classify_value_type(value_type: &str, enum_proven: bool) -> Option<(ValueKind, Option<usize>)> {
    let result = match value_type {
        "bool" => (ValueKind::Bool, Some(1)),
        "int8" => (ValueKind::Signed, Some(1)),
        "uint8" => (ValueKind::Unsigned, Some(1)),
        "int16" => (ValueKind::Signed, Some(2)),
        "uint16" => (ValueKind::Unsigned, Some(2)),
        "int" | "int32" => (ValueKind::Signed, Some(4)),
        "uint" | "uint32" => (ValueKind::Unsigned, Some(4)),
        "int64" => (ValueKind::Signed, Some(8)),
        "uint64" => (ValueKind::Unsigned, Some(8)),
        "float32" => (ValueKind::Float32, Some(4)),
        // This game build has AngelScript floatIsFloat64; both spellings occupy 8 bytes.
        "float" | "float64" | "double" => (ValueKind::Float64, Some(8)),
        _ if enum_proven => (ValueKind::Enum, None),
        _ => return None,
    };
    Some(result)
}

fn validate_raw_value(
    value_type: &str,
    value_width: usize,
    raw: &[u8],
    enum_proven: bool,
) -> Result<(), String> {
    let (kind, type_width) = classify_value_type(value_type, enum_proven)
        .ok_or_else(|| "field type is not a supported primitive or enum".to_owned())?;
    if let Some(type_width) = type_width {
        if type_width != value_width {
            return Err(format!(
                "field width {type_width} does not match bytecode store width {value_width}"
            ));
        }
    } else if !matches!(value_width, 1 | 2 | 4) {
        return Err(format!("enum store width {value_width} is unsupported"));
    }
    let required = if value_width == 8 { 8 } else { 4 };
    if raw.len() != required {
        return Err(format!(
            "serialized operand has {} bytes, expected {required}",
            raw.len()
        ));
    }
    if value_width < 4 && raw[value_width..].iter().any(|byte| *byte != 0) {
        return Err(format!(
            "narrow {value_width}-byte values require zero padding in the full dword operand"
        ));
    }
    if kind == ValueKind::Bool && raw != [0, 0, 0, 0] && raw != [1, 0, 0, 0] {
        return Err("bool operands must be canonical 0 or 1 dwords".into());
    }
    Ok(())
}

fn display_raw_value(
    value_type: &str,
    value_width: usize,
    raw: &[u8],
    enum_proven: bool,
) -> Option<String> {
    let (kind, _) = classify_value_type(value_type, enum_proven)?;
    let low = raw.get(..value_width)?;
    Some(match (kind, value_width) {
        (ValueKind::Bool, 1) => (low[0] != 0).to_string(),
        (ValueKind::Signed, 1) => i8::from_le_bytes([low[0]]).to_string(),
        (ValueKind::Signed, 2) => i16::from_le_bytes(low.try_into().ok()?).to_string(),
        (ValueKind::Signed, 4) => i32::from_le_bytes(low.try_into().ok()?).to_string(),
        (ValueKind::Signed, 8) => i64::from_le_bytes(low.try_into().ok()?).to_string(),
        (ValueKind::Unsigned | ValueKind::Enum, 1) => low[0].to_string(),
        (ValueKind::Unsigned | ValueKind::Enum, 2) => {
            u16::from_le_bytes(low.try_into().ok()?).to_string()
        }
        (ValueKind::Unsigned | ValueKind::Enum, 4) => {
            u32::from_le_bytes(low.try_into().ok()?).to_string()
        }
        (ValueKind::Unsigned, 8) => u64::from_le_bytes(low.try_into().ok()?).to_string(),
        (ValueKind::Float32, 4) => f32::from_le_bytes(low.try_into().ok()?).to_string(),
        (ValueKind::Float64, 8) => f64::from_le_bytes(low.try_into().ok()?).to_string(),
        _ => return None,
    })
}

fn verify_only_range_changed(
    before: &[u8],
    after: &[u8],
    start: usize,
    end: usize,
) -> Result<(), DefaultPatchError> {
    if before.len() != after.len() {
        return Err(DefaultPatchError::Postcondition(format!(
            "cache length changed from {} to {}",
            before.len(),
            after.len()
        )));
    }
    for (offset, (left, right)) in before.iter().zip(after).enumerate() {
        if left != right && !(start..end).contains(&offset) {
            return Err(DefaultPatchError::Postcondition(format!(
                "unexpected byte change at {offset:#x}"
            )));
        }
    }
    Ok(())
}

fn wire_error(error: super::wire::WireError) -> DefaultSiteError {
    DefaultSiteError::Wire(error.to_string())
}

pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0xf) as usize] as char);
    }
    result
}

fn sha256_hex(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::model::{Class, Field};

    fn word(opcode: u8, slot: u16) -> i32 {
        (opcode as u32 | ((slot as u32) << 16)) as i32
    }

    fn direct_v4(slot: u16, value: u32, offset: u16, type_id: u32) -> Vec<i32> {
        vec![
            word(77, slot),
            value as i32,
            word(178, offset),
            type_id as i32,
            word(90, slot),
        ]
    }

    fn module_with_class(module: &str, class: &str, super_class: Option<&str>) -> Module {
        Module {
            name: module.to_owned(),
            file: format!("{module}.as"),
            functions: Vec::new(),
            classes: vec![Class {
                name: class.to_owned(),
                super_class: super_class.map(str::to_owned),
                fields: Vec::new(),
                methods: Vec::new(),
                ctors: Vec::new(),
                flags: 0,
            }],
            enums: Vec::new(),
            globals: Vec::new(),
        }
    }

    #[test]
    fn direct_window_uses_load_this_word_as_offset_and_dword_as_type_id() {
        let code = direct_v4(7, 0x4120_0000, 128, 0x0400_121d);
        let instructions = disassemble(&code).unwrap();
        let windows = direct_windows(&code, &instructions);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].pattern, DefaultPattern::SetV4LoadThisWrtV4);
        assert_eq!(windows[0].member_offset, 128);
        assert_eq!(windows[0].owner_type_id, 0x0400_121d);
        assert_eq!(windows[0].operand_offset_dw, 1);
    }

    #[test]
    fn direct_window_requires_exact_adjacency_matching_width_and_slot() {
        let valid = direct_v4(7, 4, 128, 9);
        let mut wrong_slot = valid.clone();
        wrong_slot[4] = word(90, 8);
        let mut wrong_width = valid.clone();
        wrong_width[4] = word(91, 7);
        let mut interrupted = valid.clone();
        interrupted.insert(2, 0); // PopPtr between SetV4 and LoadThisR
        for code in [wrong_slot, wrong_width, interrupted] {
            let instructions = disassemble(&code).unwrap();
            assert!(direct_windows(&code, &instructions).is_empty());
        }
    }

    #[test]
    fn context_hash_ignores_only_value_operand() {
        let first = direct_v4(7, 4, 128, 9);
        let second = direct_v4(7, 99, 128, 9);
        let different_field = direct_v4(7, 4, 132, 9);
        let hash = |code: &[i32]| {
            let instructions = disassemble(code).unwrap();
            direct_windows(code, &instructions)[0]
                .context_sha256
                .clone()
        };
        assert_eq!(hash(&first), hash(&second));
        assert_ne!(hash(&first), hash(&different_field));
    }

    #[test]
    fn narrow_values_use_full_dword_cas_and_bool_is_canonical() {
        assert_eq!(DefaultPattern::SetV1LoadThisWrtV1.operand_width(), 4);
        assert!(validate_raw_value("bool", 1, &[0, 0, 0, 0], false).is_ok());
        assert!(validate_raw_value("bool", 1, &[1, 0, 0, 0], false).is_ok());
        assert!(validate_raw_value("bool", 1, &[2, 0, 0, 0], false).is_err());
        assert!(validate_raw_value("uint16", 2, &[0x34, 0x12, 0, 0], false).is_ok());
        assert!(validate_raw_value("uint8", 1, &[0x34, 1, 0, 0], false).is_err());
        assert!(validate_raw_value("uint16", 2, &[0x34, 0x12, 1, 0], false).is_err());
        assert!(validate_raw_value("ELooksLikeEnum", 4, &[1, 0, 0, 0], false).is_err());
        assert!(validate_raw_value("EProvenEnum", 4, &[1, 0, 0, 0], true).is_ok());
    }

    #[test]
    fn every_jump_opcode_is_a_fail_closed_initializer_gate() {
        for name in [
            "JMP", "JZ", "JNZ", "JS", "JNS", "JP", "JNP", "JMPP", "JLowZ", "JLowNZ",
        ] {
            assert!(is_branch(name), "{name}");
        }
        assert!(!is_branch("RET"));
        assert!(!is_branch("CALLSYS"));
    }

    #[test]
    fn only_one_terminal_ret_is_a_reachable_linear_initializer() {
        let mut valid = direct_v4(7, 4, 128, 9);
        valid.push(word(10, 0)); // RET
        let valid_instrs = disassemble(&valid).unwrap();
        assert!(is_reachable_linear_initializer(&valid_instrs));

        let no_ret = disassemble(&direct_v4(7, 4, 128, 9)).unwrap();
        assert!(!is_reachable_linear_initializer(&no_ret));

        let mut dead_after_ret = vec![word(10, 0)];
        dead_after_ret.extend(direct_v4(7, 4, 128, 9));
        let dead_after_ret = disassemble(&dead_after_ret).unwrap();
        assert!(!is_reachable_linear_initializer(&dead_after_ret));

        let mut two_returns = valid;
        two_returns.push(word(10, 0));
        let two_returns = disassemble(&two_returns).unwrap();
        assert!(!is_reachable_linear_initializer(&two_returns));

        let mut dead_after_throw = vec![word(212, 0)]; // ThrowException
        dead_after_throw.extend(direct_v4(7, 4, 128, 9));
        dead_after_throw.push(word(10, 0));
        let dead_after_throw = disassemble(&dead_after_throw).unwrap();
        assert!(!is_reachable_linear_initializer(&dead_after_throw));
    }

    #[test]
    fn initializer_return_must_be_canonical_plain_void() {
        let plain = super::super::types::DataType {
            token: 0x52,
            ..Default::default()
        };
        assert!(is_plain_void(&plain));

        for invalid in [
            super::super::types::DataType {
                is_reference: true,
                ..plain.clone()
            },
            super::super::types::DataType {
                is_object_const: true,
                ..plain.clone()
            },
            super::super::types::DataType {
                is_object_handle: true,
                ..plain.clone()
            },
            super::super::types::DataType {
                is_read_only: true,
                ..plain.clone()
            },
            super::super::types::DataType {
                is_auto: true,
                ..plain.clone()
            },
            super::super::types::DataType {
                if_handle_then_const: true,
                ..plain.clone()
            },
            super::super::types::DataType {
                type_info: 1,
                ..plain.clone()
            },
        ] {
            assert!(!is_plain_void(&invalid));
        }
    }

    #[test]
    fn class_hierarchy_proves_only_self_and_known_super_chain() {
        let modules = [
            module_with_class("Items", "UApple", Some("UFood")),
            module_with_class("Items.Base", "UFood", Some("UNativeItem")),
        ];
        let hierarchy = ClassHierarchy::build(&modules).unwrap();
        assert!(hierarchy.proves_ancestry("UApple", "UApple"));
        assert!(hierarchy.proves_ancestry("UApple", "UFood"));
        assert!(hierarchy.proves_ancestry("UApple", "UNativeItem"));
        assert!(!hierarchy.proves_ancestry("UApple", "UObject"));
        assert!(!hierarchy.proves_ancestry("Unknown", "Unknown"));
    }

    #[test]
    fn duplicate_bare_classes_and_hierarchy_cycles_fail_closed() {
        let duplicate = [
            module_with_class("First", "UShared", None),
            module_with_class("Second", "UShared", None),
        ];
        assert!(matches!(
            ClassHierarchy::build(&duplicate),
            Err(DefaultSiteError::DuplicateBareClass { .. })
        ));

        let cycle = [
            module_with_class("First", "UA", Some("UB")),
            module_with_class("Second", "UB", Some("UA")),
        ];
        assert!(matches!(
            ClassHierarchy::build(&cycle),
            Err(DefaultSiteError::CyclicClassHierarchy { .. })
        ));
    }

    #[test]
    fn duplicate_direct_fields_fail_before_resolver_flattening() {
        let mut module = module_with_class("Items", "UApple", None);
        module.classes[0].fields = vec![
            Field {
                name: "Value".into(),
                ty: super::super::types::DataType {
                    token: 0x44,
                    ..Default::default()
                },
                is_uproperty: false,
            },
            Field {
                name: "Value".into(),
                ty: super::super::types::DataType {
                    token: 0x50,
                    ..Default::default()
                },
                is_uproperty: false,
            },
        ];
        assert!(matches!(
            ClassHierarchy::build(&[module]),
            Err(DefaultSiteError::DuplicateDirectField { field, .. }) if field == "Value"
        ));
    }

    #[test]
    fn canonical_script_enum_type_binds_every_identity_component() {
        let base = TypeIdentity {
            name: "StatusKind".into(),
            module: "Items".into(),
            namespace: "State".into(),
        };
        let canonical = canonical_script_enum_type(&base);
        assert_ne!(
            canonical,
            canonical_script_enum_type(&TypeIdentity {
                namespace: "OtherState".into(),
                ..base.clone()
            })
        );
        assert_ne!(
            canonical,
            canonical_script_enum_type(&TypeIdentity {
                module: "OtherItems".into(),
                ..base.clone()
            })
        );
        assert_ne!(
            canonical,
            canonical_script_enum_type(&TypeIdentity {
                name: "OtherKind".into(),
                ..base
            })
        );
    }

    #[test]
    fn only_requested_byte_range_may_change() {
        let before = [0, 1, 2, 3, 4, 5];
        let mut after = before;
        after[2] = 9;
        assert!(verify_only_range_changed(&before, &after, 2, 3).is_ok());
        assert!(verify_only_range_changed(&before, &after, 3, 4).is_err());
    }
}
