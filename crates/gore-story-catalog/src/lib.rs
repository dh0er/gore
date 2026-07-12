//! Strict, generation-sealed story catalogs.
//!
//! The first revision intentionally supports one curated Gothic 1 Remake Steam generation. It
//! hashes explicit executable, Shipping cache, and Binds cache inputs, then selects only extraction
//! records reviewed against that exact triple. It does **not** infer broad NPC/quest semantics from
//! filenames or the old UE4SS object-dump catalog: generated `__InitDefaults` carry the decisive
//! links and quest metadata. A generalized cache extractor remains a separate future step.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

pub const MAX_CATALOG_JSON_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RECORD_SET_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_NPCS: usize = 2;
pub const MAX_QUEST_PARENTS: usize = 1;
pub const MAX_TEXT_BYTES: usize = 4 * 1024;
pub const AUTHORING_SELECTION_SCHEMA_REVISION: u32 = 1;
pub const QUEST_COLLISION_CATALOG_LAYER: &str = "resolved-loadout.scripts.v1";

const STORY_FORMAT: &str = "story_catalog";
const STORY_SCHEMA_REVISION: u32 = 1;
const RECORD_SET_ID: &str = "g1r-steam-1.0.3-curated-story-v1";
const RECORD_SET_BYTE_LEN: u64 = 5_499;
const RECORD_SET_SHA256: &str = "323ffe3fb3d6394c0d4397d090aabddb5e87c1ac7e5cecd14382b0a4f0516fc8";
const CATALOG_PAYLOAD_BYTE_LEN: u64 = 5_611;
const CATALOG_PAYLOAD_SHA256: &str =
    "51192393aa28cff00b1a4e59de7793a8db354e30692569719c4b46e2f9bc4853";
const VIPER_OFFLINE_EVIDENCE_ID: &str = concat!(
    "npc-logical-clone-v1:viper-current-v1:proof-format-v1:",
    "sha256-b65b551f1f7d0c783c982250c87934287141cc3bf29013ba58c9cdce5852e68a"
);
const BASE_GAME_LAYER: &str = "base-game.g1r.scripts";
const COPY_BUFFER_BYTES: usize = 64 * 1024;
const MAX_EXE_BYTES_HARD: u64 = 512 * 1024 * 1024;
const MAX_CACHE_BYTES_HARD: u64 = 1024 * 1024 * 1024;
const MAX_BINDS_BYTES_HARD: u64 = 128 * 1024 * 1024;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StoryCatalogFormat;

impl Serialize for StoryCatalogFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(STORY_FORMAT)
    }
}

impl<'de> Deserialize<'de> for StoryCatalogFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == STORY_FORMAT {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported story catalog format {value:?}; expected {STORY_FORMAT:?}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StorySchemaRevisionV1;

impl Serialize for StorySchemaRevisionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(STORY_SCHEMA_REVISION)
    }
}

impl<'de> Deserialize<'de> for StorySchemaRevisionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == STORY_SCHEMA_REVISION {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported story catalog schema revision {value}; expected {STORY_SCHEMA_REVISION}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_hex(value: &str) -> Result<Self, CatalogError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(CatalogError::Invariant(format!(
                "SHA-256 must be exactly 64 lowercase hexadecimal characters, got {value:?}"
            )));
        }
        let mut bytes = [0u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
        }
        Ok(Self(bytes))
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Sha256Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DigestVisitor;

        impl Visitor<'_> for DigestVisitor {
            type Value = Sha256Digest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a 64-character lowercase SHA-256 digest")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Sha256Digest::from_hex(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(DigestVisitor)
    }
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("validated lowercase hexadecimal byte"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSeal {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

impl std::fmt::Display for ContentSeal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} bytes / sha256 {}",
            self.byte_len, self.sha256
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameGenerationSeal {
    pub edition: String,
    pub executable: ContentSeal,
    pub shipping_cache: ContentSeal,
    pub binds_cache: ContentSeal,
}

impl std::fmt::Display for GameGenerationSeal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} [exe: {}; Shipping cache: {}; Binds cache: {}]",
            self.edition, self.executable, self.shipping_cache, self.binds_cache
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SourceEvidenceKind {
    SealedEmittedSourceAndCacheDefaultsV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogClassRef {
    catalog_layer: String,
    canonical_selector: String,
    module: String,
    relative_path: String,
    class_name: String,
    source_seal: ContentSeal,
    evidence_kind: SourceEvidenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NpcDiscoveryStatus {
    SealedCacheDefaultsVerified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NpcAuthoringQualification {
    OfflineQualified,
    CatalogObserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RuntimeQualification {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum QuestParentRole {
    Chapter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum QuestParentQualification {
    CuratedDefaultsVerified,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct BuildBlocked;

impl Serialize for BuildBlocked {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for BuildBlocked {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if bool::deserialize(deserializer)? {
            Ok(Self)
        } else {
            Err(de::Error::custom(
                "story catalog revision 1 permits only blocks_build=true",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcCatalogEntry {
    catalog_id: String,
    display_name: String,
    runtime_unique_name: String,
    character_definition: CatalogClassRef,
    ai_agent_config: CatalogClassRef,
    spawn_definition: CatalogClassRef,
    discovery_status: NpcDiscoveryStatus,
    authoring_qualification: NpcAuthoringQualification,
    runtime_qualification: RuntimeQualification,
    evidence_id: String,
    blocks_build: BuildBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestParentCatalogEntry {
    catalog_id: String,
    display_name: String,
    quest_class: CatalogClassRef,
    parent_class_name: String,
    role: QuestParentRole,
    qualification: QuestParentQualification,
    transition_qualification: RuntimeQualification,
    evidence_id: String,
    blocks_build: BuildBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifiedExtractionRecords {
    record_set_id: String,
    generation: GameGenerationSeal,
    #[serde(deserialize_with = "deserialize_npcs")]
    npcs: Vec<NpcCatalogEntry>,
    #[serde(deserialize_with = "deserialize_quest_parents")]
    quest_parents: Vec<QuestParentCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoryCatalogPayload {
    generation: GameGenerationSeal,
    record_set_id: String,
    record_set_seal: ContentSeal,
    #[serde(deserialize_with = "deserialize_npcs")]
    npcs: Vec<NpcCatalogEntry>,
    #[serde(deserialize_with = "deserialize_quest_parents")]
    quest_parents: Vec<QuestParentCatalogEntry>,
}

fn deserialize_npcs<'de, D>(deserializer: D) -> Result<Vec<NpcCatalogEntry>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_sequence(deserializer, MAX_NPCS, "NPC catalog entries")
}

fn deserialize_quest_parents<'de, D>(
    deserializer: D,
) -> Result<Vec<QuestParentCatalogEntry>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_sequence(
        deserializer,
        MAX_QUEST_PARENTS,
        "quest-parent catalog entries",
    )
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoryCatalogWire {
    format: StoryCatalogFormat,
    schema_revision: StorySchemaRevisionV1,
    catalog: StoryCatalogPayload,
    catalog_seal: ContentSeal,
}

/// Closed, read-only projection used by normal authoring clients.
///
/// It deliberately omits source modules, relative paths, and raw cache bytes. `authoring_selector`
/// is a stable identifier-safe alias derived from the pinned catalog record and role; the richer
/// `source_catalog_selector` remains provenance only and must never be copied into a Draft input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringClassSelection {
    pub catalog_layer: String,
    pub authoring_selector: String,
    pub source_catalog_selector: String,
    pub runtime_class: String,
    pub source_seal: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringQuestGiverSelection {
    pub catalog_layer: String,
    pub authoring_selector: String,
    pub source_catalog_selector: String,
    pub runtime_unique_name: String,
    pub source_seal: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringNpcSelection {
    pub catalog_id: String,
    pub display_name: String,
    pub runtime_unique_name: String,
    pub character_definition: AuthoringClassSelection,
    pub ai_agent_config: AuthoringClassSelection,
    pub spawn_definition: AuthoringClassSelection,
    pub quest_giver: AuthoringQuestGiverSelection,
    pub discovery_status: String,
    pub authoring_qualification: String,
    pub runtime_qualification: String,
    pub evidence_id: String,
    pub blocks_build: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringQuestParentSelection {
    pub catalog_id: String,
    pub display_name: String,
    pub quest_class: AuthoringClassSelection,
    pub parent_class_name: String,
    pub role: String,
    pub qualification: String,
    pub transition_qualification: String,
    pub evidence_id: String,
    pub blocks_build: bool,
}

/// Honest boundary for the current catalog revision. The pinned Shipping-cache seal is available,
/// but the complete module/path/symbol collision inventory is not part of `story_catalog.v1`.
/// Consequently a normal client can populate NPC provenance and Quest giver/parent provenance,
/// but must keep Quest creation disabled until a separately sealed inventory is supplied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoringQuestCollisionCatalogAvailability {
    pub status: String,
    pub catalog_layer: String,
    pub source_seal: ContentSeal,
    pub blocks_draft_creation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoryCatalogAuthoringSelections {
    pub schema_revision: u32,
    pub generation: GameGenerationSeal,
    pub catalog_seal: ContentSeal,
    pub npcs: Vec<AuthoringNpcSelection>,
    pub quest_parents: Vec<AuthoringQuestParentSelection>,
    pub quest_collision_catalog: AuthoringQuestCollisionCatalogAvailability,
    pub blocks_build: bool,
}

/// A fully verified `story_catalog.v1` document.
///
/// This type cannot be deserialized or constructed by callers. `from_json` and `read_catalog`
/// return it only after exact generation, curated-record, canonical-byte, and seal verification.
#[derive(Debug, Clone)]
pub struct StoryCatalogFile {
    wire: StoryCatalogWire,
    input_guard: Option<GenerationInputGuard>,
}

impl StoryCatalogFile {
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, CatalogError> {
        validate_catalog_file(self)?;
        canonical_json(&self.wire, "story catalog", MAX_CATALOG_JSON_BYTES)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, CatalogError> {
        enforce_limit(
            "story catalog JSON bytes",
            bytes.len() as u64,
            MAX_CATALOG_JSON_BYTES as u64,
        )?;
        let wire: StoryCatalogWire =
            serde_json::from_slice(bytes).map_err(|source| CatalogError::InvalidJson {
                kind: "story catalog",
                source,
            })?;
        let canonical = canonical_json(&wire, "story catalog", MAX_CATALOG_JSON_BYTES)?;
        if canonical != bytes {
            return Err(CatalogError::NonCanonicalJson {
                kind: "story catalog",
            });
        }
        let catalog = Self {
            wire,
            input_guard: None,
        };
        validate_catalog_file(&catalog)?;
        Ok(catalog)
    }

    pub fn npc_count(&self) -> usize {
        self.wire.catalog.npcs.len()
    }

    pub fn quest_parent_count(&self) -> usize {
        self.wire.catalog.quest_parents.len()
    }

    pub fn generation(&self) -> &GameGenerationSeal {
        &self.wire.catalog.generation
    }

    pub fn catalog_seal(&self) -> &ContentSeal {
        &self.wire.catalog_seal
    }

    /// Return only the pinned, bounded fields needed by a friendly NPC/Quest chooser.
    pub fn authoring_selections(&self) -> Result<StoryCatalogAuthoringSelections, CatalogError> {
        validate_catalog_file(self)?;
        let generation = self.wire.catalog.generation.clone();
        let npcs = self
            .wire
            .catalog
            .npcs
            .iter()
            .map(authoring_npc_selection)
            .collect();
        let quest_parents = self
            .wire
            .catalog
            .quest_parents
            .iter()
            .map(authoring_quest_parent_selection)
            .collect();
        Ok(StoryCatalogAuthoringSelections {
            schema_revision: AUTHORING_SELECTION_SCHEMA_REVISION,
            catalog_seal: self.wire.catalog_seal.clone(),
            npcs,
            quest_parents,
            quest_collision_catalog: AuthoringQuestCollisionCatalogAvailability {
                status: "inventory_unavailable".to_owned(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
                source_seal: generation.shipping_cache.clone(),
                blocks_draft_creation: true,
            },
            generation,
            blocks_build: true,
        })
    }
}

impl PartialEq for StoryCatalogFile {
    fn eq(&self, other: &Self) -> bool {
        self.wire == other.wire
    }
}

impl Eq for StoryCatalogFile {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationInputLimits {
    pub max_executable_bytes: u64,
    pub max_shipping_cache_bytes: u64,
    pub max_binds_cache_bytes: u64,
}

impl Default for GenerationInputLimits {
    fn default() -> Self {
        Self {
            max_executable_bytes: MAX_EXE_BYTES_HARD,
            max_shipping_cache_bytes: MAX_CACHE_BYTES_HARD,
            max_binds_cache_bytes: MAX_BINDS_BYTES_HARD,
        }
    }
}

impl GenerationInputLimits {
    fn validate(self) -> Result<Self, CatalogError> {
        for (name, value, hard_limit) in [
            (
                "max_executable_bytes",
                self.max_executable_bytes,
                MAX_EXE_BYTES_HARD,
            ),
            (
                "max_shipping_cache_bytes",
                self.max_shipping_cache_bytes,
                MAX_CACHE_BYTES_HARD,
            ),
            (
                "max_binds_cache_bytes",
                self.max_binds_cache_bytes,
                MAX_BINDS_BYTES_HARD,
            ),
        ] {
            if value == 0 || value > hard_limit {
                return Err(CatalogError::InvalidLimits(format!(
                    "{name} must be in 1..={hard_limit}, got {value}"
                )));
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationPaths {
    pub executable: PathBuf,
    pub shipping_cache: PathBuf,
    pub binds_cache: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    file: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HandleSnapshot {
    identity: FileIdentity,
    byte_len: u64,
    link_count: u64,
    change_stamp: ChangeStamp,
    is_directory: bool,
    is_reparse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangeStamp {
    values: [i64; 4],
}

#[derive(Debug, Clone)]
struct GuardedInput {
    path: PathBuf,
    identity: FileIdentity,
    byte_len: u64,
    change_stamp: ChangeStamp,
}

#[derive(Debug, Clone)]
struct GenerationInputGuard {
    inputs: [GuardedInput; 3],
}

struct CapturedGeneration {
    generation: GameGenerationSeal,
    guard: GenerationInputGuard,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("invalid story catalog limits: {0}")]
    InvalidLimits(String),
    #[error("story catalog resource limit exceeded for {kind}: {actual} > {limit}")]
    LimitExceeded {
        kind: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("story catalog input is not a regular, link-free file: {0:?}")]
    UnsafeInput(PathBuf),
    #[error("story catalog file identity changed while it was being used: {0:?}")]
    IdentityChanged(PathBuf),
    #[error("story catalog output {output:?} aliases protected generation input {input:?}")]
    OutputAliasesInput { output: PathBuf, input: PathBuf },
    #[error("a parsed catalog has no live generation-input guard and cannot be published")]
    MissingInputGuard,
    #[error("story catalog source changed while hashing {path:?}: expected {expected} bytes, read {actual}")]
    SourceChanged {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("unsupported or stale game generation: expected {expected}, got {actual}")]
    UnsupportedGeneration {
        expected: Box<GameGenerationSeal>,
        actual: Box<GameGenerationSeal>,
    },
    #[error("verified extraction records target a different game generation")]
    RecordGenerationMismatch,
    #[error("untrusted story_catalog.v1 document: {0}")]
    UntrustedCatalog(String),
    #[error("story catalog identity collision for {kind}: {value:?}")]
    Collision { kind: &'static str, value: String },
    #[error("invalid {kind} JSON: {source}")]
    InvalidJson {
        kind: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("{kind} JSON is not canonical")]
    NonCanonicalJson { kind: &'static str },
    #[error("story catalog seal mismatch for {kind}: expected {expected:?}, actual {actual:?}")]
    SealMismatch {
        kind: &'static str,
        expected: ContentSeal,
        actual: ContentSeal,
    },
    #[error("invalid story catalog invariant: {0}")]
    Invariant(String),
    #[error("story catalog I/O at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("story catalog was committed at {path:?}, but durability is uncertain: {source}")]
    CommittedDurabilityUncertain {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "story catalog was committed at {path:?}, but post-commit verification failed: {detail}"
    )]
    CommittedVerificationFailed { path: PathBuf, detail: String },
}

/// Hash explicit generation inputs without loading them in memory.
pub fn capture_generation(
    paths: &GenerationPaths,
    limits: GenerationInputLimits,
) -> Result<GameGenerationSeal, CatalogError> {
    Ok(capture_generation_guarded(paths, limits)?.generation)
}

fn capture_generation_guarded(
    paths: &GenerationPaths,
    limits: GenerationInputLimits,
) -> Result<CapturedGeneration, CatalogError> {
    let limits = limits.validate()?;
    let (executable, executable_guard) = seal_file_guarded(
        &paths.executable,
        limits.max_executable_bytes,
        "executable bytes",
    )?;
    let (shipping_cache, shipping_guard) = seal_file_guarded(
        &paths.shipping_cache,
        limits.max_shipping_cache_bytes,
        "Shipping cache bytes",
    )?;
    let (binds_cache, binds_guard) = seal_file_guarded(
        &paths.binds_cache,
        limits.max_binds_cache_bytes,
        "Binds cache bytes",
    )?;
    Ok(CapturedGeneration {
        generation: GameGenerationSeal {
            edition: "g1r-steam".to_owned(),
            executable,
            shipping_cache,
            binds_cache,
        },
        guard: GenerationInputGuard {
            inputs: [executable_guard, shipping_guard, binds_guard],
        },
    })
}

/// Build the curated revision-1 catalog only when all three generation inputs match exactly.
pub fn build_known_catalog(
    paths: &GenerationPaths,
    limits: GenerationInputLimits,
) -> Result<StoryCatalogFile, CatalogError> {
    let captured = capture_generation_guarded(paths, limits)?;
    let actual = captured.generation;
    let expected = known_generation_v1();
    if actual != expected {
        return Err(CatalogError::UnsupportedGeneration {
            expected: Box::new(expected),
            actual: Box::new(actual),
        });
    }
    let wire = build_wire_from_verified_records(actual, curated_records_v1())?;
    let catalog = StoryCatalogFile {
        wire,
        input_guard: Some(captured.guard),
    };
    validate_catalog_file(&catalog)?;
    Ok(catalog)
}

/// Build a catalog from already verified extraction records.
///
/// This is the seam for the future generalized cache extractor. The current CLI calls it only
/// with the compiled-in record set selected by an exact generation triple.
fn build_wire_from_verified_records(
    generation: GameGenerationSeal,
    mut records: VerifiedExtractionRecords,
) -> Result<StoryCatalogWire, CatalogError> {
    if generation != records.generation {
        return Err(CatalogError::RecordGenerationMismatch);
    }
    normalize_records(&mut records);
    validate_records(&records)?;
    let record_bytes = canonical_json(
        &records,
        "verified extraction records",
        MAX_RECORD_SET_BYTES,
    )?;
    let record_set_seal = seal_bytes(&record_bytes);
    let catalog = StoryCatalogPayload {
        generation,
        record_set_id: records.record_set_id,
        record_set_seal,
        npcs: records.npcs,
        quest_parents: records.quest_parents,
    };
    let catalog_bytes = canonical_json(&catalog, "story catalog payload", MAX_CATALOG_JSON_BYTES)?;
    let catalog_seal = seal_bytes(&catalog_bytes);
    let result = StoryCatalogWire {
        format: StoryCatalogFormat,
        schema_revision: StorySchemaRevisionV1,
        catalog,
        catalog_seal,
    };
    validate_wire_integrity(&result)?;
    Ok(result)
}

/// Atomically replace a catalog file, then reopen and verify its exact canonical bytes and seals.
pub fn publish_catalog_atomic(
    path: impl AsRef<Path>,
    catalog: &StoryCatalogFile,
) -> Result<(), CatalogError> {
    publish_catalog_atomic_with_durability(path.as_ref(), catalog, sync_parent_directory)
}

fn publish_catalog_atomic_with_durability<F>(
    path: &Path,
    catalog: &StoryCatalogFile,
    durability: F,
) -> Result<(), CatalogError>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    publish_catalog_atomic_with_hooks(path, catalog, |_| Ok(()), durability)
}

fn publish_catalog_atomic_with_hooks<P, F>(
    path: &Path,
    catalog: &StoryCatalogFile,
    precommit: P,
    durability: F,
) -> Result<(), CatalogError>
where
    P: FnOnce(&Path) -> Result<(), CatalogError>,
    F: FnOnce(&Path) -> io::Result<()>,
{
    let input_guard = catalog
        .input_guard
        .as_ref()
        .ok_or(CatalogError::MissingInputGuard)?;
    let bytes = catalog.to_canonical_json()?;
    let path = absolute_safe_output_path(path)?;
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| CatalogError::Invariant(format!("output has no parent: {path:?}")))?;
    prepare_output_parent(parent)?;
    validate_publish_state(&path, input_guard)?;
    let file_name = path.file_name().ok_or_else(|| {
        CatalogError::Invariant(format!("catalog output path has no file name: {path:?}"))
    })?;
    let (temporary_path, mut temporary, temporary_identity) = create_temporary(parent, file_name)?;
    let mut temporary_cleanup = TemporaryPathCleanup::new(temporary_path.clone());
    let prepared = temporary
        .write_all(&bytes)
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.sync_all())
        .map_err(|source| CatalogError::Io {
            path: temporary_path.clone(),
            source,
        });
    drop(temporary);
    prepared?;

    let staged = read_bounded(
        &temporary_path,
        MAX_CATALOG_JSON_BYTES,
        "staged story catalog JSON bytes",
    )?;
    let (staged_handle, staged_snapshot) = open_regular_no_follow(&temporary_path, true)?;
    if staged_snapshot.identity != temporary_identity || staged != bytes {
        return Err(CatalogError::IdentityChanged(temporary_path.clone()));
    }
    drop(staged_handle);
    precommit(&temporary_path)?;
    validate_publish_state(&path, input_guard)?;
    atomic_replace(&temporary_path, &path)?;
    temporary_cleanup.disarm();
    if let Err(source) = durability(parent) {
        return Err(CatalogError::CommittedDurabilityUncertain {
            path: path.clone(),
            source,
        });
    }

    let persisted = read_bounded(&path, MAX_CATALOG_JSON_BYTES, "story catalog JSON bytes")
        .map_err(|error| CatalogError::CommittedVerificationFailed {
            path: path.clone(),
            detail: error.to_string(),
        })?;
    if persisted != bytes {
        return Err(CatalogError::CommittedVerificationFailed {
            path,
            detail: "persisted bytes differ from prepared bytes".to_owned(),
        });
    }
    let reopened = StoryCatalogFile::from_json(&persisted).map_err(|error| {
        CatalogError::CommittedVerificationFailed {
            path: path.clone(),
            detail: error.to_string(),
        }
    })?;
    if &reopened != catalog {
        return Err(CatalogError::CommittedVerificationFailed {
            path,
            detail: "persisted catalog did not reopen exactly".to_owned(),
        });
    }
    Ok(())
}

pub fn read_catalog(path: impl AsRef<Path>) -> Result<StoryCatalogFile, CatalogError> {
    let path = path.as_ref();
    let bytes = read_bounded(path, MAX_CATALOG_JSON_BYTES, "story catalog JSON bytes")?;
    StoryCatalogFile::from_json(&bytes)
}

fn authoring_npc_selection(entry: &NpcCatalogEntry) -> AuthoringNpcSelection {
    let character_definition = authoring_class_selection(
        &entry.catalog_id,
        "character_definition",
        &entry.character_definition,
    );
    AuthoringNpcSelection {
        catalog_id: entry.catalog_id.clone(),
        display_name: entry.display_name.clone(),
        runtime_unique_name: entry.runtime_unique_name.clone(),
        quest_giver: AuthoringQuestGiverSelection {
            catalog_layer: entry.character_definition.catalog_layer.clone(),
            authoring_selector: authoring_selector_alias(&entry.catalog_id, "quest_giver"),
            source_catalog_selector: entry.character_definition.canonical_selector.clone(),
            runtime_unique_name: entry.runtime_unique_name.clone(),
            source_seal: entry.character_definition.source_seal.clone(),
        },
        character_definition,
        ai_agent_config: authoring_class_selection(
            &entry.catalog_id,
            "ai_agent_config",
            &entry.ai_agent_config,
        ),
        spawn_definition: authoring_class_selection(
            &entry.catalog_id,
            "spawn_definition",
            &entry.spawn_definition,
        ),
        discovery_status: match entry.discovery_status {
            NpcDiscoveryStatus::SealedCacheDefaultsVerified => {
                "sealed_cache_defaults_verified".to_owned()
            }
        },
        authoring_qualification: match entry.authoring_qualification {
            NpcAuthoringQualification::OfflineQualified => "offline_qualified".to_owned(),
            NpcAuthoringQualification::CatalogObserved => "catalog_observed".to_owned(),
        },
        runtime_qualification: runtime_qualification(entry.runtime_qualification),
        evidence_id: entry.evidence_id.clone(),
        blocks_build: true,
    }
}

fn authoring_quest_parent_selection(
    entry: &QuestParentCatalogEntry,
) -> AuthoringQuestParentSelection {
    AuthoringQuestParentSelection {
        catalog_id: entry.catalog_id.clone(),
        display_name: entry.display_name.clone(),
        quest_class: authoring_class_selection(
            &entry.catalog_id,
            "quest_parent",
            &entry.quest_class,
        ),
        parent_class_name: entry.parent_class_name.clone(),
        role: match entry.role {
            QuestParentRole::Chapter => "chapter".to_owned(),
        },
        qualification: match entry.qualification {
            QuestParentQualification::CuratedDefaultsVerified => {
                "curated_defaults_verified".to_owned()
            }
        },
        transition_qualification: runtime_qualification(entry.transition_qualification),
        evidence_id: entry.evidence_id.clone(),
        blocks_build: true,
    }
}

fn authoring_class_selection(
    catalog_id: &str,
    role: &str,
    class: &CatalogClassRef,
) -> AuthoringClassSelection {
    AuthoringClassSelection {
        catalog_layer: class.catalog_layer.clone(),
        authoring_selector: authoring_selector_alias(catalog_id, role),
        source_catalog_selector: class.canonical_selector.clone(),
        runtime_class: class.class_name.clone(),
        source_seal: class.source_seal.clone(),
    }
}

fn runtime_qualification(value: RuntimeQualification) -> String {
    match value {
        RuntimeQualification::RuntimeUnqualified => "runtime_unqualified".to_owned(),
    }
}

fn authoring_selector_alias(catalog_id: &str, role: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"gore-story-catalog.authoring-selector-v1\0");
    for value in [catalog_id.as_bytes(), role.as_bytes()] {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value);
    }
    let bytes: [u8; 32] = digest.finalize().into();
    format!("Catalog_{}", Sha256Digest::from_bytes(bytes))
}

pub fn known_generation_v1() -> GameGenerationSeal {
    GameGenerationSeal {
        edition: "g1r-steam".to_owned(),
        executable: known_seal(
            171_698_176,
            "f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5",
        ),
        shipping_cache: known_seal(
            123_394_250,
            "1018f1cfe6b99a650eecb33afb96752d691d2088ead27808971b812f04ecb4c2",
        ),
        binds_cache: known_seal(
            5_903_938,
            "46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea",
        ),
    }
}

fn curated_records_v1() -> VerifiedExtractionRecords {
    let generation = known_generation_v1();
    VerifiedExtractionRecords {
        record_set_id: RECORD_SET_ID.to_owned(),
        generation,
        npcs: vec![
            curated_npc(
                "g1r:npc:om_grd_asghan_263",
                "Asghan",
                "OM_GRD_Asghan_263",
                "AI.AIAgent.Human.Config.OM_GRD_Asghan_263.CharacterDefinition_OM_GRD_Asghan_263",
                "AI/AIAgent/Human/Config/OM_GRD_Asghan_263/CharacterDefinition_OM_GRD_Asghan_263.as",
                "UCharacterDefinition_Human_OM_GRD_Asghan_263",
                known_seal(460, "2312e01be5dd91d043b03acbd487f310d47b99107d765ce31ad87aa77eb5723e"),
                "AI.AIAgent.Human.Config.OM_GRD_Asghan_263.DailyRoutine_OM_GRD_Asghan_263",
                "AI/AIAgent/Human/Config/OM_GRD_Asghan_263/DailyRoutine_OM_GRD_Asghan_263.as",
                "UAIAgentConfig_Human_OM_GRD_Asghan_263",
                known_seal(932, "b728be66667b1b220438c40c11d0881eab01f6a7cc9094ea935b90a1da36eae8"),
                "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
                NpcAuthoringQualification::OfflineQualified,
                "npc-logical-clone-v1",
            ),
            curated_npc(
                "g1r:npc:om_stt_viper_302",
                "Viper",
                "OM_STT_Viper_302",
                "AI.AIAgent.Human.Config.OM_STT_Viper_302.CharacterDefinition_OM_STT_Viper_302",
                "AI/AIAgent/Human/Config/OM_STT_Viper_302/CharacterDefinition_OM_STT_Viper_302.as",
                "UCharacterDefinition_Human_OM_STT_Viper_302",
                known_seal(455, "1a4c6caad0511154f4622722f38ec5f85cc2e12f500224f90f4e0208614e7c73"),
                "AI.AIAgent.Human.Config.OM_STT_Viper_302.DailyRoutine_OM_STT_Viper_302",
                "AI/AIAgent/Human/Config/OM_STT_Viper_302/DailyRoutine_OM_STT_Viper_302.as",
                "UAIAgentConfig_Human_OM_STT_Viper_302",
                known_seal(932, "dde3f35f70f23a1ae77f0768d7a947fc2fbd9deaac4b3c12a5bad4f35725220b"),
                "USpawnAIAgentDefinition_OM_STT_Viper_302",
                NpcAuthoringQualification::OfflineQualified,
                VIPER_OFFLINE_EVIDENCE_ID,
            ),
        ],
        quest_parents: vec![QuestParentCatalogEntry {
            catalog_id: "g1r:quest-parent:swampcamp_scchapter2".to_owned(),
            display_name: "Swamp Camp — Chapter 2".to_owned(),
            quest_class: class_ref(
                "Story.G1R.Quest.Quest_SwampCamp_SCCHAPTER2",
                "Story/G1R/Quest/Quest_SwampCamp_SCCHAPTER2.as",
                "UQuest_SwampCamp_SCCHAPTER2",
                known_seal(856, "5f5060a2740794853fcf0aa38306e183637e81658ab3e9f9b97eee8c5bdd74dd"),
            ),
            parent_class_name: "UQuest_SwampCamp".to_owned(),
            role: QuestParentRole::Chapter,
            qualification: QuestParentQualification::CuratedDefaultsVerified,
            transition_qualification: RuntimeQualification::RuntimeUnqualified,
            evidence_id: "current-cache-defaults-swampcamp-chapter2-20260712".to_owned(),
            blocks_build: BuildBlocked,
        }],
    }
}

#[allow(clippy::too_many_arguments)]
fn curated_npc(
    catalog_id: &str,
    display_name: &str,
    runtime_unique_name: &str,
    character_module: &str,
    character_path: &str,
    character_class: &str,
    character_seal: ContentSeal,
    config_module: &str,
    config_path: &str,
    config_class: &str,
    config_seal: ContentSeal,
    spawn_class: &str,
    authoring_qualification: NpcAuthoringQualification,
    evidence_id: &str,
) -> NpcCatalogEntry {
    NpcCatalogEntry {
        catalog_id: catalog_id.to_owned(),
        display_name: display_name.to_owned(),
        runtime_unique_name: runtime_unique_name.to_owned(),
        character_definition: class_ref(
            character_module,
            character_path,
            character_class,
            character_seal,
        ),
        ai_agent_config: class_ref(config_module, config_path, config_class, config_seal),
        spawn_definition: class_ref(
            "Spawning.SpawningDefinition_Human",
            "Spawning/SpawningDefinition_Human.as",
            spawn_class,
            known_seal(
                96_033,
                "e49a3a5f8ac2a589f40878f6f248ab8743adefeab07081754f681cb85c36b86b",
            ),
        ),
        discovery_status: NpcDiscoveryStatus::SealedCacheDefaultsVerified,
        authoring_qualification,
        runtime_qualification: RuntimeQualification::RuntimeUnqualified,
        evidence_id: evidence_id.to_owned(),
        blocks_build: BuildBlocked,
    }
}

fn class_ref(
    module: &str,
    relative_path: &str,
    class_name: &str,
    source_seal: ContentSeal,
) -> CatalogClassRef {
    CatalogClassRef {
        catalog_layer: BASE_GAME_LAYER.to_owned(),
        canonical_selector: format!("script-class:{module}/{class_name}"),
        module: module.to_owned(),
        relative_path: relative_path.to_owned(),
        class_name: class_name.to_owned(),
        source_seal,
        evidence_kind: SourceEvidenceKind::SealedEmittedSourceAndCacheDefaultsV1,
    }
}

fn known_seal(byte_len: u64, digest: &str) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_hex(digest).expect("known digest is canonical"),
    }
}

fn normalize_records(records: &mut VerifiedExtractionRecords) {
    records
        .npcs
        .sort_by(|left, right| left.catalog_id.cmp(&right.catalog_id));
    records
        .quest_parents
        .sort_by(|left, right| left.catalog_id.cmp(&right.catalog_id));
}

fn validate_catalog_file(catalog: &StoryCatalogFile) -> Result<(), CatalogError> {
    let wire = &catalog.wire;
    validate_wire_integrity(wire)?;

    let expected_generation = known_generation_v1();
    if wire.catalog.generation != expected_generation {
        return Err(CatalogError::UnsupportedGeneration {
            expected: Box::new(expected_generation),
            actual: Box::new(wire.catalog.generation.clone()),
        });
    }
    if wire.catalog.record_set_id != RECORD_SET_ID {
        return Err(CatalogError::UntrustedCatalog(format!(
            "record_set_id {:?} is not the compiled V1 record set {:?}",
            wire.catalog.record_set_id, RECORD_SET_ID
        )));
    }

    let records = records_from_wire(wire);
    let mut expected_records = curated_records_v1();
    normalize_records(&mut expected_records);
    if records != expected_records {
        return Err(CatalogError::UntrustedCatalog(
            "record content differs from the compiled curated V1 record set".to_owned(),
        ));
    }

    let expected_record_seal = known_seal(RECORD_SET_BYTE_LEN, RECORD_SET_SHA256);
    if wire.catalog.record_set_seal != expected_record_seal {
        return Err(CatalogError::SealMismatch {
            kind: "compiled curated V1 record set",
            expected: expected_record_seal,
            actual: wire.catalog.record_set_seal.clone(),
        });
    }
    let expected_catalog_seal = known_seal(CATALOG_PAYLOAD_BYTE_LEN, CATALOG_PAYLOAD_SHA256);
    if wire.catalog_seal != expected_catalog_seal {
        return Err(CatalogError::SealMismatch {
            kind: "compiled curated V1 catalog payload",
            expected: expected_catalog_seal,
            actual: wire.catalog_seal.clone(),
        });
    }
    Ok(())
}

fn records_from_wire(wire: &StoryCatalogWire) -> VerifiedExtractionRecords {
    VerifiedExtractionRecords {
        record_set_id: wire.catalog.record_set_id.clone(),
        generation: wire.catalog.generation.clone(),
        npcs: wire.catalog.npcs.clone(),
        quest_parents: wire.catalog.quest_parents.clone(),
    }
}

fn validate_wire_integrity(wire: &StoryCatalogWire) -> Result<(), CatalogError> {
    validate_generation(&wire.catalog.generation)?;
    validate_text("record_set_id", &wire.catalog.record_set_id)?;
    let records = records_from_wire(wire);
    validate_records(&records)?;
    let record_bytes = canonical_json(
        &records,
        "verified extraction records",
        MAX_RECORD_SET_BYTES,
    )?;
    let actual_record_seal = seal_bytes(&record_bytes);
    if wire.catalog.record_set_seal != actual_record_seal {
        return Err(CatalogError::SealMismatch {
            kind: "verified extraction records",
            expected: wire.catalog.record_set_seal.clone(),
            actual: actual_record_seal,
        });
    }
    let payload_bytes = canonical_json(
        &wire.catalog,
        "story catalog payload",
        MAX_CATALOG_JSON_BYTES,
    )?;
    let actual_catalog_seal = seal_bytes(&payload_bytes);
    if wire.catalog_seal != actual_catalog_seal {
        return Err(CatalogError::SealMismatch {
            kind: "story catalog payload",
            expected: wire.catalog_seal.clone(),
            actual: actual_catalog_seal,
        });
    }
    Ok(())
}

fn validate_records(records: &VerifiedExtractionRecords) -> Result<(), CatalogError> {
    validate_generation(&records.generation)?;
    validate_text("record_set_id", &records.record_set_id)?;
    enforce_limit("NPC count", records.npcs.len() as u64, MAX_NPCS as u64)?;
    enforce_limit(
        "quest-parent count",
        records.quest_parents.len() as u64,
        MAX_QUEST_PARENTS as u64,
    )?;
    validate_strict_catalog_order(
        "NPC catalog_id array",
        records.npcs.iter().map(|entry| entry.catalog_id.as_str()),
    )?;
    validate_strict_catalog_order(
        "quest-parent catalog_id array",
        records
            .quest_parents
            .iter()
            .map(|entry| entry.catalog_id.as_str()),
    )?;

    let mut catalog_ids = BTreeSet::new();
    let mut runtime_names = BTreeSet::new();
    let mut class_names = BTreeSet::new();
    let mut selectors = BTreeSet::new();
    for npc in &records.npcs {
        validate_catalog_id(&npc.catalog_id)?;
        insert_identity(&mut catalog_ids, "catalog_id", &npc.catalog_id)?;
        validate_ascii_identifier("NPC runtime_unique_name", &npc.runtime_unique_name)?;
        insert_identity(
            &mut runtime_names,
            "NPC runtime_unique_name",
            &npc.runtime_unique_name,
        )?;
        validate_text("NPC display_name", &npc.display_name)?;
        validate_text("NPC evidence_id", &npc.evidence_id)?;
        for class in [
            &npc.character_definition,
            &npc.ai_agent_config,
            &npc.spawn_definition,
        ] {
            validate_class_ref(class)?;
            insert_identity(&mut class_names, "script class", &class.class_name)?;
            insert_identity(
                &mut selectors,
                "canonical selector",
                &class.canonical_selector,
            )?;
        }
    }
    for quest in &records.quest_parents {
        validate_catalog_id(&quest.catalog_id)?;
        insert_identity(&mut catalog_ids, "catalog_id", &quest.catalog_id)?;
        validate_text("quest display_name", &quest.display_name)?;
        validate_ascii_identifier("quest parent_class_name", &quest.parent_class_name)?;
        validate_text("quest evidence_id", &quest.evidence_id)?;
        validate_class_ref(&quest.quest_class)?;
        insert_identity(
            &mut class_names,
            "script class",
            &quest.quest_class.class_name,
        )?;
        insert_identity(
            &mut selectors,
            "canonical selector",
            &quest.quest_class.canonical_selector,
        )?;
    }
    Ok(())
}

fn validate_strict_catalog_order<'a>(
    kind: &'static str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), CatalogError> {
    let mut previous: Option<&str> = None;
    for value in values {
        if let Some(prior) = previous {
            if prior >= value {
                return Err(CatalogError::Invariant(format!(
                    "{kind} must be strictly increasing; found {prior:?} before {value:?}"
                )));
            }
        }
        previous = Some(value);
    }
    Ok(())
}

fn validate_generation(generation: &GameGenerationSeal) -> Result<(), CatalogError> {
    validate_text("generation edition", &generation.edition)?;
    for (kind, seal) in [
        ("executable seal", &generation.executable),
        ("Shipping cache seal", &generation.shipping_cache),
        ("Binds cache seal", &generation.binds_cache),
    ] {
        validate_seal(kind, seal)?;
    }
    Ok(())
}

fn validate_class_ref(value: &CatalogClassRef) -> Result<(), CatalogError> {
    for (kind, text) in [
        ("catalog layer", value.catalog_layer.as_str()),
        ("canonical selector", value.canonical_selector.as_str()),
        ("module", value.module.as_str()),
        ("relative path", value.relative_path.as_str()),
        ("class name", value.class_name.as_str()),
    ] {
        validate_text(kind, text)?;
    }
    if value.catalog_layer != BASE_GAME_LAYER {
        return Err(CatalogError::Invariant(format!(
            "unsupported catalog layer {:?}",
            value.catalog_layer
        )));
    }
    validate_module(&value.module)?;
    validate_ascii_identifier("class name", &value.class_name)?;
    let expected_selector = format!("script-class:{}/{}", value.module, value.class_name);
    if value.canonical_selector != expected_selector {
        return Err(CatalogError::Invariant(format!(
            "noncanonical script selector {:?}; expected {:?}",
            value.canonical_selector, expected_selector
        )));
    }
    let expected_path = format!("{}.as", value.module.replace('.', "/"));
    if value.relative_path != expected_path {
        return Err(CatalogError::Invariant(format!(
            "source path {:?} disagrees with module {:?}; expected {:?}",
            value.relative_path, value.module, expected_path
        )));
    }
    if value.relative_path.contains('\\')
        || value.relative_path.starts_with('/')
        || value
            .relative_path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
        || !value.relative_path.ends_with(".as")
    {
        return Err(CatalogError::Invariant(format!(
            "unsafe/noncanonical source path {:?}",
            value.relative_path
        )));
    }
    validate_seal("class source seal", &value.source_seal)
}

fn validate_seal(kind: &'static str, seal: &ContentSeal) -> Result<(), CatalogError> {
    if seal.byte_len == 0 {
        return Err(CatalogError::Invariant(format!(
            "{kind} byte_len must be non-zero"
        )));
    }
    Ok(())
}

fn validate_text(kind: &'static str, value: &str) -> Result<(), CatalogError> {
    if value.is_empty() {
        return Err(CatalogError::Invariant(format!("{kind} must not be empty")));
    }
    enforce_limit(kind, value.len() as u64, MAX_TEXT_BYTES as u64)?;
    if value.chars().any(char::is_control) {
        return Err(CatalogError::Invariant(format!(
            "{kind} must not contain control characters"
        )));
    }
    Ok(())
}

fn insert_identity(
    identities: &mut BTreeSet<String>,
    kind: &'static str,
    value: &str,
) -> Result<(), CatalogError> {
    validate_text(kind, value)?;
    if !value.is_ascii() || value.bytes().any(|byte| !byte.is_ascii_graphic()) {
        return Err(CatalogError::Invariant(format!(
            "{kind} must contain only printable non-whitespace ASCII characters"
        )));
    }
    let folded = value.to_ascii_lowercase();
    if !identities.insert(folded) {
        return Err(CatalogError::Collision {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_catalog_id(value: &str) -> Result<(), CatalogError> {
    validate_text("catalog_id", value)?;
    let segments: Vec<_> = value.split(':').collect();
    if segments.len() < 3
        || segments.iter().any(|segment| {
            segment.is_empty()
                || !segment.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'-' | b'.')
                })
                || !segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                || !segment
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
    {
        return Err(CatalogError::Invariant(format!(
            "catalog_id {value:?} must be at least three colon-separated lowercase ASCII slug segments"
        )));
    }
    Ok(())
}

fn validate_ascii_identifier(kind: &'static str, value: &str) -> Result<(), CatalogError> {
    validate_text(kind, value)?;
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        unreachable!("validate_text rejected an empty identifier");
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(CatalogError::Invariant(format!(
            "{kind} {value:?} must match the ASCII identifier grammar [A-Za-z_][A-Za-z0-9_]*"
        )));
    }
    Ok(())
}

fn validate_module(value: &str) -> Result<(), CatalogError> {
    validate_text("module", value)?;
    if value
        .split('.')
        .any(|segment| validate_ascii_identifier("module segment", segment).is_err())
    {
        return Err(CatalogError::Invariant(format!(
            "module {value:?} must be dot-separated ASCII identifiers"
        )));
    }
    Ok(())
}

#[cfg(test)]
fn seal_file(path: &Path, max: u64, kind: &'static str) -> Result<ContentSeal, CatalogError> {
    Ok(seal_file_guarded(path, max, kind)?.0)
}

fn seal_file_guarded(
    path: &Path,
    max: u64,
    kind: &'static str,
) -> Result<(ContentSeal, GuardedInput), CatalogError> {
    let (mut file, initial) = open_regular_no_follow(path, true)?;
    enforce_limit(kind, initial.byte_len, max)?;
    if initial.byte_len == 0 {
        return Err(CatalogError::Invariant(format!(
            "{kind} must not be empty at {path:?}"
        )));
    }
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer).map_err(|source| CatalogError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(CatalogError::LimitExceeded {
                kind,
                actual: u64::MAX,
                limit: max,
            })?;
        enforce_limit(kind, total, max)?;
        hasher.update(&buffer[..count]);
    }
    if total != initial.byte_len {
        return Err(CatalogError::SourceChanged {
            path: path.to_path_buf(),
            expected: initial.byte_len,
            actual: total,
        });
    }
    let final_snapshot = snapshot_open_handle(&file, path)?;
    validate_regular_snapshot(path, final_snapshot, true)?;
    if final_snapshot.identity != initial.identity
        || final_snapshot.change_stamp != initial.change_stamp
    {
        return Err(CatalogError::IdentityChanged(path.to_path_buf()));
    }
    if final_snapshot.byte_len != initial.byte_len {
        return Err(CatalogError::SourceChanged {
            path: path.to_path_buf(),
            expected: initial.byte_len,
            actual: final_snapshot.byte_len,
        });
    }
    let (_reopened, reopened_snapshot) = open_regular_no_follow(path, true)?;
    if reopened_snapshot.identity != initial.identity
        || reopened_snapshot.change_stamp != initial.change_stamp
    {
        return Err(CatalogError::IdentityChanged(path.to_path_buf()));
    }
    let seal = ContentSeal {
        byte_len: total,
        sha256: Sha256Digest::from_bytes(hasher.finalize().into()),
    };
    let guard = GuardedInput {
        path: path.to_path_buf(),
        identity: initial.identity,
        byte_len: initial.byte_len,
        change_stamp: initial.change_stamp,
    };
    Ok((seal, guard))
}

fn open_regular_no_follow(
    path: &Path,
    require_single_link: bool,
) -> Result<(File, HandleSnapshot), CatalogError> {
    let file = open_regular_handle_no_follow(path)
        .map_err(|source| classify_no_follow_open_error(path, source))?;
    let snapshot = snapshot_open_handle(&file, path)?;
    validate_regular_snapshot(path, snapshot, require_single_link)?;
    Ok((file, snapshot))
}

fn validate_regular_snapshot(
    path: &Path,
    snapshot: HandleSnapshot,
    require_single_link: bool,
) -> Result<(), CatalogError> {
    if snapshot.is_directory
        || snapshot.is_reparse
        || (require_single_link && snapshot.link_count != 1)
    {
        return Err(CatalogError::UnsafeInput(path.to_path_buf()));
    }
    Ok(())
}

fn classify_no_follow_open_error(path: &Path, source: io::Error) -> CatalogError {
    #[cfg(unix)]
    if source.raw_os_error() == Some(libc::ELOOP) {
        return CatalogError::UnsafeInput(path.to_path_buf());
    }
    CatalogError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(windows)]
fn open_regular_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_regular_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(windows)]
fn snapshot_open_handle(file: &File, path: &Path) -> Result<HandleSnapshot, CatalogError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_BASIC_INFO,
    };

    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the duration of the call.
    let result = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    if result == 0 {
        return Err(CatalogError::Io {
            path: path.to_path_buf(),
            source: io::Error::last_os_error(),
        });
    }
    let mut basic = FILE_BASIC_INFO::default();
    // SAFETY: `file` owns a valid handle and `basic` is a correctly sized writable buffer for
    // `FileBasicInfo` for the duration of the call.
    let basic_result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            std::ptr::addr_of_mut!(basic).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if basic_result == 0 {
        return Err(CatalogError::Io {
            path: path.to_path_buf(),
            source: io::Error::last_os_error(),
        });
    }
    Ok(HandleSnapshot {
        identity: FileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        byte_len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        link_count: u64::from(info.nNumberOfLinks),
        change_stamp: ChangeStamp {
            values: [
                basic.ChangeTime,
                basic.LastWriteTime,
                basic.CreationTime,
                i64::from(basic.FileAttributes),
            ],
        },
        is_directory: info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
        is_reparse: info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}

#[cfg(unix)]
fn snapshot_open_handle(file: &File, path: &Path) -> Result<HandleSnapshot, CatalogError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(|source| CatalogError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(HandleSnapshot {
        identity: FileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        },
        byte_len: metadata.len(),
        link_count: metadata.nlink(),
        change_stamp: ChangeStamp {
            values: [
                metadata.mtime(),
                metadata.mtime_nsec(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            ],
        },
        is_directory: metadata.is_dir(),
        is_reparse: false,
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
) -> Result<Vec<u8>, CatalogError> {
    struct BoundedBuffer {
        bytes: Vec<u8>,
        max: usize,
        exceeded: bool,
    }

    impl Write for BoundedBuffer {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let Some(new_len) = self.bytes.len().checked_add(buffer.len()) else {
                self.exceeded = true;
                return Err(io::Error::other("canonical JSON size overflow"));
            };
            if new_len > self.max {
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
        bytes: Vec::with_capacity(max.min(COPY_BUFFER_BYTES)),
        max,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut output, value);
    if output.exceeded {
        return Err(CatalogError::LimitExceeded {
            kind,
            actual: (max as u64).saturating_add(1),
            limit: max as u64,
        });
    }
    result.map_err(|source| CatalogError::InvalidJson { kind, source })?;
    Ok(output.bytes)
}

fn enforce_limit(kind: &'static str, actual: u64, limit: u64) -> Result<(), CatalogError> {
    if actual > limit {
        Err(CatalogError::LimitExceeded {
            kind,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn read_bounded(path: &Path, max: usize, kind: &'static str) -> Result<Vec<u8>, CatalogError> {
    let (mut file, initial) = open_regular_no_follow(path, true)?;
    enforce_limit(kind, initial.byte_len, max as u64)?;
    let mut bytes = Vec::with_capacity((initial.byte_len as usize).min(max));
    {
        let mut bounded = (&mut file).take((max as u64).saturating_add(1));
        bounded
            .read_to_end(&mut bytes)
            .map_err(|source| CatalogError::Io {
                path: path.to_path_buf(),
                source,
            })?;
    }
    enforce_limit(kind, bytes.len() as u64, max as u64)?;
    let final_snapshot = snapshot_open_handle(&file, path)?;
    validate_regular_snapshot(path, final_snapshot, true)?;
    if final_snapshot.identity != initial.identity
        || final_snapshot.change_stamp != initial.change_stamp
    {
        return Err(CatalogError::IdentityChanged(path.to_path_buf()));
    }
    if bytes.len() as u64 != initial.byte_len || final_snapshot.byte_len != initial.byte_len {
        return Err(CatalogError::SourceChanged {
            path: path.to_path_buf(),
            expected: initial.byte_len,
            actual: bytes.len() as u64,
        });
    }
    let (_reopened, reopened) = open_regular_no_follow(path, true)?;
    if reopened.identity != initial.identity || reopened.change_stamp != initial.change_stamp {
        return Err(CatalogError::IdentityChanged(path.to_path_buf()));
    }
    Ok(bytes)
}

fn absolute_safe_output_path(path: &Path) -> Result<PathBuf, CatalogError> {
    use std::path::Component;

    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(CatalogError::Invariant(format!(
            "catalog output must be non-empty and must not contain '..': {path:?}"
        )));
    }
    if !path.is_absolute()
        && path
            .components()
            .any(|component| matches!(component, Component::Prefix(_)))
    {
        return Err(CatalogError::Invariant(format!(
            "drive-relative catalog output is not supported: {path:?}"
        )));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| CatalogError::Io {
                path: PathBuf::from("."),
                source,
            })?
            .join(path)
    };
    if absolute.file_name().is_none() {
        return Err(CatalogError::Invariant(format!(
            "catalog output path has no file name: {absolute:?}"
        )));
    }
    Ok(absolute)
}

fn prepare_output_parent(parent: &Path) -> Result<(), CatalogError> {
    let mut ancestors: Vec<_> = parent.ancestors().collect();
    ancestors.reverse();
    for directory in ancestors {
        if directory.as_os_str().is_empty() {
            continue;
        }
        match open_directory_no_follow(directory) {
            Ok(_) => {}
            Err(CatalogError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                match fs::create_dir(directory) {
                    Ok(()) => {}
                    Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(source) => {
                        return Err(CatalogError::Io {
                            path: directory.to_path_buf(),
                            source,
                        });
                    }
                }
                open_directory_no_follow(directory)?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn validate_output_ancestors(path: &Path) -> Result<(), CatalogError> {
    let parent = path
        .parent()
        .ok_or_else(|| CatalogError::Invariant(format!("output has no parent: {path:?}")))?;
    for directory in parent.ancestors() {
        if !directory.as_os_str().is_empty() {
            open_directory_no_follow(directory)?;
        }
    }
    Ok(())
}

fn validate_publish_state(output: &Path, guard: &GenerationInputGuard) -> Result<(), CatalogError> {
    validate_output_ancestors(output)?;

    let mut snapshots = Vec::with_capacity(guard.inputs.len());
    for input in &guard.inputs {
        let (_file, snapshot) = open_regular_no_follow(&input.path, false)?;
        if snapshot.identity != input.identity
            || snapshot.byte_len != input.byte_len
            || snapshot.change_stamp != input.change_stamp
        {
            return Err(CatalogError::IdentityChanged(input.path.clone()));
        }
        snapshots.push(snapshot);
    }

    let output_snapshot = open_optional_regular_no_follow(output)?;
    if let Some(snapshot) = output_snapshot {
        for input in &guard.inputs {
            if snapshot.identity == input.identity {
                return Err(CatalogError::OutputAliasesInput {
                    output: output.to_path_buf(),
                    input: input.path.clone(),
                });
            }
        }
        if snapshot.link_count != 1 {
            return Err(CatalogError::UnsafeInput(output.to_path_buf()));
        }
    }
    for (input, snapshot) in guard.inputs.iter().zip(snapshots) {
        if snapshot.link_count != 1 {
            return Err(CatalogError::UnsafeInput(input.path.clone()));
        }
    }
    Ok(())
}

fn open_optional_regular_no_follow(path: &Path) -> Result<Option<HandleSnapshot>, CatalogError> {
    match open_regular_handle_no_follow(path) {
        Ok(file) => {
            let snapshot = snapshot_open_handle(&file, path)?;
            validate_regular_snapshot(path, snapshot, false)?;
            Ok(Some(snapshot))
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(classify_no_follow_open_error(path, source)),
    }
}

fn open_directory_no_follow(path: &Path) -> Result<File, CatalogError> {
    let file = open_directory_handle_no_follow(path)
        .map_err(|source| classify_no_follow_open_error(path, source))?;
    let snapshot = snapshot_open_handle(&file, path)?;
    if !snapshot.is_directory || snapshot.is_reparse {
        return Err(CatalogError::UnsafeInput(path.to_path_buf()));
    }
    Ok(file)
}

#[cfg(windows)]
fn open_directory_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_directory_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

struct TemporaryPathCleanup {
    path: PathBuf,
    armed: bool,
}

impl TemporaryPathCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryPathCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn create_temporary(
    parent: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(PathBuf, File, FileIdentity), CatalogError> {
    for _ in 0..128 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".{}.story-catalog-{}-{sequence:016x}.tmp",
            file_name.to_string_lossy(),
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                let validation = snapshot_open_handle(&file, &path).and_then(|snapshot| {
                    validate_regular_snapshot(&path, snapshot, true)?;
                    Ok(snapshot.identity)
                });
                match validation {
                    Ok(identity) => return Ok((path, file, identity)),
                    Err(error) => {
                        drop(file);
                        let _ = fs::remove_file(&path);
                        return Err(error);
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(CatalogError::Io { path, source }),
        }
    }
    Err(CatalogError::Invariant(
        "could not allocate a unique catalog staging file".to_owned(),
    ))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), CatalogError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source_wide: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both buffers are stable and NUL-terminated for the duration of the call.
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(CatalogError::Io {
            path: destination.to_path_buf(),
            source: io::Error::last_os_error(),
        })
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), CatalogError> {
    fs::rename(source, destination).map_err(|source| CatalogError::Io {
        path: destination.to_path_buf(),
        source,
    })
}

#[cfg(windows)]
fn sync_parent_directory(_parent: &Path) -> io::Result<()> {
    // `MoveFileExW(..., MOVEFILE_WRITE_THROUGH)` performs the write-through commit on Windows.
    Ok(())
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    let directory = open_directory_handle_no_follow(parent)?;
    directory.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_generation(byte: u8) -> GameGenerationSeal {
        GameGenerationSeal {
            edition: "test".to_owned(),
            executable: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([byte; 32]),
            },
            shipping_cache: ContentSeal {
                byte_len: 2,
                sha256: Sha256Digest::from_bytes([byte.wrapping_add(1); 32]),
            },
            binds_cache: ContentSeal {
                byte_len: 3,
                sha256: Sha256Digest::from_bytes([byte.wrapping_add(2); 32]),
            },
        }
    }

    fn synthetic_records() -> VerifiedExtractionRecords {
        let generation = synthetic_generation(1);
        let mut records = curated_records_v1();
        records.record_set_id = "test-records".to_owned();
        records.generation = generation;
        records.npcs.truncate(1);
        records.quest_parents.truncate(1);
        records
    }

    fn trusted_catalog() -> StoryCatalogFile {
        let wire =
            build_wire_from_verified_records(known_generation_v1(), curated_records_v1()).unwrap();
        let catalog = StoryCatalogFile {
            wire,
            input_guard: None,
        };
        validate_catalog_file(&catalog).unwrap();
        catalog
    }

    fn attach_test_input_guard(catalog: &mut StoryCatalogFile, root: &Path) -> GenerationPaths {
        let paths = GenerationPaths {
            executable: root.join("guard-executable.bin"),
            shipping_cache: root.join("guard-shipping.cache"),
            binds_cache: root.join("guard-binds.cache"),
        };
        fs::write(&paths.executable, b"guard executable").unwrap();
        fs::write(&paths.shipping_cache, b"guard shipping cache").unwrap();
        fs::write(&paths.binds_cache, b"guard binds cache").unwrap();
        let (_, executable) =
            seal_file_guarded(&paths.executable, 1024, "test executable").unwrap();
        let (_, shipping) =
            seal_file_guarded(&paths.shipping_cache, 1024, "test shipping cache").unwrap();
        let (_, binds) = seal_file_guarded(&paths.binds_cache, 1024, "test binds cache").unwrap();
        catalog.input_guard = Some(GenerationInputGuard {
            inputs: [executable, shipping, binds],
        });
        paths
    }

    fn reseal_wire(wire: &mut StoryCatalogWire) {
        let records = records_from_wire(wire);
        let record_bytes = canonical_json(
            &records,
            "verified extraction records",
            MAX_RECORD_SET_BYTES,
        )
        .unwrap();
        wire.catalog.record_set_seal = seal_bytes(&record_bytes);
        let payload_bytes = canonical_json(
            &wire.catalog,
            "story catalog payload",
            MAX_CATALOG_JSON_BYTES,
        )
        .unwrap();
        wire.catalog_seal = seal_bytes(&payload_bytes);
    }

    #[cfg(windows)]
    fn create_file_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }

    #[cfg(unix)]
    fn create_file_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_directory_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn create_directory_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[test]
    fn curated_catalog_is_truthful_sorted_and_canonical() {
        let catalog = trusted_catalog();
        let bytes = catalog.to_canonical_json().unwrap();
        assert_eq!(StoryCatalogFile::from_json(&bytes).unwrap(), catalog);
        assert_eq!(catalog.wire.catalog.npcs.len(), 2);
        assert_eq!(
            catalog.wire.catalog.npcs[0].runtime_qualification,
            RuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(
            catalog.wire.catalog.quest_parents[0].transition_qualification,
            RuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(
            catalog.wire.catalog.npcs[0].authoring_qualification,
            NpcAuthoringQualification::OfflineQualified
        );
        assert!(catalog.wire.catalog.npcs.iter().all(|npc| {
            npc.authoring_qualification == NpcAuthoringQualification::OfflineQualified
                && npc.runtime_qualification == RuntimeQualification::RuntimeUnqualified
        }));
    }

    #[test]
    fn authoring_projection_is_bounded_friendly_and_never_invents_quest_collisions() {
        let catalog = trusted_catalog();
        let first = catalog.authoring_selections().unwrap();
        let second = catalog.authoring_selections().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.schema_revision, AUTHORING_SELECTION_SCHEMA_REVISION);
        assert_eq!(first.npcs.len(), MAX_NPCS);
        assert_eq!(first.quest_parents.len(), MAX_QUEST_PARENTS);
        assert!(first.blocks_build);
        assert_eq!(
            first.quest_collision_catalog.status,
            "inventory_unavailable"
        );
        assert!(first.quest_collision_catalog.blocks_draft_creation);
        assert_eq!(
            first.quest_collision_catalog.source_seal,
            first.generation.shipping_cache
        );

        let mut aliases = BTreeSet::new();
        for npc in &first.npcs {
            assert_eq!(npc.authoring_qualification, "offline_qualified");
            assert_eq!(npc.runtime_qualification, "runtime_unqualified");
            assert!(npc.blocks_build);
            for (role, class) in [
                ("character_definition", &npc.character_definition),
                ("ai_agent_config", &npc.ai_agent_config),
                ("spawn_definition", &npc.spawn_definition),
            ] {
                assert!(class.authoring_selector.starts_with("Catalog_"));
                assert_eq!(class.authoring_selector.len(), 72);
                assert_eq!(
                    class.authoring_selector,
                    authoring_selector_alias(&npc.catalog_id, role)
                );
                assert_eq!(class.catalog_layer, BASE_GAME_LAYER);
                assert!(class
                    .authoring_selector
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'));
                assert!(aliases.insert(class.authoring_selector.clone()));
                assert!(class.source_catalog_selector.starts_with("script-class:"));
                assert!(class
                    .source_catalog_selector
                    .ends_with(&format!("/{}", class.runtime_class)));
            }
            assert_eq!(
                npc.quest_giver.authoring_selector,
                authoring_selector_alias(&npc.catalog_id, "quest_giver")
            );
            assert_eq!(
                npc.quest_giver.source_catalog_selector,
                npc.character_definition.source_catalog_selector
            );
            assert!(aliases.insert(npc.quest_giver.authoring_selector.clone()));
        }
        let parent = &first.quest_parents[0];
        assert_eq!(parent.transition_qualification, "runtime_unqualified");
        assert!(parent.blocks_build);
        assert_eq!(
            parent.quest_class.authoring_selector,
            authoring_selector_alias(&parent.catalog_id, "quest_parent")
        );
        assert!(parent
            .quest_class
            .source_catalog_selector
            .ends_with(&format!("/{}", parent.quest_class.runtime_class)));
        assert!(aliases.insert(parent.quest_class.authoring_selector.clone()));

        let wire = serde_json::to_value(&first).unwrap();
        let encoded = serde_json::to_string(&wire).unwrap();
        assert!(!encoded.contains("relative_path"));
        assert!(!encoded.contains("\"module\""));
        assert!(!encoded.contains("modules"));
        assert!(!encoded.contains("symbols"));
    }

    #[test]
    fn curated_records_contain_only_reviewed_build_blocked_story_rows() {
        let records = curated_records_v1();
        assert_eq!(records.npcs.len(), 2);
        assert_eq!(records.quest_parents.len(), 1);

        let asghan = records
            .npcs
            .iter()
            .find(|npc| npc.runtime_unique_name == "OM_GRD_Asghan_263")
            .unwrap();
        assert_eq!(
            asghan.authoring_qualification,
            NpcAuthoringQualification::OfflineQualified
        );
        assert_eq!(
            asghan.runtime_qualification,
            RuntimeQualification::RuntimeUnqualified
        );

        let viper = records
            .npcs
            .iter()
            .find(|npc| npc.runtime_unique_name == "OM_STT_Viper_302")
            .unwrap();
        assert_eq!(
            viper.authoring_qualification,
            NpcAuthoringQualification::OfflineQualified
        );
        assert_eq!(
            viper.runtime_qualification,
            RuntimeQualification::RuntimeUnqualified
        );
        assert_eq!(viper.evidence_id, VIPER_OFFLINE_EVIDENCE_ID);
        validate_catalog_id(&viper.evidence_id).unwrap();
        assert_eq!(
            serde_json::to_value(viper).unwrap()["blocks_build"],
            serde_json::Value::Bool(true)
        );

        let quest = &records.quest_parents[0];
        assert_eq!(quest.quest_class.class_name, "UQuest_SwampCamp_SCCHAPTER2");
        assert_eq!(quest.parent_class_name, "UQuest_SwampCamp");
        assert_eq!(
            quest.transition_qualification,
            RuntimeQualification::RuntimeUnqualified
        );
    }

    #[test]
    fn record_order_does_not_change_catalog_bytes() {
        let generation = known_generation_v1();
        let normal =
            build_wire_from_verified_records(generation.clone(), curated_records_v1()).unwrap();
        let mut reversed = curated_records_v1();
        reversed.npcs.reverse();
        reversed.quest_parents.reverse();
        let reversed = build_wire_from_verified_records(generation, reversed).unwrap();
        assert_eq!(
            canonical_json(&normal, "story catalog", MAX_CATALOG_JSON_BYTES).unwrap(),
            canonical_json(&reversed, "story catalog", MAX_CATALOG_JSON_BYTES).unwrap()
        );
    }

    #[test]
    fn strict_json_rejects_duplicates_noncanonical_and_false_build_gate() {
        let catalog = trusted_catalog();
        let bytes = catalog.to_canonical_json().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        let duplicate = text.replacen(
            "\"format\":\"story_catalog\"",
            "\"format\":\"story_catalog\",\"format\":\"story_catalog\"",
            1,
        );
        assert!(matches!(
            StoryCatalogFile::from_json(duplicate.as_bytes()),
            Err(CatalogError::InvalidJson { .. })
        ));
        let mut noncanonical = bytes;
        noncanonical.push(b'\n');
        assert!(matches!(
            StoryCatalogFile::from_json(&noncanonical),
            Err(CatalogError::NonCanonicalJson { .. })
        ));
        let false_gate = text.replacen("\"blocks_build\":true", "\"blocks_build\":false", 1);
        assert!(matches!(
            StoryCatalogFile::from_json(false_gate.as_bytes()),
            Err(CatalogError::InvalidJson { .. })
        ));
    }

    #[test]
    fn trusted_reader_rejects_forged_generation_records_and_reordering_even_when_resealed() {
        let catalog = trusted_catalog();

        let mut forged_generation = catalog.wire.clone();
        forged_generation.catalog.generation.edition = "g1r-steam-forged".to_owned();
        reseal_wire(&mut forged_generation);
        let bytes =
            canonical_json(&forged_generation, "story catalog", MAX_CATALOG_JSON_BYTES).unwrap();
        assert!(matches!(
            StoryCatalogFile::from_json(&bytes),
            Err(CatalogError::UnsupportedGeneration { .. })
        ));

        let mut forged_qualification = catalog.wire.clone();
        forged_qualification.catalog.npcs[1].authoring_qualification =
            NpcAuthoringQualification::CatalogObserved;
        reseal_wire(&mut forged_qualification);
        let bytes = canonical_json(
            &forged_qualification,
            "story catalog",
            MAX_CATALOG_JSON_BYTES,
        )
        .unwrap();
        assert!(matches!(
            StoryCatalogFile::from_json(&bytes),
            Err(CatalogError::UntrustedCatalog(_))
        ));

        let mut reordered = catalog.wire.clone();
        reordered.catalog.npcs.reverse();
        reseal_wire(&mut reordered);
        let bytes = canonical_json(&reordered, "story catalog", MAX_CATALOG_JSON_BYTES).unwrap();
        assert!(matches!(
            StoryCatalogFile::from_json(&bytes),
            Err(CatalogError::Invariant(message)) if message.contains("strictly increasing")
        ));
    }

    #[test]
    fn trusted_reader_rejects_pre_viper_offline_catalog_bytes_and_status() {
        let catalog = trusted_catalog();
        let mut old = catalog.wire.clone();
        let viper = old
            .catalog
            .npcs
            .iter_mut()
            .find(|npc| npc.runtime_unique_name == "OM_STT_Viper_302")
            .unwrap();
        viper.authoring_qualification = NpcAuthoringQualification::CatalogObserved;
        viper.evidence_id = "current-cache-defaults-viper-20260712".to_owned();
        reseal_wire(&mut old);

        assert_eq!(old.catalog.record_set_seal.byte_len, 5_410);
        assert_eq!(
            old.catalog.record_set_seal.sha256.to_string(),
            "c6ca7fc2537046c767468181b8e2301758035343d165a3f8e02dc3ae8f670de0"
        );
        assert_eq!(old.catalog_seal.byte_len, 5_522);
        assert_eq!(
            old.catalog_seal.sha256.to_string(),
            "62f17d78b4d18be4809aba4cadf8530943d21590e21f4a85b46123accc115072"
        );

        let old_bytes = canonical_json(&old, "old story catalog", MAX_CATALOG_JSON_BYTES).unwrap();
        assert!(String::from_utf8_lossy(&old_bytes).contains("\"catalog_observed\""));
        assert!(matches!(
            StoryCatalogFile::from_json(&old_bytes),
            Err(CatalogError::UntrustedCatalog(_))
        ));
    }

    #[test]
    fn parser_rejects_nested_duplicate_unknown_depth_and_oversize_sequences() {
        let catalog = trusted_catalog();
        let bytes = catalog.to_canonical_json().unwrap();
        let text = String::from_utf8(bytes).unwrap();

        let nested_duplicate = text.replacen(
            "\"display_name\":\"Asghan\"",
            "\"display_name\":\"Asghan\",\"display_name\":\"Asghan\"",
            1,
        );
        assert!(matches!(
            StoryCatalogFile::from_json(nested_duplicate.as_bytes()),
            Err(CatalogError::InvalidJson { .. })
        ));

        let nested_unknown = text.replacen(
            "\"display_name\":\"Asghan\"",
            "\"display_name\":\"Asghan\",\"unexpected_nested_field\":0",
            1,
        );
        assert!(matches!(
            StoryCatalogFile::from_json(nested_unknown.as_bytes()),
            Err(CatalogError::InvalidJson { .. })
        ));

        let depth_value = format!("{}0{}", "[".repeat(160), "]".repeat(160));
        let deep_unknown = text.replacen(
            "\"schema_revision\":1",
            &format!("\"schema_revision\":1,\"unexpected_depth\":{depth_value}"),
            1,
        );
        assert!(matches!(
            StoryCatalogFile::from_json(deep_unknown.as_bytes()),
            Err(CatalogError::InvalidJson { .. })
        ));

        let mut oversized = catalog.wire.clone();
        oversized
            .catalog
            .npcs
            .push(oversized.catalog.npcs[0].clone());
        let bytes = canonical_json(&oversized, "story catalog", MAX_CATALOG_JSON_BYTES).unwrap();
        assert!(matches!(
            StoryCatalogFile::from_json(&bytes),
            Err(CatalogError::InvalidJson { .. })
        ));
    }

    #[test]
    fn folded_identities_require_strict_ascii_grammars() {
        let generation = known_generation_v1();
        let mut bad_catalog_id = curated_records_v1();
        bad_catalog_id.npcs[0].catalog_id = "g1r:npc:asghän".to_owned();
        assert!(matches!(
            build_wire_from_verified_records(generation.clone(), bad_catalog_id),
            Err(CatalogError::Invariant(message)) if message.contains("lowercase ASCII slug")
        ));

        let mut bad_runtime_name = curated_records_v1();
        bad_runtime_name.npcs[0].runtime_unique_name = "Asghän".to_owned();
        assert!(matches!(
            build_wire_from_verified_records(generation, bad_runtime_name),
            Err(CatalogError::Invariant(message)) if message.contains("ASCII identifier grammar")
        ));
    }

    #[test]
    fn json_and_input_limits_fail_closed() {
        let oversized = vec![b' '; MAX_CATALOG_JSON_BYTES + 1];
        assert!(matches!(
            StoryCatalogFile::from_json(&oversized),
            Err(CatalogError::LimitExceeded {
                kind: "story catalog JSON bytes",
                ..
            })
        ));
        assert!(matches!(
            canonical_json(&"123456789", "bounded test JSON", 8),
            Err(CatalogError::LimitExceeded {
                kind: "bounded test JSON",
                ..
            })
        ));

        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("exe");
        fs::write(&file, b"12").unwrap();
        assert!(matches!(
            seal_file(&file, 1, "executable bytes"),
            Err(CatalogError::LimitExceeded { .. })
        ));
        assert!(matches!(
            read_bounded(&file, 1, "bounded input bytes"),
            Err(CatalogError::LimitExceeded {
                kind: "bounded input bytes",
                ..
            })
        ));
    }

    #[test]
    fn stale_and_record_generation_mismatch_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("exe");
        let cache = root.path().join("cache");
        let binds = root.path().join("binds");
        fs::write(&executable, b"exe").unwrap();
        fs::write(&cache, b"cache").unwrap();
        fs::write(&binds, b"binds").unwrap();
        let paths = GenerationPaths {
            executable,
            shipping_cache: cache,
            binds_cache: binds,
        };
        assert!(matches!(
            build_known_catalog(&paths, GenerationInputLimits::default()),
            Err(CatalogError::UnsupportedGeneration { .. })
        ));

        let mut records = synthetic_records();
        records.generation = synthetic_generation(9);
        assert!(matches!(
            build_wire_from_verified_records(synthetic_generation(1), records),
            Err(CatalogError::RecordGenerationMismatch)
        ));
    }

    #[test]
    fn identity_collisions_are_case_insensitive_and_global() {
        let generation = known_generation_v1();
        let mut records = curated_records_v1();
        records.npcs[1].runtime_unique_name = records.npcs[0].runtime_unique_name.clone();
        assert!(matches!(
            build_wire_from_verified_records(generation.clone(), records),
            Err(CatalogError::Collision {
                kind: "NPC runtime_unique_name",
                ..
            })
        ));

        let mut records = curated_records_v1();
        records.npcs[1].spawn_definition.class_name = records.npcs[0]
            .spawn_definition
            .class_name
            .to_ascii_uppercase();
        records.npcs[1].spawn_definition.canonical_selector = format!(
            "script-class:{}/{}",
            records.npcs[1].spawn_definition.module, records.npcs[1].spawn_definition.class_name
        );
        assert!(matches!(
            build_wire_from_verified_records(generation, records),
            Err(CatalogError::Collision {
                kind: "script class",
                ..
            })
        ));
    }

    #[test]
    fn selector_and_path_must_be_derived_from_class_identity() {
        let generation = known_generation_v1();
        let mut bad_selector = curated_records_v1();
        bad_selector.npcs[0].character_definition.canonical_selector =
            "script-class:wrong/UWrong".to_owned();
        assert!(matches!(
            build_wire_from_verified_records(generation.clone(), bad_selector),
            Err(CatalogError::Invariant(message)) if message.contains("noncanonical script selector")
        ));

        let mut bad_path = curated_records_v1();
        bad_path.npcs[0].character_definition.relative_path = "wrong.as".to_owned();
        assert!(matches!(
            build_wire_from_verified_records(generation, bad_path),
            Err(CatalogError::Invariant(message)) if message.contains("disagrees with module")
        ));
    }

    #[test]
    fn payload_and_record_seal_drift_are_rejected() {
        let catalog = trusted_catalog();
        let mut record_drift = catalog.clone();
        record_drift.wire.catalog.record_set_seal.byte_len += 1;
        assert!(matches!(
            record_drift.to_canonical_json(),
            Err(CatalogError::SealMismatch {
                kind: "verified extraction records",
                ..
            })
        ));

        let mut payload_drift = catalog;
        payload_drift.wire.catalog_seal.byte_len += 1;
        assert!(matches!(
            payload_drift.to_canonical_json(),
            Err(CatalogError::SealMismatch {
                kind: "story catalog payload",
                ..
            })
        ));
    }

    #[test]
    fn atomic_publication_replaces_then_reopens_exactly() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nested").join("story_catalog.v1.json");
        let mut catalog = trusted_catalog();
        attach_test_input_guard(&mut catalog, root.path());
        publish_catalog_atomic(&path, &catalog).unwrap();
        assert_eq!(read_catalog(&path).unwrap(), catalog);

        fs::write(&path, b"old bytes").unwrap();
        publish_catalog_atomic(&path, &catalog).unwrap();
        assert_eq!(read_catalog(&path).unwrap(), catalog);
        let residue: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(residue.is_empty());
    }

    #[test]
    fn secure_file_reads_reject_hardlinks_and_final_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("original.bin");
        let hardlink = root.path().join("hardlink.bin");
        fs::write(&original, b"sealed bytes").unwrap();
        fs::hard_link(&original, &hardlink).unwrap();
        assert!(matches!(
            seal_file_guarded(&original, 1024, "test bytes"),
            Err(CatalogError::UnsafeInput(path)) if path == original
        ));

        let symlink_target = root.path().join("symlink-target.bin");
        let symlink = root.path().join("symlink.bin");
        fs::write(&symlink_target, b"target bytes").unwrap();
        match create_file_symlink(&symlink_target, &symlink) {
            Ok(()) => assert!(matches!(
                seal_file_guarded(&symlink, 1024, "test bytes"),
                Err(CatalogError::UnsafeInput(path)) if path == symlink
            )),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => panic!("could not create test file symlink: {error}"),
        }

        let catalog = trusted_catalog();
        let catalog_path = root.path().join("catalog.json");
        let catalog_hardlink = root.path().join("catalog-hardlink.json");
        fs::write(&catalog_path, catalog.to_canonical_json().unwrap()).unwrap();
        fs::hard_link(&catalog_path, &catalog_hardlink).unwrap();
        assert!(matches!(
            read_catalog(&catalog_path),
            Err(CatalogError::UnsafeInput(path)) if path == catalog_path
        ));
    }

    #[test]
    fn handle_change_stamp_detects_same_length_rewrite() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("same-length.bin");
        fs::write(&path, b"before").unwrap();
        let (before_handle, before) = open_regular_no_follow(&path, true).unwrap();
        drop(before_handle);

        // Some Windows filesystems coalesce metadata timestamps for back-to-back rewrites. Cross
        // that clock tick so this test exercises the intended change-stamp branch deterministically.
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&path, b"after!").unwrap();
        let (after_handle, after) = open_regular_no_follow(&path, true).unwrap();
        drop(after_handle);
        assert_eq!(before.identity, after.identity);
        assert_eq!(before.byte_len, after.byte_len);
        assert_ne!(before.change_stamp.values[0], after.change_stamp.values[0]);
    }

    #[test]
    fn publisher_rejects_input_aliases_hardlinks_reparses_and_changed_identities() {
        let root = tempfile::tempdir().unwrap();
        let mut catalog = trusted_catalog();
        let paths = attach_test_input_guard(&mut catalog, root.path());
        let executable_before = fs::read(&paths.executable).unwrap();
        assert!(matches!(
            publish_catalog_atomic(&paths.executable, &catalog),
            Err(CatalogError::OutputAliasesInput { .. })
        ));
        assert_eq!(fs::read(&paths.executable).unwrap(), executable_before);

        let unrelated = root.path().join("unrelated.bin");
        let hardlinked_output = root.path().join("hardlinked-output.json");
        fs::write(&unrelated, b"unrelated").unwrap();
        fs::hard_link(&unrelated, &hardlinked_output).unwrap();
        assert!(matches!(
            publish_catalog_atomic(&hardlinked_output, &catalog),
            Err(CatalogError::UnsafeInput(path)) if path == hardlinked_output
        ));

        let final_target = root.path().join("final-target.bin");
        let final_symlink = root.path().join("final-symlink.json");
        fs::write(&final_target, b"target").unwrap();
        match create_file_symlink(&final_target, &final_symlink) {
            Ok(()) => assert!(matches!(
                publish_catalog_atomic(&final_symlink, &catalog),
                Err(CatalogError::UnsafeInput(path)) if path == final_symlink
            )),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => panic!("could not create output symlink: {error}"),
        }

        let real_directory = root.path().join("real-directory");
        let linked_directory = root.path().join("linked-directory");
        fs::create_dir(&real_directory).unwrap();
        match create_directory_symlink(&real_directory, &linked_directory) {
            Ok(()) => {
                let output = linked_directory.join("catalog.json");
                assert!(matches!(
                    publish_catalog_atomic(&output, &catalog),
                    Err(CatalogError::UnsafeInput(path)) if path == linked_directory
                ));
                assert!(!real_directory.join("catalog.json").exists());
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => panic!("could not create output directory symlink: {error}"),
        }

        fs::remove_file(&paths.binds_cache).unwrap();
        fs::write(&paths.binds_cache, b"replacement guard").unwrap();
        let output = root.path().join("changed-input-output.json");
        assert!(matches!(
            publish_catalog_atomic(&output, &catalog),
            Err(CatalogError::IdentityChanged(path)) if path == paths.binds_cache
        ));
        assert!(!output.exists());
    }

    #[test]
    fn publisher_distinguishes_precommit_durability_and_verification_failures() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("missing-guard.json");
        assert!(matches!(
            publish_catalog_atomic(&output, &trusted_catalog()),
            Err(CatalogError::MissingInputGuard)
        ));
        assert!(!output.exists());

        let mut precommit_catalog = trusted_catalog();
        attach_test_input_guard(&mut precommit_catalog, root.path());
        let precommit_output = root.path().join("injected-precommit.json");
        assert!(matches!(
            publish_catalog_atomic_with_hooks(
                &precommit_output,
                &precommit_catalog,
                |temporary_path| {
                    assert!(temporary_path.exists());
                    Err(CatalogError::Invariant(
                        "injected precommit failure".to_owned(),
                    ))
                },
                sync_parent_directory,
            ),
            Err(CatalogError::Invariant(message)) if message == "injected precommit failure"
        ));
        assert!(!precommit_output.exists());
        let temporary_residue: Vec<_> = fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".story-catalog-")
            })
            .collect();
        assert!(temporary_residue.is_empty());

        let mut uncertain_catalog = trusted_catalog();
        attach_test_input_guard(&mut uncertain_catalog, root.path());
        let uncertain_output = root.path().join("uncertain.json");
        assert!(matches!(
            publish_catalog_atomic_with_durability(
                &uncertain_output,
                &uncertain_catalog,
                |_| Err(io::Error::other("injected directory sync failure")),
            ),
            Err(CatalogError::CommittedDurabilityUncertain { path, .. })
                if path == uncertain_output
        ));
        assert_eq!(read_catalog(&uncertain_output).unwrap(), uncertain_catalog);

        let mut verification_catalog = trusted_catalog();
        attach_test_input_guard(&mut verification_catalog, root.path());
        let verification_output = root.path().join("verification-failed.json");
        assert!(matches!(
            publish_catalog_atomic_with_durability(
                &verification_output,
                &verification_catalog,
                |_| {
                    fs::write(&verification_output, b"tampered after commit")?;
                    Ok(())
                },
            ),
            Err(CatalogError::CommittedVerificationFailed { path, .. })
                if path == verification_output
        ));
        assert_eq!(
            fs::read(&verification_output).unwrap(),
            b"tampered after commit"
        );
    }
}
