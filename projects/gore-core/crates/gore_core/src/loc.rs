//! AlkimiaLocalization `.lcache` codec — Gothic 1 Remake's encrypted localization DB.
//!
//! The whole file is AES-256-ECB encrypted. Decrypted layout:
//!   prefix:      1 byte
//!   magic:       i32 len + bytes  (== "LCACHE")
//!   lang_count:  i32
//!   languages:   lang_count × FString
//!   group_count: i32
//!   groups:      group_count × (main Record, meta Record)
//!                Record = key FString, pair_count i32, pairs × (lang FString, value FString)
//!   tail:        zero padding to a 16-byte boundary
//!
//! FString: i32 count. `0` = empty; `> 0` = `count` UTF-8 bytes (incl trailing
//! NUL); `< 0` = `-count` UTF-16LE units (incl trailing NUL).
//!
//! Each group's `main` record holds the localized strings: key = text id, pairs
//! = one (language, value) per language. `export` flattens that to
//! `{ id: { language: value } }`. Editing preserves every unchanged field's
//! original bytes, so re-encoding a file with no edits is byte-identical.
//!
//! Format ported from a community Python tool and validated byte-exact against
//! the shipped `AlkimiaLocalization_00000000.lcache`.

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;
use std::collections::BTreeMap;

/// AES-256 key: the 32 ASCII bytes of this string (used verbatim, not hex-decoded).
const AES_KEY: &[u8; 32] = b"8f93ff6fa254d9c536ad88c1ff1d812b";
const MAGIC: &[u8] = b"LCACHE";

#[derive(Debug, thiserror::Error)]
pub enum LcacheError {
    #[error("data size {0} is not a multiple of the 16-byte AES block")]
    BadSize(usize),
    #[error("unexpected end of data at offset {0}")]
    Eof(usize),
    #[error("LCACHE magic not found after decryption")]
    BadMagic,
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error("non-zero bytes after parsed data (format mismatch)")]
    DirtyTail,
    #[error("key '{0}' not found")]
    KeyNotFound(String),
    #[error("language '{lang}' not found for key '{key}'")]
    LangNotFound { key: String, lang: String },
}

fn aes_ecb(data: &[u8], encrypt: bool) -> Result<Vec<u8>, LcacheError> {
    if data.len() % 16 != 0 {
        return Err(LcacheError::BadSize(data.len()));
    }
    let cipher = Aes256::new(GenericArray::from_slice(AES_KEY));
    let mut out = data.to_vec();
    for chunk in out.chunks_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        if encrypt {
            cipher.encrypt_block(block);
        } else {
            cipher.decrypt_block(block);
        }
    }
    Ok(out)
}

/// One length-prefixed string field. Keeps its original bytes so an unedited
/// field re-serializes identically.
#[derive(Clone, Debug)]
struct FString {
    raw: Vec<u8>,
    text: String,
    changed: bool,
}

impl FString {
    fn to_bytes(&self) -> Vec<u8> {
        if self.changed {
            encode_fstring(&self.text)
        } else {
            self.raw.clone()
        }
    }
}

/// Mirror of the game's writer: ASCII -> UTF-8 + NUL (positive byte count);
/// otherwise UTF-16LE + NUL (negative unit count).
fn encode_fstring(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    if text.is_ascii() {
        let mut raw = text.as_bytes().to_vec();
        raw.push(0);
        out.extend_from_slice(&(raw.len() as i32).to_le_bytes());
        out.extend_from_slice(&raw);
    } else {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut raw = Vec::with_capacity(units.len() * 2 + 2);
        for u in &units {
            raw.extend_from_slice(&u.to_le_bytes());
        }
        raw.extend_from_slice(&[0, 0]);
        let count = -((raw.len() / 2) as i32);
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&raw);
    }
    out
}

#[derive(Clone, Debug)]
struct Pair {
    lang: FString,
    value: FString,
}

#[derive(Clone, Debug)]
struct Record {
    key: FString,
    pairs: Vec<Pair>,
}

impl Record {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.key.to_bytes();
        out.extend_from_slice(&(self.pairs.len() as i32).to_le_bytes());
        for p in &self.pairs {
            out.extend_from_slice(&p.lang.to_bytes());
            out.extend_from_slice(&p.value.to_bytes());
        }
        out
    }
}

#[derive(Clone, Debug)]
struct Group {
    main: Record,
    meta: Record,
}

/// A parsed `.lcache`.
pub struct Lcache {
    prefix: Vec<u8>,
    magic_raw: Vec<u8>,
    lang_count_raw: Vec<u8>,
    languages: Vec<FString>,
    group_count: i32,
    groups: Vec<Group>,
    tail: Vec<u8>,
}

struct Reader<'a> {
    data: &'a [u8],
    off: usize,
}

impl<'a> Reader<'a> {
    fn read_i32(&mut self) -> Result<i32, LcacheError> {
        if self.off + 4 > self.data.len() {
            return Err(LcacheError::Eof(self.off));
        }
        let v = i32::from_le_bytes(self.data[self.off..self.off + 4].try_into().unwrap());
        self.off += 4;
        Ok(v)
    }

    /// Length-prefixed raw bytes (i32 len + len bytes); returns the payload.
    fn read_raw_string(&mut self) -> Result<Vec<u8>, LcacheError> {
        let len = self.read_i32()?;
        if len < 0 || self.off + len as usize > self.data.len() {
            return Err(LcacheError::Invalid("raw string length"));
        }
        let payload = self.data[self.off..self.off + len as usize].to_vec();
        self.off += len as usize;
        Ok(payload)
    }

    fn read_fstring(&mut self) -> Result<FString, LcacheError> {
        let start = self.off;
        let count = self.read_i32()?;
        if count == 0 {
            return Ok(FString {
                raw: self.data[start..self.off].to_vec(),
                text: String::new(),
                changed: false,
            });
        }
        if count > 0 {
            let n = count as usize;
            if self.off + n > self.data.len() {
                return Err(LcacheError::Invalid("FString byte length"));
            }
            let bytes = &self.data[self.off..self.off + n];
            self.off += n;
            let trimmed = if bytes.last() == Some(&0) { &bytes[..n - 1] } else { bytes };
            let text = String::from_utf8_lossy(trimmed).into_owned();
            Ok(FString { raw: self.data[start..self.off].to_vec(), text, changed: false })
        } else {
            // `count.unsigned_abs()` avoids the `-count` overflow when count is
            // i32::MIN (which would panic in debug builds).
            let units = count.unsigned_abs() as usize;
            let bytes = units * 2;
            if self.off + bytes > self.data.len() {
                return Err(LcacheError::Invalid("FString wide length"));
            }
            let slice = &self.data[self.off..self.off + bytes];
            self.off += bytes;
            let mut u16s: Vec<u16> = slice
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            if u16s.last() == Some(&0) {
                u16s.pop();
            }
            let text = String::from_utf16_lossy(&u16s);
            Ok(FString { raw: self.data[start..self.off].to_vec(), text, changed: false })
        }
    }

    fn read_record(&mut self) -> Result<Record, LcacheError> {
        let key = self.read_fstring()?;
        let pair_count = self.read_i32()?;
        if !(0..=256).contains(&pair_count) {
            return Err(LcacheError::Invalid("pair count"));
        }
        let mut pairs = Vec::with_capacity(pair_count as usize);
        for _ in 0..pair_count {
            let lang = self.read_fstring()?;
            let value = self.read_fstring()?;
            pairs.push(Pair { lang, value });
        }
        Ok(Record { key, pairs })
    }
}

impl Lcache {
    /// Decrypt and parse an encrypted `.lcache` file's bytes.
    pub fn decode(encrypted: &[u8]) -> Result<Self, LcacheError> {
        let plain = aes_ecb(encrypted, false)?;
        if plain.len() < 16 {
            return Err(LcacheError::Invalid("file too small"));
        }
        let mut r = Reader { data: &plain, off: 0 };
        let prefix = plain[0..1].to_vec();
        r.off = 1;
        let magic = r.read_raw_string()?;
        if magic != MAGIC {
            return Err(LcacheError::BadMagic);
        }
        let lang_count_pos = r.off;
        let lang_count = r.read_i32()?;
        if !(1..=128).contains(&lang_count) {
            return Err(LcacheError::Invalid("language count"));
        }
        let lang_count_raw = plain[lang_count_pos..lang_count_pos + 4].to_vec();
        let mut languages = Vec::with_capacity(lang_count as usize);
        for _ in 0..lang_count {
            languages.push(r.read_fstring()?);
        }
        let group_count = r.read_i32()?;
        if group_count < 0 {
            return Err(LcacheError::Invalid("group count"));
        }
        // Each group is two records of at least 8 bytes (key length + pair
        // count), so a count exceeding what the remaining bytes allow is a
        // malformed/hostile file — reject before allocating or looping.
        if group_count as usize > (plain.len() - r.off) / 16 {
            return Err(LcacheError::Invalid("group count"));
        }
        let mut groups = Vec::with_capacity(group_count as usize);
        for _ in 0..group_count {
            let main = r.read_record()?;
            let meta = r.read_record()?;
            groups.push(Group { main, meta });
        }
        let tail = plain[r.off..].to_vec();
        if tail.iter().any(|&b| b != 0) {
            return Err(LcacheError::DirtyTail);
        }
        Ok(Lcache {
            prefix,
            magic_raw: plain[1..lang_count_pos].to_vec(),
            lang_count_raw,
            languages,
            group_count,
            groups,
            tail,
        })
    }

    /// Rebuild the plaintext and re-encrypt. Unedited fields keep their original
    /// bytes; the result is padded with zeros to a 16-byte boundary.
    pub fn encode(&self) -> Result<Vec<u8>, LcacheError> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.prefix);
        out.extend_from_slice(&self.magic_raw);
        out.extend_from_slice(&self.lang_count_raw);
        for l in &self.languages {
            out.extend_from_slice(&l.to_bytes());
        }
        out.extend_from_slice(&self.group_count.to_le_bytes());
        for g in &self.groups {
            out.extend_from_slice(&g.main.to_bytes());
            out.extend_from_slice(&g.meta.to_bytes());
        }
        out.extend_from_slice(&self.tail);
        let pad = (16 - (out.len() % 16)) % 16;
        out.extend(std::iter::repeat(0u8).take(pad));
        aes_ecb(&out, true)
    }

    /// All language tags declared in the file header.
    pub fn languages(&self) -> Vec<String> {
        self.languages.iter().map(|f| f.text.clone()).collect()
    }

    pub fn key_count(&self) -> usize {
        self.groups.len()
    }

    /// Flatten to `{ text_id: { language: value } }`, dropping empty values
    /// (and, when `keep_empty` is false, ids with no values at all).
    pub fn export(&self, keep_empty: bool) -> BTreeMap<String, BTreeMap<String, String>> {
        let mut out = BTreeMap::new();
        for g in &self.groups {
            let mut langs = BTreeMap::new();
            for p in &g.main.pairs {
                if keep_empty || !p.value.text.is_empty() {
                    langs.insert(p.lang.text.clone(), p.value.text.clone());
                }
            }
            if keep_empty || !langs.is_empty() {
                out.insert(g.main.key.text.clone(), langs);
            }
        }
        out
    }

    /// Set the value for (key, language). Returns an error if either is absent.
    pub fn set_value(&mut self, key: &str, lang: &str, text: &str) -> Result<(), LcacheError> {
        let group = self
            .groups
            .iter_mut()
            .find(|g| g.main.key.text == key)
            .ok_or_else(|| LcacheError::KeyNotFound(key.to_string()))?;
        let pair = group
            .main
            .pairs
            .iter_mut()
            .find(|p| p.lang.text == lang)
            .ok_or_else(|| LcacheError::LangNotFound {
                key: key.to_string(),
                lang: lang.to_string(),
            })?;
        pair.value.text = text.to_string();
        pair.value.changed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a tiny in-memory lcache the same way the game would, encrypt it, then
    // round-trip through decode/encode to prove the codec is byte-faithful.
    fn synthetic() -> Vec<u8> {
        fn fstr(s: &str) -> Vec<u8> {
            super::encode_fstring(s)
        }
        let mut plain = Vec::new();
        plain.push(0u8); // prefix
        plain.extend_from_slice(&(MAGIC.len() as i32).to_le_bytes());
        plain.extend_from_slice(MAGIC);
        plain.extend_from_slice(&2i32.to_le_bytes()); // lang_count
        plain.extend_from_slice(&fstr("english"));
        plain.extend_from_slice(&fstr("german"));
        plain.extend_from_slice(&1i32.to_le_bytes()); // group_count
                                                      // group 0 main
        plain.extend_from_slice(&fstr("itfo_cheese")); // key
        plain.extend_from_slice(&2i32.to_le_bytes()); // pair_count
        plain.extend_from_slice(&fstr("english"));
        plain.extend_from_slice(&fstr("Cheese"));
        plain.extend_from_slice(&fstr("german"));
        plain.extend_from_slice(&fstr("Käse")); // non-ascii -> utf16le path
                                                // group 0 meta (empty key, no pairs)
        plain.extend_from_slice(&fstr(""));
        plain.extend_from_slice(&0i32.to_le_bytes());
        // pad to 16
        let pad = (16 - (plain.len() % 16)) % 16;
        plain.extend(std::iter::repeat(0u8).take(pad));
        aes_ecb(&plain, true).unwrap()
    }

    #[test]
    fn decodes_keys_languages_and_values() {
        let lc = Lcache::decode(&synthetic()).unwrap();
        assert_eq!(lc.languages(), vec!["english", "german"]);
        assert_eq!(lc.key_count(), 1);
        let map = lc.export(false);
        assert_eq!(map["itfo_cheese"]["english"], "Cheese");
        assert_eq!(map["itfo_cheese"]["german"], "Käse");
    }

    #[test]
    fn reencode_without_edits_is_byte_identical() {
        let enc = synthetic();
        let lc = Lcache::decode(&enc).unwrap();
        assert_eq!(lc.encode().unwrap(), enc);
    }

    #[test]
    fn edit_round_trips_through_encrypt() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.set_value("itfo_cheese", "english", "Stinky Cheese").unwrap();
        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(re.export(false)["itfo_cheese"]["english"], "Stinky Cheese");
        // untouched value preserved
        assert_eq!(re.export(false)["itfo_cheese"]["german"], "Käse");
    }

    #[test]
    fn set_value_errors_on_missing_key_or_lang() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        assert!(matches!(
            lc.set_value("nope", "english", "x"),
            Err(LcacheError::KeyNotFound(_))
        ));
        assert!(matches!(
            lc.set_value("itfo_cheese", "klingon", "x"),
            Err(LcacheError::LangNotFound { .. })
        ));
    }
}
