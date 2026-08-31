import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/detail_tabs.dart';
import 'support/ui_settings_test_store.dart';

/// A wolf carries its jaw in its weapon slot the way a mercenary carries a
/// sword, but nobody can do anything with it: the game names none of them,
/// draws none of them, and never lets the player hold one. The editor leaves
/// them out.
const _jawId = 'WolfJaw';
const _jawPath = '/Script/Angelscript.$_jawId';
const _cheeseId = 'ItFo_Cheese';
const _cheesePath = '/Script/Angelscript.$_cheeseId';

void main() {
  Future<void> pumpApp(WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
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

  Future<void> openNpcInventory(
    WidgetTester tester, [
    String name = 'Wolf',
  ]) async {
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
    await tester.tap(find.text(name).first);
    await tester.pumpAndSettle();
    // The bundled item stats are read through rootBundle, which needs real
    // async to complete; they are what identifies a natural weapon.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('nothing is removable before the stats can tell rows apart', (
    tester,
  ) async {
    // Deliberately WITHOUT the real-async step openNpcInventory takes: the
    // bundled stats have not answered yet, so a creature's jaw cannot be told
    // from anything else. Queueing a removal in that window would disarm the
    // creature.
    await pumpApp(tester);
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Wolf').first);
    await tester.pumpAndSettle();

    // The rows are there — they are simply not removable yet.
    expect(find.text('Wheel of Cheese'), findsOneWidget);
    expect(find.byIcon(Icons.delete_outline), findsNothing);
  });

  testWidgets('a creature whose only item is its weapon carries nothing', (
    tester,
  ) async {
    // A wolf carries ONLY its jaw. Asking the raw list whether there is
    // anything here still said yes, so the pane showed the one message it has
    // for a list that emptied out — "the pending removal hides every item" —
    // with nothing pending at all.
    await pumpApp(tester);
    await openNpcInventory(tester, 'Warg');

    expect(
      find.textContaining('pending removal hides every item'),
      findsNothing,
    );
    expect(find.text(_jawId), findsNothing);
  });

  testWidgets('a creature carries no rows for its own weapon', (tester) async {
    await pumpApp(tester);
    await openNpcInventory(tester);

    expect(find.text('Wheel of Cheese'), findsOneWidget);
    expect(find.text(_jawId), findsNothing);
    // Nor by the label it used to wear, nor as a count of one in the rail.
    expect(find.text('Natural weapon'), findsNothing);
    expect(find.textContaining('(1)'), findsWidgets);
    expect(find.textContaining('(2)'), findsNothing);
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
            'total': 2,
            'characters': [
              {
                'globalId': 'Wolf-A',
                'uniqueName': 'Wolf',
                'isDead': false,
                'hasInventory': true,
                'hasKnowledge': false,
                'hasEvents': false,
              },
              {
                'globalId': 'Warg-B',
                'uniqueName': 'Warg',
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
            'total': 2,
            'offset': 0,
            'limit': payload['limit'] ?? 100,
            'count': 2,
            'npcs': [
              {'id': 'Wolf-A', 'name': 'Wolf A'},
              {'id': 'Warg-B', 'name': 'Warg B'},
            ],
          },
        };
      case 'private.npc.inventory':
        return {
          'ok': true,
          'data': {
            'id': payload['id'],
            'itemStackCount': payload['id'] == 'Warg-B' ? 1 : 2,
            'items': [
              {
                'id': _jawId,
                'path': _jawPath,
                'count': 1,
                'removable': true,
                'slotId': 0,
                'containerType': 'MeleeSlot',
              },
              if (payload['id'] != 'Warg-B')
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
