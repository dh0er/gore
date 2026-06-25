//! gore-mod — assemble one unified mod **bundle** (item overrides + localized-text edits +
//! audio replacements) and deploy/undeploy it to the game.
//!
//! Pipeline: `BuildSpec` → [`build_bundle`] → bundle dir (`gore-mod.json` manifest + payloads)
//! → [`deploy`]/[`undeploy`]. Each content domain is a manifest **component** with its own
//! deploy mechanism (UE4SS Lua = runtime mod; loc + audio = loose-file patches applied against
//! the user's own pristine game files, with `*.gore-bak` backups). The manifest is the
//! hand-off contract for a future stand-alone mod-manager; this crate does single-mod deploy.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gore_modgen::gen::{gen_lua, MetaConfig, OverridesConfig, SingleOverride};

pub type Files = BTreeMap<String, Vec<u8>>;

// ── Errors ───────────────────────────────────────────────────────────────────
#[derive(Debug, thiserror::Error)]
pub enum ModError {
    #[error("io: {0}")]
    Io(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("loc: {0}")]
    Loc(#[from] gore_loc::loc::LcacheError),
    #[error("fmod: {0}")]
    Fmod(String),
    #[error("{0}")]
    Other(String),
}
type Result<T> = std::result::Result<T, ModError>;

fn io<E: std::fmt::Display>(ctx: &str) -> impl FnOnce(E) -> ModError + '_ {
    move |e| ModError::Io(format!("{ctx}: {e}"))
}

// ── Spec / manifest types ──────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMeta {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
}

/// One audio sample replacement: put `wav_path`'s audio in place of `sample` in `bank`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioReplacement {
    pub bank: String,   // e.g. "SFX.bank"
    pub sample: String, // FSB5 sample name in that bank
    pub wav_path: String,
}

/// Declarative build input — the union of the editor domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSpec {
    pub meta: ModMeta,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default)]
    pub overrides: Vec<SingleOverride>,
    /// `{ locId: { setName: text } }`
    #[serde(default)]
    pub loc_edits: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub audio: Vec<AudioReplacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    /// A UE4SS Lua mod folder at `path`, deployed to `ue4ss/Mods/<name>`.
    Ue4ssLua { name: String, path: String },
    /// Declarative loc edits at `path` (`{id:{set:text}}`), applied to the .lcache.
    LocPatch { path: String },
    /// Audio patch dir at `path` (manifest.json + wavs), applied to `banks`.
    AudioPatch { path: String, banks: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub format: u32,
    #[serde(rename = "mod")]
    pub mod_meta: ModMeta,
    pub components: Vec<Component>,
}

pub struct Bundle {
    pub files: Files,
    pub manifest: ModManifest,
}

// ── Build ──────────────────────────────────────────────────────────────────────
/// Assemble the in-memory bundle (files + manifest) from a declarative spec.
pub fn build_bundle(spec: &BuildSpec) -> Result<Bundle> {
    let mut files = Files::new();
    let mut components = Vec::new();
    let name = &spec.meta.name;
    if !is_safe_mod_name(name) {
        return Err(ModError::Other(format!(
            "invalid mod name {name:?}: must be a single path component with no \
             separators, '..', or control characters"
        )));
    }

    // overrides → UE4SS Lua mod
    if !spec.overrides.is_empty() {
        let cfg = OverridesConfig {
            meta: MetaConfig { name: name.clone(), delay_ms: spec.delay_ms },
            overrides: spec.overrides.clone(),
        };
        let lua = gen_lua(&cfg);
        files.insert(format!("ue4ss/{name}/enabled.txt"), Vec::new());
        files.insert(format!("ue4ss/{name}/Scripts/main.lua"), lua.into_bytes());
        components.push(Component::Ue4ssLua { name: name.clone(), path: format!("ue4ss/{name}") });
    }

    // loc edits → declarative patch
    if !spec.loc_edits.is_empty() {
        files.insert("loc/edits.json".into(), serde_json::to_vec_pretty(&spec.loc_edits)?);
        components.push(Component::LocPatch { path: "loc/edits.json".into() });
    }

    // audio → manifest + wavs (no game audio, just the replacements)
    if !spec.audio.is_empty() {
        let mut map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for a in &spec.audio {
            let wav = std::fs::read(&a.wav_path).map_err(io(&format!("reading wav {}", a.wav_path)))?;
            let fname = format!("{}__{}.wav", sanitize(&a.bank), sanitize(&a.sample));
            files.insert(format!("audio/{fname}"), wav);
            map.entry(a.bank.clone()).or_default().insert(a.sample.clone(), format!("audio/{fname}"));
        }
        let banks: Vec<String> = map.keys().cloned().collect();
        files.insert("audio/manifest.json".into(), serde_json::to_vec_pretty(&map)?);
        components.push(Component::AudioPatch { path: "audio".into(), banks });
    }

    let manifest = ModManifest { format: 1, mod_meta: spec.meta.clone(), components };
    files.insert("gore-mod.json".into(), serde_json::to_vec_pretty(&manifest)?);
    Ok(Bundle { files, manifest })
}

/// Write a built bundle's files under `dir` (creating parent dirs).
pub fn write_bundle(dir: &Path, bundle: &Bundle) -> Result<()> {
    for (rel, bytes) in &bundle.files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io("create dir"))?;
        }
        std::fs::write(&path, bytes).map_err(io(&format!("writing {}", path.display())))?;
    }
    Ok(())
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

/// A safe mod name is a single normal path component: non-empty, no path separators, no `..`,
/// no control characters — so it can't escape the bundle/UE4SS Mods directory.
fn is_safe_mod_name(name: &str) -> bool {
    use std::path::Component;
    if name.is_empty() || name.chars().any(char::is_control) {
        return false;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    let mut comps = Path::new(name).components();
    matches!((comps.next(), comps.next()), (Some(Component::Normal(_)), None))
}

/// A safe single filename: non-empty, no separators, no `..`, no control chars.
fn is_safe_filename(name: &str) -> bool {
    is_safe_mod_name(name)
}

/// A safe relative path inside the bundle: non-empty, not absolute, every component a normal
/// name (no `..`, no root/prefix), no control characters — so it can't escape the bundle dir.
fn is_safe_rel_path(p: &str) -> bool {
    use std::path::Component;
    if p.is_empty() || p.chars().any(char::is_control) {
        return false;
    }
    let path = Path::new(p);
    if path.is_absolute() {
        return false;
    }
    let mut any = false;
    for c in path.components() {
        match c {
            Component::Normal(_) => any = true,
            _ => return false,
        }
    }
    any
}

// ── Game paths ──────────────────────────────────────────────────────────────────
/// Resolved game-install locations. `root` is the game folder that contains `G1R/`.
pub struct GamePaths {
    pub ue4ss_mods: PathBuf,
    pub fmod_desktop: PathBuf,
    pub lcache: Option<PathBuf>,
}

pub fn resolve_game_paths(root: &Path) -> GamePaths {
    let g1r = if root.file_name().is_some_and(|n| n == "G1R") {
        root.to_path_buf()
    } else {
        root.join("G1R")
    };
    let lcache = {
        let cache = g1r.join("Story").join("Cache");
        std::fs::read_dir(&cache).ok().and_then(|rd| {
            let mut matches: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("AlkimiaLocalization") && n.ends_with(".lcache"))
                })
                .collect();
            // Deterministic when several caches exist: pick the most recently modified
            // (the active one), matching gore-loc's locator.
            matches.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
            matches.pop()
        })
    };
    GamePaths {
        ue4ss_mods: g1r.join("Binaries").join("Win64").join("ue4ss").join("Mods"),
        fmod_desktop: g1r.join("Content").join("FMOD").join("Desktop"),
        lcache,
    }
}

// ── Deploy / undeploy ────────────────────────────────────────────────────────────
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeployRecord {
    pub mod_name: String,
    /// deployed UE4SS mod dir (absolute), if any
    pub ue4ss_mod_dir: Option<String>,
    /// (live_path, backup_path, created_by_this_deploy) to restore on undeploy. `created` is
    /// false when the `*.gore-bak` already existed (it belongs to a previous deployment), so a
    /// rollback restores from but does not delete it.
    pub backups: Vec<(String, String, bool)>,
}

const RECORD_NAME: &str = "gore-mod.deployed.json";

fn record_path(root: &Path) -> PathBuf {
    root.join(RECORD_NAME)
}

/// A fully-prepared deployment: everything to write, computed in memory so the failure-prone
/// work happens BEFORE the game is touched.
struct DeployPlan {
    ue4ss: Option<(PathBuf, PathBuf)>, // (source dir in bundle, dest under ue4ss/Mods)
    writes: Vec<(PathBuf, Vec<u8>)>,   // (live game file, new contents)
}

/// Deploy a built bundle dir into the game at `game_root`. Two phases so a previous working
/// deployment is never lost to a failed new one:
/// 1. **prepare** — decode/inject/encode every change in memory; on any error the game is
///    untouched and the previous mod stays active.
/// 2. **commit** — revert the previous mod's footprint this deploy won't overwrite, then apply
///    (fs ops only); if a commit write fails, the partial deploy is rolled back to pristine.
/// Single active mod.
pub fn deploy(bundle_dir: &Path, game_root: &Path) -> Result<DeployRecord> {
    let manifest_bytes = std::fs::read(bundle_dir.join("gore-mod.json"))
        .map_err(io("reading gore-mod.json"))?;
    let manifest: ModManifest = serde_json::from_slice(&manifest_bytes)?;
    // An empty bundle has nothing to apply; deploying it would only retire the active mod.
    if manifest.components.is_empty() {
        return Err(ModError::Other("bundle has no components to deploy".into()));
    }
    let gp = resolve_game_paths(game_root);

    // PHASE 1 — prepare (no game writes). The previous deployment is left intact if this fails.
    let plan = prepare(bundle_dir, &manifest, &gp)?;

    // PHASE 2 — commit. `undo` captures the exact pre-deploy state (prior file bytes + the
    // previous UE4SS mod kept aside), so any failure restores it precisely.
    let prev = read_record(game_root);
    let mut record = DeployRecord { mod_name: manifest.mod_meta.name.clone(), ..Default::default() };
    let mut undo = Undo::default();

    // (a) apply the new mod. On failure restore the exact prior state (previous mod or pristine).
    if let Err(e) = commit_new(&plan, &mut record, &mut undo) {
        undo.rollback();
        return Err(e);
    }

    // (b) persist the record BEFORE retiring the previous mod, so a write failure here restores
    //     the previous mod fully and rolls the new writes back.
    let write_res = serde_json::to_vec_pretty(&record)
        .map_err(ModError::from)
        .and_then(|b| std::fs::write(record_path(game_root), b).map_err(io("writing deploy record")));
    if let Err(e) = write_res {
        undo.rollback();
        return Err(e);
    }

    // (c) committed — drop the kept-aside previous UE4SS mod, then retire the previous mod's
    //     remaining footprint (best-effort; can't fail the deploy).
    undo.discard();
    retire_previous(prev.as_ref(), &plan);
    Ok(record)
}

/// Captures the exact pre-deploy state so a failed deploy can restore it precisely, rather than
/// only reverting to the game-pristine `*.gore-bak`.
#[derive(Default)]
struct Undo {
    /// (live, prior bytes) — write back on rollback to restore the exact pre-deploy content.
    files: Vec<(PathBuf, Vec<u8>)>,
    /// `*.gore-bak` files THIS deploy created — remove on rollback (adopted ones are kept).
    created_baks: Vec<PathBuf>,
    /// (old-aside dir, dst) — a previous UE4SS mod moved aside: restore on rollback, drop on success.
    ue4ss_old: Option<(PathBuf, PathBuf)>,
    /// a UE4SS mod installed where there was none — remove on rollback.
    ue4ss_fresh: Option<PathBuf>,
}

impl Undo {
    fn rollback(self) {
        for (live, bytes) in &self.files {
            let _ = atomic_write(live, bytes);
        }
        for bak in &self.created_baks {
            let _ = std::fs::remove_file(bak);
        }
        if let Some((old, dst)) = &self.ue4ss_old {
            let _ = std::fs::remove_dir_all(dst);
            let _ = std::fs::rename(old, dst);
        } else if let Some(dst) = &self.ue4ss_fresh {
            let _ = std::fs::remove_dir_all(dst);
        }
    }

    fn discard(self) {
        if let Some((old, _)) = &self.ue4ss_old {
            let _ = std::fs::remove_dir_all(old);
        }
    }
}

/// Build everything to write, in memory. Any error here leaves the game untouched.
fn prepare(bundle_dir: &Path, manifest: &ModManifest, gp: &GamePaths) -> Result<DeployPlan> {
    let mut plan = DeployPlan { ue4ss: None, writes: Vec::new() };
    for comp in &manifest.components {
        match comp {
            Component::Ue4ssLua { name, path } => {
                // The manifest may come from an untrusted bundle: reject names/paths that could
                // escape the bundle source or the UE4SS Mods directory.
                if !is_safe_mod_name(name) || !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!(
                        "unsafe ue4ss component in manifest: name={name:?} path={path:?}"
                    )));
                }
                plan.ue4ss = Some((bundle_dir.join(path), gp.ue4ss_mods.join(name)));
            }
            Component::LocPatch { path } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe loc patch path: {path:?}")));
                }
                let lcache = gp.lcache.clone().ok_or_else(|| {
                    ModError::Other("no AlkimiaLocalization .lcache found in game".into())
                })?;
                let pristine = read_pristine(&lcache)?;
                let edits: BTreeMap<String, BTreeMap<String, String>> =
                    serde_json::from_slice(&std::fs::read(bundle_dir.join(path)).map_err(io("reading edits.json"))?)?;
                let mut lc = gore_loc::loc::Lcache::decode(&pristine)?;
                for (id, langs) in &edits {
                    for (set, text) in langs {
                        lc.set_value(id, set, text)?;
                    }
                }
                plan.writes.push((lcache, lc.encode()?));
            }
            Component::AudioPatch { path, banks: _ } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe audio patch path: {path:?}")));
                }
                let map: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path).join("manifest.json")).map_err(io("reading audio manifest"))?,
                )?;
                for (bank, samples) in &map {
                    if !is_safe_filename(bank) {
                        return Err(ModError::Other(format!("unsafe bank name: {bank:?}")));
                    }
                    let bank_path = gp.fmod_desktop.join(bank);
                    let pristine = read_pristine(&bank_path)?;
                    let mut repl = Vec::new();
                    for (sample, wav_rel) in samples {
                        if !is_safe_rel_path(wav_rel) {
                            return Err(ModError::Other(format!("unsafe wav path: {wav_rel:?}")));
                        }
                        let wav = std::fs::read(bundle_dir.join(wav_rel)).map_err(io("reading patch wav"))?;
                        let (rate, ch, pcm) = gore_fmod::read_wav_pcm16(&wav).map_err(ModError::Fmod)?;
                        repl.push((
                            sample.clone(),
                            gore_fmod::Pcm16Sample { name: sample.clone(), freq: rate, channels: ch, pcm },
                        ));
                    }
                    let new_bank = gore_fmod::replace_samples(&pristine, gore_fmod::GOTHIC_STUDIO_KEY, repl)
                        .map_err(ModError::Fmod)?;
                    plan.writes.push((bank_path, new_bank));
                }
            }
        }
    }
    Ok(plan)
}

/// Apply a prepared plan. Reverts the previous mod's footprint not overwritten by this deploy
/// (keeping a single active mod), then writes the new contents with `*.gore-bak` backups.
/// Apply the new mod's writes (fs ops only). On error the caller rolls these back to pristine.
fn commit_new(plan: &DeployPlan, record: &mut DeployRecord, undo: &mut Undo) -> Result<()> {
    if let Some((src, dst)) = &plan.ue4ss {
        // Stage into a sibling temp dir, then swap into place — a failed/partial copy never
        // destroys a previous same-named UE4SS mod already at `dst`.
        let staging = staging_dir(dst);
        let _ = std::fs::remove_dir_all(&staging);
        // If this copy fails, `dst` (a previous mod) is untouched and the undo is empty, so
        // rollback won't delete it.
        copy_dir(src, &staging)?;
        if dst.exists() {
            // Move the old mod aside (atomic), then swap the staged copy in. The old dir is kept
            // (via the undo) until the whole deploy commits; on any failure it is moved back.
            let old = staging_old(dst);
            let _ = std::fs::remove_dir_all(&old);
            std::fs::rename(dst, &old).map_err(io("moving old ue4ss mod aside"))?;
            match std::fs::rename(&staging, dst) {
                Ok(()) => {
                    record.ue4ss_mod_dir = Some(dst.display().to_string());
                    undo.ue4ss_old = Some((old, dst.clone()));
                }
                Err(e) => {
                    let _ = std::fs::rename(&old, dst); // restore the previous mod
                    let _ = std::fs::remove_dir_all(&staging);
                    return Err(io("installing ue4ss mod")(e));
                }
            }
        } else {
            std::fs::rename(&staging, dst).map_err(io("installing ue4ss mod"))?;
            record.ue4ss_mod_dir = Some(dst.display().to_string());
            undo.ue4ss_fresh = Some(dst.clone());
        }
    }
    for (live, bytes) in &plan.writes {
        // Snapshot the current (pre-deploy) bytes so rollback restores the EXACT prior state —
        // the previous mod's content, not just the game-pristine backup.
        undo.files.push((live.clone(), std::fs::read(live).unwrap_or_default()));
        let (bak, created) = backup(live, record)?;
        if created {
            undo.created_baks.push(bak);
        }
        atomic_write(live, bytes)?;
    }
    Ok(())
}

/// Retire the previous mod's leftover footprint after the new mod is live and recorded:
/// restore loose files the new deploy did not overwrite, and remove a differently-named UE4SS
/// mod. Best-effort — never fails the deploy, and only drops a `*.gore-bak` after a successful
/// restore from it.
fn retire_previous(prev: Option<&DeployRecord>, plan: &DeployPlan) {
    let Some(prev) = prev else { return };
    for (live, bak, _created) in &prev.backups {
        if plan.writes.iter().any(|(p, _)| p.display().to_string() == *live) {
            continue;
        }
        let (live, bak) = (Path::new(live), Path::new(bak));
        if bak.exists() {
            if let Ok(b) = std::fs::read(bak) {
                if atomic_write(live, &b).is_ok() {
                    let _ = std::fs::remove_file(bak);
                }
            }
        }
    }
    if let Some(prev_dir) = &prev.ue4ss_mod_dir {
        let new_dir = plan.ue4ss.as_ref().map(|(_, dst)| dst.display().to_string());
        if new_dir.as_deref() != Some(prev_dir.as_str()) {
            let _ = std::fs::remove_dir_all(prev_dir);
        }
    }
}

fn staging_dir(dst: &Path) -> PathBuf {
    let mut s = dst.as_os_str().to_os_string();
    s.push(".gore-new");
    PathBuf::from(s)
}

fn staging_old(dst: &Path) -> PathBuf {
    let mut s = dst.as_os_str().to_os_string();
    s.push(".gore-old");
    PathBuf::from(s)
}

fn read_record(game_root: &Path) -> Option<DeployRecord> {
    std::fs::read(record_path(game_root))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

/// Pristine bytes for a game file: its `*.gore-bak` if a prior deploy preserved one, else the
/// live file (assumed pristine). Never writes.
fn read_pristine(live: &Path) -> Result<Vec<u8>> {
    let bak = bak_path(live);
    let src = if bak.exists() { bak.as_path() } else { live };
    std::fs::read(src).map_err(io(&format!("reading pristine {}", live.display())))
}

fn bak_path(live: &Path) -> PathBuf {
    let mut s = live.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

/// Restore every backup in `record` (copy `*.gore-bak` → live, delete the backup) and remove
/// the deployed UE4SS mod. Best-effort; used by both undeploy and deploy-rollback.
/// Undeploy: restore every live file from its backup and remove the UE4SS mod. Returns true
/// only if EVERY file was restored — a missing recorded backup, a failed read, or a failed
/// write all count as failure, and the surviving `*.gore-bak` files are kept so undeploy can be
/// retried. (Deploy rollback uses [`Undo`] instead, to restore the exact prior state.)
fn restore_record(record: &DeployRecord) -> bool {
    let mut all_ok = true;
    for (live, bak, _created) in &record.backups {
        let (live, bak) = (Path::new(live), Path::new(bak));
        if !bak.exists() {
            all_ok = false; // recorded backup is gone — this file can't be restored
            continue;
        }
        match std::fs::read(bak) {
            Ok(bytes) if atomic_write(live, &bytes).is_ok() => {
                let _ = std::fs::remove_file(bak);
            }
            _ => all_ok = false,
        }
    }
    if let Some(dir) = &record.ue4ss_mod_dir {
        if Path::new(dir).exists() && std::fs::remove_dir_all(dir).is_err() {
            all_ok = false;
        }
    }
    all_ok
}

/// Undo the active gore-mod deployment at `game_root`: restore every backup and remove the
/// UE4SS mod. No-op if nothing is deployed.
pub fn undeploy(game_root: &Path) -> Result<Option<DeployRecord>> {
    let rp = record_path(game_root);
    let Ok(bytes) = std::fs::read(&rp) else {
        return Ok(None);
    };
    let record: DeployRecord = serde_json::from_slice(&bytes)?;
    if restore_record(&record) {
        let _ = std::fs::remove_file(&rp);
        Ok(Some(record))
    } else {
        // Keep the record (and the surviving backups) so the restore can be retried.
        Err(ModError::Other(
            "some game files could not be restored (locked or unwritable); backups and the \
             deploy record were kept — close the game and retry undeploy"
                .into(),
        ))
    }
}

/// Back up `live` to `live.gore-bak` if no backup exists yet (preserving the pristine file),
/// register it in `record`, and return the backup path. The backup is the pristine source.
fn backup(live: &Path, record: &mut DeployRecord) -> Result<(PathBuf, bool)> {
    if !live.exists() {
        return Err(ModError::Other(format!("game file not found: {}", live.display())));
    }
    let bak = bak_path(live);
    let created = !bak.exists();
    if created {
        std::fs::copy(live, &bak).map_err(io("creating backup"))?;
    }
    record
        .backups
        .push((live.display().to_string(), bak.display().to_string(), created));
    Ok((bak, created))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".gore-tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(io("writing temp"))?;
    std::fs::rename(&tmp, path).map_err(io("renaming temp"))?;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(io("create dir"))?;
    for entry in std::fs::read_dir(src).map_err(io(&format!("reading {}", src.display())))? {
        let entry = entry.map_err(io("dir entry"))?;
        let ft = entry.file_type().map_err(io("file type"))?;
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else if ft.is_file() {
            std::fs::copy(entry.path(), &to).map_err(io("copy file"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_modgen::gen::OverrideValue;

    #[test]
    fn build_bundle_overrides_loc_audio() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("tone.wav");
        // minimal 16-bit PCM WAV, 1 sample
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36u32 + 2).to_le_bytes());
        w.extend_from_slice(b"WAVE");
        w.extend_from_slice(b"fmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&48000u32.to_le_bytes());
        w.extend_from_slice(&96000u32.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&2u32.to_le_bytes());
        w.extend_from_slice(&0i16.to_le_bytes());
        std::fs::write(&wav, &w).unwrap();

        let mut loc = BTreeMap::new();
        let mut langs = BTreeMap::new();
        langs.insert("german_new".to_string(), "Käse".to_string());
        loc.insert("itfo_cheese".to_string(), langs);

        let spec = BuildSpec {
            meta: ModMeta { name: "MyMod".into(), version: "1.0".into(), author: "me".into() },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ItFo_Apple".into(),
                field: "m_Value".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(500),
            }],
            loc_edits: loc,
            audio: vec![AudioReplacement {
                bank: "SFX.bank".into(),
                sample: "SFX_UI_X".into(),
                wav_path: wav.display().to_string(),
            }],
        };

        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("ue4ss/MyMod/Scripts/main.lua"));
        assert!(bundle.files.contains_key("ue4ss/MyMod/enabled.txt"));
        assert!(bundle.files.contains_key("loc/edits.json"));
        assert!(bundle.files.contains_key("audio/manifest.json"));
        assert!(bundle.files.contains_key("audio/SFX_bank__SFX_UI_X.wav"));
        assert!(bundle.files.contains_key("gore-mod.json"));
        assert_eq!(bundle.manifest.components.len(), 3);

        // round-trip manifest
        let mj = &bundle.files["gore-mod.json"];
        let m: ModManifest = serde_json::from_slice(mj).unwrap();
        assert_eq!(m.mod_meta.name, "MyMod");
    }

    #[test]
    fn empty_name_rejected() {
        let spec = BuildSpec {
            meta: ModMeta { name: "".into(), version: String::new(), author: String::new() },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
        };
        assert!(build_bundle(&spec).is_err());
    }
}
