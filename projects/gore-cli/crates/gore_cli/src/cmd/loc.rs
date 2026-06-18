//! `gore-cli loc` — merge the per-language `gore_loc_<lang>.json` dumps (produced
//! by the gore-dump UE4SS mod, one pass per options-menu language) into a single
//! `loc_catalog.json` keyed by class id, with every language folded under each
//! localized field.
//!
//! Per-language input shape (one file per language):
//!   `{ "language": "de", "entries": { "<id>": { "kind": "item",
//!      "text": { "m_Name": "Käse", "m_Description": "..." } } } }`
//!
//! Merged output:
//!   `{ "languages": ["de","en",...], "entries": { "<id>": { "kind": "item",
//!      "text": { "m_Name": { "de": "Käse", "en": "Cheese" }, ... } } } }`
//!
//! The language tag comes from each file's `language` field (authoritative), not
//! the filename. Both gore-save and gore-mod consume the merged catalog.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct LangFile {
    language: String,
    entries: BTreeMap<String, LangEntry>,
}

#[derive(Deserialize)]
struct LangEntry {
    kind: String,
    /// field name -> localized string in this file's language
    text: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct MergedEntry {
    kind: String,
    /// field name -> language -> localized string
    text: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Serialize)]
struct Merged {
    languages: Vec<String>,
    entries: BTreeMap<String, MergedEntry>,
}

pub fn run(input: PathBuf, out: PathBuf) -> Result<()> {
    let files = collect_files(&input)?;
    if files.is_empty() {
        bail!(
            "no gore_loc_*.json files found in '{}'",
            input.display()
        );
    }

    let mut parsed = Vec::with_capacity(files.len());
    for f in &files {
        let json = fs::read_to_string(f)
            .with_context(|| format!("reading '{}'", f.display()))?;
        let lf: LangFile = serde_json::from_str(&json)
            .with_context(|| format!("parsing '{}'", f.display()))?;
        parsed.push(lf);
    }

    let merged = merge(parsed);
    let json = serde_json::to_string(&merged).context("serializing merged catalog")?;
    fs::write(&out, json).with_context(|| format!("writing '{}'", out.display()))?;
    println!(
        "Merged {} languages ({}), {} entries -> {}",
        merged.languages.len(),
        merged.languages.join(", "),
        merged.entries.len(),
        out.display()
    );
    Ok(())
}

/// Find `gore_loc_*.json` files in a directory, or treat `input` as one file.
fn collect_files(input: &Path) -> Result<Vec<PathBuf>> {
    if input.is_dir() {
        let mut v = Vec::new();
        for entry in fs::read_dir(input)
            .with_context(|| format!("reading dir '{}'", input.display()))?
        {
            let p = entry?.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("gore_loc_") && name.ends_with(".json") {
                    v.push(p);
                }
            }
        }
        v.sort();
        Ok(v)
    } else {
        Ok(vec![input.to_path_buf()])
    }
}

/// Fold per-language files into one catalog. Language order follows first
/// appearance, then sorted for stable output.
fn merge(files: Vec<LangFile>) -> Merged {
    let mut languages: Vec<String> = Vec::new();
    let mut entries: BTreeMap<String, MergedEntry> = BTreeMap::new();

    for lf in files {
        if !languages.contains(&lf.language) {
            languages.push(lf.language.clone());
        }
        for (id, entry) in lf.entries {
            let me = entries.entry(id).or_insert_with(|| MergedEntry {
                kind: entry.kind.clone(),
                text: BTreeMap::new(),
            });
            for (field, s) in entry.text {
                me.text
                    .entry(field)
                    .or_default()
                    .insert(lf.language.clone(), s);
            }
        }
    }

    languages.sort();
    Merged { languages, entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lang_file(value: serde_json::Value) -> LangFile {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn folds_languages_under_each_field() {
        let de = lang_file(json!({
            "language": "de",
            "entries": {
                "ItFo_Cheese": {"kind": "item", "text": {"m_Name": "Käse", "m_Description": "Lecker."}}
            }
        }));
        let en = lang_file(json!({
            "language": "en",
            "entries": {
                "ItFo_Cheese": {"kind": "item", "text": {"m_Name": "Cheese", "m_Description": "Tasty."}}
            }
        }));

        let m = merge(vec![de, en]);
        assert_eq!(m.languages, vec!["de", "en"]);
        let cheese = &m.entries["ItFo_Cheese"];
        assert_eq!(cheese.kind, "item");
        assert_eq!(cheese.text["m_Name"]["de"], "Käse");
        assert_eq!(cheese.text["m_Name"]["en"], "Cheese");
        assert_eq!(cheese.text["m_Description"]["en"], "Tasty.");
    }

    #[test]
    fn unions_ids_across_languages() {
        // An id present in only one language still appears, with just that language.
        let de = lang_file(json!({
            "language": "de",
            "entries": {"A": {"kind": "item", "text": {"m_Name": "Apfel"}}}
        }));
        let en = lang_file(json!({
            "language": "en",
            "entries": {"B": {"kind": "npc", "text": {"m_Name": "Diego"}}}
        }));

        let m = merge(vec![de, en]);
        assert_eq!(m.entries.len(), 2);
        assert_eq!(m.entries["A"].text["m_Name"].get("de").map(String::as_str), Some("Apfel"));
        assert!(m.entries["A"].text["m_Name"].get("en").is_none());
        assert_eq!(m.entries["B"].kind, "npc");
    }

    #[test]
    fn languages_are_sorted_and_deduped() {
        let a = lang_file(json!({"language": "ru", "entries": {}}));
        let b = lang_file(json!({"language": "de", "entries": {}}));
        let c = lang_file(json!({"language": "de", "entries": {}}));
        let m = merge(vec![a, b, c]);
        assert_eq!(m.languages, vec!["de", "ru"]);
    }
}
