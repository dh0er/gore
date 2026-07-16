import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';

void main() {
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
    expect(find.text('Die Welt der Verurteilten'), findsAtLeastNWidgets(1));
    expect(find.text('Overview'), findsOneWidget);
    expect(find.text('Public save name'), findsOneWidget);
    // Header pills summarise chapter and time played for the save.
    expect(find.text('Chapter 1'), findsOneWidget);
    expect(find.text('1 hr 56 min'), findsOneWidget);
    expect(find.text('Profile 0'), findsWidgets);
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
    expect(find.byType(DropdownButtonFormField<int>), findsOneWidget);
    expect(
      find.byKey(const ValueKey('remove-selected-save-profile')),
      findsOneWidget,
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

    // Edit the public save name — button label gains count.
    await tester.enterText(
      find.widgetWithText(TextField, 'Public save name'),
      'Much Longer Save Name',
    );
    await tester.pump();
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
    await tester.tap(find.widgetWithText(Tab, 'Attributes'));
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

    await tester.scrollUntilVisible(
      find.text('Position'),
      120,
      scrollable: find.byType(Scrollable).last,
    );
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
    await tester.tap(find.widgetWithText(Tab, 'Inventory'));
    await tester.pumpAndSettle();

    // Stacks are grouped by category in a sidebar; Food is selected first
    // (it precedes Miscellaneous in category order), so Cheese is visible and
    // the misc Ore stack lives behind its own category tile.
    expect(find.text('Food & potions (1)'), findsOneWidget);
    expect(find.text('Miscellaneous (1)'), findsOneWidget);
    expect(find.text('ItFo_Cheese'), findsOneWidget);

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

    expect(find.text('G1R-001.sav.bak.200'), findsOneWidget);
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
    expect(find.text('PersistentDataList.sav.bak.250'), findsOneWidget);
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

    await tester.tap(find.text('Inspection JSON'));
    await tester.pumpAndSettle();

    expect(find.textContaining('"format"'), findsOneWidget);
  });

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

    // Enter a draft in the public name field on Overview.
    await tester.enterText(
      find.widgetWithText(TextField, 'Public save name'),
      'Draft Name',
    );
    await tester.pump();
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

    // The draft text must still be visible in the field.
    final field = find.widgetWithText(TextField, 'Public save name');
    final editableText = tester.widget<EditableText>(
      find.descendant(of: field, matching: find.byType(EditableText)),
    );
    expect(editableText.controller.text, 'Draft Name');
    // Save button still shows 1.
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
  });

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

    // Enter a draft in the public name field.
    final originalName = 'Die Welt der Verurteilten';
    await tester.enterText(
      find.widgetWithText(TextField, 'Public save name'),
      'Edited Name',
    );
    await tester.pump();
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

    // The field must display the canonical (original) name again.
    final field = find.widgetWithText(TextField, 'Public save name');
    final editableText = tester.widget<EditableText>(
      find.descendant(of: field, matching: find.byType(EditableText)),
    );
    expect(editableText.controller.text, originalName);
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
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Inventory is now a sub-tab inside the Charaktere (Characters) tab.
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(Tab, 'Inventory'));
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

      // Tapping the disabled row cannot turn its expected path into a failed
      // inspection. Cleanup remains available via its separate unlink action.
      await tester.tap(find.text('Lost save'));
      await tester.pump();
      expect(
        core.requests.where((request) => request.command == 'inspect_save'),
        isEmpty,
      );

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

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

class _FakeCoreService implements GoresaveCoreService {
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
                'playerSaveName': 'Die Welt der Verurteilten',
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
            'public': {
              'slotName': 'G1R-001',
              'playerSaveName': 'Die Welt der Verurteilten',
            },
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
                'itemStackCount': 1,
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
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'restoredFrom': payload['backupPath'],
            'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.300',
          },
        };
      case 'search_typed_properties':
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
