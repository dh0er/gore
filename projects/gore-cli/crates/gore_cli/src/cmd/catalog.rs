//! `gore-cli catalog` — emit a gore-cli catalog JSON from a model.json.
//!
//! NOTE: the output format is the gore-cli catalog shape
//! (`[{id, display_name, category}]`) which differs from the Python pipeline's
//! bundled `item_catalog.json` (`[{id, path, category}]`). This divergence is
//! intentional for the modding toolchain; do not use this output as a drop-in
//! replacement for the Python-generated allow-list.

use anyhow::{Context, Result};
use gore_core::{
    catalog::{category_for_id, CatalogEntry, CatalogModel, ItemCategory},
    model::ReflectionModel,
};
use std::{fs, path::PathBuf};

/// Prefixes that identify concrete item classes (not base definitions or CDOs).
const ITEM_PREFIXES: &[&str] = &[
    "ItFo_", "ItMw_", "ItRw_", "ItAm_", "ItAt_",
    "ItAr_", "ItWr_", "ItPo_", "ItLs_", "ItMi_",
];

fn is_item_class(name: &str) -> bool {
    ITEM_PREFIXES.iter().any(|p| name.starts_with(p))
}

pub fn run(input: PathBuf, out: PathBuf) -> Result<()> {
    let json = fs::read_to_string(&input)
        .with_context(|| format!("reading model.json '{}'", input.display()))?;
    let model: ReflectionModel = serde_json::from_str(&json)
        .with_context(|| "parsing model.json")?;

    let mut catalog = CatalogModel::default();
    for cls in &model.classes {
        if is_item_class(&cls.name) {
            catalog.entries.push(CatalogEntry {
                id: cls.name.clone(),
                display_name: cls.name.clone(), // display name = id until object dump provides names
                category: category_for_id(&cls.name),
            });
        }
    }

    // Filter Unknown to avoid noise from unprefixed classes that slipped through
    catalog.entries.retain(|e| e.category != ItemCategory::Unknown);

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_out = serde_json::to_string_pretty(&catalog.entries)?;
    fs::write(&out, json_out)
        .with_context(|| format!("writing catalog to '{}'", out.display()))?;

    println!("Wrote {} catalog entries -> {}", catalog.entries.len(), out.display());
    Ok(())
}
