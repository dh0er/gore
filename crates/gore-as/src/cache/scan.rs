//! Tolerant length-prefixed string scanner.
//!
//! The per-type record layout after the header is not yet fully reverse-
//! engineered. This scanner walks `u32`-length-prefixed ASCII names and resyncs
//! by advancing one byte when a candidate is not a plausible name. It is an
//! investigation aid for mapping the type table, NOT a format-accurate parser.

/// A name found by [`scan_strings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedString {
    /// Byte offset of the `u32` length prefix.
    pub offset: usize,
    /// The length prefix value (bytes of the name, may include a trailing NUL).
    pub len: u32,
    /// Decoded text with any trailing NUL stripped.
    pub text: String,
}

/// Scan up to `max` length-prefixed ASCII names starting at `start`.
pub fn scan_strings(bytes: &[u8], start: usize, max: usize) -> Vec<ScannedString> {
    let mut out = Vec::new();
    let mut o = start;
    while o + 4 <= bytes.len() && out.len() < max {
        let len = u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        if (1..=256).contains(&len) && o + 4 + len as usize <= bytes.len() {
            let raw = &bytes[o + 4..o + 4 + len as usize];
            if is_plausible_name(raw) {
                let text = String::from_utf8_lossy(raw)
                    .trim_end_matches('\0')
                    .to_string();
                out.push(ScannedString {
                    offset: o,
                    len,
                    text,
                });
                o += 4 + len as usize;
                continue;
            }
        }
        o += 1;
    }
    out
}

fn is_plausible_name(raw: &[u8]) -> bool {
    let body = raw.strip_suffix(b"\0").unwrap_or(raw);
    if body.is_empty() {
        return false;
    }
    body.iter()
        .all(|&c| c == b'.' || c == b'_' || c == b':' || c.is_ascii_alphanumeric())
}
