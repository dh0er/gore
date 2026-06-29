# Armor Support — Tier A (Catalog Item + Category) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make armor a first-class inventory item in the save editor — armor appears under its own "Armor" category (instead of "Other") and can be added to the bag like any other item.

**Architecture:** Armor classes are `<Faction>_Armor[...]` / `Armor_<Camp>_<NPC>_NNN` (not `It*`). Teach the catalog generator to recognize armor item classes and tag them `armor`; regenerate the bundled `item_catalog.json` (the Rust core's addItem allow-list embeds it via `include_str!`); add an `Armor` category to the catalog model and the save-editor Flutter UI. The core display scan already returns armor rows — no scan change needed for Tier A. Equipped indicator, equip/unequip, and upgrade editing are Tiers B and C (separate plans).

**Tech Stack:** Rust (`gore-catalog`, `gore-save`), `cargo test`; Flutter/Dart (`apps/save-editor`), `flutter test` + `flutter gen-l10n`.

**Spec:** `docs/superpowers/specs/2026-06-29-inventory-armor-support-design.md`

**Test fixtures:** real decompressed payloads in `work/decompressed/G1R-0{16,19,20}.host.bin` (gitignored scratch, already present in this worktree). G1R-019 carries three armors: `Ore_Armor_H` (equipped), `Crw_Armor_H`, `Ore_Armor_M` (bag).

---

## File Structure

- `crates/gore-catalog/src/pipeline.rs` — add `is_armor_item_class` + armor discriminator; integrate into `parse_item_classes` and `build_item_catalog` (category `"armor"`).
- `crates/gore-catalog/src/lib.rs` — add `ItemCategory::Armor`; map armor ids in `item_category_from_id` and `category_for_id`; update the two `Armor_OC_Gomez` assertions.
- `crates/gore-catalog/tests/catalog_test.rs` — update `category_unknown_for_unrecognized` (armor now `Armor`).
- `apps/save-editor/assets/item_catalog.json` — regenerated artifact (now includes armor entries).
- `apps/save-editor/lib/features/editor/domain/item_categories.dart` — `ItemCategory.armor` enum variant; `_isArmorId` helper; `itemCategoryFromId` armor branch; `localizedItemCategoryLabel` case.
- `apps/save-editor/lib/features/editor/ui/sidebar_tile.dart` — `iconForItemCategory` armor case.
- `apps/save-editor/lib/l10n/app_*.arb` (12 files) + regenerated `app_localizations*.dart` — new `itemCategoryArmor` key.
- Tests: `crates/gore-catalog/src/pipeline.rs` (`#[cfg(test)] mod tests`), `crates/gore-catalog/src/lib.rs` (`#[cfg(test)] mod tests`), `crates/gore-save/src/lib.rs` (real-payload add test), `apps/save-editor/test/features/editor/domain/item_categories_test.dart`.

---

## Task 1: Armor item-class discriminator (catalog generator)

**Files:**
- Modify: `crates/gore-catalog/src/pipeline.rs` (add `is_armor_item_class`, extend `parse_item_classes`)
- Test: `crates/gore-catalog/src/pipeline.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `crates/gore-catalog/src/pipeline.rs`:

```rust
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
    assert!(!is_armor_item_class("Armor_OC_EBR_Gomez_100_VisualDefinition"));
    assert!(!is_armor_item_class("BaseArmorDefinition"));
    assert!(!is_armor_item_class("ArmorVisualsDefinition_Human"));

    // Upgrade-component tier pieces -> rejected (edited via Tier C, not added).
    assert!(!is_armor_item_class("Org_Armor_Top_H_01"));
    assert!(!is_armor_item_class("Sld_Armor_Mid_L_02"));

    // Non-item noise that merely contains "Armor" -> rejected.
    assert!(!is_armor_item_class("GE_Crw_Armor_H"));
    assert!(!is_armor_item_class("GothicAchievement_Armor_01"));
    assert!(!is_armor_item_class("OC_Armory_Door"));
    assert!(!is_armor_item_class("Spawner_OC_Castle_Armory_Misc_01"));
    assert!(!is_armor_item_class("Hit_SuperArmor_Player"));
    assert!(!is_armor_item_class("CharacterVisualsDefinition_OreArmor"));

    // Ordinary It* items are not armor.
    assert!(!is_armor_item_class("ItMi_Orenugget"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-catalog armor_discriminator_accepts_items_rejects_companions`
Expected: FAIL — `cannot find function `is_armor_item_class``.

- [ ] **Step 3: Write minimal implementation**

Add above `parse_item_classes` in `crates/gore-catalog/src/pipeline.rs`:

```rust
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
        "GE_", "GA_", "GC_", "GVL_", "CS_", "Choice", "Document", "Conversation",
        "DailyRoutine", "Module_", "AIAgent", "CharacterDefinition",
        "CharacterVisuals", "AllArmors", "Quest", "Memory", "Spawner",
        "Glossary", "Gothic", "Hit_", "SpawnAIAgent", "SpawnMeshes", "OC_",
    ];
    if NON_ITEM_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return false;
    }
    // Item families: `<2-4 alpha>_Armor...` or `Armor_<CAMP>_...`.
    let faction_armor = {
        let mut parts = name.splitn(2, '_');
        let head = parts.next().unwrap_or("");
        let tail = parts.next().unwrap_or("");
        (2..=4).contains(&head.len())
            && head.chars().all(|c| c.is_ascii_alphabetic())
            && tail.starts_with("Armor")
    };
    faction_armor || name.starts_with("Armor_")
}
```

Then change the filter inside `parse_item_classes` (line ~39):

```rust
            if name.starts_with("It") || is_armor_item_class(&name) {
                names.insert(name);
            }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-catalog armor_discriminator_accepts_items_rejects_companions`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-catalog/src/pipeline.rs
git commit -m "feat(catalog): recognize armor item classes in the dump parser"
```

---

## Task 2: Tag armor entries with category "armor" in the generated catalog

**Files:**
- Modify: `crates/gore-catalog/src/pipeline.rs` (`build_item_catalog` category resolution)
- Test: `crates/gore-catalog/src/pipeline.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `pipeline.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-catalog armor_entries_get_armor_category`
Expected: FAIL — `Ore_Armor_H` resolves to `"special"` (unmatched prefix), not `"armor"`.

- [ ] **Step 3: Write minimal implementation**

In `build_item_catalog` (`pipeline.rs`), change the category resolution so armor is detected before the `It*` prefix table. Replace the `let category: String = if let Some(cat) = item_explicit(name) {` block's head with:

```rust
        let category: String = if is_armor_item_class(name) {
            "armor".to_string()
        } else if let Some(cat) = item_explicit(name) {
            cat.to_string()
        } else {
```

(The rest of the `else` body — the prefix-table loop and the `"special"` fallback — is unchanged.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-catalog armor_entries_get_armor_category`
Expected: PASS.

- [ ] **Step 5: Run the full crate test suite (catch regressions in existing pipeline tests)**

Run: `cargo test -p gore-catalog`
Expected: PASS except the two pre-existing `Armor_OC_Gomez` assertions, fixed in Task 3. If they fail here, that is expected and addressed next.

- [ ] **Step 6: Commit**

```bash
git add crates/gore-catalog/src/pipeline.rs
git commit -m "feat(catalog): tag armor classes with the armor category"
```

---

## Task 3: Add `ItemCategory::Armor` to the catalog model

**Files:**
- Modify: `crates/gore-catalog/src/lib.rs` (`ItemCategory` enum, `label`, `item_category_from_id`, `category_for_id`, inline `unknown_ids_map_to_other` test)
- Modify: `crates/gore-catalog/tests/catalog_test.rs` (`category_unknown_for_unrecognized`)

- [ ] **Step 1: Write/Update the failing tests**

In `crates/gore-catalog/src/lib.rs` `#[cfg(test)] mod tests`, replace the body of `unknown_ids_map_to_other` (the line asserting `Armor_OC_Gomez` → `Other`) and add an armor test:

```rust
    #[test]
    fn unknown_ids_map_to_other() {
        assert_eq!(item_category_from_id(""), ItemCategory::Other);
        assert_eq!(item_category_from_id("ItIg_Worldsplitter"), ItemCategory::Other);
    }

    #[test]
    fn armor_ids_map_to_armor() {
        assert_eq!(item_category_from_id("Ore_Armor_H"), ItemCategory::Armor);
        assert_eq!(item_category_from_id("Org_Armor"), ItemCategory::Armor);
        assert_eq!(item_category_from_id("Armor_OC_Gomez"), ItemCategory::Armor);
        assert_eq!(category_for_id("Ore_Armor_H"), ItemCategory::Armor);
        assert_eq!(category_for_id("Armor_OC_Gomez"), ItemCategory::Armor);
    }
```

In `crates/gore-catalog/tests/catalog_test.rs`, replace `category_unknown_for_unrecognized`:

```rust
#[test]
fn category_armor_for_armor_classes() {
    assert_eq!(category_for_id("Armor_OC_Gomez"), ItemCategory::Armor);
    assert_eq!(category_for_id("Ore_Armor_H"), ItemCategory::Armor);
}

#[test]
fn category_unknown_for_unrecognized() {
    assert_eq!(category_for_id("TotallyUnknownClass"), ItemCategory::Unknown);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gore-catalog armor_ids_map_to_armor`
Expected: FAIL — no `ItemCategory::Armor` variant.

- [ ] **Step 3: Write minimal implementation**

In `crates/gore-catalog/src/lib.rs`:

(a) Add the variant to the `ItemCategory` enum — place it right after `RangedWeapon` so weapons and armor sit together (declaration order drives UI order in the mirrored Dart enum; keep them consistent):

```rust
    MeleeWeapon,
    RangedWeapon,
    /// Wearable armor (`<Fac>_Armor_*`, `Armor_<Camp>_<NPC>_*`).
    Armor,
    Ammunition,
```

(b) Add a `label()` arm (find the `label` match and add):

```rust
            ItemCategory::Armor => "Armor",
```

(c) In `item_category_from_id`, add an armor branch. Armor ids are not `It*`, so add it as the first check (before the `ItMw_` chain), reusing a shared helper:

```rust
pub fn item_category_from_id(id: &str) -> ItemCategory {
    if is_armor_id(id) {
        ItemCategory::Armor
    } else if id.starts_with("ItMw_") {
        ItemCategory::MeleeWeapon
    } else if id.starts_with("ItRw_") {
```

(d) In `category_for_id`, add at the top (before the `ItFo_` check):

```rust
pub fn category_for_id(id: &str) -> ItemCategory {
    if is_armor_id(id) {
        return ItemCategory::Armor;
    }
    if id.starts_with("ItFo_") {
```

(e) Add the shared `is_armor_id` helper (display-side classifier — broader than the catalog's `is_armor_item_class`: it also classifies tier pieces as armor so any armor already in a save is grouped correctly):

```rust
/// Display-side armor classifier: true for any armor class name (base, per-NPC,
/// or tier piece). Broader than the catalog's `is_armor_item_class`, which
/// additionally excludes tier pieces from the *addable* set.
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
    (2..=4).contains(&head.len())
        && head.chars().all(|c| c.is_ascii_alphabetic())
        && tail.starts_with("Armor")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gore-catalog`
Expected: PASS (all tests, including the updated assertions).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-catalog/src/lib.rs crates/gore-catalog/tests/catalog_test.rs
git commit -m "feat(catalog): add Armor category to the catalog model"
```

---

## Task 4: Regenerate the bundled item catalog

**Files:**
- Modify (regenerated artifact): `apps/save-editor/assets/item_catalog.json`

The catalog is generated from the UE4SS object dump on the user's machine:
`D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\UE4SS_ObjectDump.txt`

- [ ] **Step 1: Regenerate the catalog**

Run (from the worktree root):

```bash
cargo run -p gore -- catalog --kind item \
  "D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/UE4SS_ObjectDump.txt" \
  -o apps/save-editor/assets/item_catalog.json
```

Expected: prints skipped classes to stderr; writes the JSON. (If the dump path differs, use the actual `UE4SS_ObjectDump.txt`.)

- [ ] **Step 2: Verify armor is present and companions are absent**

Run:

```bash
grep -c '"category": "armor"' apps/save-editor/assets/item_catalog.json
grep -c 'VisualsDefinition\|VisualDefinition' apps/save-editor/assets/item_catalog.json
grep -E '"id": "(Ore_Armor_H|Crw_Armor_H|Ore_Armor_M|Org_Armor)"' apps/save-editor/assets/item_catalog.json
```

Expected: first count > 0 (roughly ~110, base armors only — tier pieces excluded); second count == 0; the four known real-save armors all present.

- [ ] **Step 3: Confirm the Rust core embed still compiles (catalog is `include_str!`-embedded)**

Run: `cargo build -p gore-save`
Expected: builds (the larger JSON is parsed at runtime by `item_catalog_paths`; no compile impact).

- [ ] **Step 4: Commit**

```bash
git add apps/save-editor/assets/item_catalog.json
git commit -m "chore(catalog): regenerate item_catalog.json with armor entries"
```

---

## Task 5: Verify armor is addable through the core (real-payload test)

**Files:**
- Test: `crates/gore-save/src/lib.rs` `#[cfg(test)] mod tests`

This proves the regenerated catalog makes `addItem` accept an armor path and that the splice round-trips byte-faithfully on a real save.

- [ ] **Step 1: Write the failing (ignored, real-payload) test**

Add to the `#[cfg(test)] mod tests` block in `crates/gore-save/src/lib.rs`:

```rust
    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn add_armor_item_roundtrips_on_real_payload() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let mut payload = std::fs::read(path).unwrap();
        // An armor NOT already in this save's MainContainer.
        let armor_path = "/Script/Angelscript.Vlk_Armor_L";
        assert!(is_item_definition_class(armor_path), "armor must be in the catalog allow-list");
        let edit = PrivateInventoryAddItemEdit { path: armor_path.to_string(), count: 1 };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "byte-faithful: no trailing/lost bytes");
        let summary = main_container_summary(&root).expect("main container");
        assert!(summary.all_paths.contains(armor_path), "armor now in MainContainer");
    }
```

- [ ] **Step 2: Run it (must fail before Task 4's catalog regen, pass after)**

Run:

```bash
GORESAVE_PAYLOAD_BIN="work/decompressed/G1R-016.host.bin" \
  cargo test -p gore-save add_armor_item_roundtrips_on_real_payload -- --ignored --nocapture
```

Expected: PASS once the regenerated catalog (Task 4) contains `Vlk_Armor_L`. (If it fails on `is_item_definition_class`, Task 4 did not include that armor — re-check the discriminator/regen.)

- [ ] **Step 3: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "test(gore-save): armor addItem round-trips on a real payload"
```

---

## Task 6: Add the `Armor` category to the save-editor UI enum + mapping

**Files:**
- Modify: `apps/save-editor/lib/features/editor/domain/item_categories.dart`
- Test: `apps/save-editor/test/features/editor/domain/item_categories_test.dart`

- [ ] **Step 1: Write the failing test**

Add to `apps/save-editor/test/features/editor/domain/item_categories_test.dart`:

```dart
  test('armor classes categorize as armor', () {
    expect(itemCategoryFromId('Ore_Armor_H'), ItemCategory.armor);
    expect(itemCategoryFromId('Org_Armor'), ItemCategory.armor);
    expect(itemCategoryFromId('Armor_OC_Gomez'), ItemCategory.armor);
    expect(itemCategoryFromId('Org_Armor_Top_H_01'), ItemCategory.armor);
    // non-armor unaffected
    expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
    expect(itemCategoryFromId('SomethingElse'), ItemCategory.other);
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/save-editor && flutter test test/features/editor/domain/item_categories_test.dart`
Expected: FAIL — no `ItemCategory.armor`.

- [ ] **Step 3: Write minimal implementation**

In `apps/save-editor/lib/features/editor/domain/item_categories.dart`:

(a) Add the variant right after `rangedWeapon` (matches the Rust enum order):

```dart
  meleeWeapon('Melee weapons'),
  rangedWeapon('Ranged weapons'),
  armor('Armor'),
  ammunition('Ammunition'),
```

(b) Add the armor branch at the TOP of `itemCategoryFromId` (armor ids are not `It*`):

```dart
ItemCategory itemCategoryFromId(String id) {
  if (_isArmorId(id)) return ItemCategory.armor;
  if (id.startsWith('ItMw_')) return ItemCategory.meleeWeapon;
```

(c) Add the helper (mirrors the Rust `is_armor_id`) at the bottom of the file:

```dart
/// True for any armor class name (base, per-NPC, or tier piece). Mirrors the
/// Rust `gore_catalog::is_armor_id` display-side classifier.
bool _isArmorId(String id) {
  if (!id.contains('Armor')) return false;
  if (id.startsWith('Armor_')) return true;
  final parts = id.split('_');
  if (parts.length < 2) return false;
  final head = parts.first;
  return head.length >= 2 &&
      head.length <= 4 &&
      RegExp(r'^[A-Za-z]+$').hasMatch(head) &&
      parts[1].startsWith('Armor');
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/save-editor && flutter test test/features/editor/domain/item_categories_test.dart`
Expected: FAIL to COMPILE — `localizedItemCategoryLabel` and `iconForItemCategory` switches are now non-exhaustive. That is fixed in Tasks 7-8. (If you prefer a green checkpoint here, do Tasks 7 and 8 before re-running.)

- [ ] **Step 5: Commit (after Tasks 7-8 make it compile — see note)**

This task's commit is combined with Tasks 7-8 (they must land together for the app to compile). Proceed to Task 7.

---

## Task 7: Add the `itemCategoryArmor` localization key

**Files:**
- Modify: `apps/save-editor/lib/l10n/app_en.arb` (+ the other 11 `app_*.arb` files)
- Regenerated: `apps/save-editor/lib/l10n/app_localizations*.dart`
- Modify: `apps/save-editor/lib/features/editor/domain/item_categories.dart` (`localizedItemCategoryLabel`)

- [ ] **Step 1: Add the key to every ARB file**

Add `"itemCategoryArmor"` next to `"itemCategoryAmmunition"` in each file. Use these values:

- `app_en.arb`: `"itemCategoryArmor": "Armor",`
- `app_de.arb`: `"itemCategoryArmor": "Rüstungen",`
- `app_es.arb`: `"itemCategoryArmor": "Armaduras",`
- `app_fr.arb`: `"itemCategoryArmor": "Armures",`
- `app_it.arb`: `"itemCategoryArmor": "Armature",`
- `app_ja.arb`: `"itemCategoryArmor": "鎧",`
- `app_pl.arb`: `"itemCategoryArmor": "Zbroje",`
- `app_pt.arb`: `"itemCategoryArmor": "Armaduras",`
- `app_pt_BR.arb`: `"itemCategoryArmor": "Armaduras",`
- `app_ru.arb`: `"itemCategoryArmor": "Броня",`
- `app_zh.arb`: `"itemCategoryArmor": "护甲",`
- `app_zh_Hans.arb`: `"itemCategoryArmor": "护甲",`

(Match the exact JSON formatting/trailing-comma style of each file. ARB entries with no placeholders need no metadata block.)

- [ ] **Step 2: Regenerate localizations**

Run: `cd apps/save-editor && flutter gen-l10n`
Expected: regenerates `app_localizations*.dart` with a `String get itemCategoryArmor;` getter and per-locale overrides. No errors.

- [ ] **Step 3: Wire the label switch**

In `apps/save-editor/lib/features/editor/domain/item_categories.dart`, add to `localizedItemCategoryLabel`'s switch, next to the ammunition case:

```dart
    ItemCategory.armor => l10n.itemCategoryArmor,
```

- [ ] **Step 4: Verify (defer running until Task 8 completes the other switch)**

Proceed to Task 8; the analyzer will be green once both switches are exhaustive.

---

## Task 8: Add the armor icon

**Files:**
- Modify: `apps/save-editor/lib/features/editor/ui/sidebar_tile.dart` (`iconForItemCategory`)

- [ ] **Step 1: Add the icon case**

In `apps/save-editor/lib/features/editor/ui/sidebar_tile.dart`, add to `iconForItemCategory`'s switch, next to the weapon cases:

```dart
    ItemCategory.armor => Icons.shield_outlined,
```

- [ ] **Step 2: Analyze + run the domain tests**

Run:

```bash
cd apps/save-editor && flutter analyze lib/features/editor/domain/item_categories.dart lib/features/editor/ui/sidebar_tile.dart && flutter test test/features/editor/domain/item_categories_test.dart
```

Expected: analyze clean (both switches exhaustive); the armor categorization test passes.

- [ ] **Step 3: Commit Tasks 6-8 together**

```bash
git add apps/save-editor/lib/features/editor/domain/item_categories.dart \
        apps/save-editor/lib/features/editor/ui/sidebar_tile.dart \
        apps/save-editor/lib/l10n/
git commit -m "feat(save-editor): show armor under its own Armor category"
```

---

## Task 9: Full verification

- [ ] **Step 1: Rust suites**

Run: `cargo test -p gore-catalog && cargo test -p gore-save`
Expected: PASS.

- [ ] **Step 2: Real-payload armor add**

Run:

```bash
GORESAVE_PAYLOAD_BIN="work/decompressed/G1R-016.host.bin" \
  cargo test -p gore-save add_armor_item_roundtrips_on_real_payload -- --ignored
```

Expected: PASS.

- [ ] **Step 3: Flutter analyze + tests**

Run: `cd apps/save-editor && flutter analyze && flutter test`
Expected: analyze clean; tests pass.

- [ ] **Step 4: Manual smoke (optional, real app)**

Launch the save editor, open the user's `G1R-019.sav`. Expected: an **Armor** category (shield icon) in the inventory sidebar listing `Ore_Armor_H`, `Crw_Armor_H`, `Ore_Armor_M`; the Add-item dialog has an Armor category listing addable armors. (Equipped badge + equip/unequip are Tier B; upgrades are Tier C.)

---

## Self-Review notes

- **Spec coverage (Tier A):** catalog discriminator (Task 1), armor category in JSON (Task 2) + model (Task 3), regen (Task 4), addable verified (Task 5), UI category/label/icon (Tasks 6-8). Tier A's "equipped indicator" is intentionally deferred to Tier B (it requires the same container-aware core work as equip/unequip).
- **Two classifiers, on purpose:** `is_armor_item_class` (catalog/addable — excludes tier pieces) vs `is_armor_id` (display — includes tier pieces). Names differ; both documented.
- **No new core display code in Tier A** — armor rows already return from `summarize_private_inventory_items` (verified via `inspect_save` probe on G1R-019).
- **Catalog regen has no snapshot-gate test** — Task 4 is a manual artifact regen; the only tests that move are the two `Armor_OC_Gomez` assertions (Task 3).

---

## Roadmap (subsequent plans)

- **Tier B — Equipped indicator + equip/unequip:** core surfaces each row's `EInventoryTypes` container (so the UI can flag the `ArmorSlot` item as equipped); new `private.inventory.equipArmor` / `unequipArmor` edit op that relocates a slot between `MainContainer` and `ArmorSlot` `m_Slots` (single-occupancy, swap on replace), reusing the `donor_slot_template_bytes` / `ArrayInsertBytes` + `ArrayRemove` splice machinery and extending the standalone-structural-edit batch guards. UI: equipped badge + equip/unequip control. Authored as its own plan after Tier A lands.
- **Tier C — Armor upgrades:** read/edit the worn armor slot's `m_Payload.m_GenericData` string-map (`m_Current{Upper,Mid,Lower}BodyUpgrade`). UI upgrade editor. Authored after Tier B.
