//! What the native-evidence banner is allowed to say.
//!
//! These are wording tests on purpose. The failure this module exists for is not a wrong number —
//! the numbers were always right — it is a user reading "19921 unresolved type(s)" under a warning
//! about a USMAP and concluding the tool is broken. The sentences below are the fix, so they are
//! the thing worth pinning.

use gore_as::cache::default_ancestry::{
    is_supported_gameplay_tag_float32_proof_pair, DefaultAncestryError,
};
use gore_as::cache::default_evidence::{
    audited_build_for_profile_id, audited_builds, classify_candidate_failure, CandidateVerdict,
    EvidenceCounts, NativeEvidenceStatus, ObservedBuild, UsmapCandidate, UsmapProof,
};

fn observed() -> ObservedBuild {
    ObservedBuild {
        script_cache_guid: [
            0x7d, 0xd3, 0x6f, 0x66, 0x63, 0xd3, 0x43, 0x40, 0xa6, 0x39, 0x35, 0x8c, 0x73, 0xf8,
            0xe9, 0x1e,
        ],
        script_cache_len: 124_352_336,
        script_cache_sha256: [0x36; 32],
        binds_len: Some(5_908_587),
        binds_sha256: Some([0x85; 32]),
    }
}

fn unsupported() -> NativeEvidenceStatus {
    NativeEvidenceStatus::UnsupportedGeneration {
        observed: observed(),
        audited: audited_builds(),
    }
}

#[test]
fn an_unaudited_build_is_named_alongside_the_builds_that_are() {
    // The whole point of the banner: a user has to be able to quote their own build and see that
    // other builds exist and theirs is not one of them. Anything less and the only actionable
    // reading of the output is "the tool is broken".
    let banner = unsupported().banner(Some(EvidenceCounts {
        editable_sites: 1_147,
        direct_windows: 26_399,
        unresolved_fields: 5_210,
        unresolved_types: 19_921,
    }));

    assert!(
        banner.contains("has not been audited by this toolkit"),
        "the first line must state the cause, got: {banner}"
    );
    assert!(
        banner.contains("7dd36f6663d34340a639358c73f8e91e"),
        "the observed cache GUID must be printed whole so it can be quoted, got: {banner}"
    );
    assert!(
        banner.contains(&"36".repeat(32)),
        "the observed cache digest must be printed whole, got: {banner}"
    );
    assert!(
        banner.contains("124352336 bytes") && banner.contains("5908587 bytes"),
        "both observed lengths must appear, got: {banner}"
    );
    for build in audited_builds() {
        assert!(
            banner.contains(build.id),
            "audited build {} must be named, got: {banner}",
            build.id
        );
    }
    assert!(
        banner.contains("19921")
            && banner.contains("5210")
            && banner.contains("1147")
            && banner.contains("26399"),
        "the counts must sit inside the sentence that explains them, got: {banner}"
    );
    assert!(
        banner.contains("Your install is fine"),
        "the line that decides whether a user files a bug against us must survive, got: {banner}"
    );
}

#[test]
fn an_unaudited_build_is_never_blamed_on_the_reflection_dump() {
    // The bug this replaces: `UnsupportedCache` is a fact about the script cache, and it was
    // printed prefixed with a USMAP path — blaming the one input that had not changed.
    let banner = unsupported().banner(None);
    assert!(
        !banner.to_lowercase().contains("usmap"),
        "an unsupported build must not mention the USMAP at all, got: {banner}"
    );
    assert!(
        banner.contains("Native field types and native ancestry are unavailable"),
        "the cause must still be stated without counts, got: {banner}"
    );
    assert!(
        !banner.contains("window(s)"),
        "no counts were supplied, so none may be invented, got: {banner}"
    );
}

#[test]
fn every_audited_build_carries_the_sealed_pair_it_claims() {
    // This module names generations that the sealed profile table does not name, so the names are
    // written here. The IDs are not: they are read out of the table, and this is what keeps a
    // hand-written row from quietly pairing one generation's ancestry with another's map proof.
    let builds = audited_builds();
    assert!(builds.len() >= 2, "the audited set must not shrink silently");
    for build in &builds {
        assert!(
            is_supported_gameplay_tag_float32_proof_pair(
                build.ancestry_profile_id,
                build.map_proof_id
            ),
            "{} claims a pair the sealed table does not accept",
            build.id
        );
        assert_eq!(
            audited_build_for_profile_id(build.ancestry_profile_id).map(|found| found.id),
            Some(build.id),
            "{} must be reachable by its own profile ID",
            build.id
        );
    }
    for left in &builds {
        for right in &builds {
            if left.id == right.id {
                continue;
            }
            assert_ne!(left.ancestry_profile_id, right.ancestry_profile_id);
            assert!(
                !is_supported_gameplay_tag_float32_proof_pair(
                    left.ancestry_profile_id,
                    right.map_proof_id
                ),
                "{} and {} must not cross-certify",
                left.id,
                right.id
            );
        }
    }
}

#[test]
fn a_drifted_seal_is_not_reported_as_somebody_elses_broken_install() {
    // A drift means an audited build whose parser output moved: our defect, or a damaged file.
    // Rendering it in the unsupported-build wording would send the wrong person looking.
    let banner = NativeEvidenceStatus::SealDrift {
        generation_id: Some("g1r-steam-1.0.3"),
        drift: "its sealed USMAP class graph does not match",
    }
    .banner(None);
    assert!(banner.starts_with("error: "), "got: {banner}");
    assert!(
        !banner.contains("has not been audited"),
        "a drift is not an unaudited build, got: {banner}"
    );
    assert!(
        banner.contains("toolkit defect") && banner.contains("not an unsupported game build"),
        "got: {banner}"
    );
}

#[test]
fn a_failure_about_one_usmap_is_not_a_verdict_about_the_build() {
    // The classifier is the hoist. If a per-file rejection were ever classified as a verdict, the
    // loop would stop on the first stale dump and report an unaudited build that is audited.
    assert_eq!(
        classify_candidate_failure(&DefaultAncestryError::UnsupportedCache),
        CandidateVerdict::UnsupportedCache
    );
    assert_eq!(
        classify_candidate_failure(&DefaultAncestryError::UnsupportedBinds),
        CandidateVerdict::UnsupportedBinds
    );
    for drift in [
        DefaultAncestryError::UsmapGraphDrift,
        DefaultAncestryError::ResolvedProfileDrift,
        DefaultAncestryError::GameplayTagFloat32MapProfileDrift,
        DefaultAncestryError::ProfileIdDrift,
        DefaultAncestryError::GameplayTagFloat32MapProofIdDrift,
    ] {
        assert!(
            matches!(
                classify_candidate_failure(&drift),
                CandidateVerdict::SealDrift(_)
            ),
            "{drift} must be a toolkit-defect verdict"
        );
    }
    for rejected in [
        DefaultAncestryError::UnsupportedUsmap,
        DefaultAncestryError::MissingUsmapIdentity,
        DefaultAncestryError::DuplicateSchemaBridge {
            schema: "/Script/G1R.ItemDefinition".into(),
        },
        DefaultAncestryError::CyclicHierarchy {
            class: "UItemDefinition".into(),
        },
        DefaultAncestryError::BridgeResolution {
            script_class: "UItemDefinition".into(),
            path: "/Script/G1R.ItemDefinition".into(),
            error: "ambiguous".into(),
        },
    ] {
        assert!(
            matches!(
                classify_candidate_failure(&rejected),
                CandidateVerdict::Rejected(_)
            ),
            "{rejected} must reject one file, not the build"
        );
    }
}

#[test]
fn a_missing_usmap_says_which_dumps_were_examined_and_why_each_was_refused() {
    let banner = NativeEvidenceStatus::UsmapMissing {
        generation_id: Some("g1r-steam-1.0.3"),
        examined: vec![UsmapCandidate {
            path: "C:/game/G1R/Binaries/Win64/ue4ss/G1R-5.4.3-168781-272ce2f8.usmap".into(),
            sha256: Some([0x73; 32]),
            rejection: "sealed for a different build",
        }],
    }
    .banner(None);
    assert!(banner.contains("G1R-5.4.3-168781-272ce2f8.usmap"), "got: {banner}");
    assert!(banner.contains("sealed for a different build"), "got: {banner}");
    assert!(
        banner.contains("UE4SS dump produced on your machine"),
        "the remedy is a re-dump, and only this sentence says so, got: {banner}"
    );
}

#[test]
fn a_qualified_status_names_the_generation_and_not_only_its_digest() {
    let audited = audited_builds();
    let first = audited.first().expect("at least one audited build");
    let status = NativeEvidenceStatus::qualified(
        first.ancestry_profile_id,
        Some(UsmapProof {
            path: "C:/game/G1R/Binaries/Win64/ue4ss/dump.usmap".into(),
            sha256: [0x73; 32],
        }),
    );
    assert!(status.is_qualified());
    assert_eq!(status.kind(), "qualified");
    let banner = status.banner(None);
    assert!(
        banner.contains(first.id),
        "a qualified run must name the build it qualified as, got: {banner}"
    );
    assert!(banner.contains("dump.usmap"), "got: {banner}");
    assert!(
        !banner.contains('\n'),
        "a success is one line, not a banner block, got: {banner}"
    );
}

#[test]
fn a_caller_that_asked_for_no_native_evidence_is_told_nothing() {
    // `gore as default-sites` on a mini-cache from `gore as extract` has no Binds and wants none.
    // An empty banner is what keeps that case from printing a warning about a build nobody named.
    assert!(NativeEvidenceStatus::NotRequested.banner(None).is_empty());
    assert_eq!(NativeEvidenceStatus::NotRequested.kind(), "not_requested");
}
