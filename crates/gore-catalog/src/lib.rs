//! Item catalog model + prefix→category mapping.
//!
//! Mirrors the gore-save Flutter `item_categories.dart` logic so both the save
//! editor and the modding tools classify item ids identically. Prefix set
//! verified against the UE4SS object dump (see memory `gothic-remake-ue4ss-dump`).

pub mod location;
pub mod pipeline;
pub mod register;

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

/// Return the suffix of the shipped inventory icon associated with `id`.
///
/// Most item definitions and icons share the same suffix: the texture is
/// `/Game/UI/Textures/ItemIcons/T_ItemIcon_<id>`. The shipped game has a
/// smaller set of item-only variants, crafting intermediates, and legacy class
/// names without their own texture. Those resolve to the closest existing game
/// icon here so every generated catalog entry has a usable image.
///
/// The returned value intentionally excludes `T_ItemIcon_`. Two shipped assets
/// (`ItMi_Stuff_Brush` and `ItMi_Oldcoin_01`) spell that *prefix* as
/// `T_Itemicon_`; their suffix remains the unchanged item id and the texture
/// extractor is responsible for trying the two exact prefix spellings.
pub fn resolve_item_icon(id: &str) -> String {
    resolve_item_icon_with_source(id).icon
}

/// Which deterministic mapping rule [`resolve_item_icon`] used.
///
/// This is deliberately public so catalog audits can distinguish the 676
/// exact id matches from mechanical and hand-selected aliases without putting
/// that implementation metadata into the shipped JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemIconSource {
    Direct,
    SmithIntermediate,
    SmithWeapon,
    LegacySpellScroll,
    UrizielGem,
    StrongbeerEvent,
    VisualVariant,
    SemanticAlias,
}

/// One deterministic icon resolution plus its audit provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemIconResolution {
    pub icon: String,
    pub source: ItemIconSource,
    pub evidence: ItemIconEvidence,
    pub uses_fallback: bool,
}

/// Authority behind an item-icon choice.
///
/// The installed script cache is authoritative when it assigns an icon
/// directly or through an AngelScript parent. Empty and unresolved defaults
/// stay distinct so audits do not mistake a useful UI fallback for game data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemIconEvidence {
    ScriptDefault,
    InheritedScriptDefault,
    ExplicitEmptyFallback,
    UnresolvedFallback,
}

/// Resolve an item icon while retaining testable provenance.
pub fn resolve_item_icon_with_source(id: &str) -> ItemIconResolution {
    if let Some(icon) = semantic_item_icon_alias(id) {
        return item_icon_resolution(id, icon, ItemIconSource::SemanticAlias);
    }

    // Crafting definitions reuse the corresponding component or finished
    // weapon art; none of these intermediate ItMi_* classes has its own icon.
    if let Some(component) = id.strip_prefix("ItMi_Smith_Intermediate_") {
        return item_icon_resolution(
            id,
            format!("ItMi_Smith_{component}"),
            ItemIconSource::SmithIntermediate,
        );
    }
    if let Some(weapon) = id.strip_prefix("ItMi_Smith_") {
        if weapon.starts_with("1H_") || weapon.starts_with("2H_") {
            return item_icon_resolution(id, format!("ItMw_{weapon}"), ItemIconSource::SmithWeapon);
        }
    }

    // The script cache explicitly assigns the generic writing-scroll art to
    // these legacy spell classes. Ordinary ItWr_Scroll_* documents have their
    // own icons and therefore must not be rewritten.
    if let Some(spell) = id.strip_prefix("ItWr_Scroll_") {
        if LEGACY_SPELL_SCROLLS.contains(&spell) {
            return item_icon_resolution(
                id,
                "ItWr_Scroll_Generic",
                ItemIconSource::LegacySpellScroll,
            );
        }
    }

    if id.strip_prefix("ItMi_UrizielGem_").is_some_and(|variant| {
        !variant.is_empty() && variant.bytes().all(|byte| byte.is_ascii_digit())
    }) {
        return item_icon_resolution(id, "ItMs_UrizielGem", ItemIconSource::UrizielGem);
    }

    // The event-edition class adds two non-visual qualifiers to the normal
    // strong-beer item.
    if id == "ItFo_Potion_Strongbeer_Magic_EE" {
        return item_icon_resolution(
            id,
            "ItFo_Potion_Strongbeer",
            ItemIconSource::StrongbeerEvent,
        );
    }

    // Story/actor variants keep the base item's mesh and use its icon. Longest
    // suffixes come first so e.g. `_vOrc_Sleeper` is removed in one step.
    const VISUAL_VARIANT_SUFFIXES: &[&str] = &[
        "_MiltenSleeper_Player",
        "_MiltenSleeper",
        "_Xardas_Sleeper",
        "_Diego_Sleeper",
        "_vOrc_Sleeper",
        "_vOrc_Fireball",
        "_PlayerPlayTest",
        "_Darrion",
        "_Gob",
        "_Stone",
        "_Sleeper",
        "_vOrc",
    ];
    for suffix in VISUAL_VARIANT_SUFFIXES {
        if let Some(base) = id.strip_suffix(suffix) {
            return item_icon_resolution(id, base, ItemIconSource::VisualVariant);
        }
    }

    // Audited direct match: 676 of the 831 catalog entries use their exact id.
    item_icon_resolution(id, id, ItemIconSource::Direct)
}

fn item_icon_resolution(
    id: &str,
    icon: impl Into<String>,
    source: ItemIconSource,
) -> ItemIconResolution {
    ItemIconResolution {
        icon: icon.into(),
        source,
        evidence: item_icon_evidence(id),
        uses_fallback: item_icon_uses_fallback(id),
    }
}

fn item_icon_evidence(id: &str) -> ItemIconEvidence {
    if EXPLICIT_EMPTY_ITEM_ICONS.contains(&id) {
        return ItemIconEvidence::ExplicitEmptyFallback;
    }
    if UNRESOLVED_ITEM_ICONS.contains(&id) {
        return ItemIconEvidence::UnresolvedFallback;
    }
    if INHERITED_ITEM_ICONS.contains(&id) {
        return ItemIconEvidence::InheritedScriptDefault;
    }
    ItemIconEvidence::ScriptDefault
}

fn item_icon_uses_fallback(id: &str) -> bool {
    MISSING_DECLARED_ICON_ASSETS.contains(&id)
        || EXPLICIT_EMPTY_ITEM_ICONS.contains(&id)
        || UNRESOLVED_ITEM_ICONS.contains(&id)
}

// The script cache assigns these six suffixes, but those assets are absent
// from the installed texture index. Their nearest usable icon is intentional.
const MISSING_DECLARED_ICON_ASSETS: &[&str] = &[
    "ItMw_2H_Scepter_Skeletonmage",
    "ItMw_2H_Staff_Unorc",
    "ItMw_2H_Staff_Unorc_vOrc",
    "ItMw_2H_Sword_Beliar",
    "ItWr_Scroll_Orkparcment_01",
    "ItWr_Scroll_Orkparcment_02",
];

// These four recipe definitions explicitly assign `m_Icon = ""`.
const EXPLICIT_EMPTY_ITEM_ICONS: &[&str] = &[
    "ItMw_Smith_IntermediateSword_01",
    "ItMw_Smith_IntermediateSword_02",
    "ItMw_Smith_IntermediateSword_03",
    "ItMw_Smith_IntermediateSword_04",
];

// No `m_Icon` assignment exists in the class or its AngelScript parents. The
// two Sunken Tower ids still have exact-name textures, which are safe self
// fallbacks; the other seven use the hand-selected aliases below.
const UNRESOLVED_ITEM_ICONS: &[&str] = &[
    "Armor_BC_BAN_Arlin_852_02",
    "ItChestKey01",
    "ItDoorKey01",
    "ItFocusStoneBridgeItem",
    "ItIg_Worldsplitter",
    "ItKeyDefault",
    "ItMi_Meta_Pouch",
    "ItMs_SunkenTowerStone_01",
    "ItMs_SunkenTowerStone_02",
];

// These classes obtain their icon from the first AngelScript parent which
// assigns `m_Icon`; no visual guess is involved.
const INHERITED_ITEM_ICONS: &[&str] = &[
    "Armor_BC_BAN_Arlin_852",
    "Armor_OCR_GRD_Stone_219",
    "Armor_OCR_GRD_Stone_219_03",
    "Armor_OC_EBR_Gomez_100",
    "Armor_SC_VLK_Melvin_582",
    "Armor_SK_OC_WOC_Velaya_108_02",
    "ItAr_Rune_FireBall_MiltenSleeper",
    "ItAr_Rune_FireBall_MiltenSleeper_Player",
    "ItMw_1H_Sword_05_Darrion",
    "ItMw_2H_Axe_Orc_01_vOrc_Sleeper",
    "ItMw_2H_Axe_Orc_02_vOrc_Sleeper",
    "ItMw_2H_Axe_Orc_03_vOrc_Sleeper",
    "ItMw_2H_Axe_Orc_04_vOrc_Sleeper",
    "ItMw_2H_Mace_Orc_01_vOrc_Sleeper",
    "ItMw_2H_Staff_Orc_vOrc_Fireball",
    "ItMw_2H_Staff_Unorc_vOrc_BallLighting",
    "ItMw_2H_Staff_Unorc_vOrc_BreathOfDeath",
    "ItMw_2H_Staff_Unorc_vOrc_ChainLighting",
    "ItMw_2H_Staff_Unorc_vOrc_Pyrokinesis",
    "ItMw_2H_Staff_Unorc_vOrc_StormOfFire",
    "ItMw_2H_Sword_Light_01_Stone",
    "ItMw_2H_Sword_Orc_01_vOrc_Sleeper",
    "QA_Armor",
];

const LEGACY_SPELL_SCROLLS: &[&str] = &[
    "BallLightning",
    "FireBolt",
    "FireRain",
    "FistOfWind",
    "Heal",
    "IceBolt",
    "IceWave",
    "Pyrokinesis",
    "StormFist",
    "StormOfFire",
    "Telekinesis",
    "TransformBiter",
    "TransformBloodfly",
    "TransformBloodhound",
    "TransformHarpy",
    "TransformLizard",
    "TransformLurker",
    "TransformMinecrawler",
    "TransformOrcDog",
    "TransformRazor",
    "TransformShadowbeast",
    "TransformSnapper",
    "TransformSwampshark",
];

/// Cache-audited and fallback aliases whose relationship is not a safe general
/// spelling transformation.
fn semantic_item_icon_alias(id: &str) -> Option<&'static str> {
    match id {
        // Per-character and test armors reuse the matching faction garment.
        "Armor_BC_BAN_Arlin_852" => Some("Vlk_Armor_M"),
        // This variant inherits directly from the icon-less armor base. Keep
        // the independently audited faction fallback rather than pretending
        // that the script cache declares an icon for it.
        "Armor_BC_BAN_Arlin_852_02" => Some("Org_Armor"),
        "Armor_OCR_GRD_Stone_219" | "Armor_OCR_GRD_Stone_219_03" => Some("Vlk_Armor_M"),
        "Armor_OC_EBR_Gomez_100" | "Armor_SK_OC_WOC_Velaya_108_02" => Some("Ore_Armor_M"),
        "Armor_SC_VLK_Melvin_582" => Some("Vlk_Armor_M"),
        "QA_Armor" => Some("Ryl_Armor"),

        // Legacy/generic classes without a dedicated icon.
        "ItChestKey01" | "ItDoorKey01" | "ItKeyDefault" => Some("ItKe_Dungeonkey"),
        "ItFocusStoneBridgeItem" => Some("ItMs_Focus_05"),
        "ItIg_Worldsplitter" => Some("ItMs_Worldsplitter"),
        "ItMi_Meta_Pouch" => Some("ItMi_Oldcoin_01"),
        "ItMs_FakeDiggerClothes" => Some("Vlk_Armor_L"),
        "ItMs_Scroll_LoveInterest_2" | "ItMs_Scroll_WomensRevenge" => Some("ItWr_Scroll_Generic"),

        // Debug/special weapons use the nearest shipped weapon silhouette.
        "ItMw_1H_Sword_QA" => Some("ItMw_1H_Sword_Bastard_04"),
        "ItMw_2H_Scepter_Skeletonmage" => Some("ItMw_2H_Staff_Scepter"),
        "ItMw_2H_Sword_Beliar" => Some("ItMw_2H_Sword_Innos"),
        "ItRw_Bow_Diego_Sleeper" => Some("ItRw_Bow_Diego"),
        "ItRw_Bow_QA" => Some("ItRw_Bow_War_05"),
        "ItMw_2H_Staff_Unorc_GVArushat_vOrc" | "ItMw_2H_Staff_Unorc_vOrc_BreathOfDeath" => {
            Some("ItAr_Rune_BreathOfDeath")
        }
        "ItMw_2H_Staff_Unorc_VHashor_vOrc" | "ItMw_2H_Staff_Unorc_vOrc_Pyrokinesis" => {
            Some("ItAr_Rune_Pyrokinesis")
        }
        "ItMw_2H_Staff_Unorc_VKasorg_vOrc" | "ItMw_2H_Staff_Unorc_vOrc_BallLighting" => {
            Some("ItAr_Rune_BallLightning")
        }
        "ItMw_2H_Staff_Unorc_VRuuushk_vOrc" | "ItMw_2H_Staff_Unorc_vOrc_StormOfFire" => {
            Some("ItAr_Rune_StormOfFire")
        }
        "ItMw_2H_Staff_Unorc_VUnhilqt_vOrc" | "ItMw_2H_Staff_Unorc_vOrc_ChainLighting" => {
            Some("ItAr_Rune_ChainLightning")
        }
        id if id.starts_with("ItMw_2H_Staff_Unorc") => Some("ItMw_2H_Staff_Orc"),

        // The four old smithing weapon states correspond to the normal
        // material states used by the crafting UI.
        "ItMw_Smith_IntermediateSword_01" => Some("ItMi_Smith_Swordraw"),
        "ItMw_Smith_IntermediateSword_02" => Some("ItMi_Smith_Swordrawhot"),
        "ItMw_Smith_IntermediateSword_03" => Some("ItMi_Smith_Swordbladehot"),
        "ItMw_Smith_IntermediateSword_04" => Some("ItMi_Smith_Swordblade"),

        // Maps and handwritten story documents use the closest matching art.
        "ItWr_Map_Lighthouse" => Some("ItWr_Map_World"),
        "ItWr_Map_OldMineSC" => Some("ItWr_Map_OldMine"),
        "ItWr_Scroll_Cronos" => Some("ItWr_Scroll_Generic"),
        "ItWr_Scroll_Orkparcment_01" | "ItWr_Scroll_Orkparcment_02" => Some("ItWr_Scroll_Generic"),
        "ItWr_Scroll_RankWaterMages" => Some("ItWr_Scroll_Generic"),
        _ => None,
    }
}

/// One entry in a generated catalog JSON file produced by `gore-cli catalog`
/// (item pipeline). Shape: category + icon + id + path. `icon` is defaulted for
/// compatibility with older user-supplied catalogs; newly generated catalogs
/// always contain it. This is the on-disk catalog shape; there is no separate
/// in-memory catalog wrapper type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogJsonEntry {
    pub category: String,
    #[serde(default)]
    pub icon: String,
    pub id: String,
    pub path: String,
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
        assert!(entries[0].icon.is_empty());
    }

    #[test]
    fn item_icon_resolver_keeps_exact_ids_and_maps_known_aliases() {
        assert_eq!(resolve_item_icon("ItMi_Orenugget"), "ItMi_Orenugget");
        // Only the texture prefix uses the lower-case `T_Itemicon_` spelling.
        assert_eq!(resolve_item_icon("ItMi_Stuff_Brush"), "ItMi_Stuff_Brush");
        assert_eq!(resolve_item_icon("ItMi_Oldcoin_01"), "ItMi_Oldcoin_01");

        assert_eq!(
            resolve_item_icon("ItMi_Smith_Intermediate_Blade_Long"),
            "ItMi_Smith_Blade_Long"
        );
        assert_eq!(
            resolve_item_icon("ItMi_Smith_1H_Sword_03"),
            "ItMw_1H_Sword_03"
        );
        assert_eq!(
            resolve_item_icon("ItWr_Scroll_TransformHarpy"),
            "ItWr_Scroll_Generic"
        );
        assert_eq!(
            resolve_item_icon("ItWr_Scroll_Generic"),
            "ItWr_Scroll_Generic"
        );
        assert_eq!(resolve_item_icon("ItMi_UrizielGem_05"), "ItMs_UrizielGem");
        assert_eq!(
            resolve_item_icon("ItFo_Potion_Strongbeer_Magic_EE"),
            "ItFo_Potion_Strongbeer"
        );
        assert_eq!(
            resolve_item_icon("ItMw_2H_Axe_Orc_03_vOrc_Sleeper"),
            "ItMw_2H_Axe_Orc_03"
        );
        assert_eq!(
            resolve_item_icon("ItMw_2H_Staff_Unorc_vOrc_StormOfFire"),
            "ItAr_Rune_StormOfFire"
        );
        assert_eq!(
            resolve_item_icon("ItRw_Bow_Diego_Sleeper"),
            "ItRw_Bow_Diego"
        );
        assert_eq!(resolve_item_icon("QA_Armor"), "Ryl_Armor");
        assert_eq!(
            resolve_item_icon("ItMw_2H_Staff_Unorc_VKasorg_vOrc"),
            "ItAr_Rune_BallLightning"
        );
        assert_eq!(
            resolve_item_icon_with_source("ItMi_Stuff_Brush").source,
            ItemIconSource::Direct
        );
        assert_eq!(
            resolve_item_icon_with_source("ItMi_Smith_1H_Sword_03").source,
            ItemIconSource::SmithWeapon
        );
        assert_eq!(
            resolve_item_icon_with_source("ItMw_2H_Staff_Unorc").source,
            ItemIconSource::SemanticAlias
        );
    }
}
