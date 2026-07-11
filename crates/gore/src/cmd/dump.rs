use anyhow::{Context, Result};
use gore_reflect::{model::ReflectionModel, parser::parse_hpp_file};
use std::{fs, path::PathBuf};

pub fn run(sdk_dir: PathBuf, out: PathBuf) -> Result<()> {
    anyhow::ensure!(
        sdk_dir.is_dir(),
        "sdk-dir '{}' is not a directory",
        sdk_dir.display()
    );

    let mut merged = ReflectionModel::default();

    // Collect the .hpp paths and SORT them: fs::read_dir order is unspecified,
    // so without this the merged model.json (and downstream stubs/GUI models)
    // would churn between runs/machines for the same SDK.
    let mut hpp_paths: Vec<_> = fs::read_dir(&sdk_dir)
        .with_context(|| format!("reading sdk-dir '{}'", sdk_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "hpp"))
        .collect();
    hpp_paths.sort();

    for path in &hpp_paths {
        let partial =
            parse_hpp_file(path).with_context(|| format!("parsing '{}'", path.display()))?;
        merged.classes.extend(partial.classes);
        merged.enums.extend(partial.enums);
    }

    // Reconcile enum types across the merged model: a class in one .hpp may use
    // an enum declared in another, which the per-file pass could not resolve.
    merged.resolve_enum_types();

    // Write model.json
    if let Some(parent) = out.parent() {
        // out.parent() is Some("") for a bare filename like `-o model.json`;
        // create_dir_all("") errors, so only create a non-empty parent.
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(&merged)?;
    fs::write(&out, json).with_context(|| format!("writing '{}'", out.display()))?;

    println!(
        "Wrote {} classes, {} enums -> {}",
        merged.classes.len(),
        merged.enums.len(),
        out.display()
    );
    Ok(())
}
