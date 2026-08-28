use gore_catalog::{
    category_for_id, parse_catalog, resolve_item_icon, resolve_item_icon_with_source, ItemCategory,
    ItemIconEvidence, ItemIconSource,
};
use std::path::PathBuf;

fn bundled_save_editor_catalog() -> Vec<gore_catalog::CatalogJsonEntry> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/save-editor/assets/item_catalog.json");
    parse_catalog(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn category_food() {
    assert_eq!(category_for_id("ItFo_Apple"), ItemCategory::Food);
}

#[test]
fn category_weapon_melee() {
    assert_eq!(
        category_for_id("ItMw_1H_Sword_01"),
        ItemCategory::MeleeWeapon
    );
}

#[test]
fn category_misc() {
    assert_eq!(category_for_id("ItMi_Orenugget"), ItemCategory::Misc);
}

#[test]
fn category_ammunition() {
    assert_eq!(category_for_id("ItAm_Arrow"), ItemCategory::Ammunition);
}

#[test]
fn category_amulet() {
    // Amulets/rings are ItAt_Amulet_* / ItAt_Ring_* — rest of ItAt_ = Trophy
    assert_eq!(category_for_id("ItAt_Amulet_01"), ItemCategory::Jewelry);
    assert_eq!(category_for_id("ItAt_Ring_01"), ItemCategory::Jewelry);
    assert_eq!(category_for_id("ItAt_Trophy_Wolf"), ItemCategory::Trophy);
}

#[test]
fn category_rune() {
    assert_eq!(
        category_for_id("ItAr_Rune_Fireball"),
        ItemCategory::RuneOrScroll
    );
}

#[test]
fn category_key_and_mission() {
    assert_eq!(category_for_id("ItKe_Lockpick"), ItemCategory::Key);
    assert_eq!(category_for_id("ItKeyDefault"), ItemCategory::Key);
    assert_eq!(category_for_id("ItChestKey01"), ItemCategory::Key);
    assert_eq!(category_for_id("ItDoorKey01"), ItemCategory::Key);
    assert_eq!(category_for_id("ItMs_Ashes"), ItemCategory::Mission);
}

#[test]
fn category_armor_for_armor_classes() {
    assert_eq!(category_for_id("Armor_OC_Gomez"), ItemCategory::Armor);
    assert_eq!(category_for_id("Ore_Armor_H"), ItemCategory::Armor);
}

#[test]
fn category_unknown_for_unrecognized() {
    assert_eq!(
        category_for_id("TotallyUnknownClass"),
        ItemCategory::Unknown
    );
}

#[test]
fn bundled_item_catalog_has_complete_deterministic_icons() {
    let entries = bundled_save_editor_catalog();
    assert_eq!(entries.len(), 831);

    let mut source_counts = [0usize; 8];
    let mut evidence_counts = [0usize; 4];
    for entry in &entries {
        assert!(!entry.icon.is_empty(), "{} has no icon", entry.id);
        assert_ne!(
            entry.icon, "NoItem",
            "{} uses an empty placeholder",
            entry.id
        );
        assert_ne!(entry.icon, "WIP", "{} uses a grey placeholder", entry.id);
        assert_eq!(entry.icon, resolve_item_icon(&entry.id), "{}", entry.id);
        let resolution = resolve_item_icon_with_source(&entry.id);
        source_counts[resolution.source as usize] += 1;
        evidence_counts[resolution.evidence as usize] += 1;
    }

    assert_eq!(source_counts[ItemIconSource::Direct as usize], 676);
    assert_eq!(source_counts.iter().sum::<usize>(), 831);
    assert_eq!(
        evidence_counts,
        [795, 23, 4, 9],
        "script default, inherited script default, explicit empty, unresolved"
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| resolve_item_icon_with_source(&entry.id).uses_fallback)
            .count(),
        19
    );
}

#[test]
fn lower_case_texture_prefix_exceptions_keep_the_item_id_as_icon() {
    let entries = bundled_save_editor_catalog();
    for id in ["ItMi_Stuff_Brush", "ItMi_Oldcoin_01"] {
        let entry = entries.iter().find(|entry| entry.id == id).unwrap();
        assert_eq!(entry.icon, id);
        assert_eq!(
            resolve_item_icon_with_source(id).source,
            ItemIconSource::Direct
        );
    }
}

#[test]
fn icon_evidence_distinguishes_defaults_inheritance_and_fallbacks() {
    assert_eq!(
        resolve_item_icon_with_source("ItMi_Smith_1H_Sword_03").evidence,
        ItemIconEvidence::ScriptDefault
    );
    assert_eq!(
        resolve_item_icon_with_source("QA_Armor").evidence,
        ItemIconEvidence::InheritedScriptDefault
    );
    assert_eq!(
        resolve_item_icon_with_source("ItMw_Smith_IntermediateSword_01").evidence,
        ItemIconEvidence::ExplicitEmptyFallback
    );
    assert_eq!(
        resolve_item_icon_with_source("ItMs_SunkenTowerStone_01").evidence,
        ItemIconEvidence::UnresolvedFallback
    );
    assert!(resolve_item_icon_with_source("ItMw_2H_Scepter_Skeletonmage").uses_fallback);
    assert_eq!(
        resolve_item_icon_with_source("ItMw_2H_Scepter_Skeletonmage").evidence,
        ItemIconEvidence::ScriptDefault
    );
}

#[test]
fn inherited_icon_matrix_matches_the_installed_script_cache() {
    let expected = [
        ("Armor_BC_BAN_Arlin_852", "Vlk_Armor_M"),
        ("Armor_OCR_GRD_Stone_219", "Vlk_Armor_M"),
        ("Armor_OCR_GRD_Stone_219_03", "Vlk_Armor_M"),
        ("Armor_OC_EBR_Gomez_100", "Ore_Armor_M"),
        ("Armor_SC_VLK_Melvin_582", "Vlk_Armor_M"),
        ("Armor_SK_OC_WOC_Velaya_108_02", "Ore_Armor_M"),
        ("ItAr_Rune_FireBall_MiltenSleeper", "ItAr_Rune_FireBall"),
        (
            "ItAr_Rune_FireBall_MiltenSleeper_Player",
            "ItAr_Rune_FireBall",
        ),
        ("ItMw_1H_Sword_05_Darrion", "ItMw_1H_Sword_05"),
        ("ItMw_2H_Axe_Orc_01_vOrc_Sleeper", "ItMw_2H_Axe_Orc_01"),
        ("ItMw_2H_Axe_Orc_02_vOrc_Sleeper", "ItMw_2H_Axe_Orc_02"),
        ("ItMw_2H_Axe_Orc_03_vOrc_Sleeper", "ItMw_2H_Axe_Orc_03"),
        ("ItMw_2H_Axe_Orc_04_vOrc_Sleeper", "ItMw_2H_Axe_Orc_04"),
        ("ItMw_2H_Mace_Orc_01_vOrc_Sleeper", "ItMw_2H_Mace_Orc_01"),
        ("ItMw_2H_Staff_Orc_vOrc_Fireball", "ItMw_2H_Staff_Orc"),
        (
            "ItMw_2H_Staff_Unorc_vOrc_BallLighting",
            "ItAr_Rune_BallLightning",
        ),
        (
            "ItMw_2H_Staff_Unorc_vOrc_BreathOfDeath",
            "ItAr_Rune_BreathOfDeath",
        ),
        (
            "ItMw_2H_Staff_Unorc_vOrc_ChainLighting",
            "ItAr_Rune_ChainLightning",
        ),
        (
            "ItMw_2H_Staff_Unorc_vOrc_Pyrokinesis",
            "ItAr_Rune_Pyrokinesis",
        ),
        (
            "ItMw_2H_Staff_Unorc_vOrc_StormOfFire",
            "ItAr_Rune_StormOfFire",
        ),
        ("ItMw_2H_Sword_Light_01_Stone", "ItMw_2H_Sword_Light_01"),
        ("ItMw_2H_Sword_Orc_01_vOrc_Sleeper", "ItMw_2H_Sword_Orc_01"),
        ("QA_Armor", "Ryl_Armor"),
    ];

    for (id, icon) in expected {
        let resolution = resolve_item_icon_with_source(id);
        assert_eq!(resolution.icon, icon, "{id}");
        assert_eq!(
            resolution.evidence,
            ItemIconEvidence::InheritedScriptDefault,
            "{id}"
        );
        assert!(!resolution.uses_fallback, "{id}");
    }
}
