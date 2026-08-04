//! What replaces the compiler.
//!
//! Before the table, a generation's identity was seven `const`s across six files, and three
//! separate language rules did real work: a `[u8; 32]` compared with `!=` made "there is exactly
//! one audited USMAP" a type-level fact, a two-variant enum made a missed field a compile error in
//! seven match arms, and a fixed-length array made the count show up in the diff. Fields of a
//! struct literal have none of that — twenty-four values of the right types compile, whatever they
//! say. These tests are what stands in for it, and they run in the default suite because a
//! qualification gate nobody runs is not a gate.
//!
//! What none of them restore: nothing here forces a row's digests to have come from a real run
//! against real game bytes. `every_row_derives_its_own_published_ids` proves a row is *consistent*;
//! only the artifact `every_row_has_a_committed_qualification_artifact` demands, plus a person
//! reading it, proves a row is *true*.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gore_generation::{
    binds_digests_for_sha256, derived_profile_sha256, expected_id_sha256,
    is_supported_tag_proof_pair, map_proof_sha256, row_by_id, row_by_profile_id, row_for_executable,
    row_for_file_seals, row_for_script_cache, row_for_script_cache_guid, rows,
    rows_for_binds_sha256, CacheFingerprint, GenerationRow, QUALIFICATION_ARTIFACTS,
};

fn fingerprint_of(row: &GenerationRow) -> CacheFingerprint {
    CacheFingerprint {
        sha256: row.script_cache_mutation_stable_sha256,
        scalar_operand_count: row.scalar_default_operand_count,
        tag_operand_count: row.gameplay_tag_float32_operand_count,
    }
}

fn qualifications_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("qualifications")
}

#[test]
fn every_row_derives_its_own_published_ids() {
    // This is the one that makes a pasted digest impossible to hide. The profile ID is a hash of
    // all ten sealed components, and it is what appears in every `DefaultSiteSelector` and every
    // receipt — so getting any single component wrong and copying a neighbouring row's ID cannot
    // both be true. It calls the same two functions the runtime calls when it admits a cache, so
    // the table cannot drift away from the check that reads it.
    for row in rows() {
        let profile = derived_profile_sha256(&row.profile_components());
        assert_eq!(
            expected_id_sha256(row.native_ancestry_profile_id),
            Some(profile),
            "{}: the published native-ancestry profile ID is not the digest of this row's own \
             components, so at least one of them is wrong or the ID was copied",
            row.id
        );
        assert_eq!(
            expected_id_sha256(row.gameplay_tag_float32_map_proof_id),
            Some(map_proof_sha256(
                &profile,
                &row.gameplay_tag_float32_map_profile_sha256
            )),
            "{}: the published tag-map proof ID is not this row's profile joined with its own \
             map-field profile",
            row.id
        );
    }
}

#[test]
fn rows_are_pairwise_distinct() {
    // The failure this exists for: copy the row above, edit the executable and the script cache,
    // and ship a generation that still carries its predecessor's profile ID. The old two-variant
    // enum caught that by refusing to compile until all seven arms were filled. `binds_cache` and
    // `usmap` are deliberately absent — Steam has already shipped a build that reused both byte
    // for byte, and rejecting that would be rejecting the truth.
    let rows = rows();
    for (index, left) in rows.iter().enumerate() {
        for right in &rows[index + 1..] {
            assert_ne!(left.id, right.id, "two rows share an id");
            assert_ne!(
                left.script_cache_guid, right.script_cache_guid,
                "{} and {} share a script-cache GUID",
                left.id, right.id
            );
            assert_ne!(
                left.script_cache_mutation_stable_sha256, right.script_cache_mutation_stable_sha256,
                "{} and {} share a combined cache fingerprint",
                left.id, right.id
            );
            assert_ne!(
                left.native_ancestry_profile_id, right.native_ancestry_profile_id,
                "{} and {} publish one ancestry profile ID",
                left.id, right.id
            );
            assert_ne!(
                left.gameplay_tag_float32_map_proof_id, right.gameplay_tag_float32_map_proof_id,
                "{} and {} publish one tag-map proof ID",
                left.id, right.id
            );
            assert_ne!(
                left.record_set_id, right.record_set_id,
                "{} and {} share a curated record-set id",
                left.id, right.id
            );
            assert_ne!(
                left.executable.sha256, right.executable.sha256,
                "{} and {} share an executable, so `row_for_executable` would be ambiguous",
                left.id, right.id
            );
            assert_ne!(
                left.shipping_cache.sha256, right.shipping_cache.sha256,
                "{} and {} share a Shipping cache",
                left.id, right.id
            );
        }
    }
}

#[test]
fn binds_digests_are_consistent_per_file() {
    // When one Binds file was sealed by one constant this was true by construction. It is now an
    // assumption `binds_digests_for_sha256` depends on: two rows naming the same bytes but
    // different parser output would make the lookup pick a winner, and the loser's sealed field
    // map would silently stop being mutation evidence.
    for row in rows() {
        let sharing: Vec<_> = rows_for_binds_sha256(&row.binds_cache.sha256).collect();
        assert!(
            sharing.iter().any(|found| found.id == row.id),
            "{} is not found by its own Binds digest",
            row.id
        );
        for found in &sharing {
            assert_eq!(
                (found.binds_field_map_sha256, found.binds_class_path_map_sha256),
                (row.binds_field_map_sha256, row.binds_class_path_map_sha256),
                "{} and {} name the same Binds.Cache bytes but seal different parser output",
                row.id,
                found.id
            );
            assert_eq!(
                found.binds_cache.byte_len, row.binds_cache.byte_len,
                "{} and {} name the same Binds digest with different lengths",
                row.id, found.id
            );
        }
        assert_eq!(
            binds_digests_for_sha256(&row.binds_cache.sha256),
            Some((row.binds_field_map_sha256, row.binds_class_path_map_sha256)),
            "{}: the Binds digests are not reachable from the file that carries them",
            row.id
        );
    }
    assert_eq!(binds_digests_for_sha256(&[0; 32]), None);
}

#[test]
fn row_fields_are_canonically_shaped() {
    // Cheap shape rules, one per way a hand-written row has actually been got wrong elsewhere in
    // this repo: a trailing space in a string that is compared, a zero-filled digest left behind
    // by a placeholder, an ID that does not carry the `sha256:` prefix everything downstream
    // parses.
    for row in rows() {
        for (name, value) in [
            ("id", row.id),
            ("label", row.label),
            ("edition", row.edition),
            ("record_set_id", row.record_set_id),
            ("catalog_label", row.catalog_label),
            ("record_seal_kind", row.record_seal_kind),
            ("catalog_seal_kind", row.catalog_seal_kind),
            ("audited_item_generation", row.audited_item_generation),
        ] {
            assert!(!value.is_empty(), "{}: {name} is empty", row.id);
            assert_eq!(value, value.trim(), "{}: {name} is not trimmed", row.id);
            assert!(value.is_ascii(), "{}: {name} is not ASCII", row.id);
        }
        assert_eq!(
            row.edition, "g1r-steam",
            "{}: only the Steam edition has ever been audited; a new channel is not a row edit",
            row.id
        );
        for (name, id) in [
            ("native_ancestry_profile_id", row.native_ancestry_profile_id),
            (
                "gameplay_tag_float32_map_proof_id",
                row.gameplay_tag_float32_map_proof_id,
            ),
        ] {
            assert!(
                expected_id_sha256(id).is_some(),
                "{}: {name} is not `sha256:` plus 64 lowercase hex characters",
                row.id
            );
        }
        for (name, seal) in [
            ("executable", row.executable),
            ("shipping_cache", row.shipping_cache),
            ("binds_cache", row.binds_cache),
            ("usmap", row.usmap),
            ("record_set_seal", row.record_set_seal),
            ("catalog_payload_seal", row.catalog_payload_seal),
        ] {
            assert!(seal.byte_len > 0, "{}: {name} seals an empty file", row.id);
            assert_ne!(seal.sha256, [0; 32], "{}: {name} has a zero digest", row.id);
        }
        for (name, digest) in [
            ("binds_field_map_sha256", row.binds_field_map_sha256),
            (
                "binds_class_path_map_sha256",
                row.binds_class_path_map_sha256,
            ),
            ("usmap_class_graph_sha256", row.usmap_class_graph_sha256),
            (
                "resolved_class_profile_sha256",
                row.resolved_class_profile_sha256,
            ),
            (
                "gameplay_tag_float32_map_profile_sha256",
                row.gameplay_tag_float32_map_profile_sha256,
            ),
            ("script_cache_mutation_stable_sha256", row.script_cache_mutation_stable_sha256),
        ] {
            assert_ne!(digest, [0; 32], "{}: {name} has a zero digest", row.id);
        }
        assert_ne!(
            row.script_cache_guid, [0; 16],
            "{}: the script-cache GUID is zero",
            row.id
        );
        assert!(
            row.scalar_default_operand_count > 0 && row.gameplay_tag_float32_operand_count > 0,
            "{}: an operand count of zero would admit any cache the fingerprint happened to match",
            row.id
        );
    }
}

#[test]
fn every_row_is_reachable_through_every_gate() {
    // Each consumer admits a build through a different key, and a row that answers one of them but
    // not the others is the shape of a half-finished addition: `gore as` recognises the install
    // and `gore story-catalog` refuses it, or the other way round. The cross terms generalize the
    // old hardcoded V1/V2 non-crossing check to every pair, which is what makes a third row safe.
    for row in rows() {
        assert_eq!(row_by_id(row.id).map(|found| found.id), Some(row.id));
        assert_eq!(
            row_for_script_cache(&row.script_cache_guid, &fingerprint_of(row)).map(|found| found.id),
            Some(row.id)
        );
        assert_eq!(
            row_for_script_cache_guid(&row.script_cache_guid).map(|found| found.id),
            Some(row.id)
        );
        assert_eq!(
            row_for_file_seals(&row.executable, &row.shipping_cache, &row.binds_cache)
                .map(|found| found.id),
            Some(row.id)
        );
        assert_eq!(
            row_for_executable(&row.executable).map(|found| found.id),
            Some(row.id)
        );
        assert_eq!(
            row_by_profile_id(row.native_ancestry_profile_id).map(|found| found.id),
            Some(row.id)
        );
        assert!(is_supported_tag_proof_pair(
            row.native_ancestry_profile_id,
            row.gameplay_tag_float32_map_proof_id
        ));
    }

    for left in rows() {
        for right in rows() {
            if left.id == right.id {
                continue;
            }
            assert!(
                !is_supported_tag_proof_pair(
                    left.native_ancestry_profile_id,
                    right.gameplay_tag_float32_map_proof_id
                ),
                "a {} ancestry profile paired with a {} map proof must not be a supported pair",
                left.id,
                right.id
            );
            assert!(
                row_for_script_cache(&left.script_cache_guid, &fingerprint_of(right)).is_none(),
                "the {} cache GUID must not admit the {} fingerprint",
                left.id,
                right.id
            );
        }
    }

    assert!(row_by_id("g1r-steam-not-a-build").is_none());
    assert!(!is_supported_tag_proof_pair("sha256:unknown", "sha256:unknown"));
    let mut unknown_guid = rows()[0].script_cache_guid;
    unknown_guid[0] ^= 1;
    assert!(row_for_script_cache_guid(&unknown_guid).is_none());
    let mut hybrid = rows()[0].executable;
    hybrid.byte_len += 1;
    assert!(row_for_executable(&hybrid).is_none());
}

#[test]
fn every_row_has_a_committed_qualification_artifact() {
    // The governance half, and the reason the refactor is not a net loss. A row is a claim about a
    // real game build; this is the file where the run that produced the claim is written down, and
    // requiring it is what a reviewer reads instead of re-deriving twenty-four values. The
    // directory is compared against the list as well, so an artifact that was written but never
    // registered — or a row added without one — fails here rather than being noticed later.
    let mut registered = BTreeMap::new();
    for (id, text) in QUALIFICATION_ARTIFACTS {
        assert!(
            registered.insert(id, text).is_none(),
            "{id} has two qualification artifacts"
        );
    }
    assert_eq!(
        registered.len(),
        rows().len(),
        "there must be exactly one qualification artifact per row"
    );

    let mut on_disk: Vec<_> = std::fs::read_dir(qualifications_dir())
        .expect("read the qualifications directory")
        .map(|entry| {
            entry
                .expect("read a qualification entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    on_disk.sort();
    let mut expected: Vec<_> = registered.keys().map(|id| format!("{id}.json")).collect();
    expected.sort();
    assert_eq!(
        on_disk, expected,
        "qualifications/ and the embedded artifact list disagree"
    );

    for row in rows() {
        let text = registered
            .get(row.id)
            .copied()
            .unwrap_or_else(|| panic!("{} has no committed qualification artifact", row.id));
        let artifact: serde_json::Value =
            serde_json::from_str(text).expect("a qualification artifact is JSON");
        let string = |key: &str| {
            artifact[key]
                .as_str()
                .unwrap_or_else(|| panic!("{}: {key} must be a string", row.id))
                .to_owned()
        };
        let number = |key: &str| {
            artifact[key]
                .as_u64()
                .unwrap_or_else(|| panic!("{}: {key} must be a number", row.id))
        };

        assert_eq!(string("generation_id"), row.id);
        assert_eq!(string("label"), row.label);
        assert_eq!(
            string("native_ancestry_profile_id"),
            row.native_ancestry_profile_id,
            "{}: the artifact qualifies a different ancestry profile than the row publishes",
            row.id
        );
        assert_eq!(
            string("gameplay_tag_float32_map_proof_id"),
            row.gameplay_tag_float32_map_proof_id,
            "{}: the artifact qualifies a different tag-map proof than the row publishes",
            row.id
        );
        assert_eq!(
            number("scalar_default_operand_count"),
            row.scalar_default_operand_count as u64
        );
        assert_eq!(
            number("gameplay_tag_float32_operand_count"),
            row.gameplay_tag_float32_operand_count as u64
        );
        // The anchors the configured real-game suites assert. They are not row fields — they are
        // the observations a row is only trustworthy because somebody made. Two of the three are
        // the same on every build there has ever been, and both would be alarming if they moved:
        // eight is the entire `TMap<FGameplayTag,float32>` surface `patch-tag-map` rests on, and a
        // field the ancestry cannot place is a field the toolkit must refuse to write.
        assert_eq!(number("gameplay_tag_float32_map_field_count"), 8);
        assert_eq!(number("unresolved_fields_with_ancestry"), 0);
        assert!(
            number("class_count") > 0,
            "{}: the qualification records no bridged class at all",
            row.id
        );
        assert_eq!(
            number("direct_windows"),
            row.scalar_default_operand_count as u64,
            "{}: the qualified direct-window count and the sealed scalar operand count are the \
             same measurement and must agree",
            row.id
        );

        let witnesses = artifact["witnesses"]
            .as_object()
            .unwrap_or_else(|| panic!("{}: witnesses must be an object", row.id));
        for key in [
            "native_ancestry_profile_id",
            "gameplay_tag_float32_map_proof_id",
            "scalar_default_operand_count",
            "gameplay_tag_float32_operand_count",
            "class_count",
            "gameplay_tag_float32_map_field_count",
            "unresolved_fields_with_ancestry",
            "direct_windows",
        ] {
            let witness = witnesses
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("{}: {key} names no witness", row.id));
            assert!(
                witness.contains("::"),
                "{}: the witness for {key} must name the test that produced it, got {witness:?}",
                row.id
            );
        }
    }

    // A count that can be anything is not a check, so here is the part of it that is knowable
    // without the game: the bridged class list is a pure function of the `Binds.Cache` bridge and
    // the USMAP class graph, and `resolved_class_profile_sha256` is the digest of exactly that
    // list. Two rows that seal the same digest and record different counts means one artifact was
    // copied rather than measured. It says nothing about a row whose digest is its own — only the
    // named witness and a reader do.
    let mut counted: BTreeMap<[u8; 32], (&str, u64)> = BTreeMap::new();
    for row in rows() {
        let artifact: serde_json::Value =
            serde_json::from_str(registered[row.id]).expect("a qualification artifact is JSON");
        let class_count = artifact["class_count"].as_u64().expect("class_count");
        if let Some((first, expected)) = counted.get(&row.resolved_class_profile_sha256) {
            assert_eq!(
                class_count, *expected,
                "{} and {first} seal one resolved class profile but qualify {class_count} and \
                 {expected} bridged classes",
                row.id
            );
        } else {
            counted.insert(row.resolved_class_profile_sha256, (row.id, class_count));
        }
    }
}
