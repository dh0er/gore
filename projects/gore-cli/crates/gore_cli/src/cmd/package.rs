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

    // Create zip
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let zip_file = File::create(&out)
        .with_context(|| format!("creating zip '{}'", out.display()))?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Walk mod_dir recursively and add every file
    add_dir_to_zip(&mut zip, &mod_dir, &mod_dir, options)?;

    zip.finish().context("finalizing zip")?;
    println!("Packaged mod -> {}", out.display());
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base)
            .expect("path must be under base");
        // Use forward slashes inside zip (cross-platform)
        let zip_name = relative.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(&zip_name, options)?;
            add_dir_to_zip(zip, base, &path, options)?;
        } else {
            zip.start_file(&zip_name, options)?;
            let content = fs::read(&path)
                .with_context(|| format!("reading '{}'", path.display()))?;
            zip.write_all(&content)?;
        }
    }
    Ok(())
}
