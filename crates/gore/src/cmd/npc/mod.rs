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
pub mod routine;
pub mod routine_plan;
pub mod routine_spots;
pub mod sites;
pub mod stage;
pub mod workspace;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{bail, ensure, Context, Result};
use clap::Subcommand;
use gore_as::cache::{emit_all::PreparedEmit, faithfulness, model, refs::RefResolver};

use super::as_cache::{load_native_api_with_proof, read_module_cache};
use super::find::{bundled_catalog, CatalogEntry};
use sites::Site;

#[derive(Subcommand)]
pub enum NpcAction {
    /// Edit and inspect a new NPC's daily schedule
    Routine {
        #[command(subcommand)]
        action: routine::RoutineAction,
    },
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
    /// Snapshot the authored modules and print their compile and bundle commands
    Stage {
        /// The workspace directory written by `new` or `delete`
        dir: PathBuf,
        /// Obsolete: omit this option; staging now snapshots only the authored modules
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
    /// Check out a shipped character's own module for edits that preserve its existing default targets
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
    /// List every level script that offers world points for character placement
    Levels {
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
        NpcAction::Routine { action } => routine::run(action),
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
        NpcAction::Levels { cache, game, json } => list_levels(cache, game, json),
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

/// The script cache to inspect: the one named, else the live one in the resolved install.
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

/// Authored work and its checks use the same pristine base as the standalone compiler.
/// Inspection commands can still show the live installed state.
fn authoring_cache_path(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(cache) = cache {
        return Ok(cache);
    }
    let root = gore_loc::config::game_root(game).context("resolving the game path")?;
    cache_path(None, Some(root.clone()))?;
    Ok(gore_mod::pristine_script_cache_source(&root)
        .context("selecting the pristine NPC base cache")?
        .path)
}

/// Resolve the current pristine input while retaining explicit selections for later commands.
struct AuthoringInputs {
    cache: PathBuf,
    explicit_cache: bool,
    game: Option<PathBuf>,
}

impl AuthoringInputs {
    fn resolve(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<Self> {
        let explicit_cache = cache.is_some();
        // An explicit cache remains usable offline without consulting another installation.
        let game = if game.is_some() || cache.is_none() {
            let root = gore_loc::config::game_root(game).context("resolving the NPC game path")?;
            Some(
                fs::canonicalize(&root)
                    .with_context(|| format!("resolving NPC game directory {}", root.display()))?,
            )
        } else {
            None
        };
        let cache = authoring_cache_path(cache, game.clone())?;
        let cache = fs::canonicalize(&cache)
            .with_context(|| format!("resolving NPC script cache {}", cache.display()))?;
        Ok(Self {
            cache,
            explicit_cache,
            game,
        })
    }

    fn command(&self, action: &str, dir: &Path) -> String {
        // A game-selected pristine cache may be an owned backup removed by undeploy. Keep the
        // installation selection so the next command finds its then-current pristine source.
        let cache = if self.explicit_cache {
            format!(
                " --cache {}",
                stage::shell_quote(&self.cache.display().to_string())
            )
        } else {
            String::new()
        };
        let game = self
            .game
            .as_ref()
            .map(|path| {
                format!(
                    " --game {}",
                    stage::shell_quote(&path.display().to_string())
                )
            })
            .unwrap_or_default();
        format!(
            "gore npc {action} {}{cache}{game}",
            stage::shell_quote(&dir.display().to_string()),
        )
    }
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

/// Compare every declaration in an authored module with all classes in the installed cache.
/// The spawn class alone is insufficient: visuals, settings, routines, and trader configs are
/// emitted into the same new module and must also be globally unique.
fn colliding_class_names<'a>(
    shipped: impl Iterator<Item = &'a str>,
    authored_source: &str,
) -> Vec<String> {
    let existing: std::collections::HashSet<String> =
        shipped.map(str::to_ascii_lowercase).collect();
    let mut collisions: Vec<String> = defaults::parse_classes(authored_source)
        .into_iter()
        .filter(|class| existing.contains(&class.name.to_ascii_lowercase()))
        .map(|class| class.name)
        .collect();
    collisions.sort();
    collisions.dedup();
    collisions
}

fn cache_class_collisions(modules: &[model::Module], authored_source: &str) -> Vec<String> {
    colliding_class_names(
        modules
            .iter()
            .flat_map(|module| module.classes.iter().map(|class| class.name.as_str())),
        authored_source,
    )
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

/// Resolve the shipped module that owns the selected NPC's character definition.
///
/// Checkout manifests are editable input, so their module name cannot be the authority for which
/// character is being changed. Follow the selected NPC's exact cache-backed class chain instead.
fn checkout_target_module(path: &Path, npc_id: &str) -> Result<(String, String)> {
    let bytes = read_module_cache(path)?;
    let mut resolver = RefResolver::build(&bytes).context("building the reference resolver")?;
    let modules = model::parse_modules(&bytes).context("parsing modules")?;
    let loaded = load_native_api_with_proof(path);
    let prepared = PreparedEmit::new(&modules, &mut resolver, loaded.map(|loaded| loaded.native))
        .context("preparing the emitted modules")?
        .with_class_defaults(true);

    let spawn_class = generate::spawn_class(npc_id);
    let mut classes = BTreeMap::new();
    let mut wanted = Some(spawn_class.clone());
    for _ in 0..CHAIN_HOPS {
        let class_name = wanted.take().with_context(|| {
            format!("the class chain for {npc_id} ends before its character definition")
        })?;
        let index = module_of_class(&modules, &class_name)
            .with_context(|| format!("no module declares {class_name}"))?;
        let source = prepared
            .emit_module(index)
            .with_context(|| format!("emitting {}", modules[index].name))?;
        for class in defaults::parse_classes(&source) {
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

    let definition = chain::resolve(&classes, &spawn_class)
        .character_definition
        .with_context(|| format!("the class chain for {npc_id} has no character definition"))?;
    let index = module_of_class(&modules, &definition)
        .with_context(|| format!("no module declares {definition}"))?;
    Ok((definition, modules[index].name.clone()))
}

fn checkout_module_binding_finding(
    edit: &workspace::ModuleEdit,
    definition: &str,
    expected_module: &str,
) -> Option<check::Finding> {
    (edit.module != expected_module).then(|| check::Finding {
        severity: check::Severity::Blocking,
        message: format!(
            "checkout for {definition} must edit its declaring module {expected_module}, not {}",
            edit.module
        ),
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

fn suppression_scope_finding(
    emitted: &Emitted,
    spawn_class: &str,
    level_module: &str,
) -> Option<check::Finding> {
    let modules: BTreeSet<&str> = sites_for(emitted, spawn_class)
        .into_iter()
        .map(|site| site.module.as_str())
        .collect();
    if modules.len() == 1 && modules.contains(level_module) {
        return None;
    }
    Some(check::Finding {
        severity: check::Severity::Blocking,
        message: format!(
            "the suppression edits {level_module}, but {spawn_class} is placed from {}. A \
             suppression must cover every shipped spawn module",
            if modules.is_empty() {
                "no level scripts".to_string()
            } else {
                modules.into_iter().collect::<Vec<_>>().join(", ")
            }
        ),
    })
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

#[derive(Debug, PartialEq, Eq)]
struct Level {
    name: String,
    module: String,
    world_points: usize,
    occupied_points: usize,
}

/// Short, readable label alongside the exact module name accepted by `sites --level`.
fn level_name(module: &str) -> String {
    let mut name = module.strip_prefix(LEVEL_SCRIPT_PREFIX).unwrap_or(module);
    if let Some(rest) = name.strip_prefix("Map_") {
        let mut parts = rest.splitn(3, '_');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(x), Some(y), Some(tail))
                if x.strip_prefix('x')
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                    && y.strip_prefix('y').is_some_and(|n| {
                        !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
                    }) =>
            {
                name = tail;
            }
            _ => name = rest,
        }
    }
    for suffix in ["_AI_script", "_AI", "_script"] {
        if let Some(short) = name.strip_suffix(suffix) {
            name = short;
            break;
        }
    }
    name.to_string()
}

fn levels_from_world_points(points: &[sites::WorldPoint]) -> Vec<Level> {
    let mut by_module: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for point in points {
        let entry = by_module.entry(&point.module).or_default();
        entry.0 += 1;
        entry.1 += usize::from(point.is_occupied());
    }
    let mut levels: Vec<Level> = by_module
        .into_iter()
        .map(|(module, (world_points, occupied_points))| Level {
            name: level_name(module),
            module: module.to_string(),
            world_points,
            occupied_points,
        })
        .collect();
    levels.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.module.cmp(&b.module)));
    levels
}

fn list_levels(cache: Option<PathBuf>, game: Option<PathBuf>, json: bool) -> Result<()> {
    let emitted = emit_index(cache, game, None)?;
    let levels = levels_from_world_points(&emitted.world_points);
    if json {
        let rows: Vec<serde_json::Value> = levels
            .iter()
            .map(|level| {
                serde_json::json!({
                    "name": level.name,
                    "module": level.module,
                    "world_points": level.world_points,
                    "occupied_points": level.occupied_points,
                    "free_points": level.world_points - level.occupied_points,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "count": rows.len(),
                "levels": rows,
            }))?
        );
        return Ok(());
    }
    for level in &levels {
        println!("{:<22} {}", level.name, level.module);
    }
    println!("{} level(s)", levels.len());
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
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::create_dir(out).with_context(|| {
        format!(
            "creating new NPC workspace {} (it must not already exist)",
            out.display()
        )
    })?;
    fs::create_dir(out.join("pristine"))
        .with_context(|| format!("creating pristine/ in {}", out.display()))?;
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

fn ensure_human_template(
    classes: &BTreeMap<String, defaults::EmittedClass>,
    from: &str,
) -> Result<()> {
    let chain = chain::resolve(classes, &generate::spawn_class(from));
    let config = format!("UAIAgentConfig_Human_{from}");
    let definition = format!("UCharacterDefinition_Human_{from}");
    ensure!(
        chain.ai_agent_config.as_deref() == Some(config.as_str())
            && chain.character_definition.as_deref() == Some(definition.as_str())
            && classes.contains_key(&config)
            && classes.contains_key(&definition),
        "--from {from:?} is not a supported human NPC template: its spawn must resolve through {config} to {definition}"
    );
    Ok(())
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
    if let Some(waypoint) = &request.waypoint {
        routine_plan::validate_name_literal(waypoint, "waypoint")?;
    }

    let inputs = AuthoringInputs::resolve(cache, game)?;
    let path = inputs.cache.clone();
    let template_spawn = generate::spawn_class(&request.from);
    let emitted = emit_index(Some(path.clone()), None, Some(&template_spawn))?;
    let modules = model::parse_modules(&read_module_cache(&path)?)
        .context("parsing modules for NPC class validation")?;

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
    ensure_human_template(&emitted.classes, &request.from)?;
    let new_spawn = generate::spawn_class(&request.id);
    if let Some(guild) = &request.guild {
        let guild_class = format!("UCharacterDefinition_Human_{guild}");
        ensure!(
            is_valid_id(guild)
                && module_of_class(&modules, &guild_class).is_some()
                && is_derivable_base(&emitted.subclass_counts, &guild_class),
            "unknown, invalid, or final guild base {guild:?}: {guild_class} must be a declared \
             class with at least one subclass in this cache"
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
    // Die mitgeschleppten Werte NICHT wegwerfen: in der Spawn-Definition steckt
    // `AIAgentCharacterClass`, der Actor-Blueprint. Ohne ihn hat die Figur keinen Koerper.
    let (config_parent, config_defaults) = derivable_parent(
        &emitted.classes,
        counts,
        &format!("UAIAgentConfig_Human_{}", request.from),
    );
    let (spawn_parent, spawn_defaults) =
        derivable_parent(&emitted.classes, counts, &template_spawn);

    // Die Aussehensklasse liegt nicht in der Kette, sondern in einem geteilten Modul.
    let visuals_class = format!("UCharacterVisualsDefinition_Human_{}", request.from);
    let mut visuals_classes = emitted.classes.clone();
    let i = module_of_class(&modules, &visuals_class).with_context(|| {
        format!(
            "--from {:?} is not a supported human NPC template: {visuals_class} is missing",
            request.from
        )
    })?;
    let source = emit_named_module(&path, &modules[i].name)?
        .with_context(|| format!("no emitted source for {visuals_class}"))?;
    for class in defaults::parse_classes(&source) {
        visuals_classes.insert(class.name.clone(), class);
    }
    ensure!(
        visuals_classes.contains_key(&visuals_class),
        "--from {:?} is not a supported human NPC template: {visuals_class} is missing",
        request.from
    );
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
        config_defaults,
        spawn_parent,
        spawn_defaults,
    };
    let source = generate::source(&npc);
    let collisions = cache_class_collisions(&modules, &source);
    if !collisions.is_empty() {
        bail!(
            "generated NPC classes already exist in this game: {}. Pick another id",
            collisions.join(", ")
        );
    }
    let routine = generate::routine_class(&npc);
    let edited = edit::add_spawn(pristine, &request.at, &new_spawn, routine.as_deref())
        .with_context(|| format!("adding the spawn line to {level_module}"))?;

    let module_relative = generate::relative_path(&request.id);
    let module_leaf = leaf_of(&module_relative).to_string();
    let level_relative = format!("{}.as", level_module.replace('.', "/"));
    let level_leaf = leaf_of(&level_relative).to_string();
    ensure!(
        !module_leaf.eq_ignore_ascii_case(&level_leaf),
        "NPC source {module_leaf} collides with level source {level_leaf}; choose another --id"
    );

    create_workspace(out)?;
    fs::write(out.join(&module_leaf), &source).with_context(|| format!("writing {module_leaf}"))?;
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
    let class_count = source
        .lines()
        .filter(|line| line.starts_with("class "))
        .count();
    println!("  {module_leaf}  the character, {class_count} classes");
    println!("  {level_leaf}  one added spawn line at {}", request.at);
    // Wo die Figur am Ende steht, ist nicht der Weltpunkt, sondern ihr Wegpunkt: der Tagesablauf
    // teleportiert sie dorthin. Und nur der Wegpunkt hat Koordinaten, die jemand ansteuern kann —
    // ein Weltpunkt steht in keinem Katalog. Ohne diese Zeile sucht man die eigene Figur.
    match (
        &request.waypoint,
        gore_catalog::location::LocationCatalog::bundled(),
    ) {
        // Bewusst zurueckhaltend. Eine fruehere Fassung sagte hier "it ends up at <waypoint>";
        // im Spiel nachgemessen stimmte das nicht — die Figur blieb an ihrem Weltpunkt, und
        // `location` und `spawnLocation` im Spielstand waren gleich. Der Tagesablauf uebersetzt
        // und wird getragen; dass er die Figur bewegt, hat noch kein Lauf gesehen.
        (Some(waypoint), Ok(spots)) => match spots.resolve(waypoint) {
            Some(spot) => println!(
                "  its routine schedules {waypoint} in {} — x {:.0}  y {:.0}  z {:.0}, but no run \
                 has yet seen a character move to its scheduled spot",
                spot.a, spot.x, spot.y, spot.z
            ),
            None => println!(
                "  WARNING: {waypoint} is not a known spot. The game ignores an unknown waypoint \
                 without a word, so the routine has nowhere to send anyone"
            ),
        },
        (None, _) => println!("  no --waypoint given, so it carries no routine"),
        (Some(_), Err(_)) => {}
    }
    // Wo eine Figur wirklich steht, weiss nur der Spielstand: Weltpunkte stehen in keinem
    // Katalog. `cargo run -p gore-save --example probe -- <save> private.npc.position "" id=<key>`
    // liest die Koordinate aus, sobald sie einmal erschienen ist.
    println!(
        "  it spawns at {}, whose position no catalog knows — read it from a save once it is there",
        request.at
    );
    println!("  {}", render::translation_line(&emitted, &level_module));
    if let Some(warning) = &occupied_warning {
        println!("  WARNING: {warning}");
    }
    if request.modular_visuals {
        println!(
            "  NOTE: --modular-visuals has no shipped precedent; check the comment in {module_leaf}"
        );
    }
    println!("next: {}", inputs.command("check", out));
    Ok(())
}

/// `gore npc delete` — eine ausgelieferte Figur nicht mehr setzen lassen.
fn suppress(npc: &str, cache: Option<PathBuf>, game: Option<PathBuf>, out: &Path) -> Result<()> {
    let spawn_class = generate::spawn_class(npc);
    let inputs = AuthoringInputs::resolve(cache, game)?;
    let emitted = emit_index(Some(inputs.cache.clone()), None, Some(&spawn_class))?;
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
    println!("next: {}", inputs.command("check", out));
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

fn validate_manifest_modules(manifest: &workspace::Manifest) -> Result<()> {
    ensure!(
        manifest.cache_sha256.len() == 64
            && manifest
                .cache_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "invalid NPC manifest cache_sha256: expected 64 lowercase hex characters"
    );
    let safe = |value: &str| {
        let path = Path::new(value);
        !value.contains(['\\', ':'])
            && value
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
            && path.components().next().is_some()
            && path
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)))
    };
    for edit in &manifest.modules {
        ensure!(
            safe(&edit.relative_path),
            "unsafe NPC module path: {}",
            edit.relative_path
        );
        ensure!(
            safe(&edit.source_file),
            "unsafe NPC source path: {}",
            edit.source_file
        );
        if let Some(pristine) = &edit.pristine_file {
            ensure!(safe(pristine), "unsafe NPC pristine path: {pristine}");
        }
    }

    let level = manifest
        .level_edit()
        .context("the manifest names no edited level script")?;
    if let Some(authored) = manifest.authored_module() {
        ensure!(
            !authored
                .source_file
                .eq_ignore_ascii_case(&level.source_file),
            "the NPC and level modules share a source filename: {}",
            level.source_file
        );
    }
    let level_path = format!("{}.as", manifest.level_module.replace('.', "/"));
    ensure!(
        level.op == "edit" && level.relative_path == level_path && level.pristine_file.is_some(),
        "the NPC level module entry does not match its generated path and edit operation"
    );
    match manifest.operation {
        workspace::Operation::New | workspace::Operation::Clone => {
            let authored = manifest
                .authored_module()
                .context("the manifest names no authored NPC module")?;
            ensure!(
                manifest.modules.len() == 2
                    && authored.module == generate::module_name(&manifest.npc_id)
                    && authored.relative_path == generate::relative_path(&manifest.npc_id)
                    && authored.pristine_file.is_none(),
                "a new NPC workspace must contain only its generated module and level edit"
            );
        }
        workspace::Operation::Checkout | workspace::Operation::Suppress => ensure!(
            manifest.modules.len() == 1,
            "a checkout or suppression must contain only its edited module"
        ),
    }
    Ok(())
}

/// `gore npc check` — das Arbeitsverzeichnis gegen den Vertrag prüfen.
fn check_workspace(dir: &Path, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let manifest = read_manifest(dir)?;
    validate_manifest_modules(&manifest)?;
    let inputs = AuthoringInputs::resolve(cache, game)?;
    routine::check_managed(dir, &manifest, inputs.game.clone())?;
    let spawn_class = generate::spawn_class(&manifest.npc_id);
    let path = inputs.cache.clone();
    let emitted = emit_index(Some(path.clone()), None, Some(&spawn_class))?;

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

    findings.extend(workspace_source_findings(dir, &manifest, &path)?.findings);

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
        println!("next: {}", inputs.command("stage", dir));
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
    println!("next: {}", inputs.command("stage", dir));
    Ok(())
}

/// Read the authored files again at staging time: a successful earlier `check` is not a lock.
struct WorkspaceSourceInspection {
    findings: Vec<check::Finding>,
    module_sources: BTreeMap<String, String>,
}

fn workspace_source_findings(
    dir: &Path,
    manifest: &workspace::Manifest,
    cache: &Path,
) -> Result<WorkspaceSourceInspection> {
    let spawn_class = generate::spawn_class(&manifest.npc_id);
    let mut findings = Vec::new();
    let mut module_sources = BTreeMap::new();
    let Some(level) = manifest.level_edit() else {
        bail!("the manifest names no edited level script");
    };
    let edited_path = dir.join(&level.source_file);
    let edited = fs::read_to_string(&edited_path)
        .with_context(|| format!("reading {}", edited_path.display()))?;
    module_sources.insert(level.source_file.clone(), edited.clone());
    let pristine_rel = level
        .pristine_file
        .as_deref()
        .unwrap_or("pristine/missing.as");
    let pristine_path = dir.join(pristine_rel);
    let pristine = fs::read_to_string(&pristine_path)
        .with_context(|| format!("reading {}", pristine_path.display()))?;
    let cached = emit_named_module(cache, &level.module)?;
    findings.extend(check::guard_pristine_source(
        cached.as_deref(),
        &pristine,
        &level.module,
    ));
    // Ein Checkout aendert Werte im eigenen Modul der Figur; ein Verfassen aendert Spawn-Zeilen
    // in einem fremden Levelskript. Zwei Absichten, zwei Waechter.
    match manifest.operation {
        workspace::Operation::Checkout => {
            let (definition, expected_module) = checkout_target_module(cache, &manifest.npc_id)?;
            if let Some(finding) =
                checkout_module_binding_finding(level, &definition, &expected_module)
            {
                findings.push(finding);
            }
            findings.extend(check::guard_checkout_diff(&pristine, &edited));
        }
        workspace::Operation::Suppress => {
            let emitted = emit_index(Some(cache.to_path_buf()), None, Some(&spawn_class))?;
            if let Some(finding) =
                suppression_scope_finding(&emitted, &spawn_class, &manifest.level_module)
            {
                findings.push(finding);
            }
            findings.extend(check::guard_suppressed_spawn(
                &pristine,
                &edited,
                &spawn_class,
            ));
        }
        workspace::Operation::New | workspace::Operation::Clone => {
            findings.extend(check::guard_level_diff(&pristine, &edited, &spawn_class));
            match manifest.world_points.as_slice() {
                [world_point] => findings.extend(check::guard_generated_spawn(
                    &pristine,
                    &edited,
                    world_point,
                    &spawn_class,
                )),
                _ => findings.push(check::Finding {
                    severity: check::Severity::Blocking,
                    message: "a new NPC workspace must name exactly one spawn world point"
                        .to_string(),
                }),
            }
        }
    }

    if let Some(authored) = manifest.authored_module() {
        let source_path = dir.join(&authored.source_file);
        let source = fs::read_to_string(&source_path)
            .with_context(|| format!("reading {}", source_path.display()))?;
        module_sources.insert(authored.source_file.clone(), source.clone());
        let from = manifest
            .derived_from
            .as_deref()
            .context("the authored NPC has no template id")?;
        let template_spawn = generate::spawn_class(from);
        let template = emit_index(Some(cache.to_path_buf()), None, Some(&template_spawn))?;
        let (_, spawn_defaults) = derivable_parent(
            &template.classes,
            &template.subclass_counts,
            &template_spawn,
        );
        let expected_actor = spawn_defaults
            .iter()
            .rev()
            .find_map(|line| line.strip_prefix("AIAgentCharacterClass = "))
            .with_context(|| format!("template {from} has no inherited actor blueprint"))?;
        findings.extend(check::guard_authored_module(
            &source,
            &manifest.npc_id,
            expected_actor,
        ));

        let modules = model::parse_modules(&read_module_cache(cache)?)
            .context("parsing modules for NPC class validation")?;
        let collisions = cache_class_collisions(&modules, &source);
        if !collisions.is_empty() {
            findings.push(check::Finding {
                severity: check::Severity::Blocking,
                message: format!(
                    "authored classes collide with the installed game: {}",
                    collisions.join(", ")
                ),
            });
        }

        let spots = gore_catalog::location::LocationCatalog::bundled()
            .context("reading the bundled location catalog")?;
        findings.extend(routine_waypoint_findings(&source, &spots));
    }

    Ok(WorkspaceSourceInspection {
        findings,
        module_sources,
    })
}

fn routine_waypoint_findings(
    source: &str,
    spots: &gore_catalog::location::LocationCatalog,
) -> Vec<check::Finding> {
    check::scheduled_waypoints(source)
        .into_iter()
        .filter(|waypoint| spots.resolve(waypoint).is_none())
        .map(|waypoint| check::Finding {
            severity: check::Severity::Blocking,
            message: format!(
                "the routine sends the character to {waypoint:?}, which is not a known spot. \
                 The game silently ignores unknown waypoints; choose a spot from `gore npc routine spots`"
            ),
        })
        .collect()
}

fn stage_path_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    let linked = metadata.file_type().is_symlink();
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt as _;
        linked || metadata.file_attributes() & 0x400 != 0
    };
    linked
}

fn validate_stage_output_target(path: &Path, kind: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                !stage_path_is_link_or_reparse(&metadata),
                "NPC stage {kind} output is a link or reparse point: {}",
                path.display()
            );
            ensure!(
                metadata.is_file(),
                "NPC stage {kind} output is not a regular file: {}",
                path.display()
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context(format!("reading {}", path.display())),
    }
    Ok(())
}

const STAGE_WORK_OWNER: &str = ".gore-npc-work-owner";

fn stage_work_owner_bytes(path: &Path) -> Result<Vec<u8>> {
    let root = fs::canonicalize(path.parent().context("NPC work directory has no parent")?)?;
    let work = root.join(path.file_name().context("NPC work directory has no name")?);
    Ok(format!("GORE NPC compiler scratch v1\n{}\n", work.display()).into_bytes())
}

/// Inspect before any stage writes: the compiler will recursively replace this scratch's tree.
/// Only an empty directory can be claimed; populated scratch must already carry our marker.
fn validate_stage_work_dir(path: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("reading NPC compiler work directory"),
    };
    ensure!(
        metadata.is_dir() && !stage_path_is_link_or_reparse(&metadata),
        "NPC compiler work directory is not a real directory: {}",
        path.display()
    );
    let owner = path.join(STAGE_WORK_OWNER);
    match fs::symlink_metadata(&owner) {
        Ok(metadata) => {
            let expected = stage_work_owner_bytes(path)?;
            ensure!(
                metadata.is_file()
                    && !stage_path_is_link_or_reparse(&metadata)
                    && metadata.len() == expected.len() as u64
                    && fs::read(&owner)? == expected,
                "NPC compiler work directory has an invalid ownership marker: {}",
                path.display()
            );
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ensure!(
                fs::read_dir(path)?.next().transpose()?.is_none(),
                "refusing unowned nonempty NPC compiler work directory: {}. Choose a different \
                 workspace or move that directory aside; its contents have been preserved",
                path.display()
            );
            Ok(false)
        }
        Err(error) => Err(error).context("reading NPC compiler work ownership"),
    }
}

fn prepare_stage_work_dir(path: &Path) -> Result<()> {
    validate_stage_work_dir(path)?;
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error).with_context(|| format!("creating {}", path.display())),
    }
    if !validate_stage_work_dir(path)? {
        let owner = path.join(STAGE_WORK_OWNER);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&owner)
            .with_context(|| format!("claiming NPC compiler scratch {}", path.display()))?;
        file.write_all(&stage_work_owner_bytes(path)?)?;
        file.sync_all()?;
    }
    Ok(())
}

fn write_stage_output_atomic(path: &Path, bytes: &[u8], kind: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("NPC stage {kind} output has no parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("creating temporary {kind} in {}", parent.display()))?;
    temporary
        .write_all(bytes)
        .with_context(|| format!("writing temporary {kind} for {}", path.display()))?;
    temporary
        .flush()
        .with_context(|| format!("flushing temporary {kind} for {}", path.display()))?;
    temporary.persist(path).map_err(|error| {
        anyhow::anyhow!(
            "publishing NPC stage {kind} {}: {}",
            path.display(),
            error.error
        )
    })?;
    Ok(())
}

fn write_staged_source(dir: &Path, source: &str) -> Result<()> {
    let snapshot = dir.join(stage::STAGED_SOURCE_NAME);
    validate_stage_output_target(&snapshot, "source")?;
    write_stage_output_atomic(&snapshot, source.as_bytes(), "source")
}

fn validate_stage_directory(path: &Path) -> Result<()> {
    for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
        let metadata = fs::symlink_metadata(ancestor)
            .with_context(|| format!("reading NPC staging directory {}", ancestor.display()))?;
        ensure!(
            metadata.is_dir() && !stage_path_is_link_or_reparse(&metadata),
            "NPC staging directory is not a real directory: {}",
            ancestor.display()
        );
    }
    Ok(())
}

/// Copy only the inspected source bytes into a fresh, private snapshot. Each run owns a
/// distinct directory; neither prior snapshots nor workspace edits can leak into this one.
/// The caller publishes its commands only after every source has been written successfully.
fn stage_sparse_snapshot(
    dir: &Path,
    manifest: &workspace::Manifest,
    module_sources: &BTreeMap<String, String>,
) -> Result<tempfile::TempDir> {
    validate_manifest_modules(manifest)?;
    validate_stage_directory(dir)?;
    let workspace = fs::canonicalize(dir)
        .with_context(|| format!("resolving NPC workspace {}", dir.display()))?;
    let snapshot = tempfile::Builder::new()
        .prefix(".gore-npc-overlays-")
        .tempdir_in(&workspace)
        .context("creating NPC source snapshot")?;
    for edit in &manifest.modules {
        let source = module_sources.get(&edit.source_file).with_context(|| {
            format!(
                "the validated NPC source snapshot is missing {}",
                edit.source_file
            )
        })?;
        let target = snapshot.path().join(&edit.relative_path);
        let mut parent = snapshot.path().to_path_buf();
        let relative = Path::new(&edit.relative_path);
        for part in relative.parent().into_iter().flat_map(Path::components) {
            parent.push(part.as_os_str());
            match fs::create_dir(&parent) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error).context("creating NPC snapshot directory"),
            }
            validate_stage_directory(&parent)?;
        }
        validate_stage_directory(&parent)?;
        // create_new refuses any pre-existing file, symlink, or hard link. The snapshot
        // never shares source-file identities with the authored workspace or another run.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .with_context(|| format!("creating NPC snapshot source {}", target.display()))?;
        file.write_all(source.as_bytes())
            .with_context(|| format!("writing NPC snapshot source {}", target.display()))?;
        file.sync_all()?;
    }
    Ok(snapshot)
}

/// `gore npc stage` — Quellen sichern, Spec schreiben, Bau-Kommandos drucken.
fn stage_workspace(
    dir: &Path,
    tree: Option<&Path>,
    mod_name: &str,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<()> {
    ensure!(
        tree.is_none(),
        "--tree is obsolete for `gore npc stage`; omit it. Staging now snapshots only the \
         authored modules for `gore as compile --overlays`, without exporting a full tree"
    );
    gore_mod::validate_mod_name(mod_name).context("invalid --mod-name")?;
    validate_stage_directory(dir)?;
    let spec_path = dir.join("spec.json");
    validate_stage_output_target(&spec_path, "spec")?;
    // Keep the suffix used by both printed commands and MCP's derived write classification.
    let command_dir = fs::canonicalize(dir)
        .with_context(|| format!("resolving NPC stage workspace {}", dir.display()))?;
    let work = stage::work_dir(&command_dir);
    validate_stage_work_dir(&work)?;
    let manifest = read_manifest(dir)?;
    validate_manifest_modules(&manifest)?;
    let inputs = AuthoringInputs::resolve(cache, game)?;
    routine::check_managed(dir, &manifest, inputs.game.clone())?;
    let path = &inputs.cache;
    let game = Some(stage::compiler_game_for(&manifest, path, inputs.game)?);
    let inspection = workspace_source_findings(dir, &manifest, &path)?;
    let blocking: Vec<_> = inspection
        .findings
        .iter()
        .filter(|finding| finding.severity == check::Severity::Blocking)
        .collect();
    if !blocking.is_empty() {
        for finding in blocking {
            eprintln!("  [blocking] {}", finding.message);
        }
        bail!("workspace source failed the NPC guards; run `gore npc check` for details");
    }
    let route = stage::route_of(&manifest);
    prepare_stage_work_dir(&work)?;

    let staged_overlay = match route {
        stage::Route::Overlays => Some(stage_sparse_snapshot(
            dir,
            &manifest,
            &inspection.module_sources,
        )?),
        stage::Route::SingleModule => {
            let level = manifest
                .level_edit()
                .expect("single-module route has a level edit");
            let source = inspection
                .module_sources
                .get(&level.source_file)
                .with_context(|| {
                    format!(
                        "the validated NPC source snapshot is missing {}",
                        level.source_file
                    )
                })?;
            write_staged_source(dir, source)?;
            None
        }
    };
    let source_root = staged_overlay
        .as_ref()
        .map(|snapshot| snapshot.path().display().to_string())
        .unwrap_or_default();

    let spec = stage::spec_json(&manifest, mod_name);
    write_stage_output_atomic(
        &spec_path,
        format!("{}\n", serde_json::to_string_pretty(&spec)?).as_bytes(),
        "spec",
    )
    .with_context(|| format!("writing {}", spec_path.display()))?;

    let game_arg = game.as_ref().map(|path| path.display().to_string());
    let commands = stage::build_commands(
        &manifest,
        &inspection.module_sources,
        &command_dir.display().to_string(),
        &source_root,
        mod_name,
        game_arg.as_deref(),
    )?;

    if let Some(snapshot) = staged_overlay {
        let path = snapshot.keep();
        println!(
            "staged {} authored modules in {} (unchanged modules stay in the base cache)",
            manifest.modules.len(),
            path.display()
        );
    }
    println!("wrote {}", spec_path.display());
    println!("now run:");
    for command in &commands {
        println!("  {command}");
    }
    let deploy_game_arg = game_arg
        .as_deref()
        .map(|path| format!(" --game {}", stage::shell_quote(path)))
        .unwrap_or_default();
    println!(
        "then: gore mod deploy --bundle {}{}",
        stage::shell_quote(
            &command_dir
                .join("build")
                .join(mod_name)
                .display()
                .to_string()
        ),
        deploy_game_arg
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
    let document = display_name_edits(id, name, english);
    let content = serde_json::to_string_pretty(&document)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .with_context(|| format!("creating new NPC text file {}", out.display()))?;
    writeln!(file, "{content}").with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {}", out.display());
    println!("  {} -> {name:?} in both German columns", id.to_lowercase());
    println!("next: {}", display_name_import_command(out));
    Ok(())
}

fn display_name_import_command(out: &Path) -> String {
    // This helper has no selected installation. Require a deliberate input and write a sibling
    // artifact instead of falling back to an in-place import into the configured game.
    let mut artifact = out.with_extension("lcache");
    if out
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lcache"))
    {
        let mut name = out.as_os_str().to_os_string();
        name.push(".patched.lcache");
        artifact = PathBuf::from(name);
    }
    format!(
        "gore loc import --lcache {} --out {} --edits {} --add-missing",
        stage::shell_quote("<path/to/chosen-input.lcache>"),
        stage::shell_quote(&artifact.display().to_string()),
        stage::shell_quote(&out.display().to_string()),
    )
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
    let inputs = AuthoringInputs::resolve(cache, game)?;
    let path = inputs.cache.clone();
    let emitted = emit_index(Some(path.clone()), None, Some(&spawn_class))?;
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
    println!("  edit the values; keep every existing default target, class name and parent");
    println!("next: {}", inputs.command("check", out));
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

fn is_derivable_base(subclass_counts: &BTreeMap<String, usize>, class_name: &str) -> bool {
    subclass_counts.get(class_name).copied().unwrap_or(0) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_output_never_follows_a_dangling_link() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("outside");
        let output = temp.path().join("workspace");
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &output);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&target, &output);
        if let Err(error) = link_result {
            eprintln!("skip: this account cannot create a directory symlink: {error}");
            return;
        }

        assert!(create_workspace(&output).is_err());
        assert!(!target.exists());
        assert!(fs::symlink_metadata(&output).is_ok());

        let nested = temp.path().join("missing").join("workspace");
        create_workspace(&nested).unwrap();
        assert!(nested.join("pristine").is_dir());
    }

    #[test]
    fn npc_text_output_never_follows_a_dangling_link() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("outside.json");
        let output = temp.path().join("name.json");
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &output);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_file(&target, &output);
        if let Err(error) = link_result {
            eprintln!("skip: this account cannot create a file symlink: {error}");
            return;
        }

        assert!(write_display_name("TEST", "Name", None, &output).is_err());
        assert!(!target.exists());
        assert!(fs::symlink_metadata(&output).is_ok());
    }

    #[test]
    fn authored_npc_collision_check_covers_every_declared_class() {
        let source = "class UCharacterDefinition_Human_MINE : UBase\n{}\n\
                      class UCharacterVisualsDefinition_Human_MINE : UBase\n{}\n\
                      class UAIAgentConfig_Human_MINE : UBase\n{}\n\
                      class USpawnAIAgentDefinition_MINE : UBase\n{}\n\
                      class UConversationCharacterSettings_Ambient_MINE : UBase\n{}\n\
                      class UDailyRoutine_MINE_Start : UBase\n{}\n\
                      class UTraderConfig_MINE : UBase\n{}\n";
        let shipped = [
            "UTraderConfig_MINE",
            "UCharacterDefinition_Human_MINE",
            "UConversationCharacterSettings_Ambient_MINE",
            "UDailyRoutine_MINE_Start",
            "UCharacterVisualsDefinition_Human_MINE",
            "UAIAgentConfig_Human_MINE",
            "USpawnAIAgentDefinition_MINE",
        ];
        let mut expected = shipped.map(str::to_string);
        expected.sort();
        assert_eq!(colliding_class_names(shipped.into_iter(), source), expected);
        assert!(colliding_class_names(
            ["UTraderConfig_MINE"].into_iter(),
            "class USpawnAIAgentDefinition_MINE : UBase\n{}\n"
        )
        .is_empty());
    }

    #[test]
    fn authored_npc_collision_check_is_case_insensitive() {
        assert_eq!(
            colliding_class_names(
                ["UCharacterDefinition_Human_OC_STT_Diego"].into_iter(),
                "class UCharacterDefinition_Human_oc_stt_diego : UBase\n{}\n"
            ),
            vec!["UCharacterDefinition_Human_oc_stt_diego".to_string()]
        );
    }

    #[test]
    fn only_non_final_character_definition_classes_are_derivable_guild_bases() {
        let counts = BTreeMap::from([
            ("UCharacterDefinition_Human_OldCamp_Guard".to_string(), 4),
            ("UCharacterDefinition_Human_OC_STT_Diego".to_string(), 0),
        ]);
        assert!(is_derivable_base(
            &counts,
            "UCharacterDefinition_Human_OldCamp_Guard"
        ));
        assert!(!is_derivable_base(
            &counts,
            "UCharacterDefinition_Human_OC_STT_Diego"
        ));
    }

    #[test]
    fn non_human_spawn_chain_is_not_a_new_npc_template() {
        let source = r#"class USpawnAIAgentDefinition_Creature_Molerat : USpawnAIAgentDefinition
{
    default AIAgentConfigClass = UAIAgentConfig_Creature_Molerat::StaticClass();
}
class UAIAgentConfig_Creature_Molerat : UAIAgentConfig
{
    default m_CharacterDefinition = UCharacterDefinition_Creature_Molerat::StaticClass();
}
class UCharacterDefinition_Creature_Molerat : UCharacterDefinition
{
}
"#;
        let classes = defaults::parse_classes(source)
            .into_iter()
            .map(|class| (class.name.clone(), class))
            .collect();
        let error = ensure_human_template(&classes, "Creature_Molerat").unwrap_err();
        assert!(error
            .to_string()
            .contains("not a supported human NPC template"));

        let human_source = source
            .replace(
                "UAIAgentConfig_Creature_Molerat",
                "UAIAgentConfig_Human_Creature_Molerat",
            )
            .replace(
                "UCharacterDefinition_Creature_Molerat",
                "UCharacterDefinition_Human_Creature_Molerat",
            );
        let human_classes = defaults::parse_classes(&human_source)
            .into_iter()
            .map(|class| (class.name.clone(), class))
            .collect();
        assert!(ensure_human_template(&human_classes, "Creature_Molerat").is_ok());
    }

    #[test]
    fn author_rejects_waypoints_that_cannot_be_embedded_in_a_name_literal() {
        for waypoint in ["FP_BAD\"", "FP_BAD\\PATH", "FP_BAD\nINJECTED"] {
            let tmp = tempfile::TempDir::new().unwrap();
            let out = tmp.path().join("workspace");
            let error = author(
                &NewRequest {
                    id: "MY_NPC".to_string(),
                    from: "OC_STT_Diego".to_string(),
                    guild: None,
                    at: "UWP_TEST".to_string(),
                    waypoint: Some(waypoint.to_string()),
                    trader: false,
                    modular_visuals: false,
                },
                None,
                None,
                &out,
            )
            .unwrap_err();
            assert!(error.to_string().contains("safe ASCII text"));
            assert!(!out.exists());
        }
    }

    #[test]
    fn authored_routine_unknown_waypoint_blocks_staging() {
        let spots = gore_catalog::location::LocationCatalog::bundled().unwrap();
        let source = "class UDailyRoutine_Test : UAIState_DailyRoutine_Human\n{\n    default Schedule(0, 0, UAIState_Stand(), n\"NO_SUCH_ROUTINE_SPOT\", 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);\n}\n";
        let findings = routine_waypoint_findings(source, &spots);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, check::Severity::Blocking);
        assert!(findings[0].message.contains("NO_SUCH_ROUTINE_SPOT"));
        assert!(routine_waypoint_findings(
            &source.replace("NO_SUCH_ROUTINE_SPOT", "FP_XT_WAIT_OUTSIDE"),
            &spots
        )
        .is_empty());
    }

    #[test]
    fn manifest_rejects_extra_modules_and_escape_paths() {
        let level = workspace::ModuleEdit {
            module: "LevelScripts.Test".to_string(),
            relative_path: "LevelScripts/Test.as".to_string(),
            source_file: "Test.as".to_string(),
            pristine_file: Some("pristine/Test.as".to_string()),
            op: "edit".to_string(),
        };
        let authored = workspace::ModuleEdit {
            module: generate::module_name("MINE"),
            relative_path: generate::relative_path("MINE"),
            source_file: "MINE.as".to_string(),
            pristine_file: None,
            op: "add".to_string(),
        };
        let mut manifest = workspace::Manifest {
            operation: workspace::Operation::New,
            npc_id: "MINE".to_string(),
            derived_from: None,
            modules: vec![authored, level],
            world_points: vec!["UWP_A".to_string()],
            level_module: "LevelScripts.Test".to_string(),
            cache_sha256: "a".repeat(64),
            modular_visuals: false,
        };
        assert!(validate_manifest_modules(&manifest).is_ok());
        manifest.cache_sha256 = "abc".to_string();
        assert!(validate_manifest_modules(&manifest).is_err());
        manifest.cache_sha256 = "a".repeat(64);
        manifest.modules.push(manifest.modules[1].clone());
        assert!(validate_manifest_modules(&manifest).is_err());
        manifest.modules.pop();
        manifest.modules[1].relative_path = "../outside.as".to_string();
        assert!(validate_manifest_modules(&manifest).is_err());
        manifest.modules[1].relative_path = "LevelScripts/Test.as".to_string();
        manifest.modules[0].source_file = "TEST.as".to_string();
        let error = validate_manifest_modules(&manifest).unwrap_err();
        assert!(error.to_string().contains("share a source filename"));
    }

    #[test]
    fn checkout_binding_accepts_only_the_resolved_definition_module() {
        let edit = workspace::ModuleEdit {
            module: "AI.AIAgent.Human.Config.OC_STT_Diego.OC_STT_Diego".to_string(),
            relative_path: "AI/AIAgent/Human/Config/OC_STT_Diego/OC_STT_Diego.as".to_string(),
            source_file: "OC_STT_Diego.as".to_string(),
            pristine_file: Some("pristine/OC_STT_Diego.as".to_string()),
            op: "edit".to_string(),
        };
        let definition = "UCharacterDefinition_Human_OC_STT_Diego";
        let expected = "AI.AIAgent.Human.Config.OC_STT_Diego.OC_STT_Diego";

        assert!(checkout_module_binding_finding(&edit, definition, expected).is_none());
        let mut retargeted = edit;
        retargeted.module = "AI.AIAgent.Human.Config.OTHER.OTHER".to_string();
        let finding = checkout_module_binding_finding(&retargeted, definition, expected)
            .expect("retargeting must be blocked");
        assert_eq!(finding.severity, check::Severity::Blocking);
        assert!(finding.message.contains("OC_STT_Diego"));
        assert!(finding.message.contains("OTHER"));
    }

    fn sparse_stage_fixture() -> (workspace::Manifest, BTreeMap<String, String>) {
        let authored = workspace::ModuleEdit {
            module: generate::module_name("MINE"),
            relative_path: generate::relative_path("MINE"),
            source_file: "MINE.as".to_string(),
            pristine_file: None,
            op: "add".to_string(),
        };
        let level = workspace::ModuleEdit {
            module: "LevelScripts.Test".to_string(),
            relative_path: "LevelScripts/Test.as".to_string(),
            source_file: "Test.as".to_string(),
            pristine_file: Some("pristine/Test.as".to_string()),
            op: "edit".to_string(),
        };
        let manifest = workspace::Manifest {
            operation: workspace::Operation::New,
            npc_id: "MINE".to_string(),
            derived_from: Some("OC_STT_Diego".to_string()),
            modules: vec![authored, level],
            world_points: vec!["UWP_A".to_string()],
            level_module: "LevelScripts.Test".to_string(),
            cache_sha256: "a".repeat(64),
            modular_visuals: false,
        };
        let snapshots = BTreeMap::from([
            (
                "MINE.as".to_string(),
                "validated authored source".to_string(),
            ),
            ("Test.as".to_string(), "validated level source".to_string()),
        ]);

        (manifest, snapshots)
    }

    #[test]
    fn npc_authoring_and_staging_use_the_owned_pristine_base_without_writing() {
        use sha2::{Digest, Sha256};
        let temp = tempfile::tempdir().unwrap();
        let game = fs::canonicalize(temp.path()).unwrap();
        let live = gore_mod::resolve_game_paths(&game).script_cache;
        let backup = live.with_extension("Cache.gore-bak");
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        let pristine = b"original cache";
        let deployed = b"installed script mod";
        fs::write(&live, deployed).unwrap();
        fs::write(&backup, pristine).unwrap();
        let identity = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
        let record = gore_mod::DeployRecord {
            mod_name: "NpcStageFixture".into(),
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: [(live.display().to_string(), identity(deployed))].into(),
            backup_hashes: [(backup.display().to_string(), identity(pristine))].into(),
            ..Default::default()
        };
        let record_path = gore_mod::deploy_record_path(&game);
        let record_bytes = serde_json::to_vec(&record).unwrap();
        fs::write(&record_path, &record_bytes).unwrap();
        let (mut manifest, _) = sparse_stage_fixture();
        manifest.cache_sha256 = format!("{:x}", Sha256::digest(pristine));

        let selected = authoring_cache_path(None, Some(game.clone())).unwrap();
        assert_eq!(selected, backup);
        assert_eq!(cache_path(None, Some(game.clone())).unwrap(), live);
        assert_eq!(
            stage::compiler_game_for(&manifest, &selected, Some(game.clone())).unwrap(),
            game
        );
        // An explicit selection of the installed mod must still fail the exact base seal.
        assert!(stage::compiler_game_for(&manifest, &live, Some(game.clone())).is_err());
        assert_eq!(fs::read(&live).unwrap(), deployed);
        assert_eq!(fs::read(&backup).unwrap(), pristine);
        assert_eq!(fs::read(&record_path).unwrap(), record_bytes);
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn npc_handoffs_keep_the_resolved_install_and_explicit_cache_selection() {
        use sha2::{Digest, Sha256};
        let temp = tempfile::tempdir().unwrap();
        let selected_game = temp.path().join("chosen game 'one'");
        let other_game = temp.path().join("other game");
        let selected_cache = temp.path().join("selected cache 'copy'.Cache");
        let bytes = b"same cache in different installations";
        for game in [&selected_game, &other_game] {
            let live = gore_mod::resolve_game_paths(game).script_cache;
            fs::create_dir_all(live.parent().unwrap()).unwrap();
            fs::write(&live, bytes).unwrap();
        }
        fs::write(&selected_cache, bytes).unwrap();
        // Isolate configuration in a child process; never redirect this test runner's settings.
        if std::env::var_os("GORE_NPC_HANDOFF_TEST_CHILD").is_none() {
            let settings = temp.path().join("settings");
            gore_loc::config::save_to(
                &settings.join("gore/config.json"),
                &gore_loc::config::Config {
                    game_path: Some(other_game.display().to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "cmd::npc::tests::npc_handoffs_keep_the_resolved_install_and_explicit_cache_selection",
                    "--nocapture",
                ])
                .env("GORE_NPC_HANDOFF_TEST_CHILD", "1")
                .env("XDG_DATA_HOME", &settings)
                .env("LOCALAPPDATA", &settings)
                .env("APPDATA", &settings)
                .env("GORE_DISABLE_GAME_AUTODETECT", "1")
                .output()
                .unwrap();
            assert!(
                child.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&child.stdout),
                String::from_utf8_lossy(&child.stderr)
            );
            return;
        }
        let configured_game = gore_loc::config::game_root(None).unwrap();
        assert_eq!(
            fs::read(gore_mod::resolve_game_paths(&configured_game).script_cache).unwrap(),
            bytes
        );
        let inputs = AuthoringInputs::resolve(
            Some(selected_cache.clone()),
            Some(selected_game.join("G1R/Script")),
        )
        .unwrap();
        assert_eq!(inputs.cache, fs::canonicalize(&selected_cache).unwrap());
        assert_eq!(inputs.game, Some(fs::canonicalize(&selected_game).unwrap()));
        assert_ne!(
            inputs.game,
            Some(fs::canonicalize(&configured_game).unwrap())
        );
        let (mut manifest, _) = sparse_stage_fixture();
        manifest.cache_sha256 = faithfulness::cache_seal(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            stage::compiler_game_for(&manifest, &inputs.cache, inputs.game.clone()).unwrap(),
            fs::canonicalize(&selected_game).unwrap()
        );
        let workspace = temp.path().join("my NPC 'workspace'");
        for action in ["check", "stage"] {
            let command = inputs.command(action, &workspace);
            assert!(command.starts_with(&format!(
                "gore npc {action} {} --cache {}",
                stage::shell_quote(&workspace.display().to_string()),
                stage::shell_quote(&inputs.cache.display().to_string()),
            )));
            assert!(command.ends_with(&format!(
                " --game {}",
                stage::shell_quote(&inputs.game.as_ref().unwrap().display().to_string())
            )));
            assert!(!command.contains(&other_game.display().to_string()));
            assert!(!command.contains(&configured_game.display().to_string()));
        }
        assert_eq!(fs::read(&selected_cache).unwrap(), bytes);
        for game in [&selected_game, &other_game] {
            assert_eq!(
                fs::read(gore_mod::resolve_game_paths(game).script_cache).unwrap(),
                bytes
            );
            assert!(!game.join(".gore-install-mutation.lock").exists());
        }

        let default_inputs = AuthoringInputs::resolve(None, None).unwrap();
        assert_eq!(
            default_inputs.game,
            Some(fs::canonicalize(&configured_game).unwrap())
        );
        for action in ["check", "stage"] {
            let command = default_inputs.command(action, &workspace);
            assert!(!command.contains(" --cache "));
            assert!(command.ends_with(&format!(
                " --game {}",
                stage::shell_quote(&default_inputs.game.as_ref().unwrap().display().to_string())
            )));
        }

        // Model an owned deployed cache and its pristine backup in this synthetic installation.
        let game = inputs.game.as_ref().unwrap();
        let live = gore_mod::resolve_game_paths(game).script_cache;
        let backup = live.with_extension("Cache.gore-bak");
        let deployed = b"installed NPC mod";
        fs::write(&live, deployed).unwrap();
        fs::write(&backup, bytes).unwrap();
        let identity = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
        let record = gore_mod::DeployRecord {
            mod_name: "NpcHandoffFixture".into(),
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: [(live.display().to_string(), identity(deployed))].into(),
            backup_hashes: [(backup.display().to_string(), identity(bytes))].into(),
            ..Default::default()
        };
        let record_path = gore_mod::deploy_record_path(game);
        fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();

        let game_only = AuthoringInputs::resolve(None, inputs.game.clone()).unwrap();
        assert_eq!(game_only.cache, fs::canonicalize(&backup).unwrap());
        let commands: Vec<_> = ["check", "stage"]
            .map(|action| game_only.command(action, &workspace))
            .into();
        for command in &commands {
            assert!(!command.contains(" --cache "));
            assert!(!command.contains(".gore-bak"));
            assert!(command.ends_with(&format!(
                " --game {}",
                stage::shell_quote(&game.display().to_string())
            )));
        }
        assert_eq!(
            stage::compiler_game_for(&manifest, &game_only.cache, game_only.game.clone()).unwrap(),
            *game
        );

        // Restore the synthetic install and remove ownership just as undeploy does. Following
        // either game-only hint now reselects the live pristine cache with the same base seal.
        fs::write(&live, fs::read(&backup).unwrap()).unwrap();
        fs::remove_file(&backup).unwrap();
        fs::remove_file(&record_path).unwrap();
        let resumed = AuthoringInputs::resolve(None, game_only.game.clone()).unwrap();
        assert_eq!(resumed.cache, fs::canonicalize(&live).unwrap());
        for (action, command) in ["check", "stage"].into_iter().zip(commands) {
            assert_eq!(resumed.command(action, &workspace), command);
        }
        assert_eq!(
            stage::compiler_game_for(&manifest, &resumed.cache, resumed.game.clone()).unwrap(),
            *game
        );
        assert_eq!(fs::read(&live).unwrap(), bytes);
        assert_eq!(fs::read(&selected_cache).unwrap(), bytes);
        assert!(!game.join(".gore-install-mutation.lock").exists());

        let offline = AuthoringInputs::resolve(Some(selected_cache), None).unwrap();
        assert!(offline.game.is_none());
        assert!(!offline.command("check", &workspace).contains(" --game "));
        assert!(offline.command("stage", &workspace).contains(" --cache "));
    }

    #[test]
    fn sparse_staging_copies_only_validated_sources_and_binds_their_hashes() {
        use sha2::{Digest, Sha256};
        let temp = tempfile::tempdir().unwrap();
        let (manifest, sources) = sparse_stage_fixture();
        fs::write(temp.path().join("MINE.as"), "changed after validation").unwrap();
        fs::write(temp.path().join("Test.as"), "changed after validation").unwrap();
        fs::write(temp.path().join("Unrelated.as"), "not an overlay").unwrap();
        fs::create_dir(temp.path().join("pristine")).unwrap();
        fs::write(temp.path().join("pristine/Test.as"), "original source").unwrap();

        let staged = stage_sparse_snapshot(&temp.path().join("."), &manifest, &sources).unwrap();
        let commands = stage::build_commands(
            &manifest,
            &sources,
            &fs::canonicalize(temp.path()).unwrap().display().to_string(),
            &staged.path().display().to_string(),
            "TestMod",
            None,
        )
        .unwrap();
        assert!(commands[0].contains(" --overlays "));
        assert!(commands[0].contains(&format!("--expect-base-sha256 '{}'", manifest.cache_sha256)));
        for edit in &manifest.modules {
            let actual = fs::read(staged.path().join(&edit.relative_path)).unwrap();
            assert_eq!(actual, sources[&edit.source_file].as_bytes());
            let digest = format!("{:x}", Sha256::digest(&actual));
            assert!(commands[0].contains(&stage::shell_quote(&format!(
                "{}:{}:{}:{digest}",
                edit.op, edit.module, edit.relative_path
            ))));
        }
        assert_eq!(fs::read_dir(staged.path()).unwrap().count(), 2);
        assert!(!staged.path().join("Unrelated.as").exists());
        assert!(!staged.path().join("pristine").exists());
        assert!(!staged.path().join(workspace::MANIFEST_NAME).exists());
    }

    #[test]
    fn repeated_sparse_staging_never_reuses_or_clobbers_an_existing_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let (manifest, mut sources) = sparse_stage_fixture();
        let legacy = temp.path().join(".gore-npc-staged-tree");
        fs::create_dir(&legacy).unwrap();
        fs::write(legacy.join("keep.as"), "pre-existing work").unwrap();
        let first = stage_sparse_snapshot(temp.path(), &manifest, &sources)
            .unwrap()
            .keep();
        fs::write(first.join("Unrelated.as"), "old contamination").unwrap();
        sources.insert("MINE.as".to_string(), "second authored source".to_string());
        let second = stage_sparse_snapshot(temp.path(), &manifest, &sources).unwrap();
        assert_ne!(first, second.path());
        assert_eq!(
            fs::read_to_string(first.join(generate::relative_path("MINE"))).unwrap(),
            "validated authored source"
        );
        assert_eq!(
            fs::read_to_string(second.path().join(generate::relative_path("MINE"))).unwrap(),
            "second authored source"
        );
        assert!(!second.path().join("Unrelated.as").exists());
        assert_eq!(
            fs::read_to_string(legacy.join("keep.as")).unwrap(),
            "pre-existing work"
        );
    }

    #[test]
    fn separate_workspaces_and_hard_linked_sources_have_independent_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let (manifest, sources) = sparse_stage_fixture();
        let first_workspace = temp.path().join("first");
        let second_workspace = temp.path().join("second");
        fs::create_dir(&first_workspace).unwrap();
        fs::create_dir(&second_workspace).unwrap();
        let source = first_workspace.join("MINE.as");
        fs::write(&source, &sources["MINE.as"]).unwrap();
        fs::hard_link(&source, second_workspace.join("MINE.as")).unwrap();
        let first = stage_sparse_snapshot(&first_workspace, &manifest, &sources).unwrap();
        let second = stage_sparse_snapshot(&second_workspace, &manifest, &sources).unwrap();
        fs::write(&source, "changed workspace source").unwrap();
        fs::write(
            first.path().join(generate::relative_path("MINE")),
            "changed snapshot",
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(second.path().join(generate::relative_path("MINE"))).unwrap(),
            "validated authored source"
        );
        assert_eq!(
            fs::read_to_string(second_workspace.join("MINE.as")).unwrap(),
            "changed workspace source"
        );
    }

    #[test]
    fn incomplete_or_unsafe_sparse_sources_leave_no_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let (mut manifest, mut sources) = sparse_stage_fixture();
        // The first source was already copied when the absent second source is detected.
        sources.remove("Test.as");
        let error = stage_sparse_snapshot(temp.path(), &manifest, &sources).unwrap_err();
        assert!(error.to_string().contains("missing Test.as"));
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
        manifest.modules[0].relative_path = "../outside.as".to_string();
        let error = stage_sparse_snapshot(temp.path(), &manifest, &sources).unwrap_err();
        assert!(error.to_string().contains("unsafe NPC module path"));
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[test]
    fn sparse_staging_rejects_a_linked_workspace_without_touching_its_target() {
        let temp = tempfile::tempdir().unwrap();
        let (manifest, sources) = sparse_stage_fixture();
        let outside = temp.path().join("outside");
        let link = temp.path().join("link");
        fs::create_dir(&outside).unwrap();
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&outside, &link);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&outside, &link);
        if let Err(error) = link_result {
            eprintln!("skip: this account cannot create a directory symlink: {error}");
            return;
        }
        let error = stage_sparse_snapshot(&link, &manifest, &sources).unwrap_err();
        assert!(error.to_string().contains("not a real directory"));
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }

    #[test]
    fn stage_spec_replaces_a_hard_link_without_touching_its_peer() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside.json");
        let spec = temp.path().join("spec.json");
        fs::write(&outside, b"keep this file").unwrap();
        fs::hard_link(&outside, &spec).unwrap();

        validate_stage_output_target(&spec, "spec").unwrap();
        write_stage_output_atomic(&spec, b"new spec", "spec").unwrap();

        assert_eq!(fs::read(&spec).unwrap(), b"new spec");
        assert_eq!(fs::read(&outside).unwrap(), b"keep this file");
    }

    #[test]
    fn staged_single_module_source_survives_later_workspace_edits() {
        let temp = tempfile::tempdir().unwrap();
        let workspace_source = temp.path().join("Level.as");
        fs::write(&workspace_source, "validated source").unwrap();
        let validated = fs::read_to_string(&workspace_source).unwrap();
        write_staged_source(temp.path(), &validated).unwrap();

        fs::write(&workspace_source, "changed after stage").unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join(stage::STAGED_SOURCE_NAME)).unwrap(),
            "validated source"
        );
    }

    #[test]
    fn stage_rejects_a_linked_spec_before_workspace_work() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside.json");
        let spec = temp.path().join("spec.json");
        fs::write(&outside, b"keep this file").unwrap();
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&outside, &spec);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_file(&outside, &spec);
        if let Err(error) = link_result {
            eprintln!("skip: this account cannot create a file symlink: {error}");
            return;
        }

        let error = stage_workspace(temp.path(), None, "TestMod", None, None).unwrap_err();
        assert!(
            error.to_string().contains("link or reparse point"),
            "{error:#}"
        );
        assert_eq!(fs::read(&outside).unwrap(), b"keep this file");
    }

    #[test]
    fn stage_rejects_a_linked_compiler_work_directory() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let outside = temp.path().join("outside");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&outside).unwrap();
        let work = stage::work_dir(&fs::canonicalize(&workspace).unwrap());
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&outside, &work);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&outside, &work);
        if let Err(error) = link_result {
            eprintln!("skip: this account cannot create a directory symlink: {error}");
            return;
        }

        let error = prepare_stage_work_dir(&work).unwrap_err();
        assert!(
            error.to_string().contains("not a real directory"),
            "{error:#}"
        );
        assert!(outside.is_dir());
    }

    #[test]
    fn stage_rejects_unowned_scratch_before_touching_snapshots_or_specs() {
        let temp = tempfile::tempdir().unwrap();
        for operation in [workspace::Operation::New, workspace::Operation::Checkout] {
            let workspace = temp.path().join(format!("{operation:?}"));
            fs::create_dir(&workspace).unwrap();
            let work = stage::work_dir(&workspace);
            fs::create_dir_all(work.join("tree")).unwrap();
            let sentinel = work.join("tree/hand-authored.as");
            let sentinel_bytes = b"\x00unowned scratch\r\n\xff";
            fs::write(&sentinel, sentinel_bytes).unwrap();
            let spec = workspace.join("spec.json");
            fs::write(&spec, b"existing spec bytes").unwrap();
            let snapshot = workspace.join(stage::STAGED_SOURCE_NAME);
            fs::write(&snapshot, b"existing staged source").unwrap();
            let (mut manifest, _) = sparse_stage_fixture();
            manifest.operation = operation;
            if operation == workspace::Operation::Checkout {
                manifest.modules.remove(0);
            }
            write_manifest(&workspace, &manifest).unwrap();
            let count = fs::read_dir(&workspace).unwrap().count();
            let error = stage_workspace(&workspace, None, "TestMod", None, None).unwrap_err();
            assert!(error.to_string().contains("unowned nonempty"), "{error:#}");
            assert!(prepare_stage_work_dir(&work).is_err());
            assert_eq!(fs::read(&sentinel).unwrap(), sentinel_bytes);
            assert_eq!(fs::read(&spec).unwrap(), b"existing spec bytes");
            assert_eq!(fs::read(&snapshot).unwrap(), b"existing staged source");
            assert_eq!(fs::read_dir(&workspace).unwrap().count(), count);
            assert_eq!(fs::read_dir(&work).unwrap().count(), 1);
            assert!(!work.join(STAGE_WORK_OWNER).exists());
        }
    }

    #[test]
    fn stage_claims_empty_scratch_and_reuses_only_its_owned_directory() {
        let temp = tempfile::tempdir().unwrap();
        for preexisting in [false, true] {
            let work = temp.path().join(format!("workspace-{preexisting}.work"));
            if preexisting {
                fs::create_dir(&work).unwrap();
            }
            prepare_stage_work_dir(&work).unwrap();
            let owner = work.join(STAGE_WORK_OWNER);
            let owner_bytes = fs::read(&owner).unwrap();
            fs::create_dir(work.join("tree")).unwrap();
            let compiled = work.join("tree/compiled.as");
            fs::write(&compiled, b"previous compilation").unwrap();
            prepare_stage_work_dir(&work).unwrap();
            assert_eq!(fs::read(&compiled).unwrap(), b"previous compilation");
            assert_eq!(fs::read(&owner).unwrap(), owner_bytes);

            // A copied marker or corrupt claim cannot authorize another scratch tree.
            let unowned = temp.path().join(format!("other-{preexisting}.work"));
            fs::create_dir_all(unowned.join("tree")).unwrap();
            fs::write(unowned.join(STAGE_WORK_OWNER), &owner_bytes).unwrap();
            fs::write(unowned.join("tree/keep.as"), b"unowned bytes").unwrap();
            let error = prepare_stage_work_dir(&unowned).unwrap_err();
            assert!(error.to_string().contains("invalid ownership marker"));
            assert_eq!(
                fs::read(unowned.join("tree/keep.as")).unwrap(),
                b"unowned bytes"
            );
            fs::remove_file(&owner).unwrap();
            assert!(prepare_stage_work_dir(&work).is_err());
            assert!(!owner.exists());
            assert_eq!(fs::read(&compiled).unwrap(), b"previous compilation");
        }
    }

    #[test]
    fn npc_text_hint_requires_a_chosen_input_and_a_separate_quoted_artifact() {
        for edits in [
            "npc files/name 'edits'.json",
            "npc files/name 'edits'.lcache",
            "npc files/name 'edits'.LCACHE",
        ] {
            let edits = Path::new(edits);
            let command = display_name_import_command(edits);
            let output = if edits
                .extension()
                .unwrap()
                .to_str()
                .unwrap()
                .eq_ignore_ascii_case("lcache")
            {
                PathBuf::from(format!("{}.patched.lcache", edits.display()))
            } else {
                edits.with_extension("lcache")
            };
            assert_ne!(output, edits);
            assert_eq!(output.parent(), edits.parent());
            assert_eq!(
                command,
                format!(
                    "gore loc import --lcache {} --out {} --edits {} --add-missing",
                    stage::shell_quote("<path/to/chosen-input.lcache>"),
                    stage::shell_quote(&output.display().to_string()),
                    stage::shell_quote(&edits.display().to_string()),
                )
            );
        }
    }

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
    fn levels_include_free_only_modules_and_keep_old_camp_variants_distinct() {
        let point = |name: &str, module: &str, occupied: bool| sites::WorldPoint {
            name: name.to_string(),
            module: module.to_string(),
            occupants: if occupied {
                vec!["USpawnAIAgentDefinition_Test".to_string()]
            } else {
                Vec::new()
            },
        };
        let levels = levels_from_world_points(&[
            point("UWP_A", "LevelScripts.Map_x2_y1_OldCamp_AI_script", true),
            point("UWP_B", "LevelScripts.Map_x2_y1_OldCamp_AI_script", false),
            point("UWP_C", "LevelScripts.Map_OldCamp_IE_script", false),
            point("UWP_D", "LevelScripts.TrollCanyon_AI_script", false),
        ]);
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].name, "OldCamp");
        assert_eq!(levels[0].world_points, 2);
        assert_eq!(levels[0].occupied_points, 1);
        assert_eq!(levels[1].name, "OldCamp_IE");
        assert_eq!(levels[1].module, "LevelScripts.Map_OldCamp_IE_script");
        assert_eq!(levels[2].name, "TrollCanyon");
        assert_eq!(levels[2].occupied_points, 0);
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
    fn suppression_scope_rejects_partial_or_wrong_level_modules() {
        let spawn = "USpawnAIAgentDefinition_MINE";
        let first = "LevelScripts.First";
        let second = "LevelScripts.Second";
        let single = emitted(vec![
            site("UWP_A", first, spawn),
            site("UWP_B", first, spawn),
        ]);
        assert!(suppression_scope_finding(&single, spawn, first).is_none());
        assert!(suppression_scope_finding(&single, spawn, second).is_some());

        let multiple = emitted(vec![
            site("UWP_A", first, spawn),
            site("UWP_B", second, spawn),
        ]);
        let finding = suppression_scope_finding(&multiple, spawn, first).unwrap();
        assert_eq!(finding.severity, check::Severity::Blocking);
        assert!(finding.message.contains(first) && finding.message.contains(second));
        assert!(suppression_scope_finding(&emitted(Vec::new()), spawn, first).is_some());
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
