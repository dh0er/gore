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

use std::collections::HashMap;

use super::header::CacheHeader;
use super::types::DataType;
use super::walk_modules::module_region_end;
use super::wire::{Cursor, WireError};

/// Resolved-name lookup built from a cache's tail tables.
#[derive(Debug, Default)]
pub struct RefResolver {
    type_by_ptr: HashMap<i64, String>,
    func_by_ptr: HashMap<i64, String>,
    global_by_ptr: HashMap<i64, String>,
    prop_by_key: HashMap<i64, String>,
    /// PropertyReferences OldTypeId per member key (for field-assignment casts).
    prop_type_id: HashMap<i64, i32>,
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
    /// FunctionReferences parameter DataTypes (for arg-type-driven casts at call sites).
    func_params: HashMap<i64, Vec<DataType>>,
    /// FunctionReferences return DataType.
    func_ret: HashMap<i64, DataType>,
    /// FunctionReferences owning class name (from the ObjectType ptr) — disambiguates native
    /// method overloads when looking up arity in the Binds.Cache native API.
    func_owner: HashMap<i64, String>,
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
        let tail = module_region_end(bytes)?;
        let mut c = Cursor::at(bytes, tail);
        let mut r = RefResolver::default();

        // T1 TypeReferences: int64 key + (Name, Module, Namespace, TArray<DataType>)
        let type_reference_count = c.read_count("TypeReferences")?;
        c.ensure_minimum_remaining(type_reference_count, 24, "TypeReferences")?;
        for _ in 0..type_reference_count {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            c.read_sia()?; // Module
            c.read_sia()?; // Namespace
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
            r.type_by_ptr.insert(key, name);
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
        let function_reference_count = c.read_count("FunctionReferences")?;
        c.ensure_minimum_remaining(function_reference_count, 80, "FunctionReferences")?;
        for _ in 0..function_reference_count {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            let module = c.read_sia()?; // Module (declaring module name, batch-25f)
            let ns = c.read_sia()?; // Namespace
            c.skip(4)?; // bIsConst
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
            r.func_params.insert(key, params);
            r.func_ret.insert(key, ret);
            if let Some(cls) = r.type_by_ptr.get(&objtype) {
                r.func_owner.insert(key, cls.clone());
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
            let name = c.read_sia()?;
            c.read_sia()?; // Module
            let ns = c.read_sia()?; // Namespace
            let is_string = c.read_bool4()?;
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
            r.prop_by_key.insert(key, name);
            r.prop_type_id.insert(key, old_type_id);
        }
        let _ = CacheHeader::SIZE; // (header parsed elsewhere)
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
    pub fn type_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.type_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// True if `name` is a known type (so a call to it is a constructor, not a method).
    pub fn is_type_name(&self, name: &str) -> bool {
        self.type_names.contains(name)
    }

    /// Inject the script-class hierarchy (class name -> super name) from parsed modules.
    pub fn set_class_hierarchy(&mut self, supers: HashMap<String, String>) {
        self.class_super = supers;
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
    /// Free/static calls without an owner retain the unambiguous by-name fallback.
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
                let cache = self.func_params.get(&ptr)?.len();
                (by_name <= cache).then_some(by_name)
            }),
            None => n.arity_by_name(name),
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
    /// True if `name` exists as a MEMBER anywhere: T3 method names (native or script, referenced
    /// by bytecode), injected script-class method declarations, or any Binds native signature
    /// name. The production emit runs WITHOUT Binds (JOURNAL: binds-arity emit regressed), so the
    /// cache-derived sets are the load-bearing sources; Binds adds coverage when loaded.
    /// batch-32b (N6, spec batch31-nomatch-illegalop §1.10): universal UObject members shadow a
    /// same-named free script global inside EVERY class body (all script classes derive UObject),
    /// but are invisible to the cache-derived sets unless some bytecode call references them as a
    /// T3 method — `GetName(EPerceptionCharacterType)` resolved against the inherited
    /// `FName UObject::GetName() const` instead of the free script fn (EventResponses ×2).
    /// Static list; `::`-over-qualification of a non-shadowed global stays harmless.
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
    /// Number of StaticNames entries (debug aid).
    pub fn static_name_count(&self) -> usize {
        self.static_names.len()
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
    fn truncated_huge_tail_count_fails_before_resolver_allocation() {
        let mut bytes = vec![0u8; 16];
        bytes.extend_from_slice(&super::super::header::CACHE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&50_000_000i32.to_le_bytes());
        let error = RefResolver::build(&bytes).unwrap_err();
        assert!(
            error.to_string().contains("unexpected end of data"),
            "{error}"
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
}
