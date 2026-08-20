//! Emit ALL modules of a precompiled cache as recompilable `.as` into a directory, mirroring
//! each module's ScriptRelativeFilename. Free-function name collisions across modules are
//! de-collided per-module (AngelScript compiles all loose `.as` into one global scope).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::disasm::disassemble;
use super::model::{Func, Module};
use super::refs::RefResolver;

#[derive(Debug)]
pub struct EmitAllStats {
    pub written: usize,
    /// Functions with an editable body actually written to the source tree.
    pub functions: usize,
    /// Raw function/method/constructor records parsed from the cache, including generated
    /// accessors/default initializers and duplicate records intentionally omitted by the emitter.
    pub cache_function_records: usize,
    /// Number of modules containing at least one signature-preserving fallback body.
    pub stubbed: usize,
    /// Exact number of signature-preserving fallback bodies across all modules.
    pub stubbed_functions: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EmitAllError {
    #[error("io: {0}")]
    Io(String),
    #[error("invalid module layout: {0}")]
    InvalidLayout(String),
    #[error("module index {index} is out of range for {modules} modules")]
    InvalidModuleIndex { index: usize, modules: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParameterSignature(Vec<Vec<String>>);

/// Deterministic per-module names used only by a full loose-source tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FreeFunctionRenamePlan {
    per_module: Vec<BTreeMap<String, String>>,
    /// Token-preserving parameter signatures for every collision-bound original name. These are
    /// the same deduplicated signatures `emit_module` writes, without the function name.
    required_signatures: Vec<BTreeMap<String, BTreeSet<ParameterSignature>>>,
    original_names: BTreeSet<String>,
    /// Arity is enough to prove these global calls cannot bind one of the renamed overloads. A
    /// renamed overload with N parameters is conservatively treated as callable with 0..=N
    /// arguments because cache metadata does not retain its default-argument boundary.
    safe_global_call_arities: BTreeMap<String, BTreeSet<usize>>,
}

impl FreeFunctionRenamePlan {
    fn renames_for_module(&self, module_index: usize) -> &BTreeMap<String, String> {
        self.per_module
            .get(module_index)
            .unwrap_or_else(|| empty_renames())
    }

    fn renamed(&self, module_index: usize, name: &str) -> Option<&str> {
        self.renames_for_module(module_index)
            .get(name)
            .map(String::as_str)
    }

    /// Rewrite only top-level free-function declaration identifiers. Bytecode-derived call sites
    /// are already renamed by `RefResolver::set_free_fn_renames`; touching arbitrary source tokens
    /// would also mutate methods, literals, globals, and comments.
    fn rewrite_emitted_module(&self, module_index: usize, source: &str) -> String {
        let declarations =
            rewrite_top_level_declarations(source, self.renames_for_module(module_index));
        qualify_emitted_collision_calls(&declarations, &self.original_names)
    }

    /// Make an authored overlay consistent with the collision-renamed vanilla tree. Existing edit
    /// declarations can be rewritten safely because their declaring module is known. Bare calls
    /// and global calls that could bind a renamed overload are ambiguous in authored source, so
    /// reject them before starting the compiler. A globally qualified call remains safe only when
    /// its arity can bind an unchanged overload but no renamed one.
    fn prepare_overlay(
        &self,
        mods: &[Module],
        op: &str,
        module_name: &str,
        source: &str,
    ) -> Result<String, String> {
        let rewritten = if op == "edit" {
            let indices = mods
                .iter()
                .enumerate()
                .filter(|(_, module)| module.name == module_name)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let [module_index] = indices.as_slice() else {
                return Err(format!(
                    "edit overlay module {module_name:?} must identify exactly one base module (found {})",
                    indices.len()
                ));
            };
            let renames = self.renames_for_module(*module_index);
            let required = self
                .required_signatures
                .get(*module_index)
                .unwrap_or_else(|| empty_required_signatures());
            validate_collision_bound_declarations(source, renames, required).map_err(|error| {
                format!("edited module {module_name:?} has invalid collision-bound declarations: {error}")
            })?;
            rewrite_top_level_declarations(source, renames)
        } else {
            source.to_owned()
        };

        let unresolved = unresolved_collision_calls(
            &rewritten,
            &self.original_names,
            &self.safe_global_call_arities,
        );
        if !unresolved.is_empty() {
            return Err(format!(
                "authored overlay contains collision-ambiguous free call(s): {}; use the deterministic renamed function or remove the call",
                unresolved.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }

        Ok(rewritten)
    }
}

fn empty_renames() -> &'static BTreeMap<String, String> {
    static EMPTY: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(BTreeMap::new)
}

fn empty_required_signatures() -> &'static BTreeMap<String, BTreeSet<ParameterSignature>> {
    static EMPTY: std::sync::OnceLock<BTreeMap<String, BTreeSet<ParameterSignature>>> =
        std::sync::OnceLock::new();
    EMPTY.get_or_init(BTreeMap::new)
}

/// Populate resolver inputs that preserve cache semantics without inventing emit-tree-only names.
/// `as decompile` uses this path so its output continues to describe the inspected cache.
/// Names whose const return SOME caller cannot hold. A caller stores an object result with
/// `STOREOBJ`; if that slot also receives any other write — a null store, a handle copy, a call
/// whose return is not const — then no single declaration can own it, and AngelScript has no way
/// to drop the qualifier at the store ("No conversion from 'const X' to 'X' available"). Such a
/// name keeps the stripped return type, exactly as before the qualifier was restored.
fn unusable_const_returns(mods: &[Module], refs: &RefResolver) -> HashSet<String> {
    let mut unusable = HashSet::new();
    for module in mods {
        let functions = module.functions.iter().chain(
            module
                .classes
                .iter()
                .flat_map(|class| class.methods.iter().chain(class.ctors.iter())),
        );
        for function in functions {
            let Ok(instrs) = disassemble(&function.bytecode) else {
                continue;
            };
            // slot -> the const-returning callee stored into it, and whether anything else wrote it
            let mut const_source: HashMap<i32, String> = HashMap::new();
            let mut written_otherwise: HashSet<i32> = HashSet::new();
            for (index, ins) in instrs.iter().enumerate() {
                let destination = match ins.op.name {
                    "STOREOBJ" | "RefCpyV" | "ClrVPtr" | "LOADOBJ" => {
                        ins.words.first().map(|w| *w as i16 as i32).unwrap_or(0)
                    }
                    _ => continue,
                };
                if destination <= 0 {
                    continue;
                }
                let producer = (ins.op.name == "STOREOBJ")
                    .then(|| index.checked_sub(1).and_then(|j| instrs.get(j)))
                    .flatten()
                    .and_then(|previous| match previous.op.name {
                        "CALL" | "CALLINTF" | "CALLBND" => {
                            let id = previous.dwords.first().copied().unwrap_or(0) as i32;
                            Some((refs.func_ret_by_id(id)?, refs.func_by_id(id)?))
                        }
                        "CALLSYS" | "Thiscall1" => {
                            let ptr = previous.qwords.first().copied().unwrap_or(0) as i64;
                            Some((refs.func_ret_by_ptr(ptr)?, refs.func_by_ptr(ptr)?))
                        }
                        _ => None,
                    })
                    .filter(|(ret, _)| ret.token == 5 && (ret.is_object_const || ret.is_read_only))
                    .map(|(_, name)| name.to_owned());
                match producer {
                    Some(name) => {
                        const_source.insert(destination, name);
                    }
                    None => {
                        written_otherwise.insert(destination);
                    }
                }
            }
            for (slot, name) in const_source {
                if written_otherwise.contains(&slot) {
                    unusable.insert(name);
                }
            }
        }
    }
    unusable
}

pub fn prepare_resolver_semantics(
    mods: &[Module],
    refs: &mut RefResolver,
    native: Option<super::binds::NativeApi>,
) {
    let hierarchy = mods
        .iter()
        .flat_map(|module| module.classes.iter())
        .map(|class| {
            (
                class.name.clone(),
                class
                    .super_class
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_default(),
            )
        })
        .collect();
    refs.set_class_hierarchy(hierarchy);

    let fields = mods
        .iter()
        .flat_map(|module| module.classes.iter())
        .map(|class| {
            (
                class.name.clone(),
                class
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.ty.base_name(refs)))
                    .collect(),
            )
        })
        .collect();
    refs.set_class_fields(fields);
    let non_const = mods
        .iter()
        .flat_map(|module| &module.classes)
        .map(|class| {
            let methods = class
                .methods
                .iter()
                .filter(|method| !method.is_const_method())
                .map(|method| method.name.clone())
                .collect::<HashSet<_>>();
            (class.name.clone(), methods)
        })
        .collect();
    refs.set_non_const_methods(non_const);
    let declared = mods
        .iter()
        .flat_map(|module| &module.classes)
        .map(|class| {
            let methods = class
                .methods
                .iter()
                .map(|method| format!("{}/{}", method.name, method.params.len()))
                .collect::<HashSet<_>>();
            (class.name.clone(), methods)
        })
        .collect();
    refs.set_class_methods(declared);
    let mut param_defaults = HashMap::new();
    for module in mods {
        for function in &module.functions {
            if function.param_defaults.iter().any(|d| !d.is_empty()) {
                param_defaults.insert(
                    (String::new(), function.name.clone()),
                    function.param_defaults.clone(),
                );
            }
        }
        for class in &module.classes {
            for method in class.methods.iter().chain(class.ctors.iter()) {
                if method.param_defaults.iter().any(|d| !d.is_empty()) {
                    param_defaults.insert(
                        (class.name.clone(), method.name.clone()),
                        method.param_defaults.clone(),
                    );
                }
            }
        }
    }
    refs.set_param_defaults(param_defaults);
    refs.set_unusable_const_returns(unusable_const_returns(mods, refs));
    refs.add_method_names(
        mods.iter()
            .flat_map(|module| module.classes.iter())
            .flat_map(|class| class.methods.iter())
            .map(|method| method.name.clone()),
    );
    if let Some(native) = native {
        refs.set_native_api(native);
    }
}

/// Populate every resolver input required by a full-tree emit and return the exact deterministic
/// declaration plan consumed by the opaque prepared API.
fn prepare_resolver_for_emit(
    mods: &[Module],
    refs: &mut RefResolver,
    native: Option<super::binds::NativeApi>,
) -> Result<FreeFunctionRenamePlan, String> {
    prepare_resolver_semantics(mods, refs, native);
    let plan = free_function_rename_plan(mods, refs)?;
    let mut rename_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (module_index, names) in plan.per_module.iter().enumerate() {
        if names.is_empty() {
            continue;
        }
        rename_map.insert(
            mods[module_index].name.clone(),
            names.iter().map(|(a, b)| (a.clone(), b.clone())).collect(),
        );
    }
    refs.set_free_fn_renames(&rename_map);
    Ok(plan)
}

/// Mirror the exact free-function inclusion and overload identity used by `emit::emit_module`.
/// These helpers deliberately live together; changing the emitter's rules requires updating the
/// synthetic parity tests in `tests/emit_all_test.rs`.
fn emitted_free_functions<'a>(module: &'a Module, refs: &RefResolver) -> Vec<(&'a Func, String)> {
    let class_names: HashSet<&str> = module
        .classes
        .iter()
        .map(|class| class.name.as_str())
        .collect();
    let class_members: HashMap<&str, HashSet<&str>> = module
        .classes
        .iter()
        .map(|class| {
            (
                class.name.as_str(),
                class
                    .methods
                    .iter()
                    .chain(class.ctors.iter())
                    .map(|function| function.name.as_str())
                    .collect(),
            )
        })
        .collect();
    let mut seen = HashSet::new();
    module
        .functions
        .iter()
        .filter(|function| {
            !is_generated_function(function, &class_names, &class_members)
                && !super::emit::is_generated_spawn(function, refs)
        })
        .filter_map(|function| {
            let params = free_param_signature(function, refs);
            // The collision key carries the NAMESPACE. Two same-named free functions in
            // different namespaces are already distinct declarations that a qualified call
            // reaches unambiguously; renaming them would only invent a symbol the base cache
            // does not have. Only a genuine global-scope clash still needs the rename.
            let signature = format!("{}::{}({params})", function.namespace, function.name);
            seen.insert(signature.clone())
                .then_some((function, signature))
        })
        .collect()
}

fn is_generated_function(
    function: &Func,
    class_names: &HashSet<&str>,
    class_members: &HashMap<&str, HashSet<&str>>,
) -> bool {
    function.name == "StaticClass"
        || class_names.contains(function.name.as_str())
        || class_members
            .get(function.namespace.as_str())
            .is_some_and(|members| members.contains(function.name.as_str()))
}

fn free_param_signature(function: &Func, refs: &RefResolver) -> String {
    function
        .params
        .iter()
        .map(|parameter| {
            let ty = parameter.ty.render(refs);
            let reference = if parameter.ty.is_reference {
                match parameter.flags & 3 {
                    2 => "&out",
                    3 => "&inout",
                    _ => "&in",
                }
            } else {
                ""
            };
            format!("{ty}{reference}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn function_parameter_signature(function: &Func, refs: &RefResolver) -> ParameterSignature {
    ParameterSignature(
        function
            .params
            .iter()
            .map(|parameter| {
                let rendered = parameter.ty.render(refs);
                let mut tokens = code_tokens(&rendered)
                    .iter()
                    .map(|token| token_text(&rendered, token).to_owned())
                    .collect::<Vec<_>>();
                if parameter.ty.is_reference {
                    tokens.push("&".into());
                    tokens.push(
                        match parameter.flags & 3 {
                            2 => "out",
                            3 => "inout",
                            _ => "in",
                        }
                        .into(),
                    );
                }
                tokens
            })
            .collect(),
    )
}

fn free_function_rename_plan(
    mods: &[Module],
    refs: &RefResolver,
) -> Result<FreeFunctionRenamePlan, String> {
    let mut sig_mods: HashMap<String, HashSet<usize>> = HashMap::new();
    let emitted = mods
        .iter()
        .map(|module| emitted_free_functions(module, refs))
        .collect::<Vec<_>>();
    for (module_index, _) in mods.iter().enumerate() {
        for (_, signature) in &emitted[module_index] {
            sig_mods
                .entry(signature.clone())
                .or_default()
                .insert(module_index);
        }
    }

    let mut colliding = vec![BTreeSet::new(); mods.len()];
    for (module_index, _) in mods.iter().enumerate() {
        for (function, signature) in &emitted[module_index] {
            if sig_mods
                .get(signature)
                .is_some_and(|participants| participants.len() > 1)
            {
                // One colliding overload binds the whole same-name family in every participating
                // module. The resolver map is function-id based, so cross-module calls to the
                // non-colliding siblings follow the declaration rename as well.
                colliding[module_index].insert(function.name.clone());
            }
        }
    }

    let mut reserved = BTreeSet::new();
    for module in mods {
        reserved.extend(
            module
                .functions
                .iter()
                .map(|function| function.name.clone()),
        );
        reserved.extend(module.globals.iter().map(|global| global.name.clone()));
        reserved.extend(module.classes.iter().map(|class| class.name.clone()));
        reserved.extend(
            module
                .enums
                .iter()
                .map(|definition| definition.name.clone()),
        );
    }
    let mut plan = FreeFunctionRenamePlan {
        per_module: vec![BTreeMap::new(); mods.len()],
        required_signatures: vec![BTreeMap::new(); mods.len()],
        original_names: BTreeSet::new(),
        safe_global_call_arities: BTreeMap::new(),
    };
    for (module_index, names) in colliding.into_iter().enumerate() {
        for name in names {
            let preferred = format!("{name}_g{module_index}");
            let mut target = preferred.clone();
            let mut discriminator = 0usize;
            while reserved.contains(&target) || refs.native_name_exists(&target) {
                discriminator += 1;
                target = format!("{preferred}_r{discriminator}");
            }
            reserved.insert(target.clone());
            plan.original_names.insert(name.clone());
            let signatures = emitted[module_index]
                .iter()
                .filter(|(function, _)| function.name == name)
                .map(|(function, _)| function_parameter_signature(function, refs))
                .collect::<BTreeSet<_>>();
            plan.required_signatures[module_index].insert(name.clone(), signatures);
            plan.per_module[module_index].insert(name, target);
        }
    }

    // The bytecode emitter explicitly qualifies calls that retain an original collision-family
    // name. Such a call is safe only when its arity names an unrenamed overload and no renamed
    // overload could accept that many arguments. This keeps emitted-source round trips usable
    // without allowing `::Name(...)` to bind a different cached collision in the sparse compiler.
    let mut renamed_max_arities = BTreeMap::<String, usize>::new();
    let mut unrenamed_arities = BTreeMap::<String, BTreeSet<usize>>::new();
    for (module_index, functions) in emitted.iter().enumerate() {
        for (function, _) in functions {
            if !plan.original_names.contains(&function.name) {
                continue;
            }
            if plan.renamed(module_index, &function.name).is_some() {
                renamed_max_arities
                    .entry(function.name.clone())
                    .and_modify(|arity| *arity = (*arity).max(function.params.len()))
                    .or_insert(function.params.len());
            } else {
                unrenamed_arities
                    .entry(function.name.clone())
                    .or_default()
                    .insert(function.params.len());
            }
        }
    }
    for (name, arities) in unrenamed_arities {
        let Some(renamed_max_arity) = renamed_max_arities.get(&name) else {
            continue;
        };
        let safe = arities
            .into_iter()
            .filter(|arity| arity > renamed_max_arity)
            .collect::<BTreeSet<_>>();
        if !safe.is_empty() {
            plan.safe_global_call_arities.insert(name, safe);
        }
    }

    // A rename plan is acceptable only if the final loose-source global scope has no duplicate
    // emitted free-function signature. Treat any residue as an internal error before writing.
    let mut final_signatures = BTreeMap::<String, usize>::new();
    for (module_index, functions) in emitted.iter().enumerate() {
        for (function, _) in functions {
            let name = plan
                .renamed(module_index, &function.name)
                .unwrap_or(&function.name);
            // Same key as the plan: a namespace is part of the declaration's identity, so two
            // same-named functions in different namespaces are not a duplicate.
            let signature = format!(
                "{}::{name}({})",
                function.namespace,
                free_param_signature(function, refs)
            );
            if let Some(previous) = final_signatures.insert(signature.clone(), module_index) {
                return Err(format!(
                    "free-function signature {signature} remains duplicated in modules {:?} and {:?}",
                    mods[previous].name, mods[module_index].name
                ));
            }
        }
    }
    Ok(plan)
}

#[derive(Debug, Clone, Copy)]
struct CodeToken {
    start: usize,
    end: usize,
    brace_depth: usize,
    identifier: bool,
}

#[derive(Debug, Clone, Copy)]
struct FunctionDeclaration {
    name_token: usize,
    open_paren: usize,
    close_paren: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexState {
    Code,
    LineComment,
    BlockComment,
    Quoted(u8),
}

fn code_tokens(source: &str) -> Vec<CodeToken> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut state = LexState::Code;
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        match state {
            LexState::Code => {
                if bytes[index..].starts_with(b"//") {
                    state = LexState::LineComment;
                    index += 2;
                } else if bytes[index..].starts_with(b"/*") {
                    state = LexState::BlockComment;
                    index += 2;
                } else if matches!(bytes[index], b'\'' | b'"') {
                    state = LexState::Quoted(bytes[index]);
                    index += 1;
                } else if bytes[index] == b'{' {
                    tokens.push(CodeToken {
                        start: index,
                        end: index + 1,
                        brace_depth: depth,
                        identifier: false,
                    });
                    depth += 1;
                    index += 1;
                } else if bytes[index] == b'}' {
                    depth = depth.saturating_sub(1);
                    tokens.push(CodeToken {
                        start: index,
                        end: index + 1,
                        brace_depth: depth,
                        identifier: false,
                    });
                    index += 1;
                } else if is_identifier_start(bytes[index]) {
                    let start = index;
                    index += 1;
                    while index < bytes.len() && is_identifier_continue(bytes[index]) {
                        index += 1;
                    }
                    tokens.push(CodeToken {
                        start,
                        end: index,
                        brace_depth: depth,
                        identifier: true,
                    });
                } else if bytes[index].is_ascii_whitespace() {
                    index += 1;
                } else {
                    tokens.push(CodeToken {
                        start: index,
                        end: index + 1,
                        brace_depth: depth,
                        identifier: false,
                    });
                    index += 1;
                }
            }
            LexState::LineComment => {
                if bytes[index] == b'\n' {
                    state = LexState::Code;
                }
                index += 1;
            }
            LexState::BlockComment => {
                if bytes[index..].starts_with(b"*/") {
                    state = LexState::Code;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            LexState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else {
                    if bytes[index] == quote {
                        state = LexState::Code;
                    }
                    index += 1;
                }
            }
        }
    }
    tokens
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn token_text<'a>(source: &'a str, token: &CodeToken) -> &'a str {
    &source[token.start..token.end]
}

fn matching_token(
    source: &str,
    tokens: &[CodeToken],
    open: usize,
    left: &str,
    right: &str,
) -> Option<usize> {
    if token_text(source, tokens.get(open)?) != left {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token_text(source, token) {
            value if value == left => depth += 1,
            value if value == right => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn function_declarations(source: &str, tokens: &[CodeToken]) -> Vec<FunctionDeclaration> {
    let mut declarations = Vec::new();
    for (name_token, token) in tokens.iter().enumerate() {
        if !token.identifier
            || tokens
                .get(name_token + 1)
                .is_none_or(|next| token_text(source, next) != "(")
        {
            continue;
        }
        let open_paren = name_token + 1;
        let Some(close_paren) = matching_token(source, tokens, open_paren, "(", ")") else {
            continue;
        };
        let mut body_open = close_paren + 1;
        while tokens.get(body_open).is_some_and(|next| {
            next.identifier
                && matches!(
                    token_text(source, next),
                    "const" | "final" | "override" | "property"
                )
        }) {
            body_open += 1;
        }
        if tokens
            .get(body_open)
            .is_some_and(|next| token_text(source, next) == "{")
        {
            declarations.push(FunctionDeclaration {
                name_token,
                open_paren,
                close_paren,
            });
        }
    }
    declarations
}

fn rewrite_top_level_declarations(source: &str, renames: &BTreeMap<String, String>) -> String {
    let tokens = code_tokens(source);
    let declarations = function_declarations(source, &tokens);
    let mut replacements = Vec::<(usize, usize, &str)>::new();
    for declaration in declarations {
        let identifier = tokens[declaration.name_token];
        if identifier.brace_depth != 0 {
            continue;
        }
        let name = &source[identifier.start..identifier.end];
        if let Some(target) = renames.get(name) {
            replacements.push((identifier.start, identifier.end, target.as_str()));
        }
    }
    if replacements.is_empty() {
        return source.to_owned();
    }
    let extra = replacements
        .iter()
        .map(|(start, end, target)| target.len().saturating_sub(end - start))
        .sum::<usize>();
    let mut output = String::with_capacity(source.len() + extra);
    let mut copied = 0usize;
    for (start, end, target) in replacements {
        output.push_str(&source[copied..start]);
        output.push_str(target);
        copied = end;
    }
    output.push_str(&source[copied..]);
    output
}

fn declaration_parameter_segments<'a>(
    source: &str,
    tokens: &'a [CodeToken],
    declaration: FunctionDeclaration,
) -> Option<Vec<&'a [CodeToken]>> {
    let inner = &tokens[declaration.open_paren + 1..declaration.close_paren];
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut segments = Vec::<&[CodeToken]>::new();
    let mut start = 0usize;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    for (index, token) in inner.iter().enumerate() {
        match token_text(source, token) {
            "<" => angle += 1,
            ">" => angle = angle.checked_sub(1)?,
            "(" => paren += 1,
            ")" => paren = paren.checked_sub(1)?,
            "[" => bracket += 1,
            "]" => bracket = bracket.checked_sub(1)?,
            "{" => brace += 1,
            "}" => brace = brace.checked_sub(1)?,
            "," if angle == 0 && paren == 0 && bracket == 0 && brace == 0 => {
                if start == index {
                    return None;
                }
                segments.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if angle != 0 || paren != 0 || bracket != 0 || brace != 0 || start == inner.len() {
        return None;
    }
    segments.push(&inner[start..]);
    Some(segments)
}

fn parameter_matches_expected(source: &str, segment: &[CodeToken], expected: &[String]) -> bool {
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut end = segment.len();
    for (index, token) in segment.iter().enumerate() {
        let valid = match token_text(source, token) {
            "<" => {
                angle += 1;
                true
            }
            ">" => angle.checked_sub(1).is_some_and(|next| {
                angle = next;
                true
            }),
            "(" => {
                paren += 1;
                true
            }
            ")" => paren.checked_sub(1).is_some_and(|next| {
                paren = next;
                true
            }),
            "[" => {
                bracket += 1;
                true
            }
            "]" => bracket.checked_sub(1).is_some_and(|next| {
                bracket = next;
                true
            }),
            "{" => {
                brace += 1;
                true
            }
            "}" => brace.checked_sub(1).is_some_and(|next| {
                brace = next;
                true
            }),
            "=" if angle == 0 && paren == 0 && bracket == 0 && brace == 0 => {
                end = index;
                break;
            }
            _ => true,
        };
        if !valid {
            return false;
        }
    }
    if end == segment.len() && (angle != 0 || paren != 0 || bracket != 0 || brace != 0) {
        return false;
    }
    let declaration = &segment[..end];
    let exact = declaration.len() == expected.len()
        && declaration
            .iter()
            .zip(expected)
            .all(|(actual, expected)| token_text(source, actual) == expected);
    let named = declaration.len() == expected.len() + 1
        && declaration[..expected.len()]
            .iter()
            .zip(expected)
            .all(|(actual, expected)| token_text(source, actual) == expected)
        && declaration.last().is_some_and(|token| token.identifier);
    exact || named
}

fn declaration_matches_signature(
    source: &str,
    tokens: &[CodeToken],
    declaration: FunctionDeclaration,
    expected: &ParameterSignature,
) -> bool {
    let Some(segments) = declaration_parameter_segments(source, tokens, declaration) else {
        return false;
    };
    segments.len() == expected.0.len()
        && segments
            .iter()
            .zip(&expected.0)
            .all(|(actual, expected)| parameter_matches_expected(source, actual, expected))
}

fn display_signature(signature: &ParameterSignature) -> String {
    signature
        .0
        .iter()
        .map(|parameter| parameter.join(" "))
        .collect::<Vec<_>>()
        .join(", ")
}

fn validate_collision_bound_declarations(
    source: &str,
    renames: &BTreeMap<String, String>,
    required: &BTreeMap<String, BTreeSet<ParameterSignature>>,
) -> Result<(), String> {
    let tokens = code_tokens(source);
    let declarations = function_declarations(source, &tokens);
    let targets = renames
        .iter()
        .map(|(original, target)| (target.as_str(), original.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeMap::<String, BTreeSet<ParameterSignature>>::new();
    for declaration in declarations {
        let identifier = tokens[declaration.name_token];
        if identifier.brace_depth != 0 {
            continue;
        }
        let name = token_text(source, &identifier);
        let original = if renames.contains_key(name) {
            name
        } else if let Some(original) = targets.get(name) {
            original
        } else {
            continue;
        };
        let expected = required
            .get(original)
            .ok_or_else(|| format!("internal plan has no signatures for {original}"))?;
        let matches = expected
            .iter()
            .filter(|signature| {
                declaration_matches_signature(source, &tokens, declaration, signature)
            })
            .cloned()
            .collect::<Vec<_>>();
        let [signature] = matches.as_slice() else {
            return Err(format!(
                "declaration {name}(...) does not match exactly one required signature for {}",
                renames.get(original).map(String::as_str).unwrap_or(name)
            ));
        };
        if !seen
            .entry(original.to_owned())
            .or_default()
            .insert(signature.clone())
        {
            return Err(format!(
                "duplicate collision-bound overload {name}({})",
                display_signature(signature)
            ));
        }
    }
    let mut problems = Vec::new();
    for (original, signatures) in required {
        let actual = seen.get(original).cloned().unwrap_or_default();
        for missing in signatures.difference(&actual) {
            problems.push(format!(
                "missing {}({})",
                renames[original],
                display_signature(missing)
            ));
        }
        for extra in actual.difference(signatures) {
            problems.push(format!(
                "unexpected {}({})",
                renames[original],
                display_signature(extra)
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join(", "))
    }
}

fn unresolved_collision_calls(
    source: &str,
    originals: &BTreeSet<String>,
    safe_global_call_arities: &BTreeMap<String, BTreeSet<usize>>,
) -> BTreeSet<String> {
    let tokens = code_tokens(source);
    let declarations = function_declarations(source, &tokens);
    let declaration_names = declarations
        .iter()
        .map(|declaration| declaration.name_token)
        .collect::<HashSet<_>>();
    let mut unresolved = BTreeSet::new();
    for (index, identifier) in tokens.iter().enumerate() {
        if !identifier.identifier {
            continue;
        }
        let name = token_text(source, identifier);
        if !originals.contains(name) || declaration_names.contains(&index) {
            continue;
        }
        let call = tokens
            .get(index + 1)
            .is_some_and(|token| token_text(source, token) == "(");
        let handle = (index > 0 && token_text(source, &tokens[index - 1]) == "@")
            || (index >= 3
                && token_text(source, &tokens[index - 1]) == ":"
                && token_text(source, &tokens[index - 2]) == ":"
                && token_text(source, &tokens[index - 3]) == "@");
        if !call && !handle {
            continue;
        }
        if index > 0 && token_text(source, &tokens[index - 1]) == "." {
            continue; // explicit object/this/super member, with arbitrary trivia around `.`
        }
        let leading_global = index >= 2
            && token_text(source, &tokens[index - 1]) == ":"
            && token_text(source, &tokens[index - 2]) == ":"
            && (index == 2
                || !tokens[index - 3].identifier
                || is_angelscript_keyword(token_text(source, &tokens[index - 3])));
        if call
            && leading_global
            && call_argument_count(source, &tokens, tokens[index + 1].start).is_some_and(|arity| {
                safe_global_call_arities
                    .get(name)
                    .is_some_and(|safe| safe.contains(&arity))
            })
        {
            continue;
        }
        unresolved.insert(name.to_owned());
    }
    unresolved
}

fn is_angelscript_keyword(value: &str) -> bool {
    matches!(
        value,
        "abstract"
            | "access"
            | "and"
            | "and_eq"
            | "as"
            | "auto"
            | "bool"
            | "break"
            | "case"
            | "cast"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "default"
            | "delegate"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "event"
            | "explicit"
            | "external"
            | "false"
            | "final"
            | "float"
            | "for"
            | "from"
            | "funcdef"
            | "get"
            | "if"
            | "import"
            | "in"
            | "inout"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "interface"
            | "is"
            | "mixin"
            | "namespace"
            | "not"
            | "not_eq"
            | "null"
            | "or"
            | "or_eq"
            | "out"
            | "override"
            | "private"
            | "property"
            | "protected"
            | "return"
            | "set"
            | "shared"
            | "super"
            | "switch"
            | "struct"
            | "this"
            | "true"
            | "try"
            | "typedef"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "void"
            | "while"
            | "xor"
            | "xor_eq"
    )
}

/// Count top-level arguments in one call without mistaking commas in strings, comments, generic
/// type lists, nested calls, indexing, or initializer lists for separators.
fn probable_template_close(source: &str, tokens: &[CodeToken], open_angle: usize) -> Option<usize> {
    let open = tokens
        .binary_search_by_key(&open_angle, |token| token.start)
        .ok()?;
    if token_text(source, &tokens[open]) != "<"
        || open == 0
        || !tokens[open - 1].identifier
        || tokens
            .get(open + 1)
            .is_none_or(|token| matches!(token_text(source, token), "<" | "="))
    {
        return None;
    }

    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token_text(source, token) {
            "<" => {
                if tokens
                    .get(index + 1)
                    .is_some_and(|next| matches!(token_text(source, next), "<" | "="))
                {
                    return None;
                }
                depth += 1;
            }
            ">" => {
                depth = depth.checked_sub(1)?;
                if depth != 0 {
                    continue;
                }
                let next = tokens.get(index + 1)?;
                let valid_suffix = matches!(token_text(source, next), "(" | "[" | "{" | "@" | ".")
                    || (token_text(source, next) == ":"
                        && tokens
                            .get(index + 2)
                            .is_some_and(|after| token_text(source, after) == ":"));
                return valid_suffix.then_some(token.start);
            }
            "(" | ")" | "{" | "}" | ";" => return None,
            _ => {}
        }
    }
    None
}

fn call_argument_count(source: &str, tokens: &[CodeToken], open_paren: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open_paren) != Some(&b'(') {
        return None;
    }
    let mut state = LexState::Code;
    let mut parens = 1usize;
    let mut brackets = 0usize;
    let mut braces = 0usize;
    let mut angles = 0usize;
    let mut complete_arguments = 0usize;
    let mut current_argument = false;
    let mut index = open_paren + 1;
    while index < bytes.len() {
        match state {
            LexState::Code => {
                let call_level = parens == 1 && brackets == 0 && braces == 0;
                let top_level = call_level && angles == 0;
                if bytes[index..].starts_with(b"//") {
                    state = LexState::LineComment;
                    index += 2;
                } else if bytes[index..].starts_with(b"/*") {
                    state = LexState::BlockComment;
                    index += 2;
                } else if matches!(bytes[index], b'\'' | b'"') {
                    if top_level {
                        current_argument = true;
                    }
                    state = LexState::Quoted(bytes[index]);
                    index += 1;
                } else {
                    match bytes[index] {
                        b'(' => {
                            if top_level {
                                current_argument = true;
                            }
                            parens += 1;
                        }
                        b')' if call_level => {
                            if angles != 0 {
                                return None;
                            }
                            return if current_argument {
                                Some(complete_arguments + 1)
                            } else if complete_arguments == 0 {
                                Some(0)
                            } else {
                                None
                            };
                        }
                        b')' => parens = parens.checked_sub(1)?,
                        b'[' => {
                            if top_level {
                                current_argument = true;
                            }
                            brackets += 1;
                        }
                        b']' => brackets = brackets.checked_sub(1)?,
                        b'{' => {
                            if top_level {
                                current_argument = true;
                            }
                            braces += 1;
                        }
                        b'}' => braces = braces.checked_sub(1)?,
                        b'<' if call_level
                            && (angles > 0
                                || probable_template_close(source, tokens, index).is_some()) =>
                        {
                            if top_level {
                                current_argument = true;
                            }
                            angles += 1;
                        }
                        b'>' if call_level && angles > 0 => angles -= 1,
                        b',' if top_level => {
                            if !current_argument {
                                return None;
                            }
                            complete_arguments += 1;
                            current_argument = false;
                        }
                        byte if top_level && !byte.is_ascii_whitespace() => {
                            current_argument = true;
                        }
                        _ => {}
                    }
                    index += 1;
                }
            }
            LexState::LineComment => {
                if bytes[index] == b'\n' {
                    state = LexState::Code;
                }
                index += 1;
            }
            LexState::BlockComment => {
                if bytes[index..].starts_with(b"*/") {
                    state = LexState::Code;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            LexState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else {
                    if bytes[index] == quote {
                        state = LexState::Code;
                    }
                    index += 1;
                }
            }
        }
    }
    None
}

/// Mark bare collision-bound calls in bytecode-derived output as explicitly global. Unlike an
/// authored overlay, the emitter already resolved these call sites by function id/pointer, so this
/// does not guess an overload. The overlay scanner retains the qualifier only for an arity that
/// cannot bind one of the renamed overloads.
fn qualify_emitted_collision_calls(source: &str, originals: &BTreeSet<String>) -> String {
    let tokens = code_tokens(source);
    let declarations = function_declarations(source, &tokens);
    let declaration_names = declarations
        .iter()
        .map(|declaration| declaration.name_token)
        .collect::<HashSet<_>>();
    let mut insertions = Vec::new();
    for (index, identifier) in tokens.iter().enumerate() {
        if !identifier.identifier
            || !originals.contains(token_text(source, identifier))
            || declaration_names.contains(&index)
        {
            continue;
        }
        let call = tokens
            .get(index + 1)
            .is_some_and(|token| token_text(source, token) == "(");
        let handle = index > 0 && token_text(source, &tokens[index - 1]) == "@";
        if !call && !handle {
            continue;
        }
        if index > 0 && token_text(source, &tokens[index - 1]) == "." {
            continue;
        }
        if index >= 2
            && token_text(source, &tokens[index - 1]) == ":"
            && token_text(source, &tokens[index - 2]) == ":"
        {
            continue;
        }
        insertions.push(identifier.start);
    }
    if insertions.is_empty() {
        return source.to_owned();
    }
    let mut output = String::with_capacity(source.len() + insertions.len() * 2);
    let mut copied = 0usize;
    for position in insertions {
        output.push_str(&source[copied..position]);
        output.push_str("::");
        copied = position;
    }
    output.push_str(&source[copied..]);
    output
}

#[derive(Debug, Clone)]
struct ModuleLayout {
    relative: String,
    key: String,
}

/// One sealed, add-only source overlay prepared for a project-wide compiler check.
///
/// This type deliberately carries source text rather than an input path: managed callers seal the
/// bytes before entering gore-as, and the project check must never reopen caller-controlled files.
#[derive(Debug, Clone)]
pub(crate) struct CompileAddOverlay<'a> {
    pub module_name: &'a str,
    pub relative_path: &'a str,
    pub source: &'a str,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCompileAddOverlay {
    pub module_name: String,
    pub relative_path: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedModuleIdentity {
    pub module_name: String,
    pub relative_path: String,
}

fn windows_casefold(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

fn module_name_key(name: &str) -> Result<String, String> {
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err(format!("unsafe empty/control-bearing module name {name:?}"));
    }
    Ok(windows_casefold(name))
}

fn windows_reserved_component(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
}

fn normalize_output_path(raw: &str) -> Result<ModuleLayout, String> {
    if raw.is_empty() || raw.chars().any(char::is_control) {
        return Err(format!("unsafe empty/control-bearing output path {raw:?}"));
    }
    let slash = raw.replace('\\', "/");
    if slash.starts_with('/') || slash.ends_with('/') {
        return Err(format!("output path must be a relative file path: {raw:?}"));
    }
    let mut components = Vec::new();
    for component in slash.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".."
            || component.contains(':')
            || component
                .chars()
                .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
            || component.ends_with([' ', '.'])
            || windows_reserved_component(component)
        {
            return Err(format!(
                "unsafe Windows output path component {component:?} in {raw:?}"
            ));
        }
        components.push(component);
    }
    if components.is_empty() {
        return Err(format!("output path has no file component: {raw:?}"));
    }
    let relative = components.join("/");
    Ok(ModuleLayout {
        key: windows_casefold(&relative),
        relative,
    })
}

fn module_output_path(module: &Module) -> Result<ModuleLayout, String> {
    let raw = if module.file.is_empty() {
        format!("{}.as", module.name)
    } else {
        module.file.clone()
    };
    normalize_output_path(&raw)
}

fn validate_module_layout(mods: &[Module]) -> Result<Vec<ModuleLayout>, String> {
    let mut names = BTreeMap::<String, usize>::new();
    let mut paths = BTreeMap::<String, usize>::new();
    let mut layout: Vec<ModuleLayout> = Vec::with_capacity(mods.len());
    for (index, module) in mods.iter().enumerate() {
        let name_key = module_name_key(&module.name)?;
        if let Some(previous) = names.insert(name_key, index) {
            return Err(format!(
                "module names {:?} and {:?} collide under Windows case folding",
                mods[previous].name, module.name
            ));
        }
        let output = module_output_path(module)?;
        if let Some(previous) = overlapping_path(&paths, &output.key) {
            return Err(format!(
                "module output paths {:?} and {:?} collide as the same path or as a file/directory ancestor under Windows",
                layout[previous].relative, output.relative
            ));
        }
        paths.insert(output.key.clone(), index);
        layout.push(output);
    }
    Ok(layout)
}

/// Validate the exact module-name/path manifest produced by a regenerated cache. Keeping this on
/// the emitter's layout primitive prevents the project checker from growing a second, subtly
/// different Windows-path policy.
pub(crate) fn validated_module_identities(
    mods: &[Module],
) -> Result<Vec<ValidatedModuleIdentity>, String> {
    let layout = validate_module_layout(mods)?;
    Ok(mods
        .iter()
        .zip(layout)
        .map(|(module, output)| ValidatedModuleIdentity {
            module_name: module.name.clone(),
            relative_path: output.relative,
        })
        .collect())
}

fn path_keys_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn overlapping_path(paths: &BTreeMap<String, usize>, key: &str) -> Option<usize> {
    if let Some(index) = paths.get(key) {
        return Some(*index);
    }
    for (separator, _) in key.match_indices('/') {
        if let Some(index) = paths.get(&key[..separator]) {
            return Some(*index);
        }
    }
    let descendant_prefix = format!("{key}/");
    paths
        .range(descendant_prefix.clone()..)
        .next()
        .filter(|(candidate, _)| candidate.starts_with(&descendant_prefix))
        .map(|(_, index)| *index)
}

/// Opaque, fully prepared view of one parsed cache. Construction validates the complete module
/// layout, installs resolver semantics and function-id collision renames, and proves that the
/// final emitted free-function signatures are unique.
pub struct PreparedEmit<'a> {
    mods: &'a [Module],
    refs: &'a RefResolver,
    rename_plan: FreeFunctionRenamePlan,
    layout: Vec<ModuleLayout>,
    class_defaults: bool,
}

impl<'a> PreparedEmit<'a> {
    pub fn new(
        mods: &'a [Module],
        refs: &'a mut RefResolver,
        native: Option<super::binds::NativeApi>,
    ) -> Result<Self, EmitAllError> {
        let layout = validate_module_layout(mods).map_err(EmitAllError::InvalidLayout)?;
        let rename_plan =
            prepare_resolver_for_emit(mods, refs, native).map_err(EmitAllError::InvalidLayout)?;
        Ok(Self {
            mods,
            refs,
            rename_plan,
            layout,
            class_defaults: false,
        })
    }

    /// Emit one module using the same full-cache resolver and collision plan as `emit_tree`.
    /// Write class `default` statements. OFF unless opted into: emitted source is also hashed
    /// into sealed evidence and fed back to the compiler, and both need the historical shape.
    /// Turn it on for source a person is going to read.
    pub fn with_class_defaults(mut self, class_defaults: bool) -> Self {
        self.class_defaults = class_defaults;
        self
    }

    pub fn emit_module(&self, module_index: usize) -> Result<String, EmitAllError> {
        let module = self
            .mods
            .get(module_index)
            .ok_or(EmitAllError::InvalidModuleIndex {
                index: module_index,
                modules: self.mods.len(),
            })?;
        let source = super::emit::emit_module_with(module, self.refs, self.class_defaults);
        Ok(self
            .rename_plan
            .rewrite_emitted_module(module_index, &source))
    }

    /// Normalized module output paths validated by the exact full-tree compile layout rules.
    pub(super) fn collision_relative_paths(&self) -> impl Iterator<Item = &str> {
        self.layout.iter().map(|layout| layout.relative.as_str())
    }

    /// Raw collision-bound leaves plus every deterministic final declaration rename.
    pub(super) fn collision_rename_names(&self) -> impl Iterator<Item = &str> {
        self.rename_plan
            .original_names
            .iter()
            .map(String::as_str)
            .chain(
                self.rename_plan
                    .per_module
                    .iter()
                    .flat_map(|renames| renames.values().map(String::as_str)),
            )
    }

    /// Validate and rewrite an authored overlay against this prepared cache. Ambiguous bare,
    /// global, or namespace-qualified calls and handles to a collision-bound name fail closed.
    /// Explicit `receiver.Name` access and arity-disjoint emitted global calls remain safe.
    pub fn prepare_overlay(
        &self,
        op: &str,
        module_name: &str,
        source: &str,
    ) -> Result<String, String> {
        self.rename_plan
            .prepare_overlay(self.mods, op, module_name, source)
    }

    pub(crate) fn prepare_compile_overlay(
        &self,
        op: &str,
        module_name: &str,
        rel_path: &str,
        source: &str,
    ) -> Result<(String, String), String> {
        let requested = normalize_output_path(rel_path)?;
        let output_relative = match op {
            "edit" => {
                let indices = self
                    .mods
                    .iter()
                    .enumerate()
                    .filter(|(_, module)| module.name == module_name)
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let [module_index] = indices.as_slice() else {
                    return Err(format!(
                        "edit module {module_name:?} must identify exactly one base module (found {})",
                        indices.len()
                    ));
                };
                let expected = &self.layout[*module_index];
                if requested.key != expected.key {
                    return Err(format!(
                        "edit path {:?} does not match base module {:?} path {:?}",
                        requested.relative, module_name, expected.relative
                    ));
                }
                expected.relative.clone()
            }
            "add" => {
                let requested_name = module_name_key(module_name)?;
                if let Some(existing) = self
                    .mods
                    .iter()
                    .find(|module| windows_casefold(&module.name) == requested_name)
                {
                    return Err(format!(
                        "add module name {module_name:?} collides with base module {:?}",
                        existing.name
                    ));
                }
                if let Some((index, _)) = self
                    .layout
                    .iter()
                    .enumerate()
                    .find(|(_, output)| path_keys_overlap(&output.key, &requested.key))
                {
                    return Err(format!(
                        "add path {:?} collides with base module {:?} path {:?} as the same path or a file/directory ancestor",
                        requested.relative, self.mods[index].name, self.layout[index].relative
                    ));
                }
                requested.relative.clone()
            }
            _ => return Err(format!("invalid overlay operation {op:?}")),
        };
        let source = self.prepare_overlay(op, module_name, source)?;
        Ok((source, output_relative))
    }

    /// Prepare a complete set of sealed add-only overlays for one shared compiler tree.
    ///
    /// All identity and layout checks finish before any source is returned to the caller, which in
    /// turn finishes before `emit_tree` can touch the workspace. The returned order is stable and
    /// independent of store/entity iteration order.
    pub(crate) fn prepare_compile_add_overlays(
        &self,
        overlays: &[CompileAddOverlay<'_>],
    ) -> Result<Vec<PreparedCompileAddOverlay>, String> {
        let mut overlay_names = BTreeMap::<String, usize>::new();
        let mut overlay_paths = BTreeMap::<String, usize>::new();
        let mut prepared = Vec::with_capacity(overlays.len());

        for (index, overlay) in overlays.iter().enumerate() {
            let name_key = module_name_key(overlay.module_name)?;
            let namespace_segments = overlay.module_name.split('.').collect::<Vec<_>>();
            if namespace_segments.iter().any(|segment| {
                segment.is_empty()
                    || !segment.chars().next().is_some_and(|character| {
                        character == '_' || character.is_ascii_alphabetic()
                    })
                    || segment
                        .chars()
                        .any(|character| character != '_' && !character.is_ascii_alphanumeric())
            }) {
                return Err(format!(
                    "add module namespace {:?} is not a dot-separated AngelScript identifier",
                    overlay.module_name
                ));
            }

            if let Some(base) = self
                .mods
                .iter()
                .find(|module| windows_casefold(&module.name) == name_key)
            {
                return Err(format!(
                    "add module namespace {:?} collides with base module {:?} under Windows case folding",
                    overlay.module_name, base.name
                ));
            }
            if let Some(previous) = overlay_names.insert(name_key.clone(), index) {
                return Err(format!(
                    "add module namespaces {:?} and {:?} collide under Windows case folding",
                    overlays[previous].module_name, overlay.module_name
                ));
            }

            let requested = normalize_output_path(overlay.relative_path)?;
            if let Some((base_index, _)) = self
                .layout
                .iter()
                .enumerate()
                .find(|(_, base)| path_keys_overlap(&base.key, &requested.key))
            {
                return Err(format!(
                    "add path {:?} collides with base module {:?} path {:?} as the same path or a file/directory ancestor",
                    requested.relative,
                    self.mods[base_index].name,
                    self.layout[base_index].relative
                ));
            }
            if let Some(previous) = overlapping_path(&overlay_paths, &requested.key) {
                return Err(format!(
                    "add paths {:?} and {:?} collide under Windows case folding as the same path or a file/directory ancestor",
                    prepared
                        .get(previous)
                        .map(|value: &PreparedCompileAddOverlay| value.relative_path.as_str())
                        .unwrap_or(overlays[previous].relative_path),
                    requested.relative
                ));
            }
            overlay_paths.insert(requested.key.clone(), index);

            let expected =
                normalize_output_path(&format!("{}.as", overlay.module_name.replace('.', "/")))?;
            if requested.relative != expected.relative {
                return Err(format!(
                    "add module namespace {:?} requires relative path {:?}, got {:?}",
                    overlay.module_name, expected.relative, requested.relative
                ));
            }

            let source = self.prepare_overlay("add", overlay.module_name, overlay.source)?;
            prepared.push(PreparedCompileAddOverlay {
                module_name: overlay.module_name.to_owned(),
                relative_path: requested.relative,
                source,
            });
        }

        prepared.sort_by(|left, right| {
            windows_casefold(&left.relative_path)
                .cmp(&windows_casefold(&right.relative_path))
                .then_with(|| {
                    windows_casefold(&left.module_name).cmp(&windows_casefold(&right.module_name))
                })
                .then_with(|| left.relative_path.cmp(&right.relative_path))
                .then_with(|| left.module_name.cmp(&right.module_name))
        });
        Ok(prepared)
    }

    /// Emit the prepared full tree. Module identities and normalized paths were validated by
    /// `new` before this method can create the output directory or write a file.
    pub fn emit_tree(&self, outdir: &Path) -> Result<EmitAllStats, EmitAllError> {
        let io = |ctx: &str| {
            let ctx = ctx.to_string();
            move |error: std::io::Error| EmitAllError::Io(format!("{ctx}: {error}"))
        };
        std::fs::create_dir_all(outdir).map_err(io(&format!("creating {}", outdir.display())))?;
        let outdir = outdir
            .canonicalize()
            .map_err(io(&format!("resolving {}", outdir.display())))?;

        for output in &self.layout {
            let mut current = outdir.clone();
            for component in output.relative.split('/') {
                current.push(component);
                if std::fs::symlink_metadata(&current)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return Err(EmitAllError::InvalidLayout(format!(
                        "symlinked output path component {}",
                        current.display()
                    )));
                }
            }
        }

        let (
            mut written,
            mut functions,
            mut cache_function_records,
            mut stubbed,
            mut stubbed_functions,
        ) = (0usize, 0usize, 0usize, 0usize, 0usize);
        for (module_index, module) in self.mods.iter().enumerate() {
            let source = self.emit_module(module_index)?;
            functions += super::emit::emitted_body_count(module, self.refs);
            cache_function_records += module.functions.len()
                + module
                    .classes
                    .iter()
                    .map(|class| class.methods.len() + class.ctors.len())
                    .sum::<usize>();
            let module_stubs = source.matches("body not fully recovered — stub [").count();
            if module_stubs != 0 {
                stubbed += 1;
                stubbed_functions += module_stubs;
            }
            let path = outdir.join(&self.layout[module_index].relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(io(&format!("creating {}", parent.display())))?;
            }
            std::fs::write(&path, source).map_err(io(&format!("writing {}", path.display())))?;
            written += 1;
        }
        Ok(EmitAllStats {
            written,
            functions,
            cache_function_records,
            stubbed,
            stubbed_functions,
        })
    }
}

/// Convenience entry point for a complete loose-source tree.
pub fn emit_all_tree(
    mods: &[Module],
    refs: &mut RefResolver,
    native: Option<super::binds::NativeApi>,
    outdir: &Path,
) -> Result<EmitAllStats, EmitAllError> {
    PreparedEmit::new(mods, refs, native)?.emit_tree(outdir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signature(parameters: &[&str]) -> ParameterSignature {
        ParameterSignature(
            parameters
                .iter()
                .map(|parameter| {
                    code_tokens(parameter)
                        .iter()
                        .map(|token| token_text(parameter, token).to_owned())
                        .collect()
                })
                .collect(),
        )
    }

    fn declaration_matches(source: &str, expected: &ParameterSignature) -> bool {
        let tokens = code_tokens(source);
        let declarations = function_declarations(source, &tokens);
        let [declaration] = declarations.as_slice() else {
            panic!("expected exactly one function declaration in {source:?}");
        };
        declaration_matches_signature(source, &tokens, *declaration, expected)
    }

    #[test]
    fn declaration_rewrite_touches_only_top_level_function_names() {
        let source = r#"// Foo stays
void Foo() {}
void Caller() { Foo(); Object.Foo(); }
class C { void Foo() { Foo(); } }
const FName Label = n"Foo";
"#;
        let rewritten = rewrite_top_level_declarations(
            source,
            &BTreeMap::from([("Foo".to_owned(), "Foo_g0".to_owned())]),
        );
        assert!(rewritten.contains("void Foo_g0() {}"));
        assert!(rewritten.contains("void Caller() { Foo(); Object.Foo(); }"));
        assert!(rewritten.contains("class C { void Foo() { Foo(); } }"));
        assert!(rewritten.contains("// Foo stays"));
        assert!(rewritten.contains("n\"Foo\""));
    }

    #[test]
    fn emitted_collision_calls_are_globally_qualified_without_touching_other_tokens() {
        let source = r#"// Shared() stays
void Shared(int Value) {}
void Caller(bool Flag) {
    Shared(1); Object.Shared(); Namespace::Shared(); ::Shared(2);
    int Value = Flag ? Shared(3) : Shared(4);
    switch (Value) { case 1: Shared(5); }
    Callback@ Cb = @Shared;
}
const FName Label = n"Shared() @Shared";
"#;
        let qualified =
            qualify_emitted_collision_calls(source, &BTreeSet::from(["Shared".to_owned()]));
        assert!(qualified.contains("void Shared(int Value) {}"));
        assert!(
            qualified.contains("::Shared(1); Object.Shared(); Namespace::Shared(); ::Shared(2);")
        );
        assert!(qualified.contains("Flag ? ::Shared(3) : ::Shared(4)"));
        assert!(qualified.contains("case 1: ::Shared(5);"));
        assert!(qualified.contains("Callback@ Cb = @::Shared;"));
        assert!(qualified.contains("// Shared() stays"));
        assert!(qualified.contains("n\"Shared() @Shared\""));
    }

    #[test]
    fn collision_call_arity_distinguishes_operators_from_template_arguments() {
        let originals = BTreeSet::from(["Shared".to_owned()]);
        let safe_arities = BTreeMap::from([("Shared".to_owned(), BTreeSet::from([1, 2]))]);
        for source in [
            "void Caller(int a, int b) { ::Shared(a < b); }",
            "void Caller(int flags, int other) { ::Shared(flags << 1, other); }",
            "void Caller(int a, int b, int c, int d) { ::Shared(a <= b, c >= d); }",
            "void Caller() { ::Shared(TMap<int, TArray<float>>(), 1); }",
        ] {
            assert!(
                unresolved_collision_calls(source, &originals, &safe_arities).is_empty(),
                "safe global call was rejected in {source:?}"
            );
        }
    }

    #[test]
    fn exact_parameter_matching_preserves_token_boundaries_and_nested_type_syntax() {
        let foo_bar = signature(&["FooBar"]);
        assert!(declaration_matches(
            "void Shared(FooBar Value = FooBar()) {}",
            &foo_bar
        ));
        assert!(!declaration_matches("void Shared(Foo Bar) {}", &foo_bar));

        let const_foo = signature(&["const Foo"]);
        assert!(declaration_matches(
            "void Shared(const /* trivia */ Foo Value) {}",
            &const_foo
        ));
        assert!(!declaration_matches(
            "void Shared(constFoo Value) {}",
            &const_foo
        ));

        let nested = signature(&["const TArray<Foo@>[] &in"]);
        assert!(declaration_matches(
            "void Shared(const TArray<Foo@>[] /* trivia */ &in Values = Make<Foo@>(1, 2)) {}",
            &nested
        ));
        assert!(!declaration_matches(
            "void Shared(const TArray<Foo@>[] &out Values) {}",
            &nested
        ));
        assert!(!declaration_matches(
            "void Shared(const TArray<Foo@>[] &in Values Extra) {}",
            &nested
        ));
    }

    #[test]
    fn edit_path_casefold_match_uses_the_base_modules_canonical_relative_path() {
        let modules = vec![Module {
            name: "Fixture".into(),
            file: "Dir/Fixture.as".into(),
            functions: Vec::new(),
            classes: Vec::new(),
            enums: Vec::new(),
            globals: Vec::new(),
        }];
        let mut refs = RefResolver::default();
        let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
        let (_, relative) = prepared
            .prepare_compile_overlay("edit", "Fixture", "dir\\FIXTURE.AS", "// replacement")
            .unwrap();
        assert_eq!(relative, "Dir/Fixture.as");
    }
}
