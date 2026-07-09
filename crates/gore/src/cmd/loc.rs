//! `gore-cli loc` — read and edit the game's localized text directly from the
//! encrypted AlkimiaLocalization `.lcache` (no game run needed).
//!
//! - `export`: decrypt + flatten to `{ text_id: { language: value } }` for every
//!   id and language the game ships. Consumed by gore-save and gore-mod.
//! - `import`: apply `{ id: { language: value } }` edits and re-encrypt the
//!   `.lcache` (a static text / translation mod). Unedited fields keep their
//!   original bytes, so a no-edit round-trip is byte-identical.

use anyhow::{Context, Result};
use gore_loc::loc::Lcache;
use gore_loc::{config, loc_store, paths};
use std::io::Write as _;
use std::{collections::BTreeMap, fs, path::PathBuf};

type LocMap = BTreeMap<String, BTreeMap<String, String>>;

/// Auto-detect (or use `--lcache`) the game's localization cache and write the
/// shared `gore/loc_catalog.json`. Prompts for confirmation unless `--yes`.
pub fn extract(lcache: Option<PathBuf>, yes: bool) -> Result<()> {
    let resolved = resolve_extract_lcache(lcache)
        .ok_or_else(|| anyhow::anyhow!(
            "no AlkimiaLocalization .lcache found (tried --lcache, the configured \
             game path, then Steam auto-detect). Pass --lcache <path-to-.lcache or game dir>."
        ))?;

    if !yes {
        println!("Extract localized text from:\n  {}", resolved.display());
        println!("into shared catalog:\n  {}", paths::loc_catalog_path().display());
        print!("Proceed? [y/N] ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let meta = loc_store::extract(Some(&resolved)).context("extracting localization")?;
    println!(
        "Extracted {} ids across {} languages -> {}",
        meta.id_count,
        meta.languages.len(),
        meta.catalog_path
    );
    Ok(())
}

/// Resolve the `.lcache` for `extract`, mirroring every other command's game-path
/// precedence: explicit `--lcache` > the configured `game_path` > Steam
/// auto-detect. The configured path is normalized to the install root via
/// [`config::game_root`] (so an exe / `Win64` / intermediate path resolves the
/// same as it does for `mod`/`mgr`/`texture`), and each level falls back to the
/// next when it can't resolve a cache — so a stale configured path never blocks
/// extraction, it just yields to Steam auto-detect.
fn resolve_extract_lcache(lcache: Option<PathBuf>) -> Option<PathBuf> {
    // 1. An explicit --lcache is authoritative: the user pointed us at it.
    if let Some(hint) = lcache {
        return loc_store::resolve_lcache(Some(&hint));
    }
    // 2. The configured game path (else Steam), normalized to the G1R-containing
    //    root exactly like the other commands, then find the cache under it.
    if let Ok(root) = config::game_root(None) {
        if let Some(found) = loc_store::resolve_lcache(Some(&root)) {
            return Some(found);
        }
    }
    // 3. Fall back to a direct Steam `.lcache` scan (covers a stale configured
    //    path, or an install whose root normalization missed but discover finds).
    loc_store::resolve_lcache(None)
}

/// Print whether a shared catalog exists and its provenance.
pub fn status() -> Result<()> {
    // Key off the catalog file (like the apps), so a leftover loc_meta.json
    // without its catalog isn't reported as an extracted catalog.
    if !loc_store::catalog_present() {
        println!(
            "no loc catalog extracted yet -> run `gore-cli loc extract` (shared dir: {})",
            paths::shared_data_dir().display()
        );
        return Ok(());
    }
    match loc_store::status() {
        Some(m) => {
            println!("loc catalog: present");
            println!("  ids:        {}", m.id_count);
            println!("  languages:  {} [{}]", m.languages.len(), m.languages.join(", "));
            println!("  source:     {} ({} bytes)", m.source_path, m.source_bytes);
            println!("  extracted:  {} (unix)", m.extracted_at);
            println!("  path:       {}", m.catalog_path);
        }
        None => {
            // Catalog exists but its metadata doesn't (e.g. a catalog write that
            // succeeded before the meta write failed).
            println!("loc catalog: present (no metadata)");
            println!("  path:       {}", paths::loc_catalog_path().display());
        }
    }
    Ok(())
}

pub fn export(lcache: PathBuf, out: PathBuf, keep_empty: bool) -> Result<()> {
    let enc = fs::read(&lcache)
        .with_context(|| format!("reading lcache '{}'", lcache.display()))?;
    let lc = Lcache::decode(&enc).context("decoding lcache")?;
    let map = lc.export(keep_empty);
    fs::write(&out, serde_json::to_vec(&map).context("serializing")?)
        .with_context(|| format!("writing '{}'", out.display()))?;
    println!(
        "Exported {} ids across {} languages [{}] -> {}",
        map.len(),
        lc.languages().len(),
        lc.languages().join(", "),
        out.display()
    );
    Ok(())
}

pub fn import(lcache: PathBuf, edits: PathBuf, out: Option<PathBuf>) -> Result<()> {
    let enc = fs::read(&lcache)
        .with_context(|| format!("reading lcache '{}'", lcache.display()))?;
    let mut lc = Lcache::decode(&enc).context("decoding lcache")?;

    let edits_json = fs::read_to_string(&edits)
        .with_context(|| format!("reading edits '{}'", edits.display()))?;
    let edits: LocMap = serde_json::from_str(&edits_json)
        .context("parsing edits (expected {\"id\":{\"lang\":\"text\"}})")?;

    let mut applied = 0usize;
    for (key, langs) in &edits {
        for (lang, text) in langs {
            lc.set_value(key, lang, text)
                .with_context(|| format!("editing {key}/{lang}"))?;
            applied += 1;
        }
    }

    let out_path = out.unwrap_or(lcache);
    // Write via temp + rename so an interrupted/failed write never truncates the
    // only game .lcache in place (import overwrites it directly without --out).
    let bytes = lc.encode().context("encoding lcache")?;
    loc_store::write_atomic(&out_path, &bytes)
        .with_context(|| format!("writing '{}'", out_path.display()))?;
    println!("Applied {applied} edit(s) -> {}", out_path.display());
    Ok(())
}
