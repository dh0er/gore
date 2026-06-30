use gore_catalog::{category_for_id, ItemCategory};

#[test]
fn category_food() {
    assert_eq!(category_for_id("ItFo_Apple"), ItemCategory::Food);
}

#[test]
fn category_weapon_melee() {
    assert_eq!(category_for_id("ItMw_1H_Sword_01"), ItemCategory::MeleeWeapon);
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
    assert_eq!(category_for_id("ItAr_Rune_Fireball"), ItemCategory::RuneOrScroll);
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
    assert_eq!(category_for_id("TotallyUnknownClass"), ItemCategory::Unknown);
}
