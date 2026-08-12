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
    analyze::{Conflict, Severity},
    apply::undeploy_all,
    import,
    status::ManagerStatus,
    store::StoreSnapshot,
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
        game: Option<PathBuf>,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Show whether the game is in sync with the target loadout
    Status {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
    },
    /// Undeploy everything the manager has active (restore pristine)
    Reset {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
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
        MgrAction::Import {
            path,
            library,
            loadout,
        } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let outcome = import::import_detailed(&lib, &path)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("importing {}", path.display()))?;
            let entry = &outcome.entry;
            // Library publication and loadout registration are deliberately two commits. Import
            // has released its Library lock before Store acquires its canonical root set and repairs any
            // valid partial publication. A loadout error leaves the imported library entry intact.
            StoreSnapshot::open(&lib, &ld_path)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| {
                    format!(
                        "imported {:?} into the library but failed to reconcile the loadout",
                        entry.id
                    )
                })?;

            println!(
                "imported {} ({}) [{:?}] disposition={} matched_by={}",
                entry.id,
                entry.name,
                entry.kind,
                outcome.disposition.as_str(),
                outcome.matched_by.as_str()
            );
            Ok(())
        }

        MgrAction::List { library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let ld = store.loadout();
            let mods = store.mods();

            // One row per loadout entry (in loadout order), joined to its metadata
            // by id; library mods not in the loadout are appended after.
            println!(
                "{:<3} {:<28} {:<26} {:<14} comps",
                "on", "id", "name", "kind"
            );
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
            for m in mods {
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

        MgrAction::Remove {
            id,
            library,
            loadout,
        } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let removed = import::remove(&lib, &id).map_err(|e| anyhow::anyhow!("{e}"))?;
            StoreSnapshot::open(&lib, &ld_path)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| {
                    format!("removed {id:?} from the library but failed to reconcile the loadout")
                })?;

            println!("removed {id}: {removed}");
            Ok(())
        }

        MgrAction::Enable {
            id,
            library,
            loadout,
        } => set_enabled(&id, true, library, loadout),
        MgrAction::Disable {
            id,
            library,
            loadout,
        } => set_enabled(&id, false, library, loadout),

        MgrAction::Order {
            id,
            pos,
            library,
            loadout,
        } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);
            let mut store =
                StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut to = 0;
            store
                .update_loadout(|ld| {
                    let from = ld
                        .entries
                        .iter()
                        .position(|entry| entry.id == id)
                        .ok_or_else(|| {
                            gore_mod::ModError::Other(format!("no loadout entry with id {id:?}"))
                        })?;
                    to = pos.min(ld.entries.len() - 1);
                    let entry = ld.entries.remove(from);
                    ld.entries.insert(to, entry);
                    Ok(())
                })
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let order: Vec<&str> = store
                .loadout()
                .entries
                .iter()
                .map(|e| e.id.as_str())
                .collect();
            println!("moved {id} to position {to}; order: {}", order.join(" -> "));
            Ok(())
        }

        MgrAction::Analyze { library, loadout } => {
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let conflicts = store.analyze();

            if conflicts.is_empty() {
                println!("no conflicts");
            } else {
                for c in &conflicts {
                    println!("{}", format_conflict(c));
                }
            }
            Ok(())
        }

        MgrAction::Apply {
            game,
            library,
            loadout,
        } => {
            let game = gore_loc::config::game_root(game)?;
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);

            let store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let report = match store.apply(&game) {
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

        MgrAction::Status {
            game,
            library,
            loadout,
        } => {
            let game = gore_loc::config::game_root(game)?;
            let lib = library_of(library);
            let ld_path = loadout_of(loadout);
            let store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let st = store.status(&game).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{}", describe_status(&st));
            Ok(())
        }

        MgrAction::Reset { game } => {
            let game = gore_loc::config::game_root(game)?;
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

fn format_conflict(conflict: &Conflict) -> String {
    let chain = conflict.mods.join(" -> ");
    if conflict.severity == Severity::Info {
        format!(
            "{:?} {:?} {}: {} (advisory; no winner)",
            conflict.severity, conflict.kind, conflict.target, chain
        )
    } else {
        let winner = conflict.mods.last().map(String::as_str).unwrap_or("");
        format!(
            "{:?} {:?} {}: {} (winner: {})",
            conflict.severity, conflict.kind, conflict.target, chain, winner
        )
    }
}

/// Set the `enabled` flag of loadout entry `id`; error if it isn't in the loadout.
/// The selected library is part of the same strict Store snapshot and must be forwarded.
fn set_enabled(
    id: &str,
    enabled: bool,
    library: Option<PathBuf>,
    loadout: Option<PathBuf>,
) -> Result<()> {
    let lib = library_of(library);
    let ld_path = loadout_of(loadout);
    let mut store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    store
        .update_loadout(|loadout| {
            let entry = loadout
                .entries
                .iter_mut()
                .find(|entry| entry.id == id)
                .ok_or_else(|| {
                    gore_mod::ModError::Other(format!(
                        "no loadout entry with id {id:?} (import it first)"
                    ))
                })?;
            entry.enabled = enabled;
            Ok(())
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{id}: {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

/// A friendly one-liner per [`ManagerStatus`] variant (the Debug form is verbose).
fn describe_status(st: &ManagerStatus) -> String {
    match st {
        ManagerStatus::NothingDeployed => "nothing deployed".to_string(),
        ManagerStatus::RecoveryRequired => {
            "recovery required: previous apply was interrupted (run undeploy first)".to_string()
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use gore_mod::mgr::analyze::ConflictKind;

    fn conflict(severity: Severity) -> Conflict {
        Conflict {
            kind: if severity == Severity::Info {
                ConflictKind::Ue4ssUnknown
            } else {
                ConflictKind::Cdo
            },
            target: if severity == Severity::Info {
                "<unknown>".into()
            } else {
                "A.Value".into()
            },
            mods: vec!["first".into(), "last".into()],
            severity,
        }
    }

    #[test]
    fn info_advisory_has_no_false_winner() {
        assert_eq!(
            format_conflict(&conflict(Severity::Info)),
            "Info Ue4ssUnknown <unknown>: first -> last (advisory; no winner)"
        );
    }

    #[test]
    fn proven_conflict_keeps_later_wins_output() {
        assert_eq!(
            format_conflict(&conflict(Severity::Soft)),
            "Soft Cdo A.Value: first -> last (winner: last)"
        );
    }
}
