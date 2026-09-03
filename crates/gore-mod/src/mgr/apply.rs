//! Declarative apply: realize an enabled loadout into ONE manager-owned deployment.
//!
//! [`apply_loadout`] composes every enabled library mod's components into a single [`DeployPlan`]
//! and commits it through the same recovery-aware machinery the single-bundle deploy uses. It always
//! rebuilds from pristine, so a toggle/reorder is realized by re-applying (never an incremental
//! patch on top of old state). Crucially it builds the ENTIRE (fallible) plan FIRST — reading each
//! pristine base from the prior deployment's backups — and only commits once that plan is
//! complete, then transactionally replaces the prior deployment through `commit_plan`, so a
//! bad/missing/undecodable mod or a commit failure can roll back to the working deployment.
//! This transaction guarantee covers errors returned in-process. The multi-target commit has no
//! write-ahead journal: after an abrupt process/OS crash, the pre-written deploy record supports a
//! later pristine undeploy, but does not promise automatic restoration of the exact old loadout.
//!
//! ## Rawfile-vs-patch ordering
//! Within a single apply, each target game file is materialized in two conceptual layers:
//!   1. **base** — if any enabled mod supplies a `RawFile` for that target (whole-file replacement),
//!      its bytes become the BASE for that file. Loadout order decides the winner: a later mod's
//!      rawfile replaces an earlier one's. If no mod supplies a rawfile, the base is the game's
//!      pristine file (read from the prior deployment's backup, or the live file when nothing was
//!      deployed).
//!   2. **patches** — every loc / audio / AngelScript edit from ALL enabled mods is then applied ON
//!      TOP of that base. So a mod can ship a rawfile `.lcache` as the base and another mod's loc
//!      patch still lands on top of it; the loc/audio/script merges (later-wins per key) are
//!      independent of which mod (if any) supplied the base.
//!
//! A rawfile whose target is never further patched is still written (its base bytes are the final
//! content). This is why the two passes below are: (1) collect rawfile base-overrides per target,
//! then (2) fold loc/audio/scripts on top and finally emit any un-patched rawfile bases verbatim.
//!
//! Voice ZIP edits are independent of rawfiles: they merge case-insensitively per archive/member,
//! then rewrite each archive once from its own pristine/prior-backup base.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::loadout::{Loadout, LoadoutEntry};
use super::model::{
    ComponentInfo, LibraryEntry, LibraryRoot, ModEntryMeta, PayloadTreeSnapshot, RawTarget,
    TreeSnapshotLimits,
};
use crate::{DeployPlan, DeployRecord, ModError};

/// Outcome of an apply: the enabled mods realized (display names, in loadout order) and any
/// non-fatal warnings (a rawfile with no live target, a loc id/lang missing from this install, …).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApplyReport {
    pub applied: Vec<String>,
    pub warnings: Vec<String>,
}

/// Finite resource envelope for rebuilding one manager loadout.  The file/total ceilings mirror
/// import's 8/16-GiB opaque-payload envelope, while formats that must be decoded in memory have
/// deliberately smaller limits.  Tests use a tiny copy through `apply_loadout_with_limits` so the
/// rejection paths are exercised without allocating giant fixtures.
#[derive(Debug, Clone, Copy)]
struct ApplyLimits {
    max_payload_path_bytes: usize,
    max_manifest_bytes: u64,
    max_manifest_total_bytes: u64,
    max_manifest_entries: usize,
    max_wav_bytes: u64,
    max_wav_total_bytes: u64,
    max_mini_bytes: u64,
    max_mini_total_bytes: u64,
    max_raw_file_bytes: u64,
    max_raw_total_bytes: u64,
    max_patch_base_bytes: u64,
    max_pristine_total_bytes: u64,
    max_generated_file_bytes: u64,
    max_generated_total_bytes: u64,
    max_tree_entries: usize,
    max_tree_depth: usize,
    max_tree_file_bytes: u64,
    max_tree_total_bytes: u64,
    max_additive_file_bytes: u64,
    max_additive_total_bytes: u64,
    max_loose_file_bytes: u64,
    max_loose_total_bytes: u64,
}

const DEFAULT_APPLY_LIMITS: ApplyLimits = ApplyLimits {
    max_payload_path_bytes: 4 * 1024,
    max_manifest_bytes: 16 * 1024 * 1024,
    max_manifest_total_bytes: 256 * 1024 * 1024,
    max_manifest_entries: 100_000,
    max_wav_bytes: 512 * 1024 * 1024,
    max_wav_total_bytes: 4 * 1024 * 1024 * 1024,
    max_mini_bytes: 1024 * 1024 * 1024,
    max_mini_total_bytes: 4 * 1024 * 1024 * 1024,
    max_raw_file_bytes: 8 * 1024 * 1024 * 1024,
    max_raw_total_bytes: 16 * 1024 * 1024 * 1024,
    max_patch_base_bytes: 1024 * 1024 * 1024,
    max_pristine_total_bytes: 8 * 1024 * 1024 * 1024,
    max_generated_file_bytes: 2 * 1024 * 1024 * 1024,
    max_generated_total_bytes: 8 * 1024 * 1024 * 1024,
    max_tree_entries: 100_000,
    max_tree_depth: 32,
    max_tree_file_bytes: 8 * 1024 * 1024 * 1024,
    max_tree_total_bytes: 16 * 1024 * 1024 * 1024,
    max_additive_file_bytes: 8 * 1024 * 1024 * 1024,
    max_additive_total_bytes: 16 * 1024 * 1024 * 1024,
    // Loose files are opaque bytes streamed to disk, never decoded, so they share the rawfile
    // envelope rather than the smaller in-memory ceilings.
    max_loose_file_bytes: 8 * 1024 * 1024 * 1024,
    max_loose_total_bytes: 16 * 1024 * 1024 * 1024,
};

#[derive(Debug, Default)]
struct ApplyBudget {
    manifest_bytes: u64,
    manifest_entries: usize,
    wav_bytes: u64,
    mini_bytes: u64,
    raw_bytes: u64,
    pristine_bytes: u64,
    generated_bytes: u64,
    tree_entries: usize,
    tree_bytes: u64,
    additive_bytes: u64,
    loose_bytes: u64,
}

#[derive(Debug, Clone)]
struct PendingPayload {
    entry: LibraryEntry,
    rel: PathBuf,
}

#[derive(Debug, Clone)]
struct PendingRaw {
    entry: LibraryEntry,
    rel: PathBuf,
}

/// Windows live-file identity for the three raw targets. Bank filenames are case-insensitive, but
/// the map value retains the actual winning path instead of ever publishing this folded key.
fn raw_target_identity(target: &RawTarget) -> String {
    match target {
        RawTarget::Lcache => "lcache".into(),
        RawTarget::Bank { name } => format!("bank:{}", crate::windows_file_name_key(name)),
        RawTarget::ScriptCache => "script_cache".into(),
    }
}

/// Resolve a bank spelling to the existing on-disk entry. This keeps case-insensitive Windows
/// semantics in cross-platform tests and preserves the real path spelling for records/backups.
fn resolve_bank_target_path(desktop: &Path, name: &str) -> crate::Result<PathBuf> {
    let exact = desktop.join(name);
    if exact.is_file() {
        return Ok(exact);
    }
    let folded = crate::windows_file_name_key(name);
    let mut matched = None;
    for entry in std::fs::read_dir(desktop).map_err(crate::io(&format!(
        "reading FMOD bank directory {}",
        desktop.display()
    )))? {
        let entry = entry.map_err(crate::io("reading FMOD bank entry"))?;
        if crate::windows_file_name_key(&entry.file_name().to_string_lossy()) != folded {
            continue;
        }
        if matched.is_some() {
            return Err(ModError::Other(format!(
                "multiple FMOD banks differ only by case for {name:?}"
            )));
        }
        matched = Some(entry.path());
    }
    Ok(matched.unwrap_or(exact))
}

#[derive(Debug, Clone)]
struct PendingAdditive {
    entry: LibraryEntry,
    rel: PathBuf,
    label: &'static str,
    destination: PathBuf,
    required: bool,
}

fn charge_bytes(kind: &str, total: &mut u64, amount: u64, limit: u64) -> crate::Result<()> {
    let next = total
        .checked_add(amount)
        .ok_or_else(|| ModError::Other(format!("{kind} byte count overflowed")))?;
    if next > limit {
        return Err(ModError::Other(format!(
            "{kind} total byte limit exceeded: {next} > {limit}"
        )));
    }
    *total = next;
    Ok(())
}

fn remaining_bytes(kind: &str, total: u64, limit: u64) -> crate::Result<u64> {
    limit.checked_sub(total).ok_or_else(|| {
        ModError::Other(format!(
            "{kind} accounting exceeded its total byte limit: {total} > {limit}"
        ))
    })
}

fn charge_entries(kind: &str, total: &mut usize, amount: usize, limit: usize) -> crate::Result<()> {
    let next = total
        .checked_add(amount)
        .ok_or_else(|| ModError::Other(format!("{kind} entry count overflowed")))?;
    if next > limit {
        return Err(ModError::Other(format!(
            "{kind} entry count limit exceeded: {next} > {limit}"
        )));
    }
    *total = next;
    Ok(())
}

fn validate_payload_rel(rel: &str, label: &str, limits: ApplyLimits) -> crate::Result<()> {
    if rel.len() > limits.max_payload_path_bytes {
        return Err(ModError::Other(format!(
            "{label} path byte limit exceeded: {} > {}",
            rel.len(),
            limits.max_payload_path_bytes
        )));
    }
    if !crate::is_safe_rel_path(rel) {
        return Err(ModError::Other(format!("unsafe {label} path: {rel:?}")));
    }
    Ok(())
}

/// Validate the sidecar-only component fields that the default apply path rejects before
/// publication. Apply invokes this before reading that component's payload; preflight uses the
/// same boundary so syntactically valid but undeployable enabled metadata never advertises Apply.
pub(super) fn validate_component_descriptor_for_default_apply(
    component: &ComponentInfo,
) -> crate::Result<()> {
    validate_component_descriptor(component, DEFAULT_APPLY_LIMITS)
}

fn validate_component_descriptor(
    component: &ComponentInfo,
    limits: ApplyLimits,
) -> crate::Result<()> {
    match component {
        ComponentInfo::Ue4ssLua { name, rel, .. } => {
            if !crate::is_safe_mod_name(name) {
                return Err(ModError::Other(format!(
                    "unsafe ue4ss component: name={name:?} rel={rel:?}"
                )));
            }
            validate_payload_rel(rel, "ue4ss component", limits)
        }
        ComponentInfo::LocPatch { rel, .. } => {
            validate_payload_rel(rel, "localization patch", limits)
        }
        ComponentInfo::AudioPatch { rel, .. } => {
            validate_payload_rel(rel, "audio patch", limits)?;
            validate_payload_rel(&format!("{rel}/manifest.json"), "audio manifest", limits)
        }
        ComponentInfo::TexturePatch { rel, .. } => {
            validate_payload_rel(rel, "texture component", limits)
        }
        ComponentInfo::AngelScriptPatch { rel, .. } => {
            validate_payload_rel(rel, "script patch", limits)?;
            validate_payload_rel(&format!("{rel}/manifest.json"), "script manifest", limits)
        }
        ComponentInfo::FilePatch { rel, .. } => {
            validate_payload_rel(rel, "loose file component", limits)?;
            validate_payload_rel(
                &format!("{rel}/manifest.json"),
                "loose file manifest",
                limits,
            )
        }
        ComponentInfo::PakFilePatch { rel, .. } => {
            validate_payload_rel(rel, "pak file component", limits)
        }
        ComponentInfo::VoiceArchivePatch { rel, .. } => {
            validate_payload_rel(rel, "voice patch", limits)
        }
        ComponentInfo::Triplet { rel_base, .. } => {
            validate_payload_rel(rel_base, "triplet", limits)?;
            for ext in ["utoc", "ucas", "pak"] {
                validate_payload_rel(&format!("{rel_base}.{ext}"), "triplet", limits)?;
            }
            Ok(())
        }
        ComponentInfo::LoosePak { rel, .. } => validate_payload_rel(rel, "loose pak", limits),
        ComponentInfo::RawFile { rel, target_file } => {
            validate_payload_rel(rel, "raw file", limits)?;
            if let RawTarget::Bank { name } = target_file {
                if name.len() > limits.max_payload_path_bytes || !crate::is_safe_filename(name) {
                    return Err(ModError::Other(format!("unsafe bank name: {name:?}")));
                }
            }
            Ok(())
        }
    }
}

fn read_manifest_payload(
    payload: &PendingPayload,
    label: &str,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<Vec<u8>> {
    let remaining = remaining_bytes(
        "manager manifests",
        budget.manifest_bytes,
        limits.max_manifest_total_bytes,
    )?;
    let bytes = payload.entry.read_payload_bounded(
        &payload.rel,
        label,
        limits.max_manifest_bytes.min(remaining),
    )?;
    charge_bytes(
        "manager manifests",
        &mut budget.manifest_bytes,
        bytes.len() as u64,
        limits.max_manifest_total_bytes,
    )?;
    Ok(bytes)
}

fn read_pending_payload(
    payload: &PendingPayload,
    label: &str,
    file_limit: u64,
    total: &mut u64,
    total_limit: u64,
) -> crate::Result<Vec<u8>> {
    let remaining = remaining_bytes(label, *total, total_limit)?;
    let bytes =
        payload
            .entry
            .read_payload_bounded(&payload.rel, label, file_limit.min(remaining))?;
    charge_bytes(label, total, bytes.len() as u64, total_limit)?;
    Ok(bytes)
}

fn read_raw_for_patch(
    source: &PendingRaw,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<Vec<u8>> {
    let remaining = remaining_bytes(
        "manager raw files",
        budget.raw_bytes,
        limits.max_raw_total_bytes,
    )?;
    let bytes = source.entry.read_payload_bounded(
        &source.rel,
        "raw-file patch base",
        limits
            .max_raw_file_bytes
            .min(limits.max_patch_base_bytes)
            .min(remaining),
    )?;
    charge_bytes(
        "manager raw files",
        &mut budget.raw_bytes,
        bytes.len() as u64,
        limits.max_raw_total_bytes,
    )?;
    Ok(bytes)
}

fn snapshot_raw_payload(
    source: &PendingRaw,
    max_payload_bytes: u64,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<(tempfile::TempPath, u64)> {
    let remaining = remaining_bytes(
        "manager raw files",
        budget.raw_bytes,
        limits.max_raw_total_bytes,
    )?;
    let (candidate, len) = source.entry.snapshot_payload_bounded(
        &source.rel,
        "raw-file payload",
        limits
            .max_raw_file_bytes
            .min(max_payload_bytes)
            .min(remaining),
    )?;
    charge_bytes(
        "manager raw files",
        &mut budget.raw_bytes,
        len,
        limits.max_raw_total_bytes,
    )?;
    Ok((candidate, len))
}

/// Keep the last script entry for each exact module target while preserving the original order
/// of all winners. `targets` lists every module each entry carries (a multi-module mini names only
/// one of them in its manifest). An entry survives only when it is the last contributor of every
/// module it carries and is dropped when a later entry re-targets all of them; a mini that would
/// lose only some of its modules is refused, because a multi-module mini composes as one unit and
/// its stale rows for the replaced module would otherwise stay in the ID plan and the tail. The
/// returned set records winning edits whose target was introduced by an earlier, now-shadowed add;
/// composition may retry those winners as adds if the effective base does not already contain the
/// target. `analyze` uses the same target strings as `ScriptModule` identity, so this intentionally
/// does not case-fold names.
fn retain_last_script_target_winners(
    scripts: Vec<(String, String, PendingPayload)>,
    targets: Vec<Vec<String>>,
) -> crate::Result<(Vec<(String, String, PendingPayload)>, BTreeSet<String>)> {
    debug_assert_eq!(scripts.len(), targets.len());
    let mut last_by_target = BTreeMap::<String, usize>::new();
    for (index, entry_targets) in targets.iter().enumerate() {
        for module in entry_targets {
            last_by_target.insert(module.clone(), index);
        }
    }

    let mut winners = Vec::new();
    winners
        .try_reserve_exact(last_by_target.len())
        .map_err(|error| {
            ModError::Other(format!("cannot reserve winning script entries: {error}"))
        })?;
    let mut prior_add_targets = BTreeSet::new();
    let mut winner_edits_after_add = BTreeSet::new();
    for (index, (script, entry_targets)) in scripts.into_iter().zip(targets).enumerate() {
        let shadowed: Vec<&String> = entry_targets
            .iter()
            .filter(|module| last_by_target.get(*module) != Some(&index))
            .collect();
        if shadowed.is_empty() {
            if script.0 == "edit" && prior_add_targets.contains(&script.1) {
                winner_edits_after_add.insert(script.1.clone());
            }
            winners.push(script);
        } else if shadowed.len() == entry_targets.len() {
            if script.0 == "add" {
                prior_add_targets.extend(entry_targets);
            }
        } else {
            return Err(ModError::Other(format!(
                "script mini {} of {:?} carries modules {entry_targets:?}, but later entries re-target {shadowed:?}: a multi-module mini composes as one unit, so disable or reorder one of them",
                script.2.rel.display(),
                script.2.entry
            )));
        }
    }
    Ok((winners, winner_edits_after_add))
}

fn validate_standalone_script_candidate(
    candidate: &Path,
    expected_len: u64,
    limit: u64,
) -> crate::Result<()> {
    use std::io::Read as _;

    if expected_len > limit {
        return Err(ModError::Other(format!(
            "standalone script-cache replacement exceeds the {limit} byte validation limit: {expected_len}"
        )));
    }
    let capacity = usize::try_from(expected_len).map_err(|_| {
        ModError::Other(format!(
            "standalone script-cache candidate is too large for this platform: {expected_len} bytes"
        ))
    })?;
    let file = std::fs::File::open(candidate).map_err(crate::io(&format!(
        "opening standalone script-cache candidate {}",
        candidate.display()
    )))?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|error| {
        ModError::Other(format!(
            "standalone script-cache candidate cannot be buffered for validation ({expected_len} bytes): {error}"
        ))
    })?;
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(crate::io(&format!(
            "reading standalone script-cache candidate {}",
            candidate.display()
        )))?;
    let observed = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if observed > limit {
        return Err(ModError::Other(format!(
            "standalone script-cache replacement exceeds the {limit} byte validation limit: {observed}"
        )));
    }
    if observed != expected_len {
        return Err(ModError::Other(format!(
            "standalone script-cache candidate changed while validating: expected {expected_len} bytes, read {observed}"
        )));
    }
    gore_as::cache::splice::validate_standalone_script_cache(&bytes)
        .map_err(|error| ModError::Other(format!("validate standalone script cache: {error}")))?;
    // Publication retains the original disk-backed candidate. Release the bounded validation copy
    // before hashing or adding that candidate to the deploy plan.
    drop(bytes);
    Ok(())
}

fn snapshot_loose_payload(
    source: &PendingPayload,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<tempfile::TempPath> {
    let remaining = remaining_bytes(
        "manager loose files",
        budget.loose_bytes,
        limits.max_loose_total_bytes,
    )?;
    let (candidate, len) = source.entry.snapshot_payload_bounded(
        &source.rel,
        "loose file payload",
        limits.max_loose_file_bytes.min(remaining),
    )?;
    charge_bytes(
        "manager loose files",
        &mut budget.loose_bytes,
        len,
        limits.max_loose_total_bytes,
    )?;
    Ok(candidate)
}

fn read_pristine_for_patch(
    live: &Path,
    prior: Option<&DeployRecord>,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<(Vec<u8>, crate::PristineSource)> {
    let remaining = remaining_bytes(
        "manager pristine patch bases",
        budget.pristine_bytes,
        limits.max_pristine_total_bytes,
    )?;
    let (bytes, source) = crate::read_pristine_bounded_with_source(
        live,
        prior,
        limits.max_patch_base_bytes.min(remaining),
    )?;
    charge_bytes(
        "manager pristine patch bases",
        &mut budget.pristine_bytes,
        bytes.len() as u64,
        limits.max_pristine_total_bytes,
    )?;
    Ok((bytes, source))
}

fn ensure_generated_fits(
    len: usize,
    limits: ApplyLimits,
    budget: &ApplyBudget,
) -> crate::Result<()> {
    let len = u64::try_from(len)
        .map_err(|_| ModError::Other("generated output byte count overflowed".into()))?;
    let remaining = remaining_bytes(
        "manager generated outputs",
        budget.generated_bytes,
        limits.max_generated_total_bytes,
    )?;
    let effective = limits.max_generated_file_bytes.min(remaining);
    if len > effective {
        return Err(ModError::Other(format!(
            "manager generated output exceeds its bounded remaining byte limit: {len} > {effective}"
        )));
    }
    Ok(())
}

fn stage_generated_output(
    plan: &mut DeployPlan,
    live: PathBuf,
    bytes: Vec<u8>,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<()> {
    ensure_generated_fits(bytes.len(), limits, budget)?;
    let len = bytes.len() as u64;
    charge_bytes(
        "manager generated outputs",
        &mut budget.generated_bytes,
        len,
        limits.max_generated_total_bytes,
    )?;
    let mut candidate = tempfile::Builder::new()
        .prefix(".gore-manager-generated-")
        .tempfile()
        .map_err(crate::io("creating generated-output candidate"))?;
    use std::io::Write as _;
    candidate
        .write_all(&bytes)
        .map_err(crate::io("writing generated-output candidate"))?;
    candidate
        .as_file()
        .sync_all()
        .map_err(crate::io("syncing generated-output candidate"))?;
    plan.file_writes
        .push(crate::DiskWrite::seal(live, candidate.into_temp_path())?);
    Ok(())
}

fn snapshot_component_tree(
    entry: &LibraryEntry,
    rel: &str,
    label: &str,
    limits: ApplyLimits,
    budget: &mut ApplyBudget,
) -> crate::Result<PayloadTreeSnapshot> {
    let remaining_entries = limits
        .max_tree_entries
        .checked_sub(budget.tree_entries)
        .ok_or_else(|| ModError::Other("manager tree entry accounting overflowed".into()))?;
    let remaining_bytes = remaining_bytes(
        "manager tree snapshots",
        budget.tree_bytes,
        limits.max_tree_total_bytes,
    )?;
    let snapshot = entry.snapshot_payload_tree(
        Path::new(rel),
        label,
        TreeSnapshotLimits {
            max_entries: remaining_entries,
            max_path_bytes: limits.max_payload_path_bytes,
            max_depth: limits.max_tree_depth,
            max_file_bytes: limits.max_tree_file_bytes,
            max_total_bytes: remaining_bytes,
        },
    )?;
    charge_entries(
        "manager tree snapshots",
        &mut budget.tree_entries,
        snapshot.entries(),
        limits.max_tree_entries,
    )?;
    charge_bytes(
        "manager tree snapshots",
        &mut budget.tree_bytes,
        snapshot.bytes(),
        limits.max_tree_total_bytes,
    )?;
    Ok(snapshot)
}

/// Realize `loadout`'s enabled entries into one manager deployment at `game_root`, reading each
/// mod's payload from `<library_dir>/<id>/`. Refuses to run over a studio (non-manager) deployment.
/// The result is a full recomputation of the enabled set, committed with in-process rollback over
/// the previous manager deployment when one exists. It is recovery-aware, not crash-atomic across
/// every target; see the module-level safety note.
pub fn apply_loadout(
    game_root: &Path,
    library_dir: &Path,
    loadout: &Loadout,
) -> crate::Result<ApplyReport> {
    apply_loadout_with_limits(game_root, library_dir, loadout, DEFAULT_APPLY_LIMITS, false)
}

/// Store routes have already completed replacement recovery while holding the library mutation
/// lock, which they retain through this call. Keeping this entry point crate-private prevents an
/// unlocked caller from accidentally bypassing recovery.
pub(crate) fn apply_loadout_after_store_snapshot(
    game_root: &Path,
    library_dir: &Path,
    loadout: &Loadout,
) -> crate::Result<ApplyReport> {
    apply_loadout_with_limits(game_root, library_dir, loadout, DEFAULT_APPLY_LIMITS, true)
}

fn apply_loadout_with_limits(
    game_root: &Path,
    library_dir: &Path,
    loadout: &Loadout,
    limits: ApplyLimits,
    library_recovery_is_held: bool,
) -> crate::Result<ApplyReport> {
    // Absolutize like deploy()/undeploy() so every derived + persisted path is absolute. This MUST
    // happen before reading the record: deploy/undeploy/status all key the record off `abs_root`, so
    // reading it under the caller's raw (possibly symlinked/junction) `game_root` could miss an
    // existing record — skipping the studio guard below and letting the later undeploy remove a
    // studio deployment apply was supposed to refuse.
    let abs_root = crate::abs_root(game_root);
    let gp = crate::resolve_game_paths(&abs_root);

    // (1) Read the prior manager deployment (if any). A studio deployment is off-limits — replacing
    //     it would silently drop a hand-built mod. We KEEP `prev`: the plan below reads pristine
    //     bytes from its backups while that deployment is still live. `commit_plan` receives the
    //     prior record and owns the swap/rollback, so even a late write/record failure restores the
    //     exact working deployment instead of leaving the manager merely pristine.
    let prev = crate::read_record(&abs_root)?;
    let prior = prev.as_ref().map(|stored| &stored.record);
    if prior.is_some_and(|record| record.phase == crate::DeployPhase::RecoveryRequired) {
        return Err(crate::recovery_required_error());
    }
    if let Some(p) = prior {
        if p.owner != "manager" {
            return Err(ModError::Other(format!(
                "STUDIO_DEPLOY_ACTIVE:{}",
                p.mod_name
            )));
        }
    }

    // Loadouts are persisted input. Validate ALL slots (including disabled ones) before the empty
    // branch can undeploy an active manager deployment.
    loadout.validate()?;
    if loadout.entries.iter().any(|entry| entry.enabled) && !library_recovery_is_held {
        super::import::recover_library_for_read(library_dir)?;
    }

    // (3) Load the enabled entries' metadata, remembering each one's 0-based slot among the
    //     ENABLED entries (drives per-mod `gm{idx:03}` naming and numeric patch priority).
    struct Loaded<'a> {
        idx: usize,
        entry: &'a LoadoutEntry,
        library_entry: LibraryEntry,
        meta: ModEntryMeta,
    }
    let library = if loadout.entries.iter().any(|entry| entry.enabled) {
        Some(LibraryRoot::open(library_dir)?)
    } else {
        None
    };
    let mut loaded: Vec<Loaded> = Vec::new();
    for entry in loadout.entries.iter().filter(|e| e.enabled) {
        let idx = loaded.len();
        let library_entry = library
            .as_ref()
            .expect("an enabled entry opens the library")
            .entry(&entry.id)?;
        let meta = library_entry.read_meta()?;
        loaded.push(Loaded {
            idx,
            entry,
            library_entry,
            meta,
        });
    }

    // (4) EMPTY enabled set: nothing to deploy. Reset to pristine and leave it that way (do NOT
    //     commit an empty manager record). This is the one branch that undeploys without a rebuild.
    if loaded.is_empty() {
        crate::undeploy(game_root)?;
        return Ok(ApplyReport {
            applied: Vec::new(),
            warnings: Vec::new(),
        });
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut plan = DeployPlan::default();

    // Accumulators (all later-wins on key collisions, in loadout order):
    //   rawfile bases per target file, then loc/audio/script patches layered on top.
    struct PendingLoc {
        /// Most recently encountered spelling of this case-insensitive id.
        id: String,
        /// Folded language -> (most recent spelling, text).
        values: BTreeMap<String, (String, String)>,
    }
    let mut rawfile_sources: BTreeMap<String, (PathBuf, PendingRaw)> = BTreeMap::new();
    // Keyed by the CASE-FOLDED destination, so loadout order gives later-wins before the plan is
    // built. This is not cosmetic: `first_duplicate_dst` rejects a plan with two writes to one
    // path, and it *resolves* those paths — so on Windows two mods spelling one file differently
    // (`Normal.PNG` against `normal.png`) would otherwise turn the ordinary two-mod overlap that
    // `mgr analyze` reports, whose `norm_loose` folds exactly the same way, into a hard apply
    // failure. The value carries the winning mod's own spelling, which is what gets written and
    // recorded.
    let mut loose_files: BTreeMap<String, (PathBuf, PendingPayload)> = BTreeMap::new();
    let mut loc: BTreeMap<String, PendingLoc> = BTreeMap::new();
    // Bank identity is Windows-case-insensitive; sample identity deliberately remains exact. The
    // companion map retains the last effective bank spelling for the one generated output path.
    let mut audio: BTreeMap<(String, String), PendingPayload> = BTreeMap::new();
    let mut audio_bank_spellings: BTreeMap<String, String> = BTreeMap::new();
    let mut scripts: Vec<(String, String, PendingPayload)> = Vec::new();
    let mut voice = crate::PendingVoiceEdits::new();
    let mut voice_order = 0usize;
    let mut budget = ApplyBudget::default();
    let mut additive_sources: Vec<PendingAdditive> = Vec::new();
    let mut tree_snapshots: Vec<PayloadTreeSnapshot> = Vec::new();

    // ── PASS 1: additive components + collect rawfile base-overrides ───────────────────────────
    // A globally-unique index per texture component (NOT the mod slot `l.idx`, which is constant
    // across a mod's components): `prepare_texture_component` scopes its cook/pack temp dirs and the
    // output triplet name by this index, so two TexturePatch components in one mod would otherwise
    // collide and clobber each other's output.
    let mut tex_comp_idx = 0usize;
    // Same reasoning for pak-file components, on its own counter so a bundle carrying both kinds
    // cannot have one kind's index collide with the other's temp dir.
    let mut pak_files_comp_idx = 0usize;
    // One pak-shadow oracle for the whole composed apply, built at most once and only if a loose
    // destination actually asks.
    let mut shadow = crate::PakShadowIndex::new(&gp.root);
    for l in &loaded {
        for comp in &l.meta.components {
            validate_component_descriptor(comp, limits)?;
            match comp {
                ComponentInfo::Ue4ssLua { name, rel, .. } => {
                    if !crate::is_safe_mod_name(name) {
                        return Err(ModError::Other(format!(
                            "unsafe ue4ss component in {}: name={name:?} rel={rel:?}",
                            l.entry.id
                        )));
                    }
                    // Verify the source exists NOW — before the deferred undeploy — so a
                    // deleted/corrupt library entry fails the apply instead of tearing down the
                    // working deployment and then failing during commit_plan's copy.
                    validate_payload_rel(rel, "ue4ss component", limits)?;
                    let snapshot = snapshot_component_tree(
                        &l.library_entry,
                        rel,
                        "ue4ss component",
                        limits,
                        &mut budget,
                    )?;
                    let src = snapshot.path().to_path_buf();
                    let dst = gp.ue4ss_mods.join(format!("gm{:03}_{}", l.idx, name));
                    plan.ue4ss_dirs.push((src, dst));
                    tree_snapshots.push(snapshot);
                }
                ComponentInfo::TexturePatch { rel, .. } => {
                    validate_payload_rel(rel, "texture component", limits)?;
                    let snapshot = snapshot_component_tree(
                        &l.library_entry,
                        rel,
                        "texture component",
                        limits,
                        &mut budget,
                    )?;
                    // Cook + pack a Zen triplet. The manager name keeps the enabled slot visible
                    // and unique, while the numeric suffix added to every destination below is the
                    // part Unreal actually interprets as patch priority. The shared single-mod
                    // builder intentionally keeps its existing names.
                    let manager_name = format!("gm{:03}_{}", l.idx, l.meta.id);
                    let (triplets, temporary_root) = crate::prepare_texture_component(
                        snapshot.bundle_root(),
                        rel,
                        &manager_name,
                        tex_comp_idx,
                        &gp,
                    )?;
                    plan.texture_triplets
                        .extend(prioritize_generated_containers(triplets, l.idx)?);
                    plan.temporary_roots.push(temporary_root);
                    tex_comp_idx += 1;
                }
                ComponentInfo::Triplet { rel_base, .. } => {
                    validate_payload_rel(rel_base, "triplet", limits)?;
                    // A mountable triplet needs BOTH its `.utoc` (table of contents) and `.ucas`
                    // (the container payload) — the `.utoc` alone is unmountable. Require both up
                    // front (before the deferred undeploy) so an incomplete/corrupt triplet fails
                    // here rather than mid-copy after the working deployment is already gone. The
                    // `.pak` stub is optional and copied below only if present.
                    let stem = slot_pak_stem(rel_base, l.idx)?;
                    for ext in ["utoc", "ucas", "pak"] {
                        let rel = format!("{rel_base}.{ext}");
                        validate_payload_rel(&rel, "triplet", limits)?;
                        let dst = mods_dir(&gp).join(format!("{stem}.{ext}"));
                        additive_sources.push(PendingAdditive {
                            entry: l.library_entry.clone(),
                            rel: PathBuf::from(rel),
                            label: "triplet component",
                            destination: dst,
                            required: ext != "pak",
                        });
                    }
                }
                ComponentInfo::LoosePak { rel, .. } => {
                    validate_payload_rel(rel, "loose pak", limits)?;
                    let base = Path::new(rel)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("pak");
                    let stem = slot_stem(base, l.idx)?;
                    let dst = mods_dir(&gp).join(format!("{stem}.pak"));
                    additive_sources.push(PendingAdditive {
                        entry: l.library_entry.clone(),
                        rel: PathBuf::from(rel),
                        label: "loose pak",
                        destination: dst,
                        required: true,
                    });
                }
                ComponentInfo::RawFile { rel, target_file } => {
                    validate_payload_rel(rel, "raw file", limits)?;
                    // Resolve the single live target this rawfile replaces wholesale.
                    let target = match target_file {
                        RawTarget::Lcache => match gp.lcache.clone() {
                            Some(p) => p,
                            None => {
                                warnings.push(format!(
                                    "{}: no AlkimiaLocalization .lcache in this install — \
                                     skipping raw lcache replacement",
                                    l.entry.id
                                ));
                                continue;
                            }
                        },
                        RawTarget::Bank { name } => {
                            if name.len() > limits.max_payload_path_bytes
                                || !crate::is_safe_filename(name)
                            {
                                return Err(ModError::Other(format!("unsafe bank name: {name:?}")));
                            }
                            resolve_bank_target_path(&gp.fmod_desktop, name)?
                        }
                        RawTarget::ScriptCache => gp.script_cache.clone(),
                    };
                    // A rawfile replaces a WHOLE existing game file. If the resolved target isn't
                    // present in this install (an incompatible mod — a bank/version this install
                    // lacks), skip it with a warning HERE, before the deferred undeploy — otherwise
                    // commit_plan fails while backing up the missing file, after the working
                    // deployment is already gone (consistent with the lcache-absent skip above and
                    // the additive-source checks).
                    if !target.is_file() {
                        warnings.push(format!(
                            "{}: raw-file target {} not present in this install — skipping",
                            l.entry.id,
                            target.display()
                        ));
                        continue;
                    }
                    // Later mod wins the base for this target. Keep only a validated library
                    // reference for now: patched targets are bounded-read once when decoded;
                    // unpatched whole-file replacements become disk-backed snapshots below.
                    rawfile_sources.insert(
                        raw_target_identity(target_file),
                        (
                            target,
                            PendingRaw {
                                entry: l.library_entry.clone(),
                                rel: PathBuf::from(rel),
                            },
                        ),
                    );
                }
                ComponentInfo::LocPatch { rel, .. } => {
                    validate_payload_rel(rel, "localization patch", limits)?;
                    let payload = PendingPayload {
                        entry: l.library_entry.clone(),
                        rel: PathBuf::from(rel),
                    };
                    let edits: BTreeMap<String, BTreeMap<String, String>> =
                        serde_json::from_slice(&read_manifest_payload(
                            &payload,
                            "localization manifest",
                            limits,
                            &mut budget,
                        )?)?;
                    let edit_count = edits.values().try_fold(0usize, |count, sets| {
                        count.checked_add(sets.len().max(1)).ok_or_else(|| {
                            ModError::Other("localization edit count overflowed".into())
                        })
                    })?;
                    charge_entries(
                        "manager manifests",
                        &mut budget.manifest_entries,
                        edit_count,
                        limits.max_manifest_entries,
                    )?;
                    for (id, sets) in edits {
                        let folded_id = id.to_ascii_lowercase();
                        let pending = loc.entry(folded_id).or_insert_with(|| PendingLoc {
                            id: id.clone(),
                            values: BTreeMap::new(),
                        });
                        // Traversal is loadout order, so overwriting both spellings and values
                        // here preserves manager's later-mod-wins contract even for aliases.
                        pending.id = id;
                        for (set, text) in sets {
                            pending.values.insert(set.to_ascii_lowercase(), (set, text));
                            // later-wins
                        }
                    }
                }
                ComponentInfo::AudioPatch { rel, .. } => {
                    validate_payload_rel(rel, "audio patch", limits)?;
                    // `rel` is authored portable syntax and was validated above.  Keep the
                    // internally-appended child portable too: `Path::join` renders `\\` on
                    // Windows, which the payload boundary correctly rejects as non-portable.
                    let manifest_rel = PathBuf::from(format!("{rel}/manifest.json"));
                    validate_payload_rel(
                        &manifest_rel.to_string_lossy(),
                        "audio manifest",
                        limits,
                    )?;
                    let manifest_payload = PendingPayload {
                        entry: l.library_entry.clone(),
                        rel: manifest_rel,
                    };
                    let map: BTreeMap<String, BTreeMap<String, String>> =
                        serde_json::from_slice(&read_manifest_payload(
                            &manifest_payload,
                            "audio manifest",
                            limits,
                            &mut budget,
                        )?)?;
                    let sample_count = map.values().try_fold(0usize, |count, samples| {
                        count
                            .checked_add(samples.len().max(1))
                            .ok_or_else(|| ModError::Other("audio sample count overflowed".into()))
                    })?;
                    charge_entries(
                        "manager manifests",
                        &mut budget.manifest_entries,
                        sample_count,
                        limits.max_manifest_entries,
                    )?;
                    for (bank, samples) in map {
                        if bank.len() > limits.max_payload_path_bytes
                            || !crate::is_safe_filename(&bank)
                        {
                            return Err(ModError::Other(format!("unsafe bank name: {bank:?}")));
                        }
                        let bank_key = crate::windows_file_name_key(&bank);
                        if !samples.is_empty() {
                            audio_bank_spellings.insert(bank_key.clone(), bank.clone());
                        }
                        for (sample, wav_rel) in samples {
                            validate_payload_rel(&wav_rel, "WAV", limits)?;
                            audio.insert(
                                (bank_key.clone(), sample),
                                PendingPayload {
                                    entry: l.library_entry.clone(),
                                    rel: PathBuf::from(wav_rel),
                                },
                            );
                            // later-wins
                        }
                    }
                }
                ComponentInfo::AngelScriptPatch { rel, .. } => {
                    validate_payload_rel(rel, "script patch", limits)?;
                    let manifest_rel = PathBuf::from(format!("{rel}/manifest.json"));
                    validate_payload_rel(
                        &manifest_rel.to_string_lossy(),
                        "script manifest",
                        limits,
                    )?;
                    let manifest_payload = PendingPayload {
                        entry: l.library_entry.clone(),
                        rel: manifest_rel,
                    };
                    let entries: Vec<crate::ScriptEntry> =
                        serde_json::from_slice(&read_manifest_payload(
                            &manifest_payload,
                            "script manifest",
                            limits,
                            &mut budget,
                        )?)?;
                    charge_entries(
                        "manager manifests",
                        &mut budget.manifest_entries,
                        entries.len(),
                        limits.max_manifest_entries,
                    )?;
                    for e in entries {
                        validate_payload_rel(&e.mini, "mini-cache", limits)?;
                        scripts.push((
                            e.op,
                            e.module,
                            PendingPayload {
                                entry: l.library_entry.clone(),
                                rel: PathBuf::from(e.mini),
                            },
                        ));
                    }
                }
                ComponentInfo::FilePatch { rel, .. } => {
                    validate_payload_rel(rel, "loose file component", limits)?;
                    let manifest_rel = PathBuf::from(format!("{rel}/manifest.json"));
                    validate_payload_rel(
                        &manifest_rel.to_string_lossy(),
                        "loose file manifest",
                        limits,
                    )?;
                    let manifest_payload = PendingPayload {
                        entry: l.library_entry.clone(),
                        rel: manifest_rel,
                    };
                    let map: BTreeMap<String, String> =
                        serde_json::from_slice(&read_manifest_payload(
                            &manifest_payload,
                            "loose file manifest",
                            limits,
                            &mut budget,
                        )?)?;
                    charge_entries(
                        "manager manifests",
                        &mut budget.manifest_entries,
                        map.len(),
                        limits.max_manifest_entries,
                    )?;
                    for (game_path, payload_rel) in map {
                        crate::validate_loose_game_path(&game_path)?;
                        validate_payload_rel(&payload_rel, "loose file payload", limits)?;
                        let target = gp.root.join(crate::loose_relative_os_path(&game_path));
                        // A loose file REPLACES an existing game file. If this install does not
                        // have it (a mod built against another version), skip it with a warning
                        // HERE, before the deferred undeploy — otherwise commit_plan fails while
                        // backing up the missing file, after the working deployment is gone.
                        if !target.is_file() {
                            warnings.push(format!(
                                "{}: loose-file target {} not present in this install — skipping",
                                l.entry.id,
                                target.display()
                            ));
                            continue;
                        }
                        // A destination one of the shipped containers already carries is inert:
                        // Unreal consults a mounted pak before the file on disk, so the write
                        // would succeed and change nothing. Warn and SKIP rather than fail, for
                        // the same reason the missing-target case does — the manager composes many
                        // mods and one bad destination must not brick the whole loadout after the
                        // working deployment is gone.
                        if let Some(pak) = shadow.owning_pak(&game_path)? {
                            warnings.push(format!(
                                "{}: loose-file target {game_path} is already packed in {pak}, so \
                                 the packed copy wins and replacing it on disk would change \
                                 nothing — skipping (the mod's author wants a \"pak_files\" \
                                 section instead)",
                                l.entry.id
                            ));
                            continue;
                        }
                        loose_files.insert(
                            game_path.to_lowercase(),
                            (
                                target,
                                PendingPayload {
                                    entry: l.library_entry.clone(),
                                    rel: PathBuf::from(payload_rel),
                                },
                            ),
                        );
                        // later-wins
                    }
                }
                ComponentInfo::PakFilePatch { rel, .. } => {
                    validate_payload_rel(rel, "pak file component", limits)?;
                    let snapshot = snapshot_component_tree(
                        &l.library_entry,
                        rel,
                        "pak file component",
                        limits,
                        &mut budget,
                    )?;
                    // Additive, so no destination has to exist and nothing is shadow-checked:
                    // `meta.id` gives cross-mod uniqueness of the pak name, `pak_files_comp_idx`
                    // per-component uniqueness within a mod.
                    //
                    // The `gm{idx:03}` prefix keeps the generated filename unique and inspectable.
                    // Unreal does not derive patch priority from that prefix: the numeric
                    // `_<priority>_P` suffix added to the destination below is what makes a later
                    // enabled loadout entry win a contested path.
                    let manager_name = format!("gm{:03}_{}", l.idx, l.meta.id);
                    let (paks, temporary_root) = crate::prepare_pak_file_component(
                        snapshot.bundle_root(),
                        rel,
                        &manager_name,
                        pak_files_comp_idx,
                        &gp,
                    )?;
                    plan.texture_triplets
                        .extend(prioritize_generated_containers(paks, l.idx)?);
                    plan.temporary_roots.push(temporary_root);
                    pak_files_comp_idx += 1;
                }
                ComponentInfo::VoiceArchivePatch { rel, .. } => {
                    validate_payload_rel(rel, "voice patch", limits)?;
                    let snapshot = snapshot_component_tree(
                        &l.library_entry,
                        rel,
                        "voice patch",
                        limits,
                        &mut budget,
                    )?;
                    crate::merge_voice_component(
                        snapshot.bundle_root(),
                        rel,
                        &mut voice,
                        &mut voice_order,
                    )?;
                }
            }
        }
    }

    // Pak/utoc/ucas payloads can be multi-GiB, so never retain them in `Vec<u8>`. Open each source
    // exactly once and stream that same no-follow handle into a private candidate. Passing the
    // aggregate remainder into the snapshot makes the size check happen before any bytes are
    // copied; charging the observed snapshot then accounts for the exact retained disk footprint.
    // This avoids a preflight/reopen gap where a mutable library path could name a different file.
    let mut additive_candidates: Vec<tempfile::TempPath> =
        Vec::with_capacity(additive_sources.len());
    for source in additive_sources {
        let remaining = remaining_bytes(
            "manager additive payloads",
            budget.additive_bytes,
            limits.max_additive_total_bytes,
        )?;
        let snapshot_limit = limits.max_additive_file_bytes.min(remaining);
        let snapshot = if source.required {
            Some(source.entry.snapshot_payload_bounded(
                &source.rel,
                source.label,
                snapshot_limit,
            )?)
        } else {
            source.entry.snapshot_optional_payload_bounded(
                &source.rel,
                source.label,
                snapshot_limit,
            )?
        };
        let Some((candidate, observed)) = snapshot else {
            continue;
        };
        charge_bytes(
            "manager additive payloads",
            &mut budget.additive_bytes,
            observed,
            limits.max_additive_total_bytes,
        )?;
        let candidate_path = candidate.to_path_buf();
        additive_candidates.push(candidate);
        plan.managed_paks.push((candidate_path, source.destination));
    }

    // ── PASS 2: materialize patch targets on top of their base ────────────────────────────────
    // Generated targets are moved to disk-backed candidates as soon as each composition finishes;
    // only the one currently decoded output is resident in memory.

    // loc → decode base, set each (id,set), re-encode. Base = a rawfile lcache override if present,
    // else the pristine .lcache.
    if !loc.is_empty() {
        if let Some(lcache) = gp.lcache.clone() {
            let (base, pristine_source) =
                match rawfile_sources.remove(&raw_target_identity(&RawTarget::Lcache)) {
                    Some((_target, source)) => {
                        let pristine_source = crate::select_pristine_source(&lcache, prior)?;
                        (
                            read_raw_for_patch(&source, limits, &mut budget)?,
                            pristine_source,
                        )
                    }
                    // Read pristine from the PRIOR deployment's backup (via `prev`) — the live file is
                    // still the prior-modded one until the deferred undeploy below.
                    None => read_pristine_for_patch(&lcache, prior, limits, &mut budget)?,
                };
            plan.bind_backup_identity(&lcache, pristine_source.basis)?;
            let mut lc = gore_loc::loc::Lcache::decode(&base)?;
            let declared: BTreeMap<String, String> = lc
                .languages()
                .into_iter()
                .map(|language| (language.to_ascii_lowercase(), language))
                .collect();
            for pending in loc.values() {
                let id = &pending.id;
                let mut valid = BTreeMap::new();
                for (folded_set, (set, text)) in &pending.values {
                    if let Some(canonical) = declared.get(folded_set) {
                        valid.insert(canonical.clone(), text.clone());
                    } else {
                        warnings.push(format!(
                            "loc {id}|{set}: language '{set}' is not declared in the lcache header"
                        ));
                    }
                }

                if lc.has_key(id) {
                    // Read before the writes below borrow the cache mutably, and for the same
                    // reason the deploy path reads it: the answer is about the id's slots as they
                    // stand, not as this apply leaves them.
                    let carried: Vec<String> = lc
                        .languages_for(id)
                        .into_iter()
                        .map(str::to_ascii_lowercase)
                        .collect();
                    for (set, text) in &valid {
                        // Best-effort: a language absent from THIS install's existing record is a
                        // warning, not a hard failure (a mod built against a different version).
                        if let Err(e) = lc.set_value(id, set, text) {
                            warnings.push(format!("loc {id}|{set}: {e}"));
                            continue;
                        }
                        // The edit landed; whether anyone will see it is the other question. This
                        // route applies the same bundles as `gore mod deploy`, which reports this,
                        // and a managed apply was silently leaving an edit the game never shows.
                        if let Some(winner) =
                            crate::shadowing_generation(&carried, &set.to_ascii_lowercase(), |w| {
                                pending.values.contains_key(w)
                            })
                        {
                            warnings.push(format!(
                                "loc {id}|{set}: the id also carries '{winner}', which the game \
                                 displays instead"
                            ));
                        }
                    }
                } else {
                    // Missing ids are the core loc-mod use case (new dialog/quest text). Validate
                    // and append all usable translations together, preserving header order and
                    // avoiding a partially-built key when one patch language is unsupported.
                    if !valid.is_empty() {
                        if let Err(e) = lc.add_key(id, &valid) {
                            warnings.push(format!("loc {id}: {e}"));
                        }
                    }
                }
            }
            stage_generated_output(&mut plan, lcache, lc.encode()?, limits, &mut budget)?;
        } else {
            warnings.push(
                "loc edits present but no AlkimiaLocalization .lcache in this install — skipping"
                    .into(),
            );
        }
    }

    // audio → per bank, build PCM16 replacements against the bank base (rawfile override or pristine).
    if !audio.is_empty() {
        let fmod_key = crate::resolve_fmod_key(&gp);
        // Group (bank,sample)→wav into bank→[(sample,wav)].
        let mut by_bank: BTreeMap<String, Vec<(String, PendingPayload)>> = BTreeMap::new();
        for ((bank_key, sample), wav) in &audio {
            by_bank
                .entry(bank_key.clone())
                .or_default()
                .push((sample.clone(), wav.clone()));
        }
        for (bank_key, samples) in by_bank {
            let bank = audio_bank_spellings.get(&bank_key).ok_or_else(|| {
                ModError::Other(format!(
                    "audio bank spelling was not retained for identity {bank_key:?}"
                ))
            })?;
            let raw_target = RawTarget::Bank { name: bank.clone() };
            let raw_source = rawfile_sources.remove(&raw_target_identity(&raw_target));
            // A raw winner already resolved the real on-disk spelling while collecting bases.
            // Otherwise resolve the audio manifest's spelling against the same Windows identity.
            let bank_path = match &raw_source {
                Some((target, _)) => target.clone(),
                None => resolve_bank_target_path(&gp.fmod_desktop, bank)?,
            };
            let (base, pristine_source) = match raw_source {
                Some((_target, source)) => {
                    let pristine_source = crate::select_pristine_source(&bank_path, prior)?;
                    (
                        read_raw_for_patch(&source, limits, &mut budget)?,
                        pristine_source,
                    )
                }
                // Pristine from the prior deployment's backup (live is still modded until undeploy).
                None => read_pristine_for_patch(&bank_path, prior, limits, &mut budget)?,
            };
            plan.bind_backup_identity(&bank_path, pristine_source.basis)?;
            let mut repl = Vec::with_capacity(samples.len());
            for (sample, wav_payload) in samples {
                let wav = read_pending_payload(
                    &wav_payload,
                    "audio WAV payloads",
                    limits.max_wav_bytes,
                    &mut budget.wav_bytes,
                    limits.max_wav_total_bytes,
                )?;
                let (rate, ch, pcm) = gore_fmod::read_wav_pcm16(&wav).map_err(ModError::Fmod)?;
                repl.push((
                    sample.clone(),
                    gore_fmod::Pcm16Sample {
                        name: sample,
                        freq: rate,
                        channels: ch,
                        pcm,
                    },
                ));
            }
            let new_bank =
                gore_fmod::replace_samples(&base, &fmod_key, repl).map_err(ModError::Fmod)?;
            stage_generated_output(&mut plan, bank_path, new_bank, limits, &mut budget)?;
        }
    }

    // scripts → fold add/edit onto the script-cache base (rawfile override or pristine cache).
    if !scripts.is_empty() {
        if let Some((op, module, _)) = scripts
            .iter()
            .find(|(op, _, _)| op != "add" && op != "edit")
        {
            return Err(ModError::Other(format!(
                "invalid script op {op:?} for module {module:?}"
            )));
        }
        // Conflict analysis and the UI promise exact-target later-wins semantics for every module
        // a mini carries, not only the one its manifest names. Read the carried module names first
        // (a mini that cannot be read here fails with its real error in the passes below), then
        // reduce before inventory, canonicalization, and composition so a shadowed mini cannot
        // still collide in the global ID plan or attempt a duplicate module splice.
        let mut target_scan_bytes = 0u64;
        let mut targets = Vec::new();
        targets.try_reserve_exact(scripts.len()).map_err(|error| {
            ModError::Other(format!("cannot reserve script target lists: {error}"))
        })?;
        for (_, module, mini_payload) in &scripts {
            let carried = read_pending_payload(
                mini_payload,
                "script mini-cache target scan",
                limits.max_mini_bytes,
                &mut target_scan_bytes,
                limits.max_mini_total_bytes,
            )
            .ok()
            .and_then(|mini| gore_as::cache::walk_modules::module_names(&mini).ok())
            .filter(|names| names.iter().any(|name| name == module));
            targets.push(carried.unwrap_or_else(|| vec![module.clone()]));
        }
        let (winning_scripts, winner_edits_after_add) =
            retain_last_script_target_winners(scripts, targets)?;
        scripts = winning_scripts;
        let (base, pristine_source) =
            match rawfile_sources.remove(&raw_target_identity(&RawTarget::ScriptCache)) {
                Some((_target, source)) => {
                    let pristine_source = crate::select_pristine_source(&gp.script_cache, prior)?;
                    (
                        read_raw_for_patch(&source, limits, &mut budget)?,
                        pristine_source,
                    )
                }
                None => read_pristine_for_patch(&gp.script_cache, prior, limits, &mut budget)?,
            };
        plan.bind_backup_identity(&gp.script_cache, pristine_source.basis)?;
        // Pass 1 inventories the complete loadout while retaining only one source mini at a time.
        // Canonical assignments therefore depend on the portable-identity union, never mod order.
        let mut loadout_builder = gore_as::cache::splice::LoadoutScriptIdPlanBuilder::new(&base)
            .map_err(|e| ModError::Other(format!("prepare script composition: {e}")))?;
        for (_, module, mini_payload) in &scripts {
            let mini = read_pending_payload(
                mini_payload,
                "script mini-cache payloads",
                limits.max_mini_bytes,
                &mut budget.mini_bytes,
                limits.max_mini_total_bytes,
            )?;
            loadout_builder
                .inspect(&mini)
                .map_err(|e| ModError::Other(format!("inspect script mini {module}: {e}")))?;
        }
        let loadout_plan = loadout_builder
            .finish()
            .map_err(|e| ModError::Other(format!("finish script ID plan: {e}")))?;

        // Pass 2 rereads the SHA-bound source minis and immediately seals each canonical result on
        // private disk. Separate phase budgets preserve the existing 4-GiB logical source envelope
        // while bounding the additional I/O and temporary footprint to the same amount.
        let mut rewrite_source_bytes = 0u64;
        let mut canonical_output_bytes = 0u64;
        let mut canonical_minis = Vec::new();
        canonical_minis
            .try_reserve_exact(scripts.len())
            .map_err(|error| {
                ModError::Other(format!(
                    "cannot reserve canonical script mini candidates: {error}"
                ))
            })?;
        for (_, module, mini_payload) in &scripts {
            let mini = read_pending_payload(
                mini_payload,
                "script mini-cache canonicalization",
                limits.max_mini_bytes,
                &mut rewrite_source_bytes,
                limits.max_mini_total_bytes,
            )?;
            let canonical = gore_as::cache::splice::remap_module_to_base_with_loadout_plan(
                &mini,
                &base,
                &loadout_plan,
            )
            .map_err(|e| ModError::Other(format!("canonicalize script mini {module}: {e}")))?;
            canonical_minis.push(crate::seal_script_mini(
                canonical,
                limits.max_mini_bytes,
                &mut canonical_output_bytes,
                limits.max_mini_total_bytes,
            )?);
        }
        drop(loadout_plan);

        // Pass 3 builds the guard only after the plan's large base context is gone. Reopen, verify,
        // and compose each tempfile in loadout order; consuming it cleans disk incrementally.
        let mut merge_guard = gore_as::cache::splice::SequentialMiniGuard::new(&base)
            .map_err(|e| ModError::Other(format!("prepare script composition: {e}")))?;
        let mut acc = base;
        let mut canonical_read_bytes = 0u64;
        for ((op, module, _), sealed) in scripts.iter().zip(canonical_minis) {
            let mini = crate::read_sealed_script_mini(
                &sealed,
                limits.max_mini_bytes,
                &mut canonical_read_bytes,
                limits.max_mini_total_bytes,
            )?;
            acc = match op.as_str() {
                "add" => merge_guard
                    .compose_add(&acc, &mini)
                    .map_err(|e| ModError::Other(format!("splice {module}: {e}")))?,
                // A multi-module mini edits and adds its modules as one unit.
                "edit" if gore_as::cache::walk_modules::module_count(&mini) > 1 => {
                    crate::require_multi_module_edit_target(&acc, &mini, module)?;
                    merge_guard
                        .compose_upsert(&acc, &mini)
                        .map_err(|e| ModError::Other(format!("replace {module}: {e}")))?
                }
                "edit" if winner_edits_after_add.contains(module) => merge_guard
                    .compose_edit_or_add(&acc, &mini, module)
                    .map_err(|e| ModError::Other(format!("replace or splice {module}: {e}")))?,
                "edit" => merge_guard
                    .compose_edit(&acc, &mini, module)
                    .map_err(|e| ModError::Other(format!("replace {module}: {e}")))?,
                other => {
                    return Err(ModError::Other(format!(
                        "invalid script op {other:?} for module {module:?}"
                    )));
                }
            };
            ensure_generated_fits(acc.len(), limits, &budget)?;
        }
        stage_generated_output(&mut plan, gp.script_cache.clone(), acc, limits, &mut budget)?;
    }

    // Any rawfile whose target was NOT further patched remains a whole-file replacement. Snapshot
    // it to a verified private temp and publish through DeployPlan's disk-backed write path. A
    // standalone script cache is the one content-aware raw target: validate the exact private
    // candidate within the in-memory patch-base envelope, then release that validation copy before
    // the original candidate is published. Any error still drops every candidate before the game
    // is touched.
    for (_identity, (target, source)) in rawfile_sources {
        let pristine_source = crate::select_pristine_source(&target, prior)?;
        plan.bind_backup_identity(&target, pristine_source.basis)?;
        // Script caches are decoded for structural validation, so enforce the in-memory ceiling at
        // the opened-file metadata gate instead of first copying up to the generic 8-GiB raw limit.
        let snapshot_limit = if target == gp.script_cache {
            limits.max_patch_base_bytes
        } else {
            limits.max_raw_file_bytes
        };
        let (candidate, len) = snapshot_raw_payload(&source, snapshot_limit, limits, &mut budget)?;
        if target == gp.script_cache {
            validate_standalone_script_candidate(&candidate, len, limits.max_patch_base_bytes)?;
        }
        plan.file_writes
            .push(crate::DiskWrite::seal(target, candidate)?);
    }

    // Loose files have no patch layer to fold onto them: the winning payload IS the final content.
    // Publish through the same disk-backed write path the rawfiles use, so even a multi-GiB
    // replacement never becomes a resident `Vec` and an apply error drops every candidate before
    // the game is touched.
    for (_folded, (target, source)) in loose_files {
        let pristine_source = crate::select_pristine_source(&target, prior)?;
        plan.bind_backup_identity(&target, pristine_source.basis)?;
        let candidate = snapshot_loose_payload(&source, limits, &mut budget)?;
        plan.file_writes
            .push(crate::DiskWrite::seal(target, candidate)?);
    }

    crate::prepare_voice_archive_writes(&voice, &gp, prior, &mut plan)?;

    // (6) The full plan is built — every fallible read/decode/cook above succeeded and the prior
    //     deployment is still intact. Commit directly against `prev`: commit_plan snapshots the
    //     exact active bytes, persists crash-recovery state, swaps the new loadout, and restores the
    //     old deployment + record on any late failure. It also reconciles prior leftovers and
    //     mirrors the manager footprint into legacy record fields.
    //
    // Validate self-colliding deploy targets BEFORE the destructive undeploy: commit_plan rejects a
    // duplicate-dst plan, but only AFTER this point, so without this pre-check a plan where two
    // enabled components map to the same dst would tear down the active deployment and THEN fail —
    // turning a rejected apply into a destructive one. Catch it here so a failed apply stays
    // non-destructive (commit_plan still re-checks for its other callers / the empty-live case).
    if let Some(dup) = crate::first_duplicate_dst(&plan) {
        return Err(ModError::Other(format!("duplicate deploy target: {dup}")));
    }
    let record = DeployRecord {
        owner: "manager".into(),
        mod_name: "<manager>".into(),
        manager_container_priority_schema: Some(crate::MANAGER_CONTAINER_PRIORITY_SCHEMA),
        loadout: loaded
            .iter()
            .map(|l| LoadoutEntry {
                id: l.entry.id.clone(),
                enabled: true,
            })
            .collect(),
        // Snapshot each deployed mod's content fingerprint so status can detect a same-id UPDATE
        // (a re-import that changed a mod's components but kept its id) — a loadout-id match alone
        // would otherwise report InSync over stale deployed bytes.
        deployed_fingerprints: loaded
            .iter()
            .map(|l| (l.entry.id.clone(), l.meta.fingerprint()))
            .collect(),
        ..Default::default()
    };
    crate::commit_plan(&gp, &abs_root, plan, record, prev)?;
    // Explicitly document/latch the lifetime: managed-pak source paths above point into these
    // private candidates and must remain present until commit has completed its streaming copies.
    drop(additive_candidates);
    drop(tree_snapshots);

    Ok(ApplyReport {
        applied: loaded.iter().map(|l| l.meta.name.clone()).collect(),
        warnings,
    })
}

/// Undeploy whatever the manager (or a studio deploy) has active at `game_root`. Thin wrapper over
/// [`crate::undeploy`]; `Ok(true)` if a deployment was present and undone, `Ok(false)` if nothing
/// was deployed.
pub fn undeploy_all(game_root: &Path) -> crate::Result<bool> {
    Ok(crate::undeploy(game_root)?.is_some())
}

/// Undeploy only a Manager-owned deployment. A Studio deploy is preserved and refused, including
/// if the deploy owner changes while this operation waits for install ownership.
pub fn undeploy_manager_only(game_root: &Path) -> crate::Result<bool> {
    Ok(crate::undeploy_manager(game_root)?.is_some())
}

// ── naming helpers ────────────────────────────────────────────────────────────────────────────

/// `<root>/G1R/Content/Paks/~mods` — where manager paks/triplets mount.
fn mods_dir(gp: &crate::GamePaths) -> PathBuf {
    gp.root
        .join("G1R")
        .join("Content")
        .join("Paks")
        .join("~mods")
}

/// Slot-prefixed pak stem for a foreign triplet whose `rel_base` file stem may already carry the
/// shipping `zzz_…_P` decoration: strip a leading `zzz_` and trailing `_P`, then re-wrap as
/// `zzz_gm{idx:03}_{sanitized}_{idx + 1}_P`. The `gm` field keeps manager targets unique; only the
/// final numeric field is interpreted by Unreal as patch priority.
fn slot_pak_stem(rel_base: &str, idx: usize) -> crate::Result<String> {
    let raw = Path::new(rel_base)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(rel_base);
    slot_stem(raw, idx)
}

/// Wrap `raw` (a bare mod/pak name) as the slot-prefixed `~mods` stem
/// `zzz_gm{idx:03}_{clean}_{idx + 1}_P`, where `clean` has any leading `zzz_` / trailing `_P`
/// stripped and is sanitized to a safe stem.
fn slot_stem(raw: &str, idx: usize) -> crate::Result<String> {
    let mut s = raw;
    if let Some(rest) = s.strip_prefix("zzz_") {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix("_P") {
        s = rest;
    }
    Ok(format!(
        "zzz_gm{:03}_{}_{}_P",
        idx,
        sanitize_stem(s),
        manager_patch_priority(idx)?
    ))
}

/// Manager loadout indices are zero-based, while their Unreal patch versions are deliberately
/// one-based. Loadout validation caps enabled entries at 1,000, but keep this helper fail-closed so
/// future direct callers cannot wrap a `usize` into a low-priority filename.
fn manager_patch_priority(idx: usize) -> crate::Result<usize> {
    idx.checked_add(1)
        .ok_or_else(|| ModError::Other("manager patch priority overflowed".into()))
}

/// The shared texture/pak-file builders keep their stable single-mod output names. Manager Apply
/// changes only the generated *destination* name, inserting its strict loadout priority directly
/// before `_P`; the temporary source and its lifetime stay untouched.
fn prioritize_generated_containers(
    containers: Vec<(PathBuf, PathBuf)>,
    idx: usize,
) -> crate::Result<Vec<(PathBuf, PathBuf)>> {
    let priority = manager_patch_priority(idx)?;
    containers
        .into_iter()
        .map(|(src, dst)| {
            let extension = dst
                .extension()
                .and_then(|value| value.to_str())
                .filter(|value| matches!(*value, "pak" | "utoc" | "ucas"))
                .ok_or_else(|| {
                    ModError::Other(format!(
                        "generated manager container has an unsupported destination: {}",
                        dst.display()
                    ))
                })?;
            let stem = dst
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    ModError::Other(format!(
                        "generated manager container has a non-portable destination: {}",
                        dst.display()
                    ))
                })?;
            let base = stem.strip_suffix("_P").ok_or_else(|| {
                ModError::Other(format!(
                    "generated manager container is missing its _P suffix: {}",
                    dst.display()
                ))
            })?;
            let prioritized = dst.with_file_name(format!("{base}_{priority}_P.{extension}"));
            Ok((src, prioritized))
        })
        .collect()
}

/// Fold anything that isn't `[A-Za-z0-9_-]` to `_` (mirrors lib.rs `sanitize`), so a pak name can't
/// introduce path separators or other unsafe characters into the `~mods` filename.
fn sanitize_stem(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mgr::loadout::Loadout;
    use crate::mgr::model::{ComponentInfo, ModEntryMeta, ModKind, RawTarget, META_FILE};
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
    use aes::Aes256;
    use std::fs;

    #[cfg(unix)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test file symlink failed: {error}"),
        }
    }

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

    // ── .lcache fixture (mirrors gore-loc's own test encoder — DO NOT edit gore-loc) ────────────
    // The whole file is AES-256-ECB encrypted with the 32 ASCII bytes below (the exact key
    // gore-loc uses, copied here so the fixture is a real, decodable .lcache without reaching into
    // gore-loc's private test helpers).
    const LCACHE_AES_KEY: &[u8; 32] = b"8f93ff6fa254d9c536ad88c1ff1d812b";

    /// One FString: i32 count; empty→lone 0; ASCII→utf8+NUL (positive byte count); else
    /// utf16le+NUL (negative unit count). Byte-identical to gore-loc's `encode_fstring`.
    fn fstr(s: &str) -> Vec<u8> {
        if s.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut out = Vec::new();
        if s.is_ascii() {
            let mut raw = s.as_bytes().to_vec();
            raw.push(0);
            out.extend_from_slice(&(raw.len() as i32).to_le_bytes());
            out.extend_from_slice(&raw);
        } else {
            let units: Vec<u16> = s.encode_utf16().collect();
            let mut raw = Vec::with_capacity(units.len() * 2 + 2);
            for u in &units {
                raw.extend_from_slice(&u.to_le_bytes());
            }
            raw.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&(-((raw.len() / 2) as i32)).to_le_bytes());
            out.extend_from_slice(&raw);
        }
        out
    }

    fn aes_ecb_encrypt(plain: &[u8]) -> Vec<u8> {
        let cipher = Aes256::new(GenericArray::from_slice(LCACHE_AES_KEY));
        let mut out = plain.to_vec();
        for chunk in out.chunks_mut(16) {
            cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
        }
        out
    }

    /// Build a real encrypted 1-language `.lcache` with the given `(key, value)` records, all in
    /// language `german`. Two keys by default let a loc-last-wins test share one id across mods.
    fn build_lcache(records: &[(&str, &str)]) -> Vec<u8> {
        // The shipped cache is ordinally sorted by id and runtime lookup relies on that order.
        let mut records = records.to_vec();
        records.sort_by(|a, b| a.0.cmp(b.0));
        let mut plain = Vec::new();
        plain.push(0u8); // prefix
        plain.extend_from_slice(&(b"LCACHE".len() as i32).to_le_bytes());
        plain.extend_from_slice(b"LCACHE");
        plain.extend_from_slice(&1i32.to_le_bytes()); // lang_count
        plain.extend_from_slice(&fstr("german"));
        plain.extend_from_slice(&(records.len() as i32).to_le_bytes()); // group_count
        for (key, val) in records {
            // main record: key + 1 pair (german → value)
            plain.extend_from_slice(&fstr(key));
            plain.extend_from_slice(&1i32.to_le_bytes());
            plain.extend_from_slice(&fstr("german"));
            plain.extend_from_slice(&fstr(val));
            // meta record: empty key, no pairs
            plain.extend_from_slice(&fstr(""));
            plain.extend_from_slice(&0i32.to_le_bytes());
        }
        let pad = (16 - (plain.len() % 16)) % 16;
        plain.resize(plain.len() + pad, 0);
        aes_ecb_encrypt(&plain)
    }

    /// A `.lcache` whose header declares `langs` and whose one record carries `pairs`.
    ///
    /// `build_lcache` above declares `german` alone, which cannot express the case the shadow
    /// warning is about: an id carrying both a generation and a newer one.
    fn build_lcache_with_langs(langs: &[&str], key: &str, pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut plain = Vec::new();
        plain.push(0u8); // prefix
        plain.extend_from_slice(&(b"LCACHE".len() as i32).to_le_bytes());
        plain.extend_from_slice(b"LCACHE");
        plain.extend_from_slice(&(langs.len() as i32).to_le_bytes());
        for language in langs {
            plain.extend_from_slice(&fstr(language));
        }
        plain.extend_from_slice(&1i32.to_le_bytes()); // group_count
        plain.extend_from_slice(&fstr(key));
        plain.extend_from_slice(&(pairs.len() as i32).to_le_bytes());
        for (language, value) in pairs {
            plain.extend_from_slice(&fstr(language));
            plain.extend_from_slice(&fstr(value));
        }
        plain.extend_from_slice(&fstr("")); // meta record
        plain.extend_from_slice(&0i32.to_le_bytes());
        let pad = (16 - (plain.len() % 16)) % 16;
        plain.resize(plain.len() + pad, 0);
        aes_ecb_encrypt(&plain)
    }

    /// Decrypt+decode a `.lcache` and read a single (key, german) value for assertions.
    fn read_loc(bytes: &[u8], key: &str) -> String {
        let lc = gore_loc::loc::Lcache::decode(bytes).unwrap();
        lc.export(false)
            .get(key)
            .and_then(|m| m.get("german"))
            .cloned()
            .unwrap_or_default()
    }

    // ── audio bank fixture (a REAL pristine RIFF/FEV bank the audio arm can inject into) ─────────
    // gore-fmod ships no bank-builder: `build_fsb5_pcm16` emits only the inner FSB5 block (magic
    // `FSB5`), which is NOT a `.bank` — `parse_bank`/`is_pristine_bank` reject it ("not a RIFF/FEV
    // bank"). The audio arm reads the base bank via `read_pristine` (whose no-backup branch gates on
    // `is_pristine_bank`) and calls `gore_fmod::replace_samples`, which needs a full RIFF/FEV
    // wrapper: a top-level LIST/PROJ/BNKI region with an SNDH entry pointing at the FSB5 and a WAV
    // node referencing (SoundBankIndex 0, SubsoundIndex 0). So we hand-roll that minimal wrapper
    // here around a `build_fsb5_pcm16` FSB5. Byte layout is validated by both gore-fmod walkers
    // (`parse_bank` and the private `gather`/`inject_fsb5`), which frame every sub-chunk as
    // `[fourcc][u32 size][body]` starting right after PROJ — so BNKI carries its own size too.

    /// One PCM16 sample named `sample` (mono, `freq` Hz) wrapped in a pristine `.bank`, encrypted
    /// with `key`. `is_pristine_bank` returns true and `replace_samples` accepts it as the base.
    fn build_pristine_bank(sample: &str, freq: u32, pcm: &[i16], key: &[u8]) -> Vec<u8> {
        use gore_fmod::{build_fsb5_pcm16, fsb5_encrypt};
        let mut fsb5 = build_fsb5_pcm16(sample, freq, 1, pcm).unwrap();
        fsb5_encrypt(&mut fsb5, key);
        let u32b = |v: u32| v.to_le_bytes();

        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&u32b(0)); // riff size @0x04 (backpatched)
        b.extend_from_slice(b"FEV ");
        // FMT chunk @0x0C: parse_bank reads the bank version at absolute 0x14, so FMT's body must
        // start there. version 0x30 (>0x28) → 8-byte SNDH entries carrying explicit FSB5 sizes.
        b.extend_from_slice(b"FMT ");
        let fmt_size_pos = b.len(); // 0x10
        b.extend_from_slice(&u32b(0));
        assert_eq!(b.len(), 0x14, "FMT body must land at 0x14");
        b.extend_from_slice(&u32b(0x30)); // version
        b.extend_from_slice(&u32b(0)); // filler
        let fmt_size = (b.len() - (fmt_size_pos + 4)) as u32;
        b[fmt_size_pos..fmt_size_pos + 4].copy_from_slice(&u32b(fmt_size));

        // Top-level LIST enclosing PROJ/BNKI and the sub-chunks.
        b.extend_from_slice(b"LIST");
        let list_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let list_body = b.len();
        b.extend_from_slice(b"PROJ");
        // Both walkers begin at list_body+4 (right after PROJ) framing chunks as
        // [fourcc][u32 size][body]; BNKI is the first sub-chunk header → give it a size.
        b.extend_from_slice(b"BNKI");
        b.extend_from_slice(&u32b(0)); // empty BNKI body

        // SNDH: 4-byte chunk-version prefix (its low 2 bytes double as the injector's X16 count)
        // + one 8-byte entry (abs FSB5 offset, FSB5 size).
        b.extend_from_slice(b"SNDH");
        let sndh_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let sndh_body = b.len();
        b.extend_from_slice(&[2u8, 0, 0, 0]); // X16 count = 1 (1<<1)
        let sndh_entry = b.len();
        b.extend_from_slice(&u32b(0)); // entry.offset (backpatched)
        b.extend_from_slice(&u32b(0)); // entry.size   (backpatched)
        let sndh_size = (b.len() - sndh_body) as u32;
        b[sndh_size_pos..sndh_size_pos + 4].copy_from_slice(&u32b(sndh_size));

        // WAV node: body ≥ 0x1A; SoundBankIndex (i32 @+0x12) and SubsoundIndex (i32 @+0x16) = (0,0),
        // so inject_fsb5 repoints (0,0) → the appended FSB5.
        b.extend_from_slice(b"WAV ");
        let wav_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let wav_body = b.len();
        b.extend_from_slice(&[0u8; 0x1A]);
        let wav_size = (b.len() - wav_body) as u32;
        b[wav_size_pos..wav_size_pos + 4].copy_from_slice(&u32b(wav_size));

        let list_size = (b.len() - list_body) as u32;
        b[list_size_pos..list_size_pos + 4].copy_from_slice(&u32b(list_size));

        // SND chunk carrying the encrypted FSB5, 32-aligned.
        b.extend_from_slice(b"SND ");
        let snd_size_pos = b.len();
        b.extend_from_slice(&u32b(0));
        let pad = (32 - (b.len() % 32)) % 32;
        b.resize(b.len() + pad, 0);
        let fsb5_abs = b.len() as u32;
        b.extend_from_slice(&fsb5);
        let snd_size = (b.len() - (snd_size_pos + 4)) as u32;
        b[snd_size_pos..snd_size_pos + 4].copy_from_slice(&u32b(snd_size));

        b[sndh_entry..sndh_entry + 4].copy_from_slice(&u32b(fsb5_abs));
        b[sndh_entry + 4..sndh_entry + 8].copy_from_slice(&u32b(fsb5.len() as u32));
        let riff = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&u32b(riff));
        // Sanity: our synthetic bank satisfies the exact gate the audio arm relies on.
        assert!(
            gore_fmod::is_pristine_bank(&b),
            "fixture bank must be pristine"
        );
        b
    }

    /// Decode the LAST FSB5 in `bank` and return its first sample's interleaved PCM16 frames.
    /// After `replace_samples` the injected sample lives in the appended FSB5 (#1); on a pristine
    /// bank this is FSB5 #0 (the original). Lets a test assert the LIVE bank now carries the
    /// injected pattern (proving materialization, not merely that the file changed).
    fn decode_last_fsb5_pcm(bank: &[u8], key: &[u8]) -> Vec<i16> {
        let entries = gore_fmod::parse_bank(bank).expect("parse_bank");
        let e = entries.last().expect("bank has an FSB5");
        let mut blk = bank[e.fsb5_offset..e.fsb5_offset + e.fsb5_size].to_vec();
        gore_fmod::fsb5_decrypt(&mut blk, key);
        let fsb = gore_fmod::parse_fsb5(&blk).expect("parse_fsb5");
        assert_eq!(fsb.codec, gore_fmod::Codec::Pcm16, "expected PCM16 FSB5");
        let s = &fsb.samples[0];
        let start = (fsb.data_section + s.data_offset) as usize;
        let end = start + s.num_samples as usize * s.channels as usize * 2;
        blk[start..end]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    }

    // ── precompiled-script cache fixture (a REAL cache the script arm can splice) ────────────────
    // gore-as's own splice tests read their `.Cache` samples from `work/reversing/gore-as/samples/`
    // — a gitignored scratch dir NOT present in-tree (only an 8 KB header slice is committed as a
    // fixture, and it is not a spliceable module cache). Rather than depend on those, we synthesize
    // a minimal-but-VALID cache from the documented wire format (header.rs / wire.rs / walk_modules
    // / tables.rs): a 0x18 header (16-byte hash + magic + Modules count), N module TMap entries with
    // zero functions/classes/enums/globals/imports, and 7 EMPTY global tail tables (28 zero bytes).
    // This is exactly the shape `splice_auto`'s case-(b) fast path and `replace_module` accept, and
    // it round-trips through `module_count`/`module_names`/`module_region_end` — verified by the
    // tests below driving the REAL `gore_as::cache::splice` functions, no game bytes required.

    /// `FStringInArchive`: i32 length (chars); if >0, `length+1` bytes incl trailing NUL.
    fn as_sia(s: &str) -> Vec<u8> {
        if s.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut o = (s.len() as i32).to_le_bytes().to_vec();
        o.extend_from_slice(s.as_bytes());
        o.push(0);
        o
    }

    /// UE `FString` (the `Modules` TMap key): i32 len (= chars+1 incl NUL); then `len` bytes.
    fn as_fstring(s: &str) -> Vec<u8> {
        if s.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut o = ((s.len() + 1) as i32).to_le_bytes().to_vec();
        o.extend_from_slice(s.as_bytes());
        o.push(0);
        o
    }

    /// One `FAngelscriptPrecompiledModule` value with no members (all count-prefixed arrays empty),
    /// laid out exactly as `walk_modules::read_module` consumes it.
    fn as_module_value(module: &str) -> Vec<u8> {
        let mut m = Vec::new();
        m.extend_from_slice(&as_sia(module)); // ModuleName
        for _ in 0..5 {
            m.extend_from_slice(&0i32.to_le_bytes()); // Functions/Classes/Enums/Globals/Imports = 0
        }
        m.extend_from_slice(&0i64.to_le_bytes()); // CodeHash
        m.extend_from_slice(&0i32.to_le_bytes()); // ImportedModules (TArray<SIA>) = 0
        m.extend_from_slice(&as_sia("")); // StaticsClassName
        m.extend_from_slice(&0i32.to_le_bytes()); // DeclaredEvents = 0
        m.extend_from_slice(&0i32.to_le_bytes()); // DeclaredDelegates = 0
        m.extend_from_slice(&as_sia("")); // ScriptRelativeFilename
        m.extend_from_slice(&0i32.to_le_bytes()); // PostInitFunctions = 0
        m
    }

    /// A full minimal precompiled cache carrying `modules` (each a name→empty-module TMap entry)
    /// with empty global tail tables. Accepted by `splice_auto`/`replace_module`.
    fn build_script_cache(modules: &[&str]) -> Vec<u8> {
        use gore_as::cache::header::CACHE_MAGIC;
        let mut out = Vec::new();
        out.extend_from_slice(&[0u8; 16]); // 16-byte validation hash (unchecked by the loader)
        out.extend_from_slice(&CACHE_MAGIC.to_le_bytes()); // magic @0x10
        out.extend_from_slice(&(modules.len() as u32).to_le_bytes()); // Modules count @0x14
        for name in modules {
            out.extend_from_slice(&as_fstring(name)); // TMap key
            out.extend_from_slice(&as_module_value(name)); // TMap value
        }
        out.extend_from_slice(&[0u8; 7 * 4]); // 7 empty tail tables
        out
    }

    /// One-module allow-new-shaped cache with `STR 0`, private T1â€“T7 rows, and deterministic
    /// synthetic pointer/id keys. This exercises manager composition beyond the original empty
    /// T1â€“T5/T7 fixture that missed real class-module collisions.
    fn probe_data_type(token: i32) -> Vec<u8> {
        let mut out = vec![0u8; 24]; // six serialized bool words
        out.extend_from_slice(&0i64.to_le_bytes()); // primitive TypeInfo
        out.extend_from_slice(&token.to_le_bytes());
        out
    }

    fn probe_function(name: &str, bytecode: &[i32], id: i32) -> Vec<u8> {
        let mut out = as_sia(name);
        out.extend_from_slice(&as_sia("")); // namespace
        out.extend_from_slice(&probe_data_type(0x52)); // void return
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter types
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter names
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter flags
        out.extend_from_slice(&0i32.to_le_bytes()); // parameter defaults
        out.extend_from_slice(&0i32.to_le_bytes()); // traits (non-const)
        out.extend_from_slice(&(bytecode.len() as i32).to_le_bytes());
        for &word in bytecode {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&0i32.to_le_bytes()); // bytecode references
        out.extend_from_slice(&0i32.to_le_bytes()); // variable space
        out.extend_from_slice(&0i32.to_le_bytes()); // object variable types
        out.extend_from_slice(&0i32.to_le_bytes()); // object variable positions
        out.extend_from_slice(&0i32.to_le_bytes()); // object variables on heap
        out.extend_from_slice(&0i32.to_le_bytes()); // variable info program positions
        out.extend_from_slice(&0i32.to_le_bytes()); // variable info offsets
        out.extend_from_slice(&0i32.to_le_bytes()); // variable info options
        out.extend_from_slice(&0i32.to_le_bytes()); // stack needed
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // declared at
        out.extend_from_slice(&0i32.to_le_bytes()); // line numbers
        out.extend_from_slice(&0i32.to_le_bytes()); // not a UFunction
        out
    }

    fn probe_property(name: &str) -> Vec<u8> {
        let mut out = as_sia(name);
        out.extend_from_slice(&probe_data_type(0x44)); // int
        out.extend_from_slice(&0i32.to_le_bytes()); // not private
        out.extend_from_slice(&0i32.to_le_bytes()); // not protected
        out.extend_from_slice(&0i32.to_le_bytes()); // no Unreal property tail
        out
    }

    fn probe_class(type_name: &str, method_name: &str, property_name: &str, id: i32) -> Vec<u8> {
        let mut out = as_sia(type_name);
        out.extend_from_slice(&as_sia("")); // namespace
        out.extend_from_slice(&0i32.to_le_bytes()); // flags
        out.extend_from_slice(&1i32.to_le_bytes()); // properties
        out.extend_from_slice(&probe_property(property_name));
        out.extend_from_slice(&1i32.to_le_bytes()); // methods
        out.extend_from_slice(&probe_function(method_name, &[10], id)); // RET
        out.extend_from_slice(&1i32.to_le_bytes()); // method table
        out.extend_from_slice(&0i32.to_le_bytes()); // Methods[0]
        out.extend_from_slice(&0i64.to_le_bytes()); // derived from
        out.extend_from_slice(&0i64.to_le_bytes()); // shadow type
        out.extend_from_slice(&0i32.to_le_bytes()); // constructors
        out.extend_from_slice(&0i32.to_le_bytes()); // factory refs
        out.extend_from_slice(&7i32.to_le_bytes()); // fixed behavior slots 0..6
        out.extend_from_slice(&[0u8; 7 * 8]);
        out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
        out.extend_from_slice(&0i32.to_le_bytes()); // behavior function types
        out.extend_from_slice(&0i32.to_le_bytes()); // no Unreal class tail
        out
    }

    fn probe_global(name: &str) -> Vec<u8> {
        let mut out = as_sia(name);
        out.extend_from_slice(&as_sia("")); // namespace
        out.extend_from_slice(&probe_data_type(0x44)); // int
        out.extend_from_slice(&1i32.to_le_bytes()); // default initialized
        out
    }

    fn build_script_cache_with_static_name_and_keys(
        module: &str,
        name: &str,
        type_name: &str,
        identity_seed: i32,
        key_seed: i32,
        type_id: i32,
    ) -> Vec<u8> {
        use gore_as::cache::header::CACHE_MAGIC;
        let method_name = format!("ProbeFunc{identity_seed}");
        let property_name = format!("ProbeField{identity_seed}");
        let global_name = format!("ProbeGlobal{identity_seed}");

        let mut out = vec![0u8; 16];
        out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&as_fstring(module));
        out.extend_from_slice(&as_sia(module));
        out.extend_from_slice(&1i32.to_le_bytes()); // functions
        out.extend_from_slice(&probe_function(
            "StaticNameProbe",
            &[60, 10], // STR StaticNames[0], RET
            0x1200_0000i32 + key_seed,
        ));
        out.extend_from_slice(&1i32.to_le_bytes()); // classes
        out.extend_from_slice(&probe_class(
            &type_name,
            &method_name,
            &property_name,
            0x1300_0000i32 + key_seed,
        ));
        out.extend_from_slice(&0i32.to_le_bytes()); // enums
        out.extend_from_slice(&1i32.to_le_bytes()); // globals
        out.extend_from_slice(&probe_global(&global_name));
        out.extend_from_slice(&0i32.to_le_bytes()); // imports
        out.extend_from_slice(&0i64.to_le_bytes()); // code hash
        out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
        out.extend_from_slice(&as_sia("")); // statics class
        out.extend_from_slice(&[0u8; 2 * 4]); // events/delegates
        out.extend_from_slice(&as_sia("")); // relative filename
        out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions

        let type_ptr = 0x6000_0000_1000_0000i64 + i64::from(key_seed);
        let func_ptr = 0x6000_0000_2000_0000i64 + i64::from(key_seed);
        let global_ptr = 0x6000_0000_3000_0000i64 + i64::from(key_seed);
        let func_id = 1_000_000i32 + key_seed;

        out.extend_from_slice(&1u32.to_le_bytes()); // T1 type
        out.extend_from_slice(&type_ptr.to_le_bytes());
        out.extend_from_slice(&as_sia(type_name));
        out.extend_from_slice(&as_sia(module));
        out.extend_from_slice(&as_sia(""));
        out.extend_from_slice(&0u32.to_le_bytes()); // no subtypes

        out.extend_from_slice(&1u32.to_le_bytes()); // T2 type id -> ptr
        out.extend_from_slice(&type_id.to_le_bytes());
        out.extend_from_slice(&type_ptr.to_le_bytes());

        out.extend_from_slice(&1u32.to_le_bytes()); // T3 function
        out.extend_from_slice(&func_ptr.to_le_bytes());
        out.extend_from_slice(&as_sia(&method_name));
        out.extend_from_slice(&as_sia(module));
        out.extend_from_slice(&as_sia(""));
        out.extend_from_slice(&0i32.to_le_bytes()); // not const
        out.extend_from_slice(&0i32.to_le_bytes()); // not imported
        out.extend_from_slice(&1i32.to_le_bytes()); // method: concrete owner below
        out.extend_from_slice(&type_ptr.to_le_bytes()); // owner
        out.extend_from_slice(&0u32.to_le_bytes()); // no params
        out.extend_from_slice(&[0u8; 24]); // void return DataType flags
        out.extend_from_slice(&0i64.to_le_bytes());
        out.extend_from_slice(&0x52i32.to_le_bytes());

        out.extend_from_slice(&1u32.to_le_bytes()); // T4 function id -> ptr
        out.extend_from_slice(&func_id.to_le_bytes());
        out.extend_from_slice(&func_ptr.to_le_bytes());

        out.extend_from_slice(&1u32.to_le_bytes()); // T5 global
        out.extend_from_slice(&global_ptr.to_le_bytes());
        out.extend_from_slice(&as_sia(&global_name));
        out.extend_from_slice(&as_sia(module));
        out.extend_from_slice(&as_sia(""));
        out.extend_from_slice(&0i32.to_le_bytes()); // not a string

        out.extend_from_slice(&1u32.to_le_bytes()); // T6 count
        out.extend_from_slice(&as_sia(name));

        out.extend_from_slice(&1u32.to_le_bytes()); // T7 property
        let property_key = (i64::from(type_id) << 1) | (4i64 << 33) | 1;
        out.extend_from_slice(&property_key.to_le_bytes());
        out.extend_from_slice(&as_sia(&property_name));
        out.extend_from_slice(&type_id.to_le_bytes());
        out
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ScriptAssignments {
        pointers: BTreeMap<String, i64>,
        type_ids: BTreeMap<String, i32>,
        function_ids: BTreeMap<String, i32>,
    }

    fn script_assignments(bytes: &[u8]) -> ScriptAssignments {
        use gore_as::cache::tables::parse_tail_tables;
        use gore_as::cache::walk_modules::module_region_end;
        use gore_as::cache::wire::Cursor;

        let tail = parse_tail_tables(bytes, module_region_end(bytes).unwrap()).unwrap();
        let mut type_names = BTreeMap::new();
        let mut function_names = BTreeMap::new();
        let mut pointers = BTreeMap::new();
        for &start in &tail.tables[0].entry_starts {
            let mut cursor = Cursor::at(bytes, start);
            let ptr = cursor.read_i64().unwrap();
            let name = cursor.read_sia().unwrap();
            type_names.insert(ptr, name.clone());
            pointers.insert(format!("T1:{name}"), ptr);
        }
        for &start in &tail.tables[2].entry_starts {
            let mut cursor = Cursor::at(bytes, start);
            let ptr = cursor.read_i64().unwrap();
            let name = cursor.read_sia().unwrap();
            function_names.insert(ptr, name.clone());
            pointers.insert(format!("T3:{name}"), ptr);
        }
        for &start in &tail.tables[4].entry_starts {
            let mut cursor = Cursor::at(bytes, start);
            let ptr = cursor.read_i64().unwrap();
            let name = cursor.read_sia().unwrap();
            pointers.insert(format!("T5:{name}"), ptr);
        }
        let mut type_ids = BTreeMap::new();
        for &start in &tail.tables[1].entry_starts {
            let mut cursor = Cursor::at(bytes, start);
            let id = cursor.read_i32().unwrap();
            let ptr = cursor.read_i64().unwrap();
            if let Some(name) = type_names.get(&ptr) {
                type_ids.insert(name.clone(), id);
            }
        }
        let mut function_ids = BTreeMap::new();
        for &start in &tail.tables[3].entry_starts {
            let mut cursor = Cursor::at(bytes, start);
            let id = cursor.read_i32().unwrap();
            let ptr = cursor.read_i64().unwrap();
            if let Some(name) = function_names.get(&ptr) {
                function_ids.insert(name.clone(), id);
            }
        }
        ScriptAssignments {
            pointers,
            type_ids,
            function_ids,
        }
    }

    // ── fake game tree ──────────────────────────────────────────────────────────────────────────

    struct FakeGame {
        _tmp: tempfile::TempDir,
        root: PathBuf,
        lib: PathBuf,
    }

    impl FakeGame {
        /// A minimal on-disk game tree with the dirs deploy touches, a 2-record pristine `.lcache`,
        /// an empty `~mods`, and an (empty) library dir.
        fn new() -> FakeGame {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().join("game");
            for p in [
                "G1R/Binaries/Win64/ue4ss/Mods",
                "G1R/Content/FMOD/Desktop",
                "G1R/Content/Paks/~mods",
                "G1R/Script",
                "G1R/Story/Cache",
                "G1R/Story/VoiceOver",
            ] {
                fs::create_dir_all(root.join(p)).unwrap();
            }
            // Pristine localization the deploy will patch against.
            fs::write(
                root.join("G1R/Story/Cache/AlkimiaLocalization_0.lcache"),
                build_lcache(&[("itfo_cheese", "Cheese"), ("itfo_apple", "Apple")]),
            )
            .unwrap();
            let lib = tmp.path().join("library");
            fs::create_dir_all(&lib).unwrap();
            FakeGame {
                _tmp: tmp,
                root,
                lib,
            }
        }

        fn lcache(&self) -> PathBuf {
            self.root
                .join("G1R/Story/Cache/AlkimiaLocalization_0.lcache")
        }
        fn mods(&self) -> PathBuf {
            self.root.join("G1R/Content/Paks/~mods")
        }
        /// `<root>/G1R/Content/FMOD/Desktop/<name>` — where `RawTarget::Bank`/audio patches land.
        fn bank(&self, name: &str) -> PathBuf {
            self.root.join("G1R/Content/FMOD/Desktop").join(name)
        }
        /// `<root>/G1R/Script/PrecompiledScript_Shipping.Cache` — the script-cache live target.
        fn script_cache(&self) -> PathBuf {
            self.root
                .join("G1R/Script/PrecompiledScript_Shipping.Cache")
        }

        fn voice_archive(&self, name: &str) -> PathBuf {
            self.root.join("G1R/Story/VoiceOver").join(name)
        }

        fn add_voice_mod(
            &self,
            id: &str,
            name: &str,
            archive: &str,
            op: crate::VoicePatchOp,
            archive_path: &str,
            ogg: &[u8],
        ) -> String {
            let payload = "voice/payload/0.ogg";
            let manifest = crate::VoicePatchManifest {
                format: 1,
                executable_generation: None,
                edits: vec![crate::VoicePatchEntry {
                    archive: archive.into(),
                    op,
                    archive_path: archive_path.into(),
                    ogg: payload.into(),
                    observation: None,
                    payload_seal: None,
                }],
            };
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::VoiceArchivePatch {
                    rel: "voice".into(),
                    targets: vec![format!("{archive}|{archive_path}")],
                }],
                |dir| {
                    fs::create_dir_all(dir.join("voice/payload")).unwrap();
                    fs::write(
                        dir.join("voice/manifest.json"),
                        serde_json::to_vec_pretty(&manifest).unwrap(),
                    )
                    .unwrap();
                    fs::write(dir.join(payload), ogg).unwrap();
                },
            )
        }

        /// Add an audio-patch mod: `audio/manifest.json` = {bank:{sample:wav_rel}} plus the WAV.
        fn add_audio_mod(
            &self,
            id: &str,
            name: &str,
            bank: &str,
            sample: &str,
            wav: &[u8],
        ) -> String {
            let wav_rel = format!("audio/0_{sample}.wav");
            let mut manifest: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            manifest
                .entry(bank.into())
                .or_default()
                .insert(sample.into(), wav_rel.clone());
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::AudioPatch {
                    rel: "audio".into(),
                    targets: vec![],
                }],
                |dir| {
                    fs::create_dir_all(dir.join("audio")).unwrap();
                    fs::write(
                        dir.join("audio/manifest.json"),
                        serde_json::to_vec(&manifest).unwrap(),
                    )
                    .unwrap();
                    fs::write(dir.join(&wav_rel), wav).unwrap();
                },
            )
        }

        /// Add an AngelScript-patch mod: `scripts/manifest.json` = [{op,module,mini}] + the mini cache.
        fn add_script_mod(
            &self,
            id: &str,
            name: &str,
            op: &str,
            module: &str,
            mini: &[u8],
        ) -> String {
            let mini_rel = "scripts/0_mod.cache".to_string();
            let entries = vec![crate::ScriptEntry {
                op: op.into(),
                module: module.into(),
                mini: mini_rel.clone(),
            }];
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::AngelScriptPatch {
                    rel: "scripts".into(),
                    targets: vec![],
                }],
                |dir| {
                    fs::create_dir_all(dir.join("scripts")).unwrap();
                    fs::write(
                        dir.join("scripts/manifest.json"),
                        serde_json::to_vec(&entries).unwrap(),
                    )
                    .unwrap();
                    fs::write(dir.join(&mini_rel), mini).unwrap();
                },
            )
        }

        /// Add a rawfile mod that replaces one whole game file (`target`) with `bytes`.
        fn add_rawfile_mod(&self, id: &str, name: &str, target: RawTarget, bytes: &[u8]) -> String {
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::RawFile {
                    rel: "raw.bin".into(),
                    target_file: target,
                }],
                |dir| fs::write(dir.join("raw.bin"), bytes).unwrap(),
            )
        }

        /// Materialize a library mod dir `<lib>/<id>/` with a sidecar for `components`, then
        /// return its id. `write_payload` runs against the entry dir so the caller can drop the
        /// payload files each component points at.
        fn add_mod(
            &self,
            id: &str,
            name: &str,
            components: Vec<ComponentInfo>,
            write_payload: impl FnOnce(&Path),
        ) -> String {
            let dir = self.lib.join(id);
            fs::create_dir_all(&dir).unwrap();
            let meta = ModEntryMeta {
                id: id.into(),
                kind: ModKind::Goremod,
                name: name.into(),
                version: String::new(),
                author: String::new(),
                imported_at: "2026-07-03T00:00:00Z".into(),
                source: String::new(),
                components,
            };
            fs::write(
                dir.join(META_FILE),
                serde_json::to_vec_pretty(&meta).unwrap(),
            )
            .unwrap();
            write_payload(&dir);
            id.to_string()
        }

        /// Add a loc-patch mod editing one (id → german) value.
        fn add_loc_mod(&self, id: &str, name: &str, loc_id: &str, value: &str) -> String {
            let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            edits
                .entry(loc_id.into())
                .or_default()
                .insert("german".into(), value.into());
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::LocPatch {
                    rel: "loc/edits.json".into(),
                    targets: vec![],
                }],
                |dir| {
                    fs::create_dir_all(dir.join("loc")).unwrap();
                    fs::write(
                        dir.join("loc/edits.json"),
                        serde_json::to_vec(&edits).unwrap(),
                    )
                    .unwrap();
                },
            )
        }

        /// `<root>/<game_path>` — the live file a loose-file component replaces.
        fn loose_file(&self, game_path: &str) -> PathBuf {
            self.root.join(crate::loose_relative_os_path(game_path))
        }

        /// Add a loose-file mod: `files/manifest.json` = {game_path: payload_rel} plus the payload.
        fn add_loose_file_mod(
            &self,
            id: &str,
            name: &str,
            game_path: &str,
            bytes: &[u8],
        ) -> String {
            let payload_rel = "files/0_payload".to_string();
            let manifest: BTreeMap<String, String> =
                BTreeMap::from([(game_path.to_string(), payload_rel.clone())]);
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::FilePatch {
                    rel: "files".into(),
                    targets: vec![game_path.into()],
                }],
                |dir| {
                    fs::create_dir_all(dir.join("files")).unwrap();
                    fs::write(
                        dir.join("files/manifest.json"),
                        serde_json::to_vec(&manifest).unwrap(),
                    )
                    .unwrap();
                    fs::write(dir.join(&payload_rel), bytes).unwrap();
                },
            )
        }

        /// Add one manager-library `PakFilePatch`. The payload manifest is the same shape as a
        /// loose `FilePatch`; apply must materialize it as an additive manager-owned pak instead of
        /// replacing the destination in place.
        fn add_pak_file_mod(&self, id: &str, name: &str, game_path: &str, bytes: &[u8]) -> String {
            let payload_rel = "pak_files/0_payload".to_string();
            let manifest: BTreeMap<String, String> =
                BTreeMap::from([(game_path.to_string(), payload_rel.clone())]);
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::PakFilePatch {
                    rel: "pak_files".into(),
                    targets: vec![game_path.into()],
                }],
                |dir| {
                    fs::create_dir_all(dir.join("pak_files")).unwrap();
                    fs::write(
                        dir.join("pak_files/manifest.json"),
                        serde_json::to_vec(&manifest).unwrap(),
                    )
                    .unwrap();
                    fs::write(dir.join(&payload_rel), bytes).unwrap();
                },
            )
        }

        /// Add a loose-pak mod (a single `<stem>.pak`).
        fn add_pak_mod(&self, id: &str, name: &str, pak_stem: &str, bytes: &[u8]) -> String {
            let rel = format!("{pak_stem}.pak");
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::LoosePak {
                    rel: rel.clone(),
                    targets: vec![],
                }],
                |dir| fs::write(dir.join(&rel), bytes).unwrap(),
            )
        }
    }

    fn loadout(entries: &[(&str, bool)]) -> Loadout {
        Loadout {
            format: 1,
            entries: entries
                .iter()
                .map(|(id, en)| LoadoutEntry {
                    id: (*id).into(),
                    enabled: *en,
                })
                .collect(),
        }
    }

    fn assert_no_apply_artifacts(game: &FakeGame, live: &Path, expected: &[u8]) {
        assert_eq!(fs::read(live).unwrap(), expected);
        assert!(
            !crate::bak_path(live).exists(),
            "failed plan construction must not create a game backup"
        );
        assert!(
            !crate::record_path(&game.root).exists(),
            "failed plan construction must not create a deploy record"
        );
        assert!(
            !game.root.join(".gore-install-mutation.lock").exists(),
            "failed plan construction must not acquire the durable install mutation lock"
        );
    }

    #[test]
    fn bounded_apply_manifest_rejects_before_game_mutation() {
        let game = FakeGame::new();
        let pristine = fs::read(game.lcache()).unwrap();
        let id = game.add_mod(
            "oversized-loc",
            "OversizedLoc",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(dir.join("loc/edits.json"), b"12345").unwrap();
            },
        );
        let error = apply_loadout_with_limits(
            &game.root,
            &game.lib,
            &loadout(&[(&id, true)]),
            ApplyLimits {
                max_manifest_bytes: 4,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("localization manifest exceeds the 4 byte limit"),
            "{error}"
        );
        assert_no_apply_artifacts(&game, &game.lcache(), &pristine);
    }

    #[test]
    fn bounded_apply_wav_and_mini_reads_reject_before_game_mutation() {
        let audio_game = FakeGame::new();
        let key = gore_fmod::GOTHIC_STUDIO_KEY;
        let bank = build_pristine_bank("shout", 44_100, &[1, 2, 3, 4], key);
        fs::write(audio_game.bank("Voice.bank"), &bank).unwrap();
        let wav = gore_fmod::wav_pcm16(44_100, 1, &[10, 20, 30, 40]);
        let audio =
            audio_game.add_audio_mod("oversized-wav", "OversizedWav", "Voice.bank", "shout", &wav);
        let error = apply_loadout_with_limits(
            &audio_game.root,
            &audio_game.lib,
            &loadout(&[(&audio, true)]),
            ApplyLimits {
                max_wav_bytes: 4,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("audio WAV payloads exceeds the 4 byte limit"),
            "{error}"
        );
        assert_no_apply_artifacts(&audio_game, &audio_game.bank("Voice.bank"), &bank);

        let script_game = FakeGame::new();
        let base = build_script_cache(&["_gore_base"]);
        fs::write(script_game.script_cache(), &base).unwrap();
        let mini = build_script_cache(&["_gore_added"]);
        let script = script_game.add_script_mod(
            "oversized-mini",
            "OversizedMini",
            "add",
            "_gore_added",
            &mini,
        );
        let error = apply_loadout_with_limits(
            &script_game.root,
            &script_game.lib,
            &loadout(&[(&script, true)]),
            ApplyLimits {
                max_mini_bytes: 4,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("script mini-cache payloads exceeds the 4 byte limit"),
            "{error}"
        );
        assert_no_apply_artifacts(&script_game, &script_game.script_cache(), &base);
    }

    #[test]
    fn rawfile_per_file_and_aggregate_limits_leave_game_pristine() {
        let per_file_game = FakeGame::new();
        let pristine = fs::read(per_file_game.lcache()).unwrap();
        let raw = per_file_game.add_rawfile_mod(
            "oversized-raw",
            "OversizedRaw",
            RawTarget::Lcache,
            b"12345",
        );
        let error = apply_loadout_with_limits(
            &per_file_game.root,
            &per_file_game.lib,
            &loadout(&[(&raw, true)]),
            ApplyLimits {
                max_raw_file_bytes: 4,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("raw-file payload exceeds the 4 byte limit"),
            "{error}"
        );
        assert_no_apply_artifacts(&per_file_game, &per_file_game.lcache(), &pristine);

        let aggregate_game = FakeGame::new();
        let pristine_loc = fs::read(aggregate_game.lcache()).unwrap();
        let pristine_script = build_script_cache(&["_gore_pristine"]);
        fs::write(aggregate_game.script_cache(), &pristine_script).unwrap();
        let loc = aggregate_game.add_rawfile_mod("raw-loc", "RawLoc", RawTarget::Lcache, b"1234");
        let raw_script = build_script_cache(&["_complete_replacement"]);
        let raw_script_len = u64::try_from(raw_script.len()).unwrap();
        let script = aggregate_game.add_rawfile_mod(
            "raw-script",
            "RawScript",
            RawTarget::ScriptCache,
            &raw_script,
        );
        let error = apply_loadout_with_limits(
            &aggregate_game.root,
            &aggregate_game.lib,
            &loadout(&[(&loc, true), (&script, true)]),
            ApplyLimits {
                max_raw_file_bytes: raw_script_len,
                max_raw_total_bytes: raw_script_len + 3,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("manager raw files total byte limit")
                || error.contains("raw-file payload exceeds"),
            "{error}"
        );
        assert_no_apply_artifacts(&aggregate_game, &aggregate_game.lcache(), &pristine_loc);
        assert_eq!(
            fs::read(aggregate_game.script_cache()).unwrap(),
            pristine_script
        );
        assert!(!crate::bak_path(&aggregate_game.script_cache()).exists());
    }

    #[test]
    fn additive_pak_limits_reject_before_copying_to_game() {
        let game = FakeGame::new();
        let pak = game.add_pak_mod("large-pak", "LargePak", "large_P", b"12345");
        let error = apply_loadout_with_limits(
            &game.root,
            &game.lib,
            &loadout(&[(&pak, true)]),
            ApplyLimits {
                max_additive_file_bytes: 4,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("loose pak exceeds the 4 byte limit"),
            "{error}"
        );
        assert!(fs::read_dir(game.mods()).unwrap().next().is_none());
        assert!(!crate::record_path(&game.root).exists());

        let aggregate_game = FakeGame::new();
        let first = aggregate_game.add_pak_mod("pak-a", "PakA", "first_P", b"1234");
        let second = aggregate_game.add_pak_mod("pak-b", "PakB", "second_P", b"5678");
        let error = apply_loadout_with_limits(
            &aggregate_game.root,
            &aggregate_game.lib,
            &loadout(&[(&first, true), (&second, true)]),
            ApplyLimits {
                max_additive_file_bytes: 4,
                max_additive_total_bytes: 7,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("manager additive payloads")
                || error.contains("loose pak exceeds the 3 byte limit"),
            "{error}"
        );
        assert!(
            fs::read_dir(aggregate_game.mods())
                .unwrap()
                .next()
                .is_none(),
            "aggregate rejection must happen before any game copy"
        );
        assert!(!crate::record_path(&aggregate_game.root).exists());
    }

    // ── tests ─────────────────────────────────────────────────────────────────────────────────

    /// Two enabled mods edit the SAME loc id; the later one (loadout order) wins, and the pristine
    /// value of the other id is preserved.
    #[test]
    fn apply_two_mods_loc_last_wins() {
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        let b = g.add_loc_mod("mod-b", "Bravo", "itfo_cheese", "Brie");
        let lo = loadout(&[(&a, true), (&b, true)]);

        let report = apply_loadout(&g.root, &g.lib, &lo).unwrap();
        assert_eq!(
            report.applied,
            vec!["Alpha".to_string(), "Bravo".to_string()]
        );
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );

        let live = fs::read(g.lcache()).unwrap();
        assert_eq!(read_loc(&live, "itfo_cheese"), "Brie", "later mod must win");
        assert_eq!(
            read_loc(&live, "itfo_apple"),
            "Apple",
            "untouched id preserved"
        );
    }

    #[test]
    fn apply_voice_archives_later_wins_from_pristine_and_failure_keeps_active_loadout() {
        let g = FakeGame::new();
        let live = g.voice_archive("German.zip");
        let original = crate::tests::test_ogg(16_000);
        crate::tests::write_test_voice_zip(
            &live,
            &[
                ("NPC/Hero/hello.ogg", &original),
                ("metadata.txt", b"untouched"),
            ],
        );
        let pristine_zip = fs::read(&live).unwrap();
        let first = crate::tests::test_ogg(32_000);
        let second = crate::tests::test_ogg(48_000);
        let a = g.add_voice_mod(
            "mod-voice-a",
            "Voice Alpha",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &first,
        );
        let b = g.add_voice_mod(
            "mod-voice-b",
            "Voice Bravo",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &second,
        );

        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_eq!(
            crate::tests::read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            first
        );
        assert_eq!(fs::read(crate::bak_path(&live)).unwrap(), pristine_zip);

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, true)])).unwrap();
        assert!(report.warnings.is_empty());
        assert_eq!(
            crate::tests::read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            second
        );
        assert_eq!(
            crate::tests::read_test_zip_entry(&live, "metadata.txt").unwrap(),
            b"untouched"
        );
        assert_eq!(fs::read(crate::bak_path(&live)).unwrap(), pristine_zip);

        // A failure after the full plan was prepared but during the live commit must restore the
        // exact active manager deployment, not merely leave the game pristine. Inject one target-
        // scoped atomic-write failure to force that late branch deterministically.
        let c = g.add_voice_mod(
            "mod-voice-c",
            "Voice Charlie",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &crate::tests::test_ogg(22_050),
        );
        crate::fail_next_atomic_write(&live);
        let before_late_failure_live = fs::read(&live).unwrap();
        let record_path = g.root.join("gore-mod.deployed.json");
        let before_late_failure_record = fs::read(&record_path).unwrap();
        let late_error = apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&a, true), (&b, true), (&c, true)]),
        )
        .unwrap_err()
        .to_string();
        assert!(
            late_error.contains("atomic-write failure"),
            "unexpected late commit error: {late_error}"
        );
        assert_eq!(fs::read(&live).unwrap(), before_late_failure_live);
        assert_eq!(
            fs::read(&record_path).unwrap(),
            before_late_failure_record,
            "late commit failure must restore the active manager record"
        );

        // Missing targets are conservative hard failures, and all archive/payload/Ogg validation
        // happens before the active manager deployment is undeployed.
        let missing = g.add_voice_mod(
            "mod-voice-missing",
            "Missing Voice",
            "Missing.zip",
            crate::VoicePatchOp::Add,
            "GORE/new.ogg",
            &crate::tests::test_ogg(44_100),
        );
        let before_live = fs::read(&live).unwrap();
        let before_record = fs::read(&record_path).unwrap();
        let error = apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&a, true), (&b, true), (&missing, true)]),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("partial voice patch"),
            "unexpected error: {error}"
        );
        assert_eq!(fs::read(&live).unwrap(), before_live);
        assert_eq!(fs::read(&record_path).unwrap(), before_record);

        // A library payload can be corrupted after import. Apply revalidates it before undeploy,
        // so the currently working voice archive and record still remain untouched.
        fs::write(g.lib.join(&b).join("voice/payload/0.ogg"), b"corrupt").unwrap();
        let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, true)]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("voice archive"), "unexpected error: {error}");
        assert_eq!(fs::read(&live).unwrap(), before_live);
        assert_eq!(fs::read(&record_path).unwrap(), before_record);

        undeploy_all(&g.root).unwrap();
        assert_eq!(fs::read(&live).unwrap(), pristine_zip);
    }

    /// Exercise a genuinely partial multi-target commit: localization is written first, then the
    /// voice archive's atomic-write temp path is blocked. The late error must roll localization,
    /// voice, both pristine backups, and the manager record back to the exact active loadout.
    #[test]
    fn late_second_target_failure_rolls_back_exact_active_loadout() {
        let g = FakeGame::new();
        let loc = g.lcache();
        let voice = g.voice_archive("German.zip");
        let original_voice = crate::tests::test_ogg(16_000);
        crate::tests::write_test_voice_zip(&voice, &[("NPC/Hero/hello.ogg", &original_voice)]);

        let loc_a = g.add_loc_mod("loc-a", "Loc Alpha", "itfo_cheese", "Gouda");
        let voice_a_bytes = crate::tests::test_ogg(32_000);
        let voice_a = g.add_voice_mod(
            "voice-a",
            "Voice Alpha",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &voice_a_bytes,
        );
        apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&loc_a, true), (&voice_a, true)]),
        )
        .unwrap();

        let record = g.root.join("gore-mod.deployed.json");
        let before_loc = fs::read(&loc).unwrap();
        let before_voice = fs::read(&voice).unwrap();
        let before_loc_bak = fs::read(crate::bak_path(&loc)).unwrap();
        let before_voice_bak = fs::read(crate::bak_path(&voice)).unwrap();
        let before_record = fs::read(&record).unwrap();

        let loc_b = g.add_loc_mod("loc-b", "Loc Bravo", "itfo_cheese", "Brie");
        let voice_b_bytes = crate::tests::test_ogg(48_000);
        let voice_b = g.add_voice_mod(
            "voice-b",
            "Voice Bravo",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &voice_b_bytes,
        );

        // `plan.writes` contains BTreeMap-composed localization writes first; voice rewrites are
        // appended afterwards. Injecting failure only for the voice write therefore fails after the
        // first live target was replaced, not during preparation or staging.
        crate::fail_next_atomic_write(&voice);
        let error = apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&loc_b, true), (&voice_b, true)]),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("atomic-write failure"),
            "unexpected error: {error}"
        );

        assert_eq!(fs::read(&loc).unwrap(), before_loc);
        assert_eq!(fs::read(&voice).unwrap(), before_voice);
        assert_eq!(fs::read(crate::bak_path(&loc)).unwrap(), before_loc_bak);
        assert_eq!(fs::read(crate::bak_path(&voice)).unwrap(), before_voice_bak);
        assert_eq!(fs::read(&record).unwrap(), before_record);

        // Prove the rejected plan really does change both targets when its late blocker is gone.
        apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&loc_b, true), (&voice_b, true)]),
        )
        .unwrap();
        assert_eq!(read_loc(&fs::read(&loc).unwrap(), "itfo_cheese"), "Brie");
        assert_eq!(
            crate::tests::read_test_zip_entry(&voice, "NPC/Hero/hello.ogg").unwrap(),
            voice_b_bytes
        );
    }

    /// If Steam/hotfix bytes drift while a manager loadout is active, a failed re-apply must put
    /// both the externally-updated live file and the prior record's old backup back exactly. That
    /// keeps the old record self-consistent, and a later undeploy must preserve the hotfix instead
    /// of restoring its stale pre-hotfix backup over it.
    #[test]
    fn drift_refresh_late_failure_restores_old_backup_and_preserves_hotfix() {
        let g = FakeGame::new();
        let loc = g.lcache();
        let voice = g.voice_archive("German.zip");
        let original_voice = crate::tests::test_ogg(16_000);
        crate::tests::write_test_voice_zip(&voice, &[("NPC/Hero/hello.ogg", &original_voice)]);
        let pristine_voice = fs::read(&voice).unwrap();

        let loc_a = g.add_loc_mod("loc-a", "Loc Alpha", "itfo_cheese", "Gouda");
        let voice_a_bytes = crate::tests::test_ogg(32_000);
        let voice_a = g.add_voice_mod(
            "voice-a",
            "Voice Alpha",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &voice_a_bytes,
        );
        apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&loc_a, true), (&voice_a, true)]),
        )
        .unwrap();

        let record = g.root.join("gore-mod.deployed.json");
        let stale_loc_bak = fs::read(crate::bak_path(&loc)).unwrap();
        let before_voice = fs::read(&voice).unwrap();
        let before_voice_bak = fs::read(crate::bak_path(&voice)).unwrap();
        let before_record = fs::read(&record).unwrap();

        // Simulate a game hotfix replacing this live cache underneath the active deployment.
        let hotfix_loc = build_lcache(&[
            ("itfo_cheese", "Hotfix Cheese"),
            ("itfo_apple", "Hotfix Apple"),
        ]);
        fs::write(&loc, &hotfix_loc).unwrap();

        let loc_b = g.add_loc_mod("loc-b", "Loc Bravo", "itfo_cheese", "Brie");
        let voice_b = g.add_voice_mod(
            "voice-b",
            "Voice Bravo",
            "German.zip",
            crate::VoicePatchOp::Replace,
            "NPC/Hero/hello.ogg",
            &crate::tests::test_ogg(48_000),
        );
        crate::fail_next_atomic_write(&voice);
        let error = apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&loc_b, true), (&voice_b, true)]),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("atomic-write failure"),
            "unexpected error: {error}"
        );

        assert_eq!(fs::read(&loc).unwrap(), hotfix_loc);
        assert_eq!(fs::read(crate::bak_path(&loc)).unwrap(), stale_loc_bak);
        assert_eq!(fs::read(&voice).unwrap(), before_voice);
        assert_eq!(fs::read(crate::bak_path(&voice)).unwrap(), before_voice_bak);
        assert_eq!(fs::read(&record).unwrap(), before_record);

        undeploy_all(&g.root).unwrap();
        assert_eq!(
            fs::read(&loc).unwrap(),
            hotfix_loc,
            "undeploy must not restore a stale pre-hotfix localization backup"
        );
        assert_eq!(fs::read(&voice).unwrap(), pristine_voice);
    }

    /// Loc patches may introduce a brand-new id. All declared languages are added together;
    /// unsupported languages remain best-effort warnings, and an existing id with such a
    /// language remains untouched.
    #[test]
    fn apply_warns_when_the_edited_generation_is_shadowed() {
        // `gore mod deploy` reports this; the manager applies the same bundles by another route and
        // did not, so a managed apply wrote an edit the game never shows and said nothing about it.
        let g = FakeGame::new();
        fs::write(
            g.lcache(),
            build_lcache_with_langs(
                &["german", "german_new"],
                "itfo_cheese",
                &[("german", "Käse"), ("german_new", "Bergkäse")],
            ),
        )
        .unwrap();

        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("german".into(), "Emmentaler".into());
        let id = g.add_mod(
            "mod-shadowed-loc",
            "Shadowed Loc",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(
                    dir.join("loc/edits.json"),
                    serde_json::to_vec(&edits).unwrap(),
                )
                .unwrap();
            },
        );

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("itfo_cheese") && w.contains("german_new")),
            "warnings: {:?}",
            report.warnings
        );

        // And the edit still lands: it was a legitimate write, just not a visible one.
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Emmentaler"
        );
    }

    #[test]
    fn apply_loc_patch_adds_missing_id_and_skips_unsupported_languages() {
        let g = FakeGame::new();
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("goremod_new_dialog".into())
            .or_default()
            .extend([
                ("german".into(), "Neue Zeile".into()),
                ("english".into(), "New line".into()),
            ]);
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("english".into(), "Cheese".into());
        let id = g.add_mod(
            "mod-new-loc",
            "New Loc",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(
                    dir.join("loc/edits.json"),
                    serde_json::to_vec(&edits).unwrap(),
                )
                .unwrap();
            },
        );

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap();
        assert_eq!(report.warnings.len(), 2, "warnings: {:?}", report.warnings);
        assert!(report
            .warnings
            .iter()
            .all(|warning| warning.contains("english")));

        let live = fs::read(g.lcache()).unwrap();
        let decoded = gore_loc::loc::Lcache::decode(&live).unwrap();
        let exported = decoded.export(false);
        assert_eq!(exported["goremod_new_dialog"]["german"], "Neue Zeile");
        assert!(!exported["goremod_new_dialog"].contains_key("english"));
        assert_eq!(exported["itfo_cheese"]["german"], "Cheese");
    }

    #[test]
    fn apply_loc_id_and_language_aliases_preserve_loadout_later_wins() {
        let g = FakeGame::new();
        let make_edits = |id: &str, language: &str, text: &str| {
            let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            edits
                .entry(id.into())
                .or_default()
                .insert(language.into(), text.into());
            edits
        };
        let first_edits = make_edits("goremod_case_id", "german", "Erste Zeile");
        let second_edits = make_edits("GOREMOD_CASE_ID", "German", "Zweite Zeile");
        let a = g.add_mod(
            "mod-case-a",
            "Case A",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(
                    dir.join("loc/edits.json"),
                    serde_json::to_vec(&first_edits).unwrap(),
                )
                .unwrap();
            },
        );
        let b = g.add_mod(
            "mod-case-b",
            "Case B",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(
                    dir.join("loc/edits.json"),
                    serde_json::to_vec(&second_edits).unwrap(),
                )
                .unwrap();
            },
        );

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, true)])).unwrap();
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );
        let exported = gore_loc::loc::Lcache::decode(&fs::read(g.lcache()).unwrap())
            .unwrap()
            .export(false);
        let matches: Vec<_> = exported
            .iter()
            .filter(|(id, _)| id.eq_ignore_ascii_case("goremod_case_id"))
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "case aliases must produce one lcache group"
        );
        assert_eq!(matches[0].1["german"], "Zweite Zeile");
    }

    #[test]
    fn apply_new_loc_collapses_language_case_aliases() {
        let g = FakeGame::new();
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("goremod_language_alias".into())
            .or_default()
            .extend([
                ("German".into(), "Erste Zeile".into()),
                ("german".into(), "Zweite Zeile".into()),
            ]);
        let id = g.add_mod(
            "mod-language-alias",
            "Language Alias",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(
                    dir.join("loc/edits.json"),
                    serde_json::to_vec(&edits).unwrap(),
                )
                .unwrap();
            },
        );

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap();
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );
        let exported = gore_loc::loc::Lcache::decode(&fs::read(g.lcache()).unwrap())
            .unwrap()
            .export(false);
        assert_eq!(exported["goremod_language_alias"]["german"], "Zweite Zeile");
        assert_eq!(exported["goremod_language_alias"].len(), 1);
    }

    #[test]
    fn manager_container_names_encode_strict_numeric_patch_priority() {
        assert_eq!(
            slot_stem("zzz_Alpha Menu_P", 0).unwrap(),
            "zzz_gm000_Alpha_Menu_1_P"
        );
        assert_eq!(
            slot_pak_stem("paks/Bravo_P", 1).unwrap(),
            "zzz_gm001_Bravo_2_P"
        );
        assert_eq!(slot_stem("last_P", 999).unwrap(), "zzz_gm999_last_1000_P");
        assert!(manager_patch_priority(usize::MAX).is_err());

        let sources = [
            PathBuf::from("temp/generated_tex_P.utoc"),
            PathBuf::from("temp/generated_tex_P.ucas"),
            PathBuf::from("temp/generated_tex_P.pak"),
            PathBuf::from("temp/generated_files_P.pak"),
        ];
        let destinations = [
            PathBuf::from("mods/zzz_gm001_mod_hash_0_tex_P.utoc"),
            PathBuf::from("mods/zzz_gm001_mod_hash_0_tex_P.ucas"),
            PathBuf::from("mods/zzz_gm001_mod_hash_0_tex_P.pak"),
            PathBuf::from("mods/zzz_gm001_mod_hash_1_files_P.pak"),
        ];
        let generated = prioritize_generated_containers(
            sources
                .iter()
                .cloned()
                .zip(destinations.iter().cloned())
                .collect(),
            1,
        )
        .unwrap();
        assert_eq!(
            generated.iter().map(|(src, _)| src).collect::<Vec<_>>(),
            sources.iter().collect::<Vec<_>>(),
            "manager priority changes destinations, never retained temporary sources"
        );
        assert_eq!(
            generated
                .iter()
                .map(|(_, dst)| dst.file_name().unwrap().to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            [
                "zzz_gm001_mod_hash_0_tex_2_P.utoc",
                "zzz_gm001_mod_hash_0_tex_2_P.ucas",
                "zzz_gm001_mod_hash_0_tex_2_P.pak",
                "zzz_gm001_mod_hash_1_files_2_P.pak",
            ]
        );
    }

    /// Two loose-pak mods receive strict Unreal numeric patch versions. Reversing the loadout
    /// swaps both versions and payloads, removes every old manager target, and leaves unrelated
    /// user files untouched.
    #[test]
    fn apply_orders_paks_by_slot() {
        let g = FakeGame::new();
        let a = g.add_pak_mod("mod-a", "Alpha", "alpha_P", b"PAK-A");
        let b = g.add_pak_mod("mod-b", "Bravo", "bravo_P", b"PAK-B");
        let unrelated = g.mods().join("user-owned.pak");
        fs::write(&unrelated, b"KEEP").unwrap();
        let lo = loadout(&[(&a, true), (&b, true)]);

        apply_loadout(&g.root, &g.lib, &lo).unwrap();

        let a_dst = g.mods().join("zzz_gm000_alpha_1_P.pak");
        let b_dst = g.mods().join("zzz_gm001_bravo_2_P.pak");
        assert!(a_dst.is_file(), "slot-0 pak missing: {}", a_dst.display());
        assert!(b_dst.is_file(), "slot-1 pak missing: {}", b_dst.display());
        assert_eq!(fs::read(&a_dst).unwrap(), b"PAK-A");
        assert_eq!(fs::read(&b_dst).unwrap(), b"PAK-B");

        apply_loadout(&g.root, &g.lib, &loadout(&[(&b, true), (&a, true)])).unwrap();
        let b_reordered = g.mods().join("zzz_gm000_bravo_1_P.pak");
        let a_reordered = g.mods().join("zzz_gm001_alpha_2_P.pak");
        assert_eq!(fs::read(&b_reordered).unwrap(), b"PAK-B");
        assert_eq!(fs::read(&a_reordered).unwrap(), b"PAK-A");
        assert!(!a_dst.exists(), "old Alpha priority survived reapply");
        assert!(!b_dst.exists(), "old Bravo priority survived reapply");
        assert_eq!(fs::read(&unrelated).unwrap(), b"KEEP");
    }

    #[test]
    fn apply_triplet_snapshots_required_members_and_skips_missing_optional_pak() {
        let g = FakeGame::new();
        let triplet = g.add_mod(
            "triplet-a",
            "TripletA",
            vec![ComponentInfo::Triplet {
                rel_base: "paks/container_P".into(),
                targets: Vec::new(),
            }],
            |dir| {
                fs::create_dir_all(dir.join("paks")).unwrap();
                fs::write(dir.join("paks/container_P.utoc"), b"UTOC").unwrap();
                fs::write(dir.join("paks/container_P.ucas"), b"UCAS").unwrap();
            },
        );

        apply_loadout(&g.root, &g.lib, &loadout(&[(&triplet, true)])).unwrap();

        let stem = "zzz_gm000_container_1_P";
        assert_eq!(
            fs::read(g.mods().join(format!("{stem}.utoc"))).unwrap(),
            b"UTOC"
        );
        assert_eq!(
            fs::read(g.mods().join(format!("{stem}.ucas"))).unwrap(),
            b"UCAS"
        );
        assert!(!g.mods().join(format!("{stem}.pak")).exists());
    }

    #[test]
    fn apply_triplet_reorder_swaps_numeric_priority_for_every_sidecar() {
        let g = FakeGame::new();
        let add_triplet = |id: &str, label: &str| {
            let rel_base = format!("paks/zzz_{label}_P");
            g.add_mod(
                id,
                label,
                vec![ComponentInfo::Triplet {
                    rel_base: rel_base.clone(),
                    targets: Vec::new(),
                }],
                |dir| {
                    fs::create_dir_all(dir.join("paks")).unwrap();
                    for extension in ["utoc", "ucas", "pak"] {
                        fs::write(
                            dir.join(format!("{rel_base}.{extension}")),
                            format!("{label}-{extension}"),
                        )
                        .unwrap();
                    }
                },
            )
        };
        let alpha = add_triplet("triplet-alpha", "alpha");
        let bravo = add_triplet("triplet-bravo", "bravo");
        let unrelated = g.mods().join("user-owned.pak");
        fs::write(&unrelated, b"KEEP").unwrap();
        let assert_triplet = |stem: &str, label: &str| {
            for extension in ["utoc", "ucas", "pak"] {
                assert_eq!(
                    fs::read(g.mods().join(format!("{stem}.{extension}"))).unwrap(),
                    format!("{label}-{extension}").as_bytes(),
                    "{label} {extension} did not follow its loadout priority"
                );
            }
        };
        let paths_for = |stem: &str| {
            ["utoc", "ucas", "pak"].map(|extension| g.mods().join(format!("{stem}.{extension}")))
        };

        apply_loadout(&g.root, &g.lib, &loadout(&[(&alpha, true), (&bravo, true)])).unwrap();

        let alpha_first = "zzz_gm000_alpha_1_P";
        let bravo_second = "zzz_gm001_bravo_2_P";
        assert_triplet(alpha_first, "alpha");
        assert_triplet(bravo_second, "bravo");
        let old_paths = [paths_for(alpha_first), paths_for(bravo_second)].concat();

        apply_loadout(&g.root, &g.lib, &loadout(&[(&bravo, true), (&alpha, true)])).unwrap();

        assert_triplet("zzz_gm000_bravo_1_P", "bravo");
        assert_triplet("zzz_gm001_alpha_2_P", "alpha");
        for old in old_paths {
            assert!(
                !old.exists(),
                "old triplet sidecar survived: {}",
                old.display()
            );
        }
        assert_eq!(fs::read(&unrelated).unwrap(), b"KEEP");
    }

    #[test]
    fn reapply_migrates_recorded_legacy_manager_name_and_reset_cleans_new_name() {
        let g = FakeGame::new();
        let id = g.add_pak_mod("mod-a", "Alpha", "alpha_P", b"PAK-A");
        let meta: ModEntryMeta = serde_json::from_slice(
            &fs::read(g.lib.join(&id).join(META_FILE)).unwrap(),
        )
        .unwrap();
        let legacy = g.mods().join("zzz_gm000_alpha_P.pak");
        fs::write(&legacy, b"OLD-MANAGER-PAK").unwrap();
        let unrelated = g.mods().join("user-owned.pak");
        fs::write(&unrelated, b"KEEP").unwrap();
        let legacy_key = fs::canonicalize(&legacy).unwrap().display().to_string();
        let legacy_record = DeployRecord {
            owner: "manager".into(),
            loadout: loadout(&[(&id, true)]).entries,
            managed_paks: vec![legacy_key.clone()],
            // Manager records mirror additive files for older readers.
            texture_triplets: vec![legacy_key.clone()],
            deployed_hashes: BTreeMap::from([(
                legacy_key.clone(),
                crate::sha256_file(&legacy).unwrap(),
            )]),
            deployed_fingerprints: BTreeMap::from([(id.clone(), meta.fingerprint())]),
            ..Default::default()
        };
        fs::write(
            crate::record_path(&g.root),
            serde_json::to_vec(&legacy_record).unwrap(),
        )
        .unwrap();

        assert_eq!(
            crate::mgr::status::status(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap(),
            crate::mgr::status::ManagerStatus::ChangesPending {
                deployed: loadout(&[(&id, true)]).entries,
                target: loadout(&[(&id, true)]).entries,
            },
            "matching legacy loadout and fingerprints still need the naming migration"
        );

        apply_loadout(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap();

        let current = g.mods().join("zzz_gm000_alpha_1_P.pak");
        assert_eq!(fs::read(&current).unwrap(), b"PAK-A");
        assert!(!legacy.exists(), "recorded legacy name survived reapply");
        let record = crate::read_record(&g.root).unwrap().unwrap().record;
        assert!(record
            .managed_paks
            .iter()
            .any(|path| crate::same_path(&current, path)));
        assert!(!record
            .managed_paks
            .iter()
            .any(|path| crate::same_path(&legacy, path)));
        assert_eq!(
            record.manager_container_priority_schema,
            Some(crate::MANAGER_CONTAINER_PRIORITY_SCHEMA)
        );
        assert_eq!(
            crate::mgr::status::status(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap(),
            crate::mgr::status::ManagerStatus::InSync {
                loadout: loadout(&[(&id, true)]).entries,
            }
        );
        assert_eq!(fs::read(&unrelated).unwrap(), b"KEEP");

        assert!(undeploy_all(&g.root).unwrap());
        assert!(!current.exists(), "Reset left the prioritized manager pak");
        assert!(!legacy.exists(), "Reset recreated the legacy manager pak");
        assert_eq!(fs::read(&unrelated).unwrap(), b"KEEP");
        assert!(!crate::record_path(&g.root).exists());
    }

    /// A self-colliding new loadout (two components mapping to the SAME deploy dst) must be rejected
    /// BEFORE the active deployment is torn down, so a failed apply stays non-destructive.
    #[test]
    fn self_colliding_apply_rejected_without_undeploying_active() {
        let g = FakeGame::new();
        // A clean deployment we expect to survive the later failed apply.
        let a = g.add_pak_mod("mod-a", "Alpha", "alpha_P", b"PAK-A");
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        let a_dst = g.mods().join("zzz_gm000_alpha_1_P.pak");
        assert!(a_dst.is_file(), "precondition: slot-0 pak deployed");

        // A mod with two loose paks whose file stems collide → both map to
        // zzz_gm000_dup_1_P.pak.
        let c = g.add_mod(
            "mod-c",
            "Clash",
            vec![
                ComponentInfo::LoosePak {
                    rel: "x/dup.pak".into(),
                    targets: vec![],
                },
                ComponentInfo::LoosePak {
                    rel: "y/dup.pak".into(),
                    targets: vec![],
                },
            ],
            |dir| {
                fs::create_dir_all(dir.join("x")).unwrap();
                fs::create_dir_all(dir.join("y")).unwrap();
                fs::write(dir.join("x/dup.pak"), b"DUP-X").unwrap();
                fs::write(dir.join("y/dup.pak"), b"DUP-Y").unwrap();
            },
        );

        let err = apply_loadout(&g.root, &g.lib, &loadout(&[(&c, true)])).unwrap_err();
        assert!(
            err.to_string().contains("duplicate deploy target"),
            "expected a duplicate-target rejection, got: {err}"
        );
        // The previous deployment must be intact — the rejected apply must not have undeployed it.
        assert!(
            a_dst.is_file(),
            "active deployment was torn down by a rejected apply: {}",
            a_dst.display()
        );
    }

    /// The deploy record is manager-owned, snapshots the enabled loadout (ids, all enabled), and
    /// records the installed managed paks.
    #[test]
    fn apply_writes_manager_record_with_loadout() {
        let g = FakeGame::new();
        let a = g.add_pak_mod("mod-a", "Alpha", "alpha_P", b"PAK-A");
        let b = g.add_loc_mod("mod-b", "Bravo", "itfo_apple", "Pomme");
        // mod-c disabled → excluded from the record snapshot.
        let lo = loadout(&[(&a, true), (&b, true), ("mod-c", false)]);

        apply_loadout(&g.root, &g.lib, &lo).unwrap();

        let rec = crate::read_record(&g.root)
            .unwrap()
            .expect("record written")
            .record;
        assert_eq!(rec.owner, "manager");
        assert_eq!(rec.phase, crate::DeployPhase::Applied);
        assert_eq!(
            rec.loadout,
            vec![
                LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: true
                },
                LoadoutEntry {
                    id: "mod-b".into(),
                    enabled: true
                },
            ],
            "record loadout = enabled snapshot in order"
        );
        assert_eq!(rec.managed_paks.len(), 1, "one managed pak recorded");
        assert!(rec.managed_paks[0].ends_with("zzz_gm000_alpha_1_P.pak"));
    }

    #[test]
    fn apply_ignores_malformed_future_manager_private_metadata() {
        let g = FakeGame::new();
        let id = g.add_pak_mod("mod-a", "Alpha", "alpha_P", b"PAK-A");
        let sidecar = g.lib.join(&id).join(META_FILE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
        value["_manager"] = serde_json::json!(["future", 2]);
        fs::write(&sidecar, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

        apply_loadout(&g.root, &g.lib, &loadout(&[(&id, true)])).unwrap();
        assert!(g.mods().join("zzz_gm000_alpha_1_P.pak").is_file());
    }

    /// A re-apply that fails while BUILDING the plan (a mod with an undecodable payload) must not
    /// touch the previously-applied manager deployment — the undeploy is deferred until the whole
    /// plan is built, so the prior record + deployed content survive the failure.
    #[test]
    fn failed_apply_preserves_prior_deployment() {
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        // First apply succeeds: a manager deployment of [a] is live.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda"
        );

        // A mod whose loc payload is corrupt JSON: PASS 1 fails to parse it → apply errors before
        // the deferred undeploy.
        let bad = g.add_mod(
            "bad",
            "Bad",
            vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec![],
            }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(dir.join("loc/edits.json"), b"{ not valid json").unwrap();
            },
        );

        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&bad, true)]))
            .expect_err("a corrupt mod payload must fail the apply");

        // Prior deployment intact: record still manager-owned with loadout [a], and the live
        // .lcache still carries mod-a's edit (NOT reverted to pristine by an early undeploy).
        let rec = crate::read_record(&g.root)
            .unwrap()
            .expect("prior record must survive a failed apply")
            .record;
        assert_eq!(rec.owner, "manager");
        assert_eq!(
            rec.loadout,
            vec![LoadoutEntry {
                id: "mod-a".into(),
                enabled: true
            }]
        );
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda",
            "prior deployment content must remain after a failed re-apply"
        );
    }

    #[test]
    fn unsafe_loadout_ids_fail_before_active_deployment_mutation() {
        let g = FakeGame::new();
        let active = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        apply_loadout(&g.root, &g.lib, &loadout(&[(&active, true)])).unwrap();
        let record_before = fs::read(crate::record_path(&g.root)).unwrap();
        let live_before = fs::read(g.lcache()).unwrap();

        // Make both paths point at a readable, valid-looking sidecar. Before containment checks,
        // apply would read this metadata outside the manager library and replace the live deploy.
        let outside = g.lib.parent().unwrap().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let outside_meta = ModEntryMeta {
            id: "outside".into(),
            kind: ModKind::Goremod,
            name: "Outside".into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-07-03T00:00:00Z".into(),
            source: String::new(),
            components: Vec::new(),
        };
        fs::write(
            outside.join(super::super::model::META_FILE),
            serde_json::to_vec(&outside_meta).unwrap(),
        )
        .unwrap();
        let absolute = outside.display().to_string();

        for (id, enabled) in [
            ("../outside", true),
            (absolute.as_str(), true),
            // Disabled malicious slots matter too: otherwise the empty-enabled branch undeploys.
            ("../outside", false),
        ] {
            let error = apply_loadout(&g.root, &g.lib, &loadout(&[(id, enabled)]))
                .unwrap_err()
                .to_string();
            assert!(error.contains("invalid loadout entry"), "{id:?}: {error}");
            assert_eq!(
                fs::read(crate::record_path(&g.root)).unwrap(),
                record_before
            );
            assert_eq!(fs::read(g.lcache()).unwrap(), live_before);
        }
    }

    #[test]
    fn mismatched_sidecar_id_fails_before_active_deployment_mutation() {
        let g = FakeGame::new();
        let active = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        apply_loadout(&g.root, &g.lib, &loadout(&[(&active, true)])).unwrap();
        let record_before = fs::read(crate::record_path(&g.root)).unwrap();
        let live_before = fs::read(g.lcache()).unwrap();

        let bad = g.add_mod("bad", "Bad", Vec::new(), |_| {});
        let sidecar = g.lib.join(&bad).join(super::super::model::META_FILE);
        let mut meta: ModEntryMeta = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
        meta.id = "some-other-entry".into();
        fs::write(&sidecar, serde_json::to_vec(&meta).unwrap()).unwrap();

        let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&bad, true)]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("sidecar id mismatch"), "{error}");
        assert_eq!(
            fs::read(crate::record_path(&g.root)).unwrap(),
            record_before
        );
        assert_eq!(fs::read(g.lcache()).unwrap(), live_before);
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn linked_entry_and_payload_are_rejected_before_active_mutation() {
        let g = FakeGame::new();
        let active = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        apply_loadout(&g.root, &g.lib, &loadout(&[(&active, true)])).unwrap();
        let record_before = fs::read(crate::record_path(&g.root)).unwrap();
        let live_before = fs::read(g.lcache()).unwrap();

        let outside_entry = g.lib.parent().unwrap().join("outside-entry");
        fs::create_dir_all(&outside_entry).unwrap();
        let linked_meta = ModEntryMeta {
            id: "linked-entry".into(),
            kind: ModKind::ForeignPak,
            name: "Linked entry".into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-07-03T00:00:00Z".into(),
            source: String::new(),
            components: vec![ComponentInfo::LoosePak {
                rel: "outside_P.pak".into(),
                targets: Vec::new(),
            }],
        };
        fs::write(
            outside_entry.join(super::super::model::META_FILE),
            serde_json::to_vec(&linked_meta).unwrap(),
        )
        .unwrap();
        fs::write(outside_entry.join("outside_P.pak"), b"outside").unwrap();
        assert!(
            make_dir_link(&outside_entry, &g.lib.join("linked-entry")),
            "test requires symbolic-link creation support"
        );
        let error = apply_loadout(&g.root, &g.lib, &loadout(&[("linked-entry", true)]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("symbolic link") || error.contains("reparse point"),
            "{error}"
        );
        assert_eq!(
            fs::read(crate::record_path(&g.root)).unwrap(),
            record_before
        );
        assert_eq!(fs::read(g.lcache()).unwrap(), live_before);

        let outside_payload = g.lib.parent().unwrap().join("outside-payload.pak");
        fs::write(&outside_payload, b"outside payload").unwrap();
        let linked_payload = g.add_mod(
            "linked-payload",
            "Linked payload",
            vec![ComponentInfo::LoosePak {
                rel: "payload_P.pak".into(),
                targets: Vec::new(),
            }],
            |_| {},
        );
        assert!(
            make_file_link(
                &outside_payload,
                &g.lib.join(&linked_payload).join("payload_P.pak"),
            ),
            "test requires symbolic-link creation support"
        );
        let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&linked_payload, true)]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("symbolic link") || error.contains("reparse point"),
            "{error}"
        );
        assert_eq!(
            fs::read(crate::record_path(&g.root)).unwrap(),
            record_before
        );
        assert_eq!(fs::read(g.lcache()).unwrap(), live_before);
    }

    /// A missing ADDITIVE source (a loose pak whose file is gone) is caught during plan-building —
    /// before the deferred undeploy — so the prior deployment is not torn down by a copy that would
    /// have failed inside commit_plan.
    #[test]
    fn missing_additive_source_fails_before_undeploy() {
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();

        // A mod whose LoosePak component points at a pak that was never written to disk.
        let bad = g.add_mod(
            "bad",
            "Bad",
            vec![ComponentInfo::LoosePak {
                rel: "ghost_P.pak".into(),
                targets: vec![],
            }],
            |_dir| {}, // deliberately do NOT create the pak
        );

        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&bad, true)]))
            .expect_err("a missing additive source must fail the apply");

        let rec = crate::read_record(&g.root)
            .unwrap()
            .expect("prior record survives")
            .record;
        assert_eq!(
            rec.loadout,
            vec![LoadoutEntry {
                id: "mod-a".into(),
                enabled: true
            }]
        );
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda"
        );
    }

    /// A rawfile whose live target isn't present in this install (an incompatible bank) is skipped
    /// with a warning — NOT a hard error — and the rest of the apply still lands.
    #[test]
    fn rawfile_missing_target_is_skipped_with_warning() {
        let g = FakeGame::new();
        let base = g.add_loc_mod("mod-base", "Base", "itfo_cheese", "Gouda");
        let raw = g.add_rawfile_mod(
            "raw",
            "Raw",
            RawTarget::Bank {
                name: "Ghost.bank".into(),
            },
            b"whatever",
        );

        let report =
            apply_loadout(&g.root, &g.lib, &loadout(&[(&base, true), (&raw, true)])).unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("not present in this install")),
            "expected a skip warning, got: {:?}",
            report.warnings
        );
        // The absent bank was not created, and the base loc edit still applied.
        assert!(!g.bank("Ghost.bank").exists());
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda"
        );
    }

    /// Two enabled mods replace the SAME loose file. The later one wins — and the accumulator that
    /// makes that true also has to dedupe BEFORE the plan is built: `first_duplicate_dst` rejects a
    /// plan with two writes to one path, so without it an ordinary two-mod overlap would become a
    /// hard apply failure instead of a later-wins merge.
    #[test]
    fn two_mods_replacing_one_loose_file_apply_as_later_wins_not_as_a_collision() {
        let g = FakeGame::new();
        let cursor = "G1R/Content/Slate/Cursors/Normal/Normal.PNG";
        let live = g.loose_file(cursor);
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        fs::write(&live, b"shipped").unwrap();
        let a = g.add_loose_file_mod("mod-a", "Alpha", cursor, b"alpha-cursor");
        let b = g.add_loose_file_mod("mod-b", "Bravo", cursor, b"bravo-cursor");

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, true)])).unwrap();
        assert_eq!(
            report.applied,
            vec!["Alpha".to_string(), "Bravo".to_string()],
            "both mods must apply; only the file is contested"
        );
        assert_eq!(fs::read(&live).unwrap(), b"bravo-cursor");
        assert_eq!(
            fs::read(crate::bak_path(&live)).unwrap(),
            b"shipped",
            "the pristine file must be preserved exactly once"
        );

        assert!(undeploy_all(&g.root).unwrap());
        assert_eq!(fs::read(&live).unwrap(), b"shipped");
        assert!(!crate::bak_path(&live).exists());
    }

    /// A loose-file replacement whose target this install does not have is skipped with a warning —
    /// NOT a hard error — and, like the rawfile skip, the decision is taken before the deferred
    /// undeploy so an incompatible mod cannot tear down the working deployment and then fail while
    /// backing up a file that was never there.
    #[test]
    fn loose_file_missing_target_is_skipped_with_warning() {
        let g = FakeGame::new();
        let ghost = "G1R/Content/Slate/Cursors/Normal/Ghost.PNG";
        let base = g.add_loc_mod("mod-base", "Base", "itfo_cheese", "Gouda");
        let missing = g.add_loose_file_mod("ghost", "Ghost", ghost, b"whatever");

        let report = apply_loadout(
            &g.root,
            &g.lib,
            &loadout(&[(&base, true), (&missing, true)]),
        )
        .unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("not present in this install")),
            "expected a skip warning, got: {:?}",
            report.warnings
        );
        assert!(!g.loose_file(ghost).exists(), "a skip must not create it");
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda"
        );
    }

    #[test]
    fn bounded_apply_loose_file_rejects_before_game_mutation() {
        let g = FakeGame::new();
        let cursor = "G1R/Content/Slate/Cursors/Normal/Normal.PNG";
        let live = g.loose_file(cursor);
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        fs::write(&live, b"shipped").unwrap();
        let id = g.add_loose_file_mod("oversized-loose", "OversizedLoose", cursor, b"12345");

        let error = apply_loadout_with_limits(
            &g.root,
            &g.lib,
            &loadout(&[(&id, true)]),
            ApplyLimits {
                max_loose_file_bytes: 4,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("loose file payload exceeds the 4 byte limit"),
            "{error}"
        );
        assert_no_apply_artifacts(&g, &live, b"shipped");
    }

    /// A present but invalid record is recovery state, not "nothing deployed".
    #[test]
    fn apply_rejects_corrupt_record_without_touching_it() {
        let g = FakeGame::new();
        let bytes = b"{ broken recovery record";
        fs::write(crate::record_path(&g.root), bytes).unwrap();

        let error = apply_loadout(&g.root, &g.lib, &Loadout::default()).unwrap_err();
        assert!(
            error.to_string().contains("parsing deploy record"),
            "{error}"
        );
        assert_eq!(fs::read(crate::record_path(&g.root)).unwrap(), bytes);
    }

    #[test]
    fn apply_refuses_recovery_required_record() {
        let g = FakeGame::new();
        let record = DeployRecord {
            owner: "manager".into(),
            phase: crate::DeployPhase::RecoveryRequired,
            ..Default::default()
        };
        fs::write(
            crate::record_path(&g.root),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();

        let error = apply_loadout(&g.root, &g.lib, &Loadout::default()).unwrap_err();
        assert!(error.to_string().contains("RECOVERY_REQUIRED"), "{error}");
    }

    /// Applying over an active STUDIO (non-manager) deployment is refused with STUDIO_DEPLOY_ACTIVE
    /// and does not touch the studio record.
    #[test]
    fn apply_refuses_studio_record() {
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Edam");
        // Seed a studio record (owner == "").
        let studio = DeployRecord {
            mod_name: "SoloMod".into(),
            ..Default::default()
        };
        fs::write(
            crate::record_path(&g.root),
            serde_json::to_vec(&studio).unwrap(),
        )
        .unwrap();

        let err = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap_err();
        assert!(
            err.to_string().contains("STUDIO_DEPLOY_ACTIVE"),
            "got: {err}"
        );
        assert!(err.to_string().contains("SoloMod"));
        // Studio record untouched (guard tripped before any undeploy/commit).
        let after = crate::read_record(&g.root).unwrap().unwrap().record;
        assert_eq!(after.mod_name, "SoloMod");
        assert_eq!(after.owner, "");
    }

    /// A single imported-format-2 shape can carry the replace-only `FilePatch` route and the
    /// additive `PakFilePatch` route together. Manager apply records both exact destinations;
    /// undeploy restores the former and deletes the latter while preserving unrelated files.
    /// The test is hermetic filesystem/receipt evidence only, not an Unreal/runtime claim.
    #[test]
    fn mixed_file_and_pak_file_patch_share_one_receipt_and_clean_undeploy() {
        let game = FakeGame::new();
        let loose_target = "G1R/Content/Movies/Intro.bk2";
        let pak_target = "G1R/Content/Slate/Cursors/Normal/Normal.PNG";
        let live = game.loose_file(loose_target);
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        fs::write(&live, b"PRISTINE-INTRO").unwrap();
        let unrelated = game.mods().join("user-owned.pak");
        fs::write(&unrelated, b"KEEP").unwrap();

        let loose_payload = "files/0_payload";
        let pak_payload = "pak_files/0_payload";
        let id = game.add_mod(
            "mod-mixed",
            "Mixed",
            vec![
                ComponentInfo::FilePatch {
                    rel: "files".into(),
                    targets: vec![loose_target.into()],
                },
                ComponentInfo::PakFilePatch {
                    rel: "pak_files".into(),
                    targets: vec![pak_target.into()],
                },
            ],
            |dir| {
                fs::create_dir_all(dir.join("files")).unwrap();
                fs::create_dir_all(dir.join("pak_files")).unwrap();
                let loose_manifest =
                    BTreeMap::from([(loose_target.to_string(), loose_payload.to_string())]);
                let pak_manifest =
                    BTreeMap::from([(pak_target.to_string(), pak_payload.to_string())]);
                fs::write(
                    dir.join("files/manifest.json"),
                    serde_json::to_vec(&loose_manifest).unwrap(),
                )
                .unwrap();
                fs::write(
                    dir.join("pak_files/manifest.json"),
                    serde_json::to_vec(&pak_manifest).unwrap(),
                )
                .unwrap();
                fs::write(dir.join(loose_payload), b"MODDED-INTRO").unwrap();
                fs::write(dir.join(pak_payload), b"PACKED-CURSOR").unwrap();
            },
        );

        let report = apply_loadout(&game.root, &game.lib, &loadout(&[(&id, true)])).unwrap();
        assert_eq!(report.applied, vec!["Mixed"]);
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );
        assert_eq!(fs::read(&live).unwrap(), b"MODDED-INTRO");
        assert_eq!(fs::read(crate::bak_path(&live)).unwrap(), b"PRISTINE-INTRO");

        let manager_name = format!("gm000_{id}");
        let pak_name = format!(
            "zzz_{manager_name}_{}_0_files_1_P.pak",
            crate::name_hash(&manager_name)
        );
        let pak = game.mods().join(pak_name);
        assert_eq!(
            gore_tex::container::list_pak_files(&pak).unwrap(),
            vec![pak_target.to_string()]
        );
        let record = crate::read_record(&game.root).unwrap().unwrap().record;
        assert_eq!(record.owner, "manager");
        assert_eq!(record.loadout, loadout(&[(&id, true)]).entries);
        assert!(record.backups.iter().any(|(recorded_live, backup, _)| {
            crate::same_path(&live, recorded_live)
                && crate::same_path(&crate::bak_path(&live), backup)
        }));
        assert!(record
            .texture_triplets
            .iter()
            .any(|recorded| crate::same_path(&pak, recorded)));
        assert!(record
            .deployed_hashes
            .keys()
            .any(|recorded| crate::same_path(&live, recorded)));
        assert!(record
            .deployed_hashes
            .keys()
            .any(|recorded| crate::same_path(&pak, recorded)));

        assert!(undeploy_all(&game.root).unwrap());
        assert_eq!(fs::read(&live).unwrap(), b"PRISTINE-INTRO");
        assert!(!crate::bak_path(&live).exists());
        assert!(!pak.exists());
        assert_eq!(fs::read(&unrelated).unwrap(), b"KEEP");
        assert!(!crate::record_path(&game.root).exists());
    }

    /// Two `PakFilePatch` mods are rebuilt into the enabled loadout's exact gm000/gm001 slots.
    /// Reordering swaps their archive bytes into the corresponding new slots, reapplying a
    /// narrowed loadout removes every stale archive, and undeploy removes only manager-owned paks.
    /// This proves deterministic filesystem/receipt behavior only; it does not qualify Unreal
    /// mount priority or runtime behavior.
    #[test]
    fn pak_file_patch_reorder_reapply_and_undeploy_are_deterministic() {
        let game = FakeGame::new();
        let target = "G1R/Content/Slate/Cursors/Normal/Normal.PNG";
        let alpha = game.add_pak_file_mod("mod-alpha", "Alpha", target, b"ALPHA-CURSOR");
        let bravo = game.add_pak_file_mod("mod-bravo", "Bravo", target, b"BRAVO-CURSOR");
        let unrelated = game.mods().join("user-owned.pak");
        fs::write(&unrelated, b"KEEP").unwrap();

        let pak_name = |slot: usize, id: &str, component: usize| {
            let manager_name = format!("gm{slot:03}_{id}");
            format!(
                "zzz_{manager_name}_{}_{}_files_{}_P.pak",
                crate::name_hash(&manager_name),
                component,
                slot + 1
            )
        };
        let manager_paks = || {
            let mut names = fs::read_dir(game.mods())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|name| {
                    name.starts_with("zzz_gm")
                        && name.contains("_files_")
                        && name.ends_with("_P.pak")
                })
                .collect::<Vec<_>>();
            names.sort();
            names
        };

        let alpha_slot_0 = pak_name(0, &alpha, 0);
        let bravo_slot_1 = pak_name(1, &bravo, 1);
        apply_loadout(
            &game.root,
            &game.lib,
            &loadout(&[(&alpha, true), (&bravo, true)]),
        )
        .unwrap();
        assert_eq!(
            manager_paks(),
            vec![alpha_slot_0.clone(), bravo_slot_1.clone()]
        );
        let alpha_archive = fs::read(game.mods().join(&alpha_slot_0)).unwrap();
        let bravo_archive = fs::read(game.mods().join(&bravo_slot_1)).unwrap();
        assert_ne!(
            alpha_archive, bravo_archive,
            "different payloads must produce distinguishable archives"
        );
        for name in [&alpha_slot_0, &bravo_slot_1] {
            assert_eq!(
                gore_tex::container::list_pak_files(&game.mods().join(name)).unwrap(),
                vec![target.to_string()],
                "the additive archive must claim the declared game path"
            );
        }

        // Reorder: each mod gets the new enabled slot and component ordinal. The archive bytes
        // follow the mod, while every old slot path is removed by the transactional reapply.
        let bravo_slot_0 = pak_name(0, &bravo, 0);
        let alpha_slot_1 = pak_name(1, &alpha, 1);
        apply_loadout(
            &game.root,
            &game.lib,
            &loadout(&[(&bravo, true), (&alpha, true)]),
        )
        .unwrap();
        assert_eq!(
            manager_paks(),
            vec![bravo_slot_0.clone(), alpha_slot_1.clone()]
        );
        assert_eq!(
            fs::read(game.mods().join(&bravo_slot_0)).unwrap(),
            bravo_archive
        );
        assert_eq!(
            fs::read(game.mods().join(&alpha_slot_1)).unwrap(),
            alpha_archive
        );
        assert!(!game.mods().join(&alpha_slot_0).exists());
        assert!(!game.mods().join(&bravo_slot_1).exists());

        // Disable the former first entry. Alpha compacts back to gm000/ordinal 0 and no archive
        // from the previous two-mod deployment survives.
        apply_loadout(
            &game.root,
            &game.lib,
            &loadout(&[(&bravo, false), (&alpha, true)]),
        )
        .unwrap();
        assert_eq!(manager_paks(), vec![alpha_slot_0.clone()]);
        assert_eq!(
            fs::read(game.mods().join(&alpha_slot_0)).unwrap(),
            alpha_archive
        );
        assert!(!game.mods().join(&bravo_slot_0).exists());
        assert!(!game.mods().join(&alpha_slot_1).exists());

        assert!(undeploy_all(&game.root).unwrap());
        assert!(
            manager_paks().is_empty(),
            "undeploy must remove every owned pak"
        );
        assert_eq!(fs::read(&unrelated).unwrap(), b"KEEP");
        assert!(!crate::record_path(&game.root).exists());
    }

    /// Re-applying after disabling a mod recomputes from pristine: the disabled mod's pak is gone
    /// and the loc reflects only the still-enabled mod's edit (not a stale merge).
    #[test]
    fn reapply_after_toggle_recomputes() {
        let g = FakeGame::new();
        // mod-a: a pak + edits cheese→Gouda. mod-b: edits cheese→Brie (wins while enabled).
        let a = g.add_mod(
            "mod-a",
            "Alpha",
            vec![
                ComponentInfo::LoosePak {
                    rel: "alpha_P.pak".into(),
                    targets: vec![],
                },
                ComponentInfo::LocPatch {
                    rel: "loc/edits.json".into(),
                    targets: vec![],
                },
            ],
            |dir| {
                fs::write(dir.join("alpha_P.pak"), b"PAK-A").unwrap();
                fs::create_dir_all(dir.join("loc")).unwrap();
                let mut e: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
                e.entry("itfo_cheese".into())
                    .or_default()
                    .insert("german".into(), "Gouda".into());
                fs::write(dir.join("loc/edits.json"), serde_json::to_vec(&e).unwrap()).unwrap();
            },
        );
        let b = g.add_loc_mod("mod-b", "Bravo", "itfo_cheese", "Brie");

        // Both enabled: mod-b wins, mod-a's pak present.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, true)])).unwrap();
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Brie"
        );
        assert!(g.mods().join("zzz_gm000_alpha_1_P.pak").is_file());

        // Disable mod-b and re-apply: pristine base → only mod-a's Gouda, mod-a's pak now slot 0.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, false)])).unwrap();
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda",
            "must recompute from pristine, not merge onto stale Brie"
        );
        // mod-a stays slot 0 (still the first ENABLED entry).
        assert!(g.mods().join("zzz_gm000_alpha_1_P.pak").is_file());
        // No orphan from mod-b (it never shipped a pak) and nothing left over.
        let entries: Vec<_> = fs::read_dir(g.mods())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "exactly mod-a's pak in ~mods: {entries:?}"
        );
    }

    /// undeploy_all restores the pristine .lcache byte-for-byte and reports whether anything was
    /// deployed.
    #[test]
    fn undeploy_all_restores_pristine() {
        let g = FakeGame::new();
        let pristine = fs::read(g.lcache()).unwrap();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Emmental");

        assert!(!undeploy_all(&g.root).unwrap(), "nothing deployed yet");
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_ne!(
            fs::read(g.lcache()).unwrap(),
            pristine,
            "deploy changed the lcache"
        );

        assert!(undeploy_all(&g.root).unwrap(), "a deployment was undone");
        assert_eq!(
            fs::read(g.lcache()).unwrap(),
            pristine,
            "pristine restored byte-identical"
        );
        assert!(
            !crate::record_path(&g.root).exists(),
            "record removed after clean undeploy"
        );
    }

    /// A RawFile lcache mod supplies the BASE; a separate LocPatch mod's edit lands ON TOP of it.
    /// Final lcache = the rawfile's content with the loc edit applied (the rawfile-vs-patch layering).
    #[test]
    fn rawfile_lcache_is_base_then_patched() {
        let g = FakeGame::new();
        // A distinct base lcache: cheese="RawCheese", apple="RawApple" — different from pristine.
        let raw_bytes = build_lcache(&[("itfo_cheese", "RawCheese"), ("itfo_apple", "RawApple")]);
        let raw = g.add_mod(
            "mod-raw",
            "RawBase",
            vec![ComponentInfo::RawFile {
                rel: "loc.lcache".into(),
                target_file: RawTarget::Lcache,
            }],
            |dir| fs::write(dir.join("loc.lcache"), &raw_bytes).unwrap(),
        );
        // A loc-patch mod edits cheese on top.
        let patch = g.add_loc_mod("mod-patch", "Patch", "itfo_cheese", "PatchedCheese");
        // Raw first (base), patch second (on top).
        let lo = loadout(&[(&raw, true), (&patch, true)]);

        let report = apply_loadout(&g.root, &g.lib, &lo).unwrap();
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );

        let live = fs::read(g.lcache()).unwrap();
        // apple came from the RAW base (proves the base is the rawfile, not pristine)...
        assert_eq!(
            read_loc(&live, "itfo_apple"),
            "RawApple",
            "base must be the rawfile"
        );
        // ...cheese was patched on top of that base.
        assert_eq!(
            read_loc(&live, "itfo_cheese"),
            "PatchedCheese",
            "loc patch lands on the raw base"
        );
    }

    /// The status ladder end-to-end via apply: NothingDeployed → InSync → ChangesPending →
    /// GameUpdated (truncating a deployed live file).
    #[test]
    fn status_transitions() {
        use crate::mgr::status::{status, ManagerStatus};
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        let b = g.add_loc_mod("mod-b", "Bravo", "itfo_apple", "Pomme");
        let target = loadout(&[(&a, true), (&b, true)]);

        // 1) Nothing deployed.
        assert_eq!(
            status(&g.root, &g.lib, &target).unwrap(),
            ManagerStatus::NothingDeployed
        );

        // 2) Apply the target → InSync (apply records each mod's fingerprint; the library is
        //    unchanged, so status confirms it and reports InSync).
        apply_loadout(&g.root, &g.lib, &target).unwrap();
        assert_eq!(
            status(&g.root, &g.lib, &target).unwrap(),
            ManagerStatus::InSync {
                loadout: vec![
                    LoadoutEntry {
                        id: "mod-a".into(),
                        enabled: true
                    },
                    LoadoutEntry {
                        id: "mod-b".into(),
                        enabled: true
                    },
                ]
            }
        );

        // 3) Ask for a different target (mod-b disabled) → ChangesPending.
        let narrowed = loadout(&[(&a, true), (&b, false)]);
        assert!(matches!(
            status(&g.root, &g.lib, &narrowed).unwrap(),
            ManagerStatus::ChangesPending { .. }
        ));

        // 4) Externally truncate a deployed live file → GameUpdated.
        fs::write(g.lcache(), b"").unwrap();
        match status(&g.root, &g.lib, &target).unwrap() {
            ManagerStatus::GameUpdated { drifted } => {
                assert_eq!(drifted.len(), 1);
                assert!(drifted[0].ends_with("AlkimiaLocalization_0.lcache"));
            }
            other => panic!("expected GameUpdated, got {other:?}"),
        }
    }

    /// End-to-end same-id UPDATE: apply mod-a (whose fingerprint is recorded), then re-import it
    /// under the SAME id with different components (rewrite its library sidecar) — the loadout ids
    /// are unchanged, but the content fingerprint now differs from the recorded one. Status must
    /// report ChangesPending (the deployed bytes are stale), NOT InSync — the bug this fix targets.
    #[test]
    fn status_same_id_update_is_changes_pending() {
        use crate::mgr::status::{status, ManagerStatus};
        let g = FakeGame::new();
        // A loc mod editing cheese→Gouda; apply records its fingerprint.
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        let target = loadout(&[(&a, true)]);
        apply_loadout(&g.root, &g.lib, &target).unwrap();
        assert_eq!(
            status(&g.root, &g.lib, &target).unwrap(),
            ManagerStatus::InSync {
                loadout: vec![LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: true
                }],
            }
        );

        // Re-import mod-a as an UPDATE: SAME id, but a different loc edit → different components →
        // different fingerprint. Rewrite only the library sidecar (what a re-import produces).
        let dir = g.lib.join("mod-a");
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("german".into(), "Brie".into());
        fs::write(
            dir.join("loc/edits.json"),
            serde_json::to_vec(&edits).unwrap(),
        )
        .unwrap();
        let updated = ModEntryMeta {
            id: "mod-a".into(),
            kind: ModKind::Goremod,
            name: "Alpha".into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-07-03T00:00:00Z".into(),
            source: String::new(),
            // A DIFFERENT target than the original (which had itfo_cheese|german too, but the
            // fingerprint hashes the serialized components — here we add a second target so it
            // provably differs regardless of import_at).
            components: vec![ComponentInfo::LocPatch {
                rel: "loc/edits.json".into(),
                targets: vec!["itfo_cheese|german".into(), "itfo_apple|german".into()],
            }],
        };
        fs::write(
            dir.join(META_FILE),
            serde_json::to_vec_pretty(&updated).unwrap(),
        )
        .unwrap();

        // Loadout ids unchanged, but the library content fingerprint moved → ChangesPending.
        assert_eq!(
            status(&g.root, &g.lib, &target).unwrap(),
            ManagerStatus::ChangesPending {
                deployed: vec![LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: true
                }],
                target: vec![LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: true
                }],
            }
        );
    }

    // ── AUDIO materialization ───────────────────────────────────────────────────────────────────

    /// End-to-end proof that the audio arm MATERIALIZES an injected sample (not merely that a file
    /// changed): a synthesized pristine bank holds sample "shout" with a known PCM pattern; an
    /// AudioPatch mod points "shout" at a WAV carrying a DIFFERENT known pattern; after
    /// `apply_loadout` the LIVE bank is decoded back and its "shout" PCM must equal the injected
    /// pattern (and differ from the original). Exercises the real
    /// `read_wav_pcm16` → `replace_samples` path with the `GOTHIC_STUDIO_KEY` fallback
    /// (`resolve_fmod_key` finds no `gore_fmod_key.json`, so the constant key is used on both sides).
    #[test]
    fn apply_audio_replaces_sample() {
        let g = FakeGame::new();
        let key = gore_fmod::GOTHIC_STUDIO_KEY;

        // Pristine bank: "shout" = an ascending known pattern.
        let orig: Vec<i16> = (0..64).map(|i| (i * 300 - 9000) as i16).collect();
        let bank = build_pristine_bank("shout", 44100, &orig, key);
        fs::write(g.bank("Voice.bank"), &bank).unwrap();
        // Precondition: the live bank really does decode to the original pattern.
        assert_eq!(
            decode_last_fsb5_pcm(&fs::read(g.bank("Voice.bank")).unwrap(), key),
            orig
        );

        // The replacement WAV carries a DIFFERENT known pattern (also a different length).
        let repl: Vec<i16> = (0..80).map(|i| (12000 - i * 250) as i16).collect();
        let wav = gore_fmod::wav_pcm16(44100, 1, &repl);
        let a = g.add_audio_mod("mod-audio", "AudioMod", "Voice.bank", "shout", &wav);

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_eq!(report.applied, vec!["AudioMod".to_string()]);
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );

        // Decode the LIVE bank: "shout" now carries the injected pattern, not the original.
        let live = fs::read(g.bank("Voice.bank")).unwrap();
        assert!(
            !gore_fmod::is_pristine_bank(&live),
            "modded bank has a 2nd FSB5"
        );
        let got = decode_last_fsb5_pcm(&live, key);
        assert_eq!(got, repl, "live bank must carry the INJECTED sample PCM");
        assert_ne!(got, orig, "injected sample must differ from the original");
    }

    #[test]
    fn apply_audio_bank_case_variants_share_one_output_and_last_patch_wins() {
        fn run(reverse: bool) {
            let g = FakeGame::new();
            let key = gore_fmod::GOTHIC_STUDIO_KEY;
            let live = g.bank("Voice.bank");
            fs::write(&live, build_pristine_bank("shout", 44100, &[0i16; 64], key)).unwrap();

            let pcm_a: Vec<i16> = (0..40).map(|i| (1000 + i * 20) as i16).collect();
            let pcm_b: Vec<i16> = (0..52).map(|i| (8000 - i * 30) as i16).collect();
            let a = g.add_audio_mod(
                "audio-a",
                "AudioA",
                "Voice.bank",
                "shout",
                &gore_fmod::wav_pcm16(44100, 1, &pcm_a),
            );
            let b = g.add_audio_mod(
                "audio-b",
                "AudioB",
                "voice.BANK",
                "shout",
                &gore_fmod::wav_pcm16(44100, 1, &pcm_b),
            );
            let (entries, expected) = if reverse {
                ([(b.as_str(), true), (a.as_str(), true)], pcm_a.as_slice())
            } else {
                ([(a.as_str(), true), (b.as_str(), true)], pcm_b.as_slice())
            };

            let report = apply_loadout(&g.root, &g.lib, &loadout(&entries)).unwrap();
            assert!(
                report.warnings.is_empty(),
                "warnings: {:?}",
                report.warnings
            );
            assert_eq!(
                decode_last_fsb5_pcm(&fs::read(&live).unwrap(), key),
                expected,
                "the last patch of one case-insensitive bank/sample target must win"
            );

            let record = crate::read_record(&g.root).unwrap().unwrap().record;
            assert_eq!(record.backups.len(), 1, "one live bank must be backed up");
            assert_eq!(
                record.deployed_hashes.len(),
                1,
                "case aliases must materialize one live output"
            );
            let bank_names: Vec<String> = fs::read_dir(live.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.to_lowercase().ends_with(".bank"))
                .collect();
            let expected_name = if cfg!(windows) && !reverse {
                "voice.BANK"
            } else {
                "Voice.bank"
            };
            assert_eq!(bank_names, vec![expected_name]);
        }

        run(false);
        run(true);
    }

    #[test]
    fn apply_audio_bank_identity_does_not_expand_sharp_s_into_ss() {
        fn run(reverse: bool) {
            let g = FakeGame::new();
            let key = gore_fmod::GOTHIC_STUDIO_KEY;
            let sharp_live = g.bank("Voiceß.bank");
            let ss_live = g.bank("VoiceSS.bank");
            fs::write(
                &sharp_live,
                build_pristine_bank("shout", 44100, &[0i16; 32], key),
            )
            .unwrap();
            fs::write(
                &ss_live,
                build_pristine_bank("shout", 44100, &[0i16; 32], key),
            )
            .unwrap();

            let sharp_pcm: Vec<i16> = (0..40).map(|i| (1000 + i * 20) as i16).collect();
            let ss_pcm: Vec<i16> = (0..52).map(|i| (8000 - i * 30) as i16).collect();
            let sharp = g.add_audio_mod(
                "audio-sharp",
                "AudioSharp",
                "Voiceß.bank",
                "shout",
                &gore_fmod::wav_pcm16(44100, 1, &sharp_pcm),
            );
            let ss = g.add_audio_mod(
                "audio-ss",
                "AudioSs",
                "VoiceSS.bank",
                "shout",
                &gore_fmod::wav_pcm16(44100, 1, &ss_pcm),
            );
            let entries = if reverse {
                [(ss.as_str(), true), (sharp.as_str(), true)]
            } else {
                [(sharp.as_str(), true), (ss.as_str(), true)]
            };

            let report = apply_loadout(&g.root, &g.lib, &loadout(&entries)).unwrap();
            assert!(
                report.warnings.is_empty(),
                "warnings: {:?}",
                report.warnings
            );
            assert_eq!(
                decode_last_fsb5_pcm(&fs::read(&sharp_live).unwrap(), key),
                sharp_pcm,
                "the sharp-s bank must retain its own patch"
            );
            assert_eq!(
                decode_last_fsb5_pcm(&fs::read(&ss_live).unwrap(), key),
                ss_pcm,
                "the SS bank must retain its own patch"
            );

            let record = crate::read_record(&g.root).unwrap().unwrap().record;
            assert_eq!(record.backups.len(), 2, "both distinct banks need backups");
            assert_eq!(
                record.deployed_hashes.len(),
                2,
                "both distinct banks must materialize their own output"
            );
        }

        run(false);
        run(true);
    }

    /// A `RawFile{Bank}` supplies the whole bank BASE; an AudioPatch then injects on top of it —
    /// mirroring the loc rawfile-then-patch layering for audio. Match the two bank spellings using
    /// Windows identity and prove composition is independent of their relative loadout order.
    #[test]
    fn apply_audio_rawfile_bank_is_base_then_patched() {
        fn run(patch_first: bool) {
            let g = FakeGame::new();
            let key = gore_fmod::GOTHIC_STUDIO_KEY;
            // Live pristine bank (pattern P0). The rawfile base overrides it with pattern P1.
            fs::write(
                g.bank("Voice.bank"),
                build_pristine_bank("shout", 44100, &[0i16; 64], key),
            )
            .unwrap();
            let base_pat: Vec<i16> = (0..64).map(|i| (i * 100) as i16).collect();
            let raw_bank = build_pristine_bank("shout", 44100, &base_pat, key);
            let raw = g.add_rawfile_mod(
                "mod-rawbank",
                "RawBank",
                RawTarget::Bank {
                    name: "Voice.bank".into(),
                },
                &raw_bank,
            );
            // Deliberately use different casing: it is the same bank on the target Windows host.
            let patch_pat: Vec<i16> = (0..48).map(|i| (7000 - i * 100) as i16).collect();
            let wav = gore_fmod::wav_pcm16(44100, 1, &patch_pat);
            let patch =
                g.add_audio_mod("mod-audiopatch", "AudioPatch", "voice.BANK", "shout", &wav);
            let entries = if patch_first {
                [(patch.as_str(), true), (raw.as_str(), true)]
            } else {
                [(raw.as_str(), true), (patch.as_str(), true)]
            };

            let report = apply_loadout(&g.root, &g.lib, &loadout(&entries)).unwrap();
            assert!(
                report.warnings.is_empty(),
                "warnings: {:?}",
                report.warnings
            );

            let live = fs::read(g.bank("Voice.bank")).unwrap();
            let got = decode_last_fsb5_pcm(&live, key);
            assert_eq!(
                got, patch_pat,
                "patch pattern must win over the rawfile base"
            );
            // The original sub-bank remains embedded after injection. Its PCM proves the raw bank,
            // not the game's zero-filled pristine bank, supplied the composed base.
            let view = gore_fmod::read_bank(&live, key).unwrap();
            let (block, fsb) = &view.sub_banks[0];
            let wav = gore_fmod::extract_wav(block, fsb, 0).unwrap();
            let (_, _, base_pcm) = gore_fmod::read_wav_pcm16(&wav).unwrap();
            assert_eq!(base_pcm, base_pat, "raw winner must supply the base");
        }

        run(false);
        run(true);
    }

    #[test]
    fn apply_raw_banks_casefold_last_wins_in_both_orders() {
        fn run(reverse: bool) {
            let g = FakeGame::new();
            let key = gore_fmod::GOTHIC_STUDIO_KEY;
            let live = g.bank("Voice.bank");
            fs::write(&live, build_pristine_bank("shout", 44100, &[0i16; 16], key)).unwrap();

            let bytes_a = build_pristine_bank("shout", 44100, &[111i16; 24], key);
            let bytes_b = build_pristine_bank("shout", 44100, &[222i16; 32], key);
            let a = g.add_rawfile_mod(
                "raw-a",
                "RawA",
                RawTarget::Bank {
                    name: "Voice.bank".into(),
                },
                &bytes_a,
            );
            let b = g.add_rawfile_mod(
                "raw-b",
                "RawB",
                RawTarget::Bank {
                    name: "voice.BANK".into(),
                },
                &bytes_b,
            );
            let (entries, expected) = if reverse {
                ([(b.as_str(), true), (a.as_str(), true)], &bytes_a)
            } else {
                ([(a.as_str(), true), (b.as_str(), true)], &bytes_b)
            };

            let report = apply_loadout(&g.root, &g.lib, &loadout(&entries)).unwrap();
            assert!(
                report.warnings.is_empty(),
                "warnings: {:?}",
                report.warnings
            );
            assert_eq!(fs::read(&live).unwrap().as_slice(), expected.as_slice());

            let bank_names: Vec<String> = fs::read_dir(live.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.to_lowercase().ends_with(".bank"))
                .collect();
            // Windows keeps the last winner's spelling for this one case-insensitive file. Unix
            // test hosts resolve back to the single existing entry to emulate that identity.
            let expected_name = if cfg!(windows) && !reverse {
                "voice.BANK"
            } else {
                "Voice.bank"
            };
            assert_eq!(bank_names, vec![expected_name]);
        }

        run(false);
        run(true);
    }

    // ── SCRIPTS materialization ─────────────────────────────────────────────────────────────────

    /// End-to-end proof that the script arm MATERIALIZES a splice: a synthesized pristine cache
    /// holds one module; an AngelScriptPatch `add` mod ships a 1-module mini-cache; after
    /// `apply_loadout` the LIVE cache is re-walked with the real `gore_as` reader and must contain
    /// BOTH modules (count bumped, new module name present) — i.e. `splice_auto` actually ran on the
    /// bytes on disk, not just by construction. (See `build_script_cache` for why a synthetic cache
    /// is used: gore-as's real samples are gitignored scratch, absent in-tree.)
    #[test]
    fn apply_scripts_splice_module() {
        use gore_as::cache::walk_modules::{module_count, module_names};
        let g = FakeGame::new();

        // Pristine cache with a single base module.
        let base = build_script_cache(&["_gore_base"]);
        fs::write(g.script_cache(), &base).unwrap();
        assert_eq!(module_count(&base), 1);

        // Mod ships a 1-module mini to ADD.
        let mini = build_script_cache(&["_gore_added"]);
        let a = g.add_script_mod("mod-as", "AsMod", "add", "_gore_added", &mini);

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_eq!(report.applied, vec!["AsMod".to_string()]);
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );

        // The LIVE cache now has 2 modules including the added one — proving the splice ran on disk.
        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(
            module_count(&live),
            2,
            "add must bump the module count on the live cache"
        );
        let names = module_names(&live).unwrap();
        assert_eq!(
            names,
            vec!["_gore_base".to_string(), "_gore_added".to_string()]
        );
    }

    #[test]
    fn apply_rejects_stale_script_minis_regardless_of_mod_origin() {
        for (kind, id) in [
            (ModKind::Goremod, "gore-script-mini"),
            (ModKind::ForeignMixed, "external-script-mini"),
        ] {
            let g = FakeGame::new();
            let mut base = build_script_cache(&["_gore_base"]);
            base[..16].copy_from_slice(&[0x11; 16]);
            fs::write(g.script_cache(), &base).unwrap();

            let mut mini = build_script_cache(&["_gore_added"]);
            mini[..16].copy_from_slice(&[0x22; 16]);
            let script = g.add_script_mod(id, "StaleScriptMini", "add", "_gore_added", &mini);
            let sidecar = g.lib.join(&script).join(META_FILE);
            let mut meta: ModEntryMeta =
                serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
            meta.kind = kind;
            fs::write(&sidecar, serde_json::to_vec_pretty(&meta).unwrap()).unwrap();
            let before_tree = crate::tree_fingerprint(&g.root).unwrap();

            let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&script, true)]))
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("does not match target base GUID")
                    && error.contains("remap the module against this exact game cache"),
                "{kind:?} must use the same script validation; unexpected error: {error}"
            );
            assert_no_apply_artifacts(&g, &g.script_cache(), &base);
            assert_eq!(
                crate::tree_fingerprint(&g.root).unwrap(),
                before_tree,
                "refused {kind:?} script composition must leave the game tree byte-identical"
            );
        }
    }

    #[test]
    fn script_overlay_validates_an_external_raw_replacement_before_mutation() {
        let g = FakeGame::new();
        let base = build_script_cache(&["_gore_base"]);
        fs::write(g.script_cache(), &base).unwrap();

        let raw = g.add_rawfile_mod(
            "external-raw-script",
            "ExternalRawScript",
            RawTarget::ScriptCache,
            b"not a script cache",
        );
        let raw_sidecar = g.lib.join(&raw).join(META_FILE);
        let mut raw_meta: ModEntryMeta =
            serde_json::from_slice(&fs::read(&raw_sidecar).unwrap()).unwrap();
        raw_meta.kind = ModKind::ForeignRawfile;
        fs::write(&raw_sidecar, serde_json::to_vec_pretty(&raw_meta).unwrap()).unwrap();

        let mini = build_script_cache(&["_gore_added"]);
        let patch = g.add_script_mod(
            "script-overlay",
            "ScriptOverlay",
            "add",
            "_gore_added",
            &mini,
        );
        let before_tree = crate::tree_fingerprint(&g.root).unwrap();

        let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&raw, true), (&patch, true)]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("prepare script composition"),
            "unexpected error: {error}"
        );
        assert_no_apply_artifacts(&g, &g.script_cache(), &base);
        assert_eq!(
            crate::tree_fingerprint(&g.root).unwrap(),
            before_tree,
            "an invalid external raw base must be refused before the game changes"
        );
    }

    #[test]
    fn script_overlay_uses_complete_raw_replacement_as_its_base_in_both_orders() {
        use gore_as::cache::walk_modules::{module_count, module_names};

        fn run(reverse: bool) {
            let g = FakeGame::new();

            let mut pristine = build_script_cache(&["_installed_pristine"]);
            pristine[..16].copy_from_slice(&[0x11; 16]);
            fs::write(g.script_cache(), &pristine).unwrap();

            // A complete replacement owns its GUID. The mini was built against that replacement,
            // not against the installed cache, so it carries the replacement's exact GUID.
            let raw_guid = [0xa5; 16];
            let mut raw_cache = build_script_cache(&["_raw_base"]);
            raw_cache[..16].copy_from_slice(&raw_guid);
            let raw = g.add_rawfile_mod(
                "raw-script-base",
                "RawScriptBase",
                RawTarget::ScriptCache,
                &raw_cache,
            );

            let mut mini = build_script_cache(&["_raw_patch"]);
            mini[..16].copy_from_slice(&raw_guid);
            let patch = g.add_script_mod(
                "raw-bound-script-patch",
                "RawBoundScriptPatch",
                "add",
                "_raw_patch",
                &mini,
            );

            let entries = if reverse {
                [(patch.as_str(), true), (raw.as_str(), true)]
            } else {
                [(raw.as_str(), true), (patch.as_str(), true)]
            };
            let report = match apply_loadout(&g.root, &g.lib, &loadout(&entries)) {
                Ok(report) => report,
                Err(error) => {
                    assert_no_apply_artifacts(&g, &g.script_cache(), &pristine);
                    panic!("valid raw-bound script composition failed: {error}");
                }
            };
            assert!(
                report.warnings.is_empty(),
                "warnings: {:?}",
                report.warnings
            );

            let live = fs::read(g.script_cache()).unwrap();
            assert_eq!(
                &live[..16],
                &raw_guid,
                "the raw base must own the output GUID"
            );
            assert_eq!(module_count(&live), 2);
            assert_eq!(
                module_names(&live).unwrap(),
                vec!["_raw_base".to_string(), "_raw_patch".to_string()],
                "the complete raw base and its bound patch must compose regardless of loadout order"
            );
        }

        run(false);
        run(true);
    }

    /// The script `edit` op materializes a `replace_module`: a 2-module base has one module swapped
    /// in place (count unchanged), and the LIVE cache re-walks with the replacement present and the
    /// old module gone.
    #[test]
    fn apply_scripts_replace_module() {
        use gore_as::cache::walk_modules::{module_count, module_names};
        let g = FakeGame::new();

        // Base with two modules; we will replace "_gore_old".
        let base = build_script_cache(&["_gore_keep", "_gore_old"]);
        fs::write(g.script_cache(), &base).unwrap();

        let repl_mini = build_script_cache(&["_gore_new"]);
        let a = g.add_script_mod("mod-as", "AsMod", "edit", "_gore_old", &repl_mini);

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );

        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(module_count(&live), 2, "edit keeps the module count");
        let names = module_names(&live).unwrap();
        assert!(
            names.contains(&"_gore_keep".to_string()),
            "kept module preserved: {names:?}"
        );
        assert!(
            names.contains(&"_gore_new".to_string()),
            "replacement present: {names:?}"
        );
        assert!(
            !names.contains(&"_gore_old".to_string()),
            "old module replaced: {names:?}"
        );
    }

    #[test]
    fn apply_scripts_composes_two_independent_allow_new_minis() {
        fn run(reverse: bool) -> ScriptAssignments {
            let g = FakeGame::new();
            let base = build_script_cache(&["_gore_base"]);
            fs::write(g.script_cache(), &base).unwrap();
            // These two real production-domain ScriptObject identities have the same masked FNV
            // start, 0x0B3F6760. Independently prepared minis therefore carry the same T2 ID even
            // though their other provisional keys differ. The old sequential path rejected the
            // second T2/T7 row; the loadout plan must allocate both identities together.
            const COLLIDING_TYPE_ID: i32 = 0x0B3F_6760;
            let mini_a = build_script_cache_with_static_name_and_keys(
                "_gore_collision_14",
                "FirstName",
                "CollisionType1901",
                1,
                1,
                COLLIDING_TYPE_ID,
            );
            let mini_b = build_script_cache_with_static_name_and_keys(
                "_gore_collision_7",
                "SecondName",
                "CollisionType2149",
                2,
                2,
                COLLIDING_TYPE_ID,
            );
            let a = g.add_script_mod("mod-as-a", "AsA", "add", "_gore_collision_14", &mini_a);
            let b = g.add_script_mod("mod-as-b", "AsB", "add", "_gore_collision_7", &mini_b);
            let ordered = if reverse {
                loadout(&[(&b, true), (&a, true)])
            } else {
                loadout(&[(&a, true), (&b, true)])
            };

            let report = apply_loadout(&g.root, &g.lib, &ordered).unwrap();
            assert!(
                report.warnings.is_empty(),
                "warnings: {:?}",
                report.warnings
            );

            let live = fs::read(g.script_cache()).unwrap();
            let names = gore_as::cache::walk_modules::module_names(&live).unwrap();
            let expected_names = if reverse {
                vec!["_gore_base", "_gore_collision_7", "_gore_collision_14"]
            } else {
                vec!["_gore_base", "_gore_collision_14", "_gore_collision_7"]
            };
            assert_eq!(names, expected_names);
            let refs = gore_as::cache::refs::RefResolver::build(&live).unwrap();
            let expected_static = if reverse {
                ["SecondName", "FirstName"]
            } else {
                ["FirstName", "SecondName"]
            };
            assert_eq!(refs.static_name(0), Some(expected_static[0]));
            assert_eq!(refs.static_name(1), Some(expected_static[1]));

            let assignments = script_assignments(&live);
            let type_1 = assignments.type_ids["CollisionType1901"];
            let type_2 = assignments.type_ids["CollisionType2149"];
            assert_ne!(type_1, type_2);
            assert_eq!(refs.type_by_id(type_1), Some("CollisionType1901"));
            assert_eq!(refs.type_by_id(type_2), Some("CollisionType2149"));
            let func_1 = assignments.function_ids["ProbeFunc1"];
            let func_2 = assignments.function_ids["ProbeFunc2"];
            assert_ne!(func_1, func_2);
            assert_eq!(refs.func_by_id(func_1), Some("ProbeFunc1"));
            assert_eq!(refs.func_by_id(func_2), Some("ProbeFunc2"));
            assert_eq!(refs.member(type_1, 4), Some("ProbeField1"));
            assert_eq!(refs.member(type_2, 4), Some("ProbeField2"));
            let functions =
                gore_as::cache::walk_modules::collect_function_bytecodes(&live).unwrap();
            let indices: Vec<u16> = functions
                .iter()
                .filter_map(|function| {
                    gore_as::cache::disasm::disassemble(&function.bytecode)
                        .unwrap()
                        .into_iter()
                        .find(|ins| ins.op.name == "STR")
                        .map(|instruction| instruction.words[0])
                })
                .collect();
            assert_eq!(indices, vec![0, 1], "later mini operand must be rebased");
            assignments
        }

        let forward = run(false);
        let reverse = run(true);
        assert_eq!(
            forward, reverse,
            "portable identities must keep the same canonical pointer/ID assignment under loadout reorder"
        );
    }

    #[test]
    fn apply_scripts_same_target_keeps_later_winner_and_independent_target() {
        fn run(reverse: bool) {
            let g = FakeGame::new();
            let base = build_script_cache(&["_gore_base"]);
            fs::write(g.script_cache(), &base).unwrap();

            // Both contenders claim the exact same manifest target and carry that same module name.
            // Their private symbols make it observable which mini actually reached composition.
            let mini_a = build_script_cache_with_static_name_and_keys(
                "_gore_shared",
                "SharedFromA",
                "SharedTypeA",
                31,
                31,
                0x0801_0031,
            );
            let mini_b = build_script_cache_with_static_name_and_keys(
                "_gore_shared",
                "SharedFromB",
                "SharedTypeB",
                32,
                32,
                0x0801_0032,
            );
            let independent_mini = build_script_cache_with_static_name_and_keys(
                "_gore_independent",
                "IndependentName",
                "IndependentType",
                33,
                33,
                0x0801_0033,
            );
            let a = g.add_script_mod("script-a", "Script A", "add", "_gore_shared", &mini_a);
            let independent = g.add_script_mod(
                "script-independent",
                "Independent",
                "add",
                "_gore_independent",
                &independent_mini,
            );
            let b = g.add_script_mod("script-b", "Script B", "add", "_gore_shared", &mini_b);
            let ordered = if reverse {
                loadout(&[(&b, true), (&independent, true), (&a, true)])
            } else {
                loadout(&[(&a, true), (&independent, true), (&b, true)])
            };

            apply_loadout(&g.root, &g.lib, &ordered).unwrap();

            let live = fs::read(g.script_cache()).unwrap();
            assert_eq!(
                gore_as::cache::walk_modules::module_names(&live).unwrap(),
                vec!["_gore_base", "_gore_independent", "_gore_shared"]
            );
            let refs = gore_as::cache::refs::RefResolver::build(&live).unwrap();
            let (winner_name, winner_type, loser_type) = if reverse {
                ("SharedFromA", "SharedTypeA", "SharedTypeB")
            } else {
                ("SharedFromB", "SharedTypeB", "SharedTypeA")
            };
            assert_eq!(refs.static_name(0), Some("IndependentName"));
            assert_eq!(refs.static_name(1), Some(winner_name));
            assert_eq!(refs.static_name(2), None, "only two minis may be composed");

            let assignments = script_assignments(&live);
            assert!(assignments.type_ids.contains_key("IndependentType"));
            assert!(assignments.type_ids.contains_key(winner_type));
            assert!(
                !assignments.type_ids.contains_key(loser_type),
                "the earlier entry for one exact module target must not reach planning or compose"
            );
        }

        run(false);
        run(true);
    }

    #[test]
    fn apply_scripts_multi_module_mini_is_one_unit_under_later_wins() {
        use gore_as::cache::walk_modules::{collect_function_bytecodes, module_names};
        let g = FakeGame::new();
        let base = build_script_cache(&["_gore_base"]);
        fs::write(g.script_cache(), &base).unwrap();

        // X carries two modules but names only `_gore_a`; Y adds `_gore_b` alone.
        let multi = build_script_cache(&["_gore_a", "_gore_b"]);
        let mini_b = build_script_cache_with_static_name_and_keys(
            "_gore_b",
            "FromY",
            "TypeY",
            41,
            41,
            0x0801_0041,
        );
        let x = g.add_script_mod("script-x", "Script X", "add", "_gore_a", &multi);
        let y = g.add_script_mod("script-y", "Script Y", "add", "_gore_b", &mini_b);

        // Y later re-targets only one of X's modules: X would keep `_gore_a` but lose `_gore_b`,
        // and its stale `_gore_b` rows cannot be pruned from the plan or the tail. Refuse.
        let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&x, true), (&y, true)]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("composes as one unit") && error.contains("_gore_b"),
            "{error}"
        );
        assert_eq!(
            module_names(&fs::read(g.script_cache()).unwrap()).unwrap(),
            vec!["_gore_base"],
            "a refused loadout must leave the pristine cache untouched"
        );

        // X later carries `_gore_b` too, so Y contributes nothing and is dropped; X composes alone.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&y, true), (&x, true)])).unwrap();
        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(
            module_names(&live).unwrap(),
            vec!["_gore_base", "_gore_a", "_gore_b"]
        );
        assert!(
            !collect_function_bytecodes(&live)
                .unwrap()
                .iter()
                .any(|f| f.func.starts_with("_gore_b::")),
            "X's empty `_gore_b` must have won over Y's"
        );
    }

    #[test]
    fn apply_scripts_edit_after_shadowed_add_uses_only_the_winner() {
        for target_exists_in_base in [false, true] {
            let g = FakeGame::new();
            let base_modules: &[&str] = if target_exists_in_base {
                &["_gore_base", "_gore_shared"]
            } else {
                &["_gore_base"]
            };
            fs::write(g.script_cache(), build_script_cache(base_modules)).unwrap();

            let shadowed = build_script_cache_with_static_name_and_keys(
                "_gore_shared",
                "ShadowedAdd",
                "ShadowedType",
                41,
                41,
                0x0801_0041,
            );
            let winner = build_script_cache_with_static_name_and_keys(
                "_gore_shared",
                "WinningEdit",
                "WinningType",
                42,
                42,
                0x0801_0042,
            );
            let add = g.add_script_mod(
                "script-prerequisite-add",
                "Prerequisite Add",
                "add",
                "_gore_shared",
                &shadowed,
            );
            let edit = g.add_script_mod(
                "script-winning-edit",
                "Winning Edit",
                "edit",
                "_gore_shared",
                &winner,
            );

            apply_loadout(&g.root, &g.lib, &loadout(&[(&add, true), (&edit, true)]))
                .unwrap_or_else(|error| {
                    panic!(
                        "same-target add -> edit failed (target_exists_in_base={target_exists_in_base}): {error}"
                    )
                });

            let live = fs::read(g.script_cache()).unwrap();
            assert_eq!(
                gore_as::cache::walk_modules::module_names(&live).unwrap(),
                vec!["_gore_base", "_gore_shared"]
            );
            let refs = gore_as::cache::refs::RefResolver::build(&live).unwrap();
            assert_eq!(refs.static_name(0), Some("WinningEdit"));
            assert_eq!(refs.static_name(1), None);
            let assignments = script_assignments(&live);
            assert!(assignments.type_ids.contains_key("WinningType"));
            assert!(!assignments.type_ids.contains_key("ShadowedType"));
        }
    }

    /// A standalone `RawFile{ScriptCache}` is structurally validated from its private candidate,
    /// then that same candidate is published VERBATIM. Its GUID belongs to the complete replacement
    /// and deliberately need not match the installed cache. Origin never changes this policy.
    #[test]
    fn apply_scripts_rawfile_written_verbatim() {
        for (kind, id, guid) in [
            (ModKind::Goremod, "gore-raw-script", 0xa5),
            (ModKind::ForeignRawfile, "foreign-raw-script", 0x5a),
        ] {
            let g = FakeGame::new();
            let pristine = build_script_cache(&["_gore_pristine"]);
            fs::write(g.script_cache(), &pristine).unwrap();

            let mut raw = build_script_cache(&["_complete_replacement"]);
            raw[..16].copy_from_slice(&[guid; 16]);
            assert_ne!(&raw[..16], &pristine[..16]);
            let entry = g.add_rawfile_mod(id, "RawSc", RawTarget::ScriptCache, &raw);
            let sidecar = g.lib.join(&entry).join(META_FILE);
            let mut meta: ModEntryMeta =
                serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
            meta.kind = kind;
            fs::write(&sidecar, serde_json::to_vec_pretty(&meta).unwrap()).unwrap();

            let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&entry, true)])).unwrap();
            assert!(
                report.warnings.is_empty(),
                "{kind:?} warnings: {:?}",
                report.warnings
            );
            assert_eq!(
                fs::read(g.script_cache()).unwrap(),
                raw,
                "{kind:?} full ScriptCache replacement must remain byte-identical"
            );
        }
    }

    #[test]
    fn standalone_script_rawfiles_reject_bad_containers_before_mutation() {
        let valid = build_script_cache(&["_complete_replacement"]);
        let short = vec![0u8; gore_as::cache::header::CacheHeader::SIZE - 1];
        let mut wrong_magic = valid.clone();
        wrong_magic[0x10..0x14].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        let mut truncated = valid.clone();
        truncated.pop();
        let mut impossible_count = valid.clone();
        impossible_count[0x14..0x18].copy_from_slice(&u32::MAX.to_le_bytes());
        let duplicate_module_key = build_script_cache(&["DuplicateModule", "DuplicateModule"]);

        for (case, malformed) in [
            ("short", short),
            ("wrong-magic", wrong_magic),
            ("truncated", truncated),
            ("impossible-count", impossible_count),
            ("duplicate-module-key", duplicate_module_key),
        ] {
            for (kind, origin) in [
                (ModKind::Goremod, "gore"),
                (ModKind::ForeignRawfile, "foreign"),
            ] {
                let g = FakeGame::new();
                let pristine = build_script_cache(&["_gore_pristine"]);
                fs::write(g.script_cache(), &pristine).unwrap();
                let id = format!("{origin}-{case}");
                let entry = g.add_rawfile_mod(
                    &id,
                    "MalformedScriptCache",
                    RawTarget::ScriptCache,
                    &malformed,
                );
                let sidecar = g.lib.join(&entry).join(META_FILE);
                let mut meta: ModEntryMeta =
                    serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
                meta.kind = kind;
                fs::write(&sidecar, serde_json::to_vec_pretty(&meta).unwrap()).unwrap();

                let error = apply_loadout(&g.root, &g.lib, &loadout(&[(&entry, true)]))
                    .unwrap_err()
                    .to_string();
                assert!(
                    error.contains("validate standalone script cache"),
                    "{kind:?}/{case} unexpected error: {error}"
                );
                assert_no_apply_artifacts(&g, &g.script_cache(), &pristine);
            }
        }
    }

    #[test]
    fn standalone_script_rawfile_uses_patch_base_limit_before_snapshot() {
        let g = FakeGame::new();
        let pristine = build_script_cache(&["_gore_pristine"]);
        fs::write(g.script_cache(), &pristine).unwrap();
        let raw = build_script_cache(&["_complete_replacement"]);
        let entry = g.add_rawfile_mod(
            "bounded-raw-script",
            "BoundedRawScript",
            RawTarget::ScriptCache,
            &raw,
        );
        let limit = u64::try_from(raw.len() - 1).unwrap();
        let before_tree = crate::tree_fingerprint(&g.root).unwrap();

        let error = apply_loadout_with_limits(
            &g.root,
            &g.lib,
            &loadout(&[(&entry, true)]),
            ApplyLimits {
                max_patch_base_bytes: limit,
                ..DEFAULT_APPLY_LIMITS
            },
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains(&format!("raw-file payload exceeds the {limit} byte limit")),
            "unexpected error: {error}"
        );
        assert_no_apply_artifacts(&g, &g.script_cache(), &pristine);
        assert_eq!(
            crate::tree_fingerprint(&g.root).unwrap(),
            before_tree,
            "the opened-file size gate must reject before snapshotting or game mutation"
        );
    }

    /// Real-cache splice, opt-in like gore-as's own `#[ignore]` tests. The synthetic fixtures above
    /// prove the arm's plumbing; this proves it against ACTUAL game bytes when available. Run with:
    ///   GORE_TEST_CACHE=<...>/PrecompiledScript_Shipping.Cache
    ///   GORE_TEST_MINI=<...>/one-module-mini.Cache
    ///   cargo test -p gore-mod -- --ignored apply_scripts_splice_real
    #[test]
    #[ignore]
    fn apply_scripts_splice_real() {
        use gore_as::cache::walk_modules::{module_count, module_names};
        let (Ok(cache), Ok(mini)) = (
            std::env::var("GORE_TEST_CACHE"),
            std::env::var("GORE_TEST_MINI"),
        ) else {
            eprintln!("skip: set GORE_TEST_CACHE and GORE_TEST_MINI to real cache + 1-module mini");
            return;
        };
        let base = fs::read(&cache).expect("read real cache");
        let mini_bytes = fs::read(&mini).expect("read real mini");
        let before = module_count(&base);
        let new_name = module_names(&mini_bytes)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let g = FakeGame::new();
        fs::write(g.script_cache(), &base).unwrap();
        let a = g.add_script_mod("mod-as-real", "AsReal", "add", &new_name, &mini_bytes);

        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();

        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(
            module_count(&live),
            before + 1,
            "real splice adds exactly one module"
        );
        assert!(
            module_names(&live).unwrap().contains(&new_name),
            "added module present"
        );
    }
}
