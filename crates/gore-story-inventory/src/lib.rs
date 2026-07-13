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

pub use quest_capability::{
    reopen_quest_collision_capability_artifact_v1, QuestCollisionBuildStatus,
    QuestCollisionCapabilityArtifactError, QuestCollisionCapabilityArtifactV1,
    QuestCollisionCapabilityArtifactVerificationError, QuestCollisionCapabilityError,
    QuestCollisionCoverage, QuestCollisionPublicationStatus, QuestCollisionRuntimeQualification,
    VerifiedQuestCollisionCapability, BASE_GAME_AND_EXACT_PROJECT_COLLISION_LAYER,
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
        EntityPayload as AuthoringEntityPayload, NpcDraft as AuthoringNpcDraft,
        NpcDraftInput as AuthoringNpcDraftInput, NpcParentClassInput, OriginRef, ProjectRevision2,
        SchemaRevisionV2, TypedRef,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal as AuthoringContentSeal, EntityId as AuthoringEntityId,
        FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        Sha256Digest as AuthoringSha256Digest, LOGICAL_NPC_CLONE_GENERATOR_ID,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };
    use std::collections::BTreeSet;

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
