pub mod as_cache;
pub mod asset;
pub mod audio;
pub mod catalog;
pub mod config;
pub mod deploy_shared;
pub mod dialog;
pub mod doctor;
pub mod dump;
pub mod dump_mod;
pub mod find;
pub mod gen;
pub mod gui_model;
pub mod guide;
pub mod loc;
pub mod location;
pub mod location_catalog;
pub mod mcp;
pub mod mgr;
pub mod modcmd;
pub mod npc;
pub mod package;
pub mod scaffold;
pub mod story_catalog;
pub mod stubs;
pub mod sync;
pub mod texture;
pub mod voice;

/// Case-insensitive substring test shared by every bounded listing's `--filter`.
///
/// It lives here rather than beside one of them because the two commands answer the same question
/// about different files, and a fold that drifted apart would make `voice list --filter X` and
/// `audio list --filter X` disagree about what "contains" means -- silently, and only for the
/// callers whose text is not plain ASCII.
///
/// The fold is `str::to_lowercase`, which is what `gore_vo`'s `fold_case` applies to a voice
/// `--basename`. Folding only ASCII would move a false negative one code point up instead of
/// removing it: German archives are the documented target, and `--filter MÜLLER` must not report
/// nothing about an archive where `extract --basename DIA_MÜLLER_01.OGG` resolves. `needle` is
/// already lowercased by the caller, once per run rather than once per candidate; an empty needle
/// keeps everything.
pub fn contains_case_insensitive(haystack: &str, lowercase_needle: &str) -> bool {
    haystack.to_lowercase().contains(lowercase_needle)
}

/// Apply the canonical bundle/mod-name contract everywhere the CLI writes a mod directory.
///
/// Keeping this as a small adapter preserves the command-specific `anyhow` contexts while making
/// scaffolding, Lua generation, packaging, bundle building, and dialog staging agree on portability.
pub fn validate_mod_name(name: &str) -> anyhow::Result<()> {
    gore_mod::validate_mod_name(name).map_err(anyhow::Error::from)
}

/// Validate override class/field names against a reflection model.
///
/// Shared by `gore gen --model` and `gore mod build --model` so the two paths
/// that generate the same Lua cannot drift into two different verdicts or two
/// different error texts. `source` is the file the overrides were authored in
/// (an `overrides.toml` for `gen`, a build spec for `mod build`), so the message
/// names the file the reader has to edit.
pub fn validate_overrides_against_model(
    cfg: &gore_modgen::gen::OverridesConfig,
    model_path: &std::path::Path,
    source: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let json = std::fs::read_to_string(model_path)
        .with_context(|| format!("reading model.json '{}'", model_path.display()))?;
    let model: gore_reflect::model::ReflectionModel =
        serde_json::from_str(&json).with_context(|| "parsing model.json")?;

    let errors = gore_modgen::validate::validate_config(cfg, &model);
    if errors.is_empty() {
        return Ok(());
    }
    // validate_config emits at most one error per override, so the ratio is honest.
    let list = errors
        .iter()
        .map(|e| format!("  - {e}"))
        .collect::<Vec<_>>()
        .join("\n");
    anyhow::bail!(
        "{} of {} override(s) in '{}' do not match the model '{}':\n{}\n\
         The generated Lua resolves each class by name at runtime, so a name this model does \
         not carry is one the game will not resolve either: the mod retries once a second for \
         120 attempts, then writes one \"gave up\" line to UE4SS.log and changes nothing. \
         Correct the names, or run without --model if the model is older than your game build.",
        errors.len(),
        cfg.overrides.len(),
        source,
        model_path.display(),
        list
    );
}

#[cfg(test)]
mod mod_name_tests {
    use super::validate_mod_name;

    #[test]
    fn valid_mod_name_accepted() {
        assert!(validate_mod_name("MyMod").is_ok());
        assert!(validate_mod_name("my_mod_123").is_ok());
        assert!(validate_mod_name("GoreBalanceMod").is_ok());
    }

    #[test]
    fn empty_name_rejected() {
        assert!(validate_mod_name("").is_err());
    }

    #[test]
    fn control_chars_rejected() {
        // A newline could break out of a `--` comment in scaffolded/generated Lua.
        assert!(validate_mod_name("Bad\nMod").is_err());
        assert!(validate_mod_name("Bad\tMod").is_err());
        assert!(validate_mod_name("Bad\rMod").is_err());
    }

    #[test]
    fn dotdot_rejected() {
        assert!(validate_mod_name("..").is_err());
        assert!(validate_mod_name("../evil").is_err());
    }

    #[test]
    fn forward_slash_rejected() {
        assert!(validate_mod_name("a/b").is_err());
    }

    #[test]
    fn backslash_rejected() {
        assert!(validate_mod_name(r"a\b").is_err());
    }

    #[test]
    fn path_with_prefix_rejected() {
        assert!(validate_mod_name("subdir/MyMod").is_err());
    }

    #[test]
    fn portable_mod_name_byte_limit_matches_bundle_engine() {
        assert!(validate_mod_name(&"a".repeat(198)).is_ok());
        let error = validate_mod_name(&"a".repeat(199)).unwrap_err();
        assert!(
            error.to_string().contains("at most 198 UTF-8 bytes"),
            "{error}"
        );
    }
}

#[cfg(test)]
mod filter_tests {
    use super::contains_case_insensitive;

    #[test]
    fn the_shared_filter_folds_past_ascii_so_one_spelling_is_not_two_answers() {
        // Both listings feed this from a `--filter` a person typed. `to_ascii_lowercase` would keep
        // `MÜLLER` from matching `DIA_Müller_01.ogg` and answer "no such recording" about a file
        // that is right there, and an FMOD bank with an umlaut in a sample name would go the same
        // way. An empty needle is a filter that was given nothing to exclude, so it excludes
        // nothing.
        assert!(contains_case_insensitive("DIA_Müller_01.ogg", "müller"));
        assert!(contains_case_insensitive("SFX_UI_Click", "ui_click"));
        assert!(contains_case_insensitive("anything", ""));
        assert!(!contains_case_insensitive("SFX_UI_Click", "orcdog"));
    }
}
