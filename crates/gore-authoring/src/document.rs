//! Bounded raw-JSON dispatch across the closed schemas carried by authoring format 2.

use std::collections::BTreeSet;
use std::fmt;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::{ProjectJsonError, ProjectRevision2, ProjectV2, MAX_PROJECT_JSON_BYTES};

/// One parsed authoring document, dispatched by its format and schema revision markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectDocument {
    Revision1(ProjectV2),
    Revision2(ProjectRevision2),
}

impl ProjectDocument {
    /// Checks the bounded raw JSON for duplicate object keys, probes the two dispatch markers,
    /// and then parses the same bytes again through the selected closed revision model.
    pub fn from_json(json: &str) -> Result<Self, ProjectDocumentError> {
        if json.len() > MAX_PROJECT_JSON_BYTES {
            return Err(ProjectDocumentError::InputTooLarge {
                actual: json.len(),
                limit: MAX_PROJECT_JSON_BYTES,
            });
        }

        let probe = serde_json::from_str::<ProjectProbe>(json)
            .map_err(ProjectDocumentError::InvalidProbeJson)?;
        if probe.format != 2 {
            return Err(ProjectDocumentError::UnsupportedFormat {
                found: probe.format,
            });
        }

        match probe.schema_revision {
            1 => ProjectV2::from_json(json)
                .map(Self::Revision1)
                .map_err(ProjectDocumentError::InvalidRevision1),
            2 => ProjectRevision2::from_json(json)
                .map(Self::Revision2)
                .map_err(ProjectDocumentError::InvalidRevision2),
            found => Err(ProjectDocumentError::UnsupportedSchemaRevision { found }),
        }
    }

    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::Revision1(project) => project.to_canonical_json(),
            Self::Revision2(project) => project.to_canonical_json(),
        }
    }
}

impl Serialize for ProjectDocument {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Revision1(project) => project.serialize(serializer),
            Self::Revision2(project) => project.serialize(serializer),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectDocumentError {
    #[error("authoring project JSON exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid authoring project dispatch JSON: {0}")]
    InvalidProbeJson(#[source] serde_json::Error),
    #[error("unsupported authoring project format {found}; expected 2")]
    UnsupportedFormat { found: u32 },
    #[error("unsupported authoring schema revision {found}; expected 1 or 2")]
    UnsupportedSchemaRevision { found: u32 },
    #[error("invalid schema-revision-1 authoring project: {0}")]
    InvalidRevision1(#[source] ProjectJsonError),
    #[error("invalid schema-revision-2 authoring project: {0}")]
    InvalidRevision2(#[source] ProjectJsonError),
}

struct ProjectProbe {
    format: u32,
    schema_revision: u32,
}

impl<'de> Deserialize<'de> for ProjectProbe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ProjectProbeVisitor)
    }
}

struct ProjectProbeVisitor;

impl<'de> Visitor<'de> for ProjectProbeVisitor {
    type Value = ProjectProbe;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an authoring project JSON object")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut format = None;
        let mut schema_revision = None;

        while let Some(key) = access.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key {key:?}"
                )));
            }
            match key.as_str() {
                "format" => format = Some(access.next_value::<u32>()?),
                "schema_revision" => schema_revision = Some(access.next_value::<u32>()?),
                _ => {
                    access.next_value::<DuplicateSafeIgnored>()?;
                }
            }
        }

        Ok(ProjectProbe {
            format: format.ok_or_else(|| de::Error::missing_field("format"))?,
            schema_revision: schema_revision
                .ok_or_else(|| de::Error::missing_field("schema_revision"))?,
        })
    }
}

/// Allocation-light JSON sink that still descends into every object and rejects duplicate keys.
struct DuplicateSafeIgnored;

impl<'de> Deserialize<'de> for DuplicateSafeIgnored {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateSafeIgnoredVisitor)
    }
}

struct DuplicateSafeIgnoredVisitor;

impl<'de> Visitor<'de> for DuplicateSafeIgnoredVisitor {
    type Value = DuplicateSafeIgnored;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any duplicate-key-free JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while access.next_element::<DuplicateSafeIgnored>()?.is_some() {}
        Ok(DuplicateSafeIgnored)
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        while let Some(key) = access.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key {key:?}"
                )));
            }
            access.next_value::<DuplicateSafeIgnored>()?;
        }
        Ok(DuplicateSafeIgnored)
    }
}
