//! Deterministic revision-3 NPC source regeneration primitives.
//!
//! Inputs and persisted fingerprints are owned directly by the current revision-3 model.

use sha2::{Digest, Sha256};

use crate::{
    LogicalNpcCloneDraftError, NpcDraftInput, Sha256Digest, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION,
};

/// Generated names that must remain unique across every Story module in one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedStoryIdentity {
    pub module_namespace: String,
    pub module_relative_path: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcRegenerationError {
    #[error(
        "generator contract mismatch: expected {expected_id}@{expected_version}, got {actual_id}@{actual_version}"
    )]
    GeneratorContract {
        expected_id: &'static str,
        expected_version: u32,
        actual_id: String,
        actual_version: u32,
    },
    #[error("invalid NPC provenance: {0}")]
    InvalidNpcProvenance(String),
    #[error("invalid NPC generator intent: {0}")]
    InvalidNpcIntent(#[source] LogicalNpcCloneDraftError),
    #[error("could not fingerprint NPC generator input: {0}")]
    NpcFingerprint(#[source] serde_json::Error),
}

pub(crate) fn validate_generator_contract(
    actual_id: &str,
    actual_version: u32,
    expected_id: &'static str,
    expected_version: u32,
) -> Result<(), Revision3NpcRegenerationError> {
    if actual_id == expected_id && actual_version == expected_version {
        Ok(())
    } else {
        Err(Revision3NpcRegenerationError::GeneratorContract {
            expected_id,
            expected_version,
            actual_id: actual_id.to_owned(),
            actual_version,
        })
    }
}

pub(crate) fn validate_npc_input_provenance(
    input: &NpcDraftInput,
) -> Result<(), Revision3NpcRegenerationError> {
    if input.target.executable.byte_len == 0 {
        return Err(Revision3NpcRegenerationError::InvalidNpcProvenance(
            "target executable seal has zero byte length".to_owned(),
        ));
    }
    for (label, parent) in [
        (
            "parent_character_definition",
            &input.parent_character_definition,
        ),
        ("parent_ai_agent_config", &input.parent_ai_agent_config),
        ("parent_spawn_definition", &input.parent_spawn_definition),
    ] {
        if parent.generation != input.target {
            return Err(Revision3NpcRegenerationError::InvalidNpcProvenance(
                format!("{label} generation does not match target"),
            ));
        }
        if parent.source_seal.byte_len == 0 {
            return Err(Revision3NpcRegenerationError::InvalidNpcProvenance(
                format!("{label} source seal has zero byte length"),
            ));
        }
        if !canonical_catalog_layer(&parent.catalog_layer) {
            return Err(Revision3NpcRegenerationError::InvalidNpcProvenance(
                format!("{label} catalog layer is not a canonical lowercase identifier"),
            ));
        }
        if !canonical_selector(&parent.canonical_selector) {
            return Err(Revision3NpcRegenerationError::InvalidNpcProvenance(
                format!("{label} selector is not a canonical technical identifier"),
            ));
        }
    }
    Ok(())
}

fn canonical_catalog_layer(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let mut previous_separator = true;
    for byte in value.bytes() {
        let separator = matches!(byte, b'.' | b'-' | b'_');
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || separator)
            || (separator && previous_separator)
        {
            return false;
        }
        previous_separator = separator;
    }
    !previous_separator
}

fn canonical_selector(value: &str) -> bool {
    if value.is_empty() || value.len() > 96 {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !value.starts_with("__")
}

pub(crate) fn fingerprint_npc_input(
    input: &NpcDraftInput,
) -> Result<Sha256Digest, serde_json::Error> {
    let canonical = serde_json::to_vec(input)?;
    let mut hasher = Sha256::new();
    hasher.update(b"gore-authoring.revision3.npc-draft.input-fingerprint\0");
    hasher.update((LOGICAL_NPC_CLONE_GENERATOR_ID.len() as u64).to_be_bytes());
    hasher.update(LOGICAL_NPC_CLONE_GENERATOR_ID.as_bytes());
    hasher.update(u64::from(LOGICAL_NPC_CLONE_GENERATOR_VERSION).to_be_bytes());
    hasher.update((canonical.len() as u64).to_be_bytes());
    hasher.update(canonical);
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}
