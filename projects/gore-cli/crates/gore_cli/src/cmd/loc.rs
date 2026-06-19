//! `gore-cli loc` — read and edit the game's localized text directly from the
//! encrypted AlkimiaLocalization `.lcache` (no game run needed).
//!
//! - `export`: decrypt + flatten to `{ text_id: { language: value } }` for every
//!   id and language the game ships. Consumed by gore-save and gore-mod.
//! - `import`: apply `{ id: { language: value } }` edits and re-encrypt the
//!   `.lcache` (a static text / translation mod). Unedited fields keep their
//!   original bytes, so a no-edit round-trip is byte-identical.

use anyhow::{Context, Result};
use gore_core::loc::Lcache;
use std::{collections::BTreeMap, fs, path::PathBuf};

type LocMap = BTreeMap<String, BTreeMap<String, String>>;

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
    fs::write(&out_path, lc.encode().context("encoding lcache")?)
        .with_context(|| format!("writing '{}'", out_path.display()))?;
    println!("Applied {applied} edit(s) -> {}", out_path.display());
    Ok(())
}
