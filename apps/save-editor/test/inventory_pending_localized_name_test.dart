import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';
import 'support/detail_tabs.dart';

/// The queued add/remove cards used to name the item from its class id alone
/// ("2H Sword Heavy 02"), while the picker that produced it and the inventory
/// rows beside it both showed the localized game name. Both pending cards must
/// resolve the loc catalog like every other item row, keeping the asset path as
/// the technical subtitle.
void main() {
  const cheeseId = 'ItFo_Cheese';
  const cheesePath = '/Script/Angelscript.$cheeseId';
  const swordId = 'ItMw_2H_Sword_Heavy_02';
  const swordPath = '/Script/Angelscript.$swordId';

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
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
          // Catalog keys are lowercased by the real loader; the override
          // supplies them already lowercased.
          locCatalogProvider.overrideWith(
            (ref) async => const {
              'itfo_cheese': {'english': 'Wheel of Cheese'},
              'itmw_2h_sword_heavy_02': {'english': 'Heavy Two-Hander'},
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
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
  }

  testWidgets('a queued add names the item from the loc catalog', (
    tester,
  ) async {
    await pumpApp(tester, _NpcInventoryCoreService());
    await openNpcInventory(tester);

    // Pick the sword out of the bundled catalog by its class id.
    await tester.tap(find.widgetWithText(FilledButton, 'Add item'));
    await tester.pump();
    // The picker reads the bundled item catalog through rootBundle, which needs
    // real (non-fake) async to complete; until then it shows a spinner that
    // pumpAndSettle would time out on.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pumpAndSettle();
    // The inventory card behind the dialog has its own search field, so scope
    // the query to the dialog's.
    final dialogSearch = find
        .descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(TextField),
        )
        .first;
    await tester.enterText(dialogSearch, swordId);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Heavy Two-Hander').first);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Add'));
    await tester.pumpAndSettle();

    // The queued card reads like the picker did, not "2H Sword Heavy 02".
    expect(find.text('Heavy Two-Hander'), findsOneWidget);
    expect(find.text('2H Sword Heavy 02'), findsNothing);
    // The full asset path stays below it as the technical id.
    expect(find.text(swordPath), findsOneWidget);
  });

  testWidgets('a queued removal names the item from the loc catalog', (
    tester,
  ) async {
    await pumpApp(tester, _NpcInventoryCoreService());
    await openNpcInventory(tester);

    // The bundled item stats decide what a row IS, and until they answer the
    // editor offers no removal — reading them needs real async.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.delete_outline).first);
    await tester.pumpAndSettle();

    // Only the pending card is left (the row it replaced is hidden), and it
    // carries the localized name rather than the id-derived "Cheese".
    expect(find.text('Wheel of Cheese'), findsOneWidget);
    expect(find.text('Cheese'), findsNothing);
    expect(find.textContaining(cheesePath), findsOneWidget);
  });

  testWidgets('without a catalog the card still falls back to the id name', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(_NpcInventoryCoreService()),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
          locCatalogProvider.overrideWith((ref) async => const {}),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
    await openNpcInventory(tester);

    // The bundled item stats decide what a row IS, and until they answer the
    // editor offers no removal — reading them needs real async.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.delete_outline).first);
    await tester.pumpAndSettle();

    expect(find.text('Cheese'), findsOneWidget);
  });
}

/// NPC with a single removable MainContainer item, add + remove both writable.
class _NpcInventoryCoreService implements GoresaveCoreService {
  static const _cheesePath = '/Script/Angelscript.ItFo_Cheese';

  @override
  String get description => 'pending-localized-name-fake-core';

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
                'globalId': 'Lizard-A',
                'uniqueName': 'Lizard',
                'isDead': false,
                'hasInventory': false,
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
              {'id': 'Lizard-A', 'name': 'Lizard A'},
            ],
          },
        };
      case 'private.npc.inventory':
        return {
          'ok': true,
          'data': {
            'id': payload['id'],
            'itemStackCount': 1,
            'items': [
              {
                'id': 'ItFo_Cheese',
                'path': _cheesePath,
                'count': 3,
                'removable': true,
                'slotId': 0,
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
