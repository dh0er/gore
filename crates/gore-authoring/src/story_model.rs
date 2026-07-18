//! Wire-stable Story value objects used by the managed revision-3 model.
//!
//! This module contains the canonical leaf values used throughout current Story authoring.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize};

use crate::{ArchiveSeal, AssetRef, ContentSeal, GameGenerationAnchor, LocaleCode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizationEntry {
    pub loc_id: String,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub texts: BTreeMap<LocaleCode, String>,
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
pub struct NpcParentClassInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub canonical_selector: String,
    pub runtime_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDraftInput {
    pub target: GameGenerationAnchor,
    pub module_namespace: String,
    pub unique_name: String,
    pub parent_character_definition: NpcParentClassInput,
    pub parent_ai_agent_config: NpcParentClassInput,
    pub parent_spawn_definition: NpcParentClassInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestGiverInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub canonical_selector: String,
    pub runtime_unique_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestParentInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub canonical_selector: String,
    pub runtime_class: String,
}

/// Complete collision inventory consumed while deterministically generating a Quest module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuestCollisionCatalogInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    #[serde(default)]
    pub modules: BTreeSet<String>,
    #[serde(default)]
    pub relative_paths: BTreeSet<String>,
    #[serde(default)]
    pub symbols: BTreeSet<String>,
}

impl<'de> Deserialize<'de> for QuestCollisionCatalogInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "generation",
            "source_seal",
            "catalog_layer",
            "modules",
            "relative_paths",
            "symbols",
        ];

        struct CollisionCatalogVisitor;

        impl<'de> Visitor<'de> for CollisionCatalogVisitor {
            type Value = QuestCollisionCatalogInput;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded quest collision catalog object")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut generation = None;
                let mut source_seal = None;
                let mut catalog_layer = None;
                let mut modules = None;
                let mut relative_paths = None;
                let mut symbols = None;
                let mut remaining_count = crate::quest::MAX_COLLISION_ENTRIES;
                let mut remaining_bytes = crate::quest::MAX_COLLISION_TOTAL_BYTES;

                while let Some(field) = access.next_key::<String>()? {
                    match field.as_str() {
                        "generation" => {
                            set_once(&mut generation, access.next_value()?, "generation")?
                        }
                        "source_seal" => {
                            set_once(&mut source_seal, access.next_value()?, "source_seal")?
                        }
                        "catalog_layer" => {
                            set_once(&mut catalog_layer, access.next_value()?, "catalog_layer")?
                        }
                        "modules" => {
                            if modules.is_some() {
                                return Err(de::Error::duplicate_field("modules"));
                            }
                            let bounded = access.next_value_seed(CollisionSetSeed {
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= bounded.count;
                            remaining_bytes -= bounded.bytes;
                            modules = Some(bounded.values);
                        }
                        "relative_paths" => {
                            if relative_paths.is_some() {
                                return Err(de::Error::duplicate_field("relative_paths"));
                            }
                            let bounded = access.next_value_seed(CollisionSetSeed {
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= bounded.count;
                            remaining_bytes -= bounded.bytes;
                            relative_paths = Some(bounded.values);
                        }
                        "symbols" => {
                            if symbols.is_some() {
                                return Err(de::Error::duplicate_field("symbols"));
                            }
                            let bounded = access.next_value_seed(CollisionSetSeed {
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= bounded.count;
                            remaining_bytes -= bounded.bytes;
                            symbols = Some(bounded.values);
                        }
                        _ => return Err(de::Error::unknown_field(&field, FIELDS)),
                    }
                }

                Ok(QuestCollisionCatalogInput {
                    generation: generation.ok_or_else(|| de::Error::missing_field("generation"))?,
                    source_seal: source_seal
                        .ok_or_else(|| de::Error::missing_field("source_seal"))?,
                    catalog_layer: catalog_layer
                        .ok_or_else(|| de::Error::missing_field("catalog_layer"))?,
                    modules: modules.unwrap_or_default(),
                    relative_paths: relative_paths.unwrap_or_default(),
                    symbols: symbols.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "QuestCollisionCatalogInput",
            FIELDS,
            CollisionCatalogVisitor,
        )
    }
}

fn set_once<E, T>(slot: &mut Option<T>, value: T, field: &'static str) -> Result<(), E>
where
    E: de::Error,
{
    if slot.replace(value).is_some() {
        Err(E::duplicate_field(field))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct BoundedCollisionSet {
    values: BTreeSet<String>,
    count: usize,
    bytes: usize,
}

pub(crate) struct CollisionSetSeed {
    pub(crate) remaining_count: usize,
    pub(crate) remaining_bytes: usize,
}

struct BoundedCollisionString(String);

impl<'de> Deserialize<'de> for BoundedCollisionString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoundedStringVisitor;

        impl Visitor<'_> for BoundedStringVisitor {
            type Value = BoundedCollisionString;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "a collision string of at most {} bytes",
                    crate::quest::MAX_COLLISION_ENTRY_BYTES
                )
            }

            fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(value)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                validate_collision_string_length(value)?;
                Ok(BoundedCollisionString(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                validate_collision_string_length(&value)?;
                Ok(BoundedCollisionString(value))
            }
        }

        deserializer.deserialize_string(BoundedStringVisitor)
    }
}

fn validate_collision_string_length<E>(value: &str) -> Result<(), E>
where
    E: de::Error,
{
    if value.len() > crate::quest::MAX_COLLISION_ENTRY_BYTES {
        Err(E::custom(format!(
            "collision entry is {} bytes; maximum is {}",
            value.len(),
            crate::quest::MAX_COLLISION_ENTRY_BYTES
        )))
    } else {
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for CollisionSetSeed {
    type Value = BoundedCollisionSet;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CollisionSetVisitor {
            remaining_count: usize,
            remaining_bytes: usize,
        }

        impl<'de> Visitor<'de> for CollisionSetVisitor {
            type Value = BoundedCollisionSet;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded array of unique collision strings")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                if sequence
                    .size_hint()
                    .is_some_and(|hint| hint > self.remaining_count)
                {
                    return Err(de::Error::custom(format!(
                        "collision catalog exceeds the remaining aggregate entry budget {}; maximum total is {}",
                        self.remaining_count,
                        crate::quest::MAX_COLLISION_ENTRIES,
                    )));
                }
                let mut values = BTreeSet::new();
                let mut count = 0usize;
                let mut bytes = 0usize;
                while let Some(BoundedCollisionString(value)) =
                    sequence.next_element::<BoundedCollisionString>()?
                {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| de::Error::custom("collision entry count overflow"))?;
                    if count > self.remaining_count {
                        return Err(de::Error::custom(format!(
                            "collision catalog exceeds the remaining aggregate entry budget {}; maximum total is {}",
                            self.remaining_count,
                            crate::quest::MAX_COLLISION_ENTRIES,
                        )));
                    }
                    bytes = bytes.checked_add(value.len()).ok_or_else(|| {
                        de::Error::custom("collision catalog byte count overflow")
                    })?;
                    if bytes > self.remaining_bytes {
                        return Err(de::Error::custom(format!(
                            "collision catalog exceeds the {}-byte limit",
                            crate::quest::MAX_COLLISION_TOTAL_BYTES
                        )));
                    }
                    if !values.insert(value) {
                        return Err(de::Error::custom("duplicate collision set value"));
                    }
                }
                Ok(BoundedCollisionSet {
                    values,
                    count,
                    bytes,
                })
            }
        }

        deserializer.deserialize_seq(CollisionSetVisitor {
            remaining_count: self.remaining_count,
            remaining_bytes: self.remaining_bytes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptAuthoringStatus {
    OfflineDraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptRuntimeStatus {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptModuleStatus {
    pub authoring: ScriptAuthoringStatus,
    pub runtime: ScriptRuntimeStatus,
}

impl ScriptModuleStatus {
    pub const OFFLINE_DRAFT_RUNTIME_UNQUALIFIED: Self = Self {
        authoring: ScriptAuthoringStatus::OfflineDraft,
        runtime: ScriptRuntimeStatus::RuntimeUnqualified,
    };
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
    fn collision_catalog_rejects_duplicate_entries_before_publication() {
        let json = r#"{
            "generation":{"executable":{"byte_len":1,"sha256":"0101010101010101010101010101010101010101010101010101010101010101"}},
            "source_seal":{"byte_len":1,"sha256":"0202020202020202020202020202020202020202020202020202020202020202"},
            "catalog_layer":"catalog.v1",
            "modules":["m","m"],"relative_paths":[],"symbols":[]
        }"#;
        assert!(serde_json::from_str::<QuestCollisionCatalogInput>(json).is_err());
    }
}
