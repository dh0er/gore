import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

/// An NPC inventory surfaces multiple containers (MainContainer + the equipped
/// MeleeSlot weapon + the ore Pouch). The card must (a) show every container's
/// row as a normal row (NO container badge — the badges were removed as
/// confusing), and (b) echo `containerType` (+ slotId for removes) back on the
/// edits so the core targets the right container's slot.
void main() {
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

  Future<void> openNpcInventory(WidgetTester tester) async {
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(Tab, 'Inventory'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
  }

  testWidgets('NPC weapon (MeleeSlot) and ore (Pouch) rows show as normal rows '
      'with NO container badge', (tester) async {
    final core = _MultiContainerNpcInventoryCoreService();
    await pumpApp(tester, core);
    await openNpcInventory(tester);

    // A flat search across all categories surfaces every container's row at once
    // (the weapon, the ore, and the MainContainer apple). All appear as normal
    // rows; the confusing per-container badges were removed.
    await tester.enterText(find.widgetWithText(TextField, 'Filter items'), 'It');
    await tester.pumpAndSettle();

    // The weapon + ore rows are present (by their ids — no loc catalog in test).
    expect(find.text('ItMw_Sword'), findsOneWidget);
    expect(find.text('ItMi_Orenugget'), findsOneWidget);
    // No container badge labels are rendered anywhere.
    expect(find.text('Weapon'), findsNothing);
    expect(find.text('Bow'), findsNothing);
    expect(find.text('Pouch'), findsNothing);
  });

  testWidgets('a count edit on the Pouch ore echoes containerType + slotId',
      (tester) async {
    final core = _MultiContainerNpcInventoryCoreService();
    await pumpApp(tester, core);
    await openNpcInventory(tester);

    // Flat list (search) shows every row; edit the Pouch ore row to 99. The row
    // is found by the ore's id since there is no longer a container badge.
    await tester.enterText(find.widgetWithText(TextField, 'Filter items'), 'It');
    await tester.pumpAndSettle();

    final oreField = find.descendant(
      of: find.ancestor(
        of: find.text('ItMi_Orenugget'),
        matching: find.byType(ListTile),
      ),
      matching: find.byType(TextField),
    );
    expect(oreField, findsOneWidget);
    await tester.enterText(oreField, '99');
    await tester.pump();

    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();

    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits =
        (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(1));
    final value = edits.single['value'] as Map<String, Object?>;
    expect(value['count'], 99);
    expect(value['containerType'], 'Pouch');
    expect(value['slotId'], 5);
    expect(value['actorId'], 'Lizard-A');
  });

  testWidgets('an empty NPC inventory shows the empty message', (tester) async {
    final core = _MultiContainerNpcInventoryCoreService(emptyInventory: true);
    await pumpApp(tester, core);
    await openNpcInventory(tester);

    expect(find.text('This inventory is empty.'), findsOneWidget);
    // No item filter field / rows when there is nothing to show.
    expect(find.widgetWithText(TextField, 'Filter items'), findsNothing);
  });
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// NPC inventory with three containers: a MainContainer apple, an equipped
/// MeleeSlot sword, and a Pouch ore stack (each removable + editable).
class _MultiContainerNpcInventoryCoreService implements GoresaveCoreService {
  _MultiContainerNpcInventoryCoreService({this.emptyInventory = false});

  /// When true, `private.npc.inventory` returns zero items (to exercise the
  /// empty-inventory message).
  final bool emptyInventory;

  final requests = <_RecordedRequest>[];

  @override
  String get description => 'multi-container-npc-inventory-fake-core';

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
        // Backs the Charaktere master list (one unpaginated response). The
        // globalId is rendered as the row subtitle, so tests select a row by
        // tapping its GlobalId text (e.g. 'Lizard-A').
        return {
          'ok': true,
          'data': {
            'total': 2,
            'characters': [
              {
                'globalId': 'Lizard-A',
                'uniqueName': 'Lizard',
                'isDead': false,
                'hasInventory': false,
                'hasKnowledge': false,
                'hasEvents': false,
              },
              {
                'globalId': 'Lizard-B',
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
              {'id': 'Lizard-A', 'name': 'Lizard A', 'isDead': false},
            ],
          },
        };
      case 'private.npc.inventory':
        if (emptyInventory) {
          // Empty but still editable (add is possible) → the card renders and
          // shows the friendly empty message (not the locked "no stacks" pane).
          return {
            'ok': true,
            'data': {
              'id': payload['id'],
              'itemStackCount': 0,
              'items': <Object?>[],
              'mainContainerPaths': <String>[],
              'writable': ['private.inventory.addItem'],
            },
          };
        }
        return {
          'ok': true,
          'data': {
            'id': payload['id'],
            'itemStackCount': 3,
            'items': [
              {
                'id': 'ItFo_Apple',
                'path': '/Script/Angelscript.ItFo_Apple',
                'count': 4,
                'removable': true,
                'slotId': 0,
                'containerType': 'MainContainer',
              },
              {
                'id': 'ItMw_Sword',
                'path': '/Script/Angelscript.ItMw_Sword',
                'count': 1,
                'removable': true,
                'slotId': 1,
                'containerType': 'MeleeSlot',
              },
              {
                'id': 'ItMi_Orenugget',
                'path': '/Script/Angelscript.ItMi_Orenugget',
                'count': 12,
                'removable': true,
                'slotId': 5,
                'containerType': 'Pouch',
              },
            ],
            'mainContainerPaths': <String>[],
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
