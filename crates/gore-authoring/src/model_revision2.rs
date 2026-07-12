//! Closed schema-revision-2 model for authoring format 2.
//!
//! Revision 2 deliberately owns every entity, payload, reference, and payload-supporting enum.
//! That keeps revision 1 frozen when later revision-2 work adds NPC or quest variants. Stable
//! value objects such as IDs, project metadata, generation anchors, and asset-store records remain
//! shared across schema revisions.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;

use serde::de::{MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    ArchiveSeal, AssetRef, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor,
    LocaleCode, ProjectId, ProjectJsonError, ProjectMeta, MAX_PROJECT_JSON_BYTES,
};

/// Second closed schema revision carried inside authoring format 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchemaRevisionV2;

impl Serialize for SchemaRevisionV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(2)
    }
}

impl<'de> Deserialize<'de> for SchemaRevisionV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let revision = u32::deserialize(deserializer)?;
        if revision == 2 {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported authoring schema revision {revision}; expected 2"
            )))
        }
    }
}

/// Closed revision-2 entity kinds. This type is intentionally separate from revision 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    LocalizationEntry,
    DialogLine,
    VoiceSlot,
    VoiceTake,
}

/// Revision-2 project-qualified authored reference.
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
    pub loc_id: String,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub texts: BTreeMap<LocaleCode, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogLine {
    pub localization: TypedRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_hint: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub voice_slots: BTreeMap<LocaleCode, TypedRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceOperation {
    Add,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceMemberProof {
    Present { uncompressed_size: u64, crc32: u32 },
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceTarget {
    pub archive: String,
    pub member: String,
    pub operation: VoiceOperation,
    pub archive_seal: ArchiveSeal,
    pub member_proof: VoiceMemberProof,
}

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
    pub target_resolution: VoiceTargetResolution,
    #[serde(default)]
    pub candidates: Vec<TypedRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<TypedRef>,
}

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

/// Closed revision-2 payload set. Later variants can be added here without changing revision 1.
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

/// Canonical authoring snapshot for format 2, schema revision 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectRevision2 {
    pub format: FormatV2,
    pub schema_revision: SchemaRevisionV2,
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
struct ProjectRevision2Wire {
    format: FormatV2,
    schema_revision: SchemaRevisionV2,
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

impl<'de> Deserialize<'de> for ProjectRevision2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProjectRevision2Wire::deserialize(deserializer)?;
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

impl ProjectRevision2 {
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
    fn schema_marker_accepts_only_revision_two() {
        assert!(serde_json::from_str::<SchemaRevisionV2>("2").is_ok());
        assert!(serde_json::from_str::<SchemaRevisionV2>("1").is_err());
        assert!(serde_json::from_str::<SchemaRevisionV2>("3").is_err());
    }

    #[test]
    fn sha_digest_remains_a_valid_unique_map_key() {
        let digest = "ab".repeat(32).parse::<crate::Sha256Digest>().unwrap();
        let json = format!("{{\"{digest}\":{{\"byte_len\":1,\"media_type\":\"audio/ogg\"}}}}");
        let parsed = serde_json::from_str::<BTreeMap<crate::Sha256Digest, crate::AssetMeta>>(&json);
        assert!(parsed.is_ok());
    }
}
