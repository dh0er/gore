//! `gore-cli deploy-shared` — copy the gore-lua shared/ tree into the game's
//! `ue4ss/Mods/shared/`, so mods can `require("gore-lua")`.

use anyhow::{bail, Context, Result};
use std::{fs, path::Path, path::PathBuf};

pub fn run(src: Option<PathBuf>, game: PathBuf) -> Result<()> {
    let src = match src {
        Some(s) => s,
        None => resolve_default_src()?,
    };
    if !src.is_dir() {
        bail!("source '{}' is not a directory", src.display());
    }
    let mods = game.join("ue4ss").join("Mods");
    if !mods.is_dir() {
        bail!(
            "'{}' does not look like a game dir (no ue4ss/Mods)",
            game.display()
        );
    }
    // Canonicalize both sides so we can reject a destination that lives inside the
    // source: copying a dir into its own subtree recurses until the path/disk blows up.
    let src = src
        .canonicalize()
        .with_context(|| format!("resolving {}", src.display()))?;
    let dest_root = mods
        .canonicalize()
        .with_context(|| format!("resolving {}", mods.display()))?
        .join("shared");
    // A symlinked destination would be written THROUGH by copy_dir, escaping Mods or looping
    // back into the source; refuse it outright.
    if let Ok(meta) = fs::symlink_metadata(&dest_root) {
        if meta.file_type().is_symlink() {
            bail!(
                "destination '{}' is a symlink; refusing to deploy through it",
                dest_root.display()
            );
        }
    }
    // Resolve an existing destination to its real path (a lexical path can't be compared
    // reliably against the canonicalized source), then reject a destination strictly INSIDE
    // the source: the sibling staging dir would land under src and copy_dir would recurse into
    // it. dest == src is fine — staging is a sibling of src, so the in-place refresh is safe.
    let dest_real = dest_root.canonicalize().unwrap_or_else(|_| dest_root.clone());
    if dest_real != src && dest_real.starts_with(&src) {
        bail!(
            "destination '{}' is inside source '{}' — would copy into itself",
            dest_root.display(),
            src.display()
        );
    }
    // `Mods/shared` is a namespace shared by multiple mods, so we must NOT wipe it — only the
    // top-level entries the SDK actually provides (currently `gore-lua/`). Stage the full copy
    // in a sibling temp first (atomic: a failed copy leaves the old SDK intact), then for each
    // SDK-provided top-level entry replace just that entry under dest_root, leaving unrelated
    // libraries other mods stored there untouched.
    fs::create_dir_all(&dest_root).with_context(|| format!("creating {}", dest_root.display()))?;
    let staging = dest_root.with_file_name(".gore-shared.tmp");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("clearing staging dir {}", staging.display()))?;
    }
    let n = copy_dir(&src, &staging)?;
    for entry in fs::read_dir(&staging).with_context(|| format!("reading {}", staging.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        let dst = dest_root.join(&name);
        // Refuse to clobber a symlinked destination entry (would write outside Mods).
        if fs::symlink_metadata(&dst).is_ok_and(|m| m.file_type().is_symlink()) {
            bail!("destination '{}' is a symlink; refusing to replace it", dst.display());
        }
        if dst.is_dir() {
            fs::remove_dir_all(&dst).with_context(|| format!("clearing {}", dst.display()))?;
        } else if dst.exists() {
            fs::remove_file(&dst).with_context(|| format!("removing {}", dst.display()))?;
        }
        fs::rename(entry.path(), &dst)
            .with_context(|| format!("moving {} -> {}", entry.path().display(), dst.display()))?;
    }
    let _ = fs::remove_dir_all(&staging); // best-effort cleanup of the now-empty staging dir
    println!("deployed {n} file(s) to {}", dest_root.display());
    Ok(())
}

/// Locate the bundled `shared/` SDK relative to the gore-cli executable, so
/// `deploy-shared` works regardless of the caller's working directory. Tries packaged
/// layouts (next to the binary) first, then the dev tree (repo root above target/).
fn resolve_default_src() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("locating gore-cli executable")?;
    let exe_dir = exe.parent().unwrap_or(Path::new("."));
    // Packaged layouts: shared/ beside the binary.
    for c in [exe_dir.join("shared"), exe_dir.join("gore-lua").join("shared")] {
        if c.is_dir() {
            return Ok(c);
        }
    }
    // Dev tree: the `target/` holding the binary can be at the workspace root
    // (`target/debug/`) OR crate-local (`crates/gore_cli/target/debug/`), so
    // the repo root is an unknown number of levels up. Walk ancestors for the SDK.
    for anc in exe_dir.ancestors() {
        let c = anc.join("lua").join("shared");
        if c.is_dir() {
            return Ok(c);
        }
    }
    bail!(
        "could not locate the bundled gore-lua shared/ SDK relative to '{}'; \
         pass --src <path-to-lua/shared>",
        exe_dir.display()
    )
}

fn copy_dir(src: &Path, dest: &Path) -> Result<usize> {
    // A pre-existing symlinked destination (anywhere in the tree, e.g. Mods/shared/gore-lua ->
    // /tmp/elsewhere) would be FOLLOWED by create_dir_all / copy, writing outside Mods. Refuse
    // it; this runs at every recursion level, so nested links are caught too.
    if let Ok(meta) = fs::symlink_metadata(dest) {
        if meta.file_type().is_symlink() {
            bail!(
                "destination '{}' is a symlink; refusing to deploy through it",
                dest.display()
            );
        }
    }
    fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
    let mut count = 0;
    for entry in fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        // Use the entry's own type (does NOT follow symlinks, unlike `Path::is_dir`) and
        // skip symlinks entirely so a linked dir can't loop or escape the source tree.
        let ft = entry.file_type().with_context(|| format!("stat {}", entry.path().display()))?;
        if ft.is_symlink() {
            continue;
        }
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if ft.is_dir() {
            count += copy_dir(&from, &to)?;
        } else {
            // Refuse to write through an existing symlinked file destination.
            if fs::symlink_metadata(&to).is_ok_and(|m| m.file_type().is_symlink()) {
                bail!(
                    "destination '{}' is a symlink; refusing to deploy through it",
                    to.display()
                );
            }
            fs::copy(&from, &to).with_context(|| format!("copying {}", from.display()))?;
            count += 1;
        }
    }
    Ok(count)
}
