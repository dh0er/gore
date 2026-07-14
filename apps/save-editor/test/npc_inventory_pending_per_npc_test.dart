import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';

/// Regression test mirroring `npc_attribute_pending_per_npc_test.dart` for the
/// Inventory tab: NPC inventory pending edits must be keyed PER-NPC
/// (`inventory:$id`) so editing NPC-A's inventory, switching to NPC-B, and
/// editing NPC-B keeps BOTH edits — each carrying its own NPC's `actorId` — and
/// the shared selector reflects the same actor as the Attribute tab.
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
    'editing NPC-A then NPC-B inventory keeps both edits with their own actorId',
    (tester) async {
      final core = _NpcInventoryCoreService();
      await pumpApp(tester, core);

      // Open the Charaktere tab (shared master list) then its Inventar sub-tab.
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Inventory'));
      await tester.pumpAndSettle();

      // Select NPC-A and bump its single item's count.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      expect(find.widgetWithText(TextField, 'Count'), findsOneWidget);
      await tester.enterText(find.widgetWithText(TextField, 'Count'), '11');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch to NPC-B — A's edit must survive under its own per-NPC key.
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
      // The field now shows B's saved count, not A's draft.
      final bField = tester.widget<EditableText>(
        find.descendant(
          of: find.widgetWithText(TextField, 'Count'),
          matching: find.byType(EditableText),
        ),
      );
      expect(bField.controller.text, isNot('11'));

      // Edit NPC-B's count → two pending edits (A + B).
      await tester.enterText(find.widgetWithText(TextField, 'Count'), '22');
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

      // Save: the single write_save batch carries BOTH edits, each tagged with
      // its NPC's actorId.
      await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(2));

      Map<String, Object?> valueFor(String actorId) {
        return edits
            .map((e) => e['value'] as Map<String, Object?>)
            .firstWhere((v) => v['actorId'] == actorId);
      }

      expect(valueFor('Lizard-A')['count'], 11);
      expect(valueFor('Lizard-B')['count'], 22);
      // Both are setItemCount edits.
      for (final e in edits) {
        expect(e['path'], 'private.inventory.setItemCount');
      }
    },
  );

  testWidgets(
    'shared selection: NPC chosen on Inventory tab reflects on Attribute tab',
    (tester) async {
      final core = _NpcInventoryCoreService();
      await pumpApp(tester, core);

      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Inventory'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();

      // The Attribute sub-tab reads the SAME shared selectedActor — switching to
      // it (no re-select) loads Lizard-B's NPC attributes (the fake returns
      // Lizard-B's attribute row), proving selection is shared across sub-tabs.
      await tester.tap(find.widgetWithText(Tab, 'Attributes'));
      await tester.pumpAndSettle();
      final attrReq = core.requests.lastWhere(
        (r) => r.command == 'private.npc.attributes',
      );
      expect(attrReq.payload['id'], 'Lizard-B');
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

/// Fake core with two NPCs, each exposing a single count-editable inventory
/// stack via `private.npc.inventory` (same shape as the player summary).
class _NpcInventoryCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'npc-inventory-fake-core';

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
              // Player inventory present but empty — the player path is not
              // exercised here; NPC inventory drives the test.
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
        final count = id == 'Lizard-A' ? 3 : 5;
        return {
          'ok': true,
          'data': {
            'id': id,
            'itemStackCount': 1,
            'items': [
              {
                'id': 'Potion',
                'path': 'MainContainer[0]',
                'count': count,
                'removable': true,
              },
            ],
            'mainContainerPaths': ['MainContainer[0]'],
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
          'data': {
            'attributes': [
              {
                'key': 'Health',
                'base': 10.0,
                'current': 10.0,
                'basePath': ['AttributesByGlobalId', '{${payload['id']}}'],
                'currentPath': ['AttributesByGlobalId', '{${payload['id']}}'],
              },
            ],
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
