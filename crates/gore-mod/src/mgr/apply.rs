//! Declarative apply: realize an enabled loadout into ONE manager-owned deployment.
//!
//! [`apply_loadout`] composes every enabled library mod's components into a single [`DeployPlan`]
//! and commits it through the same crash-safe machinery the single-bundle deploy uses. It always
//! rebuilds from pristine, so a toggle/reorder is realized by re-applying (never an incremental
//! patch on top of old state). Crucially it builds the ENTIRE (fallible) plan FIRST — reading each
//! pristine base from the prior deployment's backups — and only undeploys+commits once that plan is
//! complete, so a bad/missing/undecodable mod fails the apply without first wiping the working
//! deployment.
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
    // (1) Read the prior manager deployment (if any). A studio deployment is off-limits — replacing
    //     it would silently drop a hand-built mod. We KEEP `prev`: the plan below reads pristine
    //     bytes from its backups while that deployment is still live, and we only undeploy once the
    //     full (fallible) plan is built — so a bad/missing/undecodable mod fails the apply WITHOUT
    //     first wiping the working deployment (see the deferred undeploy before commit_plan).
    let prev = crate::read_record(game_root);
    if let Some(p) = &prev {
        if p.owner != "manager" {
            return Err(ModError::Other(format!("STUDIO_DEPLOY_ACTIVE:{}", p.mod_name)));
        }
    }

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

    // (4) EMPTY enabled set: nothing to deploy. Reset to pristine and leave it that way (do NOT
    //     commit an empty manager record). This is the one branch that undeploys without a rebuild.
    if loaded.is_empty() {
        crate::undeploy(game_root)?;
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
                // Read pristine from the PRIOR deployment's backup (via `prev`) — the live file is
                // still the prior-modded one until the deferred undeploy below.
                None => crate::read_pristine(&lcache, prev.as_ref())?.0,
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
                // Pristine from the prior deployment's backup (live is still modded until undeploy).
                None => crate::read_pristine(&bank_path, prev.as_ref())?.0,
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

    // (6) The full plan is built — every fallible read/decode/cook above succeeded, so the prior
    //     deployment is still intact if we got here. NOW reset to pristine and commit the new
    //     manager deployment. Deferring the undeploy to this point is what makes a failed apply
    //     non-destructive (a bad mod errors out above without wiping the working deploy). prev = None
    //     to commit_plan: the undeploy just removed the old record + backups, so the post-undeploy
    //     live is pristine and there is no leftover for commit_plan to reconcile. commit_plan rejects
    //     self-colliding dsts and mirrors the manager footprint into the legacy record fields.
    crate::undeploy(game_root)?;
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
        while b.len() % 32 != 0 {
            b.push(0);
        }
        let fsb5_abs = b.len() as u32;
        b.extend_from_slice(&fsb5);
        let snd_size = (b.len() - (snd_size_pos + 4)) as u32;
        b[snd_size_pos..snd_size_pos + 4].copy_from_slice(&u32b(snd_size));

        b[sndh_entry..sndh_entry + 4].copy_from_slice(&u32b(fsb5_abs));
        b[sndh_entry + 4..sndh_entry + 8].copy_from_slice(&u32b(fsb5.len() as u32));
        let riff = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&u32b(riff));
        // Sanity: our synthetic bank satisfies the exact gate the audio arm relies on.
        assert!(gore_fmod::is_pristine_bank(&b), "fixture bank must be pristine");
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
        blk[start..end].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect()
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
        /// `<root>/G1R/Content/FMOD/Desktop/<name>` — where `RawTarget::Bank`/audio patches land.
        fn bank(&self, name: &str) -> PathBuf {
            self.root.join("G1R/Content/FMOD/Desktop").join(name)
        }
        /// `<root>/G1R/Script/PrecompiledScript_Shipping.Cache` — the script-cache live target.
        fn script_cache(&self) -> PathBuf {
            self.root.join("G1R/Script/PrecompiledScript_Shipping.Cache")
        }

        /// Add an audio-patch mod: `audio/manifest.json` = {bank:{sample:wav_rel}} plus the WAV.
        fn add_audio_mod(&self, id: &str, name: &str, bank: &str, sample: &str, wav: &[u8]) -> String {
            let wav_rel = format!("audio/0_{sample}.wav");
            let mut manifest: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            manifest.entry(bank.into()).or_default().insert(sample.into(), wav_rel.clone());
            self.add_mod(
                id,
                name,
                vec![ComponentInfo::AudioPatch { rel: "audio".into(), targets: vec![] }],
                |dir| {
                    fs::create_dir_all(dir.join("audio")).unwrap();
                    fs::write(dir.join("audio/manifest.json"), serde_json::to_vec(&manifest).unwrap())
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
                vec![ComponentInfo::AngelScriptPatch { rel: "scripts".into(), targets: vec![] }],
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
                vec![ComponentInfo::RawFile { rel: "raw.bin".into(), target_file: target }],
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

    /// A re-apply that fails while BUILDING the plan (a mod with an undecodable payload) must not
    /// touch the previously-applied manager deployment — the undeploy is deferred until the whole
    /// plan is built, so the prior record + deployed content survive the failure.
    #[test]
    fn failed_apply_preserves_prior_deployment() {
        let g = FakeGame::new();
        let a = g.add_loc_mod("mod-a", "Alpha", "itfo_cheese", "Gouda");
        // First apply succeeds: a manager deployment of [a] is live.
        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_eq!(read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"), "Gouda");

        // A mod whose loc payload is corrupt JSON: PASS 1 fails to parse it → apply errors before
        // the deferred undeploy.
        let bad = g.add_mod(
            "bad",
            "Bad",
            vec![ComponentInfo::LocPatch { rel: "loc/edits.json".into(), targets: vec![] }],
            |dir| {
                fs::create_dir_all(dir.join("loc")).unwrap();
                fs::write(dir.join("loc/edits.json"), b"{ not valid json").unwrap();
            },
        );

        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true), (&bad, true)]))
            .expect_err("a corrupt mod payload must fail the apply");

        // Prior deployment intact: record still manager-owned with loadout [a], and the live
        // .lcache still carries mod-a's edit (NOT reverted to pristine by an early undeploy).
        let rec = crate::read_record(&g.root).expect("prior record must survive a failed apply");
        assert_eq!(rec.owner, "manager");
        assert_eq!(rec.loadout, vec![LoadoutEntry { id: "mod-a".into(), enabled: true }]);
        assert_eq!(
            read_loc(&fs::read(g.lcache()).unwrap(), "itfo_cheese"),
            "Gouda",
            "prior deployment content must remain after a failed re-apply"
        );
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
        let orig: Vec<i16> = (0..64).map(|i| (i as i32 * 300 - 9000) as i16).collect();
        let bank = build_pristine_bank("shout", 44100, &orig, key);
        fs::write(g.bank("Voice.bank"), &bank).unwrap();
        // Precondition: the live bank really does decode to the original pattern.
        assert_eq!(decode_last_fsb5_pcm(&fs::read(g.bank("Voice.bank")).unwrap(), key), orig);

        // The replacement WAV carries a DIFFERENT known pattern (also a different length).
        let repl: Vec<i16> = (0..80).map(|i| (12000 - i as i32 * 250) as i16).collect();
        let wav = gore_fmod::wav_pcm16(44100, 1, &repl);
        let a = g.add_audio_mod("mod-audio", "AudioMod", "Voice.bank", "shout", &wav);

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert_eq!(report.applied, vec!["AudioMod".to_string()]);
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);

        // Decode the LIVE bank: "shout" now carries the injected pattern, not the original.
        let live = fs::read(g.bank("Voice.bank")).unwrap();
        assert!(!gore_fmod::is_pristine_bank(&live), "modded bank has a 2nd FSB5");
        let got = decode_last_fsb5_pcm(&live, key);
        assert_eq!(got, repl, "live bank must carry the INJECTED sample PCM");
        assert_ne!(got, orig, "injected sample must differ from the original");
    }

    /// A `RawFile{Bank}` supplies the whole bank BASE; an AudioPatch then injects on top of it —
    /// mirroring the loc rawfile-then-patch layering for audio. The base bank's sample starts as
    /// one pattern; the final live bank must carry the patch's pattern.
    #[test]
    fn apply_audio_rawfile_bank_is_base_then_patched() {
        let g = FakeGame::new();
        let key = gore_fmod::GOTHIC_STUDIO_KEY;
        // Live pristine bank (pattern P0). The rawfile base will OVERRIDE this with pattern P1.
        fs::write(
            g.bank("Voice.bank"),
            build_pristine_bank("shout", 44100, &[0i16; 64], key),
        )
        .unwrap();
        let base_pat: Vec<i16> = (0..64).map(|i| (i as i32 * 100) as i16).collect();
        let raw_bank = build_pristine_bank("shout", 44100, &base_pat, key);
        let raw = g.add_rawfile_mod(
            "mod-rawbank",
            "RawBank",
            RawTarget::Bank { name: "Voice.bank".into() },
            &raw_bank,
        );
        // Patch injects pattern P2 on top of the rawfile base.
        let patch_pat: Vec<i16> = (0..48).map(|i| (7000 - i as i32 * 100) as i16).collect();
        let wav = gore_fmod::wav_pcm16(44100, 1, &patch_pat);
        let patch = g.add_audio_mod("mod-audiopatch", "AudioPatch", "Voice.bank", "shout", &wav);

        // Raw first (base), patch second (on top).
        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&raw, true), (&patch, true)])).unwrap();
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);

        let got = decode_last_fsb5_pcm(&fs::read(g.bank("Voice.bank")).unwrap(), key);
        assert_eq!(got, patch_pat, "patch pattern must win over the rawfile base");
        assert_ne!(got, base_pat, "must not be the un-patched rawfile base");
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
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);

        // The LIVE cache now has 2 modules including the added one — proving the splice ran on disk.
        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(module_count(&live), 2, "add must bump the module count on the live cache");
        let names = module_names(&live).unwrap();
        assert_eq!(names, vec!["_gore_base".to_string(), "_gore_added".to_string()]);
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
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);

        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(module_count(&live), 2, "edit keeps the module count");
        let names = module_names(&live).unwrap();
        assert!(names.contains(&"_gore_keep".to_string()), "kept module preserved: {names:?}");
        assert!(names.contains(&"_gore_new".to_string()), "replacement present: {names:?}");
        assert!(!names.contains(&"_gore_old".to_string()), "old module replaced: {names:?}");
    }

    /// The script-cache arm's rawfile path: a `RawFile{ScriptCache}` with arbitrary bytes and NO
    /// script patches is written to the live cache VERBATIM (base with no overlay). This exercises
    /// the deterministic orchestration of the script-cache target without needing real gore-as
    /// bytes — the whole-file replacement that a script rawfile performs.
    #[test]
    fn apply_scripts_rawfile_written_verbatim() {
        let g = FakeGame::new();
        // A live pristine cache that must be fully replaced by the rawfile.
        fs::write(g.script_cache(), b"PRISTINE-CACHE-BYTES").unwrap();
        let raw = b"\x00\x01\x02RAW-SCRIPT-CACHE\xff\xfe".to_vec();
        let a = g.add_rawfile_mod("mod-rawsc", "RawSc", RawTarget::ScriptCache, &raw);

        let report = apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();
        assert!(report.warnings.is_empty(), "warnings: {:?}", report.warnings);
        assert_eq!(
            fs::read(g.script_cache()).unwrap(),
            raw,
            "rawfile ScriptCache must be written verbatim as the live cache"
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
        let (Ok(cache), Ok(mini)) =
            (std::env::var("GORE_TEST_CACHE"), std::env::var("GORE_TEST_MINI"))
        else {
            eprintln!("skip: set GORE_TEST_CACHE and GORE_TEST_MINI to real cache + 1-module mini");
            return;
        };
        let base = fs::read(&cache).expect("read real cache");
        let mini_bytes = fs::read(&mini).expect("read real mini");
        let before = module_count(&base);
        let new_name = module_names(&mini_bytes).unwrap().into_iter().next().unwrap();

        let g = FakeGame::new();
        fs::write(g.script_cache(), &base).unwrap();
        let a = g.add_script_mod("mod-as-real", "AsReal", "add", &new_name, &mini_bytes);

        apply_loadout(&g.root, &g.lib, &loadout(&[(&a, true)])).unwrap();

        let live = fs::read(g.script_cache()).unwrap();
        assert_eq!(module_count(&live), before + 1, "real splice adds exactly one module");
        assert!(module_names(&live).unwrap().contains(&new_name), "added module present");
    }
}
