//! Bounded, offline-only source generation for a discovery-shaped quest draft.
//!
//! The generated module contains exactly one `UG1RQuest` root, one subobjective, their generated
//! defaults, and two read-only lookup helpers. It contains no transition predicate or action,
//! dialog, effect, reward, journal, failure, filesystem, compiler, game, or save operation.

use std::collections::BTreeSet;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::{ContentSeal, EntityId, GameGenerationAnchor, Sha256Digest};

pub const DRAFT_QUEST_GENERATOR_VERSION: u32 = 1;
pub const MAX_DRAFT_QUEST_TITLE_BYTES: usize = 128;
pub const MAX_DRAFT_QUEST_DESCRIPTION_BYTES: usize = 512;
pub const MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES: usize = 128;
pub const MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES: usize = 128;

const MAX_IDENTIFIER_BYTES: usize = 96;
const MAX_MODULE_BYTES: usize = 255;
const MAX_MODULE_SEGMENTS: usize = 16;
const MAX_COLLISION_ENTRIES: usize = 100_000;
const MAX_COLLISION_ENTRY_BYTES: usize = 512;
const MAX_COLLISION_TOTAL_BYTES: usize = 16 * 1024 * 1024;

const QUEST_BASE_CLASS: &str = "UG1RQuest";
const HERO_UNIQUE_NAME: &str = "Hero";
const ROOT_KIND: &str = "EQuestKind::Side";
const OBJECTIVE_KIND: &str = "EQuestKind::Subobjective";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftQuestAuthoringStatus {
    OfflineDraft,
}

/// Source generation never turns a caller-supplied generation seal into runtime evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftQuestDiscoveryStatus {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftQuestTransitionStatus {
    TransitionsRuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftQuestCapabilityStatus {
    pub authoring: DraftQuestAuthoringStatus,
    pub discovery: DraftQuestDiscoveryStatus,
    pub transitions: DraftQuestTransitionStatus,
}

impl DraftQuestCapabilityStatus {
    pub const OFFLINE_DRAFT_RUNTIME_UNQUALIFIED: Self = Self {
        authoring: DraftQuestAuthoringStatus::OfflineDraft,
        discovery: DraftQuestDiscoveryStatus::RuntimeUnqualified,
        transitions: DraftQuestTransitionStatus::TransitionsRuntimeUnqualified,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftQuestFixedShape {
    pub quest_base_class: &'static str,
    pub root_kind: &'static str,
    pub objective_kind: &'static str,
    pub root_external_start: bool,
    pub objective_external_start: bool,
    pub objective_external_success: bool,
    pub objective_succeeds_parent: bool,
}

impl DraftQuestFixedShape {
    pub const DISCOVERY_ONLY: Self = Self {
        quest_base_class: QUEST_BASE_CLASS,
        root_kind: ROOT_KIND,
        objective_kind: OBJECTIVE_KIND,
        root_external_start: true,
        objective_external_start: true,
        objective_external_success: true,
        objective_succeeds_parent: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftQuestField {
    GameGeneration,
    GiverGeneration,
    GiverSourceSeal,
    GiverCatalogLayer,
    GiverSelector,
    GiverRuntimeUniqueName,
    ParentGeneration,
    ParentSourceSeal,
    ParentCatalogLayer,
    ParentSelector,
    ParentRuntimeClass,
    CollisionGeneration,
    CollisionSourceSeal,
    CollisionCatalogLayer,
    ModuleNamespace,
    ModuleSegment { index: usize },
    TechnicalId,
    TextHelper,
    Title,
    Description,
    ObjectiveTitle,
}

impl fmt::Display for DraftQuestField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GameGeneration => formatter.write_str("game generation"),
            Self::GiverGeneration => formatter.write_str("giver generation"),
            Self::GiverSourceSeal => formatter.write_str("giver catalog source seal"),
            Self::GiverCatalogLayer => formatter.write_str("giver catalog layer"),
            Self::GiverSelector => formatter.write_str("giver canonical selector"),
            Self::GiverRuntimeUniqueName => formatter.write_str("giver runtime unique name"),
            Self::ParentGeneration => formatter.write_str("parent quest generation"),
            Self::ParentSourceSeal => formatter.write_str("parent quest catalog source seal"),
            Self::ParentCatalogLayer => formatter.write_str("parent quest catalog layer"),
            Self::ParentSelector => formatter.write_str("parent quest canonical selector"),
            Self::ParentRuntimeClass => formatter.write_str("parent quest runtime class"),
            Self::CollisionGeneration => formatter.write_str("collision catalog generation"),
            Self::CollisionSourceSeal => formatter.write_str("collision catalog source seal"),
            Self::CollisionCatalogLayer => formatter.write_str("collision catalog layer"),
            Self::ModuleNamespace => formatter.write_str("module namespace"),
            Self::ModuleSegment { index } => write!(formatter, "module namespace segment {index}"),
            Self::TechnicalId => formatter.write_str("quest technical id"),
            Self::TextHelper => formatter.write_str("text helper identifier"),
            Self::Title => formatter.write_str("draft quest title"),
            Self::Description => formatter.write_str("draft quest description"),
            Self::ObjectiveTitle => formatter.write_str("draft objective title"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftQuestCollisionKind {
    Module,
    RelativePath,
    Symbol,
}

impl fmt::Display for DraftQuestCollisionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => formatter.write_str("module"),
            Self::RelativePath => formatter.write_str("relative path"),
            Self::Symbol => formatter.write_str("symbol"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftQuestSkeletonError {
    #[error("{field} must have a non-zero byte length")]
    InvalidSeal { field: DraftQuestField },
    #[error("{field} does not match the draft game generation")]
    GenerationMismatch { field: DraftQuestField },
    #[error("quest entity id must not be all zeroes")]
    ZeroEntityId,
    #[error("{field} must not be empty")]
    EmptyValue { field: DraftQuestField },
    #[error("{field} is {actual} bytes; maximum is {max}")]
    ValueTooLong {
        field: DraftQuestField,
        actual: usize,
        max: usize,
    },
    #[error("{field} contains invalid character {character:?} at byte {byte_index}")]
    InvalidCharacter {
        field: DraftQuestField,
        byte_index: usize,
        character: char,
    },
    #[error("{field} is reserved by AngelScript or the generator")]
    ReservedIdentifier { field: DraftQuestField },
    #[error("{field} is not canonical; expected {expected:?}")]
    NonCanonicalIdentifier {
        field: DraftQuestField,
        expected: String,
    },
    #[error("module namespace has {actual} segments; maximum is {max}")]
    TooManyModuleSegments { actual: usize, max: usize },
    #[error("module namespace segment {index} is a reserved portable filesystem name")]
    ReservedModuleSegment { index: usize },
    #[error("parent quest class must begin with \"UQuest_\"")]
    InvalidParentQuestClass,
    #[error("parent quest class collides with generated class {class_name}")]
    ParentClassCollision { class_name: String },
    #[error("{field} must not have leading or trailing whitespace")]
    NonCanonicalText { field: DraftQuestField },
    #[error("collision catalog contains {actual} entries; maximum is {max}")]
    TooManyCollisionEntries { actual: usize, max: usize },
    #[error("collision catalog contains {actual} bytes; maximum is {max}")]
    CollisionCatalogTooLarge { actual: usize, max: usize },
    #[error("unsafe collision {kind} entry {value:?}")]
    UnsafeCollisionEntry {
        kind: DraftQuestCollisionKind,
        value: String,
    },
    #[error("duplicate case-insensitive collision {kind} entry {value:?}")]
    DuplicateCollisionEntry {
        kind: DraftQuestCollisionKind,
        value: String,
    },
    #[error("generated {kind} {name:?} collides with the sealed catalog")]
    GeneratedNameCollision {
        kind: DraftQuestCollisionKind,
        name: String,
    },
    #[error("generated symbols {first:?} and {second:?} collide case-insensitively")]
    GeneratedSymbolCollision { first: String, second: String },
}

/// Exact giver identity from one sealed character catalog layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogQualifiedQuestGiver {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
    canonical_selector: String,
    runtime_unique_name: String,
}

impl CatalogQualifiedQuestGiver {
    pub fn new(
        generation: GameGenerationAnchor,
        source_seal: ContentSeal,
        catalog_layer: impl Into<String>,
        canonical_selector: impl Into<String>,
        runtime_unique_name: impl Into<String>,
    ) -> Result<Self, DraftQuestSkeletonError> {
        validate_generation(DraftQuestField::GiverGeneration, &generation)?;
        validate_seal(DraftQuestField::GiverSourceSeal, &source_seal)?;
        let catalog_layer = catalog_layer.into();
        validate_catalog_layer(DraftQuestField::GiverCatalogLayer, &catalog_layer)?;
        let canonical_selector = canonical_selector.into();
        validate_identifier(
            DraftQuestField::GiverSelector,
            &canonical_selector,
            MAX_IDENTIFIER_BYTES,
        )?;
        let runtime_unique_name = runtime_unique_name.into();
        validate_identifier(
            DraftQuestField::GiverRuntimeUniqueName,
            &runtime_unique_name,
            MAX_IDENTIFIER_BYTES,
        )?;
        Ok(Self {
            generation,
            source_seal,
            catalog_layer,
            canonical_selector,
            runtime_unique_name,
        })
    }

    pub fn generation(&self) -> &GameGenerationAnchor {
        &self.generation
    }

    pub fn source_seal(&self) -> &ContentSeal {
        &self.source_seal
    }

    pub fn catalog_layer(&self) -> &str {
        &self.catalog_layer
    }

    pub fn canonical_selector(&self) -> &str {
        &self.canonical_selector
    }

    pub fn runtime_unique_name(&self) -> &str {
        &self.runtime_unique_name
    }
}

/// Exact parent quest identity from one sealed catalog layer. The runtime class is the only
/// component lowered into source; layer, selector, generation, and seal remain provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogQualifiedParentQuest {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
    canonical_selector: String,
    runtime_class: String,
}

impl CatalogQualifiedParentQuest {
    pub fn new(
        generation: GameGenerationAnchor,
        source_seal: ContentSeal,
        catalog_layer: impl Into<String>,
        canonical_selector: impl Into<String>,
        runtime_class: impl Into<String>,
    ) -> Result<Self, DraftQuestSkeletonError> {
        validate_generation(DraftQuestField::ParentGeneration, &generation)?;
        validate_seal(DraftQuestField::ParentSourceSeal, &source_seal)?;
        let catalog_layer = catalog_layer.into();
        validate_catalog_layer(DraftQuestField::ParentCatalogLayer, &catalog_layer)?;
        let canonical_selector = canonical_selector.into();
        validate_identifier(
            DraftQuestField::ParentSelector,
            &canonical_selector,
            MAX_IDENTIFIER_BYTES,
        )?;
        let runtime_class = runtime_class.into();
        validate_identifier(
            DraftQuestField::ParentRuntimeClass,
            &runtime_class,
            MAX_IDENTIFIER_BYTES,
        )?;
        if !runtime_class.starts_with("UQuest_") {
            return Err(DraftQuestSkeletonError::InvalidParentQuestClass);
        }
        Ok(Self {
            generation,
            source_seal,
            catalog_layer,
            canonical_selector,
            runtime_class,
        })
    }

    pub fn generation(&self) -> &GameGenerationAnchor {
        &self.generation
    }

    pub fn source_seal(&self) -> &ContentSeal {
        &self.source_seal
    }

    pub fn catalog_layer(&self) -> &str {
        &self.catalog_layer
    }

    pub fn canonical_selector(&self) -> &str {
        &self.canonical_selector
    }

    pub fn runtime_class(&self) -> &str {
        &self.runtime_class
    }
}

/// Case-folded inventories must come from a separately sealed catalog. The generator checks them
/// but never treats their seal as runtime qualification evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestCollisionCatalog {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
    modules: BTreeSet<String>,
    relative_paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
}

impl DraftQuestCollisionCatalog {
    pub fn new(
        generation: GameGenerationAnchor,
        source_seal: ContentSeal,
        catalog_layer: impl Into<String>,
        modules: Vec<String>,
        relative_paths: Vec<String>,
        symbols: Vec<String>,
    ) -> Result<Self, DraftQuestSkeletonError> {
        validate_generation(DraftQuestField::CollisionGeneration, &generation)?;
        validate_seal(DraftQuestField::CollisionSourceSeal, &source_seal)?;
        let catalog_layer = catalog_layer.into();
        validate_catalog_layer(DraftQuestField::CollisionCatalogLayer, &catalog_layer)?;
        let count = modules
            .len()
            .checked_add(relative_paths.len())
            .and_then(|count| count.checked_add(symbols.len()))
            .unwrap_or(usize::MAX);
        if count > MAX_COLLISION_ENTRIES {
            return Err(DraftQuestSkeletonError::TooManyCollisionEntries {
                actual: count,
                max: MAX_COLLISION_ENTRIES,
            });
        }
        let bytes = modules
            .iter()
            .chain(&relative_paths)
            .chain(&symbols)
            .try_fold(0usize, |total, value| total.checked_add(value.len()))
            .unwrap_or(usize::MAX);
        if bytes > MAX_COLLISION_TOTAL_BYTES {
            return Err(DraftQuestSkeletonError::CollisionCatalogTooLarge {
                actual: bytes,
                max: MAX_COLLISION_TOTAL_BYTES,
            });
        }

        Ok(Self {
            generation,
            source_seal,
            catalog_layer,
            modules: fold_collision_entries(DraftQuestCollisionKind::Module, modules)?,
            relative_paths: fold_collision_entries(
                DraftQuestCollisionKind::RelativePath,
                relative_paths,
            )?,
            symbols: fold_collision_entries(DraftQuestCollisionKind::Symbol, symbols)?,
        })
    }

    pub fn generation(&self) -> &GameGenerationAnchor {
        &self.generation
    }

    pub fn source_seal(&self) -> &ContentSeal {
        &self.source_seal
    }

    /// The explicitly supplied inventory/loadout layer. This generator checks only this layer;
    /// production qualification must prove coverage of the fully resolved loadout.
    pub fn catalog_layer(&self) -> &str {
        &self.catalog_layer
    }

    fn contains(&self, kind: DraftQuestCollisionKind, value: &str) -> bool {
        let key = value.to_ascii_lowercase();
        match kind {
            DraftQuestCollisionKind::Module => self.modules.contains(&key),
            DraftQuestCollisionKind::RelativePath => self.relative_paths.contains(&key),
            DraftQuestCollisionKind::Symbol => self.symbols.contains(&key),
        }
    }

    fn anchor(&self) -> DraftQuestCatalogLayerAnchor {
        DraftQuestCatalogLayerAnchor {
            generation: self.generation.clone(),
            source_seal: self.source_seal.clone(),
            catalog_layer: self.catalog_layer.clone(),
        }
    }
}

/// Provenance for one sealed catalog/loadout layer. It is not runtime qualification evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestCatalogLayerAnchor {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
}

impl DraftQuestCatalogLayerAnchor {
    pub fn generation(&self) -> &GameGenerationAnchor {
        &self.generation
    }

    pub fn source_seal(&self) -> &ContentSeal {
        &self.source_seal
    }

    pub fn catalog_layer(&self) -> &str {
        &self.catalog_layer
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestTechnicalNames {
    pub module_namespace: String,
    pub module_relative_path: String,
    pub root_class: String,
    pub objective_class: String,
    pub text_helper: String,
    pub root_getter: String,
    pub objective_getter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSkeletonInput {
    pub target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub module_namespace: String,
    /// Canonical uppercase underscore spelling, for example `GORE_PROBE_ASGHAN_MINI`.
    pub technical_id: String,
    pub text_helper: String,
    pub parent_quest: CatalogQualifiedParentQuest,
    pub giver: CatalogQualifiedQuestGiver,
    pub title: String,
    pub description: String,
    pub objective_title: String,
    pub collision_catalog: DraftQuestCollisionCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSkeletonV1 {
    input: DraftQuestSkeletonInput,
    technical_names: DraftQuestTechnicalNames,
    input_fingerprint: Sha256Digest,
}

impl DraftQuestSkeletonV1 {
    pub fn new(input: DraftQuestSkeletonInput) -> Result<Self, DraftQuestSkeletonError> {
        validate_generation(DraftQuestField::GameGeneration, &input.target)?;
        if input.quest_id.as_bytes().iter().all(|byte| *byte == 0) {
            return Err(DraftQuestSkeletonError::ZeroEntityId);
        }
        if input.giver.generation != input.target {
            return Err(DraftQuestSkeletonError::GenerationMismatch {
                field: DraftQuestField::GiverGeneration,
            });
        }
        if input.parent_quest.generation != input.target {
            return Err(DraftQuestSkeletonError::GenerationMismatch {
                field: DraftQuestField::ParentGeneration,
            });
        }
        if input.collision_catalog.generation != input.target {
            return Err(DraftQuestSkeletonError::GenerationMismatch {
                field: DraftQuestField::CollisionGeneration,
            });
        }
        validate_module_namespace(&input.module_namespace)?;
        validate_technical_id(&input.technical_id)?;
        validate_source_function_identifier(
            DraftQuestField::TextHelper,
            &input.text_helper,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_literal_text(
            DraftQuestField::Title,
            &input.title,
            MAX_DRAFT_QUEST_TITLE_BYTES,
        )?;
        validate_literal_text(
            DraftQuestField::Description,
            &input.description,
            MAX_DRAFT_QUEST_DESCRIPTION_BYTES,
        )?;
        validate_literal_text(
            DraftQuestField::ObjectiveTitle,
            &input.objective_title,
            MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES,
        )?;

        let pascal = technical_id_pascal_case(&input.technical_id);
        let technical_names = DraftQuestTechnicalNames {
            module_namespace: input.module_namespace.clone(),
            module_relative_path: format!("{}.as", input.module_namespace.replace('.', "/")),
            root_class: format!("UQuest_{}", input.technical_id),
            objective_class: format!("UQuest_{}_OBJ_DONE", input.technical_id),
            text_helper: input.text_helper.clone(),
            root_getter: format!("Get{pascal}"),
            objective_getter: format!("Get{pascal}Objective"),
        };
        validate_generated_name_lengths(&technical_names)?;
        check_generated_symbol_collisions(&technical_names)?;
        if input
            .parent_quest
            .runtime_class
            .eq_ignore_ascii_case(&technical_names.root_class)
            || input
                .parent_quest
                .runtime_class
                .eq_ignore_ascii_case(&technical_names.objective_class)
        {
            return Err(DraftQuestSkeletonError::ParentClassCollision {
                class_name: input.parent_quest.runtime_class.clone(),
            });
        }
        check_catalog_collisions(&input.collision_catalog, &technical_names)?;
        let input_fingerprint = fingerprint_input(&input);
        Ok(Self {
            input,
            technical_names,
            input_fingerprint,
        })
    }

    pub fn target(&self) -> &GameGenerationAnchor {
        &self.input.target
    }

    pub fn quest_id(&self) -> EntityId {
        self.input.quest_id
    }

    pub fn technical_names(&self) -> &DraftQuestTechnicalNames {
        &self.technical_names
    }

    pub fn input_fingerprint(&self) -> Sha256Digest {
        self.input_fingerprint
    }

    /// Generate only an in-memory source string and metadata. No external action occurs.
    pub fn generate(&self) -> DraftQuestGeneratedSource {
        let names = &self.technical_names;
        let giver = self.input.giver.runtime_unique_name();
        let source = format!(
            "FText {text_helper}(const FName Text)\n{{\n    FString Value = Text.ToString();\n    return FText::FromString(Value);\n}}\n\nclass {root} : {base}\n{{\n    default ParentQuestClass = {parent}::StaticClass();\n    default QuestKind = {root_kind};\n    default InvolvedCharacters.Add(n\"{hero}\");\n    default InvolvedCharacters.Add(n\"{giver}\");\n    default QuestGiverCharacterUniqueName = n\"{giver}\";\n    default NameText = {text_helper}(n\"{title}\");\n    default DescriptionText = {text_helper}(\n        n\"{description}\"\n    );\n    default bExternalStartTrigger = true;\n}}\n\n{root} {root_getter}()\n{{\n    UQuestSubsystem Subsystem = UQuestSubsystem::Get();\n    if (Subsystem == nullptr)\n        return nullptr;\n\n    TSubclassOf<UQuest> QuestClass =\n        TSubclassOf<UQuest>({root}::StaticClass());\n    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);\n    if (Quest == nullptr)\n        return nullptr;\n\n    return Cast<{root}>(Quest);\n}}\n\nclass {objective} : {base}\n{{\n    default ParentQuestClass = {root}::StaticClass();\n    default QuestKind = {objective_kind};\n    default NameText = {text_helper}(n\"{objective_title}\");\n    default bExternalStartTrigger = true;\n    default bExternalSuccessTrigger = true;\n    default bSucceedParent = true;\n}}\n\n{objective} {objective_getter}()\n{{\n    UQuestSubsystem Subsystem = UQuestSubsystem::Get();\n    if (Subsystem == nullptr)\n        return nullptr;\n\n    TSubclassOf<UQuest> QuestClass =\n        TSubclassOf<UQuest>({objective}::StaticClass());\n    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);\n    if (Quest == nullptr)\n        return nullptr;\n\n    return Cast<{objective}>(Quest);\n}}\n",
            text_helper = names.text_helper,
            root = names.root_class,
            base = QUEST_BASE_CLASS,
            parent = self.input.parent_quest.runtime_class(),
            root_kind = ROOT_KIND,
            hero = HERO_UNIQUE_NAME,
            giver = giver,
            title = self.input.title,
            description = self.input.description,
            root_getter = names.root_getter,
            objective = names.objective_class,
            objective_kind = OBJECTIVE_KIND,
            objective_title = self.input.objective_title,
            objective_getter = names.objective_getter,
        );
        let source_sha256 = Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into());
        DraftQuestGeneratedSource {
            target: self.input.target.clone(),
            quest_id: self.input.quest_id,
            generator_version: DRAFT_QUEST_GENERATOR_VERSION,
            giver: self.input.giver.clone(),
            parent_quest: self.input.parent_quest.clone(),
            collision_catalog: self.input.collision_catalog.anchor(),
            technical_names: names.clone(),
            fixed_shape: DraftQuestFixedShape::DISCOVERY_ONLY,
            source,
            source_sha256,
            input_fingerprint: self.input_fingerprint,
            status: DraftQuestCapabilityStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestGeneratedSource {
    pub target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub generator_version: u32,
    pub giver: CatalogQualifiedQuestGiver,
    pub parent_quest: CatalogQualifiedParentQuest,
    pub collision_catalog: DraftQuestCatalogLayerAnchor,
    pub technical_names: DraftQuestTechnicalNames,
    pub fixed_shape: DraftQuestFixedShape,
    pub source: String,
    pub source_sha256: Sha256Digest,
    /// Canonical hash of every semantic/provenance input, separate from emitted source bytes.
    pub input_fingerprint: Sha256Digest,
    pub status: DraftQuestCapabilityStatus,
}

fn validate_generation(
    field: DraftQuestField,
    generation: &GameGenerationAnchor,
) -> Result<(), DraftQuestSkeletonError> {
    validate_seal(field, &generation.executable)
}

fn validate_seal(
    field: DraftQuestField,
    seal: &ContentSeal,
) -> Result<(), DraftQuestSkeletonError> {
    if seal.byte_len == 0 {
        Err(DraftQuestSkeletonError::InvalidSeal { field })
    } else {
        Ok(())
    }
}

/// Canonical catalog-layer ids use lowercase ASCII alphanumeric components separated by one of
/// `.`, `-`, or `_`. This is intentionally display-name-free and can encode provider, edition,
/// pinned version, and loadout identity without accepting ambiguous whitespace or path syntax.
fn validate_catalog_layer(
    field: DraftQuestField,
    value: &str,
) -> Result<(), DraftQuestSkeletonError> {
    validate_nonempty_bounded(field, value, MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES)?;
    let mut previous_was_separator = false;
    for (byte_index, character) in value.char_indices() {
        let separator = matches!(character, '.' | '-' | '_');
        let valid = character.is_ascii_lowercase() || character.is_ascii_digit() || separator;
        if !valid
            || (byte_index == 0 && !character.is_ascii_lowercase())
            || (separator && previous_was_separator)
        {
            return Err(DraftQuestSkeletonError::InvalidCharacter {
                field,
                byte_index,
                character,
            });
        }
        previous_was_separator = separator;
    }
    if previous_was_separator {
        return Err(DraftQuestSkeletonError::NonCanonicalIdentifier {
            field,
            expected: value.trim_end_matches(['.', '-', '_']).to_owned(),
        });
    }
    Ok(())
}

fn validate_module_namespace(value: &str) -> Result<(), DraftQuestSkeletonError> {
    validate_nonempty_bounded(DraftQuestField::ModuleNamespace, value, MAX_MODULE_BYTES)?;
    let segments = value.split('.').collect::<Vec<_>>();
    if segments.len() > MAX_MODULE_SEGMENTS {
        return Err(DraftQuestSkeletonError::TooManyModuleSegments {
            actual: segments.len(),
            max: MAX_MODULE_SEGMENTS,
        });
    }
    for (index, segment) in segments.into_iter().enumerate() {
        validate_identifier(
            DraftQuestField::ModuleSegment { index },
            segment,
            MAX_IDENTIFIER_BYTES,
        )?;
        if is_reserved_portable_segment(segment) {
            return Err(DraftQuestSkeletonError::ReservedModuleSegment { index });
        }
    }
    Ok(())
}

fn validate_technical_id(value: &str) -> Result<(), DraftQuestSkeletonError> {
    validate_identifier(DraftQuestField::TechnicalId, value, MAX_IDENTIFIER_BYTES)?;
    let expected = value.to_ascii_uppercase();
    if value != expected || value.starts_with('_') || value.ends_with('_') || value.contains("__") {
        return Err(DraftQuestSkeletonError::NonCanonicalIdentifier {
            field: DraftQuestField::TechnicalId,
            expected,
        });
    }
    Ok(())
}

fn technical_id_pascal_case(value: &str) -> String {
    value
        .split('_')
        .map(|segment| {
            let mut bytes = segment.to_ascii_lowercase().into_bytes();
            bytes[0] = bytes[0].to_ascii_uppercase();
            String::from_utf8(bytes).expect("validated ASCII technical id")
        })
        .collect()
}

fn validate_generated_name_lengths(
    names: &DraftQuestTechnicalNames,
) -> Result<(), DraftQuestSkeletonError> {
    for (field, value) in [
        (DraftQuestField::TechnicalId, names.root_class.as_str()),
        (DraftQuestField::TechnicalId, names.objective_class.as_str()),
        (DraftQuestField::TextHelper, names.text_helper.as_str()),
        (DraftQuestField::TechnicalId, names.root_getter.as_str()),
        (
            DraftQuestField::TechnicalId,
            names.objective_getter.as_str(),
        ),
    ] {
        if value.len() > MAX_IDENTIFIER_BYTES {
            return Err(DraftQuestSkeletonError::ValueTooLong {
                field,
                actual: value.len(),
                max: MAX_IDENTIFIER_BYTES,
            });
        }
    }
    Ok(())
}

fn fingerprint_input(input: &DraftQuestSkeletonInput) -> Sha256Digest {
    let mut hasher = Sha256::new();
    fingerprint_bytes(
        &mut hasher,
        "schema",
        b"gore-authoring.draft-quest-skeleton-v1.input-fingerprint",
    );
    fingerprint_u64(
        &mut hasher,
        "generator.version",
        u64::from(DRAFT_QUEST_GENERATOR_VERSION),
    );
    fingerprint_seal(&mut hasher, "target.executable", &input.target.executable);
    fingerprint_bytes(&mut hasher, "quest.entity-id", input.quest_id.as_bytes());
    fingerprint_string(&mut hasher, "module.namespace", &input.module_namespace);
    fingerprint_string(&mut hasher, "quest.technical-id", &input.technical_id);
    fingerprint_string(&mut hasher, "source.text-helper", &input.text_helper);

    fingerprint_seal(
        &mut hasher,
        "giver.generation.executable",
        &input.giver.generation.executable,
    );
    fingerprint_seal(
        &mut hasher,
        "giver.catalog-source",
        &input.giver.source_seal,
    );
    fingerprint_string(
        &mut hasher,
        "giver.catalog-layer",
        &input.giver.catalog_layer,
    );
    fingerprint_string(
        &mut hasher,
        "giver.canonical-selector",
        &input.giver.canonical_selector,
    );
    fingerprint_string(
        &mut hasher,
        "giver.runtime-unique-name",
        &input.giver.runtime_unique_name,
    );

    fingerprint_seal(
        &mut hasher,
        "parent.generation.executable",
        &input.parent_quest.generation.executable,
    );
    fingerprint_seal(
        &mut hasher,
        "parent.catalog-source",
        &input.parent_quest.source_seal,
    );
    fingerprint_string(
        &mut hasher,
        "parent.catalog-layer",
        &input.parent_quest.catalog_layer,
    );
    fingerprint_string(
        &mut hasher,
        "parent.canonical-selector",
        &input.parent_quest.canonical_selector,
    );
    fingerprint_string(
        &mut hasher,
        "parent.runtime-class",
        &input.parent_quest.runtime_class,
    );

    fingerprint_string(&mut hasher, "text.title", &input.title);
    fingerprint_string(&mut hasher, "text.description", &input.description);
    fingerprint_string(&mut hasher, "text.objective-title", &input.objective_title);

    fingerprint_seal(
        &mut hasher,
        "collision.generation.executable",
        &input.collision_catalog.generation.executable,
    );
    fingerprint_seal(
        &mut hasher,
        "collision.catalog-source",
        &input.collision_catalog.source_seal,
    );
    fingerprint_string(
        &mut hasher,
        "collision.catalog-layer",
        &input.collision_catalog.catalog_layer,
    );
    fingerprint_set(
        &mut hasher,
        "collision.modules",
        &input.collision_catalog.modules,
    );
    fingerprint_set(
        &mut hasher,
        "collision.relative-paths",
        &input.collision_catalog.relative_paths,
    );
    fingerprint_set(
        &mut hasher,
        "collision.symbols",
        &input.collision_catalog.symbols,
    );

    // Fixed lowering semantics are included as defense-in-depth if implementation constants ever
    // change without the required generator-version bump.
    fingerprint_string(&mut hasher, "shape.quest-base-class", QUEST_BASE_CLASS);
    fingerprint_string(&mut hasher, "shape.hero-unique-name", HERO_UNIQUE_NAME);
    fingerprint_string(&mut hasher, "shape.root-kind", ROOT_KIND);
    fingerprint_string(&mut hasher, "shape.objective-kind", OBJECTIVE_KIND);
    fingerprint_bytes(&mut hasher, "shape.flags", &[1, 1, 1, 1]);
    fingerprint_string(&mut hasher, "capability.authoring", "OfflineDraft");
    fingerprint_string(&mut hasher, "capability.discovery", "RuntimeUnqualified");
    fingerprint_string(
        &mut hasher,
        "capability.transitions",
        "TransitionsRuntimeUnqualified",
    );

    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn fingerprint_seal(hasher: &mut Sha256, field: &str, seal: &ContentSeal) {
    fingerprint_u64(hasher, &format!("{field}.byte-length"), seal.byte_len);
    fingerprint_bytes(hasher, &format!("{field}.sha256"), seal.sha256.as_bytes());
}

fn fingerprint_set(hasher: &mut Sha256, field: &str, values: &BTreeSet<String>) {
    fingerprint_u64(hasher, &format!("{field}.count"), values.len() as u64);
    for value in values {
        fingerprint_string(hasher, field, value);
    }
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

fn check_generated_symbol_collisions(
    names: &DraftQuestTechnicalNames,
) -> Result<(), DraftQuestSkeletonError> {
    let mut seen = std::collections::BTreeMap::<String, &str>::new();
    for symbol in [
        names.root_class.as_str(),
        names.objective_class.as_str(),
        names.text_helper.as_str(),
        names.root_getter.as_str(),
        names.objective_getter.as_str(),
    ] {
        let key = symbol.to_ascii_lowercase();
        if let Some(first) = seen.insert(key, symbol) {
            return Err(DraftQuestSkeletonError::GeneratedSymbolCollision {
                first: first.to_owned(),
                second: symbol.to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_source_function_identifier(
    field: DraftQuestField,
    value: &str,
    max: usize,
) -> Result<(), DraftQuestSkeletonError> {
    validate_identifier(field, value, max)?;
    // gore-as identifies these names as compiler/binding-generated free functions. Our fixed
    // shapes do not use every generated signature, but reserving the complete name family avoids
    // a source function being hidden or colliding as compiler behavior evolves.
    if [
        "StaticClass",
        "Spawn",
        "Get",
        "GetOrCreate",
        "Create",
        "GetG1R",
    ]
    .iter()
    .any(|reserved| value.eq_ignore_ascii_case(reserved))
    {
        return Err(DraftQuestSkeletonError::ReservedIdentifier { field });
    }
    Ok(())
}

fn validate_identifier(
    field: DraftQuestField,
    value: &str,
    max: usize,
) -> Result<(), DraftQuestSkeletonError> {
    validate_nonempty_bounded(field, value, max)?;
    for (index, character) in value.char_indices() {
        let valid = if index == 0 {
            character.is_ascii_alphabetic() || character == '_'
        } else {
            character.is_ascii_alphanumeric() || character == '_'
        };
        if !valid {
            return Err(DraftQuestSkeletonError::InvalidCharacter {
                field,
                byte_index: index,
                character,
            });
        }
    }
    if value.starts_with("__") || is_angelscript_reserved(value) {
        return Err(DraftQuestSkeletonError::ReservedIdentifier { field });
    }
    Ok(())
}

fn validate_literal_text(
    field: DraftQuestField,
    value: &str,
    max: usize,
) -> Result<(), DraftQuestSkeletonError> {
    validate_nonempty_bounded(field, value, max)?;
    if value.trim() != value {
        return Err(DraftQuestSkeletonError::NonCanonicalText { field });
    }
    for (byte_index, character) in value.char_indices() {
        if (!character.is_ascii_graphic() && character != ' ') || matches!(character, '"' | '\\') {
            return Err(DraftQuestSkeletonError::InvalidCharacter {
                field,
                byte_index,
                character,
            });
        }
    }
    Ok(())
}

fn validate_nonempty_bounded(
    field: DraftQuestField,
    value: &str,
    max: usize,
) -> Result<(), DraftQuestSkeletonError> {
    if value.is_empty() {
        return Err(DraftQuestSkeletonError::EmptyValue { field });
    }
    if value.len() > max {
        return Err(DraftQuestSkeletonError::ValueTooLong {
            field,
            actual: value.len(),
            max,
        });
    }
    Ok(())
}

fn fold_collision_entries(
    kind: DraftQuestCollisionKind,
    values: Vec<String>,
) -> Result<BTreeSet<String>, DraftQuestSkeletonError> {
    let mut folded = BTreeSet::new();
    for value in values {
        if !safe_collision_entry(kind, &value) {
            return Err(DraftQuestSkeletonError::UnsafeCollisionEntry { kind, value });
        }
        let key = value.to_ascii_lowercase();
        if !folded.insert(key) {
            return Err(DraftQuestSkeletonError::DuplicateCollisionEntry { kind, value });
        }
    }
    Ok(folded)
}

fn safe_collision_entry(kind: DraftQuestCollisionKind, value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_COLLISION_ENTRY_BYTES
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_control())
        || value.contains('\\')
        || value.contains(':')
        || value.contains('"')
    {
        return false;
    }
    match kind {
        DraftQuestCollisionKind::Module => value
            .split('.')
            .all(|segment| safe_bare_identifier(segment) && !is_reserved_portable_segment(segment)),
        DraftQuestCollisionKind::Symbol => safe_bare_identifier(value),
        DraftQuestCollisionKind::RelativePath => {
            !value.starts_with('/')
                && value.split('/').all(|segment| {
                    !segment.is_empty()
                        && segment != "."
                        && segment != ".."
                        && !is_reserved_portable_segment(segment.trim_end_matches(".as"))
                        && (safe_bare_identifier(segment)
                            || segment
                                .strip_suffix(".as")
                                .is_some_and(safe_bare_identifier))
                })
        }
    }
}

fn safe_bare_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !value.starts_with("__")
        && !is_angelscript_reserved(value)
}

fn check_catalog_collisions(
    catalog: &DraftQuestCollisionCatalog,
    names: &DraftQuestTechnicalNames,
) -> Result<(), DraftQuestSkeletonError> {
    let candidates = [
        (
            DraftQuestCollisionKind::Module,
            names.module_namespace.as_str(),
        ),
        (
            DraftQuestCollisionKind::RelativePath,
            names.module_relative_path.as_str(),
        ),
        (DraftQuestCollisionKind::Symbol, names.root_class.as_str()),
        (
            DraftQuestCollisionKind::Symbol,
            names.objective_class.as_str(),
        ),
        (DraftQuestCollisionKind::Symbol, names.text_helper.as_str()),
        (DraftQuestCollisionKind::Symbol, names.root_getter.as_str()),
        (
            DraftQuestCollisionKind::Symbol,
            names.objective_getter.as_str(),
        ),
    ];
    for (kind, value) in candidates {
        if catalog.contains(kind, value) {
            return Err(DraftQuestSkeletonError::GeneratedNameCollision {
                kind,
                name: value.to_owned(),
            });
        }
    }
    Ok(())
}

fn is_reserved_portable_segment(value: &str) -> bool {
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
