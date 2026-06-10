# Player Tab Hero Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Player tab's useless raw-FString picker with grouped, label-friendly editors for every hero gameplay attribute, backed by the typed property engine.

**Architecture:** The hero's attributes live in the decoded private payload at `m_GenericData/{CharacterStates}/AnyCharacterType/AttributesByGlobalId/{Hero}/AttributeSetsByClass/{<set class>}/Attributes/{<id>}/BaseValue|CurrentValue` (all `FloatProperty`). The existing `search_typed_properties` core command finds them and `private.typed.setValue` writes them — except that `AttributeSetsByClass` is a map with **ObjectProperty keys**, which the typed path layer cannot address today (they collapse to `{?}` and `resolve` fails with `map key "?" not found`; verified empirically on a real save). Task 1 fixes that in the Rust core. The Dart side then gets a parser that folds search hits into grouped `HeroAttribute` pairs, a batch write method, and a new `HeroStatsCard` widget in the Player tab. The old FString editor is deleted.

**Tech Stack:** Rust (goresave_core), Flutter/Dart (apps/goresave), flutter_test, cargo test.

**Spec:** `docs/superpowers/specs/2026-06-10-player-tab-hero-editing-design.md`

**Verified groundwork (already proven on a real save, G1R-007):**
- Search query `AttributesByGlobalId {Hero}` (two whitespace-separated terms; terms are substring-matched case-insensitively against the display path, which is joined with `" › "`) returns exactly the hero attribute leaves.
- With Object map keys made addressable, the full path round-trips: a write of MaxHealth BaseValue 64→65 through `private.typed.setValue` succeeded and read back correctly.
- Environment note: the codec host's compress runtime selftest has a fixed 5s timeout (`RUNTIME_SELFTEST_WORKER_TIMEOUT`) that can be exceeded on a busy machine, failing writes with "runtime selftest worker timed out after 5000 ms". That is a pre-existing issue, out of scope here (tracked separately). If the end-to-end verification in Task 8 hits it, retry on an idle machine — it is not caused by these changes.

---

### Task 1: Rust core — make Object map keys addressable in typed paths

The single source of truth for map-key segments is `map_key_to_string` in `crates/goresave_core/src/properties.rs` (~line 214). Search labels and `resolve` both call it, so extending it fixes both directions at once and keeps them in lockstep.

**Files:**
- Modify: `crates/goresave_core/src/properties.rs` (function `map_key_to_string`, ~line 214; tests at the bottom of the file)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/goresave_core/src/properties.rs`, next to `map_of_instanced_payload()` (~line 1761). The fixture mirrors the real save shape: a `MapProperty` with `ObjectProperty` keys whose values are `InstancedStruct`s containing a `FloatProperty`.

```rust
    fn float_property(name: &str, value: f32) -> Vec<u8> {
        let mut out = tag(name, "FloatProperty");
        out.extend_from_slice(&header(4, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn map_of_object_keyed_instanced_payload() -> Vec<u8> {
        let nested = {
            let mut n = float_property("BaseValue", 64.0);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/G1R.GameplayAttributeData");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("/Script/G1R.AttributeSet_Health")); // ObjectProperty key
        map_body.extend_from_slice(&instanced); // value: InstancedStruct inline

        let mut props = tag("AttributeSetsByClass", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("ObjectProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        root("/Script/Test.Save", &props)
    }

    #[test]
    fn search_addresses_object_map_keys() {
        let payload = map_of_object_keyed_instanced_payload();
        let root = parse_private_root(&payload).unwrap();
        let (hits, total) = search_properties(&root, "basevalue", 0, 100);
        assert_eq!(total, 1);
        let hit = &hits[0];
        assert_eq!(
            hit.path,
            vec![
                "AttributeSetsByClass",
                "{/Script/G1R.AttributeSet_Health}",
                "BaseValue"
            ]
        );
        assert_eq!(hit.type_name, "FloatProperty");
        assert!(hit.editable);
        // The search-built path must round-trip through resolve().
        let segs = parse_path(&hit.path).unwrap();
        assert_eq!(
            resolve(&root.properties, &segs).unwrap().value,
            PropertyValue::Float(64.0)
        );
    }
```

If `float_property` already exists in the test module, reuse it instead of adding a duplicate.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p goresave_core search_addresses_object_map_keys`
Expected: FAIL — the hit path contains `"{?}"` instead of the class path, so the path assertion (or the `resolve` round-trip with `map key "?" not found`) fails.

- [ ] **Step 3: Implement the fix**

In `map_key_to_string` (~line 214), add `PropertyValue::Object` to the string-like key arm:

```rust
fn map_key_to_string(key: &PropertyValue) -> Option<String> {
    match key {
        PropertyValue::Str(s)
        | PropertyValue::Name(s)
        | PropertyValue::Enum(s)
        | PropertyValue::Object(s) => Some(s.clone()),
        PropertyValue::Int(i) => Some(i.to_string()),
        PropertyValue::Struct(StructValue::Guid(raw)) => Some(hex_guid(raw)),
        _ => None,
    }
}
```

Also update the doc comment on `PathSeg` (~line 177) so the `{key}` line reads `Str/Name/Enum/Object keys` instead of `Str/Name/Enum keys`.

- [ ] **Step 4: Run the test to verify it passes, plus the whole crate**

Run: `cargo test -p goresave_core search_addresses_object_map_keys`
Expected: PASS

Run: `cargo test -p goresave_core`
Expected: all tests pass (no existing test asserts the `{?}` label; if one does, update it to expect the now-addressable key).

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/properties.rs
git commit -m "feat(core): address Object-keyed map entries in typed property paths"
```

---

### Task 2: Commit the core dev examples

Two examples were written during feasibility probing and are already present but untracked. They are generic CLI tools (all machine-specific paths are arguments) and are the verification vehicle for Task 8.

**Files:**
- Already created (untracked): `crates/goresave_core/examples/dump_typed.rs` — pages through `search_typed_properties` and prints every hit as a JSON line.
- Already created (untracked): `crates/goresave_core/examples/try_typed_edit.rs` — applies one `private.typed.setValue` edit via the `write_save` command.

- [ ] **Step 1: Build both examples**

Run: `cargo build -p goresave_core --example dump_typed --example try_typed_edit --release`
Expected: `Finished` with no warnings about these files.

- [ ] **Step 2: Commit**

```bash
git add crates/goresave_core/examples/dump_typed.rs crates/goresave_core/examples/try_typed_edit.rs
git commit -m "chore(core): add typed-property dump and edit dev examples"
```

---

### Task 3: Dart — hero attribute model, parser, grouping (TDD)

Pure-Dart logic in a new focused file. No Flutter imports.

**Files:**
- Create: `apps/goresave/lib/features/editor/domain/hero_attributes.dart`
- Test: `apps/goresave/test/features/editor/domain/hero_attributes_test.dart`

- [ ] **Step 1: Write the failing tests**

Create `apps/goresave/test/features/editor/domain/hero_attributes_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';

TypedPropertyHit _heroHit(
  String setClass,
  String id,
  String leaf,
  String value, {
  String type = 'FloatProperty',
  bool editable = true,
}) {
  final path = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{Hero}',
    'AttributeSetsByClass',
    '{$setClass}',
    'Attributes',
    '{$id}',
    leaf,
  ];
  return TypedPropertyHit(
    path: path,
    display: path.join(' › '),
    type: type,
    value: value,
    editable: editable,
  );
}

void main() {
  test('pairs BaseValue and CurrentValue leaves into one attribute', () {
    final attributes = parseHeroAttributes([
      _heroHit('/Script/G1R.AttributeSet_Health', 'MaxHealth', 'BaseValue', '64'),
      _heroHit('/Script/G1R.AttributeSet_Health', 'MaxHealth', 'CurrentValue', '64'),
    ]);

    expect(attributes, hasLength(1));
    final attribute = attributes.single;
    expect(attribute.id, 'MaxHealth');
    expect(attribute.setClass, '/Script/G1R.AttributeSet_Health');
    expect(attribute.baseValue, 64);
    expect(attribute.currentValue, 64);
    expect(attribute.basePath, isNotNull);
    expect(attribute.basePath!.last, 'BaseValue');
    expect(attribute.currentPath!.last, 'CurrentValue');
  });

  test('keeps same-id attributes from different sets separate', () {
    final attributes = parseHeroAttributes([
      _heroHit('/Script/G1R.AttributeSet_Health', 'RecoveryRatePerHourOfSleep',
          'BaseValue', '0.125'),
      _heroHit('/Script/G1R.AttributeSet_Mana', 'RecoveryRatePerHourOfSleep',
          'BaseValue', '-0.125'),
    ]);

    expect(attributes, hasLength(2));
    expect(attributes.map((a) => a.setClass).toSet(), hasLength(2));
  });

  test('skips non-attribute, non-editable and non-float hits', () {
    final nonHero = TypedPropertyHit(
      path: const ['m_GenericData', '{CharacterStates}', 'GlobalIDFormat'],
      display: 'GlobalIDFormat',
      type: 'StrProperty',
      value: 'x',
      editable: true,
    );
    final attributes = parseHeroAttributes([
      nonHero,
      _heroHit('/Script/G1R.AttributeSet_Health', 'Health', 'BaseValue', '35',
          editable: false),
      _heroHit('/Script/G1R.AttributeSet_Health', 'Health', 'CurrentValue', '35',
          type: 'StrProperty'),
    ]);

    expect(attributes, isEmpty);
  });

  test('assigns known ids to their groups and unknown ids to advanced', () {
    expect(heroAttributeGroup('MaxHealth'), HeroAttributeGroup.core);
    expect(heroAttributeGroup('SkillPoints'), HeroAttributeGroup.core);
    expect(heroAttributeGroup('Critical_OneHand'), HeroAttributeGroup.combat);
    expect(heroAttributeGroup('Resistance_Fire'), HeroAttributeGroup.resistances);
    expect(heroAttributeGroup('PickPocketing'), HeroAttributeGroup.thieving);
    expect(heroAttributeGroup('Swampweed'), HeroAttributeGroup.advanced);
    expect(heroAttributeGroup('SomeFutureAttribute'), HeroAttributeGroup.advanced);
  });

  test('sorts core attributes in display order before unknown ones', () {
    final attributes = parseHeroAttributes([
      _heroHit('/Script/G1R.AttributeSet_Strength', 'Strength', 'BaseValue', '10'),
      _heroHit('/Script/G1R.AttributeSet_Health', 'MaxHealth', 'BaseValue', '64'),
      _heroHit('/Script/G1R.AttributeSet_Health', 'Health', 'BaseValue', '35'),
    ]);

    expect(attributes.map((a) => a.id).toList(),
        ['Health', 'MaxHealth', 'Strength']);
  });

  test('labels SkillPoints as learn points', () {
    expect(heroAttributeLabel('SkillPoints'), 'Skill points (LP)');
    expect(heroAttributeLabel('MaxHealth'), 'MaxHealth');
  });
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run (from `apps/goresave`): `flutter test test/features/editor/domain/hero_attributes_test.dart`
Expected: FAIL — `hero_attributes.dart` does not exist.

- [ ] **Step 3: Implement the model and parser**

Create `apps/goresave/lib/features/editor/domain/hero_attributes.dart`:

```dart
import 'editor_models.dart';

/// One hero gameplay attribute: the BaseValue/CurrentValue pair found at
/// `.../AttributesByGlobalId/{Hero}/AttributeSetsByClass/{setClass}/Attributes/{id}/...`
/// in the typed property tree. Paths are `private.typed.setValue` addressable.
class HeroAttribute {
  const HeroAttribute({
    required this.id,
    required this.setClass,
    this.basePath,
    this.currentPath,
    this.baseValue,
    this.currentValue,
  });

  final String id;
  final String setClass;
  final List<String>? basePath;
  final List<String>? currentPath;
  final double? baseValue;
  final double? currentValue;
}

enum HeroAttributeGroup { core, combat, resistances, thieving, advanced }

const heroCoreAttributeOrder = [
  'Health',
  'MaxHealth',
  'Mana',
  'MaxMana',
  'Strength',
  'Dexterity',
  'Level',
  'Experience',
  'SkillPoints',
  'MagicianLevel',
];

const heroCombatAttributes = [
  'Critical_Fists',
  'Critical_OneHand',
  'Critical_TwoHand',
  'Critical_Orc',
];

const heroResistanceAttributes = [
  'Resistance_Blunt',
  'Resistance_Edge',
  'Resistance_Point',
  'Resistance_Fire',
  'Resistance_Energy',
  'Resistance_Ice',
  'Resistance_Wind',
  'Resistance_Falling',
];

const heroThievingAttributes = [
  'LockpickDurability',
  'LockpickPrecision',
  'PickPocketing',
];

HeroAttributeGroup heroAttributeGroup(String id) {
  if (heroCoreAttributeOrder.contains(id)) return HeroAttributeGroup.core;
  if (heroCombatAttributes.contains(id)) return HeroAttributeGroup.combat;
  if (heroResistanceAttributes.contains(id)) {
    return HeroAttributeGroup.resistances;
  }
  if (heroThievingAttributes.contains(id)) return HeroAttributeGroup.thieving;
  return HeroAttributeGroup.advanced;
}

/// Display label for an attribute id. SkillPoints are Gothic's learn points,
/// which is what players actually look for.
String heroAttributeLabel(String id) {
  if (id == 'SkillPoints') return 'Skill points (LP)';
  return id;
}

int _groupRank(String id) {
  const orders = [
    heroCoreAttributeOrder,
    heroCombatAttributes,
    heroResistanceAttributes,
    heroThievingAttributes,
  ];
  final group = heroAttributeGroup(id);
  if (group == HeroAttributeGroup.advanced) return 1 << 20;
  final order = orders[group.index];
  return (group.index << 12) + order.indexOf(id);
}

/// Fold typed search hits into hero attributes. Only editable FloatProperty
/// leaves named BaseValue/CurrentValue under `AttributesByGlobalId/{Hero}`
/// count; everything else in the result page is ignored. Attributes with the
/// same id in different attribute sets stay separate entries.
List<HeroAttribute> parseHeroAttributes(List<TypedPropertyHit> hits) {
  final byPrefix = <String, _HeroAttributeBuilder>{};
  for (final hit in hits) {
    if (!hit.editable || hit.type != 'FloatProperty') continue;
    final path = hit.path;
    if (path.length < 4) continue;
    final leaf = path.last;
    if (leaf != 'BaseValue' && leaf != 'CurrentValue') continue;
    final heroIndex = path.indexOf('{Hero}');
    if (heroIndex < 1 || path[heroIndex - 1] != 'AttributesByGlobalId') {
      continue;
    }
    final idSegment = path[path.length - 2];
    if (!idSegment.startsWith('{') || !idSegment.endsWith('}')) continue;
    final id = idSegment.substring(1, idSegment.length - 1);
    final setIndex = path.indexOf('AttributeSetsByClass');
    var setClass = '';
    if (setIndex >= 0 && setIndex + 1 < path.length) {
      final seg = path[setIndex + 1];
      if (seg.startsWith('{') && seg.endsWith('}')) {
        setClass = seg.substring(1, seg.length - 1);
      }
    }
    final prefix = path.sublist(0, path.length - 1).join(' ');
    final builder = byPrefix.putIfAbsent(
      prefix,
      () => _HeroAttributeBuilder(id: id, setClass: setClass),
    );
    final value = double.tryParse(hit.value);
    if (leaf == 'BaseValue') {
      builder.basePath = path;
      builder.baseValue = value;
    } else {
      builder.currentPath = path;
      builder.currentValue = value;
    }
  }
  final attributes = byPrefix.values.map((b) => b.build()).toList()
    ..sort((a, b) {
      final rank = _groupRank(a.id).compareTo(_groupRank(b.id));
      if (rank != 0) return rank;
      final byId = a.id.compareTo(b.id);
      if (byId != 0) return byId;
      return a.setClass.compareTo(b.setClass);
    });
  return attributes;
}

class _HeroAttributeBuilder {
  _HeroAttributeBuilder({required this.id, required this.setClass});

  final String id;
  final String setClass;
  List<String>? basePath;
  List<String>? currentPath;
  double? baseValue;
  double? currentValue;

  HeroAttribute build() {
    return HeroAttribute(
      id: id,
      setClass: setClass,
      basePath: basePath,
      currentPath: currentPath,
      baseValue: baseValue,
      currentValue: currentValue,
    );
  }
}
```

Note the prefix join uses `' '` as separator so path segments containing `'/'` (the set-class object paths) cannot collide.

- [ ] **Step 4: Run the tests to verify they pass**

Run (from `apps/goresave`): `flutter test test/features/editor/domain/hero_attributes_test.dart`
Expected: PASS (all 6 tests)

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/hero_attributes.dart apps/goresave/test/features/editor/domain/hero_attributes_test.dart
git commit -m "feat(editor): parse hero attributes from typed property search hits"
```

---

### Task 4: Dart — notifier: load hero attributes + batch typed writes (TDD)

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart` (add two methods near `writeTypedValue`, ~line 781)
- Modify: `apps/goresave/lib/features/editor/domain/hero_attributes.dart` (add `TypedValueEdit` + `HeroAttributesResult`)
- Test: `apps/goresave/test/editor_notifier_test.dart`

- [ ] **Step 1: Write the failing tests**

Add to `apps/goresave/test/editor_notifier_test.dart` (same pattern as the existing `writeTypedValue sends host-backed typed setValue edit` test at ~line 426; `_RecordingCoreService` is defined at the bottom of that file). Add the import at the top: `import 'package:goresave/features/editor/domain/hero_attributes.dart';`

```dart
  test('loadHeroAttributes searches the hero attribute subtree', () async {
    final core = _RecordingCoreService(
      typedSearchData: {
        'query': 'AttributesByGlobalId {Hero}',
        'offset': 0,
        'limit': 1000,
        'total': 2,
        'count': 2,
        'results': [
          {
            'path': [
              'm_GenericData',
              '{CharacterStates}',
              'AnyCharacterType',
              'AttributesByGlobalId',
              '{Hero}',
              'AttributeSetsByClass',
              '{/Script/G1R.AttributeSet_Health}',
              'Attributes',
              '{MaxHealth}',
              'BaseValue',
            ],
            'display': '…',
            'type': 'FloatProperty',
            'value': '64',
            'editable': true,
          },
          {
            'path': [
              'm_GenericData',
              '{CharacterStates}',
              'AnyCharacterType',
              'AttributesByGlobalId',
              '{Hero}',
              'AttributeSetsByClass',
              '{/Script/G1R.AttributeSet_Health}',
              'Attributes',
              '{MaxHealth}',
              'CurrentValue',
            ],
            'display': '…',
            'type': 'FloatProperty',
            'value': '64',
            'editable': true,
          },
        ],
      },
    );
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    final result = await notifier.loadHeroAttributes();

    final search = core.requests.lastWhere(
      (request) => request.command == 'search_typed_properties',
    );
    expect(search.payload['query'], 'AttributesByGlobalId {Hero}');
    expect(search.payload['limit'], 1000);
    expect(result.error, isNull);
    expect(result.attributes, hasLength(1));
    expect(result.attributes.single.id, 'MaxHealth');
  });

  test('writeTypedValues batches several typed edits into one write', () async {
    final core = _RecordingCoreService();
    final notifier = EditorNotifier(
      core,
      saveDir: r'C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames',
      codecHostPath: r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      gameExePath:
          r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');

    await notifier.writeTypedValues(const [
      TypedValueEdit(path: ['a', '{B}', 'BaseValue'], value: 65),
      TypedValueEdit(path: ['a', '{B}', 'CurrentValue'], value: 65),
    ]);

    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(write.payload['backup'], isTrue);
    expect(write.payload['edits'], [
      {
        'path': 'private.typed.setValue',
        'value': {
          'path': ['a', '{B}', 'BaseValue'],
          'value': 65,
        },
      },
      {
        'path': 'private.typed.setValue',
        'value': {
          'path': ['a', '{B}', 'CurrentValue'],
          'value': 65,
        },
      },
    ]);
  });
```

If `_RecordingCoreService` does not yet answer `search_typed_properties`, extend it: add an optional `typedSearchData` constructor parameter (a `Map<String, Object?>`), and in its `execute` method return `{'ok': true, 'data': typedSearchData ?? {...empty result...}}` for that command, mirroring how its other commands respond.

- [ ] **Step 2: Run the tests to verify they fail**

Run (from `apps/goresave`): `flutter test test/editor_notifier_test.dart`
Expected: FAIL — `loadHeroAttributes`, `writeTypedValues`, `TypedValueEdit` undefined.

- [ ] **Step 3: Implement**

Add to `apps/goresave/lib/features/editor/domain/hero_attributes.dart`:

```dart
/// One pending `private.typed.setValue` edit.
class TypedValueEdit {
  const TypedValueEdit({required this.path, required this.value});

  final List<String> path;
  final Object value;
}

/// Result of loading the hero attribute subtree.
class HeroAttributesResult {
  const HeroAttributesResult({this.attributes = const [], this.error});

  final List<HeroAttribute> attributes;
  final String? error;
}
```

Add to `apps/goresave/lib/features/editor/domain/editor_notifier.dart` (next to `writeTypedValue`, import `hero_attributes.dart`):

```dart
  /// Search query that returns exactly the hero attribute leaves: both terms
  /// must appear in the display path, which only holds for entries under
  /// AttributesByGlobalId/{Hero}.
  static const heroAttributesQuery = 'AttributesByGlobalId {Hero}';

  /// Load every hero gameplay attribute from the typed property tree. The
  /// decode cache is already seeded by inspect, so this does not pay a
  /// second full private-payload decode.
  Future<HeroAttributesResult> loadHeroAttributes() async {
    final result = await searchTypedProperties(
      heroAttributesQuery,
      limit: 1000,
    );
    if (result.error != null) {
      return HeroAttributesResult(error: result.error);
    }
    return HeroAttributesResult(
      attributes: parseHeroAttributes(result.results),
    );
  }

  /// Apply several typed edits as one write_save call: one backup, one
  /// re-inspect, all-or-nothing from the user's point of view.
  Future<bool> writeTypedValues(List<TypedValueEdit> edits) async {
    if (edits.isEmpty) return true;
    final savePath = state.selectedPath;
    if (savePath == null) return false;
    return _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        'edits': [
          for (final edit in edits)
            {
              'path': 'private.typed.setValue',
              'value': {'path': edit.path, 'value': edit.value},
            },
        ],
        ..._codecPayload(),
      },
      message: (data) => _backupMessage(
        edits.length == 1
            ? 'Typed value saved with backup'
            : '${edits.length} typed values saved with backup',
        data,
      ),
    );
  }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run (from `apps/goresave`): `flutter test test/editor_notifier_test.dart`
Expected: PASS (including all pre-existing tests)

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/editor_notifier.dart apps/goresave/lib/features/editor/domain/hero_attributes.dart apps/goresave/test/editor_notifier_test.dart
git commit -m "feat(editor): load hero attributes and batch typed value writes"
```

---

### Task 5: Dart — HeroStatsCard widget (new file, widget-tested)

A public widget in its own file (editor_page.dart is 3500+ lines; new UI goes in a focused file). It takes plain callbacks instead of the notifier so widget tests need no notifier/core plumbing.

**Files:**
- Create: `apps/goresave/lib/features/editor/ui/hero_stats_card.dart`
- Test: `apps/goresave/test/features/editor/ui/hero_stats_card_test.dart`

- [ ] **Step 1: Write the failing widget test**

Create `apps/goresave/test/features/editor/ui/hero_stats_card_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart';

HeroAttribute _attribute(String id, String setClass, double value) {
  final prefix = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{Hero}',
    'AttributeSetsByClass',
    '{$setClass}',
    'Attributes',
    '{$id}',
  ];
  return HeroAttribute(
    id: id,
    setClass: setClass,
    basePath: [...prefix, 'BaseValue'],
    currentPath: [...prefix, 'CurrentValue'],
    baseValue: value,
    currentValue: value,
  );
}

void main() {
  testWidgets('renders groups and saves dirty rows as one batch',
      (tester) async {
    final saved = <List<TypedValueEdit>>[];
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Critical_OneHand', '/Script/G1R.AttributeSet_Critical', 3),
      _attribute('Swampweed', '/Script/G1R.AttributeSet_Drugs', 0),
    ];

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(attributes: attributes),
              save: (edits) async {
                saved.add(edits);
                return true;
              },
              editable: true,
              reloadKey: 'save-1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Groups: core and combat sections visible, advanced collapsed.
    expect(find.text('Main stats'), findsOneWidget);
    expect(find.text('Combat skills'), findsOneWidget);
    expect(find.text('Advanced'), findsOneWidget);
    expect(find.text('MaxHealth'), findsOneWidget);
    // Advanced group is collapsed: its row is not built.
    expect(find.text('Swampweed'), findsNothing);

    // Edit MaxHealth base value, then save.
    final baseField = find.widgetWithText(TextField, 'MaxHealth base');
    await tester.enterText(baseField, '99');
    await tester.pump();
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(saved, hasLength(1));
    expect(saved.single, hasLength(1));
    final edit = saved.single.single;
    expect(edit.path.last, 'BaseValue');
    expect(edit.path[edit.path.length - 2], '{MaxHealth}');
    expect(edit.value, 99);
  });

  testWidgets('expanding advanced shows remaining attributes', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(attributes: [
                _attribute('Swampweed', '/Script/G1R.AttributeSet_Drugs', 0),
              ]),
              save: (_) async => true,
              editable: true,
              reloadKey: 'save-1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Advanced'));
    await tester.pumpAndSettle();

    expect(find.text('Swampweed'), findsOneWidget);
  });

  testWidgets('shows load error inline', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: HeroStatsCard(
            load: () async =>
                const HeroAttributesResult(error: 'decode failed'),
            save: (_) async => true,
            editable: true,
            reloadKey: 'save-1',
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
  });
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run (from `apps/goresave`): `flutter test test/features/editor/ui/hero_stats_card_test.dart`
Expected: FAIL — `hero_stats_card.dart` does not exist.

- [ ] **Step 3: Implement the widget**

Create `apps/goresave/lib/features/editor/ui/hero_stats_card.dart`:

```dart
import 'package:flutter/material.dart';

import '../domain/hero_attributes.dart';

/// Grouped editors for every hero gameplay attribute. Data arrives through
/// [load] (typed property search) and leaves through [save] (one batched
/// private.typed.setValue write). [reloadKey] identifies the inspected save:
/// when it changes, pending edits are dropped and the card reloads.
class HeroStatsCard extends StatefulWidget {
  const HeroStatsCard({
    super.key,
    required this.load,
    required this.save,
    required this.editable,
    required this.reloadKey,
  });

  final Future<HeroAttributesResult> Function() load;
  final Future<bool> Function(List<TypedValueEdit> edits) save;
  final bool editable;
  final Object reloadKey;

  @override
  State<HeroStatsCard> createState() => _HeroStatsCardState();
}

class _HeroStatsCardState extends State<HeroStatsCard> {
  List<HeroAttribute> _attributes = const [];
  String? _error;
  bool _loading = false;
  // Pending field texts keyed by the typed path (joined). Cleared on reload.
  final Map<String, String> _pending = {};
  bool _advancedExpanded = false;

  static const _groupTitles = {
    HeroAttributeGroup.core: 'Main stats',
    HeroAttributeGroup.combat: 'Combat skills',
    HeroAttributeGroup.resistances: 'Resistances',
    HeroAttributeGroup.thieving: 'Thieving',
    HeroAttributeGroup.advanced: 'Advanced',
  };

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void didUpdateWidget(covariant HeroStatsCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _reload();
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _pending.clear();
    });
    final result = await widget.load();
    if (!mounted) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _attributes = result.attributes;
    });
  }

  String _pathKey(List<String> path) => path.join(' ');

  void _onFieldChanged(List<String>? path, String text) {
    if (path == null) return;
    _pending[_pathKey(path)] = text;
  }

  Future<void> _save() async {
    final edits = <TypedValueEdit>[];
    for (final attribute in _attributes) {
      for (final (path, original) in [
        (attribute.basePath, attribute.baseValue),
        (attribute.currentPath, attribute.currentValue),
      ]) {
        if (path == null) continue;
        final text = _pending[_pathKey(path)];
        if (text == null) continue;
        final value = double.tryParse(text.trim());
        if (value == null) {
          setState(() => _error = 'Invalid number: "$text"');
          return;
        }
        if (value == original) continue;
        edits.add(TypedValueEdit(path: path, value: value));
      }
    }
    if (edits.isEmpty) return;
    setState(() => _error = null);
    await widget.save(edits);
    // The save triggers a re-inspect upstream; reloadKey changes and this
    // card reloads with fresh values.
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.monitor_heart_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Hero stats',
                    style: theme.textTheme.titleMedium,
                  ),
                ),
                Tooltip(
                  message: 'Save hero stats',
                  child: IconButton.filledTonal(
                    icon: const Icon(Icons.save_outlined),
                    onPressed:
                        widget.editable && !_loading ? _save : null,
                  ),
                ),
              ],
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  _error!,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ),
            if (_loading)
              const Padding(
                padding: EdgeInsets.all(16),
                child: Center(child: CircularProgressIndicator()),
              )
            else
              ..._buildGroups(context),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildGroups(BuildContext context) {
    final theme = Theme.of(context);
    final byGroup = <HeroAttributeGroup, List<HeroAttribute>>{};
    for (final attribute in _attributes) {
      byGroup
          .putIfAbsent(heroAttributeGroup(attribute.id), () => [])
          .add(attribute);
    }
    final widgets = <Widget>[];
    for (final group in HeroAttributeGroup.values) {
      final attributes = byGroup[group];
      if (attributes == null || attributes.isEmpty) continue;
      if (group == HeroAttributeGroup.advanced) {
        widgets.add(
          ExpansionTile(
            tilePadding: EdgeInsets.zero,
            title: Text(
              _groupTitles[group]!,
              style: theme.textTheme.titleSmall,
            ),
            initiallyExpanded: _advancedExpanded,
            onExpansionChanged: (open) => _advancedExpanded = open,
            children: [for (final a in attributes) _row(a)],
          ),
        );
        continue;
      }
      widgets
        ..add(const SizedBox(height: 12))
        ..add(Text(_groupTitles[group]!, style: theme.textTheme.titleSmall))
        ..add(const SizedBox(height: 4))
        ..addAll([for (final a in attributes) _row(a)]);
    }
    return widgets;
  }

  Widget _row(HeroAttribute attribute) {
    final duplicate =
        _attributes.where((a) => a.id == attribute.id).length > 1;
    return _HeroAttributeRow(
      // Keyed by save identity and full path so a different save (or set)
      // never reuses stale field state.
      key: ValueKey(
        '${widget.reloadKey}-${attribute.setClass}-${attribute.id}',
      ),
      attribute: attribute,
      duplicate: duplicate,
      editable: widget.editable,
      onBaseChanged: (text) => _onFieldChanged(attribute.basePath, text),
      onCurrentChanged: (text) =>
          _onFieldChanged(attribute.currentPath, text),
    );
  }
}

class _HeroAttributeRow extends StatefulWidget {
  const _HeroAttributeRow({
    super.key,
    required this.attribute,
    required this.duplicate,
    required this.editable,
    required this.onBaseChanged,
    required this.onCurrentChanged,
  });

  final HeroAttribute attribute;
  final bool duplicate;
  final bool editable;
  final ValueChanged<String> onBaseChanged;
  final ValueChanged<String> onCurrentChanged;

  @override
  State<_HeroAttributeRow> createState() => _HeroAttributeRowState();
}

class _HeroAttributeRowState extends State<_HeroAttributeRow> {
  late final TextEditingController _baseController;
  late final TextEditingController _currentController;

  @override
  void initState() {
    super.initState();
    _baseController = TextEditingController(
      text: formatHeroValue(widget.attribute.baseValue),
    );
    _currentController = TextEditingController(
      text: formatHeroValue(widget.attribute.currentValue),
    );
  }

  @override
  void dispose() {
    _baseController.dispose();
    _currentController.dispose();
    super.dispose();
  }

  String get _label {
    final label = heroAttributeLabel(widget.attribute.id);
    if (!widget.duplicate) return label;
    final setName = widget.attribute.setClass.split('.').last;
    return '$label ($setName)';
  }

  @override
  Widget build(BuildContext context) {
    final name = widget.attribute.id;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 620;
          final baseField = TextField(
            controller: _baseController,
            enabled: widget.editable && widget.attribute.basePath != null,
            onChanged: widget.onBaseChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(labelText: '$name base'),
          );
          final currentField = TextField(
            controller: _currentController,
            enabled:
                widget.editable && widget.attribute.currentPath != null,
            onChanged: widget.onCurrentChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(labelText: '$name current'),
          );
          final label = Text(
            _label,
            style: Theme.of(context).textTheme.labelLarge,
          );
          if (compact) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                label,
                const SizedBox(height: 6),
                baseField,
                const SizedBox(height: 6),
                currentField,
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              SizedBox(width: 170, child: label),
              Expanded(child: baseField),
              const SizedBox(width: 8),
              Expanded(child: currentField),
            ],
          );
        },
      ),
    );
  }
}

/// Integers render without a decimal point; everything else keeps up to two
/// decimals (mirrors the attribute formatting used elsewhere in the editor).
String formatHeroValue(double? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  return value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run (from `apps/goresave`): `flutter test test/features/editor/ui/hero_stats_card_test.dart`
Expected: PASS (3 tests). If the tap on the collapsed `Advanced` tile fails because it is off-screen, wrap with `await tester.ensureVisible(find.text('Advanced'));` before tapping.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/hero_stats_card.dart apps/goresave/test/features/editor/ui/hero_stats_card_test.dart
git commit -m "feat(ui): add grouped hero stats card backed by typed properties"
```

---

### Task 6: Wire HeroStatsCard into the Player tab, gate the legacy card

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`
  - `_PrivatePanel.build` (~line 1071): insert the card into the Player ListView
  - `_PrivatePlayerSummaryCard` (~line 1745): hide the legacy heuristic attributes editor when the typed card is active
  - add import for `hero_stats_card.dart`

- [ ] **Step 1: Add the import**

At the top of `editor_page.dart`, with the other relative imports:

```dart
import 'hero_stats_card.dart';
```

- [ ] **Step 2: Insert the card in `_PrivatePanel.build`**

The Player ListView currently reads (~lines 1075–1099):

```dart
        children: [
          if (title == 'Player' && inspection.privatePlayer.hasData) ...[
            _PrivatePlayerSummaryCard(
              player: inspection.privatePlayer,
              notifier: notifier,
              savePath: inspection.path,
              // Reuse the panel's already compress-gated flag.
              editable: editable,
            ),
            const SizedBox(height: 16),
          ],
          if (editable) ...[
            _PrivateFStringEditor(
              strings: inspection.privateStrings,
              notifier: notifier,
            ),
            const SizedBox(height: 16),
          ],
```

Change it to (the FString block is deleted in Task 7; this task only adds the new card and the gating flag):

```dart
        children: [
          if (title == 'Player' && inspection.privatePlayer.hasData) ...[
            _PrivatePlayerSummaryCard(
              player: inspection.privatePlayer,
              notifier: notifier,
              savePath: inspection.path,
              // Reuse the panel's already compress-gated flag.
              editable: editable,
              // The typed hero stats card supersedes the heuristic
              // attribute editor whenever the strict typed parse is OK.
              showLegacyAttributes: !inspection.privateTypedVerified,
            ),
            const SizedBox(height: 16),
          ],
          if (title == 'Player' && inspection.privateTypedVerified) ...[
            HeroStatsCard(
              // New SaveInspection instance after every write/refresh —
              // changing identity drops pending edits and reloads.
              reloadKey: inspection,
              load: notifier.loadHeroAttributes,
              save: notifier.writeTypedValues,
              editable: editable,
            ),
            const SizedBox(height: 16),
          ],
          if (editable) ...[
            _PrivateFStringEditor(
              strings: inspection.privateStrings,
              notifier: notifier,
            ),
            const SizedBox(height: 16),
          ],
```

- [ ] **Step 3: Gate the legacy attributes editor in `_PrivatePlayerSummaryCard`**

Add the field and constructor parameter:

```dart
  const _PrivatePlayerSummaryCard({
    required this.player,
    required this.notifier,
    this.savePath,
    this.editable = true,
    this.showLegacyAttributes = true,
  });
```

```dart
  final bool showLegacyAttributes;
```

And change the attributes block (~line 1828):

```dart
            if (player.attributes.isNotEmpty) ...[
```

to:

```dart
            if (showLegacyAttributes && player.attributes.isNotEmpty) ...[
```

- [ ] **Step 4: Analyze and run the full Flutter test suite**

Run (from `apps/goresave`): `flutter analyze`
Expected: no new issues.

Run (from `apps/goresave`): `flutter test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/editor_page.dart
git commit -m "feat(ui): show typed hero stats card in the Player tab"
```

---

### Task 7: Delete the FString editor UI and its notifier method

The core write path `private.replaceFString` stays (API compatibility); only the UI surface and the now-unused Dart plumbing go.

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`
  - remove the `_PrivateFStringEditor` usage block in `_PrivatePanel.build` (after Task 6 it sits right after the HeroStatsCard block)
  - remove `class _PrivateFStringEditor` and `class _PrivateFStringEditorState` (currently ~lines 2756–2931, between `_InventoryDiagnostics`-related widgets and `_AllDataPanel`)
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart` — remove `writePrivateFString` (~lines 618–640)

- [ ] **Step 1: Remove the usage block**

In `_PrivatePanel.build`, delete:

```dart
          if (editable) ...[
            _PrivateFStringEditor(
              strings: inspection.privateStrings,
              notifier: notifier,
            ),
            const SizedBox(height: 16),
          ],
```

- [ ] **Step 2: Remove the two classes**

Delete `class _PrivateFStringEditor extends StatefulWidget { ... }` and `class _PrivateFStringEditorState extends State<_PrivateFStringEditor> { ... }` entirely (the state class contains the `widget.notifier.writePrivateFString(` call at ~line 2879 — that is the only caller in the app).

- [ ] **Step 3: Remove the notifier method**

In `editor_notifier.dart`, delete `Future<void> writePrivateFString({...})` (~lines 618–640, the method whose edit path is `'private.replaceFString'`).

Check there are no remaining references:

Run: `grep -rn "writePrivateFString\|_PrivateFStringEditor" apps/goresave/lib apps/goresave/test`
Expected: no matches. If a notifier test covers `writePrivateFString`, delete that test too — the behaviour is intentionally removed from the app layer.

Note: `inspection.privateStrings` stays — `_InventoryDiagnostics` (~line 2530) still uses it.

- [ ] **Step 4: Analyze and test**

Run (from `apps/goresave`): `flutter analyze`
Expected: no issues (especially no unused-element warnings).

Run (from `apps/goresave`): `flutter test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/editor_page.dart apps/goresave/lib/features/editor/domain/editor_notifier.dart apps/goresave/test
git commit -m "feat(ui): remove raw private FString editor from the Player tab"
```

---

### Task 8: Full verification

- [ ] **Step 1: Rust suite**

Run: `cargo test -p goresave_core`
Expected: PASS.

- [ ] **Step 2: Flutter suite + analyzer**

Run (from `apps/goresave`): `flutter analyze; flutter test`
Expected: clean analyze, all tests pass.

- [ ] **Step 3: End-to-end check against a real save (best effort)**

Copy a real save and run the committed examples (paths are for the dev machine; adjust if needed):

```powershell
Copy-Item "C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-007.sav" "$env:TEMP\g1r_e2e.sav" -Force
cargo build -p goresave_core --example dump_typed --example try_typed_edit --release
# Read the hero MaxHealth paths:
.\target\release\examples\dump_typed.exe "$env:TEMP\g1r_e2e.sav" ".\target\release\goresave_g1r_codec_host.exe" "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe" "AttributesByGlobalId {Hero} MaxHealth"
# Write BaseValue 65 through the typed path printed above, then re-run the dump to confirm the readback.
```

Expected: the dump shows `{/Script/G1R.AttributeSet_Health}` (not `{?}`) in the path; the edit applies and reads back. If the write fails with "runtime selftest worker timed out after 5000 ms", that is the pre-existing 5s codec-host timeout under load (see header note) — retry idle; it does not indicate a regression in this work.

- [ ] **Step 4: Manual app smoke test (optional, needs the desktop app)**

Launch the app, select a save, open the Player tab: the hero stats card shows grouped values (Main stats first, Advanced collapsed), the raw FString card is gone, editing MaxHealth and saving produces a backup message and refreshed values.
