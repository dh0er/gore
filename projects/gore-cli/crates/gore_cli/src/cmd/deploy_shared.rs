//! `gore-cli deploy-shared` — copy the gore-lua shared/ tree into the game's
//! `ue4ss/Mods/shared/`, so mods can `require("gorelib")`.

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
    // reliably against the canonicalized source), then reject a destination inside the
    // source: copying a dir into its own subtree recurses until the path/disk blows up.
    let dest_real = dest_root.canonicalize().unwrap_or_else(|_| dest_root.clone());
    if dest_real.starts_with(&src) {
        bail!(
            "destination '{}' is inside source '{}' — would copy into itself",
            dest_root.display(),
            src.display()
        );
    }
    let n = copy_dir(&src, &dest_root)?;
    println!("deployed {n} file(s) to {}", dest_root.display());
    Ok(())
}

/// Locate the bundled `shared/` SDK relative to the gore-cli executable, so
/// `deploy-shared` works regardless of the caller's working directory. Tries packaged
/// layouts (next to the binary) first, then the dev tree (repo root above target/).
fn resolve_default_src() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("locating gore-cli executable")?;
    let exe_dir = exe.parent().unwrap_or(Path::new("."));
    let candidates = [
        exe_dir.join("shared"),                  // packaged: shared/ beside the binary
        exe_dir.join("gore-lua").join("shared"), // packaged: gore-lua/shared/ beside binary
        // dev: target/{debug,release}/gore-cli -> repo root/projects/gore-lua/shared
        exe_dir.join("..").join("..").join("projects").join("gore-lua").join("shared"),
    ];
    for c in candidates {
        if c.is_dir() {
            return Ok(c);
        }
    }
    bail!(
        "could not locate the bundled gore-lua shared/ SDK relative to '{}'; \
         pass --src <path-to-gore-lua/shared>",
        exe_dir.display()
    )
}

fn copy_dir(src: &Path, dest: &Path) -> Result<usize> {
    // A pre-existing symlinked destination (anywhere in the tree, e.g. Mods/shared/gorelib ->
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
