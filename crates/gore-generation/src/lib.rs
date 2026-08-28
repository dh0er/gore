//! The audited game-generation table.
//!
//! One [`GenerationRow`] per exact shipped build. Every sealed file identity, every parser-output
//! digest and every published derived ID for that build is a field of exactly one row, so a new
//! generation is one struct literal instead of seven constants scattered over six files in two
//! crates that cannot see each other. The crate is a leaf on purpose: `sha2` and nothing else, so
//! `gore-as`, `gore-story-catalog` and `gore-ffi` can all depend on it without any of them
//! depending on another.
//!
//! **What a row does not prove.** Nothing here forces a row's digests to have come from a real run
//! against real game bytes. `every_row_derives_its_own_published_ids` proves a row is internally
//! consistent — any one of its ten sealed components being wrong changes the profile ID the row
//! publishes, and that ID appears in every selector and every receipt — and
//! `every_row_has_a_committed_qualification_artifact` proves somebody committed the measurement
//! that produced it. Only a human reading that artifact proves the row is *true*. Adding a row is a
//! governance act, not a table edit.
//!
//! [`qualify`] is the machine-side half of that governance: the pure derivations `gore as qualify`
//! runs against an installation so that the numbers in a new row are measured rather than typed. It
//! proposes; it never admits.

use sha2::{Digest, Sha256};

pub mod qualify;

/// Versioned combined script-cache fingerprint format. It is one of the nine components of a row's
/// derived profile ID, so it lives beside the rows rather than where it is computed: a format bump
/// that did not reach the table would otherwise mint IDs no row could reproduce.
pub const CACHE_FINGERPRINT_FORMAT: &str = "gore-as-default-cache-fingerprint-v2-scalar-tag";

/// Shape string bound into every GameplayTag-to-float32 map proof ID. Not build-bound.
const GAMEPLAY_TAG_FLOAT32_MAP_SEMANTIC_ID: &[u8] =
    b"usmap-class-declared-case-sensitive-array-dim-1:Map{key=Struct(GameplayTag),value=Float}";

/// A file identified by its exact length and SHA-256. Const-constructible, so it can be a row
/// field; `gore-story-catalog` converts it to its own serde-carrying `ContentSeal` at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSeal {
    pub byte_len: u64,
    pub sha256: [u8; 32],
}

/// The three script-cache facts that, with the cache GUID, decide which row a cache belongs to.
/// Mirrors `gore_as::cache::default_fingerprint::DefaultCacheFingerprint`; `gore-as` converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheFingerprint {
    pub sha256: [u8; 32],
    pub scalar_operand_count: usize,
    pub tag_operand_count: usize,
}

/// The nine identity components a native-ancestry profile ID is derived from. Built from a row by
/// [`GenerationRow::profile_components`] and, at run time, from *observed* bytes by `gore-as`. Both
/// go through [`derived_profile_sha256`], which is what stops a sealed ID from drifting away from
/// the check that admits a cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileComponents {
    pub fingerprint_format: &'static str,
    pub script_cache_guid: [u8; 16],
    pub script_cache_mutation_stable_sha256: [u8; 32],
    pub scalar_default_operand_count: usize,
    pub gameplay_tag_float32_operand_count: usize,
    pub binds_source_sha256: [u8; 32],
    pub binds_bridge_sha256: [u8; 32],
    pub usmap_source_sha256: [u8; 32],
    pub usmap_graph_sha256: [u8; 32],
    pub resolved_profile_sha256: [u8; 32],
}

/// One audited game generation. Every sealed identity and every published derived ID for one exact
/// shipped build is a field of exactly one of these.
///
/// A row is only ever added together with `qualifications/<id>.json`, the committed record of the
/// run that produced its numbers. The table cannot tell a measured digest from a pasted one; the
/// artifact and the person who read it are what can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationRow {
    // --- naming -------------------------------------------------------------------------------
    /// Stable machine key. Compared and printed, never parsed.
    pub id: &'static str,
    /// Banner text for humans.
    pub label: &'static str,
    pub edition: &'static str,

    // --- raw file identity (`gore-story-catalog`'s gate) --------------------------------------
    pub executable: FileSeal,
    pub shipping_cache: FileSeal,
    pub binds_cache: FileSeal,
    /// The UE4SS reflection dump. It is produced on a player's machine, not shipped by Steam, so
    /// two rows may legitimately share one. `byte_len` is diagnostic; nothing checks it today.
    pub usmap: FileSeal,

    // --- script-cache identity (`gore-as`'s gate) ---------------------------------------------
    pub script_cache_guid: [u8; 16],
    pub script_cache_mutation_stable_sha256: [u8; 32],
    pub scalar_default_operand_count: usize,
    pub gameplay_tag_float32_operand_count: usize,

    // --- parser-output digests (defend against a silent parser change) ------------------------
    pub binds_field_map_sha256: [u8; 32],
    pub binds_class_path_map_sha256: [u8; 32],
    pub usmap_class_graph_sha256: [u8; 32],
    pub resolved_class_profile_sha256: [u8; 32],
    pub gameplay_tag_float32_map_profile_sha256: [u8; 32],

    // --- published derived IDs (re-derivable from the two blocks above) -----------------------
    pub native_ancestry_profile_id: &'static str,
    pub gameplay_tag_float32_map_proof_id: &'static str,

    // --- story-catalog revision payload -------------------------------------------------------
    pub record_set_id: &'static str,
    pub record_set_seal: FileSeal,
    pub catalog_payload_seal: FileSeal,
    /// Appears verbatim in `CatalogError` text, so it is a compatibility surface.
    pub catalog_label: &'static str,
    pub record_seal_kind: &'static str,
    pub catalog_seal_kind: &'static str,

    // --- item authoring -----------------------------------------------------------------------
    pub audited_item_generation: &'static str,
}

impl GenerationRow {
    /// The nine components whose digest is this row's [`Self::native_ancestry_profile_id`].
    pub fn profile_components(&self) -> ProfileComponents {
        ProfileComponents {
            fingerprint_format: CACHE_FINGERPRINT_FORMAT,
            script_cache_guid: self.script_cache_guid,
            script_cache_mutation_stable_sha256: self.script_cache_mutation_stable_sha256,
            scalar_default_operand_count: self.scalar_default_operand_count,
            gameplay_tag_float32_operand_count: self.gameplay_tag_float32_operand_count,
            binds_source_sha256: self.binds_cache.sha256,
            binds_bridge_sha256: self.binds_class_path_map_sha256,
            usmap_source_sha256: self.usmap.sha256,
            usmap_graph_sha256: self.usmap_class_graph_sha256,
            resolved_profile_sha256: self.resolved_class_profile_sha256,
        }
    }
}

/// Steam 1.0.3 Hotfix 1 — the first generation this toolkit sealed.
pub const ROW_G1R_1_0_3: GenerationRow = GenerationRow {
    id: "g1r-steam-1.0.3",
    label: "Steam 1.0.3 Hotfix 1",
    edition: "g1r-steam",

    executable: FileSeal {
        byte_len: 171_698_176,
        sha256: hex("f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5"),
    },
    shipping_cache: FileSeal {
        byte_len: 123_394_250,
        sha256: hex("1018f1cfe6b99a650eecb33afb96752d691d2088ead27808971b812f04ecb4c2"),
    },
    binds_cache: FileSeal {
        byte_len: 5_903_938,
        sha256: hex("46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea"),
    },
    usmap: FileSeal {
        byte_len: 2_516_955,
        sha256: hex("73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca"),
    },

    script_cache_guid: hex("450d65c04f0c014fbec568016378e69a"),
    script_cache_mutation_stable_sha256: hex(
        "01fe4e37cc3a5dee15c2beb49a3f406110774b5e300f2de4ad811d0df9addd6b",
    ),
    scalar_default_operand_count: 26_339,
    gameplay_tag_float32_operand_count: 1_432,

    binds_field_map_sha256: hex(
        "5ddf7fa6df36ac00d07bd068fcf19ad61a3f4b836133513966dc379b24241707",
    ),
    binds_class_path_map_sha256: hex(
        "cffbce6feb2f8c14dc5f25193741f58951c16f270a76773125d0e507d36e95c4",
    ),
    usmap_class_graph_sha256: hex(
        "0e64322222d3d32c5cd41254532d518be5feb722a24ed0142284fa4ec91d679d",
    ),
    resolved_class_profile_sha256: hex(
        "1763379bcb89816d072724515475b28613031eff3b866244f2331ac59c4064fa",
    ),
    gameplay_tag_float32_map_profile_sha256: hex(
        "5fa2e35616cb6b04a3060202e55ff575d8e8aeab5a25602aeddc10b3ad542708",
    ),

    native_ancestry_profile_id:
        "sha256:98da5430f213b0107bd7361fa3c78316bf5320fbd15a53a9258d50d8d3ac9ed5",
    gameplay_tag_float32_map_proof_id:
        "sha256:f20ce5ce571f3d121046ac1942e0705cfb30c3761a3e390cd5d77ea2c16159cc",

    record_set_id: "g1r-steam-1.0.3-curated-story-v1",
    record_set_seal: FileSeal {
        byte_len: 5_499,
        sha256: hex("323ffe3fb3d6394c0d4397d090aabddb5e87c1ac7e5cecd14382b0a4f0516fc8"),
    },
    catalog_payload_seal: FileSeal {
        byte_len: 5_611,
        sha256: hex("51192393aa28cff00b1a4e59de7793a8db354e30692569719c4b46e2f9bc4853"),
    },
    catalog_label: "compiled curated V1",
    record_seal_kind: "compiled curated V1 record set",
    catalog_seal_kind: "compiled curated V1 catalog payload",

    audited_item_generation: "g1r-steam-generation-v1",
};

/// Steam 1.0.3 Hotfix 2, Steam BuildID 24169431. It reused the previous `Binds.Cache` and USMAP
/// byte for byte and changed only the executable and the script cache — which is why those two
/// seals are the only fields it does not share with [`ROW_G1R_1_0_3`].
pub const ROW_G1R_24169431: GenerationRow = GenerationRow {
    id: "g1r-steam-24169431",
    label: "Steam 1.0.3 Hotfix 2 (build 24169431)",
    edition: "g1r-steam",

    executable: FileSeal {
        byte_len: 171_704_320,
        sha256: hex("b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d"),
    },
    shipping_cache: FileSeal {
        byte_len: 123_394_250,
        sha256: hex("757d8624f0c7480f63cc14a1ba2d7e43f461a529064b0c0cfbf523a54639e385"),
    },
    binds_cache: FileSeal {
        byte_len: 5_903_938,
        sha256: hex("46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea"),
    },
    usmap: FileSeal {
        byte_len: 2_516_955,
        sha256: hex("73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca"),
    },

    script_cache_guid: hex("43521b38497e984f8abbc035eb4cb1d7"),
    script_cache_mutation_stable_sha256: hex(
        "21211187eca2889f04e2baf95da22d4e7188287341b76eded1188a1d085434c5",
    ),
    scalar_default_operand_count: 26_339,
    gameplay_tag_float32_operand_count: 1_432,

    binds_field_map_sha256: hex(
        "5ddf7fa6df36ac00d07bd068fcf19ad61a3f4b836133513966dc379b24241707",
    ),
    binds_class_path_map_sha256: hex(
        "cffbce6feb2f8c14dc5f25193741f58951c16f270a76773125d0e507d36e95c4",
    ),
    usmap_class_graph_sha256: hex(
        "0e64322222d3d32c5cd41254532d518be5feb722a24ed0142284fa4ec91d679d",
    ),
    resolved_class_profile_sha256: hex(
        "1763379bcb89816d072724515475b28613031eff3b866244f2331ac59c4064fa",
    ),
    gameplay_tag_float32_map_profile_sha256: hex(
        "5fa2e35616cb6b04a3060202e55ff575d8e8aeab5a25602aeddc10b3ad542708",
    ),

    native_ancestry_profile_id:
        "sha256:b7e13f7f3756e97a07194bdbd6ba6a1f2cb99179888d0d8e581f505be969b645",
    gameplay_tag_float32_map_proof_id:
        "sha256:b56365eff74dc11610c0e2f08dcb41923773bbb6efc954403c6ea09c48239b8a",

    record_set_id: "g1r-steam-1.0.3-curated-story-v2",
    record_set_seal: FileSeal {
        byte_len: 5_499,
        sha256: hex("3dcf62650b9c4c5c320988644adb3e10a4f3888dba9447b8d0ef06da2d541def"),
    },
    catalog_payload_seal: FileSeal {
        byte_len: 5_611,
        sha256: hex("e93bbd62fc824ca8166c3ab9b67f21cf1493295969bf19adff438d665fc16bc3"),
    },
    catalog_label: "compiled curated V2",
    record_seal_kind: "compiled curated V2 record set",
    catalog_seal_kind: "compiled curated V2 catalog payload",

    audited_item_generation: "g1r-steam-hotfix-24169431",
};

/// Steam BuildID 24340829, shipped 2026-07-31. The first generation to move all four sealed files
/// at once: a new executable, a new script cache, the first new `Binds.Cache` since 1.0.3, and the
/// first USMAP re-dump. The reflection layout moved with it — 12 native classes added, 4 removed,
/// 60 class properties added — but no class was reparented and no property changed which class
/// declares it, which is why every previously audited default site still resolves to the same owner
/// and this is a re-seal rather than a re-audit. See `qualifications/g1r-steam-24340829.json`.
pub const ROW_G1R_24340829: GenerationRow = GenerationRow {
    id: "g1r-steam-24340829",
    label: "Steam build 24340829 (2026-07-31 update)",
    edition: "g1r-steam",

    executable: FileSeal {
        byte_len: 171_787_776,
        sha256: hex("ab2c8d9e286a437bc5343748faf40959a77e9dc7c542ff9361f1ffaeca5c811c"),
    },
    shipping_cache: FileSeal {
        byte_len: 124_352_336,
        sha256: hex("36124f1cdd4caae555423581aa40631af0ac80d5cef42528382739f932b0e728"),
    },
    binds_cache: FileSeal {
        byte_len: 5_908_587,
        sha256: hex("854f58a695d0170144957f085c1e8c0f9ef40b271e35e90f79ffbccff8d999c5"),
    },
    // `G1R-5.4.3-171261-272ce2f8.usmap`, dumped 2026-07-31 against this executable. The
    // `…-168781-…` dump beside it still hashes to the first two rows' seal and describes the
    // previous build; sealing from it would have left 12 Binds-named classes with no ancestry.
    usmap: FileSeal {
        byte_len: 2_487_358,
        sha256: hex("67b9e645bcfd9f4f360e11f611da92896cf7d6280f2747ad0c8bbe7542f024c7"),
    },

    script_cache_guid: hex("7dd36f6663d34340a639358c73f8e91e"),
    script_cache_mutation_stable_sha256: hex(
        "cad69ccdf7803d699fac14fe13dda3e4696c7aa9cbb80bc81a2341d95d919b13",
    ),
    scalar_default_operand_count: 26_399,
    gameplay_tag_float32_operand_count: 1_432,

    binds_field_map_sha256: hex(
        "cdcda9a21ec63d1ebe47f1bd6454a1e20a345e1cd961510f6826f1f1cf520395",
    ),
    binds_class_path_map_sha256: hex(
        "628d133fb6c6733c0d3d9e2710ae79d2758235582e9afed13201b84abaa7612b",
    ),
    usmap_class_graph_sha256: hex(
        "642a8e0bc80301935a8a46498c00761b048cbac22bbd74fa914e86b6399fe321",
    ),
    resolved_class_profile_sha256: hex(
        "a09e158df376b1f20b302f6eaa1e7476e660a7244c8d737b5a38f67243b52f5f",
    ),
    // Unchanged from the first two rows, and that is a measured result rather than an inheritance:
    // the eight `TMap<FGameplayTag,float32>` declarations are the same eight, on the same seven
    // owners, at the same canonical paths, in both USMAP dumps.
    gameplay_tag_float32_map_profile_sha256: hex(
        "5fa2e35616cb6b04a3060202e55ff575d8e8aeab5a25602aeddc10b3ad542708",
    ),

    native_ancestry_profile_id:
        "sha256:0c45197727b24cdcab7eb18e28ed248906f6e9dd1449f7e983eda59535eb1e4a",
    gameplay_tag_float32_map_proof_id:
        "sha256:18624b5b78f756af2820a928bec7a5607ad9b3d75c9a422790d81ddf8b87dc96",

    record_set_id: "g1r-steam-1.0.3-curated-story-v3",
    record_set_seal: FileSeal {
        byte_len: 5_499,
        sha256: hex("aac21b0bb339184f241a9533c180ba00c07602fbb16c134079d6ae527fc153b8"),
    },
    catalog_payload_seal: FileSeal {
        byte_len: 5_611,
        sha256: hex("cb6ac92d746a8dffd738eefb9fa2d15d9ba84b2940e1115c133a2d155a250c7d"),
    },
    catalog_label: "compiled curated V3",
    record_seal_kind: "compiled curated V3 record set",
    catalog_seal_kind: "compiled curated V3 catalog payload",

    audited_item_generation: "g1r-steam-build-24340829",
};

/// Steam BuildID 24878692, shipped 2026-08-27/28. The script-cache wire format, class graph,
/// resolved class bridge and GameplayTag-map surface remain unchanged, while the exact cache and
/// ordered native API moved. The fresh 172709 UE4SS dump adds
/// `/Script/G1R.RagdollConfig.m_FollowBone`; the previous 171261 dump is therefore not valid
/// evidence for this row. The ten fewer scalar windows are fully accounted for by removed and
/// added game-content modules rather than a parser regression. See
/// `qualifications/g1r-steam-24878692.json`.
pub const ROW_G1R_24878692: GenerationRow = GenerationRow {
    id: "g1r-steam-24878692",
    label: "Steam build 24878692 (2026-08-27/28 update)",
    edition: "g1r-steam",

    executable: FileSeal {
        byte_len: 171_792_384,
        sha256: hex("824fbc94f2ac7f45927a0754605666c37af862d66156a15f8bf6813759d9e8e0"),
    },
    shipping_cache: FileSeal {
        byte_len: 124_459_412,
        sha256: hex("7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511"),
    },
    binds_cache: FileSeal {
        byte_len: 5_908_985,
        sha256: hex("aa73402c11d4007035a2df32c55e50086a6d9c5b6da8619cdfcb4df53f02cea2"),
    },
    usmap: FileSeal {
        byte_len: 2_415_439,
        sha256: hex("6270c9d5fe22228ac78fc6a10330c2dd861e7d161d236a69eb550e6300b9c6bf"),
    },

    script_cache_guid: hex("7835bcc09c5eee488d72cb5ffb0fb0c3"),
    script_cache_mutation_stable_sha256: hex(
        "35a89f22ead3f3eed06fd5ccd00def41ae96da392bad78339984bf5630ed7512",
    ),
    scalar_default_operand_count: 26_389,
    gameplay_tag_float32_operand_count: 1_432,

    binds_field_map_sha256: hex(
        "d7feb69355e5d66d02c66a9d5ca6ff3d675520bea10093cb8001bde0d47aafda",
    ),
    binds_class_path_map_sha256: hex(
        "628d133fb6c6733c0d3d9e2710ae79d2758235582e9afed13201b84abaa7612b",
    ),
    usmap_class_graph_sha256: hex(
        "642a8e0bc80301935a8a46498c00761b048cbac22bbd74fa914e86b6399fe321",
    ),
    resolved_class_profile_sha256: hex(
        "a09e158df376b1f20b302f6eaa1e7476e660a7244c8d737b5a38f67243b52f5f",
    ),
    gameplay_tag_float32_map_profile_sha256: hex(
        "5fa2e35616cb6b04a3060202e55ff575d8e8aeab5a25602aeddc10b3ad542708",
    ),

    native_ancestry_profile_id:
        "sha256:e82fd03d8fa86a1417a6c4d23444d129e2aae293368b1912b077956af2b51827",
    gameplay_tag_float32_map_proof_id:
        "sha256:92df7492e40d6f65db8789beb8660e4d365b4e42d0450e8845e834d691d148b3",

    record_set_id: "g1r-steam-1.0.3-curated-story-v4",
    record_set_seal: FileSeal {
        byte_len: 5_499,
        sha256: hex("e7cf13f8b83ad3e111af7c0be83ae7d571d03cfa7d1f9755695f5344dd9680f9"),
    },
    catalog_payload_seal: FileSeal {
        byte_len: 5_611,
        sha256: hex("9438cc344ccc67556daf00b10be2b2310d1dcd30183e689eac95e261596622d3"),
    },
    catalog_label: "compiled curated V4",
    record_seal_kind: "compiled curated V4 record set",
    catalog_seal_kind: "compiled curated V4 catalog payload",

    audited_item_generation: "g1r-steam-24878692",
};

/// Every audited generation, oldest first. A fixed-length array so the length appears in the diff
/// of any commit that adds a row — the one piece of the old array-shaped friction worth keeping.
pub static GENERATION_ROWS: [GenerationRow; 4] = [
    ROW_G1R_1_0_3,
    ROW_G1R_24169431,
    ROW_G1R_24340829,
    ROW_G1R_24878692,
];

/// The committed qualification artifact per row, keyed by [`GenerationRow::id`]. A row without one
/// fails `every_row_has_a_committed_qualification_artifact`.
pub const QUALIFICATION_ARTIFACTS: [(&str, &str); 4] = [
    (
        ROW_G1R_1_0_3.id,
        include_str!("../qualifications/g1r-steam-1.0.3.json"),
    ),
    (
        ROW_G1R_24169431.id,
        include_str!("../qualifications/g1r-steam-24169431.json"),
    ),
    (
        ROW_G1R_24340829.id,
        include_str!("../qualifications/g1r-steam-24340829.json"),
    ),
    (
        ROW_G1R_24878692.id,
        include_str!("../qualifications/g1r-steam-24878692.json"),
    ),
];

pub fn rows() -> &'static [GenerationRow] {
    &GENERATION_ROWS
}

pub fn row_by_id(id: &str) -> Option<&'static GenerationRow> {
    GENERATION_ROWS.iter().find(|row| row.id == id)
}

/// The `gore-as` admission gate: an exact cache GUID *and* an exact combined fingerprint.
pub fn row_for_script_cache(
    script_cache_guid: &[u8; 16],
    fingerprint: &CacheFingerprint,
) -> Option<&'static GenerationRow> {
    GENERATION_ROWS.iter().find(|row| {
        script_cache_guid == &row.script_cache_guid
            && fingerprint.sha256 == row.script_cache_mutation_stable_sha256
            && fingerprint.scalar_operand_count == row.scalar_default_operand_count
            && fingerprint.tag_operand_count == row.gameplay_tag_float32_operand_count
    })
}

/// The GUID half of that gate on its own. `Binds.Cache` evidence is admitted by GUID alone and the
/// fingerprint is checked one layer up; the two layers are deliberately separate.
pub fn row_for_script_cache_guid(script_cache_guid: &[u8; 16]) -> Option<&'static GenerationRow> {
    GENERATION_ROWS
        .iter()
        .find(|row| script_cache_guid == &row.script_cache_guid)
}

/// The `gore-story-catalog` gate: the complete executable/Shipping/Binds triple. Matching one or
/// two of the three is intentionally insufficient.
pub fn row_for_file_seals(
    executable: &FileSeal,
    shipping_cache: &FileSeal,
    binds_cache: &FileSeal,
) -> Option<&'static GenerationRow> {
    GENERATION_ROWS.iter().find(|row| {
        &row.executable == executable
            && &row.shipping_cache == shipping_cache
            && &row.binds_cache == binds_cache
    })
}

/// Executables are pairwise distinct across rows (`rows_are_pairwise_distinct`), so the executable
/// alone is an unambiguous key. Item authoring uses it because a project anchor holds nothing else.
pub fn row_for_executable(executable: &FileSeal) -> Option<&'static GenerationRow> {
    GENERATION_ROWS
        .iter()
        .find(|row| &row.executable == executable)
}

pub fn row_by_profile_id(native_ancestry_profile_id: &str) -> Option<&'static GenerationRow> {
    GENERATION_ROWS
        .iter()
        .find(|row| row.native_ancestry_profile_id == native_ancestry_profile_id)
}

/// Whether the two selector identities are one exact supported ancestry/map-proof pair.
/// Independently recognized IDs from different generations are deliberately rejected.
pub fn is_supported_tag_proof_pair(
    native_ancestry_profile_id: &str,
    gameplay_tag_float32_map_proof_id: &str,
) -> bool {
    GENERATION_ROWS.iter().any(|row| {
        row.native_ancestry_profile_id == native_ancestry_profile_id
            && row.gameplay_tag_float32_map_proof_id == gameplay_tag_float32_map_proof_id
    })
}

/// Rows sharing one `Binds.Cache` file. Steam has already shipped a build that reused the previous
/// one byte for byte, so this is a set, not a lookup.
pub fn rows_for_binds_sha256(sha256: &[u8; 32]) -> impl Iterator<Item = &'static GenerationRow> {
    let wanted = *sha256;
    GENERATION_ROWS
        .iter()
        .filter(move |row| row.binds_cache.sha256 == wanted)
}

/// The `(field map, class-path map)` parser-output digests sealed for a `Binds.Cache`, or `None`
/// when no row carries it — or when the rows carrying it disagree, which
/// `binds_digests_are_consistent_per_file` proves cannot happen.
pub fn binds_digests_for_sha256(sha256: &[u8; 32]) -> Option<([u8; 32], [u8; 32])> {
    let mut digests: Option<([u8; 32], [u8; 32])> = None;
    for row in rows_for_binds_sha256(sha256) {
        let row_digests = (row.binds_field_map_sha256, row.binds_class_path_map_sha256);
        if let Some(seen) = digests {
            if seen != row_digests {
                return None;
            }
        } else {
            digests = Some(row_digests);
        }
    }
    digests
}

/// Identity of the complete atomic production evidence tuple: versioned combined cache fingerprint
/// and exact range counts, cache GUID, Binds bytes/bridge, and USMAP bytes/graphs.
pub fn derived_profile_sha256(components: &ProfileComponents) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update((components.fingerprint_format.len() as u32).to_le_bytes());
    hash.update(components.fingerprint_format.as_bytes());
    hash.update(components.script_cache_guid);
    hash.update(components.script_cache_mutation_stable_sha256);
    hash.update((components.scalar_default_operand_count as u64).to_le_bytes());
    hash.update((components.gameplay_tag_float32_operand_count as u64).to_le_bytes());
    hash.update(components.binds_source_sha256);
    hash.update(components.binds_bridge_sha256);
    hash.update(components.usmap_source_sha256);
    hash.update(components.usmap_graph_sha256);
    hash.update(components.resolved_profile_sha256);
    hash.finalize().into()
}

pub fn map_proof_sha256(ancestry_profile: &[u8; 32], field_profile: &[u8; 32]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(ancestry_profile);
    hash.update(field_profile);
    hash.update(GAMEPLAY_TAG_FLOAT32_MAP_SEMANTIC_ID);
    hash.finalize().into()
}

/// Decode a published `sha256:<64 lowercase hex>` ID back to its digest. `None` for anything else.
pub fn expected_id_sha256(id: &str) -> Option<[u8; 32]> {
    let hex = id.strip_prefix("sha256:")?;
    if hex.len() != 64 {
        return None;
    }
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&hex[offset..offset + 2], 16).ok()?;
    }
    Some(output)
}

/// Build a seal from the two things a person actually has: a byte count and a pasted digest.
/// `const`, so a mistyped digest is a compile error rather than a seal that quietly means
/// something else. Exposed because the archived-executable matrix in `gore-as` seals files that
/// are not generations and would otherwise grow its own decoder.
pub const fn file_seal(byte_len: u64, sha256: &str) -> FileSeal {
    FileSeal {
        byte_len,
        sha256: hex(sha256),
    }
}

/// Digests are written as hex in the table because a reviewer has to compare them against a hash
/// somebody pasted from a terminal. Decoding is `const`, so a mistyped digest is a compile error
/// rather than a row that quietly means something else.
const fn hex<const N: usize>(text: &str) -> [u8; N] {
    let bytes = text.as_bytes();
    assert!(
        bytes.len() == N * 2,
        "a sealed digest must be exactly twice its byte length in lowercase hex"
    );
    let mut output = [0u8; N];
    let mut index = 0;
    while index < N {
        output[index] = (nibble(bytes[index * 2]) << 4) | nibble(bytes[index * 2 + 1]);
        index += 1;
    }
    output
}

const fn nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("a sealed digest must be lowercase hexadecimal"),
    }
}
