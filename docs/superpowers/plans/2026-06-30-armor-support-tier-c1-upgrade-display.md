# Armor Support — Tier C1 (Upgrade Display) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Show the worn armor's applied upgrades (read-only) — e.g. the equipped armor displays "Upper: Heavy02, Mid: Heavy02, Lower: Heavy02", or nothing when un-upgraded.

**Architecture:** The worn armor (the single item in the `ArmorSlot` container) carries its upgrade state in `m_Payload.m_GenericData`, a `ReplicatedStringMap` struct with parallel arrays: `m_Keys` (`m_CurrentUpperBodyUpgrade`/`m_CurrentMidBodyUpgrade`/`m_CurrentLowerBodyUpgrade`) and `m_Values` (e.g. `m_UpperBody_Heavy02_ArmorUpgrade`, or empty = none). The core reads these arrays from the typed tree and attaches them to the equipped armor's inventory row; the UI renders chips. Read-only (editing upgrades = Tier C2, which needs a new array-element-replace patch primitive — out of scope here).

**Tech Stack:** Rust (`gore-save`); Flutter/Dart (`apps/save-editor`).

**Spec:** `docs/superpowers/specs/2026-06-29-inventory-armor-support-design.md`

**Fixtures (ABSOLUTE paths for `GORESAVE_PAYLOAD_BIN`):**
- `C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-020.host.bin` — worn `Org_Armor` WITH upgrades (all three `Heavy02`).
- `C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-016.host.bin` — worn `Org_Armor` with NO upgrades (empty values).

---

## File Structure

- `crates/gore-save/src/lib.rs` — extend `ArmorSlotSummary` with the worn armor's upgrade pairs; a helper `slot_upgrade_pairs(slot)` that walks `m_Payload.m_GenericData.m_Keys`/`m_Values`; attach an `upgrades` field to the equipped inventory row in `summarize_private_inventory_payload`.
- `apps/save-editor/lib/features/editor/domain/editor_models.dart` — `PrivateInventoryItem.upgrades` (a `List<ArmorUpgrade>` or `Map<String,String>`).
- `apps/save-editor/lib/features/editor/ui/editor_page.dart` — render upgrade chips on the equipped armor row.
- Tests: `crates/gore-save/src/lib.rs` (real-payload, 020 + 016), `apps/save-editor/test/...` (model).

---

## Task 1: Read worn-armor upgrade pairs (core)

**Files:**
- Modify: `crates/gore-save/src/lib.rs` — `ArmorSlotSummary`, `armor_slot_summary`, new helper `slot_upgrade_pairs`
- Test: `crates/gore-save/src/lib.rs` `#[cfg(test)] mod tests`

The worn armor's `m_Payload.m_GenericData` is a `ReplicatedStringMap` struct with `m_Keys` (Array of `Name`) and `m_Values` (Array of `Str`). Pair them positionally; keep only entries whose value is non-empty. Helper walks the typed slot value via `struct_element_property` (which returns a `&Property` from a struct element) and reads the two arrays.

- [ ] **Step 1: Write the failing real-payload tests**

```rust
    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn armor_upgrades_read_from_worn_armor() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let payload = std::fs::read(path).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        let summary = armor_slot_summary(&root).expect("armor slot resolves");
        // G1R-020: worn Org_Armor has all three Heavy02 upgrades.
        let ups = &summary.upgrades;
        let find = |k: &str| ups.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
        assert_eq!(find("m_CurrentUpperBodyUpgrade"), Some("m_UpperBody_Heavy02_ArmorUpgrade"));
        assert_eq!(find("m_CurrentMidBodyUpgrade"), Some("m_MidBody_Heavy02_ArmorUpgrade"));
        assert_eq!(find("m_CurrentLowerBodyUpgrade"), Some("m_LowerBody_Heavy02_ArmorUpgrade"));
    }

    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn armor_upgrades_empty_when_not_upgraded() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let payload = std::fs::read(path).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        let summary = armor_slot_summary(&root).expect("armor slot resolves");
        // G1R-016: worn Org_Armor has empty upgrade values -> no non-empty pairs.
        assert!(summary.upgrades.is_empty(), "expected no upgrades, got {:?}", summary.upgrades);
    }
```

- [ ] **Step 2: Run to verify they fail**

Run (020 case):
`GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-020.host.bin" cargo test -p gore-save armor_upgrades_read_from_worn_armor -- --ignored`
Expected: FAIL — `ArmorSlotSummary` has no `upgrades` field.

- [ ] **Step 3: Implement**

(a) Add a field to `ArmorSlotSummary`:

```rust
struct ArmorSlotSummary {
    equipped_paths: std::collections::HashSet<String>,
    /// `(key, value)` upgrade pairs from the worn armor's `m_GenericData`,
    /// non-empty values only (e.g. `("m_CurrentUpperBodyUpgrade",
    /// "m_UpperBody_Heavy02_ArmorUpgrade")`). Empty when not upgraded or no
    /// armor is worn.
    upgrades: Vec<(String, String)>,
}
```

(b) Add a helper that extracts the non-empty upgrade pairs from one slot's payload. Place it near `slot_item_definition`:

```rust
/// Read the non-empty `(key, value)` upgrade pairs from a slot's
/// `m_Payload.m_GenericData` ReplicatedStringMap (parallel `m_Keys`/`m_Values`
/// arrays). Returns empty when the slot has no payload map or all values empty.
fn slot_upgrade_pairs(slot: &PropertyValue) -> Vec<(String, String)> {
    let Some(payload) = struct_element_property(slot, "m_Payload") else {
        return Vec::new();
    };
    let PropertyValue::Struct(properties::StructValue::Properties(payload_props)) = &payload.value
    else {
        return Vec::new();
    };
    let Some(generic) = payload_props.iter().find(|p| p.name == "m_GenericData") else {
        return Vec::new();
    };
    let PropertyValue::Struct(properties::StructValue::Properties(map_props)) = &generic.value
    else {
        return Vec::new();
    };
    let array_strings = |name: &str| -> Vec<String> {
        map_props
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| match &p.value {
                PropertyValue::Array { elements } => Some(
                    elements
                        .iter()
                        .map(|e| match e {
                            PropertyValue::Name(s) | PropertyValue::Str(s) => s.clone(),
                            _ => String::new(),
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    };
    let keys = array_strings("m_Keys");
    let values = array_strings("m_Values");
    keys.into_iter()
        .zip(values)
        .filter(|(_, v)| !v.is_empty())
        .collect()
}
```

(c) In `armor_slot_summary`, after collecting `equipped_paths`, also collect upgrades from the first armor slot that has any. Change the slot loop to capture upgrades:

```rust
    let mut equipped_paths = std::collections::HashSet::new();
    let mut upgrades = Vec::new();
    for slot in &slots {
        if let Some(path) = slot_item_definition(slot) {
            if !path.is_empty() {
                equipped_paths.insert(path.to_string());
            }
        }
        if upgrades.is_empty() {
            upgrades = slot_upgrade_pairs(slot);
        }
    }
    Some(ArmorSlotSummary { equipped_paths, upgrades })
```

Confirm `properties::StructValue` and `PropertyValue::{Name,Str,Array}` are imported/in-scope (the file already uses `PropertyValue` and `properties::` extensively; add a `use` only if the helper doesn't compile).

- [ ] **Step 4: Run to verify both tests pass**

Run 020: `GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-020.host.bin" cargo test -p gore-save armor_upgrades_read_from_worn_armor -- --ignored` → PASS
Run 016: `GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-016.host.bin" cargo test -p gore-save armor_upgrades_empty_when_not_upgraded -- --ignored` → PASS

- [ ] **Step 5: Build (the new struct field may be unused until Task 2)**

Run: `cargo build -p gore-save`. If a `field never read` warning blocks the build, proceed to Task 2 (which consumes it) or add `#[allow(dead_code)]` temporarily and remove in Task 2.

- [ ] **Step 6: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(gore-save): read worn-armor upgrade pairs from m_GenericData"
```

---

## Task 2: Attach `upgrades` to the equipped armor row (core)

**Files:**
- Modify: `crates/gore-save/src/lib.rs` — `summarize_private_inventory_payload` (the `equipped` loop)
- Test: `crates/gore-save/src/lib.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing real-payload test**

```rust
    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn equipped_row_carries_upgrades() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let payload = std::fs::read(path).unwrap();
        let refs = scan_fstrings(&payload, 0);
        let root = properties::parse_private_root(&payload).unwrap();
        let main_container = main_container_summary(&root);
        let armor_slot = armor_slot_summary(&root);
        let inv = summarize_private_inventory_payload(
            &payload, &refs, main_container.as_ref(), armor_slot.as_ref(),
        );
        let items = inv["items"].as_array().unwrap();
        // G1R-020: the equipped Org_Armor row carries 3 upgrade entries.
        let worn = items
            .iter()
            .find(|i| i["equipped"].as_bool() == Some(true))
            .expect("an equipped row");
        let ups = worn["upgrades"].as_array().expect("upgrades array");
        assert_eq!(ups.len(), 3);
        assert!(ups.iter().any(|u| u["value"].as_str() == Some("m_UpperBody_Heavy02_ArmorUpgrade")));
        // a non-equipped row has an empty upgrades array
        let other = items.iter().find(|i| i["equipped"].as_bool() != Some(true)).unwrap();
        assert_eq!(other["upgrades"].as_array().map(|a| a.len()), Some(0));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run with 020 path. Expected: FAIL — no `upgrades` key on rows.

- [ ] **Step 3: Implement**

In `summarize_private_inventory_payload`, in the loop that sets `equipped`, also set `upgrades` — the upgrade pairs only on the equipped row, an empty array otherwise:

```rust
    for item in &mut items {
        let path = item["path"].as_str().unwrap_or("");
        let is_equipped = !path.is_empty()
            && armor_slot.is_some_and(|a| a.equipped_paths.contains(path));
        item["equipped"] = json!(is_equipped);
        item["upgrades"] = if is_equipped {
            json!(armor_slot
                .map(|a| a
                    .upgrades
                    .iter()
                    .map(|(k, v)| json!({ "key": k, "value": v }))
                    .collect::<Vec<_>>())
                .unwrap_or_default())
        } else {
            json!([])
        };
    }
```

(Replace the existing `equipped`-only loop body with this; keep it a separate loop after the `removable` loop.)

- [ ] **Step 4: Update golden tests**

Run `cargo test -p gore-save`. Rows now always carry an `upgrades: []` field; the three golden-comparison tests updated for `equipped` (`write_save_updates_private_inventory_item_count_by_id`, `inspect_save_prefers_item_stacks_inside_private_inventory_region`, `inspect_save_reports_private_inventory_summary`) will need `"upgrades": []` added next to their `"equipped": false`. Add ONLY that field. Report which tests you touched.

- [ ] **Step 5: Run to verify**

Run the new test with 020 → PASS. Run full `cargo test -p gore-save` → all green.

- [ ] **Step 6: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(gore-save): attach upgrades to the equipped armor inventory row"
```

---

## Task 3: `PrivateInventoryItem.upgrades` (Dart model)

**Files:**
- Modify: `apps/save-editor/lib/features/editor/domain/editor_models.dart`
- Test: `apps/save-editor/test/features/editor/domain/editor_models_test.dart`

- [ ] **Step 1: Write the failing test**

```dart
  test('PrivateInventoryItem parses armor upgrades', () {
    final item = PrivateInventoryItem.fromJson({
      'id': 'Org_Armor',
      'path': '/Script/Angelscript.Org_Armor',
      'count': 1,
      'equipped': true,
      'upgrades': [
        {'key': 'm_CurrentUpperBodyUpgrade', 'value': 'm_UpperBody_Heavy02_ArmorUpgrade'},
      ],
    });
    expect(item.upgrades.length, 1);
    expect(item.upgrades.first.key, 'm_CurrentUpperBodyUpgrade');
    expect(item.upgrades.first.value, 'm_UpperBody_Heavy02_ArmorUpgrade');

    final plain = PrivateInventoryItem.fromJson({'id': 'X', 'path': 'p'});
    expect(plain.upgrades, isEmpty);
  });
```

- [ ] **Step 2: Run → fail** (`cd apps/save-editor && flutter test test/features/editor/domain/editor_models_test.dart`).

- [ ] **Step 3: Implement**

Add a small value type and the field. Near `PrivateInventoryItem`:

```dart
class ArmorUpgrade {
  const ArmorUpgrade({required this.key, required this.value});
  final String key;
  final String value;
}
```

In `PrivateInventoryItem`: constructor param `this.upgrades = const [],`; field `final List<ArmorUpgrade> upgrades;`; in `fromJson`:

```dart
      upgrades: (json['upgrades'] as List?)
              ?.whereType<Map<Object?, Object?>>()
              .map((u) => ArmorUpgrade(
                    key: u['key'] as String? ?? '',
                    value: u['value'] as String? ?? '',
                  ))
              .toList() ??
          const [],
```

- [ ] **Step 4: Run → pass.**

- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/domain/editor_models.dart apps/save-editor/test/features/editor/domain/editor_models_test.dart
git commit -m "feat(save-editor): parse armor upgrades on inventory items"
```

---

## Task 4: Render upgrade chips on the equipped armor row (Dart UI)

**Files:**
- Modify: `apps/save-editor/lib/features/editor/ui/editor_page.dart`
- Modify: `apps/save-editor/lib/l10n/app_*.arb` (12) + regenerate — `armorUpgradesLabel`

- [ ] **Step 1: Add l10n label**

Add `armorUpgradesLabel` to all 12 arb files (en "Upgrades", de "Verbesserungen", es "Mejoras", fr "Améliorations", it "Potenziamenti", ja "強化", pl "Ulepszenia", pt "Melhorias", pt_BR "Melhorias", ru "Улучшения", zh "升级", zh_Hans "升级"). Then `cd apps/save-editor && flutter gen-l10n`.

- [ ] **Step 2: Render in the subtitle**

In the inventory `ListTile` (the same one with the Equipped badge), when `item.upgrades.isNotEmpty`, extend the `subtitle` to show the upgrade tiers. Prettify each value: strip the `m_<Part>Body_` prefix and `_ArmorUpgrade` suffix, leaving e.g. `Heavy02`; label the part from the key (`Upper`/`Mid`/`Lower`). Build a `Wrap` of small `Chip`s under the existing subtitle path. Keep the existing subtitle (path) and add the chips below it via a `Column`:

```dart
subtitle: Column(
  crossAxisAlignment: CrossAxisAlignment.start,
  mainAxisSize: MainAxisSize.min,
  children: [
    Text(item.path), // or the existing subtitle expression, preserved verbatim
    if (item.upgrades.isNotEmpty)
      Padding(
        padding: const EdgeInsets.only(top: 4),
        child: Wrap(
          spacing: 4,
          runSpacing: 2,
          children: [
            for (final u in item.upgrades)
              Chip(
                visualDensity: VisualDensity.compact,
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                label: Text('${_upgradePart(u.key)}: ${_upgradeTier(u.value)}'),
              ),
          ],
        ),
      ),
  ],
),
```

Add two top-level prettifier helpers in editor_page.dart (or a small private util):

```dart
String _upgradePart(String key) {
  if (key.contains('Upper')) return 'Upper';
  if (key.contains('Mid')) return 'Mid';
  if (key.contains('Lower')) return 'Lower';
  return key;
}

String _upgradeTier(String value) {
  // m_UpperBody_Heavy02_ArmorUpgrade -> Heavy02
  var v = value;
  for (final p in ['m_UpperBody_', 'm_MidBody_', 'm_LowerBody_']) {
    if (v.startsWith(p)) { v = v.substring(p.length); break; }
  }
  return v.replaceAll('_ArmorUpgrade', '');
}
```

IMPORTANT: read the ACTUAL current `subtitle:` expression first and preserve it verbatim as the first child of the Column. Do not disturb the title (Equipped badge) or trailing.

- [ ] **Step 3: Analyze + test**

Run: `cd apps/save-editor && flutter analyze lib && flutter test`
Expected: analyze clean; all tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/editor_page.dart apps/save-editor/lib/l10n/
git commit -m "feat(save-editor): show armor upgrade tiers on the equipped row"
```

---

## Task 5: Full verification

- [ ] **Step 1: Rust** — both fixtures:
```bash
GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-020.host.bin" cargo test -p gore-save armor_upgrades_read_from_worn_armor equipped_row_carries_upgrades -- --ignored
GORESAVE_PAYLOAD_BIN="C:/sbx/goresave/.claude/worktrees/gracious-tu-652deb/work/decompressed/G1R-016.host.bin" cargo test -p gore-save armor_upgrades_empty_when_not_upgraded -- --ignored
cargo test -p gore-save
```
Expected: all PASS.

- [ ] **Step 2: Flutter** — `cd apps/save-editor && flutter analyze lib && flutter test` → clean + pass.

- [ ] **Step 3: Manual smoke (optional)** — open `G1R-020.sav`: the equipped `Org_Armor` shows three upgrade chips (Upper/Mid/Lower: Heavy02). `G1R-016.sav`: equipped armor, no chips.

---

## Self-Review notes

- Read-only — no edit ops, no `writable` changes. Editing upgrades (C2) needs a new array-element-replace patch primitive (`resolve` currently refuses to end on an array element, properties.rs:316) and is a separate plan.
- `upgrades` is additive on every row (empty array for non-equipped) — same pattern as `equipped`/`removable`; golden tests updated accordingly.
- Core emits raw `key`/`value`; the UI does the prettifying (Upper/Mid/Lower + tier), keeping the core dumb and stable.
