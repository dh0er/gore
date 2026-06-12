# Inventory Categorization + Add-Item Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Group the entire inventory view by item category, ship a bundled searchable item catalog, and add a backup-first `private.inventory.addItem` write op so users can add items not present in the save.

**Architecture:** Pure Dart category mapping shared by the inventory list and the picker. Catalog JSON generated offline from a UE4SS object dump by a Python script, bundled as a Flutter asset. New Rust edit op duplicates an existing item-stack object instance inside `m_Inventory.m_Items` (ObjectInstances), patches its definition path + count, fixes all size/count fields, validates by re-parse.

**Tech Stack:** Flutter/Dart (`apps/goresave`), Rust (`crates/goresave_core`), Python 3 (`tools/`).

**Spec:** `docs/superpowers/specs/2026-06-12-inventory-add-item-design.md`

**Branch:** `feat/inventory-categories-add-item` (create from `main` before Task 1; commit docs as first commit).

**Test commands:**
- Dart: `flutter test` in `apps/goresave` (single file: `flutter test test/<file>`)
- Rust: `cargo test -p goresave_core` in repo root
- Python: `python -m pytest tools/test_build_item_catalog.py -v`

---

### Task 1: Dart category model + mapping helper

**Files:**
- Create: `apps/goresave/lib/features/editor/domain/item_categories.dart`
- Test: `apps/goresave/test/features/editor/domain/item_categories_test.dart`

- [ ] **Step 1: Write the failing test**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';

void main() {
  test('maps known prefixes to categories', () {
    expect(itemCategoryFromId('ItMw_1H_Sword_01'), ItemCategory.meleeWeapon);
    expect(itemCategoryFromId('ItRw_Bow_Diego_Sleeper'), ItemCategory.rangedWeapon);
    expect(itemCategoryFromId('ItAr_Rune_FireBall_Base'), ItemCategory.rune);
    expect(itemCategoryFromId('ItAr_Scroll_Charm'), ItemCategory.scroll);
    expect(itemCategoryFromId('ItFo_Apple'), ItemCategory.food);
    expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
    expect(itemCategoryFromId('ItAt_Wolf_Fur'), ItemCategory.trophy);
    expect(itemCategoryFromId('ItWr_Map_OldWorld'), ItemCategory.writing);
    expect(itemCategoryFromId('ItMs_Ashes'), ItemCategory.mission);
    expect(itemCategoryFromId('ItKe_Lockpick'), ItemCategory.key);
    expect(itemCategoryFromId('ItKeyDefault'), ItemCategory.key);
    expect(itemCategoryFromId('ItChestKey01'), ItemCategory.key);
    expect(itemCategoryFromId('ItDoorKey01'), ItemCategory.key);
    expect(itemCategoryFromId('ItAm_Amulet_01'), ItemCategory.amulet);
  });

  test('unknown ids map to other', () {
    expect(itemCategoryFromId('Armor_OC_EBR_Gomez_100'), ItemCategory.other);
    expect(itemCategoryFromId(''), ItemCategory.other);
    expect(itemCategoryFromId('ItIg_Worldsplitter'), ItemCategory.other);
  });

  test('display name strips prefix', () {
    expect(itemDisplayNameFromId('ItMi_Orenugget'), 'Orenugget');
    expect(itemDisplayNameFromId('ItAr_Rune_FireBall_Base'), 'Rune FireBall Base');
    expect(itemDisplayNameFromId('NoPrefix'), 'NoPrefix');
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `flutter test test/features/editor/domain/item_categories_test.dart` (in `apps/goresave`)
Expected: FAIL — `item_categories.dart` does not exist.

- [ ] **Step 3: Implement**

```dart
/// Item categories for Gothic 1 Remake inventory items, derived from the
/// Angelscript class-name prefix (e.g. `ItMi_Orenugget` -> misc).
///
/// Prefix set verified against the UE4SS object dump of 2026-06-12; see
/// docs/superpowers/specs/2026-06-12-inventory-add-item-design.md.
enum ItemCategory {
  meleeWeapon('Melee weapons'),
  rangedWeapon('Ranged weapons'),
  rune('Runes'),
  scroll('Spell scrolls'),
  food('Food & potions'),
  misc('Miscellaneous'),
  trophy('Animal trophies'),
  writing('Writings'),
  mission('Mission items'),
  key('Keys'),
  amulet('Amulets'),
  other('Other');

  const ItemCategory(this.label);

  final String label;
}

ItemCategory itemCategoryFromId(String id) {
  if (id.startsWith('ItMw_')) return ItemCategory.meleeWeapon;
  if (id.startsWith('ItRw_')) return ItemCategory.rangedWeapon;
  if (id.startsWith('ItAr_Rune_')) return ItemCategory.rune;
  if (id.startsWith('ItAr_Scroll_')) return ItemCategory.scroll;
  if (id.startsWith('ItFo_')) return ItemCategory.food;
  if (id.startsWith('ItMi_')) return ItemCategory.misc;
  if (id.startsWith('ItAt_')) return ItemCategory.trophy;
  if (id.startsWith('ItWr_')) return ItemCategory.writing;
  if (id.startsWith('ItMs_')) return ItemCategory.mission;
  if (id.startsWith('ItKe_') ||
      id.startsWith('ItKey') ||
      id.startsWith('ItChestKey') ||
      id.startsWith('ItDoorKey')) {
    return ItemCategory.key;
  }
  if (id.startsWith('ItAm_')) return ItemCategory.amulet;
  return ItemCategory.other;
}

/// Human-readable name derived from the class id; never reads game
/// localization data (legal posture: identifiers only).
String itemDisplayNameFromId(String id) {
  const prefixes = ['ItMw_', 'ItRw_', 'ItAr_', 'ItFo_', 'ItMi_', 'ItAt_',
      'ItWr_', 'ItMs_', 'ItKe_', 'ItAm_'];
  var name = id;
  for (final prefix in prefixes) {
    if (name.startsWith(prefix)) {
      name = name.substring(prefix.length);
      break;
    }
  }
  return name.replaceAll('_', ' ').trim().isEmpty ? id : name.replaceAll('_', ' ').trim();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `flutter test test/features/editor/domain/item_categories_test.dart`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/item_categories.dart apps/goresave/test/features/editor/domain/item_categories_test.dart
git commit -m "feat(editor): add item category mapping for inventory"
```

---

### Task 2: Grouping helper (pure, testable)

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/item_categories.dart` (append)
- Test: `apps/goresave/test/features/editor/domain/item_categories_test.dart` (append)

- [ ] **Step 1: Write the failing test (append to existing test file)**

```dart
// add imports at top:
// import 'package:goresave/features/editor/domain/editor_models.dart';

  test('groups items by category in enum order, non-empty only', () {
    const items = [
      PrivateInventoryItem(id: 'ItMi_Orenugget', path: 'p1', count: 3),
      PrivateInventoryItem(id: 'ItMw_1H_Sword', path: 'p2', count: 1),
      PrivateInventoryItem(id: 'ItMi_Gold', path: 'p3', count: 9),
      PrivateInventoryItem(id: 'Weird_Thing', path: 'p4', count: 1),
    ];
    final groups = groupInventoryItems(items);
    expect(groups.map((g) => g.category).toList(),
        [ItemCategory.meleeWeapon, ItemCategory.misc, ItemCategory.other]);
    expect(groups[1].items.map((i) => i.id).toList(),
        ['ItMi_Gold', 'ItMi_Orenugget']); // sorted by id within group
  });
```

- [ ] **Step 2: Run test — expect FAIL (`groupInventoryItems` undefined)**

- [ ] **Step 3: Implement (append to `item_categories.dart`)**

```dart
import 'editor_models.dart';

class InventoryItemGroup {
  const InventoryItemGroup({required this.category, required this.items});

  final ItemCategory category;
  final List<PrivateInventoryItem> items;
}

/// Groups items by category. Groups appear in [ItemCategory] declaration
/// order, empty groups are omitted, items are sorted by id within a group.
List<InventoryItemGroup> groupInventoryItems(
  List<PrivateInventoryItem> items,
) {
  final byCategory = <ItemCategory, List<PrivateInventoryItem>>{};
  for (final item in items) {
    byCategory.putIfAbsent(itemCategoryFromId(item.id), () => []).add(item);
  }
  return [
    for (final category in ItemCategory.values)
      if (byCategory.containsKey(category))
        InventoryItemGroup(
          category: category,
          items: byCategory[category]!
            ..sort((a, b) => a.id.compareTo(b.id)),
        ),
  ];
}
```

Note: `PrivateInventoryItem` needs a `const` constructor — it already has one (`editor_models.dart:589`).

- [ ] **Step 4: Run full test file — expect PASS**

- [ ] **Step 5: Commit** — `feat(editor): add inventory grouping helper`

---

### Task 3: Categorized inventory list UI

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart:1546-1693` (`_PrivateInventorySummaryCardState`)

- [ ] **Step 1: Replace the flat list with grouped sections**

In `_PrivateInventorySummaryCardState`:
- Add field `final Set<ItemCategory> _collapsed = {};`
- Keep the existing query filter and `.take(80)` cap, then group:

```dart
final groups = groupInventoryItems(items);
```

- Replace the `ListView.separated` (lines 1639-1671) with a single
  `ListView.builder` over a flattened entry list (header + item rows), so
  there is no nested-scrollable problem:

```dart
// above build(): sealed-ish row model
// (file-private classes at the bottom of editor_page.dart)
abstract class _InventoryRow {}

class _InventoryHeaderRow implements _InventoryRow {
  _InventoryHeaderRow(this.group);
  final InventoryItemGroup group;
}

class _InventoryItemRow implements _InventoryRow {
  _InventoryItemRow(this.item);
  final PrivateInventoryItem item;
}
```

```dart
final rows = <_InventoryRow>[
  for (final group in groups) ...[
    _InventoryHeaderRow(group),
    if (!_collapsed.contains(group.category))
      ...group.items.map(_InventoryItemRow.new),
  ],
];
```

Header tile:

```dart
ListTile(
  dense: true,
  onTap: () => setState(() {
    _collapsed.contains(group.category)
        ? _collapsed.remove(group.category)
        : _collapsed.add(group.category);
  }),
  leading: Icon(
    _collapsed.contains(group.category)
        ? Icons.chevron_right
        : Icons.expand_more,
  ),
  title: Text(
    '${group.category.label} (${group.items.length})',
    style: Theme.of(context).textTheme.labelLarge,
  ),
)
```

Item tiles keep the existing `ListTile` body (icon, `SelectableText`,
`_InventoryItemCountEditor` trailing) unchanged.

- [ ] **Step 2: Run analyzer + existing tests**

Run: `flutter analyze` and `flutter test` (in `apps/goresave`)
Expected: no new analyzer issues; all tests pass.

- [ ] **Step 3: Manual smoke** — `flutter run -d windows`, open a save,
  Inventory tab shows collapsible category sections; search still filters.

- [ ] **Step 4: Commit** — `feat(editor): group inventory list by item category`

---

### Task 4: Catalog generator script

**Files:**
- Create: `tools/build_item_catalog.py`
- Test: `tools/test_build_item_catalog.py`
- Create (generated): `apps/goresave/assets/item_catalog.json`
- Modify: `apps/goresave/pubspec.yaml:60-61` (add asset)

- [ ] **Step 1: Write the failing test**

```python
import json
from pathlib import Path

from build_item_catalog import build_catalog, parse_dump_classes

FIXTURE = """\
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
"""


def test_parse_dump_classes_dedupes():
    names = parse_dump_classes(FIXTURE.splitlines())
    assert names.count("ItMi_Orenugget") == 1
    assert "SomethingElse" not in names  # only It* candidates


def test_build_catalog_filters_and_categorizes():
    entries, skipped = build_catalog(parse_dump_classes(FIXTURE.splitlines()))
    by_id = {e["id"]: e for e in entries}
    assert by_id["ItMi_Orenugget"] == {
        "id": "ItMi_Orenugget",
        "path": "/Script/Angelscript.ItMi_Orenugget",
        "category": "misc",
    }
    assert by_id["ItAr_Rune_FireBall"]["category"] == "rune"
    assert by_id["ItAr_Scroll_Charm"]["category"] == "scroll"
    assert by_id["ItKeyDefault"]["category"] == "key"
    assert by_id["ItIg_Worldsplitter"]["category"] == "special"
    # excluded entirely:
    assert "ItAr_Rune_FireBall_Base" not in by_id  # _Base suffix
    assert "ItemAnimConfig_Meatbug" not in by_id   # config class
    assert "ItAI_Plank" not in by_id               # AI prop
    # exclusions are reported, not silent:
    assert "ItAr_Rune_FireBall_Base" in skipped
    assert "ItemAnimConfig_Meatbug" in skipped


def test_output_sorted_and_stable():
    entries, _ = build_catalog(parse_dump_classes(FIXTURE.splitlines()))
    ids = [e["id"] for e in entries]
    assert ids == sorted(ids)
```

- [ ] **Step 2: Run test — expect FAIL (module missing)**

Run: `python -m pytest tools/test_build_item_catalog.py -v`

- [ ] **Step 3: Implement `tools/build_item_catalog.py`**

```python
#!/usr/bin/env python3
"""Build apps/goresave/assets/item_catalog.json from a UE4SS object dump.

Usage:
    python tools/build_item_catalog.py <UE4SS_ObjectDump.txt> [-o OUT.json]

The dump must come from Gothic 1 Remake with UE4SS's DumpAllObjects().
Only class identifiers are extracted (id, path, category) - no localized
names, stats, or other game content.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

CLASS_RE = re.compile(r"ASClass /Script/Angelscript\.(It[A-Za-z0-9_]+)")

CATEGORY_BY_PREFIX = [
    ("ItMw_", "melee_weapon"),
    ("ItRw_", "ranged_weapon"),
    ("ItAr_Rune_", "rune"),
    ("ItAr_Scroll_", "scroll"),
    ("ItFo_", "food"),
    ("ItMi_", "misc"),
    ("ItAt_", "trophy"),
    ("ItWr_", "writing"),
    ("ItMs_", "mission"),
    ("ItKe_", "key"),
    ("ItAm_", "amulet"),
]

# Non-inventory classes that match the It* scan.
EXCLUDE_PREFIXES = (
    "ItemAnimConfig",
    "ItemSpawnManagerConfig",
    "ItemCollisionFX",
    "ItemVisualWorldTargetConfig",
    "ItAI_",
)

# Known singletons that carry no category prefix.
EXPLICIT = {
    "ItKeyDefault": "key",
    "ItChestKey01": "key",
    "ItDoorKey01": "key",
    "ItIg_Worldsplitter": "special",
    "ItFocusStoneBridgeItem": "special",
}


def parse_dump_classes(lines) -> list[str]:
    names: set[str] = set()
    for line in lines:
        match = CLASS_RE.search(line)
        if match:
            names.add(match.group(1))
    return sorted(names)


def build_catalog(names: list[str]) -> tuple[list[dict], list[str]]:
    entries: list[dict] = []
    skipped: list[str] = []
    for name in names:
        if name.startswith(EXCLUDE_PREFIXES) or name.endswith("_Base"):
            skipped.append(name)
            continue
        category = EXPLICIT.get(name)
        if category is None:
            for prefix, cat in CATEGORY_BY_PREFIX:
                if name.startswith(prefix):
                    category = cat
                    break
        if category is None:
            category = "special"
            skipped.append(f"{name} (unmatched prefix -> special)")
        entries.append({
            "id": name,
            "path": f"/Script/Angelscript.{name}",
            "category": category,
        })
    return entries, skipped


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument(
        "-o", "--out", type=Path,
        default=Path(__file__).resolve().parent.parent
        / "apps" / "goresave" / "assets" / "item_catalog.json",
    )
    args = parser.parse_args()

    names = parse_dump_classes(
        args.dump.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    entries, skipped = build_catalog(names)
    args.out.write_text(
        json.dumps(entries, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(entries)} items to {args.out}")
    if skipped:
        print(f"skipped {len(skipped)} classes:")
        for name in skipped:
            print(f"  - {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Run test — expect PASS**

- [ ] **Step 5: Generate the real catalog**

Run (PowerShell):
```powershell
python tools\build_item_catalog.py "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\UE4SS_ObjectDump.txt"
```
Expected: `wrote ~800 items to ...item_catalog.json` (896 unique classes minus
~80 excluded/`_Base`). Review the skipped list — every skipped class must be a
config/AI/base class, not a real item; if a real item shows up there, extend
`CATEGORY_BY_PREFIX`/`EXPLICIT`.

- [ ] **Step 6: Register asset in `apps/goresave/pubspec.yaml`**

```yaml
  assets:
    - assets/goresave_icon.png
    - assets/item_catalog.json
```

- [ ] **Step 7: Commit** — `feat(tools): add item catalog generator + bundled catalog`

---

### Task 5: Catalog loader in Flutter

**Files:**
- Create: `apps/goresave/lib/features/editor/domain/item_catalog.dart`
- Test: `apps/goresave/test/features/editor/domain/item_catalog_test.dart`

- [ ] **Step 1: Write the failing test**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';

void main() {
  test('parses catalog json', () {
    const json = '''
[
  {"category": "misc", "id": "ItMi_Orenugget", "path": "/Script/Angelscript.ItMi_Orenugget"},
  {"category": "rune", "id": "ItAr_Rune_FireBall", "path": "/Script/Angelscript.ItAr_Rune_FireBall"}
]''';
    final catalog = ItemCatalog.fromJsonString(json);
    expect(catalog.entries, hasLength(2));
    expect(catalog.entries.first.id, 'ItAr_Rune_FireBall'); // sorted by id
    expect(catalog.entries.first.category, 'rune');
  });

  testWidgets('loads bundled asset', (tester) async {
    final catalog = await ItemCatalog.loadBundled();
    expect(catalog.entries.length, greaterThan(500));
    expect(catalog.entries.any((e) => e.id == 'ItMi_Orenugget'), isTrue);
  });
}
```

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement**

```dart
import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class ItemCatalogEntry {
  const ItemCatalogEntry({
    required this.id,
    required this.path,
    required this.category,
  });

  final String id;
  final String path;
  final String category;
}

class ItemCatalog {
  const ItemCatalog(this.entries);

  final List<ItemCatalogEntry> entries;

  static ItemCatalog fromJsonString(String json) {
    final list = (jsonDecode(json) as List)
        .whereType<Map<String, Object?>>()
        .map((e) => ItemCatalogEntry(
              id: e['id'] as String? ?? '',
              path: e['path'] as String? ?? '',
              category: e['category'] as String? ?? 'special',
            ))
        .where((e) => e.id.isNotEmpty && e.path.isNotEmpty)
        .toList()
      ..sort((a, b) => a.id.compareTo(b.id));
    return ItemCatalog(list);
  }

  static Future<ItemCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/item_catalog.json'));
}
```

`testWidgets` + `rootBundle` needs `TestWidgetsFlutterBinding.ensureInitialized()` — `testWidgets` does that itself.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit** — `feat(editor): load bundled item catalog`

---

### Task 6: Rust — parse + advertise `private.inventory.addItem`

**Files:**
- Modify: `crates/goresave_core/src/lib.rs` (edit parsing ~3955, structural-edit guard ~3982, writable lists ~2271 + ~3161, inventory writable gate)
- Test: existing test module in `lib.rs` (follow the file's `#[cfg(test)]` conventions; find tests for `parse_private_inventory_item_count_edit` and mirror them)

- [ ] **Step 1: Write failing tests** — mirror the existing count-edit parse tests:
  - valid edit `{"path": "private.inventory.addItem", "value": {"path": "/Script/Angelscript.ItMi_Orenugget", "count": 5}}` parses to `PrivateEdit::InventoryAddItem`
  - `count: 0` → `InvalidRequest`
  - path not matching `looks_item_definition_path` → `InvalidRequest`
  - two `addItem` edits in one batch → `UnsupportedEdit` (structural limit)
  - `addItem` + `arrayDuplicate` in one batch → `UnsupportedEdit`

- [ ] **Step 2: Run `cargo test -p goresave_core` — expect FAIL**

- [ ] **Step 3: Implement**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateInventoryAddItemEdit {
    path: String,
    count: i32,
}
```

- Add `PrivateEdit::InventoryAddItem(PrivateInventoryAddItemEdit)`.
- `parse_private_inventory_add_item_edit(edit)`: require `value.path` string
  passing `looks_item_definition_path`, `value.count` integer ≥ 1.
- Wire into the match in `apply_private_edits` (`lib.rs:3939`).
- Extend the structural-edit counter (`lib.rs:3982-4002`) to count
  `InventoryAddItem` alongside `ArrayRemove`/`ArrayDuplicate` (offsets shift).
- Advertise `"private.inventory.addItem"` in the `writable` arrays at
  `lib.rs:2271-2280` and `lib.rs:3161-3167` under the same conditions as the
  typed ops (typed parse ok) AND `scope == "player_inventory_region"` — same
  gate the summary uses for `setItemCount` writability.

- [ ] **Step 4: Run tests — expect parse tests PASS (apply fn may be `todo!()`-free stub returning `UnsupportedEdit` until Task 7)**

- [ ] **Step 5: Commit** — `feat(core): parse private.inventory.addItem edits`

---

### Task 7: Rust — addItem payload mechanics

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`, `crates/goresave_core/src/properties.rs`
- Test: fixture-based tests next to existing inventory/container-edit tests (check `fixtures/` and existing tests around `properties.rs:3029` for the fixture-building pattern; reuse the same synthetic-payload builders)

**Read first:** `properties.rs:1569-1597` (ObjectInstances parsing — instance
byte ranges), `properties.rs:1076-1128` (`ArrayDuplicate` — splice + count +
size-chain fixups), `properties.rs:862-882` (`patch_string` — length-changing
string patch with enclosing size fixups), `lib.rs:922` (ObjectInstances
rejection in `patch_container`).

- [ ] **Step 1: Write failing tests** (synthetic payload with an
  ObjectInstances `m_Items` array of two item instances, built with the same
  helpers the `ArrayDuplicate` tests use):
  - happy path: add `/Script/Angelscript.ItMi_Sulfur` count 7 → re-parse ok,
    `summarize_private_inventory_items` shows 3 items incl. new one with count 7
  - duplicate path → error mentioning "already"
  - empty inventory (zero instances) → error mentioning "no template"
  - payload that fails typed parse → error

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement `apply_private_inventory_add_item_to_payload`**

Algorithm (each sub-step validated against the parsed tree, abort on any miss):
1. `properties::parse_private_root(payload)` must be `Ok`; locate the
   `m_Inventory` StructProperty and its `m_Items` ObjectInstances array via
   the parsed tree (extend `properties.rs` with a resolver that returns the
   array's count-field offset, size-field chain, and per-instance byte
   ranges — the same data `ArrayDuplicate` uses, exposed for ObjectInstances).
2. Scan instances for an existing `m_ItemDefinition` equal to the target
   path → error if found.
3. Template choice: first instance whose definition id starts with `ItMi_`,
   else first instance (assumption verified in Task 8; revisit if the
   in-game test fails).
4. Splice a copy of the template instance bytes immediately after the
   template; bump the ObjectInstances element count; add the byte delta to
   the array size field and every enclosing size field (port of
   `ArrayDuplicate`, `properties.rs:1090-1128`).
5. Inside the new instance: patch the `m_ItemDefinition` FString to the
   target path (length-changing — fix the instance-internal size fields the
   same way `patch_string` does), set `m_ItemCount` to the requested count.
   If instances carry a serialized unique name, append/replace a numeric
   suffix to make it unique (determine while implementing against the
   fixture + real save; if no name field exists, skip).
6. Re-parse the modified payload with `parse_private_root`; require `Ok` and
   require the new item to appear in `summarize_private_inventory_items`
   with the requested count; otherwise return an error (write is then
   aborted upstream, nothing is persisted).

- [ ] **Step 4: Run — expect PASS, plus full `cargo test -p goresave_core`**

- [ ] **Step 5: Commit** — `feat(core): implement inventory addItem payload edit`

---

### Task 8: Real-save verification (manual gate — do NOT skip)

**Files:** none (verification only; fixes loop back into Task 7)

- [ ] **Step 1:** Copy a real save, apply `private.inventory.addItem` for
  `ItMi_Sulfur` (or another common misc item not in the save) via the app or
  a small Rust test binary against the copy.
- [ ] **Step 2:** Re-inspect the modified save in goresave — item listed with
  the right count, typed parse still `ok`.
- [ ] **Step 3:** Load the modified save in Gothic 1 Remake — game loads,
  item is in the inventory, no corruption on subsequent save.
- [ ] **Step 4:** Record findings (template homogeneity, instance naming) in
  `integration_test/` notes. If the game rejects the save, debug in Task 7
  before any UI work (superpowers:systematic-debugging).

---

### Task 9: Add-item model + dialog UI

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_models.dart` (append after `InventoryItemCountChange`, line 625)
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart` (`_InventoryPanel`, `_PrivateInventorySummaryCard`)
- Test: `apps/goresave/test/features/editor/domain/editor_models_test.dart` (or the file's existing model-test location), widget test for the dialog

- [ ] **Step 1: Model + failing test**

```dart
class InventoryItemAdd {
  const InventoryItemAdd({required this.path, required this.count});

  final String path;
  final int count;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.inventory.addItem',
      'value': {'path': path, 'count': count},
    };
  }
}
```

Test: `toEditJson()` shape matches the Rust parser's expectation.

- [ ] **Step 2: Dialog**

New private widget `_AddInventoryItemDialog` in `editor_page.dart`:
- Loads `ItemCatalog.loadBundled()` (`FutureBuilder`).
- Search `TextField` (substring on id/path, case-insensitive — same semantics
  as the list filter).
- Entries grouped with the same header style as Task 3, using
  `itemCategoryFromId`/`itemDisplayNameFromId`; entries whose `path` is
  already in `inventory.items` are excluded.
- Count field (default 1, `int.tryParse`, ≥ 1), "Add" returns
  `InventoryItemAdd`.
- "Add item" button sits in the card header row, visible when the panel is
  editable AND `inventory.writable.contains('private.inventory.addItem')`.
- Pending state: a single `InventoryItemAdd? _pendingAdd` in
  `_PrivateInventorySummaryCardState` (core allows ONE structural edit per
  write — UI enforces one pending add at a time; a second add is allowed
  only after save+refresh). Pushed via the existing
  `notifier.setPendingEdit('inventory', ...)` payload by concatenating
  count-change edits + the add edit.

- [ ] **Step 3: Widget test** — pump the dialog with a fake catalog (inject
  via constructor parameter `Future<ItemCatalog>? catalogOverride` to avoid
  asset loading), verify: search filters, existing item hidden, returns
  correct `InventoryItemAdd`.

- [ ] **Step 4: `flutter analyze` + `flutter test` — expect clean/PASS**

- [ ] **Step 5: Manual smoke** — add an item end-to-end on a save copy.

- [ ] **Step 6: Commit** — `feat(editor): add-item picker dialog for inventory`

---

### Task 10: Changelog + docs

**Files:**
- Modify: `CHANGELOG.md` (new Unreleased/next-version section)
- Modify: `integration_test/` notes (add manual add-item test steps)
- Modify: `README.md` features list (one line: categorized inventory + add items)

- [ ] **Step 1:** Write entries.
- [ ] **Step 2:** Commit — `docs: changelog + manual test notes for inventory add-item`

---

## Self-review notes

- Spec coverage: categorized list (Task 3), catalog pipeline (Task 4),
  loader (Task 5), Rust op (Tasks 6-7), real-save gate (Task 8 = spec
  Phase 1; ordered after the pure-UI tasks because categorization is
  independent and user-requested first), picker (Task 9), docs (Task 10).
- Rust Tasks 6-7 reference existing functions by line; the executor must
  re-locate them (lines shift) — names are authoritative, not numbers.
- One structural edit per write enforced both core-side (Task 6 Step 3) and
  UI-side (Task 9 Step 2).
