//! `gore mod` — build/deploy a unified bundle (overrides + loc + audio + voice ZIPs + more).
//! Thin CLI over the `gore-mod` crate; same engine the mod-studio GUI uses via FFI.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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

pub fn build(spec_path: PathBuf, out: PathBuf, model: Option<PathBuf>) -> Result<()> {
    let json = std::fs::read_to_string(&spec_path)
        .with_context(|| format!("reading spec '{}'", spec_path.display()))?;
    let spec: gore_mod::BuildSpec = serde_json::from_str(&json).context("parsing build spec")?;
    // Override class/field names are the one part of a spec that nothing else checks: an unknown
    // class produces a well-formed bundle whose Lua never resolves, and the only report of that is
    // a "gave up" line in UE4SS.log after two minutes of retries. Validate before building, so a
    // rejected spec never reaches write_bundle (which clears <out>/<mod-name> first).
    if !spec.overrides.is_empty() {
        let cfg = gore_modgen::gen::OverridesConfig {
            meta: gore_modgen::gen::MetaConfig {
                name: spec.meta.name.clone(),
                delay_ms: spec.delay_ms,
            },
            overrides: spec.overrides.clone(),
        };
        match &model {
            Some(model_path) => {
                crate::cmd::validate_overrides_against_model(
                    &cfg,
                    model_path,
                    &spec_path.display().to_string(),
                )?;
                eprintln!(
                    "{}",
                    checked_against_model(
                        cfg.overrides.len(),
                        model_path,
                        cfg.overrides.iter().map(|o| o.module.as_str()),
                    )
                );
            }
            // stdout stays the bundle result so `--json`-style consumers see one clean document.
            None => eprintln!(
                "note: no --model, so none of the {} override class and field names were checked. \
                 An unknown class never resolves in game: the mod retries once a second for 120 \
                 attempts and reports it only in UE4SS.log. See the catalogs-and-models guide page \
                 for building a model.json.",
                cfg.overrides.len()
            ),
        }
    }
    // Asset paths written in the spec are resolved against the SPEC's own directory, exactly like
    // `gore audio replace --map`. A path written next to the spec has to mean the file next to the
    // spec: an agent or GUI that runs this command chooses neither the working directory nor,
    // usually, knows what it is.
    let base = spec_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let bundle = gore_mod::build_bundle_relative_to(&spec, base)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("building bundle from spec '{}'", spec_path.display()))?;
    let dir = out.join(&spec.meta.name);
    gore_mod::write_bundle(&dir, &bundle).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "built bundle: {} ({} components, {} files)",
        dir.display(),
        bundle.manifest.components.len(),
        bundle.files.len()
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
        eprintln!(
            "See the text-and-dialogs guide page on which language key to write."
        );
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
        assert!(message.contains("/Script/<module>.Default__<class>"), "{message}");

        // Named, deduplicated and sorted, so a typo stands next to the correct spelling.
        assert!(message.ends_with("In use: Angelscript, Angelscrpt"), "{message}");

        // And no run of spaces from a mangled line continuation, because this is the one line a
        // reader is meant to actually read.
        assert!(!message.contains("  "), "{message}");
    }
}
