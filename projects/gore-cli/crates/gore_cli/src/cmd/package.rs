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

    // Require REAL files (non-following metadata): a directory, dangling, or
    // symlinked path must not pass. The walker skips symlinks, so a symlinked
    // required file would otherwise validate but be omitted from the zip.
    if !is_real_file(&enabled_txt) {
        bail!("mod missing required real file 'enabled.txt' in '{}'", mod_dir.display());
    }
    if !is_real_file(&main_lua) {
        bail!("mod missing required real file 'Scripts/main.lua' in '{}'", mod_dir.display());
    }

    // The final path component is the mod name, used as the top directory
    // inside the zip so users can extract straight into ue4ss/Mods/. Canonicalize
    // first so `package .` / `./` resolves to the real directory name instead of
    // falling back to "Mod" (file_name() of "." is None).
    let mod_dir_abs = fs::canonicalize(&mod_dir).unwrap_or_else(|_| mod_dir.clone());
    let mod_name = mod_dir_abs
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Mod".to_string());

    // Create zip
    if let Some(parent) = out.parent() {
        // Some("") for a bare filename like `-o MyMod.zip`; skip empty parents.
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    // Refuse an output path inside the mod dir: File::create would truncate a
    // source file (e.g. `-o MyMod/Scripts/main.lua`) before the walk, corrupting
    // the mod while still "succeeding".
    let out_parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let out_parent_abs = fs::canonicalize(&out_parent).unwrap_or(out_parent);
    if out_parent_abs.starts_with(&mod_dir_abs) {
        bail!(
            "output path '{}' is inside the mod directory '{}'; choose a location outside it",
            out.display(),
            mod_dir.display()
        );
    }
    let zip_file = File::create(&out)
        .with_context(|| format!("creating zip '{}'", out.display()))?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // The zip was just created; if it lives inside mod_dir, exclude it from the
    // walk so the archive doesn't try to package itself.
    let out_canon = fs::canonicalize(&out).ok();

    // Walk mod_dir recursively and add every file, prefixed with mod_name/
    add_dir_to_zip(&mut zip, &mod_dir, &mod_dir, &mod_name, options, out_canon.as_deref())?;

    zip.finish().context("finalizing zip")?;
    println!("Packaged mod -> {}", out.display());
    Ok(())
}

/// True only for a real regular file (not a directory, not a symlink). Uses
/// symlink_metadata so a symlinked path is rejected (the walker skips symlinks).
fn is_real_file(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|m| m.is_file()).unwrap_or(false)
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    base: &Path,
    dir: &Path,
    mod_name: &str,
    options: zip::write::SimpleFileOptions,
    out_skip: Option<&Path>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        // Skip symlinks: is_dir() would follow them and could walk outside the
        // mod dir or loop back into it. Use the entry's own (non-following) type.
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            eprintln!("skipping symlink in mod dir: {}", path.display());
            continue;
        }
        // Never package the output archive into itself.
        if let Some(skip) = out_skip {
            if fs::canonicalize(&path).ok().as_deref() == Some(skip) {
                continue;
            }
        }
        let relative = path
            .strip_prefix(base)
            .with_context(|| format!("'{}' is not under '{}'", path.display(), base.display()))?;
        // Build the forward-slash zip path from real path COMPONENTS rather than
        // string-replacing '\\' -> '/'. On Unix a filename may legally contain a
        // backslash; naive replacement could manufacture a `..` traversal token
        // (zip-slip). Reject anything that isn't a normal component.
        let mut parts = Vec::new();
        for comp in relative.components() {
            match comp {
                std::path::Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
                _ => bail!(
                    "refusing to package unsafe path component in '{}'",
                    relative.display()
                ),
            }
        }
        let zip_name = format!("{mod_name}/{}", parts.join("/"));

        if file_type.is_dir() {
            zip.add_directory(&zip_name, options)?;
            add_dir_to_zip(zip, base, &path, mod_name, options, out_skip)?;
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
    fn rejects_directory_in_place_of_required_file() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        // enabled.txt is a DIRECTORY, not a file.
        fs::create_dir_all(mod_dir.join("enabled.txt")).unwrap();
        fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
        fs::write(mod_dir.join("Scripts").join("main.lua"), "-- lua").unwrap();
        let out = tmp.path().join("MyMod.zip");
        assert!(run(mod_dir, out).is_err(), "a directory at enabled.txt must fail");
    }

    #[cfg(unix)]
    #[test]
    fn backslash_filename_does_not_become_traversal() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
        fs::write(mod_dir.join("enabled.txt"), "").unwrap();
        fs::write(mod_dir.join("Scripts").join("main.lua"), "-- lua").unwrap();
        // A regular file whose name literally contains a backslash (legal on Unix).
        fs::write(mod_dir.join(r"..\escape.lua"), "x").unwrap();
        let out = tmp.path().join("MyMod.zip");
        run(mod_dir, out.clone()).unwrap();
        let mut archive = ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(!names.iter().any(|n| n.contains("../")),
            "backslash must not be normalized into a traversal: {names:?}");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_required_file_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
        // enabled.txt is a symlink to a real file elsewhere.
        let real = tmp.path().join("real_enabled.txt");
        fs::write(&real, "").unwrap();
        std::os::unix::fs::symlink(&real, mod_dir.join("enabled.txt")).unwrap();
        fs::write(mod_dir.join("Scripts").join("main.lua"), "-- lua").unwrap();
        let out = tmp.path().join("MyMod.zip");
        assert!(run(mod_dir, out).is_err(), "a symlinked enabled.txt must be rejected");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_dir_is_skipped_not_followed() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
        fs::write(mod_dir.join("enabled.txt"), "").unwrap();
        fs::write(mod_dir.join("Scripts").join("main.lua"), "-- lua").unwrap();
        // A symlink inside the mod dir pointing outside it.
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "x").unwrap();
        std::os::unix::fs::symlink(&outside, mod_dir.join("link")).unwrap();

        let out = tmp.path().join("MyMod.zip");
        run(mod_dir, out.clone()).unwrap();
        let mut archive = ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(!names.iter().any(|n| n.contains("secret.txt")),
            "must not follow the symlink: {names:?}");
        assert!(!names.iter().any(|n| n.contains("/link")),
            "symlink itself must not be archived: {names:?}");
    }

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

    #[test]
    fn mod_name_derived_via_canonicalize_for_dotted_path() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("RealName");
        let scripts_dir = mod_dir.join("Scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(mod_dir.join("enabled.txt"), "").unwrap();
        fs::write(scripts_dir.join("main.lua"), "-- lua").unwrap();

        // A trailing "/." makes file_name() None (like `package .`); the
        // canonicalize step must still recover "RealName". (No cwd change, so
        // this is safe under parallel test execution.)
        let dotted = mod_dir.join(".");
        let out = tmp.path().join("out.zip");
        run(dotted, out.clone()).unwrap();

        let mut archive = ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            names.iter().any(|n| n.starts_with("RealName/")),
            "archive root must be the real dir name, not 'Mod': {names:?}"
        );
    }

    #[test]
    fn output_inside_mod_dir_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        let scripts_dir = mod_dir.join("Scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(mod_dir.join("enabled.txt"), "").unwrap();
        fs::write(scripts_dir.join("main.lua"), "-- lua").unwrap();

        // Output inside the mod dir must be rejected BEFORE any file is created
        // (otherwise File::create could truncate a source file).
        let out_zip = mod_dir.join("MyMod.zip");
        assert!(run(mod_dir.clone(), out_zip.clone()).is_err(), "in-mod-dir output must be rejected");
        assert!(!out_zip.exists(), "no archive should have been created");
        // A source file inside the mod dir must remain intact.
        assert_eq!(fs::read_to_string(scripts_dir.join("main.lua")).unwrap(), "-- lua");
    }

    #[test]
    fn output_outside_mod_dir_succeeds() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("MyMod");
        let scripts_dir = mod_dir.join("Scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(mod_dir.join("enabled.txt"), "").unwrap();
        fs::write(scripts_dir.join("main.lua"), "-- lua").unwrap();
        let out_zip = tmp.path().join("MyMod.zip"); // sibling of mod_dir, outside it
        run(mod_dir, out_zip.clone()).unwrap();
        assert!(out_zip.is_file());
    }
}
