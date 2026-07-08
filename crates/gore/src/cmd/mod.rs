pub mod as_cache;
pub mod audio;
pub mod catalog;
pub mod config;
pub mod deploy_shared;
pub mod dump;
pub mod dump_mod;
pub mod gen;
pub mod gui_model;
pub mod loc;
pub mod mgr;
pub mod modcmd;
pub mod package;
pub mod scaffold;
pub mod stubs;
pub mod sync;
pub mod texture;

/// Validate that `name` is a safe single-component mod name that can be
/// appended to a path without escaping the parent directory.
/// Rejects: empty names, names containing path separators (`/` or `\`),
/// the special component `..`, and anything that `Path::components()` treats
/// as more than one component or a non-normal component.
pub fn validate_mod_name(name: &str) -> anyhow::Result<()> {
    use std::path::Component;
    if name.is_empty() {
        anyhow::bail!("mod name must not be empty");
    }
    // Control characters (newline, tab, …) are never valid in a mod/dir name,
    // and a newline embedded in generated Lua/scaffold output could terminate a
    // comment and inject executable code.
    if name.chars().any(char::is_control) {
        anyhow::bail!("mod name must not contain control characters: {name:?}");
    }
    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("mod name must not contain path separators: '{name}'");
    }
    let path = std::path::Path::new(name);
    let components: Vec<_> = path.components().collect();
    if components.len() != 1 {
        anyhow::bail!("mod name must be a single path component: '{name}'");
    }
    match components[0] {
        Component::Normal(_) => Ok(()),
        _ => anyhow::bail!("mod name is not a valid directory name: '{name}'"),
    }
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
}
