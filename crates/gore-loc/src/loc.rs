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
    #[error("key '{0}' already exists")]
    DuplicateKey(String),
    #[error("localization key must not be empty")]
    EmptyKey,
    #[error("language '{0}' is not declared in the lcache header")]
    UnknownLanguage(String),
    #[error("language '{0}' was specified more than once (case-insensitively)")]
    DuplicateLanguage(String),
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
    fn new(text: impl Into<String>) -> Self {
        Self {
            raw: Vec::new(),
            text: text.into(),
            changed: true,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        if self.changed {
            encode_fstring(&self.text)
        } else {
            self.raw.clone()
        }
    }
}

/// Mirror of the game's writer: empty -> a lone `0` count (no payload, matching
/// how the decoder represents empty); ASCII -> UTF-8 + NUL (positive byte
/// count); otherwise UTF-16LE + NUL (negative unit count).
fn encode_fstring(text: &str) -> Vec<u8> {
    if text.is_empty() {
        return 0i32.to_le_bytes().to_vec();
    }
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
    /// Set once any value is edited. Keeps the original alignment `tail` only on
    /// an unedited round trip; once the serialized length changes, the tail is
    /// dropped and padding is recomputed (so repeated imports don't accrete
    /// extra trailing blocks).
    dirty: bool,
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
            let trimmed = if bytes.last() == Some(&0) {
                &bytes[..n - 1]
            } else {
                bytes
            };
            let text = String::from_utf8_lossy(trimmed).into_owned();
            Ok(FString {
                raw: self.data[start..self.off].to_vec(),
                text,
                changed: false,
            })
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
            Ok(FString {
                raw: self.data[start..self.off].to_vec(),
                text,
                changed: false,
            })
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
        let mut r = Reader {
            data: &plain,
            off: 0,
        };
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
            dirty: false,
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
        // Keep the original alignment tail only for an unedited round trip (byte
        // identical). Once a value changed, the serialized length differs, so the
        // old tail no longer aligns — drop it and let padding be recomputed below,
        // mirroring the game writer and avoiding extra blocks on repeat imports.
        if !self.dirty {
            out.extend_from_slice(&self.tail);
        }
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

    /// Return whether `key` exists, using the same exact-then-ASCII-case-insensitive
    /// matching as [`Self::set_value`].
    pub fn has_key(&self, key: &str) -> bool {
        self.find_key(key).is_some()
    }

    /// The language tags `key` actually carries, in stored order.
    ///
    /// The header declares every language the file knows, but the records are sparse: an id holds
    /// only the slots it has, which is why [`Self::set_value`] can fail on a language the header
    /// lists. Returns an empty vector when the key does not exist. Matching follows
    /// [`Self::set_value`]: exact first, then ASCII-case-insensitive.
    pub fn languages_for(&self, key: &str) -> Vec<&str> {
        match self.find_key(key) {
            Some(idx) => self.groups[idx]
                .main
                .pairs
                .iter()
                .map(|p| p.lang.text.as_str())
                .collect(),
            None => Vec::new(),
        }
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
    ///
    /// The key is matched exactly first, then case-insensitively: the editor/catalog canonicalizes
    /// loc ids to lowercase while the `.lcache` keeps each record's original casing, so a lowercased
    /// id must still update a mixed-case record rather than silently failing.
    /// Language names use the same exact-then-case-insensitive lookup while retaining the cache's
    /// canonical spelling on disk.
    pub fn set_value(&mut self, key: &str, lang: &str, text: &str) -> Result<(), LcacheError> {
        let idx = self
            .find_key(key)
            .ok_or_else(|| LcacheError::KeyNotFound(key.to_string()))?;
        let group = &mut self.groups[idx];
        let pair_idx = group
            .main
            .pairs
            .iter()
            .position(|p| p.lang.text == lang)
            .or_else(|| {
                group
                    .main
                    .pairs
                    .iter()
                    .position(|p| p.lang.text.eq_ignore_ascii_case(lang))
            })
            .ok_or_else(|| LcacheError::LangNotFound {
                key: key.to_string(),
                lang: lang.to_string(),
            })?;
        let pair = &mut group.main.pairs[pair_idx];
        pair.value.text = text.to_string();
        pair.value.changed = true;
        self.dirty = true;
        Ok(())
    }

    /// Insert a new localization id with the supplied language values.
    ///
    /// The id and language names are matched case-insensitively for validation.
    /// ASCII casing in new ids is canonicalized to the shipped cache's lowercase form,
    /// while stored language names use the canonical spelling and order declared
    /// by the file header. Languages without a supplied value are omitted, which
    /// mirrors the shape of the shipped cache. Validation is all-or-nothing: on
    /// error, the cache is left unchanged.
    pub fn add_key(
        &mut self,
        key: &str,
        values: &BTreeMap<String, String>,
    ) -> Result<(), LcacheError> {
        if key.is_empty() {
            return Err(LcacheError::EmptyKey);
        }
        let canonical_key = key.to_ascii_lowercase();
        if self.has_key(&canonical_key) {
            return Err(LcacheError::DuplicateKey(key.to_string()));
        }
        if self
            .groups
            .windows(2)
            .any(|pair| pair[0].main.key.text.as_str() > pair[1].main.key.text.as_str())
        {
            return Err(LcacheError::Invalid("localization id order"));
        }

        // Resolve every requested name before mutating `self`. A BTreeMap may
        // contain differently-cased spellings of the same language, so reject
        // that ambiguity explicitly instead of silently choosing one value.
        let mut resolved: BTreeMap<usize, &String> = BTreeMap::new();
        for (requested, value) in values {
            let lang_idx = self
                .find_language(requested)
                .ok_or_else(|| LcacheError::UnknownLanguage(requested.clone()))?;
            if resolved.insert(lang_idx, value).is_some() {
                return Err(LcacheError::DuplicateLanguage(requested.clone()));
            }
        }

        let pairs = resolved
            .into_iter()
            .map(|(lang_idx, value)| Pair {
                lang: FString::new(self.languages[lang_idx].text.clone()),
                value: FString::new(value.clone()),
            })
            .collect();
        let new_group_count = self
            .groups
            .len()
            .checked_add(1)
            .and_then(|count| i32::try_from(count).ok())
            .ok_or(LcacheError::Invalid("group count"))?;
        let group = Group {
            main: Record {
                key: FString::new(canonical_key.clone()),
                pairs,
            },
            // New text ids do not need optional metadata such as `Expression`;
            // real ordinary groups use an empty meta record.
            meta: Record {
                key: FString::new(""),
                pairs: Vec::new(),
            },
        };

        // The shipped cache is sorted by text id, and the runtime lookup relies on that ordering
        // (an otherwise well-formed id appended after `zombie` decodes offline but resolves as a
        // missing string-table entry in game). Keep the on-disk groups in the same ordinal order;
        // the existing records themselves remain byte-identical and are only shifted as a unit.
        let insert_at = self
            .groups
            .partition_point(|existing| existing.main.key.text.as_str() < canonical_key.as_str());
        self.groups.insert(insert_at, group);
        self.group_count = new_group_count;
        self.dirty = true;
        Ok(())
    }

    /// Update an existing `(key, language)` pair, insert a missing language pair
    /// into an existing key, or append a new key when the id is absent.
    pub fn set_or_add_value(
        &mut self,
        key: &str,
        lang: &str,
        text: &str,
    ) -> Result<(), LcacheError> {
        let lang_idx = self
            .find_language(lang)
            .ok_or_else(|| LcacheError::UnknownLanguage(lang.to_string()))?;
        let Some(group_idx) = self.find_key(key) else {
            let mut values = BTreeMap::new();
            values.insert(self.languages[lang_idx].text.clone(), text.to_string());
            return self.add_key(key, &values);
        };

        let canonical_lang = self.languages[lang_idx].text.clone();
        let group = &mut self.groups[group_idx];
        if let Some(pair) = group
            .main
            .pairs
            .iter_mut()
            .find(|p| p.lang.text.eq_ignore_ascii_case(&canonical_lang))
        {
            pair.value.text = text.to_string();
            pair.value.changed = true;
        } else {
            // Pair order follows header order. Preserve any unusual/unknown
            // existing pairs after the declared languages rather than dropping
            // or rewriting them.
            let header_order: BTreeMap<String, usize> = self
                .languages
                .iter()
                .enumerate()
                .map(|(idx, language)| (language.text.to_ascii_lowercase(), idx))
                .collect();
            let insert_at = group
                .main
                .pairs
                .iter()
                .position(|pair| {
                    header_order
                        .get(&pair.lang.text.to_ascii_lowercase())
                        .is_some_and(|&idx| idx > lang_idx)
                })
                .unwrap_or(group.main.pairs.len());
            group.main.pairs.insert(
                insert_at,
                Pair {
                    lang: FString::new(canonical_lang),
                    value: FString::new(text),
                },
            );
        }
        self.dirty = true;
        Ok(())
    }

    fn find_key(&self, key: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|g| g.main.key.text == key)
            .or_else(|| {
                self.groups
                    .iter()
                    .position(|g| g.main.key.text.eq_ignore_ascii_case(key))
            })
    }

    fn find_language(&self, lang: &str) -> Option<usize> {
        self.languages
            .iter()
            .position(|language| language.text == lang)
            .or_else(|| {
                self.languages
                    .iter()
                    .position(|language| language.text.eq_ignore_ascii_case(lang))
            })
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
        lc.set_value("itfo_cheese", "english", "Stinky Cheese")
            .unwrap();
        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(re.export(false)["itfo_cheese"]["english"], "Stinky Cheese");
        // untouched value preserved
        assert_eq!(re.export(false)["itfo_cheese"]["german"], "Käse");
    }

    #[test]
    fn empty_edit_encodes_as_zero_count_like_the_game() {
        // The decoder reads count==0 as empty, so clearing a value must encode
        // the same lone-zero-count form, not a length-1 NUL field.
        assert_eq!(super::encode_fstring(""), 0i32.to_le_bytes().to_vec());

        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.set_value("itfo_cheese", "english", "").unwrap();
        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(re.export(true)["itfo_cheese"]["english"], "");
    }

    #[test]
    fn repeated_import_of_an_edited_file_is_stable() {
        // A length-changing edit must not accrete trailing padding blocks when
        // the file is decoded and re-encoded again (dropping the stale tail).
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.set_value(
            "itfo_cheese",
            "english",
            "A considerably longer cheese name",
        )
        .unwrap();
        let once = lc.encode().unwrap();
        let twice = Lcache::decode(&once).unwrap().encode().unwrap();
        assert_eq!(once, twice, "re-importing must not grow or alter the file");
        assert_eq!(once.len() % 16, 0);
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

    #[test]
    fn set_value_matches_key_case_insensitively() {
        // The editor/catalog lowercases ids while the .lcache keeps original casing; a
        // differently-cased id must still update the record instead of silently failing.
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.set_value("ITFO_Cheese", "english", "Gouda").unwrap();
        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(re.export(false)["itfo_cheese"]["english"], "Gouda");
    }

    #[test]
    fn set_value_matches_language_case_insensitively() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.set_value("itfo_cheese", "English", "Gouda").unwrap();
        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(re.export(false)["itfo_cheese"]["english"], "Gouda");
    }

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(lang, value)| (lang.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn add_key_round_trips_and_preserves_existing_values() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.add_key(
            "itfo_bread",
            &values(&[("english", "Bread"), ("german", "Brötchen")]),
        )
        .unwrap();

        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        let map = re.export(false);
        assert_eq!(re.key_count(), 2);
        assert_eq!(map["itfo_bread"]["english"], "Bread");
        assert_eq!(map["itfo_bread"]["german"], "Brötchen");
        assert_eq!(map["itfo_cheese"]["english"], "Cheese");
    }

    /// Adding a group changes only the group count and inserts one record at its sorted position;
    /// the original header and group records stay byte-identical.
    #[test]
    fn add_key_inserts_sorted_without_rewriting_existing_records() {
        let enc = synthetic();
        let lc0 = Lcache::decode(&enc).unwrap();
        let plain0 = aes_ecb(&enc, false).unwrap();
        let count_offset = 1
            + lc0.magic_raw.len()
            + lc0.lang_count_raw.len()
            + lc0
                .languages
                .iter()
                .map(|language| language.raw.len())
                .sum::<usize>();
        let groups_end = plain0.len() - lc0.tail.len();
        let original_groups = &plain0[count_offset + 4..groups_end];

        let mut lc = Lcache::decode(&enc).unwrap();
        lc.add_key("itfo_bread", &values(&[("german", "Brot")]))
            .unwrap();
        let plain1 = aes_ecb(&lc.encode().unwrap(), false).unwrap();
        let decoded = Lcache::decode(&lc.encode().unwrap()).unwrap();
        let inserted_len =
            decoded.groups[0].main.to_bytes().len() + decoded.groups[0].meta.to_bytes().len();

        assert_eq!(&plain1[..count_offset], &plain0[..count_offset]);
        assert_eq!(
            i32::from_le_bytes(plain1[count_offset..count_offset + 4].try_into().unwrap()),
            2
        );
        assert_eq!(
            &plain1[count_offset + 4 + inserted_len
                ..count_offset + 4 + inserted_len + original_groups.len()],
            original_groups
        );
        assert_eq!(
            decoded
                .groups
                .iter()
                .map(|group| group.main.key.text.as_str())
                .collect::<Vec<_>>(),
            vec!["itfo_bread", "itfo_cheese"]
        );
        assert_eq!(plain1.len() % 16, 0);
    }

    #[test]
    fn add_key_keeps_global_order_across_multiple_insert_positions() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.add_key("zzz_probe", &values(&[("english", "last")]))
            .unwrap();
        lc.add_key("aaa_probe", &values(&[("english", "first")]))
            .unwrap();

        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(
            re.groups
                .iter()
                .map(|group| group.main.key.text.as_str())
                .collect::<Vec<_>>(),
            vec!["aaa_probe", "itfo_cheese", "zzz_probe"]
        );
    }

    #[test]
    fn add_key_rejects_unsorted_input_before_mutating() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.add_key("zzz_probe", &values(&[("english", "last")]))
            .unwrap();
        lc.groups.swap(0, 1);
        let before = lc.groups.len();

        assert!(matches!(
            lc.add_key("aaa_probe", &values(&[("english", "first")])),
            Err(LcacheError::Invalid("localization id order"))
        ));
        assert_eq!(lc.groups.len(), before);
        assert!(!lc.has_key("aaa_probe"));
    }

    #[test]
    fn add_key_uses_empty_meta_and_only_supplied_languages() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.add_key("itfo_bread", &values(&[("german", "Brot")]))
            .unwrap();

        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        let group = re
            .groups
            .iter()
            .find(|group| group.main.key.text == "itfo_bread")
            .unwrap();
        assert_eq!(group.main.key.text, "itfo_bread");
        assert_eq!(group.main.pairs.len(), 1);
        assert_eq!(group.main.pairs[0].lang.text, "german");
        assert_eq!(group.meta.key.text, "");
        assert!(group.meta.pairs.is_empty());
    }

    #[test]
    fn add_key_orders_pairs_by_header_not_map_order() {
        fn fstr(s: &str) -> Vec<u8> {
            super::encode_fstring(s)
        }
        let mut plain = Vec::new();
        plain.push(0u8);
        plain.extend_from_slice(&(MAGIC.len() as i32).to_le_bytes());
        plain.extend_from_slice(MAGIC);
        plain.extend_from_slice(&2i32.to_le_bytes());
        plain.extend_from_slice(&fstr("german"));
        plain.extend_from_slice(&fstr("english"));
        plain.extend_from_slice(&0i32.to_le_bytes());
        let pad = (16 - (plain.len() % 16)) % 16;
        plain.extend(std::iter::repeat(0u8).take(pad));

        let mut lc = Lcache::decode(&aes_ecb(&plain, true).unwrap()).unwrap();
        lc.add_key(
            "itfo_bread",
            &values(&[("english", "Bread"), ("german", "Brot")]),
        )
        .unwrap();
        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        let langs: Vec<&str> = re.groups[0]
            .main
            .pairs
            .iter()
            .map(|pair| pair.lang.text.as_str())
            .collect();
        assert_eq!(langs, vec!["german", "english"]);
    }

    #[test]
    fn add_key_validates_before_mutating() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        assert!(matches!(
            lc.add_key(
                "itfo_bread",
                &values(&[("english", "Bread"), ("klingon", "Qapla")])
            ),
            Err(LcacheError::UnknownLanguage(language)) if language == "klingon"
        ));
        assert!(!lc.has_key("itfo_bread"));
        assert_eq!(lc.key_count(), 1);
    }

    #[test]
    fn add_key_rejects_empty_duplicate_key_and_duplicate_language_aliases() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        assert!(matches!(
            lc.add_key("", &values(&[("english", "x")])),
            Err(LcacheError::EmptyKey)
        ));
        assert!(matches!(
            lc.add_key("ITFO_Cheese", &values(&[("english", "x")])),
            Err(LcacheError::DuplicateKey(_))
        ));
        assert!(matches!(
            lc.add_key(
                "itfo_bread",
                &values(&[("german", "eins"), ("GERMAN", "zwei")])
            ),
            Err(LcacheError::DuplicateLanguage(_))
        ));
        assert_eq!(lc.key_count(), 1);
    }

    #[test]
    fn add_key_canonicalizes_id_and_language_case_and_reimport_is_stable() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.add_key("ITFO_Bread", &values(&[("German", "Brot")]))
            .unwrap();
        let once = lc.encode().unwrap();
        let decoded = Lcache::decode(&once).unwrap();
        assert_eq!(decoded.export(false)["itfo_bread"]["german"], "Brot");
        assert_eq!(decoded.encode().unwrap(), once);
    }

    #[test]
    fn has_key_matches_case_insensitively() {
        let lc = Lcache::decode(&synthetic()).unwrap();
        assert!(lc.has_key("itfo_cheese"));
        assert!(lc.has_key("ITFO_CHEESE"));
        assert!(!lc.has_key("itfo_bread"));
    }

    #[test]
    fn set_or_add_value_updates_existing_and_creates_missing_key() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.set_or_add_value("itfo_cheese", "english", "Gouda")
            .unwrap();
        lc.set_or_add_value("itfo_bread", "german", "Brot").unwrap();

        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        assert_eq!(re.export(false)["itfo_cheese"]["english"], "Gouda");
        assert_eq!(re.export(false)["itfo_bread"]["german"], "Brot");
        assert_eq!(
            re.groups
                .iter()
                .find(|group| group.main.key.text == "itfo_bread")
                .unwrap()
                .main
                .pairs
                .len(),
            1
        );
    }

    #[test]
    fn set_or_add_value_inserts_missing_language_in_header_order() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        lc.add_key("itfo_bread", &values(&[("german", "Brot")]))
            .unwrap();
        lc.set_or_add_value("itfo_bread", "english", "Bread")
            .unwrap();

        let re = Lcache::decode(&lc.encode().unwrap()).unwrap();
        let group = re
            .groups
            .iter()
            .find(|group| group.main.key.text == "itfo_bread")
            .unwrap();
        let langs: Vec<&str> = group
            .main
            .pairs
            .iter()
            .map(|pair| pair.lang.text.as_str())
            .collect();
        assert_eq!(langs, vec!["english", "german"]);
    }

    #[test]
    fn set_or_add_value_rejects_unknown_language_without_adding_key() {
        let mut lc = Lcache::decode(&synthetic()).unwrap();
        assert!(matches!(
            lc.set_or_add_value("itfo_bread", "klingon", "Qapla"),
            Err(LcacheError::UnknownLanguage(_))
        ));
        assert!(!lc.has_key("itfo_bread"));
    }

    /// Optional read-only smoke test against the shipped game cache. The live
    /// file is read into memory and never modified.
    #[test]
    fn real_lcache_add_key_survives_round_trip() {
        let Ok(path) = std::env::var("GORE_LOC_REAL_LCACHE") else {
            eprintln!("GORE_LOC_REAL_LCACHE not set; skipping real-file test");
            return;
        };
        let encrypted = std::fs::read(path).unwrap();
        let original = Lcache::decode(&encrypted).unwrap();
        assert_eq!(original.encode().unwrap(), encrypted);
        assert!(original
            .groups
            .windows(2)
            .all(|pair| pair[0].main.key.text.as_str() <= pair[1].main.key.text.as_str()));

        // Keep a compact diagnostic for reverse-engineering optional metadata.
        // It is emitted only when the real-file environment gate is enabled.
        let mut meta_keys: BTreeMap<String, usize> = BTreeMap::new();
        let mut meta_pair_names: BTreeMap<String, usize> = BTreeMap::new();
        let mut meta_pair_examples: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut meta_examples = Vec::new();
        for group in &original.groups {
            if !group.meta.key.text.is_empty() {
                *meta_keys.entry(group.meta.key.text.clone()).or_default() += 1;
            }
            for pair in &group.meta.pairs {
                *meta_pair_names.entry(pair.lang.text.clone()).or_default() += 1;
                let examples = meta_pair_examples
                    .entry(pair.lang.text.clone())
                    .or_default();
                if examples.len() < 12 {
                    examples.push((group.main.key.text.clone(), pair.value.text.clone()));
                }
            }
            if meta_examples.len() < 20
                && (!group.meta.key.text.is_empty() || !group.meta.pairs.is_empty())
            {
                meta_examples.push((
                    group.main.key.text.clone(),
                    group.meta.key.text.clone(),
                    group
                        .meta
                        .pairs
                        .iter()
                        .map(|pair| (pair.lang.text.clone(), pair.value.text.clone()))
                        .collect::<Vec<_>>(),
                ));
            }
        }
        eprintln!(
            "real lcache metadata: groups={}, meta_keys={meta_keys:?}, \
             pair_names={meta_pair_names:?}, pair_examples={meta_pair_examples:?}, \
             examples={meta_examples:?}",
            original.groups.len()
        );

        let key = "goremod_test_added_key";
        assert!(
            !original.has_key(key),
            "test key unexpectedly exists in game cache"
        );
        let before = original.export(true);
        let mut edited = Lcache::decode(&encrypted).unwrap();
        edited
            .add_key(
                key,
                &values(&[
                    ("english", "Added by gore-loc"),
                    ("german", "Von gore-loc hinzugefügt"),
                ]),
            )
            .unwrap();
        let encoded = edited.encode().unwrap();
        let decoded = Lcache::decode(&encoded).unwrap();
        assert!(decoded
            .groups
            .windows(2)
            .all(|pair| pair[0].main.key.text.as_str() <= pair[1].main.key.text.as_str()));

        let mut after = decoded.export(true);
        assert_eq!(after[key]["english"], "Added by gore-loc");
        assert_eq!(after[key]["german"], "Von gore-loc hinzugefügt");
        after.remove(key);
        assert_eq!(after, before, "pre-existing localization semantics changed");
    }
}
