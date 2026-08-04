//! `gore mod` — build/deploy a unified bundle (overrides + loc + audio + voice ZIPs + more).
//! Thin CLI over the `gore-mod` crate; same engine the mod-studio GUI uses via FFI.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// `gore mod build --spec spec.json --out DIR` → write the bundle dir.
pub fn build(spec_path: PathBuf, out: PathBuf) -> Result<()> {
    let json = std::fs::read_to_string(&spec_path)
        .with_context(|| format!("reading spec '{}'", spec_path.display()))?;
    let spec: gore_mod::BuildSpec = serde_json::from_str(&json).context("parsing build spec")?;
    // Asset paths written in the spec are resolved against the SPEC's own directory, exactly like
    // `gore audio replace --map`. A path written next to the spec has to mean the file next to the
    // spec: an agent or GUI that runs this command chooses neither the working directory nor,
    // usually, knows what it is.
    let base = spec_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let bundle = gore_mod::build_bundle_relative_to(&spec, base)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("building bundle from spec '{}'", spec_path.display()))?;
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
pub fn deploy(bundle: PathBuf, game: Option<PathBuf>) -> Result<()> {
    let game = gore_loc::config::game_root(game)?;
    let rec = gore_mod::deploy(&bundle, &game).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "deployed '{}' ({} backup(s))",
        rec.mod_name,
        rec.backups.len()
    );
    Ok(())
}

/// `gore mod undeploy --game ROOT` → restore the active mod's backups.
pub fn undeploy(game: Option<PathBuf>) -> Result<()> {
    let game = gore_loc::config::game_root(game)?;
    match gore_mod::undeploy(&game).map_err(|e| anyhow::anyhow!("{e}"))? {
        Some(rec) => println!(
            "undeployed '{}' ({} restored)",
            rec.mod_name,
            rec.backups.len()
        ),
        None => println!("nothing deployed"),
    }
    Ok(())
}
