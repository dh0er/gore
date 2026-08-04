import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';

/// Regression test for Bug #7: switching away from an edited NPC inventory and
/// returning must REHYDRATE the card from the queued per-NPC draft. Editing a
/// SECOND item on the revisited NPC must keep BOTH that NPC's edits — without
/// rehydration the second push would replace the stored entry with only the new
/// edit, silently dropping the first.
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
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    'returning to an edited NPC inventory keeps earlier edits when editing again',
    (tester) async {
      final core = _TwoItemNpcInventoryCoreService();
      await pumpApp(tester, core);

      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Inventory'));
      await tester.pumpAndSettle();

      // NPC-A has two items. Edit the FIRST item's count.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      final countFields = find.widgetWithText(TextField, 'Count');
      expect(countFields, findsNWidgets(2));
      await tester.enterText(countFields.at(0), '11');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch to NPC-B, then back to NPC-A. The card is rebuilt with empty
      // local state but A's queued edit survives in the registry.
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();

      // The first item's field shows the rehydrated draft (11), not the saved 3.
      final revisited = find.widgetWithText(TextField, 'Count');
      final firstField = tester.widget<EditableText>(
        find.descendant(
          of: revisited.at(0),
          matching: find.byType(EditableText),
        ),
      );
      expect(firstField.controller.text, '11');
      // Still exactly one queued edit so far.
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Now edit the SECOND item's count. BOTH A edits must be queued.
      await tester.enterText(revisited.at(1), '22');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

      // Save: the write batch must carry BOTH count edits for NPC-A.
      await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(2));
      final values = edits
          .map((e) => e['value'] as Map<String, Object?>)
          .toList();
      expect(values.every((v) => v['actorId'] == 'Lizard-A'), isTrue);
      final counts = values.map((v) => v['count']).toSet();
      expect(counts, {11, 22});
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

/// Fake core where each NPC's MainContainer holds TWO distinct count-editable
/// stacks, so a multi-item draft can be built and re-verified on revisit.
class _TwoItemNpcInventoryCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'two-item-npc-inventory-fake-core';

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
            'total': 2,
            'offset': 0,
            'limit': payload['limit'] ?? 100,
            'count': 2,
            'npcs': [
              {'id': 'Lizard-A', 'name': 'Lizard A'},
              {'id': 'Lizard-B', 'name': 'Lizard B'},
            ],
          },
        };
      case 'private.npc.inventory':
        final id = payload['id'] as String;
        final base = id == 'Lizard-A' ? 3 : 5;
        return {
          'ok': true,
          'data': {
            'id': id,
            'itemStackCount': 2,
            'items': [
              {
                'id': 'Potion',
                'path': 'MainContainer[0]',
                'count': base,
                'removable': true,
                'slotId': 0,
              },
              {
                'id': 'Apple',
                'path': 'MainContainer[1]',
                'count': base + 1,
                'removable': true,
                'slotId': 1,
              },
            ],
            'mainContainerPaths': ['MainContainer[0]', 'MainContainer[1]'],
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
