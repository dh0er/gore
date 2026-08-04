use std::fmt;
use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Error returned when a fixed-width lowercase hexadecimal identifier is malformed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FixedHexError {
    #[error("expected {expected} lowercase hexadecimal characters, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("invalid lowercase hexadecimal character at byte {index}: {character:?}")]
    InvalidCharacter { index: usize, character: char },
}

fn decode_lower_hex<const N: usize>(value: &str) -> Result<[u8; N], FixedHexError> {
    let expected = N * 2;
    if value.len() != expected {
        return Err(FixedHexError::InvalidLength {
            expected,
            actual: value.len(),
        });
    }

    let mut bytes = [0; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = lower_hex_nibble(pair[0]).ok_or_else(|| FixedHexError::InvalidCharacter {
            index: index * 2,
            character: char::from(pair[0]),
        })?;
        let low = lower_hex_nibble(pair[1]).ok_or_else(|| FixedHexError::InvalidCharacter {
            index: index * 2 + 1,
            character: char::from(pair[1]),
        })?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn lower_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

macro_rules! fixed_hex_type {
    ($name:ident, $bytes:expr, $label:literal) => {
        #[doc = $label]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; $bytes]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; $bytes]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; $bytes] {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = FixedHexError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                decode_lower_hex(value).map(Self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&encode_lower_hex(&self.0))
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(de::Error::custom)
            }
        }
    };
}

fixed_hex_type!(
    ProjectId,
    16,
    "Immutable 128-bit project identity, serialized as 32 lowercase hexadecimal characters."
);
fixed_hex_type!(
    EntityId,
    16,
    "Immutable 128-bit entity identity, serialized as 32 lowercase hexadecimal characters."
);
fixed_hex_type!(
    Sha256Digest,
    32,
    "SHA-256 digest, serialized as exactly 64 lowercase hexadecimal characters."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_hex_types_require_canonical_lowercase_width() {
        let entity: EntityId = "00112233445566778899aabbccddeeff".parse().unwrap();
        assert_eq!(entity.to_string(), "00112233445566778899aabbccddeeff");
        assert!(matches!(
            "0011".parse::<EntityId>(),
            Err(FixedHexError::InvalidLength { .. })
        ));
        assert!(matches!(
            "00112233445566778899AABBCCDDEEFF".parse::<EntityId>(),
            Err(FixedHexError::InvalidCharacter { index: 20, .. })
        ));

        let digest = "ab".repeat(32).parse::<Sha256Digest>().unwrap();
        assert_eq!(digest.to_string(), "ab".repeat(32));
    }
}
