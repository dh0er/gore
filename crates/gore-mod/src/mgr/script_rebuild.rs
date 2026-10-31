//! Source-backed script updates and confirmations for changed vanilla modules.
//!
//! Confirmations describe one exact loadout and effective pristine cache. They grant no
//! authority to skip compiler errors, package validation, or deployment recovery checks.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One vanilla module whose original contents have changed since the mod was authored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptModuleUpdateWarning {
    pub mod_id: String,
    pub mod_name: String,
    pub module: String,
    pub reason: String,
    pub original_sha256: String,
    pub current_sha256: Option<String>,
}

/// The exact warning set which the user must review before an update is applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRebuildConfirmation {
    pub token: String,
    pub warnings: Vec<ScriptModuleUpdateWarning>,
}

pub(super) fn require_confirmation(
    base: &[u8],
    loadout_context: &[u8],
    warnings: Vec<ScriptModuleUpdateWarning>,
    accepted_token: Option<&str>,
) -> crate::Result<Vec<String>> {
    if warnings.is_empty() {
        return Ok(Vec::new());
    }
    let mut digest = Sha256::new();
    digest.update(b"gore.manager.script-update-confirmation.v1\0");
    digest.update(Sha256::digest(base));
    digest.update(Sha256::digest(loadout_context));
    digest.update(serde_json::to_vec(&warnings)?);
    let token = format!("{:x}", digest.finalize());
    if accepted_token != Some(token.as_str()) {
        return Err(crate::ModError::ScriptRebuildConfirmationRequired(
            ScriptRebuildConfirmation { token, warnings },
        ));
    }
    Ok(warnings
        .iter()
        .map(|warning| {
            format!(
                "{}: rebuilding {} over a changed or removed vanilla module was confirmed; the mod may replace game-update fixes",
                warning.mod_name, warning.module
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning() -> ScriptModuleUpdateWarning {
        ScriptModuleUpdateWarning {
            mod_id: "diego-dialog".into(),
            mod_name: "Diego dialog".into(),
            module: "Story.Diego".into(),
            reason: "vanilla_module_changed".into(),
            original_sha256: "a".repeat(64),
            current_sha256: Some("b".repeat(64)),
        }
    }

    fn token(base: &[u8], context: &[u8], warning: ScriptModuleUpdateWarning) -> String {
        match require_confirmation(base, context, vec![warning], None).unwrap_err() {
            crate::ModError::ScriptRebuildConfirmationRequired(confirmation) => confirmation.token,
            error => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn confirmation_accepts_only_the_reviewed_cache_loadout_and_warning_set() {
        let accepted = token(
            b"new game cache",
            b"ordered mods and source seals",
            warning(),
        );
        assert_eq!(
            require_confirmation(
                b"new game cache",
                b"ordered mods and source seals",
                vec![warning()],
                Some(&accepted),
            )
            .unwrap()
            .len(),
            1
        );
        assert_ne!(
            accepted,
            token(
                b"newer game cache",
                b"ordered mods and source seals",
                warning()
            )
        );
        assert_ne!(
            accepted,
            token(b"new game cache", b"reordered mods", warning())
        );
        let mut changed = warning();
        changed.current_sha256 = None;
        changed.reason = "vanilla_module_removed".into();
        assert_ne!(
            accepted,
            token(b"new game cache", b"ordered mods and source seals", changed)
        );
        assert!(require_confirmation(
            b"newer game cache",
            b"ordered mods and source seals",
            vec![warning()],
            Some(&accepted)
        )
        .is_err());
    }

    #[test]
    fn unchanged_vanilla_needs_no_confirmation() {
        assert!(require_confirmation(b"cache", b"loadout", Vec::new(), None)
            .unwrap()
            .is_empty());
    }
}
