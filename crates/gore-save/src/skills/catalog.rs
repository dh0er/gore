//! Authoritative catalog of the Gothic 1 Remake hero's learnable skills.
//!
//! A fresh save only contains the handful of skills the hero has already
//! learned (as `GE_Skill_*` GameplayEffect classes in the hero's ActiveEffects
//! array), so the editor cannot discover the rest by scanning the save. This
//! catalog records every skill the game defines, letting the editor offer (and
//! synthesize) any skill even on an empty hero.
//!
//! Ported from the reference editor's `catalog.py`
//! (github.com/Xetoxyc/gothic-remake-savegame-editor). SOURCE OF TRUTH: the
//! `GE_Skill_*` classes observed in real savegames. When a save shows a class
//! not listed here, ADD IT.

/// Prefix shared by every hero-skill GameplayEffect class reference stored in a
/// save. The full value is `GE_PREFIX + base (+ "_" + suffix)`.
pub const GE_PREFIX: &str = "/Script/Angelscript.Default__GE_Skill_";

/// How a skill's GE class name is built and how its tiers are offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Ranked: class = `GE_Skill_<base>_<tier>` (e.g. `Melee_OneHanded_Master`).
    Ladder,
    /// Magic circle ladder (Amateur, 1..6): `GE_Skill_Mage_Circle_<tier>`.
    Circle,
    /// Single learned state "Trained": `GE_Skill_<base>_Trained`.
    Hunting,
    /// On/off skill with NO tier suffix: `GE_Skill_<base>`.
    Binary,
    /// Fixed single-suffix skill (currently unused — Orcish became a ladder).
    Language,
}

/// One catalogued skill.
#[derive(Clone, Copy, Debug)]
pub struct SkillDef {
    /// Base name inside the GE class (e.g. `Melee_OneHanded`, `Mage_Circle`).
    pub base: &'static str,
    /// Human-readable name shown in the UI.
    pub label: &'static str,
    /// UI grouping (Combat/Thievery/Hunting/Movement/Crafting/Magic/Language).
    pub category: &'static str,
    pub kind: Kind,
    /// Ordered learnable tiers ABOVE Untrained (ladder/circle only; else empty).
    pub ladder: &'static [&'static str],
    /// True when an `_Untrained` GE class exists, so lowering to Untrained is a
    /// rename rather than an unlearn (array element delete).
    pub has_untrained: bool,
    /// Optional `(tier, hint)` extra text shown next to a tier (e.g.
    /// Blacksmithing Trained -> "1H weapons").
    pub tier_labels: &'static [(&'static str, &'static str)],
}

/// Category display order used when sorting skills for the UI.
pub const CATEGORY_ORDER: &[&str] = &[
    "Combat",
    "Crafting",
    "Hunting",
    "Language",
    "Magic",
    "Movement",
    "Thievery",
];

const NO_TIERS: &[&str] = &[];
const NO_HINTS: &[(&str, &str)] = &[];
const RANK2: &[&str] = &["Trained", "Master"];
const THIEF: &[&str] = &["Skilled", "Master"];
const CIRCLE: &[&str] = &["Amateur", "1", "2", "3", "4", "5", "6"];

/// The complete skill roster. Keep grouped by category for readability.
pub const SKILLS: &[SkillDef] = &[
    // ---- Combat (ranked) -------------------------------------------------
    // All melee ladders have an `_Untrained` baseline class (confirmed in the
    // UE4SS object dump), so lowering to it is a retarget, not an unlearn.
    SkillDef { base: "Melee_OneHanded", label: "One-Handed", category: "Combat", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: NO_HINTS },
    SkillDef { base: "Melee_TwoHanded", label: "Two-Handed", category: "Combat", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: NO_HINTS },
    SkillDef { base: "Melee_Fists", label: "Fists", category: "Combat", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: NO_HINTS },
    SkillDef { base: "Melee_Orc", label: "Orc Weapons", category: "Combat", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: NO_HINTS },
    SkillDef { base: "Ranged_Bow", label: "Bow", category: "Combat", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: NO_HINTS },
    SkillDef { base: "Ranged_Crossbow", label: "Crossbow", category: "Combat", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: NO_HINTS },
    // ---- Thievery (ranked) ----------------------------------------------
    SkillDef { base: "Picklock", label: "Lockpicking", category: "Thievery", kind: Kind::Ladder, ladder: THIEF, has_untrained: true, tier_labels: NO_HINTS },
    SkillDef { base: "Pickpocket", label: "Pickpocketing", category: "Thievery", kind: Kind::Ladder, ladder: THIEF, has_untrained: true, tier_labels: NO_HINTS },
    // ---- Hunting (single "Trained" state) -------------------------------
    SkillDef { base: "Hunting_Organ", label: "Take Organs", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Teeth", label: "Break Teeth", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Claw", label: "Take Claws", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Fur", label: "Skin Fur", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Skin", label: "Skin", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Fins", label: "Take Fins", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Stings", label: "Take Stingers", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Secretion", label: "Take Secretion", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_SkullArmor", label: "Take Skull Plates", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_SkinSwampshark", label: "Skin Swampshark", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_MCPlate", label: "Take Minecrawler Plates", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Scutes", label: "Take Scutes", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_UluMulu", label: "Take Ulu-Mulu Trophies", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_MandibleMineCrawler", label: "Take Minecrawler Mandibles", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Reptiles", label: "Skin Reptiles", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_ShadowbeastHorn", label: "Take Shadowbeast Horn", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_Spines", label: "Take Spines", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_StingsBloodfly", label: "Take Bloodfly Stingers", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_TeethSwampshark", label: "Break Swampshark Teeth", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_TongueOfFire", label: "Take Fire Tongue", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Hunting_TrollHorn", label: "Take Troll Horn", category: "Hunting", kind: Kind::Hunting, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    // ---- Movement / utility (binary) ------------------------------------
    SkillDef { base: "Acrobatics", label: "Acrobatics", category: "Movement", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Wallclimbing", label: "Wall Climbing", category: "Movement", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Riding", label: "Riding", category: "Movement", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Sneak", label: "Sneaking", category: "Movement", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Diving", label: "Diving", category: "Movement", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Scavenging", label: "Scavenging", category: "Movement", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    // ---- Crafting -------------------------------------------------------
    SkillDef { base: "Crafting_Alchemy", label: "Alchemy", category: "Crafting", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    SkillDef { base: "Crafting_Inscription", label: "Rune Inscription", category: "Crafting", kind: Kind::Binary, ladder: NO_TIERS, has_untrained: false, tier_labels: NO_HINTS },
    // Blacksmithing is ranked: Trained (II) forges 1H weapons, Master (III) 2H;
    // an `_Untrained` baseline class exists (UE4SS dump), so Beginner retargets.
    SkillDef { base: "Crafting_Blacksmith", label: "Blacksmithing", category: "Crafting", kind: Kind::Ladder, ladder: RANK2, has_untrained: true, tier_labels: &[("Trained", "1H weapons"), ("Master", "2H weapons")] },
    // Mining (Schürfen): ranked Untrained/Skilled/Master, like the thievery
    // skills; an `_Untrained` baseline class exists.
    SkillDef { base: "Mining", label: "Mining", category: "Crafting", kind: Kind::Ladder, ladder: THIEF, has_untrained: true, tier_labels: NO_HINTS },
    // ---- Magic ----------------------------------------------------------
    // Mage_Circle has an `_Untrained` baseline class, so Circle 0 (Amateur) is a
    // learn and the lowest state retargets rather than unlearns.
    SkillDef { base: "Mage_Circle", label: "Magic Circle", category: "Magic", kind: Kind::Circle, ladder: CIRCLE, has_untrained: true, tier_labels: NO_HINTS },
    // ---- Language -------------------------------------------------------
    // Orcish is ranked Untrained/Skilled/Master (all three classes confirmed in
    // the UE4SS dump), with an `_Untrained` baseline.
    SkillDef { base: "Orcish", label: "Orcish Language", category: "Language", kind: Kind::Ladder, ladder: THIEF, has_untrained: true, tier_labels: NO_HINTS },
];

/// Tier suffixes that mark the END of a `GE_Skill_<base>_<tier>` class name, so
/// the base can be split off. Mirrors the reference editor's `_KNOWN_TIERS`.
const KNOWN_TIERS: &[&str] = &[
    "Untrained",
    "Trained",
    "Master",
    "Amateur",
    "Apprentice",
    "Skilled",
    "Journeyman",
    "Adept",
    "Expert",
];

/// The UI value that represents the freshly-learned state for a non-ladder
/// skill (what the "Learn" action selects).
pub fn learn_value(kind: Kind) -> &'static str {
    match kind {
        Kind::Hunting => "Trained",
        Kind::Binary => "Learned",
        Kind::Language => "Learned",
        // ladder/circle learn straight into a tier value; not used here.
        Kind::Ladder | Kind::Circle => "Trained",
    }
}

/// The GE class suffix a non-ladder skill's learned state maps to. Decoupled
/// from [`learn_value`] because Orcish is learned as the `_Untrained` class but
/// the UI does not label it "Untrained".
fn learn_suffix(kind: Kind) -> &'static str {
    match kind {
        Kind::Hunting => "Trained",
        Kind::Binary => "",
        Kind::Language => "Untrained",
        Kind::Ladder | Kind::Circle => "",
    }
}

/// Look up a skill by its base name.
pub fn find(base: &str) -> Option<&'static SkillDef> {
    SKILLS.iter().find(|s| s.base == base)
}

/// Every tier value a skill accepts (the `value`s of its UI options, learned or
/// roster): `Untrained` plus each ladder rung for ladder/circle skills, or
/// `Untrained` plus the learn value for on/off (hunting/binary/language) skills.
/// Used to validate a `private.skills.set` before it composes a GE class.
pub fn valid_tiers(def: &SkillDef) -> Vec<&'static str> {
    match def.kind {
        Kind::Ladder | Kind::Circle => {
            let mut tiers = Vec::with_capacity(def.ladder.len() + 1);
            tiers.push("Untrained");
            tiers.extend_from_slice(def.ladder);
            tiers
        }
        other => vec!["Untrained", learn_value(other)],
    }
}

/// Full GE class path for a `(base, chosen value)`. For ladder/circle skills the
/// value IS the tier suffix; for hunting/binary/language the suffix is fixed.
pub fn skill_class(base: &str, value: &str) -> String {
    let kind = find(base).map(|s| s.kind).unwrap_or(Kind::Ladder);
    let suffix = match kind {
        Kind::Ladder | Kind::Circle => value,
        other => learn_suffix(other),
    };
    if suffix.is_empty() {
        format!("{GE_PREFIX}{base}")
    } else {
        format!("{GE_PREFIX}{base}_{suffix}")
    }
}

/// Split a full GE class string into `(base, tier)`. `tier` is `None` for a
/// suffix-less binary skill. Returns `None` when the string is not a
/// `GE_Skill_*` reference.
///
/// Mirrors the reference editor's `_skill_split`: the numeric `Mage_Circle`
/// tiers (`_1`..`_6`) are not in [`KNOWN_TIERS`], so that base is special-cased.
pub fn split_class(full: &str) -> Option<(String, Option<String>)> {
    let raw = full.strip_prefix(GE_PREFIX)?;
    // Mage_Circle_<Amateur|1..6>: the tier can be numeric, so match the base.
    if let Some(tier) = raw.strip_prefix("Mage_Circle_") {
        return Some(("Mage_Circle".to_string(), Some(tier.to_string())));
    }
    if raw == "Mage_Circle" {
        return Some(("Mage_Circle".to_string(), None));
    }
    if let Some((base, tier)) = raw.rsplit_once('_') {
        if KNOWN_TIERS.contains(&tier) {
            return Some((base.to_string(), Some(tier.to_string())));
        }
    }
    Some((raw.to_string(), None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_ladder_class() {
        assert_eq!(
            split_class("/Script/Angelscript.Default__GE_Skill_Melee_OneHanded_Master"),
            Some(("Melee_OneHanded".to_string(), Some("Master".to_string())))
        );
    }

    #[test]
    fn split_binary_class_has_no_tier() {
        assert_eq!(
            split_class("/Script/Angelscript.Default__GE_Skill_Sneak"),
            Some(("Sneak".to_string(), None))
        );
    }

    #[test]
    fn split_hunting_class() {
        assert_eq!(
            split_class("/Script/Angelscript.Default__GE_Skill_Hunting_Fur_Trained"),
            Some(("Hunting_Fur".to_string(), Some("Trained".to_string())))
        );
    }

    #[test]
    fn split_circle_numeric_tier() {
        assert_eq!(
            split_class("/Script/Angelscript.Default__GE_Skill_Mage_Circle_6"),
            Some(("Mage_Circle".to_string(), Some("6".to_string())))
        );
        assert_eq!(
            split_class("/Script/Angelscript.Default__GE_Skill_Mage_Circle_Amateur"),
            Some(("Mage_Circle".to_string(), Some("Amateur".to_string())))
        );
    }

    #[test]
    fn split_rejects_non_skill() {
        assert_eq!(split_class("/Script/Foo.Bar"), None);
    }

    #[test]
    fn skill_class_roundtrips_ladder() {
        assert_eq!(
            skill_class("Melee_OneHanded", "Master"),
            "/Script/Angelscript.Default__GE_Skill_Melee_OneHanded_Master"
        );
    }

    #[test]
    fn skill_class_binary_has_no_suffix() {
        assert_eq!(
            skill_class("Sneak", "Learned"),
            "/Script/Angelscript.Default__GE_Skill_Sneak"
        );
    }

    #[test]
    fn skill_class_hunting_fixed_suffix() {
        assert_eq!(
            skill_class("Hunting_Fur", "Trained"),
            "/Script/Angelscript.Default__GE_Skill_Hunting_Fur_Trained"
        );
    }

    #[test]
    fn skill_class_language_learns_as_untrained() {
        // Orcish is a ladder now, but the language-kind suffix mapping is kept.
        assert_eq!(learn_suffix(Kind::Language), "Untrained");
    }

    #[test]
    fn every_skill_base_is_unique() {
        let mut bases: Vec<&str> = SKILLS.iter().map(|s| s.base).collect();
        bases.sort_unstable();
        let mut deduped = bases.clone();
        deduped.dedup();
        assert_eq!(bases, deduped, "duplicate skill base in catalog");
    }

    #[test]
    fn every_category_is_ordered() {
        for s in SKILLS {
            assert!(
                CATEGORY_ORDER.contains(&s.category),
                "category {:?} missing from CATEGORY_ORDER",
                s.category
            );
        }
    }
}
