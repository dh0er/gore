use gore_core::catalog::{category_for_id, CatalogEntry, CatalogModel, ItemCategory};

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
fn catalog_model_lookup() {
    let mut model = CatalogModel::default();
    model.entries.push(CatalogEntry {
        id: "ItFo_Apple".to_string(),
        display_name: "Apple".to_string(),
        category: ItemCategory::Food,
    });
    assert!(model.find("ItFo_Apple").is_some());
    assert!(model.find("ItFo_Cheese").is_none());
}
