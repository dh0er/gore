use std::path::PathBuf;

use anyhow::{Context, Result};
use gore_story_catalog::{
    build_known_catalog, publish_catalog_atomic, GenerationInputLimits, GenerationPaths,
};

pub fn run(executable: PathBuf, cache: PathBuf, binds: PathBuf, out: PathBuf) -> Result<()> {
    let paths = GenerationPaths {
        executable,
        shipping_cache: cache,
        binds_cache: binds,
    };
    let catalog = build_known_catalog(&paths, GenerationInputLimits::default()).with_context(|| {
        format!(
            "failed to build a sealed story catalog from executable {:?}, Shipping cache {:?}, and Binds cache {:?}",
            paths.executable, paths.shipping_cache, paths.binds_cache
        )
    })?;
    publish_catalog_atomic(&out, &catalog)
        .with_context(|| format!("failed to publish story catalog to {out:?}"))?;

    println!(
        "wrote story_catalog.v1 to {} ({} NPCs, {} quest parents, generation {}, catalog sha256 {})",
        out.display(),
        catalog.npc_count(),
        catalog.quest_parent_count(),
        catalog.generation().edition,
        catalog.catalog_seal().sha256,
    );
    Ok(())
}
