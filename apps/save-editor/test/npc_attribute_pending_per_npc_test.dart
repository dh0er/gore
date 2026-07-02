import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

/// Regression test for the phantom cross-NPC edit bug: NPC attribute pending
/// edits must be keyed PER-NPC (`npc.attributes:$id`) so that editing NPC-A,
/// switching to NPC-B, and editing NPC-B keeps BOTH edits — each applied to the
/// correct NPC on Save — instead of NPC-A's edit silently surviving under a
/// shared key (or being clobbered by NPC-B's).
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

  testWidgets(
    'editing NPC-A then NPC-B keeps both edits keyed to their own NPC',
    (tester) async {
      final core = _NpcCoreService();
      await pumpApp(tester, core);

      // Open the Charaktere tab (shared master list) then its Attribute
      // sub-tab (which hosts the NPC editor).
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Attributes'));
      await tester.pumpAndSettle();

      // Select NPC-A from the shared master list and edit its Health base. With
      // an empty loc catalog the row subtitle shows the raw GlobalId.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      expect(find.widgetWithText(TextField, 'Health base'), findsOneWidget);
      await tester.enterText(
        find.widgetWithText(TextField, 'Health base'),
        '111',
      );
      await tester.pump();
      // One pending edit registered (NPC-A).
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch to NPC-B. selectActor does NOT clear pending edits, and the panel
      // reloads B's rows — but A's edit must remain under its own key.
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();
      // A's edit survives the switch: the Save count is still 1, and the field
      // now shows B's saved value (not A's draft).
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
      final bField = tester.widget<EditableText>(
        find.descendant(
          of: find.widgetWithText(TextField, 'Health base'),
          matching: find.byType(EditableText),
        ),
      );
      expect(bField.controller.text, isNot('111'));

      // Edit NPC-B's Health base. Now there are two pending edits (A + B).
      await tester.enterText(
        find.widgetWithText(TextField, 'Health base'),
        '222',
      );
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

      // Save. The single write_save batch must contain BOTH edits, each
      // targeting its own NPC's distinct GlobalId typed path.
      await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(2));

      // Each edit is a typed setValue carrying its NPC's GlobalId path + value.
      Map<String, Object?> editFor(String npcId) {
        return edits.firstWhere((e) {
          final value = e['value'] as Map<String, Object?>;
          final path = (value['path'] as List).cast<String>();
          return path.contains('{$npcId}');
        });
      }

      final aEdit = editFor('Lizard-A');
      final bEdit = editFor('Lizard-B');
      // NPC-A's edit was NOT lost and NOT clobbered — it kept value 111.
      expect((aEdit['value'] as Map)['value'], 111.0);
      // NPC-B's edit kept value 222.
      expect((bEdit['value'] as Map)['value'], 222.0);
      // Both target the BaseValue leaf of their own NPC.
      expect(
        ((aEdit['value'] as Map)['path'] as List).last,
        'BaseValue',
      );
      expect(
        ((bEdit['value'] as Map)['path'] as List).last,
        'BaseValue',
      );
    },
  );

  testWidgets(
    'selecting an NPC with no edits leaves no stale pending entry',
    (tester) async {
      final core = _NpcCoreService();
      await pumpApp(tester, core);

      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(Tab, 'Attributes'));
      await tester.pumpAndSettle();

      // Edit NPC-A (1 pending), then visit NPC-B but make NO edit.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.widgetWithText(TextField, 'Health base'),
        '111',
      );
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();

      // Visiting B without editing must not add a pending entry for B; the count
      // stays at 1 (only A's edit).
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Saving writes exactly A's one edit.
      await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(1));
      final value = edits.single['value'] as Map<String, Object?>;
      expect((value['path'] as List).cast<String>(), contains('{Lizard-A}'));
      expect(value['value'], 111.0);
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

/// Fake core with two NPCs (Lizard-A, Lizard-B), each exposing a single Health
/// attribute under its own GlobalId typed path. Mirrors the production
/// `private.npc.list` / `private.npc.attributes` contract.
class _NpcCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'npc-fake-core';

  @override
  bool get isAvailable => true;

  List<String> _attrPath(String npcId, String leaf) => [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{$npcId}',
    'AttributeSetsByClass',
    '{/Script/G1R.AttributeSet_Health}',
    'Attributes',
    '{Health}',
    leaf,
  ];

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
                'chapterId': 1,
                'timePlayedSeconds': 6963.34,
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
            'public': {
              'slotName': 'G1R-001',
              'playerSaveName': 'Die Welt der Verurteilten',
            },
            'private': {
              'status': 'decoded',
              'preview': false,
              'decompressedSize': 9,
              'stringCount': 1,
              'strings': ['Hero'],
              'typedParse': {
                'status': 'ok',
                'propertyCount': 1,
                'maxDepth': 1,
              },
              'player': {
                'saveVersionNumber': 17,
                'currentWorld': 'WORLD',
                'playerName': 'Hero',
                'profileName': '0',
                'attributes': [
                  {'id': 'Health', 'baseValue': 40.0, 'currentValue': 25.0},
                ],
                'writable': [
                  'private.player.setAttribute',
                  'private.typed.setValue',
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
      case 'private.npc.attributes':
        final id = payload['id'] as String;
        // Distinct saved Base/Current per NPC so the seeded values differ.
        final base = id == 'Lizard-A' ? 10.0 : 20.0;
        return {
          'ok': true,
          'data': {
            'attributes': [
              {
                'key': 'Health',
                'base': base,
                'current': base,
                'basePath': _attrPath(id, 'BaseValue'),
                'currentPath': _attrPath(id, 'CurrentValue'),
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
