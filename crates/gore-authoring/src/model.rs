use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::{MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::{EntityId, ProjectId, Sha256Digest};

/// The only accepted project format marker for this crate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FormatV2;

impl Serialize for FormatV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(2)
    }
}

impl<'de> Deserialize<'de> for FormatV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let format = u32::deserialize(deserializer)?;
        if format == 2 {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported authoring project format {format}; expected 2"
            )))
        }
    }
}

/// First closed schema revision carried inside format 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchemaRevisionV1;

impl Serialize for SchemaRevisionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for SchemaRevisionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let revision = u32::deserialize(deserializer)?;
        if revision == 1 {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported authoring schema revision {revision}; expected 1"
            )))
        }
    }
}

/// Error returned when a locale is not in the canonical subset used by phase 1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocaleCodeError {
    #[error("locale must not be empty")]
    Empty,
    #[error("locale exceeds 35 ASCII characters")]
    TooLong,
    #[error("locale language must contain 2..=8 lowercase ASCII letters")]
    InvalidLanguage,
    #[error("locale segment {index} is not 1..=8 ASCII letters or digits")]
    InvalidSegment { index: usize },
    #[error("locale is not canonical; expected {expected:?}")]
    NonCanonical { expected: String },
}

/// Canonical BCP-47-shaped locale used as a stable map key (`de`, `pt-BR`, `zh-Hans`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocaleCode(String);

impl LocaleCode {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for LocaleCode {
    type Err = LocaleCodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(LocaleCodeError::Empty);
        }
        if value.len() > 35 {
            return Err(LocaleCodeError::TooLong);
        }
        if !value.is_ascii() {
            return Err(LocaleCodeError::InvalidLanguage);
        }

        let segments = value.split('-').collect::<Vec<_>>();
        let language = segments[0];
        if !(2..=8).contains(&language.len())
            || !language.bytes().all(|byte| byte.is_ascii_lowercase())
        {
            return Err(LocaleCodeError::InvalidLanguage);
        }

        let mut canonical = language.to_owned();
        for (index, segment) in segments.iter().enumerate().skip(1) {
            if segment.is_empty()
                || segment.len() > 8
                || !segment.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                return Err(LocaleCodeError::InvalidSegment { index });
            }
            canonical.push('-');
            if segment.len() == 4 && segment.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                let mut bytes = segment.to_ascii_lowercase().into_bytes();
                bytes[0] = bytes[0].to_ascii_uppercase();
                canonical.push_str(std::str::from_utf8(&bytes).expect("ASCII locale segment"));
            } else if segment.len() == 2 && segment.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                canonical.push_str(&segment.to_ascii_uppercase());
            } else {
                canonical.push_str(&segment.to_ascii_lowercase());
            }
        }

        if value != canonical {
            return Err(LocaleCodeError::NonCanonical {
                expected: canonical,
            });
        }
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for LocaleCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for LocaleCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LocaleCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
}

/// Generic immutable file/content seal used for generation and catalog provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSeal {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

/// Distribution-independent game generation identity.
///
/// No Steam app/build identifier is assumed. The executable bytes are the generation anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameGenerationAnchor {
    pub executable: ContentSeal,
}

/// Closed phase-1 entity kinds. New kinds require a format-aware schema change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    LocalizationEntry,
    DialogLine,
    VoiceSlot,
    VoiceTake,
}

/// A project-qualified authored reference carrying identity and expected entity kind.
///
/// Resolution is exclusively against the named project's [`ProjectV2::entities`], and phase 1
/// requires that project to be the containing project. Vanilla or dependency catalog identities
/// are never resolved by `EntityId`; they live in [`OriginRef::Vanilla`] with an explicit catalog
/// layer, selector, and seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedRef {
    pub project_id: ProjectId,
    pub id: EntityId,
    pub expected_kind: EntityKind,
}

impl TypedRef {
    pub const fn new(project_id: ProjectId, id: EntityId, expected_kind: EntityKind) -> Self {
        Self {
            project_id,
            id,
            expected_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizationEntry {
    /// Exact localization identity; no lookup falls back to display names.
    pub loc_id: String,
    /// Authored values keyed by canonical project locale.
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub texts: BTreeMap<LocaleCode, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogLine {
    pub localization: TypedRef,
    /// Informational only until a qualified NPC identity catalog exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_hint: Option<String>,
    /// At most one semantic voice slot per project locale. A slot can retain multiple takes.
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub voice_slots: BTreeMap<LocaleCode, TypedRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRef {
    pub sha256: Sha256Digest,
    pub byte_len: u64,
    pub logical_name: String,
}

/// One immutable blob described by the project AssetStore index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetMeta {
    pub byte_len: u64,
    pub media_type: String,
}

/// Closed content-addressed AssetStore index. Blob/package I/O is intentionally out of scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetStoreIndex {
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub assets: BTreeMap<Sha256Digest, AssetMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSeal {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceOperation {
    Add,
    Replace,
}

/// Exact member-presence observation made against [`VoiceTarget::archive_seal`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceMemberProof {
    Present { uncompressed_size: u64, crc32: u32 },
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceTarget {
    /// One ZIP filename below the game's fixed VoiceOver directory.
    pub archive: String,
    /// Exact, case-sensitive member path inside the ZIP.
    pub member: String,
    pub operation: VoiceOperation,
    /// Exact source archive observed while authoring.
    pub archive_seal: ArchiveSeal,
    /// Present/absent proof from that exact sealed archive snapshot.
    pub member_proof: VoiceMemberProof,
}

/// Closed result of resolving a semantic line/language slot against sealed voice catalogs.
///
/// Draft work retains zero or every exact match; only `Resolved` is eligible for lowering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceTargetResolution {
    Unresolved,
    Ambiguous { candidates: Vec<VoiceTarget> },
    Resolved { target: VoiceTarget },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OggCodec {
    Vorbis,
    Opus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OggMetadata {
    pub codec: OggCodec,
    pub channels: u8,
    pub sample_rate: u32,
    pub pages: u32,
    pub logical_streams: u32,
}

/// Author-managed production state for one take. This is distinct from derived build/runtime
/// readiness, which users cannot set manually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceTakeStatus {
    Draft,
    Recorded,
    Reviewed,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceTake {
    pub locale: LocaleCode,
    pub asset: AssetRef,
    pub ogg: OggMetadata,
    pub status: VoiceTakeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSlot {
    pub locale: LocaleCode,
    /// Zero, multiple, or one exact deployment match from sealed language catalogs.
    pub target_resolution: VoiceTargetResolution,
    /// Ordered author-facing alternatives. Reuse is explicit through typed references.
    #[serde(default)]
    pub candidates: Vec<TypedRef>,
    /// The one candidate lowered for this slot. Draft slots may intentionally have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<TypedRef>,
}

/// Durable entity provenance. Only `Vanilla` resolves outside the authored graph, and it is
/// qualified by catalog layer, canonical selector, generation, and exact catalog seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum OriginRef {
    New {
        authored_runtime_id: String,
    },
    Vanilla {
        generation: GameGenerationAnchor,
        catalog_layer: String,
        canonical_selector: String,
        source_seal: ContentSeal,
    },
    Imported {
        importer: String,
        source_seal: ContentSeal,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        external_identity: Option<String>,
    },
    Generated {
        generator_id: String,
        generator_version: u32,
        owner: TypedRef,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EntityPayload {
    LocalizationEntry(LocalizationEntry),
    DialogLine(DialogLine),
    VoiceSlot(VoiceSlot),
    VoiceTake(VoiceTake),
}

impl EntityPayload {
    pub const fn kind(&self) -> EntityKind {
        match self {
            Self::LocalizationEntry(_) => EntityKind::LocalizationEntry,
            Self::DialogLine(_) => EntityKind::DialogLine,
            Self::VoiceSlot(_) => EntityKind::VoiceSlot,
            Self::VoiceTake(_) => EntityKind::VoiceTake,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: EntityId,
    pub display_name: String,
    pub origin: OriginRef,
    #[serde(default)]
    pub revision: u64,
    pub payload: EntityPayload,
}

impl Entity {
    pub const fn kind(&self) -> EntityKind {
        self.payload.kind()
    }
}

/// Canonical phase-1 authoring snapshot.
///
/// Every collection that affects serialization is ordered. IDs and digests are
/// strict lowercase fixed-width values, so a deserialize/serialize round trip
/// has one stable JSON spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectV2 {
    pub format: FormatV2,
    pub schema_revision: SchemaRevisionV1,
    pub project_id: ProjectId,
    #[serde(default)]
    pub revision: u64,
    pub meta: ProjectMeta,
    pub target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_locales")]
    pub authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub entities: BTreeMap<EntityId, Entity>,
    pub asset_store: AssetStoreIndex,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectV2Wire {
    format: FormatV2,
    schema_revision: SchemaRevisionV1,
    project_id: ProjectId,
    #[serde(default)]
    revision: u64,
    meta: ProjectMeta,
    target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_locales")]
    authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    entities: BTreeMap<EntityId, Entity>,
    asset_store: AssetStoreIndex,
}

impl<'de> Deserialize<'de> for ProjectV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProjectV2Wire::deserialize(deserializer)?;
        for (key, entity) in &wire.entities {
            if key != &entity.id {
                return Err(de::Error::custom(format!(
                    "entity map key {key} does not match embedded id {}",
                    entity.id
                )));
            }
        }
        Ok(Self {
            format: wire.format,
            schema_revision: wire.schema_revision,
            project_id: wire.project_id,
            revision: wire.revision,
            meta: wire.meta,
            target: wire.target,
            authoring_locales: wire.authoring_locales,
            entities: wire.entities,
            asset_store: wire.asset_store,
        })
    }
}

impl ProjectV2 {
    pub fn from_json(json: &str) -> Result<Self, ProjectJsonError> {
        if json.len() > MAX_PROJECT_JSON_BYTES {
            return Err(ProjectJsonError::InputTooLarge {
                actual: json.len(),
                limit: MAX_PROJECT_JSON_BYTES,
            });
        }
        serde_json::from_str(json).map_err(ProjectJsonError::InvalidJson)
    }

    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Coarse phase-1 ceiling checked before JSON parsing or model allocation.
pub const MAX_PROJECT_JSON_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ProjectJsonError {
    #[error("authoring project JSON exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid authoring project JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
}

fn deserialize_unique_locales<'de, D>(deserializer: D) -> Result<BTreeSet<LocaleCode>, D::Error>
where
    D: Deserializer<'de>,
{
    let locales = Vec::<LocaleCode>::deserialize(deserializer)?;
    let mut unique = BTreeSet::new();
    for locale in locales {
        if !unique.insert(locale.clone()) {
            return Err(de::Error::custom(format!(
                "duplicate authoring locale {locale}"
            )));
        }
    }
    Ok(unique)
}

fn deserialize_unique_btree_map<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord + fmt::Display,
    V: Deserialize<'de>,
{
    struct UniqueMapVisitor<K, V>(PhantomData<(K, V)>);

    impl<'de, K, V> Visitor<'de> for UniqueMapVisitor<K, V>
    where
        K: Deserialize<'de> + Ord + fmt::Display,
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object with unique keys")
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry()? {
                match values.entry(key) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        return Err(de::Error::custom(format!(
                            "duplicate map key {}",
                            entry.key()
                        )));
                    }
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(UniqueMapVisitor(PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_codes_use_one_canonical_spelling() {
        for value in ["de", "pt-BR", "zh-Hans", "de-1996"] {
            assert_eq!(value.parse::<LocaleCode>().unwrap().as_str(), value);
        }
        assert!(matches!(
            "PT-br".parse::<LocaleCode>(),
            Err(LocaleCodeError::InvalidLanguage) | Err(LocaleCodeError::NonCanonical { .. })
        ));
        assert!(matches!(
            "zh-hans".parse::<LocaleCode>(),
            Err(LocaleCodeError::NonCanonical { .. })
        ));
    }

    #[test]
    fn format_marker_rejects_everything_except_two() {
        assert!(serde_json::from_str::<FormatV2>("2").is_ok());
        assert!(serde_json::from_str::<FormatV2>("1").is_err());
        assert!(serde_json::from_str::<SchemaRevisionV1>("1").is_ok());
        assert!(serde_json::from_str::<SchemaRevisionV1>("2").is_err());
    }
}
