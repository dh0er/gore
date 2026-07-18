//! Revision-independent value objects used by the managed revision-3 project model.
//!
//! These format primitives have one canonical wire spelling and are shared by the working store,
//! snapshot V2, and revision-3 authoring flows.

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::{MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::Sha256Digest;

/// The only accepted managed-project format marker.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSeal {
    pub byte_len: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameGenerationAnchor {
    pub executable: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRef {
    pub sha256: Sha256Digest,
    pub byte_len: u64,
    pub logical_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetMeta {
    pub byte_len: u64,
    pub media_type: String,
}

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

/// Coarse ceiling checked before managed project JSON parsing or allocation.
pub const MAX_PROJECT_JSON_BYTES: usize = 16 * 1024 * 1024;

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
    fn foundation_wire_values_retain_their_canonical_spelling() {
        assert_eq!(serde_json::to_string(&FormatV2).unwrap(), "2");
        assert_eq!(
            serde_json::to_string(&"pt-BR".parse::<LocaleCode>().unwrap()).unwrap(),
            "\"pt-BR\""
        );
        assert!(serde_json::from_str::<LocaleCode>("\"PT-br\"").is_err());
    }
}
