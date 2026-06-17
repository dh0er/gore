use anyhow::{Context, Result};
use gore_core::{
    model::ReflectionModel,
    parser::parse_hpp_file,
};
use std::{
    fs,
    path::PathBuf,
};

pub fn run(sdk_dir: PathBuf, _object_dump: Option<PathBuf>, out: PathBuf) -> Result<()> {
    anyhow::ensure!(sdk_dir.is_dir(), "sdk-dir '{}' is not a directory", sdk_dir.display());

    let mut merged = ReflectionModel::default();

    // Collect and parse all .hpp files in sdk_dir
    let entries = fs::read_dir(&sdk_dir)
        .with_context(|| format!("reading sdk-dir '{}'", sdk_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "hpp").unwrap_or(false) {
            let partial = parse_hpp_file(&path)
                .with_context(|| format!("parsing '{}'", path.display()))?;
            merged.classes.extend(partial.classes);
            merged.enums.extend(partial.enums);
        }
    }

    // Write model.json
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&merged)?;
    fs::write(&out, json)
        .with_context(|| format!("writing '{}'", out.display()))?;

    println!("Wrote {} classes, {} enums -> {}", merged.classes.len(), merged.enums.len(), out.display());
    Ok(())
}
