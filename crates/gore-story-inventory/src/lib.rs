//! Canonical, sealed base-game AngelScript collision inventories.
//!
//! This crate turns the bounded cache-only collector in `gore-as` into a durable artifact. An
//! artifact is tied to one exact game-generation description and to the exact Shipping/Binds
//! bytes from which its module, relative-path, and conservative bare-symbol sets were collected.
//! Reopening is intentionally expensive and fail-closed: the caller supplies those source bytes,
//! every source seal is checked, the inventory is collected again, and the result must match.
//!
//! Revision 1 covers only the unmodified base-game cache layer. It is **not** a resolved mod
//! loadout, does not qualify AngelScript runtime behavior, and cannot authorize build, deployment,
//! or publication. In particular, it must not be relabelled as `resolved-loadout.scripts.v1` to
//! bypass Quest collision checks. A later loadout artifact can deterministically union this layer
//! with enabled mod/project layers while retaining their individual provenance.

use std::marker::PhantomData;

use gore_as::cache::collision_inventory::{
    collect_collision_inventory, CollisionInventory, CollisionInventoryError,
    MAX_COLLISION_ENTRIES, MAX_COLLISION_ENTRY_BYTES, MAX_COLLISION_TOTAL_BYTES,
};
use gore_story_catalog::{known_generation_v1, GameGenerationSeal, StoryCatalogFile};
pub use gore_story_catalog::{ContentSeal, Sha256Digest};
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

mod quest_capability;
mod quest_capability_v2;
mod revision3_quest_context_transaction_v1;
mod revision3_quest_persistence_v3;
mod revision3_quest_transaction_v3;

// The S3 end-to-end test needs the private synthetic source fixture below to create a genuinely
// fresh capability, while `gore-story-build` normally depends on this crate. Including only the
// production implementation under `cfg(test)` avoids both a dev-dependency cycle and any
// production test constructor/authority API. The same file is also compiled normally by
// `gore-story-build`; its unit tests live outside the included implementation.
#[cfg(test)]
extern crate self as gore_story_inventory;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../gore-story-build/src/revision3_quest.rs"]
mod revision3_quest_s3_test_subject;

pub use quest_capability::{
    reopen_quest_collision_capability_artifact_v1, PreparedQuestCollisionArtifactFinalizeError,
    PreparedQuestCollisionArtifactV1, QuestCollisionBuildStatus,
    QuestCollisionCapabilityArtifactError, QuestCollisionCapabilityArtifactV1,
    QuestCollisionCapabilityArtifactVerificationError, QuestCollisionCapabilityError,
    QuestCollisionCoverage, QuestCollisionPublicationStatus, QuestCollisionRuntimeQualification,
    VerifiedQuestCollisionCapability, BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
};
pub use quest_capability_v2::{
    reopen_quest_collision_capability_artifact_v2, PreparedQuestCollisionArtifactFinalizeErrorV2,
    PreparedQuestCollisionArtifactV2, QuestCollisionCapabilityArtifactErrorV2,
    QuestCollisionCapabilityArtifactV2,
    Revision3QuestCollisionCapabilityArtifactVerificationErrorV2,
    Revision3QuestCollisionCapabilityErrorV2, Revision3QuestCollisionCoverageV2,
    VerifiedRevision3QuestCollisionCapabilityV2,
    BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
};
pub use revision3_quest_context_transaction_v1::{
    apply_revision3_quest_context_edit_transaction_v1, Revision3QuestContextEditBindingErrorV1,
    Revision3QuestContextEditBuildStatusV1, Revision3QuestContextEditConflictV1,
    Revision3QuestContextEditErrorV1, Revision3QuestContextEditOutcomeV1,
    Revision3QuestContextEditProjectTransportErrorV1, Revision3QuestContextEditPublicationStatusV1,
    Revision3QuestContextEditRequestJsonErrorV1, Revision3QuestContextEditRequestV1,
    Revision3QuestContextEditRuntimeStatusV1,
    MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1,
};
pub use revision3_quest_persistence_v3::{
    prepare_revision3_quest_draft_persistence_v3, Revision3QuestDraftPersistenceErrorV3,
    Revision3QuestDraftPersistencePreparationV3, Revision3QuestDraftPersistenceValidationErrorV3,
};
pub use revision3_quest_transaction_v3::{
    apply_revision3_quest_draft_transaction_v3, Revision3QuestArtifactAuthorityV3,
    Revision3QuestDraftBindingErrorV3, Revision3QuestDraftBuildStatusV3,
    Revision3QuestDraftConflictV3, Revision3QuestDraftInsertErrorV3,
    Revision3QuestDraftInsertOutcomeV3, Revision3QuestDraftInsertRequestJsonErrorV3,
    Revision3QuestDraftInsertRequestV3, Revision3QuestDraftIntentV3,
    Revision3QuestDraftProjectTransportErrorV3, Revision3QuestDraftPublicationStatusV3,
    Revision3QuestDraftRuntimeStatusV3, Revision3QuestEntityRoleV3,
    Revision3QuestSourceInspectionStatusV3, Revision3StoryIdentityKindV3,
    MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3,
};

/// Exact identity of the only layer represented by revision 1.
pub const BASE_GAME_SCRIPT_INVENTORY_LAYER: &str = "base-game.g1r.scripts.inventory.v1";
/// Maximum accepted canonical artifact envelope.
pub const MAX_INVENTORY_JSON_BYTES: usize = 24 * 1024 * 1024;
/// Maximum edition label retained in the generation description.
pub const MAX_GENERATION_EDITION_BYTES: usize = 256;
/// Hard in-memory bound applied to Shipping cache bytes before hashing or parsing.
pub const MAX_SHIPPING_CACHE_SOURCE_BYTES: usize = 1024 * 1024 * 1024;
/// Hard in-memory bound applied to Binds cache bytes before hashing or parsing.
pub const MAX_BINDS_CACHE_SOURCE_BYTES: usize = 128 * 1024 * 1024;

const INVENTORY_FORMAT: &str = "story_script_collision_inventory";
const INVENTORY_SCHEMA_REVISION: u32 = 1;
const SOURCE_PAIR_DOMAIN: &[u8] = b"gore-story-inventory.v1.shipping-plus-binds\0";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct InventoryFormat;

impl Serialize for InventoryFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(INVENTORY_FORMAT)
    }
}

impl<'de> Deserialize<'de> for InventoryFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = BoundedString::<64>::deserialize(deserializer)?.0;
        if value == INVENTORY_FORMAT {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported collision inventory format {value:?}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct InventorySchemaRevision;

impl Serialize for InventorySchemaRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(INVENTORY_SCHEMA_REVISION)
    }
}

impl<'de> Deserialize<'de> for InventorySchemaRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == INVENTORY_SCHEMA_REVISION {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported collision inventory schema revision {value}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct BaseGameLayer;

impl Serialize for BaseGameLayer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(BASE_GAME_SCRIPT_INVENTORY_LAYER)
    }
}

impl<'de> Deserialize<'de> for BaseGameLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = BoundedString::<128>::deserialize(deserializer)?.0;
        if value == BASE_GAME_SCRIPT_INVENTORY_LAYER {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported collision inventory layer {value:?}"
            )))
        }
    }
}

/// Honest coverage statement carried by every revision-1 artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryCoverage {
    BaseGameOnly,
}

/// This offline inventory never constitutes runtime qualification evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryRuntimeQualification {
    RuntimeUnqualified,
}

/// Publication is outside this artifact's capability boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryPublicationStatus {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventorySource {
    shipping_cache: ContentSeal,
    binds_cache: ContentSeal,
    /// Domain-separated seal over the ordered pair of exact source buffers.
    source_pair_seal: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryPayload {
    generation: GameGenerationSeal,
    story_catalog_seal: ContentSeal,
    catalog_layer: BaseGameLayer,
    coverage: InventoryCoverage,
    runtime_qualification: InventoryRuntimeQualification,
    publication_status: InventoryPublicationStatus,
    source: InventorySource,
    #[serde(deserialize_with = "deserialize_collision_entries")]
    modules: Vec<String>,
    #[serde(deserialize_with = "deserialize_collision_entries")]
    relative_paths: Vec<String>,
    #[serde(deserialize_with = "deserialize_collision_entries")]
    symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryWire {
    format: InventoryFormat,
    schema_revision: InventorySchemaRevision,
    inventory: InventoryPayload,
    payload_seal: ContentSeal,
}

/// Fully source-verified revision-1 collision inventory.
///
/// Callers cannot construct or deserialize this type directly. Use
/// [`build_base_game_inventory`] or [`reopen_base_game_inventory`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseGameCollisionInventory {
    wire: InventoryWire,
}

impl BaseGameCollisionInventory {
    /// Return the exact canonical JSON spelling of this verified artifact.
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, StoryInventoryError> {
        validate_wire(&self.wire)?;
        canonical_json(&self.wire, "collision inventory artifact")
    }

    pub fn generation(&self) -> &GameGenerationSeal {
        &self.wire.inventory.generation
    }

    pub const fn catalog_layer(&self) -> &'static str {
        BASE_GAME_SCRIPT_INVENTORY_LAYER
    }

    pub fn source_pair_seal(&self) -> &ContentSeal {
        &self.wire.inventory.source.source_pair_seal
    }

    /// Seal of the exact trusted story catalog that authorized this base-game inventory.
    pub fn story_catalog_seal(&self) -> &ContentSeal {
        &self.wire.inventory.story_catalog_seal
    }

    /// Seal of the canonical inner inventory payload.
    ///
    /// This is suitable as immutable artifact provenance. It is not runtime or resolved-loadout
    /// evidence.
    pub fn payload_seal(&self) -> &ContentSeal {
        &self.wire.payload_seal
    }

    pub const fn coverage(&self) -> InventoryCoverage {
        InventoryCoverage::BaseGameOnly
    }

    pub const fn runtime_qualification(&self) -> InventoryRuntimeQualification {
        InventoryRuntimeQualification::RuntimeUnqualified
    }

    pub const fn publication_status(&self) -> InventoryPublicationStatus {
        InventoryPublicationStatus::NotSupported
    }

    pub fn modules(&self) -> &[String] {
        &self.wire.inventory.modules
    }

    pub fn relative_paths(&self) -> &[String] {
        &self.wire.inventory.relative_paths
    }

    pub fn symbols(&self) -> &[String] {
        &self.wire.inventory.symbols
    }

    /// Move the verified collision domains into another closed in-crate capability without
    /// cloning their string allocations.
    pub(crate) fn into_collision_domains(self) -> (Vec<String>, Vec<String>, Vec<String>) {
        (
            self.wire.inventory.modules,
            self.wire.inventory.relative_paths,
            self.wire.inventory.symbols,
        )
    }
}

/// Collect and seal an inventory from exact Shipping and Binds cache bytes.
///
/// `catalog` is the closed, fully verified generation/catalog capability. The resulting artifact
/// binds both its exact catalog seal and the catalog's executable identity, not merely caller-
/// supplied cache checksums. This function verifies that both supplied buffers are within hard
/// source limits and exactly match the catalog's cache seals before invoking the parser. It
/// performs no filesystem access, game launch, compilation, deployment, or publishing.
pub fn build_base_game_inventory(
    catalog: &StoryCatalogFile,
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<BaseGameCollisionInventory, StoryInventoryError> {
    validate_catalog_capability(catalog)?;
    let generation = catalog.generation().clone();
    verify_source_buffers(&generation, shipping_cache, binds_cache, None)?;
    let collected = collect_collision_inventory(
        shipping_cache,
        binds_cache,
        *generation.shipping_cache.sha256.as_bytes(),
        *generation.binds_cache.sha256.as_bytes(),
    )?;
    build_from_collected(
        generation,
        catalog.catalog_seal().clone(),
        shipping_cache,
        binds_cache,
        collected,
    )
}

/// Reopen an artifact only after canonical, payload-seal, source-seal, and recollection checks.
///
/// Supplying only JSON is intentionally insufficient: an attacker can recompute a plain checksum
/// after changing a collision set. Recollection from the exact sealed sources closes that gap.
pub fn reopen_base_game_inventory(
    catalog: &StoryCatalogFile,
    json: &[u8],
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<BaseGameCollisionInventory, StoryInventoryError> {
    validate_catalog_capability(catalog)?;
    let artifact = parse_structural(json)?;
    verify_catalog_binding(&artifact.wire.inventory, catalog)?;
    verify_source_buffers(
        &artifact.wire.inventory.generation,
        shipping_cache,
        binds_cache,
        Some(&artifact.wire.inventory.source.source_pair_seal),
    )?;
    let generation = &artifact.wire.inventory.generation;
    let collected = collect_collision_inventory(
        shipping_cache,
        binds_cache,
        *generation.shipping_cache.sha256.as_bytes(),
        *generation.binds_cache.sha256.as_bytes(),
    )?;
    verify_collected_matches(&artifact.wire.inventory, &collected)?;
    Ok(artifact)
}

fn build_from_collected(
    generation: GameGenerationSeal,
    story_catalog_seal: ContentSeal,
    shipping_cache: &[u8],
    binds_cache: &[u8],
    collected: CollisionInventory,
) -> Result<BaseGameCollisionInventory, StoryInventoryError> {
    let source_pair_seal = seal_source_pair(shipping_cache, binds_cache)?;
    let payload = InventoryPayload {
        source: InventorySource {
            shipping_cache: generation.shipping_cache.clone(),
            binds_cache: generation.binds_cache.clone(),
            source_pair_seal,
        },
        generation,
        story_catalog_seal,
        catalog_layer: BaseGameLayer,
        coverage: InventoryCoverage::BaseGameOnly,
        runtime_qualification: InventoryRuntimeQualification::RuntimeUnqualified,
        publication_status: InventoryPublicationStatus::NotSupported,
        modules: collected.modules.into_iter().collect(),
        relative_paths: collected.relative_paths.into_iter().collect(),
        symbols: collected.symbols.into_iter().collect(),
    };
    validate_payload(&payload)?;
    let payload_seal = seal_bytes(&canonical_json(&payload, "collision inventory payload")?);
    let artifact = BaseGameCollisionInventory {
        wire: InventoryWire {
            format: InventoryFormat,
            schema_revision: InventorySchemaRevision,
            inventory: payload,
            payload_seal,
        },
    };
    validate_wire(&artifact.wire)?;
    Ok(artifact)
}

fn parse_structural(json: &[u8]) -> Result<BaseGameCollisionInventory, StoryInventoryError> {
    if json.len() > MAX_INVENTORY_JSON_BYTES {
        return Err(StoryInventoryError::LimitExceeded {
            kind: "collision inventory JSON bytes",
            actual: json.len(),
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    let wire: InventoryWire =
        serde_json::from_slice(json).map_err(StoryInventoryError::InvalidJson)?;
    let canonical = canonical_json(&wire, "collision inventory artifact")?;
    if canonical != json {
        return Err(StoryInventoryError::NonCanonicalJson);
    }
    validate_wire(&wire)?;
    Ok(BaseGameCollisionInventory { wire })
}

fn validate_wire(wire: &InventoryWire) -> Result<(), StoryInventoryError> {
    validate_payload(&wire.inventory)?;
    let payload = canonical_json(&wire.inventory, "collision inventory payload")?;
    let actual = seal_bytes(&payload);
    if actual != wire.payload_seal {
        return Err(StoryInventoryError::PayloadSealMismatch);
    }
    Ok(())
}

fn validate_payload(payload: &InventoryPayload) -> Result<(), StoryInventoryError> {
    validate_generation(&payload.generation)?;
    if payload.generation != known_generation_v1() {
        return Err(StoryInventoryError::UnsupportedGeneration);
    }
    validate_nonzero_seal("story catalog", &payload.story_catalog_seal)?;
    if payload.source.shipping_cache != payload.generation.shipping_cache
        || payload.source.binds_cache != payload.generation.binds_cache
    {
        return Err(StoryInventoryError::Invariant(
            "source cache seals disagree with the generation description".into(),
        ));
    }
    validate_nonzero_seal("source pair", &payload.source.source_pair_seal)?;
    let count = payload
        .modules
        .len()
        .checked_add(payload.relative_paths.len())
        .and_then(|value| value.checked_add(payload.symbols.len()))
        .unwrap_or(usize::MAX);
    if count > MAX_COLLISION_ENTRIES {
        return Err(StoryInventoryError::LimitExceeded {
            kind: "collision inventory entries",
            actual: count,
            max: MAX_COLLISION_ENTRIES,
        });
    }
    let mut aggregate = 0usize;
    for (kind, entries) in [
        ("module", &payload.modules),
        ("relative path", &payload.relative_paths),
        ("symbol", &payload.symbols),
    ] {
        if !strictly_sorted(entries) {
            return Err(StoryInventoryError::Invariant(format!(
                "{kind} entries are not in strict canonical order"
            )));
        }
        for entry in entries.iter() {
            validate_entry(kind, entry)?;
            aggregate =
                aggregate
                    .checked_add(entry.len())
                    .ok_or(StoryInventoryError::LimitExceeded {
                        kind: "aggregate collision entry bytes",
                        actual: usize::MAX,
                        max: MAX_COLLISION_TOTAL_BYTES,
                    })?;
            if aggregate > MAX_COLLISION_TOTAL_BYTES {
                return Err(StoryInventoryError::LimitExceeded {
                    kind: "aggregate collision entry bytes",
                    actual: aggregate,
                    max: MAX_COLLISION_TOTAL_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn validate_generation(generation: &GameGenerationSeal) -> Result<(), StoryInventoryError> {
    if generation.edition.is_empty()
        || generation.edition.len() > MAX_GENERATION_EDITION_BYTES
        || generation.edition.chars().any(char::is_control)
    {
        return Err(StoryInventoryError::Invariant(
            "generation edition is empty, oversized, or contains controls".into(),
        ));
    }
    validate_nonzero_seal("executable", &generation.executable)?;
    validate_nonzero_seal("Shipping cache", &generation.shipping_cache)?;
    validate_nonzero_seal("Binds cache", &generation.binds_cache)
}

fn validate_catalog_capability(catalog: &StoryCatalogFile) -> Result<(), StoryInventoryError> {
    // StoryCatalogFile is closed, but revalidate its complete compiled catalog equivalence at the
    // capability boundary rather than trusting that a value has not been corrupted in-process.
    catalog.to_canonical_json()?;
    if catalog.generation() != &known_generation_v1() {
        return Err(StoryInventoryError::UnsupportedGeneration);
    }
    Ok(())
}

fn verify_catalog_binding(
    payload: &InventoryPayload,
    catalog: &StoryCatalogFile,
) -> Result<(), StoryInventoryError> {
    if &payload.generation != catalog.generation()
        || &payload.story_catalog_seal != catalog.catalog_seal()
    {
        return Err(StoryInventoryError::CatalogBindingMismatch);
    }
    Ok(())
}

fn validate_nonzero_seal(
    kind: &'static str,
    seal: &ContentSeal,
) -> Result<(), StoryInventoryError> {
    if seal.byte_len == 0 {
        return Err(StoryInventoryError::Invariant(format!(
            "{kind} seal has a zero byte length"
        )));
    }
    Ok(())
}

fn verify_source_buffers(
    generation: &GameGenerationSeal,
    shipping_cache: &[u8],
    binds_cache: &[u8],
    expected_pair: Option<&ContentSeal>,
) -> Result<(), StoryInventoryError> {
    enforce_source_byte_limits(shipping_cache.len(), binds_cache.len())?;
    verify_buffer("Shipping cache", shipping_cache, &generation.shipping_cache)?;
    verify_buffer("Binds cache", binds_cache, &generation.binds_cache)?;
    if let Some(expected) = expected_pair {
        let actual = seal_source_pair(shipping_cache, binds_cache)?;
        if &actual != expected {
            return Err(StoryInventoryError::SourcePairSealMismatch);
        }
    }
    Ok(())
}

fn enforce_source_byte_limits(
    shipping_cache_bytes: usize,
    binds_cache_bytes: usize,
) -> Result<(), StoryInventoryError> {
    if shipping_cache_bytes > MAX_SHIPPING_CACHE_SOURCE_BYTES {
        return Err(StoryInventoryError::LimitExceeded {
            kind: "Shipping cache source bytes",
            actual: shipping_cache_bytes,
            max: MAX_SHIPPING_CACHE_SOURCE_BYTES,
        });
    }
    if binds_cache_bytes > MAX_BINDS_CACHE_SOURCE_BYTES {
        return Err(StoryInventoryError::LimitExceeded {
            kind: "Binds cache source bytes",
            actual: binds_cache_bytes,
            max: MAX_BINDS_CACHE_SOURCE_BYTES,
        });
    }
    Ok(())
}

fn verify_buffer(
    kind: &'static str,
    bytes: &[u8],
    expected: &ContentSeal,
) -> Result<(), StoryInventoryError> {
    let actual_len =
        u64::try_from(bytes.len()).map_err(|_| StoryInventoryError::LimitExceeded {
            kind,
            actual: usize::MAX,
            max: usize::MAX,
        })?;
    if actual_len != expected.byte_len {
        return Err(StoryInventoryError::SourceLengthMismatch {
            kind,
            expected: expected.byte_len,
            actual: actual_len,
        });
    }
    if Sha256::digest(bytes).as_slice() != expected.sha256.as_bytes() {
        return Err(StoryInventoryError::SourceDigestMismatch { kind });
    }
    Ok(())
}

fn verify_collected_matches(
    payload: &InventoryPayload,
    collected: &CollisionInventory,
) -> Result<(), StoryInventoryError> {
    let matches = payload.modules.iter().eq(collected.modules.iter())
        && payload
            .relative_paths
            .iter()
            .eq(collected.relative_paths.iter())
        && payload.symbols.iter().eq(collected.symbols.iter());
    if !matches {
        return Err(StoryInventoryError::RecollectedInventoryMismatch);
    }
    Ok(())
}

fn seal_source_pair(
    shipping_cache: &[u8],
    binds_cache: &[u8],
) -> Result<ContentSeal, StoryInventoryError> {
    enforce_source_byte_limits(shipping_cache.len(), binds_cache.len())?;
    let byte_len = shipping_cache
        .len()
        .checked_add(binds_cache.len())
        .ok_or(StoryInventoryError::SourcePairTooLarge)?;
    let byte_len = u64::try_from(byte_len).map_err(|_| StoryInventoryError::SourcePairTooLarge)?;
    let mut hasher = Sha256::new();
    hasher.update(SOURCE_PAIR_DOMAIN);
    hasher.update((shipping_cache.len() as u64).to_be_bytes());
    hasher.update(shipping_cache);
    hasher.update((binds_cache.len() as u64).to_be_bytes());
    hasher.update(binds_cache);
    Ok(ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes(hasher.finalize().into()),
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
) -> Result<Vec<u8>, StoryInventoryError> {
    let bytes = serde_json::to_vec(value).map_err(StoryInventoryError::SerializeJson)?;
    if bytes.len() > MAX_INVENTORY_JSON_BYTES {
        return Err(StoryInventoryError::LimitExceeded {
            kind,
            actual: bytes.len(),
            max: MAX_INVENTORY_JSON_BYTES,
        });
    }
    Ok(bytes)
}

fn strictly_sorted(entries: &[String]) -> bool {
    entries.windows(2).all(|pair| pair[0] < pair[1])
}

fn validate_entry(kind: &'static str, value: &str) -> Result<(), StoryInventoryError> {
    if value.is_empty()
        || value.len() > MAX_COLLISION_ENTRY_BYTES
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(StoryInventoryError::InvalidEntry {
            kind,
            value: value.into(),
        });
    }
    Ok(())
}

#[derive(Debug)]
struct BoundedString<const MAX: usize>(String);

impl<'de, const MAX: usize> Deserialize<'de> for BoundedString<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor<const MAX: usize>;
        impl<const MAX: usize> Visitor<'_> for StringVisitor<MAX> {
            type Value = BoundedString<MAX>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "a UTF-8 string of at most {MAX} bytes")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedString(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedString(value))
            }
        }
        deserializer.deserialize_string(StringVisitor::<MAX>)
    }
}

fn deserialize_collision_entries<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EntriesVisitor(PhantomData<Vec<String>>);
    impl<'de> Visitor<'de> for EntriesVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                formatter,
                "at most {MAX_COLLISION_ENTRIES} bounded collision entries"
            )
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let hint = sequence.size_hint().unwrap_or(0);
            if hint > MAX_COLLISION_ENTRIES {
                return Err(de::Error::invalid_length(hint, &self));
            }
            let mut entries = Vec::with_capacity(hint.min(MAX_COLLISION_ENTRIES));
            while entries.len() < MAX_COLLISION_ENTRIES {
                match sequence.next_element::<BoundedString<MAX_COLLISION_ENTRY_BYTES>>()? {
                    Some(value) => entries.push(value.0),
                    None => return Ok(entries),
                }
            }
            if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_COLLISION_ENTRIES + 1, &self));
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_seq(EntriesVisitor(PhantomData))
}

#[derive(Debug, thiserror::Error)]
pub enum StoryInventoryError {
    #[error("collision inventory resource limit exceeded for {kind}: {actual} > {max}")]
    LimitExceeded {
        kind: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("invalid collision inventory JSON: {0}")]
    InvalidJson(serde_json::Error),
    #[error("could not serialize canonical collision inventory JSON: {0}")]
    SerializeJson(serde_json::Error),
    #[error("collision inventory JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("collision inventory payload seal does not match its canonical payload")]
    PayloadSealMismatch,
    #[error("collision inventory does not target the compiled known game generation")]
    UnsupportedGeneration,
    #[error("collision inventory generation or catalog seal disagrees with the trusted catalog")]
    CatalogBindingMismatch,
    #[error("{kind} byte length mismatch: expected {expected}, got {actual}")]
    SourceLengthMismatch {
        kind: &'static str,
        expected: u64,
        actual: u64,
    },
    #[error("{kind} digest does not match its generation seal")]
    SourceDigestMismatch { kind: &'static str },
    #[error("combined Shipping/Binds source seal mismatch")]
    SourcePairSealMismatch,
    #[error("combined Shipping/Binds source length overflow")]
    SourcePairTooLarge,
    #[error("recollected source inventory does not match the sealed artifact")]
    RecollectedInventoryMismatch,
    #[error("invalid {kind} collision entry {value:?}")]
    InvalidEntry { kind: &'static str, value: String },
    #[error("invalid collision inventory invariant: {0}")]
    Invariant(String),
    #[error(transparent)]
    Collector(#[from] CollisionInventoryError),
    #[error(transparent)]
    Catalog(#[from] gore_story_catalog::CatalogError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_as::cache::header::CACHE_MAGIC;
    use gore_authoring::model_revision2::{
        Entity as AuthoringEntity, EntityKind as AuthoringEntityKind,
        EntityPayload as AuthoringEntityPayload, LocalizationEntry as AuthoringLocalizationEntry,
        NpcDraft as AuthoringNpcDraft, NpcDraftInput as AuthoringNpcDraftInput,
        NpcParentClassInput, OriginRef, ProjectRevision2, SchemaRevisionV2, TypedRef,
    };
    use gore_authoring::{
        migrate_revision2_to_revision3, AssetMeta, AssetStoreIndex, AssetVerification,
        ContentSeal as AuthoringContentSeal, EntityId as AuthoringEntityId, FormatV2,
        GameGenerationAnchor, ProjectId, ProjectMeta, QuestCollisionArtifactRef,
        QuestCollisionCatalogInput, QuestTransitionPlanV1, Revision3Entity, Revision3EntityKind,
        Revision3EntityPayload, Revision3OriginRef, Revision3QuestDraft, Revision3QuestDraftInput,
        Revision3TypedRef, Sha256Digest as AuthoringSha256Digest, WorkingHead, WorkingProjectStore,
        WorkingStoreError, WorkingStoreLimits, LOGICAL_NPC_CLONE_GENERATOR_ID,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_PROJECT_JSON_BYTES,
        MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE,
        QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, QUEST_COLLISION_CATALOG_LAYER,
        QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION,
        REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
        REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
    };
    use sha2::Sha256;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::revision3_quest_s3_test_subject::{
        prepare_revision3_quest_source_inspection, regenerate_revision3_quest_module,
        QuestInspectionBuildStatus, QuestInspectionPublicationStatus,
        QuestInspectionRuntimeQualification, QuestInspectionScope, Revision3QuestInspectionError,
        Revision3QuestSourceInspectionPlanV2,
    };

    static S3_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct S3TestRoot(PathBuf);

    impl S3TestRoot {
        fn new(label: &str) -> Self {
            let sequence = S3_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gore-story-inventory-s3-{label}-{}-{sequence}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for S3TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn seal(bytes: &[u8]) -> ContentSeal {
        seal_bytes(bytes)
    }

    fn generation(shipping: &[u8], binds: &[u8]) -> GameGenerationSeal {
        GameGenerationSeal {
            edition: "synthetic-g1r-test".into(),
            executable: seal(b"exe"),
            shipping_cache: seal(shipping),
            binds_cache: seal(binds),
        }
    }

    fn trusted_catalog() -> StoryCatalogFile {
        let fixture = include_bytes!("../../gore-ffi/tests/fixtures/story_catalog_v1.json");
        StoryCatalogFile::from_json(fixture.strip_suffix(b"\n").unwrap_or(fixture)).unwrap()
    }

    fn collected() -> CollisionInventory {
        CollisionInventory {
            modules: BTreeSet::from(["gore.alpha".into(), "gore.zeta".into()]),
            relative_paths: BTreeSet::from(["gore/alpha.as".into(), "gore/zeta.as".into()]),
            symbols: BTreeSet::from(["ualpha".into(), "uzeta".into()]),
        }
    }

    fn artifact() -> BaseGameCollisionInventory {
        let shipping = b"synthetic shipping";
        let binds = b"synthetic binds";
        let catalog = trusted_catalog();
        build_from_collected(
            catalog.generation().clone(),
            catalog.catalog_seal().clone(),
            shipping,
            binds,
            collected(),
        )
        .unwrap()
    }

    fn authoring_seal(seal: &ContentSeal) -> AuthoringContentSeal {
        AuthoringContentSeal {
            byte_len: seal.byte_len,
            sha256: AuthoringSha256Digest::from_bytes(*seal.sha256.as_bytes()),
        }
    }

    fn authoring_target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: authoring_seal(&trusted_catalog().generation().executable),
        }
    }

    fn authoring_project_id(value: u8) -> ProjectId {
        ProjectId::from_bytes([value; 16])
    }

    fn authoring_entity_id(value: u8) -> AuthoringEntityId {
        AuthoringEntityId::from_bytes([value; 16])
    }

    fn empty_authoring_project() -> ProjectRevision2 {
        ProjectRevision2 {
            format: FormatV2,
            schema_revision: SchemaRevisionV2,
            project_id: authoring_project_id(1),
            revision: 4,
            meta: ProjectMeta {
                name: "verified collision capability".into(),
                version: "0.1.0".into(),
                author: "test".into(),
            },
            target: authoring_target(),
            authoring_locales: BTreeSet::new(),
            entities: Default::default(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn npc_parent(
        target: &GameGenerationAnchor,
        seal_value: u8,
        runtime_class: &str,
    ) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: target.clone(),
            source_seal: AuthoringContentSeal {
                byte_len: 100,
                sha256: AuthoringSha256Digest::from_bytes([seal_value; 32]),
            },
            catalog_layer: "base-game.test.characters".into(),
            canonical_selector: format!("Catalog{runtime_class}"),
            runtime_class: runtime_class.into(),
        }
    }

    fn add_authoring_npc(
        project: &mut ProjectRevision2,
        module_namespace: &str,
        unique_name: &str,
    ) {
        let owner_id = authoring_entity_id(10);
        let module_id = authoring_entity_id(11);
        let owner = TypedRef::new(project.project_id, owner_id, AuthoringEntityKind::NpcDraft);
        let draft = AuthoringNpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.into(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: AuthoringNpcDraftInput {
                target: project.target.clone(),
                module_namespace: module_namespace.into(),
                unique_name: unique_name.into(),
                parent_character_definition: npc_parent(
                    &project.target,
                    10,
                    "UCharacterDefinition_Human_Base",
                ),
                parent_ai_agent_config: npc_parent(
                    &project.target,
                    11,
                    "UAIAgentConfig_Human_Base",
                ),
                parent_spawn_definition: npc_parent(
                    &project.target,
                    12,
                    "USpawnAIAgentDefinition_Base",
                ),
            },
            script_module: TypedRef::new(
                project.project_id,
                module_id,
                AuthoringEntityKind::ScriptModule,
            ),
        };
        let script = draft.regenerate_script_module(owner.clone()).unwrap();
        project.entities.insert(
            owner_id,
            AuthoringEntity {
                id: owner_id,
                display_name: unique_name.into(),
                origin: OriginRef::New {
                    authored_runtime_id: unique_name.into(),
                },
                revision: 0,
                payload: AuthoringEntityPayload::NpcDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            AuthoringEntity {
                id: module_id,
                display_name: format!("{unique_name} script"),
                origin: OriginRef::Generated {
                    generator_id: script.generator_id.clone(),
                    generator_version: script.generator_version,
                    owner,
                },
                revision: 0,
                payload: AuthoringEntityPayload::ScriptModule(script),
            },
        );
    }

    fn artifact_with_extra_collision(kind: &str, value: &str) -> BaseGameCollisionInventory {
        let mut base = artifact();
        let domain = match kind {
            "module" => &mut base.wire.inventory.modules,
            "relative path" => &mut base.wire.inventory.relative_paths,
            "symbol" => &mut base.wire.inventory.symbols,
            _ => panic!("unsupported collision domain"),
        };
        domain.push(value.into());
        domain.sort();
        let payload = canonical_json(&base.wire.inventory, "test payload").unwrap();
        base.wire.payload_seal = seal_bytes(&payload);
        validate_wire(&base.wire).unwrap();
        base
    }

    fn artifact_with_module_collisions(values: Vec<String>) -> BaseGameCollisionInventory {
        let mut base = artifact();
        base.wire.inventory.modules = values;
        base.wire.inventory.relative_paths.clear();
        base.wire.inventory.symbols.clear();
        let payload =
            canonical_json(&base.wire.inventory, "test module collision payload").unwrap();
        base.wire.payload_seal = seal_bytes(&payload);
        validate_wire(&base.wire).unwrap();
        base
    }

    fn distinct_collision_values(count: usize) -> Vec<String> {
        (0..count)
            .map(|index| format!("budget.m{index:06}"))
            .collect()
    }

    fn distinct_collision_values_with_exact_bytes(total_bytes: usize) -> Vec<String> {
        const PREFIX_BYTES: usize = 14; // `budget.m` plus six decimal digits.
        assert!(total_bytes >= PREFIX_BYTES);
        let count = total_bytes.div_ceil(MAX_COLLISION_ENTRY_BYTES);
        assert!(count <= MAX_COLLISION_ENTRIES);
        let mut remaining = total_bytes;
        let mut values = Vec::with_capacity(count);
        for index in 0..count {
            let entries_after = count - index - 1;
            let reserved_after = entries_after * PREFIX_BYTES;
            let len = (remaining - reserved_after).min(MAX_COLLISION_ENTRY_BYTES);
            let prefix = format!("budget.m{index:06}");
            assert_eq!(prefix.len(), PREFIX_BYTES);
            values.push(format!("{prefix}{}", "a".repeat(len - PREFIX_BYTES)));
            remaining -= len;
        }
        assert_eq!(remaining, 0);
        values
    }

    fn push_sia(output: &mut Vec<u8>, value: &str) {
        if value.is_empty() {
            output.extend_from_slice(&0i32.to_le_bytes());
        } else {
            output.extend_from_slice(&(value.len() as i32).to_le_bytes());
            output.extend_from_slice(value.as_bytes());
            output.push(0);
        }
    }

    fn push_fstring(output: &mut Vec<u8>, value: &str) {
        output.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        output.extend_from_slice(value.as_bytes());
        output.push(0);
    }

    fn minimal_cache() -> Vec<u8> {
        let mut output = vec![0u8; 16];
        output.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
        output.extend_from_slice(&1u32.to_le_bytes());
        push_fstring(&mut output, "Base.Module");
        push_sia(&mut output, "Base.Module");
        for _ in 0..5 {
            output.extend_from_slice(&0i32.to_le_bytes());
        }
        output.extend_from_slice(&0i64.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        push_sia(&mut output, "");
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        push_sia(&mut output, "Base/Module.as");
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&1i32.to_le_bytes());
        output.extend_from_slice(&101i64.to_le_bytes());
        push_sia(&mut output, "TailOnlyType");
        push_sia(&mut output, "");
        push_sia(&mut output, "");
        output.extend_from_slice(&0i32.to_le_bytes());
        for _ in 1..7 {
            output.extend_from_slice(&0i32.to_le_bytes());
        }
        output
    }

    fn push_cstr(output: &mut Vec<u8>, value: &str) {
        output.extend_from_slice(&((value.len() + 1) as u32).to_le_bytes());
        output.extend_from_slice(value.as_bytes());
        output.push(0);
    }

    fn minimal_binds() -> Vec<u8> {
        let mut output = 1u32.to_le_bytes().to_vec();
        push_cstr(&mut output, "UNativeType");
        push_cstr(&mut output, "/Script/Test.NativeType");
        output.extend_from_slice(&1u32.to_le_bytes());
        push_cstr(&mut output, "void NativeCall()");
        push_cstr(&mut output, "NativeCall");
        output.extend_from_slice(&[0u8; 32]);
        output
    }

    #[test]
    fn public_build_requires_a_trusted_catalog_and_its_exact_sealed_sources() {
        let shipping = minimal_cache();
        let binds = minimal_binds();
        let catalog = trusted_catalog();
        assert!(matches!(
            build_base_game_inventory(&catalog, &shipping, &binds),
            Err(StoryInventoryError::SourceLengthMismatch {
                kind: "Shipping cache",
                ..
            })
        ));
    }

    #[test]
    fn canonical_artifact_is_deterministic_closed_and_structurally_reopens() {
        let first = artifact();
        let second = artifact();
        let json = first.to_canonical_json().unwrap();
        assert_eq!(json, second.to_canonical_json().unwrap());
        assert_eq!(parse_structural(&json).unwrap(), first);
        assert_eq!(first.catalog_layer(), BASE_GAME_SCRIPT_INVENTORY_LAYER);
        assert_eq!(first.story_catalog_seal(), trusted_catalog().catalog_seal());
        assert_eq!(first.coverage(), InventoryCoverage::BaseGameOnly);
        assert_eq!(
            first.runtime_qualification(),
            InventoryRuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(
            first.publication_status(),
            InventoryPublicationStatus::NotSupported
        );
        assert_eq!(first.modules(), ["gore.alpha", "gore.zeta"]);
    }

    #[test]
    fn source_buffers_verify_individual_and_domain_separated_pair_seals() {
        let shipping = b"shipping";
        let binds = b"binds";
        let generation = generation(shipping, binds);
        let pair = seal_source_pair(shipping, binds).unwrap();
        verify_source_buffers(&generation, shipping, binds, Some(&pair)).unwrap();

        assert!(matches!(
            verify_source_buffers(&generation, b"short", binds, Some(&pair)),
            Err(StoryInventoryError::SourceLengthMismatch {
                kind: "Shipping cache",
                ..
            })
        ));
        let same_length_tamper = b"shippinh";
        assert!(matches!(
            verify_source_buffers(&generation, same_length_tamper, binds, Some(&pair)),
            Err(StoryInventoryError::SourceDigestMismatch {
                kind: "Shipping cache"
            })
        ));

        let wrong_pair = seal_source_pair(b"shippingbind", b"s").unwrap();
        assert_ne!(
            pair, wrong_pair,
            "length framing must distinguish source pairs"
        );
    }

    #[test]
    fn whitespace_unknown_duplicate_and_wrong_layer_fail_closed() {
        let json = String::from_utf8(artifact().to_canonical_json().unwrap()).unwrap();
        let spaced = format!(" {json}");
        assert!(matches!(
            parse_structural(spaced.as_bytes()),
            Err(StoryInventoryError::NonCanonicalJson)
        ));

        let unknown = json.replacen("{\"format\":", "{\"unknown\":true,\"format\":", 1);
        assert!(matches!(
            parse_structural(unknown.as_bytes()),
            Err(StoryInventoryError::InvalidJson(_))
        ));
        let duplicate = json.replacen(
            "{\"format\":",
            "{\"format\":\"story_script_collision_inventory\",\"format\":",
            1,
        );
        assert!(matches!(
            parse_structural(duplicate.as_bytes()),
            Err(StoryInventoryError::InvalidJson(_))
        ));
        let wrong_layer = json.replace(
            BASE_GAME_SCRIPT_INVENTORY_LAYER,
            "resolved-loadout.scripts.v1",
        );
        assert!(matches!(
            parse_structural(wrong_layer.as_bytes()),
            Err(StoryInventoryError::InvalidJson(_))
        ));
    }

    #[test]
    fn payload_tampering_fails_even_when_outer_json_is_canonical() {
        let json = artifact().to_canonical_json().unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        value["inventory"]["symbols"][0] = serde_json::Value::String("changed".into());
        let tampered = serde_json::to_vec(&value).unwrap();
        assert!(matches!(
            parse_structural(&tampered),
            Err(StoryInventoryError::PayloadSealMismatch)
                | Err(StoryInventoryError::NonCanonicalJson)
        ));
    }

    #[test]
    fn a_resealed_but_forged_inventory_still_disagrees_with_recollection() {
        let mut artifact = artifact();
        artifact.wire.inventory.symbols[0] = "forged".into();
        let payload = canonical_json(&artifact.wire.inventory, "test payload").unwrap();
        artifact.wire.payload_seal = seal_bytes(&payload);
        validate_wire(&artifact.wire).unwrap();
        assert!(matches!(
            verify_collected_matches(&artifact.wire.inventory, &collected()),
            Err(StoryInventoryError::RecollectedInventoryMismatch)
        ));
    }

    #[test]
    fn forged_generation_or_catalog_cannot_become_trusted_by_resealing() {
        let catalog = trusted_catalog();
        let shipping = b"synthetic shipping";
        let binds = b"synthetic binds";

        let mut forged_generation = artifact().wire;
        forged_generation.inventory.generation.executable = seal(b"attacker executable");
        let payload = canonical_json(&forged_generation.inventory, "test payload").unwrap();
        forged_generation.payload_seal = seal_bytes(&payload);
        let json = canonical_json(&forged_generation, "test artifact").unwrap();
        assert!(matches!(
            reopen_base_game_inventory(&catalog, &json, shipping, binds),
            Err(StoryInventoryError::UnsupportedGeneration)
        ));

        let mut forged_catalog = artifact().wire;
        forged_catalog.inventory.story_catalog_seal = seal(b"attacker catalog");
        let payload = canonical_json(&forged_catalog.inventory, "test payload").unwrap();
        forged_catalog.payload_seal = seal_bytes(&payload);
        let json = canonical_json(&forged_catalog, "test artifact").unwrap();
        assert!(matches!(
            reopen_base_game_inventory(&catalog, &json, shipping, binds),
            Err(StoryInventoryError::CatalogBindingMismatch)
        ));
    }

    #[test]
    fn source_limits_reject_oversize_before_seal_checks_or_collection() {
        assert!(matches!(
            enforce_source_byte_limits(MAX_SHIPPING_CACHE_SOURCE_BYTES + 1, 0),
            Err(StoryInventoryError::LimitExceeded {
                kind: "Shipping cache source bytes",
                actual,
                max: MAX_SHIPPING_CACHE_SOURCE_BYTES,
            }) if actual == MAX_SHIPPING_CACHE_SOURCE_BYTES + 1
        ));
        assert!(matches!(
            enforce_source_byte_limits(0, MAX_BINDS_CACHE_SOURCE_BYTES + 1),
            Err(StoryInventoryError::LimitExceeded {
                kind: "Binds cache source bytes",
                actual,
                max: MAX_BINDS_CACHE_SOURCE_BYTES,
            }) if actual == MAX_BINDS_CACHE_SOURCE_BYTES + 1
        ));
    }

    #[test]
    fn ordering_duplicates_entry_and_aggregate_bounds_fail_closed() {
        let mut wire = artifact().wire;
        wire.inventory.modules.swap(0, 1);
        assert!(matches!(
            validate_payload(&wire.inventory),
            Err(StoryInventoryError::Invariant(_))
        ));

        wire = artifact().wire;
        wire.inventory
            .modules
            .insert(1, wire.inventory.modules[0].clone());
        assert!(matches!(
            validate_payload(&wire.inventory),
            Err(StoryInventoryError::Invariant(_))
        ));

        wire = artifact().wire;
        wire.inventory.symbols = vec!["a".repeat(MAX_COLLISION_ENTRY_BYTES + 1)];
        assert!(matches!(
            validate_payload(&wire.inventory),
            Err(StoryInventoryError::InvalidEntry { .. })
        ));

        wire = artifact().wire;
        wire.inventory.symbols = (0..=MAX_COLLISION_ENTRIES)
            .map(|index| format!("symbol{index:06}"))
            .collect();
        assert!(matches!(
            validate_payload(&wire.inventory),
            Err(StoryInventoryError::LimitExceeded {
                kind: "collision inventory entries",
                ..
            })
        ));

        wire = artifact().wire;
        let chunk = "a".repeat(MAX_COLLISION_ENTRY_BYTES - 8);
        wire.inventory.symbols = (0..=(MAX_COLLISION_TOTAL_BYTES / chunk.len()))
            .map(|index| format!("{index:08}{chunk}"))
            .collect();
        assert!(matches!(
            validate_payload(&wire.inventory),
            Err(StoryInventoryError::LimitExceeded {
                kind: "aggregate collision entry bytes",
                ..
            })
        ));
    }

    #[test]
    fn raw_json_and_sequence_deserializers_reject_limits_before_trust() {
        let oversized = vec![b' '; MAX_INVENTORY_JSON_BYTES + 1];
        assert!(matches!(
            parse_structural(&oversized),
            Err(StoryInventoryError::LimitExceeded {
                kind: "collision inventory JSON bytes",
                ..
            })
        ));

        let long = "x".repeat(MAX_COLLISION_ENTRY_BYTES + 1);
        let error = serde_json::from_str::<InventoryPayload>(&format!(
            "{{\"generation\":{{\"edition\":\"x\",\"executable\":{{\"byte_len\":1,\"sha256\":\"{}\"}},\"shipping_cache\":{{\"byte_len\":1,\"sha256\":\"{}\"}},\"binds_cache\":{{\"byte_len\":1,\"sha256\":\"{}\"}}}},\"story_catalog_seal\":{{\"byte_len\":1,\"sha256\":\"{}\"}},\"catalog_layer\":\"{}\",\"coverage\":\"base_game_only\",\"runtime_qualification\":\"runtime_unqualified\",\"publication_status\":\"not_supported\",\"source\":{{\"shipping_cache\":{{\"byte_len\":1,\"sha256\":\"{}\"}},\"binds_cache\":{{\"byte_len\":1,\"sha256\":\"{}\"}},\"source_pair_seal\":{{\"byte_len\":2,\"sha256\":\"{}\"}}}},\"modules\":[\"{}\"],\"relative_paths\":[],\"symbols\":[]}}",
            "00".repeat(32),
            "00".repeat(32),
            "00".repeat(32),
            "00".repeat(32),
            BASE_GAME_SCRIPT_INVENTORY_LAYER,
            "00".repeat(32),
            "00".repeat(32),
            "00".repeat(32),
            long
        ))
        .unwrap_err();
        assert!(error.to_string().contains("invalid length"));
    }

    #[test]
    fn capability_is_deterministic_closed_and_honest_about_unsupported_layers() {
        let catalog = trusted_catalog();
        let project = empty_authoring_project();
        let first = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project).unwrap();
        let second =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.project_id(), project.project_id);
        assert_eq!(first.project_revision(), project.revision);
        assert_eq!(first.project_target(), &project.target);
        assert_eq!(first.canonical_project(), second.canonical_project());
        assert_eq!(first.combined_source_seal(), second.combined_source_seal());
        assert_eq!(
            first.coverage(),
            QuestCollisionCoverage::BaseGameAndExactProjectOnly
        );
        assert_eq!(
            first.runtime_qualification(),
            QuestCollisionRuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(first.build_status(), QuestCollisionBuildStatus::Blocked);
        assert_eq!(
            first.publication_status(),
            QuestCollisionPublicationStatus::NotSupported
        );
        assert_eq!(
            first.catalog_layer(),
            BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
        );
        assert!(!first.catalog_layer().contains("resolved-loadout"));
        assert!(first.contains_module("GORE.ALPHA"));
        assert!(first.contains_relative_path("Gore/Alpha.as"));
        assert!(first.contains_symbol("UAlpha"));

        let input = first.into_quest_collision_input(&project).unwrap();
        assert_eq!(
            input.catalog_layer,
            BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
        );
        assert_eq!(input.generation, project.target);
        assert!(input.modules.contains("gore.alpha"));
        assert!(input.relative_paths.contains("gore/alpha.as"));
        assert!(input.symbols.contains("ualpha"));
    }

    #[test]
    fn capability_unions_exact_project_regeneration_and_seals_revision() {
        let catalog = trusted_catalog();
        let empty = empty_authoring_project();
        let empty_capability =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &empty).unwrap();
        let mut project = empty.clone();
        add_authoring_npc(&mut project, "Project.Npcs.NewOne", "ProjectNpc");
        let capability =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project).unwrap();

        assert!(capability.contains_module("PROJECT.NPCS.NEWONE"));
        assert!(capability.contains_relative_path("Project/Npcs/NewOne.as"));
        assert!(capability.contains_symbol("UCharacterDefinition_Human_ProjectNpc"));
        assert!(capability.contains_symbol("UAIAgentConfig_Human_ProjectNpc"));
        assert!(capability.contains_symbol("USpawnAIAgentDefinition_ProjectNpc"));
        assert_ne!(
            capability.combined_source_seal(),
            empty_capability.combined_source_seal()
        );

        let mut next_revision = empty;
        next_revision.revision += 1;
        let next =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &next_revision).unwrap();
        assert_ne!(
            empty_capability.combined_source_seal(),
            next.combined_source_seal()
        );
    }

    #[test]
    fn parent_and_giver_resolution_comes_only_from_the_exact_catalog() {
        let catalog = trusted_catalog();
        let selections = catalog.authoring_selections().unwrap();
        let expected_parent = selections.quest_parents.first().unwrap();
        let expected_giver = selections.npcs.first().unwrap();
        let capability = VerifiedQuestCollisionCapability::bind(
            artifact(),
            &catalog,
            &empty_authoring_project(),
        )
        .unwrap();

        let parent = capability
            .resolve_parent(&expected_parent.catalog_id)
            .unwrap();
        assert!(capability.authorizes_parent(&parent));
        assert_eq!(
            parent.canonical_selector,
            expected_parent.quest_class.authoring_selector
        );
        assert_eq!(
            parent.runtime_class,
            expected_parent.quest_class.runtime_class
        );
        let giver = capability
            .resolve_giver(&expected_giver.catalog_id)
            .unwrap();
        assert!(capability.authorizes_giver(&giver));
        assert_eq!(
            giver.canonical_selector,
            expected_giver.quest_giver.authoring_selector
        );
        assert_eq!(
            giver.runtime_unique_name,
            expected_giver.quest_giver.runtime_unique_name
        );
        assert!(matches!(
            capability.resolve_parent("not-in-catalog"),
            Err(QuestCollisionCapabilityError::UnknownParent(_))
        ));
        assert!(matches!(
            capability.resolve_giver("not-in-catalog"),
            Err(QuestCollisionCapabilityError::UnknownGiver(_))
        ));
        let mut forged_parent = parent;
        forged_parent.runtime_class.push_str("_Forged");
        assert!(!capability.authorizes_parent(&forged_parent));
        let mut forged_giver = giver;
        forged_giver.runtime_unique_name.push_str("_Forged");
        assert!(!capability.authorizes_giver(&forged_giver));
    }

    #[test]
    fn base_project_collisions_fail_closed_in_every_domain() {
        let catalog = trusted_catalog();
        let mut project = empty_authoring_project();
        add_authoring_npc(&mut project, "Project.Npcs.Collision", "CollisionNpc");
        let cases = [
            ("module", "project.npcs.collision"),
            ("relative path", "project/npcs/collision.as"),
            ("symbol", "ucharacterdefinition_human_collisionnpc"),
        ];

        for (kind, value) in cases {
            let base = artifact_with_extra_collision(kind, value);
            assert!(matches!(
                VerifiedQuestCollisionCapability::bind(base, &catalog, &project),
                Err(QuestCollisionCapabilityError::BaseProjectCollision {
                    kind: actual_kind,
                    value: actual_value,
                    ..
                }) if actual_kind == kind && actual_value == value
            ));
        }
    }

    #[test]
    fn catalog_target_and_project_drift_are_rejected() {
        let catalog = trusted_catalog();
        let project = empty_authoring_project();

        let mut wrong_catalog_binding = artifact();
        wrong_catalog_binding.wire.inventory.story_catalog_seal = seal(b"other catalog");
        let payload =
            canonical_json(&wrong_catalog_binding.wire.inventory, "test payload").unwrap();
        wrong_catalog_binding.wire.payload_seal = seal_bytes(&payload);
        assert!(matches!(
            VerifiedQuestCollisionCapability::bind(wrong_catalog_binding, &catalog, &project),
            Err(QuestCollisionCapabilityError::CatalogBindingMismatch)
        ));

        let mut wrong_target = project.clone();
        wrong_target.target.executable.sha256 = AuthoringSha256Digest::from_bytes([0x99; 32]);
        assert!(matches!(
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &wrong_target),
            Err(QuestCollisionCapabilityError::TargetMismatch)
        ));

        let capability =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project).unwrap();
        let mut changed_head = project;
        changed_head.revision += 1;
        assert!(matches!(
            capability.into_quest_collision_input(&changed_head),
            Err(QuestCollisionCapabilityError::ProjectDrift)
        ));
    }

    #[test]
    fn source_bound_bridge_returns_the_capability_needed_by_the_quest_generator() {
        let catalog = trusted_catalog();
        let project = empty_authoring_project();
        let stored = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project)
            .unwrap()
            .into_artifact()
            .unwrap();
        let reopened = reopen_quest_collision_capability_artifact_v1(
            stored.canonical_json(),
            stored.artifact_seal(),
            stored.source_seal(),
        )
        .unwrap();

        let authoritative = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project)
            .unwrap()
            .verify_artifact_exact(&reopened)
            .unwrap();
        let input = authoritative.into_quest_collision_input(&project).unwrap();
        assert_eq!(
            input.catalog_layer,
            BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
        );
        assert_eq!(input.source_seal.byte_len, reopened.source_seal().byte_len);
        assert!(!input.modules.is_empty());
        assert!(!input.relative_paths.is_empty());
        assert!(!input.symbols.is_empty());
    }

    #[test]
    fn prepared_capsule_is_byte_exact_with_the_legacy_api_and_finalizes_once() {
        let catalog = trusted_catalog();
        let project = empty_authoring_project();
        let legacy = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project)
            .unwrap()
            .into_artifact()
            .unwrap();
        let prepared = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project)
            .unwrap()
            .prepare_artifact()
            .unwrap();

        assert_eq!(prepared.artifact(), &legacy);
        assert_eq!(
            prepared.artifact().canonical_json(),
            legacy.canonical_json()
        );
        let (materialized, input) = prepared.finalize(&project).unwrap();

        assert_eq!(materialized, legacy);
        assert_eq!(
            input.catalog_layer,
            BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER
        );
        assert_eq!(
            input.source_seal.byte_len,
            materialized.source_seal().byte_len
        );
        assert!(!input.modules.is_empty());
        assert!(!input.relative_paths.is_empty());
        assert!(!input.symbols.is_empty());
    }

    #[test]
    fn prepared_finalize_consumes_and_rejects_every_exact_project_head_drift_gate() {
        let catalog = trusted_catalog();
        let project = empty_authoring_project();

        let mut changed_id = project.clone();
        changed_id.project_id = authoring_project_id(2);
        let mut changed_revision = project.clone();
        changed_revision.revision += 1;
        let mut changed_target = project.clone();
        changed_target.target.executable.sha256 = AuthoringSha256Digest::from_bytes([0xa3; 32]);
        let mut changed_canonical_snapshot = project.clone();
        changed_canonical_snapshot.meta.name.push_str(" drift");

        for changed_head in [
            changed_id,
            changed_revision,
            changed_target,
            changed_canonical_snapshot,
        ] {
            let prepared = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &project)
                .unwrap()
                .prepare_artifact()
                .unwrap();
            let error = prepared.finalize(&changed_head).unwrap_err();
            assert!(matches!(
                error,
                PreparedQuestCollisionArtifactFinalizeError::Project(
                    QuestCollisionCapabilityError::ProjectDrift
                )
            ));
            // `finalize` takes the capsule by value, so this failure path cannot be retried or
            // converted back into authority with the now-rejected prepared value.
        }
    }

    struct S3Fixture {
        _root: S3TestRoot,
        store: WorkingProjectStore,
        catalog: StoryCatalogFile,
        collision_source: ProjectRevision2,
        project: gore_authoring::ProjectRevision3,
        quest_id: AuthoringEntityId,
    }

    impl S3Fixture {
        fn fresh_capability(&self) -> VerifiedQuestCollisionCapability {
            VerifiedQuestCollisionCapability::bind(
                artifact(),
                &self.catalog,
                &self.collision_source,
            )
            .unwrap()
        }

        fn canonical(&self, project: &gore_authoring::ProjectRevision3) -> String {
            project.to_canonical_json().unwrap()
        }
    }

    fn s3_fixture(label: &str) -> S3Fixture {
        let root = S3TestRoot::new(label);
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let collision_source = empty_authoring_project();
        let basis = migrate_revision2_to_revision3(&collision_source)
            .unwrap()
            .project;
        let basis_checkpoint = store.prepare_revision3_checkpoint(None, &basis).unwrap();

        let collision_artifact =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &collision_source)
                .unwrap()
                .into_artifact()
                .unwrap();
        let imported = store
            .import_quest_collision_artifact_v1(collision_artifact.canonical_json(), None)
            .unwrap();
        assert_eq!(
            imported.artifact,
            authoring_seal(collision_artifact.artifact_seal())
        );

        let selection =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &collision_source)
                .unwrap();
        let parent = selection
            .resolve_parent("g1r:quest-parent:swampcamp_scchapter2")
            .unwrap();
        let giver = selection
            .resolve_giver("g1r:npc:om_grd_asghan_263")
            .unwrap();

        let quest_id = authoring_entity_id(0x71);
        let module_id = authoring_entity_id(0x72);
        let mut project = basis;
        project.revision += 1;
        project.asset_store.assets.insert(
            imported.artifact.sha256,
            AssetMeta {
                byte_len: imported.asset_meta.byte_len,
                media_type: imported.asset_meta.media_type,
            },
        );
        let quest = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: project.target.clone(),
                quest_id,
                module_namespace: "GoreMods.Quests.S3AsghanTrial".to_owned(),
                technical_id: "GORE_S3_ASGHAN_TRIAL".to_owned(),
                text_helper: "GoreS3QuestText".to_owned(),
                parent_quest: parent,
                giver,
                title: "Asghan Trial".to_owned(),
                description: "Prove that the gate is secure.".to_owned(),
                objective_title: "Report to Asghan".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: None,
                collision_catalog: QuestCollisionArtifactRef {
                    generation: project.target.clone(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
                    artifact: imported.artifact,
                    source_seal: authoring_seal(collision_artifact.source_seal()),
                    basis_snapshot: basis_checkpoint.head.snapshot,
                },
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
        };
        let collision_input =
            VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &collision_source)
                .unwrap()
                .into_quest_collision_input(&collision_source)
                .unwrap();
        let module = regenerate_revision3_quest_module(&quest, collision_input).unwrap();
        let owner = Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        );
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "S3 Asghan Trial".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: quest.input.technical_id.clone(),
                },
                revision: 0,
                payload: Revision3EntityPayload::QuestDraft(quest),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "S3 Asghan Trial source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                },
                revision: 0,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        );
        project.validate_closed_model().unwrap();

        S3Fixture {
            _root: root,
            store,
            catalog,
            collision_source,
            project,
            quest_id,
        }
    }

    fn s3_quest_mut(
        project: &mut gore_authoring::ProjectRevision3,
        quest_id: AuthoringEntityId,
    ) -> &mut Revision3QuestDraft {
        let Revision3EntityPayload::QuestDraft(quest) =
            &mut project.entities.get_mut(&quest_id).unwrap().payload
        else {
            panic!("expected S3 Quest Draft")
        };
        quest
    }

    fn publish_revision3_head(
        root: &S3TestRoot,
        store: &WorkingProjectStore,
        project: &gore_authoring::ProjectRevision3,
    ) -> WorkingHead {
        let prepared = store.prepare_revision3_checkpoint(None, project).unwrap();
        fs::write(root.0.join("gore-project.json"), &prepared.head_bytes).unwrap();
        prepared.head
    }

    fn advance_revision3_head(
        root: &S3TestRoot,
        store: &WorkingProjectStore,
        expected_head: &WorkingHead,
        project: &gore_authoring::ProjectRevision3,
    ) -> WorkingHead {
        let prepared = store
            .prepare_revision3_checkpoint(Some(expected_head), project)
            .unwrap();
        fs::write(root.0.join("gore-project.json"), &prepared.head_bytes).unwrap();
        prepared.head
    }

    fn prepare_v3_transaction_artifact(
        store: &WorkingProjectStore,
        head: &WorkingHead,
        catalog: &StoryCatalogFile,
    ) -> PreparedQuestCollisionArtifactV2 {
        VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(head)
                .unwrap(),
        )
        .unwrap()
        .prepare_artifact()
        .unwrap()
    }

    fn request_v3(
        project: &gore_authoring::ProjectRevision3,
        head: &WorkingHead,
        ordinal: u8,
    ) -> Revision3QuestDraftInsertRequestV3 {
        Revision3QuestDraftInsertRequestV3 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            quest_id: authoring_entity_id(0x80 + ordinal * 2),
            script_module_id: authoring_entity_id(0x81 + ordinal * 2),
            display_name: format!("Authority-sensitive Quest {ordinal}"),
            intent: Revision3QuestDraftIntentV3 {
                module_namespace: format!("GoreMods.Quests.AuthorityQuest{ordinal}"),
                technical_id: format!("GORE_AUTHORITY_QUEST_{ordinal}"),
                text_helper: format!("GoreAuthorityQuest{ordinal}Text"),
                parent_catalog_id: "g1r:quest-parent:swampcamp_scchapter2".to_owned(),
                giver_catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
                title: format!("Authority Quest {ordinal}"),
                description: "Exercise the exact-current multi-Quest transaction.".to_owned(),
                objective_title: format!("Finish authority Quest {ordinal}"),
                additional_objective_titles: Vec::new(),
            },
        }
    }

    fn context_edit_request_v1(
        project: &gore_authoring::ProjectRevision3,
        head: &WorkingHead,
        catalog: &StoryCatalogFile,
        quest_id: AuthoringEntityId,
    ) -> Revision3QuestContextEditRequestV1 {
        let entity = &project.entities[&quest_id];
        let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
            panic!("expected context-edit Quest Draft")
        };
        Revision3QuestContextEditRequestV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_story_catalog_seal: catalog.catalog_seal().clone(),
            quest_id,
            expected_quest_revision: entity.revision,
            description: quest.input.description.clone(),
            parent_catalog_id: "g1r:quest-parent:swampcamp_scchapter2".to_owned(),
            giver_catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
        }
    }

    fn alternate_test_parent(
        project: &gore_authoring::ProjectRevision3,
        quest_id: AuthoringEntityId,
    ) -> gore_authoring::Revision3QuestParentInput {
        let Revision3EntityPayload::QuestDraft(quest) = &project.entities[&quest_id].payload else {
            panic!("expected context-edit Quest Draft")
        };
        let mut parent = quest.input.parent_quest.clone();
        parent.canonical_selector = "Catalog_TestAlternateParent".to_owned();
        parent.runtime_class = "UQuest_TestAlternateParent".to_owned();
        parent
    }

    fn prepare_context_edit_artifact_v1(
        store: &WorkingProjectStore,
        head: &WorkingHead,
        catalog: &StoryCatalogFile,
        alternate_parent: Option<gore_authoring::Revision3QuestParentInput>,
    ) -> PreparedQuestCollisionArtifactV2 {
        let mut prepared = prepare_v3_transaction_artifact(store, head, catalog);
        if let Some(parent) = alternate_parent {
            prepared
                .insert_test_parent_selection("g1r:quest-parent:test-alternate".to_owned(), parent);
        }
        prepared
    }

    fn assert_context_edit_exact_delta(
        before: &gore_authoring::ProjectRevision3,
        outcome: &Revision3QuestContextEditOutcomeV1,
        quest_id: AuthoringEntityId,
        expected_description: &str,
        expected_parent: &gore_authoring::Revision3QuestParentInput,
        expected_giver: &gore_authoring::Revision3QuestGiverInput,
    ) {
        let after = outcome.project();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(after.asset_store, before.asset_store);
        assert_eq!(after.entities.len(), before.entities.len());

        let before_quest_entity = &before.entities[&quest_id];
        let Revision3EntityPayload::QuestDraft(before_quest) = &before_quest_entity.payload else {
            panic!("expected basis Quest Draft")
        };
        let module_id = before_quest.script_module.id;
        let before_module_entity = &before.entities[&module_id];
        let Revision3EntityPayload::ScriptModule(before_module) = &before_module_entity.payload
        else {
            panic!("expected basis ScriptModule")
        };
        let after_quest_entity = &after.entities[&quest_id];
        let Revision3EntityPayload::QuestDraft(after_quest) = &after_quest_entity.payload else {
            panic!("expected edited Quest Draft")
        };
        let after_module_entity = &after.entities[&module_id];
        let Revision3EntityPayload::ScriptModule(after_module) = &after_module_entity.payload
        else {
            panic!("expected edited ScriptModule")
        };

        assert_eq!(after_quest_entity.id, before_quest_entity.id);
        assert_eq!(
            after_quest_entity.display_name,
            before_quest_entity.display_name
        );
        assert_eq!(after_quest_entity.origin, before_quest_entity.origin);
        assert_eq!(
            after_quest_entity.revision,
            before_quest_entity.revision + 1
        );
        assert_eq!(after_module_entity.id, before_module_entity.id);
        assert_eq!(
            after_module_entity.display_name,
            before_module_entity.display_name
        );
        assert_eq!(after_module_entity.origin, before_module_entity.origin);
        assert_eq!(
            after_module_entity.revision,
            before_module_entity.revision + 1
        );

        assert_eq!(after_quest.input.description, expected_description);
        assert_eq!(&after_quest.input.parent_quest, expected_parent);
        assert_eq!(&after_quest.input.giver, expected_giver);
        let mut normalized_quest = after_quest.clone();
        normalized_quest.input.description = before_quest.input.description.clone();
        normalized_quest.input.parent_quest = before_quest.input.parent_quest.clone();
        normalized_quest.input.giver = before_quest.input.giver.clone();
        assert_eq!(normalized_quest, *before_quest);

        assert_eq!(after_module.generator_id, before_module.generator_id);
        assert_eq!(
            after_module.generator_version,
            before_module.generator_version
        );
        assert_eq!(after_module.owner, before_module.owner);
        assert_eq!(
            after_module.module_namespace,
            before_module.module_namespace
        );
        assert_eq!(
            after_module.module_relative_path,
            before_module.module_relative_path
        );
        assert_eq!(after_module.status, before_module.status);
        let expected_module = gore_authoring::regenerate_revision3_quest_module_v2(
            after_quest,
            QuestCollisionCatalogInput {
                generation: after_quest.input.collision_catalog.generation.clone(),
                source_seal: after_quest.input.collision_catalog.source_seal.clone(),
                catalog_layer: after_quest.input.collision_catalog.catalog_layer.clone(),
                modules: BTreeSet::new(),
                relative_paths: BTreeSet::new(),
                symbols: BTreeSet::new(),
            },
        )
        .unwrap();
        assert_eq!(after_module, &expected_module);

        let mut normalized_project = after.clone();
        normalized_project.revision = before.revision;
        normalized_project
            .entities
            .insert(quest_id, before_quest_entity.clone());
        normalized_project
            .entities
            .insert(module_id, before_module_entity.clone());
        assert_eq!(&normalized_project, before);
    }

    fn apply_v3(
        store: &WorkingProjectStore,
        head: &WorkingHead,
        catalog: &StoryCatalogFile,
        project: &gore_authoring::ProjectRevision3,
        request: &Revision3QuestDraftInsertRequestV3,
    ) -> Revision3QuestDraftInsertOutcomeV3 {
        apply_revision3_quest_draft_transaction_v3(
            prepare_v3_transaction_artifact(store, head, catalog),
            &project.to_canonical_json().unwrap(),
            &request.to_canonical_json().unwrap(),
        )
        .unwrap()
    }

    fn stage_v3_artifact_for_next_source(
        store: &WorkingProjectStore,
        head: &WorkingHead,
        outcome: &Revision3QuestDraftInsertOutcomeV3,
    ) {
        // C1 deliberately has no V2 store persistence API. The existing lower storage primitive
        // is format-agnostic at the byte CAS boundary; ignore its legacy metadata and retain the
        // V2 metadata already placed in the candidate project by the transaction.
        let imported = store
            .import_quest_collision_artifact_v1(
                outcome.collision_artifact.canonical_json(),
                Some(head),
            )
            .unwrap();
        assert_eq!(
            imported.artifact,
            authoring_seal(outcome.collision_artifact.artifact_seal())
        );
    }

    fn authoring_raw_seal(bytes: &[u8]) -> AuthoringContentSeal {
        AuthoringContentSeal {
            byte_len: bytes.len() as u64,
            sha256: AuthoringSha256Digest::from_bytes(Sha256::digest(bytes).into()),
        }
    }

    fn stored_asset_path(root: &S3TestRoot, digest: AuthoringSha256Digest) -> PathBuf {
        let hex = digest.to_string();
        root.0
            .join("assets")
            .join("sha256")
            .join(&hex[..2])
            .join(&hex[2..])
    }

    fn v2_semantic_seal(bytes: &[u8]) -> ContentSeal {
        let mut hasher = Sha256::new();
        hasher.update(
            b"gore-story-inventory.quest-collision-capability.v2.exact-current-revision3-payload\0",
        );
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        ContentSeal {
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(hasher.finalize().into()),
        }
    }

    fn reopen_forged_v2(bytes: &[u8]) -> QuestCollisionCapabilityArtifactV2 {
        let raw = seal(bytes);
        let semantic = v2_semantic_seal(bytes);
        reopen_quest_collision_capability_artifact_v2(bytes, &raw, &semantic).unwrap()
    }

    fn replace_v2_wire_field<T: serde::Serialize>(
        canonical: &str,
        field: &str,
        old: &T,
        new: &T,
    ) -> String {
        let needle = format!("\"{field}\":{}", serde_json::to_string(old).unwrap());
        let replacement = format!("\"{field}\":{}", serde_json::to_string(new).unwrap());
        let replaced = canonical.replacen(&needle, &replacement, 1);
        assert_ne!(replaced, canonical, "missing V2 wire field {field}");
        replaced
    }

    fn fresh_v2_verification_error(
        store: &WorkingProjectStore,
        head: &WorkingHead,
        catalog: &StoryCatalogFile,
        forged_canonical: &str,
    ) -> Revision3QuestCollisionCapabilityArtifactVerificationErrorV2 {
        let forged = reopen_forged_v2(forged_canonical.as_bytes());
        VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(head)
                .unwrap(),
        )
        .unwrap()
        .verify_artifact_exact(&forged)
        .unwrap_err()
    }

    fn regenerate_s3_pair_after_intent_change(
        project: &mut gore_authoring::ProjectRevision3,
        quest_id: AuthoringEntityId,
    ) {
        let quest = s3_quest_mut(project, quest_id).clone();
        let collision_input = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let module = regenerate_revision3_quest_module(&quest, collision_input).unwrap();
        let module_id = quest.script_module.id;
        let Revision3EntityPayload::ScriptModule(persisted) =
            &mut project.entities.get_mut(&module_id).unwrap().payload
        else {
            panic!("expected S3 ScriptModule")
        };
        *persisted = module;
        project.validate_closed_model().unwrap();
    }

    fn add_second_s3_quest_pair(
        project: &mut gore_authoring::ProjectRevision3,
        first_quest_id: AuthoringEntityId,
    ) -> AuthoringEntityId {
        let first = s3_quest_mut(project, first_quest_id).clone();
        let quest_id = authoring_entity_id(0x73);
        let module_id = authoring_entity_id(0x74);
        let quest = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: first.input.target.clone(),
                quest_id,
                module_namespace: "GoreMods.Quests.SecondAsghanTrial".to_owned(),
                technical_id: "GORE_SECOND_ASGHAN_TRIAL".to_owned(),
                text_helper: "GoreSecondQuestText".to_owned(),
                parent_quest: first.input.parent_quest,
                giver: first.input.giver,
                title: "Second Asghan Trial".to_owned(),
                description: "Prove that repeated Quest authoring remains collision-safe."
                    .to_owned(),
                objective_title: "Report to Asghan again".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: None,
                collision_catalog: first.input.collision_catalog,
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
        };
        let collision_input = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let module = regenerate_revision3_quest_module(&quest, collision_input).unwrap();
        let owner = Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        );
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Second S3 Asghan Trial".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: quest.input.technical_id.clone(),
                },
                revision: 0,
                payload: Revision3EntityPayload::QuestDraft(quest),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Second S3 Asghan Trial source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                },
                revision: 0,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        );
        project.revision += 1;
        project.validate_closed_model().unwrap();
        quest_id
    }

    #[test]
    fn revision3_v2_capability_zero_prior_roundtrips_and_finalizes_linearly() {
        let root = S3TestRoot::new("v2-zero-prior");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let revision2 = empty_authoring_project();
        let project = migrate_revision2_to_revision3(&revision2).unwrap().project;
        let head = publish_revision3_head(&root, &store, &project);
        let project_json = project.to_canonical_json().unwrap();

        let source = store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        let capability =
            VerifiedRevision3QuestCollisionCapabilityV2::bind(artifact(), &catalog, source)
                .unwrap();
        assert_eq!(capability.current_head(), &head);
        assert_eq!(
            capability.current_project(),
            &authoring_raw_seal(project_json.as_bytes())
        );
        assert_eq!(capability.prior_quest_count(), 0);
        assert!(capability.contains_module("GORE.ALPHA"));
        assert!(capability.contains_relative_path("GORE/ALPHA.AS"));
        assert!(capability.contains_symbol("UALPHA"));
        let oversized_query = "x".repeat(MAX_COLLISION_ENTRY_BYTES + 1);
        assert!(!capability.contains_module(&oversized_query));
        assert!(matches!(
            capability.resolve_parent(&oversized_query),
            Err(Revision3QuestCollisionCapabilityErrorV2::InvalidCatalogQuery { .. })
        ));
        let artifact_v2 = capability.into_artifact().unwrap();
        assert_eq!(artifact_v2.current_head(), &head);
        assert_eq!(artifact_v2.prior_quest_count(), 0);

        let reopened = reopen_quest_collision_capability_artifact_v2(
            artifact_v2.canonical_json(),
            artifact_v2.artifact_seal(),
            artifact_v2.source_seal(),
        )
        .unwrap();
        assert_eq!(reopened, artifact_v2);

        let fresh_source = store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        VerifiedRevision3QuestCollisionCapabilityV2::bind(artifact(), &catalog, fresh_source)
            .unwrap()
            .verify_artifact_exact(&reopened)
            .unwrap();

        let prepared = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            &catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap()
        .prepare_artifact()
        .unwrap();
        assert_eq!(prepared.artifact(), &artifact_v2);
        let (materialized, input) = prepared.finalize().unwrap();
        assert_eq!(materialized, artifact_v2);
        assert_eq!(
            input.catalog_layer,
            BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2
        );
        assert_eq!(
            input.source_seal,
            authoring_seal(materialized.source_seal())
        );
    }

    #[test]
    fn revision3_v2_capability_includes_regenerated_prior_quest_and_exact_head_bindings() {
        let fixture = s3_fixture("v2-prior");
        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
        let source = fixture
            .store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        let prior = source.prior_quests().get(&fixture.quest_id).unwrap();
        let prior_module = prior.module_namespace().to_owned();
        let prior_path = prior.module_relative_path().to_owned();
        let prior_symbols = prior.symbols().to_vec();
        let prior_evidence = source.prior_quest_evidence().clone();
        let nonquest_project = source.nonquest_basis().canonical_project().clone();

        let capability =
            VerifiedRevision3QuestCollisionCapabilityV2::bind(artifact(), &fixture.catalog, source)
                .unwrap();
        assert_eq!(capability.prior_quest_count(), 1);
        assert!(capability.contains_module(&prior_module));
        assert!(capability.contains_relative_path(&prior_path));
        for symbol in &prior_symbols {
            assert!(capability.contains_symbol(symbol));
        }
        assert!(capability
            .resolve_parent("g1r:quest-parent:swampcamp_scchapter2")
            .is_ok());
        assert!(capability
            .resolve_giver("g1r:npc:om_grd_asghan_263")
            .is_ok());

        let artifact_v2 = capability.into_artifact().unwrap();
        assert_eq!(artifact_v2.current_head(), &head);
        assert_eq!(artifact_v2.nonquest_project(), &nonquest_project);
        assert_eq!(artifact_v2.prior_quest_count(), 1);
        assert_eq!(artifact_v2.prior_quest_evidence(), &prior_evidence);
        let reopened = reopen_quest_collision_capability_artifact_v2(
            artifact_v2.canonical_json(),
            artifact_v2.artifact_seal(),
            artifact_v2.source_seal(),
        )
        .unwrap();
        let fresh = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            &fixture.catalog,
            fixture
                .store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap()
        .verify_artifact_exact(&reopened)
        .unwrap();
        let input = fresh.into_quest_collision_input();
        assert!(input.modules.contains(&prior_module.to_ascii_lowercase()));
        assert!(input
            .relative_paths
            .contains(&prior_path.to_ascii_lowercase()));
        for symbol in prior_symbols {
            assert!(input.symbols.contains(&symbol.to_ascii_lowercase()));
        }
    }

    #[test]
    fn revision3_v2_capability_unions_npc_and_two_prior_quests_deterministically() {
        let mut fixture = s3_fixture("v2-two-prior");

        let mut npc_revision2 = empty_authoring_project();
        add_authoring_npc(
            &mut npc_revision2,
            "Project.Npcs.MultiQuestWitness",
            "MultiQuestWitness",
        );
        let npc_revision3 = migrate_revision2_to_revision3(&npc_revision2)
            .unwrap()
            .project;
        for (id, entity) in npc_revision3.entities {
            assert!(fixture.project.entities.insert(id, entity).is_none());
        }
        let second_quest_id = add_second_s3_quest_pair(&mut fixture.project, fixture.quest_id);
        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);

        let source = fixture
            .store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        assert_eq!(source.prior_quest_count(), 2);
        let first = source.prior_quests().get(&fixture.quest_id).unwrap();
        let second = source.prior_quests().get(&second_quest_id).unwrap();
        let expected_modules = [
            first.module_namespace().to_ascii_lowercase(),
            second.module_namespace().to_ascii_lowercase(),
            "project.npcs.multiquestwitness".to_owned(),
        ];
        let expected_paths = [
            first.module_relative_path().to_ascii_lowercase(),
            second.module_relative_path().to_ascii_lowercase(),
            "project/npcs/multiquestwitness.as".to_owned(),
        ];

        let capability =
            VerifiedRevision3QuestCollisionCapabilityV2::bind(artifact(), &fixture.catalog, source)
                .unwrap();
        for value in &expected_modules {
            assert!(capability.contains_module(value));
        }
        for value in &expected_paths {
            assert!(capability.contains_relative_path(value));
        }
        let first_artifact = capability.into_artifact().unwrap();
        assert_eq!(first_artifact.prior_quest_count(), 2);

        let second_artifact = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            &fixture.catalog,
            fixture
                .store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap()
        .into_artifact()
        .unwrap();
        assert_eq!(second_artifact, first_artifact);
        assert_eq!(
            second_artifact.canonical_json(),
            first_artifact.canonical_json()
        );
    }

    #[test]
    fn revision3_v2_artifact_is_domain_and_schema_distinct_from_v1() {
        let root = S3TestRoot::new("v2-domain");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let revision2 = empty_authoring_project();
        let project = migrate_revision2_to_revision3(&revision2).unwrap().project;
        let head = publish_revision3_head(&root, &store, &project);
        let v1 = VerifiedQuestCollisionCapability::bind(artifact(), &catalog, &revision2)
            .unwrap()
            .into_artifact()
            .unwrap();
        let v2 = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            &catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap()
        .into_artifact()
        .unwrap();

        assert_ne!(v1.catalog_layer(), v2.catalog_layer());
        assert_ne!(v1.source_seal(), v2.source_seal());
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v2(
                v1.canonical_json(),
                v1.artifact_seal(),
                v1.source_seal(),
            ),
            Err(QuestCollisionCapabilityArtifactErrorV2::SourceSealMismatch)
        ));
        assert!(matches!(
            reopen_quest_collision_capability_artifact_v1(
                v2.canonical_json(),
                v2.artifact_seal(),
                v2.source_seal(),
            ),
            Err(QuestCollisionCapabilityArtifactError::SourceSealMismatch)
        ));
    }

    #[test]
    fn revision3_v2_structural_forgery_never_rehydrates_fresh_authority() {
        let root = S3TestRoot::new("v2-forgery");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let artifact_v2 = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact(),
            &catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap()
        .into_artifact()
        .unwrap();
        let canonical = String::from_utf8(artifact_v2.canonical_json().to_vec()).unwrap();

        let mut changed_head = artifact_v2.current_head().clone();
        changed_head.snapshot.sha256 = AuthoringSha256Digest::from_bytes([0xa1; 32]);
        let forged_head = replace_v2_wire_field(
            &canonical,
            "current_head",
            artifact_v2.current_head(),
            &changed_head,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_head),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::CurrentHeadMismatch
        ));

        let mut changed_base = artifact_v2.base_inventory_payload_seal().clone();
        changed_base.sha256 = Sha256Digest::from_bytes([0xa2; 32]);
        let forged_base = replace_v2_wire_field(
            &canonical,
            "base_inventory_payload_seal",
            artifact_v2.base_inventory_payload_seal(),
            &changed_base,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_base),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::BaseInventoryPayloadSealMismatch
        ));

        let mut changed_catalog = artifact_v2.story_catalog_seal().clone();
        changed_catalog.sha256 = Sha256Digest::from_bytes([0xa3; 32]);
        let forged_catalog = replace_v2_wire_field(
            &canonical,
            "story_catalog_seal",
            artifact_v2.story_catalog_seal(),
            &changed_catalog,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_catalog),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::StoryCatalogSealMismatch
        ));

        let changed_project_id = ProjectId::from_bytes([0xa4; 16]);
        let forged_project_id = replace_v2_wire_field(
            &canonical,
            "project_id",
            &artifact_v2.project_id(),
            &changed_project_id,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_project_id),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::ProjectIdMismatch
        ));

        let changed_revision = artifact_v2.project_revision() + 1;
        let forged_revision = replace_v2_wire_field(
            &canonical,
            "project_revision",
            &artifact_v2.project_revision(),
            &changed_revision,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_revision),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::ProjectRevisionMismatch
        ));

        let mut changed_target = artifact_v2.project_target().clone();
        changed_target.executable.sha256 = AuthoringSha256Digest::from_bytes([0xa5; 32]);
        let forged_target = replace_v2_wire_field(
            &canonical,
            "project_target",
            artifact_v2.project_target(),
            &changed_target,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_target),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::ProjectTargetMismatch
        ));

        let mut changed_current = artifact_v2.current_project().clone();
        changed_current.sha256 = AuthoringSha256Digest::from_bytes([0xa6; 32]);
        let forged_current = replace_v2_wire_field(
            &canonical,
            "current_project",
            artifact_v2.current_project(),
            &changed_current,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_current),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::CurrentProjectMismatch
        ));

        let mut changed_nonquest = artifact_v2.nonquest_project().clone();
        changed_nonquest.sha256 = AuthoringSha256Digest::from_bytes([0xa7; 32]);
        let forged_nonquest = replace_v2_wire_field(
            &canonical,
            "nonquest_project",
            artifact_v2.nonquest_project(),
            &changed_nonquest,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_nonquest),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::NonQuestProjectMismatch
        ));

        let forged_count = replace_v2_wire_field(
            &canonical,
            "prior_quest_count",
            &artifact_v2.prior_quest_count(),
            &1u64,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_count),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::PriorQuestCountMismatch
        ));

        let mut changed_prior = artifact_v2.prior_quest_evidence().clone();
        changed_prior.sha256 = AuthoringSha256Digest::from_bytes([0xa8; 32]);
        let forged_prior = replace_v2_wire_field(
            &canonical,
            "prior_quest_evidence",
            artifact_v2.prior_quest_evidence(),
            &changed_prior,
        );
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_prior),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::PriorQuestEvidenceMismatch
        ));

        let original_modules = vec!["gore.alpha".to_owned(), "gore.zeta".to_owned()];
        let forged_modules = vec!["gore.beta".to_owned(), "gore.zeta".to_owned()];
        let forged_collision =
            replace_v2_wire_field(&canonical, "modules", &original_modules, &forged_modules);
        assert!(matches!(
            fresh_v2_verification_error(&store, &head, &catalog, &forged_collision),
            Revision3QuestCollisionCapabilityArtifactVerificationErrorV2::SemanticSourceSealMismatch
        ));
    }

    #[test]
    fn revision3_v2_bind_rejects_prior_catalog_drift_and_base_collision() {
        for (label, mutate, expected) in [
            ("parent-source", 0u8, "parent"),
            ("parent-layer", 1u8, "parent"),
            ("parent-selector", 2u8, "parent"),
            ("parent-runtime", 3u8, "parent"),
            ("giver-source", 4u8, "giver"),
            ("giver-layer", 5u8, "giver"),
            ("giver-selector", 6u8, "giver"),
            ("giver-runtime", 7u8, "giver"),
            ("base-collision", 8u8, "base"),
        ] {
            let mut fixture = s3_fixture(&format!("v2-{label}"));
            match mutate {
                0 => {
                    s3_quest_mut(&mut fixture.project, fixture.quest_id)
                        .input
                        .parent_quest
                        .source_seal
                        .sha256 = AuthoringSha256Digest::from_bytes([0xc0; 32]);
                }
                1 => s3_quest_mut(&mut fixture.project, fixture.quest_id)
                    .input
                    .parent_quest
                    .catalog_layer
                    .push_str(".drift"),
                2 => s3_quest_mut(&mut fixture.project, fixture.quest_id)
                    .input
                    .parent_quest
                    .canonical_selector
                    .push_str("_drift"),
                3 => s3_quest_mut(&mut fixture.project, fixture.quest_id)
                    .input
                    .parent_quest
                    .runtime_class
                    .push_str("_Drift"),
                4 => {
                    s3_quest_mut(&mut fixture.project, fixture.quest_id)
                        .input
                        .giver
                        .source_seal
                        .sha256 = AuthoringSha256Digest::from_bytes([0xc4; 32]);
                }
                5 => s3_quest_mut(&mut fixture.project, fixture.quest_id)
                    .input
                    .giver
                    .catalog_layer
                    .push_str(".drift"),
                6 => s3_quest_mut(&mut fixture.project, fixture.quest_id)
                    .input
                    .giver
                    .canonical_selector
                    .push_str("_drift"),
                7 => s3_quest_mut(&mut fixture.project, fixture.quest_id)
                    .input
                    .giver
                    .runtime_unique_name
                    .push_str("_DRIFT"),
                _ => {
                    s3_quest_mut(&mut fixture.project, fixture.quest_id)
                        .input
                        .module_namespace = "gore.alpha".to_owned();
                }
            }
            regenerate_s3_pair_after_intent_change(&mut fixture.project, fixture.quest_id);
            let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
            let source = fixture
                .store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap();
            let error = VerifiedRevision3QuestCollisionCapabilityV2::bind(
                artifact(),
                &fixture.catalog,
                source,
            )
            .unwrap_err();
            match expected {
                "parent" => assert!(matches!(
                    error,
                    Revision3QuestCollisionCapabilityErrorV2::PriorQuestParentDrift { .. }
                )),
                "giver" => assert!(matches!(
                    error,
                    Revision3QuestCollisionCapabilityErrorV2::PriorQuestGiverDrift { .. }
                )),
                _ => assert!(matches!(
                    error,
                    Revision3QuestCollisionCapabilityErrorV2::BaseCurrentCollision {
                        kind: "module",
                        ..
                    }
                )),
            }
        }
    }

    #[test]
    fn revision3_prior_generation_drift_is_rejected_before_v2_capability_binding() {
        for role in ["parent", "giver"] {
            let mut fixture = s3_fixture(&format!("v2-{role}-generation"));
            let quest = s3_quest_mut(&mut fixture.project, fixture.quest_id);
            let generation = if role == "parent" {
                &mut quest.input.parent_quest.generation
            } else {
                &mut quest.input.giver.generation
            };
            generation.executable.sha256 = AuthoringSha256Digest::from_bytes([0xcf; 32]);
            let collision_input = QuestCollisionCatalogInput {
                generation: quest.input.collision_catalog.generation.clone(),
                source_seal: quest.input.collision_catalog.source_seal.clone(),
                catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
                modules: BTreeSet::new(),
                relative_paths: BTreeSet::new(),
                symbols: BTreeSet::new(),
            };
            assert!(regenerate_revision3_quest_module(quest, collision_input).is_err());
        }
    }

    #[test]
    fn revision3_v2_bind_rejects_base_collisions_for_nonquest_and_prior_in_all_domains() {
        let catalog = trusted_catalog();

        let mut revision2 = empty_authoring_project();
        add_authoring_npc(&mut revision2, "Project.Npcs.Collision", "CollisionNpc");
        let nonquest_cases = [
            ("module", "project.npcs.collision"),
            ("relative path", "project/npcs/collision.as"),
            ("symbol", "ucharacterdefinition_human_collisionnpc"),
        ];
        for (kind, value) in nonquest_cases {
            let root = S3TestRoot::new(&format!("v2-nonquest-{kind}"));
            let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
            let project = migrate_revision2_to_revision3(&revision2).unwrap().project;
            let head = publish_revision3_head(&root, &store, &project);
            let source = store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap();
            assert!(matches!(
                VerifiedRevision3QuestCollisionCapabilityV2::bind(
                    artifact_with_extra_collision(kind, value),
                    &catalog,
                    source,
                ),
                Err(Revision3QuestCollisionCapabilityErrorV2::BaseCurrentCollision {
                    kind: actual_kind,
                    value: actual_value,
                    owner,
                }) if actual_kind == kind
                    && actual_value == value
                    && owner == authoring_entity_id(10)
            ));
        }

        let fixture = s3_fixture("v2-prior-domain-matrix");
        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
        let evidence_source = fixture
            .store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        let prior = evidence_source
            .prior_quests()
            .get(&fixture.quest_id)
            .unwrap();
        let prior_cases = [
            ("module", prior.module_namespace().to_ascii_lowercase()),
            (
                "relative path",
                prior.module_relative_path().to_ascii_lowercase(),
            ),
            ("symbol", prior.symbols()[0].to_ascii_lowercase()),
        ];
        drop(evidence_source);
        for (kind, value) in prior_cases {
            let source = fixture
                .store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap();
            assert!(matches!(
                VerifiedRevision3QuestCollisionCapabilityV2::bind(
                    artifact_with_extra_collision(kind, &value),
                    &catalog,
                    source,
                ),
                Err(Revision3QuestCollisionCapabilityErrorV2::BaseCurrentCollision {
                    kind: actual_kind,
                    value: actual_value,
                    owner,
                }) if actual_kind == kind
                    && actual_value == value
                    && owner == fixture.quest_id
            ));
        }
    }

    #[test]
    fn revision3_v2_bind_requires_exact_base_catalog_and_project_target() {
        let catalog = trusted_catalog();
        let revision2 = empty_authoring_project();
        let project = migrate_revision2_to_revision3(&revision2).unwrap().project;
        let root = S3TestRoot::new("v2-bindings");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let head = publish_revision3_head(&root, &store, &project);

        let mut wrong_catalog_binding = artifact();
        wrong_catalog_binding.wire.inventory.story_catalog_seal = seal(b"different catalog");
        let payload = canonical_json(
            &wrong_catalog_binding.wire.inventory,
            "wrong-catalog V2 test payload",
        )
        .unwrap();
        wrong_catalog_binding.wire.payload_seal = seal_bytes(&payload);
        assert!(matches!(
            VerifiedRevision3QuestCollisionCapabilityV2::bind(
                wrong_catalog_binding,
                &catalog,
                store
                    .prepare_current_revision3_quest_collision_source_v2(&head)
                    .unwrap(),
            ),
            Err(Revision3QuestCollisionCapabilityErrorV2::CatalogBindingMismatch)
        ));

        let mut wrong_target_revision2 = empty_authoring_project();
        wrong_target_revision2.target.executable.sha256 =
            AuthoringSha256Digest::from_bytes([0xb1; 32]);
        let wrong_target_project = migrate_revision2_to_revision3(&wrong_target_revision2)
            .unwrap()
            .project;
        let wrong_root = S3TestRoot::new("v2-target");
        let wrong_store =
            WorkingProjectStore::at(&wrong_root.0, WorkingStoreLimits::default()).unwrap();
        let wrong_head = publish_revision3_head(&wrong_root, &wrong_store, &wrong_target_project);
        assert!(matches!(
            VerifiedRevision3QuestCollisionCapabilityV2::bind(
                artifact(),
                &catalog,
                wrong_store
                    .prepare_current_revision3_quest_collision_source_v2(&wrong_head)
                    .unwrap(),
            ),
            Err(Revision3QuestCollisionCapabilityErrorV2::TargetMismatch)
        ));
    }

    #[test]
    fn revision3_v2_bind_budget_accepts_exact_combined_limits_and_rejects_plus_one() {
        let catalog = trusted_catalog();
        let mut revision2 = empty_authoring_project();
        add_authoring_npc(
            &mut revision2,
            "Project.Npcs.BudgetWitness",
            "BudgetWitness",
        );
        let project = migrate_revision2_to_revision3(&revision2).unwrap().project;
        let root = S3TestRoot::new("v2-bind-budget");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let head = publish_revision3_head(&root, &store, &project);
        let evidence = store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        let identities = evidence.nonquest_basis().story_identities();
        let current_count = identities.modules().len()
            + identities.relative_paths().len()
            + identities.symbols().len();
        let current_bytes = identities
            .modules()
            .keys()
            .chain(identities.relative_paths().keys())
            .chain(identities.symbols().keys())
            .map(String::len)
            .sum::<usize>();
        assert!(current_count > 0);
        assert!(current_bytes > 1);
        drop(evidence);

        let exact_count_base = MAX_COLLISION_ENTRIES - current_count;
        let exact_count = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact_with_module_collisions(distinct_collision_values(exact_count_base)),
            &catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap();
        assert!(exact_count.contains_module(&format!("budget.m{:06}", exact_count_base - 1)));
        drop(exact_count);

        assert!(matches!(
            VerifiedRevision3QuestCollisionCapabilityV2::bind(
                artifact_with_module_collisions(distinct_collision_values(exact_count_base + 1)),
                &catalog,
                store
                    .prepare_current_revision3_quest_collision_source_v2(&head)
                    .unwrap(),
            ),
            Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
                kind: "collision entry count",
                actual,
                max: MAX_COLLISION_ENTRIES,
            }) if actual == MAX_COLLISION_ENTRIES + 1
        ));

        let exact_base_bytes = MAX_COLLISION_TOTAL_BYTES - current_bytes;
        let exact_bytes = VerifiedRevision3QuestCollisionCapabilityV2::bind(
            artifact_with_module_collisions(distinct_collision_values_with_exact_bytes(
                exact_base_bytes,
            )),
            &catalog,
            store
                .prepare_current_revision3_quest_collision_source_v2(&head)
                .unwrap(),
        )
        .unwrap();
        drop(exact_bytes);

        assert!(matches!(
            VerifiedRevision3QuestCollisionCapabilityV2::bind(
                artifact_with_module_collisions(distinct_collision_values_with_exact_bytes(
                    exact_base_bytes + 1,
                )),
                &catalog,
                store
                    .prepare_current_revision3_quest_collision_source_v2(&head)
                    .unwrap(),
            ),
            Err(Revision3QuestCollisionCapabilityErrorV2::Limit {
                kind: "aggregate collision entry bytes",
                actual,
                max: MAX_COLLISION_TOTAL_BYTES,
            }) if actual == MAX_COLLISION_TOTAL_BYTES + 1
        ));
    }

    #[test]
    fn s3_store_to_fresh_capability_to_exact_source_plan_is_end_to_end() {
        let fixture = s3_fixture("happy");
        let canonical = fixture.canonical(&fixture.project);
        let prepared =
            prepare_revision3_quest_source_inspection(&fixture.store, &canonical, fixture.quest_id)
                .unwrap();
        assert_eq!(
            prepared.collision_source_project(),
            &fixture.collision_source
        );
        let plan = prepared.lower(fixture.fresh_capability()).unwrap();
        assert_eq!(plan.scope, QuestInspectionScope::SourceInspectionOnly);
        assert_eq!(plan.build_status, QuestInspectionBuildStatus::Blocked);
        assert_eq!(
            plan.runtime_qualification,
            QuestInspectionRuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(
            plan.publication_status,
            QuestInspectionPublicationStatus::NotSupported
        );
        assert_eq!(plan.module.quest.id, fixture.quest_id);
        assert!(plan
            .module
            .generated
            .source
            .contains("UQuest_GORE_S3_ASGHAN_TRIAL"));

        let plan_json = plan.to_canonical_json().unwrap();
        assert_eq!(
            Revision3QuestSourceInspectionPlanV2::from_json(&plan_json).unwrap(),
            plan
        );
        plan.verify_against_sources(&fixture.store, &canonical, fixture.fresh_capability())
            .unwrap();
        assert!(plan.content_seal().unwrap().byte_len > 0);

        let mut oversized_input = plan.clone();
        oversized_input.module.draft_input.byte_len = u64::MAX;
        assert!(matches!(
            oversized_input.to_canonical_json(),
            Err(Revision3QuestInspectionError::PlanInvariant(_))
        ));
    }

    #[test]
    fn s3_raw_semantic_basis_and_persisted_module_drift_fail_before_plan() {
        let fixture = s3_fixture("drift");

        let mut raw = fixture.project.clone();
        let (raw_sha256, raw_byte_len) = {
            let raw_ref = &mut s3_quest_mut(&mut raw, fixture.quest_id)
                .input
                .collision_catalog
                .artifact;
            raw_ref.sha256 = AuthoringSha256Digest::from_bytes([0xa1; 32]);
            (raw_ref.sha256, raw_ref.byte_len)
        };
        raw.asset_store.assets.insert(
            raw_sha256,
            AssetMeta {
                byte_len: raw_byte_len,
                media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE.to_owned(),
            },
        );
        assert!(matches!(
            prepare_revision3_quest_source_inspection(
                &fixture.store,
                &fixture.canonical(&raw),
                fixture.quest_id,
            ),
            Err(Revision3QuestInspectionError::ArtifactUnavailable { .. })
        ));

        let mut semantic = fixture.project.clone();
        s3_quest_mut(&mut semantic, fixture.quest_id)
            .input
            .collision_catalog
            .source_seal
            .sha256 = AuthoringSha256Digest::from_bytes([0xa2; 32]);
        assert!(matches!(
            prepare_revision3_quest_source_inspection(
                &fixture.store,
                &fixture.canonical(&semantic),
                fixture.quest_id,
            ),
            Err(Revision3QuestInspectionError::InvalidArtifact(
                QuestCollisionCapabilityArtifactError::SourceSealMismatch
            ))
        ));

        let mut basis = fixture.project.clone();
        s3_quest_mut(&mut basis, fixture.quest_id)
            .input
            .collision_catalog
            .basis_snapshot
            .sha256 = AuthoringSha256Digest::from_bytes([0xa3; 32]);
        assert!(matches!(
            prepare_revision3_quest_source_inspection(
                &fixture.store,
                &fixture.canonical(&basis),
                fixture.quest_id,
            ),
            Err(Revision3QuestInspectionError::BasisUnavailable { .. })
        ));

        let mut module = fixture.project.clone();
        let module_id = s3_quest_mut(&mut module, fixture.quest_id).script_module.id;
        let Revision3EntityPayload::ScriptModule(persisted) =
            &mut module.entities.get_mut(&module_id).unwrap().payload
        else {
            panic!("expected S3 ScriptModule")
        };
        persisted.source.push_str("\n// persisted drift\n");
        persisted.source_sha256 =
            AuthoringSha256Digest::from_bytes(Sha256::digest(persisted.source.as_bytes()).into());
        let module_json = fixture.canonical(&module);
        let prepared = prepare_revision3_quest_source_inspection(
            &fixture.store,
            &module_json,
            fixture.quest_id,
        )
        .unwrap();
        assert!(matches!(
            prepared.lower(fixture.fresh_capability()),
            Err(Revision3QuestInspectionError::PersistedModuleDrift { .. })
        ));
    }

    #[test]
    fn revision3_v3_first_quest_is_authority_bound_atomic_canonical_and_unqualified() {
        let root = S3TestRoot::new("v3-first-quest");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let request = request_v3(&project, &head, 1);
        let original_json = project.to_canonical_json().unwrap();

        let outcome = apply_v3(&store, &head, &catalog, &project, &request);
        let deterministic_replay = apply_v3(&store, &head, &catalog, &project, &request);

        assert_eq!(outcome, deterministic_replay);
        assert_eq!(project.to_canonical_json().unwrap(), original_json);
        assert_eq!(outcome.basis_head, head);
        assert_eq!(outcome.project.revision, project.revision + 1);
        assert_eq!(outcome.project.entities.len(), project.entities.len() + 2);
        assert_eq!(outcome.project.asset_store.assets.len(), 1);
        assert_eq!(outcome.quest_id, request.quest_id);
        assert_eq!(outcome.script_module_id, request.script_module_id);
        assert_eq!(
            outcome.build_status,
            Revision3QuestDraftBuildStatusV3::Blocked
        );
        assert_eq!(
            outcome.runtime_status,
            Revision3QuestDraftRuntimeStatusV3::RuntimeUnqualified
        );
        assert_eq!(
            outcome.artifact_authority,
            Revision3QuestArtifactAuthorityV3::NotGranted
        );
        assert_eq!(
            outcome.source_inspection,
            Revision3QuestSourceInspectionStatusV3::FreshCapabilityRequired
        );
        assert_eq!(
            outcome.publication_status,
            Revision3QuestDraftPublicationStatusV3::NotSupported
        );
        assert_eq!(
            gore_authoring::ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
            outcome.project
        );

        let Revision3EntityPayload::QuestDraft(quest) =
            &outcome.project.entities[&request.quest_id].payload
        else {
            panic!("expected authority-sensitive Quest")
        };
        let Revision3EntityPayload::ScriptModule(module) =
            &outcome.project.entities[&request.script_module_id].payload
        else {
            panic!("expected authority-sensitive Quest module")
        };
        assert_eq!(quest.input.quest_id, request.quest_id);
        assert_eq!(quest.input.target, project.target);
        assert_eq!(
            quest.input.collision_catalog.catalog_layer,
            QUEST_COLLISION_CATALOG_LAYER_V2
        );
        assert_eq!(
            quest.input.collision_catalog.artifact,
            authoring_seal(outcome.collision_artifact.artifact_seal())
        );
        assert_eq!(
            quest.input.collision_catalog.source_seal,
            authoring_seal(outcome.collision_artifact.source_seal())
        );
        assert_eq!(quest.input.collision_catalog.basis_snapshot, head.snapshot);
        assert!(quest
            .input
            .parent_quest
            .canonical_selector
            .starts_with("Catalog_"));
        assert_eq!(quest.input.giver.runtime_unique_name, "OM_GRD_Asghan_263");
        assert_eq!(module.owner.id, request.quest_id);
        assert_eq!(
            outcome.project.asset_store.assets
                [&authoring_seal(outcome.collision_artifact.artifact_seal()).sha256],
            AssetMeta {
                byte_len: outcome.collision_artifact.artifact_seal().byte_len,
                media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
            }
        );
    }

    #[test]
    fn revision3_v3_second_and_multi_quest_chain_uses_each_fresh_exact_current_head() {
        let root = S3TestRoot::new("v3-multi-quest");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let mut project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let mut head = publish_revision3_head(&root, &store, &project);
        let initial_revision = project.revision;

        for ordinal in 1..=3 {
            let request = request_v3(&project, &head, ordinal);
            let prior_entities = project.entities.len();
            let prior_assets = project.asset_store.assets.len();
            let prior_revision = project.revision;
            let outcome = apply_v3(&store, &head, &catalog, &project, &request);
            assert_eq!(outcome.project.revision, prior_revision + 1);
            assert_eq!(outcome.project.entities.len(), prior_entities + 2);
            assert_eq!(outcome.project.asset_store.assets.len(), prior_assets + 1);
            for prior in 1..=ordinal {
                assert!(outcome
                    .project
                    .entities
                    .contains_key(&authoring_entity_id(0x80 + prior * 2)));
                assert!(outcome
                    .project
                    .entities
                    .contains_key(&authoring_entity_id(0x81 + prior * 2)));
            }

            stage_v3_artifact_for_next_source(&store, &head, &outcome);
            let next_head = advance_revision3_head(&root, &store, &head, &outcome.project);
            project = outcome.project;
            head = next_head;
        }

        assert_eq!(project.revision, initial_revision + 3);
        assert_eq!(project.entities.len(), 6);
        assert_eq!(project.asset_store.assets.len(), 3);
        let source = store
            .prepare_current_revision3_quest_collision_source_v2(&head)
            .unwrap();
        assert_eq!(source.prior_quest_count(), 3);
        let capability =
            VerifiedRevision3QuestCollisionCapabilityV2::bind(artifact(), &catalog, source)
                .unwrap();
        assert!(capability.contains_module("GoreMods.Quests.AuthorityQuest1"));
        assert!(capability.contains_module("GoreMods.Quests.AuthorityQuest2"));
        assert!(capability.contains_module("GoreMods.Quests.AuthorityQuest3"));
    }

    #[test]
    fn revision3_v3_persistence_prepares_first_and_second_heads_without_publishing() {
        let root = S3TestRoot::new("v3-persist-two");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let mut project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let mut head = publish_revision3_head(&root, &store, &project);

        for ordinal in 1..=2 {
            let mut request = request_v3(&project, &head, ordinal);
            if ordinal == 1 {
                request.intent.additional_objective_titles =
                    vec!["Inspect the gate".to_owned(), "Report to Asghan".to_owned()];
            }
            let outcome = apply_v3(&store, &head, &catalog, &project, &request);
            let prepared = prepare_revision3_quest_draft_persistence_v3(&store, outcome).unwrap();

            assert_eq!(prepared.basis_head, head);
            assert!(!prepared.imported_artifact.deduplicated);
            assert_eq!(
                store
                    .open_current_revision3(AssetVerification::Full)
                    .unwrap()
                    .head,
                head,
                "preparation must never publish the fixed head"
            );
            let reopened = store
                .open_revision3_head_bytes(&prepared.checkpoint.head_bytes, AssetVerification::Full)
                .unwrap();
            assert_eq!(reopened.head, prepared.checkpoint.head);
            assert_eq!(reopened.project, prepared.project);
            if ordinal == 1 {
                let Revision3EntityPayload::QuestDraft(quest) =
                    &reopened.project.entities[&request.quest_id].payload
                else {
                    panic!("expected persisted multi-objective Quest")
                };
                let Revision3EntityPayload::ScriptModule(module) =
                    &reopened.project.entities[&request.script_module_id].payload
                else {
                    panic!("expected persisted multi-objective Quest module")
                };
                assert_eq!(
                    quest.generator_version,
                    REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
                );
                assert_eq!(
                    quest.input.additional_objective_titles,
                    ["Inspect the gate", "Report to Asghan"]
                );
                assert_eq!(module.generator_version, quest.generator_version);
                let first = module
                    .source
                    .find("class UQuest_GORE_AUTHORITY_QUEST_1_OBJ_DONE")
                    .unwrap();
                let second = module
                    .source
                    .find("class UQuest_GORE_AUTHORITY_QUEST_1_OBJ_2")
                    .unwrap();
                let third = module
                    .source
                    .find("class UQuest_GORE_AUTHORITY_QUEST_1_OBJ_3")
                    .unwrap();
                assert!(first < second && second < third);
                assert_eq!(module.source.matches("bSucceedParent = true").count(), 1);
                assert!(module.source[third..].contains("default bSucceedParent = true;"));
            }
            assert!(stored_asset_path(&root, prepared.imported_artifact.artifact.sha256).is_file());

            fs::write(
                root.0.join("gore-project.json"),
                &prepared.checkpoint.head_bytes,
            )
            .unwrap();
            let published = store
                .open_current_revision3(AssetVerification::Full)
                .unwrap();
            assert_eq!(published.project, prepared.project);
            project = prepared.project;
            head = prepared.checkpoint.head;
        }

        assert_eq!(project.entities.len(), 4);
        assert_eq!(project.asset_store.assets.len(), 2);
    }

    #[test]
    fn revision3_v3_persistence_exact_replay_deduplicates_without_head_change() {
        let root = S3TestRoot::new("v3-persist-dedupe");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let request = request_v3(&project, &head, 1);
        let first_outcome = apply_v3(&store, &head, &catalog, &project, &request);
        let second_outcome = apply_v3(&store, &head, &catalog, &project, &request);

        let first = prepare_revision3_quest_draft_persistence_v3(&store, first_outcome).unwrap();
        let second = prepare_revision3_quest_draft_persistence_v3(&store, second_outcome).unwrap();

        assert!(!first.imported_artifact.deduplicated);
        assert!(second.imported_artifact.deduplicated);
        assert_eq!(
            first.imported_artifact.artifact,
            second.imported_artifact.artifact
        );
        assert_eq!(first.checkpoint, second.checkpoint);
        assert_eq!(
            store
                .open_current_revision3(AssetVerification::Full)
                .unwrap()
                .head,
            head
        );
    }

    #[test]
    fn revision3_v3_persistence_rejects_stale_and_forged_outcomes_before_publication() {
        let root = S3TestRoot::new("v3-persist-forged");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let request = request_v3(&project, &head, 1);

        let mut transport_drift = apply_v3(&store, &head, &catalog, &project, &request);
        transport_drift.canonical_project_json.push('\n');
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, transport_drift),
            Err(Revision3QuestDraftPersistenceErrorV3::CandidateProject(_))
        ));

        let mut project_drift = apply_v3(&store, &head, &catalog, &project, &request);
        project_drift.project.meta.name.push_str(" forged");
        project_drift.canonical_project_json = project_drift.project.to_canonical_json().unwrap();
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, project_drift),
            Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                Revision3QuestDraftPersistenceValidationErrorV3::CandidateProjectMetadataMismatch
            ))
        ));

        for invalid_display_name in [
            String::new(),
            "control\nname".to_owned(),
            "x".repeat(MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES + 1),
        ] {
            let mut display_drift = apply_v3(&store, &head, &catalog, &project, &request);
            display_drift
                .project
                .entities
                .get_mut(&request.quest_id)
                .unwrap()
                .display_name = invalid_display_name;
            display_drift.canonical_project_json =
                display_drift.project.to_canonical_json().unwrap();
            assert!(matches!(
                prepare_revision3_quest_draft_persistence_v3(&store, display_drift),
                Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                    Revision3QuestDraftPersistenceValidationErrorV3::QuestEntityMismatch
                ))
            ));
        }

        let mut outcome_id_drift = apply_v3(&store, &head, &catalog, &project, &request);
        outcome_id_drift.quest_id = authoring_entity_id(0xd1);
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, outcome_id_drift),
            Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                Revision3QuestDraftPersistenceValidationErrorV3::CandidateEntityDeltaMismatch
            ))
        ));

        let mut artifact_drift = apply_v3(&store, &head, &catalog, &project, &request);
        let artifact_json =
            std::str::from_utf8(artifact_drift.collision_artifact.canonical_json()).unwrap();
        let forged_artifact = replace_v2_wire_field(
            artifact_json,
            "project_revision",
            &project.revision,
            &(project.revision + 1),
        );
        artifact_drift.collision_artifact = reopen_forged_v2(forged_artifact.as_bytes());
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, artifact_drift),
            Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                Revision3QuestDraftPersistenceValidationErrorV3::ArtifactBasisMismatch
            ))
        ));

        let mut meta_drift = apply_v3(&store, &head, &catalog, &project, &request);
        let raw_digest = authoring_seal(meta_drift.collision_artifact.artifact_seal()).sha256;
        let Revision3EntityPayload::QuestDraft(quest) = &mut meta_drift
            .project
            .entities
            .get_mut(&request.quest_id)
            .unwrap()
            .payload
        else {
            panic!("expected Quest Draft")
        };
        quest.input.collision_catalog.artifact.byte_len += 1;
        quest.input.collision_catalog.source_seal.byte_len += 1;
        meta_drift
            .project
            .asset_store
            .assets
            .get_mut(&raw_digest)
            .unwrap()
            .byte_len += 1;
        meta_drift.canonical_project_json = meta_drift.project.to_canonical_json().unwrap();
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, meta_drift),
            Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                Revision3QuestDraftPersistenceValidationErrorV3::CandidateAssetDeltaMismatch
            ))
        ));

        let mut ref_drift = apply_v3(&store, &head, &catalog, &project, &request);
        let Revision3EntityPayload::QuestDraft(quest) = &mut ref_drift
            .project
            .entities
            .get_mut(&request.quest_id)
            .unwrap()
            .payload
        else {
            panic!("expected Quest Draft")
        };
        quest.input.collision_catalog.basis_snapshot.sha256 =
            AuthoringSha256Digest::from_bytes([0xc1; 32]);
        ref_drift.canonical_project_json = ref_drift.project.to_canonical_json().unwrap();
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, ref_drift),
            Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                Revision3QuestDraftPersistenceValidationErrorV3::QuestEntityMismatch
            ))
        ));

        let mut advanced_project = project.clone();
        advanced_project.revision += 1;
        advanced_project.meta.name.push_str(" advanced");
        let advanced_head = advance_revision3_head(&root, &store, &head, &advanced_project);
        let stale = apply_v3(
            &WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap(),
            &advanced_head,
            &catalog,
            &advanced_project,
            &request_v3(&advanced_project, &advanced_head, 2),
        );
        let stale_artifact = stale.collision_artifact;
        let mut foreign_artifact_outcome = apply_v3(
            &store,
            &advanced_head,
            &catalog,
            &advanced_project,
            &request_v3(&advanced_project, &advanced_head, 3),
        );
        foreign_artifact_outcome.collision_artifact = stale_artifact;
        // The fixed head is still the advanced one; a deliberately stale basis token fails before
        // immutable artifact installation.
        foreign_artifact_outcome.basis_head = head.clone();
        let raw = authoring_seal(foreign_artifact_outcome.collision_artifact.artifact_seal());
        assert!(matches!(
            prepare_revision3_quest_draft_persistence_v3(&store, foreign_artifact_outcome),
            Err(Revision3QuestDraftPersistenceErrorV3::Validation(
                Revision3QuestDraftPersistenceValidationErrorV3::BasisHeadMismatch
            ))
        ));
        assert!(!stored_asset_path(&root, raw.sha256).exists());
        assert_eq!(
            store
                .open_current_revision3(AssetVerification::Full)
                .unwrap()
                .head,
            advanced_head
        );
    }

    #[test]
    fn revision3_v3_persistence_head_race_after_import_leaves_verified_artifact_orphan() {
        let root = S3TestRoot::new("v3-persist-after-import-race");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let request = request_v3(&project, &head, 1);
        let outcome = apply_v3(&store, &head, &catalog, &project, &request);
        let artifact = authoring_seal(outcome.collision_artifact.artifact_seal());
        let mut raced_project = project.clone();
        raced_project.revision += 1;
        raced_project.meta.name.push_str(" raced");
        let raced = store
            .prepare_revision3_checkpoint(Some(&head), &raced_project)
            .unwrap();

        let result = crate::revision3_quest_persistence_v3::prepare_revision3_quest_draft_persistence_v3_with_after_import_hook(
            &store,
            outcome,
            || {
                fs::write(root.0.join("gore-project.json"), &raced.head_bytes)?;
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3QuestDraftPersistenceErrorV3::Store(
                WorkingStoreError::HeadConflict {
                    expected: Some(expected),
                    actual: Some(actual),
                }
            )) if expected == head && actual == raced.head
        ));
        assert!(stored_asset_path(&root, artifact.sha256).is_file());
        assert_eq!(
            store
                .open_current_revision3(AssetVerification::Full)
                .unwrap()
                .project,
            raced_project
        );
    }

    #[test]
    fn revision3_v3_transport_request_and_exact_binding_fail_closed_before_mutation() {
        let root = S3TestRoot::new("v3-transport-binding");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let canonical_project = project.to_canonical_json().unwrap();
        let request = request_v3(&project, &head, 1);
        let canonical_request = request.to_canonical_json().unwrap();

        let evaluate = |project_json: &str, request_json: &str| {
            apply_revision3_quest_draft_transaction_v3(
                prepare_v3_transaction_artifact(&store, &head, &catalog),
                project_json,
                request_json,
            )
            .unwrap_err()
        };

        assert!(matches!(
            evaluate(&"x".repeat(MAX_PROJECT_JSON_BYTES + 1), &canonical_request),
            Revision3QuestDraftInsertErrorV3::ProjectTransport(
                Revision3QuestDraftProjectTransportErrorV3::InputTooLarge { .. }
            )
        ));
        assert!(matches!(
            evaluate(&(canonical_project.clone() + "\n"), &canonical_request),
            Revision3QuestDraftInsertErrorV3::ProjectTransport(
                Revision3QuestDraftProjectTransportErrorV3::CurrentProjectSealMismatch
            )
        ));
        let duplicate_project = canonical_project.replacen(
            "{\"format\":",
            &format!("{{\"revision\":{},\"format\":", project.revision),
            1,
        );
        assert!(matches!(
            evaluate(&duplicate_project, &canonical_request),
            Revision3QuestDraftInsertErrorV3::ProjectTransport(
                Revision3QuestDraftProjectTransportErrorV3::CurrentProjectSealMismatch
            )
        ));
        let mut other_valid_project = project.clone();
        other_valid_project.meta.name.push_str(" seal drift");
        assert!(matches!(
            evaluate(
                &other_valid_project.to_canonical_json().unwrap(),
                &canonical_request
            ),
            Revision3QuestDraftInsertErrorV3::ProjectTransport(
                Revision3QuestDraftProjectTransportErrorV3::CurrentProjectSealMismatch
            )
        ));

        assert!(matches!(
            evaluate(&canonical_project, &(canonical_request.clone() + "\n")),
            Revision3QuestDraftInsertErrorV3::Request(
                Revision3QuestDraftInsertRequestJsonErrorV3::NonCanonicalJson
            )
        ));
        let duplicate_request = canonical_request.replacen(
            "{\"expected_head\":",
            &format!(
                "{{\"expected_revision\":{},\"expected_head\":",
                project.revision
            ),
            1,
        );
        assert!(matches!(
            evaluate(&canonical_project, &duplicate_request),
            Revision3QuestDraftInsertErrorV3::Request(
                Revision3QuestDraftInsertRequestJsonErrorV3::InvalidJson(_)
            )
        ));
        let forged_parent =
            canonical_request.replacen("\"intent\":{", "\"parent_quest\":{},\"intent\":{", 1);
        assert!(matches!(
            evaluate(&canonical_project, &forged_parent),
            Revision3QuestDraftInsertErrorV3::Request(
                Revision3QuestDraftInsertRequestJsonErrorV3::InvalidJson(_)
            )
        ));

        let mut wrong_head = request.clone();
        wrong_head.expected_head.snapshot.sha256 = AuthoringSha256Digest::from_bytes([0xa1; 32]);
        assert!(matches!(
            evaluate(&canonical_project, &wrong_head.to_canonical_json().unwrap()),
            Revision3QuestDraftInsertErrorV3::Binding(
                Revision3QuestDraftBindingErrorV3::CurrentHeadMismatch
            )
        ));
        let mut wrong_id = request.clone();
        wrong_id.expected_project_id = authoring_project_id(0xa2);
        assert!(matches!(
            evaluate(&canonical_project, &wrong_id.to_canonical_json().unwrap()),
            Revision3QuestDraftInsertErrorV3::Binding(
                Revision3QuestDraftBindingErrorV3::ProjectIdentityMismatch
            )
        ));
        let mut wrong_revision = request;
        wrong_revision.expected_revision += 1;
        assert!(matches!(
            evaluate(
                &canonical_project,
                &wrong_revision.to_canonical_json().unwrap()
            ),
            Revision3QuestDraftInsertErrorV3::Binding(
                Revision3QuestDraftBindingErrorV3::ProjectRevisionMismatch
            )
        ));
        assert_eq!(project.to_canonical_json().unwrap(), canonical_project);
    }

    #[test]
    fn revision3_v3_checks_module_path_and_each_of_five_symbols_against_exact_union() {
        let root = S3TestRoot::new("v3-seven-generated-collisions");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let catalog = trusted_catalog();
        let project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let request = request_v3(&project, &head, 1);
        let project_json = project.to_canonical_json().unwrap();
        let request_json = request.to_canonical_json().unwrap();
        let collisions = [
            (
                "module",
                "GoreMods.Quests.AuthorityQuest1",
                Revision3StoryIdentityKindV3::ModuleNamespace,
            ),
            (
                "relative path",
                "GoreMods/Quests/AuthorityQuest1.as",
                Revision3StoryIdentityKindV3::ModuleRelativePath,
            ),
            (
                "symbol",
                "UQuest_GORE_AUTHORITY_QUEST_1",
                Revision3StoryIdentityKindV3::GeneratedSymbol,
            ),
            (
                "symbol",
                "UQuest_GORE_AUTHORITY_QUEST_1_OBJ_DONE",
                Revision3StoryIdentityKindV3::GeneratedSymbol,
            ),
            (
                "symbol",
                "GoreAuthorityQuest1Text",
                Revision3StoryIdentityKindV3::GeneratedSymbol,
            ),
            (
                "symbol",
                "GetGoreAuthorityQuest1",
                Revision3StoryIdentityKindV3::GeneratedSymbol,
            ),
            (
                "symbol",
                "GetGoreAuthorityQuest1Objective",
                Revision3StoryIdentityKindV3::GeneratedSymbol,
            ),
        ];

        for (domain, value, expected_kind) in collisions {
            let prepared = VerifiedRevision3QuestCollisionCapabilityV2::bind(
                artifact_with_extra_collision(domain, &value.to_ascii_lowercase()),
                &catalog,
                store
                    .prepare_current_revision3_quest_collision_source_v2(&head)
                    .unwrap(),
            )
            .unwrap()
            .prepare_artifact()
            .unwrap();
            assert!(matches!(
                apply_revision3_quest_draft_transaction_v3(
                    prepared,
                    &project_json,
                    &request_json
                ),
                Err(Revision3QuestDraftInsertErrorV3::Conflict(
                    Revision3QuestDraftConflictV3::StoryIdentityCollision { kind, value: actual }
                )) if kind == expected_kind && actual == value
            ));
        }
    }

    #[test]
    fn revision3_v3_rejects_runtime_catalog_entity_revision_and_intent_conflicts_atomically() {
        let catalog = trusted_catalog();

        let runtime_root = S3TestRoot::new("v3-runtime-collision");
        let runtime_store =
            WorkingProjectStore::at(&runtime_root.0, WorkingStoreLimits::default()).unwrap();
        let mut revision2 = empty_authoring_project();
        add_authoring_npc(
            &mut revision2,
            "Project.Npcs.RuntimeCollisionWitness",
            "GORE_AUTHORITY_QUEST_1",
        );
        let runtime_project = migrate_revision2_to_revision3(&revision2).unwrap().project;
        let runtime_head = publish_revision3_head(&runtime_root, &runtime_store, &runtime_project);
        let runtime_request = request_v3(&runtime_project, &runtime_head, 1);
        assert!(matches!(
            apply_revision3_quest_draft_transaction_v3(
                prepare_v3_transaction_artifact(&runtime_store, &runtime_head, &catalog),
                &runtime_project.to_canonical_json().unwrap(),
                &runtime_request.to_canonical_json().unwrap(),
            ),
            Err(Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::RuntimeIdentityCollision { .. }
            ))
        ));

        let nonstory_root = S3TestRoot::new("v3-nonstory-runtime-is-not-collision");
        let nonstory_store =
            WorkingProjectStore::at(&nonstory_root.0, WorkingStoreLimits::default()).unwrap();
        let mut nonstory_revision2 = empty_authoring_project();
        let localization_id = authoring_entity_id(0x61);
        nonstory_revision2.entities.insert(
            localization_id,
            AuthoringEntity {
                id: localization_id,
                display_name: "Non-Story runtime witness".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_AUTHORITY_QUEST_1".to_owned(),
                },
                revision: 0,
                payload: AuthoringEntityPayload::LocalizationEntry(AuthoringLocalizationEntry {
                    loc_id: "GORE_NONSTORY_RUNTIME_WITNESS".to_owned(),
                    texts: Default::default(),
                }),
            },
        );
        let nonstory_project = migrate_revision2_to_revision3(&nonstory_revision2)
            .unwrap()
            .project;
        let nonstory_head =
            publish_revision3_head(&nonstory_root, &nonstory_store, &nonstory_project);
        let nonstory_request = request_v3(&nonstory_project, &nonstory_head, 1);
        let nonstory_outcome = apply_revision3_quest_draft_transaction_v3(
            prepare_v3_transaction_artifact(&nonstory_store, &nonstory_head, &catalog),
            &nonstory_project.to_canonical_json().unwrap(),
            &nonstory_request.to_canonical_json().unwrap(),
        )
        .unwrap();
        assert!(nonstory_outcome
            .project
            .entities
            .contains_key(&nonstory_request.quest_id));

        let root = S3TestRoot::new("v3-semantic-conflicts");
        let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
        let mut project = migrate_revision2_to_revision3(&empty_authoring_project())
            .unwrap()
            .project;
        let head = publish_revision3_head(&root, &store, &project);
        let project_json = project.to_canonical_json().unwrap();
        let evaluate = |request: &Revision3QuestDraftInsertRequestV3| {
            apply_revision3_quest_draft_transaction_v3(
                prepare_v3_transaction_artifact(&store, &head, &catalog),
                &project_json,
                &request.to_canonical_json().unwrap(),
            )
            .unwrap_err()
        };

        let mut unknown_parent = request_v3(&project, &head, 1);
        unknown_parent.intent.parent_catalog_id = "g1r:quest-parent:missing".to_owned();
        assert!(matches!(
            evaluate(&unknown_parent),
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::CatalogSelection(
                    Revision3QuestCollisionCapabilityErrorV2::UnknownParent(_)
                )
            )
        ));
        let mut unknown_giver = request_v3(&project, &head, 1);
        unknown_giver.intent.giver_catalog_id = "g1r:npc:missing".to_owned();
        assert!(matches!(
            evaluate(&unknown_giver),
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::CatalogSelection(
                    Revision3QuestCollisionCapabilityErrorV2::UnknownGiver(_)
                )
            )
        ));
        let mut zero = request_v3(&project, &head, 1);
        zero.quest_id = authoring_entity_id(0);
        assert!(matches!(
            evaluate(&zero),
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::ZeroEntityId { .. }
            )
        ));
        let mut shared = request_v3(&project, &head, 1);
        shared.script_module_id = shared.quest_id;
        assert!(matches!(
            evaluate(&shared),
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::SharedEntityId
            )
        ));
        let mut invalid_intent = request_v3(&project, &head, 1);
        invalid_intent.intent.technical_id = "not canonical".to_owned();
        assert!(matches!(
            evaluate(&invalid_intent),
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::InvalidQuestIntent { .. }
            )
        ));

        project.revision = u64::MAX;
        let overflow_root = S3TestRoot::new("v3-revision-overflow");
        let overflow_store =
            WorkingProjectStore::at(&overflow_root.0, WorkingStoreLimits::default()).unwrap();
        let overflow_head = publish_revision3_head(&overflow_root, &overflow_store, &project);
        let overflow_request = request_v3(&project, &overflow_head, 1);
        assert!(matches!(
            apply_revision3_quest_draft_transaction_v3(
                prepare_v3_transaction_artifact(&overflow_store, &overflow_head, &catalog),
                &project.to_canonical_json().unwrap(),
                &overflow_request.to_canonical_json().unwrap(),
            ),
            Err(Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::ProjectRevisionOverflow
            ))
        ));
    }

    #[test]
    fn revision3_context_edit_description_parent_giver_and_all_have_an_exact_closed_delta() {
        let fixture = s3_fixture("context-edit-delta");
        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
        let before = &fixture.project;
        let Revision3EntityPayload::QuestDraft(basis_quest) =
            &before.entities[&fixture.quest_id].payload
        else {
            panic!("expected context-edit Quest")
        };
        let basis_parent = basis_quest.input.parent_quest.clone();
        let basis_giver = basis_quest.input.giver.clone();
        let alternate_parent = alternate_test_parent(before, fixture.quest_id);
        let viper = fixture
            .fresh_capability()
            .resolve_giver("g1r:npc:om_stt_viper_302")
            .unwrap();

        let mut description_request =
            context_edit_request_v1(before, &head, &fixture.catalog, fixture.quest_id);
        description_request.description = "Only the description changed.".to_owned();
        let description_outcome = apply_revision3_quest_context_edit_transaction_v1(
            prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None),
            &before.to_canonical_json().unwrap(),
            &description_request.to_canonical_json().unwrap(),
        )
        .unwrap();
        assert_context_edit_exact_delta(
            before,
            &description_outcome,
            fixture.quest_id,
            &description_request.description,
            &basis_parent,
            &basis_giver,
        );

        let mut parent_request =
            context_edit_request_v1(before, &head, &fixture.catalog, fixture.quest_id);
        parent_request.parent_catalog_id = "g1r:quest-parent:test-alternate".to_owned();
        let parent_outcome = apply_revision3_quest_context_edit_transaction_v1(
            prepare_context_edit_artifact_v1(
                &fixture.store,
                &head,
                &fixture.catalog,
                Some(alternate_parent.clone()),
            ),
            &before.to_canonical_json().unwrap(),
            &parent_request.to_canonical_json().unwrap(),
        )
        .unwrap();
        assert_context_edit_exact_delta(
            before,
            &parent_outcome,
            fixture.quest_id,
            &parent_request.description,
            &alternate_parent,
            &basis_giver,
        );

        let mut giver_request =
            context_edit_request_v1(before, &head, &fixture.catalog, fixture.quest_id);
        giver_request.giver_catalog_id = "g1r:npc:om_stt_viper_302".to_owned();
        let giver_outcome = apply_revision3_quest_context_edit_transaction_v1(
            prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None),
            &before.to_canonical_json().unwrap(),
            &giver_request.to_canonical_json().unwrap(),
        )
        .unwrap();
        assert_context_edit_exact_delta(
            before,
            &giver_outcome,
            fixture.quest_id,
            &giver_request.description,
            &basis_parent,
            &viper,
        );

        let mut all_request =
            context_edit_request_v1(before, &head, &fixture.catalog, fixture.quest_id);
        all_request.description = "Description, parent, and giver changed together.".to_owned();
        all_request.parent_catalog_id = "g1r:quest-parent:test-alternate".to_owned();
        all_request.giver_catalog_id = "g1r:npc:om_stt_viper_302".to_owned();
        let all_outcome = apply_revision3_quest_context_edit_transaction_v1(
            prepare_context_edit_artifact_v1(
                &fixture.store,
                &head,
                &fixture.catalog,
                Some(alternate_parent.clone()),
            ),
            &before.to_canonical_json().unwrap(),
            &all_request.to_canonical_json().unwrap(),
        )
        .unwrap();
        assert_context_edit_exact_delta(
            before,
            &all_outcome,
            fixture.quest_id,
            &all_request.description,
            &alternate_parent,
            &viper,
        );
        assert_eq!(all_outcome.basis_head(), &head);
        assert_eq!(all_outcome.quest_id(), fixture.quest_id);
        assert_eq!(all_outcome.script_module_id(), all_outcome.module_id());
        assert_eq!(all_outcome.quest_revision(), 1);
        assert_eq!(all_outcome.script_module_revision(), 1);
        assert_eq!(
            all_outcome.script_module_revision(),
            all_outcome.module_revision()
        );
        assert_eq!(
            all_outcome.build_status(),
            Revision3QuestContextEditBuildStatusV1::Blocked
        );
        assert_eq!(
            all_outcome.runtime_status(),
            Revision3QuestContextEditRuntimeStatusV1::RuntimeUnqualified
        );
        assert_eq!(
            all_outcome.publication_status(),
            Revision3QuestContextEditPublicationStatusV1::NotSupported
        );
        assert_eq!(
            gore_authoring::ProjectRevision3::from_json(all_outcome.canonical_project_json())
                .unwrap(),
            *all_outcome.project()
        );
    }

    #[test]
    fn revision3_context_edit_preserves_one_and_eight_objective_shapes() {
        for objective_count in [1usize, 8] {
            let mut fixture = s3_fixture(&format!("context-edit-objectives-{objective_count}"));
            if objective_count == 8 {
                let module_id = {
                    let quest = s3_quest_mut(&mut fixture.project, fixture.quest_id);
                    quest.generator_version = REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION;
                    quest.input.additional_objective_titles = (2..=8)
                        .map(|ordinal| format!("Objective {ordinal}"))
                        .collect();
                    quest.script_module.id
                };
                let Revision3OriginRef::Generated {
                    generator_version, ..
                } = &mut fixture.project.entities.get_mut(&module_id).unwrap().origin
                else {
                    panic!("expected generated multi-objective module origin")
                };
                *generator_version = REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION;
                regenerate_s3_pair_after_intent_change(&mut fixture.project, fixture.quest_id);
            }
            let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
            let before = fixture.project.clone();
            let Revision3EntityPayload::QuestDraft(before_quest) =
                &before.entities[&fixture.quest_id].payload
            else {
                panic!("expected context-edit Quest")
            };
            assert_eq!(
                1 + before_quest.input.additional_objective_titles.len(),
                objective_count
            );
            let expected_parent = before_quest.input.parent_quest.clone();
            let expected_giver = before_quest.input.giver.clone();
            let mut request =
                context_edit_request_v1(&before, &head, &fixture.catalog, fixture.quest_id);
            request.description = format!("Edit a Quest with {objective_count} objectives.");
            let outcome = apply_revision3_quest_context_edit_transaction_v1(
                prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None),
                &before.to_canonical_json().unwrap(),
                &request.to_canonical_json().unwrap(),
            )
            .unwrap();
            assert_context_edit_exact_delta(
                &before,
                &outcome,
                fixture.quest_id,
                &request.description,
                &expected_parent,
                &expected_giver,
            );
            let Revision3EntityPayload::QuestDraft(after_quest) =
                &outcome.project().entities[&fixture.quest_id].payload
            else {
                panic!("expected edited Quest")
            };
            assert_eq!(
                after_quest.input.additional_objective_titles,
                before_quest.input.additional_objective_titles
            );
        }
    }

    #[test]
    fn revision3_context_edit_preserves_semantic_v4_transition_plan() {
        let mut fixture = s3_fixture("context-edit-semantic-v4");
        let transition_plan = QuestTransitionPlanV1::legacy_seed(1).unwrap();
        let module_id = {
            let quest = s3_quest_mut(&mut fixture.project, fixture.quest_id);
            quest.generator_version = REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION;
            quest.input.transition_plan = Some(Box::new(transition_plan.clone()));
            quest.script_module.id
        };
        let Revision3OriginRef::Generated {
            generator_version, ..
        } = &mut fixture.project.entities.get_mut(&module_id).unwrap().origin
        else {
            panic!("expected generated semantic module origin")
        };
        *generator_version = REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION;
        regenerate_s3_pair_after_intent_change(&mut fixture.project, fixture.quest_id);

        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
        let before = fixture.project.clone();
        let Revision3EntityPayload::QuestDraft(before_quest) =
            &before.entities[&fixture.quest_id].payload
        else {
            panic!("expected semantic Quest")
        };
        let expected_parent = before_quest.input.parent_quest.clone();
        let expected_giver = before_quest.input.giver.clone();
        let mut request =
            context_edit_request_v1(&before, &head, &fixture.catalog, fixture.quest_id);
        request.description = "Edit context without changing lifecycle semantics.".to_owned();

        let outcome = apply_revision3_quest_context_edit_transaction_v1(
            prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None),
            &before.to_canonical_json().unwrap(),
            &request.to_canonical_json().unwrap(),
        )
        .unwrap();
        assert_context_edit_exact_delta(
            &before,
            &outcome,
            fixture.quest_id,
            &request.description,
            &expected_parent,
            &expected_giver,
        );
        let Revision3EntityPayload::QuestDraft(after_quest) =
            &outcome.project().entities[&fixture.quest_id].payload
        else {
            panic!("expected edited semantic Quest")
        };
        assert_eq!(
            after_quest.generator_version,
            REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        );
        assert_eq!(
            after_quest.input.transition_plan.as_deref(),
            Some(&transition_plan)
        );
    }

    #[test]
    fn revision3_context_edit_rejects_stale_unknown_noop_invalid_and_transport_inputs() {
        let fixture = s3_fixture("context-edit-conflicts");
        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
        let project_json = fixture.project.to_canonical_json().unwrap();
        let base_request =
            context_edit_request_v1(&fixture.project, &head, &fixture.catalog, fixture.quest_id);
        let evaluate = |request: &Revision3QuestContextEditRequestV1| {
            apply_revision3_quest_context_edit_transaction_v1(
                prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None),
                &project_json,
                &request.to_canonical_json().unwrap(),
            )
            .unwrap_err()
        };

        assert!(matches!(
            evaluate(&base_request),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::NoChanges
            )
        ));
        let mut stale_head = base_request.clone();
        stale_head.expected_head.snapshot.sha256 = AuthoringSha256Digest::from_bytes([0xa1; 32]);
        assert!(matches!(
            evaluate(&stale_head),
            Revision3QuestContextEditErrorV1::Binding(
                Revision3QuestContextEditBindingErrorV1::CurrentHeadMismatch
            )
        ));
        let mut stale_project_id = base_request.clone();
        stale_project_id.expected_project_id = authoring_project_id(0xa2);
        assert!(matches!(
            evaluate(&stale_project_id),
            Revision3QuestContextEditErrorV1::Binding(
                Revision3QuestContextEditBindingErrorV1::ProjectIdentityMismatch
            )
        ));
        let mut stale_revision = base_request.clone();
        stale_revision.expected_revision += 1;
        assert!(matches!(
            evaluate(&stale_revision),
            Revision3QuestContextEditErrorV1::Binding(
                Revision3QuestContextEditBindingErrorV1::ProjectRevisionMismatch
            )
        ));
        let mut stale_catalog = base_request.clone();
        stale_catalog.expected_story_catalog_seal.byte_len += 1;
        assert!(matches!(
            evaluate(&stale_catalog),
            Revision3QuestContextEditErrorV1::Binding(
                Revision3QuestContextEditBindingErrorV1::StoryCatalogSealMismatch
            )
        ));
        let mut stale_quest = base_request.clone();
        stale_quest.expected_quest_revision += 1;
        assert!(matches!(
            evaluate(&stale_quest),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::QuestRevisionConflict { .. }
            )
        ));
        let mut unknown_quest = base_request.clone();
        unknown_quest.quest_id = authoring_entity_id(0xee);
        assert!(matches!(
            evaluate(&unknown_quest),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::InvalidQuestEntity { .. }
            )
        ));
        let mut zero_quest = base_request.clone();
        zero_quest.quest_id = authoring_entity_id(0);
        assert!(matches!(
            evaluate(&zero_quest),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::ZeroQuestId
            )
        ));
        let mut unknown_parent = base_request.clone();
        unknown_parent.parent_catalog_id = "g1r:quest-parent:missing".to_owned();
        assert!(matches!(
            evaluate(&unknown_parent),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::CatalogSelection(
                    Revision3QuestCollisionCapabilityErrorV2::UnknownParent(_)
                )
            )
        ));
        let mut unknown_giver = base_request.clone();
        unknown_giver.giver_catalog_id = "g1r:npc:missing".to_owned();
        assert!(matches!(
            evaluate(&unknown_giver),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::CatalogSelection(
                    Revision3QuestCollisionCapabilityErrorV2::UnknownGiver(_)
                )
            )
        ));
        let mut invalid_description = base_request.clone();
        invalid_description.description = "x".repeat(513);
        assert!(matches!(
            evaluate(&invalid_description),
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::InvalidQuestContext { .. }
            )
        ));

        assert!(matches!(
            apply_revision3_quest_context_edit_transaction_v1(
                prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None,),
                &(project_json.clone() + "\n"),
                &base_request.to_canonical_json().unwrap(),
            ),
            Err(Revision3QuestContextEditErrorV1::ProjectTransport(
                Revision3QuestContextEditProjectTransportErrorV1::CurrentProjectSealMismatch
            ))
        ));
        assert!(matches!(
            apply_revision3_quest_context_edit_transaction_v1(
                prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None,),
                &"x".repeat(MAX_PROJECT_JSON_BYTES + 1),
                &base_request.to_canonical_json().unwrap(),
            ),
            Err(Revision3QuestContextEditErrorV1::ProjectTransport(
                Revision3QuestContextEditProjectTransportErrorV1::InputTooLarge { .. }
            ))
        ));
        assert!(matches!(
            apply_revision3_quest_context_edit_transaction_v1(
                prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None,),
                &project_json,
                &"x".repeat(MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1 + 1),
            ),
            Err(Revision3QuestContextEditErrorV1::Request(
                Revision3QuestContextEditRequestJsonErrorV1::InputTooLarge { .. }
            ))
        ));
        assert_eq!(fixture.project.to_canonical_json().unwrap(), project_json);
    }

    #[test]
    fn revision3_context_edit_rejects_all_revision_overflows() {
        #[derive(Clone, Copy)]
        enum OverflowKind {
            Project,
            Quest,
            Module,
        }
        for kind in [
            OverflowKind::Project,
            OverflowKind::Quest,
            OverflowKind::Module,
        ] {
            let label = match kind {
                OverflowKind::Project => "project",
                OverflowKind::Quest => "quest",
                OverflowKind::Module => "module",
            };
            let mut fixture = s3_fixture(&format!("context-edit-overflow-{label}"));
            let module_id = {
                let Revision3EntityPayload::QuestDraft(quest) =
                    &fixture.project.entities[&fixture.quest_id].payload
                else {
                    panic!("expected overflow Quest")
                };
                quest.script_module.id
            };
            match kind {
                OverflowKind::Project => fixture.project.revision = u64::MAX,
                OverflowKind::Quest => {
                    fixture
                        .project
                        .entities
                        .get_mut(&fixture.quest_id)
                        .unwrap()
                        .revision = u64::MAX;
                }
                OverflowKind::Module => {
                    fixture
                        .project
                        .entities
                        .get_mut(&module_id)
                        .unwrap()
                        .revision = u64::MAX;
                }
            }
            let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
            let mut request = context_edit_request_v1(
                &fixture.project,
                &head,
                &fixture.catalog,
                fixture.quest_id,
            );
            request.description = format!("Trigger {label} revision overflow.");
            let error = apply_revision3_quest_context_edit_transaction_v1(
                prepare_context_edit_artifact_v1(&fixture.store, &head, &fixture.catalog, None),
                &fixture.project.to_canonical_json().unwrap(),
                &request.to_canonical_json().unwrap(),
            )
            .unwrap_err();
            match kind {
                OverflowKind::Project => assert!(matches!(
                    error,
                    Revision3QuestContextEditErrorV1::Conflict(
                        Revision3QuestContextEditConflictV1::ProjectRevisionOverflow
                    )
                )),
                OverflowKind::Quest => assert!(matches!(
                    error,
                    Revision3QuestContextEditErrorV1::Conflict(
                        Revision3QuestContextEditConflictV1::QuestRevisionOverflow { .. }
                    )
                )),
                OverflowKind::Module => assert!(matches!(
                    error,
                    Revision3QuestContextEditErrorV1::Conflict(
                        Revision3QuestContextEditConflictV1::ScriptModuleRevisionOverflow { .. }
                    )
                )),
            }
        }
    }

    #[test]
    fn revision3_context_edit_cannot_acquire_authority_for_a_drifted_owned_module() {
        let mut fixture = s3_fixture("context-edit-module-drift");
        let module_id = {
            let Revision3EntityPayload::QuestDraft(quest) =
                &fixture.project.entities[&fixture.quest_id].payload
            else {
                panic!("expected drift Quest")
            };
            quest.script_module.id
        };
        let Revision3EntityPayload::ScriptModule(module) = &mut fixture
            .project
            .entities
            .get_mut(&module_id)
            .unwrap()
            .payload
        else {
            panic!("expected drift ScriptModule")
        };
        module.source.push_str("// adversarial drift\n");
        module.source_sha256 = AuthoringSha256Digest::from_bytes(
            <Sha256 as sha2::Digest>::digest(module.source.as_bytes()).into(),
        );
        fixture.project.validate_closed_model().unwrap();
        let head = publish_revision3_head(&fixture._root, &fixture.store, &fixture.project);
        assert!(matches!(
            fixture
                .store
                .prepare_current_revision3_quest_collision_source_v2(&head),
            Err(gore_authoring::Revision3QuestCollisionSourceErrorV2::PersistedModuleDrift {
                quest,
                module,
                ..
            }) if quest == fixture.quest_id && module == module_id
        ));
    }

    #[test]
    #[ignore = "requires exact pinned executable, Shipping Cache, and Binds Cache paths"]
    fn configured_real_combined_artifact_golden() {
        let executable = std::env::var_os("GORE_STORY_INVENTORY_REAL_EXE")
            .expect("set GORE_STORY_INVENTORY_REAL_EXE for the ignored real golden");
        let shipping_path = std::env::var_os("GORE_STORY_INVENTORY_REAL_SHIPPING")
            .expect("set GORE_STORY_INVENTORY_REAL_SHIPPING for the ignored real golden");
        let binds_path = std::env::var_os("GORE_STORY_INVENTORY_REAL_BINDS")
            .expect("set GORE_STORY_INVENTORY_REAL_BINDS for the ignored real golden");
        let shipping = std::fs::read(&shipping_path).unwrap();
        let binds = std::fs::read(&binds_path).unwrap();
        let catalog = gore_story_catalog::build_known_catalog_with_shipping_snapshot(
            std::path::Path::new(&executable),
            &shipping,
            std::path::Path::new(&binds_path),
            gore_story_catalog::GenerationInputLimits::default(),
        )
        .unwrap();
        let base = build_base_game_inventory(&catalog, &shipping, &binds).unwrap();
        assert_eq!(base.payload_seal().byte_len, 3_517_746);

        let project = empty_authoring_project();
        let artifact = VerifiedQuestCollisionCapability::bind(base, &catalog, &project)
            .unwrap()
            .into_artifact()
            .unwrap();
        assert_eq!(artifact.canonical_json().len(), 3_517_569);
        assert_eq!(
            artifact.artifact_seal().sha256.to_string(),
            "89d87887f531e6ea837bf4f00adcb987cb85f8bec5afb2b07d1343b8b407422f"
        );
        assert_eq!(
            artifact.source_seal().sha256.to_string(),
            "945f60dd495fad5bb19864ba270566b64325751c82607309b99e7ec71ad2f8f9"
        );
        let reopened = reopen_quest_collision_capability_artifact_v1(
            artifact.canonical_json(),
            artifact.artifact_seal(),
            artifact.source_seal(),
        )
        .unwrap();
        let fresh_base = build_base_game_inventory(&catalog, &shipping, &binds).unwrap();
        let authoritative = VerifiedQuestCollisionCapability::bind(fresh_base, &catalog, &project)
            .unwrap()
            .verify_artifact_exact(&reopened)
            .unwrap();
        assert_eq!(authoritative.combined_source_seal(), artifact.source_seal());
    }
}
