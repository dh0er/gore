//! Canonical, bounded persistence for structurally verified NPC archetype linkage.
//!
//! A catalog can be built or reopened only with a closed [`StoryCatalogFile`] capability for the
//! exact registered generation and the exact Shipping/Binds bytes named by that capability.
//! Linkage is verified offline from sealed cache defaults. Runtime behavior, mod building,
//! deployment, and publication remain explicitly unsupported.

use std::io::{self, Write};

use gore_as::cache::npc_archetypes::{
    collect_npc_archetypes, NpcArchetypeClassEvidence, NpcArchetypeCollection,
    NpcArchetypeDefaultEdgeEvidence, NpcArchetypeRecord, NpcArchetypeRejection,
    NpcArchetypeRejectionReason, NpcArchetypeSeal, NpcBlueprintFamily, MAX_NPC_ARCHETYPES,
    MAX_NPC_BINDS_BYTES, MAX_NPC_CACHE_BYTES, MAX_NPC_CLASSES, MAX_NPC_STRING_BYTES,
};
use gore_story_catalog::{
    is_supported_generation, ContentSeal, GameGenerationSeal, Sha256Digest, StoryCatalogFile,
};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest as _, Sha256};

pub const MAX_CATALOG_JSON_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PAYLOAD_JSON_BYTES: usize = 15 * 1024 * 1024;
pub const MAX_SOURCE_IDENTITY_JSON_BYTES: usize = 4096;
pub const MAX_RECORDS: usize = MAX_NPC_ARCHETYPES;
pub const MAX_REJECTIONS: usize = MAX_NPC_CLASSES;
pub const MAX_TEXT_BYTES: usize = MAX_NPC_STRING_BYTES;
pub const MAX_TOTAL_TEXT_BYTES: usize = 12 * 1024 * 1024;

// A decoded JSON string can require at most six source bytes per output byte (`\uXXXX`).
// Bounding raw tokens before serde runs prevents an attacker-controlled `String` allocation up to
// the much larger whole-document limit while still accepting every valid bounded text value.
const MAX_JSON_STRING_TOKEN_BYTES: usize = MAX_TEXT_BYTES * 6;

const FORMAT: &str = "npc_archetype_catalog";
const SCHEMA_REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkageQualification {
    SealedLinkageVerified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeQualification {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcCatalogQualification {
    pub linkage: LinkageQualification,
    pub runtime: RuntimeQualification,
    pub build: SupportStatus,
    pub deploy: SupportStatus,
    pub publication: SupportStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcCatalogClassEvidence {
    pub class_name: String,
    pub super_class: Option<String>,
    pub module_name: String,
    pub relative_path: String,
    pub source_seal: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcCatalogDefaultEdgeEvidence {
    pub owner_class: String,
    pub field_name: String,
    pub assigned_value: String,
    pub instruction_offset_dwords: u64,
    pub init_defaults_bytecode_seal: ContentSeal,
    pub evidence_sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogBlueprintFamily {
    HumanBase,
    HumanWoman,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcCatalogRecord {
    pub spawn: NpcCatalogClassEvidence,
    pub ai_config: NpcCatalogClassEvidence,
    pub character_definition: NpcCatalogClassEvidence,
    pub actor_blueprint: String,
    pub blueprint_family: CatalogBlueprintFamily,
    pub spawn_ai_edge: NpcCatalogDefaultEdgeEvidence,
    pub spawn_blueprint_edge: NpcCatalogDefaultEdgeEvidence,
    pub ai_character_edge: NpcCatalogDefaultEdgeEvidence,
    pub evidence_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcCatalogRejectionReason {
    MissingInitDefaults {
        owner_class: String,
    },
    AmbiguousInitDefaults {
        owner_class: String,
        count: u64,
    },
    InvalidInitDefaultsBytecode {
        owner_class: String,
        detail: String,
    },
    MissingDefaultEdge {
        owner_class: String,
        field_name: String,
    },
    AmbiguousDefaultEdge {
        owner_class: String,
        field_name: String,
        count: u64,
    },
    MissingReferencedClass {
        role: String,
        class_name: String,
    },
    WrongAncestry {
        role: String,
        class_name: String,
        required_base: String,
    },
    InheritanceCycle {
        role: String,
        class_name: String,
    },
    NonInheritableClass {
        role: String,
        class_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcCatalogRejection {
    pub spawn_class: String,
    pub reason: NpcCatalogRejectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcCatalogPayload {
    extractor_records_sha256: Sha256Digest,
    #[serde(deserialize_with = "deserialize_records")]
    records: Vec<NpcCatalogRecord>,
    #[serde(deserialize_with = "deserialize_rejections")]
    rejections: Vec<NpcCatalogRejection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcCatalogSourceIdentity {
    shipping_cache: ContentSeal,
    binds_cache: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcCatalogSource {
    pub shipping_cache: ContentSeal,
    pub binds_cache: ContentSeal,
    pub source_pair_seal: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcCatalogBody {
    generation: GameGenerationSeal,
    story_catalog_seal: ContentSeal,
    qualification: NpcCatalogQualification,
    source: NpcCatalogSource,
    payload: NpcCatalogPayload,
    payload_seal: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcCatalogWire {
    format: String,
    schema_revision: u32,
    catalog: NpcCatalogBody,
    catalog_seal: ContentSeal,
}

/// Closed `npc_archetype_catalog.v1` capability.
///
/// Callers cannot construct or deserialize this type without the closed Story catalog and exact
/// generation bytes. It exposes read-only records and evidence; it provides no build, deploy, or
/// publication operation.
#[derive(Debug, Clone)]
pub struct NpcArchetypeCatalogFile {
    wire: NpcCatalogWire,
}

impl PartialEq for NpcArchetypeCatalogFile {
    fn eq(&self, other: &Self) -> bool {
        self.wire == other.wire
    }
}

impl Eq for NpcArchetypeCatalogFile {}

impl NpcArchetypeCatalogFile {
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, NpcCatalogError> {
        validate_wire_integrity(&self.wire)?;
        canonical_json(&self.wire, "NPC archetype catalog", MAX_CATALOG_JSON_BYTES)
    }

    pub fn from_json(
        bytes: &[u8],
        story_catalog: &StoryCatalogFile,
        shipping_cache: &[u8],
        binds_cache: &[u8],
    ) -> Result<Self, NpcCatalogError> {
        enforce_limit(
            "NPC archetype catalog JSON bytes",
            bytes.len(),
            MAX_CATALOG_JSON_BYTES,
        )?;
        preflight_json_string_tokens(bytes)?;
        validate_capability_and_inputs(story_catalog, shipping_cache, binds_cache)?;
        let wire: NpcCatalogWire =
            serde_json::from_slice(bytes).map_err(NpcCatalogError::InvalidJson)?;
        let canonical = canonical_json(&wire, "NPC archetype catalog", MAX_CATALOG_JSON_BYTES)?;
        if canonical != bytes {
            return Err(NpcCatalogError::NonCanonicalJson);
        }
        let result = Self { wire };
        validate_catalog_against_inputs(&result, story_catalog, shipping_cache, binds_cache)?;
        Ok(result)
    }

    pub fn generation(&self) -> &GameGenerationSeal {
        &self.wire.catalog.generation
    }

    pub fn story_catalog_seal(&self) -> &ContentSeal {
        &self.wire.catalog.story_catalog_seal
    }

    pub fn qualification(&self) -> &NpcCatalogQualification {
        &self.wire.catalog.qualification
    }

    pub fn source(&self) -> &NpcCatalogSource {
        &self.wire.catalog.source
    }

    pub fn payload_seal(&self) -> &ContentSeal {
        &self.wire.catalog.payload_seal
    }

    pub fn catalog_seal(&self) -> &ContentSeal {
        &self.wire.catalog_seal
    }

    pub fn extractor_records_sha256(&self) -> &Sha256Digest {
        &self.wire.catalog.payload.extractor_records_sha256
    }

    pub fn records(&self) -> &[NpcCatalogRecord] {
        &self.wire.catalog.payload.records
    }

    pub fn rejections(&self) -> &[NpcCatalogRejection] {
        &self.wire.catalog.payload.rejections
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NpcCatalogError {
    #[error("closed Story catalog capability is invalid: {0}")]
    InvalidStoryCatalog(String),
    #[error("unsupported Story catalog generation")]
    UnsupportedGeneration,
    #[error("{kind} is {actual} bytes/items; maximum is {max}")]
    LimitExceeded {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("{kind} does not match the closed Story catalog generation")]
    GenerationInputMismatch { kind: &'static str },
    #[error("NPC archetype extraction failed: {0}")]
    Extraction(String),
    #[error("invalid NPC archetype catalog JSON: {0}")]
    InvalidJson(serde_json::Error),
    #[error("NPC archetype catalog JSON is not canonical")]
    NonCanonicalJson,
    #[error("NPC archetype catalog seal mismatch for {0}")]
    SealMismatch(&'static str),
    #[error("invalid NPC archetype catalog invariant: {0}")]
    Invariant(String),
}

/// Build a closed canonical catalog from the exact generation bytes.
///
/// This is offline inspection only. It performs no mod build, deployment, game launch, runtime
/// qualification, filesystem write, or publication.
pub fn build_npc_archetype_catalog(
    story_catalog: &StoryCatalogFile,
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<NpcArchetypeCatalogFile, NpcCatalogError> {
    validate_capability_and_inputs(story_catalog, shipping_cache, binds_cache)?;
    let payload = extract_payload(story_catalog.generation(), shipping_cache, binds_cache)?;
    let source = build_source(story_catalog.generation())?;
    let payload_bytes = canonical_json(&payload, "NPC archetype payload", MAX_PAYLOAD_JSON_BYTES)?;
    let body = NpcCatalogBody {
        generation: story_catalog.generation().clone(),
        story_catalog_seal: story_catalog.catalog_seal().clone(),
        qualification: NpcCatalogQualification {
            linkage: LinkageQualification::SealedLinkageVerified,
            runtime: RuntimeQualification::RuntimeUnqualified,
            build: SupportStatus::NotSupported,
            deploy: SupportStatus::NotSupported,
            publication: SupportStatus::NotSupported,
        },
        source,
        payload,
        payload_seal: seal_bytes(&payload_bytes),
    };
    let body_bytes = canonical_json(&body, "NPC archetype catalog body", MAX_CATALOG_JSON_BYTES)?;
    let result = NpcArchetypeCatalogFile {
        wire: NpcCatalogWire {
            format: FORMAT.to_owned(),
            schema_revision: SCHEMA_REVISION,
            catalog: body,
            catalog_seal: seal_bytes(&body_bytes),
        },
    };
    validate_catalog_against_inputs(&result, story_catalog, shipping_cache, binds_cache)?;
    Ok(result)
}

fn validate_capability_and_inputs(
    story_catalog: &StoryCatalogFile,
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<(), NpcCatalogError> {
    story_catalog
        .to_canonical_json()
        .map_err(|error| NpcCatalogError::InvalidStoryCatalog(error.to_string()))?;
    let generation = story_catalog.generation();
    if !is_supported_generation(generation) {
        return Err(NpcCatalogError::UnsupportedGeneration);
    }
    validate_exact_input(
        "Shipping cache",
        shipping_cache,
        MAX_NPC_CACHE_BYTES,
        &generation.shipping_cache,
    )?;
    validate_exact_input(
        "Binds cache",
        binds_cache,
        MAX_NPC_BINDS_BYTES,
        &generation.binds_cache,
    )?;
    Ok(())
}

fn validate_exact_input(
    kind: &'static str,
    bytes: &[u8],
    max: usize,
    expected: &ContentSeal,
) -> Result<(), NpcCatalogError> {
    enforce_limit(kind, bytes.len(), max)?;
    if bytes.len() as u64 != expected.byte_len {
        return Err(NpcCatalogError::GenerationInputMismatch { kind });
    }
    if seal_bytes(bytes) != *expected {
        return Err(NpcCatalogError::GenerationInputMismatch { kind });
    }
    Ok(())
}

fn extract_payload(
    generation: &GameGenerationSeal,
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<NpcCatalogPayload, NpcCatalogError> {
    let collection = collect_npc_archetypes(
        shipping_cache,
        binds_cache,
        *generation.shipping_cache.sha256.as_bytes(),
        *generation.binds_cache.sha256.as_bytes(),
    )
    .map_err(|error| NpcCatalogError::Extraction(error.to_string()))?;
    if convert_seal(collection.shipping_cache_seal) != generation.shipping_cache
        || convert_seal(collection.binds_cache_seal) != generation.binds_cache
    {
        return Err(NpcCatalogError::Invariant(
            "extractor source seals differ from the closed generation".to_owned(),
        ));
    }
    collection_to_payload(collection)
}

fn collection_to_payload(
    collection: NpcArchetypeCollection,
) -> Result<NpcCatalogPayload, NpcCatalogError> {
    let mut payload = NpcCatalogPayload {
        extractor_records_sha256: Sha256Digest::from_bytes(collection.records_sha256),
        records: collection.records.into_iter().map(convert_record).collect(),
        rejections: collection
            .rejections
            .into_iter()
            .map(convert_rejection)
            .collect(),
    };
    normalize_payload(&mut payload);
    validate_payload(&payload)?;
    Ok(payload)
}

fn convert_record(record: NpcArchetypeRecord) -> NpcCatalogRecord {
    NpcCatalogRecord {
        spawn: convert_class(record.spawn),
        ai_config: convert_class(record.ai_config),
        character_definition: convert_class(record.character_definition),
        actor_blueprint: record.actor_blueprint,
        blueprint_family: match record.blueprint_family {
            NpcBlueprintFamily::HumanBase => CatalogBlueprintFamily::HumanBase,
            NpcBlueprintFamily::HumanWoman => CatalogBlueprintFamily::HumanWoman,
            NpcBlueprintFamily::Other => CatalogBlueprintFamily::Other,
        },
        spawn_ai_edge: convert_edge(record.spawn_ai_edge),
        spawn_blueprint_edge: convert_edge(record.spawn_blueprint_edge),
        ai_character_edge: convert_edge(record.ai_character_edge),
        evidence_sha256: Sha256Digest::from_bytes(record.evidence_sha256),
    }
}

fn convert_class(value: NpcArchetypeClassEvidence) -> NpcCatalogClassEvidence {
    NpcCatalogClassEvidence {
        class_name: value.class_name,
        super_class: value.super_class,
        module_name: value.module_name,
        relative_path: value.relative_path,
        source_seal: convert_seal(value.source_seal),
    }
}

fn convert_edge(value: NpcArchetypeDefaultEdgeEvidence) -> NpcCatalogDefaultEdgeEvidence {
    NpcCatalogDefaultEdgeEvidence {
        owner_class: value.owner_class,
        field_name: value.field_name,
        assigned_value: value.assigned_value,
        instruction_offset_dwords: value.instruction_offset_dwords as u64,
        init_defaults_bytecode_seal: convert_seal(value.init_defaults_bytecode_seal),
        evidence_sha256: Sha256Digest::from_bytes(value.evidence_sha256),
    }
}

fn convert_seal(value: NpcArchetypeSeal) -> ContentSeal {
    ContentSeal {
        byte_len: value.byte_len,
        sha256: Sha256Digest::from_bytes(value.sha256),
    }
}

fn convert_rejection(value: NpcArchetypeRejection) -> NpcCatalogRejection {
    NpcCatalogRejection {
        spawn_class: value.spawn_class,
        reason: match value.reason {
            NpcArchetypeRejectionReason::MissingInitDefaults { owner_class } => {
                NpcCatalogRejectionReason::MissingInitDefaults { owner_class }
            }
            NpcArchetypeRejectionReason::AmbiguousInitDefaults { owner_class, count } => {
                NpcCatalogRejectionReason::AmbiguousInitDefaults {
                    owner_class,
                    count: count as u64,
                }
            }
            NpcArchetypeRejectionReason::InvalidInitDefaultsBytecode {
                owner_class,
                detail,
            } => NpcCatalogRejectionReason::InvalidInitDefaultsBytecode {
                owner_class,
                detail,
            },
            NpcArchetypeRejectionReason::MissingDefaultEdge {
                owner_class,
                field_name,
            } => NpcCatalogRejectionReason::MissingDefaultEdge {
                owner_class,
                field_name,
            },
            NpcArchetypeRejectionReason::AmbiguousDefaultEdge {
                owner_class,
                field_name,
                count,
            } => NpcCatalogRejectionReason::AmbiguousDefaultEdge {
                owner_class,
                field_name,
                count: count as u64,
            },
            NpcArchetypeRejectionReason::MissingReferencedClass { role, class_name } => {
                NpcCatalogRejectionReason::MissingReferencedClass {
                    role: role.to_owned(),
                    class_name,
                }
            }
            NpcArchetypeRejectionReason::WrongAncestry {
                role,
                class_name,
                required_base,
            } => NpcCatalogRejectionReason::WrongAncestry {
                role: role.to_owned(),
                class_name,
                required_base: required_base.to_owned(),
            },
            NpcArchetypeRejectionReason::InheritanceCycle { role, class_name } => {
                NpcCatalogRejectionReason::InheritanceCycle {
                    role: role.to_owned(),
                    class_name,
                }
            }
            NpcArchetypeRejectionReason::NonInheritableClass { role, class_name } => {
                NpcCatalogRejectionReason::NonInheritableClass {
                    role: role.to_owned(),
                    class_name,
                }
            }
        },
    }
}

fn build_source(generation: &GameGenerationSeal) -> Result<NpcCatalogSource, NpcCatalogError> {
    let identity = NpcCatalogSourceIdentity {
        shipping_cache: generation.shipping_cache.clone(),
        binds_cache: generation.binds_cache.clone(),
    };
    let bytes = canonical_json(
        &identity,
        "NPC source identity",
        MAX_SOURCE_IDENTITY_JSON_BYTES,
    )?;
    Ok(NpcCatalogSource {
        shipping_cache: identity.shipping_cache,
        binds_cache: identity.binds_cache,
        source_pair_seal: seal_bytes(&bytes),
    })
}

fn normalize_payload(payload: &mut NpcCatalogPayload) {
    payload.records.sort_by(|left, right| {
        left.spawn
            .class_name
            .cmp(&right.spawn.class_name)
            .then_with(|| left.evidence_sha256.cmp(&right.evidence_sha256))
    });
    payload.records.dedup();
    payload.rejections.sort_by(|left, right| {
        left.spawn_class
            .cmp(&right.spawn_class)
            .then_with(|| rejection_sort_key(&left.reason).cmp(rejection_sort_key(&right.reason)))
    });
    payload.rejections.dedup();
}

fn rejection_sort_key(reason: &NpcCatalogRejectionReason) -> &'static str {
    match reason {
        NpcCatalogRejectionReason::MissingInitDefaults { .. } => "missing_init_defaults",
        NpcCatalogRejectionReason::AmbiguousInitDefaults { .. } => "ambiguous_init_defaults",
        NpcCatalogRejectionReason::InvalidInitDefaultsBytecode { .. } => {
            "invalid_init_defaults_bytecode"
        }
        NpcCatalogRejectionReason::MissingDefaultEdge { .. } => "missing_default_edge",
        NpcCatalogRejectionReason::AmbiguousDefaultEdge { .. } => "ambiguous_default_edge",
        NpcCatalogRejectionReason::MissingReferencedClass { .. } => "missing_referenced_class",
        NpcCatalogRejectionReason::WrongAncestry { .. } => "wrong_ancestry",
        NpcCatalogRejectionReason::InheritanceCycle { .. } => "inheritance_cycle",
        NpcCatalogRejectionReason::NonInheritableClass { .. } => "non_inheritable_class",
    }
}

fn validate_catalog_against_inputs(
    catalog: &NpcArchetypeCatalogFile,
    story_catalog: &StoryCatalogFile,
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<(), NpcCatalogError> {
    validate_capability_and_inputs(story_catalog, shipping_cache, binds_cache)?;
    validate_wire_integrity(&catalog.wire)?;
    if &catalog.wire.catalog.generation != story_catalog.generation() {
        return Err(NpcCatalogError::UnsupportedGeneration);
    }
    if &catalog.wire.catalog.story_catalog_seal != story_catalog.catalog_seal() {
        return Err(NpcCatalogError::Invariant(
            "Story catalog capability seal differs from the NPC catalog".to_owned(),
        ));
    }
    let expected_payload =
        extract_payload(story_catalog.generation(), shipping_cache, binds_cache)?;
    if catalog.wire.catalog.payload != expected_payload {
        return Err(NpcCatalogError::Invariant(
            "payload differs from extraction over the exact generation bytes".to_owned(),
        ));
    }
    Ok(())
}

fn validate_wire_integrity(wire: &NpcCatalogWire) -> Result<(), NpcCatalogError> {
    if wire.format != FORMAT {
        return Err(NpcCatalogError::Invariant(format!(
            "format {:?} is not {:?}",
            wire.format, FORMAT
        )));
    }
    if wire.schema_revision != SCHEMA_REVISION {
        return Err(NpcCatalogError::Invariant(format!(
            "schema revision {} is not {}",
            wire.schema_revision, SCHEMA_REVISION
        )));
    }
    if !is_supported_generation(&wire.catalog.generation) {
        return Err(NpcCatalogError::UnsupportedGeneration);
    }
    if wire.catalog.source.shipping_cache != wire.catalog.generation.shipping_cache
        || wire.catalog.source.binds_cache != wire.catalog.generation.binds_cache
    {
        return Err(NpcCatalogError::Invariant(
            "source seals differ from generation seals".to_owned(),
        ));
    }
    let expected_source = build_source(&wire.catalog.generation)?;
    if wire.catalog.source != expected_source {
        return Err(NpcCatalogError::SealMismatch("source pair"));
    }
    validate_payload(&wire.catalog.payload)?;
    let payload_bytes = canonical_json(
        &wire.catalog.payload,
        "NPC archetype payload",
        MAX_PAYLOAD_JSON_BYTES,
    )?;
    if wire.catalog.payload_seal != seal_bytes(&payload_bytes) {
        return Err(NpcCatalogError::SealMismatch("payload"));
    }
    let body_bytes = canonical_json(
        &wire.catalog,
        "NPC archetype catalog body",
        MAX_CATALOG_JSON_BYTES,
    )?;
    if wire.catalog_seal != seal_bytes(&body_bytes) {
        return Err(NpcCatalogError::SealMismatch("catalog"));
    }
    Ok(())
}

fn validate_payload(payload: &NpcCatalogPayload) -> Result<(), NpcCatalogError> {
    enforce_limit("NPC record count", payload.records.len(), MAX_RECORDS)?;
    enforce_limit(
        "NPC rejection count",
        payload.rejections.len(),
        MAX_REJECTIONS,
    )?;
    validate_strict_order(
        "record spawn classes",
        payload
            .records
            .iter()
            .map(|record| record.spawn.class_name.as_str()),
    )?;
    validate_strict_order(
        "rejected spawn classes",
        payload
            .rejections
            .iter()
            .map(|rejection| rejection.spawn_class.as_str()),
    )?;

    let mut text = TextBudget::default();
    for record in &payload.records {
        validate_record(record, &mut text)?;
    }
    for rejection in &payload.rejections {
        validate_rejection(rejection, &mut text)?;
    }
    Ok(())
}

fn validate_record(
    record: &NpcCatalogRecord,
    text: &mut TextBudget,
) -> Result<(), NpcCatalogError> {
    for class in [
        &record.spawn,
        &record.ai_config,
        &record.character_definition,
    ] {
        text.add("class name", &class.class_name)?;
        if let Some(super_class) = &class.super_class {
            text.add("super class", super_class)?;
        }
        text.add("module name", &class.module_name)?;
        text.add("relative path", &class.relative_path)?;
        validate_nonempty_seal("emitted source", &class.source_seal)?;
    }
    text.add("actor blueprint", &record.actor_blueprint)?;
    validate_edge(&record.spawn_ai_edge, text)?;
    validate_edge(&record.spawn_blueprint_edge, text)?;
    validate_edge(&record.ai_character_edge, text)?;
    if record.spawn_ai_edge.owner_class != record.spawn.class_name
        || record.spawn_ai_edge.assigned_value != record.ai_config.class_name
        || record.spawn_blueprint_edge.owner_class != record.spawn.class_name
        || record.spawn_blueprint_edge.assigned_value != record.actor_blueprint
        || record.ai_character_edge.owner_class != record.ai_config.class_name
        || record.ai_character_edge.assigned_value != record.character_definition.class_name
    {
        return Err(NpcCatalogError::Invariant(format!(
            "record {:?} has internally inconsistent linkage evidence",
            record.spawn.class_name
        )));
    }
    if record.evidence_sha256.as_bytes() == &[0; 32] {
        return Err(NpcCatalogError::Invariant(format!(
            "record {:?} has a zero evidence digest",
            record.spawn.class_name
        )));
    }
    Ok(())
}

fn validate_edge(
    edge: &NpcCatalogDefaultEdgeEvidence,
    text: &mut TextBudget,
) -> Result<(), NpcCatalogError> {
    text.add("edge owner", &edge.owner_class)?;
    text.add("edge field", &edge.field_name)?;
    text.add("edge value", &edge.assigned_value)?;
    validate_nonempty_seal("initializer bytecode", &edge.init_defaults_bytecode_seal)?;
    if edge.evidence_sha256.as_bytes() == &[0; 32] {
        return Err(NpcCatalogError::Invariant(
            "default edge has a zero evidence digest".to_owned(),
        ));
    }
    Ok(())
}

fn validate_rejection(
    rejection: &NpcCatalogRejection,
    text: &mut TextBudget,
) -> Result<(), NpcCatalogError> {
    text.add("rejected spawn", &rejection.spawn_class)?;
    match &rejection.reason {
        NpcCatalogRejectionReason::MissingInitDefaults { owner_class }
        | NpcCatalogRejectionReason::AmbiguousInitDefaults { owner_class, .. }
        | NpcCatalogRejectionReason::InvalidInitDefaultsBytecode { owner_class, .. }
        | NpcCatalogRejectionReason::MissingDefaultEdge { owner_class, .. }
        | NpcCatalogRejectionReason::AmbiguousDefaultEdge { owner_class, .. } => {
            text.add("rejection owner", owner_class)?;
        }
        NpcCatalogRejectionReason::MissingReferencedClass { role, class_name }
        | NpcCatalogRejectionReason::InheritanceCycle { role, class_name }
        | NpcCatalogRejectionReason::NonInheritableClass { role, class_name } => {
            text.add("rejection role", role)?;
            text.add("rejected class", class_name)?;
        }
        NpcCatalogRejectionReason::WrongAncestry {
            role,
            class_name,
            required_base,
        } => {
            text.add("rejection role", role)?;
            text.add("rejected class", class_name)?;
            text.add("required base", required_base)?;
        }
    }
    match &rejection.reason {
        NpcCatalogRejectionReason::InvalidInitDefaultsBytecode { detail, .. } => {
            text.add("bytecode rejection detail", detail)?;
        }
        NpcCatalogRejectionReason::MissingDefaultEdge { field_name, .. }
        | NpcCatalogRejectionReason::AmbiguousDefaultEdge { field_name, .. } => {
            text.add("rejected field", field_name)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_nonempty_seal(kind: &'static str, seal: &ContentSeal) -> Result<(), NpcCatalogError> {
    if seal.byte_len == 0 || seal.sha256.as_bytes() == &[0; 32] {
        return Err(NpcCatalogError::Invariant(format!(
            "{kind} seal must be nonempty"
        )));
    }
    Ok(())
}

fn validate_strict_order<'a>(
    kind: &'static str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), NpcCatalogError> {
    let mut prior = None;
    for value in values {
        if prior.is_some_and(|previous| previous >= value) {
            return Err(NpcCatalogError::Invariant(format!(
                "{kind} must be strictly increasing"
            )));
        }
        prior = Some(value);
    }
    Ok(())
}

#[derive(Default)]
struct TextBudget {
    bytes: usize,
}

impl TextBudget {
    fn add(&mut self, kind: &'static str, value: &str) -> Result<(), NpcCatalogError> {
        if value.is_empty() {
            return Err(NpcCatalogError::Invariant(format!(
                "{kind} must not be empty"
            )));
        }
        enforce_limit(kind, value.len(), MAX_TEXT_BYTES)?;
        let next = self.bytes.saturating_add(value.len());
        enforce_limit("aggregate catalog text bytes", next, MAX_TOTAL_TEXT_BYTES)?;
        self.bytes = next;
        Ok(())
    }
}

fn deserialize_records<'de, D>(deserializer: D) -> Result<Vec<NpcCatalogRecord>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_sequence(deserializer, MAX_RECORDS, "NPC archetype records")
}

fn deserialize_rejections<'de, D>(deserializer: D) -> Result<Vec<NpcCatalogRejection>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_sequence(deserializer, MAX_REJECTIONS, "NPC archetype rejections")
}

fn deserialize_bounded_sequence<'de, D, T>(
    deserializer: D,
    max: usize,
    label: &'static str,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct BoundedSequenceVisitor<T> {
        max: usize,
        label: &'static str,
        marker: std::marker::PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for BoundedSequenceVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "at most {} {}", self.max, self.label)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let hint = sequence.size_hint().unwrap_or(0);
            if hint > self.max {
                return Err(de::Error::invalid_length(hint, &self));
            }
            let mut values = Vec::with_capacity(hint.min(self.max));
            while values.len() < self.max {
                match sequence.next_element()? {
                    Some(value) => values.push(value),
                    None => return Ok(values),
                }
            }
            if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_length(self.max + 1, &self));
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(BoundedSequenceVisitor {
        max,
        label,
        marker: std::marker::PhantomData,
    })
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn canonical_json<T: Serialize>(
    value: &T,
    kind: &'static str,
    max: usize,
) -> Result<Vec<u8>, NpcCatalogError> {
    struct BoundedBuffer {
        bytes: Vec<u8>,
        max: usize,
        exceeded: bool,
    }

    impl Write for BoundedBuffer {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let next = self.bytes.len().checked_add(buffer.len());
            if next.is_none_or(|length| length > self.max) {
                self.exceeded = true;
                return Err(io::Error::other("canonical JSON limit exceeded"));
            }
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut output = BoundedBuffer {
        bytes: Vec::with_capacity(max.min(64 * 1024)),
        max,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut output, value);
    if output.exceeded {
        return Err(NpcCatalogError::LimitExceeded {
            kind,
            actual: max.saturating_add(1),
            max,
        });
    }
    result.map_err(NpcCatalogError::InvalidJson)?;
    Ok(output.bytes)
}

fn enforce_limit(kind: &'static str, actual: usize, max: usize) -> Result<(), NpcCatalogError> {
    if actual > max {
        Err(NpcCatalogError::LimitExceeded { kind, actual, max })
    } else {
        Ok(())
    }
}

fn preflight_json_string_tokens(bytes: &[u8]) -> Result<(), NpcCatalogError> {
    let mut in_string = false;
    let mut escaped = false;
    let mut raw_len = 0usize;

    for &byte in bytes {
        if !in_string {
            if byte == b'"' {
                in_string = true;
                escaped = false;
                raw_len = 0;
            }
            continue;
        }

        if byte == b'"' && !escaped {
            in_string = false;
            continue;
        }

        raw_len = raw_len.saturating_add(1);
        enforce_limit(
            "raw JSON string token bytes",
            raw_len,
            MAX_JSON_STRING_TOKEN_BYTES,
        )?;
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_story_catalog::known_supported_generations;
    use std::collections::BTreeSet;

    fn story_catalog() -> StoryCatalogFile {
        let fixture = include_bytes!("../../gore-ffi/tests/fixtures/story_catalog_v1.json");
        let canonical = fixture.strip_suffix(b"\n").unwrap_or(fixture);
        StoryCatalogFile::from_json(canonical).expect("closed Story catalog fixture")
    }

    fn class(name: &str) -> NpcCatalogClassEvidence {
        NpcCatalogClassEvidence {
            class_name: name.to_owned(),
            super_class: Some("UBase".to_owned()),
            module_name: "Synthetic.Npcs".to_owned(),
            relative_path: "Synthetic/Npcs.as".to_owned(),
            source_seal: seal_bytes(b"source"),
        }
    }

    fn edge(owner: &str, field: &str, value: &str, offset: u64) -> NpcCatalogDefaultEdgeEvidence {
        NpcCatalogDefaultEdgeEvidence {
            owner_class: owner.to_owned(),
            field_name: field.to_owned(),
            assigned_value: value.to_owned(),
            instruction_offset_dwords: offset,
            init_defaults_bytecode_seal: seal_bytes(b"bytecode"),
            evidence_sha256: Sha256Digest::from_bytes([offset as u8 + 1; 32]),
        }
    }

    fn record(spawn: &str) -> NpcCatalogRecord {
        let ai = format!("UAI_{spawn}");
        let character = format!("UCharacter_{spawn}");
        let blueprint = format!("Blueprint'/Game/{spawn}.{spawn}_C'");
        NpcCatalogRecord {
            spawn: class(spawn),
            ai_config: class(&ai),
            character_definition: class(&character),
            actor_blueprint: blueprint.clone(),
            blueprint_family: CatalogBlueprintFamily::Other,
            spawn_ai_edge: edge(spawn, "AIAgentConfigClass", &ai, 0),
            spawn_blueprint_edge: edge(spawn, "AIAgentCharacterClass", &blueprint, 10),
            ai_character_edge: edge(&ai, "m_CharacterDefinition", &character, 20),
            evidence_sha256: Sha256Digest::from_bytes([7; 32]),
        }
    }

    fn rejection(spawn: &str) -> NpcCatalogRejection {
        NpcCatalogRejection {
            spawn_class: spawn.to_owned(),
            reason: NpcCatalogRejectionReason::MissingDefaultEdge {
                owner_class: spawn.to_owned(),
                field_name: "AIAgentConfigClass".to_owned(),
            },
        }
    }

    fn payload() -> NpcCatalogPayload {
        NpcCatalogPayload {
            extractor_records_sha256: Sha256Digest::from_bytes([9; 32]),
            records: vec![record("USpawnB"), record("USpawnA")],
            rejections: vec![rejection("USpawnRejectedB"), rejection("USpawnRejectedA")],
        }
    }

    fn file_from_payload(payload: NpcCatalogPayload) -> NpcArchetypeCatalogFile {
        let story = story_catalog();
        file_from_payload_for_generation(
            payload,
            story.generation().clone(),
            story.catalog_seal().clone(),
        )
    }

    fn file_from_payload_for_generation(
        mut payload: NpcCatalogPayload,
        generation: GameGenerationSeal,
        story_catalog_seal: ContentSeal,
    ) -> NpcArchetypeCatalogFile {
        normalize_payload(&mut payload);
        validate_payload(&payload).unwrap();
        let source = build_source(&generation).unwrap();
        let payload_bytes = canonical_json(&payload, "payload", MAX_PAYLOAD_JSON_BYTES).unwrap();
        let body = NpcCatalogBody {
            generation,
            story_catalog_seal,
            qualification: NpcCatalogQualification {
                linkage: LinkageQualification::SealedLinkageVerified,
                runtime: RuntimeQualification::RuntimeUnqualified,
                build: SupportStatus::NotSupported,
                deploy: SupportStatus::NotSupported,
                publication: SupportStatus::NotSupported,
            },
            source,
            payload,
            payload_seal: seal_bytes(&payload_bytes),
        };
        let body_bytes = canonical_json(&body, "body", MAX_CATALOG_JSON_BYTES).unwrap();
        let file = NpcArchetypeCatalogFile {
            wire: NpcCatalogWire {
                format: FORMAT.to_owned(),
                schema_revision: SCHEMA_REVISION,
                catalog: body,
                catalog_seal: seal_bytes(&body_bytes),
            },
        };
        validate_wire_integrity(&file.wire).unwrap();
        file
    }

    #[test]
    fn every_audited_generation_is_accepted_only_as_an_exact_triple() {
        // This named the second generation and asserted it differed from the first, so it proved
        // nothing about any build added later. Reading the whole supported set instead means a new
        // audited generation is covered here the moment it is admitted anywhere.
        let audited = known_supported_generations();
        assert!(
            audited.len() >= 2,
            "one generation would make the distinctness check below vacuous"
        );
        for (index, generation) in audited.iter().enumerate() {
            for other in audited.iter().skip(index + 1) {
                assert_ne!(generation, other, "audited generations must be distinct");
            }
            assert!(is_supported_generation(generation));

            let file = file_from_payload_for_generation(
                payload(),
                generation.clone(),
                seal_bytes(b"trusted supported Story catalog"),
            );
            assert_eq!(file.generation(), generation);
            assert_eq!(file.source().shipping_cache, generation.shipping_cache);
            assert_eq!(file.source().binds_cache, generation.binds_cache);
            validate_wire_integrity(&file.wire).unwrap();

            let mut source_drift = file.wire.clone();
            source_drift.catalog.source.shipping_cache.sha256 =
                Sha256Digest::from_bytes([0x5c; 32]);
            let body = canonical_json(
                &source_drift.catalog,
                "source-drift body",
                MAX_CATALOG_JSON_BYTES,
            )
            .unwrap();
            source_drift.catalog_seal = seal_bytes(&body);
            assert!(matches!(
                validate_wire_integrity(&source_drift),
                Err(NpcCatalogError::Invariant(_))
            ));

            let mut nearby_unknown = file.wire.clone();
            nearby_unknown.catalog.generation.executable.sha256 =
                Sha256Digest::from_bytes([0xa5; 32]);
            assert_eq!(
                nearby_unknown.catalog.generation.edition,
                generation.edition
            );
            assert!(!is_supported_generation(&nearby_unknown.catalog.generation));
            let body = canonical_json(
                &nearby_unknown.catalog,
                "unknown-generation body",
                MAX_CATALOG_JSON_BYTES,
            )
            .unwrap();
            nearby_unknown.catalog_seal = seal_bytes(&body);
            assert!(matches!(
                validate_wire_integrity(&nearby_unknown),
                Err(NpcCatalogError::UnsupportedGeneration)
            ));
        }
    }

    #[test]
    fn normalization_sorts_deduplicates_and_seals_all_three_layers() {
        let mut payload = payload();
        payload.records.push(payload.records[0].clone());
        payload.rejections.push(payload.rejections[0].clone());
        let file = file_from_payload(payload);
        assert_eq!(
            file.records()
                .iter()
                .map(|record| record.spawn.class_name.as_str())
                .collect::<Vec<_>>(),
            ["USpawnA", "USpawnB"]
        );
        assert_eq!(
            file.rejections()
                .iter()
                .map(|rejection| rejection.spawn_class.as_str())
                .collect::<Vec<_>>(),
            ["USpawnRejectedA", "USpawnRejectedB"]
        );
        for seal in [
            &file.source().source_pair_seal,
            file.payload_seal(),
            file.catalog_seal(),
        ] {
            assert!(seal.byte_len > 0);
            assert_ne!(seal.sha256.as_bytes(), &[0; 32]);
        }
        assert_ne!(file.source().source_pair_seal, *file.payload_seal());
        assert_ne!(*file.payload_seal(), *file.catalog_seal());
        assert_eq!(
            file.qualification().linkage,
            LinkageQualification::SealedLinkageVerified
        );
        assert_eq!(
            file.qualification().runtime,
            RuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(file.qualification().build, SupportStatus::NotSupported);
        assert_eq!(file.qualification().deploy, SupportStatus::NotSupported);
        assert_eq!(
            file.qualification().publication,
            SupportStatus::NotSupported
        );
    }

    #[test]
    fn conflicting_duplicate_identity_and_linkage_drift_fail_closed() {
        let mut duplicate = payload();
        let mut conflict = duplicate.records[0].clone();
        conflict.actor_blueprint.push_str("_Different");
        duplicate.records.push(conflict);
        normalize_payload(&mut duplicate);
        assert!(matches!(
            validate_payload(&duplicate),
            Err(NpcCatalogError::Invariant(_))
        ));

        let mut file = file_from_payload(payload());
        file.wire.catalog.payload.records[0]
            .spawn_ai_edge
            .assigned_value = "UWrong".to_owned();
        assert!(matches!(
            validate_wire_integrity(&file.wire),
            Err(NpcCatalogError::Invariant(_))
        ));
    }

    #[test]
    fn bounded_sequence_rejects_the_first_excess_element() {
        fn deserialize_two<'de, D>(deserializer: D) -> Result<Vec<de::IgnoredAny>, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_bounded_sequence(deserializer, 2, "test entries")
        }

        #[derive(Debug, Deserialize)]
        struct Two(#[serde(deserialize_with = "deserialize_two")] Vec<de::IgnoredAny>);

        let accepted: Two = serde_json::from_str("[null,null]").unwrap();
        assert_eq!(accepted.0.len(), 2);
        let error = serde_json::from_str::<Two>("[null,null,null]").unwrap_err();
        assert!(error.to_string().contains("invalid length 3"));
    }

    #[test]
    fn json_string_preflight_rejects_before_deserializing_an_oversized_token() {
        let mut bytes = Vec::with_capacity(MAX_JSON_STRING_TOKEN_BYTES + 3);
        bytes.push(b'"');
        bytes.extend(std::iter::repeat_n(b'a', MAX_JSON_STRING_TOKEN_BYTES + 1));
        bytes.push(b'"');
        assert!(matches!(
            preflight_json_string_tokens(&bytes),
            Err(NpcCatalogError::LimitExceeded {
                kind: "raw JSON string token bytes",
                actual,
                max: MAX_JSON_STRING_TOKEN_BYTES,
            }) if actual == MAX_JSON_STRING_TOKEN_BYTES + 1
        ));

        let escaped_at_limit = format!("\"{}\"", "\\u0041".repeat(MAX_TEXT_BYTES));
        preflight_json_string_tokens(escaped_at_limit.as_bytes()).unwrap();
    }

    #[test]
    fn public_entrypoints_require_closed_capability_exact_bytes_and_prebound_json() {
        let story = story_catalog();
        assert!(matches!(
            build_npc_archetype_catalog(&story, &[], &[]),
            Err(NpcCatalogError::GenerationInputMismatch {
                kind: "Shipping cache"
            })
        ));
        let oversized = vec![b' '; MAX_CATALOG_JSON_BYTES + 1];
        assert!(matches!(
            NpcArchetypeCatalogFile::from_json(&oversized, &story, &[], &[]),
            Err(NpcCatalogError::LimitExceeded {
                kind: "NPC archetype catalog JSON bytes",
                ..
            })
        ));
        let canonical = file_from_payload(payload()).to_canonical_json().unwrap();
        assert!(matches!(
            NpcArchetypeCatalogFile::from_json(&canonical, &story, &[], &[]),
            Err(NpcCatalogError::GenerationInputMismatch {
                kind: "Shipping cache"
            })
        ));
    }

    #[test]
    fn conversion_preserves_every_structural_evidence_and_rejection_field() {
        let source = NpcArchetypeSeal {
            byte_len: 12,
            sha256: [1; 32],
        };
        let bytecode = NpcArchetypeSeal {
            byte_len: 24,
            sha256: [2; 32],
        };
        let class = |name: &str| NpcArchetypeClassEvidence {
            class_name: name.to_owned(),
            super_class: Some("UBase".to_owned()),
            module_name: "M".to_owned(),
            relative_path: "M.as".to_owned(),
            source_seal: source,
        };
        let edge =
            |owner: &str, field: &str, value: &str, offset| NpcArchetypeDefaultEdgeEvidence {
                owner_class: owner.to_owned(),
                field_name: field.to_owned(),
                assigned_value: value.to_owned(),
                instruction_offset_dwords: offset,
                init_defaults_bytecode_seal: bytecode,
                evidence_sha256: [offset as u8 + 3; 32],
            };
        let collection = NpcArchetypeCollection {
            shipping_cache_seal: NpcArchetypeSeal {
                byte_len: 1,
                sha256: [8; 32],
            },
            binds_cache_seal: NpcArchetypeSeal {
                byte_len: 1,
                sha256: [9; 32],
            },
            records: vec![NpcArchetypeRecord {
                spawn: class("USpawn"),
                ai_config: class("UAI"),
                character_definition: class("UCharacter"),
                actor_blueprint: "Blueprint'/Game/Human.Human_C'".to_owned(),
                blueprint_family: NpcBlueprintFamily::HumanBase,
                spawn_ai_edge: edge("USpawn", "AIAgentConfigClass", "UAI", 1),
                spawn_blueprint_edge: edge(
                    "USpawn",
                    "AIAgentCharacterClass",
                    "Blueprint'/Game/Human.Human_C'",
                    2,
                ),
                ai_character_edge: edge("UAI", "m_CharacterDefinition", "UCharacter", 3),
                evidence_sha256: [4; 32],
            }],
            rejections: vec![NpcArchetypeRejection {
                spawn_class: "URejected".to_owned(),
                reason: NpcArchetypeRejectionReason::WrongAncestry {
                    role: "AI config",
                    class_name: "UBad".to_owned(),
                    required_base: "UAIAgentConfig_Human",
                },
            }],
            records_sha256: [5; 32],
        };
        let payload = collection_to_payload(collection).unwrap();
        let record = &payload.records[0];
        assert_eq!(record.spawn.source_seal, convert_seal(source));
        assert_eq!(
            record.spawn_ai_edge.init_defaults_bytecode_seal,
            convert_seal(bytecode)
        );
        assert_eq!(record.spawn_ai_edge.instruction_offset_dwords, 1);
        assert_eq!(record.spawn_ai_edge.evidence_sha256.as_bytes(), &[4; 32]);
        assert_eq!(record.evidence_sha256.as_bytes(), &[4; 32]);
        assert!(matches!(
            payload.rejections[0].reason,
            NpcCatalogRejectionReason::WrongAncestry {
                ref role,
                ref class_name,
                ref required_base,
            } if role == "AI config" && class_name == "UBad" && required_base == "UAIAgentConfig_Human"
        ));
    }

    #[test]
    fn canonical_wire_is_stable_and_rejects_unknown_or_unordered_content() {
        let file = file_from_payload(payload());
        let first = file.to_canonical_json().unwrap();
        let second = file.to_canonical_json().unwrap();
        assert_eq!(first, second);
        let parsed: NpcCatalogWire = serde_json::from_slice(&first).unwrap();
        assert_eq!(parsed, file.wire);

        let mut unordered = file.wire.clone();
        unordered.catalog.payload.records.reverse();
        assert!(matches!(
            validate_wire_integrity(&unordered),
            Err(NpcCatalogError::Invariant(_))
        ));
        let text = String::from_utf8(first).unwrap();
        let unknown = text.replacen("{\"format\"", "{\"unknown\":1,\"format\"", 1);
        assert!(serde_json::from_str::<NpcCatalogWire>(&unknown).is_err());
    }

    #[test]
    #[ignore = "requires explicit GORE_AS_REAL_CACHE and GORE_AS_BINDS sealed fixtures"]
    fn configured_real_catalog_golden_is_stable_and_reopens() {
        let cache_path = std::env::var_os("GORE_AS_REAL_CACHE")
            .expect("GORE_AS_REAL_CACHE is required for the ignored real NPC catalog golden");
        let binds_path = std::env::var_os("GORE_AS_BINDS")
            .expect("GORE_AS_BINDS is required for the ignored real NPC catalog golden");
        let cache = std::fs::read(cache_path).expect("read configured Shipping cache");
        let binds = std::fs::read(binds_path).expect("read configured Binds cache");
        let story = story_catalog();
        let catalog = build_npc_archetype_catalog(&story, &cache, &binds)
            .expect("build real NPC archetype catalog");
        assert_eq!(catalog.records().len(), 634);
        assert_eq!(catalog.rejections().len(), 416);
        assert_eq!(
            catalog
                .records()
                .iter()
                .map(|record| record.ai_config.class_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            621
        );
        assert_eq!(
            catalog.extractor_records_sha256().to_string(),
            "c2e82d49572b00f01490e1b1b2c12d58099c5f4057b39bed2d4e5bb829367536"
        );
        let bytes = catalog.to_canonical_json().unwrap();
        let reopened = NpcArchetypeCatalogFile::from_json(&bytes, &story, &cache, &binds)
            .expect("reopen real NPC archetype catalog");
        assert_eq!(reopened, catalog);
        assert_eq!(bytes.len(), 1_808_069);
        assert_eq!(catalog.source().source_pair_seal.byte_len, 228);
        assert_eq!(catalog.payload_seal().byte_len, 1_806_762);
        assert_eq!(catalog.catalog_seal().byte_len, 1_807_892);
        assert_eq!(
            catalog.source().source_pair_seal.sha256.to_string(),
            "aaeabcbee66bfd7402d88282827e76393fbbcb03d9a9e8f8f8eae4d38c056dd4"
        );
        assert_eq!(
            catalog.payload_seal().sha256.to_string(),
            "bc84dd8023a2df28e280e385e363748884fe5a49a94e78c990aacfe6271c6d7d"
        );
        assert_eq!(
            catalog.catalog_seal().sha256.to_string(),
            "b7f1f08f1c10b38a461af45724d9e722c670e67cad49e00356851a85cda46ec1"
        );
    }
}
