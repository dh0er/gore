//! Pipeline catalog builders — faithful Rust port of the three Python scripts.
//!
//! Each builder takes raw lines from a UE4SS `UE4SS_ObjectDump.txt` and
//! produces a `Vec<serde_json::Value>` whose JSON representation is
//! **byte-identical** to the output of the corresponding Python script
//! (indent=2, sort_keys=True, trailing newline, same category strings).
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
            if name.starts_with("It") {
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

/// One item catalog entry (id + path + category).
///
/// Field order in the struct must match the alphabetical JSON key order required
/// by `sort_keys=True`: `category` < `id` < `path`.
#[derive(Debug, Clone, Serialize)]
pub struct ItemEntry {
    pub category: String,
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
        let category: String = if let Some(cat) = item_explicit(name) {
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
/// Field order: `category` < `id` (alphabetical, sort_keys=True).
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeEntry {
    pub category: String,
    pub id: String,
    // Future: #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    // pub names: std::collections::BTreeMap<String, String>,
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
            } else if name.starts_with("Choice") {
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
        .map(|(id, category)| KnowledgeEntry { category, id })
        .collect();
    // Python: entries.sort(key=lambda e: e["id"]) — already sorted from BTreeMap
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
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
        assert_eq!(by_id["ItMi_Orenugget"].path, "/Script/Angelscript.ItMi_Orenugget");
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
        assert_eq!(by_id["OC_STT_Diego"].class, "CharacterDefinition_Human_OC_STT_Diego");
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
        assert_eq!(ids.iter().filter(|&&id| id == "Topic_Diego_209799").count(), 1);
        assert!(!ids.iter().any(|id| id.contains("Sword") || id.contains("CharacterDefinition")));
    }

    #[test]
    fn item_json_format() {
        let lines: Vec<&str> = ITEM_FIXTURE.lines().collect();
        let (entries, _) = build_item_catalog(&lines);
        let json = to_catalog_json(&entries).unwrap();
        // Must end with trailing newline
        assert!(json.ends_with('\n'));
        // Parse back and check field order (category before id before path)
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        let ore = parsed.iter().find(|e| e["id"] == "ItMi_Orenugget").unwrap();
        assert_eq!(ore["category"], "misc");
        assert_eq!(ore["path"], "/Script/Angelscript.ItMi_Orenugget");
    }
}
