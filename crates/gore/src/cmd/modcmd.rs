//! `gore mod` — build/deploy a unified bundle (overrides + loc + audio + voice ZIPs + more).
//! Thin CLI over the `gore-mod` crate; same engine the mod-studio GUI uses via FFI.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

/// `gore mod build --spec spec.json --out DIR` → write the bundle dir.
/// What `--model` validation actually establishes, said so that the part it does not cover is
/// not read into it.
///
/// `validate_config` resolves the class and the field against the reflection model. It does not
/// look at `module`, because the model carries no module or package information to look at — and
/// the generated Lua builds `/Script/<module>.Default__<class>` out of exactly that field. A
/// misspelling there produces a CDO path nothing resolves, and the mod then behaves precisely like
/// an unknown class: 120 retries a second apart, one line in UE4SS.log, nothing changed in the
/// game. "checked N override(s)" invited a reader to believe that had been ruled out.
///
/// The modules in play are named rather than described, because a typo is recognisable on sight
/// and unrecognisable in prose.
fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

pub(super) fn canonical_destination(path: &Path) -> Result<PathBuf> {
    let absolute = absolute_path(path);
    for ancestor in absolute.ancestors() {
        match std::fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for component in absolute.strip_prefix(ancestor)?.components() {
                    match component {
                        Component::CurDir => {}
                        Component::ParentDir => {
                            resolved.pop();
                        }
                        Component::Normal(name) => resolved.push(name),
                        Component::Prefix(_) | Component::RootDir => unreachable!(),
                    }
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("resolving {}", path.display()));
            }
        }
    }
    anyhow::bail!("could not resolve {}", path.display())
}

fn path_starts_with_portably(path: &Path, prefix: &Path) -> bool {
    let mut parts = path.components();
    prefix.components().all(|expected| {
        parts.next().is_some_and(|actual| {
            actual.as_os_str().to_string_lossy().to_lowercase()
                == expected.as_os_str().to_string_lossy().to_lowercase()
        })
    })
}

fn reject_work_dir_inside_bundle(work_dir: &Path, out: &Path, mod_name: &str) -> Result<()> {
    let bundle = canonical_destination(&out.join(mod_name))?;
    let work = canonical_destination(work_dir)?;
    if path_starts_with_portably(&work, &bundle) {
        anyhow::bail!(
            "--work-dir {} is inside the published bundle {}; choose a directory outside it",
            work_dir.display(),
            out.join(mod_name).display()
        );
    }
    Ok(())
}

fn checked_against_model<'a>(
    count: usize,
    model_path: &std::path::Path,
    modules: impl Iterator<Item = &'a str>,
) -> String {
    let modules: std::collections::BTreeSet<&str> = modules.collect();
    format!(
        "checked {count} override class and field name(s) against '{}'. The module each one \
         names is NOT checked — the model carries none — and \
         `/Script/<module>.Default__<class>` is built from it, so a misspelling there resolves \
         to nothing exactly as silently as an unknown class would. In use: {}",
        model_path.display(),
        modules.into_iter().collect::<Vec<_>>().join(", ")
    )
}

/// Inspect every module carried by a script mini before values claim a module. Keep the
/// declared target too: single-module replacements use that target at deployment.
fn inspect_value_script_inputs(
    scripts: &[gore_mod::ScriptModule],
    base: &Path,
) -> Result<(Vec<String>, Vec<[u8; 32]>)> {
    let mut occupied = std::collections::BTreeSet::new();
    let mut seals = Vec::with_capacity(scripts.len());
    for script in scripts {
        let path = base.join(&script.mini_cache);
        // Match the deploy engine's per-mini bound; do not allocate an unbounded source file.
        let bytes = super::as_cache::read_regular_bounded(
            &path,
            512 * 1024 * 1024,
            "VALUE_SCRIPT_INPUT",
        )?;
        super::as_cache::validate_module_cache(&path, &bytes, "VALUE_SCRIPT_INPUT")?;
        occupied.insert(script.module_name.clone());
        let carried = gore_as::cache::walk_modules::module_names(&bytes)
            .with_context(|| format!("reading modules in {}", path.display()))?;
        // A single-module edit may be retargeted by its manifest name. Multi-module edits
        // upsert every carried module; additions retain the mini's own module names.
        if carried.len() != 1 || script.op == "add" {
            occupied.extend(carried);
        }
        seals.push(Sha256::digest(&bytes).into());
    }
    Ok((occupied.into_iter().collect(), seals))
}

/// The bundle must retain the exact minis inspected before value compilation, even when
/// another build changes a supplied file while the standalone compiler is running.
fn verify_value_script_inputs(bundle: &gore_mod::Bundle, seals: &[[u8; 32]]) -> Result<()> {
    if seals.is_empty() {
        return Ok(());
    }
    let manifest = bundle
        .files
        .get("scripts/manifest.json")
        .context("script manifest missing after value build")?;
    let entries: Vec<gore_mod::ScriptEntry> = serde_json::from_slice(manifest)?;
    anyhow::ensure!(
        entries.len() >= seals.len(),
        "script inputs missing after value build"
    );
    for (entry, expected) in entries.iter().zip(seals) {
        let mini = bundle
            .files
            .get(&entry.mini)
            .with_context(|| format!("script mini {} missing after value build", entry.mini))?;
        let actual: [u8; 32] = Sha256::digest(mini).into();
        anyhow::ensure!(
            actual == *expected,
            "script mini for {} changed during value compilation; rebuild with stable script inputs",
            entry.module
        );
    }
    Ok(())
}

pub fn build(
    spec_path: PathBuf,
    out: PathBuf,
    model: Option<PathBuf>,
    game: Option<PathBuf>,
    work_dir: Option<PathBuf>,
) -> Result<()> {
    let _ = model;
    let json = std::fs::read_to_string(&spec_path)
        .with_context(|| format!("reading spec '{}'", spec_path.display()))?;
    let mut spec: gore_mod::BuildSpec = serde_json::from_str(&json).context("parsing build spec")?;
    // `.value-minis` is the compiler workspace under `out`. Rejecting it here is before that
    // workspace is created, so a rebuild cannot write an invocation child into the old bundle.
    gore_mod::validate_mod_name(&spec.meta.name)?;
    if !spec.overrides.is_empty() {
        anyhow::bail!(
            "bundle overrides are retired. Author item and stat defaults in the `values` section \
             and build that. UE4SS is not used for first-party values."
        );
    }
    if !spec.dialog_topics.is_empty() {
        anyhow::bail!(
            "dialog_topics is retired. Dialog edits deploy as an AngelScript mini-cache, without \
             a UE4SS registration adapter."
        );
    }
    // Every relative asset input belongs to the spec's directory, including the minis inspected
    // before compilation; it must resolve the same way when the bundle is assembled below.
    let base = spec_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut value_invocation_dirs = Vec::new();
    let mut script_input_seals = Vec::new();
    let value_generation = if spec.values.is_empty() {
        None
    } else {
        let game = gore_loc::config::game_root(game).context("resolving game path for values")?;
        let work_dir = work_dir.ok_or_else(|| {
            anyhow::anyhow!("`--work-dir` is required when the spec contains `values`")
        })?;
        reject_work_dir_inside_bundle(&work_dir, &out, &spec.meta.name)?;
        let source = gore_mod::pristine_script_cache_source(&game)?;
        let mini_dir = absolute_path(&out).join(".value-minis");
        let (occupied, seals) = inspect_value_script_inputs(&spec.scripts, base)?;
        script_input_seals = seals;
        let (scripts, cache_sha, invocation_dirs) = crate::cmd::value::compile_values_into_scripts(
            &game,
            &work_dir,
            &source.path,
            &spec.values,
            &mini_dir,
            &occupied,
        )?;
        value_invocation_dirs = invocation_dirs;
        let compiled_any_value = !scripts.is_empty();
        spec.scripts.extend(scripts);
        let targets: Vec<String> = spec
            .values
            .iter()
            .map(|edit| match &edit.tag {
                Some(tag) => format!("{}.{tag}.{}", edit.class, edit.field),
                None => format!("{}.{}", edit.class, edit.field),
            })
            .collect();
        spec.values.clear();
        if !compiled_any_value && !spec_has_deployable_content(&spec) {
            anyhow::bail!(
                "every requested value already matches the installed default, so there is \
                 nothing to build"
            );
        }
        Some(serde_json::json!({
            "cache_sha256": cache_sha,
            "from_backup": source.from_backup,
            "targets": targets,
        }))
    };
    // Asset paths written in the spec are resolved against the SPEC's own directory, exactly like
    // `gore audio replace --map`. A path written next to the spec has to mean the file next to the
    // spec: an agent or GUI that runs this command chooses neither the working directory nor,
    // usually, knows what it is.
    let mut bundle = gore_mod::build_bundle_relative_to(&spec, base)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("building bundle from spec '{}'", spec_path.display()))?;
    verify_value_script_inputs(&bundle, &script_input_seals)?;
    if let Some(generation) = value_generation {
        bundle.files.insert(
            "scripts/value-generation.json".into(),
            serde_json::to_vec_pretty(&generation)?,
        );
    }
    let dir = out.join(&spec.meta.name);
    gore_mod::write_bundle(&dir, &bundle).map_err(|e| anyhow::anyhow!("{e}"))?;
    crate::cmd::value::remove_value_invocation_dirs(&value_invocation_dirs)?;
    println!(
        "built bundle: {} ({} components, {} files)",
        dir.display(),
        bundle.manifest.components.len(),
        bundle.files.len()
    );
    Ok(())
}

fn spec_has_deployable_content(spec: &gore_mod::BuildSpec) -> bool {
    !spec.scripts.is_empty()
        || !spec.loc_edits.is_empty()
        || !spec.audio.is_empty()
        || !spec.texture.is_empty()
        || !spec.files.is_empty()
        || !spec.pak_files.is_empty()
        || !spec.voice.is_empty()
}

/// `gore mod inspect BUNDLE_OR_ZIP` → validate and summarize a built GORE bundle offline.
pub fn inspect(bundle: PathBuf, json: bool) -> Result<()> {
    let report = gore_mod::mgr::import::inspect_gore_bundle(&bundle)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("inspecting GORE bundle '{}'", bundle.display()))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let version = if report.mod_meta.version.is_empty() {
        String::new()
    } else {
        format!(" {}", report.mod_meta.version)
    };
    let author = if report.mod_meta.author.is_empty() {
        String::new()
    } else {
        format!(" by {}", report.mod_meta.author)
    };
    println!(
        "valid GORE bundle: {}{version}{author}",
        report.mod_meta.name
    );
    println!(
        "format {}, {} component(s), {} file(s), {} byte(s)",
        report.bundle_format, report.component_count, report.file_count, report.total_file_bytes
    );
    for component in &report.components {
        let name = component
            .name
            .as_deref()
            .map(|name| format!(" name={name}"))
            .unwrap_or_default();
        println!(
            "  - {} path={} targets={} coverage={:?}{name}",
            component.component_type,
            component.path,
            component.target_count,
            component.footprint_coverage
        );
    }
    println!("manifest sha256: {}", report.manifest_sha256);
    println!("tree sha256: {}", report.tree_sha256);
    println!(
        "not proven: game-install compatibility, cross-mod/load-order behavior, or runtime effect"
    );
    Ok(())
}

/// `gore mod deploy --bundle DIR --game ROOT` → apply to the game install.
pub fn deploy(bundle: PathBuf, game: Option<PathBuf>) -> Result<()> {
    let game = gore_loc::config::game_root(game)?;
    let rec = gore_mod::deploy(&bundle, &game).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "deployed '{}' ({} backup(s))",
        rec.mod_name,
        rec.backups.len()
    );
    // Both go to stderr after the result line: the deploy succeeded, and these are findings about
    // individual edits rather than a failure of it.
    //
    // Reported as two separate things because they call for opposite responses. A skipped edit was
    // never written and the spec has to change. A shadowed edit WAS written and is simply not the
    // one the game reads — telling somebody that it "did not apply" invites them to undo a
    // deployment that worked.
    if !rec.loc_skipped.is_empty() {
        eprintln!(
            "warning: {} localization edit(s) could not be written — the id carries no slot for \
             that language:",
            rec.loc_skipped.len()
        );
        for warning in &rec.loc_skipped {
            eprintln!("  - {warning}");
        }
    }
    if !rec.loc_shadowed.is_empty() {
        eprintln!(
            "note: {} localization edit(s) were written but will not be seen — the id also carries \
             a newer generation of the same language, and the game reads that one:",
            rec.loc_shadowed.len()
        );
        for warning in &rec.loc_shadowed {
            eprintln!("  - {warning}");
        }
    }
    if !rec.loc_skipped.is_empty() || !rec.loc_shadowed.is_empty() {
        eprintln!("See the text-and-dialogs guide page on which language key to write.");
    }
    Ok(())
}

/// `gore mod undeploy --game ROOT` → restore the active mod's backups.
pub fn undeploy(game: Option<PathBuf>) -> Result<()> {
    let game = gore_loc::config::game_root(game)?;
    match gore_mod::undeploy(&game).map_err(|e| anyhow::anyhow!("{e}"))? {
        Some(rec) => println!(
            "undeployed '{}' ({} restored)",
            rec.mod_name,
            rec.backups.len()
        ),
        None => println!("nothing deployed"),
    }
    Ok(())
}

#[cfg(test)]
mod validation_message_tests {
    use super::checked_against_model;
    use std::path::Path;

    fn empty_script_mini(names: &[&str]) -> Vec<u8> {
        fn string(value: &str, fstring: bool) -> Vec<u8> {
            let length = value.len() as i32 + i32::from(fstring);
            let mut bytes = length.to_le_bytes().to_vec();
            if !value.is_empty() || fstring {
                bytes.extend_from_slice(value.as_bytes());
                bytes.push(0);
            }
            bytes
        }
        let mut bytes = vec![0; 16];
        bytes.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&(names.len() as u32).to_le_bytes());
        for name in names {
            bytes.extend(string(name, true));
            bytes.extend(string(name, false));
            bytes.extend_from_slice(&[0; 32]); // functions/classes/enums/globals/imports/code hash/modules
            bytes.extend(string("", false));
            bytes.extend_from_slice(&[0; 8]); // events/delegates
            bytes.extend(string(&format!("{name}.as"), false));
            bytes.extend_from_slice(&[0; 4]); // post-init functions
        }
        bytes.extend(vec![0; 4 * gore_as::cache::tables::N_TABLES]);
        bytes
    }

    fn script_spec(path: &str, target: &str, op: &str) -> gore_mod::BuildSpec {
        serde_json::from_value(serde_json::json!({
            "meta": {"name":"M"},
            "scripts": [{"op":op,"module_name":target,"mini_cache":path}],
        })).unwrap()
    }

    #[test]
    fn values_reserve_secondary_mini_modules_and_resolve_relative_spec_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("multi.cache"), empty_script_mini(&["A", "B"])).unwrap();
        let spec = script_spec("nested/multi.cache", "A", "edit");
        let (occupied, _) = super::inspect_value_script_inputs(&spec.scripts, temp.path()).unwrap();
        assert_eq!(occupied, ["A", "B"]);
        // Single-module replacement targets come from the manifest rather than a copied name.
        std::fs::write(nested.join("multi.cache"), empty_script_mini(&["A"])).unwrap();
        let spec = script_spec("nested/multi.cache", "B", "edit");
        let (occupied, _) = super::inspect_value_script_inputs(&spec.scripts, temp.path()).unwrap();
        assert_eq!(occupied, ["B"]);
        let spec = script_spec("nested/multi.cache", "B", "add");
        let (occupied, _) = super::inspect_value_script_inputs(&spec.scripts, temp.path()).unwrap();
        assert_eq!(occupied, ["A", "B"]);
    }

    #[test]
    fn values_refuse_script_input_drift_before_bundle_publication() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("multi.cache");
        std::fs::write(&path, empty_script_mini(&["A", "B"])).unwrap();
        let spec = script_spec("multi.cache", "A", "edit");
        let (_, seals) = super::inspect_value_script_inputs(&spec.scripts, temp.path()).unwrap();
        let bundle = gore_mod::build_bundle_relative_to(&spec, temp.path()).unwrap();
        super::verify_value_script_inputs(&bundle, &seals).unwrap();
        std::fs::write(&path, empty_script_mini(&["A", "C"])).unwrap();
        let changed = gore_mod::build_bundle_relative_to(&spec, temp.path()).unwrap();
        assert!(super::verify_value_script_inputs(&changed, &seals)
            .unwrap_err().to_string().contains("changed during value compilation"));
    }

    #[test]
    fn values_refuse_a_script_input_that_is_not_a_module_cache() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("wrong.cache"), b"Binds.Cache, not a mini").unwrap();
        let spec = script_spec("wrong.cache", "A", "edit");
        assert!(super::inspect_value_script_inputs(&spec.scripts, temp.path()).is_err());
    }

    #[test]
    fn the_message_says_what_was_not_checked_and_names_the_modules() {
        // The reported defect: "checked 3 override(s)" read as "this mod is known to resolve",
        // while a mistyped module produces a CDO path nothing finds and a mod that does nothing.
        let message = checked_against_model(
            3,
            Path::new("model.json"),
            ["Angelscript", "Angelscrpt", "Angelscript"].into_iter(),
        );

        assert!(message.contains("class and field name(s)"), "{message}");
        assert!(message.contains("NOT checked"), "{message}");
        assert!(
            message.contains("/Script/<module>.Default__<class>"),
            "{message}"
        );

        // Named, deduplicated and sorted, so a typo stands next to the correct spelling.
        assert!(
            message.ends_with("In use: Angelscript, Angelscrpt"),
            "{message}"
        );

        // And no run of spaces from a mangled line continuation, because this is the one line a
        // reader is meant to actually read.
        assert!(!message.contains("  "), "{message}");
    }

    #[test]
    fn a_values_only_spec_with_no_compiled_script_has_nothing_to_deploy() {
        let empty: gore_mod::BuildSpec =
            serde_json::from_str(r#"{"meta":{"name":"M"}}"#).unwrap();
        assert!(!super::spec_has_deployable_content(&empty));
        let with_script: gore_mod::BuildSpec = serde_json::from_str(
            r#"{"meta":{"name":"M"},"scripts":[{"op":"edit","module_name":"Mod","mini_cache":"a.mini.cache"}]}"#,
        )
        .unwrap();
        assert!(super::spec_has_deployable_content(&with_script));
    }

    #[test]
    fn value_work_dir_cannot_be_inside_the_published_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("build");
        let bundle = out.join("MyMod");
        std::fs::create_dir_all(&bundle).unwrap();
        assert!(super::reject_work_dir_inside_bundle(&bundle, &out, "MyMod").is_err());
        assert!(super::reject_work_dir_inside_bundle(
            &bundle.join("work"),
            &out,
            "MyMod"
        )
        .is_err());
        assert!(super::reject_work_dir_inside_bundle(
            &bundle.join("other/../work"),
            &out,
            "MyMod"
        )
        .is_err());
        assert!(super::reject_work_dir_inside_bundle(
            &out.join("work"),
            &out,
            "MyMod"
        )
        .is_ok());
        let fresh_out = temp.path().join("fresh");
        assert!(super::reject_work_dir_inside_bundle(
            &fresh_out.join("mymod/work"),
            &fresh_out,
            "MyMod"
        )
        .is_err());
        assert!(super::reject_work_dir_inside_bundle(
            &fresh_out.join("MyModOther/work"),
            &fresh_out,
            "MyMod"
        )
        .is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn value_work_dir_symlink_into_bundle_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("build");
        let bundle = out.join("MyMod");
        std::fs::create_dir_all(&bundle).unwrap();
        let link = temp.path().join("linked-bundle");
        std::os::unix::fs::symlink(&bundle, &link).unwrap();
        assert!(super::reject_work_dir_inside_bundle(
            &link.join("work"),
            &out,
            "MyMod"
        )
        .is_err());
    }
}
