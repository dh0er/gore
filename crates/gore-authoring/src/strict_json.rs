use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// Walk an arbitrary JSON value without materializing it and reject duplicate object keys at
/// every depth. Callers still parse the same raw bytes through their closed target type after
/// this preflight; a `serde_json::Value` round trip must never sit between the wire and that type.
pub(crate) fn reject_duplicate_object_keys(json: &str) -> Result<(), serde_json::Error> {
    serde_json::from_str::<DuplicateSafeIgnored>(json).map(|_| ())
}

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
