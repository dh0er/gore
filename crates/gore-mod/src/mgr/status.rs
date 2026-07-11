//! Declarative manager status: diff what the manager currently has DEPLOYED (from the on-disk
//! deploy record) against a TARGET loadout, so the UI/CLI can show whether a re-apply is needed.
//!
//! The states form a priority ladder — the first that applies wins:
//!   1. no record ....................... [`ManagerStatus::NothingDeployed`]
//!   2. an interrupted apply record ...... [`ManagerStatus::RecoveryRequired`] (recover before any
//!      new apply)
//!   3. a non-manager (studio) record ... [`ManagerStatus::StudioDeployActive`] (manager must not
//!      touch a single-mod studio deployment — apply refuses it too)
//!   4. a deployed live/backup file drifted [`ManagerStatus::GameUpdated`] (Steam verified/updated
//!      a file we wrote, or a pristine backup is no longer trustworthy; the deployment is stale
//!      regardless of the loadout)
//!   5. deployed loadout == target AND every enabled mod's library fingerprint still matches the
//!      one recorded at deploy .......... [`ManagerStatus::InSync`]
//!   6. otherwise ....................... [`ManagerStatus::ChangesPending`] (loadout differs, OR a
//!      same-id mod was re-imported/updated so its content fingerprint no longer matches — the
//!      deployed bytes are stale even though the id set is unchanged)
//!
//! Pure read-only: it reads the record, verifies the recorded live files and pristine backups, and
//! reads each enabled mod's library sidecar to fingerprint it; it never writes.

use std::path::Path;

use serde::Serialize;

use super::loadout::{Loadout, LoadoutEntry};
use super::model::LibraryRoot;

/// The manager's deployment state relative to a target loadout. `#[serde(tag = "state")]` so the
/// UI can switch on a single discriminant field.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ManagerStatus {
    /// No deploy record — nothing is deployed.
    NothingDeployed,
    /// A pre-write record survived an interrupted/failed apply. Only recovery undeploy is safe.
    RecoveryRequired,
    /// A single-mod (studio) deployment is active; the manager won't diff/replace it.
    StudioDeployActive { mod_name: String },
    /// The deployed loadout matches the target; no re-apply needed.
    InSync { loadout: Vec<LoadoutEntry> },
    /// The deployed loadout differs from the target — a re-apply would change the game.
    ChangesPending {
        deployed: Vec<LoadoutEntry>,
        target: Vec<LoadoutEntry>,
    },
    /// One or more files this manager owns were changed externally (e.g. a Steam update or
    /// integrity verify). `drifted` lists those live or backup paths, sorted. The deployment is
    /// stale; re-applying rebuilds against the refreshed game files.
    GameUpdated { drifted: Vec<String> },
}

/// Report the manager's state at `game_root` relative to `target` (see [`ManagerStatus`]).
/// `library_dir` is the mod library root, needed to fingerprint each enabled mod's current
/// on-disk content and compare it to what the deploy record snapshotted — so a same-id UPDATE
/// (a re-import that changed a mod but kept its id) is reported as [`ManagerStatus::ChangesPending`]
/// rather than [`ManagerStatus::InSync`].
pub fn status(
    game_root: &Path,
    library_dir: &Path,
    target: &Loadout,
) -> crate::Result<ManagerStatus> {
    // Match deploy/undeploy's record location logic so status reads the SAME record they wrote,
    // regardless of whether the caller passed the install dir or its `G1R` child, or a relative
    // path from a different cwd.
    let game_root = crate::abs_root(game_root);
    let Some(stored) = crate::read_record(&game_root)? else {
        return Ok(ManagerStatus::NothingDeployed);
    };
    let record = stored.record;

    if record.phase == crate::DeployPhase::RecoveryRequired
        || !record.file_cleanup_claims.is_empty()
        || !record.ue4ss_cleanup_claims.is_empty()
    {
        return Ok(ManagerStatus::RecoveryRequired);
    }

    // A studio (non-manager) deployment is off-limits to the manager: it doesn't own it and can't
    // meaningfully diff a single hand-built bundle against a loadout.
    if record.owner != "manager" {
        return Ok(ManagerStatus::StudioDeployActive {
            mod_name: record.mod_name,
        });
    }

    // Drift beats loadout comparison: if the game changed a file we wrote, the deployment is stale
    // no matter what the loadout says. A missing recorded file counts as drifted (it was our
    // modded file and is now gone/replaced).
    let mut drifted: Vec<String> = record
        .deployed_hashes
        .iter()
        .filter(|(live, expected)| {
            !crate::file_matches_recorded_hash(Path::new(live.as_str()), expected)
        })
        .map(|(live, _)| live.clone())
        .collect();
    for (live, backup, _) in &record.backups {
        let expected = crate::deployed_hash_for_path(live, &record.deployed_hashes);
        if expected
            .is_none_or(|hash| !crate::file_matches_recorded_hash(Path::new(live.as_str()), hash))
        {
            drifted.push(live.clone());
        }

        // Backups are part of the deployment's owned state too: undeploy/re-apply may consume
        // their pristine bytes, so an absent, malformed, or mismatched identity must never be
        // reported InSync. Accept canonical aliases in the identity map for the same reason live
        // identities do (notably Windows `\\?\` versus plain absolute paths).
        //
        // Pre-backup-identity records fail closed. The only safe compatibility case is a regular,
        // non-link live/backup pair with byte-identical content: either copy is then harmless.
        let backup_path = Path::new(backup.as_str());
        let backup_matches = match crate::backup_hash_for_path(backup_path, &record.backup_hashes) {
            Some(hash) => {
                hash.starts_with("sha256:") && crate::file_matches_recorded_hash(backup_path, hash)
            }
            None => match (
                crate::sha256_file(Path::new(live.as_str())),
                crate::sha256_file(backup_path),
            ) {
                (Ok(live_hash), Ok(backup_hash)) => live_hash == backup_hash,
                _ => false,
            },
        };
        if !backup_matches {
            drifted.push(backup.clone());
        }
    }

    // Additive ownership is content-based: every managed pak/triplet has a SHA-256 entry and every
    // active UE4SS directory has a deterministic tree fingerprint. Missing identities in a legacy
    // record are unverifiable and therefore drifted rather than silently adopted by path.
    for path in record
        .managed_paks
        .iter()
        .chain(record.texture_triplets.iter())
    {
        let expected = crate::deployed_hash_for_path(path, &record.deployed_hashes);
        if expected.is_none_or(|hash| {
            !hash.starts_with("sha256:")
                || !crate::file_matches_recorded_hash(Path::new(path.as_str()), hash)
        }) {
            drifted.push(path.clone());
        }
    }
    for path in record
        .ue4ss_mod_dir
        .iter()
        .chain(record.ue4ss_mod_dirs.iter())
    {
        let expected = crate::tree_fingerprint_for_path(
            Path::new(path.as_str()),
            &record.ue4ss_tree_fingerprints,
        );
        if expected.is_none_or(|fingerprint| {
            !crate::tree_matches_recorded_fingerprint(Path::new(path.as_str()), fingerprint)
        }) {
            drifted.push(path.clone());
        }
    }
    for path in &record.stale_ue4ss_dirs {
        let mirrors_active = record
            .ue4ss_mod_dir
            .iter()
            .chain(record.ue4ss_mod_dirs.iter())
            .any(|active| crate::same_path(Path::new(path), active));
        if !mirrors_active && crate::path_exists_no_follow(Path::new(path)) {
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
    let target_enabled: Vec<LoadoutEntry> = target
        .entries
        .iter()
        .filter(|e| e.enabled)
        .cloned()
        .collect();
    if record.loadout != target_enabled {
        return Ok(ManagerStatus::ChangesPending {
            deployed: record.loadout,
            target: target_enabled,
        });
    }

    // Loadout ids/order/enabled all match. But a mod can be re-imported UNDER THE SAME ID (an
    // update) — its components/bytes change while its id (and thus the loadout) does not, so the
    // check above can't see it. Compare each enabled target mod's CURRENT library fingerprint to
    // the one the record snapshotted at deploy: any mismatch means the deployed bytes are stale.
    // A mod whose library sidecar is now missing/unreadable, or that has no recorded fingerprint
    // (a pre-fingerprint record, or a mod added since), counts as changed too — never InSync over
    // content we can't confirm is current.
    let library = match LibraryRoot::open(library_dir) {
        Ok(library) => library,
        Err(_) => {
            return Ok(ManagerStatus::ChangesPending {
                deployed: record.loadout,
                target: target_enabled,
            });
        }
    };
    for e in &target_enabled {
        let current = match read_library_fingerprint(&library, &e.id) {
            Some(fp) => fp,
            // Can't read/parse the library entry → treat as changed (re-apply rebuilds from it).
            None => {
                return Ok(ManagerStatus::ChangesPending {
                    deployed: record.loadout,
                    target: target_enabled,
                });
            }
        };
        if record.deployed_fingerprints.get(&e.id) != Some(&current) {
            return Ok(ManagerStatus::ChangesPending {
                deployed: record.loadout,
                target: target_enabled,
            });
        }
    }

    Ok(ManagerStatus::InSync {
        loadout: record.loadout,
    })
}

/// Read `<library_dir>/<id>/gore-manager-meta.json` and compute its content [`ModEntryMeta::fingerprint`].
/// `None` if the sidecar is missing or unparseable — the caller treats that as "changed" so a
/// removed/corrupt library entry can never leave status reporting InSync over stale deployed bytes.
fn read_library_fingerprint(library: &LibraryRoot, id: &str) -> Option<String> {
    Some(library.entry(id).ok()?.read_meta().ok()?.fingerprint())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mgr::model::{ComponentInfo, ModEntryMeta, ModKind, META_FILE};
    use crate::{record_path, DeployPhase, DeployRecord};
    use std::collections::BTreeMap;

    #[cfg(unix)]
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test directory symlink failed: {error}"),
        }
    }

    fn le(id: &str, enabled: bool) -> LoadoutEntry {
        LoadoutEntry {
            id: id.into(),
            enabled,
        }
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
        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::NothingDeployed
        );
    }

    #[test]
    fn corrupt_record_is_a_hard_status_error() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        std::fs::create_dir_all(&game).unwrap();
        let bytes = b"{ definitely not a deploy record";
        std::fs::write(record_path(&game), bytes).unwrap();

        let error = status(&game, &lib, &Loadout::default()).unwrap_err();
        assert!(
            error.to_string().contains("parsing deploy record"),
            "{error}"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), bytes);
    }

    #[test]
    fn interrupted_record_reports_recovery_required() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        write_record(
            &game,
            &DeployRecord {
                owner: "manager".into(),
                phase: DeployPhase::RecoveryRequired,
                ..Default::default()
            },
        );

        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::RecoveryRequired
        );
    }

    /// A studio (owner == "") record → StudioDeployActive, carrying the mod name.
    #[test]
    fn studio_record_reports_active() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let rec = DeployRecord {
            mod_name: "SoloMod".into(),
            ..Default::default()
        };
        write_record(&game, &rec);
        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::StudioDeployActive {
                mod_name: "SoloMod".into()
            }
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
        let voice = game.join("G1R/Story/VoiceOver");
        let f_drift = voice.join("live_drift.zip");
        let f_gone = voice.join("live_gone.zip");
        std::fs::create_dir_all(&voice).unwrap();
        std::fs::write(&f_drift, b"DEPLOYED-BYTES").unwrap();
        std::fs::write(&f_gone, b"ALSO-DEPLOYED").unwrap();
        let mut hashes = BTreeMap::new();
        hashes.insert(
            f_drift.display().to_string(),
            crate::content_hash(b"DEPLOYED-BYTES"),
        );
        hashes.insert(
            f_gone.display().to_string(),
            crate::content_hash(b"ALSO-DEPLOYED"),
        );
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

    #[test]
    fn legacy_in_place_backup_without_hash_is_drifted_not_in_sync() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/legacy.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = crate::bak_path(&live);
        std::fs::write(&live, b"legacy-modded").unwrap();
        std::fs::write(&backup, b"legacy-pristine").unwrap();
        let record = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: vec![le("mod-a", true)],
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(
                live.display().to_string(),
                crate::content_hash(b"legacy-modded"),
            )]),
            ..Default::default()
        };
        write_record(&game, &record);

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::GameUpdated {
                drifted: vec![backup.display().to_string()]
            }
        );
    }

    #[test]
    fn legacy_identical_backup_without_hash_can_remain_in_sync() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/legacy-identical.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = crate::bak_path(&live);
        std::fs::write(&live, b"identical-bytes").unwrap();
        std::fs::write(&backup, b"identical-bytes").unwrap();
        let fingerprint = write_lib_meta(&lib, &lib_meta_with("mod-a", "same"));
        let deployed = vec![le("mod-a", true)];
        let record = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(
                live.display().to_string(),
                crate::content_hash(b"identical-bytes"),
            )]),
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fingerprint)]),
            ..Default::default()
        };
        write_record(&game, &record);

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
        );
    }

    #[test]
    fn game_updated_when_backup_is_missing_or_mismatched() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let voice = game.join("G1R/Story/VoiceOver");
        std::fs::create_dir_all(&voice).unwrap();

        let missing_live = voice.join("missing.zip");
        let mismatched_live = voice.join("mismatched.zip");
        for live in [&missing_live, &mismatched_live] {
            std::fs::write(live, b"deployed").unwrap();
        }
        let missing_backup = crate::bak_path(&missing_live);
        let mismatched_backup = crate::bak_path(&mismatched_live);
        std::fs::write(&missing_backup, b"pristine").unwrap();
        let missing_hash = crate::sha256_file(&missing_backup).unwrap();
        std::fs::remove_file(&missing_backup).unwrap();
        std::fs::write(&mismatched_backup, b"expected-pristine").unwrap();
        let mismatched_hash = crate::sha256_file(&mismatched_backup).unwrap();
        std::fs::write(&mismatched_backup, b"externally-replaced").unwrap();

        let live_hashes = [&missing_live, &mismatched_live]
            .into_iter()
            .map(|live| (live.display().to_string(), crate::content_hash(b"deployed")))
            .collect();
        let record = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: vec![le("mod-a", true)],
            backups: vec![
                (
                    missing_live.display().to_string(),
                    missing_backup.display().to_string(),
                    true,
                ),
                (
                    mismatched_live.display().to_string(),
                    mismatched_backup.display().to_string(),
                    true,
                ),
            ],
            deployed_hashes: live_hashes,
            backup_hashes: BTreeMap::from([
                (missing_backup.display().to_string(), missing_hash),
                (mismatched_backup.display().to_string(), mismatched_hash),
            ]),
            ..Default::default()
        };
        write_record(&game, &record);

        let mut expected = vec![
            missing_backup.display().to_string(),
            mismatched_backup.display().to_string(),
        ];
        expected.sort();
        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::GameUpdated { drifted: expected }
        );
    }

    #[test]
    fn invalid_backup_identity_is_a_hard_status_error() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/invalid-identity.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = crate::bak_path(&live);
        std::fs::write(&live, b"deployed").unwrap();
        std::fs::write(&backup, b"pristine").unwrap();

        for invalid_hash in [
            crate::content_hash(b"pristine"),
            format!("sha256:{}", "z".repeat(64)),
        ] {
            let record = DeployRecord {
                mod_name: "manager".into(),
                owner: "manager".into(),
                backups: vec![(
                    live.display().to_string(),
                    backup.display().to_string(),
                    true,
                )],
                deployed_hashes: BTreeMap::from([(
                    live.display().to_string(),
                    crate::content_hash(b"deployed"),
                )]),
                backup_hashes: BTreeMap::from([(backup.display().to_string(), invalid_hash)]),
                ..Default::default()
            };
            write_record(&game, &record);

            let error = status(&game, &lib, &Loadout::default()).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("invalid backup SHA-256 identity"),
                "{error}"
            );
        }
    }

    #[test]
    fn exact_and_alias_backup_hash_keys_are_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let voice = game.join("G1R/Story/VoiceOver");
        std::fs::create_dir_all(&voice).unwrap();
        let exact_live = voice.join("exact.zip");
        let alias_live = voice.join("alias.zip");
        let exact_backup = crate::bak_path(&exact_live);
        let alias_backup = crate::bak_path(&alias_live);
        std::fs::write(&exact_live, b"exact-deployed").unwrap();
        std::fs::write(&alias_live, b"alias-deployed").unwrap();
        std::fs::write(&exact_backup, b"exact-pristine").unwrap();
        std::fs::write(&alias_backup, b"alias-pristine").unwrap();
        #[cfg(windows)]
        let alias_key = alias_backup.display().to_string().to_uppercase();
        #[cfg(not(windows))]
        let alias_key = format!(
            "{}//{}",
            alias_backup.parent().unwrap().display(),
            alias_backup.file_name().unwrap().to_string_lossy()
        );
        assert_ne!(alias_key, alias_backup.display().to_string());

        let fingerprint = write_lib_meta(&lib, &lib_meta_with("mod-a", "same"));
        let deployed = vec![le("mod-a", true)];
        let record = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: deployed.clone(),
            backups: vec![
                (
                    exact_live.display().to_string(),
                    exact_backup.display().to_string(),
                    true,
                ),
                (
                    alias_live.display().to_string(),
                    alias_backup.display().to_string(),
                    true,
                ),
            ],
            deployed_hashes: BTreeMap::from([
                (
                    exact_live.display().to_string(),
                    crate::content_hash(b"exact-deployed"),
                ),
                (
                    alias_live.display().to_string(),
                    crate::content_hash(b"alias-deployed"),
                ),
            ]),
            backup_hashes: BTreeMap::from([
                (
                    exact_backup.display().to_string(),
                    crate::sha256_file(&exact_backup).unwrap(),
                ),
                (alias_key, crate::sha256_file(&alias_backup).unwrap()),
            ]),
            deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fingerprint)]),
            ..Default::default()
        };
        write_record(&game, &record);

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
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
            ManagerStatus::GameUpdated {
                drifted: vec![missing_pak.display().to_string()]
            }
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
    fn game_updated_when_additive_content_is_replaced_at_same_path() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let pak = game.join("G1R/Content/Paks/~mods/owned.pak");
        std::fs::create_dir_all(pak.parent().unwrap()).unwrap();
        std::fs::write(&pak, b"deployed").unwrap();
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: vec![le("mod-a", true)],
            managed_paks: vec![pak.display().to_string()],
            deployed_hashes: BTreeMap::from([(
                pak.display().to_string(),
                crate::sha256_file(&pak).unwrap(),
            )]),
            ..Default::default()
        };
        write_record(&game, &rec);
        std::fs::write(&pak, b"external replacement").unwrap();

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::GameUpdated {
                drifted: vec![pak.display().to_string()]
            }
        );
    }

    #[test]
    fn game_updated_when_ue4ss_tree_changes_at_same_path() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let tree = game.join("G1R/Binaries/Win64/ue4ss/Mods/Owned");
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::write(tree.join("deployed.txt"), b"deployed").unwrap();
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            loadout: vec![le("mod-a", true)],
            ue4ss_mod_dirs: vec![tree.display().to_string()],
            ue4ss_tree_fingerprints: BTreeMap::from([(
                tree.display().to_string(),
                crate::tree_fingerprint(&tree).unwrap(),
            )]),
            ..Default::default()
        };
        write_record(&game, &rec);
        std::fs::write(tree.join("external.txt"), b"external").unwrap();

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::GameUpdated {
                drifted: vec![tree.display().to_string()]
            }
        );
    }

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
            deployed_hashes: BTreeMap::from([(
                pak.display().to_string(),
                crate::sha256_file(&pak).unwrap(),
            )]),
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
        let live = game.join("G1R/Story/VoiceOver/live_ok.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"STILL-OURS").unwrap();
        let mut hashes = BTreeMap::new();
        hashes.insert(
            live.display().to_string(),
            crate::content_hash(b"STILL-OURS"),
        );
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
        assert_ne!(
            old_fp, new_fp,
            "precondition: the update must change the fingerprint"
        );

        // Loadout ids/order unchanged, but the content fingerprint differs → ChangesPending.
        let target = loadout(&[("mod-a", true)]);
        assert_eq!(
            status(&game, &lib, &target).unwrap(),
            ManagerStatus::ChangesPending {
                deployed,
                target: vec![le("mod-a", true)]
            }
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

    #[cfg(any(unix, windows))]
    #[test]
    fn linked_library_entry_cannot_supply_an_in_sync_fingerprint() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&lib).unwrap();
        let meta = lib_meta_with("mod-a", "same");
        let fingerprint = write_lib_meta(&outside, &meta);
        if !make_dir_link(&outside.join("mod-a"), &lib.join("mod-a")) {
            return;
        }

        let deployed = vec![le("mod-a", true)];
        write_record(
            &game,
            &DeployRecord {
                mod_name: "manager".into(),
                owner: "manager".into(),
                loadout: deployed.clone(),
                deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fingerprint)]),
                ..Default::default()
            },
        );

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::ChangesPending {
                deployed,
                target: vec![le("mod-a", true)]
            }
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
            ManagerStatus::ChangesPending {
                deployed,
                target: vec![le("mod-a", true)]
            }
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
            ManagerStatus::ChangesPending {
                deployed,
                target: vec![le("mod-a", true)]
            }
        );
    }
}
