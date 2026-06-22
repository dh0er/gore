//! `gore-cli deploy-shared` — copy the gore-lua shared/ tree into the game's
//! `ue4ss/Mods/shared/`, so mods can `require("gorelib")`.

use anyhow::{bail, Context, Result};
use std::{fs, path::Path, path::PathBuf};

pub fn run(src: PathBuf, game: PathBuf) -> Result<()> {
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
    if dest_root.starts_with(&src) {
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

fn copy_dir(src: &Path, dest: &Path) -> Result<usize> {
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
            fs::copy(&from, &to).with_context(|| format!("copying {}", from.display()))?;
            count += 1;
        }
    }
    Ok(count)
}
