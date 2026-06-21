//! `gore-cli deploy-shared` — copy the gore-lua shared/ tree into the game's
//! `ue4ss/Mods/shared/`, so mods can `require("gorelib")`.

use anyhow::{bail, Context, Result};
use std::{fs, path::Path, path::PathBuf};

pub fn run(src: PathBuf, game: PathBuf) -> Result<()> {
    if !src.is_dir() {
        bail!("source '{}' is not a directory", src.display());
    }
    let dest_root = game.join("ue4ss").join("Mods").join("shared");
    if !game.join("ue4ss").join("Mods").is_dir() {
        bail!(
            "'{}' does not look like a game dir (no ue4ss/Mods)",
            game.display()
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
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            count += copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).with_context(|| format!("copying {}", from.display()))?;
            count += 1;
        }
    }
    Ok(count)
}
