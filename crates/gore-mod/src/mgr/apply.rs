//! Declarative apply: realize an enabled loadout into ONE manager-owned deployment.
//!
//! [`apply_loadout`] composes every enabled library mod's components into a single [`DeployPlan`]
//! and commits it through the same crash-safe machinery the single-bundle deploy uses. It always
//! starts from pristine: it undeploys whatever the manager had before, then rebuilds from scratch,
//! so a toggle/reorder is realized by re-applying (never an incremental patch on top of old state).
//!
//! ## Rawfile-vs-patch ordering
//! Within a single apply, each target game file is materialized in two conceptual layers:
//!   1. **base** — if any enabled mod supplies a `RawFile` for that target (whole-file replacement),
//!      its bytes become the BASE for that file. Loadout order decides the winner: a later mod's
//!      rawfile replaces an earlier one's. If no mod supplies a rawfile, the base is the game's
//!      pristine file (as restored by the pre-apply undeploy).
//!   2. **patches** — every loc / audio / AngelScript edit from ALL enabled mods is then applied ON
//!      TOP of that base. So a mod can ship a rawfile `.lcache` as the base and another mod's loc
//!      patch still lands on top of it; the loc/audio/script merges (later-wins per key) are
//!      independent of which mod (if any) supplied the base.
//! A rawfile whose target is never further patched is still written (its base bytes are the final
//! content). This is why the two passes below are: (1) collect rawfile base-overrides per target,
//! then (2) fold loc/audio/scripts on top and finally emit any un-patched rawfile bases verbatim.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::loadout::{Loadout, LoadoutEntry};
use super::model::{ComponentInfo, ModEntryMeta, RawTarget, META_FILE};
use crate::{DeployPlan, DeployRecord, ModError};

/// Outcome of an apply: the enabled mods realized (display names, in loadout order) and any
/// non-fatal warnings (a rawfile with no live target, a loc id/lang missing from this install, …).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApplyReport {
    pub applied: Vec<String>,
    pub warnings: Vec<String>,
}

/// Realize `loadout`'s enabled entries into one manager deployment at `game_root`, reading each
/// mod's payload from `<library_dir>/<id>/`. Refuses to run over a studio (non-manager) deployment.
/// Always undeploys first (pristine base), so the result is a full recomputation of the enabled set.
pub fn apply_loadout(
    game_root: &Path,
    library_dir: &Path,
    loadout: &Loadout,
) -> crate::Result<ApplyReport> {
    // (1) A studio deployment is off-limits — replacing it would silently drop a hand-built mod.
    if let Some(prev) = crate::read_record(game_root) {
        if prev.owner != "manager" {
            return Err(ModError::Other(format!("STUDIO_DEPLOY_ACTIVE:{}", prev.mod_name)));
        }
    }

    // (2) Reset to pristine: undo whatever the manager had deployed. Tolerate "nothing deployed".
    crate::undeploy(game_root)?;

    // Absolutize like deploy()/undeploy() so every derived + persisted path is absolute.
    let abs_root = crate::abs_root(game_root);
    let gp = crate::resolve_game_paths(&abs_root);

    // (3) Load the enabled entries' metadata, remembering each one's 0-based slot among the
    //     ENABLED entries (drives per-mod `gm{idx:03}` naming / mount order).
    struct Loaded<'a> {
        idx: usize,
        entry: &'a LoadoutEntry,
        dir: PathBuf,
        meta: ModEntryMeta,
    }
    let mut loaded: Vec<Loaded> = Vec::new();
    for entry in loadout.entries.iter().filter(|e| e.enabled) {
        let idx = loaded.len();
        let dir = library_dir.join(&entry.id);
        let meta_path = dir.join(META_FILE);
        let bytes = std::fs::read(&meta_path).map_err(|e| {
            ModError::Other(format!("reading metadata for loadout entry {}: {e}", entry.id))
        })?;
        let meta: ModEntryMeta = serde_json::from_slice(&bytes).map_err(|e| {
            ModError::Other(format!("corrupt metadata for loadout entry {}: {e}", entry.id))
        })?;
        loaded.push(Loaded { idx, entry, dir, meta });
    }

    // (4) EMPTY enabled set: nothing to deploy. We already reset to pristine in (2); leave it that
    //     way (do NOT commit an empty manager record).
    if loaded.is_empty() {
        return Ok(ApplyReport { applied: Vec::new(), warnings: Vec::new() });
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut plan = DeployPlan::default();

    // Accumulators (all later-wins on key collisions, in loadout order):
    //   rawfile bases per target file, then loc/audio/script patches layered on top.
    let mut rawfile_bases: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    let mut loc: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut audio: BTreeMap<(String, String), PathBuf> = BTreeMap::new();
    let mut scripts: Vec<(String, String, Vec<u8>)> = Vec::new();

    // ── PASS 1: additive components + collect rawfile base-overrides ───────────────────────────
    for l in &loaded {
        for comp in &l.meta.components {
            match comp {
                ComponentInfo::Ue4ssLua { name, rel, .. } => {
                    if !crate::is_safe_mod_name(name) || !crate::is_safe_rel_path(rel) {
                        return Err(ModError::Other(format!(
                            "unsafe ue4ss component in {}: name={name:?} rel={rel:?}",
                            l.entry.id
                        )));
                    }
                    let dst = gp.ue4ss_mods.join(format!("gm{:03}_{}", l.idx, name));
                    plan.ue4ss_dirs.push((l.dir.join(rel), dst));
                }
                ComponentInfo::TexturePatch { rel, .. } => {
                    // Cook + pack a Zen triplet; `meta.id` gives cross-mod uniqueness of the pak name.
                    let triplets = crate::prepare_texture_component(&l.dir, rel, &l.meta.id, l.idx, &gp)?;
                    plan.texture_triplets.extend(triplets);
                }
                ComponentInfo::Triplet { rel_base, .. } => {
                    if !crate::is_safe_rel_path(rel_base) {
                        return Err(ModError::Other(format!("unsafe triplet path: {rel_base:?}")));
                    }
                    let stem = slot_pak_stem(rel_base, l.idx);
                    for ext in ["utoc", "ucas", "pak"] {
                        let src = l.dir.join(format!("{rel_base}.{ext}"));
                        if src.is_file() {
                            let dst = mods_dir(&gp).join(format!("{stem}.{ext}"));
                            plan.managed_paks.push((src, dst));
                        }
                    }
                }
                ComponentInfo::LoosePak { rel, .. } => {
                    if !crate::is_safe_rel_path(rel) {
                        return Err(ModError::Other(format!("unsafe loose pak path: {rel:?}")));
                    }
                    let src = l.dir.join(rel);
                    let base = Path::new(rel).file_stem().and_then(|s| s.to_str()).unwrap_or("pak");
                    let stem = slot_stem(base, l.idx);
                    let dst = mods_dir(&gp).join(format!("{stem}.pak"));
                    plan.managed_paks.push((src, dst));
                }
                ComponentInfo::RawFile { rel, target_file } => {
                    if !crate::is_safe_rel_path(rel) {
                        return Err(ModError::Other(format!("unsafe raw file path: {rel:?}")));
                    }
                    let src = l.dir.join(rel);
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
                            if !crate::is_safe_filename(name) {
                                return Err(ModError::Other(format!("unsafe bank name: {name:?}")));
                            }
                            gp.fmod_desktop.join(name)
                        }
                        RawTarget::ScriptCache => gp.script_cache.clone(),
                    };
                    let bytes = std::fs::read(&src)
                        .map_err(crate::io(&format!("reading raw file {}", src.display())))?;
                    // Later mod wins the base for this target.
                    rawfile_bases.insert(target, bytes);
                }
                ComponentInfo::LocPatch { rel, .. } => {
                    if !crate::is_safe_rel_path(rel) {
                        return Err(ModError::Other(format!("unsafe loc patch path: {rel:?}")));
                    }
                    let edits: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_slice(
                        &std::fs::read(l.dir.join(rel)).map_err(crate::io("reading loc edits"))?,
                    )?;
                    for (id, sets) in edits {
                        for (set, text) in sets {
                            loc.insert((id.clone(), set), text); // later-wins
                        }
                    }
                }
                ComponentInfo::AudioPatch { rel, .. } => {
                    if !crate::is_safe_rel_path(rel) {
                        return Err(ModError::Other(format!("unsafe audio patch path: {rel:?}")));
                    }
                    let map: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_slice(
                        &std::fs::read(l.dir.join(rel).join("manifest.json"))
                            .map_err(crate::io("reading audio manifest"))?,
                    )?;
                    for (bank, samples) in map {
                        if !crate::is_safe_filename(&bank) {
                            return Err(ModError::Other(format!("unsafe bank name: {bank:?}")));
                        }
                        for (sample, wav_rel) in samples {
                            if !crate::is_safe_rel_path(&wav_rel) {
                                return Err(ModError::Other(format!("unsafe wav path: {wav_rel:?}")));
                            }
                            audio.insert((bank.clone(), sample), l.dir.join(&wav_rel)); // later-wins
                        }
                    }
                }
                ComponentInfo::AngelScriptPatch { rel, .. } => {
                    if !crate::is_safe_rel_path(rel) {
                        return Err(ModError::Other(format!("unsafe script patch path: {rel:?}")));
                    }
                    let entries: Vec<crate::ScriptEntry> = serde_json::from_slice(
                        &std::fs::read(l.dir.join(rel).join("manifest.json"))
                            .map_err(crate::io("reading script manifest"))?,
                    )?;
                    for e in entries {
                        if !crate::is_safe_rel_path(&e.mini) {
                            return Err(ModError::Other(format!("unsafe mini path: {:?}", e.mini)));
                        }
                        let mini = std::fs::read(l.dir.join(&e.mini))
                            .map_err(crate::io("reading mini-cache"))?;
                        scripts.push((e.op, e.module, mini));
                    }
                }
            }
        }
    }

    // ── PASS 2: materialize patch targets on top of their base ────────────────────────────────
    // `writes` is keyed by target path so a rawfile base and its overlaying patch collapse to ONE
    // final write per file (patch wins), and an un-patched rawfile still lands exactly once.
    let mut writes: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();

    // loc → decode base, set each (id,set), re-encode. Base = a rawfile lcache override if present,
    // else the pristine .lcache.
    if !loc.is_empty() {
        if let Some(lcache) = gp.lcache.clone() {
            let base = match rawfile_bases.get(&lcache) {
                Some(b) => b.clone(),
                None => crate::read_pristine(&lcache, None)?.0,
            };
            let mut lc = gore_loc::loc::Lcache::decode(&base)?;
            for ((id, set), text) in &loc {
                // Best-effort: an id/lang absent from THIS install's .lcache is a warning, not a
                // hard failure (a mod built against a different game version).
                if let Err(e) = lc.set_value(id, set, text) {
                    warnings.push(format!("loc {id}|{set}: {e}"));
                }
            }
            writes.insert(lcache.clone(), lc.encode()?);
            plan.refresh_baks.push(lcache);
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
        let mut by_bank: BTreeMap<String, Vec<(String, PathBuf)>> = BTreeMap::new();
        for ((bank, sample), wav) in &audio {
            by_bank.entry(bank.clone()).or_default().push((sample.clone(), wav.clone()));
        }
        for (bank, samples) in by_bank {
            let bank_path = gp.fmod_desktop.join(&bank);
            let base = match rawfile_bases.get(&bank_path) {
                Some(b) => b.clone(),
                None => crate::read_pristine(&bank_path, None)?.0,
            };
            let mut repl = Vec::with_capacity(samples.len());
            for (sample, wav_path) in samples {
                let wav = std::fs::read(&wav_path).map_err(crate::io("reading patch wav"))?;
                let (rate, ch, pcm) = gore_fmod::read_wav_pcm16(&wav).map_err(ModError::Fmod)?;
                repl.push((
                    sample.clone(),
                    gore_fmod::Pcm16Sample { name: sample, freq: rate, channels: ch, pcm },
                ));
            }
            let new_bank =
                gore_fmod::replace_samples(&base, &fmod_key, repl).map_err(ModError::Fmod)?;
            writes.insert(bank_path.clone(), new_bank);
            plan.refresh_baks.push(bank_path);
        }
    }

    // scripts → fold add/edit onto the script-cache base (rawfile override or pristine cache).
    if !scripts.is_empty() {
        let base = match rawfile_bases.get(&gp.script_cache) {
            Some(b) => b.clone(),
            None => crate::pristine_script_cache(&abs_root)?,
        };
        let mut acc = base;
        for (op, module, mini) in &scripts {
            acc = match op.as_str() {
                "add" => gore_as::cache::splice::splice_auto(&acc, mini)
                    .map_err(|e| ModError::Other(format!("splice {module}: {e}")))?,
                "edit" => gore_as::cache::splice::replace_module(&acc, mini, module)
                    .map_err(|e| ModError::Other(format!("replace {module}: {e}")))?,
                other => {
                    return Err(ModError::Other(format!(
                        "invalid script op {other:?} for module {module:?}"
                    )))
                }
            };
        }
        writes.insert(gp.script_cache.clone(), acc);
        plan.refresh_baks.push(gp.script_cache.clone());
    }

    // Any rawfile whose target was NOT further patched: emit its base bytes verbatim (so every
    // rawfile lands even with no overlaying patch). A patched target is already in `writes`.
    for (target, bytes) in rawfile_bases {
        if !writes.contains_key(&target) {
            plan.refresh_baks.push(target.clone());
            writes.insert(target, bytes);
        }
    }

    plan.writes = writes.into_iter().collect();

    // (6) Commit as a manager-owned deployment. prev = None: the pre-apply undeploy already reset
    //     the game to pristine and removed the old record, so there is no leftover to reconcile.
    //     commit_plan rejects self-colliding dsts and mirrors the manager footprint into the
    //     legacy record fields automatically.
    let record = DeployRecord {
        owner: "manager".into(),
        mod_name: "<manager>".into(),
        loadout: loaded.iter().map(|l| LoadoutEntry { id: l.entry.id.clone(), enabled: true }).collect(),
        ..Default::default()
    };
    crate::commit_plan(&gp, &abs_root, plan, record, None)?;

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

// ── naming helpers ────────────────────────────────────────────────────────────────────────────

/// `<root>/G1R/Content/Paks/~mods` — where manager paks/triplets mount. Derived like the texture
/// arm: `gp.ue4ss_mods` is `<root>/G1R/Binaries/Win64/ue4ss/Mods`, so its 5th ancestor is `<root>`.
fn mods_dir(gp: &crate::GamePaths) -> PathBuf {
    let root = gp.ue4ss_mods.ancestors().nth(5).unwrap_or(&gp.ue4ss_mods);
    root.join("G1R").join("Content").join("Paks").join("~mods")
}

/// Slot-prefixed pak stem for a foreign triplet whose `rel_base` file stem may already carry the
/// shipping `zzz_…_P` decoration: strip a leading `zzz_` and trailing `_P`, then re-wrap as
/// `zzz_gm{idx:03}_{sanitized}_P` so paks sort by loadout slot in `~mods`.
fn slot_pak_stem(rel_base: &str, idx: usize) -> String {
    let raw = Path::new(rel_base).file_stem().and_then(|s| s.to_str()).unwrap_or(rel_base);
    slot_stem(raw, idx)
}

/// Wrap `raw` (a bare mod/pak name) as the slot-prefixed `~mods` stem `zzz_gm{idx:03}_{clean}_P`,
/// where `clean` has any leading `zzz_` / trailing `_P` stripped and is sanitized to a safe stem.
fn slot_stem(raw: &str, idx: usize) -> String {
    let mut s = raw;
    if let Some(rest) = s.strip_prefix("zzz_") {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix("_P") {
        s = rest;
    }
    format!("zzz_gm{:03}_{}_P", idx, sanitize_stem(s))
}

/// Fold anything that isn't `[A-Za-z0-9_-]` to `_` (mirrors lib.rs `sanitize`), so a pak name can't
/// introduce path separators or other unsafe characters into the `~mods` filename.
fn sanitize_stem(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mgr::loadout::Loadout;
    use crate::mgr::model::{ComponentInfo, ModEntryMeta, ModKind, RawTarget};
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
    use aes::Aes256;
    use std::fs;

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
        plain.extend(std::iter::repeat(0u8).take(pad));
        aes_ecb_encrypt(&plain)
    }

    /// Decrypt+decode a `.lcache` and read a single (key, german) value for assertions.
    fn read_loc(bytes: &[u8], key: &str) -> String {
        let lc = gore_loc::loc::Lcache::decode(bytes).unwrap();
        lc.export(false).get(key).and_then(|m| m.get("german")).cloned().unwrap_or_default()
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
            FakeGame { _tmp: tmp, root, lib }
        }

        fn lcache(&self) -> PathBuf {
            self.root.join("G1R/Story/Cache/AlkimiaLocalization_0.lcache")
        }
        fn mods(&self) -> PathBuf {
            self.root.join("G1R/Content/Paks/~mods")
        }
        fn ue4ss(&self) -> PathBuf {
            self.root.join("G1R/Binaries/Win64/ue4ss/Mods")
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
            fs::write(dir.join(META_FILE), serde_json::to_vec_pretty(&meta).unwrap()).unwrap();
            write_payload(&dir);
            id.to_string()
        }

        /// Add a loc-patch mod editing one (id → german) value.
        fn add_loc_mod(&self, id: &str, name: &str, loc_id: &str, value: &str) -> String {
            let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            edits.entry(loc_id.into()).or_default().insert("german".into(), value.into());
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::LocPatch { rel: "loc/edits.json".into(), targets: vec![] }],
                |dir| {
                    fs::create_dir_all(dir.join("loc")).unwrap();
                    fs::write(dir.join("loc/edits.json"), serde_json::to_vec(&edits).unwrap()).unwrap();
                },
            )
        }

        /// Add a loose-pak mod (a single `<stem>.pak`).
        fn add_pak_mod(&self, id: &str, name: &str, pak_stem: &str, bytes: &[u8]) -> String {
            let rel = format!("{pak_stem}.pak");
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::LoosePak { rel: rel.clone(), targets: vec![] }],
                |dir| fs::write(dir.join(&rel), bytes).unwrap(),
            )
        }
    }

    fn loadout(entries: &[(&str, bool)]) -> Loadout {
        Loadout {
            format: 1,
            entries: entries
                .iter()
                .map(|(id, en)| LoadoutEntry { id: (*id).into(), enabled: *en })
                .collect(),
        }
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
        assert_eq!(report.applied, vec!["Alpha".to_string(), "Bravo".to_string()]);
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);

        let live = fs::read(g.lcache()).unwrap();
        assert_eq!(read_loc(&live, "itfo_cheese"), "Brie", "later mod must win");
        assert_eq!(read_loc(&live, "itfo_apple"), "Apple", "untouched id preserved");
    }

    /// Two loose-pak mods land in `~mods` under per-slot names `zzz_gm000_*_P.pak` and
    /// `zzz_gm001_*_P.pak`, so mount order follows loadout order.
    #[test]
    fn apply_orders_paks_by_slot() {
        let g = FakeGame::new();
        let a = g.add_pak_mod("mod-a", "Alpha", "alpha_P", b"PAK-A");
        let b = g.add_pak_mod("mod-b", "Bravo", "bravo_P", b"PAK-B");
        let lo = loadout(&[(&a, true), (&b, true)]);

        apply_loadout(&g.root, &g.lib, &lo).unwrap();

        let a_dst = g.mods().join("zzz_gm000_alpha_P.pak");
        let b_dst = g.mods().join("zzz_gm001_bravo_P.pak");
        assert!(a_dst.is_file(), "slot-0 pak missing: {}", a_dst.display());
        assert!(b_dst.is_file(), "slot-1 pak missing: {}", b_dst.display());
        assert_eq!(fs::read(&a_dst).unwrap(), b"PAK-A");
        assert_eq!(fs::read(&b_dst).unwrap(), b"PAK-B");
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

        let rec = crate::read_record(&g.root).expect("record written");
        assert_eq!(rec.owner, "manager");
        assert_eq!(
            rec.loadout,
            vec![
                LoadoutEntry { id: "mod-a".into(), enabled: true },
                LoadoutEntry { id: "mod-b".into(), enabled: true },
            ],
            "record loadout = enabled snapshot in order"
        );
        assert_eq!(rec.managed_paks.len(), 1, "one managed pak recorded");
        assert!(rec.managed_paks[0].ends_with("zzz_gm000_alpha_P.pak"));
    }

    /// Applying over an active STUDIO (non-manager) deployment is refused with STUDIO_DEPLOY_ACTIVE
    /// and does not touch the studio record.
    #[test]
    fn apply_refuses_studio_record() {
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Edam");
        // Seed a studio record (owner == "").
        let studio = DeployRecord { mod_name: "SoloMod".into(), ..Default::default() };
        fs::write(crate::record_path(&g.root), serde_json::to_vec(&studio).unwrap()).unwrap();

        let err = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap_err();
        assert!(err.to_string().contains("STUDIO_DEPLOY_ACTIVE"), "got: {err}");
        assert!(err.to_string().contains("SoloMod"));
        // Studio record untouched (guard tripped before any undeploy/commit).
        let after = crate::read_record(&g.root).unwrap();
        assert_eq!(after.mod_name, "SoloMod");
        assert_eq!(after.owner, "");
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
                ComponentInfo::LoosePak { rel: "alpha_P.pak".into(), targets: vec![] },
                ComponentInfo::LocPatch { rel: "loc/edits.json".into(), targets: vec![] },
            ],
            |dir| {
                fs::write(dir.join("alpha_P.pak"), b"PAK-A").unwrap();
                fs::create_dir_all(dir.join("loc")).unwrap();
                let mut e: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
                e.entry("itfo_cheese".into()).or_default().insert("german".into(), "Gouda".into());
                fs::write(dir.join("loc/edits.json"), serde_json::to_vec(&e).unwrap()).unwrap();
            },
        );
        let b = g.add_loc_mod("mod-b", "Bravo", "itfo_cheese", "Brie");

        // Both enabled: mod-b wins, mod-a's pak present.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, true)])).unwrap();
        assert_eq!(read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"), "Brie");
        assert!(g.mods().join("zzz_gm000_alpha_P.pak").is_file());

        // Disable mod-b and re-apply: pristine base → only mod-a's Gouda, mod-a's pak now slot 0.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&b, false)])).unwrap();
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda",
            "must recompute from pristine, not merge onto stale Brie"
        );
        // mod-a stays slot 0 (still the first ENABLED entry).
        assert!(g.mods().join("zzz_gm000_alpha_P.pak").is_file());
        // No orphan from mod-b (it never shipped a pak) and nothing left over.
        let entries: Vec<_> = fs::read_dir(g.mods()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(entries.len(), 1, "exactly mod-a's pak in ~mods: {entries:?}");
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
        assert_ne!(fs::read(g.lcache()).unwrap(), pristine, "deploy changed the lcache");

        assert!(undeploy_all(&g.root).unwrap(), "a deployment was undone");
        assert_eq!(fs::read(g.lcache()).unwrap(), pristine, "pristine restored byte-identical");
        assert!(!crate::record_path(&g.root).exists(), "record removed after clean undeploy");
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
            vec![ComponentInfo::RawFile { rel: "loc.lcache".into(), target_file: RawTarget::Lcache }],
            |dir| fs::write(dir.join("loc.lcache"), &raw_bytes).unwrap(),
        );
        // A loc-patch mod edits cheese on top.
        let patch = g.add_loc_mod("mod-patch", "Patch", "itfo_cheese", "PatchedCheese");
        // Raw first (base), patch second (on top).
        let lo = loadout(&[(&raw, true), (&patch, true)]);

        let report = apply_loadout(&g.root, &g.lib, &lo).unwrap();
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);

        let live = fs::read(g.lcache()).unwrap();
        // apple came from the RAW base (proves the base is the rawfile, not pristine)...
        assert_eq!(read_loc(&live, "itfo_apple"), "RawApple", "base must be the rawfile");
        // ...cheese was patched on top of that base.
        assert_eq!(read_loc(&live, "itfo_cheese"), "PatchedCheese", "loc patch lands on the raw base");
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
        assert_eq!(status(&g.root, &target).unwrap(), ManagerStatus::NothingDeployed);

        // 2) Apply the target → InSync.
        apply_loadout(&g.root, &g.lib, &target).unwrap();
        assert_eq!(
            status(&g.root, &target).unwrap(),
            ManagerStatus::InSync {
                loadout: vec![
                    LoadoutEntry { id: "mod-a".into(), enabled: true },
                    LoadoutEntry { id: "mod-b".into(), enabled: true },
                ]
            }
        );

        // 3) Ask for a different target (mod-b disabled) → ChangesPending.
        let narrowed = loadout(&[(&a, true), (&b, false)]);
        assert!(matches!(
            status(&g.root, &narrowed).unwrap(),
            ManagerStatus::ChangesPending { .. }
        ));

        // 4) Externally truncate a deployed live file → GameUpdated.
        fs::write(g.lcache(), b"").unwrap();
        match status(&g.root, &target).unwrap() {
            ManagerStatus::GameUpdated { drifted } => {
                assert_eq!(drifted.len(), 1);
                assert!(drifted[0].ends_with("AlkimiaLocalization_0.lcache"));
            }
            other => panic!("expected GameUpdated, got {other:?}"),
        }
    }
}
