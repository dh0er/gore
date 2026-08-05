//! Cross-language completeness for the generation table.
//!
//! Mod Studio keeps its own copies of the sealed generation triple, in Dart, because the tables are
//! read from a synchronous constructor path that cannot await an FFI call. Nothing in either
//! language can see the other, so a row added to `gore-generation` and forgotten in Dart produces a
//! `FormatException('Story catalog generation is not an exact supported generation triple')` for a
//! user on a build the toolkit does support — a message that names neither their build nor the
//! supported ones. This is a string grep, it is ugly, and it is exactly the friction the compiler
//! used to supply and cannot supply across a language boundary.
//!
//! The NPC source-inspection label must delegate to the same closed executable predicate as NPC
//! drafting. Otherwise a newly admitted row can author and verify an NPC while silently losing its
//! friendly saved-parent label in the UI.

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
fn dart_generation_tables_cover_every_row() {
    for row in gore_generation::rows() {
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
        gore_generation::rows().len(),
        "revision3_project_bootstrap.dart declares {declared} supported generations and the table \
         has {}",
        gore_generation::rows().len()
    );

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
