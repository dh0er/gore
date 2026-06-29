# Armor Support — Tier B1 (Equipped Indicator) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Show which armor is currently equipped (worn) in the inventory — the armor that lives in the player's `ArmorSlot` container is flagged with an "Equipped" badge.

**Architecture:** The player inventory is a `TMap<EInventoryTypes, Container>`. The worn armor is the item in the `ArmorSlot` container's `m_Slots`; carried items are in `MainContainer`. The core already returns all armor rows; this plan adds container-awareness: a typed-tree helper `armor_slot_summary` collects the equipped armor paths, the inventory summary marks each row `equipped`, and the Flutter UI renders a badge. Read-only — no save edits (equip/unequip is Tier B2).

**Tech Stack:** Rust (`gore-save`), `cargo test`; Flutter/Dart (`apps/save-editor`), `flutter test` + `flutter analyze`.

**Spec:** `docs/superpowers/specs/2026-06-29-inventory-armor-support-design.md`

**Fixtures:** `work/decompressed/G1R-019.host.bin` — ArmorSlot=`Ore_Armor_H` (equipped), MainContainer=`Crw_Armor_H`,`Ore_Armor_M`. Use ABSOLUTE paths for `GORESAVE_PAYLOAD_BIN` (cargo's cwd differs from the worktree root).

---

## File Structure

- `crates/gore-save/src/lib.rs` — add `ARMOR_SLOT_ENUM_LABEL` const, `ArmorSlotSummary` struct, `armor_slot_summary` fn (mirrors `main_container_summary`); thread an `armor_slot: Option<&ArmorSlotSummary>` param into `summarize_private_inventory_payload` and mark each row `equipped`; compute it at the inspect call site (~line 2820).
- `apps/save-editor/lib/features/editor/domain/editor_models.dart` — `PrivateInventoryItem.equipped` field + `fromJson`.
- `apps/save-editor/lib/features/editor/ui/editor_page.dart` — equipped badge in the inventory item row.
- Tests: `crates/gore-save/src/lib.rs` (real-payload), `apps/save-editor/test/...` (model + optional widget).

---

## Task 1: `armor_slot_summary` — collect equipped armor paths (core)

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (add const + struct + fn near `main_container_summary` ~line 5639; `MAIN_CONTAINER_ENUM_LABEL` is at ~5509)
- Test: `crates/gore-save/src/lib.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing real-payload test**

Add to the `#[cfg(test)] mod tests` block (near the other `#[ignore]` real-payload tests):

```rust
    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn armor_slot_summary_finds_equipped_armor() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let payload = std::fs::read(path).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        let summary = armor_slot_summary(&root).expect("armor slot container resolves");
        // G1R-019: the worn armor is Ore_Armor_H; the other two are in the bag.
        assert!(summary.equipped_paths.contains("/Script/Angelscript.Ore_Armor_H"));
        assert!(!summary.equipped_paths.contains("/Script/Angelscript.Crw_Armor_H"));
        assert!(!summary.equipped_paths.contains("/Script/Angelscript.Ore_Armor_M"));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-019.host.bin" cargo test -p gore-save armor_slot_summary_finds_equipped_armor -- --ignored`
Expected: FAIL — `cannot find function `armor_slot_summary``.

- [ ] **Step 3: Implement**

Add near `MAIN_CONTAINER_ENUM_LABEL` (the `const` ~line 5509):

```rust
/// Enum label of the player's worn-armor container in the inventory map.
const ARMOR_SLOT_ENUM_LABEL: &str = "EInventoryTypes::ArmorSlot";
```

Add near `main_container_summary` (mirror its container-resolution shape):

```rust
/// The item-definition paths currently in the player's `ArmorSlot` container —
/// i.e. the worn armor. At most one in practice, modeled as a set for
/// robustness. `None` when the inventory or the ArmorSlot container is absent.
struct ArmorSlotSummary {
    equipped_paths: std::collections::HashSet<String>,
}

/// Resolve the player's `ArmorSlot` container and collect the item-definition
/// paths it holds. Mirrors `main_container_summary`'s container resolution.
fn armor_slot_summary(root: &properties::RootObject) -> Option<ArmorSlotSummary> {
    let inventory_path = resolve_inventory_path(root)?;
    let resolve_child = |suffix: &[&str]| -> Option<properties::PropertyValue> {
        let mut segs = inventory_path.clone();
        segs.extend(suffix.iter().map(|s| s.to_string()));
        let parsed = properties::parse_path(&segs).ok()?;
        properties::resolve(&root.properties, &parsed)
            .ok()
            .map(|prop| prop.value.clone())
    };
    let properties::PropertyValue::Array { elements: keys } = resolve_child(&["m_Keys"])? else {
        return None;
    };
    let index = keys.iter().position(|element| {
        matches!(element, properties::PropertyValue::Enum(label)
            if label == ARMOR_SLOT_ENUM_LABEL)
    })?;
    let segment = format!("[{index}]");
    let properties::PropertyValue::Array { elements: slots } =
        resolve_child(&["m_Values", "Items", &segment, "m_Slots"])?
    else {
        return None;
    };
    let mut equipped_paths = std::collections::HashSet::new();
    for slot in &slots {
        if let Some(path) = slot_item_definition(slot) {
            if !path.is_empty() {
                equipped_paths.insert(path.to_string());
            }
        }
    }
    Some(ArmorSlotSummary { equipped_paths })
}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-019.host.bin" cargo test -p gore-save armor_slot_summary_finds_equipped_armor -- --ignored`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(gore-save): resolve the equipped-armor (ArmorSlot) container"
```

---

## Task 2: Mark inventory rows `equipped` in the summary (core)

**Files:**
- Modify: `crates/gore-save/src/lib.rs` — `summarize_private_inventory_payload` signature + the per-row marking loop (~line 3009); the inspect call site (~line 2820-2825)
- Test: `crates/gore-save/src/lib.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing real-payload test**

```rust
    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn inventory_rows_flag_equipped_armor() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let payload = std::fs::read(path).unwrap();
        let refs = scan_fstrings(&payload, 0);
        let root = properties::parse_private_root(&payload).unwrap();
        let main_container = main_container_summary(&root);
        let armor_slot = armor_slot_summary(&root);
        let inv = summarize_private_inventory_payload(
            &payload,
            &refs,
            main_container.as_ref(),
            armor_slot.as_ref(),
        );
        let items = inv["items"].as_array().unwrap();
        let equipped: Vec<&str> = items
            .iter()
            .filter(|i| i["equipped"].as_bool() == Some(true))
            .map(|i| i["path"].as_str().unwrap_or(""))
            .collect();
        // G1R-019: exactly the worn Ore_Armor_H is flagged equipped.
        assert!(equipped.contains(&"/Script/Angelscript.Ore_Armor_H"));
        assert!(!equipped.contains(&"/Script/Angelscript.Crw_Armor_H"));
        // a non-armor row is never equipped
        assert!(items.iter().all(|i| {
            let p = i["path"].as_str().unwrap_or("");
            !p.contains("ItMi_") || i["equipped"].as_bool() == Some(false)
        }));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-019.host.bin" cargo test -p gore-save inventory_rows_flag_equipped_armor -- --ignored`
Expected: FAIL to COMPILE — `summarize_private_inventory_payload` takes 3 args, not 4.

- [ ] **Step 3: Implement**

(a) Change the signature of `summarize_private_inventory_payload` (~line 2979) to add the param:

```rust
fn summarize_private_inventory_payload(
    payload: &[u8],
    refs: &[FStringRef],
    main_container: Option<&MainContainerSummary>,
    armor_slot: Option<&ArmorSlotSummary>,
) -> Value {
```

(b) In the per-row loop (~line 3009, right after the `removable` marking), add:

```rust
    for item in &mut items {
        let path = item["path"].as_str().unwrap_or("");
        item["equipped"] = json!(
            !path.is_empty()
                && armor_slot.is_some_and(|a| a.equipped_paths.contains(path))
        );
    }
```

(Keep this as a second loop after the existing `removable` loop, or fold the assignment into the existing loop — either is fine; do not disturb the `removable` logic.)

(c) At the inspect call site (~line 2820), compute `armor_slot` alongside `main_container` and pass it:

```rust
            let main_container = typed_result
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(main_container_summary);
            let armor_slot = typed_result
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(armor_slot_summary);
            let inventory = summarize_private_inventory_payload(
                &payload,
                &refs,
                main_container.as_ref(),
                armor_slot.as_ref(),
            );
```

- [ ] **Step 4: Fix any other call sites**

Run: `cargo build -p gore-save`. If any other caller of `summarize_private_inventory_payload` exists (e.g. an existing unit test), update it to pass `None` for the new `armor_slot` argument. Report any such site.

- [ ] **Step 5: Run to verify it passes**

Run: `GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-019.host.bin" cargo test -p gore-save inventory_rows_flag_equipped_armor -- --ignored`
Expected: PASS.

- [ ] **Step 6: Run the whole crate suite (no regressions)**

Run: `cargo test -p gore-save`
Expected: PASS (181+ tests; the two new ones are `#[ignore]`-gated).

- [ ] **Step 7: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(gore-save): flag equipped armor rows in the inventory summary"
```

---

## Task 3: `PrivateInventoryItem.equipped` (Dart model)

**Files:**
- Modify: `apps/save-editor/lib/features/editor/domain/editor_models.dart` (`PrivateInventoryItem` ~line 674)
- Test: `apps/save-editor/test/features/editor/domain/editor_models_test.dart` (create if absent, else add to the existing inventory model test)

- [ ] **Step 1: Write the failing test**

Add (in the appropriate existing test file for `PrivateInventoryItem`, or create `editor_models_test.dart` with the standard `flutter_test` import):

```dart
  test('PrivateInventoryItem parses equipped flag', () {
    final equipped = PrivateInventoryItem.fromJson({
      'id': 'Ore_Armor_H',
      'path': '/Script/Angelscript.Ore_Armor_H',
      'count': 1,
      'equipped': true,
    });
    expect(equipped.equipped, isTrue);

    final plain = PrivateInventoryItem.fromJson({
      'id': 'ItMi_Orenugget',
      'path': '/Script/Angelscript.ItMi_Orenugget',
      'count': 5,
    });
    expect(plain.equipped, isFalse); // defaults false when absent
  });
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd apps/save-editor && flutter test test/features/editor/domain/editor_models_test.dart`
Expected: FAIL — `equipped` getter not defined.

- [ ] **Step 3: Implement**

In `PrivateInventoryItem` (~line 674), add the field, constructor param, and fromJson line, mirroring the existing `removable` handling:

```dart
  const PrivateInventoryItem({
    required this.id,
    required this.path,
    this.count,
    this.removable = false,
    this.equipped = false,
  });

  factory PrivateInventoryItem.fromJson(Map<Object?, Object?> json) {
    return PrivateInventoryItem(
      id: json['id'] as String? ?? '',
      path: json['path'] as String? ?? '',
      count: (json['count'] as num?)?.toInt(),
      removable: json['removable'] as bool? ?? false,
      equipped: json['equipped'] as bool? ?? false,
    );
  }

  final String id;
  final String path;
  final int? count;
  final bool removable;
  /// True for the worn armor (the item in the player's ArmorSlot container).
  final bool equipped;
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd apps/save-editor && flutter test test/features/editor/domain/editor_models_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/domain/editor_models.dart apps/save-editor/test/features/editor/domain/editor_models_test.dart
git commit -m "feat(save-editor): parse the equipped flag on inventory items"
```

---

## Task 4: Equipped badge in the inventory row (Dart UI)

**Files:**
- Modify: `apps/save-editor/lib/features/editor/ui/editor_page.dart` — the inventory `ListTile` (~line 1940-1992) / `_inventoryItemTrailing` (~line 2016)
- Modify: `apps/save-editor/lib/l10n/app_*.arb` (12) + regenerate — new `equippedBadge` label
- Test: `apps/save-editor/test/widget_test.dart` (extend existing inventory widget test if practical)

- [ ] **Step 1: Add the localized label**

Add `"equippedBadge"` to each of the 12 `app_*.arb` files near other inventory labels:
- en: `"equippedBadge": "Equipped",`
- de: `"equippedBadge": "Angelegt",`
- es: `"equippedBadge": "Equipado",`
- fr: `"equippedBadge": "Équipé",`
- it: `"equippedBadge": "Equipaggiato",`
- ja: `"equippedBadge": "装備中",`
- pl: `"equippedBadge": "Założone",`
- pt: `"equippedBadge": "Equipado",`
- pt_BR: `"equippedBadge": "Equipado",`
- ru: `"equippedBadge": "Надето",`
- zh: `"equippedBadge": "已装备",`
- zh_Hans: `"equippedBadge": "已装备",`

Then: `cd apps/save-editor && flutter gen-l10n` (expect `String get equippedBadge;` generated, no errors).

- [ ] **Step 2: Render the badge**

In the inventory `ListTile` building (the row in `_PrivateInventorySummaryCard.build`, around the `ListTile` with `title:` from `localizedGameName(...) ?? item.id`), add a small badge when `item.equipped`. Add it to the `title` row as a trailing `Chip`/`Container`, e.g. wrap the title:

```dart
title: Row(
  children: [
    Flexible(child: Text(localizedGameName(context, item.id) ?? item.id)),
    if (item.equipped) ...[
      const SizedBox(width: 8),
      Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.primaryContainer,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          AppLocalizations.of(context)!.equippedBadge,
          style: Theme.of(context).textTheme.labelSmall,
        ),
      ),
    ],
  ],
),
```

(Match the exact existing `title:` expression and surrounding widget style in editor_page.dart; the above shows the intended structure — adapt to how the row currently builds its title. Keep the existing subtitle/trailing untouched.)

- [ ] **Step 3: Analyze + test**

Run: `cd apps/save-editor && flutter analyze lib && flutter test`
Expected: analyze clean; all tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/editor_page.dart apps/save-editor/lib/l10n/
git commit -m "feat(save-editor): show an Equipped badge on the worn armor row"
```

---

## Task 5: Full verification

- [ ] **Step 1: Rust**

Run: `cargo test -p gore-save` and the two ignored tests with the absolute fixture path:
```bash
GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-019.host.bin" \
  cargo test -p gore-save armor_slot_summary_finds_equipped_armor inventory_rows_flag_equipped_armor -- --ignored
```
Expected: all PASS.

- [ ] **Step 2: Flutter**

Run: `cd apps/save-editor && flutter analyze lib && flutter test`
Expected: analyze clean; tests pass.

- [ ] **Step 3: Manual smoke (optional)**

Open `G1R-019.sav`: the Armor category shows `Ore_Armor_H` with an "Equipped" badge; `Crw_Armor_H` and `Ore_Armor_M` without.

---

## Self-Review notes

- `armor_slot_summary` is a faithful structural mirror of `main_container_summary` (same `resolve_child` + enum-label-match pattern) — only the enum label and the collected field differ.
- `equipped` defaults `false` everywhere (absent JSON, non-armor rows, saves with no ArmorSlot container) — purely additive, no behavior change for existing rows.
- Read-only: no new edit ops, no `writable` changes. Equip/unequip is Tier B2.
- Duplicate-path edge case: if the same armor class is both worn and in the bag, both rows flag equipped (path-based). Acceptable for an indicator; Tier B2's equip op operates per-container, not per-row-path.
