import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/trader_models.dart';
import 'package:goresave/providers/data_providers.dart';

/// The Handel (trade) sub-tab. A merchant's shop is NOT his inventory: it lives
/// in a global array addressed by index, and his ore inside that shop is what he
/// can pay with. These tests pin the three things that are easy to get wrong —
/// index (not name) addressing, "no ore line" being distinct from zero, and a
/// structural add/remove being kept out of the batched edits.
void main() {
  group('trader edit encoding', () {
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

  group('Handel tab', () {
    Future<void> pumpApp(WidgetTester tester, GoresaveCoreService core) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
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
    }

    testWidgets('the player is not a merchant and gets a clean empty state', (
      tester,
    ) async {
      final core = _TraderCoreService();
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Trade'));
      await tester.pumpAndSettle();

      expect(find.text('This character does not trade.'), findsOneWidget);
      // A non-merchant must not cost a detail round trip.
      expect(
        core.requests.where((r) => r.command == 'private.traders.detail'),
        isEmpty,
      );
    });

    testWidgets('a merchant shows his ore and both stock sections', (
      tester,
    ) async {
      final core = _TraderCoreService(playerIsTrader: true);
      await pumpApp(tester, core);
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Trade'));
      await tester.pumpAndSettle();

      expect(find.text('Ore (purchasing power)'), findsOneWidget);
      expect(find.text('Stock'), findsOneWidget);
      expect(find.text('Restock baseline'), findsOneWidget);
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
  _TraderCoreService({this.playerIsTrader = false});

  final bool playerIsTrader;
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
                'totalSeconds': 937101.34,
                'traded': true,
                'generatedEventCount': 11,
                'placeholder': false,
              },
            ],
            'writable': [
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
            'totalSeconds': 937101.34,
            'traded': true,
            'generatedEventCount': 11,
            'placeholder': false,
            'items': [
              {
                'path': kTraderOrePath,
                'id': 'ItMi_Orenugget',
                'count': 55,
                'unknownItem': false,
              },
              {
                'path': '/Script/Angelscript.ItFo_Loaf',
                'id': 'ItFo_Loaf',
                'count': 3,
                'unknownItem': false,
              },
            ],
            'defaultItems': [
              {
                'path': kTraderOrePath,
                'id': 'ItMi_Orenugget',
                'count': 64,
                'unknownItem': false,
              },
              {
                'path': '/Script/Angelscript.ItFo_Loaf',
                'id': 'ItFo_Loaf',
                'count': 3,
                'unknownItem': false,
              },
            ],
            'generatedEvents': ['OnWorldStart'],
            'hasItemsByDifficulty': false,
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
          'data': {'total': 0, 'characters': <Object?>[]},
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
