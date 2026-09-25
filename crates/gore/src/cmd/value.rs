//! Inspect and rewrite game-defined class defaults from the Shipping script cache.
//!
//! Discovery reads the pristine cache and sibling `Binds.Cache` through the existing emitter.
//! It does not read a UE4SS object dump or a `.usmap`. Only a class-scope `default` assignment
//! or one `Field.Add(GameplayTag::Tag, scalar)` entry is writable. Trader stock, instance
//! inventories, and values already stored in a save are reported as outside this mechanism.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gore_as::cache::emit_all::PreparedEmit;
use gore_as::cache::faithfulness;
use gore_as::cache::model;
use gore_as::cache::refs::RefResolver;
use gore_mod::{ScriptModule, ValueEdit, ValueLiteral};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::as_cache::{
    load_native_api_with_proof, read_module_cache, AsCmd, AsCompilerBackendArgsV1,
    AsCompilerBackendV1,
};
use super::npc::defaults::{self, EmittedClass};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultKind {
    Assignment,
    TagMap,
    UnsupportedCall,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InspectedDefault {
    pub field: String,
    pub kind: DefaultKind,
    pub type_name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassInspection {
    pub class: String,
    pub module: String,
    pub relative_path: String,
    pub cache_sha256: String,
    pub binds_sha256: Option<String>,
    pub defaults: Vec<InspectedDefault>,
    pub outside_class_defaults: &'static [&'static str],
}

const OUTSIDE: &[&str] = &["trader_stock", "instance_inventory", "save_values"];

#[derive(Subcommand)]
pub enum ValueAction {
    /// Show one class's recovered defaults, their types, and the current values
    Inspect {
        /// Exact AngelScript class, for example `UItFo_Apple`
        #[arg(long)]
        class: String,
        /// Pristine Shipping script cache. Overrides `--game` when both are passed.
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. The pristine cache is selected, including an owned backup.
        #[arg(long)]
        game: Option<PathBuf>,
        /// Emit one JSON document
        #[arg(long)]
        json: bool,
    },
}

pub fn run(action: ValueAction) -> Result<()> {
    match action {
        ValueAction::Inspect {
            class,
            cache,
            game,
            json,
        } => inspect(class, cache, game, json),
    }
}

pub fn inspect(
    class: String,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let path = resolve_cache(cache, game)?;
    let report = inspect_class(&path, &class)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("class: {}", report.class);
    println!("module: {}", report.module);
    println!("path: {}", report.relative_path);
    println!("cache: {}", report.cache_sha256);
    if let Some(binds) = &report.binds_sha256 {
        println!("binds: {binds}");
    }
    println!("source: class_default");
    if report.defaults.is_empty() {
        println!("defaults: none recovered");
    }
    for item in &report.defaults {
        let tag = item
            .tag
            .as_deref()
            .map(|tag| format!(" tag={tag}"))
            .unwrap_or_default();
        println!(
            "  {}  {}  {}{tag}  [{}]",
            item.field, item.type_name, item.value, kind_label(&item.kind)
        );
    }
    println!(
        "not class defaults (unsupported here): {}",
        OUTSIDE.join(", ")
    );
    Ok(())
}

fn kind_label(kind: &DefaultKind) -> &'static str {
    match kind {
        DefaultKind::Assignment => "assignment",
        DefaultKind::TagMap => "tag_map",
        DefaultKind::UnsupportedCall => "unsupported",
    }
}

pub fn inspect_class(cache: &Path, class_name: &str) -> Result<ClassInspection> {
    with_prepared(cache, |modules, prepared, bytes, binds_sha256| {
        let (module_index, module_name) = module_of_class(modules, class_name)
            .with_context(|| format!("no shipped class {class_name} in {}", cache.display()))?;
        let source = prepared
            .emit_module(module_index)
            .with_context(|| format!("emitting {module_name}"))?;
        let class = defaults::parse_classes(&source)
            .into_iter()
            .find(|class| class.name == class_name)
            .with_context(|| format!("{class_name} was not recovered from {module_name}"))?;
        let relative_path = prepared
            .module_relative_path(module_index)
            .unwrap_or("")
            .to_string();
        Ok(ClassInspection {
            class: class_name.to_string(),
            module: module_name,
            relative_path,
            cache_sha256: hex_sha256(bytes),
            binds_sha256,
            defaults: inspected_defaults(&class),
            outside_class_defaults: OUTSIDE,
        })
    })
}

fn inspected_defaults(class: &EmittedClass) -> Vec<InspectedDefault> {
    let mut out = Vec::new();
    for (field, rhs) in &class.assignments {
        let (type_name, value) = classify_literal(rhs);
        out.push(InspectedDefault {
            field: field.clone(),
            kind: DefaultKind::Assignment,
            type_name: type_name.to_string(),
            value,
            tag: None,
            source: "class_default",
        });
    }
    for call in &class.calls {
        if let Some((field, tag, literal)) = tag_map_entry(call) {
            let (type_name, value) = classify_literal(&literal);
            out.push(InspectedDefault {
                field,
                kind: DefaultKind::TagMap,
                type_name: type_name.to_string(),
                value,
                tag: Some(tag),
                source: "class_default",
            });
        } else {
            out.push(InspectedDefault {
                field: call.clone(),
                kind: DefaultKind::UnsupportedCall,
                type_name: "unsupported".into(),
                value: call.clone(),
                tag: None,
                source: "class_default",
            });
        }
    }
    out
}

/// Compile every `values` edit into `op = "edit"` script modules and append them to `scripts`.
pub fn compile_values_into_scripts(
    game: &Path,
    work_dir: &Path,
    cache: &Path,
    edits: &[ValueEdit],
    out_dir: &Path,
) -> Result<(Vec<ScriptModule>, String)> {
    if edits.is_empty() {
        return Ok((Vec::new(), String::new()));
    }
    with_prepared(cache, |modules, prepared, bytes, _binds| {
        let mut by_module: BTreeMap<usize, Vec<&ValueEdit>> = BTreeMap::new();
        for edit in edits {
            let (index, _) = module_of_class(modules, &edit.class)
                .with_context(|| format!("unknown class {}", edit.class))?;
            by_module.entry(index).or_default().push(edit);
        }
        std::fs::create_dir_all(work_dir)
            .with_context(|| format!("creating {}", work_dir.display()))?;
        std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
        clear_owned_mini_caches(out_dir)?;
        let cache_sha = hex_sha256(bytes);
        let mut scripts = Vec::new();
        for (index, module_edits) in by_module {
            let module_name = modules[index].name.clone();
            let relative = prepared
                .module_relative_path(index)
                .with_context(|| format!("{module_name} has no emit path"))?
                .to_string();
            let source = prepared
                .emit_module(index)
                .with_context(|| format!("emitting {module_name}"))?;
            let rewritten = apply_edits(&source, &module_edits)?;
            prove_only_requested_statements_changed(&source, &rewritten, &module_edits)?;
            let source_path = work_dir.join(format!("{}.as", sanitize(&module_name)));
            std::fs::write(&source_path, &rewritten)
                .with_context(|| format!("writing {}", source_path.display()))?;
            let mini_path = out_dir.join(format!("{}.mini.cache", sanitize(&module_name)));
            super::as_cache::run(AsCmd::CompileModule {
                op: "edit".into(),
                module: module_name.clone(),
                rel_path: relative,
                source: source_path,
                work_dir: work_dir.join(sanitize(&module_name)),
                allow_new_symbols: false,
                out: mini_path.clone(),
                game: Some(game.to_path_buf()),
                expect_base: None,
                expect_base_sha256: Some(cache_sha.clone()),
                expect_source_sha256: None,
                no_diagnostics: true,
                diagnostics_hook: None,
                diagnostics_inject_delay_ms: 0,
                compiler: AsCompilerBackendArgsV1 {
                    backend: AsCompilerBackendV1::Standalone,
                    standalone_sidecar: None,
                    standalone_sidecar_sha256: None,
                    compiler_profile_manifest: None,
                    compiler_profile_root: None,
                    standalone_scratch_root: None,
                    generation_receipt: None,
                },
            })
            .with_context(|| format!("compiling {module_name}"))?;
            scripts.push(ScriptModule {
                op: "edit".into(),
                module_name,
                mini_cache: mini_path.display().to_string(),
            });
        }
        Ok((scripts, cache_sha))
    })
}

pub fn apply_edits(source: &str, edits: &[&ValueEdit]) -> Result<String> {
    let mut current = source.to_string();
    for edit in edits {
        current = apply_one(&current, edit)?;
    }
    Ok(current)
}

fn apply_one(source: &str, edit: &ValueEdit) -> Result<String> {
    let classes = defaults::parse_classes(source);
    let matches: Vec<_> = classes
        .iter()
        .filter(|class| class.name == edit.class)
        .collect();
    if matches.len() != 1 {
        bail!(
            "class {} must occur once in the emitted module, found {}",
            edit.class,
            matches.len()
        );
    }
    let class = matches[0];
    let (start, end) = class_span(source, &class.name)?;
    let body = &source[start..end];
    let rewritten = if let Some(tag) = &edit.tag {
        rewrite_tag(body, &edit.field, tag, &edit.value)?
    } else if class.assignments.iter().any(|(field, _)| field == &edit.field) {
        let old = class
            .assignments
            .iter()
            .find(|(field, _)| field == &edit.field)
            .map(|(_, rhs)| rhs.as_str())
            .unwrap();
        rewrite_assignment(body, &edit.field, old, &edit.value)?
    } else {
        let entries: Vec<_> = class
            .calls
            .iter()
            .filter_map(|call| tag_map_entry(call))
            .filter(|(field, _, _)| field == &edit.field)
            .collect();
        match entries.as_slice() {
            [(field, tag, _)] => rewrite_tag(body, field, tag, &edit.value)?,
            [] => bail!(
                "unsupported or unknown field {}.{} — only a recovered class default assignment \
                 or a single GameplayTag map entry can be written",
                edit.class,
                edit.field
            ),
            _ => bail!(
                "{}.{} has more than one tag entry; pass `tag` to select one",
                edit.class, edit.field
            ),
        }
    };
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..start]);
    out.push_str(&rewritten);
    out.push_str(&source[end..]);
    Ok(out)
}

fn rewrite_assignment(body: &str, field: &str, old: &str, value: &ValueLiteral) -> Result<String> {
    let old_type = classify_literal(old).0;
    if old_type != "unsupported" && old_type != value.type_name() {
        bail!(
            "{field} is {old_type} ({old}); refusing to write {}",
            value.type_name()
        );
    }
    let needle = format!("default {field} = {old};");
    let count = body.matches(&needle).count();
    if count != 1 {
        bail!("expected one `{needle}` in {field}'s class, found {count}");
    }
    let rendered = render_literal(value, old.ends_with('f'))?;
    Ok(body.replacen(&needle, &format!("default {field} = {rendered};"), 1))
}

fn rewrite_tag(body: &str, field: &str, tag: &str, value: &ValueLiteral) -> Result<String> {
    let classes = defaults::parse_classes(body);
    let Some(class) = classes.iter().find(|class| {
        class
            .calls
            .iter()
            .filter_map(|call| tag_map_entry(call))
            .any(|(name, found, _)| name == field && found == tag)
    }) else {
        bail!("could not re-read {field}");
    };
    let Some((_, _, old)) = class
        .calls
        .iter()
        .filter_map(|call| tag_map_entry(call))
        .find(|(name, found, _)| name == field && found == tag)
    else {
        bail!("no class default {field}.Add(GameplayTag::{tag}, ...) to edit");
    };
    let old_type = classify_literal(&old).0;
    if old_type != "unsupported" && old_type != value.type_name() {
        bail!(
            "{field} tag {tag} is {old_type} ({old}); refusing to write {}",
            value.type_name()
        );
    }
    let needle = format!("default {field}.Add(GameplayTag::{tag}, {old});");
    if body.matches(&needle).count() != 1 {
        bail!("tag entry {field}/{tag} is not a single recovered default statement");
    }
    let rendered = render_literal(value, old.ends_with('f'))?;
    Ok(body.replacen(
        &needle,
        &format!("default {field}.Add(GameplayTag::{tag}, {rendered});"),
        1,
    ))
}

fn prove_only_requested_statements_changed(
    before: &str,
    after: &str,
    edits: &[&ValueEdit],
) -> Result<()> {
    if before.lines().count() != after.lines().count() {
        bail!("value edit changed the number of source lines");
    }
    let mut changed = 0usize;
    for (left, right) in before.lines().zip(after.lines()) {
        if left != right {
            changed += 1;
            let interesting = edits.iter().any(|edit| {
                right.contains(&format!("default {}", edit.field))
                    && edit.tag.as_ref().map(|tag| right.contains(tag)).unwrap_or(true)
            });
            if !interesting {
                bail!("value edit changed an unrelated line: {right}");
            }
        }
    }
    if changed == 0 {
        bail!("value edit did not change any default statement");
    }
    Ok(())
}

fn render_literal(value: &ValueLiteral, float_suffix: bool) -> Result<String> {
    Ok(match value {
        ValueLiteral::Int(number) => number.to_string(),
        ValueLiteral::Float(number) => {
            if !number.is_finite() {
                bail!("float values must be finite");
            }
            let mut text = format!("{number}");
            if !text.contains('.') && !text.contains('e') && !text.contains('E') {
                text.push_str(".0");
            }
            if float_suffix {
                text.push('f');
            }
            text
        }
        ValueLiteral::Bool(flag) => if *flag { "true" } else { "false" }.into(),
        ValueLiteral::Str(text) => {
            if text.contains(['\n', '\r', '"', '\\']) {
                bail!("string defaults containing quotes, backslashes, or newlines are unsupported");
            }
            format!("\"{text}\"")
        }
    })
}

fn classify_literal(rhs: &str) -> (&'static str, String) {
    let text = rhs.trim();
    if text == "true" || text == "false" {
        return ("bool", text.to_string());
    }
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        return ("str", text[1..text.len() - 1].to_string());
    }
    let numeric = text.strip_suffix('f').unwrap_or(text);
    if numeric.contains('.') || numeric.contains('e') || numeric.contains('E') {
        if numeric.parse::<f64>().is_ok() {
            return ("float", text.to_string());
        }
    } else if numeric.parse::<i64>().is_ok() {
        return ("int", text.to_string());
    }
    ("unsupported", text.to_string())
}

fn tag_map_entry(call: &str) -> Option<(String, String, String)> {
    let (field, rest) = call.split_once(".Add(")?;
    if !field.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return None;
    }
    let rest = rest.strip_suffix(')').unwrap_or(rest);
    let (tag_expr, literal) = rest.split_once(',')?;
    let tag = tag_expr.trim().strip_prefix("GameplayTag::")?;
    if !tag.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return None;
    }
    Some((field.to_string(), tag.to_string(), literal.trim().to_string()))
}

fn clear_owned_mini_caches(out_dir: &Path) -> Result<()> {
    let entries = std::fs::read_dir(out_dir)
        .with_context(|| format!("reading {}", out_dir.display()))?;
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading {}", out_dir.display()))?
            .path();
        let owned = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".mini.cache"));
        if owned && path.is_file() {
            std::fs::remove_file(&path)
                .with_context(|| format!("removing leftover mini {}", path.display()))?;
        }
    }
    Ok(())
}

fn class_span(source: &str, class_name: &str) -> Result<(usize, usize)> {
    let marker = format!("class {class_name}");
    let mut search_from = 0;
    let start = loop {
        let Some(relative) = source[search_from..].find(&marker) else {
            bail!("emitted source has no exact declaration `{marker}`");
        };
        let start = search_from + relative;
        let after = start + marker.len();
        let boundary = source[after..]
            .chars()
            .next()
            .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        if boundary {
            break start;
        }
        search_from = after;
    };
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    let mut opened = false;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                depth += 1;
                opened = true;
            }
            b'}' => {
                depth -= 1;
                if opened && depth == 0 {
                    return Ok((start, index + 1));
                }
            }
            _ => {}
        }
        index += 1;
    }
    bail!("class {class_name} body is not closed")
}

fn with_prepared<T>(
    cache: &Path,
    body: impl FnOnce(&[model::Module], &PreparedEmit<'_>, &[u8], Option<String>) -> Result<T>,
) -> Result<T> {
    let bytes = read_module_cache(cache)?;
    let mut resolver = RefResolver::build(&bytes).context("building the reference resolver")?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let loaded = load_native_api_with_proof(cache);
    let binds_sha256 = loaded.as_ref().map(|loaded| {
        loaded
            .sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    });
    let prepared = PreparedEmit::new(&modules, &mut resolver, loaded.map(|loaded| loaded.native))
        .context("preparing emitted modules")?
        .with_class_defaults(true);
    let _ = faithfulness::cache_seal(&bytes);
    body(&modules, &prepared, &bytes, binds_sha256)
}

fn module_of_class(modules: &[model::Module], class_name: &str) -> Option<(usize, String)> {
    modules.iter().enumerate().find_map(|(index, module)| {
        module
            .classes
            .iter()
            .any(|class| class.name == class_name)
            .then(|| (index, module.name.clone()))
    })
}

fn resolve_cache(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(cache) = cache {
        return Ok(cache);
    }
    let game = gore_loc::config::game_root(game).context("resolving game path")?;
    let source = gore_mod::pristine_script_cache_source(&game)?;
    Ok(source.path)
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"
class UItFo_Apple : UItemDefinition {
    default m_Value = 4;
    default m_Weight = 1.0f;
    default m_Name = "Apple";
}
class UItMw_1H_Sword_Old_01 : USword1H {
    default m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Edge, 10.0f);
    default m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Blunt, 2.0f);
}
"#;

    fn edit(class: &str, field: &str, tag: Option<&str>, value: ValueLiteral) -> ValueEdit {
        ValueEdit {
            class: class.into(),
            field: field.into(),
            tag: tag.map(str::to_string),
            value,
        }
    }

    #[test]
    fn assignment_and_one_tag_change_and_the_rest_stays() {
        let apple = edit("UItFo_Apple", "m_Value", None, ValueLiteral::Int(500));
        let sword = edit(
            "UItMw_1H_Sword_Old_01",
            "m_DamageBase",
            Some("Item_Damage_Physical_Edge"),
            ValueLiteral::Float(15.0),
        );
        let edited = apply_edits(SOURCE, &[&apple, &sword]).unwrap();
        assert!(edited.contains("default m_Value = 500;"));
        assert!(edited.contains(
            "default m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Edge, 15.0f);"
        ));
        assert!(edited.contains("default m_Weight = 1.0f;"));
        assert!(edited.contains(
            "default m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Blunt, 2.0f);"
        ));
        prove_only_requested_statements_changed(SOURCE, &edited, &[&apple, &sword]).unwrap();
    }

    #[test]
    fn prefix_class_name_does_not_rewrite_the_longer_class() {
        let source = r#"
class UFooBar : UItem {
    default m_Value = 1;
}
class UFoo : UItem {
    default m_Value = 2;
}
"#;
        let shorter = edit("UFoo", "m_Value", None, ValueLiteral::Int(9));
        let edited = apply_edits(source, &[&shorter]).unwrap();
        assert!(edited.contains("class UFooBar : UItem {\n    default m_Value = 1;"));
        assert!(edited.contains("class UFoo : UItem {\n    default m_Value = 9;"));
        let longer = edit("UFooBar", "m_Value", None, ValueLiteral::Int(7));
        let edited = apply_edits(source, &[&longer]).unwrap();
        assert!(edited.contains("class UFooBar : UItem {\n    default m_Value = 7;"));
        assert!(edited.contains("class UFoo : UItem {\n    default m_Value = 2;"));
    }

    #[test]
    fn rebuild_clears_a_leftover_mini_cache() {
        let dir = std::env::temp_dir().join(format!(
            "gore-value-minis-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let leftover = dir.join("Module.mini.cache");
        std::fs::write(&leftover, b"old").unwrap();
        std::fs::write(dir.join("keep.txt"), b"keep").unwrap();
        clear_owned_mini_caches(&dir).unwrap();
        assert!(!leftover.exists());
        assert_eq!(std::fs::read(dir.join("keep.txt")).unwrap(), b"keep");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn wrong_type_and_missing_tag_fail_before_any_compile() {
        let wrong = edit("UItFo_Apple", "m_Value", None, ValueLiteral::Float(1.0));
        assert!(apply_edits(SOURCE, &[&wrong]).unwrap_err().to_string().contains("int"));
        let missing = edit(
            "UItMw_1H_Sword_Old_01",
            "m_DamageBase",
            None,
            ValueLiteral::Float(1.0),
        );
        assert!(apply_edits(SOURCE, &[&missing])
            .unwrap_err()
            .to_string()
            .contains("tag"));
        let unknown = edit("UItFo_Apple", "m_NotAField", None, ValueLiteral::Int(1));
        assert!(apply_edits(SOURCE, &[&unknown])
            .unwrap_err()
            .to_string()
            .contains("unsupported"));
    }
}
