//! Pure, bounded collision inventory for offline Quest authoring.
//!
//! The result is deliberately a conservative bare-name union. It is complete for the current
//! generator's module/path/bare-symbol checks, but it does not claim that methods, namespaced
//! functions, overloads, fields, and types share one AngelScript declaration scope.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::binds::{visit_collision_names, CollisionNameVisitError, NativeApi};
use super::emit_all::PreparedEmit;
use super::model::{self, Module};
use super::refs::RefResolver;
use super::tables::parse_tail_tables;
use super::walk_modules::{module_names, module_region_end};

pub const MAX_COLLISION_ENTRIES: usize = 100_000;
pub const MAX_COLLISION_ENTRY_BYTES: usize = 512;
pub const MAX_COLLISION_TOTAL_BYTES: usize = 16 * 1024 * 1024;

/// Canonical lowercase collision keys accepted directly by revision-2 Quest authoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionInventory {
    pub modules: BTreeSet<String>,
    pub relative_paths: BTreeSet<String>,
    pub symbols: BTreeSet<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CollisionInventoryError {
    #[error("invalid Shipping cache: {0}")]
    InvalidShippingCache(String),
    #[error("Shipping cache does not match its trusted generation seal")]
    ShippingSealMismatch,
    #[error("invalid or incomplete Binds cache")]
    InvalidBindsCache,
    #[error("Binds cache does not match its trusted generation seal")]
    BindsSealMismatch,
    #[error("Shipping module key {key:?} disagrees with embedded module name {embedded:?}")]
    ModuleIdentityMismatch { key: String, embedded: String },
    #[error("invalid Shipping module layout: {0}")]
    InvalidModuleLayout(String),
    #[error("collision {kind} entry is {actual} bytes; maximum is {max}")]
    EntryTooLong {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("collision inventory contains more than {max} entries")]
    TooManyEntries { max: usize },
    #[error("collision inventory exceeds {max} aggregate bytes")]
    TooManyBytes { max: usize },
}

/// Parse already-bounded Shipping and Binds buffers into a deterministic authoring inventory.
///
/// This function performs no filesystem access, compilation, game launch, deployment, or runtime
/// qualification. Both expected SHA-256 values must come from the trusted generation tuple; no
/// cache is parsed or materialized before both exact byte seals match. Callers remain responsible
/// for sealing the canonical artifact built from this result.
pub fn collect_collision_inventory(
    shipping_cache: &[u8],
    binds_cache: &[u8],
    expected_shipping_sha256: [u8; 32],
    expected_binds_sha256: [u8; 32],
) -> Result<CollisionInventory, CollisionInventoryError> {
    let actual_shipping_sha256: [u8; 32] = Sha256::digest(shipping_cache).into();
    if actual_shipping_sha256 != expected_shipping_sha256 {
        return Err(CollisionInventoryError::ShippingSealMismatch);
    }
    let actual_binds_sha256: [u8; 32] = Sha256::digest(binds_cache).into();
    if actual_binds_sha256 != expected_binds_sha256 {
        return Err(CollisionInventoryError::BindsSealMismatch);
    }

    let tail = module_region_end(shipping_cache)
        .map_err(|error| CollisionInventoryError::InvalidShippingCache(error.to_string()))?;
    let tables = parse_tail_tables(shipping_cache, tail)
        .map_err(|error| CollisionInventoryError::InvalidShippingCache(error.to_string()))?;
    if tables.end != shipping_cache.len() {
        return Err(CollisionInventoryError::InvalidShippingCache(format!(
            "tail tables end at {}, input has {} bytes",
            tables.end,
            shipping_cache.len()
        )));
    }

    let keys = module_names(shipping_cache)
        .map_err(|error| CollisionInventoryError::InvalidShippingCache(error.to_string()))?;
    let modules = model::parse_modules(shipping_cache)
        .map_err(|error| CollisionInventoryError::InvalidShippingCache(error.to_string()))?;
    let refs = RefResolver::build(shipping_cache)
        .map_err(|error| CollisionInventoryError::InvalidShippingCache(error.to_string()))?;
    let mut builder = InventoryBuilder::new(InventoryLimits::default());
    match visit_collision_names(binds_cache, |name| builder.insert_symbol(name)) {
        Ok(()) => {}
        Err(CollisionNameVisitError::InvalidBinds) => {
            return Err(CollisionInventoryError::InvalidBindsCache);
        }
        Err(CollisionNameVisitError::Visitor(error)) => return Err(error),
    }
    let mut tail_names = refs.collision_names().collect::<Vec<_>>();
    tail_names.sort_unstable();
    for name in tail_names {
        builder.insert_symbol(name)?;
    }
    let native =
        NativeApi::from_bytes(binds_cache).ok_or(CollisionInventoryError::InvalidBindsCache)?;
    collect_preparsed_with_builder(&keys, &modules, native, refs, builder)
}

#[derive(Debug, Clone, Copy)]
struct InventoryLimits {
    entries: usize,
    entry_bytes: usize,
    total_bytes: usize,
}

impl Default for InventoryLimits {
    fn default() -> Self {
        Self {
            entries: MAX_COLLISION_ENTRIES,
            entry_bytes: MAX_COLLISION_ENTRY_BYTES,
            total_bytes: MAX_COLLISION_TOTAL_BYTES,
        }
    }
}

#[cfg(test)]
fn collect_preparsed(
    keys: &[String],
    modules: &[Module],
    native: NativeApi,
    native_types: impl IntoIterator<Item = String>,
    refs: RefResolver,
    limits: InventoryLimits,
) -> Result<CollisionInventory, CollisionInventoryError> {
    let mut builder = InventoryBuilder::new(limits);
    let mut native_callables = native.collision_callable_names().collect::<Vec<_>>();
    native_callables.sort_unstable();
    for name in native_callables {
        builder.insert_symbol(name)?;
    }
    for name in native_types {
        builder.insert_symbol(&name)?;
    }
    let mut tail_names = refs.collision_names().collect::<Vec<_>>();
    tail_names.sort_unstable();
    for name in tail_names {
        builder.insert_symbol(name)?;
    }

    collect_preparsed_with_builder(keys, modules, native, refs, builder)
}

fn collect_preparsed_with_builder(
    keys: &[String],
    modules: &[Module],
    native: NativeApi,
    mut refs: RefResolver,
    mut builder: InventoryBuilder,
) -> Result<CollisionInventory, CollisionInventoryError> {
    if keys.len() != modules.len() {
        return Err(CollisionInventoryError::InvalidShippingCache(format!(
            "module key/model count mismatch: {} != {}",
            keys.len(),
            modules.len()
        )));
    }
    for (key, module) in keys.iter().zip(modules) {
        if key != &module.name {
            return Err(CollisionInventoryError::ModuleIdentityMismatch {
                key: key.clone(),
                embedded: module.name.clone(),
            });
        }
    }

    let prepared = PreparedEmit::new(modules, &mut refs, Some(native))
        .map_err(|error| CollisionInventoryError::InvalidModuleLayout(error.to_string()))?;

    for module in modules {
        builder.insert_module(&module.name)?;
        for function in &module.functions {
            builder.insert_symbol(&function.name)?;
        }
        for class in &module.classes {
            builder.insert_symbol(&class.name)?;
            if let Some(super_class) = &class.super_class {
                builder.insert_symbol(super_class)?;
            }
            for field in &class.fields {
                builder.insert_symbol(&field.name)?;
            }
            for function in class.methods.iter().chain(&class.ctors) {
                builder.insert_symbol(&function.name)?;
            }
        }
        for definition in &module.enums {
            builder.insert_symbol(&definition.name)?;
            for (name, _) in &definition.entries {
                builder.insert_symbol(name)?;
            }
        }
        for global in &module.globals {
            builder.insert_symbol(&global.name)?;
        }
    }
    for path in prepared.collision_relative_paths() {
        builder.insert_relative_path(path)?;
    }
    for name in prepared.collision_rename_names() {
        builder.insert_symbol(name)?;
    }

    Ok(builder.inventory)
}

struct InventoryBuilder {
    inventory: CollisionInventory,
    retained_entries: usize,
    retained_bytes: usize,
    limits: InventoryLimits,
}

impl InventoryBuilder {
    fn new(limits: InventoryLimits) -> Self {
        Self {
            inventory: CollisionInventory {
                modules: BTreeSet::new(),
                relative_paths: BTreeSet::new(),
                symbols: BTreeSet::new(),
            },
            retained_entries: 0,
            retained_bytes: 0,
            limits,
        }
    }

    fn insert_module(&mut self, value: &str) -> Result<(), CollisionInventoryError> {
        if safe_module(value) {
            self.insert(Domain::Module, value)?;
        }
        Ok(())
    }

    fn insert_relative_path(&mut self, value: &str) -> Result<(), CollisionInventoryError> {
        if safe_relative_path(value) {
            self.insert(Domain::RelativePath, value)?;
        }
        Ok(())
    }

    fn insert_symbol(&mut self, value: &str) -> Result<(), CollisionInventoryError> {
        if safe_bare_identifier(value) {
            self.insert(Domain::Symbol, value)?;
        }
        Ok(())
    }

    fn insert(&mut self, domain: Domain, value: &str) -> Result<(), CollisionInventoryError> {
        if value.len() > self.limits.entry_bytes {
            return Err(CollisionInventoryError::EntryTooLong {
                kind: domain.label(),
                actual: value.len(),
                max: self.limits.entry_bytes,
            });
        }
        let canonical = value.to_ascii_lowercase();
        let set = match domain {
            Domain::Module => &mut self.inventory.modules,
            Domain::RelativePath => &mut self.inventory.relative_paths,
            Domain::Symbol => &mut self.inventory.symbols,
        };
        if set.contains(&canonical) {
            return Ok(());
        }
        let next_entries = self.retained_entries.saturating_add(1);
        if next_entries > self.limits.entries {
            return Err(CollisionInventoryError::TooManyEntries {
                max: self.limits.entries,
            });
        }
        let next_bytes = self.retained_bytes.saturating_add(canonical.len());
        if next_bytes > self.limits.total_bytes {
            return Err(CollisionInventoryError::TooManyBytes {
                max: self.limits.total_bytes,
            });
        }
        set.insert(canonical);
        self.retained_entries = next_entries;
        self.retained_bytes = next_bytes;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Domain {
    Module,
    RelativePath,
    Symbol,
}

impl Domain {
    fn label(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::RelativePath => "relative path",
            Self::Symbol => "symbol",
        }
    }
}

fn safe_module(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value
            .split('.')
            .all(|segment| safe_bare_identifier(segment) && !is_reserved_portable_segment(segment))
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && !value.starts_with('/')
        && !value.contains('\\')
        && value.split('/').all(|segment| {
            let lowercase = segment.to_ascii_lowercase();
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && !is_reserved_portable_segment(lowercase.trim_end_matches(".as"))
                && (safe_bare_identifier(segment)
                    || lowercase
                        .strip_suffix(".as")
                        .is_some_and(safe_bare_identifier))
        })
}

fn safe_bare_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !value.starts_with("__")
        && !is_angelscript_reserved(value)
}

fn is_reserved_portable_segment(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
}

fn is_angelscript_reserved(value: &str) -> bool {
    const RESERVED: &[&str] = &[
        "abstract",
        "access",
        "and",
        "and_eq",
        "as",
        "auto",
        "bool",
        "break",
        "case",
        "cast",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "delegate",
        "do",
        "double",
        "else",
        "enum",
        "event",
        "explicit",
        "external",
        "false",
        "final",
        "float",
        "for",
        "from",
        "funcdef",
        "get",
        "if",
        "import",
        "in",
        "inout",
        "int",
        "int8",
        "int16",
        "int32",
        "int64",
        "interface",
        "is",
        "mixin",
        "namespace",
        "not",
        "not_eq",
        "null",
        "or",
        "or_eq",
        "out",
        "override",
        "private",
        "property",
        "protected",
        "return",
        "set",
        "shared",
        "super",
        "switch",
        "struct",
        "this",
        "true",
        "try",
        "typedef",
        "uint",
        "uint8",
        "uint16",
        "uint32",
        "uint64",
        "void",
        "while",
        "xor",
        "xor_eq",
    ];
    RESERVED
        .iter()
        .any(|reserved| value.eq_ignore_ascii_case(reserved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::header::CACHE_MAGIC;
    use crate::cache::model::{Class, EnumDef, Field, Func, Global, Param};
    use crate::cache::types::DataType;

    fn function(name: &str) -> Func {
        Func {
            name: name.to_owned(),
            namespace: String::new(),
            param_defaults: Vec::new(),
            ret: DataType::default(),
            params: Vec::<Param>::new(),
            bytecode: Vec::new(),
            variable_space: 0,
            obj_locals: Vec::new(),
            is_ufunction: false,
            traits: 0,
        }
    }

    fn module(name: &str, file: &str, function_name: &str) -> Module {
        Module {
            name: name.to_owned(),
            file: file.to_owned(),
            functions: vec![function(function_name)],
            classes: vec![Class {
                name: format!("U{name}"),
                namespace: String::new(),
                super_class: Some("UBaseType".to_owned()),
                fields: vec![Field {
                    name: "MemberValue".to_owned(),
                    ty: DataType::default(),
                    is_uproperty: false,
                }],
                methods: vec![function("MemberCall")],
                ctors: vec![function(&format!("U{name}"))],
                flags: 0,
            }],
            enums: vec![EnumDef {
                name: "EMode".to_owned(),
                namespace: String::new(),
                entries: vec![("ModeEntry".to_owned(), 0)],
            }],
            globals: vec![Global {
                name: "GlobalValue".to_owned(),
                namespace: String::new(),
                ty: DataType::default(),
                value: None,
                init: None,
            }],
        }
    }

    fn native() -> NativeApi {
        NativeApi::from_test_arities(&[], &[("NativeCall", Some(0))])
    }

    #[test]
    fn preparsed_inventory_covers_domains_and_final_renames_deterministically() {
        let modules = vec![
            module("Alpha", "Dir\\Alpha.as", "CollideFn"),
            module("Beta", "Dir/Beta.as", "CollideFn"),
        ];
        let keys = vec!["Alpha".to_owned(), "Beta".to_owned()];
        let first = collect_preparsed(
            &keys,
            &modules,
            native(),
            ["NativeType".to_owned()],
            RefResolver::from_test_collision_names(&["TailOnlyType"]),
            InventoryLimits::default(),
        )
        .unwrap();
        let second = collect_preparsed(
            &keys,
            &modules,
            native(),
            ["NativeType".to_owned()],
            RefResolver::from_test_collision_names(&["TailOnlyType"]),
            InventoryLimits::default(),
        )
        .unwrap();
        let without_tail = collect_preparsed(
            &keys,
            &modules,
            native(),
            ["NativeType".to_owned()],
            RefResolver::default(),
            InventoryLimits::default(),
        )
        .unwrap();
        assert_eq!(first, second);
        assert_ne!(first, without_tail);
        assert!(!without_tail.symbols.contains("tailonlytype"));
        assert_eq!(
            first.modules,
            BTreeSet::from(["alpha".into(), "beta".into()])
        );
        assert_eq!(
            first.relative_paths,
            BTreeSet::from(["dir/alpha.as".into(), "dir/beta.as".into()])
        );
        for symbol in [
            "collidefn",
            "collidefn_g0",
            "collidefn_g1",
            "ualpha",
            "ubasetype",
            "membervalue",
            "membercall",
            "emode",
            "modeentry",
            "globalvalue",
            "nativecall",
            "nativetype",
            "tailonlytype",
        ] {
            assert!(first.symbols.contains(symbol), "missing {symbol}");
        }
    }

    #[test]
    fn module_identity_and_casefold_path_aliases_fail_closed() {
        let modules = vec![module("Alpha", "Dir/Foo.as", "One")];
        let error = collect_preparsed(
            &["Other".to_owned()],
            &modules,
            native(),
            Vec::new(),
            RefResolver::default(),
            InventoryLimits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CollisionInventoryError::ModuleIdentityMismatch { .. }
        ));

        let aliases = vec![
            module("Alpha", "Dir/Foo.as", "One"),
            module("Beta", "dir/./foo.AS", "Two"),
        ];
        let error = collect_preparsed(
            &["Alpha".to_owned(), "Beta".to_owned()],
            &aliases,
            native(),
            Vec::new(),
            RefResolver::default(),
            InventoryLimits::default(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CollisionInventoryError::InvalidModuleLayout(_)
        ));
    }

    #[test]
    fn canonical_symbol_aliases_collapse_and_limits_never_retain_excess() {
        let limits = InventoryLimits {
            entries: 2,
            entry_bytes: 4,
            total_bytes: 6,
        };
        let mut builder = InventoryBuilder::new(limits);
        builder.insert_symbol("Name").unwrap();
        builder.insert_symbol("name").unwrap();
        assert_eq!(builder.retained_entries, 1);
        assert_eq!(builder.retained_bytes, 4);

        let error = builder.insert_symbol("Other").unwrap_err();
        assert!(matches!(
            error,
            CollisionInventoryError::EntryTooLong { .. }
        ));
        assert_eq!(builder.retained_entries, 1);
        assert_eq!(builder.retained_bytes, 4);

        builder.insert_symbol("Ab").unwrap();
        let error = builder.insert_symbol("Cd").unwrap_err();
        assert!(matches!(
            error,
            CollisionInventoryError::TooManyEntries { .. }
        ));
        assert_eq!(builder.retained_entries, 2);
        assert_eq!(builder.retained_bytes, 6);
    }

    #[test]
    fn path_extension_casefolds_before_authoring_filter() {
        let mut builder = InventoryBuilder::new(InventoryLimits::default());
        builder.insert_relative_path("Dir/Foo.AS").unwrap();
        assert_eq!(
            builder.inventory.relative_paths,
            BTreeSet::from(["dir/foo.as".into()])
        );
    }

    #[test]
    fn binds_candidates_stream_through_budget_before_native_api_materialization() {
        let binds = minimal_binds();
        let mut builder = InventoryBuilder::new(InventoryLimits {
            entries: 1,
            entry_bytes: MAX_COLLISION_ENTRY_BYTES,
            total_bytes: MAX_COLLISION_TOTAL_BYTES,
        });
        let error = visit_collision_names(&binds, |name| builder.insert_symbol(name)).unwrap_err();
        assert!(matches!(
            error,
            CollisionNameVisitError::Visitor(CollisionInventoryError::TooManyEntries { max: 1 })
        ));
        assert_eq!(builder.retained_entries, 1);
        assert_eq!(
            builder.inventory.symbols,
            BTreeSet::from(["unativetype".into()])
        );
    }

    fn push_sia(output: &mut Vec<u8>, value: &str) {
        if value.is_empty() {
            output.extend_from_slice(&0i32.to_le_bytes());
        } else {
            output.extend_from_slice(&(value.len() as i32).to_le_bytes());
            output.extend_from_slice(value.as_bytes());
            output.push(0);
        }
    }

    fn push_fstring(output: &mut Vec<u8>, value: &str) {
        output.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        output.extend_from_slice(value.as_bytes());
        output.push(0);
    }

    fn minimal_cache() -> Vec<u8> {
        let mut output = vec![0u8; 16];
        output.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        output.extend_from_slice(&1u32.to_le_bytes());
        push_fstring(&mut output, "Base.Module");
        push_sia(&mut output, "Base.Module");
        for _ in 0..5 {
            output.extend_from_slice(&0i32.to_le_bytes());
        }
        output.extend_from_slice(&0i64.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        push_sia(&mut output, "");
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        push_sia(&mut output, "Base/Module.as");
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&1i32.to_le_bytes());
        output.extend_from_slice(&101i64.to_le_bytes());
        push_sia(&mut output, "TailOnlyType");
        push_sia(&mut output, "");
        push_sia(&mut output, "");
        output.extend_from_slice(&0i32.to_le_bytes());
        for _ in 1..7 {
            output.extend_from_slice(&0i32.to_le_bytes());
        }
        output
    }

    fn push_cstr(output: &mut Vec<u8>, value: &str) {
        output.extend_from_slice(&((value.len() + 1) as u32).to_le_bytes());
        output.extend_from_slice(value.as_bytes());
        output.push(0);
    }

    fn minimal_binds() -> Vec<u8> {
        let mut output = 1u32.to_le_bytes().to_vec();
        push_cstr(&mut output, "UNativeType");
        push_cstr(&mut output, "/Script/Test.NativeType");
        output.extend_from_slice(&1u32.to_le_bytes());
        push_cstr(&mut output, "void NativeCall()");
        push_cstr(&mut output, "NativeCall");
        output.extend_from_slice(&[0u8; 32]);
        output
    }

    #[test]
    fn raw_collector_accepts_complete_buffers_and_rejects_corruption() {
        let cache = minimal_cache();
        let binds = minimal_binds();
        let shipping_seal: [u8; 32] = Sha256::digest(&cache).into();
        let binds_seal: [u8; 32] = Sha256::digest(&binds).into();
        let inventory =
            collect_collision_inventory(&cache, &binds, shipping_seal, binds_seal).unwrap();
        assert_eq!(inventory.modules, BTreeSet::from(["base.module".into()]));
        assert!(inventory.symbols.contains("unativetype"));
        assert!(inventory.symbols.contains("nativecall"));
        assert!(inventory.symbols.contains("tailonlytype"));

        let mut trailing = cache.clone();
        trailing.push(0);
        let trailing_seal: [u8; 32] = Sha256::digest(&trailing).into();
        assert!(matches!(
            collect_collision_inventory(&trailing, &binds, trailing_seal, binds_seal),
            Err(CollisionInventoryError::InvalidShippingCache(_))
        ));
        let truncated_binds = &binds[..8];
        let truncated_seal: [u8; 32] = Sha256::digest(truncated_binds).into();
        assert!(matches!(
            collect_collision_inventory(&cache, truncated_binds, shipping_seal, truncated_seal),
            Err(CollisionInventoryError::InvalidBindsCache)
        ));

        let mut impossible_count = binds.clone();
        impossible_count[..4].copy_from_slice(&2u32.to_le_bytes());
        let impossible_seal: [u8; 32] = Sha256::digest(&impossible_count).into();
        assert!(matches!(
            collect_collision_inventory(&cache, &impossible_count, shipping_seal, impossible_seal),
            Err(CollisionInventoryError::InvalidBindsCache)
        ));

        let mut tampered = binds.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert!(matches!(
            collect_collision_inventory(&cache, &tampered, shipping_seal, binds_seal),
            Err(CollisionInventoryError::BindsSealMismatch)
        ));

        let mut shipping_tamper = cache.clone();
        *shipping_tamper.last_mut().unwrap() ^= 1;
        assert!(matches!(
            collect_collision_inventory(&shipping_tamper, &binds, shipping_seal, binds_seal),
            Err(CollisionInventoryError::ShippingSealMismatch)
        ));
    }

    #[test]
    fn configured_real_cache_inventory_stays_within_authoring_bounds() {
        let (Some(cache_path), Some(binds_path)) = (
            std::env::var_os("GORE_AS_REAL_CACHE"),
            std::env::var_os("GORE_AS_BINDS"),
        ) else {
            return;
        };
        let cache = std::fs::read(cache_path).expect("read configured Shipping cache");
        let binds = std::fs::read(binds_path).expect("read configured Binds cache");
        let expected_shipping_sha256 = [
            0x10, 0x18, 0xf1, 0xcf, 0xe6, 0xb9, 0x9a, 0x65, 0x0e, 0xec, 0xb3, 0x3a, 0xfb, 0x96,
            0x75, 0x2d, 0x69, 0x1d, 0x20, 0x88, 0xea, 0xd2, 0x78, 0x08, 0x97, 0x1b, 0x81, 0x2f,
            0x04, 0xec, 0xb4, 0xc2,
        ];
        let expected_binds_sha256 = [
            0x46, 0xe6, 0x62, 0x9a, 0xd5, 0xca, 0xcc, 0x11, 0x2b, 0x99, 0x22, 0xd4, 0x8a, 0x1a,
            0xa9, 0x48, 0xf4, 0x05, 0x72, 0xd7, 0x28, 0x57, 0x05, 0xb9, 0x81, 0xc3, 0xec, 0xa3,
            0xdc, 0x61, 0x5f, 0xea,
        ];
        let first = collect_collision_inventory(
            &cache,
            &binds,
            expected_shipping_sha256,
            expected_binds_sha256,
        )
        .expect("collect real inventory");
        let second = collect_collision_inventory(
            &cache,
            &binds,
            expected_shipping_sha256,
            expected_binds_sha256,
        )
        .expect("repeat real inventory");
        assert_eq!(first, second);
        let count = first.modules.len() + first.relative_paths.len() + first.symbols.len();
        let bytes = first
            .modules
            .iter()
            .chain(&first.relative_paths)
            .chain(&first.symbols)
            .map(String::len)
            .sum::<usize>();
        assert!(count <= MAX_COLLISION_ENTRIES, "{count}");
        assert!(bytes <= MAX_COLLISION_TOTAL_BYTES, "{bytes}");
        assert!(first
            .modules
            .iter()
            .chain(&first.relative_paths)
            .chain(&first.symbols)
            .all(|value| value == &value.to_ascii_lowercase()));
        eprintln!(
            "real collision inventory: modules={}, paths={}, symbols={}, bytes={bytes}",
            first.modules.len(),
            first.relative_paths.len(),
            first.symbols.len()
        );
    }
}
