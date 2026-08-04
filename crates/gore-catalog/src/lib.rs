//! Item catalog model + prefix→category mapping.
//!
//! Mirrors the gore-save Flutter `item_categories.dart` logic so both the save
//! editor and the modding tools classify item ids identically. Prefix set
//! verified against the UE4SS object dump (see memory `gothic-remake-ue4ss-dump`).

pub mod location;
pub mod pipeline;

use serde::{Deserialize, Serialize};

/// Item categories derived from the Angelscript class-name prefix
/// (e.g. `ItMi_Orenugget` -> Misc). Order matches the Dart enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemCategory {
    MeleeWeapon,
    RangedWeapon,
    /// Wearable armor (`<Fac>_Armor_*`, `Armor_<Camp>_<NPC>_*`).
    Armor,
    Ammunition,
    Rune,
    Scroll,
    /// Combined rune+scroll category (used by gore-cli category_for_id).
    RuneOrScroll,
    Food,
    Misc,
    Amulet,
    Ring,
    /// Combined amulet+ring category (used by gore-cli category_for_id).
    Jewelry,
    Trophy,
    Writing,
    /// Writing/document category alias (used by gore-cli category_for_id).
    Document,
    Mission,
    Key,
    /// Potions and liquid substances (ItPo_*, ItLs_*).
    Potion,
    Other,
    /// Unknown prefix — not in the known prefix table.
    Unknown,
}

impl ItemCategory {
    /// Human-readable label (matches the Flutter labels).
    pub fn label(self) -> &'static str {
        match self {
            ItemCategory::MeleeWeapon => "Melee weapons",
            ItemCategory::RangedWeapon => "Ranged weapons",
            ItemCategory::Armor => "Armor",
            ItemCategory::Ammunition => "Ammunition",
            ItemCategory::Rune => "Runes",
            ItemCategory::Scroll => "Spell scrolls",
            ItemCategory::RuneOrScroll => "Runes & scrolls",
            ItemCategory::Food => "Food & potions",
            ItemCategory::Misc => "Miscellaneous",
            ItemCategory::Amulet => "Amulets",
            ItemCategory::Ring => "Rings",
            ItemCategory::Jewelry => "Jewelry",
            ItemCategory::Trophy => "Animal trophies",
            ItemCategory::Writing => "Writings",
            ItemCategory::Document => "Documents",
            ItemCategory::Mission => "Mission items",
            ItemCategory::Key => "Keys",
            ItemCategory::Potion => "Potions",
            ItemCategory::Other => "Other",
            ItemCategory::Unknown => "Unknown",
        }
    }
}

/// Display-side armor classifier: true for any armor class name (base, per-NPC,
/// or tier piece). Broader than the catalog generator's `is_armor_item_class`,
/// which additionally excludes tier pieces from the *addable* set.
pub fn is_armor_id(id: &str) -> bool {
    if !id.contains("Armor") {
        return false;
    }
    if id.starts_with("Armor_") {
        return true;
    }
    let mut parts = id.splitn(2, '_');
    let head = parts.next().unwrap_or("");
    let tail = parts.next().unwrap_or("");
    // Tail must be exactly `Armor` or start with `Armor_`; `starts_with("Armor")`
    // alone would also match an `Armory` segment (e.g. `NC_Armory_Door`).
    (2..=4).contains(&head.len())
        && head.chars().all(|c| c.is_ascii_alphabetic())
        && (tail == "Armor" || tail.starts_with("Armor_"))
}

/// Classify an item id by its Angelscript class-name prefix. The ordering of
/// checks is significant (`ItAm_`, `ItAr_Rune_`/`ItAr_Scroll_`, and the
/// `ItAt_Amulet_`/`ItAt_Ring_` specializations before the generic `ItAt_`).
///
/// This is the original fine-grained classification used by gore-save.
pub fn item_category_from_id(id: &str) -> ItemCategory {
    if is_armor_id(id) {
        ItemCategory::Armor
    } else if id.starts_with("ItMw_") {
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

/// Classify an item id using the gore-cli coarser category set.
///
/// Uses combined categories (RuneOrScroll, Jewelry, Potion) suited to the
/// modding toolchain. Returns `ItemCategory::Unknown` for unrecognized prefixes.
pub fn category_for_id(id: &str) -> ItemCategory {
    if is_armor_id(id) {
        return ItemCategory::Armor;
    }
    if id.starts_with("ItFo_") {
        return ItemCategory::Food;
    }
    if id.starts_with("ItMw_") {
        return ItemCategory::MeleeWeapon;
    }
    if id.starts_with("ItRw_") {
        return ItemCategory::RangedWeapon;
    }
    if id.starts_with("ItAm_") {
        return ItemCategory::Ammunition;
    }
    if id.starts_with("ItAt_Amulet_") || id.starts_with("ItAt_Ring_") {
        return ItemCategory::Jewelry;
    }
    if id.starts_with("ItAt_") {
        return ItemCategory::Trophy;
    }
    if id.starts_with("ItAr_") {
        return ItemCategory::RuneOrScroll;
    }
    if id.starts_with("ItWr_") {
        return ItemCategory::Document;
    }
    if id.starts_with("ItMs_") {
        return ItemCategory::Mission;
    }
    if id.starts_with("ItKe_")
        || id.starts_with("ItKey")
        || id.starts_with("ItChestKey")
        || id.starts_with("ItDoorKey")
    {
        return ItemCategory::Key;
    }
    if id.starts_with("ItPo_") || id.starts_with("ItLs_") {
        return ItemCategory::Potion;
    }
    if id.starts_with("ItMi_") {
        return ItemCategory::Misc;
    }
    ItemCategory::Unknown
}

/// One entry in a generated catalog JSON file produced by `gore-cli catalog`
/// (item pipeline). Shape: id + path + category string (matches the historical
/// `build_item_catalog.py` output, which `gore-cli catalog --kind item`
/// reproduces byte-for-byte). This is the on-disk catalog shape; there is no
/// separate in-memory catalog wrapper type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogJsonEntry {
    pub id: String,
    pub path: String,
    pub category: String,
}

/// Parse a catalog JSON string (array of [`CatalogJsonEntry`]).
pub fn parse_catalog(json: &str) -> serde_json::Result<Vec<CatalogJsonEntry>> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_prefixes() {
        assert_eq!(
            item_category_from_id("ItMw_1H_Sword_01"),
            ItemCategory::MeleeWeapon
        );
        assert_eq!(
            item_category_from_id("ItRw_Bow_Diego"),
            ItemCategory::RangedWeapon
        );
        assert_eq!(
            item_category_from_id("ItAm_Arrow"),
            ItemCategory::Ammunition
        );
        assert_eq!(
            item_category_from_id("ItAr_Rune_FireBall"),
            ItemCategory::Rune
        );
        assert_eq!(
            item_category_from_id("ItAr_Scroll_Charm"),
            ItemCategory::Scroll
        );
        assert_eq!(item_category_from_id("ItFo_Apple"), ItemCategory::Food);
        assert_eq!(item_category_from_id("ItMi_Orenugget"), ItemCategory::Misc);
        assert_eq!(
            item_category_from_id("ItAt_Amulet_OfDeath"),
            ItemCategory::Amulet
        );
        assert_eq!(
            item_category_from_id("ItAt_Ring_OfLife"),
            ItemCategory::Ring
        );
        assert_eq!(item_category_from_id("ItAt_Wolf_Fur"), ItemCategory::Trophy);
        assert_eq!(item_category_from_id("ItWr_Map"), ItemCategory::Writing);
        assert_eq!(item_category_from_id("ItMs_Ashes"), ItemCategory::Mission);
        assert_eq!(item_category_from_id("ItKe_Lockpick"), ItemCategory::Key);
        assert_eq!(item_category_from_id("ItKeyDefault"), ItemCategory::Key);
        assert_eq!(item_category_from_id("ItChestKey01"), ItemCategory::Key);
    }

    #[test]
    fn unknown_ids_map_to_other() {
        assert_eq!(item_category_from_id(""), ItemCategory::Other);
        assert_eq!(
            item_category_from_id("ItIg_Worldsplitter"),
            ItemCategory::Other
        );
    }

    #[test]
    fn armor_ids_map_to_armor() {
        assert_eq!(item_category_from_id("Ore_Armor_H"), ItemCategory::Armor);
        assert_eq!(item_category_from_id("Org_Armor"), ItemCategory::Armor);
        assert_eq!(item_category_from_id("Armor_OC_Gomez"), ItemCategory::Armor);
        assert_eq!(category_for_id("Ore_Armor_H"), ItemCategory::Armor);
        assert_eq!(category_for_id("Armor_OC_Gomez"), ItemCategory::Armor);
        // An "Armory" segment is not armor.
        assert!(!is_armor_id("NC_Armory_Door"));
        assert_eq!(item_category_from_id("NC_Armory_Door"), ItemCategory::Other);
        assert_eq!(category_for_id("NC_Armory_Door"), ItemCategory::Unknown);
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
