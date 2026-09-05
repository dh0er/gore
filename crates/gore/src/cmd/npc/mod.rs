//! `gore npc` — die Figuren des Spiels lesen und Änderungen an ihnen verfassen.
//!
//! Diese Stufe liest nur. Eine Figur ist im Spiel kein Datensatz, sondern eine Kette von
//! AngelScript-Klassen; alles hier löst diese Kette über den emittierten Quelltext auf, nicht
//! über Bytecode, damit die Kernfunktionen rein und ohne Spielinstallation prüfbar bleiben.

pub mod chain;
pub mod check;
pub mod defaults;
pub mod edit;
pub mod generate;
pub mod render;
pub mod sites;
pub mod stage;
pub mod workspace;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
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
    /// Author a new character derived from a shipped one
    New {
        /// Id of the new character, for example MY_NPC
        id: String,
        /// The shipped character to derive from: its looks, stats and voice
        #[arg(long)]
        from: String,
        /// Replace the faction with this guild base, for example OldCamp_Guard
        #[arg(long)]
        guild: Option<String>,
        /// World point to spawn at, from `gore npc sites`
        #[arg(long)]
        at: String,
        /// Waypoint for the daily routine
        #[arg(long)]
        waypoint: Option<String>,
        /// Add an empty trader configuration
        #[arg(long)]
        trader: bool,
        /// Build the looks from parts at runtime instead of borrowing a prebaked model. No shipped
        /// character does this; unproven
        #[arg(long)]
        modular_visuals: bool,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
        /// Output workspace directory; must not exist
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Stop a shipped character from being placed in the world
    Delete {
        /// The character to remove
        npc: String,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
        /// Output workspace directory; must not exist
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Check an authored workspace against the current compile contract
    Check {
        /// The workspace directory written by `new` or `delete`
        dir: PathBuf,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Build the source tree and print the commands that compile an authored character
    Stage {
        /// The workspace directory written by `new` or `delete`
        dir: PathBuf,
        /// Where to keep the emitted source tree between runs. Required for a new character
        #[arg(long)]
        tree: Option<PathBuf>,
        /// Name of the mod being built
        #[arg(long, default_value = "MyNpcMod")]
        mod_name: String,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Write a character's display name as a `gore loc import --edits` document
    Text {
        /// The character id, for example MY_NPC
        id: String,
        /// The name to show above the character's dialog lines
        #[arg(long)]
        name: String,
        /// Also set the English columns to this name
        #[arg(long)]
        english: Option<String>,
        /// Output file; must not exist
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Take a shipped character's own module out for editing
    Checkout {
        /// The character to edit, for example OC_STT_Diego
        npc: String,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
        /// Output workspace directory; must not exist
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Clone a shipped character. Same result as `new`, named for what it does
    Clone {
        /// The shipped character to copy
        source: String,
        /// Id of the new character, for example MY_NPC
        #[arg(long)]
        id: String,
        /// Replace the faction with this guild base, for example OldCamp_Guard
        #[arg(long)]
        guild: Option<String>,
        /// World point to spawn at, from `gore npc sites`
        #[arg(long)]
        at: String,
        /// Waypoint for the daily routine
        #[arg(long)]
        waypoint: Option<String>,
        /// Add an empty trader configuration
        #[arg(long)]
        trader: bool,
        /// Read this script cache instead of the installed one
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
        /// Output workspace directory; must not exist
        #[arg(short, long)]
        out: PathBuf,
    },
    /// List the world points the level scripts can place characters at
    Sites {
        /// Keep only sites whose level-script module contains this text
        #[arg(long)]
        level: Option<String>,
        /// Keep only world points nobody is spawned at
        #[arg(long, conflicts_with = "occupied")]
        free: bool,
        /// Keep only world points that already spawn somebody
        #[arg(long)]
        occupied: bool,
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
        NpcAction::New {
            id,
            from,
            guild,
            at,
            waypoint,
            trader,
            modular_visuals,
            cache,
            game,
            out,
        } => author(
            &NewRequest {
                id,
                from,
                guild,
                at,
                waypoint,
                trader,
                modular_visuals,
            },
            cache,
            game,
            &out,
        ),
        NpcAction::Delete {
            npc,
            cache,
            game,
            out,
        } => suppress(&npc, cache, game, &out),
        NpcAction::Check { dir, cache, game } => check_workspace(&dir, cache, game),
        NpcAction::Stage {
            dir,
            tree,
            mod_name,
            cache,
            game,
        } => stage_workspace(&dir, tree.as_deref(), &mod_name, cache, game),
        NpcAction::Text {
            id,
            name,
            english,
            out,
        } => write_display_name(&id, &name, english.as_deref(), &out),
        NpcAction::Checkout {
            npc,
            cache,
            game,
            out,
        } => checkout(&npc, cache, game, &out),
        NpcAction::Clone {
            source,
            id,
            guild,
            at,
            waypoint,
            trader,
            cache,
            game,
            out,
        } => author(
            &NewRequest {
                id,
                from: source,
                guild,
                at,
                waypoint,
                trader,
                modular_visuals: false,
            },
            cache,
            game,
            &out,
        ),
        NpcAction::Sites {
            level,
            free,
            occupied,
            npc,
            max,
            cache,
            game,
            json,
        } => list_sites(
            &SitesFilter {
                level,
                free,
                occupied,
                npc,
            },
            max,
            cache,
            game,
            json,
        ),
    }
}

/// Was `gore npc new` verlangt. Eigener Typ, weil clap sonst über die Argumentzahl klagt.
pub struct NewRequest {
    pub id: String,
    pub from: String,
    pub guild: Option<String>,
    pub at: String,
    pub waypoint: Option<String>,
    pub trader: bool,
    pub modular_visuals: bool,
}

// ─── Reading the cache ───────────────────────────────────────────

/// Der emittierte Baum, einmal aufgebaut: jede Klasse nach Namen, jede Spawn-Stelle, plus die
/// Siegel, mit denen die Übersetzungstreue nachgeschlagen wird.
pub struct Emitted {
    pub classes: BTreeMap<String, defaults::EmittedClass>,
    pub sites: Vec<Site>,
    /// Der emittierte Quelltext je Levelskript. Sie werden für die Spawn-Stellen ohnehin
    /// emittiert; sie hier zu behalten erspart dem Verfassen einen zweiten Durchlauf.
    pub level_sources: BTreeMap<String, String>,
    /// Jeder Weltpunkt, belegt oder frei. `sites` sind nur die belegten; wer eine Figur setzt,
    /// will fast immer einen freien, und die sind mit 2729 von 3939 in der Ueberzahl.
    pub world_points: Vec<sites::WorldPoint>,
    /// Wie viele Klassen im Spiel von einer Klasse erben. Aus dem geparsten Modell, ohne Emit.
    ///
    /// Der Compiler erklärt das erzeugte `__InitDefaults` einer Klasse **ohne Unterklassen** für
    /// `final`. Von so einer Klasse abzuleiten und eigene `default`-Zeilen mitzubringen wird mit
    /// „declared as final and cannot be overridden" abgelehnt. Diese Zählung ist deshalb keine
    /// Statistik, sondern die Frage, ob eine Klasse als Elternklasse überhaupt in Frage kommt.
    pub subclass_counts: BTreeMap<String, usize>,
    pub cache_seal: [u8; 32],
    pub binds_seal: Option<[u8; 32]>,
}

impl Emitted {
    /// Die Cache-Kennung als Kleinbuchstaben-Hex, wie sie ins Manifest geht.
    pub fn cache_sha256(&self) -> String {
        self.cache_seal.iter().map(|b| format!("{b:02x}")).collect()
    }
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

/// Namensraum der Levelskripte. Nur diese 29 Module tragen Spawn-Stellen.
const LEVEL_SCRIPT_PREFIX: &str = "LevelScripts.";

/// Wie viele Glieder die Klassenkette höchstens hat: Spawn → AIAgentConfig → CharacterDefinition.
const CHAIN_HOPS: usize = 3;

/// Das Modul, das `class_name` deklariert — aus dem geparsten Modell, ohne zu emittieren.
fn module_of_class(modules: &[model::Module], class_name: &str) -> Option<usize> {
    modules
        .iter()
        .position(|module| module.classes.iter().any(|class| class.name == class_name))
}

/// Den Baum für ein Kommando aufbauen — und **nur** die Module emittieren, die es braucht.
///
/// Den ganzen Baum zu emittieren ist nie richtig: `Map.MainMap.WorldPointManagerConfig_MainMap`
/// braucht allein viele Minuten, um seine Klassen-Defaults zurückzugewinnen (ein gewöhnliches
/// Modul braucht gut zwei Sekunden, `emit-all` über alle 7317 dauert 19 Minuten). Ein Kommando,
/// das darauf wartet, antwortet nicht. Die Levelskripte allein sind in gut zwei Sekunden da, und
/// die Klassenkette findet ihre Module über das geparste Modell, ohne dafür zu emittieren.
///
/// `spawn_class` verlangt zusätzlich die Kette ab dieser Spawn-Definition; ohne sie werden nur
/// die Spawn-Stellen gesammelt.
fn emit_index(
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
    spawn_class: Option<&str>,
) -> Result<Emitted> {
    let path = cache_path(cache, game)?;
    let bytes = read_module_cache(&path)?;
    let mut resolver = RefResolver::build(&bytes).context("building the reference resolver")?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let loaded = load_native_api_with_proof(&path);
    let binds_seal = loaded.as_ref().map(|loaded| loaded.sha256);
    let prepared = PreparedEmit::new(&modules, &mut resolver, loaded.map(|l| l.native))
        .context("preparing the emitted modules")?
        .with_class_defaults(true);

    let emit = |index: usize| -> Result<String> {
        prepared
            .emit_module(index)
            .with_context(|| format!("emitting {}", modules[index].name))
    };

    let mut classes = BTreeMap::new();
    let mut found = Vec::new();
    let mut points = Vec::new();
    let mut level_sources = BTreeMap::new();
    for (index, module) in modules.iter().enumerate() {
        if !module.name.starts_with(LEVEL_SCRIPT_PREFIX) {
            continue;
        }
        let source = emit(index)?;
        found.extend(sites::parse_sites(&module.name, &source));
        points.extend(sites::parse_world_points(&module.name, &source));
        level_sources.insert(module.name.clone(), source);
    }

    // Der Kette Glied für Glied folgen: jedes Modul erst suchen, dann emittieren. Ein fehlendes
    // Glied bricht ab statt zu raten — `chain::resolve` berichtet die Lücke dann als `None`.
    let mut wanted = spawn_class.map(str::to_string);
    for _ in 0..CHAIN_HOPS {
        let Some(class_name) = wanted.take() else {
            break;
        };
        let Some(index) = module_of_class(&modules, &class_name) else {
            break;
        };
        for class in defaults::parse_classes(&emit(index)?) {
            classes.insert(class.name.clone(), class);
        }
        wanted = classes
            .get(&class_name)
            .and_then(|class| {
                chain::assigned(class, chain::SPAWN_AI_FIELD)
                    .or_else(|| chain::assigned(class, chain::AI_CHARACTER_FIELD))
            })
            .and_then(defaults::static_class_target)
            .map(str::to_string);
    }

    let mut subclass_counts: BTreeMap<String, usize> = BTreeMap::new();
    for module in &modules {
        for class in &module.classes {
            if let Some(parent) = &class.super_class {
                *subclass_counts.entry(parent.clone()).or_insert(0) += 1;
            }
        }
    }

    Ok(Emitted {
        classes,
        sites: found,
        world_points: points,
        level_sources,
        subclass_counts,
        cache_seal: faithfulness::cache_seal(&bytes),
        binds_seal,
    })
}

/// Den Quelltext eines beliebigen Moduls emittieren, an seinem Namen.
///
/// Für Stücke, die außerhalb der Klassenkette liegen — die Gesprächseinstellungen der Vorlage
/// etwa, aus denen die Stimme kommt. Emittiert genau ein Modul, nicht den Baum.
fn emit_named_module(path: &std::path::Path, module_name: &str) -> Result<Option<String>> {
    let bytes = read_module_cache(path)?;
    let mut resolver = RefResolver::build(&bytes).context("building the reference resolver")?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let Some(index) = modules.iter().position(|module| module.name == module_name) else {
        return Ok(None);
    };
    let loaded = load_native_api_with_proof(path);
    let prepared = PreparedEmit::new(&modules, &mut resolver, loaded.map(|l| l.native))
        .context("preparing the emitted modules")?
        .with_class_defaults(true);
    Ok(Some(
        prepared
            .emit_module(index)
            .with_context(|| format!("emitting {module_name}"))?,
    ))
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
    let spawn_class = format!("USpawnAIAgentDefinition_{npc}");
    let emitted = emit_index(cache, game, Some(&spawn_class))?;
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

/// Wonach `gore npc sites` einschraenkt.
pub struct SitesFilter {
    pub level: Option<String>,
    pub free: bool,
    pub occupied: bool,
    pub npc: Option<String>,
}

fn list_sites(
    filter: &SitesFilter,
    max: usize,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let emitted = emit_index(cache, game, None)?;
    let wanted_spawn = filter.npc.as_deref().map(|npc| generate::spawn_class(npc));
    let hits: Vec<&sites::WorldPoint> = emitted
        .world_points
        .iter()
        .filter(|point| {
            filter
                .level
                .as_deref()
                .is_none_or(|needle| point.module.contains(needle))
        })
        .filter(|point| !filter.free || !point.is_occupied())
        .filter(|point| !filter.occupied || point.is_occupied())
        .filter(|point| {
            wanted_spawn
                .as_deref()
                .is_none_or(|wanted| point.occupants.iter().any(|o| o == wanted))
        })
        .collect();

    if json {
        let rows: Vec<serde_json::Value> = hits
            .iter()
            .take(max)
            .map(|point| {
                serde_json::json!({
                    "world_point": point.name,
                    "module": point.module,
                    "occupants": point.occupants,
                    "free": !point.is_occupied(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "matched": hits.len(),
                "listed": rows.len(),
                "free": hits.iter().filter(|p| !p.is_occupied()).count(),
                "sites": rows,
            }))?
        );
        return Ok(());
    }

    for point in hits.iter().take(max) {
        let who = if point.is_occupied() {
            point.occupants.join(", ")
        } else {
            "(free)".to_string()
        };
        println!("{}  {}  {}", point.name, who, point.module);
    }
    let free = hits.iter().filter(|p| !p.is_occupied()).count();
    println!(
        "{} of {} shown, {free} of them free",
        hits.len().min(max),
        hits.len()
    );
    Ok(())
}

// ─── Authoring ───────────────────────────────────────────────────

/// Wie tief `check` und die Fehlermeldungen nach Namensvorschlägen suchen.
const SUGGESTION_LIMIT: usize = 5;

/// Ein Bezeichner, den AngelScript als Klassennamensteil trägt.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Die Namen, die `needle` am ehesten gemeint haben könnte.
fn nearest<'a>(candidates: impl Iterator<Item = &'a str>, needle: &str) -> Vec<String> {
    let lower = needle.to_lowercase();
    let mut hits: Vec<String> = candidates
        .filter(|name| name.to_lowercase().contains(&lower))
        .map(str::to_string)
        .collect();
    hits.sort();
    hits.dedup();
    hits.truncate(SUGGESTION_LIMIT);
    hits
}

/// Das Verzeichnis anlegen — und sich weigern, in ein vorhandenes zu schreiben.
fn create_workspace(out: &Path) -> Result<()> {
    if out.exists() {
        bail!(
            "{} already exists. Point -o at a directory that does not exist yet, so nothing of \
             yours is overwritten",
            out.display()
        );
    }
    fs::create_dir_all(out.join("pristine"))
        .with_context(|| format!("creating {}", out.display()))?;
    Ok(())
}

/// Den Dateinamen eines Moduls im Arbeitsverzeichnis: sein letztes Pfadstück.
fn leaf_of(relative_path: &str) -> &str {
    relative_path.rsplit('/').next().unwrap_or(relative_path)
}

/// Die Stimme der Vorlage, aus ihren Gesprächseinstellungen.
///
/// Die liegen außerhalb der Klassenkette, in einem eigenen Modul neben der Figur. Findet sich
/// keines, bekommt die neue Figur keine Stimme eingetragen — das ist eine Lücke, die der Autor
/// selbst füllen kann, kein Grund abzubrechen.
fn voice_of(path: &Path, template: &str) -> Result<Option<String>> {
    let module =
        format!("AI.AIAgent.Human.Config.{template}.ConversationCharacterSettings_{template}");
    let Some(source) = emit_named_module(path, &module)? else {
        return Ok(None);
    };
    // Die Stimme steht als Aufruf da, nicht als Zuweisung:
    // `default VoiceTypeSubsets.Add(FVoiceTypeSubset(GameplayTag::VoiceType_...));`
    for class in defaults::parse_classes(&source) {
        for call in &class.calls {
            if let Some(at) = call.find("GameplayTag::") {
                let rest = &call[at + "GameplayTag::".len()..];
                let end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                if rest[..end].starts_with("VoiceType") {
                    return Ok(Some(rest[..end].to_string()));
                }
            }
        }
    }
    Ok(None)
}

/// `gore npc new` — eine Figur verfassen und das Arbeitsverzeichnis schreiben.
fn author(
    request: &NewRequest,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
    out: &Path,
) -> Result<()> {
    if !is_valid_id(&request.id) {
        bail!(
            "{:?} is not a usable character id: it becomes part of a class name, so it may hold \
             only letters, digits and underscores, and may not start with a digit",
            request.id
        );
    }

    let path = cache_path(cache.clone(), game.clone())?;
    let template_spawn = generate::spawn_class(&request.from);
    let emitted = emit_index(cache, game, Some(&template_spawn))?;

    // Erst prüfen, dann schreiben. Ein halb angelegtes Arbeitsverzeichnis wäre schlimmer als eine
    // Fehlermeldung.
    if !emitted.classes.contains_key(&template_spawn) {
        bail!(
            "no character {} in this cache — {template_spawn} is not declared. \
             `gore npc list {}` shows the ids that exist",
            request.from,
            request.from
        );
    }
    let new_spawn = generate::spawn_class(&request.id);
    if emitted.classes.contains_key(&new_spawn) {
        bail!(
            "{} is already a character in this game. Pick an id nothing ships under",
            request.id
        );
    }

    let Some(point) = emitted
        .world_points
        .iter()
        .find(|point| point.name == request.at)
    else {
        let near = nearest(
            emitted.world_points.iter().map(|point| point.name.as_str()),
            &request.at,
        );
        let hint = if near.is_empty() {
            "`gore npc sites --free` lists the empty ones".to_string()
        } else {
            format!("did you mean one of: {}", near.join(", "))
        };
        bail!("no world point {} in this game — {hint}", request.at);
    };
    let level_module = point.module.clone();
    // Zwei Koerper an einem Punkt stehen ineinander: der Fokus greift nur einen, und je nach
    // Blickwinkel verschwindet der andere. Im Spiel gesehen, nicht vermutet.
    let occupied_warning = point.is_occupied().then(|| {
        format!(
            "{} already spawns {}. Two characters at one world point stand inside each other: \
             only one can be focused and the other flickers depending on where you look from. \
             `gore npc sites --free --level {}` lists points with nobody on them",
            request.at,
            point.occupants.join(", "),
            level_module
                .strip_prefix(LEVEL_SCRIPT_PREFIX)
                .unwrap_or(&level_module),
        )
    });

    let Some(pristine) = emitted.level_sources.get(&level_module) else {
        bail!("no emitted source for {level_module}");
    };

    // Fuer jedes Glied die naechste ableitbare Elternklasse suchen. Von der Vorlage selbst geht
    // es nicht: ihr `__InitDefaults` ist `final`, weil nichts von ihr erbt.
    let counts = &emitted.subclass_counts;
    let (definition_parent, definition_defaults) = derivable_parent(
        &emitted.classes,
        counts,
        &format!("UCharacterDefinition_Human_{}", request.from),
    );
    let (config_parent, _) = derivable_parent(
        &emitted.classes,
        counts,
        &format!("UAIAgentConfig_Human_{}", request.from),
    );
    let (spawn_parent, _) = derivable_parent(&emitted.classes, counts, &template_spawn);

    // Die Aussehensklasse liegt nicht in der Kette, sondern in einem geteilten Modul. Sie wird
    // hier einzeln nachgeschlagen; findet sich keine, bleibt die Vorlage stehen und der Compiler
    // sagt es deutlicher, als eine Vermutung hier es könnte.
    let visuals_class = format!("UCharacterVisualsDefinition_Human_{}", request.from);
    let mut visuals_classes = emitted.classes.clone();
    if let Some(index) = model::parse_modules(&read_module_cache(&path)?)
        .ok()
        .and_then(|modules| module_of_class(&modules, &visuals_class).map(|i| (modules, i)))
    {
        let (modules, i) = index;
        if let Some(source) = emit_named_module(&path, &modules[i].name)? {
            for class in defaults::parse_classes(&source) {
                visuals_classes.insert(class.name.clone(), class);
            }
        }
    }
    let (visuals_parent, visuals_defaults) =
        derivable_parent(&visuals_classes, counts, &visuals_class);

    let npc = generate::NewNpc {
        id: request.id.clone(),
        derived_from: request.from.clone(),
        guild: request.guild.clone(),
        waypoint: request.waypoint.clone(),
        voice_tag: voice_of(&path, &request.from)?,
        modular_visuals: request.modular_visuals,
        trader: request.trader,
        definition_parent,
        definition_defaults,
        visuals_parent,
        visuals_defaults,
        config_parent,
        spawn_parent,
    };
    let routine = generate::routine_class(&npc);
    let edited = edit::add_spawn(pristine, &request.at, &new_spawn, routine.as_deref())
        .with_context(|| format!("adding the spawn line to {level_module}"))?;

    create_workspace(out)?;

    let module_relative = generate::relative_path(&request.id);
    let module_leaf = leaf_of(&module_relative).to_string();
    fs::write(out.join(&module_leaf), generate::source(&npc))
        .with_context(|| format!("writing {module_leaf}"))?;

    let level_relative = format!("{}.as", level_module.replace('.', "/"));
    let level_leaf = leaf_of(&level_relative).to_string();
    fs::write(out.join(&level_leaf), &edited).with_context(|| format!("writing {level_leaf}"))?;
    fs::write(out.join("pristine").join(&level_leaf), pristine)
        .with_context(|| format!("writing pristine/{level_leaf}"))?;

    let manifest = workspace::Manifest {
        operation: workspace::Operation::New,
        npc_id: request.id.clone(),
        derived_from: Some(request.from.clone()),
        modules: vec![
            workspace::ModuleEdit {
                module: generate::module_name(&request.id),
                relative_path: module_relative,
                source_file: module_leaf.clone(),
                pristine_file: None,
                op: "add".to_string(),
            },
            workspace::ModuleEdit {
                module: level_module.clone(),
                relative_path: level_relative,
                source_file: level_leaf.clone(),
                pristine_file: Some(format!("pristine/{level_leaf}")),
                op: "edit".to_string(),
            },
        ],
        world_points: vec![request.at.clone()],
        level_module: level_module.clone(),
        cache_sha256: emitted.cache_sha256(),
        modular_visuals: request.modular_visuals,
    };
    write_manifest(out, &manifest)?;

    println!("authored {} in {}", request.id, out.display());
    let class_count = generate::source(&npc)
        .lines()
        .filter(|line| line.starts_with("class "))
        .count();
    println!("  {module_leaf}  the character, {class_count} classes");
    println!("  {level_leaf}  one added spawn line at {}", request.at);
    println!("  {}", render::translation_line(&emitted, &level_module));
    if let Some(warning) = &occupied_warning {
        println!("  WARNING: {warning}");
    }
    if request.modular_visuals {
        println!(
            "  NOTE: --modular-visuals has no shipped precedent; check the comment in {module_leaf}"
        );
    }
    println!("next: gore npc check {}", out.display());
    Ok(())
}

/// `gore npc delete` — eine ausgelieferte Figur nicht mehr setzen lassen.
fn suppress(npc: &str, cache: Option<PathBuf>, game: Option<PathBuf>, out: &Path) -> Result<()> {
    let spawn_class = generate::spawn_class(npc);
    let emitted = emit_index(cache, game, Some(&spawn_class))?;
    if !emitted.classes.contains_key(&spawn_class) {
        bail!(
            "no character {npc} in this cache — {spawn_class} is not declared. \
             `gore npc list {npc}` shows the ids that exist"
        );
    }
    let mine = sites_for(&emitted, &spawn_class);
    if mine.is_empty() {
        bail!(
            "{npc} is never placed by a level script, so there is no spawn line to remove. \
             Characters spawned another way cannot be suppressed this way"
        );
    }
    let modules: BTreeSet<&str> = mine.iter().map(|site| site.module.as_str()).collect();
    if modules.len() > 1 {
        bail!(
            "{npc} is placed from {} level scripts ({}). One bundle entry carries one edited \
             level script, so suppressing this character would need one mod per script",
            modules.len(),
            modules.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    let level_module = mine[0].module.clone();
    let Some(pristine) = emitted.level_sources.get(&level_module) else {
        bail!("no emitted source for {level_module}");
    };
    let edited = edit::remove_spawn(pristine, &spawn_class)
        .with_context(|| format!("removing the spawn lines from {level_module}"))?;

    create_workspace(out)?;
    let level_relative = format!("{}.as", level_module.replace('.', "/"));
    let level_leaf = leaf_of(&level_relative).to_string();
    fs::write(out.join(&level_leaf), &edited).with_context(|| format!("writing {level_leaf}"))?;
    fs::write(out.join("pristine").join(&level_leaf), pristine)
        .with_context(|| format!("writing pristine/{level_leaf}"))?;

    let manifest = workspace::Manifest {
        operation: workspace::Operation::Suppress,
        npc_id: npc.to_string(),
        derived_from: None,
        modules: vec![workspace::ModuleEdit {
            module: level_module.clone(),
            relative_path: level_relative,
            source_file: level_leaf.clone(),
            pristine_file: Some(format!("pristine/{level_leaf}")),
            op: "edit".to_string(),
        }],
        world_points: mine.iter().map(|site| site.world_point.clone()).collect(),
        level_module: level_module.clone(),
        cache_sha256: emitted.cache_sha256(),
        modular_visuals: false,
    };
    write_manifest(out, &manifest)?;

    println!("{npc} will no longer be placed, from {}", out.display());
    for site in &mine {
        println!("  removed from {}", site.world_point);
    }
    println!("  {}", render::translation_line(&emitted, &level_module));
    // Der Körper steht schon in jedem Spielstand, der ihn einmal gesehen hat. Das muss dastehen,
    // sonst hält jemand den Mod für kaputt.
    println!(
        "  NOTE: this only stops future placement. A save that already spawned {npc} still \
         carries that body"
    );
    println!("next: gore npc check {}", out.display());
    Ok(())
}

/// Das Manifest atomar genug schreiben: eine frische Datei in einem frischen Verzeichnis.
fn write_manifest(out: &Path, manifest: &workspace::Manifest) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest).context("serializing the manifest")?;
    fs::write(out.join(workspace::MANIFEST_NAME), format!("{json}\n"))
        .with_context(|| format!("writing {}", workspace::MANIFEST_NAME))
}

/// Das Manifest eines Arbeitsverzeichnisses lesen.
fn read_manifest(dir: &Path) -> Result<workspace::Manifest> {
    let path = dir.join(workspace::MANIFEST_NAME);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {}. Is {} a workspace written by `gore npc new` or `gore npc delete`?",
            path.display(),
            dir.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// `gore npc check` — das Arbeitsverzeichnis gegen den Vertrag prüfen.
fn check_workspace(dir: &Path, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let manifest = read_manifest(dir)?;
    let spawn_class = generate::spawn_class(&manifest.npc_id);
    let emitted = emit_index(cache, game, Some(&spawn_class))?;

    let mut findings: Vec<check::Finding> = Vec::new();

    // Die Grundlage zuerst: gegen eine andere Cache zu prüfen hiesse, gar nicht zu pruefen.
    if emitted.cache_sha256() != manifest.cache_sha256 {
        findings.push(check::Finding {
            severity: check::Severity::Blocking,
            message: format!(
                "this workspace was authored against script cache {} but the installed one is {}. \
                 The game was patched or another cache is configured; author it again",
                &manifest.cache_sha256[..16],
                &emitted.cache_sha256()[..16]
            ),
        });
    }

    let Some(level) = manifest.level_edit() else {
        bail!("the manifest names no edited level script");
    };
    let edited = fs::read_to_string(dir.join(&level.source_file))
        .with_context(|| format!("reading {}", level.source_file))?;
    let pristine_rel = level
        .pristine_file
        .as_deref()
        .unwrap_or("pristine/missing.as");
    let pristine = fs::read_to_string(dir.join(pristine_rel))
        .with_context(|| format!("reading {pristine_rel}"))?;
    // Ein Checkout aendert Werte im eigenen Modul der Figur; ein Verfassen aendert Spawn-Zeilen
    // in einem fremden Levelskript. Zwei Absichten, zwei Waechter.
    if manifest.operation == workspace::Operation::Checkout {
        findings.extend(check::guard_checkout_diff(&pristine, &edited));
    } else {
        findings.extend(check::guard_level_diff(&pristine, &edited, &spawn_class));
    }

    if let Some(authored) = manifest.authored_module() {
        let source = fs::read_to_string(dir.join(&authored.source_file))
            .with_context(|| format!("reading {}", authored.source_file))?;
        findings.extend(check::guard_authored_module(&source, &manifest.npc_id));

        if emitted.classes.contains_key(&spawn_class) {
            findings.push(check::Finding {
                severity: check::Severity::Blocking,
                message: format!(
                    "{} is already a character in this game. The authored module would collide \
                     with the shipped one",
                    manifest.npc_id
                ),
            });
        }

        let spots = gore_catalog::location::LocationCatalog::bundled()
            .context("reading the bundled location catalog")?;
        for waypoint in check::scheduled_waypoints(&source) {
            if spots.resolve(&waypoint).is_none() {
                findings.push(check::Finding {
                    severity: check::Severity::Warning,
                    message: format!(
                        "the routine sends the character to {waypoint:?}, which is not a known \
                         spot. The game ignores an unknown waypoint without a word, so the \
                         character would simply never go there"
                    ),
                });
            }
        }
    }

    println!(
        "{}",
        render::translation_line(&emitted, &manifest.level_module)
    );
    if findings.is_empty() {
        println!("no problems found in {}", dir.display());
        println!(
            "offline-checked only: that this character appears, keeps its routine and survives a \
             save is not proven in game"
        );
        println!("next: gore npc stage {}", dir.display());
        return Ok(());
    }

    let blocking = findings
        .iter()
        .filter(|f| f.severity == check::Severity::Blocking)
        .count();
    for finding in &findings {
        let tag = match finding.severity {
            check::Severity::Blocking => "blocking",
            check::Severity::Warning => "warning",
        };
        println!("  [{tag}] {}", finding.message);
    }
    if blocking > 0 {
        bail!("{blocking} blocking problem(s) in {}", dir.display());
    }
    println!("{} warning(s), nothing blocking", findings.len());
    println!("next: gore npc stage {}", dir.display());
    Ok(())
}

/// Den Quellbaum vorhalten: einmal emittieren, danach an der Cache-Kennung wiedererkennen.
///
/// Der Lauf kostet rund 19 Minuten, davon das meiste ein einziges Modul
/// (`Map.MainMap.WorldPointManagerConfig_MainMap`). Ihn bei jedem `stage` zu wiederholen waere
/// nicht zumutbar, also bekommt der Baum einen Stempel und wird wiederverwendet.
fn ensure_tree(tree: &Path, cache_sha256: &str, path: &Path) -> Result<()> {
    let stamp_path = tree.join(stage::TREE_STAMP_NAME);
    if let Ok(text) = fs::read_to_string(&stamp_path) {
        if let Ok(stamp) = serde_json::from_str::<stage::TreeStamp>(&text) {
            if stamp.cache_sha256 == cache_sha256 {
                println!(
                    "reusing the source tree in {} ({} modules)",
                    tree.display(),
                    stamp.modules
                );
                return Ok(());
            }
        }
        bail!(
            "the source tree in {} was emitted from a different script cache. Delete it and let \
             this command write a fresh one",
            tree.display()
        );
    }
    if tree.exists() && fs::read_dir(tree)?.next().is_some() {
        bail!(
            "{} is not empty and carries no tree stamp. Point --tree at a fresh directory",
            tree.display()
        );
    }

    println!(
        "emitting the source tree into {} — this takes around 19 minutes, once per game version",
        tree.display()
    );
    let bytes = read_module_cache(path)?;
    let mut resolver = RefResolver::build(&bytes).context("building the reference resolver")?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let loaded = load_native_api_with_proof(path);
    let prepared = PreparedEmit::new(&modules, &mut resolver, loaded.map(|l| l.native))
        .context("preparing the emitted modules")?
        .with_class_defaults(true);
    fs::create_dir_all(tree).with_context(|| format!("creating {}", tree.display()))?;
    prepared
        .emit_tree(tree)
        .with_context(|| format!("emitting the tree into {}", tree.display()))?;
    let stamp = stage::TreeStamp {
        cache_sha256: cache_sha256.to_string(),
        modules: modules.len(),
    };
    fs::write(
        &stamp_path,
        format!("{}\n", serde_json::to_string_pretty(&stamp)?),
    )
    .with_context(|| format!("writing {}", stamp_path.display()))?;
    println!("emitted {} modules", modules.len());
    Ok(())
}

/// Die verfassten Dateien an ihre Stellen im Baum kopieren.
fn overlay_authored(dir: &Path, tree: &Path, manifest: &workspace::Manifest) -> Result<()> {
    for edit in &manifest.modules {
        let target = tree.join(&edit.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        let source = dir.join(&edit.source_file);
        fs::copy(&source, &target)
            .with_context(|| format!("copying {} to {}", source.display(), target.display()))?;
    }
    Ok(())
}

/// `gore npc stage` — Baum herrichten, Spec schreiben, Bau-Kommandos drucken.
fn stage_workspace(
    dir: &Path,
    tree: Option<&Path>,
    mod_name: &str,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<()> {
    gore_mod::validate_mod_name(mod_name).context("invalid --mod-name")?;
    let manifest = read_manifest(dir)?;
    let path = cache_path(cache, game.clone())?;
    // Der Weg haengt auch am Inhalt: ein Modul mit Klassen-Defaults lehnt `compile-module` ab.
    let edited_source = match manifest.level_edit() {
        Some(edit) => fs::read_to_string(dir.join(&edit.source_file))
            .with_context(|| format!("reading {}", edit.source_file))?,
        None => String::new(),
    };
    let route = stage::route_of(&manifest, &edited_source);

    let tree_display = match (route, tree) {
        (stage::Route::FullTree, None) => {
            let why = if manifest.authored_module().is_some() {
                "a new character brings a module of its own that the level script calls, and the \
                 two only compile together"
            } else {
                "this module carries class defaults, and `compile-module` refuses those: it \
                 cannot inventory their `__InitDefaults` and says so rather than guessing"
            };
            bail!(
                "this work needs a source tree: pass --tree <dir>. {why}. The tree is emitted \
                 once per game version and reused after that"
            )
        }
        (stage::Route::FullTree, Some(tree)) => {
            ensure_tree(tree, &manifest.cache_sha256, &path)?;
            overlay_authored(dir, tree, &manifest)?;
            tree.display().to_string()
        }
        // Der Ein-Modul-Weg overlayt die Quelle selbst; ein Baum waere verschenkte Zeit.
        (stage::Route::SingleModule, _) => "(not needed)".to_string(),
    };

    // Der Compiler verlangt einen **vorhandenen** privaten Arbeitsordner und bricht sonst mit
    // "reading compiler workspace metadata" ab. Ihn hier anzulegen erspart dem Nutzer ein
    // Kommando, das aussieht, als sei es vollständig, und dann scheitert.
    let work = PathBuf::from(format!("{}.work", dir.display()));
    fs::create_dir_all(&work).with_context(|| format!("creating {}", work.display()))?;

    let spec = stage::spec_json(&manifest, mod_name);
    let spec_path = dir.join("spec.json");
    fs::write(
        &spec_path,
        format!("{}\n", serde_json::to_string_pretty(&spec)?),
    )
    .with_context(|| format!("writing {}", spec_path.display()))?;

    let game_arg = game.as_ref().map(|path| path.display().to_string());
    let commands = stage::build_commands(
        &manifest,
        &edited_source,
        &dir.display().to_string(),
        &tree_display,
        mod_name,
        game_arg.as_deref(),
    );

    println!("wrote {}", spec_path.display());
    println!("now run:");
    for command in &commands {
        println!("  {command}");
    }
    println!(
        "then: gore mod deploy --bundle {}/build/{mod_name}",
        dir.display()
    );
    println!(
        "offline-prepared only: whether this character appears in game is decided by that run, \
         not by this one"
    );
    Ok(())
}

/// Die deutschen Spalten, in die ein Anzeigename gehoert.
///
/// Beide, nicht nur eine: wo `german_new` existiert, gewinnt sie gegen `german`. Ein Dokument,
/// das nur `german` setzt, ist dort ein stiller Fehlschlag — ein Fehler, der in diesem Projekt
/// schon einmal Zeit gekostet hat. Fuer eine neue Id existiert keine von beiden, beide zu setzen
/// ist also richtig und schadet nirgends.
const GERMAN_COLUMNS_OUT: &[&str] = &["german", "german_new"];

/// Die englischen Spalten, nach derselben Regel.
const ENGLISH_COLUMNS_OUT: &[&str] = &["english", "english_new", "english_newer"];

/// Das Bearbeitungsdokument fuer einen Anzeigenamen.
///
/// Die Lokalisierungs-Id einer Figur ist ihre Id in Kleinbuchstaben — `oc_stt_diego` fuer
/// `OC_STT_Diego`.
pub fn display_name_edits(id: &str, german: &str, english: Option<&str>) -> serde_json::Value {
    let mut columns = serde_json::Map::new();
    for column in GERMAN_COLUMNS_OUT {
        columns.insert((*column).to_string(), serde_json::json!(german));
    }
    if let Some(english) = english {
        for column in ENGLISH_COLUMNS_OUT {
            columns.insert((*column).to_string(), serde_json::json!(english));
        }
    }
    serde_json::json!({ id.to_lowercase(): columns })
}

/// `gore npc text` — den Anzeigenamen als `gore loc import --edits`-Dokument schreiben.
fn write_display_name(id: &str, name: &str, english: Option<&str>, out: &Path) -> Result<()> {
    if out.exists() {
        bail!(
            "{} already exists. Point -o at a file that does not exist yet",
            out.display()
        );
    }
    let document = display_name_edits(id, name, english);
    fs::write(
        out,
        format!("{}\n", serde_json::to_string_pretty(&document)?),
    )
    .with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {}", out.display());
    println!("  {} -> {name:?} in both German columns", id.to_lowercase());
    println!("next: gore loc import --edits {}", out.display());
    Ok(())
}

/// `gore npc checkout` — das eigene Modul einer ausgelieferten Figur zum Bearbeiten herausnehmen.
///
/// Herausgenommen wird das Modul, das ihre `CharacterDefinition` deklariert: dort stehen Werte,
/// Inventar, Fraktion und Faehigkeiten. Aussehen und Spawn-Definition liegen in geteilten Modulen
/// (`InteractiveObjects/NpcVisualLibrary.as`, `Spawning/SpawningDefinition_Human.as`) und bleiben
/// bewusst aussen vor — sie mitzunehmen hiesse, ein von hunderten Figuren geteiltes Modul zu
/// ersetzen, um eine einzige zu aendern.
fn checkout(npc: &str, cache: Option<PathBuf>, game: Option<PathBuf>, out: &Path) -> Result<()> {
    let spawn_class = generate::spawn_class(npc);
    let path = cache_path(cache.clone(), game.clone())?;
    let emitted = emit_index(cache, game, Some(&spawn_class))?;
    if !emitted.classes.contains_key(&spawn_class) {
        bail!(
            "no character {npc} in this cache — {spawn_class} is not declared. \
             `gore npc list {npc}` shows the ids that exist"
        );
    }
    let chain = chain::resolve(&emitted.classes, &spawn_class);
    let Some(definition) = chain.character_definition.as_deref() else {
        bail!(
            "the chain of {npc} does not reach a character definition, so there is nothing to \
             check out. `gore npc show {npc}` shows where it breaks"
        );
    };

    let bytes = read_module_cache(&path)?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let Some(index) = module_of_class(&modules, definition) else {
        bail!("no module declares {definition}");
    };
    let module_name = modules[index].name.clone();
    let Some(source) = emit_named_module(&path, &module_name)? else {
        bail!("no emitted source for {module_name}");
    };

    let relative_path = format!("{}.as", module_name.replace('.', "/"));
    let leaf = leaf_of(&relative_path).to_string();

    create_workspace(out)?;
    fs::write(out.join(&leaf), &source).with_context(|| format!("writing {leaf}"))?;
    fs::write(out.join("pristine").join(&leaf), &source)
        .with_context(|| format!("writing pristine/{leaf}"))?;

    let manifest = workspace::Manifest {
        operation: workspace::Operation::Checkout,
        npc_id: npc.to_string(),
        derived_from: None,
        modules: vec![workspace::ModuleEdit {
            module: module_name.clone(),
            relative_path,
            source_file: leaf.clone(),
            pristine_file: Some(format!("pristine/{leaf}")),
            op: "edit".to_string(),
        }],
        world_points: sites_for(&emitted, &spawn_class)
            .iter()
            .map(|site| site.world_point.clone())
            .collect(),
        level_module: module_name.clone(),
        cache_sha256: emitted.cache_sha256(),
        modular_visuals: false,
    };
    write_manifest(out, &manifest)?;

    let class_count = defaults::parse_classes(&source).len();
    println!("checked {npc} out into {}", out.display());
    println!("  {leaf}  {class_count} classes from {module_name}");
    println!("  {}", render::translation_line(&emitted, &module_name));
    println!(
        "  edit the values; class names and their parents have to stay as they are, because they \
         are the character's identity in the cache"
    );
    println!("next: gore npc check {}", out.display());
    Ok(())
}

/// Von welcher Klasse abgeleitet werden darf, und welche Werte dabei mitkommen müssen.
///
/// Der Compiler erklärt das erzeugte `__InitDefaults` einer Klasse ohne Unterklassen für `final`.
/// Eine ausgelieferte Figur ist fast immer so ein Blatt, also ist „von Diego ableiten" kein
/// gangbarer Weg — der Versuch endet in
/// `Method '…::__InitDefaults()' declared as final and cannot be overridden`.
///
/// Deshalb wird von hier aus aufwärts gegangen, bis eine Klasse mit Geschwistern kommt, und alles,
/// was dabei übersprungen wird, wird ausgeschrieben. Die Reihenfolge bleibt dabei die des Spiels:
/// je näher an der Vorlage, desto später steht der Wert und desto stärker gewinnt er.
fn derivable_parent(
    classes: &BTreeMap<String, defaults::EmittedClass>,
    subclass_counts: &BTreeMap<String, usize>,
    class_name: &str,
) -> (String, Vec<String>) {
    const MAX_CLIMB: usize = 8;
    const IDENTITY: &[&str] = &["m_UniqueName", "m_CharacterVisualsDefinition"];

    let mut collected: Vec<String> = Vec::new();
    let mut current = class_name.to_string();
    for _ in 0..MAX_CLIMB {
        if subclass_counts.get(&current).copied().unwrap_or(0) > 0 {
            return (current, collected);
        }
        let Some(class) = classes.get(&current) else {
            // Ohne emittierte Quelle ist nichts auszuschreiben; dann bleibt nur, es zu versuchen.
            return (current, collected);
        };
        let mut own: Vec<String> = class
            .assignments
            .iter()
            .filter(|(lhs, _)| !IDENTITY.contains(&lhs.as_str()))
            .map(|(lhs, rhs)| format!("{lhs} = {rhs}"))
            .collect();
        own.extend(class.calls.iter().cloned());
        // Die Werte der übersprungenen Klasse gehören vor die schon gesammelten: die stammen von
        // einer näheren Nachfahrin und müssen später stehen.
        own.extend(collected);
        collected = own;

        let Some(parent) = class.super_class.clone() else {
            return (current, collected);
        };
        current = parent;
    }
    (current, collected)
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
            level_sources: BTreeMap::new(),
            world_points: Vec::new(),
            subclass_counts: BTreeMap::new(),
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

    #[test]
    fn a_display_name_document_is_keyed_by_the_lowercased_id() {
        let document = display_name_edits("MY_NPC", "Hannes", None);
        assert!(document.get("my_npc").is_some());
        assert!(document.get("MY_NPC").is_none());
    }

    #[test]
    fn both_german_columns_are_written() {
        // `german_new` gewinnt, wo sie existiert. Nur `german` zu setzen waere dort ein stiller
        // Fehlschlag, und der sieht aus wie ein kaputtes Werkzeug.
        let document = display_name_edits("MY_NPC", "Hannes", None);
        let columns = &document["my_npc"];
        assert_eq!(columns["german"], "Hannes");
        assert_eq!(columns["german_new"], "Hannes");
    }

    #[test]
    fn english_is_written_to_every_english_column_when_asked_for() {
        let document = display_name_edits("MY_NPC", "Hannes", Some("Hank"));
        let columns = &document["my_npc"];
        for column in ["english", "english_new", "english_newer"] {
            assert_eq!(columns[column], "Hank", "{column}");
        }
    }

    #[test]
    fn without_an_english_name_no_english_column_is_touched() {
        let document = display_name_edits("MY_NPC", "Hannes", None);
        let columns = document["my_npc"].as_object().expect("columns");
        assert!(columns.keys().all(|key| key.starts_with("german")));
    }
}
