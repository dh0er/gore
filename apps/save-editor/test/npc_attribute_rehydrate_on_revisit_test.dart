import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';
import 'support/detail_tabs.dart';

/// Regression test for Bug #8: switching away from an NPC whose attribute draft
/// is queued and returning must REHYDRATE the panel's local field state from the
/// stored per-NPC entry. Editing a SECOND attribute on the revisited NPC must
/// keep BOTH that NPC's attribute edits — without rehydration the panel's
/// _recomputePending would emit only the newly-dirty field and the parent would
/// replace the stored entry, dropping the earlier edit (while the Save badge had
/// counted it).
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
    'returning to an edited NPC keeps earlier attribute edits when editing again',
    (tester) async {
      final core = _TwoAttributeNpcCoreService();
      await pumpApp(tester, core);

      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Attributes'));
      await tester.pumpAndSettle();

      // NPC-A exposes Health + Strength (both in Main stats). Edit Health base.
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('npc-attribute:Health:base')),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const ValueKey('npc-attribute:Health:base')),
        '111',
      );
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Switch to NPC-B, then back to NPC-A. The panel reload drops its local
      // _pending but A's queued edit survives in the registry.
      await tester.tap(find.text('Lizard-B'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();

      // Health base shows the rehydrated draft (111), not the saved 10.
      final healthField = tester.widget<EditableText>(
        find.descendant(
          of: find.byKey(const ValueKey('npc-attribute:Health:base')),
          matching: find.byType(EditableText),
        ),
      );
      expect(healthField.controller.text, '111');
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

      // Edit a SECOND attribute (Strength base). Both A edits must be queued.
      expect(
        find.byKey(const ValueKey('npc-attribute:Strength:base')),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const ValueKey('npc-attribute:Strength:base')),
        '99',
      );
      await tester.pump();
      expect(find.widgetWithText(FilledButton, 'Save (2)'), findsOneWidget);

      // Save: the write batch must carry BOTH attribute edits for NPC-A.
      await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
      await tester.pumpAndSettle();

      final write = core.requests.lastWhere((r) => r.command == 'write_save');
      final edits = (write.payload['edits'] as List)
          .cast<Map<String, Object?>>();
      expect(edits, hasLength(2));

      Map<String, Object?> editForKey(String key) {
        return edits.firstWhere((e) {
          final value = e['value'] as Map<String, Object?>;
          final path = (value['path'] as List).cast<String>();
          return path.contains('{$key}');
        });
      }

      final health = editForKey('Health')['value'] as Map<String, Object?>;
      final strength = editForKey('Strength')['value'] as Map<String, Object?>;
      expect(health['value'], 111.0);
      expect(strength['value'], 99.0);
      // Both target NPC-A's GlobalId path.
      for (final e in edits) {
        final path = ((e['value'] as Map)['path'] as List).cast<String>();
        expect(path, contains('{Lizard-A}'));
      }
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

/// Fake core with two NPCs. NPC-A exposes both Health and Strength attributes so
/// a multi-attribute draft can be built and re-verified on revisit.
class _TwoAttributeNpcCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'two-attribute-npc-fake-core';

  @override
  bool get isAvailable => true;

  List<String> _attrPath(String npcId, String key, String leaf) => [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{$npcId}',
    'AttributeSetsByClass',
    '{/Script/G1R.AttributeSet_$key}',
    'Attributes',
    '{$key}',
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
      case 'private.npc.attributes':
        final id = payload['id'] as String;
        final base = id == 'Lizard-A' ? 10.0 : 20.0;
        return {
          'ok': true,
          'data': {
            'attributes': [
              {
                'key': 'Health',
                'base': base,
                'current': base,
                'basePath': _attrPath(id, 'Health', 'BaseValue'),
                'currentPath': _attrPath(id, 'Health', 'CurrentValue'),
              },
              {
                'key': 'Strength',
                'base': base + 5,
                'current': base + 5,
                'basePath': _attrPath(id, 'Strength', 'BaseValue'),
                'currentPath': _attrPath(id, 'Strength', 'CurrentValue'),
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
