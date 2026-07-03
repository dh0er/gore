//! Import mods into the manager library — plus [`list`]/[`remove`] over the imported entries.
//!
//! [`import`] materializes any supported source (dir, `.zip`, single game file) into a staging
//! dir, detects what it is (a goremod bundle via `gore-mod.json`, else a foreign-mod scan),
//! extracts each component's game-side **targets** (for later conflict analysis), and activates
//! the staged dir as `<library>/<id>/` with a [`META_FILE`] sidecar.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::model::{ComponentInfo, ModEntryMeta, ModKind, RawTarget, META_FILE};
use crate::{Component, ModError, ModManifest, ScriptEntry};

/// Import `source` (a folder, `.zip` archive, or single recognized game file) into the library
/// at `library_dir`, returning the entry metadata that was also written as its sidecar.
///
/// Pipeline: materialize into a `.staging-*` dir under the library (same volume, so activation
/// is a rename) → detect components + extract targets → write the sidecar → swap into place.
/// Re-importing the SAME source (same name AND same source file/dir name) replaces its entry —
/// a mod update — because the id folds both into its hash. Two DIFFERENT mods that happen to
/// share a display name but come from different sources get DISTINCT ids and coexist, rather than
/// one silently clobbering the other.
pub fn import(library_dir: &Path, source: &Path) -> crate::Result<ModEntryMeta> {
    if !source.exists() {
        return Err(ModError::Other(format!("import source not found: {}", source.display())));
    }
    std::fs::create_dir_all(library_dir).map_err(crate::io("creating library dir"))?;

    // Canonical view so `.`/trailing-separator sources still yield a usable name.
    let canon = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let source_name = canon
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .ok_or_else(|| ModError::Other(format!("cannot derive a name from {}", source.display())))?;
    let fallback_name = if canon.is_dir() {
        source_name.clone()
    } else {
        canon.file_stem().and_then(|n| n.to_str()).unwrap_or(&source_name).to_string()
    };

    let staging = library_dir.join(format!(
        ".staging-{}",
        crate::name_hash(&format!(
            "{}|{}|{:?}",
            canon.display(),
            std::process::id(),
            std::time::SystemTime::now()
        ))
    ));
    // Cleans the staging dir on EVERY early-return path; defused only after activation.
    let mut guard = StagingGuard(Some(staging.clone()));
    std::fs::create_dir_all(&staging).map_err(crate::io("creating staging dir"))?;

    materialize(&canon, &staging)?;
    wrap_root_ue4ss(&staging, &fallback_name)?;

    let (manifest, components) = detect(&staging)?;
    if components.is_empty() {
        return Err(ModError::Other(format!(
            "nothing importable recognized in {}",
            source.display()
        )));
    }
    let kind = if manifest.is_some() { ModKind::Goremod } else { foreign_kind(&components) };
    let (name, version, author) = match &manifest {
        Some(m) => (m.mod_meta.name.clone(), m.mod_meta.version.clone(), m.mod_meta.author.clone()),
        None => (fallback_name, String::new(), String::new()),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Fold the display name and the FULL canonical source path into the disambiguating hash: a
    // re-import of the same source path resolves to the same id and replaces the entry (update),
    // while two different mods that share a display name AND a bare filename but live in different
    // directories (e.g. two `mod.zip` in different folders) still get different ids and coexist
    // instead of one silently clobbering the other. The `slug(name)` prefix keeps the dir
    // human-readable.
    let id = format!(
        "{}-{}",
        slug(&name),
        crate::name_hash(&format!("{name}\0{}", canon.display()))
    );
    let meta = ModEntryMeta {
        id: id.clone(),
        kind,
        name,
        version,
        author,
        imported_at: format_utc(now),
        source: source_name,
        components,
    };

    // Sidecar goes into staging BEFORE the swap so the entry appears fully formed — a
    // concurrent `list()` never sees a half-imported dir it would have to skip.
    std::fs::write(staging.join(META_FILE), serde_json::to_vec_pretty(&meta)?)
        .map_err(crate::io("writing entry sidecar"))?;
    let entry_dir = library_dir.join(&id);
    if entry_dir.exists() {
        // Same source (name + source name) ⇒ same id: re-import replaces the previous copy (an
        // update). A different source with the same display name hashes to a different id, so it
        // lands in its own dir here instead of overwriting this one.
        std::fs::remove_dir_all(&entry_dir).map_err(crate::io("replacing existing entry"))?;
    }
    std::fs::rename(&staging, &entry_dir).map_err(crate::io("activating library entry"))?;
    guard.0 = None; // staging IS the entry now — nothing to clean
    Ok(meta)
}

/// Delete library entry `id` (the dir `<library_dir>/<id>`); `Ok(false)` if it doesn't exist.
pub fn remove(library_dir: &Path, id: &str) -> crate::Result<bool> {
    // `id` becomes a path component — refuse anything that could climb out of the library.
    if !crate::is_safe_mod_name(id) {
        return Err(ModError::Other(format!("invalid library entry id {id:?}")));
    }
    let dir = library_dir.join(id);
    let Ok(md) = std::fs::symlink_metadata(&dir) else { return Ok(false) };
    if md.is_dir() {
        std::fs::remove_dir_all(&dir)
            .map_err(crate::io(&format!("removing entry {}", dir.display())))?;
    } else {
        std::fs::remove_file(&dir)
            .map_err(crate::io(&format!("removing entry {}", dir.display())))?;
    }
    Ok(true)
}

/// All library entries, sorted by name. Entries with an unreadable/corrupt sidecar are skipped
/// (with a note on stderr), a missing library dir is an empty library.
pub fn list(library_dir: &Path) -> crate::Result<Vec<ModEntryMeta>> {
    let rd = match std::fs::read_dir(library_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(crate::io(&format!("reading library {}", library_dir.display()))(e)),
    };
    let mut out = Vec::new();
    for entry in rd.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Dot-dirs are transient staging areas (possibly a concurrent import), not entries.
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let parsed = std::fs::read(path.join(META_FILE))
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice::<ModEntryMeta>(&b).map_err(|e| e.to_string()));
        match parsed {
            Ok(meta) => out.push(meta),
            Err(e) => {
                eprintln!("gore-mod: skipping unreadable library entry {}: {e}", path.display());
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    Ok(out)
}

// ── Materialization ─────────────────────────────────────────────────────────

/// Copy/extract `source` into the empty `staging` dir.
fn materialize(source: &Path, staging: &Path) -> crate::Result<()> {
    if source.is_dir() {
        return crate::copy_dir(source, staging);
    }
    if !source.is_file() {
        return Err(ModError::Other(format!(
            "import source is neither a file nor a directory: {}",
            source.display()
        )));
    }
    let file_name = source.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        ModError::Other(format!("source file name is not valid unicode: {}", source.display()))
    })?;
    let ext =
        source.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "zip" => extract_zip(source, staging),
        "7z" | "rar" => Err(ModError::Other(format!(
            "archive format .{ext} not supported — extract manually and import the folder"
        ))),
        "utoc" | "ucas" | "pak" | "lcache" | "bank" | "cache" => {
            std::fs::copy(source, staging.join(file_name))
                .map_err(crate::io(&format!("copying {}", source.display())))?;
            // A container file only works as a set: pull the same-stem siblings along.
            if ext == "utoc" || ext == "ucas" {
                for sib_ext in ["utoc", "ucas", "pak"] {
                    if sib_ext == ext {
                        continue;
                    }
                    let sib = source.with_extension(sib_ext);
                    if let Some(sib_name) = sib.file_name().and_then(|n| n.to_str()) {
                        if sib.is_file() {
                            std::fs::copy(&sib, staging.join(sib_name))
                                .map_err(crate::io(&format!("copying sibling {}", sib.display())))?;
                        }
                    }
                }
            }
            Ok(())
        }
        _ => Err(ModError::Other(format!(
            "unrecognized import source {}: expected a folder, .zip, a pak/utoc container, \
             or a known game file (.lcache/.bank/PrecompiledScript*.Cache)",
            source.display()
        ))),
    }
}

/// Extract a zip into `staging`, refusing any entry whose name could escape it.
fn extract_zip(zip_path: &Path, staging: &Path) -> crate::Result<()> {
    let file = std::fs::File::open(zip_path)
        .map_err(crate::io(&format!("opening zip {}", zip_path.display())))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ModError::Other(format!("reading zip {}: {e}", zip_path.display())))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ModError::Other(format!("reading zip entry {i}: {e}")))?;
        let raw_name = entry.name().to_string();
        let Some(rel) = safe_zip_entry(&raw_name) else {
            return Err(ModError::Other(format!(
                "zip entry {raw_name:?} has an unsafe path (absolute, drive letter, or '..') — \
                 refusing to extract"
            )));
        };
        let dest = staging.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(crate::io("creating zip dir"))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(crate::io("creating zip parent dir"))?;
        }
        let mut out = std::fs::File::create(&dest)
            .map_err(crate::io(&format!("creating {}", dest.display())))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(crate::io(&format!("extracting {raw_name}")))?;
    }
    Ok(())
}

/// Normalized safe relative path for a zip entry, or `None` if it must be rejected
/// (absolute, drive letter, `..`, control chars). Trailing `/` (dir markers) is dropped.
fn safe_zip_entry(name: &str) -> Option<String> {
    let n = name.replace('\\', "/");
    let n = n.trim_end_matches('/');
    if n.is_empty() || n.starts_with('/') || n.contains(':') || n.chars().any(char::is_control) {
        return None;
    }
    if n.split('/').any(|c| c.is_empty() || c == "." || c == "..") {
        return None;
    }
    // Platform-aware second opinion (prefix/root components etc.).
    if !crate::is_safe_rel_path(n) {
        return None;
    }
    Some(n.to_string())
}

/// A source that IS a UE4SS mod (root holds `Scripts/main.lua`) gets nested into a `<name>/`
/// subdir, so entries are uniformly "mod dirs inside the entry" and a later deploy-copy of the
/// mod dir can never drag the sidecar along.
fn wrap_root_ue4ss(staging: &Path, name: &str) -> crate::Result<()> {
    if !staging.join("Scripts").join("main.lua").is_file() {
        return Ok(());
    }
    let tmp = staging.join(".gore-wrap");
    std::fs::create_dir(&tmp).map_err(crate::io("creating wrap dir"))?;
    let entries: Vec<_> = std::fs::read_dir(staging)
        .map_err(crate::io("reading staging"))?
        .filter_map(|e| e.ok())
        .collect();
    for e in entries {
        if e.file_name().to_string_lossy() == ".gore-wrap" {
            continue;
        }
        std::fs::rename(e.path(), tmp.join(e.file_name())).map_err(crate::io("wrapping mod dir"))?;
    }
    // The wrapped dir becomes the UE4SS mod name — keep it a single safe component.
    let safe = if crate::is_safe_mod_name(name) { name.to_string() } else { slug(name) };
    std::fs::rename(&tmp, staging.join(&safe)).map_err(crate::io("naming mod dir"))?;
    Ok(())
}

// ── Detection ───────────────────────────────────────────────────────────────

/// Detect what the staged tree is: a goremod bundle (`gore-mod.json` at the root or nested at
/// most two folders deep — the usual "zip contains a folder" shipping shapes) or foreign files.
fn detect(staging: &Path) -> crate::Result<(Option<ModManifest>, Vec<ComponentInfo>)> {
    if let Some(bundle_dir) = find_manifest_dir(staging) {
        let bytes = std::fs::read(bundle_dir.join("gore-mod.json"))
            .map_err(crate::io("reading gore-mod.json"))?;
        let manifest: ModManifest = serde_json::from_slice(&bytes)?;
        let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
        let prefix = rel_str(staging, &bundle_dir); // "" when the bundle is the staging root
        let comps = goremod_components(&bundle_dir, &prefix, &manifest, &raw)?;
        Ok((Some(manifest), comps))
    } else {
        Ok((None, scan_foreign(staging)?))
    }
}

/// First dir at depth ≤2 (BFS, sorted — deterministic) containing `gore-mod.json`.
fn find_manifest_dir(root: &Path) -> Option<PathBuf> {
    if root.join("gore-mod.json").is_file() {
        return Some(root.to_path_buf());
    }
    let mut level = vec![root.to_path_buf()];
    for _ in 0..2 {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            let mut subs: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect();
            subs.sort();
            for sub in subs {
                if sub.join("gore-mod.json").is_file() {
                    return Some(sub);
                }
                next.push(sub);
            }
        }
        level = next;
    }
    None
}

/// Map a goremod manifest to library components, reading each payload to extract its targets.
/// `prefix` is the bundle dir's path relative to the entry root (rels must resolve from there);
/// `raw` is the manifest's raw JSON, used for fields the current [`Component`] doesn't carry.
fn goremod_components(
    bundle_dir: &Path,
    prefix: &str,
    manifest: &ModManifest,
    raw: &serde_json::Value,
) -> crate::Result<Vec<ComponentInfo>> {
    let raw_comps = raw.get("components").and_then(|v| v.as_array());
    let mut out = Vec::new();
    for (i, comp) in manifest.components.iter().enumerate() {
        // The manifest may come from an untrusted archive: no path may escape the entry dir.
        // (`..` patterns keep this compiling if variants grow extra fields.)
        let comp_path = match comp {
            Component::Ue4ssLua { path, .. }
            | Component::LocPatch { path, .. }
            | Component::AudioPatch { path, .. }
            | Component::TexturePatch { path, .. }
            | Component::AngelScriptPatch { path, .. } => path,
        };
        if !crate::is_safe_rel_path(comp_path) {
            return Err(ModError::Other(format!(
                "unsafe component path in gore-mod.json: {comp_path:?}"
            )));
        }
        out.push(match comp {
            Component::LocPatch { path, .. } => {
                let edits: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path)).map_err(crate::io("reading loc edits"))?,
                )?;
                let mut targets: Vec<String> = edits
                    .iter()
                    .flat_map(|(id, sets)| sets.keys().map(move |set| format!("{id}|{set}")))
                    .collect();
                targets.sort();
                ComponentInfo::LocPatch { rel: join_rel(prefix, path), targets }
            }
            Component::AudioPatch { path, .. } => {
                let map: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path).join("manifest.json"))
                        .map_err(crate::io("reading audio manifest"))?,
                )?;
                let mut targets: Vec<String> = map
                    .iter()
                    .flat_map(|(bank, samples)| samples.keys().map(move |s| format!("{bank}|{s}")))
                    .collect();
                targets.sort();
                ComponentInfo::AudioPatch { rel: join_rel(prefix, path), targets }
            }
            Component::TexturePatch { path, assets, .. } => {
                let mut targets = assets.clone();
                targets.sort();
                ComponentInfo::TexturePatch { rel: join_rel(prefix, path), targets }
            }
            Component::AngelScriptPatch { path, .. } => {
                let entries: Vec<ScriptEntry> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path).join("manifest.json"))
                        .map_err(crate::io("reading script manifest"))?,
                )?;
                let mut targets: Vec<String> = entries.iter().map(|e| e.module.clone()).collect();
                targets.sort();
                ComponentInfo::AngelScriptPatch { rel: join_rel(prefix, path), targets }
            }
            Component::Ue4ssLua { name, path, .. } => {
                // Take `targets` from the RAW json: today's manifests don't have the field, a
                // future gore-mod.json may — this stays correct either way with no compile-time
                // coupling to the Component schema.
                let mut targets: Vec<String> = raw_comps
                    .and_then(|a| a.get(i))
                    .and_then(|c| c.get("targets"))
                    .and_then(|t| t.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                targets.sort();
                let opaque = targets.is_empty();
                ComponentInfo::Ue4ssLua { name: name.clone(), rel: join_rel(prefix, path), targets, opaque }
            }
        });
    }
    Ok(out)
}

/// Walk the staged tree and collect foreign components (deterministic: sorted per dir).
fn scan_foreign(root: &Path) -> crate::Result<Vec<ComponentInfo>> {
    let mut out = Vec::new();
    scan_dir(root, root, 0, &mut out)?;
    Ok(out)
}

/// Deepest directory nesting `scan_dir` will descend into. Real mods are shallow; a cap here
/// bounds the recursion so a symlink loop (or a maliciously deep archive) can't recurse forever
/// / overflow the stack — past the cap we just stop descending (files already at that depth are
/// still classified).
const MAX_SCAN_DEPTH: usize = 16;

fn scan_dir(root: &Path, dir: &Path, depth: usize, out: &mut Vec<ComponentInfo>) -> crate::Result<()> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .map_err(crate::io(&format!("scanning {}", dir.display())))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if path.join("Scripts").join("main.lua").is_file() {
                // A UE4SS Lua mod dir is one opaque component; don't scan inside it.
                let name = e.file_name().to_string_lossy().into_owned();
                out.push(ComponentInfo::Ue4ssLua {
                    name,
                    rel: rel_str(root, &path),
                    targets: Vec::new(),
                    opaque: true,
                });
            } else if depth < MAX_SCAN_DEPTH {
                // Stop descending past the cap — a symlink loop would otherwise recurse forever.
                scan_dir(root, &path, depth + 1, out)?;
            }
        } else if ft.is_file() {
            classify_file(root, &path, out);
        }
    }
    Ok(())
}

/// Classify one foreign file into a component (or nothing). Target extraction is best-effort:
/// an unparsable container still imports, just with an empty (unknown) footprint.
fn classify_file(root: &Path, path: &Path, out: &mut Vec<ComponentInfo>) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return };
    let lower = name.to_ascii_lowercase();
    let rel = rel_str(root, path);
    if lower.starts_with("precompiledscript") && lower.ends_with(".cache") {
        out.push(ComponentInfo::RawFile { rel, target_file: RawTarget::ScriptCache });
    } else if lower.ends_with(".lcache") {
        out.push(ComponentInfo::RawFile { rel, target_file: RawTarget::Lcache });
    } else if lower.ends_with(".bank") {
        out.push(ComponentInfo::RawFile { rel, target_file: RawTarget::Bank { name: name.to_string() } });
    } else if lower.ends_with(".utoc") {
        // Only a complete pair is mountable; a lone .utoc is not importable on its own.
        if path.with_extension("ucas").is_file() {
            let targets = gore_tex::container::list_packages(path).unwrap_or_default();
            out.push(ComponentInfo::Triplet {
                rel_base: rel_str(root, &path.with_extension("")),
                targets,
            });
        }
    } else if lower.ends_with("_p.pak") && !path.with_extension("utoc").is_file() {
        // A pak WITH a sibling .utoc belongs to that triplet, not to a loose-pak component.
        let targets = gore_tex::container::list_pak_files(path).unwrap_or_default();
        out.push(ComponentInfo::LoosePak { rel, targets });
    }
}

/// Kind for a foreign import: the single component class, or Mixed for ≥2 classes.
fn foreign_kind(components: &[ComponentInfo]) -> ModKind {
    let mut classes = std::collections::BTreeSet::new();
    for c in components {
        classes.insert(match c {
            ComponentInfo::Triplet { .. } => 0u8,
            ComponentInfo::LoosePak { .. } => 1,
            ComponentInfo::Ue4ssLua { .. } => 2,
            ComponentInfo::RawFile { .. } => 3,
            _ => 4, // goremod-only shapes never come from the foreign scan
        });
    }
    if classes.len() != 1 {
        return ModKind::ForeignMixed;
    }
    match classes.into_iter().next().unwrap() {
        0 => ModKind::ForeignTriplet,
        1 => ModKind::ForeignPak,
        2 => ModKind::ForeignUe4ss,
        3 => ModKind::ForeignRawfile,
        _ => ModKind::ForeignMixed,
    }
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// `p` relative to `root` as a '/'-separated string (entry-relative component paths).
fn rel_str(root: &Path, p: &Path) -> String {
    match p.strip_prefix(root) {
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => p.display().to_string(),
    }
}

/// Join a bundle-dir prefix (may be "") and a manifest-relative path with '/'.
fn join_rel(prefix: &str, path: &str) -> String {
    let norm = path.replace('\\', "/");
    if prefix.is_empty() {
        norm
    } else {
        format!("{prefix}/{norm}")
    }
}

/// Lowercase alnum+`-` slug of a mod name for the library id (never empty).
fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

/// `secs` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (UTC, RFC 3339) — std-only.
fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's `civil_from_days`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// Removes the staging dir on drop unless defused (`.0 = None`) — covers every failure path.
struct StagingGuard(Option<PathBuf>);

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_bundle, write_bundle, BuildSpec, ModMeta, ScriptModule};
    use gore_modgen::gen::{OverrideValue, SingleOverride};
    use std::fs;
    use std::io::Write as _;

    /// Build + write a real goremod bundle (1 override, 1 loc edit, 1 script module) and
    /// return its dir. The name deliberately has a space: id slugging must handle it.
    fn mk_goremod_bundle(root: &Path) -> PathBuf {
        let mini = root.join("TestModule.mini.cache");
        fs::write(&mini, b"FAKE-MINI-CACHE-BYTES").unwrap();
        let mut loc = BTreeMap::new();
        loc.insert(
            "itfo_cheese".to_string(),
            BTreeMap::from([("german".to_string(), "X".to_string())]),
        );
        let spec = BuildSpec {
            meta: ModMeta { name: "Target Probe".into(), version: "0.9".into(), author: "tester".into() },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ItFo_Apple".into(),
                field: "m_Value".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(500),
            }],
            loc_edits: loc,
            audio: vec![],
            texture: vec![],
            scripts: vec![ScriptModule {
                op: "add".into(),
                module_name: "TestModule".into(),
                mini_cache: mini.display().to_string(),
            }],
        };
        let bundle = build_bundle(&spec).unwrap();
        let bdir = root.join("Target Probe");
        write_bundle(&bdir, &bundle).unwrap();
        bdir
    }

    /// Zip every file under `dir` (names relative to `dir`, '/'-separated), each entry name
    /// prefixed with `prefix` (empty = zip root).
    fn zip_dir_with_prefix(dir: &Path, prefix: &str, zip_path: &Path) {
        fn add(zw: &mut zip::ZipWriter<fs::File>, root: &Path, dir: &Path, prefix: &str) {
            let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let p = e.path();
                if p.is_dir() {
                    add(zw, root, &p, prefix);
                } else {
                    let rel = p
                        .strip_prefix(root)
                        .unwrap()
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/");
                    let name = if prefix.is_empty() { rel } else { format!("{prefix}/{rel}") };
                    zw.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
                    zw.write_all(&fs::read(&p).unwrap()).unwrap();
                }
            }
        }
        let mut zw = zip::ZipWriter::new(fs::File::create(zip_path).unwrap());
        add(&mut zw, dir, dir, prefix);
        zw.finish().unwrap();
    }

    fn assert_goremod_components(meta: &ModEntryMeta, want_prefix: &str) {
        let pre = |s: &str| if want_prefix.is_empty() { s.to_string() } else { format!("{want_prefix}/{s}") };
        let (mut saw_loc, mut saw_as, mut saw_lua) = (false, false, false);
        for c in &meta.components {
            match c {
                ComponentInfo::LocPatch { rel, targets } => {
                    saw_loc = true;
                    assert_eq!(rel, &pre("loc/edits.json"));
                    assert_eq!(targets, &vec!["itfo_cheese|german".to_string()]);
                }
                ComponentInfo::AngelScriptPatch { rel, targets } => {
                    saw_as = true;
                    assert_eq!(rel, &pre("scripts"));
                    assert_eq!(targets, &vec!["TestModule".to_string()]);
                }
                ComponentInfo::Ue4ssLua { name, rel, targets, opaque } => {
                    saw_lua = true;
                    assert_eq!(name, "Target Probe");
                    assert_eq!(rel, &pre("ue4ss/Target Probe"));
                    // Written so it KEEPS passing once gore-mod.json grows a `targets` field
                    // on ue4ss_lua components: opacity must simply track target absence.
                    assert_eq!(*opaque, targets.is_empty());
                }
                other => panic!("unexpected component in goremod import: {other:?}"),
            }
        }
        assert!(saw_loc && saw_as && saw_lua, "missing components: {:?}", meta.components);
    }

    /// [import 1] A goremod bundle DIR imports as kind Goremod with manifest meta and
    /// per-component targets extracted from the payload files.
    #[test]
    fn import_goremod_bundle_dir_extracts_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path());

        let meta = import(&lib, &bdir).unwrap();
        assert_eq!(meta.kind, ModKind::Goremod);
        assert_eq!(meta.name, "Target Probe");
        assert_eq!(meta.version, "0.9");
        assert_eq!(meta.author, "tester");
        assert_eq!(meta.source, "Target Probe");
        assert!(
            meta.id.starts_with("target-probe-") && meta.id.len() == "target-probe-".len() + 8,
            "id: {}",
            meta.id
        );
        assert_goremod_components(&meta, "");

        // The entry dir holds the payload + sidecar; list() round-trips the same meta.
        let entry = lib.join(&meta.id);
        assert!(entry.join(META_FILE).is_file());
        assert!(entry.join("gore-mod.json").is_file());
        assert!(entry.join("loc").join("edits.json").is_file());
        assert_eq!(list(&lib).unwrap(), vec![meta]);
    }

    /// [import 2] The SAME bundle zipped (manifest at zip root) imports to the same CONTENT as the
    /// dir (kind, name, components). The id now differs because it folds in the source name (dir
    /// "Target Probe" vs "Target Probe.zip") — so a dir and its zip are treated as two distinct
    /// sources, which lets both coexist rather than clobbering each other.
    #[test]
    fn import_zip_bundle_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let bdir = mk_goremod_bundle(tmp.path());
        let from_dir = import(&tmp.path().join("lib-a"), &bdir).unwrap();

        let zp = tmp.path().join("Target Probe.zip");
        zip_dir_with_prefix(&bdir, "", &zp);
        let from_zip = import(&tmp.path().join("lib-b"), &zp).unwrap();

        assert_eq!(from_zip.kind, ModKind::Goremod);
        assert_eq!(from_zip.name, from_dir.name);
        // Same slug prefix (same display name), different hash suffix (different source name).
        assert!(from_zip.id.starts_with("target-probe-"));
        assert_ne!(from_zip.id, from_dir.id, "dir vs zip are distinct sources → distinct ids");
        assert_eq!(from_zip.components, from_dir.components);
        assert_eq!(from_zip.source, "Target Probe.zip");
    }

    /// [import 3] A zip whose bundle sits BELOW the root (nested folders, the usual way mods
    /// are shipped) still imports; component rels carry the folder prefix so they resolve
    /// against the entry dir.
    #[test]
    fn import_zip_nested_bundle_prefixes_rels() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path());
        let zp = tmp.path().join("nested.zip");
        zip_dir_with_prefix(&bdir, "Wrap/Sub", &zp);

        let meta = import(&lib, &zp).unwrap();
        assert_eq!(meta.kind, ModKind::Goremod);
        assert_eq!(meta.name, "Target Probe");
        assert_goremod_components(&meta, "Wrap/Sub");
        assert!(lib.join(&meta.id).join("Wrap/Sub/gore-mod.json").is_file());
    }

    /// [import 4] Zip entries that would escape the staging dir (`..`) abort the import,
    /// nothing is extracted outside, and the staging dir is cleaned up.
    #[test]
    fn import_zip_rejects_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let zp = tmp.path().join("evil.zip");
        let mut zw = zip::ZipWriter::new(fs::File::create(&zp).unwrap());
        zw.start_file("../evil.txt", zip::write::SimpleFileOptions::default()).unwrap();
        zw.write_all(b"boo").unwrap();
        zw.finish().unwrap();

        let err = import(&lib, &zp).unwrap_err().to_string();
        assert!(err.contains("evil.txt"), "err: {err}");
        // `..` relative to a staging dir directly under the library would land here:
        assert!(!lib.join("evil.txt").exists());
        assert!(!tmp.path().join("evil.txt").exists());
        // ...and no staging leftovers survive the failed import.
        let leftovers: Vec<_> = fs::read_dir(&lib)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name()).collect())
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
    }

    /// [import 5] A single foreign `*_P.pak` file imports as ForeignPak; a dummy (unparsable)
    /// pak yields empty targets rather than failing the import.
    #[test]
    fn import_foreign_pak_lists_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let pak = tmp.path().join("foo_P.pak");
        fs::write(&pak, b"definitely not a real pak").unwrap();

        let meta = import(&lib, &pak).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignPak);
        assert_eq!(meta.name, "foo_P");
        assert_eq!(meta.source, "foo_P.pak");
        assert_eq!(
            meta.components,
            vec![ComponentInfo::LoosePak { rel: "foo_P.pak".into(), targets: vec![] }]
        );
        assert!(lib.join(&meta.id).join("foo_P.pak").is_file());
    }

    /// [import 6] A `.utoc` + sibling `.ucas` pair is ONE Triplet component (unparsable dummy
    /// container → empty targets, import still succeeds).
    #[test]
    fn import_triplet_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("BarMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("bar.utoc"), b"junk").unwrap();
        fs::write(src.join("bar.ucas"), b"junk").unwrap();

        let meta = import(&lib, &src).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignTriplet);
        assert_eq!(meta.name, "BarMod");
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Triplet { rel_base: "bar".into(), targets: vec![] }]
        );
    }

    /// [import 6b] Importing the `.utoc` FILE directly pulls its same-stem siblings along.
    #[test]
    fn import_utoc_file_copies_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::write(tmp.path().join("bar.utoc"), b"junk").unwrap();
        fs::write(tmp.path().join("bar.ucas"), b"junk").unwrap();
        fs::write(tmp.path().join("bar.pak"), b"junk").unwrap();

        let meta = import(&lib, &tmp.path().join("bar.utoc")).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignTriplet);
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Triplet { rel_base: "bar".into(), targets: vec![] }]
        );
        let entry = lib.join(&meta.id);
        assert!(entry.join("bar.ucas").is_file());
        assert!(entry.join("bar.pak").is_file());
    }

    /// [import 7] All-raw-files dir → ForeignRawfile with one RawFile component per file and
    /// the right live-target mapping; adding a pak to the mix → ForeignMixed.
    #[test]
    fn import_rawfiles_and_mixed() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let raw = tmp.path().join("RawStuff");
        fs::create_dir_all(&raw).unwrap();
        fs::write(raw.join("AlkimiaLocalization_0.lcache"), b"x").unwrap();
        fs::write(raw.join("SFX.bank"), b"x").unwrap();
        fs::write(raw.join("PrecompiledScript_Shipping.Cache"), b"x").unwrap();

        let meta = import(&lib, &raw).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignRawfile);
        assert_eq!(
            meta.components,
            vec![
                ComponentInfo::RawFile {
                    rel: "AlkimiaLocalization_0.lcache".into(),
                    target_file: RawTarget::Lcache,
                },
                ComponentInfo::RawFile {
                    rel: "PrecompiledScript_Shipping.Cache".into(),
                    target_file: RawTarget::ScriptCache,
                },
                ComponentInfo::RawFile {
                    rel: "SFX.bank".into(),
                    target_file: RawTarget::Bank { name: "SFX.bank".into() },
                },
            ]
        );

        let mixed = tmp.path().join("MixedStuff");
        fs::create_dir_all(&mixed).unwrap();
        fs::write(mixed.join("Music.bank"), b"x").unwrap();
        fs::write(mixed.join("extra_P.pak"), b"x").unwrap();
        let meta = import(&lib, &mixed).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignMixed);
        assert_eq!(meta.components.len(), 2);
    }

    /// [import 8] `.7z`/`.rar` are rejected with a "extract manually" pointer.
    #[test]
    fn import_rejects_7z() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let p = tmp.path().join("a.7z");
        fs::write(&p, b"7z\xbc\xaf\x27\x1c").unwrap();
        let err = import(&lib, &p).unwrap_err().to_string();
        assert!(err.contains("extract manually"), "err: {err}");
    }

    /// [import 9] A dir that IS a UE4SS mod (root `Scripts/main.lua`) is wrapped into a named
    /// subdir so the entry stays uniform and the deployable dir excludes the sidecar.
    #[test]
    fn import_ue4ss_mod_dir_wraps_root() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("MyLuaMod");
        fs::create_dir_all(src.join("Scripts")).unwrap();
        fs::write(src.join("Scripts").join("main.lua"), b"-- lua").unwrap();
        fs::write(src.join("enabled.txt"), b"").unwrap();

        let meta = import(&lib, &src).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignUe4ss);
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Ue4ssLua {
                name: "MyLuaMod".into(),
                rel: "MyLuaMod".into(),
                targets: vec![],
                opaque: true,
            }]
        );
        let entry = lib.join(&meta.id);
        assert!(entry.join("MyLuaMod").join("Scripts").join("main.lua").is_file());
        assert!(entry.join("MyLuaMod").join("enabled.txt").is_file());
    }

    /// [import 10] A source with nothing recognizable in it is an error, not an empty entry.
    #[test]
    fn import_empty_dir_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("Nothing");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("readme.txt"), b"hi").unwrap();
        let err = import(&lib, &src).unwrap_err().to_string();
        assert!(err.contains("nothing importable"), "err: {err}");
        // failed import leaves no staging dir behind
        let leftovers: Vec<_> = fs::read_dir(&lib)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name()).collect())
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
    }

    /// [import 11] Re-importing the SAME source (same name + same source dir/file name) REPLACES
    /// its entry (same id, one copy) — a mod update.
    #[test]
    fn reimport_same_source_replaces_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("BarMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("bar.utoc"), b"junk").unwrap();
        fs::write(src.join("bar.ucas"), b"junk").unwrap();

        let a = import(&lib, &src).unwrap();
        let b = import(&lib, &src).unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(list(&lib).unwrap().len(), 1);
    }

    /// [import 11b] Two mods that share a display NAME but come from DIFFERENT sources must get
    /// distinct ids and coexist — otherwise the old name-only id let one silently clobber the
    /// other (data loss). A goremod bundle's name comes from its manifest, so importing the SAME
    /// manifest-name bundle once as a dir ("Target Probe") and once as a differently-named zip
    /// ("other.zip") yields identical display names but different `source`s → different ids.
    #[test]
    fn different_source_same_name_coexist() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path()); // manifest name "Target Probe"

        let from_dir = import(&lib, &bdir).unwrap();
        let zp = tmp.path().join("other.zip");
        zip_dir_with_prefix(&bdir, "", &zp);
        let from_zip = import(&lib, &zp).unwrap();

        assert_eq!(from_dir.name, from_zip.name, "precondition: same display name");
        assert_ne!(from_dir.source, from_zip.source, "precondition: different source");
        assert_ne!(from_dir.id, from_zip.id, "distinct sources must not collide into one id");
        assert_eq!(list(&lib).unwrap().len(), 2, "both must coexist");
    }

    /// [import 11c] The nastier collision the name-only id missed: two DIFFERENT mods that share
    /// both a display name AND a bare filename but live in different directories (`a/mod` vs
    /// `b/mod`). Only the FULL source path disambiguates them; a filename-only hash would give
    /// both the same id and silently clobber the first. Must yield distinct ids and coexist.
    #[test]
    fn same_filename_different_dir_coexist() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let a = tmp.path().join("a").join("mod");
        let b = tmp.path().join("b").join("mod");
        for d in [&a, &b] {
            fs::create_dir_all(d).unwrap();
            fs::write(d.join("bar.utoc"), b"junk").unwrap();
            fs::write(d.join("bar.ucas"), b"junk").unwrap();
        }
        let from_a = import(&lib, &a).unwrap();
        let from_b = import(&lib, &b).unwrap();
        assert_eq!(from_a.name, from_b.name, "precondition: same display name");
        assert_eq!(from_a.source, from_b.source, "precondition: same bare filename");
        assert_ne!(from_a.id, from_b.id, "same-name+filename in different dirs must not collide");
        assert_eq!(list(&lib).unwrap().len(), 2, "both must coexist");
    }

    /// [remove] Deletes exactly the entry dir; absent id → Ok(false); ids that could climb
    /// out of the library are refused.
    #[test]
    fn remove_deletes_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let pak = tmp.path().join("foo_P.pak");
        fs::write(&pak, b"x").unwrap();
        let meta = import(&lib, &pak).unwrap();
        assert!(lib.join(&meta.id).is_dir());

        assert!(remove(&lib, &meta.id).unwrap());
        assert!(!lib.join(&meta.id).exists());
        assert!(!remove(&lib, &meta.id).unwrap(), "second remove must be false");
        assert!(!remove(&lib, "never-existed").unwrap());
        assert!(remove(&lib, "..").is_err(), "path-escaping id must be refused");
    }

    /// [list] Corrupt/unreadable sidecars are skipped (not fatal), non-entries are ignored,
    /// missing library dir is an empty list.
    #[test]
    fn list_skips_corrupt_meta() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        assert_eq!(list(&lib).unwrap(), vec![], "missing library dir");

        let src = tmp.path().join("GoodMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("stuff.lcache"), b"x").unwrap();
        let good = import(&lib, &src).unwrap();

        let broken = lib.join("zz-broken");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join(META_FILE), b"{ this is not json").unwrap();
        let no_meta = lib.join("zz-no-meta");
        fs::create_dir_all(&no_meta).unwrap();
        fs::write(lib.join("stray.txt"), b"x").unwrap();

        let all = list(&lib).unwrap();
        assert_eq!(all.len(), 1, "only the good entry: {all:?}");
        assert_eq!(all[0], good);
    }

    /// [import 12] A pathologically DEEP source tree imports without hanging or overflowing the
    /// stack: `scan_dir` stops descending past `MAX_SCAN_DEPTH`. A recognizable file is placed
    /// ABOVE the cap so the import still succeeds (finds something) rather than erroring empty.
    #[test]
    fn scan_dir_depth_capped() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("DeepMod");
        fs::create_dir_all(&src).unwrap();
        // A rawfile near the top guarantees a component regardless of the deep tree below.
        fs::write(src.join("top.lcache"), b"x").unwrap();
        // Build a 20-deep nested chain (deeper than the depth cap of 16).
        let mut d = src.clone();
        for i in 0..20 {
            d = d.join(format!("d{i}"));
            fs::create_dir_all(&d).unwrap();
        }
        // A file only reachable BELOW the cap: it must not be classified (proves we stopped),
        // but its presence must not break the scan either.
        fs::write(d.join("buried.lcache"), b"x").unwrap();

        let meta = import(&lib, &src).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignRawfile);
        // Exactly the top-level rawfile was collected; the buried one past the cap was skipped.
        assert_eq!(
            meta.components,
            vec![ComponentInfo::RawFile { rel: "top.lcache".into(), target_file: RawTarget::Lcache }]
        );
    }

    /// The epoch→RFC3339 formatter, incl. a leap day and a modern date.
    #[test]
    fn utc_timestamp_formats_correctly() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(1_000_000_000), "2001-09-09T01:46:40Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_767_225_600), "2026-01-01T00:00:00Z");
    }
}
