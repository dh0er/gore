use anyhow::{bail, Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipWriter};

pub fn run(mod_dir: PathBuf, out: PathBuf) -> Result<()> {
    // Validate required files
    let enabled_txt = mod_dir.join("enabled.txt");
    let main_lua = mod_dir.join("Scripts").join("main.lua");

    if !enabled_txt.exists() {
        bail!("mod missing required file 'enabled.txt' in '{}'", mod_dir.display());
    }
    if !main_lua.exists() {
        bail!("mod missing required file 'Scripts/main.lua' in '{}'", mod_dir.display());
    }

    // The final path component is the mod name, used as the top directory
    // inside the zip so users can extract straight into ue4ss/Mods/.
    let mod_name = mod_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Mod".to_string());

    // Create zip
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let zip_file = File::create(&out)
        .with_context(|| format!("creating zip '{}'", out.display()))?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Walk mod_dir recursively and add every file, prefixed with mod_name/
    add_dir_to_zip(&mut zip, &mod_dir, &mod_dir, &mod_name, options)?;

    zip.finish().context("finalizing zip")?;
    println!("Packaged mod -> {}", out.display());
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    base: &Path,
    dir: &Path,
    mod_name: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base)
            .expect("path must be under base");
        // Use forward slashes inside zip (cross-platform), prefixed with mod_name/
        let relative_str = relative.to_string_lossy().replace('\\', "/");
        let zip_name = format!("{mod_name}/{relative_str}");

        if path.is_dir() {
            zip.add_directory(&zip_name, options)?;
            add_dir_to_zip(zip, base, &path, mod_name, options)?;
        } else {
            zip.start_file(&zip_name, options)?;
            let content = fs::read(&path)
                .with_context(|| format!("reading '{}'", path.display()))?;
            zip.write_all(&content)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod package_tests {
    use super::*;
    use tempfile::TempDir;
    use zip::ZipArchive;

    #[test]
    fn zip_entries_are_prefixed_with_mod_name() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        let scripts_dir = mod_dir.join("Scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(mod_dir.join("enabled.txt"), "").unwrap();
        fs::write(scripts_dir.join("main.lua"), "-- lua").unwrap();

        let out_zip = tmp.path().join("MyMod.zip");
        run(mod_dir, out_zip.clone()).unwrap();

        let zip_file = File::open(&out_zip).unwrap();
        let mut archive = ZipArchive::new(zip_file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();

        assert!(
            names.iter().any(|n| n == "MyMod/enabled.txt"),
            "expected MyMod/enabled.txt, got: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "MyMod/Scripts/main.lua"),
            "expected MyMod/Scripts/main.lua, got: {names:?}"
        );
        // Must NOT have root-level entries
        assert!(
            !names.iter().any(|n| n == "enabled.txt"),
            "unexpected root-level enabled.txt: {names:?}"
        );
    }
}
