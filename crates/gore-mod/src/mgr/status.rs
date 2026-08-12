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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::Serialize;

use super::loadout::{Loadout, LoadoutEntry};
use super::model::LibraryRoot;

const MAX_PREFLIGHT_STATUS_META_BYTES: u64 = 16 * 1024 * 1024;
// One synchronous status snapshot shares these ceilings across every recorded live file, backup,
// and UE4SS tree. Successful exact-path reads are cached below, so duplicate record entries are
// evidence aliases rather than repeated I/O.
// Preflight admits a supported 16-GiB voice archive plus its equally-sized pristine backup with
// room for the rest of the deployment. Public `status` retains its pre-existing no-total-ceiling
// semantics while still benefiting from duplicate-read caching and growth-bounded individual I/O.
const MAX_PREFLIGHT_STATUS_HASH_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_PREFLIGHT_STATUS_HASH_ENTRIES: u64 = 250_000;
const MAX_PREFLIGHT_STATUS_TREE_ENTRIES: u64 = 250_000;
const MAX_PREFLIGHT_STATUS_PATH_ENTRIES: u64 = 250_000;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectionFailurePolicy {
    TreatAsDrift,
    Preserve,
}

fn metadata_budget_for_status(failure_policy: InspectionFailurePolicy) -> u64 {
    match failure_policy {
        InspectionFailurePolicy::TreatAsDrift => u64::MAX,
        InspectionFailurePolicy::Preserve => MAX_PREFLIGHT_STATUS_META_BYTES,
    }
}

struct DeploymentInspection {
    remaining_hash_bytes: u64,
    remaining_hash_entries: u64,
    remaining_tree_entries: u64,
    remaining_path_entries: u64,
    file_matches: HashMap<(String, String), bool>,
    legacy_hashes: HashMap<String, String>,
    tree_fingerprints: HashMap<String, String>,
    sha256: HashMap<String, String>,
}

impl DeploymentInspection {
    fn new(
        remaining_hash_bytes: u64,
        remaining_hash_entries: u64,
        remaining_tree_entries: u64,
        remaining_path_entries: u64,
    ) -> Self {
        Self {
            remaining_hash_bytes,
            remaining_hash_entries,
            remaining_tree_entries,
            remaining_path_entries,
            file_matches: HashMap::new(),
            legacy_hashes: HashMap::new(),
            tree_fingerprints: HashMap::new(),
            sha256: HashMap::new(),
        }
    }

    fn charge_file_hash(&mut self) -> crate::Result<()> {
        if self.remaining_hash_entries == 0 {
            return Err(crate::ModError::InspectionBound(
                "deployment inspection exhausted its file-hash budget".into(),
            ));
        }
        self.remaining_hash_entries -= 1;
        Ok(())
    }

    fn deployment_path_key(&mut self, path: &Path) -> crate::Result<String> {
        if self.remaining_path_entries == 0 {
            return Err(crate::ModError::InspectionBound(
                "deployment inspection exhausted its path-entry budget".into(),
            ));
        }
        self.remaining_path_entries -= 1;

        // Preserve the old `same_path` semantics by resolving existing aliases, but do so only
        // once per budgeted record entry instead of once per active/stale pair. Missing or
        // inaccessible paths still get a component-normalized lexical key.
        let canonical = std::fs::canonicalize(path).ok();
        let normalized: std::path::PathBuf = canonical
            .as_deref()
            .unwrap_or(path)
            .components()
            .collect();
        Ok(crate::record_path_key(&normalized))
    }
}

#[derive(Debug)]
struct Ue4ssPathIndex<'a> {
    active_keys: HashSet<String>,
    active: Vec<(&'a str, String)>,
    stale: Vec<(&'a str, String)>,
    fingerprints: HashMap<String, &'a str>,
}

fn ue4ss_path_index<'a>(
    record: &'a crate::DeployRecord,
    inspection: &mut DeploymentInspection,
) -> crate::Result<Ue4ssPathIndex<'a>> {
    let mut active_keys = HashSet::new();
    let mut active = Vec::new();
    for path in record
        .ue4ss_mod_dir
        .iter()
        .chain(record.ue4ss_mod_dirs.iter())
    {
        let key = inspection.deployment_path_key(Path::new(path))?;
        if active_keys.insert(key.clone()) {
            active.push((path.as_str(), key));
        }
    }

    let mut fingerprints = HashMap::new();
    for (path, fingerprint) in &record.ue4ss_tree_fingerprints {
        let key = inspection.deployment_path_key(Path::new(path))?;
        // BTreeMap iteration is stable; retaining the first alias preserves the old
        // `tree_fingerprint_for_path` lookup order while making subsequent reads constant-time.
        fingerprints.entry(key).or_insert(fingerprint.as_str());
    }

    let mut seen_stale = HashSet::new();
    let mut stale = Vec::new();
    for path in &record.stale_ue4ss_dirs {
        let key = inspection.deployment_path_key(Path::new(path))?;
        if seen_stale.insert(key.clone()) {
            stale.push((path.as_str(), key));
        }
    }
    Ok(Ue4ssPathIndex {
        active_keys,
        active,
        stale,
        fingerprints,
    })
}

fn metadata_for_status(path: &Path) -> crate::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::ModError::Io(format!(
            "reading deployment path metadata {}: {error}",
            path.display()
        ))),
    }
}

fn path_exists_for_status(
    path: &Path,
    failure_policy: InspectionFailurePolicy,
) -> crate::Result<bool> {
    if failure_policy == InspectionFailurePolicy::TreatAsDrift {
        return Ok(crate::path_exists_no_follow(path));
    }
    Ok(metadata_for_status(path)?.is_some())
}

fn file_matches_for_status(
    path: &Path,
    expected: &str,
    failure_policy: InspectionFailurePolicy,
    inspection: &mut DeploymentInspection,
) -> crate::Result<bool> {
    let cache_key = (crate::record_path_key(path), expected.to_owned());
    if let Some(cached) = inspection.file_matches.get(&cache_key) {
        return Ok(*cached);
    }
    let sha256_prefixed = expected.starts_with("sha256:");
    let sha256_expected = expected.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    let legacy_expected = expected.len() == 16
        && expected
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    let matches = if failure_policy == InspectionFailurePolicy::TreatAsDrift {
        if sha256_expected {
            sha256_for_status(path, failure_policy, inspection)?
                .as_deref()
                == Some(expected)
        } else if sha256_prefixed || !legacy_expected {
            false
        } else {
            legacy_hash_for_status(path, failure_policy, inspection)?.as_deref() == Some(expected)
        }
    } else {
        let Some(metadata) = metadata_for_status(path)? else {
            inspection.file_matches.insert(cache_key, false);
            return Ok(false);
        };
        if crate::metadata_is_link(&metadata) || !metadata.is_file() {
            false
        } else if sha256_expected {
            sha256_for_status(path, failure_policy, inspection)?
                .as_deref()
                == Some(expected)
        } else if sha256_prefixed {
            return Err(crate::ModError::Other(format!(
                "invalid recorded SHA-256 identity for {}",
                path.display()
            )));
        } else if !legacy_expected {
            return Err(crate::ModError::Other(format!(
                "invalid recorded legacy content hash for {}",
                path.display()
            )));
        } else {
            legacy_hash_for_status(path, failure_policy, inspection)?.as_deref() == Some(expected)
        }
    };
    inspection.file_matches.insert(cache_key, matches);
    Ok(matches)
}

fn tree_matches_for_status(
    path: &Path,
    expected: &str,
    failure_policy: InspectionFailurePolicy,
    inspection: &mut DeploymentInspection,
) -> crate::Result<bool> {
    if failure_policy == InspectionFailurePolicy::Preserve {
        let Some(metadata) = metadata_for_status(path)? else {
            return Ok(false);
        };
        if crate::metadata_is_link(&metadata) || !metadata.is_dir() {
            return Ok(false);
        }
        if !crate::valid_sha256_identity(expected) {
            return Err(crate::ModError::Other(format!(
                "invalid recorded UE4SS tree SHA-256 identity for {}",
                path.display()
            )));
        }
    }
    let cache_key = crate::record_path_key(path);
    if let Some(cached) = inspection.tree_fingerprints.get(&cache_key) {
        return Ok(cached == expected);
    }
    let current = if failure_policy == InspectionFailurePolicy::TreatAsDrift {
        let Ok(current) = crate::tree_fingerprint_bounded(
            path,
            &mut inspection.remaining_hash_bytes,
            &mut inspection.remaining_tree_entries,
        ) else {
            return Ok(false);
        };
        current
    } else {
        crate::tree_fingerprint_bounded(
            path,
            &mut inspection.remaining_hash_bytes,
            &mut inspection.remaining_tree_entries,
        )?
    };
    let matches = current == expected;
    inspection.tree_fingerprints.insert(cache_key, current);
    Ok(matches)
}

fn sha256_for_status(
    path: &Path,
    failure_policy: InspectionFailurePolicy,
    inspection: &mut DeploymentInspection,
) -> crate::Result<Option<String>> {
    let cache_key = crate::record_path_key(path);
    if let Some(cached) = inspection.sha256.get(&cache_key) {
        return Ok(Some(cached.clone()));
    }
    if let Err(error) = inspection.charge_file_hash() {
        return match failure_policy {
            InspectionFailurePolicy::TreatAsDrift => Ok(None),
            InspectionFailurePolicy::Preserve => Err(error),
        };
    }
    let hashed = crate::sha256_file_bounded(path, &mut inspection.remaining_hash_bytes);
    let hashed = match (failure_policy, hashed) {
        (_, Ok(hashed)) => hashed,
        (InspectionFailurePolicy::TreatAsDrift, Err(_)) => return Ok(None),
        (InspectionFailurePolicy::Preserve, Err(error)) => return Err(error),
    };
    inspection.sha256.insert(cache_key, hashed.clone());
    Ok(Some(hashed))
}

fn legacy_hash_for_status(
    path: &Path,
    failure_policy: InspectionFailurePolicy,
    inspection: &mut DeploymentInspection,
) -> crate::Result<Option<String>> {
    let cache_key = crate::record_path_key(path);
    if let Some(cached) = inspection.legacy_hashes.get(&cache_key) {
        return Ok(Some(cached.clone()));
    }
    if let Err(error) = inspection.charge_file_hash() {
        return match failure_policy {
            InspectionFailurePolicy::TreatAsDrift => Ok(None),
            InspectionFailurePolicy::Preserve => Err(error),
        };
    }
    let hashed = crate::content_hash_file_bounded(path, &mut inspection.remaining_hash_bytes);
    let hashed = match (failure_policy, hashed) {
        (_, Ok(hashed)) => hashed,
        (InspectionFailurePolicy::TreatAsDrift, Err(_)) => return Ok(None),
        (InspectionFailurePolicy::Preserve, Err(error)) => {
            return match crate::inspection_bound_from_io(&error) {
                Some(message) => Err(crate::ModError::InspectionBound(message)),
                None => Err(crate::io(&format!(
                    "hashing deployed file {}",
                    path.display()
                ))(error)),
            }
        }
    };
    inspection
        .legacy_hashes
        .insert(cache_key, hashed.clone());
    Ok(Some(hashed))
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
    status_with_failure_policy(
        game_root,
        library_dir,
        target,
        InspectionFailurePolicy::TreatAsDrift,
    )
}

/// Preflight must distinguish confirmed drift (including a missing owned path) from evidence it
/// could not inspect. The public status command retains its conservative drift contract.
pub(super) fn status_for_preflight(
    game_root: &Path,
    library_dir: &Path,
    target: &Loadout,
) -> crate::Result<ManagerStatus> {
    status_with_failure_policy(
        game_root,
        library_dir,
        target,
        InspectionFailurePolicy::Preserve,
    )
}

fn status_with_failure_policy(
    game_root: &Path,
    library_dir: &Path,
    target: &Loadout,
    failure_policy: InspectionFailurePolicy,
) -> crate::Result<ManagerStatus> {
    // Match deploy/undeploy's record location logic so status reads the SAME record they wrote,
    // regardless of whether the caller passed the install dir or its `G1R` child, or a relative
    // path from a different cwd.
    let game_root = crate::abs_root(game_root);
    let Some(stored) = crate::read_record(&game_root)? else {
        return Ok(ManagerStatus::NothingDeployed);
    };
    let record = stored.record;

    let recovery_required = record.phase == crate::DeployPhase::RecoveryRequired
        || !record.file_cleanup_claims.is_empty()
        || !record.ue4ss_cleanup_claims.is_empty();
    if failure_policy == InspectionFailurePolicy::Preserve {
        crate::validate_record_identities(&record)?;
    }
    if recovery_required {
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
    let mut inspection = match failure_policy {
        InspectionFailurePolicy::TreatAsDrift => {
            DeploymentInspection::new(u64::MAX, u64::MAX, u64::MAX, u64::MAX)
        }
        InspectionFailurePolicy::Preserve => DeploymentInspection::new(
            MAX_PREFLIGHT_STATUS_HASH_BYTES,
            MAX_PREFLIGHT_STATUS_HASH_ENTRIES,
            MAX_PREFLIGHT_STATUS_TREE_ENTRIES,
            MAX_PREFLIGHT_STATUS_PATH_ENTRIES,
        ),
    };
    let mut drifted = Vec::new();
    for (live, expected) in &record.deployed_hashes {
        if !file_matches_for_status(
            Path::new(live.as_str()),
            expected,
            failure_policy,
            &mut inspection,
        )? {
            drifted.push(live.clone());
        }
    }
    for (live, backup, _) in &record.backups {
        let expected = crate::deployed_hash_for_path(live, &record.deployed_hashes);
        let live_matches = match expected {
            Some(hash) => file_matches_for_status(
                Path::new(live.as_str()),
                hash,
                failure_policy,
                &mut inspection,
            )?,
            None => false,
        };
        if !live_matches {
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
                hash.starts_with("sha256:")
                    && file_matches_for_status(
                        backup_path,
                        hash,
                        failure_policy,
                        &mut inspection,
                    )?
            }
            None if failure_policy == InspectionFailurePolicy::Preserve => {
                let live_path = Path::new(live.as_str());
                let live_metadata = metadata_for_status(live_path)?;
                let backup_metadata = metadata_for_status(backup_path)?;
                let live_is_file = live_metadata.as_ref().is_some_and(|metadata| {
                    !crate::metadata_is_link(metadata) && metadata.is_file()
                });
                let backup_is_file = backup_metadata.as_ref().is_some_and(|metadata| {
                    !crate::metadata_is_link(metadata) && metadata.is_file()
                });
                if !live_is_file || !backup_is_file {
                    false
                } else {
                    let live_hash =
                        sha256_for_status(live_path, failure_policy, &mut inspection)?;
                    let backup_hash =
                        sha256_for_status(backup_path, failure_policy, &mut inspection)?;
                    live_hash.is_some() && live_hash == backup_hash
                }
            }
            None => {
                let live_hash = sha256_for_status(
                    Path::new(live.as_str()),
                    failure_policy,
                    &mut inspection,
                )?;
                let backup_hash =
                    sha256_for_status(backup_path, failure_policy, &mut inspection)?;
                live_hash.is_some() && live_hash == backup_hash
            }
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
        let matches = match expected {
            Some(hash) if hash.starts_with("sha256:") => {
                file_matches_for_status(
                    Path::new(path.as_str()),
                    hash,
                    failure_policy,
                    &mut inspection,
                )?
            }
            _ => false,
        };
        if !matches {
            drifted.push(path.clone());
        }
    }
    let Ue4ssPathIndex {
        active_keys: active_ue4ss_paths,
        active: active_ue4ss_entries,
        stale: stale_ue4ss_paths,
        fingerprints: ue4ss_fingerprints,
    } = ue4ss_path_index(&record, &mut inspection)?;
    for (path, key) in active_ue4ss_entries {
        let expected = ue4ss_fingerprints.get(&key).copied();
        let matches = match expected {
            Some(fingerprint) => {
                tree_matches_for_status(
                    Path::new(path),
                    fingerprint,
                    failure_policy,
                    &mut inspection,
                )?
            }
            None => false,
        };
        if !matches {
            drifted.push(path.to_owned());
        }
    }
    for (path, key) in stale_ue4ss_paths {
        if !active_ue4ss_paths.contains(&key)
            && path_exists_for_status(Path::new(path), failure_policy)?
        {
            drifted.push(path.to_owned());
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
    let mut remaining_meta_bytes = metadata_budget_for_status(failure_policy);
    if !library_fingerprints_match(
        &library,
        &target_enabled,
        &record.deployed_fingerprints,
        &mut remaining_meta_bytes,
    ) {
        return Ok(ManagerStatus::ChangesPending {
            deployed: record.loadout,
            target: target_enabled,
        });
    }

    Ok(ManagerStatus::InSync {
        loadout: record.loadout,
    })
}

/// Read `<library_dir>/<id>/gore-manager-meta.json` and compute its content [`ModEntryMeta::fingerprint`].
/// `None` if the sidecar is missing or unparseable — the caller treats that as "changed" so a
/// removed/corrupt library entry can never leave status reporting InSync over stale deployed bytes.
fn library_fingerprints_match(
    library: &LibraryRoot,
    target: &[LoadoutEntry],
    deployed: &BTreeMap<String, String>,
    remaining_meta_bytes: &mut u64,
) -> bool {
    let mut inspected_ids = HashSet::new();
    for entry in target {
        if !inspected_ids.insert(entry.id.as_str()) {
            continue;
        }
        let Some(current) =
            read_library_fingerprint(library, &entry.id, remaining_meta_bytes)
        else {
            return false;
        };
        if deployed.get(&entry.id) != Some(&current) {
            return false;
        }
    }
    true
}

fn read_library_fingerprint(
    library: &LibraryRoot,
    id: &str,
    remaining_meta_bytes: &mut u64,
) -> Option<String> {
    Some(
        library
            .entry(id)
            .ok()?
            .read_meta_bounded(remaining_meta_bytes)
            .ok()?
            .fingerprint(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_tempfile as tempfile;
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

    #[test]
    fn preflight_rejects_malformed_recovery_identities() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/owned.zip");
        let tree = game.join("G1R/Binaries/Win64/ue4ss/Mods/Owned");
        let cases = [
            DeployRecord {
                deployed_hashes: BTreeMap::from([(live.display().to_string(), "malformed".into())]),
                ..Default::default()
            },
            DeployRecord {
                recovery_file_hashes: BTreeMap::from([(
                    live.display().to_string(),
                    vec!["malformed".into()],
                )]),
                ..Default::default()
            },
            DeployRecord {
                recovery_file_hashes: BTreeMap::from([(live.display().to_string(), Vec::new())]),
                ..Default::default()
            },
            DeployRecord {
                ue4ss_mod_dirs: vec![tree.display().to_string()],
                ue4ss_tree_fingerprints: BTreeMap::from([(
                    tree.display().to_string(),
                    "malformed".into(),
                )]),
                ..Default::default()
            },
            DeployRecord {
                ue4ss_mod_dirs: vec![tree.display().to_string()],
                recovery_tree_fingerprints: BTreeMap::from([(
                    tree.display().to_string(),
                    vec!["malformed".into()],
                )]),
                ..Default::default()
            },
            DeployRecord {
                ue4ss_mod_dirs: vec![tree.display().to_string()],
                recovery_tree_fingerprints: BTreeMap::from([(
                    tree.display().to_string(),
                    Vec::new(),
                )]),
                ..Default::default()
            },
        ];

        for mut record in cases {
            record.mod_name = "manager".into();
            record.owner = "manager".into();
            record.phase = DeployPhase::RecoveryRequired;
            write_record(&game, &record);
            assert_eq!(
                status(&game, &lib, &Loadout::default()).unwrap(),
                ManagerStatus::RecoveryRequired
            );
            assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());
        }
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

    #[test]
    fn preflight_rejects_malformed_studio_ownership_before_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/owned.zip");
        let record = DeployRecord {
            mod_name: "SoloMod".into(),
            deployed_hashes: BTreeMap::from([(live.display().to_string(), "malformed".into())]),
            ..Default::default()
        };
        write_record(&game, &record);
        let before = std::fs::read(record_path(&game)).unwrap();

        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::StudioDeployActive {
                mod_name: "SoloMod".into()
            }
        );
        let error = status_for_preflight(&game, &lib, &Loadout::default()).unwrap_err();
        assert!(
            error.to_string().contains("invalid deployed file identity"),
            "{error}"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), before);
    }

    #[test]
    fn preflight_rejects_noncanonical_studio_backup_before_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/owned.zip");
        let backup = crate::bak_path(&live);
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"deployed").unwrap();
        std::fs::write(&backup, b"pristine").unwrap();
        let canonical = crate::sha256_file(&backup).unwrap();
        let noncanonical = format!(
            "sha256:{}",
            canonical
                .strip_prefix("sha256:")
                .unwrap()
                .to_ascii_uppercase()
        );
        assert_ne!(noncanonical, canonical);
        let record = DeployRecord {
            mod_name: "SoloMod".into(),
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(
                live.display().to_string(),
                crate::content_hash(b"deployed"),
            )]),
            backup_hashes: BTreeMap::from([(backup.display().to_string(), noncanonical)]),
            ..Default::default()
        };
        write_record(&game, &record);
        let before = std::fs::read(record_path(&game)).unwrap();

        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::StudioDeployActive {
                mod_name: "SoloMod".into()
            }
        );
        let error = status_for_preflight(&game, &lib, &Loadout::default()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid backup SHA-256 identity"),
            "{error}"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), before);
    }

    #[test]
    fn preflight_rejects_noncanonical_manager_ownership_before_reapply() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/owned.zip");
        let backup = crate::bak_path(&live);
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"deployed").unwrap();
        std::fs::write(&backup, b"pristine").unwrap();
        let live_identity = crate::content_hash(b"deployed");
        let noncanonical_live = live_identity.to_ascii_uppercase();
        assert_ne!(noncanonical_live, live_identity);
        let backup_identity = crate::sha256_file(&backup).unwrap();
        let noncanonical_backup = format!(
            "sha256:{}",
            backup_identity
                .strip_prefix("sha256:")
                .unwrap()
                .to_ascii_uppercase()
        );
        assert_ne!(noncanonical_backup, backup_identity);
        let record = |deployed, backup_hash| DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(live.display().to_string(), deployed)]),
            backup_hashes: BTreeMap::from([(backup.display().to_string(), backup_hash)]),
            ..Default::default()
        };
        let cases = [
            record(noncanonical_live, backup_identity),
            record(live_identity, noncanonical_backup),
        ];

        for record in cases {
            write_record(&game, &record);
            let before = std::fs::read(record_path(&game)).unwrap();
            assert!(matches!(
                status(&game, &lib, &Loadout::default()).unwrap(),
                ManagerStatus::GameUpdated { .. }
            ));
            assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());
            assert_eq!(std::fs::read(record_path(&game)).unwrap(), before);
        }
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
    fn preflight_status_preserves_ownership_inspection_failures() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let live = game.join("G1R/Story/VoiceOver/owned.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"deployed").unwrap();
        let record = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            deployed_hashes: BTreeMap::from([(live.display().to_string(), "malformed".into())]),
            ..Default::default()
        };
        write_record(&game, &record);

        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::GameUpdated {
                drifted: vec![live.display().to_string()]
            }
        );
        assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());

        std::fs::remove_file(&live).unwrap();
        assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());

        std::fs::create_dir(&live).unwrap();
        assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());
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
    fn preflight_status_preserves_invalid_ue4ss_tree_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let tree = game.join("G1R/Binaries/Win64/ue4ss/Mods/Owned");
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::write(tree.join("deployed.txt"), b"deployed").unwrap();
        let record = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            ue4ss_mod_dirs: vec![tree.display().to_string()],
            ue4ss_tree_fingerprints: BTreeMap::from([(
                tree.display().to_string(),
                "malformed".into(),
            )]),
            ..Default::default()
        };
        write_record(&game, &record);

        assert_eq!(
            status(&game, &lib, &Loadout::default()).unwrap(),
            ManagerStatus::GameUpdated {
                drifted: vec![tree.display().to_string()]
            }
        );
        assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());

        std::fs::remove_dir_all(&tree).unwrap();
        assert!(status_for_preflight(&game, &lib, &Loadout::default()).is_err());
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

    #[test]
    fn status_ignores_malformed_future_manager_private_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let lib = tmp.path().join("lib");
        let public = lib_meta_with("mod-a", "same");
        let fp = write_lib_meta(&lib, &public);
        let sidecar = lib.join("mod-a").join(META_FILE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
        value["_manager"] = serde_json::json!({"future_private_format": 2});
        std::fs::write(&sidecar, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let deployed = vec![le("mod-a", true)];
        write_record(
            &game,
            &DeployRecord {
                mod_name: "manager".into(),
                owner: "manager".into(),
                loadout: deployed.clone(),
                deployed_fingerprints: BTreeMap::from([("mod-a".to_string(), fp)]),
                ..Default::default()
            },
        );

        assert_eq!(
            status(&game, &lib, &loadout(&[("mod-a", true)])).unwrap(),
            ManagerStatus::InSync { loadout: deployed }
        );
    }

    #[test]
    fn fingerprint_pass_has_an_aggregate_budget_and_deduplicates_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let alpha = lib_meta_with("alpha", "same");
        let beta = lib_meta_with("beta", "same");
        let alpha_fp = write_lib_meta(&lib, &alpha);
        let beta_fp = write_lib_meta(&lib, &beta);
        let sidecar_len = |id: &str| {
            std::fs::metadata(lib.join(id).join(META_FILE))
                .unwrap()
                .len()
        };
        let alpha_len = sidecar_len("alpha");
        let beta_len = sidecar_len("beta");
        let library = LibraryRoot::open(&lib).unwrap();
        let deployed = BTreeMap::from([
            ("alpha".to_string(), alpha_fp),
            ("beta".to_string(), beta_fp),
        ]);

        let mut duplicate_budget = alpha_len;
        assert!(library_fingerprints_match(
            &library,
            &[le("alpha", true), le("alpha", true)],
            &deployed,
            &mut duplicate_budget,
        ));
        assert_eq!(duplicate_budget, 0);

        let distinct = [le("alpha", true), le("beta", true)];
        let mut short_budget = alpha_len + beta_len - 1;
        assert!(!library_fingerprints_match(
            &library,
            &distinct,
            &deployed,
            &mut short_budget,
        ));

        let mut exact_budget = alpha_len + beta_len;
        assert!(library_fingerprints_match(
            &library,
            &distinct,
            &deployed,
            &mut exact_budget,
        ));
        assert_eq!(exact_budget, 0);
    }

    #[test]
    fn metadata_budget_preserves_public_status_and_bounds_preflight() {
        assert_eq!(
            metadata_budget_for_status(InspectionFailurePolicy::TreatAsDrift),
            u64::MAX
        );
        assert_eq!(
            metadata_budget_for_status(InspectionFailurePolicy::Preserve),
            MAX_PREFLIGHT_STATUS_META_BYTES
        );
    }

    #[test]
    fn ue4ss_path_index_deduplicates_aliases_and_bounds_linear_work() {
        let tmp = tempfile::tempdir().unwrap();
        let active = tmp.path().join("game/G1R/Binaries/Win64/ue4ss/Mods/Active");
        let stale = tmp.path().join("game/G1R/Binaries/Win64/ue4ss/Mods/Stale");
        let active_alias = format!("{}{}", active.display(), std::path::MAIN_SEPARATOR);
        let stale_alias = format!("{}{}", stale.display(), std::path::MAIN_SEPARATOR);
        let record = DeployRecord {
            ue4ss_mod_dir: Some(active.display().to_string()),
            ue4ss_mod_dirs: vec![active_alias.clone()],
            stale_ue4ss_dirs: vec![active_alias, stale.display().to_string(), stale_alias],
            ue4ss_tree_fingerprints: BTreeMap::from([
                (active.display().to_string(), "same".into()),
                (
                    format!("{}{}", active.display(), std::path::MAIN_SEPARATOR),
                    "same".into(),
                ),
            ]),
            ..Default::default()
        };

        // Two active, two fingerprints, and three stale entries consume seven units, regardless
        // of the number of possible pairs. Normalized aliases retain only one key apiece.
        let mut exact = DeploymentInspection::new(0, 0, 0, 7);
        let Ue4ssPathIndex {
            active_keys,
            active: active_paths,
            stale: stale_paths,
            fingerprints,
        } = ue4ss_path_index(&record, &mut exact).unwrap();
        assert_eq!(exact.remaining_path_entries, 0);
        assert_eq!(active_keys.len(), 1);
        assert_eq!(active_paths.len(), 1);
        assert_eq!(stale_paths.len(), 2);
        assert_eq!(fingerprints.len(), 1);
        assert!(active_keys.contains(&stale_paths[0].1));
        assert!(!active_keys.contains(&stale_paths[1].1));

        let mut short = DeploymentInspection::new(0, 0, 0, 6);
        let error = ue4ss_path_index(&record, &mut short).unwrap_err();
        assert!(matches!(&error, crate::ModError::InspectionBound(_)));
        assert!(error.to_string().contains("path-entry budget"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn ue4ss_path_index_preserves_existing_short_name_aliases() {
        use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

        fn short_path(path: &Path) -> Option<std::path::PathBuf> {
            let input: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let required = unsafe { GetShortPathNameW(input.as_ptr(), std::ptr::null_mut(), 0) };
            if required == 0 {
                return None;
            }
            let mut output = vec![0u16; required as usize];
            let written = unsafe {
                GetShortPathNameW(input.as_ptr(), output.as_mut_ptr(), output.len() as u32)
            };
            if written == 0 || written as usize >= output.len() {
                return None;
            }
            output.truncate(written as usize);
            Some(std::ffi::OsString::from_wide(&output).into())
        }

        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let library = tmp.path().join("library");
        let active = game.join(
            "G1R/Binaries/Win64/ue4ss/Mods/A deliberately long UE4SS directory name",
        );
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&library).unwrap();
        let Some(short_name) =
            short_path(&active).and_then(|path| path.file_name().map(|name| name.to_os_string()))
        else {
            return;
        };
        let short_alias = active.parent().unwrap().join(short_name);
        if crate::record_path_key(&active) == crate::record_path_key(&short_alias) {
            return;
        }
        assert!(short_alias.is_dir());

        write_record(
            &game,
            &DeployRecord {
                mod_name: "manager".into(),
                owner: "manager".into(),
                ue4ss_mod_dirs: vec![active.display().to_string()],
                stale_ue4ss_dirs: vec![short_alias.display().to_string()],
                ue4ss_tree_fingerprints: BTreeMap::from([(
                    short_alias.display().to_string(),
                    crate::tree_fingerprint(&active).unwrap(),
                )]),
                ..Default::default()
            },
        );

        let expected = ManagerStatus::InSync {
            loadout: Vec::new(),
        };
        assert_eq!(status(&game, &library, &Loadout::default()).unwrap(), expected);
        assert_eq!(
            status_for_preflight(&game, &library, &Loadout::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn deployment_hashing_is_cached_and_aggregate_bounded() {
        assert!(
            MAX_PREFLIGHT_STATUS_HASH_BYTES
                >= gore_vo::Limits::default().max_archive_bytes * 2
        );
        let tmp = tempfile::tempdir().unwrap();
        let alpha = tmp.path().join("alpha.bin");
        let beta = tmp.path().join("beta.bin");
        std::fs::write(&alpha, b"alpha").unwrap();
        std::fs::write(&beta, b"beta").unwrap();
        let alpha_hash = crate::sha256_file(&alpha).unwrap();
        let beta_hash = crate::sha256_file(&beta).unwrap();
        let mut files = DeploymentInspection::new(5, 2, 0, u64::MAX);

        for _ in 0..2 {
            assert!(file_matches_for_status(
                &alpha,
                &alpha_hash,
                InspectionFailurePolicy::Preserve,
                &mut files,
            )
            .unwrap());
        }
        assert_eq!(files.remaining_hash_bytes, 0);
        let error = file_matches_for_status(
            &beta,
            &beta_hash,
            InspectionFailurePolicy::Preserve,
            &mut files,
        )
        .unwrap_err();
        assert!(matches!(&error, crate::ModError::InspectionBound(_)));
        assert!(error.to_string().contains("hashing budget"), "{error}");

        let alpha_legacy_hash = crate::content_hash(b"alpha");
        let mut legacy = DeploymentInspection::new(4, 1, 0, u64::MAX);
        let error = file_matches_for_status(
            &alpha,
            &alpha_legacy_hash,
            InspectionFailurePolicy::Preserve,
            &mut legacy,
        )
        .unwrap_err();
        assert!(matches!(&error, crate::ModError::InspectionBound(_)));
        assert!(error.to_string().contains("hashing budget"), "{error}");

        let mut file_entries = DeploymentInspection::new(100, 1, 0, u64::MAX);
        assert!(file_matches_for_status(
            &alpha,
            &alpha_hash,
            InspectionFailurePolicy::Preserve,
            &mut file_entries,
        )
        .unwrap());
        let error = file_matches_for_status(
            &beta,
            &beta_hash,
            InspectionFailurePolicy::Preserve,
            &mut file_entries,
        )
        .unwrap_err();
        assert!(matches!(&error, crate::ModError::InspectionBound(_)));
        assert!(error.to_string().contains("file-hash budget"), "{error}");

        let tree = tmp.path().join("tree");
        let other_tree = tmp.path().join("other-tree");
        std::fs::create_dir(&tree).unwrap();
        std::fs::create_dir(&other_tree).unwrap();
        std::fs::write(tree.join("payload"), b"tree").unwrap();
        std::fs::write(other_tree.join("payload"), b"tree").unwrap();
        let tree_hash = crate::tree_fingerprint(&tree).unwrap();
        let mut trees = DeploymentInspection::new(4, 0, 1, u64::MAX);
        for _ in 0..2 {
            assert!(tree_matches_for_status(
                &tree,
                &tree_hash,
                InspectionFailurePolicy::Preserve,
                &mut trees,
            )
            .unwrap());
        }
        assert_eq!(trees.remaining_hash_bytes, 0);
        assert_eq!(trees.remaining_tree_entries, 0);
        let error = tree_matches_for_status(
            &other_tree,
            &tree_hash,
            InspectionFailurePolicy::Preserve,
            &mut trees,
        )
        .unwrap_err();
        assert!(matches!(&error, crate::ModError::InspectionBound(_)));
        assert!(error.to_string().contains("tree-entry budget"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn deployment_hash_caches_fold_windows_record_path_aliases() {
        fn toggle_verbatim(path: &Path) -> std::path::PathBuf {
            let text = path.to_string_lossy();
            if let Some(plain) = text.strip_prefix(r"\\?\") {
                plain.into()
            } else {
                format!(r"\\?\{text}").into()
            }
        }

        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("alpha.bin");
        std::fs::write(&file, b"alpha").unwrap();
        let case_alias = file.with_file_name("ALPHA.BIN");
        let verbatim_alias = toggle_verbatim(&case_alias);
        assert!(verbatim_alias.is_file());
        assert_eq!(
            crate::record_path_key(&file),
            crate::record_path_key(&verbatim_alias)
        );

        let legacy_hash = crate::content_hash(b"alpha");
        let wrong_legacy_hash = crate::content_hash(b"other");
        let mut legacy = DeploymentInspection::new(5, 1, 0, u64::MAX);
        assert!(file_matches_for_status(
            &file,
            &legacy_hash,
            InspectionFailurePolicy::Preserve,
            &mut legacy,
        )
        .unwrap());
        assert!(file_matches_for_status(
            &verbatim_alias,
            &legacy_hash,
            InspectionFailurePolicy::Preserve,
            &mut legacy,
        )
        .unwrap());
        assert!(!file_matches_for_status(
            &case_alias,
            &wrong_legacy_hash,
            InspectionFailurePolicy::Preserve,
            &mut legacy,
        )
        .unwrap());
        assert_eq!(legacy.remaining_hash_bytes, 0);
        assert_eq!(legacy.remaining_hash_entries, 0);
        assert_eq!(legacy.file_matches.len(), 2);
        assert_eq!(legacy.legacy_hashes.len(), 1);

        let mut sha256 = DeploymentInspection::new(5, 1, 0, u64::MAX);
        assert_eq!(
            sha256_for_status(&file, InspectionFailurePolicy::Preserve, &mut sha256).unwrap(),
            sha256_for_status(
                &verbatim_alias,
                InspectionFailurePolicy::Preserve,
                &mut sha256,
            )
            .unwrap()
        );
        assert_eq!(sha256.remaining_hash_bytes, 0);
        assert_eq!(sha256.remaining_hash_entries, 0);
        assert_eq!(sha256.sha256.len(), 1);

        let tree = tmp.path().join("tree");
        std::fs::create_dir(&tree).unwrap();
        std::fs::write(tree.join("payload"), b"tree").unwrap();
        let tree_alias = toggle_verbatim(&tree.with_file_name("TREE"));
        assert!(tree_alias.is_dir());
        let tree_hash = crate::tree_fingerprint(&tree).unwrap();
        let mut trees = DeploymentInspection::new(4, 0, 1, u64::MAX);
        assert!(tree_matches_for_status(
            &tree,
            &tree_hash,
            InspectionFailurePolicy::Preserve,
            &mut trees,
        )
        .unwrap());
        assert!(tree_matches_for_status(
            &tree_alias,
            &tree_hash,
            InspectionFailurePolicy::Preserve,
            &mut trees,
        )
        .unwrap());
        assert_eq!(trees.remaining_hash_bytes, 0);
        assert_eq!(trees.remaining_tree_entries, 0);
        assert_eq!(trees.tree_fingerprints.len(), 1);
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
