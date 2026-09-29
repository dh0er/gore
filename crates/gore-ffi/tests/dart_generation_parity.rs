//! Cross-language completeness for generations admitted by Mod Studio.
//!
//! Mod Studio keeps its own copies of the sealed generation triple, in Dart, because the tables are
//! read from a synchronous constructor path that cannot await an FFI call. Nothing in either
//! language can see the other, so a Studio-admitted row forgotten in Dart produces a
//! `FormatException('Story catalog generation is not an exact supported generation triple')` for a
//! user on a build Studio does support — a message that names neither their build nor the
//! supported ones. This string check supplies the cross-language guard the compiler cannot.
//!
//! The NPC source-inspection label must delegate to the same closed executable predicate as NPC
//! drafting. Otherwise a newly admitted row can author and verify an NPC while silently losing its
//! friendly saved-parent label in the UI.
//!
//! Build 25414091 is qualified for item/default operations but Studio's NPC scripts still need a
//! native API snapshot. Its deliberate Studio exclusion is checked below; other new rows must be
//! added to Dart or explicitly gated here.

const PROJECT_BOOTSTRAP: &str =
    include_str!("../../../apps/mod-studio/lib/project/revision3_project_bootstrap.dart");
const NPC_DRAFT: &str =
    include_str!("../../../apps/mod-studio/lib/project/revision3_npc_draft.dart");
const NPC_SOURCE_INSPECTION: &str =
    include_str!("../../../apps/mod-studio/lib/project/revision3_npc_source_inspection.dart");

fn encode_hex(bytes: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[test]
fn dart_generation_tables_cover_every_studio_admitted_row() {
    let blocked = gore_generation::row_by_id("g1r-steam-25414091")
        .expect("the Studio-blocked build remains an audited generation");
    assert_eq!(
        gore_generation::rows()
            .iter()
            .filter(|row| row.id == blocked.id)
            .count(),
        1,
        "the Studio exception must name exactly one generation"
    );
    let blocked_executable = encode_hex(&blocked.executable.sha256);
    let blocked_shipping = encode_hex(&blocked.shipping_cache.sha256);
    assert_eq!(blocked.executable.byte_len, 171_798_528);
    assert_eq!(
        blocked_executable,
        "7394f840702df3ddb94d1a45a43c2a81ff36d7d65325047c698dfb65337b99b5"
    );
    assert_eq!(blocked.shipping_cache.byte_len, 124_459_998);
    assert_eq!(
        blocked_shipping,
        "6c1e1fbb9de3bc92064bd472905d6d3708261fb1323dc21321fbe10579ab4c24"
    );
    assert!(
        !PROJECT_BOOTSTRAP.contains(&blocked_executable),
        "Mod Studio must not admit build 25414091 without its native NPC API snapshot"
    );
    assert!(
        !PROJECT_BOOTSTRAP.contains(&blocked_shipping),
        "Mod Studio must not admit the unqualified build 25414091 Shipping cache"
    );
    assert!(
        !NPC_DRAFT.contains(&blocked_executable),
        "NPC drafting must not admit build 25414091 without its native API snapshot"
    );

    let studio_rows: Vec<_> = gore_generation::rows()
        .iter()
        .filter(|row| row.id != blocked.id)
        .collect();
    for row in &studio_rows {
        for (name, seal) in [
            ("executable", row.executable),
            ("shipping_cache", row.shipping_cache),
            ("binds_cache", row.binds_cache),
        ] {
            let sha256 = encode_hex(&seal.sha256);
            assert!(
                PROJECT_BOOTSTRAP.contains(&sha256),
                "revision3_project_bootstrap.dart does not carry the {name} digest of {}; a Mod \
                 Studio user on that build would be told their install is not supported",
                row.id
            );
            assert!(
                PROJECT_BOOTSTRAP.contains(&seal.byte_len.to_string()),
                "revision3_project_bootstrap.dart does not carry the {name} byte length of {}",
                row.id
            );
        }

        let executable_sha256 = encode_hex(&row.executable.sha256);
        assert!(
            NPC_DRAFT.contains(&executable_sha256),
            "revision3_npc_draft.dart does not carry the executable digest of {}, so NPC drafting \
             would refuse an audited build",
            row.id
        );
        assert!(
            NPC_DRAFT.contains(&row.executable.byte_len.to_string()),
            "revision3_npc_draft.dart does not carry the executable byte length of {}",
            row.id
        );
    }

    // The digests above prove nothing was left out; this proves nothing was left over. A stale
    // entry for a generation the table no longer carries would keep answering for a build nobody
    // audits any more.
    let declared = PROJECT_BOOTSTRAP.matches("edition: 'g1r-steam'").count();
    assert_eq!(
        declared,
        studio_rows.len(),
        "revision3_project_bootstrap.dart declares {declared} supported generations and the table \
         admits {} to Studio",
        studio_rows.len()
    );

    for marker in [
        "const _authoringRevision3NpcExecutableByteLengthV",
        "const _authoringRevision3NpcExecutableSha256V",
    ] {
        let declared = NPC_DRAFT.matches(marker).count();
        assert_eq!(
            declared,
            studio_rows.len(),
            "revision3_npc_draft.dart declares {declared} `{marker}` rows and the table admits {} \
             to Studio",
            studio_rows.len()
        );
    }

    assert!(
        NPC_SOURCE_INSPECTION.contains("_authoringRevision3NpcIsSupportedExecutable("),
        "revision3_npc_source_inspection.dart must reuse the exact NPC generation predicate before \
         showing a friendly saved-parent label"
    );
    assert!(
        !NPC_SOURCE_INSPECTION.contains("_authoringRevision3NpcExecutableByteLengthV"),
        "revision3_npc_source_inspection.dart carries a private generation gate; reuse the shared \
         exact NPC executable predicate so later rows cannot lose their friendly parent label"
    );
}
