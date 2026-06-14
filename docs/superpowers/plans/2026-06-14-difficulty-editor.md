# Difficulty Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show each savegame's difficulty (Novice/Gothic/Hard/Custom) in place of the useless "MainMap" badge, and let the user edit difficulty from a collapsible card in the Overview tab — writing the current save, and optionally the profile and all of the profile's saves, with mandatory backups.

**Architecture:** Difficulty is stored per-save in both the uncompressed public payload (`SaveGamePublicData`) and the compressed private payload (`SaveDataPayload`), and also in `PersistentDataList.sav` (`ProfileData`). The private save copy is authoritative for gameplay. Phase 1 surfaces the per-save difficulty (read-only) from the public payload and replaces the map badges. Phase 2 adds an all-or-nothing multi-file write via a new `write_difficulty` core command, reusing the existing GVAS fstring-splice, GSAV rebuild, private-edit/Kraken, and backup/atomic-replace machinery.

**Tech Stack:** Rust core (`crates/goresave_core`), Flutter + Riverpod app (`apps/goresave`). Tests: Rust `#[cfg(test)]` unit tests with synthetic GVAS buffers (`fstring()` helper); Flutter `flutter_test` widget tests.

**Reference spec:** `docs/superpowers/specs/2026-06-14-profile-difficulty-tab-design.md`

---

## Background facts (verified)

- Asset-path prefix: `/Script/Angelscript.`
- Preset values: `DifficultyPreset_{Easy,Standard,Hard,Custom}`; sub-settings
  `CombatDifficultySettings_{Easy,Standard,Hard}`, `ResourcesDifficultySettings_{…}`,
  `ProgressionDifficultySettings_{…}`.
- UI label mapping: `Easy`→Novice, `Standard`→Gothic, `Hard`→Hard, `Custom`→Custom.
- Properties: `m_difficultyPreset`, `m_customCombatSettings`,
  `m_customResourcesSettings`, `m_customProgressionSettings` are **ClassProperty**
  (value is an inline FString asset path). `m_FakeSloppyCombos` (Flow Helper),
  `m_PermanentDeath` (Permadeath) are **BoolProperty**.
- All difficulty properties are serialized on every save and every profile, so all
  edits are in-place splices (no structural insertion).
- Existing helpers to reuse:
  - `scan_fstrings(data, 0) -> Vec<FStringRef>` (fields: `value`, `utf16`,
    `len_offset`, `total_len`).
  - `value_after_property_in_range(refs, start, end, name) -> Option<String>`
    (lib.rs:2086).
  - `read_bool_property_in_range(payload, refs, start, end, name) -> Option<bool>`
    (lib.rs:2127).
  - `find_ref_in_range(refs, start, end, value) -> Option<usize>` (lib.rs:2138).
  - `replace_str_property_fstring_in_range(payload, refs, start, end, name, value)`
    (lib.rs:6334) — currently rejects non-`StrProperty`.
  - `write_str_property_value(payload, size_offset, value_ref, new_value)` (lib.rs:6077).
  - `split_gsav` / `build_gsav` / `replace_public_fstring` (lib.rs:6305).
  - `apply_private_edits` dispatch (lib.rs:4025); `apply_public_edit` (lib.rs:3994).
  - `write_save_internal` pipeline (lib.rs:3762): `create_backup_with_suffix`
    (lib.rs:1717), `shared_backup_suffix` (lib.rs:1729), `begin_replace` (lib.rs:1666),
    `PendingReplace::{commit,rollback}`.
  - Bool value byte location: `type_ref.len_offset + type_ref.total_len + 8`
    (from `read_bool_property_at`, lib.rs:3349).
  - Command dispatch match arm lives in `execute_json`'s handler near lib.rs:458.

---

# PHASE 1 — Surface & display difficulty (read-only, shippable)

## Task 1: Core — parse per-save difficulty from a GVAS payload

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add near the other parsing tests in the `tests` module (after lib.rs:6410):

```rust
#[test]
fn parse_difficulty_settings_reads_preset_and_bools() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&fstring("m_difficultyPreset"));
    payload.extend_from_slice(&fstring("ClassProperty"));
    payload.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Custom"));
    payload.extend_from_slice(&fstring("m_customCombatSettings"));
    payload.extend_from_slice(&fstring("ClassProperty"));
    payload.extend_from_slice(&fstring("/Script/Angelscript.CombatDifficultySettings_Hard"));
    payload.extend_from_slice(&fstring("m_FakeSloppyCombos"));
    payload.extend_from_slice(&fstring("BoolProperty"));
    payload.extend_from_slice(&[0u8; 8]); // array_index + size
    payload.push(1); // bool value byte
    payload.extend_from_slice(&fstring("m_PermanentDeath"));
    payload.extend_from_slice(&fstring("BoolProperty"));
    payload.extend_from_slice(&[0u8; 8]);
    payload.push(0);

    let d = parse_difficulty_settings(&payload);
    assert_eq!(d.preset.as_deref(), Some("DifficultyPreset_Custom"));
    assert_eq!(d.combat.as_deref(), Some("CombatDifficultySettings_Hard"));
    assert_eq!(d.flow_helper, Some(true));
    assert_eq!(d.permadeath, Some(false));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core parse_difficulty_settings_reads_preset_and_bools`
Expected: FAIL — `cannot find function parse_difficulty_settings` / `DifficultySettings`.

- [ ] **Step 3: Write minimal implementation**

Add the struct (near `ProfileSummary`, lib.rs:150) and the parser (near
`value_after_property_in_range`). The struct stores the **short class name** (the
substring after the last `.`), so the UI maps it without knowing the package prefix.

```rust
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultySettings {
    pub preset: Option<String>,
    pub combat: Option<String>,
    pub resources: Option<String>,
    pub progression: Option<String>,
    pub flow_helper: Option<bool>,
    pub permadeath: Option<bool>,
}

fn short_class_name(path: &str) -> String {
    path.rsplit('.').next().unwrap_or(path).to_string()
}

fn parse_difficulty_settings(payload: &[u8]) -> DifficultySettings {
    let refs = scan_fstrings(payload, 0);
    let end = refs.len();
    let class = |name: &str| {
        value_after_property_in_range(&refs, 0, end, name).map(|v| short_class_name(&v))
    };
    DifficultySettings {
        preset: class("m_difficultyPreset"),
        combat: class("m_customCombatSettings"),
        resources: class("m_customResourcesSettings"),
        progression: class("m_customProgressionSettings"),
        flow_helper: read_bool_property_in_range(payload, &refs, 0, end, "m_FakeSloppyCombos"),
        permadeath: read_bool_property_in_range(payload, &refs, 0, end, "m_PermanentDeath"),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core parse_difficulty_settings_reads_preset_and_bools`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): parse per-save difficulty settings from a GVAS payload"
```

## Task 2: Core — expose difficulty on inspect_save and list_saves

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

The save's public payload bytes come from `split_gsav(data).public_payload`. Add a small
helper that extracts difficulty from a full GSAV file, returning `None` for non-GSAV.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn difficulty_for_gsav_bytes_none_for_non_gsav() {
    assert!(difficulty_for_gsav_bytes(b"NOPE").is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core difficulty_for_gsav_bytes_none_for_non_gsav`
Expected: FAIL — `cannot find function difficulty_for_gsav_bytes`.

- [ ] **Step 3: Write minimal implementation**

```rust
fn difficulty_for_gsav_bytes(data: &[u8]) -> Option<DifficultySettings> {
    if !data.starts_with(b"GSAV") {
        return None;
    }
    let parts = split_gsav(data).ok()?;
    Some(parse_difficulty_settings(parts.public_payload))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core difficulty_for_gsav_bytes_none_for_non_gsav`
Expected: PASS.

- [ ] **Step 5: Wire into list_saves (SaveListItem)**

In the `SaveListItem` struct (search `struct SaveListItem`), add:
```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    difficulty: Option<DifficultySettings>,
```
In the scan builder (lib.rs:554, the `Ok(info)` arm), compute once before `saves.push`:
```rust
                let difficulty = difficulty_for_gsav_bytes(&data);
```
and add `difficulty,` to the `SaveListItem { … }`. In the `Err` arm add `difficulty: None,`.

- [ ] **Step 6: Wire into inspect output**

Find where `inspect_save` / `inspect_bytes` builds its returned JSON object (the object
that includes `"public"`). Add a top-level field:
```rust
        "difficulty": difficulty_for_gsav_bytes(data),
```
(Use the in-scope full-file byte slice; in `inspect_bytes` that is the `data` param.)

- [ ] **Step 7: Build & run the core test suite**

Run: `cargo test -p goresave_core`
Expected: PASS (no regressions).

- [ ] **Step 8: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): expose per-save difficulty on inspect_save and list_saves"
```

## Task 3: Dart — difficulty model + fields on SaveSlot/SaveInspection

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_models.dart`

- [ ] **Step 1: Write the failing test**

Create `apps/goresave/test/difficulty_settings_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

void main() {
  test('DifficultySettings.fromJson maps fields and label', () {
    final d = DifficultySettings.fromJson({
      'preset': 'DifficultyPreset_Custom',
      'combat': 'CombatDifficultySettings_Hard',
      'flowHelper': true,
      'permadeath': false,
    });
    expect(d.presetLabel, 'Custom');
    expect(d.combatLabel, 'Hard');
    expect(d.flowHelper, true);
    expect(d.permadeath, false);
  });

  test('presetLabel maps Easy to Novice and Standard to Gothic', () {
    expect(DifficultySettings(preset: 'DifficultyPreset_Easy').presetLabel, 'Novice');
    expect(DifficultySettings(preset: 'DifficultyPreset_Standard').presetLabel, 'Gothic');
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/goresave && flutter test test/difficulty_settings_test.dart`
Expected: FAIL — `DifficultySettings` undefined.

- [ ] **Step 3: Write minimal implementation**

Add to `editor_models.dart` (top-level, near `ProfileSummary`):

```dart
/// Maps a difficulty class short-name suffix to its UI label.
String _difficultyLevelLabel(String? className) {
  if (className == null) return '-';
  if (className.endsWith('_Easy')) return 'Novice';
  if (className.endsWith('_Standard')) return 'Gothic';
  if (className.endsWith('_Hard')) return 'Hard';
  if (className.endsWith('_Custom')) return 'Custom';
  return className;
}

class DifficultySettings {
  const DifficultySettings({
    this.preset,
    this.combat,
    this.resources,
    this.progression,
    this.flowHelper,
    this.permadeath,
  });

  factory DifficultySettings.fromJson(Map<String, Object?> json) {
    return DifficultySettings(
      preset: json['preset'] as String?,
      combat: json['combat'] as String?,
      resources: json['resources'] as String?,
      progression: json['progression'] as String?,
      flowHelper: json['flowHelper'] as bool?,
      permadeath: json['permadeath'] as bool?,
    );
  }

  static DifficultySettings? maybeFromJson(Object? json) =>
      json is Map ? DifficultySettings.fromJson(json.cast<String, Object?>()) : null;

  final String? preset;
  final String? combat;
  final String? resources;
  final String? progression;
  final bool? flowHelper;
  final bool? permadeath;

  String get presetLabel => _difficultyLevelLabel(preset);
  String get combatLabel => _difficultyLevelLabel(combat);
  String get resourcesLabel => _difficultyLevelLabel(resources);
  String get progressionLabel => _difficultyLevelLabel(progression);
}
```

Add `final DifficultySettings? difficulty;` plus `this.difficulty` to the constructors
of **both** `SaveSlot` (lib line ~168) and `SaveInspection`, and in each `fromJson`:
```dart
      difficulty: DifficultySettings.maybeFromJson(json['difficulty']),
```
(For `SaveInspection.fromJson` the difficulty is a top-level key, same as `format`.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/goresave && flutter test test/difficulty_settings_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/editor_models.dart apps/goresave/test/difficulty_settings_test.dart
git commit -m "feat(app): add DifficultySettings model and fields on SaveSlot/SaveInspection"
```

## Task 4: Dart — replace map badges with difficulty label

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`

- [ ] **Step 1: Replace the sidebar subtitle (editor_page.dart:520-523)**

```dart
  final difficulty = save.difficulty?.presetLabel;
  if (difficulty != null && difficulty != '-') {
    parts.add(difficulty);
  }
```
(Delete the previous `mapName` block.)

- [ ] **Step 2: Replace the header pill (editor_page.dart:1068-1072)**

```dart
                            if (inspection.difficulty?.presetLabel != null &&
                                inspection.difficulty!.presetLabel != '-')
                              _InfoPill(
                                icon: Icons.local_fire_department_outlined,
                                label: inspection.difficulty!.presetLabel,
                              ),
```

- [ ] **Step 3: Replace the diagnostics metric (editor_page.dart:938-939)**

```dart
                        if (inspection.difficulty?.presetLabel != null &&
                            inspection.difficulty!.presetLabel != '-')
                          'Difficulty': inspection.difficulty!.presetLabel,
```

- [ ] **Step 4: Verify it compiles / analyzer clean**

Run: `cd apps/goresave && flutter analyze lib/features/editor/ui/editor_page.dart`
Expected: No issues (no remaining references to `save.mapName` / `inspection.mapName`
in these three spots).

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/editor_page.dart
git commit -m "feat(app): show per-save difficulty in place of the MainMap badge"
```

## Task 5: Dart — read-only Difficulty card in Overview

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`

Mirror the existing `_CollapsibleCardHeader` card (the "Diagnostics & details" card,
~lib.939). Read the Overview panel build method first to find the column where cards are
listed, and insert the new card just below the header card.

- [ ] **Step 1: Add the card widget**

```dart
class _DifficultyCard extends StatefulWidget {
  const _DifficultyCard({required this.inspection});
  final SaveInspection inspection;
  @override
  State<_DifficultyCard> createState() => _DifficultyCardState();
}

class _DifficultyCardState extends State<_DifficultyCard> {
  bool _expanded = false;
  @override
  Widget build(BuildContext context) {
    final d = widget.inspection.difficulty;
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _CollapsibleCardHeader(
              icon: Icons.local_fire_department_outlined,
              title: 'Difficulty',
              subtitle: d?.presetLabel ?? 'Unknown',
              expanded: _expanded,
              onToggle: () => setState(() => _expanded = !_expanded),
            ),
            if (_expanded && d != null) ...[
              const SizedBox(height: 8),
              _MetricGrid(
                metrics: {
                  'Preset': d.presetLabel,
                  'Close Combat Flow Helper': d.flowHelper == true ? 'On' : 'Off',
                  'Permadeath': d.permadeath == true ? 'On' : 'Off',
                  'Combat': d.combatLabel,
                  'Resources': d.resourcesLabel,
                  'Progression': d.progressionLabel,
                },
              ),
            ],
          ],
        ),
      ),
    );
  }
}
```

- [ ] **Step 2: Insert it in the Overview panel**

In the Overview panel's `Column`/`ListView` children, add below the header card:
```dart
            _DifficultyCard(inspection: inspection),
```
(Use the `inspection` variable already in scope in that panel.)

- [ ] **Step 3: Verify**

Run: `cd apps/goresave && flutter analyze lib/features/editor/ui/editor_page.dart`
Expected: No issues.

- [ ] **Step 4: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/editor_page.dart
git commit -m "feat(app): add read-only Difficulty card to the Overview tab"
```

**Phase 1 checkpoint:** Build and run the app (`/run` or `flutter run -d windows` in
`apps/goresave`), open a save, confirm the sidebar/header show Novice/Gothic/Hard/Custom
and the Difficulty card shows correct values. This phase is shippable on its own.

---

# PHASE 2 — Edit difficulty (writes)

## Task 6: Core — generalize the fstring splice to ClassProperty

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn replace_class_property_value_in_range_rewrites_path() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&fstring("m_difficultyPreset"));
    payload.extend_from_slice(&fstring("ClassProperty"));
    payload.extend_from_slice(&[0u8; 4]); // flags
    payload.extend_from_slice(&0u32.to_le_bytes()); // size (rewritten)
    payload.push(0); // tag
    payload.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Custom"));

    let refs = scan_fstrings(&payload, 0);
    replace_class_or_str_property_in_range(
        &mut payload, &refs, 0, refs.len(),
        "m_difficultyPreset",
        "/Script/Angelscript.DifficultyPreset_Easy",
    )
    .unwrap();

    let refs2 = scan_fstrings(&payload, 0);
    assert_eq!(
        value_after_property_in_range(&refs2, 0, refs2.len(), "m_difficultyPreset").as_deref(),
        Some("/Script/Angelscript.DifficultyPreset_Easy"),
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core replace_class_property_value_in_range_rewrites_path`
Expected: FAIL — `cannot find function replace_class_or_str_property_in_range`.

- [ ] **Step 3: Write minimal implementation**

Copy `replace_str_property_fstring_in_range` (lib.rs:6334) to a new function that accepts
`StrProperty` or `ClassProperty` (value layout is identical):

```rust
fn replace_class_or_str_property_in_range(
    payload: &mut Vec<u8>,
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    property_name: &str,
    new_value: &str,
) -> Result<(), CoreError> {
    let name_idx = find_ref_in_range(refs, start_idx, end_idx, property_name)
        .ok_or_else(|| CoreError::Parse(format!("property {property_name} was not found")))?;
    let type_ref = refs
        .get(name_idx + 1)
        .ok_or_else(|| CoreError::Parse(format!("type for {property_name} was not found")))?;
    if type_ref.value != "StrProperty" && type_ref.value != "ClassProperty" {
        return Err(CoreError::Parse(format!(
            "property {property_name} is not a StrProperty/ClassProperty"
        )));
    }
    let value_ref = refs
        .get(name_idx + 2)
        .ok_or_else(|| CoreError::Parse(format!("value for {property_name} was not found")))?;
    if value_ref.utf16 {
        return Err(CoreError::UnsupportedEdit(
            "UTF-16 FString replacement is not implemented yet".to_string(),
        ));
    }
    let size_offset = type_ref.len_offset + type_ref.total_len + 4;
    write_str_property_value(payload, size_offset, value_ref, new_value)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core replace_class_property_value_in_range_rewrites_path`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): in-place splice for ClassProperty asset-path values"
```

## Task 7: Core — in-place BoolProperty writer

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn write_bool_property_in_range_flips_value_byte() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&fstring("m_PermanentDeath"));
    payload.extend_from_slice(&fstring("BoolProperty"));
    payload.extend_from_slice(&[0u8; 8]);
    payload.push(0); // value byte

    let refs = scan_fstrings(&payload, 0);
    write_bool_property_in_range(&mut payload, &refs, 0, refs.len(), "m_PermanentDeath", true)
        .unwrap();

    let refs2 = scan_fstrings(&payload, 0);
    assert_eq!(
        read_bool_property_in_range(&payload, &refs2, 0, refs2.len(), "m_PermanentDeath"),
        Some(true),
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core write_bool_property_in_range_flips_value_byte`
Expected: FAIL — `cannot find function write_bool_property_in_range`.

- [ ] **Step 3: Write minimal implementation**

```rust
fn write_bool_property_in_range(
    payload: &mut [u8],
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    name: &str,
    value: bool,
) -> Result<(), CoreError> {
    let name_idx = find_ref_in_range(refs, start_idx, end_idx, name)
        .ok_or_else(|| CoreError::Parse(format!("property {name} was not found")))?;
    let type_ref = refs
        .get(name_idx + 1)
        .ok_or_else(|| CoreError::Parse(format!("type for {name} was not found")))?;
    if type_ref.value != "BoolProperty" {
        return Err(CoreError::Parse(format!("property {name} is not a BoolProperty")));
    }
    let offset = type_ref.len_offset + type_ref.total_len + 8;
    *payload
        .get_mut(offset)
        .ok_or_else(|| CoreError::Parse(format!("bool value for {name} is out of range")))? =
        if value { 1 } else { 0 };
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core write_bool_property_in_range_flips_value_byte`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): in-place BoolProperty value writer"
```

## Task 8: Core — apply a difficulty change to a GVAS payload (range)

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

This is the shared mutator for a **single-record GVAS payload** (a save's public payload
or its decoded private payload — each holds one `SaveGamePublicData`/`SaveDataPayload`
record). It applies only the requested fields, mapping UI levels to asset paths, and
re-scans before each splice because splices shift byte offsets. The multi-profile
`PersistentDataList` case is handled separately in Task 10.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn apply_save_difficulty_sets_preset_and_subsettings() {
    let mut payload = Vec::new();
    for name in ["m_difficultyPreset", "m_customCombatSettings",
                 "m_customResourcesSettings", "m_customProgressionSettings"] {
        payload.extend_from_slice(&fstring(name));
        payload.extend_from_slice(&fstring("ClassProperty"));
        payload.extend_from_slice(&[0u8; 4]);
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.push(0);
        payload.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Easy"));
    }
    payload.extend_from_slice(&fstring("m_PermanentDeath"));
    payload.extend_from_slice(&fstring("BoolProperty"));
    payload.extend_from_slice(&[0u8; 8]);
    payload.push(0);

    let req = DifficultyRequest {
        preset: "Custom".to_string(),
        combat: Some("Hard".to_string()),
        resources: None,
        progression: None,
        flow_helper: None,
        permadeath: Some(true),
    };
    apply_save_difficulty(&mut payload, &req).unwrap();

    let refs = scan_fstrings(&payload, 0);
    let end = refs.len();
    assert_eq!(
        value_after_property_in_range(&refs, 0, end, "m_difficultyPreset").as_deref(),
        Some("/Script/Angelscript.DifficultyPreset_Custom"),
    );
    assert_eq!(
        value_after_property_in_range(&refs, 0, end, "m_customCombatSettings").as_deref(),
        Some("/Script/Angelscript.CombatDifficultySettings_Hard"),
    );
    // Resources/Progression had no explicit level -> default to Gothic (Standard) for Custom.
    assert_eq!(
        value_after_property_in_range(&refs, 0, end, "m_customResourcesSettings").as_deref(),
        Some("/Script/Angelscript.ResourcesDifficultySettings_Standard"),
    );
    assert_eq!(
        read_bool_property_in_range(&payload, &refs, 0, end, "m_PermanentDeath"),
        Some(true),
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core apply_save_difficulty_sets_preset_and_subsettings`
Expected: FAIL — `DifficultyRequest` / `apply_save_difficulty` undefined.

- [ ] **Step 3: Write minimal implementation**

```rust
const ANGELSCRIPT: &str = "/Script/Angelscript.";

fn level_suffix(label: &str) -> Result<&'static str, CoreError> {
    match label {
        "Novice" => Ok("Easy"),
        "Gothic" => Ok("Standard"),
        "Hard" => Ok("Hard"),
        other => Err(CoreError::InvalidRequest(format!("unknown difficulty level {other}"))),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DifficultyRequest {
    preset: String, // Novice|Gothic|Hard|Custom
    #[serde(default)] combat: Option<String>,
    #[serde(default)] resources: Option<String>,
    #[serde(default)] progression: Option<String>,
    #[serde(default)] flow_helper: Option<bool>,
    #[serde(default)] permadeath: Option<bool>,
}

impl DifficultyRequest {
    /// Resolved short class names (no package prefix).
    fn class_edits(&self) -> Result<Vec<(&'static str, String)>, CoreError> {
        let preset = if self.preset == "Custom" {
            "DifficultyPreset_Custom".to_string()
        } else {
            format!("DifficultyPreset_{}", level_suffix(&self.preset)?)
        };
        // Sub-setting levels: explicit for Custom (default Gothic); mirror preset otherwise.
        let lvl = |explicit: &Option<String>| -> Result<&'static str, CoreError> {
            if self.preset == "Custom" {
                level_suffix(explicit.as_deref().unwrap_or("Gothic"))
            } else {
                level_suffix(&self.preset)
            }
        };
        Ok(vec![
            ("m_difficultyPreset", preset),
            ("m_customCombatSettings", format!("CombatDifficultySettings_{}", lvl(&self.combat)?)),
            ("m_customResourcesSettings", format!("ResourcesDifficultySettings_{}", lvl(&self.resources)?)),
            ("m_customProgressionSettings", format!("ProgressionDifficultySettings_{}", lvl(&self.progression)?)),
        ])
    }
    /// Permadeath is locked off for Novice.
    fn resolved_permadeath(&self) -> Option<bool> {
        if self.preset == "Novice" { Some(false) } else { self.permadeath }
    }
}

fn apply_save_difficulty(payload: &mut Vec<u8>, req: &DifficultyRequest) -> Result<(), CoreError> {
    // Class properties: re-scan before each splice (lengths shift).
    for (name, class) in req.class_edits()? {
        let refs = scan_fstrings(payload, 0);
        let end = refs.len();
        if find_ref_in_range(&refs, 0, end, name).is_some() {
            replace_class_or_str_property_in_range(
                payload, &refs, 0, end, name, &format!("{ANGELSCRIPT}{class}"),
            )?;
        }
    }
    // Bools.
    if let Some(perma) = req.resolved_permadeath() {
        let refs = scan_fstrings(payload, 0);
        write_bool_property_in_range(payload, &refs, 0, refs.len(), "m_PermanentDeath", perma)?;
    }
    if let Some(flow) = req.flow_helper {
        let refs = scan_fstrings(payload, 0);
        write_bool_property_in_range(payload, &refs, 0, refs.len(), "m_FakeSloppyCombos", flow)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core apply_save_difficulty_sets_preset_and_subsettings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): apply a difficulty request to a single-record GVAS payload"
```

## Task 9: Core — write difficulty into one save (public + private)

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

Public payload: splice via `split_gsav`/`build_gsav` (like `replace_public_fstring`).
Private payload: route through `apply_private_edits` with a new edit kind.

- [ ] **Step 1: Add a private edit arm**

In `apply_private_edits`'s match (lib.rs:4044), add:
```rust
            "private.difficulty.set" => {
                parse_private_difficulty_edit(edit).map(PrivateEdit::Difficulty)
            }
```
Add a `Difficulty(DifficultyRequest)` variant to the `PrivateEdit` enum, a
`parse_private_difficulty_edit(edit) -> Result<DifficultyRequest, CoreError>` that
deserializes `edit.value` into `DifficultyRequest`, and in the payload-mutation step
(where each `PrivateEdit` is applied to the decoded private payload) handle
`PrivateEdit::Difficulty(req) => apply_save_difficulty(payload, &req)?`.
(Find the apply loop by searching `PrivateEdit::TypedSetValue` in the mutation function.)

- [ ] **Step 2: Add the public + private orchestrator with a test**

Test (uses the GSAV test scaffolding already present — search for an existing
`build_gsav`-based test to copy the public/stream/trailer setup):

```rust
#[test]
fn write_difficulty_into_save_updates_public_payload() {
    // Build a GSAV whose public payload carries a Custom preset, then set Novice.
    let mut public = Vec::new();
    public.extend_from_slice(&fstring("m_difficultyPreset"));
    public.extend_from_slice(&fstring("ClassProperty"));
    public.extend_from_slice(&[0u8; 4]);
    public.extend_from_slice(&0u32.to_le_bytes());
    public.push(0);
    public.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Custom"));
    public.extend_from_slice(&fstring("m_PermanentDeath"));
    public.extend_from_slice(&fstring("BoolProperty"));
    public.extend_from_slice(&[0u8; 8]);
    public.push(1);

    let gsav = build_gsav(2, &public, &[], &[]);
    let req = DifficultyRequest {
        preset: "Novice".into(), combat: None, resources: None,
        progression: None, flow_helper: None, permadeath: Some(true),
    };
    let out = write_difficulty_into_save_public(&gsav, &req).unwrap();
    let d = difficulty_for_gsav_bytes(&out).unwrap();
    assert_eq!(d.preset.as_deref(), Some("DifficultyPreset_Easy"));
    assert_eq!(d.permadeath, Some(false)); // Novice forces off
}
```

Implementation:
```rust
fn write_difficulty_into_save_public(
    data: &[u8],
    req: &DifficultyRequest,
) -> Result<Vec<u8>, CoreError> {
    let parts = split_gsav(data)?;
    let mut public_payload = parts.public_payload.to_vec();
    let compressed_stream = parts.compressed_stream.to_vec();
    let trailer = parts.trailer.to_vec();
    let version = parts.version;
    apply_save_difficulty(&mut public_payload, req)?;
    Ok(build_gsav(version, &public_payload, &compressed_stream, &trailer))
}
```

(The private side is exercised through `apply_private_edits` in Task 11's command test,
which needs the codec backend; keep the unit test here to the public path.)

- [ ] **Step 3: Run tests**

Run: `cargo test -p goresave_core write_difficulty_into_save_updates_public_payload`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): write difficulty into a save's public payload + private edit kind"
```

## Task 10: Core — write difficulty into a profile range in PersistentDataList

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

Reuse the profile boundary logic from `parse_profile_summaries` (lib.rs:904): locate the
target profile's `[start_idx, end_idx)` ref range by `m_ProfileId`, then splice within it.
Because splices shift bytes, re-scan and re-derive the range bound after each edit.

- [ ] **Step 1: Write the failing test** (synthetic two-profile buffer)

```rust
#[test]
fn write_profile_difficulty_targets_only_the_named_profile() {
    let mut data = b"GVAS".to_vec();
    data.extend_from_slice(&fstring("m_Profiles"));
    // profile 0
    data.extend_from_slice(&fstring("m_ProfileName"));
    data.extend_from_slice(&fstring("m_ProfileId"));
    data.extend_from_slice(&fstring("IntProperty"));
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&4u32.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&0i32.to_le_bytes());
    data.extend_from_slice(&fstring("m_difficultyPreset"));
    data.extend_from_slice(&fstring("ClassProperty"));
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Custom"));
    // profile 1
    data.extend_from_slice(&fstring("m_ProfileName"));
    data.extend_from_slice(&fstring("m_ProfileId"));
    data.extend_from_slice(&fstring("IntProperty"));
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&4u32.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&1i32.to_le_bytes());
    data.extend_from_slice(&fstring("m_difficultyPreset"));
    data.extend_from_slice(&fstring("ClassProperty"));
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Easy"));
    data.extend_from_slice(&fstring("SavedDataVersion"));

    let req = DifficultyRequest {
        preset: "Hard".into(), combat: None, resources: None,
        progression: None, flow_helper: None, permadeath: None,
    };
    write_profile_difficulty(&mut data, 1, &req).unwrap();

    // Profile 1 changed, profile 0 untouched.
    let refs = scan_fstrings(&data, 0);
    let presets: Vec<_> = refs.iter().filter(|r| r.value.contains("DifficultyPreset_")).map(|r| r.value.clone()).collect();
    assert!(presets.iter().any(|p| p.ends_with("DifficultyPreset_Custom")));
    assert!(presets.iter().any(|p| p.ends_with("DifficultyPreset_Hard")));
    assert!(!presets.iter().any(|p| p.ends_with("DifficultyPreset_Easy")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core write_profile_difficulty_targets_only_the_named_profile`
Expected: FAIL — `write_profile_difficulty` undefined.

- [ ] **Step 3: Write minimal implementation**

```rust
fn profile_range_by_id(refs: &[FStringRef], profile_id: i32, payload: &[u8]) -> Option<(usize, usize)> {
    let profiles_idx = refs.iter().position(|r| r.value == "m_Profiles")?;
    let profiles_end = refs.iter().enumerate().skip(profiles_idx + 1)
        .find(|(_, r)| r.value == "SavedDataVersion").map(|(i, _)| i)
        .unwrap_or(refs.len());
    let starts: Vec<usize> = refs.iter().enumerate().take(profiles_end).skip(profiles_idx + 1)
        .filter_map(|(i, r)| (r.value == "m_ProfileName").then_some(i)).collect();
    for (ord, &start) in starts.iter().enumerate() {
        let end = starts.get(ord + 1).copied().unwrap_or(profiles_end);
        let id = read_i32_property_in_range(payload, refs, start, end, "m_ProfileId")
            .unwrap_or(ord as i32);
        if id == profile_id {
            return Some((start, end));
        }
    }
    None
}

fn write_profile_difficulty(
    data: &mut Vec<u8>,
    profile_id: i32,
    req: &DifficultyRequest,
) -> Result<(), CoreError> {
    if !data.starts_with(b"GVAS") {
        return Err(CoreError::Parse("PersistentDataList.sav is not a GVAS file".into()));
    }
    // Apply field-by-field, re-locating the profile range after each splice
    // (reuses the same resolution helpers as apply_save_difficulty for consistency).
    for (name, class) in req.class_edits()? {
        let refs = scan_fstrings(data, 0);
        let (s, e) = profile_range_by_id(&refs, profile_id, data)
            .ok_or_else(|| CoreError::Validation(format!("profile {profile_id} not found")))?;
        // Skip silently if a field is absent in this profile range.
        if find_ref_in_range(&refs, s, e, name).is_some() {
            replace_class_or_str_property_in_range(data, &refs, s, e, name, &format!("{ANGELSCRIPT}{class}"))?;
        }
    }
    for (name, val) in [("m_PermanentDeath", req.resolved_permadeath()), ("m_FakeSloppyCombos", req.flow_helper)] {
        let Some(v) = val else { continue };
        let refs = scan_fstrings(data, 0);
        let (s, e) = profile_range_by_id(&refs, profile_id, data)
            .ok_or_else(|| CoreError::Validation(format!("profile {profile_id} not found")))?;
        if find_ref_in_range(&refs, s, e, name).is_some() {
            write_bool_property_in_range(data, &refs, s, e, name, v)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core write_profile_difficulty_targets_only_the_named_profile`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): write difficulty into a single profile range in PersistentDataList"
```

## Task 11: Core — `write_difficulty` command (multi-target, backed up, atomic)

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

Payload:
```json
{
  "command": "write_difficulty",
  "difficulty": { "preset": "...", "combat": "...", "resources": "...",
                  "progression": "...", "flowHelper": true, "permadeath": false },
  "targets": { "saves": ["<path>", ...], "profile": { "path": "<PersistentDataList.sav>", "profileId": 1 } },
  "binaryHost": { ... },
  "backup": true
}
```

- [ ] **Step 1: Add the dispatch arm**

In `execute_json`'s match (after the `"write_save"` arm, lib.rs:458):
```rust
        "write_difficulty" => {
            let req: DifficultyRequest = serde_json::from_value(
                payload.get("difficulty").cloned().unwrap_or(Value::Null),
            ).map_err(|e| CoreError::InvalidRequest(e.to_string()))?;
            let backup = payload.get("backup").and_then(Value::as_bool).unwrap_or(true);
            let codec_backend = payload.get("binaryHost").map(binary_host_backend_from_config).transpose()?;
            let codec_backend = codec_backend.as_ref().map(|b| b as &dyn codec_backend::CodecBackend);
            let targets = payload.get("targets").cloned().unwrap_or(Value::Null);
            Ok(write_difficulty_internal(&req, &targets, backup, codec_backend)?)
        }
```

- [ ] **Step 2: Write the orchestrator with a multi-target file test**

Test (writes two temp GSAV saves + a temp PersistentDataList, runs the orchestrator,
asserts all three updated and `.bak` backups exist). Use `tempfile::tempdir` like
existing write tests; reuse a GSAV save built as in Task 9 and a PersistentDataList as in
Task 10. Assert: each target's difficulty re-parses to the new preset, and a backup file
was created next to each.

Implementation:
```rust
struct DifficultyWritePlan {
    path: PathBuf,
    original: Vec<u8>,
    edited: Vec<u8>,
}

fn write_difficulty_internal(
    req: &DifficultyRequest,
    targets: &Value,
    backup: bool,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Value, CoreError> {
    let mut plans: Vec<DifficultyWritePlan> = Vec::new();

    // Saves: edit public payload + private payload.
    if let Some(saves) = targets.get("saves").and_then(Value::as_array) {
        for save in saves {
            let path = PathBuf::from(save.as_str().ok_or_else(|| {
                CoreError::InvalidRequest("targets.saves entries must be strings".into())
            })?);
            let original = fs::read(&path)?;
            let public_done = write_difficulty_into_save_public(&original, req)?;
            // Private payload via the existing private-edit pipeline.
            let edit = Edit {
                path: "private.difficulty.set".to_string(),
                value: serde_json::to_value(req).map_err(|e| CoreError::InvalidRequest(e.to_string()))?,
            };
            let edited = apply_private_edits(&public_done, &[&edit], codec_backend)?;
            inspect_bytes(&edited, None, false)?;
            if edited.starts_with(b"GSAV") {
                let rebuilt = rebuild_gsav_preserving_stream(&edited)?;
                if rebuilt != edited {
                    return Err(CoreError::Validation(
                        "edited GSAV does not rebuild byte-identically".into(),
                    ));
                }
            }
            plans.push(DifficultyWritePlan { path, original, edited });
        }
    }

    // Profile: edit PersistentDataList range.
    if let Some(profile) = targets.get("profile").filter(|v| !v.is_null()) {
        let path = PathBuf::from(profile.get("path").and_then(Value::as_str).ok_or_else(|| {
            CoreError::InvalidRequest("targets.profile.path is required".into())
        })?);
        let profile_id = profile.get("profileId").and_then(Value::as_i64).ok_or_else(|| {
            CoreError::InvalidRequest("targets.profile.profileId is required".into())
        })? as i32;
        let original = fs::read(&path)?;
        let mut edited = original.clone();
        write_profile_difficulty(&mut edited, profile_id, req)?;
        plans.push(DifficultyWritePlan { path, original, edited });
    }

    if plans.is_empty() {
        return Err(CoreError::InvalidRequest("write_difficulty requires at least one target".into()));
    }

    // One shared backup suffix across every target.
    let changed: Vec<&DifficultyWritePlan> = plans.iter().filter(|p| p.original != p.edited).collect();
    if backup {
        let paths: Vec<&Path> = changed.iter().map(|p| p.path.as_path()).collect();
        if !paths.is_empty() {
            let suffix = shared_backup_suffix(&paths);
            for p in &changed {
                create_backup_with_suffix(&p.path, &suffix)?;
            }
        }
    }

    // Stage tmp files + validate each by re-parsing.
    let mut tmps = Vec::new();
    for p in &changed {
        let tmp = p.path.with_extension("sav.tmp-goresave");
        fs::write(&tmp, &p.edited)?;
        inspect_save(&tmp, false)?;
        tmps.push((p.path.clone(), tmp));
    }
    // Atomic replace all; rollback everything if any fails.
    let mut committed: Vec<PendingReplace> = Vec::new();
    for (target, tmp) in &tmps {
        match begin_replace(target, tmp) {
            Ok(pending) => committed.push(pending),
            Err(err) => {
                for p in committed { p.rollback(); }
                return Err(err);
            }
        }
    }
    for p in committed { p.commit(); }

    for (target, _) in &tmps {
        invalidate_decoded_payload_cache(target);
    }
    Ok(json!({
        "targetsWritten": changed.len(),
        "paths": changed.iter().map(|p| p.path.display().to_string()).collect::<Vec<_>>(),
    }))
}
```

(`DifficultyRequest` already derives both `Serialize` and `Deserialize` from Task 8, so
`serde_json::to_value(req)` for the private edit works.)

> NOTE: `begin_replace` returns a `PendingReplace`; confirm its `commit`/`rollback`
> signatures (lib.rs:1666) and that `commit` consumes `self`. Adjust the loop if
> `PendingReplace` is not `Send`/movable as written — the `write_save_internal` pattern
> at lib.rs:3854 is the reference.

- [ ] **Step 3: Run the test + full suite**

Run: `cargo test -p goresave_core`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): write_difficulty command — multi-target, backed up, atomic"
```

## Task 12: Manual authority check (verification, not code)

- [ ] Edit one real save's difficulty via the new command (e.g. a small CLI/test
  harness or the app once Task 14 lands), launch the game, load that save, and confirm
  the difficulty changed in-game. If only the public or only the private copy matters,
  note it in the spec and narrow `write_difficulty_into_save_public` / the private edit
  accordingly. Record the result in the spec's "Authority note".

## Task 13: Dart — notifier method for write_difficulty

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart`

- [ ] **Step 1: Add the method**

Mirror the existing `write_save` tracked-execute (`editor_notifier.dart:297-320`):
```dart
  Future<void> writeDifficulty({
    required Map<String, Object?> difficulty,
    required List<String> savePaths,
    ({String path, int profileId})? profile,
  }) async {
    final targets = <String, Object?>{
      'saves': savePaths,
      if (profile != null)
        'profile': {'path': profile.path, 'profileId': profile.profileId},
    };
    await _runTrackedWrite('write_difficulty', payload: {
      'difficulty': difficulty,
      'targets': targets,
      'backup': true,
      // include binaryHost the same way write_save does (copy that wiring)
    });
  }
```
(Use whatever helper `write_save` uses to attach `binaryHost`; copy it verbatim.)

- [ ] **Step 2: Analyze**

Run: `cd apps/goresave && flutter analyze lib/features/editor/domain/editor_notifier.dart`
Expected: No issues.

- [ ] **Step 3: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/editor_notifier.dart
git commit -m "feat(app): EditorNotifier.writeDifficulty dispatch"
```

## Task 14: Dart — make the Difficulty card editable

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`

Convert `_DifficultyCard` to an editing form with its own buffer.

- [ ] **Step 1: Replace the card body with editable controls**

State fields: `String _preset; bool _flow; bool _perma; String _combat/_resources/_progression;`
initialized from `widget.inspection.difficulty` (labels), plus `bool _alsoProfile=false,
_allSaves=false; bool _dirty=false;`.

Build:
- Preset selector: `SegmentedButton`/`ChoiceChip`s for `['Novice','Gothic','Hard','Custom']`.
- Custom block enabled only when `_preset=='Custom'`:
  - two `SwitchListTile`s — "Close Combat Flow Helper" (`_flow`), "Permadeath"
    (`_perma`, disabled when `_preset=='Novice'`).
  - three level pickers (Combat/Resources/Progression) over `['Novice','Gothic','Hard']`,
    enabled only for Custom.
- Editability per the matrix: Flow always enabled; Permadeath enabled unless Novice;
  sub-pickers enabled only for Custom.
- An explanation `Text` block: "Difficulty is stored in this save (gameplay), in the
  profile (menu default), and in every other save. This edits only the current save
  unless you tick the boxes below."
- Two `CheckboxListTile`s: "Also update the profile", "Apply to all saves of this
  profile".
- A Save and a Reset `FilledButton`/`TextButton`, enabled when `_dirty`.

- [ ] **Step 2: Wire Save**

On Save, assemble the difficulty map and targets and call the notifier. The card needs
the `EditorNotifier`, the current save path, the resolved profile, and the profile's save
paths — pass them in via constructor from the Overview panel (which has `notifier`,
`state.selectedSave`, `state.activeProfile`, and `state.saves`). Compute:
```dart
final savePaths = <String>[inspection.path!];
if (_allSaves && profile != null) {
  savePaths
    ..clear()
    ..addAll(profileSavePaths); // all SaveSlot.path where persistentProfileId == profile.profileId
}
final difficulty = {
  'preset': _preset,
  if (_preset == 'Custom') 'combat': _combat,
  if (_preset == 'Custom') 'resources': _resources,
  if (_preset == 'Custom') 'progression': _progression,
  'flowHelper': _flow,
  'permadeath': _preset == 'Novice' ? false : _perma,
};
await notifier.writeDifficulty(
  difficulty: difficulty,
  savePaths: savePaths,
  profile: _alsoProfile && profile != null
      ? (path: persistentDataListPath, profileId: profile.profileId)
      : null,
);
```
`persistentDataListPath`: derive from the save directory + `PersistentDataList.sav`
(the save path's parent). `profileSavePaths`: filter `state.saves` by
`persistentProfileId == profile.profileId`, take `.path`.

- [ ] **Step 3: Guard unsaved difficulty edits on profile/save switch**

In `editor_notifier.dart` the profile-switch guard (lib.rs:347) blocks on
`hasUnsavedEdits`. Ensure the card's dirty state participates: simplest is to keep the
card's edits in the notifier's pending-edit set, OR add a lightweight
`difficultyDirty` flag to `EditorState` that the card toggles via a notifier setter and
that the guard checks. Implement the flag approach:
- add `bool difficultyDirty` to `EditorState` (+ copyWith), default false;
- add `EditorNotifier.setDifficultyDirty(bool)`;
- include it in the guard's "unsaved" condition;
- clear it after a successful `writeDifficulty` and on `inspect`.

- [ ] **Step 4: Analyze + widget test**

Create `apps/goresave/test/difficulty_card_test.dart` with a widget test that pumps the
card with a Custom inspection, taps preset = Novice, and asserts the Permadeath switch is
disabled and the sub-pickers are disabled. (Use `ProviderScope` overrides or a fake
notifier; follow an existing widget test in `apps/goresave/test` for the harness.)

Run: `cd apps/goresave && flutter analyze && flutter test test/difficulty_card_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/editor_page.dart apps/goresave/lib/features/editor/domain/editor_notifier.dart apps/goresave/test/difficulty_card_test.dart
git commit -m "feat(app): editable Difficulty card with profile/all-saves propagation"
```

## Task 15: End-to-end verification

- [ ] Run the full suites: `cargo test -p goresave_core` and
  `cd apps/goresave && flutter test`. Expected: all PASS.
- [ ] `/run` the app: change a save's difficulty (Custom→Novice), Save, reopen — confirm
  the card, sidebar badge, and header pill reflect the change and a backup exists.
- [ ] Tick "all saves" + "profile", Save, confirm every slot + PersistentDataList updated
  and each has a backup under one shared suffix.

---

## Self-review notes (addressed)

- **Spec coverage:** read/display (Tasks 1-5), per-save public+private write (Tasks 6-9),
  profile write (Task 10), multi-target/backups/atomic (Task 11), badge replacement
  (Task 4), editability matrix (Task 14), authority check (Task 12). Covered.
- **Type consistency:** `DifficultyRequest` (Rust, Serialize+Deserialize),
  `DifficultySettings` (Rust read struct + Dart model),
  `replace_class_or_str_property_in_range`, `write_bool_property_in_range`,
  `apply_difficulty_to_range`, `write_profile_difficulty`, `write_difficulty_internal`,
  `EditorNotifier.writeDifficulty` used consistently.
- **Known implementer checkpoints (not placeholders — verify against real code):**
  `PendingReplace` commit/rollback ownership (Task 11 note); the exact private-edit
  apply loop location for `PrivateEdit::Difficulty` (Task 9); `binaryHost` wiring in the
  notifier (Task 13). Each names the reference site to copy.
