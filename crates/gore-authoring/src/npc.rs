//! Offline-only source generation for the three-class logical NPC clone proven in
//! `docs/guide/studio-npc.md`. This module deliberately has no compiler, package, filesystem, game,
//! spawn, dialog, quest, or save integration.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::Sha256Digest;

/// Stable generator identity carried by every logical NPC clone draft.
pub const LOGICAL_NPC_CLONE_GENERATOR_ID: &str = "gore-authoring.logical-npc-clone-draft";
/// Increment when the accepted inputs, fixed semantics, or emitted source shape changes.
pub const LOGICAL_NPC_CLONE_GENERATOR_VERSION: u32 = 1;

/// Maximum byte length accepted for one AngelScript identifier.
pub const MAX_ANGELSCRIPT_IDENTIFIER_BYTES: usize = 96;
/// Conservative bound for the `m_UniqueName` value appended to generated class prefixes.
pub const MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES: usize = 64;
/// Maximum byte length accepted for a dot-separated module namespace.
pub const MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES: usize = 255;
/// Maximum number of components in a module namespace.
pub const MAX_ANGELSCRIPT_MODULE_SEGMENTS: usize = 16;

const CHARACTER_DEFINITION_CLASS_PREFIX: &str = "UCharacterDefinition_Human_";
const AI_AGENT_CONFIG_CLASS_PREFIX: &str = "UAIAgentConfig_Human_";
const SPAWN_DEFINITION_CLASS_PREFIX: &str = "USpawnAIAgentDefinition_";

const CHARACTER_DEFINITION_PARENT_PREFIX: &str = "UCharacterDefinition_";
const AI_AGENT_CONFIG_PARENT_PREFIX: &str = "UAIAgentConfig_";
const SPAWN_DEFINITION_PARENT_PREFIX: &str = "USpawnAIAgentDefinition_";

/// User-facing input field associated with a logical-clone validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalNpcCloneField {
    ModuleNamespace,
    ModuleSegment { index: usize },
    UniqueName,
    ParentCharacterDefinition,
    ParentAiAgentConfig,
    ParentSpawnDefinition,
}

impl fmt::Display for LogicalNpcCloneField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModuleNamespace => formatter.write_str("module namespace"),
            Self::ModuleSegment { index } => {
                write!(formatter, "module namespace segment {index}")
            }
            Self::UniqueName => formatter.write_str("technical identity/UniqueName"),
            Self::ParentCharacterDefinition => {
                formatter.write_str("parent CharacterDefinition class")
            }
            Self::ParentAiAgentConfig => formatter.write_str("parent AIAgentConfig class"),
            Self::ParentSpawnDefinition => {
                formatter.write_str("parent SpawnAIAgentDefinition class")
            }
        }
    }
}

/// Fail-closed validation errors for the bounded offline NPC draft generator.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LogicalNpcCloneDraftError {
    #[error("{field} must not be empty")]
    EmptyValue { field: LogicalNpcCloneField },
    #[error("{field} is {actual} bytes; maximum is {max}")]
    ValueTooLong {
        field: LogicalNpcCloneField,
        actual: usize,
        max: usize,
    },
    #[error("module namespace has {actual} segments; maximum is {max}")]
    TooManyModuleSegments { actual: usize, max: usize },
    #[error("{field} starts with invalid AngelScript identifier character {character:?}")]
    InvalidIdentifierStart {
        field: LogicalNpcCloneField,
        character: char,
    },
    #[error(
        "{field} contains invalid AngelScript identifier character {character:?} at byte {byte_index}"
    )]
    InvalidIdentifierCharacter {
        field: LogicalNpcCloneField,
        byte_index: usize,
        character: char,
    },
    #[error("{field} is reserved by AngelScript or the generator")]
    ReservedIdentifier { field: LogicalNpcCloneField },
    #[error("module namespace segment {index} is a reserved portable filesystem name")]
    ReservedModuleSegment { index: usize },
    #[error("{field} must begin with {expected_prefix:?}")]
    UnexpectedParentClassPrefix {
        field: LogicalNpcCloneField,
        expected_prefix: &'static str,
    },
    #[error("{field} collides with generated class {class_name}")]
    ClassNameCollision {
        field: LogicalNpcCloneField,
        class_name: String,
    },
}

/// The three class identifiers deterministically derived from one logical UniqueName.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalNpcCloneClassNames {
    pub character_definition: String,
    pub ai_agent_config: String,
    pub spawn_definition: String,
}

/// Offline authoring capability. There is intentionally no production-ready variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalNpcCloneAuthoringStatus {
    OfflineDraft,
}

/// Runtime qualification capability. Generation never claims runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalNpcCloneRuntimeStatus {
    RuntimeUnqualified,
}

/// Fixed capability status returned with every generated logical-clone source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalNpcCloneCapabilityStatus {
    pub authoring: LogicalNpcCloneAuthoringStatus,
    pub runtime: LogicalNpcCloneRuntimeStatus,
}

impl LogicalNpcCloneCapabilityStatus {
    pub const OFFLINE_DRAFT_RUNTIME_UNQUALIFIED: Self = Self {
        authoring: LogicalNpcCloneAuthoringStatus::OfflineDraft,
        runtime: LogicalNpcCloneRuntimeStatus::RuntimeUnqualified,
    };
}

/// Validated inputs for the offline-only three-class logical NPC clone shape.
///
/// This draft carries no claim about class discovery, spawning, visuals, dialog, quests,
/// persistence, saves, or production readiness. It only produces source for the linked class
/// chain proven by the offline NPC authoring probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalNpcCloneDraft {
    module_namespace: String,
    unique_name: String,
    parent_character_definition: String,
    parent_ai_agent_config: String,
    parent_spawn_definition: String,
    class_names: LogicalNpcCloneClassNames,
    input_fingerprint: Sha256Digest,
}

impl LogicalNpcCloneDraft {
    /// Validate exact identifiers and construct an immutable offline draft.
    pub fn new(
        module_namespace: impl Into<String>,
        unique_name: impl Into<String>,
        parent_character_definition: impl Into<String>,
        parent_ai_agent_config: impl Into<String>,
        parent_spawn_definition: impl Into<String>,
    ) -> Result<Self, LogicalNpcCloneDraftError> {
        let module_namespace = module_namespace.into();
        let unique_name = unique_name.into();
        let parent_character_definition = parent_character_definition.into();
        let parent_ai_agent_config = parent_ai_agent_config.into();
        let parent_spawn_definition = parent_spawn_definition.into();

        validate_module_namespace(&module_namespace)?;
        validate_identifier(
            LogicalNpcCloneField::UniqueName,
            &unique_name,
            MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES,
        )?;
        validate_parent_identifier(
            LogicalNpcCloneField::ParentCharacterDefinition,
            &parent_character_definition,
            CHARACTER_DEFINITION_PARENT_PREFIX,
        )?;
        validate_parent_identifier(
            LogicalNpcCloneField::ParentAiAgentConfig,
            &parent_ai_agent_config,
            AI_AGENT_CONFIG_PARENT_PREFIX,
        )?;
        validate_parent_identifier(
            LogicalNpcCloneField::ParentSpawnDefinition,
            &parent_spawn_definition,
            SPAWN_DEFINITION_PARENT_PREFIX,
        )?;

        let class_names = derive_class_names(&unique_name);
        validate_parent_collisions(
            &class_names,
            &parent_character_definition,
            &parent_ai_agent_config,
            &parent_spawn_definition,
        )?;
        let input_fingerprint = fingerprint_input(
            &module_namespace,
            &unique_name,
            &parent_character_definition,
            &parent_ai_agent_config,
            &parent_spawn_definition,
        );

        Ok(Self {
            module_namespace,
            unique_name,
            parent_character_definition,
            parent_ai_agent_config,
            parent_spawn_definition,
            class_names,
            input_fingerprint,
        })
    }

    pub fn module_namespace(&self) -> &str {
        &self.module_namespace
    }

    /// Exact name emitted into `default m_UniqueName = n"..."`.
    pub fn unique_name(&self) -> &str {
        &self.unique_name
    }

    pub fn parent_character_definition(&self) -> &str {
        &self.parent_character_definition
    }

    pub fn parent_ai_agent_config(&self) -> &str {
        &self.parent_ai_agent_config
    }

    pub fn parent_spawn_definition(&self) -> &str {
        &self.parent_spawn_definition
    }

    pub fn class_names(&self) -> &LogicalNpcCloneClassNames {
        &self.class_names
    }

    /// Canonical hash of every semantic input plus the fixed generator semantics.
    pub const fn input_fingerprint(&self) -> Sha256Digest {
        self.input_fingerprint
    }

    /// Generate deterministic source and module metadata without filesystem or runtime actions.
    pub fn generate(&self) -> LogicalNpcCloneSource {
        let classes = self.class_names.clone();
        let source = format!(
            "class {character}\n    : {parent_character}\n{{\n    default m_UniqueName = n\"{unique_name}\";\n}}\n\nclass {agent}\n    : {parent_agent}\n{{\n    default m_CharacterDefinition =\n        {character}::StaticClass();\n}}\n\nclass {spawn}\n    : {parent_spawn}\n{{\n    default AIAgentConfigClass =\n        {agent}::StaticClass();\n}}\n",
            character = classes.character_definition,
            parent_character = self.parent_character_definition,
            unique_name = self.unique_name,
            agent = classes.ai_agent_config,
            parent_agent = self.parent_ai_agent_config,
            spawn = classes.spawn_definition,
            parent_spawn = self.parent_spawn_definition,
        );
        let source_sha256 = Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into());

        LogicalNpcCloneSource {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID,
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            module_relative_path: format!("{}.as", self.module_namespace.replace('.', "/")),
            module_namespace: self.module_namespace.clone(),
            unique_name: self.unique_name.clone(),
            classes,
            source,
            source_sha256,
            input_fingerprint: self.input_fingerprint,
            status: LogicalNpcCloneCapabilityStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        }
    }
}

/// Deterministic in-memory result of [`LogicalNpcCloneDraft::generate`].
///
/// The relative path is metadata for a later, separately authorized compiler workflow. This type
/// does not write it, compile it, deploy it, start the game, or mutate a save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalNpcCloneSource {
    pub generator_id: &'static str,
    pub generator_version: u32,
    pub module_namespace: String,
    pub module_relative_path: String,
    pub unique_name: String,
    pub classes: LogicalNpcCloneClassNames,
    pub source: String,
    pub source_sha256: Sha256Digest,
    /// Includes non-source provenance such as the module namespace as well as fixed semantics.
    pub input_fingerprint: Sha256Digest,
    pub status: LogicalNpcCloneCapabilityStatus,
}

fn derive_class_names(unique_name: &str) -> LogicalNpcCloneClassNames {
    LogicalNpcCloneClassNames {
        character_definition: format!("{CHARACTER_DEFINITION_CLASS_PREFIX}{unique_name}"),
        ai_agent_config: format!("{AI_AGENT_CONFIG_CLASS_PREFIX}{unique_name}"),
        spawn_definition: format!("{SPAWN_DEFINITION_CLASS_PREFIX}{unique_name}"),
    }
}

fn fingerprint_input(
    module_namespace: &str,
    unique_name: &str,
    parent_character_definition: &str,
    parent_ai_agent_config: &str,
    parent_spawn_definition: &str,
) -> Sha256Digest {
    let mut hasher = Sha256::new();
    fingerprint_bytes(
        &mut hasher,
        "schema",
        b"gore-authoring.logical-npc-clone-draft-v1.input-fingerprint",
    );
    fingerprint_string(&mut hasher, "generator.id", LOGICAL_NPC_CLONE_GENERATOR_ID);
    fingerprint_u64(
        &mut hasher,
        "generator.version",
        u64::from(LOGICAL_NPC_CLONE_GENERATOR_VERSION),
    );
    fingerprint_string(&mut hasher, "module.namespace", module_namespace);
    fingerprint_string(&mut hasher, "npc.unique-name", unique_name);
    fingerprint_string(
        &mut hasher,
        "parent.character-definition",
        parent_character_definition,
    );
    fingerprint_string(
        &mut hasher,
        "parent.ai-agent-config",
        parent_ai_agent_config,
    );
    fingerprint_string(
        &mut hasher,
        "parent.spawn-definition",
        parent_spawn_definition,
    );

    // Fixed validation/lowering semantics are bound as defense-in-depth if an implementation
    // constant ever changes without the required generator-version bump.
    fingerprint_string(
        &mut hasher,
        "shape.character-definition-class-prefix",
        CHARACTER_DEFINITION_CLASS_PREFIX,
    );
    fingerprint_string(
        &mut hasher,
        "shape.ai-agent-config-class-prefix",
        AI_AGENT_CONFIG_CLASS_PREFIX,
    );
    fingerprint_string(
        &mut hasher,
        "shape.spawn-definition-class-prefix",
        SPAWN_DEFINITION_CLASS_PREFIX,
    );
    fingerprint_string(
        &mut hasher,
        "validation.character-definition-parent-prefix",
        CHARACTER_DEFINITION_PARENT_PREFIX,
    );
    fingerprint_string(
        &mut hasher,
        "validation.ai-agent-config-parent-prefix",
        AI_AGENT_CONFIG_PARENT_PREFIX,
    );
    fingerprint_string(
        &mut hasher,
        "validation.spawn-definition-parent-prefix",
        SPAWN_DEFINITION_PARENT_PREFIX,
    );
    fingerprint_string(&mut hasher, "capability.authoring", "OfflineDraft");
    fingerprint_string(&mut hasher, "capability.runtime", "RuntimeUnqualified");

    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn fingerprint_string(hasher: &mut Sha256, field: &str, value: &str) {
    fingerprint_bytes(hasher, field, value.as_bytes());
}

fn fingerprint_u64(hasher: &mut Sha256, field: &str, value: u64) {
    fingerprint_bytes(hasher, field, &value.to_be_bytes());
}

fn fingerprint_bytes(hasher: &mut Sha256, field: &str, value: &[u8]) {
    hasher.update((field.len() as u64).to_be_bytes());
    hasher.update(field.as_bytes());
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn validate_module_namespace(value: &str) -> Result<(), LogicalNpcCloneDraftError> {
    validate_nonempty_bounded(
        LogicalNpcCloneField::ModuleNamespace,
        value,
        MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES,
    )?;
    let segments = value.split('.').collect::<Vec<_>>();
    if segments.len() > MAX_ANGELSCRIPT_MODULE_SEGMENTS {
        return Err(LogicalNpcCloneDraftError::TooManyModuleSegments {
            actual: segments.len(),
            max: MAX_ANGELSCRIPT_MODULE_SEGMENTS,
        });
    }
    for (index, segment) in segments.into_iter().enumerate() {
        let field = LogicalNpcCloneField::ModuleSegment { index };
        validate_identifier(field, segment, MAX_ANGELSCRIPT_IDENTIFIER_BYTES)?;
        if is_reserved_portable_path_segment(segment) {
            return Err(LogicalNpcCloneDraftError::ReservedModuleSegment { index });
        }
    }
    Ok(())
}

fn validate_parent_identifier(
    field: LogicalNpcCloneField,
    value: &str,
    expected_prefix: &'static str,
) -> Result<(), LogicalNpcCloneDraftError> {
    validate_identifier(field, value, MAX_ANGELSCRIPT_IDENTIFIER_BYTES)?;
    if !value.starts_with(expected_prefix) || value.len() == expected_prefix.len() {
        return Err(LogicalNpcCloneDraftError::UnexpectedParentClassPrefix {
            field,
            expected_prefix,
        });
    }
    Ok(())
}

fn validate_identifier(
    field: LogicalNpcCloneField,
    value: &str,
    max: usize,
) -> Result<(), LogicalNpcCloneDraftError> {
    validate_nonempty_bounded(field, value, max)?;
    let mut characters = value.char_indices();
    let (_, first) = characters.next().expect("non-empty identifier");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(LogicalNpcCloneDraftError::InvalidIdentifierStart {
            field,
            character: first,
        });
    }
    for (byte_index, character) in characters {
        if !(character.is_ascii_alphanumeric() || character == '_') {
            return Err(LogicalNpcCloneDraftError::InvalidIdentifierCharacter {
                field,
                byte_index,
                character,
            });
        }
    }
    if value.starts_with("__") || is_angelscript_reserved(value) {
        return Err(LogicalNpcCloneDraftError::ReservedIdentifier { field });
    }
    Ok(())
}

fn validate_nonempty_bounded(
    field: LogicalNpcCloneField,
    value: &str,
    max: usize,
) -> Result<(), LogicalNpcCloneDraftError> {
    if value.is_empty() {
        return Err(LogicalNpcCloneDraftError::EmptyValue { field });
    }
    if value.len() > max {
        return Err(LogicalNpcCloneDraftError::ValueTooLong {
            field,
            actual: value.len(),
            max,
        });
    }
    Ok(())
}

fn validate_parent_collisions(
    classes: &LogicalNpcCloneClassNames,
    parent_character_definition: &str,
    parent_ai_agent_config: &str,
    parent_spawn_definition: &str,
) -> Result<(), LogicalNpcCloneDraftError> {
    let parents = [
        (
            LogicalNpcCloneField::ParentCharacterDefinition,
            parent_character_definition,
        ),
        (
            LogicalNpcCloneField::ParentAiAgentConfig,
            parent_ai_agent_config,
        ),
        (
            LogicalNpcCloneField::ParentSpawnDefinition,
            parent_spawn_definition,
        ),
    ];
    let generated = [
        classes.character_definition.as_str(),
        classes.ai_agent_config.as_str(),
        classes.spawn_definition.as_str(),
    ];
    for (field, parent) in parents {
        if let Some(class_name) = generated
            .iter()
            .find(|class_name| parent.eq_ignore_ascii_case(class_name))
        {
            return Err(LogicalNpcCloneDraftError::ClassNameCollision {
                field,
                class_name: (*class_name).to_owned(),
            });
        }
    }
    Ok(())
}

fn is_reserved_portable_path_segment(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || upper.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

fn is_angelscript_reserved(value: &str) -> bool {
    const RESERVED: &[&str] = &[
        "abstract",
        "access",
        "and",
        "and_eq",
        "as",
        "auto",
        "bool",
        "break",
        "case",
        "cast",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "delegate",
        "do",
        "double",
        "else",
        "enum",
        "event",
        "explicit",
        "external",
        "false",
        "final",
        "float",
        "for",
        "from",
        "funcdef",
        "get",
        "if",
        "import",
        "in",
        "inout",
        "int",
        "int8",
        "int16",
        "int32",
        "int64",
        "interface",
        "is",
        "mixin",
        "namespace",
        "not",
        "not_eq",
        "null",
        "or",
        "or_eq",
        "out",
        "override",
        "private",
        "property",
        "protected",
        "return",
        "set",
        "shared",
        "super",
        "switch",
        "struct",
        "this",
        "true",
        "try",
        "typedef",
        "uint",
        "uint8",
        "uint16",
        "uint32",
        "uint64",
        "void",
        "while",
        "xor",
        "xor_eq",
    ];
    RESERVED
        .iter()
        .any(|reserved| value.eq_ignore_ascii_case(reserved))
}
