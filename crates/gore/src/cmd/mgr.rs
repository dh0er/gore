//! `gore mgr` — the multi-mod manager CLI namespace. Thin wrapper over the
//! `gore-mod` manager engine (`gore_mod::mgr`): a **library** of imported mods,
//! a **loadout** (which mods are enabled, in mount order), conflict analysis, and
//! composing the enabled set into ONE deployment against the game.
//!
//! Every subcommand uses either the shared per-user Store or one explicit
//! `--library` / `--loadout` pair. Requiring both overrides prevents a custom
//! loadout from being reconciled against an unrelated library. Engine errors
//! (`gore_mod::ModError`) are wrapped into `anyhow` here.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::io::Write as _;
use std::path::PathBuf;

use gore_mod::mgr::{
    self,
    analyze::{Conflict, Severity},
    apply::undeploy_manager_only,
    import,
    model::FootprintCoverage,
    status::ManagerStatus,
    store::StoreSnapshot,
};
use gore_mod::{ManagerInstallRecoveryOutcome, ManagerInstallRecoveryReadiness};

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
    /// Inspect Manager readiness and recovery evidence without changing anything
    Preflight {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
        #[arg(long)]
        library: Option<PathBuf>,
        #[arg(long)]
        loadout: Option<PathBuf>,
        /// Emit the stable native preflight response as JSON
        #[arg(long)]
        json: bool,
    },
    /// Recover one exact abandoned Manager mutation selected by its preflight token
    Recover {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
        /// Exact opaque action token reported by `mgr preflight`
        #[arg(long)]
        expected_guard_id: String,
        /// Confirm recovery without the interactive y/N prompt
        #[arg(long)]
        yes: bool,
        /// Emit the stable native recovery response as JSON (requires --yes)
        #[arg(long, requires = "yes")]
        json: bool,
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
        /// Emit the full native status report, including bounded owned-path evidence, as JSON
        #[arg(long)]
        json: bool,
    },
    /// Undeploy the active Manager deployment (never removes a Studio deploy)
    Reset {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
    },
}

/// Resolve one complete Store identity. A lone override cannot identify which Library and Loadout
/// belong together, and opening that mixed pair could destructively reconcile the wrong file.
fn store_paths(library: Option<PathBuf>, loadout: Option<PathBuf>) -> Result<(PathBuf, PathBuf)> {
    match (library, loadout) {
        (None, None) => Ok((mgr::paths::library_dir(), mgr::paths::loadout_path())),
        (Some(library), Some(loadout)) => Ok((library, loadout)),
        _ => anyhow::bail!(
            "--library and --loadout overrides must be supplied together so they identify one manager store"
        ),
    }
}

pub fn run(action: MgrAction) -> Result<()> {
    match action {
        MgrAction::Import {
            path,
            library,
            loadout,
        } => {
            let (lib, ld_path) = store_paths(library, loadout)?;

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
            let (lib, ld_path) = store_paths(library, loadout)?;

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
            let (lib, ld_path) = store_paths(library, loadout)?;

            let removed = import::remove(&lib, &id).map_err(|e| anyhow::anyhow!("{e}"))?;
            StoreSnapshot::open(&lib, &ld_path)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| {
                    format!("removed {id:?} from the library but failed to reconcile the loadout")
                })?;

            println!("removed {id}: {removed}");
            if removed {
                println!(
                    "library and loadout updated; run 'gore mgr apply' to update any deployed game"
                );
            }
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
            let (lib, ld_path) = store_paths(library, loadout)?;
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
            let (lib, ld_path) = store_paths(library, loadout)?;

            let store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let conflicts = store.analyze();

            if conflicts.is_empty() {
                println!("no recognized conflicts");
            } else {
                for c in &conflicts {
                    println!("{}", format_conflict(c));
                }
            }
            if let Some(warning) = coverage_warning(&store) {
                println!("warning: {warning}");
            }
            Ok(())
        }

        MgrAction::Preflight {
            game,
            library,
            loadout,
            json,
        } => {
            let game = gore_loc::config::game_root(game)?;
            let (lib, ld_path) = store_paths(library, loadout)?;
            let report = mgr::preflight::preflight_v1(&game, &lib, &ld_path);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "preflight": report,
                    }))?
                );
            } else {
                print_preflight(&report);
            }
            Ok(())
        }

        MgrAction::Recover {
            game,
            expected_guard_id,
            yes,
            json,
        } => {
            let game = gore_loc::config::game_root(game)?;
            recover_selected_manager_install(&game, &expected_guard_id, yes, json)
        }

        MgrAction::Apply {
            game,
            library,
            loadout,
        } => {
            let game = gore_loc::config::game_root(game)?;
            let (lib, ld_path) = store_paths(library, loadout)?;

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
            json,
        } => {
            let game = gore_loc::config::game_root(game)?;
            let (lib, ld_path) = store_paths(library, loadout)?;
            let store = StoreSnapshot::open(&lib, &ld_path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let report = store
                .status_report(&game)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "status": report,
                    }))?
                );
            } else {
                println!("{}", describe_status(&game, &report.status));
            }
            Ok(())
        }

        MgrAction::Reset { game } => {
            let game = gore_loc::config::game_root(game)?;
            let removed = match undeploy_manager_only(&game) {
                Ok(removed) => removed,
                Err(error) => {
                    let message = error.to_string();
                    if let Some(name) = message.strip_prefix("STUDIO_DEPLOY_ACTIVE:") {
                        anyhow::bail!(
                            "refusing: a studio deploy is active ({name}) — undeploy it from Mod Studio"
                        );
                    }
                    return Err(anyhow::anyhow!(message)).context("resetting manager deployment");
                }
            };
            if removed {
                println!("reset: undeployed the active manager deployment");
            } else {
                println!("reset: nothing was deployed");
            }
            Ok(())
        }
    }
}

fn print_preflight(report: &mgr::preflight::ManagerPreflightV1) {
    for check in &report.checks {
        println!(
            "{:?}: {:?} code={} action={} — {}",
            check.id, check.state, check.code, check.action, check.detail
        );
        if let Some(token) = &check.action_token {
            println!("  recovery token: {token}");
        }
        for item in &check.items {
            println!("  {item}");
        }
    }
}

fn validate_recovery_selection(
    readiness: &ManagerInstallRecoveryReadiness,
    expected_guard_id: &str,
) -> Result<()> {
    match readiness {
        ManagerInstallRecoveryReadiness::AbandonedManager { guard_id }
            if guard_id == expected_guard_id =>
        {
            Ok(())
        }
        ManagerInstallRecoveryReadiness::AbandonedManager { .. } => anyhow::bail!(
            "recovery selection changed; run 'gore mgr preflight' again and use its exact token"
        ),
        ManagerInstallRecoveryReadiness::Missing => {
            anyhow::bail!("no abandoned Manager mutation is available for recovery")
        }
        ManagerInstallRecoveryReadiness::Active => anyhow::bail!(
            "recovery blocked: a GORE installation change is active; wait for it to finish"
        ),
        ManagerInstallRecoveryReadiness::CompileOrAmbiguous => anyhow::bail!(
            "recovery blocked: script-build recovery or an unclear lock owner requires recovery help"
        ),
        ManagerInstallRecoveryReadiness::Invalid => anyhow::bail!(
            "recovery blocked: GORE could not inspect the installation lock safely"
        ),
    }
}

fn recover_selected_manager_install(
    game_root: &std::path::Path,
    expected_guard_id: &str,
    yes: bool,
    json: bool,
) -> Result<()> {
    if expected_guard_id.is_empty() {
        anyhow::bail!("--expected-guard-id must not be empty");
    }
    if expected_guard_id.len() > 512 {
        anyhow::bail!("--expected-guard-id exceeds its 512-byte limit");
    }

    let readiness = gore_mod::probe_manager_install_recovery(game_root);
    validate_recovery_selection(&readiness, expected_guard_id)?;

    if !yes {
        println!("Recover this exact abandoned Manager installation change?");
        println!("  game: {}", game_root.display());
        println!("  recovery token: {expected_guard_id}");
        println!(
            "Recovery may restore the recorded pristine state or preserve a change that already completed."
        );
        print!("Proceed? [y/N] ");
        std::io::stdout()
            .flush()
            .context("flushing recovery prompt")?;
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .context("reading recovery confirmation")?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let outcome = gore_mod::recover_manager_install(game_root, expected_guard_id)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .context("recovering the selected Manager installation change")?;
    let completed = recovery_outcome_completed(outcome);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": completed,
                "outcome": outcome,
            }))?
        );
    } else if completed {
        println!("{}", describe_recovery_outcome(outcome));
    }
    if completed {
        Ok(())
    } else {
        anyhow::bail!(describe_recovery_outcome(outcome))
    }
}

fn recovery_outcome_completed(outcome: ManagerInstallRecoveryOutcome) -> bool {
    matches!(
        outcome,
        ManagerInstallRecoveryOutcome::AlreadyClean
            | ManagerInstallRecoveryOutcome::PreMutationLockCleared
            | ManagerInstallRecoveryOutcome::RecoveredToPristine
            | ManagerInstallRecoveryOutcome::CompletedApplyPreserved
            | ManagerInstallRecoveryOutcome::CompletedUndeployConfirmed
    )
}

fn describe_recovery_outcome(outcome: ManagerInstallRecoveryOutcome) -> &'static str {
    match outcome {
        ManagerInstallRecoveryOutcome::AlreadyClean => {
            "recovery: the selected installation change is already clean"
        }
        ManagerInstallRecoveryOutcome::Busy => {
            "recovery not performed: another installation change became active"
        }
        ManagerInstallRecoveryOutcome::PreMutationLockCleared => {
            "recovery complete: the abandoned pre-mutation lock was cleared"
        }
        ManagerInstallRecoveryOutcome::RecoveredToPristine => {
            "recovery complete: the installation was restored to its recorded pristine state"
        }
        ManagerInstallRecoveryOutcome::CompletedApplyPreserved => {
            "recovery complete: the already-completed Manager apply was preserved"
        }
        ManagerInstallRecoveryOutcome::CompletedUndeployConfirmed => {
            "recovery complete: the already-completed Manager reset was confirmed"
        }
        ManagerInstallRecoveryOutcome::CompileRecoveryRequired => {
            "recovery not performed: script-build recovery is required"
        }
        ManagerInstallRecoveryOutcome::InspectionFailed => {
            "recovery not performed: the installation could not be inspected safely"
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CoverageGaps {
    partial: usize,
    advisory: usize,
    opaque: usize,
}

fn coverage_gaps(store: &StoreSnapshot) -> CoverageGaps {
    let mut gaps = CoverageGaps::default();
    for selected in store.loadout().entries.iter().filter(|entry| entry.enabled) {
        let Some(meta) = store.mods().iter().find(|meta| meta.id == selected.id) else {
            continue;
        };
        for component in &meta.components {
            match component.footprint_coverage() {
                FootprintCoverage::Exact => {}
                FootprintCoverage::Partial => gaps.partial += 1,
                FootprintCoverage::Advisory => gaps.advisory += 1,
                FootprintCoverage::Opaque => gaps.opaque += 1,
            }
        }
    }
    gaps
}

fn coverage_warning(store: &StoreSnapshot) -> Option<String> {
    let gaps = coverage_gaps(store);
    (gaps != CoverageGaps::default()).then(|| {
        format!(
            "conflict analysis is incomplete for enabled components (partial={}, advisory={}, opaque={})",
            gaps.partial, gaps.advisory, gaps.opaque
        )
    })
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
    let (lib, ld_path) = store_paths(library, loadout)?;
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
fn describe_status(game_root: &std::path::Path, st: &ManagerStatus) -> String {
    let recovery = gore_mod::probe_manager_install_recovery(game_root);
    if !matches!(recovery, ManagerInstallRecoveryReadiness::Missing) {
        return describe_recovery_status(&recovery);
    }

    match st {
        ManagerStatus::NothingDeployed => "nothing deployed".to_string(),
        ManagerStatus::RecoveryRequired => describe_recovery_status(&recovery),
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

fn describe_recovery_status(readiness: &ManagerInstallRecoveryReadiness) -> String {
    match readiness {
        ManagerInstallRecoveryReadiness::Missing => {
            "recovery required: previous apply was interrupted (run 'gore mgr reset' before \
             applying again)"
                .to_string()
        }
        ManagerInstallRecoveryReadiness::Active => {
            "recovery blocked: a GORE installation change is still active (wait for it to finish; \
             do not delete lock files)"
                .to_string()
        }
        ManagerInstallRecoveryReadiness::AbandonedManager { .. } => {
            "recovery required: an interrupted Manager change was abandoned (run 'gore mgr \
             preflight' to obtain its exact token, then 'gore mgr recover --expected-guard-id \
             <TOKEN>'; do not delete lock files or run reset before recovery)"
                .to_string()
        }
        ManagerInstallRecoveryReadiness::CompileOrAmbiguous => {
            "recovery blocked: script-build recovery or an unclear lock source needs recovery help \
             (do not delete lock files or run reset/undeploy)"
                .to_string()
        }
        ManagerInstallRecoveryReadiness::Invalid => {
            "recovery blocked: GORE could not inspect the installation lock safely (leave recovery \
             data unchanged; do not delete lock files or run reset/undeploy)"
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_as::compile::InstallMutationGuard;
    use gore_mod::mgr::analyze::ConflictKind;

    #[test]
    fn custom_store_overrides_must_be_paired() {
        assert!(store_paths(Some(PathBuf::from("library")), None).is_err());
        assert!(store_paths(None, Some(PathBuf::from("loadout.json"))).is_err());
        assert_eq!(
            store_paths(
                Some(PathBuf::from("library")),
                Some(PathBuf::from("loadout.json")),
            )
            .unwrap(),
            (PathBuf::from("library"), PathBuf::from("loadout.json")),
        );
    }

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

    #[test]
    fn recovery_status_keeps_each_native_recovery_lane_honest() {
        let missing = describe_recovery_status(&ManagerInstallRecoveryReadiness::Missing);
        assert!(missing.contains("gore mgr reset"), "{missing}");

        let active = describe_recovery_status(&ManagerInstallRecoveryReadiness::Active);
        assert!(active.contains("wait for it to finish"), "{active}");
        assert!(!active.contains("reset/undeploy only after"), "{active}");

        let abandoned =
            describe_recovery_status(&ManagerInstallRecoveryReadiness::AbandonedManager {
                guard_id: "opaque-test-token".into(),
            });
        assert!(abandoned.contains("gore mgr preflight"), "{abandoned}");
        assert!(abandoned.contains("gore mgr recover"), "{abandoned}");
        assert!(abandoned.contains("exact token"), "{abandoned}");
        assert!(!abandoned.contains("opaque-test-token"), "{abandoned}");

        let compile =
            describe_recovery_status(&ManagerInstallRecoveryReadiness::CompileOrAmbiguous);
        assert!(compile.contains("recovery help"), "{compile}");
        assert!(compile.contains("do not delete lock files"), "{compile}");

        let invalid = describe_recovery_status(&ManagerInstallRecoveryReadiness::Invalid);
        assert!(invalid.contains("could not inspect"), "{invalid}");
        assert!(invalid.contains("do not delete lock files"), "{invalid}");
    }

    #[test]
    fn only_completed_recovery_outcomes_are_cli_successes() {
        for outcome in [
            ManagerInstallRecoveryOutcome::AlreadyClean,
            ManagerInstallRecoveryOutcome::PreMutationLockCleared,
            ManagerInstallRecoveryOutcome::RecoveredToPristine,
            ManagerInstallRecoveryOutcome::CompletedApplyPreserved,
            ManagerInstallRecoveryOutcome::CompletedUndeployConfirmed,
        ] {
            assert!(recovery_outcome_completed(outcome), "{outcome:?}");
        }
        for outcome in [
            ManagerInstallRecoveryOutcome::Busy,
            ManagerInstallRecoveryOutcome::CompileRecoveryRequired,
            ManagerInstallRecoveryOutcome::InspectionFailed,
        ] {
            assert!(!recovery_outcome_completed(outcome), "{outcome:?}");
        }
    }

    #[test]
    fn status_prioritizes_every_lock_lane_without_a_deploy_record() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path();
        let nothing = ManagerStatus::NothingDeployed;

        assert_eq!(describe_status(game, &nothing), "nothing deployed");

        let guard = InstallMutationGuard::acquire(game, "gore-mod:manager-apply").unwrap();
        let lock = guard.path().to_path_buf();
        assert_eq!(
            describe_status(game, &nothing),
            describe_recovery_status(&ManagerInstallRecoveryReadiness::Active)
        );
        drop(guard);

        let guard = InstallMutationGuard::acquire(game, "gore-mod:manager-apply").unwrap();
        guard.preserve_for_manual_recovery();
        assert_eq!(
            describe_status(game, &nothing),
            describe_recovery_status(&ManagerInstallRecoveryReadiness::AbandonedManager {
                guard_id: "opaque".into(),
            })
        );
        std::fs::remove_file(&lock).unwrap();

        let guard = InstallMutationGuard::acquire(game, "gore-as:compile").unwrap();
        guard.preserve_for_manual_recovery();
        assert_eq!(
            describe_status(game, &nothing),
            describe_recovery_status(&ManagerInstallRecoveryReadiness::CompileOrAmbiguous)
        );
        std::fs::remove_file(&lock).unwrap();

        std::fs::write(&lock, b"not an install-mutation record").unwrap();
        assert_eq!(
            describe_status(game, &nothing),
            describe_recovery_status(&ManagerInstallRecoveryReadiness::Invalid)
        );
        std::fs::remove_file(&lock).unwrap();

        assert_eq!(describe_status(game, &nothing), "nothing deployed");
    }

    #[test]
    fn recovery_selection_accepts_only_the_exact_abandoned_manager_token() {
        assert!(validate_recovery_selection(
            &ManagerInstallRecoveryReadiness::AbandonedManager {
                guard_id: "selected-token".into(),
            },
            "selected-token",
        )
        .is_ok());

        let changed = validate_recovery_selection(
            &ManagerInstallRecoveryReadiness::AbandonedManager {
                guard_id: "new-token".into(),
            },
            "selected-token",
        )
        .unwrap_err()
        .to_string();
        assert!(changed.contains("selection changed"), "{changed}");

        for (readiness, expected) in [
            (ManagerInstallRecoveryReadiness::Missing, "no abandoned"),
            (ManagerInstallRecoveryReadiness::Active, "active"),
            (
                ManagerInstallRecoveryReadiness::CompileOrAmbiguous,
                "script-build recovery",
            ),
            (ManagerInstallRecoveryReadiness::Invalid, "inspect"),
        ] {
            let error = validate_recovery_selection(&readiness, "selected-token")
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "got {error:?}");
        }
    }
}
