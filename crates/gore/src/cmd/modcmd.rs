//! `gore mod` — build / deploy / undeploy a unified mod bundle (overrides + loc + audio).
//! Thin CLI over the `gore-mod` crate; same engine the mod-studio GUI uses via FFI.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// `gore mod build --spec spec.json --out DIR` → write the bundle dir.
pub fn build(spec: PathBuf, out: PathBuf) -> Result<()> {
    let json = std::fs::read_to_string(&spec)
        .with_context(|| format!("reading spec '{}'", spec.display()))?;
    let spec: gore_mod::BuildSpec = serde_json::from_str(&json).context("parsing build spec")?;
    let bundle = gore_mod::build_bundle(&spec).map_err(|e| anyhow::anyhow!("{e}"))?;
    let dir = out.join(&spec.meta.name);
    gore_mod::write_bundle(&dir, &bundle).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "built bundle: {} ({} components, {} files)",
        dir.display(),
        bundle.manifest.components.len(),
        bundle.files.len()
    );
    Ok(())
}

/// `gore mod deploy --bundle DIR --game ROOT` → apply to the game install.
pub fn deploy(bundle: PathBuf, game: PathBuf) -> Result<()> {
    let rec = gore_mod::deploy(&bundle, &game).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("deployed '{}' ({} backup(s))", rec.mod_name, rec.backups.len());
    Ok(())
}

/// `gore mod undeploy --game ROOT` → restore the active mod's backups.
pub fn undeploy(game: PathBuf) -> Result<()> {
    match gore_mod::undeploy(&game).map_err(|e| anyhow::anyhow!("{e}"))? {
        Some(rec) => println!("undeployed '{}' ({} restored)", rec.mod_name, rec.backups.len()),
        None => println!("nothing deployed"),
    }
    Ok(())
}
