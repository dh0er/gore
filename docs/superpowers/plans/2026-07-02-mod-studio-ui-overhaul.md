# Mod-Studio UI Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Five UI changes to `apps/mod-studio`: scripts tree browser (textures-style), localized-alphabetical item list, two-column dialogs sidebar, audio bank TabBar with categorized SFX split view, and localized main-tab labels ("Scripte").

**Architecture:** Flutter desktop app (Riverpod, gen-l10n). Task 1 lands all new l10n keys first (later tasks compile against the generated getters). Tasks 2–5 are independent and parallelizable. Task 6 (scripts tab) depends on Task 5 (shared tree widget extraction). Spec: `docs/superpowers/specs/2026-07-02-mod-studio-ui-overhaul-design.md`.

**Tech Stack:** Flutter/Dart, flutter_riverpod, flutter gen-l10n (`l10n.yaml`, generated files committed). Verify with `flutter analyze` + `flutter test` in `apps/mod-studio` (dart MCP hangs on this machine — CLI only).

**Working directory for all commands:** `apps/mod-studio` inside the repo worktree.

---

### Task 1: l10n keys, main-tab labels, overrides-panel headers

**Files:**
- Modify: `lib/l10n/app_en.arb` (template), `lib/l10n/app_de.arb`, and the 10 other arbs (`app_es`, `app_fr`, `app_it`, `app_ja`, `app_pl`, `app_pt`, `app_pt_BR`, `app_ru`, `app_zh`, `app_zh_Hans`)
- Modify: `lib/home_page.dart:255-287` (TabBar), `lib/editor/ui/overrides_panel.dart:93-113` (section headers)
- Regenerate: `lib/l10n/app_localizations*.dart` via `flutter gen-l10n` (committed)

- [ ] **Step 1: Add keys to `app_en.arb`** (after the existing `tab*` keys):

```json
"tabDialogs": "Dialogs",
"tabAudio": "Audio",
"tabTextures": "Textures",
"tabScripts": "Scripts",
"sectionItemValues": "Item values",
"sectionLocalizedText": "Localized text",
"audioCatCreatures": "Creatures",
"audioCatObjects": "Objects",
"audioCatMagic": "Magic",
"audioCatMovement": "Movement",
"audioCatWorld": "World",
"audioCatAction": "Action",
"audioCatCombat": "Combat",
"audioCatPhysics": "Physics",
"audioCatItems": "Items",
"audioCatUi": "UI",
"audioCatFoley": "Foley",
"audioCatUnderwater": "Underwater",
"audioCatVision": "Vision",
"audioCatDialog": "Dialog",
"audioCatOther": "Other"
```

- [ ] **Step 2: Translate into the 11 other arbs.** German (fixed by design):

```json
"tabDialogs": "Dialoge",
"tabAudio": "Audio",
"tabTextures": "Texturen",
"tabScripts": "Scripte",
"sectionItemValues": "Item-Werte",
"sectionLocalizedText": "Lokalisierte Texte",
"audioCatCreatures": "Kreaturen",
"audioCatObjects": "Objekte",
"audioCatMagic": "Magie",
"audioCatMovement": "Bewegung",
"audioCatWorld": "Welt",
"audioCatAction": "Aktionen",
"audioCatCombat": "Kampf",
"audioCatPhysics": "Physik",
"audioCatItems": "Items",
"audioCatUi": "UI",
"audioCatFoley": "Foley",
"audioCatUnderwater": "Unterwasser",
"audioCatVision": "Visionen",
"audioCatDialog": "Dialog",
"audioCatOther": "Sonstige"
```

Other languages: idiomatic translations following each file's existing style (match how `tabItems`/`categoryMeleeWeapons` are handled there). "Scripts"-family words: es "Scripts", fr "Scripts", it "Script", ja "スクリプト", pl "Skrypty", pt/pt_BR "Scripts", ru "Скрипты", zh/zh_Hans "脚本".

- [ ] **Step 3: Use the keys.** In `home_page.dart` replace the four hardcoded `Tab` labels (`'Dialoge'` :264, `'Audio'` :268, `'Textures'` :272, `'AngelScript'` :276) with `l10n.tabDialogs`, `l10n.tabAudio`, `l10n.tabTextures`, `l10n.tabScripts` (drop `const` where needed). In `overrides_panel.dart` replace `_SectionHeader('Item values')`→`l10n.sectionItemValues`, `'Localized text'`→`l10n.sectionLocalizedText`, `'Audio'`→`l10n.tabAudio`, `'Textures'`→`l10n.tabTextures`, `'AngelScript'`→`l10n.tabScripts` (obtain `AppLocalizations.of(context)` in the build method; drop `const`).

- [ ] **Step 4: Regenerate + verify.** Run `flutter gen-l10n`, then `flutter analyze`. Expected: no errors, no untranslated-message warnings for the new keys.

- [ ] **Step 5: Commit** — `feat(mod-studio): localize main tabs and section headers; rename AngelScript tab to Scripts/Scripte`

---

### Task 2: Items list alphabetical by localized name

**Files:**
- Modify: `lib/catalog/ui/catalog_browser.dart:49-73`
- Test: `test/catalog/display_sort_test.dart` (create)

The displayed list (category view AND search results) must sort by localized display name; provider/group id-sorts stay untouched.

- [ ] **Step 1: Write failing test.** Extract-friendly pure helper; test first:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/ui/catalog_browser.dart' show sortByDisplayName;

CatalogItem item(String id) => CatalogItem(id: id, fields: const []);

void main() {
  test('sorts by localized name, case-insensitive, id tiebreak', () {
    final items = [item('ItMw_Zweihander'), item('ItMw_Axt'), item('ItMw_Beil')];
    final names = {'ItMw_Zweihander': 'Anderthalbhänder', 'ItMw_Axt': 'zerbrochene Axt', 'ItMw_Beil': 'Beil'};
    final sorted = sortByDisplayName(items, (i) => names[i.id]!);
    expect(sorted.map((i) => names[i.id]).toList(),
        ['Anderthalbhänder', 'Beil', 'zerbrochene Axt']);
  });
}
```

Adjust the `CatalogItem` constructor call to the real signature in `item_entry.dart` (positional/named as defined there; use the minimal valid construction).

- [ ] **Step 2: Run** `flutter test test/catalog/display_sort_test.dart` — expect FAIL (no `sortByDisplayName`).

- [ ] **Step 3: Implement.** In `catalog_browser.dart` add a top-level function:

```dart
List<CatalogItem> sortByDisplayName(
    List<CatalogItem> items, String Function(CatalogItem) nameOf) {
  return [...items]..sort((a, b) {
    final c = nameOf(a).toLowerCase().compareTo(nameOf(b).toLowerCase());
    return c != 0 ? c : a.id.compareTo(b.id);
  });
}
```

Then change the `shownItems` computation (currently :71-73) to wrap both branches: `final shownItems = sortByDisplayName(searching ? filtered : (groups...items ?? []), nameOf);`

- [ ] **Step 4: Run** the test again — expect PASS. Run `flutter analyze` — clean.

- [ ] **Step 5: Commit** — `feat(mod-studio): sort item list alphabetically by localized name`

---

### Task 3: Dialogs tab two-column sidebar (Items layout)

**Files:**
- Modify: `lib/dialog/domain/dialog_catalog_provider.dart` (add `isBark` to `DialogLineRow`)
- Rewrite: `lib/dialog/ui/dialoge_tab.dart` browser portion (`_DialogBrowser`, drop `_GroupHeader` expand/collapse)
- Test: `test/dialog/dialog_catalog_provider_test.dart` (extend or create)

- [ ] **Step 1: Failing test** for line rows carrying `isBark` (needed to map lines to sidebar groups):

```dart
test('line rows carry bark flag matching their group', () {
  final rows = buildDialogRows({
    'info_aaron_001': {'de_A': 'Hallo'},
    'gvl_aaron_002': {'de_A': 'Weg da'},
  });
  final lines = rows.whereType<DialogLineRow>().toList();
  expect(lines.singleWhere((l) => l.id == 'info_aaron_001').isBark, false);
  expect(lines.singleWhere((l) => l.id == 'gvl_aaron_002').isBark, true);
});
```

Adapt to the provider's actual construction API: if grouping logic lives inline in `dialogRowsProvider`, extract it as a pure `List<DialogRow> buildDialogRows(Map<String, Map<String, String>> catalog)` so it's testable, and have the provider delegate to it.

- [ ] **Step 2: Run** `flutter test test/dialog/` — expect FAIL.

- [ ] **Step 3: Implement.** Add `final bool isBark;` to `DialogLineRow` (constructor + call sites in the grouping code). Extract `buildDialogRows` if not already pure.

- [ ] **Step 4: Run tests** — PASS.

- [ ] **Step 5: Rebuild the browser UI** in `dialoge_tab.dart`, mirroring `catalog_browser.dart:75-142` exactly:
  - Keep: 560px outer pane, search field on top, `_selectedDialogIdProvider`, `_DialogEditor` right pane, edited-dot/preview-subtitle line tiles (:210-239).
  - New state: `_selectedGroupKey` (`String?`, format `'$isBark:$speaker'`, same key as today :81) in `_DialogBrowserState`; remove `_expanded`/`_collapsed`.
  - Not searching: `Row` → `SizedBox(width: 230, child: ListView(...))` of `SidebarTile`s — one per `DialogGroupRow` in provider order (conversations already sort before barks). `icon:` `Icons.forum_outlined` for conversations, `Icons.campaign_outlined` for barks (reuse `iconForItemCategory`-style mapping inline — `SidebarTile` takes an `IconData`, check its constructor in `lib/catalog/ui/sidebar_tile.dart`). `label:` `l10n.categoryWithCount(speaker, lineCount)`. Selected-group fallback like `catalog_browser.dart:66-69` (if selected group vanished, select first). Then `VerticalDivider(width: 1)` + `Expanded(ListView.builder)` of the selected group's line tiles (`rows.whereType<DialogLineRow>().where((l) => '${l.isBark}:${l.speaker}' == selectedKey)`).
  - Searching (query non-empty): sidebar hidden, flat `ListView` of ALL matching line rows across groups (reuse existing `_matches` logic :117-132).

- [ ] **Step 6: Verify.** `flutter analyze` clean; `flutter test` green. Manual smoke optional.

- [ ] **Step 7: Commit** — `feat(mod-studio): dialogs tab speaker sidebar (items-style two-column layout)`

---

### Task 4: Audio bank TabBar + categorized SFX split view

**Files:**
- Create: `lib/audio/domain/sfx_categories.dart`
- Test: `test/audio/sfx_categories_test.dart` (create)
- Modify: `lib/audio/ui/audio_tab.dart` (`_AudioBrowser`)

- [ ] **Step 1: Failing test:**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/sfx_categories.dart';

void main() {
  test('categorizes by second token, case-folded, with merges', () {
    expect(sfxCategoryForSample('SFX_CREA_Golem_Ice_M_Creak_Loop_L1_01'), SfxCategory.creatures);
    expect(sfxCategoryForSample('SFX_OBJ_GolemAltar_OrbLoop_L1_01'), SfxCategory.objects);
    expect(sfxCategoryForSample('SFX_Objects_Chest_Open_01'), SfxCategory.objects);
    expect(sfxCategoryForSample('SFX_Magic_Fear_Cast_L9_01'), SfxCategory.magic);
    expect(sfxCategoryForSample('SFX_MAGIC_Impact_01'), SfxCategory.magic);
    expect(sfxCategoryForSample('SFX_MOVE_Footsteps_Human_Grass_Walk_07'), SfxCategory.movement);
    expect(sfxCategoryForSample('SFX_WORLD_Lava_AMB_02'), SfxCategory.world);
    expect(sfxCategoryForSample('SFX_ACTION_Sweat_Swipe_L1_05'), SfxCategory.action);
    expect(sfxCategoryForSample('SFX_ACTIONS_Foo'), SfxCategory.action);
    expect(sfxCategoryForSample('SFX_COMBAT_Ranged_Bow_Draw_04'), SfxCategory.combat);
    expect(sfxCategoryForSample('SFX_UI_X'), SfxCategory.ui);
    expect(sfxCategoryForSample('taiko_hit'), SfxCategory.other);
    expect(sfxCategoryForSample('SFX'), SfxCategory.other);
  });
}
```

- [ ] **Step 2: Run** `flutter test test/audio/sfx_categories_test.dart` — FAIL (file missing).

- [ ] **Step 3: Implement `sfx_categories.dart`:**

```dart
import '../../l10n/app_localizations.dart';

/// Categories for SFX.bank samples, derived from the sample name's second
/// `_` token (validated against the real bank, 7218 samples).
enum SfxCategory {
  creatures, objects, magic, movement, world, action, combat,
  physics, items, ui, foley, underwater, vision, dialog, other;

  String localizedLabel(AppLocalizations l10n) => switch (this) {
        creatures => l10n.audioCatCreatures,
        objects => l10n.audioCatObjects,
        magic => l10n.audioCatMagic,
        movement => l10n.audioCatMovement,
        world => l10n.audioCatWorld,
        action => l10n.audioCatAction,
        combat => l10n.audioCatCombat,
        physics => l10n.audioCatPhysics,
        items => l10n.audioCatItems,
        ui => l10n.audioCatUi,
        foley => l10n.audioCatFoley,
        underwater => l10n.audioCatUnderwater,
        vision => l10n.audioCatVision,
        dialog => l10n.audioCatDialog,
        other => l10n.audioCatOther,
      };
}

SfxCategory sfxCategoryForSample(String name) {
  final parts = name.split('_');
  if (parts.length < 2) return SfxCategory.other;
  return switch (parts[1].toUpperCase()) {
    'CREA' => SfxCategory.creatures,
    'OBJ' || 'OBJECTS' => SfxCategory.objects,
    'MAGIC' => SfxCategory.magic,
    'MOVE' => SfxCategory.movement,
    'WORLD' => SfxCategory.world,
    'ACTION' || 'ACTIONS' => SfxCategory.action,
    'COMBAT' => SfxCategory.combat,
    'PHYSICS' => SfxCategory.physics,
    'ITEMS' => SfxCategory.items,
    'UI' => SfxCategory.ui,
    'FOLEY' => SfxCategory.foley,
    'UNDERWATER' => SfxCategory.underwater,
    'VISION' => SfxCategory.vision,
    'DIALOG' => SfxCategory.dialog,
    _ => SfxCategory.other,
  };
}
```

(Match the file's switch style to the Dart SDK constraint in `pubspec.yaml` — if pattern-switch unsupported, use a plain `switch`/`case` with returns.)

- [ ] **Step 4: Run test** — PASS.

- [ ] **Step 5: TabBar.** In `_AudioBrowserState` (`audio_tab.dart:40+`): add `SingleTickerProviderStateMixin` + `TabController(length: kModdableBanks.length, vsync: this)`; dispose it. Replace the `ChoiceChip` `Wrap` (:89-103) with a full-width `TabBar` above the split view: `TabBar(controller: _tabs, tabs: [for (final b in kModdableBanks) Tab(text: p.basenameWithoutExtension(b))], onTap/listener → _selectBank(bank))`. Keep `_bankFileName` state + `_selectBank` clearing selection (:54-60). Use `isScrollable: false`, `labelColor`/theme defaults.

- [ ] **Step 6: SFX split view.** Only when `_bankFileName == 'SFX.bank'` and search query empty: inside the 560px left pane `Row` → `SizedBox(width: 230)` category sidebar (`SidebarTile` from `lib/catalog/ui/sidebar_tile.dart`, icon `Icons.music_note` or per-category icons optional — use one neutral icon `Icons.graphic_eq`) listing `SfxCategory.values` that have ≥1 sample, label `l10n.categoryWithCount(cat.localizedLabel(l10n), count)`; state `SfxCategory? _selectedCategory` with first-group fallback; `VerticalDivider(width: 1)`; sample list filtered `samples.where((s) => sfxCategoryForSample(s.name) == selected)`. Non-SFX banks and active search: current flat list over the whole bank (existing behavior, :136-180). Detail pane, preview/replace, staged panel untouched.

- [ ] **Step 7: Verify.** `flutter analyze` clean, `flutter test` green.

- [ ] **Step 8: Commit** — `feat(mod-studio): audio bank TabBar and categorized SFX browser`

---

### Task 5: Extract shared path-tree browser from textures tab

**Files:**
- Create: `lib/app/ui/path_tree.dart`
- Modify: `lib/textures/ui/texture_tab.dart` (delete `_RawNode`/`_DisplayNode`/`_ensureTree`/`_toDisplay`/`_nodeSort`/`_treeBrowser`, use the widget)
- Test: `test/app/path_tree_test.dart` (create)

Pure refactor of `texture_tab.dart:216-363,739-764` into a reusable widget. Public API:

```dart
class PathTreeBrowser extends StatefulWidget {
  const PathTreeBrowser({
    super.key,
    required this.paths,          // slash-separated leaf paths (sorted or not)
    required this.selectedPath,   // highlighted leaf
    required this.onSelect,       // leaf tap
    required this.leafIcon,       // e.g. Icons.image_outlined / Icons.description_outlined
    this.markedPaths = const {},  // trailing check icon (staged)
  });
  final List<String> paths;
  final String? selectedPath;
  final ValueChanged<String> onSelect;
  final IconData leafIcon;
  final Set<String> markedPaths;
  ...
}
```

Internals moved verbatim: raw prefix tree (`_RawNode`), single-child folder-chain compression (`_DisplayNode`, label `"A/B"`), folders-before-leaves case-insensitive sort, `Set<String> _expanded` folder-id state, lazy flattened `ListView.builder`, indent `depth * 14.0`, folder rows with chevron + folder icon + leaf-count badge. Tree rebuild caching: rebuild `_RawNode` root in `didUpdateWidget` only when `!identical(oldWidget.paths, widget.paths)` (preserves the texture tab's identity-cache behavior).

- [ ] **Step 1: Failing widget test:**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/ui/path_tree.dart';

void main() {
  testWidgets('compresses single-child chains and expands folders', (t) async {
    String? tapped;
    await t.pumpWidget(MaterialApp(
        home: Scaffold(
            body: PathTreeBrowser(
      paths: const ['A/B/leaf1.uasset', 'A/B/leaf2.uasset', 'C/leaf3.uasset'],
      selectedPath: null,
      onSelect: (p) => tapped = p,
      leafIcon: Icons.image_outlined,
    ))));
    expect(find.text('A/B'), findsOneWidget); // compressed chain
    expect(find.text('leaf1.uasset'), findsNothing); // collapsed
    await t.tap(find.text('A/B'));
    await t.pumpAndSettle();
    expect(find.text('leaf1.uasset'), findsOneWidget);
    await t.tap(find.text('leaf1.uasset'));
    expect(tapped, 'A/B/leaf1.uasset');
  });
}
```

- [ ] **Step 2: Run** — FAIL (no such file).
- [ ] **Step 3: Implement** `path_tree.dart` by moving the texture-tab code; adapt names/labels only (no behavior change). Leaf label = last path segment (same as today: `_DisplayNode.label` for leaves).
- [ ] **Step 4: Run** new test — PASS.
- [ ] **Step 5: Refactor `texture_tab.dart`** to `PathTreeBrowser(paths: entries, selectedPath: _selected, onSelect: _select, leafIcon: Icons.image_outlined, markedPaths: staged)`. Flat search list stays in the tab. Delete now-dead private tree code.
- [ ] **Step 6: Verify** `flutter analyze` + `flutter test` (existing texture tests must stay green).
- [ ] **Step 7: Commit** — `refactor(mod-studio): extract shared PathTreeBrowser from textures tab`

---

### Task 6: Scripts tab rebuild — tree of all AngelScripts (depends on Task 5)

**Files:**
- Rewrite: `lib/scripts/ui/script_tab.dart` (layout; delete `_ModulePicker` and `_StagedList`)
- Test: existing `test/` scripts tests must stay green; no new domain logic (UI only)

Module → tree path: `m.file.isEmpty ? '${m.name}.as' : m.file` (backslashes already normalized to `/` upstream). Staged key = relPath (`ScriptMod.key`).

- [ ] **Step 1: New layout** in `ScriptTab.build` (replacing the 360px `_StagedList` + `_ModDetail` row):

```
Column(
  Expanded(Row(
    Expanded(flex: 2, Column(
      searchField,
      Expanded(query.isEmpty
        ? PathTreeBrowser(
            paths: [for (final m in modules) relPathOf(m)],
            selectedPath: selectedRelPath,
            onSelect: select,
            leafIcon: Icons.description_outlined,
            markedPaths: stagedRelPaths)
        : flatHitList),   // ListTile: title=name, subtitle=relPath, staged check
      countCaption,        // '<n> modules'
    )),
    VerticalDivider(width: 1),
    Expanded(flex: 3, detailPane),
  )),
  _StagedScriptsPanel(),   // ExpansionTile, audio-style
)
```

  - `modules` from `scriptModulesProvider` (`.when` with loading/error like the textures tab guards); `stagedRelPaths` = `scriptModsProvider` keys.
  - Selection stays in `_selectedModuleProvider` (relPath).
  - Search: case-insensitive substring on `m.name` OR relPath, no 200 cap (lazy builder).

- [ ] **Step 2: Detail pane.** Selected relPath staged (`mods[relPath] != null`) → existing `_ModDetail(mod)` unchanged. Selected but unstaged → info card: module name, relPath, and `FilledButton('Edit')` running today's `_editExisting` body (`script_tab.dart:163-194`) minus the picker: `scriptEmitModule` to temp `.as`, `setMod(ScriptMod(op: ScriptOp.edit, moduleName: name, relPath: relPath, asPath: tmp))`, keep selection. Nothing selected → existing placeholder text.

- [ ] **Step 3: `_StagedScriptsPanel`** (bottom, modeled on `audio_tab.dart:344-395` `_StagedReplacementsPanel`): `ExpansionTile` titled with staged count; children = one dense `ListTile` per staged mod (op icon `add_box_outlined`/`edit_note_outlined`, title `moduleName`, subtitle relPath + compile-freshness text reused from old `_StagedList`, trailing delete → `removeMod`, tap → select). Header trailing: `TextButton.icon('Add new')` running the existing `_addNew` flow (:114-135) — file picker + relPath prompt (staged-only module, appears in this panel, not in the vanilla tree).

- [ ] **Step 4: Delete** `_ModulePicker` (:197-242) and `_StagedList` (:49-112); fix imports.

- [ ] **Step 5: Verify.** `flutter analyze` clean; `flutter test` green (adapt any widget tests referencing deleted classes).

- [ ] **Step 6: Commit** — `feat(mod-studio): scripts tab tree browser over all vanilla AngelScript modules`

---

### Task 7: Final verification

- [ ] **Step 1:** `flutter analyze` — zero issues.
- [ ] **Step 2:** `flutter test` — all green.
- [ ] **Step 3:** `flutter build windows --debug` compiles (smoke).
- [ ] **Step 4:** Fix anything found; commit fixes.

---

## Execution notes

- **Order:** Task 1 first (l10n getters must exist). Then Tasks 2, 3, 4, 5 in parallel (disjoint files). Task 6 after Task 5. Task 7 last.
- **Conflict map:** Task 1 owns `home_page.dart`, arbs, generated l10n, `overrides_panel.dart`. Task 2 owns `catalog_browser.dart`. Task 3 owns `dialog/**`. Task 4 owns `audio/**` (+ new domain file). Task 5 owns `textures/**` + new `app/ui/path_tree.dart`. Task 6 owns `scripts/ui/**`. No shared files across parallel tasks.
- Windows quirks (from memory): use `flutter` CLI directly; dart MCP analyze hangs — never use it.
