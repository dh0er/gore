import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/features/editor/ui/inventory_item_visual.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/detail_tabs.dart';
import 'support/ui_settings_test_store.dart';

/// A wolf carries its jaw in its weapon slot the way a mercenary carries a
/// sword, and the game names none of them: the row reads "Natural weapon", a
/// label that exists only in the editor. Searching had to be taught it, and the
/// narrow-pane copy of the row visual had to be taught its glyph.
const _jawId = 'WolfJaw';
const _jawPath = '/Script/Angelscript.$_jawId';
const _cheeseId = 'ItFo_Cheese';
const _cheesePath = '/Script/Angelscript.$_cheeseId';

void main() {
  Future<void> pumpApp(WidgetTester tester, {required Size size}) async {
    await tester.binding.setSurfaceSize(size);
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(_NaturalWeaponCoreService()),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(TestUiSettingsStore()),
          locCatalogProvider.overrideWith(
            (ref) async => const {
              'itfo_cheese': {'english': 'Wheel of Cheese'},
            },
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> openNpcInventory(WidgetTester tester) async {
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Wolf').first);
    await tester.pumpAndSettle();
    // The bundled item stats are read through rootBundle, which needs real
    // async to complete; until they land no row knows what a WolfJaw is.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('searching finds a creature weapon by the label on its row', (
    tester,
  ) async {
    await pumpApp(tester, size: const Size(1400, 1000));
    await openNpcInventory(tester);
    expect(find.text('Natural weapon'), findsOneWidget);

    // The filter matched the loc catalog only, so the one word actually on the
    // row found nothing.
    final search = find.widgetWithText(TextField, 'Filter items');
    await tester.enterText(search, 'natural');
    await tester.pumpAndSettle();
    expect(find.text('Natural weapon'), findsOneWidget);
    expect(find.text('Wheel of Cheese'), findsNothing);
  });

  testWidgets('a stand-in glyph is what the visual draws', (tester) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: InventoryItemVisual(
              itemId: _jawId,
              fallbackGameIcon: gameIconCreature,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      tester.widget<GameIcon>(find.byType(GameIcon)).name,
      gameIconCreature,
    );
  });

  test('every row visual offers the same stand-in', () {
    // Below 360px the leading visual is dropped and a small copy moves into the
    // title row. That copy was left with the category mark, so a jaw wore a
    // question mark on a narrow pane. Both are built from one helper now; this
    // keeps a third copy from being added without it.
    final source = File(
      'lib/features/editor/ui/inventory_detail.dart',
    ).readAsStringSync();
    final constructions = 'InventoryItemVisual('.allMatches(source).length;
    final withStandIn = 'fallbackGameIcon:'.allMatches(source).length;
    expect(constructions, greaterThan(1));
    expect(withStandIn, constructions);
  });
}

class _NaturalWeaponCoreService implements GoresaveCoreService {
  @override
  String get description => 'natural-weapon-fake-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
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
                'playerSaveName': 'Save',
                'chapterId': 1,
                'autoSave': true,
                'slotName': 'G1R-001',
              },
            ],
            'profiles': [
              {
                'profileId': 0,
                'profileName': '0',
                'quickSaveSlots': <String>[],
                'autoSaveSlots': <String>[],
                'savedSlots': ['G1R-001'],
              },
            ],
            'activeProfileId': 0,
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
                'attributes': <Object?>[],
                'writable': <String>[],
              },
              'inventory': {
                'itemStackCount': 0,
                'items': <Object?>[],
                'writable': ['private.inventory.setItemCount'],
              },
            },
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
            'total': 1,
            'characters': [
              {
                'globalId': 'Wolf-A',
                'uniqueName': 'Wolf',
                'isDead': false,
                'hasInventory': true,
                'hasKnowledge': false,
                'hasEvents': false,
              },
            ],
          },
        };
      case 'private.npc.list':
        return {
          'ok': true,
          'data': {
            'total': 1,
            'offset': 0,
            'limit': payload['limit'] ?? 100,
            'count': 1,
            'npcs': [
              {'id': 'Wolf-A', 'name': 'Wolf A'},
            ],
          },
        };
      case 'private.npc.inventory':
        return {
          'ok': true,
          'data': {
            'id': payload['id'],
            'itemStackCount': 2,
            'items': [
              {
                'id': _jawId,
                'path': _jawPath,
                'count': 1,
                'removable': true,
                'slotId': 0,
                'containerType': 'MeleeSlot',
              },
              {
                'id': _cheeseId,
                'path': _cheesePath,
                'count': 3,
                'removable': true,
                'slotId': 0,
                'containerType': 'MainContainer',
              },
            ],
            'mainContainerPaths': [_cheesePath],
            'writable': [
              'private.inventory.setItemCount',
              'private.inventory.addItem',
              'private.inventory.removeItem',
            ],
          },
        };
      case 'private.npc.attributes':
        return {
          'ok': true,
          'data': {'attributes': <Object?>[]},
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
