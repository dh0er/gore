import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/character_category_catalog.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';

import 'support/ui_settings_test_store.dart';
import 'support/detail_tabs.dart';

void main() {
  test('system font fallbacks prefer the locale-specific CJK face', () {
    expect(
      buildGoresaveTheme(
        locale: const Locale('ja'),
      ).textTheme.bodyMedium?.fontFamilyFallback,
      ['Yu Gothic UI', 'Microsoft YaHei UI'],
    );
    expect(
      buildGoresaveTheme(
        locale: const Locale('zh'),
      ).textTheme.bodyMedium?.fontFamilyFallback,
      ['Microsoft YaHei UI', 'Yu Gothic UI'],
    );
  });

  testWidgets('title progress is centered in the available title-bar space', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final itemCatalog = Completer<ItemIconCatalog>();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(_EmptyCoreService()),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
          itemIconCatalogProvider.overrideWith((ref) => itemCatalog.future),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pump();

    final availableSpace = find.byKey(
      const ValueKey('title-progress-available-space'),
    );
    final progress = find.byKey(const ValueKey('title-progress-item-images'));
    expect(availableSpace, findsOneWidget);
    expect(progress, findsOneWidget);
    expect(
      tester.getCenter(progress).dx,
      closeTo(tester.getCenter(availableSpace).dx, 0.5),
    );

    itemCatalog.complete(const ItemIconCatalog.empty());
    await tester.pumpAndSettle();
  });

  testWidgets('title bar fits the minimum window at 200 percent UI scale', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(960, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final itemCatalog = Completer<ItemIconCatalog>();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(_EmptyCoreService()),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(uiScale: 2),
          ),
          itemIconCatalogProvider.overrideWith((ref) => itemCatalog.future),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(
      tester
          .getSize(find.byKey(const ValueKey('title-progress-available-space')))
          .width,
      greaterThanOrEqualTo(96),
    );
    expect(
      find.byKey(const ValueKey('title-progress-item-images')),
      findsOneWidget,
    );

    itemCatalog.complete(const ItemIconCatalog.empty());
    await tester.pumpAndSettle();
  });

  testWidgets('title measurement follows the ambient text scale', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    tester.platformDispatcher.textScaleFactorTestValue = 2;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(_EmptyCoreService()),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byKey(const ValueKey('title-brand'))).width,
      greaterThan(200),
    );
  });

  testWidgets('renders editor shell with fake save data', (tester) async {
    // Desktop window size so the inventory/diagnostics accordion (which fills
    // the available height) has room to lay out.
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );

    await tester.pumpAndSettle();

    expect(find.text('GORE Save Editor'), findsOneWidget);
    expect(
      _effectiveTextStyle(tester, find.text('GORE Save Editor')).fontFamily,
      notoSerifFontFamily,
    );
    expect(find.text('Die Welt der Verurteilten'), findsAtLeastNWidgets(1));
    expect(find.text('Overview'), findsOneWidget);
    expect(find.byKey(const ValueKey('edit-save-name')), findsOneWidget);
    // Header pills summarise chapter and time played for the save.
    expect(find.text('Chapter 1'), findsOneWidget);
    expect(find.text('1 hr 56 min'), findsAtLeastNWidgets(1));
    expect(find.text('Profile 1'), findsWidgets);
    // The profile header carries the difficulty chip (profile-wide difficulty).
    expect(find.text('Custom'), findsAtLeastNWidgets(1));
    // The profile menu contains only real profiles and the dedicated Other
    // saves view; file opening is offered inside that view, not in this menu.
    await tester.tap(find.byTooltip('Switch profile'));
    await tester.pumpAndSettle();
    expect(find.text('Other saves'), findsOneWidget);
    expect(find.text('Open file'), findsNothing);
    await tester.tapAt(const Offset(900, 500));
    await tester.pumpAndSettle();

    // Every registered save exposes its authoritative profile association on
    // Overview (the fake fixture has profile 0 selected).
    expect(find.text('Save profile'), findsOneWidget);
    expect(find.byType(DropdownButton<int>), findsOneWidget);
    expect(
      find.byKey(const ValueKey('remove-selected-save-profile')),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('save-profile-card')),
        matching: find.byKey(const ValueKey('remove-selected-save-profile')),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('selected-save-header-card')),
        matching: find.byKey(const ValueKey('remove-selected-save-profile')),
      ),
      findsNothing,
    );

    // The header shows the save's screenshot on the Overview tab.
    expect(find.bySemanticsLabel('Screenshot for G1R-001'), findsWidgets);
    // Diagnostics + inspection JSON no longer live on Overview: they moved into
    // the Settings debug section (covered by its own test below).
    expect(find.text('Diagnostics & details'), findsNothing);
    expect(find.text('Inspection JSON'), findsNothing);

    // Global Save button starts disabled (no pending edits yet).
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
          .onPressed,
      isNull,
    );

    // Edit the save name — the global button label gains a pending count.
    await tester.tap(find.byKey(const ValueKey('edit-save-name')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('edit-save-name-field')),
      'Much Longer Save Name',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-save-name')));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    // Tap the global Save button.
    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();

    final publicWrite = core.requests.lastWhere(
      (r) => r.command == 'write_save',
    );
    expect(publicWrite.payload['edits'], [
      {'path': 'public.m_PlayerSaveName', 'value': 'Much Longer Save Name'},
    ]);
    expect(publicWrite.payload['syncPersistentDataList'], isTrue);
    expect(publicWrite.payload['backup'], isTrue);

    // Button disabled again after save.
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
          .onPressed,
      isNull,
    );

    // Attributes is now a sub-tab inside the Charaktere (Characters) tab; open
    // that first, then its Attribute sub-tab. The Player row is pinned + selected
    // by default in the shared master list, so the player attribute view shows.
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Attributes'));
    await tester.pumpAndSettle();

    // Player summary card and name editor fields are deleted.
    expect(find.text('Player summary'), findsNothing);
    expect(find.text('Save version'), findsNothing);
    expect(find.text('Current world'), findsNothing);
    expect(find.text('Profile name'), findsNothing);
    expect(find.widgetWithText(TextField, 'Private player name'), findsNothing);
    expect(
      find.widgetWithText(TextField, 'Private profile name'),
      findsNothing,
    );

    // No individual per-editor save buttons.
    expect(find.byTooltip('Save Health attribute'), findsNothing);
    expect(find.byTooltip('Save hero transform'), findsNothing);

    // Legacy path (no typedParse in fixture): attributes render inside their
    // own Card titled 'Hero attributes'.
    await tester.scrollUntilVisible(
      find.text('Hero attributes'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    expect(find.text('Health'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('legacy-attribute:Health:base')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('legacy-attribute:Health:current')),
      findsOneWidget,
    );
    await tester.enterText(
      find.byKey(const ValueKey('legacy-attribute:Health:base')),
      '77',
    );
    await tester.enterText(
      find.byKey(const ValueKey('legacy-attribute:Health:current')),
      '66',
    );
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    // The player's transform editor now lives in the Charaktere → Position
    // sub-tab (its only home; two copies would both drive the one 'transform'
    // pending key). It renders there regardless of privateTypedVerified, so
    // this legacy fixture still reaches it.
    await tester.tap(detailTab('Position'));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(TextField, 'Location X'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Location Y'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Location Z'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Rotation pitch'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Rotation yaw'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Rotation roll'), findsOneWidget);

    await tester.enterText(find.widgetWithText(TextField, 'Location X'), '100');
    await tester.enterText(find.widgetWithText(TextField, 'Location Y'), '200');
    await tester.enterText(find.widgetWithText(TextField, 'Location Z'), '300');
    await tester.enterText(
      find.widgetWithText(TextField, 'Rotation pitch'),
      '1',
    );
    await tester.enterText(find.widgetWithText(TextField, 'Rotation yaw'), '2');
    await tester.enterText(
      find.widgetWithText(TextField, 'Rotation roll'),
      '3',
    );
    await tester.pump();

    // Two pending edits: attr:Health + transform.
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final combinedWrite = core.requests.lastWhere(
      (r) => r.command == 'write_save',
    );
    expect(combinedWrite.payload['backup'], isTrue);
    final edits = combinedWrite.payload['edits'] as List;
    // Stable key order: 'attr:Health' < 'transform'.
    expect(edits, hasLength(2));
    expect(edits[0]['path'], 'private.player.setAttribute');
    expect(edits[0]['value'], {
      'id': 'Health',
      'baseValue': 77.0,
      'currentValue': 66.0,
    });
    expect(edits[1]['path'], 'private.player.setTransform');
    expect(edits[1]['value'], {
      'location': {'x': 100.0, 'y': 200.0, 'z': 300.0},
      'rotation': {'pitch': 1.0, 'yaw': 2.0, 'roll': 3.0},
    });

    // Still inside the Charaktere tab from the Attributes navigation above, so
    // switching to the Inventar sub-tab needs no Characters prefix. The shared
    // Player selection carries over, so the player inventory shows.
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();

    // Stacks are grouped by the game's own inventory tabs in a sidebar, led by
    // "All" — the tab the game opens on — so both stacks are listed at once and
    // each also lives behind its own tile.
    //
    // The bundled item stats — which would file the nugget under Materials,
    // the way the game does — are loaded from the asset bundle, and that is
    // real I/O the fake async of a widget test never completes. This asserts
    // the class-name fallback the editor uses until they arrive.
    expect(find.text('All (2)'), findsOneWidget);
    expect(find.text('Food (1)'), findsOneWidget);
    expect(find.text('Miscellaneous (1)'), findsOneWidget);
    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsOneWidget);

    // Picking a category narrows the list to it; "All" brings everything back.
    await tester.tap(find.text('Food (1)'));
    await tester.pumpAndSettle();
    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsNothing);
    await tester.tap(find.text('All (2)'));
    await tester.pumpAndSettle();
    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsOneWidget);

    // No old per-item save buttons.
    expect(find.byTooltip('Save ItFo_Cheese count'), findsNothing);
    // No old batch save button text.
    expect(find.widgetWithText(FilledButton, 'Save 2 changes'), findsNothing);

    // Searching matches across all categories, not just the selected one: the
    // misc Ore stack surfaces even though Food is the active category.
    await tester.enterText(
      find.widgetWithText(TextField, 'Filter items'),
      'orenugget',
    );
    await tester.pump();
    expect(find.text('ItMi_Orenugget'), findsOneWidget);
    expect(find.text('ItFo_Cheese'), findsNothing);
    // Clear the filter to resume category browsing.
    await tester.enterText(find.widgetWithText(TextField, 'Filter items'), '');
    await tester.pump();

    // Edit the visible Cheese stack.
    await tester.enterText(
      find.descendant(
        of: find.ancestor(
          of: find.text('ItFo_Cheese'),
          matching: find.byType(ListTile),
        ),
        matching: find.widgetWithText(TextField, 'Count'),
      ),
      '7',
    );
    await tester.pump();

    // Switch to the Miscellaneous category to reach the Ore stack.
    await tester.tap(find.text('Miscellaneous (1)'));
    await tester.pumpAndSettle();
    expect(find.text('ItMi_Orenugget'), findsOneWidget);
    expect(find.text('42'), findsAtLeastNWidgets(1));

    final oreCountField = find.descendant(
      of: find.ancestor(
        of: find.text('ItMi_Orenugget'),
        matching: find.byType(ListTile),
      ),
      matching: find.widgetWithText(TextField, 'Count'),
    );
    await tester.enterText(oreCountField, '44');
    await tester.pump();
    final oreEditable = tester.widget<EditableText>(
      find.descendant(of: oreCountField, matching: find.byType(EditableText)),
    );
    expect(
      oreEditable.controller.selection,
      const TextSelection.collapsed(offset: 2),
    );

    // Both inventory edits (one per category) survive the category switch and
    // are reflected in the global button count.
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final batchWrite = core.requests.lastWhere(
      (r) => r.command == 'write_save',
    );
    expect(batchWrite.payload['backup'], isTrue);
    final batchEdits = batchWrite.payload['edits'] as List;
    expect(batchEdits, hasLength(2));
    final batchPaths = batchEdits.map((e) => e['value']['id']).toList();
    expect(batchPaths, containsAll(['ItMi_Orenugget', 'ItFo_Cheese']));

    await tester.enterText(
      find.widgetWithText(TextField, 'Filter items'),
      'cheese',
    );
    await tester.pumpAndSettle();

    expect(find.text('ItFo_Cheese'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsNothing);

    await tester.tap(find.widgetWithText(Tab, 'World'));
    await tester.pumpAndSettle();

    // Overview/summary card is gone; sidebar entries are visible instead.
    expect(find.text('Progression summary'), findsNothing);
    expect(find.text('Quests total'), findsNothing);
    expect(find.text('Knowledge NPCs'), findsNothing);

    // Sidebar: Quests is default selection; quest list loads immediately.
    // 'Quests' appears in the sidebar tile (the detail card has no title row).
    expect(find.text('Quests'), findsAtLeastNWidgets(1));
    // Knowledge and Events are no longer sidebar sections here: they moved to
    // detail-only panels (KnowledgeDetail / EventsDetail) keyed by a shared
    // character selection and are mounted from the Characters tab instead.
    // Factions remains alongside Quests in this sidebar.
    expect(find.text('Factions'), findsOneWidget);
    // Quests detail loads and shows the fake quest name.
    expect(find.text('Sleeper'), findsOneWidget);

    // Search quests — filter is inside the Quests detail's TextField.
    await tester.enterText(
      find.widgetWithText(TextField, 'Search quests'),
      'sleeper',
    );
    await tester.pumpAndSettle();

    expect(find.text('Sleeper'), findsOneWidget);

    // Two TabBars now exist in the tree: the scrollable top-level bar and the
    // Charaktere tab's inner sub-tab bar (kept alive off-screen). Drag the
    // top-level one (built first) to reveal the 'Backups' tab.
    await tester.drag(find.byType(TabBar).first, const Offset(-500, 0));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(Tab, 'Backups'), warnIfMissed: false);
    await tester.pumpAndSettle();

    // Twice: as the unnamed backup's title, and as its "File" fact below.
    expect(find.text('G1R-001.sav.bak.200'), findsNWidgets(2));
    expect(find.text('Before edit'), findsOneWidget);

    await tester.tap(
      find.byTooltip('Restore G1R-001.sav.bak.200'),
      warnIfMissed: false,
    );
    await tester.pumpAndSettle();

    final restore = core.requests.lastWhere(
      (r) => r.command == 'restore_backup',
    );
    expect(restore.payload, {
      'path': r'C:\tmp\saves\G1R-001.sav',
      'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.200',
    });

    await tester.scrollUntilVisible(
      find.text('Profile backups'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();

    expect(find.text('Profile backups'), findsOneWidget);
    // Title plus "File" fact, same as the slot backup above.
    expect(find.text('PersistentDataList.sav.bak.250'), findsNWidgets(2));
    expect(find.text('Before companion edit'), findsOneWidget);
    // Companion (PersistentDataList.sav) backups are restorable: restoring one
    // targets PersistentDataList.sav in the save folder, not the selected slot.
    expect(
      find.byTooltip('Restore PersistentDataList.sav.bak.250'),
      findsOneWidget,
    );
    await tester.tap(
      find.byTooltip('Restore PersistentDataList.sav.bak.250'),
      warnIfMissed: false,
    );
    await tester.pumpAndSettle();
    final companionRestore = core.requests.lastWhere(
      (r) => r.command == 'restore_backup',
    );
    expect(companionRestore.payload, {
      'path': r'C:\tmp\saves\PersistentDataList.sav',
      'backupPath': r'C:\tmp\saves\PersistentDataList.sav.bak.250',
    });
  });

  testWidgets('All data shows source-aware nodes and edits a native vector', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(Tab, 'All data'));
    await tester.pumpAndSettle();

    final search = core.requests.lastWhere(
      (request) => request.command == 'search_typed_properties',
    );
    expect(search.payload['includeNodes'], isTrue);
    expect(search.payload['source'], 'private');
    expect(find.text('PRIVATE typed'), findsOneWidget);
    expect(find.text('Transform › Location'), findsOneWidget);
    expect(find.text('nativeStruct'), findsOneWidget);
    expect(find.text('12 children'), findsOneWidget);
    expect(find.widgetWithText(TextFormField, 'X'), findsOneWidget);
    final titleBottom = tester
        .getBottomLeft(find.text('Transform › Location'))
        .dy;
    final badgeTop = tester.getTopLeft(find.text('nativeStruct')).dy;
    expect(
      badgeTop - titleBottom,
      lessThan(20),
      reason: 'single-line titles must not reserve three lines above badges',
    );
    final containerRow = find.byKey(const ValueKey('private:2'));
    final containerTitle = find.descendant(
      of: containerRow,
      matching: find.text('Events'),
    );
    final containerBadge = find.descendant(
      of: containerRow,
      matching: find.text('array'),
    );
    expect(
      tester.getTopLeft(containerBadge).dy -
          tester.getBottomLeft(containerTitle).dy,
      lessThan(20),
      reason: 'read-only container cards use the same compact title layout',
    );
    final containerCard = find.descendant(
      of: containerRow,
      matching: find.byType(AnimatedContainer),
    );
    expect(
      tester.getSize(containerCard).height,
      lessThan(110),
      reason: 'a one-line read-only value must not reserve four text lines',
    );
    final containerValue = find.descendant(
      of: containerRow,
      matching: find.text('12 elements'),
    );
    expect(
      (tester.getTopLeft(containerTitle).dy -
              tester.getTopLeft(containerValue).dy)
          .abs(),
      lessThan(3),
      reason: 'card information and its value must share the top alignment',
    );

    final queryField = find.byWidgetPredicate(
      (widget) =>
          widget is TextField &&
          widget.textInputAction == TextInputAction.search,
    );
    final initialSearchCount = core.requests
        .where((request) => request.command == 'search_typed_properties')
        .length;
    await tester.enterText(queryField, 'Location');
    await tester.pump(const Duration(milliseconds: 500));
    expect(
      core.requests
          .where((request) => request.command == 'search_typed_properties')
          .length,
      initialSearchCount,
      reason: 'typing must not enqueue an exhaustive scan per keystroke',
    );
    await tester.testTextInput.receiveAction(TextInputAction.search);
    await tester.pumpAndSettle();
    final submittedSearch = core.requests.lastWhere(
      (request) => request.command == 'search_typed_properties',
    );
    expect(submittedSearch.payload['query'], 'Location');

    await tester.enterText(find.widgetWithText(TextFormField, 'X'), '9');
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
  });

  testWidgets('Settings debug section exposes codec status and inspection '
      'JSON', (tester) async {
    // Tall surface so all Settings cards (including the debug section) lay out
    // without scrolling.
    await tester.binding.setSurfaceSize(const Size(1400, 1600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Reveal and open the Settings tab (last entry in the scrollable tab bar).
    await tester.drag(find.byType(TabBar).first, const Offset(-800, 0));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(Tab, 'Settings'), warnIfMissed: false);
    await tester.pumpAndSettle();

    // Collapsed by default: neither the codec status nor the raw JSON shows yet.
    expect(find.text('Advanced (debug)'), findsOneWidget);
    expect(find.text('Font'), findsOneWidget);
    expect(find.text('Codec ready'), findsNothing);
    expect(find.text('Inspection JSON'), findsNothing);

    // Expand the debug section.
    await tester.tap(find.text('Advanced (debug)'));
    await tester.pumpAndSettle();

    // Codec status and the ID preference appear, but the raw JSON remains
    // collapsed until explicitly opened.
    expect(find.text('Codec ready'), findsOneWidget);
    expect(find.text('Inspection JSON'), findsOneWidget);
    expect(find.text('Show additional technical IDs'), findsOneWidget);
    expect(find.textContaining('"format"'), findsNothing);
    final objectIdsToggle = find.ancestor(
      of: find.text('Show additional technical IDs'),
      matching: find.byType(SwitchListTile),
    );
    expect(tester.widget<SwitchListTile>(objectIdsToggle).value, isFalse);

    await tester.tap(find.text('Show additional technical IDs'));
    await tester.pumpAndSettle();

    expect(tester.widget<SwitchListTile>(objectIdsToggle).value, isTrue);

    final fontDropdown = find.byKey(const ValueKey('ui-font-family-dropdown'));
    expect(
      tester.getTopLeft(fontDropdown).dy,
      greaterThan(tester.getTopLeft(find.text('Language')).dy),
    );
    expect(
      tester.widget<DropdownButton<UiFontFamily>>(fontDropdown).value,
      UiFontFamily.notoSerif,
    );
    final fontItems = tester
        .widget<DropdownButton<UiFontFamily>>(fontDropdown)
        .items!;
    Text fontItem(UiFontFamily font) =>
        fontItems.singleWhere((item) => item.value == font).child as Text;
    expect(fontItem(UiFontFamily.system).style?.fontFamily, 'Segoe UI');
    expect(fontItem(UiFontFamily.podkova).style?.fontFamily, podkovaFontFamily);
    expect(
      fontItem(UiFontFamily.notoSerif).style?.fontFamily,
      notoSerifFontFamily,
    );
    final settingsContext = tester.element(find.text('Appearance'));
    expect(
      Theme.of(settingsContext).textTheme.bodyMedium?.fontFamily,
      notoSerifFontFamily,
    );
    expect(
      Theme.of(settingsContext).textTheme.titleMedium?.fontFamily,
      notoSerifFontFamily,
    );
    expect(
      _effectiveTextStyle(tester, find.text('GORE Save Editor')).fontFamily,
      notoSerifFontFamily,
    );

    tester.widget<DropdownButton<UiFontFamily>>(fontDropdown).onChanged!(
      UiFontFamily.podkova,
    );
    await tester.pumpAndSettle();

    expect(
      tester.widget<DropdownButton<UiFontFamily>>(fontDropdown).value,
      UiFontFamily.podkova,
    );
    expect(
      Theme.of(
        tester.element(find.text('Appearance')),
      ).textTheme.bodyMedium?.fontFamily,
      podkovaFontFamily,
    );
    expect(
      _effectiveTextStyle(tester, find.text('GORE Save Editor')).fontFamily,
      podkovaFontFamily,
    );

    tester.widget<DropdownButton<UiFontFamily>>(fontDropdown).onChanged!(
      UiFontFamily.notoSerif,
    );
    await tester.pumpAndSettle();
    expect(
      Theme.of(
        tester.element(find.text('Appearance')),
      ).textTheme.bodyMedium?.fontFamily,
      notoSerifFontFamily,
    );
    expect(
      _effectiveTextStyle(tester, find.text('GORE Save Editor')).fontFamily,
      notoSerifFontFamily,
    );

    await tester.tap(find.text('Inspection JSON'));
    await tester.pumpAndSettle();

    expect(find.textContaining('"format"'), findsOneWidget);
  });

  for (final localeCode in ['ja', 'zh-Hans']) {
    testWidgets('Podkova falls back to Noto Serif for $localeCode', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final core = _FakeCoreService();
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            coreServiceProvider.overrideWithValue(core),
            editorSettingsStoreProvider.overrideWithValue(
              const NoopEditorSettingsStore(),
            ),
            uiSettingsStoreProvider.overrideWithValue(
              TestUiSettingsStore(
                appLocale: localeCode,
                uiFontFamily: UiFontFamily.podkova,
              ),
            ),
          ],
          child: const GoresaveApp(),
        ),
      );
      await tester.pumpAndSettle();

      final scaffoldContext = tester.element(find.byType(Scaffold).first);
      final l10n = AppLocalizations.of(scaffoldContext);
      expect(
        Theme.of(scaffoldContext).textTheme.bodyMedium?.fontFamily,
        localeCode == 'ja' ? notoSerifJpFontFamily : notoSerifScFontFamily,
      );

      await tester.drag(find.byType(TabBar).first, const Offset(-800, 0));
      await tester.pumpAndSettle();
      await tester.tap(
        find.widgetWithText(Tab, l10n.tabSettings),
        warnIfMissed: false,
      );
      await tester.pumpAndSettle();

      final fontDropdown = tester.widget<DropdownButton<UiFontFamily>>(
        find.byKey(const ValueKey('ui-font-family-dropdown')),
      );
      expect(fontDropdown.value, UiFontFamily.notoSerif);
      expect(
        fontDropdown.items!.map((item) => item.value),
        isNot(contains(UiFontFamily.podkova)),
      );
    });
  }

  testWidgets('switching tabs preserves unsaved edit and Save count', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Enter a draft through the title's edit dialog on Overview.
    await tester.tap(find.byKey(const ValueKey('edit-save-name')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('edit-save-name-field')),
      'Draft Name',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-save-name')));
    await tester.pumpAndSettle();
    // Save button now shows 1 pending edit.
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    // Switch to another top-level tab (Charaktere).
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();

    // Save count must still be 1 (tab switch must not drop pending edits).
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    // Switch back to Overview tab.
    await tester.tap(find.widgetWithText(Tab, 'Overview'));
    await tester.pumpAndSettle();

    // The draft text must still be visible in the title.
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('selected-save-name')))
          .data,
      'Draft Name',
    );
    // Save button still shows 1.
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
  });

  testWidgets('responsive header keeps pending name and game-time drafts', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService(gameTimeTotalSeconds: 1413433);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('edit-save-name')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('edit-save-name-field')),
      'Responsive draft',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-save-name')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('game-time-badge')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('game-time-day-field')),
      '17',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-game-time')));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

    await tester.binding.setSurfaceSize(const Size(850, 1000));
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('selected-save-name')))
          .data,
      'Responsive draft',
    );
    expect(find.text('Day 17 · 08:37:13'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);
  });

  testWidgets('wide header grows when scaled content exceeds the screenshot', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(2000, 1600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _FakeCoreService(gameTimeTotalSeconds: 1413433),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(uiScale: 2),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      tester
          .getSize(find.byKey(const ValueKey('selected-save-header-details')))
          .height,
      greaterThan(
        tester.getSize(find.byKey(const ValueKey('header-screenshot'))).height,
      ),
    );
  });

  testWidgets('confirming an unnamed save fallback leaves it unchanged', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _FakeCoreService(playerSaveName: null),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('edit-save-name')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('confirm-save-name')));
    await tester.pumpAndSettle();

    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsNothing);
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
          .onPressed,
      isNull,
    );
  });

  testWidgets(
    'empty save name uses the slot fallback without creating an edit',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            coreServiceProvider.overrideWithValue(
              _FakeCoreService(playerSaveName: ''),
            ),
            editorSettingsStoreProvider.overrideWithValue(
              const NoopEditorSettingsStore(),
            ),
            uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
          ],
          child: const GoresaveApp(),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        tester
            .widget<Text>(find.byKey(const ValueKey('selected-save-name')))
            .data,
        'G1R-001',
      );
      await tester.tap(find.byKey(const ValueKey('edit-save-name')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('confirm-save-name')));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsNothing);
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets('Reset button discards pending and restores field text', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Confirm Reset is disabled with no pending edits.
    final resetFinder = find.widgetWithText(OutlinedButton, 'Reset');
    expect(resetFinder, findsOneWidget);
    expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNull);

    // Enter a draft through the title's edit dialog.
    final originalName = 'Die Welt der Verurteilten';
    await tester.tap(find.byKey(const ValueKey('edit-save-name')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('edit-save-name-field')),
      'Edited Name',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-save-name')));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
    // Reset should now be enabled.
    expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNotNull);

    // Tap Reset.
    await tester.tap(resetFinder);
    await tester.pumpAndSettle();

    // Pending count must be 0 and Reset disabled again.
    expect(find.widgetWithText(FilledButton, 'Save'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
          .onPressed,
      isNull,
    );
    expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNull);

    // The title must display the canonical (original) name again.
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('selected-save-name')))
          .data,
      originalName,
    );
  });

  testWidgets(
    'invalid-only draft enables Reset, blocks Save, and guards rescan',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            coreServiceProvider.overrideWithValue(_FakeCoreService()),
            editorSettingsStoreProvider.overrideWithValue(
              const NoopEditorSettingsStore(),
            ),
          ],
          child: const GoresaveApp(),
        ),
      );
      await tester.pumpAndSettle();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(GoresaveApp)),
      );
      container.read(editorProvider.notifier).setStoryStateEditInvalid(true);
      await tester.pump();

      final resetFinder = find.widgetWithText(OutlinedButton, 'Reset');
      final saveFinder = find.widgetWithText(FilledButton, 'Save (1)');
      expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNotNull);
      expect(tester.widget<FilledButton>(saveFinder).onPressed, isNull);

      await tester.tap(find.byTooltip('Rescan save folder'));
      await tester.pumpAndSettle();
      expect(find.text('Discard unsaved changes?'), findsOneWidget);
      expect(
        find.text(
          'Rescanning reloads every save and discards your 1 unsaved change.',
        ),
        findsOneWidget,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      await tester.tap(resetFinder);
      await tester.pumpAndSettle();
      expect(find.widgetWithText(FilledButton, 'Save'), findsOneWidget);
      expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNull);
    },
  );

  testWidgets('shows loading spinner in main editor view', (tester) async {
    final core = _SlowInspectCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );

    await tester.pump();

    expect(find.bySemanticsLabel('Loading editor data'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    core.completePending();
    await tester.pumpAndSettle();

    expect(find.bySemanticsLabel('Loading editor data'), findsNothing);
  });

  testWidgets('non-removable inventory item shows a disabled trash button with an '
      'explanatory tooltip', (tester) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _RemovableInventoryCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
          // The editor offers no removal until the stats have said what each
          // row IS. Answer for them here rather than wait on the bundle.
          itemStatsCatalogProvider.overrideWith(
            (ref) async => const ItemStatsCatalog(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Inventory is now a sub-tab inside the Charaktere (Characters) tab.
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();

    // Food is the first category, so the non-removable Cheese stack is visible.
    final cheeseRow = find.ancestor(
      of: find.text('ItFo_Cheese'),
      matching: find.byType(ListTile),
    );
    expect(cheeseRow, findsOneWidget);
    // The trash button renders even though the item is non-removable, but it is
    // disabled and explains why via its tooltip.
    final cheeseDeleteTooltip = find.descendant(
      of: cheeseRow,
      matching: find.byTooltip(
        "Can't delete: this item is likely equipped or "
        'assigned to a hotkey slot',
      ),
    );
    expect(cheeseDeleteTooltip, findsOneWidget);
    final cheeseDelete = tester.widget<IconButton>(
      find.descendant(
        of: cheeseDeleteTooltip,
        matching: find.byType(IconButton),
      ),
    );
    expect(cheeseDelete.onPressed, isNull);

    // Switch to the removable Orenugget stack: its trash button is enabled with
    // the standard remove tooltip.
    await tester.tap(find.text('Miscellaneous (1)'));
    await tester.pumpAndSettle();

    final oreRow = find.ancestor(
      of: find.text('ItMi_Orenugget'),
      matching: find.byType(ListTile),
    );
    final oreDeleteTooltip = find.descendant(
      of: oreRow,
      matching: find.byTooltip('Remove item from inventory'),
    );
    expect(oreDeleteTooltip, findsOneWidget);
    final oreDelete = tester.widget<IconButton>(
      find.descendant(of: oreDeleteTooltip, matching: find.byType(IconButton)),
    );
    expect(oreDelete.onPressed, isNotNull);
  });

  testWidgets('game time uses a compact badge and edits in a roomy dialog', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FakeCoreService(gameTimeTotalSeconds: 1413433);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('game-time-badge')), findsOneWidget);
    expect(find.text('Day 16 · 08:37:13'), findsOneWidget);
    final badgeBottom = tester
        .getBottomLeft(find.byKey(const ValueKey('game-time-badge')))
        .dy;
    final summaryBottom = tester
        .getBottomLeft(find.byKey(const ValueKey('chapter-badge')))
        .dy;
    expect(
      tester.getBottomLeft(find.byKey(const ValueKey('time-played-badge'))).dy,
      closeTo(summaryBottom, 0.5),
    );
    expect(
      tester.getTopLeft(find.byKey(const ValueKey('game-time-badge'))).dy,
      greaterThan(summaryBottom),
    );
    expect(
      badgeBottom,
      closeTo(
        tester
            .getBottomLeft(find.byKey(const ValueKey('delete-selected-save')))
            .dy,
        0.5,
      ),
    );
    expect(
      tester.getBottomLeft(find.byKey(const ValueKey('selected-save-path'))).dy,
      lessThan(
        tester.getTopLeft(find.byKey(const ValueKey('header-badges'))).dy - 20,
      ),
    );
    expect(find.byKey(const ValueKey('game-time-day-field')), findsNothing);

    await tester.tap(find.byKey(const ValueKey('game-time-badge')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('game-time-dialog')), findsOneWidget);

    for (final key in [
      'game-time-day-field',
      'game-time-hour-field',
      'game-time-minute-field',
      'game-time-second-field',
    ]) {
      expect(
        tester.getSize(find.byKey(ValueKey(key))).width,
        greaterThanOrEqualTo(110),
      );
    }

    await tester.enterText(
      find.byKey(const ValueKey('game-time-day-field')),
      '17',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-game-time')));
    await tester.pumpAndSettle();

    expect(find.text('Day 17 · 08:37:13'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();
    final write = core.requests.lastWhere(
      (request) => request.command == 'write_save',
    );
    expect(write.payload['edits'], [
      {
        'path': 'private.typed.setValue',
        'value': {
          'path': [
            'm_GenericData',
            '{GameTime}',
            'CurrentTime',
            'TotalSeconds',
          ],
          'value': 1499833.0,
        },
      },
    ]);
  });

  testWidgets('overview statistics use authoritative save data', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1500, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.runAsync(loadCharacterCategoryCatalog);
    final core = _StatisticsCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('overview-statistics-section')),
      findsOneWidget,
    );
    expect(
      tester
          .getSize(find.byKey(const ValueKey('overview-statistics-section')))
          .width,
      lessThanOrEqualTo(1280),
    );
    for (final card in ['time', 'character', 'quests', 'progress']) {
      expect(find.byKey(ValueKey('statistics-card-$card')), findsOneWidget);
    }
    expect(
      find.byKey(const ValueKey('statistics-section-inventory')),
      findsOneWidget,
    );
    final timeCard = find.byKey(const ValueKey('statistics-card-time'));
    final characterCard = find.byKey(
      const ValueKey('statistics-card-character'),
    );
    final questCard = find.byKey(const ValueKey('statistics-card-quests'));
    final encountersCard = find.byKey(
      const ValueKey('statistics-card-progress'),
    );
    String metricValue(String label) {
      Element? metric;
      tester.element(find.text(label)).visitAncestorElements((candidate) {
        if (candidate.widget is Column) {
          metric = candidate;
          return false;
        }
        return true;
      });
      final texts = <String>[];
      void collectText(Element element) {
        final widget = element.widget;
        if (widget is Text && widget.data != null) texts.add(widget.data!);
        element.visitChildElements(collectText);
      }

      metric!.visitChildElements(collectText);
      return texts.last;
    }

    final inventoryCard = find.byKey(
      const ValueKey('statistics-section-inventory'),
    );
    expect(tester.getTopLeft(timeCard).dy, tester.getTopLeft(characterCard).dy);
    expect(tester.getTopLeft(timeCard).dy, tester.getTopLeft(questCard).dy);
    expect(tester.getSize(timeCard).height, lessThan(270));
    for (final card in [characterCard, questCard]) {
      expect(tester.getSize(card).height, tester.getSize(timeCard).height);
      expect(tester.getSize(card).width, lessThan(450));
    }
    expect(
      tester.getSize(inventoryCard).height,
      tester.getSize(encountersCard).height,
    );
    expect(tester.getSize(encountersCard).height, lessThan(270));
    expect(
      tester.getSize(inventoryCard).width,
      tester.getSize(encountersCard).width,
    );
    expect(
      tester.getSize(encountersCard).width,
      greaterThan(tester.getSize(timeCard).width),
    );
    expect(
      tester.getTopLeft(encountersCard).dy - tester.getBottomLeft(timeCard).dy,
      closeTo(14, 0.1),
    );
    expect(
      tester.getTopLeft(inventoryCard).dy,
      tester.getTopLeft(encountersCard).dy,
    );
    final chapterLabel = find.descendant(
      of: timeCard,
      matching: find.text('Chapter'),
    );
    final chapterValue = find.descendant(
      of: timeCard,
      matching: find.text('1'),
    );
    expect(
      tester.getTopLeft(chapterValue).dy,
      greaterThan(tester.getTopLeft(chapterLabel).dy),
    );
    expect(
      tester.getTopLeft(chapterValue).dx,
      closeTo(tester.getTopLeft(chapterLabel).dx, 0.1),
    );
    expect(
      find.text(
        'A compact summary of character, quest, world, and save progress.',
      ),
      findsNothing,
    );
    expect(find.byKey(const ValueKey('statistics-summary')), findsNothing);
    expect(find.byKey(const ValueKey('statistics-more')), findsNothing);
    expect(find.byType(ExpansionTile), findsNothing);
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-card-time')),
        matching: find.text('Day 16, 08:37:13'),
      ),
      findsOneWidget,
    );
    final statisticsTexts = tester
        .widgetList<Text>(
          find.descendant(
            of: find.byKey(const ValueKey('overview-statistics-section')),
            matching: find.byType(Text),
          ),
        )
        .map((widget) => widget.data)
        .whereType<String>()
        .toList();
    expect(
      statisticsTexts,
      contains('New Camp\nMercenary'),
      reason: statisticsTexts.join(' | '),
    );
    final guildValue = tester.widget<Text>(find.text('New Camp\nMercenary'));
    expect(guildValue.maxLines, 2);
    expect(guildValue.overflow, isNull);
    final worldTimeValue = tester.widget<Text>(find.text('Day 16, 08:37:13'));
    expect(worldTimeValue.maxLines, 2);
    expect(worldTimeValue.overflow, isNull);
    expect(find.text('Killed monsters'), findsOneWidget);
    expect(find.text('Defeated NPCs'), findsOneWidget);
    expect(find.text('Killed NPCs'), findsOneWidget);
    expect(find.text('Known NPCs'), findsOneWidget);
    expect(find.text('Known teachers'), findsOneWidget);
    expect(metricValue('Killed monsters'), '1');
    expect(metricValue('Defeated NPCs'), '1');
    expect(metricValue('Killed NPCs'), '1');
    expect(metricValue('Known NPCs'), '2');
    expect(metricValue('Known traders'), '2');
    expect(metricValue('Known teachers'), '2');
    expect(metricValue('Open crimes'), '1');
    expect(metricValue('Available'), '1');
    expect(metricValue('Running'), '1');
    expect(
      core.requests.where(
        (request) =>
            request.command == 'query_progression' &&
            request.payload['section'] == 'events' &&
            request.payload['character'] == 'Hero_Global' &&
            request.payload['limit'] == EditorPageSize.statistics,
      ),
      hasLength(1),
      reason:
          'Overview must own its aggregate event scan without prefetching it twice.',
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-card-character')),
        matching: find.text('18'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-card-progress')),
        matching: find.text('1'),
      ),
      findsAtLeastNWidgets(1),
    );
    expect(find.text('Learned skills'), findsOneWidget);
    expect(find.text('Open crimes'), findsOneWidget);
    for (final removed in [
      'Item stacks',
      'Dead characters',
      'Hostile factions',
      'Equipped',
      'Position',
      'Knowledge entries',
      'Location',
    ]) {
      expect(find.text(removed), findsNothing);
    }
  });

  testWidgets('overview clears a guild after a later expulsion', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1500, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.runAsync(loadCharacterCategoryCatalog);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _ExpelledStatisticsCoreService(),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pumpAndSettle();

    final characterCard = find.byKey(
      const ValueKey('statistics-card-character'),
    );
    expect(
      find.descendant(of: characterCard, matching: find.text('Guild')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: characterCard, matching: find.text('Not available')),
      findsOneWidget,
    );
    expect(find.text('New Camp\nMercenary'), findsNothing);
  });

  testWidgets('unavailable private statistics are not rendered as zero', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1500, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _UnavailableStatisticsCoreService(),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-card-quests')),
        matching: find.text('Not available'),
      ),
      findsNWidgets(4),
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-card-progress')),
        matching: find.text('Not available'),
      ),
      findsAtLeastNWidgets(1),
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-section-inventory')),
        matching: find.text('Not available'),
      ),
      findsNWidgets(3),
    );
  });

  testWidgets('truncated inventory statistics remain unavailable', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1500, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _TruncatedStatisticsCoreService(),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-section-inventory')),
        matching: find.text('Not available'),
      ),
      findsNWidgets(2),
    );
  });

  testWidgets('unknown inventory counts keep aggregate totals unavailable', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1500, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _UnknownCountStatisticsCoreService(),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-section-inventory')),
        matching: find.text('Not available'),
      ),
      findsNWidgets(2),
    );
  });

  testWidgets('global inventory observations remain unavailable', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1500, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(
            _GlobalInventoryStatisticsCoreService(),
          ),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.descendant(
        of: find.byKey(const ValueKey('statistics-section-inventory')),
        matching: find.text('Not available'),
      ),
      findsNWidgets(2),
    );
  });

  testWidgets(
    'save list shows file name and both delete actions require confirmation',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final core = _DeletingSaveCoreService();
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            coreServiceProvider.overrideWithValue(core),
            editorSettingsStoreProvider.overrideWithValue(
              const NoopEditorSettingsStore(),
            ),
            uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
          ],
          child: const GoresaveApp(),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('save-file-name-G1R-001')),
        findsOneWidget,
      );
      expect(find.text('G1R-001.sav'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('save-actions-menu-0-G1R-001')),
        findsOneWidget,
      );
      expect(
        tester.getSize(
          find.byKey(const ValueKey('save-actions-menu-0-G1R-001')),
        ),
        const Size(32, 32),
      );
      expect(
        find.byKey(const ValueKey('remove-save-profile-0-G1R-001')),
        findsNothing,
      );
      expect(find.byKey(const ValueKey('delete-save-0-G1R-001')), findsNothing);
      expect(
        find.byKey(const ValueKey('delete-selected-save')),
        findsOneWidget,
      );
      expect(
        tester.getTopLeft(find.byKey(const ValueKey('edit-save-name'))).dx -
            tester
                .getTopRight(find.byKey(const ValueKey('selected-save-name')))
                .dx,
        closeTo(4, 0.5),
      );
      expect(
        tester
            .getBottomRight(find.byKey(const ValueKey('delete-selected-save')))
            .dy,
        closeTo(
          tester
              .getBottomRight(find.byKey(const ValueKey('header-screenshot')))
              .dy,
          0.5,
        ),
      );
      expect(
        tester
                .getTopLeft(find.byKey(const ValueKey('save-subtitle-G1R-001')))
                .dy -
            tester
                .getBottomLeft(find.byKey(const ValueKey('save-title-G1R-001')))
                .dy,
        closeTo(3, 0.5),
      );

      final profileCopyTop = tester.getTopLeft(
        find.byKey(const ValueKey('save-profile-copy')),
      );
      final profileSelectorTop = tester.getTopLeft(
        find.byKey(const ValueKey('save-profile-selector')),
      );
      expect(profileCopyTop.dy, closeTo(profileSelectorTop.dy, 0.5));
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('selected-save-header-card')),
          matching: find.byKey(const ValueKey('delete-selected-save')),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('save-profile-card')),
          matching: find.byKey(const ValueKey('remove-selected-save-profile')),
        ),
        findsOneWidget,
      );
      expect(
        tester
            .widget<Text>(
              find.descendant(
                of: find.byKey(const ValueKey('remove-selected-save-profile')),
                matching: find.text('Remove from profile'),
              ),
            )
            .overflow,
        isNull,
      );

      await tester.tap(
        find.byKey(const ValueKey('save-actions-menu-0-G1R-001')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('remove-save-profile-0-G1R-001')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('delete-save-0-G1R-001')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('delete-save-0-G1R-001')));
      await tester.pumpAndSettle();
      expect(find.text('Delete savegame?'), findsOneWidget);
      expect(
        find.textContaining(
          'G1R-001.sav)? It will be removed from Profile 1',
          findRichText: true,
        ),
        findsOneWidget,
      );
      expect(
        core.requests.where((request) => request.command == 'delete_save'),
        isEmpty,
      );
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('delete-selected-save')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Delete save'));
      await tester.pumpAndSettle();

      final request = core.requests.singleWhere(
        (request) => request.command == 'delete_save',
      );
      expect(request.payload['path'], r'C:\tmp\saves\G1R-001.sav');
      expect(
        request.payload['persistentPath'],
        r'C:\tmp\saves\PersistentDataList.sav',
      );
      expect(request.payload['slot'], 'G1R-001');
      expect(request.payload['profileId'], 0);
      expect(request.payload['backup'], isTrue);

      expect(
        find.widgetWithText(TextButton, 'Restore G1R-001.sav'),
        findsOneWidget,
      );
      await tester.tap(find.widgetWithText(TextButton, 'Restore G1R-001.sav'));
      await tester.pumpAndSettle();

      final restore = core.requests.lastWhere(
        (request) => request.command == 'restore_deleted_save',
      );
      expect(restore.payload, {
        'path': r'C:\tmp\saves\G1R-001.sav',
        'backupPath': r'C:\tmp\saves\goresave_backups\G1R-001.sav.bak.301',
        'expectedPersistentSha1': 'post-delete-profile-sha',
        'expectedSaveSha1': 'deleted-save-sha',
        'expectedPersistentBackupSha1': 'deleted-persistent-sha',
      });
    },
  );

  testWidgets('deleted-save recovery blocks registered save-name edits', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(_RecoveryCoreService()),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<IconButton>(find.byKey(const ValueKey('edit-save-name')))
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<TextButton>(
            find.widgetWithText(TextButton, 'Restore G1R-009.sav'),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('profile menu opens a dedicated persistent Other saves list', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _UnassignedProfileCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Neither detached save leaks into profile 0's sidebar.
    expect(find.text('Older unassigned'), findsNothing);
    expect(find.text('Newest unassigned'), findsNothing);
    expect(find.text('Assigned save'), findsAtLeastNWidgets(1));

    await tester.tap(find.byTooltip('Switch profile'));
    await tester.pumpAndSettle();

    final otherSavesRow = find.byKey(
      const ValueKey('profile-menu-other-saves'),
    );
    expect(otherSavesRow, findsOneWidget);
    expect(find.text('Other saves'), findsOneWidget);
    expect(find.text('Open file'), findsNothing);
    expect(find.text('Newest unassigned'), findsNothing);
    expect(find.text('Older unassigned'), findsNothing);

    await tester.tap(otherSavesRow);
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('other-saves-open-file')), findsOneWidget);
    expect(find.text('Newest unassigned'), findsAtLeastNWidgets(1));
    expect(find.text('Older unassigned'), findsOneWidget);
    expect(find.text('Assigned save'), findsNothing);
    expect(find.byTooltip('Remove entry'), findsNWidgets(2));

    await tester.tap(
      find.byKey(const ValueKey(r'remove-other-save-C:\tmp\saves\G1R-002.sav')),
    );
    await tester.pumpAndSettle();
    expect(find.text('Older unassigned'), findsNothing);
    expect(find.text('Newest unassigned'), findsAtLeastNWidgets(1));
  });

  testWidgets('profile selector returns to authoritative value after failure', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _FailedProfileAssignmentCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    final selector = find.byKey(const ValueKey('save-profile-selector'));
    expect(
      find.descendant(of: selector, matching: find.text('Profile 1')),
      findsOneWidget,
    );

    await tester.tap(find.byType(DropdownButton<int>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Profile 2').last);
    await tester.pumpAndSettle();

    final assignment = core.requests.singleWhere(
      (request) => request.command == 'assign_save_profile',
    );
    expect(assignment.payload['profileId'], 1);
    expect(
      find.descendant(of: selector, matching: find.text('Profile 1')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: selector, matching: find.text('Profile 2')),
      findsNothing,
    );
  });

  testWidgets('profile switcher follows game slots instead of internal ids', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final core = _CrossedProfileNamesCoreService();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.descendant(
        of: find.byTooltip('Switch profile'),
        matching: find.text('Profile 1'),
      ),
      findsOneWidget,
    );
    await tester.tap(find.byTooltip('Switch profile'));
    await tester.pumpAndSettle();

    final profileChoices = tester
        .widgetList<PopupMenuItem<String>>(find.byType(PopupMenuItem<String>))
        .map((item) => item.value)
        .where((value) => value?.startsWith('profile:') ?? false);
    expect(profileChoices, [
      'profile:0',
      'profile:1',
      'profile:3',
      'profile:2',
    ]);
    expect(find.text('Profile 1 (1 saves)'), findsOneWidget);
    expect(find.text('Profile 2 (0 saves)'), findsOneWidget);
    expect(find.text('Profile 3 (0 saves)'), findsOneWidget);
    expect(find.text('Profile 4 (0 saves)'), findsOneWidget);

    await tester.tap(find.text('Profile 4 (0 saves)'));
    await tester.pumpAndSettle();
    final state = ProviderScope.containerOf(
      tester.element(find.byType(Scaffold).first),
    ).read(editorProvider);
    expect(state.selectedProfileId, 2);
  });

  testWidgets(
    'missing profile save is marked, not inspectable, and removable after confirmation',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final core = _MissingProfileCoreService();
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            coreServiceProvider.overrideWithValue(core),
            editorSettingsStoreProvider.overrideWithValue(
              const NoopEditorSettingsStore(),
            ),
            uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
          ],
          child: const GoresaveApp(),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Lost save'), findsOneWidget);
      expect(
        find.text(
          'File missing: G1R-009.sav is missing. It may have been deleted, moved, or renamed; '
          'the profile still references it.',
        ),
        findsOneWidget,
      );
      expect(
        core.requests.where((request) => request.command == 'inspect_save'),
        isEmpty,
      );
      expect(
        find.byKey(const ValueKey('save-actions-menu-0-G1R-009')),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('delete-save-0-G1R-009')), findsNothing);

      // Tapping the disabled row cannot turn its expected path into a failed
      // inspection. Cleanup remains available via its separate unlink action.
      await tester.tap(find.text('Lost save'));
      await tester.pump();
      expect(
        core.requests.where((request) => request.command == 'inspect_save'),
        isEmpty,
      );

      await tester.tap(
        find.byKey(const ValueKey('save-actions-menu-0-G1R-009')),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('delete-save-0-G1R-009')), findsNothing);
      await tester.tap(
        find.byKey(const ValueKey('remove-save-profile-0-G1R-009')),
      );
      await tester.pumpAndSettle();
      expect(find.text('Remove save from profile?'), findsOneWidget);
      expect(
        core.requests.where(
          (request) => request.command == 'remove_save_from_profile',
        ),
        isEmpty,
      );

      await tester.tap(
        find.widgetWithText(FilledButton, 'Remove from profile'),
      );
      await tester.pumpAndSettle();

      final remove = core.requests.singleWhere(
        (request) => request.command == 'remove_save_from_profile',
      );
      expect(remove.payload['slot'], 'G1R-009');
      expect(remove.payload['profileId'], 0);
      expect(find.text('Lost save'), findsNothing);
    },
  );
}

TextStyle _effectiveTextStyle(WidgetTester tester, Finder finder) {
  final text = tester.widget<Text>(finder);
  final inheritedStyle = DefaultTextStyle.of(tester.element(finder)).style;
  return text.style == null ? inheritedStyle : inheritedStyle.merge(text.style);
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

class _FakeCoreService implements GoresaveCoreService {
  _FakeCoreService({
    this.gameTimeTotalSeconds,
    this.playerSaveName = 'Die Welt der Verurteilten',
  });

  final double? gameTimeTotalSeconds;
  final String? playerSaveName;
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'fake-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    requests.add(_RecordedRequest(command, Map<String, Object?>.from(payload)));
    switch (command) {
      case 'scan_save_dir':
        return {
          'ok': true,
          'data': {
            'saveRoot': r'C:\tmp\saves',
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'slot': 'G1R-001',
                'format': 'GSAV',
                'fileSize': 914367,
                'sha1': 'abc',
                'status': 'ok',
                'persistentProfileId': 0,
                'playerSaveName': playerSaveName,
                'persistentPlayerSaveName':
                    'Die Welt der Verurteilten, Tag 1, 13:07',
                'chapterId': 1,
                'mapName': 'MainMap',
                'timePlayedSeconds': 6963.34,
                'quickSave': false,
                'autoSave': true,
                'slotName': 'G1R-001',
                'compressionMethod': 'Oodle',
                'chunkCount': 451,
                'screenshot': {
                  'mimeType': 'image/png',
                  'byteLength': 68,
                  'bytesBase64':
                      'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=',
                },
              },
            ],
            'profiles': [
              {
                'profileId': 0,
                'profileName': '0',
                'quickSaveSlots': ['G1R-001', 'G1R-002', 'G1R-003'],
                'autoSaveSlots': ['G1R-001', 'G1R-002'],
                'savedSlots': ['G1R-001'],
                'difficultyPreset': 'DifficultyPreset_Custom',
                'maxQuick': 3,
                'maxAuto': 2,
              },
            ],
            'activeProfileId': 0,
          },
        };
      case 'inspect_save':
        final preview = payload.containsKey('privateChunkLimit');
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 914367,
            'sha1': 'abc',
            'trailerSize': 44,
            'screenshot': {
              'mimeType': 'image/png',
              'byteLength': 68,
              'bytesBase64':
                  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=',
            },
            'public': {'slotName': 'G1R-001', 'playerSaveName': playerSaveName},
            'difficulty': {'preset': 'DifficultyPreset_Custom'},
            'persistent': {
              'playerSaveName': 'Die Welt der Verurteilten, Tag 1, 13:07',
              'chapterId': 1,
              'mapName': 'MainMap',
              'timePlayedSeconds': 6963.34,
              'timeLoadedSeconds': 0.0,
              'quickSave': false,
              'autoSave': true,
              'profileId': 0,
            },
            'compressedStream': {
              'method': 'Oodle',
              'algorithmId': 2,
              'chunkCount': 451,
              'compressedSize': 905728,
              'uncompressedSize': 59049891,
              'trailingSize': 44,
            },
            'private': {
              'status': preview ? 'decoded_preview' : 'decoded',
              'typedParse': {
                'status': preview ? 'skipped_preview' : 'ok',
                'propertyCount': preview ? 0 : 2,
                'maxDepth': preview ? 0 : 2,
              },
              'message': preview
                  ? 'Private payload preview decoded through the G1R codec host.'
                  : 'Private payload decoded through the G1R codec host.',
              'preview': preview,
              'decodedChunkCount': preview ? 1 : null,
              'totalChunkCount': preview ? 541 : null,
              'decompressedSize': 59049891,
              'stringCount': preview ? 1 : 3,
              'strings': preview ? ['Hero'] : ['Hero', 'ChapterOne', 'OreBar'],
              'player': {
                'saveVersionNumber': 17,
                'currentWorld': 'WORLD',
                'playerName': 'Hero',
                'profileName': '0',
                'transform': {
                  'location': {'x': 10.0, 'y': 20.0, 'z': 30.0},
                  'rotation': {'pitch': 40.0, 'yaw': 50.0, 'roll': 60.0},
                },
                'attributes': [
                  {'id': 'Health', 'baseValue': 40.0, 'currentValue': 25.0},
                  {'id': 'Strength', 'baseValue': 10.0, 'currentValue': 10.0},
                ],
                'scriptPaths': ['/Script/Angelscript.GothicFinalDataGame'],
                'properties': ['m_SaveVersionNumber', 'm_CurrentWorld'],
                'writable': [
                  'private.player.setPlayerName',
                  'private.profile.setProfileName',
                  'private.player.setAttribute',
                  'private.player.setTransform',
                ],
              },
              'inventory': {
                'candidateCount': 2,
                'candidates': ['ITMI_GOLD', 'BP_Item_Ore'],
                'itemStackCount': 2,
                'itemScope': 'player_inventory_region',
                'items': [
                  {
                    'id': 'ItMi_Orenugget',
                    'path': '/Script/Angelscript.ItMi_Orenugget',
                    'count': 42,
                  },
                  {
                    'id': 'ItFo_Cheese',
                    'path': '/Script/Angelscript.ItFo_Cheese',
                    'count': 1,
                  },
                ],
                'scriptPaths': ['/Script/G1R.InventorySaveGameData'],
                'properties': ['m_InventoryItems', 'm_StackCount'],
                'writable': ['private.inventory.setItemCount'],
              },
              'progression': {
                'status': 'ok',
                'questTotal': 3,
                'questStates': {'Available': 1, 'Running': 1, 'Succeeded': 1},
                'knowledgeCharacters': 2,
                'knowledgeEntries': 5,
                'memoryCharacters': 1,
                'memoryEvents': 12,
                'writable': [
                  'private.typed.setValue',
                  'private.typed.setAdd',
                  'private.typed.setRemove',
                  'private.typed.arrayRemove',
                  'private.typed.arrayDuplicate',
                ],
              },
            },
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'backups': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav.bak.200',
                'fileName': 'G1R-001.sav.bak.200',
                'fileSize': 913000,
                'sha1': 'backup-sha',
                'createdEpoch': 200,
                'status': 'ok',
                'playerSaveName': 'Before edit',
              },
            ],
            'companionBackups': [
              {
                'path': r'C:\tmp\saves\PersistentDataList.sav.bak.250',
                'fileName': 'PersistentDataList.sav.bak.250',
                'fileSize': 4096,
                'sha1': 'persistent-backup-sha',
                'createdEpoch': 250,
                'status': 'ok',
                'scope': 'persistent_data_list',
                'slotName': 'G1R-001',
                'playerSaveName': 'Before companion edit',
              },
            ],
          },
        };
      case 'check_codec':
        return {
          'ok': true,
          'data': {
            'available': true,
            'canDecompress': true,
            'canCompress': true,
            'status': 'ready',
            'adapter': 'pure_rust_kraken',
            'message': 'Codec host is ready.',
          },
        };
      case 'private.characters.list':
        // Backs the Charaktere master list. This test drives the pinned Player
        // row (selected by default), so no spawned actors are needed here.
        return {
          'ok': true,
          'data': {'total': 0, 'characters': <Object?>[]},
        };
      case 'write_save':
        final syncPersistent = payload['syncPersistentDataList'] == true;
        return {
          'ok': true,
          'data': {
            'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1',
            if (syncPersistent) ...{
              'persistentBackupPath':
                  r'C:\tmp\saves\PersistentDataList.sav.bak.2',
              'persistentBytesChanged': true,
            },
          },
        };
      case 'restore_backup':
      case 'restore_deleted_save':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'restoredFrom': payload['backupPath'],
            'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.300',
          },
        };
      case 'delete_save':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'slot': payload['slot'],
            'profileId': payload['profileId'],
            'backupPath': r'C:\tmp\saves\goresave_backups\G1R-001.sav.bak.301',
            'persistentBackupPath':
                r'C:\tmp\saves\goresave_backups\PersistentDataList.sav.bak.301',
            'persistentPostDeleteSha1': 'post-delete-profile-sha',
            'deletedSaveSha1': 'deleted-save-sha',
            'deletedPersistentSha1': 'deleted-persistent-sha',
          },
        };
      case 'dismiss_deleted_save_recovery':
        return {
          'ok': true,
          'data': {'dismissed': true},
        };
      case 'search_typed_properties':
        if (payload['query'] == 'GameTime' && gameTimeTotalSeconds != null) {
          return {
            'ok': true,
            'data': {
              'source': 'private',
              'offset': 0,
              'limit': payload['limit'] ?? 1000,
              'total': 1,
              'count': 1,
              'summary': {
                'sources': {'private': 1},
                'kinds': {'scalar': 1},
                'types': {'DoubleProperty': 1},
                'editable': 1,
                'readOnly': 0,
                'typedSources': ['private'],
              },
              'results': [
                {
                  'id': 'private:game-time',
                  'source': 'private',
                  'path': [
                    'm_GenericData',
                    '{GameTime}',
                    'CurrentTime',
                    'TotalSeconds',
                  ],
                  'display':
                      'm_GenericData{GameTime} › CurrentTime › TotalSeconds',
                  'type': 'DoubleProperty',
                  'kind': 'scalar',
                  'value': gameTimeTotalSeconds.toString(),
                  'editValue': gameTimeTotalSeconds,
                  'editable': true,
                  'childCount': 0,
                  'depth': 2,
                },
              ],
            },
          };
        }
        return {
          'ok': true,
          'data': {
            'source': payload['source'] ?? 'private',
            'offset': payload['offset'] ?? 0,
            'limit': payload['limit'] ?? 50,
            'total': 2,
            'count': 2,
            'summary': {
              'sources': {'private': 2},
              'kinds': {'nativeStruct': 1, 'array': 1},
              'types': {'StructProperty': 1, 'ArrayProperty': 1},
              'editable': 1,
              'readOnly': 1,
              'typedSources': ['private'],
            },
            'results': [
              {
                'id': 'private:1',
                'source': 'private',
                'path': ['Transform', 'Location'],
                'display': 'Transform › Location',
                'type': 'StructProperty',
                'structType': 'Vector',
                'kind': 'nativeStruct',
                'value': 'x: 1, y: 2, z: 3',
                'editValue': {'x': 1.0, 'y': 2.0, 'z': 3.0},
                'editable': true,
                'childCount': 0,
                'depth': 1,
              },
              {
                'id': 'private:2',
                'source': 'private',
                'path': ['Events'],
                'display': 'Events',
                'type': 'ArrayProperty',
                'kind': 'array',
                'value': '12 elements',
                'editable': false,
                'childCount': 12,
                'depth': 0,
              },
            ],
          },
        };
      case 'query_progression':
        final section = payload['section'] as String? ?? 'quests';
        if (section == 'quests') {
          return {
            'ok': true,
            'data': {
              'section': 'quests',
              'total': 1,
              'offset': 0,
              'limit': 100,
              'count': 1,
              'stateCounts': {'Running': 1},
              'quests': [
                {
                  'questClass': '/Script/Angelscript.Quest_OldCamp_SLEEPER',
                  'id': 'Quest_OldCamp_SLEEPER',
                  'group': 'OldCamp',
                  'name': 'SLEEPER',
                  'currentState': 'EQuestState::Running',
                  'statePath': [
                    'QuestDataByClass',
                    '{/Script/Angelscript.Quest_OldCamp_SLEEPER}',
                    'CurrentState',
                  ],
                  'writable': true,
                },
              ],
            },
          };
        }
        if (section == 'knowledge') {
          final character = payload['character'] as String?;
          if (character == null) {
            return {
              'ok': true,
              'data': {
                'section': 'knowledge',
                'total': 1,
                'offset': 0,
                'limit': 100,
                'count': 1,
                'characters': [
                  {'name': 'OC_STT_Diego', 'entryCount': 2},
                ],
              },
            };
          }
          return {
            'ok': true,
            'data': {
              'section': 'knowledge',
              'character': character,
              'total': 1,
              'offset': 0,
              'limit': 200,
              'count': 1,
              'entries': ['Voiceline_info_diego'],
              'setPath': [
                'CharacterKnowledgeByUniqueName',
                '{$character}',
                'Knowledge',
              ],
            },
          };
        }
        // events section
        final character = payload['character'] as String?;
        if (character == null) {
          return {
            'ok': true,
            'data': {
              'section': 'events',
              'total': 1,
              'offset': 0,
              'limit': 100,
              'count': 1,
              'characters': [
                {'id': 'Hero', 'eventCount': 1},
              ],
            },
          };
        }
        return {
          'ok': true,
          'data': {
            'section': 'events',
            'character': character,
            'total': 1,
            'offset': 0,
            'limit': 100,
            'count': 1,
            'events': [
              {
                'index': 0,
                'tags': ['Memory.Quest.Started'],
                'timeSeconds': 100.0,
                'affected': 'Hero',
              },
            ],
            'arrayPath': [
              'LongTermMemoryByGlobalId',
              '{$character}',
              'MemorizedEvents',
            ],
          },
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}

class _EmptyCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command != 'scan_save_dir') {
      return super.execute(command, payload: payload);
    }
    requests.add(_RecordedRequest(command, Map<String, Object?>.from(payload)));
    return {
      'ok': true,
      'data': {
        'saveRoot': r'C:\tmp\saves',
        'saves': <Object?>[],
        'profiles': <Object?>[],
        'activeProfileId': null,
      },
    };
  }
}

class _RecoveryCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'scan_save_dir') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    data['deletedSaveRecovery'] = {
      'targetPath': r'C:\tmp\saves\G1R-009.sav',
      'backupPath': r'C:\tmp\saves\goresave_backups\G1R-009.sav.bak.301',
      'persistentPostDeleteSha1': 'post-delete-profile-sha',
      'deletedSaveSha1': 'deleted-save-sha',
      'deletedPersistentSha1': 'deleted-persistent-sha',
    };
    return response;
  }
}

class _StatisticsCoreService extends _FakeCoreService {
  _StatisticsCoreService() : super(gameTimeTotalSeconds: 1413433);

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      final response = await super.execute(command, payload: payload);
      final data = (response['data'] as Map).cast<String, Object?>();
      final saves = data['saves'] as List;
      (saves.single as Map).remove('screenshot');
      return response;
    }
    if (command == 'inspect_save') {
      final response = await super.execute(command, payload: payload);
      final data = (response['data'] as Map).cast<String, Object?>();
      data.remove('screenshot');
      final private = (data['private'] as Map).cast<String, Object?>();
      final progression = (private['progression'] as Map)
          .cast<String, Object?>();
      final questStates = (progression['questStates'] as Map)
          .cast<String, Object?>();
      questStates['Unavailable'] = 40;
      questStates['NotRunning'] = 30;
      private['factions'] = {
        // One global crime implicates both guilds. The Overview must use the
        // unique global count instead of summing the duplicated guild rows.
        'openCrimes': 1,
        'guilds': [
          {
            'guild': 'Guild.Human.OldCamp',
            'label': 'OldCamp',
            'total': 1,
            'forgiven': 0,
            'unforgiven': 1,
            'isHostile': true,
            'crimes': {'assault': 1},
          },
          {
            'guild': 'Guild.Human.NewCamp',
            'label': 'NewCamp',
            'total': 1,
            'forgiven': 0,
            'unforgiven': 1,
            'isHostile': false,
            'crimes': {'assault': 1},
          },
        ],
      };
      return response;
    }
    if (command == 'private.characters.list') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'total': 4,
          'characters': [
            {
              'globalId': 'Hero_Global',
              'uniqueName': 'Hero',
              'isDead': false,
              'hasInventory': true,
              'hasKnowledge': true,
              'hasEvents': true,
            },
            {
              'globalId': 'Diego_Global',
              'uniqueName': 'OC_STT_Diego',
              'isDead': false,
              'hasInventory': true,
              'hasKnowledge': true,
              'hasEvents': true,
              'isTrader': true,
            },
            {
              'globalId': 'Molerat_Global',
              'uniqueName': 'Creature_Molerat',
              'isDead': true,
              'hasInventory': false,
              'hasKnowledge': false,
              'hasEvents': true,
            },
            {
              'globalId': null,
              'uniqueName': 'NC_BAU_Homer_935',
              'isDead': false,
              'hasInventory': false,
              'hasKnowledge': true,
              'hasEvents': false,
              'isTrader': true,
            },
          ],
        },
      };
    }
    if (command == 'private.skills.list') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'actor': 'Hero',
          'found': true,
          'skills': [
            {
              'base': 'OneHanded',
              'label': 'One-handed',
              'category': 'Combat',
              'kind': 'ladder',
              'learned': true,
              'current': 'Trained',
              'hasUntrained': true,
              'options': [
                {'value': 'Untrained'},
                {'value': 'Trained'},
              ],
            },
            {
              'base': 'Sneaking',
              'label': 'Sneaking',
              'category': 'Thief',
              'kind': 'binary',
              'learned': true,
              'current': 'Learned',
              'hasUntrained': true,
              'options': [
                {'value': 'Untrained'},
                {'value': 'Learned'},
              ],
            },
          ],
        },
      };
    }
    if (command == 'private.factions.list') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'openCrimes': 1,
          'guilds': [
            {
              'guild': 'Guild.Human.OldCamp',
              'label': 'OldCamp',
              'total': 1,
              'forgiven': 0,
              'unforgiven': 1,
              'isHostile': true,
              'crimes': {'assault': 1},
            },
            {
              'guild': 'Guild.Human.NewCamp',
              'label': 'NewCamp',
              'total': 1,
              'forgiven': 0,
              'unforgiven': 1,
              'isHostile': false,
              'crimes': {'assault': 1},
            },
          ],
        },
      };
    }
    if (command == 'search_typed_properties' &&
        payload['query'] == 'AttributesByGlobalId {Hero}') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      const values = {
        'Level': 18.0,
        'Experience': 42850.0,
        'SkillPoints': 12.0,
        'Health': 140.0,
        'MaxHealth': 160.0,
        'Mana': 42.0,
        'MaxMana': 60.0,
      };
      final results = [
        for (final entry in values.entries)
          {
            'id': 'private:hero-${entry.key}',
            'source': 'private',
            'path': [
              'AttributesByGlobalId',
              '{Hero}',
              'AttributeSetsByClass',
              '{HeroAttributes}',
              '{${entry.key}}',
              'CurrentValue',
            ],
            'display':
                'AttributesByGlobalId › {Hero} › {${entry.key}} › CurrentValue',
            'type': 'FloatProperty',
            'kind': 'scalar',
            'value': entry.value.toString(),
            'editValue': entry.value,
            'editable': true,
            'childCount': 0,
            'depth': 5,
          },
      ];
      return {
        'ok': true,
        'data': {
          'source': 'private',
          'offset': 0,
          'limit': payload['limit'] ?? 1000,
          'total': results.length,
          'count': results.length,
          'summary': {
            'sources': {'private': results.length},
            'kinds': {'scalar': results.length},
            'types': {'FloatProperty': results.length},
            'editable': results.length,
            'readOnly': 0,
            'typedSources': ['private'],
          },
          'results': results,
        },
      };
    }
    if (command == 'query_progression' &&
        payload['section'] == 'events' &&
        payload['character'] == 'Hero_Global') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'section': 'events',
          'character': 'Hero_Global',
          'total': 9,
          'offset': 0,
          'limit': payload['limit'] ?? 500,
          'count': 9,
          'events': [
            {
              'index': 0,
              'tags': ['Memory.Guild.Joined', 'Guild.Human.OldCamp.Guard'],
              'timeSeconds': 100.0,
              'affected': 'Hero',
            },
            {
              'index': 1,
              'tags': [
                'Memory.Character.Defeated.Kill',
                'Species.Creature.Scavenger',
              ],
              'timeSeconds': 200.0,
              'affected': 'Creature_Scavenger_Adult',
            },
            {
              'index': 2,
              'tags': ['Memory.Execution'],
              'timeSeconds': 300.0,
              'affected': 'OC_STT_Diego-01234567-89ab-cdef-0123-456789abcdef',
            },
            {
              'index': 3,
              'tags': ['Memory.Character.Defeated'],
              'timeSeconds': 400.0,
              'affected': 'OC_STT_Diego',
            },
            {
              'index': 4,
              'tags': ['Memory.Guild.Joined', 'Guild.Human.NewCamp.Mercenary'],
              'affected': 'Hero',
            },
            {
              'index': 5,
              'tags': ['Memory.Character.Defeated.Kill'],
              // `Human` is a real catch-all (`other`) catalog entry. It must
              // stay unknown rather than inflating the monster count.
              'affected': 'Human',
            },
            for (final loss in const [
              (6, 'Memory.WasDefeated'),
              (7, 'Memory.Combat.WasDefeated'),
              (8, 'Memory.SaveAndLoad.Defeated'),
            ])
              {
                'index': loss.$1,
                'tags': [loss.$2],
                'affected': 'OC_STT_Diego',
              },
          ],
          'arrayPath': [
            'LongTermMemoryByGlobalId',
            '{Hero_Global}',
            'MemorizedEvents',
          ],
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

class _ExpelledStatisticsCoreService extends _StatisticsCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'query_progression' ||
        payload['section'] != 'events' ||
        payload['character'] != 'Hero_Global') {
      return response;
    }

    final data = (response['data'] as Map).cast<String, Object?>();
    final events = data['events'] as List<Object?>;
    events.add({
      'index': 9,
      'tags': ['Memory.Guild.Expelled', 'Guild.Human.NewCamp.Mercenary'],
      'affected': 'Hero',
    });
    data['total'] = events.length;
    data['count'] = events.length;
    return response;
  }
}

class _UnavailableStatisticsCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'private.skills.list') {
      return {
        'ok': true,
        'data': {
          'actor': 'Hero',
          'found': false,
          'skills': [
            {
              'base': 'Sneaking',
              'label': 'Sneaking',
              'category': 'Thief',
              'kind': 'binary',
              'learned': false,
              'current': 'Untrained',
              'hasUntrained': true,
              'options': [
                {'value': 'Untrained'},
                {'value': 'Learned'},
              ],
            },
          ],
        },
      };
    }
    final response = await super.execute(command, payload: payload);
    if (command != 'inspect_save') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    data['private'] = {
      'status': 'unavailable',
      'typedParse': {'status': 'failed'},
      'progression': {'status': 'failed'},
      'inventory': <String, Object?>{},
    };
    return response;
  }
}

class _TruncatedStatisticsCoreService extends _StatisticsCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'inspect_save') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    final private = (data['private'] as Map).cast<String, Object?>();
    final inventory = (private['inventory'] as Map).cast<String, Object?>();
    inventory['itemStackCount'] = 4097;
    return response;
  }
}

class _UnknownCountStatisticsCoreService extends _StatisticsCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'inspect_save') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    final private = (data['private'] as Map).cast<String, Object?>();
    final inventory = (private['inventory'] as Map).cast<String, Object?>();
    final items = inventory['items'] as List<Object?>;
    (items.first as Map).remove('count');
    return response;
  }
}

class _GlobalInventoryStatisticsCoreService extends _StatisticsCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'inspect_save') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    final private = (data['private'] as Map).cast<String, Object?>();
    final inventory = (private['inventory'] as Map).cast<String, Object?>();
    inventory['itemScope'] = 'global_observed';
    return response;
  }
}

class _DeletingSaveCoreService extends _FakeCoreService {
  bool _deleted = false;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir' && _deleted) {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'saveRoot': r'C:\tmp\saves',
          'saves': <Object?>[],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'quickSaveSlots': ['G1R-001', 'G1R-002', 'G1R-003'],
              'autoSaveSlots': ['G1R-001', 'G1R-002'],
              'savedSlots': <Object?>[],
              'difficultyPreset': 'DifficultyPreset_Custom',
              'maxQuick': 3,
              'maxAuto': 2,
            },
          ],
          'activeProfileId': 0,
        },
      };
    }

    final response = await super.execute(command, payload: payload);
    if (command == 'delete_save') _deleted = true;
    if (command == 'restore_deleted_save') _deleted = false;
    return response;
  }
}

class _FailedProfileAssignmentCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'scan_save_dir') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    final profiles = data['profiles'] as List<Object?>;
    profiles.add({
      'profileId': 1,
      'profileName': '1',
      'quickSaveSlots': <String>[],
      'autoSaveSlots': <String>[],
      'savedSlots': <String>[],
      'difficultyPreset': 'DifficultyPreset_Custom',
      'maxQuick': 3,
      'maxAuto': 2,
    });
    return response;
  }
}

class _CrossedProfileNamesCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'scan_save_dir') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    data['profiles'] = [
      {'profileId': 3, 'profileName': '2', 'savedSlots': <String>[]},
      {'profileId': 1, 'profileName': '1', 'savedSlots': <String>[]},
      {'profileId': 2, 'profileName': '3', 'savedSlots': <String>[]},
      {
        'profileId': 0,
        'profileName': '0',
        'savedSlots': ['G1R-001'],
      },
    ];
    return response;
  }
}

class _MissingProfileCoreService extends _FakeCoreService {
  var _removed = false;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'saveRoot': r'C:\tmp\saves',
          'saves': _removed
              ? <Object?>[]
              : [
                  {
                    'path': r'C:\tmp\saves\G1R-009.sav',
                    'slot': 'G1R-009',
                    'format': 'MISSING',
                    'fileSize': 0,
                    'sha1': '',
                    'status': 'missing',
                    'persistentPlayerSaveName': 'Lost save',
                    'persistentProfileId': 0,
                  },
                ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': _removed ? <String>[] : ['G1R-009'],
            },
          ],
          'activeProfileId': 0,
        },
      };
    }
    if (command == 'remove_save_from_profile') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      _removed = true;
      return {
        'ok': true,
        'data': {
          'slot': payload['slot'],
          'profileId': payload['profileId'],
          'bytesChanged': true,
          'backupPath': null,
          'persistentBackupPath':
              r'C:\tmp\saves\goresave_backups\PersistentDataList.sav.bak.1',
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

class _UnassignedProfileCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      requests.add(
        _RecordedRequest(command, Map<String, Object?>.from(payload)),
      );
      return {
        'ok': true,
        'data': {
          'saveRoot': r'C:\tmp\saves',
          'saves': [
            {
              'path': r'C:\tmp\saves\G1R-001.sav',
              'slot': 'G1R-001',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'assigned',
              'status': 'ok',
              'playerSaveName': 'Assigned save',
              'timePlayedSeconds': 100.0,
              'persistentProfileId': 0,
            },
            {
              'path': r'C:\tmp\saves\G1R-002.sav',
              'slot': 'G1R-002',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'old',
              'status': 'ok',
              'playerSaveName': 'Older unassigned',
              'timePlayedSeconds': 10.0,
            },
            {
              'path': r'C:\tmp\saves\G1R-003.sav',
              'slot': 'G1R-003',
              'format': 'GSAV',
              'fileSize': 100,
              'sha1': 'new',
              'status': 'ok',
              'playerSaveName': 'Newest unassigned',
              'timePlayedSeconds': 20.0,
            },
          ],
          'profiles': [
            {
              'profileId': 0,
              'profileName': '0',
              'savedSlots': ['G1R-001'],
            },
          ],
          'activeProfileId': 0,
        },
      };
    }
    return super.execute(command, payload: payload);
  }
}

/// A fake core that enables the inventory remove (trash) UI: it verifies the
/// typed parse, advertises `private.inventory.removeItem`, and marks the
/// Orenugget stack removable while the Cheese stack is NOT removable (its asset
/// path occurs in more than one container — e.g. also equipped / in a
/// quickslot — so the core can't unambiguously remove it).
class _RemovableInventoryCoreService extends _FakeCoreService {
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final result = await super.execute(command, payload: payload);
    if (command != 'inspect_save') return result;
    final data = (result['data'] as Map).cast<String, Object?>();
    final private = (data['private'] as Map).cast<String, Object?>();
    // Mark the typed parse verified so addItem/removeItem are gated open.
    private['typedParse'] = {'status': 'ok', 'propertyCount': 1, 'maxDepth': 1};
    final inventory = (private['inventory'] as Map).cast<String, Object?>();
    inventory['writable'] = const [
      'private.inventory.setItemCount',
      'private.inventory.removeItem',
    ];
    final items = (inventory['items'] as List)
        .map((e) => (e as Map).cast<String, Object?>())
        .toList();
    for (final item in items) {
      // Orenugget is uniquely in the MainContainer → removable; Cheese also
      // lives in another container → not removable (trash disabled).
      item['removable'] = item['id'] == 'ItMi_Orenugget';
    }
    inventory['items'] = items;
    return result;
  }
}

class _SlowInspectCoreService extends _FakeCoreService {
  final _pending = <Completer<void>>[];

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'inspect_save') {
      final completer = Completer<void>();
      _pending.add(completer);
      await completer.future;
    }
    return super.execute(command, payload: payload);
  }

  void completePending() {
    for (final completer in _pending) {
      if (!completer.isCompleted) {
        completer.complete();
      }
    }
    _pending.clear();
  }
}
