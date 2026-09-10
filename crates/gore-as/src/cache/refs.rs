//! Reference resolver: map bytecode operands (engine type-ids, function-ids,
//! global/object pointers, member keys) to NAMES, using the 7 global tail tables.
//!
//! Resolution chains (per `work/reversing/gore-as/findings/decompile-refs.md`):
//! - type-id (DW)   -> TypeIdReferenceToPointer[id]      -> TypeReferences[ptr].Name
//! - func-id (DW)   -> FunctionIdReferenceToPointer[id]  -> FunctionReferences[ptr].Name
//! - type-ptr (QW)  -> TypeReferences[ptr].Name
//! - func-ptr (QW)  -> FunctionReferences[ptr].Name
//! - global-ptr(QW) -> GlobalReferences[ptr].Name
//! - member         -> PropertyReferences[(typeId<<1)|(offset<<33)|1].Name

use std::collections::{HashMap, HashSet};

use super::header::CacheHeader;
use super::types::DataType;
use super::walk_modules::module_region_end;
use super::wire::{Cursor, WireError};

/// Drop every space from a tokenized source snippet so it can be compared against a packed
/// render. `FGameplayTag :: Empty` and `FGameplayTag::Empty` are the same default; a string
/// literal is left alone, since spaces inside it are content.
/// Drop namespace qualifiers from every identifier in a rendered type
/// (`TSubclassOf<G1R::AIGroup::UAIGroup_StateEvent>` -> `TSubclassOf<UAIGroup_StateEvent>`).
/// A function's const-return verdict is per NAME AND per const qualifier: a class may declare
/// both `T f()` and `const T f() const`, and those two rows are not in disagreement.
/// `name/arity/const` — the shape `set_class_methods` stores.
fn is_const_key(key: &str, method: &str) -> bool {
    key.strip_prefix(method)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|rest| rest.ends_with("/const"))
}

pub(crate) fn const_return_key(name: &str, is_const_method: bool) -> String {
    format!("{name}/{}", if is_const_method { "const" } else { "" })
}

pub(crate) fn strip_namespaces(ty: &str) -> String {
    let mut out = String::with_capacity(ty.len());
    let mut token = String::new();
    for ch in ty.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            token.push(ch);
            continue;
        }
        out.push_str(token.rsplit("::").next().unwrap_or(&token));
        token.clear();
        out.push(ch);
    }
    out.push_str(token.rsplit("::").next().unwrap_or(&token));
    out
}

pub(crate) fn pack_tokens(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in value.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

/// Complete serialized identity of one `TypeReferences` entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeIdentity {
    pub name: String,
    pub module: String,
    pub namespace: String,
}

/// Resolved-name lookup built from a cache's tail tables.
#[derive(Debug, Default)]
pub struct RefResolver {
    /// GUID copied from the script cache this resolver was built from. Native field types may
    /// authorize cache mutation only when the loaded Binds.Cache is sealed for this exact GUID.
    script_cache_guid: Option<[u8; 16]>,
    type_by_ptr: HashMap<i64, String>,
    type_identity_by_ptr: HashMap<i64, TypeIdentity>,
    /// Bare type name -> its declaring namespace, only while every row with that name agrees.
    /// A name that appears in two namespaces cannot be qualified from the name alone, so it is
    /// removed rather than guessed.
    type_ns_by_name: HashMap<String, Option<String>>,
    /// Enum name -> its entries, in declaration order. The cache carries the enumerator NAMES for
    /// every script enum, and a constant written as its name is not the same expression as one
    /// built with a conversion: the compiler stores a named constant where the destination is,
    /// and builds a converted one before it goes looking.
    enum_entries: HashMap<String, Vec<(String, i32)>>,
    /// Class -> its methods that are NOT declared const (injected from the parsed modules).
    non_const_methods: HashMap<String, HashSet<String>>,
    /// Class -> the `name/arity` keys it declares ITSELF (not inherited). An override calling
    /// the method it overrides was written `Super::`, and rendering it as `this.` would recurse.
    class_methods: HashMap<String, HashSet<String>>,
    /// Return type by function NAME, for the names every declaration agrees on. A name two
    /// declarations disagree about carries no witness and is absent (see `names_returning`).
    func_ret_names: HashMap<String, String>,
    /// Method names the cache records a CONST overload for (see `names_a_const_method`).
    const_method_names: HashSet<String>,
    /// `(owner, function)` -> the declared default argument of each parameter, normalized.
    /// An empty entry means that parameter has none. The owner is `""` for a free function.
    param_defaults: HashMap<(String, String), Vec<String>>,
    func_by_ptr: HashMap<i64, String>,
    global_by_ptr: HashMap<i64, String>,
    prop_by_key: HashMap<i64, String>,
    /// PropertyReferences OldTypeId per member key (for field-assignment casts).
    prop_type_id: HashMap<i64, i32>,
    /// T7 keys seen more than once. Even byte-identical duplicate rows are ambiguous wire input
    /// for the semantic oracle and must never be treated as one proven declaration.
    duplicate_prop_keys: std::collections::HashSet<i64>,
    typeid_to_ptr: HashMap<i32, i64>,
    funcid_to_ptr: HashMap<i32, i64>,
    /// GlobalReferences with bIsString=true: the Name is the literal string text.
    global_is_string: std::collections::HashSet<i64>,
    /// FunctionReferences with bIsMethod=true (receiver split for calls).
    func_is_method: std::collections::HashSet<i64>,
    /// Template type SubTypes per type ptr (e.g. TSubclassOf -> [UObject]).
    type_subtypes: HashMap<i64, Vec<DataType>>,
    /// Set of all type names (to recognise constructor calls).
    type_names: std::collections::HashSet<String>,
    /// Namespace per global ref ptr (e.g. an `FColor::Red` constant's `FColor`).
    global_ns: HashMap<i64, String>,
    /// Script class -> its super class name (injected from the parsed modules after build).
    /// Lets call sites distinguish a legal upcast from an unrelated-object arg.
    class_super: HashMap<String, String>,
    /// Script class -> (field name -> COMPOSED field type name), injected from the parsed
    /// modules after build. Lets the emitter fold INHERITED fields into a class's field-type
    /// map (batch-21 Class B: `this.<inherited TMap>.opIndex(int)` needed the enum key wrap).
    class_fields: HashMap<String, HashMap<String, String>>,
    /// Exact own const-object-field types; ordinary composed field maps remain unchanged.
    const_object_fields: HashMap<(i64, String), DataType>,
    /// Exact own script int fields; no bare-class-name fallback.
    script_int_fields: HashSet<(i64, String)>,
    /// FunctionReferences parameter DataTypes (for arg-type-driven casts at call sites).
    func_params: HashMap<i64, Vec<DataType>>,
    /// Function names the cache records with a callable no-argument form — either a row with no
    /// parameters at all, or one whose every parameter carries a default. A rendered `X()` whose
    /// name is known but has no such form is a call that LOST its arguments.
    zero_arg_names: HashSet<String>,
    /// Every function name the cache records, so an unknown name can be told from a known one.
    known_func_names: HashSet<String>,
    /// `name/const` keys whose const return no caller can hold (see `unusable_const_returns`).
    unusable_const_return_names: HashSet<String>,
    /// Function pointers the cache records as CONST methods.
    const_method_ptrs: HashSet<i64>,
    /// Function names whose recorded rows DISAGREE about a const return. Re-emitting the
    /// qualifier for those breaks the language's "must have the same return type as in the base
    /// class" rule, so they keep the stripped form.
    inconsistent_const_return_names: HashSet<String>,
    /// Function name -> declared arity -> per-position "this parameter accepts a temporary" (by
    /// value, or by const reference). Rows are kept per arity because a name can carry unrelated
    /// overloads, and a call renders without the arguments that only restate a default, so the
    /// lookup consults every declared arity from the rendered one upwards.
    temporary_arg_positions: HashMap<String, HashMap<usize, Vec<bool>>>,
    /// The same, for CONSTRUCTORS, keyed by the type they build. A constructor is recorded under
    /// the behaviour name `$beh0`, not under the type's own name, so a call written as
    /// `FThing(a, b)` finds nothing in the map above — and "nothing" is refused, not unknown.
    ctor_arg_positions: HashMap<String, HashMap<usize, Vec<bool>>>,
    /// `name/type` keys for one-parameter functions whose parameter accepts a TEMPORARY — by
    /// value, or by const reference. A name is recorded only when EVERY one-parameter row of it
    /// taking that type accepts one, so a single non-const-reference overload disqualifies it.
    temporary_arg_methods: HashSet<String>,
    /// `Type<Sub>::name/arity` keys the cache's own function table records, namespaces stripped.
    /// Whether a value type has a default constructor, a copy constructor or an `opAssign` is
    /// what decides which SHAPE a local of it has to be written in.
    type_methods: HashSet<String>,
    /// FunctionReferences return DataType.
    func_ret: HashMap<i64, DataType>,
    /// FunctionReferences owning class name (from the ObjectType ptr) — disambiguates native
    /// method overloads when looking up arity in the Binds.Cache native API.
    func_owner: HashMap<i64, String>,
    /// Exact ObjectType pointer for named script constructors and destructors.
    script_ctor_owner: HashMap<i64, i64>,
    /// FunctionReferences namespace (e.g. `Gameplay`, `Math`, `System`) for free/static native
    /// functions — a call must be qualified `Namespace::func(...)` or the global-scope lookup
    /// fails with "No matching signatures". Empty for un-namespaced globals and for methods.
    func_ns: HashMap<i64, String>,
    /// Native AngelScript API arities parsed from Binds.Cache (fallback arity for native calls
    /// whose param count isn't carried in the script FunctionReferences).
    native: Option<super::binds::NativeApi>,
    /// StaticNames tail table (table 5): the `n"..."` FName-literal pool. The synthesized
    /// accessor `const FName& __STATIC_NAME(int Id)` (decl string in the shipping exe) returns
    /// `FAngelscriptManager::StaticNames[Id]`, which PrepareToFinalizePrecompiledModules
    /// populates from THIS table — so the bytecode's int operand indexes it directly.
    static_names: Vec<String>,
    /// Names that exist as a METHOD somewhere: T3 FunctionReferences flagged bIsMethod
    /// (native or script methods actually referenced by bytecode) plus script-class method
    /// declarations injected from the parsed modules. Batch-24b shadow gate: a free script
    /// global sharing such a name is SHADOWED by member lookup inside classes and must be
    /// `::`-qualified.
    method_names: std::collections::HashSet<String>,
    /// T3 FunctionReferences declaring-module name per NON-method function ptr (batch-25f):
    /// keys the cross-module free-fn rename map, matching the parsed `Module::name` exactly.
    func_module: HashMap<i64, String>,
    /// batch-25f: per-function-ptr rename for cross-module free-fn collisions — the emit-side
    /// collision scan renames each colliding declaration `Name -> Name_g<mi>` with a TEXT pass
    /// over the DECLARING module only; this id-keyed map lets CALL/CALLINTF sites in EVERY
    /// module resolve the renamed symbol.
    free_fn_renames: HashMap<i64, String>,
}

impl RefResolver {
    /// Parse a cache's 7 tail tables into name lookups.
    pub fn build(bytes: &[u8]) -> Result<Self, WireError> {
        // Bound all serialized row counts/bytes before the resolver materializes its lookup maps.
        // Bytediff and other public readers accept external caches without going through the
        // sequential composition guard, so they need the same allocation-light gate here.
        super::remap::preflight_tail_tables(bytes)?;
        let tail = module_region_end(bytes)?;
        let mut c = Cursor::at(bytes, tail);
        let mut r = RefResolver {
            script_cache_guid: CacheHeader::parse(bytes).ok().map(|header| header.hash),
            ..RefResolver::default()
        };

        // T1 TypeReferences: int64 key + (Name, Module, Namespace, TArray<DataType>)
        let type_reference_count = c.read_count("TypeReferences")?;
        c.ensure_minimum_remaining(type_reference_count, 24, "TypeReferences")?;
        for _ in 0..type_reference_count {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            let module = c.read_sia()?;
            let namespace = c.read_sia()?;
            let nsub = c.read_count("TypeRef.SubTypes")?;
            c.ensure_minimum_remaining(nsub, 36, "TypeRef.SubTypes")?;
            if nsub > 0 {
                let mut subs = Vec::new();
                for _ in 0..nsub {
                    subs.push(DataType::read(&mut c)?);
                }
                r.type_subtypes.insert(key, subs);
            }
            r.type_names.insert(name.clone());
            r.type_by_ptr.insert(key, name.clone());
            let declared = (!namespace.is_empty()).then(|| namespace.clone());
            match r.type_ns_by_name.get(&name) {
                Some(known) if *known != declared => {
                    r.type_ns_by_name.insert(name.clone(), None);
                }
                Some(_) => {}
                None => {
                    r.type_ns_by_name.insert(name.clone(), declared);
                }
            }
            r.type_identity_by_ptr.insert(
                key,
                TypeIdentity {
                    name,
                    module,
                    namespace,
                },
            );
        }
        // T2 TypeIdReferenceToPointer: int32 id -> int64 ptr
        let type_id_count = c.read_count("TypeIdRef")?;
        c.ensure_minimum_remaining(type_id_count, 12, "TypeIdRef")?;
        for _ in 0..type_id_count {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            r.typeid_to_ptr.insert(id, ptr);
        }
        // T3 FunctionReferences: int64 key + (Name, Module, Namespace, 3 bool, int64, params, ret)
        // The owning-type keys are composed after the parse: composing one needs the finished
        // type tables, which are still borrowed mutably here.
        let mut owned_methods: Vec<(i64, String, usize)> = Vec::new();
        let mut one_arg_params: Vec<(String, DataType, bool)> = Vec::new();
        let mut ctor_params: Vec<(i64, Vec<bool>)> = Vec::new();
        let mut const_returns: HashMap<String, bool> = HashMap::new();
        let function_reference_count = c.read_count("FunctionReferences")?;
        c.ensure_minimum_remaining(function_reference_count, 80, "FunctionReferences")?;
        for _ in 0..function_reference_count {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            let module = c.read_sia()?; // Module (declaring module name, batch-25f)
            let ns = c.read_sia()?; // Namespace
            let is_const_method = c.read_bool4()?; // bIsConst
            c.skip(4)?; // bIsImportedDecl
            let is_method = c.read_bool4()?;
            let objtype = c.read_i64()?; // ObjectType ptr (owning class)
            let nparams = c.read_count("FuncRef.Params")?;
            c.ensure_minimum_remaining(nparams, 36, "FuncRef.Params")?;
            let mut params = Vec::new();
            for _ in 0..nparams {
                params.push(DataType::read(&mut c)?);
            }
            let ret = DataType::read(&mut c)?; // ReturnType
            if is_method {
                r.func_is_method.insert(key);
                r.method_names.insert(name.clone());
            }
            // Always record params — even an empty list — so the call-site arg-count check
            // can stub a zero-param method that was decompiled with phantom args.
            r.known_func_names.insert(name.clone());
            if params.is_empty() {
                r.zero_arg_names.insert(name.clone());
            }
            if is_method && objtype != 0 {
                owned_methods.push((objtype, name.clone(), params.len()));
            }
            // Keyed WITH the method's own const qualifier: `T f()` and `const T f() const` are
            // an accessor pair, not a disagreement, and collapsing them stripped the qualifier
            // from both halves.
            let returns_const = ret.is_object_const || ret.is_read_only;
            let const_key = const_return_key(&name, is_const_method);
            match const_returns.get(&const_key) {
                Some(seen) if *seen != returns_const => {
                    r.inconsistent_const_return_names.insert(const_key);
                }
                None => {
                    const_returns.insert(const_key, returns_const);
                }
                _ => {}
            }
            {
                // Return type by name, for a caller that has been rendered to text and has the
                // name and nothing else. Two declarations that disagree leave the name blank
                // rather than guessing which one a call site meant.
                let returned = ret.base_name(&r);
                match r.func_ret_names.get(&name) {
                    Some(seen) if *seen != returned => {
                        r.func_ret_names.insert(name.clone(), String::new());
                    }
                    Some(_) => {}
                    None => {
                        r.func_ret_names.insert(name.clone(), returned);
                    }
                }
            }
            if is_const_method {
                r.const_method_ptrs.insert(key);
                // By NAME as well: a caller rendered as text has the name and nothing else, and
                // a const call's result may not be thrown away.
                r.const_method_names.insert(name.clone());
            }
            {
                let accepts: Vec<bool> = params
                    .iter()
                    .map(|p| !p.is_reference || p.is_object_const || p.is_read_only)
                    .collect();
                if name == "$beh0" && objtype != 0 {
                    ctor_params.push((objtype, accepts.clone()));
                }
                let by_arity = r.temporary_arg_positions.entry(name.clone()).or_default();
                match by_arity.get_mut(&accepts.len()) {
                    Some(seen) => {
                        for (slot, accepted) in seen.iter_mut().zip(&accepts) {
                            *slot &= *accepted;
                        }
                    }
                    None => {
                        by_arity.insert(accepts.len(), accepts);
                    }
                }
            }
            if let [only] = params.as_slice() {
                // `!is_reference` is a by-value parameter; a reference one has to be const.
                let accepts_temporary =
                    !only.is_reference || only.is_object_const || only.is_read_only;
                one_arg_params.push((name.clone(), only.clone(), accepts_temporary));
            }
            r.func_params.insert(key, params);
            r.func_ret.insert(key, ret);
            if let Some(cls) = r.type_by_ptr.get(&objtype) {
                r.func_owner.insert(key, cls.clone());
                if is_method && (cls == &name || name.strip_prefix('~') == Some(cls.as_str())) {
                    r.script_ctor_owner.insert(key, objtype);
                }
            }
            // Only a non-method (free/static) function needs namespace qualification; a method is
            // rendered via its receiver. Record the namespace so the call site can prefix it.
            if !is_method && !ns.is_empty() {
                r.func_ns.insert(key, ns);
            }
            // Declaring module of a free function (batch-25f rename-map key; methods are
            // rendered via their receiver and never rename).
            if !is_method && !module.is_empty() {
                r.func_module.insert(key, module);
            }
            r.func_by_ptr.insert(key, name);
        }
        // T4 FunctionIdReferenceToPointer: int32 id -> int64 ptr
        let function_id_count = c.read_count("FuncIdRef")?;
        c.ensure_minimum_remaining(function_id_count, 12, "FuncIdRef")?;
        for _ in 0..function_id_count {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            r.funcid_to_ptr.insert(id, ptr);
        }
        // T5 GlobalReferences: int64 key + (Name, Module, Namespace, int32 bIsString)
        let global_reference_count = c.read_count("GlobalReferences")?;
        c.ensure_minimum_remaining(global_reference_count, 24, "GlobalReferences")?;
        for _ in 0..global_reference_count {
            let key = c.read_i64()?;
            let name_pos = c.pos();
            let name = c.read_sia_bytes()?;
            c.read_sia()?; // Module
            let ns = c.read_sia()?; // Namespace
            let is_string = c.read_bool4()?;
            let name = if is_string {
                name.decode_utf8(name_pos)?
            } else {
                name.decode_ansi()
            };
            if is_string {
                r.global_is_string.insert(key);
            }
            if !ns.is_empty() {
                r.global_ns.insert(key, ns);
            }
            r.global_by_ptr.insert(key, name);
        }
        // T6 StaticNames: TArray<SIA> — the FName-literal pool `__STATIC_NAME(Id)` indexes.
        let n_static = c.read_count("StaticNames")?;
        c.ensure_minimum_remaining(n_static, 4, "StaticNames")?;
        for _ in 0..n_static {
            r.static_names.push(c.read_sia()?);
        }
        // T7 PropertyReferences: int64 key + (Name, int32 OldTypeId)
        let property_reference_count = c.read_count("PropertyReferences")?;
        c.ensure_minimum_remaining(property_reference_count, 16, "PropertyReferences")?;
        for _ in 0..property_reference_count {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            let old_type_id = c.read_i32()?; // OldTypeId
            if r.prop_by_key.contains_key(&key) {
                r.duplicate_prop_keys.insert(key);
            }
            r.prop_by_key.insert(key, name);
            r.prop_type_id.insert(key, old_type_id);
        }
        let mut one_arg_verdict: HashMap<String, bool> = HashMap::new();
        for (name, param, accepts_temporary) in one_arg_params {
            let key = format!("{name}/{}", strip_namespaces(&param.base_name(&r)));
            let entry = one_arg_verdict.entry(key).or_insert(true);
            *entry &= accepts_temporary;
        }
        r.temporary_arg_methods = one_arg_verdict
            .into_iter()
            .filter_map(|(key, accepts)| accepts.then_some(key))
            .collect();
        for (objtype, accepts) in ctor_params {
            let Some(owner) = r.composed_type_name(objtype) else {
                continue;
            };
            let by_arity = r
                .ctor_arg_positions
                .entry(strip_namespaces(&owner).to_string())
                .or_default();
            match by_arity.get_mut(&accepts.len()) {
                Some(seen) => {
                    for (slot, accepted) in seen.iter_mut().zip(&accepts) {
                        *slot &= *accepted;
                    }
                }
                None => {
                    by_arity.insert(accepts.len(), accepts);
                }
            }
        }
        for (objtype, name, arity) in owned_methods {
            let Some(owner) = r.composed_type_name(objtype) else {
                continue;
            };
            r.type_methods
                .insert(format!("{}::{name}/{arity}", strip_namespaces(&owner)));
        }
        Ok(r)
    }

    pub fn type_by_id(&self, id: i32) -> Option<&str> {
        self.typeid_to_ptr
            .get(&id)
            .and_then(|p| self.type_by_ptr.get(p))
            .map(|s| s.as_str())
    }
    /// Type name by id WITH template subtypes composed
    /// (`TArrayConstIterator<AGothicCharacter>`), mirroring `DataType::render` — `type_by_id`
    /// returns the bare head, which is a template-arity error when used as a declaration.
    /// Falls back to the bare name when the T1 entry records no subtypes.
    pub fn type_by_id_composed(&self, id: i32) -> Option<String> {
        let ptr = self.typeid_to_ptr.get(&id)?;
        let base = self.type_by_ptr.get(ptr)?;
        match self.type_subtypes.get(ptr) {
            Some(subs) if !subs.is_empty() => {
                let inner: Vec<String> = subs.iter().map(|s| s.base_name(self)).collect();
                Some(format!("{base}<{}>", inner.join(", ")))
            }
            _ => Some(base.clone()),
        }
    }
    pub fn func_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_by_ptr.get(p))
            .map(|s| s.as_str())
    }
    /// Owning class name of a function by ptr (the ObjectType the method belongs to). Used to
    /// qualify `Class::StaticClass()` with the TARGET class, not the calling class.
    pub fn func_owner_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_owner.get(&ptr).map(|s| s.as_str())
    }
    /// Owning class name of a function by id.
    pub fn func_owner_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_owner.get(p))
            .map(|s| s.as_str())
    }
    /// Exact owner identity of a named script constructor; never a bare-name lookup.
    pub(crate) fn script_constructor_type_by_id(&self, id: i32) -> Option<&TypeIdentity> {
        let ptr = self.funcid_to_ptr.get(&id)?;
        let owner = self.type_identity_by_ptr.get(self.script_ctor_owner.get(ptr)?)?;
        (self.func_by_ptr.get(ptr)? == &owner.name).then_some(owner)
    }
    /// Exact owner identity of a named script destructor, sharing the behavior-owner table.
    pub(crate) fn script_destructor_type_by_id(&self, id: i32) -> Option<&TypeIdentity> {
        let ptr = self.funcid_to_ptr.get(&id)?;
        let owner = self.type_identity_by_ptr.get(self.script_ctor_owner.get(ptr)?)?;
        (self.func_by_ptr.get(ptr)? == &format!("~{}", owner.name)).then_some(owner)
    }
    pub fn type_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.type_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Full type identity for COPY's serialized type-id operand.
    pub(crate) fn type_identity_by_id(&self, id: i32) -> Option<&TypeIdentity> {
        self.typeid_to_ptr.get(&id).and_then(|ptr| self.type_identity_by_ptr.get(ptr))
    }
    /// Full module/namespace/name identity for an exact serialized type pointer.
    pub fn type_identity_by_ptr(&self, ptr: i64) -> Option<&TypeIdentity> {
        self.type_identity_by_ptr.get(&ptr)
    }
    /// Declaring AngelScript namespace of a type, empty at global scope. A reference from
    /// another namespace has to spell it out (`G1R::UStoryG1R`), or the name does not resolve
    /// and everything built on it degrades to `Unknown`.
    /// Declaring namespace of a type looked up by BARE name, when unambiguous. Used where only
    /// the rendered name survived and the pointer is long gone.
    pub fn type_ns_by_name(&self, name: &str) -> Option<&str> {
        self.type_ns_by_name
            .get(name)
            .and_then(|namespace| namespace.as_deref())
    }
    pub fn type_ns_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.type_identity_by_ptr
            .get(&ptr)
            .map(|identity| identity.namespace.as_str())
            .filter(|namespace| !namespace.is_empty())
    }
    /// True if `name` is a known type (so a call to it is a constructor, not a method).
    pub fn is_type_name(&self, name: &str) -> bool {
        self.type_names.contains(name)
    }

    /// Inject the script-class hierarchy (class name -> super name) from parsed modules.
    pub fn set_class_hierarchy(&mut self, supers: HashMap<String, String>) {
        self.class_super = supers;
    }
    /// Sparse qualified field evidence, keyed by the exact serialized owner pointer.
    /// Ambiguous owner identities or repeated field declarations provide no witness.
    pub(crate) fn set_qualified_fields(
        &mut self,
        fields: impl IntoIterator<Item = (TypeIdentity, String, DataType)>,
    ) {
        let mut owners = HashMap::new();
        for (ptr, owner) in &self.type_identity_by_ptr {
            owners.entry((owner.module.as_str(), owner.namespace.as_str(), owner.name.as_str()))
                .and_modify(|known| *known = None).or_insert(Some(*ptr));
        }
        let mut qualified = HashMap::new();
        let mut integers = HashSet::new();
        let mut seen = HashSet::new();
        for (owner, field, ty) in fields {
            let Some(Some(ptr)) = owners.get(&(
                owner.module.as_str(), owner.namespace.as_str(), owner.name.as_str(),
            )) else { continue; };
            let key = (*ptr, field);
            if !seen.insert(key.clone()) {
                qualified.remove(&key);
                integers.remove(&key);
                continue;
            }
            if !owner.module.is_empty() && ty.token == 0x44 && ty.type_info == 0
                && !ty.is_reference && !ty.is_object_handle && !ty.is_auto
            {
                integers.insert(key.clone());
            }
            if ty.token == 5 && ty.is_object_const && ty.is_object_handle && !ty.is_read_only
                && self.type_by_ptr.contains_key(&ty.type_info)
            {
                qualified.insert(key, ty);
            }
        }
        self.const_object_fields = qualified;
        self.script_int_fields = integers;
    }

    pub(crate) fn is_script_int_field(&self, owner_id: i32, offset: i32) -> bool {
        let Some(owner) = self.typeid_to_ptr.get(&owner_id) else { return false; };
        let Some(name) = self.member(owner_id, offset) else { return false; };
        self.script_int_fields.contains(&(*owner, name.to_owned()))
    }

    pub(crate) fn const_object_field_accepts(&self, owner_id: i32, offset: i32, param: &DataType) -> bool {
        let Some(owner) = self.typeid_to_ptr.get(&owner_id) else { return false; };
        let Some(name) = self.member(owner_id, offset) else { return false; };
        self.const_object_fields.get(&(*owner, name.to_owned())).is_some_and(|field| {
            field.token == param.token && field.type_info == param.type_info
                && (field.is_reference, field.is_object_const, field.is_object_handle,
                    field.is_read_only, field.is_auto, field.if_handle_then_const)
                == (param.is_reference, param.is_object_const, param.is_object_handle,
                    param.is_read_only, param.is_auto, param.if_handle_then_const)
        })
    }

    /// Inject per-class field-type maps (class -> field -> composed type) from parsed modules.
    pub fn set_class_fields(&mut self, fields: HashMap<String, HashMap<String, String>>) {
        self.class_fields = fields;
    }
    /// Field-type map of a single script class (own fields only; walk supers via
    /// [`Self::class_super_of`] for the inherited view).
    pub fn class_field_types(&self, class: &str) -> Option<&HashMap<String, String>> {
        self.class_fields.get(class)
    }
    /// Field VALUE type declared directly on `class`, without walking its superclasses.
    /// Mutation callers use this when the bytecode owner itself is part of the semantic identity;
    /// inheriting a same-named base field would mislabel the declaring owner.
    pub fn own_field_type_by_class(&self, class: &str, field: &str) -> Option<&str> {
        self.class_fields
            .get(class)
            .and_then(|fields| fields.get(field))
            .map(String::as_str)
    }
    /// Direct super-class name of a script class (None for engine types / roots).
    pub fn class_super_of(&self, class: &str) -> Option<&str> {
        self.class_super
            .get(class)
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
    }
    /// Field VALUE type by containing class name + field name, resolved through the injected
    /// per-class field maps (walking script supers, cycle-bounded). Correct for FOREIGN script
    /// classes/structs — where `member_type` (PropertyReferences.OldTypeId) only names the
    /// OWNER type, not the field's own type.
    pub fn field_type_by_class(&self, class: &str, field: &str) -> Option<&str> {
        let mut cur = class;
        for _ in 0..64 {
            if let Some(t) = self.class_fields.get(cur).and_then(|m| m.get(field)) {
                return Some(t);
            }
            match self.class_super.get(cur) {
                Some(s) if !s.is_empty() => cur = s,
                _ => return None,
            }
        }
        None
    }
    /// True if `name` is a class DEFINED in a script module (vs an engine/native type).
    pub fn is_script_class(&self, name: &str) -> bool {
        self.class_super.contains_key(name)
    }
    /// True if `sub` is `sup` or transitively derives from it (within the script hierarchy,
    /// extended by the known-native links below).
    pub fn is_subclass(&self, sub: &str, sup: &str) -> bool {
        // batch-30b (C4, specs/batch29-errortail.md §4): known NATIVE ancestor links the
        // script hierarchy cannot see — the walk dead-ends at the first native super
        // (UAIGroup_Combat : UAIGroup_Combat_Base [native] : ... : UGothicAIGroup), so the
        // inheritance-aware member-candidate merge dropped the slot to UObject (8×
        // "'X' is not a member of 'UObject'" in CalculateScore_Implementation). Precedent:
        // KNOWN_NATIVE_ARITY. Evidence for the entry: the same function passes
        // `UAIGroup_Combat::StaticClass()` into a `TSubclassOf<UGothicAIGroup>` (vanilla-
        // compiled => UAIGroup_Combat derives UGothicAIGroup), and single inheritance places
        // UGothicAIGroup at or above the direct super UAIGroup_Combat_Base.
        // batch-31d (N7, spec batch31-nomatch-illegalop §1.7): AGothicNPCState derives
        // AGothicCharacterState — evidence: the vanilla-compiled corpus passes
        // `GetAllNPCStates()` elements (TArray<AGothicNPCState>) into script params typed
        // AGothicCharacterState (the OldCamp guard CALL 0x2436d), and the
        // GASCharacterStateMixins free fn `ExchangeDailyRoutineToClass(AGothicCharacterState
        // Character, ...)` wraps exactly `Cast<AGothicNPCState>(Character)` — both directions
        // of the single-inheritance proof.
        // batch-33a: AGothicCharacter derives ACharacter — evidence: vanilla-compiled
        // bytecode reads ACharacter fields (CapsuleComponent/Mesh, ADDSi on the native
        // tid) off Cast<AGothicCharacter> results corpus-wide (XardasSleeper Initialize,
        // CreatureTeleport DoTeleport*), and the 30a-C6d GA_FallingRagdoll axiom in
        // provably_derived encodes the same edge. ASpellProjectileVisual derives
        // AProjectileVisual — evidence: ASpellBallVisual_AS (: ASpellProjectileVisual)
        // method bodies access `this.m_CollisionComp` declared on native
        // AProjectileVisual (ADDSi tid 0x400199b, GA_Spell_BallLightning family),
        // vanilla-compiled => the chain passes through AProjectileVisual; single
        // inheritance makes the link row sound (intermediates stay transparent to the
        // walk — precedent: the UAIGroup_Combat_Base row).
        // batch-41d: UAbilityTask_AI derives UAbilityTaskGeneric — evidence: the vanilla-compiled
        // TryPerformActionNow body upcasts `local_36 (UAbilityTaskGeneric) = local_34
        // (UAITask_CombatMove : UGothicCharacterAITask : UAbilityTask_AI)` (a legal derived->base
        // handle copy), proving UAITask_CombatMove's chain passes through UAbilityTaskGeneric; the
        // script walk dead-ends at the native super UAbilityTask_AI, so this link lets the
        // reciprocal member store `this.ActiveActionTask (UAITask_CombatMove) = local_36` recover
        // the required `Cast<UAITask_CombatMove>`. Single inheritance keeps the row sound.
        const KNOWN_NATIVE_HIERARCHY: &[(&str, &str)] = &[
            ("UAIGroup_Combat_Base", "UGothicAIGroup"),
            ("AGothicNPCState", "AGothicCharacterState"),
            ("AGothicCharacter", "ACharacter"),
            ("ASpellProjectileVisual", "AProjectileVisual"),
            ("UAbilityTask_AI", "UAbilityTaskGeneric"),
        ];
        if sub == sup {
            return true;
        }
        let mut cur = sub;
        for _ in 0..64 {
            // bound the walk against cycles; on a script-map dead end, follow a known
            // native link before giving up.
            let next = self.class_super.get(cur).map(String::as_str).or_else(|| {
                KNOWN_NATIVE_HIERARCHY
                    .iter()
                    .find(|(c, _)| *c == cur)
                    .map(|(_, p)| *p)
            });
            match next {
                Some(s) if s == sup => return true,
                Some(s) => cur = s,
                None => return false,
            }
        }
        false
    }
    /// Template SubTypes for a type ptr (e.g. TSubclassOf -> [UObject]).
    pub fn type_subtypes(&self, ptr: i64) -> Option<&[DataType]> {
        self.type_subtypes.get(&ptr).map(|v| v.as_slice())
    }
    pub fn func_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Namespace (`Gameplay`, `Math`, ...) for a free/static native function by ptr, if any.
    pub fn func_ns_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_ns.get(&ptr).map(|s| s.as_str())
    }
    /// Namespace for a free/static native function by id, if any.
    pub fn func_ns_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_ns.get(p))
            .map(|s| s.as_str())
    }
    /// Target class of a `StaticClass` call: StaticClass is a namespaced free fn whose
    /// Namespace IS the (fully-qualified) target class — the LAST `::` segment is the class
    /// name (objtype is NULL for StaticClass, so func_owner can't carry it).
    pub fn staticclass_class_by_id(&self, id: i32) -> Option<&str> {
        self.func_ns_by_id(id)
            .map(|ns| ns.rsplit("::").next().unwrap_or(ns))
    }
    pub fn staticclass_class_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_ns_by_ptr(ptr)
            .map(|ns| ns.rsplit("::").next().unwrap_or(ns))
    }
    /// Parameter DataTypes for a function by ptr (excludes the receiver).
    pub fn func_params_by_ptr(&self, ptr: i64) -> Option<&[DataType]> {
        self.func_params.get(&ptr).map(|v| v.as_slice())
    }
    /// Parameter DataTypes for a function by id.
    pub fn func_params_by_id(&self, id: i32) -> Option<&[DataType]> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_params.get(p))
            .map(|v| v.as_slice())
    }
    /// Return DataType for a function by ptr.
    pub fn func_ret_by_ptr(&self, ptr: i64) -> Option<&DataType> {
        self.func_ret.get(&ptr)
    }
    /// Return DataType for a function by id.
    pub fn func_ret_by_id(&self, id: i32) -> Option<&DataType> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_ret.get(p))
    }

    /// Attach the Binds.Cache native API (for arity fallback on native method calls).
    pub fn set_native_api(&mut self, api: super::binds::NativeApi) {
        self.native = Some(api);
    }
    /// Best-known native arity for a call by function ptr. Prefer an exact `(owning class,
    /// name)` match. For an owner-bearing VALUE/template method with no exact match, a
    /// globally-unambiguous name may only LOWER/equal the cache parameter count (useful for
    /// source-default args such as `TArray::Last()`); it may never exceed it and steal a deeper
    /// enclosing operand. UObject/Actor methods deliberately do not use that class-agnostic
    /// fallback: generated/K2 wrappers are frequently absent from the exact Binds record and
    /// collide with unrelated methods (`AActor::GetComponent(2)` vs
    /// `FHitResult::GetComponent(0)`, and `GetComponentsByClass(2)` vs its one-arg return-value
    /// wrapper). Their cache FunctionReference is the only owner-specific signature evidence.
    /// `FPerceptionHandler::AddEvent(1)` versus the unrelated Binds-only
    /// `UTimelineComponent::AddEvent(2)` is the concrete over-count this gate prevents.
    /// Ownerless namespace calls reject a conflicting by-name arity; the exact cache
    /// declaration remains the fallback. Unnamespaced calls retain the historical lookup.
    pub fn native_arity_by_ptr(&self, ptr: i64, name: &str) -> Option<usize> {
        // batch-20 Class C: natives whose tail-table FunctionReferences param list UNDERCOUNTS
        // the live game API (proven by the in-game error candidates). Keyed (owner, name); the
        // live-compiler signature is authoritative, so this overrides even a Binds arity.
        // FGameplayEffectSpec::SetByCallerMagnitude(FGameplayTag DataTag, float32 Magnitude):
        // the cache lists only DataTag, so the float Magnitude was dropped (17 in-game errors).
        const KNOWN_NATIVE_ARITY: &[(&str, &str, usize)] =
            &[("FGameplayEffectSpec", "SetByCallerMagnitude", 2)];
        if let Some(cls) = self.func_owner.get(&ptr) {
            if let Some((_, _, a)) = KNOWN_NATIVE_ARITY
                .iter()
                .find(|(c, n, _)| c == cls && n == &name)
            {
                return Some(*a);
            }
        }
        let n = self.native.as_ref()?;
        match self.func_owner.get(&ptr) {
            Some(cls) => n.arity(cls, name).or_else(|| {
                let bytes = cls.as_bytes();
                let object_class = matches!(bytes.first(), Some(b'U') | Some(b'A'))
                    && bytes.get(1).is_some_and(u8::is_ascii_uppercase);
                if object_class {
                    return None;
                }
                let by_name = n.arity_by_name(name)?;
                let params = self.func_params.get(&ptr)?;
                // A native value comparison's explicit tolerance is not a default
                // argument proved by an unrelated one-argument Binds method.
                let plain = |t: &DataType, token| t.token == token && t.type_info == 0
                    && !t.is_reference && !t.is_object_handle && !t.is_object_const
                    && !t.is_read_only && !t.is_auto && !t.if_handle_then_const;
                if by_name == 1 && self.is_method_by_ptr(ptr) && self.is_const_method_by_ptr(ptr)
                    && self.func_ret.get(&ptr).is_some_and(|t| plain(t, 0x41)) {
                    if let [value, tolerance] = params.as_slice() {
                        if value.token == 5 && value.is_reference && value.is_object_const && value.is_read_only
                            && !value.is_object_handle && !value.is_auto && !value.if_handle_then_const
                            && matches!(tolerance.token, 0x50 | 0x51 | 0x5e) && plain(tolerance, tolerance.token)
                            && self.type_identity_by_ptr(value.type_info).is_some_and(|ty| ty.name == *cls
                                && ty.name.starts_with('F') && ty.module.is_empty() && ty.namespace.is_empty()) {
                            return None;
                        }
                    }
                }
                (by_name <= params.len()).then_some(by_name)
            }),
            None => {
                let arity = n.arity_by_name(name)?;
                // A global actor factory has a name/bool/level tail absent from
                // an unrelated task method's three-argument Binds declaration.
                // Its complete typed cache signature owns both frame and arity.
                let actor_factory = (|| {
                    if arity != 3 || self.is_method_by_ptr(ptr) || self.func_ns.contains_key(&ptr) { return None; }
                    let [class, location, rotation, actor_name, deferred, level] = self.func_params.get(&ptr)?.as_slice() else { return None; };
                    let plain = |t: &DataType| !t.is_reference && !t.is_object_const && !t.is_read_only
                        && !t.is_auto && !t.if_handle_then_const;
                    let native = |t: &DataType, name: &str| t.token == 5 && self.type_identity_by_ptr(t.type_info)
                        .is_some_and(|i| i.name == name && i.module.is_empty() && i.namespace.is_empty());
                    let ret = self.func_ret.get(&ptr)?;
                    if !native(ret, "AActor") || !ret.is_object_handle || !plain(ret)
                        || !native(level, "ULevel") || !level.is_object_handle || !plain(level)
                        || deferred.token != 0x41 || deferred.type_info != 0 || deferred.is_object_handle || !plain(deferred)
                        || [(class, "TSubclassOf"), (location, "FVector"), (rotation, "FRotator"), (actor_name, "FName")]
                            .iter().any(|(t, name)| !native(t, name) || !t.is_reference || !t.is_object_const
                                || !t.is_read_only || t.is_object_handle || t.is_auto || t.if_handle_then_const)
                    { return None; }
                    let [subtype] = self.type_subtypes(class.type_info)? else { return None; };
                    (subtype.token == 5 && subtype.type_info == ret.type_info && subtype.is_object_handle && plain(subtype)).then_some(())
                })();
                if actor_factory.is_some() { return None; }
                // Binds may omit this namespace entirely: its two-arg MagicScript::LogInfo
                // must not truncate the three-arg VLog::LogInfo frame and orphan its FName.
                if self.func_ns.contains_key(&ptr)
                    && self.func_params.get(&ptr).is_some_and(|p| p.len() != arity)
                {
                    return None;
                }
                Some(arity)
            }
        }
    }
    /// Best-known native arity for a call by function id.
    pub fn native_arity_by_id(&self, id: i32, name: &str) -> Option<usize> {
        match self.funcid_to_ptr.get(&id) {
            Some(&ptr) => self.native_arity_by_ptr(ptr, name),
            None => self.native.as_ref()?.arity_by_name(name),
        }
    }
    /// True if `name` exists ANYWHERE in the Binds.Cache native API (any class' member or any
    /// global; ambiguous-arity overloads count). Binds absent -> false, so callers degrade to
    /// the status quo. Batch-24b: over-approximates "some class in the (native) ancestry has a
    /// same-named member that would SHADOW a script global" — safe, because `::`-qualifying a
    /// non-shadowed global resolves identically.
    pub fn native_name_exists(&self, name: &str) -> bool {
        self.native.as_ref().is_some_and(|n| n.has_name(name))
    }
    /// Enum VALUE type of a NATIVE struct's field, for the WRTV1 1-byte-write guard
    /// (batch-25a, specs/batch23-cantconvert.md G2). The script cache cannot resolve these:
    /// PropertyReferences.OldTypeId is the OWNER struct (verified: FWidgetAlignment.
    /// VerticalAlignment -> "FWidgetAlignment"), and several of the enums are not even in the
    /// T1 type table. The PRODUCTION source is this in-crate table — the emit runs without
    /// Binds.Cache (no `GORE_AS_BINDS`, no sibling next to the input cache; the Binds-loaded
    /// arity trim is a proven regression, batch-24b report), so a Binds-side parse alone would
    /// never fire. Every entry is verified against the shipped Binds.Cache field decls
    /// (`binds.rs` test `validate_field_types_against_real_binds_cache`) and keyed by the
    /// exact (member-load type-id -> owner, member) pair observed at enum stores/argument pushes.
    /// The Binds field-type table (when loaded, dev runs) extends coverage as a fallback.
    pub fn native_field_type(&self, class: &str, field: &str) -> Option<&str> {
        // Keep authored-default rendering on the same sealed evidence that admitted mutation.
        // Unqualified generations still fall through to the read-only sources below.
        if let Some(verified) = self.verified_source_cache_native_default_field_type(class, field) {
            return Some(verified);
        }
        const KNOWN_NATIVE_FIELD_TYPES: &[(&str, &str, &str)] = &[
            (
                "FWidgetAlignment",
                "VerticalAlignment",
                "EVerticalAlignment",
            ),
            (
                "FWidgetAlignment",
                "HorizontalAlignment",
                "EHorizontalAlignment",
            ),
            ("FPerceivedAgent", "Relationship", "ERelationship"),
            ("FPerceivedAgent", "Hostility", "ERelationshipHostility"),
            (
                "FPerceivedAgent",
                "RelativeRank",
                "ERelationshipRelativeRank",
            ),
            (
                "FFXPerceptionSoundArea",
                "PerceptionLoudness",
                "EPerceptionNoiseLoudness",
            ),
            (
                "FALoadingScreenSettings",
                "Layout",
                "EAsyncLoadingScreenLayout",
            ),
            (
                "FALoadingScreenSettings",
                "PlaybackType",
                "EMoviePlaybackType",
            ),
            ("FTextAppearance", "Justification", "ETextJustify"),
            (
                "FInteractionAnimTransition",
                "TransitionKind",
                "EInteractionInputKind",
            ),
            ("FWeatherSaveGame", "CurrentWeather", "EWeather"),
            // Native crime-victim handle fields read through
            // `LoadRObjR; PshRPtr` into `TArray<Enum>::Add`. PropertyReferences exposes only
            // the containing F-struct, so these Binds-verified value types are the cache-free
            // witness used by the member-register argument channel.
            (
                "FCrimeVictimPersonHandle",
                "RelationshipTowardsPerson",
                "ERelationship",
            ),
            (
                "FCrimeVictimPersonHandle",
                "RelativeRankTowardsPerson",
                "ERelationshipRelativeRank",
            ),
            (
                "FCrimeVictimGuildHandle",
                "RelationshipTowardsGuild",
                "ERelationship",
            ),
            (
                "FCrimeVictimGuildHandle",
                "RelativeRankTowardsGuild",
                "ERelationshipRelativeRank",
            ),
            // batch-30b (C9 G2 rows, specs/batch29-errortail.md §9): the two Letterbox
            // enum fields rendered as bool stores (`= (local_80 != 0)`) — 5×
            // "Can't implicitly convert from 'bool' to 'EVerticalAlignment&'" in the
            // LoadingScreen SetupGeneralLoadingScreen family. Owner derived from the
            // ADDSi tid at the WRTV1 sites (0x4002a20 -> FLetterboxLayoutSettings,
            // offsets 0/1); the sibling FWidgetAlignment rows above already render
            // their EVerticalAlignment(...) casts.
            (
                "FLetterboxLayoutSettings",
                "VerticalLoadingWidgetPosition",
                "EVerticalAlignment",
            ),
            (
                "FLetterboxLayoutSettings",
                "VerticalTipWidgetPosition",
                "EVerticalAlignment",
            ),
            // batch-30c: core-math FLOAT fields — NOT in the Binds field tables (math types
            // are special-registered; probed None), so these rows are excluded from the
            // binds.rs mirror test. Evidence is the in-game diagnostic itself: reads of
            // these fields into int slots emit "Implicit conversion from float to integer
            // loses precision" (the compiler names the source float), UE5 core math is
            // double ('float' in Hazelight AS). Consumed by the member-load float-source
            // typing (structure.rs) for the RDR8 int(...) wraps; the enum-filtered nfty
            // consumers ignore non-enum rows by construction.
            ("FVector", "X", "float"),
            ("FVector", "Y", "float"),
            ("FVector", "Z", "float"),
            ("FRotator", "Pitch", "float"),
            ("FRotator", "Yaw", "float"),
            ("FRotator", "Roll", "float"),
        ];
        // batch-40b (specs/rgt-and-methods-triage.md PART 1): NATIVE-struct FLOAT-family fields
        // the script cache cannot type. The production emit runs WITHOUT Binds.Cache, so the
        // batch-40 float-const-store fix (float literal instead of raw int bit-pattern) was inert
        // for these NATIVE structs: `float_field_type` reaches `native_field_type`, which — absent
        // binds — returned None, leaving `top.ty = None` so the WRTV `float_lit` reinterpret never
        // fired (e.g. `FLightValues.SourceWidth = local_1101;` = the int bits of 250.0f, which the
        // AS compiler then iTOf-coerces to the garbage float 1.13e9). These rows are the CACHE-FREE
        // production source. The owner CLASS is still resolved cache-only (via the ADDSi type-id ->
        // `type_by_id`, always present); this table only supplies the (class, field) -> float TYPE
        // that used to require binds. Every entry is `float32` (verified). Enumerated by diffing the
        // with-binds vs no-binds emit (the ADDSi member-store idiom sites) and confirmed against the
        // shipped Binds.Cache field decls by `binds.rs::validate_float_field_types_against_real_binds_cache`.
        const KNOWN_NATIVE_FLOAT_FIELDS: &[(&str, &str, &str)] = &[
            (
                "FALoadingScreenSettings",
                "MinimumLoadingScreenDisplayTime",
                "float32",
            ),
            ("FAlphaBlendArgs", "BlendTime", "float32"),
            ("FCameraBehaviour", "m_ArmLength", "float32"),
            ("FCameraBehaviour", "m_LagSpeed", "float32"),
            ("FCameraBehaviour", "m_SpellPitchLimit", "float32"),
            ("FCameraBehaviour", "m_SpellYawLimit", "float32"),
            ("FDodgeData", "m_SuperArmorResistanceMultiplier", "float32"),
            ("FFreezeParams", "m_BlendOutDuration", "float32"),
            ("FFreezeParams", "m_CustomTimeDilation", "float32"),
            ("FFreezeParams", "m_FreezeDuration", "float32"),
            ("FGameplayCueParameters", "NormalizedMagnitude", "float32"),
            ("FGameplayCueParameters", "RawMagnitude", "float32"),
            (
                "FGameplayEffectContext_HitResponse",
                "BowStretch",
                "float32",
            ),
            (
                "FGameplayEffectContext_HitResponse",
                "MultiplierSuperArmor",
                "float32",
            ),
            (
                "FGothicFlyDiveSettings",
                "AdaptToCollisionSampleZDistance",
                "float32",
            ),
            (
                "FGothicFlyDiveSettings",
                "CharacterZDivergeOffset",
                "float32",
            ),
            (
                "FGothicFlyDiveSettings",
                "GroundedMoveBeforeGoalDistance",
                "float32",
            ),
            ("FGothicFlyDiveSettings", "UseFlyDiveMinDistance", "float32"),
            (
                "FGothicPathfollowSettings",
                "AgentRadiusMultiplier",
                "float32",
            ),
            (
                "FGothicPathfollowSettings",
                "CrowdAgentRadiusMultiplier",
                "float32",
            ),
            (
                "FGothicPathfollowSettings",
                "CrowdAgentSeparationWeight",
                "float32",
            ),
            (
                "FInteractionAnimTransition",
                "BlockOtherTransitionsForSeconds",
                "float32",
            ),
            ("FInteractionAnimTransition", "CooldownSeconds", "float32"),
            ("FInteractionAnimTransition", "Probability", "float32"),
            ("FInteractionAnimTransition", "Weight", "float32"),
            ("FLightSet", "BarnDoorAngle", "float32"),
            ("FLightSet", "BarnDoorLength", "float32"),
            ("FLightSet", "IndirectLightingIntensity", "float32"),
            ("FLightSet", "VolumetricScatteringIntensity", "float32"),
            ("FLightValues", "AttenuationRadius", "float32"),
            ("FLightValues", "SourceHeight", "float32"),
            ("FLightValues", "SourceWidth", "float32"),
            ("FMemorizedEvent", "Magnitude", "float32"),
            (
                "FPathfollowModifyAvoidVelocitySettings",
                "FastSpeedVelocityMultiplier",
                "float32",
            ),
            (
                "FPathfollowModifyAvoidVelocitySettings",
                "MediumRangeVelocityMultiplier",
                "float32",
            ),
            (
                "FPathfollowModifyAvoidVelocitySettings",
                "ShortRangeVelocityMultiplier",
                "float32",
            ),
            (
                "FPathfollowMoveFocusSettings",
                "FocalPointHeightMultiplier",
                "float32",
            ),
            ("FPerceptionHandler", "DelaySeconds", "float32"),
            ("FRelativeCrimeDataEntry", "BaseSeverity", "float32"),
            ("FRememberedPerception", "Magnitude", "float32"),
            ("FRememberedPerception", "TimeUpdated", "float32"),
            ("FScalableFloat", "Value", "float32"),
            ("FScoredItemAction", "Score", "float32"),
            ("FSlateFontInfo", "Size", "float32"),
            ("FTipSettings", "TipSwapTime", "float32"),
            ("FTipSettings", "TipWrapAt", "float32"),
        ];
        if let Some((_, _, t)) = KNOWN_NATIVE_FIELD_TYPES
            .iter()
            .find(|(c, f, _)| *c == class && *f == field)
        {
            return Some(t);
        }
        // batch-40b: NATIVE-struct FLOAT-family field types the script cache cannot resolve.
        // Kept in a SEPARATE table from the enum rows above because these are consumed only by
        // the float-family-gated `float_field_type` (WRTV float-const store + RDR8 read wraps),
        // never the enum/int cast gates. Every row is verified against the shipped Binds.Cache
        // by `binds.rs::validate_float_field_types_against_real_binds_cache`.
        if let Some((_, _, t)) = KNOWN_NATIVE_FLOAT_FIELDS
            .iter()
            .find(|(c, f, _)| *c == class && *f == field)
        {
            return Some(t);
        }
        self.native
            .as_ref()
            .and_then(|n| n.field_type(class, field))
    }
    /// Declared VALUE type of a field of a NATIVE class or struct, from the loaded
    /// `Binds.Cache` plain-field scan.
    ///
    /// This is the channel that answers for members the script cache structurally cannot type:
    /// `PropertyReferences` stores only (name, OWNER OldTypeId), and a field declared on a
    /// NATIVE base (`UItemDefinition::m_Value`, `UWeaponDefinition::m_SuperArmorDamageBase`)
    /// appears in no script class-fields map, so both script-side channels resolve to `None`
    /// and the owner name is all `member_type` can offer. Without this the decompiler cannot
    /// tell a `WRTV4` of `0x0000000a` (int `10`) from one of `0x41200000` (float `10.0f`) and
    /// drops the store instead of guessing — which silently lost every scalar class default on
    /// a native base.
    ///
    /// Read-only evidence, deliberately ungated: it reports what the installed `Binds.Cache`
    /// declares for ANY build. Cache MUTATION keeps requiring the sealed, audited witness in
    /// [`Self::verified_native_default_field_type`]; this accessor must never be substituted
    /// there. Absent `Binds.Cache`, this returns `None` and callers keep their prior behaviour.
    pub fn native_field_value_type(&self, class: &str, field: &str) -> Option<&str> {
        self.verified_source_cache_native_default_field_type(class, field)
            .or_else(|| {
                self.native
                    .as_ref()
                    .and_then(|native| native.plain_field_type(class, field))
            })
    }

    /// Native field type admissible as a cache-mutation witness. Unlike the decompiler's
    /// best-effort [`Self::native_field_type`], this succeeds only for a SHA-256-sealed,
    /// independently audited Binds.Cache profile paired with its audited script-cache GUID.
    /// Callers must pass the GUID parsed from the same cache being inspected; an unknown GUID
    /// returns no witness without affecting [`Self::native_field_type`].
    pub fn verified_native_default_field_type(
        &self,
        script_cache_guid: &[u8; 16],
        class: &str,
        field: &str,
    ) -> Option<&str> {
        self.native
            .as_ref()
            .and_then(|native| native.verified_default_field_type(script_cache_guid, class, field))
    }

    /// Native field type admissible for authoring defaults from the cache that built this
    /// resolver. A synthetic resolver, invalid header, or foreign Binds.Cache has no witness.
    pub(crate) fn verified_source_cache_native_default_field_type(
        &self,
        class: &str,
        field: &str,
    ) -> Option<&str> {
        self.script_cache_guid
            .as_ref()
            .and_then(|guid| self.verified_native_default_field_type(guid, class, field))
    }
    /// batch-32d: CONST object-handle fields of NATIVE structs — the live compiler treats a
    /// read of these as `const U*`, so a plain store into a same-typed local fails "Can't
    /// implicitly convert from 'const UItemDefinition' to 'UItemDefinition'" (and a same-type
    /// `Cast<>` provably does NOT strip const in-game, batch-21 Class C). The script cache
    /// carries no constness for foreign fields (PropertyReferences = Name + OWNER OldTypeId),
    /// so this in-crate row is the production source, keyed by the ADDSi (owner, member) pair
    /// observed at the failing RefCpyV site (CharacterAI_Gothic
    /// SelectBestItemActionFromInventory :3002). Returns the field's BASE value type; the
    /// consumer (RefCpyV arm) compares it against the destination's declared type and emits
    /// the CONSTSTORE marker so the decl gains the `const` qualifier. Deliberately separate
    /// from KNOWN_NATIVE_FIELD_TYPES: those rows mirror the Binds field DECLS (validated by
    /// the binds.rs test) and feed enum/float-filtered consumers.
    pub fn native_field_const_object(&self, class: &str, field: &str) -> Option<&'static str> {
        // batch-41d: FGameplayEventData's object-handle fields are declared const in the engine
        // (`TObjectPtr<const AActor> Instigator/Target`, `TWeakObjectPtr<const UObject>
        // OptionalObject`). A member read of a `const FGameplayEventData &inout` PARAM yields a
        // const handle, so batch-41a's `local_N = EventData.Target;` recovery into a same-typed
        // non-const local failed "Can't implicitly convert from 'const AActor' to 'AActor'" in
        // generate-mode (18 sites: GA_Defeated/MCQueen/Xardas/Summon* etc.). Same production
        // source + exact-type-match consumer discipline as the FItemActionHandler row; the read
        // slots are only ever used const-safely (null-check / method receiver / Cast<Derived>
        // source), so const-declaring them is regression-free.
        const KNOWN_NATIVE_CONST_OBJECT_FIELDS: &[(&str, &str, &str)] = &[
            ("FItemActionHandler", "ItemDefinition", "UItemDefinition"),
            ("FGameplayEventData", "Instigator", "AActor"),
            ("FGameplayEventData", "Target", "AActor"),
            ("FGameplayEventData", "OptionalObject", "UObject"),
        ];
        KNOWN_NATIVE_CONST_OBJECT_FIELDS
            .iter()
            .find(|(c, f, _)| *c == class && *f == field)
            .map(|(_, _, t)| *t)
    }
    /// Inject script-class METHOD names from the parsed modules (a shadowing member need not be
    /// referenced by any bytecode — e.g. `UCM_CastSpell_Base::CastSpell()` shadows the free
    /// `CastSpell(AI, int)` even if the method itself is never called).
    pub fn add_method_names<I: IntoIterator<Item = String>>(&mut self, names: I) {
        self.method_names.extend(names);
    }
    /// Install `class -> methods that are NOT declared const` from the parsed modules. A const
    /// method may not call one of these on `this`, so the emitter needs the set to decide
    /// whether re-emitting a `const` qualifier keeps the body compiling.
    /// The enum tables, keyed by bare name. A name two modules disagree about is dropped rather
    /// than guessed at, the same way the namespace table treats an ambiguous name.
    pub fn set_enum_entries(&mut self, by_name: HashMap<String, Vec<(String, i32)>>) {
        self.enum_entries = by_name;
    }

    /// The enumerator an enum gives a value, where the enum is known and exactly one entry has it.
    pub fn enumerator_name(&self, ty: &str, value: i32) -> Option<&str> {
        let entries = self.enum_entries.get(ty)?;
        let mut hit = entries.iter().filter(|(_, entry)| *entry == value);
        let (name, _) = hit.next()?;
        hit.next().is_none().then_some(name.as_str())
    }

    pub fn set_non_const_methods(&mut self, by_class: HashMap<String, HashSet<String>>) {
        self.non_const_methods = by_class;
    }
    /// Install `class -> its OWN declared `name/arity` keys` from the parsed modules.
    pub fn set_class_methods(&mut self, by_class: HashMap<String, HashSet<String>>) {
        self.const_method_names.extend(
            by_class
                .values()
                .flatten()
                .filter_map(|key| key.strip_suffix("/const"))
                .filter_map(|key| key.rsplit_once('/').map(|(name, _)| name.to_owned())),
        );
        self.class_methods = by_class;
    }

    /// The return type every declaration of `name` agrees on, if they do agree.
    pub fn names_returning(&self, name: &str) -> Option<&str> {
        self.func_ret_names
            .get(name)
            .map(String::as_str)
            .filter(|ty| !ty.is_empty())
    }

    /// True when the cache records a CONST method by this name. A const call has no side effect
    /// to keep, so its result may not be thrown away ("Result of expression is unused", which
    /// this compiler treats as an error).
    pub fn names_a_const_method(&self, method: &str) -> bool {
        self.const_method_names.contains(method)
    }
    /// True when `class` or an ancestor declares a CONST overload of `method`. A const method
    /// calling it resolves to that overload, so the call does not make the caller non-const.
    pub fn has_const_overload(&self, class: &str, method: &str) -> bool {
        let mut current = Some(class.to_owned());
        for _ in 0..64 {
            let Some(name) = current else {
                break;
            };
            if self
                .class_methods
                .get(&name)
                .is_some_and(|methods| methods.iter().any(|key| is_const_key(key, method)))
            {
                return true;
            }
            current = self.class_super_of(&name).map(str::to_owned);
        }
        false
    }

    /// True when `class` declares `method` with that many parameters itself — an OVERRIDE. A
    /// same-named method with a different parameter count is an overload, and a call to the
    /// ancestor's version resolves to the ancestor either way.
    pub fn class_overrides_method(&self, class: &str, method: &str, arity: usize) -> bool {
        self.class_methods
            .get(class)
            .is_some_and(|methods| methods.contains(&format!("{method}/{arity}")))
    }
    /// Install `(owner, function) -> per-parameter default argument text` from the parsed
    /// modules. Whitespace is normalized on the way in: the cache stores the defaults
    /// TOKENIZED (`FGameplayTagContainer ( )`), the emitter renders them packed.
    pub fn set_param_defaults(&mut self, defaults: HashMap<(String, String), Vec<String>>) {
        self.param_defaults = defaults
            .into_iter()
            .map(|(key, values)| {
                let packed: Vec<String> = values.iter().map(|value| pack_tokens(value)).collect();
                // A function whose every parameter has a default can be written with no
                // arguments at all, so it is a legitimate `X()` at a call site.
                if !packed.is_empty() && packed.iter().all(|value| !value.is_empty()) {
                    self.zero_arg_names.insert(key.1.clone());
                }
                (key, packed)
            })
            .collect();
    }

    /// The owning type's name with its template subtypes (`TSubclassOf<UItemDefinition>`).
    fn composed_type_name(&self, ptr: i64) -> Option<String> {
        let base = self.type_by_ptr.get(&ptr)?.clone();
        match self.type_subtypes(ptr) {
            Some(subs) if !subs.is_empty() => {
                let inner: Vec<String> = subs.iter().map(|s| s.base_name(self)).collect();
                Some(format!("{base}<{}>", inner.join(", ")))
            }
            _ => Some(base),
        }
    }

    /// Install the names whose const return some caller cannot hold.
    pub fn set_unusable_const_returns(&mut self, names: HashSet<String>) {
        self.unusable_const_return_names = names;
    }

    /// True when the cache's rows for `name` disagree about returning a const value. An override
    /// family has to declare ONE return type, so a re-emitted qualifier would not compile.
    pub fn const_return_is_inconsistent(&self, name: &str, is_const_method: bool) -> bool {
        let key = const_return_key(name, is_const_method);
        self.inconsistent_const_return_names.contains(&key)
            || self.unusable_const_return_names.contains(&key)
    }

    /// True when the cache records this function pointer as a CONST method.
    pub fn is_const_method_by_ptr(&self, ptr: i64) -> bool {
        self.const_method_ptrs.contains(&ptr)
    }

    /// True when the cache records this function id as a CONST method.
    pub fn is_const_method_by_id(&self, id: i32) -> bool {
        self.funcid_to_ptr
            .get(&id)
            .is_some_and(|ptr| self.const_method_ptrs.contains(ptr))
    }

    /// True when every declaration of `name` a call with `rendered` arguments could reach takes
    /// parameter `position` by value or by const reference — the positions where a temporary
    /// expression is legal. Declarations with fewer parameters than the call renders cannot be
    /// the callee; ones with more are reachable through default arguments.
    pub fn arg_position_accepts_temporary(
        &self,
        name: &str,
        rendered: usize,
        position: usize,
    ) -> bool {
        let Some(by_arity) = self
            .temporary_arg_positions
            .get(name)
            .or_else(|| self.ctor_arg_positions.get(name))
        else {
            return false;
        };
        let mut reachable = by_arity
            .iter()
            .filter(|(arity, _)| **arity >= rendered)
            .peekable();
        reachable.peek().is_some()
            && reachable.all(|(_, accepts)| accepts.get(position).copied().unwrap_or(false))
    }

    /// True when every declaration of `name` a call with `rendered` arguments could reach takes
    /// parameter `position` by NON-CONST reference, so the callee writes through it. This is not
    /// the negation of [`Self::arg_position_accepts_temporary`]: a name the cache does not know
    /// proves nothing either way, and both answers stay `false` for it.
    pub fn arg_position_is_written_through(
        &self,
        name: &str,
        rendered: usize,
        position: usize,
    ) -> bool {
        let Some(by_arity) = self.temporary_arg_positions.get(name) else {
            return false;
        };
        let mut reachable = by_arity
            .iter()
            .filter(|(arity, _)| **arity >= rendered)
            .peekable();
        reachable.peek().is_some()
            && reachable.all(|(_, accepts)| accepts.get(position).copied() == Some(false))
    }

    /// True when every one-parameter `name` in the cache that takes `ty` takes it by value or by
    /// const reference — so a TEMPORARY may be written at that call site.
    pub fn one_arg_call_accepts_temporary(&self, name: &str, ty: &str) -> bool {
        self.temporary_arg_methods
            .contains(&format!("{name}/{}", strip_namespaces(ty)))
    }

    /// True when the cache's own function table records `type::name` with that many parameters.
    /// Namespaces are ignored on both sides — a rendered type carries them, the table does not.
    pub fn type_has_method(&self, ty: &str, name: &str, arity: usize) -> bool {
        self.type_methods
            .contains(&format!("{}::{name}/{arity}", strip_namespaces(ty)))
    }

    /// True when a rendered `name()` can be a real call: the cache does not know the name at all
    /// (nothing to check it against), or it knows a no-argument form of it.
    pub fn zero_arg_call_is_plausible(&self, name: &str) -> bool {
        !self.known_func_names.contains(name) || self.zero_arg_names.contains(name)
    }
    /// Declared default arguments of `owner::function`, if the cache recorded any. Walks the
    /// script hierarchy, because a call's target type is often a subclass of the declarer.
    pub fn param_defaults(&self, owner: &str, function: &str) -> Option<&[String]> {
        let mut current = Some(owner.to_string());
        for _ in 0..64 {
            let name = current?;
            if let Some(defaults) = self
                .param_defaults
                .get(&(name.clone(), function.to_string()))
            {
                return Some(defaults.as_slice());
            }
            current = self.class_super_of(&name).map(|s| s.to_string());
        }
        None
    }
    /// True when `method` is a known NON-const method of `class` or of any script ancestor.
    pub fn calls_non_const_method(&self, class: &str, method: &str) -> bool {
        let mut current = Some(class.to_string());
        for _ in 0..64 {
            let Some(name) = current else { break };
            if self
                .non_const_methods
                .get(&name)
                .is_some_and(|methods| methods.contains(method))
            {
                return true;
            }
            current = self.class_super_of(&name).map(|s| s.to_string());
        }
        false
    }
    /// batch-25f: install the cross-module free-fn rename map. `by_module` is
    /// `module name -> (original fn name -> renamed name)` — exactly the collision set the
    /// emit-side scan feeds the per-module `rename_free_fn` TEXT pass, so declarations and
    /// call sites can never disagree. Keyed here per FUNCTION PTR (id-based), never by bare
    /// name — the mixed-overload hazard the text pass documents is inherited from its gate
    /// (a module's name only enters the collision set when EVERY emittable same-name overload
    /// collides).
    pub fn set_free_fn_renames(&mut self, by_module: &HashMap<String, HashMap<String, String>>) {
        let mut m: HashMap<i64, String> = HashMap::new();
        if !by_module.is_empty() {
            for (ptr, name) in &self.func_by_ptr {
                if self.func_is_method.contains(ptr) {
                    continue;
                }
                let renamed = self
                    .func_module
                    .get(ptr)
                    .and_then(|module| by_module.get(module))
                    .and_then(|names| names.get(name));
                if let Some(new) = renamed {
                    m.insert(*ptr, new.clone());
                }
            }
        }
        self.free_fn_renames = m;
    }
    /// Renamed leaf for a free function by CALL id, when its declaration was collision-renamed
    /// (batch-25f). None = not renamed (the overwhelmingly common case).
    pub fn renamed_free_fn_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.free_fn_renames.get(p))
            .map(|s| s.as_str())
    }
    /// True if `name` exists as a member in cached/script/native evidence.
    ///
    /// The bytecode emitter now globally qualifies every structurally proven free script global
    /// inside a class, because a production cache without Binds cannot make this inventory
    /// complete. Keep the query for collision planning, diagnostics and evidence tests.
    pub fn member_name_exists(&self, name: &str) -> bool {
        const UNIVERSAL_UOBJECT_MEMBERS: [&str; 5] =
            ["GetName", "GetClass", "GetOuter", "GetWorld", "GetFName"];
        self.method_names.contains(name)
            || UNIVERSAL_UOBJECT_MEMBERS.contains(&name)
            || self.native_name_exists(name)
    }
    pub fn global_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.global_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Namespace for a global ref ptr (empty/absent -> None).
    pub fn global_ns(&self, ptr: i64) -> Option<&str> {
        self.global_ns.get(&ptr).map(|s| s.as_str())
    }
    /// True if the global at `ptr` is actually a string literal (Name = the text).
    /// Every string literal the cache's tables carry.
    ///
    /// A strict base-keyspace remap can only resolve strings that are already here, so an edit
    /// that introduces a brand-new literal cannot be carried back onto this cache. Callers use
    /// this to say so before a compile rather than after one.
    pub(super) fn string_globals(&self) -> impl Iterator<Item = &str> {
        self.global_is_string
            .iter()
            .filter_map(|pointer| self.global_by_ptr.get(pointer).map(String::as_str))
    }

    pub fn global_is_string(&self, ptr: i64) -> bool {
        self.global_is_string.contains(&ptr)
    }
    /// True if the function (by ptr) is a method (receiver split for calls).
    pub fn is_method_by_ptr(&self, ptr: i64) -> bool {
        self.func_is_method.contains(&ptr)
    }
    /// True if the function (by id) is a method.
    pub fn is_method_by_id(&self, id: i32) -> bool {
        self.funcid_to_ptr
            .get(&id)
            .map(|p| self.func_is_method.contains(p))
            .unwrap_or(false)
    }
    /// FName-literal text for a `__STATIC_NAME(Id)` index into the StaticNames tail table.
    /// None if out of range (e.g. a mini-cache with empty tail tables).
    pub fn static_name(&self, id: i64) -> Option<&str> {
        usize::try_from(id)
            .ok()
            .and_then(|i| self.static_names.get(i))
            .map(|s| s.as_str())
    }
    /// Every FName-literal text carried by the StaticNames tail table.
    ///
    /// This is deliberately separate from [`Self::string_globals`]: `n"..."` and ordinary
    /// string literals occupy different remap domains even when their text is identical.
    pub(super) fn static_names(&self) -> impl Iterator<Item = &str> {
        self.static_names.iter().map(String::as_str)
    }
    /// Number of StaticNames entries (debug aid).
    pub fn static_name_count(&self) -> usize {
        self.static_names.len()
    }

    /// Conservative bare names carried only by the Shipping cache tail tables.
    ///
    /// This intentionally collapses declaration scopes for the offline Quest collision inventory.
    /// String-literal globals are excluded because their `Name` is payload text, not a symbol.
    pub(super) fn collision_names(&self) -> impl Iterator<Item = &str> {
        self.type_names
            .iter()
            .map(String::as_str)
            .chain(self.func_by_ptr.values().map(String::as_str))
            .chain(
                self.global_by_ptr
                    .iter()
                    .filter(|(pointer, _)| !self.global_is_string.contains(pointer))
                    .map(|(_, name)| name.as_str()),
            )
            .chain(self.prop_by_key.values().map(String::as_str))
    }

    #[cfg(test)]
    pub(crate) fn from_test_default_return_conditional(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(100, "FExecutor"), (101, "UAbility"), (102, "AState"), (103, "ACharacter"), (104, "FVector"), (105, "UTask")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let value = DataType { token: 5, type_info: 100, ..Default::default() };
        let handle = |ptr| DataType { token: 5, type_info: ptr, is_object_handle: true, ..Default::default() };
        for (ptr, name, owner) in [(1, "$beh0", "FExecutor"), (2, "Actor", "AState"), (3, "Radius", "AActor"),
            (4, "opAssign", "FExecutor"), (5, "$beh2", "FExecutor")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            r.func_params.insert(ptr, Vec::new()); r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        r.const_method_ptrs.extend([2, 3]); r.func_ret.insert(2, handle(103));
        r.func_ret.insert(3, DataType { token: 0x50, ..Default::default() });
        let reference = DataType { is_reference: true, ..value.clone() };
        r.func_params.insert(4, vec![reference.clone()]); r.func_ret.insert(4, reference);
        r.funcid_to_ptr.insert(6, 6); r.func_by_ptr.insert(6, "Dispatch".into()); r.func_ret.insert(6, value);
        r.func_ns.insert(6, String::new());
        r.func_params.insert(6, vec![handle(101), handle(103), DataType { token: 0x51,
            is_object_const: true, is_read_only: true, ..Default::default() }]);
        match fault {
            1 => r.func_params.get_mut(&6).unwrap()[2].token = 0x50,
            2 => r.func_ret.get_mut(&3).unwrap().token = 0x51,
            3 => r.func_ret.get_mut(&6).unwrap().is_reference = true,
            4 => r.func_params.get_mut(&4).unwrap()[0].type_info = 104,
            5 => { r.func_params.get_mut(&1).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
            6 => r.func_ret.get_mut(&5).unwrap().token = 0x41,
            7 => r.func_owner.insert(2, "OtherState".into()).map(|_| ()).unwrap(),
            8 => r.type_identity_by_ptr.get_mut(&100).unwrap().namespace = "Other".into(),
            9 => { r.func_is_method.insert(6); }
            10 => r.func_ret.get_mut(&2).unwrap().is_read_only = true,
            11 => { r.func_ns.insert(6, "Other".into()); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_argument_copy(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name, namespace) in [(101, "FSource", "Qualified"), (102, "FResult", "Qualified"), (103, "FSource", "Other")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_names.insert(name.into()); r.typeid_to_ptr.insert(ptr as i32, ptr);
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), namespace: namespace.into(), module: "Synthetic".into() });
        }
        for (id, owner, name) in [(1, "FResult", "FResult"), (2, "FSource", "FSource"),
            (3, "FResult", "FResult"), (4, "FSource", "~FSource")] {
            r.funcid_to_ptr.insert(id as i32, id); r.func_by_ptr.insert(id, name.into());
            r.func_owner.insert(id, owner.into()); r.func_is_method.insert(id);
            r.func_ret.insert(id, DataType { token: 0x52, ..Default::default() });
            r.func_params.insert(id, Vec::new());
        }
        r.script_ctor_owner.extend([(1, 102), (2, if fault == 1 { 103 } else { 101 }), (3, 102)]);
        r.func_params.insert(3, vec![DataType { token: 5, type_info: if fault == 2 { 103 } else { 101 },
            is_reference: true, is_object_const: fault != 3, is_read_only: fault != 3, ..Default::default() },
            DataType { token: 0x50, is_object_const: true, is_read_only: true, ..Default::default() }]);
        if fault == 4 { r.func_is_method.remove(&3); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_constructors(
        namespaces: &[&str], name: &str, params: &[DataType],
    ) -> Self {
        let mut r = Self::default();
        r.type_names.insert(name.to_owned());
        for (index, namespace) in namespaces.iter().enumerate() {
            let key = index as i64 + 1;
            let owner = key + 100;
            r.type_by_ptr.insert(owner, name.to_owned());
            r.type_identity_by_ptr.insert(owner, TypeIdentity {
                name: name.to_owned(), module: "Synthetic".to_owned(),
                namespace: (*namespace).to_owned(),
            });
            r.funcid_to_ptr.insert(key as i32, key);
            r.func_by_ptr.insert(key, name.to_owned());
            r.func_is_method.insert(key);
            r.func_owner.insert(key, name.to_owned());
            r.script_ctor_owner.insert(key, owner);
            r.func_params.insert(key, params.to_vec());
            r.func_ret.insert(key, DataType { token: 0x52, ..Default::default() });
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_fstring_literal_operand(string_literal: bool) -> Self {
        let mut r = Self::from_test_fname_string_operators(string_literal, "FString");
        r.func_params.get_mut(&2).unwrap()[0].type_info = 101;
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_temporary_effect_context(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(101, "FGameplayEffectContextHandle"), (102, "FGameplayEffectSpecHandle"),
            (103, "TSubclassOf"), (104, "UAbilitySystemComponent")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (ptr, name, owner) in [(1, "MakeEffectContext", "UAbilitySystemComponent"),
            (2, "MakeOutgoingSpec", "UAbilitySystemComponent"), (3, "$beh2", "FGameplayEffectContextHandle")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
        }
        r.const_method_ptrs.extend([1, 2]);
        r.func_params.insert(1, vec![]); r.func_params.insert(3, vec![]);
        r.func_params.insert(2, vec![DataType { token: 5, type_info: 103, ..Default::default() },
            DataType { token: 0x50, ..Default::default() }, DataType { token: 5, type_info: 101, ..Default::default() }]);
        r.func_ret.insert(1, DataType { token: 5, type_info: 101, ..Default::default() });
        r.func_ret.insert(2, DataType { token: 5, type_info: 102, ..Default::default() });
        r.func_ret.insert(3, DataType { token: 0x52, ..Default::default() });
        r.func_ret_names.insert("MakeEffectContext".into(), "FGameplayEffectContextHandle".into());
        r.func_ret_names.insert("MakeOutgoingSpec".into(), "FGameplayEffectSpecHandle".into());
        match fault {
            1 => { r.const_method_ptrs.remove(&1); }
            2 => { r.func_params.get_mut(&2).unwrap()[2].type_info = 102; }
            3 => { r.func_ret.get_mut(&1).unwrap().is_reference = true; }
            4 => { r.func_owner.insert(3, "FGameplayEffectSpecHandle".into()); }
            5 => { r.type_identity_by_ptr.get_mut(&101).unwrap().module = "Script".into(); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_fname_string_operators(string_literal: bool, arg_type: &str) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "FString".into());
        r.type_by_ptr.insert(102, arg_type.into());
        for (ptr, name) in [(1, "opAdd_r"), (2, "opAdd"), (3, "$beh2")] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_is_method.insert(ptr);
            r.func_owner.insert(ptr, "FString".into());
        }
        for ptr in [1, 2] {
            r.func_params.insert(ptr, vec![DataType { token: 5, type_info: 102,
                is_reference: true, is_object_const: true, ..Default::default() }]);
            r.func_ret.insert(ptr, DataType { token: 5, type_info: 101, ..Default::default() });
        }
        r.func_by_ptr.insert(4, "GetName".into());
        r.func_params.insert(4, Vec::new());
        r.func_ret.insert(4, DataType { token: 5, type_info: 102, ..Default::default() });
        r.global_by_ptr.insert(100, "prefix".into());
        if string_literal { r.global_is_string.insert(100); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_const_native_store(owner: &str, field: &str, value: &str) -> Self {
        let mut r = Self::from_test_member_chain(&[(owner, field), (value, "")]);
        r.func_by_ptr.insert(3, "Make".into());
        r.func_params.insert(3, Vec::new());
        r.func_ret.insert(3, DataType { token: 5, type_info: 2,
            is_object_const: true, is_object_handle: true, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_rvo_then_member_address() -> Self {
        let mut r = Self::from_test_member_chain(&[("UHost", "Distance")]);
        r.type_by_ptr.insert(101, "FString".into());
        r.funcid_to_ptr.insert(10, 10);
        r.func_by_ptr.insert(10, "MakeValue".into());
        r.func_owner.insert(10, "UHost".into());
        r.func_is_method.insert(10);
        r.func_params.insert(10, Vec::new());
        r.func_ret.insert(10, DataType { token: 5, type_info: 101, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reused_bool_guards(fault: u8) -> Self {
        let mut r = Self::default();
        let boolean = DataType { token: 0x41, ..Default::default() };
        for (ptr, name, params) in [(1, "First", vec![]), (2, "Second", vec![boolean.clone()]),
            (3, "Ignore", vec![]), (4, "Final", vec![])] {
            r.funcid_to_ptr.insert(ptr as i32, ptr);
            r.func_by_ptr.insert(ptr, name.into()); r.func_params.insert(ptr, params);
            r.func_ret.insert(ptr, boolean.clone());
        }
        for (ptr, name) in [(5, "$beh2"), (6, "$beh0")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, "FHitResult".into());
            r.func_is_method.insert(ptr); r.func_params.insert(ptr, vec![]);
            r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        r.type_by_ptr.insert(100, "FHitResult".into());
        r.type_identity_by_ptr.insert(100, TypeIdentity { name: "FHitResult".into(), module: String::new(), namespace: String::new() });
        r.temporary_arg_positions.insert("Second".into(), HashMap::from([(1, vec![true])]));
        if fault == 1 { r.func_ret.get_mut(&1).unwrap().token = 0x44; }
        if fault == 2 { r.func_ret.get_mut(&2).unwrap().is_reference = true; }
        if fault == 3 { r.func_ret.get_mut(&3).unwrap().token = 0x44; }
        if fault == 4 { r.func_ret.get_mut(&4).unwrap().is_object_handle = true; }
        if fault == 5 { r.func_owner.insert(5, "FOtherHit".into()); }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_copied_negated_boolean(token: i32, reference: bool) -> Self {
        let mut r=Self::default();
        r.func_by_ptr.insert(1,"Trace".into());r.funcid_to_ptr.insert(1,1);
        r.func_ret.insert(1,DataType {token,is_reference:reference,..Default::default()});
        r.func_params.insert(1,vec![]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_prepared_enum_argument(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("FVector",""),("FHitResult",""),("FLinearColor",""),("UObject",""),
            ("AActor",""),("ETraceKind",""),("ECollisionKind",""),("TArray",""),("EDrawKind","")]);
        for (p,name) in [(1,"FVector"),(2,"FHitResult"),(3,"FLinearColor"),(4,"UObject"),(5,"AActor"),
            (6,"ETraceKind"),(7,"ECollisionKind"),(8,"TArray"),(9,"EDrawKind")] {
            r.type_identity_by_ptr.insert(p,TypeIdentity {name:name.into(),module:String::new(),namespace:String::new()});
        }
        let value=|p| DataType {token:5,type_info:p,..Default::default()};
        let boolean=DataType {token:0x41,..Default::default()};
        let float=DataType {token:0x50,..Default::default()};let void=DataType {token:0x52,..Default::default()};
        let vector=DataType {is_object_const:true,is_read_only:true,..value(1)};
        let trace=vec![DataType {is_object_handle:true,is_object_const:true,..value(4)},vector.clone(),vector,value(6),boolean.clone(),
            DataType {is_reference:true,is_object_const:true,is_read_only:true,..value(8)},value(9),
            DataType {is_reference:true,..value(2)},boolean.clone(),value(3),value(3),float.clone()];
        for (p,name,owner,params,ret) in [(10,"Convert",None,vec![value(7)],value(6)),
            (20,"$beh0",Some("FLinearColor"),vec![float;4],void.clone()),(21,"$beh0",Some("FHitResult"),vec![],void),
            (30,"Trace",None,trace,boolean),(40,"GetLocation",Some("AActor"),vec![],value(1))] {
            r.func_by_ptr.insert(p,name.into());r.func_params.insert(p,params);r.func_ret.insert(p,ret);
            if let Some(owner)=owner {r.func_owner.insert(p,owner.into());r.func_is_method.insert(p);}
        }
        r.const_method_ptrs.insert(40);
        r.global_by_ptr.insert(99,"__WorldContext".into());r.global_by_ptr.insert(100,"Red".into());r.global_by_ptr.insert(101,"Green".into());
        r.temporary_arg_positions.insert("Trace".into(),HashMap::from([(11,vec![true,true,true,true,true,true,false,true,true,true,true])]));
        r.ctor_arg_positions.insert("ETraceKind".into(),HashMap::from([(1,vec![true])]));
        if fault==1 {r.func_ret.get_mut(&10).unwrap().is_reference=true;}
        if fault==2 {r.func_params.get_mut(&30).unwrap()[3].type_info=7;}
        if fault==3 {r.func_ret.get_mut(&40).unwrap().type_info=3;}
        if fault==4 {r.const_method_ptrs.remove(&40);}
        if fault==5 {r.func_params.get_mut(&20).unwrap()[0].token=0x51;}
        if fault==6 {r.func_owner.insert(21,"FOtherHit".into());}
        if fault==7 {r.func_ret.get_mut(&30).unwrap().token=0x44;}
        if fault==8 {r.func_is_method.insert(30);}
        if fault==9 {r.global_by_ptr.insert(99,"OtherGlobal".into());}
        if fault==10 {r.func_params.get_mut(&30).unwrap()[1].is_read_only=false;}
        if fault==11 {r.type_identity_by_ptr.get_mut(&6).unwrap().module="Other".into();}
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_prepared_enum_getter_argument(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("EWeather",""),("FWeatherState",""),("FWeatherResult",""),("AWeatherHost","")]);
        for (p,name) in [(1,"EWeather"),(2,"FWeatherState"),(3,"FWeatherResult"),(4,"AWeatherHost")] {
            r.type_identity_by_ptr.insert(p,TypeIdentity {name:name.into(),module:String::new(),namespace:String::new()});
        }
        let value=|p|DataType {token:5,type_info:p,..Default::default()};
        let mut enum_arg=value(1);enum_arg.is_object_const=true;enum_arg.is_read_only=true;
        let state=DataType {is_reference:true,is_object_const:true,is_read_only:true,..value(2)};
        for (p,name,owner,params,ret) in [(10,"GetCurrentWeather","AWeatherHost",vec![],value(1)),
            (20,"GetChoices","AWeatherHost",vec![enum_arg,state],value(3)),
            (30,"Inspect","FWeatherResult",vec![],DataType {token:0x52,..Default::default()})] {
            r.func_by_ptr.insert(p,name.into());r.func_owner.insert(p,owner.into());r.func_is_method.insert(p);
            r.func_params.insert(p,params);r.func_ret.insert(p,ret);r.funcid_to_ptr.insert(p as i32,p);
        }
        r.temporary_arg_positions.insert("GetChoices".into(),HashMap::from([(2,vec![true,false])]));
        r.ctor_arg_positions.insert("EWeather".into(),HashMap::from([(1,vec![true])]));
        match fault {
            1=>r.func_ret.get_mut(&10).unwrap().is_reference=true,
            2=>{r.func_is_method.remove(&10);},
            3=>r.func_params.get_mut(&10).unwrap().push(value(1)),
            4=>r.func_params.get_mut(&20).unwrap()[0].type_info=2,
            5=>r.func_params.get_mut(&20).unwrap()[0].is_reference=true,
            6=>r.func_params.get_mut(&20).unwrap()[1].is_reference=false,
            7=>r.func_params.get_mut(&20).unwrap()[1].is_read_only=false,
            8=>{r.func_is_method.remove(&20);},
            9=>r.func_ret.get_mut(&20).unwrap().is_object_handle=true,
            10=>r.func_ret.get_mut(&20).unwrap().type_info=2,
            11=>r.type_identity_by_ptr.get_mut(&1).unwrap().module="Other".into(),
            12=>r.func_params.get_mut(&20).unwrap().truncate(1),
            _=>{},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_guard_field_selection(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UConfig", ""), ("UState", ""), ("UObject", ""), ("UState", "")]);
        for (id, name) in [(1, "UConfig"), (2, "UState"), (3, "UObject"), (4, "UState")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(),
                module: if id == 3 { "" } else if id == 4 { "OtherModule" } else { "Fixture" }.into(),
                namespace: String::new() });
        }
        let mut fields: HashMap<String, HashMap<String, String>> = HashMap::new();
        for (id, owner, entries) in [(1, "UConfig", vec![(0, "A"), (8, "B"), (16, "C")]),
            (2, "UState", vec![(0, "FallbackA"), (8, "FallbackB"), (16, "FallbackC"),
                (24, "ResolvedA"), (32, "ResolvedB"), (40, "ResolvedC")])]
        {
            for (offset, name) in entries {
                let key = ((id as i64) << 1) | ((offset as i64) << 33) | 1;
                r.prop_by_key.insert(key, name.into());
                r.prop_type_id.insert(key, if fault == 2 && id == 2 { 4 } else { id });
                fields.entry(owner.into()).or_default().insert(name.into(),
                    if fault == 1 && id == 1 { "float32" } else { "float" }.into());
            }
        }
        r.set_class_fields(fields);
        r.func_by_ptr.insert(10, "AcceptConfig".into());
        r.func_ret_names.insert("AcceptConfig".into(), "bool".into());
        r.func_ret.insert(10, DataType { token: if fault == 3 { 0x44 } else { 0x41 }, ..Default::default() });
        r.func_params.insert(10, vec![DataType { token: 5, type_info: 3,
            is_object_handle: true, is_object_const: true, is_reference: fault == 4, ..Default::default() }]);
        if fault == 5 { r.func_is_method.insert(10); }
        if fault == 6 { r.func_ns.insert(10, "Other".into()); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_member_chain(owners: &[(&str, &str)]) -> Self {
        let mut r = Self::default();
        for (index, (owner, field)) in owners.iter().enumerate() {
            let id = index as i32 + 1;
            r.typeid_to_ptr.insert(id, id as i64);
            r.type_by_ptr.insert(id as i64, (*owner).to_owned());
            r.type_names.insert((*owner).to_owned());
            if !field.is_empty() {
                // Each synthetic owner has its one property at byte offset zero.
                r.prop_by_key.insert(((id as i64) << 1) | 1, (*field).to_owned());
            }
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_const_assignment(ty: &str) -> Self {
        let mut r = Self::default();
        r.temporary_arg_methods.insert(format!("opAssign/{ty}"));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_const_object_fields() -> Self {
        let mut r = Self::default();
        // Same bare owner in another namespace/module; same bare value type at
        // a different pointer. None may borrow the first owner's field witness.
        for (id, module, namespace, name) in [
            (1, "Fixture", "One", "FHolder"), (2, "Fixture", "Two", "FHolder"),
            (3, "Other", "One", "FHolder"), (4, "", "First", "UValue"),
            (5, "", "Second", "UValue"),
        ] {
            r.typeid_to_ptr.insert(id, id as i64);
            r.type_by_ptr.insert(id as i64, name.into());
            r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(id as i64, TypeIdentity {
                module: module.into(), namespace: namespace.into(), name: name.into(),
            });
            if id <= 3 { r.prop_by_key.insert(((id as i64) << 1) | 1, "Value".into()); }
        }
        r.set_qualified_fields([(
            TypeIdentity { module: "Fixture".into(), namespace: "One".into(), name: "FHolder".into() },
            "Value".into(), DataType { token: 5, type_info: 4,
                is_object_const: true, is_object_handle: true, ..Default::default() },
        )]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_default_rvo_return(fault: u8) -> Self {
        let mut r = Self::from_test_native_default_constructor("TArray", 0, true);
        r.type_by_ptr.insert(102, "FEntry".into()); r.type_names.insert("FEntry".into());
        r.type_subtypes.insert(101, vec![DataType { token: 5, type_info: 102, ..Default::default() }]);
        for (id, name) in [(2, "opAssign"), (3, "$beh2")] {
            r.func_by_ptr.insert(id, name.into()); r.func_owner.insert(id, "TArray".into());
            r.func_is_method.insert(id); r.func_params.insert(id, Vec::new());
        }
        r.func_params.insert(2, vec![DataType { token: 5, type_info: if fault == 1 { 102 } else { 101 },
            is_reference: true, is_read_only: fault != 2, ..Default::default() }]);
        r.func_ret.insert(2, DataType { token: 5, type_info: 101, is_reference: true, ..Default::default() });
        r.func_ret.insert(3, DataType { token: 0x52, ..Default::default() });
        if fault == 3 { r.func_owner.insert(3, "FOther".into()); }
        if fault == 4 { r.func_is_method.remove(&1); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_returned_string_scope(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UHost", "Key"), ("FName", ""), ("FString", ""), ("FReplicatedStringMap", "")]);
        for (p, name) in [(1, "UHost"), (2, "FName"), (3, "FString"), (4, "FReplicatedStringMap")] {
            r.type_identity_by_ptr.insert(p, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        r.prop_type_id.insert(3, 1);
        r.set_class_fields(HashMap::from([("UHost".into(), HashMap::from([("Key".into(), "FName".into())]))]));
        let value = |ty| DataType { token: 5, type_info: ty, ..Default::default() };
        let input = |ty| DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value(ty) };
        for (p, name, owner) in [(1, "GetDataFrom", "UHost"), (2, "$beh0", "FName"), (3, "$beh2", "FString"),
            (4, "Use", "UHost"), (5, "Later", "UHost"), (7, "ContainsData", "UHost")] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into()); r.func_is_method.insert(p);
            r.func_params.insert(p, Vec::new()); r.func_ret.insert(p, DataType { token: 0x52, ..Default::default() });
        }
        r.func_ret.insert(1, value(3)); r.func_ret.get_mut(&7).unwrap().token = 0x41;
        for p in [1, 7] { r.const_method_ptrs.insert(p); r.func_params.insert(p, vec![input(4), input(2)]); }
        r.func_params.insert(2, vec![input(3)]); r.func_params.insert(4, vec![value(2)]);
        for name in ["Use", "FName"] { r.temporary_arg_positions.insert(name.into(), HashMap::from([(1, vec![true])])); }
        match fault {
            1 => { r.const_method_ptrs.remove(&1); }
            2 => r.func_ret.get_mut(&1).unwrap().is_reference = true,
            3 => r.func_ret.get_mut(&1).unwrap().type_info = 2,
            4 => r.func_params.get_mut(&2).unwrap()[0].type_info = 2,
            5 => r.func_params.get_mut(&2).unwrap()[0].is_object_const = false,
            6 => { r.func_params.insert(3, vec![value(3)]); }
            7 => { r.func_owner.insert(3, "FName".into()); }
            8 => r.type_identity_by_ptr.get_mut(&3).unwrap().module = "Script".into(),
            9 => r.type_identity_by_ptr.get_mut(&2).unwrap().namespace = "Other".into(),
            10 => r.func_params.get_mut(&1).unwrap()[1].type_info = 3,
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_literal_string_scope(fault: u8) -> Self {
        let mut r = Self::from_test_native_default_constructor("FString", 0, true);
        r.type_by_ptr.insert(102, "FSettings".into()); r.type_names.insert("FSettings".into());
        r.global_by_ptr.insert(100, "Warning".into()); if fault != 1 { r.global_is_string.insert(100); }
        let input = DataType { token: 5, type_info: 101, is_reference: true,
            is_read_only: true, ..Default::default() };
        r.func_params.insert(1, vec![DataType { type_info: if fault == 2 { 102 } else { 101 },
            is_read_only: fault != 3, ..input.clone() }]);
        for (id, name, owner) in [(2, "Setup", "UHost"), (3, "$beh2", "FString"),
            (4, "Use", "UHost"), (5, "$beh2", "FSettings"), (6, "Later", "UHost")] {
            r.func_by_ptr.insert(id, name.into()); r.funcid_to_ptr.insert(id as i32, id);
            r.func_owner.insert(id, owner.into()); r.func_is_method.insert(id);
            r.func_ret.insert(id, DataType { token: 0x52, ..Default::default() });
            r.func_params.insert(id, Vec::new());
        }
        r.func_params.insert(2, vec![input.clone()]);
        r.func_params.insert(4, vec![DataType { type_info: 102, ..input }]);
        r.func_ret.insert(2, DataType { token: 5, type_info: 102, ..Default::default() });
        for name in ["Setup", "Use"] {
            r.temporary_arg_positions.insert(name.into(), HashMap::from([(1, vec![true])]));
        }
        if fault == 4 { r.func_ret.get_mut(&6).unwrap().token = 0x41; }
        if fault == 5 { r.func_params.insert(3, vec![DataType::default()]); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_literal_string_rvo_return(literal: &str, fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(101, "FString"), (102, "FName")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
        }
        r.global_by_ptr.insert(100, literal.into()); if fault != 1 { r.global_is_string.insert(100); }
        for (id, name, owner) in [(1, "$beh0", "FString"), (2, "Level", "UAbility"),
            (3, "Append", "FString"), (4, "$beh0", "FName"), (5, "$beh0", "FName"),
            (6, "$beh2", "FString")] {
            r.func_by_ptr.insert(id, name.into()); r.func_is_method.insert(id);
            r.func_owner.insert(id, owner.into()); r.func_params.insert(id, Vec::new());
            r.func_ret.insert(id, DataType { token: 0x52, ..Default::default() });
        }
        for (id, token, ty) in [(1, 5, 101), (3, 0x44, 0), (4, 5, 101), (5, 5, 102)] {
            r.func_params.insert(id, vec![DataType { token, type_info: if fault == 2 && id == 4 { 102 } else { ty },
                is_reference: true, is_object_const: fault != 3, is_read_only: fault != 3, ..Default::default() }]);
        }
        r.func_ret.insert(2, DataType { token: if fault == 4 { 0x50 } else { 0x44 }, ..Default::default() });
        r.func_ret.insert(3, DataType { token: 5, type_info: 101, is_reference: fault != 5, ..Default::default() });
        if fault == 6 { r.func_params.insert(6, vec![DataType::default()]); }
        r.temporary_arg_positions.insert("Append".into(), HashMap::from([(1, vec![true])]));
        r.ctor_arg_positions.insert("FName".into(), HashMap::from([(1, vec![true])]));
        r.zero_arg_names.insert("Level".into());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_out_argument_read_order(readonly: bool) -> Self {
        let mut r = Self::default();
        r.temporary_arg_positions.insert("ReadAttribute".into(),
            HashMap::from([(3, vec![true, true, readonly])]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_literal_value_lifetime(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(100, "FLocalizedValue".into());
        r.type_by_ptr.insert(101, "FString".into());
        r.global_by_ptr.insert(10, "text".into()); if fault != 1 { r.global_is_string.insert(10); }
        r.set_class_fields(HashMap::from([("FSettings".into(), HashMap::from([("Value".into(), "FLocalizedValue".into())]))]));
        for (id, name) in [(1, "$beh0"), (2, "opAssign"), (3, "opAssign"), (4, "$beh2")] {
            r.func_by_ptr.insert(id, name.into()); r.func_is_method.insert(id);
            r.func_owner.insert(id, if fault == 5 && id == 1 { "FOther" } else { "FLocalizedValue" }.into());
            r.func_params.insert(id, Vec::new());
            r.func_ret.insert(id, DataType { token: 0x52, ..Default::default() });
        }
        for (id, ty) in [(2, 101), (3, 100)] {
            r.func_params.insert(id, vec![DataType { token: 5, type_info: if fault == 2 { 999 } else { ty },
                is_reference: true, is_object_const: fault != 3, is_read_only: fault != 3, ..Default::default() }]);
            r.func_ret.insert(id, DataType { token: 5, type_info: 100, is_reference: true, ..Default::default() });
        }
        if fault == 4 { r.func_params.insert(4, vec![DataType::default()]); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_field_initializer(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(101, "Example"), (201, "FSoftValue"), (202, "FClassValue"), (301, "FName"), (302, "UClass")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: if ptr == 101 { "Module" } else { "" }.into(),
                namespace: if ptr == 101 { "NS" } else { "" }.into() });
        }
        r.typeid_to_ptr.insert(11, 101);
        for (offset, name) in [(8, "Path"), (16, "Kind")] {
            let key = (11i64 << 1) | ((offset as i64) << 33) | 1;
            r.prop_by_key.insert(key, name.into()); r.prop_type_id.insert(key, if fault == 1 { 99 } else { 11 });
        }
        for (id, name, owner) in [(1, "__STATIC_NAME", ""), (2, "$beh0", "FSoftValue"), (3, "opAssign", "FSoftValue"),
            (4, "StaticClass", ""), (5, "$beh0", "FClassValue"), (6, "opAssign", "FClassValue")] {
            r.func_by_ptr.insert(id, name.into());
            if !owner.is_empty() { r.func_is_method.insert(id); r.func_owner.insert(id, if fault == 2 { "FOther" } else { owner }.into()); }
            r.func_params.insert(id, Vec::new()); r.func_ret.insert(id, DataType { token: 0x52, ..Default::default() });
        }
        r.funcid_to_ptr.insert(44, 4); r.static_names.push("/Game/Example".into());
        r.func_params.insert(1, vec![DataType { token: 0x44, ..Default::default() }]);
        r.func_ret.insert(1, DataType { token: 5, type_info: 301, is_reference: true, is_object_const: true, is_read_only: true, ..Default::default() });
        r.func_params.insert(3, vec![DataType { token: 5, type_info: if fault == 3 { 302 } else { 301 }, ..Default::default() }]);
        r.func_ret.insert(3, DataType { token: 5, type_info: 201, is_reference: true, ..Default::default() });
        r.func_ret.insert(4, DataType { token: 5, type_info: 302, is_object_handle: fault != 4, ..Default::default() });
        r.func_params.insert(6, vec![DataType { token: 5, type_info: 302, is_object_handle: true, ..Default::default() }]);
        if fault == 5 { r.func_params.insert(2, vec![DataType::default()]); }
        if fault >= 6 {
            r.type_by_ptr.insert(203, "FBaseValue".into());
            let mut identity = r.type_identity_by_ptr[&201].clone();
            identity.name = "FBaseValue".into();
            if fault == 9 { identity.namespace = "Other".into(); }
            r.type_identity_by_ptr.insert(203, identity);
            r.func_ret.get_mut(&3).unwrap().type_info = 203;
            if fault == 7 { r.func_ret.get_mut(&3).unwrap().is_object_handle = true; }
            if fault == 8 { r.func_ret.get_mut(&3).unwrap().is_read_only = true; }
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_name_initializer(fault: u8) -> Self {
        let mut r = Self::from_test_native_field_initializer(0);
        for (offset, name) in [(24, "TimeA"), (32, "TimeB"), (40, "Enabled"), (48, "NameA"), (56, "NameB")] {
            let key = (11i64 << 1) | ((offset as i64) << 33) | 1;
            r.prop_by_key.insert(key, name.into());
            r.prop_type_id.insert(key, if fault == 1 && offset == 48 { 99 } else { 11 });
        }
        r.static_names = vec!["First".into(), "Second".into()];
        r.func_by_ptr.insert(7, "$beh0".into()); r.func_is_method.insert(7);
        r.func_owner.insert(7, if fault == 2 { "FOther" } else { "FName" }.into());
        r.func_params.insert(7, vec![DataType { token: 5, type_info: if fault == 3 { 201 } else { 301 },
            is_reference: fault != 4, is_object_const: fault != 5, is_read_only: true, ..Default::default() }]);
        r.func_ret.insert(7, DataType { token: if fault == 6 { 0x41 } else { 0x52 }, ..Default::default() });
        if fault == 7 { r.func_ret.get_mut(&1).unwrap().is_read_only = false; }
        if fault == 8 { r.type_identity_by_ptr.get_mut(&301).unwrap().namespace = "Other".into(); }
        if fault == 9 { r.const_method_ptrs.insert(7); }
        for (ptr, name) in [(8, "EnableTick"), (9, "EnableMovement")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_is_method.insert(ptr);
            r.func_owner.insert(ptr, "Example".into());
            r.func_params.insert(ptr, vec![DataType { token: 0x41, ..Default::default() }]);
            r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_direct_field(fault: u8) -> Self {
        let mut r = Self::from_test_native_field_initializer(if fault <= 2 { fault } else { 0 });
        r.func_params.insert(2, vec![DataType { token: 5, type_info: if fault == 3 { 302 } else { 301 },
            is_reference: fault == 4, ..Default::default() }]);
        if fault == 5 { r.func_ret.get_mut(&2).unwrap().token = 0x41; }
        if fault == 6 { r.func_ret.get_mut(&1).unwrap().is_object_const = false; }
        if fault == 7 { r.type_identity_by_ptr.get_mut(&201).unwrap().namespace = "Other".into(); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_captured_handle_member_reference(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("AVolume", "Settings"), ("FSettings", "Width"), ("FOther", "")]);
        for (id, name) in [(1, "AVolume"), (2, "FSettings"), (3, "FOther")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
            r.prop_type_id.insert((id << 1) | 1, id as i32);
        }
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("AVolume", "Settings", if fault == 1 { "FOther" } else { "FSettings" }),
            ("FSettings", "Width", "float32")], &[], None));
        r.func_by_ptr.insert(1, "GetValue".into()); r.func_owner.insert(1, "UGetter".into());
        r.func_is_method.insert(1); r.func_params.insert(1, Vec::new());
        r.func_ret.insert(1, DataType { token: 5, type_info: 1, is_reference: true, is_object_handle: true, ..Default::default() });
        match fault {
            2 => { r.func_ret.get_mut(&1).unwrap().is_reference = false; },
            3 => { r.func_ret.get_mut(&1).unwrap().is_object_handle = false; },
            4 => { r.func_ret.get_mut(&1).unwrap().is_object_const = true; },
            5 => { r.func_ret.get_mut(&1).unwrap().is_read_only = true; },
            6 => { r.func_params.get_mut(&1).unwrap().push(DataType { token: 0x44, ..Default::default() }); },
            7 => { r.const_method_ptrs.insert(1); },
            8 => { r.func_is_method.remove(&1); },
            9 => { r.prop_type_id.insert(3, 3); },
            10 => { r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(); },
            11 => { r.type_identity_by_ptr.get_mut(&2).unwrap().namespace = "Other".into(); },
            12 => { r.prop_type_id.insert(5, 3); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_psf_member_copy(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FEntry", "Name"), ("FName", ""), ("FOther", "")]);
        for (id, name) in [(1, "FEntry"), (2, "FName"), (3, "FOther")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        r.prop_type_id.insert((1 << 1) | 1, 1);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[("FEntry", "Name", if fault == 1 { "FOther" } else { "FName" })], &[], None));
        let value = DataType { token: 5, type_info: 2, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value.clone() };
        for (ptr, name, owner, ret, params) in [
            (1, "$beh0", "FName", DataType { token: 0x52, ..Default::default() }, vec![reference.clone()]),
            (2, "OtherName", "", value, vec![]),
            (3, "opEquals", "FName", DataType { token: 0x41, ..Default::default() }, vec![reference]),
        ] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, params);
            if !owner.is_empty() { r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); }
        }
        r.const_method_ptrs.insert(3);
        match fault {
            2 => { r.func_params.get_mut(&1).unwrap()[0].type_info = 3; },
            3 => { r.func_params.get_mut(&1).unwrap()[0].is_reference = false; },
            4 => { r.func_params.get_mut(&1).unwrap()[0].is_object_const = false; },
            5 => { r.func_params.get_mut(&1).unwrap()[0].is_read_only = false; },
            6 => { r.func_params.get_mut(&1).unwrap()[0].is_object_handle = true; },
            7 => { r.func_owner.insert(1, "FOther".into()); },
            8 => { r.func_ret.get_mut(&1).unwrap().is_reference = true; },
            9 => { r.func_is_method.remove(&1); },
            10 => { r.const_method_ptrs.insert(1); },
            11 => { r.prop_type_id.insert((1 << 1) | 1, 3); },
            12 => { r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(); },
            13 => { r.type_identity_by_ptr.get_mut(&2).unwrap().namespace = "Other".into(); },
            14 => { r.native = None; },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_value_selection(fault: u8) -> Self {
        let mut r = Self::from_test_native_psf_member_copy(fault);
        let void = DataType { token: 0x52, ..Default::default() };
        let value_ref = DataType { token: 5, type_info: 2, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() };
        for (ptr, name, owner, ret, params) in [
            (2, "$beh0", "FName", void.clone(), vec![]),
            (3, "$beh2", "FName", void.clone(), vec![]),
            (4, "Touch", "FName", void.clone(), vec![DataType { token: 0x44, ..Default::default() }]),
            (5, "ShouldTaunt", "", DataType { token: 0x41, ..Default::default() }, vec![]),
            (6, "UseNames", "FEntry", void, vec![value_ref.clone(), value_ref]),
            (7, "Left", "", DataType { token: 0x51, ..Default::default() }, vec![]),
            (8, "Right", "", DataType { token: 0x51, ..Default::default() }, vec![]),
        ] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, params);
            r.const_method_ptrs.remove(&ptr);
            if !owner.is_empty() { r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); }
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_typed_psf_conversion(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FString", ""), ("FName", "")]);
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "FString".into(), module: String::new(), namespace: String::new() });
        let string = DataType { token: 5, type_info: 1, ..Default::default() };
        let name = DataType { token: 5, type_info: 2, ..Default::default() };
        let reference = |t: DataType| DataType { is_reference: true, is_object_const: true, is_read_only: true, ..t };
        let void = DataType { token: 0x52, ..Default::default() };
        for (id, callee, ret, args) in [(1, "$beh0", void.clone(), vec![reference(string.clone())]),
            (2, "Source", string, vec![]), (3, "Allowed", DataType { token: 0x41, ..Default::default() }, vec![]),
            (4, "Save", void.clone(), vec![reference(name)]), (5, "$beh2", void, vec![])] {
            r.func_by_ptr.insert(id, callee.into()); r.func_ret.insert(id, ret); r.func_params.insert(id, args);
        }
        r.func_is_method.extend([1, 5]); r.func_owner.insert(1, "FName".into()); r.func_owner.insert(5, "FString".into());
        if fault == 1 { r.func_params.get_mut(&1).unwrap()[0].type_info = 2; }
        if fault == 2 { r.func_params.get_mut(&1).unwrap()[0].is_reference = false; }
        if fault == 3 { r.func_params.get_mut(&1).unwrap()[0].is_object_const = false; }
        if fault == 4 { r.func_params.get_mut(&1).unwrap()[0].is_read_only = false; }
        if fault == 5 { r.func_params.get_mut(&1).unwrap()[0].is_object_handle = true; }
        if fault == 6 { r.func_owner.insert(1, "FOther".into()); }
        if fault == 7 { r.const_method_ptrs.insert(1); }
        if fault == 8 { r.func_ret.get_mut(&1).unwrap().is_reference = true; }
        if fault == 9 { r.type_identity_by_ptr.get_mut(&1).unwrap().namespace = "Other".into(); }
        if fault == 10 { r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(); }
        if fault == 11 { r.func_is_method.remove(&1); }
        if fault == 12 { r.func_params.get_mut(&1).unwrap().clear(); }
        if (13..=16).contains(&fault) {
            let p = &mut r.func_params.get_mut(&1).unwrap()[0];
            p.is_reference = false; p.is_object_const = false; p.is_read_only = false;
            p.is_auto = fault == 14; p.if_handle_then_const = fault == 15; p.is_object_handle = fault == 16;
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_entry_constructed_value(fault: u8) -> Self {
        let mut r = Self::default();
        for (id, name) in [(101, "FVector"), (102, "FRotator"), (103, "FTransform")] {
            r.type_by_ptr.insert(id, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let void = DataType { token: 0x52, ..Default::default() };
        let reference = |id| DataType { token: 5, type_info: id, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() };
        for (id, name, owner, args) in [(1, "$beh0", "FVector", vec![]),
            (2, "$beh0", "FRotator", vec![]),
            (3, "$beh0", "FTransform", vec![reference(102), reference(101), reference(101)]),
            (6, "$beh2", "FVector", vec![])] {
            r.func_by_ptr.insert(id, name.into()); r.func_owner.insert(id, owner.into());
            r.func_is_method.insert(id); r.func_ret.insert(id, void.clone()); r.func_params.insert(id, args);
        }
        r.func_by_ptr.insert(4, "Use".into()); r.func_ret.insert(4, void);
        r.func_params.insert(4, vec![reference(103), DataType { token: 0x44, ..Default::default() }]);
        r.global_by_ptr.insert(100, "OneVector".into()); r.global_ns.insert(100, "FVector".into());
        match fault {
            1 => r.func_params.get_mut(&3).unwrap()[1].is_read_only = false,
            2 => r.type_identity_by_ptr.get_mut(&101).unwrap().module = "Script".into(),
            3 => r.func_ret.get_mut(&1).unwrap().token = 0x41,
            4 => { r.func_is_method.remove(&1); }
            5 => { r.func_owner.insert(1, "FOther".into()); }
            6 => r.func_params.get_mut(&3).unwrap()[1].type_info = 102,
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_rvo_constructor_input(fault: u8) -> Self {
        let mut r = Self::from_test_entry_constructed_value(0);
        r.type_by_ptr.insert(104, "USceneComponent".into()); r.type_names.insert("USceneComponent".into());
        r.type_identity_by_ptr.insert(104, TypeIdentity { name: "USceneComponent".into(), module: String::new(), namespace: String::new() });
        r.func_by_ptr.insert(7, "GetSocketLocation".into()); r.func_owner.insert(7, "USceneComponent".into());
        r.func_is_method.insert(7); r.const_method_ptrs.insert(7); r.func_params.insert(7, Vec::new());
        r.func_ret.insert(7, DataType { token: 5, type_info: 101, ..Default::default() });
        r.func_by_ptr.insert(8, "Rotation".into());
        r.func_params.insert(8, vec![DataType { token: 5, type_info: 101, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() }]);
        r.func_ret.insert(8, DataType { token: 5, type_info: 102, ..Default::default() });
        match fault {
            1 => r.func_ret.get_mut(&7).unwrap().type_info = 102,
            2 => r.func_ret.get_mut(&7).unwrap().is_reference = true,
            3 => r.func_ret.get_mut(&7).unwrap().is_object_handle = true,
            4 => { r.const_method_ptrs.remove(&7); }
            5 => { r.func_is_method.remove(&7); }
            6 => r.func_params.get_mut(&3).unwrap()[1].is_read_only = false,
            7 => r.func_params.get_mut(&3).unwrap()[1].type_info = 104,
            8 => r.type_identity_by_ptr.get_mut(&101).unwrap().module = "Script".into(),
            9 => r.func_ret.get_mut(&7).unwrap().is_read_only = true,
            10 => { r.func_owner.insert(7, "FValue".into()); }
            11 => { r.func_params.remove(&7); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_conditional_constructor_arguments(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "FVector".into()); r.type_names.insert("FVector".into());
        r.type_identity_by_ptr.insert(101, TypeIdentity { name: "FVector".into(), module: String::new(), namespace: String::new() });
        for ptr in [1, 2] {
            r.func_by_ptr.insert(ptr, "$beh0".into()); r.func_owner.insert(ptr, "FVector".into());
            r.func_is_method.insert(ptr); r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        r.func_params.insert(1, vec![DataType { token: 0x51, ..Default::default() }; 3]);
        r.func_params.insert(2, vec![DataType { token: 5, type_info: 101, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() }]);
        r.ctor_arg_positions.insert("FVector".into(), HashMap::from([(3, vec![true; 3])]));
        match fault {
            1 => r.func_params.get_mut(&1).unwrap()[0].token = 0x50,
            2 => r.func_params.get_mut(&1).unwrap()[0].is_reference = true,
            3 => r.type_identity_by_ptr.get_mut(&101).unwrap().namespace = "Other".into(),
            4 => r.func_ret.get_mut(&1).unwrap().token = 0x41,
            5 => { r.func_is_method.remove(&1); }
            6 => { r.func_params.get_mut(&1).unwrap().pop(); }
            7 => { r.const_method_ptrs.insert(1); }
            8 => { r.func_owner.insert(1, "FPoint".into()); }
            9 => r.func_params.get_mut(&1).unwrap()[0].is_read_only = true,
            10 => r.type_identity_by_ptr.get_mut(&101).unwrap().module = "Script".into(),
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_default_constructor(owner: &str, params: usize, returns_void: bool) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, owner.into());
        r.type_names.insert(owner.into());
        r.func_by_ptr.insert(1, "$beh0".into());
        r.func_owner.insert(1, owner.into());
        r.func_is_method.insert(1);
        r.func_params.insert(1, vec![DataType { token: 0x44, ..Default::default() }; params]);
        r.func_ret.insert(1, DataType { token: if returns_void { 0x52 } else { 0x41 }, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_narrowed_argument_lives(fault: u8) -> Self {
        let mut r = Self::from_test_widened_division_fields(if fault == 1 { "float32" } else { "double" });
        r.func_by_ptr.insert(1, "Height".into()); r.func_owner.insert(1, "UComponent".into());
        r.func_is_method.insert(1); r.const_method_ptrs.insert(1); r.func_params.insert(1, Vec::new());
        r.func_ret.insert(1, DataType { token: 0x50, is_reference: true, is_object_const: true,
            is_read_only: true, ..Default::default() });
        match fault {
            2 => r.func_ret.get_mut(&1).unwrap().token = 0x51,
            3 => r.func_ret.get_mut(&1).unwrap().is_reference = false,
            4 => r.func_ret.get_mut(&1).unwrap().is_object_const = false,
            5 => r.func_ret.get_mut(&1).unwrap().is_read_only = false,
            6 => { r.const_method_ptrs.remove(&1); }
            7 => { r.func_is_method.remove(&1); }
            8 => r.func_params.get_mut(&1).unwrap().push(DataType { token: 0x44, ..Default::default() }),
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_widened_division_fields(field_type: &str) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(100, "UCounter".into()); r.typeid_to_ptr.insert(100, 100);
        r.type_identity_by_ptr.insert(100, TypeIdentity { name: "UCounter".into(), module: "Test".into(), namespace: String::new() });
        for (offset, name) in [(8, "MoveCount"), (12, "AttackCount")] {
            let key = (100i64 << 1) | (offset << 33) | 1;
            r.prop_by_key.insert(key, name.into()); r.prop_type_id.insert(key, 100);
        }
        r.set_class_fields(HashMap::from([("UCounter".into(), HashMap::from([
            ("MoveCount".into(), field_type.into()), ("AttackCount".into(), field_type.into())]))]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_pushed_handle_selections(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(100, "AState"), (200, "ACharacterChild"), (201, "AActor"), (202, "UObject"), (300, "UReceiver"), (400, "FTag"), (500, "UOther")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let handle = |ptr| DataType { token: 5, type_info: ptr, is_object_handle: true, ..Default::default() };
        for (ptr, name) in [(1, "IsValid"), (2, "GetCharacter"), (3, "Consume"), (4, "GetReceiver")] { r.func_by_ptr.insert(ptr, name.into()); }
        for (ptr, owner) in [(2, if fault == 3 { "UOther" } else { "AState" }), (3, "UReceiver"), (4, "UOwner")] {
            r.func_is_method.insert(ptr); r.func_owner.insert(ptr, owner.into());
        }
        r.funcid_to_ptr.insert(4, 4);
        r.func_ret.insert(1, DataType { token: if fault == 4 { 0x44 } else { 0x41 }, ..Default::default() });
        r.func_ret.insert(2, handle(200)); r.func_ret.insert(4, handle(300));
        r.func_ret.insert(3, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(1, vec![handle(202)]); r.func_params.insert(2, Vec::new()); r.func_params.insert(4, Vec::new());
        let mut params = vec![DataType { token: 5, type_info: 400, is_reference: true, is_object_const: true, is_read_only: true, ..Default::default() },
            DataType { token: 0x50, ..Default::default() }, handle(201), handle(201), handle(201), handle(if fault == 1 { 500 } else { 201 })];
        if fault == 2 { params.pop(); }
        r.func_params.insert(3, params); r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reused_value_chain_argument(mutable: bool) -> Self {
        let mut r = Self::default();
        for (p, name) in [(100, "FVector"), (101, "ACharacter")] { r.type_by_ptr.insert(p, name.into()); r.type_names.insert(name.into()); }
        let value = DataType { token: 5, type_info: 100, ..Default::default() };
        for (p, name, owner) in [(1, "GetVelocity", "ACharacter"), (2, "Normalize", "FVector"), (3, "opNeg", "FVector")] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into()); r.func_is_method.insert(p);
            r.func_ret.insert(p, value.clone()); r.func_params.insert(p, Vec::new());
            r.const_method_ptrs.insert(p);
        }
        r.funcid_to_ptr.insert(101, 4); r.func_by_ptr.insert(4, "Check".into());
        let handle = DataType { token: 5, type_info: 101, is_object_handle: true, ..Default::default() };
        r.func_ret.insert(4, DataType { token: 0x41, ..Default::default() });
        r.func_params.insert(4, vec![handle.clone(), handle, DataType { is_reference: true, is_object_const: !mutable,
            is_read_only: !mutable, ..value }, DataType { token: 0x51, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_value_operands_before_return(mutable: bool) -> Self {
        let mut r = Self::default();
        for (p, name) in [(100, "FVector"), (101, "FTransform")] { r.type_by_ptr.insert(p, name.into()); r.type_names.insert(name.into()); }
        let value = DataType { token: 5, type_info: 100, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: !mutable, is_read_only: !mutable, ..value.clone() };
        for (p, name, owner) in [(1, "GetLocation", "FTransform"), (2, "opAdd", "FVector"), (3, "opDiv", "FVector"), (4, "$beh0", "FVector")] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into());
            r.func_is_method.insert(p); r.const_method_ptrs.insert(p); r.func_ret.insert(p, value.clone());
        }
        r.func_params.insert(1, Vec::new()); r.func_params.insert(2, vec![reference.clone()]);
        r.func_params.insert(3, vec![DataType { token: 0x51, ..Default::default() }]);
        r.func_params.insert(4, vec![reference]); r.func_ret.insert(4, DataType { token: 0x52, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reused_guard_cast_getter(fault: u8) -> Self {
        let names = [("UMagic", "Target"), ("ABridge", ""), ("USpell", "Bridge"), ("UDelegate", ""),
            ("FTag", ""), ("FName", ""), ("AActor", ""), ("UObject", "")];
        let mut r = Self::from_test_member_chain(&names);
        for (index, (name, _)) in names.iter().enumerate() {
            let id = index as i64 + 1;
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: (*name).into(),
                module: if [2, 3].contains(&id) { "Fixture".into() } else { String::new() },
                namespace: if fault == 4 && id == 7 { "Other".into() } else { String::new() } });
            if matches!(id, 1 | 3) { r.prop_type_id.insert((id << 1) | 1, id as i32); }
        }
        let key = (3i64 << 1) | (8i64 << 33) | 1;
        r.prop_by_key.insert(key, "Handle".into()); r.prop_type_id.insert(key, 3);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("UMagic", "Target", if fault == 1 { "UObject" } else { "AActor" })], &[], None));
        r.set_class_fields(HashMap::from([("USpell".into(), HashMap::from([
            ("Bridge".into(), "ABridge".into()), ("Handle".into(), "UDelegate".into())]))]));
        let handle = |ty| DataType { token: 5, type_info: ty, is_object_handle: true, ..Default::default() };
        let value = |ty| DataType { token: 5, type_info: ty, ..Default::default() };
        for (ptr, name) in [(1, "Interrupt"), (2, "Target"), (3, "opCast"), (4, "__STATIC_NAME"), (5, "Avatar"), (6, "Attach")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_params.insert(ptr, Vec::new());
            r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        for ptr in [1, 2, 3, 5] { r.func_is_method.insert(ptr); }
        for ptr in [2, 5] { r.func_ret.insert(ptr, handle(7)); }
        if fault == 2 { r.func_ret.get_mut(&5).unwrap().is_reference = true; }
        r.func_ret.insert(4, DataType { is_reference: true, is_object_const: true, ..value(6) });
        r.func_params.insert(4, vec![DataType { token: 0x44, ..Default::default() }]);
        r.func_ret.insert(6, handle(4)); r.func_params.insert(6, vec![handle(if fault == 3 { 8 } else { 7 }), value(5), handle(8), value(6)]);
        r.temporary_arg_positions.insert("Attach".into(), HashMap::from([(4, vec![true, false, true, false])]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reused_null_guard_getters(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UDebugOwner", "Controlled"), ("UComponent", ""),
            ("UAbilityBase", "TargetEffect"), ("TSubclassOf", ""), ("UEffect", "")]);
        for (id, name) in [(1, "UDebugOwner"), (2, "UComponent"), (3, "UAbilityBase"), (4, "TSubclassOf"), (5, "UEffect")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if id == 1 { "Script".into() } else { String::new() } });
        }
        r.prop_type_id.insert(3, if fault == 1 { 3 } else { 1 }); r.prop_type_id.insert(7, 3);
        let key = (3i64 << 1) | (8i64 << 33) | 1;
        r.prop_by_key.insert(key, "CasterEffect".into()); r.prop_type_id.insert(key, 3);
        r.type_subtypes.insert(4, vec![DataType { token: 5, type_info: 5, ..Default::default() }]);
        r.set_class_fields(HashMap::from([("UDebugOwner".into(), HashMap::from([("Controlled".into(),
            if fault == 2 { "UOther".into() } else { "UComponent".into() })]))]));
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("UAbilityBase", "TargetEffect", "TSubclassOf<UEffect>"), ("UAbilityBase", "CasterEffect", "TSubclassOf<UEffect>")], &[], None));
        for (ptr, name, owner) in [(10, "GetComponent", "UAbilityBase"), (11, "Remove", "UComponent")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            if !(fault == 3 && ptr == 10) { r.func_is_method.insert(ptr); }
        }
        let handle = DataType { token: 5, type_info: 2, is_object_handle: true, ..Default::default() };
        r.func_ret.insert(10, DataType { is_object_const: fault == 4, ..handle.clone() });
        r.func_params.insert(10, Vec::new());
        r.func_ret.insert(11, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(11, vec![DataType { token: 5, type_info: 4, ..Default::default() },
            DataType { type_info: if fault == 5 { 5 } else { 2 }, ..handle }, DataType { token: 0x44, ..Default::default() }]);
        // RefResolver::build derives this separate gate from every parameter
        // row; direct fixture insertion must supply the same derived metadata.
        let params = &r.func_params[&11];
        let accepts = params.iter().map(|p| !p.is_reference || p.is_object_const || p.is_read_only).collect();
        r.temporary_arg_positions.insert("Remove".into(), HashMap::from([(params.len(), accepts)]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_member_result_before_arguments(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FPoint", "Z"), ("FPoint", "Z")]);
        for id in [1, 2] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: "FPoint".into(), namespace: String::new(),
                module: if id == 2 || fault == 1 { "Other".into() } else { String::new() } });
        }
        r.prop_type_id.insert(3, if fault == 2 { 2 } else { 1 });
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("FPoint", "Z", if fault == 3 { "double" } else { "float" })], &[], None));
        r.funcid_to_ptr.insert(10, 10); r.func_by_ptr.insert(10, "Floor".into());
        r.func_owner.insert(10, "AOwner".into());
        if fault != 4 { r.func_is_method.insert(10); }
        r.func_ret.insert(10, DataType { token: 5, type_info: 1,
            is_reference: fault == 5, is_object_handle: fault == 6, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_range_widening(fault: u8) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(10, if fault == 1 { "Other" } else { "RandRange" }.into());
        r.func_ns.insert(10, if fault == 2 { "Other" } else { "Math" }.into());
        if fault == 3 { r.func_is_method.insert(10); }
        if fault == 4 { r.func_owner.insert(10, "FMath".into()); }
        r.func_ret.insert(10, DataType { token: if fault == 5 { 0x50 } else { 0x51 },
            is_reference: fault == 6, ..Default::default() });
        r.func_params.insert(10, (0..if fault == 7 { 3 } else { 2 }).map(|_| DataType {
            token: if fault == 8 { 0x50 } else { 0x51 }, is_reference: fault == 9,
            is_object_handle: fault == 10, ..Default::default() }).collect());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_scalar_predicate_widening(fault: u8) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(10, "Near".into());
        if fault == 1 { r.func_is_method.insert(10); }
        if fault == 2 { r.func_owner.insert(10, "FMath".into()); }
        r.func_ret.insert(10, DataType { token: if fault == 3 { 0x44 } else { 0x41 }, ..Default::default() });
        r.func_params.insert(10, (0..if fault == 4 { 2 } else { 3 }).map(|_| DataType {
            token: if fault == 5 { 0x50 } else { 0x51 }, is_reference: fault == 6, ..Default::default() }).collect());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_copy_before_value_chain(mutable: bool) -> Self {
        let mut r = Self::default(); r.type_by_ptr.insert(100, "FVector".into()); r.type_names.insert("FVector".into());
        let value = DataType { token: 5, type_info: 100, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: !mutable, is_read_only: !mutable, ..value.clone() };
        for (p, name, owner) in [(1, "$beh0", "FVector"), (2, "Location", "AActor"), (3, "opSub", "FVector"), (4, "Normalize", "FVector")] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into()); r.func_is_method.insert(p);
            r.const_method_ptrs.insert(p); r.func_ret.insert(p, value.clone());
        }
        r.func_ret.insert(1, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(1, vec![reference.clone()]); r.func_params.insert(2, Vec::new());
        r.func_params.insert(3, vec![reference.clone()]);
        r.func_params.insert(4, vec![DataType { token: 0x51, ..Default::default() }, reference]); r
    }

    #[cfg(test)]
    pub(crate) fn from_test_constructed_return_value(wrong_owner: bool) -> Self {
        let mut r = Self::from_test_script_constructors(&[""], "FPayload",
            &[DataType { token: 0x50, ..Default::default() }]);
        r.typeid_to_ptr.insert(101, 101);
        r.funcid_to_ptr.insert(2, 2); r.func_by_ptr.insert(2, "FPayload".into());
        r.func_owner.insert(2, "FPayload".into()); r.func_is_method.insert(2);
        r.script_ctor_owner.insert(2, if wrong_owner { 102 } else { 101 });
        r.func_params.insert(2, Vec::new()); r.func_ret.insert(2, DataType { token: 0x52, ..Default::default() }); r
    }

    #[cfg(test)]
    pub(crate) fn from_test_boxed_enum_result(wrong_identity: bool) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(100, "EStage".into()); r.type_names.insert("EStage".into());
        r.type_identity_by_ptr.insert(100, TypeIdentity { name: "EStage".into(), module: "State".into(), namespace: String::new() });
        r.typeid_to_ptr.insert(100, if wrong_identity { 101 } else { 100 });
        r.funcid_to_ptr.insert(1, 1); r.func_by_ptr.insert(1, "Stage".into()); r.func_is_method.insert(1);
        r.func_params.insert(1, Vec::new());
        r.func_ret.insert(1, DataType { token: 5, type_info: 100, ..Default::default() }); r
    }

    #[cfg(test)]
    pub(crate) fn from_test_changed_bool_fields() -> Self {
        let mut r = Self::default();
        for (id,module) in [(1,"Fixture"),(2,"Other")] {
            r.typeid_to_ptr.insert(id,id as i64); r.type_by_ptr.insert(id as i64,"UWatcher".into());
            r.type_identity_by_ptr.insert(id as i64,TypeIdentity { name:"UWatcher".into(), module:module.into(),namespace:String::new() });
            for (off,name) in ["A","LastA","B","LastB","C","LastC","Settled"].iter().enumerate() {
                r.prop_by_key.insert(((id as i64)<<1)|((off as i64)<<33)|1,(*name).into());
            }
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_global_copy_before_handle(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "FGameplayTag"), (2, "FPerceivedInteractiveObject"), (3, "AState"), (4, "UObject")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let void = DataType { token: 0x52, ..Default::default() };
        let tag = DataType { token: 5, type_info: 1, is_reference: true, is_object_const: true, is_read_only: true, ..Default::default() };
        let handle = DataType { token: 5, type_info: 3, is_object_handle: true, ..Default::default() };
        for (ptr, name, owner) in [(10, "$beh0", "FGameplayTag"), (20, "$beh2", "FGameplayTag"), (30, "GetState", "FAgent"), (40, "Register", "UCrime")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            r.func_ret.insert(ptr, void.clone());
        }
        r.func_params.insert(10, vec![tag.clone()]); r.func_params.insert(20, vec![]);
        r.const_method_ptrs.insert(30); r.func_ret.insert(30, handle.clone());
        r.func_params.insert(30, vec![DataType { type_info: 4, is_object_const: true, ..handle.clone() }]);
        r.funcid_to_ptr.insert(40, 40);
        r.func_params.insert(40, vec![tag.clone(), handle.clone(), handle, tag.clone(), DataType { type_info: 2, ..tag }]);
        r.global_by_ptr.insert(50, "Crime_Interaction".into()); r.global_ns.insert(50, "GameplayTag".into());
        r.temporary_arg_positions.insert("Register".into(), HashMap::from([(5, vec![true; 5])]));
        match fault {
            1 => r.func_params.get_mut(&10).unwrap()[0].is_read_only = false,
            2 => { r.func_owner.insert(20, "FOther".into()); }
            3 => r.func_ret.get_mut(&30).unwrap().is_reference = true,
            4 => r.func_params.get_mut(&40).unwrap()[0].type_info = 2,
            5 => r.func_params.get_mut(&40).unwrap()[1].type_info = 4,
            6 => { r.const_method_ptrs.remove(&30); }
            7 => r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(),
            8 => { r.global_by_ptr.remove(&50); }
            9 => r.func_params.get_mut(&20).unwrap().push(void),
            10 => { r.func_is_method.remove(&40); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_assigned_value_before_getter(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(1, "FVector".into()); r.type_names.insert("FVector".into());
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "FVector".into(), module: String::new(), namespace: String::new() });
        let value = DataType { token: 5, type_info: 1, ..Default::default() };
        let input = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value.clone() };
        let scalar = DataType { token: 0x51, ..Default::default() };
        for (ptr, name, owner, constant) in [(1, "$beh0", "FVector", false), (2, "opAssign", "FVector", false),
            (3, "GetLocation", "AActor", true), (4, "opAdd", "FVector", true), (5, "opSub", "FVector", true),
            (6, "Normalize", "FVector", false), (7, "Sink", "USceneComponent", false), (8, "$beh0", "FVector", false)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            if constant { r.const_method_ptrs.insert(ptr); }
        }
        r.func_ret.insert(1, DataType { token: 0x52, ..Default::default() }); r.func_params.insert(1, vec![scalar.clone(); 3]);
        r.func_ret.insert(2, DataType { is_reference: true, ..value.clone() }); r.func_params.insert(2, vec![input.clone()]);
        r.func_ret.insert(3, value.clone()); r.func_params.insert(3, Vec::new());
        for ptr in [4, 5] { r.func_ret.insert(ptr, value.clone()); r.func_params.insert(ptr, vec![input.clone()]); }
        r.func_ret.insert(6, DataType { token: 0x41, ..Default::default() }); r.func_params.insert(6, vec![scalar]);
        r.func_ret.insert(7, DataType { token: 0x52, ..Default::default() }); r.func_params.insert(7, vec![input.clone()]);
        r.func_ret.insert(8, DataType { token: 0x52, ..Default::default() }); r.func_params.insert(8, vec![input]);
        r.temporary_arg_positions.insert("Sink".into(), HashMap::from([(1, vec![true])]));
        r.ctor_arg_positions.insert("FVector".into(), HashMap::from([(3, vec![true; 3])]));
        match fault {
            1 => r.func_params.get_mut(&1).unwrap()[1].token = 0x50,
            2 => r.func_params.get_mut(&2).unwrap()[0].is_read_only = false,
            3 => r.func_ret.get_mut(&2).unwrap().is_reference = false,
            4 => r.func_params.get_mut(&4).unwrap()[0].type_info = 2,
            5 => r.func_ret.get_mut(&3).unwrap().is_reference = true,
            6 => { r.func_params.get_mut(&3).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
            7 => { r.const_method_ptrs.remove(&4); }
            8 => { r.func_owner.insert(1, "OtherVector".into()); }
            9 => r.type_identity_by_ptr.get_mut(&1).unwrap().namespace = "Other".into(),
            10 => r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(),
            11 => r.func_ret.get_mut(&1).unwrap().token = 0x41,
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_vector_self_assignment(fault: u8) -> Self {
        let mut r = Self::from_test_assigned_value_before_getter(0);
        r.func_by_ptr.insert(9, "opMul".into()); r.func_owner.insert(9, "FVector".into());
        r.func_is_method.insert(9); r.const_method_ptrs.insert(9);
        r.func_ret.insert(9, DataType { token: 5, type_info: 1, ..Default::default() });
        r.func_params.insert(9, vec![DataType { token: 0x51, ..Default::default() }]);
        r.func_by_ptr.insert(10, "ScaleFactor".into());
        r.func_ret.insert(10, DataType { token: 0x51, ..Default::default() }); r.func_params.insert(10, vec![]);
        match fault {
            1 => r.func_params.get_mut(&9).unwrap()[0].token = 0x50,
            2 => r.func_params.get_mut(&2).unwrap()[0].is_read_only = false,
            3 => r.func_ret.get_mut(&2).unwrap().is_reference = false,
            4 => r.func_params.get_mut(&4).unwrap()[0].type_info = 2,
            5 => r.func_ret.get_mut(&5).unwrap().is_reference = true,
            6 => { r.const_method_ptrs.remove(&9); }
            7 => { r.const_method_ptrs.insert(2); }
            8 => { r.func_owner.insert(4, "FOther".into()); }
            9 => r.type_identity_by_ptr.get_mut(&1).unwrap().namespace = "Other".into(),
            10 => r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(),
            11 => { r.func_by_ptr.insert(9, "opMulAssign".into()); }
            12 => r.func_ret.get_mut(&2).unwrap().is_object_const = true,
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_context_receiver_before_nested_rvo(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "FName"), (2, "FInteractionSpotHandle"), (3, "FTransform"), (4, "FVector"), (5, "UObject")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let value = |ptr| DataType { token: 5, type_info: ptr, ..Default::default() };
        let void = DataType { token: 0x52, ..Default::default() };
        for (ptr, name, owner, constant, ret, args) in [
            (10, "$beh0", "FInteractionSpotHandle", false, void.clone(), vec![value(1)]),
            (11, "GetTransform", "FInteractionSpotHandle", true, value(3),
                vec![DataType { is_object_handle: true, is_object_const: true, ..value(5) }]),
            (12, "GetLocation", "FTransform", true, value(4), vec![]),
            (13, "$beh2", "FInteractionSpotHandle", false, void, vec![])] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            if constant { r.const_method_ptrs.insert(ptr); }
            r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, args);
        }
        r.global_by_ptr.insert(100, "__WorldContext".into());
        r.ctor_arg_positions.insert("FInteractionSpotHandle".into(), HashMap::from([(1, vec![true])]));
        match fault {
            1 => r.func_params.get_mut(&10).unwrap()[0].type_info = 2,
            2 => r.func_params.get_mut(&10).unwrap()[0].is_reference = true,
            3 => r.func_ret.get_mut(&11).unwrap().is_reference = true,
            4 => { r.const_method_ptrs.remove(&11); }
            5 => { r.func_owner.insert(12, "FOther".into()); }
            6 => r.func_ret.get_mut(&12).unwrap().type_info = 3,
            7 => { r.global_by_ptr.insert(100, "OtherGlobal".into()); }
            8 => r.func_params.get_mut(&11).unwrap()[0].is_object_const = false,
            9 => r.type_identity_by_ptr.get_mut(&2).unwrap().module = "Script".into(),
            10 => { r.func_owner.insert(13, "FOther".into()); }
            11 => { r.const_method_ptrs.insert(13); }
            12 => r.func_ret.get_mut(&10).unwrap().token = 0x41,
            13 => { r.func_is_method.remove(&12); }
            14 => r.func_params.get_mut(&12).unwrap().push(value(1)),
            15 => r.type_identity_by_ptr.get_mut(&5).unwrap().namespace = "Other".into(),
            16 => { r.const_method_ptrs.insert(10); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_copied_index_argument(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name, module) in [(1, "TSubclassOf", "GAS.Skills"), (2, "TArray", "GAS.Skills"), (3, "FContext", ""),
            (4, "FResult", ""), (5, "UGothicAbilityComponent", ""), (6, "USkill", "GAS.Skills"),
            (7, "TSubclassOf", ""), (8, "UGameplayEffect", "")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: module.into(), namespace: String::new() });
        }
        let value = |ptr| DataType { token: 5, type_info: ptr, ..Default::default() };
        r.type_subtypes.insert(2, vec![value(1)]);
        r.type_subtypes.insert(1, vec![DataType { is_object_handle: true, ..value(6) }]);
        r.type_subtypes.insert(7, vec![DataType { is_object_handle: true, ..value(8) }]);
        for (ptr, name, owner, constant) in [(10, "MakeContext", "UAbilityComponent", true), (11, "$beh0", "TSubclassOf", false),
            (12, "opIndex", "TArray", false), (13, "Apply", "UAbilityComponent", false),
            (14, "$beh2", "FContext", false), (15, "$beh2", "FResult", false)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            if constant { r.const_method_ptrs.insert(ptr); }
        }
        r.func_params.insert(10, Vec::new()); r.func_ret.insert(10, value(3));
        r.func_params.insert(11, vec![DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value(1) }]);
        r.func_ret.insert(11, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(12, vec![DataType { token: 0x44, ..Default::default() }]);
        r.func_ret.insert(12, DataType { is_reference: true, ..value(1) });
        r.func_params.insert(13, vec![value(7), DataType { token: 0x50, ..Default::default() }, value(3)]);
        r.func_ret.insert(13, value(4));
        for ptr in [14, 15] { r.func_params.insert(ptr, Vec::new()); r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() }); }
        r.temporary_arg_positions.insert("Apply".into(), HashMap::from([(3, vec![true; 3])]));
        r.ctor_arg_positions.insert("TSubclassOf".into(), HashMap::from([(1, vec![true])]));
        match fault {
            1 => r.type_subtypes.get_mut(&2).unwrap()[0].type_info = 7,
            2 => r.func_ret.get_mut(&12).unwrap().type_info = 7,
            3 => r.func_params.get_mut(&11).unwrap()[0].is_object_const = false,
            4 => r.func_ret.get_mut(&11).unwrap().token = 0x41,
            5 => r.func_ret.get_mut(&10).unwrap().is_reference = true,
            6 => { r.func_params.get_mut(&10).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
            7 => r.func_params.get_mut(&13).unwrap()[2].type_info = 4,
            8 => r.func_params.get_mut(&13).unwrap()[1].token = 0x51,
            9 => { r.func_owner.insert(13, "OtherAbility".into()); }
            10 => r.func_params.get_mut(&12).unwrap()[0].is_reference = true,
            11 => { r.const_method_ptrs.insert(12); }
            12 => r.func_ret.get_mut(&13).unwrap().is_object_handle = true,
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_ordered_vector_arguments(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FVector", ""), ("ECollisionChannel", ""), ("AGothicCharacter", "")]);
        for (id, name) in [(1, "FVector"), (2, "ECollisionChannel"), (3, "AGothicCharacter")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if id == 1 && fault == 1 { "Other" } else { "" }.into() });
        }
        for (id, name, owner) in [(10, "GetLocation", "AActor"), (20, "Floor", "AHost"), (30, "Contains", "AHost")] {
            r.funcid_to_ptr.insert(id, id as i64); r.func_by_ptr.insert(id as i64, name.into());
            r.func_owner.insert(id as i64, owner.into()); r.func_is_method.insert(id as i64);
        }
        if fault != 2 { r.const_method_ptrs.insert(10); }
        let value = DataType { token: 5, type_info: 1, ..Default::default() };
        let input = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value.clone() };
        let scalar = DataType { token: if fault == 3 { 0x50 } else { 0x51 }, ..Default::default() };
        r.func_ret.insert(10, value.clone()); r.func_params.insert(10, Vec::new());
        r.func_ret.insert(20, value.clone());
        r.func_params.insert(20, vec![input.clone(), DataType { token: 5, type_info: 2, ..Default::default() }, scalar.clone(), scalar]);
        r.func_ret.insert(30, DataType { token: 0x41, ..Default::default() });
        r.func_params.insert(30, vec![input.clone(), if fault == 4 { value } else { input }]);
        if fault == 5 { r.func_is_method.remove(&20); }
        if fault == 6 { r.func_params.get_mut(&10).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
        for (name, count) in [("Floor", 4), ("Contains", 2)] {
            r.temporary_arg_positions.insert(name.into(), HashMap::from([(count, vec![true; count])]));
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_ordered_vector_field_receiver(fault: u8) -> Self {
        let mut r = Self::from_test_ordered_vector_arguments(if fault <= 6 { fault } else { 0 });
        for (id, name) in [(4, "AHost"), (5, "AOtherHost")] {
            r.typeid_to_ptr.insert(id, id as i64); r.type_by_ptr.insert(id as i64, name.into());
            r.type_identity_by_ptr.insert(id as i64, TypeIdentity { name: name.into(),
                module: "Fixture".into(), namespace: String::new() });
        }
        let key = (4 << 1) | 1;
        r.prop_by_key.insert(key, "Target".into()); r.prop_type_id.insert(key, 4);
        r.set_class_fields(HashMap::from([("AHost".into(), HashMap::from([("Target".into(),
            if fault == 7 { "UObject" } else { "AGothicCharacter" }.into())]))]));
        r.class_super.insert("AHost".into(), "AActor".into());
        for (ptr, name) in [(40, "Hit"), (41, "Miss")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
            r.func_params.insert(ptr, vec![]);
        }
        match fault {
            8 => { r.prop_type_id.insert(key, 5); }
            9 => r.type_identity_by_ptr.get_mut(&4).unwrap().namespace = "Other".into(),
            10 => { r.duplicate_prop_keys.insert(key); }
            11 => { r.class_fields.remove("AHost");
                r.class_fields.insert("AActor".into(), HashMap::from([("Target".into(), "AGothicCharacter".into())])); }
            12 => { r.class_super.insert("AGothicCharacter".into(), "AActor".into()); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_eager_clamp_bounds(narrow: bool) -> Self {
        let mut r = Self::default();
        for (p, name, count) in [(1, "Min", 2), (2, "Max", 2), (3, "Clamp", 3)] {
            let scalar = DataType { token: if narrow && p == 2 { 0x50 } else { 0x51 }, ..Default::default() };
            r.func_by_ptr.insert(p, name.into()); r.func_params.insert(p, vec![scalar.clone(); count]); r.func_ret.insert(p, scalar);
            r.temporary_arg_positions.insert(name.into(), HashMap::from([(count, vec![true; count])]));
        }
        r.funcid_to_ptr.insert(4, 4); r.func_by_ptr.insert(4, "Other::Max".into());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_forwarded_getter_field(fault: u8) -> Self {
        let mut r = Self::default();
        for (p, name) in [(99, "UHolder"), (100, "AState"), (101, "ANpcState"), (102, "AOther")] {
            r.type_by_ptr.insert(p, name.into()); r.typeid_to_ptr.insert(p as i32, p); r.type_names.insert(name.into());
        }
        for (p, name, owner) in [(1, "GetState", "UHolder"), (2, "opCast", "UObject"), (3, "GetSource", "ANpcState")] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, if fault == 3 && p == 3 { "AOther" } else { owner }.into());
            if !(fault == 4 && p == 3) { r.func_is_method.insert(p); }
            r.func_params.insert(p, Vec::new());
            r.func_ret.insert(p, DataType { token: 5, type_info: if fault == 1 && p == 3 { 102 } else { 100 }, is_object_handle: true, ..Default::default() });
        }
        let key = (99i64 << 1) | (8i64 << 33) | 1;
        r.prop_by_key.insert(key, "Target".into()); r.prop_type_id.insert(key, 99);
        r.set_class_fields(HashMap::from([("UHolder".into(), HashMap::from([("Target".into(), if fault == 2 { "AOther" } else { "AState" }.into())]))]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_value_handle_store(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(100, "FPosition"), (101, "FPositionOther"), (102, "AArm"), (103, "AHost")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into()); r.typeid_to_ptr.insert(ptr as i32, ptr);
        }
        for (id, name) in [(1, "MakePosition"), (2, "MakeArm")] {
            r.funcid_to_ptr.insert(id as i32, id); r.func_by_ptr.insert(id, name.into());
            r.func_owner.insert(id, "AHost".into()); r.func_ns.insert(id, if fault == 7 && id == 2 { "Other" } else { "" }.into());
            if !(fault == 6 && id == 2) { r.func_is_method.insert(id); }
        }
        r.func_ret.insert(1, DataType { token: 5, type_info: 100, is_reference: fault == 1, ..Default::default() });
        r.func_ret.insert(2, DataType { token: 5, type_info: 102, is_object_handle: fault != 5, ..Default::default() });
        r.func_params.insert(1, vec![DataType { token: 5, type_info: 103, is_object_handle: fault != 2, ..Default::default() }]);
        r.func_params.insert(2, vec![DataType { token: 5, type_info: if fault == 3 { 101 } else { 100 },
            is_reference: true, is_object_const: fault != 4, ..Default::default() }]);
        r.temporary_arg_positions.insert("MakeArm".into(), HashMap::from([(1, vec![true])]));
        let key = (102i64 << 1) | (8i64 << 33) | 1;
        r.prop_by_key.insert(key, "Owner".into()); r.prop_type_id.insert(key, 102);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_completed_value_argument_order(fault: u8) -> Self {
        let mut r=Self::default();
        for (ptr,name) in [(100,"FTag"),(200,"FResult"),(300,"UProvider")] {
            r.type_by_ptr.insert(ptr,name.into());r.type_names.insert(name.into());
        }
        for (ptr,name,owner) in [(1,"MakeTag","UProvider"),(2,"GetTag","UProvider"),
            (3,"Relate","UProvider"),(4,"$beh2","FTag"),(5,"$beh2","FResult")] {
            r.func_by_ptr.insert(ptr,name.into());r.func_owner.insert(ptr,owner.into());
            r.func_is_method.insert(ptr);r.func_params.insert(ptr,Vec::new());
        }
        r.funcid_to_ptr.insert(1,1);
        for ptr in [1,2,3] {r.const_method_ptrs.insert(ptr);}
        r.func_ret.insert(1,DataType {token:5,type_info:100,is_reference:fault == 1,..Default::default()});
        r.func_ret.insert(2,DataType {token:5,type_info:if fault == 2 {200} else {100},
            is_reference:fault == 3,is_object_handle:fault == 4,..Default::default()});
        if fault == 5 {r.func_params.insert(2,vec![DataType {token:0x44,..Default::default()}]);}
        if fault == 6 {r.const_method_ptrs.remove(&2);}
        r.func_ret.insert(3,DataType {token:5,type_info:200,..Default::default()});
        r.func_params.insert(3,vec![DataType {token:5,type_info:100,is_reference:true,is_object_const:true,is_read_only:true,..Default::default()};3]);
        r.func_ret.insert(4,DataType {token:if fault == 7 {0x41} else {0x52},..Default::default()});
        r.func_ret.insert(5,DataType {token:0x52,..Default::default()});
        r.global_by_ptr.insert(6,"Global".into());r.global_ns.insert(6,"Tag".into());
        r.temporary_arg_positions.insert("Relate".into(),HashMap::from([(3,vec![true,true,true])]));
        r.set_class_methods(HashMap::from([("UProvider".into(),HashSet::from([
            "MakeTag/0/const".into(),"GetTag/0/const".into(),"Relate/3/const".into()]))]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_context_discarded_value(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(100, "FContext"), (200, "FSpec"), (300, "FResult"), (400, "UComponent")] { r.type_by_ptr.insert(ptr, name.into()); }
        for (ptr, name, owner, ret) in [(1, "MakeContext", "UComponent", 100), (2, "MakeSpec", "UComponent", 200),
            (3, "Apply", "UComponent", 300), (4, "$beh2", "FResult", 0), (5, "$beh2", "FSpec", 0),
            (6, "$beh2", "FContext", 0), (7, "Work", "", 0)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            if ptr != 7 && !(fault == 1 && ptr == 3) { r.func_is_method.insert(ptr); }
            r.func_ret.insert(ptr, DataType { token: if ret == 0 { 0x52 } else { 5 }, type_info: ret,
                is_reference: fault == 2 && ptr == 3, ..Default::default() });
            r.func_params.insert(ptr, vec![]);
        }
        let spec = DataType { token: 5, type_info: if fault == 3 { 100 } else { 200 }, is_reference: true,
            is_object_const: fault != 4, is_read_only: true, ..Default::default() };
        r.func_params.insert(3, vec![spec, DataType { token: 5, type_info: 400, is_object_handle: fault != 5, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_value_before_handle_argument(fault: u8) -> Self {
        let mut r = Self::from_test_context_discarded_value(if fault <= 5 { fault } else { 0 });
        for (ptr, name) in [(100, "FContext"), (200, "FSpec"), (300, "FResult"), (400, "UComponent"), (500, "ATarget")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if fault == 9 && ptr == 200 { "Foreign" } else { "" }.into() });
        }
        r.func_by_ptr.insert(8, "GetComponent".into()); r.func_owner.insert(8, "ATarget".into());
        r.func_is_method.insert(8); r.const_method_ptrs.extend([2, 8]);
        r.func_params.insert(8, Vec::new());
        r.func_ret.insert(8, DataType { token: 5, type_info: 400, is_object_handle: fault != 6, ..Default::default() });
        if fault == 7 { r.const_method_ptrs.remove(&8); }
        if fault == 8 { r.func_owner.insert(4, "FContext".into()); }
        if fault == 10 { r.func_ret.get_mut(&2).unwrap().is_reference = true; }
        if fault == 11 { r.func_params.get_mut(&5).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
        r.temporary_arg_positions.insert("Apply".into(), HashMap::from([(2, vec![true, true])]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_retained_after_value(token: i32, reference: bool, name: &str) -> Self {
        let mut r = Self::from_test_retained_receiver(5, true);
        r.func_by_ptr.insert(4, name.into());
        r.func_ret.insert(4, DataType { token, is_reference: reference, ..Default::default() });
        r.func_by_ptr.insert(5, "Apply".into());
        r.func_ret.insert(5, DataType { token: 5, ..Default::default() });
        r.func_is_method.insert(5);
        r.func_params.insert(5, vec![DataType { token: 5, is_reference: true, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_returning_bool_switch(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UOwner","Config"),("FConfig","Active")]);
        for (id,name) in [(1,"UOwner"),(2,"FConfig")] {
            r.type_identity_by_ptr.insert(id,TypeIdentity { name:name.into(),
                module: if fault == 3 && id == 2 { "Other" } else { "Fixture" }.into(),
                namespace: if fault == 4 { "Other" } else { "" }.into() });
        }
        for (id,offset,name) in [(1,0,"Config"),(2,0,"Active"),(2,1,"Passive")] {
            let key = (id << 1) | (offset << 33) | 1;
            r.prop_by_key.insert(key,name.into());
            r.prop_type_id.insert(key,if fault == 2 { 1 } else { id as i32 });
        }
        r.class_fields.insert("UOwner".into(),HashMap::from([("Config".into(),
            if fault == 5 { "OtherConfig" } else { "FConfig" }.into())]));
        r.class_fields.insert("FConfig".into(),HashMap::from([("Active".into(),
            if fault == 1 { "int8" } else { "bool" }.into()),("Passive".into(),"bool".into())]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_widened_final_product(token: i32, reference: bool) -> Self {
        let mut r=Self::default();
        r.funcid_to_ptr.insert(1,10); r.func_by_ptr.insert(10,"Value".into());
        r.func_ret.insert(10,DataType { token,is_reference:reference,..Default::default() });
        r.func_params.insert(10,Vec::new());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_negated_narrow_argument(fault: u8) -> Self {
        let mut r=Self::from_test_copied_int_field_read("TSubclassOf<UEffect>",false);
        r.type_by_ptr.insert(2,"TSubclassOf".into());
        r.type_identity_by_ptr.insert(2,TypeIdentity { name:"TSubclassOf".into(),module:String::new(),namespace:String::new() });
        r.funcid_to_ptr.insert(101,101);
        for (ptr,name,token) in [(101,"GetCost",0x51),(202,"Apply",0x52)] {
            r.func_by_ptr.insert(ptr,name.into()); r.func_owner.insert(ptr,"UConfig".into()); r.func_is_method.insert(ptr);
            r.func_ret.insert(ptr,DataType { token,..Default::default() }); r.func_params.insert(ptr,Vec::new());
        }
        r.func_params.insert(202,vec![DataType { token:0x50,..Default::default() },DataType { token:0x50,..Default::default() },
            DataType { token:5,type_info:2,..Default::default() }]);
        if fault==1 { r.func_ret.get_mut(&101).unwrap().token=0x50; }
        if fault==2 { r.func_ret.get_mut(&101).unwrap().is_reference=true; }
        if fault==3 { r.func_params.get_mut(&101).unwrap().push(DataType::default()); }
        if fault==4 { r.func_is_method.remove(&101); }
        if fault==5 { r.func_owner.insert(101,"UOther".into()); }
        if fault==6 { r.func_params.get_mut(&202).unwrap()[0].token=0x51; }
        if fault==7 { r.func_params.get_mut(&202).unwrap()[1].is_reference=true; }
        if fault==8 { r.func_params.get_mut(&202).unwrap()[2].is_object_handle=true; }
        if fault==9 { r.type_identity_by_ptr.get_mut(&2).unwrap().module="Script".into(); }
        if fault==10 { r.func_owner.insert(202,"FValue".into()); }
        if fault==11 { r.func_ret.get_mut(&202).unwrap().token=0x41; }
        if fault==12 { r.type_identity_by_ptr.get_mut(&1).unwrap().module.clear(); }
        if fault==13 { r.prop_type_id.insert(3,2); }
        if fault==14 { r.class_fields.get_mut("UConfig").unwrap().insert("Limit".into(),"TArray<UEffect>".into()); }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_copied_int_field_read(ty: &str, native: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("UConfig", "Limit")]);
        let owner = TypeIdentity { name: "UConfig".into(),
            module: if native { "" } else { "Fixture" }.into(), namespace: String::new() };
        r.type_identity_by_ptr.insert(1, owner.clone());
        r.prop_type_id.insert(3, 1);
        r.set_class_fields(HashMap::from([("UConfig".into(), HashMap::from([("Limit".into(), ty.into())]))]));
        let token = match ty { "int" => 0x44, "uint" => 0x4B, "int64" => 0x47,
            "float32" => 0x50, "bool" => 0x41, _ => 5 };
        r.set_qualified_fields([(owner, "Limit".into(), DataType { token, ..Default::default() })]);
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_scalar_argument_order(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FVector", "X"), ("URegion", "Minimum"), ("FOther", "")]);
        for (id, name, module) in [(1, "FVector", ""), (2, "URegion", "Fixture"), (3, "FOther", "")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: module.into(), namespace: String::new() });
        }
        for (id, offset, name) in [(1, 0, "X"), (1, 8, "Y"), (1, 16, "Z"),
            (2, 0, "Minimum"), (2, 8, "Radius"), (2, 16, "Maximum")] {
            let key = (id << 1) | ((offset as i64) << 33) | 1;
            r.prop_by_key.insert(key, name.into()); r.prop_type_id.insert(key, id as i32);
        }
        r.set_class_fields(HashMap::from([("URegion".into(), HashMap::from([
            ("Minimum".into(), "float".into()), ("Radius".into(), "float".into()), ("Maximum".into(), "float".into())]))]));
        let vector = DataType { token: 5, type_info: 1, ..Default::default() };
        r.func_by_ptr.insert(1, "GetLocation".into()); r.func_owner.insert(1, "AActor".into());
        r.func_is_method.insert(1); r.const_method_ptrs.insert(1);
        r.func_ret.insert(1, vector); r.func_params.insert(1, Vec::new());
        r.func_by_ptr.insert(2, "$beh0".into()); r.func_owner.insert(2, "FVector".into()); r.func_is_method.insert(2);
        r.func_ret.insert(2, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(2, vec![DataType { token: 0x51, ..Default::default() }; 3]);
        r.ctor_arg_positions.insert("FVector".into(), HashMap::from([(3, vec![true; 3])]));
        match fault {
            1 => { r.prop_by_key.insert(3 | (16i64 << 33), "Unknown".into()); },
            2 => { r.prop_type_id.insert(3 | (16i64 << 33), 3); },
            3 => { r.func_ret.get_mut(&1).unwrap().is_reference = true; },
            4 => { r.func_ret.get_mut(&1).unwrap().is_object_handle = true; },
            5 => { r.func_params.get_mut(&1).unwrap().push(DataType { token: 0x51, ..Default::default() }); },
            6 => { r.func_is_method.remove(&1); },
            7 => { r.const_method_ptrs.remove(&1); },
            8 => { r.set_class_fields(HashMap::from([("URegion".into(), HashMap::from([
                ("Minimum".into(), "float32".into()), ("Radius".into(), "float".into()), ("Maximum".into(), "float".into())]))])); },
            9 => { r.prop_type_id.insert(5 | (8i64 << 33), 1); },
            10 => { r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Foreign".into(); },
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_copied_binary_receiver(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FVector", ""), ("UStorm", "Height")]);
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "FVector".into(), module: String::new(), namespace: String::new() });
        r.type_identity_by_ptr.insert(2, TypeIdentity { name: "UStorm".into(), module: "Fixture".into(), namespace: String::new() });
        r.set_class_fields(HashMap::from([("UStorm".into(), HashMap::from([("Height".into(), "float".into())]))]));
        r.prop_type_id.insert(5, 2);
        let vector = DataType { token: 5, type_info: 1, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..vector.clone() };
        for (p, name, ret, params) in [
            (10, "$beh0", DataType { token: 0x52, ..Default::default() }, vec![reference.clone()]),
            (20, "opMul", vector.clone(), vec![DataType { token: 0x51, ..Default::default() }]),
            (30, "opAdd", vector, vec![reference]),
        ] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, "FVector".into()); r.func_is_method.insert(p);
            r.func_ret.insert(p, ret); r.func_params.insert(p, params);
        }
        r.const_method_ptrs.extend([20, 30]);
        r.global_by_ptr.insert(40, "FVector::UpVector".into());
        if fault == 1 { r.func_params.get_mut(&10).unwrap()[0].is_reference = false; }
        if fault == 2 { r.const_method_ptrs.remove(&20); }
        if fault == 3 { r.func_ret.get_mut(&30).unwrap().is_reference = true; }
        if fault == 4 { r.func_params.get_mut(&20).unwrap()[0].token = 0x50; }
        if fault == 5 { r.func_owner.insert(30, "FOther".into()); }
        if fault == 6 { r.set_class_fields(HashMap::from([("UStorm".into(), HashMap::from([("Height".into(), "float32".into())]))])); }
        if fault == 7 { r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_assigned_value_return(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("FVector",""),("TArray","")]);
        r.type_identity_by_ptr.insert(1,TypeIdentity { name:"FVector".into(),module:String::new(),namespace:String::new() });
        let vector=DataType { token:5,type_info:1,..Default::default() };
        let reference=DataType { is_reference:true,is_object_const:true,is_read_only:true,..vector.clone() };
        for (p,name,owner,ret,params) in [
            (10,"opDiv","FVector",vector.clone(),vec![DataType {token:0x51,..Default::default()}]),
            (20,"opAssign","FVector",DataType {is_reference:true,..vector.clone()},vec![reference.clone()]),
            (30,"$beh0","FVector",DataType {token:0x52,..Default::default()},vec![reference]),
            (40,"$beh2","TArray",DataType {token:0x52,..Default::default()},vec![]),
            (50,"opAddAssign","FVector",vector.clone(),vec![DataType {is_reference:true,is_object_const:true,is_read_only:true,..vector.clone()}])] {
            r.func_by_ptr.insert(p,name.into());r.func_owner.insert(p,owner.into());r.func_is_method.insert(p);
            r.func_ret.insert(p,ret);r.func_params.insert(p,params);
        }
        r.const_method_ptrs.insert(10);
        if fault==1 {r.func_owner.insert(20,"FOther".into());}
        if fault==2 {r.func_params.get_mut(&20).unwrap()[0].is_reference=false;}
        if fault==3 {r.func_ret.get_mut(&10).unwrap().is_reference=true;}
        if fault==4 {r.func_owner.insert(40,"FOther".into());}
        if fault==5 {r.const_method_ptrs.remove(&10);}
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_narrowed_difference(fault:u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("UEvent","Duration"),("FClock",""),("UObject","")]);
        r.type_identity_by_ptr.insert(1,TypeIdentity{name:"UEvent".into(),module:"Fixture".into(),namespace:String::new()});
        r.prop_by_key.insert(3|(8i64<<33),"Start".into());r.prop_type_id.insert(3,1);r.prop_type_id.insert(3|(8i64<<33),1);
        r.set_class_fields(HashMap::from([("UEvent".into(),HashMap::from([
            ("Duration".into(),if fault==1 {"float32"} else {"float"}.into()),("Start".into(),"FClock".into())]))]));
        r.global_by_ptr.insert(20,if fault==2 {"Other"} else {"__WorldContext"}.into());
        r.func_by_ptr.insert(10,"Age".into());r.func_owner.insert(10,if fault==3 {"OtherClock"} else {"FClock"}.into());
        r.func_is_method.insert(10);r.const_method_ptrs.insert(10);
        r.func_ret.insert(10,DataType {token:if fault==4 {0x51} else {0x50},..Default::default()});
        r.func_params.insert(10,vec![DataType {token:5,type_info:3,is_object_handle:true,..Default::default()}]);
        if fault==5 {r.const_method_ptrs.remove(&10);}
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_index_fields(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("FRole","Kind"),("EKind",""),("TMap","")]);
        r.type_identity_by_ptr.insert(1,TypeIdentity {name:"FRole".into(),module:"Fixture".into(),namespace:String::new()});
        r.prop_by_key.insert(3 | (4i64<<33),"Index".into());
        r.prop_type_id.insert(3,1);r.prop_type_id.insert(3 | (4i64<<33),if fault==2 {2} else {1});
        r.set_class_fields(HashMap::from([("FRole".into(),HashMap::from([
            ("Kind".into(),"EKind".into()),("Index".into(),if fault==1 {"float"} else {"int"}.into())]))]));
        r.func_by_ptr.insert(10,"opIndex".into());r.func_is_method.insert(10);
        r.func_params.insert(10,vec![DataType {token:5,type_info:2,is_reference:fault!=3,
            is_object_const:true,is_read_only:true,..Default::default()}]);
        r.func_ret.insert(10,DataType {token:5,type_info:3,is_reference:true,..Default::default()});
        r.temporary_arg_positions.insert("Read".into(),HashMap::from([(2,vec![true,true])]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_value_receiver_before_argument(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("FVector",""),("AActor","")]);
        r.type_identity_by_ptr.insert(1,TypeIdentity {name:"FVector".into(),module:String::new(),namespace:String::new()});
        r.type_identity_by_ptr.insert(2,TypeIdentity {name:"AActor".into(),module:String::new(),namespace:String::new()});
        let value=DataType {token:5,type_info:1,..Default::default()};
        let reference=DataType {is_reference:true,is_object_const:true,is_read_only:true,..value.clone()};
        let double=DataType {token:0x51,..Default::default()};
        for (ptr,name,owner,params,ret) in [(10,"GetActorLocation","AActor",vec![],value.clone()),
            (20,"opSub","FVector",vec![reference.clone()],value.clone()),
            (30,"GetSafeNormal2D","FVector",vec![double.clone(),reference.clone()],value.clone()),
            (40,"DotProduct","FVector",vec![reference],double)] {
            r.func_by_ptr.insert(ptr,name.into());r.func_owner.insert(ptr,owner.into());
            r.func_is_method.insert(ptr);r.const_method_ptrs.insert(ptr);
            r.func_params.insert(ptr,params);r.func_ret.insert(ptr,ret);
        }
        r.set_class_methods(HashMap::from([("FVector".into(),HashSet::from([
            "opSub/1/const".into(),"GetSafeNormal2D/2/const".into(),"DotProduct/1/const".into()])),
            ("AActor".into(),HashSet::from(["GetActorLocation/0/const".into()]))]));
        r.temporary_arg_positions.insert("GetSafeNormal2D".into(),HashMap::from([(2,vec![false,true])]));
        for name in ["opSub","DotProduct"] {r.temporary_arg_positions.insert(name.into(),HashMap::from([(1,vec![true])]));}
        if fault==1 {r.func_params.get_mut(&30).unwrap()[0].token=0x50;}
        if fault==2 {r.func_params.get_mut(&30).unwrap()[1].is_read_only=false;}
        if fault==3 {r.func_ret.get_mut(&30).unwrap().is_reference=true;}
        if fault==4 {r.func_params.get_mut(&40).unwrap()[0].type_info=2;}
        if fault==5 {r.func_owner.insert(40,"OtherVector".into());}
        if fault==6 {r.const_method_ptrs.remove(&20);}
        if fault==7 {r.func_ret.get_mut(&40).unwrap().token=0x50;}
        if fault==8 {r.type_identity_by_ptr.get_mut(&1).unwrap().module="Other".into();}
        if fault==9 {r.func_ret.get_mut(&30).unwrap().is_object_handle=true;}
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_value_before_return_construction(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("FVector",""),("FExecutor",""),("UAI",""),("UScriptAI","")]);
        for (id,name) in [(1,"FVector"),(2,"FExecutor"),(3,"UAI"),(4,"UScriptAI")] {
            r.type_identity_by_ptr.insert(id,TypeIdentity { name:name.into(),module:String::new(),namespace:String::new() });
        }
        let vector=DataType { token:5,type_info:1,..Default::default() };
        let reference=DataType { is_reference:true,is_object_const:true,is_read_only:true,..vector.clone() };
        let executor=DataType { type_info:2,..vector.clone() };
        let handle=DataType { type_info:3,is_object_handle:true,..vector.clone() };
        let void=DataType {token:0x52,..Default::default()};
        for (ptr,name,owner) in [(10,"CrossProduct",Some("FVector")),(11,"Location",Some("UAI")),
            (12,"Normalize",Some("FVector")),(20,"$beh0",Some("FExecutor")),(30,"opNeg",Some("FVector")),
            (40,"Consume",None),(50,"opAssign",Some("FExecutor")),(60,"$beh2",Some("FExecutor"))] {
            r.func_by_ptr.insert(ptr,name.into()); r.funcid_to_ptr.insert(ptr as i32,ptr);
            if let Some(owner)=owner { r.func_owner.insert(ptr,owner.into());r.func_is_method.insert(ptr); }
        }
        r.const_method_ptrs.extend([10,11,12,30]);
        // PreparedEmit supplies class overload keys separately from const pointer flags.
        let mut methods=HashSet::from(["Normalize/0/const".into(),"CrossProduct/1/const".into()]);
        if fault!=1 {methods.insert("opNeg/0/const".into());}
        r.set_class_methods(HashMap::from([("FVector".into(),methods)]));
        if fault==1 {r.const_method_ptrs.remove(&30);}
        for p in [10,11,12,30] {r.func_ret.insert(p,vector.clone());r.func_params.insert(p,Vec::new());}
        r.func_params.insert(10,vec![if fault==2 {vector.clone()} else {reference.clone()}]);
        r.func_params.insert(40,vec![handle,reference]);r.func_ret.insert(40,executor.clone());
        r.func_params.insert(50,vec![DataType {is_reference:true,..executor.clone()}]);
        r.func_ret.insert(50,DataType {is_reference:true,..executor});
        for p in [20,60] {r.func_ret.insert(p,void.clone());r.func_params.insert(p,Vec::new());}
        if fault==3 {r.func_owner.insert(20,"OtherExecutor".into());}
        if fault==4 {r.func_params.get_mut(&40).unwrap()[1].type_info=2;}
        if fault==5 {r.func_ret.get_mut(&40).unwrap().is_object_handle=true;}
        if fault==6 {r.func_params.get_mut(&20).unwrap().push(vector);}
        r.temporary_arg_positions.insert("CrossProduct".into(),HashMap::from([(1,vec![true])]));
        r.temporary_arg_positions.insert("Consume".into(),HashMap::from([(2,vec![false,true])]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_scalar_rvo_member_comparison(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FVector", "Z"), ("AActor", ""), ("FRotator", "Roll")]);
        for (ptr, name) in [(1, "FVector"), (2, "AActor"), (3, "FRotator")] {
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if fault == 7 && ptr == 1 { "Foreign" } else { "" }.into() });
        }
        r.prop_type_id.insert(3, if fault == 5 { 3 } else { 1 });
        r.prop_type_id.insert(7, 3);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("FVector", "Z", if fault == 6 { "float32" } else { "float" }),
            ("FRotator", "Roll", "float")], &[], None));
        r.func_by_ptr.insert(10, "Location".into());
        r.func_owner.insert(10, if fault == 4 { "UOther" } else { "AActor" }.into());
        if fault != 2 { r.func_is_method.insert(10); }
        if fault != 1 { r.const_method_ptrs.insert(10); }
        r.func_params.insert(10, if fault == 3 { vec![DataType { token: 0x51, ..Default::default() }] } else { vec![] });
        let mut ret = DataType { token: 5, type_info: 1, ..Default::default() };
        match fault {
            8 => ret.is_reference = true, 9 => ret.is_object_handle = true,
            10 => ret.is_object_const = true, 11 => ret.is_read_only = true,
            12 => ret.is_auto = true, 13 => ret.if_handle_then_const = true,
            _ => {},
        }
        r.func_ret.insert(10, ret);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_local_compound(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FVector", "Z")]);
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "FVector".into(), namespace: String::new(),
            module: if fault == 2 { "Foreign" } else { "" }.into() });
        r.prop_type_id.insert(3, if fault == 3 { 2 } else { 1 });
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("FVector", "Z", if fault == 1 { "float32" } else { "float" })], &[], None));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_indexed_int_compound(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FEntry", "Amount"), ("TArray", ""), ("FOther", "")]);
        for (ptr, name) in [(1, "FEntry"), (2, "TArray"), (3, "FOther")] {
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if fault == 13 && ptr == 1 { "" } else { "Fixture" }.into() });
        }
        r.prop_type_id.insert(3, if fault == 1 { 3 } else { 1 });
        r.set_class_fields(HashMap::from([("FEntry".into(), HashMap::from([("Amount".into(),
            if fault == 2 { "float" } else { "int" }.into())]))]));
        let mut element = DataType { token: 5, type_info: if fault == 3 { 3 } else { 1 }, ..Default::default() };
        if fault == 4 { element.is_object_handle = true; }
        r.type_subtypes.insert(2, vec![element]);
        r.func_by_ptr.insert(10, "opIndex".into());
        r.func_owner.insert(10, if fault == 5 { "TMap" } else { "TArray" }.into());
        if fault != 6 { r.func_is_method.insert(10); }
        if fault == 7 { r.const_method_ptrs.insert(10); }
        r.func_params.insert(10, vec![DataType { token: if fault == 8 { 0x51 } else { 0x44 },
            is_reference: fault == 9, ..Default::default() }]);
        r.func_ret.insert(10, DataType { token: 5, type_info: 1, is_reference: fault != 10,
            is_object_const: fault == 11, is_read_only: fault == 12, ..Default::default() });
        if fault == 14 { r.type_subtypes.remove(&2); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_addressed_compound_updates(fault:u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("UOwner","")]);
        r.type_identity_by_ptr.insert(1,TypeIdentity {name:"UOwner".into(),module:"Fixture".into(),namespace:String::new()});
        let mut fields=HashMap::new();
        for (offset,name) in [(0,"Angle"),(8,"Distance"),(16,"Interval")] {
            let key=(1i64<<1)|((offset as i64)<<33)|1;
            r.prop_by_key.insert(key,name.into());r.prop_type_id.insert(key,if fault==2 {2} else {1});
            fields.insert(name.into(),if fault==1 {"float32"} else {"float"}.into());
        }
        r.class_fields.insert("UOwner".into(),fields);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_repeated_field_handle(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UAbility", "Target"), ("AUnit", "")]);
        for (id, name) in [(1, "UAbility"), (2, "AUnit")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(),
                module: if fault == 2 { "Script" } else { "" }.into(), namespace: if fault == 3 { "Other" } else { "" }.into() });
        }
        r.prop_type_id.insert(3, if fault == 4 { 2 } else { 1 });
        if fault != 5 { r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("UAbility", "Target", if fault == 1 { "UOther" } else { "AUnit" })], &[], None)); }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_guarded_field_receiver(fault: u8) -> Self {
        let mut r = Self::from_test_repeated_field_handle(if fault <= 5 { fault } else { 0 });
        r.typeid_to_ptr.insert(3, 3); r.type_by_ptr.insert(3, "ASpecialUnit".into());
        r.type_identity_by_ptr.insert(3, TypeIdentity { name: "ASpecialUnit".into(), module: "Fixture".into(), namespace: String::new() });
        let int = DataType { token: 0x44, ..Default::default() }; let void = DataType { token: 0x52, ..Default::default() };
        for (ptr, name, owner, ret, args) in [
            (10, "GetLevel", "UAbility", int.clone(), vec![]),
            (11, "ApplyLevel", "ASpecialUnit", void.clone(), vec![int]),
            (12, "opCast", "UObject", void, vec![DataType { token: 0x3b, is_reference: true, ..Default::default() }]),
        ] {
            r.funcid_to_ptr.insert(ptr as i32, ptr); r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr); r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, args);
        }
        r.const_method_ptrs.insert(12);
        match fault {
            6 => r.func_ret.get_mut(&10).unwrap().token = 0x50,
            7 => r.func_params.get_mut(&11).unwrap()[0].is_reference = true,
            8 => r.func_params.get_mut(&12).unwrap()[0].is_reference = false,
            9 => { r.const_method_ptrs.remove(&12); }
            10 => r.type_identity_by_ptr.get_mut(&3).unwrap().namespace = "Other".into(),
            11 => { r.func_is_method.remove(&10); }
            12 => r.func_ret.get_mut(&11).unwrap().token = 0x44,
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_native_float_update(fault: u8) -> Self {
        let mut r = Self::default(); let float = DataType { token: 0x50, ..Default::default() };
        r.func_by_ptr.insert(10, "Round".into()); r.func_ret.insert(10, float.clone()); r.func_params.insert(10, vec![float.clone()]);
        r.funcid_to_ptr.insert(11, 11); r.func_by_ptr.insert(11, "GetAmount".into()); r.func_ret.insert(11, float.clone());
        r.func_params.insert(11, vec![DataType { token: 5, is_object_handle: true, ..Default::default() }, float]);
        match fault {
            1 => r.func_params.get_mut(&10).unwrap()[0].is_reference = true,
            2 => r.func_ret.get_mut(&10).unwrap().token = 0x51,
            3 => { r.func_is_method.insert(10); }
            4 => r.func_ret.get_mut(&11).unwrap().token = 0x44,
            5 => r.func_params.get_mut(&11).unwrap()[0].is_object_handle = false,
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_native_vector_sign(fault: u8) -> Self {
        let mut r = Self::from_test_native_vector_field_update(if fault<=6 {fault} else {0});
        r.global_ns.insert(9,"FVector".into());
        r.func_by_ptr.insert(14,"CrossProduct".into());r.func_owner.insert(14,"FVector".into());r.func_is_method.insert(14);r.const_method_ptrs.insert(14);
        r.func_ret.insert(14,r.func_ret[&12].clone());r.func_params.insert(14,r.func_params[&12].clone());
        match fault {
            7=>{r.global_ns.insert(9,"Other".into());}
            8=>r.func_ret.get_mut(&14).unwrap().is_reference=true,
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_vector_field_update(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FVector", "Z")]);
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "FVector".into(), module: if fault == 1 { "Script" } else { "" }.into(), namespace: String::new() });
        r.prop_type_id.insert(3, 1); r.global_by_ptr.insert(9, "UpVector".into());
        let value = DataType { token: 5, type_info: 1, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value.clone() };
        let scalar = DataType { token: 0x51, ..Default::default() };
        for (ptr, name, ret, args) in [
            (10,"RotateAngleAxis",value.clone(),vec![scalar.clone(),reference.clone()]),
            (11,"opMul",value.clone(),vec![scalar]),(12,"opAdd",value,vec![reference.clone()]),
            (13,"$beh0",DataType {token:0x52,..Default::default()},vec![reference]),
        ] {
            r.func_by_ptr.insert(ptr,name.into()); r.func_owner.insert(ptr,"FVector".into()); r.func_is_method.insert(ptr);
            r.func_ret.insert(ptr,ret); r.func_params.insert(ptr,args); if ptr != 13 {r.const_method_ptrs.insert(ptr);}
        }
        match fault {
            2 => r.func_params.get_mut(&11).unwrap()[0].token=0x50,
            3 => r.func_params.get_mut(&12).unwrap()[0].is_read_only=false,
            4 => r.func_ret.get_mut(&10).unwrap().is_reference=true,
            5 => {r.const_method_ptrs.remove(&11);}
            6 => {r.func_owner.insert(13,"FOther".into());}
            _=>{}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_ordered_distance_roots(fault: u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("UState","Gain"),("FVector","")]);
        for (id,name,module) in [(1,"UState","Fixture"),(2,"FVector","")] {
            r.type_identity_by_ptr.insert(id,TypeIdentity {name:name.into(),module:module.into(),namespace:String::new()});
        }
        r.prop_type_id.insert(3,1);
        r.class_fields.insert("UState".into(),HashMap::from([("Gain".into(),if fault==6 {"float"}else{"float32"}.into())]));
        let vector=DataType {token:5,type_info:2,..Default::default()};
        let reference=DataType {is_reference:true,is_object_const:true,is_read_only:true,..vector.clone()};
        let scalar=DataType {token:0x51,..Default::default()};
        for (ptr,name,ret,args) in [(10,"GetLocation",vector,vec![]),(11,"Distance",scalar.clone(),vec![reference.clone(),reference]),
            (12,"Sqrt",scalar.clone(),vec![scalar])] {
            r.funcid_to_ptr.insert(ptr as i32,ptr);r.func_by_ptr.insert(ptr,name.into());r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
        }
        r.func_is_method.extend([10,11]);r.const_method_ptrs.insert(10);
        match fault {
            1=>r.func_ret.get_mut(&10).unwrap().is_reference=true,
            2=>r.func_params.get_mut(&11).unwrap()[1].is_read_only=false,
            3=>r.func_ret.get_mut(&12).unwrap().token=0x50,
            4=>{r.func_is_method.insert(12);}
            5=>{r.const_method_ptrs.remove(&10);}
            _=>{}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_discarded_fstring_predecessor(fault:u8)->Self {
        let mut r=Self::from_test_member_chain(&[("FString","")]);
        r.type_identity_by_ptr.insert(1,TypeIdentity {name:"FString".into(),module:if fault==1 {"Script"}else{""}.into(),namespace:String::new()});
        for (ptr,text) in [(20,"ignored"),(21,"prefix")] {r.global_by_ptr.insert(ptr,text.into());r.global_is_string.insert(ptr);}
        let reference=DataType {token:5,type_info:1,is_reference:true,is_object_const:true,is_read_only:true,..Default::default()};
        let void=DataType {token:0x52,..Default::default()};
        for (ptr,name,ret,args) in [(10,"$beh0",void.clone(),vec![reference.clone()]),(11,"$beh2",void,vec![]),
            (12,"Append",DataType {is_object_const:false,is_read_only:false,..reference.clone()},vec![reference])] {
            r.func_by_ptr.insert(ptr,name.into());r.func_owner.insert(ptr,"FString".into());r.func_is_method.insert(ptr);r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
        }
        match fault {
            2=>r.func_params.get_mut(&10).unwrap()[0].is_read_only=false,
            3=>{r.const_method_ptrs.insert(11);}
            4=>r.func_ret.get_mut(&12).unwrap().is_reference=false,
            5=>{r.global_is_string.remove(&20);}
            6=>{r.func_owner.insert(12,"FOther".into());}
            _=>{}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_ordered_navigation_vectors(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UState","OffsetDistance"),("UBase","AI"),("FVector",""),("AGothicCharacter","")]);
        for (id,name,module) in [(1,"UState","Fixture"),(2,"UBase","Fixture"),(3,"FVector",""),(4,"AGothicCharacter","")] {
            r.type_identity_by_ptr.insert(id,TypeIdentity {name:name.into(),module:module.into(),namespace:String::new()});
        }
        r.prop_type_id.extend([(3,1),(5,2)]);
        r.class_fields.insert("UState".into(),HashMap::from([("OffsetDistance".into(),"float".into())]));
        r.global_by_ptr.extend([(9,"ZeroVector".into()),(8,"UpVector".into())]);
        let value = DataType {token:5,type_info:3,..Default::default()};
        let reference = DataType {is_reference:true,is_object_const:true,is_read_only:true,..value.clone()};
        let scalar = DataType {token:0x51,..Default::default()};
        let actor = DataType {token:5,type_info:4,is_object_handle:true,..Default::default()};
        for (ptr,name,owner,ret,args) in [
            (10,"GetCharacter","UGameplayAbility_AI",actor,vec![]),(11,"GetFeetLocation","AGothicCharacter",value.clone(),vec![]),
            (12,"opSub","FVector",value.clone(),vec![reference.clone()]),
            (13,"GetSafeNormal2D","FVector",value.clone(),vec![scalar.clone(),reference.clone()]),
            (14,"CrossProduct","FVector",value.clone(),vec![reference.clone()]),
            (15,"opMul","FVector",value.clone(),vec![scalar]),(16,"opAdd","FVector",value,vec![reference]),
        ] {
            r.func_by_ptr.insert(ptr,name.into());r.func_owner.insert(ptr,owner.into());r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
            r.func_is_method.insert(ptr);r.const_method_ptrs.insert(ptr);
        }
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&3).unwrap().module="Script".into(),
            2=>r.func_params.get_mut(&13).unwrap()[0].token=0x50,
            3=>r.func_params.get_mut(&14).unwrap()[0].is_read_only=false,
            4=>r.func_ret.get_mut(&15).unwrap().is_reference=true,
            5=>{r.const_method_ptrs.remove(&11);}
            6=>{r.class_fields.get_mut("UState").unwrap().insert("OffsetDistance".into(),"float32".into());}
            7=>{r.global_by_ptr.insert(8,"ForwardVector".into());}
            8=>r.func_ret.get_mut(&10).unwrap().is_object_handle=false,
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_repeated_event_scope(fault: u8) -> Self {
        let mut r=Self::default();
        let types=["FGameplayEffectContext_HitResponse","FGameplayEventData","FGameplayEffectContextHandle","AActor",
            "FGameplayEffectSpec","UCombatConfig","UDataModule_Combat","UComboAttackConfig","FGameplayTag","FExecution","FWeapon","EMode"];
        for (n,name) in types.iter().enumerate() {
            let id=n as i64+1;r.typeid_to_ptr.insert(id as i32,id);r.type_by_ptr.insert(id,(*name).into());r.type_names.insert((*name).into());
            r.type_identity_by_ptr.insert(id,TypeIdentity {name:(*name).into(),module:String::new(),namespace:String::new()});
        }
        for (id,off,name) in [(1,128,"Impact"),(2,16,"Target"),(2,8,"Instigator"),(2,24,"OptionalObject")] {
            let key=((id as i64)<<1)|((off as i64)<<33)|1;r.prop_by_key.insert(key,name.into());r.prop_type_id.insert(key,id);
        }
        let value=|ty|DataType {token:5,type_info:ty,..Default::default()};
        let reference=|ty,constant|DataType {is_reference:true,is_object_const:constant,is_read_only:constant,..value(ty)};
        let handle=|ty|DataType {is_object_handle:true,..value(ty)};
        let void=DataType {token:0x52,..Default::default()};
        for (ptr,owner,name,constant,ret,args,ns) in [
            (20,5,"GetContext",true,value(3),vec![],""),(21,3,"GetHitContext",true,value(1),vec![],""),
            (22,3,"$beh2",false,void.clone(),vec![],""),(23,1,"$beh0",false,void.clone(),vec![reference(1,true)],""),
            (24,1,"$beh2",false,void.clone(),vec![],""),(25,9,"opAssign",false,reference(9,false),vec![reference(9,true)],""),
            (26,2,"$beh0",false,void.clone(),vec![],""),(27,0,"GetCombatDataModule",false,handle(7),vec![handle(4)],"DataModule"),
            (28,7,"GetCurrentCombo",true,handle(8),vec![],""),(29,0,"InitializeHitData",false,void.clone(),vec![reference(2,false),reference(1,false)],"GothicGAS"),
            (30,0,"SendGameplayEvent",false,void.clone(),vec![handle(4),value(9),reference(2,false)],"GothicGAS"),
            (31,2,"$beh2",false,void.clone(),vec![],""),(32,6,"GetParryMode",true,value(12),vec![],""),
            (33,0,"AddTargetInputData",false,void,vec![reference(2,false),value(9)],"GothicGAS"),
        ] {
            r.func_by_ptr.insert(ptr,name.into());r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
            if owner!=0 {r.func_owner.insert(ptr,types[owner-1].into());r.func_is_method.insert(ptr);}
            if constant {r.const_method_ptrs.insert(ptr);}if !ns.is_empty(){r.func_ns.insert(ptr,ns.into());}
        }
        for (ptr,name) in [(100,"Impact"),(101,"Outgoing"),(102,"Incoming")] {r.global_by_ptr.insert(ptr,name.into());r.global_ns.insert(ptr,"GameplayTag".into());}
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&1).unwrap().module="Script".into(),
            2=>r.func_params.get_mut(&23).unwrap()[0].is_read_only=false,
            3=>r.func_ret.get_mut(&20).unwrap().is_reference=true,
            4=>{r.func_params.get_mut(&26).unwrap().push(value(9));}
            5=>{r.func_owner.insert(31,"FOther".into());}
            6=>{r.func_ns.insert(30,"Other".into());}
            7=>r.func_params.get_mut(&29).unwrap()[0].type_info=1,
            8=>{r.global_ns.insert(101,"Other".into());}
            9=>{r.const_method_ptrs.remove(&28);}
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_scoped_or_result(fault: u8) -> Self {
        let mut r=Self::default();
        for (id,name) in [(2,"FGameplayTag"),(3,"ACharacterState")] { r.type_identity_by_ptr.insert(id,TypeIdentity {name:name.into(),module:String::new(),namespace:String::new()}); }
        r.global_by_ptr.insert(9,"Right".into());r.global_ns.insert(9,"GameplayTag".into());
        r.funcid_to_ptr.insert(10,10);r.func_by_ptr.insert(10,"GetActor".into());r.func_is_method.insert(10);
        r.func_ret.insert(10,DataType {token:5,type_info:3,is_object_handle:true,..Default::default()});r.func_params.insert(10,vec![]);
        r.func_by_ptr.insert(11,"HasTag".into());r.func_owner.insert(11,"ACharacterState".into());r.func_is_method.insert(11);r.const_method_ptrs.insert(11);
        r.func_ret.insert(11,DataType {token:0x41,..Default::default()});
        r.func_params.insert(11,vec![DataType {token:5,type_info:2,is_reference:true,is_object_const:true,is_read_only:true,..Default::default()}]);
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&2).unwrap().module="Script".into(),
            2=>r.func_ret.get_mut(&11).unwrap().token=0x44,
            3=>r.func_ret.get_mut(&10).unwrap().is_reference=true,
            4=>{r.const_method_ptrs.remove(&11);}
            5=>{r.global_ns.insert(9,"Other".into());}
            6=>r.func_params.get_mut(&11).unwrap()[0].is_read_only=false,
            7=>{r.func_owner.insert(11,"AOther".into());}
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_bool_branch(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UBase", "Flag"), ("USpecial", "")]);
        for (id, name) in [(1, "UBase"), (2, "USpecial")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: if fault == 4 && id == 2 { "FValue" } else { name }.into(),
                module: if id == 2 || fault == 2 { "Script" } else { "" }.into(),
                namespace: if fault == 5 { "Other" } else { "" }.into() });
        }
        r.prop_type_id.insert(3, if fault == 3 { 2 } else { 1 });
        r.class_super.insert("USpecial".into(), "UNativeMiddle".into());
        if fault != 6 {
            r.set_native_api(super::binds::NativeApi::from_test_field_types(
                &[("UBase", "Flag", if fault == 1 { "int8" } else { "bool" })], &[], None));
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_normalized_bool_member(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FEntry","Flag"),("FFilter","")]);
        for (id,name) in [(1,"FEntry"),(2,"FFilter")] {
            r.type_identity_by_ptr.insert(id,TypeIdentity { name:name.into(),namespace:String::new(),
                module:if id==2 || fault==2 {"Fixture"} else {""}.into() });
        }
        r.prop_type_id.insert(3,if fault==3 {2} else {1});
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("FEntry","Flag",if fault==1 {"int"} else {"bool"})],&[],None));
        r.func_by_ptr.insert(10,"Proceed".into());
        r.func_ret.insert(10,DataType { token:5,type_info:if fault==4 {2} else {1},
            is_reference:fault!=5,is_object_const:true,is_read_only:true,..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_double_product_bool_argument(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UOwner", "Config")]);
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "UOwner".into(), module: "Fixture".into(), namespace: String::new() });
        for (offset, name) in [(0, "Config"), (8, "Hit")] {
            let key = (1i64 << 1) | ((offset as i64) << 33) | 1;
            r.prop_by_key.insert(key,name.into()); r.prop_type_id.insert(key,if fault == 4 { 2 } else { 1 });
        }
        r.class_fields.insert("UOwner".into(), HashMap::from([("Config".into(),"UConfig".into()),
            ("Hit".into(),if fault == 5 { "UOther" } else { "UHit" }.into())]));
        for (ptr,name,owner) in [(10,"Speed",Some("UConfig")), (20,"Min",None), (30,"SetLength",Some("UHit"))] {
            r.func_by_ptr.insert(ptr,name.into());
            if let Some(owner) = owner { r.func_owner.insert(ptr,owner.into()); r.func_is_method.insert(ptr); }
        }
        if fault != 7 { r.func_ns.insert(20,"Math".into()); }
        if fault == 6 { r.func_is_method.insert(20); }
        let narrow = DataType { token: 0x50, ..Default::default() };
        let wide = DataType { token: 0x51, ..Default::default() };
        r.func_ret.insert(10,if fault == 1 { wide.clone() } else { narrow.clone() });
        r.func_params.insert(10,if fault == 8 { vec![narrow.clone()] } else { Vec::new() });
        r.func_ret.insert(20,wide.clone());
        r.func_params.insert(20,vec![wide.clone(),if fault == 2 { narrow.clone() } else { wide }]);
        r.func_ret.insert(30,DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(30,vec![narrow,DataType { token: if fault == 3 { 0x44 } else { 0x41 }, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_retained_double_quotient(narrow: bool) -> Self {
        let mut r = Self::from_test_literal_product_call(narrow);
        let value = r.func_ret.get(&1).unwrap().clone();
        r.func_by_ptr.insert(2, "Lerp".into()); r.func_ns.insert(2, "Math".into());
        r.func_ret.insert(2, value.clone());
        r.func_params.insert(2, vec![DataType { is_reference: true, is_read_only: true, ..value }; 3]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_literal_product_call(narrow: bool) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(1, "Max".into()); r.func_ns.insert(1, "Math".into());
        let double = DataType { token: if narrow { 0x50 } else { 0x51 }, ..Default::default() };
        r.func_ret.insert(1, double.clone()); r.func_params.insert(1, vec![double.clone(), double]);
        r.prop_by_key.insert((1 << 1) | 1, "Scale".into());
        r.prop_type_id.insert((1 << 1) | 1, 1);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_eager_sum_before_field(narrow: bool) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(1, "UHolder".into()); r.typeid_to_ptr.insert(1, 1);
        r.func_by_ptr.insert(1, "Min".into()); r.func_ns.insert(1, "Math".into());
        let double = DataType { token: if narrow { 0x50 } else { 0x51 }, ..Default::default() };
        r.func_ret.insert(1, double.clone()); r.func_params.insert(1, vec![double.clone(), double]);
        for (offset, name) in [(0i64, "Suppressed"), (8, "Max")] {
            let key = (1i64 << 1) | (offset << 33) | 1;
            r.prop_by_key.insert(key, name.into()); r.prop_type_id.insert(key, 1);
        }
        r.set_class_fields(HashMap::from([("UHolder".into(), HashMap::from([("Suppressed".into(), "float".into()), ("Max".into(), "float".into())]))]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_eager_quotient(narrow: bool, mutable: bool) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(1, "Mix".into()); r.func_ns.insert(1, "Math".into());
        let double = DataType { token: if narrow { 0x50 } else { 0x51 }, ..Default::default() };
        r.func_ret.insert(1, double.clone());
        r.func_params.insert(1, vec![DataType { is_reference: true, is_read_only: !mutable, ..double }; 3]);
        for (offset, name) in [(0i64, "Max"), (8, "Min")] {
            let key = (1 << 1) | (offset << 33) | 1;
            r.prop_by_key.insert(key, name.into()); r.prop_type_id.insert(key, 1);
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_foreach_getter(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "UActor"), (2, "UComponent"), (3, "TArrayIterator")] {
            r.type_by_ptr.insert(ptr, name.into()); r.typeid_to_ptr.insert(ptr as i32, ptr);
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let key = (2 << 1) | (160 << 33) | 1;
        r.prop_by_key.insert(key, "Items".into()); r.prop_type_id.insert(key, 2);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("UComponent", "Items", if fault == 9 { "TSet<UItem>" } else { "TArray<UItem>" })], &[], None));
        for (ptr, name, owner, ty, handle) in [(10, "GetComponent", "UActor", 2, true), (11, "Iterator", "TArray", 3, false)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr); r.func_params.insert(ptr, vec![]);
            r.func_ret.insert(ptr, DataType { token: 5, type_info: ty, is_object_handle: handle, ..Default::default() });
        }
        r.const_method_ptrs.insert(10);
        match fault {
            1 => { r.const_method_ptrs.remove(&10); }
            2 => { r.func_is_method.remove(&10); }
            3 => { r.func_owner.insert(10, "UOther".into()); }
            4 => r.func_params.get_mut(&10).unwrap().push(DataType::default()),
            5 => r.func_ret.get_mut(&10).unwrap().is_reference = true,
            6 => r.func_ret.get_mut(&10).unwrap().is_object_handle = false,
            7 => r.type_identity_by_ptr.get_mut(&2).unwrap().module = "Script".into(),
            8 => { r.prop_type_id.insert(key, 1); }
            10 => { r.func_owner.insert(11, "TSet".into()); }
            11 => { r.const_method_ptrs.insert(11); }
            12 => r.func_params.get_mut(&11).unwrap().push(DataType::default()),
            13 => r.func_ret.get_mut(&11).unwrap().is_reference = true,
            14 => r.func_ret.get_mut(&11).unwrap().is_object_handle = true,
            15 => r.type_identity_by_ptr.get_mut(&3).unwrap().namespace = "Foreign".into(),
            16 => { r.native = None; }
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_completed_string_argument(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "FString"), (2, "ECategory")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let string = DataType { token: 5, type_info: 1, ..Default::default() };
        let void = DataType { token: 0x52, ..Default::default() };
        let input = |token, ty| DataType { token, type_info: ty, is_reference: true, is_object_const: true, is_read_only: true, ..Default::default() };
        for (p, name, ret, args) in [(10, "opAdd", string.clone(), vec![input(5, 1)]),
            (11, "$beh2", void.clone(), vec![]), (12, "opAdd", string.clone(), vec![input(0x51, 0)])] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, "FString".into()); r.func_is_method.insert(p);
            r.func_ret.insert(p, ret); r.func_params.insert(p, args);
        }
        r.const_method_ptrs.extend([10, 12]);
        r.func_by_ptr.insert(13, "Write".into()); r.func_ns.insert(13, "TestLog".into());
        r.func_ret.insert(13, void);
        r.func_params.insert(13, vec![DataType { token: 5, type_info: 2, ..Default::default() },
            DataType { is_object_const: true, is_read_only: true, ..string }]);
        match fault {
            1 => r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(),
            2 => r.func_ret.get_mut(&12).unwrap().is_reference = true,
            3 => r.func_params.get_mut(&10).unwrap()[0].type_info = 2,
            4 => r.func_params.get_mut(&12).unwrap()[0].token = 0x44,
            5 => r.func_params.get_mut(&12).unwrap()[0].is_reference = false,
            6 => { r.const_method_ptrs.remove(&12); }
            7 => { r.func_owner.insert(12, "FOther".into()); }
            8 => { r.func_by_ptr.insert(10, "opAssign".into()); }
            9 => { r.func_by_ptr.insert(11, "$beh0".into()); }
            10 => { r.const_method_ptrs.insert(11); }
            11 => r.func_params.get_mut(&11).unwrap().push(DataType::default()),
            12 => { r.func_is_method.insert(13); }
            13 => r.func_params.get_mut(&13).unwrap()[1].is_reference = true,
            14 => r.func_params.get_mut(&13).unwrap()[0].token = 0x44,
            15 => r.func_ret.get_mut(&13).unwrap().token = 0x41,
            16 => { r.func_ns.remove(&13); }
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_name_predicate_argument(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("AHost", "Component"), ("FName", "")]);
        for (id, name) in [(1, "AHost"), (2, "FName")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: if fault == 8 { "Script" } else { "" }.into(), namespace: String::new() });
        }
        r.prop_type_id.insert(3, if fault == 9 { 2 } else { 1 });
        if fault != 11 { r.set_native_api(super::binds::NativeApi::from_test_field_types(&[("AHost", "Component", if fault == 10 { "FValue" } else { "UComponent" })], &[], None)); }
        let value = DataType { token: 5, type_info: 2, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value.clone() };
        for (ptr, name, ret, args) in [
            (10, "__STATIC_NAME", reference.clone(), vec![DataType { token: 0x44, ..Default::default() }]),
            (11, "$beh0", DataType { token: 0x52, ..Default::default() }, vec![reference]),
            (12, "TestName", DataType { token: 0x41, ..Default::default() }, vec![value])
        ] { r.func_by_ptr.insert(ptr, name.into()); r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, args); }
        for (ptr, owner) in [(11, "FName"), (12, "UComponent")] { r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); }
        r.const_method_ptrs.insert(12);
        match fault {
            1 => r.func_ret.get_mut(&10).unwrap().is_reference = false,
            2 => r.func_params.get_mut(&10).unwrap()[0].token = 0x50,
            3 => r.func_params.get_mut(&11).unwrap()[0].is_reference = false,
            4 => { r.func_owner.insert(11, "FOther".into()); }
            5 => r.func_params.get_mut(&12).unwrap()[0].is_reference = true,
            6 => r.func_ret.get_mut(&12).unwrap().token = 0x44,
            7 => { r.const_method_ptrs.remove(&12); }
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_retained_text_comparison(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "FString"), (2, "FText")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let value = |ptr| DataType { token: 5, type_info: ptr, ..Default::default() };
        let input = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value(1) };
        r.funcid_to_ptr.insert(20, 20); r.func_by_ptr.insert(20, "ReadText".into());
        r.func_ret.insert(20, value(2)); r.func_params.insert(20, vec![input.clone()]);
        for (ptr, name, owner, ret, args, constant) in [
            (21, "ToString", "FText", value(1), vec![], true),
            (22, "opEquals", "FString", DataType { token: 0x41, ..Default::default() }, vec![input], true),
            (23, "$beh2", "FString", DataType { token: 0x52, ..Default::default() }, vec![], false),
            (24, "$beh2", "FText", DataType { token: 0x52, ..Default::default() }, vec![], false)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, args);
            if constant { r.const_method_ptrs.insert(ptr); }
        }
        match fault {
            1 => r.func_ret.get_mut(&20).unwrap().is_reference = true,
            2 => r.func_ret.get_mut(&20).unwrap().type_info = 1,
            3 => r.func_params.get_mut(&20).unwrap()[0].is_reference = false,
            4 => { r.func_owner.insert(21, "FString".into()); }
            5 => { r.const_method_ptrs.remove(&21); }
            6 => r.func_ret.get_mut(&21).unwrap().is_reference = true,
            7 => r.func_ret.get_mut(&22).unwrap().token = 0x44,
            8 => r.func_params.get_mut(&22).unwrap()[0].is_reference = false,
            9 => { r.func_owner.insert(23, "FText".into()); }
            10 => { r.const_method_ptrs.insert(24); }
            11 => r.type_identity_by_ptr.get_mut(&2).unwrap().module = "Script".into(),
            12 => { r.func_is_method.insert(20); }
            13 => r.type_identity_by_ptr.get_mut(&1).unwrap().namespace = "Other".into(),
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_const_set_field() -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(201, "TSet"), (202, "TArray")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: "Module".into(), namespace: String::new() });
            r.type_subtypes.insert(ptr, vec![DataType { token: 0x44, ..Default::default() }]);
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_reused_foreach_lives(fault: u8) -> Self {
        let mut r = Self::default();
        for (id, name) in [(1, "UHost"), (2, "UFlow"), (3, "TArrayIterator"), (4, "UNode")] {
            let ptr = id as i64 + 100;
            r.typeid_to_ptr.insert(id, ptr); r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: "Script".into(), namespace: String::new() });
        }
        let key = |id: i64, offset: i64| (id << 1) | (offset << 33) | 1;
        for (id, offset, name) in [(1, 8, "First"), (1, 24, "Second"), (2, 16, "Nodes")] {
            r.prop_by_key.insert(key(id, offset), name.into()); r.prop_type_id.insert(key(id, offset), id as i32);
        }
        r.class_fields.insert("UHost".into(), HashMap::from([("First".into(), "UFlow".into()), ("Second".into(), "UFlow".into())]));
        r.class_fields.insert("UFlow".into(), HashMap::from([("Nodes".into(), "TArray<UNode>".into())]));
        for (ptr, name, owner, ty, reference, handle) in [(10, "Iterator", "TArray", 103, false, false),
            (11, "Proceed", "TArrayIterator", 104, true, true)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            r.func_params.insert(ptr, vec![]);
            r.func_ret.insert(ptr, DataType { token: 5, type_info: ty, is_reference: reference, is_object_handle: handle, ..Default::default() });
        }
        match fault {
            1 => r.func_ret.get_mut(&10).unwrap().is_reference = true,
            2 => { r.func_is_method.remove(&10); }
            3 => r.func_ret.get_mut(&11).unwrap().is_reference = false,
            4 => r.func_ret.get_mut(&11).unwrap().is_object_handle = false,
            5 => { r.const_method_ptrs.insert(10); }
            6 => { r.func_owner.insert(11, "TOtherIterator".into()); }
            7 => { r.prop_type_id.insert(key(2, 16), 1); }
            8 => { r.class_fields.get_mut("UHost").unwrap().insert("Second".into(), "UOther".into()); }
            9 => { r.class_fields.get_mut("UFlow").unwrap().insert("Nodes".into(), "TSet<UNode>".into()); }
            10 => r.func_ret.get_mut(&10).unwrap().type_info = 104,
            11 => r.func_params.get_mut(&10).unwrap().push(DataType::default()),
            12 => r.func_ret.get_mut(&11).unwrap().is_read_only = true,
            _ => {}
        }
        r
    }


    #[cfg(test)]
    pub(crate) fn from_test_direct_enum_index(fault: u8) -> Self {
        let mut r = Self::from_test_hidden_return_enum_arguments(fault);
        r.type_identity_by_ptr.get_mut(&6).unwrap().module = "Script".into();
        r.typeid_to_ptr.insert(1, 6);
        let key = (1 << 1) | (12 << 33) | 1;
        r.prop_by_key.insert(key, "Choices".into()); r.prop_type_id.insert(key, 1);
        r.class_fields.insert("UHost".into(), HashMap::from([("Choices".into(), "TArray<ESecond>".into())]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_hidden_return_enum_arguments(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("EFirst", ""), ("ESecond", ""),
            ("FState", ""), ("FResult", ""), ("TArray", ""), ("UHost", "")]);
        for (p, name) in [(1, "EFirst"), (2, "ESecond"), (3, "FState"),
            (4, "FResult"), (5, "TArray"), (6, "UHost")] {
            r.type_identity_by_ptr.insert(p, TypeIdentity { name: name.into(),
                module: String::new(), namespace: String::new() });
        }
        let value = |p| DataType { token: 5, type_info: p, ..Default::default() };
        let constant = |p| DataType { is_object_const: true, is_read_only: true, ..value(p) };
        r.type_subtypes.insert(5, vec![value(2)]);
        for (p, name, owner, params, ret) in [
            (10, "GetFirst", "UHost", vec![], value(1)),
            (20, "opIndex", "TArray", vec![DataType { token: 0x44, ..Default::default() }],
                DataType { is_reference: true, ..value(2) }),
            (30, "Make", "UHost", vec![constant(1), constant(2),
                DataType { is_reference: true, ..constant(3) }], value(4)),
            (40, "Inspect", "FResult", vec![], DataType { token: 0x52, ..Default::default() }),
        ] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into());
            r.func_is_method.insert(p); r.func_params.insert(p, params); r.func_ret.insert(p, ret);
            r.funcid_to_ptr.insert(p as i32, p);
        }
        r.temporary_arg_positions.insert("Make".into(), HashMap::from([(3, vec![true, true, false])]));
        for name in ["EFirst", "ESecond"] {
            r.ctor_arg_positions.insert(name.into(), HashMap::from([(1, vec![true])]));
        }
        match fault {
            1 => r.func_ret.get_mut(&30).unwrap().is_reference = true,
            2 => r.func_ret.get_mut(&30).unwrap().is_object_handle = true,
            3 => r.func_ret.get_mut(&30).unwrap().type_info = 3,
            4 => { r.func_ret.remove(&30); },
            5 => { r.func_is_method.remove(&30); },
            6 => r.func_ret.get_mut(&20).unwrap().is_reference = false,
            7 => r.func_ret.get_mut(&20).unwrap().type_info = 1,
            8 => { r.func_owner.insert(20, "TMap".into()); },
            9 => { r.func_is_method.remove(&20); },
            10 => r.func_params.get_mut(&20).unwrap()[0].token = 0x51,
            11 => r.type_identity_by_ptr.get_mut(&2).unwrap().namespace = "Other".into(),
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_getter_before_global_cast(fault: u8) -> Self {
        let mut r = Self::default();
        for (id, name) in [(1, "UProducer"), (2, "UComponent"), (3, "USpecialComponent"), (4, "FTag")] {
            let ptr = i64::from(id) + 100;
            r.typeid_to_ptr.insert(id, ptr); r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (p, name, owner) in [(1, "GetComponent", "UProducer"), (2, "opCast", "UObject"), (3, "Test", "UNativeComponentBase")] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into());
            r.func_is_method.insert(p); r.const_method_ptrs.insert(p); r.func_params.insert(p, vec![]);
        }
        r.func_ret.insert(1, DataType { token: 5, type_info: 102, is_object_handle: true, ..Default::default() });
        r.func_ret.insert(2, DataType { token: 0x52, ..Default::default() });
        r.func_ret.insert(3, DataType { token: 0x41, ..Default::default() });
        r.func_params.insert(2, vec![DataType { token: 0x3b, is_reference: true, ..Default::default() }]);
        r.func_params.insert(3, vec![DataType { token: 5, type_info: 104, ..Default::default() }]);
        r.global_by_ptr.insert(100, "Ready".into()); r.global_ns.insert(100, "Tags".into());
        // Cast<T> is a typed nested call argument in the actual inliner.
        r.temporary_arg_positions.insert("Cast".into(), [(1, vec![true])].into_iter().collect());
        match fault {
            1 => { r.func_ret.get_mut(&1).unwrap().is_object_handle = false; },
            2 => { r.func_ret.get_mut(&1).unwrap().is_reference = true; },
            3 => { r.func_params.get_mut(&1).unwrap().push(DataType::default()); },
            4 => { r.const_method_ptrs.remove(&1); },
            5 => { r.func_owner.insert(1, "UOther".into()); },
            6 => { r.type_identity_by_ptr.get_mut(&101).unwrap().module = "Foreign".into(); },
            7 => { r.type_identity_by_ptr.get_mut(&102).unwrap().namespace = "Foreign".into(); },
            8 => { r.type_identity_by_ptr.get_mut(&103).unwrap().module = "Foreign".into(); },
            9 => { r.func_params.get_mut(&2).unwrap()[0].is_reference = false; },
            10 => { r.func_owner.insert(2, "UOther".into()); },
            11 => { r.func_ret.get_mut(&3).unwrap().token = 0x44; },
            12 => { r.func_params.get_mut(&3).unwrap()[0].is_reference = true; },
            13 => { r.type_identity_by_ptr.get_mut(&104).unwrap().namespace = "Foreign".into(); },
            14 => { r.const_method_ptrs.remove(&3); },
            15 => { r.func_is_method.remove(&3); },
            16 => { r.global_by_ptr.remove(&100); },
            17 => { r.global_is_string.insert(100); },
            18 => { r.global_ns.remove(&100); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_const_native_field_enum_loop(fault: u8) -> Self {
        let mut r = Self::default();
        for (id, name) in [(1, "FEntry"), (2, "FChild"), (3, "TArray"),
            (4, "TArrayIterator"), (5, "TArrayConstIterator"), (6, "EDisposition")] {
            r.typeid_to_ptr.insert(id, i64::from(id) + 100);
            r.type_by_ptr.insert(i64::from(id) + 100, name.into());
            r.type_identity_by_ptr.insert(i64::from(id) + 100,
                TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let key = |id: i64, offset: i64| (id << 1) | (offset << 33) | 1;
        for (id, offset, name) in [(1, 0, "ID"), (1, 17, "Disposition"), (1, 40, "Children"),
            (2, 8, "Disposition"), (4, 16, "CanProceed"), (5, 16, "CanProceed")] {
            r.prop_by_key.insert(key(id, offset), name.into());
            r.prop_type_id.insert(key(id, offset), id as i32);
        }
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("FEntry", "Disposition", "EDisposition"),
            ("FEntry", "Children", if fault == 21 { "TSet<FChild>" } else { "TArray<FChild>" }),
            ("FChild", "Disposition", if fault == 12 { "int" } else { "EDisposition" })], &[], None));
        for (p, name, owner, ty, reference, constant) in [
            (1, "Iterator", "TArray", 104, false, false),
            (2, "Proceed", "TArrayIterator", 101, true, false),
            (3, "Iterator", "TArray", 105, false, false),
            (4, "Proceed", "TArrayConstIterator", 102, true, true)] {
            r.func_by_ptr.insert(p, name.into()); r.func_owner.insert(p, owner.into());
            r.func_is_method.insert(p); r.func_params.insert(p, vec![]);
            r.func_ret.insert(p, DataType { token: 5, type_info: ty, is_reference: reference,
                is_object_const: constant, is_read_only: constant, ..Default::default() });
        }
        r.const_method_ptrs.insert(3);
        r.temporary_arg_positions.insert("int".into(), [(1, vec![true])].into_iter().collect());
        match fault {
            1 => { r.type_identity_by_ptr.get_mut(&101).unwrap().namespace = "Foreign".into(); },
            2 => { r.prop_type_id.insert(key(1, 0), 2); },
            3 => { r.prop_type_id.insert(key(1, 40), 2); },
            4 => { r.const_method_ptrs.remove(&3); },
            5 => { r.func_ret.get_mut(&3).unwrap().type_info = 104; },
            6 => { r.func_ret.get_mut(&2).unwrap().is_reference = false; },
            7 => { r.func_is_method.remove(&2); },
            8 => { r.func_params.get_mut(&3).unwrap().push(DataType::default()); },
            9 => { r.type_identity_by_ptr.get_mut(&104).unwrap().module = "Foreign".into(); },
            10 => { r.duplicate_prop_keys.insert(key(1, 40)); },
            11 => { r.native = None; },
            13 => { r.type_identity_by_ptr.get_mut(&102).unwrap().module = "Foreign".into(); },
            14 => { r.prop_type_id.insert(key(2, 8), 1); },
            15 => { r.func_ret.get_mut(&4).unwrap().is_object_const = false; },
            16 => { r.func_ret.get_mut(&4).unwrap().is_reference = false; },
            17 => { r.func_params.get_mut(&4).unwrap().push(DataType::default()); },
            18 => { r.func_is_method.remove(&4); },
            19 => { r.func_owner.insert(4, "TArrayIterator".into()); },
            20 => { r.duplicate_prop_keys.insert(key(2, 8)); },
            22 => { r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
                ("FChild", "Disposition", "EDisposition")], &[], None)); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_const_field_iterator(is_const: bool) -> Self {
        let mut r = Self::default();
        r.typeid_to_ptr.insert(1, 100); r.type_by_ptr.insert(100, "UPackage".into()); r.type_by_ptr.insert(101, "TSetConstIterator".into());
        r.prop_by_key.insert((1 << 1) | 1, "Items".into()); r.prop_type_id.insert((1 << 1) | 1, 1);
        r.func_by_ptr.insert(1, "Iterator".into()); r.func_is_method.insert(1);
        if is_const { r.const_method_ptrs.insert(1); }
        r.func_params.insert(1, vec![]);
        r.func_ret.insert(1, DataType { token: 5, type_info: 101, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_adjacent_default_arguments(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(11, "FProbeHit"), (12, "FProbeContext"), (13, "FProbeResult")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (ptr, name, owner) in [(1,"$beh0",Some("FProbeHit")), (2,"$beh0",Some("FProbeContext")),
            (3,"$beh2",Some("FProbeHit")), (4,"$beh2",Some("FProbeContext")),
            (5,"Configure",Some("FProbeContext")), (6,"Consume",None),
            (7,"$beh2",Some("FProbeResult")), (8,"After",None)] {
            r.func_by_ptr.insert(ptr,name.into());
            if let Some(owner) = owner { r.func_owner.insert(ptr,owner.into()); r.func_is_method.insert(ptr); }
            r.func_ret.insert(ptr,DataType { token:0x52,..Default::default() });
            r.func_params.insert(ptr,Vec::new());
        }
        r.func_ret.insert(6,DataType { token:5,type_info:13,..Default::default() });
        r.func_ret_names.insert("Consume".into(),"FProbeResult".into());
        r.func_params.insert(6,vec![DataType { token:5,type_info:12,..Default::default() },
            DataType { token:5,type_info:11,is_reference:true,is_object_const:true,is_read_only:true,..Default::default() }]);
        if fault == 1 { r.func_params.get_mut(&6).unwrap()[1].is_reference=false; }
        if fault == 2 { r.func_params.get_mut(&6).unwrap()[1].is_read_only=false; }
        if fault == 3 { r.func_params.get_mut(&6).unwrap()[1].type_info=12; }
        if fault == 4 { r.type_identity_by_ptr.get_mut(&11).unwrap().module="Script".into(); }
        if fault == 5 { r.type_identity_by_ptr.get_mut(&11).unwrap().namespace="Other".into(); }
        if fault == 6 { r.func_owner.insert(3,"FOther".into()); }
        if fault == 7 { r.func_ret.get_mut(&3).unwrap().token=0x44; }
        if fault == 8 { r.func_params.insert(3,vec![DataType { token:0x44,..Default::default() }]); }
        if fault == 9 { r.func_is_method.remove(&1); }
        if fault == 10 { r.func_ret.get_mut(&1).unwrap().token=0x44; }
        if fault == 11 { r.func_is_method.insert(6); }
        if fault == 12 { r.func_ret.get_mut(&6).unwrap().is_reference=true; }
        if fault == 13 { r.func_params.get_mut(&6).unwrap()[0].is_reference=true; }
        if fault == 14 { r.func_ns.insert(3,"Other".into()); }
        if fault == 15 { r.const_method_ptrs.insert(1); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_ordered_default_argument(wrong_type: bool) -> Self {
        let mut r = Self::default();
        for (ptr, name, owner) in [(1, "$beh0", "FContext"), (2, "$beh0", "FSpec"), (3, "$beh2", "FContext")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr);
            r.func_ret.insert(ptr, DataType { token: if wrong_type { 0x44 } else { 0x52 }, ..Default::default() });
            r.func_params.insert(ptr, if ptr == 2 { vec![DataType { token: 0x44, ..Default::default() }] } else { vec![] });
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_cleanup_integer_comparison(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(100, "FBox"), (200, "FTime")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (ptr, name, owner, ret) in [(1, "Adjust", "FBox", DataType { token: 5, type_info: 100, is_reference: true, ..Default::default() }),
            (2, "Count", "FBox", DataType { token: 0x44, ..Default::default() }),
            (3, "$beh2", "FBox", DataType { token: 0x52, ..Default::default() }),
            (4, "$beh2", "FTime", DataType { token: 0x52, ..Default::default() })] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr); r.func_ret.insert(ptr, ret); r.func_params.insert(ptr, Vec::new());
        }
        r.const_method_ptrs.insert(2);
        r.func_params.insert(1, vec![DataType { token: 5, type_info: 200, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() }]);
        r.funcid_to_ptr.insert(99, 99); r.func_by_ptr.insert(99, "Other".into());
        r.func_ret.insert(99, DataType { token: 0x41, ..Default::default() });
        r.func_params.insert(99, Vec::new()); r.func_ret_names.insert("Other".into(), "bool".into());
        if fault == 1 { r.func_ret.get_mut(&2).unwrap().token = 0x50; }
        if fault == 2 { r.func_ret.get_mut(&2).unwrap().is_reference = true; }
        if fault == 3 { r.const_method_ptrs.remove(&2); }
        if fault == 4 { r.func_by_ptr.insert(3, "Work".into()); }
        if fault == 5 { r.func_owner.insert(3, "FOther".into()); }
        if fault == 6 { r.func_params.insert(4, vec![DataType::default()]); }
        if fault == 7 { r.func_ret.get_mut(&1).unwrap().is_reference = false; }
        if fault == 8 { r.type_identity_by_ptr.get_mut(&100).unwrap().module = "Script".into(); }
        if fault == 9 { r.func_ret.remove(&2); }
        if fault == 10 { r.func_params.insert(2, vec![DataType::default()]); }
        if fault == 11 { r.func_params.get_mut(&1).unwrap()[0].type_info = 100; }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_retained_receiver(result_token: i32, is_method: bool) -> Self {
        let mut r = Self::default();
        for (id, name, token) in [(1, "Create", 5), (2, "Count", result_token),
            (3, "$beh2", 0x52), (4, "Act", 0x52)] {
            r.func_by_ptr.insert(id, name.into());
            r.func_ret.insert(id, DataType { token, ..Default::default() });
            r.func_params.insert(id, Vec::new());
        }
        r.func_owner.insert(2, "FValue".into());
        if is_method { r.func_is_method.insert(2); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_const_handle_field_comparison(saved: DataType, method: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("UCombat", "Controller"), ("UController", "Root"), ("URoot", "")]);
        for (id, name, ret) in [(101, "GetRootNode", saved), (102, "GetCombat",
            DataType { token: 5, type_info: 1, is_object_handle: true, ..Default::default() })]
        {
            let ptr = id as i64;
            r.funcid_to_ptr.insert(id, ptr);
            r.func_by_ptr.insert(ptr, name.into());
            r.func_params.insert(ptr, Vec::new());
            r.func_ret.insert(ptr, ret);
            if method { r.func_is_method.insert(ptr); }
            r.const_method_ptrs.insert(ptr);
        }
        r.set_class_fields(HashMap::from([
            ("UCombat".into(), HashMap::from([("Controller".into(), "UController".into())])),
            ("UController".into(), HashMap::from([("Root".into(), "URoot".into())])),
        ]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_pointer_comparison_call(ret: DataType) -> Self {
        let mut r = Self::from_test_collision_names(&["AActor", "APawn"]);
        r.func_by_ptr.insert(1, "GetPawn".into());
        r.func_params.insert(1, Vec::new());
        r.func_ret.insert(1, ret);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_field_return_lifetime(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FResult", "Severity"), ("UActor", "Level")]);
        for (id, name) in [(1, "FResult"), (2, "UActor"), (3, "FOther")] {
            r.type_by_ptr.insert(id, name.into());
            r.type_identity_by_ptr.insert(id, TypeIdentity {name: name.into(), module: "Fixture".into(), namespace: String::new()});
        }
        r.prop_type_id.insert(3, if fault == 1 {2} else {1});
        r.prop_type_id.insert(5, 2);
        r.set_class_fields(HashMap::from([
            ("FResult".into(), HashMap::from([("Severity".into(), "float".into())])),
            ("UActor".into(), HashMap::from([("Level".into(), "int".into())]))]));
        for (id, name) in [(10,"Evaluate"),(11,"Classify"),(12,"ShouldContinue"),(13,"~FResult"),(14,"Work")] {
            r.funcid_to_ptr.insert(id, id as i64); r.func_by_ptr.insert(id as i64, name.into());
            r.func_params.insert(id as i64, Vec::new()); r.func_is_method.insert(id as i64);
            r.func_owner.insert(id as i64, if id == 13 {if fault == 2 {"FOther"} else {"FResult"}} else {"UActor"}.into());
        }
        r.func_ret.insert(10, DataType {token:5,type_info:if fault == 3 {3} else {1},..Default::default()});
        r.func_ret.insert(11, DataType {token:0x44,..Default::default()});
        r.func_params.insert(11, vec![DataType {token:0x51,is_object_const:true,is_read_only:true,..Default::default()}]);
        r.func_ret.insert(12, DataType {token:if fault == 4 {0x44} else {0x41},..Default::default()});
        r.func_ret.insert(13, DataType {token:if fault == 5 {0x41} else {0x52},..Default::default()});
        r.func_ret.insert(14, DataType {token:0x52,..Default::default()});
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_switch_return_lifetime(fault: u8) -> Self {
        let mut r = Self::from_test_script_cleanup(if fault <= 5 {fault} else {0});
        for (id, name) in [(101,"FContext"),(102,"UActor"),(103,"FOther")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name:name.into(), module:"Fixture".into(), namespace:String::new() });
        }
        if fault == 6 { r.prop_type_id.insert((11 << 1) | (136i64 << 33) | 1, 12); r.typeid_to_ptr.insert(12,103); }
        if fault == 7 { r.set_class_fields(HashMap::from([("FContext".into(), HashMap::from([("Result".into(), "int".into())]))])); }
        r.funcid_to_ptr.insert(3,3); r.func_by_ptr.insert(3,"Respond".into());
        r.func_owner.insert(3,"UActor".into()); r.func_is_method.insert(3);
        r.func_params.insert(3,vec![DataType {token:0x41,..Default::default()}]);
        r.func_ret.insert(3,DataType {token:if fault == 8 {0x44} else {0x52},..Default::default()});
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_cleanup(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "FContext".into());
        r.type_by_ptr.insert(102, "UActor".into()); r.type_names.insert("FContext".into());
        r.type_by_ptr.insert(103, "FOther".into());
        r.typeid_to_ptr.insert(11, 101);
        r.prop_by_key.insert((11 << 1) | (136i64 << 33) | 1, "Result".into());
        r.prop_type_id.insert((11 << 1) | (136i64 << 33) | 1, 11);
        r.set_class_fields(HashMap::from([("FContext".into(), HashMap::from([("Result".into(), "EContextResult".into())]))]));
        for (id,name) in [(1,"ReadContext"),(2,"~FContext")] {
            r.funcid_to_ptr.insert(id,id as i64); r.func_by_ptr.insert(id as i64,name.into());
            r.func_params.insert(id as i64,Vec::new());
        }
        r.func_params.insert(1, vec![DataType { token:5, type_info:102,
            is_object_handle:true, is_object_const:true, ..Default::default() }; 2]);
        r.func_ret.insert(1,DataType {token:5,type_info:if fault == 5 {103} else {101},..Default::default()});
        r.func_ret.insert(2,DataType {token:if fault == 3 {0x41} else {0x52},..Default::default()});
        r.func_owner.insert(2,if fault == 2 {"Other"} else {"FContext"}.into());
        if fault != 1 {r.func_is_method.insert(2);}
        if fault == 4 {r.func_params.insert(2,vec![DataType::default()]);}
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_left_literal_carrier(fault: u8) -> Self {
        let mut r = Self::default();
        r.funcid_to_ptr.insert(1,1); r.func_by_ptr.insert(1,"GetAttack".into());
        r.func_ret.insert(1, DataType { token:5, type_info:if fault >= 4 {101} else {100}, is_object_handle:true,
            is_reference:fault == 1, ..Default::default() });
        r.func_params.insert(1, if fault == 2 { vec![DataType::default()] } else { vec![] });
        r.func_is_method.insert(1);
        r.typeid_to_ptr.insert(100,100);
        r.type_identity_by_ptr.insert(100, TypeIdentity { name:"UAttack".into(), module:String::new(), namespace:String::new() });
        if fault >= 4 {
            r.type_identity_by_ptr.insert(101, TypeIdentity { name:"USpecialAttack".into(), module:"Moves".into(),
                namespace:if fault == 6 {"Other"} else {""}.into() });
            r.class_super.insert("USpecialAttack".into(),if fault == 5 {"UOther"} else {"UAttack"}.into());
            r.class_super.insert("UAttack".into(),"UObject".into());
        }
        r.prop_by_key.insert((100 << 1) | 1,"Minimum".into());
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("UAttack","Minimum",if fault == 3 {"int"} else {"float"})], &[],None));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_result(ret: DataType) -> Self {
        let mut r = Self::default();
        r.funcid_to_ptr.insert(1, 1);
        r.func_by_ptr.insert(1, "Read".into());
        r.func_ret.insert(1, ret);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_handle_reader(fault: u8) -> Self {
        let mut r = Self::from_test_script_result(DataType { token: 5, type_info: 1,
            is_object_handle: true, is_reference: fault == 1, is_object_const: fault == 2,
            ..Default::default() });
        r.type_by_ptr.insert(1, "UValue".into());
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "UValue".into(),
            module: if fault == 3 { "Script" } else { "" }.into(), namespace: String::new() });
        if fault == 4 { r.func_is_method.insert(1); }
        r.func_params.insert(1, vec![DataType { token: 5, type_info: 2, is_object_handle: true,
            is_reference: fault == 5, is_object_const: fault != 6,
            is_read_only: fault == 7, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_nested_enum_index(key_const: bool) -> Self {
        let mut r = Self::default();
        for (id, name) in [(1, "USystem"), (2, "UContainer"), (3, "FResult"), (4, "EKind"), (5, "UWrong")] {
            let ptr = id as i64 + 100;
            r.typeid_to_ptr.insert(id, ptr);
            r.type_by_ptr.insert(ptr, name.into());
            r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity {
                module: "Synthetic".into(), namespace: String::new(), name: name.into(),
            });
        }
        for (id, name) in [(1, "Groups"), (2, "Roles"), (5, "Roles")] {
            let key = ((id as i64) << 1) | 1;
            r.prop_by_key.insert(key, name.into());
            r.prop_type_id.insert(key, id);
        }
        r.enum_entries.insert("EKind".into(), vec![("Spare".into(), 3)]);
        for (ptr, owner) in [(201, "TMap"), (202, "TArray")] {
            r.func_by_ptr.insert(ptr, "opIndex".into());
            r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr);
        }
        r.func_params.insert(201, vec![DataType { token: 5, type_info: 104,
            is_reference: true, is_object_const: key_const, is_read_only: key_const, ..Default::default() }]);
        r.func_params.insert(202, vec![DataType { token: 0x44, ..Default::default() }]);
        r.func_ret.insert(201, DataType { token: 5, type_info: 102,
            is_reference: true, is_object_handle: true, ..Default::default() });
        r.func_ret.insert(202, DataType { token: 5, type_info: 103,
            is_reference: true, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_script_default_copy() -> Self {
        let mut r = Self::from_test_script_constructors(&["Qualified"], "FPayload", &[]);
        r.typeid_to_ptr.extend([(101, 101), (102, 102)]);
        r.funcid_to_ptr.insert(2, 2);
        r.func_by_ptr.insert(2, "~FPayload".into());
        r.func_owner.insert(2, "FPayload".into());
        r.func_is_method.insert(2);
        r.func_params.insert(2, Vec::new());
        r.func_ret.insert(2, DataType { token: 0x52, ..Default::default() });
        r.type_identity_by_ptr.insert(102, TypeIdentity {
            name: "FPayload".into(), module: "Synthetic".into(), namespace: "Other".into(),
        });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_assignment_order_calls() -> Self {
        let mut r = Self::from_test_collision_names(&["UStorage"]);
        for (ptr, name, ret) in [
            (1, "MakeStorage", DataType { token: 5, type_info: 1, is_object_handle: true, ..Default::default() }),
            (2, "Now", DataType { token: 0x51, ..Default::default() }),
            (3, "At", DataType { token: 0x51, is_reference: true, ..Default::default() }),
        ] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_ret.insert(ptr, ret);
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_global_operator(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "FFlags".into());
        r.type_identity_by_ptr.insert(101, TypeIdentity { name: "FFlags".into(), module: String::new(),
            namespace: if fault == 6 { "Other".into() } else { String::new() } });
        for (ptr, ns, name) in [(10,"Checks","Combined"), (20,"Bits","First"), (21,"Bits","Second"),
            (22,"Bits","Third"), (23,"","Fourth")] {
            r.global_by_ptr.insert(ptr, name.into());
            if !ns.is_empty() { r.global_ns.insert(ptr, ns.into()); }
        }
        if fault == 7 { r.global_ns.insert(10, "Other".into()); }
        if fault == 5 { r.global_is_string.insert(20); }
        let value = DataType { token: 5, type_info: 101, ..Default::default() };
        let reference = DataType { is_reference: true, is_object_const: true, is_read_only: fault != 3, ..value.clone() };
        for (ptr, name) in [(1,"$beh0"), (2,"opOr"), (3,"$beh2")] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_owner.insert(ptr, if fault == 4 && ptr == 2 { "FOther" } else { "FFlags" }.into());
            if fault != 1 || ptr != 2 { r.func_is_method.insert(ptr); }
            r.func_params.insert(ptr, if ptr == 3 { vec![] } else { vec![reference.clone()] });
            r.func_ret.insert(ptr, if ptr == 2 { DataType { type_info: if fault == 2 { 102 } else { 101 }, ..value.clone() } }
                else { DataType { token: 0x52, ..Default::default() } });
        }
        r.funcid_to_ptr.insert(4, 1);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_fname_global_copy(source_namespace: &str, source_name: &str, copy_type: i64) -> Self {
        let mut r = Self::from_test_collision_names(&["FName", "FVector"]);
        r.global_by_ptr.insert(10, "Anywhere".into());
        r.global_ns.insert(10, "Location".into());
        r.global_by_ptr.insert(20, source_name.into());
        if !source_namespace.is_empty() { r.global_ns.insert(20, source_namespace.into()); }
        r.funcid_to_ptr.insert(30, 30);
        r.func_by_ptr.insert(30, "$beh0".into());
        r.func_owner.insert(30, "FName".into());
        r.func_params.insert(30, vec![DataType {
            token: 5, type_info: copy_type, is_reference: true,
            is_object_const: true, ..Default::default()
        }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_copied_native_int_conversion(fault: u8) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(1, "Choose".into()); r.func_ns.insert(1, "Math".into());
        for name in ["Use", "float"] {
            r.temporary_arg_positions.insert(name.into(), HashMap::from([(1, vec![true])]));
        }
        let int = DataType { token: 0x44, ..Default::default() };
        r.func_ret.insert(1, DataType { token: if fault == 1 { 0x51 } else { 0x44 },
            is_reference: fault == 2, is_object_handle: fault == 3, ..Default::default() });
        r.func_params.insert(1, vec![int.clone(), DataType { token: if fault == 4 { 0x45 } else { 0x44 },
            is_reference: fault == 5, ..Default::default() }]);
        if fault == 6 { r.func_is_method.insert(1); }
        if fault == 7 { r.func_owner.insert(1, "UOwner".into()); }
        if fault == 8 { r.func_params.get_mut(&1).unwrap().push(int); }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_float_getters_before_arguments(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(100, "UValues".into());
        r.type_identity_by_ptr.insert(100, TypeIdentity { name: "UValues".into(), module: String::new(), namespace: String::new() });
        for (ptr, name) in [(1, "GetLeft"), (2, "GetRight"), (3, "Blend")] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_ret.insert(ptr, DataType { token: 0x50, ..Default::default() });
            r.func_params.insert(ptr, if ptr == 3 { vec![DataType { token: 0x50, ..Default::default() }; 2] } else { vec![] });
            if ptr < 3 { r.func_is_method.insert(ptr); r.func_owner.insert(ptr, "UValues".into()); }
        }
        r.const_method_ptrs.insert(1); // The second getter need not be const.
        r.temporary_arg_positions.insert("Blend".into(), HashMap::from([(2, vec![true, true])]));
        match fault {
            1 => r.func_ret.get_mut(&1).unwrap().token = 0x51,
            2 => r.func_params.get_mut(&1).unwrap().push(DataType::default()),
            3 => { r.func_owner.insert(1, "UOther".into()); },
            4 => r.type_identity_by_ptr.get_mut(&100).unwrap().namespace = "Other".into(),
            5 => { r.func_params.get_mut(&3).unwrap().pop(); },
            6 => r.func_ret.get_mut(&3).unwrap().is_reference = true,
            7 => { r.func_is_method.insert(3); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_weak_member_null_comparison(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("AOwner", "First"), ("AOther", "")]);
        for (ptr, name) in [(1, "AOwner"), (2, "AOther"), (100, "TWeakObjectPtr"),
            (101, "TWeakObjectPtr"), (200, "UFirst"), (201, "USecond")] {
            r.type_by_ptr.insert(ptr, if fault == 13 && ptr == 100 { "TSoftObjectPtr" } else { name }.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: r.type_by_ptr[&ptr].clone(), namespace: String::new(),
                module: if fault == 10 && ptr == 1 { "Script" } else { "" }.into() });
        }
        r.prop_type_id.insert(3, if fault == 9 { 2 } else { 1 });
        let second = (1i64 << 1) | (8i64 << 33) | 1;
        r.prop_by_key.insert(second, "Second".into()); r.prop_type_id.insert(second, 1);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("AOwner", "First", if fault == 8 { "TWeakObjectPtr<UOther>" } else { "TWeakObjectPtr<UFirst>" }),
            ("AOwner", "Second", "TWeakObjectPtr<USecond>")], &[], None));
        for (ty, subtype, ctor, assign, equals) in [(100, 200, 10, 11, 12), (101, 201, 20, 21, 22)] {
            r.type_subtypes.insert(ty, vec![DataType { token: 5, type_info: subtype, ..Default::default() }]);
            for (ptr, name) in [(ctor, "$beh0"), (assign, "opAssign"), (equals, "opEquals")] {
                r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, "TWeakObjectPtr".into()); r.func_is_method.insert(ptr);
            }
            r.const_method_ptrs.insert(equals);
            r.func_params.insert(ctor, vec![]); r.func_ret.insert(ctor, DataType { token: 0x52, ..Default::default() });
            r.func_params.insert(assign, vec![DataType { token: 5, type_info: ty, is_reference: true,
                is_object_const: true, is_read_only: true, ..Default::default() }]);
            r.func_ret.insert(assign, DataType { token: 5, type_info: ty, is_reference: true, ..Default::default() });
            r.func_params.insert(equals, vec![DataType { token: 5, type_info: subtype, is_object_handle: true, ..Default::default() }]);
            r.func_ret.insert(equals, DataType { token: 0x41, ..Default::default() });
        }
        match fault {
            1 => r.func_params.get_mut(&11).unwrap()[0].type_info = 101,
            2 => r.func_params.get_mut(&11).unwrap()[0].is_object_const = false,
            3 => r.func_ret.get_mut(&11).unwrap().is_reference = false,
            4 => { r.const_method_ptrs.remove(&12); },
            5 => r.func_params.get_mut(&12).unwrap()[0].type_info = 201,
            6 => { r.func_params.get_mut(&10).unwrap().push(DataType { token: 0x44, ..Default::default() }); },
            7 => r.func_ret.get_mut(&10).unwrap().token = 0x41,
            11 => r.type_subtypes.get_mut(&100).unwrap()[0].type_info = 201,
            12 => { r.type_subtypes.remove(&100); },
            14 => { r.func_is_method.remove(&10); },
            _ => {},
        }
        r.func_by_ptr.insert(30, "Count".into());
        r.func_params.insert(30, vec![]);
        r.func_ret.insert(30, DataType { token: 0x44, ..Default::default() });
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_soft_member_null_comparison(fault: u8) -> Self {
        let mut r = Self::from_test_weak_member_null_comparison(0);
        r.type_by_ptr.insert(100, "TSoftObjectPtr".into());
        r.type_identity_by_ptr.get_mut(&100).unwrap().name = "TSoftObjectPtr".into();
        r.typeid_to_ptr.insert(3, 3); r.type_by_ptr.insert(3, "FNested".into());
        r.type_identity_by_ptr.insert(3, TypeIdentity { name: "FNested".into(), module: String::new(), namespace: String::new() });
        r.prop_by_key.insert(7, "Material".into()); r.prop_type_id.insert(7, 3);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("AOwner", "First", if fault == 3 { "FOther" } else { "FNested" }),
            ("FNested", "Material", if fault == 4 { "TSoftObjectPtr<USecond>" } else { "TSoftObjectPtr<UFirst>" })], &[], None));
        for ptr in [10, 11, 12, 13] { r.func_owner.insert(ptr, "TSoftObjectPtr".into()); }
        r.func_by_ptr.insert(13, "$beh2".into()); r.func_is_method.insert(13);
        r.func_params.insert(13, vec![]); r.func_ret.insert(13, DataType { token: 0x52, ..Default::default() });
        match fault {
            1 => r.type_identity_by_ptr.get_mut(&3).unwrap().module = "Script".into(),
            2 => { r.prop_type_id.insert(7, 2); },
            5 => { r.func_owner.insert(13, "FString".into()); },
            6 => { r.const_method_ptrs.insert(13); },
            7 => r.func_params.get_mut(&13).unwrap().push(DataType { token: 0x44, ..Default::default() }),
            8 => r.func_ret.get_mut(&13).unwrap().token = 0x41,
            9 => r.func_params.get_mut(&12).unwrap()[0].type_info = 201,
            10 => r.func_params.get_mut(&11).unwrap()[0].type_info = 101,
            11 => r.type_identity_by_ptr.get_mut(&100).unwrap().name = "TWeakObjectPtr".into(),
            12 => { r.type_subtypes.remove(&100); },
            13 => r.native = None,
            14 => { r.func_is_method.remove(&13); },
            15 => r.type_identity_by_ptr.get_mut(&3).unwrap().name = "ANested".into(),
            16 => r.type_identity_by_ptr.get_mut(&3).unwrap().namespace = "Other".into(),
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_field_product(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UBase", "First"), ("UDerived", "Second")]);
        for (ptr, name) in [(1, "UBase"), (2, "UDerived")] {
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: "Fixture".into(), namespace: String::new() });
            r.prop_type_id.insert((ptr << 1) | 1, ptr as i32);
        }
        r.set_class_fields(HashMap::from([
            ("UBase".into(), HashMap::from([("First".into(), if fault == 1 { "float32" } else { "float" }.into())])),
            ("UDerived".into(), HashMap::from([("Second".into(), if fault == 2 { "int64" } else { "float" }.into())]))]));
        r.func_by_ptr.insert(10, "Blend".into());
        r.func_ret.insert(10, DataType { token: 0x51, ..Default::default() });
        r.func_params.insert(10, vec![DataType { token: 0x51, ..Default::default() }; 2]);
        r.temporary_arg_positions.insert("Blend".into(), HashMap::from([(2, vec![true, true])]));
        match fault {
            3 => { r.prop_type_id.insert(3, 2); },
            4 => r.type_identity_by_ptr.get_mut(&1).unwrap().namespace = "Other".into(),
            5 => r.type_identity_by_ptr.get_mut(&1).unwrap().module.clear(),
            6 => r.func_params.get_mut(&10).unwrap()[0].is_reference = true,
            7 => r.func_ret.get_mut(&10).unwrap().token = 0x50,
            8 => { r.func_is_method.insert(10); },
            9 => { r.func_owner.insert(10, "FMath".into()); },
            10 => { r.func_params.get_mut(&10).unwrap().pop(); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_literal_receiver_before_name(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "FString"), (2, "FName")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (ptr, literal) in [(100, "First prefix"), (101, "Second prefix"), (102, "Third prefix")] {
            r.global_by_ptr.insert(ptr, literal.into()); r.global_is_string.insert(ptr);
        }
        for (ptr, name, owner) in [(10, "$beh0", "FString"), (11, "GetLabel", "UObject"),
            (12, "$beh0", "FString"), (13, "Append", "FString"), (14, "$beh2", "FString")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr); r.func_params.insert(ptr, Vec::new());
            r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        let input = |ty| DataType { token: 5, type_info: ty, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() };
        for (ptr, ty) in [(10, 1), (12, 2), (13, 1)] { r.func_params.insert(ptr, vec![input(ty)]); }
        r.func_ret.insert(11, DataType { token: 5, type_info: 2, ..Default::default() });
        r.func_ret.insert(13, DataType { token: 5, type_info: 1, is_reference: true, ..Default::default() });
        r.const_method_ptrs.insert(11); r.zero_arg_names.insert("GetLabel".into());
        r.func_ret_names.insert("GetLabel".into(), "FName".into());
        r.ctor_arg_positions.insert("FString".into(), HashMap::from([(1, vec![true])]));
        r.temporary_arg_positions.insert("Append".into(), HashMap::from([(1, vec![true])]));
        match fault {
            1 => r.global_is_string.clear(),
            2 => r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(),
            3 => r.type_identity_by_ptr.get_mut(&2).unwrap().namespace = "Other".into(),
            4 => r.func_params.get_mut(&10).unwrap()[0].type_info = 2,
            5 => r.func_params.get_mut(&12).unwrap()[0].is_reference = false,
            6 => { r.const_method_ptrs.remove(&11); },
            7 => r.func_ret.get_mut(&11).unwrap().is_reference = true,
            8 => { r.func_owner.insert(11, "FOther".into()); },
            9 => { r.func_by_ptr.insert(13, "Other".into()); },
            10 => r.func_ret.get_mut(&13).unwrap().is_reference = false,
            11 => r.func_params.get_mut(&13).unwrap()[0].type_info = 2,
            12 => { r.func_params.insert(14, vec![input(1)]); },
            13 => { r.func_owner.insert(14, "FName".into()); },
            14 => { r.const_method_ptrs.insert(10); },
            15 => { r.func_is_method.remove(&12); },
            16 => { r.func_params.insert(11, vec![input(2)]); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_nested_bool_copy(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(10, "Valid"), (11, "Allowed")] {
            r.func_by_ptr.insert(ptr, name.into()); r.funcid_to_ptr.insert(ptr as i32, ptr);
            r.func_ret.insert(ptr, DataType { token: 0x41, ..Default::default() });
            r.func_ret_names.insert(name.into(), "bool".into());
            r.func_params.insert(ptr, Vec::new());
        }
        r.func_params.insert(11, vec![DataType { token: 0x44, ..Default::default() }]);
        r.temporary_arg_positions.insert("Allowed".into(), HashMap::from([(1, vec![true])]));
        r.zero_arg_names.insert("Valid".into());
        match fault {
            1 => r.func_ret.get_mut(&10).unwrap().token = 0x44,
            2 => r.func_ret.get_mut(&11).unwrap().token = 0x44,
            3 => r.func_ret.get_mut(&11).unwrap().is_reference = true,
            4 => r.func_ret.get_mut(&10).unwrap().is_object_const = true,
            5 => r.func_ret.get_mut(&11).unwrap().type_info = 1,
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_int_field_predicate(old_owner: i32) -> Self {
        let mut r = Self::from_test_copied_int_field_read("int", false);
        r.typeid_to_ptr.insert(2, 2); r.type_by_ptr.insert(2, "UConfig".into());
        r.type_identity_by_ptr.insert(2, TypeIdentity { name: "UConfig".into(), module: "Foreign".into(), namespace: String::new() });
        r.prop_type_id.insert(3, old_owner);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_prepared_loot_arguments(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "AUnit"), (2, "UInventory"), (3, "TSet"), (4, "FTag")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (ptr, name, owner) in [(10, "Inventory", "AUnit"), (11, "Items", "UInventory"),
            (12, "Num", "TSet"), (13, "$beh2", "TSet"), (14, "Take", "ULooter"), (15, "Send", "UEvents")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr);
            r.func_params.insert(ptr, vec![]); r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
        }
        let integer = DataType { token: 0x44, ..Default::default() };
        let float = DataType { token: 0x50, ..Default::default() };
        let value = |ptr| DataType { token: 5, type_info: ptr, ..Default::default() };
        for (ptr, ret, name, ty) in [(10, DataType { is_object_handle: true, ..value(2) }, "Inventory", "UInventory"),
            (11, value(3), "Items", "TSet"), (12, integer.clone(), "Num", "int")] {
            r.func_ret.insert(ptr, ret); r.const_method_ptrs.insert(ptr);
            r.func_ret_names.insert(name.into(), ty.into()); r.zero_arg_names.insert(name.into());
        }
        r.func_params.insert(14, vec![DataType { is_object_handle: true, ..value(1) }, value(4), integer.clone()]);
        r.func_ret.insert(14, DataType { token: 0x41, ..Default::default() });
        r.func_params.insert(15, vec![float.clone(), float]);
        r.func_ret_names.insert("Take".into(), "bool".into());
        r.temporary_arg_positions.insert("Take".into(), HashMap::from([(3, vec![true; 3])]));
        r.temporary_arg_positions.insert("Send".into(), HashMap::from([(2, vec![true; 2])]));
        r.ctor_arg_positions.insert("float32".into(), HashMap::from([(1, vec![true])]));
        match fault {
            1 => r.type_identity_by_ptr.get_mut(&3).unwrap().module = "Script".into(),
            2 => r.func_ret.get_mut(&10).unwrap().type_info = 3,
            3 => r.func_ret.get_mut(&11).unwrap().is_reference = true,
            4 => r.func_ret.get_mut(&12).unwrap().token = 0x50,
            5 => { r.const_method_ptrs.remove(&12); },
            6 => { r.func_owner.insert(13, "FOther".into()); },
            7 => { r.func_params.insert(13, vec![integer]); },
            8 => r.func_params.get_mut(&14).unwrap()[2].is_reference = true,
            9 => r.func_params.get_mut(&14).unwrap()[2].token = 0x50,
            10 => { r.func_params.get_mut(&14).unwrap().pop(); },
            11 => { r.func_is_method.remove(&14); },
            12 => r.func_ret.get_mut(&14).unwrap().token = 0x44,
            13 => { r.func_params.insert(11, vec![integer]); },
            14 => { r.const_method_ptrs.remove(&10); },
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_context_null_and_double_seed(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "UObject"), (2, "UClass"), (3, "TSubclassOf"), (4, "FVector"),
            (5, "AController"), (6, "AGothicCharacter"), (7, "UMovementHost"), (8, "UQueryFilter")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(),
                module: if ptr == 7 { "Fixture" } else { "" }.into(), namespace: String::new() });
        }
        let value = |p| DataType { token: 5, type_info: p, ..Default::default() };
        let handle = |p| DataType { is_object_handle: true, ..value(p) };
        let vector = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value(4) };
        let void = DataType { token: 0x52, ..Default::default() };
        let double = DataType { token: 0x51, ..Default::default() };
        r.type_subtypes.insert(3, vec![value(8)]);
        for (ptr, name, owner, params, ret) in [
            (101, "$beh0", Some("TSubclassOf"), vec![handle(2)], void.clone()),
            (102, "Trace", None, vec![handle(1), vector.clone(), vector, DataType { is_reference: true, ..value(4) }, value(3), handle(5)], DataType { token: 0x41, ..Default::default() }),
            (103, "GetLocation", Some("AGothicCharacter"), vec![], value(4)),
            (104, "$beh0", Some("FVector"), vec![], void.clone()),
            (105, "Min", None, vec![double.clone(); 2], double.clone()),
            (106, "Notify", None, vec![], void.clone()),
            (107, "Use", None, vec![double], void),
        ] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_params.insert(ptr, params); r.func_ret.insert(ptr, ret);
            if let Some(owner) = owner { r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); }
        }
        r.const_method_ptrs.insert(103); r.func_ns.insert(102, "Nav".into()); r.func_ns.insert(105, "Math".into());
        r.global_by_ptr.insert(99, "__WorldContext".into());
        r.typeid_to_ptr.insert(11, 7); r.typeid_to_ptr.insert(12, 6);
        let key = (11 << 1) | (8i64 << 33) | 1;
        r.prop_by_key.insert(key, "Distance".into()); r.prop_type_id.insert(key, 11);
        r.set_class_fields(HashMap::from([("UMovementHost".into(), HashMap::from([("Distance".into(), "float".into())]))]));
        r.temporary_arg_positions.insert("Trace".into(), HashMap::from([(6, vec![true, true, true, false, true, true])]));
        r.temporary_arg_positions.insert("Use".into(), HashMap::from([(1, vec![true])]));
        match fault {
            1 => { r.global_by_ptr.insert(99, "OtherGlobal".into()); }
            2 => { r.func_owner.insert(101, "TOtherSubclass".into()); }
            3 => r.func_params.get_mut(&101).unwrap()[0].type_info = 1,
            4 => r.func_params.get_mut(&102).unwrap()[0].type_info = 6,
            5 => r.func_params.get_mut(&102).unwrap()[4].is_reference = true,
            6 => r.func_params.get_mut(&102).unwrap()[4].type_info = 4,
            7 => r.func_params.get_mut(&102).unwrap()[3].is_reference = false,
            8 => { r.func_is_method.insert(102); }
            9 => r.func_ret.get_mut(&102).unwrap().token = 0x44,
            10 => { r.func_ns.insert(102, "OtherNav".into()); }
            11 => r.type_identity_by_ptr.get_mut(&3).unwrap().module = "Script".into(),
            12 => { r.type_subtypes.insert(3, vec![value(1)]); }
            13 => r.func_params.get_mut(&103).unwrap().push(value(3)),
            14 => r.func_params.get_mut(&102).unwrap()[1].is_object_const = false,
            15 => r.func_params.get_mut(&102).unwrap()[5].type_info = 6,
            16 => r.func_ret.get_mut(&105).unwrap().token = 0x50,
            17 => r.func_params.get_mut(&105).unwrap()[1].is_reference = true,
            18 => { r.func_ns.insert(105, "OtherMath".into()); }
            19 => { r.class_fields.get_mut("UMovementHost").unwrap().insert("Distance".into(), "float32".into()); }
            20 => { r.prop_type_id.insert(key, 12); }
            21 => { r.type_identity_by_ptr.get_mut(&7).unwrap().namespace = "Other".into(); }
            22 => { r.func_is_method.remove(&103); }
            23 => { r.const_method_ptrs.remove(&103); }
            24 => r.func_params.get_mut(&103).unwrap().push(DataType { token: 0x51, ..Default::default() }),
            25 => r.func_ret.get_mut(&103).unwrap().token = 0x51,
            _ => {}
        }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_loop_seed_and_eager_comparison(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1, "FVector"), (2, "ACharacter"), (3, "TArray")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        let value = DataType { token: 5, type_info: 1, ..Default::default() };
        let input = DataType { is_reference: true, is_object_const: true, is_read_only: true, ..value.clone() };
        let integer = DataType { token: 0x44, ..Default::default() };
        for (ptr, name, owner, params, ret) in [
            (10, "GetLocation", "ACharacter", vec![], value),
            (20, "Classify", "UHost", vec![input.clone(), input.clone()], integer.clone()),
            (30, "IndexSide", "UHost", vec![DataType { token: 5, type_info: 2, is_object_handle: true, ..Default::default() }, input], integer.clone()),
            (40, "Num", "TArray", vec![], integer),
        ] {
            r.funcid_to_ptr.insert(ptr as i32, ptr); r.func_by_ptr.insert(ptr, name.into());
            r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); r.const_method_ptrs.insert(ptr);
            r.func_params.insert(ptr, params); r.func_ret.insert(ptr, ret);
        }
        for name in ["Classify", "IndexSide"] {
            r.temporary_arg_positions.insert(name.into(), HashMap::from([(2, vec![true; 2])]));
        }
        match fault {
            1 => { r.const_method_ptrs.remove(&10); }
            2 => r.func_ret.get_mut(&10).unwrap().is_reference = true,
            3 => { r.func_params.get_mut(&10).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
            4 => { r.func_owner.insert(10, "UOther".into()); }
            5 => r.type_identity_by_ptr.get_mut(&1).unwrap().module = "Script".into(),
            6 => r.type_identity_by_ptr.get_mut(&2).unwrap().namespace = "Other".into(),
            7 => { r.func_is_method.remove(&20); }
            8 => { r.const_method_ptrs.remove(&30); }
            9 => { r.func_owner.insert(30, "UOther".into()); }
            10 => { r.func_ns.insert(20, "Other".into()); }
            11 => r.func_ret.get_mut(&20).unwrap().token = 0x45,
            12 => r.func_ret.get_mut(&30).unwrap().is_reference = true,
            13 => r.func_params.get_mut(&20).unwrap()[0].is_read_only = false,
            14 => r.func_params.get_mut(&20).unwrap()[1].type_info = 2,
            15 => r.func_params.get_mut(&30).unwrap()[0].is_object_handle = false,
            16 => r.func_params.get_mut(&30).unwrap()[1].is_reference = false,
            17 => { r.func_params.get_mut(&30).unwrap().pop(); }
            _ => {}
        }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_enum_guard_and_unused_script_value(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name, module) in [(1, "FRecord", "Records"), (2, "EDisposition", ""), (3, "FQuery", ""), (4, "FRecord", "OtherRecords")] {
            r.type_by_ptr.insert(ptr, name.into()); r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: module.into(), namespace: String::new() });
        }
        let void = DataType { token: 0x52, ..Default::default() };
        for (ptr, name, owner, ret) in [
            (10, "Disposition", None, DataType { token: 5, type_info: 2, ..Default::default() }),
            (20, "FRecord", Some("FRecord"), void.clone()),
            (30, "~FRecord", Some("FRecord"), void.clone()),
            (40, "GetCount", Some("FQuery"), DataType { token: 0x44, ..Default::default() }),
            (41, "$beh2", Some("FQuery"), void.clone()), (50, "First", None, void.clone()), (51, "Second", None, void),
        ] {
            r.funcid_to_ptr.insert(ptr as i32, ptr); r.func_by_ptr.insert(ptr, name.into());
            r.func_params.insert(ptr, Vec::new()); r.func_ret.insert(ptr, ret);
            if let Some(owner) = owner { r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); }
        }
        r.script_ctor_owner.extend([(20, 1), (30, 1)]); r.const_method_ptrs.insert(40);
        r.ctor_arg_positions.insert("int".into(), HashMap::from([(1, vec![true])]));
        match fault {
            1 => r.func_ret.get_mut(&10).unwrap().token = 0x44,
            2 => r.func_ret.get_mut(&10).unwrap().is_reference = true,
            3 => r.func_ret.get_mut(&10).unwrap().is_object_handle = true,
            4 => r.func_ret.get_mut(&10).unwrap().is_object_const = true,
            5 => r.func_ret.get_mut(&10).unwrap().is_auto = true,
            6 => r.func_ret.get_mut(&10).unwrap().type_info = 3,
            7 => { r.func_is_method.insert(10); }
            8 => { r.script_ctor_owner.insert(20, 4); }
            9 => { r.script_ctor_owner.insert(30, 4); }
            10 => { r.func_by_ptr.insert(20, "OtherMethod".into()); }
            11 => { r.func_by_ptr.insert(30, "OtherMethod".into()); }
            12 => { r.func_is_method.remove(&20); }
            13 => { r.const_method_ptrs.insert(30); }
            14 => r.func_params.get_mut(&20).unwrap().push(DataType { token: 0x44, ..Default::default() }),
            15 => r.func_params.get_mut(&30).unwrap().push(DataType { token: 0x44, ..Default::default() }),
            16 => r.func_ret.get_mut(&20).unwrap().token = 0x41,
            17 => r.func_ret.get_mut(&30).unwrap().is_reference = true,
            18 => r.type_identity_by_ptr.get_mut(&1).unwrap().module.clear(),
            19 => { r.script_ctor_owner.remove(&30); }
            _ => {}
        }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_context_double_predicate(ty: &str, fault: u8) -> Self {
        let mut r = Self::from_test_copied_int_field_read(ty, false);
        r.typeid_to_ptr.insert(2, 2); r.type_by_ptr.insert(2, "UConfig".into());
        r.type_identity_by_ptr.insert(2, TypeIdentity { name: "UConfig".into(), module: "Foreign".into(), namespace: String::new() });
        match fault {
            1 => { r.prop_type_id.insert(3, 2); },
            2 => { r.type_identity_by_ptr.get_mut(&1).unwrap().module.clear(); },
            3 => { r.class_fields.get_mut("UConfig").unwrap().remove("Limit"); },
            4 => { r.prop_by_key.remove(&3); },
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_actor_factory(fault: u8) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(1,"AActor"), (2,"TSubclassOf"), (3,"FVector"), (4,"FRotator"), (5,"FName"), (6,"ULevel")] {
            r.type_by_ptr.insert(ptr, name.into());
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name:name.into(), module:String::new(), namespace:String::new() });
        }
        let value = |type_info| DataType { token:5,type_info,..Default::default() };
        let cref = |type_info| DataType { is_reference:true,is_object_const:true,is_read_only:true,..value(type_info) };
        r.type_subtypes.insert(2,vec![DataType { is_object_handle:true,..value(1) }]);
        r.func_by_ptr.insert(10,"MakeActor".into());
        r.func_ret.insert(10,DataType { is_object_handle:true,..value(1) });
        r.func_ret_names.insert("MakeActor".into(),"AActor".into());
        r.func_params.insert(10,vec![cref(2),cref(3),cref(4),cref(5),DataType { token:0x41,..Default::default() },
            DataType { is_object_handle:true,..value(6) }]);
        r.native=Some(crate::cache::binds::NativeApi::from_test_arities(&[("UTask","MakeActor",3)],&[("MakeActor",Some(3))]));
        match fault {
            1 => { r.func_is_method.insert(10); },
            2 => { r.func_params.get_mut(&10).unwrap().pop(); },
            3 => r.func_ret.get_mut(&10).unwrap().is_object_handle=false,
            4 => r.func_ret.get_mut(&10).unwrap().is_reference=true,
            5 => r.type_identity_by_ptr.get_mut(&1).unwrap().module="Script".into(),
            6 => r.type_identity_by_ptr.get_mut(&5).unwrap().namespace="Other".into(),
            7 => r.func_params.get_mut(&10).unwrap()[3].is_reference=false,
            8 => r.func_params.get_mut(&10).unwrap()[3].is_read_only=false,
            9 => r.func_params.get_mut(&10).unwrap()[3].is_object_const=false,
            10 => r.func_params.get_mut(&10).unwrap()[3].is_object_handle=true,
            11 => r.func_params.get_mut(&10).unwrap()[3].type_info=3,
            12 => r.func_params.get_mut(&10).unwrap()[4].token=0x44,
            13 => r.func_params.get_mut(&10).unwrap()[4].is_reference=true,
            14 => r.func_params.get_mut(&10).unwrap()[5].is_object_handle=false,
            15 => r.type_subtypes.get_mut(&2).unwrap()[0].type_info=6,
            16 => { r.type_subtypes.remove(&2); },
            17 => { r.func_owner.insert(10,"UTask".into()); r.func_is_method.insert(10); },
            18 => r.func_params.get_mut(&10).unwrap()[1].is_auto=true,
            19 => r.func_params.get_mut(&10).unwrap()[2].if_handle_then_const=true,
            20 => r.type_subtypes.get_mut(&2).unwrap()[0].is_object_handle=false,
            _ => {}
        }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_copied_bool_argument(token: i32) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(1, "SetFlag".into()); r.func_is_method.insert(1);
        r.func_ret.insert(1, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(1, vec![DataType { token, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_equality_lifetimes(fault: u8) -> Self {
        let mut r = Self::from_test_named_value_equality(if fault <= 4 { fault } else { 0 });
        r.type_identity_by_ptr.insert(100, TypeIdentity { name: "FValue".into(),
            module: String::new(), namespace: String::new() });
        r.global_by_ptr.insert(50, "Value".into());
        r.func_by_ptr.insert(2, "Create".into()); r.func_owner.insert(2, "UOwner".into());
        r.func_is_method.insert(2); r.const_method_ptrs.insert(2);
        r.func_params.insert(2, Vec::new());
        r.func_ret.insert(2, DataType { token: 5, type_info: 100, ..Default::default() });
        r.func_by_ptr.insert(3, "$beh2".into()); r.func_owner.insert(3, "FValue".into());
        r.func_is_method.insert(3); r.func_params.insert(3, Vec::new());
        r.func_ret.insert(3, DataType { token: 0x52, ..Default::default() });
        if fault == 5 { r.global_by_ptr.remove(&50); }
        if fault == 6 { r.func_owner.insert(3, "FOther".into()); }
        if fault == 7 { r.func_params.insert(3, vec![DataType { token: 0x41, ..Default::default() }]); }
        if fault == 8 { r.func_ret.get_mut(&2).unwrap().is_reference = true; }
        if fault == 9 { r.func_is_method.remove(&2); }
        if fault == 10 { r.const_method_ptrs.remove(&2); }
        if fault == 11 { r.func_params.remove(&1); }
        if fault == 12 { r.type_identity_by_ptr.get_mut(&100).unwrap().module = "Script".into(); }
        if fault == 13 { r.type_identity_by_ptr.get_mut(&100).unwrap().namespace = "Other".into(); }
        if fault == 14 { r.func_params.get_mut(&1).unwrap()[0].type_info = 200; }
        if fault == 15 { r.func_ret.get_mut(&1).unwrap().is_reference = true; }
        if fault == 16 { r.func_params.get_mut(&1).unwrap()[0].is_read_only = false; }
        if fault == 17 { r.func_ret.get_mut(&3).unwrap().token = 0x41; }
        if fault == 18 { r.func_is_method.remove(&1); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_value_equality(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(100, "FValue".into());
        r.func_by_ptr.insert(1, "opEquals".into());
        r.func_owner.insert(1, if fault == 1 { "FOther" } else { "FValue" }.into());
        r.func_is_method.insert(1);
        if fault != 2 { r.const_method_ptrs.insert(1); }
        r.func_ret.insert(1, DataType { token: if fault == 3 { 0x44 } else { 0x41 }, ..Default::default() });
        r.func_params.insert(1, vec![DataType { token: 5, type_info: 100,
            is_reference: true, is_object_const: fault != 4, is_read_only: true, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_enum_before_conditional_comparison(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("ERelation", ""), ("ARelationOwner", "")]);
        r.type_identity_by_ptr.insert(1, TypeIdentity { name: "ERelation".into(), module: String::new(), namespace: String::new() });
        let enumeration = DataType { token: 5, type_info: 1, ..Default::default() };
        let actor = DataType { token: 5, type_info: 2, is_object_handle: true, is_object_const: true, ..Default::default() };
        for (id, name, ret) in [(101, "GetRelation", enumeration), (102, "Ready", DataType { token: 0x41, ..Default::default() })] {
            r.funcid_to_ptr.insert(id, id as i64);
            r.func_by_ptr.insert(id as i64, name.into());
            r.func_ret.insert(id as i64, ret);
            r.func_params.insert(id as i64, vec![actor.clone()]);
        }
        r.func_owner.insert(101, "ARelationOwner".into());
        r.func_is_method.insert(101); r.const_method_ptrs.insert(101);
        r.func_ret_names.insert("GetRelation".into(), "ERelation".into());
        r.func_ret_names.insert("Ready".into(), "bool".into());
        r.temporary_arg_positions.insert("GetRelation".into(), HashMap::from([(1, vec![true])]));
        r.temporary_arg_positions.insert("Ready".into(), HashMap::from([(1, vec![true])]));
        r.ctor_arg_positions.insert("int".into(), HashMap::from([(1, vec![true])]));
        if fault == 1 { r.func_ret.get_mut(&101).unwrap().is_reference = true; }
        if fault == 2 { r.func_ret.get_mut(&101).unwrap().is_object_handle = true; }
        if fault == 3 { r.func_ret.get_mut(&101).unwrap().token = 0x44; }
        if fault == 4 { r.func_ret.get_mut(&101).unwrap().type_info = 2; }
        if fault == 5 { r.func_ret.get_mut(&102).unwrap().token = 0x44; }
        if fault == 6 { r.func_ret.get_mut(&102).unwrap().is_reference = true; }
        if fault == 7 { r.func_ret.remove(&101); }
        if fault == 8 { r.func_ret.get_mut(&101).unwrap().is_read_only = true; }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_segment_lifetimes(fault:u8)->Self {
        let mut r=Self::default();let vector=DataType {token:5,type_info:1,..Default::default()};
        let input=DataType {is_reference:true,is_object_const:true,is_read_only:true,..vector.clone()};
        let void=DataType {token:0x52,..Default::default()};let wide=DataType {token:0x51,..Default::default()};
        r.type_identity_by_ptr.insert(1,TypeIdentity {name:"FVector".into(),module:String::new(),namespace:String::new()});
        for (ptr,owner,name,constant,ret,args) in [(10,"FVector","$beh0",false,void.clone(),vec![input.clone()]),
            (11,"FVector","$beh0",false,void.clone(),vec![]),(12,"FVector","opMul",true,vector.clone(),vec![wide.clone()]),
            (13,"FVector","opAdd",true,vector.clone(),vec![input.clone()]),(14,"FVector","Distance",true,wide,vec![input]),
            (15,"AActor","GetActorLocation",true,vector.clone(),vec![]),(16,"AActor","GetActorForwardVector",true,vector.clone(),vec![]),
            (17,"AGothicCharacter","GetFeetLocation",true,vector.clone(),vec![]),(18,"AActor","GetActorUpVector",true,vector.clone(),vec![]),
            (19,"","FindNearestPointsOnLineSegments",false,void,(0..6).map(|i|DataType {is_reference:i>=4,..vector.clone()}).collect())] {
            r.func_by_ptr.insert(ptr,name.into());r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
            if !owner.is_empty(){r.func_owner.insert(ptr,owner.into());r.func_is_method.insert(ptr);}if constant{r.const_method_ptrs.insert(ptr);}
        }
        r.func_ns.insert(19,"Math".into());
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&1).unwrap().module="Script".into(),
            2=>r.func_params.get_mut(&10).unwrap()[0].is_reference=false,
            3=>r.func_params.get_mut(&19).unwrap()[4].is_reference=false,
            4=>{r.const_method_ptrs.remove(&12);}
            5=>r.func_ret.get_mut(&17).unwrap().is_object_handle=true,
            6=>{r.func_ns.insert(19,"Other".into());}
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_executor_bridge_lives(fault:u8)->Self {
        let mut r=Self::default();let task=DataType {token:5,type_info:20,..Default::default()};
        r.type_identity_by_ptr.insert(20,TypeIdentity {name:"FAbilityTaskExecutor".into(),module:String::new(),namespace:String::new()});
        for (id,name) in [(10,"WaitUntilCanMove"),(11,"WaitUntilNoLongerBlocked"),(12,"TurnToActor"),(13,"FindAndPickUpItem"),(14,"WasSuccessful")] {
            r.funcid_to_ptr.insert(id,id as i64);r.func_by_ptr.insert(id as i64,name.into());
            r.func_ret.insert(id as i64,if id==14{DataType {token:0x41,..Default::default()}}else{task.clone()});
            r.func_params.insert(id as i64,if id==14{vec![DataType {is_reference:true,is_object_const:true,is_read_only:true,..task.clone()}]}else{vec![]});
        }
        for (ptr,name,ret) in [(15,"$beh2",DataType {token:0x52,..Default::default()}),(16,"GetResult",DataType {token:5,type_info:21,is_reference:true,is_object_const:true,is_read_only:true,..Default::default()})] {
            r.func_by_ptr.insert(ptr,name.into());r.func_owner.insert(ptr,"FAbilityTaskExecutor".into());r.func_is_method.insert(ptr);r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,vec![]);
        }
        r.const_method_ptrs.insert(16);
        match fault {
            1=>r.func_ret.get_mut(&10).unwrap().is_reference=true,
            2=>{r.const_method_ptrs.insert(15);}
            3=>r.func_params.get_mut(&14).unwrap()[0].is_reference=false,
            4=>r.func_ret.get_mut(&16).unwrap().is_object_const=false,
            5=>r.type_identity_by_ptr.get_mut(&20).unwrap().module="Script".into(),
            6=>{r.func_is_method.insert(13);}
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_filter_copies(fault:u8) -> Self {
        let mut r=Self::default();
        for (ptr,name) in [(20,"FMemoryFilter"),(21,"FInGameTime")] {
            r.type_identity_by_ptr.insert(ptr,TypeIdentity {name:name.into(),module:String::new(),namespace:String::new()});
        }
        let filter=DataType {token:5,type_info:20,..Default::default()};
        let void=DataType {token:0x52,..Default::default()};
        for (ptr,owner,name,constant,ret,args) in [(10,"FMemoryFilter","$beh0",false,void.clone(),vec![DataType {is_reference:true,is_object_const:true,is_read_only:true,..filter.clone()}]),
            (11,"FMemoryFilter","$beh2",false,void.clone(),vec![]),(12,"FMemoryFilter","GetCount",true,DataType {token:0x44,..Default::default()},vec![]),
            (13,"FInGameTime","$beh2",false,void,vec![]),(14,"FMemoryFilter","Affecting",false,DataType {is_reference:true,..filter.clone()},vec![]),
            (15,"FMemoryFilter","AfterTime",false,DataType {is_reference:true,..filter},vec![])] {
            r.func_by_ptr.insert(ptr,name.into());r.func_owner.insert(ptr,owner.into());r.func_is_method.insert(ptr);r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
            if constant{r.const_method_ptrs.insert(ptr);}
        }
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&20).unwrap().module="Script".into(),
            2=>r.func_params.get_mut(&10).unwrap()[0].is_read_only=false,
            3=>{r.const_method_ptrs.remove(&12);}
            4=>r.func_ret.get_mut(&14).unwrap().is_reference=false,
            5=>{r.func_owner.insert(13,"Other".into());}
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_named_set_return(fault:u8) -> Self {
        let mut r=Self::default();
        for (ptr,name) in [(1,"TSet"),(2,"FGameplayTag"),(3,"UOwner"),(4,"AState")] {
            r.type_by_ptr.insert(ptr,name.into());r.type_names.insert(name.into());
            r.type_identity_by_ptr.insert(ptr,TypeIdentity {name:name.into(),module:String::new(),namespace:String::new()});
        }
        let value=|ty|DataType {token:5,type_info:ty,..Default::default()};
        let reference=|ty|DataType {is_reference:true,is_object_const:true,is_read_only:true,..value(ty)};
        r.type_subtypes.insert(1,vec![DataType {is_object_handle:true,..value(4)}]);
        let void=DataType {token:0x52,..Default::default()};
        for (p,owner,name,constant,ret,args) in [(10,"FGameplayTag","IsValid",true,DataType {token:0x41,..Default::default()},vec![]),
            (11,"TSet","$beh0",false,void.clone(),vec![]),(12,"TSet","opAssign",false,DataType {is_reference:true,..value(1)},vec![reference(1)]),
            (13,"TSet","$beh2",false,void,vec![]),(14,"","Get",false,DataType {is_object_handle:true,..value(3)},vec![]),
            (15,"UOwner","GetOwners",true,value(1),vec![reference(2)])] {
            r.func_by_ptr.insert(p,name.into());r.func_ret.insert(p,ret);r.func_params.insert(p,args);
            if !owner.is_empty(){r.func_owner.insert(p,owner.into());r.func_is_method.insert(p);}if constant{r.const_method_ptrs.insert(p);}
        }
        r.func_ns.insert(14,"UOwner".into());
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&1).unwrap().module="Script".into(),
            2=>r.func_ret.get_mut(&15).unwrap().type_info=2,
            3=>r.func_params.get_mut(&12).unwrap()[0].is_read_only=false,
            4=>{r.const_method_ptrs.remove(&15);}
            5=>{r.func_ns.insert(14,"Other".into());}
            6=>r.func_ret.get_mut(&14).unwrap().is_reference=true,
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_loop_deadline(fault:u8) -> Self {
        let mut r=Self::from_test_member_chain(&[("UState","Duration"),("FInGameTime",""),("UObject","")]);
        for (id,name,module) in [(1,"UState","Fixture"),(2,"FInGameTime",""),(3,"UObject","")] {
            r.type_identity_by_ptr.insert(id,TypeIdentity {name:name.into(),module:module.into(),namespace:String::new()});
        }
        r.prop_type_id.insert(3,1);r.class_fields.insert("UState".into(),HashMap::from([("Duration".into(),"float".into())]));
        r.global_by_ptr.insert(9,"__WorldContext".into());
        let context=DataType {token:5,type_info:3,is_object_handle:true,is_object_const:true,..Default::default()};
        let time=DataType {token:5,type_info:2,..Default::default()};let boolean=DataType {token:0x41,..Default::default()};let void=DataType {token:0x52,..Default::default()};
        for (ptr,owner,name,constant,ret,args) in [(10,"","XRealtimeSecondsFromNow",false,time,vec![context.clone(),DataType {token:0x50,..Default::default()}]),
            (11,"FInGameTime","IsTimeInThePast",true,boolean.clone(),vec![context]),(12,"UAbilityTaskCoroutine","WaitOneTick",false,void.clone(),vec![]),
            (13,"FInGameTime","$beh2",false,void,vec![]),(14,"UState","Continue",false,boolean,vec![])] {
            r.func_by_ptr.insert(ptr,name.into());r.func_ret.insert(ptr,ret);r.func_params.insert(ptr,args);
            if !owner.is_empty(){r.func_owner.insert(ptr,owner.into());r.func_is_method.insert(ptr);}if constant{r.const_method_ptrs.insert(ptr);}
        }
        r.func_ns.insert(10,"FInGameTime".into());r.funcid_to_ptr.insert(14,14);
        match fault {
            1=>r.type_identity_by_ptr.get_mut(&2).unwrap().module="Script".into(),
            2=>r.func_params.get_mut(&10).unwrap()[1].token=0x51,
            3=>{r.class_fields.get_mut("UState").unwrap().insert("Duration".into(),"float32".into());}
            4=>{r.const_method_ptrs.remove(&11);}
            5=>r.func_ret.get_mut(&14).unwrap().token=0x44,
            6=>{r.func_owner.insert(13,"FOther".into());}
            7=>{r.global_by_ptr.insert(9,"OtherContext".into());}
            _=>{}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_void_guard_bool(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UState", "Early"), ("UState", "Visible")]);
        for id in [1,2] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name:"UState".into(), module:"Fixture".into(), namespace:String::new() });
            r.prop_type_id.insert((id << 1) | 1, id as i32);
        }
        r.class_fields.insert("UState".into(), HashMap::from([("Early".into(),"bool".into()),("Visible".into(),"bool".into())]));
        for (p,name) in [(101,"Saved"),(102,"Other")] {
            r.func_by_ptr.insert(p,name.into()); r.func_ret.insert(p,DataType {token:0x41,..Default::default()});
        }
        match fault {
            1 => r.func_ret.get_mut(&101).unwrap().token=0x44,
            2 => r.func_ret.get_mut(&102).unwrap().is_reference=true,
            3 => { r.class_fields.get_mut("UState").unwrap().insert("Visible".into(),"int".into()); }
            4 => { r.prop_type_id.insert(5,9); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_eager_bool_calls(saved_token: i32) -> Self {
        let mut r = Self::default();
        for (id, name, token) in [(101, "Saved", saved_token), (102, "Other", 0x41)] {
            r.funcid_to_ptr.insert(id, id as i64);
            r.func_by_ptr.insert(id as i64, name.into());
            r.func_ret.insert(id as i64, DataType { token, ..Default::default() });
            r.func_params.insert(id as i64, vec![]);
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_eager_negated_cleanup_bool(fault: u8) -> Self {
        let mut r = Self::from_test_eager_bool_calls(0x41);
        r.type_by_ptr.insert(100, "FBox".into()); r.type_names.insert("FBox".into());
        r.type_identity_by_ptr.insert(100, TypeIdentity { name: "FBox".into(),
            module: if fault == 6 { "Script" } else { "" }.into(), namespace: String::new() });
        for (ptr, name, token) in [(103, "IsEmpty", 0x41), (104, "$beh2", 0x52),
            (105, "Container", 5), (106, "Work", 0x52)] {
            r.func_by_ptr.insert(ptr, name.into()); r.funcid_to_ptr.insert(ptr as i32, ptr);
            r.func_ret.insert(ptr, DataType { token, type_info: if ptr == 105 { 100 } else { 0 }, ..Default::default() });
            r.func_params.insert(ptr, Vec::new());
        }
        for ptr in [103, 104] { r.func_owner.insert(ptr, "FBox".into()); r.func_is_method.insert(ptr); }
        r.const_method_ptrs.insert(103);
        if fault == 1 { r.func_ret.get_mut(&103).unwrap().token = 0x44; }
        if fault == 2 { r.func_ret.get_mut(&103).unwrap().is_reference = true; }
        if fault == 3 { r.func_by_ptr.insert(104, "Observe".into()); }
        if fault == 4 { r.func_owner.insert(104, "FOther".into()); }
        if fault == 5 { r.func_params.insert(104, vec![DataType::default()]); }
        if fault == 7 { r.func_params.insert(103, vec![DataType::default()]); }
        if fault == 8 { r.func_owner.insert(103, "FOther".into()); }
        if fault == 9 { r.const_method_ptrs.remove(&103); }
        if fault == 10 { r.func_ret.get_mut(&104).unwrap().token = 0x41; }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_terminal_return_arithmetic(narrow: bool) -> Self {
        let mut r = Self::default();
        let wide = DataType { token: 0x51, ..Default::default() };
        for (ptr, name, argc) in [(1, "Size", 0), (2, "DistanceSquared", 0),
            (3, "Projection", 0), (4, "Clamp", 3), (5, "Consume", 1)] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_ret.insert(ptr, if narrow && ptr == 2 {
                DataType { token: 0x50, ..Default::default() }
            } else { wide.clone() });
            r.func_params.insert(ptr, vec![wide.clone(); argc]);
        }
        r.func_ns.insert(4, "Math".into());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_terminal_clamp_assignment(fault: u8) -> Self {
        let mut r = Self::from_test_terminal_return_arithmetic(false);
        match fault {
            1 => r.func_ret.get_mut(&3).unwrap().token = 0x50,
            2 => r.func_ret.get_mut(&4).unwrap().token = 0x50,
            3 => r.func_params.get_mut(&4).unwrap()[0].is_reference = true,
            4 => r.func_params.get_mut(&4).unwrap().truncate(2),
            5 => { r.func_ns.insert(4, "Other".into()); },
            6 => { r.func_by_ptr.insert(4, "Other".into()); },
            _ => {},
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reference_copy_initializers(by_ref: bool, same_type: bool) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "FBox".into());
        r.type_by_ptr.insert(102, "FOther".into());
        for (ptr, name) in [(1, "First"), (3, "Second")] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_ret.insert(ptr, DataType { token: 5, type_info: 101, is_reference: by_ref, ..Default::default() });
        }
        r.func_by_ptr.insert(2, "$beh0".into());
        r.func_owner.insert(2, "FBox".into());
        r.func_is_method.insert(2);
        r.func_ret.insert(2, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(2, vec![DataType { token: 5, type_info: if same_type { 101 } else { 102 },
            is_reference: true, is_object_const: true, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_map_reference_binding() -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "TMapIterator".into());
        r.type_by_ptr.insert(102, "AGothicCharacterState".into());
        for (ptr, name, owner) in [(1, "Proceed", "TMapIterator"), (2, "Iterator", "TMap"),
            (3, "GetKey", "TMapIterator")]
        {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr); r.func_params.insert(ptr, Vec::new());
        }
        r.func_ret.insert(1, DataType { token: 5, type_info: 101, is_reference: true, ..Default::default() });
        r.func_ret.insert(2, DataType { token: 5, type_info: 101, ..Default::default() });
        r.func_ret.insert(3, DataType { token: 5, type_info: 102, is_reference: true,
            is_object_handle: true, is_read_only: true, ..Default::default() });
        r.const_method_ptrs.insert(3);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_iterator_reference_before_argument(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "TSetIterator".into());
        r.type_by_ptr.insert(102, "TSubclassOf<UTask>".into());
        r.type_by_ptr.insert(103, "UEntry".into());
        for (id, name, owner) in [(1, "Proceed", "TSetIterator"), (2, "Iterator", "TSet"),
            (3, "opIndex", "TMap"), (4, "Use", "AOwner")]
        {
            r.func_by_ptr.insert(id, name.into()); r.func_owner.insert(id, owner.into());
            r.func_is_method.insert(id); r.func_params.insert(id, Vec::new());
        }
        let value = DataType { token: 5, type_info: 102, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() };
        r.func_ret.insert(1, value.clone());
        r.func_ret.insert(2, DataType { token: 5, type_info: 101, ..Default::default() });
        r.func_ret.insert(3, DataType { token: 5, type_info: 103,
            is_reference: true, is_object_handle: true, ..Default::default() });
        r.func_params.insert(3, vec![value.clone()]);
        r.func_ret.insert(4, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(4, vec![value, DataType { token: 0x51,
            is_object_const: true, is_read_only: true, ..Default::default() }]);
        r.funcid_to_ptr.insert(4, 4);
        r.temporary_arg_positions.insert("Use".into(), [(2, vec![true, true])].into_iter().collect());
        if fault == 1 { r.func_ret.get_mut(&1).unwrap().is_reference = false; }
        if fault == 2 { r.func_ret.get_mut(&1).unwrap().is_object_const = false; }
        if fault == 3 { r.func_params.get_mut(&4).unwrap()[0].type_info = 103; }
        if fault == 4 { r.func_params.get_mut(&4).unwrap()[1].token = 0x50; }
        if fault == 5 { r.func_is_method.remove(&1); }
        if fault == 6 { r.func_owner.insert(1, "TOtherIterator".into()); }
        if fault == 7 { r.func_ret.get_mut(&3).unwrap().is_reference = false; }
        if fault == 8 { r.func_is_method.remove(&4); }
        if fault == 9 { r.func_params.get_mut(&4).unwrap()[0].is_read_only = false; }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_mutable_f32_iterator(ret: DataType, method: bool) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "TArrayIterator".into());
        for (ptr, name, owner) in [(1, "Proceed", "TArrayIterator"), (2, "Iterator", "TArray")] {
            r.func_by_ptr.insert(ptr, name.into());
            r.func_owner.insert(ptr, owner.into());
            r.func_params.insert(ptr, Vec::new());
            if method { r.func_is_method.insert(ptr); }
        }
        r.func_ret.insert(1, ret);
        r.func_ret.insert(2, DataType { token: 5, type_info: 101, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_handle_getter_argument(ret: DataType, param: DataType) -> Self {
        let mut r = Self::from_test_member_chain(&[("UHolder", "Nodes")]);
        r.type_by_ptr.insert(101, "UNode".into());
        r.type_by_ptr.insert(102, "UOther".into());
        for (ptr, name) in [(1, "opIndex"), (2, "Remove"), (3, "Contains")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, "TMap".into());
            r.func_is_method.insert(ptr);
            r.func_ret.insert(ptr, if ptr == 1 { ret.clone() }
                else { DataType { token: 0x41, ..Default::default() } });
            r.func_params.insert(ptr, vec![if ptr == 3 { param.clone() }
                else { DataType { token: 5, type_info: 101, is_reference: true,
                    is_object_handle: true, is_object_const: true, is_read_only: true, ..Default::default() } }]);
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_typed_getter_copies(field_type: &str, object_const: bool, wrong_owner: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("FGroup", "MemberHandles"),
            ("FMember", "CharacterState"), ("UNode", "Target"), ("TArray", ""), ("FName", "")]);
        for (id, name) in [(1, "FGroup"), (2, "FMember"), (3, "UNode")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { module: "Fixture".into(),
                namespace: String::new(), name: name.into() });
            if id <= 2 { r.prop_type_id.insert((id << 1) | 1, id as i32); }
        }
        r.prop_type_id.insert((3 << 1) | 1, if wrong_owner { 1 } else { 3 });
        if wrong_owner { r.prop_type_id.insert((2 << 1) | 1, 1); }
        r.set_class_fields(HashMap::from([
            ("FGroup".into(), HashMap::from([("MemberHandles".into(), "TArray<FMember>".into())])),
            ("UNode".into(), HashMap::from([("Target".into(), "UNode".into())])),
            ("FMember".into(), HashMap::from([("CharacterState".into(), field_type.into())])),
        ]));
        for (ptr, name, ty, handle) in [(1, "opIndex", 1, false),
            (2, "opIndex", 2, false), (3, "Last", 3, true), (6, "opIndex", 3, true)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, "TArray".into());
            r.func_is_method.insert(ptr);
            r.func_params.insert(ptr, vec![DataType { token: 0x44, ..Default::default() }]);
            r.func_ret.insert(ptr, DataType { token: 5, type_info: ty, is_reference: true,
                is_object_handle: handle, is_object_const: object_const, ..Default::default() });
        }
        r.func_by_ptr.insert(7, "opIndex".into()); r.func_owner.insert(7, "TMap".into());
        r.func_is_method.insert(7);
        r.func_params.insert(7, vec![DataType { token: 5, type_info: 3, is_reference: true,
            is_object_handle: true, is_object_const: true, is_read_only: true, ..Default::default() }]);
        r.func_ret.insert(7, DataType { token: 5, type_info: 2, is_reference: true,
            is_object_const: object_const, ..Default::default() });
        r.func_by_ptr.insert(4, "IsValid".into());
        r.func_params.insert(4, vec![DataType { token: 5, type_info: 3,
            is_object_handle: true, ..Default::default() }]);
        r.func_ret.insert(4, DataType { token: 0x41, ..Default::default() });
        r.func_by_ptr.insert(5, "__STATIC_NAME".into());
        r.func_params.insert(5, vec![DataType { token: 0x44, ..Default::default() }]);
        r.func_ret.insert(5, DataType { token: 5, type_info: 5, is_reference: true,
            is_object_const: true, is_read_only: true, ..Default::default() });
        r.static_names.push("None".into());
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_guarded_handle_parameter_read(argument: DataType) -> Self {
        let mut r = Self::from_test_typed_getter_copies("UNode", false, false);
        r.funcid_to_ptr.insert(1, 4);
        r.func_params.insert(4, vec![argument]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_destroyed_enum_member(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FEvaluation", "Kind"), ("UEvaluator", ""),
            ("ERelation", ""), ("AActor", "")]);
        for (ptr, name) in [(1, "FEvaluation"), (2, "UEvaluator"), (3, "ERelation"), (4, "AActor")] {
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { name: name.into(), module: String::new(),
                namespace: if fault == 5 && ptr == 3 { "Other" } else { "" }.into() });
        }
        r.prop_type_id.insert(3, if fault == 1 { 2 } else { 1 });
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("FEvaluation", "Kind", if fault == 2 { "EOther" } else { "ERelation" })], &[], None));
        for (ptr, name) in [(10, "Acquire"), (11, "Evaluate"), (12, "$beh2")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_params.insert(ptr, Vec::new());
        }
        r.func_is_method.extend([11, 12]); r.const_method_ptrs.insert(11);
        r.func_owner.insert(11, if fault == 6 { "UOther" } else { "UEvaluator" }.into());
        r.func_owner.insert(12, if fault == 3 { "FOther" } else { "FEvaluation" }.into());
        r.func_ret.insert(10, DataType { token: 5, type_info: 2, is_object_handle: true, ..Default::default() });
        r.func_ret.insert(11, DataType { token: 5, type_info: if fault == 4 { 2 } else { 1 },
            is_reference: fault == 7, ..Default::default() });
        r.func_ret.insert(12, DataType { token: if fault == 8 { 0x44 } else { 0x52 }, ..Default::default() });
        if fault == 9 { r.func_params.get_mut(&12).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
        if fault == 10 { r.func_is_method.insert(10); }
        if fault == 11 { r.duplicate_prop_keys.insert(3); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_ordered_parameter_equality(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FInput", "Field"), ("TWeakValue", ""),
            ("AActor", ""), ("AGothicCharacter", "")]);
        for (id, name) in [(1, "FInput"), (2, "TWeakValue"), (3, "AActor"), (4, "AGothicCharacter")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: String::new(),
                namespace: if fault == 5 && id == 4 { "Other" } else { "" }.into() });
        }
        if fault == 6 {
            r.type_by_ptr.insert(3, "UObject".into());
            r.type_identity_by_ptr.get_mut(&3).unwrap().name = "UObject".into();
        }
        r.prop_type_id.insert(3, if fault == 1 { 2 } else { 1 });
        r.type_subtypes.insert(2, vec![DataType { token: 5, type_info: 3, ..Default::default() }]);
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("FInput", "Field", if fault == 2 { "FOther" } else { "TWeakValue<AActor>" })], &[], None));
        for (id, name, owner) in [(1, "Acquire", "FInput"), (2, "opEquals", "TWeakValue")] {
            r.func_by_ptr.insert(id, name.into()); r.func_owner.insert(id, owner.into());
            r.func_is_method.insert(id); r.const_method_ptrs.insert(id);
        }
        if fault == 3 { r.const_method_ptrs.remove(&2); }
        r.func_ret.insert(1, DataType { token: 5, type_info: 4, is_object_handle: true, ..Default::default() });
        r.func_params.insert(1, Vec::new());
        r.func_ret.insert(2, DataType { token: 0x41, ..Default::default() });
        r.func_params.insert(2, vec![DataType { token: 5, type_info: 3, is_object_handle: true,
            is_reference: fault == 4, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_cast_member_assignment(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UConfig", "Override"), ("ABaseProjectile", "Collision"),
            ("UCollision", "Weapon"), ("TSubclassOf", ""), ("UWeapon", ""), ("USpecialWeapon", ""),
            ("AScriptProjectile", ""), ("UObject", "")]);
        for (id, name) in [(1, "UConfig"), (2, "ABaseProjectile"), (3, "UCollision"), (4, "TSubclassOf"),
            (5, "UWeapon"), (6, "USpecialWeapon"), (7, "AScriptProjectile"), (8, "UObject")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(),
                module: if matches!(id, 1 | 7) { "Fixture" } else { "" }.into(),
                namespace: if fault == 7 && id == 2 { "Wrong" } else { "" }.into() });
        }
        for id in [1i64, 2, 3] { r.prop_type_id.insert((id << 1) | 1,
            if fault == 5 && id == 2 { 3 } else { id as i32 }); }
        r.type_subtypes.insert(4, vec![DataType { token: 5, type_info: 5, ..Default::default() }]);
        r.typeid_to_ptr.insert(0x0800_0007, 7);
        r.class_super.insert("AScriptProjectile".into(), "ABaseProjectile".into());
        r.set_class_fields(HashMap::from([("UConfig".into(), HashMap::from([
            ("Override".into(), "TSubclassOf<USpecialWeapon>".into())]))]));
        r.set_native_api(super::binds::NativeApi::from_test_field_types(&[
            ("ABaseProjectile", "Collision", if fault == 6 { "UOther" } else { "UCollision" }),
            ("UCollision", "Weapon", if fault == 4 { "TSubclassOf<USpecialWeapon>" } else { "TSubclassOf<UWeapon>" }),
        ], &[], None));
        for (ptr, name, owner) in [(1, "opCast", "UObject"), (2, "opAssign", "TSubclassOf")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr,
                if fault == 8 && ptr == 1 { "UOther" } else { owner }.into());
            r.func_is_method.insert(ptr);
        }
        r.const_method_ptrs.insert(1);
        if fault == 3 { r.const_method_ptrs.insert(2); }
        r.func_ret.insert(1, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(1, vec![DataType { token: 0x3b, is_reference: true, ..Default::default() }]);
        r.func_ret.insert(2, DataType { token: 5, type_info: 4, is_reference: true, ..Default::default() });
        r.func_params.insert(2, vec![DataType { token: 5, type_info: if fault == 1 { 6 } else { 4 },
            is_reference: true, is_object_const: true, is_read_only: fault != 2, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_cast_value_frame(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("FString", ""), ("TSubclassOf", ""),
            ("UClass", ""), ("UObject", ""), ("UResponse", ""), ("UOwner", ""), ("UBaseResponse", "")]);
        for (id, name) in [(1, "FString"), (2, "TSubclassOf"), (3, "UClass"), (4, "UObject"), (5, "UResponse"), (6, "UOwner"), (7, "UBaseResponse")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if id >= 5 { "Fixture" } else { "" }.into() });
        }
        r.typeid_to_ptr.insert(0x0800_0005, 5);
        r.set_class_hierarchy(HashMap::from([("UResponse".into(), "UBaseResponse".into())]));
        for (ptr, name, owner, result, handle) in [(1, "Get", "TSubclassOf", 3, true),
            (2, "GetDefaultObject", "UClass", 4, true), (3, "opCast", "UObject", 0, false),
            (11, "GetClass", "UOwner", 2, false), (12, "GetDisplayName", "UBaseResponse", 1, false)] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            r.func_is_method.insert(ptr); r.const_method_ptrs.insert(ptr);
            r.func_params.insert(ptr, Vec::new());
            r.func_ret.insert(ptr, DataType { token: if result == 0 { 0x52 } else { 5 },
                type_info: result, is_object_handle: handle, ..Default::default() });
            if ptr >= 11 { r.funcid_to_ptr.insert(ptr as i32, ptr); }
        }
        r.func_params.insert(3, vec![DataType { token: 0x3b, is_reference: true, ..Default::default() }]);
        match fault {
            1 => { r.func_ret.get_mut(&12).unwrap().is_reference = true; }
            2 => { r.func_owner.insert(12, "UOther".into()); }
            3 => { r.const_method_ptrs.remove(&1); }
            4 => { r.func_params.get_mut(&11).unwrap().push(DataType { token: 0x44, ..Default::default() }); }
            5 => { r.func_params.get_mut(&3).unwrap()[0].is_read_only = true; }
            6 => { r.func_ret.get_mut(&2).unwrap().is_object_handle = false; }
            7 => { r.set_class_hierarchy(HashMap::new()); }
            _ => {}
        }
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_cast_field_return(subclass: bool, fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UAttack", "Required"), ("FTag", ""),
            ("TSubclassOf", ""), ("UWeapon", ""), ("FRecord", "Weighted"), ("FWeighted", "Move"), ("UBase", "")]);
        for (id, name) in [(1, "UAttack"), (2, "FTag"), (3, "TSubclassOf"), (4, "UWeapon"),
            (5, "FRecord"), (6, "FWeighted"), (7, "UBase")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { module: if id == 1 { "Fixture" } else { "" }.into(),
                namespace: String::new(), name: name.into() });
        }
        r.prop_type_id.insert(3, if fault == 1 { 7 } else { 1 });
        r.type_subtypes.insert(3, vec![DataType { token: 5, type_info: 4, is_object_handle: true, ..Default::default() }]);
        let result = if subclass { 3 } else { 2 };
        for (ptr, name, owner) in [(9, "opCast", "UObject"),
            (10, if subclass { "opAssign" } else { "$beh0" }, if subclass { "TSubclassOf" } else { "FTag" }),
            (11, "$beh0", "TSubclassOf")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            if ptr != 10 || fault != 3 { r.func_is_method.insert(ptr); }
            r.func_ret.insert(ptr, if ptr == 10 && subclass {
                DataType { token: 5, type_info: result, is_reference: true, ..Default::default() }
            } else { DataType { token: 0x52, ..Default::default() } });
        }
        r.func_params.insert(9, vec![DataType { token: 0x3b, is_reference: true, ..Default::default() }]);
        r.func_params.insert(10, vec![DataType { token: 5, type_info: if fault == 2 { 4 } else { result },
            is_reference: true, is_object_const: true, is_read_only: fault != 4, ..Default::default() }]);
        r.func_params.insert(11, vec![]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_early_member_receiver(method: bool, wrong_type: bool, wrong_owner: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("UState", "Component"),
            ("UComponent", ""), ("FPayload", "")]);
        for (ptr, name) in [(1, "UState"), (2, "UComponent")] {
            r.type_identity_by_ptr.insert(ptr, TypeIdentity { module: String::new(),
                namespace: String::new(), name: name.into() });
        }
        r.prop_type_id.insert(3, if wrong_owner { 2 } else { 1 });
        r.set_class_fields(HashMap::from([("UState".into(), HashMap::from([
            ("Component".into(), if wrong_type { "UOther" } else { "UComponent" }.into())]))]));
        for (ptr, name, owner) in [(1, "$beh0", "FPayload"), (2, "Collect", "UComponent"),
            (3, "Remove", "UComponent"), (4, "Unrelated", "FPayload")] {
            r.func_by_ptr.insert(ptr, name.into()); r.func_owner.insert(ptr, owner.into());
            if ptr != 2 || method { r.func_is_method.insert(ptr); }
            r.func_ret.insert(ptr, DataType { token: 0x52, ..Default::default() });
            r.func_params.insert(ptr, if ptr == 2 {
                vec![DataType { token: 5, type_info: 3, is_reference: true, ..Default::default() }]
            } else if ptr == 3 { vec![DataType { token: 0x44, ..Default::default() }] } else { vec![] });
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_inlined_cast_cleanup(boolean: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("UNode", ""), ("UOther", "")]);
        for (id, name) in [(1, "UNode"), (2, "UOther")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(),
                module: "Fixture".into(), namespace: String::new() });
        }
        r.func_by_ptr.insert(1, "opCast".into());
        r.funcid_to_ptr.insert(2, 2); r.func_by_ptr.insert(2, "Check".into());
        r.func_ret.insert(2, DataType { token: if boolean { 0x41 } else { 0x44 }, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_delegate_int32_push(fault: u8) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(1, if fault == 1 { "__Evt_PushArgument__int" } else { "__Evt_PushArgument__int32" }.into());
        r.func_ret.insert(1, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(1, vec![DataType { token: if fault == 2 { 0x41 } else { 0x44 },
            is_reference: true, is_object_const: true, is_read_only: fault != 3, ..Default::default() }]);
        if fault == 4 { r.func_is_method.insert(1); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_parameter_field_comparison(ret: DataType, wrong_owner: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("UHolder", "Target"), ("UNode", "")]);
        for (id, name) in [(1, "UHolder"), (2, "UNode")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { module: "Fixture".into(),
                namespace: String::new(), name: name.into() });
        }
        r.prop_type_id.insert((1 << 1) | 1, if wrong_owner { 2 } else { 1 });
        r.set_class_fields(HashMap::from([("UHolder".into(),
            HashMap::from([("Target".into(), "UNode".into())]))]));
        r.func_by_ptr.insert(3, "GetNode".into());
        r.func_ret.insert(3, ret);
        r.func_params.insert(3, Vec::new());
        r.func_is_method.insert(3);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_parameter_comparison_upcast(fault: u8) -> Self {
        let mut r = Self::from_test_parameter_field_comparison(DataType::default(), false);
        r.type_by_ptr.insert(2, "AActor".into());
        r.type_identity_by_ptr.get_mut(&2).unwrap().name = "AActor".into();
        let field = match fault { 1 => "UNode", 2 => "const AGothicCharacter", _ => "AGothicCharacter" };
        r.set_class_fields(HashMap::from([("UHolder".into(),
            HashMap::from([("Target".into(), field.into())]))]));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reused_parameter_comparison(fault: u8) -> Self {
        let mut r = Self::from_test_parameter_comparison_upcast(0);
        r.type_identity_by_ptr.get_mut(&2).unwrap().module.clear();
        r.type_by_ptr.insert(1, "ADemonAreaVisual".into());
        r.type_identity_by_ptr.get_mut(&1).unwrap().name = "ADemonAreaVisual".into();
        r.type_by_ptr.insert(4, "ARing".into()); r.typeid_to_ptr.insert(4, 4);
        r.type_identity_by_ptr.insert(4, TypeIdentity { name: "ARing".into(), module: "Ring".into(), namespace: String::new() });
        r.class_super.insert("ARing".into(), "ADemonAreaVisual".into());
        r.class_super.insert("ADemonAreaVisual".into(), "AActor".into());
        r.set_class_fields(HashMap::from([("ADemonAreaVisual".into(), HashMap::from([("Target".into(), "AActor".into())]))]));
        r.func_by_ptr.insert(3, "GetOwner".into()); r.func_owner.insert(3, "AActor".into());
        r.const_method_ptrs.insert(3);
        r.func_ret.insert(3, DataType { token: 5, type_info: 2, is_object_handle: true, ..Default::default() });
        r.func_by_ptr.insert(5, "Work".into()); r.funcid_to_ptr.insert(5, 5);
        r.func_params.insert(5, Vec::new()); r.func_ret.insert(5, DataType { token: 0x52, ..Default::default() });
        match fault {
            1 => { r.func_by_ptr.insert(3, "OtherGetter".into()); }
            2 => { r.func_owner.insert(3, "UObject".into()); }
            3 => { r.const_method_ptrs.remove(&3); }
            4 => { r.func_params.insert(3, vec![DataType::default()]); }
            5 => r.func_ret.get_mut(&3).unwrap().is_reference = true,
            6 => r.func_ret.get_mut(&3).unwrap().is_object_const = true,
            7 => r.func_ret.get_mut(&3).unwrap().type_info = 4,
            8 => r.type_identity_by_ptr.get_mut(&2).unwrap().module = "Script".into(),
            9 => { r.prop_type_id.insert((1 << 1) | 1, 4); }
            _ => {}
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_cast_member_receiver(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UOwner", "Component"), ("UComponent", ""),
            ("UDerived", ""), ("FEvent", ""), ("ESpeed", "")]);
        for (id, name) in [(1, "UOwner"), (2, "UComponent"), (3, "UDerived"), (4, "FEvent"), (5, "ESpeed")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), namespace: String::new(),
                module: if id == 3 { "Script".into() } else { String::new() } });
        }
        r.prop_type_id.insert(3, if fault == 2 { 2 } else { 1 });
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("UOwner", "Component", if fault == 1 { "UOther" } else { "UComponent" })], &[], None));
        r.func_by_ptr.insert(10, "opCast".into()); r.func_is_method.insert(10);
        r.func_ret.insert(10, DataType { token: 0x52, ..Default::default() });
        r.funcid_to_ptr.insert(11, 11); r.func_by_ptr.insert(11, "Apply".into());
        r.func_owner.insert(11, if fault == 3 { "UOther" } else { "UDerived" }.into());
        if fault != 4 { r.func_is_method.insert(11); }
        r.func_ret.insert(11, DataType { token: 0x52, ..Default::default() });
        r.func_params.insert(11, vec![DataType { token: 5, type_info: 4, is_reference: true,
            is_object_const: fault == 5, ..Default::default() },
            DataType { token: 5, type_info: 5, is_object_const: true, ..Default::default() }]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_native_handle_read(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UHolder", "Target"), ("AActor", ""),
            ("UScriptHolder", "Avatar"), ("UOtherHolder", "Target")]);
        for (id, name) in [(1, "UHolder"), (2, "AActor"), (3, "UScriptHolder"), (4, "UOtherHolder")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity {
                module: if id == 3 || (fault == 2 && id == 1) { "Fixture".into() } else { String::new() },
                namespace: if fault == 4 && id == 2 { "Other".into() } else { String::new() },
                name: name.into() });
            r.prop_type_id.insert((id << 1) | 1, if fault == 3 && id == 1 { 4 } else { id as i32 });
        }
        let field_type = if fault == 1 { "UOther" } else { "AActor" };
        r.set_native_api(super::binds::NativeApi::from_test_field_types(
            &[("UHolder", "Target", field_type), ("UOtherHolder", "Target", "AActor")], &[], None));
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reused_cast_handle(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("AGoblin", ""), ("AOther", "")]);
        for (id, name) in [(1, "AGoblin"), (2, "AOther")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: "Creatures".into(), namespace: String::new() });
        }
        r.func_by_ptr.insert(2, "opCast".into());
        if fault != 1 { r.func_is_method.insert(2); }
        r.func_ret.insert(2, DataType { token: if fault == 2 { 0x41 } else { 0x52 }, is_reference: fault == 3, ..Default::default() });
        let argument = DataType { token: if fault == 4 { 0x44 } else { 0x3b }, is_reference: fault != 5,
            is_object_handle: fault == 6, ..Default::default() };
        r.func_params.insert(2, match fault { 7 => vec![], 8 => vec![argument.clone(), argument], _ => vec![argument] });
        r
    }
    #[cfg(test)]
    pub(crate) fn from_test_returned_loop_handle(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("AGothicCharacter", ""), ("AActor", ""),
            ("TArray", ""), ("TArrayIterator", "CanProceed")]);
        let handle = DataType { token: 5, type_info: 1, is_object_handle: true, ..Default::default() };
        let array = DataType { token: 5, type_info: 3, ..Default::default() };
        let iterator = DataType { token: 5, type_info: 4, ..Default::default() };
        let void = DataType { token: 0x52, ..Default::default() };
        r.type_subtypes.insert(3, vec![handle.clone()]);
        for (p, name, owner, ret, args) in [
            (1, "GetChildren", "", array, vec![]),
            (2, "Iterator", "TArray", iterator, vec![]),
            (3, "Proceed", "TArrayIterator", DataType { is_reference: true, ..handle }, vec![]),
            (4, "$beh2", "TArray", void, vec![]),
            (5, "GetDistanceTo", "AActor", DataType { token: 0x50, ..Default::default() },
                vec![DataType { token: 5, type_info: 2, is_object_handle: true, is_object_const: true, ..Default::default() }]),
        ] {
            r.func_by_ptr.insert(p, name.into()); r.funcid_to_ptr.insert(p as i32, p);
            r.func_ret.insert(p, ret); r.func_params.insert(p, args);
            if !owner.is_empty() { r.func_owner.insert(p, owner.into()); r.func_is_method.insert(p); }
        }
        r.const_method_ptrs.insert(5);
        if fault == 1 { r.func_ret.get_mut(&3).unwrap().type_info = 2; }
        if fault == 2 { r.func_ret.get_mut(&3).unwrap().is_reference = false; }
        if fault == 3 { r.func_ret.get_mut(&3).unwrap().is_object_const = true; }
        if fault == 4 { r.func_owner.insert(2, "TSet".into()); }
        if fault == 5 { r.func_ret.get_mut(&4).unwrap().token = 0x41; }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_loop_result_handle_aliases(fault: u8) -> Self {
        let mut r = Self::from_test_member_chain(&[("UCharacter", ""), ("TArrayIterator", "CanProceed"), ("UOther", "")]);
        for (id, name) in [(1, "UCharacter"), (2, "TArrayIterator"), (3, "UOther")] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: name.into(), module: String::new(), namespace: String::new() });
        }
        for (id, name) in [(1, "Iterator"), (2, "Proceed"), (3, "Select")] {
            r.func_by_ptr.insert(id, name.into()); r.funcid_to_ptr.insert(id as i32, id); r.func_is_method.insert(id);
        }
        r.func_params.insert(3, vec![DataType { token: 5, type_info: if fault == 2 { 3 } else { 1 },
            is_object_handle: true, is_reference: fault == 1, ..Default::default() }]);
        r.func_ret.insert(3, DataType { token: 0x52, ..Default::default() });
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reloaded_field_address_read(fault: u8) -> Self {
        let mut r = Self::from_test_reloaded_field_sum(false);
        let scalar = DataType { token: 0x51, ..Default::default() };
        let input = DataType { is_reference: true, is_object_const: true, is_read_only: true,
            ..scalar.clone() };
        r.func_by_ptr.insert(10, "Blend".into());
        r.func_ret.insert(10, scalar);
        r.func_params.insert(10, vec![input; 3]);
        if fault == 1 { r.func_params.get_mut(&10).unwrap()[2].is_read_only = false; }
        if fault == 2 { r.func_params.get_mut(&10).unwrap()[2].token = 0x50; }
        if fault == 3 { r.func_ret.get_mut(&10).unwrap().is_reference = true; }
        if fault == 4 { r.func_is_method.insert(10); }
        if fault == 5 { r.func_params.get_mut(&10).unwrap().pop(); }
        if fault == 6 { r.func_params.get_mut(&10).unwrap()[2].is_object_const = false; }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_reloaded_field_sum(wrong_owner: bool) -> Self {
        let mut r = Self::from_test_member_chain(&[("FHost", "Radius"), ("FHost", "Speed")]);
        for id in [1, 2] {
            r.type_identity_by_ptr.insert(id, TypeIdentity { name: "FHost".into(), module: "Fixture".into(),
                namespace: if wrong_owner && id == 2 { "Other" } else { "" }.into() });
            r.prop_type_id.insert((id << 1) | 1, id as i32);
        }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_handle_iterator(by_ref: bool, handle: bool, method: bool) -> Self {
        let mut r = Self::default();
        r.func_by_ptr.insert(1, "Proceed".into());
        r.func_owner.insert(1, "TArrayIterator<AActor>".into());
        r.func_params.insert(1, Vec::new());
        r.func_ret.insert(1, DataType { token: 5, type_info: 101,
            is_reference: by_ref, is_object_handle: handle, ..Default::default() });
        if method { r.func_is_method.insert(1); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_operator_rvo_argument(fault: u8) -> Self {
        let mut r = Self::default();
        r.type_by_ptr.insert(101, "FTime".into());
        let value = DataType { token: 5, type_info: 101, ..Default::default() };
        let input = DataType { is_reference: true, is_read_only: true, ..value.clone() };
        for (id, name) in [(1, "Now"), (2, "opSub"), (3, "$beh2"), (4, "Consume"), (5, "FromHours")] {
            r.func_by_ptr.insert(id, name.into()); r.funcid_to_ptr.insert(id as i32, id);
            r.func_ret.insert(id, value.clone()); r.func_params.insert(id, Vec::new());
        }
        r.func_owner.insert(2, "FTime".into()); r.func_owner.insert(3, "FTime".into());
        r.func_is_method.insert(2); if fault != 3 { r.const_method_ptrs.insert(2); }
        r.func_params.insert(2, vec![input.clone()]);
        if fault == 4 { r.func_ret.get_mut(&2).unwrap().is_reference = true; }
        r.func_ret.insert(4, DataType { token: 0x41, ..Default::default() });
        let handle = DataType { token: 5, type_info: 103, is_object_handle: true, ..Default::default() };
        r.func_params.insert(4, vec![handle.clone(), DataType { type_info: 102, ..input.clone() },
            DataType { type_info: if fault == 1 { 102 } else { 101 }, is_read_only: fault != 2, ..input }, handle]);
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_short_value_lifetimes(const_argument: bool, enum_reference: bool, const_discard: bool) -> Self {
        let mut r = Self::default();
        for (ptr, name) in [(101, "FString"), (102, "FSettings"), (103, "EOutcome")] {
            r.type_by_ptr.insert(ptr, name.into());
        }
        // Native enum metadata has a type name, but no script enumerator table.
        for (ptr, name, owner, token, type_info) in [
            (1, "MakeValue", "", 5, 101), (2, "Consume", "UBase", 5, 102),
            (3, "$beh2", "FString", 0x52, 0), (4, "$beh2", "FSettings", 0x52, 0),
            (5, "GetResult", "FString", 5, 103), (6, "PlayEffect", "UFX", 5, 101),
        ] {
            r.func_by_ptr.insert(ptr, name.into());
            r.funcid_to_ptr.insert(ptr as i32, ptr);
            r.func_ret.insert(ptr, DataType { token, type_info, ..Default::default() });
            r.func_params.insert(ptr, Vec::new());
            if !owner.is_empty() { r.func_owner.insert(ptr, owner.into()); r.func_is_method.insert(ptr); }
        }
        r.func_params.insert(2, vec![DataType { token: 5, type_info: 101,
            is_reference: true, is_object_const: const_argument, ..Default::default() }]);
        r.func_ret.insert(5, DataType { token: 5, type_info: 103,
            is_reference: enum_reference, is_object_const: true, ..Default::default() });
        r.const_method_names.insert("PlayEffect".into()); // Another overload is const.
        if const_discard { r.const_method_ptrs.insert(6); }
        r
    }

    #[cfg(test)]
    pub(crate) fn from_test_collision_names(names: &[&str]) -> Self {
        let mut resolver = Self::default();
        for (index, name) in names.iter().enumerate() {
            resolver.type_names.insert((*name).to_owned());
            resolver
                .type_by_ptr
                .insert(index as i64 + 1, (*name).to_owned());
        }
        resolver
    }
    /// Composed CONTAINER type of a NATIVE class's field (batch-25e,
    /// specs/batch23-nomatch.md E; precedent: KNOWN_NATIVE_ARITY). The script cache stores no
    /// value types for native-class fields, so `cast_container_args` could never derive the
    /// key/value enums for e.g. `this.m_CollisionComp.m_CustomCollisionResponse.Add(1, 1)`
    /// (25 in-game errors: TMap::Add/FindOrAdd/Find with bare int keys). Every entry's
    /// subtypes are taken VERBATIM from the live compiler's `Candidates are:` lines in
    /// capture.batch24-0705 (authoritative), keyed by the exact ADDSi-tid owners probed at
    /// the failing sites (all three UHit*CollisionComponent variants carry their own
    /// property-reference key). FWeatherSaveGame.DailyWeathers joined in batch-31c (N3
    /// Fix 3): capture.batch30-0705 OnDayElapsed(413:48) provides the candidate line
    /// `bool TArray::Contains(const EWeather&in Value) const` — same never-guess rule.
    pub fn known_native_field_subtype(&self, class: &str, field: &str) -> Option<&'static str> {
        const KNOWN_NATIVE_FIELD_SUBTYPES: &[(&str, &str, &str)] = &[
            (
                "UHitBoxCollisionComponent",
                "m_CustomCollisionResponse",
                "TMap<ECollisionChannel, ECollisionResponse>",
            ),
            (
                "UHitCapsuleCollisionComponent",
                "m_CustomCollisionResponse",
                "TMap<ECollisionChannel, ECollisionResponse>",
            ),
            (
                "UHitConeCollisionComponent",
                "m_CustomCollisionResponse",
                "TMap<ECollisionChannel, ECollisionResponse>",
            ),
            (
                "FFXParticleSystem",
                "NiagaraSystemPathBySurfaceType",
                "TMap<EPhysicalSurface, TSoftObjectPtr<UNiagaraSystem>>",
            ),
            (
                "FWeatherSaveGame",
                "WeatherModifiers",
                "TMap<EWeather, float32>",
            ),
            ("FWeatherSaveGame", "DailyWeathers", "TArray<EWeather>"),
        ];
        KNOWN_NATIVE_FIELD_SUBTYPES
            .iter()
            .find(|(c, f, _)| *c == class && *f == field)
            .map(|(_, _, t)| *t)
    }

    /// Member name from a containing type-id + byte offset.
    pub fn member(&self, type_id: i32, offset: i32) -> Option<&str> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_by_key.get(&key).map(|s| s.as_str())
    }
    /// Atomically resolve one unambiguous T7 row as `(Name, OldTypeId)`.
    ///
    /// This is the semantic-oracle accessor: it refuses every duplicate key (including identical
    /// duplicate rows) and any internally incomplete lookup. Callers can then resolve the retained
    /// serialized `OldTypeId` through this cache's own T2 -> T1 chain. The older name-only helpers
    /// remain available to decompilation paths whose compatibility behavior predates this gate.
    pub(crate) fn member_identity(&self, type_id: i32, offset: i32) -> Option<(&str, i32)> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        if self.duplicate_prop_keys.contains(&key) {
            return None;
        }
        self.prop_by_key
            .get(&key)
            .map(String::as_str)
            .zip(self.prop_type_id.get(&key).copied())
    }
    /// Member's type NAME (e.g. `bool`, `ECrimeDurationType`) from type-id + byte offset,
    /// resolved via its PropertyReferences OldTypeId. Used to cast field-assignment RHS.
    pub fn member_type(&self, type_id: i32, offset: i32) -> Option<&str> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_type_id
            .get(&key)
            .and_then(|id| self.type_by_id(*id))
    }
    /// [`Self::member_type`], composed variant (declaring class INCLUDING template subtypes,
    /// e.g. `TArrayConstIterator<AGothicCharacter>` instead of the bare head). Used where the
    /// name becomes a slot DECLARATION (is-not-a-member.md §2.1/§2.2); the bare variant stays
    /// for the head-comparing cast paths in structure.rs.
    pub fn member_type_composed(&self, type_id: i32, offset: i32) -> Option<String> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_type_id
            .get(&key)
            .and_then(|id| self.type_by_id_composed(*id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_type_key_ignores_namespaces_on_both_sides() {
        assert_eq!(
            strip_namespaces("TSubclassOf<G1R::AIGroup::UAIGroup_StateEvent>"),
            "TSubclassOf<UAIGroup_StateEvent>"
        );
        assert_eq!(strip_namespaces("TMap<A, G1R::B>"), "TMap<A, B>");
        assert_eq!(strip_namespaces("FVector"), "FVector");
    }

    #[test]
    fn a_types_recorded_methods_are_found_through_its_namespaced_spelling() {
        let mut refs = RefResolver::default();
        refs.type_methods
            .insert("TSubclassOf<UAIGroup_StateEvent>::$beh0/0".to_owned());
        assert!(refs.type_has_method("TSubclassOf<G1R::AIGroup::UAIGroup_StateEvent>", "$beh0", 0));
        assert!(!refs.type_has_method("TSubclassOf<UAIGroup_StateEvent>", "$beh0", 1));
    }

    #[test]
    fn a_const_return_is_kept_unless_the_rows_disagree_or_a_caller_cannot_hold_it() {
        let mut refs = RefResolver::default();
        refs.inconsistent_const_return_names
            .insert(const_return_key("GetRootNode", false));
        refs.set_unusable_const_returns(HashSet::from([const_return_key("GetSpawnedActor", true)]));
        assert!(refs.const_return_is_inconsistent("GetRootNode", false));
        assert!(refs.const_return_is_inconsistent("GetSpawnedActor", true));
        // The same names under the OTHER qualifier are separate rows: `T f()` next to
        // `const T f() const` is an accessor pair, not a disagreement.
        assert!(!refs.const_return_is_inconsistent("GetRootNode", true));
        assert!(!refs.const_return_is_inconsistent("GetSpawnedActor", false));
        assert!(!refs.const_return_is_inconsistent("GetSelectedItem", false));
    }

    #[test]
    fn a_one_argument_call_accepts_a_temporary_only_when_every_overload_does() {
        let mut refs = RefResolver::default();
        refs.temporary_arg_methods
            .insert("Add/FCrimeSetup".to_owned());
        assert!(refs.one_arg_call_accepts_temporary("Add", "FCrimeSetup"));
        assert!(refs.one_arg_call_accepts_temporary("Add", "G1R::Crime::FCrimeSetup"));
        assert!(!refs.one_arg_call_accepts_temporary("Add", "FOtherSetup"));
        assert!(!refs.one_arg_call_accepts_temporary("Consume", "FCrimeSetup"));
    }

    #[test]
    fn an_unknown_name_may_be_called_with_no_arguments_but_a_known_one_may_not() {
        let mut refs = RefResolver::default();
        refs.known_func_names.insert("RequireFalse".to_owned());
        refs.known_func_names.insert("Num".to_owned());
        refs.zero_arg_names.insert("Num".to_owned());
        assert!(!refs.zero_arg_call_is_plausible("RequireFalse"));
        assert!(refs.zero_arg_call_is_plausible("Num"));
        assert!(refs.zero_arg_call_is_plausible("SomethingTheCacheNeverRecorded"));
    }

    #[test]
    fn truncated_huge_tail_count_fails_before_resolver_allocation() {
        let mut bytes = vec![0u8; 16];
        bytes.extend_from_slice(&super::super::header::CACHE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&50_000_000i32.to_le_bytes());
        let error = RefResolver::build(&bytes).unwrap_err();
        assert!(
            matches!(
                error,
                WireError::BadLen {
                    field: "tail keyed rows",
                    ..
                }
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn native_arity_never_borrows_a_name_match_from_an_unrelated_owner() {
        let mut refs = RefResolver::default();
        refs.func_owner.insert(10, "FPerceptionHandler".to_string());
        refs.func_owner.insert(11, "FExactOwner".to_string());
        refs.func_owner.insert(13, "TArray".to_string());
        refs.func_owner.insert(14, "AActor".to_string());
        refs.func_owner.insert(15, "AExactActor".to_string());
        refs.func_params.insert(10, vec![DataType::default()]);
        refs.func_params.insert(13, vec![DataType::default()]);
        refs.func_params
            .insert(14, vec![DataType::default(), DataType::default()]);
        refs.func_params
            .insert(15, vec![DataType::default(), DataType::default()]);
        refs.native = Some(super::super::binds::NativeApi::from_test_arities(
            &[
                ("FExactOwner", "Exact", 1),
                ("AExactActor", "ExactObject", 1),
            ],
            &[
                ("AddEvent", Some(2)),
                ("Exact", Some(3)),
                ("Last", Some(0)),
                ("GetComponent", Some(0)),
                ("ExactObject", Some(0)),
            ],
        ));

        // The only Binds AddEvent is an unrelated two-arg method. The cache declaration for
        // the owner-known FPerceptionHandler method must remain authoritative.
        assert_eq!(refs.native_arity_by_ptr(10, "AddEvent"), None);
        // Exact owner/name evidence still overrides the cache declaration.
        assert_eq!(refs.native_arity_by_ptr(11, "Exact"), Some(1));
        // Ownerless free/static calls retain the safe globally-unambiguous fallback.
        assert_eq!(refs.native_arity_by_ptr(12, "AddEvent"), Some(2));
        // A by-name arity that only suppresses source-default args remains safe.
        assert_eq!(refs.native_arity_by_ptr(13, "Last"), Some(0));
        // A class-agnostic Binds hit for an object method is not owner evidence. Keep the
        // two-parameter AActor cache declaration instead of borrowing FHitResult's zero args.
        assert_eq!(refs.native_arity_by_ptr(14, "GetComponent"), None);
        // An exact object-owner entry remains authoritative; only the name-only fallback is barred.
        assert_eq!(refs.native_arity_by_ptr(15, "ExactObject"), Some(1));
    }

    #[test]
    fn a_native_value_comparison_keeps_its_tolerance_despite_an_unrelated_name_match() {
        let mut refs = RefResolver::default();
        refs.func_owner.insert(10, "FVector".into());
        refs.func_is_method.insert(10); refs.const_method_ptrs.insert(10);
        refs.func_ret.insert(10, DataType { token: 0x41, ..Default::default() });
        refs.type_identity_by_ptr.insert(100, TypeIdentity { name: "FVector".into(), module: String::new(), namespace: String::new() });
        refs.func_params.insert(10, vec![
            DataType { token: 5, type_info: 100, is_reference: true, is_object_const: true, is_read_only: true, ..Default::default() },
            DataType { token: 0x51, ..Default::default() },
        ]);
        refs.native = Some(super::super::binds::NativeApi::from_test_arities(&[], &[("Compare", Some(1))]));
        for token in [0x50, 0x51, 0x5e] {
            refs.func_params.get_mut(&10).unwrap()[1].token = token;
            assert_eq!(refs.native_arity_by_ptr(10, "Compare"), None);
            assert_eq!(refs.native_arity_by_ptr(10, "Compare").or_else(|| refs.func_params_by_ptr(10).map(|p| p.len())), Some(2));
        }
        // Exact owner evidence and unrelated default-argument signatures retain their rules.
        refs.native = Some(super::super::binds::NativeApi::from_test_arities(&[("FVector", "Compare", 1)], &[("Compare", Some(1))]));
        assert_eq!(refs.native_arity_by_ptr(10, "Compare"), Some(1));
        refs.native = Some(super::super::binds::NativeApi::from_test_arities(&[], &[("Compare", Some(1))]));
        refs.func_params.get_mut(&10).unwrap()[1].is_reference = true;
        assert_eq!(refs.native_arity_by_ptr(10, "Compare"), Some(1));
        refs.func_params.get_mut(&10).unwrap()[1].is_reference = false;
        refs.type_identity_by_ptr.get_mut(&100).unwrap().namespace = "Other".into();
        assert_eq!(refs.native_arity_by_ptr(10, "Compare"), Some(1));
    }

    #[test]
    fn namespaced_native_arity_rejects_a_conflicting_bare_name() {
        let mut refs = RefResolver::default();
        refs.func_ns.insert(10, "VLog".into());
        refs.func_params.insert(10, vec![DataType::default(); 3]);
        refs.native = Some(super::super::binds::NativeApi::from_test_arities(
            &[], &[("LogInfo", Some(2))],
        ));
        assert_eq!(refs.native_arity_by_ptr(10, "LogInfo"), None);
        // CALLSYS's existing fallback keeps the exact three-argument physical frame.
        assert_eq!(refs.native_arity_by_ptr(10, "LogInfo")
            .or_else(|| refs.func_params_by_ptr(10).map(|p| p.len())), Some(3));
        // An unrelated longer signature must not consume an enclosing operand either.
        refs.func_params.insert(10, vec![DataType::default()]);
        assert_eq!(refs.native_arity_by_ptr(10, "LogInfo"), None);
    }

    #[test]
    fn namespaced_native_arity_preserves_matching_unknown_and_owned_cases() {
        let mut refs = RefResolver::default();
        refs.func_ns.insert(10, "VLog".into());
        refs.func_params.insert(10, vec![DataType::default(); 2]);
        refs.func_params.insert(11, vec![DataType::default(); 3]);
        refs.func_ns.insert(12, "VLog".into());
        refs.func_owner.insert(12, "FExactOwner".into());
        refs.func_params.insert(12, vec![DataType::default(); 3]);
        refs.func_ns.insert(13, "VLog".into());
        refs.native = Some(super::super::binds::NativeApi::from_test_arities(
            &[("FExactOwner", "LogInfo", 1)], &[("LogInfo", Some(2))],
        ));
        assert_eq!(refs.native_arity_by_ptr(10, "LogInfo"), Some(2));
        assert_eq!(refs.native_arity_by_ptr(11, "LogInfo"), Some(2));
        assert_eq!(refs.native_arity_by_ptr(12, "LogInfo"), Some(1));
        // Missing cache params provide no contradictory count; don't alter that fallback.
        assert_eq!(refs.native_arity_by_ptr(13, "LogInfo"), Some(2));
    }

    #[test]
    fn source_cache_guid_gates_mutation_types_but_not_read_only_evidence() {
        fn empty_cache(guid: [u8; 16], magic: u32) -> Vec<u8> {
            let mut bytes = guid.to_vec();
            bytes.extend_from_slice(&magic.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            for _ in 0..7 {
                bytes.extend_from_slice(&0u32.to_le_bytes());
            }
            bytes
        }

        fn attach_test_binds(refs: &mut RefResolver) {
            refs.set_native_api(super::super::binds::NativeApi::from_test_field_types(
                &[("UItemDefinition", "m_Value", "bool")],
                &[("UItemDefinition", "m_Value", "int")],
                Some(gore_generation::GENERATION_ROWS[0].binds_cache.sha256),
            ));
        }

        let row = &gore_generation::GENERATION_ROWS[0];
        let mut refs = RefResolver::build(&empty_cache(
            row.script_cache_guid,
            super::super::header::CACHE_MAGIC,
        ))
        .expect("build exact-pair resolver");
        attach_test_binds(&mut refs);

        assert_eq!(
            refs.verified_native_default_field_type(&[0; 16], "UItemDefinition", "m_Value",),
            None
        );
        assert_eq!(
            refs.verified_source_cache_native_default_field_type(
                "UItemDefinition",
                "m_Value"
            ),
            Some("int"),
            "the source-cache channel accepts the exactly paired generation"
        );
        assert_eq!(
            refs.native_field_type("UItemDefinition", "m_Value"),
            Some("int"),
            "rendering must prefer the sealed type that admitted authored defaults"
        );
        assert_eq!(
            refs.native_field_value_type("UItemDefinition", "m_Value"),
            Some("int")
        );

        let mut foreign_guid = row.script_cache_guid;
        foreign_guid[0] ^= 1;
        let mut foreign = RefResolver::build(&empty_cache(
            foreign_guid,
            super::super::header::CACHE_MAGIC,
        ))
        .expect("build foreign-generation resolver");
        attach_test_binds(&mut foreign);
        assert_eq!(
            foreign.verified_source_cache_native_default_field_type(
                "UItemDefinition",
                "m_Value"
            ),
            None,
            "a foreign script-cache GUID must not authorize default authoring"
        );
        assert_eq!(
            foreign.native_field_value_type("UItemDefinition", "m_Value"),
            Some("bool"),
            "read-only decompilation keeps its best-effort Binds evidence"
        );
        assert_eq!(
            foreign.native_field_type("UItemDefinition", "m_Value"),
            Some("bool")
        );

    }

    #[test]
    fn exact_field_lookup_does_not_borrow_an_inherited_declaration() {
        let mut refs = RefResolver::default();
        refs.class_fields.insert(
            "Base".into(),
            [("Value".into(), "int".into())].into_iter().collect(),
        );
        refs.class_fields.insert("Mid".into(), HashMap::new());
        refs.class_super.insert("Mid".into(), "Base".into());

        assert_eq!(refs.field_type_by_class("Mid", "Value"), Some("int"));
        assert_eq!(refs.own_field_type_by_class("Mid", "Value"), None);
        assert_eq!(refs.own_field_type_by_class("Base", "Value"), Some("int"));
    }
}

#[cfg(test)]
mod const_object_field_tests {
    use super::*;

    #[test]
    fn integer_field_evidence_stays_with_its_qualified_owner() {
        let mut refs = RefResolver::from_test_copied_int_field_read("int", false);
        let owner = refs.type_identity_by_ptr[&1].clone();
        let ty = DataType { token: 0x44, ..Default::default() };
        assert!(refs.is_script_int_field(1, 0));
        assert!(!refs.is_script_int_field(1, 4));
        let mut foreign = owner.clone(); foreign.namespace = "Other".into();
        refs.type_identity_by_ptr.insert(2, foreign.clone());
        // A same-named class in another namespace cannot donate the field type.
        refs.set_qualified_fields([(foreign.clone(), "Limit".into(), ty.clone())]);
        assert!(!refs.is_script_int_field(1, 0));
        refs.set_qualified_fields([(owner.clone(), "Limit".into(), ty.clone()),
            (foreign, "Limit".into(), DataType { token: 0x50, ..Default::default() })]);
        assert!(refs.is_script_int_field(1, 0));
        let other = DataType { token: 0x50, ..Default::default() };
        for declarations in [[ty.clone(), other.clone()], [other, ty.clone()]] {
            refs.set_qualified_fields(declarations.into_iter().map(|t| (owner.clone(), "Limit".into(), t)));
            assert!(!refs.is_script_int_field(1, 0));
        }
        refs.type_identity_by_ptr.insert(3, owner.clone());
        refs.set_qualified_fields([(owner, "Limit".into(), ty)]);
        assert!(!refs.is_script_int_field(1, 0));
    }

    #[test]
    fn qualified_field_evidence_rejects_owner_and_value_type_collisions() {
        let mut refs = RefResolver::from_test_const_object_fields();
        let ty = DataType { token: 5, type_info: 4,
            is_object_const: true, is_object_handle: true, ..Default::default() };
        assert!(refs.const_object_field_accepts(1, 0, &ty));
        for owner in [2, 3, 99] { assert!(!refs.const_object_field_accepts(owner, 0, &ty)); }
        assert!(!refs.const_object_field_accepts(1, 1, &ty));
        let mut other = ty.clone(); other.type_info = 5;
        assert!(!refs.const_object_field_accepts(1, 0, &other));
        let owner = TypeIdentity { module: "Fixture".into(), namespace: "One".into(), name: "FHolder".into() };
        let mut mutable = ty.clone(); mutable.is_object_const = false;
        // Both declaration orders must reject a mutable/const duplicate.
        for declarations in [[ty.clone(), mutable.clone()], [mutable, ty.clone()]] {
            refs.set_qualified_fields(declarations.into_iter()
                .map(|field| (owner.clone(), "Value".into(), field)));
            assert!(!refs.const_object_field_accepts(1, 0, &ty));
        }
        refs.type_identity_by_ptr.insert(6, owner.clone());
        refs.set_qualified_fields([(owner, "Value".into(), ty.clone())]);
        assert!(!refs.const_object_field_accepts(1, 0, &ty));
    }
}
