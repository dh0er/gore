//! `gore-cli catalog` — generate a catalog JSON from a UE4SS object dump.
//!
//! Usage:
//!   gore-cli catalog --kind item    <dump.txt> -o item_catalog.json
//!   gore-cli catalog --kind npc     <dump.txt> -o npc_catalog.json
//!   gore-cli catalog --kind knowledge <dump.txt> -o knowledge_catalog.json
//!
//! Output is byte-identical to the Python pipeline scripts.

use anyhow::{Context, Result};
use clap::ValueEnum;
use gore_core::catalog::pipeline;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, ValueEnum)]
pub enum CatalogKind {
    Item,
    Npc,
    Knowledge,
}

pub fn run(kind: CatalogKind, dump: PathBuf, out: PathBuf) -> Result<()> {
    // UE4SS object dumps can contain non-UTF-8 bytes in unrelated object names;
    // decode lossily (like the original Python builders) so one bad byte doesn't
    // abort catalog generation. The `ASClass` lines we scan are ASCII.
    let bytes = fs::read(&dump)
        .with_context(|| format!("reading dump '{}'", dump.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();

    let json = match kind {
        CatalogKind::Item => {
            let (entries, skipped) = pipeline::build_item_catalog(&lines);
            if !skipped.is_empty() {
                eprintln!("skipped {} classes:", skipped.len());
                for s in &skipped {
                    eprintln!("  - {s}");
                }
            }
            let count = entries.len();
            let j = pipeline::to_catalog_json(&entries)
                .context("serialising item catalog")?;
            eprintln!("wrote {count} items to {}", out.display());
            j
        }
        CatalogKind::Npc => {
            let (entries, skipped) = pipeline::build_npc_catalog(&lines);
            if !skipped.is_empty() {
                eprintln!("skipped {} classes:", skipped.len());
                for s in &skipped {
                    eprintln!("  - {s}");
                }
            }
            let count = entries.len();
            let j = pipeline::to_catalog_json(&entries)
                .context("serialising npc catalog")?;
            eprintln!("wrote {count} npcs to {}", out.display());
            j
        }
        CatalogKind::Knowledge => {
            let entries = pipeline::build_knowledge_catalog(&lines);
            let count = entries.len();
            let j = pipeline::to_catalog_json(&entries)
                .context("serialising knowledge catalog")?;
            eprintln!("wrote {count} knowledge tokens to {}", out.display());
            j
        }
    };

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&out, json.as_bytes())
        .with_context(|| format!("writing catalog to '{}'", out.display()))?;
    Ok(())
}
