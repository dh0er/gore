//! Recompilable `.as` emitter: a parsed [`model::Module`] -> AngelScript source the
//! GAME compiler accepts (per `work/reversing/gore-as/findings/recompile-*.md`).
//!
//! Rules: flat top-level file (module name is the file PATH, not a namespace, so no
//! wrapper); no `import` (automaticImports=1); `class X : Super`; UFUNCTION()/UPROPERTY()
//! only when the stored flag is set; skip generator-synthesized symbols (StaticClass,
//! the class-name ctor wrapper). Function bodies come from the structured decompiler with
//! hoisted local declarations; bodies the decompiler can't recover fall back to a
//! signature-matched STUB so the module still compiles.

use super::disasm::Instr;
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
            .map(|s| {
                s.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// A method's `const` qualifier (asTRAIT_CONST) is part of its identity in the cache's
/// FunctionReferences table, so dropping it produces a DIFFERENT symbol and every reference to
/// it fails to resolve when the module is remapped back onto the base cache. It is therefore
/// re-emitted for every method the cache marks const, EXCEPT where the recovered body would no
/// longer compile under a read-only receiver.
///
/// This replaced a hand-maintained allowlist of ~20 individually verified methods. That list
/// existed because a blanket re-emit once cost 636 in-game compile errors, but the recovered
/// bodies have improved since: on the current tree, restoring all 6,247 qualifiers costs a
/// single family, which the body check below covers exactly.
/// True when the recovered body calls a NON-const method on `this`, which a `const` receiver
/// cannot do ("Non-const method call on read-only object reference").
///
/// The cache's own const flag says what the ORIGINAL body was allowed to do; the recovered body
/// can differ — most often by calling a helper through `this` that the original reached some
/// other way. Checking the body keeps the qualifier wherever it still holds instead of falling
/// back to a hand-maintained list.
fn body_calls_non_const_method(body: &str, class_name: Option<&str>, refs: &RefResolver) -> bool {
    let Some(class) = class_name else {
        return false;
    };
    let bytes = body.as_bytes();
    let mut index = 0usize;
    while let Some(found) = body[index..].find("this.") {
        let start = index + found + "this.".len();
        let mut end = start;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        // Only a CALL constrains the receiver; a member read is fine on a const object.
        // A method that also exists as a CONST overload does not constrain the caller: the call
        // resolves to that overload. The accessor pairs (`T& f()` / `const T& f() const`) are
        // exactly this, and treating them as non-const cost their callers their own `const`.
        let called = &body[start..end];
        if bytes.get(end) == Some(&b'(')
            && refs.calls_non_const_method(class, called)
            && !refs.has_const_overload(class, called)
        {
            return true;
        }
        index = end.max(start);
    }
    false
}

use super::default_source::{recover as recover_defaults, DefaultsRecovery};
use super::disasm::disassemble;
use super::model::{Class, Func, Module};
use super::refs::RefResolver;
use super::structure::{body_statements_ctor, CONSTSTORE, RVODEF};
use super::types::token_keyword;
use super::walk_modules::FuncCode;

/// Exact number of source functions for which [`emit_module`] writes a body.
///
/// This deliberately follows the same generated-accessor, generated-default,
/// delegate-wrapper, and duplicate-signature filters as the emitter. Counting
/// raw cache records would overstate the denominator because those records are
/// not represented by editable function bodies in the emitted source.
pub(crate) fn emitted_body_count(m: &Module, refs: &RefResolver) -> usize {
    let class_names: HashSet<&str> = m.classes.iter().map(|c| c.name.as_str()).collect();
    let class_members: HashMap<&str, HashSet<&str>> = m
        .classes
        .iter()
        .map(|c| {
            let members = c
                .methods
                .iter()
                .chain(c.ctors.iter())
                .map(|f| f.name.as_str())
                .collect();
            (c.name.as_str(), members)
        })
        .collect();

    let mut total = 0usize;
    for class in &m.classes {
        // A generated delegate wrapper is reconstructed as one declaration with
        // no source body; none of its cached implementation methods are emitted.
        if delegate_wrapper_decl(class, refs).is_some() {
            continue;
        }
        total += class.ctors.len();
        let mut seen = HashSet::new();
        total += class
            .methods
            .iter()
            .filter(|method| !method.name.starts_with("__"))
            .filter(|method| seen.insert(format!("{}({})", method.name, param_sig(method, refs))))
            .count();
    }

    let mut seen_free = HashSet::new();
    total
        + m.functions
            .iter()
            .filter(|function| {
                !is_generated(function, &class_names, &class_members)
                    && !is_generated_spawn(function, refs)
            })
            .filter(|function| {
                seen_free.insert(format!("{}({})", function.name, param_sig(function, refs)))
            })
            .count()
}

/// Emit a whole module as recompilable AngelScript.
/// Emit one module WITHOUT class `default` statements.
///
/// This is the shape every existing consumer expects: the compile baseline, the sealed NPC
/// archetype evidence, the qualification digests, and the source Mod Studio hands to its edit
/// flow all either hash this text or feed it back to the compiler. Writing defaults is opted
/// INTO through [`emit_module_with`], so no caller acquires them by accident.
pub fn emit_module(m: &Module, refs: &RefResolver) -> String {
    emit_module_with(m, refs, false)
}

/// Emit one module.
///
/// `class_defaults` writes the class-scope `default` statements recovered from the generated
/// `__InitDefaults` — everything an item, NPC or config class is made of. It is OFF by default
/// everywhere, because a module that authors defaults makes the compiler regenerate that
/// method, and the remap behind `compile-module --op edit` does not yet follow the references
/// the regenerated body introduces; source that is going to be hashed or recompiled must keep
/// the historical shape. `gore as emit` / `emit-all` turn it on for the human reading it.
pub fn emit_module_with(m: &Module, refs: &RefResolver, class_defaults: bool) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "// gore-as decompiled module: {} ({})", m.name, m.file);
    let _ = writeln!(
        s,
        "// NOTE: local names + string literals are not stored in the cache.\n"
    );

    let mut namespaces = NamespaceWriter::default();
    for e in &m.enums {
        namespaces.enter(&mut s, &e.namespace);
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

    // A global declared inside a namespace must be re-declared inside one, because every
    // reference site renders it qualified (`Location::Spawnpoint`) — emitting the declaration at
    // global scope makes the compiler reject the reference with "Unknown scope 'Location'".
    for g in &m.globals {
        if g.name.starts_with("__") {
            continue; // generator-synthesized (e.g. __StaticType_X)
        }
        let namespace = g.namespace.as_str();
        namespaces.enter(&mut s, namespace);
        let indent = if namespace.is_empty() { "" } else { "    " };
        let base = g.ty.base_name(refs);
        if !is_primitive(&base) && !is_enum(&base) {
            // AngelScript globals MUST be const, but an FName/F-struct can't take `= 0`.
            // FName constants are almost always named after their value -> `n"Name"`; other
            // structs get a default-constructed const (their real value isn't recoverable).
            if base == "FName" {
                // The declaration alone does not carry the value; the global's initializer
                // bytecode does, as the `__STATIC_NAME` index it pushes. Falling back to the
                // global's own name is a guess that is right often enough to look correct and
                // wrong where it matters (`Spawnpoint` really holds `LOCATION_Spawnpoint`).
                let literal = fname_initializer(g, refs).unwrap_or_else(|| g.name.clone());
                let _ = writeln!(s, "{indent}const FName {} = n\"{literal}\";", g.name);
            } else {
                let _ = writeln!(s, "{indent}const {base} {} = {base}();", g.name);
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
            let _ = writeln!(s, "{indent}const {base} {} = {base}({inner});", g.name);
        } else {
            let _ = writeln!(s, "{indent}const {base} {} = {inner};", g.name);
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

    let (module_defaults, defaults_note) = if class_defaults {
        recover_module_defaults(m, refs)
    } else {
        (HashMap::new(), None)
    };
    if let Some(note) = &defaults_note {
        let _ = writeln!(
            s,
            "// NOTE: class defaults are not authored in this module: {note}."
        );
        let _ = writeln!(
            s,
            "// They are carried over byte-exact when this module is recompiled.\n"
        );
    }

    for c in &m.classes {
        // batch-24c: a compiler-generated delegate/event wrapper class re-emits as its ORIGINAL
        // one-line declaration (the verbatim class can never compile — see the detector's doc).
        namespaces.enter(&mut s, &c.namespace);
        if let Some(decl) = delegate_wrapper_decl(c, refs) {
            s.push_str(&decl);
            continue;
        }
        emit_class(
            &mut s,
            c,
            refs,
            module_defaults.get(c.name.as_str()).map(Vec::as_slice),
        );
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
        namespaces.enter(&mut s, &f.namespace);
        emit_function(&mut s, f, refs, false, false, 0);
    }
    namespaces.close(&mut s);
    s
}

/// The AngelScript-UE binding auto-generates factory free functions for every actor/component
/// class. The cache also carries them as module functions, so emitting them duplicates the native
/// binding ("a function with the same name and parameters already exists" — un-stubbable, the
/// declaration itself collides). Skip the exact generated shapes:
///   - actor:     `<Actor> Spawn(const FVector&, const FRotator&, const FName&, bool, ULevel)`
///   - component: `<Comp> Get|GetOrCreate|Create(const AActor, const FName&)`
pub(crate) fn is_generated_spawn(f: &Func, refs: &RefResolver) -> bool {
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

/// Batch-24c (specs/batch23-waitseconds.md Class 2 Shape A): the Hazelight compiler
/// AUTO-GENERATES a wrapper class for every `delegate` / `event` declaration — an `_Inner`
/// field typed `_FScriptDelegate` (single-cast) / `_FMulticastScriptDelegate` (multicast)
/// plus Execute*/Broadcast/Bind/Add methods built on the compiler intrinsics
/// (`__DelegateSignature`, `__Evt_PushArgument*`, `__Evt_ExecuteDelegate`). The cache stores
/// the GENERATED class; re-emitting it verbatim can never compile (the intrinsics and the
/// generated copy/assign forms have no source form: 17 cannot-pass + 12 cant-convert errors
/// + 12 copyctor raw stubs across 12 wrapper classes). Detect the wrapper structurally and
/// return the original one-line declaration — the byte-faithful form, from which the
/// compiler regenerates the identical wrapper:
///
/// ```angelscript
/// event void FPlayerBeginOverlapFireGolemArenaEvent(AActor Actor);
/// delegate void FSoulHarvestVisualDelegate(ASoulHarvestCharacter_Visual CurrentInstance);
/// ```
///
/// (decl syntax per the vendored fork docs, hazelight-docs/page_scripting_delegates_.html).
/// Gates (belt-and-braces against a hand-written lookalike): no super class, EXACTLY one
/// field named `_Inner` of the internal delegate type, and the signature-carrier method
/// present — `Broadcast` (event) / `Execute` or `ExecuteIfBound` (delegate), whose params
/// (names cache-preserved) and return type form the declared signature.
fn delegate_wrapper_decl(c: &Class, refs: &RefResolver) -> Option<String> {
    if c.super_class.as_deref().is_some_and(|s| !s.is_empty()) {
        return None;
    }
    let [field] = c.fields.as_slice() else {
        return None;
    };
    if field.name != "_Inner" {
        return None;
    }
    let (kw, carrier) = match field.ty.base_name(refs).as_str() {
        "_FMulticastScriptDelegate" => ("event", c.methods.iter().find(|f| f.name == "Broadcast")?),
        "_FScriptDelegate" => (
            "delegate",
            // prefer `Execute` (carries a RetVal delegate's return type); `ExecuteIfBound`
            // as the fallback carrier (void-returning delegates always have it).
            c.methods
                .iter()
                .find(|f| f.name == "Execute")
                .or_else(|| c.methods.iter().find(|f| f.name == "ExecuteIfBound"))?,
        ),
        _ => return None,
    };
    let ret = carrier
        .ret
        .render(refs)
        .trim_start_matches("const ")
        .to_string();
    Some(format!(
        "{kw} {ret} {}({});\n\n",
        c.name,
        render_params(carrier, refs)
    ))
}

/// Qualify a rendered declaration type with its namespace when the name alone would not resolve.
///
/// Most type strings already come from `DataType::render`, which qualifies. A local's type can
/// also be inferred from a call owner or a construct behaviour, and those channels only ever had
/// the bare name — which silently declares an `Unknown` local as soon as the type lives in a
/// namespace. Only the head is qualified; template arguments render through their own path.
fn qualify_decl_type(ty: &str, refs: &RefResolver) -> String {
    if ty.contains("::") {
        return ty.to_string();
    }
    let (head, rest) = match ty.split_once('<') {
        Some((head, rest)) => (head, Some(rest)),
        None => (ty, None),
    };
    let Some(namespace) = refs.type_ns_by_name(head) else {
        return ty.to_string();
    };
    match rest {
        Some(rest) => format!("{namespace}::{head}<{rest}"),
        None => format!("{namespace}::{head}"),
    }
}

/// Tracks which AngelScript namespace block is currently open while a module is written.
///
/// A declaration's namespace is part of its identity in the cache's reference tables, so a
/// recompile that drops it produces a DIFFERENT symbol: `UQuest_NewCamp`, declared in
/// `G1R::Quest`, came back as a global-scope class and every reference to it then failed to
/// resolve against the base cache. Declaration ORDER is preserved (it maps onto the cache's
/// tables), so the block opens and closes as the namespace changes rather than grouping
/// declarations by namespace.
#[derive(Default)]
struct NamespaceWriter<'a> {
    open: Option<&'a str>,
}

impl<'a> NamespaceWriter<'a> {
    /// Enter `namespace`, closing whatever else was open. An empty name is global scope.
    fn enter(&mut self, s: &mut String, namespace: &'a str) {
        if self.open == Some(namespace) {
            return;
        }
        self.close(s);
        if !namespace.is_empty() {
            let _ = writeln!(s, "namespace {namespace}");
            let _ = writeln!(s, "{{");
        }
        self.open = Some(namespace);
    }

    fn close(&mut self, s: &mut String) {
        if self.open.is_some_and(|namespace| !namespace.is_empty()) {
            let _ = writeln!(s, "}}");
        }
        self.open = None;
    }
}

/// Name of the one compiler-generated method that holds a class's default statements.
const INIT_DEFAULTS: &str = "__InitDefaults";

/// Largest `__InitDefaults` bytecode, in dwords, that default-statement recovery will attempt.
/// Recovery is superlinear in statement count, and the few initializers above this bound are
/// machine-generated world/voice tables rather than authored defaults.
const MAX_INIT_DEFAULTS_DWORDS: usize = 1_000_000;

/// The recovery bound, overridable per run through `GORE_AS_MAX_DEFAULTS_DWORDS`. The default
/// covers the whole shipped corpus, including the machine-generated main-map worldpoint table
/// (852k dwords, the largest initializer in the game); lower it for a faster emit.
fn max_init_defaults_dwords() -> usize {
    std::env::var("GORE_AS_MAX_DEFAULTS_DWORDS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(MAX_INIT_DEFAULTS_DWORDS)
}

/// Recover the class-scope `default` statements of every class in a module.
///
/// All-or-nothing per MODULE, deliberately. `generated_defaults` can carry an omitted
/// `__InitDefaults` record through a recompile byte-exact, but only for a module whose authored
/// source declares no defaults at all — once the source authors one, the compiler regenerates
/// the method from that source and the carried copy would be stale. A module authoring SOME of
/// its defaults would therefore silently drop the rest, which is game data loss. So one class
/// that cannot be recovered suppresses the whole module, which lands it back on the byte-exact
/// carry-through.
///
/// Returns the per-class statements plus, when the module was suppressed, the reason to record
/// in the emitted header.
fn recover_module_defaults<'a>(
    m: &'a Module,
    refs: &RefResolver,
) -> (HashMap<&'a str, Vec<String>>, Option<String>) {
    let mut recovered: HashMap<&str, Vec<String>> = HashMap::new();
    for c in &m.classes {
        // A delegate/event wrapper re-emits as its original one-line declaration, so it has no
        // class body to hold statements.
        if delegate_wrapper_decl(c, refs).is_some() {
            continue;
        }
        let Some(init) = c.methods.iter().find(|f| f.name == INIT_DEFAULTS) else {
            continue;
        };
        // Cost gate, checked before the expensive render. A handful of generated manager and
        // voice-table initializers are enormous (the largest is 389k instructions of unrolled
        // `AddWorldPoint`/`AddWorldPosition` calls); recovering them would dominate emit time
        // for tables no one hand-authors. They fall back to the byte-exact carry.
        let bound = max_init_defaults_dwords();
        if init.bytecode.len() > bound {
            return (
                HashMap::new(),
                Some(format!(
                    "{} (initializer is {} dwords, over the {bound} recovery bound)",
                    c.name,
                    init.bytecode.len()
                )),
            );
        }
        let fields = class_field_types(c, refs);
        let mut rendered = String::new();
        emit_function_ctor(
            &mut rendered,
            init,
            refs,
            true,
            false,
            1,
            None,
            Some(&fields),
            Some(&c.name),
        );
        if std::env::var_os("GORE_AS_DEFAULTS_DEBUG").is_some() {
            eprintln!(
                "[defaults] ---- {} ----
{rendered}",
                c.name
            );
        }
        match recover_defaults(&rendered) {
            DefaultsRecovery::Recovered(statements) => {
                if let Some(lost) = statements
                    .iter()
                    .find_map(|statement| call_that_lost_its_arguments(statement, refs))
                {
                    return (
                        HashMap::new(),
                        Some(format!(
                            "{} (call `{lost}()` has no no-argument form, so the recovered                              statement lost an argument)",
                            c.name
                        )),
                    );
                }
                recovered.insert(c.name.as_str(), statements);
            }
            DefaultsRecovery::Rejected(reason) => {
                return (HashMap::new(), Some(format!("{} ({reason})", c.name)));
            }
        }
    }
    (recovered, None)
}

/// Field name -> base type name for a class, including every INHERITED script field, so the body
/// renderer can type `this.field = <int>` stores and cast them. Own fields win a name collision
/// (shadowing is illegal in AngelScript anyway); the super walk is cycle-bounded.
///
/// Batch-21 Class B residue: the own-fields map left e.g. `this.RoleCategoryContainers.opIndex(
/// <int>)` (field declared on the super class) untyped, so the container-subtype enum-key wrap
/// never fired.
/// A rendered `Name()` whose function the cache knows only WITH parameters. The structurer lost
/// the argument (the fluent AI-rule chains lose their receiver and argument together), and a
/// `default` statement written from it would silently mean something else — so the class falls
/// back to the byte-exact carry. A type name is a constructor, not a call, and is exempt.
fn call_that_lost_its_arguments(statement: &str, refs: &RefResolver) -> Option<String> {
    let bytes = statement.as_bytes();
    let mut at = 0usize;
    while let Some(found) = statement[at..].find("()") {
        let end = at + found;
        let mut start = end;
        while start > 0 {
            let b = bytes[start - 1];
            if b.is_ascii_alphanumeric() || b == b'_' {
                start -= 1;
            } else {
                break;
            }
        }
        let name = &statement[start..end];
        if !name.is_empty()
            && !name.bytes().next().is_some_and(|b| b.is_ascii_digit())
            && !refs.is_type_name(name)
            && !refs.zero_arg_call_is_plausible(name)
        {
            return Some(name.to_owned());
        }
        at = end + 2;
    }
    None
}

fn class_field_types(c: &Class, refs: &RefResolver) -> HashMap<String, String> {
    let mut field_types: HashMap<String, String> = c
        .fields
        .iter()
        .map(|f| (f.name.clone(), f.ty.base_name(refs)))
        .collect();
    let mut cur = c
        .super_class
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    for _ in 0..64 {
        let Some(sup) = cur else { break };
        if let Some(fs) = refs.class_field_types(&sup) {
            for (k, v) in fs {
                field_types.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
        cur = refs.class_super_of(&sup).map(|s| s.to_string());
    }
    field_types
}

fn emit_class(s: &mut String, c: &Class, refs: &RefResolver, defaults: Option<&[String]>) {
    // batch-30a (C6b, specs/batch29-errortail.md §6): the cache Class record's asOBJ_*
    // Flags discriminate script VALUE types (asOBJ_VALUE, 0x2 — vanilla `struct`) from
    // reference types (asOBJ_REF, 0x1 — vanilla `class`). Emitting the 46 value types as
    // `class` gave them Hazelight REFERENCE semantics: `const T` params became const
    // HANDLES, so ctor bodies value-assigning from them failed ("Can't implicitly convert
    // from 'const FVoiceReactionData' to 'FVoiceReactionData'") and const& arguments could
    // not bind ("Cannot pass a reference of type 'FCrimeSeverityStackValues const&' into
    // non-const reference parameter"). The corpus proves the discriminator: every compiling
    // `ctor(const F* &inout P) { this.X = P; }` site uses a NATIVE struct; the only failing
    // ones use script types flagged asOBJ_VALUE. `struct` restores value semantics (the
    // compiler generates the const&in opAssign, matching the native-struct behaviour) and
    // is the byte-faithful vanilla keyword. Value types here never carry a super class,
    // and delegate/event wrappers (also VALUE-flagged) take the one-liner path above.
    let kw = if c.flags & 0x2 != 0 {
        "struct"
    } else {
        "class"
    };
    match &c.super_class {
        Some(sup) if !sup.is_empty() => {
            let _ = writeln!(s, "{kw} {} : {}", c.name, sup);
        }
        _ => {
            let _ = writeln!(s, "{kw} {}", c.name);
        }
    }
    let _ = writeln!(s, "{{");
    let super_name = c.super_class.as_deref().filter(|s| !s.is_empty());
    let field_types = class_field_types(c, refs);
    // The constructors are rendered FIRST, into their own buffer: a constructor that does
    // nothing but give members their values is the compiler's lowering of member INITIALIZERS,
    // and those belong on the declarations below. Written as a constructor body instead, the
    // member is default-constructed first — a behaviour the base cache may not have, which
    // costs the module its splicability.
    let mut constructors = String::new();
    for ctor in &c.ctors {
        emit_function_ctor(
            &mut constructors,
            ctor,
            refs,
            true,
            true,
            1,
            super_name,
            Some(&field_types),
            Some(&c.name),
        );
    }
    let member_initializers = extract_member_initializers(&mut constructors);
    for f in &c.fields {
        // Drop a leading `const`: UE-AS UPROPERTY members aren't const-assignable, yet the
        // generated constructor assigns them — keeping `const` causes "Cannot assign" errors.
        let ty = f.ty.render(refs);
        let ty = ty.strip_prefix("const ").unwrap_or(&ty);
        if f.is_uproperty {
            let _ = writeln!(s, "    UPROPERTY()");
        }
        match member_initializers.get(&f.name) {
            Some(value) => {
                let _ = writeln!(s, "    {ty} {} = {value};", f.name);
            }
            None => {
                let _ = writeln!(s, "    {ty} {};", f.name);
            }
        }
    }
    if !c.fields.is_empty() {
        s.push('\n');
    }
    // Class-scope default statements, recovered from the compiler-generated `__InitDefaults`.
    // They carry everything a data class IS (name, value, damage, icon, tags), so a class
    // without them decompiles to an empty shell. Written before the constructors, matching the
    // layout the game's own AngelScript source generator uses.
    if let Some(statements) = defaults.filter(|d| !d.is_empty()) {
        for statement in statements {
            let _ = writeln!(s, "    default {statement}");
        }
        s.push('\n');
    }
    s.push_str(&constructors);
    // Dedup methods by name+parameters+const: the cache can carry two entries that render to the
    // same signature, which AngelScript rejects as "a function with the same name and parameters
    // already exists". A method's own `const` qualifier is part of what distinguishes them,
    // though: `T f()` next to `const T f() const` is the ordinary accessor pair, and keying
    // without it dropped the const half of every one of them.
    let mut seen_sigs: HashSet<String> = HashSet::new();
    for m in &c.methods {
        // `__InitDefaults` (and other `__`-prefixed generator methods) set the CDO defaults
        // via raw `__StaticType_*` symbols and untyped literals we can't reconstruct offline;
        // they are auto-generated boilerplate, not hand-written script — skip them so the
        // class compiles. (Runtime UPROPERTY defaults are lost; real script logic is intact.)
        if m.name.starts_with("__") {
            continue;
        }
        let const_method = m.is_const_method();
        if !seen_sigs.insert(format!(
            "{}({}){}",
            m.name,
            param_sig(m, refs),
            const_method
        )) {
            if std::env::var_os("GORE_AS_DUP_DIAG").is_some() {
                eprintln!(
                    "[dup] {}::{}({}) traits={:#x} const_method={} ret={} ret_const={}",
                    c.name,
                    m.name,
                    param_sig(m, refs),
                    m.traits,
                    m.is_const_method(),
                    m.ret.render(refs),
                    m.ret.is_object_const || m.ret.is_read_only,
                );
            }
            continue; // duplicate signature
        }
        emit_function_ctor(
            s,
            m,
            refs,
            true,
            false,
            1,
            None,
            Some(&field_types),
            Some(&c.name),
        );
    }
    let _ = writeln!(s, "}}\n");
}

fn emit_function(
    s: &mut String,
    f: &Func,
    refs: &RefResolver,
    is_method: bool,
    is_ctor: bool,
    depth: usize,
) {
    emit_function_ctor(s, f, refs, is_method, is_ctor, depth, None, None, None);
}

#[allow(clippy::too_many_arguments)]
fn emit_function_ctor(
    s: &mut String,
    f: &Func,
    refs: &RefResolver,
    is_method: bool,
    is_ctor: bool,
    depth: usize,
    super_ctor: Option<&str>,
    fields: Option<&HashMap<String, String>>,
    class_name: Option<&str>,
) {
    let ind = "    ".repeat(depth);
    // The return type keeps its `const`: it is part of the method identity the base cache
    // recorded, and a caller that stores such a value declares its local const to match (see
    // `infer_const_result_slots`). The exception is a name whose rows DISAGREE about it — an
    // override family has to declare one return type, so those keep the stripped form, and with
    // it the caller-side locals that could not be const either.
    let ret = if refs.const_return_is_inconsistent(&f.name, is_method && f.is_const_method()) {
        f.ret.render(refs).trim_start_matches("const ").to_string()
    } else {
        f.ret.render(refs)
    };
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
    let mut local_types: HashMap<i32, String> = f
        .obj_locals
        .iter()
        .map(|(slot, tinfo)| {
            let ty = super::types::DataType {
                token: 5,
                type_info: *tinfo,
                is_object_handle: true,
                ..Default::default()
            }
            .base_name(refs);
            (*slot, ty)
        })
        .collect();
    // batch-30a (C6c): snapshot the cache's own (vanilla) object-local types before any
    // override mutates the map — the member-override gate below compares against these.
    let vanilla_obj_types = local_types.clone();
    // batch-34: member-access-derived declaring class per slot (the field's OWNER type is a
    // TYPE LOWER-BOUND: the slot's real type must have that member). Computed BEFORE the
    // call-arg (`slot_overrides`) pass so that pass cannot widen a slot below a type that
    // provably has an accessed member. `member_widen_below` below turns it into a guard.
    let member_overrides = infer_slot_types_from_members(f, refs, fields, class_name);
    // A candidate `cand` for `slot` widens BELOW a member lower-bound iff the slot has member
    // evidence `lb` that the VANILLA type provably satisfies (`is_subclass(vanilla, lb)`) while
    // `cand` does NOT provably satisfy it (`!is_subclass(cand, lb)`). Comparing against the
    // vanilla type (which compiled every member access in the real source) sidesteps the gaps
    // in KNOWN_NATIVE_HIERARCHY — e.g. ACharacter's native chain up to AActor is absent, so a
    // direct `is_subclass(ACharacter, AActor)` can't see the widen, but `is_subclass(vanilla=
    // AGothicCharacter, lb=ACharacter)` holds and `is_subclass(cand=AActor, ACharacter)` does
    // not, correctly rejecting the AActor widen (AInvulnerableVisual::SetUpCollisions,
    // ARagdollFallingActor::Initialize, UGA_Death_Meatbug — `.CapsuleComponent` off AActor).
    let member_widen_below = |slot: &i32, cand: &String| {
        let Some(lb) = member_overrides.get(slot) else {
            return false;
        };
        let Some(vanilla) = vanilla_obj_types.get(slot) else {
            return false;
        };
        cand != lb && refs.is_subclass(vanilla, lb) && !refs.is_subclass(cand, lb)
    };
    // consumer-side override: never-written arg slots get the type their callee expects (fixes
    // mis-typed default/optional-arg slots — FName->UAIState_DailyRoutine, TSubclassOf<X>->X).
    // outref_overrides: PSF slots feeding float-family REFERENCE params (declaration-only, below).
    // float_args/keep_ints (batch-25g): numeric BY-VALUE arg slots — see structure::ArgSlotHints.
    let (slot_overrides, outref_overrides, float_args, keep_ints, int_refs, small_args) =
        infer_slot_types(f, refs);
    // batch-28 (specs/batch27-floatwarnings.md §2): unified numeric-kind dataflow pass.
    // Anti-seed imports: slots value-pushed into int-family params (keep_ints), PSF-bound to
    // int-family reference params (int_refs), or bool&-bound (outref "bool") hold int/bool
    // bits at that point — float evidence on the same slot is slot reuse -> poison.
    let num_anti: HashSet<i32> = keep_ints
        .iter()
        .chain(int_refs.iter())
        .copied()
        // non-float out-ref bindings hold non-float data at the call — bool (batch-28) and
        // the batch-31c enum/struct out-slots alike; float evidence on them is slot reuse.
        .chain(
            outref_overrides
                .iter()
                .filter(|(_, t)| !matches!(t.as_str(), "float" | "float32" | "double"))
                .map(|(s, _)| *s),
        )
        .collect();
    let numkinds = infer_float_flow(
        f,
        &fc,
        refs,
        fields,
        &float_args,
        &outref_overrides,
        &num_anti,
    );
    let mut enum_overrides = infer_enum_flow(f, refs);
    enum_overrides.retain(|slot, ty| {
        !numkinds.contains_key(slot)
            && !float_args.contains_key(slot)
            && !keep_ints.contains(slot)
            && !int_refs.contains(slot)
            && !small_args.contains_key(slot)
            && outref_overrides.get(slot).is_none_or(|other| other == ty)
            && slot_overrides.get(slot).is_none_or(|other| other == ty)
    });
    // Keep the bool refinement scoped to the same proven enum state-machine slice. This is the
    // only place it is required for semantic lowering, and avoids changing unrelated constructor
    // scratch solely because it happens to initialize a bool field.
    let mut bool_overrides = if enum_overrides.is_empty() {
        HashSet::new()
    } else {
        infer_bool_field_slots(f, refs, fields)
    };
    // `NOT` is AngelScript's LOGICAL not and exists only for a bool variable, so wherever it
    // appears its operand slot is a bool. Left as an int the body writes
    // `local = int(local == 0);` and the compiler materializes a comparison vanilla never had.
    bool_overrides.extend(not_operand_slots(f));
    bool_overrides.extend(bool_call_result_slots(f, refs));
    bool_overrides.extend(comparison_result_slots(f));
    // The type each slot's captured call result actually has — the witness the operand gates
    // need to tell a plain read from a read the declaration converts.
    let call_result_types = call_result_types(f, refs);
    // Producers the SOURCE kept as statements of their own — see `statement_producer_slots`.
    let statement_producers = statement_producer_slots(f);
    // Where vanilla destroyed a value slot mid-expression it was calling on a temporary there.
    let temporary_receivers = temporary_receiver_slots(f, refs);
    // The elements of a range-for: released once per iteration, and what the loop fold matches.
    let loop_elements = loop_element_slots(f);
    // Where a widening's result was copied ON, the source named it; folding that name away
    // changes the width the arithmetic behind it happens at.
    let widened = widened_slots(f);
    // Slots the function does not touch until it has branched: their declaration lives in the
    // block that touches them, not at the top.
    let touched_after_branch = slots_touched_only_after_a_branch(f);
    // Handles vanilla ALIASED into a slot of their own: that alias is a name the source wrote.
    let aliased = handle_alias_slots(f);
    // How often each slot is default-constructed: more than once and the source spelled the
    // temporary out at every use.
    let constructions = default_construction_counts(f, refs);
    // The same witness types the slots a bool travels INTO: the merge slot of a short-circuit
    // or a guarded assignment is written by a copy, not by the call itself.
    bool_overrides.extend(
        call_result_types
            .iter()
            .filter(|(_, ty)| *ty == "bool")
            .map(|(slot, _)| *slot),
    );
    // A slot read ONE BYTE wide into the value register is a bool: `CpyVtoR1 v5` is how a branch
    // takes its condition, and nothing else reads a slot that way. Typing such a merge slot int
    // writes the short circuit vanilla merged into one condition as an int carrier over two arms.
    bool_overrides.extend(byte_read_slots(f));
    // A slot the function itself writes a 4- or 8-byte literal into is not a bool: the compiler
    // stores a bool with the 1-byte `SetV1`. That store is the slot's own width evidence, and it
    // outranks both rules above — where they disagree the compiler reused one slot for two
    // things, and typing it bool writes `local = false;` where vanilla wrote an int.
    let wide_stores = wide_literal_store_slots(f);
    // Unless the same slot is READ one byte wide. `CpyVtoR1 v5` takes v5 as a bool — the compiler
    // reads a branch value that way and no other — so a slot written `SetV4 v5, 0` and read
    // `CpyVtoR1 v5` is a bool the compiler happened to clear four bytes of. Typing it int writes
    // the short circuit vanilla merged into one condition as an int carrier over two arms.
    let read_as_bool = byte_read_slots(f);
    bool_overrides.retain(|slot| !wide_stores.contains(slot) || read_as_bool.contains(slot));
    // `NOT` proves the slot is a bool outright, so it outranks the int-family USE hints (a slot
    // pushed into an int parameter): those describe how a value is passed, not what it is, and
    // leaving them in charge writes `local = int(local == 0);` for a plain `!`. Contradicting
    // TYPE evidence (enum, float family, a declared parameter type) still wins — that is slot
    // reuse, and typing it bool would break the other use.
    // `NOT` and the callee's declared return type are both PROOFS of the slot's type; the
    // int-family hints below only describe how a value is passed.
    let mut proven_bool = not_operand_slots(f);
    proven_bool.extend(comparison_result_slots(f));
    proven_bool.extend(
        call_result_types
            .iter()
            .filter(|(_, ty)| *ty == "bool")
            .map(|(slot, _)| *slot),
    );
    bool_overrides.retain(|slot| {
        let proven = proven_bool.contains(slot);
        !enum_overrides.contains_key(slot)
            && !numkinds.contains_key(slot)
            && !float_args.contains_key(slot)
            && (proven || !keep_ints.contains(slot))
            && (proven || !int_refs.contains(slot))
            && (proven || !small_args.contains_key(slot))
            && outref_overrides
                .get(slot)
                .is_none_or(|other| other == "bool")
            && slot_overrides.get(slot).is_none_or(|other| other == "bool")
    });
    for (slot, ty) in &slot_overrides {
        // batch-29c (C2, specs/batch29-errortail.md §2): never let an ARGLESS template head
        // (the $beh0 construct OWNER / a copy-ctor param whose DataType carries no SubTypes —
        // bare `TSubclassOf`) downgrade an already-composed body type from the authoritative
        // obj_locals entry (`TSubclassOf<UGameplayAbility>`). The $beh0/value-ctor renders
        // compose their type from THIS map, and the bare head emitted
        // `local_170 = TSubclassOf(local_106);` ("Template 'TSubclassOf' expects 1 sub
        // type(s)" + paired ctor no-match, 52 [E] lines). Mirror of the declaration-side
        // gate below — the DECL already kept the composed name, only the body map regressed.
        if !ty.contains('<') {
            if let Some(prev) = local_types.get(slot) {
                if prev.contains('<') && prev.split('<').next() == Some(ty.as_str()) {
                    continue;
                }
            }
        }
        // batch-31d (N7, spec batch31-nomatch-illegalop §1.7): most-derived merge — a
        // param-derived candidate that is a PROVABLE ANCESTOR of the cache's own obj_locals
        // type must not widen it. The vanilla static type compiled every use in the vanilla
        // source (member access on the ancestor stays legal on the derived type), while the
        // widened type loses derived-only natives: the OldCamp `GetAllNPCStates()` loop
        // element (obj_locals AGothicNPCState) paired with a guard CALL's
        // AGothicCharacterState param -> `local_24.RemoveFromWorld()` on the BASE type,
        // 5× "No matching signatures" (the FreeMine twins, guard-free, kept NPCState and
        // compile). Hierarchy comparability via is_subclass (script-super walk +
        // KNOWN_NATIVE_HIERARCHY).
        if let Some(vanilla) = vanilla_obj_types.get(slot) {
            if vanilla != ty && refs.is_subclass(vanilla, ty) {
                continue;
            }
        }
        // batch-34: member-access lower-bound. A call-arg candidate (e.g. `AActor` from an
        // `IgnoreActorWhenMoving(AActor, bool)`/`SetOwner(AActor)` param) must not widen a slot
        // below a type that provably has a member the body reads off it — the batch-31d guard
        // above only fires when the widen target is an is_subclass-visible ANCESTOR of vanilla,
        // but the native chain ACharacter->..->AActor is absent from KNOWN_NATIVE_HIERARCHY, so
        // AActor slips past it. Anchoring on the member's declaring class vs the vanilla type
        // closes that gap (the `.CapsuleComponent`-on-AActor regressions).
        if member_widen_below(slot, ty) {
            continue;
        }
        local_types.insert(*slot, ty.clone());
    }
    // batch-20 Class C: unlike the float out-refs (declaration-only), a BOOL out-ref slot must
    // also be known to the BODY renderer — its `local_N = local_M;` int-copy needs the
    // `(... != 0)` wrap (no implicit int->bool in AS) and its pushes must render bare.
    for (slot, ty) in &outref_overrides {
        if ty == "bool" {
            local_types.insert(*slot, ty.clone());
        } else if !matches!(ty.as_str(), "float" | "float32" | "double") {
            // batch-31c (N3 Fix 2 / N8): enum/struct out-slots reach the body renderer too —
            // the SetV/CpyVtoV enum wraps and the typed pushes need the slot's type
            // (`EInventoryTypes local_7 = local_8;` must wrap the int copy). or_insert:
            // an obj_locals/consumer entry stays authoritative.
            local_types.entry(*slot).or_insert_with(|| ty.clone());
        }
    }
    // batch-28 (spec §2.4.2): the dataflow pass's float slots feed the body renderer FIRST —
    // the pass beats infer_locals' width guesses below, while or_insert keeps the
    // obj_locals/consumer/bool entries above authoritative. I64 entries are declaration-only.
    for (slot, kd) in &numkinds {
        let kw = match kd {
            NumKind::F32 => "float32",
            NumKind::F64 => "float",
            NumKind::I64 => continue,
        };
        local_types.entry(*slot).or_insert_with(|| kw.to_string());
    }
    // batch-20 Class C (SetByCallerMagnitude): float-family slots (from the width-typed ops,
    // e.g. `dTOf w35`) must survive the nested-call stack-split retain (`!is_int` keeps them)
    // and render bare — an untyped `PshV4` push is dropped as a stranded int temporary, which
    // ate the float Magnitude arg pushed before a chained `GetSpec()` call (17 in-game errors).
    // Never overrides an object/consumer-derived type (entry API).
    for (slot, ty) in infer_locals(f, refs) {
        if matches!(ty.as_str(), "float" | "float32" | "double") {
            local_types.entry(slot).or_insert(ty);
        }
    }
    // batch-25g: slots value-pushed into a float-family BY-VALUE parameter are float-typed
    // in the VM (the compiler converts before the push) — same body-typing treatment as the
    // width-op floats above: bare typed pushes that survive the nested-call retain
    // (SaveWorldFloatData's payload was purged as a stranded int under a nested GetWorld()).
    for (slot, ty) in &float_args {
        local_types.entry(*slot).or_insert_with(|| ty.clone());
    }
    // Exact enum-call + same-enum-return evidence, with a separately proven raw-copy chain.
    // Override only primitive guesses; object/struct/cache types remain authoritative.
    for (slot, ty) in &enum_overrides {
        if local_types
            .get(slot)
            .map(|known| is_primitive(known) || known == ty)
            .unwrap_or(true)
        {
            local_types.insert(*slot, ty.clone());
        }
    }
    for slot in &bool_overrides {
        if local_types
            .get(slot)
            .map(|known| is_primitive(known))
            .unwrap_or(true)
        {
            local_types.insert(*slot, "bool".into());
        }
    }
    // A slot the compiler fills from a `Cast<T>` holds a T. The cache can record the coarser
    // handle type it was allocated with (`UObject`), and the structurer then has to write a
    // SECOND `Cast<T>` at every method call on it just to make the call legal — a cast vanilla
    // never had. Where the recorded type is a base the cast is narrower than, take the cast's.
    for (slot, ty) in cast_result_slots(f, refs) {
        let narrows = local_types
            .get(&slot)
            .is_some_and(|wide| wide != &ty && (wide == "UObject" || refs.is_subclass(&ty, wide)));
        if narrows {
            local_types.insert(slot, ty);
        }
    }
    // member-access-derived types (`member_overrides`, computed above as the type lower-bound):
    // the field's declaring class is the strongest signal for a slot used as a member-access
    // base; apply AFTER (overriding) the call-arg guess.
    // batch-30a (C6c, specs/batch29-errortail.md §6c): when a slot's ONLY object write is a
    // STOREOBJ of a single call's result AND that call's return type equals the cache's own
    // obj_locals type, the vanilla static type is authoritative — a member-derived candidate
    // (the field's DECLARING class, e.g. StateTag's UAbilityTask_StateBasedAction) must not
    // widen it. The member access stays legal (the declaring class is an ancestor of the
    // vanilla type in the real hierarchy — the bytecode proves it compiled), and the exact
    // type keeps `return local;` exact and kills the bogus `Cast<DeclaringClass>` render
    // (3× `UAbilityTask_StateBasedAction& -> UAbilityTask_InteractWith`, NativeAICommands).
    let sole_call_store = infer_sole_call_store_types(f, refs);
    let member_widen_blocked = |slot: &i32, ty: &String| {
        sole_call_store
            .get(slot)
            .is_some_and(|ct| vanilla_obj_types.get(slot) == Some(ct) && ct != ty)
    };
    for (slot, ty) in &member_overrides {
        if member_widen_blocked(slot, ty) {
            continue;
        }
        // batch-33a: 31d most-derived merge, mirrored onto the MEMBER pass — a field's
        // DECLARING class that is a PROVABLE ANCESTOR of the cache's own obj_locals type
        // must not widen it. The member access stays legal on the derived vanilla type
        // (the bytecode proves it compiled), while widening loses derived-only methods:
        // `Cast<ABallLightningVisual>` result copied into the vanilla-typed slot, then
        // `local_2.m_CollisionComp` (declared on native AProjectileVisual) re-typed the
        // slot to the base -> `AProjectileVisual::SetSpellLevel(int)` no-match (9 fns:
        // BallLightning/Orc/FireBall_Orc/Heal spells, Explode/StormOfFire visuals,
        // Xardas Initialize, both CreatureTeleport DoTeleport* via ACharacter's
        // CapsuleComponent/Mesh). C6c (sole-call gate above) covered only call-stored
        // slots; this covers the RefCpyV/copy-written rest.
        if let Some(vanilla) = vanilla_obj_types.get(slot) {
            if vanilla != ty && refs.is_subclass(vanilla, ty) {
                continue;
            }
        }
        local_types.insert(*slot, ty.clone());
    }
    // iterator-instance subtypes (illegal-op-round2.md A1): the T1 entry for a `T*Iterator`
    // template INSTANCE carries no SubTypes, so every slot typed from it declares as a bare
    // head (`TArrayConstIterator local_N;`) — a template-arity error that makes the local
    // `Unknown`. Derive `<T>` from the `Iterator()` call's container receiver; applied AFTER
    // the member pass so the composed type wins over the bare member-derived head.
    // Qualify every inferred local type ONCE, before the body renderer sees the map: the body's
    // `Cast<T>` renders and the hoisted declaration have to name the same type, and several
    // inference channels (call owner, construct behaviour) only ever carried the bare name.
    let mut local_types: HashMap<i32, String> = local_types
        .into_iter()
        .map(|(slot, ty)| {
            let qualified = qualify_decl_type(&ty, refs);
            (slot, qualified)
        })
        .collect();
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
    // batch-25g: hand the numeric value-arg slot sets to the body renderer (float-literal
    // SetV* rendering + the int retain keep flag).
    let hints = super::structure::ArgSlotHints {
        // batch-28: the pass's float slots extend the SetV*-literal render set (C3 — the
        // `int64 local = 0; local = 0.33;` asymmetry) and the S4 outref-literal bonus.
        float_slots: float_args
            .keys()
            .copied()
            .chain(
                numkinds
                    .iter()
                    .filter(|(_, k)| !matches!(k, NumKind::I64))
                    .map(|(s, _)| *s),
            )
            .collect(),
        keep_ints,
    };
    let body = body_statements_ctor(
        &fc,
        refs,
        depth + 1,
        super_ctor,
        Some(&f.ret),
        fields,
        Some(&param_types),
        class_name,
        Some(&local_types),
        Some(&hints),
    );
    // batch-30c (C9 accessor-ambiguity, specs/batch29-errortail.md §9): the recovered ctor
    // default-writes `this.WalkSpeed = ...;` collide with the class's inherited SetWalkSpeed
    // property accessor ("Assigned property also has a SetWalkSpeed accessor declared. Write
    // is ambiguous." — a module-killer under warnings-as-errors). Vanilla's ctor bytecode is
    // GENERATED from UPROPERTY defaults (no source statement to warn on), so no property-
    // write spelling can reproduce it warning-free; the accessor-call spelling is the
    // corpus-proven compiling form (`this.SetWalkSpeed(...)` call sites). Exact-keyed to the
    // two captured ctors — a per-site fix, not a mechanism.
    let body = if is_ctor
        && matches!(
            class_name,
            Some("UAIState_Warning" | "UAIState_Warning_Crime")
        )
        && body.contains("this.WalkSpeed = ")
    {
        body.lines()
            .map(|l| match l.split_once("this.WalkSpeed = ") {
                Some((ind2, rhs)) if ind2.trim().is_empty() && rhs.ends_with(';') => {
                    format!("{ind2}this.SetWalkSpeed({});\n", &rhs[..rhs.len() - 1])
                }
                _ => format!("{l}\n"),
            })
            .collect()
    } else {
        body
    };
    // Batch-21 Class C: CONSTSTORE-marked stores carry a const object handle of the local's
    // EXACT type (a same-type Cast<T> does NOT strip const in-game — every batch-20 exact-type
    // Cast site failed "No conversion from 'const X' to 'X'"). Vanilla declared these locals
    // `const`; strip the marker and const-qualify the declaration below.
    let (body, const_slots) = strip_const_store_markers(&body);
    // CASCADE GATE: a const handle can't be COPIED into a non-const local / `__return` /
    // member (handle assignment preserves const), so any slot consumed as a bare copy-RHS
    // keeps its non-const declaration (the store keeps the status-quo error) unless the copy
    // target is itself const-marked. Method calls / call args / `!= nullptr` / Cast<Derived>
    // uses are const-safe: vanilla used the same value through a const local, and Class A
    // restored the faithful const method qualifiers + const param renders.
    let mut const_slots = const_slots;
    // batch-30a (C6a, specs/batch29-errortail.md §6a): propagate const FORWARD through
    // same-type handle copies BEFORE the shrink loop. `local_M = local_N;` with N
    // const-marked previously DROPPED N (copy into a non-const local), keeping the
    // status-quo error at the const-returning call store (`const UComboAttackConfig ->
    // UComboAttackConfig`, GetNextAttackConfig chains). Vanilla-faithfulness argument: the
    // bytecode stores a const handle into N and RefCpyV-copies N into M with no cast, so
    // vanilla's declarations for BOTH slots were const — const-marking M reproduces the
    // vanilla dataflow exactly. Gated to OBJECT handles of the identical recorded type
    // (a const VALUE-type declaration would reject the assignment itself); the shrink loop
    // below still drops the whole chain if any link leaks into `__return`/members.
    if !const_slots.is_empty() {
        let obj_ty = |n: &i32| {
            local_types
                .get(n)
                .filter(|t| t.starts_with('U') || t.starts_with('A'))
        };
        loop {
            let mut grew = false;
            for l in body.lines() {
                let t = l.trim();
                let Some(rest) = t.strip_suffix(';') else {
                    continue;
                };
                let Some((lhs, rhs)) = rest.split_once(" = ") else {
                    continue;
                };
                let (Some(m), Some(n)) = (
                    lhs.strip_prefix("local_")
                        .and_then(|d| d.parse::<i32>().ok()),
                    rhs.strip_prefix("local_")
                        .and_then(|d| d.parse::<i32>().ok()),
                ) else {
                    continue;
                };
                if const_slots.contains(&n)
                    && !const_slots.contains(&m)
                    && obj_ty(&m).is_some()
                    && obj_ty(&m) == obj_ty(&n)
                {
                    const_slots.insert(m);
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
    }
    // batch-41d: when the function's own return type is a CONST object handle (vanilla marked
    // `const T GetX()`), a `return local_n;` of a const-marked slot is const->const = legal, so
    // the return use must NOT strip the slot's const (the strip would relocate the error to the
    // const-member-read assignment: "const T -> T", GetSelectedItem). The signature re-adds the
    // `const` too (see `ret_sig` below). Gated to object-const returns only — a non-const-return
    // getter still strips (the pre-batch-41d behaviour). Mirrors DataType::render's const
    // condition (is_read_only || is_object_const) so `ret` and the kept slot stay consistent.
    let ret_obj_const = f.ret.is_object_handle && (f.ret.is_object_const || f.ret.is_read_only);
    loop {
        let keep: HashSet<i32> = const_slots
            .iter()
            .copied()
            .filter(|n| {
                let needle = format!("= local_{n};");
                !body.lines().any(|l| {
                    let t = l.trim();
                    if !t.ends_with(needle.as_str()) {
                        return false;
                    }
                    let lhs = t[..t.len() - needle.len()].trim_end();
                    let lhs_const = lhs
                        .strip_prefix("local_")
                        .and_then(|d| d.parse::<i32>().ok())
                        .map(|m| const_slots.contains(&m))
                        .unwrap_or(false);
                    !lhs_const
                }) && (ret_obj_const || !body.contains(&format!("return local_{n};")))
            })
            .collect();
        // fixed point: dropping a slot can invalidate a copy INTO it that justified keeping
        // its source, so iterate until stable (bounded: the set only shrinks).
        if keep.len() == const_slots.len() {
            break;
        }
        const_slots = keep;
    }
    let const_slots = const_slots;
    // The structurer names VM value slots uniformly, including expression temporaries that the
    // source compiler never declared. In a proven enum state-machine slice, remove a primitive /
    // enum scratch only when EVERY source reference can be folded through an immediately-adjacent
    // producer -> single consumer pair. The whole-slot gate is important: a partial rewrite could
    // move a value across a branch or leave a declaration whose eager default initializer changes
    // the regenerated bytecode. Object/value-struct locals are never candidates.
    let inferred_locals = infer_locals(f, refs);
    // The declarations further down apply the bool proofs to these types; a pass that asks what
    // a slot holds has to see the same answer the declaration will print.
    let proven_locals = {
        let mut typed = inferred_locals.clone();
        for slot in &bool_overrides {
            typed.insert(*slot, "bool".to_string());
        }
        typed
    };
    // The slot types the DECLARATIONS will carry, as far as they are decided here: the inferred
    // table with the numeric-kind, enum and bool overrides the hoist below applies in that order.
    // A fold that compares a slot against a type has to compare against the type the slot will be
    // declared with — reading the table inference started from refuses, among others, every
    // `bool` the cache proved from a comparison or a call's return.
    let declared_locals = {
        let mut view = inferred_locals.clone();
        for (slot, kd) in &numkinds {
            let kw = match kd {
                NumKind::F32 => "float32",
                NumKind::F64 => "float",
                NumKind::I64 => "int64",
            };
            if matches!(
                view.get(slot).map(String::as_str),
                None | Some("int" | "int64" | "float" | "double")
            ) {
                view.insert(*slot, kw.to_string());
            }
        }
        for (slot, ty) in &enum_overrides {
            if view
                .get(slot)
                .map(|known| is_primitive(known) || known == ty)
                .unwrap_or(true)
            {
                view.insert(*slot, ty.clone());
            }
        }
        for slot in &bool_overrides {
            if view.get(slot).map(|known| is_primitive(known)).unwrap_or(true) {
                view.insert(*slot, "bool".into());
            }
        }
        view
    };
    let const_result_slots = infer_const_result_slots(f, refs);
    let body = if enum_overrides.is_empty() {
        body
    } else {
        let candidates: HashSet<i32> = used_locals(&body)
            .into_iter()
            .filter(|slot| {
                let typed_state =
                    enum_overrides.contains_key(slot) || bool_overrides.contains(slot);
                let primitive_scratch = inferred_locals
                    .get(slot)
                    .map(|ty| is_primitive(ty))
                    .unwrap_or_else(|| !f.obj_locals.iter().any(|(obj_slot, _)| obj_slot == slot));
                // Preserve a compiler-reused primitive carrier with multiple definition/use
                // segments. Folding each adjacent segment is source-semantically valid, but it
                // can split one VM live range into several compiler temporaries and defeat the
                // byte-faithfulness oracle. Proven enum/bool state slots remain eligible because
                // their concrete type is the purpose of this pass; ordinary primitive scratch is
                // eliminated only when it has a single producer assignment in the whole body.
                adjacent_value_slot_is_candidate(&body, *slot, typed_state, primitive_scratch)
            })
            .collect();
        rewrite_adjacent_value_temporaries(&body, &candidates).0
    };
    let (body, _) = rewrite_value_temporaries(&body, &inferred_locals);
    let body = drop_dead_stores(&body);
    let body = fold_literal_temporaries(&body, refs);
    let body = fold_constant_comparisons(&body);
    let body = fold_double_negations(&body);
    let body = fold_negated_stores(&body);
    // `__InitDefaults` is where the class's values live, and their recovery is fail-closed: a
    // temporary left without a reader there costs the whole class its `default` statements
    // (measured: 358 classes). Move only what a call reads there, never a plain operand.
    let body = inline_call_argument_temporaries(
        &body,
        refs,
        &declared_locals,
        fields,
        !f.name.contains("__InitDefaults"),
        &call_result_types,
        &statement_producers,
        &temporary_receivers,
        &loop_elements,
        &widened,
        &aliased,
    );
    // Runs after the producers have moved into their readers: only then is the value arm of a
    // short circuit the single assignment it was in source.
    let body = fold_short_circuits(&body, &proven_locals, refs, fields, &HashMap::new());
    let body = join_short_circuit_chains(&body);
    // Again, now that the chain IS one expression: the negation fold ran before the short
    // circuits were recovered, so `X = A && B; X = !X;` was still two branches then. Left as two
    // statements the negation costs a copy out, a `NOT` and a copy back where vanilla applied
    // `NOT` in place — and the named result stops the compiler folding the chain's own left test
    // into its branch.
    let body = fold_negated_stores(&body);
    // Again, now that a short circuit IS an expression. The sweep above ran before the folds,
    // so a value the source wrote inside a call's argument as `A && B` was still an `if`/`else`
    // over a named local when the producers moved, and nothing looked at it afterwards. Writing
    // the operand into a name is not free: the compiler then materializes the left side of the
    // `&&` into a variable before branching, where vanilla branched on the comparison itself.
    let body = inline_call_argument_temporaries(
        &body,
        refs,
        &declared_locals,
        fields,
        !f.name.contains("__InitDefaults"),
        &call_result_types,
        &statement_producers,
        &temporary_receivers,
        &loop_elements,
        &widened,
        &aliased,
    );
    let body = rewrite_operator_calls(&body);
    let body = fold_cast_diamonds(&body);
    // A third time, now that a cast IS an expression. Until the diamond folded, the cast stood as
    // a branch over a named slot and the producer feeding it stood on a line of its own — a line
    // between a temporary and its reader that does not feed that reader, which is what the sweep
    // refuses to move across. Fold the diamond and both go away; nothing had looked again.
    // Vanilla proves the temporary had no name: it calls straight on the cast's own slot, where a
    // named handle would have been aliased into its own with `RefCpyV` first.
    let body = inline_call_argument_temporaries(
        &body,
        refs,
        &declared_locals,
        fields,
        !f.name.contains("__InitDefaults"),
        &call_result_types,
        &statement_producers,
        &temporary_receivers,
        &loop_elements,
        &widened,
        &aliased,
    );
    let body = drop_unreachable_statements(&body);
    // All three run before the declarations are hoisted, so a temporary they empty out never
    // gets one.
    // A function that returns by REFERENCE keeps its named local: the name is what makes the
    // returned thing outlive the expression (same condition as `ref_ret` below).
    let returns_by_reference = f.ret.is_reference && f.ret.token == 5 && !f.ret.is_object_handle;
    let body = fold_condition_temporaries(&body, &declared_locals, refs, fields);
    let body = fold_alias_copies(&body, &declared_locals);
    let body = fold_copy_out_temporaries(&body, &declared_locals, &const_result_slots, fields);
    let body = fold_cast_operands(&body, &declared_locals, &call_result_types);
    // What a member path can start from: a local the declarations will name, or one of the
    // function's own parameters. Both carry their type in a table; neither needs an inference.
    let path_roots: HashMap<String, String> = declared_locals
        .iter()
        .map(|(slot, ty)| (format!("local_{slot}"), ty.clone()))
        .chain(
            f.params
                .iter()
                .filter(|p| !p.name.is_empty())
                .map(|p| (p.name.clone(), p.ty.base_name(refs))),
        )
        .collect();
    let body = fold_enum_round_trips(&body, fields, &path_roots, refs);
    let body = fold_member_read_temporaries(
        &body,
        &widened,
        &declared_locals,
        fields,
        &path_roots,
        refs,
        &member_read_slots(f),
    );
    let body =
        fold_returned_temporaries(&body, &declared_locals, refs, &ret, returns_by_reference);
    // hoist every referenced local; infer_locals types what it can, the rest default to `int`
    // (a wrong type just becomes a compile error the in-game loop force-stubs, rather than the
    // whole function stubbing on an undeclared identifier).
    let used = used_locals(&body);
    let mut locals = inferred_locals;
    // batch-28: unified numeric-kind dataflow (C1/C2/C3/C4c/C4d) — overrides ONLY the
    // width-guessed primitive keywords; farg/outref/member/callret overrides still apply
    // after and keep their precedence. `Some("float")` IS overridable: that is exactly the
    // dTOf/f-op dst mis-keyword (a 4-byte slot declared with the 8-byte keyword) -> float32.
    for (slot, kd) in &numkinds {
        let kw = match kd {
            NumKind::F32 => "float32",
            NumKind::F64 => "float",
            NumKind::I64 => "int64",
        };
        match locals.get(slot).map(String::as_str) {
            None | Some("int" | "int64" | "float" | "double") => {
                locals.insert(*slot, kw.to_string());
            }
            _ => {} // opCast/object/ret-retype float32 entries win
        }
    }
    for (slot, ty) in &enum_overrides {
        if locals
            .get(slot)
            .map(|known| is_primitive(known) || known == ty)
            .unwrap_or(true)
        {
            locals.insert(*slot, ty.clone());
        }
    }
    for slot in &bool_overrides {
        if locals
            .get(slot)
            .map(|known| is_primitive(known))
            .unwrap_or(true)
        {
            locals.insert(*slot, "bool".into());
        }
    }
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
            // batch-31d (N7): declaration-side mirror of the most-derived merge above — the
            // vanilla obj_locals type wins over a param-derived ANCESTOR candidate.
            if let Some(vanilla) = vanilla_obj_types.get(slot) {
                if vanilla != ty && refs.is_subclass(vanilla, ty) {
                    continue;
                }
            }
            // batch-34: declaration-side mirror of the member-access lower-bound guard — a
            // call-arg candidate must not widen the declaration below a member's declaring
            // class (keeps `AGothicCharacter local_N;` where the body reads `.CapsuleComponent`).
            if member_widen_below(slot, ty) {
                continue;
            }
            locals.insert(*slot, ty.clone());
        }
    }
    // member-derived declaring-class types override the cache's wrong/general slot type
    for (slot, ty) in &member_overrides {
        // batch-30a (C6c): same gate as the body map — a sole-call-written slot whose call
        // return type matches the vanilla obj_locals entry keeps the exact vanilla type.
        if used.contains(slot) && !member_widen_blocked(slot, ty) {
            // batch-33a: declaration-side mirror of the most-derived member merge above —
            // the vanilla obj_locals type wins over a field-declaring-class ANCESTOR.
            if let Some(vanilla) = vanilla_obj_types.get(slot) {
                if vanilla != ty && refs.is_subclass(vanilla, ty) {
                    continue;
                }
            }
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
    // batch-25g: float value-arg slots declare with the callee's float keyword (their SetV*
    // constants now render as float literals — `float32 local_1 = 0.0; ... local_1 = 1.0f;`).
    // Primitive-only upgrade, before the authoritative out-ref pass below.
    for (slot, ty) in &float_args {
        if used.contains(slot) && locals.get(slot).map(|t| is_primitive(t)).unwrap_or(true) {
            locals.insert(*slot, ty.clone());
        }
    }
    // batch-28b (spec §5.1): small-int by-value arg slots (uint8/int8/uint16/int16 params —
    // FColor ctor args, FindLastChar/FindChar char params, SetMovementMode's NewCustomMode).
    // Fully gated in infer_slot_types (SetV-only op profile + constant range fit);
    // declaration-only, primitive-upgrade-only — the C4a/C4b truncate/signedness classes.
    for (slot, ty) in &small_args {
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
    let mut oor_args: Vec<i32> = used_idents(&body, "arg")
        .into_iter()
        .filter(|&n| n as usize >= f.params.len())
        .collect();
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
    let percep_stub_param = (ref_ret && f.ret.base_name(refs) == "FPerceptionHandler")
        .then(|| {
            f.params.iter().enumerate().find_map(|(i, p)| {
                (p.ty.base_name(refs) == "UCharacterPerceptionComponent").then(|| {
                    if p.name.is_empty() {
                        format!("arg{i}")
                    } else {
                        p.name.clone()
                    }
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
    } else if ret_obj_const
        && reason.is_none()
        && const_slots
            .iter()
            .any(|n| body.contains(&format!("return local_{n};")))
    {
        // batch-41d: a getter that returns a const-marked slot (`return this.<constMember>;`)
        // needs its return type const too, so `return local_n;` is const->const. Vanilla marked
        // this return `const T` (f.ret.is_object_const); re-add the qualifier the generic strip
        // at line ~349 removed. Only when a const slot is actually returned (the stub/RVODEF
        // paths use the non-const `ret` for `T __r;`, which stays correct).
        format!("const {ret}")
    } else {
        ret.clone()
    };
    if f.is_ufunction {
        let _ = writeln!(s, "{ind}UFUNCTION()");
    }
    if is_ctor {
        let _ = writeln!(s, "{ind}{}({params})", f.name); // constructors have no return type
    } else {
        // Vanilla `const` methods (asTRAIT_CONST in FunctionTraits) need their qualifier back
        // so const-handle callers compile ("Non-const method call on read-only object
        // reference") — but a blanket re-emit makes `this` read-only inside every RECOVERED
        // body, which is not const-faithful (batch-21 HARNESS-REGRESSED +636, read-only errors
        // 150 -> 2763). Batch-23b: emit the qualifier ONLY for the per-method body-const-
        // safety-verified set (specs/const-safety.md); everything else stays non-const and
        // keeps its caller-side residue.
        let constq = if is_method
            && f.is_const_method()
            && !body_calls_non_const_method(&body, class_name, refs)
        {
            " const"
        } else {
            ""
        };
        let _ = writeln!(s, "{ind}{ret_sig} {}({params}){constq}", f.name);
    }
    let _ = writeln!(s, "{ind}{{");

    if reason.is_none() {
        // Class A (emission-classes.md): FStatID/FScopeCycleCounter have neither a default
        // constructor nor an opAssign, so BOTH the bare hoisted declaration and the later
        // whole-object ctor-assign fail. When every reference to such a local is the
        // write-only `local_N = TY(...);` shape, suppress the hoist and rewrite each
        // assignment to a declaration-with-initializer (in-place construction — the original
        // source form). Any other reference shape keeps the hoist (status quo).
        // Last, after every other override: the declaration has to name what the structurer's
        // view names, or the body calls a method the declared type does not have. Same narrowing
        // rule and same witness as the view — a base the cast is narrower than, or the `?` the
        // cache records for a cast out-slot.
        for (slot, ty) in cast_result_slots(f, refs) {
            let narrows = locals.get(&slot).is_some_and(|wide| {
                wide != &ty && (wide == "UObject" || wide == "?" || refs.is_subclass(&ty, wide))
            });
            if narrows {
                locals.insert(slot, ty);
            }
        }
        let (body, suppressed) = rewrite_ctor_only_locals(&body, &locals);
        // FAbilityTaskExecutor's opAssign takes a NON-const reference, so assigning a by-value
        // call result to a declared local ('local = DrawMeleeWeapon(AI);') fails "Cannot pass a
        // temporary value into non-const reference parameter" (2841 in-game errors). The only
        // legal form is declaration-with-initializer (copy-construction). Rewrite qualifying
        // executor locals to decl-init at their assignment sites.
        let (body, na_suppressed) = rewrite_no_assign_locals(&body, &locals);
        let (body, discarded) = drop_unread_call_results(&body, &locals, refs);
        // Batch-20 Class A residue: executor locals whose reference shape failed the decl-init
        // gates above (multi-assign with reads, read-before-assign, cross-block reads) still
        // carry `local_N = <call>;` assignments — temporary into non-const opAssign. Split each
        // into `TY __na_tK = <call>; local_N = __na_tK;` — the lvalue assign compiles (proven
        // in-game: `__return = local_16;` never errored in the batch-19 capture) and the temp
        // lives/dies on adjacent lines of the same block, so it is scope-safe by construction.
        // Before the split below: `__return = <call>; return __return;` is a RETURN, and a
        // return copy-constructs — it never goes through the non-const `opAssign` the split
        // exists to avoid. Folded first, there is no assignment left for the split to name.
        let body = fold_return_slot_stores(&body);
        let body = fold_return_slot_arms(&body);
        // Only where vanilla let the arm FALL THROUGH into the rest of the function. Where the
        // last thing before the epilogue is a jump INTO it, both arms jumped to a common join —
        // which is what an `else` behind a returning arm compiles to, and dropping it emits one
        // jump fewer than vanilla had.
        let body = if epilogue_is_joined(f) {
            body
        } else {
            drop_else_after_returning_arm(&body)
        };
        let body = rewrite_no_assign_residual_assigns(&body, &locals, &ret);
        // Iterator locals have no default ctor either; declare them at their `Iterator()` call.
        let (body, iter_suppressed) = rewrite_iterator_decl_init(&body, &locals);
        // Same for value-type temporaries: declaring one bare and assigning afterwards asks for
        // a default-construct behaviour AND an `opAssign`, and the base cache has neither.
        // The iterator idiom is a range-for the compiler desugared; write it back as one. Runs
        // BEFORE the value-type decl-init rewrite, which would otherwise give the loop element a
        // declaration and hide the idiom.
        let (body, foreach_suppressed) = rewrite_foreach_loops(&body, &locals, refs);
        // Before the declaration merge: a conversion naming the type the value already has hides
        // the copy-construction the merge is looking for.
        let body = drop_redundant_conversions(&body, fields, &path_roots, refs);
        let (body, value_suppressed) =
            rewrite_value_decl_init(&body, &locals, refs, &copy_constructed_slots(f, refs));
        // A local that receives a CONST call result has to be const as well, and a const local is
        // declared where it gets its value.
        let (body, const_suppressed) =
            rewrite_const_decl_init(&body, &locals, &const_result_slots, refs);
        // Everything left: a local whose first reference is the assignment that gives it its
        // value was DECLARED there in the source. Hoisting the declaration and assigning
        // afterwards makes the compiler spend its own temporary for the value and copy it into
        // the declared slot — a copy vanilla never emitted, and the single largest remaining
        // class of divergence.
        let already_declared_at_use: HashSet<i32> = [
            &suppressed,
            &na_suppressed,
            &discarded,
            &iter_suppressed,
            &value_suppressed,
            &foreach_suppressed,
            &const_suppressed,
            &const_slots,
        ]
        .into_iter()
        .flatten()
        .copied()
        .collect();
        let (body, first_use_suppressed) =
            rewrite_first_use_decl_init(&body, &locals, refs, &already_declared_at_use);
        let (body, first_write_suppressed) = rewrite_bare_decl_at_first_write(
            &body,
            &locals,
            refs,
            &already_declared_at_use,
            &first_use_suppressed,
        );
        // Where the declarations start, so the whole block plus the body it heads can be handed
        // to the sink below: a declaration only moves once it stands next to the code that uses
        // it, and the two are written from different places.
        let declarations_at = s.len();
        // Hoist local declarations. A primitive may stay bare only when its first source-level
        // reference is a top-level write-only assignment; this is the same definite-assignment
        // proof used for inferred enums. Everything else gets an explicit default initializer so
        // the game's warnings-as-errors policy cannot reject a branch-only first write.
        for (slot, ty) in &locals {
            let qualified = qualify_decl_type(ty, refs);
            let ty = &qualified;
            if suppressed.contains(slot)
                || na_suppressed.contains(slot)
                || discarded.contains(slot)
                || iter_suppressed.contains(slot)
                || value_suppressed.contains(slot)
                || foreach_suppressed.contains(slot)
                || const_suppressed.contains(slot)
                || first_use_suppressed.contains(slot)
                || first_write_suppressed.contains(slot)
            {
                continue; // declared at its (rewritten) assignment/Iterator() site instead
            }
            if enum_overrides
                .get(slot)
                .is_some_and(|enum_ty| enum_ty == ty)
                && !first_top_level_assignment_before_read(&body, *slot)
            {
                // A hoisted enum local is not definitely initialized merely because the enum
                // itself is a value type. Keep the vanilla-like bare declaration only when its
                // first source-level reference is a whole-function-scope, write-only assignment;
                // otherwise initialize it to the underlying zero value so branch-only writes do
                // not trip warnings-as-errors in the game compiler.
                let _ = writeln!(s, "{ind}    {ty} local_{slot} = {ty}(0);");
            } else if is_primitive(ty)
                // A local vanilla never initialized shows up as a `SetV4 v, 0` prologue the
                // original does not have. The eager initializer is only needed where the value
                // could be read before it is written: either the first reference is the write
                // itself, or every read is dominated by one. The second proof used to be
                // restricted to proven-enum functions with several assignments; the proof does
                // not depend on either.
                && (first_top_level_assignment_before_read(&body, *slot)
                    || all_reads_lexically_dominated_by_assignment(&body, *slot))
            {
                let _ = writeln!(s, "{ind}    {ty} local_{slot};");
            } else if is_primitive(ty) {
                let _ = writeln!(s, "{ind}    {ty} local_{slot} = {};", default_for(ty));
            } else if const_slots.contains(slot) {
                // batch-21 Class C: at least one store is a const handle of this exact type;
                // `const` on the declaration matches the vanilla form (non-const stores into a
                // const handle remain legal, so mixed-source slots are safe).
                let _ = writeln!(s, "{ind}    const {ty} local_{slot};");
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
        // Last of the body passes: the destination of a compound assignment is a member of
        // something the source NAMED, and the naming is handed out above. Earlier the receiver
        // still reads `this.GetG1R()`, and a call in the path is exactly what must not fold —
        // evaluating it twice is not the same as evaluating it once.
        // Before the compound-assignment fold, which would rewrite the middle line out of the
        // shape this one matches on.
        let body = collapse_single_use_accumulators(&body, &widened);
        let body = fold_enum_call_round_trips(&body, &call_result_types, fields, &path_roots, refs, returns_by_reference, has_enum_conversions(f));
        let body = fold_compound_assignments(&body, fields, &path_roots, refs);
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
        // Declarations and body are one text now. A struct declared at function scope costs a
        // constructor at entry and a destructor on every path out; where vanilla touched the slot
        // only after it had branched, the declaration stood in the block that touches it.
        let rendered = sink_declarations_into_their_block(&s[declarations_at..], &touched_after_branch);
        // Same text, same reason as the sink: the declaration lives in `s`, its uses in `body`.
        let rendered = spell_out_repeated_temporaries(&rendered, &constructions);
        // And once more here, for the same reason: an accumulator whose declaration was hoisted
        // has its `T X;` in this text and its `X = ...` lines in the body, so the split form is
        // only whole once the two are joined.
        let rendered = collapse_single_use_accumulators(&rendered, &widened);
        let rendered = fold_enum_call_round_trips(&rendered, &call_result_types, fields, &path_roots, refs, returns_by_reference, has_enum_conversions(f));
        let rendered = fold_compound_assignments(&rendered, fields, &path_roots, refs);
        // Again on the joined text: a short circuit whose CONDITION is itself a short circuit is
        // only one condition once the inner one has folded, and the pass that folds it ran before
        // that happened. The outer arm then still stands as an if/else over a bool carrier.
        let rendered = fold_short_circuits(&rendered, &proven_locals, refs, fields, &path_roots);
        let rendered = join_short_circuit_chains(&rendered);
        let rendered =
            fold_returned_temporaries(&rendered, &declared_locals, refs, &ret, returns_by_reference);
        let rendered = fold_negated_stores(&rendered);
        let rendered = fold_assigned_temporaries(&rendered, fields, &path_roots, refs);
        // Before the folds move anything: a struct handed on by address is only recognisable
        // while its declaration and the call that takes it stand in the same text.
        let rendered = restore_dropped_struct_arguments(&rendered, &address_push_counts(f));
        // The rest of the fold chain, for the same reason as the passes above it: each of these
        // asks about a declaration, and until the hoist has been joined back on there is nothing
        // for them to ask about.
        let rendered = fold_condition_temporaries(&rendered, &declared_locals, refs, fields);
        let rendered = fold_alias_copies(&rendered, &declared_locals);
        let rendered =
            fold_copy_out_temporaries(&rendered, &declared_locals, &const_result_slots, fields);
        let rendered = fold_cast_operands(&rendered, &declared_locals, &call_result_types);
        let rendered = fold_enum_round_trips(&rendered, fields, &path_roots, refs);
        let rendered = inline_single_use_literals(&rendered);
        let rendered = collapse_single_use_accumulators(&rendered, &widened);
        let rendered = inline_bool_chain_into_next_condition(&rendered);
        let rendered = fold_bool_member_comparisons(&rendered, fields, &path_roots, refs);
        let rendered = drop_redundant_conversions(&rendered, fields, &path_roots, refs);
        let rendered =
            spell_out_argument_temporaries(&rendered, &argument_constructed_slots(f, refs), refs);
        let rendered = fold_widening_aliases(&rendered, &declared_locals, &widened);
        let rendered = drop_block_end_handle_releases(&rendered);
        let rendered = drop_unused_declarations(&rendered);
        let rendered = fold_member_read_temporaries(
            &rendered,
            &widened,
            &declared_locals,
            fields,
            &path_roots,
            refs,
            &member_read_slots(f),
        );
        s.truncate(declarations_at);
        s.push_str(&rendered);
    } else {
        // stub fallback so the module still compiles (reason recorded for aggregation)
        let _ = writeln!(
            s,
            "{ind}    // body not fully recovered — stub [{}]",
            reason.unwrap()
        );
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
            let nm = if p.name.is_empty() {
                format!("arg{i}")
            } else {
                p.name.clone()
            };
            // A parameter's DEFAULT is part of the declaration, not of the function's identity
            // in the cache tables. Rendering it back is what makes a call that omits the
            // argument legal — and omitting it is what the call site does, because spelling the
            // default out compiles into construct behaviours vanilla never had. The cache
            // stores the text tokenized (`FGameplayTagContainer ( )`), so pack it.
            let default = f
                .param_defaults
                .get(i)
                .filter(|value| !value.is_empty())
                .map(|value| format!(" = {}", super::refs::pack_tokens(value)))
                .unwrap_or_default();
            format!("{ty} {amp}{nm}{default}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Strip [`CONSTSTORE`] markers from a rendered body, returning the cleaned body plus the
/// set of `local_N` slots whose store RHS was a const handle of the local's exact type —
/// those declarations get a `const` qualifier (batch-21 Class C). A marked store whose LHS
/// is not a plain `local_N` (e.g. `__return`) just loses the marker (status-quo behavior).
fn strip_const_store_markers(body: &str) -> (String, HashSet<i32>) {
    let mut slots = HashSet::new();
    if !body.contains(CONSTSTORE) {
        return (body.to_string(), slots);
    }
    let mut out = String::with_capacity(body.len());
    for line in body.lines() {
        if let Some(pos) = line.find(CONSTSTORE) {
            let lhs = line[..pos].trim_start();
            if let Some(n) = lhs
                .strip_prefix("local_")
                .and_then(|r| r.strip_suffix(" = "))
                .and_then(|d| d.parse::<i32>().ok())
            {
                slots.insert(n);
            }
            out.extend(line.chars().filter(|c| *c != CONSTSTORE));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    (out, slots)
}

/// A body is "recoverable" only if it has NO recovery gaps: the decompiler emits a
/// `// <mnemonic> ...` comment for every op it can't lower, an unresolved-operand
/// placeholder `?` (e.g. `if (? != ?)`) when a comparison/operand couldn't be recovered,
/// and may reference a `local_N` that wasn't inferred. Any of these is a syntax/semantic
/// error that aborts the module's parse, so such a function falls back to a clean stub.
/// Returns `None` if the body is recoverable, else `Some(reason)` for the first gap found —
/// the reason string is emitted in the stub comment so the stub causes can be aggregated.
fn stub_reason(
    body: &str,
    locals: &BTreeMap<i32, String>,
    param_count: usize,
    ret_is_void: bool,
) -> Option<String> {
    // an ARGMISMATCH sentinel (\u{2}<code>) — extract its cause code for aggregation.
    if let Some(i) = body.find('\u{2}') {
        let code: String = body[i + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        return Some(format!(
            "argmismatch:{}",
            if code.is_empty() { "?" } else { &code }
        ));
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
        if &b[i..i + pl] == prefix.as_bytes() && (i == 0 || !is_ident(b[i - 1])) {
            let mut j = i + pl;
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > start && (j >= b.len() || !is_ident(b[j])) {
                if let Ok(n) = body[start..j].parse::<i32>() {
                    out.insert(n);
                }
                i = j;
                continue;
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
    matches!(
        ty.split('<').next().unwrap_or(ty).bytes().next(),
        Some(b'F') | Some(b'T')
    )
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
fn infer_slot_types(
    f: &Func,
    refs: &RefResolver,
) -> (
    HashMap<i32, String>,
    HashMap<i32, String>,
    HashMap<i32, String>,
    HashSet<i32>,
    HashSet<i32>,
    HashMap<i32, String>,
) {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => {
            return (
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashSet::new(),
                HashSet::new(),
                HashMap::new(),
            );
        }
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32);
    let writes = |op: &str| {
        op.starts_with("SetV")
            || op.starts_with("CpyVtoV")
            || op.starts_with("CpyRtoV")
            || op.starts_with("RDR")
            || op.contains("TO")
            || op == "STOREOBJ"
            || op == "PopRPtr"
            || op.starts_with("ADD")
            || op.starts_with("SUB")
            || op.starts_with("MUL")
            || op.starts_with("DIV")
            || op.starts_with("MOD")
            || op.starts_with("NEG")
            || op.starts_with("Inc")
            || op.starts_with("Dec")
            || op == "NOT"
            || op.starts_with("B")
            || op == "ALLOC"
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
                                                                   // batch-25g: numeric BY-VALUE argument slots (see structure::ArgSlotHints). A slot
                                                                   // VALUE-pushed (never PSF) into a float-family param is float-typed in the VM — the
                                                                   // compiler inserts the int->float conversion BEFORE the push, so any direct PshV4 into a
                                                                   // float param proves the slot holds float bits. Int-family pairs only earn the retain
                                                                   // keep flag (no retype). Conflicts (differing float kws, or float+int for one slot) drop.
    let mut farg: HashMap<i32, Option<String>> = HashMap::new();
    let mut ikeep: HashSet<i32> = HashSet::new();
    // batch-28: PSF slots bound to an INT-family reference param — int lvalues; anti-seeds
    // for `infer_float_flow` (floating such a decl breaks the `int&` binding; the exact
    // batch-9-class risk named in specs/batch27-floatwarnings.md §2.6).
    let mut iref: HashSet<i32> = HashSet::new();
    // batch-28b (spec §5.1): small-int BY-VALUE arg slots (token 0x45 int8 / 0x46 int16 /
    // 0x4C uint8 / 0x4D uint16) — declaration retype candidates for the C4a/C4b truncate/
    // signedness classes. Conflicts (differing small kws, or ANY full-width int pairing)
    // resolve to None; the SetV-only op-profile + range gates apply below.
    let mut sarg: HashMap<i32, Option<String>> = HashMap::new();
    // shared numeric BY-VALUE channel (used by both ordinary-call pairing and the $beh0
    // ctor-arg pairing): float params -> farg, int-family params -> ikeep (+ sarg small-ints).
    let val_arg = |s: i32,
                   pt: &super::types::DataType,
                   farg: &mut HashMap<i32, Option<String>>,
                   ikeep: &mut HashSet<i32>,
                   sarg: &mut HashMap<i32, Option<String>>| {
        match pt.token {
            0x50 | 0x51 | 0x5E => {
                let kw = super::types::token_keyword(pt.token).to_string();
                match farg.get(&s) {
                    None => {
                        farg.insert(s, Some(kw));
                    }
                    Some(Some(prev)) if *prev != kw => {
                        farg.insert(s, None);
                    }
                    _ => {}
                }
            }
            0x45 | 0x46 | 0x4C | 0x4D => {
                ikeep.insert(s);
                let kw = super::types::token_keyword(pt.token).to_string();
                match sarg.get(&s) {
                    None => {
                        sarg.insert(s, Some(kw));
                    }
                    Some(Some(prev)) if *prev != kw => {
                        sarg.insert(s, None);
                    }
                    _ => {}
                }
            }
            0x44 | 0x47 | 0x4B | 0x4E => {
                ikeep.insert(s);
                // a full-width int pairing disqualifies the small-int retype (the widened
                // push may carry values outside the small range).
                sarg.insert(s, None);
            }
            // batch-33b (N1 residue): a slot VALUE-pushed into a BOOL by-value param is a
            // real argument exactly like the int-family/enum cases above — bool slots are
            // untyped ints in the model, so the push died in the nested-call split
            // (UCM_RollToSide lost Math::RandBool()'s bFlipSide to five intervening
            // GetNavAgentLocation/opSub/GetUnsafeNormal calls -> RollToEvade 2-arg
            // no-match). Keep-only: the render-side `(local_N != 0)` wrap is the existing
            // bool-param cast_arg path, which fires once the arg survives.
            0x41 => {
                ikeep.insert(s);
            }
            // batch-31c (N1, spec batch31-nomatch-illegalop §1.4): a slot VALUE-pushed into a
            // known ENUM by-value param is a REAL argument, not a stranded SetV temporary —
            // it earns the retain keep flag exactly like the int-family pairs (enum slots are
            // not in the typed-locals map, so their pushes are Arg::int and died in the
            // nested-call split: TargetCharacterOfInterest lost its FocusPriority to an
            // intervening GetCharacterOfInterest()). Keep-only — the render-side wrap is the
            // EXISTING cast_arg enum wrap, which fires once the arg survives.
            5 if super::structure::is_enum_name(&pt.base_name(refs)) => {
                ikeep.insert(s);
            }
            _ => {}
        }
    };
    let pair = |ostack: &mut Vec<Option<(i32, bool)>>,
                params: Option<&[super::types::DataType]>,
                is_method: bool,
                ret_struct: bool,
                cand: &mut HashMap<i32, Option<String>>,
                outref: &mut HashMap<i32, Option<String>>,
                farg: &mut HashMap<i32, Option<String>>,
                ikeep: &mut HashSet<i32>,
                iref: &mut HashSet<i32>,
                sarg: &mut HashMap<i32, Option<String>>| {
        let Some(params) = params else {
            ostack.clear();
            return;
        };
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
        let total = if is_method {
            params.len() + 1 + rvo
        } else {
            params.len() + rvo
        };
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
                    // batch-31c (N1): PARAM slots (negative frame offsets) join the model, but
                    // ONLY for the keep channel — their declarations are fixed by the
                    // signature (never retyped; farg/sarg/cand/outref stay locals-only), and
                    // enum-typed params pushed by value are exactly the Proof-A/B purge
                    // victims (FocusPriority w65534). Float-family params are excluded: the
                    // signature already types them (typed pushes survive the retain), and
                    // routing them into ikeep would anti-seed a float slot. PSF pushes of the
                    // same slot are naturally NEUTRAL here (the gate is value-push-only).
                    if *s < 0 {
                        if !*is_psf && !pt.is_reference {
                            match pt.token {
                                // 0x41: batch-33b — bool params join the keep set (see val_arg).
                                0x41 | 0x44 | 0x45 | 0x46 | 0x47 | 0x4B | 0x4C | 0x4D | 0x4E => {
                                    ikeep.insert(*s);
                                }
                                5 if super::structure::is_enum_name(&pt.base_name(refs)) => {
                                    ikeep.insert(*s);
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }
                    let ty = pt.base_name(refs);
                    match cand.get(s) {
                        None => {
                            cand.insert(*s, Some(ty));
                        }
                        Some(Some(prev)) if *prev != ty => {
                            cand.insert(*s, None);
                        }
                        _ => {}
                    }
                    // out-ref float pass: a PSF'd (address-pushed) slot feeding a float-family
                    // REFERENCE param must be declared with exactly that type (see fn doc).
                    // batch-20 Class C: bool& out params too (GetFloatAttributeFromAbility-
                    // SystemComponent's `bool& bSuccessfullyFoundAttribute` — the slot was
                    // declared int, and an int lvalue can't bind to bool&; 34 in-game errors).
                    if *is_psf && pt.is_reference && matches!(pt.token, 0x41 | 0x50 | 0x51 | 0x5E) {
                        let kw = super::types::token_keyword(pt.token).to_string();
                        match outref.get(s) {
                            None => {
                                outref.insert(*s, Some(kw));
                            }
                            Some(Some(prev)) if *prev != kw => {
                                outref.insert(*s, None);
                            }
                            _ => {}
                        }
                    }
                    // batch-31c (N3 Fix 2 + N8, spec batch31-nomatch-illegalop §1.5/§1.9):
                    // out-param slot typing — a PSF'd slot feeding a NON-const identifier-
                    // typed reference param (`EInventoryTypes&out`, `FVector2D&out`) adopts
                    // the callee's param type; the call cannot compile otherwise, and the
                    // slot is an untyped scratch today (declared `int`, poisoning every
                    // downstream use: GetFirstItemWithType / GetCameraShotMode /
                    // PointCorrection's 9-line FVector2D cascade). Enum + F-struct heads
                    // only; const refs are READS (&in) and prove nothing about the slot.
                    // Same conflict-drop map as the float/bool out-refs.
                    if *is_psf
                        && pt.is_reference
                        && pt.token == 5
                        && !pt.is_object_const
                        && !pt.is_read_only
                        && !pt.is_object_handle
                    {
                        let ty = pt.base_name(refs);
                        let f_struct = ty.starts_with('F')
                            && ty.as_bytes().get(1).is_some_and(|c| c.is_ascii_uppercase());
                        if super::structure::is_enum_name(&ty) || f_struct {
                            match outref.get(s) {
                                None => {
                                    outref.insert(*s, Some(ty));
                                }
                                Some(Some(prev)) if *prev != ty => {
                                    outref.insert(*s, None);
                                }
                                _ => {}
                            }
                        }
                    }
                    // batch-28: int-family reference binding — see `iref` above.
                    if *is_psf
                        && pt.is_reference
                        && matches!(
                            pt.token,
                            0x44 | 0x45 | 0x46 | 0x47 | 0x4B | 0x4C | 0x4D | 0x4E
                        )
                    {
                        iref.insert(*s);
                    }
                    // batch-25g: numeric BY-VALUE args (value-pushed, non-reference params);
                    // batch-28b routes them through the shared `val_arg` channel (farg/ikeep
                    // + the small-int sarg candidates).
                    if !*is_psf && !pt.is_reference {
                        val_arg(*s, pt, farg, ikeep, sarg);
                    }
                }
            }
        }
    };
    for ins in &instrs {
        match ins.op.name {
            "PshVPtr" | "PshV4" | "PshV8" | "PSF" => {
                let s = w0(ins).unwrap_or(0);
                // batch-31c (N1): param slots (negative offsets) enter the model too — the
                // pair() body routes them into the keep-only channel. Slot 0 (`this`) stays
                // opaque.
                ostack.push(if s != 0 {
                    Some((s, ins.op.name == "PSF"))
                } else {
                    None
                });
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
            // batch-32c (spec batch31 §1.8 site-2 root cause, corrected by disasm): PopRPtr
            // POPS the top stack entry into the reference register (the member-read idiom
            // `PshVPtr w0 ; ADDSi ; RDSPtr ; ADDSi ; PopRPtr ; RDR4`). Unmodeled, it left a
            // ghost entry that shifted every later pairing one param deeper —
            // SetTimerDelegate's bool bMaxOncePerFrame slot paired against the float32
            // InitialStartDelay -> `float32 local_8` decl -> bare render where the bool wrap
            // `(local_8 != 0)` was needed (2 in-game no-match).
            "PopRPtr" => {
                ostack.pop();
            }
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let rs = refs
                    .func_ret_by_id(id)
                    .map(|d| !d.is_reference && ret_is_struct(&d.base_name(refs)))
                    .unwrap_or(false);
                pair(
                    &mut ostack,
                    refs.func_params_by_id(id),
                    refs.is_method_by_id(id),
                    rs,
                    &mut cand,
                    &mut outref,
                    &mut farg,
                    &mut ikeep,
                    &mut iref,
                    &mut sarg,
                );
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
                        // batch-31c: param slots (negative) never enter `cand` — their
                        // declarations are fixed by the signature.
                        if let Some(ty) = owner.filter(|_| rslot > 0) {
                            if !ty.is_empty() {
                                match cand.get(&rslot) {
                                    None => {
                                        cand.insert(rslot, Some(ty));
                                    }
                                    Some(Some(prev)) if *prev != ty => {
                                        cand.insert(rslot, None);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    // batch-28b (spec §5.1): pair the nargs ctor args (the entries BELOW the
                    // receiver) against the behaviour's params for the NUMERIC channels only
                    // (farg/ikeep/sarg — never `cand`, preserving the arm's receiver-
                    // mistyping guard). Args are reverse-pushed like every call type, so the
                    // entry nearest the receiver is params[0]. Feeds the FColor uint8 /
                    // FRotator float ctor args (C4a/C4b + infer_float_flow seed (f)).
                    if let Some(params) = refs.func_params_by_ptr(ptr) {
                        let args_end = ostack.len().saturating_sub(1); // receiver on top
                        let take = params.len().min(args_end);
                        for (i, slot) in ostack[args_end - take..args_end].iter().rev().enumerate()
                        {
                            if let (Some((s, is_psf)), Some(pt)) = (slot, params.get(i)) {
                                // batch-31c: locals only — negative (param) slots must not
                                // reach the farg/sarg retype maps.
                                if *s > 0 && !*is_psf && !pt.is_reference {
                                    val_arg(*s, pt, &mut farg, &mut ikeep, &mut sarg);
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
                // batch-32c (site-3b root cause): the `opCast` behaviour consumes exactly
                // THREE pushes (`TYPEID ; PSF <dst> ; PshVPtr <src> ; CALLSYS opCast`) and
                // pushes nothing. Routing it through pair() desynced the model at every Cast
                // diamond (unknown params -> ostack.clear() wiped the enclosing call's
                // pending args; known params mis-popped) — the bool&-out pairing for
                // GetFloatAttributeFromAbilitySystemComponent's `local_33` never happened
                // (declared int, "expected bool&, but got int"). Mirror of the $beh0
                // special-case above; structure.rs has its own dedicated opCast arm.
                if refs.func_by_ptr(ptr) == Some("opCast") {
                    let drop_n = 3.min(ostack.len());
                    ostack.truncate(ostack.len() - drop_n);
                    continue;
                }
                let rs = refs
                    .func_ret_by_ptr(ptr)
                    .map(|d| !d.is_reference && ret_is_struct(&d.base_name(refs)))
                    .unwrap_or(false);
                pair(
                    &mut ostack,
                    refs.func_params_by_ptr(ptr),
                    refs.is_method_by_ptr(ptr),
                    rs,
                    &mut cand,
                    &mut outref,
                    &mut farg,
                    &mut ikeep,
                    &mut iref,
                    &mut sarg,
                );
            }
            _ => {}
        }
    }
    let obj = cand
        .into_iter()
        .filter_map(|(slot, ty)| match ty {
            Some(t)
                if !written.contains(&slot)
                    && !is_primitive(t.trim_end_matches('@'))
                    && t != "void"
                    && !t.is_empty() =>
            {
                Some((slot, t))
            }
            _ => None,
        })
        .collect();
    // out-ref float slots ARE written (the callee's whole point is to write them; typically a
    // `SetV4 slot, 0` init precedes) — no never-written filter. Conflicts already dropped.
    let outref = outref
        .into_iter()
        .filter_map(|(slot, ty)| ty.map(|t| (slot, t)))
        .collect();
    // batch-25g: resolve the numeric value-arg candidates. A slot paired with BOTH an
    // int-family and a float-family param is ambiguous slot reuse — drop it from both sets.
    let mut float_args: HashMap<i32, String> = farg
        .into_iter()
        .filter_map(|(slot, ty)| ty.map(|t| (slot, t)))
        .collect();
    let both: Vec<i32> = float_args
        .keys()
        .copied()
        .filter(|s| ikeep.contains(s))
        .collect();
    for s in both {
        float_args.remove(&s);
        ikeep.remove(&s);
        sarg.remove(&s);
    }
    // batch-28b gates (spec §5.1): a small-int retype candidate must (b) have an op profile
    // of ONLY SetV1/SetV2/SetV4 constant writes + PshV4 value pushes — any arithmetic/
    // bitwise/conversion/compare/PSF/copy participation disqualifies (e.g. FindLastChar's
    // `int&` out-slot does SUBi math and must NOT retype) — and (c) every tracked SetV
    // constant must fit the target range (a mis-scoped -1 into uint8 would turn a warning
    // into an error). (SetV2: the FindLastChar/FindChar TCHAR consts are 2-byte writes —
    // `SetV2 w16, 0x3a` — spec §5.1 cites them as SetV4; disasm-verified SetV2.)
    // Conservative: an unrelated word operand that numerically collides with the slot also
    // disqualifies.
    let mut small_args: HashMap<i32, String> = sarg
        .into_iter()
        .filter_map(|(slot, ty)| ty.map(|t| (slot, t)))
        .collect();
    if !small_args.is_empty() {
        let fits = |kw: &str, bits: u32| -> bool {
            let v = bits as i32;
            match kw {
                "int8" => (-128..=127).contains(&v),
                "int16" => (-32768..=32767).contains(&v),
                "uint8" => (0..=255).contains(&v),
                "uint16" => (0..=65535).contains(&v),
                _ => false,
            }
        };
        for ins in &instrs {
            let n = ins.op.name;
            for (wi, &wd) in ins.words.iter().enumerate() {
                let s = wd as i16 as i32;
                let Some(kw) = small_args.get(&s) else {
                    continue;
                };
                let ok = match n {
                    "SetV1" | "SetV2" | "SetV4" => {
                        wi == 0 && fits(kw, ins.dwords.first().copied().unwrap_or(0))
                    }
                    "PshV4" => true,
                    _ => false,
                };
                if !ok {
                    small_args.remove(&s);
                }
            }
        }
    }
    (obj, outref, float_args, ikeep, iref, small_args)
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
                let (ph, nh) = (
                    prev.split('<').next().unwrap_or(prev),
                    ty.split('<').next().unwrap_or(&ty),
                );
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
                // field's value type. batch-32a: resolved through the cross-module class-fields
                // index (batch-30c `field_type_by_class`, keyed by the tid's OWNER class — no
                // cross-class field-name collision is possible), because every proven A5 owner
                // is a FOREIGN class the old this-class-only gate could never match (T7
                // OldTypeId names the owner; batch-29 §1.2). Own-class sites resolve through
                // the same index; the `fields` param stays as fallback for a driver without
                // the injected index. The body renderer emits the matching `dst = obj.field;`
                // assignment ONLY when this typing landed (slot_type == value type) — see the
                // structure.rs CpyRtoV8 arm.
                if let Some(next) = instrs.get(i + 1) {
                    if next.op.name == "CpyRtoV8" {
                        let dst = next.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
                        if dst > 0 {
                            let vty = refs.member(tid, off).and_then(|fname| {
                                refs.type_by_id(tid)
                                    .and_then(|cls| refs.field_type_by_class(cls, fname))
                                    .map(|s| s.to_string())
                                    .or_else(|| {
                                        (refs.type_by_id(tid) == class_name)
                                            .then(|| fields.and_then(|m| m.get(fname)).cloned())
                                            .flatten()
                                    })
                            });
                            if let Some(vty) = vty {
                                record(&mut cand, dst, vty);
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
    cand.into_iter()
        .filter_map(|(s, t)| t.map(|t| (s, t)))
        .collect()
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
            let ty = super::types::DataType {
                token: 5,
                type_info: *tinfo,
                is_object_handle: true,
                ..Default::default()
            }
            .base_name(refs);
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
    let consume =
        |stack: &mut Vec<Ent>, params: Option<usize>, is_method: bool, ret_struct: bool| {
            let Some(n) = params else {
                stack.clear();
                return;
            };
            let rvo = (ret_struct && is_method) as usize;
            let total = if is_method { n + 1 + rvo } else { n };
            stack.truncate(stack.len() - total.min(stack.len()));
        };
    for ins in &instrs {
        match ins.op.name {
            "PshVPtr" | "PSF" => {
                stack.push(Ent::Slot {
                    slot: w0(ins),
                    psf: ins.op.name == "PSF",
                });
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
                let rs = refs
                    .func_ret_by_id(id)
                    .map(|d| !d.is_reference && ret_is_struct(&d.base_name(refs)))
                    .unwrap_or(false);
                consume(
                    &mut stack,
                    refs.func_params_by_id(id).map(|p| p.len()),
                    refs.is_method_by_id(id),
                    rs,
                );
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
                    if let Ent::Slot {
                        slot: out_slot,
                        psf: true,
                    } = *out
                    {
                        record_iterator_candidate(refs, known, &mut cand, out_slot, ptr, container);
                    }
                }
                let rs = refs
                    .func_ret_by_ptr(ptr)
                    .map(|d| !d.is_reference && ret_is_struct(&d.base_name(refs)))
                    .unwrap_or(false);
                consume(
                    &mut stack,
                    refs.func_params_by_ptr(ptr).map(|p| p.len()),
                    refs.is_method_by_ptr(ptr),
                    rs,
                );
            }
            _ => {}
        }
    }
    cand.into_iter()
        .filter_map(|(s, t)| t.map(|t| (s, t)))
        .collect()
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
    let Some(head) = refs.func_ret_by_ptr(ptr).map(|d| d.base_name(refs)) else {
        return;
    };
    if !head.starts_with('T') || !head.contains("Iterator") {
        return;
    }
    let ty = if head.contains('<') {
        head
    } else {
        // the container must itself be a subtyped template (`TArray<AGothicCharacter>`).
        let Some(c) = container else { return };
        let (Some(lt), Some(gt)) = (c.find('<'), c.rfind('>')) else {
            return;
        };
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
/// batch-30a (C6c): slots whose ONLY object write is a `STOREOBJ` DIRECTLY following a
/// call, mapped to that call's returned object base type. Conservative: any second
/// object write to the slot (another STOREOBJ, or a `RefCpyV` copy) disqualifies it,
/// as does a STOREOBJ whose producing call/return type is unknown. Consumed by the
/// member-override gate in `emit_function` (vanilla obj_locals type wins over a
/// member-derived WIDENING when the cache and the sole producing call agree).
fn infer_sole_call_store_types(f: &Func, refs: &RefResolver) -> HashMap<i32, String> {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return HashMap::new(),
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
    let mut cand: HashMap<i32, Option<String>> = HashMap::new();
    for (i, ins) in instrs.iter().enumerate() {
        let dst = match ins.op.name {
            "STOREOBJ" | "RefCpyV" => w0(ins),
            _ => continue,
        };
        if dst <= 0 {
            continue;
        }
        // the producing call's object return type — only for a STOREOBJ right after it.
        let ret = (ins.op.name == "STOREOBJ")
            .then(|| i.checked_sub(1).and_then(|j| instrs.get(j)))
            .flatten()
            .and_then(|prev| match prev.op.name {
                "CALL" | "CALLINTF" | "CALLBND" => {
                    let id = prev.dwords.first().copied().unwrap_or(0) as i32;
                    refs.func_ret_by_id(id)
                        .filter(|d| d.token == 5)
                        .map(|d| d.base_name(refs))
                }
                "CALLSYS" | "Thiscall1" => {
                    let ptr = prev.qwords.first().copied().unwrap_or(0) as i64;
                    refs.func_ret_by_ptr(ptr)
                        .filter(|d| d.token == 5)
                        .map(|d| d.base_name(refs))
                }
                _ => None,
            });
        match (cand.get(&dst), ret) {
            (None, Some(t)) => {
                cand.insert(dst, Some(t));
            }
            // second write / unknown producer -> disqualify (keep a tombstone).
            _ => {
                cand.insert(dst, None);
            }
        }
    }
    cand.into_iter()
        .filter_map(|(s, t)| t.map(|t| (s, t)))
        .collect()
}

/// Slots whose every object write is a `STOREOBJ` right after a call that returns a CONST object.
/// AngelScript rejects storing such a value into a plain local ("Can't implicitly convert from
/// 'const X' to 'X'"), so vanilla declared these const — and a const local has to be initialized
/// where it is declared. A write from anything else — a handle copy, a call whose return is not
/// provably const — drops the slot.
fn infer_const_result_slots(f: &Func, refs: &RefResolver) -> HashSet<i32> {
    let instrs = match disassemble(&f.bytecode) {
        Ok(instrs) => instrs,
        Err(_) => return HashSet::new(),
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
    let mut candidate: HashMap<i32, bool> = HashMap::new();
    for (index, ins) in instrs.iter().enumerate() {
        let destination = match ins.op.name {
            "STOREOBJ" | "RefCpyV" => w0(ins),
            _ => continue,
        };
        if destination <= 0 {
            continue;
        }
        let returns_const = (ins.op.name == "STOREOBJ")
            .then(|| index.checked_sub(1).and_then(|j| instrs.get(j)))
            .flatten()
            .and_then(|previous| match previous.op.name {
                "CALL" | "CALLINTF" | "CALLBND" => {
                    let id = previous.dwords.first().copied().unwrap_or(0) as i32;
                    Some((
                        refs.func_ret_by_id(id)?,
                        refs.func_by_id(id)?,
                        refs.is_const_method_by_id(id),
                    ))
                }
                "CALLSYS" | "Thiscall1" => {
                    let ptr = previous.qwords.first().copied().unwrap_or(0) as i64;
                    Some((
                        refs.func_ret_by_ptr(ptr)?,
                        refs.func_by_ptr(ptr)?,
                        refs.is_const_method_by_ptr(ptr),
                    ))
                }
                _ => None,
            })
            // A name whose rows disagree about the qualifier is re-emitted WITHOUT it (see
            // `emit_function_ctor`), so its result is not const in the recompiled source either.
            .is_some_and(|(ret, name, is_const_method)| {
                ret.token == 5
                    && (ret.is_object_const || ret.is_read_only)
                    && !refs.const_return_is_inconsistent(name, is_const_method)
            });
        // Every write has to come from a provably const-returning call. A slot the compiler
        // re-used for several of them is still const-valued — the declaration rewrite gives each
        // definition its own declaration — but one write from anything else (a handle copy, a
        // non-const call) settles it as a plain local for good.
        let verdict = candidate.get(&destination).copied().unwrap_or(true) && returns_const;
        candidate.insert(destination, verdict);
    }
    candidate
        .into_iter()
        .filter_map(|(slot, is_const)| is_const.then_some(slot))
        .collect()
}

/// Declare a const-valued local where it gets its value. A `const` local cannot be hoisted: the
/// language requires it to be initialized at its declaration.
fn rewrite_const_decl_init(
    body: &str,
    locals: &BTreeMap<i32, String>,
    const_slots: &HashSet<i32>,
    refs: &RefResolver,
) -> (String, HashSet<i32>) {
    if const_slots.is_empty() {
        return (body.to_owned(), HashSet::new());
    }
    let want = |slot: i32, _ty: &str| const_slots.contains(&slot);
    rewrite_decl_at_assignment(body, locals, &want, &|ty| {
        format!("const {}", qualify_decl_type(ty, refs))
    })
}

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
            && (op.starts_with("SetV")
                || op.starts_with("CpyVtoV")
                || op.starts_with("RDR")
                || op.contains("TO")
                || op == "PopRPtr"
                || op.starts_with("ADD")
                || op.starts_with("SUB")
                || op.starts_with("MUL")
                || op.starts_with("DIV")
                || op.starts_with("MOD")
                || op.starts_with("NEG")
                || op.starts_with("Inc")
                || op.starts_with("Dec")
                || op == "NOT")
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
    let consume =
        |stack: &mut Vec<Option<i32>>, params: Option<usize>, is_method: bool, ret_struct: bool| {
            let Some(n) = params else {
                stack.clear();
                return;
            };
            let rvo = (ret_struct && is_method) as usize;
            let total = if is_method { n + 1 + rvo } else { n };
            stack.truncate(stack.len() - total.min(stack.len()));
        };
    // rendered return type + receiver slot of the just-completed call (None once anything
    // non-benign executes, so a later CpyRtoV can't adopt a stale type).
    let mut last: Option<(String, Option<i32>)> = None;
    // batch-31d (N8, spec batch31-nomatch-illegalop §1.9): value type of the LAST ADDSi
    // member access, surviving a following PopRPtr — the `PshVPtr base ; ADDSi off,tid ;
    // [ADDSi ...] ; PopRPtr ; CpyRtoV8 dst` idiom is a MEMBER READ into the register, and
    // the dst slot (PointCorrection's w24 = `CalculatedPosition.Position`, FVector2D) was
    // width-declared `int`, poisoning 9 error lines (5 no-match + 2 no-conversion-to-math
    // + 2 implconv). The ADDSi tid resolves the field's value type independent of the base
    // slot (script owners only — field_type_by_class; native owners yield None). NOTE: the
    // spec's §1.9 "&out scratch" premise is disasm-REFUTED (w24 is never PSF'd; it feeds
    // BY-VALUE FVector2D params) — this tracker is the corrected lever, same declaration-
    // only, primitive-upgrade-only safety posture as the A3 call-result case.
    let mut member_ty: Option<String> = None; // last ADDSi's field value type
    let mut member_reg: Option<String> = None; // ... after PopRPtr loaded the register
    let mut ostack: Vec<Option<i32>> = Vec::new();
    let mut cand: HashMap<i32, Option<String>> = HashMap::new();
    for ins in &instrs {
        // PopRPtr moves the member pointer into the register (member_ty -> member_reg, the
        // form a following CpyRtoV may consume); everything except ADDSi/PopRPtr/SUSPEND/
        // CpyRtoV kills both trackers.
        match ins.op.name {
            "ADDSi" | "SUSPEND" | "CpyRtoV4" | "CpyRtoV8" => {}
            "PopRPtr" => {
                member_reg = member_ty.take();
            }
            _ => {
                member_ty = None;
                member_reg = None;
            }
        }
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
                let off = w0(ins);
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                member_ty = refs.member(tid, off).and_then(|field| {
                    refs.type_by_id(tid)
                        .and_then(|cls| refs.field_type_by_class(cls, field))
                        .map(|s| s.to_string())
                });
                last = None;
            }
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let is_m = refs.is_method_by_id(id);
                let recv = if is_m {
                    ostack.last().copied().flatten()
                } else {
                    None
                };
                let ret = refs.func_ret_by_id(id).map(|d| d.base_name(refs));
                let rs = refs
                    .func_ret_by_id(id)
                    .map(|d| !d.is_reference && ret_is_struct(&d.base_name(refs)))
                    .unwrap_or(false);
                consume(
                    &mut ostack,
                    refs.func_params_by_id(id).map(|p| p.len()),
                    is_m,
                    rs,
                );
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
                let recv = if is_m {
                    ostack.last().copied().flatten()
                } else {
                    None
                };
                let ret = refs.func_ret_by_ptr(ptr).map(|d| d.base_name(refs));
                let rs = refs
                    .func_ret_by_ptr(ptr)
                    .map(|d| !d.is_reference && ret_is_struct(&d.base_name(refs)))
                    .unwrap_or(false);
                consume(
                    &mut ostack,
                    refs.func_params_by_ptr(ptr).map(|p| p.len()),
                    is_m,
                    rs,
                );
                last = ret.map(|t| (t, recv));
            }
            "CpyRtoV4" | "CpyRtoV8" => {
                // A3 call-result case first; else the batch-31d member-read case (the
                // PopRPtr'd ADDSi chain) — identical candidate/conflict discipline, and
                // usable_ret_type keeps primitives/enums/bare templates out either way.
                let src_ty = last.take().or_else(|| member_reg.take().map(|t| (t, None)));
                if let Some((ty, recv)) = src_ty {
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
    cand.into_iter()
        .filter_map(|(s, t)| t.map(|t| (s, t)))
        .collect()
}

/// Propagate one proven enum type through raw four-byte local copies, but only along paths that
/// end in an enum-underlying-width conversion. A raw `CpyVtoV4` alone is ambiguous (it may be an
/// enum-to-int assignment), while `CpyVtoV4 dst, enum_src; sbTOi/ubTOi int_dst, dst` proves that
/// `dst` still carries the enum-width value. Every participating slot must have an enum-compatible
/// opcode profile; arithmetic, bool, float, address, or unrelated uses reject that slot.
fn propagate_proven_enum_slots(
    instrs: &[super::disasm::Instr],
    seeds: Vec<(i32, String)>,
) -> HashMap<i32, String> {
    let mut edges = Vec::new(); // (destination, source)
    let mut narrow_reads = HashSet::new();
    for ins in instrs {
        match ins.op.name {
            "CpyVtoV4" => {
                if let (Some(&dst), Some(&src)) = (ins.words.first(), ins.words.get(1)) {
                    edges.push((dst as i16 as i32, src as i16 as i32));
                }
            }
            "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" => {
                if let Some(&src) = ins.words.get(1) {
                    narrow_reads.insert(src as i16 as i32);
                }
            }
            _ => {}
        }
    }

    let profile_ok = |slot: i32| {
        instrs.iter().all(|ins| {
            ins.words.iter().enumerate().all(|(wi, &word)| {
                if word as i16 as i32 != slot {
                    return true;
                }
                match ins.op.name {
                    "SetV1" | "SetV2" | "SetV4" | "CpyRtoV4" | "CpyVtoR4" | "PshV4" => wi == 0,
                    "CpyVtoV4" => wi < 2,
                    "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" => wi == 1,
                    _ => false,
                }
            })
        })
    };

    // Only copy destinations that eventually feed a width conversion are type-preserving enough
    // to inherit the enum. Work backwards so an arbitrary number of compiler copy temporaries is
    // allowed without retyping a dead or generic enum-to-int copy.
    let mut relevant = narrow_reads;
    loop {
        let mut changed = false;
        for &(dst, src) in &edges {
            if relevant.contains(&dst) {
                changed |= relevant.insert(src);
            }
        }
        if !changed {
            break;
        }
    }

    let mut types: HashMap<i32, Option<String>> = HashMap::new();
    for (slot, ty) in seeds {
        if slot <= 0 || !profile_ok(slot) {
            continue;
        }
        match types.get(&slot) {
            None => {
                types.insert(slot, Some(ty));
            }
            Some(Some(prev)) if *prev != ty => {
                types.insert(slot, None);
            }
            _ => {}
        }
    }
    loop {
        let mut changed = false;
        for &(dst, src) in &edges {
            if !relevant.contains(&dst) || !profile_ok(dst) {
                continue;
            }
            let Some(Some(src_ty)) = types.get(&src).cloned() else {
                continue;
            };
            match types.get(&dst) {
                None => {
                    types.insert(dst, Some(src_ty));
                    changed = true;
                }
                Some(Some(prev)) if *prev != src_ty => {
                    types.insert(dst, None);
                    changed = true;
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }
    types
        .into_iter()
        .filter_map(|(slot, ty)| ty.map(|ty| (slot, ty)))
        .collect()
}

/// Enum local inference for enum-returning state-machine functions. The seed requires the exact
/// same enum type from a resolved call result, copied into a local that is also used as a return
/// payload of this enum-returning function. That double witness prevents a coincidental raw V4
/// scratch from becoming an enum. [`propagate_proven_enum_slots`] supplies the separately-gated
/// copy-chain extension used by loop status variables.
fn infer_enum_flow(f: &Func, refs: &RefResolver) -> HashMap<i32, String> {
    if f.ret.token != 5 || f.ret.is_reference {
        return HashMap::new();
    }
    let ret_enum = f.ret.base_name(refs);
    if !is_enum(&ret_enum) {
        return HashMap::new();
    }
    let instrs = match disassemble(&f.bytecode) {
        Ok(instrs) => instrs,
        Err(_) => return HashMap::new(),
    };
    let return_slots: HashSet<i32> = instrs
        .iter()
        .filter(|ins| ins.op.name == "CpyVtoR4")
        .filter_map(|ins| ins.words.first().map(|word| *word as i16 as i32))
        .collect();
    let mut last_ret: Option<String> = None;
    let mut seeds = Vec::new();
    for ins in &instrs {
        match ins.op.name {
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                last_ret = refs
                    .func_ret_by_id(id)
                    .filter(|ty| !ty.is_reference && ty.token == 5)
                    .map(|ty| ty.base_name(refs))
                    .filter(|ty| ty == &ret_enum);
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                last_ret = refs
                    .func_ret_by_ptr(ptr)
                    .filter(|ty| !ty.is_reference && ty.token == 5)
                    .map(|ty| ty.base_name(refs))
                    .filter(|ty| ty == &ret_enum);
            }
            "CpyRtoV4" => {
                let slot = ins.words.first().map(|word| *word as i16 as i32);
                if let (Some(ty), Some(slot)) = (last_ret.take(), slot) {
                    if slot > 0 && return_slots.contains(&slot) {
                        seeds.push((slot, ty));
                    }
                }
            }
            "SUSPEND" => {}
            _ => last_ret = None,
        }
    }
    propagate_proven_enum_slots(&instrs, seeds)
}

fn bool_slot_profile_is_safe(instrs: &[super::disasm::Instr], slot: i32) -> bool {
    instrs.iter().enumerate().all(|(index, ins)| {
        ins.words.iter().enumerate().all(|(wi, &word)| {
            if word as i16 as i32 != slot {
                return true;
            }
            match ins.op.name {
                "SetV1" => wi == 0 && ins.dwords.first().copied().unwrap_or(2) <= 1,
                "RDR1" | "WRTV1" | "NOT" | "CpyVtoR1" => wi == 0,
                // A T* result is canonical 0/1 in the full value register. Reject an arbitrary
                // call/arithmetic register copy into the bool slot.
                "CpyRtoV4" => {
                    wi == 0
                        && index.checked_sub(1).is_some_and(|prev| {
                            matches!(
                                instrs[prev].op.name,
                                "TZ" | "TNZ" | "TS" | "TNS" | "TP" | "TNP"
                            )
                        })
                }
                // Propagating a proven bool OUT is harmless; accepting an unproved source INTO
                // the candidate would change integer truthiness into a bool conversion.
                "CpyVtoV4" => wi == 1,
                _ => false,
            }
        })
    })
}

/// Infer a primitive bool local from exact own-field byte reads/writes. A candidate needs a
/// resolved `bool` field witness, no unresolved/non-bool byte-field access on the same slot, and a
/// whole-function bool-only opcode profile. This turns compiler bool temporaries back into bools
/// without treating arbitrary SetV1/int8 scratch as boolean.
/// The type each slot's captured call result has: `CpyRtoV*`/`STOREOBJ` right after a call copies
/// that call's return value into the slot, and the cache records what the callee returns. A slot
/// that captures two DIFFERENT types is the compiler reusing it, and carries no witness at all.
fn call_result_types(f: &Func, refs: &RefResolver) -> HashMap<i32, String> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashMap::new();
    };
    let mut types: HashMap<i32, String> = HashMap::new();
    let mut conflicting: HashSet<i32> = HashSet::new();
    let mut returned: Option<String> = None;
    for ins in &instrs {
        match ins.op.name {
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                returned = refs.func_ret_by_id(id).map(|d| d.base_name(refs));
            }
            "CALLSYS" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                returned = refs.func_ret_by_ptr(ptr).map(|d| d.base_name(refs));
            }
            "CpyRtoV1" | "CpyRtoV4" | "CpyRtoV8" | "STOREOBJ" => {
                if let (Some(ty), Some(slot)) = (
                    returned.take(),
                    ins.words.first().map(|word| *word as i16 as i32),
                ) {
                    record_slot_type(&mut types, &mut conflicting, slot, ty);
                }
            }
            // A slot-to-slot copy carries the value, and with it what the value is. Without this
            // step the type stops at the compiler's own scratch slot and the one the source
            // actually named is left untyped.
            "CpyVtoV1" | "CpyVtoV4" | "CpyVtoV8" => {
                returned = None;
                let dst = ins.words.first().map(|word| *word as i16 as i32);
                let src = ins.words.get(1).map(|word| *word as i16 as i32);
                if let (Some(dst), Some(ty)) = (dst, src.and_then(|src| types.get(&src)).cloned()) {
                    record_slot_type(&mut types, &mut conflicting, dst, ty);
                }
            }
            _ => returned = None,
        }
    }
    types.retain(|slot, _| !conflicting.contains(slot));
    types
}

/// Note what a slot holds, or that it holds two different things and proves nothing.
fn record_slot_type(
    types: &mut HashMap<i32, String>,
    conflicting: &mut HashSet<i32>,
    slot: i32,
    ty: String,
) {
    if slot <= 0 {
        return;
    }
    match types.get(&slot) {
        Some(known) if *known != ty => {
            conflicting.insert(slot);
        }
        _ => {
            types.insert(slot, ty);
        }
    }
}

/// Slots whose producer the SOURCE kept as a statement of its own. AngelScript evaluates a
/// call's arguments last to first and emits each argument's code immediately before its own
/// push, so a value that is written and only pushed AFTER some other operand went on the stack
/// was computed before the call — a statement. A value pushed with nothing in between was
/// evaluated at the call.
///
/// Only the proven case is collected. A slot this says nothing about is not thereby proven
/// inline: reading it that way refuses far more than it should (measured: 4,222 functions).
fn statement_producer_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let pushes = |name: &str| matches!(name, "PshVPtr" | "PshV4" | "PshV8" | "PshC4" | "PshC8" | "PshGPtr" | "PshNull" | "PSF");
    let mut statements = HashSet::new();
    for (at, ins) in instrs.iter().enumerate() {
        if !matches!(
            ins.op.name,
            "STOREOBJ" | "CpyRtoV1" | "CpyRtoV4" | "CpyRtoV8"
        ) {
            continue;
        }
        let Some(slot) = ins.words.first().map(|word| *word as i16 as i32) else {
            continue;
        };
        if slot <= 0 {
            continue;
        }
        // Walk forward to this slot's own push, counting what went on the stack before it.
        let mut others = 0usize;
        for next in &instrs[at + 1..] {
            if !pushes(next.op.name) {
                if matches!(next.op.name, "CALL" | "CALLSYS" | "CALLINTF" | "CALLBND") {
                    break; // the call went out without pushing this slot
                }
                continue;
            }
            if next.words.first().map(|word| *word as i16 as i32) == Some(slot)
                && matches!(next.op.name, "PshVPtr" | "PshV4" | "PshV8" | "PSF")
            {
                if others > 0 {
                    statements.insert(slot);
                }
                break;
            }
            others += 1;
        }
    }
    statements
}

/// Slots vanilla itself treated as a full-expression TEMPORARY: destroyed exactly once, right
/// where the expression that used them ended, rather than at every exit from a block.
///
/// AngelScript destroys a named block-scope value local on every path out of its block, so its
/// destructor appears once per exit and always in the trailing group before a `RET` or a `JMP`.
/// A destructor that stands mid-expression, with ordinary work behind it, can only belong to a
/// temporary — which means vanilla ran that method on a temporary at that exact site, and so may
/// we. That is a stronger answer than `has_const_overload`, whose table has nothing to say about
/// most native value types.
fn temporary_receiver_slots(f: &Func, refs: &RefResolver) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let destroys = |ins: &Instr| {
        ins.op.name == "CALLSYS"
            && refs.func_by_ptr(ins.qwords.first().copied().unwrap_or(0) as i64) == Some("$beh2")
    };
    let mut destructions: HashMap<i32, Vec<usize>> = HashMap::new();
    for (at, ins) in instrs.iter().enumerate() {
        if at == 0 || !destroys(ins) || instrs[at - 1].op.name != "PSF" {
            continue;
        }
        let Some(slot) = instrs[at - 1].words.first().map(|word| *word as i16 as i32) else {
            continue;
        };
        destructions.entry(slot).or_default().push(at);
    }
    destructions
        .into_iter()
        .filter(|(slot, at)| *slot > 0 && at.len() == 1)
        .filter(|(_, at)| {
            // Mid-expression: what follows is real work, not the rest of a scope's cleanup group
            // running into the block's exit.
            instrs[at[0] + 1..]
                .iter()
                .find(|next| next.op.name != "PSF" && !destroys(next))
                .is_some_and(|next| !matches!(next.op.name, "RET" | "JMP"))
        })
        .map(|(slot, _)| slot)
        .collect()
}

/// Slots a `Cast<T>` fills, with the T it fills them with: the cast writes its own out-slot and
/// the source's slot takes it through `PshVPtr <out>; RefCpyV <slot>`, with the diamond's join
/// jump and null arm in between.
fn cast_result_slots(f: &Func, refs: &RefResolver) -> Vec<(i32, String)> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let (mut tid, mut out) = (None, None);
    for (at, ins) in instrs.iter().enumerate() {
        match ins.op.name {
            "TYPEID" => {
                tid = ins.dwords.first().map(|d| *d as i32);
                out = None;
            }
            "PSF" if tid.is_some() => out = ins.words.first().map(|word| *word as i16 as i32),
            "CALLSYS"
                if refs.func_by_ptr(ins.qwords.first().copied().unwrap_or(0) as i64)
                    == Some("opCast") =>
            {
                let target = tid.and_then(|tid| super::structure::resolve_cast_typeid(refs, tid));
                if let (Some(target), Some(out)) = (target, out) {
                    if matches!(target.bytes().next(), Some(b'U') | Some(b'A')) {
                        let copy = (at + 1..instrs.len().min(at + 5))
                            .find(|k| !matches!(instrs[*k].op.name, "JMP" | "ClrVPtr"))
                            .filter(|k| {
                                instrs[*k].op.name == "PshVPtr"
                                    && instrs[*k].words.first().map(|w| *w as i16 as i32)
                                        == Some(out)
                            });
                        let slot = copy
                            .and_then(|k| instrs.get(k + 1))
                            .filter(|next| next.op.name == "RefCpyV")
                            .and_then(|next| next.words.first())
                            .map(|word| *word as i16 as i32);
                        if let Some(slot) = slot.filter(|slot| *slot > 0) {
                            found.push((slot, target));
                        }
                    }
                }
                tid = None;
                out = None;
            }
            _ => {}
        }
    }
    found
}

/// Slots vanilla ALIASED a handle into with `RefCpyV`.
///
/// Nothing in an expression needs that instruction. Calling on a handle an expression produced
/// pushes the handle straight from wherever it landed; the alias exists because the source gave
/// the handle a name and every later mention reads it through that name. So its presence is the
/// proof a name was there, and its absence — vanilla calling on the producing slot itself — the
/// proof there was none.
fn handle_alias_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    instrs
        .iter()
        .filter(|ins| ins.op.name == "RefCpyV")
        .filter_map(|ins| ins.words.first().map(|word| *word as i16 as i32))
        .filter(|slot| *slot > 0)
        .collect()
}

/// The slots of a WIDENING that the source gave a NAME to.
///
/// A widening (`fTOd` and friends) writes a value into a slot wider than the value's own type.
/// Two shapes produce one, and only one of them is a name:
///
/// * `fTOd t, x; CpyVtoV8 n, t` — the widened value is copied ON into another slot. Nothing in an
///   expression needs that copy; it is a declaration, `float n = <float32 expr>`. Both `t` and `n`
///   belong to it.
/// * `fTOd t, x; MULd t, t, y` — the widening is consumed where it lands. That is an expression
///   temporary and folding it away is exactly right.
///
/// Only the first shape is protected, and it has to be: fold the name away and the arithmetic
/// behind it happens at the narrower width — vanilla's `CMPd` against a `f64` becomes a single
/// `CMPIf`, which is a different comparison, not a shorter spelling of the same one.
fn widened_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let slot = |word: Option<&u16>| word.map(|word| *word as i16 as i32);
    let widened: HashSet<i32> = instrs
        .iter()
        .filter(|ins| matches!(ins.op.name, "fTOd" | "iTOd" | "uTOd" | "i64TOd" | "u64TOd"))
        .filter_map(|ins| slot(ins.words.first()))
        .filter(|dest| *dest > 0)
        .collect();
    let mut named = HashSet::new();
    for ins in &instrs {
        if !matches!(ins.op.name, "CpyVtoV4" | "CpyVtoV8") {
            continue;
        }
        let (Some(dest), Some(src)) = (slot(ins.words.first()), slot(ins.words.get(1))) else {
            continue;
        };
        if dest > 0 && widened.contains(&src) {
            named.insert(dest);
            named.insert(src);
        }
    }
    named
}

/// Slots the compiler RELEASES with `FreeNullV8` — the element of a range-for, released at the
/// end of every iteration. A slot with an iteration-scoped lifetime is not a droppable alias for
/// something that outlives it, and dropping it is what leaves the loop rendered as
/// `while (it.CanProceed)` instead of the range-for the source wrote: `rewrite_foreach_loops`
/// matches on that very element.
fn loop_element_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    instrs
        .iter()
        .filter(|ins| matches!(ins.op.name, "FreeNullV8" | "FreeNullV4" | "FREE"))
        .filter_map(|ins| ins.words.first().map(|word| *word as i16 as i32))
        .filter(|slot| *slot > 0)
        .collect()
}

/// Slots that take a COMPARISON's result. `CMP*` + `T*` leaves a boolean in the value register,
/// and the slot that catches it holds a bool — typed int instead, every read of it is wrapped
/// `(x != 0)` and the write becomes `int(<cmp>)`, which costs a compare and a test per use.
fn comparison_result_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let mut slots = HashSet::new();
    let mut tested = false;
    for ins in &instrs {
        match ins.op.name {
            "TZ" | "TNZ" | "TS" | "TNS" | "TP" | "TNP" => tested = true,
            "CpyRtoV1" | "CpyRtoV4" => {
                if tested {
                    if let Some(slot) = ins.words.first().map(|word| *word as i16 as i32) {
                        if slot > 0 {
                            slots.insert(slot);
                        }
                    }
                }
                tested = false;
            }
            _ => tested = false,
        }
    }
    slots
}

/// Slots that take a bool-returning call's result. `CpyRtoV*` right after a call copies the
/// value register into the slot, so the slot holds what the callee returns. Left typed int,
/// every read of it is wrapped `(x != 0)` and the compiler materializes a comparison and a test
/// where vanilla just copied the byte.
fn bool_call_result_slots(f: &Func, refs: &RefResolver) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let mut slots = HashSet::new();
    let mut returns_bool = false;
    for ins in &instrs {
        match ins.op.name {
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                returns_bool = refs
                    .func_ret_by_id(id)
                    .map(|d| d.base_name(refs))
                    .as_deref()
                    == Some("bool");
            }
            "CALLSYS" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                returns_bool = refs
                    .func_ret_by_ptr(ptr)
                    .map(|d| d.base_name(refs))
                    .as_deref()
                    == Some("bool");
            }
            "CpyRtoV1" | "CpyRtoV4" => {
                if returns_bool {
                    if let Some(slot) = ins.words.first().map(|word| *word as i16 as i32) {
                        if slot > 0 {
                            slots.insert(slot);
                        }
                    }
                }
                returns_bool = false;
            }
            _ => returns_bool = false,
        }
    }
    slots
}

/// Slots the function stores a 4- or 8-byte literal into. A bool is stored with `SetV1`, so a
/// wider store is the slot's own width evidence — EXCEPT for one compiler idiom: the `&&`
/// short-circuit writes its `false` result with `SetV4 slot, 0` between the conditional jump that
/// short-circuited and the jump over the right-hand operand. That slot holds the bool result of
/// the whole expression, and reading its width as int costs the `!` in 190 functions (measured:
/// every one of the 724 wide stores landing on a NOT-proven slot has exactly this shape).
/// Slots the function reads ONE BYTE wide into the value register (`CpyVtoR1`).
///
/// That read is how a branch takes its condition, and nothing else uses it. A slot read that way
/// holds a bool, whatever width some store into it happened to have.
fn byte_read_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    instrs
        .iter()
        .filter(|ins| ins.op.name == "CpyVtoR1")
        .filter_map(|ins| ins.words.first().map(|word| *word as i16 as i32))
        .filter(|slot| *slot > 0)
        .collect()
}

fn wide_literal_store_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let short_circuit_result = |at: usize| {
        let before = at
            .checked_sub(1)
            .map(|prev| is_conditional_jump(instrs[prev].op.name));
        before == Some(true)
            && instrs.get(at + 1).map(|next| next.op.name) == Some("JMP")
            && instrs[at].dwords.first().copied() == Some(0)
    };
    instrs
        .iter()
        .enumerate()
        .filter(|(at, ins)| {
            matches!(ins.op.name, "SetV4" | "SetV8") && !short_circuit_result(*at)
        })
        .filter_map(|(_, ins)| ins.words.first().map(|word| *word as i16 as i32))
        .filter(|slot| *slot > 0)
        .collect()
}

/// The conditional-jump opcodes (the structurer's own list).
fn is_conditional_jump(name: &str) -> bool {
    matches!(
        name,
        "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
    )
}

/// Slots `NOT` is applied to — AngelScript's logical not, which only takes a bool variable.
fn not_operand_slots(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    instrs
        .iter()
        .filter(|ins| ins.op.name == "NOT")
        .filter_map(|ins| ins.words.first().map(|word| *word as i16 as i32))
        .filter(|slot| *slot > 0)
        .collect()
}

fn infer_bool_field_slots(
    f: &Func,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
) -> HashSet<i32> {
    let Some(fields) = fields else {
        return HashSet::new();
    };
    let instrs = match disassemble(&f.bytecode) {
        Ok(instrs) => instrs,
        Err(_) => return HashSet::new(),
    };
    let mut ref_is_bool: Option<bool> = None;
    let mut proven = HashSet::new();
    let mut conflict = HashSet::new();
    for ins in &instrs {
        match ins.op.name {
            "LoadThisR" => {
                let off = ins.words.first().map(|word| *word as i16 as i32);
                let tid = ins.dwords.first().copied().map(|id| id as i32);
                ref_is_bool = off
                    .zip(tid)
                    .and_then(|(off, tid)| refs.member(tid, off))
                    .and_then(|field| fields.get(field))
                    .map(|ty| ty == "bool");
            }
            "CHKREF" | "ChkRefS" | "ChkNullV" | "SUSPEND" => {}
            "RDR1" | "WRTV1" => {
                if let Some(slot) = ins.words.first().map(|word| *word as i16 as i32) {
                    if slot > 0 {
                        if ref_is_bool == Some(true) {
                            proven.insert(slot);
                        } else {
                            conflict.insert(slot);
                        }
                    }
                }
                ref_is_bool = None;
            }
            _ => ref_is_bool = None,
        }
    }
    proven.retain(|slot| !conflict.contains(slot) && bool_slot_profile_is_safe(&instrs, *slot));
    proven
}

/// Filter/compose one A3 return-type candidate (see [`infer_call_result_types`]):
/// primitives/enums/placeholders are rejected (int declarations already work for them); a bare
/// template head (`TMapIteratorPair` with no `<...>` in its T1 entry) is composed from the
/// receiver iterator's inferred instantiation, or rejected when that isn't available either.
fn usable_ret_type(ty: String, recv: Option<i32>, known: &HashMap<i32, String>) -> Option<String> {
    if ty.is_empty()
        || ty == "void"
        || ty == "?"
        || ty == "auto"
        || is_primitive(&ty)
        || is_enum(&ty)
    {
        return None;
    }
    let b = ty.as_bytes();
    let bare_template =
        !ty.contains('<') && b.len() >= 2 && b[0] == b'T' && b[1].is_ascii_uppercase();
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

/// batch-28 (specs/batch27-floatwarnings.md §2.3): numeric kind of a slot, as proven by the
/// unified dataflow pass. Keyword mapping: `F32 -> "float32"`, `F64 -> "float"` (the faithful
/// vanilla keyword in this `floatIsFloat64` build), `I64 -> "int64"`. Conflict = removal
/// (status-quo declaration).
#[derive(Clone, Copy, PartialEq, Debug)]
enum NumKind {
    F32,
    F64,
    I64,
}

/// Merge one piece of kind evidence into `slot` (returns true when anything changed).
/// Float evidence REPLACES an I64 guess (SetV8 raw bits are float64 bits whenever the slot
/// has float evidence); F32-vs-F64 is a REAL width conflict -> poison; float evidence landing
/// on an anti-seeded (proven-int) slot -> poison.
fn nk_apply(
    kind: &mut HashMap<i32, NumKind>,
    poison: &mut HashSet<i32>,
    anti: &HashSet<i32>,
    slot: i32,
    k: NumKind,
) -> bool {
    if poison.contains(&slot) {
        return false;
    }
    if !matches!(k, NumKind::I64) && anti.contains(&slot) {
        kind.remove(&slot);
        poison.insert(slot);
        return true;
    }
    match kind.get(&slot).copied() {
        None => {
            kind.insert(slot, k);
            true
        }
        Some(prev) if prev == k => false,
        Some(NumKind::I64) => {
            kind.insert(slot, k);
            true
        }
        Some(_) if matches!(k, NumKind::I64) => false, // float evidence wins, I64 seed dropped
        Some(_) => {
            kind.remove(&slot);
            poison.insert(slot);
            true
        }
    }
}

/// Recover the numeric value behind the VM's explicit local-address dereference idiom:
/// `PshVPtr slot; PopRPtr; RDRx value` reads `slot` into `value`, while the corresponding
/// `WRTVx value` writes it back. The address itself may previously have travelled through a
/// `CpyRtoV8` (iterator `Proceed()` returning `T&`), so the ordinary width-copy pass sees only
/// an opaque 8-byte pointer and otherwise declares the pointee slot as `int`.
///
/// Requiring the exact adjacent three-op shape is the safety gate: a pushed pointer used as a
/// call argument/member base is not numeric evidence. The resulting edge is symmetric because
/// RDR/WRTV both prove that the local and the value-register slot have the same pointee kind;
/// the existing anti-seed/width-conflict propagation still poisons reused or contradictory slots.
fn indirect_numeric_edges(instrs: &[super::disasm::Instr]) -> Vec<(i32, i32, bool)> {
    instrs
        .windows(3)
        .filter_map(|w| {
            if w[0].op.name != "PshVPtr" || w[1].op.name != "PopRPtr" {
                return None;
            }
            let is8 = match w[2].op.name {
                "RDR4" | "WRTV4" => false,
                "RDR8" | "WRTV8" => true,
                _ => return None,
            };
            let addr = w[0].words.first().map(|v| *v as i16 as i32).unwrap_or(0);
            let value = w[2].words.first().map(|v| *v as i16 as i32).unwrap_or(0);
            Some((addr, value, is8))
        })
        .collect()
}

/// batch-28: unified numeric-kind dataflow (spec §2.3). The VM never converts implicitly —
/// every numeric conversion is an explicit `*TO*` opcode and `SetV*`/`CpyVtoV*`/`CpyRtoV*`/
/// `RDR*`/`WRTV*` are TYPELESS width copies — so a slot's numeric kind is statically
/// decidable from its op profile and float-ness propagates deterministically across the
/// typeless copies. Root cause of the C1/C2/C3/C4c/C4d warning classes: the declaration side
/// was blind to float evidence on op OPERANDS, float call returns through `CpyRtoV*`, float
/// member reads through `RDR8`, and kind propagation across `CpyVtoV*`.
///
/// Seeds:
///  (a) float-op OPERANDS: every word of f-ops -> F32, of d-ops -> F64 (same op lists as
///      `structure::float_operand_slots`);
///  (b) width-changing conversions, BOTH sides (fTOd/dTOf/iTOd/…/dTOi/…); the same-width
///      float<->int casts fTOi/fTOu poison their slots (one offset changing kind
///      mid-lifetime is undecidable — bail to status quo);
///  (c) call results: CpyRtoV4 with last-call by-value ret token 0x50 -> F32, CpyRtoV8 with
///      0x51|0x5E -> F64 (same `last`-tracking discipline as `infer_call_result_types`);
///  (d) member reads/writes: Load*R + RDR4/8 (dst) / WRTV4/8 (src) when the field's VALUE
///      type resolves via the own-class `fields` map or `field_type_by_class` — width-matched
///      (native-struct fields stay unresolved; propagation may still type the slot);
///  (e) by-value float param slots (via `build_param_off_map_rvo`; int-family by-value param
///      offsets are anti-seeds);
///  (f) imports: `float_args`/`outrefs` float keywords, CpyVtoR4/8 float-return payloads
///      (mirrors the infer_locals ret-retype so propagation sees it);
///  (g) I64 seeds (LOWEST priority — any float evidence wins): SetV8 dsts + int64-op operands.
///
/// Anti-seeds (poison on float contact): int arith/bitwise/inc/dec operands, the int sides of
/// conversions, CMPi/CMPu(64)/CMPIi/CMPIu operands, NOT slots, TZ/TNZ-tested register
/// payloads, plus the imported `anti` set (int-family value args / `int&` / `bool&` bindings).
///
/// Propagation to fixed point over CpyVtoV edges and exact local-address RDR/WRTV edges:
/// 8-byte edges carry {F64, I64} (an F32
/// endpoint is a width conflict -> poison both); 4-byte edges carry {F32} (an F64 endpoint ->
/// poison both). A poisoned endpoint poisons its copy partner — the copy pair must agree, or
/// the retype would CREATE a new float->int warning on the copy line (batch-9 class).
///
/// Output: POSITIVE, non-object slots only, conflicts removed. Decompile mode never calls
/// this (emit-only context).
fn infer_float_flow(
    f: &Func,
    fc: &FuncCode,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
    float_args: &HashMap<i32, String>,
    outrefs: &HashMap<i32, String>,
    anti_imports: &HashSet<i32>,
) -> HashMap<i32, NumKind> {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return HashMap::new(),
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32).unwrap_or(0);
    let w1 = |ins: &super::disasm::Instr| ins.words.get(1).map(|w| *w as i16 as i32).unwrap_or(0);
    let mut anti: HashSet<i32> = anti_imports.clone();
    let mut poison: HashSet<i32> = HashSet::new();
    let mut seeds: Vec<(i32, NumKind)> = Vec::new();
    let mut edges = indirect_numeric_edges(&instrs); // (dst, src, is_8_byte)

    // (e) by-value param slots: float tokens seed, int-family tokens anti-seed (params are
    // never declared by us — they matter as propagation sources across CpyVtoV copies).
    let (param_offs, _rvo) = super::decompile::build_param_off_map_rvo(fc, &instrs, refs);
    for (off, idx) in &param_offs {
        let Some(pt) = fc.param_types.get(*idx) else {
            continue;
        };
        if pt.is_reference {
            continue; // the frame offset holds a pointer, not the value
        }
        match pt.token {
            0x50 => seeds.push((*off, NumKind::F32)),
            0x51 | 0x5E => seeds.push((*off, NumKind::F64)),
            0x44 | 0x45 | 0x46 | 0x47 | 0x4B | 0x4C | 0x4D | 0x4E => {
                anti.insert(*off);
            }
            _ => {}
        }
    }
    // (f) imported float evidence from the arg-pairing walk (by-value float args + float
    // out-refs; bool out-refs arrive through `anti_imports`).
    for m in [float_args, outrefs] {
        for (slot, kw) in m {
            match kw.as_str() {
                "float32" => seeds.push((*slot, NumKind::F32)),
                "float" | "double" => seeds.push((*slot, NumKind::F64)),
                _ => {}
            }
        }
    }
    // trackers: just-completed call's by-value return token (survives only SUSPEND, consumed
    // by CpyRtoV*), pending member ref (tid, member-off, base-is-slot0; survives CHKREF-family
    // checks), and the slot behind a CpyVtoR4 for the TZ/TNZ anti-seed.
    let mut last_ret: Option<i32> = None;
    let mut ref_field: Option<(i32, i32, bool)> = None;
    let mut last_vreg4: Option<i32> = None;
    for ins in &instrs {
        let n = ins.op.name;
        // ---- tracker consumers ----
        match n {
            "CpyRtoV4" => {
                if last_ret == Some(0x50) {
                    seeds.push((w0(ins), NumKind::F32));
                }
            }
            "CpyRtoV8" => {
                if matches!(last_ret, Some(0x51) | Some(0x5E)) {
                    seeds.push((w0(ins), NumKind::F64));
                }
            }
            "RDR4" | "RDR8" | "WRTV4" | "WRTV8" => {
                if let Some((tid, moff, own)) = ref_field {
                    let fty = refs.member(tid, moff).and_then(|name| {
                        let own_ty = if own {
                            fields.and_then(|m| m.get(name))
                        } else {
                            None
                        };
                        own_ty.map(String::as_str).or_else(|| {
                            refs.type_by_id(tid)
                                .and_then(|cls| refs.field_type_by_class(cls, name))
                        })
                    });
                    let is8 = n.ends_with('8');
                    match fty.map(|t| t.trim_start_matches("const ")) {
                        Some("float" | "double") if is8 => seeds.push((w0(ins), NumKind::F64)),
                        Some("float32") if !is8 => seeds.push((w0(ins), NumKind::F32)),
                        _ => {} // unknown / width-mismatched field: no seed
                    }
                }
            }
            "TZ" | "TNZ" => {
                if let Some(s) = last_vreg4 {
                    anti.insert(s);
                }
            }
            _ => {}
        }
        // ---- per-op seeds / anti-seeds / copy edges ----
        match n {
            // (a) float-op operand seeds — same op lists as structure::float_operand_slots.
            "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf" | "ADDIf"
            | "SUBIf" | "MULIf" | "CMPf" | "CMPIf" => {
                for &wd in &ins.words {
                    seeds.push((wd as i16 as i32, NumKind::F32));
                }
            }
            "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd" | "CMPd" => {
                for &wd in &ins.words {
                    seeds.push((wd as i16 as i32, NumKind::F64));
                }
            }
            // (b) width-changing conversions, both sides.
            "fTOd" => {
                seeds.push((w0(ins), NumKind::F64));
                seeds.push((w1(ins), NumKind::F32));
            }
            "dTOf" => {
                seeds.push((w0(ins), NumKind::F32));
                seeds.push((w1(ins), NumKind::F64));
            }
            "iTOd" | "uTOd" | "i64TOd" | "u64TOd" => {
                seeds.push((w0(ins), NumKind::F64));
                anti.insert(w1(ins));
            }
            "iTOf" | "uTOf" | "i64TOf" | "u64TOf" => {
                seeds.push((w0(ins), NumKind::F32));
                anti.insert(w1(ins));
            }
            "dTOi" | "dTOu" | "dTOi64" | "dTOu64" => {
                seeds.push((w1(ins), NumKind::F64));
                anti.insert(w0(ins));
            }
            "fTOi64" | "fTOu64" => {
                seeds.push((w1(ins), NumKind::F32));
                anti.insert(w0(ins));
            }
            // same-width float<->int casts: one offset holds both kinds mid-lifetime — bail.
            "fTOi" | "fTOu" => {
                poison.insert(w0(ins));
                poison.insert(w1(ins));
            }
            // int<->int conversions: both sides proven int.
            "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" | "iTOb" | "iTOw" | "i64TOi" | "iTOi64"
            | "uTOi64" => {
                anti.insert(w0(ins));
                anti.insert(w1(ins));
            }
            // anti-seeds: genuine int arithmetic/bitwise/compare evidence.
            "ADDi" | "SUBi" | "MULi" | "DIVi" | "MODi" | "IncVi" | "DecVi" | "NEGi" | "BAND"
            | "BOR" | "BXOR" | "BSLL" | "BSRA" | "BSRL" | "BNOT" | "ADDIi" | "SUBIi" | "MULIi"
            | "CMPi" | "CMPu" | "CMPIi" | "CMPIu" | "NOT" => {
                for &wd in &ins.words {
                    anti.insert(wd as i16 as i32);
                }
            }
            // (g) int64 ops: I64 seeds (lowest priority) AND anti (poison on float contact).
            "ADDi64" | "SUBi64" | "MULi64" | "DIVi64" | "MODi64" | "CMPi64" | "CMPu64" => {
                for &wd in &ins.words {
                    anti.insert(wd as i16 as i32);
                    seeds.push((wd as i16 as i32, NumKind::I64));
                }
            }
            "SetV8" => seeds.push((w0(ins), NumKind::I64)),
            // (f) float return payloads (mirror of the infer_locals ret-retype).
            "CpyVtoR4" => {
                if f.ret.token == 0x50 {
                    seeds.push((w0(ins), NumKind::F32));
                }
            }
            "CpyVtoR8" => {
                if matches!(f.ret.token, 0x51 | 0x5E) {
                    seeds.push((w0(ins), NumKind::F64));
                }
            }
            // typeless slot-to-slot copies: propagation edges.
            "CpyVtoV4" => edges.push((w0(ins), w1(ins), false)),
            "CpyVtoV8" => edges.push((w0(ins), w1(ins), true)),
            _ => {}
        }
        // ---- tracker updates ----
        last_ret = match n {
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                refs.func_ret_by_id(id)
                    .filter(|d| !d.is_reference)
                    .map(|d| d.token)
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                if refs.func_by_ptr(ptr) == Some("$beh0") {
                    None
                } else {
                    refs.func_ret_by_ptr(ptr)
                        .filter(|d| !d.is_reference)
                        .map(|d| d.token)
                }
            }
            "SUSPEND" => last_ret,
            _ => None,
        };
        ref_field = match n {
            "LoadThisR" => Some((
                ins.dwords.first().copied().unwrap_or(0) as i32,
                w0(ins),
                true,
            )),
            "LoadRObjR" | "LoadVObjR" => Some((
                ins.dwords.first().copied().unwrap_or(0) as i32,
                w1(ins),
                false,
            )),
            "CHKREF" | "ChkRefS" | "ChkNullV" | "SUSPEND" => ref_field,
            _ => None,
        };
        last_vreg4 = match n {
            "CpyVtoR4" => Some(w0(ins)),
            "SUSPEND" => last_vreg4,
            _ => None,
        };
    }
    // ---- apply seeds (anti set complete), then propagate to fixed point ----
    let mut kind: HashMap<i32, NumKind> = HashMap::new();
    for (s, k) in seeds {
        if poison.contains(&s) {
            continue;
        }
        nk_apply(&mut kind, &mut poison, &anti, s, k);
    }
    for s in &poison {
        kind.remove(s);
    }
    loop {
        let mut changed = false;
        for &(a, b, is8) in &edges {
            let pa = poison.contains(&a);
            let pb = poison.contains(&b);
            if pa || pb {
                if !(pa && pb) {
                    kind.remove(&a);
                    kind.remove(&b);
                    poison.insert(a);
                    poison.insert(b);
                    changed = true;
                }
                continue;
            }
            let ka = kind.get(&a).copied();
            let kb = kind.get(&b).copied();
            // a kind whose width contradicts the copy width: conflict, poison both ends.
            let bad = if is8 { NumKind::F32 } else { NumKind::F64 };
            if ka == Some(bad) || kb == Some(bad) {
                kind.remove(&a);
                kind.remove(&b);
                poison.insert(a);
                poison.insert(b);
                changed = true;
                continue;
            }
            // domain per edge width: 8-byte edges carry {F64, I64}, 4-byte edges {F32}.
            let dom = |k: Option<NumKind>| match (k, is8) {
                (Some(NumKind::F64) | Some(NumKind::I64), true) => k,
                (Some(NumKind::F32), false) => k,
                _ => None,
            };
            if let Some(k) = dom(ka) {
                changed |= nk_apply(&mut kind, &mut poison, &anti, b, k);
            }
            if let Some(k) = dom(kb) {
                changed |= nk_apply(&mut kind, &mut poison, &anti, a, k);
            }
        }
        if !changed {
            break;
        }
    }
    // POSITIVE local slots only; obj_locals-typed slots are outside the numeric domain.
    let obj: HashSet<i32> = f.obj_locals.iter().map(|(s, _)| *s).collect();
    kind.retain(|s, _| *s > 0 && !obj.contains(s) && !poison.contains(s));
    kind
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
        let Some(rest) = t.strip_prefix("arg") else {
            continue;
        };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(n) = digits.parse::<i32>() else {
            continue;
        };
        if !oor_args.contains(&n) {
            continue;
        }
        let after = rest[digits.len()..].trim_start();
        let Some(rhs) = after.strip_prefix("= ") else {
            continue;
        };
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
///
/// batch-25h: FAngelscriptGameThreadScopeWorldContext — the compiler-inserted RAII
/// world-context scope guard (GetPlayerRelationshipForGlossary rendered a hoisted decl +
/// else-branch ctor-assign; the type has no default ctor/opAssign). Routed through THIS
/// decl-init rewrite (its write-only ctor-assign shape passes the existing gate; the
/// declaration sinks into the assigning block, which is scope-safe because the gate proves
/// the object is never read) rather than dropping the statement: the guard's side effect IS
/// its lifetime — it sets the game-thread world context for every native call in the scope,
/// so deleting it would silently change runtime native-call resolution at OTHER potential
/// sites of the same type. Any future site with reads keeps the hoist (status-quo error,
/// never a wrong rewrite).
fn has_no_default_ctor(ty: &str) -> bool {
    matches!(
        ty,
        "FStatID" | "FScopeCycleCounter" | "FAngelscriptGameThreadScopeWorldContext"
    )
}

/// batch-25i: net brace delta of an emitted body line. The structurer/emitter only produce
/// STRUCTURAL braces as standalone lines (`{ind}{{` / `{ind}}}`), so counting is keyed on the
/// trimmed content being exactly `{`/`}` — statement lines whose string literals contain
/// braces (`"Value {0}"`) trim to longer text and never miscount.
fn brace_net(line: &str) -> i32 {
    match line.trim() {
        "{" => 1,
        "}" => -1,
        _ => 0,
    }
}

/// A hoisted inferred-enum local can keep a bare declaration only if its first body reference is
/// an unconditional, write-only assignment at function scope. This is deliberately a textual
/// proof over the already-structured body: any read, self-reference, branch/loop-local first
/// write, malformed brace nesting, or ambiguous statement fails closed and asks the declaration
/// emitter for an explicit enum-zero initializer.
fn first_top_level_assignment_before_read(body: &str, slot: i32) -> bool {
    let ident = format!("local_{slot}");
    let assignment_prefix = format!("{ident} = ");
    let mut depth = 0i32;

    for line in body.lines() {
        if count_ident(line, &ident) != 0 {
            let trimmed = line.trim();
            return depth == 0
                && trimmed.starts_with(&assignment_prefix)
                && trimmed.ends_with(';')
                && count_ident(line, &ident) == 1;
        }

        depth += brace_net(line);
        if depth < 0 {
            return false;
        }
    }

    false
}

/// Prove definite assignment for a compiler-reused primitive carrier in structured emitted source.
/// Every read must be preceded by a write-only assignment in the same lexical block or an ancestor
/// block. Assignments made inside a child block are discarded when leaving it, so a conditional or
/// sibling write can never justify a later read. This intentionally accepts loop/case-local carrier
/// reuse while rejecting read-before-write, self-assignment, malformed braces, and branch leakage.
fn all_reads_lexically_dominated_by_assignment(body: &str, slot: i32) -> bool {
    let ident = format!("local_{slot}");
    let lines: Vec<&str> = body.lines().collect();
    let mut at = 0usize;
    let mut seen = SlotReads {
        every_read_dominated: true,
        ..SlotReads::default()
    };
    walk_assignment_scope(&lines, &mut at, &ident, false, &mut seen);
    at == lines.len() && seen.every_read_dominated && seen.assignment && seen.read
}

/// What [`all_reads_lexically_dominated_by_assignment`] learned about one slot.
#[derive(Default)]
struct SlotReads {
    assignment: bool,
    read: bool,
    every_read_dominated: bool,
}

/// One scope's effect on the slot: whether every path through it ends with the slot assigned, and
/// whether it always leaves the enclosing block (so the code after it is not on this path).
#[derive(Clone, Copy, Default)]
struct ScopeEffect {
    assigns: bool,
    leaves: bool,
}

/// Walk one scope. An `if`/`else` assigns for the scope around it when each arm either assigns or
/// leaves; a loop never does, because it may not run.
fn walk_assignment_scope(
    lines: &[&str],
    at: &mut usize,
    ident: &str,
    mut assigned: bool,
    seen: &mut SlotReads,
) -> ScopeEffect {
    let mut leaves = false;
    while *at < lines.len() {
        let line = lines[*at];
        let trimmed = line.trim();
        if trimmed == "}" {
            break;
        }
        *at += 1;
        if count_ident(line, ident) > 0 {
            if assignment_rhs_for(line, ident).is_some() {
                assigned = true;
                seen.assignment = true;
            } else {
                seen.read = true;
                if !assigned {
                    seen.every_read_dominated = false;
                }
            }
        }
        leaves |= trimmed.starts_with("return ") || trimmed == "return;";
        if lines.get(*at).map(|l| l.trim()) != Some("{") {
            continue;
        }
        *at += 1;
        let then = walk_assignment_scope(lines, at, ident, assigned, seen);
        *at += 1; // the closing brace
        let mut arms = ScopeEffect::default();
        if lines.get(*at).map(|l| l.trim()) == Some("else") {
            *at += 1;
            if lines.get(*at).map(|l| l.trim()) == Some("{") {
                *at += 1;
                let other = walk_assignment_scope(lines, at, ident, assigned, seen);
                *at += 1; // the closing brace
                arms = ScopeEffect {
                    assigns: (then.assigns || then.leaves) && (other.assigns || other.leaves),
                    leaves: then.leaves && other.leaves,
                };
            }
        } else {
            // A one-armed `if` that always leaves puts the code after it on the fall path only.
            arms.assigns = then.leaves && assigned;
        }
        if trimmed.starts_with("if (") {
            assigned |= arms.assigns;
            leaves |= arms.leaves;
        }
    }
    ScopeEffect {
        assigns: assigned,
        leaves,
    }
}

/// batch-25i: the innermost brace-block span containing line `i`: returns `(start, end)` line
/// indices of the opening/closing brace lines (exclusive bounds for content). When `i` sits at
/// function top level, returns `(0, lines.len())` — the whole body.
fn block_span(lines: &[&str], i: usize) -> (usize, usize) {
    let mut start = 0usize;
    let mut bal = 0i32;
    for j in (0..i).rev() {
        bal += brace_net(lines[j]);
        if bal > 0 {
            start = j;
            break;
        }
    }
    let mut end = lines.len();
    bal = 0;
    for (j, l) in lines.iter().enumerate().skip(i + 1) {
        bal += brace_net(l);
        if bal < 0 {
            end = j;
            break;
        }
    }
    (start, end)
}

/// batch-25i: identifier-boundary rename of `ident` -> `new` in one line (same boundary rule
/// as [`count_ident`]; byte-copies non-matches so UTF-8 string literals survive intact).
fn rename_ident(line: &str, ident: &str, new: &str) -> String {
    let (b, ib) = (line.as_bytes(), ident.as_bytes());
    let is_id = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut out: Vec<u8> = Vec::with_capacity(b.len() + 8);
    let mut i = 0usize;
    while i < b.len() {
        if i + ib.len() <= b.len()
            && &b[i..i + ib.len()] == ib
            && (i == 0 || !is_id(b[i - 1]))
            && (i + ib.len() == b.len() || !is_id(b[i + ib.len()]))
        {
            out.extend_from_slice(new.as_bytes());
            i += ib.len();
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
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

/// Conservative purity predicate used only by the adjacent value-temporary folder. These forms
/// cannot mutate state while a read is moved from the preceding statement into its sole consumer.
fn is_simple_pure_expr(expr: &str) -> bool {
    let expr = expr.trim();
    if expr.is_empty() {
        return false;
    }
    if expr.starts_with('(') && expr.ends_with(')') {
        let mut depth = 0i32;
        let mut wraps_whole = true;
        for (i, b) in expr.bytes().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            if depth < 0 || (depth == 0 && i + 1 != expr.len()) {
                wraps_whole = false;
                break;
            }
        }
        if wraps_whole && depth == 0 {
            return is_simple_pure_expr(&expr[1..expr.len() - 1]);
        }
    }
    if let Some(rest) = expr.strip_prefix('!') {
        return is_simple_pure_expr(rest.trim());
    }
    if expr == "true" || expr == "false" || expr == "nullptr" {
        return true;
    }
    if expr
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'+'))
        && !expr.contains("..")
    {
        return true;
    }
    // Primitive and enum conversion syntax is value-only. Calls on objects / namespaces are not
    // admitted here: even a zero-argument getter may mutate state.
    let Some(open) = expr.find('(') else {
        return false;
    };
    if !expr.ends_with(')') || expr[open + 1..expr.len() - 1].contains(['(', ')', ',']) {
        return false;
    }
    let head = expr[..open].trim();
    let inner = &expr[open + 1..expr.len() - 1];
    (is_primitive(head) || is_enum(head)) && is_simple_pure_expr(inner)
}

fn leading_indent(line: &str) -> &str {
    let trimmed = line.trim_start();
    &line[..line.len() - trimmed.len()]
}

fn assignment_rhs_for<'a>(line: &'a str, ident: &str) -> Option<&'a str> {
    let trimmed = line.trim();
    let prefix = format!("{ident} = ");
    let rhs = trimmed.strip_prefix(&prefix)?.strip_suffix(';')?.trim();
    (!rhs.is_empty() && count_ident(line, ident) == 1).then_some(rhs)
}

fn local_assignment_count(body: &str, slot: i32) -> usize {
    let ident = format!("local_{slot}");
    body.lines()
        .filter(|line| assignment_rhs_for(line, &ident).is_some())
        .count()
}

fn adjacent_value_slot_is_candidate(
    body: &str,
    slot: i32,
    typed_state: bool,
    primitive_scratch: bool,
) -> bool {
    typed_state || (primitive_scratch && local_assignment_count(body, slot) <= 1)
}

fn is_zero_arg_member_chain(suffix: &str) -> bool {
    let mut rest = suffix;
    while !rest.is_empty() {
        let Some(after_dot) = rest.strip_prefix('.') else {
            return false;
        };
        let Some(paren) = after_dot.find("()") else {
            return false;
        };
        let name = &after_dot[..paren];
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return false;
        }
        rest = &after_dot[paren + 2..];
    }
    true
}

/// Replace the sole use of an immediately preceding local assignment. The accepted consumers are
/// intentionally tiny and evaluation-order explicit: whole-value assignment/return/control tests;
/// a comparison against one pure operand; unary bool inversion; or a pure value inserted as the
/// sole argument of one member call followed only by zero-argument member calls.
fn fold_adjacent_consumer(line: &str, ident: &str, rhs: &str) -> Option<String> {
    let indent = leading_indent(line);
    let trimmed = line.trim();

    if trimmed == format!("return {ident};") {
        return Some(format!("{indent}return {rhs};"));
    }
    for keyword in ["if", "while", "switch"] {
        if trimmed == format!("{keyword} ({ident})") {
            return Some(format!("{indent}{keyword} ({rhs})"));
        }
    }
    if trimmed == format!("{ident} = !{ident};") && is_simple_pure_expr(rhs) {
        return Some(format!("{indent}{ident} = !({rhs});"));
    }

    // `receiver.Method(local_N);` — a whole call statement whose sole argument is the folded
    // value. The receiver is a pure member path, so running it before the producer instead of
    // after cannot observe a different value, and no sibling argument can reorder.
    if let Some(call) = trimmed.strip_suffix(';') {
        let marker = format!("({ident})");
        if let Some((prefix, suffix)) = call.split_once(&marker) {
            if suffix.is_empty()
                && count_ident(call, ident) == 1
                && !prefix.is_empty()
                && prefix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
            {
                return Some(format!("{indent}{prefix}({rhs});"));
            }
        }
    }

    if let Some(rest) = trimmed
        .strip_prefix("if (")
        .and_then(|s| s.strip_suffix(')'))
    {
        for op in [" <= ", " >= ", " == ", " != ", " < ", " > "] {
            let Some((left, right)) = rest.split_once(op) else {
                continue;
            };
            // An impure producer may only remain on the LEFT: the original producer statement
            // ran before both comparison operands, and left-to-right evaluation preserves that.
            // Moving it to the right of even a non-mutating field read could observe a different
            // pre-mutation field value. A pure producer is safe on either side.
            let preserves_order = (left.trim() == ident && is_simple_pure_expr(right))
                || (right.trim() == ident && is_simple_pure_expr(left) && is_simple_pure_expr(rhs));
            if preserves_order && count_ident(rest, ident) == 1 {
                let replaced = rename_ident(rest, ident, &format!("({rhs})"));
                return Some(format!("{indent}if ({replaced})"));
            }
        }
    }

    let statement = trimmed.strip_suffix(';')?;
    let (lhs, consumer_rhs) = statement.split_once(" = ")?;
    if count_ident(lhs, ident) != 0 {
        return None;
    }
    if consumer_rhs.trim() == ident {
        return Some(format!("{indent}{lhs} = {rhs};"));
    }
    if !is_simple_pure_expr(rhs) || count_ident(consumer_rhs, ident) != 1 {
        return None;
    }

    // `receiver.Member(local_N).ZeroArgCall()` only: the receiver path is pure, the folded value
    // is the call's sole argument, and no sibling argument can reorder evaluation.
    let marker = format!("({ident})");
    let (prefix, suffix) = consumer_rhs.split_once(&marker)?;
    if prefix.is_empty()
        || !prefix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
        || !prefix.contains('.')
        || !is_zero_arg_member_chain(suffix)
    {
        return None;
    }
    let replaced = consumer_rhs.replacen(&marker, &format!("({rhs})"), 1);
    Some(format!("{indent}{lhs} = {replaced};"))
}

fn try_eliminate_adjacent_value_slot(body: &str, slot: i32) -> Option<String> {
    let ident = format!("local_{slot}");
    if !body.lines().any(|line| count_ident(line, &ident) != 0) {
        return None;
    }
    let trailing_newline = body.ends_with('\n');
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0usize;
        while i + 1 < lines.len() {
            let Some(rhs) = assignment_rhs_for(&lines[i], &ident).map(str::to_owned) else {
                i += 1;
                continue;
            };
            if leading_indent(&lines[i]) != leading_indent(&lines[i + 1]) {
                i += 1;
                continue;
            }
            let Some(folded) = fold_adjacent_consumer(&lines[i + 1], &ident, &rhs) else {
                i += 1;
                continue;
            };
            lines[i + 1] = folded;
            lines.remove(i);
            changed = true;
            i = i.saturating_sub(1);
        }
    }
    if lines.iter().any(|line| count_ident(line, &ident) != 0) {
        return None;
    }
    let mut out = lines.join("\n");
    if trailing_newline {
        out.push('\n');
    }
    Some(out)
}

/// Fixed-point whole-slot elimination. A candidate is committed only when every reference to it
/// disappears; otherwise its trial rewrite is discarded atomically. Descending slot order plus the
/// fixed point lets an inner temporary disappear before its producer becomes adjacent to a second
/// temporary, without relying on target-specific slot numbers.
fn rewrite_adjacent_value_temporaries(
    body: &str,
    candidates: &HashSet<i32>,
) -> (String, HashSet<i32>) {
    let mut ordered: Vec<i32> = candidates.iter().copied().collect();
    ordered.sort_unstable_by(|a, b| b.cmp(a));
    let mut out = body.to_owned();
    let mut eliminated = HashSet::new();
    loop {
        let mut progress = false;
        for &slot in &ordered {
            if eliminated.contains(&slot) {
                continue;
            }
            if let Some(next) = try_eliminate_adjacent_value_slot(&out, slot) {
                out = next;
                eliminated.insert(slot);
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    (out, eliminated)
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
/// - SINGLE assignment + reads, ALL reads inside the assignment's innermost brace block and
///   after it (batch-25i: was "same indentation", which wrongly accepted equal-depth lines in
///   DIFFERENT blocks — e.g. two braced switch cases — leaving later references undeclared
///   once the decl-init became block-scoped): decl-init in place, name kept — later reads are
///   lvalue uses (a non-const ref binds to an lvalue; only temporaries fail), and block-span
///   containment means the declaration dominates every read.
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
        let lines: Vec<&str> = out.lines().collect();
        let mut assigns = 0usize;
        let mut reads = 0usize;
        let mut first_is_assign = false;
        let mut first = true;
        let mut assign_line: Option<usize> = None; // line index of the (first) assignment
        let mut read_lines: Vec<usize> = Vec::new();
        let mut ok = true;
        for (i, line) in lines.iter().enumerate() {
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
                assign_line.get_or_insert(i);
            } else {
                reads += c;
                read_lines.push(i);
                if c > 1 {
                    ok = false; // multi-occurrence non-assign line (self-referential etc.) — bail
                    break;
                }
            }
        }
        if !ok || assigns == 0 || !first_is_assign {
            continue;
        }
        let write_only = reads == 0;
        // batch-25i scope gate for the single-assign+reads shape: every read must sit INSIDE
        // the assignment's innermost brace block (and after it — guaranteed by first_is_assign
        // plus line order), so the block-scoped decl-init dominates every read.
        let in_block = || {
            let Some(a) = assign_line else { return false };
            let (_, end) = block_span(&lines, a);
            read_lines.iter().all(|&r| r > a && r < end)
        };
        if !write_only && !(assigns == 1 && in_block()) {
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
                // Nothing reads the slot: vanilla just made the call and destroyed the result
                // where it stood. Declaring a local for it costs the declaration AND sinks the
                // destructor to the end of the function.
                let discarded = write_only
                    .then(|| rest.strip_prefix(" = ").and_then(|v| v.strip_suffix(';')))
                    .flatten()
                    .filter(|value| value.ends_with(')'));
                if let Some(value) = discarded {
                    let _ = writeln!(rewritten, "{indent}{value};");
                } else if k == 1 {
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

/// A local whose every occurrence is `local_N = <call>;` is nothing but a name for the call's
/// discarded result. Vanilla made the call and destroyed the result where it stood; naming it
/// costs the declaration and, for a value type, sinks the destructor to the end of the function.
/// Write the call as the statement it is — the same rule as the `no_assign_type` write-only arm,
/// for every other type (`FName`, `const UStoryG1R`, …).
fn drop_unread_call_results(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
) -> (String, HashSet<i32>) {
    let mut dropped: HashSet<i32> = HashSet::new();
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    for slot in locals.keys() {
        let ident = format!("local_{slot}");
        let mut sites: Vec<usize> = Vec::new();
        let mut every_use_is_a_discard = true;
        for (at, line) in lines.iter().enumerate() {
            if count_ident(line, &ident) == 0 {
                continue;
            }
            match discardable_call(line, &ident, refs) {
                Some(_) => sites.push(at),
                None => {
                    every_use_is_a_discard = false;
                    break;
                }
            }
        }
        if !every_use_is_a_discard || sites.is_empty() {
            continue;
        }
        for at in sites {
            let indent: String = lines[at].chars().take_while(|c| c.is_whitespace()).collect();
            let call = discardable_call(&lines[at], &ident, refs).expect("matched above");
            lines[at] = format!("{indent}{call};");
        }
        dropped.insert(*slot);
    }
    let mut joined = lines.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    (joined, dropped)
}

/// The name of the call a value ENDS in. `A(x).B()` is a call to `B`, and reading the first name
/// instead asks about the wrong function entirely.
fn outer_callee(value: &str) -> Option<String> {
    let inner = value.strip_suffix(')')?;
    let mut depth = 1usize;
    for (at, c) in inner.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    return Some(inner[..at].rsplit(['.', ':']).next()?.to_owned());
                }
            }
            _ => {}
        }
    }
    None
}

/// The call a statement assigns to `ident` and nothing else, when that call's result may be
/// thrown away at all. Two shapes may not: CONSTRUCTING a value and dropping it is "Result of
/// expression is unused", and a handful of bound functions are `nodiscard` — neither the script
/// cache nor `Binds.Cache` records that flag, so the names come from what the compiler reported
/// (1,207 and 265 errors respectively, measured on this corpus).
fn discardable_call(line: &str, ident: &str, refs: &RefResolver) -> Option<String> {
    // What the cache cannot say and the compiler did: `nodiscard` is a property of the C++
    // binding and appears in neither the script cache nor `Binds.Cache`, and one const method
    // (`GetCurrentCombo`) the function table records without its const flag. Every name here was
    // reported by the compiler on this corpus, not guessed.
    const REFUSED_BY_THE_COMPILER: [&str; 8] = [
        "FScopeCycleCounter",
        "RandRange",
        "FString",
        "FName",
        "FPlane",
        "Abs",
        "Sqrt",
        "GetCurrentCombo",
    ];
    let statement = line.trim().strip_suffix(';')?;
    let (head, value) = statement.split_once(" = ")?;
    let declares = head.split_whitespace().last()? == ident;
    if !declares || count_ident(value, ident) != 0 || !value.ends_with(')') || !value.contains('(')
    {
        return None;
    }
    let callee = outer_callee(value)?;
    let callee = callee.as_str();
    // A parenthesized expression has no callee at all, and a CONST method has no side effect to
    // keep — the compiler refuses to throw either result away.
    if callee.is_empty()
        || !callee.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || refs.names_a_const_method(callee)
    {
        return None;
    }
    let constructs = callee
        .split('<')
        .next()
        .and_then(|name| {
            let bytes = name.as_bytes();
            (bytes.len() >= 2).then(|| {
                matches!(bytes[0], b'F' | b'U' | b'A' | b'E' | b'T') && bytes[1].is_ascii_uppercase()
            })
        })
        .unwrap_or(false);
    (!constructs && !REFUSED_BY_THE_COMPILER.contains(&callee)).then(|| value.to_owned())
}

/// Residual pass behind [`rewrite_no_assign_locals`]: any REMAINING `local_N = <call>;` (or
/// `__return = <call>;`) statement whose LHS is a no-assign type still assigns a temporary into
/// the non-const `opAssign(TY&)` and fails in-game. Split the statement in place:
///
/// ```text
/// FAbilityTaskExecutor __na_tK = <call>;   // copy-construction from the temporary — legal
/// local_N = __na_tK;                       // opAssign from an LVALUE — legal (capture-proven)
/// ```
///
/// The temp is declared and consumed on adjacent lines of the same block, so no scope/dominance
/// analysis is needed (unlike the decl-init rewrites above, which rename or sink declarations).
/// Only call-like RHS (ends in `)`) is split; a bare lvalue RHS (`__return = local_16;`) already
/// compiles. Lines rewritten by the decl-init passes start with the type name, not `local_N =`,
/// so the two passes never overlap.
fn rewrite_no_assign_residual_assigns(
    body: &str,
    locals: &BTreeMap<i32, String>,
    ret_ty: &str,
) -> String {
    // LHS ident → its declared type, for every no-assign-typed assignable name in this body.
    let is_candidate = |ident: &str| -> Option<&str> {
        if ident == "__return" {
            return no_assign_type(ret_ty).then_some(ret_ty);
        }
        let slot: i32 = ident.strip_prefix("local_")?.parse().ok()?;
        let ty = locals.get(&slot)?;
        no_assign_type(ty).then_some(ty.as_str())
    };
    let mut k = 0usize;
    let mut out = String::with_capacity(body.len() + 64);
    for line in body.lines() {
        let t = line.trim_start();
        let split = t
            .split_once(" = ")
            .and_then(|(lhs, rhs)| Some((is_candidate(lhs)?, lhs, rhs)))
            .filter(|(_, _, rhs)| rhs.ends_with(");"));
        match split {
            Some((ty, lhs, rhs)) => {
                k += 1;
                let indent = &line[..line.len() - t.len()];
                let rhs = &rhs[..rhs.len() - 1]; // strip trailing `;`
                let _ = writeln!(out, "{indent}{ty} __na_t{k} = {rhs};");
                let _ = writeln!(out, "{indent}{lhs} = __na_t{k};");
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// Iterator locals (`TArrayIterator<T>`, `TSetIterator<T>`, `TMapIterator<T>`, ... incl. Const
/// variants) have NO default constructor, so a bare hoisted `TArrayIterator<T> local_N;` fails
/// "No default constructor". The only legal form is declaration-with-initializer from the
/// `Iterator()` call that produces it: `auto local_N = container.Iterator();`. Unlike the
/// ctor-only (FStatID) case an iterator IS read afterwards (in its loop).
///
/// batch-25i (scope-aware rework): batch-24d braces every switch case body, so an in-place
/// decl-init inside a braced case is BLOCK-SCOPED — the old first-mention-only rewrite left
/// any reference in a LATER case/block undeclared (APuzzleStoneTorch_Manager::InitPuzzleState:
/// case 0/1 declared `auto local_8`, case 3 still referenced `local_8` -> "'local_8' is not
/// declared", the single batch-24 regression). References are now grouped by the innermost
/// brace block of each assignment: every group must START with a `local_N = ...;` assignment
/// and contain only references inside that assignment's block span. Group 1 keeps the name;
/// each later group decl-inits a FRESH name (`local_N_2`, ...) and renames its in-block
/// references — the compiler-reused slot becomes one source-level local per block, all
/// initialized (the original per-case iterators). ANY reference that doesn't fit this shape
/// (read before assign, a bare read outside every assign's block) keeps the hoisted
/// declaration — the status-quo error, never a broken reference.
/// A value-struct temporary the source never named: the compiler put one call result in a slot
/// and consumed it in the very next statement. Naming it forces a copy — `$beh0(const T&)` or
/// `opAssign` — that the base cache has no row for, and the module stops being splicable. Fold
/// every producer into its consumer; `rewrite_adjacent_value_temporaries` commits a slot only
/// when every reference to it disappears, so a partial fold is impossible.
fn rewrite_value_temporaries(body: &str, locals: &BTreeMap<i32, String>) -> (String, HashSet<i32>) {
    let candidates: HashSet<i32> = locals
        .iter()
        .filter(|(_, ty)| is_value_struct_type(ty))
        .map(|(slot, _)| *slot)
        .filter(|slot| produced_only_by_calls(body, *slot))
        .collect();
    if candidates.is_empty() {
        return (body.to_owned(), HashSet::new());
    }
    rewrite_adjacent_value_temporaries(body, &candidates)
}

/// `Cast<T>(x)` lowers to a null-guarded diamond: the compiler tests the source, casts inside the
/// branch and leaves the destination null otherwise. Written back as that diamond, a class
/// default cannot be authored at all (a `default` statement carries an expression, not a block),
/// and every ordinary body carries seven lines where the source had one. `Cast<T>(nullptr)` is
/// itself null, so folding the diamond back into the cast is exactly what the compiler undid.
fn fold_cast_diamonds(body: &str) -> String {
    if !body.contains("Cast<") {
        return body.to_owned();
    }
    let trailing_newline = body.ends_with('\n');
    let lines: Vec<&str> = body.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut at = 0usize;
    while at < lines.len() {
        match cast_diamond(&lines[at..]) {
            Some((consumed, folded)) => {
                out.push(folded);
                at += consumed;
            }
            None => {
                out.push(lines[at].to_owned());
                at += 1;
            }
        }
    }
    let mut joined = out.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// Matches the seven-line (or eight, with an explicit null store) diamond at the head of `lines`
/// and returns how many lines it spans plus the single statement that replaces them.
fn cast_diamond(lines: &[&str]) -> Option<(usize, String)> {
    if lines.len() < 7 {
        return None;
    }
    let indent = leading_indent(lines[0]);
    let guard = lines[0]
        .trim()
        .strip_prefix("if (")?
        .strip_suffix(" != nullptr)")?;
    if lines[1].trim() != "{" || lines[3].trim() != "}" || lines[4].trim() != "else" {
        return None;
    }
    if lines[5].trim() != "{" {
        return None;
    }
    let assignment = lines[2].trim().strip_suffix(';')?;
    let (destination, cast) = assignment.split_once(" = ")?;
    if !cast.starts_with("Cast<") || !cast.ends_with(&format!("({guard})")) {
        return None;
    }
    // The else branch must leave the destination null — either by saying nothing (a bare object
    // declaration is already null) or by storing null explicitly.
    let (span, closing) = match lines[6].trim() {
        "}" => (7, 6),
        _ if lines.len() > 7
            && lines[6].trim() == format!("{destination} = nullptr;")
            && lines[7].trim() == "}" =>
        {
            (8, 7)
        }
        _ => return None,
    };
    if lines[closing].trim() != "}" {
        return None;
    }
    Some((span, format!("{indent}{assignment};")))
}

/// AngelScript's binary operators are methods, and the structurer recovers the method: the AI
/// rule tables come back as `FAssessmentBits(A).opOr(B)` where the source wrote `A | B`. The two
/// compile to the same code, but only the operator form is a shape a class-scope `default`
/// statement can carry. Rewrite the call back into its operator.
fn rewrite_operator_calls(body: &str) -> String {
    const OPERATORS: &[(&str, &str)] = &[
        (".opOr(", "|"),
        (".opAnd(", "&"),
        (".opXor(", "^"),
        (".opShl(", "<<"),
        (".opShr(", ">>"),
        (".opMod(", "%"),
        // Not an infix operator: `opIndex` is subscript, rendered `recv[key]`.
        (".opIndex(", "[]"),
    ];
    if !OPERATORS.iter().any(|(call, _)| body.contains(call)) {
        return body.to_owned();
    }
    let trailing_newline = body.ends_with('\n');
    let mut out: Vec<String> = Vec::new();
    for line in body.lines() {
        let mut line = line.to_owned();
        let mut cursor = 0usize;
        // Bound: each pass either rewrites one call (shortening the line) or advances the cursor.
        while cursor < line.len() {
            let Some((at, call, op)) = OPERATORS
                .iter()
                .filter_map(|(call, op)| {
                    line[cursor..].find(call).map(|i| (cursor + i, *call, *op))
                })
                .min_by_key(|(at, _, _)| *at)
            else {
                break;
            };
            let open = at + call.len() - 1;
            match (matching_paren(&line, open), expression_start(&line, at)) {
                (Some(close), Some(start)) => {
                    let folded = if op == "[]" {
                        format!("{}[{}]", &line[start..at], &line[open + 1..close])
                    } else {
                        format!("({} {op} {})", &line[start..at], &line[open + 1..close])
                    };
                    line = format!("{}{folded}{}", &line[..start], &line[close + 1..]);
                    cursor = start;
                }
                _ => cursor = open + 1,
            }
        }
        out.push(line);
    }
    let mut joined = out.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// Index of the `)` closing the `(` at `open`, skipping string literals.
fn matching_paren(line: &str, open: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        if in_string {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Start of the expression that ends at `end` — the receiver of the operator call. Walks back
/// over one balanced call/index chain; a quote inside it means the receiver holds a string
/// literal, where walking backwards cannot tell an opening quote from a closing one, so the
/// rewrite is abandoned for that call.
fn expression_start(line: &str, end: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = end;
    let mut depth = 0i32;
    while i > 0 {
        let b = bytes[i - 1];
        match b {
            b'"' => return None,
            b')' | b']' => depth += 1,
            b'(' | b'[' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            _ if depth > 0 => {}
            _ if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':') => {}
            _ => break,
        }
        i -= 1;
    }
    (i < end).then_some(i)
}

/// A slot that holds one literal and is read once is not a variable the source declared — the
/// compiler put the constant straight into the expression, and writing it back as a local costs
/// a wider store plus a narrowing conversion (`SetV4` + `iTOb` where vanilla has `SetV1`). Fold
/// the literal into its use. A literal has no side effect and no evaluation order, so the only
/// thing this can change is which slot the recompiler allocates.
fn fold_literal_temporaries(body: &str, refs: &RefResolver) -> String {
    let trailing_newline = body.ends_with('\n');
    let lines: Vec<&str> = body.lines().collect();
    let mut folded: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    let mut dropped = false;
    for slot in used_locals(body) {
        let ident = format!("local_{slot}");
        // Definitions in order, and the region each one owns: up to the next definition of the
        // same slot. The compiler re-uses one slot for several constants, so a slot with three
        // literal stores is three source constants, not one variable.
        let definitions: Vec<usize> = folded
            .iter()
            .enumerate()
            .filter(|(_, line)| assignment_rhs_for(line, &ident).is_some_and(is_foldable_literal))
            .map(|(index, _)| index)
            .collect();
        if definitions.is_empty() {
            continue;
        }
        let all_definitions: Vec<usize> = folded
            .iter()
            .enumerate()
            .filter(|(_, line)| assignment_rhs_for(line, &ident).is_some())
            .map(|(index, _)| index)
            .collect();
        for (order, definition) in definitions.iter().copied().enumerate() {
            let _ = order;
            let region_end = all_definitions
                .iter()
                .copied()
                .find(|other| *other > definition)
                .unwrap_or(folded.len());
            let uses: Vec<usize> = (definition + 1..region_end)
                .filter(|index| count_ident(&folded[*index], &ident) > 0)
                .collect();
            let [use_line] = uses[..] else {
                continue;
            };
            if count_ident(&folded[use_line], &ident) != 1
                || assignment_target_is_rooted_at_ident(&folded[use_line], &ident)
                || !sole_use_is_a_conversion(&folded[use_line], &ident, refs)
            {
                continue;
            }
            let literal = assignment_rhs_for(&folded[definition], &ident)
                .expect("the definition matched above")
                .to_owned();
            folded[use_line] = rename_ident(&folded[use_line], &ident, &literal);
            folded[definition].clear();
            dropped = true;
        }
    }
    if !dropped {
        return body.to_owned();
    }
    let mut joined = folded
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// True when the ident's only appearance is somewhere a literal is certainly legal: the whole
/// argument of a TYPE conversion (`ERelationship(local_5)`), or an operand of a comparison
/// against a literal (`(local_5 != 0)`). Anything else may be a by-reference parameter, where a
/// literal is "Not a valid reference", and parameter types are not visible in the rendered text.
fn sole_use_is_a_conversion(line: &str, ident: &str, refs: &RefResolver) -> bool {
    for operator in [" != ", " == ", " < ", " > ", " <= ", " >= "] {
        for pattern in [format!("({ident}{operator}"), format!("{operator}{ident})")] {
            if line.contains(&pattern) {
                return true;
            }
        }
    }
    // A literal is legal wherever a TEMPORARY is, and the cache says which parameter positions
    // those are — the same witness the producer inlining asks. That is the position this fold
    // used to refuse for want of one, and it is where most of these constants go.
    if let Some((callee, arguments)) = call_arguments(line) {
        let rendered = arguments.len();
        if arguments.iter().enumerate().any(|(position, argument)| {
            argument == ident && refs.arg_position_accepts_temporary(&callee, rendered, position)
        }) {
            return true;
        }
    }
    let marker = format!("({ident})");
    let Some(at) = line.find(&marker) else {
        return false;
    };
    let head: String = line[..at]
        .chars()
        .rev()
        .take_while(|c| {
            c.is_ascii_alphanumeric() || *c == '_' || *c == ':' || *c == '<' || *c == '>'
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let head = head.trim_start_matches(':');
    !head.is_empty() && (is_primitive(head) || is_enum(head) || refs.is_type_name(head))
}

/// The compiler evaluates an argument expression where the argument sits; the structurer has to
/// turn it into a statement first, which puts the evaluation BEFORE the whole call's other
/// pushes. Written back that way it recompiles into a different instruction order than vanilla's
/// — the single largest remaining class of divergence. Inline a temporary that only exists to
/// carry one argument back into the call.
///
/// Gates, all of them necessary: the temporary is defined once by a CALL and read once; the read
/// is a whole argument of a later call in the same block; the parameter at that position accepts
/// a temporary (from the cache's own parameter table); and every statement in between is itself
/// a temporary the SAME call consumes, so nothing else changes its order.
fn inline_call_argument_temporaries(
    body: &str,
    refs: &RefResolver,
    locals: &BTreeMap<i32, String>,
    fields: Option<&HashMap<String, String>>,
    operands_move: bool,
    call_types: &HashMap<i32, String>,
    statement_producers: &HashSet<i32>,
    temporary_receivers: &HashSet<i32>,
    loop_elements: &HashSet<i32>,
    widened: &HashSet<i32>,
    aliased: &HashSet<i32>,
) -> String {
    const MAX_ARGUMENT_INLINE_PASSES: usize = 8;
    let trailing_newline = body.ends_with('\n');
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = false;
    // Work backwards: the last argument's temporary is the innermost, and inlining it first
    // keeps the earlier ones' positions valid.
    // Repeat until nothing moves: inlining one argument can reveal that the statement above it
    // belongs to the same call after all.
    for _ in 0..MAX_ARGUMENT_INLINE_PASSES {
        let mut moved = false;
        for index in (0..lines.len()).rev() {
            // Every temporary this statement can take back: one per call argument, plus the
            // receiver its own call runs on.
            let called = call_arguments(&lines[index]);
            let mut candidates: Vec<(String, Position)> = Vec::new();
            // A temporary this statement refuses in one position must not come back through
            // another: the operand sweep below sees the same name and knows less about it.
            let mut refused: Vec<String> = Vec::new();
            for (callee, arguments) in &call_sites(&lines[index]) {
                for (position, argument) in arguments.iter().enumerate() {
                    let Some(temp) = argument_temporary(argument) else {
                        continue;
                    };
                    // A member chain reads the temporary and passes ITS OWN result, so what the
                    // parameter receives does not change — only a bare temporary has to be
                    // checked against the parameter.
                    if *argument == temp
                        && !refs.arg_position_accepts_temporary(callee, arguments.len(), position)
                    {
                        inline_reject("param", callee, &temp, &lines[index]);
                        refused.push(temp);
                        continue;
                    }
                    candidates.push((temp, Position::Argument));
                }
            }
            candidates.extend(
                receiver_temporary(&lines[index]).map(|temp| (temp, Position::Receiver)),
            );
            // A temporary is not only a call's argument or receiver: it is any operand the
            // statement reads (`if (local_14 >= local_10)`). Those move only with type evidence
            // — see the gate in `inline_temporary_into`.
            let operands: Vec<String> = statement_temporaries(&lines[index])
                .into_iter()
                .filter(|_| operands_move)
                .filter(|temp| {
                    !refused.contains(temp) && !candidates.iter().any(|(known, _)| known == temp)
                })
                .collect();
            candidates.extend(operands.into_iter().map(|temp| (temp, Position::Operand)));
            let callee = called
                .map(|(callee, _)| callee)
                .unwrap_or_else(|| "<receiver>".to_owned());
            for (temp, position) in candidates {
                if inline_temporary_into(
                    &mut lines, index, &temp, &callee, position, locals, refs, fields, call_types,
                    statement_producers, temporary_receivers, loop_elements, widened, aliased,
                ) {
                    changed = true;
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
    if !changed {
        return body.to_owned();
    }
    let mut joined = lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// Move the producer of `temp` into the statement at `index`, when that statement is the only
/// reader of that value and nothing in between would have its order changed.
fn inline_temporary_into(
    lines: &mut [String],
    index: usize,
    temp: &str,
    callee: &str,
    position: Position,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
    call_types: &HashMap<i32, String>,
    statement_producers: &HashSet<i32>,
    temporary_receivers: &HashSet<i32>,
    loop_elements: &HashSet<i32>,
    widened: &HashSet<i32>,
    aliased: &HashSet<i32>,
) -> bool {
    let receiver = position == Position::Receiver;
    // The element of a range-for lives for one iteration and the compiler releases it at the end
    // of each; moving it into its reader takes away what the range-for fold matches on.
    if temp
        .strip_prefix("local_")
        .and_then(|slot| slot.parse::<i32>().ok())
        .is_some_and(|slot| {
            loop_elements.contains(&slot) || widened.contains(&slot) || aliased.contains(&slot)
        })
    {
        inline_reject("named-by-vanilla", callee, temp, &lines[index]);
        return false;
    }
    // Where the bytecode PROVES the source evaluated this producer before it pushed the call's
    // other arguments, it was a statement of its own, and moving it into the call would reorder
    // the evaluation. Absence of that proof is not proof of the opposite: a slot the witness
    // says nothing about keeps the gates below.
    if temp
        .strip_prefix("local_")
        .and_then(|slot| slot.parse::<i32>().ok())
        .is_some_and(|slot| statement_producers.contains(&slot))
    {
        inline_reject("order", callee, &temp, &lines[index]);
        return false;
    }
    // The slot's declaration converts whenever the captured call's return type is not the type
    // the slot was declared with. Where they agree, the read is plain and the value may take its
    // place anywhere the slot could stand.
    let reads_plain = || {
        let slot = temp.strip_prefix("local_").and_then(|n| n.parse::<i32>().ok());
        let (Some(slot), Some(declared)) = (slot, temporary_type(locals, temp)) else {
            return false;
        };
        // Only a SCALAR carries this witness: a struct in arithmetic goes through operator
        // overloads whose result type the declaration is still what fixes ("No conversion from
        // 'FVector' to math type available").
        is_primitive(declared)
            && call_types
                .get(&slot)
                .is_some_and(|captured| same_scalar_type(captured, declared))
    };
    // A value struct reached through a call IS a temporary, and AngelScript refuses a NON-const
    // method on one ("Cannot call non-const method on a temporary object"). An object handle is
    // always safe; a value struct only when the cache records a const overload of the method it
    // runs. The body has no declarations yet at this point, so the type comes from the slot
    // table.
    if receiver {
        let ty = temporary_type(locals, temp);
        let const_method = || {
            let (ty, method) = ty.zip(receiver_method(&lines[index], temp))?;
            Some(refs.has_const_overload(
                &super::structure::bare_type_name(ty),
                &method,
            ))
        };
        // Where VANILLA destroyed this slot mid-expression it was itself calling on a temporary
        // at that site, which settles the question the const table cannot answer for a native
        // value type.
        let vanilla_used_a_temporary = temp
            .strip_prefix("local_")
            .and_then(|slot| slot.parse::<i32>().ok())
            .is_some_and(|slot| temporary_receivers.contains(&slot));
        if !ty.is_some_and(is_object_handle_type)
            && const_method() != Some(true)
            && !vanilla_used_a_temporary
        {
            inline_reject("receiver", callee, &temp, &lines[index]);
            return false;
        }
    }
    if count_ident(&lines[index], temp) != 1 {
        inline_reject("uses", callee, &temp, &lines[index]);
        return false; // inlining a value read twice would evaluate it twice
    }
    // The compiler re-uses one slot for several temporaries, so the definition that matters is
    // the LAST one before this call, and it owns the lines up to the next definition of the
    // same slot.
    let Some(definition) = (0..index)
        .rev()
        .find(|line| definition_value(&lines[*line], temp).is_some())
    else {
        inline_reject("definitions", callee, &temp, &lines[index]);
        return false;
    };
    let region_end = (definition + 1..lines.len())
        .find(|line| definition_value(&lines[*line], temp).is_some())
        .unwrap_or(lines.len());
    let uses_in_region: Vec<usize> = (definition + 1..region_end)
        .filter(|line| count_ident(&lines[*line], temp) > 0)
        .collect();
    if uses_in_region != [index] {
        inline_reject("uses", callee, &temp, &lines[index]);
        return false; // this call has to be the only reader of that definition
    }
    let value = definition_value(&lines[definition], temp)
        .expect("the definition matched above")
        .to_owned();
    // An argument or a receiver takes the value as it is: whatever conversion the call needs is
    // written at the call. Any OTHER operand had the conversion in the slot's declaration, so it
    // moves only where the read has provably the same type — a member of `this` the class field
    // map types exactly like the slot.
    let movable = match position {
        Position::Operand => {
            (operand_read_takes_a_value(&lines[index])
                || (reads_plain() && assigns_a_scalar(&lines[index], locals))
                // A `return` is refused because a returned REFERENCE may not be a temporary.
                // A bool is not one, and the cache can say so.
                || (lines[index].trim().starts_with("return ")
                    && renders_a_bool(&value, locals, refs, fields)))
                && (same_typed_own_field(&value, temp, locals, fields)
                    || is_call_result(&value)
                    || names_a_static_class(&value, temp, locals))
        }
        // An argument or a receiver takes the value as it is, so a member read may travel there
        // too — reading a member has no side effect of its own, and the field map proves the
        // slot's declaration was not also converting it. A value that renders as a BOOL may
        // travel as well: the parameter row already said the position takes a value, and a bool
        // is the one type the slot's declaration cannot have been converting on the way in.
        _ => {
            is_call_result(&value)
                || same_typed_own_field(&value, temp, locals, fields)
                || names_a_static_class(&value, temp, locals)
                || (temporary_type(locals, temp) == Some("bool")
                    && renders_a_bool(&value, locals, refs, fields))
        }
    };
    if !movable {
        inline_reject("not-a-call", callee, &temp, &lines[index]);
        return false;
    }
    // The rendered read compares the slot against zero, which is how an int-typed slot spells a
    // bool test (`(local_3 != 0)`, `local_8.HasTag(..) == 0`). Comparing a BOOL to an int does
    // not compile, so only a value that says int outright may take that slot's place.
    if !value.starts_with("int(")
        && !reads_plain()
        && [" != 0", " == 0"]
            .iter()
            .any(|test| lines[index].contains(&format!("{temp}{test}")))
    {
        inline_reject("wrapped", callee, &temp, &lines[index]);
        return false;
    }
    // Everything between has to feed THIS call as well, or the order of a side effect would
    // change. "Feeds this call" is checked against the line as it stands, so a temporary that an
    // already-inlined argument still refers to counts.
    let between_feeds_this_call = (definition + 1..index).all(|line| {
        // A line already consumed by an earlier inlining is gone, not in the way.
        lines[line].is_empty()
            || defined_temporary(&lines[line])
                .is_some_and(|feeder| count_ident(&lines[index], &feeder) > 0)
    });
    if !between_feeds_this_call {
        inline_reject("between", callee, &temp, &lines[index]);
        return false;
    }
    // Mixing `&&` and `||` without parentheses is a warning here, and warnings are errors
    // (measured: 12 of them). A value that carries one logical operator into a line that already
    // holds the other gets its own parentheses — the precedence that was already meant.
    let mixes_logical_operators = |a: &str, b: &str| {
        (a.contains(" && ") && b.contains(" || ")) || (a.contains(" || ") && b.contains(" && "))
    };
    let value = match mixes_logical_operators(&value, &lines[index]) {
        true => format!("({value})"),
        false => value,
    };
    lines[index] = rename_ident(&lines[index], temp, &value);
    lines[definition].clear();
    true
}

/// Whether an operand read can take a value in place of the slot. It cannot in a `return` of a
/// reference (the returned reference would outlive the temporaries the expression built), and it
/// cannot inside arithmetic, where the slot declaration is what typed the operands. A comparison
/// takes either side as it is, which is the case worth moving.
fn operand_read_takes_a_value(line: &str) -> bool {
    let Some(expression) = statement_expression(line) else {
        return false;
    };
    !line.trim().starts_with("return ") && !expression.contains(['*', '/', '+', '-'])
}

/// The statement gives its value to a SCALAR. Struct arithmetic goes through operator overloads
/// whose result the declaration is still what fixes, so a scalar witness says nothing there
/// ("No conversion from 'FVector' to math type available").
fn assigns_a_scalar(line: &str, locals: &BTreeMap<i32, String>) -> bool {
    let Some((head, _)) = line.trim().split_once(" = ") else {
        return true; // not a declaration or assignment — nothing to disagree with
    };
    let mut words = head.split_whitespace();
    let Some(name) = words.next_back() else {
        return true;
    };
    match words.next_back() {
        Some(ty) => is_primitive(ty.trim_start_matches("const ")),
        // A bare `local_N = …`: the slot table is what says what it holds.
        None => temporary_type(locals, name).is_some_and(is_primitive),
    }
}

/// Where in a statement a temporary was found.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Argument,
    Receiver,
    Operand,
}

/// `this.<Field>` whose declared field type is exactly the slot's. Reading a member has no side
/// effect of its own, so it can be read where it is used instead of being materialized into a
/// slot first — but only when the slot's declaration was not also performing a conversion.
/// A bare class NAME stored into a class-typed slot: `local_4 = UHumanFists;`.
///
/// The decompiler renders a static class reference that way, and vanilla builds it as a temporary
/// wherever it is used — `PshGPtr __StaticType_UHumanFists; CHKREF; opImplConv; STOREOBJ`. It has
/// no operands of its own, so moving it into its reader cannot reorder anything, and leaving it
/// behind keeps a name in an arm that vanilla wrote as one expression.
fn names_a_static_class(value: &str, temp: &str, locals: &BTreeMap<i32, String>) -> bool {
    if !value.starts_with(|c: char| c.is_ascii_uppercase())
        || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return false;
    }
    temp.strip_prefix("local_")
        .and_then(|rest| rest.split('_').next())
        .and_then(|rest| rest.parse::<i32>().ok())
        .and_then(|slot| locals.get(&slot))
        .is_some_and(|ty| ty == "UClass" || ty.starts_with("TSubclassOf<"))
}

fn same_typed_own_field(
    value: &str,
    temp: &str,
    locals: &BTreeMap<i32, String>,
    fields: Option<&HashMap<String, String>>,
) -> bool {
    let Some(field) = value.strip_prefix("this.") else {
        return false;
    };
    if field.is_empty() || field.contains(['.', '(', '[', ' ']) {
        return false;
    }
    let (Some(fields), Some(slot_type)) = (fields, temporary_type(locals, temp)) else {
        return false;
    };
    fields
        .get(field)
        .is_some_and(|declared| same_scalar_type(declared, slot_type))
}

/// The same type under either of its names. The slot table calls the fork's 8-byte float
/// `double` where a class field calls it `float`; that is one type, not a conversion.
fn same_scalar_type(a: &str, b: &str) -> bool {
    fn canonical(ty: &str) -> &str {
        match ty {
            "double" | "float64" => "float",
            other => other,
        }
    }
    canonical(a) == canonical(b)
}

/// Every `local_N` the statement's expression reads.
fn statement_temporaries(line: &str) -> Vec<String> {
    let Some(expression) = statement_expression(line) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (at, _) in expression.match_indices("local_") {
        if expression[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let digits: String = expression[at + 6..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if digits.is_empty() {
            continue;
        }
        let name = format!("local_{digits}");
        if !found.contains(&name) {
            found.push(name);
        }
    }
    found
}

/// The method a statement's call runs DIRECTLY on `temp`. A chain through a field
/// (`local_4.Handle.Reset()`) is not it: the call runs on the field, not on the temporary.
fn receiver_method(line: &str, temp: &str) -> Option<String> {
    let rest = statement_expression(line)?
        .strip_prefix(temp)?
        .strip_prefix('.')?;
    let name = rest.split('(').next()?;
    (!name.is_empty() && !name.contains('.')).then(|| name.to_owned())
}

/// The type the slot table gives `temp`.
fn temporary_type<'a>(locals: &'a BTreeMap<i32, String>, temp: &str) -> Option<&'a str> {
    let slot = temp.strip_prefix("local_")?.parse::<i32>().ok()?;
    locals.get(&slot).map(String::as_str)
}

/// A UObject/AActor handle by UE’s own naming rule.
fn is_object_handle_type(ty: &str) -> bool {
    let bare = super::structure::bare_type_name(ty.trim_start_matches("const "));
    let bytes = bare.as_bytes();
    if bytes.len() < 2 || !matches!(bytes[0], b'A' | b'U') {
        return false;
    }
    // `AGothicCharacter`, and also `A_FireGolem_EnvironmentAttack_ArenaLavaStream`: the prefix may
    // be separated from the name by an underscore, and reading that as a value type made a range
    // for over handles look like one over values — where a non-const call really would be a write.
    bytes[1].is_ascii_uppercase()
        || (bytes[1] == b'_' && bytes.get(2).is_some_and(u8::is_ascii_uppercase))
}

/// The value is a CALL’s result. A parenthesized expression ends in `)` as well and has no call
/// to move, and moving it into a receiver position makes a temporary out of it.
fn is_call_result(value: &str) -> bool {
    // `!(<call>)` reads the same value the call produced, one operator on top, and a fully
    // parenthesized call is still that call.
    if let Some(negated) = value.strip_prefix('!') {
        return is_call_result(negated);
    }
    if value.starts_with('(')
        && matching_paren(value, 0) == Some(value.len() - 1)
        && value.len() > 2
    {
        return is_call_result(&value[1..value.len() - 1]);
    }
    let Some(inner) = value.strip_suffix(')') else {
        return false;
    };
    let mut depth = 1usize;
    for (at, c) in inner.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    return inner[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '>');
                }
            }
            _ => {}
        }
    }
    false
}

/// The expression a statement evaluates, whatever statement it is: the right-hand side of an
/// assignment, the value of a `return`, or the condition of an `if`/`while`/`switch`. A call
/// standing in a condition is still a call, and its arguments still have producers that belong
/// back inside it — reading only assignments left every condition's call alone.
fn statement_expression(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    for keyword in ["if ", "while ", "switch "] {
        if let Some(rest) = trimmed.strip_prefix(keyword) {
            let rest = rest.trim();
            return (rest.starts_with('(') && matching_paren(rest, 0)? == rest.len() - 1)
                .then(|| rest[1..rest.len() - 1].trim());
        }
    }
    let statement = trimmed.strip_suffix(';')?;
    if let Some(value) = statement.strip_prefix("return ") {
        return Some(value.trim());
    }
    Some(match statement.split_once(" = ") {
        Some((_, rhs)) => rhs,
        None => statement,
    })
}

/// The temporary a statement’s own call runs on. `local_4.GetDistanceTo(x)` reads `local_4` AFTER
/// the argument — which is where vanilla evaluates it; a producer line above the call evaluates
/// it before instead, and that reordering is the whole difference.
fn receiver_temporary(line: &str) -> Option<String> {
    let mut expression = statement_expression(line)?;
    // A negated or parenthesized value still runs on the same receiver.
    loop {
        let stripped = expression.strip_prefix('!').unwrap_or(expression);
        let stripped = if stripped.starts_with('(')
            && matching_paren(stripped, 0) == Some(stripped.len() - 1)
            && stripped.len() > 2
        {
            &stripped[1..stripped.len() - 1]
        } else {
            stripped
        };
        if stripped == expression {
            break;
        }
        expression = stripped;
    }
    let (head, rest) = expression.split_once('.')?;
    (is_local_ident(head) && rest.contains('(') && count_ident(rest, head) == 0)
        .then(|| head.to_owned())
}

/// The value a statement gives `ident`, whether it declares it at the same time or not.
fn definition_value<'a>(line: &'a str, ident: &str) -> Option<&'a str> {
    if let Some(rhs) = assignment_rhs_for(line, ident) {
        return Some(rhs);
    }
    let trimmed = line.trim().strip_suffix(';')?;
    let (head, rhs) = trimmed.split_once(" = ")?;
    (head.split_whitespace().last() == Some(ident)
        && count_ident(rhs, ident) == 0
        && !rhs.is_empty())
    .then_some(rhs)
}

/// The temporary a statement defines, if it is a plain `local_N = …;` definition.
fn defined_temporary(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let (head, _) = trimmed.split_once(" = ")?;
    let name = head.split_whitespace().last()?;
    is_local_ident(name).then(|| name.to_owned())
}

/// The temporary an argument is built from: the argument itself, or the receiver a member chain
/// starts at. `Say(local_6.GetAI(), …)` reads `local_6` exactly where the bare form would, so the
/// producer moves back into the call the same way — and that is the difference between evaluating
/// the receiver before the other arguments and evaluating it where vanilla does.
fn argument_temporary(argument: &str) -> Option<String> {
    if is_local_ident(argument) {
        return Some(argument.to_owned());
    }
    let (head, rest) = argument.split_once('.')?;
    (is_local_ident(head) && count_ident(rest, head) == 0).then(|| head.to_owned())
}

/// `local_12` and nothing else.
fn is_local_ident(text: &str) -> bool {
    text.strip_prefix("local_")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
}

/// The callee name and top-level argument list of a statement that IS one call.
fn call_arguments(line: &str) -> Option<(String, Vec<String>)> {
    call_of_expression(statement_expression(line)?)
}

/// Every call ON a line, outermost first: the statement's own call, and then the calls its
/// arguments are. An argument that is itself a call has arguments of its own, and each of those
/// is read where it stands — the outer call's parameter table says nothing about them.
fn call_sites(line: &str) -> Vec<(String, Vec<String>)> {
    let mut sites = Vec::new();
    let mut pending: Vec<String> = statement_expression(line).into_iter().map(str::to_owned).collect();
    while let Some(expression) = pending.pop() {
        let Some((callee, arguments)) = call_of_expression(&expression) else {
            continue;
        };
        pending.extend(arguments.iter().cloned());
        sites.push((callee, arguments));
    }
    sites
}

/// The callee and arguments of an expression that IS one call, or None.
fn call_of_expression(expression: &str) -> Option<(String, Vec<String>)> {
    // `!(<call>)` and `(<call>)` are still that call, and its parameters still decide what may
    // stand in each position.
    let mut statement = expression;
    loop {
        let stripped = statement.strip_prefix('!').unwrap_or(statement);
        let stripped = if stripped.starts_with('(')
            && matching_paren(stripped, 0) == Some(stripped.len() - 1)
            && stripped.len() > 2
        {
            &stripped[1..stripped.len() - 1]
        } else {
            stripped
        };
        if stripped == statement {
            break;
        }
        statement = stripped;
    }
    let open = statement.find('(')?;
    if matching_paren(statement, open)? != statement.len() - 1 {
        return None; // not a single call: something follows the closing parenthesis
    }
    let callee = statement[..open].rsplit(['.', ':']).next()?.to_owned();
    if callee.is_empty()
        || !callee
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    let inner = &statement[open + 1..statement.len() - 1];
    let mut arguments = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (at, byte) in inner.bytes().enumerate() {
        if in_string {
            match byte {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'(' | b'<' | b'[' => depth += 1,
            b')' | b'>' | b']' => depth -= 1,
            b',' if depth == 0 => {
                arguments.push(inner[start..at].trim().to_owned());
                start = at + 1;
            }
            _ => {}
        }
    }
    if !inner.trim().is_empty() {
        arguments.push(inner[start..].trim().to_owned());
    }
    Some((callee, arguments))
}

/// A constructor whose whole body is `super();` and member stores is the compiler's lowering of
/// member INITIALIZERS. Take those stores out of the constructor and return them, so the field
/// declarations can carry them: written back as constructor statements, the member is
/// default-constructed first, which asks for a behaviour the base cache may not have.
///
/// Anything else in the body — a call, a branch, a local — means the constructor really is a
/// constructor, and it keeps every statement.
fn extract_member_initializers(constructors: &mut String) -> HashMap<String, String> {
    let mut initializers = HashMap::new();
    let lines: Vec<&str> = constructors.lines().collect();
    let mut keep: Vec<String> = Vec::with_capacity(lines.len());
    let mut index = 0usize;
    while index < lines.len() {
        // A constructor: signature, `{`, body…, `}`.
        let Some(open) = lines.get(index + 1).filter(|line| line.trim() == "{") else {
            keep.push(lines[index].to_owned());
            index += 1;
            continue;
        };
        let _ = open;
        let Some(close) = lines
            .iter()
            .enumerate()
            .skip(index + 2)
            .find(|(_, line)| line.trim() == "}")
            .map(|(at, _)| at)
        else {
            keep.push(lines[index].to_owned());
            index += 1;
            continue;
        };
        let body = &lines[index + 2..close];
        // A value that reads a constructor PARAMETER (or a local) is not an initializer — the
        // member takes what the caller passed, and moving it to the declaration would reference
        // a name that does not exist there.
        let parameters = signature_parameters(lines[index]);
        let stores: Option<Vec<(String, String)>> = body
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed != "super();" && trimmed != "return;"
            })
            .map(|line| {
                member_store(line).filter(|(_, value)| {
                    count_ident(value, "local") == 0
                        && !value.contains("local_")
                        && parameters
                            .iter()
                            .all(|parameter| count_ident(value, parameter) == 0)
                })
            })
            .collect();
        match stores {
            Some(stores) if !stores.is_empty() => {
                for (field, value) in stores {
                    initializers.insert(field, value);
                }
                keep.push(lines[index].to_owned());
                keep.push(lines[index + 1].to_owned());
                for line in body {
                    let trimmed = line.trim();
                    if trimmed == "super();" || trimmed == "return;" {
                        keep.push((*line).to_owned());
                    }
                }
                keep.push(lines[close].to_owned());
            }
            _ => {
                for line in &lines[index..=close] {
                    keep.push((*line).to_owned());
                }
            }
        }
        index = close + 1;
    }
    // A constructor the pass above could not lift from — one that also takes a value from a
    // parameter — still carries the member initializer the compiler puts at the top of EVERY
    // constructor. The declaration now says it once; saying it again here is a store vanilla
    // never emitted (measured: 283 of them).
    if !initializers.is_empty() {
        let mut body_indent = String::new();
        let mut written: HashSet<String> = HashSet::new();
        keep.retain(|line| {
            if line.trim() == "{" {
                written.clear();
                body_indent = format!("{}    ", indent_of(line));
                return true;
            }
            // Only a store at the constructor's own level: one inside a branch happens on some
            // paths and not others, which is not what a declaration says.
            if indent_of(line) != body_indent {
                return true;
            }
            let Some((field, value)) = member_store(line) else {
                return true;
            };
            let first_write = written.insert(field.clone());
            !(first_write && initializers.get(&field).is_some_and(|init| *init == value))
        });
        *constructors = keep.join("\n");
        constructors.push('\n');
    }
    initializers
}

/// `local_N = <expr>; if (local_N)` is `if (<expr>)`. The name buys a declaration and a copy of
/// the value into its own slot — vanilla tested the expression's own slot and branched.
///
/// Where the slot IS the whole condition, the type table has to call it `bool` and the value has
/// to render as one: a condition is a boolean context, and handing it an `int` is a compile
/// error, not a conversion. Where the slot is an `int` the emitter reads as a bool by comparing
/// it against zero, the comparison goes with the slot — but only once the value is PROVEN a
/// bool, because otherwise the same conversion is asked for in the other direction (measured:
/// folding any left-hand relation without that proof costs 44 errors).
///
/// Either way only a slot read exactly once, on that very line: a second reader would have to
/// evaluate the expression again.
fn fold_condition_temporaries(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
) -> String {
    // A field of the class carries its type in the class's own map, which `renders_a_bool` does
    // not read.
    let is_a_bool = |value: &str| {
        renders_a_bool(value, locals, refs, fields)
            || value
                .strip_prefix("this.")
                .and_then(|field| fields?.get(field))
                .is_some_and(|ty| ty == "bool")
    };
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            let (name, value) = slot_store(lines[at])?;
            let tested = lines.get(at + 1)?.trim();
            let condition = tested.strip_prefix("if (")?.strip_suffix(')')?;
            if value.contains(&name)
                || value.chars().any(char::is_control)
                || count_ident(tested, &name) != 1
                || !read_once_by_the_next_line(&lines, at, &name)
            {
                return None;
            }
            // The whole condition IS the slot: a boolean context, so both the slot and the value
            // have to be bools — handing a condition an `int` is an error, not a conversion.
            let whole = (condition == name
                || condition == format!("!{name}")
                || condition == format!("!({name})"))
                .then(|| match condition == name {
                    true => format!("({value})"),
                    false => format!("(!({value}))"),
                })
                .filter(|_| {
                    temporary_type(locals, &name) == Some("bool")
                        && renders_a_bool(&value, locals, refs, fields)
                });
            // Or the slot is an INT the emitter compares against zero to read as a bool. Where
            // the value is a bool, that comparison is the int slot's doing and nothing else's:
            // the source tested the value. Keeping it would ask the compiler to convert an `int`
            // to a `bool`, which it refuses (measured: 44 errors).
            let unwrapped = || {
                let negated = match condition.strip_prefix(name.as_str())? {
                    " != 0" => false,
                    " == 0" => true,
                    _ => return None,
                };
                is_a_bool(&value).then(|| match negated {
                    true => format!("(!({value}))"),
                    false => format!("({value})"),
                })
            };
            let folded = whole.or_else(unwrapped)?;
            Some(format!("{}if {folded}", indent_of(lines[at + 1])))
        })();
        match folded {
            Some(replacement) => {
                kept.push(replacement);
                at += 2;
            }
            None => {
                kept.push(lines[at].to_string());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}


/// Whether the store at `at` is read exactly once — by the line right after it — and by nothing
/// else before the slot is written again. The compiler reuses ONE slot for many unrelated
/// temporaries, so counting the name over the whole body answers a question nobody asked: what
/// matters is the live range of THIS store.
fn read_once_by_the_next_line(lines: &[&str], at: usize, name: &str) -> bool {
    read_once_at(lines, at, at + 1, name)
}

/// The same question where the reader is not the next line: the store at `at` is read by
/// `reader` and by nothing else before the slot is written again.
fn read_once_at(lines: &[&str], at: usize, reader: usize, name: &str) -> bool {
    // Forward, from past the reader to the next write of the same slot.
    for line in &lines[reader + 1..] {
        if is_definition_line(line, name) {
            break;
        }
        if count_ident(line, name) > 0 {
            return false;
        }
    }
    // A loop carries the value back: a read placed BEFORE the store inside the same loop body
    // reads what this store left behind on the previous pass.
    let indent = indent_of(lines[at]).len();
    for start in (0..at).rev() {
        let trimmed = lines[start].trim_start();
        if indent_of(lines[start]).len() >= indent
            || !(trimmed.starts_with("while (") || trimmed.starts_with("for ("))
        {
            continue;
        }
        if block_end(lines, start + 1).is_some_and(|end| end > at) {
            return !lines[start + 1..at]
                .iter()
                .any(|line| count_ident(line, name) > 0);
        }
        break;
    }
    true
}

/// `local_A = local_B; <reader of local_A>` is the reader reading `local_B`. A handle assigned
/// straight from another handle and read once is a name for something that already had one, and
/// it costs the push and the `RefCpyV` that name it.
///
/// Only an alias of the SAME declared type: a copy between different handle types is an implicit
/// cast, and the reader would then see the other type. And only a slot read exactly once, on the
/// line right after the copy — anything further away could see `local_B` reassigned in between.
fn fold_alias_copies(body: &str, locals: &BTreeMap<i32, String>) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            let (target, source) = slot_store(lines[at])?;
            if !is_local_ident(&source) || source == target {
                return None;
            }
            let ty = temporary_type(locals, &target)?;
            if temporary_type(locals, &source) != Some(ty) {
                return None;
            }

            // The single reader may sit further down, but only inside the same block and with
            // nothing reassigning the source on the way: a copy that does not dominate its
            // reader, or a source that has moved on, is not the same value any more.
            let indent = indent_of(lines[at]);
            let mut found = None;
            for (offset, line) in lines[at + 1..].iter().enumerate() {
                if !line.trim().is_empty() && indent_of(line).len() < indent.len() {
                    break; // left the block the copy stands in
                }
                if slot_store(line).is_some_and(|(lhs, _)| lhs == source) {
                    break; // the source moved on
                }
                if count_ident(line, &target) > 0 {
                    let usable = count_ident(line, &target) == 1
                        && !slot_store(line).is_some_and(|(lhs, _)| lhs == target)
                        && indent_of(line) == indent;
                    found = usable.then_some(at + 1 + offset);
                    break;
                }
            }
            let reader = found?;
            // And nothing reads this copy's value after that: the slot is the compiler's, reused
            // for other temporaries elsewhere in the body, so the name's total count says
            // nothing.
            if !read_once_at(&lines, at, reader, &target) {
                return None;
            }
            let mut out: Vec<String> = lines[at + 1..reader].iter().map(|l| (*l).to_owned()).collect();
            out.push(rename_ident(lines[reader], &target, &source));
            Some((out, reader + 1))
        })();
        match folded {
            Some((replacement, after)) => {
                kept.extend(replacement);
                at = after;
            }
            None => {
                kept.push(lines[at].to_string());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}


/// `local_A = <expr>; local_B = local_A;` is `local_B = <expr>;`. A cast, and any call the
/// compiler lands in a slot of its own, writes that slot and then copies it into the one the
/// source named. Naming BOTH makes the recompile allocate two variables where the original had
/// one and the compiler's temporary — and pay a `PshVPtr`/`RefCpyV` to get between them.
///
/// Only where the carrier is read exactly once, by that copy, and where the two slots agree on
/// type AND on constness: the carrier's declaration is what performs a conversion, and handing
/// the value straight to the target would skip it.
fn fold_copy_out_temporaries(
    body: &str,
    locals: &BTreeMap<i32, String>,
    const_slots: &HashSet<i32>,
    fields: Option<&HashMap<String, String>>,
) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            let (carrier, value) = slot_store(lines[at])?;
            let copy = lines.get(at + 1)?.trim().strip_suffix(';')?;
            let (target, copied) = copy.split_once(" = ")?;
            if copied != carrier || target == carrier {
                return None;
            }
            let a = slot_of(&carrier)?;
            let carrier_type = locals.get(&a)?;
            // The target says what it holds either through the slot table or, for a member of
            // this class, through the class's own field map. Anything else — an index, a member
            // of something else — has no type here and keeps its carrier.
            let target_type = match slot_of(target) {
                Some(b) => {
                    if const_slots.contains(&a) != const_slots.contains(&b) {
                        return None;
                    }
                    locals.get(&b)
                }
                None => fields?.get(target.strip_prefix("this.")?),
            };
            if target_type != Some(carrier_type)
                || indent_of(lines[at]) != indent_of(lines[at + 1])
                || !read_once_at(&lines, at, at + 1, &carrier)
            {
                return None;
            }
            Some(format!("{}{target} = {value};", indent_of(lines[at])))
        })();
        match folded {
            Some(replacement) => {
                kept.push(replacement);
                at += 2;
            }
            None => {
                kept.push(lines[at].to_string());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// `local_N = <call>; … Cast<T>(local_N) …` is the cast reading the call. A cast always
/// materializes its operand into a slot of its own — vanilla's stream shows `CALLSYS f; STOREOBJ
/// d; CmpPtrNull d; JZ; TYPEID; PSF out; PshVPtr d; CALLSYS opCast`, where `d` is the compiler's,
/// not the source's. Naming it makes the recompile spend a second slot and a copy.
///
/// The witness that the name performs no conversion is the one the producer sweep already uses:
/// the slot catches EXACTLY what the callee returns. Where the declaration widened the value, the
/// cast would otherwise see the other type.
fn fold_cast_operands(
    body: &str,
    locals: &BTreeMap<i32, String>,
    call_types: &HashMap<i32, String>,
) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            let (name, value) = slot_store(lines[at])?;
            let slot = slot_of(&name)?;
            let declared = locals.get(&slot)?;
            if call_types.get(&slot) != Some(declared) {
                return None;
            }
            let reader = lines.get(at + 1)?;
            let marker = format!("({name})");
            let opens = reader.find(&marker)?;
            // The operand of a CAST, not of anything else: the character before the parenthesis
            // closes the template argument.
            if reader[..opens].chars().next_back() != Some('>')
                || !reader[..opens].contains("Cast<")
                || count_ident(reader, &name) != 1
                || !read_once_at(&lines, at, at + 1, &name)
            {
                return None;
            }
            Some(rename_ident(reader, &name, &value))
        })();
        match folded {
            Some(replacement) => {
                kept.push(replacement);
                at += 2;
            }
            None => {
                kept.push(lines[at].to_string());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// `local_N = int(this.F); … EFoo(local_N) …` is the reader reading `this.F`. A byte-backed enum
/// field read into an `int` slot and cast back at the use site is a round trip the source never
/// wrote — it costs a widening and a narrowing, and it costs the slot.
///
/// The witness is an exact name match, not an inference: the class field map says what `this.F`
/// is, and the cast at the read spells the same type. Where the two names differ, the conversion
/// is real and the slot stays.
fn fold_enum_round_trips(
    body: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            let (name, value) = slot_store(lines[at])?;
            let read = value.strip_prefix("int(")?.strip_suffix(')')?;
            let declared = &enum_of_member_path(read, fields, roots, refs)?;
            let field = read;
            let reader = lines.get(at + 1)?;
            let cast = format!("{declared}({name})");
            if !reader.contains(&cast)
                || count_ident(reader, &name) != 1
                || !read_once_at(&lines, at, at + 1, &name)
            {
                return None;
            }
            Some(reader.replacen(&cast, field, 1))
        })();
        match folded {
            Some(replacement) => {
                kept.push(replacement);
                at += 2;
            }
            None => {
                kept.push(lines[at].to_string());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    // The same round trip written INLINE, with no slot to travel through:
    // `EPerceptionSense(int(this.Sense))` is `this.Sense`. Same witness — the class field map
    // names the field's type and the cast spells the same name — and the same refusal where the
    // two names differ, because then the conversion is real.
    joined = rewrite_inline_enum_round_trips(&joined, fields, roots, refs);
    joined
}

/// `X.F = X.F + 1;` is `X.F += 1;`. Spelling the destination twice makes the compiler compute
/// its address twice — an extra `LoadRObjR` per statement — where vanilla loaded it once and
/// wrote back through the same pointer.
///
/// Only a pure member path may fold: no parentheses and no brackets, so nothing in the
/// destination is a call or an index whose second evaluation could differ. And only where the two
/// spellings are character-for-character the same, which is what makes them the same object.
/// `T X = <expr>; X = X <op> <rest>;` followed by a single read of `X` is one expression.
///
/// The compiler evaluates `<expr>`, applies `<op>`, and passes the result on — the same three
/// steps in the same order as the inlined `(<expr> <op> <rest>)`. What the name costs is a copy:
/// vanilla widens or loads straight into the slot the arithmetic runs on (`fTOd t, x; MULd t, t,
/// y`), while a declared local makes the compiler copy the value on to the slot it declared.
///
/// Two witnesses have to hold. `X` is mentioned exactly three times in the body, so the read
/// after the accumulation is its last, and `X` is not one of the slots `widened_slots` proves the
/// source NAMED — there the copy IS the declaration and taking it out changes the width the
/// arithmetic happens at.
fn collapse_single_use_accumulators(body: &str, widened: &HashSet<i32>) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len().saturating_sub(2) {
            // Either `T X = <init>;` on one line, or the same thing split in two — `T X;` and
            // then `X = <init>;`. The split form is what a declaration hoist leaves behind, and
            // it is most of the sites here.
            let (span, indent, name, init) = match declaration_with_initializer(&lines[index]) {
                Some((indent, name, init)) => (1usize, indent, name, init),
                None => {
                    let Some((indent, name)) = bare_declaration(&lines[index]) else {
                        continue;
                    };
                    let Some(init) = lines
                        .get(index + 1)
                        .map(|line| line.trim())
                        .and_then(|line| line.strip_prefix(&format!("{name} = ")))
                        .and_then(|line| line.strip_suffix(';'))
                        .filter(|init| !init.contains(&name))
                    else {
                        continue;
                    };
                    (2usize, indent, name, init.to_owned())
                }
            };
            if name
                .strip_prefix("local_")
                .and_then(|slot| slot.split('_').next())
                .and_then(|slot| slot.parse::<i32>().ok())
                .is_some_and(|slot| widened.contains(&slot))
            {
                continue;
            }
            let Some(accumulate) = lines.get(index + span).map(|line| line.trim()) else {
                continue;
            };
            let _ = accumulate;
            // A CHAIN of accumulations, not just one: `X = X + a; X = X + b;` is a single
            // expression in vanilla, and it accumulates in the slot the value landed in.
            let mut folded_value = init.clone();
            let mut steps = 0usize;
            while let Some(line) = lines.get(index + span + steps).map(|line| line.trim()) {
                let Some(value) = line
                    .strip_prefix(&format!("{name} = "))
                    .and_then(|rest| rest.strip_suffix(';'))
                else {
                    break;
                };
                // The name may stand on either side of the operator — `X = X * m` and
                // `X = Radius + X` are both the compiler accumulating into the slot it read
                // into. The written order is kept, so a non-commutative operator stays what it
                // was.
                let next = if let Some(rest) = value.strip_prefix(&format!("{name} ")) {
                    let Some((op, right)) = rest.split_once(' ') else {
                        break;
                    };
                    if !matches!(op, "+" | "-" | "*" | "/") || right.contains(&name) {
                        break;
                    }
                    format!("({folded_value} {op} {right})")
                } else if let Some(head) = value.strip_suffix(&format!(" {name}")) {
                    let Some((left, op)) = head.rsplit_once(' ') else {
                        break;
                    };
                    if !matches!(op, "+" | "-" | "*" | "/") || left.contains(&name) {
                        break;
                    }
                    format!("({left} {op} {folded_value})")
                } else {
                    break;
                };
                folded_value = next;
                steps += 1;
            }
            if steps == 0 {
                continue;
            }
            // The declaration, twice per accumulation, and the single read that consumes it —
            // plus, where the declaration is split from its first write, that write's mention.
            let expected = 2 * steps + if span == 1 { 2 } else { 3 };
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != expected {
                continue;
            }
            let Some(reader) = (index + span + steps..lines.len())
                .find(|at| count_ident(&lines[*at], &name) == 1)
            else {
                continue;
            };
            let folded = folded_value;
            lines[reader] = rename_ident(&lines[reader], &name, &folded);
            lines.drain(index..index + span + steps);
            let _ = indent;
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The `(indent, name, initializer)` of a `T NAME = <init>;` line whose NAME is a decompiler
/// local. A declaration is the only place a type stands before the name, which is what separates
/// it from the assignment that may follow.
fn declaration_with_initializer(line: &str) -> Option<(String, String, String)> {
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = line.trim().strip_suffix(';')?;
    let (left, init) = rest.split_once(" = ")?;
    let name = left.rsplit(' ').next()?;
    if left.len() == name.len() || !is_decompiler_local(name) {
        return None;
    }
    // A declared type never contains a call or an index.
    let ty = &left[..left.len() - name.len() - 1];
    if ty.contains('(') || ty.contains('[') || ty.contains('=') {
        return None;
    }
    Some((indent, name.to_owned(), init.to_owned()))
}

/// A name the decompiler handed out: `local_12`, or its versioned form `local_12_2`.
fn is_decompiler_local(name: &str) -> bool {
    name.strip_prefix("local_")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '_'))
}

/// `int X = int(<call>);` read once as `E(X)` is the call itself.
///
/// The enum round-trip fold walks a MEMBER path to find the enum a value came from. A call has no
/// path to walk, so the pair of casts stayed and the compiler emitted the round trip they ask for
/// — `sbTOi` down to an int and `iTOb` back up — where vanilla passed the call's own slot.
///
/// The witness is the callee's declared return type, which the cache carries and
/// `call_result_types` has already resolved per slot: when the enum being constructed IS what the
/// call returns, both casts name a type the value already has. Anything else — a different enum,
/// an unresolved callee, a second read of the name — is left alone.
/// The enum a line CONSTRUCTS around a name: `EGenericTaskResult(local_63)` gives
/// `EGenericTaskResult`.
fn enum_around(line: &str, name: &str) -> Option<String> {
    let at = line.find(&format!("({name})"))?;
    let head = &line[..at];
    let start = head
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |i| i + 1);
    let ty = &head[start..];
    (!ty.is_empty() && super::structure::is_enum_name(ty)).then(|| ty.to_owned())
}

/// Whether the function converts between an enum's byte and an int anywhere.
fn has_enum_conversions(f: &Func) -> bool {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return true; // unreadable: assume it does, so nothing is folded on a guess
    };
    instrs
        .iter()
        .any(|ins| matches!(ins.op.name, "iTOb" | "sbTOi" | "iTOsb" | "bTOi"))
}

fn fold_enum_call_round_trips(
    body: &str,
    call_types: &HashMap<i32, String>,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
    returns_by_reference: bool,
    vanilla_converts: bool,
) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len() {
            let Some((_, name, init)) = declaration_with_initializer(&lines[index]) else {
                continue;
            };
            let Some(inner) = init.strip_prefix("int(").and_then(|rest| rest.strip_suffix(')'))
            else {
                continue;
            };
            if !lines[index].trim_start().starts_with("int ") {
                continue;
            }
            let Some(slot) = name
                .strip_prefix("local_")
                .and_then(|rest| rest.split('_').next())
                .and_then(|rest| rest.parse::<i32>().ok())
            else {
                continue;
            };
            // The enum either came out of a CALL, where the callee's declared return type says
            // so, or was read from a member PATH, where the field's own type does.
            let resolved = call_types.get(&slot).cloned().or_else(|| {
                type_of_member_path(inner, fields, roots, refs)
            });
            // Where the value's own type cannot be resolved, the FUNCTION can still say the round
            // trip is not there: a widening and a narrowing leave `sbTOi` and `iTOb` behind, and
            // a function whose bytecode holds neither converted nothing anywhere. The enum named
            // by the reader is then the type the value already has.
            let returned = match resolved.as_ref() {
                Some(returned) => returned.clone(),
                None if !vanilla_converts => {
                    let Some(named) = lines[index + 1..]
                        .iter()
                        .find_map(|line| enum_around(line, &name))
                    else {
                        continue;
                    };
                    named
                }
                None => continue,
            };
            let returned = &returned;
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                continue;
            }
            let round_trip = format!("{returned}({name})");
            let Some(reader) = (index + 1..lines.len()).find(|at| lines[*at].contains(&round_trip))
            else {
                continue;
            };
            // A function that returns BY REFERENCE keeps a read THROUGH A LOCAL in a name of its
            // own. The returned reference outlives the expression, and the local it was read
            // through is cleaned up before the caller sees it — "Resulting reference cannot be
            // returned. The expression uses objects that during cleanup may invalidate it."
            // A call's result is not that: it is not read through anything that goes away here.
            let reads_through_a_local = inner.starts_with("local_");
            if returns_by_reference
                && reads_through_a_local
                && lines[reader].trim_start().starts_with("return ")
            {
                continue;
            }
            lines[reader] = lines[reader].replace(&round_trip, inner);
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Slots the function does not touch until AFTER its first conditional jump.
///
/// A local declared at function scope is CONSTRUCTED at function scope: the compiler runs its
/// `$beh0` before the first branch and its `$beh2` on every path out, including the early returns
/// that never reach the value. Vanilla doing neither is the proof that the declaration stood
/// inside the block that uses it. The first touch of the slot is where the compiler put it.
fn slots_touched_only_after_a_branch(f: &Func) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let first_branch = instrs
        .iter()
        .position(|ins| ins.op.name.starts_with('J') && ins.op.name != "JMP");
    let Some(first_branch) = first_branch else {
        return HashSet::new();
    };
    let mut before = HashSet::new();
    let mut after = HashSet::new();
    for (at, ins) in instrs.iter().enumerate() {
        // Which words of an instruction are frame offsets depends on its format; the operand
        // decoder carries that table, so ask it rather than guess.
        for slot in super::bytediff::addressed_slots(ins).into_iter().filter(|slot| *slot > 0) {
            if at < first_branch {
                before.insert(slot);
            } else {
                after.insert(slot);
            }
        }
    }
    // A HANDLE is a different question from a struct. A struct declared at function scope costs
    // a constructor at entry and a destructor on every path out, which is what this set is for.
    // A handle costs neither — but declared inside a BLOCK it costs an explicit `FreeNullV8` at
    // the block's end, which a function-scope handle does not have. So a handle only belongs
    // inside the block if vanilla releases it there.
    let mut handles = HashSet::new();
    let mut released = HashSet::new();
    for ins in &instrs {
        let slot = ins.words.first().map(|word| *word as i16 as i32);
        match ins.op.name {
            "STOREOBJ" | "RefCpyV" => {
                if let Some(slot) = slot {
                    handles.insert(slot);
                }
            }
            "FreeNullV8" => {
                if let Some(slot) = slot {
                    released.insert(slot);
                }
            }
            _ => {}
        }
    }
    after
        .difference(&before)
        .copied()
        .filter(|slot| !handles.contains(slot) || released.contains(slot))
        .collect()
}

/// Moves a bare declaration down into the block that holds every mention of it.
///
/// The decompiler writes a local's declaration where the slot table lists it, which is the top of
/// the function. A struct declared there costs a constructor at entry and a destructor on every
/// path out — including the early returns that leave before the value exists. `after_branch`
/// carries the slots vanilla proves were not declared there.
fn sink_declarations_into_their_block(body: &str, after_branch: &HashSet<i32>) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    // One move per pass, then everything is derived again: a move renumbers every line after it,
    // and two moves planned against the same numbering land in each other's way.
    for _ in 0..lines.len() {
        let Some((from, to, text)) = next_declaration_to_sink(&lines, after_branch) else {
            break;
        };
        lines.remove(from);
        let to = if to > from { to - 1 } else { to };
        lines.insert(to, text);
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The first declaration in `lines` that may move, as `(from, to, text)`.
fn next_declaration_to_sink(
    lines: &[String],
    after_branch: &HashSet<i32>,
) -> Option<(usize, usize, String)> {
    let depths = block_depths(lines);
    for (index, line) in lines.iter().enumerate() {
        let Some((indent, name)) = bare_declaration(line) else {
            continue;
        };
        let Some(slot) = name
            .strip_prefix("local_")
            .and_then(|rest| rest.split('_').next())
            .and_then(|rest| rest.parse::<i32>().ok())
        else {
            continue;
        };
        if !after_branch.contains(&slot) {
            continue;
        }
        // One declaration only: a second one elsewhere means moving this is what makes the two
        // collide.
        if lines
            .iter()
            .filter(|line| {
                bare_declaration(line).is_some_and(|(_, other)| other == name)
                    || declaration_with_initializer(line)
                        .is_some_and(|(_, other, _)| other == name)
            })
            .count()
            != 1
        {
            continue;
        }
        let mentions: Vec<usize> = (0..lines.len())
            .filter(|at| *at != index && count_ident(&lines[*at], &name) > 0)
            .collect();
        let (Some(first), Some(last)) = (mentions.first(), mentions.last()) else {
            continue;
        };
        // A mention ABOVE the declaration means the name is already read where the declaration
        // would no longer stand.
        if *first < index {
            continue;
        }
        // One block holds them all when nothing between them ever leaves the depth they sit at.
        let depth = depths[*first];
        if depth <= depths[index] || (*first..=*last).any(|at| depths[at] < depth) {
            continue;
        }
        // The block's opening line is the last one shallower than the mentions.
        let Some(open) = (0..*first).rev().find(|at| depths[*at] < depth) else {
            continue;
        };
        // It has to be a block the declaration can move INTO, and one that opens below the
        // declaration: a brace of its own, never a `case` label or a one-line body, and never a
        // loop — a struct declared in a loop body is built and destroyed once per iteration,
        // which is not what a function-scope declaration did.
        if open <= index || lines[open].trim() != "{" {
            continue;
        }
        let heads_a_loop = lines[..open]
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| {
                let head = line.trim_start();
                head.starts_with("for") || head.starts_with("while") || head.starts_with("do")
            });
        if heads_a_loop {
            continue;
        }
        let extra = "    ".repeat(depth - depths[index]);
        return Some((index, open + 1, format!("{indent}{extra}{}", line.trim())));
    }
    None
}

/// The brace depth each line SITS at: a line that opens a block belongs to the outer one, a
/// closing brace to the block it closes.
fn block_depths(lines: &[String]) -> Vec<usize> {
    let mut depths = Vec::with_capacity(lines.len());
    let mut depth = 0usize;
    for line in lines {
        let trimmed = line.trim();
        let closes = trimmed.starts_with('}');
        if closes {
            depth = depth.saturating_sub(1);
        }
        depths.push(depth);
        let opens = line.matches('{').count();
        let shuts = line.matches('}').count();
        depth = (depth + opens).saturating_sub(if closes { shuts - 1 } else { shuts });
    }
    depths
}

/// The `(indent, name)` of a `T NAME;` line — a declaration with no initializer.
fn bare_declaration(line: &str) -> Option<(String, String)> {
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = line.trim().strip_suffix(';')?;
    if rest.contains('=') || rest.contains('(') {
        return None;
    }
    let name = rest.rsplit(' ').next()?;
    if rest.len() == name.len() || !is_decompiler_local(name) {
        return None;
    }
    Some((indent, name.to_owned()))
}

/// A value the decompiler wrapped in one pair of brackets, unwrapped — but only when the pair
/// really does span the whole thing, so `(a) + (b)` keeps both.
fn unwrap_brackets(value: &str) -> &str {
    let Some(inner) = value.strip_prefix('(').and_then(|rest| rest.strip_suffix(')')) else {
        return value;
    };
    let mut depth = 0i32;
    for byte in inner.bytes() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return value;
                }
            }
            _ => {}
        }
    }
    if depth == 0 { inner } else { value }
}

/// How often the function DEFAULT-CONSTRUCTS each slot: `PSF <slot>; CALLSYS $beh0`.
///
/// One construction is a declaration. Two or more of the SAME slot is the compiler reusing one
/// piece of frame for a temporary the source wrote out again at each place it was needed —
/// `f(..., FGameplayTag(), ...)` three times over, not one named local passed three times.
fn default_construction_counts(f: &Func, refs: &RefResolver) -> HashMap<i32, usize> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashMap::new();
    };
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for pair in instrs.windows(2) {
        if pair[0].op.name != "PSF" || pair[1].op.name != "CALLSYS" {
            continue;
        }
        let ptr = pair[1].qwords.first().copied().unwrap_or(0) as i64;
        if refs.func_by_ptr(ptr) != Some("$beh0") {
            continue;
        }
        let Some(slot) = pair[0].words.first().map(|word| *word as i16 as i32) else {
            continue;
        };
        if slot > 0 {
            *counts.entry(slot).or_default() += 1;
        }
    }
    counts
}

/// A bare declaration for a slot the function constructs more than once is not a declaration.
///
/// The source wrote the value where it was used, and the compiler put each of those temporaries
/// in the same piece of frame. Written as one named local it is constructed once and passed on,
/// which is a construction fewer at every use after the first.
fn spell_out_repeated_temporaries(body: &str, constructions: &HashMap<i32, usize>) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len() {
            let Some((_, name)) = bare_declaration(&lines[index]) else {
                continue;
            };
            let Some(slot) = name
                .strip_prefix("local_")
                .and_then(|rest| rest.split('_').next())
                .and_then(|rest| rest.parse::<i32>().ok())
            else {
                continue;
            };
            if constructions.get(&slot).copied().unwrap_or(0) < 2 {
                continue;
            }
            let ty = lines[index].trim().trim_end_matches(';');
            let ty = ty[..ty.len() - name.len()].trim().to_owned();
            // Only where every mention READS it: a write through the name would have to happen to
            // something, and each spelled-out temporary is a different something.
            if lines.iter().enumerate().any(|(at, line)| {
                at != index && line.trim().starts_with(&format!("{name} ")) || line.contains(&format!("{name}."))
            }) {
                continue;
            }
            // Every use in ONE block. Two constructions of a slot mean two temporaries the
            // compiler laid on the same piece of frame — but that is only true where they belong
            // to one scope. Across blocks it is the other thing that shares a slot: a separate
            // declaration per block, whose lifetimes do not overlap. Spelling THOSE out inline
            // puts a temporary where the source had a named local, and a parameter taking a
            // non-const reference refuses one.
            let depths = block_depths(&lines);
            let mentions: Vec<usize> = (0..lines.len())
                .filter(|at| *at != index && count_ident(&lines[*at], &name) > 0)
                .collect();
            let (Some(first), Some(last)) = (mentions.first(), mentions.last()) else {
                continue;
            };
            let depth = depths[*first];
            if mentions.iter().any(|at| depths[*at] != depth)
                || (*first..=*last).any(|at| depths[at] < depth)
            {
                continue;
            }
            let fresh = format!("{ty}()");
            for at in 0..lines.len() {
                if at != index {
                    lines[at] = rename_ident(&lines[at], &name, &fresh);
                }
            }
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// How often the function hands each slot's ADDRESS to a call as an ARGUMENT.
///
/// A `PSF` immediately before the call is the RECEIVER — that is the last thing pushed. One with
/// anything else between it and the call went on the stack as an argument.
fn address_push_counts(f: &Func) -> HashMap<i32, usize> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashMap::new();
    };
    let calls = |name: &str| matches!(name, "CALL" | "CALLSYS" | "CALLINTF" | "CALLBND");
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for (at, ins) in instrs.iter().enumerate() {
        if ins.op.name != "PSF" || instrs.get(at + 1).is_some_and(|next| calls(next.op.name)) {
            continue;
        }
        if let Some(slot) = ins.words.first().map(|word| *word as i16 as i32) {
            if slot > 0 {
                *counts.entry(slot).or_default() += 1;
            }
        }
    }
    counts
}

/// A struct the source built and then HANDED ON, where the rendering lost the hand-off.
///
/// Vanilla pushes the slot's own address at the argument (`PSF v8`); where the argument came out
/// as a freshly default-constructed `T()` instead, the call receives an empty struct and
/// everything done to the named one is thrown away — `RemoveActiveEffectsWithTags` asked to
/// remove effects with NO tags rather than the one just added.
///
/// The witness is a count: vanilla pushes the slot once more than the text mentions it. Only the
/// single-hand-off case is repaired, and only where the placeholder stands AFTER the last mention
/// of the name, so nothing is put where the value did not exist yet.
fn restore_dropped_struct_arguments(body: &str, pushes: &HashMap<i32, usize>) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    for index in 0..lines.len() {
        let Some((_, name)) = bare_declaration(&lines[index]) else {
            continue;
        };
        let ty = {
            let trimmed = lines[index].trim().trim_end_matches(';');
            trimmed[..trimmed.len() - name.len()].trim().to_owned()
        };
        let Some(slot) = name
            .strip_prefix("local_")
            .and_then(|rest| rest.split('_').next())
            .and_then(|rest| rest.parse::<i32>().ok())
        else {
            continue;
        };
        // The name has to stand for something already — a struct built and then used — and
        // vanilla has to hand its address to a call somewhere.
        let mentions: usize = lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() - 1;
        if mentions == 0 || pushes.get(&slot).copied().unwrap_or(0) == 0 {
            continue;
        }
        let placeholder = format!("{ty}()");
        let last_mention = (0..lines.len())
            .rev()
            .find(|at| count_ident(&lines[*at], &name) > 0);
        let Some(last_mention) = last_mention else {
            continue;
        };
        let candidates: Vec<usize> = (last_mention + 1..lines.len())
            .filter(|at| lines[*at].matches(&placeholder).count() == 1)
            .collect();
        if candidates.len() != 1 {
            continue;
        }
        let at = candidates[0];
        lines[at] = lines[at].replacen(&placeholder, &name, 1);
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// `T X = <expr>; <target> = X;` is `<target> = <expr>;`.
///
/// The name buys nothing: the compiler evaluates the expression, puts it in the slot the name
/// asked for, and copies it straight on to the target. Vanilla writes the result where it belongs
/// and the copy is not there. Mentioned exactly twice, so the read that follows is its last, and
/// neither side may mention the name itself.
fn fold_assigned_temporaries(
    body: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            let (_, name, init) = declaration_with_initializer(lines[at])?;
            let declared = {
                let head = lines[at].trim().split(" = ").next()?;
                head[..head.len() - name.len()].trim().to_owned()
            };
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                return None;
            }
            let next = lines.get(at + 1)?;
            let (target, value) = next.trim().strip_suffix(';')?.split_once(" = ")?;
            // The declaration may be the CONVERSION: `bool X = <cmp>; IntThing = X;` narrows on
            // the way out, and folding it hands the target a bool it cannot take. Fold only where
            // the target's own type is the one the declaration gave the value, and refuse
            // outright where the target's type cannot be resolved.
            if type_of_member_path(target, fields, roots, refs).as_deref() != Some(declared.as_str())
            {
                return None;
            }
            if value != name
                || count_ident(target, &name) > 0
                || count_ident(&init, &name) > 0
                || indent_of(lines[at]) != indent_of(next)
                || init.contains(RVODEF)
            {
                return None;
            }
            Some(format!("{}{target} = {init};", indent_of(lines[at])))
        })();
        match folded {
            Some(replacement) => {
                kept.push(replacement);
                at += 2;
            }
            None => {
                kept.push(lines[at].to_owned());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// Drops a declaration nothing mentions any more.
///
/// The declarations are written from the set of names the body used BEFORE the folds ran; a fold
/// that takes the last mention away leaves the declaration behind, and an unused local still
/// costs the slot the compiler allocates for it.
fn drop_unused_declarations(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let dead = bare_declaration(line)
            .or_else(|| declaration_with_initializer(line).map(|(indent, name, _)| (indent, name)))
            .is_some_and(|(_, name)| {
                // Only a bare declaration may go: one with an initializer may be running a call.
                bare_declaration(line).is_some()
                    && lines
                        .iter()
                        .enumerate()
                        .all(|(at, other)| at == index || count_ident(other, &name) == 0)
            });
        if !dead {
            kept.push((*line).to_owned());
        }
    }
    let mut joined = kept.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// Drops the handle release a BLOCK already performs.
///
/// A handle declared inside a block is released by the compiler when the block ends. Rendering
/// that release as a statement of its own — `local_6 = nullptr;` as the block's last line — asks
/// for it twice, and vanilla has one `FreeNullV8` where the regen has two.
///
/// Only for a declaration that stands INSIDE a block: at function scope there is no block end to
/// do it, and the statement is the only release there is.
fn drop_block_end_handle_releases(text: &str) -> String {
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let depths = block_depths(&lines);
    let Some(body_depth) = depths.iter().copied().min() else {
        return text.to_owned();
    };
    let mut drop: Vec<bool> = vec![false; lines.len()];
    for (index, line) in lines.iter().enumerate() {
        let Some((_, name)) = bare_declaration(line) else {
            continue;
        };
        if depths[index] <= body_depth {
            continue;
        }
        let release = format!("{name} = nullptr;");
        // The block this declaration sits in ends at the first line shallower than it.
        let Some(close) = (index + 1..lines.len()).find(|at| depths[*at] < depths[index]) else {
            continue;
        };
        if close > 0 && lines[close - 1].trim() == release {
            drop[close - 1] = true;
        }
    }
    let mut joined = lines
        .iter()
        .enumerate()
        .filter(|(at, _)| !drop[*at])
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// `int X = <bool member>; ... (X != 0) ...` is that member.
///
/// The slot fell back to `int`, and an int carrying a bool has to be compared to reach a bool
/// again — so a one-line `return this.m_Activated;` came back as a read, a copy, a comparison and
/// a conversion where vanilla has `RDR1` and a return. The field's own declared type is the
/// witness: where it IS a bool, the comparison is asking for what the value already is.
fn fold_bool_member_comparisons(
    body: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len() {
            let Some((_, name, path)) = declaration_with_initializer(&lines[index]) else {
                continue;
            };
            if !lines[index].trim_start().starts_with("int ") {
                continue;
            }
            if type_of_member_path(&path, fields, roots, refs).as_deref() != Some("bool") {
                continue;
            }
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                continue;
            }
            let comparison = format!("({name} != 0)");
            let Some(reader) = (index + 1..lines.len()).find(|at| lines[*at].contains(&comparison))
            else {
                continue;
            };
            lines[reader] = lines[reader].replace(&comparison, &path);
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// `T X = T(<member>);` where the member IS a `T` already.
///
/// The conversion builds a temporary and assigns it — a default construction and an `opAssign`
/// where vanilla copy-constructs the declaration straight from the member. Naming the type a
/// value already has converts nothing, and the field's own declared type says whether that is the
/// case.
fn drop_redundant_conversions(
    text: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    for line in &mut lines {
        // Either a declaration, `T X = T(<member>);`, or the assignment a hoist left behind,
        // `X = T(<member>);` — the same conversion either way, and the same nothing it converts.
        let Some((indent, name, init)) = declaration_with_initializer(line).or_else(|| {
            let statement = line.trim().strip_suffix(';')?;
            let (target, value) = statement.split_once(" = ")?;
            is_decompiler_local(target).then(|| {
                (indent_of(line), target.to_owned(), value.to_owned())
            })
        }) else {
            continue;
        };
        let head = line.trim().split(" = ").next().unwrap_or("");
        let declared = head[..head.len().saturating_sub(name.len())].trim().to_owned();
        let ty = match declared.is_empty() {
            // An assignment carries no type of its own; the value names the one it converts to.
            true => init.split('(').next().unwrap_or("").to_owned(),
            false => declared.clone(),
        };
        if ty.is_empty() {
            continue;
        }
        let Some(inner) = init
            .strip_prefix(&format!("{ty}("))
            .and_then(|rest| rest.strip_suffix(')'))
        else {
            continue;
        };
        if inner.contains('(') || inner.contains(',') {
            continue;
        }
        if type_of_member_path(inner, fields, roots, refs).as_deref() != Some(ty.as_str()) {
            continue;
        }
        *line = match declared.is_empty() {
            true => format!("{indent}{name} = {inner};"),
            false => format!("{indent}{declared} {name} = {inner};"),
        };
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// `bool X = <chain>;` read once by the very next condition is that condition's own operand.
///
/// Vanilla branches on the comparison it just made; a name in between makes the compiler
/// materialize the value into a slot and read it back one byte wide before the branch. The value
/// travels in brackets, so a chain mixing `&&` and `||` keeps the grouping it had — mixing them
/// without brackets is a warning here, and warnings are errors.
fn inline_bool_chain_into_next_condition(body: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len().saturating_sub(1) {
            let Some((_, name, value)) = declaration_with_initializer(&lines[index]) else {
                continue;
            };
            if !lines[index].trim_start().starts_with("bool ") {
                continue;
            }
            if !(value.contains("&&") || value.contains("||")) {
                continue;
            }
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                continue;
            }
            let reader = index + 1;
            let trimmed = lines[reader].trim_start();
            if count_ident(&lines[reader], &name) != 1
                || !(trimmed.starts_with("if (") || trimmed.starts_with("return "))
            {
                continue;
            }
            lines[reader] = rename_ident(&lines[reader], &name, &format!("({value})"));
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// A declaration whose value is a LITERAL and which is read exactly once is that literal.
///
/// The compiler materializes a constant into a slot wherever it is used, with or without a name,
/// so moving it costs nothing and cannot reorder anything — but the name standing between a value
/// and the statement that accumulates into it hides the accumulation from the fold that would
/// take the name away.
fn inline_single_use_literals(body: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len() {
            let Some((_, name, init)) = declaration_with_initializer(&lines[index]) else {
                continue;
            };
            let literal = init.parse::<f64>().is_ok()
                || matches!(init.as_str(), "true" | "false" | "nullptr")
                || init
                    .strip_suffix('f')
                    .is_some_and(|head| head.parse::<f64>().is_ok());
            if !literal {
                continue;
            }
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                continue;
            }
            let Some(reader) = (index + 1..lines.len())
                .find(|at| count_ident(&lines[*at], &name) == 1)
            else {
                continue;
            };
            // Only where the name is an OPERAND of an expression. A bare argument may be an
            // out-parameter, and a literal is "Not a valid reference"; a name on the left of `=`
            // is being written, and a literal is not an l-value. Both were measured, 61 errors.
            let reads_as_an_operand = ["*", "+", "-", "/", "<", ">", "==", "!=", "<=", ">="]
                .iter()
                .any(|op| {
                    lines[reader].contains(&format!(" {op} {name}"))
                        || lines[reader].contains(&format!("{name} {op} "))
                });
            if !reads_as_an_operand {
                continue;
            }
            lines[reader] = rename_ident(&lines[reader], &name, &init);
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Slots the function constructs INSIDE an argument list: the `PSF slot; CALLSYS $beh0` pair
/// stands right after a push.
///
/// A declaration constructs its value before the statement that uses it runs. A temporary written
/// into the argument list is constructed while the arguments are being evaluated — after whatever
/// was pushed before it. So the instruction in front of the construction says which of the two the
/// source wrote.
fn argument_constructed_slots(f: &Func, refs: &RefResolver) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let pushes = |name: &str| {
        matches!(
            name,
            "PshV4" | "PshV8" | "PshVPtr" | "PshC4" | "PshC8" | "PshGPtr" | "PshNull" | "PSF"
        )
    };
    let mut out = HashSet::new();
    for (at, pair) in instrs.windows(2).enumerate() {
        if pair[0].op.name != "PSF" || pair[1].op.name != "CALLSYS" || at == 0 {
            continue;
        }
        let ptr = pair[1].qwords.first().copied().unwrap_or(0) as i64;
        if refs.func_by_ptr(ptr) != Some("$beh0") {
            continue;
        }
        if !pushes(instrs[at - 1].op.name) {
            continue;
        }
        if let Some(slot) = pair[0].words.first().map(|word| *word as i16 as i32) {
            if slot > 0 {
                out.insert(slot);
            }
        }
    }
    out
}

/// A declared temporary that vanilla built inside the argument list is written there.
///
/// `FGameplayTag local_4;` standing before the call constructs the value before the call's other
/// arguments are pushed; `Call(…, FGameplayTag(), …)` constructs it among them, which is the order
/// vanilla has. Only where the name is mentioned exactly once after its declaration, that mention
/// is an argument of a call, and the cache says that position is not written through — a
/// parameter taking a non-const reference refuses a temporary outright.
fn spell_out_argument_temporaries(
    text: &str,
    constructed: &HashSet<i32>,
    refs: &RefResolver,
) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len() {
            let Some((_, name)) = bare_declaration(&lines[index]) else {
                continue;
            };
            let Some(slot) = name
                .strip_prefix("local_")
                .and_then(|rest| rest.split('_').next())
                .and_then(|rest| rest.parse::<i32>().ok())
            else {
                continue;
            };
            if !constructed.contains(&slot) {
                continue;
            }
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                continue;
            }
            let Some(reader) =
                (index + 1..lines.len()).find(|at| count_ident(&lines[*at], &name) == 1)
            else {
                continue;
            };
            let Some((callee, arguments)) = call_arguments(&lines[reader]) else {
                continue;
            };
            let rendered = arguments.len();
            let Some(position) = arguments.iter().position(|argument| *argument == name) else {
                continue;
            };
            if refs.arg_position_is_written_through(&callee, rendered, position) {
                continue;
            }
            let head = lines[index].trim().trim_end_matches(';');
            let ty = head[..head.len() - name.len()].trim().to_owned();
            lines[reader] = rename_ident(&lines[reader], &name, &format!("{ty}()"));
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// `float X = <float32 local>;` read once is that local, widened where it is used.
///
/// The alias fold asks for the two types to match, so a declaration that only WIDENS kept its
/// name — and the name costs the copy that fills it, where vanilla widens straight into the
/// arithmetic. Where the widened value was copied ON, `widened` holds the slot and it keeps its
/// name; that is the declaration the source really wrote.
fn fold_widening_aliases(
    text: &str,
    locals: &BTreeMap<i32, String>,
    widened: &HashSet<i32>,
) -> String {
    let slot_of_name = |name: &str| -> Option<i32> {
        name.strip_prefix("local_")
            .and_then(|rest| rest.split('_').next())
            .and_then(|rest| rest.parse::<i32>().ok())
    };
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..lines.len() {
            let Some((_, name, source)) = declaration_with_initializer(&lines[index]) else {
                continue;
            };
            let Some(slot) = slot_of_name(&name) else {
                continue;
            };
            if widened.contains(&slot) {
                continue;
            }
            let head = lines[index].trim().split(" = ").next().unwrap_or("");
            let ty = head[..head.len().saturating_sub(name.len())].trim();
            if !matches!(ty, "float" | "double") {
                continue;
            }
            if slot_of_name(&source).and_then(|from| locals.get(&from)).map(String::as_str)
                != Some("float32")
            {
                continue;
            }
            if lines.iter().map(|line| count_ident(line, &name)).sum::<usize>() != 2 {
                continue;
            }
            let Some(reader) =
                (index + 1..lines.len()).find(|at| count_ident(&lines[*at], &name) == 1)
            else {
                continue;
            };
            lines[reader] = rename_ident(&lines[reader], &name, &source);
            lines.remove(index);
            changed = true;
            break;
        }
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn fold_compound_assignments(
    body: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> String {
    const OPERATORS: [&str; 7] = [" + ", " - ", " * ", " / ", " | ", " & ", " ^ "];
    let pure_member_path = |path: &str| {
        path.contains('.')
            && !path.is_empty()
            && path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':'))
    };
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    let mut at = 0usize;
    while at < lines.len() {
        // Written out in one statement.
        let direct = (|| {
            let statement = lines[at].trim().strip_suffix(';')?;
            let (target, value) = statement.split_once(" = ")?;
            if !pure_member_path(target) {
                return None;
            }
            // The decompiler brackets a value it built and may name the read's own type on the
            // way in: `X.F = (int(X.F) + 1);`. Neither changes what runs — vanilla loads the
            // member's reference ONCE and reads, adds and writes back through it — but both hide
            // the shape from the match. The cast comes off only where the field already HAS that
            // type, so a real narrowing is never rewritten into arithmetic at another width.
            let value = unwrap_brackets(value);
            let rest = value.strip_prefix(target).or_else(|| {
                let ty = type_of_member_path(target, fields, roots, refs);
                if std::env::var_os("GORE_AS_COMPOUND_DIAG").is_some() {
                    eprintln!("[compound] cast {ty:?} for {target} | {value}");
                }
                value.strip_prefix(&format!("{}({target})", ty?))
            })?;
            let operator = OPERATORS.iter().find(|op| rest.starts_with(**op))?;
            let addend = &rest[operator.len()..];
            (!addend.is_empty() && !addend.contains(target))
                .then(|| format!("{}{target} {}= {addend};", indent_of(lines[at]), operator.trim()))
        })();
        if let Some(replacement) = direct {
            kept.push(replacement);
            at += 1;
            continue;
        }
        // Or through a carrier the decompiler put in: read the member, change it, write it back.
        // Three statements, one member path, and a local nothing else in the body touches.
        let refuse = |reason: &str| -> Option<String> {
            if std::env::var_os("GORE_AS_COMPOUND_DIAG").is_some() {
                eprintln!("[compound] {reason} | {}", lines[at].trim());
            }
            None
        };
        let carried = (|| {
            // The read may carry its own declaration: `int local_N = int(X.F);`.
            let (carrier, read) = slot_store(lines[at]).or_else(|| {
                let statement = lines[at].trim().strip_suffix(';')?;
                let (head, value) = statement.split_once(" = ")?;
                let name = head.rsplit(' ').next()?;
                // A local a splitting pass has versioned reads `local_27_2`, which the plain
                // `local_<digits>` test refuses — and those are most of the sites here.
                let named_local = name.strip_prefix("local_").is_some_and(|rest| {
                    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit() || b == b'_')
                });
                (named_local && head.contains(' ')).then(|| (name.to_owned(), value.to_owned()))
            })?;
            let path = read.strip_prefix("int(").and_then(|r| r.strip_suffix(')')).unwrap_or(&read);
            if !pure_member_path(path) {
                return refuse("path");
            }
            // `local_N = local_N <op> <addend>;` — the value reads its own target, which is
            // exactly what `slot_store` refuses, so it is split here.
            let (changed, value) = lines
                .get(at + 1)?
                .trim()
                .strip_suffix(';')?
                .split_once(" = ")?;
            if changed != carrier {
                return refuse("carrier");
            }
            let Some(rest) = value.strip_prefix(carrier.as_str()) else {
                return refuse("self-read");
            };
            let Some(operator) = OPERATORS.iter().find(|op| rest.starts_with(**op)) else {
                return refuse("operator");
            };
            let addend = &rest[operator.len()..];
            // `X.F = local_N;` — the target is a member path, which `slot_store` also refuses.
            let (written, back) = lines
                .get(at + 2)?
                .trim()
                .strip_suffix(';')?
                .split_once(" = ")?;
            if written != path || back != carrier || addend.is_empty() || addend.contains(&carrier)
            {
                return refuse("write-back");
            }
            // The carrier exists only for this round trip: the read, the two in `c = c + n`, and
            // the write-back — four mentions, no more.
            if count_ident(body, &carrier) != 4 {
                return refuse("mentions");
            }
            Some(format!(
                "{}{path} {}= {addend};",
                indent_of(lines[at]),
                operator.trim()
            ))
        })();
        match carried {
            Some(replacement) => {
                kept.push(replacement);
                at += 3;
            }
            None => {
                kept.push(lines[at].to_owned());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// The enum a member PATH names, walked one field at a time. The root is `this`, a local or a
/// parameter — the class's own field map answers the first, the slot table and the parameter list
/// answer the others — and the cache's per-class field types answer every step after that. `None`
/// unless every step resolves and the last one is an enum: a path this cannot follow keeps its
/// cast, because then the conversion may be real.
fn enum_of_member_path(
    path: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> Option<String> {
    let mut steps = path.split('.');
    let root = steps.next()?;
    // The root is `this`, and then the first step is a field of this class — or it is a local or
    // a parameter, and its own declared type starts the walk.
    let mut ty = match root {
        "this" => fields?.get(steps.next()?)?.clone(),
        named => roots.get(named)?.clone(),
    };
    for step in steps {
        ty = refs
            .field_type_by_class(&ty, step)
            .or_else(|| refs.native_field_type(&ty, step))
            // A field declared by a NATIVE class appears in no script class-fields map — the
            // installed `Binds.Cache` is what declares it, and it is read-only evidence.
            .or_else(|| refs.native_field_value_type(&ty, step))?
            .to_owned();
    }
    ty.starts_with('E').then_some(ty)
}

/// Slots a direct member read (`RDR1`/`RDR2`/`RDR4`/`RDR8`) writes. The read puts the member's
/// own value there and converts nothing on the way, so where the body also assigns the slot only
/// once and reads it once, the name in front of it is free to go — even where the type tables
/// cannot follow the path, which is what happens for a field declared by a NATIVE base class.
fn member_read_slots(f: &Func) -> HashMap<i32, u8> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashMap::new();
    };
    instrs
        .iter()
        .filter_map(|ins| {
            let width = match ins.op.name {
                "RDR1" => 1u8,
                "RDR2" => 2,
                "RDR4" => 4,
                "RDR8" => 8,
                _ => return None,
            };
            let slot = ins.words.first().map(|word| *word as i16 as i32)?;
            (slot > 0).then_some((slot, width))
        })
        .collect()
}

/// How wide a declared type is, where the declaration cannot be doing anything but hold the
/// value. `None` for anything this cannot answer, which refuses the fold rather than guess.
fn declared_width(ty: &str) -> Option<u8> {
    match ty {
        "bool" | "int8" | "uint8" => Some(1),
        "int16" | "uint16" => Some(2),
        "int" | "uint" | "float32" => Some(4),
        "float" | "double" | "int64" | "uint64" => Some(8),
        // An enum is byte-backed in this build, and a read of one is `RDR1`.
        name if super::structure::is_enum_name(name) => Some(1),
        _ => None,
    }
}

/// The declared type a member PATH names, by the same walk without the enum requirement.
fn type_of_member_path(
    path: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> Option<String> {
    let mut steps = path.split('.');
    let root = steps.next()?;
    // The first step off `this` is a field of the class, and it may be indexed like any later one.
    let mut ty = match root {
        "this" => {
            let step = steps.next()?;
            let (step, indexed) = match step.split_once('[') {
                Some((name, _)) => (name, true),
                None => (step, false),
            };
            let ty = fields?.get(step)?.clone();
            match indexed {
                true => element_type(&ty)?.to_owned(),
                false => ty,
            }
        }
        named => roots.get(named.split('[').next()?)?.clone(),
    };
    for step in steps {
        // An INDEXED step reads an element: `Freepoints[Idx].WalkSpeed` walks through the array's
        // element type, not the array's own.
        let (step, indexed) = match step.split_once('[') {
            Some((name, _)) => (name, true),
            None => (step, false),
        };
        // A declared type may carry the namespace it was declared in (`G1R::UStoryG1R`) while the
        // class tables are keyed by the bare name. Ask for both, in that order.
        let bare = ty.rsplit("::").next().unwrap_or(&ty).to_owned();
        ty = refs
            .field_type_by_class(&ty, step)
            .or_else(|| refs.native_field_type(&ty, step))
            // A field declared by a NATIVE class appears in no script class-fields map — the
            // installed `Binds.Cache` is what declares it, and it is read-only evidence.
            .or_else(|| refs.native_field_value_type(&ty, step))
            .or_else(|| refs.field_type_by_class(&bare, step))
            .or_else(|| refs.native_field_type(&bare, step))
            .or_else(|| refs.native_field_value_type(&bare, step))?
            .to_owned();
        if indexed {
            ty = element_type(&ty)?.to_owned();
        }
    }
    Some(ty)
}

/// The element type of a container type: `TArray<FFoo>` gives `FFoo`.
fn element_type(ty: &str) -> Option<&str> {
    let inner = ty.split_once('<')?.1.strip_suffix('>')?;
    // Only a single parameter: a map's element is not named by its first one.
    (!inner.contains(',')).then_some(inner.trim())
}

/// `T local_N = this.Member;` read once is that member read where it is read. Vanilla reads a
/// member straight into the instruction that uses it; a name in between costs the slot and a copy
/// out of it.
///
/// The path has to be pure — no call and no index, so reading it twice cannot differ — nothing
/// may assign it between the read and its use, and the slot's declared type has to be the
/// member's own, or the declaration was performing a conversion.
/// Every proper prefix of a member path: `a.b.c` gives `a` and `a.b`.
fn path_prefixes(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut so_far = String::new();
    for step in path.split('.').take(path.split('.').count() - 1) {
        if !so_far.is_empty() {
            so_far.push('.');
        }
        so_far.push_str(step);
        out.push(so_far.clone());
    }
    out
}

fn fold_member_read_temporaries(
    body: &str,
    widened: &HashSet<i32>,
    locals: &BTreeMap<i32, String>,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
    direct_reads: &HashMap<i32, u8>,
) -> String {
    // A path, not a literal: `5.0f` also contains a dot, and reading it as a member path made
    // every float constant look like an unresolvable member.
    let pure_path = |path: &str| {
        // A bare PARAMETER name is a path too: reading it twice has no side effect, exactly like
        // reading a member, and the decompiler names a parameter read the same way it names a
        // member read. `roots` holds the function's locals as well, and a local is NOT one of
        // these: it is a name the source may write again, and treating a copy between two locals
        // as a member read takes the initialisation out from under an accumulator.
        (path.contains('.') || (roots.contains_key(path) && !is_decompiler_local(path)))
            && path.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':'))
    };
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = (|| {
            // The read may stand as an assignment, `local_N = X.F;`, or carry its own
            // declaration, `T local_N = X.F;` — the same read either way. Which of the two the
            // decompiler wrote depends on whether the declaration was hoisted, and that has
            // nothing to do with whether the source named the value.
            let (name, path) = slot_store(lines[at]).or_else(|| {
                declaration_with_initializer(lines[at]).map(|(_, name, init)| (name, init))
            })?;
            let slot = slot_of(&name)?;
            if !pure_path(&path) {
                return None;
            }
            // Either the type tables agree that the declaration converts nothing, or the
            // bytecode says so outright: the slot's only write is the member read itself.
            let member = type_of_member_path(&path, fields, roots, refs);
            // The read's WIDTH has to be the slot's own: `RDR1` into an `int` slot is the
            // widening the declaration performs, and folding it away asks the compiler to
            // convert somewhere else (measured: 25 errors, `int` from `bool`).
            let read_puts_it_there = count_ident(body, &name) == 2
                && locals
                    .get(&slot)
                    .and_then(|ty| declared_width(ty))
                    .is_some_and(|width| direct_reads.get(&slot) == Some(&width));
            // A declaration may WIDEN and still be the same read: `float X = <float32 member>`
            // converts, but so does the use it feeds, and vanilla does it there. What decides is
            // whether the widened value was copied ON — `widened` holds the slots where it was,
            // and those keep their name.
            let widens_only = matches!(
                (
                    locals.get(&slot).map(String::as_str),
                    member.as_deref()
                ),
                (Some("float" | "double"), Some("float32"))
            ) && !widened.contains(&slot);
            if locals.get(&slot) != member.as_ref() && !read_puts_it_there && !widens_only {
                if std::env::var_os("GORE_AS_MEMBER_DIAG").is_some() {
                    eprintln!(
                        "[member] {} slot={:?} member={:?} | {}",
                        match member.is_none() {
                            true => "unresolved",
                            false => "type",
                        },
                        locals.get(&slot),
                        member,
                        lines[at].trim()
                    );
                }
                return None;
            }
            let mut reader = None;
            for (offset, line) in lines[at + 1..].iter().enumerate() {
                if slot_store(line).is_some_and(|(target, _)| target == path)
                    || assignment_target_is_rooted_at_ident(line, &path)
                {
                    break; // the member moved on
                }
                // Leaving the block the read stands in. What the read took is still the same
                // value out there, but the path that names it need not be: a local declared
                // inside the block does not reach past its brace.
                if indent_of(line).len() < indent_of(lines[at]).len() {
                    break;
                }
                // A write to any PREFIX of the path retires the value the read took —
                // `local_32 = nullptr;` between the two makes the moved read a read of null.
                if path_prefixes(&path).iter().any(|prefix| is_definition_line(line, prefix)) {
                    break;
                }
                if count_ident(line, &name) > 0 {
                    reader = (count_ident(line, &name) == 1).then_some(at + 1 + offset);
                    break;
                }
            }
            let reader = reader?;
            if !read_once_at(&lines, at, reader, &name) {
                return None;
            }
            // The name may be a COPY the source made on purpose: a value read out of a const
            // member and then changed. Moving the read back puts the change on the member itself
            // — "Non-const method call on read-only object reference".
            let calls_non_const = super::structure::word_positions(lines[reader], &name)
                .into_iter()
                .filter_map(|at| {
                    let rest = &lines[reader][at + name.len()..];
                    let path = rest.strip_prefix('.')?;
                    let end = path.find('(')?;
                    Some(path[..end].to_owned())
                })
                .any(|path| {
                    let method = path.rsplit('.').next().unwrap_or(&path).to_owned();
                    // Reaching THROUGH a field: only the cache's own const-method list can say,
                    // the way the range-for element check asks it.
                    if path.contains('.') {
                        return !refs.names_a_const_method(&method);
                    }
                    member.as_deref().is_some_and(|ty| {
                        refs.calls_non_const_method(super::structure::bare_type_name(ty), &method)
                    })
                });
            if calls_non_const {
                return None;
            }
            let mut out: Vec<String> =
                lines[at + 1..reader].iter().map(|l| (*l).to_owned()).collect();
            out.push(rename_ident(lines[reader], &name, &path));
            Some((out, reader + 1))
        })();
        match folded {
            Some((replacement, after)) => {
                kept.extend(replacement);
                at = after;
            }
            None => {
                kept.push(lines[at].to_owned());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// `EFoo(int(<path>))` written inline, with no slot to travel through, is `<path>`.
fn rewrite_inline_enum_round_trips(
    body: &str,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
    refs: &RefResolver,
) -> String {
    let mut out = body.to_owned();
    let mut at = 0usize;
    while let Some(found) = out[at..].find("(int(this.") {
        let open = at + found;
        // The enum name in front of the parenthesis, and the path inside the two.
        let head = out[..open].rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
        let name = &out[head.map_or(0, |k| k + 1)..open];
        let inner = open + "(int(".len();
        let close = out[inner..].find("))").map(|k| inner + k);
        match close.filter(|_| !name.is_empty()) {
            Some(close) => {
                let path = out[inner..close].to_owned();
                let resolved = enum_of_member_path(&path, fields, roots, refs);
                if resolved.as_deref() == Some(name) {
                    let start = head.map_or(0, |k| k + 1);
                    out.replace_range(start..close + 2, &path);
                    at = start + path.len();
                    continue;
                }
                at = close + 2;
            }
            None => at = open + 1,
        }
    }
    out
}

/// The slot number a `local_N` identifier names.
fn slot_of(ident: &str) -> Option<i32> {
    ident.strip_prefix("local_")?.parse().ok()
}

/// `local_N = <expr>; return local_N;` is `return <expr>;`. The name is the whole cost: a
/// declaration, a store, a copy back out, and for a value type a destructor that sinks to the end
/// of the function. The source returned the expression.
///
/// A slot whose declared type is not the function's own return type is left alone: there the
/// store IS a conversion, and returning the expression directly would convert somewhere else —
/// or not at all.
fn fold_returned_temporaries(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    ret: &str,
    returns_by_reference: bool,
) -> String {
    if returns_by_reference {
        return body.to_owned();
    }
    let returns_the_same_type = |ident: &str| -> bool {
        let Some(slot) = ident.strip_prefix("local_").and_then(|s| s.parse::<i32>().ok()) else {
            return false;
        };
        // Both names have to be spelled the same way before they can be compared. The slot table
        // holds the BARE class name — the opCast retype writes what `type_by_id` returns — while
        // the return type is rendered with its namespace, so the comparison was dead for every
        // namespaced class (measured: 680 sites, all of them `return Cast<T>(…)`).
        locals
            .get(&slot)
            .is_some_and(|ty| qualify_decl_type(ty, refs) == ret)
    };
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let folded = lines
            .get(at + 1)
            .and_then(|next| next.trim().strip_prefix("return ")?.strip_suffix(';'))
            .filter(|name| returns_the_same_type(name))
            .and_then(|name| {
                let store = lines[at].trim();
                let value = store
                    .strip_prefix(name)?
                    .strip_prefix(" = ")?
                    .strip_suffix(';')?;
                let usable = !value.is_empty()
                    && !value.contains('\u{1}')
                    && !value.contains('\u{2}')
                    && !value.contains(name)
                    && !value.contains("__return")
                    && !value.contains(RVODEF)
                    && indent_of(lines[at]) == indent_of(lines[at + 1]);
                usable.then(|| format!("{}return {value};", indent_of(lines[at])))
            });
        match folded {
            Some(replacement) => {
                kept.push(replacement);
                at += 2;
            }
            None => {
                kept.push(lines[at].to_string());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// The leading whitespace of a line.
fn indent_of(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// An `else` behind an arm that RETURNS is not a branch — the rest of the function is sequential.
/// The structurer decides the shape before the emitter turns a return-slot store into a real
/// `return`, so a block it wrote as an `else` can still be one after the arm has learned to
/// return; the compiler then adds a jump to a join that is the very next instruction, which
/// vanilla never had.
/// Whether the function's exit is a JOIN — the instruction before its final `RET` jumps to that
/// `RET` rather than running into it.
///
/// A returning arm followed by an `else` compiles to exactly that: each arm jumps to a shared
/// epilogue. An arm that falls through has nothing to jump to, so the jump is absent. That makes
/// the last instruction the witness for which of the two shapes the source was written in.
fn epilogue_is_joined(f: &Func) -> bool {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return false;
    };
    let Some(ret) = instrs.iter().rposition(|ins| ins.op.name == "RET") else {
        return false;
    };
    if ret == 0 {
        return false;
    }
    let before = &instrs[ret - 1];
    if before.op.name != "JMP" {
        return false;
    }
    // A jump's operand is a signed dword offset relative to the dword AFTER it.
    let Some(offset) = before.dwords.first().map(|word| *word as i32) else {
        return false;
    };
    let target = before.offset_dw as i64 + 2 + i64::from(offset);
    target == instrs[ret].offset_dw as i64
}

fn drop_else_after_returning_arm(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        let dropped = (|| {
            let indent = indent_of(lines[at]);
            if lines[at].trim() != "}"
                || lines.get(at + 1)?.trim() != "else"
                || lines.get(at + 2)?.trim() != "{"
                || indent_of(lines[at + 1]) != indent
                || indent_of(lines[at + 2]) != indent
            {
                return None;
            }
            // The arm this `}` closes has to end in a return of its own.
            let returns = kept
                .iter()
                .rev()
                .find(|line| !line.trim().is_empty())
                .is_some_and(|line| line.trim_start().starts_with("return"));
            if !returns {
                return None;
            }
            // The matching `}` of the else block, so its body can be lifted one level.
            let mut depth = 0i32;
            let mut close = None;
            for (offset, line) in lines[at + 2..].iter().enumerate() {
                depth += brace_net(line);
                if depth == 0 {
                    close = Some(at + 2 + offset);
                    break;
                }
            }
            let close = close?;
            let body: Vec<String> = lines[at + 3..close]
                .iter()
                .map(|line| line.strip_prefix("    ").unwrap_or(line).to_owned())
                .collect();
            Some((body, close + 1))
        })();
        match dropped {
            Some((lifted, after)) => {
                kept.push(lines[at].to_owned());
                kept.extend(lifted);
                at = after;
            }
            None => {
                kept.push(lines[at].to_owned());
                at += 1;
            }
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// Every arm of an if/else ending in `__return = <val>;`, with the shared `return __return;` as
/// the function's last statement, is each arm returning its own value. The hidden return slot
/// then needs no declaration at all — and vanilla constructs the returned object inside the arm
/// that produces it, not once at the top of the function whatever happens afterwards.
///
/// Only where EVERY store to the slot is the last statement of its block and nothing else reads
/// it: a path that falls through without assigning still needs the shared return.
fn fold_return_slot_arms(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    // The shared exit is spelled `return __return;` or, before that rewrite, the unrecovered
    // default-return marker — both mean the same statement here.
    let Some(last) = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .filter(|at| {
            let tail = lines[*at].trim();
            tail == "return __return;" || tail == format!("return {RVODEF};")
        })
    else {
        return body.to_owned();
    };
    let names_the_slot = lines[last].contains("__return");
    // The shared return has to close an if/else, or some path reaches it without a value.
    if last == 0 || lines[last - 1].trim() != "}" {
        return body.to_owned();
    }
    let stores: Vec<usize> = (0..last)
        .filter(|at| lines[*at].trim().starts_with("__return = "))
        .collect();
    // Exactly the two arms of the if/else the shared return closes, and nothing else. A store
    // nested deeper leaves a path that reaches the shared return without a value — one such
    // function ("Not all paths return a value") is what a looser rule costs.
    // Each store is the last statement of its arm, the two arms are the two halves of ONE
    // if/else, and that if/else is the last thing in the function. The arms may hold anything
    // else before their store; requiring them to hold nothing else missed most of the shape.
    let two_arms = matches!(stores.as_slice(), [then, other]
        if lines.get(then + 1).is_some_and(|line| line.trim() == "}")
            && lines.get(then + 2).is_some_and(|line| line.trim() == "else")
            && lines.get(then + 3).is_some_and(|line| line.trim() == "{")
            && *other > then + 3
            && lines.get(other + 1).is_some_and(|line| line.trim() == "}")
            && other + 2 == last);
    if !two_arms || body.matches("__return").count() != stores.len() + usize::from(names_the_slot)
    {
        return body.to_owned();
    }
    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    for (at, line) in lines.iter().enumerate() {
        if at == last {
            continue;
        }
        match stores.contains(&at) {
            true => {
                let value = line.trim().trim_start_matches("__return = ");
                kept.push(format!("{}return {value}", indent_of(line)));
            }
            false => kept.push((*line).to_owned()),
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// `__return = <val>;` immediately before `return __return;` is one `return <val>;`. The hidden
/// out-slot of a by-value struct return is the compiler's, not the source's: naming it costs a
/// construct at the top of the function and an assignment at every return, which is two
/// instructions vanilla never emitted. Only an adjacent, resolved store folds — a `return
/// __return;` that stands alone still says the path did not provably write the slot.
fn fold_return_slot_stores(body: &str) -> String {
    let mut kept: Vec<String> = Vec::new();
    for line in body.lines() {
        // The default-return marker only becomes `return __return;` further down; both spellings
        // mean the same statement here.
        let returns_the_slot = matches!(line.trim(), "return __return;")
            || line.trim() == format!("return {RVODEF};");
        let folded = returns_the_slot
            .then(|| kept.last().cloned())
            .flatten()
            .and_then(|store| {
                let value = store.trim().strip_prefix("__return = ")?.strip_suffix(';')?;
                let usable = !value.is_empty()
                    && !value.contains('\u{1}')
                    && !value.contains('\u{2}')
                    && !value.contains("__return")
                    && !value.contains(RVODEF);
                let indent: String = store.chars().take_while(|c| c.is_whitespace()).collect();
                usable.then(|| format!("{indent}return {value};"))
            });
        match folded {
            Some(replacement) => {
                kept.pop();
                kept.push(replacement);
            }
            None => kept.push(line.to_string()),
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// AngelScript's `&&` and `||` do not evaluate their right side when the left already decides the
/// answer, and the compiler lowers that to a branch that writes the deciding CONSTANT straight
/// into the expression's result slot — a 4-byte store for `&&`'s `false`, a 1-byte one for
/// `||`'s `true`. Recovered arm by arm it reads as an `if`/`else` over a declared bool, which
/// costs the declaration, a temporary for the constant and a copy.
///
/// `if (c) { X = false; } else { X = b; }` is `X = !(c) && b;`
/// `if (c) { X = true;  } else { X = b; }` is `X = c || b;`
///
/// The condition may well be the slot itself — `if (!(X)) { X = false; } else { X = b; }` is
/// `X = X && b`, which is what a CHAIN of `&&` lowers to, one link at a time. The slot is read
/// before it is written there, so folding it is the same statement, not a cycle.
///
/// `&&` and `||` take BOOL operands, and an operand that is not one is not merely refused — it
/// takes the compiler down (measured: the whole tree, twice). So both sides are checked against
/// the cache before anything is folded.
fn fold_short_circuits(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    let mut at = 0usize;
    while at < lines.len() {
        if let Some((folded, after)) = short_circuit(&lines, at, locals, refs, fields, roots) {
            kept.push(folded);
            at = after;
            continue;
        }
        kept.push(lines[at].to_string());
        at += 1;
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// The eight lines of a short circuit starting at `at`, rendered as one statement, plus the index
/// just past them.
fn short_circuit(
    lines: &[&str],
    at: usize,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
    roots: &HashMap<String, String>,
) -> Option<(String, usize)> {
    let condition = lines.get(at)?.trim().strip_prefix("if (")?.strip_suffix(')')?;
    if lines.get(at + 1)?.trim() != "{" {
        return None;
    }
    let then_end = block_end(lines, at + 1)?;
    if lines.get(then_end + 1)?.trim() != "else" || lines.get(then_end + 2)?.trim() != "{" {
        return None;
    }
    let else_end = block_end(lines, then_end + 2)?;
    // The deciding arm is one store of the constant that settles the whole expression.
    if then_end != at + 3 {
        sc_reject("then-arm", lines[at]);
        return None;
    }
    let (target, deciding) = slot_store(lines.get(at + 2)?)?;
    // The value arm ends in the store to the same slot. Where it takes a step through a
    // temporary of its own first — which is how the compiler evaluates the right-hand operand —
    // that step IS the value, and vanilla spends the same slot for it.
    let (else_target, value) = slot_store(lines.get(else_end - 1)?)?;
    let value = match else_end - (then_end + 3) {
        1 => value,
        2 => {
            let (carrier, carried) = slot_store(lines.get(then_end + 3)?)?;
            let arm = lines[then_end + 3..else_end].join("\n");
            match carrier == value && count_ident(&arm, &carrier) == 2 {
                true => carried,
                false => {
                    sc_reject("carrier", lines[at]);
                    return None;
                }
            }
        }
        _ => {
            sc_reject("else-arm", lines[at]);
            return None;
        }
    };
    if target != else_target {
        sc_reject("target", lines[at]);
        return None;
    }
    // BOTH operands have to be bool: the slot the expression writes, and the value the other arm
    // gives it. The condition is one already — it is what the branch tested.
    // A PARAMETER carries its type in the signature, not in the slot table: `const bool
    // bIsImmortal` is as much a bool as any local, and asking only the locals left the outer arm
    // of `A || bIsImmortal` standing as an if/else over a carrier.
    let value_is_bool = renders_a_bool(&value, locals, refs, fields)
        || roots.get(value.as_str()).is_some_and(|ty| ty == "bool");
    if temporary_type(locals, &target) != Some("bool") || !value_is_bool {
        sc_reject(
            match temporary_type(locals, &target) {
                Some("bool") => "value-not-bool",
                _ => "target-not-bool",
            },
            lines[at],
        );
        return None;
    }
    if [condition, value.as_str()]
        .iter()
        .any(|part| part.chars().any(char::is_control))
    {
        return None;
    }
    let (left, operator) = match deciding.as_str() {
        // The left operand of `&&` is the condition the branch did NOT take. Turning the
        // comparison around is what vanilla wrote; wrapping it in `!` makes the compiler
        // materialize the negation instead of inverting the jump (measured: `CmpPtrNull; TZ;
        // CpyRtoV4; NOT` for what vanilla did with `CmpPtrNull; JNZ`).
        "false" => (turned_around(condition), "&&"),
        "true" => (condition.to_owned(), "||"),
        _ => return None,
    };
    let left = parenthesize_mixed(&left, operator);
    let value = parenthesize_mixed(&value, operator);
    let indent = indent_of(lines[at]);
    Some((
        format!("{indent}{target} = {left} {operator} {value};"),
        else_end + 1,
    ))
}

/// One side of a logical operator, parenthesized where it holds the OTHER one. Mixing `&&` and
/// `||` without parentheses is a warning here, and the game compiles warnings as errors — so the
/// precedence that was already meant has to be written down.
fn parenthesize_mixed(part: &str, operator: &str) -> String {
    let other = match operator {
        "&&" => "||",
        _ => "&&",
    };
    match part.contains(other) {
        true => format!("({part})"),
        false => part.to_owned(),
    }
}

/// Why a short circuit was not folded, behind `GORE_AS_SC_DIAG`.
fn sc_reject(reason: &str, line: &str) {
    if std::env::var_os("GORE_AS_SC_DIAG").is_some() {
        eprintln!("[sc-reject] {reason} | {}", line.trim());
    }
}

/// The index of the `}` closing the `{` at `open`, or None if the block does not close inside
/// `lines`.
fn block_end(lines: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, line) in lines.iter().enumerate().skip(open) {
        match line.trim() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// `X = <a>; X = X && <b>;` is `X = <a> && <b>;`. A chain of `&&` lowers to one branch per link,
/// and each link recovers as its own statement writing the same slot — which then costs a copy
/// per link. Only a slot the first statement does not itself read is joined, and the left side is
/// parenthesized where the two operators would otherwise re-bind it.
fn join_short_circuit_chains(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<String> = Vec::new();
    for line in lines {
        let joined = (|| {
            // Parsed here rather than through `slot_store`, which refuses a store that reads its
            // own target — and reading its own target is exactly what a chain link does.
            let (target, value) = line.trim().strip_suffix(';')?.split_once(" = ")?;
            if !is_local_ident(target) {
                return None;
            }
            let previous = kept.last()?;
            let (before, first) = slot_store(previous)?;
            if before != target || count_ident(&first, target) != 0 {
                return None;
            }
            let (operator, rest) = ["&&", "||"]
                .iter()
                .find_map(|op| Some((*op, value.strip_prefix(&format!("{target} {op} "))?)))?;
            // Mixing `&&` and `||` without parentheses is a WARNING here, and warnings are
            // errors (measured: 10 of them). Either side that holds the other operator gets its
            // own parentheses — which is what the precedence already meant, said out loud.
            let left = parenthesize_mixed(&first, operator);
            let rest = parenthesize_mixed(rest, operator);
            Some(format!(
                "{}{target} = {left} {operator} {rest};",
                indent_of(previous)
            ))
        })();
        match joined {
            Some(replacement) => {
                kept.pop();
                kept.push(replacement);
            }
            None => kept.push(line.to_string()),
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// A condition with its comparison turned around. Only a single top-level relation is turned;
/// anything else keeps its `!`, which is at least correct.
fn turned_around(condition: &str) -> String {
    if let Some(inner) = condition.strip_prefix("!(").and_then(|c| c.strip_suffix(')')) {
        return inner.to_owned();
    }
    if !condition.contains("&&") && !condition.contains("||") {
        for (operator, turned) in [
            (" <= ", " > "),
            (" >= ", " < "),
            (" < ", " >= "),
            (" > ", " <= "),
            (" == ", " != "),
            (" != ", " == "),
        ] {
            if let Some(at) = condition.find(operator) {
                return format!(
                    "{}{turned}{}",
                    &condition[..at],
                    &condition[at + operator.len()..]
                );
            }
        }
    }
    format!("!({condition})")
}

/// Whether a rendered value is a bool: a slot the type table calls one, a bool literal, or a call
/// every declaration of that name returns bool from. An operand that is not one may not carry a
/// `&&` — and the compiler does not merely refuse it, it goes down.
fn renders_a_bool(
    value: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    fields: Option<&HashMap<String, String>>,
) -> bool {
    if matches!(value, "true" | "false") || temporary_type(locals, value) == Some("bool") {
        return true;
    }
    // A field of the class carries its type in the class's own map — the slot table knows
    // nothing about `this.bFlag`.
    if let Some(field) = value.strip_prefix("this.") {
        if fields.and_then(|map| map.get(field)).is_some_and(|ty| ty == "bool") {
            return true;
        }
    }
    // A negation and a comparison are bools whatever they wrap, and a fully parenthesized value
    // is whatever it holds.
    if let Some(negated) = value.strip_prefix('!') {
        return renders_a_bool(negated, locals, refs, fields);
    }
    if value.starts_with('(')
        && matching_paren(value, 0) == Some(value.len() - 1)
        && value.len() > 2
    {
        return renders_a_bool(&value[1..value.len() - 1], locals, refs, fields);
    }
    if [" == ", " != ", " < ", " > ", " <= ", " >= ", " && ", " || "]
        .iter()
        .any(|operator| value.contains(operator))
    {
        return true;
    }
    outer_callee(value).is_some_and(|callee| refs.names_returning(&callee) == Some("bool"))
}

/// `local_N = <value>;` — a plain assignment to a bare local, with no declaration in front.
fn slot_store(line: &str) -> Option<(String, String)> {
    let (target, value) = line.trim().strip_suffix(';')?.split_once(" = ")?;
    (is_local_ident(target) && count_ident(value, target) == 0 && !value.is_empty())
        .then(|| (target.to_owned(), value.to_owned()))
}

/// `local_N = <expr>; local_N = !local_N;` is one `local_N = !(<expr>);`. The slot is the
/// compiler's own temporary for the negation, and the round trip through it costs a copy in and
/// a copy out that vanilla never spent — it applies `NOT` to the value where it already sits.
fn fold_negated_stores(body: &str) -> String {
    let mut kept: Vec<String> = Vec::new();
    for line in body.lines() {
        let folded = negated_self_store(line)
            .and_then(|slot| {
                let previous = kept.last()?;
                let (target, value) = previous.trim().strip_suffix(';')?.split_once(" = ")?;
                let declares = target.split_whitespace().last()? == slot.as_str();
                // A value that is ALREADY a negation folds too: `X = !(e); X = !X;` is the
                // double negation vanilla wrote, and it emits `NOT` twice on the one slot. Kept
                // apart, the two negations travel through a name and cost a copy each way.
                (declares && count_ident(value, &slot) == 0).then(|| {
                    let indent: String =
                        previous.chars().take_while(|c| c.is_whitespace()).collect();
                    format!("{indent}{target} = !({value});")
                })
            });
        match folded {
            Some(replacement) => {
                kept.pop();
                kept.push(replacement);
            }
            None => kept.push(line.to_string()),
        }
    }
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// The slot a statement negates in place (`local_5 = !local_5;`).
fn negated_self_store(line: &str) -> Option<String> {
    let (target, value) = line.trim().strip_suffix(';')?.split_once(" = ")?;
    let negated = value.strip_prefix('!')?;
    (is_local_ident(target) && negated == target).then(|| target.to_owned())
}

/// Drop what follows a statement that always leaves the block. Recovering a branch's own return
/// leaves behind the statement the shared exit used to render, and the compiler treats
/// "Unreachable code" as an error.
fn drop_unreachable_statements(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut at = 0usize;
    strip_unreachable(&lines, &mut at, &mut kept);
    drop_return_after_exhaustive_switch(&mut kept);
    let mut joined = kept.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// Copy statements until this block's closing brace (left for the caller) or the end of the
/// body. Returns true when the run always leaves the block — a `return`/`break`/`continue`, or
/// an `if`/`else` whose arms BOTH leave.
/// A bare `return;` after a `switch` that has a `default:` and no `break` anywhere in it: every
/// arm leaves through a `return`, so the switch never falls out the bottom and the statement
/// behind it is unreachable — which this compiler reports as a warning, and treats as an error.
///
/// The `break`-free requirement is what makes "every arm returns" safe to conclude without a
/// control-flow graph: an arm that neither breaks nor returns would fall through to the next arm,
/// and the last arm is the `default`.
fn drop_return_after_exhaustive_switch(kept: &mut Vec<&str>) {
    let Some(last) = kept.len().checked_sub(1) else {
        return;
    };
    if kept[last].trim() != "return;" || last == 0 || kept[last - 1].trim() != "}" {
        return;
    }
    let mut depth = 0i32;
    for at in (0..last).rev() {
        depth += kept[at].matches('}').count() as i32;
        depth -= kept[at].matches('{').count() as i32;
        if depth != 0 {
            continue;
        }
        // `at` opens the block the `}` above closed; the `switch (` heads it.
        if !at
            .checked_sub(1)
            .is_some_and(|line| kept[line].trim().starts_with("switch ("))
        {
            return;
        }
        let arms = &kept[at..last];
        if arms.iter().any(|line| line.trim().starts_with("break"))
            || !arms.iter().any(|line| line.trim() == "default:")
        {
            return;
        }
        kept.truncate(last);
        return;
    }
}

fn strip_unreachable<'a>(lines: &[&'a str], at: &mut usize, kept: &mut Vec<&'a str>) -> bool {
    let mut leaves = false;
    while *at < lines.len() {
        let line = lines[*at];
        let trimmed = line.trim();
        if trimmed == "}" {
            break;
        }
        let mut dead = Vec::new();
        let out = if leaves { &mut dead } else { &mut *kept };
        out.push(line);
        *at += 1;
        let mut statement_leaves = trimmed.starts_with("return ")
            || trimmed == "return;"
            || trimmed == "break;"
            || trimmed == "continue;";
        if lines.get(*at).map(|l| l.trim()) == Some("{") {
            out.push(lines[*at]);
            *at += 1;
            let then_leaves = strip_unreachable(lines, at, out);
            if let Some(close) = lines.get(*at) {
                out.push(close);
                *at += 1;
            }
            let mut arms_leave = false;
            if lines.get(*at).map(|l| l.trim()) == Some("else") {
                out.push(lines[*at]);
                *at += 1;
                if lines.get(*at).map(|l| l.trim()) == Some("{") {
                    out.push(lines[*at]);
                    *at += 1;
                    let else_leaves = strip_unreachable(lines, at, out);
                    if let Some(close) = lines.get(*at) {
                        out.push(close);
                        *at += 1;
                    }
                    arms_leave = then_leaves && else_leaves;
                }
            }
            // Only a two-armed `if` leaves: a loop may run zero times, and a one-armed `if`
            // has a path around it.
            statement_leaves = arms_leave && trimmed.starts_with("if (");
        }
        leaves |= statement_leaves;
    }
    leaves
}

/// Diagnostic counter for why an argument could not be moved back into its call.
fn inline_reject(reason: &str, callee: &str, temp: &str, statement: &str) {
    if std::env::var_os("GORE_AS_INLINE_DIAG").is_some() {
        eprintln!("[inline-reject] {reason} {callee} {temp} | {}", statement.trim());
    }
}

/// The parameter NAMES of a rendered signature line.
fn signature_parameters(signature: &str) -> Vec<String> {
    let Some(open) = signature.find('(') else {
        return Vec::new();
    };
    let Some(close) = matching_paren(signature, open) else {
        return Vec::new();
    };
    signature[open + 1..close]
        .split(',')
        .filter_map(|parameter| {
            let name = parameter
                .split_whitespace()
                .last()?
                .trim_start_matches('&')
                .trim_end_matches(')');
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

/// `this.<Field> = <value>;` -> (field, value). Only a direct member of `this`.
fn member_store(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().strip_suffix(';')?;
    let (target, value) = trimmed.split_once(" = ")?;
    let field = target.strip_prefix("this.")?;
    (!field.contains(['.', '(', '[', ' ']) && !value.is_empty())
        .then(|| (field.to_owned(), value.to_owned()))
}

/// The `FName` literal a global's initializer builds it from: `PshC4 <static-name id>` followed
/// by `CALLSYS __STATIC_NAME`. Any other shape yields None, and the caller keeps its fallback.
fn fname_initializer(global: &super::model::Global, refs: &RefResolver) -> Option<String> {
    let init = global.init.as_ref()?;
    let instrs = disassemble(&init.bytecode).ok()?;
    let mut pushed: Option<i64> = None;
    for ins in &instrs {
        match ins.op.name {
            "PshC4" => {
                pushed = ins.dwords.first().map(|value| *value as i64);
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                if refs.func_by_ptr(ptr) == Some("__STATIC_NAME") {
                    return pushed
                        .and_then(|id| refs.static_name(id))
                        .map(str::to_owned);
                }
                pushed = None;
            }
            _ => {}
        }
    }
    None
}

/// `!(!(x))` is what a recovered double branch reads like; the source tested `x`. The two
/// negations survive into the bytecode as two `NOT`s where vanilla branches on the value itself.
fn fold_double_negations(body: &str) -> String {
    if !body.contains("!(!(") {
        return body.to_owned();
    }
    let trailing_newline = body.ends_with('\n');
    let mut out: Vec<String> = Vec::new();
    for line in body.lines() {
        let mut line = line.to_owned();
        // Bound: every pass removes one `!(!(`, and the line only gets shorter.
        while let Some(at) = line.find("!(!(") {
            let inner_open = at + 3;
            let Some(inner_close) = matching_paren(&line, inner_open) else {
                break;
            };
            if line.as_bytes().get(inner_close + 1) != Some(&b')') {
                break;
            }
            let inner = line[inner_open + 1..inner_close].to_owned();
            line = format!("{}{inner}{}", &line[..at], &line[inner_close + 2..]);
        }
        out.push(line);
    }
    let mut joined = out.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// `(0 != 0)` is how a folded bool constant reads; the source said `false`. The compiler folds
/// both to the same constant, but only the literal keeps the one-byte store vanilla emitted
/// instead of a compare and a test.
fn fold_constant_comparisons(body: &str) -> String {
    const COMPARISONS: [(&str, &str); 6] = [
        ("(0 != 0)", "false"),
        ("(1 != 0)", "true"),
        ("(0 == 0)", "true"),
        ("(1 == 0)", "false"),
        // A bool literal folded into an int->bool wrap: `(true != 0)` does not compile at all.
        ("(true != 0)", "true"),
        ("(false != 0)", "false"),
    ];
    if !COMPARISONS
        .iter()
        .any(|(pattern, _)| body.contains(pattern))
    {
        return body.to_owned();
    }
    let mut out = body.to_owned();
    for (pattern, literal) in COMPARISONS {
        out = out.replace(pattern, literal);
    }
    // A control-flow condition keeps its parentheses: the comparison WAS the whole condition,
    // and `while false` does not parse.
    let trailing_newline = out.ends_with('\n');
    let mut fixed: Vec<String> = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim_start();
        let mut replaced = line.to_owned();
        for keyword in ["if ", "while ", "switch "] {
            if let Some(rest) = trimmed.strip_prefix(keyword) {
                if rest == "true" || rest == "false" {
                    replaced = format!("{}{keyword}({rest})", leading_indent(line));
                }
            }
        }
        fixed.push(replaced);
    }
    let mut joined = fixed.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// A plain numeric or boolean literal — nothing that could carry a side effect.
fn is_foldable_literal(value: &str) -> bool {
    if value == "true" || value == "false" {
        return true;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    let digits = digits
        .strip_suffix('f')
        .filter(|rest| rest.contains('.'))
        .unwrap_or(digits);
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && digits.bytes().filter(|b| *b == b'.').count() <= 1
}

/// The statement assigns THROUGH the ident (`local_5.Field = x;` or `local_5 = x;`).
fn assignment_target_is_rooted_at_ident(statement: &str, ident: &str) -> bool {
    let trimmed = statement.trim_start();
    let Some(rest) = trimmed.strip_prefix(ident) else {
        return false;
    };
    rest.split(" = ")
        .next()
        .is_some_and(|target| !target.contains('('))
        && rest.contains(" = ")
}

/// A store into a slot the rendered source never reads back: the structurer resolved the value at
/// its use site (a float literal, say) and left the raw producer behind. The store is dead in the
/// emitted source, and keeping it costs a local the original never declared. Drop it only when the
/// right-hand side is a pure expression, so no call can be lost with it.
fn drop_dead_stores(body: &str) -> String {
    let trailing_newline = body.ends_with('\n');
    let live = used_locals(body);
    let mut dead: HashSet<i32> = HashSet::new();
    for slot in live {
        let ident = format!("local_{slot}");
        let mut reads = 0usize;
        let mut stores = 0usize;
        for line in body.lines() {
            let hits = count_ident(line, &ident);
            if hits == 0 {
                continue;
            }
            match assignment_rhs_for(line, &ident) {
                Some(rhs) if is_simple_pure_expr(rhs) => stores += 1,
                _ => reads += hits,
            }
        }
        if reads == 0 && stores > 0 {
            dead.insert(slot);
        }
    }
    if dead.is_empty() {
        return body.to_owned();
    }
    let mut out: Vec<&str> = Vec::new();
    for line in body.lines() {
        let drop_it = dead.iter().any(|slot| {
            let ident = format!("local_{slot}");
            assignment_rhs_for(line, &ident).is_some()
        });
        if !drop_it {
            out.push(line);
        }
    }
    let mut joined = out.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    joined
}

/// Every store into the slot must be a call or constructor expression whose value already has the
/// slot's declared type. A store of a bare literal is a declaration-site conversion — `FText x =
/// "id";` — and folding the literal into the consumer would hand it the raw `FString` instead.
fn produced_only_by_calls(body: &str, slot: i32) -> bool {
    let ident = format!("local_{slot}");
    let mut stores = 0usize;
    for line in body.lines() {
        let Some(rhs) = assignment_rhs_for(line, &ident) else {
            continue;
        };
        stores += 1;
        if !rhs.ends_with(')') || !rhs.contains('(') || rhs.starts_with('"') {
            return false;
        }
    }
    stores > 0
}

/// True when the base cache builds a value of this type the way the STRUCTURER recovered it —
/// default-construct, then assign. Turning that into a declaration-with-initializer would ask
/// for a copy constructor the cache has no row for, and the module would stop being splicable.
/// The reverse case (no default constructor, no `opAssign`) is why the rewrite exists at all,
/// so the cache's own function table decides which shape each type gets.
fn constructs_by_assignment(ty: &str, refs: &RefResolver) -> bool {
    // Not `bare_type_name`: it splits on the LAST `::`, which cuts a template's subtype apart
    // (`TSubclassOf<G1R::X>` -> `X>`). `type_has_method` strips the namespaces itself.
    let ty = ty.trim_start_matches("const ");
    !refs.type_has_method(ty, "$beh0", 1)
        && refs.type_has_method(ty, "$beh0", 0)
        && refs.type_has_method(ty, "opAssign", 1)
}

fn is_value_struct_type(ty: &str) -> bool {
    !is_primitive(ty)
        && !ty.starts_with("const ")
        && matches!(
            super::structure::bare_type_name(ty).bytes().next(),
            Some(b'F') | Some(b'T')
        )
}

/// `Iterator()` / `CanProceed` / `Proceed()` is what `for (auto X : container)` desugars to. The
/// structurer recovers that desugared shape faithfully, but writing it back out has to NAME the
/// iterator, and a named iterator is copy-constructed — a `$beh0(const T&)` the base cache has no
/// row for, which costs the module its splicability. Fold the idiom back into the range-for the
/// source actually wrote. Returns the rewritten body plus the element slots whose hoisted
/// declaration the loop header now owns.
/// One line per refused range-for, behind `GORE_AS_FOREACH_DIAG`, so the reasons can be counted
/// over the whole corpus instead of guessed at from one example.
fn foreach_reject(reason: &str) {
    if std::env::var_os("GORE_AS_FOREACH_DIAG").is_some() {
        eprintln!("[foreach-reject] {reason}");
    }
}

fn rewrite_foreach_loops(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
) -> (String, HashSet<i32>) {
    let trailing_newline = body.ends_with('\n');
    let lines: Vec<&str> = body.lines().collect();
    let mut drop_line = vec![false; lines.len()];
    let mut replace: Vec<Option<String>> = vec![None; lines.len()];
    let mut suppressed = HashSet::new();

    // Every place that has the idiom's SHAPE, found before any of them is judged. A function may
    // run the same slot through two loops one after the other — the compiler reuses the frame —
    // and each loop's own `elem = it.Proceed();` would otherwise read, to the other, as a mention
    // of the element outside its loop. Measured: 71 of the refusals here were exactly that.
    let candidates: Vec<(usize, usize, String)> = (0..lines.len())
        .filter(|i| i + 3 < lines.len())
        .filter_map(|i| {
            let (iter, _) = iterator_decl(lines[i])?;
            (lines[i + 1].trim() == format!("while (local_{iter}.CanProceed)")
                && lines[i + 2].trim() == "{")
            .then_some(())?;
            let elem = proceed_assignment(lines[i + 3], iter)?;
            let end = matching_close(&lines, i + 2)?;
            Some((i, end, format!("local_{elem}")))
        })
        .collect();

    for i in 0..lines.len() {
        if i + 3 >= lines.len() || drop_line[i] {
            continue;
        }
        let Some((iter, container)) = iterator_decl(lines[i]) else {
            continue;
        };
        if lines[i + 1].trim() != format!("while (local_{iter}.CanProceed)")
            || lines[i + 2].trim() != "{"
        {
            foreach_reject("not-the-idiom-shape");
            continue;
        }
        let Some(elem) = proceed_assignment(lines[i + 3], iter) else {
            foreach_reject("no-proceed-assignment");
            continue;
        };
        let Some(end) = matching_close(&lines, i + 2) else {
            continue;
        };
        // The iterator may appear nowhere but the three idiom lines, and the element nowhere
        // outside the loop body: the range-for scopes both, so any other reference would be to
        // a name that no longer exists.
        let iter_ident = format!("local_{iter}");
        let elem_ident = format!("local_{elem}");
        let iter_uses: usize = lines.iter().map(|l| count_ident(l, &iter_ident)).sum();
        // The element's own bare declaration is the one mention outside the loop that does not
        // count: the range-for header declares the element itself, so that line is what the
        // header REPLACES. Measured over the corpus, it is the only thing standing outside for
        // 100 of the loops that were refused here.
        let hoisted_declaration = lines
            .iter()
            .position(|line| {
                bare_declaration(line).is_some_and(|(_, name)| name == elem_ident)
            })
            .filter(|at| *at < i + 3 || *at > end);
        // A mention inside ANOTHER loop that runs the same element is that loop's own.
        let owned_elsewhere = |n: usize| {
            candidates.iter().any(|(start, close, elem)| {
                *elem == elem_ident && *start != i && n >= *start && n <= *close
            })
        };
        let elem_outside: usize = lines
            .iter()
            .enumerate()
            .filter(|(n, _)| *n < i + 3 || *n > end)
            .filter(|(n, _)| Some(*n) != hoisted_declaration && !owned_elsewhere(*n))
            .map(|(_, l)| count_ident(l, &elem_ident))
            .sum();
        if iter_uses != 3 || elem_outside != 0 {
            if iter_uses != 3 {
                foreach_reject("iterator-mentioned-elsewhere");
            } else {
                let outside = lines
                    .iter()
                    .enumerate()
                    .filter(|(n, _)| *n < i + 3 || *n > end)
                    .find(|(_, l)| count_ident(l, &elem_ident) > 0)
                    .map(|(_, l)| l.trim())
                    .unwrap_or("");
                foreach_reject(&format!("element-mentioned-outside | {outside}"));
            }
            continue;
        }
        // The range-for element is READ-ONLY. A body that writes through it, or calls a method
        // the cache records as non-const, only compiles in the while-shape the structurer
        // recovered — so that loop keeps it.
        if element_is_written_through(
            &lines[i + 4..end],
            &elem_ident,
            locals.get(&elem).map(String::as_str),
            refs,
        ) {
            foreach_reject("element-written-through");
            continue;
        }
        // A range-for element is not assignable, so every write to it inside the loop has to be
        // the compiler's own handle release. One that is anything else means this is not the
        // idiom it looks like, and the loop keeps its recovered while-shape.
        let releases: Vec<usize> = (i + 4..end)
            .filter(|n| assignment_rhs_for(lines[*n], &elem_ident).is_some())
            .collect();
        if releases
            .iter()
            .any(|n| assignment_rhs_for(lines[*n], &elem_ident) != Some("nullptr"))
        {
            continue;
        }
        let indent = leading_indent(lines[i]);
        replace[i] = Some(format!("{indent}for (auto {elem_ident} : {container})"));
        drop_line[i + 1] = true;
        drop_line[i + 3] = true;
        for n in releases {
            drop_line[n] = true;
        }
        // The header owns the element now, so the declaration it replaces goes with the rest.
        if let Some(at) = hoisted_declaration {
            drop_line[at] = true;
        }
        suppressed.insert(elem);
    }

    if suppressed.is_empty() {
        return inline_foreach_containers(body);
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for (n, line) in lines.iter().enumerate() {
        if drop_line[n] {
            continue;
        }
        out.push(replace[n].clone().unwrap_or_else(|| (*line).to_owned()));
    }
    let mut joined = out.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    let (joined, inlined) = inline_foreach_containers(&joined);
    suppressed.extend(inlined);
    (joined, suppressed)
}

/// The structurer materializes the iterated container into its own slot. Vanilla iterated the
/// member directly, and the extra slot costs a container copy the base cache has no behaviour for.
/// Inline it when the local is written once from a pure path, read only by the loop header, and
/// the loop body never touches that path — so iterating the member instead of a copy of it cannot
/// observe a mutation the copy would have hidden.
fn inline_foreach_containers(body: &str) -> (String, HashSet<i32>) {
    let lines: Vec<&str> = body.lines().collect();
    let mut drop_line = vec![false; lines.len()];
    let mut replace: Vec<Option<String>> = vec![None; lines.len()];
    let mut inlined: HashSet<i32> = HashSet::new();

    for i in 0..lines.len() {
        let trimmed = lines[i].trim();
        let Some(rest) = trimmed.strip_prefix("for (auto ") else {
            continue;
        };
        let Some((elem, container)) = rest.strip_suffix(')').and_then(|r| r.split_once(" : "))
        else {
            continue;
        };
        if !container.starts_with("local_") {
            continue;
        }
        let uses: usize = lines.iter().map(|l| count_ident(l, container)).sum();
        if uses != 2 {
            continue;
        }
        let Some((decl, path)) = lines
            .iter()
            .enumerate()
            .find(|(n, l)| *n != i && count_ident(l, container) == 1)
            .and_then(|(n, l)| container_decl_path(l, container).map(|p| (n, p)))
        else {
            continue;
        };
        let Some(end) = matching_close(&lines, i + 1) else {
            continue;
        };
        // The loop body must not touch the container's own path: iterating the member (or the
        // getter's result) instead of a copy of it would otherwise observe a mutation the copy
        // hid. For a call that is the callee, plus a local receiver — `this` is deliberately not
        // watched, since almost every body mentions it.
        let callee = path.split('(').next().unwrap_or(&path);
        let receiver = callee
            .rsplit_once('.')
            .map(|(head, _)| head)
            .filter(|head| head.starts_with("local_"));
        if lines[i + 1..=end]
            .iter()
            .any(|l| l.contains(callee) || receiver.is_some_and(|r| l.contains(r)))
        {
            continue;
        }
        let Some(slot) = container
            .strip_prefix("local_")
            .and_then(|rest| rest.parse::<i32>().ok())
        else {
            continue;
        };
        let indent = leading_indent(lines[i]);
        replace[i] = Some(format!("{indent}for (auto {elem} : {path})"));
        drop_line[decl] = true;
        // The local has no definition left anywhere, so its hoisted declaration has to go too —
        // a bare declaration of a container type asks for a default constructor the base cache
        // may not have, and the module stops being splicable over it.
        inlined.insert(slot);
    }

    if inlined.is_empty() {
        return (body.to_owned(), inlined);
    }
    let trailing_newline = body.ends_with('\n');
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for (n, line) in lines.iter().enumerate() {
        if drop_line[n] {
            continue;
        }
        out.push(replace[n].clone().unwrap_or_else(|| (*line).to_owned()));
    }
    let mut joined = out.join("\n");
    if trailing_newline {
        joined.push('\n');
    }
    (joined, inlined)
}

/// `TMap<A, B> local_N = this.m_Field;` -> the right-hand pure member path.
///
/// A CALL is deliberately not accepted here. Iterating a getter's result instead of the local the
/// compiler materialized changes which `Iterator()` overload the recompile picks, and the pick is
/// not decidable from the cache: for one module the call form is the one the base cache has a row
/// for, for another it is the local form, and the return type's own reference/const flags predict
/// neither. Both shapes were measured; the local form is the one the structurer recovered from the
/// bytecode, so it stays.
fn container_decl_path(line: &str, ident: &str) -> Option<String> {
    let trimmed = line.trim().strip_suffix(';')?;
    let (lhs, rhs) = trimmed.split_once(" = ")?;
    if !lhs.ends_with(ident) || rhs.is_empty() {
        return None;
    }
    rhs.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
        .then(|| rhs.to_owned())
}

/// True when the loop body writes through `ident` — assigns into it, calls through one of its
/// fields, or calls a method the cache declares non-const on its type. Unknown (native) types
/// answer only through the shape rules, which is why a field-then-call counts as a write.
fn element_is_written_through(
    body: &[&str],
    ident: &str,
    ty: Option<&str>,
    refs: &RefResolver,
) -> bool {
    for line in body {
        let trimmed = line.trim();
        // The compiler's own handle release is not a source write; the rewrite drops it.
        if trimmed == format!("{ident} = nullptr;") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(ident) {
            let target = rest.split(" = ").next().unwrap_or(rest);
            if rest.len() > target.len() && !target.contains('(') {
                return true; // `local_E… = …;`
            }
        }
        // Handing the element to a parameter a TEMPORARY could not bind to means the parameter
        // is a reference and the callee can write through it — and a range-for element is
        // read-only ("No matching signatures", measured).
        if let Some((callee, arguments)) = call_arguments(line) {
            let rendered = arguments.len();
            if arguments.iter().enumerate().any(|(position, argument)| {
                argument == ident
                    && refs.arg_position_is_written_through(&callee, rendered, position)
            }) {
                return true;
            }
        }
        for at in super::structure::word_positions(line, ident) {
            let rest = &line[at + ident.len()..];
            let Some(call) = rest.find('(') else {
                continue;
            };
            let path = &rest[..call];
            if !path.starts_with('.') || path.contains([' ', ',', ')', '[']) {
                continue;
            }
            let dots = path.bytes().filter(|b| *b == b'.').count();
            let method = path.rsplit('.').next().unwrap_or("");
            if dots > 1 {
                // `local_E.Field.Method(…)` reaches through the field. That only mutates when
                // the method can: the cache records which ones are const, and a const call on a
                // read-only range-for element is exactly what vanilla wrote.
                if !refs.names_a_const_method(method) {
                    return true;
                }
                continue;
            }
            // A HANDLE element is a different question: `for (auto Arm : Arms)` binds the
            // handle, and the handle's own constness is not the object's — vanilla calls
            // `Arm.FinalizeAttack()`, a non-const method, from exactly such a loop. Only a VALUE
            // element is read-only in a way a non-const call would break.
            if ty.is_some_and(|ty| {
                !is_object_handle_type(ty)
                    && refs.calls_non_const_method(super::structure::bare_type_name(ty), method)
            }) {
                return true;
            }
        }
    }
    false
}

/// `auto local_N = <pure path>.Iterator();` -> (N, container path).
fn iterator_decl(line: &str) -> Option<(i32, String)> {
    let rest = line.trim().strip_prefix("auto local_")?;
    let (slot, rest) = rest.split_once(" = ")?;
    let slot: i32 = slot.parse().ok()?;
    let container = rest.strip_suffix(".Iterator();")?;
    // A pure member path is evaluated once by the range-for exactly as it was by the call, so
    // the fold cannot move an observable side effect.
    if container.is_empty()
        || !container
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
    {
        return None;
    }
    Some((slot, container.to_owned()))
}

/// `local_E = local_I.Proceed();` / `auto local_E = local_I.Proceed();` -> E.
fn proceed_assignment(line: &str, iter: i32) -> Option<i32> {
    let trimmed = line.trim();
    // The element may already carry a declaration (`auto`, or a value type the decl-init rewrite
    // gave it in an earlier pass).
    let trimmed = match trimmed.split_once(" local_") {
        Some((head, _)) if !head.contains('(') && !head.contains('=') => &trimmed[head.len() + 1..],
        _ => trimmed,
    };
    let rest = trimmed.strip_prefix("local_")?;
    let (slot, rest) = rest.split_once(" = ")?;
    let slot: i32 = slot.parse().ok()?;
    (rest == format!("local_{iter}.Proceed();")).then_some(slot)
}

/// Index of the `}` closing the block whose `{` is at `open`.
fn matching_close(lines: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (n, line) in lines.iter().enumerate().skip(open) {
        depth += brace_net(line);
        if depth == 0 {
            return Some(n);
        }
    }
    None
}

fn rewrite_iterator_decl_init(
    body: &str,
    locals: &BTreeMap<i32, String>,
) -> (String, HashSet<i32>) {
    let is_iter = |_slot: i32, ty: &str| {
        let h = ty.split('<').next().unwrap_or(ty);
        matches!(
            h,
            "TArrayIterator"
                | "TArrayConstIterator"
                | "TSetIterator"
                | "TSetConstIterator"
                | "TMapIterator"
                | "TMapConstIterator"
        )
    };
    // `auto`, not the inferred iterator head — see the render comment below.
    rewrite_decl_at_assignment(body, locals, &is_iter, &|_| "auto".to_string())
}

/// A VALUE-type local (`F*`/`T*`) that is hoisted and then assigned costs two symbols vanilla
/// does not have: a default-construct behaviour for the declaration and an `opAssign` for the
/// store. Vanilla builds the value at the point of use, so declaring at the first assignment
/// reproduces that and keeps the module splicable. Iterators are handled above; primitives,
/// object handles and const locals are left alone.
/// Slots the function COPY-constructs: `PSF slot; CALLSYS $beh0` where that `$beh0` takes a
/// parameter. A default construction takes none, so the parameter row tells the two apart.
fn copy_constructed_slots(f: &Func, refs: &RefResolver) -> HashSet<i32> {
    let Ok(instrs) = disassemble(&f.bytecode) else {
        return HashSet::new();
    };
    let mut out = HashSet::new();
    for pair in instrs.windows(2) {
        if pair[0].op.name != "PSF" || pair[1].op.name != "CALLSYS" {
            continue;
        }
        let ptr = pair[1].qwords.first().copied().unwrap_or(0) as i64;
        if refs.func_by_ptr(ptr) != Some("$beh0") {
            continue;
        }
        if refs.func_params_by_ptr(ptr).is_none_or(|params| params.is_empty()) {
            continue;
        }
        if let Some(slot) = pair[0].words.first().map(|word| *word as i16 as i32) {
            if slot > 0 {
                out.insert(slot);
            }
        }
    }
    out
}

fn rewrite_value_decl_init(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    copy_constructed: &HashSet<i32>,
) -> (String, HashSet<i32>) {
    // A slot vanilla COPY-constructs was declared with its value: the `$beh0` there takes a
    // parameter, which a default construction does not. That is a fact about this function, and
    // it outranks the general reading of the type — `constructs_by_assignment` describes what a
    // type usually does, not what the source wrote here.
    let is_value = |slot: i32, ty: &str| {
        copy_constructed.contains(&slot)
            || (is_value_struct_type(ty) && !constructs_by_assignment(ty, refs))
    };
    rewrite_decl_at_assignment(body, locals, &is_value, &|ty| ty.to_string())
}

/// A local whose first reference WRITES it through a member or an element — `local_N.Field = …;`
/// — was declared just there in the source. It cannot take a declaration-with-initializer (there
/// is no whole value to initialize it with), so the hoist put it at the top of the function and
/// the compiler ran its constructor before the guards that decide whether it is needed at all.
///
/// Only where the write stands at the function's own level and nothing reads the slot before it.
fn rewrite_bare_decl_at_first_write(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    already: &HashSet<i32>,
    placed: &HashSet<i32>,
) -> (String, HashSet<i32>) {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let mut suppressed = HashSet::new();
    for (slot, ty) in locals {
        if already.contains(slot) || placed.contains(slot) {
            continue;
        }
        let ident = format!("local_{slot}");
        // The first reference, and the brace depth it sits at — a function's own level is depth
        // zero, whether the function is a method or free (its body indent differs).
        let mut depth = 0i32;
        let mut first = None;
        for (at, line) in lines.iter().enumerate() {
            if count_ident(line, &ident) > 0 {
                first = Some((at, depth));
                break;
            }
            depth += brace_net(line);
        }
        let Some((at, depth)) = first else {
            continue;
        };
        let trimmed = lines[at].trim_start();
        // Either the first reference stands at the function's own level, whatever it does with
        // the slot: writing it through a member, handing it to a call that fills it, or reading
        // it. The declaration belongs there because that is where the source put it — vanilla
        // constructs the value behind the guard that decides whether it is needed, not at the
        // top of the function. (A plain `local_N = …` first reference belongs to the
        // declaration-with-initializer pass above, and those slots are already spoken for.)
        // Only a type that default-constructs itself: a PRIMITIVE declared bare and then read is
        // "may not be initialized", which is a warning, which is an error here (measured: 7).
        let first_use_at_top = depth == 0 && !is_primitive(ty);
        // …or the whole body mentions the slot exactly once, and the declaration belongs on that
        // line whatever depth it sits at: nothing outside can be looking at it.
        let sole_mention = count_ident(body, &ident) == 1 && !is_primitive(ty);
        if (!first_use_at_top && !sole_mention)
            || count_ident(&lines[at], &ident) != 1
            || !trimmed.ends_with(';')
        {
            continue;
        }
        let indent = indent_of(&lines[at]);
        lines.insert(at, format!("{indent}{} {ident};", qualify_decl_type(ty, refs)));
        suppressed.insert(*slot);
    }
    let mut joined = lines.join("\n");
    if body.ends_with('\n') {
        joined.push('\n');
    }
    (joined, suppressed)
}

/// Every remaining local whose first reference is the assignment that gives it its value: the
/// source declared it there. Hoisting the declaration instead makes the compiler put the value in
/// a temporary of its own and copy it into the declared slot.
///
/// Runs last, so a slot one of the typed rewrites above already placed keeps that placement.
fn rewrite_first_use_decl_init(
    body: &str,
    locals: &BTreeMap<i32, String>,
    refs: &RefResolver,
    already: &HashSet<i32>,
) -> (String, HashSet<i32>) {
    // Only where the assignment stands at the FUNCTION's own level. Inside a loop or a branch a
    // declaration is entered and left again with the block, and the compiler spends the slot's
    // construction and release on every pass — which vanilla, having hoisted it, does not
    // (measured: 94 functions lost against 46 gained when this was not required).
    let wanted =
        |slot: i32, _ty: &str| !already.contains(&slot) && first_top_level_assignment_before_read(body, slot);
    rewrite_decl_at_assignment(body, locals, &wanted, &|ty| qualify_decl_type(ty, refs))
}

/// Shared engine for both: declare a local at the assignment that first gives it a value,
/// renaming later independent assignment groups so each keeps its own scope. Bails for a slot
/// whose first reference is a READ — that one has to stay hoisted.
/// A line that DEFINES the local — a plain assignment or a declaration-with-initializer.
fn is_definition_line(line: &str, ident: &str) -> bool {
    let trimmed = line.trim_start();
    count_ident(line, ident) == 1
        && trimmed.ends_with(';')
        && (trimmed.starts_with(&format!("{ident} = ")) || declares_and_initializes(trimmed, ident))
}

/// `TSubclassOf<UActorComponent> local_52 = …;` — a definition that already carries its own
/// declaration head.
fn declares_and_initializes(trimmed: &str, ident: &str) -> bool {
    let Some((head, rest)) = trimmed.split_once(&format!(" {ident} = ")) else {
        return false;
    };
    !head.is_empty() && !head.contains(['(', ')', '=', ',', '.']) && !rest.is_empty()
}

fn rewrite_decl_at_assignment(
    body: &str,
    locals: &BTreeMap<i32, String>,
    want: &dyn Fn(i32, &str) -> bool,
    decl_head: &dyn Fn(&str) -> String,
) -> (String, HashSet<i32>) {
    let mut suppressed: HashSet<i32> = HashSet::new();
    let mut out = body.to_string();
    for (slot, ty) in locals {
        if !want(*slot, ty) {
            continue;
        }
        let ident = format!("local_{slot}");
        let pat = format!("{ident} = ");
        let lines: Vec<&str> = out.lines().collect();
        let refs: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| count_ident(l, &ident) > 0)
            .map(|(i, _)| i)
            .collect();
        if refs.is_empty() {
            continue;
        }
        // Group references: each group starts at an assignment line and swallows every
        // following reference inside that assignment's innermost block span.
        let mut groups: Vec<(usize, Vec<usize>)> = Vec::new(); // (assign line, later refs)
        let mut ok = true;
        let mut k = 0usize;
        while k < refs.len() {
            let i = refs[k];
            let t = lines[i].trim_start();
            // An earlier pass may already have turned the FIRST definition into a
            // declaration-with-initializer. That is still a definition, and refusing to see one
            // left every LATER assignment as a bare `local_N = …` — an `opAssign` the base cache
            // does not have for that type, which costs the module its splicability.
            let is_assign = count_ident(lines[i], &ident) == 1
                && (t.starts_with(&pat) || declares_and_initializes(t, &ident))
                && t.ends_with(';');
            if !is_assign {
                ok = false; // read before any in-block assignment — keep the hoist
                break;
            }
            let (_, end) = block_span(&lines, i);
            let mut members: Vec<usize> = Vec::new();
            k += 1;
            // Stop at the NEXT definition as well as at the block end: the compiler reuses one
            // slot for two source temporaries in the same block, and swallowing the second
            // definition as a member left it a bare `local_N = …` — an `opAssign` the base
            // cache has no row for. It becomes its own group (and its own declaration) instead.
            while k < refs.len() && refs[k] < end && !is_definition_line(lines[refs[k]], &ident) {
                members.push(refs[k]);
                k += 1;
            }
            groups.push((i, members));
        }
        if !ok || groups.is_empty() {
            continue;
        }
        // Render. `auto`, not the inferred iterator type: the cache's recorded return type
        // does not reliably distinguish Const vs mutable Iterator overloads (the receiver's
        // constness decides in-source), so a spelled-out head fails "Can't implicitly convert
        // TXIterator<T> <-> TXConstIterator<T>" whenever we guess wrong (72 in-game errors;
        // the batch-9 const-flip regression was the same coin's other face). AngelScript's
        // auto infers the exact overload the compiler resolves.
        let mut new_names: HashMap<usize, String> = HashMap::new(); // line -> name for renames
        let mut decl_lines: HashSet<usize> = HashSet::new();
        for (g, (assign, members)) in groups.iter().enumerate() {
            if !declares_and_initializes(lines[*assign].trim_start(), &ident) {
                decl_lines.insert(*assign);
            }
            if g > 0 {
                let fresh = format!("{ident}_{}", g + 1);
                new_names.insert(*assign, fresh.clone());
                for m in members {
                    new_names.insert(*m, fresh.clone());
                }
            }
        }
        let mut rewritten = String::with_capacity(out.len() + 64);
        for (i, line) in lines.iter().enumerate() {
            let line: String = match new_names.get(&i) {
                Some(new) => rename_ident(line, &ident, new),
                None => (*line).to_string(),
            };
            if decl_lines.contains(&i) {
                let t = line.trim_start();
                let indent = &line[..line.len() - t.len()];
                let head = decl_head(ty);
                let _ = writeln!(rewritten, "{indent}{head} {t}");
            } else {
                rewritten.push_str(&line);
                rewritten.push('\n');
            }
        }
        out = rewritten;
        suppressed.insert(*slot);
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
    // Float return-payload retype: a slot copied into the value register (`CpyVtoR8`/
    // `CpyVtoR4`) in a function returning the matching-width float family IS the float
    // return value, so its `SetV*`-guessed int/int64 declaration must become the return
    // type keyword (structure.rs `float_operand_slots` renders those constants as float
    // literals — `local_4 = -55.0;` needs `float local_4;`, not `int64`). Never overrides
    // object-typed slots (rank 0).
    {
        let copy_op = match f.ret.token {
            0x51 | 0x5E => Some("CpyVtoR8"),
            0x50 => Some("CpyVtoR4"),
            _ => None,
        };
        if let Some(op) = copy_op {
            let kw = token_keyword(f.ret.token).to_string();
            for ins in &instrs {
                if ins.op.name != op {
                    continue;
                }
                let Some(dst) = ins.words.first().map(|w| *w as i16 as i32) else {
                    continue;
                };
                if dst <= 0 {
                    continue;
                }
                match locals.get(&dst) {
                    None => {
                        locals.insert(dst, kw.clone());
                    }
                    Some(prev) if matches!(prev.as_str(), "int" | "int64" | "float" | "double") => {
                        locals.insert(dst, kw.clone());
                    }
                    _ => {}
                }
            }
        }
    }
    let _ = token_keyword; // keep import used if obj path elided
    locals
}

fn writes_int(n: &str) -> bool {
    matches!(
        n,
        "SetV4" | "SetV1" | "ADDi" | "SUBi" | "MULi" | "DIVi" | "MODi" | "IncVi" | "DecVi"
        | "NEGi" | "BAND" | "BOR" | "BXOR" | "BSLL" | "BSRA" | "ADDIi" | "SUBIi" | "MULIi"
        | "CpyVtoR4" | "RDR4" | "CpyRtoV4"
        // conversions whose RESULT is a 32-bit int/uint (*TO i/u/b/w)
        | "fTOi" | "fTOu" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" | "dTOi" | "dTOu"
        | "iTOb" | "iTOw" | "i64TOi"
    )
}
fn writes_float(n: &str) -> bool {
    matches!(
        n,
        "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf"
        | "ADDIf" | "SUBIf" | "MULIf"
        // conversions whose RESULT is float (*TO f)
        | "iTOf" | "uTOf" | "dTOf" | "i64TOf" | "u64TOf"
    )
}
fn writes_double(n: &str) -> bool {
    matches!(
        n,
        "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd"
        // conversions whose RESULT is double (*TO d)
        | "iTOd" | "uTOd" | "fTOd" | "i64TOd" | "u64TOd"
    )
}
fn writes_int64(n: &str) -> bool {
    matches!(
        n,
        "SetV8" | "ADDi64" | "SUBi64" | "MULi64" | "DIVi64"
        // conversions whose RESULT is a 64-bit int/uint (*TO i64/u64)
        | "uTOi64" | "iTOi64" | "fTOi64" | "dTOi64" | "fTOu64" | "dTOu64"
    )
}

/// Heuristic: a UE enum type name (`E` + uppercase), which is int-castable like a primitive.
fn is_enum(ty: &str) -> bool {
    let b = super::structure::bare_type_name(ty).as_bytes();
    b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase()
}

/// True for AngelScript primitive scalar types (need an explicit initializer).
fn is_primitive(ty: &str) -> bool {
    matches!(
        ty,
        "bool"
            | "int"
            | "int8"
            | "int16"
            | "int64"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint64"
            | "float"
            | "float32"
            | "double"
    )
}

/// A default initializer literal for a base type.
fn default_for(ty: &str) -> String {
    match ty {
        "float" | "double" => "0.0".into(),
        // batch-28 rider: constant-exactness is silent for 0.0 either way, but the
        // f-suffix is the faithful vanilla form for the 4-byte float.
        "float32" => "0.0f".into(),
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
        "bool" => {
            if v != 0 {
                "true".into()
            } else {
                "false".into()
            }
        }
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

#[cfg(test)]
mod enum_call_round_trip_tests {
    use super::fold_enum_call_round_trips;
    use std::collections::HashMap;

    fn types() -> HashMap<i32, String> {
        HashMap::from([(5, "ERelationship".to_owned())])
    }

    #[test]
    fn a_round_trip_through_the_calls_own_enum_is_the_call() {
        let body = "    int local_5 = int(this.GetRelationship(Entry));\n    this.Severities.Find(ERelationship(local_5), local_2);\n";
        assert_eq!(
            fold_enum_call_round_trips(body, &types()),
            "    this.Severities.Find(this.GetRelationship(Entry), local_2);\n",
            "both casts name the type the call already returns"
        );
    }

    #[test]
    fn a_round_trip_through_another_enum_is_a_conversion() {
        let body = "    int local_5 = int(this.GetRelationship(Entry));\n    this.Severities.Find(EGuild(local_5), local_2);\n";
        assert_eq!(
            fold_enum_call_round_trips(body, &types()),
            body,
            "a different enum converts, and the conversion has to stay"
        );
    }

    #[test]
    fn an_unresolved_callee_keeps_its_casts() {
        let body = "    int local_5 = int(this.GetRelationship(Entry));\n    this.Severities.Find(ERelationship(local_5), local_2);\n";
        assert_eq!(
            fold_enum_call_round_trips(body, &HashMap::new()),
            body,
            "with no return type there is no witness, so nothing moves"
        );
    }
}

#[cfg(test)]
mod declaration_sink_tests {
    use super::sink_declarations_into_their_block;
    use std::collections::HashSet;

    const BODY: &str = "    FThing local_9;\n    if (Guard())\n    {\n        return;\n    }\n    if (Other())\n    {\n        local_9.Field = 1;\n        Use(local_9);\n    }\n    return;\n";

    #[test]
    fn a_declaration_used_in_one_block_moves_into_it() {
        let sunk = sink_declarations_into_their_block(BODY, &HashSet::from([9]));
        assert_eq!(
            sunk,
            "    if (Guard())\n    {\n        return;\n    }\n    if (Other())\n    {\n        FThing local_9;\n        local_9.Field = 1;\n        Use(local_9);\n    }\n    return;\n",
            "the constructor belongs where the block that uses it begins"
        );
    }

    #[test]
    fn a_slot_touched_before_the_branch_stays_where_it_was() {
        assert_eq!(
            sink_declarations_into_their_block(BODY, &HashSet::new()),
            BODY,
            "without the witness the declaration was at function scope and stays there"
        );
    }

    #[test]
    fn mentions_in_two_blocks_stay_at_function_scope() {
        let body = "    FThing local_9;\n    if (A())\n    {\n        Use(local_9);\n    }\n    if (B())\n    {\n        Use(local_9);\n    }\n";
        assert_eq!(
            sink_declarations_into_their_block(body, &HashSet::from([9])),
            body,
            "no single block holds every mention, so nothing can hold the declaration"
        );
    }
}

#[cfg(test)]
mod bracket_tests {
    use super::unwrap_brackets;

    #[test]
    fn one_pair_around_the_whole_value_comes_off() {
        assert_eq!(unwrap_brackets("(int(X.F) + 1)"), "int(X.F) + 1");
    }

    #[test]
    fn brackets_that_only_look_like_a_pair_stay() {
        assert_eq!(
            unwrap_brackets("(a) + (b)"),
            "(a) + (b)",
            "the leading and trailing brackets belong to different groups"
        );
    }

    #[test]
    fn an_unbracketed_value_is_returned_as_it_is() {
        assert_eq!(unwrap_brackets("X.F + 1"), "X.F + 1");
    }
}

#[cfg(test)]
mod accumulator_tests {
    use super::collapse_single_use_accumulators;
    use std::collections::HashSet;

    const BODY: &str = "    float local_10 = Thing.GetRadius();\n    local_10 = local_10 * Multiplier;\n    return Use(local_10);\n";

    #[test]
    fn accumulator_collapses_into_its_single_reader() {
        let folded = collapse_single_use_accumulators(BODY, &HashSet::new());
        assert_eq!(
            folded, "    return Use((Thing.GetRadius() * Multiplier));\n",
            "declare, accumulate, read once is one expression: {folded}"
        );
    }

    #[test]
    fn accumulator_read_twice_keeps_its_name() {
        let body = "    float local_10 = Thing.GetRadius();\n    local_10 = local_10 * Multiplier;\n    return Use(local_10, local_10);\n";
        assert_eq!(
            collapse_single_use_accumulators(body, &HashSet::new()),
            body,
            "a value read twice has to keep the name it is read through"
        );
    }

    #[test]
    fn accumulator_on_a_named_widening_keeps_its_name() {
        assert_eq!(
            collapse_single_use_accumulators(BODY, &HashSet::from([10])),
            BODY,
            "where the copy IS the declaration, folding it changes the width of the arithmetic"
        );
    }
}

#[cfg(test)]
mod indirect_numeric_edge_tests {
    use super::{
        adjacent_value_slot_is_candidate, all_reads_lexically_dominated_by_assignment,
        bool_slot_profile_is_safe, first_top_level_assignment_before_read, indirect_numeric_edges,
        local_assignment_count, propagate_proven_enum_slots, rewrite_adjacent_value_temporaries,
    };
    use crate::cache::disasm::{disassemble, Instr};
    use std::collections::HashSet;

    fn word_op(opcode: u8, slot: u16) -> i32 {
        ((slot as u32) << 16 | opcode as u32) as i32
    }

    fn ins(name: &'static str, words: &[u16]) -> Instr {
        let op = crate::cache::isa::OPCODES
            .iter()
            .find(|op| op.name == name)
            .expect("test opcode");
        Instr {
            offset_dw: 0,
            op,
            words: words.to_vec(),
            dwords: Vec::new(),
            qwords: Vec::new(),
        }
    }

    fn ins_imm(name: &'static str, words: &[u16], immediate: u32) -> Instr {
        let mut instruction = ins(name, words);
        instruction.dwords.push(immediate);
        instruction
    }

    #[test]
    fn exact_local_address_reads_and_writes_form_width_typed_edges() {
        let read = disassemble(&[
            word_op(48, 20), // PshVPtr w20
            58,              // PopRPtr
            word_op(94, 2),  // RDR4 w2
        ])
        .expect("read shape");
        assert_eq!(indirect_numeric_edges(&read), vec![(20, 2, false)]);

        let write = disassemble(&[
            word_op(48, 24), // PshVPtr w24
            58,              // PopRPtr
            word_op(91, 6),  // WRTV8 w6
        ])
        .expect("write shape");
        assert_eq!(indirect_numeric_edges(&write), vec![(24, 6, true)]);
    }

    #[test]
    fn non_adjacent_or_non_local_pointer_shapes_do_not_form_edges() {
        let interrupted = disassemble(&[
            word_op(48, 20), // PshVPtr w20
            49,              // RDSPtr: no exact local-address transfer
            58,              // PopRPtr
            word_op(94, 2),  // RDR4 w2
        ])
        .expect("interrupted shape");
        assert!(indirect_numeric_edges(&interrupted).is_empty());

        let no_transfer = disassemble(&[
            word_op(48, 20), // ordinary pointer push
            word_op(94, 2),  // no PopRPtr
        ])
        .expect("non-transfer shape");
        assert!(indirect_numeric_edges(&no_transfer).is_empty());
    }

    #[test]
    fn proven_enum_call_return_propagates_only_to_narrow_read_copy_chain() {
        let instrs = vec![
            ins("CpyRtoV4", &[5]),
            ins("SetV1", &[5]),
            ins("CpyVtoV4", &[4, 5]),
            ins("sbTOi", &[2, 4]),
            ins("CpyVtoR4", &[5]),
            // A generic copy without a downstream enum-width read must stay untyped.
            ins("CpyVtoV4", &[3, 5]),
        ];
        let types = propagate_proven_enum_slots(&instrs, vec![(5, "EResult".into())]);
        assert_eq!(types.get(&5).map(String::as_str), Some("EResult"));
        assert_eq!(types.get(&4).map(String::as_str), Some("EResult"));
        assert!(!types.contains_key(&3));
    }

    #[test]
    fn enum_copy_propagation_fails_closed_on_arithmetic_or_type_conflict() {
        let arithmetic = vec![
            ins("CpyRtoV4", &[5]),
            ins("CpyVtoV4", &[4, 5]),
            ins("sbTOi", &[2, 4]),
            ins("ADDIi", &[4, 4]),
            ins("CpyVtoR4", &[5]),
        ];
        let types = propagate_proven_enum_slots(&arithmetic, vec![(5, "EResult".into())]);
        assert_eq!(types.get(&5).map(String::as_str), Some("EResult"));
        assert!(!types.contains_key(&4));

        let conflict = vec![
            ins("CpyVtoV4", &[4, 5]),
            ins("CpyVtoV4", &[4, 6]),
            ins("sbTOi", &[2, 4]),
        ];
        let types = propagate_proven_enum_slots(
            &conflict,
            vec![(5, "EFirst".into()), (6, "ESecond".into())],
        );
        assert!(!types.contains_key(&4));
    }

    #[test]
    fn bool_field_slot_profile_accepts_only_canonical_boolean_writes() {
        let safe = vec![
            ins("TZ", &[]),
            ins("CpyRtoV4", &[8]),
            ins("CpyVtoV4", &[7, 8]),
            ins_imm("SetV1", &[8], 1),
            ins("WRTV1", &[8]),
            ins("RDR1", &[8]),
            ins("NOT", &[8]),
            ins("CpyVtoR1", &[8]),
        ];
        assert!(bool_slot_profile_is_safe(&safe, 8));

        let mut non_bool_constant = safe.clone();
        non_bool_constant.push(ins_imm("SetV1", &[8], 2));
        assert!(!bool_slot_profile_is_safe(&non_bool_constant, 8));

        let arbitrary_register_copy = vec![ins_imm("SetV4", &[1], 7), ins("CpyRtoV4", &[8])];
        assert!(!bool_slot_profile_is_safe(&arbitrary_register_copy, 8));
    }

    #[test]
    fn enum_bare_declaration_requires_first_unconditional_write_only_assignment() {
        assert!(first_top_level_assignment_before_read(
            "local_4 = local_5;\nreturn local_4;\n",
            4
        ));
        assert!(first_top_level_assignment_before_read(
            "if (ready)\n{\n    Work();\n}\nlocal_4 = local_5;\n",
            4
        ));

        assert!(!first_top_level_assignment_before_read(
            "Use(local_4);\nlocal_4 = local_5;\n",
            4
        ));
        assert!(!first_top_level_assignment_before_read(
            "local_4 = local_4;\n",
            4
        ));
        assert!(!first_top_level_assignment_before_read(
            "if (ready)\n{\n    local_4 = local_5;\n}\nreturn local_4;\n",
            4
        ));
    }

    #[test]
    fn lexical_assignment_proof_accepts_local_reuse_but_rejects_branch_leakage() {
        let loop_local = concat!(
            "while (ready)\n",
            "{\n",
            "    local_2 = this.Index;\n",
            "    Use(local_2);\n",
            "    local_2 = this.Items.Num();\n",
            "    if (local_2 > 0)\n",
            "    {\n",
            "        UseAgain(local_2);\n",
            "    }\n",
            "}\n",
        );
        assert!(all_reads_lexically_dominated_by_assignment(loop_local, 2));

        let sibling_cases = concat!(
            "switch (kind)\n",
            "{\n",
            "case 0:\n",
            "{\n",
            "    local_6 = 0;\n",
            "    Use(local_6);\n",
            "}\n",
            "case 1:\n",
            "{\n",
            "    local_6 = 1;\n",
            "    Use(local_6);\n",
            "}\n",
            "}\n",
        );
        assert!(all_reads_lexically_dominated_by_assignment(
            sibling_cases,
            6
        ));

        for rejected in [
            "Use(local_2);\nlocal_2 = 1;\n",
            "local_2 = local_2;\n",
            "if (ready)\n{\n    local_2 = 1;\n}\nUse(local_2);\n",
            "{\nlocal_2 = 1;\n",
        ] {
            assert!(
                !all_reads_lexically_dominated_by_assignment(rejected, 2),
                "unexpectedly accepted:\n{rejected}"
            );
        }
    }

    #[test]
    fn adjacent_value_temporaries_fold_only_when_the_whole_slot_disappears() {
        let body = concat!(
            "local_5 = EResult(0);\n",
            "local_4 = local_5;\n",
            "while (local_4)\n",
            "{\n",
            "    local_2 = this.Index;\n",
            "    local_5 = this.Nodes.opIndex(local_2).Tick();\n",
            "    local_4 = local_5;\n",
            "    local_2 = int(local_4);\n",
            "    switch (local_2)\n",
            "    {\n",
            "    }\n",
            "    local_8 = true;\n",
            "    this.Flag = local_8;\n",
            "    local_2 = this.Nodes.Num();\n",
            "    local_6 = this.Index;\n",
            "    if (local_2 <= local_6)\n",
            "    {\n",
            "        local_6 = 0;\n",
            "        this.Index = local_6;\n",
            "        local_8 = this.Flag;\n",
            "        local_8 = !local_8;\n",
            "        if (local_8)\n",
            "        {\n",
            "            local_5 = EResult(1);\n",
            "            return local_5;\n",
            "        }\n",
            "    }\n",
            "}\n",
        );
        let candidates = HashSet::from([2, 4, 5, 6, 8]);
        let (folded, eliminated) = rewrite_adjacent_value_temporaries(body, &candidates);

        assert_eq!(eliminated, HashSet::from([2, 5, 6, 8]), "{folded}");
        assert!(folded.contains("local_4 = EResult(0);"), "{folded}");
        assert!(
            folded.contains("local_4 = this.Nodes.opIndex(this.Index).Tick();"),
            "{folded}"
        );
        assert!(folded.contains("this.Flag = true;"), "{folded}");
        assert!(
            folded.contains("if ((this.Nodes.Num()) <= (this.Index))"),
            "{folded}"
        );
        assert!(folded.contains("if (!(this.Flag))"), "{folded}");
        assert!(folded.contains("return EResult(1);"), "{folded}");
        for eliminated_ident in ["local_2", "local_5", "local_6", "local_8"] {
            assert!(!folded.contains(eliminated_ident), "{folded}");
        }
        assert!(
            folded.contains("local_4"),
            "live state local was lost:\n{folded}"
        );
    }

    #[test]
    fn primitive_value_carriers_with_multiple_definitions_are_not_fold_candidates() {
        let body = "local_2 = First();\nUse(local_2);\nlocal_2 = Second();\nUse(local_2);\n";
        assert_eq!(local_assignment_count(body, 2), 2);
        assert!(!adjacent_value_slot_is_candidate(body, 2, false, true));
        assert!(adjacent_value_slot_is_candidate(
            "local_2 = Once();\nUse(local_2);\n",
            2,
            false,
            true
        ));
        assert!(adjacent_value_slot_is_candidate(body, 2, true, true));
        assert!(!adjacent_value_slot_is_candidate(body, 2, false, false));
    }

    #[test]
    fn adjacent_value_temporary_rejects_gaps_multiuse_and_unsafe_call_order() {
        for body in [
            "local_2 = SideEffect();\nOther();\nreturn local_2;\n",
            "local_2 = 1;\nUse(local_2, local_2);\n",
            "local_2 = this.Value;\nout = Mutate().Use(local_2);\n",
            "local_2 = Mutate();\nif (this.Value <= local_2)\n{\n}\n",
            "local_2 = 1;\nif (ready)\n{\n    return local_2;\n}\n",
        ] {
            let (folded, eliminated) =
                rewrite_adjacent_value_temporaries(body, &HashSet::from([2]));
            assert_eq!(folded, body);
            assert!(eliminated.is_empty());
        }
    }
}

#[cfg(test)]
mod source_shape_tests {
    use super::{
        drop_dead_stores, fold_cast_diamonds, inline_foreach_containers, produced_only_by_calls,
        rewrite_foreach_loops, rewrite_operator_calls, rewrite_value_temporaries,
    };
    use crate::cache::refs::RefResolver;
    use std::collections::BTreeMap;

    fn locals(pairs: &[(i32, &str)]) -> BTreeMap<i32, String> {
        pairs
            .iter()
            .map(|(slot, ty)| (*slot, (*ty).to_owned()))
            .collect()
    }

    #[test]
    fn folds_a_value_temporary_into_the_call_that_consumes_it() {
        let body =
            "    local_7 = this.MakeTransition(n\"Loop\");\n    this.Transitions.Add(local_7);\n";
        let (out, gone) = rewrite_value_temporaries(body, &locals(&[(7, "FTransition")]));
        assert_eq!(
            out,
            "    this.Transitions.Add(this.MakeTransition(n\"Loop\"));\n"
        );
        assert!(gone.contains(&7));
    }

    #[test]
    fn keeps_a_temporary_whose_store_is_a_declaration_site_conversion() {
        // `FText local_6 = "id";` converts; folding the literal would hand the consumer an FString.
        let body = "    local_6 = \"UI_Hint1\";\n    this.TipText.Add(local_6);\n";
        assert!(!produced_only_by_calls(body, 6));
        let (out, gone) = rewrite_value_temporaries(body, &locals(&[(6, "FText")]));
        assert_eq!(out, body);
        assert!(gone.is_empty());
    }

    #[test]
    fn rebuilds_the_range_for_the_compiler_desugared() {
        let body = concat!(
            "        auto local_6 = this.m_Arms.Iterator();\n",
            "        while (local_6.CanProceed)\n",
            "        {\n",
            "            local_16 = local_6.Proceed();\n",
            "            local_16.Cancel();\n",
            "            local_16 = nullptr;\n",
            "        }\n",
        );
        let (out, gone) =
            rewrite_foreach_loops(body, &locals(&[(16, "AActor")]), &RefResolver::default());
        assert_eq!(
            out,
            concat!(
                "        for (auto local_16 : this.m_Arms)\n",
                "        {\n",
                "            local_16.Cancel();\n",
                "        }\n",
            )
        );
        assert!(gone.contains(&16));
    }

    #[test]
    fn rebuilds_the_range_for_when_the_element_already_carries_a_declaration() {
        let body = concat!(
            "        auto local_8 = Loot.Iterator();
",
            "        while (local_8.CanProceed)
",
            "        {
",
            "            FItemVirtualData local_16 = local_8.Proceed();
",
            "            Use(local_16);
",
            "        }
",
        );
        let (out, gone) = rewrite_foreach_loops(
            body,
            &locals(&[(16, "FItemVirtualData")]),
            &RefResolver::default(),
        );
        assert_eq!(
            out,
            concat!(
                "        for (auto local_16 : Loot)
",
                "        {
",
                "            Use(local_16);
",
                "        }
",
            )
        );
        assert!(gone.contains(&16));
    }

    #[test]
    fn keeps_the_while_shape_when_the_element_is_written_with_a_value() {
        let body = concat!(
            "        auto local_6 = this.m_Arms.Iterator();\n",
            "        while (local_6.CanProceed)\n",
            "        {\n",
            "            local_16 = local_6.Proceed();\n",
            "            local_16 = OtherActor;\n",
            "        }\n",
        );
        let (out, gone) =
            rewrite_foreach_loops(body, &locals(&[(16, "AActor")]), &RefResolver::default());
        assert_eq!(out, body);
        assert!(gone.is_empty());
    }

    #[test]
    fn keeps_the_while_shape_when_the_element_outlives_the_loop() {
        let body = concat!(
            "        auto local_6 = this.m_Arms.Iterator();\n",
            "        while (local_6.CanProceed)\n",
            "        {\n",
            "            local_16 = local_6.Proceed();\n",
            "        }\n",
            "        return local_16;\n",
        );
        let (out, _) =
            rewrite_foreach_loops(body, &locals(&[(16, "AActor")]), &RefResolver::default());
        assert_eq!(out, body);
    }

    #[test]
    fn iterates_the_member_instead_of_a_copy_of_it() {
        let body = concat!(
            "        TMap<A, B> local_20 = this.m_Map;\n",
            "        for (auto local_40 : local_20)\n",
            "        {\n",
            "            this.Update(local_40.GetKey());\n",
            "        }\n",
        );
        assert_eq!(
            inline_foreach_containers(body).0,
            concat!(
                "        for (auto local_40 : this.m_Map)\n",
                "        {\n",
                "            this.Update(local_40.GetKey());\n",
                "        }\n",
            )
        );
    }

    #[test]
    fn keeps_the_copy_when_the_loop_body_touches_the_member() {
        let body = concat!(
            "        TMap<A, B> local_20 = this.m_Map;\n",
            "        for (auto local_40 : local_20)\n",
            "        {\n",
            "            this.m_Map.Remove(local_40.GetKey());\n",
            "        }\n",
        );
        assert_eq!(inline_foreach_containers(body).0, body);
    }

    #[test]
    fn folds_the_null_guarded_cast_back_into_the_cast() {
        let body = concat!(
            "        if (local_4 != nullptr)
",
            "        {
",
            "            local_6 = Cast<UPlayerConfig>(local_4);
",
            "        }
",
            "        else
",
            "        {
",
            "        }
",
            "        this.CharacterConfig = local_6;
",
        );
        assert_eq!(
            fold_cast_diamonds(body),
            concat!(
                "        local_6 = Cast<UPlayerConfig>(local_4);
",
                "        this.CharacterConfig = local_6;
",
            )
        );
    }

    #[test]
    fn keeps_a_guarded_block_that_is_not_the_cast_idiom() {
        let body = concat!(
            "        if (local_4 != nullptr)
",
            "        {
",
            "            local_6 = Cast<UPlayerConfig>(local_4);
",
            "        }
",
            "        else
",
            "        {
",
            "            this.Fallback();
",
            "        }
",
        );
        assert_eq!(fold_cast_diamonds(body), body);
    }

    #[test]
    fn writes_a_recovered_operator_method_as_its_operator() {
        let body = "    Rules.RequireFalse(FBits(A::One).opOr(A::Two).opOr(A::Three));
";
        assert_eq!(
            rewrite_operator_calls(body),
            "    Rules.RequireFalse(((FBits(A::One) | A::Two) | A::Three));
"
        );
    }

    #[test]
    fn writes_a_recovered_index_operator_as_a_subscript() {
        let body = "    this.Presets.opIndex(n\"Torch\").LightRadius = 350.0f;
";
        assert_eq!(
            rewrite_operator_calls(body),
            "    this.Presets[n\"Torch\"].LightRadius = 350.0f;
"
        );
    }

    #[test]
    fn leaves_an_operator_call_whose_receiver_holds_a_string() {
        let body = "    Set(Name(\"a)b\").opOr(X));
";
        assert_eq!(rewrite_operator_calls(body), body);
    }

    #[test]
    fn drops_a_store_the_rendered_source_never_reads() {
        let body = "    local_2 = 1045220557;\n    this.Weight = 0.2f;\n";
        assert_eq!(drop_dead_stores(body), "    this.Weight = 0.2f;\n");
    }

    #[test]
    fn keeps_a_store_whose_value_is_read_back() {
        let body = "    local_2 = 3;\n    this.Weight = local_2;\n";
        assert_eq!(drop_dead_stores(body), body);
    }

    #[test]
    fn keeps_an_unread_store_that_could_have_run_a_call() {
        let body = "    local_2 = this.Consume();\n    return;\n";
        assert_eq!(drop_dead_stores(body), body);
    }
}
