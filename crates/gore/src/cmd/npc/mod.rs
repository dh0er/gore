//! `gore npc` — die Figuren des Spiels lesen und Änderungen an ihnen verfassen.
//!
//! Diese Stufe liest nur. Eine Figur ist im Spiel kein Datensatz, sondern eine Kette von
//! AngelScript-Klassen; alles hier löst diese Kette über den emittierten Quelltext auf, nicht
//! über Bytecode, damit die Kernfunktionen rein und ohne Spielinstallation prüfbar bleiben.

pub mod defaults;
pub mod render;

use anyhow::Result;
use clap::Subcommand;

use super::find::{bundled_catalog, CatalogEntry};

#[derive(Subcommand)]
pub enum NpcAction {
    /// List the characters the game ships
    List {
        /// Keep only entries whose id or class contains this text
        filter: Option<String>,
        /// Keep only one category (human, creature, other)
        #[arg(long)]
        category: Option<String>,
        /// Max rows to print
        #[arg(long, default_value_t = 50)]
        max: usize,
        /// Emit one JSON document instead of the human-readable lines
        #[arg(long)]
        json: bool,
    },
}

pub fn run(action: NpcAction) -> Result<()> {
    match action {
        NpcAction::List {
            filter,
            category,
            max,
            json,
        } => list(filter.as_deref(), category.as_deref(), max, json),
    }
}

/// Every bundled NPC row, narrowed by `filter` and `category`.
pub fn select<'a>(
    entries: &'a [CatalogEntry],
    filter: Option<&str>,
    category: Option<&str>,
) -> Vec<&'a CatalogEntry> {
    let filter = filter.map(str::to_lowercase);
    let category = category.map(str::to_lowercase);
    entries
        .iter()
        .filter(|entry| entry.domain == "npc")
        .filter(|entry| match &category {
            Some(wanted) => entry.category.to_lowercase() == *wanted,
            None => true,
        })
        .filter(|entry| match &filter {
            Some(needle) => {
                entry.id.to_lowercase().contains(needle)
                    || entry
                        .class
                        .as_deref()
                        .is_some_and(|class| class.to_lowercase().contains(needle))
            }
            None => true,
        })
        .collect()
}

fn list(filter: Option<&str>, category: Option<&str>, max: usize, json: bool) -> Result<()> {
    let entries = bundled_catalog()?;
    let hits = select(&entries, filter, category);
    if json {
        let rows: Vec<serde_json::Value> = hits
            .iter()
            .take(max)
            .map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "category": entry.category,
                    "class": entry.class,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "matched": hits.len(),
                "listed": rows.len(),
                "npcs": rows,
            }))?
        );
        return Ok(());
    }
    for entry in hits.iter().take(max) {
        println!("{}", render::list_line(entry));
    }
    println!("{} of {} shown", hits.len().min(max), hits.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(domain: &'static str, id: &str, category: &str, class: Option<&str>) -> CatalogEntry {
        CatalogEntry {
            domain,
            id: id.to_string(),
            category: category.to_string(),
            class: class.map(str::to_string),
            module: None,
            loc_key: None,
            caption: None,
        }
    }

    /// Vier Zeilen, die jede Auswahlregel einmal treffen und einmal verfehlen.
    fn catalog() -> Vec<CatalogEntry> {
        vec![
            entry(
                "npc",
                "OC_STT_Diego",
                "human",
                Some("CharacterDefinition_Human_OC_STT_Diego"),
            ),
            entry(
                "npc",
                "MST_Molerat",
                "creature",
                Some("CharacterDefinition_Creature_MST_Molerat"),
            ),
            entry("npc", "NoClassAtAll", "other", None),
            // Der Katalog trägt Gegenstände und Wissen in derselben Liste; eine Zeile aus einer
            // anderen Domäne darf hier nie herausfallen, auch wenn ihre Id passt.
            entry(
                "item",
                "ItMi_Diego_Key",
                "misc",
                Some("Item_ItMi_Diego_Key"),
            ),
        ]
    }

    #[test]
    fn no_filter_returns_every_npc_row() {
        let entries = catalog();
        let hits = select(&entries, None, None);
        assert_eq!(hits.len(), 3);
        assert!(hits.iter().all(|entry| entry.domain == "npc"));
    }

    #[test]
    fn a_filter_matches_a_substring_of_the_id_case_insensitively() {
        let entries = catalog();
        let hits = select(&entries, Some("diego"), None);
        assert_eq!(
            hits.iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["OC_STT_Diego"]
        );
    }

    #[test]
    fn a_filter_also_matches_the_class_when_the_id_does_not() {
        let entries = catalog();
        let hits = select(&entries, Some("characterdefinition_creature"), None);
        assert_eq!(
            hits.iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["MST_Molerat"]
        );
    }

    #[test]
    fn a_row_without_a_class_is_kept_by_an_id_match_and_dropped_otherwise() {
        let entries = catalog();
        assert_eq!(select(&entries, Some("noclass"), None).len(), 1);
        assert!(select(&entries, Some("characterdefinition"), None)
            .iter()
            .all(|entry| entry.class.is_some()));
    }

    #[test]
    fn a_category_keeps_only_that_category() {
        let entries = catalog();
        let hits = select(&entries, None, Some("CREATURE"));
        assert_eq!(
            hits.iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["MST_Molerat"]
        );
    }

    #[test]
    fn a_category_and_a_filter_both_have_to_match() {
        let entries = catalog();
        assert!(select(&entries, Some("diego"), Some("creature")).is_empty());
        assert_eq!(select(&entries, Some("diego"), Some("human")).len(), 1);
    }

    #[test]
    fn a_row_from_another_domain_is_never_returned() {
        let entries = catalog();
        // "diego" steht in der Gegenstandszeile genauso wie in der Figurenzeile; nur die Figur
        // darf zurückkommen, sonst beantwortet `npc list` Fragen über Gegenstände.
        for hits in [
            select(&entries, Some("diego"), None),
            select(&entries, Some("itmi"), None),
            select(&entries, None, Some("misc")),
            select(&entries, None, None),
        ] {
            assert!(hits.iter().all(|entry| entry.domain == "npc"), "{hits:?}");
        }
    }
}
