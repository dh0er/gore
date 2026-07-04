//! Declarative manager status: diff what the manager currently has DEPLOYED (from the on-disk
//! deploy record) against a TARGET loadout, so the UI/CLI can show whether a re-apply is needed.
//!
//! The states form a priority ladder — the first that applies wins:
//!   1. no record ....................... [`ManagerStatus::NothingDeployed`]
//!   2. a non-manager (studio) record ... [`ManagerStatus::StudioDeployActive`] (manager must not
//!      touch a single-mod studio deployment — apply refuses it too)
//!   3. a deployed live file drifted .... [`ManagerStatus::GameUpdated`] (Steam verified/updated a
//!      file we wrote; the deployment is stale regardless of the loadout)
//!   4. deployed loadout == target AND every enabled mod's library fingerprint still matches the
//!      one recorded at deploy .......... [`ManagerStatus::InSync`]
//!   5. otherwise ....................... [`ManagerStatus::ChangesPending`] (loadout differs, OR a
//!      same-id mod was re-imported/updated so its content fingerprint no longer matches — the
//!      deployed bytes are stale even though the id set is unchanged)
//!
//! Pure read-only: it reads the record, stats the recorded live files, and reads each enabled
//! mod's library sidecar to fingerprint it; it never writes.

use std::path::Path;

use serde::Serialize;

use super::loadout::{Loadout, LoadoutEntry};
use super::model::{ModEntryMeta, META_FILE};

/// The manager's deployment state relative to a target loadout. `#[serde(tag = "state")]` so the
/// UI can switch on a single discriminant field.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ManagerStatus {
    /// No deploy record — nothing is deployed.
    NothingDeployed,
    /// A single-mod (studio) deployment is active; the manager won't diff/replace it.
    StudioDeployActive { mod_name: String },
    /// The deployed loadout matches the target; no re-apply needed.
    InSync { loadout: Vec<LoadoutEntry> },
    /// The deployed loadout differs from the target — a re-apply would change the game.
    ChangesPending { deployed: Vec<LoadoutEntry>, target: Vec<LoadoutEntry> },
    /// One or more files this manager deployed were changed externally (e.g. a Steam update or
    /// integrity verify). `drifted` lists those live paths, sorted. The deployment is stale;
    /// re-applying rebuilds against the refreshed game files.
    GameUpdated { drifted: Vec<String> },
}

/// Report the manager's state at `game_root` relative to `target` (see [`ManagerStatus`]).
/// `library_dir` is the mod library root, needed to fingerprint each enabled mod's current
/// on-disk content and compare it to what the deploy record snapshotted — so a same-id UPDATE
/// (a re-import that changed a mod but kept its id) is reported as [`ManagerStatus::ChangesPending`]
/// rather than [`ManagerStatus::InSync`].
pub fn status(game_root: &Path, library_dir: &Path, target: &Loadout) -> crate::Result<ManagerStatus> {
    // Match deploy/undeploy's record location logic so status reads the SAME record they wrote,
    // regardless of whether the caller passed the install dir or its `G1R` child, or a relative
    // path from a different cwd.
    let game_root = crate::abs_root(game_root);
    let Some(record) = crate::read_record(&game_root) else {
        return Ok(ManagerStatus::NothingDeployed);
    };

    // A studio (non-manager) deployment is off-limits to the manager: it doesn't own it and can't
    // meaningfully diff a single hand-built bundle against a loadout.
    if record.owner != "manager" {
        return Ok(ManagerStatus::StudioDeployActive { mod_name: record.mod_name });
    }

    // Drift beats loadout comparison: if the game changed a file we wrote, the deployment is stale
    // no matter what the loadout says. A missing recorded file counts as drifted (it was our
    // modded file and is now gone/replaced).
    let mut drifted: Vec<String> = record
        .deployed_hashes
        .iter()
        .filter(|(live, expected)| match std::fs::read(Path::new(live.as_str())) {
            Ok(cur) => &crate::content_hash(&cur) != *expected,
            Err(_) => true, // gone / unreadable → drifted
        })
        .map(|(live, _)| live.clone())
        .collect();

    // Additive-only deployments (foreign paks, texture triplets, UE4SS mod dirs) carry NO
    // `deployed_hashes` entry — they're whole-file/dir copies into `~mods`/`ue4ss/Mods`, not
    // in-place patches. Deleting one externally would otherwise leave `drifted` empty → InSync,
    // wrongly disabling Apply while managed content is missing. So treat any recorded additive path
    // that no longer exists on disk as drifted too (existence-only — these have no per-file hash).
    for path in
        record.managed_paks.iter().chain(record.texture_triplets.iter()).chain(record.ue4ss_mod_dirs.iter())
    {
        if !Path::new(path.as_str()).exists() {
            drifted.push(path.clone());
        }
    }

    if !drifted.is_empty() {
        drifted.sort();
        drifted.dedup();
        return Ok(ManagerStatus::GameUpdated { drifted });
    }

    // Compare the deployed snapshot against the target's ENABLED entries only, order-sensitively
    // (position is mount order). The recorded loadout is already the enabled-only snapshot apply
    // wrote, so this is a straight `Vec<LoadoutEntry>` equality.
    let target_enabled: Vec<LoadoutEntry> =
        target.entries.iter().filter(|e| e.enabled).cloned().collect();
    if record.loadout != target_enabled {
        return Ok(ManagerStatus::ChangesPending { deployed: record.loadout, target: target_enabled });
    }

    // Loadout ids/order/enabled all match. But a mod can be re-imported UNDER THE SAME ID (an
    // update) — its components/bytes change while its id (and thus the loadout) does not, so the
    // check above can't see it. Compare each enabled target mod's CURRENT library fingerprint to
    // the one the record snapshotted at deploy: any mismatch means the deployed bytes are stale.
    // A mod whose library sidecar is now missing/unreadable, or that has no recorded fingerprint
    // (a pre-fingerprint record, or a mod added since), counts as changed too — never InSync over
    // content we can't confirm is current.
    for e in &target_enabled {
        let current = match read_library_fingerprint(library_dir, &e.id) {
            Some(fp) => fp,
            // Can't read/parse the library entry → treat as changed (re-apply rebuilds from it).
            None => {
                return Ok(ManagerStatus::ChangesPending {
                    deployed: record.loadout,
                    target: target_enabled,
                })
            }
        };
        if record.deployed_fingerprints.get(&e.id) != Some(&current) {
            return Ok(ManagerStatus::ChangesPending {
                deployed: record.loadout,
                target: target_enabled,
            });
        }
    }

    Ok(ManagerStatus::InSync { loadout: record.loadout })
}

/// Read `<library_dir>/<id>/gore-manager-meta.json` and compute its content [`ModEntryMeta::fingerprint`].
/// `None` if the sidecar is missing or unparseable — the caller treats that as "changed" so a
/// removed/corrupt library entry can never leave status reporting InSync over stale deployed bytes.
fn read_library_fingerprint(library_dir: &Path, id: &str) -> Option<String> {
    let bytes = std::fs::read(library_dir.join(id).join(META_FILE)).ok()?;
    let meta: ModEntryMeta = serde_json::from_slice(&bytes).ok()?;
    Some(meta.fingerprint())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mgr::model::{ComponentInfo, ModKind};
    use crate::{record_path, DeployRecord};
    use std::collections::BTreeMap;

    fn le(id: &str, enabled: bool) -> LoadoutEntry {
        LoadoutEntry { id: id.into(), enabled }
    }

    fn loadout(entries: &[(&str, bool)]) -> Loadout {
        Loadout {
            format: 1,
            entries: entries.iter().map(|(id, en)| le(id, *en)).collect(),
        }
    }

    fn write_record(game: &Path, rec: &DeployRecord) {
        std::fs::create_dir_all(game).unwrap();
        std::fs::write(record_path(game), serde_json::to_vec(rec).unwrap()).unwrap();
    }

    /// Build a `ModEntryMeta` for library id `id` carrying `components`. `imported_at` is a
    /// parameter so a test can simulate a re-import (bump it and/or the components to change the
    /// fingerprint) without waiting a real second.
    fn meta(id: &str, imported_at: &str, components: Vec<ComponentInfo>) -> ModEntryMeta {
        ModEntryMeta {
            id: id.into(),
            kind: ModKind::Goremod,
            name: id.into(),
            version: String::new(),
            author: String::new(),
            imported_at: imported_at.into(),
            source: String::new(),
            components,
        }
    }

    /// Write `<lib>/<id>/gore-manager-meta.json` and return the meta's fingerprint (what a matching
    /// deploy record should record for `id`).
    fn write_lib_meta(lib: &Path, m: &ModEntryMeta) -> String {
        let dir = lib.join(&m.id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(META_FILE), serde_json::to_vec_pretty(m).unwrap()).unwrap();
        m.fingerprint()
    }

    /// A one-component library meta whose fingerprint we can vary via `set` (folded into the
    /// component's loc target) — a compact way to make two same-id metas that differ in content.
    fn lib_meta_with(id: &str, set: &str) -> ModEntryMeta {
        meta(
            id,
            "2026-07-03T00:00:00Z",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![format!("itfo_x|{set}")],
            }],
        )
    }

    /// No record → NothingDeployed.
    #[test]
    fn nothing_deployed_without_record() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        std::fs::create_dir_all(&game).unwrap();
        assert_eq!(status(&game, &lib, &Loadout::default()).unwrap(), ManagerStatus::NothingDeployed);
    }

    /// A studio (owner == "") record → StudioDeployActive, carrying the mod name.
    #[test]
    fn studio_record_reports_active() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let rec = DeployRecord { mod_name: "SoloMod".into(), ..Default::default() };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::StudioDeployActive { mod_name: "SoloMod".into() }
        );
    }

    /// A manager record whose loadout equals the target's enabled entries AND whose recorded
    /// fingerprints still match the library → InSync; a disabled target entry is excluded from the
    /// comparison.
    #[test]
    fn in_sync_when_deployed_matches_target_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        // Two enabled mods, each with a library meta; the record snapshots their fingerprints.
        let ma = write_lib_meta(&lib, &lib_meta_with("mod-a", "a"));
        let mb = write_lib_meta(&lib, &lib_meta_with("mod-b", "b"));
        let deployed = vec![le("mod-a", true), le("mod-b", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            deployed_fingerprints: BTreeMap::from([
                ("mod-a".to_string(), ma),
                ("mod-b".to_string(), mb),
            ]),
            ..Default::default()
        };
        write_record(&game, &rec);
        // Target has the same two enabled plus a disabled one (which must be ignored).
        let target = loadout(&[("mod-a", true), ("mod-b", true), ("mod-c", false)]);
        assert_eq!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
        );
    }

    /// A manager record whose loadout differs from the target → ChangesPending with both sides.
    #[test]
    fn changes_pending_when_target_differs() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            ..Default::default()
        };
        write_record(&game, &rec);
        let target = loadout(&[("mod-a", true), ("mod-b", true)]);
        assert_eq!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::ChangesPending {
                deployed,
                target: vec![le("mod-a", true), le("mod-b", true)],
            }
        );
    }

    /// Order matters: same ids in a different mount order is a pending change, not in-sync.
    #[test]
    fn order_sensitive_comparison() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: vec![le("mod-a", true), le("mod-b", true)],
            ..Default::default()
        };
        write_record(&game, &rec);
        let target = loadout(&[("mod-b", true), ("mod-a", true)]);
        assert!(matches!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::ChangesPending { .. }
        ));
    }

    /// A recorded live file whose current bytes no longer match its deployed hash (or is missing)
    /// → GameUpdated listing the drifted paths sorted — even if the loadout still matches.
    #[test]
    fn game_updated_when_deployed_file_drifts() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        // Two files we "deployed"; one will be modified externally, one removed.
        let f_drift = game.join("live_drift.bin");
        let f_gone = game.join("live_gone.bin");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(&f_drift, b"DEPLOYED-BYTES").unwrap();
        std::fs::write(&f_gone, b"ALSO-DEPLOYED").unwrap();
        let mut hashes = BTreeMap::new();
        hashes.insert(f_drift.display().to_string(), crate::content_hash(b"DEPLOYED-BYTES"));
        hashes.insert(f_gone.display().to_string(), crate::content_hash(b"ALSO-DEPLOYED"));
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            deployed_hashes: hashes,
            ..Default::default()
        };
        write_record(&game, &rec);

        // Externally change one file and delete the other → both drift.
        std::fs::write(&f_drift, b"STEAM-UPDATED-THIS").unwrap();
        std::fs::remove_file(&f_gone).unwrap();

        // Loadout still matches, but drift wins (before any fingerprint check).
        let target = loadout(&[("mod-a", true)]);
        let mut expected = vec![f_drift.display().to_string(), f_gone.display().to_string()];
        expected.sort();
        assert_eq!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::GameUpdated { drifted: expected }
        );
    }

    /// An additive-only deployment (a managed pak, no `deployed_hashes`) whose recorded file was
    /// deleted externally → GameUpdated listing that path — NOT InSync. Additive paths carry no
    /// per-file hash, so existence is the only drift signal; without this check a missing managed
    /// pak/ue4ss dir would leave `drifted` empty and Apply would stay disabled while files are gone.
    #[test]
    fn game_updated_when_additive_pak_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        std::fs::create_dir_all(&game).unwrap();
        // A managed pak we "deployed" that is NOT present on disk (deleted by the user / a verify).
        let missing_pak = game.join("G1R/Content/Paks/~mods/zzz_gm000_foo_P.pak");
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            managed_paks: vec![missing_pak.display().to_string()],
            ..Default::default()
        };
        write_record(&game, &rec);

        // Loadout still matches the record, but the additive file is gone → drift wins.
        let target = loadout(&[("mod-a", true)]);
        assert_eq!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::GameUpdated { drifted: vec![missing_pak.display().to_string()] }
        );
    }

    /// A recorded UE4SS mod DIR that no longer exists is drift too (dirs use the same existence
    /// check as additive files).
    #[test]
    fn game_updated_when_ue4ss_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        std::fs::create_dir_all(&game).unwrap();
        let missing_dir = game.join("G1R/Binaries/Win64/ue4ss/Mods/gm000_Foo");
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: vec![le("mod-a", true)],
            ue4ss_mod_dirs: vec![missing_dir.display().to_string()],
            ..Default::default()
        };
        write_record(&game, &rec);
        match status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap() {
            ManagerStatus::GameUpdated { drifted } => {
                assert_eq!(drifted, vec![missing_dir.display().to_string()]);
            }
            other => panic!("expected GameUpdated for a missing ue4ss dir, got {other:?}"),
        }
    }

    /// Additive paths that DO exist do not fire drift — InSync still wins when everything is present.
    #[test]
    fn no_drift_when_additive_paths_present() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let pak = mods.join("zzz_gm000_foo_P.pak");
        std::fs::write(&pak, b"PAK").unwrap();
        // A matching library meta so the fingerprint check also passes.
        let fp = write_lib_meta(&lib, &lib_meta_with("mod-a", "a"));
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            managed_paks: vec![pak.display().to_string()],
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fp)]),
            ..Default::default()
        };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
        );
    }

    /// When every recorded live file still matches its hash, drift does NOT fire — InSync wins.
    #[test]
    fn no_drift_when_files_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("live_ok.bin");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(&live, b"STILL-OURS").unwrap();
        let mut hashes = BTreeMap::new();
        hashes.insert(live.display().to_string(), crate::content_hash(b"STILL-OURS"));
        let fp = write_lib_meta(&lib, &lib_meta_with("mod-a", "a"));
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            deployed_hashes: hashes,
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fp)]),
            ..Default::default()
        };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
        );
    }

    /// BUG focus: a mod re-imported UNDER THE SAME ID (an update) — its library meta now has
    /// different components (and thus a different fingerprint) than the deploy record snapshotted,
    /// while the loadout ids/order are unchanged. Status must report ChangesPending (the deployed
    /// bytes are stale), NOT InSync.
    #[test]
    fn same_id_update_reports_changes_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        // Deploy-time library meta + the fingerprint the record snapshots.
        let old_fp = write_lib_meta(&lib, &lib_meta_with("mod-a", "old"));
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), old_fp.clone())]),
            ..Default::default()
        };
        write_record(&game, &rec);

        // Re-import "mod-a" as an UPDATE: same id, different components → different fingerprint.
        let new_fp = write_lib_meta(&lib, &lib_meta_with("mod-a", "new"));
        assert_ne!(old_fp, new_fp, "precondition: the update must change the fingerprint");

        // Loadout ids/order unchanged, but the content fingerprint differs → ChangesPending.
        let target = loadout(&[("mod-a", true)]);
        assert_eq!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::ChangesPending { deployed, target: vec![le("mod-a", true)] }
        );
    }

    /// The mirror of the above: the library is UNCHANGED since deploy (same fingerprint) and the
    /// loadout matches → InSync. Guards against the fingerprint check falsely firing when nothing
    /// changed.
    #[test]
    fn in_sync_when_fingerprints_match() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let fp = write_lib_meta(&lib, &lib_meta_with("mod-a", "same"));
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fp)]),
            ..Default::default()
        };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
        );
    }

    /// A pre-fingerprint deploy record (empty `deployed_fingerprints`) whose loadout still matches
    /// must report ChangesPending, not InSync: we can't confirm the deployed bytes are current, so
    /// a one-time re-apply is the safe outcome.
    #[test]
    fn missing_recorded_fingerprint_reports_changes_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        write_lib_meta(&lib, &lib_meta_with("mod-a", "a"));
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            // deployed_fingerprints intentionally left empty (old record).
            ..Default::default()
        };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::ChangesPending { deployed, target: vec![le("mod-a", true)] }
        );
    }

    /// A mod whose library meta is now unreadable/removed, but whose id is still in the (matching)
    /// loadout, must report ChangesPending — never panic, never InSync over content we can't read.
    #[test]
    fn unreadable_library_meta_reports_changes_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        // Record a fingerprint for mod-a, but do NOT write its library meta (removed since deploy).
        let deployed = vec![le("mod-a", true)];
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), "deadbeef".to_string())]),
            ..Default::default()
        };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::ChangesPending { deployed, target: vec![le("mod-a", true)] }
        );
    }
}
