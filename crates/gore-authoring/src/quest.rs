//! Bounded, offline-only source generation for a discovery-shaped quest draft.
//!
//! Version 1 contains exactly one `UG1RQuest` root and one objective. Version 2 retains those
//! identities and adds an ordered, bounded objective list. Both contain generated defaults and
//! read-only lookup helpers but no transition predicate or action, dialog, effect, reward,
//! journal, failure, filesystem, compiler, game, or save operation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use sha2::{Digest, Sha256};

use crate::model_revision3::{
    QuestTransitionConditionAtomV1, QuestTransitionEdgeV1, QuestTransitionEffectKindV1,
    QuestTransitionNodeV1, QuestTransitionPlanV1, QuestTransitionPredicateV1,
    QuestTransitionStateTestV1, QuestTransitionV1, MAX_QUEST_TRANSITION_EFFECTS_V1,
    MAX_QUEST_TRANSITION_PREDICATE_ATOMS_V1, MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1,
};
use crate::{ContentSeal, EntityId, GameGenerationAnchor, Sha256Digest};

/// Stable generator identity for the discovery-only quest skeleton.
pub const DRAFT_QUEST_GENERATOR_ID: &str = "gore-authoring.draft-quest-skeleton";
pub const DRAFT_QUEST_GENERATOR_VERSION: u32 = 1;
/// Generator version for the ordered multi-objective extension. Version 1 remains frozen so
/// existing one-objective projects regenerate byte-for-byte.
pub const DRAFT_QUEST_MULTI_OBJECTIVE_GENERATOR_VERSION: u32 = 2;
pub const DRAFT_QUEST_SEMANTIC_GENERATOR_VERSION: u32 = 3;
pub const MAX_DRAFT_QUEST_TITLE_BYTES: usize = 128;
pub const MAX_DRAFT_QUEST_DESCRIPTION_BYTES: usize = 512;
pub const MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES: usize = 128;
pub const MAX_DRAFT_QUEST_OBJECTIVES: usize = 8;
pub const MAX_DRAFT_QUEST_OBJECTIVE_TITLES_BYTES: usize =
    MAX_DRAFT_QUEST_OBJECTIVES * MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES;
pub const MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES: usize = 128;

const MAX_IDENTIFIER_BYTES: usize = 96;
const MAX_MODULE_BYTES: usize = 255;
const MAX_MODULE_SEGMENTS: usize = 16;
pub(crate) const MAX_COLLISION_ENTRIES: usize = 100_000;
pub(crate) const MAX_COLLISION_ENTRY_BYTES: usize = 512;
pub(crate) const MAX_COLLISION_TOTAL_BYTES: usize = 16 * 1024 * 1024;

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
    AdditionalObjectiveTitle { index: usize },
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
            Self::ObjectiveTitle => formatter.write_str("draft objective title 1"),
            Self::AdditionalObjectiveTitle { index } => {
                write!(formatter, "draft objective title {}", index + 1)
            }
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
    #[error("draft has {actual} objectives; maximum is {max}")]
    TooManyObjectives { actual: usize, max: usize },
    #[error("draft objective titles contain {actual} bytes; maximum is {max}")]
    ObjectiveTitlesTooLarge { actual: usize, max: usize },
    #[error("draft objective titles {first} and {second} are duplicates")]
    DuplicateObjectiveTitle { first: usize, second: usize },
    #[error("invalid semantic Quest transition plan: {reason}")]
    InvalidTransitionPlan { reason: String },
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
            generator_id: DRAFT_QUEST_GENERATOR_ID,
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

/// Technical identities added by the ordered multi-objective generator. Objective 1 deliberately
/// keeps the frozen version-1 class/getter names in [DraftQuestTechnicalNames].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestAdditionalObjectiveTechnicalNames {
    pub ordinal: usize,
    pub objective_class: String,
    pub objective_getter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestMultiObjectiveTechnicalNames {
    pub base: DraftQuestTechnicalNames,
    pub additional_objectives: Vec<DraftQuestAdditionalObjectiveTechnicalNames>,
}

/// Version-2 input extends the frozen version-1 shell without changing its serialized or emitted
/// shape. The additional list must be non-empty; callers with one objective continue to use V1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSkeletonInputV2 {
    pub base: DraftQuestSkeletonInput,
    pub additional_objective_titles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSkeletonV2 {
    input: DraftQuestSkeletonInput,
    additional_objective_titles: Vec<String>,
    technical_names: DraftQuestMultiObjectiveTechnicalNames,
    input_fingerprint: Sha256Digest,
}

/// Validate the complete ordered title list independently of source/collision inputs. This is
/// shared by the closed revision-3 model and the generator so malformed persisted semantics fail
/// before artifact authority is consulted.
pub fn validate_draft_quest_objective_titles(
    first: &str,
    additional: &[String],
) -> Result<(), DraftQuestSkeletonError> {
    let objective_count = 1usize.saturating_add(additional.len());
    if objective_count > MAX_DRAFT_QUEST_OBJECTIVES {
        return Err(DraftQuestSkeletonError::TooManyObjectives {
            actual: objective_count,
            max: MAX_DRAFT_QUEST_OBJECTIVES,
        });
    }
    let title_bytes = std::iter::once(first)
        .chain(additional.iter().map(String::as_str))
        .try_fold(0usize, |total, title| total.checked_add(title.len()))
        .unwrap_or(usize::MAX);
    if title_bytes > MAX_DRAFT_QUEST_OBJECTIVE_TITLES_BYTES {
        return Err(DraftQuestSkeletonError::ObjectiveTitlesTooLarge {
            actual: title_bytes,
            max: MAX_DRAFT_QUEST_OBJECTIVE_TITLES_BYTES,
        });
    }

    let mut folded_titles = std::collections::BTreeMap::<String, usize>::new();
    for (zero_based_index, title) in std::iter::once(first)
        .chain(additional.iter().map(String::as_str))
        .enumerate()
    {
        let field = if zero_based_index == 0 {
            DraftQuestField::ObjectiveTitle
        } else {
            DraftQuestField::AdditionalObjectiveTitle {
                index: zero_based_index,
            }
        };
        validate_literal_text(field, title, MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES)?;
        let folded = title.to_ascii_lowercase();
        if let Some(first_index) = folded_titles.insert(folded, zero_based_index) {
            return Err(DraftQuestSkeletonError::DuplicateObjectiveTitle {
                first: first_index + 1,
                second: zero_based_index + 1,
            });
        }
    }
    Ok(())
}

impl QuestTransitionPlanV1 {
    /// Upgrade the frozen v2/v3 objective shape without changing its emitted lifecycle behavior.
    pub fn legacy_seed(objective_count: usize) -> Result<Self, DraftQuestSkeletonError> {
        if objective_count == 0 {
            return Err(invalid_transition_plan(
                "a Quest transition plan must contain at least one objective",
            ));
        }
        if objective_count > MAX_DRAFT_QUEST_OBJECTIVES {
            return Err(DraftQuestSkeletonError::TooManyObjectives {
                actual: objective_count,
                max: MAX_DRAFT_QUEST_OBJECTIVES,
            });
        }
        let objective_slots = (1..=objective_count)
            .map(|ordinal| ordinal as u16)
            .collect::<Vec<_>>();
        let mut transitions = vec![
            external_transition(
                QuestTransitionNodeV1::Root,
                QuestTransitionEdgeV1::Availability,
                false,
            ),
            external_transition(
                QuestTransitionNodeV1::Root,
                QuestTransitionEdgeV1::Start,
                false,
            ),
        ];
        for (index, slot) in objective_slots.iter().copied().enumerate() {
            let node = QuestTransitionNodeV1::Objective { slot };
            transitions.push(external_transition(
                node,
                QuestTransitionEdgeV1::Availability,
                false,
            ));
            transitions.push(external_transition(
                node,
                QuestTransitionEdgeV1::Start,
                false,
            ));
            transitions.push(external_transition(
                node,
                QuestTransitionEdgeV1::Success,
                index + 1 == objective_count,
            ));
        }
        Ok(Self {
            objective_order: objective_slots.clone(),
            next_slot_ordinal: (objective_count + 1) as u16,
            objective_slots,
            transitions,
        })
    }
}

fn external_transition(
    node: QuestTransitionNodeV1,
    edge: QuestTransitionEdgeV1,
    succeeds_parent: bool,
) -> QuestTransitionV1 {
    QuestTransitionV1 {
        node,
        edge,
        external_allowed: true,
        predicate: None,
        effects: Vec::new(),
        succeeds_parent,
    }
}

/// Validate a semantic transition plan independently of source/collision inputs.
///
/// `objective_count` is the number of positional titles carried by the legacy input fields.
pub fn validate_draft_quest_transition_plan_v1(
    plan: &QuestTransitionPlanV1,
    objective_count: usize,
) -> Result<(), DraftQuestSkeletonError> {
    if objective_count == 0 {
        return Err(invalid_transition_plan(
            "a Quest transition plan must contain at least one objective",
        ));
    }
    if objective_count > MAX_DRAFT_QUEST_OBJECTIVES {
        return Err(DraftQuestSkeletonError::TooManyObjectives {
            actual: objective_count,
            max: MAX_DRAFT_QUEST_OBJECTIVES,
        });
    }
    if plan.objective_slots.len() != objective_count {
        return Err(invalid_transition_plan(format!(
            "objective_slots has {} entries but the Quest carries {objective_count} titles",
            plan.objective_slots.len()
        )));
    }
    if plan.objective_slots.contains(&0)
        || plan
            .objective_slots
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(invalid_transition_plan(
            "objective_slots must be unique non-zero ordinals in ascending order",
        ));
    }
    let active_slots = plan
        .objective_slots
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if !active_slots.contains(&1) {
        return Err(invalid_transition_plan(
            "objective slot 1 is the frozen legacy identity and must remain active",
        ));
    }
    let ordered_slots = plan
        .objective_order
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if plan.objective_order.len() != plan.objective_slots.len()
        || ordered_slots.len() != plan.objective_order.len()
        || ordered_slots != active_slots
    {
        return Err(invalid_transition_plan(
            "objective_order must be a full permutation of objective_slots",
        ));
    }
    let max_slot = plan.objective_slots.last().copied().unwrap_or(0);
    if plan.next_slot_ordinal == 0 || plan.next_slot_ordinal <= max_slot {
        return Err(invalid_transition_plan(
            "next_slot_ordinal must be strictly greater than every active objective slot",
        ));
    }

    let valid_node = |node: QuestTransitionNodeV1| match node {
        QuestTransitionNodeV1::Root => true,
        QuestTransitionNodeV1::Objective { slot } => active_slots.contains(&slot),
    };
    let mut by_edge =
        BTreeMap::<(QuestTransitionNodeV1, QuestTransitionEdgeV1), &QuestTransitionV1>::new();
    let mut effect_graphs = BTreeMap::<
        QuestTransitionEffectKindV1,
        BTreeMap<QuestTransitionNodeV1, BTreeSet<QuestTransitionNodeV1>>,
    >::new();

    if plan
        .transitions
        .windows(2)
        .any(|pair| (pair[0].node, pair[0].edge) >= (pair[1].node, pair[1].edge))
    {
        return Err(invalid_transition_plan(
            "transitions must be unique and sorted by node then lifecycle edge",
        ));
    }

    for transition in &plan.transitions {
        if !valid_node(transition.node) {
            return Err(invalid_transition_plan(
                "a transition refers to an inactive objective slot",
            ));
        }
        if by_edge
            .insert((transition.node, transition.edge), transition)
            .is_some()
        {
            return Err(invalid_transition_plan(
                "each node may define at most one transition per lifecycle edge",
            ));
        }
        if !transition.external_allowed && transition.predicate.is_none() {
            return Err(invalid_transition_plan(
                "every transition requires an external or predicate driver",
            ));
        }
        if let Some(predicate) = &transition.predicate {
            validate_transition_predicate(predicate, &valid_node)?;
        }
        if transition.effects.len() > MAX_QUEST_TRANSITION_EFFECTS_V1 {
            return Err(invalid_transition_plan(format!(
                "a transition has {} effects; maximum is {MAX_QUEST_TRANSITION_EFFECTS_V1}",
                transition.effects.len()
            )));
        }
        if transition.edge == QuestTransitionEdgeV1::Availability && !transition.effects.is_empty()
        {
            return Err(invalid_transition_plan(
                "availability predicates have no lifecycle handler and cannot carry effects",
            ));
        }
        if transition.succeeds_parent
            && (!matches!(transition.node, QuestTransitionNodeV1::Objective { .. })
                || transition.edge != QuestTransitionEdgeV1::Success)
        {
            return Err(invalid_transition_plan(
                "succeeds_parent is valid only on an objective success edge",
            ));
        }

        let mut exact_effects = BTreeSet::new();
        let mut terminal_by_target = BTreeMap::new();
        if transition.succeeds_parent {
            // `bSucceedParent` is an implicit Succeed(root) action. Model it in the same
            // conflict/cycle graph as explicit handler effects so source generation cannot emit
            // order-dependent parent terminal behavior.
            terminal_by_target.insert(
                QuestTransitionNodeV1::Root,
                QuestTransitionEffectKindV1::Succeed,
            );
            effect_graphs
                .entry(QuestTransitionEffectKindV1::Succeed)
                .or_default()
                .entry(transition.node)
                .or_default()
                .insert(QuestTransitionNodeV1::Root);
        }
        if transition.effects.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid_transition_plan(
                "effects must be unique and sorted by target then effect kind",
            ));
        }
        for effect in &transition.effects {
            if !valid_node(effect.target) {
                return Err(invalid_transition_plan(
                    "an effect targets an inactive objective slot",
                ));
            }
            if effect.target == transition.node {
                return Err(invalid_transition_plan(
                    "a transition cannot apply an effect to its own node",
                ));
            }
            if !exact_effects.insert(effect.clone()) {
                return Err(invalid_transition_plan(
                    "a transition contains a duplicate effect",
                ));
            }
            if matches!(
                effect.effect,
                QuestTransitionEffectKindV1::Succeed | QuestTransitionEffectKindV1::Fail
            ) {
                if let Some(previous) = terminal_by_target.insert(effect.target, effect.effect) {
                    if previous == effect.effect {
                        return Err(invalid_transition_plan(
                            "an explicit effect duplicates implicit parent success",
                        ));
                    } else {
                        return Err(invalid_transition_plan(
                            "one handler cannot both succeed and fail the same target",
                        ));
                    }
                }
            }
            effect_graphs
                .entry(effect.effect)
                .or_default()
                .entry(transition.node)
                .or_default()
                .insert(effect.target);
        }
    }

    let nodes = std::iter::once(QuestTransitionNodeV1::Root)
        .chain(
            plan.objective_slots
                .iter()
                .copied()
                .map(|slot| QuestTransitionNodeV1::Objective { slot }),
        )
        .collect::<Vec<_>>();
    for node in &nodes {
        for edge in [
            QuestTransitionEdgeV1::Availability,
            QuestTransitionEdgeV1::Start,
        ] {
            if !by_edge.contains_key(&(*node, edge)) {
                return Err(invalid_transition_plan(
                    "every plan node requires availability and start transitions",
                ));
            }
        }
        if matches!(node, QuestTransitionNodeV1::Objective { .. })
            && !by_edge.contains_key(&(*node, QuestTransitionEdgeV1::Success))
            && !by_edge.contains_key(&(*node, QuestTransitionEdgeV1::Failure))
        {
            return Err(invalid_transition_plan(
                "every objective requires a success or failure transition",
            ));
        }
        let success = by_edge.get(&(*node, QuestTransitionEdgeV1::Success));
        let failure = by_edge.get(&(*node, QuestTransitionEdgeV1::Failure));
        if let (Some(success), Some(failure)) = (success, failure) {
            if matches!(
                (&success.predicate, &failure.predicate),
                (Some(success), Some(failure)) if transition_predicates_may_overlap(success, failure)
            ) {
                return Err(invalid_transition_plan(
                    "success and failure automatic predicates must be provably disjoint",
                ));
            }
        }
    }
    for graph in effect_graphs.values() {
        reject_effect_cycle(graph, &nodes)?;
    }
    Ok(())
}

fn transition_predicates_may_overlap(
    left: &QuestTransitionPredicateV1,
    right: &QuestTransitionPredicateV1,
) -> bool {
    left.any_of.iter().any(|left_group| {
        right.any_of.iter().any(|right_group| {
            let mut polarities = BTreeMap::new();
            for atom in left_group.all_of.iter().chain(&right_group.all_of) {
                let key = (atom.node, atom.test);
                if polarities
                    .insert(key, atom.negated)
                    .is_some_and(|previous| previous != atom.negated)
                {
                    return false;
                }
            }
            reject_lifecycle_state_contradictions(&polarities).is_ok()
        })
    })
}

fn validate_transition_predicate(
    predicate: &QuestTransitionPredicateV1,
    valid_node: &impl Fn(QuestTransitionNodeV1) -> bool,
) -> Result<(), DraftQuestSkeletonError> {
    if predicate.any_of.is_empty()
        || predicate.any_of.len() > MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1
    {
        return Err(invalid_transition_plan(format!(
            "predicate any_of must contain 1..={MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1} groups"
        )));
    }
    let mut groups = BTreeSet::<BTreeSet<QuestTransitionConditionAtomV1>>::new();
    if predicate.any_of.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(invalid_transition_plan(
            "predicate conjunctions must be unique and sorted lexicographically",
        ));
    }
    for group in &predicate.any_of {
        if group.all_of.is_empty() || group.all_of.len() > MAX_QUEST_TRANSITION_PREDICATE_ATOMS_V1 {
            return Err(invalid_transition_plan(format!(
                "predicate all_of must contain 1..={MAX_QUEST_TRANSITION_PREDICATE_ATOMS_V1} atoms"
            )));
        }
        let mut atoms = BTreeSet::new();
        let mut polarities = BTreeMap::new();
        if group.all_of.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid_transition_plan(
                "predicate atoms must be unique and sorted lexicographically",
            ));
        }
        for atom in &group.all_of {
            if !valid_node(atom.node) {
                return Err(invalid_transition_plan(
                    "a predicate atom refers to an inactive objective slot",
                ));
            }
            if !atoms.insert(atom.clone()) {
                return Err(invalid_transition_plan(
                    "a predicate conjunction contains a duplicate atom",
                ));
            }
            let key = (atom.node, atom.test);
            if let Some(previous) = polarities.insert(key, atom.negated) {
                if previous != atom.negated {
                    return Err(invalid_transition_plan(
                        "a predicate conjunction contains a direct contradiction",
                    ));
                }
            }
        }
        reject_lifecycle_state_contradictions(&polarities)?;
        if !groups.insert(atoms) {
            return Err(invalid_transition_plan(
                "a predicate contains a duplicate conjunction",
            ));
        }
    }
    Ok(())
}

fn reject_lifecycle_state_contradictions(
    polarities: &BTreeMap<(QuestTransitionNodeV1, QuestTransitionStateTestV1), bool>,
) -> Result<(), DraftQuestSkeletonError> {
    let nodes = polarities
        .keys()
        .map(|(node, _)| *node)
        .collect::<BTreeSet<_>>();
    for node in nodes {
        let positive = |test| polarities.get(&(node, test)) == Some(&false);
        let negative = |test| polarities.get(&(node, test)) == Some(&true);
        let terminal = positive(QuestTransitionStateTestV1::Succeeded)
            || positive(QuestTransitionStateTestV1::Failed)
            || positive(QuestTransitionStateTestV1::Completed);
        let contradictory = (positive(QuestTransitionStateTestV1::Succeeded)
            && positive(QuestTransitionStateTestV1::Failed))
            || (positive(QuestTransitionStateTestV1::Running) && terminal)
            || (negative(QuestTransitionStateTestV1::Started)
                && (positive(QuestTransitionStateTestV1::Running) || terminal))
            || (negative(QuestTransitionStateTestV1::Completed)
                && (positive(QuestTransitionStateTestV1::Succeeded)
                    || positive(QuestTransitionStateTestV1::Failed)))
            || (positive(QuestTransitionStateTestV1::Completed)
                && negative(QuestTransitionStateTestV1::Succeeded)
                && negative(QuestTransitionStateTestV1::Failed));
        if contradictory {
            return Err(invalid_transition_plan(
                "a predicate conjunction contains incompatible lifecycle states",
            ));
        }
    }
    Ok(())
}

fn reject_effect_cycle(
    graph: &BTreeMap<QuestTransitionNodeV1, BTreeSet<QuestTransitionNodeV1>>,
    nodes: &[QuestTransitionNodeV1],
) -> Result<(), DraftQuestSkeletonError> {
    fn visit(
        node: QuestTransitionNodeV1,
        graph: &BTreeMap<QuestTransitionNodeV1, BTreeSet<QuestTransitionNodeV1>>,
        visiting: &mut BTreeSet<QuestTransitionNodeV1>,
        visited: &mut BTreeSet<QuestTransitionNodeV1>,
    ) -> bool {
        if visited.contains(&node) {
            return false;
        }
        if !visiting.insert(node) {
            return true;
        }
        if graph.get(&node).is_some_and(|targets| {
            targets
                .iter()
                .copied()
                .any(|target| visit(target, graph, visiting, visited))
        }) {
            return true;
        }
        visiting.remove(&node);
        visited.insert(node);
        false
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in nodes {
        if visit(*node, graph, &mut visiting, &mut visited) {
            return Err(invalid_transition_plan(
                "same-kind transition effects must not form a cycle",
            ));
        }
    }
    Ok(())
}

fn invalid_transition_plan(reason: impl Into<String>) -> DraftQuestSkeletonError {
    DraftQuestSkeletonError::InvalidTransitionPlan {
        reason: reason.into(),
    }
}

impl DraftQuestSkeletonV2 {
    pub fn new(input: DraftQuestSkeletonInputV2) -> Result<Self, DraftQuestSkeletonError> {
        let DraftQuestSkeletonInputV2 {
            base,
            additional_objective_titles,
        } = input;
        let objective_count = 1usize.saturating_add(additional_objective_titles.len());
        if objective_count > MAX_DRAFT_QUEST_OBJECTIVES {
            return Err(DraftQuestSkeletonError::TooManyObjectives {
                actual: objective_count,
                max: MAX_DRAFT_QUEST_OBJECTIVES,
            });
        }
        if additional_objective_titles.is_empty() {
            return Err(DraftQuestSkeletonError::EmptyValue {
                field: DraftQuestField::AdditionalObjectiveTitle { index: 1 },
            });
        }

        validate_draft_quest_objective_titles(&base.objective_title, &additional_objective_titles)?;

        // V1 closes the complete old validation/collision surface first. This is what guarantees
        // a version-2 extension cannot weaken the frozen one-objective contract.
        let frozen = DraftQuestSkeletonV1::new(base)?;
        let DraftQuestSkeletonV1 {
            input,
            technical_names: base_names,
            ..
        } = frozen;
        let mut additional_names = Vec::with_capacity(additional_objective_titles.len());
        for ordinal in 2..=objective_count {
            additional_names.push(DraftQuestAdditionalObjectiveTechnicalNames {
                ordinal,
                objective_class: format!("UQuest_{}_OBJ_{ordinal}", input.technical_id),
                objective_getter: format!("{}{}", base_names.objective_getter, ordinal),
            });
        }

        validate_multi_objective_name_lengths(&additional_names)?;
        check_multi_objective_symbol_collisions(&base_names, &additional_names)?;
        for names in &additional_names {
            if input
                .parent_quest
                .runtime_class
                .eq_ignore_ascii_case(&names.objective_class)
            {
                return Err(DraftQuestSkeletonError::ParentClassCollision {
                    class_name: input.parent_quest.runtime_class.clone(),
                });
            }
            for symbol in [&names.objective_class, &names.objective_getter] {
                if input
                    .collision_catalog
                    .contains(DraftQuestCollisionKind::Symbol, symbol)
                {
                    return Err(DraftQuestSkeletonError::GeneratedNameCollision {
                        kind: DraftQuestCollisionKind::Symbol,
                        name: symbol.clone(),
                    });
                }
            }
        }

        let technical_names = DraftQuestMultiObjectiveTechnicalNames {
            base: base_names,
            additional_objectives: additional_names,
        };
        let input_fingerprint = fingerprint_multi_objective_input(
            &input,
            &additional_objective_titles,
            &technical_names,
        );
        Ok(Self {
            input,
            additional_objective_titles,
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

    pub fn technical_names(&self) -> &DraftQuestMultiObjectiveTechnicalNames {
        &self.technical_names
    }

    pub fn input_fingerprint(&self) -> Sha256Digest {
        self.input_fingerprint
    }

    /// Emit a deterministic root followed by objectives in author order. The first objective
    /// retains the V1 identities; only the final objective carries `bSucceedParent = true`.
    /// This is source shape, not runtime transition qualification.
    pub fn generate(&self) -> DraftQuestMultiObjectiveGeneratedSource {
        let names = &self.technical_names.base;
        let giver = self.input.giver.runtime_unique_name();
        let mut source = format!(
            "FText {text_helper}(const FName Text)\n{{\n    FString Value = Text.ToString();\n    return FText::FromString(Value);\n}}\n\nclass {root} : {base}\n{{\n    default ParentQuestClass = {parent}::StaticClass();\n    default QuestKind = {root_kind};\n    default InvolvedCharacters.Add(n\"{hero}\");\n    default InvolvedCharacters.Add(n\"{giver}\");\n    default QuestGiverCharacterUniqueName = n\"{giver}\";\n    default NameText = {text_helper}(n\"{title}\");\n    default DescriptionText = {text_helper}(\n        n\"{description}\"\n    );\n    default bExternalStartTrigger = true;\n}}\n\n{root} {root_getter}()\n{{\n    UQuestSubsystem Subsystem = UQuestSubsystem::Get();\n    if (Subsystem == nullptr)\n        return nullptr;\n\n    TSubclassOf<UQuest> QuestClass =\n        TSubclassOf<UQuest>({root}::StaticClass());\n    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);\n    if (Quest == nullptr)\n        return nullptr;\n\n    return Cast<{root}>(Quest);\n}}\n\n",
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
        );

        let objective_count = 1 + self.additional_objective_titles.len();
        let objectives = std::iter::once((
            names.objective_class.as_str(),
            names.objective_getter.as_str(),
            self.input.objective_title.as_str(),
        ))
        .chain(
            self.technical_names
                .additional_objectives
                .iter()
                .zip(&self.additional_objective_titles)
                .map(|(names, title)| {
                    (
                        names.objective_class.as_str(),
                        names.objective_getter.as_str(),
                        title.as_str(),
                    )
                }),
        );
        for (zero_based_index, (objective, objective_getter, objective_title)) in
            objectives.enumerate()
        {
            source.push_str(&format!(
                "class {objective} : {base}\n{{\n    default ParentQuestClass = {root}::StaticClass();\n    default QuestKind = {objective_kind};\n    default NameText = {text_helper}(n\"{objective_title}\");\n    default bExternalStartTrigger = true;\n    default bExternalSuccessTrigger = true;\n",
                base = QUEST_BASE_CLASS,
                root = names.root_class,
                objective_kind = OBJECTIVE_KIND,
                text_helper = names.text_helper,
            ));
            if zero_based_index + 1 == objective_count {
                source.push_str("    default bSucceedParent = true;\n");
            }
            source.push_str(&format!(
                "}}\n\n{objective} {objective_getter}()\n{{\n    UQuestSubsystem Subsystem = UQuestSubsystem::Get();\n    if (Subsystem == nullptr)\n        return nullptr;\n\n    TSubclassOf<UQuest> QuestClass =\n        TSubclassOf<UQuest>({objective}::StaticClass());\n    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);\n    if (Quest == nullptr)\n        return nullptr;\n\n    return Cast<{objective}>(Quest);\n}}\n",
            ));
            if zero_based_index + 1 != objective_count {
                source.push('\n');
            }
        }
        let source_sha256 = Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into());
        DraftQuestMultiObjectiveGeneratedSource {
            target: self.input.target.clone(),
            quest_id: self.input.quest_id,
            generator_id: DRAFT_QUEST_GENERATOR_ID,
            generator_version: DRAFT_QUEST_MULTI_OBJECTIVE_GENERATOR_VERSION,
            giver: self.input.giver.clone(),
            parent_quest: self.input.parent_quest.clone(),
            collision_catalog: self.input.collision_catalog.anchor(),
            technical_names: self.technical_names.clone(),
            fixed_shape: DraftQuestFixedShape::DISCOVERY_ONLY,
            source,
            source_sha256,
            input_fingerprint: self.input_fingerprint,
            status: DraftQuestCapabilityStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSemanticObjectiveTechnicalNames {
    pub slot: u16,
    pub objective_class: String,
    pub objective_getter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSemanticTechnicalNames {
    pub base: DraftQuestTechnicalNames,
    /// Stable technical identities in ascending slot order, independent of presentation order.
    pub objectives: Vec<DraftQuestSemanticObjectiveTechnicalNames>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSkeletonInputV3 {
    pub base: DraftQuestSkeletonInput,
    pub additional_objective_titles: Vec<String>,
    pub transition_plan: QuestTransitionPlanV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSkeletonV3 {
    input: DraftQuestSkeletonInput,
    additional_objective_titles: Vec<String>,
    transition_plan: QuestTransitionPlanV1,
    technical_names: DraftQuestSemanticTechnicalNames,
    input_fingerprint: Sha256Digest,
}

impl DraftQuestSkeletonV3 {
    pub fn new(input: DraftQuestSkeletonInputV3) -> Result<Self, DraftQuestSkeletonError> {
        let DraftQuestSkeletonInputV3 {
            base,
            additional_objective_titles,
            transition_plan,
        } = input;
        validate_draft_quest_objective_titles(&base.objective_title, &additional_objective_titles)?;
        let objective_count = 1 + additional_objective_titles.len();
        validate_draft_quest_transition_plan_v1(&transition_plan, objective_count)?;

        // Close the frozen validation surface first. Slot 1 is retained by every V4 plan, so all
        // V1 technical-name and catalog checks remain applicable without weakening compatibility.
        let frozen = DraftQuestSkeletonV1::new(base)?;
        let DraftQuestSkeletonV1 {
            input,
            technical_names: base_names,
            ..
        } = frozen;
        let objectives = transition_plan
            .objective_slots
            .iter()
            .copied()
            .map(|slot| {
                if slot == 1 {
                    DraftQuestSemanticObjectiveTechnicalNames {
                        slot,
                        objective_class: base_names.objective_class.clone(),
                        objective_getter: base_names.objective_getter.clone(),
                    }
                } else {
                    DraftQuestSemanticObjectiveTechnicalNames {
                        slot,
                        objective_class: format!("UQuest_{}_OBJ_{slot}", input.technical_id),
                        objective_getter: format!("{}{slot}", base_names.objective_getter),
                    }
                }
            })
            .collect::<Vec<_>>();
        validate_semantic_objective_names(&objectives)?;
        check_semantic_symbol_collisions(&base_names, &objectives)?;
        for objective in objectives.iter().filter(|objective| objective.slot != 1) {
            if input
                .parent_quest
                .runtime_class
                .eq_ignore_ascii_case(&objective.objective_class)
            {
                return Err(DraftQuestSkeletonError::ParentClassCollision {
                    class_name: input.parent_quest.runtime_class.clone(),
                });
            }
            for symbol in [&objective.objective_class, &objective.objective_getter] {
                if input
                    .collision_catalog
                    .contains(DraftQuestCollisionKind::Symbol, symbol)
                {
                    return Err(DraftQuestSkeletonError::GeneratedNameCollision {
                        kind: DraftQuestCollisionKind::Symbol,
                        name: symbol.clone(),
                    });
                }
            }
        }
        let technical_names = DraftQuestSemanticTechnicalNames {
            base: base_names,
            objectives,
        };
        let input_fingerprint = fingerprint_semantic_quest_input(
            &input,
            &additional_objective_titles,
            &transition_plan,
        );
        Ok(Self {
            input,
            additional_objective_titles,
            transition_plan,
            technical_names,
            input_fingerprint,
        })
    }

    pub fn technical_names(&self) -> &DraftQuestSemanticTechnicalNames {
        &self.technical_names
    }

    pub fn input_fingerprint(&self) -> Sha256Digest {
        self.input_fingerprint
    }

    pub fn generate(&self) -> DraftQuestSemanticGeneratedSource {
        let base_names = &self.technical_names.base;
        let giver = self.input.giver.runtime_unique_name();
        let mut source = format!(
            "FText {text_helper}(const FName Text)\n{{\n    FString Value = Text.ToString();\n    return FText::FromString(Value);\n}}\n\nclass {root} : {base}\n{{\n    default ParentQuestClass = {parent}::StaticClass();\n    default QuestKind = {root_kind};\n    default InvolvedCharacters.Add(n\"{hero}\");\n    default InvolvedCharacters.Add(n\"{giver}\");\n    default QuestGiverCharacterUniqueName = n\"{giver}\";\n    default NameText = {text_helper}(n\"{title}\");\n    default DescriptionText = {text_helper}(\n        n\"{description}\"\n    );\n",
            text_helper = base_names.text_helper,
            root = base_names.root_class,
            base = QUEST_BASE_CLASS,
            parent = self.input.parent_quest.runtime_class(),
            root_kind = ROOT_KIND,
            hero = HERO_UNIQUE_NAME,
            giver = giver,
            title = self.input.title,
            description = self.input.description,
        );
        self.render_node_class_tail(&mut source, QuestTransitionNodeV1::Root);
        source.push_str("}\n\n");
        render_quest_getter(&mut source, &base_names.root_class, &base_names.root_getter);
        source.push('\n');

        let titles = std::iter::once(self.input.objective_title.as_str())
            .chain(self.additional_objective_titles.iter().map(String::as_str))
            .collect::<Vec<_>>();
        for (position, slot) in self
            .transition_plan
            .objective_order
            .iter()
            .copied()
            .enumerate()
        {
            let objective = self.objective_names(slot);
            source.push_str(&format!(
                "class {objective} : {base}\n{{\n    default ParentQuestClass = {root}::StaticClass();\n    default QuestKind = {objective_kind};\n    default NameText = {text_helper}(n\"{objective_title}\");\n",
                objective = objective.objective_class,
                base = QUEST_BASE_CLASS,
                root = base_names.root_class,
                objective_kind = OBJECTIVE_KIND,
                text_helper = base_names.text_helper,
                objective_title = titles[position],
            ));
            self.render_node_class_tail(&mut source, QuestTransitionNodeV1::Objective { slot });
            source.push_str("}\n\n");
            render_quest_getter(
                &mut source,
                &objective.objective_class,
                &objective.objective_getter,
            );
            if position + 1 != self.transition_plan.objective_order.len() {
                source.push('\n');
            }
        }

        let source_sha256 = Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into());
        DraftQuestSemanticGeneratedSource {
            target: self.input.target.clone(),
            quest_id: self.input.quest_id,
            generator_id: DRAFT_QUEST_GENERATOR_ID,
            generator_version: DRAFT_QUEST_SEMANTIC_GENERATOR_VERSION,
            giver: self.input.giver.clone(),
            parent_quest: self.input.parent_quest.clone(),
            collision_catalog: self.input.collision_catalog.anchor(),
            technical_names: self.technical_names.clone(),
            source,
            source_sha256,
            input_fingerprint: self.input_fingerprint,
            status: DraftQuestCapabilityStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        }
    }

    fn render_node_class_tail(&self, source: &mut String, node: QuestTransitionNodeV1) {
        let transitions = [
            QuestTransitionEdgeV1::Availability,
            QuestTransitionEdgeV1::Start,
            QuestTransitionEdgeV1::Success,
            QuestTransitionEdgeV1::Failure,
        ]
        .into_iter()
        .filter_map(|edge| self.transition(node, edge))
        .collect::<Vec<_>>();
        if transitions.iter().any(|transition| {
            transition.edge == QuestTransitionEdgeV1::Availability && !transition.external_allowed
        }) {
            // UG1RQuest availability is externally driven by default. Emitting only the false
            // override preserves byte-for-byte legacy-seed upgrades while making opt-out real.
            source.push_str("    default bExternalAvailabilityTrigger = false;\n");
        }
        if transitions.iter().any(|transition| {
            transition.edge == QuestTransitionEdgeV1::Start && transition.external_allowed
        }) {
            source.push_str("    default bExternalStartTrigger = true;\n");
        }
        if transitions.iter().any(|transition| {
            transition.edge == QuestTransitionEdgeV1::Success && transition.external_allowed
        }) {
            source.push_str("    default bExternalSuccessTrigger = true;\n");
        }
        if transitions.iter().any(|transition| {
            transition.edge == QuestTransitionEdgeV1::Failure && transition.external_allowed
        }) {
            source.push_str("    default bExternalFailTrigger = true;\n");
        }
        if transitions
            .iter()
            .any(|transition| transition.succeeds_parent)
        {
            source.push_str("    default bSucceedParent = true;\n");
        }
        for transition in &transitions {
            if let Some(predicate) = &transition.predicate {
                self.render_predicate_method(source, node, transition.edge, predicate);
            }
        }
        for transition in transitions {
            if !transition.effects.is_empty() {
                self.render_effect_handler(source, transition);
            }
        }
    }

    fn render_predicate_method(
        &self,
        source: &mut String,
        owner: QuestTransitionNodeV1,
        edge: QuestTransitionEdgeV1,
        predicate: &QuestTransitionPredicateV1,
    ) {
        let method = match edge {
            QuestTransitionEdgeV1::Availability => "ShouldBeAvailable_Implementation",
            QuestTransitionEdgeV1::Start => "ShouldStart_Implementation",
            QuestTransitionEdgeV1::Success => "ShouldSucceed_Implementation",
            QuestTransitionEdgeV1::Failure => "ShouldFail_Implementation",
        };
        source.push_str(&format!("\n    UFUNCTION()\n    bool {method}()\n    {{\n"));
        let referenced = predicate
            .any_of
            .iter()
            .flat_map(|group| group.all_of.iter().map(|atom| atom.node))
            .filter(|node| *node != owner)
            .collect::<BTreeSet<_>>();
        for node in &referenced {
            let names = self.names_for_node(*node);
            source.push_str(&format!(
                "        {class} {local} = {getter}();\n",
                class = names.0,
                local = transition_local_name(*node),
                getter = names.1,
            ));
        }
        if !referenced.is_empty() {
            source.push('\n');
        }
        source.push_str("        return ");
        source.push_str(&render_predicate_expression(predicate, owner));
        source.push_str(";\n    }\n");
    }

    fn render_effect_handler(&self, source: &mut String, transition: &QuestTransitionV1) {
        let method = match transition.edge {
            QuestTransitionEdgeV1::Start => "HandleQuestStarted_Implementation",
            QuestTransitionEdgeV1::Success => "HandleQuestSucceeded_Implementation",
            QuestTransitionEdgeV1::Failure => "HandleQuestFailed_Implementation",
            QuestTransitionEdgeV1::Availability => unreachable!("validated without effects"),
        };
        source.push_str(&format!("\n    UFUNCTION()\n    void {method}()\n    {{\n"));
        let targets = transition
            .effects
            .iter()
            .map(|effect| effect.target)
            .collect::<BTreeSet<_>>();
        for target in &targets {
            let names = self.names_for_node(*target);
            source.push_str(&format!(
                "        {class} {local} = {getter}();\n",
                class = names.0,
                local = transition_local_name(*target),
                getter = names.1,
            ));
        }
        if !targets.is_empty() {
            source.push('\n');
        }
        for effect in &transition.effects {
            let local = transition_local_name(effect.target);
            let (guard, call) = match effect.effect {
                QuestTransitionEffectKindV1::Start => {
                    (format!("!{local}.HasBeenStarted()"), "StartQuest")
                }
                QuestTransitionEffectKindV1::Succeed => {
                    (format!("{local}.IsRunning()"), "SucceedQuest")
                }
                QuestTransitionEffectKindV1::Fail => (format!("{local}.IsRunning()"), "FailQuest"),
            };
            source.push_str(&format!(
                "        if ({local} != nullptr && {guard})\n            {local}.{call}(nullptr);\n"
            ));
        }
        source.push_str("    }\n");
    }

    fn transition(
        &self,
        node: QuestTransitionNodeV1,
        edge: QuestTransitionEdgeV1,
    ) -> Option<&QuestTransitionV1> {
        self.transition_plan
            .transitions
            .iter()
            .find(|transition| transition.node == node && transition.edge == edge)
    }

    fn objective_names(&self, slot: u16) -> &DraftQuestSemanticObjectiveTechnicalNames {
        self.technical_names
            .objectives
            .iter()
            .find(|objective| objective.slot == slot)
            .expect("validated active objective slot")
    }

    fn names_for_node(&self, node: QuestTransitionNodeV1) -> (&str, &str) {
        match node {
            QuestTransitionNodeV1::Root => (
                &self.technical_names.base.root_class,
                &self.technical_names.base.root_getter,
            ),
            QuestTransitionNodeV1::Objective { slot } => {
                let objective = self.objective_names(slot);
                (&objective.objective_class, &objective.objective_getter)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestSemanticGeneratedSource {
    pub target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub generator_id: &'static str,
    pub generator_version: u32,
    pub giver: CatalogQualifiedQuestGiver,
    pub parent_quest: CatalogQualifiedParentQuest,
    pub collision_catalog: DraftQuestCatalogLayerAnchor,
    pub technical_names: DraftQuestSemanticTechnicalNames,
    pub source: String,
    pub source_sha256: Sha256Digest,
    pub input_fingerprint: Sha256Digest,
    pub status: DraftQuestCapabilityStatus,
}

fn render_quest_getter(source: &mut String, class: &str, getter: &str) {
    source.push_str(&format!(
        "{class} {getter}()\n{{\n    UQuestSubsystem Subsystem = UQuestSubsystem::Get();\n    if (Subsystem == nullptr)\n        return nullptr;\n\n    TSubclassOf<UQuest> QuestClass =\n        TSubclassOf<UQuest>({class}::StaticClass());\n    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);\n    if (Quest == nullptr)\n        return nullptr;\n\n    return Cast<{class}>(Quest);\n}}\n"
    ));
}

fn transition_local_name(node: QuestTransitionNodeV1) -> String {
    match node {
        QuestTransitionNodeV1::Root => "RootQuest".to_owned(),
        QuestTransitionNodeV1::Objective { slot } => format!("ObjectiveQuest{slot}"),
    }
}

fn render_predicate_expression(
    predicate: &QuestTransitionPredicateV1,
    owner: QuestTransitionNodeV1,
) -> String {
    predicate
        .any_of
        .iter()
        .map(|group| {
            let atoms = group
                .all_of
                .iter()
                .map(|atom| render_condition_atom(atom, owner))
                .collect::<Vec<_>>()
                .join(" && ");
            format!("({atoms})")
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

fn render_condition_atom(
    atom: &QuestTransitionConditionAtomV1,
    owner: QuestTransitionNodeV1,
) -> String {
    let receiver = if atom.node == owner {
        "this".to_owned()
    } else {
        transition_local_name(atom.node)
    };
    let state = match atom.test {
        QuestTransitionStateTestV1::Available => format!("{receiver}.IsAvailable()"),
        QuestTransitionStateTestV1::Running => format!("{receiver}.IsRunning()"),
        QuestTransitionStateTestV1::Started => format!("{receiver}.HasBeenStarted()"),
        QuestTransitionStateTestV1::Succeeded => format!("{receiver}.HasSucceeded()"),
        QuestTransitionStateTestV1::Failed => format!("{receiver}.HasFailed()"),
        QuestTransitionStateTestV1::Completed => {
            format!("({receiver}.HasSucceeded() || {receiver}.HasFailed())")
        }
    };
    let state = if atom.negated {
        format!("!({state})")
    } else {
        state
    };
    if atom.node == owner {
        state
    } else {
        format!("{receiver} != nullptr && {state}")
    }
}

fn validate_semantic_objective_names(
    objectives: &[DraftQuestSemanticObjectiveTechnicalNames],
) -> Result<(), DraftQuestSkeletonError> {
    for objective in objectives {
        for value in [&objective.objective_class, &objective.objective_getter] {
            if value.len() > MAX_IDENTIFIER_BYTES {
                return Err(DraftQuestSkeletonError::ValueTooLong {
                    field: DraftQuestField::TechnicalId,
                    actual: value.len(),
                    max: MAX_IDENTIFIER_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn check_semantic_symbol_collisions(
    base: &DraftQuestTechnicalNames,
    objectives: &[DraftQuestSemanticObjectiveTechnicalNames],
) -> Result<(), DraftQuestSkeletonError> {
    let mut seen = BTreeMap::<String, String>::new();
    for symbol in [&base.root_class, &base.text_helper, &base.root_getter]
        .into_iter()
        .chain(
            objectives
                .iter()
                .flat_map(|objective| [&objective.objective_class, &objective.objective_getter]),
        )
    {
        let folded = symbol.to_ascii_lowercase();
        if let Some(first) = seen.insert(folded, symbol.clone()) {
            return Err(DraftQuestSkeletonError::GeneratedSymbolCollision {
                first,
                second: symbol.clone(),
            });
        }
    }
    Ok(())
}

fn fingerprint_semantic_quest_input(
    input: &DraftQuestSkeletonInput,
    additional_titles: &[String],
    plan: &QuestTransitionPlanV1,
) -> Sha256Digest {
    let mut hasher = Sha256::new();
    fingerprint_bytes(
        &mut hasher,
        "schema",
        b"gore-authoring.draft-quest-skeleton-v3.input-fingerprint",
    );
    fingerprint_bytes(
        &mut hasher,
        "frozen-v1.input-fingerprint",
        fingerprint_input(input).as_bytes(),
    );
    for (index, title) in additional_titles.iter().enumerate() {
        fingerprint_string(
            &mut hasher,
            &format!("objective.{}.title", index + 2),
            title,
        );
    }
    let plan_bytes = serde_json::to_vec(plan).expect("closed transition plan is serializable");
    fingerprint_bytes(&mut hasher, "transition-plan-v1", &plan_bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestMultiObjectiveGeneratedSource {
    pub target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub generator_id: &'static str,
    pub generator_version: u32,
    pub giver: CatalogQualifiedQuestGiver,
    pub parent_quest: CatalogQualifiedParentQuest,
    pub collision_catalog: DraftQuestCatalogLayerAnchor,
    pub technical_names: DraftQuestMultiObjectiveTechnicalNames,
    pub fixed_shape: DraftQuestFixedShape,
    pub source: String,
    pub source_sha256: Sha256Digest,
    pub input_fingerprint: Sha256Digest,
    pub status: DraftQuestCapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestGeneratedSource {
    pub target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub generator_id: &'static str,
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

fn fingerprint_multi_objective_input(
    input: &DraftQuestSkeletonInput,
    additional_titles: &[String],
    names: &DraftQuestMultiObjectiveTechnicalNames,
) -> Sha256Digest {
    let mut hasher = Sha256::new();
    fingerprint_bytes(
        &mut hasher,
        "schema",
        b"gore-authoring.draft-quest-skeleton-v2.input-fingerprint",
    );
    fingerprint_u64(
        &mut hasher,
        "generator.version",
        u64::from(DRAFT_QUEST_MULTI_OBJECTIVE_GENERATOR_VERSION),
    );
    fingerprint_bytes(
        &mut hasher,
        "frozen-v1.input-fingerprint",
        fingerprint_input(input).as_bytes(),
    );
    fingerprint_u64(
        &mut hasher,
        "objective.count",
        (1 + additional_titles.len()) as u64,
    );
    fingerprint_string(&mut hasher, "objective.title", &input.objective_title);
    for (offset, (title, technical)) in additional_titles
        .iter()
        .zip(&names.additional_objectives)
        .enumerate()
    {
        let ordinal = offset + 2;
        fingerprint_string(&mut hasher, &format!("objective.{ordinal}.title"), title);
        fingerprint_string(
            &mut hasher,
            &format!("objective.{ordinal}.class"),
            &technical.objective_class,
        );
        fingerprint_string(
            &mut hasher,
            &format!("objective.{ordinal}.getter"),
            &technical.objective_getter,
        );
    }
    fingerprint_bytes(&mut hasher, "shape.last-objective-succeeds-parent", &[1]);
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

fn validate_multi_objective_name_lengths(
    names: &[DraftQuestAdditionalObjectiveTechnicalNames],
) -> Result<(), DraftQuestSkeletonError> {
    for names in names {
        for value in [&names.objective_class, &names.objective_getter] {
            if value.len() > MAX_IDENTIFIER_BYTES {
                return Err(DraftQuestSkeletonError::ValueTooLong {
                    field: DraftQuestField::TechnicalId,
                    actual: value.len(),
                    max: MAX_IDENTIFIER_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn check_multi_objective_symbol_collisions(
    base: &DraftQuestTechnicalNames,
    additional: &[DraftQuestAdditionalObjectiveTechnicalNames],
) -> Result<(), DraftQuestSkeletonError> {
    let mut seen = std::collections::BTreeMap::<String, String>::new();
    for symbol in [
        &base.root_class,
        &base.objective_class,
        &base.text_helper,
        &base.root_getter,
        &base.objective_getter,
    ]
    .into_iter()
    .chain(
        additional
            .iter()
            .flat_map(|names| [&names.objective_class, &names.objective_getter]),
    ) {
        let key = symbol.to_ascii_lowercase();
        if let Some(first) = seen.insert(key, symbol.clone()) {
            return Err(DraftQuestSkeletonError::GeneratedSymbolCollision {
                first,
                second: symbol.clone(),
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
