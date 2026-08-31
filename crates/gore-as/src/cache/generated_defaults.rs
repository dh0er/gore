//! Fail-closed carry-through for compiler-generated class-default methods.
//!
//! The source emitter never writes compiler-generated method bodies. When an edit authors no
//! class-scope `default` statements at all, recompilation therefore omits `__InitDefaults` and its
//! local `Class.MethodTable` slot. This fallback restores those pieces, plus every other executable
//! record omitted by the emitter inside the same module, only when the regenerated/remapped module
//! proves that every surrounding identity and layout is unchanged. Complete authored defaults
//! instead regenerate `__InitDefaults` and bypass this carry path. This module never decompiles or
//! synthesizes defaults.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use super::header::CacheHeader;
use super::model;
use super::refs::RefResolver;
use super::tables::parse_tail_tables;
use super::walk_modules::{module_count, module_ranges, module_region_end};
use super::wire::Cursor;

const MAX_RECORDS: usize = 2_000_000;
const MAX_CODE_DWORDS: usize = 20_000_000;
const DATA_TYPE_SIZE: usize = 36;
const MIN_MODULE_ENTRY_BYTES: usize = 60;
const MIN_FUNCTION_BYTES: usize = 120;
const MIN_CLASS_BYTES: usize = 64;
const MIN_PROPERTY_BYTES: usize = 52;
const MIN_ENUM_BYTES: usize = 16;
const MIN_GLOBAL_BYTES: usize = 48;
const MIN_IMPORT_BYTES: usize = 60;

#[derive(Clone, Debug)]
struct FunctionRecord {
    name: String,
    namespace: String,
    traits: i32,
    /// Signed per-build record identifier. This is not the positive T4 engine-ID domain and may
    /// drift for the same declaration between Shipping and regen; only collisions are forbidden.
    id: i32,
    raw: Range<usize>,
    /// Declaration bytes before FunctionTraits.
    declaration: Range<usize>,
    /// Name through FunctionTraits. This is the complete declaration identity before bytecode.
    signature: Range<usize>,
    /// bIsUFunction plus its optional metadata/flags, which follow body/debug data on the wire.
    ufunction_tail: Range<usize>,
}

#[derive(Clone, Debug)]
struct ClassRecord {
    name: String,
    namespace: String,
    /// Class identity, flags, and the complete property array, ending before Methods.Num.
    prefix: Range<usize>,
    methods_count_pos: usize,
    methods: Vec<FunctionRecord>,
    /// Count plus every local method index. Inherited slots may be `-1` and make this array much
    /// longer than Methods; it must be carried byte-exact rather than regenerated heuristically.
    method_table: Range<usize>,
    method_table_values: Vec<i32>,
    derived_and_shadow: Range<usize>,
    constructors: Vec<FunctionRecord>,
    factory_and_behavior_refs: Range<usize>,
    /// Count plus every compiler-generated behavior function. The emitter consumes these records
    /// for class reconstruction but cannot author them as source, so they are carried byte-exact.
    behaviors_block: Range<usize>,
    behaviors: Vec<FunctionRecord>,
    behavior_types: Range<usize>,
    preprocessor_tail: Range<usize>,
}

#[derive(Clone, Debug)]
struct GlobalRecord {
    /// Declaration through the Global.HasInitFunction discriminator.  A present initializer body
    /// is represented separately as a FunctionRecord and is intentionally not structural.
    declaration: Range<usize>,
    initializer: Option<FunctionRecord>,
}

#[derive(Clone, Debug)]
struct ModuleEntry {
    key: String,
    name: String,
    file: String,
    functions_count_pos: usize,
    functions: Vec<FunctionRecord>,
    functions_end: usize,
    classes: Vec<ClassRecord>,
    enums: Range<usize>,
    enum_entries: Vec<Range<usize>>,
    globals: Range<usize>,
    global_entries: Vec<GlobalRecord>,
    global_init_functions: Vec<FunctionRecord>,
    imports: Range<usize>,
    import_entries: Vec<Range<usize>>,
    imported_modules: Vec<Range<usize>>,
    statics_class: Range<usize>,
    declared_events: Vec<Range<usize>>,
    declared_delegates: Vec<Range<usize>>,
    post_init_functions: Vec<Range<usize>>,
    /// Everything after CodeHash (imports/statics/events/delegates/file/post-init names).
    post_code_hash: Range<usize>,
}

/// A base-bound, raw carry plan. Construction proves that the exact base module is internally
/// resolvable by strict self-remap and that every generated record is the supported default form.
#[derive(Clone, Debug)]
pub(crate) struct GeneratedDefaultsPlan {
    module_name: String,
    base_entry: Vec<u8>,
    base: ModuleEntry,
    /// Free-function indices omitted by the source emitter and therefore restored from the base.
    generated_free_indices: HashSet<usize>,
    /// Function IDs belonging to every untouched base module. Replacement output must not reuse
    /// any of them; Function.Id is cache-wide in the shipping cache, not module-local.
    outside_function_ids: HashMap<i32, String>,
    generated_count: usize,
    carry_mode: GeneratedDefaultsCarryMode,
}

/// Whether a defaults carry can use the historical strict keyspace or must retain the minimal
/// rows needed by an appended, authored class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GeneratedDefaultsCarryMode {
    Strict,
    HybridNewClass,
}

/// Shipping reflection metadata belongs to the existing declaration, not to the regenerated
/// body. The source emitter cannot spell every Unreal/AngelScript trait back into source (most
/// importantly UFUNCTION override/event flags), so an existing-module edit must restore that
/// metadata after remap while retaining the compiler's new bytecode and frame layout.
#[derive(Clone, Debug)]
pub(crate) struct ExistingFunctionMetadataPlan {
    module_name: String,
    base_entry: Vec<u8>,
    base: ModuleEntry,
}

/// A fail-closed structural guard for an existing-module edit.
///
/// The source emitter deliberately cannot reproduce every Shipping reflection detail.  Function
/// traits and UFUNCTION descriptors are repaired by [`ExistingFunctionMetadataPlan`], but that
/// leaves class and property metadata outside its scope.  A selective FullGraph edit may append
/// whole new classes/functions, yet it must not silently turn an edit of an existing class into a
/// reflection/layout edit.  This plan therefore proves that every pre-existing class remains the
/// unchanged ordered prefix of the regenerated module.  It copies nothing from the base: a drift
/// is an error.  New classes must be appended, which keeps their compiler-authored structure
/// intact while making an accidental rewrite of an old class unrepresentable.
#[derive(Clone, Debug)]
pub(crate) struct ExistingModuleStructurePlan {
    module_name: String,
    base_entry: Vec<u8>,
    base: ModuleEntry,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FunctionMetadataIdentity {
    owner: String,
    category: &'static str,
    declaration: Vec<u8>,
}

#[derive(Clone, Debug)]
struct FunctionMetadataSite {
    identity: FunctionMetadataIdentity,
    function: FunctionRecord,
}

impl ExistingModuleStructurePlan {
    pub(crate) fn prepare(base_cache: &[u8], module_name: &str) -> Result<Self, String> {
        let header = CacheHeader::parse(base_cache)
            .map_err(|error| format!("parsing module-structure base header: {error}"))?;
        let ranges = module_ranges(base_cache)
            .map_err(|error| format!("walking module-structure base modules: {error}"))?;
        if ranges.len() != header.type_count as usize {
            return Err(format!(
                "module-structure base header/walker module count mismatch: {}/{}",
                header.type_count,
                ranges.len()
            ));
        }
        let matches = ranges
            .iter()
            .filter(|(key, _, _)| key == module_name)
            .collect::<Vec<_>>();
        let [(map_key, start, end)] = matches.as_slice() else {
            return Err(format!(
                "module-structure preservation requires exactly one base module named \
                 {module_name:?}, found {}",
                matches.len()
            ));
        };
        let base_entry = base_cache
            .get(*start..*end)
            .ok_or_else(|| "module-structure base module range is out of bounds".to_string())?
            .to_vec();
        let base = parse_entry(&base_entry, "module-structure base module")?;
        if base.key != **map_key || base.name != module_name || base.key != base.name {
            return Err(format!(
                "module-structure base identity mismatch: requested {module_name:?}, map \
                 key/serialized key/name are {:?}/{:?}/{:?}",
                map_key, base.key, base.name
            ));
        }
        Ok(Self {
            module_name: module_name.to_string(),
            base_entry,
            base,
        })
    }

    /// Verify the fully remapped mini.  Call this after generated-default carry and function
    /// metadata normalization: those qualified repairs may restore a Shipping method table, but
    /// this plan itself never writes stale base bytes into the regenerated module.
    pub(crate) fn verify(&self, mini: &[u8]) -> Result<(), String> {
        let header = CacheHeader::parse(mini)
            .map_err(|error| format!("parsing module-structure mini header: {error}"))?;
        if header.type_count != 1 || module_count(mini) != 1 {
            return Err(format!(
                "module-structure preservation requires a one-module mini, found {} modules",
                header.type_count
            ));
        }
        let module_end = module_region_end(mini)
            .map_err(|error| format!("walking module-structure mini: {error}"))?;
        let tables = parse_tail_tables(mini, module_end)
            .map_err(|error| format!("parsing module-structure mini tail: {error}"))?;
        if tables.end != mini.len() {
            return Err(format!(
                "module-structure mini tail ends at {:#x}, file ends at {:#x}",
                tables.end,
                mini.len()
            ));
        }
        let regen_bytes = mini
            .get(CacheHeader::SIZE..module_end)
            .ok_or_else(|| "module-structure mini module range is invalid".to_string())?;
        let regen = parse_entry(regen_bytes, "module-structure mini module")?;
        if regen.key != self.base.key
            || regen.name != self.base.name
            || regen.file != self.base.file
            || regen.name != self.module_name
        {
            return Err(format!(
                "module-structure identity mismatch: base key/name/file {:?}/{:?}/{:?}, \
                 regenerated {:?}/{:?}/{:?}",
                self.base.key, self.base.name, self.base.file, regen.key, regen.name, regen.file
            ));
        }
        if regen.classes.len() < self.base.classes.len() {
            return Err(format!(
                "module-structure class count shrank in {:?}: base {}, regenerated {}",
                self.module_name,
                self.base.classes.len(),
                regen.classes.len()
            ));
        }
        if regen.functions.len() < self.base.functions.len() {
            return Err(format!(
                "module-structure free-function count shrank in {:?}: base {}, regenerated {}",
                self.module_name,
                self.base.functions.len(),
                regen.functions.len()
            ));
        }

        // A new complete class/function is legal. The compiler may place compiler-generated
        // free helpers (notably StaticClass) between Shipping functions, so free functions need
        // a stable-subsequence proof rather than an artificial append-only rule. Existing
        // declarations still may neither disappear nor reorder.
        compare_existing_free_function_subsequence(
            &self.base_entry,
            &self.base.functions,
            regen_bytes,
            &regen,
            self.base.classes.len(),
            "module free functions",
        )?;
        compare_structure_record_prefix(
            &self.base_entry,
            &self.base.enum_entries,
            regen_bytes,
            &regen.enum_entries,
            "module enums",
        )?;
        compare_global_record_prefix(
            &self.base_entry,
            &self.base.global_entries,
            regen_bytes,
            &regen.global_entries,
            "module globals",
        )?;
        compare_structure_record_prefix(
            &self.base_entry,
            &self.base.import_entries,
            regen_bytes,
            &regen.import_entries,
            "module imports",
        )?;
        compare_structure_record_prefix(
            &self.base_entry,
            &self.base.imported_modules,
            regen_bytes,
            &regen.imported_modules,
            "module imported-modules",
        )?;
        compare_structure_range(
            &self.base_entry,
            &self.base.statics_class,
            regen_bytes,
            &regen.statics_class,
            "module statics-class",
        )?;
        compare_structure_record_prefix(
            &self.base_entry,
            &self.base.declared_events,
            regen_bytes,
            &regen.declared_events,
            "module declared-events",
        )?;
        compare_structure_record_prefix(
            &self.base_entry,
            &self.base.declared_delegates,
            regen_bytes,
            &regen.declared_delegates,
            "module declared-delegates",
        )?;
        compare_structure_record_prefix(
            &self.base_entry,
            &self.base.post_init_functions,
            regen_bytes,
            &regen.post_init_functions,
            "module post-init-functions",
        )?;
        for (index, (base_class, regen_class)) in
            self.base.classes.iter().zip(&regen.classes).enumerate()
        {
            if base_class.name != regen_class.name || base_class.namespace != regen_class.namespace
            {
                return Err(format!(
                    "module-structure class identity/order drift at index {index}: base {}::{}, \
                     regenerated {}::{}",
                    base_class.namespace, base_class.name, regen_class.namespace, regen_class.name
                ));
            }
            compare_structure_range(
                &self.base_entry,
                &base_class.prefix,
                regen_bytes,
                &regen_class.prefix,
                &format!("class {} flags/properties", base_class.name),
            )?;
            compare_function_declarations(
                &self.base_entry,
                &base_class.methods,
                regen_bytes,
                &regen_class.methods,
                &format!("class {} methods", base_class.name),
            )?;
            compare_structure_range(
                &self.base_entry,
                &base_class.method_table,
                regen_bytes,
                &regen_class.method_table,
                &format!("class {} MethodTable", base_class.name),
            )?;
            compare_structure_range(
                &self.base_entry,
                &base_class.derived_and_shadow,
                regen_bytes,
                &regen_class.derived_and_shadow,
                &format!("class {} DerivedFrom/ShadowType", base_class.name),
            )?;
            compare_function_declarations(
                &self.base_entry,
                &base_class.constructors,
                regen_bytes,
                &regen_class.constructors,
                &format!("class {} constructors", base_class.name),
            )?;
            compare_structure_range(
                &self.base_entry,
                &base_class.factory_and_behavior_refs,
                regen_bytes,
                &regen_class.factory_and_behavior_refs,
                &format!("class {} factory/behavior refs", base_class.name),
            )?;
            compare_function_declarations(
                &self.base_entry,
                &base_class.behaviors,
                regen_bytes,
                &regen_class.behaviors,
                &format!("class {} behavior functions", base_class.name),
            )?;
            compare_structure_range(
                &self.base_entry,
                &base_class.behavior_types,
                regen_bytes,
                &regen_class.behavior_types,
                &format!("class {} behavior function types", base_class.name),
            )?;
            compare_structure_range(
                &self.base_entry,
                &base_class.preprocessor_tail,
                regen_bytes,
                &regen_class.preprocessor_tail,
                &format!("class {} preprocessor metadata", base_class.name),
            )?;
        }
        Ok(())
    }
}

impl ExistingFunctionMetadataPlan {
    pub(crate) fn prepare(base_cache: &[u8], module_name: &str) -> Result<Self, String> {
        let header = CacheHeader::parse(base_cache)
            .map_err(|error| format!("parsing function-metadata base header: {error}"))?;
        let ranges = module_ranges(base_cache)
            .map_err(|error| format!("walking function-metadata base modules: {error}"))?;
        if ranges.len() != header.type_count as usize {
            return Err(format!(
                "function-metadata base header/walker module count mismatch: {}/{}",
                header.type_count,
                ranges.len()
            ));
        }
        let matches = ranges
            .iter()
            .filter(|(key, _, _)| key == module_name)
            .collect::<Vec<_>>();
        let [(map_key, start, end)] = matches.as_slice() else {
            return Err(format!(
                "function-metadata preservation requires exactly one base module named \
                 {module_name:?}, found {}",
                matches.len()
            ));
        };
        let base_entry = base_cache
            .get(*start..*end)
            .ok_or_else(|| "function-metadata base module range is out of bounds".to_string())?
            .to_vec();
        let base = parse_entry(&base_entry, "function-metadata base module")?;
        if base.key != **map_key || base.name != module_name || base.key != base.name {
            return Err(format!(
                "function-metadata base identity mismatch: requested {module_name:?}, map \
                 key/serialized key/name are {:?}/{:?}/{:?}",
                map_key, base.key, base.name
            ));
        }
        function_metadata_sites(&base_entry, &base, "base")?;
        Ok(Self {
            module_name: module_name.to_string(),
            base_entry,
            base,
        })
    }

    pub(crate) fn apply(&self, remapped_mini: &[u8]) -> Result<Vec<u8>, String> {
        self.apply_inner(remapped_mini, true)
    }

    /// Normalize the Shipping metadata of every existing declaration already present in a
    /// regenerated mini. Missing base declarations are tolerated only for the intermediate
    /// generated-default carry step, which restores compiler-omitted records before [`apply`]
    /// performs the complete fail-closed check.
    pub(crate) fn apply_present(&self, remapped_mini: &[u8]) -> Result<Vec<u8>, String> {
        self.apply_inner(remapped_mini, false)
    }

    fn apply_inner(
        &self,
        remapped_mini: &[u8],
        require_all_existing: bool,
    ) -> Result<Vec<u8>, String> {
        let header = CacheHeader::parse(remapped_mini)
            .map_err(|error| format!("parsing remapped function-metadata mini: {error}"))?;
        if header.type_count != 1 || module_count(remapped_mini) != 1 {
            return Err(format!(
                "function-metadata preservation requires a one-module mini, found {} modules",
                header.type_count
            ));
        }
        let module_end = module_region_end(remapped_mini)
            .map_err(|error| format!("walking remapped function-metadata mini: {error}"))?;
        let tables = parse_tail_tables(remapped_mini, module_end)
            .map_err(|error| format!("parsing remapped function-metadata tail: {error}"))?;
        if tables.end != remapped_mini.len() {
            return Err(format!(
                "remapped function-metadata mini tail ends at {:#x}, file ends at {:#x}",
                tables.end,
                remapped_mini.len()
            ));
        }
        let regen_bytes = remapped_mini
            .get(CacheHeader::SIZE..module_end)
            .ok_or_else(|| "remapped function-metadata module range is invalid".to_string())?;
        let regen = parse_entry(regen_bytes, "remapped function-metadata module")?;
        if regen.key != self.base.key
            || regen.name != self.base.name
            || regen.file != self.base.file
            || regen.name != self.module_name
        {
            return Err(format!(
                "function-metadata module identity mismatch: base key/name/file \
                 {:?}/{:?}/{:?}, regenerated {:?}/{:?}/{:?}",
                self.base.key, self.base.name, self.base.file, regen.key, regen.name, regen.file
            ));
        }

        let base_sites = function_metadata_sites(&self.base_entry, &self.base, "base")?;
        let regen_sites = function_metadata_sites(regen_bytes, &regen, "regenerated")?;
        let base_by_identity = base_sites
            .iter()
            .map(|site| (&site.identity, site))
            .collect::<HashMap<_, _>>();
        let regen_by_identity = regen_sites
            .iter()
            .map(|site| (&site.identity, site))
            .collect::<HashMap<_, _>>();
        if require_all_existing {
            for base in &base_sites {
                if !regen_by_identity.contains_key(&base.identity) {
                    return Err(format!(
                        "regenerated module is missing existing {} identity in {}",
                        base.identity.category, base.identity.owner
                    ));
                }
            }
        }

        let mut replacements = Vec::<(Range<usize>, Vec<u8>)>::new();
        for regenerated in &regen_sites {
            let Some(base) = base_by_identity.get(&regenerated.identity) else {
                // A declaration absent from Shipping is genuinely new (including methods of a
                // new class). Its compiler-authored traits and UFUNCTION tail remain untouched.
                continue;
            };
            let mut replacement = Vec::new();
            replacement.extend_from_slice(range_bytes(
                regen_bytes,
                regenerated.function.raw.start..regenerated.function.signature.end - 4,
                "regenerated function declaration",
            )?);
            replacement.extend_from_slice(range_bytes(
                &self.base_entry,
                base.function.signature.end - 4..base.function.signature.end,
                "base FunctionTraits",
            )?);
            replacement.extend_from_slice(range_bytes(
                regen_bytes,
                regenerated.function.signature.end..regenerated.function.ufunction_tail.start,
                "regenerated function body/frame",
            )?);
            replacement.extend_from_slice(range_bytes(
                &self.base_entry,
                base.function.ufunction_tail.clone(),
                "base UFUNCTION tail",
            )?);
            replacements.push((regenerated.function.raw.clone(), replacement));
        }
        replacements.sort_by_key(|(range, _)| range.start);
        for pair in replacements.windows(2) {
            if pair[0].0.end > pair[1].0.start {
                return Err("function-metadata replacement ranges overlap".into());
            }
        }

        let mut rebuilt = Vec::with_capacity(regen_bytes.len());
        let mut cursor = 0usize;
        for (range, replacement) in &replacements {
            rebuilt.extend_from_slice(range_bytes(
                regen_bytes,
                cursor..range.start,
                "function-metadata rebuild prefix",
            )?);
            rebuilt.extend_from_slice(replacement);
            cursor = range.end;
        }
        rebuilt.extend_from_slice(range_bytes(
            regen_bytes,
            cursor..regen_bytes.len(),
            "function-metadata rebuild suffix",
        )?);

        let mut out = Vec::with_capacity(
            CacheHeader::SIZE + rebuilt.len() + remapped_mini.len() - module_end,
        );
        out.extend_from_slice(&remapped_mini[..CacheHeader::SIZE]);
        out.extend_from_slice(&rebuilt);
        out.extend_from_slice(&remapped_mini[module_end..]);
        self.verify(regen_bytes, &regen_sites, &out, require_all_existing)?;
        Ok(out)
    }

    fn verify(
        &self,
        regen_bytes: &[u8],
        regen_sites: &[FunctionMetadataSite],
        output: &[u8],
        require_all_existing: bool,
    ) -> Result<(), String> {
        let output_end = module_region_end(output)
            .map_err(|error| format!("walking function-metadata output: {error}"))?;
        let output_header = CacheHeader::parse(output)
            .map_err(|error| format!("parsing function-metadata output header: {error}"))?;
        if output_header.type_count != 1 || module_count(output) != 1 {
            return Err("function-metadata output is not a one-module mini".into());
        }
        let output_tables = parse_tail_tables(output, output_end)
            .map_err(|error| format!("parsing function-metadata output tail: {error}"))?;
        if output_tables.end != output.len() {
            return Err(format!(
                "function-metadata output tail ends at {:#x}, file ends at {:#x}",
                output_tables.end,
                output.len()
            ));
        }
        let output_bytes = output
            .get(CacheHeader::SIZE..output_end)
            .ok_or_else(|| "function-metadata output module range is invalid".to_string())?;
        let output_entry = parse_entry(output_bytes, "function-metadata output")?;
        let output_sites = function_metadata_sites(output_bytes, &output_entry, "output")?;
        let output_by_identity = output_sites
            .iter()
            .map(|site| (&site.identity, site))
            .collect::<HashMap<_, _>>();
        let base_sites =
            function_metadata_sites(&self.base_entry, &self.base, "base verification")?;
        let base_by_identity = base_sites
            .iter()
            .map(|site| (&site.identity, site))
            .collect::<HashMap<_, _>>();
        if require_all_existing {
            for base in &base_sites {
                if !output_by_identity.contains_key(&base.identity) {
                    return Err(format!(
                        "function-metadata output is missing existing {} identity in {}",
                        base.identity.category, base.identity.owner
                    ));
                }
            }
        }

        for regenerated in regen_sites {
            let output = output_by_identity
                .get(&regenerated.identity)
                .ok_or_else(|| {
                    format!(
                        "function-metadata output lost {} {}",
                        regenerated.identity.category, regenerated.identity.owner
                    )
                })?;
            if let Some(base) = base_by_identity.get(&regenerated.identity) {
                compare_range(
                    &self.base_entry,
                    &(base.function.signature.end - 4..base.function.signature.end),
                    output_bytes,
                    &(output.function.signature.end - 4..output.function.signature.end),
                    "preserved FunctionTraits",
                )?;
                compare_range(
                    &self.base_entry,
                    &base.function.ufunction_tail,
                    output_bytes,
                    &output.function.ufunction_tail,
                    "preserved UFUNCTION tail",
                )?;
                compare_range(
                    regen_bytes,
                    &(regenerated.function.signature.end
                        ..regenerated.function.ufunction_tail.start),
                    output_bytes,
                    &(output.function.signature.end..output.function.ufunction_tail.start),
                    "regenerated function body/frame",
                )?;
            } else {
                compare_range(
                    regen_bytes,
                    &regenerated.function.raw,
                    output_bytes,
                    &output.function.raw,
                    "new function record",
                )?;
            }
        }
        Ok(())
    }
}

fn function_metadata_sites(
    bytes: &[u8],
    entry: &ModuleEntry,
    context: &str,
) -> Result<Vec<FunctionMetadataSite>, String> {
    let mut sites = Vec::new();
    let mut class_identities = HashSet::new();
    for class in &entry.classes {
        if !class_identities.insert((class.namespace.as_str(), class.name.as_str())) {
            return Err(format!(
                "function-metadata {context} contains duplicate class identity {}::{}",
                class.namespace, class.name
            ));
        }
    }
    let mut add = |owner: String,
                   category: &'static str,
                   function: &FunctionRecord|
     -> Result<(), String> {
        sites.push(FunctionMetadataSite {
            identity: FunctionMetadataIdentity {
                owner,
                category,
                declaration: range_bytes(bytes, function.declaration.clone(), "function identity")?
                    .to_vec(),
            },
            function: function.clone(),
        });
        Ok(())
    };
    for function in &entry.functions {
        add(entry.name.clone(), "free function", function)?;
    }
    for class in &entry.classes {
        let owner = format!("{}::{}", class.namespace, class.name);
        for function in &class.methods {
            add(owner.clone(), "method", function)?;
        }
        for function in &class.constructors {
            add(owner.clone(), "constructor", function)?;
        }
        for function in &class.behaviors {
            add(owner.clone(), "behavior", function)?;
        }
    }
    for function in &entry.global_init_functions {
        add(entry.name.clone(), "global initializer", function)?;
    }
    let mut identities = HashSet::new();
    for site in &sites {
        if !identities.insert(site.identity.clone()) {
            return Err(format!(
                "function-metadata {context} contains ambiguous duplicate {} identity in {}",
                site.identity.category, site.identity.owner
            ));
        }
    }
    Ok(sites)
}

fn range_bytes<'a>(bytes: &'a [u8], range: Range<usize>, what: &str) -> Result<&'a [u8], String> {
    bytes
        .get(range)
        .ok_or_else(|| format!("function-metadata {what} range is invalid"))
}

impl GeneratedDefaultsPlan {
    /// Build a plan for exactly one existing module. `None` means the module has no omitted
    /// generated methods and needs no carry-through.
    pub(crate) fn prepare(
        base_cache: &[u8],
        modules: &[model::Module],
        module_name: &str,
    ) -> Result<Option<Self>, String> {
        let header = CacheHeader::parse(base_cache)
            .map_err(|error| format!("parsing generated-default base header: {error}"))?;
        let minimum_len = CacheHeader::SIZE
            .checked_add(
                (header.type_count as usize)
                    .checked_mul(MIN_MODULE_ENTRY_BYTES)
                    .ok_or_else(|| {
                        "generated-default base module-count size overflow".to_string()
                    })?,
            )
            .and_then(|value| value.checked_add(super::tables::N_TABLES * 4))
            .ok_or_else(|| "generated-default base minimum size overflow".to_string())?;
        if base_cache.len() < minimum_len {
            return Err(format!(
                "generated-default base declares {} modules but has only {} bytes (minimum {})",
                header.type_count,
                base_cache.len(),
                minimum_len
            ));
        }
        let base_module_end = module_region_end(base_cache)
            .map_err(|error| format!("walking generated-default base modules: {error}"))?;
        let base_tables = parse_tail_tables(base_cache, base_module_end)
            .map_err(|error| format!("parsing generated-default base tail: {error}"))?;
        if base_tables.tables.len() != super::tables::N_TABLES
            || base_tables.end != base_cache.len()
        {
            return Err(format!(
                "generated-default base must contain exactly seven tail tables ending at EOF; \
                 tail ended at {:#x}, file ends at {:#x}",
                base_tables.end,
                base_cache.len()
            ));
        }

        let indices = modules
            .iter()
            .enumerate()
            .filter_map(|(index, module)| (module.name == module_name).then_some(index))
            .collect::<Vec<_>>();
        let [module_index] = indices.as_slice() else {
            return Err(format!(
                "generated-default carry requires exactly one base module named \
                 {module_name:?}, found {}",
                indices.len()
            ));
        };

        let ranges = module_ranges(base_cache)
            .map_err(|error| format!("walking base modules for generated defaults: {error}"))?;
        if ranges.len() != header.type_count as usize {
            return Err(format!(
                "generated-default base header/walker module count mismatch: {}/{}",
                header.type_count,
                ranges.len()
            ));
        }
        if ranges.len() != modules.len() {
            return Err(format!(
                "generated-default base identity mismatch: raw walker found {} modules but model \
                 parser found {}",
                ranges.len(),
                modules.len()
            ));
        }
        let (map_key, start, end) = ranges
            .get(*module_index)
            .ok_or_else(|| "generated-default base module index is out of range".to_string())?;
        let entry_bytes = base_cache
            .get(*start..*end)
            .ok_or_else(|| "generated-default base module range is out of bounds".to_string())?
            .to_vec();
        let entry = parse_entry(&entry_bytes, "base module")?;
        if entry.name != module_name || entry.key != *map_key {
            return Err(format!(
                "generated-default base identity mismatch: requested {module_name:?}, map key is \
                 {map_key:?}, serialized key/name are {:?}/{:?}",
                entry.key, entry.name
            ));
        }
        // compile-module's extraction path addresses the regenerated TMap by this same target
        // string. A differing map key/inner name is not safe to guess around.
        if entry.key != entry.name {
            return Err(format!(
                "generated-default carry does not support differing module key/name {:?}/{:?}",
                entry.key, entry.name
            ));
        }
        if entry.file != modules[*module_index].file {
            return Err(format!(
                "generated-default source identity drift in base module {module_name:?}: raw file \
                 {:?}, parsed file {:?}",
                entry.file, modules[*module_index].file
            ));
        }
        validate_model_identity(&entry, &modules[*module_index], module_name)?;

        let mut generated_count = 0usize;
        let mut class_names = HashSet::new();
        for class in &entry.classes {
            if !class_names.insert(class.name.as_str()) {
                return Err(format!(
                    "generated-default carry found duplicate class identity {}::{:?}",
                    module_name, class.name
                ));
            }
            validate_method_table(class, "base")?;
            let mut generated_names = HashSet::new();
            for (index, method) in class.methods.iter().enumerate() {
                if !method.name.starts_with("__") {
                    continue;
                }
                if method.name != "__InitDefaults" {
                    return Err(format!(
                        "generated-default carry supports only __InitDefaults, found \
                         {}::{}::{}",
                        module_name, class.name, method.name
                    ));
                }
                if !generated_names.insert(method.name.as_str()) {
                    return Err(format!(
                        "generated-default carry found duplicate method identity \
                         {}::{}::{}",
                        module_name, class.name, method.name
                    ));
                }
                if !class.method_table_values.contains(&(index as i32)) {
                    return Err(format!(
                        "generated-default method {}::{}::{} is absent from its base MethodTable",
                        module_name, class.name, method.name
                    ));
                }
                generated_count += 1;
            }
        }
        if generated_count == 0 {
            return Ok(None);
        }
        validate_unique_function_ids(&entry, "base")?;
        let outside_function_ids =
            validate_cache_wide_function_ids(base_cache, &ranges, *module_index)?;
        let refs = RefResolver::build(base_cache)
            .map_err(|error| format!("building generated-default reference resolver: {error}"))?;
        let generated_free_indices = entry
            .functions
            .iter()
            .zip(&modules[*module_index].functions)
            .enumerate()
            .filter_map(|(index, (raw, parsed))| {
                (is_generated_free_shape(&entry, raw)
                    || super::emit::is_generated_spawn(parsed, &refs))
                .then_some(index)
            })
            .collect::<HashSet<_>>();

        // Resolve every embedded/bytecode reference in the exact vanilla module against the exact
        // vanilla tail tables. A strict self-remap must be a byte-exact no-op on the module entry;
        // this rejects missing rows/unresolved refs before any game callback is possible.
        let extracted = super::splice::extract_module(base_cache, &entry.key)
            .map_err(|error| format!("extracting base module for defaults proof: {error}"))?;
        let (self_remapped, _) = super::remap::remap_module_to_base(&extracted, base_cache)
            .map_err(|error| {
                format!("base generated-default references are unresolved: {error}")
            })?;
        let self_end = module_region_end(&self_remapped)
            .map_err(|error| format!("walking self-remapped defaults proof: {error}"))?;
        let self_entry = self_remapped
            .get(CacheHeader::SIZE..self_end)
            .ok_or_else(|| "self-remapped defaults proof range is out of bounds".to_string())?;
        if self_entry != entry_bytes {
            return Err(format!(
                "strict self-remap changed base module {module_name:?}; generated records are not \
                 proven byte-exact in the base keyspace"
            ));
        }

        Ok(Some(Self {
            module_name: module_name.to_owned(),
            base_entry: entry_bytes,
            base: entry,
            generated_free_indices,
            outside_function_ids,
            generated_count,
            carry_mode: GeneratedDefaultsCarryMode::Strict,
        }))
    }

    /// Prepare a carry for a source overlay that adds defaults only for appended classes.
    /// Existing base classes remain byte-exact carries; their authored defaults would make that
    /// carry stale and are therefore a hard error rather than an implicit mixed policy.
    pub(crate) fn prepare_hybrid(
        base_cache: &[u8],
        modules: &[model::Module],
        module_name: &str,
        authored_defaults: &HashSet<String>,
    ) -> Result<Option<Self>, String> {
        let mut plan = match Self::prepare(base_cache, modules, module_name)? {
            Some(plan) => plan,
            None => return Ok(None),
        };
        if let Some(class) = plan
            .base
            .classes
            .iter()
            .find(|class| authored_defaults.contains(&class.name))
        {
            return Err(format!(
                "hybrid default carry refuses authored defaults for existing base class {}::{}",
                module_name, class.name
            ));
        }
        plan.carry_mode = GeneratedDefaultsCarryMode::HybridNewClass;
        Ok(Some(plan))
    }

    pub(crate) fn generated_count(&self) -> usize {
        self.generated_count
    }

    pub(crate) fn allows_new_symbols(&self) -> bool {
        self.carry_mode == GeneratedDefaultsCarryMode::HybridNewClass
    }

    /// Carry base generated records into a one-module mini after strict remap. Every declaration,
    /// class-layout, and metadata gate is checked before output construction; post-parse then
    /// proves exact generated/non-generated record provenance and MethodTable preservation.
    pub(crate) fn apply(&self, remapped_mini: &[u8]) -> Result<Vec<u8>, String> {
        match self.carry_mode {
            GeneratedDefaultsCarryMode::Strict => self.apply_strict(remapped_mini),
            GeneratedDefaultsCarryMode::HybridNewClass => self.apply_hybrid(remapped_mini),
        }
    }

    fn apply_strict(&self, remapped_mini: &[u8]) -> Result<Vec<u8>, String> {
        let mini_header = CacheHeader::parse(remapped_mini)
            .map_err(|error| format!("parsing remapped defaults mini header: {error}"))?;
        if mini_header.type_count != 1 || module_count(remapped_mini) != 1 {
            return Err(format!(
                "generated-default carry requires a one-module mini, found {} modules",
                mini_header.type_count
            ));
        }
        let module_end = module_region_end(remapped_mini)
            .map_err(|error| format!("walking remapped defaults mini: {error}"))?;
        let tables = parse_tail_tables(remapped_mini, module_end)
            .map_err(|error| format!("parsing remapped defaults mini tail: {error}"))?;
        if tables.end != remapped_mini.len() {
            return Err(format!(
                "remapped defaults mini tail ends at {:#x}, file ends at {:#x}",
                tables.end,
                remapped_mini.len()
            ));
        }
        // Strict remap emits seven empty tables. Requiring that exact shape prevents a caller from
        // smuggling opt-in new-symbol rows into a carry operation built from old class defaults.
        if remapped_mini[module_end..].iter().any(|&byte| byte != 0) {
            return Err(
                "generated-default carry requires strict remap with seven empty tail tables".into(),
            );
        }

        let regen_entry_bytes = remapped_mini
            .get(CacheHeader::SIZE..module_end)
            .ok_or_else(|| "remapped defaults module range is out of bounds".to_string())?;
        let regen = parse_entry(regen_entry_bytes, "strict-remapped module")?;
        self.validate_regenerated(regen_entry_bytes, &regen)?;

        let generated_method_bytes = self
            .base
            .classes
            .iter()
            .flat_map(|class| class.methods.iter())
            .filter(|method| method.name.starts_with("__"))
            .try_fold(0usize, |total, method| total.checked_add(method.raw.len()))
            .ok_or_else(|| "defaults rebuild generated-method size overflow".to_string())?;
        let behavior_bytes = self
            .base
            .classes
            .iter()
            .try_fold(0usize, |total, class| {
                total.checked_add(class.behaviors_block.len())
            })
            .ok_or_else(|| "defaults rebuild behavior size overflow".to_string())?;
        let rebuild_capacity = regen_entry_bytes
            .len()
            .checked_add(generated_method_bytes)
            .and_then(|value| value.checked_add(behavior_bytes))
            .ok_or_else(|| "defaults rebuild capacity overflow".to_string())?;
        let mut rebuilt_entry = Vec::new();
        rebuilt_entry
            .try_reserve_exact(rebuild_capacity)
            .map_err(|error| {
                format!("reserving generated-default rebuild ({rebuild_capacity} bytes): {error}")
            })?;
        let has_generated_free = !self.generated_free_indices.is_empty();
        let mut cursor = if has_generated_free {
            rebuilt_entry.extend_from_slice(
                regen_entry_bytes
                    .get(..regen.functions_count_pos)
                    .ok_or_else(|| "defaults rebuild function prefix is invalid".to_string())?,
            );
            let function_count = i32::try_from(self.base.functions.len())
                .map_err(|_| "base free-function count does not fit i32".to_string())?;
            rebuilt_entry.extend_from_slice(&function_count.to_le_bytes());
            for (index, (base_function, regen_function)) in
                self.base.functions.iter().zip(&regen.functions).enumerate()
            {
                let (source, range) = if self.generated_free_indices.contains(&index) {
                    (self.base_entry.as_slice(), &base_function.raw)
                } else {
                    (regen_entry_bytes, &regen_function.raw)
                };
                rebuilt_entry.extend_from_slice(source.get(range.clone()).ok_or_else(|| {
                    "defaults rebuild free-function range is invalid".to_string()
                })?);
            }
            regen.functions_end
        } else {
            0
        };
        for (base_class, regen_class) in self.base.classes.iter().zip(&regen.classes) {
            let carries_defaults = base_class
                .methods
                .iter()
                .any(|method| method.name.starts_with("__"));
            if carries_defaults {
                rebuilt_entry.extend_from_slice(
                    regen_entry_bytes
                        .get(cursor..regen_class.methods_count_pos)
                        .ok_or_else(|| {
                            "defaults rebuild class prefix range is invalid".to_string()
                        })?,
                );
                let method_count = i32::try_from(base_class.methods.len())
                    .map_err(|_| "base method count does not fit i32".to_string())?;
                rebuilt_entry.extend_from_slice(&method_count.to_le_bytes());

                let mut regen_non_generated = regen_class.methods.iter();
                for base_method in &base_class.methods {
                    let (source, range) = if base_method.name.starts_with("__") {
                        (self.base_entry.as_slice(), &base_method.raw)
                    } else {
                        let regen_method = regen_non_generated.next().ok_or_else(|| {
                            "defaults rebuild ran out of regenerated non-generated methods"
                                .to_string()
                        })?;
                        (regen_entry_bytes, &regen_method.raw)
                    };
                    rebuilt_entry.extend_from_slice(
                        source.get(range.clone()).ok_or_else(|| {
                            "defaults rebuild method range is invalid".to_string()
                        })?,
                    );
                }
                if regen_non_generated.next().is_some() {
                    return Err(
                        "defaults rebuild left unconsumed regenerated non-generated methods".into(),
                    );
                }
                rebuilt_entry.extend_from_slice(
                    self.base_entry
                        .get(base_class.method_table.clone())
                        .ok_or_else(|| "base defaults MethodTable range is invalid".to_string())?,
                );
                cursor = regen_class.method_table.end;
            }

            if !base_class.behaviors.is_empty() {
                rebuilt_entry.extend_from_slice(
                    regen_entry_bytes
                        .get(cursor..regen_class.behaviors_block.start)
                        .ok_or_else(|| {
                            "defaults rebuild behavior prefix range is invalid".to_string()
                        })?,
                );
                rebuilt_entry.extend_from_slice(
                    self.base_entry
                        .get(base_class.behaviors_block.clone())
                        .ok_or_else(|| "base behavior block range is invalid".to_string())?,
                );
                cursor = regen_class.behaviors_block.end;
            }
        }
        rebuilt_entry.extend_from_slice(
            regen_entry_bytes
                .get(cursor..)
                .ok_or_else(|| "defaults rebuild final module range is invalid".to_string())?,
        );

        // Keep the regenerated Module.CodeHash in the copied suffix. It describes the authored
        // compile generation and is not a serialized-byte integrity checksum (module splicing
        // already preserves arbitrary per-module hashes); copying the vanilla hash would falsely
        // label edited ordinary bodies as the old source generation.
        let mut out = Vec::with_capacity(
            CacheHeader::SIZE + rebuilt_entry.len() + (remapped_mini.len() - module_end),
        );
        out.extend_from_slice(&remapped_mini[..CacheHeader::SIZE]);
        out.extend_from_slice(&rebuilt_entry);
        out.extend_from_slice(&remapped_mini[module_end..]);
        self.verify_output(remapped_mini, regen_entry_bytes, &out)?;
        Ok(out)
    }

    /// Carry only the omitted generated records of pre-existing classes.  Appended classes and
    /// their generated `__InitDefaults` remain exactly as the compiler produced them, as do the
    /// minimal new-symbol rows in the mini tail.
    fn apply_hybrid(&self, remapped_mini: &[u8]) -> Result<Vec<u8>, String> {
        let mini_header = CacheHeader::parse(remapped_mini)
            .map_err(|error| format!("parsing hybrid defaults mini header: {error}"))?;
        if mini_header.type_count != 1 || module_count(remapped_mini) != 1 {
            return Err(format!(
                "hybrid generated-default carry requires a one-module mini, found {} modules",
                mini_header.type_count
            ));
        }
        let module_end = module_region_end(remapped_mini)
            .map_err(|error| format!("walking hybrid defaults mini: {error}"))?;
        let tables = parse_tail_tables(remapped_mini, module_end)
            .map_err(|error| format!("parsing hybrid defaults mini tail: {error}"))?;
        if tables.end != remapped_mini.len() {
            return Err(format!(
                "hybrid defaults mini tail ends at {:#x}, file ends at {:#x}",
                tables.end,
                remapped_mini.len()
            ));
        }
        let regen_entry_bytes = remapped_mini
            .get(CacheHeader::SIZE..module_end)
            .ok_or_else(|| "hybrid defaults module range is out of bounds".to_string())?;
        let regen = parse_entry(regen_entry_bytes, "hybrid-remapped module")?;
        let free_matches = self.validate_hybrid_regenerated(regen_entry_bytes, &regen)?;

        let generated_method_bytes = self
            .base
            .classes
            .iter()
            .flat_map(|class| class.methods.iter())
            .filter(|method| method.name.starts_with("__"))
            .try_fold(0usize, |total, method| total.checked_add(method.raw.len()))
            .ok_or_else(|| "hybrid defaults generated-method size overflow".to_string())?;
        let behavior_bytes = self
            .base
            .classes
            .iter()
            .try_fold(0usize, |total, class| {
                total.checked_add(class.behaviors_block.len())
            })
            .ok_or_else(|| "hybrid defaults behavior size overflow".to_string())?;
        let rebuild_capacity = regen_entry_bytes
            .len()
            .checked_add(generated_method_bytes)
            .and_then(|value| value.checked_add(behavior_bytes))
            .ok_or_else(|| "hybrid defaults rebuild capacity overflow".to_string())?;
        let mut rebuilt_entry = Vec::new();
        rebuilt_entry
            .try_reserve_exact(rebuild_capacity)
            .map_err(|error| {
                format!("reserving hybrid defaults rebuild ({rebuild_capacity} bytes): {error}")
            })?;

        let mut cursor = 0usize;
        if !self.generated_free_indices.is_empty() {
            rebuilt_entry.extend_from_slice(
                regen_entry_bytes
                    .get(..regen.functions_count_pos)
                    .ok_or_else(|| "hybrid defaults rebuild function prefix is invalid".to_string())?,
            );
            let function_count = i32::try_from(regen.functions.len())
                .map_err(|_| "hybrid defaults regenerated free-function count does not fit i32".to_string())?;
            rebuilt_entry.extend_from_slice(&function_count.to_le_bytes());
            let mut base_at_regen = vec![None; regen.functions.len()];
            for (base_index, &regen_index) in free_matches.iter().enumerate() {
                let slot = base_at_regen.get_mut(regen_index).ok_or_else(|| {
                    "hybrid defaults matched free-function index is out of bounds".to_string()
                })?;
                if slot.replace(base_index).is_some() {
                    return Err("hybrid defaults matched one regenerated free function twice".into());
                }
            }
            for (regen_index, regen_function) in regen.functions.iter().enumerate() {
                let (source, range) = match base_at_regen[regen_index] {
                    Some(base_index) if self.generated_free_indices.contains(&base_index) => {
                        (self.base_entry.as_slice(), &self.base.functions[base_index].raw)
                    }
                    _ => (regen_entry_bytes, &regen_function.raw),
                };
                rebuilt_entry.extend_from_slice(source.get(range.clone()).ok_or_else(|| {
                    "hybrid defaults rebuild free-function range is invalid".to_string()
                })?);
            }
            cursor = regen.functions_end;
        }
        for (base_class, regen_class) in self.base.classes.iter().zip(&regen.classes) {
            let carries_defaults = base_class
                .methods
                .iter()
                .any(|method| method.name.starts_with("__"));
            if carries_defaults {
                rebuilt_entry.extend_from_slice(
                    regen_entry_bytes
                        .get(cursor..regen_class.methods_count_pos)
                        .ok_or_else(|| {
                            "hybrid defaults rebuild class prefix range is invalid".to_string()
                        })?,
                );
                let method_count = i32::try_from(base_class.methods.len())
                    .map_err(|_| "hybrid defaults base method count does not fit i32".to_string())?;
                rebuilt_entry.extend_from_slice(&method_count.to_le_bytes());
                let mut regen_non_generated = regen_class.methods.iter();
                for base_method in &base_class.methods {
                    let (source, range) = if base_method.name.starts_with("__") {
                        (self.base_entry.as_slice(), &base_method.raw)
                    } else {
                        let regen_method = regen_non_generated.next().ok_or_else(|| {
                            "hybrid defaults rebuild ran out of regenerated non-generated methods"
                                .to_string()
                        })?;
                        (regen_entry_bytes, &regen_method.raw)
                    };
                    rebuilt_entry.extend_from_slice(source.get(range.clone()).ok_or_else(|| {
                        "hybrid defaults rebuild method range is invalid".to_string()
                    })?);
                }
                if regen_non_generated.next().is_some() {
                    return Err(
                        "hybrid defaults rebuild left unconsumed regenerated non-generated methods"
                            .into(),
                    );
                }
                rebuilt_entry.extend_from_slice(
                    self.base_entry
                        .get(base_class.method_table.clone())
                        .ok_or_else(|| {
                            "hybrid defaults base MethodTable range is invalid".to_string()
                        })?,
                );
                cursor = regen_class.method_table.end;
            }
            if !base_class.behaviors.is_empty() {
                rebuilt_entry.extend_from_slice(
                    regen_entry_bytes
                        .get(cursor..regen_class.behaviors_block.start)
                        .ok_or_else(|| {
                            "hybrid defaults rebuild behavior prefix is invalid".to_string()
                        })?,
                );
                rebuilt_entry.extend_from_slice(
                    self.base_entry
                        .get(base_class.behaviors_block.clone())
                        .ok_or_else(|| {
                            "hybrid defaults base behavior block range is invalid".to_string()
                        })?,
                );
                cursor = regen_class.behaviors_block.end;
            }
        }
        rebuilt_entry.extend_from_slice(
            regen_entry_bytes
                .get(cursor..)
                .ok_or_else(|| "hybrid defaults rebuild final module range is invalid".to_string())?,
        );

        let mut out = Vec::with_capacity(
            CacheHeader::SIZE + rebuilt_entry.len() + (remapped_mini.len() - module_end),
        );
        out.extend_from_slice(&remapped_mini[..CacheHeader::SIZE]);
        out.extend_from_slice(&rebuilt_entry);
        out.extend_from_slice(&remapped_mini[module_end..]);
        self.verify_hybrid_output(remapped_mini, regen_entry_bytes, &out, &free_matches)?;
        Ok(out)
    }

    fn validate_hybrid_regenerated(
        &self,
        regen_bytes: &[u8],
        regen: &ModuleEntry,
    ) -> Result<Vec<usize>, String> {
        if regen.key != self.base.key
            || regen.name != self.base.name
            || regen.file != self.base.file
        {
            return Err(format!(
                "hybrid defaults module identity drift: base key/name/file {:?}/{:?}/{:?}, \
                 regenerated {:?}/{:?}/{:?}",
                self.base.key, self.base.name, self.base.file, regen.key, regen.name, regen.file
            ));
        }
        if regen.classes.len() < self.base.classes.len() {
            return Err(format!(
                "hybrid defaults class count shrank in {:?}: base {}, regenerated {}",
                self.module_name,
                self.base.classes.len(),
                regen.classes.len()
            ));
        }
        validate_unique_function_ids(regen, "hybrid regenerated")?;
        let free_matches = self.validate_hybrid_module_functions(regen_bytes, regen)?;
        let mut class_names = HashSet::new();
        for class in &regen.classes {
            if !class_names.insert((class.namespace.as_str(), class.name.as_str())) {
                return Err(format!(
                    "hybrid regenerated module contains duplicate class {}::{}",
                    class.namespace, class.name
                ));
            }
        }
        for (base_class, regen_class) in self.base.classes.iter().zip(&regen.classes) {
            if base_class.name != regen_class.name || base_class.namespace != regen_class.namespace
            {
                return Err(format!(
                    "hybrid defaults class identity/order drift: base {}::{}, regenerated {}::{}",
                    base_class.namespace, base_class.name, regen_class.namespace, regen_class.name
                ));
            }
            if let Some(method) = regen_class
                .methods
                .iter()
                .find(|method| method.name.starts_with("__"))
            {
                return Err(format!(
                    "hybrid defaults refuses existing class {} authored/generated {}",
                    regen_class.name, method.name
                ));
            }
            let base_non_generated = base_class
                .methods
                .iter()
                .filter(|method| !method.name.starts_with("__"))
                .collect::<Vec<_>>();
            if base_non_generated.len() != regen_class.methods.len() {
                return Err(format!(
                    "hybrid defaults method count drift in {}: base has {} non-generated, regenerated has {}",
                    base_class.name,
                    base_non_generated.len(),
                    regen_class.methods.len()
                ));
            }
            for (index, (base_method, regen_method)) in base_non_generated
                .iter()
                .zip(&regen_class.methods)
                .enumerate()
            {
                compare_function(
                    &self.base_entry,
                    base_method,
                    regen_bytes,
                    regen_method,
                    &format!("hybrid {} method {index}", base_class.name),
                )?;
            }
            validate_method_table(regen_class, "hybrid regenerated")?;
        }
        Ok(free_matches)
    }

    fn verify_hybrid_output(
        &self,
        original_mini: &[u8],
        regen_entry_bytes: &[u8],
        output: &[u8],
        free_matches: &[usize],
    ) -> Result<(), String> {
        if module_count(output) != 1 {
            return Err("hybrid defaults postcondition changed module count".into());
        }
        let output_end = module_region_end(output)
            .map_err(|error| format!("walking hybrid defaults output: {error}"))?;
        let output_tables = parse_tail_tables(output, output_end)
            .map_err(|error| format!("parsing hybrid defaults output tail: {error}"))?;
        if output_tables.end != output.len() {
            return Err("hybrid defaults output tail does not end at EOF".into());
        }
        let original_end = module_region_end(original_mini)
            .map_err(|error| format!("re-walking original hybrid mini: {error}"))?;
        if output[output_end..] != original_mini[original_end..] {
            return Err("hybrid defaults carry changed new-symbol tail bytes".into());
        }
        let output_entry_bytes = output
            .get(CacheHeader::SIZE..output_end)
            .ok_or_else(|| "hybrid defaults output module range is invalid".to_string())?;
        let carried = parse_entry(output_entry_bytes, "hybrid carried output")?;
        let regen = parse_entry(regen_entry_bytes, "hybrid postcondition regen")?;
        validate_unique_function_ids(&carried, "hybrid carried output")?;
        validate_function_ids_against_outside(
            &carried,
            &self.outside_function_ids,
            "hybrid carried output",
        )?;
        if carried.functions.len() != regen.functions.len()
            || carried.classes.len() != regen.classes.len()
            || carried.classes.len() < self.base.classes.len()
        {
            return Err("hybrid defaults carried record counts are inconsistent".into());
        }
        if free_matches.len() != self.base.functions.len() {
            return Err("hybrid defaults postcondition has incomplete base free-function mapping".into());
        }
        let mut base_at_regen = vec![None; regen.functions.len()];
        for (base_index, &regen_index) in free_matches.iter().enumerate() {
            let slot = base_at_regen.get_mut(regen_index).ok_or_else(|| {
                "hybrid defaults postcondition free-function mapping is out of bounds".to_string()
            })?;
            if slot.replace(base_index).is_some() {
                return Err("hybrid defaults postcondition mapped one regenerated free function twice".into());
            }
        }
        for (regen_index, (regen_function, out_function)) in regen
            .functions
            .iter()
            .zip(&carried.functions)
            .enumerate()
        {
            let (expected_bytes, expected_range) = match base_at_regen[regen_index] {
                Some(base_index) if self.generated_free_indices.contains(&base_index) => {
                    (self.base_entry.as_slice(), &self.base.functions[base_index].raw)
                }
                _ => (regen_entry_bytes, &regen_function.raw),
            };
            compare_range(
                expected_bytes,
                expected_range,
                output_entry_bytes,
                &out_function.raw,
                "hybrid free function",
            )?;
        }
        for ((base_class, regen_class), out_class) in self
            .base
            .classes
            .iter()
            .zip(&regen.classes)
            .zip(&carried.classes)
        {
            if base_class.name != out_class.name || base_class.namespace != out_class.namespace {
                return Err("hybrid defaults carried existing class identity changed".into());
            }
            if base_class.methods.len() != out_class.methods.len() {
                return Err(format!(
                    "hybrid defaults carried method count mismatch in {}",
                    base_class.name
                ));
            }
            let mut regen_methods = regen_class.methods.iter();
            for (base_method, out_method) in base_class.methods.iter().zip(&out_class.methods) {
                let (expected_bytes, expected_range) = if base_method.name.starts_with("__") {
                    (self.base_entry.as_slice(), &base_method.raw)
                } else {
                    let regen_method = regen_methods.next().ok_or_else(|| {
                        "hybrid defaults postcondition exhausted regenerated methods".to_string()
                    })?;
                    (regen_entry_bytes, &regen_method.raw)
                };
                compare_range(
                    expected_bytes,
                    expected_range,
                    output_entry_bytes,
                    &out_method.raw,
                    &format!("hybrid {}::{}", base_class.name, base_method.name),
                )?;
            }
            if regen_methods.next().is_some() {
                return Err("hybrid defaults postcondition left regenerated methods".into());
            }
            let expected_table = self
                .base_entry
                .get(base_class.method_table.clone())
                .ok_or_else(|| "hybrid defaults expected MethodTable range is invalid".to_string())?;
            let actual_table = output_entry_bytes
                .get(out_class.method_table.clone())
                .ok_or_else(|| "hybrid defaults output MethodTable range is invalid".to_string())?;
            if expected_table != actual_table {
                return Err(format!(
                    "hybrid defaults postcondition changed {} MethodTable",
                    base_class.name
                ));
            }
        }
        for (regen_class, out_class) in regen
            .classes
            .iter()
            .skip(self.base.classes.len())
            .zip(carried.classes.iter().skip(self.base.classes.len()))
        {
            compare_range(
                regen_entry_bytes,
                &(regen_class.prefix.start..regen_class.preprocessor_tail.end),
                output_entry_bytes,
                &(out_class.prefix.start..out_class.preprocessor_tail.end),
                "hybrid appended class",
            )?;
        }
        Ok(())
    }

    fn compare_module_functions(
        &self,
        regen_bytes: &[u8],
        regen: &ModuleEntry,
    ) -> Result<(), String> {
        if self.base.functions.len() != regen.functions.len() {
            return Err(format!(
                "generated-default module free functions count drift: base {}, regenerated {}",
                self.base.functions.len(),
                regen.functions.len()
            ));
        }
        for (index, (base, regenerated)) in
            self.base.functions.iter().zip(&regen.functions).enumerate()
        {
            if !self.generated_free_indices.contains(&index) {
                compare_function(
                    &self.base_entry,
                    base,
                    regen_bytes,
                    regenerated,
                    &format!("module free functions[{index}]"),
                )?;
                continue;
            }
            self.compare_generated_free_function(regen_bytes, base, regenerated, index)?;
        }
        Ok(())
    }

    /// The source emitter omits a handful of compiler-generated free wrappers.  Their bodies are
    /// carried byte-exact, but only after their declaration and shipping metadata prove that this
    /// is the same wrapper.  The narrow 32 -> 33 factory trait transition is observed compiler
    /// behavior; every other drift is rejected before the carry.
    fn compare_generated_free_function(
        &self,
        regen_bytes: &[u8],
        base: &FunctionRecord,
        regenerated: &FunctionRecord,
        base_index: usize,
    ) -> Result<(), String> {
        if base.name != regenerated.name || base.namespace != regenerated.namespace {
            return Err(format!(
                "generated-default compiler wrapper identity drift at module function \
                 {base_index}: {}::{}/{}::{}",
                base.namespace, base.name, regenerated.namespace, regenerated.name
            ));
        }
        compare_range(
            &self.base_entry,
            &base.declaration,
            regen_bytes,
            &regenerated.declaration,
            &format!("module compiler wrapper {base_index} declaration"),
        )?;
        compare_range(
            &self.base_entry,
            &base.ufunction_tail,
            regen_bytes,
            &regenerated.ufunction_tail,
            &format!("module compiler wrapper {base_index} UFUNCTION metadata"),
        )?;
        let class_factory = self
            .base
            .classes
            .iter()
            .any(|class| class.name == base.name);
        if base.traits != regenerated.traits
            && !(class_factory && base.traits == 32 && regenerated.traits == 33)
        {
            return Err(format!(
                "generated-default compiler wrapper {} traits drift: base {}, regenerated {}",
                base.name, base.traits, regenerated.traits
            ));
        }
        Ok(())
    }

    fn validate_hybrid_module_functions(
        &self,
        regen_bytes: &[u8],
        regen: &ModuleEntry,
    ) -> Result<Vec<usize>, String> {
        let matches = existing_free_function_subsequence_indices(
            &self.base_entry,
            &self.base.functions,
            regen_bytes,
            regen,
            self.base.classes.len(),
            "hybrid module free functions",
        )?;
        for (base_index, (base, &regen_index)) in self
            .base
            .functions
            .iter()
            .zip(&matches)
            .enumerate()
        {
            let regenerated = regen.functions.get(regen_index).ok_or_else(|| {
                "hybrid defaults matched free-function index is out of bounds".to_string()
            })?;
            if self.generated_free_indices.contains(&base_index) {
                self.compare_generated_free_function(regen_bytes, base, regenerated, base_index)?;
            } else {
                compare_function(
                    &self.base_entry,
                    base,
                    regen_bytes,
                    regenerated,
                    &format!("hybrid module free functions[{base_index}]"),
                )?;
            }
        }
        Ok(matches)
    }

    fn validate_regenerated(&self, regen_bytes: &[u8], regen: &ModuleEntry) -> Result<(), String> {
        if regen.key != self.base.key
            || regen.name != self.base.name
            || regen.file != self.base.file
        {
            return Err(format!(
                "generated-default module identity drift: base key/name/file \
                 {:?}/{:?}/{:?}, regenerated {:?}/{:?}/{:?}",
                self.base.key, self.base.name, self.base.file, regen.key, regen.name, regen.file
            ));
        }
        validate_unique_function_ids(regen, "regenerated")?;
        self.compare_module_functions(regen_bytes, regen)?;
        compare_range(
            &self.base_entry,
            &self.base.enums,
            regen_bytes,
            &regen.enums,
            "enum layout",
        )?;
        compare_range(
            &self.base_entry,
            &self.base.globals,
            regen_bytes,
            &regen.globals,
            "global layout/init records",
        )?;
        compare_range(
            &self.base_entry,
            &self.base.imports,
            regen_bytes,
            &regen.imports,
            "function imports",
        )?;
        compare_range(
            &self.base_entry,
            &self.base.post_code_hash,
            regen_bytes,
            &regen.post_code_hash,
            "module metadata after CodeHash",
        )?;

        if self.base.classes.len() != regen.classes.len() {
            return Err(format!(
                "generated-default class count drift in {:?}: base {}, regenerated {}",
                self.module_name,
                self.base.classes.len(),
                regen.classes.len()
            ));
        }
        let mut regen_names = HashSet::new();
        for (base_class, regen_class) in self.base.classes.iter().zip(&regen.classes) {
            if !regen_names.insert(regen_class.name.as_str()) {
                return Err(format!(
                    "generated-default regenerated module contains duplicate class {:?}",
                    regen_class.name
                ));
            }
            if base_class.name != regen_class.name || base_class.namespace != regen_class.namespace
            {
                return Err(format!(
                    "generated-default class identity/order drift: base {}::{}, regenerated \
                     {}::{}",
                    base_class.namespace, base_class.name, regen_class.namespace, regen_class.name
                ));
            }
            compare_range(
                &self.base_entry,
                &base_class.prefix,
                regen_bytes,
                &regen_class.prefix,
                &format!("{} class flags/properties", base_class.name),
            )?;
            compare_range(
                &self.base_entry,
                &base_class.derived_and_shadow,
                regen_bytes,
                &regen_class.derived_and_shadow,
                &format!("{} DerivedFrom/ShadowType", base_class.name),
            )?;
            compare_functions(
                &self.base_entry,
                &base_class.constructors,
                regen_bytes,
                &regen_class.constructors,
                &format!("{} constructors", base_class.name),
            )?;
            compare_range(
                &self.base_entry,
                &base_class.factory_and_behavior_refs,
                regen_bytes,
                &regen_class.factory_and_behavior_refs,
                &format!("{} factory/behavior refs", base_class.name),
            )?;
            compare_functions(
                &self.base_entry,
                &base_class.behaviors,
                regen_bytes,
                &regen_class.behaviors,
                &format!("{} behavior functions", base_class.name),
            )?;
            compare_range(
                &self.base_entry,
                &base_class.behavior_types,
                regen_bytes,
                &regen_class.behavior_types,
                &format!("{} behavior types", base_class.name),
            )?;
            compare_range(
                &self.base_entry,
                &base_class.preprocessor_tail,
                regen_bytes,
                &regen_class.preprocessor_tail,
                &format!("{} class metadata", base_class.name),
            )?;
            validate_method_table(regen_class, "regenerated")?;

            if let Some(method) = regen_class
                .methods
                .iter()
                .find(|method| method.name.starts_with("__"))
            {
                return Err(format!(
                    "regenerated class {} unexpectedly authored/generated {}; refusing to \
                     overwrite it with stale defaults",
                    regen_class.name, method.name
                ));
            }
            let base_non_generated = base_class
                .methods
                .iter()
                .filter(|method| !method.name.starts_with("__"))
                .collect::<Vec<_>>();
            if base_non_generated.len() != regen_class.methods.len() {
                return Err(format!(
                    "generated-default method count drift in {}: base has {} non-generated, \
                     regenerated has {}",
                    base_class.name,
                    base_non_generated.len(),
                    regen_class.methods.len()
                ));
            }
            for (index, (base_method, regen_method)) in base_non_generated
                .iter()
                .zip(&regen_class.methods)
                .enumerate()
            {
                compare_function(
                    &self.base_entry,
                    base_method,
                    regen_bytes,
                    regen_method,
                    &format!("{} method {index}", base_class.name),
                )?;
            }
            if !base_class
                .methods
                .iter()
                .any(|method| method.name.starts_with("__"))
            {
                compare_range(
                    &self.base_entry,
                    &base_class.method_table,
                    regen_bytes,
                    &regen_class.method_table,
                    &format!("{} MethodTable", base_class.name),
                )?;
            }
        }
        Ok(())
    }

    fn verify_output(
        &self,
        original_mini: &[u8],
        regen_entry_bytes: &[u8],
        output: &[u8],
    ) -> Result<(), String> {
        if module_count(output) != 1 {
            return Err("generated-default postcondition changed module count".into());
        }
        let output_end = module_region_end(output)
            .map_err(|error| format!("walking carried defaults output: {error}"))?;
        let output_tables = parse_tail_tables(output, output_end)
            .map_err(|error| format!("parsing carried defaults output tail: {error}"))?;
        if output_tables.end != output.len() {
            return Err("generated-default carried output tail does not end at EOF".into());
        }
        let original_end = module_region_end(original_mini)
            .map_err(|error| format!("re-walking original defaults mini: {error}"))?;
        if output[output_end..] != original_mini[original_end..] {
            return Err("generated-default carry changed strict-remap tail bytes".into());
        }
        let output_entry_bytes = &output[CacheHeader::SIZE..output_end];
        let carried = parse_entry(output_entry_bytes, "carried output")?;
        let post_regen = parse_entry(regen_entry_bytes, "postcondition regen")?;
        validate_unique_function_ids(&carried, "carried output")?;
        validate_function_ids_against_outside(
            &carried,
            &self.outside_function_ids,
            "carried output",
        )?;
        if carried.classes.len() != self.base.classes.len()
            || carried.classes.len() != post_regen.classes.len()
        {
            return Err("generated-default carried class count is inconsistent".into());
        }
        if carried.functions.len() != self.base.functions.len()
            || carried.functions.len() != post_regen.functions.len()
        {
            return Err("generated-default carried free-function count is inconsistent".into());
        }
        for (index, ((base, regen), output_function)) in self
            .base
            .functions
            .iter()
            .zip(&post_regen.functions)
            .zip(&carried.functions)
            .enumerate()
        {
            let (expected_bytes, expected_range) = if self.generated_free_indices.contains(&index) {
                (self.base_entry.as_slice(), &base.raw)
            } else {
                (regen_entry_bytes, &regen.raw)
            };
            let expected = expected_bytes.get(expected_range.clone()).ok_or_else(|| {
                "generated-default expected free-function range is invalid".to_string()
            })?;
            let actual = output_entry_bytes
                .get(output_function.raw.clone())
                .ok_or_else(|| {
                    "generated-default output free-function range is invalid".to_string()
                })?;
            if expected != actual {
                return Err(format!(
                    "generated-default postcondition failed for module function {}",
                    base.name
                ));
            }
        }

        for ((base_class, regen_class), out_class) in self
            .base
            .classes
            .iter()
            .zip(&post_regen.classes)
            .zip(&carried.classes)
        {
            if base_class.name != out_class.name || base_class.namespace != out_class.namespace {
                return Err("generated-default carried class identity changed".into());
            }
            if base_class.methods.len() != out_class.methods.len() {
                return Err(format!(
                    "generated-default carried method count mismatch in {}",
                    base_class.name
                ));
            }
            let mut regen_methods = regen_class.methods.iter();
            for (base_method, out_method) in base_class.methods.iter().zip(&out_class.methods) {
                let (expected_bytes, expected_range) = if base_method.name.starts_with("__") {
                    (self.base_entry.as_slice(), &base_method.raw)
                } else {
                    let regen_method = regen_methods.next().ok_or_else(|| {
                        "generated-default postcondition exhausted regenerated methods".to_string()
                    })?;
                    (regen_entry_bytes, &regen_method.raw)
                };
                let expected = expected_bytes.get(expected_range.clone()).ok_or_else(|| {
                    "generated-default expected method range is invalid".to_string()
                })?;
                let actual = output_entry_bytes
                    .get(out_method.raw.clone())
                    .ok_or_else(|| {
                        "generated-default output method range is invalid".to_string()
                    })?;
                if expected != actual {
                    return Err(format!(
                        "generated-default postcondition failed for {}::{}",
                        base_class.name, base_method.name
                    ));
                }
            }
            if regen_methods.next().is_some() {
                return Err("generated-default postcondition left regenerated methods".into());
            }
            let expected_table = self
                .base_entry
                .get(base_class.method_table.clone())
                .ok_or_else(|| {
                    "generated-default expected MethodTable range is invalid".to_string()
                })?;
            let actual_table = output_entry_bytes
                .get(out_class.method_table.clone())
                .ok_or_else(|| {
                    "generated-default output MethodTable range is invalid".to_string()
                })?;
            if expected_table != actual_table {
                return Err(format!(
                    "generated-default postcondition changed {} MethodTable",
                    base_class.name
                ));
            }
            validate_method_table(out_class, "carried output")?;
            if base_class.behaviors.len() != out_class.behaviors.len() {
                return Err(format!(
                    "generated-default carried behavior count mismatch in {}",
                    base_class.name
                ));
            }
            for (base_behavior, out_behavior) in
                base_class.behaviors.iter().zip(&out_class.behaviors)
            {
                let expected = self
                    .base_entry
                    .get(base_behavior.raw.clone())
                    .ok_or_else(|| {
                        "generated-default expected behavior range is invalid".to_string()
                    })?;
                let actual = output_entry_bytes
                    .get(out_behavior.raw.clone())
                    .ok_or_else(|| {
                        "generated-default output behavior range is invalid".to_string()
                    })?;
                if expected != actual {
                    return Err(format!(
                        "generated-default postcondition failed for {} behavior {}",
                        base_class.name, base_behavior.name
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_model_identity(
    entry: &ModuleEntry,
    module: &model::Module,
    module_name: &str,
) -> Result<(), String> {
    if entry.functions.len() != module.functions.len()
        || entry
            .functions
            .iter()
            .zip(&module.functions)
            .any(|(raw, parsed)| {
                raw.name != parsed.name
                    || raw.namespace != parsed.namespace
                    || raw.traits != parsed.traits
            })
    {
        return Err(format!(
            "generated-default raw/model free-function identity mismatch in {module_name:?}"
        ));
    }
    if entry.classes.len() != module.classes.len() {
        return Err(format!(
            "generated-default raw/model class count mismatch in {module_name:?}: {}/{}",
            entry.classes.len(),
            module.classes.len()
        ));
    }
    for (raw, parsed) in entry.classes.iter().zip(&module.classes) {
        if raw.name != parsed.name {
            return Err(format!(
                "generated-default raw/model class order mismatch in {module_name:?}: \
                 {:?}/{:?}",
                raw.name, parsed.name
            ));
        }
        if raw.methods.len() != parsed.methods.len()
            || raw
                .methods
                .iter()
                .zip(&parsed.methods)
                .any(|(left, right)| {
                    left.name != right.name
                        || left.namespace != right.namespace
                        || left.traits != right.traits
                })
        {
            return Err(format!(
                "generated-default raw/model method identity mismatch in {}::{}",
                module_name, raw.name
            ));
        }
        if raw.constructors.len() != parsed.ctors.len()
            || raw
                .constructors
                .iter()
                .zip(&parsed.ctors)
                .any(|(left, right)| {
                    left.name != right.name
                        || left.namespace != right.namespace
                        || left.traits != right.traits
                })
        {
            return Err(format!(
                "generated-default raw/model constructor identity mismatch in {}::{}",
                module_name, raw.name
            ));
        }
    }
    Ok(())
}

fn validate_unique_function_ids(entry: &ModuleEntry, source: &str) -> Result<(), String> {
    let mut ids = HashMap::<i32, String>::new();
    record_function_ids(entry, source, &mut ids)
}

fn validate_cache_wide_function_ids(
    bytes: &[u8],
    ranges: &[(String, usize, usize)],
    target_index: usize,
) -> Result<HashMap<i32, String>, String> {
    let mut all_ids = HashMap::new();
    let mut outside_ids = HashMap::new();
    for (index, (key, start, end)) in ranges.iter().enumerate() {
        let entry_bytes = bytes.get(*start..*end).ok_or_else(|| {
            format!("generated-default cache-wide module range {key:?} is invalid")
        })?;
        let entry = parse_entry(entry_bytes, &format!("base module {key:?}"))?;
        record_function_ids(&entry, "cache-wide base", &mut all_ids)?;
        if index != target_index {
            record_function_ids(&entry, "untouched base modules", &mut outside_ids)?;
        }
    }
    Ok(outside_ids)
}

fn validate_function_ids_against_outside(
    entry: &ModuleEntry,
    outside_ids: &HashMap<i32, String>,
    source: &str,
) -> Result<(), String> {
    let mut target_ids = HashMap::new();
    record_function_ids(entry, source, &mut target_ids)?;
    for (id, identity) in target_ids {
        if let Some(outside) = outside_ids.get(&id) {
            return Err(format!(
                "generated-default {source} cache-wide function Id collision {id:#x}: \
                 {identity} and untouched {outside}"
            ));
        }
    }
    Ok(())
}

fn record_function_ids(
    entry: &ModuleEntry,
    source: &str,
    ids: &mut HashMap<i32, String>,
) -> Result<(), String> {
    let mut record = |function: &FunctionRecord, scope: &str| -> Result<(), String> {
        let identity = format!("{scope}::{}", function.name);
        if let Some(prior) = ids.insert(function.id, identity.clone()) {
            return Err(format!(
                "generated-default {source} function Id collision {:#x}: {prior} and {identity}",
                function.id
            ));
        }
        Ok(())
    };
    for function in &entry.functions {
        record(function, &entry.name)?;
    }
    for class in &entry.classes {
        let scope = format!("{}.{}", entry.name, class.name);
        for function in class
            .methods
            .iter()
            .chain(&class.constructors)
            .chain(&class.behaviors)
        {
            record(function, &scope)?;
        }
    }
    for function in &entry.global_init_functions {
        record(function, &format!("{}.<global-init>", entry.name))?;
    }
    Ok(())
}

fn is_generated_free_shape(entry: &ModuleEntry, function: &FunctionRecord) -> bool {
    function.name == "StaticClass"
        || entry
            .classes
            .iter()
            .any(|class| class.name == function.name)
        || entry.classes.iter().any(|class| {
            class.name == function.namespace
                && class
                    .methods
                    .iter()
                    .chain(&class.constructors)
                    .any(|method| method.name == function.name)
        })
}

fn validate_method_table(class: &ClassRecord, source: &str) -> Result<(), String> {
    for (slot_index, &method_index) in class.method_table_values.iter().enumerate() {
        if method_index == -1 {
            continue;
        }
        if method_index < 0 || method_index as usize >= class.methods.len() {
            return Err(format!(
                "{source} class {} MethodTable slot {} references invalid local method {} \
                 (Methods.Num={})",
                class.name,
                slot_index,
                method_index,
                class.methods.len()
            ));
        }
    }
    Ok(())
}

fn compare_functions(
    left_bytes: &[u8],
    left: &[FunctionRecord],
    right_bytes: &[u8],
    right: &[FunctionRecord],
    what: &str,
) -> Result<(), String> {
    if left.len() != right.len() {
        return Err(format!(
            "generated-default {what} count drift: base {}, regenerated {}",
            left.len(),
            right.len()
        ));
    }
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        compare_function(
            left_bytes,
            left,
            right_bytes,
            right,
            &format!("{what}[{index}]"),
        )?;
    }
    Ok(())
}

fn compare_function_declarations(
    left_bytes: &[u8],
    left: &[FunctionRecord],
    right_bytes: &[u8],
    right: &[FunctionRecord],
    what: &str,
) -> Result<(), String> {
    if left.len() != right.len() {
        return Err(format!(
            "module-structure {what} count drift: base {}, regenerated {}",
            left.len(),
            right.len()
        ));
    }
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        compare_function_declaration(
            left_bytes,
            left,
            right_bytes,
            right,
            &format!("{what}[{index}]"),
        )?;
    }
    Ok(())
}

fn compare_function_declaration(
    left_bytes: &[u8],
    left: &FunctionRecord,
    right_bytes: &[u8],
    right: &FunctionRecord,
    what: &str,
) -> Result<(), String> {
    if left.name != right.name || left.namespace != right.namespace {
        return Err(format!(
            "module-structure {what} identity drift: {:?}::{:?}/{}::{:?}",
            left.namespace, left.name, right.namespace, right.name
        ));
    }
    compare_structure_range(
        left_bytes,
        &left.declaration,
        right_bytes,
        &right.declaration,
        &format!("{what} declaration"),
    )
}

fn compare_existing_free_function_subsequence(
    left_bytes: &[u8],
    left: &[FunctionRecord],
    right_bytes: &[u8],
    regenerated_module: &ModuleEntry,
    base_class_count: usize,
    what: &str,
) -> Result<(), String> {
    existing_free_function_subsequence_indices(
        left_bytes,
        left,
        right_bytes,
        regenerated_module,
        base_class_count,
        what,
    )
    .map(|_| ())
}

/// Return the regenerated position of every existing free function, preserving declaration order.
/// Entries skipped between two existing declarations must be the one empirically established
/// compiler helper for an appended class; arbitrary authored functions never qualify.
fn existing_free_function_subsequence_indices(
    left_bytes: &[u8],
    left: &[FunctionRecord],
    right_bytes: &[u8],
    regenerated_module: &ModuleEntry,
    base_class_count: usize,
    what: &str,
) -> Result<Vec<usize>, String> {
    let right = &regenerated_module.functions;
    let new_classes =
        &regenerated_module.classes[base_class_count.min(regenerated_module.classes.len())..];
    let mut next = 0usize;
    let mut matches = Vec::with_capacity(left.len());
    for (index, base) in left.iter().enumerate() {
        loop {
            let Some(regenerated) = right.get(next) else {
                return Err(format!(
                    "module-structure {what} lost or reordered existing declaration at base \
                     index {index}: {}::{}",
                    base.namespace, base.name
                ));
            };
            if function_declaration_matches(left_bytes, base, right_bytes, regenerated)? {
                matches.push(next);
                next += 1;
                break;
            }
            if !is_new_class_compiler_helper(regenerated, new_classes) {
                return Err(format!(
                    "module-structure {what} inserted unsupported declaration before existing \
                     base index {index}: {}::{}",
                    regenerated.namespace, regenerated.name
                ));
            }
            next += 1;
        }
    }
    Ok(matches)
}

fn is_new_class_compiler_helper(function: &FunctionRecord, new_classes: &[ClassRecord]) -> bool {
    // FullGraph evidence for an appended dialog topic shows exactly this generated free helper:
    // `NewClass::StaticClass`.  Do not turn a merely similarly named authored free function into
    // an exception to the existing-function ordering proof.
    new_classes.iter().any(|class| {
        function.name == "StaticClass" && function.namespace == class.name
    })
}

fn function_declaration_matches(
    left_bytes: &[u8],
    left: &FunctionRecord,
    right_bytes: &[u8],
    right: &FunctionRecord,
) -> Result<bool, String> {
    if left.name != right.name || left.namespace != right.namespace {
        return Ok(false);
    }
    let left = left_bytes
        .get(left.declaration.clone())
        .ok_or_else(|| "module-structure base function declaration range is invalid".to_string())?;
    let right = right_bytes.get(right.declaration.clone()).ok_or_else(|| {
        "module-structure regenerated function declaration range is invalid".to_string()
    })?;
    Ok(left == right)
}

fn compare_structure_range(
    left_bytes: &[u8],
    left: &Range<usize>,
    right_bytes: &[u8],
    right: &Range<usize>,
    what: &str,
) -> Result<(), String> {
    let left = left_bytes
        .get(left.clone())
        .ok_or_else(|| format!("module-structure {what} base range is invalid"))?;
    let right = right_bytes
        .get(right.clone())
        .ok_or_else(|| format!("module-structure {what} regenerated range is invalid"))?;
    if left != right {
        return Err(format!("module-structure {what} drift"));
    }
    Ok(())
}

fn compare_global_record_prefix(
    left_bytes: &[u8],
    left: &[GlobalRecord],
    right_bytes: &[u8],
    right: &[GlobalRecord],
    what: &str,
) -> Result<(), String> {
    if right.len() < left.len() {
        return Err(format!(
            "module-structure {what} count shrank: base {}, regenerated {}",
            left.len(),
            right.len()
        ));
    }
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        compare_structure_range(
            left_bytes,
            &left.declaration,
            right_bytes,
            &right.declaration,
            &format!("{what}[{index}]"),
        )?;
    }
    Ok(())
}

fn compare_structure_record_prefix(
    left_bytes: &[u8],
    left: &[Range<usize>],
    right_bytes: &[u8],
    right: &[Range<usize>],
    what: &str,
) -> Result<(), String> {
    if right.len() < left.len() {
        return Err(format!(
            "module-structure {what} count shrank: base {}, regenerated {}",
            left.len(),
            right.len()
        ));
    }
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        compare_structure_range(
            left_bytes,
            left,
            right_bytes,
            right,
            &format!("{what}[{index}]"),
        )?;
    }
    Ok(())
}

fn compare_function(
    left_bytes: &[u8],
    left: &FunctionRecord,
    right_bytes: &[u8],
    right: &FunctionRecord,
    what: &str,
) -> Result<(), String> {
    if left.name != right.name {
        return Err(format!(
            "generated-default {what} identity drift: {:?}/{:?}",
            left.name, right.name
        ));
    }
    compare_range(
        left_bytes,
        &left.signature,
        right_bytes,
        &right.signature,
        &format!("{what} declaration/signature"),
    )?;
    compare_range(
        left_bytes,
        &left.ufunction_tail,
        right_bytes,
        &right.ufunction_tail,
        &format!("{what} UFUNCTION metadata"),
    )
}

fn compare_range(
    left_bytes: &[u8],
    left: &Range<usize>,
    right_bytes: &[u8],
    right: &Range<usize>,
    what: &str,
) -> Result<(), String> {
    let left = left_bytes
        .get(left.clone())
        .ok_or_else(|| format!("generated-default {what} base range is invalid"))?;
    let right = right_bytes
        .get(right.clone())
        .ok_or_else(|| format!("generated-default {what} regenerated range is invalid"))?;
    if left != right {
        return Err(format!("generated-default {what} drift"));
    }
    Ok(())
}

fn parse_entry(bytes: &[u8], context: &str) -> Result<ModuleEntry, String> {
    let mut cursor = Cursor::new(bytes);
    let key = cursor
        .read_fstring()
        .map_err(|error| format!("parsing {context} map key: {error}"))?;
    let name = cursor
        .read_sia()
        .map_err(|error| format!("parsing {context} module name: {error}"))?;

    let functions_count_pos = cursor.pos();
    let function_count =
        bounded_count_with_minimum(&mut cursor, "Module.Functions", context, MIN_FUNCTION_BYTES)?;
    let mut functions = reserved_vec(function_count, "Module.Functions", context)?;
    for _ in 0..function_count {
        functions.push(parse_function(&mut cursor, context)?);
    }
    let functions_end = cursor.pos();

    let class_count =
        bounded_count_with_minimum(&mut cursor, "Module.Classes", context, MIN_CLASS_BYTES)?;
    let mut classes = reserved_vec(class_count, "Module.Classes", context)?;
    for _ in 0..class_count {
        classes.push(parse_class(&mut cursor, context)?);
    }

    let enums_start = cursor.pos();
    let enum_count =
        bounded_count_with_minimum(&mut cursor, "Module.Enums", context, MIN_ENUM_BYTES)?;
    let mut enum_entries = reserved_vec(enum_count, "Module.Enums", context)?;
    for _ in 0..enum_count {
        let start = cursor.pos();
        read_sia(&mut cursor, context, "Enum.Name")?;
        read_sia(&mut cursor, context, "Enum.Namespace")?;
        skip_sia_array(&mut cursor, "Enum.Names", context)?;
        skip_fixed_array(&mut cursor, 4, "Enum.Values", context)?;
        enum_entries.push(start..cursor.pos());
    }
    let enums = enums_start..cursor.pos();

    let globals_start = cursor.pos();
    let global_count = bounded_count_with_minimum(
        &mut cursor,
        "Module.GlobalVariables",
        context,
        MIN_GLOBAL_BYTES,
    )?;
    let mut global_entries = reserved_vec(global_count, "Module.GlobalVariables", context)?;
    let mut global_init_functions = Vec::new();
    for _ in 0..global_count {
        let global = parse_global(&mut cursor, context)?;
        if let Some(function) = global.initializer.clone() {
            global_init_functions.push(function);
        }
        global_entries.push(global);
    }
    let globals = globals_start..cursor.pos();

    let imports_start = cursor.pos();
    let import_count = bounded_count_with_minimum(
        &mut cursor,
        "Module.FunctionImports",
        context,
        MIN_IMPORT_BYTES,
    )?;
    let mut import_entries = reserved_vec(import_count, "Module.FunctionImports", context)?;
    for _ in 0..import_count {
        let start = cursor.pos();
        read_sia(&mut cursor, context, "Import.Module")?;
        read_sia(&mut cursor, context, "Import.Name")?;
        read_sia(&mut cursor, context, "Import.Namespace")?;
        skip_fixed_array(
            &mut cursor,
            DATA_TYPE_SIZE,
            "Import.ParameterTypes",
            context,
        )?;
        skip_fixed_array(&mut cursor, 4, "Import.ParameterFlags", context)?;
        skip_sia_array(&mut cursor, "Import.ParameterDefaults", context)?;
        skip(&mut cursor, DATA_TYPE_SIZE, context, "Import.ReturnType")?;
        import_entries.push(start..cursor.pos());
    }
    let imports = imports_start..cursor.pos();

    skip(&mut cursor, 8, context, "Module.CodeHash")?;
    let post_start = cursor.pos();
    let imported_modules = read_sia_array_ranges(&mut cursor, "Module.ImportedModules", context)?;
    let statics_start = cursor.pos();
    read_sia(&mut cursor, context, "Module.StaticsClassName")?;
    let statics_class = statics_start..cursor.pos();
    let declared_events = read_sia_array_ranges(&mut cursor, "Module.DeclaredEvents", context)?;
    let declared_delegates =
        read_sia_array_ranges(&mut cursor, "Module.DeclaredDelegates", context)?;
    let file = read_sia(&mut cursor, context, "Module.ScriptRelativeFilename")?;
    let post_init_functions =
        read_sia_array_ranges(&mut cursor, "Module.PostInitFunctions", context)?;
    let post_code_hash = post_start..cursor.pos();
    if cursor.pos() != bytes.len() {
        return Err(format!(
            "parsing {context} stopped at {:#x}, entry ends at {:#x}",
            cursor.pos(),
            bytes.len()
        ));
    }
    Ok(ModuleEntry {
        key,
        name,
        file,
        functions_count_pos,
        functions,
        functions_end,
        classes,
        enums,
        enum_entries,
        globals,
        global_entries,
        global_init_functions,
        imports,
        import_entries,
        imported_modules,
        statics_class,
        declared_events,
        declared_delegates,
        post_init_functions,
        post_code_hash,
    })
}

fn parse_class(cursor: &mut Cursor<'_>, context: &str) -> Result<ClassRecord, String> {
    let start = cursor.pos();
    let name = read_sia(cursor, context, "Class.Name")?;
    let namespace = read_sia(cursor, context, "Class.Namespace")?;
    skip(cursor, 4, context, "Class.Flags")?;
    let property_count =
        bounded_count_with_minimum(cursor, "Class.Properties", context, MIN_PROPERTY_BYTES)?;
    for _ in 0..property_count {
        parse_property(cursor, context)?;
    }
    let prefix = start..cursor.pos();

    let methods_count_pos = cursor.pos();
    let method_count =
        bounded_count_with_minimum(cursor, "Class.Methods", context, MIN_FUNCTION_BYTES)?;
    let mut methods = reserved_vec(method_count, "Class.Methods", context)?;
    for _ in 0..method_count {
        methods.push(parse_function(cursor, context)?);
    }
    let method_table_start = cursor.pos();
    let method_table_count = bounded_count_with_minimum(cursor, "Class.MethodTable", context, 4)?;
    let mut method_table_values = reserved_vec(method_table_count, "Class.MethodTable", context)?;
    for _ in 0..method_table_count {
        method_table_values.push(
            cursor
                .read_i32()
                .map_err(|error| format!("parsing {context} Class.MethodTable: {error}"))?,
        );
    }
    let method_table = method_table_start..cursor.pos();

    let derived_start = cursor.pos();
    skip(cursor, 16, context, "Class.DerivedFrom+ShadowType")?;
    let derived_and_shadow = derived_start..cursor.pos();

    let constructor_count =
        bounded_count_with_minimum(cursor, "Class.Constructors", context, MIN_FUNCTION_BYTES)?;
    let mut constructors = reserved_vec(constructor_count, "Class.Constructors", context)?;
    for _ in 0..constructor_count {
        constructors.push(parse_function(cursor, context)?);
    }

    let refs_start = cursor.pos();
    skip_fixed_array(cursor, 8, "Class.FactoryRefs", context)?;
    skip_fixed_array(cursor, 8, "Class.BehaviorRefs", context)?;
    let factory_and_behavior_refs = refs_start..cursor.pos();

    let behaviors_start = cursor.pos();
    let behavior_count = bounded_count_with_minimum(
        cursor,
        "Class.BehaviorFunctions",
        context,
        MIN_FUNCTION_BYTES,
    )?;
    let mut behaviors = reserved_vec(behavior_count, "Class.BehaviorFunctions", context)?;
    for _ in 0..behavior_count {
        behaviors.push(parse_function(cursor, context)?);
    }
    let behaviors_block = behaviors_start..cursor.pos();
    let behavior_types_start = cursor.pos();
    skip_fixed_array(cursor, 4, "Class.BehaviorFunctionTypes", context)?;
    let behavior_types = behavior_types_start..cursor.pos();

    let preprocessor_start = cursor.pos();
    let has_preprocessor = cursor
        .read_bool4()
        .map_err(|error| format!("parsing {context} Class.HasPreprocessorData: {error}"))?;
    if has_preprocessor {
        read_sia(cursor, context, "Class.SuperClass")?;
        read_sia(cursor, context, "Class.CodeSuperClass")?;
        for _ in 0..7 {
            cursor
                .read_bool4()
                .map_err(|error| format!("parsing {context} Class.PreprocessorFlags: {error}"))?;
        }
        read_sia(cursor, context, "Class.ConfigName")?;
        read_sia(cursor, context, "Class.StaticClassGVName")?;
        cursor
            .read_bool4()
            .map_err(|error| format!("parsing {context} Class.Placeable: {error}"))?;
        skip_sia_array(cursor, "Class.MetaSpec", context)?;
        skip_sia_array(cursor, "Class.MetaValues", context)?;
        read_sia(cursor, context, "Class.ComposeOntoClassName")?;
    }
    let preprocessor_tail = preprocessor_start..cursor.pos();
    Ok(ClassRecord {
        name,
        namespace,
        prefix,
        methods_count_pos,
        methods,
        method_table,
        method_table_values,
        derived_and_shadow,
        constructors,
        factory_and_behavior_refs,
        behaviors_block,
        behaviors,
        behavior_types,
        preprocessor_tail,
    })
}

fn parse_property(cursor: &mut Cursor<'_>, context: &str) -> Result<(), String> {
    read_sia(cursor, context, "Property.Name")?;
    skip(cursor, DATA_TYPE_SIZE, context, "Property.Type")?;
    skip(cursor, 8, context, "Property.Visibility")?;
    let is_uproperty = cursor
        .read_bool4()
        .map_err(|error| format!("parsing {context} Property.IsUProperty: {error}"))?;
    if is_uproperty {
        skip_sia_array(cursor, "Property.MetaSpec", context)?;
        skip_sia_array(cursor, "Property.MetaValues", context)?;
        skip(cursor, 9 * 4, context, "Property.Flags1")?;
        let replicated = cursor
            .read_bool4()
            .map_err(|error| format!("parsing {context} Property.Replicated: {error}"))?;
        skip(cursor, 3 * 4, context, "Property.Flags2")?;
        if replicated {
            skip(cursor, 2 * 4, context, "Property.ReplicationFlags")?;
        }
        skip(cursor, 3 * 4, context, "Property.Flags3")?;
    }
    Ok(())
}

fn parse_function(cursor: &mut Cursor<'_>, context: &str) -> Result<FunctionRecord, String> {
    let start = cursor.pos();
    let name = read_sia(cursor, context, "Function.Name")?;
    let namespace = read_sia(cursor, context, "Function.Namespace")?;
    skip(cursor, DATA_TYPE_SIZE, context, "Function.ReturnType")?;
    skip_fixed_array(cursor, DATA_TYPE_SIZE, "Function.ParameterTypes", context)?;
    skip_sia_array(cursor, "Function.ParameterNames", context)?;
    skip_fixed_array(cursor, 4, "Function.ParameterFlags", context)?;
    skip_sia_array(cursor, "Function.ParameterDefaultArgs", context)?;
    let declaration = start..cursor.pos();
    let traits = cursor
        .read_i32()
        .map_err(|error| format!("parsing {context} Function.Traits: {error}"))?;
    let signature = start..cursor.pos();

    let code_count =
        bounded_count_with_limit(cursor, "Function.ByteCode", context, MAX_CODE_DWORDS)?;
    skip_product(cursor, code_count, 4, context, "Function.ByteCode")?;
    skip_fixed_array(cursor, 4, "Function.ByteCodeReferences", context)?;
    skip(cursor, 4, context, "Function.VariableSpace")?;
    skip_fixed_array(cursor, 8, "Function.ObjVariableTypes", context)?;
    skip_fixed_array(cursor, 4, "Function.ObjVariablePos", context)?;
    skip(cursor, 4, context, "Function.ObjVariablesOnHeap")?;
    skip_fixed_array(cursor, 4, "Function.VarInfoProgramPos", context)?;
    skip_fixed_array(cursor, 4, "Function.VarInfoOffset", context)?;
    skip_fixed_array(cursor, 4, "Function.VarInfoOption", context)?;
    skip(cursor, 4, context, "Function.StackNeeded")?;
    let id = cursor
        .read_i32()
        .map_err(|error| format!("parsing {context} Function.Id: {error}"))?;
    skip(cursor, 4, context, "Function.DeclaredAt")?;
    skip_fixed_array(cursor, 4, "Function.LineNumbers", context)?;

    let ufunction_start = cursor.pos();
    let is_ufunction = cursor
        .read_bool4()
        .map_err(|error| format!("parsing {context} Function.IsUFunction: {error}"))?;
    if is_ufunction {
        read_sia(cursor, context, "Function.UnrealFunctionName")?;
        skip_sia_array(cursor, "Function.MetaSpec", context)?;
        skip_sia_array(cursor, "Function.MetaValues", context)?;
        skip(cursor, 18 * 4, context, "Function.UnrealFlags")?;
    }
    let ufunction_tail = ufunction_start..cursor.pos();
    Ok(FunctionRecord {
        name,
        namespace,
        traits,
        id,
        raw: start..cursor.pos(),
        declaration,
        signature,
        ufunction_tail,
    })
}

fn parse_global(cursor: &mut Cursor<'_>, context: &str) -> Result<GlobalRecord, String> {
    let start = cursor.pos();
    read_sia(cursor, context, "Global.Name")?;
    read_sia(cursor, context, "Global.Namespace")?;
    skip(cursor, DATA_TYPE_SIZE, context, "Global.Type")?;
    let default_init = cursor
        .read_bool4()
        .map_err(|error| format!("parsing {context} Global.DefaultInit: {error}"))?;
    if !default_init {
        let pure_constant = cursor
            .read_bool4()
            .map_err(|error| format!("parsing {context} Global.PureConstant: {error}"))?;
        if pure_constant {
            skip(cursor, 8, context, "Global.PureConstantValue")?;
        } else {
            let has_init = cursor
                .read_bool4()
                .map_err(|error| format!("parsing {context} Global.HasInitFunction: {error}"))?;
            let declaration = start..cursor.pos();
            let function = parse_function(cursor, context)?;
            return Ok(GlobalRecord {
                declaration,
                initializer: has_init.then_some(function),
            });
        }
    }
    Ok(GlobalRecord {
        declaration: start..cursor.pos(),
        initializer: None,
    })
}

fn bounded_count(
    cursor: &mut Cursor<'_>,
    field: &'static str,
    context: &str,
) -> Result<usize, String> {
    bounded_count_with_limit(cursor, field, context, MAX_RECORDS)
}

fn bounded_count_with_limit(
    cursor: &mut Cursor<'_>,
    field: &'static str,
    context: &str,
    limit: usize,
) -> Result<usize, String> {
    let count = cursor
        .read_count(field)
        .map_err(|error| format!("parsing {context} {field}: {error}"))?;
    if count > limit {
        return Err(format!(
            "parsing {context} {field}: count {count} exceeds carry limit {limit}"
        ));
    }
    Ok(count)
}

fn bounded_count_with_minimum(
    cursor: &mut Cursor<'_>,
    field: &'static str,
    context: &str,
    minimum_element_bytes: usize,
) -> Result<usize, String> {
    let count = bounded_count(cursor, field, context)?;
    let minimum_bytes = count
        .checked_mul(minimum_element_bytes)
        .ok_or_else(|| format!("parsing {context} {field}: minimum byte count overflow"))?;
    if minimum_bytes > cursor.remaining() {
        return Err(format!(
            "parsing {context} {field}: count {count} requires at least {minimum_bytes} bytes, \
             only {} remain",
            cursor.remaining()
        ));
    }
    Ok(count)
}

fn reserved_vec<T>(count: usize, field: &str, context: &str) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|error| {
        format!("parsing {context} {field}: reserving {count} records failed: {error}")
    })?;
    Ok(values)
}

fn skip_fixed_array(
    cursor: &mut Cursor<'_>,
    width: usize,
    field: &'static str,
    context: &str,
) -> Result<(), String> {
    let count = bounded_count(cursor, field, context)?;
    skip_product(cursor, count, width, context, field)
}

fn skip_sia_array(
    cursor: &mut Cursor<'_>,
    field: &'static str,
    context: &str,
) -> Result<(), String> {
    let count = bounded_count_with_minimum(cursor, field, context, 4)?;
    for _ in 0..count {
        read_sia(cursor, context, field)?;
    }
    Ok(())
}

fn read_sia_array_ranges(
    cursor: &mut Cursor<'_>,
    field: &'static str,
    context: &str,
) -> Result<Vec<Range<usize>>, String> {
    let count = bounded_count_with_minimum(cursor, field, context, 4)?;
    let mut ranges = reserved_vec(count, field, context)?;
    for _ in 0..count {
        let start = cursor.pos();
        read_sia(cursor, context, field)?;
        ranges.push(start..cursor.pos());
    }
    Ok(ranges)
}

fn skip_product(
    cursor: &mut Cursor<'_>,
    count: usize,
    width: usize,
    context: &str,
    field: &str,
) -> Result<(), String> {
    let bytes = count
        .checked_mul(width)
        .ok_or_else(|| format!("parsing {context} {field}: byte count overflow"))?;
    skip(cursor, bytes, context, field)
}

fn skip(cursor: &mut Cursor<'_>, bytes: usize, context: &str, field: &str) -> Result<(), String> {
    cursor
        .skip(bytes)
        .map_err(|error| format!("parsing {context} {field}: {error}"))
}

fn read_sia(cursor: &mut Cursor<'_>, context: &str, field: &str) -> Result<String, String> {
    cursor
        .read_sia()
        .map_err(|error| format!("parsing {context} {field}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::header::CACHE_MAGIC;

    const MODULE: &str = "DefaultsFixture";
    const FILE: &str = "DefaultsFixture.as";
    const CLASS: &str = "UDefaultsFixture";

    #[derive(Clone)]
    struct MethodSpec<'a> {
        name: &'a str,
        traits: i32,
        code: &'a [i32],
    }

    fn sia(value: &str) -> Vec<u8> {
        if value.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut out = (value.len() as i32).to_le_bytes().to_vec();
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    fn datatype(type_ptr: i64, token: i32) -> Vec<u8> {
        let mut out = vec![0u8; 24];
        out.extend_from_slice(&type_ptr.to_le_bytes());
        out.extend_from_slice(&token.to_le_bytes());
        out
    }

    fn function(spec: &MethodSpec<'_>) -> Vec<u8> {
        function_with_id(spec, fixture_function_id(spec.name))
    }

    fn object_handle_datatype() -> Vec<u8> {
        let mut out = datatype(0, 5);
        out[8..12].copy_from_slice(&1i32.to_le_bytes());
        out
    }

    fn function_with_return(spec: &MethodSpec<'_>, return_type: &[u8]) -> Vec<u8> {
        function_with_return_and_id(spec, return_type, fixture_function_id(spec.name))
    }

    fn function_with_id(spec: &MethodSpec<'_>, id: i32) -> Vec<u8> {
        function_with_return_and_id(spec, &datatype(0, 0x52), id)
    }

    fn function_with_namespace(spec: &MethodSpec<'_>, namespace: &str) -> Vec<u8> {
        function_with_return_id_and_namespace(
            spec,
            &datatype(0, 0x52),
            fixture_function_id(spec.name),
            namespace,
        )
    }

    fn fixture_function_id(name: &str) -> i32 {
        name.bytes().fold(0x1357_2468u32, |hash, byte| {
            hash.rotate_left(5) ^ u32::from(byte)
        }) as i32
    }

    fn function_with_return_and_id(spec: &MethodSpec<'_>, return_type: &[u8], id: i32) -> Vec<u8> {
        function_with_return_id_and_namespace(spec, return_type, id, "")
    }

    fn function_with_return_id_and_namespace(
        spec: &MethodSpec<'_>,
        return_type: &[u8],
        id: i32,
        namespace: &str,
    ) -> Vec<u8> {
        let mut out = sia(spec.name);
        out.extend_from_slice(&sia(namespace));
        out.extend_from_slice(return_type);
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter types
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter names
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter flags
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter defaults
        out.extend_from_slice(&spec.traits.to_le_bytes());
        out.extend_from_slice(&(spec.code.len() as i32).to_le_bytes());
        for &dword in spec.code {
            out.extend_from_slice(&dword.to_le_bytes());
        }
        out.extend_from_slice(&0i32.to_le_bytes()); // bytecode references
        out.extend_from_slice(&0i32.to_le_bytes()); // variable space
        out.extend_from_slice(&0i32.to_le_bytes()); // object variable types
        out.extend_from_slice(&0i32.to_le_bytes()); // object variable positions
        out.extend_from_slice(&0i32.to_le_bytes()); // object variables on heap
        out.extend_from_slice(&0i32.to_le_bytes()); // var-info program positions
        out.extend_from_slice(&0i32.to_le_bytes()); // var-info offsets
        out.extend_from_slice(&0i32.to_le_bytes()); // var-info options
        out.extend_from_slice(&0i32.to_le_bytes()); // stack needed
        out.extend_from_slice(&id.to_le_bytes()); // id
        out.extend_from_slice(&0i32.to_le_bytes()); // declared at
        out.extend_from_slice(&0i32.to_le_bytes()); // line numbers
        out.extend_from_slice(&0i32.to_le_bytes()); // is UFUNCTION
        out
    }

    fn ufunction(spec: &MethodSpec<'_>, unreal_name: &str, flags: [i32; 18]) -> Vec<u8> {
        let mut out = function(spec);
        out.truncate(out.len() - 4); // replace bIsUFunction=false
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&sia(unreal_name));
        out.extend_from_slice(&0i32.to_le_bytes()); // metadata specifiers
        out.extend_from_slice(&0i32.to_le_bytes()); // metadata values
        for flag in flags {
            out.extend_from_slice(&flag.to_le_bytes());
        }
        out
    }

    fn function_raw<'a>(bytes: &'a [u8], function: &FunctionRecord) -> &'a [u8] {
        &bytes[CacheHeader::SIZE + function.raw.start..CacheHeader::SIZE + function.raw.end]
    }

    fn class_record(
        name: &str,
        namespace: &str,
        property_name: &str,
        methods: &[MethodSpec<'_>],
        method_table: &[i32],
        constructor_code: &[i32],
    ) -> Vec<u8> {
        class_record_with_behaviors(
            name,
            namespace,
            property_name,
            methods,
            method_table,
            constructor_code,
            &[],
        )
    }

    fn class_record_with_behaviors(
        name: &str,
        namespace: &str,
        property_name: &str,
        methods: &[MethodSpec<'_>],
        method_table: &[i32],
        constructor_code: &[i32],
        behaviors: &[MethodSpec<'_>],
    ) -> Vec<u8> {
        let mut out = sia(name);
        out.extend_from_slice(&sia(namespace));
        out.extend_from_slice(&0x1234i32.to_le_bytes()); // class flags
        out.extend_from_slice(&1i32.to_le_bytes()); // properties
        out.extend_from_slice(&sia(property_name));
        out.extend_from_slice(&datatype(0, 0x44)); // int
        out.extend_from_slice(&0i32.to_le_bytes()); // private
        out.extend_from_slice(&0i32.to_le_bytes()); // protected
        out.extend_from_slice(&0i32.to_le_bytes()); // not UPROPERTY
        out.extend_from_slice(&(methods.len() as i32).to_le_bytes());
        for method in methods {
            out.extend_from_slice(&function(method));
        }
        out.extend_from_slice(&(method_table.len() as i32).to_le_bytes());
        for &slot in method_table {
            out.extend_from_slice(&slot.to_le_bytes());
        }
        out.extend_from_slice(&0i64.to_le_bytes()); // DerivedFrom
        out.extend_from_slice(&0i64.to_le_bytes()); // ShadowType
        out.extend_from_slice(&1i32.to_le_bytes()); // constructors
        out.extend_from_slice(&function(&MethodSpec {
            name,
            traits: 0,
            code: constructor_code,
        }));
        out.extend_from_slice(&0i32.to_le_bytes()); // factory refs
        out.extend_from_slice(&0i32.to_le_bytes()); // behavior refs
        out.extend_from_slice(&(behaviors.len() as i32).to_le_bytes());
        for behavior in behaviors {
            out.extend_from_slice(&function(behavior));
        }
        out.extend_from_slice(&(behaviors.len() as i32).to_le_bytes());
        for _ in behaviors {
            out.extend_from_slice(&0i32.to_le_bytes());
        }
        out.extend_from_slice(&0i32.to_le_bytes()); // no preprocessor metadata
        out
    }

    fn class_record_with_raw_methods(name: &str, methods: &[Vec<u8>]) -> Vec<u8> {
        let mut out = sia(name);
        out.extend_from_slice(&sia(""));
        out.extend_from_slice(&0x1234i32.to_le_bytes()); // class flags
        out.extend_from_slice(&1i32.to_le_bytes()); // properties
        out.extend_from_slice(&sia("Value"));
        out.extend_from_slice(&datatype(0, 0x44));
        out.extend_from_slice(&0i32.to_le_bytes()); // private
        out.extend_from_slice(&0i32.to_le_bytes()); // protected
        out.extend_from_slice(&0i32.to_le_bytes()); // not UPROPERTY
        out.extend_from_slice(&(methods.len() as i32).to_le_bytes());
        for method in methods {
            out.extend_from_slice(method);
        }
        out.extend_from_slice(&(methods.len() as i32).to_le_bytes());
        for index in 0..methods.len() {
            out.extend_from_slice(&(index as i32).to_le_bytes());
        }
        out.extend_from_slice(&0i64.to_le_bytes()); // DerivedFrom
        out.extend_from_slice(&0i64.to_le_bytes()); // ShadowType
        out.extend_from_slice(&0i32.to_le_bytes()); // constructors
        out.extend_from_slice(&0i32.to_le_bytes()); // factory refs
        out.extend_from_slice(&0i32.to_le_bytes()); // behavior refs
        out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
        out.extend_from_slice(&0i32.to_le_bytes()); // behavior types
        out.extend_from_slice(&0i32.to_le_bytes()); // no preprocessor metadata
        out
    }

    fn cache(classes: &[Vec<u8>], free_code: &[i32], code_hash: i64) -> Vec<u8> {
        cache_with_functions(
            classes,
            &[function(&MethodSpec {
                name: "FreeFunction",
                traits: 0,
                code: free_code,
            })],
            code_hash,
        )
    }

    fn cache_with_functions(classes: &[Vec<u8>], functions: &[Vec<u8>], code_hash: i64) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(MODULE));
        out.extend_from_slice(&sia(MODULE));
        out.extend_from_slice(&(functions.len() as i32).to_le_bytes());
        for function in functions {
            out.extend_from_slice(function);
        }
        out.extend_from_slice(&(classes.len() as i32).to_le_bytes());
        for class in classes {
            out.extend_from_slice(class);
        }
        out.extend_from_slice(&0i32.to_le_bytes()); // enums
        out.extend_from_slice(&0i32.to_le_bytes()); // globals
        out.extend_from_slice(&0i32.to_le_bytes()); // imports
        out.extend_from_slice(&code_hash.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
        out.extend_from_slice(&sia("")); // statics class
        out.extend_from_slice(&0i32.to_le_bytes()); // events
        out.extend_from_slice(&0i32.to_le_bytes()); // delegates
        out.extend_from_slice(&sia(FILE));
        out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
        for _ in 0..super::super::tables::N_TABLES {
            out.extend_from_slice(&0i32.to_le_bytes());
        }
        out
    }

    fn import_record(module: &str, name: &str, namespace: &str) -> Vec<u8> {
        let mut out = sia(module);
        out.extend_from_slice(&sia(name));
        out.extend_from_slice(&sia(namespace));
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter types
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter flags
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter defaults
        out.extend_from_slice(&datatype(0, 0x44)); // return type
        out
    }

    fn cache_with_imports(imports: &[Vec<u8>], imported_modules: &[&str]) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(MODULE));
        out.extend_from_slice(&sia(MODULE));
        out.extend_from_slice(&1i32.to_le_bytes()); // functions
        out.extend_from_slice(&function(&MethodSpec {
            name: "FreeFunction",
            traits: 0,
            code: &[10],
        }));
        out.extend_from_slice(&0i32.to_le_bytes()); // classes
        out.extend_from_slice(&0i32.to_le_bytes()); // enums
        out.extend_from_slice(&0i32.to_le_bytes()); // globals
        out.extend_from_slice(&(imports.len() as i32).to_le_bytes());
        for import in imports {
            out.extend_from_slice(import);
        }
        out.extend_from_slice(&0i64.to_le_bytes()); // code hash
        out.extend_from_slice(&(imported_modules.len() as i32).to_le_bytes());
        for module in imported_modules {
            out.extend_from_slice(&sia(module));
        }
        out.extend_from_slice(&sia("")); // statics class
        out.extend_from_slice(&0i32.to_le_bytes()); // events
        out.extend_from_slice(&0i32.to_le_bytes()); // delegates
        out.extend_from_slice(&sia(FILE));
        out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
        for _ in 0..super::super::tables::N_TABLES {
            out.extend_from_slice(&0i32.to_le_bytes());
        }
        out
    }

    fn global_record(name: &str) -> Vec<u8> {
        let mut out = sia(name);
        out.extend_from_slice(&sia(""));
        out.extend_from_slice(&datatype(0, 0x44)); // int
        out.extend_from_slice(&1i32.to_le_bytes()); // default initialization
        out
    }

    fn cache_with_globals(globals: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(MODULE));
        out.extend_from_slice(&sia(MODULE));
        out.extend_from_slice(&0i32.to_le_bytes()); // functions
        out.extend_from_slice(&0i32.to_le_bytes()); // classes
        out.extend_from_slice(&0i32.to_le_bytes()); // enums
        out.extend_from_slice(&(globals.len() as i32).to_le_bytes());
        for global in globals {
            out.extend_from_slice(global);
        }
        out.extend_from_slice(&0i32.to_le_bytes()); // imports
        out.extend_from_slice(&0i64.to_le_bytes()); // code hash
        out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
        out.extend_from_slice(&sia("")); // statics class
        out.extend_from_slice(&0i32.to_le_bytes()); // events
        out.extend_from_slice(&0i32.to_le_bytes()); // delegates
        out.extend_from_slice(&sia(FILE));
        out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
        for _ in 0..super::super::tables::N_TABLES {
            out.extend_from_slice(&0i32.to_le_bytes());
        }
        out
    }

    fn base_cache() -> Vec<u8> {
        cache(
            &[class_record(
                CLASS,
                "",
                "Value",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 0,
                        code: &[10],
                    },
                    MethodSpec {
                        name: "__InitDefaults",
                        traits: 0x40000,
                        code: &[2, 42, 10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[10],
                    },
                ],
                &[-1, 0, -1, 1, 2],
                &[10],
            )],
            &[10],
            0x1111,
        )
    }

    fn regen_cache() -> Vec<u8> {
        cache(
            &[class_record(
                CLASS,
                "",
                "Value",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 0,
                        code: &[2, 7, 10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[2, 9, 10],
                    },
                ],
                &[-1, 0, -1, 1],
                &[2, 3, 10],
            )],
            &[2, 5, 10],
            0x2222,
        )
    }

    fn prepare(base: &[u8]) -> Result<GeneratedDefaultsPlan, String> {
        let modules = model::parse_modules(base).map_err(|error| error.to_string())?;
        GeneratedDefaultsPlan::prepare(base, &modules, MODULE)?
            .ok_or_else(|| "fixture unexpectedly has no defaults plan".to_string())
    }

    #[test]
    fn hybrid_carry_preserves_existing_defaults_and_retains_appended_class_defaults() {
        let base = base_cache();
        let modules = model::parse_modules(&base).unwrap();
        let authored = HashSet::from(["UNewDialogTopic".to_owned()]);
        let plan = GeneratedDefaultsPlan::prepare_hybrid(&base, &modules, MODULE, &authored)
            .unwrap()
            .expect("base fixture has generated defaults");
        assert!(plan.allows_new_symbols());

        let regen = cache(
            &[
                class_record(
                    CLASS,
                    "",
                    "Value",
                    &[
                        MethodSpec {
                            name: "Before",
                            traits: 0,
                            code: &[2, 7, 10],
                        },
                        MethodSpec {
                            name: "After",
                            traits: 4,
                            code: &[2, 9, 10],
                        },
                    ],
                    &[-1, 0, -1, 1],
                    &[2, 3, 10],
                ),
                class_record_with_raw_methods(
                    "UNewDialogTopic",
                    &[function_with_id(
                        &MethodSpec {
                            name: "__InitDefaults",
                            traits: 0x40000,
                            code: &[2, 99, 10],
                        },
                        0x6a6a_0001,
                    )],
                ),
            ],
            &[2, 5, 10],
            0x2222,
        );
        let out = plan.apply(&regen).unwrap();
        let parse = |bytes: &[u8], context| {
            let end = module_region_end(bytes).unwrap();
            parse_entry(&bytes[CacheHeader::SIZE..end], context).unwrap()
        };
        let base_entry = parse(&base, "hybrid base");
        let regen_entry = parse(&regen, "hybrid regen");
        let out_entry = parse(&out, "hybrid out");
        let base_defaults = &base_entry.classes[0].methods[1];
        let out_defaults = &out_entry.classes[0].methods[1];
        assert_eq!(
            function_raw(&out, out_defaults),
            function_raw(&base, base_defaults),
            "the existing class keeps its byte-exact base defaults"
        );
        let regen_new_defaults = &regen_entry.classes[1].methods[0];
        let out_new_defaults = &out_entry.classes[1].methods[0];
        assert_eq!(
            function_raw(&out, out_new_defaults),
            function_raw(&regen, regen_new_defaults),
            "the appended class keeps compiler-authored defaults"
        );
    }

    #[test]
    fn hybrid_carry_rejects_authored_defaults_for_an_existing_base_class() {
        let base = base_cache();
        let modules = model::parse_modules(&base).unwrap();
        let authored = HashSet::from([CLASS.to_owned(), "UNewDialogTopic".to_owned()]);
        let error = GeneratedDefaultsPlan::prepare_hybrid(&base, &modules, MODULE, &authored)
            .unwrap_err();
        assert!(
            error.contains("refuses authored defaults for existing base class"),
            "{error}"
        );
    }

    #[test]
    fn hybrid_carry_preserves_existing_generated_free_wrapper_and_new_static_class() {
        const TOPIC: &str = "UGoreDialogTopic";
        let base_class = class_record(
            CLASS,
            "",
            "Value",
            &[MethodSpec {
                name: "__InitDefaults",
                traits: 0x40000,
                code: &[2, 42, 10],
            }],
            &[0],
            &[10],
        );
        let ordinary_base = function(&MethodSpec {
            name: "FreeFunction",
            traits: 0,
            code: &[10],
        });
        let wrapper_base = function_with_return(
            &MethodSpec {
                name: "Get",
                traits: 0,
                code: &[2, 41, 10],
            },
            &object_handle_datatype(),
        );
        let base = cache_with_functions(&[base_class], &[ordinary_base, wrapper_base], 1);
        let modules = model::parse_modules(&base).unwrap();
        let plan = GeneratedDefaultsPlan::prepare_hybrid(
            &base,
            &modules,
            MODULE,
            &HashSet::from([TOPIC.to_owned()]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(plan.generated_free_indices, HashSet::from([1]));

        let ordinary_regen = function(&MethodSpec {
            name: "FreeFunction",
            traits: 0,
            code: &[2, 7, 10],
        });
        let new_static_class = function_with_namespace(
            &MethodSpec {
                name: "StaticClass",
                traits: 0,
                code: &[2, 77, 10],
            },
            TOPIC,
        );
        let wrapper_regen = function_with_return(
            &MethodSpec {
                name: "Get",
                traits: 0,
                code: &[2, 99, 10],
            },
            &object_handle_datatype(),
        );
        let regen = cache_with_functions(
            &[
                class_record(CLASS, "", "Value", &[], &[], &[2, 8, 10]),
                class_record_with_raw_methods(
                    TOPIC,
                    &[function_with_id(
                        &MethodSpec {
                            name: "__InitDefaults",
                            traits: 0x40000,
                            code: &[2, 99, 10],
                        },
                        0x6a6a_0001,
                    )],
                ),
            ],
            &[ordinary_regen, new_static_class, wrapper_regen],
            2,
        );
        let carried = plan.apply(&regen).unwrap();
        let parse = |bytes: &[u8], context| {
            let end = module_region_end(bytes).unwrap();
            parse_entry(&bytes[CacheHeader::SIZE..end], context).unwrap()
        };
        let base_entry = parse(&base, "hybrid wrapper base");
        let regen_entry = parse(&regen, "hybrid wrapper regen");
        let carried_entry = parse(&carried, "hybrid wrapper carried");
        assert_eq!(
            function_raw(&carried, &carried_entry.functions[0]),
            function_raw(&regen, &regen_entry.functions[0]),
            "ordinary existing free function remains regenerated"
        );
        assert_eq!(
            function_raw(&carried, &carried_entry.functions[1]),
            function_raw(&regen, &regen_entry.functions[1]),
            "new-class StaticClass remains compiler-authored"
        );
        assert_eq!(
            function_raw(&carried, &carried_entry.functions[2]),
            function_raw(&base, &base_entry.functions[1]),
            "existing emitter-omitted Get wrapper remains byte-exact"
        );
    }

    #[test]
    fn hybrid_carry_output_fails_structure_plan_when_existing_property_changes() {
        const TOPIC: &str = "UGoreDialogTopic";
        let base = base_cache();
        let modules = model::parse_modules(&base).unwrap();
        let carry = GeneratedDefaultsPlan::prepare_hybrid(
            &base,
            &modules,
            MODULE,
            &HashSet::from([TOPIC.to_owned()]),
        )
        .unwrap()
        .unwrap();
        let structure = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();
        let regen = cache(
            &[
                class_record(
                    CLASS,
                    "",
                    "ChangedValue",
                    &[
                        MethodSpec {
                            name: "Before",
                            traits: 0,
                            code: &[2, 7, 10],
                        },
                        MethodSpec {
                            name: "After",
                            traits: 4,
                            code: &[2, 9, 10],
                        },
                    ],
                    &[-1, 0, -1, 1],
                    &[2, 3, 10],
                ),
                class_record_with_raw_methods(
                    TOPIC,
                    &[function_with_id(
                        &MethodSpec {
                            name: "__InitDefaults",
                            traits: 0x40000,
                            code: &[2, 99, 10],
                        },
                        0x6a6a_0001,
                    )],
                ),
            ],
            &[2, 5, 10],
            0x2222,
        );
        let carried = carry.apply(&regen).unwrap();
        let error = structure.verify(&carried).unwrap_err();
        assert!(error.contains("flags/properties"), "{error}");
    }

    #[test]
    fn existing_edit_preserves_shipping_function_and_unreal_traits_only() {
        const PROPERTY_TRAIT: i32 = 0x200;
        let act = MethodSpec {
            name: "Act_Implementation",
            traits: PROPERTY_TRAIT | 0x40,
            code: &[10],
        };
        let mut shipping_flags = [0i32; 18];
        shipping_flags[0] = 0; // BlueprintCallable
        shipping_flags[1] = 1; // BlueprintOverride
        shipping_flags[2] = 1; // BlueprintEvent
        let base = cache(
            &[class_record_with_raw_methods(
                "UChoiceDiegoGamestart",
                &[ufunction(&act, "Act", shipping_flags)],
            )],
            &[10],
            1,
        );

        let regenerated_act = MethodSpec {
            name: "Act_Implementation",
            traits: 0,
            code: &[2, 77, 10],
        };
        let added = MethodSpec {
            name: "Added",
            traits: 0x20,
            code: &[2, 88, 10],
        };
        let new_class_act = MethodSpec {
            name: "Act_Implementation",
            traits: 0x20,
            code: &[2, 99, 10],
        };
        let regen = cache(
            &[
                class_record_with_raw_methods(
                    "UChoiceDiegoGamestart",
                    &[
                        ufunction(&regenerated_act, "WrongAct", [0; 18]),
                        ufunction(&added, "Added", [1; 18]),
                    ],
                ),
                class_record_with_raw_methods(
                    "UNewTopic",
                    &[ufunction(&new_class_act, "NewAct", [1; 18])],
                ),
            ],
            &[2, 55, 10],
            2,
        );
        let plan = ExistingFunctionMetadataPlan::prepare(&base, MODULE).unwrap();
        let out = plan.apply(&regen).unwrap();

        let parse = |bytes: &[u8], context| {
            let end = module_region_end(bytes).unwrap();
            parse_entry(&bytes[CacheHeader::SIZE..end], context).unwrap()
        };
        let base_entry = parse(&base, "metadata base");
        let regen_entry = parse(&regen, "metadata regen");
        let out_entry = parse(&out, "metadata output");
        let base_act = &base_entry.classes[0].methods[0];
        let regen_act = &regen_entry.classes[0].methods[0];
        let out_act = &out_entry.classes[0].methods[0];
        assert_eq!(out_act.traits, PROPERTY_TRAIT | 0x40);
        assert_eq!(
            &out[CacheHeader::SIZE + out_act.ufunction_tail.start
                ..CacheHeader::SIZE + out_act.ufunction_tail.end],
            &base[CacheHeader::SIZE + base_act.ufunction_tail.start
                ..CacheHeader::SIZE + base_act.ufunction_tail.end]
        );
        assert_eq!(
            &out[CacheHeader::SIZE + out_act.signature.end
                ..CacheHeader::SIZE + out_act.ufunction_tail.start],
            &regen[CacheHeader::SIZE + regen_act.signature.end
                ..CacheHeader::SIZE + regen_act.ufunction_tail.start]
        );
        let mut tail = Cursor::new(
            &out[CacheHeader::SIZE + out_act.ufunction_tail.start
                ..CacheHeader::SIZE + out_act.ufunction_tail.end],
        );
        assert!(tail.read_bool4().unwrap());
        assert_eq!(tail.read_sia().unwrap(), "Act");
        assert_eq!(tail.read_i32().unwrap(), 0); // MetaSpec count
        assert_eq!(tail.read_i32().unwrap(), 0); // MetaValues count
        assert_eq!(tail.read_i32().unwrap(), 0); // Callable
        assert_eq!(tail.read_i32().unwrap(), 1); // BlueprintOverride
        assert_eq!(tail.read_i32().unwrap(), 1); // BlueprintEvent

        // A new method on the old class and every method on a new class are byte-exact regen.
        for (class_index, method_index) in [(0, 1), (1, 0)] {
            let regenerated = &regen_entry.classes[class_index].methods[method_index];
            let output = &out_entry.classes[class_index].methods[method_index];
            assert_eq!(
                &out[CacheHeader::SIZE + output.raw.start..CacheHeader::SIZE + output.raw.end],
                &regen[CacheHeader::SIZE + regenerated.raw.start
                    ..CacheHeader::SIZE + regenerated.raw.end]
            );
        }
    }

    #[test]
    fn existing_edit_rejects_missing_or_ambiguous_shipping_function_identity() {
        let base = cache(
            &[class_record_with_raw_methods(
                CLASS,
                &[function(&MethodSpec {
                    name: "Existing",
                    traits: 0x200,
                    code: &[10],
                })],
            )],
            &[10],
            1,
        );
        let plan = ExistingFunctionMetadataPlan::prepare(&base, MODULE).unwrap();
        let missing = cache(&[class_record_with_raw_methods(CLASS, &[])], &[10], 2);
        assert!(plan
            .apply(&missing)
            .unwrap_err()
            .contains("missing existing method"));

        let duplicate_method = function(&MethodSpec {
            name: "Existing",
            traits: 0,
            code: &[10],
        });
        let ambiguous = cache(
            &[class_record_with_raw_methods(
                CLASS,
                &[duplicate_method.clone(), duplicate_method],
            )],
            &[10],
            3,
        );
        assert!(plan
            .apply(&ambiguous)
            .unwrap_err()
            .contains("ambiguous duplicate method"));
    }

    #[test]
    fn existing_metadata_normalization_composes_with_generated_default_carry() {
        let mut shipping_flags = [0i32; 18];
        shipping_flags[1] = 1; // BlueprintOverride
        shipping_flags[2] = 1; // BlueprintEvent
        let shipping_act = MethodSpec {
            name: "Act_Implementation",
            traits: 0x240,
            code: &[10],
        };
        let init_defaults = MethodSpec {
            name: "__InitDefaults",
            traits: 0x40000,
            code: &[2, 42, 10],
        };
        let base = cache(
            &[class_record_with_raw_methods(
                CLASS,
                &[
                    ufunction(&shipping_act, "Act", shipping_flags),
                    function(&init_defaults),
                ],
            )],
            &[10],
            1,
        );

        let regenerated_act = MethodSpec {
            name: "Act_Implementation",
            traits: 0,
            code: &[2, 77, 10],
        };
        let regen = cache(
            &[class_record_with_raw_methods(
                CLASS,
                &[ufunction(&regenerated_act, "Act_Implementation", [0; 18])],
            )],
            &[2, 55, 10],
            2,
        );

        let metadata = ExistingFunctionMetadataPlan::prepare(&base, MODULE).unwrap();
        let defaults = prepare(&base).unwrap();
        assert!(metadata
            .apply(&regen)
            .unwrap_err()
            .contains("missing existing method"));
        assert!(defaults
            .apply(&regen)
            .unwrap_err()
            .contains("declaration/signature drift"));

        let normalized = metadata.apply_present(&regen).unwrap();
        let carried = defaults.apply(&normalized).unwrap();
        let out = metadata.apply(&carried).unwrap();

        let parse = |bytes: &[u8], context| {
            let end = module_region_end(bytes).unwrap();
            parse_entry(&bytes[CacheHeader::SIZE..end], context).unwrap()
        };
        let base_entry = parse(&base, "carry metadata base");
        let regen_entry = parse(&regen, "carry metadata regen");
        let out_entry = parse(&out, "carry metadata output");
        let base_act = &base_entry.classes[0].methods[0];
        let base_defaults = &base_entry.classes[0].methods[1];
        let regen_act = &regen_entry.classes[0].methods[0];
        let out_act = &out_entry.classes[0].methods[0];
        let out_defaults = &out_entry.classes[0].methods[1];
        assert_eq!(out_act.traits, shipping_act.traits);
        assert_eq!(
            &out[CacheHeader::SIZE + out_act.ufunction_tail.start
                ..CacheHeader::SIZE + out_act.ufunction_tail.end],
            &base[CacheHeader::SIZE + base_act.ufunction_tail.start
                ..CacheHeader::SIZE + base_act.ufunction_tail.end]
        );
        assert_eq!(
            &out[CacheHeader::SIZE + out_act.signature.end
                ..CacheHeader::SIZE + out_act.ufunction_tail.start],
            &regen[CacheHeader::SIZE + regen_act.signature.end
                ..CacheHeader::SIZE + regen_act.ufunction_tail.start]
        );
        assert_eq!(
            function_raw(&out, out_defaults),
            function_raw(&base, base_defaults)
        );
    }

    #[test]
    fn existing_module_structure_allows_an_appended_dialog_topic_class() {
        let methods = [MethodSpec {
            name: "Act",
            traits: 0,
            code: &[10],
        }];
        let base = cache(
            &[class_record(CLASS, "", "Value", &methods, &[0], &[10])],
            &[10],
            0x1111,
        );
        // This is the shape used by a new topic in an existing conversation module: the old
        // class stays intact and the compiler appends a complete new class.
        let regen = cache(
            &[
                class_record(CLASS, "", "Value", &methods, &[0], &[77, 10]),
                class_record("UGoreDialogTopic", "", "Caption", &methods, &[0], &[10]),
            ],
            &[77, 10],
            0x2222,
        );
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();
        plan.verify(&regen).unwrap();
    }

    #[test]
    fn existing_module_structure_rejects_existing_property_or_class_flag_drift() {
        let methods = [MethodSpec {
            name: "Act",
            traits: 0,
            code: &[10],
        }];
        let base = cache(
            &[class_record(CLASS, "", "Value", &methods, &[0], &[10])],
            &[10],
            0x1111,
        );
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();

        let property_changed = cache(
            &[class_record(CLASS, "", "Different", &methods, &[0], &[10])],
            &[10],
            0x1111,
        );
        assert!(plan
            .verify(&property_changed)
            .unwrap_err()
            .contains("flags/properties drift"));

        let mut flags_changed = base.clone();
        let old = 0x1234i32.to_le_bytes();
        let pos = flags_changed
            .windows(old.len())
            .position(|window| window == old)
            .expect("fixture class flags");
        flags_changed[pos..pos + old.len()].copy_from_slice(&0x1235i32.to_le_bytes());
        assert!(plan
            .verify(&flags_changed)
            .unwrap_err()
            .contains("flags/properties drift"));
    }

    #[test]
    fn existing_module_structure_rejects_existing_method_table_drift() {
        let methods = [MethodSpec {
            name: "Act",
            traits: 0,
            code: &[10],
        }];
        let base = cache(
            &[class_record(CLASS, "", "Value", &methods, &[0], &[10])],
            &[10],
            0x1111,
        );
        let drifted = cache(
            &[class_record(CLASS, "", "Value", &methods, &[-1], &[10])],
            &[10],
            0x1111,
        );
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();
        assert!(plan
            .verify(&drifted)
            .unwrap_err()
            .contains("MethodTable drift"));
    }

    #[test]
    fn existing_module_structure_allows_appended_imports_but_rejects_reordering() {
        let old_import = import_record("Shipping.Provider", "OldFunction", "");
        let new_import = import_record("Gore.NewProvider", "NewFunction", "");
        let base = cache_with_imports(&[old_import.clone()], &["Shipping.Provider"]);
        let appended = cache_with_imports(
            &[old_import.clone(), new_import.clone()],
            &["Shipping.Provider", "Gore.NewProvider"],
        );
        let reordered = cache_with_imports(
            &[new_import, old_import],
            &["Gore.NewProvider", "Shipping.Provider"],
        );
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();
        plan.verify(&appended).unwrap();
        assert!(plan
            .verify(&reordered)
            .unwrap_err()
            .contains("module imports[0] drift"));
    }

    #[test]
    fn existing_module_structure_allows_appended_globals_but_rejects_prepend_or_interleave() {
        let old_first = global_record("OldFirst");
        let old_second = global_record("OldSecond");
        let new_global = global_record("NewGlobal");
        let base = cache_with_globals(&[old_first.clone(), old_second.clone()]);
        let appended = cache_with_globals(&[
            old_first.clone(),
            old_second.clone(),
            new_global.clone(),
        ]);
        let prepended = cache_with_globals(&[
            new_global.clone(),
            old_first.clone(),
            old_second.clone(),
        ]);
        let interleaved = cache_with_globals(&[old_first, new_global, old_second]);
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();

        plan.verify(&appended).unwrap();
        assert!(plan
            .verify(&prepended)
            .unwrap_err()
            .contains("module globals[0] drift"));
        assert!(plan
            .verify(&interleaved)
            .unwrap_err()
            .contains("module globals[1] drift"));
    }

    #[test]
    fn existing_module_structure_allows_only_new_class_static_class_helper_between_free_functions() {
        let first = MethodSpec {
            name: "First",
            traits: 0,
            code: &[10],
        };
        let second = MethodSpec {
            name: "Second",
            traits: 0,
            code: &[10],
        };
        let helper = MethodSpec {
            name: "StaticClass",
            traits: 0,
            code: &[10],
        };
        let topic = "UGoreDialogTopic";
        let base = cache_with_functions(&[], &[function(&first), function(&second)], 0x1111);
        let regen = cache_with_functions(
            &[class_record(topic, "", "Caption", &[], &[], &[10])],
            &[
                function(&first),
                function_with_namespace(&helper, topic),
                function(&second),
            ],
            0x2222,
        );
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();
        plan.verify(&regen).unwrap();
    }

    #[test]
    fn existing_module_structure_rejects_authored_new_class_function_between_free_functions() {
        let first = MethodSpec {
            name: "First",
            traits: 0,
            code: &[10],
        };
        let second = MethodSpec {
            name: "Second",
            traits: 0,
            code: &[10],
        };
        let sneaky = MethodSpec {
            name: "Sneaky",
            traits: 0,
            code: &[10],
        };
        let topic = "UGoreDialogTopic";
        let base = cache_with_functions(&[], &[function(&first), function(&second)], 0x1111);
        let regen = cache_with_functions(
            &[class_record(topic, "", "Caption", &[sneaky.clone()], &[0], &[10])],
            &[
                function(&first),
                function_with_namespace(&sneaky, topic),
                function(&second),
            ],
            0x2222,
        );
        let plan = ExistingModuleStructurePlan::prepare(&base, MODULE).unwrap();
        assert!(plan
            .verify(&regen)
            .unwrap_err()
            .contains("inserted unsupported declaration"));
    }

    #[test]
    #[ignore = "requires GORE_AS_CACHE and GORE_AS_FULL_GRAPH safe-cross-module fixtures"]
    fn real_full_graph_cross_module_dialog_preserves_existing_structure() {
        use crate::cache::remap::RemapOptions;
        use crate::cache::splice::{
            extract_module, remap_module_to_base_with_options, SequentialMiniGuard,
        };

        const PROVIDER: &str = "Gore.FullGraph.DiegoProbe";
        const DIALOG: &str = "Story.G1R.Conversation.Conversation_OC_STT_DIEGO";
        let base =
            std::fs::read(std::env::var_os("GORE_AS_CACHE").expect("set GORE_AS_CACHE")).unwrap();
        let full_graph =
            std::fs::read(std::env::var_os("GORE_AS_FULL_GRAPH").expect("set GORE_AS_FULL_GRAPH"))
                .unwrap();

        // The edit imports the new provider.  Compose the provider first, exactly as the
        // fixed-point selective FullGraph path does before retrying the dependent dialog edit.
        let provider = extract_module(&full_graph, PROVIDER).unwrap();
        let provider = remap_module_to_base_with_options(
            &provider,
            &base,
            RemapOptions {
                allow_new_symbols: true,
            },
        )
        .unwrap();
        let mut guard = SequentialMiniGuard::new(&base).unwrap();
        let running = guard.compose_add(&base, &provider).unwrap();

        let dialog = extract_module(&full_graph, DIALOG).unwrap();
        let dialog = remap_module_to_base_with_options(
            &dialog,
            &running,
            RemapOptions {
                allow_new_symbols: true,
            },
        )
        .unwrap();
        let metadata = ExistingFunctionMetadataPlan::prepare(&base, DIALOG).unwrap();
        let structure = ExistingModuleStructurePlan::prepare(&base, DIALOG).unwrap();
        let dialog = metadata.apply(&dialog).unwrap();
        structure.verify(&dialog).unwrap();
        guard = SequentialMiniGuard::new(&running).unwrap();
        guard.compose_edit(&running, &dialog, DIALOG).unwrap();
    }

    #[test]
    #[ignore = "requires GORE_AS_CACHE and GORE_AS_DIALOG_MINI from a strict standalone edit"]
    fn real_dialog_edit_has_complete_shipping_function_metadata() {
        let base_path = std::env::var_os("GORE_AS_CACHE").expect("set GORE_AS_CACHE");
        let mini_path = std::env::var_os("GORE_AS_DIALOG_MINI").expect("set GORE_AS_DIALOG_MINI");
        let module = std::env::var("GORE_AS_DIALOG_MODULE")
            .unwrap_or_else(|_| "Story.G1R.Conversation.Conversation_OC_STT_DIEGO".to_string());
        let base = std::fs::read(base_path).unwrap();
        let mini = std::fs::read(mini_path).unwrap();
        let plan = ExistingFunctionMetadataPlan::prepare(&base, &module).unwrap();

        // `apply` verifies that every Shipping function identity is present, that its traits and
        // complete Unreal descriptor equal the base, and that genuinely new records stay intact.
        // A correctly produced mini is therefore an exact fixed point of the preservation pass.
        assert_eq!(plan.apply(&mini).unwrap(), mini);
    }

    #[test]
    fn carries_non_tail_generated_record_and_complex_method_table_byte_exact() {
        let base = base_cache();
        let regen = regen_cache();
        let plan = prepare(&base).unwrap();
        assert_eq!(plan.generated_count(), 1);
        let carried = plan.apply(&regen).unwrap();

        let carried_end = module_region_end(&carried).unwrap();
        let carried_entry = parse_entry(&carried[CacheHeader::SIZE..carried_end], "test").unwrap();
        let base_end = module_region_end(&base).unwrap();
        let base_entry = parse_entry(&base[CacheHeader::SIZE..base_end], "test base").unwrap();
        let regen_end = module_region_end(&regen).unwrap();
        let regen_entry = parse_entry(&regen[CacheHeader::SIZE..regen_end], "test regen").unwrap();

        let out_class = &carried_entry.classes[0];
        assert_eq!(
            out_class
                .methods
                .iter()
                .map(|method| method.name.as_str())
                .collect::<Vec<_>>(),
            ["Before", "__InitDefaults", "After"]
        );
        assert_eq!(out_class.method_table_values, [-1, 0, -1, 1, 2]);
        assert_eq!(
            &carried[CacheHeader::SIZE + out_class.methods[1].raw.start
                ..CacheHeader::SIZE + out_class.methods[1].raw.end],
            &base[CacheHeader::SIZE + base_entry.classes[0].methods[1].raw.start
                ..CacheHeader::SIZE + base_entry.classes[0].methods[1].raw.end]
        );
        // Edited non-generated bodies and constructor/free-function records remain the regen's.
        assert_eq!(
            &carried[CacheHeader::SIZE + out_class.methods[0].raw.start
                ..CacheHeader::SIZE + out_class.methods[0].raw.end],
            &regen[CacheHeader::SIZE + regen_entry.classes[0].methods[0].raw.start
                ..CacheHeader::SIZE + regen_entry.classes[0].methods[0].raw.end]
        );
        assert_eq!(&carried[carried_end..], &regen[regen_end..]);
    }

    #[test]
    fn accepts_unique_per_build_function_record_id_drift() {
        // FunctionRecord.Id is a signed, per-build identifier distinct from the positive T4
        // engine-ID table. Real Shipping/regen pairs (including Biter) legitimately drift here.
        let drifted_id = 0x6a01_0203;
        let drifted_regen = cache_with_functions(
            &[class_record(
                CLASS,
                "",
                "Value",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 0,
                        code: &[2, 7, 10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[2, 9, 10],
                    },
                ],
                &[-1, 0, -1, 1],
                &[2, 3, 10],
            )],
            &[function_with_id(
                &MethodSpec {
                    name: "FreeFunction",
                    traits: 0,
                    code: &[2, 5, 10],
                },
                drifted_id,
            )],
            0x2222,
        );
        let carried = prepare(&base_cache())
            .unwrap()
            .apply(&drifted_regen)
            .unwrap();
        let end = module_region_end(&carried).unwrap();
        let entry = parse_entry(&carried[CacheHeader::SIZE..end], "drifted output").unwrap();
        assert_eq!(entry.functions[0].id, drifted_id);
    }

    #[test]
    fn carries_emitter_omitted_accessor_but_keeps_authored_free_body() {
        let base_class = class_record(
            CLASS,
            "",
            "Value",
            &[MethodSpec {
                name: "__InitDefaults",
                traits: 0x40000,
                code: &[2, 42, 10],
            }],
            &[0],
            &[10],
        );
        let regen_class = class_record(CLASS, "", "Value", &[], &[], &[2, 8, 10]);
        let normal_base = function(&MethodSpec {
            name: "FreeFunction",
            traits: 0,
            code: &[10],
        });
        let normal_regen = function(&MethodSpec {
            name: "FreeFunction",
            traits: 0,
            code: &[2, 7, 10],
        });
        let get_base = function_with_return(
            &MethodSpec {
                name: "Get",
                traits: 0,
                code: &[2, 41, 10],
            },
            &object_handle_datatype(),
        );
        let get_regen = function_with_return(
            &MethodSpec {
                name: "Get",
                traits: 0,
                code: &[2, 99, 10],
            },
            &object_handle_datatype(),
        );
        let base = cache_with_functions(&[base_class], &[normal_base, get_base], 1);
        let regen = cache_with_functions(&[regen_class], &[normal_regen, get_regen], 2);
        let plan = prepare(&base).unwrap();
        assert_eq!(plan.generated_free_indices, HashSet::from([1]));
        let carried = plan.apply(&regen).unwrap();

        let parse = |bytes: &[u8], context| {
            let end = module_region_end(bytes).unwrap();
            parse_entry(&bytes[CacheHeader::SIZE..end], context).unwrap()
        };
        let base_entry = parse(&base, "accessor base");
        let regen_entry = parse(&regen, "accessor regen");
        let out_entry = parse(&carried, "accessor output");
        assert_eq!(
            function_raw(&carried, &out_entry.functions[0]),
            function_raw(&regen, &regen_entry.functions[0])
        );
        assert_eq!(
            function_raw(&carried, &out_entry.functions[1]),
            function_raw(&base, &base_entry.functions[1])
        );

        let changed_get = function_with_return(
            &MethodSpec {
                name: "Get",
                traits: 4,
                code: &[2, 99, 10],
            },
            &object_handle_datatype(),
        );
        let drift = cache_with_functions(
            &[class_record(CLASS, "", "Value", &[], &[], &[10])],
            &[
                function(&MethodSpec {
                    name: "FreeFunction",
                    traits: 0,
                    code: &[10],
                }),
                changed_get,
            ],
            3,
        );
        assert!(plan
            .apply(&drift)
            .unwrap_err()
            .contains("compiler wrapper Get traits drift"));
    }

    #[test]
    fn carries_behavior_functions_for_classes_with_and_without_defaults() {
        let base = cache(
            &[
                class_record_with_behaviors(
                    CLASS,
                    "",
                    "Value",
                    &[MethodSpec {
                        name: "__InitDefaults",
                        traits: 0x40000,
                        code: &[2, 42, 10],
                    }],
                    &[0],
                    &[10],
                    &[MethodSpec {
                        name: "$beh_default",
                        traits: 0x40000,
                        code: &[2, 11, 10],
                    }],
                ),
                class_record_with_behaviors(
                    "UBehaviorOnly",
                    "",
                    "OtherValue",
                    &[MethodSpec {
                        name: "SecondMethod",
                        traits: 0,
                        code: &[10],
                    }],
                    &[0],
                    &[10],
                    &[MethodSpec {
                        name: "$beh_only",
                        traits: 0x40000,
                        code: &[2, 12, 10],
                    }],
                ),
            ],
            &[10],
            1,
        );
        let regen = cache(
            &[
                class_record_with_behaviors(
                    CLASS,
                    "",
                    "Value",
                    &[],
                    &[],
                    &[2, 20, 10],
                    &[MethodSpec {
                        name: "$beh_default",
                        traits: 0x40000,
                        code: &[2, 91, 10],
                    }],
                ),
                class_record_with_behaviors(
                    "UBehaviorOnly",
                    "",
                    "OtherValue",
                    &[MethodSpec {
                        name: "SecondMethod",
                        traits: 0,
                        code: &[2, 21, 10],
                    }],
                    &[0],
                    &[2, 22, 10],
                    &[MethodSpec {
                        name: "$beh_only",
                        traits: 0x40000,
                        code: &[2, 92, 10],
                    }],
                ),
            ],
            &[2, 23, 10],
            2,
        );
        let plan = prepare(&base).unwrap();
        let carried = plan.apply(&regen).unwrap();
        let base_end = module_region_end(&base).unwrap();
        let out_end = module_region_end(&carried).unwrap();
        let base_entry = parse_entry(&base[CacheHeader::SIZE..base_end], "behavior base").unwrap();
        let out_entry =
            parse_entry(&carried[CacheHeader::SIZE..out_end], "behavior output").unwrap();
        for (base_class, out_class) in base_entry.classes.iter().zip(&out_entry.classes) {
            assert_eq!(
                &base[CacheHeader::SIZE + base_class.behaviors_block.start
                    ..CacheHeader::SIZE + base_class.behaviors_block.end],
                &carried[CacheHeader::SIZE + out_class.behaviors_block.start
                    ..CacheHeader::SIZE + out_class.behaviors_block.end]
            );
        }
    }

    #[test]
    fn rejects_layout_signature_generated_and_method_table_drift() {
        let base = base_cache();
        let plan = prepare(&base).unwrap();

        let changed_layout = cache(
            &[class_record(
                CLASS,
                "",
                "OtherValue",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 0,
                        code: &[10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[10],
                    },
                ],
                &[0, 1],
                &[10],
            )],
            &[10],
            0,
        );
        assert!(plan
            .apply(&changed_layout)
            .unwrap_err()
            .contains("flags/properties"));

        let changed_namespace = cache(
            &[class_record(
                CLASS,
                "ChangedNamespace",
                "Value",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 0,
                        code: &[10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[10],
                    },
                ],
                &[0, 1],
                &[10],
            )],
            &[10],
            0,
        );
        assert!(plan
            .apply(&changed_namespace)
            .unwrap_err()
            .contains("class identity/order drift"));

        let changed_traits = cache(
            &[class_record(
                CLASS,
                "",
                "Value",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 32,
                        code: &[10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[10],
                    },
                ],
                &[0, 1],
                &[10],
            )],
            &[10],
            0,
        );
        assert!(plan
            .apply(&changed_traits)
            .unwrap_err()
            .contains("declaration/signature drift"));

        let mut changed_ufunction = regen_cache();
        let changed_end = module_region_end(&changed_ufunction).unwrap();
        let changed_entry = parse_entry(
            &changed_ufunction[CacheHeader::SIZE..changed_end],
            "ufunction fixture",
        )
        .unwrap();
        let tail = changed_entry.classes[0].methods[0].ufunction_tail.clone();
        let mut replacement = 1i32.to_le_bytes().to_vec();
        replacement.extend_from_slice(&sia("Before"));
        replacement.extend_from_slice(&0i32.to_le_bytes()); // metadata specifiers
        replacement.extend_from_slice(&0i32.to_le_bytes()); // metadata values
        replacement.extend_from_slice(&[0u8; 18 * 4]); // UFUNCTION flags
        changed_ufunction.splice(
            CacheHeader::SIZE + tail.start..CacheHeader::SIZE + tail.end,
            replacement,
        );
        assert!(plan
            .apply(&changed_ufunction)
            .unwrap_err()
            .contains("UFUNCTION metadata drift"));

        let authored_defaults = base_cache();
        assert!(plan
            .apply(&authored_defaults)
            .unwrap_err()
            .contains("unexpectedly authored/generated __InitDefaults"));

        let invalid_table = cache(
            &[class_record(
                CLASS,
                "",
                "Value",
                &[
                    MethodSpec {
                        name: "Before",
                        traits: 0,
                        code: &[10],
                    },
                    MethodSpec {
                        name: "After",
                        traits: 4,
                        code: &[10],
                    },
                ],
                &[99],
                &[10],
            )],
            &[10],
            0,
        );
        assert!(plan
            .apply(&invalid_table)
            .unwrap_err()
            .contains("invalid local method 99"));
    }

    #[test]
    fn rejects_duplicate_identity_malformed_counts_and_unresolved_base_refs() {
        let duplicate = cache(
            &[
                class_record(
                    CLASS,
                    "",
                    "Value",
                    &[MethodSpec {
                        name: "__InitDefaults",
                        traits: 0,
                        code: &[10],
                    }],
                    &[0],
                    &[10],
                ),
                class_record(
                    CLASS,
                    "",
                    "Value",
                    &[MethodSpec {
                        name: "__InitDefaults",
                        traits: 0,
                        code: &[10],
                    }],
                    &[0],
                    &[10],
                ),
            ],
            &[10],
            0,
        );
        assert!(prepare(&duplicate)
            .unwrap_err()
            .contains("duplicate class identity"));

        let base = base_cache();
        let end = module_region_end(&base).unwrap();
        let entry = parse_entry(&base[CacheHeader::SIZE..end], "valid").unwrap();
        let mut malformed_entry = base[CacheHeader::SIZE..end].to_vec();
        malformed_entry[entry.classes[0].methods_count_pos..entry.classes[0].methods_count_pos + 4]
            .copy_from_slice(&((MAX_RECORDS as i32) + 1).to_le_bytes());
        assert!(parse_entry(&malformed_entry, "malformed")
            .unwrap_err()
            .contains("exceeds carry limit"));
        malformed_entry[entry.classes[0].methods_count_pos..entry.classes[0].methods_count_pos + 4]
            .copy_from_slice(&(MAX_RECORDS as i32).to_le_bytes());
        assert!(parse_entry(&malformed_entry, "malformed")
            .unwrap_err()
            .contains("requires at least"));

        // A default-initialized global has a 48-byte minimum (two empty SIAs, DataType, bool).
        // Ten such records plus the minimal module suffix used to trip a false 52-byte bound.
        let mut minimal_globals = fstring(MODULE);
        minimal_globals.extend_from_slice(&sia(MODULE));
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // functions
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // classes
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // enums
        minimal_globals.extend_from_slice(&10i32.to_le_bytes()); // globals
        for _ in 0..10 {
            minimal_globals.extend_from_slice(&sia(""));
            minimal_globals.extend_from_slice(&sia(""));
            minimal_globals.extend_from_slice(&datatype(0, 0x44));
            minimal_globals.extend_from_slice(&1i32.to_le_bytes()); // default init
        }
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // imports
        minimal_globals.extend_from_slice(&0i64.to_le_bytes()); // code hash
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // imported modules
        minimal_globals.extend_from_slice(&sia("")); // statics class
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // events
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // delegates
        minimal_globals.extend_from_slice(&sia("")); // filename
        minimal_globals.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
        assert_eq!(
            parse_entry(&minimal_globals, "minimal globals")
                .unwrap()
                .global_init_functions
                .len(),
            0
        );

        // CALLSYS 0x1234 with no FunctionReferences row: strict self-remap must reject it before
        // a carry plan can be returned.
        let unresolved = cache(
            &[class_record(
                CLASS,
                "",
                "Value",
                &[MethodSpec {
                    name: "__InitDefaults",
                    traits: 0,
                    code: &[61, 0x1234, 0, 10],
                }],
                &[0],
                &[10],
            )],
            &[10],
            0,
        );
        assert!(prepare(&unresolved)
            .unwrap_err()
            .contains("references are unresolved"));
    }

    #[test]
    fn rejects_invalid_base_envelope_and_cache_wide_output_id_collision() {
        let base = base_cache();
        let modules = model::parse_modules(&base).unwrap();

        let mut bad_magic = base.clone();
        bad_magic[16..20].copy_from_slice(&0u32.to_le_bytes());
        assert!(GeneratedDefaultsPlan::prepare(&bad_magic, &modules, MODULE)
            .unwrap_err()
            .contains("base header"));

        let mut trailing = base.clone();
        trailing.push(0x7f);
        assert!(GeneratedDefaultsPlan::prepare(&trailing, &modules, MODULE)
            .unwrap_err()
            .contains("ending at EOF"));

        let mut truncated = base.clone();
        truncated.truncate(truncated.len() - 4);
        assert!(GeneratedDefaultsPlan::prepare(&truncated, &modules, MODULE)
            .unwrap_err()
            .contains("base tail"));

        let regen = regen_cache();
        let regen_end = module_region_end(&regen).unwrap();
        let regen_entry =
            parse_entry(&regen[CacheHeader::SIZE..regen_end], "collision regen").unwrap();
        let collision_id = regen_entry.classes[0].constructors[0].id;
        let mut plan = prepare(&base).unwrap();
        plan.outside_function_ids
            .insert(collision_id, "OtherModule::OtherFunction".into());
        assert!(plan
            .apply(&regen)
            .unwrap_err()
            .contains("cache-wide function Id collision"));

        let base_end = module_region_end(&base).unwrap();
        let base_entry = parse_entry(&base[CacheHeader::SIZE..base_end], "first module").unwrap();
        let mut duplicate_ids = HashMap::new();
        record_function_ids(&base_entry, "first module", &mut duplicate_ids).unwrap();
        assert!(
            record_function_ids(&base_entry, "second module", &mut duplicate_ids)
                .unwrap_err()
                .contains("function Id collision")
        );
    }

    #[test]
    #[ignore = "requires GORE_AS_CACHE and GORE_AS_REGEN from the qualified hotfix"]
    fn real_biter_defaults_module_passes_strict_raw_carry_gates() {
        const BITER: &str = "AI.AIAgent.Creature.Biter.Dodges_Biter";
        let base_path = std::env::var_os("GORE_AS_CACHE").expect("set GORE_AS_CACHE");
        let regen_path = std::env::var_os("GORE_AS_REGEN").expect("set GORE_AS_REGEN");
        let base = std::fs::read(base_path).unwrap();
        let regen = std::fs::read(regen_path).unwrap();
        let modules = model::parse_modules(&base).unwrap();
        let plan = GeneratedDefaultsPlan::prepare(&base, &modules, BITER)
            .unwrap()
            .expect("Biter carries defaults");
        let extracted = super::super::splice::extract_module(&regen, BITER).unwrap();
        let (strict, _) = super::super::remap::remap_module_to_base(&extracted, &base).unwrap();
        let carried = plan.apply(&strict).unwrap();
        let parsed = model::parse_modules(&carried).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0]
                .classes
                .iter()
                .flat_map(|class| &class.methods)
                .filter(|method| method.name == "__InitDefaults")
                .count(),
            1
        );
        let replaced = super::super::splice::replace_module(&base, &carried, BITER).unwrap();
        assert_eq!(module_count(&replaced), module_count(&base));
        let replaced_end = module_region_end(&replaced).unwrap();
        let replaced_tables = parse_tail_tables(&replaced, replaced_end).unwrap();
        assert_eq!(replaced_tables.end, replaced.len());
        let replaced_modules = model::parse_modules(&replaced).unwrap();
        let replaced_biter = replaced_modules
            .iter()
            .find(|module| module.name == BITER)
            .unwrap();
        assert_eq!(
            replaced_biter
                .classes
                .iter()
                .flat_map(|class| &class.methods)
                .filter(|method| method.name == "__InitDefaults")
                .count(),
            1
        );
    }

    #[test]
    #[ignore = "requires GORE_AS_CACHE from the qualified hotfix"]
    fn real_known_ambiguous_base_references_fail_closed() {
        let base_path = std::env::var_os("GORE_AS_CACHE").expect("set GORE_AS_CACHE");
        let base = std::fs::read(base_path).unwrap();
        let modules = model::parse_modules(&base).unwrap();

        let item_scoring = "AI.AIItemScoring";
        let item_error = GeneratedDefaultsPlan::prepare(&base, &modules, item_scoring).unwrap_err();
        assert!(
            item_error.contains("references are unresolved")
                && item_error.contains("ambiguous global ref"),
            "{item_error}"
        );

        let voice_types = "Story.G1R.VoiceTypes";
        let voice_error = GeneratedDefaultsPlan::prepare(&base, &modules, voice_types).unwrap_err();
        assert!(
            voice_error.contains("references are unresolved")
                && voice_error.contains("ambiguous global ref"),
            "{voice_error}"
        );
    }
}
