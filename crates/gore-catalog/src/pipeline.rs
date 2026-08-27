//! Pipeline catalog builders — faithful Rust port of the three Python scripts.
//!
//! Each builder takes raw lines from a UE4SS `UE4SS_ObjectDump.txt` and
//! produces a `Vec<serde_json::Value>` whose JSON representation keeps the
//! historical formatting contract (indent=2, alphabetic keys, trailing
//! newline, same category strings). Item rows additionally carry their
//! deterministic shipped-game icon suffix.
//!
//! # Localization hook (future)
//!
//! The entry structs below use `#[serde(default, skip_serializing_if = "...")]`
//! on every optional field so that adding
//! ```ignore
//! /// Per-language display names loaded from .locres files (not yet available).
//! #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
//! pub names: std::collections::BTreeMap<String, String>,
//! ```
//! to [`ItemEntry`], [`NpcEntry`], and [`KnowledgeEntry`] later will be
//! **non-breaking**: existing consumers see no change because
//! `skip_serializing_if` suppresses the field when empty, and `default` gives
//! a zero-value on deserialization.
//!
//! Do NOT add `names` yet — it would break the byte-identical gate.

use serde::Serialize;

// ─── Item catalog ────────────────────────────────────────────────────────────

/// True when an Angelscript class name is an armor *item* class (a class that
/// can occupy an inventory slot), as opposed to its paired visual-definition
/// companion, an upgrade-component tier piece, or unrelated noise that merely
/// contains "Armor".
///
/// Armor item classes are NOT `It*`. They are faction-armor families
/// (`<Fac>_Armor[_suffix]`, e.g. `Ore_Armor_H`, `Org_Armor`) and per-NPC
/// armors (`Armor_<CAMP>_<NPC>_NNN`). Each is paired with a
/// `*_VisualsDefinition` / `*_VisualDefinition` companion that is NOT an item.
/// The `_{Top,Mid,Bot}_` tier pieces are armor-customization components stored
/// in `BoughtArmorUpgrades.AvailableUpgrades` and applied via the worn armor's
/// upgrade string-map (Tier C) — they are not standalone bag items.
fn is_armor_item_class(name: &str) -> bool {
    if !name.contains("Armor") {
        return false;
    }
    // Companions / bases / non-item definitions.
    if name.ends_with("Definition") || name.ends_with("_Base") {
        return false;
    }
    if name.starts_with("ArmorVisualsDefinition") {
        return false;
    }
    // Upgrade-component tier pieces (Org_Armor_Top_H_01, Sld_Armor_Mid_L_02 ...).
    if name.contains("_Top_") || name.contains("_Mid_") || name.contains("_Bot_") {
        return false;
    }
    // Non-item families that contain "Armor"/"Armory"/"SuperArmor".
    const NON_ITEM_PREFIXES: &[&str] = &[
        "GE_",
        "GA_",
        "GC_",
        "GVL_",
        "CS_",
        "Choice",
        "Document",
        "Conversation",
        "DailyRoutine",
        "Module_",
        "AIAgent",
        "CharacterDefinition",
        "CharacterVisuals",
        "AllArmors",
        "Quest",
        "Memory",
        "Spawner",
        "Glossary",
        "Gothic",
        "Hit_",
        "SpawnAIAgent",
        "SpawnMeshes",
        "OC_",
    ];
    if NON_ITEM_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return false;
    }
    // Item families: `<2-4 alpha>_Armor...` or `Armor_<CAMP>_...`. The tail must
    // be exactly `Armor` or start with `Armor_` — `starts_with("Armor")` alone
    // would also accept non-item names like `NC_Armory_Door` (an "Armory"
    // segment), which must not enter the catalog/allow-list.
    let faction_armor = {
        let mut parts = name.splitn(2, '_');
        let head = parts.next().unwrap_or("");
        let tail = parts.next().unwrap_or("");
        (2..=4).contains(&head.len())
            && head.chars().all(|c| c.is_ascii_alphabetic())
            && (tail == "Armor" || tail.starts_with("Armor_"))
    };
    faction_armor || name.starts_with("Armor_")
}

/// Regex-equivalent: capture group 1 of `ASClass /Script/Angelscript.(It[A-Za-z0-9_]+)`.
fn parse_item_classes(lines: &[&str]) -> Vec<String> {
    let prefix = "ASClass /Script/Angelscript.";
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in lines {
        if let Some(rest) = line.find(prefix).map(|i| &line[i + prefix.len()..]) {
            // Take up to the first non-`[A-Za-z0-9_]` character
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.starts_with("It") || is_armor_item_class(&name) {
                names.insert(name);
            }
        }
    }
    names.into_iter().collect() // BTreeSet is already sorted
}

/// Category table — order is significant (longer prefixes must come first where
/// they share a common shorter prefix, e.g. `ItAr_Rune_` before plain `ItAr_`).
const ITEM_CATEGORY_BY_PREFIX: &[(&str, &str)] = &[
    ("ItMw_", "melee_weapon"),
    ("ItRw_", "ranged_weapon"),
    ("ItAm_", "ammunition"),
    ("ItAr_Rune_", "rune"),
    ("ItAr_Scroll_", "scroll"),
    ("ItFo_", "food"),
    ("ItMi_", "misc"),
    ("ItAt_Amulet_", "amulet"),
    ("ItAt_Ring_", "ring"),
    ("ItAt_", "trophy"),
    ("ItWr_", "writing"),
    ("ItMs_", "mission"),
    ("ItKe_", "key"),
];

const ITEM_EXCLUDE_PREFIXES: &[&str] = &[
    "ItemAnimConfig",
    "ItemSpawnManagerConfig",
    "ItemCollisionFX",
    "ItemVisualWorldTargetConfig",
    "ItAI_",
];

fn item_explicit(id: &str) -> Option<&'static str> {
    match id {
        "ItKeyDefault" => Some("key"),
        "ItChestKey01" => Some("key"),
        "ItDoorKey01" => Some("key"),
        "ItIg_Worldsplitter" => Some("special"),
        "ItFocusStoneBridgeItem" => Some("special"),
        _ => None,
    }
}

/// One item catalog entry (category + icon + id + path).
///
/// Field order in the struct must match the alphabetical JSON key order required
/// by `sort_keys=True`: `category` < `icon` < `id` < `path`.
#[derive(Debug, Clone, Serialize)]
pub struct ItemEntry {
    pub category: String,
    pub icon: String,
    pub id: String,
    pub path: String,
    // Future: #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    // pub names: std::collections::BTreeMap<String, String>,
}

/// Build the item catalog from raw dump lines.
///
/// Returns `(entries, skipped)` mirroring `build_item_catalog.build_catalog`.
pub fn build_item_catalog(lines: &[&str]) -> (Vec<ItemEntry>, Vec<String>) {
    let names = parse_item_classes(lines);
    let mut entries: Vec<ItemEntry> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for name in &names {
        let excluded = ITEM_EXCLUDE_PREFIXES.iter().any(|p| name.starts_with(p));
        if excluded || name.ends_with("_Base") {
            skipped.push(name.clone());
            continue;
        }
        let category: String = if is_armor_item_class(name) {
            "armor".to_string()
        } else if let Some(cat) = item_explicit(name) {
            cat.to_string()
        } else {
            let mut found = None;
            for (prefix, cat) in ITEM_CATEGORY_BY_PREFIX {
                if name.starts_with(prefix) {
                    found = Some(*cat);
                    break;
                }
            }
            match found {
                Some(cat) => cat.to_string(),
                None => {
                    skipped.push(format!("{} (unmatched prefix -> special)", name));
                    "special".to_string()
                }
            }
        };
        entries.push(ItemEntry {
            category,
            icon: crate::resolve_item_icon(name),
            id: name.clone(),
            path: format!("/Script/Angelscript.{}", name),
        });
    }
    // entries already in sorted order because names came from BTreeSet
    (entries, skipped)
}

// ─── NPC catalog ─────────────────────────────────────────────────────────────

/// Regex-equivalent: `ASClass /Script/Angelscript.(CharacterDefinition_[A-Za-z0-9_]+)`.
fn parse_npc_classes(lines: &[&str]) -> Vec<String> {
    let prefix = "ASClass /Script/Angelscript.CharacterDefinition_";
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in lines {
        if let Some(pos) = line.find(prefix) {
            let rest = &line[pos + "ASClass /Script/Angelscript.".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names.into_iter().collect()
}

/// One NPC catalog entry.
///
/// Field order: `category` < `class` < `id` (alphabetical, sort_keys=True).
#[derive(Debug, Clone, Serialize)]
pub struct NpcEntry {
    pub category: String,
    pub class: String,
    pub id: String,
    // Future: #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    // pub names: std::collections::BTreeMap<String, String>,
}

/// Build the NPC catalog.
pub fn build_npc_catalog(lines: &[&str]) -> (Vec<NpcEntry>, Vec<String>) {
    let class_names = parse_npc_classes(lines);
    let mut entries: Vec<NpcEntry> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for cls in &class_names {
        let rest = &cls["CharacterDefinition_".len()..];
        let (category, unique) = if let Some(stripped) = rest.strip_prefix("Human_") {
            ("human", stripped.to_string())
        } else if rest.starts_with("Creature_") {
            ("creature", rest.to_string())
        } else {
            ("other", rest.to_string())
        };
        if unique.is_empty() {
            skipped.push(cls.clone());
            continue;
        }
        entries.push(NpcEntry {
            category: category.to_string(),
            class: cls.clone(),
            id: unique,
        });
    }
    // Sort by id (Python: entries.sort(key=lambda e: e["id"]))
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    // Dedup by id (Python keeps first occurrence)
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    entries.retain(|e| seen.insert(e.id.clone()));
    (entries, skipped)
}

// ─── Knowledge catalog ───────────────────────────────────────────────────────

/// One knowledge catalog entry.
///
/// Field order is alphabetical to preserve the generator's `sort_keys=True`
/// output. Cache-derived fields are optional so catalogs generated without a
/// script cache remain byte-compatible with the original two-field format.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    pub category: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loc_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

/// Build the knowledge catalog from raw dump lines.
///
/// Mirrors `build_knowledge_catalog.parse_dump_classes` + `build_catalog`.
/// Order matters: Topic_/Info_ are checked before bare Choice.
pub fn build_knowledge_catalog(lines: &[&str]) -> Vec<KnowledgeEntry> {
    let prefix = "ASClass /Script/Angelscript.";
    let mut found: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    for line in lines {
        if let Some(pos) = line.find(prefix) {
            let rest = &line[pos + prefix.len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            // Order matches Python: Topic_ first, Info_ second, Choice last
            let category = if name.starts_with("Topic_") {
                "topic"
            } else if name.starts_with("Info_") {
                "info"
            } else if name.strip_prefix("Choice").is_some_and(|r| {
                r.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            }) {
                // Python pattern was `Choice[A-Za-z0-9_]+`: a bare abstract
                // `Choice` class (no trailing word char) is NOT a token.
                "choice"
            } else {
                continue;
            };
            // setdefault: only insert if not already present
            found.entry(name).or_insert_with(|| category.to_string());
        }
    }

    // BTreeMap iterates in sorted key order — matches Python's `sorted(found.items())`
    let mut entries: Vec<KnowledgeEntry> = found
        .into_iter()
        .map(|(id, category)| KnowledgeEntry {
            caption: None,
            category,
            id,
            loc_key: None,
            module: None,
        })
        .collect();
    // Python: entries.sort(key=lambda e: e["id"]) — already sorted from BTreeMap
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// Join exact cache-derived `(id, caption, loc_key, module)` metadata into an existing
/// knowledge catalog. Unknown cache classes are ignored and catalog entries
/// missing from the inspected cache retain their portable two-field form.
pub fn enrich_knowledge_catalog(
    entries: &mut [KnowledgeEntry],
    metadata: impl IntoIterator<Item = (String, Option<String>, Option<String>, String)>,
) {
    let by_id: std::collections::BTreeMap<String, (Option<String>, Option<String>, String)> =
        metadata
            .into_iter()
            .map(|(id, caption, loc_key, module)| (id, (caption, loc_key, module)))
            .collect();
    for entry in entries {
        if let Some((caption, loc_key, module)) = by_id.get(&entry.id) {
            entry.caption = caption.clone();
            entry.loc_key = loc_key.clone();
            entry.module = Some(module.clone());
        }
    }
}

// ─── JSON serialization helpers ──────────────────────────────────────────────

/// Serialize entries to JSON with indent=2, sort_keys=True, trailing newline.
///
/// serde_json's `to_string_pretty` already uses 2-space indentation.
/// The entry structs use alphabetically ordered fields (struct field declaration
/// order == serialization order in serde), which achieves sort_keys=True.
pub fn to_catalog_json<T: Serialize>(entries: &[T]) -> serde_json::Result<String> {
    let mut s = serde_json::to_string_pretty(entries)?;
    s.push('\n');
    Ok(s)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_FIXTURE: &str = "\
[0000025A88701900] ASClass /Script/Angelscript.ItemAnimConfig_Meatbug [n: 1] [c: 2] [or: 3]
[0000025A88701901] ASClass /Script/Angelscript.ItMi_Orenugget [n: 1] [c: 2] [or: 3]
[0000025A88701902] ASClass /Script/Angelscript.ItMi_Orenugget [n: 1] [c: 2] [or: 3]
[0000025A88701903] ASClass /Script/Angelscript.ItAr_Rune_FireBall_Base [n: 1] [c: 2] [or: 3]
[0000025A88701904] ASClass /Script/Angelscript.ItAr_Rune_FireBall [n: 1] [c: 2] [or: 3]
[0000025A88701905] ASClass /Script/Angelscript.ItAr_Scroll_Charm [n: 1] [c: 2] [or: 3]
[0000025A88701906] ASClass /Script/Angelscript.ItAI_Plank [n: 1] [c: 2] [or: 3]
[0000025A88701907] ASClass /Script/Angelscript.ItKeyDefault [n: 1] [c: 2] [or: 3]
[0000025A88701908] ASClass /Script/Angelscript.ItIg_Worldsplitter [n: 1] [c: 2] [or: 3]
[0000025A88701909] ASClass /Script/Angelscript.SomethingElse [n: 1] [c: 2] [or: 3]
[0000025A8870190A] ASClass /Script/Angelscript.ItAm_Arrow [n: 1] [c: 2] [or: 3]
[0000025A8870190B] ASClass /Script/Angelscript.ItAt_Amulet_OfDeath [n: 1] [c: 2] [or: 3]
[0000025A8870190C] ASClass /Script/Angelscript.ItAt_Ring_OfLife [n: 1] [c: 2] [or: 3]
[0000025A8870190D] ASClass /Script/Angelscript.ItAt_Wolf_Fur [n: 1] [c: 2] [or: 3]";

    #[test]
    fn item_dedupes_and_filters() {
        let lines: Vec<&str> = ITEM_FIXTURE.lines().collect();
        let (entries, skipped) = build_item_catalog(&lines);
        let by_id: std::collections::HashMap<&str, &ItemEntry> =
            entries.iter().map(|e| (e.id.as_str(), e)).collect();
        assert_eq!(by_id["ItMi_Orenugget"].category, "misc");
        assert_eq!(by_id["ItMi_Orenugget"].icon, "ItMi_Orenugget");
        assert_eq!(
            by_id["ItMi_Orenugget"].path,
            "/Script/Angelscript.ItMi_Orenugget"
        );
        assert_eq!(by_id["ItAr_Rune_FireBall"].category, "rune");
        assert_eq!(by_id["ItAr_Scroll_Charm"].category, "scroll");
        assert_eq!(by_id["ItKeyDefault"].category, "key");
        assert_eq!(by_id["ItIg_Worldsplitter"].category, "special");
        assert_eq!(by_id["ItAm_Arrow"].category, "ammunition");
        assert_eq!(by_id["ItAt_Amulet_OfDeath"].category, "amulet");
        assert_eq!(by_id["ItAt_Ring_OfLife"].category, "ring");
        assert_eq!(by_id["ItAt_Wolf_Fur"].category, "trophy");
        assert!(!by_id.contains_key("ItAr_Rune_FireBall_Base"));
        assert!(!by_id.contains_key("ItemAnimConfig_Meatbug"));
        assert!(!by_id.contains_key("ItAI_Plank"));
        assert!(skipped.contains(&"ItAr_Rune_FireBall_Base".to_string()));
    }

    #[test]
    fn item_entries_sorted() {
        let lines: Vec<&str> = ITEM_FIXTURE.lines().collect();
        let (entries, _) = build_item_catalog(&lines);
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
    }

    const NPC_FIXTURE: &[&str] = &[
        "[0001] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: A]",
        "[0002] ASClass /Script/Angelscript.CharacterDefinition_Human_NC_SLD_Gorn_699 [n: B]",
        "[0003] ASClass /Script/Angelscript.CharacterDefinition_Creature_Biter [n: C]",
        "[0004] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: D]",
        "[0005] ASClass /Script/Angelscript.ItMw_Sword01 [n: E]",
    ];

    #[test]
    fn npc_human_map_key_form() {
        let (entries, _) = build_npc_catalog(NPC_FIXTURE);
        let by_id: std::collections::HashMap<&str, &NpcEntry> =
            entries.iter().map(|e| (e.id.as_str(), e)).collect();
        assert_eq!(by_id["OC_STT_Diego"].category, "human");
        assert_eq!(
            by_id["OC_STT_Diego"].class,
            "CharacterDefinition_Human_OC_STT_Diego"
        );
    }

    #[test]
    fn npc_creature_category() {
        let (entries, _) = build_npc_catalog(NPC_FIXTURE);
        let by_id: std::collections::HashMap<&str, &NpcEntry> =
            entries.iter().map(|e| (e.id.as_str(), e)).collect();
        assert_eq!(by_id["Creature_Biter"].category, "creature");
    }

    #[test]
    fn npc_dedup_and_sorted() {
        let (entries, _) = build_npc_catalog(NPC_FIXTURE);
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
        assert_eq!(ids.iter().filter(|&&id| id == "OC_STT_Diego").count(), 1);
    }

    const KNOWLEDGE_FIXTURE: &[&str] = &[
        "[1] ASClass /Script/Angelscript.Topic_Diego_209799 [n: A]",
        "[2] ASClass /Script/Angelscript.Info_FMORGAreyouok [n: B]",
        "[3] ASClass /Script/Angelscript.ChoiceDiegoGamestart [n: C]",
        "[4] ASClass /Script/Angelscript.Topic_Diego_209799 [n: D]",
        "[5] ASClass /Script/Angelscript.ItMw_Sword01 [n: E]",
        "[6] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: F]",
    ];

    #[test]
    fn knowledge_categories() {
        let entries = build_knowledge_catalog(KNOWLEDGE_FIXTURE);
        let by_id: std::collections::HashMap<&str, &KnowledgeEntry> =
            entries.iter().map(|e| (e.id.as_str(), e)).collect();
        assert_eq!(by_id["Topic_Diego_209799"].category, "topic");
        assert_eq!(by_id["Info_FMORGAreyouok"].category, "info");
        assert_eq!(by_id["ChoiceDiegoGamestart"].category, "choice");
    }

    #[test]
    fn knowledge_dedup_sorted_filtered() {
        let entries = build_knowledge_catalog(KNOWLEDGE_FIXTURE);
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
        assert_eq!(
            ids.iter().filter(|&&id| id == "Topic_Diego_209799").count(),
            1
        );
        assert!(!ids
            .iter()
            .any(|id| id.contains("Sword") || id.contains("CharacterDefinition")));
    }

    #[test]
    fn knowledge_rejects_bare_choice_class() {
        let lines = [
            "[1] ASClass /Script/Angelscript.Choice [n: A]", // bare abstract -> excluded
            "[2] ASClass /Script/Angelscript.ChoiceDiegoStart [n: B]", // concrete -> kept
        ];
        let entries = build_knowledge_catalog(&lines);
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"ChoiceDiegoStart"), "got: {ids:?}");
        assert!(
            !ids.contains(&"Choice"),
            "bare Choice must be excluded: {ids:?}"
        );
    }

    #[test]
    fn knowledge_metadata_enrichment_is_exact_and_optional() {
        let mut entries = build_knowledge_catalog(KNOWLEDGE_FIXTURE);
        enrich_knowledge_catalog(
            &mut entries,
            [
                (
                    "Topic_Diego_209799".to_string(),
                    None,
                    Some("INFO_DIEGO_OTHERCAMPS_15_00".to_string()),
                    "Story.Conversation_Diego".to_string(),
                ),
                (
                    "Info_FMORGAreyouok".to_string(),
                    Some("[Forced Conversation]".to_string()),
                    None,
                    "Story.Conversation_Forced".to_string(),
                ),
                (
                    "Topic_NotInDump".to_string(),
                    None,
                    Some("INFO_UNUSED".to_string()),
                    "Story.Unused".to_string(),
                ),
            ],
        );
        let by_id: std::collections::HashMap<&str, &KnowledgeEntry> = entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry))
            .collect();
        assert_eq!(
            by_id["Topic_Diego_209799"].loc_key.as_deref(),
            Some("INFO_DIEGO_OTHERCAMPS_15_00")
        );
        assert_eq!(
            by_id["Topic_Diego_209799"].module.as_deref(),
            Some("Story.Conversation_Diego")
        );
        assert_eq!(
            by_id["Info_FMORGAreyouok"].caption.as_deref(),
            Some("[Forced Conversation]")
        );
        assert!(by_id["Info_FMORGAreyouok"].loc_key.is_none());

        let json = to_catalog_json(&entries).unwrap();
        assert!(json.contains("\"loc_key\": \"INFO_DIEGO_OTHERCAMPS_15_00\""));
        assert!(json.contains("\"caption\": \"[Forced Conversation]\""));
        assert!(!json.contains("Topic_NotInDump"));
    }

    #[test]
    fn armor_discriminator_accepts_items_rejects_companions() {
        // Real base/per-NPC armor item classes -> accepted.
        assert!(is_armor_item_class("Ore_Armor_H"));
        assert!(is_armor_item_class("Ore_Armor_M"));
        assert!(is_armor_item_class("Crw_Armor_H"));
        assert!(is_armor_item_class("Org_Armor"));
        assert!(is_armor_item_class("Vlk_Armor_L"));
        assert!(is_armor_item_class("Ebr_Armor_H_01"));
        assert!(is_armor_item_class("Armor_SK_OC_WOC_Velaya_108_02"));
        assert!(is_armor_item_class("Armor_OC_EBR_Gomez_100"));

        // Visual-definition companions and bases -> rejected.
        assert!(!is_armor_item_class("Ore_Armor_H_VisualsDefinition"));
        assert!(!is_armor_item_class(
            "Armor_OC_EBR_Gomez_100_VisualDefinition"
        ));
        assert!(!is_armor_item_class("BaseArmorDefinition"));
        assert!(!is_armor_item_class("ArmorVisualsDefinition_Human"));

        // Upgrade-component tier pieces -> rejected (edited via Tier C, not added).
        assert!(!is_armor_item_class("Org_Armor_Top_H_01"));
        assert!(!is_armor_item_class("Sld_Armor_Mid_L_02"));

        // Non-item noise that merely contains "Armor" -> rejected.
        assert!(!is_armor_item_class("GE_Crw_Armor_H"));
        assert!(!is_armor_item_class("GothicAchievement_Armor_01"));
        assert!(!is_armor_item_class("OC_Armory_Door"));
        // An "Armory" segment (room/building) is not an armor item, even with a
        // short prefix not covered by NON_ITEM_PREFIXES.
        assert!(!is_armor_item_class("NC_Armory_Door"));
        assert!(!is_armor_item_class("Vlk_Armory"));
        assert!(!is_armor_item_class("Spawner_OC_Castle_Armory_Misc_01"));
        assert!(!is_armor_item_class("Hit_SuperArmor_Player"));
        assert!(!is_armor_item_class("CharacterVisualsDefinition_OreArmor"));

        // Ordinary It* items are not armor.
        assert!(!is_armor_item_class("ItMi_Orenugget"));
    }

    #[test]
    fn armor_entries_get_armor_category() {
        let lines = [
            "[0001] ASClass /Script/Angelscript.Ore_Armor_H [n: 1] [c: 2]",
            "[0002] ASClass /Script/Angelscript.Ore_Armor_H_VisualsDefinition [n: 1] [c: 2]",
            "[0003] ASClass /Script/Angelscript.Armor_OC_EBR_Gomez_100 [n: 1] [c: 2]",
            "[0004] ASClass /Script/Angelscript.ItMi_Orenugget [n: 1] [c: 2]",
            "[0005] ASClass /Script/Angelscript.Org_Armor_Top_H_01 [n: 1] [c: 2]",
        ];
        let (entries, _skipped) = build_item_catalog(&lines);
        let by_id: std::collections::HashMap<&str, &ItemEntry> =
            entries.iter().map(|e| (e.id.as_str(), e)).collect();

        assert_eq!(by_id["Ore_Armor_H"].category, "armor");
        assert_eq!(by_id["Ore_Armor_H"].path, "/Script/Angelscript.Ore_Armor_H");
        assert_eq!(by_id["Armor_OC_EBR_Gomez_100"].category, "armor");
        assert_eq!(by_id["ItMi_Orenugget"].category, "misc");
        // companion + tier piece are not catalog entries at all
        assert!(!by_id.contains_key("Ore_Armor_H_VisualsDefinition"));
        assert!(!by_id.contains_key("Org_Armor_Top_H_01"));
    }

    #[test]
    fn item_json_format() {
        let lines: Vec<&str> = ITEM_FIXTURE.lines().collect();
        let (entries, _) = build_item_catalog(&lines);
        let json = to_catalog_json(&entries).unwrap();
        // Must end with trailing newline
        assert!(json.ends_with('\n'));
        // Parse back and check the generated icon field.
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        let ore = parsed.iter().find(|e| e["id"] == "ItMi_Orenugget").unwrap();
        assert_eq!(ore["category"], "misc");
        assert_eq!(ore["icon"], "ItMi_Orenugget");
        assert_eq!(ore["path"], "/Script/Angelscript.ItMi_Orenugget");
    }
}
