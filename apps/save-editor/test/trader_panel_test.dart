import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/trader_models.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/features/editor/ui/pending_structural_row.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';
import 'support/detail_tabs.dart';
import 'package:goresave/features/editor/ui/characters_tab.dart';

/// The Handel (trade) sub-tab. A merchant's shop is NOT his inventory: it lives
/// in a global array addressed by index, and his ore inside that shop is what he
/// can pay with. These tests pin the three things that are easy to get wrong —
/// index (not name) addressing, "no ore line" being distinct from zero, and a
/// structural add/remove being kept out of the batched edits.
/// A raw All-Data array removal aimed at the trader array itself.
Map<String, Object?> arrayRemoveOnTraders() => {
  'path': 'private.typed.arrayRemove',
  'value': {
    'path': ['m_GenericData', '{GameStateDataBase}', 'm_Traders'],
    'index': 0,
  },
};

void main() {
  group('trader edit encoding', () {
    test('activity time uses the exact core-provided typed path', () {
      const path = [
        'Wrapper',
        'm_GenericData',
        '{GameStateDataBase}',
        'm_Traders',
        '[11]',
        'm_TotalSeconds',
      ];
      const edit = TraderActivityTimeEdit(
        index: 11,
        propertyPath: path,
        totalSeconds: 12345.5,
      );
      expect(edit.pendingKey, 'traders:11:activityTime');
      expect(edit.toEdit(), {
        'path': 'private.typed.setValue',
        'value': {'path': path, 'value': 12345.5},
      });
    });

    test('setStock sends the map and count, addressed by index', () {
      const edit = TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 11,
        map: TraderStockMap.current,
        path: kTraderOrePath,
        count: 4242,
      );
      expect(edit.toEdit(), {
        'path': 'private.traders.setStock',
        'value': {
          'index': 11,
          'path': kTraderOrePath,
          'map': 'current',
          'count': 4242,
        },
      });
      // Length-neutral, so it may share a write with its peers.
      expect(edit.isStructural, isFalse);
    });

    test('removeItem omits the count it has no use for', () {
      const edit = TraderStockEdit(
        kind: TraderEditKind.removeItem,
        index: 3,
        map: TraderStockMap.base,
        path: '/Script/Angelscript.ItFo_Loaf',
      );
      expect(edit.toEdit()['value'], {
        'index': 3,
        'path': '/Script/Angelscript.ItFo_Loaf',
        'map': 'default',
      });
      // Splices the map body, so the notifier must give it its own write.
      expect(edit.isStructural, isTrue);
    });

    test('addItem is structural and carries its starting count', () {
      const edit = TraderStockEdit(
        kind: TraderEditKind.addItem,
        index: 0,
        map: TraderStockMap.current,
        path: '/Script/Angelscript.ItFo_Cheese',
        count: 9,
      );
      expect(edit.commandPath, 'private.traders.addItem');
      expect((edit.toEdit()['value'] as Map)['count'], 9);
      expect(edit.isStructural, isTrue);
    });

    test('the pending key separates trader, map and line', () {
      const a = TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 1,
        map: TraderStockMap.current,
        path: kTraderOrePath,
      );
      const b = TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 1,
        map: TraderStockMap.base,
        path: kTraderOrePath,
      );
      const c = TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 2,
        map: TraderStockMap.current,
        path: kTraderOrePath,
      );
      expect(a.pendingKey, isNot(b.pendingKey));
      expect(a.pendingKey, isNot(c.pendingKey));
      // Same line edited twice replaces rather than stacks.
      const again = TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 1,
        map: TraderStockMap.current,
        path: kTraderOrePath,
        count: 7,
      );
      expect(again.pendingKey, a.pendingKey);
    });
  });

  group('trader list model', () {
    test('a placeholder row never answers a name lookup', () {
      // Two shipped rows are named `None` and belong to no NPC. Matching one
      // would attach a stranger's shop to whichever character is selected.
      final result = TradersResult.fromJson({
        'traders': [
          {'index': 0, 'uniqueName': 'None', 'placeholder': true},
          {'index': 1, 'uniqueName': 'OC_STT_Dexter_329', 'ore': 55},
          {'index': 2, 'uniqueName': 'None', 'placeholder': true},
        ],
        'writable': ['private.traders.setStock'],
      });
      expect(result.forUniqueName('None'), isNull);
      expect(result.forUniqueName('OC_STT_Dexter_329')?.index, 1);
      expect(result.forUniqueName('NC_ORG_Wolf_855'), isNull);
    });

    test('a name matches case-insensitively, the way the core joins it', () {
      // A character's unique name is the stored knowledge key where one exists,
      // and that key's casing can differ from the trader row's. The core badges
      // the merchant through a lowercase match, so an exact compare here would
      // badge him and then deny it.
      final result = TradersResult.fromJson({
        'traders': [
          {'index': 4, 'uniqueName': 'OC_STT_Dexter_329', 'ore': 55},
        ],
      });
      expect(result.forUniqueName('oc_stt_dexter_329')?.index, 4);
      expect(result.forUniqueName('OC_stt_Dexter_329')?.index, 4);
      expect(result.isAmbiguous('OC_STT_Dexter_329'), isFalse);
    });

    test('two rows sharing a name are refused, not guessed between', () {
      // The index this returns is what every edit is addressed by, so picking
      // the first hit would edit an arbitrary shop. The core refuses the same
      // case.
      final result = TradersResult.fromJson({
        'traders': [
          {'index': 4, 'uniqueName': 'OC_STT_Dexter_329', 'ore': 55},
          {'index': 9, 'uniqueName': 'oc_stt_dexter_329', 'ore': 12},
        ],
      });
      expect(result.forUniqueName('OC_STT_Dexter_329'), isNull);
      expect(result.isAmbiguous('OC_STT_Dexter_329'), isTrue);
      expect(result.allForUniqueName('OC_STT_Dexter_329'), hasLength(2));
      // A name nobody carries is absent, not ambiguous — the panel says
      // different things about the two.
      expect(result.isAmbiguous('NC_ORG_Wolf_855'), isFalse);
    });

    test('a missing ore line reads as null, not zero', () {
      // Riordian stocks goods but carries no ore key. Showing 0 would claim he
      // is broke; null says the record has no such line at all.
      final result = TradersResult.fromJson({
        'traders': [
          {'index': 0, 'uniqueName': 'NC_KDW_Riordian_605', 'itemCount': 4},
          {'index': 1, 'uniqueName': 'OC_STT_Dexter_329', 'ore': 55},
        ],
      });
      expect(result.traders[0].ore, isNull);
      expect(result.traders[1].ore, 55);
    });

    test('command availability is feature-detected, not assumed', () {
      // An older core offers no trader writes; the panel must stay read-only
      // rather than send a command that does not exist.
      final old = TradersResult.fromJson({'traders': <Object?>[]});
      expect(old.canSetStock, isFalse);
      expect(old.canAddItem, isFalse);
      expect(old.canRemoveItem, isFalse);
    });
  });

  group('trader detail model', () {
    test('stock and restock baseline are separate lists', () {
      final detail = TraderDetail.fromJson({
        'index': 5,
        'uniqueName': 'OC_STT_Fisk_311',
        'ore': 50,
        'traded': true,
        'items': [
          {'path': kTraderOrePath, 'id': 'ItMi_Orenugget', 'count': 50},
        ],
        'defaultItems': [
          {'path': kTraderOrePath, 'id': 'ItMi_Orenugget', 'count': 96},
          {
            'path': '/Script/Angelscript.ItFo_Loaf',
            'id': 'ItFo_Loaf',
            'count': 3,
          },
        ],
        'generatedEvents': ['OnWorldStart'],
        'hasItemsByDifficulty': false,
      });
      expect(detail.stock(TraderStockMap.current), hasLength(1));
      // The baseline diverges in BOTH values and key set — it is not a mirror.
      expect(detail.stock(TraderStockMap.base), hasLength(2));
      expect(detail.stock(TraderStockMap.base).first.count, 96);
      expect(detail.items.first.isOre, isTrue);
      expect(detail.summary.index, 5);
      expect(detail.totalSecondsPath, isNull);
    });

    test('an uncatalogued class is flagged rather than silently editable', () {
      final detail = TraderDetail.fromJson({
        'index': 0,
        'items': [
          {
            'path': '/Script/Angelscript.ItXx_Mystery',
            'id': 'ItXx_Mystery',
            'count': 1,
            'unknownItem': true,
          },
        ],
      });
      expect(detail.items.single.unknownItem, isTrue);
    });
  });

  group('trader restock forecast', () {
    test('known Resources levels map to shipped intervals only', () {
      expect(traderRestockDays('Novice'), 2);
      expect(traderRestockDays('Gothic'), 3);
      expect(traderRestockDays('Hard'), 5);
      expect(traderRestockDays('Modded'), isNull);
    });

    test('distinguishes calendar and elapsed forecast boundaries', () {
      const activity = 22 * secondsPerDay + 21 * secondsPerHour;
      final before = TraderRestockTiming(
        activitySeconds: activity.toDouble(),
        worldSeconds: (25 * secondsPerDay - 1).toDouble(),
        intervalDays: 3,
      );
      expect(before.calendarBoundarySeconds, 25 * secondsPerDay);
      expect(
        before.elapsedBoundarySeconds,
        25 * secondsPerDay + 21 * secondsPerHour,
      );
      expect(before.state, TraderRestockForecastState.beforeWindow);

      final uncertain = TraderRestockTiming(
        activitySeconds: activity.toDouble(),
        worldSeconds: (25 * secondsPerDay).toDouble(),
        intervalDays: 3,
      );
      expect(uncertain.state, TraderRestockForecastState.boundaryOnly);

      final conservative = TraderRestockTiming(
        activitySeconds: activity.toDouble(),
        worldSeconds: (25 * secondsPerDay + 21 * secondsPerHour).toDouble(),
        intervalDays: 3,
      );
      expect(conservative.state, TraderRestockForecastState.eligibleBoth);
    });

    test('sentinel, future clock and invalid values are explicit states', () {
      expect(
        const TraderRestockTiming(
          activitySeconds: kTraderNeverActiveSeconds,
          worldSeconds: 500000,
          intervalDays: 3,
        ).state,
        TraderRestockForecastState.neverActive,
      );
      expect(
        const TraderRestockTiming(
          activitySeconds: 600000,
          worldSeconds: 500000,
          intervalDays: 3,
        ).state,
        TraderRestockForecastState.clockAhead,
      );
      expect(
        const TraderRestockTiming(
          activitySeconds: double.nan,
          worldSeconds: 500000,
          intervalDays: 3,
        ).state,
        TraderRestockForecastState.unavailable,
      );
    });

    test('make due is conservative under both interpretations', () {
      const world = 10 * secondsPerDay + 1234.0;
      const timing = TraderRestockTiming(
        activitySeconds: 9,
        worldSeconds: world,
        intervalDays: 3,
      );
      final moved = timing.makeDueActivitySeconds!;
      final after = TraderRestockTiming(
        activitySeconds: moved,
        worldSeconds: world,
        intervalDays: 3,
      );
      expect(after.state, TraderRestockForecastState.eligibleBoth);
      expect(after.elapsedBoundarySeconds, lessThan(world));
    });
  });

  group('trader edit conflicts', () {
    test('a trader edit and an m_Traders splice conflict either way', () {
      const traderEdit = TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 7,
        map: TraderStockMap.current,
        path: kTraderOrePath,
        count: 5,
      );
      final trader = traderEdit.toEdit();
      final splice = arrayRemoveOnTraders();
      expect(editsRewriteSameTarget(splice, trader), isTrue);
      expect(editsRewriteSameTarget(trader, splice), isTrue);
    });

    test('the conflict is detected as a pair, in either order', () {
      final trader = const TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 7,
        map: TraderStockMap.current,
        path: kTraderOrePath,
        count: 5,
      ).toEdit();
      final splice = arrayRemoveOnTraders();
      expect(traderArrayConflict([trader, splice]), isNotNull);
      expect(traderArrayConflict([splice, trader]), isNotNull);
      expect(traderArrayConflict([trader]), isNull);
      expect(traderArrayConflict([splice]), isNull);
    });

    test('an edit inside a trader row is not a renumbering splice', () {
      // Only an array operation ON m_Traders moves its rows. A value under one
      // row, or a container edit inside one, runs through the array without
      // touching its length — refusing those would block safe pairs.
      final trader = const TraderStockEdit(
        kind: TraderEditKind.setStock,
        index: 7,
        map: TraderStockMap.current,
        path: kTraderOrePath,
        count: 5,
      ).toEdit();
      final insideRow = {
        'path': 'private.typed.setValue',
        'value': {
          'path': [
            'm_GenericData',
            '{GameStateDataBase}',
            'm_Traders',
            '[7]',
            'm_TotalSeconds',
          ],
          'value': '1.0',
        },
      };
      final containerInsideRow = {
        'path': 'private.typed.arrayRemove',
        'value': {
          'path': [
            'm_GenericData',
            '{GameStateDataBase}',
            'm_Traders',
            '[7]',
            'm_GeneratedEvents',
          ],
          'index': 0,
        },
      };
      expect(traderArrayConflict([trader, insideRow]), isNull);
      expect(traderArrayConflict([trader, containerInsideRow]), isNull);
      expect(editsRewriteSameTarget(insideRow, trader), isFalse);
      expect(editsRewriteSameTarget(containerInsideRow, trader), isFalse);
    });

    test('an unrelated array splice does not conflict', () {
      const traderEdit = TraderStockEdit(
        kind: TraderEditKind.addItem,
        index: 7,
        map: TraderStockMap.current,
        path: '/Script/Angelscript.ItFo_Cheese',
        count: 1,
      );
      final elsewhere = {
        'path': 'private.typed.arrayRemove',
        'value': {
          'path': ['m_GenericData', '{Story}', 'SomethingElse'],
          'index': 0,
        },
      };
      expect(editsRewriteSameTarget(elsewhere, traderEdit.toEdit()), isFalse);
    });
  });

  group('Handel tab', () {
    // The ore card, addressed through its own title: the first Card on the page
    // is the price note, and the first TextField is the character search.
    // By key, not by the nearest Card: the whole panel sits on one sheet now,
    // so a Card ancestor is every row in the tab.
    final oreCard = find.byKey(const ValueKey('trader-ore-card'));
    Future<void> pumpApp(
      WidgetTester tester,
      GoresaveCoreService core, {
      bool showObjectIds = false,
      String appLocale = 'en',
      Map<String, Map<String, String>>? locCatalog,
      Size surface = const Size(1400, 1000),
    }) async {
      await tester.binding.setSurfaceSize(surface);
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            coreServiceProvider.overrideWithValue(core),
            editorSettingsStoreProvider.overrideWithValue(
              const NoopEditorSettingsStore(),
            ),
            uiSettingsStoreProvider.overrideWithValue(
              TestUiSettingsStore(
                showObjectIds: showObjectIds,
                appLocale: appLocale,
              ),
            ),
            if (locCatalog != null)
              locCatalogProvider.overrideWith((ref) async => locCatalog),
          ],
          child: const GoresaveApp(),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('the player is not a merchant and gets a clean empty state', (
      tester,
    ) async {
      final core = _TraderCoreService();
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.text('This character does not trade.'), findsOneWidget);
      // A non-merchant must not cost a detail round trip.
      expect(
        core.requests.where((r) => r.command == 'private.traders.detail'),
        isEmpty,
      );
    });

    testWidgets('shows the forecast and queues all timestamp actions safely', (
      tester,
    ) async {
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.text('Restock timer'), findsOneWidget);
      expect(find.text('Last merchant activity'), findsOneWidget);
      expect(find.textContaining('Day 13 · 00:00:00'), findsWidgets);
      expect(
        find.byTooltip('Not expected before Day 13 · 00:00:00.'),
        findsNothing,
      );
      expect(find.text('Status'), findsOneWidget);
      expect(find.text('Waiting for restock'), findsOneWidget);
      final restockCard = tester.getRect(
        find.byKey(const ValueKey('trader-restock-card')),
      );
      final statusRow = tester.getRect(
        find.byKey(const ValueKey('trader-restock-status')),
      );
      final statusValue = tester.getRect(find.text('Waiting for restock'));
      final firstFact = tester.getRect(find.text('Last merchant activity'));
      final customButton = tester.getRect(
        find.byKey(const ValueKey('trader-restock-custom')),
      );
      expect(statusRow.top, lessThan(firstFact.top));
      expect((statusValue.right - statusRow.right).abs(), lessThan(1));
      expect(
        (customButton.right - (restockCard.right - 12)).abs(),
        lessThan(1),
      );
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('trader-restock-status')),
          matching: find.byIcon(Icons.hourglass_bottom),
        ),
        findsOneWidget,
      );

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      final setNow = find.byKey(const ValueKey('trader-restock-set-now'));
      await tester.ensureVisible(setNow);
      await tester.tap(setNow);
      await tester.pumpAndSettle();
      var pending = notifier.pendingEditFor('traders:7:activityTime')!;
      expect((pending.edits.single['value'] as Map)['value'], 1000000.0);
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // A second action replaces the same pending timestamp and chooses a value
      // old enough for both the calendar-boundary and elapsed-time readings.
      final makeDue = find.byKey(const ValueKey('trader-restock-make-due'));
      await tester.ensureVisible(makeDue);
      await tester.tap(makeDue);
      await tester.pumpAndSettle();
      pending = notifier.pendingEditFor('traders:7:activityTime')!;
      expect((pending.edits.single['value'] as Map)['value'], 740799.0);
      expect(pending.edits, hasLength(1));
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('trader-restock-status')),
          matching: find.byIcon(Icons.check_circle_outline),
        ),
        findsOneWidget,
      );
      expect(find.text('Ready for restock'), findsOneWidget);
      expect(
        find.byTooltip(
          "Move the merchant's last activity far enough back that restocking should be due now.",
        ),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const ValueKey('trader-restock-revert')));
      await tester.pumpAndSettle();
      expect(notifier.pendingEditFor('traders:7:activityTime'), isNull);

      final custom = find.byKey(const ValueKey('trader-restock-custom'));
      await tester.ensureVisible(custom);
      await tester.tap(custom);
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('game-time-dialog')), findsOneWidget);
      // The loaded activity has a sub-second fraction. Confirming the whole-
      // second clock shown by the dialog must preserve it as a true no-op.
      await tester.tap(find.byKey(const ValueKey('confirm-game-time')));
      await tester.pumpAndSettle();
      expect(notifier.pendingEditFor('traders:7:activityTime'), isNull);

      await tester.tap(custom);
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const ValueKey('game-time-day-field')),
        '14',
      );
      await tester.enterText(
        find.byKey(const ValueKey('game-time-hour-field')),
        '1',
      );
      await tester.enterText(
        find.byKey(const ValueKey('game-time-minute-field')),
        '2',
      );
      await tester.enterText(
        find.byKey(const ValueKey('game-time-second-field')),
        '3',
      );
      await tester.tap(find.byKey(const ValueKey('confirm-game-time')));
      await tester.pumpAndSettle();
      pending = notifier.pendingEditFor('traders:7:activityTime')!;
      expect(
        (pending.edits.single['value'] as Map)['value'],
        14 * secondsPerDay + 3600 + 120 + 3,
      );
    });

    testWidgets('only current stock is shown and remains editable', (
      tester,
    ) async {
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      const loaf = '/Script/Angelscript.ItFo_Loaf';
      await tester.tap(find.text('Food (2)'));
      await tester.pumpAndSettle();
      final currentRow = find.byKey(
        const ValueKey((TraderStockMap.current, loaf)),
      );
      expect(
        tester
            .widget<TextField>(
              find.descendant(of: currentRow, matching: find.byType(TextField)),
            )
            .enabled,
        isTrue,
      );

      expect(find.text('Restock baseline'), findsNothing);
      expect(find.byType(SegmentedButton<TraderStockMap>), findsNothing);
      expect(find.text('Add item'), findsOneWidget);
      final baseRow = find.byKey(const ValueKey((TraderStockMap.base, loaf)));
      expect(baseRow, findsNothing);
    });

    testWidgets('replacing a pending world clock refreshes the forecast', (
      tester,
    ) async {
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      PendingSaveEdit worldClock(double seconds) => PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {'value': seconds},
          },
        ],
      );

      notifier.setPendingEdit('gameTime', worldClock(1000000));
      await tester.pumpAndSettle();
      expect(find.text('Waiting for restock'), findsOneWidget);

      // Replacing the same key keeps pendingEdits.length at one.
      notifier.setPendingEdit('gameTime', worldClock(2000000));
      await tester.pumpAndSettle();
      expect(find.text('Ready for restock'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('trader-restock-set-now')));
      await tester.pumpAndSettle();
      final activity = notifier.pendingEditFor('traders:7:activityTime')!;
      expect((activity.edits.single['value'] as Map)['value'], 2000000.0);
    });

    testWidgets('restock difficulty follows the app language', (tester) async {
      for (final (resourceClass, expected) in [
        ('Easy', '2 Tage · Novize'),
        ('Hard', '5 Tage · Schwer'),
      ]) {
        await pumpApp(
          tester,
          _DifficultyTraderCoreService(resourceClass),
          appLocale: 'de',
        );
        await tester.tap(find.widgetWithText(Tab, 'Charaktere'));
        await tester.pumpAndSettle();
        await tester.tap(detailTab('Handel'));
        await tester.pumpAndSettle();

        expect(find.text(expected), findsOneWidget);
        expect(find.textContaining('Novice'), findsNothing);
        expect(find.textContaining('Hard'), findsNothing);
      }
    });

    testWidgets('custom time can initialize a never-active merchant', (
      tester,
    ) async {
      final core = _TraderCoreService(
        playerIsTrader: true,
        activitySeconds: kTraderNeverActiveSeconds,
      );
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      final custom = find.byKey(const ValueKey('trader-restock-custom'));
      await tester.ensureVisible(custom);
      await tester.tap(custom);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('confirm-game-time')));
      await tester.pumpAndSettle();

      final pending = notifier.pendingEditFor('traders:7:activityTime')!;
      expect((pending.edits.single['value'] as Map)['value'], 1000000.0);
    });

    testWidgets('ore and restock cards share a row when space allows', (
      tester,
    ) async {
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final ore = tester.getRect(find.byKey(const ValueKey('trader-ore-card')));
      final restock = tester.getRect(
        find.byKey(const ValueKey('trader-restock-card')),
      );
      expect((ore.top - restock.top).abs(), lessThan(1));
      expect(restock.left, greaterThan(ore.right));
      expect((ore.height - restock.height).abs(), lessThan(1));
      expect(
        find.text(
          'Starting value — the amount in the trade screen can differ.',
        ),
        findsOneWidget,
      );
      expect(
        find.byTooltip(
          'The in-game figure differs: on load the game adds what accrued since his last trade — he sells surplus goods and restocks from it. This number is the starting point, not what the trade screen shows.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('trader add dialog repeats the removal warning', (
      tester,
    ) async {
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final notes = find.byKey(const ValueKey('trader-notes-card'));
      expect(
        find.descendant(
          of: notes,
          matching: find.text(
            "Changes to the merchant's inventory last only until the next restock.",
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: notes,
          matching: find.byIcon(Icons.warning_amber_outlined),
        ),
        findsOneWidget,
      );

      await tester.tap(find.widgetWithText(OutlinedButton, 'Add item'));
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      final dialogWarning = find.byKey(const ValueKey('add-item-warning'));
      expect(dialogWarning, findsOneWidget);
      expect(
        tester.widget<Text>(dialogWarning).data,
        "Changes to the merchant's inventory last only until the next restock.",
      );
    });

    testWidgets('sentinel and missing clock/path stay explicit and read-only', (
      tester,
    ) async {
      await pumpApp(
        tester,
        _TraderCoreService(
          playerIsTrader: true,
          activitySeconds: kTraderNeverActiveSeconds,
          worldSeconds: null,
          hasActivityPath: false,
        ),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(
        find.byTooltip('No merchant activity has been recorded yet.'),
        findsNothing,
      );
      expect(find.text('No activity'), findsOneWidget);
      expect(find.text('Never'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('trader-restock-status')),
          matching: find.byIcon(Icons.history_toggle_off),
        ),
        findsOneWidget,
      );
      final custom = tester.widget<IconButton>(
        find.byKey(const ValueKey('trader-restock-custom')),
      );
      expect(custom.onPressed, isNull);
    });

    testWidgets('a queued addition is visible before the save', (tester) async {
      // Regression: a new line has no counterpart in the loaded stock, so
      // without rendering the queued edit it stayed invisible until the next
      // save — unlike the inventory, which shows its queued additions.
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.add_circle_outline), findsNothing);
      expect(find.text('ItFo_Cheese'), findsNothing);

      // Queue the add the way the panel does, then let it rebuild.
      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      notifier.setTraderStockEdit(
        const TraderStockEdit(
          kind: TraderEditKind.addItem,
          index: 7,
          map: TraderStockMap.current,
          path: '/Script/Angelscript.ItFo_Cheese',
          count: 9,
        ),
      );
      await tester.pumpAndSettle();

      // Shown as a banner BESIDE the list, the way the inventory shows its own
      // queued additions — not as a row among the saved lines, which would
      // claim a state the save does not have.
      expect(find.byType(PendingStructuralRow), findsOneWidget);
      expect(find.text('ItFo_Cheese'), findsOneWidget);
      expect(find.text('×9 — pending add (not yet saved)'), findsOneWidget);
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Cancelling it takes the banner away again.
      await tester.tap(
        find.descendant(
          of: find.byType(PendingStructuralRow),
          matching: find.byIcon(Icons.close),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(PendingStructuralRow), findsNothing);
      expect(find.text('ItFo_Cheese'), findsNothing);
    });

    testWidgets('a save re-reads the stock instead of showing stale rows', (
      tester,
    ) async {
      // Regression: the tab is kept alive, and the panel only reloaded when the
      // merchant or the save path changed — neither of which a save does. The
      // reload key carries the inspection so a save re-reads.
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final before = core.requests
          .where((r) => r.command == 'private.traders.detail')
          .length;
      expect(before, greaterThan(0));

      // Queue something, save, and let the trailing refresh run.
      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      notifier.setTraderStockEdit(
        const TraderStockEdit(
          kind: TraderEditKind.setStock,
          index: 7,
          map: TraderStockMap.current,
          path: kTraderOrePath,
          count: 4242,
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
      await tester.pumpAndSettle();

      expect(
        core.requests.where((r) => r.command == 'write_save'),
        isNotEmpty,
        reason: 'the save must actually go out',
      );
      expect(
        core.requests
            .where((r) => r.command == 'private.traders.detail')
            .length,
        greaterThan(before),
        reason: 'the panel must re-read after the save',
      );
    });

    testWidgets('two additions are split into separate writes', (tester) async {
      // Regression: an insert changes how many entries a map holds, and every
      // trader edit is addressed by an index — so the core refuses two of them
      // in one write. The app has to split them itself; when its classification
      // did not mirror the core's, saving an addition to the restock baseline
      // beside one to the live stock failed outright.
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      for (final map in TraderStockMap.values) {
        notifier.setTraderStockEdit(
          TraderStockEdit(
            kind: TraderEditKind.addItem,
            index: 7,
            map: map,
            path: '/Script/Angelscript.ItFo_Cheese',
            count: 2,
          ),
        );
      }
      await tester.pumpAndSettle();
      // Only the selected map is on screen, so only its banner shows — but both
      // edits are queued and both must reach the core.
      expect(find.byType(PendingStructuralRow), findsOneWidget);

      await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
      await tester.pumpAndSettle();

      final writes = core.requests
          .where((r) => r.command == 'write_save')
          .toList();
      expect(writes, hasLength(2), reason: 'one write per insert');
      for (final w in writes) {
        expect((w.payload['edits'] as List), hasLength(1));
      }
    });

    testWidgets(
      'a queued addition prints its class path only when ids are on',
      (tester) async {
        Future<void> queueAdd(WidgetTester tester) async {
          await tester.tap(find.widgetWithText(Tab, 'Characters'));
          await tester.pumpAndSettle();
          await tester.tap(detailTab('Trade'));
          await tester.pumpAndSettle();
          ProviderScope.containerOf(tester.element(find.byType(Scaffold).first))
              .read(editorProvider.notifier)
              .setTraderStockEdit(
                const TraderStockEdit(
                  kind: TraderEditKind.addItem,
                  index: 7,
                  map: TraderStockMap.current,
                  path: '/Script/Angelscript.ItFo_Cheese',
                  count: 1,
                ),
              );
          await tester.pumpAndSettle();
        }

        await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
        await queueAdd(tester);
        expect(find.text('/Script/Angelscript.ItFo_Cheese'), findsNothing);

        await pumpApp(
          tester,
          _TraderCoreService(playerIsTrader: true),
          showObjectIds: true,
        );
        await queueAdd(tester);
        expect(find.text('/Script/Angelscript.ItFo_Cheese'), findsOneWidget);
      },
    );

    testWidgets('stock is grouped by category and the sidebar filters it', (
      tester,
    ) async {
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      // One sidebar entry per populated category, counted. Ore is not among
      // them: in the live stock it has its own card, so it leaves the list.
      expect(find.text('Melee weapons (1)'), findsOneWidget);
      expect(find.text('Ranged weapons (1)'), findsOneWidget);
      expect(find.text('Food (2)'), findsOneWidget);
      expect(find.textContaining('Miscellaneous'), findsNothing);

      // The list shows the selected category only — melee comes first.
      expect(find.text('ItMw_1H_Sword_01'), findsOneWidget);
      expect(find.text('ItFo_Loaf'), findsNothing);

      await tester.tap(find.text('Food (2)'));
      await tester.pumpAndSettle();
      expect(find.text('ItFo_Loaf'), findsOneWidget);
      expect(find.text('ItFo_Apple'), findsOneWidget);
      expect(find.text('ItMw_1H_Sword_01'), findsNothing);
    });

    testWidgets('a category sorts by the localized name, not the class id', (
      tester,
    ) async {
      // By class id ItFo_Apple leads ItFo_Loaf; by name "Brot" leads "Apfel"
      // — this is the order the user reads, so it is the order that counts.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true),
        // The real loader lowercases its keys; the override supplies them so.
        locCatalog: const {
          'itfo_apple': {'english': 'Zucchini'},
          'itfo_loaf': {'english': 'Bread'},
        },
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Food (2)'));
      await tester.pumpAndSettle();

      final bread = tester.getTopLeft(find.text('Bread')).dy;
      final zucchini = tester.getTopLeft(find.text('Zucchini')).dy;
      expect(bread, lessThan(zucchini));
    });

    testWidgets('the compact list sorts by localized name as well', (
      tester,
    ) async {
      // Under 600px the pane drops the sidebar and lists every line at once.
      // That list came straight from the core, which orders by class id, so the
      // one view without categories to lean on was also the one out of order.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true),
        surface: const Size(1200, 900),
        locCatalog: const {
          'itfo_apple': {'english': 'Zucchini'},
          'itfo_loaf': {'english': 'Bread'},
          'itmw_1h_sword_01': {'english': 'Axe'},
        },
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.byType(SidebarTile), findsNothing, reason: 'compact pane');
      final axe = tester.getTopLeft(find.text('Axe')).dy;
      final bread = tester.getTopLeft(find.text('Bread')).dy;
      final zucchini = tester.getTopLeft(find.text('Zucchini')).dy;
      expect(axe, lessThan(bread));
      expect(bread, lessThan(zucchini));
    });

    testWidgets('a count field never keeps the previous line value', (
      tester,
    ) async {
      // Regression: the field only refreshed on a pending change, and rows had
      // no key — so switching category reused one line's field State for the
      // next, showing a stale count that a submit would then write to the wrong
      // item.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      List<String> fieldTexts() => tester
          .widgetList<TextField>(find.byType(TextField))
          .map((f) => f.controller?.text ?? '')
          .toList();

      // Melee holds one sword, count 1. The ore card's own field shows 55.
      await tester.tap(find.text('Melee weapons (1)'));
      await tester.pumpAndSettle();
      expect(fieldTexts(), containsAll(<String>['55', '1']));

      // Ranged holds one arrow stack of 18 at the same list position.
      await tester.tap(find.text('Ranged weapons (1)'));
      await tester.pumpAndSettle();
      expect(fieldTexts(), containsAll(<String>['55', '18']));
      expect(
        fieldTexts().where((t) => t == '1'),
        isEmpty,
        reason: 'the sword count must not survive the category switch',
      );
    });

    testWidgets('the ore line can be dropped, not just counted down', (
      tester,
    ) async {
      // A merchant with no ore line is a state the game itself produces, and
      // setStock refuses 0 — so without a delete on the ore card there would be
      // no way to ask for it at all.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      );
      await tester.tap(
        find.descendant(
          of: oreCard,
          matching: find.byIcon(Icons.delete_outline),
        ),
      );
      await tester.pumpAndSettle();

      final queued = container
          .read(editorProvider)
          .pendingEdits
          .values
          .expand((p) => p.edits)
          .where((e) => e['path'] == 'private.traders.removeItem')
          .toList();
      expect(queued, hasLength(1));
      expect((queued.single['value'] as Map)['path'], kTraderOrePath);
      // And it is announced the way every other queued removal is.
      expect(find.byType(PendingStructuralRow), findsOneWidget);
    });

    testWidgets('a count beyond the i32 the core stores is refused', (
      tester,
    ) async {
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final field = find.descendant(
        of: oreCard,
        matching: find.byType(TextField),
      );
      await tester.enterText(field, '2147483648');
      await tester.pump();

      // Refused in place — an error under the field and nothing queued — rather
      // than the save failing later on the core's bound. The text stays as
      // typed: snapping it back under the cursor would fight the typing.
      expect(find.textContaining('2147483647'), findsOneWidget);
      expect(
        ProviderScope.containerOf(
          tester.element(find.byType(Scaffold).first),
        ).read(editorProvider).pendingEdits,
        isEmpty,
      );
    });

    testWidgets('unmodelled per-difficulty stock turns the panel read-only', (
      tester,
    ) async {
      // The edits reach only m_Items and m_DefaultItems. A save carrying stock
      // the editor does not model would take an edit, report success, and leave
      // that other stock standing — so it says so and offers nothing.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true, hasItemsByDifficulty: true),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.textContaining('per-difficulty stock'), findsOneWidget);
      expect(find.widgetWithText(OutlinedButton, 'Add item'), findsNothing);
      expect(find.byIcon(Icons.delete_outline), findsNothing);
      final oreField = tester.widget<TextField>(
        find.descendant(of: oreCard, matching: find.byType(TextField)),
      );
      expect(oreField.enabled, isFalse);
    });

    testWidgets('many queued changes scroll instead of overflowing', (
      tester,
    ) async {
      // Replacing most of a merchant's stock queues enough banners to push the
      // browser off screen; unbounded, they overflowed and took the cancel
      // buttons with them.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      for (var i = 0; i < 25; i++) {
        notifier.setTraderStockEdit(
          TraderStockEdit(
            kind: TraderEditKind.addItem,
            index: 7,
            map: TraderStockMap.current,
            path: '/Script/Angelscript.ItFo_Filler_$i',
            count: 1,
          ),
        );
      }
      await tester.pumpAndSettle();

      expect(find.byType(PendingStructuralRow), findsNWidgets(25));
      expect(tester.takeException(), isNull, reason: 'no RenderFlex overflow');
      // The stock browser is still there below them.
      expect(find.byType(SidebarTile), findsWidgets);
    });

    testWidgets('the smallest supported window bounds the banners', (
      tester,
    ) async {
      // 960x600 is the minimum the app itself allows (main.dart), and it is the
      // size that matters: a cap measured against a constant ignores the header
      // above and the list below, so the column overflowed and the browser
      // collapsed to nothing. Testing only a roomy surface hid that.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true),
        surface: const Size(960, 600),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final ore = tester.getRect(find.byKey(const ValueKey('trader-ore-card')));
      final restock = tester.getRect(
        find.byKey(const ValueKey('trader-restock-card')),
      );
      expect(restock.top, greaterThanOrEqualTo(ore.bottom));

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      for (var i = 0; i < 25; i++) {
        notifier.setTraderStockEdit(
          TraderStockEdit(
            kind: TraderEditKind.addItem,
            index: 7,
            map: TraderStockMap.current,
            path: '/Script/Angelscript.ItFo_Short_$i',
            count: 1,
          ),
        );
      }
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull, reason: 'no RenderFlex overflow');
    });

    testWidgets('a merchant with no in-world actor is not listed', (
      tester,
    ) async {
      // A trader row is keyed by name, so a row with no spawned actor could in
      // principle own one — but no real save has ever carried such a row, and
      // characters that never spawned are out of the list entirely.
      await pumpApp(tester, _TraderCoreService(orphanMerchant: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();

      // Scoped to the list: the Trade sub-tab carries the same icon, so an
      // unscoped finder would pass either way.
      expect(
        find.descendant(
          of: find.byType(CharacterMasterList),
          matching: find.byIcon(Icons.storefront_outlined),
        ),
        findsNothing,
      );
    });

    testWidgets('typing a count queues it without leaving the field', (
      tester,
    ) async {
      // The field used to queue only on Enter or a tap outside, so a typed
      // amount left Save disabled and could be overwritten by a rebuild before
      // it was ever registered. The inventory queues per keystroke; so does this.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final field = find.descendant(
        of: oreCard,
        matching: find.byType(TextField),
      );
      await tester.enterText(field, '4242');
      await tester.pump();

      final pending =
          ProviderScope.containerOf(tester.element(find.byType(Scaffold).first))
              .read(editorProvider)
              .pendingEdits
              .values
              .expand((p) => p.edits)
              .toList();
      expect(pending, hasLength(1));
      expect((pending.single['value'] as Map)['count'], 4242);

      // Back to the saved value withdraws it again, no Enter needed.
      await tester.enterText(field, '55');
      await tester.pump();
      expect(
        ProviderScope.containerOf(
          tester.element(find.byType(Scaffold).first),
        ).read(editorProvider).pendingEdits,
        isEmpty,
      );
    });

    testWidgets('clearing an edited count leaves the field empty', (
      tester,
    ) async {
      // Backspacing through a queued count clears the pending value, and the
      // sync used to restore the saved one straight back under the cursor — so
      // the field could never be emptied to type a fresh number.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final field = find.descendant(
        of: oreCard,
        matching: find.byType(TextField),
      );
      await tester.tap(field);
      await tester.pump();
      await tester.enterText(field, '12');
      await tester.pump();
      await tester.enterText(field, '');
      await tester.pump();

      expect(tester.widget<TextField>(field).controller?.text, isEmpty);
      // And an empty field queues nothing rather than a zero.
      expect(
        ProviderScope.containerOf(
          tester.element(find.byType(Scaffold).first),
        ).read(editorProvider).pendingEdits,
        isEmpty,
      );
    });

    testWidgets('leaving a cleared count field restores the saved value', (
      tester,
    ) async {
      // The focus guard keeps the sync off while typing, so without a matching
      // reset on blur an emptied or refused entry stayed on screen afterwards —
      // with no pending edit and no revert control to explain it.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final field = find.descendant(
        of: oreCard,
        matching: find.byType(TextField),
      );
      await tester.tap(field);
      await tester.pump();
      await tester.enterText(field, '');
      await tester.pump();
      expect(tester.widget<TextField>(field).controller?.text, isEmpty);

      // Focus moves away: the field goes back to what the save holds.
      FocusManager.instance.primaryFocus?.unfocus();
      await tester.pumpAndSettle();
      expect(tester.widget<TextField>(field).controller?.text, '55');
      expect(
        ProviderScope.containerOf(
          tester.element(find.byType(Scaffold).first),
        ).read(editorProvider).pendingEdits,
        isEmpty,
      );
    });

    testWidgets('the save aborts rather than splitting a trader/array pair', (
      tester,
    ) async {
      // Splitting them into two writes would slip past the core's refusal —
      // each write is fine on its own — and report both as committed while the
      // array operation renumbers the row the trade change was addressed by.
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final notifier = ProviderScope.containerOf(
        tester.element(find.byType(Scaffold).first),
      ).read(editorProvider.notifier);
      notifier.setTraderStockEdit(
        const TraderStockEdit(
          kind: TraderEditKind.setStock,
          index: 7,
          map: TraderStockMap.current,
          path: kTraderOrePath,
          count: 5,
        ),
      );
      notifier.setPendingEdit(
        'all-data:m_Traders',
        PendingSaveEdit(edits: [arrayRemoveOnTraders()]),
      );
      await tester.pumpAndSettle();

      final saved = await notifier.saveAllPending();
      await tester.pumpAndSettle();

      expect(saved, isFalse);
      expect(
        core.requests.where((r) => r.command == 'write_save'),
        isEmpty,
        reason: 'nothing may reach the save while the pair is queued',
      );
    });

    testWidgets('the in-field undo restores the text while still focused', (
      tester,
    ) async {
      // The sync that would otherwise restore it is off while the field has
      // focus, so the undo has to put the text back itself — or the discarded
      // count sits there with nothing queued behind it and Save disabled.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      final field = find.descendant(
        of: oreCard,
        matching: find.byType(TextField),
      );
      await tester.tap(field);
      await tester.pump();
      await tester.enterText(field, '4242');
      await tester.pump();
      expect(
        find.descendant(of: oreCard, matching: find.byIcon(Icons.undo)),
        findsOneWidget,
      );

      final undo = find.descendant(
        of: oreCard,
        matching: find.byIcon(Icons.undo),
      );
      await tester.ensureVisible(undo);
      await tester.tap(undo);
      await tester.pump();

      expect(tester.widget<TextField>(field).controller?.text, '55');
      expect(
        ProviderScope.containerOf(
          tester.element(find.byType(Scaffold).first),
        ).read(editorProvider).pendingEdits,
        isEmpty,
      );
    });

    testWidgets('a record missing a stock list is read-only', (tester) async {
      // An omitted map reads as an empty one, so Add looked available and the
      // save would then fail: the structural appliers resolve the property and
      // cannot create it.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true, stockMapsPresent: false),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(
        find.textContaining('does not support and cannot write'),
        findsOneWidget,
      );
      expect(find.widgetWithText(OutlinedButton, 'Add item'), findsNothing);
      expect(find.byIcon(Icons.delete_outline), findsNothing);
    });

    testWidgets('a merchant holding only ore is not called empty', (
      tester,
    ) async {
      // The ore is lifted out of the live stock into its own card, so the
      // filtered list is empty while the map is not. Reading "nothing in stock"
      // off the filtered list contradicted both the line count and the purse
      // shown right above it.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true, oreOnly: true),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.text('Nothing in stock.'), findsNothing);
      expect(find.text('1 lines'), findsOneWidget);
      expect(find.text('Ore (purchasing power)'), findsOneWidget);
    });

    testWidgets('the read-only note waits until nothing at all is writable', (
      tester,
    ) async {
      // A core with no stocked shop drops setStock but still offers addItem, so
      // announcing "read only" beside a working Add button was simply wrong.
      await pumpApp(
        tester,
        _TraderCoreService(
          playerIsTrader: true,
          writable: const ['private.traders.addItem'],
        ),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(OutlinedButton, 'Add item'), findsOneWidget);
      expect(
        find.textContaining('can only read trader data'),
        findsNothing,
        reason: 'Add works, so this core is not read-only',
      );

      // With nothing advertised at all, the note is the honest thing to show.
      await pumpApp(
        tester,
        _TraderCoreService(playerIsTrader: true, writable: const []),
      );
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();
      expect(find.textContaining('can only read trader data'), findsOneWidget);
    });

    testWidgets('a detail pane too narrow for labels shows bare icons', (
      tester,
    ) async {
      // Trade made six icon-and-label tabs out of five. A non-scrollable bar
      // splits its width evenly, and the detail pane is a fraction of the
      // window, so each tab gets far less than a label needs and the labels
      // clip. Below the breakpoint they carry the icon alone.
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();

      // The top-level bar also has six tabs, so pick the one carrying Trade.
      final bar = find.ancestor(
        of: detailTab('Trade'),
        matching: find.byType(TabBar),
      );
      final detail = tester.widget<TabBar>(bar);
      expect(detailTabsCanCarryLabels(tester.getSize(bar).width), isFalse);
      expect(
        detail.tabs.whereType<Tab>().map((t) => t.text),
        everyElement(isNull),
        reason: 'no label to clip',
      );
      // And each one still says what it is.
      expect(detailTab('Trade'), findsOneWidget);
      expect(detailTab('Dialog Knowledge'), findsOneWidget);
    });

    testWidgets('an icon-only tab still names itself to a screen reader', (
      tester,
    ) async {
      // A Tooltip alone lands the name in the node's `tooltip`, which platforms
      // surface as help text; the tab itself then reads out as "Tab 3 of 6".
      final handle = tester.ensureSemantics();
      await pumpApp(tester, _TraderCoreService(playerIsTrader: true));
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();

      final node = tester.getSemantics(detailTab('Trade'));
      expect(node.label, contains('Trade'));
      handle.dispose();
    });

    test('the tab bar takes labels once every tab has room for one', () {
      // 132px is kTabLabelPadding either side plus the longest shipped label.
      expect(detailTabsCanCarryLabels(6 * 132), isTrue);
      expect(detailTabsCanCarryLabels(6 * 132 - 1), isFalse);
    });

    testWidgets('a merchant shows ore, restock timing and current stock', (
      tester,
    ) async {
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Trade'));
      await tester.pumpAndSettle();

      expect(find.text('Ore (purchasing power)'), findsOneWidget);
      expect(find.text('Restock timer'), findsOneWidget);
      expect(find.text('Restock baseline'), findsNothing);
      expect(find.byType(SegmentedButton<TraderStockMap>), findsNothing);
      // The detail is fetched by INDEX, never by the name.
      final detail = core.requests.firstWhere(
        (r) => r.command == 'private.traders.detail',
      );
      expect(detail.payload['index'], 7);
      expect(detail.payload.containsKey('uniqueName'), isFalse);
    });
  });
}

class _RecordedRequest {
  _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// Minimal core fixture: one save, one player, and a trader array whose single
/// real row optionally carries the player's own unique name so the Handel tab
/// can be exercised without inventing a second character.
class _TraderCoreService implements GoresaveCoreService {
  _TraderCoreService({
    this.playerIsTrader = false,
    this.hasItemsByDifficulty = false,
    this.orphanMerchant = false,
    this.stockMapsPresent = true,
    this.oreOnly = false,
    this.writable,
    this.worldSeconds = 1000000,
    this.activitySeconds = 937101.34,
    this.hasActivityPath = true,
  });

  /// Override the advertised command list. The core drops setStock when no shop
  /// holds a line, while still offering addItem.
  final List<String>? writable;
  final double? worldSeconds;
  final double activitySeconds;
  final bool hasActivityPath;

  /// A merchant holding nothing but his ore: the live stock has one line, and
  /// it is the one the ore card takes out of the list.
  final bool oreOnly;

  /// A record missing one of its stock lists. No shipped save has one, which is
  /// why the fixture has to fake it.
  final bool stockMapsPresent;

  /// A knowledge-only row that owns a trader record. It has no spawned actor,
  /// which is exactly why it used to be hidden from the trade panel.
  final bool orphanMerchant;

  final bool playerIsTrader;

  /// Per-difficulty stock, which the editor does not model. Empty in every real
  /// save seen so far, so the fixture has to fake it.
  final bool hasItemsByDifficulty;
  final requests = <_RecordedRequest>[];

  /// Whatever unique name the app resolved for the pinned player row. The
  /// fixture answers `private.traders.list` with this name so the panel's join
  /// succeeds regardless of how the player row is keyed.
  static const String _playerUniqueName = 'Hero';

  @override
  String get description => 'trader-fake-core';

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
                'playerSaveName': 'Save',
                'chapterId': 1,
                'autoSave': true,
                'slotName': 'G1R-001',
              },
            ],
            'profiles': <Object?>[],
            'activeProfileId': null,
          },
        };
      case 'inspect_save':
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 914367,
            'sha1': 'abc',
            'public': {'slotName': 'G1R-001', 'playerSaveName': 'Save'},
            'private': {
              'status': 'decoded',
              'preview': false,
              'decompressedSize': 9,
              'typedParse': {'status': 'ok', 'propertyCount': 1, 'maxDepth': 1},
              'player': {
                'saveVersionNumber': 17,
                'playerName': 'Hero',
                'uniqueName': _playerUniqueName,
                'attributes': <Object?>[],
                'writable': <String>[],
              },
              'inventory': {
                'itemStackCount': 0,
                'items': <Object?>[],
                'mainContainerPaths': <String>[],
                'writable': <String>[],
              },
            },
          },
        };
      case 'private.traders.list':
        return {
          'ok': true,
          'data': {
            'traders': [
              {
                'index': 0,
                'uniqueName': 'None',
                'itemCount': 4,
                'defaultItemCount': 4,
                'ore': 75,
                'totalSeconds': -1000,
                'traded': false,
                'generatedEventCount': 1,
                'placeholder': true,
              },
              {
                'index': 7,
                'uniqueName': playerIsTrader
                    ? _playerUniqueName
                    : 'OC_STT_Dexter_329',
                'itemCount': 2,
                'defaultItemCount': 2,
                'ore': 55,
                'totalSeconds': activitySeconds,
                'traded': true,
                'generatedEventCount': 11,
                'placeholder': false,
              },
            ],
            'writable':
                writable ??
                const [
                  'private.traders.addItem',
                  'private.traders.setStock',
                  'private.traders.removeItem',
                ],
          },
        };
      case 'private.traders.detail':
        return {
          'ok': true,
          'data': {
            'index': payload['index'],
            'uniqueName': playerIsTrader
                ? _playerUniqueName
                : 'OC_STT_Dexter_329',
            'itemCount': 2,
            'defaultItemCount': 2,
            'ore': 55,
            'totalSeconds': activitySeconds,
            'traded': true,
            'generatedEventCount': 11,
            'placeholder': false,
            'stockMapsPresent': stockMapsPresent,
            'items': [
              {
                'path': kTraderOrePath,
                'id': 'ItMi_Orenugget',
                'count': 55,
                'unknownItem': false,
              },
              if (!oreOnly) ...[
                {
                  'path': '/Script/Angelscript.ItFo_Loaf',
                  'id': 'ItFo_Loaf',
                  'count': 3,
                  'unknownItem': false,
                },
                {
                  'path': '/Script/Angelscript.ItFo_Apple',
                  'id': 'ItFo_Apple',
                  'count': 7,
                  'unknownItem': false,
                },
                {
                  'path': '/Script/Angelscript.ItMw_1H_Sword_01',
                  'id': 'ItMw_1H_Sword_01',
                  'count': 1,
                  'unknownItem': false,
                },
                {
                  'path': '/Script/Angelscript.ItAm_Arrow',
                  'id': 'ItAm_Arrow',
                  'count': 18,
                  'unknownItem': false,
                },
              ],
            ],
            'defaultItems': [
              {
                'path': kTraderOrePath,
                'id': 'ItMi_Orenugget',
                'count': 64,
                'unknownItem': false,
              },
              {
                // Deliberately unlike the live stock's 3: a row reused across
                // the map switch would keep showing the wrong one.
                'path': '/Script/Angelscript.ItFo_Loaf',
                'id': 'ItFo_Loaf',
                'count': 9,
                'unknownItem': false,
              },
            ],
            'generatedEvents': ['OnWorldStart'],
            if (hasActivityPath)
              'totalSecondsPath': [
                'm_GenericData',
                '{GameStateDataBase}',
                'm_Traders',
                '[7]',
                'm_TotalSeconds',
              ],
            'hasItemsByDifficulty': hasItemsByDifficulty,
          },
        };
      case 'search_typed_properties':
        if (payload['query'] == 'GameTime' && worldSeconds != null) {
          return {
            'ok': true,
            'data': {
              'source': 'private',
              'offset': 0,
              'limit': payload['limit'] ?? 1000,
              'total': 1,
              'count': 1,
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
                  'display': 'GameTime',
                  'type': 'DoubleProperty',
                  'kind': 'scalar',
                  'value': worldSeconds.toString(),
                  'editValue': worldSeconds,
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
            'source': 'private',
            'offset': 0,
            'limit': payload['limit'] ?? 1000,
            'total': 0,
            'count': 0,
            'results': <Object?>[],
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'backups': <Object?>[],
            'companionBackups': <Object?>[],
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
        return {
          'ok': true,
          'data': {
            'total': orphanMerchant ? 1 : 0,
            'characters': orphanMerchant
                ? [
                    {
                      'globalId': null,
                      'uniqueName': 'OC_STT_Fisk_311',
                      'isDead': false,
                      'personalRelationship': null,
                      'hasInventory': false,
                      'hasKnowledge': true,
                      'hasEvents': false,
                      'isTrader': true,
                    },
                  ]
                : <Object?>[],
          },
        };
      case 'write_save':
        return {
          'ok': true,
          'data': {'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1'},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}

class _DifficultyTraderCoreService extends _TraderCoreService {
  _DifficultyTraderCoreService(this.resourceClass)
    : super(playerIsTrader: true);

  final String resourceClass;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final response = await super.execute(command, payload: payload);
    if (command != 'scan_save_dir') return response;

    final data = (response['data'] as Map).cast<String, Object?>();
    final save = ((data['saves'] as List).single as Map)
        .cast<String, Object?>();
    save['persistentProfileId'] = 0;
    data['profiles'] = [
      {
        'profileId': 0,
        'profileName': '0',
        'savedSlots': ['G1R-001'],
        'difficultyPreset': 'DifficultyPreset_Custom',
        'customResourcesSettings': 'ResourcesDifficultySettings_$resourceClass',
      },
    ];
    data['activeProfileId'] = 0;
    return response;
  }
}
