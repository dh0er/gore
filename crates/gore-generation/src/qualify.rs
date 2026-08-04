//! Deriving a row instead of transcribing one.
//!
//! A generation lands as twenty-four values, and until now every one of them was produced by a
//! person reading a number out of a test run and pasting it into a struct literal. That is where
//! the two failure modes live: a value measured against the wrong file, and a value measured by a
//! reimplementation of the thing that is supposed to produce it. This module is the machine-side
//! half of the fix — the pure derivations and the bookkeeping a qualification run needs — while
//! `gore as qualify` supplies the bytes and the real parsers.
//!
//! Nothing here seals anything. A [`DraftRow`] is a proposal; it becomes a generation only when a
//! person puts it in `lib.rs` next to a committed artifact. The separation is deliberate and is the
//! same one the crate's own header describes: the table cannot tell a measured digest from a pasted
//! one, so the value of an automated derivation is that a *person* can compare it against a
//! *previous* one, not that the machine gets to skip the person.

use sha2::{Digest, Sha256};

use crate::{
    derived_profile_sha256, map_proof_sha256, rows, FileSeal, GenerationRow, ProfileComponents,
    CACHE_FINGERPRINT_FORMAT,
};

/// The canonical digest of a parser-output row table: sort, then hash every column as a
/// little-endian `u32` length followed by its bytes.
///
/// Three of a row's five parser-output digests are this function applied to a different table —
/// the USMAP class graph, the resolved Binds/USMAP class profile, and the GameplayTag-map field
/// profile. It is written here rather than beside any one of them because a qualification run and
/// the runtime admission gate have to agree on it exactly: a digest is only evidence if the thing
/// that produced it and the thing that checks it are the same function.
pub fn canonical_rows_sha256<const N: usize>(table: &mut Vec<[String; N]>) -> [u8; 32] {
    table.sort_unstable();
    let mut hash = Sha256::new();
    for row in table.iter() {
        for value in row {
            hash.update((value.len() as u32).to_le_bytes());
            hash.update(value.as_bytes());
        }
    }
    hash.finalize().into()
}

/// Spell a digest the way every published ID, selector and receipt spells it.
pub fn publish_id(digest: &[u8; 32]) -> String {
    format!("sha256:{}", hex_lower(digest))
}

pub fn hex_lower(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble"));
        text.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble"));
    }
    text
}

/// The two IDs a build publishes, derived from measured components rather than read from a row.
///
/// This is the same pair of calls the admission gate makes, in the same order, against the same
/// two functions. A qualification run that computed them any other way would produce a row that is
/// self-consistent and describes nothing.
pub fn derive_published_ids(
    components: &ProfileComponents,
    gameplay_tag_float32_map_profile_sha256: &[u8; 32],
) -> (String, String) {
    let profile = derived_profile_sha256(components);
    let proof = map_proof_sha256(&profile, gameplay_tag_float32_map_profile_sha256);
    (publish_id(&profile), publish_id(&proof))
}

/// Assemble the nine identity components from values measured off an installation.
///
/// `fingerprint_format` is not a parameter: a run that fingerprinted a cache under a different
/// format string is not measuring the same thing the table publishes, and taking it from the
/// caller would let the two drift apart silently.
#[allow(clippy::too_many_arguments)]
pub fn observed_profile_components(
    script_cache_guid: [u8; 16],
    script_cache_mutation_stable_sha256: [u8; 32],
    scalar_default_operand_count: usize,
    gameplay_tag_float32_operand_count: usize,
    binds_source_sha256: [u8; 32],
    binds_bridge_sha256: [u8; 32],
    usmap_source_sha256: [u8; 32],
    usmap_graph_sha256: [u8; 32],
    resolved_profile_sha256: [u8; 32],
) -> ProfileComponents {
    ProfileComponents {
        fingerprint_format: CACHE_FINGERPRINT_FORMAT,
        script_cache_guid,
        script_cache_mutation_stable_sha256,
        scalar_default_operand_count,
        gameplay_tag_float32_operand_count,
        binds_source_sha256,
        binds_bridge_sha256,
        usmap_source_sha256,
        usmap_graph_sha256,
        resolved_profile_sha256,
    }
}

/// One number a qualification run measured, next to the same number on the row it is succeeding.
///
/// The digests cannot answer this question at all: a parser that silently dropped half its rows
/// produces a perfectly valid digest over what is left. Only the count says so, and only against
/// the previous generation's count. `docs/reference/game-updates.md` step 7 is the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountComparison {
    pub name: &'static str,
    /// Absent when the previous generation's artifact did not record this number.
    pub previous: Option<u64>,
    pub observed: u64,
    /// The test or function that produced `observed`.
    pub witness: String,
}

impl CountComparison {
    /// Whether the count fell against the generation before it. A count that moved by a handful is
    /// normal; a count that fell means rows stopped being parsed, and no digest will say so.
    pub fn fell(&self) -> bool {
        self.previous.is_some_and(|previous| self.observed < previous)
    }

    pub fn delta(&self) -> Option<i64> {
        self.previous
            .map(|previous| self.observed as i64 - previous as i64)
    }
}

/// The audited row a candidate build should be compared against: the newest row that shares the
/// most raw file identity with it, and the last row otherwise.
///
/// Sharing a `Binds.Cache` or a USMAP with an audited build is the strongest statement a candidate
/// can make about which generation it succeeds, and a build that reuses one of those inherits the
/// parser-output digests that go with it — which is exactly the comparison a reviewer wants in
/// front of them. Table order is generation order, so the last match is the nearest one.
pub fn nearest_row(
    binds_cache_sha256: Option<&[u8; 32]>,
    usmap_sha256: Option<&[u8; 32]>,
) -> Option<&'static GenerationRow> {
    let shared = rows().iter().rev().find(|row| {
        binds_cache_sha256.is_some_and(|sha| &row.binds_cache.sha256 == sha)
            || usmap_sha256.is_some_and(|sha| &row.usmap.sha256 == sha)
    });
    shared.or_else(|| rows().last())
}

/// A proposed generation row: every field a [`GenerationRow`] has, and `None` for every one this
/// run could not measure.
///
/// It renders as the Rust literal a person pastes into `lib.rs`, with an unmistakable placeholder
/// wherever a value is missing, because a draft that silently rendered a zero would be a row that
/// compiles and lies. Emitting one is not admission: see [`DraftRow::still_to_do`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftRow {
    pub id: String,
    pub label: String,
    pub edition: String,
    pub executable: Option<FileSeal>,
    pub shipping_cache: Option<FileSeal>,
    pub binds_cache: Option<FileSeal>,
    pub usmap: Option<FileSeal>,
    pub script_cache_guid: Option<[u8; 16]>,
    pub script_cache_mutation_stable_sha256: Option<[u8; 32]>,
    pub scalar_default_operand_count: Option<usize>,
    pub gameplay_tag_float32_operand_count: Option<usize>,
    pub binds_field_map_sha256: Option<[u8; 32]>,
    pub binds_class_path_map_sha256: Option<[u8; 32]>,
    pub usmap_class_graph_sha256: Option<[u8; 32]>,
    pub resolved_class_profile_sha256: Option<[u8; 32]>,
    pub gameplay_tag_float32_map_profile_sha256: Option<[u8; 32]>,
    pub native_ancestry_profile_id: Option<String>,
    pub gameplay_tag_float32_map_proof_id: Option<String>,
    pub record_set_id: String,
    pub record_set_seal: Option<FileSeal>,
    pub catalog_payload_seal: Option<FileSeal>,
    pub catalog_label: String,
    pub record_seal_kind: String,
    pub catalog_seal_kind: String,
    pub audited_item_generation: String,
}

/// The placeholder a missing value renders as. Chosen so that pasting an incomplete draft into
/// `lib.rs` does not compile.
pub const UNDERIVED: &str = "<<< not derived by this run >>>";

impl DraftRow {
    /// Take every value an audited row already carries. Used when a run re-qualifies a build the
    /// table already knows, so that the draft it prints can be diffed against the sealed row
    /// field by field rather than read as a new proposal.
    pub fn from_row(row: &GenerationRow) -> Self {
        Self {
            id: row.id.to_owned(),
            label: row.label.to_owned(),
            edition: row.edition.to_owned(),
            executable: Some(row.executable),
            shipping_cache: Some(row.shipping_cache),
            binds_cache: Some(row.binds_cache),
            usmap: Some(row.usmap),
            script_cache_guid: Some(row.script_cache_guid),
            script_cache_mutation_stable_sha256: Some(row.script_cache_mutation_stable_sha256),
            scalar_default_operand_count: Some(row.scalar_default_operand_count),
            gameplay_tag_float32_operand_count: Some(row.gameplay_tag_float32_operand_count),
            binds_field_map_sha256: Some(row.binds_field_map_sha256),
            binds_class_path_map_sha256: Some(row.binds_class_path_map_sha256),
            usmap_class_graph_sha256: Some(row.usmap_class_graph_sha256),
            resolved_class_profile_sha256: Some(row.resolved_class_profile_sha256),
            gameplay_tag_float32_map_profile_sha256: Some(
                row.gameplay_tag_float32_map_profile_sha256,
            ),
            native_ancestry_profile_id: Some(row.native_ancestry_profile_id.to_owned()),
            gameplay_tag_float32_map_proof_id: Some(
                row.gameplay_tag_float32_map_proof_id.to_owned(),
            ),
            record_set_id: row.record_set_id.to_owned(),
            record_set_seal: Some(row.record_set_seal),
            catalog_payload_seal: Some(row.catalog_payload_seal),
            catalog_label: row.catalog_label.to_owned(),
            record_seal_kind: row.record_seal_kind.to_owned(),
            catalog_seal_kind: row.catalog_seal_kind.to_owned(),
            audited_item_generation: row.audited_item_generation.to_owned(),
        }
    }

    /// Every field this run could not measure, in declaration order.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        let mut want = |present: bool, name: &'static str| {
            if !present {
                missing.push(name);
            }
        };
        want(self.executable.is_some(), "executable");
        want(self.shipping_cache.is_some(), "shipping_cache");
        want(self.binds_cache.is_some(), "binds_cache");
        want(self.usmap.is_some(), "usmap");
        want(self.script_cache_guid.is_some(), "script_cache_guid");
        want(
            self.script_cache_mutation_stable_sha256.is_some(),
            "script_cache_mutation_stable_sha256",
        );
        want(
            self.scalar_default_operand_count.is_some(),
            "scalar_default_operand_count",
        );
        want(
            self.gameplay_tag_float32_operand_count.is_some(),
            "gameplay_tag_float32_operand_count",
        );
        want(
            self.binds_field_map_sha256.is_some(),
            "binds_field_map_sha256",
        );
        want(
            self.binds_class_path_map_sha256.is_some(),
            "binds_class_path_map_sha256",
        );
        want(
            self.usmap_class_graph_sha256.is_some(),
            "usmap_class_graph_sha256",
        );
        want(
            self.resolved_class_profile_sha256.is_some(),
            "resolved_class_profile_sha256",
        );
        want(
            self.gameplay_tag_float32_map_profile_sha256.is_some(),
            "gameplay_tag_float32_map_profile_sha256",
        );
        want(
            self.native_ancestry_profile_id.is_some(),
            "native_ancestry_profile_id",
        );
        want(
            self.gameplay_tag_float32_map_proof_id.is_some(),
            "gameplay_tag_float32_map_proof_id",
        );
        want(self.record_set_seal.is_some(), "record_set_seal");
        want(self.catalog_payload_seal.is_some(), "catalog_payload_seal");
        missing
    }

    /// What a person still has to do after reading this draft, in the order they have to do it.
    /// A qualification run never edits `lib.rs`, so this list is the whole remaining procedure.
    pub fn still_to_do(&self) -> Vec<String> {
        let mut steps = Vec::new();
        let missing = self.missing();
        if !missing.is_empty() {
            steps.push(format!(
                "derive the {} value(s) this run could not: {}",
                missing.len(),
                missing.join(", ")
            ));
        }
        steps.push(format!(
            "read the counts above against the previous generation before accepting any of them \
             (docs/reference/game-updates.md step 7), then add the row literal to \
             crates/gore-generation/src/lib.rs, extend GENERATION_ROWS and QUALIFICATION_ARTIFACTS \
             to {} entries",
            rows().len() + 1
        ));
        steps.push(format!(
            "commit crates/gore-generation/qualifications/{}.json, then run the full suite: \
             emitting a row is not the row being in the binary",
            self.id
        ));
        steps
    }

    /// The Rust literal a person pastes into `lib.rs`.
    pub fn to_rust_literal(&self) -> String {
        let seal = |seal: &Option<FileSeal>| match seal {
            Some(seal) => format!(
                "FileSeal {{ byte_len: {}, sha256: hex(\"{}\") }}",
                seal.byte_len,
                hex_lower(&seal.sha256)
            ),
            None => format!("FileSeal {{ byte_len: 0, sha256: hex(\"{UNDERIVED}\") }}"),
        };
        let digest = |digest: &Option<[u8; 32]>| match digest {
            Some(bytes) => format!("hex(\"{}\")", hex_lower(bytes)),
            None => format!("hex(\"{UNDERIVED}\")"),
        };
        let text = |value: &Option<String>| match value {
            Some(value) => format!("\"{value}\""),
            None => format!("\"{UNDERIVED}\""),
        };
        let count = |value: &Option<usize>| match value {
            Some(value) => value.to_string(),
            None => format!("/* {UNDERIVED} */"),
        };
        let mut out = String::new();
        out.push_str(&format!(
            "pub const ROW_{}: GenerationRow = GenerationRow {{\n",
            self.id
                .to_uppercase()
                .replace(['-', '.'], "_")
                .replace("G1R_STEAM_", "G1R_")
        ));
        out.push_str(&format!("    id: \"{}\",\n", self.id));
        out.push_str(&format!("    label: \"{}\",\n", self.label));
        out.push_str(&format!("    edition: \"{}\",\n\n", self.edition));
        out.push_str(&format!("    executable: {},\n", seal(&self.executable)));
        out.push_str(&format!(
            "    shipping_cache: {},\n",
            seal(&self.shipping_cache)
        ));
        out.push_str(&format!("    binds_cache: {},\n", seal(&self.binds_cache)));
        out.push_str(&format!("    usmap: {},\n\n", seal(&self.usmap)));
        out.push_str(&format!(
            "    script_cache_guid: {},\n",
            match &self.script_cache_guid {
                Some(guid) => format!("hex(\"{}\")", hex_lower(guid)),
                None => format!("hex(\"{UNDERIVED}\")"),
            }
        ));
        out.push_str(&format!(
            "    script_cache_mutation_stable_sha256: {},\n",
            digest(&self.script_cache_mutation_stable_sha256)
        ));
        out.push_str(&format!(
            "    scalar_default_operand_count: {},\n",
            count(&self.scalar_default_operand_count)
        ));
        out.push_str(&format!(
            "    gameplay_tag_float32_operand_count: {},\n\n",
            count(&self.gameplay_tag_float32_operand_count)
        ));
        for (name, value) in [
            ("binds_field_map_sha256", &self.binds_field_map_sha256),
            (
                "binds_class_path_map_sha256",
                &self.binds_class_path_map_sha256,
            ),
            ("usmap_class_graph_sha256", &self.usmap_class_graph_sha256),
            (
                "resolved_class_profile_sha256",
                &self.resolved_class_profile_sha256,
            ),
            (
                "gameplay_tag_float32_map_profile_sha256",
                &self.gameplay_tag_float32_map_profile_sha256,
            ),
        ] {
            out.push_str(&format!("    {name}: {},\n", digest(value)));
        }
        out.push_str(&format!(
            "\n    native_ancestry_profile_id: {},\n",
            text(&self.native_ancestry_profile_id)
        ));
        out.push_str(&format!(
            "    gameplay_tag_float32_map_proof_id: {},\n\n",
            text(&self.gameplay_tag_float32_map_proof_id)
        ));
        out.push_str(&format!(
            "    record_set_id: \"{}\",\n",
            self.record_set_id
        ));
        out.push_str(&format!(
            "    record_set_seal: {},\n",
            seal(&self.record_set_seal)
        ));
        out.push_str(&format!(
            "    catalog_payload_seal: {},\n",
            seal(&self.catalog_payload_seal)
        ));
        out.push_str(&format!(
            "    catalog_label: \"{}\",\n",
            self.catalog_label
        ));
        out.push_str(&format!(
            "    record_seal_kind: \"{}\",\n",
            self.record_seal_kind
        ));
        out.push_str(&format!(
            "    catalog_seal_kind: \"{}\",\n\n",
            self.catalog_seal_kind
        ));
        out.push_str(&format!(
            "    audited_item_generation: \"{}\",\n}};\n",
            self.audited_item_generation
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expected_id_sha256;

    #[test]
    fn a_qualification_run_and_the_admission_gate_publish_one_id() {
        // The reason `derive_published_ids` exists at all rather than the command hashing the nine
        // components itself: a qualification that spelled an ID even slightly differently would
        // mint a row nothing on the runtime path could ever match, and the failure would show up
        // as a build the table admits and the tool refuses. Every sealed row is a test vector for
        // it, so this cannot pass while the two disagree.
        for row in rows() {
            let (profile_id, proof_id) = derive_published_ids(
                &row.profile_components(),
                &row.gameplay_tag_float32_map_profile_sha256,
            );
            assert_eq!(
                profile_id, row.native_ancestry_profile_id,
                "{}: qualification derives a different ancestry profile ID than the row publishes",
                row.id
            );
            assert_eq!(
                proof_id, row.gameplay_tag_float32_map_proof_id,
                "{}: qualification derives a different tag-map proof ID than the row publishes",
                row.id
            );
        }
    }

    #[test]
    fn a_row_table_digest_is_independent_of_the_order_rows_were_found_in() {
        // The USMAP walk visits classes in schema-id order and the Binds bridge in hash order, so
        // two runs over the same file can produce the same rows in different sequences. Sorting is
        // what makes the digest a function of the content; a version of this that hashed in
        // discovery order would seal the iteration order of a `HashMap`.
        let mut forward = vec![
            ["/Script/G1R.A".to_owned(), "/Script/CoreUObject.Object".to_owned()],
            ["/Script/G1R.B".to_owned(), "/Script/G1R.A".to_owned()],
        ];
        let mut reversed = vec![forward[1].clone(), forward[0].clone()];
        assert_eq!(
            canonical_rows_sha256(&mut forward),
            canonical_rows_sha256(&mut reversed)
        );
    }

    #[test]
    fn a_column_boundary_cannot_be_moved_without_changing_the_digest() {
        // Length prefixes are the whole reason this is not a plain concatenation: without them
        // `("AB", "C")` and `("A", "BC")` seal identically, and a class named for part of its
        // parent would be indistinguishable from its parent named for part of it.
        let mut split_left = vec![["AB".to_owned(), "C".to_owned()]];
        let mut split_right = vec![["A".to_owned(), "BC".to_owned()]];
        assert_ne!(
            canonical_rows_sha256(&mut split_left),
            canonical_rows_sha256(&mut split_right)
        );
    }

    #[test]
    fn a_count_that_fell_is_reported_and_a_count_that_rose_is_not() {
        // Step 7 of the update checklist in one assertion. A parser that silently stopped
        // recognising a record shape produces a digest that is perfectly valid over what survived,
        // so the only signal is the count, and the only useful comparison is against the row this
        // build succeeds.
        let fell = CountComparison {
            name: "bridged classes",
            previous: Some(6582),
            observed: 6570,
            witness: "test".to_owned(),
        };
        assert!(fell.fell());
        assert_eq!(fell.delta(), Some(-12));
        let rose = CountComparison {
            name: "bridged classes",
            previous: Some(6572),
            observed: 6582,
            witness: "test".to_owned(),
        };
        assert!(!rose.fell());
        assert_eq!(rose.delta(), Some(10));
        let unknown = CountComparison {
            name: "bridged classes",
            previous: None,
            observed: 0,
            witness: "test".to_owned(),
        };
        assert!(!unknown.fell(), "an absent baseline is not a regression");
        assert_eq!(unknown.delta(), None);
    }

    #[test]
    fn a_build_that_reuses_a_sealed_file_is_compared_against_the_row_that_sealed_it() {
        // The 24169431 build reused its predecessor's Binds.Cache and USMAP byte for byte, so this
        // is not a hypothetical: a candidate sharing one of those inherits that row's parser-output
        // digests, and comparing it against anything else would report a change that did not
        // happen.
        let first = &rows()[0];
        let newest_sharing = rows()
            .iter()
            .rev()
            .find(|row| row.binds_cache.sha256 == first.binds_cache.sha256)
            .expect("the row itself shares its own file");
        assert_eq!(
            nearest_row(Some(&first.binds_cache.sha256), None).map(|row| row.id),
            Some(newest_sharing.id),
            "the newest row sharing the file wins, because table order is generation order"
        );
        assert_ne!(
            newest_sharing.id, first.id,
            "the fixture only means anything while some later row reuses the first one's file"
        );
        assert_eq!(
            nearest_row(Some(&[0xab; 32]), Some(&[0xcd; 32])).map(|row| row.id),
            Some(rows().last().expect("a table with rows").id),
            "a build sharing nothing is still compared against the generation it succeeds"
        );
    }

    #[test]
    fn a_draft_missing_a_value_cannot_be_pasted_into_the_table() {
        // The failure this rules out is the worst one available here: a draft that rendered an
        // unmeasured digest as zeroes would compile, pass `row_fields_are_canonically_shaped`
        // only by luck, and seal a build against a value nobody derived.
        let draft = DraftRow {
            id: "g1r-steam-99999999".to_owned(),
            ..DraftRow::default()
        };
        let literal = draft.to_rust_literal();
        assert!(literal.contains(UNDERIVED));
        assert!(
            !literal.contains("0000000000000000"),
            "a value nobody measured must not render as a zero digest"
        );
        assert_eq!(draft.missing().len(), 17);
        assert!(draft.still_to_do()[0].contains("could not"));

        let complete = DraftRow::from_row(&rows()[0]);
        assert!(complete.missing().is_empty());
        assert!(!complete.to_rust_literal().contains(UNDERIVED));
        assert!(
            complete
                .to_rust_literal()
                .contains(rows()[0].native_ancestry_profile_id),
            "a re-qualified row renders the identity it already publishes, so it can be diffed"
        );
    }

    #[test]
    fn a_published_id_is_the_spelling_the_table_already_uses() {
        for row in rows() {
            let digest = expected_id_sha256(row.native_ancestry_profile_id).expect("published id");
            assert_eq!(publish_id(&digest), row.native_ancestry_profile_id);
        }
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }
}
