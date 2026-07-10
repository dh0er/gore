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

pub mod mgr;

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

/// One texture replacement: put `image_path` (a PNG) in place of cooked `asset` (in-game path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureReplacement {
    pub asset: String,      // e.g. "/Game/UI/Textures/Common/T_HardwareCursor"
    pub image_path: String, // a PNG on disk
}

/// One AngelScript module mod: splice (`op = "add"`) or replace (`op = "edit"`) the compiled
/// 1-module mini-cache at `mini_cache` into the precompiled-script cache at deploy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptModule {
    pub op: String,          // "add" | "edit"
    pub module_name: String, // the Modules TMap key (used for "edit"/replace)
    pub mini_cache: String,  // path to the compiled 1-module mini-cache on disk
}

/// One entry in a bundle's `scripts/manifest.json`: `mini` is a bundle-relative path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntry {
    pub op: String,
    pub module: String,
    pub mini: String,
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
    #[serde(default)]
    pub texture: Vec<TextureReplacement>,
    #[serde(default)]
    pub scripts: Vec<ScriptModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    /// A UE4SS Lua mod folder at `path`, deployed to `ue4ss/Mods/<name>`.
    Ue4ssLua {
        name: String,
        path: String,
        /// The `Class.Field` CDO targets this mod overrides (sorted, deduped) — the
        /// mod-manager's conflict-detection contract. `#[serde(default)]` keeps old
        /// manifests parseable; omitted from the JSON when empty to keep byte-noise low.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targets: Vec<String>,
    },
    /// Declarative loc edits at `path` (`{id:{set:text}}`), applied to the .lcache.
    LocPatch { path: String },
    /// Audio patch dir at `path` (manifest.json + wavs), applied to `banks`.
    AudioPatch { path: String, banks: Vec<String> },
    /// Texture patch dir at `path` (manifest.json + pngs); deploy cooks + packs a Zen triplet
    /// into `~mods` for `assets`. Additive — no in-place game-file patch, no `*.gore-bak`.
    TexturePatch { path: String, assets: Vec<String> },
    /// AngelScript mini-caches at `path` (manifest.json + `*.cache`); deploy splices/replaces
    /// them into `PrecompiledScript_Shipping.Cache` in place, with a `*.gore-bak` backup.
    AngelScriptPatch { path: String },
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
        // The `Class.Field` CDO targets this mod sets, for the manager's conflict detection.
        let mut targets: Vec<String> =
            spec.overrides.iter().map(|o| format!("{}.{}", o.class, o.field)).collect();
        targets.sort();
        targets.dedup();
        components.push(Component::Ue4ssLua {
            name: name.clone(),
            path: format!("ue4ss/{name}"),
            targets,
        });
    }

    // loc edits → declarative patch
    if !spec.loc_edits.is_empty() {
        files.insert("loc/edits.json".into(), serde_json::to_vec_pretty(&spec.loc_edits)?);
        components.push(Component::LocPatch { path: "loc/edits.json".into() });
    }

    // audio → manifest + wavs (no game audio, just the replacements)
    if !spec.audio.is_empty() {
        let mut map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (i, a) in spec.audio.iter().enumerate() {
            let wav = std::fs::read(&a.wav_path).map_err(io(&format!("reading wav {}", a.wav_path)))?;
            // Prefix with the index so distinct samples that sanitize to the same name can't
            // collide and overwrite each other.
            let fname = format!("{i}_{}__{}.wav", sanitize(&a.bank), sanitize(&a.sample));
            files.insert(format!("audio/{fname}"), wav);
            map.entry(a.bank.clone()).or_default().insert(a.sample.clone(), format!("audio/{fname}"));
        }
        let banks: Vec<String> = map.keys().cloned().collect();
        files.insert("audio/manifest.json".into(), serde_json::to_vec_pretty(&map)?);
        components.push(Component::AudioPatch { path: "audio".into(), banks });
    }

    // textures → manifest + pngs (source images; cooked+packed at deploy)
    if !spec.texture.is_empty() {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        for (i, t) in spec.texture.iter().enumerate() {
            let png = std::fs::read(&t.image_path)
                .map_err(io(&format!("reading png {}", t.image_path)))?;
            let fname = format!("{i}_{}.png", sanitize(&t.asset));
            files.insert(format!("texture/{fname}"), png);
            map.insert(t.asset.clone(), format!("texture/{fname}"));
        }
        let assets: Vec<String> = map.keys().cloned().collect();
        files.insert("texture/manifest.json".into(), serde_json::to_vec_pretty(&map)?);
        components.push(Component::TexturePatch { path: "texture".into(), assets });
    }

    // scripts → manifest + compiled mini-caches (spliced/replaced at deploy)
    if !spec.scripts.is_empty() {
        let mut entries: Vec<ScriptEntry> = Vec::new();
        for (i, s) in spec.scripts.iter().enumerate() {
            if s.op != "add" && s.op != "edit" {
                return Err(ModError::Other(format!(
                    "invalid script op {:?} for module {:?} (want \"add\" or \"edit\")",
                    s.op, s.module_name
                )));
            }
            let mini = std::fs::read(&s.mini_cache)
                .map_err(io(&format!("reading mini-cache {}", s.mini_cache)))?;
            let mini_rel = format!("scripts/{i}_{}.cache", sanitize(&s.module_name));
            files.insert(mini_rel.clone(), mini);
            entries.push(ScriptEntry { op: s.op.clone(), module: s.module_name.clone(), mini: mini_rel });
        }
        files.insert("scripts/manifest.json".into(), serde_json::to_vec_pretty(&entries)?);
        components.push(Component::AngelScriptPatch { path: "scripts".into() });
    }

    let manifest = ModManifest { format: 1, mod_meta: spec.meta.clone(), components };
    files.insert("gore-mod.json".into(), serde_json::to_vec_pretty(&manifest)?);
    Ok(Bundle { files, manifest })
}

/// Write a built bundle's files under `dir` (creating parent dirs).
pub fn write_bundle(dir: &Path, bundle: &Bundle) -> Result<()> {
    // Rebuild into a clean directory: a prior build of the same bundle may have left files that are
    // no longer part of it (e.g. a component the user removed). Deploy copies some component dirs
    // (like `ue4ss/<name>`) wholesale, so stale leftovers would still be shipped/deployed. Clear the
    // destination first so it holds exactly this bundle.
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .map_err(io(&format!("clearing bundle dir {}", dir.display())))?;
    }
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
    pub script_cache: PathBuf,
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
            // Deterministic when several caches exist: pick the most recently modified (the active
            // one). Sort by PATH first, then stably by mtime, then take the last — identical to
            // gore-loc's locator, so deploy patches/backs up the SAME cache the catalog was
            // extracted from even when mtimes tie or metadata can't be read.
            matches.sort();
            matches.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
            matches.pop()
        })
    };
    GamePaths {
        ue4ss_mods: g1r.join("Binaries").join("Win64").join("ue4ss").join("Mods"),
        fmod_desktop: g1r.join("Content").join("FMOD").join("Desktop"),
        lcache,
        script_cache: g1r.join("Script").join("PrecompiledScript_Shipping.Cache"),
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
    /// Previous-deployment UE4SS mod dirs (different name from the new one) that couldn't be
    /// removed at deploy time (locked/permissions). Tracked so undeploy still cleans them up;
    /// otherwise their enabled scripts would linger. `#[serde(default)]` keeps old records loadable.
    #[serde(default)]
    pub stale_ue4ss_dirs: Vec<String>,
    /// live_path → hash of the modded bytes this deploy wrote there. On undeploy/rollback, if the
    /// current live file no longer matches, the game was updated/verified externally (e.g. Steam),
    /// so the recorded `*.gore-bak` is stale and restoring it would downgrade the newer asset —
    /// the restore is skipped instead. `#[serde(default)]` keeps old records loadable.
    #[serde(default)]
    pub deployed_hashes: BTreeMap<String, String>,
    /// Absolute dst paths of additive texture-override Zen triplet files written into `~mods`.
    /// These are pure additions (no in-place game-file patch, no `*.gore-bak`); undeploy simply
    /// deletes them. `#[serde(default)]` keeps old records loadable.
    #[serde(default)]
    pub texture_triplets: Vec<String>,
    /// Who owns this deployment: `""` = legacy/studio single-mod deploy, `"manager"` = the
    /// multi-mod manager. `#[serde(default)]` keeps old records loadable (as `""`).
    #[serde(default)]
    pub owner: String,
    /// Manager only: the loadout snapshot this deployment realized (library ids + enabled),
    /// so status/apply can diff the deployed state against the current loadout.
    #[serde(default)]
    pub loadout: Vec<crate::mgr::LoadoutEntry>,
    /// Manager only: ALL deployed UE4SS mod dirs (absolute). A manager deployment
    /// (`owner == "manager"`) records EVERY dir here and leaves `ue4ss_mod_dir` = None; a studio
    /// deployment (`owner == ""`) instead puts its single dir in the legacy `ue4ss_mod_dir` and
    /// leaves this empty. Undeploy removes each entry with the same semantics as `ue4ss_mod_dir`.
    #[serde(default)]
    pub ue4ss_mod_dirs: Vec<String>,
    /// Absolute dst paths of manager-installed pak/triplet files (loose paks, foreign
    /// triplets) in `~mods`. Pure additions like `texture_triplets`; undeploy deletes them.
    #[serde(default)]
    pub managed_paks: Vec<String>,
    /// Manager only: mod id → the library entry's content [`fingerprint`] at deploy time. Lets
    /// status detect a same-id UPDATE (a re-import that changed a mod's components/bytes but not
    /// its id): if a library mod's current fingerprint differs from the one recorded here, the
    /// deployed bytes are stale even though the loadout id set still matches. Old records parse as
    /// empty (`#[serde(default)]`), where a missing fingerprint reads as changed (re-apply needed).
    ///
    /// [`fingerprint`]: crate::mgr::model::ModEntryMeta::fingerprint
    #[serde(default)]
    pub deployed_fingerprints: std::collections::BTreeMap<String, String>,
}

/// Stable content fingerprint for drift detection (not cryptographic — only distinguishes our own
/// deployed bytes from a later external overwrite). FNV-1a 64-bit: a fixed algorithm whose output
/// never changes across Rust/tool versions, unlike `DefaultHasher` (SipHash), so a record written
/// by one build is read back consistently by a later one.
pub(crate) fn content_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
    }
    format!("{h:016x}")
}

/// Short stable hash of an arbitrary string for disambiguating filenames. FNV-1a 64-bit (same
/// fixed algorithm as [`content_hash`]) truncated to 8 hex chars: distinct mod names that sanitize
/// to the same stem (e.g. `A+B` vs `A B` -> `A_B`) get distinct triplet names because this hashes
/// the ORIGINAL (unsanitized) name.
fn name_hash(s: &str) -> String {
    content_hash(s.as_bytes())[..8].to_string()
}

/// Whether `live` should be restored from its backup: true unless we recorded what we deployed
/// there and the current file no longer matches it (external update — restoring would downgrade).
fn safe_to_restore(live: &str, deployed_hashes: &BTreeMap<String, String>) -> bool {
    match deployed_hashes.get(live) {
        Some(expected) => match std::fs::read(Path::new(live)) {
            Ok(cur) => &content_hash(&cur) == expected,
            Err(_) => true, // can't read current file; fall back to the normal restore attempt
        },
        None => true, // no drift info recorded — restore as before
    }
}

const RECORD_NAME: &str = "gore-mod.deployed.json";

/// This install's FMOD bank encryption key: the one gore-dump recovered into `gore_fmod_key.json`
/// (written to `Binaries/Win64`) if present and valid, else the known [`gore_fmod::GOTHIC_STUDIO_KEY`]
/// constant. The key stays constant until a game patch changes it; a user who re-dumps after such a
/// patch can then deploy audio without the build/deploy path being stuck on the old constant.
pub(crate) fn resolve_fmod_key(gp: &GamePaths) -> Vec<u8> {
    #[derive(Deserialize)]
    struct FmodKeyFile {
        #[serde(default)]
        found: bool,
        #[serde(default)]
        encryption_key: String,
    }
    // gp.ue4ss_mods == <...>/Binaries/Win64/ue4ss/Mods, so its grandparent is Binaries/Win64.
    if let Some(win64) = gp.ue4ss_mods.parent().and_then(Path::parent) {
        let key_file = win64.join("gore_fmod_key.json");
        if let Ok(bytes) = std::fs::read(&key_file) {
            if let Ok(k) = serde_json::from_slice::<FmodKeyFile>(&bytes) {
                if k.found && !k.encryption_key.is_empty() {
                    return k.encryption_key.into_bytes();
                }
            }
        }
    }
    gore_fmod::GOTHIC_STUDIO_KEY.to_vec()
}

/// Absolutize the game root so every path derived from it (live files, `*.gore-bak`, UE4SS dirs)
/// and persisted in the deploy record is absolute. Otherwise a deploy from the install dir with a
/// relative root (e.g. `--game .`) would serialize relative paths, and a later undeploy from a
/// different working directory would resolve them against the wrong tree.
fn abs_root(root: &Path) -> PathBuf {
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
}

/// Canonical install root for the deploy record. `resolve_game_paths` accepts both the install
/// dir and its `G1R` child, so normalize to the install dir (the parent of `G1R`) — otherwise a
/// deploy via `.../G1R` and an undeploy via the Steam-detected parent would use different record
/// paths, leaving the mod silently un-undeployable.
fn record_root(root: &Path) -> PathBuf {
    if root.file_name().is_some_and(|n| n == "G1R") {
        root.parent().map(Path::to_path_buf).unwrap_or_else(|| root.to_path_buf())
    } else {
        root.to_path_buf()
    }
}

fn record_path(root: &Path) -> PathBuf {
    record_root(root).join(RECORD_NAME)
}

/// A fully-prepared deployment: everything to write, computed in memory so the failure-prone
/// work happens BEFORE the game is touched. The single-bundle [`deploy`] fills 0/1 `ue4ss_dirs`
/// entries and no `managed_paks`; the multi-mod manager composes several of each.
#[derive(Debug, Default)]
pub(crate) struct DeployPlan {
    /// (source dir on disk, dest under ue4ss/Mods) — each installed via staged swap.
    pub(crate) ue4ss_dirs: Vec<(PathBuf, PathBuf)>,
    /// (live game file, new contents)
    pub(crate) writes: Vec<(PathBuf, Vec<u8>)>,
    /// Live files whose preserved `*.gore-bak` is stale because the file drifted (game updated)
    /// since we deployed: stage must drop that backup so it re-snapshots the current pristine.
    pub(crate) refresh_baks: Vec<PathBuf>,
    /// Additive texture-override Zen triplet files to copy: (src triplet file in temp, dst in
    /// `~mods`). No backup — undeploy deletes the dst.
    pub(crate) texture_triplets: Vec<(PathBuf, PathBuf)>,
    /// Manager-installed pak/triplet files to copy: (src in the mod library, dst in `~mods`).
    /// Pure additions like `texture_triplets` (no backup; undeploy deletes the dst), but the
    /// srcs are durable library files, never temp dirs to clean up.
    pub(crate) managed_paks: Vec<(PathBuf, PathBuf)>,
}

impl DeployPlan {
    /// Dst paths of every UE4SS mod dir this plan installs.
    fn ue4ss_dsts(&self) -> Vec<String> {
        self.ue4ss_dirs.iter().map(|(_, dst)| dst.display().to_string()).collect()
    }
    /// Dst paths of every ADDITIVE `~mods` file this plan installs — texture triplets AND manager
    /// paks together. Prev-vs-new reconciliation (pre-seed + retire) must treat these as one set:
    /// a manager deploy mirrors its `managed_paks` into the legacy `texture_triplets` record field,
    /// so a prev entry recorded under either field must be considered "still installed" if THIS
    /// plan re-creates that path under either kind — otherwise retire would delete a file this
    /// deploy just wrote.
    fn additive_dsts(&self) -> Vec<String> {
        self.texture_triplets
            .iter()
            .chain(self.managed_paks.iter())
            .map(|(_, dst)| dst.display().to_string())
            .collect()
    }
}

/// First dst path that two entries of `plan` share (UE4SS dirs + texture triplets + manager paks),
/// compared with `same_path` semantics; `None` if all dsts are distinct. Used to reject a
/// self-colliding plan before any game write.
pub(crate) fn first_duplicate_dst(plan: &DeployPlan) -> Option<String> {
    let dsts: Vec<String> = plan
        .ue4ss_dirs
        .iter()
        .chain(plan.texture_triplets.iter())
        .chain(plan.managed_paks.iter())
        .map(|(_, dst)| dst.display().to_string())
        .collect();
    for (i, d) in dsts.iter().enumerate() {
        if dsts[..i].iter().any(|prev| same_path_s(prev, d)) {
            return Some(d.clone());
        }
    }
    None
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
    // Absolutize up front so every persisted path (record location, backups, UE4SS dirs) is
    // absolute and resolvable from any later working directory.
    let game_root = &abs_root(game_root);
    let gp = resolve_game_paths(game_root);

    // The previous deployment's record — used both to detect externally-updated (drifted) files
    // during prepare and to fold its leftovers during commit.
    let prev = read_record(game_root);

    // A manager-owned deployment composes MULTIPLE mods; letting the single-mod deploy fold it
    // away would silently retire the whole loadout. Refuse before any prepare/commit work.
    if prev.as_ref().is_some_and(|p| p.owner == "manager") {
        return Err(ModError::Other(
            "manager loadout active — use gore mod-manager to change deployments \
             (undeploy there first)"
                .into(),
        ));
    }

    // PHASE 1 — prepare (no game writes). The previous deployment is left intact if this fails.
    let plan = prepare(bundle_dir, &manifest, &gp, prev.as_ref())?;

    // PHASE 2 — commit.
    let record = DeployRecord { mod_name: manifest.mod_meta.name.clone(), ..Default::default() };
    commit_plan(&gp, game_root, plan, record, prev)
}

/// Commit a prepared [`DeployPlan`]: stage backups, persist the record crash-safely BEFORE any
/// live write, apply the writes, then retire the previous deployment's leftover footprint.
/// `record` is the caller-built record for THIS deployment (identity fields like `mod_name`/
/// `owner`/`loadout` already set); `prev` is the record the plan was prepared against, whose
/// not-overwritten leftovers/stale dirs/additive files are folded in here. On failure the game
/// is rolled back to the exact prior state and the previous record file is restored.
pub(crate) fn commit_plan(
    _gp: &GamePaths,
    abs_root: &Path,
    plan: DeployPlan,
    mut record: DeployRecord,
    prev: Option<DeployRecord>,
) -> Result<DeployRecord> {
    let game_root = abs_root;

    // Reject a plan that would write the SAME destination twice (across UE4SS dirs, texture
    // triplets, and manager paks). Two components/mods targeting one path would race in
    // `apply_writes`, and the undo/retire bookkeeping (which keys off the dst) would double-track
    // or mis-restore it. Compare with `same_path` semantics so `\\?\C:\..` vs the plain form (and
    // Windows case differences) still count as the same target. This runs BEFORE `stage`, so on a
    // duplicate the game is untouched.
    if let Some(dup) = first_duplicate_dst(&plan) {
        return Err(ModError::Other(format!("duplicate deploy target: {dup}")));
    }

    let prev_record_bytes = std::fs::read(record_path(game_root)).ok();

    // `undo` captures the exact pre-deploy state for an in-process rollback;
    // the record is persisted BEFORE any live write so even a crash mid-write is recoverable.
    let mut undo = Undo::default();

    // (a) Stage: snapshot prior bytes + create every *.gore-bak, and note the intended UE4SS
    //     target — but do NOT write any live game file yet.
    if let Err(e) = stage(&plan, &mut record, &mut undo) {
        undo.rollback();
        return Err(e);
    }

    // (b) Fold the previous mod's not-overwritten loose files into the record, then persist the
    //     record BEFORE touching live files. A crash after this point is recoverable via
    //     undeploy; a write failure here rolls back and restores the previous record.
    let leftovers: Vec<(String, String, bool)> = prev
        .as_ref()
        .map(|p| {
            p.backups
                .iter()
                .filter(|(live, _, _)| !plan.writes.iter().any(|(pp, _)| same_path(pp, live)))
                .map(|(l, b, _)| (l.clone(), b.clone(), false))
                .collect()
        })
        .unwrap_or_default();
    record.backups.extend(leftovers.iter().cloned());

    // Carry the previous deploy's drift hashes for the leftover (not-overwritten) files, so undeploy
    // can still detect an external update of those files and skip a stale-backup restore.
    if let Some(p) = prev.as_ref() {
        for (live, _, _) in &leftovers {
            if let Some(h) = p.deployed_hashes.get(live) {
                record.deployed_hashes.insert(live.clone(), h.clone());
            }
        }
    }

    // Crash-safety: if the previous deployment used DIFFERENT-named UE4SS dir(s), record them as
    // stale BEFORE persisting. apply_writes/retire_leftovers haven't removed them yet, so a crash
    // in this window would otherwise leave them orphaned-and-active with no record to clean them
    // up. retire_leftovers prunes any it later removes successfully.
    if let Some(prev) = prev.as_ref() {
        let new_dirs = plan.ue4ss_dsts();
        for d in prev
            .ue4ss_mod_dir
            .iter()
            .chain(prev.stale_ue4ss_dirs.iter())
            .chain(prev.ue4ss_mod_dirs.iter())
        {
            // `same_path`: records hold canonicalized (`\\?\`-prefixed) paths but the plan may hold
            // the plain form of the same dir — a raw compare would wrongly mark a still-deployed dir
            // stale and retire could then delete a dir this deploy just installed.
            if !contains_same_path(&new_dirs, d) && !contains_same_path(&record.stale_ue4ss_dirs, d) {
                record.stale_ue4ss_dirs.push(d.clone());
            }
        }
    }

    // Same for the previous deploy's additive ~mods texture triplets: any not re-created by this
    // deploy must be retired. Pre-seed them into the record BEFORE persisting (crash-safety) so a
    // crash mid-retire still lets undeploy remove them; retire_leftovers deletes + prunes the ones
    // it cleans. Without this, redeploying (esp. a different mod name or a bundle with no texture
    // component) would leave the old triplet mounted in ~mods with no record to undeploy it.
    if let Some(prev) = prev.as_ref() {
        // Compare against the UNION of this plan's additive dsts (triplets + manager paks): a
        // manager deploy mirrors managed_paks into the legacy `texture_triplets` field, so a prev
        // entry re-created by THIS plan as EITHER kind must be kept, not retired. `same_path` so a
        // canonicalized prev path matches the plan's plain form of the same file.
        let new_additive = plan.additive_dsts();
        for t in &prev.texture_triplets {
            if !contains_same_path(&new_additive, t) && !contains_same_path(&record.texture_triplets, t) {
                record.texture_triplets.push(t.clone());
            }
        }
        for p in &prev.managed_paks {
            if !contains_same_path(&new_additive, p) && !contains_same_path(&record.managed_paks, p) {
                record.managed_paks.push(p.clone());
            }
        }
    }

    // Old-binary compatibility: a pre-v2 build deserializes a manager record but silently DROPS the
    // v2-only `managed_paks`/`ue4ss_mod_dirs` fields (serde unknown-field tolerance), so ITS
    // undeploy would never remove them — leaving manager paks/dirs mounted forever. Mirror the
    // manager footprint into the legacy fields an old binary DOES read, which have identical removal
    // semantics: manager paks → `texture_triplets` (delete file), manager UE4SS dirs →
    // `stale_ue4ss_dirs` (remove_dir_all). A v2 binary now sees each path in BOTH the new and the
    // legacy field; every removal site (`retire_leftovers`, `restore_record`) guards with `!exists()`
    // so the second pass is a harmless no-op, never a double-error. Studio deploys have no such
    // fields, so this is manager-only. Done BEFORE the record is persisted so even a pre-v2 undeploy
    // after a crash cleans up. (`record.managed_paks`/`ue4ss_mod_dirs` were filled by `stage`.)
    if record.owner == "manager" {
        for p in record.managed_paks.clone() {
            if !contains_same_path(&record.texture_triplets, &p) {
                record.texture_triplets.push(p);
            }
        }
        for d in record.ue4ss_mod_dirs.clone() {
            if !contains_same_path(&record.stale_ue4ss_dirs, &d) {
                record.stale_ue4ss_dirs.push(d);
            }
        }
    }

    if let Err(e) = write_record_file(game_root, &record) {
        undo.rollback();
        restore_record_file(game_root, prev_record_bytes.as_deref());
        return Err(e);
    }

    // (c) Apply: write the live files and install the UE4SS mod. On failure restore the exact
    //     prior state and the previous record.
    if let Err(e) = apply_writes(&plan, &mut undo) {
        undo.rollback();
        restore_record_file(game_root, prev_record_bytes.as_deref());
        return Err(e);
    }

    // (c2) The live files now actually hold our content, so the drift hashes are valid — record
    //      them and persist. They were intentionally OMITTED from the pre-write record (b): had we
    //      stored them earlier, a crash between the record write and the live writes would leave
    //      the old content on disk with the new hashes recorded, and undeploy would mis-read that
    //      as an external update — skipping the restore and dropping the backup. With no hash, a
    //      crash in that window instead falls back to a plain pristine restore.
    for (live, bytes) in &plan.writes {
        record.deployed_hashes.insert(live.display().to_string(), content_hash(bytes));
    }
    // This write must be durable: without the hashes, a later Steam update couldn't be detected
    // and undeploy could restore a stale backup over an updated asset. The undo is still live
    // here, so on failure roll the whole deploy back rather than returning a half-recorded success.
    if let Err(e) = write_record_file(game_root, &record) {
        undo.rollback();
        restore_record_file(game_root, prev_record_bytes.as_deref());
        return Err(e);
    }

    // (d) committed — drop the kept-aside previous UE4SS mod(s), then retire the previous mod's
    //     footprint now (best-effort), pruning retired leftovers from the record.
    let aside_failed = undo.discard();
    let (mut changed, pending_deletes) = retire_leftovers(&leftovers, prev.as_ref(), &plan, &mut record);
    for old in aside_failed {
        // A moved-aside previous mod couldn't be removed — track it so undeploy cleans it up.
        let s = old.display().to_string();
        if !record.stale_ue4ss_dirs.contains(&s) {
            record.stale_ue4ss_dirs.push(s);
            changed = true;
        }
    }
    if changed {
        // Persist the pruned record BEFORE deleting the retired backups. Only once that write
        // succeeds is it safe to delete them — otherwise a failed rewrite would leave the on-disk
        // record referencing deleted backups, wedging a later undeploy.
        if write_record_file(game_root, &record).is_ok() {
            let mut readded = false;
            for (live, bak, hash) in pending_deletes {
                let p = Path::new(&bak);
                if std::fs::remove_file(p).is_ok() || !p.exists() {
                    continue;
                }
                // The backup is locked/read-only and couldn't be deleted. Re-track it (with its
                // drift hash) so it isn't orphaned without a record — else a future deploy could
                // treat this stale backup as pristine and downgrade an updated game file.
                record.backups.push((live.clone(), bak.clone(), false));
                if let Some(h) = hash {
                    record.deployed_hashes.insert(live, h);
                }
                readded = true;
            }
            if readded {
                // Best-effort re-persist of the re-tracked backups. The deploy already succeeded
                // and its record (sans these now-deleted-or-locked backups) is durable; a failure
                // here only means a locked stale backup stays untracked until the next deploy/
                // undeploy rewrites the record — not a corrupted or lost deployment — so swallowing
                // it (rather than failing a committed deploy) is tolerable. Log for diagnosis.
                if let Err(e) = write_record_file(game_root, &record) {
                    eprintln!("gore-mod: could not re-persist record after retiring backups: {e}");
                }
            }
        }
    }
    Ok(record)
}

fn write_record_file(game_root: &Path, record: &DeployRecord) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(record)?;
    // Write via temp + rename so a crash mid-write can't truncate an existing record (which
    // undeploy needs to parse to restore game files / clean up backups).
    atomic_write(&record_path(game_root), &bytes)
}

/// Restore the deploy record file to its pre-deploy contents on rollback (or remove it if there
/// was none), so the on-disk record matches the rolled-back game state.
fn restore_record_file(game_root: &Path, prev_bytes: Option<&[u8]>) {
    match prev_bytes {
        Some(b) => {
            let _ = atomic_write(&record_path(game_root), b);
        }
        None => {
            let _ = std::fs::remove_file(record_path(game_root));
        }
    }
}

/// Captures the exact pre-deploy state so a failed deploy can restore it precisely, rather than
/// only reverting to the game-pristine `*.gore-bak`.
#[derive(Default)]
struct Undo {
    /// (live, prior bytes) — write back on rollback to restore the exact pre-deploy content.
    files: Vec<(PathBuf, Vec<u8>)>,
    /// `*.gore-bak` files THIS deploy created — remove on rollback (adopted ones are kept).
    created_baks: Vec<PathBuf>,
    /// (bak path, prior bytes) for a STALE backup deleted during drift-refresh — write back on
    /// rollback so the restored previous record doesn't point at a now-missing backup.
    removed_baks: Vec<(PathBuf, Vec<u8>)>,
    /// (old-aside dir, dst) — previous UE4SS mods moved aside: restore on rollback, drop on success.
    ue4ss_old: Vec<(PathBuf, PathBuf)>,
    /// UE4SS mods installed where there was none — remove on rollback.
    ue4ss_fresh: Vec<PathBuf>,
    /// Additive `~mods` files copied by this deploy (texture triplets AND manager paks, in copy
    /// order), each with its PRE-OVERWRITE bytes: `None` if the file did not exist before this
    /// deploy (delete on rollback), `Some(bytes)` if it did (a same-named redeploy overwrote a
    /// currently-active file — restore the OLD bytes on rollback so a later-step failure doesn't
    /// leave the prior active deployment missing/inconsistent). Unwound in REVERSE (LIFO) so
    /// delete/restore pairs undo in the mirror order of the copies.
    texture_files: Vec<(PathBuf, Option<Vec<u8>>)>,
}

impl Undo {
    fn rollback(self) {
        for (live, bytes) in &self.files {
            let _ = atomic_write(live, bytes);
        }
        for bak in &self.created_baks {
            let _ = std::fs::remove_file(bak);
        }
        // Restore any stale backup we deleted for a drift-refresh (after removing the new one above,
        // since they share the same path) so the rolled-back previous record still resolves.
        for (bak, bytes) in &self.removed_baks {
            let _ = atomic_write(bak, bytes);
        }
        for (old, dst) in &self.ue4ss_old {
            let _ = std::fs::remove_dir_all(dst);
            let _ = std::fs::rename(old, dst);
        }
        for dst in &self.ue4ss_fresh {
            let _ = std::fs::remove_dir_all(dst);
        }
        // Restore each additive `~mods` file we copied (triplets + manager paks): put back the
        // bytes it had before this deploy overwrote them (a same-named redeploy), or delete it if
        // it's a fresh addition. Unwound in REVERSE insertion order (LIFO) so if any two copies
        // touched the same dst their delete/restore pairs undo in the exact reverse of how they
        // were applied. Best-effort.
        for (f, prior) in self.texture_files.iter().rev() {
            match prior {
                Some(bytes) => {
                    let _ = atomic_write(f, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
    }

    /// Commit: drop the previous UE4SS mods that were moved aside (`<mod>.gore-old`). Returns the
    /// dirs that couldn't be removed (locked/permissions/AV) so the caller can track them — left
    /// untracked they would keep loading under `ue4ss/Mods` with no record for undeploy to clean up.
    fn discard(self) -> Vec<PathBuf> {
        let mut failed = Vec::new();
        for (old, _) in &self.ue4ss_old {
            if std::fs::remove_dir_all(old).is_err() && old.exists() {
                // Best-effort: stop it loading meanwhile by removing its enable flag.
                let _ = std::fs::remove_file(old.join("enabled.txt"));
                failed.push(old.clone());
            }
        }
        failed
    }
}

/// Build everything to write, in memory. Any error here leaves the game untouched.
fn prepare(
    bundle_dir: &Path,
    manifest: &ModManifest,
    gp: &GamePaths,
    prev: Option<&DeployRecord>,
) -> Result<DeployPlan> {
    let mut plan = DeployPlan::default();
    for (comp_idx, comp) in manifest.components.iter().enumerate() {
        match comp {
            Component::Ue4ssLua { name, path, .. } => {
                // The manifest may come from an untrusted bundle: reject names/paths that could
                // escape the bundle source or the UE4SS Mods directory.
                if !is_safe_mod_name(name) || !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!(
                        "unsafe ue4ss component in manifest: name={name:?} path={path:?}"
                    )));
                }
                // A single bundle installs at most ONE UE4SS mod — a later Ue4ssLua component
                // replaces an earlier one (same last-wins semantics as the old Option field).
                plan.ue4ss_dirs = vec![(bundle_dir.join(path), gp.ue4ss_mods.join(name))];
            }
            Component::LocPatch { path } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe loc patch path: {path:?}")));
                }
                let lcache = gp.lcache.clone().ok_or_else(|| {
                    ModError::Other("no AlkimiaLocalization .lcache found in game".into())
                })?;
                let (pristine, drifted) = read_pristine(&lcache, prev)?;
                if drifted {
                    plan.refresh_baks.push(lcache.clone());
                }
                let edits: BTreeMap<String, BTreeMap<String, String>> =
                    serde_json::from_slice(&std::fs::read(bundle_dir.join(path)).map_err(io("reading edits.json"))?)?;
                let mut lc = gore_loc::loc::Lcache::decode(&pristine)?;
                for (id, langs) in &edits {
                    for (set, text) in langs {
                        // Best-effort: an id/set absent from THIS install's .lcache (e.g. a
                        // shared mod built against a different game version) is skipped rather
                        // than aborting the entire deploy.
                        let _ = lc.set_value(id, set, text);
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
                // Use this install's recovered FMOD bank key if gore-dump left a gore_fmod_key.json,
                // so users whose key changed after a game patch can still deploy audio; else the
                // known constant.
                let fmod_key = resolve_fmod_key(gp);
                for (bank, samples) in &map {
                    if !is_safe_filename(bank) {
                        return Err(ModError::Other(format!("unsafe bank name: {bank:?}")));
                    }
                    let bank_path = gp.fmod_desktop.join(bank);
                    let (pristine, drifted) = read_pristine(&bank_path, prev)?;
                    if drifted {
                        plan.refresh_baks.push(bank_path.clone());
                    }
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
                    let new_bank = gore_fmod::replace_samples(&pristine, &fmod_key, repl)
                        .map_err(ModError::Fmod)?;
                    plan.writes.push((bank_path, new_bank));
                }
            }
            Component::TexturePatch { path, assets: _ } => {
                let triplets = prepare_texture_component(
                    bundle_dir,
                    path,
                    &manifest.mod_meta.name,
                    comp_idx,
                    gp,
                )?;
                plan.texture_triplets.extend(triplets);
            }
            Component::AngelScriptPatch { path } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe script patch path: {path:?}")));
                }
                let entries: Vec<ScriptEntry> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path).join("manifest.json"))
                        .map_err(io("reading script manifest"))?,
                )?;
                let cache_path = gp.script_cache.clone();
                if !cache_path.exists() {
                    return Err(ModError::Other(format!(
                        "script cache not found: {} — is the game path correct?",
                        cache_path.display()
                    )));
                }
                let (pristine, drifted) = read_pristine(&cache_path, prev)?;
                if drifted {
                    plan.refresh_baks.push(cache_path.clone());
                }
                let mut running = pristine;
                for e in &entries {
                    if !is_safe_rel_path(&e.mini) {
                        return Err(ModError::Other(format!("unsafe mini path: {:?}", e.mini)));
                    }
                    let mini = std::fs::read(bundle_dir.join(&e.mini))
                        .map_err(io("reading mini-cache"))?;
                    running = match e.op.as_str() {
                        "add" => gore_as::cache::splice::splice_auto(&running, &mini)
                            .map_err(|err| ModError::Other(format!("splice {}: {err}", e.module)))?,
                        "edit" => gore_as::cache::splice::replace_module(&running, &mini, &e.module)
                            .map_err(|err| ModError::Other(format!("replace {}: {err}", e.module)))?,
                        other => return Err(ModError::Other(format!(
                            "invalid script op {other:?} for module {:?}", e.module
                        ))),
                    };
                }
                plan.writes.push((cache_path, running));
            }
        }
    }
    Ok(plan)
}

/// Prepare ONE texture component: cook each PNG in the patch dir at `path` (bundle-relative)
/// over its original cooked asset and pack the result into a Zen triplet named
/// `zzz_{sanitize(mod_name)}_{name_hash(mod_name)}_{comp_idx}_tex_P`. Returns the (src, dst)
/// triplet file pairs to copy into `~mods` — purely in-memory/temp work, no game writes.
fn prepare_texture_component(
    bundle_dir: &Path,
    path: &str,
    mod_name: &str,
    comp_idx: usize,
    gp: &GamePaths,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    if !is_safe_rel_path(path) {
        return Err(ModError::Other(format!("unsafe texture patch path: {path:?}")));
    }
    let map: BTreeMap<String, String> = serde_json::from_slice(
        &std::fs::read(bundle_dir.join(path).join("manifest.json"))
            .map_err(io("reading texture manifest"))?)?;
    // game install dir: ue4ss_mods == <root>/G1R/Binaries/Win64/ue4ss/Mods -> 5 up.
    let game_dir = gp.ue4ss_mods.ancestors().nth(5)
        .ok_or_else(|| ModError::Other("cannot derive game dir from paths".into()))?
        .to_path_buf();
    let utoc = gore_tex::paths::main_container(&game_dir)
        .map_err(|e| ModError::Other(format!("container: {e}")))?;
    let usmap = gore_tex::paths::usmap(&game_dir)
        .map_err(|e| ModError::Other(format!("usmap: {e}")))?;
    let usmap_bytes = std::fs::read(&usmap).map_err(io("reading usmap"))?;
    // Only use the cached index if it's current for this game build; a stale index
    // (game patched, .usmap/build_id changed) would map paths to outdated package
    // ids and cook the wrong texture. If stale/absent, fall back to a name scan.
    let index = gore_tex::index::TextureIndex::load_current(
        &gore_tex::paths::texture_index_path(),
        &gore_tex::index::build_id_for(&utoc, &usmap),
    );
    // Scope temp dirs by component index too (not just pid): a bundle with >1
    // TexturePatch must not have a later component's `remove_dir_all` wipe an earlier
    // one's cooked tree / packed triplet (whose src paths are already queued in
    // `plan.texture_triplets`).
    let cook_dir = std::env::temp_dir()
        .join(format!("gore-mod-tex-cook-{}-{}", std::process::id(), comp_idx));
    let _ = std::fs::remove_dir_all(&cook_dir);
    for (asset, png_rel) in &map {
        if !is_safe_rel_path(png_rel) {
            return Err(ModError::Other(format!("unsafe png path: {png_rel:?}")));
        }
        let leaf = asset.rsplit('/').next().unwrap_or(asset);
        // Map the UE mount root to its physical content path. Non-/Game
        // assets (e.g. /Engine/...) must NOT be forced under G1R/Content
        // or the override lands at the wrong virtual path and silently
        // does nothing; unknown roots (plugins) are rejected.
        let rel = gore_tex::paths::content_mount_rel(asset).ok_or_else(|| {
            ModError::Other(format!(
                "unsupported asset mount root (only /Game and /Engine): {asset}"
            ))
        })?;
        if !is_safe_rel_path(&rel) {
            return Err(ModError::Other(format!("unsafe asset path: {asset:?}")));
        }
        let dest_dir = cook_dir.join(std::path::Path::new(&rel).parent()
            .ok_or_else(|| ModError::Other(format!("bad asset path {asset}")))?);
        std::fs::create_dir_all(&dest_dir).map_err(io("mkdir cook dir"))?;
        // Unique per-asset temp dir so concurrent deploys don't clobber each other.
        let tmp_orig = gore_tex::paths::unique_temp_dir("gore-mod-tex-orig")
            .map_err(io("mkdir orig"))?;
        let orig_uasset = match index.as_ref().and_then(|i| i.entries.get(asset)) {
            Some(&pid) => gore_tex::container::unpack_asset_by_id(&utoc, &usmap, pid, leaf, &tmp_orig),
            None => gore_tex::container::unpack_asset(&utoc, &usmap, asset, &tmp_orig),
        }.map_err(|e| ModError::Other(format!("unpack {asset}: {e}")))?;
        let ua = std::fs::read(&orig_uasset).map_err(io("read uasset"))?;
        let ue = std::fs::read(orig_uasset.with_extension("uexp")).map_err(io("read uexp"))?;
        let ub = gore_tex::paths::read_optional(&orig_uasset.with_extension("ubulk"))
            .map_err(io("read ubulk"))?;
        let img = image::open(bundle_dir.join(png_rel))
            .map_err(|e| ModError::Other(format!("png {png_rel}: {e}")))?.to_rgba8();
        let (w, h) = (img.width(), img.height());
        let info = gore_tex::decode::parse(&ua, &ue, &ub, &usmap_bytes)
            .map_err(|e| ModError::Other(format!("parse {asset}: {e}")))?;
        // Unified entry: encodes mips (regular) or re-tiles (virtual
        // texture) internally based on the original's shape.
        let (na, ne, nb) = gore_tex::texdata::replace_texture_image(
            &ua, &ue, &ub, img.as_raw(), w, h, &info.format,
        )
            .map_err(|e| ModError::Other(format!("replace {asset}: {e}")))?;
        std::fs::write(dest_dir.join(format!("{leaf}.uasset")), &na).map_err(io("write uasset"))?;
        std::fs::write(dest_dir.join(format!("{leaf}.uexp")), &ne).map_err(io("write uexp"))?;
        if !nb.is_empty() {
            std::fs::write(dest_dir.join(format!("{leaf}.ubulk")), &nb).map_err(io("write ubulk"))?;
        }
        // The unpacked original is consumed (rewritten into cook_dir); drop its unique
        // temp dir now so a many-texture mod doesn't leak one multi-MB dir per asset.
        let _ = std::fs::remove_dir_all(&tmp_orig);
    }
    // Triplet name must be unique across DISTINCT mods (so one mod's mounted pak can't
    // be clobbered by another whose name sanitizes to the same stem) AND across multiple
    // texture components WITHIN this bundle. Append a stable hash of the original
    // (unsanitized) mod name for the former and the component index for the latter.
    let triplet_name = format!(
        "zzz_{}_{}_{}_tex_P",
        sanitize(mod_name),
        name_hash(mod_name),
        comp_idx
    );
    let pack_out = std::env::temp_dir()
        .join(format!("gore-mod-tex-pack-{}-{}", std::process::id(), comp_idx));
    let _ = std::fs::remove_dir_all(&pack_out);
    std::fs::create_dir_all(&pack_out).map_err(io("mkdir pack"))?;
    let triplet = gore_tex::container::repack_to_zen(&cook_dir, &triplet_name, &pack_out, &game_dir, false)
        .map_err(|e| ModError::Other(format!("pack: {e}")))?;
    // The cooked tree is now packed into the triplet; drop it. (cook_dir/pack_out are
    // pid+component scoped and cleared at the next deploy, so they don't leak per-deploy;
    // the triplet in pack_out is consumed by apply_writes copying it into ~mods.)
    let _ = std::fs::remove_dir_all(&cook_dir);
    let mods_dir = game_dir.join("G1R").join("Content").join("Paks").join("~mods");
    let mut out = Vec::new();
    for src in triplet {
        let dst = mods_dir.join(src.file_name()
            .ok_or_else(|| ModError::Other("triplet file".into()))?);
        out.push((src, dst));
    }
    Ok(out)
}

/// Stage a prepared plan WITHOUT touching any live game file: snapshot each target's current
/// (pre-deploy) bytes into the undo, create its `*.gore-bak` backup, and record the intended
/// UE4SS mod dir. This runs BEFORE the deploy record is persisted; the actual live writes happen
/// later in [`apply_writes`], so a crash between record-write and apply is still recoverable.
fn stage(plan: &DeployPlan, record: &mut DeployRecord, undo: &mut Undo) -> Result<()> {
    // Note the intended UE4SS target(s) now so the persisted record knows about them even if a
    // crash interrupts the swap in `apply_writes` — undeploy can then still clean them up.
    //   - manager (`owner == "manager"`): a deployment composes SEVERAL mods, so ALL dirs go into
    //     `ue4ss_mod_dirs` and the single-mod `ue4ss_mod_dir` stays None (an old binary that can't
    //     read the vec falls back to the legacy-field mirror set up by `commit_plan`).
    //   - studio (`owner == ""`): at most one mod, so the first (only) dir keeps the legacy
    //     single-field shape and the vec stays empty.
    let manager = record.owner == "manager";
    for (i, (_, dst)) in plan.ue4ss_dirs.iter().enumerate() {
        let s = dst.display().to_string();
        if !manager && i == 0 {
            record.ue4ss_mod_dir = Some(s);
        } else if !record.ue4ss_mod_dirs.contains(&s) {
            record.ue4ss_mod_dirs.push(s);
        }
    }
    // Record the additive texture triplet dsts so undeploy can delete them (no backup needed).
    for (_, dst) in &plan.texture_triplets {
        record.texture_triplets.push(dst.display().to_string());
    }
    // Same for the manager-installed pak/triplet dsts (additive, no backup).
    for (_, dst) in &plan.managed_paks {
        record.managed_paks.push(dst.display().to_string());
    }
    for (live, _) in &plan.writes {
        // Snapshot the current (pre-deploy) bytes so rollback restores the EXACT prior state —
        // the previous mod's content, not just the game-pristine backup. If this read fails we
        // abort BEFORE writing anything, rather than snapshot empty and risk an empty-file rollback.
        let prior = std::fs::read(live).map_err(io("reading live file for rollback snapshot"))?;
        undo.files.push((live.clone(), prior));
        // If the live file drifted (game updated) since our last deploy, its preserved backup is
        // stale: drop it so backup() re-snapshots the current file as the new pristine, instead of
        // keeping a pre-update backup that a future undeploy would restore over the newer asset.
        // The removal MUST succeed — if it can't (read-only/locked), backup() would keep the stale
        // backup, so fail the deploy now (stage runs pre-write, so the caller rolls back cleanly).
        if plan.refresh_baks.iter().any(|p| p == live) {
            let bak = bak_path(live);
            if bak.exists() {
                // Snapshot the stale backup before deleting so rollback can put it back — otherwise
                // a later-step failure would restore the previous record while its backup is gone.
                if let Ok(prior_bak) = std::fs::read(&bak) {
                    undo.removed_baks.push((bak.clone(), prior_bak));
                }
                if std::fs::remove_file(&bak).is_err() && bak.exists() {
                    return Err(ModError::Other(format!(
                        "stale backup '{}' could not be removed (read-only or locked); close the \
                         game and retry so the updated game file can be re-backed-up",
                        bak.display()
                    )));
                }
            }
        }
        let (bak, created) = backup(live, record)?;
        if created {
            undo.created_baks.push(bak);
        }
    }
    Ok(())
}

/// Perform the live changes of a staged plan: install/swap the UE4SS mod and write each target
/// file. Backups and undo snapshots were already taken by [`stage`]; on error the caller calls
/// `undo.rollback()` to restore the exact prior state.
fn apply_writes(plan: &DeployPlan, undo: &mut Undo) -> Result<()> {
    for (src, dst) in &plan.ue4ss_dirs {
        // Stage into a sibling temp dir, then swap into place — a failed/partial copy never
        // destroys a previous same-named UE4SS mod already at `dst`.
        let staging = staging_dir(dst);
        let _ = std::fs::remove_dir_all(&staging);
        // If this copy fails, `dst` (a previous mod) is untouched and the undo doesn't yet track
        // the swap, so rollback won't delete it. Clean up the partial staging dir so UE4SS can't
        // pick it up as a stray enabled mod.
        if let Err(e) = copy_dir(src, &staging) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        if dst.exists() {
            // Move the old mod aside (atomic), then swap the staged copy in. The old dir is kept
            // (via the undo) until the whole deploy commits; on any failure it is moved back.
            let old = staging_old(dst);
            let _ = std::fs::remove_dir_all(&old);
            std::fs::rename(dst, &old).map_err(io("moving old ue4ss mod aside"))?;
            match std::fs::rename(&staging, dst) {
                Ok(()) => {
                    undo.ue4ss_old.push((old, dst.clone()));
                }
                Err(e) => {
                    let _ = std::fs::rename(&old, dst); // restore the previous mod
                    let _ = std::fs::remove_dir_all(&staging);
                    return Err(io("installing ue4ss mod")(e));
                }
            }
        } else {
            std::fs::rename(&staging, dst).map_err(io("installing ue4ss mod"))?;
            undo.ue4ss_fresh.push(dst.clone());
        }
    }
    // Copy each texture triplet file into `~mods`, tracking it for rollback. Snapshot any bytes
    // already at `dst` BEFORE overwriting (a same-named redeploy targets the same paths as the
    // currently-active deployment) so rollback restores the prior active triplet rather than
    // deleting it; `None` marks a fresh addition that rollback should delete.
    for (src, dst) in &plan.texture_triplets {
        if let Some(p) = dst.parent() {
            std::fs::create_dir_all(p).map_err(io("mkdir ~mods"))?;
        }
        // Read the prior bytes first: if the copy below fails after partially writing, rollback
        // still has the original content to restore (or knows to delete a fresh file).
        let prior = match std::fs::read(dst) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io(&format!("snapshot existing triplet {}", dst.display()))(e)),
        };
        undo.texture_files.push((dst.clone(), prior));
        std::fs::copy(src, dst).map_err(io(&format!("copy triplet to {}", dst.display())))?;
    }
    // Copy each manager-installed pak/triplet file into place, tracked for rollback exactly
    // like the texture triplets above (prior bytes restored, fresh additions deleted). Their
    // srcs are durable library files — no temp pack dirs to clean up afterwards.
    for (src, dst) in &plan.managed_paks {
        if let Some(p) = dst.parent() {
            std::fs::create_dir_all(p).map_err(io("mkdir ~mods"))?;
        }
        let prior = match std::fs::read(dst) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io(&format!("snapshot existing pak {}", dst.display()))(e)),
        };
        undo.texture_files.push((dst.clone(), prior));
        std::fs::copy(src, dst).map_err(io(&format!("copy pak to {}", dst.display())))?;
    }
    // The packed triplets are now in ~mods; remove their temp pack dirs
    // (gore-mod-tex-pack-<pid>-<idx>) so a successful deploy doesn't leave a full
    // .utoc/.ucas/.pak behind in the system temp dir on every restart / pid change.
    // Done only after all copies succeed (a failed copy returns above and leaves
    // the dirs for the rollback / next run).
    let mut pack_dirs: Vec<&std::path::Path> = plan
        .texture_triplets
        .iter()
        .filter_map(|(src, _)| src.parent())
        .collect();
    pack_dirs.sort();
    pack_dirs.dedup();
    for dir in pack_dirs {
        let _ = std::fs::remove_dir_all(dir);
    }
    for (live, bytes) in &plan.writes {
        atomic_write(live, bytes)?;
    }
    Ok(())
}

/// Retire the previous mod's leftover footprint (already folded into `record` and persisted):
/// restore each leftover loose file to pristine now and, on success, drop the entry from the
/// record so undeploy won't later look for a deleted backup. Leftovers that can't be restored
/// yet stay tracked. Also removes a differently-named UE4SS mod. Returns `(changed, pending_deletes)`
/// — whether `record` changed (and so should be re-persisted), and the retired entries
/// `(live, bak, prior_hash)` whose backup deletion the caller must DEFER until the pruned record is
/// durable (and re-track if deletion fails). Best-effort — never fails.
fn retire_leftovers(
    leftovers: &[(String, String, bool)],
    prev: Option<&DeployRecord>,
    plan: &DeployPlan,
    record: &mut DeployRecord,
) -> (bool, Vec<(String, String, Option<String>)>) {
    let mut changed = false;
    let mut pending_deletes = Vec::new();
    for (live_s, bak_s, _) in leftovers {
        let (live, bak) = (Path::new(live_s), Path::new(bak_s));
        let retired = if !safe_to_restore(live_s, &record.deployed_hashes) {
            // The file was updated externally (Steam) since the previous deploy; don't overwrite
            // the newer asset. The stale backup must be deleted (deferred below).
            true
        } else if !bak.exists() {
            // No backup to restore from — the live file was NOT reverted and may still hold the
            // old patch. Keep the entry so undeploy can warn/retry, rather than silently dropping
            // it as if it were cleanly retired.
            false
        } else if std::fs::read(bak).map(|b| atomic_write(live, &b).is_ok()).unwrap_or(false) {
            // Restored. DEFER deleting the backup until the caller has durably persisted the
            // pruned record, so a failed record rewrite can't leave the on-disk record pointing
            // at an already-deleted backup (which would wedge a later undeploy).
            true
        } else {
            false // locked/unwritable — keep tracked for an undeploy retry
        };
        if retired {
            // Capture the prior hash so the caller can re-track this entry if the deferred backup
            // deletion fails (a locked/read-only stale backup must not be orphaned untracked).
            let hash = record.deployed_hashes.remove(live_s);
            record.backups.retain(|(l, b, _)| !(l == live_s && b == bak_s));
            pending_deletes.push((live_s.clone(), bak_s.clone(), hash));
            changed = true;
        }
    }
    if let Some(prev) = prev {
        // ALL comparisons here use `same_path`: the record holds canonicalized (`\\?\`-prefixed)
        // paths while the plan holds the plain form of the same file, so a raw compare could both
        // (a) fail to recognize a still-installed target as "keep" — retiring a file/dir this very
        // deploy just wrote — and (b) fail to prune a cleaned entry from the record. `same_path`
        // resolves both to the same canonical file.
        let new_dirs = plan.ue4ss_dsts();
        // Retire the previous deploy's UE4SS dir(s) AND any dirs it had already failed to remove
        // (its own `stale_ue4ss_dirs`). These were pre-seeded into `record.stale_ue4ss_dirs` for
        // crash-safety; here we actually remove them and reconcile the list: drop the ones we
        // cleaned, keep (locked/permissions) ones so a later undeploy still cleans them up.
        let prev_dirs: Vec<String> = prev
            .ue4ss_mod_dir
            .iter()
            .chain(prev.stale_ue4ss_dirs.iter())
            .chain(prev.ue4ss_mod_dirs.iter())
            .cloned()
            .collect();
        for prev_dir in prev_dirs {
            if contains_same_path(&new_dirs, &prev_dir) {
                continue;
            }
            let removed = std::fs::remove_dir_all(&prev_dir).is_ok() || !Path::new(&prev_dir).exists();
            let tracked = record.stale_ue4ss_dirs.iter().position(|d| same_path_s(d, &prev_dir));
            if removed {
                if let Some(i) = tracked {
                    record.stale_ue4ss_dirs.remove(i);
                    changed = true;
                }
            } else {
                // Couldn't remove it (locked/permissions). Best-effort: remove its enable flag so
                // UE4SS doesn't keep loading the old mod alongside the new one meanwhile, then track
                // it for a later undeploy to clean up.
                let _ = std::fs::remove_file(Path::new(&prev_dir).join("enabled.txt"));
                if tracked.is_none() {
                    record.stale_ue4ss_dirs.push(prev_dir.clone());
                    changed = true;
                }
            }
        }

        // Retire the previous deploy's additive ~mods files (texture triplets AND manager paks) not
        // re-created by this deploy. They have no backup — just delete the files. Membership is
        // tested against the UNION of the plan's additive dsts (see `DeployPlan::additive_dsts`):
        // the manager legacy-mirror means a prev entry recorded under either field may be re-created
        // by this plan under the OTHER kind, and must not be deleted. On success prune from
        // whichever record field held it (pre-seeded above); on failure (locked) keep it tracked.
        let new_additive = plan.additive_dsts();
        let prev_additive: Vec<String> = prev
            .texture_triplets
            .iter()
            .chain(prev.managed_paks.iter())
            .cloned()
            .collect();
        for t in prev_additive {
            if contains_same_path(&new_additive, &t) {
                continue; // this deploy re-creates it; it stays as the active deployment's file
            }
            let removed = std::fs::remove_file(Path::new(&t)).is_ok() || !Path::new(&t).exists();
            if removed {
                if let Some(i) = record.texture_triplets.iter().position(|x| same_path_s(x, &t)) {
                    record.texture_triplets.remove(i);
                    changed = true;
                }
                if let Some(i) = record.managed_paks.iter().position(|x| same_path_s(x, &t)) {
                    record.managed_paks.remove(i);
                    changed = true;
                }
            }
            // not removed (locked) -> leave it tracked in record for a later undeploy to retry
        }
    }
    (changed, pending_deletes)
}

fn staging_dir(dst: &Path) -> PathBuf {
    swap_temp(dst, ".gore-new")
}

fn staging_old(dst: &Path) -> PathBuf {
    swap_temp(dst, ".gore-old")
}

/// A temp path for the staged/aside UE4SS mod, placed ONE LEVEL ABOVE `ue4ss/Mods` (still on the
/// same volume, so the rename into place stays atomic). Keeping these half-written/aside dirs out
/// of `Mods` means UE4SS never scans them as enabled mods if a crash interrupts the swap — inside
/// `Mods` their `enabled.txt` would make them loadable strays the deploy record doesn't track.
fn swap_temp(dst: &Path, suffix: &str) -> PathBuf {
    match (dst.file_name(), dst.parent().and_then(Path::parent)) {
        (Some(name), Some(ue4ss_root)) => {
            let mut fname = name.to_os_string();
            fname.push(suffix);
            ue4ss_root.join(fname)
        }
        _ => {
            // Fallback: sibling-in-place (e.g. unexpected path shape).
            let mut s = dst.as_os_str().to_os_string();
            s.push(suffix);
            PathBuf::from(s)
        }
    }
}

/// Whether `a` and the stored path string `b` refer to the same file, comparing canonical
/// forms when both resolve (falling back to a lexical compare otherwise).
fn same_path(a: &Path, b: &str) -> bool {
    let b = Path::new(b);
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Whether two stored path STRINGS refer to the same file. Records hold already-canonicalized
/// paths (e.g. `\\?\C:\...` on Windows), but a plan built this run may hold the plain form of the
/// same file; a raw string `==`/`contains` would then miss the match. Canonical-form compare (with
/// a lexical fallback when a side can't be resolved) so prev-vs-new membership checks — pre-seed
/// and `retire_leftovers` — recognize logically-identical paths regardless of prefix/case.
fn same_path_s(a: &str, b: &str) -> bool {
    same_path(Path::new(a), b)
}

/// Whether `list` already contains a path referring to the same file as `p` (`same_path_s`).
fn contains_same_path(list: &[String], p: &str) -> bool {
    list.iter().any(|x| same_path_s(x, p))
}

fn read_record(game_root: &Path) -> Option<DeployRecord> {
    std::fs::read(record_path(game_root))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

/// Pristine bytes to rebuild a modded file from, plus whether the live file has DRIFTED from what
/// we previously deployed there (e.g. Steam verified/updated it). Normally the preserved
/// `*.gore-bak` is the pristine source; but if `prev` recorded a hash for this file and the
/// current live no longer matches it while a backup exists, that backup is stale (pre-update) —
/// rebuilding from it would write an old asset over the newer game file. In that case the
/// (updated) live IS the new pristine and the caller must refresh the stale backup. Never writes.
pub(crate) fn read_pristine(live: &Path, prev: Option<&DeployRecord>) -> Result<(Vec<u8>, bool)> {
    let bak = bak_path(live);
    if bak.exists() {
        let live_key = live.display().to_string();
        match prev.and_then(|p| p.deployed_hashes.get(&live_key)) {
            Some(expected) => {
                if let Ok(cur) = std::fs::read(live) {
                    if &content_hash(&cur) != expected {
                        return Ok((cur, true)); // drifted — rebuild from the updated live file
                    }
                }
            }
            None => {
                // No recorded hash to judge drift (e.g. a leftover backup from the CLI, or the
                // record was cleared). Fall back to FMOD structure: if the live BANK is a clean
                // un-injected pristine (a single FSB5), it is itself the current pristine — prefer
                // it (covering a Steam verify/update that refreshed the bank) and refresh the
                // possibly-stale backup. A non-bank (.lcache), corrupt, or already-injected live
                // has no such signal, so use the backup.
                if let Ok(cur) = std::fs::read(live) {
                    if gore_fmod::is_pristine_bank(&cur) {
                        return Ok((cur, true));
                    }
                }
            }
        }
        let bytes = std::fs::read(&bak).map_err(io(&format!("reading pristine {}", live.display())))?;
        return Ok((bytes, false));
    }
    // No backup yet — the live file is the pristine source (first deploy).
    let bytes = std::fs::read(live).map_err(io(&format!("reading pristine {}", live.display())))?;
    Ok((bytes, false))
}

pub(crate) fn bak_path(live: &Path) -> PathBuf {
    let mut s = live.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

/// The PRISTINE precompiled-script cache bytes deploy would use for `game_root`, honoring drift:
/// if a `*.gore-bak` backup exists but the live cache has DRIFTED from what we last deployed there
/// (game update/verify), the backup is stale and the (updated) live cache is the new pristine.
/// Reuses the same [`read_pristine`]/[`read_record`] logic as deploy so the compile base matches
/// the bytes the splice will later be applied against. Never writes.
pub fn pristine_script_cache(game_root: &Path) -> Result<Vec<u8>> {
    let script_cache = resolve_game_paths(game_root).script_cache;
    let record = read_record(game_root);
    read_pristine(&script_cache, record.as_ref()).map(|(bytes, _drifted)| bytes)
}

/// Undeploy: restore every live file from its backup and remove the UE4SS mod. Each entry is
/// finalized INDEPENDENTLY — restore (or skip-if-drifted) AND delete its backup as a unit, then
/// drop it from `record` — so a later locked backup can't leave earlier, already-deleted backups
/// dangling in a retained record. Returns true only if EVERYTHING was handled; otherwise the
/// still-pending entries remain in `record` so the caller can persist a pruned record and retry.
/// (Deploy rollback uses [`Undo`] instead, to restore the exact prior state.)
fn restore_record(record: &mut DeployRecord) -> bool {
    let mut all_ok = true;
    let backups = std::mem::take(&mut record.backups);
    for (live_s, bak_s, created) in backups {
        let (live, bak) = (Path::new(&live_s), Path::new(&bak_s));
        // If the live file was updated/verified externally since we deployed (e.g. Steam), the
        // recorded backup is stale — restoring it would downgrade the newer asset. Just delete the
        // stale backup (the deletion must succeed; a lingering backup with no record could later be
        // treated as pristine). Otherwise restore the live file from the backup, then delete it.
        let done = if !safe_to_restore(&live_s, &record.deployed_hashes) {
            std::fs::remove_file(bak).is_ok() || !bak.exists()
        } else if !bak.exists() {
            false // recorded backup is gone — this file can't be restored
        } else {
            match std::fs::read(bak) {
                Ok(bytes) if atomic_write(live, &bytes).is_ok() => {
                    std::fs::remove_file(bak).is_ok() || !bak.exists()
                }
                _ => false,
            }
        };
        if done {
            record.deployed_hashes.remove(&live_s);
        } else {
            record.backups.push((live_s, bak_s, created)); // keep for a retry
            all_ok = false;
        }
    }
    if let Some(dir) = record.ue4ss_mod_dir.clone() {
        if !Path::new(&dir).exists() || std::fs::remove_dir_all(&dir).is_ok() {
            record.ue4ss_mod_dir = None;
        } else {
            all_ok = false;
        }
    }
    let stale = std::mem::take(&mut record.stale_ue4ss_dirs);
    for dir in stale {
        if !Path::new(&dir).exists() || std::fs::remove_dir_all(&dir).is_ok() {
            // cleaned — drop it
        } else {
            record.stale_ue4ss_dirs.push(dir); // keep for a retry
            all_ok = false;
        }
    }
    // Additive texture triplet files in `~mods` (no backup) — delete them. A failed delete
    // (locked) must KEEP the entry and fail the undeploy (all_ok=false), so the record is not
    // deleted and a retry can still remove the lingering override; otherwise the triplet would
    // be orphaned on disk with nothing tracking it.
    for f in std::mem::take(&mut record.texture_triplets) {
        let p = Path::new(&f);
        if !p.exists() || std::fs::remove_file(p).is_ok() {
            // removed (or already gone) — drop it
        } else {
            record.texture_triplets.push(f); // locked — keep for a retry
            all_ok = false;
        }
    }
    // Manager-installed pak/triplet files in `~mods` (pure additions, no backup) — delete
    // them, with the same keep-on-failure accounting as `texture_triplets` above.
    for f in std::mem::take(&mut record.managed_paks) {
        let p = Path::new(&f);
        if !p.exists() || std::fs::remove_file(p).is_ok() {
            // removed (or already gone) — drop it
        } else {
            record.managed_paks.push(f); // locked — keep for a retry
            all_ok = false;
        }
    }
    // Manager-installed UE4SS mod dirs — remove each with the same semantics as the single
    // `ue4ss_mod_dir` above: cleaned entries are dropped, failed ones kept for a retry.
    for dir in std::mem::take(&mut record.ue4ss_mod_dirs) {
        if !Path::new(&dir).exists() || std::fs::remove_dir_all(&dir).is_ok() {
            // cleaned — drop it
        } else {
            record.ue4ss_mod_dirs.push(dir); // keep for a retry
            all_ok = false;
        }
    }
    all_ok
}

/// Undo the active gore-mod deployment at `game_root`: restore every backup and remove the
/// UE4SS mod. No-op if nothing is deployed.
pub fn undeploy(game_root: &Path) -> Result<Option<DeployRecord>> {
    // Match deploy's absolutization so the record file is found regardless of the caller's cwd.
    let game_root = &abs_root(game_root);
    let rp = record_path(game_root);
    let Ok(bytes) = std::fs::read(&rp) else {
        return Ok(None);
    };
    let mut record: DeployRecord = serde_json::from_slice(&bytes)?;
    if restore_record(&mut record) {
        let _ = std::fs::remove_file(&rp);
        // Return the original record (pre-pruning) for reporting.
        Ok(serde_json::from_slice(&bytes).ok())
    } else {
        // Persist the PRUNED record so a retry only processes what's still pending — entries whose
        // file was restored and backup deleted are not re-attempted (and won't fail the next run on
        // a now-missing backup). Then report failure so the user can resolve the lock and retry.
        let _ = write_record_file(game_root, &record);
        Err(ModError::Other(
            "some game files could not be restored (locked or unwritable); the remaining backups \
             and a pruned deploy record were kept — close the game and retry undeploy"
                .into(),
        ))
    }
}

/// Back up `live` to `live.gore-bak` if no backup exists yet (preserving the pristine file),
/// register it in `record`, and return the backup path. The backup is the pristine source.
pub(crate) fn backup(live: &Path, record: &mut DeployRecord) -> Result<(PathBuf, bool)> {
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

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".gore-tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(io("writing temp"))?;
    // `std::fs::rename` REPLACES an existing destination on every platform we target: on Windows
    // Rust implements it via `MoveFileExW(.., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`,
    // not the bare `MoveFile`/`rename()` that fails when the target exists. So this safely overwrites
    // existing game files / records in place; do NOT switch to remove-then-rename (that adds a
    // non-atomic window where a crash leaves the destination missing).
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

    /// [8] Distinct mod names that sanitize to the SAME stem (chars folded to `_`)
    /// must still produce DIFFERENT texture triplet names, so one mod's mounted pak
    /// can't clobber another's. The triplet stem mirrors the deploy code:
    /// `zzz_{sanitize(name)}_{name_hash(name)}_{idx}_tex_P`.
    #[test]
    fn distinct_mod_names_folding_to_same_stem_get_distinct_triplets() {
        let a = "A+B";
        let b = "A B";
        // Sanitize alone collides...
        assert_eq!(sanitize(a), sanitize(b), "precondition: stems must collide");
        // ...but the hash of the ORIGINAL name disambiguates.
        assert_ne!(name_hash(a), name_hash(b), "name_hash must differ");

        let name_for = |n: &str, idx: usize| {
            format!("zzz_{}_{}_{}_tex_P", sanitize(n), name_hash(n), idx)
        };
        assert_ne!(name_for(a, 0), name_for(b, 0), "triplet names must differ");
        // Same name, different component index -> still distinct (bug [2]).
        assert_ne!(name_for(a, 0), name_for(a, 1), "per-component names must differ");
        // Stable across calls (no RNG / SipHash).
        assert_eq!(name_hash(a), name_hash(a));
    }

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
            texture: vec![],
            scripts: vec![],
        };

        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("ue4ss/MyMod/Scripts/main.lua"));
        assert!(bundle.files.contains_key("ue4ss/MyMod/enabled.txt"));
        assert!(bundle.files.contains_key("loc/edits.json"));
        assert!(bundle.files.contains_key("audio/manifest.json"));
        assert!(bundle.files.contains_key("audio/0_SFX_bank__SFX_UI_X.wav"));
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
            texture: vec![],
            scripts: vec![],
        };
        assert!(build_bundle(&spec).is_err());
    }

    #[test]
    fn build_emits_texture_patch() {
        let dir = std::env::temp_dir().join("gore-mod-tex-build");
        let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("img.png");
        std::fs::write(&png, b"\x89PNG\r\n\x1a\nfake").unwrap();
        let spec = BuildSpec {
            meta: ModMeta { name: "TestMod".into(), version: String::new(), author: String::new() },
            delay_ms: 0, overrides: vec![], loc_edits: Default::default(), audio: vec![],
            texture: vec![TextureReplacement { asset: "/Game/UI/T_X".into(), image_path: png.display().to_string() }],
            scripts: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("texture/manifest.json"));
        assert!(bundle.files.keys().any(|k| k.starts_with("texture/") && k.ends_with(".png")));
        assert!(matches!(bundle.manifest.components.last(),
            Some(Component::TexturePatch { assets, .. }) if assets == &vec!["/Game/UI/T_X".to_string()]));
    }

    #[test]
    fn build_emits_angelscript_patch() {
        let dir = std::env::temp_dir().join("gore-mod-as-build");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mini = dir.join("mod.cache");
        std::fs::write(&mini, b"MINI-CACHE-BYTES").unwrap();
        let spec = BuildSpec {
            meta: ModMeta { name: "AsMod".into(), version: String::new(), author: String::new() },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![],
            scripts: vec![ScriptModule {
                op: "add".into(),
                module_name: "MyMod".into(),
                mini_cache: mini.display().to_string(),
            }],
        };
        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("scripts/manifest.json"));
        assert!(bundle.files.contains_key("scripts/0_MyMod.cache"));
        assert_eq!(bundle.files["scripts/0_MyMod.cache"], b"MINI-CACHE-BYTES");
        assert!(matches!(bundle.manifest.components.last(),
            Some(Component::AngelScriptPatch { path }) if path == "scripts"));
        // manifest round-trips to the typed entry
        let m: Vec<ScriptEntry> =
            serde_json::from_slice(&bundle.files["scripts/manifest.json"]).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].op, "add");
        assert_eq!(m[0].module, "MyMod");
        assert_eq!(m[0].mini, "scripts/0_MyMod.cache");
    }

    /// prepare() must reject a manifest whose op is neither add nor edit, naming the module.
    #[test]
    fn prepare_rejects_bad_script_op() {
        let dir = std::env::temp_dir().join("gore-mod-as-prep-badop");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/0_M.cache"), b"x").unwrap();
        let entries = vec![ScriptEntry { op: "nope".into(), module: "M".into(), mini: "scripts/0_M.cache".into() }];
        std::fs::write(dir.join("scripts/manifest.json"), serde_json::to_vec(&entries).unwrap()).unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta { name: "M".into(), version: String::new(), author: String::new() },
            components: vec![Component::AngelScriptPatch { path: "scripts".into() }],
        };
        // A game dir whose script cache file exists (content irrelevant — op is rejected first).
        let game = dir.join("game");
        let script_dir = game.join("G1R/Script");
        std::fs::create_dir_all(&script_dir).unwrap();
        std::fs::write(script_dir.join("PrecompiledScript_Shipping.Cache"), b"base").unwrap();
        let gp = resolve_game_paths(&game);
        let err = prepare(&dir, &manifest, &gp, None).unwrap_err();
        assert!(err.to_string().contains("invalid script op"), "got: {err}");
    }

    /// prepare() must error clearly when the game has no script cache.
    #[test]
    fn prepare_errors_when_no_script_cache() {
        let dir = std::env::temp_dir().join("gore-mod-as-prep-nocache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/0_M.cache"), b"x").unwrap();
        let entries = vec![ScriptEntry { op: "add".into(), module: "M".into(), mini: "scripts/0_M.cache".into() }];
        std::fs::write(dir.join("scripts/manifest.json"), serde_json::to_vec(&entries).unwrap()).unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta { name: "M".into(), version: String::new(), author: String::new() },
            components: vec![Component::AngelScriptPatch { path: "scripts".into() }],
        };
        let gp = resolve_game_paths(&dir.join("empty-game"));
        let err = prepare(&dir, &manifest, &gp, None).unwrap_err();
        assert!(err.to_string().contains("script cache not found"), "got: {err}");
    }

    /// Full splice against a real game install. Run with:
    ///   GORE_TEST_GAME="C:/.../Gothic 1 Remake"  cargo test -p gore-mod -- --ignored real_script_deploy
    #[test]
    #[ignore]
    fn real_script_deploy_add_roundtrips() {
        use gore_as::cache::walk_modules::module_count;
        let Ok(game) = std::env::var("GORE_TEST_GAME") else { return; };
        let game = std::path::PathBuf::from(game);
        let gp = resolve_game_paths(&game);
        assert!(gp.script_cache.exists(), "no script cache at {}", gp.script_cache.display());
        // A mini-cache produced by `gore as extract`/`script_compile` (Task 11). Provide its path:
        let Ok(mini) = std::env::var("GORE_TEST_MINI") else {
            eprintln!("set GORE_TEST_MINI to a 1-module mini-cache to run the splice");
            return;
        };
        let dir = std::env::temp_dir().join("gore-mod-as-real");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let spec = BuildSpec {
            meta: ModMeta { name: "RealAsMod".into(), version: String::new(), author: String::new() },
            delay_ms: 0, overrides: vec![], loc_edits: Default::default(), audio: vec![], texture: vec![],
            scripts: vec![ScriptModule { op: "add".into(), module_name: "ignored_for_add".into(), mini_cache: mini }],
        };
        let bundle = build_bundle(&spec).unwrap();
        write_bundle(&dir, &bundle).unwrap();
        let manifest = bundle.manifest;
        let before = module_count(&std::fs::read(&gp.script_cache).unwrap());
        let plan = prepare(&dir, &manifest, &gp, None).unwrap();
        let (_, spliced) = plan.writes.last().unwrap();
        assert_eq!(module_count(spliced), before + 1, "splice should add exactly one module");
    }

    #[test]
    fn retire_deletes_prev_texture_triplets_not_in_new_plan() {
        // A prior deploy left a triplet in ~mods; the new deploy has no (or a differently-named)
        // texture component. retire_leftovers must delete the stale triplet + prune it from the
        // record so it neither lingers mounted nor escapes a later undeploy.
        let dir = std::env::temp_dir().join("gore-mod-retire-tex");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old: Vec<String> = ["zzz_Old_tex_P.utoc", "zzz_Old_tex_P.ucas", "zzz_Old_tex_P.pak"]
            .iter()
            .map(|n| { let p = dir.join(n); std::fs::write(&p, b"x").unwrap(); p.display().to_string() })
            .collect();
        let prev = DeployRecord { mod_name: "Old".into(), texture_triplets: old.clone(), ..Default::default() };
        // The new record was pre-seeded with the prev triplets (as deploy() step (b) does).
        let mut record = DeployRecord { mod_name: "New".into(), texture_triplets: old.clone(), ..Default::default() };
        // New plan has NO texture triplets (e.g. a non-texture mod) -> all prev ones are stale.
        let plan = DeployPlan::default();
        let (changed, _) = retire_leftovers(&[], Some(&prev), &plan, &mut record);
        assert!(changed);
        for f in &old { assert!(!std::path::Path::new(f).exists(), "stale triplet not deleted: {f}"); }
        assert!(record.texture_triplets.is_empty(), "stale triplets not pruned from record");
    }

    #[test]
    fn undeploy_removes_recorded_texture_triplets() {
        let game = std::env::temp_dir().join("gore-mod-undeploy-tex");
        let _ = std::fs::remove_dir_all(&game);
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let files: Vec<String> = ["zzz_M_tex_P.utoc","zzz_M_tex_P.ucas","zzz_M_tex_P.pak"].iter().map(|n| {
            let p = mods.join(n); std::fs::write(&p, b"x").unwrap(); p.display().to_string()
        }).collect();
        let rec = DeployRecord { mod_name: "M".into(), texture_triplets: files.clone(), ..Default::default() };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();
        undeploy(&game).unwrap();
        for f in &files { assert!(!std::path::Path::new(f).exists(), "triplet not removed: {f}"); }
    }

    /// A record written by a pre-manager build (only the original fields) must still parse,
    /// with the v2 fields defaulted: no owner (legacy/studio), empty loadout/dirs/paks.
    #[test]
    fn legacy_record_json_parses_owner_empty() {
        let json = r#"{
            "mod_name": "OldMod",
            "ue4ss_mod_dir": "C:/game/G1R/Binaries/Win64/ue4ss/Mods/OldMod",
            "backups": [["C:/game/G1R/Story/Cache/A.lcache", "C:/game/G1R/Story/Cache/A.lcache.gore-bak", true]]
        }"#;
        let rec: DeployRecord = serde_json::from_str(json).unwrap();
        assert_eq!(rec.mod_name, "OldMod");
        assert_eq!(rec.ue4ss_mod_dir.as_deref(), Some("C:/game/G1R/Binaries/Win64/ue4ss/Mods/OldMod"));
        assert_eq!(rec.backups.len(), 1);
        assert_eq!(rec.owner, "");
        assert!(rec.loadout.is_empty());
        assert!(rec.ue4ss_mod_dirs.is_empty());
        assert!(rec.managed_paks.is_empty());
    }

    /// Undeploying a v2 record must delete its manager-installed pak files and remove each
    /// of its UE4SS mod dirs, exactly like the single-mod fields.
    #[test]
    fn restore_removes_managed_paks_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let pak = mods.join("zzz_mgr_A_P.pak");
        std::fs::write(&pak, b"pak").unwrap();
        let ue4ss_dir = game.join("G1R/Binaries/Win64/ue4ss/Mods/MgrModA");
        std::fs::create_dir_all(&ue4ss_dir).unwrap();
        std::fs::write(ue4ss_dir.join("enabled.txt"), b"").unwrap();
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            ue4ss_mod_dirs: vec![ue4ss_dir.display().to_string()],
            managed_paks: vec![pak.display().to_string()],
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();
        let restored = undeploy(&game).unwrap();
        assert!(restored.is_some(), "undeploy should report the restored record");
        assert!(!pak.exists(), "managed pak not removed");
        assert!(!ue4ss_dir.exists(), "managed ue4ss dir not removed");
    }

    #[test]
    fn rel_path_with_dotdot_is_unsafe() {
        assert!(!is_safe_rel_path("G1R/Content/../../../Foo"));
        assert!(is_safe_rel_path("G1R/Content/UI/Textures/T_X"));
    }

    /// The Ue4ssLua component must carry the `Class.Field` CDO targets of the spec's
    /// overrides (sorted, deduped) — the mod-manager's conflict-detection contract.
    #[test]
    fn build_bundle_fills_ue4ss_targets() {
        let spec = BuildSpec {
            meta: ModMeta { name: "TgtMod".into(), version: String::new(), author: String::new() },
            delay_ms: 0,
            // Deliberately unsorted so the sort is observable.
            overrides: vec![
                SingleOverride {
                    class: "ClassB".into(),
                    field: "FieldY".into(),
                    module: "Angelscript".into(),
                    value: OverrideValue::Int(1),
                },
                SingleOverride {
                    class: "ClassA".into(),
                    field: "FieldX".into(),
                    module: "Angelscript".into(),
                    value: OverrideValue::Int(2),
                },
            ],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            scripts: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        let expected = vec!["ClassA.FieldX".to_string(), "ClassB.FieldY".to_string()];
        let Some(Component::Ue4ssLua { targets, .. }) = bundle.manifest.components.first() else {
            panic!("expected a Ue4ssLua component");
        };
        assert_eq!(targets, &expected);
        // And the serialized manifest round-trips them.
        let m: ModManifest = serde_json::from_slice(&bundle.files["gore-mod.json"]).unwrap();
        assert!(matches!(
            m.components.first(),
            Some(Component::Ue4ssLua { targets, .. }) if targets == &expected
        ));
    }

    /// A format-1 manifest written before `targets` existed must still parse, with the
    /// field defaulting to empty.
    #[test]
    fn manifest_v1_json_without_targets_parses() {
        let json = r#"{
            "format": 1,
            "mod": { "name": "OldMod", "version": "", "author": "" },
            "components": [
                { "type": "ue4ss_lua", "name": "OldMod", "path": "ue4ss/OldMod" }
            ]
        }"#;
        let m: ModManifest = serde_json::from_str(json).unwrap();
        assert!(matches!(
            m.components.first(),
            Some(Component::Ue4ssLua { name, path, targets })
                if name == "OldMod" && path == "ue4ss/OldMod" && targets.is_empty()
        ));
    }

    /// The single-mod deploy must refuse to clobber a manager-owned deployment — it would
    /// silently retire the whole multi-mod loadout.
    #[test]
    fn deploy_refuses_manager_record() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let rec =
            DeployRecord { mod_name: "loadout".into(), owner: "manager".into(), ..Default::default() };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();

        // A minimal valid bundle (one override → one Ue4ssLua component).
        let spec = BuildSpec {
            meta: ModMeta { name: "Solo".into(), version: String::new(), author: String::new() },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ClassA".into(),
                field: "FieldX".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(1),
            }],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            scripts: vec![],
        };
        let bundle_dir = dir.path().join("bundle");
        write_bundle(&bundle_dir, &build_bundle(&spec).unwrap()).unwrap();

        let err = deploy(&bundle_dir, &game).unwrap_err();
        assert!(err.to_string().contains("manager"), "got: {err}");
        // The guard must trip BEFORE any commit work: the manager record is untouched.
        let after: DeployRecord =
            serde_json::from_slice(&std::fs::read(record_path(&game)).unwrap()).unwrap();
        assert_eq!(after.owner, "manager");
        assert_eq!(after.mod_name, "loadout");
    }

    // ── multi-mod commit_plan hardening ────────────────────────────────────────────────

    /// Create a small real UE4SS mod source dir (with an `enabled.txt`) under `base/<name>`.
    fn make_mod_src(base: &Path, name: &str) -> PathBuf {
        let src = base.join(name);
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("enabled.txt"), b"").unwrap();
        src
    }

    /// FIX 1: a manager-owned deployment records EVERY UE4SS dir in `ue4ss_mod_dirs` and leaves the
    /// legacy single `ue4ss_mod_dir` unset (studio behavior — first-into-legacy — is unchanged).
    #[test]
    fn manager_record_puts_all_ue4ss_dirs_in_vec() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        std::fs::create_dir_all(&mods).unwrap();
        let src_a = make_mod_src(dir.path(), "SrcA");
        let src_b = make_mod_src(dir.path(), "SrcB");
        let gp = resolve_game_paths(&game);

        let plan = DeployPlan {
            ue4ss_dirs: vec![(src_a, mods.join("ModA")), (src_b, mods.join("ModB"))],
            ..Default::default()
        };
        let record = DeployRecord { mod_name: "loadout".into(), owner: "manager".into(), ..Default::default() };
        let rec = commit_plan(&gp, &game, plan, record, None).unwrap();

        assert!(rec.ue4ss_mod_dir.is_none(), "manager must not use the legacy single dir field");
        assert_eq!(rec.ue4ss_mod_dirs.len(), 2, "all dirs go into the vec");
        assert!(rec.ue4ss_mod_dirs.iter().any(|d| d.ends_with("ModA")));
        assert!(rec.ue4ss_mod_dirs.iter().any(|d| d.ends_with("ModB")));
        assert!(mods.join("ModA").exists() && mods.join("ModB").exists(), "both dirs installed");
    }

    /// FIX 2: a manager deployment mirrors its footprint (managed paks → `texture_triplets`, UE4SS
    /// dirs → `stale_ue4ss_dirs`) into the legacy record fields a pre-v2 binary understands, and a
    /// v2 undeploy still removes everything exactly once with no double-error.
    #[test]
    fn manager_record_mirrors_footprint_into_legacy_fields() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        std::fs::create_dir_all(&mods).unwrap();
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&paks).unwrap();
        let src = make_mod_src(dir.path(), "SrcA");
        let pak_src = dir.path().join("lib_A.pak");
        std::fs::write(&pak_src, b"PAK").unwrap();
        let pak_dst = paks.join("zzz_mgr_A_P.pak");
        let ue4ss_dst = mods.join("MgrModA");
        let gp = resolve_game_paths(&game);

        let plan = DeployPlan {
            ue4ss_dirs: vec![(src, ue4ss_dst.clone())],
            managed_paks: vec![(pak_src, pak_dst.clone())],
            ..Default::default()
        };
        let record = DeployRecord { mod_name: "loadout".into(), owner: "manager".into(), ..Default::default() };
        let rec = commit_plan(&gp, &game, plan, record, None).unwrap();

        // Real (v2) fields.
        assert!(rec.ue4ss_mod_dir.is_none());
        assert_eq!(rec.ue4ss_mod_dirs.len(), 1);
        assert_eq!(rec.managed_paks.len(), 1);
        // Legacy mirror an old binary reads.
        assert!(same_path_contains(&rec.texture_triplets, &pak_dst), "pak not mirrored into texture_triplets");
        assert!(same_path_contains(&rec.stale_ue4ss_dirs, &ue4ss_dst), "dir not mirrored into stale_ue4ss_dirs");
        assert!(pak_dst.exists() && ue4ss_dst.exists(), "footprint installed");

        // v2 undeploy removes everything once, no error (the mirror + real field share paths, so
        // the second removal pass must be a harmless no-op).
        let restored = undeploy(&game).unwrap();
        assert!(restored.is_some(), "undeploy should report a restored record");
        assert!(!pak_dst.exists(), "managed pak not removed");
        assert!(!ue4ss_dst.exists(), "managed ue4ss dir not removed");
        assert!(!record_path(&game).exists(), "record file should be gone after a clean undeploy");
    }

    /// FIX 3(a): `commit_plan` rejects a self-colliding plan (two entries writing the same dst)
    /// BEFORE touching the game.
    #[test]
    fn commit_plan_rejects_duplicate_dsts() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&paks).unwrap();
        let a = dir.path().join("a.pak");
        let b = dir.path().join("b.pak");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let clash = paks.join("zzz_dup_P.pak");
        let gp = resolve_game_paths(&game);

        // Two different srcs, SAME dst — must be rejected.
        let plan = DeployPlan {
            managed_paks: vec![(a, clash.clone()), (b, clash.clone())],
            ..Default::default()
        };
        let record = DeployRecord { mod_name: "loadout".into(), owner: "manager".into(), ..Default::default() };
        let err = commit_plan(&gp, &game, plan, record, None).unwrap_err();
        assert!(err.to_string().contains("duplicate deploy target"), "got: {err}");
        // Nothing was written and no record file created (guard tripped before stage).
        assert!(!clash.exists());
        assert!(!record_path(&game).exists());
    }

    /// FIX 4: prev-vs-new membership uses `same_path`, so a prev record path in `\\?\`-canonical
    /// form and a new-plan plain path that resolve to the SAME file are recognized as identical —
    /// the file the new deploy re-creates is NOT retired.
    #[test]
    fn retire_tolerates_noncanonical_prev_paths() {
        let dir = tempfile::tempdir().unwrap();
        let paks = dir.path().join("mods");
        std::fs::create_dir_all(&paks).unwrap();
        let dst = paks.join("zzz_keep_P.pak");
        std::fs::write(&dst, b"keep").unwrap();
        // The prev record holds the CANONICAL (\\?\-prefixed on Windows) form of the same file.
        let canon = std::fs::canonicalize(&dst).unwrap().display().to_string();
        let plain = dst.display().to_string();
        assert_ne!(canon, plain, "precondition: canonical form must differ from the plain path");

        let prev = DeployRecord { mod_name: "Old".into(), texture_triplets: vec![canon.clone()], ..Default::default() };
        // The new plan re-creates the SAME file (plain path form).
        let plan = DeployPlan {
            texture_triplets: vec![(dst.clone(), dst.clone())],
            ..Default::default()
        };
        let mut record = DeployRecord { mod_name: "New".into(), texture_triplets: vec![plain.clone()], ..Default::default() };

        let _ = retire_leftovers(&[], Some(&prev), &plan, &mut record);
        assert!(dst.exists(), "file the new deploy re-creates must NOT be retired despite path-form mismatch");
    }

    /// MINOR (b): redeploying over a prev MANAGER record whose managed pak + UE4SS dir are NOT in
    /// the new plan retires both (files gone, entries pruned).
    #[test]
    fn redeploy_retires_prev_managed_paks_and_folds_ue4ss_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(&paks).unwrap();

        // Prev manager deployment: one managed pak + one UE4SS dir, both present on disk.
        let prev_pak = paks.join("zzz_prev_P.pak");
        std::fs::write(&prev_pak, b"old").unwrap();
        let prev_dir = mods.join("PrevMod");
        std::fs::create_dir_all(&prev_dir).unwrap();
        std::fs::write(prev_dir.join("enabled.txt"), b"").unwrap();
        let prev = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ue4ss_mod_dirs: vec![prev_dir.display().to_string()],
            managed_paks: vec![prev_pak.display().to_string()],
            // as a real prev manager record would also carry the legacy mirror
            stale_ue4ss_dirs: vec![prev_dir.display().to_string()],
            texture_triplets: vec![prev_pak.display().to_string()],
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&prev).unwrap()).unwrap();

        // New manager deployment: a DIFFERENT mod, nothing overlapping the prev footprint.
        let src = make_mod_src(dir.path(), "SrcNew");
        let new_dir = mods.join("NewMod");
        let gp = resolve_game_paths(&game);
        let plan = DeployPlan {
            ue4ss_dirs: vec![(src, new_dir.clone())],
            ..Default::default()
        };
        let record = DeployRecord { mod_name: "loadout".into(), owner: "manager".into(), ..Default::default() };
        let rec = commit_plan(&gp, &game, plan, record, Some(prev)).unwrap();

        assert!(!prev_pak.exists(), "prev managed pak not retired");
        assert!(!prev_dir.exists(), "prev ue4ss dir not retired");
        assert!(new_dir.exists(), "new ue4ss dir should be installed");
        assert!(!same_path_contains(&rec.managed_paks, &prev_pak), "prev pak still tracked");
        assert!(!same_path_contains(&rec.texture_triplets, &prev_pak), "prev pak still tracked (mirror)");
        assert!(!same_path_contains(&rec.stale_ue4ss_dirs, &prev_dir), "prev dir still tracked (mirror)");
        assert!(!same_path_contains(&rec.ue4ss_mod_dirs, &prev_dir), "prev dir still tracked");
    }

    /// MINOR (b): if a managed-pak copy fails mid-apply, the rollback removes the fresh pak dst(s)
    /// already copied and restores the pre-deploy record (here: none).
    #[test]
    fn rollback_removes_fresh_managed_pak_copy() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&paks).unwrap();
        // pak1 has a valid src (gets copied); pak2's src is missing (copy fails → rollback).
        let pak1_src = dir.path().join("ok.pak");
        std::fs::write(&pak1_src, b"ok").unwrap();
        let pak1_dst = paks.join("zzz_ok_P.pak");
        let pak2_src = dir.path().join("missing.pak"); // never created
        let pak2_dst = paks.join("zzz_missing_P.pak");
        let gp = resolve_game_paths(&game);

        let plan = DeployPlan {
            managed_paks: vec![(pak1_src, pak1_dst.clone()), (pak2_src, pak2_dst.clone())],
            ..Default::default()
        };
        let record = DeployRecord { mod_name: "loadout".into(), owner: "manager".into(), ..Default::default() };
        let err = commit_plan(&gp, &game, plan, record, None).unwrap_err();
        assert!(err.to_string().contains("copy pak"), "expected a copy failure, got: {err}");

        // Rollback must have deleted the already-copied fresh pak1, left no stray pak2, and (since
        // there was no prior record) removed the record file it wrote before applying.
        assert!(!pak1_dst.exists(), "fresh managed pak copy not removed on rollback");
        assert!(!pak2_dst.exists());
        assert!(!record_path(&game).exists(), "pre-write record should be gone (there was none)");
    }

    /// Test helper: does `list` hold a path referring to the same file as `p`?
    fn same_path_contains(list: &[String], p: &Path) -> bool {
        list.iter().any(|x| same_path(p, x))
    }
}
