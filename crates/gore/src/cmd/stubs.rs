use anyhow::{Context, Result};
use gore_reflect::model::{PropType, ReflectionModel};
use std::{fs, path::PathBuf};

pub fn run(model_path: PathBuf, out_dir: PathBuf, filter: Option<String>) -> Result<()> {
    let json = fs::read_to_string(&model_path)
        .with_context(|| format!("reading model.json '{}'", model_path.display()))?;
    let model: ReflectionModel =
        serde_json::from_str(&json).with_context(|| "parsing model.json")?;

    fs::create_dir_all(&out_dir)?;

    let mut count = 0;
    for cls in &model.classes {
        if let Some(prefix) = &filter {
            if !cls.name.starts_with(prefix.as_str()) {
                continue;
            }
        }
        // The class name becomes a filename; a corrupt/external model.json could
        // carry path separators or `..` and escape out_dir. Skip unsafe names.
        if !is_safe_filename(&cls.name) {
            eprintln!("skipping class with unsafe name: {:?}", cls.name);
            continue;
        }

        let mut lua = String::new();

        // @class annotation with optional parent
        if let Some(parent) = &cls.parent {
            lua.push_str(&format!("---@class {} : {}\n", cls.name, parent));
        } else {
            lua.push_str(&format!("---@class {}\n", cls.name));
        }

        // @field annotations
        for prop in &cls.properties {
            let lua_type = prop_type_to_luals(&prop.prop_type);
            lua.push_str(&format!("---@field {} {}\n", prop.name, lua_type));
        }

        // local declaration stub
        lua.push_str(&format!("local {} = {{}}\n", cls.name));

        let file_path = out_dir.join(format!("{}.lua", cls.name));
        fs::write(&file_path, lua)
            .with_context(|| format!("writing stub '{}'", file_path.display()))?;
        count += 1;
    }

    println!("Wrote {count} Lua stubs -> {}", out_dir.display());
    Ok(())
}

/// A class name is a safe stub filename only if it is a single path component
/// with no separators and is not a `.`/`..` traversal token.
fn is_safe_filename(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && std::path::Path::new(name).components().count() == 1
}

fn prop_type_to_luals(t: &PropType) -> &'static str {
    match t {
        PropType::Int => "integer",
        PropType::Float => "number",
        PropType::Bool => "boolean",
        PropType::String => "string",
        PropType::Enum(_) => "integer",
        PropType::Opaque(_) => "any",
    }
}

#[cfg(test)]
mod stubs_tests {
    use super::is_safe_filename;

    #[test]
    fn safe_and_unsafe_class_filenames() {
        assert!(is_safe_filename("UItMi_Orenugget"));
        assert!(is_safe_filename("UItemDefinition"));
        assert!(!is_safe_filename("../evil"));
        assert!(!is_safe_filename("a/b"));
        assert!(!is_safe_filename(r"a\b"));
        assert!(!is_safe_filename(".."));
        assert!(!is_safe_filename(""));
    }
}
