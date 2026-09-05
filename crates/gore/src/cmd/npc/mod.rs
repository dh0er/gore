//! `gore npc` — die Figuren des Spiels lesen und Änderungen an ihnen verfassen.
//!
//! Diese Stufe liest nur. Eine Figur ist im Spiel kein Datensatz, sondern eine Kette von
//! AngelScript-Klassen; alles hier löst diese Kette über den emittierten Quelltext auf, nicht
//! über Bytecode, damit die Kernfunktionen rein und ohne Spielinstallation prüfbar bleiben.

pub mod chain;
pub mod defaults;
pub mod render;
pub mod sites;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gore_as::cache::{emit_all::PreparedEmit, faithfulness, model, refs::RefResolver};

use super::as_cache::{load_native_api_with_proof, read_module_cache};
use super::find::{bundled_catalog, CatalogEntry};
use sites::Site;

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
    /// Print one character in full: its class chain, where it spawns, and what it inherits
    Show {
        /// Exact NPC id, for example OC_STT_Diego
        npc: String,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
        /// Emit one JSON document instead of the human-readable block
        #[arg(long)]
        json: bool,
    },
    /// List the world points the level scripts spawn characters from
    Sites {
        /// Keep only sites whose level-script module contains this text
        #[arg(long)]
        level: Option<String>,
        /// Keep only sites that spawn this character
        #[arg(long)]
        npc: Option<String>,
        /// Max rows to print
        #[arg(long, default_value_t = 50)]
        max: usize,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
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
        NpcAction::Show {
            npc,
            cache,
            game,
            json,
        } => show(&npc, cache, game, json),
        NpcAction::Sites {
            level,
            npc,
            max,
            cache,
            game,
            json,
        } => list_sites(level.as_deref(), npc.as_deref(), max, cache, game, json),
    }
}

// ─── Reading the cache ───────────────────────────────────────────

/// Der emittierte Baum, einmal aufgebaut: jede Klasse nach Namen, jede Spawn-Stelle, plus die
/// Siegel, mit denen die Übersetzungstreue nachgeschlagen wird.
pub struct Emitted {
    pub classes: BTreeMap<String, defaults::EmittedClass>,
    pub sites: Vec<Site>,
    pub cache_seal: [u8; 32],
    pub binds_seal: Option<[u8; 32]>,
}

/// The script cache to read: the one named, else the one in the resolved install.
fn cache_path(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(cache) = cache {
        return Ok(cache);
    }
    let root = gore_loc::config::game_root(game).context("resolving the game path")?;
    let paths = gore_mod::resolve_game_paths(&root);
    if !paths.script_cache.is_file() {
        bail!(
            "no script cache at {}. Pass --cache to read one directly",
            paths.script_cache.display()
        );
    }
    Ok(paths.script_cache)
}

/// Emit every module once and index what the NPC commands need out of it.
fn emit_index(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<Emitted> {
    let path = cache_path(cache, game)?;
    let bytes = read_module_cache(&path)?;
    let mut resolver = RefResolver::build(&bytes).context("building the reference resolver")?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let loaded = load_native_api_with_proof(&path);
    let binds_seal = loaded.as_ref().map(|loaded| loaded.sha256);
    let prepared = PreparedEmit::new(&modules, &mut resolver, loaded.map(|l| l.native))
        .context("preparing the emitted modules")?
        .with_class_defaults(true);

    let mut classes = BTreeMap::new();
    let mut found = Vec::new();
    for (index, module) in modules.iter().enumerate() {
        let source = prepared
            .emit_module(index)
            .with_context(|| format!("emitting {}", module.name))?;
        if module.name.starts_with("LevelScripts.") {
            found.extend(sites::parse_sites(&module.name, &source));
        }
        for class in defaults::parse_classes(&source) {
            classes.insert(class.name.clone(), class);
        }
    }
    Ok(Emitted {
        classes,
        sites: found,
        cache_seal: faithfulness::cache_seal(&bytes),
        binds_seal,
    })
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

/// Every site that spawns `spawn_class`.
fn sites_for<'a>(emitted: &'a Emitted, spawn_class: &str) -> Vec<&'a Site> {
    emitted
        .sites
        .iter()
        .filter(|site| site.spawn_definition == spawn_class)
        .collect()
}

fn show(npc: &str, cache: Option<PathBuf>, game: Option<PathBuf>, json: bool) -> Result<()> {
    let emitted = emit_index(cache, game)?;
    let spawn_class = format!("USpawnAIAgentDefinition_{npc}");
    if !emitted.classes.contains_key(&spawn_class) {
        bail!(
            "no character {npc} in this cache — {spawn_class} is not declared. \
             `gore npc list {npc}` shows the ids that exist"
        );
    }
    let resolved = chain::resolve(&emitted.classes, &spawn_class);
    let mine = sites_for(&emitted, &spawn_class);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": npc,
                "chain": {
                    "spawn_definition": resolved.spawn_definition,
                    "ai_agent_config": resolved.ai_agent_config,
                    "character_definition": resolved.character_definition,
                    "guild_base": resolved.guild_base,
                    "unique_name": resolved.unique_name,
                },
                "sites": mine.iter().map(|site| serde_json::json!({
                    "world_point": site.world_point,
                    "module": site.module,
                    "translation": render::translation_json(&emitted, &site.module),
                })).collect::<Vec<_>>(),
            }))?
        );
        return Ok(());
    }
    print!("{}", render::chain_block(npc, &resolved));
    println!("spawns at {} site(s):", mine.len());
    for site in &mine {
        println!("  {}  in {}", site.world_point, site.module);
        println!("    {}", render::translation_line(&emitted, &site.module));
    }
    Ok(())
}

fn list_sites(
    level: Option<&str>,
    npc: Option<&str>,
    max: usize,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let emitted = emit_index(cache, game)?;
    let wanted_spawn = npc.map(|npc| format!("USpawnAIAgentDefinition_{npc}"));
    let hits: Vec<&Site> = emitted
        .sites
        .iter()
        .filter(|site| level.is_none_or(|needle| site.module.contains(needle)))
        .filter(|site| {
            wanted_spawn
                .as_deref()
                .is_none_or(|wanted| site.spawn_definition == wanted)
        })
        .collect();
    if json {
        let rows: Vec<serde_json::Value> = hits
            .iter()
            .take(max)
            .map(|site| {
                serde_json::json!({
                    "world_point": site.world_point,
                    "module": site.module,
                    "spawn_definition": site.spawn_definition,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "matched": hits.len(),
                "listed": rows.len(),
                "sites": rows,
            }))?
        );
        return Ok(());
    }
    for site in hits.iter().take(max) {
        println!(
            "{}  {}  {}",
            site.world_point, site.spawn_definition, site.module
        );
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

    fn site(world_point: &str, module: &str, spawn_definition: &str) -> Site {
        Site {
            world_point: world_point.to_string(),
            module: module.to_string(),
            spawn_definition: spawn_definition.to_string(),
        }
    }

    fn emitted(sites: Vec<Site>) -> Emitted {
        Emitted {
            classes: BTreeMap::new(),
            sites,
            cache_seal: [0u8; 32],
            binds_seal: None,
        }
    }

    #[test]
    fn sites_for_returns_every_world_point_that_sets_the_same_character() {
        // Dieselbe Figur kann an mehreren Weltpunkten stehen; wer nur den ersten meldet,
        // schickt einen Mod an eine Stelle und verschweigt die zweite.
        let all = emitted(vec![
            site(
                "UWP_EZ_DIEGO_A",
                "LevelScripts.Map_x2_y2_ExchangeZone_AI_script",
                "USpawnAIAgentDefinition_OC_STT_Diego",
            ),
            site(
                "UWP_EZ_OTHER",
                "LevelScripts.Map_x2_y2_ExchangeZone_AI_script",
                "USpawnAIAgentDefinition_OC_STT_Gomez",
            ),
            site(
                "UWP_EZ_DIEGO_B",
                "LevelScripts.Map_x2_y2_ExchangeZone_AI_script",
                "USpawnAIAgentDefinition_OC_STT_Diego",
            ),
        ]);
        let mine = sites_for(&all, "USpawnAIAgentDefinition_OC_STT_Diego");
        assert_eq!(
            mine.iter()
                .map(|site| site.world_point.as_str())
                .collect::<Vec<_>>(),
            vec!["UWP_EZ_DIEGO_A", "UWP_EZ_DIEGO_B"]
        );
    }

    #[test]
    fn sites_for_a_character_nothing_spawns_is_empty_rather_than_everything() {
        let all = emitted(vec![site(
            "UWP_EZ_DIEGO_A",
            "LevelScripts.Demo",
            "USpawnAIAgentDefinition_OC_STT_Diego",
        )]);
        assert!(sites_for(&all, "USpawnAIAgentDefinition_Nobody").is_empty());
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
