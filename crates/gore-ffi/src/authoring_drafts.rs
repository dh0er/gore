//! Pure preview bridges for the bounded offline NPC and quest source generators.
//!
//! Both commands accept their generator input as an untouched nested JSON string. This keeps
//! duplicate-key rejection in serde instead of silently normalizing hostile input through
//! `serde_json::Value`. They only validate and generate in memory: no filesystem, compiler,
//! package, game, deployment, runtime-qualification, or save operation is reachable here.

use gore_authoring::{
    CatalogQualifiedParentQuest, CatalogQualifiedQuestGiver, ContentSeal,
    DraftQuestCatalogLayerAnchor, DraftQuestCollisionCatalog, DraftQuestCollisionKind,
    DraftQuestField, DraftQuestGeneratedSource, DraftQuestSkeletonError, DraftQuestSkeletonInput,
    DraftQuestSkeletonV1, EntityId, GameGenerationAnchor, LogicalNpcCloneDraft,
    LogicalNpcCloneDraftError, LogicalNpcCloneField, LogicalNpcCloneSource,
};
use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};

use crate::err;

const MAX_NPC_INPUT_JSON_BYTES: usize = 16 * 1024;
const MAX_QUEST_INPUT_JSON_BYTES: usize = 20 * 1024 * 1024;
const MAX_DRAFT_RESPONSE_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const TRUNCATED_SUFFIX: &str = "...";
// Keep these fail-before-allocation wire limits synchronized with the closed collision-catalog
// limits in `gore-authoring/src/quest.rs`. The authoring constructor remains the semantic
// authority; this duplicate guard prevents adversarial JSON from allocating millions of strings
// before that constructor gets control.
const MAX_QUEST_COLLISION_ENTRIES: usize = 100_000;
const MAX_QUEST_COLLISION_ENTRY_BYTES: usize = 512;
const MAX_QUEST_COLLISION_TOTAL_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogicalNpcCloneInput {
    module_namespace: String,
    unique_name: String,
    parent_character_definition: String,
    parent_ai_agent_config: String,
    parent_spawn_definition: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftQuestInput {
    target: GameGenerationAnchor,
    quest_id: EntityId,
    module_namespace: String,
    technical_id: String,
    text_helper: String,
    parent_quest: ParentQuestInput,
    giver: QuestGiverInput,
    title: String,
    description: String,
    objective_title: String,
    collision_catalog: CollisionCatalogInput,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParentQuestInput {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
    canonical_selector: String,
    runtime_class: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestGiverInput {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
    canonical_selector: String,
    runtime_unique_name: String,
}

#[derive(Debug)]
struct CollisionCatalogInput {
    generation: GameGenerationAnchor,
    source_seal: ContentSeal,
    catalog_layer: String,
    modules: Vec<String>,
    relative_paths: Vec<String>,
    symbols: Vec<String>,
    wire_violation: Option<CollisionWireViolation>,
}

#[derive(Debug, Clone, Copy)]
enum CollisionWireViolation {
    TooManyEntries { actual: usize },
    TooLarge { actual: usize },
    EntryTooLong { kind: DraftQuestCollisionKind },
}

impl<'de> Deserialize<'de> for CollisionCatalogInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(CollisionCatalogVisitor)
    }
}

struct CollisionCatalogVisitor;

impl<'de> Visitor<'de> for CollisionCatalogVisitor {
    type Value = CollisionCatalogInput;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded collision catalog object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut generation = None;
        let mut source_seal = None;
        let mut catalog_layer = None;
        let mut modules = None;
        let mut relative_paths = None;
        let mut symbols = None;
        let mut entry_count = 0usize;
        let mut entry_bytes = 0usize;
        let mut oversized_entry_kind = None;

        while let Some(field) = map.next_key::<CollisionCatalogField>()? {
            match field {
                CollisionCatalogField::Generation => {
                    set_once(&mut generation, map.next_value()?, "generation")?
                }
                CollisionCatalogField::SourceSeal => {
                    set_once(&mut source_seal, map.next_value()?, "source_seal")?
                }
                CollisionCatalogField::CatalogLayer => {
                    set_once(&mut catalog_layer, map.next_value()?, "catalog_layer")?
                }
                CollisionCatalogField::Modules => {
                    if modules.is_some() {
                        return Err(A::Error::duplicate_field("modules"));
                    }
                    modules = Some(map.next_value_seed(CollisionEntriesSeed {
                        entry_count: &mut entry_count,
                        entry_bytes: &mut entry_bytes,
                        oversized_entry_kind: &mut oversized_entry_kind,
                        kind: DraftQuestCollisionKind::Module,
                    })?);
                }
                CollisionCatalogField::RelativePaths => {
                    if relative_paths.is_some() {
                        return Err(A::Error::duplicate_field("relative_paths"));
                    }
                    relative_paths = Some(map.next_value_seed(CollisionEntriesSeed {
                        entry_count: &mut entry_count,
                        entry_bytes: &mut entry_bytes,
                        oversized_entry_kind: &mut oversized_entry_kind,
                        kind: DraftQuestCollisionKind::RelativePath,
                    })?);
                }
                CollisionCatalogField::Symbols => {
                    if symbols.is_some() {
                        return Err(A::Error::duplicate_field("symbols"));
                    }
                    symbols = Some(map.next_value_seed(CollisionEntriesSeed {
                        entry_count: &mut entry_count,
                        entry_bytes: &mut entry_bytes,
                        oversized_entry_kind: &mut oversized_entry_kind,
                        kind: DraftQuestCollisionKind::Symbol,
                    })?);
                }
            }
        }

        let wire_violation = if entry_count > MAX_QUEST_COLLISION_ENTRIES {
            Some(CollisionWireViolation::TooManyEntries {
                actual: entry_count,
            })
        } else if entry_bytes > MAX_QUEST_COLLISION_TOTAL_BYTES {
            Some(CollisionWireViolation::TooLarge {
                actual: entry_bytes,
            })
        } else {
            oversized_entry_kind.map(|kind| CollisionWireViolation::EntryTooLong { kind })
        };

        Ok(CollisionCatalogInput {
            generation: generation.ok_or_else(|| A::Error::missing_field("generation"))?,
            source_seal: source_seal.ok_or_else(|| A::Error::missing_field("source_seal"))?,
            catalog_layer: catalog_layer.ok_or_else(|| A::Error::missing_field("catalog_layer"))?,
            modules: modules.ok_or_else(|| A::Error::missing_field("modules"))?,
            relative_paths: relative_paths
                .ok_or_else(|| A::Error::missing_field("relative_paths"))?,
            symbols: symbols.ok_or_else(|| A::Error::missing_field("symbols"))?,
            wire_violation,
        })
    }
}

fn set_once<E, T>(slot: &mut Option<T>, value: T, field: &'static str) -> Result<(), E>
where
    E: serde::de::Error,
{
    if slot.is_some() {
        return Err(E::duplicate_field(field));
    }
    *slot = Some(value);
    Ok(())
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum CollisionCatalogField {
    Generation,
    SourceSeal,
    CatalogLayer,
    Modules,
    RelativePaths,
    Symbols,
}

struct CollisionEntriesSeed<'a> {
    entry_count: &'a mut usize,
    entry_bytes: &'a mut usize,
    oversized_entry_kind: &'a mut Option<DraftQuestCollisionKind>,
    kind: DraftQuestCollisionKind,
}

impl<'de> DeserializeSeed<'de> for CollisionEntriesSeed<'_> {
    type Value = Vec<String>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(CollisionEntriesVisitor {
            entry_count: self.entry_count,
            entry_bytes: self.entry_bytes,
            oversized_entry_kind: self.oversized_entry_kind,
            kind: self.kind,
        })
    }
}

struct CollisionEntriesVisitor<'a> {
    entry_count: &'a mut usize,
    entry_bytes: &'a mut usize,
    oversized_entry_kind: &'a mut Option<DraftQuestCollisionKind>,
    kind: DraftQuestCollisionKind,
}

impl<'de> Visitor<'de> for CollisionEntriesVisitor<'_> {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded list of collision strings")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let remaining = MAX_QUEST_COLLISION_ENTRIES.saturating_sub(*self.entry_count);
        let mut entries = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(remaining));
        while let Some(entry) = sequence.next_element_seed(CollisionEntrySeed)? {
            *self.entry_count = self.entry_count.saturating_add(1);
            let next_bytes = self.entry_bytes.saturating_add(entry.byte_len);
            *self.entry_bytes = next_bytes;
            if let Some(value) = entry.value {
                if *self.entry_count <= MAX_QUEST_COLLISION_ENTRIES
                    && *self.entry_bytes <= MAX_QUEST_COLLISION_TOTAL_BYTES
                {
                    entries.push(value);
                }
            } else {
                self.oversized_entry_kind.get_or_insert(self.kind);
            }
        }
        Ok(entries)
    }
}

struct CollisionEntrySeed;

impl<'de> DeserializeSeed<'de> for CollisionEntrySeed {
    type Value = CollisionEntry;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(CollisionEntryVisitor)
    }
}

struct CollisionEntryVisitor;

struct CollisionEntry {
    value: Option<String>,
    byte_len: usize,
}

impl Visitor<'_> for CollisionEntryVisitor {
    type Value = CollisionEntry;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a collision string of at most {MAX_QUEST_COLLISION_ENTRY_BYTES} UTF-8 bytes"
        )
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(collision_entry(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(collision_entry(value))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let byte_len = value.len();
        Ok(CollisionEntry {
            value: (byte_len <= MAX_QUEST_COLLISION_ENTRY_BYTES).then_some(value),
            byte_len,
        })
    }
}

fn collision_entry(value: &str) -> CollisionEntry {
    let byte_len = value.len();
    CollisionEntry {
        value: (byte_len <= MAX_QUEST_COLLISION_ENTRY_BYTES).then(|| value.to_owned()),
        byte_len,
    }
}

pub(crate) fn logical_npc_clone(payload: Value) -> Value {
    generate_with_limits(
        payload,
        MAX_NPC_INPUT_JSON_BYTES,
        "AUTHORING_NPC_DRAFT_INPUT_LIMIT",
        "AUTHORING_NPC_DRAFT_INPUT_INVALID",
        |raw| {
            let input: LogicalNpcCloneInput = serde_json::from_str(raw)?;
            Ok(
                match LogicalNpcCloneDraft::new(
                    input.module_namespace,
                    input.unique_name,
                    input.parent_character_definition,
                    input.parent_ai_agent_config,
                    input.parent_spawn_definition,
                ) {
                    Ok(draft) => success(npc_source_to_wire(draft.generate())),
                    Err(error) => invalid(npc_diagnostic(error)),
                },
            )
        },
    )
}

pub(crate) fn draft_quest_skeleton(payload: Value) -> Value {
    generate_with_limits(
        payload,
        MAX_QUEST_INPUT_JSON_BYTES,
        "AUTHORING_QUEST_DRAFT_INPUT_LIMIT",
        "AUTHORING_QUEST_DRAFT_INPUT_INVALID",
        |raw| {
            let input: DraftQuestInput = serde_json::from_str(raw)?;
            if let Some(violation) = input.collision_catalog.wire_violation {
                return Ok(invalid(collision_wire_diagnostic(violation)));
            }
            Ok(match build_quest(input) {
                Ok(draft) => success(quest_source_to_wire(draft.generate())),
                Err(error) => invalid(quest_diagnostic(error)),
            })
        },
    )
}

fn generate_with_limits<F>(
    payload: Value,
    input_limit: usize,
    limit_code: &str,
    invalid_code: &str,
    generate: F,
) -> Value
where
    F: FnOnce(&str) -> Result<Value, serde_json::Error>,
{
    let raw = match exact_input_json(payload) {
        Ok(raw) => raw,
        Err(response) => return response,
    };
    if raw.len() > input_limit {
        return err(
            limit_code,
            format!("input_json exceeds the {input_limit}-byte limit"),
        );
    }
    let response = match generate(&raw) {
        Ok(response) => response,
        Err(error) => err(invalid_code, bounded_message(error.to_string())),
    };
    bounded_response(response)
}

fn exact_input_json(payload: Value) -> Result<String, Value> {
    let Value::Object(mut object) = payload else {
        return Err(err(
            "AUTHORING_DRAFT_PAYLOAD_INVALID",
            "payload must be exactly an object containing input_json",
        ));
    };
    if object.len() != 1 || !object.contains_key("input_json") {
        return Err(err(
            "AUTHORING_DRAFT_PAYLOAD_INVALID",
            "payload must contain exactly the input_json field",
        ));
    }
    match object.remove("input_json") {
        Some(Value::String(raw)) => Ok(raw),
        _ => Err(err(
            "AUTHORING_DRAFT_PAYLOAD_INVALID",
            "input_json must be a string",
        )),
    }
}

fn bounded_response(response: Value) -> Value {
    match serde_json::to_vec(&response) {
        Ok(encoded) if encoded.len() <= MAX_DRAFT_RESPONSE_JSON_BYTES => response,
        Ok(_) => err(
            "AUTHORING_DRAFT_RESPONSE_LIMIT",
            format!(
                "authoring draft response exceeds the {MAX_DRAFT_RESPONSE_JSON_BYTES}-byte limit"
            ),
        ),
        Err(_) => err(
            "AUTHORING_DRAFT_RESPONSE_SERIALIZE",
            "authoring draft response serialization failed",
        ),
    }
}

fn bounded_message(mut message: String) -> String {
    if message.len() <= MAX_ERROR_MESSAGE_BYTES {
        return message;
    }
    let mut end = MAX_ERROR_MESSAGE_BYTES - TRUNCATED_SUFFIX.len();
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message.truncate(end);
    message.push_str(TRUNCATED_SUFFIX);
    message
}

fn success(generated: Value) -> Value {
    json!({
        "ok": true,
        "valid": true,
        "generated": generated,
        "diagnostics": [],
    })
}

fn invalid(diagnostic: Value) -> Value {
    json!({
        "ok": true,
        "valid": false,
        "generated": null,
        "diagnostics": [diagnostic],
    })
}

fn diagnostic(code: &'static str, field: String, message: String) -> Value {
    json!({
        "code": code,
        "field": field,
        "message": bounded_message(message),
    })
}

fn npc_diagnostic(error: LogicalNpcCloneDraftError) -> Value {
    let (code, field) = match &error {
        LogicalNpcCloneDraftError::EmptyValue { field } => ("NPC_EMPTY_VALUE", npc_field(*field)),
        LogicalNpcCloneDraftError::ValueTooLong { field, .. } => {
            ("NPC_VALUE_TOO_LONG", npc_field(*field))
        }
        LogicalNpcCloneDraftError::TooManyModuleSegments { .. } => (
            "NPC_TOO_MANY_MODULE_SEGMENTS",
            "module_namespace".to_owned(),
        ),
        LogicalNpcCloneDraftError::InvalidIdentifierStart { field, .. } => {
            ("NPC_INVALID_IDENTIFIER_START", npc_field(*field))
        }
        LogicalNpcCloneDraftError::InvalidIdentifierCharacter { field, .. } => {
            ("NPC_INVALID_IDENTIFIER_CHARACTER", npc_field(*field))
        }
        LogicalNpcCloneDraftError::ReservedIdentifier { field } => {
            ("NPC_RESERVED_IDENTIFIER", npc_field(*field))
        }
        LogicalNpcCloneDraftError::ReservedModuleSegment { index } => (
            "NPC_RESERVED_MODULE_SEGMENT",
            format!("module_namespace.segments[{index}]"),
        ),
        LogicalNpcCloneDraftError::UnexpectedParentClassPrefix { field, .. } => {
            ("NPC_UNEXPECTED_PARENT_CLASS_PREFIX", npc_field(*field))
        }
        LogicalNpcCloneDraftError::ClassNameCollision { field, .. } => {
            ("NPC_CLASS_NAME_COLLISION", npc_field(*field))
        }
    };
    diagnostic(code, field, error.to_string())
}

fn npc_field(field: LogicalNpcCloneField) -> String {
    match field {
        LogicalNpcCloneField::ModuleNamespace => "module_namespace".to_owned(),
        LogicalNpcCloneField::ModuleSegment { index } => {
            format!("module_namespace.segments[{index}]")
        }
        LogicalNpcCloneField::UniqueName => "unique_name".to_owned(),
        LogicalNpcCloneField::ParentCharacterDefinition => "parent_character_definition".to_owned(),
        LogicalNpcCloneField::ParentAiAgentConfig => "parent_ai_agent_config".to_owned(),
        LogicalNpcCloneField::ParentSpawnDefinition => "parent_spawn_definition".to_owned(),
    }
}

fn quest_diagnostic(error: DraftQuestSkeletonError) -> Value {
    let (code, field) = match &error {
        DraftQuestSkeletonError::InvalidSeal { field } => {
            ("QUEST_INVALID_SEAL", quest_field(*field))
        }
        DraftQuestSkeletonError::GenerationMismatch { field } => {
            ("QUEST_GENERATION_MISMATCH", quest_field(*field))
        }
        DraftQuestSkeletonError::ZeroEntityId => ("QUEST_ZERO_ENTITY_ID", "quest_id".to_owned()),
        DraftQuestSkeletonError::EmptyValue { field } => ("QUEST_EMPTY_VALUE", quest_field(*field)),
        DraftQuestSkeletonError::ValueTooLong { field, .. } => {
            ("QUEST_VALUE_TOO_LONG", quest_field(*field))
        }
        DraftQuestSkeletonError::InvalidCharacter { field, .. } => {
            ("QUEST_INVALID_CHARACTER", quest_field(*field))
        }
        DraftQuestSkeletonError::ReservedIdentifier { field } => {
            ("QUEST_RESERVED_IDENTIFIER", quest_field(*field))
        }
        DraftQuestSkeletonError::NonCanonicalIdentifier { field, .. } => {
            ("QUEST_NON_CANONICAL_IDENTIFIER", quest_field(*field))
        }
        DraftQuestSkeletonError::TooManyModuleSegments { .. } => (
            "QUEST_TOO_MANY_MODULE_SEGMENTS",
            "module_namespace".to_owned(),
        ),
        DraftQuestSkeletonError::ReservedModuleSegment { index } => (
            "QUEST_RESERVED_MODULE_SEGMENT",
            format!("module_namespace.segments[{index}]"),
        ),
        DraftQuestSkeletonError::InvalidParentQuestClass => (
            "QUEST_INVALID_PARENT_CLASS",
            "parent_quest.runtime_class".to_owned(),
        ),
        DraftQuestSkeletonError::ParentClassCollision { .. } => (
            "QUEST_PARENT_CLASS_COLLISION",
            "parent_quest.runtime_class".to_owned(),
        ),
        DraftQuestSkeletonError::NonCanonicalText { field } => {
            ("QUEST_NON_CANONICAL_TEXT", quest_field(*field))
        }
        DraftQuestSkeletonError::TooManyCollisionEntries { .. } => (
            "QUEST_TOO_MANY_COLLISION_ENTRIES",
            "collision_catalog".to_owned(),
        ),
        DraftQuestSkeletonError::CollisionCatalogTooLarge { .. } => (
            "QUEST_COLLISION_CATALOG_TOO_LARGE",
            "collision_catalog".to_owned(),
        ),
        DraftQuestSkeletonError::UnsafeCollisionEntry { kind, .. } => {
            ("QUEST_UNSAFE_COLLISION_ENTRY", collision_field(*kind))
        }
        DraftQuestSkeletonError::DuplicateCollisionEntry { kind, .. } => {
            ("QUEST_DUPLICATE_COLLISION_ENTRY", collision_field(*kind))
        }
        DraftQuestSkeletonError::GeneratedNameCollision { kind, .. } => {
            ("QUEST_GENERATED_NAME_COLLISION", collision_field(*kind))
        }
        DraftQuestSkeletonError::GeneratedSymbolCollision { .. } => (
            "QUEST_GENERATED_SYMBOL_COLLISION",
            "generated_symbols".to_owned(),
        ),
    };
    diagnostic(code, field, error.to_string())
}

fn quest_field(field: DraftQuestField) -> String {
    match field {
        DraftQuestField::GameGeneration => "target".to_owned(),
        DraftQuestField::GiverGeneration => "giver.generation".to_owned(),
        DraftQuestField::GiverSourceSeal => "giver.source_seal".to_owned(),
        DraftQuestField::GiverCatalogLayer => "giver.catalog_layer".to_owned(),
        DraftQuestField::GiverSelector => "giver.canonical_selector".to_owned(),
        DraftQuestField::GiverRuntimeUniqueName => "giver.runtime_unique_name".to_owned(),
        DraftQuestField::ParentGeneration => "parent_quest.generation".to_owned(),
        DraftQuestField::ParentSourceSeal => "parent_quest.source_seal".to_owned(),
        DraftQuestField::ParentCatalogLayer => "parent_quest.catalog_layer".to_owned(),
        DraftQuestField::ParentSelector => "parent_quest.canonical_selector".to_owned(),
        DraftQuestField::ParentRuntimeClass => "parent_quest.runtime_class".to_owned(),
        DraftQuestField::CollisionGeneration => "collision_catalog.generation".to_owned(),
        DraftQuestField::CollisionSourceSeal => "collision_catalog.source_seal".to_owned(),
        DraftQuestField::CollisionCatalogLayer => "collision_catalog.catalog_layer".to_owned(),
        DraftQuestField::ModuleNamespace => "module_namespace".to_owned(),
        DraftQuestField::ModuleSegment { index } => {
            format!("module_namespace.segments[{index}]")
        }
        DraftQuestField::TechnicalId => "technical_id".to_owned(),
        DraftQuestField::TextHelper => "text_helper".to_owned(),
        DraftQuestField::Title => "title".to_owned(),
        DraftQuestField::Description => "description".to_owned(),
        DraftQuestField::ObjectiveTitle => "objective_title".to_owned(),
    }
}

fn collision_field(kind: DraftQuestCollisionKind) -> String {
    match kind {
        DraftQuestCollisionKind::Module => "collision_catalog.modules".to_owned(),
        DraftQuestCollisionKind::RelativePath => "collision_catalog.relative_paths".to_owned(),
        DraftQuestCollisionKind::Symbol => "collision_catalog.symbols".to_owned(),
    }
}

fn collision_wire_diagnostic(violation: CollisionWireViolation) -> Value {
    match violation {
        CollisionWireViolation::TooManyEntries { actual } => diagnostic(
            "QUEST_TOO_MANY_COLLISION_ENTRIES",
            "collision_catalog".to_owned(),
            format!(
                "collision catalog contains {actual} entries; maximum is {MAX_QUEST_COLLISION_ENTRIES}"
            ),
        ),
        CollisionWireViolation::TooLarge { actual } => diagnostic(
            "QUEST_COLLISION_CATALOG_TOO_LARGE",
            "collision_catalog".to_owned(),
            format!(
                "collision catalog contains {actual} bytes; maximum is {MAX_QUEST_COLLISION_TOTAL_BYTES}"
            ),
        ),
        CollisionWireViolation::EntryTooLong { kind } => diagnostic(
            "QUEST_UNSAFE_COLLISION_ENTRY",
            collision_field(kind),
            format!(
                "collision {kind} entry exceeds the {MAX_QUEST_COLLISION_ENTRY_BYTES}-byte limit"
            ),
        ),
    }
}

fn build_quest(input: DraftQuestInput) -> Result<DraftQuestSkeletonV1, DraftQuestSkeletonError> {
    let parent_quest = CatalogQualifiedParentQuest::new(
        input.parent_quest.generation,
        input.parent_quest.source_seal,
        input.parent_quest.catalog_layer,
        input.parent_quest.canonical_selector,
        input.parent_quest.runtime_class,
    )?;
    let giver = CatalogQualifiedQuestGiver::new(
        input.giver.generation,
        input.giver.source_seal,
        input.giver.catalog_layer,
        input.giver.canonical_selector,
        input.giver.runtime_unique_name,
    )?;
    let collision_catalog = DraftQuestCollisionCatalog::new(
        input.collision_catalog.generation,
        input.collision_catalog.source_seal,
        input.collision_catalog.catalog_layer,
        input.collision_catalog.modules,
        input.collision_catalog.relative_paths,
        input.collision_catalog.symbols,
    )?;
    DraftQuestSkeletonV1::new(DraftQuestSkeletonInput {
        target: input.target,
        quest_id: input.quest_id,
        module_namespace: input.module_namespace,
        technical_id: input.technical_id,
        text_helper: input.text_helper,
        parent_quest,
        giver,
        title: input.title,
        description: input.description,
        objective_title: input.objective_title,
        collision_catalog,
    })
}

fn npc_source_to_wire(generated: LogicalNpcCloneSource) -> Value {
    json!({
        "generator_id": generated.generator_id,
        "generator_version": generated.generator_version,
        "module_namespace": generated.module_namespace,
        "module_relative_path": generated.module_relative_path,
        "unique_name": generated.unique_name,
        "classes": {
            "character_definition": generated.classes.character_definition,
            "ai_agent_config": generated.classes.ai_agent_config,
            "spawn_definition": generated.classes.spawn_definition,
        },
        "source": generated.source,
        "source_sha256": generated.source_sha256.to_string(),
        "input_fingerprint": generated.input_fingerprint.to_string(),
        "status": {
            "authoring": "offline_draft",
            "runtime": "runtime_unqualified",
        },
    })
}

fn quest_source_to_wire(generated: DraftQuestGeneratedSource) -> Value {
    json!({
        "target": generated.target,
        "quest_id": generated.quest_id.to_string(),
        "generator_id": generated.generator_id,
        "generator_version": generated.generator_version,
        "giver": giver_to_wire(&generated.giver),
        "parent_quest": parent_to_wire(&generated.parent_quest),
        "collision_catalog": catalog_anchor_to_wire(&generated.collision_catalog),
        "technical_names": {
            "module_namespace": generated.technical_names.module_namespace,
            "module_relative_path": generated.technical_names.module_relative_path,
            "root_class": generated.technical_names.root_class,
            "objective_class": generated.technical_names.objective_class,
            "text_helper": generated.technical_names.text_helper,
            "root_getter": generated.technical_names.root_getter,
            "objective_getter": generated.technical_names.objective_getter,
        },
        "fixed_shape": {
            "quest_base_class": generated.fixed_shape.quest_base_class,
            "root_kind": generated.fixed_shape.root_kind,
            "objective_kind": generated.fixed_shape.objective_kind,
            "root_external_start": generated.fixed_shape.root_external_start,
            "objective_external_start": generated.fixed_shape.objective_external_start,
            "objective_external_success": generated.fixed_shape.objective_external_success,
            "objective_succeeds_parent": generated.fixed_shape.objective_succeeds_parent,
        },
        "source": generated.source,
        "source_sha256": generated.source_sha256.to_string(),
        "input_fingerprint": generated.input_fingerprint.to_string(),
        "status": {
            "authoring": "offline_draft",
            "discovery": "runtime_unqualified",
            "transitions": "transitions_runtime_unqualified",
        },
    })
}

fn giver_to_wire(giver: &CatalogQualifiedQuestGiver) -> Value {
    json!({
        "generation": giver.generation(),
        "source_seal": giver.source_seal(),
        "catalog_layer": giver.catalog_layer(),
        "canonical_selector": giver.canonical_selector(),
        "runtime_unique_name": giver.runtime_unique_name(),
    })
}

fn parent_to_wire(parent: &CatalogQualifiedParentQuest) -> Value {
    json!({
        "generation": parent.generation(),
        "source_seal": parent.source_seal(),
        "catalog_layer": parent.catalog_layer(),
        "canonical_selector": parent.canonical_selector(),
        "runtime_class": parent.runtime_class(),
    })
}

fn catalog_anchor_to_wire(anchor: &DraftQuestCatalogLayerAnchor) -> Value {
    json!({
        "generation": anchor.generation(),
        "source_seal": anchor.source_seal(),
        "catalog_layer": anchor.catalog_layer(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute_json;

    fn digest(byte: &str) -> String {
        byte.repeat(32)
    }

    fn generation(byte: &str) -> Value {
        json!({"executable": {"byte_len": 1_000_000, "sha256": digest(byte)}})
    }

    fn seal(byte: &str, byte_len: u64) -> Value {
        json!({"byte_len": byte_len, "sha256": digest(byte)})
    }

    fn npc_input() -> Value {
        json!({
            "module_namespace": "GoreMods.Probe.NpcLogicalCloneV1",
            "unique_name": "GORE_LOGICAL_ASGHAN_CLONE_V1",
            "parent_character_definition": "UCharacterDefinition_Human_OM_GRD_Asghan_263",
            "parent_ai_agent_config": "UAIAgentConfig_Human_OM_GRD_Asghan_263",
            "parent_spawn_definition": "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
        })
    }

    fn quest_input() -> Value {
        json!({
            "target": generation("11"),
            "quest_id": "0123456789abcdef0123456789abcdef",
            "module_namespace": "GoreMods.Probe.AsghanMiniQuest",
            "technical_id": "GORE_PROBE_ASGHAN_MINI",
            "text_helper": "GoreProbeAsghanText",
            "parent_quest": {
                "generation": generation("11"),
                "source_seal": seal("44", 4096),
                "catalog_layer": "dependency.story-pack.quests",
                "canonical_selector": "CatalogQuest_00263",
                "runtime_class": "UQuest_SwampCamp_SCCHAPTER2",
            },
            "giver": {
                "generation": generation("11"),
                "source_seal": seal("22", 8192),
                "catalog_layer": "base-game.g1r.characters",
                "canonical_selector": "CatalogCharacter_00263",
                "runtime_unique_name": "OM_GRD_Asghan_263",
            },
            "title": "Gore probe at Asghan",
            "description": "Talk to Asghan once more to complete the probe quest.",
            "objective_title": "Talk to Asghan once more",
            "collision_catalog": {
                "generation": generation("11"),
                "source_seal": seal("33", 32768),
                "catalog_layer": "resolved-loadout.scripts.v1",
                "modules": [],
                "relative_paths": [],
                "symbols": [],
            },
        })
    }

    fn call(command: &str, raw: String) -> Value {
        let request = json!({
            "command": command,
            "payload": {"input_json": raw},
        });
        serde_json::from_str(&execute_json(&request.to_string())).unwrap()
    }

    #[test]
    fn npc_success_is_deterministic_complete_and_never_runtime_qualified() {
        let first = call(
            "authoring_logical_npc_clone_draft_v1_generate",
            npc_input().to_string(),
        );
        let second = call(
            "authoring_logical_npc_clone_draft_v1_generate",
            npc_input().to_string(),
        );
        assert_eq!(first, second);
        assert_eq!(first["ok"], true);
        assert_eq!(first["valid"], true);
        assert_eq!(first["diagnostics"], json!([]));
        assert_eq!(
            first["generated"]["generator_id"],
            "gore-authoring.logical-npc-clone-draft"
        );
        assert_eq!(first["generated"]["generator_version"], 1);
        assert_eq!(
            first["generated"]["module_relative_path"],
            "GoreMods/Probe/NpcLogicalCloneV1.as"
        );
        assert_eq!(
            first["generated"]["status"],
            json!({"authoring": "offline_draft", "runtime": "runtime_unqualified"})
        );
        assert_eq!(
            first["generated"]["source_sha256"],
            "c78f366a3701393b2657693b29a7673c38dbe59d1ac2ff6c4fb3c2a51163e5d0"
        );
        assert_eq!(
            first["generated"]["input_fingerprint"],
            "5f27b386533f60b0c7878f53c32ce5cfdccc93cf77f7a4b85d966d3b0125b7d2"
        );
    }

    #[test]
    fn quest_success_preserves_all_qualified_types_and_offline_status() {
        let response = call(
            "authoring_draft_quest_skeleton_v1_generate",
            quest_input().to_string(),
        );
        assert_eq!(response["ok"], true);
        assert_eq!(response["valid"], true);
        assert_eq!(response["diagnostics"], json!([]));
        let generated = &response["generated"];
        assert_eq!(
            generated["generator_id"],
            "gore-authoring.draft-quest-skeleton"
        );
        assert_eq!(generated["generator_version"], 1);
        assert_eq!(generated["quest_id"], "0123456789abcdef0123456789abcdef");
        assert_eq!(generated["target"], generation("11"));
        assert_eq!(generated["giver"]["source_seal"], seal("22", 8192));
        assert_eq!(
            generated["giver"]["canonical_selector"],
            "CatalogCharacter_00263"
        );
        assert_eq!(generated["parent_quest"]["source_seal"], seal("44", 4096));
        assert_eq!(
            generated["collision_catalog"]["source_seal"],
            seal("33", 32768)
        );
        assert_eq!(
            generated["source_sha256"],
            "eb38bf814685485977113cf67a679d4b4cb309a2dbcd229fae3a6d57f2a4ae82"
        );
        assert_eq!(
            generated["input_fingerprint"],
            "5987a4b5147fb76f34af3cf0f926f0c7de2450d4e370c1aee3d88bcf8121de93"
        );
        assert_eq!(
            generated["status"],
            json!({
                "authoring": "offline_draft",
                "discovery": "runtime_unqualified",
                "transitions": "transitions_runtime_unqualified",
            })
        );
        assert_eq!(generated["fixed_shape"]["root_external_start"], true);
        assert_eq!(generated["fixed_shape"]["objective_succeeds_parent"], true);
    }

    #[test]
    fn raw_duplicate_keys_unknown_fields_and_wrong_shapes_are_rejected() {
        let npc = npc_input().to_string().replacen(
            "\"unique_name\":\"GORE_LOGICAL_ASGHAN_CLONE_V1\"",
            "\"unique_name\":\"GORE_LOGICAL_ASGHAN_CLONE_V1\",\"unique_name\":\"OTHER\"",
            1,
        );
        for raw in [npc, "{\"unknown\":true}".to_owned(), "[]".to_owned()] {
            let response = call("authoring_logical_npc_clone_draft_v1_generate", raw);
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_NPC_DRAFT_INPUT_INVALID"
            );
        }

        let quest = quest_input().to_string().replacen(
            "\"technical_id\":\"GORE_PROBE_ASGHAN_MINI\"",
            "\"technical_id\":\"GORE_PROBE_ASGHAN_MINI\",\"technical_id\":\"OTHER\"",
            1,
        );
        let response = call("authoring_draft_quest_skeleton_v1_generate", quest);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_QUEST_DRAFT_INPUT_INVALID"
        );
    }

    #[test]
    fn generator_validation_is_a_typed_success_response_not_a_transport_error() {
        let mut npc = npc_input();
        npc["unique_name"] = json!("NPC\"; class Injected");
        let response = call(
            "authoring_logical_npc_clone_draft_v1_generate",
            npc.to_string(),
        );
        assert_eq!(response["ok"], true);
        assert_eq!(response["valid"], false);
        assert_eq!(response["generated"], Value::Null);
        assert_eq!(
            response["diagnostics"][0]["code"],
            "NPC_INVALID_IDENTIFIER_CHARACTER"
        );
        assert_eq!(response["diagnostics"][0]["field"], "unique_name");

        let mut quest = quest_input();
        quest["title"] = json!("Bad\"; StartQuest(nullptr); //");
        let response = call(
            "authoring_draft_quest_skeleton_v1_generate",
            quest.to_string(),
        );
        assert_eq!(response["ok"], true);
        assert_eq!(response["valid"], false);
        assert_eq!(response["generated"], Value::Null);
        assert_eq!(
            response["diagnostics"][0]["code"],
            "QUEST_INVALID_CHARACTER"
        );
        assert_eq!(response["diagnostics"][0]["field"], "title");
    }

    #[test]
    fn payload_and_input_limits_fail_closed_with_stable_codes() {
        for payload in [
            Value::Null,
            json!({}),
            json!({"input_json": "{}", "extra": 1}),
        ] {
            let response = logical_npc_clone(payload);
            assert_eq!(response["error"]["code"], "AUTHORING_DRAFT_PAYLOAD_INVALID");
        }
        let response = logical_npc_clone(json!({
            "input_json": "x".repeat(MAX_NPC_INPUT_JSON_BYTES + 1),
        }));
        assert_eq!(response["error"]["code"], "AUTHORING_NPC_DRAFT_INPUT_LIMIT");
    }

    #[test]
    fn collision_lists_stop_at_the_shared_count_bound_before_generator_allocation() {
        let raw = quest_input().to_string().replacen(
            "\"modules\":[]",
            &format!(
                "\"modules\":[{}]",
                vec!["\"\""; MAX_QUEST_COLLISION_ENTRIES + 1].join(",")
            ),
            1,
        );
        assert!(raw.len() < MAX_QUEST_INPUT_JSON_BYTES);

        let response = call("authoring_draft_quest_skeleton_v1_generate", raw);
        assert_eq!(response["ok"], true);
        assert_eq!(response["valid"], false);
        assert_eq!(
            response["diagnostics"][0]["code"],
            "QUEST_TOO_MANY_COLLISION_ENTRIES"
        );
    }

    #[test]
    fn collision_total_and_entry_limits_are_allocation_safe_typed_diagnostics() {
        let entry = format!("\"{}\"", "x".repeat(MAX_QUEST_COLLISION_ENTRY_BYTES));
        let count = MAX_QUEST_COLLISION_TOTAL_BYTES / MAX_QUEST_COLLISION_ENTRY_BYTES + 1;
        let raw = quest_input().to_string().replacen(
            "\"symbols\":[]",
            &format!("\"symbols\":[{}]", vec![entry; count].join(",")),
            1,
        );
        assert!(count < MAX_QUEST_COLLISION_ENTRIES);
        assert!(raw.len() < MAX_QUEST_INPUT_JSON_BYTES);
        let response = call("authoring_draft_quest_skeleton_v1_generate", raw);
        assert_eq!(response["ok"], true);
        assert_eq!(response["valid"], false);
        assert_eq!(
            response["diagnostics"][0]["code"],
            "QUEST_COLLISION_CATALOG_TOO_LARGE"
        );

        let raw = quest_input().to_string().replacen(
            "\"relative_paths\":[]",
            &format!(
                "\"relative_paths\":[\"{}\"]",
                "x".repeat(MAX_QUEST_COLLISION_ENTRY_BYTES + 1)
            ),
            1,
        );
        let response = call("authoring_draft_quest_skeleton_v1_generate", raw);
        assert_eq!(response["ok"], true);
        assert_eq!(response["valid"], false);
        assert_eq!(
            response["diagnostics"][0]["code"],
            "QUEST_UNSAFE_COLLISION_ENTRY"
        );
        assert_eq!(
            response["diagnostics"][0]["field"],
            "collision_catalog.relative_paths"
        );
    }
}
