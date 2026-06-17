//! Item catalog model + prefix→category mapping.
//!
//! Mirrors the gore-save Flutter `item_categories.dart` logic so both the save
//! editor and the modding tools classify item ids identically. Prefix set
//! verified against the UE4SS object dump (see memory `gothic-remake-ue4ss-dump`).

use serde::{Deserialize, Serialize};

/// Item categories derived from the Angelscript class-name prefix
/// (e.g. `ItMi_Orenugget` -> Misc). Order matches the Dart enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemCategory {
    MeleeWeapon,
    RangedWeapon,
    Ammunition,
    Rune,
    Scroll,
    Food,
    Misc,
    Amulet,
    Ring,
    Trophy,
    Writing,
    Mission,
    Key,
    Other,
}

impl ItemCategory {
    /// Human-readable label (matches the Flutter labels).
    pub fn label(self) -> &'static str {
        match self {
            ItemCategory::MeleeWeapon => "Melee weapons",
            ItemCategory::RangedWeapon => "Ranged weapons",
            ItemCategory::Ammunition => "Ammunition",
            ItemCategory::Rune => "Runes",
            ItemCategory::Scroll => "Spell scrolls",
            ItemCategory::Food => "Food & potions",
            ItemCategory::Misc => "Miscellaneous",
            ItemCategory::Amulet => "Amulets",
            ItemCategory::Ring => "Rings",
            ItemCategory::Trophy => "Animal trophies",
            ItemCategory::Writing => "Writings",
            ItemCategory::Mission => "Mission items",
            ItemCategory::Key => "Keys",
            ItemCategory::Other => "Other",
        }
    }
}

/// Classify an item id by its Angelscript class-name prefix. The ordering of
/// checks is significant (`ItAm_`, `ItAr_Rune_`/`ItAr_Scroll_`, and the
/// `ItAt_Amulet_`/`ItAt_Ring_` specializations before the generic `ItAt_`).
pub fn item_category_from_id(id: &str) -> ItemCategory {
    if id.starts_with("ItMw_") {
        ItemCategory::MeleeWeapon
    } else if id.starts_with("ItRw_") {
        ItemCategory::RangedWeapon
    } else if id.starts_with("ItAm_") {
        // Ammunition (ItAm_Arrow/ItAm_Bolt); amulets live under ItAt_.
        ItemCategory::Ammunition
    } else if id.starts_with("ItAr_Rune_") {
        ItemCategory::Rune
    } else if id.starts_with("ItAr_Scroll_") {
        ItemCategory::Scroll
    } else if id.starts_with("ItFo_") {
        ItemCategory::Food
    } else if id.starts_with("ItMi_") {
        ItemCategory::Misc
    } else if id.starts_with("ItAt_Amulet_") {
        ItemCategory::Amulet
    } else if id.starts_with("ItAt_Ring_") {
        ItemCategory::Ring
    } else if id.starts_with("ItAt_") {
        ItemCategory::Trophy
    } else if id.starts_with("ItWr_") {
        ItemCategory::Writing
    } else if id.starts_with("ItMs_") {
        ItemCategory::Mission
    } else if id.starts_with("ItKe_")
        || id.starts_with("ItKey")
        || id.starts_with("ItChestKey")
        || id.starts_with("ItDoorKey")
    {
        ItemCategory::Key
    } else {
        ItemCategory::Other
    }
}

/// One entry in the generated **item** catalog JSON (`id` + `path` +
/// `category`). This shape is item-catalog-specific: the NPC catalog uses
/// `class` instead of `path` and the knowledge catalog omits `path`, so
/// [`parse_catalog`] must not be used on those — they are frontend-only assets
/// with no Rust consumer here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub path: String,
    pub category: String,
}

/// Parse the item catalog JSON string (array of [`CatalogEntry`]).
pub fn parse_catalog(json: &str) -> serde_json::Result<Vec<CatalogEntry>> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_prefixes() {
        assert_eq!(item_category_from_id("ItMw_1H_Sword_01"), ItemCategory::MeleeWeapon);
        assert_eq!(item_category_from_id("ItRw_Bow_Diego"), ItemCategory::RangedWeapon);
        assert_eq!(item_category_from_id("ItAm_Arrow"), ItemCategory::Ammunition);
        assert_eq!(item_category_from_id("ItAr_Rune_FireBall"), ItemCategory::Rune);
        assert_eq!(item_category_from_id("ItAr_Scroll_Charm"), ItemCategory::Scroll);
        assert_eq!(item_category_from_id("ItFo_Apple"), ItemCategory::Food);
        assert_eq!(item_category_from_id("ItMi_Orenugget"), ItemCategory::Misc);
        assert_eq!(item_category_from_id("ItAt_Amulet_OfDeath"), ItemCategory::Amulet);
        assert_eq!(item_category_from_id("ItAt_Ring_OfLife"), ItemCategory::Ring);
        assert_eq!(item_category_from_id("ItAt_Wolf_Fur"), ItemCategory::Trophy);
        assert_eq!(item_category_from_id("ItWr_Map"), ItemCategory::Writing);
        assert_eq!(item_category_from_id("ItMs_Ashes"), ItemCategory::Mission);
        assert_eq!(item_category_from_id("ItKe_Lockpick"), ItemCategory::Key);
        assert_eq!(item_category_from_id("ItKeyDefault"), ItemCategory::Key);
        assert_eq!(item_category_from_id("ItChestKey01"), ItemCategory::Key);
    }

    #[test]
    fn unknown_ids_map_to_other() {
        assert_eq!(item_category_from_id("Armor_OC_Gomez"), ItemCategory::Other);
        assert_eq!(item_category_from_id(""), ItemCategory::Other);
        assert_eq!(item_category_from_id("ItIg_Worldsplitter"), ItemCategory::Other);
    }

    #[test]
    fn parses_catalog_json() {
        let json = r#"[
            {"id":"ItMi_Orenugget","path":"/Script/Angelscript.ItMi_Orenugget","category":"misc"}
        ]"#;
        let entries = parse_catalog(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "ItMi_Orenugget");
    }
}
