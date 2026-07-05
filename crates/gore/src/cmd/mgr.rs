//! `gore mgr` — the multi-mod manager CLI namespace. Thin wrapper over the
//! `gore-mod` manager engine (`gore_mod::mgr`): a **library** of imported mods,
//! a **loadout** (which mods are enabled, in mount order), conflict analysis, and
//! composing the enabled set into ONE deployment against the game.
//!
//! Every subcommand resolves its `--library` / `--loadout` overrides against the
//! shared per-user defaults (`gore_mod::mgr::paths`), so with no flags every tool
//! sees the same state. Engine errors (`gore_mod::ModError`) are wrapped into
//! `anyhow` here.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

use gore_mod::mgr::{
    self,
    analyze::analyze,
    apply::{apply_loadout, undeploy_all},
    import,
    loadout::{self, Loadout, LoadoutEntry},
    status::{status, ManagerStatus},
};

#[derive(Subcommand)]
pub enum MgrAction {
    /// Import a mod (folder, .zip, or single game file) into the library
    Import {
        /// Source folder / .zip / game file to import
        path: PathBuf,
        /// Library dir (default: the shared per-user manager library)
        #[arg(long)]
        library: Option<PathBuf>,
        /// Loadout file (default: the shared per-user loadout)
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// List library mods joined to their loadout state (enabled/order)
    List {
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Remove a mod from the library (and drop it from the loadout)
    Remove {
        /// Library entry id to remove
        id: String,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Enable a loadout entry (it will deploy on the next apply)
    Enable {
        /// Library entry id to enable
        id: String,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Disable a loadout entry (it will not deploy)
    Disable {
        /// Library entry id to disable
        id: String,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Move a loadout entry to a new position (0 = mounts first, loses conflicts)
    Order {
        /// Library entry id to move
        id: String,
        /// New 0-based position (clamped to the last slot)
        pos: usize,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Report conflicts among the enabled loadout mods
    Analyze {
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Compose the enabled loadout into one deployment against the game
    Apply {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: PathBuf,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Show whether the game is in sync with the target loadout
    Status {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: PathBuf,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Undeploy everything the manager has active (restore pristine)
    Reset {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: PathBuf,
    },
}

/// Resolve the library dir: the `--library` override or the shared default.
fn library_of(arg: Option<PathBuf>) -> PathBuf {
    arg.unwrap_or_else(mgr::paths::library_dir)
}

/// Resolve the loadout path: the `--loadout` override or the shared default.
fn loadout_of(arg: Option<PathBuf>) -> PathBuf {
    arg.unwrap_or_else(mgr::paths::loadout_path)
}

pub fn run(action: MgrAction) -> Result<()> {
    match action {
        MgrAction::Import { path, library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let entry = import::import(&lib, &path)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("importing {}", path.display()))?;

            // Also register the new mod in the loadout (disabled) so `enable`
            // can find it. Skip if an entry with this id already exists (re-import
            // / update): keep its current enabled state and position.
            let mut ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            if !ld.entries.iter().any(|e| e.id == entry.id) {
                ld.entries.push(LoadoutEntry { id: entry.id.clone(), enabled: false });
                loadout::save(&ld_path, &ld).map_err(|e| anyhow::anyhow!("{e}"))?;
            }

            println!("imported {} ({}) [{:?}]", entry.id, entry.name, entry.kind);
            Ok(())
        }

        MgrAction::List { library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let mods = import::list(&lib).map_err(|e| anyhow::anyhow!("{e}"))?;

            // One row per loadout entry (in loadout order), joined to its metadata
            // by id; library mods not in the loadout are appended after.
            println!("{:<3} {:<28} {:<26} {:<14} {}", "on", "id", "name", "kind", "comps");
            let mut shown: Vec<&str> = Vec::new();
            for e in &ld.entries {
                let meta = mods.iter().find(|m| m.id == e.id);
                let (name, kind, comps) = match meta {
                    Some(m) => (m.name.as_str(), format!("{:?}", m.kind), m.components.len()),
                    None => ("<missing from library>", "?".to_string(), 0),
                };
                println!(
                    "[{}] {:<28} {:<26} {:<14} {}",
                    if e.enabled { "x" } else { " " },
                    e.id,
                    name,
                    kind,
                    comps
                );
                shown.push(&e.id);
            }
            for m in &mods {
                if shown.iter().any(|id| *id == m.id) {
                    continue;
                }
                println!(
                    "[ ] {:<28} {:<26} {:<14} {}   (not in loadout)",
                    m.id,
                    m.name,
                    format!("{:?}", m.kind),
                    m.components.len()
                );
            }
            Ok(())
        }

        MgrAction::Remove { id, library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let removed = import::remove(&lib, &id).map_err(|e| anyhow::anyhow!("{e}"))?;

            // Drop it from the loadout too so a stale entry doesn't linger.
            let mut ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let before = ld.entries.len();
            ld.entries.retain(|e| e.id != id);
            if ld.entries.len() != before {
                loadout::save(&ld_path, &ld).map_err(|e| anyhow::anyhow!("{e}"))?;
            }

            println!("removed {id}: {removed}");
            Ok(())
        }

        MgrAction::Enable { id, loadout, .. } => set_enabled(&id, true, loadout),
        MgrAction::Disable { id, loadout, .. } => set_enabled(&id, false, loadout),

        MgrAction::Order { id, pos, loadout, .. } => {
            let ld_path = loadout_of(loadout);
            let mut ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;

            let from = ld
                .entries
                .iter()
                .position(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("no loadout entry with id {id:?}"))?;
            // Clamp to the last slot (len-1); entries is non-empty since `from` exists.
            let to = pos.min(ld.entries.len() - 1);
            let entry = ld.entries.remove(from);
            ld.entries.insert(to, entry);
            loadout::save(&ld_path, &ld).map_err(|e| anyhow::anyhow!("{e}"))?;

            let order: Vec<&str> = ld.entries.iter().map(|e| e.id.as_str()).collect();
            println!("moved {id} to position {to}; order: {}", order.join(" -> "));
            Ok(())
        }

        MgrAction::Analyze { library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let mods = import::list(&lib).map_err(|e| anyhow::anyhow!("{e}"))?;
            let ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let refs: Vec<&_> = mods.iter().collect();
            let conflicts = analyze(&refs, &ld);

            if conflicts.is_empty() {
                println!("no conflicts");
            } else {
                for c in &conflicts {
                    let chain = c.mods.join(" -> ");
                    let winner = c.mods.last().map(String::as_str).unwrap_or("");
                    println!(
                        "{:?} {:?} {}: {} (winner: {})",
                        c.severity, c.kind, c.target, chain, winner
                    );
                }
            }
            Ok(())
        }

        MgrAction::Apply { game, library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let report = match apply_loadout(&game, &lib, &ld) {
                Ok(r) => r,
                Err(e) => {
                    let msg = e.to_string();
                    // The engine flags an active studio (single-mod) deployment with a
                    // machine prefix; translate it into a friendly refusal.
                    if let Some(name) = msg.strip_prefix("STUDIO_DEPLOY_ACTIVE:") {
                        anyhow::bail!(
                            "refusing: a studio deploy is active ({name}) — undeploy it first"
                        );
                    }
                    return Err(anyhow::anyhow!("{msg}")).context("applying loadout");
                }
            };

            if report.applied.is_empty() {
                println!("applied nothing (no enabled mods)");
            } else {
                println!("applied {} mod(s):", report.applied.len());
                for name in &report.applied {
                    println!("  {name}");
                }
            }
            if !report.warnings.is_empty() {
                println!("warnings:");
                for w in &report.warnings {
                    println!("  {w}");
                }
            }
            Ok(())
        }

        MgrAction::Status { game, library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);
            let ld = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let st = status(&game, &lib, &ld).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{}", describe_status(&st));
            Ok(())
        }

        MgrAction::Reset { game } => {
            let removed = undeploy_all(&game).map_err(|e| anyhow::anyhow!("{e}"))?;
            if removed {
                println!("reset: undeployed the active manager deployment");
            } else {
                println!("reset: nothing was deployed");
            }
            Ok(())
        }
    }
}

/// Set the `enabled` flag of loadout entry `id`; error if it isn't in the loadout.
/// (`--library` is accepted for a uniform CLI surface but isn't needed here.)
fn set_enabled(id: &str, enabled: bool, loadout: Option<PathBuf>) -> Result<()> {
    let ld_path = loadout_of(loadout);
    let mut ld: Loadout = loadout::load(&ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let entry = ld
        .entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| anyhow::anyhow!("no loadout entry with id {id:?} (import it first)"))?;
    entry.enabled = enabled;
    loadout::save(&ld_path, &ld).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{id}: {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

/// A friendly one-liner per [`ManagerStatus`] variant (the Debug form is verbose).
fn describe_status(st: &ManagerStatus) -> String {
    match st {
        ManagerStatus::NothingDeployed => "nothing deployed".to_string(),
        ManagerStatus::StudioDeployActive { mod_name } => {
            format!("studio deploy active: {mod_name} (manager won't touch it)")
        }
        ManagerStatus::InSync { loadout } => {
            format!("in sync ({} mod(s) deployed)", loadout.len())
        }
        ManagerStatus::ChangesPending { deployed, target } => {
            format!(
                "changes pending: {} deployed vs {} target — re-apply needed",
                deployed.len(),
                target.len()
            )
        }
        ManagerStatus::GameUpdated { drifted } => {
            format!(
                "game updated: {} deployed file(s) drifted (re-apply to rebuild)",
                drifted.len()
            )
        }
    }
}
