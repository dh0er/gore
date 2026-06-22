use anyhow::{bail, Context, Result};
use gore_modgen::{
    gen::{gen_lua, OverridesConfig},
    validate::validate_config,
};
use gore_reflect::model::ReflectionModel;
use std::{fs, path::PathBuf};

use crate::cmd::validate_mod_name;

pub fn run(overrides_path: PathBuf, mods_dir: PathBuf, model_path: Option<PathBuf>) -> Result<()> {
    // 1. Parse overrides.toml
    let toml_str = fs::read_to_string(&overrides_path)
        .with_context(|| format!("reading overrides '{}'", overrides_path.display()))?;
    let cfg: OverridesConfig = toml::from_str(&toml_str)
        .with_context(|| "parsing overrides.toml")?;

    // 1b. Validate mod name is safe (no path traversal)
    validate_mod_name(&cfg.meta.name)
        .with_context(|| "invalid mod name in overrides.toml")?;

    // 2. Optionally validate against reflection model
    if let Some(model_path) = model_path {
        let json = fs::read_to_string(&model_path)
            .with_context(|| format!("reading model.json '{}'", model_path.display()))?;
        let model: ReflectionModel = serde_json::from_str(&json)
            .with_context(|| "parsing model.json")?;

        let errors = validate_config(&cfg, &model);
        if !errors.is_empty() {
            let msg = errors
                .iter()
                .map(|e| format!("  - {e}"))
                .collect::<Vec<_>>()
                .join("\n");
            bail!("validation errors in overrides.toml:\n{msg}");
        }
    }

    // 3. Generate Lua
    let lua = gen_lua(&cfg);

    // 4. Write mod structure
    let mod_dir = mods_dir.join(&cfg.meta.name);
    let scripts_dir = mod_dir.join("Scripts");
    fs::create_dir_all(&scripts_dir)
        .with_context(|| format!("creating mod dir '{}'", scripts_dir.display()))?;

    fs::write(mod_dir.join("enabled.txt"), "")
        .context("writing enabled.txt")?;
    fs::write(scripts_dir.join("main.lua"), &lua)
        .context("writing main.lua")?;

    println!("Generated mod '{}' -> {}", cfg.meta.name, mod_dir.display());
    Ok(())
}
