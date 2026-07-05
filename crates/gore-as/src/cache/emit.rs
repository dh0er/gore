//! Recompilable `.as` emitter: a parsed [`model::Module`] -> AngelScript source the
//! GAME compiler accepts (per `work/reversing/gore-as/findings/recompile-*.md`).
//!
//! Rules: flat top-level file (module name is the file PATH, not a namespace, so no
//! wrapper); no `import` (automaticImports=1); `class X : Super`; UFUNCTION()/UPROPERTY()
//! only when the stored flag is set; skip generator-synthesized symbols (StaticClass,
//! the class-name ctor wrapper). Function bodies come from the structured decompiler with
//! hoisted local declarations; bodies the decompiler can't recover fall back to a
//! signature-matched STUB so the module still compiles.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::sync::OnceLock;

/// Optional list (one `Class::method` or free `function` per line) of functions to force
/// into the stub fallback — for the handful the decompiler can't recover correctly that the
/// in-game compile feedback flagged (engine-object arg mismatches, float-overload ambiguity).
/// Path comes from `GORE_AS_STUBLIST`; absent => empty (no forced stubs).
fn force_stub_set() -> &'static HashSet<String> {
    static L: OnceLock<HashSet<String>> = OnceLock::new();
    L.get_or_init(|| {
        std::env::var("GORE_AS_STUBLIST")
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
            .unwrap_or_default()
    })
}

use super::disasm::disassemble;
use super::model::{Class, Func, Module};
use super::refs::RefResolver;
use super::structure::{body_statements_ctor, RVODEF};
use super::types::token_keyword;
use super::walk_modules::FuncCode;

/// Emit a whole module as recompilable AngelScript.
pub fn emit_module(m: &Module, refs: &RefResolver) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "// gore-as decompiled module: {} ({})", m.name, m.file);
    let _ = writeln!(s, "// NOTE: local names + string literals are not stored in the cache.\n");

    for e in &m.enums {
        let _ = writeln!(s, "enum {}", e.name);
        let _ = writeln!(s, "{{");
        let mut expect = 0i32;
        for (name, val) in &e.entries {
            if *val == expect {
                let _ = writeln!(s, "    {name},");
            } else {
                let _ = writeln!(s, "    {name} = {val},");
            }
            expect = val + 1;
        }
        let _ = writeln!(s, "}}\n");
    }

    for g in &m.globals {
        if g.name.starts_with("__") {
            continue; // generator-synthesized (e.g. __StaticType_X)
        }
        let base = g.ty.base_name(refs);
        if !is_primitive(&base) && !is_enum(&base) {
            // AngelScript globals MUST be const, but an FName/F-struct can't take `= 0`.
            // FName constants are almost always named after their value -> `n"Name"`; other
            // structs get a default-constructed const (their real value isn't recoverable).
            if base == "FName" {
                let _ = writeln!(s, "const FName {0} = n\"{0}\";", g.name);
            } else {
                let _ = writeln!(s, "const {base} {} = {base}();", g.name);
            }
            continue;
        }
        // a primitive/enum const; None = runtime init we didn't recover, use a stub value.
        let inner = match g.value {
            Some(v) => render_const(&base, v),
            None => default_for(&base),
        };
        // AngelScript rejects implicit int->enum, so an enum const must be cast: `EType(1)`.
        if is_enum(&base) {
            let _ = writeln!(s, "const {base} {} = {base}({inner});", g.name);
        } else {
            let _ = writeln!(s, "const {base} {} = {inner};", g.name);
        }
    }
    if !m.globals.is_empty() {
        s.push('\n');
    }

    let class_names: HashSet<&str> = m.classes.iter().map(|c| c.name.as_str()).collect();
    // class name -> its member (method/ctor) names, so a module-level function is only treated
    // as an already-emitted class member when it actually names one — not merely because its
    // namespace happens to match a class.
    let class_members: HashMap<&str, HashSet<&str>> = m
        .classes
        .iter()
        .map(|c| {
            let members: HashSet<&str> = c
                .methods
                .iter()
                .chain(c.ctors.iter())
                .map(|f| f.name.as_str())
                .collect();
            (c.name.as_str(), members)
        })
        .collect();

    for c in &m.classes {
        emit_class(&mut s, c, refs);
    }

    // free functions = module.functions that aren't generator-synthesized accessors
    let mut seen_free: HashSet<String> = HashSet::new();
    for f in &m.functions {
        if is_generated(f, &class_names, &class_members) || is_generated_spawn(f, refs) {
            continue;
        }
        if !seen_free.insert(format!("{}({})", f.name, param_sig(f, refs))) {
            continue; // duplicate signature -> "function ... already exists"
        }
        emit_function(&mut s, f, refs, false, false, 0);
    }
    s
}

/// The AngelScript-UE binding auto-generates factory free functions for every actor/component
/// class. The cache also carries them as module functions, so emitting them duplicates the native
/// binding ("a function with the same name and parameters already exists" — un-stubbable, the
/// declaration itself collides). Skip the exact generated shapes:
///   - actor:     `<Actor> Spawn(const FVector&, const FRotator&, const FName&, bool, ULevel)`
///   - component: `<Comp> Get|GetOrCreate|Create(const AActor, const FName&)`
fn is_generated_spawn(f: &Func, refs: &RefResolver) -> bool {
    if !f.ret.is_object_handle {
        return false;
    }
    let p0 = f.params.first().map(|p| p.ty.base_name(refs));
    let p0 = p0.as_deref();
    if f.name == "Spawn" && f.params.len() == 5 && p0 == Some("FVector") {
        return true;
    }
    if matches!(f.name.as_str(), "Get" | "GetOrCreate" | "Create")
        && f.params.len() == 2
        && p0 == Some("AActor")
        && f.params.get(1).map(|p| p.ty.base_name(refs)).as_deref() == Some("FName")
    {
        return true;
    }
    // subsystem/singleton accessor: `<Subsystem> Get()` / `GetG1R()` (0 params, handle return).
    if matches!(f.name.as_str(), "Get" | "GetG1R") && f.params.is_empty() {
        return true;
    }
    false
}

fn emit_class(s: &mut String, c: &Class, refs: &RefResolver) {
    match &c.super_class {
        Some(sup) if !sup.is_empty() => {
            let _ = writeln!(s, "class {} : {}", c.name, sup);
        }
        _ => {
            let _ = writeln!(s, "class {}", c.name);
        }
    }
    let _ = writeln!(s, "{{");
    for f in &c.fields {
        // Drop a leading `const`: UE-AS UPROPERTY members aren't const-assignable, yet the
        // generated constructor assigns them — keeping `const` causes "Cannot assign" errors.
        let ty = f.ty.render(refs);
        let ty = ty.strip_prefix("const ").unwrap_or(&ty);
        if f.is_uproperty {
            let _ = writeln!(s, "    UPROPERTY()");
        }
        let _ = writeln!(s, "    {ty} {};", f.name);
    }
    if !c.fields.is_empty() {
        s.push('\n');
    }
    let super_name = c.super_class.as_deref().filter(|s| !s.is_empty());
    // field name -> base type name, so the decompiler can cast `this.field = <int>` assignments.
    let field_types: HashMap<String, String> =
        c.fields.iter().map(|f| (f.name.clone(), f.ty.base_name(refs))).collect();
    for ctor in &c.ctors {
        emit_function_ctor(s, ctor, refs, true, true, 1, super_name, Some(&field_types), Some(&c.name));
    }
    // Dedup methods by name+parameters: the cache can carry two entries that render to the same
    // signature (e.g. a const- and non-const-return overload that collapse once the meaningless
    // return `const` is stripped), which AngelScript rejects as "a function with the same name
    // and parameters already exists".
    let mut seen_sigs: HashSet<String> = HashSet::new();
    for m in &c.methods {
        // `__InitDefaults` (and other `__`-prefixed generator methods) set the CDO defaults
        // via raw `__StaticType_*` symbols and untyped literals we can't reconstruct offline;
        // they are auto-generated boilerplate, not hand-written script — skip them so the
        // class compiles. (Runtime UPROPERTY defaults are lost; real script logic is intact.)
        if m.name.starts_with("__") {
            continue;
        }
        if !seen_sigs.insert(format!("{}({})", m.name, param_sig(m, refs))) {
            continue; // duplicate signature
        }
        emit_function_ctor(s, m, refs, true, false, 1, None, Some(&field_types), Some(&c.name));
    }
    let _ = writeln!(s, "}}\n");
}

fn emit_function(s: &mut String, f: &Func, refs: &RefResolver, is_method: bool, is_ctor: bool, depth: usize) {
    emit_function_ctor(s, f, refs, is_method, is_ctor, depth, None, None, None);
}

#[allow(clippy::too_many_arguments)]
fn emit_function_ctor(s: &mut String, f: &Func, refs: &RefResolver, is_method: bool, is_ctor: bool, depth: usize, super_ctor: Option<&str>, fields: Option<&HashMap<String, String>>, class_name: Option<&str>) {
    let ind = "    ".repeat(depth);
    // Strip a leading `const` from the return type: a return-by-value `const` is meaningless in
    // AngelScript, and the cache sets the const flag inconsistently between a base method and its
    // override -> "must have the same return type as in the base class". Stripping makes them match.
    let ret = f.ret.render(refs).trim_start_matches("const ").to_string();
    let params = render_params(f, refs);
    // NOTE: the signature is written AFTER the body is computed (below) — the ref-return `&`
    // rendering must know whether the body falls back to a stub / RVODEF default return.

    let fc = FuncCode {
        func: f.name.clone(),
        is_method,
        param_names: f.params.iter().map(|p| p.name.clone()).collect(),
        param_types: f.params.iter().map(|p| p.ty.clone()).collect(),
        ret: f.ret.clone(),
        bytecode: f.bytecode.clone(),
    };
    let param_types: Vec<String> = f.params.iter().map(|p| p.ty.base_name(refs)).collect();
    // object-local slot -> type name, so the decompiler can insert downcasts on stores.
    let mut local_types: HashMap<i32, String> = f.obj_locals.iter().map(|(slot, tinfo)| {
        let ty = super::types::DataType { token: 5, type_info: *tinfo, is_object_handle: true, ..Default::default() }.base_name(refs);
        (*slot, ty)
    }).collect();
    // consumer-side override: never-written arg slots get the type their callee expects (fixes
    // mis-typed default/optional-arg slots — FName->UAIState_DailyRoutine, TSubclassOf<X>->X).
    // outref_overrides: PSF slots feeding float-family REFERENCE params (declaration-only, below).
    let (slot_overrides, outref_overrides) = infer_slot_types(f, refs);
    for (slot, ty) in &slot_overrides {
        local_types.insert(*slot, ty.clone());
    }
    // member-access-derived types: the field's declaring class is the strongest signal for a
    // slot used as a member-access base; apply AFTER (overriding) the call-arg guess.
    let member_overrides = infer_slot_types_from_members(f, refs, fields, class_name);
    for (slot, ty) in &member_overrides {
        local_types.insert(*slot, ty.clone());
    }
    // iterator-instance subtypes (illegal-op-round2.md A1): the T1 entry for a `T*Iterator`
    // template INSTANCE carries no SubTypes, so every slot typed from it declares as a bare
    // head (`TArrayConstIterator local_N;`) — a template-arity error that makes the local
    // `Unknown`. Derive `<T>` from the `Iterator()` call's container receiver; applied AFTER
    // the member pass so the composed type wins over the bare member-derived head.
    let iter_overrides = infer_iterator_types(f, &fc, refs, fields, &local_types);
    for (slot, ty) in &iter_overrides {
        local_types.insert(*slot, ty.clone());
    }
    // A3 (illegal-op-round2.md): a `CpyRtoV4/CpyRtoV8 wD` that copies a CALL RESULT out of the
    // value register carries no type, so the slot declares as the write-width default (`int`)
    // and every member use fails "Illegal operation on 'int'". Adopt the call's rendered return
    // type for the DECLARATION only (never fed to the structurer -> body render unchanged).
    // Consumes the iterator subtypes above for bare `TMap*` pair returns.
    let callret_overrides = infer_call_result_types(f, refs, &local_types);
    let body = body_statements_ctor(&fc, refs, depth + 1, super_ctor, Some(&f.ret), fields, Some(&param_types), class_name, Some(&local_types));
    // hoist every referenced local; infer_locals types what it can, the rest default to `int`
    // (a wrong type just becomes a compile error the in-game loop force-stubs, rather than the
    // whole function stubbing on an undeclared identifier).
    let used = used_locals(&body);
    let mut locals = infer_locals(f, refs);
    for &n in &used {
        locals.entry(n).or_insert_with(|| "int".to_string());
    }
    // declare never-written consumer-typed slots with their inferred type (not the cache's wrong one)
    for (slot, ty) in &slot_overrides {
        if used.contains(slot) {
            // Never let a consumer-derived `?` (the AngelScript template type, e.g. an opCast
            // out-param slot) clobber a concrete type already inferred for the slot — declaring
            // `? local_N;` is a syntax error that stubs the whole function. Keep the concrete
            // type (e.g. the opCast retype) when the override is the unusable `?`.
            if ty == "?" && locals.get(slot).map(|t| t != "?").unwrap_or(false) {
                continue;
            }
            // Never let an ARGLESS template head (e.g. a `TArray` opAssign/copy-ctor param whose
            // own DataType carries no SubTypes) downgrade an already-specific instantiation from
            // the authoritative obj_locals type (`TArray<EQuestState>`). Declaring a bare `TArray`
            // is invalid (template needs args) and would stub the function; the cache's recorded
            // object-local type is the better signal, so keep it when the override is just its
            // template head with the `<...>` stripped.
            if !ty.contains('<') {
                if let Some(prev) = locals.get(slot) {
                    if let Some(head) = prev.split('<').next() {
                        if prev.contains('<') && head == ty {
                            continue;
                        }
                    }
                }
            }
            locals.insert(*slot, ty.clone());
        }
    }
    // member-derived declaring-class types override the cache's wrong/general slot type
    for (slot, ty) in &member_overrides {
        if used.contains(slot) {
            locals.insert(*slot, ty.clone());
        }
    }
    // iterator-subtype overrides (A1) replace the bare `T*Iterator` declaration with the
    // container-derived instantiation; never downgrade an already-subtyped declaration.
    for (slot, ty) in &iter_overrides {
        if used.contains(slot) && !locals.get(slot).is_some_and(|t| t.contains('<')) {
            locals.insert(*slot, ty.clone());
        }
    }
    // call-result types (A3): only upgrade a width-guessed PRIMITIVE declaration — an
    // obj_locals / member-derived / iterator-derived object type is a stronger signal
    // (all non-primitive, so they are naturally never overridden here).
    for (slot, ty) in &callret_overrides {
        if used.contains(slot) && locals.get(slot).map(|t| is_primitive(t)).unwrap_or(true) {
            locals.insert(*slot, ty.clone());
        }
    }
    // out-ref float params (batch19 class 3): the callee's `float32&`/`float&`/`double&`
    // signature is authoritative for a PSF'd out-slot — the call cannot compile otherwise
    // ("expected float32&, but got int"). DECLARATION-only (body renders the slot name
    // unchanged), applied LAST over the width-guessed primitive; never clobbers an
    // object/struct declaration (a mis-paired entry would be non-primitive-declared).
    for (slot, ty) in &outref_overrides {
        if used.contains(slot) && locals.get(slot).map(|t| is_primitive(t)).unwrap_or(true) {
            locals.insert(*slot, ty.clone());
        }
    }
    // Drop locals never referenced in the body: `obj_locals` includes profiling temporaries like
    // FScopeCycleCounter / FStatID that the body never uses, and they have no default constructor,
    // so declaring an unused one fails ("No default constructor"). An unused declaration is dead.
    locals.retain(|slot, _| used.contains(slot));
    // arg slots the bytecode reads beyond the declared parameter list (the signature parse
    // undercounts some value-type / defaulted params). Declare them as `int` locals so the
    // body compiles instead of stubbing wholesale; a wrong type the in-game loop force-stubs.
    let mut oor_args: Vec<i32> =
        used_idents(&body, "arg").into_iter().filter(|&n| n as usize >= f.params.len()).collect();
    oor_args.sort_unstable();
    // §3.3 safety net: an unmapped `argN` (signature undercount / RVO-return slot) declared as
    // `int` breaks any member/operator use on it ("Illegal operation on 'int'"). Type it from
    // its CONSUMER instead — the RHS of `argN = <expr>` (a field/local/param whose type we know)
    // — so the declaration is member-compatible. Falls back to `int` when nothing is recoverable.
    let oor_arg_types = infer_oor_arg_types(&body, &oor_args, fields, &locals, &param_types);

    // force-stub functions the in-game compile flagged as unrecoverable (by Class::method).
    let qid = match class_name {
        Some(c) => format!("{c}::{}", f.name),
        None => f.name.clone(),
    };
    let reason = if force_stub_set().contains(&qid) || force_stub_set().contains(&f.name) {
        Some("forced".to_string())
    } else {
        stub_reason(&body, &locals, f.params.len(), ret == "void")
    };

    // Class B1 (emission-classes.md): the cache flags 266 script functions with a REFERENCE
    // return (`FPerceptionHandler& OnSensedSelf(...)` — chainable builders); rendering them
    // by value makes every call site a temporary, which the game compiler rejects ("Cannot
    // call non-const method on a temporary object" / temp into non-const `&` param). Emit
    // `{ty}&` (keeping a leading `const` — meaningful on a ref return) for object-token
    // by-ref value returns. A ref return never has an RVO slot, so `__return` can't occur;
    // the only exposures are the RVODEF default-return and the stub fallback, which declare
    // `{ret} __r;` — invalid for a reference type — so those keep the by-value signature
    // (status quo).
    let ref_ret = f.ret.is_reference && f.ret.token == 5 && !f.ret.is_object_handle;
    // A STUBBED ref-return FPerceptionHandler mixin must NOT degrade to a by-value signature:
    // that poisons every CALLER ("Cannot pass a temporary value ... into non-const reference
    // parameter" at each BindAssessmentToPerception site — one stubbed mixin cascades into
    // dozens of caller stubs). Keep the `&` signature and return a typed non-temporary:
    // `return OnSensedOther(<PerceptionParam>);` — semantically degenerate (like every stub)
    // but signature-faithful, so callers compile.
    let percep_stub_param = (ref_ret
        && f.ret.base_name(refs) == "FPerceptionHandler")
        .then(|| {
            f.params.iter().enumerate().find_map(|(i, p)| {
                (p.ty.base_name(refs) == "UCharacterPerceptionComponent").then(|| {
                    if p.name.is_empty() { format!("arg{i}") } else { p.name.clone() }
                })
            })
        })
        .flatten();
    let ret_sig = if ref_ret
        && ((reason.is_none()
            && !body.contains("__return")
            && (!body.contains(RVODEF) || percep_stub_param.is_some()))
            || (reason.is_some() && percep_stub_param.is_some()))
    {
        format!("{}&", f.ret.render(refs))
    } else {
        ret.clone()
    };
    if f.is_ufunction {
        let _ = writeln!(s, "{ind}UFUNCTION()");
    }
    if is_ctor {
        let _ = writeln!(s, "{ind}{}({params})", f.name); // constructors have no return type
    } else {
        let _ = writeln!(s, "{ind}{ret_sig} {}({params})", f.name);
    }
    let _ = writeln!(s, "{ind}{{");

    if reason.is_none() {
        // Class A (emission-classes.md): FStatID/FScopeCycleCounter have neither a default
        // constructor nor an opAssign, so BOTH the bare hoisted declaration and the later
        // whole-object ctor-assign fail. When every reference to such a local is the
        // write-only `local_N = TY(...);` shape, suppress the hoist and rewrite each
        // assignment to a declaration-with-initializer (in-place construction — the original
        // source form). Any other reference shape keeps the hoist (status quo).
        let (body, suppressed) = rewrite_ctor_only_locals(&body, &locals);
        // FAbilityTaskExecutor's opAssign takes a NON-const reference, so assigning a by-value
        // call result to a declared local ('local = DrawMeleeWeapon(AI);') fails "Cannot pass a
        // temporary value into non-const reference parameter" (2841 in-game errors). The only
        // legal form is declaration-with-initializer (copy-construction). Rewrite qualifying
        // executor locals to decl-init at their assignment sites.
        let (body, na_suppressed) = rewrite_no_assign_locals(&body, &locals);
        // Iterator locals have no default ctor either; declare them at their `Iterator()` call.
        let (body, iter_suppressed) = rewrite_iterator_decl_init(&body, &locals);
        // hoist local declarations; primitives must be initialized (AngelScript errors on
        // "may not be initialized"), objects/structs/handles default-construct themselves.
        for (slot, ty) in &locals {
            if suppressed.contains(slot) || na_suppressed.contains(slot) || iter_suppressed.contains(slot) {
                continue; // declared at its (rewritten) assignment/Iterator() site instead
            }
            if is_primitive(ty) {
                let _ = writeln!(s, "{ind}    {ty} local_{slot} = {};", default_for(ty));
            } else {
                let _ = writeln!(s, "{ind}    {ty} local_{slot};");
            }
        }
        for n in &oor_args {
            match oor_arg_types.get(n) {
                Some(ty) if is_primitive(ty) => {
                    let _ = writeln!(s, "{ind}    {ty} arg{n} = {};", default_for(ty));
                }
                Some(ty) => {
                    // object/struct/handle local: default-constructs itself (no initializer).
                    let _ = writeln!(s, "{ind}    {ty} arg{n};");
                }
                None => {
                    let _ = writeln!(s, "{ind}    int arg{n} = 0;");
                }
            }
        }
        // The hidden RVO return slot is named `__return` by the decompiler. When a store arm
        // (RefCpyV / numeric-cast / etc.) writes that slot inside a branch — `__return = local_4;`
        // — the slot must be a DECLARED local or the module fails to parse ("'__return' is not
        // declared"). Declare it once (typed as the return type) whenever the body references it,
        // and fold the RVODEF default-return into it so there is a single coherent return local.
        // A handle return defaults to null on declaration, so `UFoo __return;` is valid (no
        // "no default constructor" issue that bare struct RVODEF hits).
        let uses_return_slot = body.contains("__return");
        if uses_return_slot {
            let _ = writeln!(s, "{ind}    {ret} __return;");
            // Any unrecovered-default RET in this body returns the same slot.
            s.push_str(&body.replace(RVODEF, "__return"));
        } else if body.contains(RVODEF) {
            // RVODEF marks a return whose value couldn't be recovered: substitute a type-correct
            // default. Object/AActor handles have no default constructor, so `{ret} __r;` fails to
            // compile — return `nullptr` directly (this build's null-handle literal, matching
            // PshNull/CmpPtrNull). `render` strips `@`, so detect handles via the DataType flag.
            if let Some(p) = &percep_stub_param {
                // ref-return FPerceptionHandler mixin with an unrecovered return: a by-value `__r`
                // default would force a by-value SIGNATURE and poison every caller (temporary into
                // non-const ref). Return a typed non-temporary instead; ret_sig keeps the `&`.
                s.push_str(&body.replace(RVODEF, &format!("OnSensedOther({p})")));
            } else if f.ret.is_object_handle {
                s.push_str(&body.replace(RVODEF, "nullptr"));
            } else {
                let _ = writeln!(s, "{ind}    {ret} __r;");
                s.push_str(&body.replace(RVODEF, "__r"));
            }
        } else {
            s.push_str(&body);
        }
    } else {
        // stub fallback so the module still compiles (reason recorded for aggregation)
        let _ = writeln!(s, "{ind}    // body not fully recovered — stub [{}]", reason.unwrap());
        // constructors must NOT return a value; everything else returns a default. An object
        // handle return defaults to `nullptr` (no default-constructor for engine object types;
        // `nullptr` is this build's null-handle literal — `null` parses as undeclared).
        if !is_ctor && ret != "void" {
            if let Some(p) = &percep_stub_param {
                // ref-return FPerceptionHandler mixin: signature-faithful stub (see ret_sig).
                let _ = writeln!(s, "{ind}    return OnSensedOther({p});");
            } else if f.ret.is_object_handle {
                let _ = writeln!(s, "{ind}    return nullptr;");
            } else {
                let _ = writeln!(s, "{ind}    {ret} __r; return __r;");
            }
        }
    }
    let _ = writeln!(s, "{ind}}}");
}

/// AngelScript overload identity ignores parameter NAMES — two functions are the same overload
/// when their parameter TYPES + reference modifiers match. Use this (not `render_params`, which
/// appends names) as the dedup key, so two cache entries with the same name+types but different
/// stored arg-names don't both get emitted (which fails with "a function with the same name and
/// parameters already exists").
fn param_sig(f: &Func, refs: &RefResolver) -> String {
    f.params
        .iter()
        .map(|p| {
            let ty = p.ty.render(refs);
            let amp = if p.ty.is_reference {
                match p.flags & 3 {
                    2 => "&out",
                    3 => "&inout",
                    _ => "&in",
                }
            } else {
                ""
            };
            format!("{ty}{amp}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn render_params(f: &Func, refs: &RefResolver) -> String {
    f.params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let ty = p.ty.render(refs);
            // ParameterFlags: 1=in (asTM_INREF), 2=out, 3=inout (asTM_*); &-ref when reference
            let amp = if p.ty.is_reference {
                match p.flags & 3 {
                    2 => "&out ",
                    3 => "&inout ",
                    _ => "&in ",
                }
            } else {
                ""
            };
            let nm = if p.name.is_empty() { format!("arg{i}") } else { p.name.clone() };
            format!("{ty} {amp}{nm}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A body is "recoverable" only if it has NO recovery gaps: the decompiler emits a
/// `// <mnemonic> ...` comment for every op it can't lower, an unresolved-operand
/// placeholder `?` (e.g. `if (? != ?)`) when a comparison/operand couldn't be recovered,
/// and may reference a `local_N` that wasn't inferred. Any of these is a syntax/semantic
/// error that aborts the module's parse, so such a function falls back to a clean stub.
/// Returns `None` if the body is recoverable, else `Some(reason)` for the first gap found —
/// the reason string is emitted in the stub comment so the stub causes can be aggregated.
fn stub_reason(body: &str, locals: &BTreeMap<i32, String>, param_count: usize, ret_is_void: bool) -> Option<String> {
    // an ARGMISMATCH sentinel (\u{2}<code>) — extract its cause code for aggregation.
    if let Some(i) = body.find('\u{2}') {
        let code: String = body[i + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        return Some(format!("argmismatch:{}", if code.is_empty() { "?" } else { &code }));
    }
    for l in body.lines() {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix("// ") {
            // A structurer bailout (control-flow guard hit) leaves the body truncated, so it
            // MUST still stub — but it's not missing bytecode, so report a distinct cause
            // rather than mislabeling it `opcode-uncovered`.
            let reason = if rest.starts_with("disasm error") {
                "disasm-error"
            } else if rest.starts_with("<structurer bailout>") {
                "structurer-bailout"
            } else {
                "opcode-uncovered"
            };
            return Some(reason.into());
        }
        if t.contains("(? ") || t.contains(" ?)") || t.contains(" ? ") {
            return Some("unresolved-operand".into());
        }
    }
    if !used_locals(body).iter().all(|n| locals.contains_key(n)) {
        return Some("undeclared-local".into());
    }
    let _ = param_count; // out-of-range arg slots are hoisted as locals, not stubbed
    if !ret_is_void && !body.contains("return ") {
        return Some("no-return".into());
    }
    None
}

/// Indices of every `<prefix>N` identifier in a body, at an identifier boundary
/// (so `arg` does not match inside `Target`/`FArg`, and the trailing char isn't alnum).
fn used_idents(body: &str, prefix: &str) -> HashSet<i32> {
    let mut out = HashSet::new();
    let b = body.as_bytes();
    let pl = prefix.len();
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = 0;
    while i + pl < b.len() {
        if &b[i..i + pl] == prefix.as_bytes()
            && (i == 0 || !is_ident(b[i - 1]))
        {
            let mut j = i + pl;
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() { j += 1; }
            if j > start && (j >= b.len() || !is_ident(b[j])) {
                if let Ok(n) = body[start..j].parse::<i32>() { out.insert(n); }
                i = j; continue;
            }
        }
        i += 1;
    }
    out
}

/// Slot indices of every `local_N` identifier referenced in a body.
fn used_locals(body: &str) -> HashSet<i32> {
    let mut out = HashSet::new();
    let b = body.as_bytes();
    let needle = b"local_";
    let mut i = 0;
    while i + needle.len() < b.len() {
        if &b[i..i + needle.len()] == needle {
            let mut j = i + needle.len();
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                if let Ok(n) = body[start..j].parse::<i32>() {
                    out.insert(n);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Consumer-side slot typing: a slot that is NEVER written but is passed as a call argument takes
/// the type that call's parameter expects (e.g. an optional/default-arg slot the cache mis-typed —
/// FName where UAIState_DailyRoutine is wanted, or TSubclassOf<X> where X is wanted). Returns an
/// override `slot -> type` ONLY for never-written slots with a single consistent consumer object
/// type, so it can never clobber a real producer type. Pairs args from the stack TOP against
/// params[0] onward (args are reverse-pushed, so the top entry is the FIRST source arg).
/// True if a return type name denotes a struct/template returned BY VALUE via a hidden RVO
/// out-slot (`F*` engine struct, `T*` template value like TSubclassOf). Enums/primitives/objects
/// return in a register, not an out-slot, so they are excluded.
fn ret_is_struct(ty: &str) -> bool {
    matches!(ty.split('<').next().unwrap_or(ty).bytes().next(), Some(b'F') | Some(b'T'))
}

/// Returns (consumer-typed object-slot overrides, out-ref primitive DECLARATION overrides).
///
/// The second map (batch19 class 3): a slot whose ADDRESS (`PSF`) feeds a `float32&`/`float&`/
/// `double&` reference parameter MUST be declared with exactly that float type — an
/// `&out`/`&inout` primitive reference needs an lvalue of the exact type, and the width-guessed
/// `int` declaration fails "Parameter 'data' expected float32&, but got int" (86 in-game errors,
/// e.g. `AG1RGameState::GetWorldFloatData(world, name, data)`). Declaration-side only: the body
/// already renders the slot NAME at the call site, and float-bits constants already render as
/// float literals via the cbits/float-operand path. `bool&` out-params are NOT retyped: every
/// int-slot render (SetV consts, `(x != 0)` wraps, NOT-patterns) would need a bool form —
/// renderer-wide surgery documented as skipped in specs/batch19-classes.md.
fn infer_slot_types(f: &Func, refs: &RefResolver) -> (HashMap<i32, String>, HashMap<i32, String>) {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return (HashMap::new(), HashMap::new()),
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32);
    let writes = |op: &str| {
        op.starts_with("SetV") || op.starts_with("CpyVtoV") || op.starts_with("CpyRtoV")
            || op.starts_with("RDR") || op.contains("TO") || op == "STOREOBJ" || op == "PopRPtr"
            || op.starts_with("ADD") || op.starts_with("SUB") || op.starts_with("MUL")
            || op.starts_with("DIV") || op.starts_with("MOD") || op.starts_with("NEG")
            || op.starts_with("Inc") || op.starts_with("Dec") || op == "NOT"
            || op.starts_with("B") || op == "ALLOC"
    };
    let mut written: HashSet<i32> = HashSet::new();
    for ins in &instrs {
        if writes(ins.op.name) {
            if let Some(d) = w0(ins) {
                if d > 0 {
                    written.insert(d);
                }
            }
        }
    }
    // operand-stack entries: (slot, pushed_via_PSF) — PSF-ness gates the out-ref pass (only an
    // address push can be a primitive-reference arg; a PshV4 value push never is).
    let mut ostack: Vec<Option<(i32, bool)>> = Vec::new();
    let mut cand: HashMap<i32, Option<String>> = HashMap::new(); // slot -> Some(type) | None(conflict)
    let mut outref: HashMap<i32, Option<String>> = HashMap::new(); // PSF slot -> float-ref param type
    let mut pair = |ostack: &mut Vec<Option<(i32, bool)>>, params: Option<&[super::types::DataType]>, is_method: bool, ret_struct: bool, cand: &mut HashMap<i32, Option<String>>, outref: &mut HashMap<i32, Option<String>>| {
        let Some(params) = params else { ostack.clear(); return; };
        // A method returning a struct BY VALUE (F/T/E) carries a hidden RVO out-slot pushed as the
        // last user arg (just before the receiver); count it so it is consumed, but exclude it from
        // pairing (it is NOT params[last] — pairing it would shift every real arg one param over and
        // mis-type the slot, e.g. the TSubclassOf out param landing on an EQuestState param).
        // A FREE/static struct-returning call pushes the same hidden out-slot ON TOP (no receiver
        // follows it — proven by FInGameTime::Now: `PshGPtr __WorldContext ; PSF <out> ; CALLSYS`,
        // params=[const UObject], ret=FInGameTime). Not counting it paired the PSF'd out-slot with
        // the hidden WorldContext param -> slot mis-typed UObject, clobbering the correct
        // obj_locals type (FInGameTime) and blinding build_call's free-RVO probe ("Can't
        // implicitly convert from 'UObject' to 'FInGameTime'" x144 in-game).
        let rvo = ret_struct as usize;
        let total = if is_method { params.len() + 1 + rvo } else { params.len() + rvo };
        let take = total.min(ostack.len());
        let popped = ostack.split_off(ostack.len() - take);
        // method: top popped entry is the receiver -> drop it (plus the RVO out-slot just below);
        // free call: top popped entry is the RVO out-slot itself. The rest are the user args.
        let args = if is_method && !popped.is_empty() {
            &popped[..popped.len().saturating_sub(1 + rvo)]
        } else {
            &popped[..popped.len().saturating_sub(rvo)]
        };
        // pair from the TOP: bytecode pushes args in REVERSE source order for EVERY call type
        // (proven — see structure.rs maybe_reverse_args), so the TOP entry is params[0], the one
        // below params[1], ... FRONT-anchored pairing is also robust to trailing-default omission
        // (provided args align to the FIRST params) and to a truncated model stack (missing
        // entries are the DEEPEST = trailing args). The previous END-anchored pairing
        // (top <-> params[last]) guarded against a phantom leading `this` in the cache's method
        // param lists — scanning T3 shows that phantom does not exist (every params[0]==owner hit
        // is a genuine operator overload), while end-anchoring REVERSED every multi-param pairing
        // (LocText/HasListenedTo: the FText out-slot re-pushed as `Voiceline` paired with the
        // AGothicCharacterState `Character` param -> wrong declaration, RVO probe miss, dropped
        // string arg — 400+ in-game errors).
        for (i, slot) in args.iter().rev().enumerate() {
            if let Some((s, is_psf)) = slot {
                if let Some(pt) = params.get(i) {
                    let ty = pt.base_name(refs);
                    match cand.get(s) {
                        None => { cand.insert(*s, Some(ty)); }
                        Some(Some(prev)) if *prev != ty => { cand.insert(*s, None); }
                        _ => {}
                    }
                    // out-ref float pass: a PSF'd (address-pushed) slot feeding a float-family
                    // REFERENCE param must be declared with exactly that type (see fn doc).
                    if *is_psf && pt.is_reference && matches!(pt.token, 0x50 | 0x51 | 0x5E) {
                        let kw = super::types::token_keyword(pt.token).to_string();
                        match outref.get(s) {
                            None => { outref.insert(*s, Some(kw)); }
                            Some(Some(prev)) if *prev != kw => { outref.insert(*s, None); }
                            _ => {}
                        }
                    }
                }
            }
        }
    };
    for ins in &instrs {
        match ins.op.name {
            "PshVPtr" | "PshV4" | "PshV8" | "PSF" => {
                let s = w0(ins).unwrap_or(0);
                ostack.push(if s > 0 { Some((s, ins.op.name == "PSF")) } else { None });
            }
            "PshC4" | "PshC8" | "PshNull" | "PGA" | "PshGPtr" | "PshG4" | "PshRPtr" | "STR"
            | "TYPEID" | "OBJTYPE" | "PshListElmnt" => ostack.push(None),
            // P1 (is-not-a-member.md §2.3): ADDSi rewrites the pushed pointer in place into
            // `&slotN.field` — the entry no longer identifies slot N, so a later call-arg
            // pairing must not attribute the FIELD's param type to the BASE slot (e.g.
            // `slot24.AllRequired` feeding `AppendTags(const FGameplayTagContainer&)` typed
            // slot 24 as FGameplayTagContainer). The §2.2 member peephole supplies the
            // correct owner-derived type for these bases instead.
            "ADDSi" => {
                if let Some(top) = ostack.last_mut() {
                    *top = None;
                }
            }
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let rs = refs.func_ret_by_id(id).map(|d| ret_is_struct(&d.base_name(refs))).unwrap_or(false);
                pair(&mut ostack, refs.func_params_by_id(id), refs.is_method_by_id(id), rs, &mut cand, &mut outref);
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                // `$beh0` is the in-place CONSTRUCT behaviour: `<arg> ; PSF <slot> ; CALLSYS $beh0`
                // constructs the value AT the PSF'd receiver slot (top of stack). That slot's type
                // is the construct OWNER (e.g. TSubclassOf<UQuest>), not any callee param — pairing
                // it as an ordinary arg mis-types it (EQuestState). Type the receiver from the owner
                // and consume the behaviour's operands without arg-pairing.
                if refs.func_by_ptr(ptr) == Some("$beh0") {
                    let owner = refs.func_owner_by_ptr(ptr).map(|s| s.to_string());
                    // receiver = top operand; ctor args = the params below it.
                    if let Some(Some((rslot, _))) = ostack.last().copied() {
                        if let Some(ty) = owner {
                            if !ty.is_empty() {
                                match cand.get(&rslot) {
                                    None => { cand.insert(rslot, Some(ty)); }
                                    Some(Some(prev)) if *prev != ty => { cand.insert(rslot, None); }
                                    _ => {}
                                }
                            }
                        }
                    }
                    // pop receiver + ctor args off the operand stack so they don't leak.
                    let nargs = refs.func_params_by_ptr(ptr).map(|p| p.len()).unwrap_or(0);
                    let drop_n = (1 + nargs).min(ostack.len());
                    ostack.truncate(ostack.len() - drop_n);
                    continue;
                }
                let rs = refs.func_ret_by_ptr(ptr).map(|d| ret_is_struct(&d.base_name(refs))).unwrap_or(false);
                pair(&mut ostack, refs.func_params_by_ptr(ptr), refs.is_method_by_ptr(ptr), rs, &mut cand, &mut outref);
            }
            _ => {}
        }
    }
    let obj = cand.into_iter()
        .filter_map(|(slot, ty)| match ty {
            Some(t) if !written.contains(&slot) && !is_primitive(t.trim_end_matches('@')) && t != "void" && !t.is_empty() => Some((slot, t)),
            _ => None,
        })
        .collect();
    // out-ref float slots ARE written (the callee's whole point is to write them; typically a
    // `SetV4 slot, 0` init precedes) — no never-written filter. Conflicts already dropped.
    let outref = outref.into_iter()
        .filter_map(|(slot, ty)| ty.map(|t| (slot, t)))
        .collect();
    (obj, outref)
}

/// Member-access-driven slot typing: a `LoadRObjR`/`LoadVObjR base, off, tid` reads field
/// `member(tid,off)` off the object in `base`; `member_type(tid,off)` is the field's DECLARING
/// class — exactly the type `base` must have for `base.field` to compile. The cache often types
/// the slot too generally (`UObject`) or wrong (`FGameplayTag` for a `FGameplayTagContainer`),
/// or not at all (`int`), producing "<field> is not a member of <T>". Override the slot with the
/// declaring class. Conservative: LOCAL slots only (base > 0), single consistent declaring type
/// (conflict -> drop), non-empty non-primitive.
///
/// A5 extension (illegal-op-round2.md): a `LoadRObjR/LoadVObjR` immediately followed by
/// `CpyRtoV8 wD` copies the member REFERENCE into slot `wD` — the destination's type is the
/// field's VALUE type. That is only recoverable from the emitting class's own fields map (T7
/// PropertyReferences' OldTypeId is the OWNER class, not the value type — see the structure.rs
/// ADDSi arm caveat), so it applies only when the member's owner IS the current class.
fn infer_slot_types_from_members(
    f: &Func,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
    class_name: Option<&str>,
) -> HashMap<i32, String> {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return HashMap::new(),
    };
    let mut cand: HashMap<i32, Option<String>> = HashMap::new();
    // Candidate merge, inheritance-aware (is-not-a-member.md §2.2): one slot can collect the
    // declaring classes of members from BOTH a derived class and its script base (e.g.
    // UAIGroup_Combat's `TargetEnemy` + base UGothicAIGroup's `JoinedCharacters`) — the more
    // DERIVED class has ALL the accessed members, so keep it. A bare vs composed instantiation
    // of the SAME template head keeps the composed one. Anything else (unrelated types /
    // native-only hierarchies / differing compositions) is genuine slot reuse -> drop (None),
    // exactly the pre-existing conservative behaviour.
    let record = |cand: &mut HashMap<i32, Option<String>>, slot: i32, ty: String| {
        if ty.is_empty() || is_primitive(ty.split('<').next().unwrap_or(&ty)) {
            return;
        }
        match cand.get(&slot) {
            None => {
                cand.insert(slot, Some(ty));
            }
            Some(Some(prev)) if *prev != ty => {
                let (ph, nh) = (prev.split('<').next().unwrap_or(prev), ty.split('<').next().unwrap_or(&ty));
                let merged = if ph == nh {
                    match (prev.contains('<'), ty.contains('<')) {
                        (true, false) => Some(prev.clone()),
                        (false, true) => Some(ty),
                        _ => None, // two DIFFERENT compositions of one head: conflict
                    }
                } else if refs.is_subclass(&ty, prev) {
                    Some(ty)
                } else if refs.is_subclass(prev, &ty) {
                    Some(prev.clone())
                } else {
                    None
                };
                cand.insert(slot, merged);
            }
            _ => {}
        }
    };
    for (i, ins) in instrs.iter().enumerate() {
        match ins.op.name {
            "LoadRObjR" | "LoadVObjR" => {
                let base = match ins.words.first() {
                    Some(w) => *w as i16 as i32,
                    None => continue,
                };
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                if base > 0 {
                    // base-slot typing: the field's declaring class (skip this/params).
                    // Composed (§2.1): the bare head of a template instance is a declaration
                    // arity error that turns the whole slot `Unknown`.
                    if let Some(ty) = refs.member_type_composed(tid, off) {
                        record(&mut cand, base, ty);
                    }
                }
                // A5: `LoadRObjR/LoadVObjR ; CpyRtoV8 wD` — type the copy DESTINATION with the
                // field's value type (current-class fields only; the owner check guards
                // against cross-class field-name collisions).
                if let Some(next) = instrs.get(i + 1) {
                    if next.op.name == "CpyRtoV8" {
                        let dst = next.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
                        if dst > 0
                            && class_name.is_some()
                            && refs.type_by_id(tid) == class_name
                        {
                            let vty = refs
                                .member(tid, off)
                                .and_then(|name| fields.and_then(|m| m.get(name)));
                            if let Some(vty) = vty {
                                record(&mut cand, dst, vty.clone());
                            }
                        }
                    }
                }
            }
            // §2.2 sub-case M: Idiom-A member access — `PshVPtr wN`/`PSF wN` immediately
            // followed by `ADDSi off, tid` reads a member off the object in slot N; these
            // bases never appear in LoadRObjR form, so STEP 1 was blind to them. Chained
            // walks (`PshVPtr w0; ADDSi a; RDSPtr; ADDSi b`) skip automatically: the second
            // ADDSi's predecessor is RDSPtr, not a push.
            "ADDSi" => {
                let prev = match i.checked_sub(1).and_then(|j| instrs.get(j)) {
                    Some(p) if matches!(p.op.name, "PshVPtr" | "PSF") => p,
                    _ => continue,
                };
                let base = prev.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
                if base <= 0 {
                    continue; // this / param receivers: locals only (STEP 1 scope)
                }
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                if let Some(ty) = refs.member_type_composed(tid, off) {
                    record(&mut cand, base, ty);
                }
            }
            _ => {}
        }
    }
    cand.into_iter().filter_map(|(s, t)| t.map(|t| (s, t))).collect()
}

/// A1 (illegal-op-round2.md): subtype inference for iterator locals. An `Iterator()` call
/// lowers as `PSF <out> ; ... ; <receiver push> ; CALLSYS <ptr>` — the hidden RVO out-slot
/// is pushed before the container receiver (a PSF/PshVPtr slot, or `PshVPtr w0 ; ADDSi` for
/// a `this.<field>` container), possibly with a whole interleaved call in between. The
/// iterator INSTANCE's T1 entry usually carries no SubTypes, so the out-slot would declare
/// bare (`TArrayConstIterator local_N;` — template-arity error, every use `Unknown`).
/// Compose the type as: head from `func_ret_by_ptr` (TArrayIterator vs TArrayConstIterator
/// vs TMap/TSetIterator) + the container's `<...>` subtype list. Uses the same light
/// operand-stack model as `infer_slot_types` (pushes + per-call consumption) so interleaved
/// calls don't break the out-slot/receiver association. Conservative: unknown-arity calls
/// clear the stack, un-subtyped containers are skipped, an out-slot whose existing bare
/// head disagrees is skipped, already-subtyped slots are never overridden, and conflicting
/// candidates drop (regression-free).
fn infer_iterator_types(
    f: &Func,
    fc: &FuncCode,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
    known: &HashMap<i32, String>,
) -> HashMap<i32, String> {
    let instrs = match disassemble(&fc.bytecode) {
        Ok(i) => i,
        Err(_) => return HashMap::new(),
    };
    // frame offset -> param index, for container receivers that are parameters.
    let (param_off_map, _rvo_off) = super::decompile::build_param_off_map_rvo(fc, &instrs, refs);
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
    // authoritative composed obj-local types: a `$beh0`/call-arg override may have CLOBBERED a
    // slot's entry in `known` down to a bare template head (`TMap`), while the cache's own
    // obj_locals entry composes fully (`TMap<A, B>`). Prefer whichever is subtyped.
    let obj_composed: HashMap<i32, String> = f
        .obj_locals
        .iter()
        .map(|(slot, tinfo)| {
            let ty = super::types::DataType { token: 5, type_info: *tinfo, is_object_handle: true, ..Default::default() }.base_name(refs);
            (*slot, ty)
        })
        .collect();
    // composed container type (e.g. `TArray<AGothicCharacter>`) for a receiver slot.
    let container_of = |slot: i32| -> Option<String> {
        if slot > 0 {
            // local: prefer the subtyped candidate (override map first, then obj_locals).
            return match (known.get(&slot), obj_composed.get(&slot)) {
                (Some(k), _) if k.contains('<') => Some(k.clone()),
                (_, Some(o)) if o.contains('<') => Some(o.clone()),
                (k, o) => k.or(o).cloned(),
            };
        }
        if slot == 0 && fc.is_method {
            return None; // `this` is not a container
        }
        let idx = *param_off_map.get(&slot)?;
        fc.param_types.get(idx).map(|d| d.base_name(refs))
    };
    /// Operand-stack entry: enough to identify a receiver's container and a PSF'd out-slot.
    #[derive(Clone)]
    enum Ent {
        /// `PSF wN` / `PshVPtr wN`.
        Slot { slot: i32, psf: bool },
        /// After `ADDSi` (member access rewrites the top in place): the composed container
        /// type when resolvable (`this.<field>` via the class fields map), else None.
        Member(Option<String>),
        /// Any other push (constants, globals, register re-pushes, value slots).
        Other,
    }
    let mut stack: Vec<Ent> = Vec::new();
    let mut cand: HashMap<i32, Option<String>> = HashMap::new();
    // per-call consumption, mirroring `infer_slot_types::pair`: params + receiver + RVO
    // out-slot for methods; unknown param info -> clear (conservative: drop pending slots).
    let consume = |stack: &mut Vec<Ent>, params: Option<usize>, is_method: bool, ret_struct: bool| {
        let Some(n) = params else { stack.clear(); return; };
        let rvo = (ret_struct && is_method) as usize;
        let total = if is_method { n + 1 + rvo } else { n };
        stack.truncate(stack.len() - total.min(stack.len()));
    };
    for ins in &instrs {
        match ins.op.name {
            "PshVPtr" | "PSF" => {
                stack.push(Ent::Slot { slot: w0(ins), psf: ins.op.name == "PSF" });
            }
            "PshV4" | "PshV8" | "PshC4" | "PshC8" | "PshNull" | "PGA" | "PshGPtr" | "PshG4"
            | "PshRPtr" | "STR" | "TYPEID" | "OBJTYPE" | "PshListElmnt" => stack.push(Ent::Other),
            "ADDSi" => {
                // member access rewrites the pushed pointer in place: `this.<field>` resolves
                // its container type via the class fields map; anything else is unresolvable
                // here (a foreign class's field value type isn't in the tail tables).
                let c = match stack.last() {
                    Some(Ent::Slot { slot: 0, .. }) if fc.is_method => {
                        let off = ins.words.first().copied().unwrap_or(0) as i32;
                        let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                        refs.member(tid, off)
                            .and_then(|name| fields.and_then(|m| m.get(name)))
                            .cloned()
                    }
                    _ => None,
                };
                if let Some(top) = stack.last_mut() {
                    *top = Ent::Member(c);
                }
            }
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let rs = refs.func_ret_by_id(id).map(|d| ret_is_struct(&d.base_name(refs))).unwrap_or(false);
                consume(&mut stack, refs.func_params_by_id(id).map(|p| p.len()), refs.is_method_by_id(id), rs);
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                if refs.func_by_ptr(ptr) == Some("$beh0") {
                    // in-place construct: receiver + ctor args (mirror infer_slot_types).
                    let nargs = refs.func_params_by_ptr(ptr).map(|p| p.len()).unwrap_or(0);
                    let drop_n = (1 + nargs).min(stack.len());
                    stack.truncate(stack.len() - drop_n);
                    continue;
                }
                if refs.func_by_ptr(ptr) == Some("Iterator") && stack.len() >= 2 {
                    // receiver = top; hidden RVO out-slot = the PSF entry directly below.
                    let recv = &stack[stack.len() - 1];
                    let out = &stack[stack.len() - 2];
                    let container = match recv {
                        Ent::Slot { slot, .. } => container_of(*slot),
                        Ent::Member(c) => c.clone(),
                        Ent::Other => None,
                    };
                    if let Ent::Slot { slot: out_slot, psf: true } = *out {
                        record_iterator_candidate(refs, known, &mut cand, out_slot, ptr, container);
                    }
                }
                let rs = refs.func_ret_by_ptr(ptr).map(|d| ret_is_struct(&d.base_name(refs))).unwrap_or(false);
                consume(&mut stack, refs.func_params_by_ptr(ptr).map(|p| p.len()), refs.is_method_by_ptr(ptr), rs);
            }
            _ => {}
        }
    }
    cand.into_iter().filter_map(|(s, t)| t.map(|t| (s, t))).collect()
}

/// Compose + record one iterator out-slot candidate (see [`infer_iterator_types`]).
fn record_iterator_candidate(
    refs: &RefResolver,
    known: &HashMap<i32, String>,
    cand: &mut HashMap<i32, Option<String>>,
    out_slot: i32,
    ptr: i64,
    container: Option<String>,
) {
    if out_slot <= 0 {
        return; // only local out-slots get declarations
    }
    // never override a slot that already has a subtyped (template-complete) type.
    if known.get(&out_slot).is_some_and(|t| t.contains('<')) {
        return;
    }
    // head = the Iterator() return type. Iterator INSTANCES usually serialize BARE (then the
    // container's `<...>` subtype list is appended), but some (TMap) compose fully — use
    // those directly, no container needed.
    let Some(head) = refs.func_ret_by_ptr(ptr).map(|d| d.base_name(refs)) else { return };
    if !head.starts_with('T') || !head.contains("Iterator") {
        return;
    }
    let ty = if head.contains('<') {
        head
    } else {
        // the container must itself be a subtyped template (`TArray<AGothicCharacter>`).
        let Some(c) = container else { return };
        let (Some(lt), Some(gt)) = (c.find('<'), c.rfind('>')) else { return };
        if gt <= lt + 1 {
            return;
        }
        format!("{head}<{}>", &c[lt + 1..gt])
    };
    // stack-model safety: if the slot already has a BARE type (member-pass derived), its head
    // must agree with ours — a mismatch means a mis-associated out-slot, so skip.
    if let Some(prev) = known.get(&out_slot) {
        if !prev.is_empty() && ty.split('<').next() != Some(prev.as_str()) {
            return;
        }
    }
    match cand.get(&out_slot) {
        None => {
            cand.insert(out_slot, Some(ty));
        }
        Some(Some(prev)) if *prev != ty => {
            cand.insert(out_slot, None); // slot reused across containers: drop
        }
        _ => {}
    }
}

/// A3 (illegal-op-round2.md): call-result register-copy typing. A reference/element-returning
/// call (`X.Proceed()`, `pair.GetKey()`, ...) leaves its result in the VALUE register; the
/// following `CpyRtoV4/CpyRtoV8 wD` copies it into slot `wD`, which `infer_locals` can only
/// width-guess (`int`/`int64`) — so `local_38 = local_28.Proceed(); local_38.GetActor...()`
/// fails "Illegal operation on 'int'". Track the last CALL/CALLINTF/CALLBND/CALLSYS/Thiscall1
/// and its rendered return type; when the next instruction (only benign SUSPENDs between) is a
/// `CpyRtoV*` into a local slot, adopt the return type for that slot's DECLARATION.
/// Conservative: never for obj_locals slots, never for slots also written by non-CpyRtoV ops
/// (int/float scratch reuse), never primitives/enums/`?`, and a BARE template head (e.g. the
/// `TMap*` iterator pair) is only adopted after composing `<...>` from the receiver iterator's
/// inferred instantiation (`known`, which includes the A1 pass results) — else skipped.
/// Conflicting candidates for one slot drop (regression-free).
fn infer_call_result_types(
    f: &Func,
    refs: &RefResolver,
    known: &HashMap<i32, String>,
) -> HashMap<i32, String> {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return HashMap::new(),
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
    // slots also written by a non-CpyRtoV writing op are int/float scratch (slot reuse) — never
    // adopt an object type for them. ADDSi is excluded: its first word is a member OFFSET.
    let writes_other = |op: &str| {
        op != "ADDSi"
            && (op.starts_with("SetV") || op.starts_with("CpyVtoV") || op.starts_with("RDR")
                || op.contains("TO") || op == "PopRPtr"
                || op.starts_with("ADD") || op.starts_with("SUB") || op.starts_with("MUL")
                || op.starts_with("DIV") || op.starts_with("MOD") || op.starts_with("NEG")
                || op.starts_with("Inc") || op.starts_with("Dec") || op == "NOT")
    };
    let mut disq: HashSet<i32> = HashSet::new();
    for ins in &instrs {
        if writes_other(ins.op.name) {
            let d = w0(ins);
            if d > 0 {
                disq.insert(d);
            }
        }
    }
    let obj: HashSet<i32> = f.obj_locals.iter().map(|(s, _)| *s).collect();
    // per-call operand-stack consumption, mirroring `infer_slot_types`.
    let consume = |stack: &mut Vec<Option<i32>>, params: Option<usize>, is_method: bool, ret_struct: bool| {
        let Some(n) = params else { stack.clear(); return; };
        let rvo = (ret_struct && is_method) as usize;
        let total = if is_method { n + 1 + rvo } else { n };
        stack.truncate(stack.len() - total.min(stack.len()));
    };
    // rendered return type + receiver slot of the just-completed call (None once anything
    // non-benign executes, so a later CpyRtoV can't adopt a stale type).
    let mut last: Option<(String, Option<i32>)> = None;
    let mut ostack: Vec<Option<i32>> = Vec::new();
    let mut cand: HashMap<i32, Option<String>> = HashMap::new();
    for ins in &instrs {
        match ins.op.name {
            "PshVPtr" | "PSF" => {
                let s = w0(ins);
                ostack.push((s > 0).then_some(s));
                last = None;
            }
            "PshV4" | "PshV8" | "PshC4" | "PshC8" | "PshNull" | "PGA" | "PshGPtr" | "PshG4"
            | "PshRPtr" | "STR" | "TYPEID" | "OBJTYPE" | "PshListElmnt" => {
                ostack.push(None);
                last = None;
            }
            "ADDSi" => {
                // member access rewrites the pushed pointer: no longer the bare slot.
                if let Some(top) = ostack.last_mut() {
                    *top = None;
                }
                last = None;
            }
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let is_m = refs.is_method_by_id(id);
                let recv = if is_m { ostack.last().copied().flatten() } else { None };
                let ret = refs.func_ret_by_id(id).map(|d| d.base_name(refs));
                let rs = ret.as_deref().map(ret_is_struct).unwrap_or(false);
                consume(&mut ostack, refs.func_params_by_id(id).map(|p| p.len()), is_m, rs);
                last = ret.map(|t| (t, recv));
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                if refs.func_by_ptr(ptr) == Some("$beh0") {
                    // in-place construct: receiver + ctor args, no register result.
                    let nargs = refs.func_params_by_ptr(ptr).map(|p| p.len()).unwrap_or(0);
                    let drop_n = (1 + nargs).min(ostack.len());
                    ostack.truncate(ostack.len() - drop_n);
                    last = None;
                    continue;
                }
                let is_m = refs.is_method_by_ptr(ptr);
                let recv = if is_m { ostack.last().copied().flatten() } else { None };
                let ret = refs.func_ret_by_ptr(ptr).map(|d| d.base_name(refs));
                let rs = ret.as_deref().map(ret_is_struct).unwrap_or(false);
                consume(&mut ostack, refs.func_params_by_ptr(ptr).map(|p| p.len()), is_m, rs);
                last = ret.map(|t| (t, recv));
            }
            "CpyRtoV4" | "CpyRtoV8" => {
                if let Some((ty, recv)) = last.take() {
                    let d = w0(ins);
                    if d > 0 && !obj.contains(&d) && !disq.contains(&d) {
                        if let Some(ty) = usable_ret_type(ty, recv, known) {
                            match cand.get(&d) {
                                None => {
                                    cand.insert(d, Some(ty));
                                }
                                Some(Some(prev)) if *prev != ty => {
                                    cand.insert(d, None); // slot reused across types: drop
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            "SUSPEND" => {} // benign: doesn't touch the value register
            _ => {
                last = None;
            }
        }
    }
    cand.into_iter().filter_map(|(s, t)| t.map(|t| (s, t))).collect()
}

/// Filter/compose one A3 return-type candidate (see [`infer_call_result_types`]):
/// primitives/enums/placeholders are rejected (int declarations already work for them); a bare
/// template head (`TMapIteratorPair` with no `<...>` in its T1 entry) is composed from the
/// receiver iterator's inferred instantiation, or rejected when that isn't available either.
fn usable_ret_type(ty: String, recv: Option<i32>, known: &HashMap<i32, String>) -> Option<String> {
    if ty.is_empty() || ty == "void" || ty == "?" || ty == "auto" || is_primitive(&ty) || is_enum(&ty) {
        return None;
    }
    let b = ty.as_bytes();
    let bare_template = !ty.contains('<') && b.len() >= 2 && b[0] == b'T' && b[1].is_ascii_uppercase();
    if !bare_template {
        return Some(ty);
    }
    // compose the bare head from the receiver's iterator instantiation (A1 pass result).
    let r = known.get(&recv?)?;
    if !r.split('<').next().unwrap_or(r).contains("Iterator") {
        return None;
    }
    let (lt, gt) = (r.find('<')?, r.rfind('>')?);
    if gt <= lt + 1 {
        return None;
    }
    Some(format!("{ty}<{}>", &r[lt + 1..gt]))
}

/// §3.3 consumer-driven typing of out-of-range `argN` slots. Scans the body for the RHS of an
/// `argN = <expr>` assignment (including `return argN = <expr>;`) and resolves `<expr>`'s type
/// from the maps we already have: `this.<field>` -> field type, `local_M` -> local type,
/// `<param>` -> param type. A type that supports member access makes `argN.Member` legal where a
/// bare `int` would not. Anything unresolved is left out (declared `int`, as before — no regression).
fn infer_oor_arg_types(
    body: &str,
    oor_args: &[i32],
    fields: Option<&HashMap<String, String>>,
    locals: &BTreeMap<i32, String>,
    param_types: &[String],
) -> HashMap<i32, String> {
    let mut out: HashMap<i32, String> = HashMap::new();
    if oor_args.is_empty() {
        return out;
    }
    // a primitive/enum int-ish RHS isn't worth retyping (int default already works); only adopt a
    // type that is NOT a bare primitive (i.e. a struct/handle/array the member access needs).
    let adopt = |out: &mut HashMap<i32, String>, n: i32, ty: String| {
        if !ty.is_empty() && !is_primitive(&ty) {
            out.entry(n).or_insert(ty);
        }
    };
    for line in body.lines() {
        let t = line.trim();
        let t = t.strip_prefix("return ").unwrap_or(t);
        // parse `argN = RHS;`
        let Some(rest) = t.strip_prefix("arg") else { continue };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(n) = digits.parse::<i32>() else { continue };
        if !oor_args.contains(&n) {
            continue;
        }
        let after = rest[digits.len()..].trim_start();
        let Some(rhs) = after.strip_prefix("= ") else { continue };
        let rhs = rhs.trim().trim_end_matches(';').trim();
        // `this.<field>` (single member hop) -> the field's type.
        if let Some(field) = rhs.strip_prefix("this.") {
            if !field.contains('.') && !field.contains('(') {
                if let Some(ty) = fields.and_then(|m| m.get(field)) {
                    adopt(&mut out, n, ty.clone());
                    continue;
                }
            }
        }
        // `local_M` -> that local's inferred type.
        if let Some(m) = rhs.strip_prefix("local_") {
            if let Ok(slot) = m.parse::<i32>() {
                if let Some(ty) = locals.get(&slot) {
                    adopt(&mut out, n, ty.clone());
                    continue;
                }
            }
        }
        // a bare parameter name -> its declared type (param_types is index-aligned to params,
        // but we only have names in the body; skip — names aren't carried here). Left for future.
        let _ = param_types;
    }
    out
}

/// Types with NEITHER a default constructor NOR an opAssign in this AS binding (emission-
/// classes.md Class A): a hoisted bare declaration fails ("No default constructor") AND the
/// later whole-object assignment fails ("No appropriate opAssign"); the only legal form is
/// declaration-with-initializer (in-place construction).
fn has_no_default_ctor(ty: &str) -> bool {
    matches!(ty, "FStatID" | "FScopeCycleCounter")
}

/// Count identifier-boundary occurrences of `ident` in `line` (so `local_3` does not match
/// inside `local_32` or `local_3_2`).
fn count_ident(line: &str, ident: &str) -> usize {
    let (b, ib) = (line.as_bytes(), ident.as_bytes());
    let is_id = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let (mut n, mut i) = (0usize, 0usize);
    while i + ib.len() <= b.len() {
        if &b[i..i + ib.len()] == ib
            && (i == 0 || !is_id(b[i - 1]))
            && (i + ib.len() == b.len() || !is_id(b[i + ib.len()]))
        {
            n += 1;
            i += ib.len();
        } else {
            i += 1;
        }
    }
    n
}

/// Class A (emission-classes.md): for a hoisted local whose type has no default ctor AND no
/// opAssign (`FStatID`/`FScopeCycleCounter`), when EVERY body reference is the write-only
/// whole-object ctor-assign `local_N = TY(...);`, suppress the hoisted declaration and
/// rewrite each assignment to a declaration-with-initializer `TY local_N = TY(...);`. A
/// 2nd..nth assignment to the same (compiler-reused) slot gets a fresh name (`local_N_2`) so
/// sibling-scope re-declarations of one name are avoided. If any reference does NOT match
/// the pattern (a read), the local keeps its hoist (status-quo error, never force-stub).
/// Returns the rewritten body + the slots whose hoisted declaration must be suppressed.
fn rewrite_ctor_only_locals(body: &str, locals: &BTreeMap<i32, String>) -> (String, HashSet<i32>) {
    let mut suppressed: HashSet<i32> = HashSet::new();
    let mut out = body.to_string();
    for (slot, ty) in locals {
        if !has_no_default_ctor(ty) {
            continue;
        }
        let ident = format!("local_{slot}");
        let pat = format!("{ident} = {ty}(");
        // gate: every referencing line is exactly `local_N = TY(...);` with a single occurrence.
        let mut assigns = 0usize;
        let mut ok = true;
        for line in out.lines() {
            match count_ident(line, &ident) {
                0 => continue,
                1 => {
                    let t = line.trim_start();
                    if t.starts_with(&pat) && t.ends_with(");") {
                        assigns += 1;
                    } else {
                        ok = false;
                        break;
                    }
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok || assigns == 0 {
            continue;
        }
        let mut k = 0usize;
        let mut rewritten = String::with_capacity(out.len() + 32);
        for line in out.lines() {
            let t = line.trim_start();
            if count_ident(line, &ident) == 1 && t.starts_with(&pat) && t.ends_with(");") {
                k += 1;
                let indent = &line[..line.len() - t.len()];
                let rest = &t[ident.len()..]; // ` = TY(...);`
                if k == 1 {
                    let _ = writeln!(rewritten, "{indent}{ty} {ident}{rest}");
                } else {
                    let _ = writeln!(rewritten, "{indent}{ty} {ident}_{k}{rest}");
                }
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        out = rewritten;
        suppressed.insert(*slot);
    }
    (out, suppressed)
}

/// Value types whose `opAssign` takes a NON-const reference: assigning a by-value call result
/// (`local = DrawMeleeWeapon(AI);`) fails "Cannot pass a temporary value ... into non-const
/// reference parameter"; only declaration-with-initializer (copy-construction) compiles.
fn no_assign_type(ty: &str) -> bool {
    matches!(ty, "FAbilityTaskExecutor")
}

/// Rewrite locals of a no-assign type (see [`no_assign_type`]) to declaration-with-initializer.
/// The bytecode reuses one slot for several source-level locals, so each assignment gets a fresh
/// declaration. Two safe shapes:
/// - WRITE-ONLY (any number of assignments, no reads): every assignment becomes
///   `TY local_N[_k] = ...;` with fresh names — nothing else references the slot, so sinking the
///   declarations into blocks is scope-safe (mirrors the FStatID rewrite).
/// - SINGLE assignment + reads, ALL references at the same indentation (one block): decl-init in
///   place, name kept — later reads are lvalue uses (a non-const ref binds to an lvalue; only
///   temporaries fail), and same-indent means the declaration dominates every read.
/// Anything else (read before assign, cross-block reads, self-referential RHS) keeps the hoisted
/// declaration — the status-quo compile error, never a new one.
fn rewrite_no_assign_locals(body: &str, locals: &BTreeMap<i32, String>) -> (String, HashSet<i32>) {
    let mut suppressed: HashSet<i32> = HashSet::new();
    let mut out = body.to_string();
    for (slot, ty) in locals {
        if !no_assign_type(ty) {
            continue;
        }
        let ident = format!("local_{slot}");
        let assign_pat = format!("{ident} = ");
        let mut assigns = 0usize;
        let mut reads = 0usize;
        let mut first_is_assign = false;
        let mut first = true;
        let mut indents: Vec<&str> = Vec::new();
        let mut ok = true;
        for line in out.lines() {
            let c = count_ident(line, &ident);
            if c == 0 {
                continue;
            }
            let t = line.trim_start();
            let is_assign = c == 1 && t.starts_with(&assign_pat) && t.ends_with(';');
            if first {
                first_is_assign = is_assign;
                first = false;
            }
            if is_assign {
                assigns += 1;
            } else {
                reads += c;
                if c > 1 {
                    ok = false; // multi-occurrence non-assign line (self-referential etc.) — bail
                    break;
                }
            }
            indents.push(&line[..line.len() - t.len()]);
        }
        if !ok || assigns == 0 || !first_is_assign {
            continue;
        }
        let write_only = reads == 0;
        let uniform_indent = indents.windows(2).all(|w| w[0] == w[1]);
        if !write_only && !(assigns == 1 && uniform_indent) {
            continue;
        }
        let mut k = 0usize;
        let mut rewritten = String::with_capacity(out.len() + 64);
        for line in out.lines() {
            let t = line.trim_start();
            if count_ident(line, &ident) == 1 && t.starts_with(&assign_pat) && t.ends_with(';') {
                k += 1;
                let indent = &line[..line.len() - t.len()];
                let rest = &t[ident.len()..]; // ` = <expr>;`
                if k == 1 {
                    let _ = writeln!(rewritten, "{indent}{ty} {ident}{rest}");
                } else {
                    let _ = writeln!(rewritten, "{indent}{ty} {ident}_{k}{rest}");
                }
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        out = rewritten;
        suppressed.insert(*slot);
    }
    (out, suppressed)
}

/// Iterator locals (`TArrayIterator<T>`, `TSetIterator<T>`, `TMapIterator<T>`, ... incl. Const
/// variants) have NO default constructor, so a bare hoisted `TArrayIterator<T> local_N;` fails
/// "No default constructor". The only legal form is declaration-with-initializer from the
/// `Iterator()` call that produces it: `TArrayIterator<T> local_N = container.Iterator();`. Unlike
/// the ctor-only (FStatID) case an iterator IS read afterwards (in its loop), so the gate is only
/// that the local's FIRST mention in the body is a single-`ident` assignment `local_N = ...;` —
/// then decl-init at that site is valid and nothing reads it before. Returns the rewritten body +
/// the set of slots whose hoisted declaration must be suppressed.
fn rewrite_iterator_decl_init(body: &str, locals: &BTreeMap<i32, String>) -> (String, HashSet<i32>) {
    let is_iter = |ty: &str| {
        let h = ty.split('<').next().unwrap_or(ty);
        matches!(h, "TArrayIterator" | "TArrayConstIterator" | "TSetIterator" | "TSetConstIterator"
            | "TMapIterator" | "TMapConstIterator")
    };
    let mut suppressed: HashSet<i32> = HashSet::new();
    let mut out = body.to_string();
    for (slot, ty) in locals {
        if !is_iter(ty) {
            continue;
        }
        let ident = format!("local_{slot}");
        let pat = format!("{ident} = ");
        // The FIRST line mentioning the ident must be a single-occurrence assignment `local_N = …;`.
        let mut first_ok = false;
        for line in out.lines() {
            if count_ident(line, &ident) == 0 {
                continue;
            }
            let t = line.trim_start();
            first_ok = count_ident(line, &ident) == 1 && t.starts_with(&pat) && t.ends_with(';');
            break;
        }
        if !first_ok {
            continue;
        }
        // Rewrite ONLY that first assignment to a decl-init; leave later reads untouched.
        let mut done = false;
        let mut rewritten = String::with_capacity(out.len() + 32);
        for line in out.lines() {
            let t = line.trim_start();
            if !done && count_ident(line, &ident) == 1 && t.starts_with(&pat) && t.ends_with(';') {
                let indent = &line[..line.len() - t.len()];
                // `auto`, not the inferred iterator type: the cache's recorded return type does
                // not reliably distinguish Const vs mutable Iterator overloads (the receiver's
                // constness decides in-source), so a spelled-out head fails "Can't implicitly
                // convert TXIterator<T> <-> TXConstIterator<T>" whenever we guess wrong (72
                // in-game errors; the batch-9 const-flip regression was the same coin's other
                // face). AngelScript's auto infers the exact overload the compiler resolves.
                let _ = writeln!(rewritten, "{indent}auto {t}");
                done = true;
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        if done {
            out = rewritten;
            suppressed.insert(*slot);
        }
    }
    (out, suppressed)
}

/// Infer (slot, type) for primitive + object locals to hoist as declarations.
fn infer_locals(f: &Func, refs: &RefResolver) -> BTreeMap<i32, String> {
    let mut locals: BTreeMap<i32, String> = BTreeMap::new();
    let obj: BTreeMap<i32, i64> = f.obj_locals.iter().copied().collect();
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return locals,
    };
    for ins in &instrs {
        let n = ins.op.name;
        // destination slot is the first word for writing ops
        let dst = ins.words.first().map(|w| *w as i16 as i32);
        let Some(dst) = dst else { continue };
        if dst <= 0 {
            continue; // params / this, not a local
        }
        let ty = if let Some(p) = obj.get(&dst) {
            // reuse DataType::render (template subtypes + @ only on ref types U*/A*)
            super::types::DataType {
                token: 5,
                type_info: *p,
                is_object_handle: true,
                ..Default::default()
            }
            .render(refs)
        } else if writes_float(n) {
            "float".to_string()
        } else if writes_double(n) {
            "double".to_string()
        } else if writes_int64(n) {
            "int64".to_string()
        } else if writes_int(n) {
            "int".to_string()
        } else {
            continue;
        };
        // A slot first written by an ambiguous constant (SetV4/SetV8 -> int/int64) may later
        // be written by a typed float/double op; let the wider/typed write refine the guess
        // instead of locking in the first. Object types (from `obj`) are never downgraded.
        let rank = |s: &str| match s {
            "int" => 1,
            "int64" => 2,
            "float" | "double" => 3,
            _ => 0, // object/other
        };
        match locals.get(&dst) {
            None => {
                locals.insert(dst, ty);
            }
            Some(prev) if rank(prev) > 0 && rank(&ty) > rank(prev) => {
                locals.insert(dst, ty);
            }
            _ => {}
        }
    }
    // opCast retype: a script-handle downcast `T@ dst = Cast<T>(src)` lowers to
    // `TYPEID <tid> ; PSF <dst> ; PshVPtr <src> ; CALLSYS opCast`, and the cache types the
    // out-slot `dst` as the AngelScript `?` template type. Declaring `? local_N;` is a syntax
    // error ("Expected expression value, instead found '?'") that stubs the whole function.
    // Retype `dst` to the cast's resolved target T (from the preceding TYPEID) so it declares
    // as e.g. `UGothicFinalDataGame local_N;` and the recovered `local_N = Cast<T>(src);`
    // type-checks. This is the declaration-side counterpart of the structure.rs opCast recovery.
    {
        let mut last_tid: Option<i32> = None;
        let mut last_psf: Option<i32> = None;
        for ins in &instrs {
            match ins.op.name {
                "TYPEID" => {
                    last_tid = ins.dwords.first().map(|d| *d as i32);
                    last_psf = None;
                }
                "PSF" => {
                    // first PSF after a TYPEID is the opCast out-slot destination
                    if last_tid.is_some() {
                        last_psf = ins.words.first().map(|w| *w as i16 as i32);
                    }
                }
                "CALLSYS" | "Thiscall1" => {
                    let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                    if refs.func_by_ptr(ptr) == Some("opCast") {
                        if let (Some(tid), Some(dst)) = (last_tid, last_psf) {
                            if dst > 0 {
                                if let Some(t) = super::structure::resolve_cast_typeid(refs, tid) {
                                    if t.starts_with('U') || t.starts_with('A') {
                                        locals.insert(dst, t);
                                    }
                                }
                            }
                        }
                    }
                    last_tid = None;
                    last_psf = None;
                }
                _ => {}
            }
        }
    }
    let _ = token_keyword; // keep import used if obj path elided
    locals
}

fn writes_int(n: &str) -> bool {
    matches!(n, "SetV4" | "SetV1" | "ADDi" | "SUBi" | "MULi" | "DIVi" | "MODi" | "IncVi" | "DecVi"
        | "NEGi" | "BAND" | "BOR" | "BXOR" | "BSLL" | "BSRA" | "ADDIi" | "SUBIi" | "MULIi"
        | "CpyVtoR4" | "RDR4" | "CpyRtoV4"
        // conversions whose RESULT is a 32-bit int/uint (*TO i/u/b/w)
        | "fTOi" | "fTOu" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" | "dTOi" | "dTOu"
        | "iTOb" | "iTOw" | "i64TOi")
}
fn writes_float(n: &str) -> bool {
    matches!(n, "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf"
        | "ADDIf" | "SUBIf" | "MULIf"
        // conversions whose RESULT is float (*TO f)
        | "iTOf" | "uTOf" | "dTOf" | "i64TOf" | "u64TOf")
}
fn writes_double(n: &str) -> bool {
    matches!(n, "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd"
        // conversions whose RESULT is double (*TO d)
        | "iTOd" | "uTOd" | "fTOd" | "i64TOd" | "u64TOd")
}
fn writes_int64(n: &str) -> bool {
    matches!(n, "SetV8" | "ADDi64" | "SUBi64" | "MULi64" | "DIVi64"
        // conversions whose RESULT is a 64-bit int/uint (*TO i64/u64)
        | "uTOi64" | "iTOi64" | "fTOi64" | "dTOi64" | "fTOu64" | "dTOu64")
}

/// Heuristic: a UE enum type name (`E` + uppercase), which is int-castable like a primitive.
fn is_enum(ty: &str) -> bool {
    let b = ty.as_bytes();
    b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase()
}

/// True for AngelScript primitive scalar types (need an explicit initializer).
fn is_primitive(ty: &str) -> bool {
    matches!(ty,
        "bool" | "int" | "int8" | "int16" | "int64" | "uint" | "uint8" | "uint16" | "uint64"
        | "float" | "float32" | "double")
}

/// A default initializer literal for a base type.
fn default_for(ty: &str) -> String {
    match ty {
        "float" | "double" | "float32" => "0.0".into(),
        "bool" => "false".into(),
        _ => "0".into(),
    }
}

/// Render a global's stored u64 constant per its rendered type.
fn render_const(ty: &str, v: u64) -> String {
    match ty {
        // This build is `floatIsFloat64` (types.rs): token 0x51 `float` is a 64-bit value (the
        // full u64), like `double` — render it WITHOUT the `f` suffix (which would round to
        // 32-bit / pick the wrong literal). Only token 0x50 `float32` is 32-bit and takes `f`.
        // Matches structure.rs's `fmt_float` (no suffix for 64-bit).
        "float" | "double" => format!("{}", f64::from_bits(v)),
        "float32" => format!("{}f", f32::from_bits(v as u32)),
        "bool" => if v != 0 { "true".into() } else { "false".into() },
        // Render integers per their actual width AND signedness: an unsigned type must not be
        // emitted negative (e.g. uint64 0xffff…ffff as -1), and signed types sign-extend from
        // their own width (the value lives in the low bits of the stored u64).
        "uint64" => v.to_string(),
        "uint" => (v as u32).to_string(),
        "uint16" => (v as u16).to_string(),
        "uint8" => (v as u8).to_string(),
        "int64" => (v as i64).to_string(),
        "int16" => (v as i16).to_string(),
        "int8" => (v as i8).to_string(),
        // "int" and any other int-like fallback: 32-bit signed.
        _ => (v as i32).to_string(),
    }
}

/// Is this module-level function a generator-synthesized accessor (skip it)?
fn is_generated(
    f: &Func,
    class_names: &HashSet<&str>,
    class_members: &HashMap<&str, HashSet<&str>>,
) -> bool {
    if f.name == "StaticClass" || class_names.contains(f.name.as_str()) {
        return true;
    }
    // A function whose namespace is a class is the already-emitted method ONLY if the class
    // actually declares a member with this name; a genuine free function that merely shares the
    // namespace is kept (previously it was silently dropped).
    class_members
        .get(f.namespace.as_str())
        .is_some_and(|members| members.contains(f.name.as_str()))
}
